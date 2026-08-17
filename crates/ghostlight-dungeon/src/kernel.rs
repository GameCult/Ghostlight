use crate::d20::{capped_modifier, receipt};
use crate::domain::*;
use crate::persistence::CampaignStore;
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
                .get(&assessment_digest)
                .cloned()
                .ok_or(KernelError::UnknownAssessment)?;
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
            apply_world_effect(&mut campaign, &effect)?;
            refresh_materialized_member_relevance(
                &mut campaign,
                std::iter::once(assessment.intent.actor_id.as_str()),
            );
            if assessment.intent.actor_id == campaign.player_actor_id {
                campaign.last_player_activity = Utc::now();
                campaign.away_ticks_processed = 0;
                campaign.pending_ticks = 0;
            }
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
                speaker: actor_id.clone(),
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
            refresh_materialized_member_relevance(
                &mut campaign,
                std::iter::once(actor_id.as_str()),
            );
            if actor_id == campaign.player_actor_id {
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
            campaign.world_time += Duration::minutes(i64::from(minutes));
            campaign.last_player_activity = Utc::now();
            campaign.away_ticks_processed = 0;
            campaign.pending_ticks = 0;
            commit(store, row, campaign, "wait", None)
        }
        WorldCommand::SetResolutionBudget {
            expected_revision,
            expected_resolution_epoch,
            active_cell_budget,
        } => {
            require_revision(&campaign, expected_revision)?;
            require_resolution_epoch(&campaign, expected_resolution_epoch)?;
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
            crate::resolution::apply_fission(&mut campaign, &preview)
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
                if unique_hashes.len() != wave.model_receipt_hashes.len()
                    || wave.model_receipt_hashes.len() < wave.cover.cells.len().saturating_mul(3)
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
                    stage_bindings
                        .insert((receipt.stage.clone(), receipt.snapshot_binding.clone()));
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
                }
            }
            campaign.world_time += Duration::hours(i64::from(campaign.tick_hours));
            for clock in campaign.clocks.values_mut() {
                clock.progress = clock.progress.saturating_add(1).min(clock.threshold);
            }
            let tick_number = campaign.away_ticks_processed.saturating_add(1);
            let tick_events = match resolved_plan.or(plan) {
                Some(plan) => apply_strategic_tick_plan(&mut campaign, plan)?,
                None => deterministic_strategic_tick(&mut campaign, tick_number),
            };
            if let Some(wave) = &resolution_wave {
                crate::resolution::advance_detail_debt(&mut campaign, &wave.cover);
                campaign.resolution_cover = Some(wave.cover.clone());
            }
            campaign.strategic_tick_count = campaign.strategic_tick_count.saturating_add(1);
            let player = campaign.actors.get(&campaign.player_actor_id);
            for event in &tick_events {
                let accessible = event
                    .public_channels
                    .iter()
                    .find(|channel| player.is_some_and(|actor| actor.knowledge.contains(*channel)));
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
            let event_ids = tick_events.iter().map(|event| event.id.clone()).collect();
            campaign.events.extend(tick_events);
            campaign.away_ticks_processed = tick_number.min(8);
            campaign.pending_ticks = campaign.pending_ticks.saturating_sub(1);
            commit_strategic_tick(
                store,
                row,
                campaign,
                source,
                model_receipt_hash,
                event_ids,
                resolution_wave,
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
                model_stage_receipts,
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
            aggregate_delta,
        } => {
            require_revision(&campaign, expected_revision)?;
            let before = campaign.clone();
            apply_demotion(
                &mut campaign,
                &GestaltDemotion {
                    actor_id,
                    aggregate_delta,
                },
            )?;
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
            refresh_materialized_member_relevance(&mut campaign, seen.iter().map(String::as_str));
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
        WorldCommand::ResolveNpcAction {
            expected_revision,
            proposal,
            assessment,
        } => {
            require_revision(&campaign, expected_revision)?;
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
                apply_world_effect(&mut campaign, effect)?;
                campaign.transcript.push(NarrativeTurn {
                    revision: campaign.revision + 1,
                    at: Utc::now(),
                    speaker: "world".into(),
                    text: text.clone(),
                });
                Some(roll)
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
                location_ids: vec![actor_location],
                public_channels: vec![],
            });
            refresh_materialized_member_relevance(
                &mut campaign,
                std::iter::once(proposal.actor_id.as_str()),
            );
            commit(store, row, campaign, "resolve_npc_action", roll)
        }
        WorldCommand::CreateCampaign { .. } => unreachable!(),
    }
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
    let gestalt = campaign
        .gestalts
        .get(&promotion.gestalt_id)
        .ok_or_else(|| KernelError::Invalid("gestalt is unknown".into()))?
        .clone();
    if gestalt.version != promotion.expected_gestalt_version {
        return Err(KernelError::Invalid("gestalt snapshot is stale".into()));
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

fn apply_world_effect(
    campaign: &mut Campaign,
    effect: &WorldEffectDelta,
) -> Result<(), KernelError> {
    for (actor_id, delta) in &effect.actor_conditions {
        let actor = campaign
            .actors
            .get_mut(actor_id)
            .ok_or_else(|| KernelError::Invalid("outcome actor vanished".into()))?;
        actor.conditions.extend(delta.add.clone());
        for value in &delta.remove {
            actor.conditions.remove(value);
        }
    }
    for (actor_id, additions) in &effect.actor_knowledge_additions {
        let actor = campaign
            .actors
            .get_mut(actor_id)
            .ok_or_else(|| KernelError::Invalid("outcome actor vanished".into()))?;
        actor.knowledge.extend(additions.clone());
    }
    for (actor_id, relationships) in &effect.actor_relationship_updates {
        campaign
            .actors
            .get_mut(actor_id)
            .ok_or_else(|| KernelError::Invalid("outcome actor vanished".into()))?
            .relationships
            .extend(relationships.clone());
    }
    for (actor_id, destination) in &effect.actor_moves {
        campaign
            .actors
            .get_mut(actor_id)
            .ok_or_else(|| KernelError::Invalid("outcome actor vanished".into()))?
            .location_id = destination.clone();
    }
    for (clock_id, amount) in &effect.clock_advances {
        let clock = campaign
            .clocks
            .get_mut(clock_id)
            .ok_or_else(|| KernelError::Invalid("outcome clock vanished".into()))?;
        clock.progress = clock.progress.saturating_add(*amount).min(clock.threshold);
    }
    for (institution_id, posture) in &effect.institution_postures {
        campaign
            .institutions
            .get_mut(institution_id)
            .ok_or_else(|| KernelError::Invalid("outcome institution vanished".into()))?
            .posture = posture.clone();
    }
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
        .get_mut(&gestalt_id)
        .ok_or_else(|| KernelError::Invalid("gestalt is missing".into()))?;
    let member = campaign
        .gestalt_members
        .get_mut(&member_id)
        .expect("member exists");
    fold_actor_delta(&actor, gestalt, member);
    member.materialized_actor_id = None;
    member.version += 1;
    if !demotion.aggregate_delta.knowledge_additions.is_empty()
        || !demotion.aggregate_delta.resource_additions.is_empty()
        || !demotion.aggregate_delta.pressures.is_empty()
    {
        gestalt
            .shared_knowledge
            .extend(demotion.aggregate_delta.knowledge_additions.clone());
        gestalt
            .resources
            .extend(demotion.aggregate_delta.resource_additions.clone());
        gestalt
            .pressures
            .extend(demotion.aggregate_delta.pressures.clone());
        gestalt.version += 1;
    }
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

fn apply_strategic_tick_plan(
    campaign: &mut Campaign,
    plan: crate::domain::StrategicTickPlan,
) -> Result<Vec<crate::domain::Event>, KernelError> {
    let revision = campaign.revision + 1;
    let at = campaign.world_time;
    let mut events = Vec::new();
    let mut seen_institutions = BTreeSet::new();
    for action in plan.institution_actions {
        if !seen_institutions.insert(action.institution_id.clone()) {
            return Err(KernelError::Invalid(
                "institution acts twice in one strategic tick".into(),
            ));
        }
        if action.summary.trim().is_empty() || action.posture.trim().is_empty() {
            return Err(KernelError::Invalid(
                "strategic institution action is empty".into(),
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
            .get_mut(&action.institution_id)
            .ok_or_else(|| KernelError::Invalid("strategic plan invented an institution".into()))?;
        institution.posture = action.posture;
        events.push(crate::domain::Event {
            id: format!("strategic:{revision}:institution:{}", institution.id),
            at,
            kind: "institution_action".into(),
            summary: action.summary,
            actor_ids: vec![],
            institution_ids: vec![institution.id.clone()],
            location_ids: action.location_ids,
            public_channels: action.public_channels,
        });
    }

    let mut seen_gestalts = BTreeSet::new();
    for action in plan.gestalt_actions {
        if !seen_gestalts.insert(action.gestalt_id.clone()) {
            return Err(KernelError::Invalid(
                "gestalt acts twice in one strategic tick".into(),
            ));
        }
        if action.summary.trim().is_empty()
            || action.pressure_additions.len() > 4
            || action
                .pressure_additions
                .iter()
                .any(|p| p.trim().is_empty() || p.len() > 240)
        {
            return Err(KernelError::Invalid(
                "strategic gestalt action is invalid".into(),
            ));
        }
        validate_public_channels(&action.public_channels)?;
        let gestalt = campaign
            .gestalts
            .get_mut(&action.gestalt_id)
            .ok_or_else(|| KernelError::Invalid("strategic plan invented a gestalt".into()))?;
        for pressure in action.pressure_additions {
            if !gestalt.pressures.contains(&pressure) {
                gestalt.pressures.push(pressure);
            }
        }
        gestalt.version += 1;
        events.push(crate::domain::Event {
            id: format!("strategic:{revision}:gestalt:{}", gestalt.id),
            at,
            kind: "gestalt_action".into(),
            summary: action.summary,
            actor_ids: vec![],
            institution_ids: vec![],
            location_ids: vec![gestalt.home_location_id.clone()],
            public_channels: action.public_channels,
        });
    }

    let mut seen_actors = BTreeSet::new();
    for action in plan.actor_moves {
        if !seen_actors.insert(action.actor_id.clone()) {
            return Err(KernelError::Invalid(
                "actor moves twice in one strategic tick".into(),
            ));
        }
        if action.actor_id == campaign.player_actor_id {
            return Err(KernelError::Invalid(
                "strategic simulation cannot puppet the player".into(),
            ));
        }
        if action.summary.trim().is_empty() {
            return Err(KernelError::Invalid(
                "strategic actor movement has no summary".into(),
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
        campaign
            .actors
            .get_mut(&action.actor_id)
            .expect("actor was validated")
            .location_id = action.destination_id.clone();
        events.push(crate::domain::Event {
            id: format!("strategic:{revision}:actor:{}", action.actor_id),
            at,
            kind: "actor_movement".into(),
            summary: action.summary,
            actor_ids: vec![action.actor_id],
            institution_ids: vec![],
            location_ids: vec![origin, action.destination_id],
            public_channels: action.public_channels,
        });
    }
    Ok(events)
}

fn deterministic_strategic_tick(
    campaign: &mut Campaign,
    tick_number: u8,
) -> Vec<crate::domain::Event> {
    let mut events = Vec::new();
    for institution in campaign.institutions.values_mut() {
        let summary = institution
            .goals
            .first()
            .map(|goal| format!("{} advances its interest: {}", institution.name, goal))
            .unwrap_or_else(|| format!("{} consolidates its current position", institution.name));
        institution.posture = format!("acting after strategic tick {tick_number}");
        events.push(crate::domain::Event {
            id: format!("strategic:{}:{}", campaign.revision + 1, institution.id),
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
        let summary = gestalt
            .goals
            .first()
            .map(|goal| format!("{} collectively advances: {}", gestalt.name, goal))
            .unwrap_or_else(|| format!("{} carries on its shared routine", gestalt.name));
        events.push(crate::domain::Event {
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
    events
}

fn validate_public_channels(channels: &[String]) -> Result<(), KernelError> {
    if channels.len() > 8
        || channels
            .iter()
            .any(|c| c.trim().is_empty() || c.len() > 160)
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

fn commit_with_records(
    store: &CampaignStore,
    row: cultcache_legacy::CultCacheEnvelope,
    mut campaign: Campaign,
    kind: &str,
    evidence: Vec<VaultEvidenceReceipt>,
    candidates: Vec<CanonCandidate>,
    model_receipts: Vec<crate::model::ModelStageReceipt>,
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
            })
            .await
            .unwrap();
        let error = kernel
            .command(WorldCommand::Attempt {
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
        assert_eq!(campaign.events.len(), 1);
        assert!(campaign.news.is_empty());
        assert_eq!(campaign.away_ticks_processed, 1);
        assert!(campaign.institutions["board"].posture.contains("acting"));
        let ticks = store
            .load_all::<crate::domain::StrategicTickReceipt>("strategic_tick.v1")
            .unwrap();
        assert_eq!(ticks.len(), 1);
        assert_eq!(ticks[0].source, TickSource::Scheduler);
        assert_eq!(ticks[0].event_ids, vec![campaign.events[0].id.clone()]);
        assert!(ticks[0].model_receipt_hash.is_none());
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
                summary: "The absent player obeys.".into(),
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
                summary: "The runner carries the warning to the yard.".into(),
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
                    locations: vec![location],
                    facts: vec![],
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
        assert_eq!(campaign.revision, 1);
        assert_eq!(store.keys("vault_evidence_receipt.v1").unwrap().len(), 1);
        assert_eq!(store.keys("canon_candidate.v1").unwrap().len(), 1);
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
        assert!(
            kernel
                .command(WorldCommand::DematerializeGestaltMember {
                    expected_revision: 1,
                    actor_id: "member:john".into(),
                    aggregate_delta: Default::default(),
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
                expected_revision: 3,
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

        let bad_plan = GestaltPresencePlan {
            individuations: vec![],
            demotions: vec![GestaltDemotion {
                actor_id: "member:john".into(),
                aggregate_delta: GestaltAggregateDelta::default(),
            }],
            promotions: vec![GestaltPromotion {
                gestalt_id: "village".into(),
                expected_gestalt_version: 1,
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

    #[tokio::test]
    async fn initiative_grants_one_npc_opportunity_without_faking_player_activity() {
        let dir = tempfile::tempdir().unwrap();
        let store = CampaignStore::open(dir.path().join("campaign.cc")).unwrap();
        let kernel = WorldKernel::start(store.clone());
        let mut seed = campaign();
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
                        private_delta: ActorStateDelta::default(),
                        action_proposals: vec![anna.clone()],
                    },
                    ActorReaction {
                        actor_id: "bert".into(),
                        speech: None,
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
                    inaction_reason: Some("No justified move.".into()),
                })
                .collect(),
            cover,
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
}
