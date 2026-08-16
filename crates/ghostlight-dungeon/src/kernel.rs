use crate::d20::{capped_modifier, receipt};
use crate::domain::*;
use crate::persistence::CampaignStore;
use chrono::{Duration, Utc};
use rand::Rng;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
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
    if let WorldCommand::CreateCampaign { campaign } = command {
        store
            .insert(
                "campaign.v1",
                "ghostlight.campaign.v1",
                &campaign.id.to_string(),
                &campaign,
            )
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
        } => {
            require_revision(&campaign, expected_revision)?;
            let assessment = assess(&campaign, intent);
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
            commit(store, row, campaign, "speak", None)
        }
        WorldCommand::Wait {
            expected_revision,
            minutes,
        } => {
            require_revision(&campaign, expected_revision)?;
            campaign.world_time += Duration::minutes(i64::from(minutes));
            campaign.last_player_activity = Utc::now();
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
            campaign.pending_ticks = campaign.pending_ticks.saturating_sub(1);
            commit(store, row, campaign, "strategic_tick", None)
        }
        WorldCommand::CreateCampaign { .. } => unreachable!(),
    }
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
}
