use crate::{
    domain::{Campaign, GestaltPresencePlan},
    model::{ModelPort, ModelStageReceipt, ModelStageRequest, run_validated_stage},
};
use anyhow::{Result, anyhow};
use schemars::schema_for;
use std::{collections::BTreeSet, sync::Arc};

#[derive(Clone)]
pub struct GestaltPresencePlanner {
    pub model: Arc<dyn ModelPort>,
    pub model_name: String,
}

impl GestaltPresencePlanner {
    pub async fn plan(
        &self,
        campaign: &Campaign,
        event_summary: &str,
    ) -> Result<(GestaltPresencePlan, Vec<ModelStageReceipt>)> {
        let player_location = &campaign.actors[&campaign.player_actor_id].location_id;
        let nearby_gestalts = campaign
            .gestalts
            .values()
            .filter(|gestalt| {
                crate::resolution::validate_active_gestalt_presence_location(
                    campaign,
                    &gestalt.id,
                    player_location,
                )
                .is_ok()
            })
            .collect::<Vec<_>>();
        let nearby_gestalt_ids = nearby_gestalts
            .iter()
            .map(|gestalt| gestalt.id.clone())
            .collect::<BTreeSet<_>>();
        let player = &campaign.actors[&campaign.player_actor_id];
        let nearby_dormant_members = campaign
            .gestalt_members
            .values()
            .filter(|member| {
                nearby_gestalt_ids.contains(&member.gestalt_id)
                    && crate::resolution::dormant_member_location(campaign, &member.id)
                        .is_ok_and(|location| location == *player_location)
            })
            .map(|member| {
                serde_json::json!({
                    "member": member,
                    "player_relationship_to_member": player
                        .relationships
                        .get(&format!("member:{}", member.id)),
                })
            })
            .collect::<Vec<_>>();
        let nearby_dormant_member_ids = campaign
            .gestalt_members
            .values()
            .filter(|member| {
                nearby_gestalt_ids.contains(&member.gestalt_id)
                    && crate::resolution::dormant_member_location(campaign, &member.id)
                        .is_ok_and(|location| location == *player_location)
            })
            .map(|member| member.id.clone())
            .collect::<BTreeSet<_>>();
        let materialized_members = campaign
            .gestalt_members
            .values()
            .filter_map(|member| {
                let actor_id = member.materialized_actor_id.as_ref()?;
                let actor = campaign.actors.get(actor_id)?;
                Some(serde_json::json!({
                    "member_id":member.id,
                    "member_version":member.version,
                    "actor_id":actor_id,
                    "actor_location_id":actor.location_id,
                    "relevance_lease_until_revision":member.relevance_lease_until_revision,
                }))
            })
            .collect::<Vec<_>>();
        let materialized_actor_ids = campaign
            .gestalt_members
            .values()
            .filter_map(|member| member.materialized_actor_id.clone())
            .collect::<BTreeSet<_>>();
        let admitted_individuation_stimulus = automatic_individuation_stimulus(campaign);
        let individuation_admitted = admitted_individuation_stimulus
            .as_deref()
            .is_some_and(|stimulus| stimulus == event_summary);
        let candidates = serde_json::json!({
            "player_location_id": player_location,
            "nearby_active_leaf_gestalts": nearby_gestalts,
            "nearby_dormant_members": nearby_dormant_members,
            "materialized_members": materialized_members,
        });
        let mut schema = serde_json::to_value(schema_for!(GestaltPresencePlan))?;
        constrain_presence_schema(
            &mut schema,
            &nearby_gestalt_ids,
            &nearby_dormant_member_ids,
            &materialized_actor_ids,
            player_location,
            individuation_admitted,
        )?;
        let individuation_instruction = if individuation_admitted {
            "The immediately committed event is direct player speech. If that speech makes one anonymous population member individually relevant and no supplied member fits, you may individuate exactly one durable member delta from the gestalt baseline. Use a new stable lowercase id, version 0, the exact gestalt id/version, no materialized actor id, and record only personal departures from the shared baseline."
        } else {
            "The immediately committed event does not admit a new canonical person. The individuations array must be empty. Cast only supplied existing members."
        };
        let base_prompt = format!(
            "Cast reversible Persona population presence for the next scene after this event. The purpose is to make causal continuity visible without crowding the scene or inventing coincidence. Promote an existing member when their exact durable history makes them individually relevant. When a nearby dormant member has a reciprocal player relationship and an unresolved callback signal such as an obligation, memory, or goal, promote the single strongest earned callback unless the event makes their presence implausible or dramatically harmful. An ordinary shared-location event is enough opportunity for an earned callback; do not require the player to ask for that person. Prefer an existing person over anonymous individuation whenever their exact delta supports the scene. Return no promotion when there is no earned callback or the current event conflicts with it. {individuation_instruction} Demote a materialized member when they are no longer scene-relevant. Never place a promoted or individuated member outside the player location. Aggregate deltas must remain empty; population learning requires separate review. Emit the exact JSON schema.\nSCHEMA:\n{}\nCANDIDATES:\n{}\nEVENT:\n{}",
            serde_json::to_string_pretty(&schema)?,
            candidates,
            event_summary
        );
        let snapshot_binding = format!("campaign:{}:revision:{}", campaign.id, campaign.revision);
        let mut correction = String::new();
        let mut receipts = Vec::new();
        loop {
            let mut output = run_validated_stage(
                self.model.as_ref(),
                &ModelStageRequest {
                    stage: "gestalt_presence_planner".into(),
                    model: self.model_name.clone(),
                    snapshot_binding: snapshot_binding.clone(),
                    lived_stream: format!("{base_prompt}{correction}"),
                    output_schema: Some(schema.clone()),
                    source_receipt_ids: campaign.branch_origin.evidence_receipt_ids.clone(),
                    temperature: Some(0.0),
                    max_output_tokens: Some(1_500),
                },
            )
            .await?;
            let candidate = output
                .structured
                .clone()
                .ok_or_else(|| anyhow!("presence planner produced no plan"))
                .and_then(|value| serde_json::from_value(value).map_err(Into::into))
                .and_then(|plan| {
                    validate_plan(campaign, &plan, player_location, event_summary)?;
                    Ok(plan)
                });
            match candidate {
                Ok(plan) => {
                    receipts.push(output.receipt);
                    return Ok((plan, receipts));
                }
                Err(error) if receipts.is_empty() => {
                    output.receipt.validation_result = "semantic_invalid".into();
                    output.receipt.local_validation_error =
                        Some(error.to_string().chars().take(1_000).collect());
                    let rejected = output
                        .structured
                        .as_ref()
                        .and_then(|value| serde_json::to_string(value).ok())
                        .unwrap_or_else(|| "unavailable".into());
                    receipts.push(output.receipt);
                    correction = format!(
                        "\n\nLOCAL VALIDATOR REJECTED THE PREVIOUS PRESENCE PLAN: {error}\nPREVIOUS PLAN:\n{rejected}\nReturn one corrected complete plan against the same snapshot. Promote only exact nearby_dormant_members, demote only exact materialized_members, and copy every gestalt, member, actor, location, and version from the supplied candidates."
                    );
                }
                Err(error) => {
                    return Err(anyhow!(
                        "presence planner failed semantic validation after one correction: {error}"
                    ));
                }
            }
        }
    }
}

fn constrain_presence_schema(
    schema: &mut serde_json::Value,
    nearby_gestalt_ids: &BTreeSet<String>,
    dormant_member_ids: &BTreeSet<String>,
    materialized_actor_ids: &BTreeSet<String>,
    player_location: &str,
    individuation_admitted: bool,
) -> Result<()> {
    constrain_candidate_array(
        schema,
        "promotions",
        "GestaltPromotion",
        "member_id",
        dormant_member_ids,
    )?;
    constrain_candidate_array(
        schema,
        "demotions",
        "GestaltDemotion",
        "actor_id",
        materialized_actor_ids,
    )?;
    constrain_candidate_array(
        schema,
        "individuations",
        "GestaltIndividuation",
        "gestalt_id",
        nearby_gestalt_ids,
    )?;
    schema["properties"]["individuations"]["maxItems"] = serde_json::json!(
        if individuation_admitted && !nearby_gestalt_ids.is_empty() {
            1
        } else {
            0
        }
    );
    for definition in ["GestaltPromotion", "GestaltIndividuation"] {
        if !nearby_gestalt_ids.is_empty() {
            schema["$defs"][definition]["properties"]["gestalt_id"] =
                serde_json::json!({"type":"string","enum":nearby_gestalt_ids});
        }
        schema["$defs"][definition]["properties"]["location_id"] =
            serde_json::json!({"type":"string","enum":[player_location]});
    }
    if !nearby_gestalt_ids.is_empty() {
        schema["$defs"]["GestaltMemberDelta"]["properties"]["gestalt_id"] =
            serde_json::json!({"type":"string","enum":nearby_gestalt_ids});
    }
    Ok(())
}

fn constrain_candidate_array(
    schema: &mut serde_json::Value,
    field: &str,
    definition: &str,
    id_field: &str,
    allowed_ids: &BTreeSet<String>,
) -> Result<()> {
    let array = schema
        .pointer_mut(&format!("/properties/{field}"))
        .and_then(serde_json::Value::as_object_mut)
        .ok_or_else(|| anyhow!("presence schema omitted {field}"))?;
    if allowed_ids.is_empty() {
        array.insert("maxItems".into(), serde_json::json!(0));
    } else {
        array.insert("maxItems".into(), serde_json::json!(allowed_ids.len()));
        schema["$defs"][definition]["properties"][id_field] =
            serde_json::json!({"type":"string","enum":allowed_ids});
    }
    Ok(())
}

fn validate_plan(
    campaign: &Campaign,
    plan: &GestaltPresencePlan,
    player_location: &str,
    event_summary: &str,
) -> Result<()> {
    let individuation_admitted = automatic_individuation_stimulus(campaign)
        .as_deref()
        .is_some_and(|stimulus| stimulus == event_summary);
    if plan.individuations.len() > usize::from(individuation_admitted) {
        return Err(anyhow!(
            "automatic individuation requires the exact immediately committed player speech and admits at most one person"
        ));
    }
    let mut members = BTreeSet::new();
    for individuation in &plan.individuations {
        let member = &individuation.member;
        let gestalt = campaign
            .gestalts
            .get(&individuation.gestalt_id)
            .ok_or_else(|| anyhow!("presence plan invented a gestalt"))?;
        crate::resolution::validate_active_gestalt_presence_location(
            campaign,
            &individuation.gestalt_id,
            player_location,
        )?;
        if individuation.expected_gestalt_version != gestalt.version
            || individuation.location_id != player_location
            || member.gestalt_id != individuation.gestalt_id
            || member.version != 0
            || member.materialized_actor_id.is_some()
            || member.id.trim().is_empty()
            || member.name.trim().is_empty()
            || campaign.gestalt_members.contains_key(&member.id)
            || !members.insert(member.id.clone())
        {
            return Err(anyhow!(
                "presence individuation does not match its snapshot"
            ));
        }
    }
    for promotion in &plan.promotions {
        if !members.insert(promotion.member_id.clone()) {
            return Err(anyhow!("presence plan promotes one member twice"));
        }
        let member = campaign
            .gestalt_members
            .get(&promotion.member_id)
            .ok_or_else(|| anyhow!("presence plan invented a member"))?;
        let gestalt = campaign
            .gestalts
            .get(&promotion.gestalt_id)
            .ok_or_else(|| anyhow!("presence plan invented a gestalt"))?;
        let member_location = crate::resolution::dormant_member_location(campaign, &member.id)?;
        if member.gestalt_id != promotion.gestalt_id
            || member.version != promotion.expected_member_version
            || gestalt.version != promotion.expected_gestalt_version
            || member.materialized_actor_id.is_some()
            || promotion.location_id != player_location
            || member_location != promotion.location_id
        {
            return Err(anyhow!("presence promotion does not match its snapshot"));
        }
    }
    let materialized: BTreeSet<_> = campaign
        .gestalt_members
        .values()
        .filter_map(|member| member.materialized_actor_id.clone())
        .collect();
    let mut actors = BTreeSet::new();
    for demotion in &plan.demotions {
        if !actors.insert(demotion.actor_id.clone()) || !materialized.contains(&demotion.actor_id) {
            return Err(anyhow!(
                "presence plan demotes an unknown or duplicate member"
            ));
        }
        let member = campaign
            .gestalt_members
            .values()
            .find(|member| {
                member.materialized_actor_id.as_deref() == Some(demotion.actor_id.as_str())
            })
            .expect("materialized member was validated");
        let actor = &campaign.actors[&demotion.actor_id];
        if actor.location_id == player_location
            || member.relevance_lease_until_revision > campaign.revision
        {
            return Err(anyhow!(
                "presence plan demotes a visible or recently relevant member"
            ));
        }
    }
    Ok(())
}

pub(crate) fn automatic_individuation_stimulus(campaign: &Campaign) -> Option<String> {
    let turn = campaign.transcript.last()?;
    (turn.revision == campaign.revision && turn.speaker == campaign.player_actor_id)
        .then(|| format!("{} says: {}", turn.speaker, turn.text.trim()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        domain::{GestaltMemberDelta, GestaltPersonaState, Location, NarrativeTurn},
        model::ModelStageRequest,
    };
    use async_trait::async_trait;
    use std::{
        collections::{BTreeMap, BTreeSet},
        sync::{
            Mutex,
            atomic::{AtomicBool, AtomicUsize, Ordering},
        },
    };

    struct CapturePresenceModel {
        prompt: Arc<Mutex<String>>,
    }

    struct CorrectingPresenceModel {
        calls: AtomicUsize,
        saw_semantic_correction: AtomicBool,
    }

    #[async_trait]
    impl ModelPort for CapturePresenceModel {
        async fn run(&self, request: &ModelStageRequest) -> Result<String> {
            *self.prompt.lock().unwrap() = request.lived_stream.clone();
            Ok(serde_json::json!({
                "individuations":[],
                "promotions":[],
                "demotions":[]
            })
            .to_string())
        }

        fn provider(&self) -> &'static str {
            "presence-capture"
        }
    }

    #[async_trait]
    impl ModelPort for CorrectingPresenceModel {
        async fn run(&self, request: &ModelStageRequest) -> Result<String> {
            let call = self.calls.fetch_add(1, Ordering::SeqCst);
            if call == 0 {
                return Ok(serde_json::json!({
                    "individuations":[],
                    "promotions":[{
                        "gestalt_id":"nearby-leaf",
                        "expected_gestalt_version":0,
                        "member_id":"mira",
                        "expected_member_version":99,
                        "location_id":"center"
                    }],
                    "demotions":[]
                })
                .to_string());
            }
            self.saw_semantic_correction.store(
                request
                    .lived_stream
                    .contains("LOCAL VALIDATOR REJECTED THE PREVIOUS PRESENCE PLAN")
                    && request.lived_stream.contains("expected_member_version"),
                Ordering::SeqCst,
            );
            Ok(serde_json::json!({
                "individuations":[],
                "promotions":[],
                "demotions":[]
            })
            .to_string())
        }

        fn provider(&self) -> &'static str {
            "presence-correction"
        }
    }

    #[test]
    fn presence_schema_binds_each_transition_to_exact_candidate_ids() {
        let mut schema = serde_json::to_value(schema_for!(GestaltPresencePlan)).unwrap();
        constrain_presence_schema(
            &mut schema,
            &BTreeSet::from(["nearby-leaf".into()]),
            &BTreeSet::from(["mira".into()]),
            &BTreeSet::from(["member:already-here".into()]),
            "center",
            true,
        )
        .unwrap();

        assert_eq!(
            schema["$defs"]["GestaltPromotion"]["properties"]["member_id"]["enum"],
            serde_json::json!(["mira"])
        );
        assert_eq!(
            schema["$defs"]["GestaltDemotion"]["properties"]["actor_id"]["enum"],
            serde_json::json!(["member:already-here"])
        );
        assert_eq!(
            schema["$defs"]["GestaltIndividuation"]["properties"]["gestalt_id"]["enum"],
            serde_json::json!(["nearby-leaf"])
        );
        assert_eq!(
            schema["$defs"]["GestaltPromotion"]["properties"]["location_id"]["enum"],
            serde_json::json!(["center"])
        );
    }

    #[test]
    fn presence_schema_makes_empty_candidate_lanes_structurally_empty() {
        let mut schema = serde_json::to_value(schema_for!(GestaltPresencePlan)).unwrap();
        constrain_presence_schema(
            &mut schema,
            &BTreeSet::new(),
            &BTreeSet::new(),
            &BTreeSet::new(),
            "center",
            false,
        )
        .unwrap();

        for field in ["promotions", "demotions", "individuations"] {
            assert_eq!(schema["properties"][field]["maxItems"], 0);
        }
        jsonschema::validator_for(&schema).unwrap();
    }

    #[test]
    fn only_exact_immediately_committed_player_speech_admits_individuation() {
        let mut campaign = crate::resolution::tests::campaign(0, 8);
        campaign.revision = 7;
        campaign.transcript.push(NarrativeTurn {
            revision: 7,
            at: chrono::Utc::now(),
            speaker: campaign.player_actor_id.clone(),
            text: "I ask the unnamed porter for their name.".into(),
        });
        let exact = format!(
            "{} says: I ask the unnamed porter for their name.",
            campaign.player_actor_id
        );
        assert_eq!(
            automatic_individuation_stimulus(&campaign).as_deref(),
            Some(exact.as_str())
        );

        campaign.transcript.push(NarrativeTurn {
            revision: 8,
            at: chrono::Utc::now(),
            speaker: "world".into(),
            text: "They agree to help.".into(),
        });
        campaign.revision = 8;
        assert!(automatic_individuation_stimulus(&campaign).is_none());
    }

    #[tokio::test]
    async fn presence_planner_corrects_one_semantic_failure_against_the_same_snapshot() {
        let mut campaign = crate::resolution::tests::campaign(0, 8);
        campaign.gestalts.insert(
            "nearby-leaf".into(),
            GestaltPersonaState {
                schema: "ghostlight.gestalt_persona_state.v1".into(),
                id: "nearby-leaf".into(),
                name: "Nearby leaf".into(),
                version: 0,
                home_location_id: "center".into(),
                shared_capabilities: BTreeSet::new(),
                shared_knowledge: BTreeSet::new(),
                resources: BTreeSet::new(),
                goals: vec![],
                pressures: vec![],
            },
        );
        campaign.gestalt_members.insert(
            "mira".into(),
            GestaltMemberDelta {
                schema: "ghostlight.gestalt_member_delta.v1".into(),
                id: "mira".into(),
                gestalt_id: "nearby-leaf".into(),
                version: 0,
                name: "Mira".into(),
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
                last_location_id: Some("center".into()),
                materialized_actor_id: None,
                last_relevant_revision: 0,
                relevance_lease_until_revision: 0,
            },
        );
        crate::resolution::ensure_agency_profiles(&mut campaign);
        let model = Arc::new(CorrectingPresenceModel {
            calls: AtomicUsize::new(0),
            saw_semantic_correction: AtomicBool::new(false),
        });

        let (plan, receipts) = GestaltPresencePlanner {
            model: model.clone(),
            model_name: "fixture".into(),
        }
        .plan(&campaign, "The player enters the square.")
        .await
        .unwrap();

        assert!(plan.promotions.is_empty());
        assert_eq!(model.calls.load(Ordering::SeqCst), 2);
        assert!(model.saw_semantic_correction.load(Ordering::SeqCst));
        assert_eq!(receipts.len(), 2);
        assert_eq!(receipts[0].validation_result, "semantic_invalid");
        assert_eq!(receipts[0].snapshot_binding, receipts[1].snapshot_binding);
        assert!(receipts[1].local_validation_error.is_none());
    }

    #[tokio::test]
    async fn presence_projection_contains_only_nearby_active_leaves_and_relevant_members() {
        let mut campaign = crate::resolution::tests::campaign(0, 8);
        campaign.locations.insert(
            "far".into(),
            Location {
                id: "far".into(),
                name: "Far".into(),
                container_id: None,
                routes: BTreeMap::new(),
                persistent_features: vec![],
            },
        );
        let gestalt = |id: &str, name: &str, location: &str| GestaltPersonaState {
            schema: "ghostlight.gestalt_persona_state.v1".into(),
            id: id.into(),
            name: name.into(),
            version: 0,
            home_location_id: location.into(),
            shared_capabilities: BTreeSet::new(),
            shared_knowledge: BTreeSet::new(),
            resources: BTreeSet::new(),
            goals: vec![],
            pressures: vec![],
        };
        campaign.gestalts.insert(
            "nearby-leaf".into(),
            gestalt("nearby-leaf", "Nearby leaf", "center"),
        );
        campaign
            .gestalts
            .insert("far-leaf".into(), gestalt("far-leaf", "Far leaf", "far"));
        campaign.gestalts.insert(
            "inactive-parent".into(),
            gestalt("inactive-parent", "Inactive parent", "center"),
        );
        let member = |id: &str, name: &str, gestalt_id: &str, location: &str| GestaltMemberDelta {
            schema: "ghostlight.gestalt_member_delta.v1".into(),
            id: id.into(),
            gestalt_id: gestalt_id.into(),
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
            last_location_id: Some(location.into()),
            materialized_actor_id: None,
            last_relevant_revision: 0,
            relevance_lease_until_revision: 0,
        };
        campaign.gestalt_members.insert(
            "mira".into(),
            member("mira", "Mira Nearby", "nearby-leaf", "center"),
        );
        let player_id = campaign.player_actor_id.clone();
        campaign
            .actors
            .get_mut(&player_id)
            .unwrap()
            .relationships
            .insert("member:mira".into(), "helped her cross the flood".into());
        campaign
            .gestalt_members
            .get_mut("mira")
            .unwrap()
            .relationships
            .insert(player_id, "trusts them for helping".into());
        campaign
            .gestalt_members
            .get_mut("mira")
            .unwrap()
            .obligations
            .insert("thank the player if they meet again".into());
        campaign.gestalt_members.insert(
            "far-person".into(),
            member("far-person", "Far Person", "far-leaf", "far"),
        );
        crate::resolution::ensure_agency_profiles(&mut campaign);
        let parent = campaign.agency_profiles.get_mut("inactive-parent").unwrap();
        parent.active_leaf = false;
        parent.simulation_eligible = false;

        let prompt = Arc::new(Mutex::new(String::new()));
        GestaltPresencePlanner {
            model: Arc::new(CapturePresenceModel {
                prompt: prompt.clone(),
            }),
            model_name: "fixture".into(),
        }
        .plan(&campaign, "The player works in the square.")
        .await
        .unwrap();
        let prompt = prompt.lock().unwrap();
        assert!(prompt.contains("Nearby leaf"));
        assert!(prompt.contains("Mira Nearby"));
        assert!(prompt.contains("helped her cross the flood"));
        assert!(prompt.contains("thank the player if they meet again"));
        assert!(prompt.contains("do not require the player to ask for that person"));
        assert!(!prompt.contains("Far leaf"));
        assert!(!prompt.contains("Far Person"));
        assert!(!prompt.contains("Inactive parent"));
    }
}
