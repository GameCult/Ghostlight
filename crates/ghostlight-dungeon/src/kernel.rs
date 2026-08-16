use crate::d20::{capped_modifier, receipt};
use crate::domain::*;
use crate::persistence::CampaignStore;
use chrono::{Duration, Utc};
use rand::Rng;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use thiserror::Error;
use tokio::sync::{mpsc, oneshot};

#[derive(Debug, Error)]
pub enum KernelError {
    #[error("campaign not found")]
    NotFound,
    #[error("stale revision: expected {expected}, actual {actual}")]
    Stale { expected: u64, actual: u64 },
    #[error("action is impossible: {0}")]
    Impossible(String),
    #[error("assessment is stale or unknown")]
    StaleAssessment,
    #[error("invalid command: {0}")]
    Invalid(String),
    #[error("persistence failure: {0}")]
    Persistence(String),
}

#[derive(Clone, Debug, serde::Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CommandResult {
    Created {
        campaign: Campaign,
    },
    Assessed {
        assessment: ActionAssessment,
    },
    Committed {
        campaign: Campaign,
        receipt: WorldCommitReceipt,
    },
}

struct Request {
    command: WorldCommand,
    reply: oneshot::Sender<Result<CommandResult, KernelError>>,
}

#[derive(Clone)]
pub struct WorldKernel {
    tx: mpsc::Sender<Request>,
}

impl WorldKernel {
    pub fn start(store: CampaignStore) -> Self {
        let (tx, mut rx) = mpsc::channel::<Request>(64);
        tokio::spawn(async move {
            let mut assessments = BTreeMap::new();
            while let Some(request) = rx.recv().await {
                let result = execute(&store, &mut assessments, request.command);
                let _ = request.reply.send(result);
            }
        });
        Self { tx }
    }
    pub async fn command(&self, command: WorldCommand) -> Result<CommandResult, KernelError> {
        let (reply, receive) = oneshot::channel();
        self.tx
            .send(Request { command, reply })
            .await
            .map_err(|_| KernelError::Invalid("kernel stopped".into()))?;
        receive
            .await
            .map_err(|_| KernelError::Invalid("kernel stopped".into()))?
    }
}

fn execute(
    store: &CampaignStore,
    assessments: &mut BTreeMap<String, ActionAssessment>,
    command: WorldCommand,
) -> Result<CommandResult, KernelError> {
    if let WorldCommand::CreateCampaign {
        campaign,
        evidence_receipts,
    } = command
    {
        if !store.keys("campaign.v1").map_err(persist)?.is_empty() {
            return Err(KernelError::Invalid("campaign already exists".into()));
        }
        crate::compiler::validate_campaign_seed(&campaign)
            .map_err(|error| KernelError::Invalid(error.to_string()))?;
        store
            .create_campaign(&campaign, &evidence_receipts)
            .map_err(persist)?;
        return Ok(CommandResult::Created { campaign });
    }
    let campaign_id = single_campaign_id(store)?;
    let (row, mut campaign): (_, Campaign) = store
        .load("campaign.v1", &campaign_id)
        .map_err(persist)?
        .ok_or(KernelError::NotFound)?;
    match command {
        WorldCommand::Assess {
            expected_revision,
            intent,
            proposal,
        } => {
            require_revision(&campaign, expected_revision)?;
            let assessment = match proposal {
                Some(assessment) => {
                    if assessment.campaign_id != campaign.id
                        || assessment.revision != campaign.revision
                        || assessment.intent != intent
                        || assessment.expires_at < Utc::now()
                        || crate::assessor::assessment_digest(&assessment)
                            .map_err(|error| KernelError::Invalid(error.to_string()))?
                            != assessment.digest
                    {
                        return Err(KernelError::Invalid(
                            "assessment proposal is stale or invalid".into(),
                        ));
                    }
                    assessment
                }
                None => assess(&campaign, intent),
            };
            assessments.insert(assessment.digest.clone(), assessment.clone());
            Ok(CommandResult::Assessed { assessment })
        }
        WorldCommand::Attempt { assessment_digest } => {
            let assessment = assessments
                .remove(&assessment_digest)
                .ok_or(KernelError::StaleAssessment)?;
            if assessment.revision != campaign.revision || assessment.expires_at < Utc::now() {
                return Err(KernelError::StaleAssessment);
            }
            if !assessment.admissible {
                return Err(KernelError::Impossible(
                    assessment
                        .missing_permission
                        .unwrap_or_else(|| "not admissible".into()),
                ));
            }
            let roll = receipt(
                assessment.digest.clone(),
                rand::rng().random_range(1..=20),
                assessment.modifier_total,
                assessment.dc,
            );
            let text = match roll.outcome {
                OutcomeBand::StrongSuccess => &assessment.success_stake,
                OutcomeBand::Success => &assessment.success_stake,
                OutcomeBand::Mixed => &assessment.mixed_stake,
                OutcomeBand::Failure => &assessment.failure_stake,
            };
            campaign.last_player_activity = Utc::now();
            campaign.away_ticks_processed = 0;
            campaign.pending_ticks = 0;
            campaign.transcript.push(NarrativeTurn {
                revision: campaign.revision + 1,
                at: Utc::now(),
                speaker: "world".into(),
                text: text.clone(),
            });
            commit(store, row, campaign, "attempt", Some(roll))
        }
        WorldCommand::Speak {
            expected_revision,
            actor_id,
            text,
            intended_effect,
        } => {
            require_revision(&campaign, expected_revision)?;
            if !campaign.actors.contains_key(&actor_id) {
                return Err(KernelError::Invalid("unknown actor".into()));
            }
            campaign.transcript.push(NarrativeTurn {
                revision: campaign.revision + 1,
                at: Utc::now(),
                speaker: actor_id,
                text,
            });
            if let Some(effect) = intended_effect {
                campaign.transcript.push(NarrativeTurn {
                    revision: campaign.revision + 1,
                    at: Utc::now(),
                    speaker: "system".into(),
                    text: format!("Intended effect requires assessment: {effect}"),
                });
            }
            campaign.last_player_activity = Utc::now();
            campaign.away_ticks_processed = 0;
            campaign.pending_ticks = 0;
            commit(store, row, campaign, "speak", None)
        }
        WorldCommand::Wait {
            expected_revision,
            minutes,
        } => {
            require_revision(&campaign, expected_revision)?;
            campaign.world_time += Duration::minutes(i64::from(minutes));
            campaign.last_player_activity = Utc::now();
            campaign.away_ticks_processed = 0;
            campaign.pending_ticks = 0;
            commit(store, row, campaign, "wait", None)
        }
        WorldCommand::AdvanceStrategicTick {
            expected_revision,
            source: _,
        } => {
            require_revision(&campaign, expected_revision)?;
            campaign.world_time += Duration::hours(i64::from(campaign.tick_hours));
            for clock in campaign.clocks.values_mut() {
                clock.progress = clock.progress.saturating_add(1).min(clock.threshold);
            }
            let tick_number = campaign.away_ticks_processed.saturating_add(1);
            let mut tick_events = Vec::new();
            for institution in campaign.institutions.values_mut() {
                let summary = if let Some(goal) = institution.goals.first() {
                    format!("{} advances its interest: {}", institution.name, goal)
                } else {
                    format!("{} consolidates its current position", institution.name)
                };
                institution.posture = format!("acting after strategic tick {tick_number}");
                let event_id = format!("strategic:{}:{}", campaign.revision + 1, institution.id);
                tick_events.push(crate::domain::Event {
                    id: event_id,
                    at: campaign.world_time,
                    kind: "institution_action".into(),
                    summary,
                    actor_ids: vec![],
                    institution_ids: vec![institution.id.clone()],
                    location_ids: vec![],
                    public_channels: vec![format!("institution:{}", institution.id)],
                });
            }
            for gestalt in campaign.gestalts.values_mut() {
                gestalt.version += 1;
                let summary = if let Some(goal) = gestalt.goals.first() {
                    format!("{} collectively advances: {}", gestalt.name, goal)
                } else {
                    format!("{} carries on its shared routine", gestalt.name)
                };
                tick_events.push(crate::domain::Event {
                    id: format!("strategic:{}:gestalt:{}", campaign.revision + 1, gestalt.id),
                    at: campaign.world_time,
                    kind: "gestalt_action".into(),
                    summary,
                    actor_ids: vec![],
                    institution_ids: vec![],
                    location_ids: vec![gestalt.home_location_id.clone()],
                    public_channels: vec![format!("gestalt:{}", gestalt.id)],
                });
            }
            let player = campaign.actors.get(&campaign.player_actor_id);
            for event in &tick_events {
                let accessible = event.public_channels.iter().find(|channel| {
                    player.is_some_and(|actor| {
                        actor.knowledge.contains(*channel)
                            || actor.equipment.contains("communications")
                    })
                });
                if let Some(channel) = accessible {
                    campaign.news.push(crate::domain::NewsIssue {
                        id: format!("news:{}", event.id),
                        at: campaign.world_time,
                        channel: channel.clone(),
                        headline: event.summary.clone(),
                        event_ids: vec![event.id.clone()],
                        reliability: "direct institutional channel".into(),
                    });
                }
            }
            campaign.events.extend(tick_events);
            campaign.away_ticks_processed = tick_number.min(8);
            campaign.pending_ticks = campaign.pending_ticks.saturating_sub(1);
            commit(store, row, campaign, "strategic_tick", None)
        }
        WorldCommand::ExpandRegion {
            expected_revision,
            expansion,
            evidence_receipts,
            canon_candidates,
        } => {
            require_revision(&campaign, expected_revision)?;
            crate::compiler::validate_region_expansion(&campaign, &expansion)
                .map_err(|error| KernelError::Invalid(error.to_string()))?;
            for location in expansion.locations {
                campaign.locations.insert(location.id.clone(), location);
            }
            for fact in expansion.facts {
                campaign.facts.insert(fact.id.clone(), fact);
            }
            for candidate in &canon_candidates {
                campaign
                    .canon_candidates
                    .insert(candidate.id.clone(), candidate.clone());
            }
            commit_with_records(
                store,
                row,
                campaign,
                "expand_region",
                evidence_receipts,
                canon_candidates,
            )
        }
        WorldCommand::MaterializeGestaltMember {
            expected_revision,
            gestalt_id,
            expected_gestalt_version,
            member_id,
            expected_member_version,
            location_id,
        } => {
            require_revision(&campaign, expected_revision)?;
            if !campaign.locations.contains_key(&location_id) {
                return Err(KernelError::Invalid(
                    "materialization location is unknown".into(),
                ));
            }
            let gestalt = campaign
                .gestalts
                .get(&gestalt_id)
                .ok_or_else(|| KernelError::Invalid("gestalt is unknown".into()))?
                .clone();
            if gestalt.version != expected_gestalt_version {
                return Err(KernelError::Invalid("gestalt snapshot is stale".into()));
            }
            let member = campaign
                .gestalt_members
                .get_mut(&member_id)
                .ok_or_else(|| KernelError::Invalid("gestalt member is unknown".into()))?;
            if member.gestalt_id != gestalt_id || member.version != expected_member_version {
                return Err(KernelError::Invalid(
                    "member snapshot is stale or belongs to another gestalt".into(),
                ));
            }
            if member.materialized_actor_id.is_some() {
                return Err(KernelError::Invalid(
                    "gestalt member is already materialized".into(),
                ));
            }
            let actor_id = format!("member:{}", member.id);
            if campaign.actors.contains_key(&actor_id) {
                return Err(KernelError::Invalid(
                    "materialized actor id collides".into(),
                ));
            }
            let actor = materialize_actor(&gestalt, member, &actor_id, &location_id);
            member.materialized_actor_id = Some(actor_id.clone());
            member.last_location_id = Some(location_id);
            member.version += 1;
            campaign.actors.insert(actor_id, actor);
            commit(store, row, campaign, "materialize_gestalt_member", None)
        }
        WorldCommand::DematerializeGestaltMember {
            expected_revision,
            actor_id,
            aggregate_delta,
        } => {
            require_revision(&campaign, expected_revision)?;
            let actor = campaign
                .actors
                .get(&actor_id)
                .ok_or_else(|| KernelError::Invalid("materialized actor is unknown".into()))?
                .clone();
            let member_id = campaign
                .gestalt_members
                .values()
                .find(|member| member.materialized_actor_id.as_deref() == Some(actor_id.as_str()))
                .map(|member| member.id.clone())
                .ok_or_else(|| {
                    KernelError::Invalid("actor is not a materialized gestalt member".into())
                })?;
            let gestalt_id = campaign.gestalt_members[&member_id].gestalt_id.clone();
            let gestalt = campaign
                .gestalts
                .get_mut(&gestalt_id)
                .ok_or_else(|| KernelError::Invalid("gestalt is missing".into()))?;
            let member = campaign
                .gestalt_members
                .get_mut(&member_id)
                .expect("member exists");
            fold_actor_delta(&actor, gestalt, member);
            member.materialized_actor_id = None;
            member.version += 1;
            if !aggregate_delta.knowledge_additions.is_empty()
                || !aggregate_delta.resource_additions.is_empty()
                || !aggregate_delta.pressures.is_empty()
            {
                gestalt
                    .shared_knowledge
                    .extend(aggregate_delta.knowledge_additions);
                gestalt.resources.extend(aggregate_delta.resource_additions);
                gestalt.pressures.extend(aggregate_delta.pressures);
                gestalt.version += 1;
            }
            campaign.actors.remove(&actor_id);
            commit(store, row, campaign, "dematerialize_gestalt_member", None)
        }
        WorldCommand::ResolveReactionWave {
            expected_revision,
            event_summary,
            reactions,
        } => {
            require_revision(&campaign, expected_revision)?;
            let player_location = campaign.actors[&campaign.player_actor_id]
                .location_id
                .clone();
            let mut seen = BTreeSet::new();
            for reaction in &reactions {
                if !seen.insert(reaction.actor_id.clone()) {
                    return Err(KernelError::Invalid(
                        "actor reacted twice in one wave".into(),
                    ));
                }
                let actor = campaign
                    .actors
                    .get(&reaction.actor_id)
                    .ok_or_else(|| KernelError::Invalid("reaction actor is unknown".into()))?;
                if actor.location_id != player_location {
                    return Err(KernelError::Invalid("reaction actor is not present".into()));
                }
                for proposal in &reaction.action_proposals {
                    validate_world_proposal(actor, proposal)?;
                }
            }
            for reaction in reactions {
                let actor = campaign
                    .actors
                    .get_mut(&reaction.actor_id)
                    .expect("validated actor");
                actor.memories.extend(reaction.private_delta.memories_add);
                actor
                    .conditions
                    .extend(reaction.private_delta.conditions_add);
                for value in reaction.private_delta.conditions_remove {
                    actor.conditions.remove(&value);
                }
                actor.goals.extend(reaction.private_delta.goals_add);
                actor
                    .relationships
                    .extend(reaction.private_delta.relationship_updates);
                if let Some(speech) = reaction.speech {
                    campaign.transcript.push(NarrativeTurn {
                        revision: campaign.revision + 1,
                        at: Utc::now(),
                        speaker: reaction.actor_id.clone(),
                        text: speech,
                    });
                }
                campaign
                    .pending_world_proposals
                    .extend(reaction.action_proposals);
            }
            campaign.events.push(Event {
                id: format!("reaction-wave:{}", campaign.revision + 1),
                at: campaign.world_time,
                kind: "reaction_wave".into(),
                summary: event_summary,
                actor_ids: seen.into_iter().collect(),
                institution_ids: vec![],
                location_ids: vec![player_location],
                public_channels: vec![],
            });
            commit(store, row, campaign, "reaction_wave", None)
        }
        WorldCommand::CreateCampaign { .. } => unreachable!(),
    }
}

fn validate_world_proposal(
    actor: &ActorState,
    proposal: &WorldActionProposal,
) -> Result<(), KernelError> {
    if proposal.actor_id != actor.id
        || proposal.intent.trim().is_empty()
        || proposal.intended_effect.trim().is_empty()
    {
        return Err(KernelError::Invalid(
            "world action proposal is malformed".into(),
        ));
    }
    let allowed: BTreeSet<String> = actor
        .capabilities
        .iter()
        .map(|x| format!("capability:{x}"))
        .chain(actor.knowledge.iter().map(|x| format!("knowledge:{x}")))
        .chain(actor.equipment.iter().map(|x| format!("equipment:{x}")))
        .chain(std::iter::once(format!("location:{}", actor.location_id)))
        .collect();
    if proposal
        .state_references
        .iter()
        .any(|reference| !allowed.contains(reference))
    {
        return Err(KernelError::Invalid(
            "world action proposal cites unearned state".into(),
        ));
    }
    Ok(())
}

fn overlay(
    base: &BTreeSet<String>,
    add: &BTreeSet<String>,
    remove: &BTreeSet<String>,
) -> BTreeSet<String> {
    base.difference(remove)
        .cloned()
        .chain(add.iter().cloned())
        .collect()
}
fn materialize_actor(
    gestalt: &GestaltPersonaState,
    member: &GestaltMemberDelta,
    actor_id: &str,
    location_id: &str,
) -> ActorState {
    ActorState {
        id: actor_id.into(),
        name: member.name.clone(),
        location_id: location_id.into(),
        capabilities: overlay(
            &gestalt.shared_capabilities,
            &member.capability_additions,
            &member.capability_removals,
        ),
        knowledge: overlay(
            &gestalt.shared_knowledge,
            &member.knowledge_additions,
            &member.knowledge_removals,
        ),
        equipment: member.equipment.clone(),
        conditions: member.conditions.clone(),
        obligations: member.obligations.clone(),
        relationships: member.relationships.clone(),
        goals: if member.goals.is_empty() {
            gestalt.goals.clone()
        } else {
            member.goals.clone()
        },
        memories: member.memories.clone(),
    }
}
fn fold_actor_delta(
    actor: &ActorState,
    gestalt: &GestaltPersonaState,
    member: &mut GestaltMemberDelta,
) {
    member.capability_additions = actor
        .capabilities
        .difference(&gestalt.shared_capabilities)
        .cloned()
        .collect();
    member.capability_removals = gestalt
        .shared_capabilities
        .difference(&actor.capabilities)
        .cloned()
        .collect();
    member.knowledge_additions = actor
        .knowledge
        .difference(&gestalt.shared_knowledge)
        .cloned()
        .collect();
    member.knowledge_removals = gestalt
        .shared_knowledge
        .difference(&actor.knowledge)
        .cloned()
        .collect();
    member.equipment = actor.equipment.clone();
    member.conditions = actor.conditions.clone();
    member.obligations = actor.obligations.clone();
    member.relationships = actor.relationships.clone();
    member.goals = actor.goals.clone();
    member.memories = actor.memories.clone();
    member.last_location_id = Some(actor.location_id.clone());
}

fn single_campaign_id(store: &CampaignStore) -> Result<String, KernelError> {
    let keys = store.keys("campaign.v1").map_err(persist)?;
    match keys.as_slice() {
        [id] => Ok(id.clone()),
        [] => Err(KernelError::NotFound),
        _ => Err(KernelError::Invalid(
            "a campaign store must contain exactly one campaign".into(),
        )),
    }
}

fn require_revision(c: &Campaign, expected: u64) -> Result<(), KernelError> {
    if c.revision == expected {
        Ok(())
    } else {
        Err(KernelError::Stale {
            expected,
            actual: c.revision,
        })
    }
}

fn assess(c: &Campaign, intent: ActionIntent) -> ActionAssessment {
    let actor = c.actors.get(&intent.actor_id);
    let admissible = actor.is_some() && !intent.description.trim().is_empty();
    let missing = if actor.is_none() {
        Some("actor does not exist in this branch".into())
    } else if intent.description.trim().is_empty() {
        Some("no attempt was described".into())
    } else {
        None
    };
    let modifiers = vec![];
    let modifier_total = capped_modifier(modifiers.iter().map(|m: &ContextModifier| m.value));
    let mut a = ActionAssessment {
        schema: "ghostlight.player_action_assessment.v1".into(),
        campaign_id: c.id,
        revision: c.revision,
        intent,
        admissible,
        missing_permission: missing,
        dc: 15,
        modifiers,
        modifier_total,
        effect_ceiling:
            "A bounded local consequence; no unsupported world fact or custody transfer.".into(),
        success_stake: "The intended local effect succeeds and the world reacts.".into(),
        mixed_stake: "The effect lands with the previewed cost or complication.".into(),
        failure_stake: "Opposition holds and gains a concrete advantage.".into(),
        bargains: if admissible {
            vec![]
        } else {
            vec![
                "Narrow the effect, obtain access, recruit help, or accept a concrete sacrifice."
                    .into(),
            ]
        },
        expires_at: Utc::now() + Duration::minutes(10),
        digest: String::new(),
    };
    let bytes = rmp_serde::to_vec_named(&a).expect("assessment serializes");
    a.digest = format!("sha256:{:x}", Sha256::digest(bytes));
    a
}

fn commit(
    store: &CampaignStore,
    row: cultcache_rs::CultCacheEnvelope,
    mut campaign: Campaign,
    kind: &str,
    roll: Option<RollReceipt>,
) -> Result<CommandResult, KernelError> {
    let previous_revision = campaign.revision;
    campaign.revision += 1;
    let receipt = WorldCommitReceipt {
        schema: "ghostlight.world_commit_receipt.v1".into(),
        campaign_id: campaign.id,
        previous_revision,
        revision: campaign.revision,
        command_kind: kind.into(),
        committed_at: Utc::now(),
        roll,
    };
    store
        .append_with_replace(
            &row,
            "ghostlight.campaign.v1",
            &campaign,
            "world_commit_receipt.v1",
            "ghostlight.world_commit_receipt.v1",
            &format!("{}-{}", campaign.id, campaign.revision),
            &receipt,
        )
        .map_err(persist)?;
    Ok(CommandResult::Committed { campaign, receipt })
}

fn commit_with_records(
    store: &CampaignStore,
    row: cultcache_rs::CultCacheEnvelope,
    mut campaign: Campaign,
    kind: &str,
    evidence: Vec<VaultEvidenceReceipt>,
    candidates: Vec<CanonCandidate>,
) -> Result<CommandResult, KernelError> {
    let previous_revision = campaign.revision;
    campaign.revision += 1;
    let receipt = WorldCommitReceipt {
        schema: "ghostlight.world_commit_receipt.v1".into(),
        campaign_id: campaign.id,
        previous_revision,
        revision: campaign.revision,
        command_kind: kind.into(),
        committed_at: Utc::now(),
        roll: None,
    };
    store
        .append_world_commit(
            &row,
            &campaign,
            &format!("{}-{}", campaign.id, campaign.revision),
            &receipt,
            &evidence,
            &candidates,
        )
        .map_err(persist)?;
    Ok(CommandResult::Committed { campaign, receipt })
}

fn persist(e: anyhow::Error) -> KernelError {
    KernelError::Persistence(e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{BTreeMap, BTreeSet};

    fn campaign() -> Campaign {
        let id = uuid::Uuid::new_v4();
        let actor = ActorState {
            id: "player".into(),
            name: "Player".into(),
            location_id: "room".into(),
            capabilities: BTreeSet::new(),
            knowledge: BTreeSet::new(),
            equipment: BTreeSet::new(),
            conditions: BTreeSet::new(),
            obligations: BTreeSet::new(),
            relationships: BTreeMap::new(),
            goals: vec![],
            memories: vec![],
        };
        Campaign {
            schema: "ghostlight.campaign.v1".into(),
            id,
            name: "Test".into(),
            revision: 0,
            branch_origin: BranchOrigin {
                canon_cutoff: "test".into(),
                evidence_receipt_ids: vec![],
            },
            world_time: Utc::now(),
            tick_hours: 6,
            player_actor_id: "player".into(),
            locations: BTreeMap::from([(
                "room".into(),
                Location {
                    id: "room".into(),
                    name: "Room".into(),
                    container_id: None,
                    routes: BTreeMap::new(),
                    persistent_features: vec!["stable".into()],
                },
            )]),
            actors: BTreeMap::from([("player".into(), actor)]),
            institutions: BTreeMap::new(),
            clocks: BTreeMap::new(),
            facts: BTreeMap::new(),
            transcript: vec![],
            last_player_activity: Utc::now(),
            pending_ticks: 0,
            away_ticks_processed: 0,
            events: vec![],
            news: vec![],
            canon_candidates: BTreeMap::new(),
            gestalts: BTreeMap::new(),
            gestalt_members: BTreeMap::new(),
            pending_world_proposals: vec![],
        }
    }

    #[tokio::test]
    async fn stale_command_cannot_mutate_campaign() {
        let dir = tempfile::tempdir().unwrap();
        let store = CampaignStore::open(dir.path().join("campaign.cc")).unwrap();
        let kernel = WorldKernel::start(store.clone());
        let seed = campaign();
        kernel
            .command(WorldCommand::CreateCampaign {
                campaign: seed.clone(),
                evidence_receipts: vec![],
            })
            .await
            .unwrap();
        kernel
            .command(WorldCommand::Wait {
                expected_revision: 0,
                minutes: 30,
            })
            .await
            .unwrap();
        let stale = kernel
            .command(WorldCommand::Wait {
                expected_revision: 0,
                minutes: 30,
            })
            .await;
        assert!(matches!(stale, Err(KernelError::Stale { actual: 1, .. })));
        let (_, stored): (_, Campaign) = store
            .load("campaign.v1", &seed.id.to_string())
            .unwrap()
            .unwrap();
        assert_eq!(stored.revision, 1);
        assert_eq!(stored.world_time, seed.world_time + Duration::minutes(30));
    }

    #[tokio::test]
    async fn assessment_is_private_and_attempt_commits_roll_atomically() {
        let dir = tempfile::tempdir().unwrap();
        let store = CampaignStore::open(dir.path().join("campaign.cc")).unwrap();
        let kernel = WorldKernel::start(store.clone());
        let seed = campaign();
        kernel
            .command(WorldCommand::CreateCampaign {
                campaign: seed.clone(),
                evidence_receipts: vec![],
            })
            .await
            .unwrap();
        let result = kernel
            .command(WorldCommand::Assess {
                expected_revision: 0,
                intent: ActionIntent {
                    actor_id: "player".into(),
                    description: "Open the ordinary door".into(),
                    intended_effect: "Pass through".into(),
                },
                proposal: None,
            })
            .await
            .unwrap();
        let CommandResult::Assessed { assessment } = result else {
            panic!("expected assessment")
        };
        let (_, before): (_, Campaign) = store
            .load("campaign.v1", &seed.id.to_string())
            .unwrap()
            .unwrap();
        assert_eq!(before.revision, 0);
        let result = kernel
            .command(WorldCommand::Attempt {
                assessment_digest: assessment.digest,
            })
            .await
            .unwrap();
        let CommandResult::Committed { campaign, receipt } = result else {
            panic!("expected commit")
        };
        assert_eq!(campaign.revision, 1);
        assert!(receipt.roll.is_some());
        assert_eq!(store.keys("world_commit_receipt.v1").unwrap().len(), 1);
    }

    #[tokio::test]
    async fn strategic_tick_moves_institutions_but_news_respects_access() {
        let dir = tempfile::tempdir().unwrap();
        let store = CampaignStore::open(dir.path().join("campaign.cc")).unwrap();
        let kernel = WorldKernel::start(store.clone());
        let mut seed = campaign();
        seed.institutions.insert(
            "board".into(),
            InstitutionState {
                id: "board".into(),
                name: "Board".into(),
                resources: vec!["permits".into()],
                goals: vec!["contain the strike".into()],
                posture: "watching".into(),
            },
        );
        kernel
            .command(WorldCommand::CreateCampaign {
                campaign: seed.clone(),
                evidence_receipts: vec![],
            })
            .await
            .unwrap();
        let result = kernel
            .command(WorldCommand::AdvanceStrategicTick {
                expected_revision: 0,
                source: TickSource::Scheduler,
            })
            .await
            .unwrap();
        let CommandResult::Committed { campaign, .. } = result else {
            panic!("expected commit")
        };
        assert_eq!(campaign.events.len(), 1);
        assert!(campaign.news.is_empty());
        assert_eq!(campaign.away_ticks_processed, 1);
        assert!(campaign.institutions["board"].posture.contains("acting"));
    }

    #[tokio::test]
    async fn region_expansion_commits_topology_evidence_and_candidate_together() {
        let dir = tempfile::tempdir().unwrap();
        let store = CampaignStore::open(dir.path().join("campaign.cc")).unwrap();
        let kernel = WorldKernel::start(store.clone());
        let seed = campaign();
        kernel
            .command(WorldCommand::CreateCampaign {
                campaign: seed.clone(),
                evidence_receipts: vec![],
            })
            .await
            .unwrap();
        let evidence = VaultEvidenceReceipt {
            schema: "ghostlight.vault_evidence_receipt.v1".into(),
            id: "vault:route".into(),
            provider: "fixture".into(),
            query_hash: "sha256:q".into(),
            witnesses: vec![],
            retrieved_at: Utc::now(),
        };
        let candidate = CanonCandidate {
            schema: "ghostlight.canon_candidate.v1".into(),
            id: "candidate:gate".into(),
            originating_campaign_id: seed.id,
            gap: "Who owns the gate?".into(),
            evidence_receipt_ids: vec![evidence.id.clone()],
            conflicts: vec![],
            proposed_wording: "Clarify gate ownership".into(),
            affected_vault_sources: vec![],
            status: "review".into(),
        };
        let location = Location {
            id: "annex".into(),
            name: "Annex".into(),
            container_id: None,
            routes: BTreeMap::from([(
                "back".into(),
                Route {
                    destination_id: "room".into(),
                    distance: "near".into(),
                    travel_minutes: 10,
                },
            )]),
            persistent_features: vec!["stable annex".into()],
        };
        let result = kernel
            .command(WorldCommand::ExpandRegion {
                expected_revision: 0,
                expansion: RegionExpansion {
                    origin_location_id: "room".into(),
                    locations: vec![location],
                    facts: vec![],
                },
                evidence_receipts: vec![evidence],
                canon_candidates: vec![candidate],
            })
            .await
            .unwrap();
        let CommandResult::Committed { campaign, .. } = result else {
            panic!("expected commit")
        };
        assert!(campaign.locations.contains_key("annex"));
        assert_eq!(campaign.revision, 1);
        assert_eq!(store.keys("vault_evidence_receipt.v1").unwrap().len(), 1);
        assert_eq!(store.keys("canon_candidate.v1").unwrap().len(), 1);
    }

    #[tokio::test]
    async fn gestalt_member_dematerializes_to_delta_and_returns_as_same_person() {
        let dir = tempfile::tempdir().unwrap();
        let store = CampaignStore::open(dir.path().join("campaign.cc")).unwrap();
        let kernel = WorldKernel::start(store);
        let mut seed = campaign();
        seed.gestalts.insert(
            "village".into(),
            GestaltPersonaState {
                schema: "ghostlight.gestalt_persona_state.v1".into(),
                id: "village".into(),
                name: "The villagers".into(),
                version: 0,
                home_location_id: "room".into(),
                shared_capabilities: BTreeSet::from(["basic smithing".into()]),
                shared_knowledge: BTreeSet::from(["village roads".into()]),
                resources: BTreeSet::new(),
                goals: vec!["keep the village fed".into()],
                pressures: vec![],
            },
        );
        seed.gestalt_members.insert(
            "john".into(),
            GestaltMemberDelta {
                schema: "ghostlight.gestalt_member_delta.v1".into(),
                id: "john".into(),
                gestalt_id: "village".into(),
                version: 0,
                name: "John".into(),
                capability_additions: BTreeSet::from(["master blacksmith".into()]),
                capability_removals: BTreeSet::new(),
                knowledge_additions: BTreeSet::new(),
                knowledge_removals: BTreeSet::new(),
                equipment: BTreeSet::from(["John's hammer".into()]),
                conditions: BTreeSet::new(),
                obligations: BTreeSet::new(),
                relationships: BTreeMap::from([("player".into(), "new acquaintance".into())]),
                goals: vec![],
                memories: vec!["met the player".into()],
                last_location_id: None,
                materialized_actor_id: None,
            },
        );
        kernel
            .command(WorldCommand::CreateCampaign {
                campaign: seed,
                evidence_receipts: vec![],
            })
            .await
            .unwrap();
        let first = kernel
            .command(WorldCommand::MaterializeGestaltMember {
                expected_revision: 0,
                gestalt_id: "village".into(),
                expected_gestalt_version: 0,
                member_id: "john".into(),
                expected_member_version: 0,
                location_id: "room".into(),
            })
            .await
            .unwrap();
        let CommandResult::Committed {
            campaign: first, ..
        } = first
        else {
            panic!()
        };
        assert!(
            first.actors["member:john"]
                .capabilities
                .contains("master blacksmith")
        );
        let folded = kernel
            .command(WorldCommand::DematerializeGestaltMember {
                expected_revision: 1,
                actor_id: "member:john".into(),
                aggregate_delta: GestaltAggregateDelta {
                    knowledge_additions: BTreeSet::from(["the player keeps promises".into()]),
                    ..Default::default()
                },
            })
            .await
            .unwrap();
        let CommandResult::Committed {
            campaign: folded, ..
        } = folded
        else {
            panic!()
        };
        assert!(!folded.actors.contains_key("member:john"));
        assert_eq!(
            folded.gestalt_members["john"].memories,
            vec!["met the player"]
        );
        let again = kernel
            .command(WorldCommand::MaterializeGestaltMember {
                expected_revision: 2,
                gestalt_id: "village".into(),
                expected_gestalt_version: 1,
                member_id: "john".into(),
                expected_member_version: 2,
                location_id: "room".into(),
            })
            .await
            .unwrap();
        let CommandResult::Committed {
            campaign: again, ..
        } = again
        else {
            panic!()
        };
        assert_eq!(again.actors["member:john"].name, "John");
        assert_eq!(
            again.actors["member:john"].relationships["player"],
            "new acquaintance"
        );
        assert!(
            again.gestalts["village"]
                .shared_knowledge
                .contains("the player keeps promises")
        );
    }

    #[tokio::test]
    async fn invalid_reaction_rejects_the_entire_wave_without_mutation() {
        let dir = tempfile::tempdir().unwrap();
        let store = CampaignStore::open(dir.path().join("campaign.cc")).unwrap();
        let kernel = WorldKernel::start(store.clone());
        let mut seed = campaign();
        for id in ["anna", "bert"] {
            seed.actors.insert(
                id.into(),
                ActorState {
                    id: id.into(),
                    name: id.into(),
                    location_id: "room".into(),
                    capabilities: BTreeSet::from(["observe".into()]),
                    knowledge: BTreeSet::new(),
                    equipment: BTreeSet::new(),
                    conditions: BTreeSet::new(),
                    obligations: BTreeSet::new(),
                    relationships: BTreeMap::new(),
                    goals: vec![],
                    memories: vec![],
                },
            );
        }
        kernel
            .command(WorldCommand::CreateCampaign {
                campaign: seed.clone(),
                evidence_receipts: vec![],
            })
            .await
            .unwrap();
        let reactions = vec![
            ActorReaction {
                actor_id: "anna".into(),
                speech: Some("I saw that.".into()),
                private_delta: ActorStateDelta {
                    memories_add: vec!["saw the event".into()],
                    ..Default::default()
                },
                action_proposals: vec![],
            },
            ActorReaction {
                actor_id: "bert".into(),
                speech: None,
                private_delta: ActorStateDelta::default(),
                action_proposals: vec![WorldActionProposal {
                    actor_id: "bert".into(),
                    intent: "invoke secret lore".into(),
                    intended_effect: "control the room".into(),
                    priority: 10,
                    state_references: vec!["knowledge:unearned secret lore".into()],
                }],
            },
        ];
        assert!(
            kernel
                .command(WorldCommand::ResolveReactionWave {
                    expected_revision: 0,
                    event_summary: "the player acts".into(),
                    reactions,
                })
                .await
                .is_err()
        );
        let persisted = store
            .load::<Campaign>("campaign.v1", &seed.id.to_string())
            .unwrap()
            .unwrap()
            .1;
        assert_eq!(persisted.revision, 0);
        assert!(persisted.actors["anna"].memories.is_empty());
        assert!(persisted.transcript.is_empty());
        assert!(persisted.pending_world_proposals.is_empty());
    }
}
