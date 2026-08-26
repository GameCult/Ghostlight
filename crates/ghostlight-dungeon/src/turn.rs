use crate::{
    domain::{ActorReaction, Campaign, GestaltReaction},
    model::{ModelPort, ModelStageReceipt, ModelStageRequest, run_validated_stage},
    persistence::CampaignStore,
    persona::{
        ActorInteractionRole, ExecutionPermit, PermittedActorSlice, PersonaProjectionEngine,
        PersonaSubjectKind,
    },
};
use anyhow::{Result, anyhow};
use async_trait::async_trait;
use schemars::{JsonSchema, schema_for};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, sync::Arc};

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct SpeechAddressPlan {
    /// Exact present, model-controlled Persona subjects from whom the utterance
    /// requests an answer. Mentioned subjects and passive hearers are omitted.
    pub persona_response_actor_ids: BTreeSet<String>,
}

/// Resolve conversational address from player language against the exact
/// present-subject catalog. The output is proposal-only until WorldKernel
/// validates co-presence and commits it with the speech turn.
pub async fn resolve_speech_addresses(
    model: Arc<dyn ModelPort>,
    model_name: &str,
    campaign: &Campaign,
    speaker_actor_id: &str,
    speech: &str,
) -> Result<(SpeechAddressPlan, Vec<ModelStageReceipt>)> {
    let speaker = campaign
        .actors
        .get(speaker_actor_id)
        .ok_or_else(|| anyhow!("speech actor is unknown"))?;
    let mut candidates = campaign
        .actors
        .values()
        .filter(|actor| {
            actor.id != speaker_actor_id
                && actor.location_id == speaker.location_id
                && campaign
                    .agency_profiles
                    .get(&actor.id)
                    .is_none_or(|profile| profile.simulation_eligible)
        })
        .map(|actor| {
            (
                actor.id.clone(),
                serde_json::json!({
                    "public_name": actor.name,
                    "presence": "materialized_actor",
                }),
            )
        })
        .collect::<std::collections::BTreeMap<_, _>>();
    candidates.extend(campaign.gestalt_members.values().filter_map(|member| {
        if member.materialized_actor_id.is_some()
            || !crate::resolution::dormant_member_location(campaign, &member.id)
                .is_ok_and(|location| location == speaker.location_id)
        {
            return None;
        }
        Some((
            crate::domain::gestalt_member_subject_id(&member.id),
            serde_json::json!({
                "public_name": member.name,
                "presence": "nearby_folded_person",
            }),
        ))
    }));
    candidates.extend(campaign.gestalts.values().filter_map(|gestalt| {
        crate::resolution::validate_active_gestalt_presence_location(
            campaign,
            &gestalt.id,
            &speaker.location_id,
        )
        .ok()?;
        Some((
            gestalt.id.clone(),
            serde_json::json!({
                "public_name": gestalt.name,
                "presence": "cohesive_population",
            }),
        ))
    }));
    if candidates.is_empty() {
        return Ok((
            SpeechAddressPlan {
                persona_response_actor_ids: BTreeSet::new(),
            },
            Vec::new(),
        ));
    }
    let allowed_ids = candidates.keys().cloned().collect::<BTreeSet<_>>();
    let previous_focus = campaign
        .transcript
        .iter()
        .rev()
        .find(|turn| {
            turn.speaker == speaker_actor_id && !turn.persona_response_actor_ids.is_empty()
        })
        .map(|turn| turn.persona_response_actor_ids.clone())
        .unwrap_or_default();
    let recent_public_turns = campaign
        .transcript
        .iter()
        .rev()
        .take(8)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .map(|turn| {
            serde_json::json!({
                "speaker":turn.speaker,
                "text":turn.text,
                "persona_response_actor_ids":turn.persona_response_actor_ids,
            })
        })
        .collect::<Vec<_>>();
    let mut schema = serde_json::to_value(schema_for!(SpeechAddressPlan))?;
    schema["properties"]["persona_response_actor_ids"] = serde_json::json!({
        "type":"array",
        "uniqueItems":true,
        "maxItems":allowed_ids.len(),
        "items":{"type":"string","enum":allowed_ids},
    });
    let prompt = format!(
        "You are Ghostlight's private scene-address resolver. Decide only which supplied present Persona subjects this exact speech directly asks to answer now. A nearby_folded_person is a persistent known individual currently represented through their population; selecting their exact member ID lets Ghostlight materialize that same person before appraisal. A cohesive_population is a genuine collective subject and may be directly asked for a plural response. A subject who is merely mentioned, discussed, visible, or able to overhear is not a response target. A room-wide statement need not demand an answer from everyone. Preserve ordinary conversational focus from previous_focus when pronouns or an unqualified follow-up clearly continue it. Select only exact supplied IDs. Empty is valid when the speaker asks nobody present to answer. Return exactly one JSON object matching the schema.\nSCHEMA:\n{}\nPRESENT PERSONA SUBJECTS:\n{}\nPREVIOUS FOCUS:\n{}\nRECENT PUBLIC TURNS:\n{}\nSPEAKER ACTOR ID:\n{}\nEXACT SPEECH:\n{}",
        serde_json::to_string(&schema)?,
        serde_json::to_string(&candidates)?,
        serde_json::to_string(&previous_focus)?,
        serde_json::to_string(&recent_public_turns)?,
        serde_json::to_string(speaker_actor_id)?,
        serde_json::to_string(speech)?,
    );
    let output = run_validated_stage(
        model.as_ref(),
        &ModelStageRequest {
            stage: "speech_address_resolver".into(),
            model: model_name.into(),
            snapshot_binding: format!("campaign:{}:revision:{}", campaign.id, campaign.revision),
            lived_stream: prompt,
            output_schema: Some(schema),
            source_receipt_ids: campaign.branch_origin.evidence_receipt_ids.clone(),
            temperature: Some(0.0),
            max_output_tokens: Some(256),
        },
    )
    .await?;
    let plan: SpeechAddressPlan = serde_json::from_value(
        output
            .structured
            .clone()
            .ok_or_else(|| anyhow!("speech address resolver produced no plan"))?,
    )?;
    if !plan.persona_response_actor_ids.is_subset(&allowed_ids) {
        return Err(anyhow!(
            "speech address resolver selected an unavailable actor"
        ));
    }
    Ok((plan, vec![output.receipt]))
}

pub struct SnapshotPermit {
    store: CampaignStore,
    campaign_id: uuid::Uuid,
    revision: u64,
    resolution_epoch: Option<u64>,
}
impl SnapshotPermit {
    pub fn new(store: CampaignStore, campaign_id: uuid::Uuid, revision: u64) -> Self {
        Self {
            store,
            campaign_id,
            revision,
            resolution_epoch: None,
        }
    }
    pub fn new_resolution(
        store: CampaignStore,
        campaign_id: uuid::Uuid,
        revision: u64,
        resolution_epoch: u64,
    ) -> Self {
        Self {
            store,
            campaign_id,
            revision,
            resolution_epoch: Some(resolution_epoch),
        }
    }
}
#[async_trait]
impl ExecutionPermit for SnapshotPermit {
    async fn require(&self, _: &str, _: &str, _: &str) -> Result<()> {
        let value = self
            .store
            .load::<Campaign>("campaign.v1", &self.campaign_id.to_string())?
            .map(|(_, c)| c)
            .ok_or_else(|| anyhow!("campaign vanished during Persona wave"))?;
        if value.revision != self.revision
            || self
                .resolution_epoch
                .is_some_and(|epoch| value.resolution_policy.resolution_epoch != epoch)
        {
            return Err(anyhow!("Persona projection snapshot is stale"));
        }
        Ok(())
    }
}

pub struct ReactionWaveOutput {
    pub reactions: Vec<ActorReaction>,
    pub gestalt_reactions: Vec<GestaltReaction>,
    pub receipts: Vec<ModelStageReceipt>,
}

enum PresentReactionTerminal {
    Actor(String, crate::persona::PersonaTerminalBundle),
    Gestalt(String, crate::persona::PersonaTerminalBundle),
}

pub async fn appraise_present(
    engine: PersonaProjectionEngine,
    campaign: &Campaign,
    event_summary: &str,
) -> Result<ReactionWaveOutput> {
    let response_expected_actor_ids = canonical_reaction_turn(campaign, event_summary)?
        .persona_response_actor_ids
        .clone();
    let player = campaign
        .actors
        .get(&campaign.player_actor_id)
        .ok_or_else(|| anyhow!("player actor missing"))?;
    let actors: Vec<_> = campaign
        .actors
        .values()
        .filter(|actor| {
            actor.location_id == player.location_id
                && campaign
                    .agency_profiles
                    .get(&actor.id)
                    .is_none_or(|profile| profile.simulation_eligible)
        })
        .cloned()
        .collect();
    let mut perceived_actors = campaign
        .actors
        .values()
        .filter(|actor| actor.location_id == player.location_id)
        .map(|actor| (actor.id.clone(), actor.name.clone()))
        .collect::<std::collections::BTreeMap<_, _>>();
    let local_gestalts = campaign
        .gestalts
        .values()
        .filter(|gestalt| {
            crate::resolution::validate_active_gestalt_presence_location(
                campaign,
                &gestalt.id,
                &player.location_id,
            )
            .is_ok()
        })
        .cloned()
        .collect::<Vec<_>>();
    perceived_actors.extend(
        local_gestalts
            .iter()
            .map(|gestalt| (gestalt.id.clone(), gestalt.name.clone())),
    );
    let semaphore = std::sync::Arc::new(tokio::sync::Semaphore::new(usize::from(
        campaign.resolution_policy.provider_parallelism.max(1),
    )));
    let mut jobs = tokio::task::JoinSet::new();
    for actor in actors {
        let engine = engine.clone();
        let snapshot = format!("campaign:{}:revision:{}", campaign.id, campaign.revision);
        let event = event_summary.to_owned();
        let receipts = campaign.branch_origin.evidence_receipt_ids.clone();
        let perceived_actors = perceived_actors.clone();
        let reserved_public_identities = reserved_public_identities(campaign, &actor);
        let recent_self_authored_turns = recent_self_authored_turns(campaign, &actor.id);
        let semaphore = semaphore.clone();
        let interaction_role = if response_expected_actor_ids.contains(&actor.id) {
            ActorInteractionRole::DirectResponseExpected
        } else {
            ActorInteractionRole::PresentObserver
        };
        jobs.spawn(async move {
            let _provider_slot = semaphore
                .acquire_owned()
                .await
                .map_err(|_| anyhow!("provider concurrency gate closed"))?;
            let slice = PermittedActorSlice {
                actor_id: actor.id.clone(),
                location_id: actor.location_id.clone(),
                subject_kind: PersonaSubjectKind::IndividualActor,
                snapshot_binding: snapshot,
                interaction_role,
                identity_experience: vec![format!("You are {}.", actor.name)],
                reserved_public_identities,
                memories: actor.memories,
                recent_self_authored_turns,
                perceived_events: vec![event],
                perceived_actors,
                relationships: actor
                    .relationships
                    .into_iter()
                    .map(|(id, value)| format!("{id}: {value}"))
                    .collect(),
                goals: actor.goals,
                knowledge: actor.knowledge.into_iter().collect(),
                capabilities: actor.capabilities.into_iter().collect(),
                pressures: actor
                    .conditions
                    .into_iter()
                    .chain(actor.obligations)
                    .collect(),
                affordances: actor.equipment.into_iter().collect(),
                source_receipt_ids: receipts,
            };
            let terminal = engine.execute(slice).await?;
            Ok::<_, anyhow::Error>(PresentReactionTerminal::Actor(actor.id, terminal))
        });
    }
    for gestalt in local_gestalts {
        let engine = engine.clone();
        let snapshot = format!("campaign:{}:revision:{}", campaign.id, campaign.revision);
        let event = event_summary.to_owned();
        let receipts = campaign.branch_origin.evidence_receipt_ids.clone();
        let perceived_actors = perceived_actors.clone();
        let recent_self_authored_turns = recent_self_authored_turns(campaign, &gestalt.id);
        let semaphore = semaphore.clone();
        let interaction_role = if response_expected_actor_ids.contains(&gestalt.id) {
            ActorInteractionRole::DirectResponseExpected
        } else {
            ActorInteractionRole::PresentObserver
        };
        jobs.spawn(async move {
            let _provider_slot = semaphore
                .acquire_owned()
                .await
                .map_err(|_| anyhow!("provider concurrency gate closed"))?;
            let slice = PermittedActorSlice {
                actor_id: gestalt.id.clone(),
                location_id: gestalt.home_location_id.clone(),
                subject_kind: PersonaSubjectKind::CohesiveGestalt,
                snapshot_binding: snapshot,
                interaction_role,
                identity_experience: vec![format!(
                    "You are {}, a cohesive population represented through genuinely shared state.",
                    gestalt.name
                )],
                reserved_public_identities: perceived_actors.values().cloned().collect(),
                memories: vec![],
                recent_self_authored_turns,
                perceived_events: vec![event],
                perceived_actors,
                relationships: vec![],
                goals: gestalt.goals,
                knowledge: gestalt.shared_knowledge.into_iter().collect(),
                capabilities: gestalt.shared_capabilities.into_iter().collect(),
                pressures: gestalt.pressures,
                affordances: gestalt.resources.into_iter().collect(),
                source_receipt_ids: receipts,
            };
            let terminal = engine.execute(slice).await?;
            Ok::<_, anyhow::Error>(PresentReactionTerminal::Gestalt(gestalt.id, terminal))
        });
    }
    let mut reactions = Vec::new();
    let mut gestalt_reactions = Vec::new();
    let mut receipts = Vec::new();
    while let Some(result) = jobs.join_next().await {
        match result.map_err(|e| anyhow!("Persona appraisal task failed: {e}"))?? {
            PresentReactionTerminal::Actor(actor_id, terminal) => {
                receipts.extend(terminal.stage_receipts);
                reactions.push(ActorReaction {
                    actor_id,
                    speech: terminal.proposals.speech,
                    deliberate_silence: terminal.proposals.deliberate_silence,
                    private_delta: terminal.proposals.private_delta,
                    action_proposals: terminal.proposals.world_actions,
                });
            }
            PresentReactionTerminal::Gestalt(gestalt_id, terminal) => {
                receipts.extend(terminal.stage_receipts);
                gestalt_reactions.push(GestaltReaction {
                    gestalt_id,
                    speech: terminal.proposals.speech,
                    deliberate_silence: terminal.proposals.deliberate_silence,
                });
            }
        }
    }
    reactions.sort_by(|a, b| a.actor_id.cmp(&b.actor_id));
    gestalt_reactions.sort_by(|a, b| a.gestalt_id.cmp(&b.gestalt_id));
    Ok(ReactionWaveOutput {
        reactions,
        gestalt_reactions,
        receipts,
    })
}

fn recent_self_authored_turns(campaign: &Campaign, subject_id: &str) -> Vec<String> {
    const LIMIT: usize = 8;
    let mut turns = campaign
        .transcript
        .iter()
        .rev()
        .filter(|turn| turn.speaker == subject_id)
        .take(LIMIT)
        .map(|turn| {
            format!(
                "At world revision {}, your committed public response was exactly: {}",
                turn.revision,
                turn.text.trim()
            )
        })
        .collect::<Vec<_>>();
    turns.reverse();
    turns
}

fn reserved_public_identities(
    campaign: &Campaign,
    actor: &crate::domain::ActorState,
) -> BTreeSet<String> {
    let own_identity = actor.name.trim().to_lowercase();
    let mut identities = campaign
        .actors
        .values()
        .filter(|other| other.id != actor.id && other.location_id == actor.location_id)
        .map(|other| other.name.clone())
        .collect::<BTreeSet<_>>();
    if let Some(member) = campaign
        .gestalt_members
        .values()
        .find(|member| member.materialized_actor_id.as_deref() == Some(actor.id.as_str()))
    {
        identities.extend(
            campaign
                .gestalt_members
                .values()
                .filter(|peer| peer.id != member.id && peer.gestalt_id == member.gestalt_id)
                .map(|peer| peer.name.clone()),
        );
    }
    identities.retain(|identity| identity.trim().to_lowercase() != own_identity);
    identities
}

fn canonical_reaction_turn<'a>(
    campaign: &'a Campaign,
    event_summary: &str,
) -> Result<&'a crate::domain::NarrativeTurn> {
    let event_summary = event_summary.trim();
    campaign
        .transcript
        .iter()
        .rev()
        .find(|turn| {
            let canonical = if turn.speaker == "world" {
                turn.text.trim().to_owned()
            } else {
                format!("{} says: {}", turn.speaker, turn.text.trim())
            };
            canonical == event_summary
        })
        .ok_or_else(|| anyhow!("reaction stimulus does not match a committed transcript turn"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        domain::{ActorState, GestaltMemberDelta, GestaltPersonaState, NarrativeTurn},
        model::ModelStageRequest,
        persona::{AllowAllPermit, PersonaProjectionEngine},
    };
    use chrono::Utc;
    use std::collections::{BTreeMap, BTreeSet};

    struct AddressAwareModel;
    struct DormantAddressModel;
    struct CollectiveAddressModel;

    #[async_trait]
    impl ModelPort for AddressAwareModel {
        async fn run(&self, request: &ModelStageRequest) -> Result<String> {
            Ok(match request.stage.as_str() {
                "speech_address_resolver" => {
                    assert!(request.lived_stream.contains("Refugee One"));
                    assert!(request.lived_stream.contains("Refugee Two"));
                    assert!(request.lived_stream.contains("Relay Observer"));
                    serde_json::json!({
                        "persona_response_actor_ids":["refugee-one","refugee-two"]
                    })
                    .to_string()
                }
                "projector" => "The question lands in the small circle.".into(),
                "persona" => {
                    if request.lived_stream.contains("exact direct addressee") {
                        "I answer plainly: that memory is mine alone.".into()
                    } else {
                        "I listen without taking over their answer.".into()
                    }
                }
                "interpreter" => {
                    let direct = request.lived_stream.contains("direct_response_expected");
                    serde_json::json!({
                        "private_delta":{
                            "memories_add":[],
                            "conditions_add":[],
                            "conditions_remove":[],
                            "goals_add":[],
                            "relationship_updates":{},
                        },
                        "speech":direct.then_some("that memory is mine alone"),
                        "deliberate_silence":false,
                        "reaction_priority":0,
                        "world_actions":[],
                    })
                    .to_string()
                }
                stage => return Err(anyhow!("unexpected stage {stage}")),
            })
        }

        fn provider(&self) -> &'static str {
            "fixture"
        }
    }

    #[async_trait]
    impl ModelPort for DormantAddressModel {
        async fn run(&self, request: &ModelStageRequest) -> Result<String> {
            assert_eq!(request.stage, "speech_address_resolver");
            assert!(request.lived_stream.contains("nearby_folded_person"));
            assert!(request.lived_stream.contains("member:water-cart-taren"));
            Ok(serde_json::json!({
                "persona_response_actor_ids":["member:water-cart-taren"]
            })
            .to_string())
        }

        fn provider(&self) -> &'static str {
            "fixture"
        }
    }

    #[async_trait]
    impl ModelPort for CollectiveAddressModel {
        async fn run(&self, request: &ModelStageRequest) -> Result<String> {
            Ok(match request.stage.as_str() {
                "speech_address_resolver" => {
                    assert!(request.lived_stream.contains("cohesive_population"));
                    assert!(request.lived_stream.contains("Settlement households"));
                    serde_json::json!({
                        "persona_response_actor_ids":["settlement-households"]
                    })
                    .to_string()
                }
                "projector" => {
                    "The households hear a direct request for their shared answer.".into()
                }
                "persona" => "We will not accept a slate that makes our names disappear.".into(),
                "interpreter" => serde_json::json!({
                    "private_delta":{
                        "memories_add":[],
                        "identity_adoption":null,
                        "conditions_add":[],
                        "conditions_remove":[],
                        "goals_add":[],
                        "relationship_updates":{},
                    },
                    "speech":"We will not accept a slate that makes our names disappear.",
                    "deliberate_silence":false,
                    "reaction_priority":5,
                    "world_actions":[],
                })
                .to_string(),
                stage => return Err(anyhow!("unexpected stage {stage}")),
            })
        }

        fn provider(&self) -> &'static str {
            "fixture"
        }
    }

    fn actor(id: &str, name: &str, location_id: &str) -> ActorState {
        ActorState {
            id: id.into(),
            name: name.into(),
            location_id: location_id.into(),
            capabilities: BTreeSet::from(["ordinary conversation".into()]),
            knowledge: BTreeSet::new(),
            equipment: BTreeSet::new(),
            conditions: BTreeSet::new(),
            obligations: BTreeSet::new(),
            relationships: BTreeMap::new(),
            goals: vec![],
            memories: vec![],
        }
    }

    fn addressed_campaign() -> Campaign {
        let mut campaign = crate::resolution::tests::campaign(0, 8);
        let location = campaign.actors[&campaign.player_actor_id]
            .location_id
            .clone();
        campaign.actors.insert(
            "refugee-one".into(),
            actor("refugee-one", "Refugee One", &location),
        );
        campaign.actors.insert(
            "refugee-two".into(),
            actor("refugee-two", "Refugee Two", &location),
        );
        campaign.actors.insert(
            "relay-observer".into(),
            actor("relay-observer", "Relay Observer", &location),
        );
        crate::resolution::ensure_agency_profiles(&mut campaign);
        campaign
            .agency_profiles
            .get_mut(&campaign.player_actor_id)
            .unwrap()
            .simulation_eligible = false;
        campaign
    }

    #[test]
    fn stable_subject_history_survives_folding_and_remains_bounded() {
        let mut campaign = addressed_campaign();
        for revision in 1..=10 {
            campaign.transcript.push(NarrativeTurn {
                revision,
                at: Utc::now(),
                speaker: "refugee-one".into(),
                text: format!("response {revision}"),
                persona_response_actor_ids: BTreeSet::new(),
            });
        }
        campaign.transcript.push(NarrativeTurn {
            revision: 11,
            at: Utc::now(),
            speaker: "refugee-two".into(),
            text: "another person's answer".into(),
            persona_response_actor_ids: BTreeSet::new(),
        });

        let before_folding = recent_self_authored_turns(&campaign, "refugee-one");
        assert_eq!(before_folding.len(), 8);
        assert!(before_folding[0].contains("world revision 3"));
        assert!(before_folding[0].ends_with("response 3"));
        assert!(before_folding[7].ends_with("response 10"));
        assert!(
            before_folding
                .iter()
                .all(|turn| !turn.contains("another person's answer"))
        );

        campaign.actors.remove("refugee-one");
        assert_eq!(
            recent_self_authored_turns(&campaign, "refugee-one"),
            before_folding
        );
    }

    #[test]
    fn materialized_actor_receives_dormant_peer_identities_as_social_context() {
        let mut campaign = crate::resolution::tests::campaign(0, 8);
        let location = campaign.actors[&campaign.player_actor_id]
            .location_id
            .clone();
        campaign.actors.insert(
            "member:oxygen-patient".into(),
            actor("member:oxygen-patient", "Oxygen Patient", &location),
        );
        let member =
            |id: &str, name: &str, materialized_actor_id: Option<&str>| GestaltMemberDelta {
                schema: "ghostlight.gestalt_member_delta.v1".into(),
                id: id.into(),
                gestalt_id: "refugees".into(),
                version: 0,
                name: name.into(),
                capability_additions: BTreeSet::new(),
                capability_removals: BTreeSet::new(),
                knowledge_additions: BTreeSet::new(),
                knowledge_removals: BTreeSet::new(),
                equipment: BTreeSet::new(),
                conditions: BTreeSet::new(),
                obligations: BTreeSet::new(),
                relationships: BTreeMap::new(),
                goals: vec![],
                memories: vec![],
                last_location_id: Some(location.clone()),
                materialized_actor_id: materialized_actor_id.map(str::to_owned),
                last_relevant_revision: 0,
                relevance_lease_until_revision: 0,
            };
        campaign.gestalt_members.insert(
            "oxygen-patient".into(),
            member(
                "oxygen-patient",
                "Oxygen Patient",
                Some("member:oxygen-patient"),
            ),
        );
        campaign
            .gestalt_members
            .insert("taren".into(), member("taren", "Taren", None));

        let reserved =
            reserved_public_identities(&campaign, &campaign.actors["member:oxygen-patient"]);

        assert!(reserved.contains("Taren"));
        assert!(!reserved.contains("Oxygen Patient"));
    }

    #[tokio::test]
    async fn address_resolver_selects_exact_supplied_respondents() {
        let campaign = addressed_campaign();
        let (plan, receipts) = resolve_speech_addresses(
            Arc::new(AddressAwareModel),
            "flash",
            &campaign,
            &campaign.player_actor_id,
            "Refugee One and Refugee Two: answer separately. Relay Observer, listen.",
        )
        .await
        .unwrap();
        assert_eq!(
            plan.persona_response_actor_ids,
            BTreeSet::from(["refugee-one".into(), "refugee-two".into()])
        );
        assert_eq!(receipts.len(), 1);
    }

    #[tokio::test]
    async fn address_resolver_can_select_the_exact_nearby_folded_person() {
        let mut campaign = crate::resolution::tests::campaign(0, 8);
        let location = campaign.actors[&campaign.player_actor_id]
            .location_id
            .clone();
        campaign.gestalts.insert(
            "refugees".into(),
            GestaltPersonaState {
                schema: "ghostlight.gestalt_persona_state.v1".into(),
                id: "refugees".into(),
                name: "Refugees".into(),
                version: 3,
                home_location_id: location.clone(),
                shared_capabilities: BTreeSet::new(),
                shared_knowledge: BTreeSet::new(),
                resources: BTreeSet::new(),
                goals: vec![],
                pressures: vec![],
            },
        );
        campaign.gestalt_members.insert(
            "water-cart-taren".into(),
            GestaltMemberDelta {
                schema: "ghostlight.gestalt_member_delta.v1".into(),
                id: "water-cart-taren".into(),
                gestalt_id: "refugees".into(),
                version: 7,
                name: "Taren".into(),
                capability_additions: BTreeSet::new(),
                capability_removals: BTreeSet::new(),
                knowledge_additions: BTreeSet::new(),
                knowledge_removals: BTreeSet::new(),
                equipment: BTreeSet::from(["water handcart".into()]),
                conditions: BTreeSet::new(),
                obligations: BTreeSet::new(),
                relationships: BTreeMap::new(),
                goals: vec![],
                memories: vec![],
                last_location_id: Some(location),
                materialized_actor_id: None,
                last_relevant_revision: 0,
                relevance_lease_until_revision: 0,
            },
        );
        crate::resolution::ensure_agency_profiles(&mut campaign);

        let (plan, _) = resolve_speech_addresses(
            Arc::new(DormantAddressModel),
            "flash",
            &campaign,
            &campaign.player_actor_id,
            "Taren with the water handcart, is the coupling holding?",
        )
        .await
        .unwrap();

        assert_eq!(
            plan.persona_response_actor_ids,
            BTreeSet::from(["member:water-cart-taren".into()])
        );
    }

    #[tokio::test]
    async fn exact_addressees_receive_response_duty_while_observers_keep_agency() {
        let mut campaign = addressed_campaign();
        let speech = "Refugee One and Refugee Two: answer separately.";
        campaign.transcript.push(NarrativeTurn {
            revision: campaign.revision,
            at: Utc::now(),
            speaker: campaign.player_actor_id.clone(),
            text: speech.into(),
            persona_response_actor_ids: BTreeSet::from([
                "refugee-one".into(),
                "refugee-two".into(),
            ]),
        });
        let summary = format!("{} says: {speech}", campaign.player_actor_id);
        let engine = PersonaProjectionEngine {
            model: Arc::new(AddressAwareModel),
            permit: Arc::new(AllowAllPermit),
            projector_model: "flash".into(),
            persona_model: "pro".into(),
            interpreter_model: "flash".into(),
        };
        let wave = appraise_present(engine, &campaign, &summary).await.unwrap();
        assert_eq!(wave.reactions.len(), 3);
        for actor_id in ["refugee-one", "refugee-two"] {
            assert_eq!(
                wave.reactions
                    .iter()
                    .find(|reaction| reaction.actor_id == actor_id)
                    .and_then(|reaction| reaction.speech.as_deref()),
                Some("that memory is mine alone")
            );
        }
        assert!(
            wave.reactions
                .iter()
                .find(|reaction| reaction.actor_id == "relay-observer")
                .unwrap()
                .speech
                .is_none()
        );
    }

    #[tokio::test]
    async fn local_cohesive_gestalt_is_addressable_and_appraises_in_the_foreground_wave() {
        let mut campaign = crate::resolution::tests::campaign(0, 8);
        let location = campaign.actors[&campaign.player_actor_id]
            .location_id
            .clone();
        campaign.gestalts.insert(
            "settlement-households".into(),
            GestaltPersonaState {
                schema: "ghostlight.gestalt_persona_state.v1".into(),
                id: "settlement-households".into(),
                name: "Settlement households".into(),
                version: 0,
                home_location_id: location,
                shared_capabilities: BTreeSet::from(["collective refusal".into()]),
                shared_knowledge: BTreeSet::from(["five names lack a traceable path".into()]),
                resources: BTreeSet::new(),
                goals: vec!["keep every household legible".into()],
                pressures: vec!["the slate is closing".into()],
            },
        );
        crate::resolution::ensure_agency_profiles(&mut campaign);
        campaign
            .agency_profiles
            .get_mut(&campaign.player_actor_id)
            .unwrap()
            .simulation_eligible = false;
        let speech = "Settlement households, will you refuse this slate?";
        let (plan, _) = resolve_speech_addresses(
            Arc::new(CollectiveAddressModel),
            "flash",
            &campaign,
            &campaign.player_actor_id,
            speech,
        )
        .await
        .unwrap();
        assert_eq!(
            plan.persona_response_actor_ids,
            BTreeSet::from(["settlement-households".into()])
        );
        campaign.transcript.push(NarrativeTurn {
            revision: campaign.revision,
            at: Utc::now(),
            speaker: campaign.player_actor_id.clone(),
            text: speech.into(),
            persona_response_actor_ids: plan.persona_response_actor_ids,
        });
        let summary = format!("{} says: {speech}", campaign.player_actor_id);
        let wave = appraise_present(
            PersonaProjectionEngine {
                model: Arc::new(CollectiveAddressModel),
                permit: Arc::new(AllowAllPermit),
                projector_model: "flash".into(),
                persona_model: "pro".into(),
                interpreter_model: "flash".into(),
            },
            &campaign,
            &summary,
        )
        .await
        .unwrap();
        assert!(wave.reactions.is_empty());
        assert_eq!(wave.gestalt_reactions.len(), 1);
        assert_eq!(
            wave.gestalt_reactions[0].speech.as_deref(),
            Some("We will not accept a slate that makes our names disappear.")
        );
    }
}
