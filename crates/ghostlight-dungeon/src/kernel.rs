use crate::d20::{capped_modifier, receipt};
use crate::domain::*;
use crate::persistence::CampaignStore;
use crate::session_zero::{
    CampaignMembership, CellBudgetProposal, GroupTravelProposal, TimeAdvanceProposal,
};
use chrono::{Duration, Utc};
use rand::Rng;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use thiserror::Error;
use tokio::sync::{mpsc, oneshot};

const GESTALT_RELEVANCE_LEASE_REVISIONS: u64 = 2;

#[derive(Debug, Error)]
pub enum KernelError {
    #[error("campaign not found")]
    NotFound,
    #[error("stale revision: expected {expected}, actual {actual}")]
    Stale { expected: u64, actual: u64 },
    #[error("action is impossible: {0}")]
    Impossible(String),
    #[error("assessment is unknown")]
    UnknownAssessment,
    #[error("assessment is stale at campaign revision {actual_revision}")]
    StaleAssessment {
        intent: ActionIntent,
        actual_revision: u64,
    },
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
    ResolutionUpdated {
        campaign: Campaign,
        receipt: ResolutionControlReceipt,
    },
    GovernancePending {
        campaign: Campaign,
        proposal: TimeAdvanceProposal,
    },
    TravelGovernancePending {
        campaign: Campaign,
        proposal: GroupTravelProposal,
    },
    ResolutionGovernancePending {
        campaign: Campaign,
        proposal: CellBudgetProposal,
    },
    MutationCommitted {
        state: crate::transition::ComponentWorldState,
        receipt: crate::transition::WorldMutationReceipt,
    },
}

enum KernelInput {
    World(WorldCommand),
    Mutation {
        authority: crate::transition::MutationAuthorityEnvelope,
        batch: crate::transition::WorldMutationBatch,
    },
}

struct Request {
    input: KernelInput,
    reply: oneshot::Sender<Result<CommandResult, KernelError>>,
}

#[derive(Clone)]
pub struct WorldKernel {
    tx: mpsc::Sender<Request>,
}

impl WorldKernel {
    pub fn initialize_campaign(
        store: &CampaignStore,
        command: WorldCommand,
    ) -> Result<CommandResult, KernelError> {
        if !matches!(command, WorldCommand::CreateCampaign { .. }) {
            return Err(KernelError::Invalid(
                "campaign initialization accepts only CreateCampaign".into(),
            ));
        }
        execute(store, &mut BTreeMap::new(), command)
    }

    pub fn start(store: CampaignStore) -> Self {
        let (tx, mut rx) = mpsc::channel::<Request>(64);
        tokio::spawn(async move {
            let mut assessments = BTreeMap::new();
            while let Some(request) = rx.recv().await {
                let result = match request.input {
                    KernelInput::World(command) => execute(&store, &mut assessments, command),
                    KernelInput::Mutation { authority, batch } => {
                        execute_mutation_batch(&store, authority, batch)
                    }
                };
                let _ = request.reply.send(result);
            }
        });
        Self { tx }
    }
    pub async fn command(&self, command: WorldCommand) -> Result<CommandResult, KernelError> {
        let (reply, receive) = oneshot::channel();
        self.tx
            .send(Request {
                input: KernelInput::World(command),
                reply,
            })
            .await
            .map_err(|_| KernelError::Invalid("kernel stopped".into()))?;
        receive
            .await
            .map_err(|_| KernelError::Invalid("kernel stopped".into()))?
    }

    pub async fn commit_mutation_batch(
        &self,
        authority: crate::transition::MutationAuthorityEnvelope,
        batch: crate::transition::WorldMutationBatch,
    ) -> Result<CommandResult, KernelError> {
        let (reply, receive) = oneshot::channel();
        self.tx
            .send(Request {
                input: KernelInput::Mutation { authority, batch },
                reply,
            })
            .await
            .map_err(|_| KernelError::Invalid("kernel stopped".into()))?;
        receive
            .await
            .map_err(|_| KernelError::Invalid("kernel stopped".into()))?
    }
}

fn execute_mutation_batch(
    store: &CampaignStore,
    authority: crate::transition::MutationAuthorityEnvelope,
    batch: crate::transition::WorldMutationBatch,
) -> Result<CommandResult, KernelError> {
    let key = batch.campaign_id.to_string();
    if store
        .load::<Campaign>("campaign.v1", &key)
        .map_err(persist)?
        .is_some()
    {
        return Err(KernelError::Invalid(
            "aggregate campaigns accept mutations only through their WorldCommand mailbox".into(),
        ));
    }
    let (row, state) = store
        .load::<crate::transition::ComponentWorldState>("component_world_state.v1", &key)
        .map_err(persist)?
        .ok_or(KernelError::NotFound)?;
    let application =
        crate::transition::apply_component_world_batch(&state, &authority, &batch, Utc::now())
            .map_err(|error| KernelError::Invalid(error.to_string()))?;
    store
        .commit_world_mutation_batch(
            &row,
            &application.state,
            &authority,
            &batch,
            &application.receipt,
        )
        .map_err(persist)?;
    Ok(CommandResult::MutationCommitted {
        state: application.state,
        receipt: application.receipt,
    })
}

fn execute(
    store: &CampaignStore,
    assessments: &mut BTreeMap<String, ActionAssessment>,
    command: WorldCommand,
) -> Result<CommandResult, KernelError> {
    if let WorldCommand::CreateCampaign {
        mut campaign,
        evidence_receipts,
        model_stage_receipts,
    } = command
    {
        if !store.keys("campaign.v1").map_err(persist)?.is_empty() {
            return Err(KernelError::Invalid("campaign already exists".into()));
        }
        crate::resolution::ensure_agency_profiles(&mut campaign);
        crate::compiler::validate_campaign_seed(&campaign)
            .map_err(|error| KernelError::Invalid(error.to_string()))?;
        store
            .create_campaign(&campaign, &evidence_receipts, &model_stage_receipts)
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
            validate_intent_text(&intent)?;
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
                    let actor = campaign
                        .actors
                        .get(&assessment.intent.actor_id)
                        .ok_or_else(|| KernelError::Invalid("assessment actor vanished".into()))?;
                    for (effect, stake) in [
                        (&assessment.strong_effect, &assessment.success_stake),
                        (&assessment.success_effect, &assessment.success_stake),
                        (&assessment.mixed_effect, &assessment.mixed_stake),
                        (&assessment.failure_effect, &assessment.failure_stake),
                    ] {
                        crate::assessor::validate_effect(&campaign, actor, effect, stake)
                            .map_err(|error| KernelError::Invalid(error.to_string()))?;
                        validate_bounded_coop_effect(
                            store,
                            &campaign,
                            &assessment.intent.actor_id,
                            effect,
                        )?;
                    }
                    assessment
                }
                None => assess(&campaign, intent),
            };
            assessments.insert(assessment.digest.clone(), assessment.clone());
            Ok(CommandResult::Assessed { assessment })
        }
        WorldCommand::Attempt {
            actor_id,
            assessment_digest,
        } => {
            let assessment = assessments
                .get(&assessment_digest)
                .cloned()
                .ok_or(KernelError::UnknownAssessment)?;
            if assessment.intent.actor_id != actor_id {
                return Err(KernelError::Invalid(
                    "assessment belongs to another actor".into(),
                ));
            }
            if assessment.revision != campaign.revision || assessment.expires_at < Utc::now() {
                assessments.remove(&assessment_digest);
                return Err(KernelError::StaleAssessment {
                    intent: assessment.intent,
                    actual_revision: campaign.revision,
                });
            }
            assessments.remove(&assessment_digest);
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
            let effect = match roll.outcome {
                OutcomeBand::StrongSuccess => assessment.strong_effect.clone(),
                OutcomeBand::Success => assessment.success_effect.clone(),
                OutcomeBand::Mixed => assessment.mixed_effect.clone(),
                OutcomeBand::Failure => assessment.failure_effect.clone(),
            };
            let transition = crate::legacy_transition::lower_foreground_effect(
                &campaign,
                &assessment.intent.actor_id,
                &effect,
                roll.outcome.clone(),
                crate::transition::MutationProcedure::ForegroundAttempt,
                &assessment.effect_ceiling,
                &assessment.digest,
                Some(
                    crate::legacy_transition::digest_serializable(&assessment.intent)
                        .map_err(|error| KernelError::Invalid(error.to_string()))?,
                ),
                Some(
                    crate::legacy_transition::digest_serializable(
                        &assessment.intent.intended_effect,
                    )
                    .map_err(|error| KernelError::Invalid(error.to_string()))?,
                ),
                assessment.expires_at,
            )
            .map_err(|error| KernelError::Invalid(error.to_string()))?;
            let mutation_receipt = crate::legacy_transition::apply_lowered_transition(
                &mut campaign,
                &transition,
                Utc::now(),
            )
            .map_err(|error| KernelError::Invalid(error.to_string()))?;
            refresh_materialized_member_relevance(
                &mut campaign,
                std::iter::once(assessment.intent.actor_id.as_str()),
            );
            if is_human_controlled_actor(&campaign, &assessment.intent.actor_id) {
                campaign.last_player_activity = Utc::now();
                campaign.away_ticks_processed = 0;
                campaign.pending_ticks = 0;
            }
            campaign.transcript.push(NarrativeTurn {
                revision: campaign.revision + 1,
                at: Utc::now(),
                speaker: "world".into(),
                text: text.clone(),
                persona_response_actor_ids: BTreeSet::new(),
            });
            commit_mutation_transition(
                store,
                row,
                campaign,
                "attempt",
                Some(roll),
                transition,
                mutation_receipt,
            )
        }
        WorldCommand::Speak {
            expected_revision,
            actor_id,
            text,
            intended_effect,
            persona_response_actor_ids,
        } => {
            require_revision(&campaign, expected_revision)?;
            validate_bounded_text("speech", &text, 4_000)?;
            if let Some(effect) = &intended_effect {
                validate_bounded_text("intended effect", effect, 1_000)?;
            }
            let speaker = campaign
                .actors
                .get(&actor_id)
                .ok_or_else(|| KernelError::Invalid("unknown actor".into()))?;
            for response_actor_id in &persona_response_actor_ids {
                let response_actor = campaign.actors.get(response_actor_id).ok_or_else(|| {
                    KernelError::Invalid("speech response actor is unknown".into())
                })?;
                if response_actor_id == &actor_id
                    || response_actor.location_id != speaker.location_id
                    || campaign
                        .agency_profiles
                        .get(response_actor_id)
                        .is_some_and(|profile| !profile.simulation_eligible)
                {
                    return Err(KernelError::Invalid(
                        "speech response actor is not an eligible present Persona".into(),
                    ));
                }
            }
            let response_actor_ids = persona_response_actor_ids.clone();
            campaign.transcript.push(NarrativeTurn {
                revision: campaign.revision + 1,
                at: Utc::now(),
                speaker: actor_id.clone(),
                text,
                persona_response_actor_ids,
            });
            if let Some(effect) = intended_effect {
                campaign.transcript.push(NarrativeTurn {
                    revision: campaign.revision + 1,
                    at: Utc::now(),
                    speaker: "system".into(),
                    text: format!("Intended effect requires assessment: {effect}"),
                    persona_response_actor_ids: BTreeSet::new(),
                });
            }
            refresh_materialized_member_relevance(
                &mut campaign,
                std::iter::once(actor_id.as_str())
                    .chain(response_actor_ids.iter().map(String::as_str)),
            );
            if is_human_controlled_actor(&campaign, &actor_id) {
                campaign.last_player_activity = Utc::now();
                campaign.away_ticks_processed = 0;
                campaign.pending_ticks = 0;
            }
            commit(store, row, campaign, "speak", None)
        }
        WorldCommand::Wait {
            expected_revision,
            minutes,
        } => {
            require_revision(&campaign, expected_revision)?;
            if minutes == 0 || minutes > 1_440 {
                return Err(KernelError::Invalid(
                    "wait duration must be between 1 and 1440 minutes".into(),
                ));
            }
            let source_receipt_id = format!(
                "direct-wait:{}:{}:{}",
                campaign.id, campaign.revision, minutes
            );
            let transition = crate::legacy_transition::lower_time_advance(
                &campaign,
                minutes,
                crate::transition::MutationProcedure::DirectCommand,
                &source_receipt_id,
                Utc::now() + Duration::minutes(5),
            )
            .map_err(|error| KernelError::Invalid(error.to_string()))?;
            let mutation_receipt = crate::legacy_transition::apply_lowered_transition(
                &mut campaign,
                &transition,
                Utc::now(),
            )
            .map_err(|error| KernelError::Invalid(error.to_string()))?;
            campaign.last_player_activity = Utc::now();
            campaign.away_ticks_processed = 0;
            campaign.pending_ticks = 0;
            commit_mutation_transition(
                store,
                row,
                campaign,
                "wait",
                None,
                transition,
                mutation_receipt,
            )
        }
        WorldCommand::ProposeTimeAdvance {
            expected_revision,
            member_id,
            minutes,
        } => {
            require_revision(&campaign, expected_revision)?;
            if minutes == 0 || minutes > 1_440 {
                return Err(KernelError::Invalid(
                    "time advance must be between 1 and 1440 minutes".into(),
                ));
            }
            let membership = load_campaign_membership(store, campaign.id)?;
            require_active_member(&membership, &member_id)?;
            if store
                .load_all::<TimeAdvanceProposal>("time_advance_proposal.v1")
                .map_err(persist)?
                .into_iter()
                .any(|proposal| {
                    !proposal.committed && proposal.expected_world_revision == campaign.revision
                })
            {
                return Err(KernelError::Invalid(
                    "a time-advance proposal is already pending at this revision".into(),
                ));
            }
            let proposal_id = format!("time:{}", uuid::Uuid::new_v4().simple());
            let proposal = TimeAdvanceProposal {
                schema: "ghostlight.time_advance_proposal.v1".into(),
                id: proposal_id.clone(),
                campaign_id: campaign.id,
                expected_world_revision: campaign.revision,
                minutes,
                proposer_member_id: member_id.clone(),
                approvals: BTreeSet::from([member_id]),
                committed: false,
            };
            let proposal_row = store
                .insert(
                    "time_advance_proposal.v1",
                    "ghostlight.time_advance_proposal.v1",
                    &proposal_id,
                    &proposal,
                )
                .map_err(persist)?;
            if active_member_ids(&membership) == proposal.approvals {
                commit_governed_time_advance(store, row, campaign, proposal_row, proposal)
            } else {
                Ok(CommandResult::GovernancePending { campaign, proposal })
            }
        }
        WorldCommand::ApproveTimeAdvance {
            expected_revision,
            proposal_id,
            member_id,
        } => {
            require_revision(&campaign, expected_revision)?;
            let membership = load_campaign_membership(store, campaign.id)?;
            require_active_member(&membership, &member_id)?;
            let (proposal_row, mut proposal) = store
                .load::<TimeAdvanceProposal>("time_advance_proposal.v1", &proposal_id)
                .map_err(persist)?
                .ok_or_else(|| KernelError::Invalid("time proposal does not exist".into()))?;
            if proposal.committed
                || proposal.campaign_id != campaign.id
                || proposal.expected_world_revision != campaign.revision
            {
                return Err(KernelError::Invalid(
                    "time proposal is committed, stale, or belongs to another campaign".into(),
                ));
            }
            let newly_approved = proposal.approvals.insert(member_id);
            if active_member_ids(&membership) == proposal.approvals {
                commit_governed_time_advance(store, row, campaign, proposal_row, proposal)
            } else if !newly_approved {
                Err(KernelError::Invalid(
                    "member already approved this time proposal".into(),
                ))
            } else {
                store
                    .replace(
                        &proposal_row,
                        "ghostlight.time_advance_proposal.v1",
                        &proposal,
                    )
                    .map_err(persist)?;
                Ok(CommandResult::GovernancePending { campaign, proposal })
            }
        }
        WorldCommand::ProposeGroupTravel {
            expected_revision,
            member_id,
            destination_location_id,
        } => {
            require_revision(&campaign, expected_revision)?;
            let membership = load_campaign_membership(store, campaign.id)?;
            let member = require_active_member(&membership, &member_id)?;
            let origin_location_id = campaign
                .actors
                .get(&member.actor_id)
                .ok_or_else(|| KernelError::Invalid("member actor is missing".into()))?
                .location_id
                .clone();
            if membership
                .members
                .values()
                .filter(|item| item.active)
                .any(|item| {
                    campaign
                        .actors
                        .get(&item.actor_id)
                        .is_none_or(|actor| actor.location_id != origin_location_id)
                })
            {
                return Err(KernelError::Invalid(
                    "group travel requires every player to occupy the same scene".into(),
                ));
            }
            let route = campaign
                .locations
                .get(&origin_location_id)
                .and_then(|location| {
                    location
                        .routes
                        .values()
                        .find(|route| route.destination_id == destination_location_id)
                })
                .ok_or_else(|| {
                    KernelError::Invalid("destination is not directly reachable".into())
                })?;
            if store
                .load_all::<GroupTravelProposal>("group_travel_proposal.v1")
                .map_err(persist)?
                .into_iter()
                .any(|proposal| {
                    !proposal.committed && proposal.expected_world_revision == campaign.revision
                })
            {
                return Err(KernelError::Invalid(
                    "a group-travel proposal is already pending at this revision".into(),
                ));
            }
            let proposal_id = format!("travel:{}", uuid::Uuid::new_v4().simple());
            let proposal = GroupTravelProposal {
                schema: "ghostlight.group_travel_proposal.v1".into(),
                id: proposal_id.clone(),
                campaign_id: campaign.id,
                expected_world_revision: campaign.revision,
                origin_location_id,
                destination_location_id,
                travel_minutes: route.travel_minutes,
                proposer_member_id: member_id.clone(),
                approvals: BTreeSet::from([member_id]),
                committed: false,
            };
            let proposal_row = store
                .insert(
                    "group_travel_proposal.v1",
                    "ghostlight.group_travel_proposal.v1",
                    &proposal_id,
                    &proposal,
                )
                .map_err(persist)?;
            if active_member_ids(&membership) == proposal.approvals {
                commit_governed_group_travel(
                    store,
                    row,
                    campaign,
                    proposal_row,
                    proposal,
                    &membership,
                )
            } else {
                Ok(CommandResult::TravelGovernancePending { campaign, proposal })
            }
        }
        WorldCommand::ApproveGroupTravel {
            expected_revision,
            proposal_id,
            member_id,
        } => {
            require_revision(&campaign, expected_revision)?;
            let membership = load_campaign_membership(store, campaign.id)?;
            require_active_member(&membership, &member_id)?;
            let (proposal_row, mut proposal) = store
                .load::<GroupTravelProposal>("group_travel_proposal.v1", &proposal_id)
                .map_err(persist)?
                .ok_or_else(|| {
                    KernelError::Invalid("group-travel proposal does not exist".into())
                })?;
            if proposal.committed
                || proposal.campaign_id != campaign.id
                || proposal.expected_world_revision != campaign.revision
            {
                return Err(KernelError::Invalid(
                    "group-travel proposal is committed, stale, or belongs elsewhere".into(),
                ));
            }
            let newly_approved = proposal.approvals.insert(member_id);
            if active_member_ids(&membership) == proposal.approvals {
                commit_governed_group_travel(
                    store,
                    row,
                    campaign,
                    proposal_row,
                    proposal,
                    &membership,
                )
            } else if !newly_approved {
                Err(KernelError::Invalid(
                    "member already approved this group-travel proposal".into(),
                ))
            } else {
                store
                    .replace(
                        &proposal_row,
                        "ghostlight.group_travel_proposal.v1",
                        &proposal,
                    )
                    .map_err(persist)?;
                Ok(CommandResult::TravelGovernancePending { campaign, proposal })
            }
        }
        WorldCommand::SetResolutionBudget {
            expected_revision,
            expected_resolution_epoch,
            active_cell_budget,
        } => {
            require_revision(&campaign, expected_revision)?;
            require_resolution_epoch(&campaign, expected_resolution_epoch)?;
            if campaign.resolution_policy.active_cell_budget == active_cell_budget {
                return Err(KernelError::Invalid(
                    "resolution budget command does not change the active cell budget".into(),
                ));
            }
            let previous_epoch = campaign.resolution_policy.resolution_epoch;
            campaign.resolution_policy.active_cell_budget = active_cell_budget;
            campaign.resolution_policy.pending_active_cell_budget = None;
            crate::resolution::validate_policy(&campaign.resolution_policy)
                .map_err(|error| KernelError::Invalid(error.to_string()))?;
            campaign.resolution_policy.resolution_epoch = previous_epoch.saturating_add(1);
            campaign.resolution_cover = None;
            commit_resolution_control(
                store,
                row,
                campaign,
                previous_epoch,
                "set_active_cell_budget",
            )
        }
        WorldCommand::ProposeResolutionBudget {
            expected_revision,
            expected_resolution_epoch,
            member_id,
            active_cell_budget,
        } => {
            require_revision(&campaign, expected_revision)?;
            require_resolution_epoch(&campaign, expected_resolution_epoch)?;
            let membership = load_campaign_membership(store, campaign.id)?;
            require_active_member(&membership, &member_id)?;
            if active_cell_budget == 0
                || active_cell_budget > membership.pooled_cell_allowance()
                || active_cell_budget == campaign.resolution_policy.active_cell_budget
            {
                return Err(KernelError::Invalid(
                    "cell budget must change and remain within the pooled allowance".into(),
                ));
            }
            if store
                .load_all::<CellBudgetProposal>("cell_budget_proposal.v1")
                .map_err(persist)?
                .into_iter()
                .any(|proposal| {
                    !proposal.committed
                        && proposal.expected_world_revision == campaign.revision
                        && proposal.expected_resolution_epoch
                            == campaign.resolution_policy.resolution_epoch
                })
            {
                return Err(KernelError::Invalid(
                    "a cell-budget proposal is already pending at this boundary".into(),
                ));
            }
            let proposal_id = format!("cell-budget:{}", uuid::Uuid::new_v4().simple());
            let proposal = CellBudgetProposal {
                schema: "ghostlight.cell_budget_proposal.v1".into(),
                id: proposal_id.clone(),
                campaign_id: campaign.id,
                expected_world_revision: campaign.revision,
                expected_resolution_epoch: campaign.resolution_policy.resolution_epoch,
                active_cell_budget,
                proposer_member_id: member_id.clone(),
                approvals: BTreeSet::from([member_id]),
                committed: false,
            };
            let proposal_row = store
                .insert(
                    "cell_budget_proposal.v1",
                    "ghostlight.cell_budget_proposal.v1",
                    &proposal_id,
                    &proposal,
                )
                .map_err(persist)?;
            if active_member_ids(&membership) == proposal.approvals {
                commit_governed_cell_budget(store, row, campaign, proposal_row, proposal)
            } else {
                Ok(CommandResult::ResolutionGovernancePending { campaign, proposal })
            }
        }
        WorldCommand::ApproveResolutionBudget {
            expected_revision,
            proposal_id,
            member_id,
        } => {
            require_revision(&campaign, expected_revision)?;
            let membership = load_campaign_membership(store, campaign.id)?;
            require_active_member(&membership, &member_id)?;
            let (proposal_row, mut proposal) = store
                .load::<CellBudgetProposal>("cell_budget_proposal.v1", &proposal_id)
                .map_err(persist)?
                .ok_or_else(|| {
                    KernelError::Invalid("cell-budget proposal does not exist".into())
                })?;
            if proposal.committed
                || proposal.campaign_id != campaign.id
                || proposal.expected_world_revision != campaign.revision
                || proposal.expected_resolution_epoch != campaign.resolution_policy.resolution_epoch
            {
                return Err(KernelError::Invalid(
                    "cell-budget proposal is committed, stale, or belongs elsewhere".into(),
                ));
            }
            let newly_approved = proposal.approvals.insert(member_id);
            if active_member_ids(&membership) == proposal.approvals {
                commit_governed_cell_budget(store, row, campaign, proposal_row, proposal)
            } else if !newly_approved {
                Err(KernelError::Invalid(
                    "member already approved this cell-budget proposal".into(),
                ))
            } else {
                store
                    .replace(
                        &proposal_row,
                        "ghostlight.cell_budget_proposal.v1",
                        &proposal,
                    )
                    .map_err(persist)?;
                Ok(CommandResult::ResolutionGovernancePending { campaign, proposal })
            }
        }
        WorldCommand::SetProviderParallelism {
            expected_revision,
            expected_provider_configuration_epoch,
            provider_parallelism,
        } => {
            require_revision(&campaign, expected_revision)?;
            if campaign.resolution_policy.provider_configuration_epoch
                != expected_provider_configuration_epoch
            {
                return Err(KernelError::Stale {
                    expected: expected_provider_configuration_epoch,
                    actual: campaign.resolution_policy.provider_configuration_epoch,
                });
            }
            if campaign.resolution_policy.provider_parallelism == provider_parallelism {
                return Err(KernelError::Invalid(
                    "provider parallelism command does not change provider configuration".into(),
                ));
            }
            campaign.resolution_policy.provider_parallelism = provider_parallelism;
            crate::resolution::validate_policy(&campaign.resolution_policy)
                .map_err(|error| KernelError::Invalid(error.to_string()))?;
            let previous_epoch = campaign.resolution_policy.resolution_epoch;
            campaign.resolution_policy.provider_configuration_epoch = campaign
                .resolution_policy
                .provider_configuration_epoch
                .saturating_add(1);
            commit_resolution_control(
                store,
                row,
                campaign,
                previous_epoch,
                "set_provider_parallelism",
            )
        }
        WorldCommand::ReplaceResolutionPins {
            expected_revision,
            expected_resolution_epoch,
            pins,
        } => {
            require_revision(&campaign, expected_revision)?;
            require_resolution_epoch(&campaign, expected_resolution_epoch)?;
            let mut replacement = BTreeMap::new();
            for pin in pins {
                if replacement.insert(pin.id.clone(), pin).is_some() {
                    return Err(KernelError::Invalid("duplicate resolution pin id".into()));
                }
            }
            crate::resolution::validate_pins(&campaign, &replacement)
                .map_err(|error| KernelError::Invalid(error.to_string()))?;
            let previous_epoch = campaign.resolution_policy.resolution_epoch;
            campaign.resolution_pins = replacement;
            campaign.resolution_policy.resolution_epoch = previous_epoch.saturating_add(1);
            campaign.resolution_cover = None;
            commit_resolution_control(
                store,
                row,
                campaign,
                previous_epoch,
                "replace_resolution_pins",
            )
        }
        WorldCommand::FissionGestalt {
            expected_revision,
            preview,
            evidence_receipts,
            model_stage_receipts,
        } => {
            require_revision(&campaign, expected_revision)?;
            let supplied_evidence: BTreeSet<_> = evidence_receipts
                .iter()
                .map(|receipt| receipt.id.clone())
                .collect();
            if preview
                .evidence_receipt_ids
                .iter()
                .any(|id| !supplied_evidence.contains(id))
            {
                return Err(KernelError::Invalid(
                    "fission preview evidence receipts were not supplied".into(),
                ));
            }
            let transition = crate::legacy_transition::lower_fission(
                &campaign,
                &preview,
                Utc::now() + Duration::minutes(5),
            )
            .map_err(|error| KernelError::Invalid(error.to_string()))?;
            let mutation_receipt = crate::legacy_transition::apply_lowered_fission(
                &mut campaign,
                &preview,
                &transition,
                Utc::now(),
            )
            .map_err(|error| KernelError::Invalid(error.to_string()))?;
            for candidate in &preview.canon_candidates {
                campaign
                    .canon_candidates
                    .insert(candidate.id.clone(), candidate.clone());
            }
            commit_with_records(
                store,
                row,
                campaign,
                "fission_gestalt",
                evidence_receipts,
                preview.canon_candidates,
                model_stage_receipts,
                Some((transition, mutation_receipt)),
            )
        }
        WorldCommand::AdvanceStrategicTick {
            expected_revision,
            source,
            plan,
            model_receipt_hash,
            resolution_wave,
        } => {
            require_revision(&campaign, expected_revision)?;
            if plan.is_some() && resolution_wave.is_some() {
                return Err(KernelError::Invalid(
                    "strategic tick has two competing plan authorities".into(),
                ));
            }
            let resolved_plan = resolution_wave
                .as_ref()
                .map(|wave| crate::resolution::validate_and_resolve_wave(&campaign, wave))
                .transpose()
                .map_err(|error| KernelError::Invalid(error.to_string()))?;
            if (plan.is_some() || resolution_wave.is_some())
                && model_receipt_hash
                    .as_deref()
                    .is_none_or(|hash| !valid_sha256(hash))
            {
                return Err(KernelError::Invalid(
                    "model-driven strategic tick lacks an exact model receipt hash".into(),
                ));
            }
            if let Some(wave) = &resolution_wave {
                let unique_hashes: BTreeSet<_> = wave.model_receipt_hashes.iter().collect();
                let action_count = wave
                    .appraisals
                    .iter()
                    .map(|appraisal| appraisal.actions.len())
                    .sum::<usize>();
                let outcome_digests = resolved_plan
                    .as_ref()
                    .map(crate::outcome::plan_activity_digests)
                    .unwrap_or_default();
                let outcome_stage_count = usize::from(!outcome_digests.is_empty());
                if unique_hashes.len() != wave.model_receipt_hashes.len()
                    || wave.model_receipt_hashes.len()
                        < wave
                            .cover
                            .cells
                            .len()
                            .saturating_mul(3)
                            .saturating_add(action_count)
                            .saturating_add(outcome_stage_count)
                {
                    return Err(KernelError::Invalid(
                        "resolution wave does not carry one distinct stage receipt per cell stage"
                            .into(),
                    ));
                }
                let mut stage_bindings = BTreeSet::new();
                for hash in &wave.model_receipt_hashes {
                    let Some((_, receipt)) = store
                        .load::<crate::model::ModelStageReceipt>("persona_stage_receipt.v1", hash)
                        .map_err(persist)?
                    else {
                        return Err(KernelError::Invalid(
                            "resolution wave references an unknown model receipt".into(),
                        ));
                    };
                    if receipt.storage_key() != hash {
                        return Err(KernelError::Invalid(
                            "resolution wave contains a mismatched model receipt".into(),
                        ));
                    }
                    if receipt.validation_result == "valid" {
                        stage_bindings
                            .insert((receipt.stage.clone(), receipt.snapshot_binding.clone()));
                    }
                }
                for cell in &wave.cover.cells {
                    let binding = format!(
                        "campaign:{}:revision:{}:resolution:{}:cell:{}",
                        campaign.id,
                        campaign.revision,
                        campaign.resolution_policy.resolution_epoch,
                        cell.id
                    );
                    for stage in ["cell_projector", "cell_persona", "cell_interpreter"] {
                        if !stage_bindings.contains(&(stage.into(), binding.clone())) {
                            return Err(KernelError::Invalid(format!(
                                "resolution wave lacks {stage} receipt for {}",
                                cell.id
                            )));
                        }
                    }
                    let actionful = wave
                        .appraisals
                        .iter()
                        .find(|appraisal| appraisal.cell_id == cell.id)
                        .is_some_and(|appraisal| !appraisal.actions.is_empty());
                    if actionful {
                        let appraisal = wave
                            .appraisals
                            .iter()
                            .find(|appraisal| appraisal.cell_id == cell.id)
                            .expect("active cell appraisal was validated");
                        for action in &appraisal.actions {
                            let verifier_binding =
                                crate::persona::cell_effect_verification_binding(
                                    &binding,
                                    std::slice::from_ref(action),
                                )
                                .map_err(|error| KernelError::Invalid(error.to_string()))?;
                            if !stage_bindings
                                .contains(&("cell_effect_verifier".into(), verifier_binding))
                            {
                                return Err(KernelError::Invalid(format!(
                                    "resolution wave lacks action-bound cell_effect_verifier receipt for {} action by {}",
                                    cell.id, action.subject_id
                                )));
                            }
                        }
                    }
                }
                if !outcome_digests.is_empty() {
                    let binding = crate::outcome::activity_outcome_binding(
                        campaign.id,
                        campaign.revision,
                        campaign.resolution_policy.resolution_epoch,
                        &outcome_digests,
                    );
                    if !stage_bindings.contains(&("strategic_outcome_resolver".into(), binding)) {
                        return Err(KernelError::Invalid(
                            "resolution wave lacks an activity-bound strategic outcome receipt"
                                .into(),
                        ));
                    }
                }
            }
            let applied_tick = match resolved_plan.or(plan) {
                Some(plan) => apply_strategic_tick_plan(&mut campaign, plan)?,
                None => {
                    let plan = deterministic_strategic_tick_plan();
                    apply_strategic_tick_plan(&mut campaign, plan)?
                }
            };
            let AppliedStrategicTickPlan {
                events: tick_events,
                mutation,
            } = applied_tick;
            if let Some(wave) = &resolution_wave {
                crate::resolution::advance_detail_debt(&mut campaign, &wave.cover);
                campaign.resolution_cover = Some(wave.cover.clone());
            }
            campaign.strategic_tick_count = campaign.strategic_tick_count.saturating_add(1);
            for event in &tick_events {
                for channel in &event.public_channels {
                    campaign.news.push(crate::domain::NewsIssue {
                        id: format!("news:{}:{}", event.id, stable_channel_id(channel)),
                        at: campaign.world_time,
                        channel: channel.clone(),
                        headline: event.summary.clone(),
                        event_ids: vec![event.id.clone()],
                        reliability: "direct institutional channel".into(),
                    });
                }
            }
            let event_ids = tick_events.iter().map(|event| event.id.clone()).collect();
            campaign.events.extend(tick_events);
            if source == TickSource::PlayerWait {
                campaign.last_player_activity = Utc::now();
                campaign.away_ticks_processed = 0;
                campaign.pending_ticks = 0;
            } else {
                campaign.away_ticks_processed =
                    campaign.away_ticks_processed.saturating_add(1).min(8);
                campaign.pending_ticks = campaign.pending_ticks.saturating_sub(1);
            }
            commit_strategic_tick(
                store,
                row,
                campaign,
                source,
                model_receipt_hash,
                event_ids,
                resolution_wave,
                mutation,
            )
        }
        WorldCommand::ExpandRegion {
            expected_revision,
            expansion,
            evidence_receipts,
            canon_candidates,
            model_stage_receipts,
        } => {
            require_revision(&campaign, expected_revision)?;
            let supplied_evidence = evidence_receipts
                .iter()
                .map(|receipt| receipt.id.as_str())
                .collect::<BTreeSet<_>>();
            if expansion.facts.iter().any(|fact| {
                fact.evidence_receipt_ids
                    .iter()
                    .any(|id| !supplied_evidence.contains(id.as_str()))
            }) || canon_candidates.iter().any(|candidate| {
                candidate
                    .evidence_receipt_ids
                    .iter()
                    .any(|id| !supplied_evidence.contains(id.as_str()))
            }) {
                return Err(KernelError::Invalid(
                    "region expansion evidence receipts were not supplied".into(),
                ));
            }
            crate::compiler::validate_region_expansion(&campaign, &expansion)
                .map_err(|error| KernelError::Invalid(error.to_string()))?;
            let transition = crate::legacy_transition::lower_region_expansion(
                &campaign,
                &expansion,
                Utc::now() + Duration::minutes(5),
            )
            .map_err(|error| KernelError::Invalid(error.to_string()))?;
            let mutation_receipt = crate::legacy_transition::apply_lowered_region_expansion(
                &mut campaign,
                &expansion,
                &transition,
                Utc::now(),
            )
            .map_err(|error| KernelError::Invalid(error.to_string()))?;
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
                model_stage_receipts,
                Some((transition, mutation_receipt)),
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
            let before = campaign.clone();
            apply_promotion(
                &mut campaign,
                &GestaltPromotion {
                    gestalt_id,
                    expected_gestalt_version,
                    member_id,
                    expected_member_version,
                    location_id,
                },
            )?;
            commit_gestalt_presence(
                store,
                row,
                before,
                campaign,
                "materialize_gestalt_member",
                "direct materialization command",
            )
        }
        WorldCommand::IndividuateGestaltMember {
            expected_revision,
            individuation,
        } => {
            require_revision(&campaign, expected_revision)?;
            let before = campaign.clone();
            apply_individuation(&mut campaign, &individuation)?;
            commit_gestalt_presence(
                store,
                row,
                before,
                campaign,
                "individuate_gestalt_member",
                "direct individuation command",
            )
        }
        WorldCommand::DematerializeGestaltMember {
            expected_revision,
            actor_id,
        } => {
            require_revision(&campaign, expected_revision)?;
            let before = campaign.clone();
            apply_demotion(&mut campaign, &GestaltDemotion { actor_id })?;
            commit_gestalt_presence(
                store,
                row,
                before,
                campaign,
                "dematerialize_gestalt_member",
                "direct dematerialization command",
            )
        }
        WorldCommand::ReconcileGestaltPresence {
            expected_revision,
            reason,
            plan,
        } => {
            require_revision(&campaign, expected_revision)?;
            if reason.trim().is_empty() {
                return Err(KernelError::Invalid("presence plan has no reason".into()));
            }
            if !plan.individuations.is_empty()
                && crate::gestalt::automatic_individuation_stimulus(&campaign).as_deref()
                    != Some(reason.as_str())
            {
                return Err(KernelError::Invalid(
                    "automatic individuation is admitted only by the exact immediately committed player speech"
                        .into(),
                ));
            }
            if plan.individuations.len() > 1 {
                return Err(KernelError::Invalid(
                    "one player speech can admit at most one first-relevance person".into(),
                ));
            }
            let addressed_public_identities =
                crate::gestalt::automatic_individuation_addressed_actor_ids(&campaign)
                    .iter()
                    .filter_map(|actor_id| campaign.actors.get(actor_id))
                    .map(|actor| actor.name.trim().to_lowercase())
                    .collect::<BTreeSet<_>>();
            if plan.individuations.iter().any(|individuation| {
                addressed_public_identities
                    .contains(&individuation.member.name.trim().to_lowercase())
            }) {
                return Err(KernelError::Invalid(
                    "presence individuation duplicates an already-addressed actor".into(),
                ));
            }
            let player_location = &campaign.actors[&campaign.player_actor_id].location_id;
            if plan
                .individuations
                .iter()
                .any(|entry| &entry.location_id != player_location)
                || plan
                    .promotions
                    .iter()
                    .any(|entry| &entry.location_id != player_location)
            {
                return Err(KernelError::Invalid(
                    "automatic presence reconciliation is confined to the player's active location"
                        .into(),
                ));
            }
            let before = campaign.clone();
            let mut candidate = campaign.clone();
            for demotion in &plan.demotions {
                apply_demotion(&mut candidate, demotion)?;
            }
            for individuation in &plan.individuations {
                apply_individuation(&mut candidate, individuation)?;
            }
            for promotion in &plan.promotions {
                apply_promotion(&mut candidate, promotion)?;
            }
            commit_gestalt_presence(
                store,
                row,
                before,
                candidate,
                "reconcile_gestalt_presence",
                &reason,
            )
        }
        WorldCommand::ResolveReactionWave {
            expected_revision,
            event_summary,
            reactions,
        } => {
            require_revision(&campaign, expected_revision)?;
            let witnessed_turn = canonical_witnessed_turn(&campaign, &event_summary)?;
            let response_expected_actor_ids = witnessed_turn.persona_response_actor_ids.clone();
            let witnessed_memory = format!("Witnessed: {}", canonical_turn_text(witnessed_turn));
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
                if !reaction.private_delta.memories_add.is_empty() {
                    return Err(KernelError::Invalid(
                        "reaction Interpreter cannot write actor memory".into(),
                    ));
                }
                if let Some(identity) = reaction.private_delta.identity_adoption.as_deref() {
                    validate_bounded_text("reaction identity adoption", identity, 160)?;
                    let speech = reaction.speech.as_deref().ok_or_else(|| {
                        KernelError::Invalid(
                            "reaction identity adoption requires public speech".into(),
                        )
                    })?;
                    if !speech.to_lowercase().contains(&identity.to_lowercase()) {
                        return Err(KernelError::Invalid(
                            "reaction identity adoption must copy an exact spoken handle".into(),
                        ));
                    }
                }
                if let Some(speech) = &reaction.speech {
                    validate_bounded_text("reaction speech", speech, 1_000)?;
                }
                for proposal in &reaction.action_proposals {
                    validate_world_proposal(actor, proposal)?;
                }
            }
            for response_actor_id in &response_expected_actor_ids {
                let reaction = reactions
                    .iter()
                    .find(|reaction| &reaction.actor_id == response_actor_id)
                    .ok_or_else(|| {
                        KernelError::Invalid(
                            "directly addressed Persona is absent from the reaction wave".into(),
                        )
                    })?;
                if reaction
                    .speech
                    .as_deref()
                    .is_none_or(|value| value.trim().is_empty())
                    && !reaction.deliberate_silence
                {
                    return Err(KernelError::Invalid(
                        "directly addressed Persona produced no observable response".into(),
                    ));
                }
            }
            let source_receipt_id = format!(
                "reaction-input:{}",
                crate::legacy_transition::digest_serializable(&reactions)
                    .map_err(|error| KernelError::Invalid(error.to_string()))?
            );
            let transition = crate::legacy_transition::lower_reaction_wave(
                &campaign,
                &witnessed_memory,
                &reactions,
                &source_receipt_id,
                Utc::now() + Duration::minutes(5),
            )
            .map_err(|error| KernelError::Invalid(error.to_string()))?;
            let mutation_receipt = crate::legacy_transition::apply_lowered_transition(
                &mut campaign,
                &transition,
                Utc::now(),
            )
            .map_err(|error| KernelError::Invalid(error.to_string()))?;
            campaign.pending_world_proposals.clear();
            for reaction in reactions {
                if let Some(speech) = reaction.speech {
                    campaign.transcript.push(NarrativeTurn {
                        revision: campaign.revision + 1,
                        at: Utc::now(),
                        speaker: reaction.actor_id.clone(),
                        text: speech,
                        persona_response_actor_ids: BTreeSet::new(),
                    });
                }
                if reaction.deliberate_silence {
                    campaign.transcript.push(NarrativeTurn {
                        revision: campaign.revision + 1,
                        at: Utc::now(),
                        speaker: reaction.actor_id.clone(),
                        text: "deliberately does not answer.".into(),
                        persona_response_actor_ids: BTreeSet::new(),
                    });
                }
                campaign
                    .pending_world_proposals
                    .extend(reaction.action_proposals);
            }
            refresh_materialized_member_relevance(&mut campaign, seen.iter().map(String::as_str));
            campaign.events.push(Event {
                id: format!("reaction-wave:{}", campaign.revision + 1),
                at: campaign.world_time,
                kind: "reaction_wave".into(),
                summary: event_summary,
                actor_ids: seen.into_iter().collect(),
                institution_ids: vec![],
                gestalt_ids: vec![],
                location_ids: vec![player_location],
                public_channels: vec![],
            });
            commit_mutation_transition(
                store,
                row,
                campaign,
                "reaction_wave",
                None,
                transition,
                mutation_receipt,
            )
        }
        WorldCommand::ResolveNpcAction {
            expected_revision,
            proposal,
            assessment,
        } => {
            require_revision(&campaign, expected_revision)?;
            let current_window_id = format!("reaction-wave:{}", campaign.revision);
            if campaign
                .events
                .last()
                .is_none_or(|event| event.kind != "reaction_wave" || event.id != current_window_id)
            {
                return Err(KernelError::Invalid(
                    "pending NPC initiative does not belong to the current reaction wave".into(),
                ));
            }
            let selected = crate::initiative::winner(&campaign.pending_world_proposals)
                .ok_or_else(|| KernelError::Invalid("there is no pending NPC action".into()))?;
            if proposal != selected {
                return Err(KernelError::Invalid(
                    "proposal does not own the current initiative opportunity".into(),
                ));
            }
            let actor = campaign
                .actors
                .get(&proposal.actor_id)
                .ok_or_else(|| KernelError::Invalid("initiative actor is unknown".into()))?;
            validate_world_proposal(actor, &proposal)?;
            let intent = ActionIntent {
                actor_id: proposal.actor_id.clone(),
                description: proposal.intent.clone(),
                intended_effect: proposal.intended_effect.clone(),
            };
            if assessment.campaign_id != campaign.id
                || assessment.revision != campaign.revision
                || assessment.intent != intent
                || assessment.expires_at < Utc::now()
                || crate::assessor::assessment_digest(&assessment)
                    .map_err(|error| KernelError::Invalid(error.to_string()))?
                    != assessment.digest
            {
                return Err(KernelError::Invalid(
                    "NPC assessment is stale or invalid".into(),
                ));
            }
            for (effect, stake) in [
                (&assessment.strong_effect, &assessment.success_stake),
                (&assessment.success_effect, &assessment.success_stake),
                (&assessment.mixed_effect, &assessment.mixed_stake),
                (&assessment.failure_effect, &assessment.failure_stake),
            ] {
                crate::assessor::validate_effect(&campaign, actor, effect, stake)
                    .map_err(|error| KernelError::Invalid(error.to_string()))?;
                validate_bounded_coop_effect(store, &campaign, &proposal.actor_id, effect)?;
            }
            let actor_name = actor.name.clone();
            let actor_location = actor.location_id.clone();
            campaign.pending_world_proposals.clear();
            let roll = if assessment.admissible {
                let roll = receipt(
                    assessment.digest.clone(),
                    rand::rng().random_range(1..=20),
                    assessment.modifier_total,
                    assessment.dc,
                );
                let (text, effect) = match roll.outcome {
                    OutcomeBand::StrongSuccess => {
                        (&assessment.success_stake, &assessment.strong_effect)
                    }
                    OutcomeBand::Success => (&assessment.success_stake, &assessment.success_effect),
                    OutcomeBand::Mixed => (&assessment.mixed_stake, &assessment.mixed_effect),
                    OutcomeBand::Failure => (&assessment.failure_stake, &assessment.failure_effect),
                };
                let transition = crate::legacy_transition::lower_foreground_effect(
                    &campaign,
                    &proposal.actor_id,
                    effect,
                    roll.outcome.clone(),
                    crate::transition::MutationProcedure::NpcAttempt,
                    &assessment.effect_ceiling,
                    &assessment.digest,
                    Some(
                        crate::legacy_transition::digest_serializable(&intent)
                            .map_err(|error| KernelError::Invalid(error.to_string()))?,
                    ),
                    Some(
                        crate::legacy_transition::digest_serializable(&intent.intended_effect)
                            .map_err(|error| KernelError::Invalid(error.to_string()))?,
                    ),
                    assessment.expires_at,
                )
                .map_err(|error| KernelError::Invalid(error.to_string()))?;
                let mutation_receipt = crate::legacy_transition::apply_lowered_transition(
                    &mut campaign,
                    &transition,
                    Utc::now(),
                )
                .map_err(|error| KernelError::Invalid(error.to_string()))?;
                campaign.transcript.push(NarrativeTurn {
                    revision: campaign.revision + 1,
                    at: Utc::now(),
                    speaker: "world".into(),
                    text: text.clone(),
                    persona_response_actor_ids: BTreeSet::new(),
                });
                Some((roll, transition, mutation_receipt))
            } else {
                None
            };
            campaign.events.push(Event {
                id: format!("npc-action-resolved:{}", campaign.revision + 1),
                at: campaign.world_time,
                kind: "npc_action_resolved".into(),
                summary: if assessment.admissible {
                    format!("{} attempts {}", actor_name, proposal.intent)
                } else {
                    format!("{} cannot yet attempt {}", actor_name, proposal.intent)
                },
                actor_ids: vec![proposal.actor_id.clone()],
                institution_ids: vec![],
                gestalt_ids: vec![],
                location_ids: vec![actor_location],
                public_channels: vec![],
            });
            refresh_materialized_member_relevance(
                &mut campaign,
                std::iter::once(proposal.actor_id.as_str()),
            );
            if let Some((roll, transition, mutation_receipt)) = roll {
                commit_mutation_transition(
                    store,
                    row,
                    campaign,
                    "resolve_npc_action",
                    Some(roll),
                    transition,
                    mutation_receipt,
                )
            } else {
                commit(store, row, campaign, "resolve_npc_action", None)
            }
        }
        WorldCommand::CreateCampaign { .. } => unreachable!(),
    }
}

fn validate_intent_text(intent: &ActionIntent) -> Result<(), KernelError> {
    validate_bounded_text("action description", &intent.description, 4_000)?;
    validate_bounded_text("intended effect", &intent.intended_effect, 1_000)
}

fn validate_bounded_text(label: &str, value: &str, max_chars: usize) -> Result<(), KernelError> {
    if value.trim().is_empty()
        || value.chars().count() > max_chars
        || value
            .chars()
            .any(|character| character.is_control() && !matches!(character, '\n' | '\r' | '\t'))
    {
        return Err(KernelError::Invalid(format!(
            "{label} must contain 1 to {max_chars} readable characters"
        )));
    }
    Ok(())
}

fn apply_individuation(
    campaign: &mut Campaign,
    individuation: &GestaltIndividuation,
) -> Result<(), KernelError> {
    let member = &individuation.member;
    let gestalt = campaign
        .gestalts
        .get(&individuation.gestalt_id)
        .ok_or_else(|| KernelError::Invalid("gestalt is unknown".into()))?;
    crate::resolution::validate_active_gestalt_presence_location(
        campaign,
        &individuation.gestalt_id,
        &individuation.location_id,
    )
    .map_err(|error| KernelError::Invalid(error.to_string()))?;
    if gestalt.version != individuation.expected_gestalt_version
        || member.gestalt_id != individuation.gestalt_id
        || member.version != 0
        || member.materialized_actor_id.is_some()
        || member.id.trim().is_empty()
        || member.name.trim().is_empty()
        || campaign.gestalt_members.contains_key(&member.id)
    {
        return Err(KernelError::Invalid(
            "gestalt individuation is stale or malformed".into(),
        ));
    }
    campaign
        .gestalt_members
        .insert(member.id.clone(), member.clone());
    apply_promotion(
        campaign,
        &GestaltPromotion {
            gestalt_id: individuation.gestalt_id.clone(),
            expected_gestalt_version: individuation.expected_gestalt_version,
            member_id: member.id.clone(),
            expected_member_version: 0,
            location_id: individuation.location_id.clone(),
        },
    )
}

fn apply_promotion(
    campaign: &mut Campaign,
    promotion: &GestaltPromotion,
) -> Result<(), KernelError> {
    if !campaign.locations.contains_key(&promotion.location_id) {
        return Err(KernelError::Invalid(
            "materialization location is unknown".into(),
        ));
    }
    crate::resolution::validate_active_gestalt_presence_location(
        campaign,
        &promotion.gestalt_id,
        &promotion.location_id,
    )
    .map_err(|error| KernelError::Invalid(error.to_string()))?;
    let gestalt = campaign
        .gestalts
        .get(&promotion.gestalt_id)
        .ok_or_else(|| KernelError::Invalid("gestalt is unknown".into()))?
        .clone();
    if gestalt.version != promotion.expected_gestalt_version {
        return Err(KernelError::Invalid("gestalt snapshot is stale".into()));
    }
    let member_location =
        crate::resolution::dormant_member_location(campaign, &promotion.member_id)
            .map_err(|error| KernelError::Invalid(error.to_string()))?;
    if member_location != promotion.location_id {
        return Err(KernelError::Invalid(
            "gestalt member cannot materialize outside their exact location".into(),
        ));
    }
    let member = campaign
        .gestalt_members
        .get_mut(&promotion.member_id)
        .ok_or_else(|| KernelError::Invalid("gestalt member is unknown".into()))?;
    if member.gestalt_id != promotion.gestalt_id
        || member.version != promotion.expected_member_version
    {
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
    let actor = materialize_actor(&gestalt, member, &actor_id, &promotion.location_id);
    member.materialized_actor_id = Some(actor_id.clone());
    member.last_location_id = Some(promotion.location_id.clone());
    member.last_relevant_revision = campaign.revision;
    member.relevance_lease_until_revision = campaign
        .revision
        .saturating_add(GESTALT_RELEVANCE_LEASE_REVISIONS);
    member.version += 1;
    campaign.actors.insert(actor_id, actor);
    Ok(())
}

fn apply_demotion(campaign: &mut Campaign, demotion: &GestaltDemotion) -> Result<(), KernelError> {
    let actor = campaign
        .actors
        .get(&demotion.actor_id)
        .ok_or_else(|| KernelError::Invalid("materialized actor is unknown".into()))?
        .clone();
    let member_id = campaign
        .gestalt_members
        .values()
        .find(|member| member.materialized_actor_id.as_deref() == Some(demotion.actor_id.as_str()))
        .map(|member| member.id.clone())
        .ok_or_else(|| KernelError::Invalid("actor is not a materialized gestalt member".into()))?;
    if actor.location_id == campaign.actors[&campaign.player_actor_id].location_id {
        return Err(KernelError::Invalid(
            "a visible gestalt member remains individually relevant".into(),
        ));
    }
    if campaign.gestalt_members[&member_id].relevance_lease_until_revision > campaign.revision {
        return Err(KernelError::Invalid(
            "gestalt member relevance lease has not expired".into(),
        ));
    }
    let gestalt_id = campaign.gestalt_members[&member_id].gestalt_id.clone();
    let gestalt = campaign
        .gestalts
        .get(&gestalt_id)
        .ok_or_else(|| KernelError::Invalid("gestalt is missing".into()))?;
    let member = campaign
        .gestalt_members
        .get_mut(&member_id)
        .expect("member exists");
    fold_actor_delta(&actor, gestalt, member);
    member.materialized_actor_id = None;
    member.version += 1;
    campaign.actors.remove(&demotion.actor_id);
    Ok(())
}

fn refresh_materialized_member_relevance<'a>(
    campaign: &mut Campaign,
    actor_ids: impl IntoIterator<Item = &'a str>,
) {
    let actor_ids = actor_ids.into_iter().collect::<BTreeSet<_>>();
    for member in campaign.gestalt_members.values_mut() {
        if member
            .materialized_actor_id
            .as_deref()
            .is_some_and(|actor_id| actor_ids.contains(actor_id))
        {
            member.last_relevant_revision = campaign.revision;
            member.relevance_lease_until_revision = campaign
                .revision
                .saturating_add(GESTALT_RELEVANCE_LEASE_REVISIONS);
            member.version = member.version.saturating_add(1);
        }
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

fn canonical_witnessed_turn<'a>(
    campaign: &'a Campaign,
    event_summary: &str,
) -> Result<&'a NarrativeTurn, KernelError> {
    let event_summary = event_summary.trim();
    campaign
        .transcript
        .iter()
        .rev()
        .find(|turn| canonical_turn_text(turn) == event_summary)
        .ok_or_else(|| {
            KernelError::Invalid(
                "reaction stimulus does not match a committed transcript turn".into(),
            )
        })
}

fn canonical_turn_text(turn: &NarrativeTurn) -> String {
    if turn.speaker == "world" {
        turn.text.trim().to_owned()
    } else {
        format!("{} says: {}", turn.speaker, turn.text.trim())
    }
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

fn is_human_controlled_actor(campaign: &Campaign, actor_id: &str) -> bool {
    actor_id == campaign.player_actor_id
        || campaign
            .agency_profiles
            .get(actor_id)
            .is_some_and(|profile| !profile.simulation_eligible)
}

fn require_resolution_epoch(c: &Campaign, expected: u64) -> Result<(), KernelError> {
    if c.resolution_policy.resolution_epoch != expected {
        return Err(KernelError::Invalid(format!(
            "stale resolution epoch: expected {expected}, actual {}",
            c.resolution_policy.resolution_epoch
        )));
    }
    Ok(())
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
        strong_effect: WorldEffectDelta::default(),
        success_effect: WorldEffectDelta::default(),
        mixed_effect: WorldEffectDelta::default(),
        failure_effect: WorldEffectDelta::default(),
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

#[derive(Debug)]
struct AppliedStrategicTickPlan {
    events: Vec<Event>,
    mutation: Option<(
        crate::legacy_transition::LoweredLegacyTransition,
        crate::transition::WorldMutationReceipt,
    )>,
}

impl std::ops::Deref for AppliedStrategicTickPlan {
    type Target = [Event];

    fn deref(&self) -> &Self::Target {
        &self.events
    }
}

fn apply_strategic_tick_plan(
    campaign: &mut Campaign,
    plan: crate::domain::StrategicTickPlan,
) -> Result<AppliedStrategicTickPlan, KernelError> {
    let plan = if plan.selected_actions.is_empty() {
        plan
    } else {
        let activity_outcomes = plan.activity_outcomes;
        let mut projected =
            crate::resolution::project_selected_actions(campaign, plan.selected_actions)
                .map_err(|error| KernelError::Invalid(error.to_string()))?;
        projected.activity_outcomes = activity_outcomes;
        projected
    };
    crate::outcome::validate_plan_activity_outcomes(campaign, &plan)
        .map_err(|error| KernelError::Invalid(error.to_string()))?;
    let canonical_composition = !plan.selected_actions.is_empty();
    let prospective_actor_locations = plan
        .actor_moves
        .iter()
        .map(|action| (action.actor_id.clone(), action.destination_id.clone()))
        .collect::<BTreeMap<_, _>>();
    let prospective_gestalt_locations = plan
        .gestalt_migrations
        .iter()
        .map(|action| {
            (
                action.gestalt_id.clone(),
                action.destination_location_id.clone(),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let prospective_member_locations = plan
        .member_migrations
        .iter()
        .map(|action| {
            (
                action.member_id.clone(),
                action.destination_location_id.clone(),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let lowering_plan = plan.clone();
    let activity_outcomes = plan.activity_outcomes.clone();
    let mut outcome_event_context = BTreeMap::new();
    for activity in &plan.gestalt_activities {
        outcome_event_context.insert(
            activity.action_digest.clone(),
            (
                activity.gestalt_id.clone(),
                activity.location_ids.clone(),
                activity.public_channels.clone(),
            ),
        );
    }
    for activity in &plan.actor_activities {
        outcome_event_context.insert(
            activity.action_digest.clone(),
            (
                activity.actor_id.clone(),
                activity.location_ids.clone(),
                activity.public_channels.clone(),
            ),
        );
    }
    for activity in &plan.member_activities {
        outcome_event_context.insert(
            activity.action_digest.clone(),
            (
                format!("member:{}", activity.member_id),
                activity.location_ids.clone(),
                activity.public_channels.clone(),
            ),
        );
    }
    // Every action in a strategic wave was chosen against the same committed
    // snapshot. Keep that snapshot immutable while applying to a private copy
    // so action ordering cannot rewrite another action's permissions and an
    // invalid late action cannot leave this primitive partially mutated.
    let mut next = campaign.clone();
    let revision = campaign.revision + 1;
    let at = campaign.world_time + Duration::hours(i64::from(campaign.tick_hours));
    let mut events = Vec::new();
    let mut seen_institutions = BTreeSet::new();
    for action in plan.institution_actions {
        if !seen_institutions.insert(action.institution_id.clone()) {
            return Err(KernelError::Invalid(
                "institution acts twice in one strategic tick".into(),
            ));
        }
        if action.posture.trim().is_empty() || action.posture.len() > 240 {
            return Err(KernelError::Invalid(
                "strategic institution posture is empty".into(),
            ));
        }
        if action
            .location_ids
            .iter()
            .any(|id| !campaign.locations.contains_key(id))
        {
            return Err(KernelError::Invalid(
                "strategic institution action invented a location".into(),
            ));
        }
        validate_public_channels(&action.public_channels)?;
        let institution = campaign
            .institutions
            .get(&action.institution_id)
            .ok_or_else(|| KernelError::Invalid("strategic plan invented an institution".into()))?;
        if !crate::resolution::substantive_text_change(&institution.posture, &action.posture) {
            return Err(KernelError::Invalid(
                "strategic institution action would not change its posture".into(),
            ));
        }
        let summary = format!("{} adopts posture: {}", institution.name, action.posture);
        events.push(crate::domain::Event {
            id: format!("strategic:{revision}:institution:{}", institution.id),
            at,
            kind: "institution_action".into(),
            summary,
            actor_ids: vec![],
            institution_ids: vec![institution.id.clone()],
            gestalt_ids: vec![],
            location_ids: action.location_ids,
            public_channels: action.public_channels,
        });
    }

    let mut legacy_seen_gestalts = BTreeSet::new();
    let mut seen_gestalt_pressures = BTreeSet::new();
    for action in plan.gestalt_actions {
        if !seen_gestalt_pressures.insert(action.gestalt_id.clone())
            || (!canonical_composition && !legacy_seen_gestalts.insert(action.gestalt_id.clone()))
        {
            return Err(KernelError::Invalid(
                "gestalt acts twice in one strategic tick".into(),
            ));
        }
        validate_public_channels(&action.public_channels)?;
        let gestalt = campaign
            .gestalts
            .get(&action.gestalt_id)
            .ok_or_else(|| KernelError::Invalid("strategic plan invented a gestalt".into()))?;
        crate::resolution::validate_gestalt_pressure_transition(
            &gestalt.pressures,
            &action.pressure_additions,
            &action.pressure_resolutions,
        )
        .map_err(|error| KernelError::Invalid(error.to_string()))?;
        let mut summary_parts = Vec::new();
        if !action.pressure_resolutions.is_empty() {
            summary_parts.push(format!(
                "resolves pressure: {}",
                action.pressure_resolutions.join("; ")
            ));
        }
        if !action.pressure_additions.is_empty() {
            summary_parts.push(format!(
                "takes on pressure: {}",
                action.pressure_additions.join("; ")
            ));
        }
        events.push(crate::domain::Event {
            id: format!("strategic:{revision}:gestalt:{}", gestalt.id),
            at,
            kind: "gestalt_action".into(),
            summary: format!("{} {}", gestalt.name, summary_parts.join("; ")),
            actor_ids: vec![],
            institution_ids: vec![],
            gestalt_ids: vec![gestalt.id.clone()],
            location_ids: vec![gestalt.home_location_id.clone()],
            public_channels: action.public_channels,
        });
    }

    let mut seen_gestalt_migrations = BTreeSet::new();
    for action in &plan.gestalt_migrations {
        if !seen_gestalt_migrations.insert(action.gestalt_id.clone())
            || (!canonical_composition && !legacy_seen_gestalts.insert(action.gestalt_id.clone()))
        {
            return Err(KernelError::Invalid(
                "gestalt acts twice in one strategic tick".into(),
            ));
        }
        validate_public_channels(&action.public_channels)?;
        crate::resolution::validate_gestalt_migration(
            campaign,
            &action.gestalt_id,
            &action.destination_gestalt_id,
            &action.destination_location_id,
        )
        .map_err(|error| KernelError::Invalid(error.to_string()))?;
    }
    for action in plan.gestalt_migrations {
        let origin = campaign.gestalts[&action.gestalt_id]
            .home_location_id
            .clone();
        let gestalt_name = campaign.gestalts[&action.gestalt_id].name.clone();
        let destination_name = campaign.gestalts[&action.destination_gestalt_id]
            .name
            .clone();
        events.push(crate::domain::Event {
            id: format!(
                "strategic:{revision}:gestalt-migration:{}",
                action.gestalt_id
            ),
            at,
            kind: "gestalt_migration".into(),
            summary: format!(
                "{gestalt_name} moves from {origin} to {} near {destination_name}.",
                action.destination_location_id
            ),
            actor_ids: vec![],
            institution_ids: vec![],
            gestalt_ids: vec![action.gestalt_id, action.destination_gestalt_id],
            location_ids: vec![origin, action.destination_location_id],
            public_channels: action.public_channels,
        });
    }

    let mut seen_gestalt_activities = BTreeSet::new();
    for action in plan.gestalt_activities {
        if !seen_gestalt_activities.insert(action.gestalt_id.clone())
            || (!canonical_composition && !legacy_seen_gestalts.insert(action.gestalt_id.clone()))
        {
            return Err(KernelError::Invalid(
                "gestalt acts twice in one strategic tick".into(),
            ));
        }
        validate_public_channels(&action.public_channels)?;
        let gestalt = campaign
            .gestalts
            .get(&action.gestalt_id)
            .ok_or_else(|| KernelError::Invalid("strategic plan invented a gestalt".into()))?;
        let profile = campaign
            .agency_profiles
            .get(&action.gestalt_id)
            .ok_or_else(|| KernelError::Invalid("strategic gestalt lacks agency scope".into()))?;
        let allowed_targets =
            crate::resolution::strategic_activity_targets(campaign, &action.gestalt_id);
        let unique_targets = action.target_subject_ids.iter().collect::<BTreeSet<_>>();
        let unique_locations = action.location_ids.iter().collect::<BTreeSet<_>>();
        let needs_target = !action.activity.allows_targetless_local_attempt();
        if action.target_subject_ids.len() > 4
            || unique_targets.len() != action.target_subject_ids.len()
            || action
                .target_subject_ids
                .iter()
                .any(|target| !allowed_targets.contains(target))
            || (needs_target && action.target_subject_ids.is_empty())
            || action.location_ids.len() > 4
            || unique_locations.len() != action.location_ids.len()
            || action.location_ids.iter().any(|location| {
                !profile.location_ids.contains(location)
                    && prospective_gestalt_locations.get(&action.gestalt_id) != Some(location)
            })
        {
            return Err(KernelError::Invalid(
                "strategic gestalt activity exceeds exact graph or location scope".into(),
            ));
        }
        let target_names = action
            .target_subject_ids
            .iter()
            .map(|target| agency_subject_name(campaign, target))
            .collect::<Result<Vec<_>, _>>()?;
        let locations = if action.location_ids.is_empty() {
            vec![gestalt.home_location_id.clone()]
        } else {
            action.location_ids
        };
        let institution_ids = action
            .target_subject_ids
            .iter()
            .filter(|target| campaign.institutions.contains_key(*target))
            .cloned()
            .collect();
        let actor_ids = action
            .target_subject_ids
            .iter()
            .filter(|target| {
                campaign.actors.contains_key(*target)
                    || target
                        .strip_prefix("member:")
                        .is_some_and(|member_id| campaign.gestalt_members.contains_key(member_id))
            })
            .cloned()
            .collect();
        let mut gestalt_ids = vec![action.gestalt_id.clone()];
        gestalt_ids.extend(
            action
                .target_subject_ids
                .iter()
                .filter(|target| campaign.gestalts.contains_key(*target))
                .cloned(),
        );
        events.push(crate::domain::Event {
            id: format!(
                "strategic:{revision}:gestalt-activity:{}",
                action.gestalt_id
            ),
            at,
            kind: "gestalt_activity".into(),
            summary: strategic_activity_summary(&gestalt.name, &action.activity, &target_names),
            actor_ids,
            institution_ids,
            gestalt_ids,
            location_ids: locations,
            public_channels: action.public_channels,
        });
    }

    let mut legacy_seen_actors = BTreeSet::new();
    let mut seen_actor_moves = BTreeSet::new();
    for action in plan.actor_moves {
        if !seen_actor_moves.insert(action.actor_id.clone())
            || (!canonical_composition && !legacy_seen_actors.insert(action.actor_id.clone()))
        {
            return Err(KernelError::Invalid(
                "actor moves twice in one strategic tick".into(),
            ));
        }
        if is_human_controlled_actor(campaign, &action.actor_id) {
            return Err(KernelError::Invalid(
                "strategic simulation cannot puppet a human-controlled actor".into(),
            ));
        }
        validate_public_channels(&action.public_channels)?;
        let actor = campaign
            .actors
            .get(&action.actor_id)
            .ok_or_else(|| KernelError::Invalid("strategic plan invented an actor".into()))?;
        let route = campaign
            .locations
            .get(&actor.location_id)
            .and_then(|location| {
                location
                    .routes
                    .values()
                    .find(|route| route.destination_id == action.destination_id)
            })
            .ok_or_else(|| {
                KernelError::Invalid("strategic actor movement has no direct route".into())
            })?;
        if route.travel_minutes > campaign.tick_hours.saturating_mul(60) {
            return Err(KernelError::Invalid(
                "strategic actor movement exceeds the tick duration".into(),
            ));
        }
        let origin = actor.location_id.clone();
        let actor_name = actor.name.clone();
        events.push(crate::domain::Event {
            id: format!("strategic:{revision}:actor:{}", action.actor_id),
            at,
            kind: "actor_movement".into(),
            summary: format!(
                "{actor_name} moves from {origin} to {}.",
                action.destination_id
            ),
            actor_ids: vec![action.actor_id],
            institution_ids: vec![],
            gestalt_ids: vec![],
            location_ids: vec![origin, action.destination_id],
            public_channels: action.public_channels,
        });
    }

    let mut seen_actor_activities = BTreeSet::new();
    for action in plan.actor_activities {
        if !seen_actor_activities.insert(action.actor_id.clone())
            || (!canonical_composition && !legacy_seen_actors.insert(action.actor_id.clone()))
        {
            return Err(KernelError::Invalid(
                "actor acts twice in one strategic tick".into(),
            ));
        }
        if is_human_controlled_actor(campaign, &action.actor_id) {
            return Err(KernelError::Invalid(
                "strategic simulation cannot puppet a human-controlled actor".into(),
            ));
        }
        validate_public_channels(&action.public_channels)?;
        let actor = campaign
            .actors
            .get(&action.actor_id)
            .ok_or_else(|| KernelError::Invalid("strategic plan invented an actor".into()))?;
        let allowed_targets =
            crate::resolution::strategic_activity_targets(campaign, &action.actor_id);
        let unique_targets = action.target_subject_ids.iter().collect::<BTreeSet<_>>();
        let needs_target = !action.activity.allows_targetless_local_attempt();
        if action.target_subject_ids.len() > 4
            || unique_targets.len() != action.target_subject_ids.len()
            || action
                .target_subject_ids
                .iter()
                .any(|target| !allowed_targets.contains(target))
            || (needs_target && action.target_subject_ids.is_empty())
            || action.location_ids.len() != 1
            || (action.location_ids[0] != actor.location_id
                && prospective_actor_locations.get(&action.actor_id) != action.location_ids.first())
        {
            return Err(KernelError::Invalid(
                "strategic actor activity exceeds exact graph or location scope".into(),
            ));
        }
        let target_names = action
            .target_subject_ids
            .iter()
            .map(|target| agency_subject_name(campaign, target))
            .collect::<Result<Vec<_>, _>>()?;
        let institution_ids = action
            .target_subject_ids
            .iter()
            .filter(|target| campaign.institutions.contains_key(*target))
            .cloned()
            .collect();
        let mut actor_ids = vec![action.actor_id.clone()];
        actor_ids.extend(
            action
                .target_subject_ids
                .iter()
                .filter(|target| {
                    campaign.actors.contains_key(*target)
                        || target.strip_prefix("member:").is_some_and(|member_id| {
                            campaign.gestalt_members.contains_key(member_id)
                        })
                })
                .cloned(),
        );
        actor_ids.sort();
        actor_ids.dedup();
        let mut gestalt_ids = action
            .target_subject_ids
            .iter()
            .filter(|target| campaign.gestalts.contains_key(*target))
            .cloned()
            .collect::<Vec<_>>();
        gestalt_ids.sort();
        gestalt_ids.dedup();
        events.push(Event {
            id: format!("strategic:{revision}:actor-activity:{}", action.actor_id),
            at,
            kind: "actor_activity".into(),
            summary: strategic_activity_summary(&actor.name, &action.activity, &target_names),
            actor_ids,
            institution_ids,
            gestalt_ids,
            location_ids: action.location_ids,
            public_channels: action.public_channels,
        });
    }

    let mut legacy_seen_members = BTreeSet::new();
    let mut seen_member_activities = BTreeSet::new();
    for action in plan.member_activities {
        if !seen_member_activities.insert(action.member_id.clone())
            || (!canonical_composition && !legacy_seen_members.insert(action.member_id.clone()))
        {
            return Err(KernelError::Invalid(
                "gestalt member acts twice in one strategic tick".into(),
            ));
        }
        validate_public_channels(&action.public_channels)?;
        let member = campaign
            .gestalt_members
            .get(&action.member_id)
            .filter(|member| {
                member.materialized_actor_id.is_none()
                    && member.gestalt_id == action.source_gestalt_id
            })
            .ok_or_else(|| {
                KernelError::Invalid("strategic member activity has stale identity scope".into())
            })?;
        let allowed_targets =
            crate::resolution::member_activity_targets(campaign, &action.member_id)
                .map_err(|error| KernelError::Invalid(error.to_string()))?;
        let exact_location =
            crate::resolution::dormant_member_location(campaign, &action.member_id)
                .map_err(|error| KernelError::Invalid(error.to_string()))?;
        let unique_targets = action.target_subject_ids.iter().collect::<BTreeSet<_>>();
        let needs_target = !action.activity.allows_targetless_local_attempt();
        if action.target_subject_ids.len() > 4
            || unique_targets.len() != action.target_subject_ids.len()
            || action
                .target_subject_ids
                .iter()
                .any(|target| !allowed_targets.contains(target))
            || (needs_target && action.target_subject_ids.is_empty())
            || action.location_ids.len() != 1
            || (action.location_ids[0] != exact_location
                && prospective_member_locations.get(&action.member_id)
                    != action.location_ids.first())
        {
            return Err(KernelError::Invalid(
                "strategic member activity exceeds exact graph or location scope".into(),
            ));
        }
        let target_names = action
            .target_subject_ids
            .iter()
            .map(|target| agency_subject_name(campaign, target))
            .collect::<Result<Vec<_>, _>>()?;
        let institution_ids = action
            .target_subject_ids
            .iter()
            .filter(|target| campaign.institutions.contains_key(*target))
            .cloned()
            .collect();
        let mut actor_ids = vec![format!("member:{}", action.member_id)];
        actor_ids.extend(
            action
                .target_subject_ids
                .iter()
                .filter(|target| {
                    campaign.actors.contains_key(*target)
                        || target.strip_prefix("member:").is_some_and(|member_id| {
                            campaign.gestalt_members.contains_key(member_id)
                        })
                })
                .cloned(),
        );
        actor_ids.sort();
        actor_ids.dedup();
        let mut gestalt_ids = vec![action.source_gestalt_id.clone()];
        gestalt_ids.extend(
            action
                .target_subject_ids
                .iter()
                .filter(|target| campaign.gestalts.contains_key(*target))
                .cloned(),
        );
        gestalt_ids.sort();
        gestalt_ids.dedup();
        events.push(Event {
            id: format!("strategic:{revision}:member-activity:{}", action.member_id),
            at,
            kind: "gestalt_member_activity".into(),
            summary: strategic_activity_summary(&member.name, &action.activity, &target_names),
            actor_ids,
            institution_ids,
            gestalt_ids,
            location_ids: action.location_ids,
            public_channels: action.public_channels,
        });
    }
    let mut seen_member_migrations = BTreeSet::new();
    for action in plan.member_migrations {
        if !seen_member_migrations.insert(action.member_id.clone())
            || (!canonical_composition && !legacy_seen_members.insert(action.member_id.clone()))
        {
            return Err(KernelError::Invalid(
                "gestalt member migrates twice in one strategic tick".into(),
            ));
        }
        validate_public_channels(&action.public_channels)?;
        crate::resolution::validate_member_migration(
            campaign,
            &action.member_id,
            &action.source_gestalt_id,
            &action.destination_gestalt_id,
            &action.destination_location_id,
        )
        .map_err(|error| KernelError::Invalid(error.to_string()))?;
        let origin = campaign.gestalt_members[&action.member_id]
            .last_location_id
            .clone()
            .unwrap_or_else(|| {
                campaign.gestalts[&action.source_gestalt_id]
                    .home_location_id
                    .clone()
            });
        let member_name = campaign.gestalt_members[&action.member_id].name.clone();
        events.push(crate::domain::Event {
            id: format!("strategic:{revision}:member:{}", action.member_id),
            at,
            kind: "gestalt_member_migration".into(),
            summary: format!(
                "{member_name} moves from {origin} to {} and joins {}.",
                action.destination_location_id, action.destination_gestalt_id
            ),
            actor_ids: vec![format!("member:{}", action.member_id)],
            institution_ids: vec![],
            gestalt_ids: vec![action.source_gestalt_id, action.destination_gestalt_id],
            location_ids: vec![origin, action.destination_location_id],
            public_channels: action.public_channels,
        });
    }
    let plan_digest = crate::legacy_transition::digest_serializable(&lowering_plan)
        .map_err(|error| KernelError::Invalid(error.to_string()))?;
    let mutation = crate::legacy_transition::lower_strategic_wave(
        campaign,
        &lowering_plan,
        &format!("strategic-wave:{plan_digest}"),
        Utc::now() + Duration::minutes(5),
    )
    .map_err(|error| KernelError::Invalid(error.to_string()))?
    .map(|transition| {
        let receipt =
            crate::legacy_transition::apply_lowered_transition(&mut next, &transition, Utc::now())
                .map_err(|error| KernelError::Invalid(error.to_string()))?;
        Ok::<_, KernelError>((transition, receipt))
    })
    .transpose()?;
    for outcome in activity_outcomes {
        let (source_subject_id, locations, public_channels) = outcome_event_context
            .remove(&outcome.action_digest)
            .ok_or_else(|| {
                KernelError::Invalid("strategic outcome lost its activity context".into())
            })?;
        let mut subject_ids = BTreeSet::from([source_subject_id]);
        collect_outcome_subject_ids(&outcome.effect, &mut subject_ids);
        let mut actor_ids = Vec::new();
        let mut institution_ids = Vec::new();
        let mut gestalt_ids = Vec::new();
        for subject_id in subject_ids {
            if let Some(member_id) = subject_id.strip_prefix("member:") {
                actor_ids.push(subject_id.clone());
                if let Some(member) = next.gestalt_members.get(member_id) {
                    gestalt_ids.push(member.gestalt_id.clone());
                }
            } else if next.actors.contains_key(&subject_id) {
                actor_ids.push(subject_id);
            } else if next.institutions.contains_key(&subject_id) {
                institution_ids.push(subject_id);
            } else if next.gestalts.contains_key(&subject_id) {
                gestalt_ids.push(subject_id);
            }
        }
        actor_ids.sort();
        actor_ids.dedup();
        institution_ids.sort();
        institution_ids.dedup();
        gestalt_ids.sort();
        gestalt_ids.dedup();
        let digest_suffix = outcome
            .action_digest
            .strip_prefix("sha256:")
            .unwrap_or(&outcome.action_digest)
            .chars()
            .take(16)
            .collect::<String>();
        events.push(Event {
            id: format!("strategic:{revision}:activity-outcome:{digest_suffix}"),
            at,
            kind: "strategic_activity_outcome".into(),
            summary: outcome.summary,
            actor_ids,
            institution_ids,
            gestalt_ids,
            location_ids: locations,
            public_channels,
        });
    }
    *campaign = next;
    Ok(AppliedStrategicTickPlan { events, mutation })
}

fn collect_outcome_subject_ids(
    effect: &crate::domain::StrategicOutcomeEffect,
    subjects: &mut BTreeSet<String>,
) {
    use crate::domain::StrategicOutcomeEffect;
    match effect {
        StrategicOutcomeEffect::NoMaterialChange { .. } => {}
        StrategicOutcomeEffect::ResourceCreated {
            owner_subject_id, ..
        }
        | StrategicOutcomeEffect::ResourceConsumed {
            owner_subject_id, ..
        }
        | StrategicOutcomeEffect::KnowledgeLearned {
            owner_subject_id, ..
        } => {
            subjects.insert(owner_subject_id.clone());
        }
        StrategicOutcomeEffect::ResourceTransferred {
            from_subject_id,
            to_subject_id,
            ..
        } => {
            subjects.insert(from_subject_id.clone());
            subjects.insert(to_subject_id.clone());
        }
        StrategicOutcomeEffect::GestaltPressure { gestalt_id, .. } => {
            subjects.insert(gestalt_id.clone());
        }
        StrategicOutcomeEffect::AgencyRelationShift { .. } => {}
        StrategicOutcomeEffect::MemberMemory { member_id, .. }
        | StrategicOutcomeEffect::MemberObligation { member_id, .. }
        | StrategicOutcomeEffect::MemberRelationship { member_id, .. } => {
            subjects.insert(format!("member:{member_id}"));
            if let StrategicOutcomeEffect::MemberRelationship {
                other_subject_id, ..
            } = effect
            {
                subjects.insert(other_subject_id.clone());
            }
        }
    }
}

fn strategic_activity_summary(
    source_name: &str,
    activity: &StrategicActivityKind,
    target_names: &[String],
) -> String {
    let targets = target_names.join(", ");
    match (activity, target_names.is_empty()) {
        (StrategicActivityKind::Prepare, true) => {
            format!("{source_name} undertakes preparations.")
        }
        (StrategicActivityKind::Prepare, false) => {
            format!("{source_name} undertakes preparations concerning {targets}.")
        }
        (StrategicActivityKind::Coordinate, false) => {
            format!("{source_name} attempts to coordinate with {targets}.")
        }
        (StrategicActivityKind::Investigate, true) => {
            format!("{source_name} begins a local investigation.")
        }
        (StrategicActivityKind::Investigate, false) => {
            format!("{source_name} begins investigating {targets}.")
        }
        (StrategicActivityKind::Recruit, false) => {
            format!("{source_name} attempts recruitment involving {targets}.")
        }
        (StrategicActivityKind::Obstruct, false) => {
            format!("{source_name} attempts to obstruct {targets}.")
        }
        (StrategicActivityKind::Obstruct, true) => {
            format!("{source_name} attempts local interference.")
        }
        (StrategicActivityKind::Trade, false) => {
            format!("{source_name} offers a trade to {targets}.")
        }
        (StrategicActivityKind::Communicate, true) => {
            format!("{source_name} attempts a local communication.")
        }
        (StrategicActivityKind::Communicate, false) => {
            format!("{source_name} sends a communication to {targets}.")
        }
        _ => unreachable!("validated strategic activity requires a target"),
    }
}

fn agency_subject_name(campaign: &Campaign, subject_id: &str) -> Result<String, KernelError> {
    campaign
        .actors
        .get(subject_id)
        .map(|value| value.name.clone())
        .or_else(|| {
            campaign
                .institutions
                .get(subject_id)
                .map(|value| value.name.clone())
        })
        .or_else(|| {
            campaign
                .gestalts
                .get(subject_id)
                .map(|value| value.name.clone())
        })
        .or_else(|| {
            crate::resolution::dormant_member_id_for_subject(campaign, subject_id).and_then(
                |member_id| {
                    campaign
                        .gestalt_members
                        .get(member_id)
                        .map(|value| value.name.clone())
                },
            )
        })
        .ok_or_else(|| KernelError::Invalid("strategic activity target vanished".into()))
}

fn deterministic_strategic_tick_plan() -> StrategicTickPlan {
    // Deterministic time and clock obligations are lowered by the same wave
    // transition. Without an admitted Persona wave there is no authority to
    // invent actor or institution activity.
    StrategicTickPlan::default()
}

fn validate_public_channels(channels: &[String]) -> Result<(), KernelError> {
    if channels.len() > 8
        || channels
            .iter()
            .any(|channel| !crate::resolution::information_channel_is_concrete(channel))
    {
        return Err(KernelError::Invalid(
            "strategic action has invalid information channels".into(),
        ));
    }
    Ok(())
}

fn valid_sha256(value: &str) -> bool {
    value
        .strip_prefix("sha256:")
        .is_some_and(|digest| digest.len() == 64 && digest.bytes().all(|b| b.is_ascii_hexdigit()))
}

fn commit_gestalt_presence(
    store: &CampaignStore,
    row: cultcache_legacy::CultCacheEnvelope,
    before: Campaign,
    mut campaign: Campaign,
    kind: &str,
    reason: &str,
) -> Result<CommandResult, KernelError> {
    let previous_resolution_epoch = campaign.resolution_policy.resolution_epoch;
    crate::resolution::ensure_agency_profiles(&mut campaign);
    let mut changes = Vec::new();
    for (member_id, member) in &campaign.gestalt_members {
        let previous_actor = before
            .gestalt_members
            .get(member_id)
            .and_then(|value| value.materialized_actor_id.as_deref());
        let current_actor = member.materialized_actor_id.as_deref();
        if previous_actor == current_actor {
            continue;
        }
        let (operation, actor_id) = match (previous_actor, current_actor) {
            (None, Some(actor)) => ("materialized", actor),
            (Some(actor), None) => ("dematerialized", actor),
            (Some(_), Some(actor)) => ("rematerialized", actor),
            (None, None) => unreachable!(),
        };
        let gestalt = campaign
            .gestalts
            .get(&member.gestalt_id)
            .ok_or_else(|| KernelError::Invalid("gestalt receipt lost its baseline".into()))?;
        changes.push(crate::domain::GestaltPresenceChange {
            operation: operation.into(),
            gestalt_id: member.gestalt_id.clone(),
            member_id: member_id.clone(),
            actor_id: actor_id.into(),
            gestalt_version: gestalt.version,
            member_version: member.version,
        });
    }
    if changes.is_empty() {
        return Err(KernelError::Invalid(
            "gestalt presence command produced no presence change".into(),
        ));
    }
    let previous_revision = campaign.revision;
    campaign.revision += 1;
    campaign.resolution_policy.resolution_epoch = previous_resolution_epoch.saturating_add(1);
    campaign.resolution_cover = None;
    let committed_at = Utc::now();
    let receipt = WorldCommitReceipt {
        schema: "ghostlight.world_commit_receipt.v1".into(),
        campaign_id: campaign.id,
        previous_revision,
        revision: campaign.revision,
        command_kind: kind.into(),
        committed_at,
        roll: None,
    };
    let gestalt_receipt = crate::domain::GestaltMaterializationReceipt {
        schema: "ghostlight.gestalt_materialization_receipt.v1".into(),
        campaign_id: campaign.id,
        previous_revision,
        revision: campaign.revision,
        previous_resolution_epoch,
        resolution_epoch: campaign.resolution_policy.resolution_epoch,
        reason: reason.into(),
        changes,
        committed_at,
    };
    let key = format!("{}-{}", campaign.id, campaign.revision);
    store
        .append_gestalt_presence(&row, &campaign, &key, &receipt, &gestalt_receipt)
        .map_err(persist)?;
    Ok(CommandResult::Committed { campaign, receipt })
}

fn commit_strategic_tick(
    store: &CampaignStore,
    row: cultcache_legacy::CultCacheEnvelope,
    mut campaign: Campaign,
    source: TickSource,
    model_receipt_hash: Option<String>,
    event_ids: Vec<String>,
    resolution_wave: Option<ResolutionWaveCommit>,
    mutation: Option<(
        crate::legacy_transition::LoweredLegacyTransition,
        crate::transition::WorldMutationReceipt,
    )>,
) -> Result<CommandResult, KernelError> {
    crate::resolution::ensure_agency_profiles(&mut campaign);
    let previous_revision = campaign.revision;
    campaign.revision += 1;
    let committed_at = Utc::now();
    let receipt = WorldCommitReceipt {
        schema: "ghostlight.world_commit_receipt.v1".into(),
        campaign_id: campaign.id,
        previous_revision,
        revision: campaign.revision,
        command_kind: "strategic_tick".into(),
        committed_at,
        roll: None,
    };
    let strategic = crate::domain::StrategicTickReceipt {
        schema: "ghostlight.strategic_tick.v1".into(),
        campaign_id: campaign.id,
        previous_revision,
        revision: campaign.revision,
        source,
        model_receipt_hash,
        model_receipt_hashes: resolution_wave
            .as_ref()
            .map(|wave| wave.model_receipt_hashes.clone())
            .unwrap_or_default(),
        resolution_epoch: resolution_wave.as_ref().map(|wave| wave.resolution_epoch),
        resolution_cover_id: resolution_wave.as_ref().map(|wave| {
            format!(
                "{}:{}:{}",
                campaign.id, wave.world_revision, wave.resolution_epoch
            )
        }),
        event_ids,
        committed_at,
    };
    let key = format!("{}-{}", campaign.id, campaign.revision);
    store
        .append_strategic_tick(
            &row,
            &campaign,
            &key,
            &receipt,
            &strategic,
            resolution_wave.as_ref(),
            mutation
                .as_ref()
                .map(|(transition, receipt)| (&transition.authority, &transition.batch, receipt)),
        )
        .map_err(persist)?;
    Ok(CommandResult::Committed { campaign, receipt })
}

fn commit_resolution_control(
    store: &CampaignStore,
    row: cultcache_legacy::CultCacheEnvelope,
    campaign: Campaign,
    previous_resolution_epoch: u64,
    operation: &str,
) -> Result<CommandResult, KernelError> {
    let receipt = ResolutionControlReceipt {
        schema: "ghostlight.resolution_control_receipt.v1".into(),
        campaign_id: campaign.id,
        world_revision: campaign.revision,
        previous_resolution_epoch,
        resolution_epoch: campaign.resolution_policy.resolution_epoch,
        provider_configuration_epoch: campaign.resolution_policy.provider_configuration_epoch,
        operation: operation.into(),
        active_cell_budget: campaign.resolution_policy.active_cell_budget,
        pin_ids: campaign.resolution_pins.keys().cloned().collect(),
        committed_at: Utc::now(),
    };
    store
        .append_resolution_control(&row, &campaign, &receipt)
        .map_err(persist)?;
    Ok(CommandResult::ResolutionUpdated { campaign, receipt })
}

fn commit(
    store: &CampaignStore,
    row: cultcache_legacy::CultCacheEnvelope,
    mut campaign: Campaign,
    kind: &str,
    roll: Option<RollReceipt>,
) -> Result<CommandResult, KernelError> {
    crate::resolution::ensure_agency_profiles(&mut campaign);
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
        .append_world_transition(
            &row,
            "ghostlight.campaign.v1",
            &campaign,
            &format!("{}-{}", campaign.id, campaign.revision),
            &receipt,
        )
        .map_err(persist)?;
    Ok(CommandResult::Committed { campaign, receipt })
}

fn commit_mutation_transition(
    store: &CampaignStore,
    row: cultcache_legacy::CultCacheEnvelope,
    mut campaign: Campaign,
    kind: &str,
    roll: Option<RollReceipt>,
    transition: crate::legacy_transition::LoweredLegacyTransition,
    mutation_receipt: crate::transition::WorldMutationReceipt,
) -> Result<CommandResult, KernelError> {
    crate::resolution::ensure_agency_profiles(&mut campaign);
    let previous_revision = campaign.revision;
    campaign.revision += 1;
    if mutation_receipt.previous_world_revision != previous_revision
        || mutation_receipt.world_revision != campaign.revision
    {
        return Err(KernelError::Invalid(
            "mutation receipt does not bind the campaign transition".into(),
        ));
    }
    let receipt = WorldCommitReceipt {
        schema: "ghostlight.world_commit_receipt.v1".into(),
        campaign_id: campaign.id,
        previous_revision,
        revision: campaign.revision,
        command_kind: kind.into(),
        committed_at: mutation_receipt.committed_at,
        roll,
    };
    store
        .append_world_transition_with_mutation(
            &row,
            "ghostlight.campaign.v1",
            &campaign,
            &format!("{}-{}", campaign.id, campaign.revision),
            &receipt,
            &transition.authority,
            &transition.batch,
            &mutation_receipt,
        )
        .map_err(persist)?;
    Ok(CommandResult::Committed { campaign, receipt })
}

fn load_campaign_membership(
    store: &CampaignStore,
    campaign_id: uuid::Uuid,
) -> Result<CampaignMembership, KernelError> {
    store
        .load::<CampaignMembership>("campaign_membership.v1", &campaign_id.to_string())
        .map_err(persist)?
        .map(|(_, membership)| membership)
        .ok_or_else(|| KernelError::Invalid("campaign membership is missing".into()))
}

fn optional_campaign_membership(
    store: &CampaignStore,
    campaign_id: uuid::Uuid,
) -> Result<Option<CampaignMembership>, KernelError> {
    store
        .load::<CampaignMembership>("campaign_membership.v1", &campaign_id.to_string())
        .map_err(persist)
        .map(|value| value.map(|(_, membership)| membership))
}

fn validate_bounded_coop_effect(
    store: &CampaignStore,
    campaign: &Campaign,
    acting_actor_id: &str,
    effect: &WorldEffectDelta,
) -> Result<(), KernelError> {
    let Some(membership) = optional_campaign_membership(store, campaign.id)? else {
        return Ok(());
    };
    let controlled = membership.controlled_actor_ids();
    if controlled.len() < 2 || !controlled.contains(acting_actor_id) {
        return Ok(());
    }
    if !effect.actor_moves.is_empty() {
        return Err(KernelError::Invalid(
            "co-op travel requires a unanimous group-travel proposal".into(),
        ));
    }
    let mut protected_targets = BTreeSet::new();
    protected_targets.extend(effect.actor_conditions.keys().cloned());
    protected_targets.extend(effect.actor_commitments.keys().cloned());
    protected_targets.extend(effect.actor_knowledge_additions.keys().cloned());
    protected_targets.extend(effect.actor_observations.keys().cloned());
    protected_targets.extend(effect.actor_relationship_updates.keys().cloned());
    if protected_targets
        .iter()
        .any(|target| target != acting_actor_id && controlled.contains(target))
    {
        return Err(KernelError::Invalid(
            "player-versus-player effects are unsupported in bounded co-op".into(),
        ));
    }
    Ok(())
}

fn stable_channel_id(channel: &str) -> String {
    format!("{:x}", Sha256::digest(channel.as_bytes()))[..12].to_string()
}

fn require_active_member<'a>(
    membership: &'a CampaignMembership,
    member_id: &str,
) -> Result<&'a crate::session_zero::CampaignMember, KernelError> {
    membership
        .members
        .get(member_id)
        .filter(|member| member.active)
        .ok_or_else(|| KernelError::Invalid("campaign member is not active".into()))
}

fn active_member_ids(membership: &CampaignMembership) -> BTreeSet<String> {
    membership
        .members
        .values()
        .filter(|member| member.active)
        .map(|member| member.member_id.clone())
        .collect()
}

fn commit_governed_time_advance(
    store: &CampaignStore,
    campaign_row: cultcache_legacy::CultCacheEnvelope,
    mut campaign: Campaign,
    proposal_row: cultcache_legacy::CultCacheEnvelope,
    mut proposal: TimeAdvanceProposal,
) -> Result<CommandResult, KernelError> {
    let transition = crate::legacy_transition::lower_time_advance(
        &campaign,
        proposal.minutes,
        crate::transition::MutationProcedure::Governance,
        &proposal.id,
        Utc::now() + Duration::minutes(5),
    )
    .map_err(|error| KernelError::Invalid(error.to_string()))?;
    let mutation_receipt =
        crate::legacy_transition::apply_lowered_transition(&mut campaign, &transition, Utc::now())
            .map_err(|error| KernelError::Invalid(error.to_string()))?;
    campaign.last_player_activity = Utc::now();
    campaign.away_ticks_processed = 0;
    campaign.pending_ticks = 0;
    crate::resolution::ensure_agency_profiles(&mut campaign);
    let previous_revision = campaign.revision;
    campaign.revision = campaign.revision.saturating_add(1);
    proposal.committed = true;
    let receipt = WorldCommitReceipt {
        schema: "ghostlight.world_commit_receipt.v1".into(),
        campaign_id: campaign.id,
        previous_revision,
        revision: campaign.revision,
        command_kind: "unanimous_time_advance".into(),
        committed_at: mutation_receipt.committed_at,
        roll: None,
    };
    store
        .commit_time_advance(
            &campaign_row,
            &campaign,
            &proposal_row,
            &proposal,
            &receipt,
            (&transition.authority, &transition.batch, &mutation_receipt),
        )
        .map_err(persist)?;
    Ok(CommandResult::Committed { campaign, receipt })
}

fn commit_governed_group_travel(
    store: &CampaignStore,
    campaign_row: cultcache_legacy::CultCacheEnvelope,
    mut campaign: Campaign,
    proposal_row: cultcache_legacy::CultCacheEnvelope,
    mut proposal: GroupTravelProposal,
    membership: &CampaignMembership,
) -> Result<CommandResult, KernelError> {
    let active_actor_ids = membership.controlled_actor_ids();
    if active_actor_ids.iter().any(|actor_id| {
        campaign
            .actors
            .get(actor_id)
            .is_none_or(|actor| actor.location_id != proposal.origin_location_id)
    }) {
        return Err(KernelError::Invalid(
            "group-travel proposal no longer matches the shared scene".into(),
        ));
    }
    let route_minutes = campaign
        .locations
        .get(&proposal.origin_location_id)
        .and_then(|location| {
            location
                .routes
                .values()
                .find(|route| route.destination_id == proposal.destination_location_id)
                .map(|route| route.travel_minutes)
        })
        .ok_or_else(|| KernelError::Invalid("group-travel route no longer exists".into()))?;
    if route_minutes != proposal.travel_minutes {
        return Err(KernelError::Invalid(
            "group-travel route changed after proposal".into(),
        ));
    }
    let transition = crate::legacy_transition::lower_group_travel(
        &campaign,
        &active_actor_ids,
        &proposal.origin_location_id,
        &proposal.destination_location_id,
        route_minutes,
        &proposal.id,
        Utc::now() + Duration::minutes(5),
    )
    .map_err(|error| KernelError::Invalid(error.to_string()))?;
    let mutation_receipt =
        crate::legacy_transition::apply_lowered_transition(&mut campaign, &transition, Utc::now())
            .map_err(|error| KernelError::Invalid(error.to_string()))?;
    crate::resolution::ensure_agency_profiles(&mut campaign);
    campaign.last_player_activity = Utc::now();
    campaign.away_ticks_processed = 0;
    campaign.pending_ticks = 0;
    let previous_revision = campaign.revision;
    campaign.revision = campaign.revision.saturating_add(1);
    campaign.events.push(Event {
        id: format!("group-travel:{}", campaign.revision),
        at: campaign.world_time,
        kind: "group_travel".into(),
        summary: format!(
            "The party travels from {} to {}.",
            proposal.origin_location_id, proposal.destination_location_id
        ),
        actor_ids: active_actor_ids.into_iter().collect(),
        institution_ids: vec![],
        gestalt_ids: vec![],
        location_ids: vec![
            proposal.origin_location_id.clone(),
            proposal.destination_location_id.clone(),
        ],
        public_channels: vec![],
    });
    proposal.committed = true;
    let receipt = WorldCommitReceipt {
        schema: "ghostlight.world_commit_receipt.v1".into(),
        campaign_id: campaign.id,
        previous_revision,
        revision: campaign.revision,
        command_kind: "unanimous_group_travel".into(),
        committed_at: mutation_receipt.committed_at,
        roll: None,
    };
    store
        .commit_group_travel(
            &campaign_row,
            &campaign,
            &proposal_row,
            &proposal,
            &receipt,
            (&transition.authority, &transition.batch, &mutation_receipt),
        )
        .map_err(persist)?;
    Ok(CommandResult::Committed { campaign, receipt })
}

fn commit_governed_cell_budget(
    store: &CampaignStore,
    campaign_row: cultcache_legacy::CultCacheEnvelope,
    mut campaign: Campaign,
    proposal_row: cultcache_legacy::CultCacheEnvelope,
    mut proposal: CellBudgetProposal,
) -> Result<CommandResult, KernelError> {
    let previous_epoch = campaign.resolution_policy.resolution_epoch;
    campaign.resolution_policy.active_cell_budget = proposal.active_cell_budget;
    campaign.resolution_policy.pending_active_cell_budget = None;
    campaign.resolution_policy.resolution_epoch = previous_epoch.saturating_add(1);
    campaign.resolution_cover = None;
    crate::resolution::validate_policy(&campaign.resolution_policy)
        .map_err(|error| KernelError::Invalid(error.to_string()))?;
    proposal.committed = true;
    let receipt = ResolutionControlReceipt {
        schema: "ghostlight.resolution_control_receipt.v1".into(),
        campaign_id: campaign.id,
        world_revision: campaign.revision,
        previous_resolution_epoch: previous_epoch,
        resolution_epoch: campaign.resolution_policy.resolution_epoch,
        provider_configuration_epoch: campaign.resolution_policy.provider_configuration_epoch,
        operation: "unanimous_set_active_cell_budget".into(),
        active_cell_budget: campaign.resolution_policy.active_cell_budget,
        pin_ids: campaign.resolution_pins.keys().cloned().collect(),
        committed_at: Utc::now(),
    };
    store
        .commit_cell_budget(&campaign_row, &campaign, &proposal_row, &proposal, &receipt)
        .map_err(persist)?;
    Ok(CommandResult::ResolutionUpdated { campaign, receipt })
}

fn commit_with_records(
    store: &CampaignStore,
    row: cultcache_legacy::CultCacheEnvelope,
    mut campaign: Campaign,
    kind: &str,
    evidence: Vec<VaultEvidenceReceipt>,
    candidates: Vec<CanonCandidate>,
    model_receipts: Vec<crate::model::ModelStageReceipt>,
    mutation: Option<(
        crate::legacy_transition::LoweredLegacyTransition,
        crate::transition::WorldMutationReceipt,
    )>,
) -> Result<CommandResult, KernelError> {
    crate::resolution::ensure_agency_profiles(&mut campaign);
    let previous_revision = campaign.revision;
    campaign.revision += 1;
    if let Some((_, mutation_receipt)) = &mutation
        && (mutation_receipt.previous_world_revision != previous_revision
            || mutation_receipt.world_revision != campaign.revision)
    {
        return Err(KernelError::Invalid(
            "mutation receipt does not bind the recorded world commit".into(),
        ));
    }
    let receipt = WorldCommitReceipt {
        schema: "ghostlight.world_commit_receipt.v1".into(),
        campaign_id: campaign.id,
        previous_revision,
        revision: campaign.revision,
        command_kind: kind.into(),
        committed_at: mutation
            .as_ref()
            .map(|(_, receipt)| receipt.committed_at)
            .unwrap_or_else(Utc::now),
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
            &model_receipts,
            mutation
                .as_ref()
                .map(|(transition, receipt)| (&transition.authority, &transition.batch, receipt)),
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
    use sha2::{Digest, Sha256};
    use std::collections::{BTreeMap, BTreeSet};

    fn test_action_digest(label: &str) -> String {
        format!("sha256:{:x}", Sha256::digest(label.as_bytes()))
    }

    fn resolve_test_activities(mut plan: StrategicTickPlan) -> StrategicTickPlan {
        plan.activity_outcomes.extend(
            plan.gestalt_activities
                .iter()
                .map(|activity| (activity.action_digest.clone(), activity.gestalt_id.clone()))
                .chain(
                    plan.actor_activities.iter().map(|activity| {
                        (activity.action_digest.clone(), activity.actor_id.clone())
                    }),
                )
                .chain(plan.member_activities.iter().map(|activity| {
                    (
                        activity.action_digest.clone(),
                        format!("member:{}", activity.member_id),
                    )
                }))
                .map(
                    |(action_digest, source_subject_id)| StrategicActivityOutcome {
                        schema: "ghostlight.strategic_activity_outcome.v1".into(),
                        action_digest,
                        source_subject_id,
                        band: StrategicOutcomeBand::Mixed,
                        summary: "The attempt produces no durable material change.".into(),
                        supporting_state_references: vec![],
                        effect: StrategicOutcomeEffect::NoMaterialChange {
                            reason: "No response is established in this test snapshot.".into(),
                        },
                    },
                ),
        );
        plan
    }

    fn assert_only_strategic_obligations_advanced(before: &Campaign, after: &Campaign) {
        let mut expected = before.clone();
        expected.world_time += Duration::hours(i64::from(expected.tick_hours));
        for clock in expected.clocks.values_mut() {
            clock.progress = clock.progress.saturating_add(1).min(clock.threshold);
        }
        assert_eq!(after, &expected);
    }

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
            agency_profiles: BTreeMap::new(),
            agency_relations: BTreeMap::new(),
            gestalt_lineages: BTreeMap::new(),
            resolution_policy: Default::default(),
            resolution_pins: BTreeMap::new(),
            resolution_cover: None,
            strategic_tick_count: 0,
        }
    }

    #[tokio::test]
    async fn resolved_outcome_cannot_admit_an_unrequested_gestalt_member() {
        let dir = tempfile::tempdir().unwrap();
        let store = CampaignStore::open(dir.path().join("campaign.cc")).unwrap();
        let kernel = WorldKernel::start(store.clone());
        let mut seed = campaign();
        seed.transcript.push(NarrativeTurn {
            revision: 0,
            at: Utc::now(),
            speaker: "world".into(),
            text: "They agree to help design the relay.".into(),
            persona_response_actor_ids: BTreeSet::new(),
        });
        seed.gestalts.insert(
            "refugees".into(),
            GestaltPersonaState {
                schema: "ghostlight.gestalt_persona_state.v1".into(),
                id: "refugees".into(),
                name: "Refugees".into(),
                version: 0,
                home_location_id: "room".into(),
                shared_capabilities: BTreeSet::new(),
                shared_knowledge: BTreeSet::new(),
                resources: BTreeSet::new(),
                goals: vec![],
                pressures: vec![],
            },
        );
        crate::resolution::ensure_agency_profiles(&mut seed);
        let campaign_id = seed.id;
        kernel
            .command(WorldCommand::CreateCampaign {
                campaign: seed,
                evidence_receipts: vec![],
                model_stage_receipts: vec![],
            })
            .await
            .unwrap();

        let member = GestaltMemberDelta {
            schema: "ghostlight.gestalt_member_delta.v1".into(),
            id: "relay-volunteer".into(),
            gestalt_id: "refugees".into(),
            version: 0,
            name: "Relay Volunteer".into(),
            capability_additions: BTreeSet::new(),
            capability_removals: BTreeSet::new(),
            knowledge_additions: BTreeSet::new(),
            knowledge_removals: BTreeSet::new(),
            equipment: BTreeSet::new(),
            conditions: BTreeSet::new(),
            obligations: BTreeSet::new(),
            relationships: BTreeMap::new(),
            goals: vec!["help with the relay".into()],
            memories: vec!["agreed to help".into()],
            last_location_id: Some("room".into()),
            materialized_actor_id: None,
            last_relevant_revision: 0,
            relevance_lease_until_revision: 0,
        };
        let error = kernel
            .command(WorldCommand::ReconcileGestaltPresence {
                expected_revision: 0,
                reason: "They agree to help design the relay.".into(),
                plan: GestaltPresencePlan {
                    individuations: vec![GestaltIndividuation {
                        gestalt_id: "refugees".into(),
                        expected_gestalt_version: 0,
                        member,
                        location_id: "room".into(),
                    }],
                    promotions: vec![],
                    demotions: vec![],
                },
            })
            .await
            .unwrap_err();
        assert!(
            error
                .to_string()
                .contains("immediately committed player speech")
        );
        let persisted = store
            .load::<Campaign>("campaign.v1", &campaign_id.to_string())
            .unwrap()
            .unwrap()
            .1;
        assert_eq!(persisted.revision, 0);
        assert!(persisted.gestalt_members.is_empty());
        assert_eq!(persisted.transcript.len(), 1);
    }

    #[tokio::test]
    async fn kernel_rejects_individuation_that_duplicates_addressed_actor_without_mutation() {
        let dir = tempfile::tempdir().unwrap();
        let store = CampaignStore::open(dir.path().join("campaign.cc")).unwrap();
        let kernel = WorldKernel::start(store.clone());
        let mut seed = campaign();
        seed.actors.insert(
            "taren".into(),
            ActorState {
                id: "taren".into(),
                name: "Taren".into(),
                location_id: "room".into(),
                capabilities: BTreeSet::new(),
                knowledge: BTreeSet::new(),
                equipment: BTreeSet::new(),
                conditions: BTreeSet::new(),
                obligations: BTreeSet::new(),
                relationships: BTreeMap::new(),
                goals: vec![],
                memories: vec![],
            },
        );
        seed.gestalts.insert(
            "refugees".into(),
            GestaltPersonaState {
                schema: "ghostlight.gestalt_persona_state.v1".into(),
                id: "refugees".into(),
                name: "Refugees".into(),
                version: 0,
                home_location_id: "room".into(),
                shared_capabilities: BTreeSet::new(),
                shared_knowledge: BTreeSet::new(),
                resources: BTreeSet::new(),
                goals: vec![],
                pressures: vec![],
            },
        );
        let speech = "Taren, tell me whether the regulator is holding.";
        let reason = format!("{} says: {speech}", seed.player_actor_id);
        seed.transcript.push(NarrativeTurn {
            revision: 0,
            at: Utc::now(),
            speaker: seed.player_actor_id.clone(),
            text: speech.into(),
            persona_response_actor_ids: BTreeSet::from(["taren".into()]),
        });
        crate::resolution::ensure_agency_profiles(&mut seed);
        let campaign_id = seed.id;
        kernel
            .command(WorldCommand::CreateCampaign {
                campaign: seed,
                evidence_receipts: vec![],
                model_stage_receipts: vec![],
            })
            .await
            .unwrap();

        let error = kernel
            .command(WorldCommand::ReconcileGestaltPresence {
                expected_revision: 0,
                reason,
                plan: GestaltPresencePlan {
                    individuations: vec![GestaltIndividuation {
                        gestalt_id: "refugees".into(),
                        expected_gestalt_version: 0,
                        member: GestaltMemberDelta {
                            schema: "ghostlight.gestalt_member_delta.v1".into(),
                            id: "second-taren".into(),
                            gestalt_id: "refugees".into(),
                            version: 0,
                            name: " tArEn ".into(),
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
                            last_location_id: Some("room".into()),
                            materialized_actor_id: None,
                            last_relevant_revision: 0,
                            relevance_lease_until_revision: 0,
                        },
                        location_id: "room".into(),
                    }],
                    promotions: vec![],
                    demotions: vec![],
                },
            })
            .await
            .unwrap_err();
        assert!(error.to_string().contains("already-addressed actor"));
        let persisted = store
            .load::<Campaign>("campaign.v1", &campaign_id.to_string())
            .unwrap()
            .unwrap()
            .1;
        assert_eq!(persisted.revision, 0);
        assert!(!persisted.gestalt_members.contains_key("second-taren"));
        assert_eq!(persisted.actors["taren"].name, "Taren");
    }

    #[test]
    fn composite_selected_action_commits_atomically_and_rebuilds_derived_lanes() {
        let mut value = campaign();
        value.locations.insert(
            "yard".into(),
            Location {
                id: "yard".into(),
                name: "Yard".into(),
                container_id: None,
                routes: BTreeMap::new(),
                persistent_features: vec![],
            },
        );
        value.locations.get_mut("room").unwrap().routes.insert(
            "yard-route".into(),
            crate::domain::Route {
                destination_id: "yard".into(),
                distance: "nearby".into(),
                travel_minutes: 15,
            },
        );
        let mut runner = value.actors["player"].clone();
        runner.id = "runner".into();
        runner.name = "Runner".into();
        value.actors.insert(runner.id.clone(), runner);
        crate::resolution::ensure_agency_profiles(&mut value);
        let proposal = CellActionProposal {
            subject_id: "runner".into(),
            intent: "cross into the yard and inspect it".into(),
            intended_effect: "arrive in the yard and identify one local hazard".into(),
            priority: 80,
            state_references: vec!["subject:runner".into()],
            public_channels: vec![],
            effects: vec![
                StrategicCellEffect::ActorMove {
                    actor_id: "runner".into(),
                    destination_id: "yard".into(),
                },
                StrategicCellEffect::ActorActivity {
                    actor_id: "runner".into(),
                    activity: StrategicActivityKind::Investigate,
                    target_subject_ids: vec![],
                    location_ids: vec!["yard".into()],
                },
            ],
        };
        let digest = crate::resolution::cell_action_digest(&proposal).unwrap();
        let mut plan = crate::resolution::project_selected_actions(&value, vec![proposal]).unwrap();
        plan.activity_outcomes = vec![StrategicActivityOutcome {
            schema: "ghostlight.strategic_activity_outcome.v1".into(),
            action_digest: digest,
            source_subject_id: "runner".into(),
            band: StrategicOutcomeBand::Success,
            summary: "Runner completes the inspection without a durable discovery.".into(),
            supporting_state_references: vec![],
            effect: StrategicOutcomeEffect::NoMaterialChange {
                reason: "The inspection reveals no durable new fact.".into(),
            },
        }];
        plan.actor_moves = vec![StrategicActorMove {
            actor_id: "player".into(),
            destination_id: "invented".into(),
            public_channels: vec![],
        }];

        let applied = apply_strategic_tick_plan(&mut value, plan).unwrap();

        assert_eq!(value.actors["runner"].location_id, "yard");
        assert_eq!(value.actors["player"].location_id, "room");
        assert_eq!(
            applied
                .events
                .iter()
                .map(|event| event.kind.as_str())
                .collect::<Vec<_>>(),
            vec![
                "actor_movement",
                "actor_activity",
                "strategic_activity_outcome"
            ]
        );
    }

    #[tokio::test]
    async fn mailbox_commits_one_authorized_component_mutation_batch_atomically() {
        use crate::transition::*;

        let dir = tempfile::tempdir().unwrap();
        let store = CampaignStore::open(dir.path().join("campaign.cc")).unwrap();
        let campaign_id = uuid::Uuid::new_v4();
        let campaign_subject = SubjectRef {
            kind: SubjectKind::Campaign,
            id: campaign_id.to_string(),
        };
        let state = ComponentWorldState {
            schema: "ghostlight.component_world_state.v1".into(),
            campaign_id,
            revision: 7,
            resolution_epoch: 3,
            world_time: Utc::now(),
            subjects: BTreeMap::from([(
                campaign_subject.clone(),
                TypedSubject {
                    schema: "ghostlight.typed_subject.v1".into(),
                    subject: campaign_subject.clone(),
                    lifecycle: LifecycleStatus::Active,
                    admitted_components: BTreeSet::from([WorldComponentKind::WorldTime]),
                    version: 7,
                },
            )]),
            occupancy: BTreeMap::new(),
            custody: BTreeMap::new(),
            resources: BTreeMap::new(),
            capabilities: BTreeMap::new(),
            conditions: BTreeMap::new(),
            commitments: BTreeMap::new(),
            relationships: BTreeMap::new(),
            pressures: BTreeMap::new(),
            knowledge: BTreeMap::new(),
            memories: BTreeMap::new(),
            postures: BTreeMap::new(),
            memberships: BTreeMap::new(),
            population_lineages: BTreeMap::new(),
            identities: BTreeMap::new(),
            place_profiles: BTreeMap::new(),
            propositions: BTreeMap::new(),
            topology: BTreeMap::new(),
        };
        let initial_world_time = state.world_time;
        store.create_component_world_state(&state).unwrap();

        let permit = MutationPermit {
            id: "permit:time".into(),
            operation: WorldMutationOperation::AdvanceWorldTime,
            subject_bindings: vec![MutationSubjectBinding {
                role: MutationSubjectRole::Subject,
                allowed_subjects: BTreeSet::from([campaign_subject.clone()]),
            }],
            string_constraints: BTreeMap::new(),
            integer_bounds: BTreeMap::from([(
                MutationIntegerRole::WorldMinutes,
                IntegerBounds {
                    minimum: 30,
                    maximum: 30,
                },
            )]),
            exact_mutation: None,
            maximum_uses: 1,
        };
        let mut authority = MutationAuthorityEnvelope {
            schema: "ghostlight.mutation_authority_envelope.v1".into(),
            id: "authority:time".into(),
            campaign_id,
            world_revision: 7,
            resolution_epoch: Some(3),
            procedure: MutationProcedure::Governance,
            source_subject: None,
            outcome: MutationOutcomeBinding::Deterministic,
            effect_ceiling: "Thirty minutes pass; no other component changes.".into(),
            permits: vec![permit],
            authority_receipt_ids: BTreeSet::from(["governance:time".into()]),
            expires_at: Utc::now() + Duration::minutes(5),
            digest: String::new(),
        };
        authority.digest = envelope_digest(&authority).unwrap();
        let mut batch = WorldMutationBatch {
            schema: "ghostlight.world_mutation_batch.v1".into(),
            id: "batch:time".into(),
            campaign_id,
            expected_world_revision: 7,
            expected_resolution_epoch: Some(3),
            authority_envelope_digest: authority.digest.clone(),
            source_receipt_id: "governance:time".into(),
            means_digest: None,
            intended_effect_digest: None,
            mutations: vec![PermittedWorldMutation {
                permit_id: "permit:time".into(),
                mutation: WorldMutation::AdvanceWorldTime {
                    campaign: campaign_subject,
                    minutes: 30,
                },
            }],
            digest: String::new(),
        };
        batch.digest = mutation_batch_digest(&batch).unwrap();

        let kernel = WorldKernel::start(store.clone());
        let committed = kernel
            .commit_mutation_batch(authority.clone(), batch.clone())
            .await
            .unwrap();
        let CommandResult::MutationCommitted { state, receipt } = committed else {
            panic!("component mutation did not return its typed receipt");
        };
        assert_eq!(state.revision, 8);
        assert_eq!(state.world_time, initial_world_time + Duration::minutes(30));
        assert_eq!(receipt.previous_world_revision, 7);
        assert_eq!(receipt.world_revision, 8);
        assert_eq!(store.keys("world_mutation_receipt.v1").unwrap().len(), 1);

        assert!(
            kernel
                .commit_mutation_batch(authority.clone(), batch.clone())
                .await
                .is_err()
        );
        let stored = store
            .load::<ComponentWorldState>("component_world_state.v1", &campaign_id.to_string())
            .unwrap()
            .unwrap()
            .1;
        assert_eq!(stored.revision, 8);
        assert_eq!(store.keys("world_mutation_receipt.v1").unwrap().len(), 1);

        let mut aggregate = campaign();
        aggregate.id = campaign_id;
        store.create_campaign(&aggregate, &[], &[]).unwrap();
        let error = kernel
            .commit_mutation_batch(authority, batch)
            .await
            .unwrap_err();
        assert!(
            error
                .to_string()
                .contains("only through their WorldCommand mailbox")
        );
        let stored = store
            .load::<ComponentWorldState>("component_world_state.v1", &campaign_id.to_string())
            .unwrap()
            .unwrap()
            .1;
        assert_eq!(stored.revision, 8);
        assert_eq!(store.keys("world_mutation_receipt.v1").unwrap().len(), 1);
    }

    fn hierarchical_refugee_campaign() -> Campaign {
        let mut value = campaign();
        value.locations.insert(
            "camp".into(),
            Location {
                id: "camp".into(),
                name: "Transit camp".into(),
                container_id: None,
                routes: BTreeMap::from([(
                    "to-docks".into(),
                    Route {
                        destination_id: "docks".into(),
                        distance: "across the bay".into(),
                        travel_minutes: 90,
                    },
                )]),
                persistent_features: vec!["departure board".into()],
            },
        );
        value.locations.insert(
            "docks".into(),
            Location {
                id: "docks".into(),
                name: "South docks".into(),
                container_id: None,
                routes: BTreeMap::new(),
                persistent_features: vec!["net lofts".into()],
            },
        );
        let gestalt = |id: &str,
                       name: &str,
                       location: &str,
                       capabilities: &[&str],
                       knowledge: &[&str],
                       goals: &[&str]| GestaltPersonaState {
            schema: "ghostlight.gestalt_persona_state.v1".into(),
            id: id.into(),
            name: name.into(),
            version: 0,
            home_location_id: location.into(),
            shared_capabilities: capabilities.iter().map(|value| (*value).into()).collect(),
            shared_knowledge: knowledge.iter().map(|value| (*value).into()).collect(),
            resources: BTreeSet::new(),
            goals: goals.iter().map(|value| (*value).into()).collect(),
            pressures: vec![],
        };
        value.gestalts.insert(
            "refugees-east".into(),
            gestalt(
                "refugees-east",
                "Eastern transit refugees",
                "camp",
                &["survive transit", "speak old dialect"],
                &["camp alarm", "old village"],
                &["find safety"],
            ),
        );
        value.gestalts.insert(
            "dock-neighbors".into(),
            gestalt(
                "dock-neighbors",
                "South dock neighbors",
                "docks",
                &["repair nets"],
                &["harbor routines", "public bulletin"],
                &["keep the docks running"],
            ),
        );
        value.gestalt_members.insert(
            "mira".into(),
            GestaltMemberDelta {
                schema: "ghostlight.gestalt_member_delta.v1".into(),
                id: "mira".into(),
                gestalt_id: "refugees-east".into(),
                version: 3,
                name: "Mira Venn".into(),
                capability_additions: BTreeSet::from(["weave signal cord".into()]),
                capability_removals: BTreeSet::from(["speak old dialect".into()]),
                knowledge_additions: BTreeSet::from(["the player kept a promise".into()]),
                knowledge_removals: BTreeSet::from(["camp alarm".into()]),
                equipment: BTreeSet::from(["patched blue satchel".into()]),
                conditions: BTreeSet::from(["healed burn scar".into()]),
                obligations: BTreeSet::from(["repay the player's help".into()]),
                relationships: BTreeMap::from([(
                    "player".into(),
                    "trusts them for opening the evacuation gate".into(),
                )]),
                goals: vec![],
                memories: vec!["The player carried her brother through the smoke.".into()],
                last_location_id: Some("camp".into()),
                materialized_actor_id: None,
                last_relevant_revision: 7,
                relevance_lease_until_revision: 0,
            },
        );
        crate::resolution::ensure_agency_profiles(&mut value);
        value
            .agency_profiles
            .get_mut("refugees-east")
            .unwrap()
            .parent_subject_id = Some("refugees-by-destination".into());
        value
            .agency_profiles
            .get_mut("dock-neighbors")
            .unwrap()
            .parent_subject_id = Some("southport-populations".into());
        value.agency_relations.insert(
            "refugee-resettlement".into(),
            AgencyRelation {
                schema: "ghostlight.agency_relation.v1".into(),
                id: "refugee-resettlement".into(),
                from_subject_id: "refugees-east".into(),
                to_subject_id: "dock-neighbors".into(),
                kind: AgencyRelationKind::Migration,
                strength: 90,
                active: true,
                evidence_receipt_ids: vec![],
            },
        );
        value
    }

    #[tokio::test]
    async fn approved_fission_commits_one_typed_lineage_batch_and_no_parallel_writer() {
        let dir = tempfile::tempdir().unwrap();
        let store = CampaignStore::open(dir.path().join("campaign.cc")).unwrap();
        let seed = hierarchical_refugee_campaign();
        let parent = seed.gestalts["refugees-east"].clone();
        let child = |id: &str, name: &str| GestaltPersonaState {
            schema: "ghostlight.gestalt_persona_state.v1".into(),
            id: id.into(),
            name: name.into(),
            version: 0,
            home_location_id: parent.home_location_id.clone(),
            shared_capabilities: parent.shared_capabilities.clone(),
            shared_knowledge: parent.shared_knowledge.clone(),
            resources: BTreeSet::new(),
            goals: parent.goals.clone(),
            pressures: parent.pressures.clone(),
        };
        let preview = GestaltFissionPreview {
            schema: "ghostlight.gestalt_fission_preview.v1".into(),
            campaign_id: seed.id,
            expected_world_revision: seed.revision,
            parent_gestalt_id: parent.id.clone(),
            partition_axis: AgencyAxis::Ideology,
            children: vec![
                child("refugees-returning", "Refugees planning to return"),
                child("refugees-other", "Other eastern transit refugees"),
            ],
            child_partition_values: BTreeMap::from([
                ("refugees-returning".into(), "returning".into()),
                ("refugees-other".into(), "other/unknown".into()),
            ]),
            residual_child_id: "refugees-other".into(),
            member_child_assignments: BTreeMap::from([(
                "mira".into(),
                "refugees-returning".into(),
            )]),
            resource_child_assignments: BTreeMap::new(),
            evidence_receipt_ids: vec![],
            gaps: vec![],
            canon_candidates: vec![],
            requires_approval: true,
        };
        let kernel = WorldKernel::start(store.clone());
        kernel
            .command(WorldCommand::CreateCampaign {
                campaign: seed.clone(),
                evidence_receipts: vec![],
                model_stage_receipts: vec![],
            })
            .await
            .unwrap();
        let result = kernel
            .command(WorldCommand::FissionGestalt {
                expected_revision: seed.revision,
                preview,
                evidence_receipts: vec![],
                model_stage_receipts: vec![],
            })
            .await
            .unwrap();
        let CommandResult::Committed { campaign, receipt } = result else {
            panic!("fission did not commit")
        };
        assert_eq!(receipt.command_kind, "fission_gestalt");
        assert_eq!(campaign.revision, seed.revision + 1);
        assert_eq!(campaign.gestalt_members["mira"].id, "mira");
        assert_eq!(
            campaign.gestalt_members["mira"].gestalt_id,
            "refugees-returning"
        );
        assert!(!campaign.agency_profiles["refugees-east"].active_leaf);
        assert!(campaign.agency_profiles["refugees-returning"].active_leaf);
        assert_eq!(
            store.keys("mutation_authority_envelope.v1").unwrap().len(),
            1
        );
        assert_eq!(store.keys("world_mutation_batch.v1").unwrap().len(), 1);
        assert_eq!(store.keys("world_mutation_receipt.v1").unwrap().len(), 1);
        let batch = store
            .load_all::<crate::transition::WorldMutationBatch>("world_mutation_batch.v1")
            .unwrap()
            .pop()
            .unwrap();
        let operations = batch
            .mutations
            .iter()
            .map(|mutation| mutation.mutation.operation())
            .collect::<BTreeSet<_>>();
        assert!(operations.contains(&crate::transition::WorldMutationOperation::AdmitEntity));
        assert!(operations.contains(&crate::transition::WorldMutationOperation::PopulationSplit));
        assert!(
            operations.contains(&crate::transition::WorldMutationOperation::PopulationTransfer)
        );
        assert!(!operations.contains(&crate::transition::WorldMutationOperation::ResourceCreate));
    }

    #[test]
    fn member_migration_rebases_across_unrelated_hierarchy_without_changing_the_person() {
        let mut value = hierarchical_refugee_campaign();
        let capabilities_before =
            crate::resolution::effective_member_capabilities(&value, "mira").unwrap();
        let knowledge_before =
            crate::resolution::effective_member_knowledge(&value, "mira").unwrap();
        let identity_before = value.gestalt_members["mira"].clone();
        let events = apply_strategic_tick_plan(
            &mut value,
            StrategicTickPlan {
                member_migrations: vec![StrategicMemberMigration {
                    member_id: "mira".into(),
                    source_gestalt_id: "refugees-east".into(),
                    destination_gestalt_id: "dock-neighbors".into(),
                    destination_location_id: "docks".into(),
                    public_channels: vec![],
                }],
                ..Default::default()
            },
        )
        .unwrap();
        let member = &value.gestalt_members["mira"];
        assert_eq!(member.gestalt_id, "dock-neighbors");
        assert_eq!(member.last_location_id.as_deref(), Some("docks"));
        assert_eq!(member.version, identity_before.version + 1);
        assert_eq!(member.name, identity_before.name);
        assert_eq!(member.relationships, identity_before.relationships);
        assert_eq!(member.memories, identity_before.memories);
        assert_eq!(member.equipment, identity_before.equipment);
        assert_eq!(member.conditions, identity_before.conditions);
        assert_eq!(member.obligations, identity_before.obligations);
        assert_eq!(member.goals, vec!["find safety"]);
        assert_eq!(
            crate::resolution::effective_member_capabilities(&value, "mira").unwrap(),
            capabilities_before
        );
        assert_eq!(
            crate::resolution::effective_member_knowledge(&value, "mira").unwrap(),
            knowledge_before
        );
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].kind, "gestalt_member_migration");
        assert_eq!(events[0].actor_ids, vec!["member:mira"]);
        assert_eq!(
            events[0].summary,
            "Mira Venn moves from camp to docks and joins dock-neighbors."
        );

        let actor = materialize_actor(
            &value.gestalts["dock-neighbors"],
            member,
            "member:mira",
            "docks",
        );
        assert_eq!(actor.name, "Mira Venn");
        assert_eq!(actor.capabilities, capabilities_before);
        assert_eq!(actor.knowledge, knowledge_before);
        assert_eq!(actor.relationships, identity_before.relationships);
        assert_eq!(actor.memories, identity_before.memories);
    }

    #[test]
    fn diaspora_distributes_people_across_nested_population_branches_without_merging_identity() {
        let mut value = hierarchical_refugee_campaign();
        value.locations.get_mut("camp").unwrap().routes.insert(
            "to-hills".into(),
            Route {
                destination_id: "hills".into(),
                distance: "along the ridge road".into(),
                travel_minutes: 120,
            },
        );
        value.locations.insert(
            "hills".into(),
            Location {
                id: "hills".into(),
                name: "North hills".into(),
                container_id: None,
                routes: BTreeMap::new(),
                persistent_features: vec!["terraced gardens".into()],
            },
        );

        let population = |id: &str, name: &str, location: &str| GestaltPersonaState {
            schema: "ghostlight.gestalt_persona_state.v1".into(),
            id: id.into(),
            name: name.into(),
            version: 0,
            home_location_id: location.into(),
            shared_capabilities: BTreeSet::from([format!("live among {name}")]),
            shared_knowledge: BTreeSet::from([format!("routines of {name}")]),
            resources: BTreeSet::new(),
            goals: vec![format!("sustain {name}")],
            pressures: vec![],
        };
        for (id, name, location) in [
            ("displaced-root", "Displaced people", "camp"),
            ("crisis-refugees", "Crisis refugees", "camp"),
            ("displaced-other", "Other displaced people", "camp"),
            ("crisis-other", "Other crisis refugees", "camp"),
            ("southport-root", "Southport residents", "docks"),
            ("harbor-populations", "Harbor populations", "docks"),
            ("harbor-other", "Other harbor residents", "docks"),
            ("inland-other", "Inland residents", "hills"),
            ("hill-neighbors", "North hill neighbors", "hills"),
            ("inland-unknown", "Other inland residents", "hills"),
        ] {
            value
                .gestalts
                .insert(id.into(), population(id, name, location));
        }
        crate::resolution::ensure_agency_profiles(&mut value);

        for parent_id in [
            "displaced-root",
            "crisis-refugees",
            "southport-root",
            "harbor-populations",
            "inland-other",
        ] {
            let profile = value.agency_profiles.get_mut(parent_id).unwrap();
            profile.active_leaf = false;
            profile.simulation_eligible = false;
        }
        for (child_id, parent_id) in [
            ("crisis-refugees", "displaced-root"),
            ("displaced-other", "displaced-root"),
            ("refugees-east", "crisis-refugees"),
            ("crisis-other", "crisis-refugees"),
            ("harbor-populations", "southport-root"),
            ("inland-other", "southport-root"),
            ("dock-neighbors", "harbor-populations"),
            ("harbor-other", "harbor-populations"),
            ("hill-neighbors", "inland-other"),
            ("inland-unknown", "inland-other"),
        ] {
            value
                .agency_profiles
                .get_mut(child_id)
                .unwrap()
                .parent_subject_id = Some(parent_id.into());
        }
        let lineage =
            |parent: &str, children: [&str; 2], residual: &str, axis: AgencyAxis| GestaltLineage {
                schema: "ghostlight.gestalt_lineage.v1".into(),
                parent_gestalt_id: parent.into(),
                child_gestalt_ids: children.iter().map(|id| (*id).into()).collect(),
                partition_axis: axis,
                partition_values: BTreeMap::from([
                    (children[0].into(), "selected".into()),
                    (children[1].into(), "other/unknown".into()),
                ]),
                residual_child_id: residual.into(),
                source_revision: value.revision,
            };
        value.gestalt_lineages.insert(
            "displaced-root".into(),
            lineage(
                "displaced-root",
                ["crisis-refugees", "displaced-other"],
                "displaced-other",
                AgencyAxis::Ideology,
            ),
        );
        value.gestalt_lineages.insert(
            "crisis-refugees".into(),
            lineage(
                "crisis-refugees",
                ["refugees-east", "crisis-other"],
                "crisis-other",
                AgencyAxis::Geography,
            ),
        );
        value.gestalt_lineages.insert(
            "southport-root".into(),
            lineage(
                "southport-root",
                ["harbor-populations", "inland-other"],
                "inland-other",
                AgencyAxis::Geography,
            ),
        );
        value.gestalt_lineages.insert(
            "harbor-populations".into(),
            lineage(
                "harbor-populations",
                ["dock-neighbors", "harbor-other"],
                "harbor-other",
                AgencyAxis::EconomyRole,
            ),
        );
        value.gestalt_lineages.insert(
            "inland-other".into(),
            lineage(
                "inland-other",
                ["hill-neighbors", "inland-unknown"],
                "inland-unknown",
                AgencyAxis::Authority,
            ),
        );

        let mut tovan = value.gestalt_members["mira"].clone();
        tovan.id = "tovan".into();
        tovan.name = "Tovan Ser".into();
        tovan.version = 9;
        tovan.capability_additions = BTreeSet::from(["tend terrace herbs".into()]);
        tovan.knowledge_additions = BTreeSet::from(["the player found his daughter".into()]);
        tovan.equipment = BTreeSet::from(["copper seed case".into()]);
        tovan.conditions = BTreeSet::from(["healed broken wrist".into()]);
        tovan.obligations = BTreeSet::from(["send medicine back to the camp".into()]);
        tovan.relationships = BTreeMap::from([(
            "player".into(),
            "remembers that they found his daughter".into(),
        )]);
        tovan.memories = vec!["The player found Lio beneath the fallen awning.".into()];
        value.gestalt_members.insert("tovan".into(), tovan);
        value.agency_relations.insert(
            "hill-resettlement".into(),
            AgencyRelation {
                schema: "ghostlight.agency_relation.v1".into(),
                id: "hill-resettlement".into(),
                from_subject_id: "refugees-east".into(),
                to_subject_id: "hill-neighbors".into(),
                kind: AgencyRelationKind::Migration,
                strength: 85,
                active: true,
                evidence_receipt_ids: vec![],
            },
        );

        let before = value.gestalt_members.clone();
        let source_before = value.gestalts["refugees-east"].clone();
        let docks_before = value.gestalts["dock-neighbors"].clone();
        let hills_before = value.gestalts["hill-neighbors"].clone();
        let effective_goals = |campaign: &Campaign, id: &str| {
            let member = &campaign.gestalt_members[id];
            if member.goals.is_empty() {
                campaign.gestalts[&member.gestalt_id]
                    .goals
                    .iter()
                    .cloned()
                    .collect::<BTreeSet<_>>()
            } else {
                member.goals.iter().cloned().collect::<BTreeSet<_>>()
            }
        };
        let effective_before = ["mira", "tovan"]
            .into_iter()
            .map(|id| {
                (
                    id,
                    (
                        crate::resolution::effective_member_capabilities(&value, id).unwrap(),
                        crate::resolution::effective_member_knowledge(&value, id).unwrap(),
                        effective_goals(&value, id),
                    ),
                )
            })
            .collect::<BTreeMap<_, _>>();

        let events = apply_strategic_tick_plan(
            &mut value,
            StrategicTickPlan {
                member_migrations: vec![
                    StrategicMemberMigration {
                        member_id: "mira".into(),
                        source_gestalt_id: "refugees-east".into(),
                        destination_gestalt_id: "dock-neighbors".into(),
                        destination_location_id: "docks".into(),
                        public_channels: vec![],
                    },
                    StrategicMemberMigration {
                        member_id: "tovan".into(),
                        source_gestalt_id: "refugees-east".into(),
                        destination_gestalt_id: "hill-neighbors".into(),
                        destination_location_id: "hills".into(),
                        public_channels: vec![],
                    },
                ],
                ..Default::default()
            },
        )
        .unwrap();

        for (id, destination, location) in [
            ("mira", "dock-neighbors", "docks"),
            ("tovan", "hill-neighbors", "hills"),
        ] {
            let member = &value.gestalt_members[id];
            let old = &before[id];
            assert_eq!(member.id, old.id);
            assert_eq!(member.name, old.name);
            assert_eq!(member.gestalt_id, destination);
            assert_eq!(member.last_location_id.as_deref(), Some(location));
            assert_eq!(member.version, old.version + 1);
            assert_eq!(member.relationships, old.relationships);
            assert_eq!(member.memories, old.memories);
            assert_eq!(member.equipment, old.equipment);
            assert_eq!(member.conditions, old.conditions);
            assert_eq!(member.obligations, old.obligations);
            assert_eq!(
                crate::resolution::effective_member_capabilities(&value, id).unwrap(),
                effective_before[id].0
            );
            assert_eq!(
                crate::resolution::effective_member_knowledge(&value, id).unwrap(),
                effective_before[id].1
            );
            assert_eq!(effective_goals(&value, id), effective_before[id].2);
            let actor = materialize_actor(
                &value.gestalts[destination],
                member,
                &format!("member:{id}"),
                location,
            );
            assert_eq!(actor.name, old.name);
            assert_eq!(actor.relationships, old.relationships);
            assert_eq!(actor.memories, old.memories);
        }
        for (id, before, version_increment) in [
            ("refugees-east", source_before, 1),
            ("dock-neighbors", docks_before, 1),
            ("hill-neighbors", hills_before, 1),
        ] {
            let after = &value.gestalts[id];
            assert_eq!(after.name, before.name);
            assert_eq!(after.home_location_id, before.home_location_id);
            assert_eq!(after.shared_capabilities, before.shared_capabilities);
            assert_eq!(after.shared_knowledge, before.shared_knowledge);
            assert_eq!(after.resources, before.resources);
            assert_eq!(after.goals, before.goals);
            assert_eq!(after.pressures, before.pressures);
            assert_eq!(after.version, before.version + version_increment);
        }
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].actor_ids, vec!["member:mira"]);
        assert_eq!(events[1].actor_ids, vec!["member:tovan"]);
        assert_eq!(
            events[0].gestalt_ids,
            vec!["refugees-east", "dock-neighbors"]
        );
        assert_eq!(
            events[1].gestalt_ids,
            vec!["refugees-east", "hill-neighbors"]
        );

        let depth = |leaf_id: &str| {
            let mut current = leaf_id;
            let mut depth = 0;
            while let Some(lineage) = value.gestalt_lineages.values().find(|lineage| {
                lineage
                    .child_gestalt_ids
                    .iter()
                    .any(|child| child == current)
            }) {
                depth += 1;
                current = &lineage.parent_gestalt_id;
            }
            depth
        };
        assert_eq!(depth("refugees-east"), 2);
        assert_eq!(depth("dock-neighbors"), 2);
        assert_eq!(depth("hill-neighbors"), 2);
    }

    #[test]
    fn gestalt_migration_moves_only_the_population_leaf() {
        let mut value = hierarchical_refugee_campaign();
        let member_before = value.gestalt_members["mira"].clone();
        let destination_before = value.gestalts["dock-neighbors"].clone();
        let events = apply_strategic_tick_plan(
            &mut value,
            StrategicTickPlan {
                gestalt_migrations: vec![StrategicGestaltMigration {
                    gestalt_id: "refugees-east".into(),
                    destination_gestalt_id: "dock-neighbors".into(),
                    destination_location_id: "docks".into(),
                    public_channels: vec!["camp-bulletin".into()],
                }],
                ..Default::default()
            },
        )
        .unwrap();

        assert_eq!(value.gestalts["refugees-east"].home_location_id, "docks");
        assert_eq!(value.gestalts["refugees-east"].version, 1);
        assert_eq!(
            value.agency_profiles["refugees-east"].location_ids,
            BTreeSet::from(["docks".into()])
        );
        assert_eq!(value.gestalt_members["mira"], member_before);
        assert_eq!(value.gestalts["dock-neighbors"], destination_before);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].kind, "gestalt_migration");
        assert_eq!(events[0].location_ids, vec!["camp", "docks"]);
        assert_eq!(
            events[0].gestalt_ids,
            vec!["refugees-east", "dock-neighbors"]
        );
    }

    #[test]
    fn simultaneous_migration_preserves_snapshot_scoped_activity() {
        let mut value = hierarchical_refugee_campaign();
        value.gestalts.insert(
            "camp-neighbors".into(),
            GestaltPersonaState {
                schema: "ghostlight.gestalt_persona_state.v1".into(),
                id: "camp-neighbors".into(),
                name: "Camp neighbors".into(),
                version: 0,
                home_location_id: "camp".into(),
                shared_capabilities: BTreeSet::from(["raise signal fires".into()]),
                shared_knowledge: BTreeSet::new(),
                resources: BTreeSet::new(),
                goals: vec!["keep departures visible".into()],
                pressures: vec![],
            },
        );
        crate::resolution::ensure_agency_profiles(&mut value);

        let events = apply_strategic_tick_plan(
            &mut value,
            resolve_test_activities(StrategicTickPlan {
                gestalt_migrations: vec![StrategicGestaltMigration {
                    gestalt_id: "refugees-east".into(),
                    destination_gestalt_id: "dock-neighbors".into(),
                    destination_location_id: "docks".into(),
                    public_channels: vec![],
                }],
                gestalt_activities: vec![StrategicGestaltActivity {
                    action_digest: test_action_digest("simultaneous communication"),
                    gestalt_id: "camp-neighbors".into(),
                    activity: StrategicActivityKind::Communicate,
                    target_subject_ids: vec!["refugees-east".into()],
                    location_ids: vec!["camp".into()],
                    public_channels: vec![],
                }],
                ..Default::default()
            }),
        )
        .unwrap();

        assert_eq!(value.gestalts["refugees-east"].home_location_id, "docks");
        assert_eq!(events.len(), 3);
        assert_eq!(events[1].kind, "gestalt_activity");
        assert_eq!(events[1].location_ids, vec!["camp"]);
        assert_eq!(
            events[1].summary,
            "Camp neighbors sends a communication to Eastern transit refugees."
        );
    }

    #[test]
    fn invalid_late_strategic_action_cannot_partially_apply_an_earlier_action() {
        let mut value = hierarchical_refugee_campaign();
        let before = value.clone();
        let error = apply_strategic_tick_plan(
            &mut value,
            resolve_test_activities(StrategicTickPlan {
                gestalt_migrations: vec![StrategicGestaltMigration {
                    gestalt_id: "refugees-east".into(),
                    destination_gestalt_id: "dock-neighbors".into(),
                    destination_location_id: "docks".into(),
                    public_channels: vec![],
                }],
                gestalt_activities: vec![StrategicGestaltActivity {
                    action_digest: test_action_digest("conflicting refugee preparation"),
                    gestalt_id: "refugees-east".into(),
                    activity: StrategicActivityKind::Prepare,
                    target_subject_ids: vec![],
                    location_ids: vec!["camp".into()],
                    public_channels: vec![],
                }],
                ..Default::default()
            }),
        )
        .unwrap_err();

        assert!(error.to_string().contains("acts twice"));
        assert_eq!(value, before);
    }

    #[test]
    fn population_cannot_move_without_its_exact_migration_relation() {
        let mut value = hierarchical_refugee_campaign();
        value.agency_relations.clear();
        let before = value.clone();
        let error = apply_strategic_tick_plan(
            &mut value,
            StrategicTickPlan {
                gestalt_migrations: vec![StrategicGestaltMigration {
                    gestalt_id: "refugees-east".into(),
                    destination_gestalt_id: "dock-neighbors".into(),
                    destination_location_id: "docks".into(),
                    public_channels: vec![],
                }],
                ..Default::default()
            },
        )
        .unwrap_err();
        assert!(error.to_string().contains("source-to-destination relation"));
        assert_eq!(value, before);
    }

    #[test]
    fn invalid_population_migration_batch_has_no_partial_move() {
        let mut value = hierarchical_refugee_campaign();
        let before = value.clone();
        let action = StrategicGestaltMigration {
            gestalt_id: "refugees-east".into(),
            destination_gestalt_id: "dock-neighbors".into(),
            destination_location_id: "docks".into(),
            public_channels: vec![],
        };
        let error = apply_strategic_tick_plan(
            &mut value,
            StrategicTickPlan {
                gestalt_migrations: vec![action.clone(), action],
                ..Default::default()
            },
        )
        .unwrap_err();
        assert!(error.to_string().contains("acts twice"));
        assert_eq!(value, before);
    }

    #[test]
    fn member_migration_requires_the_persons_route_and_typed_population_relation() {
        let mut value = hierarchical_refugee_campaign();
        value.agency_relations.clear();
        let before = value.clone();
        let error = apply_strategic_tick_plan(
            &mut value,
            StrategicTickPlan {
                member_migrations: vec![StrategicMemberMigration {
                    member_id: "mira".into(),
                    source_gestalt_id: "refugees-east".into(),
                    destination_gestalt_id: "dock-neighbors".into(),
                    destination_location_id: "docks".into(),
                    public_channels: vec![],
                }],
                ..Default::default()
            },
        )
        .unwrap_err();
        assert!(error.to_string().contains("migration relation"));
        assert_eq!(value.gestalt_members, before.gestalt_members);
    }

    #[test]
    fn gestalt_action_resolves_exact_pressure_and_records_only_the_typed_transition() {
        let mut value = hierarchical_refugee_campaign();
        value.gestalts.get_mut("refugees-east").unwrap().pressures =
            vec!["the storm closes the camp".into()];
        let events = apply_strategic_tick_plan(
            &mut value,
            StrategicTickPlan {
                gestalt_actions: vec![StrategicGestaltAction {
                    gestalt_id: "refugees-east".into(),
                    pressure_additions: vec!["shelter assignments remain unsettled".into()],
                    pressure_resolutions: vec!["the storm closes the camp".into()],
                    public_channels: vec![],
                }],
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(
            value.gestalts["refugees-east"].pressures,
            vec!["shelter assignments remain unsettled"]
        );
        assert_eq!(value.gestalts["refugees-east"].version, 1);
        assert_eq!(
            events[0].summary,
            "Eastern transit refugees resolves pressure: the storm closes the camp; takes on pressure: shelter assignments remain unsettled"
        );
    }

    #[test]
    fn gestalt_activity_requires_an_explicit_resolved_no_material_outcome() {
        let mut value = hierarchical_refugee_campaign();
        let before = value.clone();
        let events = apply_strategic_tick_plan(
            &mut value,
            resolve_test_activities(StrategicTickPlan {
                gestalt_activities: vec![StrategicGestaltActivity {
                    action_digest: test_action_digest("refugee coordination"),
                    gestalt_id: "refugees-east".into(),
                    activity: StrategicActivityKind::Coordinate,
                    target_subject_ids: vec!["dock-neighbors".into()],
                    location_ids: vec!["camp".into()],
                    public_channels: vec![],
                }],
                ..Default::default()
            }),
        )
        .unwrap();
        assert_only_strategic_obligations_advanced(&before, &value);
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].kind, "gestalt_activity");
        assert_eq!(
            events[0].summary,
            "Eastern transit refugees attempts to coordinate with South dock neighbors."
        );
        assert_eq!(
            events[0].gestalt_ids,
            vec!["refugees-east", "dock-neighbors"]
        );
        assert!(events[0].actor_ids.is_empty());
        assert!(events[0].institution_ids.is_empty());
        assert_eq!(events[1].kind, "strategic_activity_outcome");
        assert_eq!(
            events[1].summary,
            "The attempt produces no durable material change."
        );
    }

    #[test]
    fn strategic_outcome_materializes_a_bounded_resource_from_exact_capability() {
        let mut value = hierarchical_refugee_campaign();
        let digest = test_action_digest("refugees prepare storm lashings");
        let events = apply_strategic_tick_plan(
            &mut value,
            StrategicTickPlan {
                gestalt_activities: vec![StrategicGestaltActivity {
                    action_digest: digest.clone(),
                    gestalt_id: "refugees-east".into(),
                    activity: StrategicActivityKind::Prepare,
                    target_subject_ids: vec![],
                    location_ids: vec!["camp".into()],
                    public_channels: vec![],
                }],
                activity_outcomes: vec![StrategicActivityOutcome {
                    schema: "ghostlight.strategic_activity_outcome.v1".into(),
                    action_digest: digest,
                    source_subject_id: "refugees-east".into(),
                    band: StrategicOutcomeBand::Success,
                    summary: "The refugees finish a set of storm lashings.".into(),
                    supporting_state_references: vec![
                        "capability:survive transit".into(),
                        "location:camp".into(),
                    ],
                    effect: StrategicOutcomeEffect::ResourceCreated {
                        owner_subject_id: "refugees-east".into(),
                        resource: "storm lashings".into(),
                    },
                }],
                ..Default::default()
            },
        )
        .unwrap();

        assert!(
            value.gestalts["refugees-east"]
                .resources
                .contains("storm lashings")
        );
        assert_eq!(value.gestalts["refugees-east"].version, 1);
        assert_eq!(events.len(), 2);
        assert_eq!(events[1].kind, "strategic_activity_outcome");
    }

    #[test]
    fn invalid_outcome_bundle_cannot_apply_an_earlier_material_effect() {
        let mut value = hierarchical_refugee_campaign();
        let before = value.clone();
        let first = test_action_digest("valid material preparation");
        let second = test_action_digest("invalid resource spend");
        let error = apply_strategic_tick_plan(
            &mut value,
            StrategicTickPlan {
                gestalt_activities: vec![
                    StrategicGestaltActivity {
                        action_digest: first.clone(),
                        gestalt_id: "refugees-east".into(),
                        activity: StrategicActivityKind::Prepare,
                        target_subject_ids: vec![],
                        location_ids: vec!["camp".into()],
                        public_channels: vec![],
                    },
                    StrategicGestaltActivity {
                        action_digest: second.clone(),
                        gestalt_id: "dock-neighbors".into(),
                        activity: StrategicActivityKind::Prepare,
                        target_subject_ids: vec![],
                        location_ids: vec!["docks".into()],
                        public_channels: vec![],
                    },
                ],
                activity_outcomes: vec![
                    StrategicActivityOutcome {
                        schema: "ghostlight.strategic_activity_outcome.v1".into(),
                        action_digest: first,
                        source_subject_id: "refugees-east".into(),
                        band: StrategicOutcomeBand::Success,
                        summary: "A valid first result.".into(),
                        supporting_state_references: vec!["capability:survive transit".into()],
                        effect: StrategicOutcomeEffect::ResourceCreated {
                            owner_subject_id: "refugees-east".into(),
                            resource: "valid new shelter frame".into(),
                        },
                    },
                    StrategicActivityOutcome {
                        schema: "ghostlight.strategic_activity_outcome.v1".into(),
                        action_digest: second,
                        source_subject_id: "dock-neighbors".into(),
                        band: StrategicOutcomeBand::Failure,
                        summary: "An invalid second result.".into(),
                        supporting_state_references: vec!["capability:repair nets".into()],
                        effect: StrategicOutcomeEffect::ResourceConsumed {
                            owner_subject_id: "dock-neighbors".into(),
                            resource: "invented missing winch".into(),
                        },
                    },
                ],
                ..Default::default()
            },
        )
        .unwrap_err();

        assert!(error.to_string().contains("custody"));
        assert_eq!(value, before);
    }

    #[test]
    fn member_outcome_preserves_its_own_relationship_without_mutating_the_player() {
        let mut value = hierarchical_refugee_campaign();
        value.agency_relations.insert(
            "refugee-message-to-player".into(),
            AgencyRelation {
                schema: "ghostlight.agency_relation.v1".into(),
                id: "refugee-message-to-player".into(),
                from_subject_id: "refugees-east".into(),
                to_subject_id: "player".into(),
                kind: AgencyRelationKind::Communication,
                strength: 40,
                active: true,
                evidence_receipt_ids: vec![],
            },
        );
        let player_before = value.actors["player"].clone();
        let digest = test_action_digest("mira renews her promise");
        apply_strategic_tick_plan(
            &mut value,
            StrategicTickPlan {
                member_activities: vec![StrategicMemberActivity {
                    action_digest: digest.clone(),
                    member_id: "mira".into(),
                    source_gestalt_id: "refugees-east".into(),
                    activity: StrategicActivityKind::Communicate,
                    target_subject_ids: vec!["player".into()],
                    location_ids: vec!["camp".into()],
                    public_channels: vec![],
                }],
                activity_outcomes: vec![StrategicActivityOutcome {
                    schema: "ghostlight.strategic_activity_outcome.v1".into(),
                    action_digest: digest,
                    source_subject_id: "member:mira".into(),
                    band: StrategicOutcomeBand::Success,
                    summary: "Mira keeps the promise alive despite the distance.".into(),
                    supporting_state_references: vec!["member:mira".into()],
                    effect: StrategicOutcomeEffect::MemberRelationship {
                        member_id: "mira".into(),
                        other_subject_id: "player".into(),
                        description: "intends to find and repay the rescuer after settling".into(),
                    },
                }],
                ..Default::default()
            },
        )
        .unwrap();

        assert_eq!(value.actors["player"], player_before);
        assert_eq!(
            value.gestalt_members["mira"].relationships["player"],
            "intends to find and repay the rescuer after settling"
        );
    }

    #[test]
    fn local_investigation_needs_a_location_but_not_an_invented_actor() {
        let mut value = hierarchical_refugee_campaign();
        let before = value.clone();
        let events = apply_strategic_tick_plan(
            &mut value,
            resolve_test_activities(StrategicTickPlan {
                gestalt_activities: vec![StrategicGestaltActivity {
                    action_digest: test_action_digest("local refugee investigation"),
                    gestalt_id: "refugees-east".into(),
                    activity: StrategicActivityKind::Investigate,
                    target_subject_ids: vec![],
                    location_ids: vec!["camp".into()],
                    public_channels: vec![],
                }],
                ..Default::default()
            }),
        )
        .unwrap();
        assert_only_strategic_obligations_advanced(&before, &value);
        assert_eq!(
            events[0].summary,
            "Eastern transit refugees begins a local investigation."
        );
        assert!(events[0].actor_ids.is_empty());
        assert!(events[0].institution_ids.is_empty());
    }

    #[test]
    fn local_communication_records_the_source_without_inventing_a_listener() {
        let mut value = hierarchical_refugee_campaign();
        let before = value.clone();
        let events = apply_strategic_tick_plan(
            &mut value,
            resolve_test_activities(StrategicTickPlan {
                gestalt_activities: vec![StrategicGestaltActivity {
                    action_digest: test_action_digest("local refugee communication"),
                    gestalt_id: "refugees-east".into(),
                    activity: StrategicActivityKind::Communicate,
                    target_subject_ids: vec![],
                    location_ids: vec!["camp".into()],
                    public_channels: vec![],
                }],
                ..Default::default()
            }),
        )
        .unwrap();
        assert_only_strategic_obligations_advanced(&before, &value);
        assert_eq!(
            events[0].summary,
            "Eastern transit refugees attempts a local communication."
        );
        assert!(events[0].actor_ids.is_empty());
        assert!(events[0].institution_ids.is_empty());
        assert_eq!(events[0].gestalt_ids, vec!["refugees-east"]);
    }

    #[test]
    fn local_obstruction_records_the_source_without_inventing_a_target() {
        let mut value = hierarchical_refugee_campaign();
        let before = value.clone();
        let events = apply_strategic_tick_plan(
            &mut value,
            resolve_test_activities(StrategicTickPlan {
                gestalt_activities: vec![StrategicGestaltActivity {
                    action_digest: test_action_digest("local infrastructure obstruction"),
                    gestalt_id: "refugees-east".into(),
                    activity: StrategicActivityKind::Obstruct,
                    target_subject_ids: vec![],
                    location_ids: vec!["camp".into()],
                    public_channels: vec![],
                }],
                ..Default::default()
            }),
        )
        .unwrap();
        assert_only_strategic_obligations_advanced(&before, &value);
        assert_eq!(
            events[0].summary,
            "Eastern transit refugees attempts local interference."
        );
        assert!(events[0].actor_ids.is_empty());
        assert!(events[0].institution_ids.is_empty());
        assert_eq!(events[0].gestalt_ids, vec!["refugees-east"]);
    }

    #[test]
    fn dormant_member_can_speak_locally_without_inventing_a_listener() {
        let mut value = hierarchical_refugee_campaign();
        let before = value.clone();
        let events = apply_strategic_tick_plan(
            &mut value,
            resolve_test_activities(StrategicTickPlan {
                member_activities: vec![StrategicMemberActivity {
                    action_digest: test_action_digest("mira local communication"),
                    member_id: "mira".into(),
                    source_gestalt_id: "refugees-east".into(),
                    activity: StrategicActivityKind::Communicate,
                    target_subject_ids: vec![],
                    location_ids: vec!["camp".into()],
                    public_channels: vec![],
                }],
                ..Default::default()
            }),
        )
        .unwrap();
        assert_only_strategic_obligations_advanced(&before, &value);
        assert_eq!(
            events[0].summary,
            "Mira Venn attempts a local communication."
        );
        assert_eq!(events[0].actor_ids, vec!["member:mira"]);
        assert!(events[0].institution_ids.is_empty());
        assert_eq!(events[0].gestalt_ids, vec!["refugees-east"]);
    }

    #[test]
    fn colocated_gestalt_can_address_a_dormant_member_without_absorbing_their_agency() {
        let mut value = hierarchical_refugee_campaign();
        value
            .gestalt_members
            .get_mut("mira")
            .unwrap()
            .last_location_id = Some("docks".into());
        value.gestalt_members.get_mut("mira").unwrap().gestalt_id = "dock-neighbors".into();
        let before = value.clone();
        let events = apply_strategic_tick_plan(
            &mut value,
            resolve_test_activities(StrategicTickPlan {
                gestalt_activities: vec![StrategicGestaltActivity {
                    action_digest: test_action_digest("neighbors address mira"),
                    gestalt_id: "dock-neighbors".into(),
                    activity: StrategicActivityKind::Communicate,
                    target_subject_ids: vec!["member:mira".into()],
                    location_ids: vec!["docks".into()],
                    public_channels: vec![],
                }],
                ..Default::default()
            }),
        )
        .unwrap();
        assert_only_strategic_obligations_advanced(&before, &value);
        assert_eq!(events.len(), 2);
        assert_eq!(
            events[0].summary,
            "South dock neighbors sends a communication to Mira Venn."
        );
        assert_eq!(events[0].actor_ids, vec!["member:mira"]);
        assert_eq!(events[0].gestalt_ids, vec!["dock-neighbors"]);
    }

    #[test]
    fn gestalt_cannot_address_a_dormant_member_at_another_location() {
        let mut value = hierarchical_refugee_campaign();
        let before = value.clone();
        let error = apply_strategic_tick_plan(
            &mut value,
            resolve_test_activities(StrategicTickPlan {
                gestalt_activities: vec![StrategicGestaltActivity {
                    action_digest: test_action_digest("remote address mira"),
                    gestalt_id: "dock-neighbors".into(),
                    activity: StrategicActivityKind::Communicate,
                    target_subject_ids: vec!["member:mira".into()],
                    location_ids: vec!["docks".into()],
                    public_channels: vec![],
                }],
                ..Default::default()
            }),
        )
        .unwrap_err();
        assert!(error.to_string().contains("exact graph or location scope"));
        assert_eq!(value, before);
    }

    #[test]
    fn gestalt_activity_rejects_a_nonadjacent_target_without_mutation() {
        let mut value = hierarchical_refugee_campaign();
        value.gestalts.insert(
            "distant-crowd".into(),
            GestaltPersonaState {
                schema: "ghostlight.gestalt_persona_state.v1".into(),
                id: "distant-crowd".into(),
                name: "Distant crowd".into(),
                version: 0,
                home_location_id: "docks".into(),
                shared_capabilities: BTreeSet::new(),
                shared_knowledge: BTreeSet::new(),
                resources: BTreeSet::new(),
                goals: vec![],
                pressures: vec![],
            },
        );
        crate::resolution::ensure_agency_profiles(&mut value);
        let before = value.clone();
        let error = apply_strategic_tick_plan(
            &mut value,
            resolve_test_activities(StrategicTickPlan {
                gestalt_activities: vec![StrategicGestaltActivity {
                    action_digest: test_action_digest("nonadjacent communication"),
                    gestalt_id: "refugees-east".into(),
                    activity: StrategicActivityKind::Communicate,
                    target_subject_ids: vec!["distant-crowd".into()],
                    location_ids: vec!["camp".into()],
                    public_channels: vec![],
                }],
                ..Default::default()
            }),
        )
        .unwrap_err();
        assert!(error.to_string().contains("exact graph or location scope"));
        assert_eq!(value, before);
    }

    #[test]
    fn dormant_member_activity_preserves_identity_and_population_state() {
        let mut value = hierarchical_refugee_campaign();
        let before = value.clone();
        let events = apply_strategic_tick_plan(
            &mut value,
            resolve_test_activities(StrategicTickPlan {
                member_activities: vec![StrategicMemberActivity {
                    action_digest: test_action_digest("mira addresses refugees"),
                    member_id: "mira".into(),
                    source_gestalt_id: "refugees-east".into(),
                    activity: StrategicActivityKind::Communicate,
                    target_subject_ids: vec!["refugees-east".into()],
                    location_ids: vec!["camp".into()],
                    public_channels: vec![],
                }],
                ..Default::default()
            }),
        )
        .unwrap();
        assert_only_strategic_obligations_advanced(&before, &value);
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].kind, "gestalt_member_activity");
        assert_eq!(
            events[0].summary,
            "Mira Venn sends a communication to Eastern transit refugees."
        );
        assert_eq!(events[0].actor_ids, vec!["member:mira"]);
        assert_eq!(events[0].gestalt_ids, vec!["refugees-east"]);
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
                model_stage_receipts: vec![],
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
        assert_eq!(
            store.keys("mutation_authority_envelope.v1").unwrap().len(),
            1
        );
        assert_eq!(store.keys("world_mutation_batch.v1").unwrap().len(), 1);
        assert_eq!(store.keys("world_mutation_receipt.v1").unwrap().len(), 1);
    }

    #[tokio::test]
    async fn invalid_wait_and_oversized_speech_cannot_mutate_campaign() {
        let dir = tempfile::tempdir().unwrap();
        let store = CampaignStore::open(dir.path().join("campaign.cc")).unwrap();
        let kernel = WorldKernel::start(store.clone());
        let seed = campaign();
        kernel
            .command(WorldCommand::CreateCampaign {
                campaign: seed.clone(),
                evidence_receipts: vec![],
                model_stage_receipts: vec![],
            })
            .await
            .unwrap();
        let baseline = store
            .load::<Campaign>("campaign.v1", &seed.id.to_string())
            .unwrap()
            .unwrap()
            .1;

        assert!(
            kernel
                .command(WorldCommand::Wait {
                    expected_revision: 0,
                    minutes: 1_441,
                })
                .await
                .is_err()
        );
        assert!(
            kernel
                .command(WorldCommand::Speak {
                    expected_revision: 0,
                    actor_id: "player".into(),
                    text: "x".repeat(4_001),
                    intended_effect: None,
                    persona_response_actor_ids: BTreeSet::new(),
                })
                .await
                .is_err()
        );
        let stored = store
            .load::<Campaign>("campaign.v1", &seed.id.to_string())
            .unwrap()
            .unwrap()
            .1;
        assert_eq!(stored, baseline);
        assert!(store.keys("world_commit_receipt.v1").unwrap().is_empty());
    }

    #[tokio::test]
    async fn npc_speech_does_not_impersonate_player_activity() {
        let dir = tempfile::tempdir().unwrap();
        let store = CampaignStore::open(dir.path().join("campaign.cc")).unwrap();
        let kernel = WorldKernel::start(store);
        let mut seed = campaign();
        let inactive_since = Utc::now() - Duration::hours(2);
        seed.last_player_activity = inactive_since;
        let mut npc = seed.actors["player"].clone();
        npc.id = "npc".into();
        npc.name = "NPC".into();
        seed.actors.insert("npc".into(), npc);
        kernel
            .command(WorldCommand::CreateCampaign {
                campaign: seed,
                evidence_receipts: vec![],
                model_stage_receipts: vec![],
            })
            .await
            .unwrap();
        let result = kernel
            .command(WorldCommand::Speak {
                expected_revision: 0,
                actor_id: "npc".into(),
                text: "The world does not wait.".into(),
                intended_effect: None,
                persona_response_actor_ids: BTreeSet::new(),
            })
            .await
            .unwrap();
        let CommandResult::Committed { campaign, .. } = result else {
            panic!()
        };
        assert_eq!(campaign.last_player_activity, inactive_since);
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
                model_stage_receipts: vec![],
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
                actor_id: "player".into(),
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
        assert_eq!(store.keys("roll_receipt.v1").unwrap().len(), 1);
        assert_eq!(
            store.keys("mutation_authority_envelope.v1").unwrap().len(),
            1
        );
        assert_eq!(store.keys("world_mutation_batch.v1").unwrap().len(), 1);
        assert_eq!(store.keys("world_mutation_receipt.v1").unwrap().len(), 1);
    }

    #[tokio::test]
    async fn repair_assessment_persists_reduced_world_pressure() {
        let dir = tempfile::tempdir().unwrap();
        let store = CampaignStore::open(dir.path().join("campaign.cc")).unwrap();
        let kernel = WorldKernel::start(store.clone());
        let mut seed = campaign();
        seed.clocks.insert(
            "clinic-failure".into(),
            WorldClock {
                id: "clinic-failure".into(),
                label: "Clinic failure".into(),
                progress: 3,
                threshold: 4,
                consequence: "The regulator fails.".into(),
            },
        );
        kernel
            .command(WorldCommand::CreateCampaign {
                campaign: seed.clone(),
                evidence_receipts: vec![],
                model_stage_receipts: vec![],
            })
            .await
            .unwrap();
        let intent = ActionIntent {
            actor_id: "player".into(),
            description: "Patch the failing regulator seal".into(),
            intended_effect: "Relieve immediate clinic failure pressure".into(),
        };
        let reduction = WorldEffectDelta {
            clock_reductions: BTreeMap::from([("clinic-failure".into(), 2)]),
            ..Default::default()
        };
        let mut assessment = assess(&seed, intent.clone());
        assessment.strong_effect = reduction.clone();
        assessment.success_effect = reduction.clone();
        assessment.mixed_effect = reduction.clone();
        // Keep the effect identical across bands so this test exercises the
        // atomic reduction primitive independently of the OS-random roll.
        assessment.failure_effect = reduction;
        assessment.digest = crate::assessor::assessment_digest(&assessment).unwrap();
        kernel
            .command(WorldCommand::Assess {
                expected_revision: 0,
                intent,
                proposal: Some(assessment.clone()),
            })
            .await
            .unwrap();
        let result = kernel
            .command(WorldCommand::Attempt {
                actor_id: "player".into(),
                assessment_digest: assessment.digest,
            })
            .await
            .unwrap();
        let CommandResult::Committed { campaign, .. } = result else {
            panic!("expected repair commit")
        };

        assert_eq!(campaign.clocks["clinic-failure"].progress, 1);
        assert_eq!(campaign.revision, 1);
    }

    #[tokio::test]
    async fn stale_assessment_returns_original_intent_for_fresh_compilation() {
        let dir = tempfile::tempdir().unwrap();
        let store = CampaignStore::open(dir.path().join("campaign.cc")).unwrap();
        let kernel = WorldKernel::start(store.clone());
        let seed = campaign();
        kernel
            .command(WorldCommand::CreateCampaign {
                campaign: seed.clone(),
                evidence_receipts: vec![],
                model_stage_receipts: vec![],
            })
            .await
            .unwrap();
        let intent = ActionIntent {
            actor_id: "player".into(),
            description: "Open the ordinary door".into(),
            intended_effect: "Pass through".into(),
        };
        let CommandResult::Assessed { assessment } = kernel
            .command(WorldCommand::Assess {
                expected_revision: 0,
                intent: intent.clone(),
                proposal: None,
            })
            .await
            .unwrap()
        else {
            panic!()
        };
        kernel
            .command(WorldCommand::Speak {
                expected_revision: 0,
                actor_id: "player".into(),
                text: "Wait.".into(),
                intended_effect: None,
                persona_response_actor_ids: BTreeSet::new(),
            })
            .await
            .unwrap();
        let error = kernel
            .command(WorldCommand::Attempt {
                actor_id: "player".into(),
                assessment_digest: assessment.digest,
            })
            .await
            .unwrap_err();
        match error {
            KernelError::StaleAssessment {
                intent: stale_intent,
                actual_revision,
            } => {
                assert_eq!(stale_intent, intent);
                assert_eq!(actual_revision, 1);
            }
            other => panic!("unexpected error: {other}"),
        }
        assert!(store.keys("roll_receipt.v1").unwrap().is_empty());
    }

    #[tokio::test]
    async fn strategic_tick_without_an_admitted_wave_advances_only_deterministic_obligations() {
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
                model_stage_receipts: vec![],
            })
            .await
            .unwrap();
        let result = kernel
            .command(WorldCommand::AdvanceStrategicTick {
                expected_revision: 0,
                source: TickSource::Scheduler,
                plan: None,
                model_receipt_hash: None,
                resolution_wave: None,
            })
            .await
            .unwrap();
        let CommandResult::Committed { campaign, .. } = result else {
            panic!("expected commit")
        };
        assert!(campaign.events.is_empty());
        assert!(campaign.news.is_empty());
        assert_eq!(campaign.away_ticks_processed, 1);
        assert_eq!(campaign.institutions["board"].posture, "watching");
        assert_eq!(campaign.world_time, seed.world_time + Duration::hours(6));
        let ticks = store
            .load_all::<crate::domain::StrategicTickReceipt>("strategic_tick.v1")
            .unwrap();
        assert_eq!(ticks.len(), 1);
        assert_eq!(ticks[0].source, TickSource::Scheduler);
        assert!(ticks[0].event_ids.is_empty());
        assert!(ticks[0].model_receipt_hash.is_none());
        assert_eq!(store.keys("world_mutation_receipt.v1").unwrap().len(), 1);
    }

    #[tokio::test]
    async fn strategic_plan_moves_remote_actors_but_cannot_puppet_the_player() {
        let dir = tempfile::tempdir().unwrap();
        let store = CampaignStore::open(dir.path().join("campaign.cc")).unwrap();
        let kernel = WorldKernel::start(store.clone());
        let mut seed = campaign();
        seed.locations.get_mut("room").unwrap().routes.insert(
            "road".into(),
            Route {
                destination_id: "yard".into(),
                distance: "near".into(),
                travel_minutes: 20,
            },
        );
        seed.locations.insert(
            "yard".into(),
            Location {
                id: "yard".into(),
                name: "Yard".into(),
                container_id: None,
                routes: BTreeMap::new(),
                persistent_features: vec![],
            },
        );
        let mut npc = seed.actors["player"].clone();
        npc.id = "runner".into();
        npc.name = "Runner".into();
        seed.actors.insert(npc.id.clone(), npc);
        kernel
            .command(WorldCommand::CreateCampaign {
                campaign: seed.clone(),
                evidence_receipts: vec![],
                model_stage_receipts: vec![],
            })
            .await
            .unwrap();

        let puppet = crate::domain::StrategicTickPlan {
            actor_moves: vec![crate::domain::StrategicActorMove {
                actor_id: "player".into(),
                destination_id: "yard".into(),
                public_channels: vec![],
            }],
            ..Default::default()
        };
        assert!(matches!(
            kernel
                .command(WorldCommand::AdvanceStrategicTick {
                    expected_revision: 0,
                    source: TickSource::Scheduler,
                    plan: Some(puppet),
                    model_receipt_hash: Some(format!("sha256:{}", "a".repeat(64))),
                    resolution_wave: None,
                })
                .await,
            Err(KernelError::Invalid(_))
        ));
        let (_, unchanged): (_, Campaign) = store
            .load("campaign.v1", &seed.id.to_string())
            .unwrap()
            .unwrap();
        assert_eq!(unchanged.revision, 0);
        assert_eq!(unchanged.actors["player"].location_id, "room");

        let valid = crate::domain::StrategicTickPlan {
            actor_moves: vec![crate::domain::StrategicActorMove {
                actor_id: "runner".into(),
                destination_id: "yard".into(),
                public_channels: vec!["courier-network".into()],
            }],
            ..Default::default()
        };
        let result = kernel
            .command(WorldCommand::AdvanceStrategicTick {
                expected_revision: 0,
                source: TickSource::Scheduler,
                plan: Some(valid),
                model_receipt_hash: Some(format!("sha256:{}", "b".repeat(64))),
                resolution_wave: None,
            })
            .await
            .unwrap();
        let CommandResult::Committed { campaign, .. } = result else {
            panic!("expected commit")
        };
        assert_eq!(campaign.actors["runner"].location_id, "yard");
        assert_eq!(campaign.actors["player"].location_id, "room");
        assert_eq!(campaign.events[0].kind, "actor_movement");
        let ticks = store
            .load_all::<crate::domain::StrategicTickReceipt>("strategic_tick.v1")
            .unwrap();
        assert_eq!(
            ticks[0].model_receipt_hash,
            Some(format!("sha256:{}", "b".repeat(64)))
        );

        let actor_activity = resolve_test_activities(crate::domain::StrategicTickPlan {
            actor_activities: vec![crate::domain::StrategicActorActivity {
                action_digest: test_action_digest("runner-investigates-yard"),
                actor_id: "runner".into(),
                activity: StrategicActivityKind::Investigate,
                target_subject_ids: vec![],
                location_ids: vec!["yard".into()],
                public_channels: vec![],
            }],
            ..Default::default()
        });
        let result = kernel
            .command(WorldCommand::AdvanceStrategicTick {
                expected_revision: 1,
                source: TickSource::Scheduler,
                plan: Some(actor_activity),
                model_receipt_hash: Some(format!("sha256:{}", "c".repeat(64))),
                resolution_wave: None,
            })
            .await
            .unwrap();
        let CommandResult::Committed { campaign, .. } = result else {
            panic!("expected commit")
        };
        assert_eq!(campaign.actors["runner"].location_id, "yard");
        assert_eq!(campaign.actors["player"].location_id, "room");
        assert!(
            campaign
                .events
                .iter()
                .any(|event| event.kind == "actor_activity"
                    && event.actor_ids.contains(&"runner".to_owned()))
        );
        assert!(
            campaign
                .events
                .iter()
                .any(|event| event.kind == "strategic_activity_outcome")
        );
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
                model_stage_receipts: vec![],
            })
            .await
            .unwrap();
        let evidence = VaultEvidenceReceipt {
            schema: "ghostlight.vault_evidence_receipt.v1".into(),
            id: "vault:route".into(),
            provider: "fixture".into(),
            query_hash: "sha256:q".into(),
            witnesses: vec![SourceWitness {
                source_id: "lore/roads.md".into(),
                exact_locator: "line:10".into(),
                content_hash: "sha256:witness".into(),
                excerpt: "The annex road remains open.".into(),
                authority_lane: "canon".into(),
                temporal_scope: "fixture-era".into(),
            }],
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
                    origin_routes: BTreeMap::from([(
                        "to-annex".into(),
                        Route {
                            destination_id: "annex".into(),
                            distance: "near".into(),
                            travel_minutes: 10,
                        },
                    )]),
                    locations: vec![location],
                    facts: vec![WorldFact {
                        id: "annex-gate".into(),
                        statement: "The annex gate is maintained from the inner booth.".into(),
                        scope: FactScope::BranchLocal,
                        evidence_receipt_ids: vec![evidence.id.clone()],
                        discoverable_at_location_ids: BTreeSet::from(["annex".into()]),
                    }],
                },
                evidence_receipts: vec![evidence],
                canon_candidates: vec![candidate],
                model_stage_receipts: vec![],
            })
            .await
            .unwrap();
        let CommandResult::Committed { campaign, .. } = result else {
            panic!("expected commit")
        };
        assert!(campaign.locations.contains_key("annex"));
        assert_eq!(
            campaign.locations["room"].routes["to-annex"].destination_id,
            "annex"
        );
        assert_eq!(campaign.revision, 1);
        assert_eq!(
            campaign.facts["annex-gate"].discoverable_at_location_ids,
            BTreeSet::from(["annex".into()])
        );
        assert_eq!(store.keys("vault_evidence_receipt.v1").unwrap().len(), 1);
        assert_eq!(store.keys("canon_candidate.v1").unwrap().len(), 1);
        assert_eq!(store.keys("world_mutation_batch.v1").unwrap().len(), 1);
        assert_eq!(store.keys("world_mutation_receipt.v1").unwrap().len(), 1);
        let batches = store
            .load_all::<crate::transition::WorldMutationBatch>("world_mutation_batch.v1")
            .unwrap();
        assert_eq!(batches[0].mutations.len(), 4);
        assert_eq!(
            batches[0]
                .mutations
                .iter()
                .filter(|mutation| matches!(
                    &mutation.mutation,
                    crate::transition::WorldMutation::AdmitEntity { .. }
                ))
                .count(),
            2
        );
        let (_, manifest): (_, VaultManifest) = store
            .load("vault_manifest.v1", &seed.id.to_string())
            .unwrap()
            .unwrap();
        assert!(manifest.source_ids.contains("lore/roads.md"));
        assert!(manifest.authority_lanes.contains("canon"));
        assert!(manifest.temporal_scopes.contains("fixture-era"));
    }

    #[tokio::test]
    async fn gestalt_member_dematerializes_to_delta_and_returns_as_same_person() {
        let dir = tempfile::tempdir().unwrap();
        let store = CampaignStore::open(dir.path().join("campaign.cc")).unwrap();
        let kernel = WorldKernel::start(store.clone());
        let mut seed = campaign();
        seed.locations.insert(
            "away".into(),
            Location {
                id: "away".into(),
                name: "Away".into(),
                container_id: None,
                routes: BTreeMap::new(),
                persistent_features: vec![],
            },
        );
        seed.actors.get_mut("player").unwrap().location_id = "away".into();
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
        let gestalt_baseline = seed.gestalts["village"].clone();
        crate::resolution::ensure_agency_profiles(&mut seed);
        let demand = crate::resolution::default_demand(&seed, "pre-materialization cover");
        seed.resolution_cover = Some(crate::resolution::plan_cover(&seed, demand).unwrap());
        let john = GestaltMemberDelta {
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
            last_relevant_revision: 0,
            relevance_lease_until_revision: 0,
        };
        kernel
            .command(WorldCommand::CreateCampaign {
                campaign: seed,
                evidence_receipts: vec![],
                model_stage_receipts: vec![],
            })
            .await
            .unwrap();
        let first = kernel
            .command(WorldCommand::IndividuateGestaltMember {
                expected_revision: 0,
                individuation: GestaltIndividuation {
                    gestalt_id: "village".into(),
                    expected_gestalt_version: 0,
                    member: john,
                    location_id: "room".into(),
                },
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
        assert_eq!(first.resolution_policy.resolution_epoch, 1);
        assert!(first.resolution_cover.is_none());
        assert!(
            kernel
                .command(WorldCommand::DematerializeGestaltMember {
                    expected_revision: 1,
                    actor_id: "member:john".into(),
                })
                .await
                .unwrap_err()
                .to_string()
                .contains("lease")
        );
        kernel
            .command(WorldCommand::Wait {
                expected_revision: 1,
                minutes: 1,
            })
            .await
            .unwrap();
        let folded = kernel
            .command(WorldCommand::DematerializeGestaltMember {
                expected_revision: 2,
                actor_id: "member:john".into(),
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
        assert_eq!(folded.resolution_policy.resolution_epoch, 2);
        assert!(folded.resolution_cover.is_none());
        assert_eq!(
            folded.gestalt_members["john"].memories,
            vec!["met the player"]
        );
        let again = kernel
            .command(WorldCommand::MaterializeGestaltMember {
                expected_revision: 3,
                gestalt_id: "village".into(),
                expected_gestalt_version: 0,
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
        assert_eq!(again.gestalts["village"], gestalt_baseline);
        assert_eq!(again.resolution_policy.resolution_epoch, 3);
        assert!(again.resolution_cover.is_none());

        let bad_plan = GestaltPresencePlan {
            individuations: vec![],
            demotions: vec![GestaltDemotion {
                actor_id: "member:john".into(),
            }],
            promotions: vec![GestaltPromotion {
                gestalt_id: "village".into(),
                expected_gestalt_version: 0,
                member_id: "invented-person".into(),
                expected_member_version: 0,
                location_id: "room".into(),
            }],
        };
        assert!(
            kernel
                .command(WorldCommand::ReconcileGestaltPresence {
                    expected_revision: 4,
                    reason: "scene relevance changed".into(),
                    plan: bad_plan,
                })
                .await
                .is_err()
        );
        let persisted = store
            .load::<Campaign>("campaign.v1", &again.id.to_string())
            .unwrap()
            .unwrap()
            .1;
        assert_eq!(persisted.revision, 4);
        assert!(persisted.actors.contains_key("member:john"));
        assert_eq!(
            persisted.gestalt_members["john"]
                .materialized_actor_id
                .as_deref(),
            Some("member:john")
        );
        let receipts = store
            .load_all::<crate::domain::GestaltMaterializationReceipt>(
                "gestalt_materialization_receipt.v1",
            )
            .unwrap();
        assert_eq!(receipts.len(), 3);
        assert_eq!(receipts[0].changes[0].operation, "materialized");
        assert_eq!(receipts[1].changes[0].operation, "dematerialized");
        assert_eq!(receipts[2].changes[0].member_id, "john");
        assert_eq!(receipts[0].previous_resolution_epoch, 0);
        assert_eq!(receipts[0].resolution_epoch, 1);
        assert_eq!(receipts[1].resolution_epoch, 2);
        assert_eq!(receipts[2].resolution_epoch, 3);
    }

    #[tokio::test]
    async fn directly_addressed_persona_must_commit_an_observable_response() {
        let dir = tempfile::tempdir().unwrap();
        let store = CampaignStore::open(dir.path().join("campaign.cc")).unwrap();
        let kernel = WorldKernel::start(store.clone());
        let mut seed = campaign();
        seed.actors.insert(
            "anna".into(),
            ActorState {
                id: "anna".into(),
                name: "Anna".into(),
                location_id: "room".into(),
                capabilities: BTreeSet::new(),
                knowledge: BTreeSet::new(),
                equipment: BTreeSet::new(),
                conditions: BTreeSet::new(),
                obligations: BTreeSet::new(),
                relationships: BTreeMap::new(),
                goals: vec![],
                memories: vec![],
            },
        );
        seed.transcript.push(NarrativeTurn {
            revision: 0,
            at: seed.world_time,
            speaker: "player".into(),
            text: "Anna, answer me.".into(),
            persona_response_actor_ids: BTreeSet::from(["anna".into()]),
        });
        kernel
            .command(WorldCommand::CreateCampaign {
                campaign: seed.clone(),
                evidence_receipts: vec![],
                model_stage_receipts: vec![],
            })
            .await
            .unwrap();

        let silent = ActorReaction {
            actor_id: "anna".into(),
            speech: None,
            deliberate_silence: false,
            private_delta: ActorStateDelta::default(),
            action_proposals: vec![],
        };
        let error = kernel
            .command(WorldCommand::ResolveReactionWave {
                expected_revision: 0,
                event_summary: "player says: Anna, answer me.".into(),
                reactions: vec![silent],
            })
            .await
            .unwrap_err();
        assert!(error.to_string().contains("no observable response"));
        assert_eq!(
            store
                .load::<Campaign>("campaign.v1", &seed.id.to_string())
                .unwrap()
                .unwrap()
                .1
                .revision,
            0
        );

        let committed = kernel
            .command(WorldCommand::ResolveReactionWave {
                expected_revision: 0,
                event_summary: "player says: Anna, answer me.".into(),
                reactions: vec![ActorReaction {
                    actor_id: "anna".into(),
                    speech: None,
                    deliberate_silence: true,
                    private_delta: ActorStateDelta::default(),
                    action_proposals: vec![],
                }],
            })
            .await
            .unwrap();
        let CommandResult::Committed { campaign, .. } = committed else {
            panic!()
        };
        assert_eq!(campaign.revision, 1);
        assert_eq!(
            campaign.transcript.last().unwrap().text,
            "deliberately does not answer."
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
                model_stage_receipts: vec![],
            })
            .await
            .unwrap();
        let reactions = vec![
            ActorReaction {
                actor_id: "anna".into(),
                speech: Some("I saw that.".into()),
                deliberate_silence: false,
                private_delta: ActorStateDelta {
                    memories_add: vec!["saw the event".into()],
                    ..Default::default()
                },
                action_proposals: vec![],
            },
            ActorReaction {
                actor_id: "bert".into(),
                speech: None,
                deliberate_silence: false,
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

    #[tokio::test]
    async fn reaction_interpreter_cannot_write_actor_memory() {
        let dir = tempfile::tempdir().unwrap();
        let store = CampaignStore::open(dir.path().join("campaign.cc")).unwrap();
        let kernel = WorldKernel::start(store.clone());
        let mut seed = campaign();
        seed.transcript.push(NarrativeTurn {
            revision: 0,
            at: seed.world_time,
            speaker: "player".into(),
            text: "Tell me which seal I repaired.".into(),
            persona_response_actor_ids: BTreeSet::new(),
        });
        seed.actors.insert(
            "anna".into(),
            ActorState {
                id: "anna".into(),
                name: "Anna".into(),
                location_id: "room".into(),
                capabilities: BTreeSet::new(),
                knowledge: BTreeSet::new(),
                equipment: BTreeSet::new(),
                conditions: BTreeSet::new(),
                obligations: BTreeSet::new(),
                relationships: BTreeMap::new(),
                goals: vec![],
                memories: vec![],
            },
        );
        kernel
            .command(WorldCommand::CreateCampaign {
                campaign: seed.clone(),
                evidence_receipts: vec![],
                model_stage_receipts: vec![],
            })
            .await
            .unwrap();

        let result = kernel
            .command(WorldCommand::ResolveReactionWave {
                expected_revision: 0,
                event_summary: "player says: Tell me which seal I repaired.".into(),
                reactions: vec![ActorReaction {
                    actor_id: "anna".into(),
                    speech: None,
                    deliberate_silence: false,
                    private_delta: ActorStateDelta {
                        memories_add: vec!["I repaired the seal.".into()],
                        ..Default::default()
                    },
                    action_proposals: vec![],
                }],
            })
            .await;
        assert!(
            matches!(result, Err(KernelError::Invalid(message)) if message.contains("cannot write actor memory"))
        );
        let identity_result = kernel
            .command(WorldCommand::ResolveReactionWave {
                expected_revision: 0,
                event_summary: "player says: Tell me which seal I repaired.".into(),
                reactions: vec![ActorReaction {
                    actor_id: "anna".into(),
                    speech: Some("My name is Anna.".into()),
                    deliberate_silence: false,
                    private_delta: ActorStateDelta {
                        identity_adoption: Some("Taren".into()),
                        ..Default::default()
                    },
                    action_proposals: vec![],
                }],
            })
            .await;
        assert!(
            matches!(identity_result, Err(KernelError::Invalid(message)) if message.contains("exact spoken handle"))
        );
        let persisted = store
            .load::<Campaign>("campaign.v1", &seed.id.to_string())
            .unwrap()
            .unwrap()
            .1;
        assert_eq!(persisted.revision, 0);
        assert!(persisted.actors["anna"].memories.is_empty());
    }

    #[tokio::test]
    async fn reaction_memory_preserves_exact_committed_speaker_attribution() {
        let dir = tempfile::tempdir().unwrap();
        let store = CampaignStore::open(dir.path().join("campaign.cc")).unwrap();
        let kernel = WorldKernel::start(store.clone());
        let mut seed = campaign();
        seed.transcript.push(NarrativeTurn {
            revision: 0,
            at: seed.world_time,
            speaker: "player".into(),
            text: "Tell me which seal I repaired.".into(),
            persona_response_actor_ids: BTreeSet::new(),
        });
        seed.actors.insert(
            "anna".into(),
            ActorState {
                id: "anna".into(),
                name: "Anna".into(),
                location_id: "room".into(),
                capabilities: BTreeSet::new(),
                knowledge: BTreeSet::new(),
                equipment: BTreeSet::new(),
                conditions: BTreeSet::new(),
                obligations: BTreeSet::new(),
                relationships: BTreeMap::new(),
                goals: vec![],
                memories: vec![],
            },
        );
        kernel
            .command(WorldCommand::CreateCampaign {
                campaign: seed.clone(),
                evidence_receipts: vec![],
                model_stage_receipts: vec![],
            })
            .await
            .unwrap();

        kernel
            .command(WorldCommand::ResolveReactionWave {
                expected_revision: 0,
                event_summary: "player says: Tell me which seal I repaired.".into(),
                reactions: vec![ActorReaction {
                    actor_id: "anna".into(),
                    speech: Some("I did not witness it.".into()),
                    deliberate_silence: false,
                    private_delta: ActorStateDelta::default(),
                    action_proposals: vec![],
                }],
            })
            .await
            .unwrap();
        let persisted = store
            .load::<Campaign>("campaign.v1", &seed.id.to_string())
            .unwrap()
            .unwrap()
            .1;
        assert_eq!(
            persisted.actors["anna"].memories,
            vec!["Witnessed: player says: Tell me which seal I repaired."]
        );
        assert!(!persisted.actors["anna"].memories[0].contains("Anna repaired"));
    }

    #[tokio::test]
    async fn initiative_grants_one_npc_opportunity_without_faking_player_activity() {
        let dir = tempfile::tempdir().unwrap();
        let store = CampaignStore::open(dir.path().join("campaign.cc")).unwrap();
        let kernel = WorldKernel::start(store.clone());
        let mut seed = campaign();
        seed.transcript.push(NarrativeTurn {
            revision: 0,
            at: seed.world_time,
            speaker: "world".into(),
            text: "a disturbance".into(),
            persona_response_actor_ids: BTreeSet::new(),
        });
        seed.last_player_activity = Utc::now() - Duration::hours(2);
        for id in ["anna", "bert"] {
            seed.actors.insert(
                id.into(),
                ActorState {
                    id: id.into(),
                    name: id.into(),
                    location_id: "room".into(),
                    capabilities: BTreeSet::from(["intervene".into()]),
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
        let activity = seed.last_player_activity;
        let campaign_id = seed.id;
        kernel
            .command(WorldCommand::CreateCampaign {
                campaign: seed,
                evidence_receipts: vec![],
                model_stage_receipts: vec![],
            })
            .await
            .unwrap();
        let proposal = |actor: &str, priority| WorldActionProposal {
            actor_id: actor.into(),
            intent: "intervene".into(),
            intended_effect: "take control of the immediate situation".into(),
            priority,
            state_references: vec!["capability:intervene".into(), "location:room".into()],
        };
        let anna = proposal("anna", 9);
        let bert = proposal("bert", 4);
        let wave = kernel
            .command(WorldCommand::ResolveReactionWave {
                expected_revision: 0,
                event_summary: "a disturbance".into(),
                reactions: vec![
                    ActorReaction {
                        actor_id: "anna".into(),
                        speech: None,
                        deliberate_silence: false,
                        private_delta: ActorStateDelta::default(),
                        action_proposals: vec![anna.clone()],
                    },
                    ActorReaction {
                        actor_id: "bert".into(),
                        speech: None,
                        deliberate_silence: false,
                        private_delta: ActorStateDelta::default(),
                        action_proposals: vec![bert.clone()],
                    },
                ],
            })
            .await
            .unwrap();
        let assessment_for = |actor: &str| ActionIntent {
            actor_id: actor.into(),
            description: "intervene".into(),
            intended_effect: "take control of the immediate situation".into(),
        };
        let CommandResult::Assessed {
            assessment: bert_assessment,
        } = kernel
            .command(WorldCommand::Assess {
                expected_revision: 1,
                intent: assessment_for("bert"),
                proposal: None,
            })
            .await
            .unwrap()
        else {
            panic!()
        };
        assert!(
            kernel
                .command(WorldCommand::ResolveNpcAction {
                    expected_revision: 1,
                    proposal: bert,
                    assessment: bert_assessment,
                })
                .await
                .is_err()
        );
        let CommandResult::Assessed { assessment } = kernel
            .command(WorldCommand::Assess {
                expected_revision: 1,
                intent: assessment_for("anna"),
                proposal: None,
            })
            .await
            .unwrap()
        else {
            panic!()
        };
        let mut malformed = assessment.clone();
        malformed
            .success_effect
            .actor_conditions
            .insert("invented-actor".into(), ConditionDelta::default());
        malformed.digest = crate::assessor::assessment_digest(&malformed).unwrap();
        assert!(
            kernel
                .command(WorldCommand::ResolveNpcAction {
                    expected_revision: 1,
                    proposal: anna.clone(),
                    assessment: malformed,
                })
                .await
                .is_err()
        );
        let unchanged = store
            .load::<Campaign>("campaign.v1", &campaign_id.to_string())
            .unwrap()
            .unwrap()
            .1;
        assert_eq!(unchanged.revision, 1);
        assert_eq!(unchanged.pending_world_proposals.len(), 2);
        let resolved = kernel
            .command(WorldCommand::ResolveNpcAction {
                expected_revision: 1,
                proposal: anna,
                assessment,
            })
            .await
            .unwrap();
        let CommandResult::Committed {
            campaign: resolved, ..
        } = resolved
        else {
            panic!()
        };
        assert!(resolved.pending_world_proposals.is_empty());
        let persisted = store
            .load::<Campaign>("campaign.v1", &resolved.id.to_string())
            .unwrap()
            .unwrap()
            .1;
        assert_eq!(persisted.last_player_activity, activity);
        assert_eq!(persisted.revision, 2);
        let CommandResult::Committed { campaign, .. } = wave else {
            panic!()
        };
        assert_eq!(campaign.pending_world_proposals.len(), 2);
    }

    #[tokio::test]
    async fn npc_initiative_cannot_rebase_across_a_new_foreground_revision() {
        let dir = tempfile::tempdir().unwrap();
        let store = CampaignStore::open(dir.path().join("campaign.cc")).unwrap();
        let kernel = WorldKernel::start(store.clone());
        let mut seed = campaign();
        seed.transcript.push(NarrativeTurn {
            revision: 0,
            at: seed.world_time,
            speaker: "world".into(),
            text: "a disturbance".into(),
            persona_response_actor_ids: BTreeSet::new(),
        });
        seed.actors.insert(
            "anna".into(),
            ActorState {
                id: "anna".into(),
                name: "Anna".into(),
                location_id: "room".into(),
                capabilities: BTreeSet::from(["intervene".into()]),
                knowledge: BTreeSet::new(),
                equipment: BTreeSet::new(),
                conditions: BTreeSet::new(),
                obligations: BTreeSet::new(),
                relationships: BTreeMap::new(),
                goals: vec![],
                memories: vec![],
            },
        );
        let campaign_id = seed.id;
        kernel
            .command(WorldCommand::CreateCampaign {
                campaign: seed,
                evidence_receipts: vec![],
                model_stage_receipts: vec![],
            })
            .await
            .unwrap();
        let proposal = WorldActionProposal {
            actor_id: "anna".into(),
            intent: "intervene".into(),
            intended_effect: "take control of the immediate situation".into(),
            priority: 9,
            state_references: vec!["capability:intervene".into(), "location:room".into()],
        };
        kernel
            .command(WorldCommand::ResolveReactionWave {
                expected_revision: 0,
                event_summary: "a disturbance".into(),
                reactions: vec![ActorReaction {
                    actor_id: "anna".into(),
                    speech: None,
                    deliberate_silence: false,
                    private_delta: ActorStateDelta::default(),
                    action_proposals: vec![proposal.clone()],
                }],
            })
            .await
            .unwrap();
        kernel
            .command(WorldCommand::Speak {
                expected_revision: 1,
                actor_id: "player".into(),
                text: "I move on before Anna acts.".into(),
                intended_effect: None,
                persona_response_actor_ids: BTreeSet::new(),
            })
            .await
            .unwrap();
        let intent = ActionIntent {
            actor_id: "anna".into(),
            description: proposal.intent.clone(),
            intended_effect: proposal.intended_effect.clone(),
        };
        let CommandResult::Assessed { assessment } = kernel
            .command(WorldCommand::Assess {
                expected_revision: 2,
                intent,
                proposal: None,
            })
            .await
            .unwrap()
        else {
            panic!()
        };
        let error = kernel
            .command(WorldCommand::ResolveNpcAction {
                expected_revision: 2,
                proposal,
                assessment,
            })
            .await
            .unwrap_err();
        assert!(error.to_string().contains("current reaction wave"));

        let refreshed = kernel
            .command(WorldCommand::ResolveReactionWave {
                expected_revision: 2,
                event_summary: "player says: I move on before Anna acts.".into(),
                reactions: vec![ActorReaction {
                    actor_id: "anna".into(),
                    speech: None,
                    deliberate_silence: false,
                    private_delta: ActorStateDelta::default(),
                    action_proposals: vec![],
                }],
            })
            .await
            .unwrap();
        let CommandResult::Committed { campaign, .. } = refreshed else {
            panic!()
        };
        assert_eq!(campaign.revision, 3);
        assert!(campaign.pending_world_proposals.is_empty());
        let persisted = store
            .load::<Campaign>("campaign.v1", &campaign_id.to_string())
            .unwrap()
            .unwrap()
            .1;
        assert!(persisted.pending_world_proposals.is_empty());
    }

    #[tokio::test]
    async fn roll_commits_only_the_prevalidated_typed_outcome_delta() {
        let dir = tempfile::tempdir().unwrap();
        let store = CampaignStore::open(dir.path().join("campaign.cc")).unwrap();
        let kernel = WorldKernel::start(store.clone());
        let mut seed = campaign();
        let player_location = seed.actors["player"].location_id.clone();
        seed.facts.insert(
            "fact:door-brace-seated".into(),
            WorldFact {
                id: "fact:door-brace-seated".into(),
                statement: "The door brace is seated against the frame.".into(),
                scope: FactScope::BranchLocal,
                evidence_receipt_ids: vec![],
                discoverable_at_location_ids: BTreeSet::from([player_location]),
            },
        );
        let campaign_id = seed.id;
        kernel
            .command(WorldCommand::CreateCampaign {
                campaign: seed.clone(),
                evidence_receipts: vec![],
                model_stage_receipts: vec![],
            })
            .await
            .unwrap();
        let intent = ActionIntent {
            actor_id: "player".into(),
            description: "brace the door".into(),
            intended_effect: "become braced".into(),
        };
        let mut invalid = assess(&seed, intent.clone());
        invalid
            .success_effect
            .actor_moves
            .insert("player".into(), "nowhere".into());
        invalid.digest = crate::assessor::assessment_digest(&invalid).unwrap();
        assert!(
            kernel
                .command(WorldCommand::Assess {
                    expected_revision: 0,
                    intent: intent.clone(),
                    proposal: Some(invalid),
                })
                .await
                .is_err()
        );
        let mut hidden_finding = assess(&seed, intent.clone());
        hidden_finding
            .success_effect
            .actor_knowledge_additions
            .insert(
                "player".into(),
                BTreeSet::from(["The hidden latch is broken.".into()]),
            );
        hidden_finding.success_stake = "The hidden latch is broken.".into();
        hidden_finding.digest = crate::assessor::assessment_digest(&hidden_finding).unwrap();
        assert!(
            kernel
                .command(WorldCommand::Assess {
                    expected_revision: 0,
                    intent: intent.clone(),
                    proposal: Some(hidden_finding),
                })
                .await
                .is_err()
        );
        let delta = WorldEffectDelta {
            actor_conditions: BTreeMap::from([(
                "player".into(),
                ConditionDelta {
                    add: BTreeSet::from(["braced".into()]),
                    remove: BTreeSet::new(),
                },
            )]),
            actor_knowledge_additions: BTreeMap::from([(
                "player".into(),
                BTreeSet::from(["The door brace is seated against the frame.".into()]),
            )]),
            ..Default::default()
        };
        let mut valid = assess(&seed, intent.clone());
        valid.success_stake =
            "The door brace is seated against the frame. You become braced.".into();
        valid.mixed_stake = valid.success_stake.clone();
        valid.failure_stake = valid.success_stake.clone();
        valid.strong_effect = delta.clone();
        valid.success_effect = delta.clone();
        valid.mixed_effect = delta.clone();
        valid.failure_effect = delta;
        valid.digest = crate::assessor::assessment_digest(&valid).unwrap();
        kernel
            .command(WorldCommand::Assess {
                expected_revision: 0,
                intent,
                proposal: Some(valid.clone()),
            })
            .await
            .unwrap();
        kernel
            .command(WorldCommand::Attempt {
                actor_id: "player".into(),
                assessment_digest: valid.digest,
            })
            .await
            .unwrap();
        let persisted = store
            .load::<Campaign>("campaign.v1", &campaign_id.to_string())
            .unwrap()
            .unwrap()
            .1;
        assert_eq!(persisted.revision, 1);
        assert!(persisted.actors["player"].conditions.contains("braced"));
        assert!(
            persisted.actors["player"]
                .knowledge
                .contains("The door brace is seated against the frame.")
        );
        assert!(persisted.facts.values().any(|fact| {
            fact.statement == "The door brace is seated against the frame."
                && fact.scope == FactScope::BranchLocal
        }));
        assert_eq!(persisted.facts.len(), 1);
    }

    #[test]
    fn information_effects_reveal_existing_accessible_facts_only() {
        let statement = "The maintenance panel records a seven-minute interruption.";
        let mut seed = campaign();
        seed.facts.insert(
            "fact:panel-interruption".into(),
            WorldFact {
                id: "fact:panel-interruption".into(),
                statement: statement.into(),
                scope: FactScope::BranchLocal,
                evidence_receipt_ids: vec![],
                discoverable_at_location_ids: BTreeSet::from(["room".into()]),
            },
        );
        let effect = WorldEffectDelta {
            actor_knowledge_additions: BTreeMap::from([(
                "player".into(),
                BTreeSet::from([statement.into()]),
            )]),
            ..Default::default()
        };
        assert!(
            crate::assessor::validate_effect(&seed, &seed.actors["player"], &effect, statement,)
                .is_ok()
        );

        let mut absent = seed.clone();
        absent.facts.clear();
        assert!(
            crate::assessor::validate_effect(
                &absent,
                &absent.actors["player"],
                &effect,
                statement,
            )
            .unwrap_err()
            .to_string()
            .contains("existing accessible WorldFact")
        );

        let mut wrong_place = seed.clone();
        wrong_place
            .facts
            .get_mut("fact:panel-interruption")
            .unwrap()
            .discoverable_at_location_ids = BTreeSet::from(["elsewhere".into()]);
        assert!(
            crate::assessor::validate_effect(
                &wrong_place,
                &wrong_place.actors["player"],
                &effect,
                statement,
            )
            .is_err()
        );

        let mut speaker = seed.actors["player"].clone();
        speaker.id = "speaker".into();
        speaker.name = "Speaker".into();
        speaker.knowledge.insert(statement.into());
        seed.actors.insert(speaker.id.clone(), speaker.clone());
        seed.facts
            .get_mut("fact:panel-interruption")
            .unwrap()
            .discoverable_at_location_ids
            .clear();
        assert!(
            crate::assessor::validate_effect(&seed, &speaker, &effect, statement).is_ok(),
            "an actor may communicate an existing fact they know to another present actor"
        );
    }

    fn resolution_stage(
        cell_id: &str,
        campaign: &Campaign,
        stage: &str,
        marker: char,
    ) -> crate::model::ModelStageReceipt {
        let hash = format!("sha256:{}", marker.to_string().repeat(64));
        crate::model::ModelStageReceipt {
            schema: "ghostlight.persona_stage_receipt.v1".into(),
            receipt_hash: hash.clone(),
            provider: "fixture".into(),
            model: "fixture".into(),
            stage: stage.into(),
            snapshot_binding: format!(
                "campaign:{}:revision:{}:resolution:{}:cell:{}",
                campaign.id,
                campaign.revision,
                campaign.resolution_policy.resolution_epoch,
                cell_id
            ),
            request_hash: format!("sha256:{}", "e".repeat(64)),
            output_hash: format!("sha256:{}", "f".repeat(64)),
            source_receipt_ids: vec![],
            latency_ms: 1,
            validation_result: "valid".into(),
            local_validation_error: None,
            input_chars: 7,
            output_chars: 7,
            provider_attempts: vec![],
        }
    }

    fn inaction_wave(
        campaign: &Campaign,
        store: &CampaignStore,
    ) -> crate::domain::ResolutionWaveCommit {
        let cover = crate::resolution::plan_cover(
            campaign,
            crate::resolution::default_demand(campaign, "kernel fixture"),
        )
        .unwrap();
        let mut hashes = Vec::new();
        for (cell_index, cell) in cover.cells.iter().enumerate() {
            for (stage_index, stage) in ["cell_projector", "cell_persona", "cell_interpreter"]
                .into_iter()
                .enumerate()
            {
                let marker =
                    char::from_u32(u32::from(b'a') + (cell_index * 3 + stage_index) as u32)
                        .unwrap();
                let receipt = resolution_stage(&cell.id, campaign, stage, marker);
                store
                    .insert(
                        "persona_stage_receipt.v1",
                        "ghostlight.persona_stage_receipt.v1",
                        receipt.storage_key(),
                        &receipt,
                    )
                    .unwrap();
                hashes.push(receipt.storage_key().to_owned());
            }
        }
        crate::domain::ResolutionWaveCommit {
            schema: "ghostlight.resolution_wave_commit.v1".into(),
            world_revision: campaign.revision,
            resolution_epoch: campaign.resolution_policy.resolution_epoch,
            plan_receipt: crate::resolution::plan_receipt(campaign, &cover),
            appraisals: cover
                .cells
                .iter()
                .map(|cell| CellAppraisal {
                    schema: "ghostlight.cell_appraisal.v1".into(),
                    cell_id: cell.id.clone(),
                    world_revision: campaign.revision,
                    resolution_epoch: campaign.resolution_policy.resolution_epoch,
                    considered_subject_ids: cell.subject_ids.clone(),
                    actions: vec![],
                    inactions: cell
                        .subject_ids
                        .iter()
                        .next()
                        .map(|subject_id| crate::domain::CellInaction {
                            subject_id: subject_id.clone(),
                            reason: "No justified move.".into(),
                        })
                        .into_iter()
                        .collect(),
                })
                .collect(),
            cover,
            activity_outcomes: vec![],
            model_receipt_hashes: hashes,
        }
    }

    #[tokio::test]
    async fn resolution_budget_commits_without_advancing_fictional_state() {
        let dir = tempfile::tempdir().unwrap();
        let store = CampaignStore::open(dir.path().join("campaign.cc")).unwrap();
        let kernel = WorldKernel::start(store.clone());
        let seed = crate::resolution::tests::campaign(6, 8);
        let world_time = seed.world_time;
        kernel
            .command(WorldCommand::CreateCampaign {
                campaign: seed.clone(),
                evidence_receipts: vec![],
                model_stage_receipts: vec![],
            })
            .await
            .unwrap();
        let result = kernel
            .command(WorldCommand::SetResolutionBudget {
                expected_revision: 0,
                expected_resolution_epoch: 0,
                active_cell_budget: 3,
            })
            .await
            .unwrap();
        let CommandResult::ResolutionUpdated { campaign, receipt } = result else {
            panic!("resolution command used the world commit path")
        };
        assert_eq!(campaign.revision, 0);
        assert_eq!(campaign.world_time, world_time);
        assert_eq!(campaign.resolution_policy.active_cell_budget, 3);
        assert_eq!(campaign.resolution_policy.resolution_epoch, 1);
        assert_eq!(receipt.previous_resolution_epoch, 0);
        let unchanged = kernel
            .command(WorldCommand::SetResolutionBudget {
                expected_revision: 0,
                expected_resolution_epoch: 1,
                active_cell_budget: 3,
            })
            .await
            .unwrap_err();
        assert!(unchanged.to_string().contains("does not change"));
        assert!(
            kernel
                .command(WorldCommand::SetResolutionBudget {
                    expected_revision: 0,
                    expected_resolution_epoch: 0,
                    active_cell_budget: 4,
                })
                .await
                .is_err()
        );
        assert_eq!(
            store.keys("resolution_control_receipt.v1").unwrap().len(),
            1
        );
    }

    #[tokio::test]
    async fn provider_parallelism_changes_without_repartitioning_the_world() {
        let dir = tempfile::tempdir().unwrap();
        let store = CampaignStore::open(dir.path().join("campaign.cc")).unwrap();
        let kernel = WorldKernel::start(store);
        let mut seed = crate::resolution::tests::campaign(6, 3);
        let cover = crate::resolution::plan_cover(
            &seed,
            crate::resolution::default_demand(&seed, "existing cover"),
        )
        .unwrap();
        seed.resolution_cover = Some(cover.clone());
        let world_time = seed.world_time;
        kernel
            .command(WorldCommand::CreateCampaign {
                campaign: seed,
                evidence_receipts: vec![],
                model_stage_receipts: vec![],
            })
            .await
            .unwrap();
        let result = kernel
            .command(WorldCommand::SetProviderParallelism {
                expected_revision: 0,
                expected_provider_configuration_epoch: 0,
                provider_parallelism: 4,
            })
            .await
            .unwrap();
        let CommandResult::ResolutionUpdated { campaign, receipt } = result else {
            panic!("operator control used the fictional world commit path")
        };
        assert_eq!(campaign.revision, 0);
        assert_eq!(campaign.world_time, world_time);
        assert_eq!(campaign.resolution_policy.resolution_epoch, 0);
        assert_eq!(campaign.resolution_policy.provider_configuration_epoch, 1);
        assert_eq!(campaign.resolution_policy.provider_parallelism, 4);
        assert_eq!(campaign.resolution_cover, Some(cover));
        assert_eq!(receipt.operation, "set_provider_parallelism");
        let unchanged = kernel
            .command(WorldCommand::SetProviderParallelism {
                expected_revision: 0,
                expected_provider_configuration_epoch: 1,
                provider_parallelism: 4,
            })
            .await
            .unwrap_err();
        assert!(unchanged.to_string().contains("does not change"));
        assert!(
            kernel
                .command(WorldCommand::SetProviderParallelism {
                    expected_revision: 0,
                    expected_provider_configuration_epoch: 0,
                    provider_parallelism: 2,
                })
                .await
                .is_err()
        );
    }

    #[tokio::test]
    async fn strategic_resolution_wave_commits_cover_and_all_appraisals_atomically() {
        let dir = tempfile::tempdir().unwrap();
        let store = CampaignStore::open(dir.path().join("campaign.cc")).unwrap();
        let kernel = WorldKernel::start(store.clone());
        let seed = crate::resolution::tests::campaign(6, 2);
        kernel
            .command(WorldCommand::CreateCampaign {
                campaign: seed.clone(),
                evidence_receipts: vec![],
                model_stage_receipts: vec![],
            })
            .await
            .unwrap();
        let persisted = store
            .load::<Campaign>("campaign.v1", &seed.id.to_string())
            .unwrap()
            .unwrap()
            .1;
        let wave = inaction_wave(&persisted, &store);
        let cell_count = wave.cover.cells.len();
        let result = kernel
            .command(WorldCommand::AdvanceStrategicTick {
                expected_revision: 0,
                source: TickSource::Scheduler,
                plan: None,
                model_receipt_hash: Some(format!("sha256:{}", "9".repeat(64))),
                resolution_wave: Some(wave),
            })
            .await
            .unwrap();
        let CommandResult::Committed { campaign, .. } = result else {
            panic!()
        };
        assert_eq!(campaign.revision, 1);
        assert_eq!(campaign.strategic_tick_count, 1);
        assert!(campaign.resolution_cover.is_some());
        assert_eq!(store.keys("cell_appraisal.v1").unwrap().len(), cell_count);
        assert_eq!(store.keys("resolution_plan_receipt.v1").unwrap().len(), 1);
        assert_eq!(store.keys("strategic_tick.v1").unwrap().len(), 1);
    }

    #[tokio::test]
    async fn invalid_effect_verifier_receipt_cannot_authorize_an_actionful_cell() {
        let dir = tempfile::tempdir().unwrap();
        let store = CampaignStore::open(dir.path().join("campaign.cc")).unwrap();
        let kernel = WorldKernel::start(store.clone());
        let seed = crate::resolution::tests::campaign(3, 1);
        kernel
            .command(WorldCommand::CreateCampaign {
                campaign: seed.clone(),
                evidence_receipts: vec![],
                model_stage_receipts: vec![],
            })
            .await
            .unwrap();
        let persisted = store
            .load::<Campaign>("campaign.v1", &seed.id.to_string())
            .unwrap()
            .unwrap()
            .1;
        let mut wave = inaction_wave(&persisted, &store);
        let subject_id = wave.cover.cells[0]
            .subject_ids
            .iter()
            .next()
            .unwrap()
            .clone();
        wave.appraisals[0].actions = vec![CellActionProposal {
            subject_id: subject_id.clone(),
            intent: "adopt a new position".into(),
            intended_effect: "publish a different commitment".into(),
            priority: 1,
            state_references: vec![],
            public_channels: vec![],
            effects: vec![StrategicCellEffect::Institution {
                institution_id: subject_id,
                posture: "publishing a bounded new commitment".into(),
                location_ids: vec![],
            }],
        }];
        wave.appraisals[0].inactions.clear();
        let mut rejected_verifier = resolution_stage(
            &wave.cover.cells[0].id,
            &persisted,
            "cell_effect_verifier",
            'd',
        );
        rejected_verifier.validation_result = "semantic_invalid".into();
        rejected_verifier.local_validation_error =
            Some("typed effect reverses the decision".into());
        store
            .insert(
                "persona_stage_receipt.v1",
                "ghostlight.persona_stage_receipt.v1",
                rejected_verifier.storage_key(),
                &rejected_verifier,
            )
            .unwrap();
        wave.model_receipt_hashes
            .push(rejected_verifier.storage_key().to_owned());

        let error = kernel
            .command(WorldCommand::AdvanceStrategicTick {
                expected_revision: 0,
                source: TickSource::Scheduler,
                plan: None,
                model_receipt_hash: Some(format!("sha256:{}", "9".repeat(64))),
                resolution_wave: Some(wave),
            })
            .await
            .unwrap_err();
        assert!(
            error
                .to_string()
                .contains("lacks action-bound cell_effect_verifier"),
            "unexpected kernel rejection: {error}"
        );
        let stored = store
            .load::<Campaign>("campaign.v1", &seed.id.to_string())
            .unwrap()
            .unwrap()
            .1;
        assert_eq!(stored.revision, 0);
        assert_eq!(stored.world_time, seed.world_time);
    }

    #[tokio::test]
    async fn activity_outcome_receipt_must_bind_the_exact_selected_digest_set() {
        let dir = tempfile::tempdir().unwrap();
        let store = CampaignStore::open(dir.path().join("campaign.cc")).unwrap();
        let kernel = WorldKernel::start(store.clone());
        let seed = hierarchical_refugee_campaign();
        kernel
            .command(WorldCommand::CreateCampaign {
                campaign: seed.clone(),
                evidence_receipts: vec![],
                model_stage_receipts: vec![],
            })
            .await
            .unwrap();
        let persisted = store
            .load::<Campaign>("campaign.v1", &seed.id.to_string())
            .unwrap()
            .unwrap()
            .1;
        let mut wave = inaction_wave(&persisted, &store);
        let appraisal = wave
            .appraisals
            .iter_mut()
            .find(|appraisal| appraisal.considered_subject_ids.contains("refugees-east"))
            .unwrap();
        let proposal = CellActionProposal {
            subject_id: "refugees-east".into(),
            intent: "prepare storm lashings".into(),
            intended_effect: "finish one bounded set of camp lashings".into(),
            priority: 60,
            state_references: vec![
                "subject:refugees-east".into(),
                "capability:survive transit".into(),
                "location:camp".into(),
            ],
            public_channels: vec![],
            effects: vec![StrategicCellEffect::GestaltActivity {
                gestalt_id: "refugees-east".into(),
                activity: StrategicActivityKind::Prepare,
                target_subject_ids: vec![],
                location_ids: vec!["camp".into()],
            }],
        };
        let action_digest = crate::resolution::cell_action_digest(&proposal).unwrap();
        appraisal.actions = vec![proposal];
        appraisal.inactions.clear();
        let appraisal_cell_id = appraisal.cell_id.clone();
        let appraisal_actions = appraisal.actions.clone();
        wave.activity_outcomes = vec![StrategicActivityOutcome {
            schema: "ghostlight.strategic_activity_outcome.v1".into(),
            action_digest: action_digest.clone(),
            source_subject_id: "refugees-east".into(),
            band: StrategicOutcomeBand::Success,
            summary: "The camp finishes one set of storm lashings.".into(),
            supporting_state_references: vec!["capability:survive transit".into()],
            effect: StrategicOutcomeEffect::ResourceCreated {
                owner_subject_id: "refugees-east".into(),
                resource: "storm lashings".into(),
            },
        }];

        let base_binding = format!(
            "campaign:{}:revision:{}:resolution:{}:cell:{}",
            persisted.id,
            persisted.revision,
            persisted.resolution_policy.resolution_epoch,
            appraisal_cell_id
        );
        let mut verifier =
            resolution_stage(&appraisal_cell_id, &persisted, "cell_effect_verifier", '7');
        verifier.snapshot_binding =
            crate::persona::cell_effect_verification_binding(&base_binding, &appraisal_actions)
                .unwrap();
        let wrong_outcome = resolution_stage(
            &appraisal_cell_id,
            &persisted,
            "strategic_outcome_resolver",
            '8',
        );
        for receipt in [verifier, wrong_outcome] {
            store
                .insert(
                    "persona_stage_receipt.v1",
                    "ghostlight.persona_stage_receipt.v1",
                    receipt.storage_key(),
                    &receipt,
                )
                .unwrap();
            wave.model_receipt_hashes
                .push(receipt.storage_key().to_owned());
        }
        let mut correct_wave = wave.clone();
        let mut correct_outcome = resolution_stage(
            &appraisal_cell_id,
            &persisted,
            "strategic_outcome_resolver",
            '9',
        );
        correct_outcome.snapshot_binding = crate::outcome::activity_outcome_binding(
            persisted.id,
            persisted.revision,
            persisted.resolution_policy.resolution_epoch,
            &[action_digest],
        );
        store
            .insert(
                "persona_stage_receipt.v1",
                "ghostlight.persona_stage_receipt.v1",
                correct_outcome.storage_key(),
                &correct_outcome,
            )
            .unwrap();
        correct_wave.model_receipt_hashes.pop();
        correct_wave
            .model_receipt_hashes
            .push(correct_outcome.storage_key().to_owned());

        let error = kernel
            .command(WorldCommand::AdvanceStrategicTick {
                expected_revision: 0,
                source: TickSource::Scheduler,
                plan: None,
                model_receipt_hash: Some(format!("sha256:{}", "9".repeat(64))),
                resolution_wave: Some(wave),
            })
            .await
            .unwrap_err();
        assert!(
            error
                .to_string()
                .contains("lacks an activity-bound strategic outcome receipt"),
            "unexpected kernel rejection: {error}"
        );
        let stored = store
            .load::<Campaign>("campaign.v1", &seed.id.to_string())
            .unwrap()
            .unwrap()
            .1;
        assert_eq!(stored.revision, 0);
        assert_eq!(stored.world_time, seed.world_time);
        assert!(
            !stored.gestalts["refugees-east"]
                .resources
                .contains("storm lashings")
        );

        let result = kernel
            .command(WorldCommand::AdvanceStrategicTick {
                expected_revision: 0,
                source: TickSource::Scheduler,
                plan: None,
                model_receipt_hash: Some(format!("sha256:{}", "a".repeat(64))),
                resolution_wave: Some(correct_wave),
            })
            .await
            .unwrap();
        let CommandResult::Committed { campaign, .. } = result else {
            panic!()
        };
        assert_eq!(campaign.revision, 1);
        assert!(
            campaign.gestalts["refugees-east"]
                .resources
                .contains("storm lashings")
        );
        assert_eq!(
            store.keys("strategic_activity_outcome.v1").unwrap().len(),
            1
        );
    }

    #[tokio::test]
    async fn stale_resolution_wave_leaves_time_clocks_and_campaign_unmutated() {
        let dir = tempfile::tempdir().unwrap();
        let store = CampaignStore::open(dir.path().join("campaign.cc")).unwrap();
        let kernel = WorldKernel::start(store.clone());
        let seed = crate::resolution::tests::campaign(4, 2);
        kernel
            .command(WorldCommand::CreateCampaign {
                campaign: seed.clone(),
                evidence_receipts: vec![],
                model_stage_receipts: vec![],
            })
            .await
            .unwrap();
        let persisted = store
            .load::<Campaign>("campaign.v1", &seed.id.to_string())
            .unwrap()
            .unwrap()
            .1;
        let wave = inaction_wave(&persisted, &store);
        kernel
            .command(WorldCommand::SetResolutionBudget {
                expected_revision: 0,
                expected_resolution_epoch: 0,
                active_cell_budget: 1,
            })
            .await
            .unwrap();
        let before = store
            .load::<Campaign>("campaign.v1", &seed.id.to_string())
            .unwrap()
            .unwrap()
            .1;
        assert!(
            kernel
                .command(WorldCommand::AdvanceStrategicTick {
                    expected_revision: 0,
                    source: TickSource::Scheduler,
                    plan: None,
                    model_receipt_hash: Some(format!("sha256:{}", "8".repeat(64))),
                    resolution_wave: Some(wave),
                })
                .await
                .is_err()
        );
        let after = store
            .load::<Campaign>("campaign.v1", &seed.id.to_string())
            .unwrap()
            .unwrap()
            .1;
        assert_eq!(after, before);
        assert!(store.keys("strategic_tick.v1").unwrap().is_empty());
    }

    fn two_player_membership(campaign: &mut Campaign) -> CampaignMembership {
        let mut guest = campaign.actors["player"].clone();
        guest.id = "guest".into();
        guest.name = "Guest".into();
        campaign.actors.insert(guest.id.clone(), guest);
        crate::resolution::ensure_agency_profiles(campaign);
        campaign
            .agency_profiles
            .get_mut("player")
            .unwrap()
            .simulation_eligible = false;
        campaign
            .agency_profiles
            .get_mut("guest")
            .unwrap()
            .simulation_eligible = false;
        CampaignMembership {
            schema: "ghostlight.campaign_membership.v1".into(),
            campaign_id: campaign.id,
            governance_epoch: 0,
            host_member_id: "member:host".into(),
            members: BTreeMap::from([
                (
                    "member:host".into(),
                    crate::session_zero::CampaignMember {
                        member_id: "member:host".into(),
                        account_hash: "account:host".into(),
                        display_name: "Host".into(),
                        actor_id: "player".into(),
                        is_host: true,
                        active: true,
                        cell_allowance: 8,
                    },
                ),
                (
                    "member:guest".into(),
                    crate::session_zero::CampaignMember {
                        member_id: "member:guest".into(),
                        account_hash: "account:guest".into(),
                        display_name: "Guest".into(),
                        actor_id: "guest".into(),
                        is_host: false,
                        active: true,
                        cell_allowance: 8,
                    },
                ),
            ]),
            extraordinary_permissions: BTreeMap::new(),
        }
    }

    #[tokio::test]
    async fn group_travel_retries_uncommitted_unanimous_approval_and_moves_every_member_once() {
        let dir = tempfile::tempdir().unwrap();
        let store = CampaignStore::open(dir.path().join("campaign.cc")).unwrap();
        let mut seed = campaign();
        seed.locations.insert(
            "harbor".into(),
            Location {
                id: "harbor".into(),
                name: "Harbor".into(),
                container_id: None,
                routes: BTreeMap::new(),
                persistent_features: vec!["stone quay".into()],
            },
        );
        seed.locations.get_mut("room").unwrap().routes.insert(
            "harbor".into(),
            Route {
                destination_id: "harbor".into(),
                distance: "nearby".into(),
                travel_minutes: 20,
            },
        );
        seed.locations.insert(
            "yard".into(),
            Location {
                id: "yard".into(),
                name: "Yard".into(),
                container_id: None,
                routes: BTreeMap::from([(
                    "harbor".into(),
                    Route {
                        destination_id: "harbor".into(),
                        distance: "farther away".into(),
                        travel_minutes: 45,
                    },
                )]),
                persistent_features: vec!["freight apron".into()],
            },
        );
        let membership = two_player_membership(&mut seed);
        let campaign_id = seed.id;
        let start_time = seed.world_time;
        let kernel = WorldKernel::start(store.clone());
        kernel
            .command(WorldCommand::CreateCampaign {
                campaign: seed,
                evidence_receipts: vec![],
                model_stage_receipts: vec![],
            })
            .await
            .unwrap();
        store
            .insert(
                "campaign_membership.v1",
                "ghostlight.campaign_membership.v1",
                &campaign_id.to_string(),
                &membership,
            )
            .unwrap();
        let pending = kernel
            .command(WorldCommand::ProposeGroupTravel {
                expected_revision: 0,
                member_id: "member:host".into(),
                destination_location_id: "harbor".into(),
            })
            .await
            .unwrap();
        let CommandResult::TravelGovernancePending { proposal, campaign } = pending else {
            panic!("first approval committed a two-member proposal")
        };
        assert_eq!(campaign.revision, 0);
        assert_eq!(campaign.actors["player"].location_id, "room");
        let proposal_id = proposal.id;
        let (proposal_row, mut persisted_proposal) = store
            .load::<GroupTravelProposal>("group_travel_proposal.v1", &proposal_id)
            .unwrap()
            .unwrap();
        persisted_proposal.approvals.insert("member:guest".into());
        store
            .replace(
                &proposal_row,
                "ghostlight.group_travel_proposal.v1",
                &persisted_proposal,
            )
            .unwrap();
        let committed = kernel
            .command(WorldCommand::ApproveGroupTravel {
                expected_revision: 0,
                proposal_id: proposal_id.clone(),
                member_id: "member:host".into(),
            })
            .await
            .unwrap();
        let CommandResult::Committed { campaign, receipt } = committed else {
            panic!("unanimous travel did not commit")
        };
        assert_eq!(receipt.command_kind, "unanimous_group_travel");
        assert_eq!(campaign.revision, 1);
        assert_eq!(campaign.world_time, start_time + Duration::minutes(20));
        assert_eq!(campaign.actors["player"].location_id, "harbor");
        assert_eq!(campaign.actors["guest"].location_id, "harbor");
        assert_eq!(
            campaign.agency_profiles["player"].location_ids,
            BTreeSet::from(["harbor".into()])
        );
        assert_eq!(
            campaign.agency_profiles["guest"].location_ids,
            BTreeSet::from(["harbor".into()])
        );
        assert_eq!(
            store.keys("mutation_authority_envelope.v1").unwrap().len(),
            1
        );
        assert_eq!(store.keys("world_mutation_batch.v1").unwrap().len(), 1);
        assert_eq!(store.keys("world_mutation_receipt.v1").unwrap().len(), 1);
        let mut reload_projection = campaign.clone();
        crate::resolution::ensure_agency_profiles(&mut reload_projection);
        assert_eq!(reload_projection, campaign);
        assert!(
            kernel
                .command(WorldCommand::ApproveGroupTravel {
                    expected_revision: 0,
                    proposal_id,
                    member_id: "member:guest".into(),
                })
                .await
                .is_err()
        );
        assert_eq!(
            store
                .load::<Campaign>("campaign.v1", &campaign_id.to_string())
                .unwrap()
                .unwrap()
                .1
                .revision,
            1
        );
    }

    #[tokio::test]
    async fn kernel_rejects_player_effects_against_another_controlled_actor() {
        let dir = tempfile::tempdir().unwrap();
        let store = CampaignStore::open(dir.path().join("campaign.cc")).unwrap();
        let mut seed = campaign();
        let membership = two_player_membership(&mut seed);
        let kernel = WorldKernel::start(store.clone());
        kernel
            .command(WorldCommand::CreateCampaign {
                campaign: seed.clone(),
                evidence_receipts: vec![],
                model_stage_receipts: vec![],
            })
            .await
            .unwrap();
        store
            .insert(
                "campaign_membership.v1",
                "ghostlight.campaign_membership.v1",
                &seed.id.to_string(),
                &membership,
            )
            .unwrap();
        let intent = ActionIntent {
            actor_id: "player".into(),
            description: "shove Guest".into(),
            intended_effect: "make Guest prone".into(),
        };
        let mut assessment = assess(&seed, intent.clone());
        assessment.success_effect.actor_conditions.insert(
            "guest".into(),
            ConditionDelta {
                add: BTreeSet::from(["prone".into()]),
                remove: BTreeSet::new(),
            },
        );
        assessment.digest = crate::assessor::assessment_digest(&assessment).unwrap();
        let error = kernel
            .command(WorldCommand::Assess {
                expected_revision: 0,
                intent,
                proposal: Some(assessment),
            })
            .await
            .unwrap_err();
        assert!(error.to_string().contains("player-versus-player"));
        let stored = store
            .load::<Campaign>("campaign.v1", &seed.id.to_string())
            .unwrap()
            .unwrap()
            .1;
        assert_eq!(stored.revision, 0);
        assert!(!stored.actors["guest"].conditions.contains("prone"));
    }
}
