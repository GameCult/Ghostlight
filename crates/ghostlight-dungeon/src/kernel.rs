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
    ExternalSnapshotCommitted {
        campaign: Campaign,
        receipt: crate::consumer::ExternalSnapshotReceipt,
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
    Elaboration(crate::elaboration::FinalizedWorldElaboration),
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
    pub(crate) fn initialize_campaign(
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
                    KernelInput::Elaboration(finalized) => {
                        execute_finalized_elaboration(&store, &mut assessments, finalized)
                    }
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

    pub async fn commit_elaboration(
        &self,
        finalized: crate::elaboration::FinalizedWorldElaboration,
    ) -> Result<CommandResult, KernelError> {
        let (reply, receive) = oneshot::channel();
        self.tx
            .send(Request {
                input: KernelInput::Elaboration(finalized),
                reply,
            })
            .await
            .map_err(|_| KernelError::Invalid("kernel stopped".into()))?;
        receive
            .await
            .map_err(|_| KernelError::Invalid("kernel stopped".into()))?
    }
}

fn execute_finalized_elaboration(
    store: &CampaignStore,
    assessments: &mut BTreeMap<String, ActionAssessment>,
    finalized: crate::elaboration::FinalizedWorldElaboration,
) -> Result<CommandResult, KernelError> {
    let campaign_id = single_campaign_id(store)?;
    let (_, campaign): (_, Campaign) = store
        .load("campaign.v1", &campaign_id)
        .map_err(persist)?
        .ok_or(KernelError::NotFound)?;
    let (expected_revision, elaboration, model_stage_receipts) = finalized
        .into_kernel_parts(&campaign)
        .map_err(|error| KernelError::Invalid(error.to_string()))?;
    execute(
        store,
        assessments,
        WorldCommand::ElaborateLocality {
            expected_revision,
            elaboration,
            evidence_receipts: Vec::new(),
            canon_candidates: Vec::new(),
            model_stage_receipts,
        },
    )
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
            .create_unadmitted_fixture_campaign(
                &campaign,
                &evidence_receipts,
                &model_stage_receipts,
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
        WorldCommand::ApplyExternalSubjectSnapshot { snapshot } => {
            if snapshot.schema != "ghostlight.external_subject_snapshot.v1"
                || snapshot.campaign_id != campaign.id
            {
                return Err(KernelError::Invalid(
                    "external snapshot targets another campaign".into(),
                ));
            }
            let receipt_key = format!("{}:{}", snapshot.authority_id, snapshot.source_revision);
            if let Some((_, receipt)) = store
                .load::<crate::consumer::ExternalSnapshotReceipt>(
                    "external_snapshot_receipt.v1",
                    &receipt_key,
                )
                .map_err(persist)?
            {
                return if receipt.payload_digest == snapshot.payload_digest
                    && receipt.owner_id == snapshot.owner_id
                {
                    Ok(CommandResult::ExternalSnapshotCommitted { campaign, receipt })
                } else {
                    Err(KernelError::Invalid(
                        "external snapshot idempotency conflict".into(),
                    ))
                };
            }
            let (authority_row, authority) = store
                .load::<crate::consumer::ExternalSubjectAuthority>(
                    "external_subject_authority.v1",
                    &snapshot.authority_id,
                )
                .map_err(persist)?
                .ok_or_else(|| {
                    KernelError::Invalid("external subject authority is unknown".into())
                })?;
            if authority.campaign_id != snapshot.campaign_id
                || authority.owner_id != snapshot.owner_id
                || authority.subject_id != snapshot.projection.subject_id()
                || authority.subject_kind != snapshot.projection.subject_kind()
            {
                return Err(KernelError::Invalid(
                    "external snapshot does not match its authority".into(),
                ));
            }
            crate::consumer::validate_authority_key(&authority, &snapshot.authority_key)
                .map_err(|error| KernelError::Invalid(error.to_string()))?;
            let actual_digest = crate::consumer::snapshot_payload_digest(&snapshot)
                .map_err(|error| KernelError::Invalid(error.to_string()))?;
            if actual_digest != snapshot.payload_digest {
                return Err(KernelError::Invalid(
                    "external snapshot payload digest is invalid".into(),
                ));
            }
            if authority
                .last_source_revision
                .is_some_and(|revision| snapshot.source_revision <= revision)
            {
                return Err(KernelError::Invalid(
                    "external snapshot source revision is stale".into(),
                ));
            }
            require_revision(&campaign, snapshot.expected_world_revision)?;
            let subject_id = snapshot.projection.subject_id().to_owned();
            let mut institution_ids = Vec::new();
            let mut gestalt_ids = Vec::new();
            match &snapshot.projection {
                crate::consumer::ExternalSubjectProjection::Institution(value) => {
                    campaign
                        .institutions
                        .insert(value.id.clone(), value.clone());
                    institution_ids.push(value.id.clone());
                    if let Some(profile) = campaign.agency_profiles.get_mut(&value.id) {
                        profile.facets.insert(
                            AgencyAxis::Authority,
                            BTreeSet::from([value.id.clone(), value.posture.clone()]),
                        );
                        profile.facets.insert(
                            AgencyAxis::EconomyRole,
                            value.resources.iter().cloned().collect(),
                        );
                    }
                }
                crate::consumer::ExternalSubjectProjection::Gestalt(value) => {
                    campaign.gestalts.insert(value.id.clone(), value.clone());
                    gestalt_ids.push(value.id.clone());
                    if let Some(profile) = campaign.agency_profiles.get_mut(&value.id) {
                        profile.location_ids = BTreeSet::from([value.home_location_id.clone()]);
                        profile.facets.insert(
                            AgencyAxis::Geography,
                            BTreeSet::from([value.home_location_id.clone()]),
                        );
                        profile.facets.insert(
                            AgencyAxis::EconomyRole,
                            value.shared_capabilities.iter().cloned().collect(),
                        );
                        profile.facets.insert(
                            AgencyAxis::Information,
                            value.shared_knowledge.iter().cloned().collect(),
                        );
                    }
                }
            }
            crate::compiler::validate_campaign_runtime(&campaign)
                .map_err(|error| KernelError::Invalid(error.to_string()))?;
            let previous_world_revision = campaign.revision;
            campaign.revision = campaign.revision.saturating_add(1);
            let now = Utc::now();
            campaign.events.push(Event {
                id: format!(
                    "external-snapshot:{}:{}",
                    snapshot.authority_id, snapshot.source_revision
                ),
                at: now,
                kind: "external_subject_snapshot".into(),
                summary: format!(
                    "{} supplied an authoritative external subject snapshot.",
                    snapshot.owner_id
                ),
                actor_ids: Vec::new(),
                institution_ids,
                gestalt_ids,
                location_ids: Vec::new(),
                public_channels: Vec::new(),
            });
            let mut next_authority = authority;
            next_authority.last_source_revision = Some(snapshot.source_revision);
            next_authority.last_payload_digest = Some(snapshot.payload_digest.clone());
            let receipt = crate::consumer::ExternalSnapshotReceipt {
                schema: "ghostlight.external_snapshot_receipt.v1".into(),
                id: receipt_key,
                campaign_id: snapshot.campaign_id,
                authority_id: snapshot.authority_id,
                subject_id,
                owner_id: snapshot.owner_id,
                source_revision: snapshot.source_revision,
                payload_digest: snapshot.payload_digest,
                previous_world_revision,
                world_revision: campaign.revision,
                committed_at: now,
            };
            store
                .append_external_snapshot(
                    &row,
                    &authority_row,
                    &campaign,
                    &next_authority,
                    &receipt,
                )
                .map_err(persist)?;
            Ok(CommandResult::ExternalSnapshotCommitted { campaign, receipt })
        }
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
                let eligible_present =
                    campaign.actors.get(response_actor_id).is_some_and(|actor| {
                        actor.location_id == speaker.location_id
                            && campaign
                                .agency_profiles
                                .get(response_actor_id)
                                .is_none_or(|profile| profile.simulation_eligible)
                    });
                let eligible_folded =
                    crate::resolution::dormant_member_id_for_subject(&campaign, response_actor_id)
                        .and_then(|member_id| {
                            crate::resolution::dormant_member_location(&campaign, member_id).ok()
                        })
                        .is_some_and(|location| location == speaker.location_id);
                let eligible_gestalt = campaign.gestalts.get(response_actor_id).is_some_and(|_| {
                    crate::resolution::validate_active_gestalt_presence_location(
                        &campaign,
                        response_actor_id,
                        &speaker.location_id,
                    )
                    .is_ok()
                });
                if response_actor_id == &actor_id
                    || !(eligible_present || eligible_folded || eligible_gestalt)
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
                None,
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
            let external_subject_ids = store
                .load_all::<crate::consumer::ExternalSubjectAuthority>(
                    "external_subject_authority.v1",
                )
                .map_err(persist)?
                .into_iter()
                .map(|authority| authority.subject_id)
                .collect::<BTreeSet<_>>();
            if resolved_plan
                .as_ref()
                .or(plan.as_ref())
                .is_some_and(|plan| {
                    strategic_plan_writes_external_subject(plan, &external_subject_ids)
                })
            {
                return Err(KernelError::Invalid(
                    "strategic resolution cannot mutate an external subject or bypass its proposal boundary".into(),
                ));
            }
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
                let outcome_stage_count = outcome_digests.len();
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
                if !wave.cover.causal_follow_through.is_empty() {
                    let expected_binding =
                        crate::follow_through::nemesis_admission_binding(&campaign, &wave.cover)
                            .map_err(|error| KernelError::Invalid(error.to_string()))?;
                    if !stage_bindings.contains(&(
                        crate::follow_through::NEMESIS_STAGE.into(),
                        expected_binding,
                    )) {
                        return Err(KernelError::Invalid(
                            "resolution wave lacks the exact Nemesis receipt admitted for its causal agenda"
                                .into(),
                        ));
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
                for binding in expected_activity_outcome_bindings(&campaign, &outcome_digests) {
                    if !stage_bindings.contains(&("strategic_outcome_resolver".into(), binding)) {
                        return Err(KernelError::Invalid(
                            "resolution wave lacks an activity-bound strategic outcome receipt"
                                .into(),
                        ));
                    }
                }
                for outcome in wave.activity_outcomes.iter().filter(|outcome| {
                    !matches!(
                        outcome.effect,
                        StrategicOutcomeEffect::NoMaterialChange { .. }
                    )
                }) {
                    let binding = crate::outcome::activity_outcome_verification_binding(
                        campaign.id,
                        campaign.revision,
                        campaign.resolution_policy.resolution_epoch,
                        std::slice::from_ref(outcome),
                    )
                    .map_err(|error| KernelError::Invalid(error.to_string()))?;
                    if !stage_bindings.contains(&("strategic_outcome_verifier".into(), binding)) {
                        return Err(KernelError::Invalid(
                            "resolution wave lacks an outcome-bound strategic verifier receipt"
                                .into(),
                        ));
                    }
                }
                if !wave.strategic_individuations.is_empty() {
                    let selected_actions = resolved_plan
                        .as_ref()
                        .map(|plan| plan.selected_actions.as_slice())
                        .unwrap_or_default();
                    let candidate_digests =
                        crate::scheduler::strategic_individuation_candidate_digests(
                            &campaign,
                            selected_actions,
                        );
                    for proposal in &wave.strategic_individuations {
                        let proposal_digest =
                            crate::scheduler::strategic_individuation_proposal_digest(proposal)
                                .map_err(|error| KernelError::Invalid(error.to_string()))?;
                        let binding = crate::scheduler::strategic_individuation_binding(
                            &campaign,
                            &candidate_digests,
                            Some(&proposal_digest),
                        );
                        if !stage_bindings
                            .contains(&("strategic_individuation_selector".into(), binding))
                        {
                            return Err(KernelError::Invalid(
                                "strategic individuation lacks a payload-bound selector receipt"
                                    .into(),
                            ));
                        }
                    }
                }
            }
            if let Some(wave) = &resolution_wave {
                for proposal in &wave.strategic_individuations {
                    apply_individuation(&mut campaign, &proposal.individuation)?;
                }
            }
            let individuation_public_channels = resolved_plan
                .as_ref()
                .or(plan.as_ref())
                .map(|plan| {
                    plan.selected_actions
                        .iter()
                        .map(|action| {
                            crate::resolution::cell_action_digest(action)
                                .map(|digest| (digest, action.public_channels.clone()))
                                .map_err(|error| KernelError::Invalid(error.to_string()))
                        })
                        .collect::<Result<BTreeMap<_, _>, _>>()
                })
                .transpose()?
                .unwrap_or_default();
            let applied_tick = match resolved_plan.or(plan) {
                Some(plan) => apply_strategic_tick_plan(&mut campaign, plan)?,
                None => {
                    let plan = deterministic_strategic_tick_plan();
                    apply_strategic_tick_plan(&mut campaign, plan)?
                }
            };
            let AppliedStrategicTickPlan {
                events: mut tick_events,
                mutation,
            } = applied_tick;
            if let Some(wave) = &resolution_wave {
                for proposal in &wave.strategic_individuations {
                    let member = &proposal.individuation.member;
                    let gestalt_name = campaign
                        .gestalts
                        .get(&proposal.individuation.gestalt_id)
                        .map(|gestalt| gestalt.name.as_str())
                        .unwrap_or("their community");
                    let undertaking = member
                        .goals
                        .first()
                        .map(String::as_str)
                        .or_else(|| member.obligations.iter().next().map(String::as_str));
                    let summary = undertaking.map_or_else(
                        || format!("{} steps forward within {gestalt_name}.", member.name),
                        |undertaking| {
                            format!(
                                "{} steps forward within {gestalt_name} to {undertaking}.",
                                member.name
                            )
                        },
                    );
                    tick_events.push(crate::domain::Event {
                        id: format!(
                            "strategic:{}:individuation:{}",
                            campaign.strategic_tick_count.saturating_add(1),
                            crate::domain::canonical_gestalt_member_local_id(&member.id)
                        ),
                        at: campaign.world_time,
                        kind: "gestalt_individuation".into(),
                        summary: summary.chars().take(240).collect(),
                        actor_ids: vec![crate::domain::gestalt_member_subject_id(&member.id)],
                        institution_ids: vec![],
                        gestalt_ids: vec![proposal.individuation.gestalt_id.clone()],
                        location_ids: vec![proposal.individuation.location_id.clone()],
                        public_channels: individuation_public_channels
                            .get(&proposal.action_digest)
                            .cloned()
                            .unwrap_or_default(),
                    });
                }
            }
            if let Some(wave) = &resolution_wave {
                crate::resolution::advance_detail_debt(&mut campaign, &wave.cover);
                campaign.resolution_cover = Some(wave.cover.clone());
            }
            campaign.strategic_tick_count = campaign.strategic_tick_count.saturating_add(1);
            let event_ids = mutation
                .as_ref()
                .into_iter()
                .flat_map(|(_, receipt)| receipt.derived_event_ids.iter().cloned())
                .chain(tick_events.iter().map(|event| event.id.clone()))
                .collect();
            for event in tick_events {
                crate::domain::append_event_with_publications(&mut campaign, event);
            }
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
            validate_region_admission_evidence(&expansion, &evidence_receipts, &canon_candidates)?;
            crate::compiler::validate_new_destination_expansion(&campaign, &expansion)
                .map_err(|error| KernelError::Invalid(error.to_string()))?;
            crate::compiler::validate_civic_admission_receipts(
                &campaign,
                &expansion,
                &model_stage_receipts,
            )
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
                None,
                Some((transition, mutation_receipt)),
            )
        }
        WorldCommand::ElaborateLocality {
            expected_revision,
            elaboration,
            evidence_receipts,
            canon_candidates,
            model_stage_receipts,
        } => {
            require_revision(&campaign, expected_revision)?;
            validate_region_admission_evidence(
                &elaboration.expansion,
                &evidence_receipts,
                &canon_candidates,
            )?;
            crate::compiler::validate_locality_elaboration(&campaign, &elaboration)
                .map_err(|error| KernelError::Invalid(error.to_string()))?;
            crate::compiler::validate_civic_admission_receipts(
                &campaign,
                &elaboration.expansion,
                &model_stage_receipts,
            )
            .map_err(|error| KernelError::Invalid(error.to_string()))?;
            let transition = crate::legacy_transition::lower_region_expansion(
                &campaign,
                &elaboration.expansion,
                Utc::now() + Duration::minutes(5),
            )
            .map_err(|error| KernelError::Invalid(error.to_string()))?;
            let mutation_receipt = crate::legacy_transition::apply_lowered_region_expansion(
                &mut campaign,
                &elaboration.expansion,
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
                "elaborate_locality",
                evidence_receipts,
                canon_candidates,
                model_stage_receipts,
                None,
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
            gestalt_reactions,
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
                    validate_reaction_identity_adoption(&campaign, &reaction.actor_id, identity)?;
                }
                if let Some(speech) = &reaction.speech {
                    validate_bounded_text("reaction speech", speech, 1_000)?;
                }
                for proposal in &reaction.action_proposals {
                    validate_world_proposal(actor, proposal)?;
                }
            }
            let mut seen_gestalts = BTreeSet::new();
            for reaction in &gestalt_reactions {
                if !seen_gestalts.insert(reaction.gestalt_id.clone()) {
                    return Err(KernelError::Invalid(
                        "Gestalt reacted twice in one wave".into(),
                    ));
                }
                if seen.contains(&reaction.gestalt_id) {
                    return Err(KernelError::Invalid(
                        "reaction subject appeared as both actor and Gestalt".into(),
                    ));
                }
                crate::resolution::validate_active_gestalt_presence_location(
                    &campaign,
                    &reaction.gestalt_id,
                    &player_location,
                )
                .map_err(|_| KernelError::Invalid("reaction Gestalt is not present".into()))?;
                if let Some(speech) = &reaction.speech {
                    validate_bounded_text("Gestalt reaction speech", speech, 1_000)?;
                }
            }
            for response_actor_id in &response_expected_actor_ids {
                let actor_reaction = reactions
                    .iter()
                    .find(|reaction| &reaction.actor_id == response_actor_id);
                let gestalt_reaction = gestalt_reactions
                    .iter()
                    .find(|reaction| &reaction.gestalt_id == response_actor_id);
                let observable = actor_reaction.is_some_and(|reaction| {
                    reaction
                        .speech
                        .as_deref()
                        .is_some_and(|value| !value.trim().is_empty())
                        || reaction.deliberate_silence
                }) || gestalt_reaction.is_some_and(|reaction| {
                    reaction
                        .speech
                        .as_deref()
                        .is_some_and(|value| !value.trim().is_empty())
                        || reaction.deliberate_silence
                });
                if !observable {
                    return Err(KernelError::Invalid(
                        "directly addressed Persona is absent or produced no observable response"
                            .into(),
                    ));
                }
            }
            let source_receipt_id = format!(
                "reaction-input:{}",
                crate::legacy_transition::digest_serializable(&(&reactions, &gestalt_reactions))
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
            for reaction in gestalt_reactions {
                if let Some(speech) = reaction.speech {
                    campaign.transcript.push(NarrativeTurn {
                        revision: campaign.revision + 1,
                        at: Utc::now(),
                        speaker: reaction.gestalt_id.clone(),
                        text: speech,
                        persona_response_actor_ids: BTreeSet::new(),
                    });
                }
                if reaction.deliberate_silence {
                    campaign.transcript.push(NarrativeTurn {
                        revision: campaign.revision + 1,
                        at: Utc::now(),
                        speaker: reaction.gestalt_id,
                        text: "deliberately does not answer.".into(),
                        persona_response_actor_ids: BTreeSet::new(),
                    });
                }
            }
            refresh_materialized_member_relevance(&mut campaign, seen.iter().map(String::as_str));
            campaign.events.push(Event {
                id: format!("reaction-wave:{}", campaign.revision + 1),
                at: campaign.world_time,
                kind: "reaction_wave".into(),
                summary: event_summary,
                actor_ids: seen.into_iter().collect(),
                institution_ids: vec![],
                gestalt_ids: seen_gestalts.into_iter().collect(),
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
        WorldCommand::BindClockConsequences {
            expected_revision,
            admission,
            model_stage_receipts,
        } => {
            require_revision(&campaign, expected_revision)?;
            crate::clock::validate_binding_receipts(&campaign, &admission, &model_stage_receipts)
                .map_err(|error| KernelError::Invalid(error.to_string()))?;
            let news_count_before = campaign.news.len();
            let next_wave_index = campaign.strategic_tick_count.saturating_add(1) as usize;
            let emitted_event_ids =
                crate::clock::apply_clock_consequence_bindings(&mut campaign, &admission.bindings)
                    .map_err(|error| KernelError::Invalid(error.to_string()))?;
            commit_with_records(
                store,
                row,
                campaign,
                "bind_clock_consequences",
                Vec::new(),
                Vec::new(),
                model_stage_receipts,
                Some(ClockBindingCommitData {
                    admission,
                    emitted_event_ids,
                    news_count_before,
                    next_wave_index,
                }),
                None,
            )
        }
        WorldCommand::CreateCampaign { .. } => unreachable!(),
    }
}

fn external_outcome_writes_subject(
    effect: &StrategicOutcomeEffect,
    external_subject_ids: &BTreeSet<String>,
) -> bool {
    match effect {
        StrategicOutcomeEffect::ResourceCreated {
            owner_subject_id, ..
        }
        | StrategicOutcomeEffect::ResourceConsumed {
            owner_subject_id, ..
        }
        | StrategicOutcomeEffect::KnowledgeLearned {
            owner_subject_id, ..
        } => external_subject_ids.contains(owner_subject_id),
        StrategicOutcomeEffect::ResourceTransferred {
            from_subject_id,
            to_subject_id,
            ..
        } => {
            external_subject_ids.contains(from_subject_id)
                || external_subject_ids.contains(to_subject_id)
        }
        StrategicOutcomeEffect::KnowledgeCommunicated { to_subject_ids, .. } => to_subject_ids
            .iter()
            .any(|subject_id| external_subject_ids.contains(subject_id)),
        StrategicOutcomeEffect::NoMaterialChange { .. }
        | StrategicOutcomeEffect::GestaltPressure { .. }
        | StrategicOutcomeEffect::AgencyRelationShift { .. }
        | StrategicOutcomeEffect::MemberMemory { .. }
        | StrategicOutcomeEffect::MemberObligation { .. }
        | StrategicOutcomeEffect::MemberRelationship { .. } => false,
    }
}

fn strategic_plan_writes_external_subject(
    plan: &StrategicTickPlan,
    external_subject_ids: &BTreeSet<String>,
) -> bool {
    plan.selected_actions.iter().any(|action| {
        crate::consumer::proposal_targets(action)
            .iter()
            .any(|target| external_subject_ids.contains(*target))
    }) || plan
        .institution_actions
        .iter()
        .any(|action| external_subject_ids.contains(&action.institution_id))
        || plan
            .activity_outcomes
            .iter()
            .any(|outcome| external_outcome_writes_subject(&outcome.effect, external_subject_ids))
}

fn expected_activity_outcome_bindings(
    campaign: &Campaign,
    action_digests: &[String],
) -> Vec<String> {
    action_digests
        .iter()
        .map(|digest| {
            crate::outcome::activity_outcome_binding(
                campaign.id,
                campaign.revision,
                campaign.resolution_policy.resolution_epoch,
                std::slice::from_ref(digest),
            )
        })
        .collect()
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
    let mut member = individuation.member.clone();
    member.id = crate::domain::canonical_gestalt_member_local_id(&member.id);
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
    let actor_id = crate::domain::gestalt_member_subject_id(&member.id);
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

fn validate_reaction_identity_adoption(
    campaign: &Campaign,
    actor_id: &str,
    identity: &str,
) -> Result<(), KernelError> {
    let actor = campaign
        .actors
        .get(actor_id)
        .ok_or_else(|| KernelError::Invalid("reaction actor is unknown".into()))?;
    let identity = identity.trim().to_lowercase();
    if actor.name.trim().to_lowercase() == identity {
        return Ok(());
    }
    let local_collision = campaign.actors.values().any(|peer| {
        peer.id != actor_id
            && peer.location_id == actor.location_id
            && peer.name.trim().to_lowercase() == identity
    });
    let population_collision = campaign
        .gestalt_members
        .values()
        .find(|member| member.materialized_actor_id.as_deref() == Some(actor_id))
        .is_some_and(|member| {
            campaign.gestalt_members.values().any(|peer| {
                peer.id != member.id
                    && peer.gestalt_id == member.gestalt_id
                    && peer.name.trim().to_lowercase() == identity
            })
        });
    if local_collision || population_collision {
        return Err(KernelError::Invalid(
            "reaction identity adoption conflicts with an established local or population identity"
                .into(),
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

fn validate_region_admission_evidence(
    expansion: &RegionExpansion,
    evidence_receipts: &[VaultEvidenceReceipt],
    canon_candidates: &[CanonCandidate],
) -> Result<(), KernelError> {
    let supplied_evidence = evidence_receipts
        .iter()
        .map(|receipt| receipt.id.as_str())
        .collect::<BTreeSet<_>>();
    let missing_profile_evidence = expansion
        .population_profiles
        .iter()
        .chain(expansion.institution_profiles.iter())
        .any(|profile| {
            profile
                .evidence_receipt_ids
                .iter()
                .any(|id| !supplied_evidence.contains(id.as_str()))
        });
    let missing_relation_evidence = expansion
        .migration_relations
        .iter()
        .chain(expansion.local_relations.iter())
        .any(|relation| {
            relation
                .evidence_receipt_ids
                .iter()
                .any(|id| !supplied_evidence.contains(id.as_str()))
        });
    if expansion.facts.iter().any(|fact| {
        fact.evidence_receipt_ids
            .iter()
            .any(|id| !supplied_evidence.contains(id.as_str()))
    }) || missing_profile_evidence
        || missing_relation_evidence
        || canon_candidates.iter().any(|candidate| {
            candidate
                .evidence_receipt_ids
                .iter()
                .any(|id| !supplied_evidence.contains(id.as_str()))
        })
    {
        return Err(KernelError::Invalid(
            "region admission evidence receipts were not supplied".into(),
        ));
    }
    Ok(())
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

fn merge_strategic_outcome_event_context(
    contexts: &mut BTreeMap<String, (String, Vec<String>, Vec<String>)>,
    action_digest: &str,
    subject_id: &str,
    location_ids: &[String],
    public_channels: &[String],
) -> Result<(), KernelError> {
    let entry = contexts
        .entry(action_digest.to_owned())
        .or_insert_with(|| (subject_id.to_owned(), Vec::new(), Vec::new()));
    if entry.0 != subject_id {
        return Err(KernelError::Invalid(
            "one strategic action digest cannot belong to multiple subjects".into(),
        ));
    }
    entry.1.extend(location_ids.iter().cloned());
    entry.1.sort();
    entry.1.dedup();
    entry.2.extend(public_channels.iter().cloned());
    entry.2.sort();
    entry.2.dedup();
    Ok(())
}

fn strategic_activity_phase(
    campaign: &Campaign,
    prospective_actor_locations: &BTreeMap<String, String>,
    prospective_gestalt_locations: &BTreeMap<String, String>,
    prospective_member_locations: &BTreeMap<String, String>,
    subject_id: &str,
    location_ids: &[String],
) -> Result<u8, KernelError> {
    let (origin, destination) = if let Some(actor) = campaign.actors.get(subject_id) {
        (
            actor.location_id.as_str(),
            prospective_actor_locations
                .get(subject_id)
                .map(String::as_str),
        )
    } else if let Some(gestalt) = campaign.gestalts.get(subject_id) {
        (
            gestalt.home_location_id.as_str(),
            prospective_gestalt_locations
                .get(subject_id)
                .map(String::as_str),
        )
    } else if let Some(member_id) = subject_id.strip_prefix("member:") {
        let origin = crate::resolution::dormant_member_location(campaign, member_id)
            .map_err(|error| KernelError::Invalid(error.to_string()))?;
        return Ok(relative_activity_phase(
            &origin,
            prospective_member_locations
                .get(member_id)
                .map(String::as_str),
            location_ids,
        ));
    } else {
        return Ok(1);
    };
    Ok(relative_activity_phase(origin, destination, location_ids))
}

fn relative_activity_phase(origin: &str, destination: Option<&str>, location_ids: &[String]) -> u8 {
    let Some(destination) = destination.filter(|destination| *destination != origin) else {
        return 1;
    };
    if location_ids.is_empty() || location_ids.iter().all(|location| location == origin) {
        0
    } else if location_ids.iter().all(|location| location == destination) {
        2
    } else {
        1
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
    let activity_means = plan
        .selected_actions
        .iter()
        .map(|action| {
            crate::resolution::cell_action_digest(action)
                .map(|digest| (digest, action.intended_effect.clone()))
                .map_err(|error| KernelError::Invalid(error.to_string()))
        })
        .collect::<Result<BTreeMap<_, _>, _>>()?;
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
    let mut outcome_event_context: BTreeMap<String, (String, Vec<String>, Vec<String>)> =
        BTreeMap::new();
    for activity in &plan.gestalt_activities {
        merge_strategic_outcome_event_context(
            &mut outcome_event_context,
            &activity.action_digest,
            &activity.gestalt_id,
            &activity.location_ids,
            &activity.public_channels,
        )?;
    }
    for activity in &plan.actor_activities {
        merge_strategic_outcome_event_context(
            &mut outcome_event_context,
            &activity.action_digest,
            &activity.actor_id,
            &activity.location_ids,
            &activity.public_channels,
        )?;
    }
    for activity in &plan.member_activities {
        merge_strategic_outcome_event_context(
            &mut outcome_event_context,
            &activity.action_digest,
            &crate::domain::gestalt_member_subject_id(&activity.member_id),
            &activity.location_ids,
            &activity.public_channels,
        )?;
    }
    // Every action in a strategic wave was chosen against the same committed
    // snapshot. Keep that snapshot immutable while applying to a private copy
    // so action ordering cannot rewrite another action's permissions and an
    // invalid late action cannot leave this primitive partially mutated.
    let mut next = campaign.clone();
    let revision = campaign.revision + 1;
    let at = campaign.world_time + Duration::hours(i64::from(campaign.tick_hours));
    let mut events = Vec::new();
    let mut event_phases = BTreeMap::new();
    let mut seen_institutions = BTreeSet::new();
    for action in plan.institution_actions {
        if !seen_institutions.insert(action.institution_id.clone()) {
            return Err(KernelError::Invalid(
                "institution acts twice in one strategic tick".into(),
            ));
        }
        if action.posture.trim().is_empty() {
            return Err(KernelError::Invalid(
                "strategic institution posture is empty".into(),
            ));
        }
        if action.posture.chars().count() > MAX_POSTURE_CHARS {
            return Err(KernelError::Invalid(format!(
                "strategic institution posture exceeds {MAX_POSTURE_CHARS} characters"
            )));
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
        let summary = format!(
            "{} announces a new course: {}",
            institution.name, action.posture
        );
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
                "A declaration from {} marks this matter settled: {}",
                gestalt.name,
                action.pressure_resolutions.join("; ")
            ));
        }
        if !action.pressure_additions.is_empty() {
            summary_parts.push(format!(
                "New public demand from {}: {}",
                gestalt.name,
                action.pressure_additions.join("; ")
            ));
        }
        events.push(crate::domain::Event {
            id: format!("strategic:{revision}:gestalt:{}", gestalt.id),
            at,
            kind: "gestalt_action".into(),
            summary: summary_parts.join("; "),
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
        let origin_name = campaign
            .locations
            .get(&origin)
            .ok_or_else(|| KernelError::Invalid("gestalt migration origin is missing".into()))?
            .name
            .clone();
        let gestalt_name = campaign.gestalts[&action.gestalt_id].name.clone();
        let destination_location_name = campaign
            .locations
            .get(&action.destination_location_id)
            .ok_or_else(|| KernelError::Invalid("gestalt migration destination is missing".into()))?
            .name
            .clone();
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
                "{gestalt_name} relocates from {origin_name} to {destination_location_name} near {destination_name}."
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
        let gestalt = campaign
            .gestalts
            .get(&action.gestalt_id)
            .ok_or_else(|| KernelError::Invalid("strategic plan invented a gestalt".into()))?;
        let locations = if action.location_ids.is_empty() {
            vec![gestalt.home_location_id.clone()]
        } else {
            action.location_ids.clone()
        };
        if !seen_gestalt_activities.insert((
            action.gestalt_id.clone(),
            strategic_activity_scope_key(&action.activity, &action.target_subject_ids, &locations),
        )) || (!canonical_composition && !legacy_seen_gestalts.insert(action.gestalt_id.clone()))
        {
            return Err(KernelError::Invalid(
                "gestalt acts twice in one strategic tick".into(),
            ));
        }
        validate_public_channels(&action.public_channels)?;
        let profile = campaign
            .agency_profiles
            .get(&action.gestalt_id)
            .ok_or_else(|| KernelError::Invalid("strategic gestalt lacks agency scope".into()))?;
        let allowed_targets =
            crate::resolution::strategic_activity_targets(campaign, &action.gestalt_id);
        let unique_targets = action.target_subject_ids.iter().collect::<BTreeSet<_>>();
        let unique_locations = action.location_ids.iter().collect::<BTreeSet<_>>();
        let needs_target = action.activity.requires_explicit_target_for_gestalt();
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
        let phase = strategic_activity_phase(
            campaign,
            &prospective_actor_locations,
            &prospective_gestalt_locations,
            &prospective_member_locations,
            &action.gestalt_id,
            &locations,
        )?;
        let scope_digest = strategic_activity_scope_digest(
            &action.activity,
            &action.target_subject_ids,
            &locations,
        );
        let event = crate::domain::Event {
            id: format!(
                "strategic:{revision}:gestalt-activity:{}:{}:{scope_digest}",
                action.gestalt_id,
                strategic_activity_id(&action.activity),
            ),
            at,
            kind: "gestalt_activity".into(),
            summary: strategic_activity_summary(
                &gestalt.name,
                &action.activity,
                &target_names,
                activity_means
                    .get(&action.action_digest)
                    .map(String::as_str),
                &action.public_channels,
            ),
            actor_ids,
            institution_ids,
            gestalt_ids,
            location_ids: locations,
            public_channels: action.public_channels,
        };
        event_phases.insert(event.id.clone(), phase);
        events.push(event);
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
        let origin_name = campaign
            .locations
            .get(&origin)
            .ok_or_else(|| KernelError::Invalid("actor movement origin is missing".into()))?
            .name
            .clone();
        let destination_name = campaign
            .locations
            .get(&action.destination_id)
            .ok_or_else(|| KernelError::Invalid("actor movement destination is missing".into()))?
            .name
            .clone();
        events.push(crate::domain::Event {
            id: format!("strategic:{revision}:actor:{}", action.actor_id),
            at,
            kind: "actor_movement".into(),
            summary: format!("{actor_name} moves from {origin_name} to {destination_name}."),
            actor_ids: vec![action.actor_id],
            institution_ids: vec![],
            gestalt_ids: vec![],
            location_ids: vec![origin, action.destination_id],
            public_channels: action.public_channels,
        });
    }

    let mut seen_actor_activities = BTreeSet::new();
    for action in plan.actor_activities {
        if !seen_actor_activities.insert((
            action.actor_id.clone(),
            strategic_activity_scope_key(
                &action.activity,
                &action.target_subject_ids,
                &action.location_ids,
            ),
        )) || (!canonical_composition && !legacy_seen_actors.insert(action.actor_id.clone()))
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
        let phase = strategic_activity_phase(
            campaign,
            &prospective_actor_locations,
            &prospective_gestalt_locations,
            &prospective_member_locations,
            &action.actor_id,
            &action.location_ids,
        )?;
        let scope_digest = strategic_activity_scope_digest(
            &action.activity,
            &action.target_subject_ids,
            &action.location_ids,
        );
        let event = Event {
            id: format!(
                "strategic:{revision}:actor-activity:{}:{}:{scope_digest}",
                action.actor_id,
                strategic_activity_id(&action.activity),
            ),
            at,
            kind: "actor_activity".into(),
            summary: strategic_activity_summary(
                &actor.name,
                &action.activity,
                &target_names,
                activity_means
                    .get(&action.action_digest)
                    .map(String::as_str),
                &action.public_channels,
            ),
            actor_ids,
            institution_ids,
            gestalt_ids,
            location_ids: action.location_ids,
            public_channels: action.public_channels,
        };
        event_phases.insert(event.id.clone(), phase);
        events.push(event);
    }

    let mut legacy_seen_members = BTreeSet::new();
    let mut seen_member_activities = BTreeSet::new();
    for action in plan.member_activities {
        if !seen_member_activities.insert((
            action.member_id.clone(),
            strategic_activity_scope_key(
                &action.activity,
                &action.target_subject_ids,
                &action.location_ids,
            ),
        )) || (!canonical_composition && !legacy_seen_members.insert(action.member_id.clone()))
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
        let mut actor_ids = vec![crate::domain::gestalt_member_subject_id(&action.member_id)];
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
        let phase = strategic_activity_phase(
            campaign,
            &prospective_actor_locations,
            &prospective_gestalt_locations,
            &prospective_member_locations,
            &crate::domain::gestalt_member_subject_id(&action.member_id),
            &action.location_ids,
        )?;
        let scope_digest = strategic_activity_scope_digest(
            &action.activity,
            &action.target_subject_ids,
            &action.location_ids,
        );
        let event = Event {
            id: format!(
                "strategic:{revision}:member-activity:{}:{}:{scope_digest}",
                action.member_id,
                strategic_activity_id(&action.activity),
            ),
            at,
            kind: "gestalt_member_activity".into(),
            summary: strategic_activity_summary(
                &member.name,
                &action.activity,
                &target_names,
                activity_means
                    .get(&action.action_digest)
                    .map(String::as_str),
                &action.public_channels,
            ),
            actor_ids,
            institution_ids,
            gestalt_ids,
            location_ids: action.location_ids,
            public_channels: action.public_channels,
        };
        event_phases.insert(event.id.clone(), phase);
        events.push(event);
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
        let origin_name = campaign
            .locations
            .get(&origin)
            .ok_or_else(|| KernelError::Invalid("member migration origin is missing".into()))?
            .name
            .clone();
        let destination_location_name = campaign
            .locations
            .get(&action.destination_location_id)
            .ok_or_else(|| KernelError::Invalid("member migration destination is missing".into()))?
            .name
            .clone();
        let destination_gestalt_name = campaign.gestalts[&action.destination_gestalt_id]
            .name
            .clone();
        events.push(crate::domain::Event {
            id: format!("strategic:{revision}:member:{}", action.member_id),
            at,
            kind: "gestalt_member_migration".into(),
            summary: format!(
                "{member_name} moves from {origin_name} to {destination_location_name} and joins {destination_gestalt_name}."
            ),
            actor_ids: vec![crate::domain::gestalt_member_subject_id(&action.member_id)],
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
        if matches!(
            &outcome.effect,
            crate::domain::StrategicOutcomeEffect::NoMaterialChange { .. }
        ) {
            continue;
        }
        let mut subject_ids = BTreeSet::from([source_subject_id.clone()]);
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
        let phase = strategic_activity_phase(
            campaign,
            &prospective_actor_locations,
            &prospective_gestalt_locations,
            &prospective_member_locations,
            &source_subject_id,
            &locations,
        )?;
        let event = Event {
            id: format!("strategic:{revision}:activity-outcome:{digest_suffix}"),
            at,
            kind: "strategic_activity_outcome".into(),
            summary: outcome.summary,
            actor_ids,
            institution_ids,
            gestalt_ids,
            location_ids: locations,
            public_channels,
        };
        event_phases.insert(event.id.clone(), phase);
        events.push(event);
    }
    events.sort_by_key(|event| event_phases.get(&event.id).copied().unwrap_or(1));
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
        StrategicOutcomeEffect::KnowledgeCommunicated {
            from_subject_id,
            to_subject_ids,
            ..
        } => {
            subjects.insert(from_subject_id.clone());
            subjects.extend(to_subject_ids.iter().cloned());
        }
        StrategicOutcomeEffect::GestaltPressure { gestalt_id, .. } => {
            subjects.insert(gestalt_id.clone());
        }
        StrategicOutcomeEffect::AgencyRelationShift { .. } => {}
        StrategicOutcomeEffect::MemberMemory { member_id, .. }
        | StrategicOutcomeEffect::MemberObligation { member_id, .. }
        | StrategicOutcomeEffect::MemberRelationship { member_id, .. } => {
            subjects.insert(crate::domain::gestalt_member_subject_id(member_id));
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
    admitted_means: Option<&str>,
    public_channels: &[String],
) -> String {
    if let Some(means) = admitted_means
        .map(str::trim)
        .filter(|means| !means.is_empty())
    {
        let means = means.trim_end_matches(['.', '!', '?']);
        if !public_channels.is_empty() {
            return format!("Public statement from {source_name}: {means}.");
        }
        let mut characters = means.chars();
        let lowered = characters
            .next()
            .map(|first| first.to_lowercase().chain(characters).collect::<String>())
            .unwrap_or_default();
        if let Some(rest) = lowered.strip_prefix("attempt to ") {
            return format!("{source_name} attempts to {rest}.");
        }
        if let Some(rest) = lowered.strip_prefix("attempt ") {
            return format!("{source_name} attempts {rest}.");
        }
        return format!("{source_name} attempts to {lowered}.");
    }
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
        (StrategicActivityKind::Coordinate, true) => {
            format!("{source_name} attempts internal coordination.")
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

fn strategic_activity_id(activity: &StrategicActivityKind) -> &'static str {
    match activity {
        StrategicActivityKind::Prepare => "prepare",
        StrategicActivityKind::Coordinate => "coordinate",
        StrategicActivityKind::Investigate => "investigate",
        StrategicActivityKind::Recruit => "recruit",
        StrategicActivityKind::Obstruct => "obstruct",
        StrategicActivityKind::Trade => "trade",
        StrategicActivityKind::Communicate => "communicate",
    }
}

fn strategic_activity_scope_key(
    activity: &StrategicActivityKind,
    target_subject_ids: &[String],
    location_ids: &[String],
) -> (&'static str, Vec<String>, Vec<String>) {
    let mut targets = target_subject_ids.to_vec();
    let mut locations = location_ids.to_vec();
    targets.sort();
    locations.sort();
    (strategic_activity_id(activity), targets, locations)
}

fn strategic_activity_scope_digest(
    activity: &StrategicActivityKind,
    target_subject_ids: &[String],
    location_ids: &[String],
) -> String {
    let scope = strategic_activity_scope_key(activity, target_subject_ids, location_ids);
    let bytes = rmp_serde::to_vec_named(&scope)
        .expect("serializing a strategic activity scope made only of strings cannot fail");
    format!("{:x}", Sha256::digest(bytes))
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
    let player_location = campaign.actors[&campaign.player_actor_id]
        .location_id
        .clone();
    let visible_arrivals = changes
        .iter()
        .filter(|change| change.operation != "dematerialized")
        .filter_map(|change| {
            let actor = campaign.actors.get(&change.actor_id)?;
            (actor.location_id == player_location).then(|| {
                format!(
                    "{} is here with {} at {}.",
                    actor.name,
                    campaign.gestalts[&change.gestalt_id].name,
                    campaign.locations[&actor.location_id].name
                )
            })
        })
        .collect::<Vec<_>>();
    campaign
        .transcript
        .extend(visible_arrivals.into_iter().map(|text| NarrativeTurn {
            revision: campaign.revision,
            at: committed_at,
            speaker: "world".into(),
            text,
            persona_response_actor_ids: BTreeSet::new(),
        }));
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
    if let Some(wave) = &resolution_wave {
        for assignment in &wave.cover.causal_follow_through {
            if campaign.nemesis_attention_history.iter().any(|record| {
                record.anchor_reference == assignment.anchor_reference
                    && record.responder_subject_id == assignment.responder_subject_id
            }) {
                return Err(KernelError::Invalid(
                    "Nemesis attempted to serve an already committed causal attention window"
                        .into(),
                ));
            }
            campaign
                .nemesis_attention_history
                .push(crate::domain::NemesisAttentionRecord {
                    anchor_reference: assignment.anchor_reference.clone(),
                    responder_subject_id: assignment.responder_subject_id.clone(),
                    served_world_revision: previous_revision.saturating_add(1),
                });
        }
    }
    campaign.revision += 1;
    let committed_at = Utc::now();
    let external_authorities = store
        .load_all::<crate::consumer::ExternalSubjectAuthority>("external_subject_authority.v1")
        .map_err(persist)?;
    let external_proposals = resolution_wave
        .as_ref()
        .map(|wave| {
            external_proposals_for_wave(
                campaign.id,
                campaign.revision,
                committed_at,
                wave,
                &external_authorities,
            )
        })
        .transpose()
        .map_err(|error| KernelError::Invalid(error.to_string()))?
        .unwrap_or_default();
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
            &external_proposals,
            mutation
                .as_ref()
                .map(|(transition, receipt)| (&transition.authority, &transition.batch, receipt)),
        )
        .map_err(persist)?;
    Ok(CommandResult::Committed { campaign, receipt })
}

fn external_proposals_for_wave(
    campaign_id: uuid::Uuid,
    world_revision: u64,
    created_at: chrono::DateTime<Utc>,
    wave: &ResolutionWaveCommit,
    authorities: &[crate::consumer::ExternalSubjectAuthority],
) -> anyhow::Result<Vec<crate::consumer::ExternalWorldProposal>> {
    let mut proposals = Vec::new();
    for appraisal in &wave.appraisals {
        for action in &appraisal.actions {
            let action_digest = crate::resolution::cell_action_digest(action)?;
            for authority in authorities {
                if crate::consumer::proposal_targets(action)
                    .contains(&authority.subject_id.as_str())
                {
                    let id = crate::legacy_transition::digest_serializable(&(
                        campaign_id,
                        world_revision,
                        &authority.id,
                        &action_digest,
                    ))?;
                    proposals.push(crate::consumer::ExternalWorldProposal {
                        schema: "ghostlight.external_world_proposal.v1".into(),
                        id: format!("external-proposal:{id}"),
                        campaign_id,
                        world_revision,
                        authority_id: authority.id.clone(),
                        external_subject_id: authority.subject_id.clone(),
                        source_subject_id: action.subject_id.clone(),
                        intent: action.intent.clone(),
                        intended_effect: action.intended_effect.clone(),
                        action_digest: action_digest.clone(),
                        public_channels: action.public_channels.clone(),
                        state_references: action.state_references.clone(),
                        status: crate::consumer::ExternalProposalStatus::Pending,
                        created_at,
                    });
                }
            }
        }
    }
    Ok(proposals)
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

fn join_public_names(names: &[String]) -> String {
    match names {
        [] => "The party".into(),
        [name] => name.clone(),
        [first, second] => format!("{first} and {second}"),
        _ => format!(
            "{}, and {}",
            names[..names.len() - 1].join(", "),
            names.last().expect("non-empty names")
        ),
    }
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
    let route = campaign
        .locations
        .get(&proposal.origin_location_id)
        .and_then(|location| {
            location
                .routes
                .values()
                .find(|route| route.destination_id == proposal.destination_location_id)
                .cloned()
        })
        .ok_or_else(|| KernelError::Invalid("group-travel route no longer exists".into()))?;
    if route.travel_minutes != proposal.travel_minutes {
        return Err(KernelError::Invalid(
            "group-travel route changed after proposal".into(),
        ));
    }
    let origin_name = campaign.locations[&proposal.origin_location_id]
        .name
        .clone();
    let destination_name = campaign.locations[&proposal.destination_location_id]
        .name
        .clone();
    let traveler_names = active_actor_ids
        .iter()
        .filter_map(|actor_id| {
            campaign
                .actors
                .get(actor_id)
                .map(|actor| actor.name.clone())
        })
        .collect::<Vec<_>>();
    let transition = crate::legacy_transition::lower_group_travel(
        &campaign,
        &active_actor_ids,
        &proposal.origin_location_id,
        &proposal.destination_location_id,
        route.travel_minutes,
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
    let travel_summary = format!(
        "{} {} from {} to {} — {}. {} minutes pass.",
        join_public_names(&traveler_names),
        if traveler_names.len() == 1 {
            "travels"
        } else {
            "travel"
        },
        origin_name,
        destination_name,
        route.distance,
        route.travel_minutes
    );
    campaign.transcript.push(NarrativeTurn {
        revision: campaign.revision,
        at: campaign.world_time,
        speaker: "world".into(),
        text: travel_summary.clone(),
        persona_response_actor_ids: BTreeSet::new(),
    });
    campaign.events.push(Event {
        id: format!("group-travel:{}", campaign.revision),
        at: campaign.world_time,
        kind: "group_travel".into(),
        summary: travel_summary,
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

struct ClockBindingCommitData {
    admission: crate::clock::ClockConsequenceBindingAdmission,
    emitted_event_ids: Vec<String>,
    news_count_before: usize,
    next_wave_index: usize,
}

fn commit_with_records(
    store: &CampaignStore,
    row: cultcache_legacy::CultCacheEnvelope,
    mut campaign: Campaign,
    kind: &str,
    evidence: Vec<VaultEvidenceReceipt>,
    candidates: Vec<CanonCandidate>,
    model_receipts: Vec<crate::model::ModelStageReceipt>,
    clock_binding: Option<ClockBindingCommitData>,
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
    let committed_at = mutation
        .as_ref()
        .map(|(_, receipt)| receipt.committed_at)
        .unwrap_or_else(Utc::now);
    let receipt = WorldCommitReceipt {
        schema: "ghostlight.world_commit_receipt.v1".into(),
        campaign_id: campaign.id,
        previous_revision,
        revision: campaign.revision,
        command_kind: kind.into(),
        committed_at,
        roll: None,
    };
    let clock_binding_receipt = if let Some(binding) = clock_binding {
        let emitted_events = binding.emitted_event_ids.iter().collect::<BTreeSet<_>>();
        let emitted_news_ids = campaign
            .news
            .iter()
            .skip(binding.news_count_before)
            .filter(|news| news.event_ids.iter().any(|id| emitted_events.contains(id)))
            .map(|news| news.id.clone())
            .collect::<Vec<_>>();
        let expected_news_ids = campaign
            .events
            .iter()
            .filter(|event| emitted_events.contains(&event.id))
            .flat_map(|event| {
                event
                    .public_channels
                    .iter()
                    .map(|channel| crate::domain::event_publication_id(&event.id, channel))
            })
            .collect::<BTreeSet<_>>();
        if emitted_news_ids.iter().collect::<BTreeSet<_>>()
            != expected_news_ids.iter().collect::<BTreeSet<_>>()
        {
            return Err(KernelError::Invalid(
                "clock consequence binding did not publish the exact admitted event channels"
                    .into(),
            ));
        }
        Some(crate::clock::ClockConsequenceBindingReceipt {
            schema: "ghostlight.clock_consequence_binding_receipt.v1".into(),
            campaign_id: campaign.id,
            previous_revision,
            revision: campaign.revision,
            snapshot_binding: binding.admission.snapshot_binding,
            binding_batch_digest: binding.admission.binding_batch_digest,
            bindings: binding.admission.bindings,
            model_receipt_ids: model_receipts
                .iter()
                .map(|receipt| receipt.storage_key().to_owned())
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect(),
            accepted_model_receipt_id: binding.admission.accepted_model_receipt_id,
            emitted_event_ids: binding.emitted_event_ids,
            emitted_news_ids,
            news_count_before: binding.news_count_before,
            next_wave_index: binding.next_wave_index,
            committed_at,
        })
    } else {
        None
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
            clock_binding_receipt.as_ref(),
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
pub(crate) mod tests {
    use super::*;
    use sha2::{Digest, Sha256};
    use std::collections::{BTreeMap, BTreeSet};

    fn test_action_digest(label: &str) -> String {
        format!("sha256:{:x}", Sha256::digest(label.as_bytes()))
    }

    #[test]
    fn strategic_activity_phase_is_derived_from_exact_locations() {
        assert_eq!(
            relative_activity_phase("yard", Some("room"), &["yard".into()]),
            0
        );
        assert_eq!(
            relative_activity_phase("yard", Some("room"), &["room".into()]),
            2
        );
        assert_eq!(
            relative_activity_phase("yard", Some("room"), &["yard".into(), "room".into()]),
            1
        );
        assert_eq!(relative_activity_phase("yard", None, &["yard".into()]), 1);
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
                        crate::domain::gestalt_member_subject_id(&activity.member_id),
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

    pub(crate) fn campaign() -> Campaign {
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
            civic_systems: BTreeMap::new(),
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
            nemesis_attention_history: Vec::new(),
            strategic_tick_count: 0,
        }
    }

    fn civic_locality_elaboration() -> LocalityElaboration {
        let public_facts = [
            (
                "fact:room-authority",
                "Mayor Selka Vey holds the civic seal while the ward assembly controls appropriations.",
            ),
            (
                "fact:room-selection",
                "Residents elected Selka Vey over Oren Vale at the last mayoral ballot.",
            ),
            (
                "fact:room-resources",
                "Published berth dues fund the civic treasury.",
            ),
            (
                "fact:room-redress",
                "Residents may appeal a mayoral order to the ward petitions bench.",
            ),
        ];
        let facts = public_facts
            .iter()
            .map(|(id, statement)| WorldFact {
                id: (*id).into(),
                statement: (*statement).into(),
                scope: FactScope::BranchLocal,
                evidence_receipt_ids: vec![],
                discoverable_at_location_ids: BTreeSet::from([
                    "room".into(),
                    "civic-quarter".into(),
                ]),
            })
            .collect::<Vec<_>>();
        let institution_profile = |id: &str, authority: &str| AgencyProfile {
            schema: "ghostlight.agency_profile.v1".into(),
            id: format!("agency:{id}"),
            subject_id: id.into(),
            subject_kind: AgencySubjectKind::Institution,
            profile_version: 0,
            collective_authority_id: None,
            parent_subject_id: None,
            active_leaf: true,
            simulation_eligible: true,
            facets: BTreeMap::from([
                (AgencyAxis::Geography, BTreeSet::from(["room".into()])),
                (
                    AgencyAxis::Ideology,
                    BTreeSet::from(["public mandate".into()]),
                ),
                (AgencyAxis::Authority, BTreeSet::from([authority.into()])),
                (
                    AgencyAxis::EconomyRole,
                    BTreeSet::from(["civic administration".into()]),
                ),
                (
                    AgencyAxis::SpeciesBody,
                    BTreeSet::from(["mixed residents".into()]),
                ),
                (
                    AgencyAxis::Information,
                    BTreeSet::from(["ward notices".into()]),
                ),
            ]),
            location_ids: BTreeSet::from(["civic-quarter".into()]),
            information_channels: BTreeSet::from(["ward notice board".into()]),
            detail_debt: 0,
            last_detail_tick: 0,
            evidence_receipt_ids: vec![],
        };
        LocalityElaboration {
            target_location_id: "room".into(),
            expansion: RegionExpansion {
                origin_location_id: "room".into(),
                origin_routes: BTreeMap::from([(
                    "to-civic-quarter".into(),
                    Route {
                        destination_id: "civic-quarter".into(),
                        distance: "through the public arcade".into(),
                        travel_minutes: 5,
                    },
                )]),
                locations: vec![Location {
                    id: "civic-quarter".into(),
                    name: "Civic Quarter".into(),
                    container_id: Some("room".into()),
                    routes: BTreeMap::from([(
                        "to-room".into(),
                        Route {
                            destination_id: "room".into(),
                            distance: "back through the public arcade".into(),
                            travel_minutes: 5,
                        },
                    )]),
                    persistent_features: vec!["sealed ballot archive".into()],
                }],
                facts,
                populations: vec![GestaltPersonaState {
                    schema: "ghostlight.gestalt_persona_state.v1".into(),
                    id: "room-residents".into(),
                    name: "Room residents".into(),
                    version: 0,
                    home_location_id: "civic-quarter".into(),
                    shared_capabilities: BTreeSet::from(["participate in ward ballots".into()]),
                    shared_knowledge: public_facts
                        .iter()
                        .map(|(_, statement)| (*statement).into())
                        .collect(),
                    resources: BTreeSet::from(["ward hall".into()]),
                    goals: vec!["keep the treasury answerable".into()],
                    pressures: vec!["the mayor and assembly dispute appropriations".into()],
                }],
                population_profiles: vec![AgencyProfile {
                    schema: "ghostlight.agency_profile.v1".into(),
                    id: "agency:room-residents".into(),
                    subject_id: "room-residents".into(),
                    subject_kind: AgencySubjectKind::Gestalt,
                    profile_version: 0,
                    collective_authority_id: Some("room-residents".into()),
                    parent_subject_id: None,
                    active_leaf: true,
                    simulation_eligible: true,
                    facets: BTreeMap::from([
                        (AgencyAxis::Geography, BTreeSet::from(["room".into()])),
                        (
                            AgencyAxis::Ideology,
                            BTreeSet::from(["ward representation".into()]),
                        ),
                        (
                            AgencyAxis::Authority,
                            BTreeSet::from(["resident franchise".into()]),
                        ),
                        (
                            AgencyAxis::EconomyRole,
                            BTreeSet::from(["berth work".into()]),
                        ),
                        (
                            AgencyAxis::SpeciesBody,
                            BTreeSet::from(["mixed residents".into()]),
                        ),
                        (
                            AgencyAxis::Information,
                            BTreeSet::from(["ward notices".into()]),
                        ),
                    ]),
                    location_ids: BTreeSet::from(["civic-quarter".into()]),
                    information_channels: BTreeSet::from(["ward notice board".into()]),
                    detail_debt: 0,
                    last_detail_tick: 0,
                    evidence_receipt_ids: vec![],
                }],
                migration_relations: vec![],
                institutions: vec![
                    InstitutionState {
                        id: "mayoral-office".into(),
                        name: "Mayoral Office".into(),
                        resources: vec!["civic seal".into()],
                        goals: vec!["retain emergency spending discretion".into()],
                        posture: "press for immediate appropriation".into(),
                    },
                    InstitutionState {
                        id: "ward-assembly".into(),
                        name: "Ward Assembly".into(),
                        resources: vec!["appropriations ledger".into()],
                        goals: vec!["bind spending to public accounts".into()],
                        posture: "withhold funds pending audit".into(),
                    },
                ],
                institution_profiles: vec![
                    institution_profile("mayoral-office", "mayoral orders"),
                    institution_profile("ward-assembly", "appropriations"),
                ],
                local_relations: vec![AgencyRelation {
                    schema: "ghostlight.agency_relation.v1".into(),
                    id: "relation:mayor-assembly".into(),
                    from_subject_id: "mayoral-office".into(),
                    to_subject_id: "ward-assembly".into(),
                    kind: AgencyRelationKind::Rivalry,
                    strength: 72,
                    active: true,
                    evidence_receipt_ids: vec![],
                }],
                civic_system: Some(CivicSystemManifest {
                    schema: "ghostlight.civic_system_manifest.v1".into(),
                    version: 0,
                    jurisdiction_location_id: "room".into(),
                    governing_institution_ids: BTreeSet::from([
                        "mayoral-office".into(),
                        "ward-assembly".into(),
                    ]),
                    resident_population_ids: BTreeSet::from(["room-residents".into()]),
                    public_authority_fact_ids: BTreeSet::from(["fact:room-authority".into()]),
                    public_selection_fact_ids: BTreeSet::from(["fact:room-selection".into()]),
                    public_resource_fact_ids: BTreeSet::from(["fact:room-resources".into()]),
                    public_redress_fact_ids: BTreeSet::from(["fact:room-redress".into()]),
                    political_relation_ids: BTreeSet::from(["relation:mayor-assembly".into()]),
                    semantic_verification_receipt_id: String::new(),
                }),
            },
        }
    }

    fn civic_verifier_receipt(
        campaign: &Campaign,
        expansion: &mut RegionExpansion,
        source_receipt_ids: Vec<String>,
    ) -> crate::model::ModelStageReceipt {
        let digest = crate::compiler::civic_candidate_digest(expansion).unwrap();
        let mut receipt = crate::model::ModelStageReceipt {
            schema: "ghostlight.persona_stage_receipt.v1".into(),
            receipt_hash: String::new(),
            provider: "fixture".into(),
            model: "fixture".into(),
            stage: "destination_civic_verification".into(),
            snapshot_binding: String::new(),
            request_hash: test_action_digest("civic-verifier-request"),
            output_hash: test_action_digest("civic-verifier-output"),
            source_receipt_ids,
            latency_ms: 1,
            validation_result: "valid".into(),
            local_validation_error: None,
            input_chars: 1,
            output_chars: 1,
            provider_attempts: vec![],
        };
        receipt.rebind_snapshot(crate::compiler::civic_verifier_binding(campaign, &digest));
        expansion
            .civic_system
            .as_mut()
            .unwrap()
            .semantic_verification_receipt_id = receipt.storage_key().to_owned();
        receipt
    }

    fn titled_operations_for(
        elaboration: &LocalityElaboration,
    ) -> Vec<(
        crate::elaboration::ElaboratorTitle,
        crate::elaboration::WorldElaborationOperation,
    )> {
        use crate::elaboration::{ElaboratorTitle::*, WorldElaborationOperation::*};
        let expansion = &elaboration.expansion;
        let mut operations = Vec::new();
        for location in &expansion.locations {
            operations.push((
                Patina,
                AddPlace {
                    id: location.id.clone(),
                    name: location.name.clone(),
                    container_id: location.container_id.clone(),
                    persistent_features: location.persistent_features.clone(),
                },
            ));
            operations.extend(location.routes.iter().map(|(route_id, route)| {
                (
                    Ledger,
                    AddRoute {
                        origin_location_id: location.id.clone(),
                        route_id: route_id.clone(),
                        route: route.clone(),
                    },
                )
            }));
        }
        operations.extend(expansion.origin_routes.iter().map(|(route_id, route)| {
            (
                Ledger,
                AddRoute {
                    origin_location_id: expansion.origin_location_id.clone(),
                    route_id: route_id.clone(),
                    route: route.clone(),
                },
            )
        }));
        operations.extend(
            expansion
                .facts
                .iter()
                .cloned()
                .map(|fact| (Charter, AddFact { fact })),
        );
        operations.extend(
            expansion
                .populations
                .iter()
                .cloned()
                .zip(expansion.population_profiles.iter().cloned())
                .map(|(population, profile)| {
                    (
                        Hearth,
                        AddPopulation {
                            population,
                            profile,
                        },
                    )
                }),
        );
        operations.extend(
            expansion
                .institutions
                .iter()
                .cloned()
                .zip(expansion.institution_profiles.iter().cloned())
                .map(|(institution, profile)| {
                    (
                        Charter,
                        AddInstitution {
                            institution,
                            profile,
                        },
                    )
                }),
        );
        operations.extend(
            expansion
                .migration_relations
                .iter()
                .cloned()
                .map(|relation| (Hearth, AddMigrationRelation { relation })),
        );
        operations.extend(
            expansion
                .local_relations
                .iter()
                .cloned()
                .map(|relation| (Tangle, AddLocalRelation { relation })),
        );
        if let Some(system) = &expansion.civic_system {
            operations.push((
                Charter,
                SetCivicSystem {
                    system: system.clone(),
                },
            ));
        }
        operations
    }

    struct FixtureWorldElaborationSubAgent {
        operations: BTreeMap<
            crate::elaboration::ElaboratorTitle,
            Vec<crate::elaboration::WorldElaborationOperation>,
        >,
    }

    #[async_trait::async_trait]
    impl crate::elaboration::ElaborationSubAgentPort<crate::elaboration::WorldElaborationProposal>
        for FixtureWorldElaborationSubAgent
    {
        async fn invoke(
            &self,
            invocation: crate::elaboration::ElaborationSubAgentInvocation,
        ) -> std::result::Result<
            crate::elaboration::ElaborationSubAgentOutput<
                crate::elaboration::WorldElaborationProposal,
            >,
            crate::elaboration::ElaborationSubAgentFailure,
        > {
            let index = invocation.dispatch.title_dispatch_count as usize - 1;
            let operation = self
                .operations
                .get(&invocation.dispatch.title)
                .and_then(|operations| operations.get(index))
                .cloned()
                .ok_or_else(|| crate::elaboration::ElaborationSubAgentFailure {
                    diagnostic: "fixture elaborator received an extra dispatch".into(),
                    model_stage_receipts: Vec::new(),
                })?;
            let binding = crate::elaboration::world_elaboration_invocation_binding(
                &invocation.wave,
                &invocation.dispatch,
            )
            .map_err(|error| crate::elaboration::ElaborationSubAgentFailure {
                diagnostic: error.to_string(),
                model_stage_receipts: Vec::new(),
            })?;
            let mut model_receipt = crate::model::ModelStageReceipt {
                schema: "ghostlight.persona_stage_receipt.v1".into(),
                receipt_hash: String::new(),
                provider: "fixture".into(),
                model: "fixture-model".into(),
                stage: format!(
                    "world_elaboration_{}",
                    invocation
                        .dispatch
                        .title
                        .display_name()
                        .to_ascii_lowercase()
                ),
                snapshot_binding: String::new(),
                request_hash: test_action_digest(&format!(
                    "elaboration-request:{}",
                    invocation.dispatch.ordinal
                )),
                output_hash: test_action_digest(&format!(
                    "elaboration-output:{}",
                    invocation.dispatch.ordinal
                )),
                source_receipt_ids: Vec::new(),
                latency_ms: 1,
                validation_result: "valid".into(),
                local_validation_error: None,
                input_chars: 1,
                output_chars: 1,
                provider_attempts: Vec::new(),
            };
            model_receipt.rebind_snapshot(binding);
            Ok(crate::elaboration::ElaborationSubAgentOutput {
                proposal: crate::elaboration::WorldElaborationProposal {
                    schema: "ghostlight.world_elaboration_proposal.v1".into(),
                    operation,
                },
                model_stage_receipts: vec![model_receipt],
            })
        }
    }

    async fn dispatch_titled_operations(
        campaign: &Campaign,
        target_location_id: &str,
        operations: Vec<(
            crate::elaboration::ElaboratorTitle,
            crate::elaboration::WorldElaborationOperation,
        )>,
    ) -> crate::elaboration::ElaborationWaveRun<crate::elaboration::WorldElaborationProposal> {
        use crate::elaboration::*;
        let mut by_title = BTreeMap::<ElaboratorTitle, Vec<WorldElaborationOperation>>::new();
        for (title, operation) in operations {
            by_title.entry(title).or_default().push(operation);
        }
        let profile = WorldElaborationProfile {
            schema: "ghostlight.world_elaboration_profile.v1".into(),
            controls: by_title
                .iter()
                .map(|(title, operations)| ElaboratorControl {
                    title: *title,
                    weight: operations.len() as u16,
                })
                .collect(),
        };
        let invocation_budget = by_title.values().map(Vec::len).sum::<usize>() as u32;
        let eligible_titles = by_title.keys().copied().collect::<BTreeSet<_>>();
        let mut scheduler = ElaborationScheduler::new(&profile).unwrap();
        dispatch_elaboration_wave(
            &mut scheduler,
            world_elaboration_wave_binding(campaign, target_location_id).unwrap(),
            &eligible_titles,
            invocation_budget,
            4,
            std::sync::Arc::new(FixtureWorldElaborationSubAgent {
                operations: by_title,
            }),
        )
        .await
        .unwrap()
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
            intent: "cross into the yard, inspect it, and brace one loose gate".into(),
            intended_effect: "arrive, identify one local hazard, and attempt a bounded repair"
                .into(),
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
                StrategicCellEffect::ActorActivity {
                    actor_id: "runner".into(),
                    activity: StrategicActivityKind::Prepare,
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
            vec!["actor_movement", "actor_activity", "actor_activity"]
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
            civic_systems: BTreeMap::new(),
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
        store
            .create_unadmitted_fixture_campaign(&aggregate, &[], &[])
            .unwrap();
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
            "Mira Venn moves from Transit camp to South docks and joins South dock neighbors."
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
                &crate::domain::gestalt_member_subject_id(id),
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
        assert_eq!(
            events[0].summary,
            "Eastern transit refugees relocates from Transit camp to South docks near South dock neighbors."
        );
        assert!(!events[0].summary.contains("refugees-east"));
        assert!(!events[0].summary.contains("dock-neighbors"));
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
        assert_eq!(events.len(), 2);
        assert_eq!(events[1].kind, "gestalt_activity");
        assert_eq!(events[1].location_ids, vec!["camp"]);
        assert_eq!(
            events[1].summary,
            "Camp neighbors sends a communication to Eastern transit refugees."
        );
    }

    #[test]
    fn cohesive_gestalt_internal_coordination_reaches_canonical_and_legacy_commit_paths() {
        let action = CellActionProposal {
            subject_id: "refugees-east".into(),
            intent: "coordinate the households' own water watch".into(),
            intended_effect:
                "coordinate internal water-watch shifts without naming an external party".into(),
            priority: 80,
            state_references: vec![],
            public_channels: vec![],
            effects: vec![StrategicCellEffect::GestaltActivity {
                gestalt_id: "refugees-east".into(),
                activity: StrategicActivityKind::Coordinate,
                target_subject_ids: vec![],
                location_ids: vec!["camp".into()],
            }],
        };
        let action_digest = crate::resolution::cell_action_digest(&action).unwrap();
        let mut canonical = hierarchical_refugee_campaign();
        let canonical_events = apply_strategic_tick_plan(
            &mut canonical,
            StrategicTickPlan {
                selected_actions: vec![action],
                activity_outcomes: vec![StrategicActivityOutcome {
                    schema: "ghostlight.strategic_activity_outcome.v1".into(),
                    action_digest,
                    source_subject_id: "refugees-east".into(),
                    band: StrategicOutcomeBand::Mixed,
                    summary: "The households arrange the watch; its efficacy remains unsettled."
                        .into(),
                    supporting_state_references: vec![],
                    effect: StrategicOutcomeEffect::NoMaterialChange {
                        reason: "No external response or durable result is established.".into(),
                    },
                }],
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(canonical_events.len(), 1);
        assert_eq!(canonical_events[0].kind, "gestalt_activity");
        assert!(
            canonical_events[0]
                .summary
                .contains("internal water-watch shifts")
        );

        let mut legacy = hierarchical_refugee_campaign();
        let legacy_events = apply_strategic_tick_plan(
            &mut legacy,
            resolve_test_activities(StrategicTickPlan {
                gestalt_activities: vec![StrategicGestaltActivity {
                    action_digest: test_action_digest("internal water-watch coordination"),
                    gestalt_id: "refugees-east".into(),
                    activity: StrategicActivityKind::Coordinate,
                    target_subject_ids: vec![],
                    location_ids: vec!["camp".into()],
                    public_channels: vec![],
                }],
                ..Default::default()
            }),
        )
        .unwrap();
        assert_eq!(legacy_events.len(), 1);
        assert_eq!(
            legacy_events[0].summary,
            "Eastern transit refugees attempts internal coordination."
        );
    }

    #[test]
    fn institution_posture_uses_the_canonical_character_bound_at_commit() {
        let mut value = campaign();
        value.institutions.insert(
            "board".into(),
            InstitutionState {
                id: "board".into(),
                name: "Board".into(),
                resources: vec![],
                goals: vec![],
                posture: "watching".into(),
            },
        );
        crate::resolution::ensure_agency_profiles(&mut value);
        let events = apply_strategic_tick_plan(
            &mut value,
            StrategicTickPlan {
                institution_actions: vec![StrategicInstitutionAction {
                    institution_id: "board".into(),
                    posture: "x".repeat(MAX_POSTURE_CHARS),
                    location_ids: vec![],
                    public_channels: vec![],
                }],
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(events.len(), 1);
        assert!(
            events[0]
                .summary
                .starts_with("Board announces a new course: ")
        );
        assert!(!events[0].summary.contains("adopts posture"));
        assert_eq!(
            value.institutions["board"].posture.chars().count(),
            MAX_POSTURE_CHARS
        );

        let before_oversized = value.clone();
        assert!(
            apply_strategic_tick_plan(
                &mut value,
                StrategicTickPlan {
                    institution_actions: vec![StrategicInstitutionAction {
                        institution_id: "board".into(),
                        posture: "y".repeat(MAX_POSTURE_CHARS + 1),
                        location_ids: vec![],
                        public_channels: vec![],
                    }],
                    ..Default::default()
                },
            )
            .is_err()
        );
        assert_eq!(value, before_oversized);
    }

    #[test]
    fn canonical_action_commits_distinct_scopes_of_one_activity_kind() {
        let mut value = hierarchical_refugee_campaign();
        let targeted = StrategicCellEffect::GestaltActivity {
            gestalt_id: "refugees-east".into(),
            activity: StrategicActivityKind::Communicate,
            target_subject_ids: vec!["dock-neighbors".into()],
            location_ids: vec!["camp".into()],
        };
        let local = StrategicCellEffect::GestaltActivity {
            gestalt_id: "refugees-east".into(),
            activity: StrategicActivityKind::Communicate,
            target_subject_ids: vec![],
            location_ids: vec!["camp".into()],
        };
        let action = CellActionProposal {
            subject_id: "refugees-east".into(),
            intent: "Warn the dock neighbors and the unnamed camp assembly.".into(),
            intended_effect: "Send both warnings without merging their audiences.".into(),
            priority: 80,
            state_references: vec![],
            public_channels: vec![],
            effects: vec![targeted.clone(), local],
        };
        let action_digest = crate::resolution::cell_action_digest(&action).unwrap();
        let events = apply_strategic_tick_plan(
            &mut value,
            StrategicTickPlan {
                selected_actions: vec![action],
                activity_outcomes: vec![StrategicActivityOutcome {
                    schema: "ghostlight.strategic_activity_outcome.v1".into(),
                    action_digest,
                    source_subject_id: "refugees-east".into(),
                    band: StrategicOutcomeBand::Mixed,
                    summary: "The warnings are sent; their reception remains unsettled.".into(),
                    supporting_state_references: vec![],
                    effect: StrategicOutcomeEffect::NoMaterialChange {
                        reason: "No response is established in this test snapshot.".into(),
                    },
                }],
                ..Default::default()
            },
        )
        .unwrap();

        assert_eq!(events.len(), 2);
        assert!(events.iter().all(|event| event.kind == "gestalt_activity"));
        assert_eq!(
            events
                .iter()
                .map(|event| event.id.as_str())
                .collect::<BTreeSet<_>>()
                .len(),
            2
        );

        let before_duplicate = value.clone();
        let duplicate = CellActionProposal {
            subject_id: "refugees-east".into(),
            intent: "Repeat one exact warning twice.".into(),
            intended_effect: "Send the same warning twice.".into(),
            priority: 80,
            state_references: vec![],
            public_channels: vec![],
            effects: vec![targeted.clone(), targeted],
        };
        let duplicate_digest = crate::resolution::cell_action_digest(&duplicate).unwrap();
        let error = apply_strategic_tick_plan(
            &mut value,
            StrategicTickPlan {
                selected_actions: vec![duplicate],
                activity_outcomes: vec![StrategicActivityOutcome {
                    schema: "ghostlight.strategic_activity_outcome.v1".into(),
                    action_digest: duplicate_digest,
                    source_subject_id: "refugees-east".into(),
                    band: StrategicOutcomeBand::Mixed,
                    summary: "The repeated warning has no separate durable result.".into(),
                    supporting_state_references: vec![],
                    effect: StrategicOutcomeEffect::NoMaterialChange {
                        reason: "No distinct response is established.".into(),
                    },
                }],
                ..Default::default()
            },
        )
        .unwrap_err();
        assert!(error.to_string().contains("acts twice"), "{error}");
        assert_eq!(value, before_duplicate);

        let aliased = CellActionProposal {
            subject_id: "refugees-east".into(),
            intent: "Repeat one local warning through aliased location syntax.".into(),
            intended_effect: "Send one local warning.".into(),
            priority: 80,
            state_references: vec![],
            public_channels: vec![],
            effects: vec![
                StrategicCellEffect::GestaltActivity {
                    gestalt_id: "refugees-east".into(),
                    activity: StrategicActivityKind::Communicate,
                    target_subject_ids: vec![],
                    location_ids: vec![],
                },
                StrategicCellEffect::GestaltActivity {
                    gestalt_id: "refugees-east".into(),
                    activity: StrategicActivityKind::Communicate,
                    target_subject_ids: vec![],
                    location_ids: vec!["camp".into()],
                },
            ],
        };
        let aliased_digest = crate::resolution::cell_action_digest(&aliased).unwrap();
        let error = apply_strategic_tick_plan(
            &mut value,
            StrategicTickPlan {
                selected_actions: vec![aliased],
                activity_outcomes: vec![StrategicActivityOutcome {
                    schema: "ghostlight.strategic_activity_outcome.v1".into(),
                    action_digest: aliased_digest,
                    source_subject_id: "refugees-east".into(),
                    band: StrategicOutcomeBand::Mixed,
                    summary: "The local warning has no separate durable result.".into(),
                    supporting_state_references: vec![],
                    effect: StrategicOutcomeEffect::NoMaterialChange {
                        reason: "No distinct response is established.".into(),
                    },
                }],
                ..Default::default()
            },
        )
        .unwrap_err();
        assert!(error.to_string().contains("acts twice"), "{error}");
        assert_eq!(value, before_duplicate);
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
            "A declaration from Eastern transit refugees marks this matter settled: the storm closes the camp; New public demand from Eastern transit refugees: shelter assignments remain unsettled"
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
        assert_eq!(events.len(), 1);
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
        assert!(
            events
                .iter()
                .all(|event| event.kind != "strategic_activity_outcome")
        );
    }

    #[test]
    fn durable_publication_reports_the_published_course_not_an_unfinished_publication_attempt() {
        let summary = strategic_activity_summary(
            "Thornweald Charcoal Guilds",
            &StrategicActivityKind::Communicate,
            &["Copper Synod".into()],
            Some("Publish the caravan evidence and call every forest kiln to readiness"),
            &["root-wire broadsheet".into()],
        );

        assert_eq!(
            summary,
            "Public statement from Thornweald Charcoal Guilds: Publish the caravan evidence and call every forest kiln to readiness."
        );
        assert!(!summary.contains("attempt"));
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
        assert_eq!(events.len(), 1);
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
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].kind, "gestalt_member_activity");
        assert_eq!(
            events[0].summary,
            "Mira Venn sends a communication to Eastern transit refugees."
        );
        assert_eq!(events[0].actor_ids, vec!["member:mira"]);
        assert_eq!(events[0].gestalt_ids, vec!["refugees-east"]);
    }

    #[tokio::test]
    async fn clock_binding_commit_is_payload_bound_atomic_and_preserves_player() {
        let dir = tempfile::tempdir().unwrap();
        let store = CampaignStore::open(dir.path().join("campaign.cc")).unwrap();
        let mut seed = campaign();
        seed.revision = 14;
        seed.institutions.insert(
            "court".into(),
            InstitutionState {
                id: "court".into(),
                name: "Court".into(),
                resources: Vec::new(),
                goals: Vec::new(),
                posture: "counting knives".into(),
            },
        );
        crate::resolution::ensure_agency_profiles(&mut seed);
        seed.agency_profiles
            .get_mut("court")
            .unwrap()
            .information_channels
            .extend(["court broadsheet".into(), "palace wire".into()]);
        seed.clocks.insert(
            "coup".into(),
            WorldClock {
                id: "coup".into(),
                label: "Coup".into(),
                progress: 3,
                threshold: 3,
                consequence: "The palace guard arrests the regent at breakfast.".into(),
                consequence_scope: WorldEventScope::default(),
            },
        );
        seed.clocks.insert(
            "blackout".into(),
            WorldClock {
                id: "blackout".into(),
                label: "Blackout".into(),
                progress: 2,
                threshold: 2,
                consequence: "The archive lamps go dark and the sealed rolls vanish.".into(),
                consequence_scope: WorldEventScope::default(),
            },
        );
        store
            .create_unadmitted_fixture_campaign(&seed, &[], &[])
            .unwrap();
        let player_before = rmp_serde::to_vec_named(&seed.actors["player"]).unwrap();
        let bindings = vec![
            crate::clock::ClockConsequenceBinding {
                clock_id: "blackout".into(),
                scope: WorldEventScope {
                    actor_ids: Vec::new(),
                    institution_ids: vec!["court".into()],
                    gestalt_ids: Vec::new(),
                    location_ids: vec!["room".into()],
                    public_channels: Vec::new(),
                },
            },
            crate::clock::ClockConsequenceBinding {
                clock_id: "coup".into(),
                scope: WorldEventScope {
                    actor_ids: Vec::new(),
                    institution_ids: vec!["court".into()],
                    gestalt_ids: Vec::new(),
                    location_ids: vec!["room".into()],
                    public_channels: vec!["court broadsheet".into(), "palace wire".into()],
                },
            },
        ];
        let snapshot_binding = crate::clock::clock_consequence_binding_snapshot(&seed).unwrap();
        let binding_batch_digest =
            crate::clock::clock_consequence_binding_batch_digest(&seed, &bindings).unwrap();
        let mut accepted_receipt = crate::model::ModelStageReceipt {
            schema: "ghostlight.persona_stage_receipt.v1".into(),
            receipt_hash: String::new(),
            provider: "fixture".into(),
            model: "fixture-terra".into(),
            stage: crate::clock::CLOCK_CONSEQUENCE_BINDING_STAGE.into(),
            snapshot_binding: String::new(),
            request_hash: test_action_digest("clock-binding-request"),
            output_hash: test_action_digest("clock-binding-output"),
            source_receipt_ids: Vec::new(),
            latency_ms: 1,
            validation_result: "valid".into(),
            local_validation_error: None,
            input_chars: 1,
            output_chars: 1,
            provider_attempts: Vec::new(),
        };
        accepted_receipt.rebind_snapshot(crate::clock::clock_consequence_admission_binding(
            &snapshot_binding,
            &binding_batch_digest,
        ));
        let admission = crate::clock::ClockConsequenceBindingAdmission {
            schema: "ghostlight.clock_consequence_binding_admission.v1".into(),
            campaign_id: seed.id,
            expected_revision: seed.revision,
            snapshot_binding,
            binding_batch_digest,
            bindings: bindings.clone(),
            accepted_model_receipt_id: accepted_receipt.storage_key().to_owned(),
        };
        let kernel = WorldKernel::start(store.clone());
        let CommandResult::Committed { campaign, .. } = kernel
            .command(WorldCommand::BindClockConsequences {
                expected_revision: seed.revision,
                admission,
                model_stage_receipts: vec![accepted_receipt],
            })
            .await
            .unwrap()
        else {
            panic!("clock binding did not commit")
        };

        assert_eq!(
            rmp_serde::to_vec_named(&campaign.actors["player"]).unwrap(),
            player_before
        );
        let (_, receipt) = store
            .load::<crate::clock::ClockConsequenceBindingReceipt>(
                "clock_consequence_binding_receipt.v1",
                &format!("{}-{}", campaign.id, campaign.revision),
            )
            .unwrap()
            .unwrap();
        assert_eq!(receipt.bindings, bindings);
        assert_eq!(
            receipt.emitted_event_ids,
            ["clock-consequence:blackout", "clock-consequence:coup"]
        );
        assert_eq!(receipt.emitted_news_ids.len(), 2);
        assert_eq!(receipt.news_count_before, 0);
        assert_eq!(receipt.next_wave_index, 1);
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
    async fn speech_can_bind_a_nearby_folded_person_then_materialize_that_exact_member() {
        let dir = tempfile::tempdir().unwrap();
        let store = CampaignStore::open(dir.path().join("campaign.cc")).unwrap();
        let kernel = WorldKernel::start(store);
        let mut seed = campaign();
        seed.gestalts.insert(
            "refugees".into(),
            GestaltPersonaState {
                schema: "ghostlight.gestalt_persona_state.v1".into(),
                id: "refugees".into(),
                name: "Refugees".into(),
                version: 4,
                home_location_id: "room".into(),
                shared_capabilities: BTreeSet::new(),
                shared_knowledge: BTreeSet::new(),
                resources: BTreeSet::new(),
                goals: vec![],
                pressures: vec![],
            },
        );
        seed.gestalt_members.insert(
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
                memories: vec!["Ash repaired the coupling".into()],
                last_location_id: Some("room".into()),
                materialized_actor_id: None,
                last_relevant_revision: 0,
                relevance_lease_until_revision: 0,
            },
        );
        crate::resolution::ensure_agency_profiles(&mut seed);
        kernel
            .command(WorldCommand::CreateCampaign {
                campaign: seed,
                evidence_receipts: vec![],
                model_stage_receipts: vec![],
            })
            .await
            .unwrap();

        let spoken = kernel
            .command(WorldCommand::Speak {
                expected_revision: 0,
                actor_id: "player".into(),
                text: "Taren with the water handcart, is the coupling holding?".into(),
                intended_effect: None,
                persona_response_actor_ids: BTreeSet::from(["member:water-cart-taren".into()]),
            })
            .await
            .unwrap();
        let CommandResult::Committed { campaign, .. } = spoken else {
            panic!("speech did not commit")
        };
        assert!(!campaign.actors.contains_key("member:water-cart-taren"));
        let plan = crate::gestalt::required_addressed_promotions(&campaign).unwrap();
        assert_eq!(plan.promotions.len(), 1);
        assert_eq!(plan.promotions[0].expected_gestalt_version, 4);
        assert_eq!(plan.promotions[0].expected_member_version, 7);

        let reconciled = kernel
            .command(WorldCommand::ReconcileGestaltPresence {
                expected_revision: 1,
                reason: "player says: Taren with the water handcart, is the coupling holding?"
                    .into(),
                plan,
            })
            .await
            .unwrap();
        let CommandResult::Committed { campaign, .. } = reconciled else {
            panic!("addressed member was not materialized")
        };
        assert!(campaign.actors.contains_key("member:water-cart-taren"));
        assert_eq!(
            campaign.gestalt_members["water-cart-taren"]
                .materialized_actor_id
                .as_deref(),
            Some("member:water-cart-taren")
        );
        let arrival = campaign.transcript.last().unwrap();
        assert_eq!(arrival.speaker, "world");
        assert!(arrival.text.contains("Taren"));
        assert!(arrival.text.contains("Refugees"));
        assert!(arrival.text.contains("Room"));
        assert!(
            campaign
                .transcript
                .iter()
                .rev()
                .find(|turn| turn.speaker == campaign.player_actor_id)
                .unwrap()
                .persona_response_actor_ids
                == BTreeSet::from(["member:water-cart-taren".into()])
        );
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
        seed.institutions.insert(
            "clinic".into(),
            InstitutionState {
                id: "clinic".into(),
                name: "Clinic".into(),
                resources: Vec::new(),
                goals: Vec::new(),
                posture: "repairing the regulator".into(),
            },
        );
        seed.clocks.insert(
            "clinic-failure".into(),
            WorldClock {
                id: "clinic-failure".into(),
                label: "Clinic failure".into(),
                progress: 3,
                threshold: 4,
                consequence: "The regulator fails.".into(),
                consequence_scope: WorldEventScope {
                    actor_ids: Vec::new(),
                    institution_ids: vec!["clinic".into()],
                    gestalt_ids: Vec::new(),
                    location_ids: vec!["room".into()],
                    public_channels: Vec::new(),
                },
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
    async fn strategic_tick_receipt_includes_derived_clock_consequence() {
        let dir = tempfile::tempdir().unwrap();
        let store = CampaignStore::open(dir.path().join("campaign.cc")).unwrap();
        let kernel = WorldKernel::start(store.clone());
        let mut seed = campaign();
        seed.institutions.insert(
            "court".into(),
            InstitutionState {
                id: "court".into(),
                name: "Court".into(),
                resources: Vec::new(),
                goals: Vec::new(),
                posture: "waiting for the bell".into(),
            },
        );
        crate::resolution::ensure_agency_profiles(&mut seed);
        seed.agency_profiles
            .get_mut("court")
            .unwrap()
            .information_channels
            .insert("court broadsheet".into());
        seed.clocks.insert(
            "coup".into(),
            WorldClock {
                id: "coup".into(),
                label: "Coup".into(),
                progress: 0,
                threshold: 1,
                consequence: "The palace guard arrests the regent at breakfast.".into(),
                consequence_scope: WorldEventScope {
                    actor_ids: Vec::new(),
                    institution_ids: vec!["court".into()],
                    gestalt_ids: Vec::new(),
                    location_ids: vec!["room".into()],
                    public_channels: vec!["court broadsheet".into()],
                },
            },
        );
        kernel
            .command(WorldCommand::CreateCampaign {
                campaign: seed,
                evidence_receipts: Vec::new(),
                model_stage_receipts: Vec::new(),
            })
            .await
            .unwrap();
        let CommandResult::Committed { campaign, .. } = kernel
            .command(WorldCommand::AdvanceStrategicTick {
                expected_revision: 0,
                source: TickSource::Scheduler,
                plan: None,
                model_receipt_hash: None,
                resolution_wave: None,
            })
            .await
            .unwrap()
        else {
            panic!("expected strategic tick commit")
        };
        let ticks = store
            .load_all::<crate::domain::StrategicTickReceipt>("strategic_tick.v1")
            .unwrap();

        assert_eq!(ticks[0].event_ids, ["clock-consequence:coup"]);
        assert!(
            campaign
                .events
                .iter()
                .any(|event| event.id == "clock-consequence:coup")
        );
        assert!(
            campaign
                .news
                .iter()
                .any(|news| news.event_ids == ["clock-consequence:coup"])
        );
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
                routes: BTreeMap::from([(
                    "back".into(),
                    Route {
                        destination_id: "room".into(),
                        distance: "near".into(),
                        travel_minutes: 20,
                    },
                )]),
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
        assert_eq!(
            campaign.events[0].summary,
            "Runner moves from Room to Yard."
        );
        assert!(!campaign.events[0].summary.contains("room"));
        assert!(!campaign.events[0].summary.contains("yard"));
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
                .all(|event| event.kind != "strategic_activity_outcome")
        );

        let origin_activity = crate::domain::CellActionProposal {
            subject_id: "runner".into(),
            intent: "Repair the yard marker, then return to the room.".into(),
            intended_effect: "Attempt the repair before leaving the yard.".into(),
            priority: 50,
            state_references: vec![],
            public_channels: vec![],
            effects: vec![
                crate::domain::StrategicCellEffect::ActorMove {
                    actor_id: "runner".into(),
                    destination_id: "room".into(),
                },
                crate::domain::StrategicCellEffect::ActorActivity {
                    actor_id: "runner".into(),
                    activity: StrategicActivityKind::Prepare,
                    target_subject_ids: vec![],
                    location_ids: vec!["yard".into()],
                },
            ],
        };
        let action_digest = crate::resolution::cell_action_digest(&origin_activity).unwrap();
        let result = kernel
            .command(WorldCommand::AdvanceStrategicTick {
                expected_revision: 2,
                source: TickSource::Scheduler,
                plan: Some(StrategicTickPlan {
                    selected_actions: vec![origin_activity],
                    activity_outcomes: vec![StrategicActivityOutcome {
                        schema: "ghostlight.strategic_activity_outcome.v1".into(),
                        action_digest: action_digest.clone(),
                        source_subject_id: "runner".into(),
                        band: StrategicOutcomeBand::Mixed,
                        summary: "The yard repair remains provisional.".into(),
                        supporting_state_references: vec![],
                        effect: StrategicOutcomeEffect::NoMaterialChange {
                            reason: "The repair did not establish a durable state change.".into(),
                        },
                    }],
                    ..StrategicTickPlan::default()
                }),
                model_receipt_hash: Some(format!("sha256:{}", "d".repeat(64))),
                resolution_wave: None,
            })
            .await
            .unwrap();
        let CommandResult::Committed { campaign, .. } = result else {
            panic!("expected commit")
        };
        assert_eq!(campaign.actors["runner"].location_id, "room");
        let activity_index = campaign
            .events
            .iter()
            .position(|event| {
                event
                    .id
                    .starts_with("strategic:3:actor-activity:runner:prepare:")
            })
            .unwrap();
        let movement_index = campaign
            .events
            .iter()
            .position(|event| event.id == "strategic:3:actor:runner")
            .unwrap();
        assert_eq!(
            campaign.events[activity_index].summary,
            "Runner attempts the repair before leaving the yard."
        );
        assert!(
            campaign
                .events
                .iter()
                .all(|event| !event.id.starts_with("strategic:3:activity-outcome:"))
        );
        assert!(activity_index < movement_index);
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
                    populations: vec![],
                    population_profiles: vec![],
                    migration_relations: vec![],
                    institutions: vec![],
                    institution_profiles: vec![],
                    local_relations: vec![],
                    civic_system: None,
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
    async fn inhabited_region_admission_preserves_people_and_invalidates_only_derived_cover() {
        let dir = tempfile::tempdir().unwrap();
        let store = CampaignStore::open(dir.path().join("campaign.cc")).unwrap();
        let kernel = WorldKernel::start(store.clone());
        let mut seed = campaign();
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
                goals: vec!["seek voluntary settlement".into()],
                pressures: vec![],
            },
        );
        seed.gestalt_members.insert(
            "taren".into(),
            GestaltMemberDelta {
                schema: "ghostlight.gestalt_member_delta.v1".into(),
                id: "taren".into(),
                gestalt_id: "refugees".into(),
                version: 3,
                name: "Taren".into(),
                capability_additions: BTreeSet::new(),
                capability_removals: BTreeSet::new(),
                knowledge_additions: BTreeSet::new(),
                knowledge_removals: BTreeSet::new(),
                equipment: BTreeSet::new(),
                conditions: BTreeSet::new(),
                obligations: BTreeSet::new(),
                relationships: BTreeMap::new(),
                goals: vec!["reach the ridge villages".into()],
                memories: vec!["Ash repaired my regulator".into()],
                last_location_id: Some("room".into()),
                materialized_actor_id: None,
                last_relevant_revision: 0,
                relevance_lease_until_revision: 0,
            },
        );
        crate::resolution::ensure_agency_profiles(&mut seed);
        let original_member = seed.gestalt_members["taren"].clone();
        let original_epoch = seed.resolution_policy.resolution_epoch;
        kernel
            .command(WorldCommand::CreateCampaign {
                campaign: seed,
                evidence_receipts: vec![],
                model_stage_receipts: vec![],
            })
            .await
            .unwrap();
        let evidence = VaultEvidenceReceipt {
            schema: "ghostlight.vault_evidence_receipt.v1".into(),
            id: "vault:ridge".into(),
            provider: "fixture".into(),
            query_hash: "sha256:q".into(),
            witnesses: vec![],
            retrieved_at: Utc::now(),
        };
        let statement = "The ridge assembly governs voluntary admission.".to_owned();
        let population = GestaltPersonaState {
            schema: "ghostlight.gestalt_persona_state.v1".into(),
            id: "ridge-households".into(),
            name: "Ridge Households".into(),
            version: 0,
            home_location_id: "ridge".into(),
            shared_capabilities: BTreeSet::from(["communal agriculture".into()]),
            shared_knowledge: BTreeSet::from([statement.clone()]),
            resources: BTreeSet::from(["shared kitchen".into()]),
            goals: vec!["decide admissions collectively".into()],
            pressures: vec!["winter capacity is finite".into()],
        };
        let profile = AgencyProfile {
            schema: "ghostlight.agency_profile.v1".into(),
            id: "agency:ridge-households".into(),
            subject_id: population.id.clone(),
            subject_kind: AgencySubjectKind::Gestalt,
            profile_version: 0,
            collective_authority_id: Some(population.id.clone()),
            parent_subject_id: None,
            active_leaf: true,
            simulation_eligible: true,
            facets: BTreeMap::from([
                (AgencyAxis::Geography, BTreeSet::from(["ridge".into()])),
                (AgencyAxis::Ideology, BTreeSet::from(["consent".into()])),
                (AgencyAxis::Authority, BTreeSet::from(["assembly".into()])),
                (
                    AgencyAxis::EconomyRole,
                    BTreeSet::from(["agriculture".into()]),
                ),
                (AgencyAxis::SpeciesBody, BTreeSet::from(["mixed".into()])),
                (AgencyAxis::Information, BTreeSet::from(["local".into()])),
            ]),
            location_ids: BTreeSet::from(["ridge".into()]),
            information_channels: BTreeSet::from(["village assembly bulletin".into()]),
            detail_debt: 0,
            last_detail_tick: 0,
            evidence_receipt_ids: vec![evidence.id.clone()],
        };
        let relation = AgencyRelation {
            schema: "ghostlight.agency_relation.v1".into(),
            id: "migration:refugees:ridge".into(),
            from_subject_id: "refugees".into(),
            to_subject_id: population.id.clone(),
            kind: AgencyRelationKind::Migration,
            strength: 50,
            active: true,
            evidence_receipt_ids: vec![evidence.id.clone()],
        };
        let result = kernel
            .command(WorldCommand::ExpandRegion {
                expected_revision: 0,
                expansion: RegionExpansion {
                    origin_location_id: "room".into(),
                    origin_routes: BTreeMap::from([(
                        "to-ridge".into(),
                        Route {
                            destination_id: "ridge".into(),
                            distance: "2 km".into(),
                            travel_minutes: 20,
                        },
                    )]),
                    locations: vec![Location {
                        id: "ridge".into(),
                        name: "Ridge Village".into(),
                        container_id: None,
                        routes: BTreeMap::from([(
                            "to-room".into(),
                            Route {
                                destination_id: "room".into(),
                                distance: "2 km".into(),
                                travel_minutes: 20,
                            },
                        )]),
                        persistent_features: vec!["assembly hall".into()],
                    }],
                    facts: vec![WorldFact {
                        id: "ridge-admission".into(),
                        statement,
                        scope: FactScope::ProvisionalLocal,
                        evidence_receipt_ids: vec![evidence.id.clone()],
                        discoverable_at_location_ids: BTreeSet::from(["ridge".into()]),
                    }],
                    populations: vec![population],
                    population_profiles: vec![profile],
                    migration_relations: vec![relation],
                    institutions: vec![],
                    institution_profiles: vec![],
                    local_relations: vec![],
                    civic_system: None,
                },
                evidence_receipts: vec![evidence],
                canon_candidates: vec![],
                model_stage_receipts: vec![],
            })
            .await
            .unwrap();
        let CommandResult::Committed { campaign, .. } = result else {
            panic!("expected commit")
        };
        assert_eq!(campaign.gestalt_members["taren"], original_member);
        assert_eq!(campaign.gestalts["refugees"].home_location_id, "room");
        assert_eq!(
            campaign.gestalts["ridge-households"].home_location_id,
            "ridge"
        );
        assert_eq!(
            campaign.agency_relations["migration:refugees:ridge"].kind,
            AgencyRelationKind::Migration
        );
        assert_eq!(
            campaign.resolution_policy.resolution_epoch,
            original_epoch + 1
        );
        assert!(campaign.resolution_cover.is_none());
        assert_eq!(
            crate::resolution::gestalt_migration_destinations(&campaign, "refugees", "room")["ridge-households"],
            "ridge"
        );
    }

    #[tokio::test]
    async fn locality_elaboration_preserves_the_city_and_grounds_a_named_residents_vote() {
        let dir = tempfile::tempdir().unwrap();
        let store = CampaignStore::open(dir.path().join("campaign.cc")).unwrap();
        let kernel = WorldKernel::start(store);
        let seed = campaign();
        let original = seed.locations["room"].clone();
        let mut elaboration = civic_locality_elaboration();
        let verifier_receipt = civic_verifier_receipt(&seed, &mut elaboration.expansion, vec![]);
        kernel
            .command(WorldCommand::CreateCampaign {
                campaign: seed,
                evidence_receipts: vec![],
                model_stage_receipts: vec![],
            })
            .await
            .unwrap();

        let committed = kernel
            .command(WorldCommand::ElaborateLocality {
                expected_revision: 0,
                elaboration,
                evidence_receipts: vec![],
                canon_candidates: vec![],
                model_stage_receipts: vec![verifier_receipt],
            })
            .await
            .unwrap();
        let CommandResult::Committed {
            campaign: elaborated,
            ..
        } = committed
        else {
            panic!("expected committed locality elaboration")
        };
        assert_eq!(elaborated.locations["room"].id, original.id);
        assert_eq!(elaborated.locations["room"].name, original.name);
        assert_eq!(
            elaborated.locations["room"].persistent_features,
            original.persistent_features
        );
        assert_eq!(
            elaborated.locations["civic-quarter"]
                .container_id
                .as_deref(),
            Some("room")
        );
        assert_eq!(elaborated.institutions.len(), 2);
        assert_eq!(elaborated.civic_systems["room"].version, 0);
        assert!(
            !elaborated.civic_systems["room"]
                .semantic_verification_receipt_id
                .is_empty()
        );
        assert_eq!(
            elaborated.agency_relations["relation:mayor-assembly"].kind,
            AgencyRelationKind::Rivalry
        );
        let selection = elaborated.facts["fact:room-selection"].statement.clone();
        assert!(
            elaborated.gestalts["room-residents"]
                .shared_knowledge
                .contains(&selection)
        );

        let named = kernel
            .command(WorldCommand::IndividuateGestaltMember {
                expected_revision: 1,
                individuation: GestaltIndividuation {
                    gestalt_id: "room-residents".into(),
                    expected_gestalt_version: 0,
                    member: GestaltMemberDelta {
                        schema: "ghostlight.gestalt_member_delta.v1".into(),
                        id: "iren-vale".into(),
                        gestalt_id: "room-residents".into(),
                        version: 0,
                        name: "Iren Vale".into(),
                        capability_additions: BTreeSet::new(),
                        capability_removals: BTreeSet::new(),
                        knowledge_additions: BTreeSet::new(),
                        knowledge_removals: BTreeSet::new(),
                        equipment: BTreeSet::new(),
                        conditions: BTreeSet::new(),
                        obligations: BTreeSet::new(),
                        relationships: BTreeMap::new(),
                        goals: vec!["make berth dues transparent".into()],
                        memories: vec![
                            "I voted for Oren Vale because he promised a public berth audit."
                                .into(),
                        ],
                        last_location_id: Some("civic-quarter".into()),
                        materialized_actor_id: None,
                        last_relevant_revision: 0,
                        relevance_lease_until_revision: 0,
                    },
                    location_id: "civic-quarter".into(),
                },
            })
            .await
            .unwrap();
        let CommandResult::Committed {
            campaign: individuated,
            ..
        } = named
        else {
            panic!("expected committed resident individuation")
        };
        let resident = &individuated.actors["member:iren-vale"];
        assert!(resident.knowledge.contains(&selection));
        assert!(resident.memories[0].contains("Oren Vale"));
    }

    #[tokio::test]
    async fn elaboration_admission_keeps_the_first_writer_and_exposes_the_conflict() {
        use crate::elaboration::{
            ElaboratorTitle, WorldElaborationOperation, admit_world_elaboration_wave,
        };
        let seed = campaign();
        let first = WorldElaborationOperation::AddPlace {
            id: "north-gate".into(),
            name: "Northern Gate".into(),
            container_id: Some("room".into()),
            persistent_features: vec!["A duck statue locals call Harold".into()],
        };
        let second = WorldElaborationOperation::AddPlace {
            id: "north-gate".into(),
            name: "North Tollhouse".into(),
            container_id: Some("room".into()),
            persistent_features: vec!["A toll bell".into()],
        };
        let run = dispatch_titled_operations(
            &seed,
            "room",
            vec![
                (ElaboratorTitle::Patina, first),
                (ElaboratorTitle::Ledger, second),
            ],
        )
        .await;

        let admission = admit_world_elaboration_wave(&seed, "room", run).unwrap();

        assert_eq!(admission.accepted_operations().len(), 1);
        assert_eq!(admission.rejections().len(), 1);
        assert_eq!(
            admission.rejections()[0].kind,
            crate::elaboration::WorldElaborationRejectionKind::WriteConflict
        );
        assert_eq!(
            admission.rejections()[0].conflicting_dispatch_ordinal,
            Some(1)
        );
        assert!(admission.candidate_diagnostic().is_some());
    }

    #[tokio::test]
    async fn titled_patina_detail_reaches_canonical_state_only_through_kernel_lowering() {
        use crate::elaboration::{admit_world_elaboration_wave, finalize_world_elaboration};
        let dir = tempfile::tempdir().unwrap();
        let store = CampaignStore::open(dir.path().join("campaign.cc")).unwrap();
        let kernel = WorldKernel::start(store.clone());
        let seed = campaign();
        let mut expected = civic_locality_elaboration();
        expected.expansion.locations[0]
            .persistent_features
            .push("A weathered bronze duck statue locals call Harold".into());
        let operations = titled_operations_for(&expected);
        let expected_model_receipt_count = operations.len() + 1;
        let run = dispatch_titled_operations(&seed, "room", operations).await;
        let admission = admit_world_elaboration_wave(&seed, "room", run).unwrap();
        assert!(admission.rejections().is_empty());
        assert_eq!(admission.candidate(), Some(&expected));
        assert!(admission.accepted_operations().iter().any(|accepted| {
            accepted.dispatch.title == crate::elaboration::ElaboratorTitle::Patina
                && matches!(
                    &accepted.operation,
                    crate::elaboration::WorldElaborationOperation::AddPlace {
                        persistent_features,
                        ..
                    } if persistent_features.iter().any(|feature| feature.contains("Harold"))
                )
        }));
        let causal_receipt_ids = admission
            .model_stage_receipts()
            .iter()
            .map(|receipt| receipt.storage_key().to_owned())
            .collect::<Vec<_>>();
        let mut verifier_candidate = expected.expansion.clone();
        let missing_ancestry = civic_verifier_receipt(&seed, &mut verifier_candidate, vec![]);
        let error =
            finalize_world_elaboration(&seed, admission.clone(), missing_ancestry).unwrap_err();
        assert!(error.to_string().contains("ancestry"));
        let verifier_receipt =
            civic_verifier_receipt(&seed, &mut verifier_candidate, causal_receipt_ids);
        let finalized = finalize_world_elaboration(&seed, admission, verifier_receipt).unwrap();
        kernel
            .command(WorldCommand::CreateCampaign {
                campaign: seed,
                evidence_receipts: vec![],
                model_stage_receipts: vec![],
            })
            .await
            .unwrap();

        let result = kernel.commit_elaboration(finalized).await.unwrap();
        let CommandResult::Committed { campaign, receipt } = result else {
            panic!("expected committed titled elaboration")
        };

        assert_eq!(receipt.command_kind, "elaborate_locality");
        assert!(
            campaign.locations["civic-quarter"]
                .persistent_features
                .iter()
                .any(|feature| feature.contains("Harold"))
        );
        assert_eq!(store.keys("world_mutation_batch.v1").unwrap().len(), 1);
        assert_eq!(
            store.keys("persona_stage_receipt.v1").unwrap().len(),
            expected_model_receipt_count
        );
        let batch = store
            .load_all::<crate::transition::WorldMutationBatch>("world_mutation_batch.v1")
            .unwrap()
            .pop()
            .unwrap();
        assert!(batch.mutations.iter().any(|permitted| {
            matches!(
                &permitted.mutation,
                crate::transition::WorldMutation::AdmitEntity {
                    initial_profile: Some(crate::transition::AdmittedEntityProfile::Place {
                        persistent_features,
                        ..
                    }),
                    ..
                } if persistent_features.iter().any(|feature| feature.contains("Harold"))
            )
        }));
    }

    #[tokio::test]
    async fn stale_finalized_elaboration_cannot_reach_the_kernel_writer() {
        use crate::elaboration::{admit_world_elaboration_wave, finalize_world_elaboration};
        let dir = tempfile::tempdir().unwrap();
        let store = CampaignStore::open(dir.path().join("campaign.cc")).unwrap();
        let kernel = WorldKernel::start(store.clone());
        let seed = campaign();
        let expected = civic_locality_elaboration();
        let run = dispatch_titled_operations(&seed, "room", titled_operations_for(&expected)).await;
        let admission = admit_world_elaboration_wave(&seed, "room", run).unwrap();
        let causal_receipt_ids = admission
            .model_stage_receipts()
            .iter()
            .map(|receipt| receipt.storage_key().to_owned())
            .collect::<Vec<_>>();
        let mut verifier_candidate = expected.expansion.clone();
        let verifier_receipt =
            civic_verifier_receipt(&seed, &mut verifier_candidate, causal_receipt_ids);
        let finalized = finalize_world_elaboration(&seed, admission, verifier_receipt).unwrap();
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
                minutes: 1,
            })
            .await
            .unwrap();

        let error = kernel.commit_elaboration(finalized).await.unwrap_err();

        assert!(error.to_string().contains("stale"));
        let stored = store
            .load::<Campaign>("campaign.v1", &seed.id.to_string())
            .unwrap()
            .unwrap()
            .1;
        assert_eq!(stored.revision, 1);
        assert_eq!(stored.locations.len(), 1);
        assert_eq!(store.keys("world_mutation_batch.v1").unwrap().len(), 1);
    }

    #[tokio::test]
    async fn invalid_civic_locality_leaves_the_coarse_city_untouched() {
        let dir = tempfile::tempdir().unwrap();
        let store = CampaignStore::open(dir.path().join("campaign.cc")).unwrap();
        let kernel = WorldKernel::start(store.clone());
        let seed = campaign();
        let campaign_id = seed.id;
        kernel
            .command(WorldCommand::CreateCampaign {
                campaign: seed,
                evidence_receipts: vec![],
                model_stage_receipts: vec![],
            })
            .await
            .unwrap();
        let mut elaboration = civic_locality_elaboration();
        elaboration.expansion.populations[0]
            .shared_knowledge
            .remove("Residents elected Selka Vey over Oren Vale at the last mayoral ballot.");

        let error = kernel
            .command(WorldCommand::ElaborateLocality {
                expected_revision: 0,
                elaboration,
                evidence_receipts: vec![],
                canon_candidates: vec![],
                model_stage_receipts: vec![],
            })
            .await
            .unwrap_err();
        assert!(error.to_string().contains("public civic facts"));
        let stored = store
            .load::<Campaign>("campaign.v1", &campaign_id.to_string())
            .unwrap()
            .unwrap()
            .1;
        assert_eq!(stored.revision, 0);
        assert_eq!(stored.locations.len(), 1);
        assert!(stored.institutions.is_empty());
        assert!(stored.gestalts.is_empty());
    }

    #[tokio::test]
    async fn structurally_valid_civic_locality_requires_its_bound_semantic_receipt() {
        let dir = tempfile::tempdir().unwrap();
        let store = CampaignStore::open(dir.path().join("campaign.cc")).unwrap();
        let kernel = WorldKernel::start(store.clone());
        let seed = campaign();
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
            .command(WorldCommand::ElaborateLocality {
                expected_revision: 0,
                elaboration: civic_locality_elaboration(),
                evidence_receipts: vec![],
                canon_candidates: vec![],
                model_stage_receipts: vec![],
            })
            .await
            .unwrap_err();
        assert!(
            error.to_string().contains("semantic verifier receipt"),
            "{error}"
        );
        let stored = store
            .load::<Campaign>("campaign.v1", &campaign_id.to_string())
            .unwrap()
            .unwrap()
            .1;
        assert_eq!(stored.revision, 0);
        assert!(stored.civic_systems.is_empty());
        assert!(stored.institutions.is_empty());
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
                gestalt_reactions: vec![],
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
                gestalt_reactions: vec![],
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
    async fn directly_addressed_local_gestalt_reacts_as_its_exact_collective_subject() {
        let dir = tempfile::tempdir().unwrap();
        let store = CampaignStore::open(dir.path().join("campaign.cc")).unwrap();
        let kernel = WorldKernel::start(store.clone());
        let mut seed = campaign();
        seed.gestalts.insert(
            "households".into(),
            GestaltPersonaState {
                schema: "ghostlight.gestalt_persona_state.v1".into(),
                id: "households".into(),
                name: "Settlement households".into(),
                version: 0,
                home_location_id: "room".into(),
                shared_capabilities: BTreeSet::from(["collective refusal".into()]),
                shared_knowledge: BTreeSet::from(["the slate omits five names".into()]),
                resources: BTreeSet::new(),
                goals: vec!["keep every household traceable".into()],
                pressures: vec!["the convoy slate is closing".into()],
            },
        );
        seed.transcript.push(NarrativeTurn {
            revision: 0,
            at: seed.world_time,
            speaker: "player".into(),
            text: "Households, will you refuse the slate?".into(),
            persona_response_actor_ids: BTreeSet::from(["households".into()]),
        });
        kernel
            .command(WorldCommand::CreateCampaign {
                campaign: seed.clone(),
                evidence_receipts: vec![],
                model_stage_receipts: vec![],
            })
            .await
            .unwrap();

        let missing = kernel
            .command(WorldCommand::ResolveReactionWave {
                expected_revision: 0,
                event_summary: "player says: Households, will you refuse the slate?".into(),
                reactions: vec![],
                gestalt_reactions: vec![],
            })
            .await
            .unwrap_err();
        assert!(missing.to_string().contains("no observable response"));

        let committed = kernel
            .command(WorldCommand::ResolveReactionWave {
                expected_revision: 0,
                event_summary: "player says: Households, will you refuse the slate?".into(),
                reactions: vec![],
                gestalt_reactions: vec![GestaltReaction {
                    gestalt_id: "households".into(),
                    speech: Some(
                        "We will refuse a slate that leaves our names untraceable.".into(),
                    ),
                    deliberate_silence: false,
                }],
            })
            .await
            .unwrap();
        let CommandResult::Committed { campaign, .. } = committed else {
            panic!()
        };
        assert_eq!(campaign.revision, 1);
        assert_eq!(campaign.transcript.last().unwrap().speaker, "households");
        assert_eq!(
            campaign.events.last().unwrap().gestalt_ids,
            vec!["households"]
        );
        assert_eq!(campaign.gestalts["households"], seed.gestalts["households"]);
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
                    gestalt_reactions: vec![],
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
                gestalt_reactions: vec![],
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
                gestalt_reactions: vec![],
            })
            .await;
        assert!(
            matches!(identity_result, Err(KernelError::Invalid(message)) if message.contains("exact spoken handle"))
        );
        let conflicting_identity_result = kernel
            .command(WorldCommand::ResolveReactionWave {
                expected_revision: 0,
                event_summary: "player says: Tell me which seal I repaired.".into(),
                reactions: vec![ActorReaction {
                    actor_id: "anna".into(),
                    speech: Some("My name is Taren.".into()),
                    deliberate_silence: false,
                    private_delta: ActorStateDelta {
                        identity_adoption: Some("Taren".into()),
                        ..Default::default()
                    },
                    action_proposals: vec![],
                }],
                gestalt_reactions: vec![],
            })
            .await;
        assert!(
            matches!(conflicting_identity_result, Err(KernelError::Invalid(message)) if message.contains("established local or population identity"))
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
                gestalt_reactions: vec![],
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
                gestalt_reactions: vec![],
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
                gestalt_reactions: vec![],
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
                gestalt_reactions: vec![],
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
        let hash = test_action_digest(&format!(
            "resolution-stage:{}:{}:{}:{}:{}",
            campaign.id, campaign.revision, cell_id, stage, marker
        ));
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
                .map(|cell| {
                    let decision_owner = cell
                        .detail_focus_subject_id
                        .as_ref()
                        .or_else(|| cell.subject_ids.iter().next())
                        .expect("a simulation cell has at least one subject")
                        .clone();
                    CellAppraisal {
                        schema: "ghostlight.cell_appraisal.v1".into(),
                        cell_id: cell.id.clone(),
                        world_revision: campaign.revision,
                        resolution_epoch: campaign.resolution_policy.resolution_epoch,
                        considered_subject_ids: BTreeSet::from([decision_owner.clone()]),
                        actions: vec![],
                        inactions: vec![crate::domain::CellInaction {
                            subject_id: decision_owner,
                            reason: "No justified move.".into(),
                        }],
                    }
                })
                .collect(),
            cover,
            activity_outcomes: vec![],
            strategic_individuations: vec![],
            model_receipt_hashes: hashes,
        }
    }

    #[test]
    fn direct_strategic_plan_cannot_write_external_subject_state() {
        let external = BTreeSet::from(["external-hold".to_string()]);
        let mut plan = StrategicTickPlan::default();
        plan.institution_actions.push(StrategicInstitutionAction {
            institution_id: "external-hold".into(),
            posture: "overwritten".into(),
            location_ids: Vec::new(),
            public_channels: Vec::new(),
        });
        assert!(strategic_plan_writes_external_subject(&plan, &external));

        plan.institution_actions.clear();
        plan.activity_outcomes.push(StrategicActivityOutcome {
            schema: "ghostlight.strategic_activity_outcome.v1".into(),
            action_digest: format!("sha256:{}", "a".repeat(64)),
            source_subject_id: "foreign-court".into(),
            band: StrategicOutcomeBand::Success,
            summary: "Attempted cross-boundary resource mutation.".into(),
            supporting_state_references: Vec::new(),
            effect: StrategicOutcomeEffect::ResourceConsumed {
                owner_subject_id: "external-hold".into(),
                resource: "ore".into(),
            },
        });
        assert!(strategic_plan_writes_external_subject(&plan, &external));

        plan.activity_outcomes.clear();
        plan.selected_actions.push(CellActionProposal {
            subject_id: "foreign-court".into(),
            intent: "Contact the hold.".into(),
            intended_effect: "Request negotiation.".into(),
            priority: 1,
            state_references: Vec::new(),
            public_channels: Vec::new(),
            effects: vec![StrategicCellEffect::ActorActivity {
                actor_id: "foreign-court".into(),
                activity: StrategicActivityKind::Communicate,
                target_subject_ids: vec!["external-hold".into()],
                location_ids: Vec::new(),
            }],
        });
        assert!(strategic_plan_writes_external_subject(&plan, &external));
    }

    #[test]
    fn strategic_wave_derives_attributed_external_proposal() {
        let dir = tempfile::tempdir().unwrap();
        let store = CampaignStore::open(dir.path().join("campaign.cc")).unwrap();
        let campaign = crate::resolution::tests::campaign(2, 2);
        let mut wave = inaction_wave(&campaign, &store);
        let appraisal = wave.appraisals.first_mut().unwrap();
        let source_subject_id = appraisal.inactions[0].subject_id.clone();
        appraisal.inactions.clear();
        appraisal.actions.push(CellActionProposal {
            subject_id: source_subject_id.clone(),
            intent: "Open talks with the external hold.".into(),
            intended_effect: "Establish a negotiating channel.".into(),
            priority: 1,
            state_references: Vec::new(),
            public_channels: vec!["diplomatic".into()],
            effects: vec![StrategicCellEffect::ActorActivity {
                actor_id: source_subject_id.clone(),
                activity: StrategicActivityKind::Communicate,
                target_subject_ids: vec!["external-hold".into()],
                location_ids: Vec::new(),
            }],
        });
        let authorities = vec![crate::consumer::ExternalSubjectAuthority {
            schema: "ghostlight.external_subject_authority.v1".into(),
            id: "authority:external-hold".into(),
            campaign_id: campaign.id,
            subject_id: "external-hold".into(),
            subject_kind: AgencySubjectKind::Institution,
            owner_id: "fixture-consumer".into(),
            authority_key_sha256: crate::consumer::authority_key_digest("secret"),
            last_source_revision: None,
            last_payload_digest: None,
        }];
        let proposals =
            external_proposals_for_wave(campaign.id, 1, Utc::now(), &wave, &authorities).unwrap();
        assert_eq!(proposals.len(), 1);
        assert_eq!(proposals[0].source_subject_id, source_subject_id);
        assert_eq!(proposals[0].external_subject_id, "external-hold");
        assert_eq!(proposals[0].authority_id, "authority:external-hold");
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
    async fn kernel_rejects_a_forged_nemesis_agenda_and_commits_the_exact_receipted_one() {
        let dir = tempfile::tempdir().unwrap();
        let store = CampaignStore::open(dir.path().join("campaign.cc")).unwrap();
        let kernel = WorldKernel::start(store.clone());
        let mut seed = crate::resolution::tests::campaign(1, 1);
        let responder = "faction-0000";
        seed.agency_profiles
            .get_mut(responder)
            .unwrap()
            .information_channels
            .insert("public-court".into());
        seed.events.push(crate::domain::Event {
            id: "court-accusation".into(),
            at: seed.world_time,
            kind: "public_accusation".into(),
            summary: "The court accuses Faction 0 of concealing the winter tally.".into(),
            actor_ids: vec![],
            institution_ids: vec![],
            gestalt_ids: vec![],
            location_ids: vec![],
            public_channels: vec!["public-court".into()],
        });
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
        wave.cover.causal_follow_through = vec![crate::domain::CausalFollowThroughAssignment {
            anchor_reference: "event:court-accusation".into(),
            responder_subject_id: responder.into(),
        }];

        let error = kernel
            .command(WorldCommand::AdvanceStrategicTick {
                expected_revision: 0,
                source: TickSource::Scheduler,
                plan: None,
                model_receipt_hash: Some(format!("sha256:{}", "9".repeat(64))),
                resolution_wave: Some(wave.clone()),
            })
            .await
            .unwrap_err();
        assert!(error.to_string().contains("exact Nemesis receipt"));
        let unchanged = store
            .load::<Campaign>("campaign.v1", &seed.id.to_string())
            .unwrap()
            .unwrap()
            .1;
        assert_eq!(unchanged.revision, 0);
        assert!(unchanged.nemesis_attention_history.is_empty());

        let mut receipt = resolution_stage(
            &wave.cover.cells[0].id,
            &persisted,
            crate::follow_through::NEMESIS_STAGE,
            'z',
        );
        receipt.snapshot_binding =
            crate::follow_through::nemesis_admission_binding(&persisted, &wave.cover).unwrap();
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
        assert_eq!(campaign.nemesis_attention_history.len(), 1);
        assert_eq!(
            campaign.nemesis_attention_history[0].anchor_reference,
            "event:court-accusation"
        );
        assert_eq!(
            campaign.nemesis_attention_history[0].responder_subject_id,
            responder
        );
        assert_eq!(
            campaign.nemesis_attention_history[0].served_world_revision,
            1
        );
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

    #[test]
    fn activity_outcome_receipts_bind_each_selected_action_independently() {
        let value = hierarchical_refugee_campaign();
        let first = format!("sha256:{}", "1".repeat(64));
        let second = format!("sha256:{}", "2".repeat(64));
        let bindings = expected_activity_outcome_bindings(&value, &[first.clone(), second.clone()]);

        assert_eq!(bindings.len(), 2);
        assert_eq!(
            bindings[0],
            crate::outcome::activity_outcome_binding(
                value.id,
                value.revision,
                value.resolution_policy.resolution_epoch,
                std::slice::from_ref(&first),
            )
        );
        assert_eq!(
            bindings[1],
            crate::outcome::activity_outcome_binding(
                value.id,
                value.revision,
                value.resolution_policy.resolution_epoch,
                std::slice::from_ref(&second),
            )
        );
        assert!(
            !bindings.contains(&crate::outcome::activity_outcome_binding(
                value.id,
                value.revision,
                value.resolution_policy.resolution_epoch,
                &[first, second],
            ))
        );
    }

    #[tokio::test]
    async fn activity_outcome_receipt_must_bind_the_exact_selected_action() {
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
        let refugees_cell_id = wave
            .cover
            .cells
            .iter()
            .find(|cell| cell.subject_ids.contains("refugees-east"))
            .map(|cell| cell.id.clone())
            .unwrap();
        let appraisal = wave
            .appraisals
            .iter_mut()
            .find(|appraisal| appraisal.cell_id == refugees_cell_id)
            .unwrap();
        appraisal.considered_subject_ids = BTreeSet::from(["refugees-east".into()]);
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
        let outcome_binding = crate::outcome::activity_outcome_binding(
            persisted.id,
            persisted.revision,
            persisted.resolution_policy.resolution_epoch,
            &[action_digest],
        );
        let mut rejected_luna = resolution_stage(
            &appraisal_cell_id,
            &persisted,
            "strategic_outcome_resolver",
            '9',
        );
        rejected_luna.snapshot_binding = outcome_binding.clone();
        rejected_luna.model = crate::model::MODEL_FAST.into();
        rejected_luna.validation_result = "semantic_invalid".into();
        rejected_luna.local_validation_error = Some("fixture semantic mismatch".into());
        let mut admitted_terra = resolution_stage(
            &appraisal_cell_id,
            &persisted,
            "strategic_outcome_resolver",
            'a',
        );
        admitted_terra.snapshot_binding = outcome_binding;
        admitted_terra.model = crate::model::MODEL_BALANCED.into();
        admitted_terra.source_receipt_ids = vec![rejected_luna.storage_key().to_owned()];
        let mut outcome_verifier = resolution_stage(
            &appraisal_cell_id,
            &persisted,
            "strategic_outcome_verifier",
            'b',
        );
        outcome_verifier.snapshot_binding = crate::outcome::activity_outcome_verification_binding(
            persisted.id,
            persisted.revision,
            persisted.resolution_policy.resolution_epoch,
            &wave.activity_outcomes,
        )
        .unwrap();
        outcome_verifier.source_receipt_ids = vec![admitted_terra.storage_key().to_owned()];
        for receipt in [&rejected_luna, &admitted_terra, &outcome_verifier] {
            store
                .insert(
                    "persona_stage_receipt.v1",
                    "ghostlight.persona_stage_receipt.v1",
                    receipt.storage_key(),
                    receipt,
                )
                .unwrap();
        }
        let mut missing_verifier_wave = wave.clone();
        missing_verifier_wave.model_receipt_hashes.extend([
            rejected_luna.storage_key().to_owned(),
            admitted_terra.storage_key().to_owned(),
        ]);
        let mut correct_wave = missing_verifier_wave.clone();
        correct_wave
            .model_receipt_hashes
            .push(outcome_verifier.storage_key().to_owned());

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

        let error = kernel
            .command(WorldCommand::AdvanceStrategicTick {
                expected_revision: 0,
                source: TickSource::Scheduler,
                plan: None,
                model_receipt_hash: Some(format!("sha256:{}", "a".repeat(64))),
                resolution_wave: Some(missing_verifier_wave),
            })
            .await
            .unwrap_err();
        assert!(
            error
                .to_string()
                .contains("lacks an outcome-bound strategic verifier receipt"),
            "unexpected kernel rejection: {error}"
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
    async fn strategic_wave_atomically_individuates_and_materializes_an_action_bound_person() {
        let dir = tempfile::tempdir().unwrap();
        let store = CampaignStore::open(dir.path().join("campaign.cc")).unwrap();
        let kernel = WorldKernel::start(store.clone());
        let mut seed = hierarchical_refugee_campaign();
        seed.agency_profiles
            .get_mut("refugees-east")
            .unwrap()
            .information_channels
            .insert("storm-camp broadsheet".into());
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
        let cell_id = wave
            .cover
            .cells
            .iter()
            .find(|cell| cell.subject_ids.contains("refugees-east"))
            .unwrap()
            .id
            .clone();
        let appraisal = wave
            .appraisals
            .iter_mut()
            .find(|appraisal| appraisal.cell_id == cell_id)
            .unwrap();
        appraisal.considered_subject_ids = BTreeSet::from(["refugees-east".into()]);
        let action = CellActionProposal {
            subject_id: "refugees-east".into(),
            intent: "appoint a named storm delegation broker".into(),
            intended_effect: "make one accuser answer for the camp negotiation".into(),
            priority: 80,
            state_references: vec!["subject:refugees-east".into(), "location:camp".into()],
            public_channels: vec!["storm-camp broadsheet".into()],
            effects: vec![StrategicCellEffect::Gestalt {
                gestalt_id: "refugees-east".into(),
                pressure_additions: vec![
                    "Veska Rill says the east quartermasters sold the lower road twice".into(),
                ],
                pressure_resolutions: vec![],
            }],
        };
        let action_digest = crate::resolution::cell_action_digest(&action).unwrap();
        appraisal.actions = vec![action.clone()];
        appraisal.inactions.clear();
        wave.strategic_individuations = vec![StrategicGestaltIndividuation {
            schema: "ghostlight.strategic_gestalt_individuation.v1".into(),
            action_digest: action_digest.clone(),
            rationale:
                "She accused the east quartermasters of selling the broken lower road twice.".into(),
            individuation: GestaltIndividuation {
                gestalt_id: "refugees-east".into(),
                expected_gestalt_version: 0,
                location_id: "camp".into(),
                member: GestaltMemberDelta {
                    schema: "ghostlight.gestalt_member_delta.v1".into(),
                    id: "veska-rill".into(),
                    gestalt_id: "refugees-east".into(),
                    version: 0,
                    name: "Veska Rill".into(),
                    capability_additions: BTreeSet::new(),
                    capability_removals: BTreeSet::new(),
                    knowledge_additions: BTreeSet::new(),
                    knowledge_removals: BTreeSet::new(),
                    equipment: BTreeSet::new(),
                    conditions: BTreeSet::new(),
                    obligations: BTreeSet::from(["answer to the camp wards".into()]),
                    relationships: BTreeMap::new(),
                    goals: vec!["secure the storm route".into()],
                    memories: vec!["the deep excavation broke the lower road".into()],
                    last_location_id: Some("camp".into()),
                    materialized_actor_id: None,
                    last_relevant_revision: persisted.revision,
                    relevance_lease_until_revision: persisted.revision + 4,
                },
            },
        }];
        let base_binding = format!(
            "campaign:{}:revision:{}:resolution:{}:cell:{}",
            persisted.id, persisted.revision, persisted.resolution_policy.resolution_epoch, cell_id
        );
        let mut verifier = resolution_stage(&cell_id, &persisted, "cell_effect_verifier", '7');
        verifier.snapshot_binding = crate::persona::cell_effect_verification_binding(
            &base_binding,
            std::slice::from_ref(&action),
        )
        .unwrap();
        let mut selector = resolution_stage(
            &cell_id,
            &persisted,
            "strategic_individuation_selector",
            '8',
        );
        selector.rebind_snapshot(crate::scheduler::strategic_individuation_binding(
            &persisted,
            std::slice::from_ref(&action_digest),
            Some(
                &crate::scheduler::strategic_individuation_proposal_digest(
                    &wave.strategic_individuations[0],
                )
                .unwrap(),
            ),
        ));
        for receipt in [verifier, selector] {
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
        let mut substituted = wave.clone();
        substituted.strategic_individuations[0]
            .individuation
            .member
            .name = "Substituted Caller Payload".into();
        let error = kernel
            .command(WorldCommand::AdvanceStrategicTick {
                expected_revision: persisted.revision,
                source: TickSource::Scheduler,
                plan: None,
                model_receipt_hash: Some(format!("sha256:{}", "c".repeat(64))),
                resolution_wave: Some(substituted),
            })
            .await
            .unwrap_err();
        assert!(
            error.to_string().contains("payload-bound selector receipt"),
            "{error}"
        );
        let unchanged = store
            .load::<Campaign>("campaign.v1", &seed.id.to_string())
            .unwrap()
            .unwrap()
            .1;
        assert_eq!(unchanged.revision, persisted.revision);
        assert!(!unchanged.gestalt_members.contains_key("veska-rill"));
        let result = kernel
            .command(WorldCommand::AdvanceStrategicTick {
                expected_revision: persisted.revision,
                source: TickSource::Scheduler,
                plan: None,
                model_receipt_hash: Some(format!("sha256:{}", "b".repeat(64))),
                resolution_wave: Some(wave),
            })
            .await
            .unwrap();
        let CommandResult::Committed { campaign, .. } = result else {
            panic!()
        };
        let member = &campaign.gestalt_members["veska-rill"];
        assert_eq!(
            member.materialized_actor_id.as_deref(),
            Some("member:veska-rill")
        );
        assert_eq!(campaign.actors["member:veska-rill"].name, "Veska Rill");
        assert!(campaign.agency_profiles["member:veska-rill"].simulation_eligible);
        assert!(
            campaign
                .events
                .iter()
                .any(|event| event.kind == "gestalt_individuation")
        );
        let public_individuation = campaign
            .events
            .iter()
            .find(|event| {
                event.kind == "gestalt_individuation"
                    && event
                        .public_channels
                        .iter()
                        .any(|channel| channel == "storm-camp broadsheet")
            })
            .unwrap();
        let public_issue = campaign
            .news
            .iter()
            .find(|issue| issue.event_ids == [public_individuation.id.clone()])
            .unwrap();
        assert_eq!(
            public_individuation.summary,
            "Veska Rill steps forward within Eastern transit refugees to secure the storm route."
        );
        assert!(!public_individuation.summary.contains("selected action"));
        assert_eq!(
            public_issue.headline,
            crate::domain::committed_news_headline(&public_individuation.summary)
        );
        assert!(public_issue.headline.contains("Veska Rill"));
        let proposals = store
            .load_all::<StrategicGestaltIndividuation>("strategic_gestalt_individuation.v1")
            .unwrap();
        assert_eq!(proposals.len(), 1);
        assert_eq!(proposals[0].action_digest, action_digest);

        let mut second_wave = inaction_wave(&campaign, &store);
        let second_cell_id = second_wave
            .cover
            .cells
            .iter()
            .find(|cell| cell.subject_ids.contains("member:veska-rill"))
            .unwrap()
            .id
            .clone();
        let second_appraisal = second_wave
            .appraisals
            .iter_mut()
            .find(|appraisal| appraisal.cell_id == second_cell_id)
            .unwrap();
        second_appraisal.considered_subject_ids = BTreeSet::from(["member:veska-rill".into()]);
        let second_action = CellActionProposal {
            subject_id: "member:veska-rill".into(),
            intent: "cross the bay to confront the quartermasters".into(),
            intended_effect: "arrive at the south docks without resolving the accusation".into(),
            priority: 91,
            state_references: vec![],
            public_channels: vec![],
            effects: vec![StrategicCellEffect::ActorMove {
                actor_id: "member:veska-rill".into(),
                destination_id: "docks".into(),
            }],
        };
        second_appraisal.actions = vec![second_action.clone()];
        second_appraisal.inactions.clear();
        let second_base_binding = format!(
            "campaign:{}:revision:{}:resolution:{}:cell:{}",
            campaign.id,
            campaign.revision,
            campaign.resolution_policy.resolution_epoch,
            second_cell_id
        );
        let mut second_verifier =
            resolution_stage(&second_cell_id, &campaign, "cell_effect_verifier", 'v');
        second_verifier.snapshot_binding = crate::persona::cell_effect_verification_binding(
            &second_base_binding,
            std::slice::from_ref(&second_action),
        )
        .unwrap();
        store
            .insert(
                "persona_stage_receipt.v1",
                "ghostlight.persona_stage_receipt.v1",
                second_verifier.storage_key(),
                &second_verifier,
            )
            .unwrap();
        second_wave
            .model_receipt_hashes
            .push(second_verifier.storage_key().to_owned());
        let result = kernel
            .command(WorldCommand::AdvanceStrategicTick {
                expected_revision: campaign.revision,
                source: TickSource::Scheduler,
                plan: None,
                model_receipt_hash: Some(test_action_digest("second-veska-wave")),
                resolution_wave: Some(second_wave),
            })
            .await
            .unwrap();
        let CommandResult::Committed {
            campaign: campaign_after_second_wave,
            ..
        } = result
        else {
            panic!()
        };
        assert_eq!(
            campaign_after_second_wave.actors["member:veska-rill"].location_id,
            "docks"
        );
        assert!(campaign_after_second_wave.events.iter().any(|event| {
            event.kind == "actor_movement" && event.actor_ids.contains(&"member:veska-rill".into())
        }));
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
        let travel_turn = campaign.transcript.last().unwrap();
        assert_eq!(travel_turn.speaker, "world");
        assert!(travel_turn.text.contains("Room"));
        assert!(travel_turn.text.contains("Harbor"));
        assert!(travel_turn.text.contains("20 minutes pass"));
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
