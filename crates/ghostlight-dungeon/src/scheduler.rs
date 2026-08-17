use crate::{
    domain::{
        AgencyAxis, AgencySubjectKind, Campaign, ResolutionDemand, ResolutionWaveCommit,
        SimulationCell,
    },
    model::{ModelPort, ModelStageOutput, ModelStageRequest, run_validated_stage},
    persona::{
        CellConstituentSlice, CellMemberSlice, CellPerceivedEventSlice, CellProjectionEngine,
        ExecutionPermit, PermittedCellSlice,
    },
    resolution::{
        cell_action_limit, default_demand, plan_cover, plan_receipt, validate_and_resolve_wave,
        validate_demand,
    },
};
use anyhow::{Context, Result, anyhow};
use chrono::{DateTime, Utc};
use schemars::{JsonSchema, schema_for};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
struct DemandProjection {
    axis_weights: BTreeMap<AgencyAxis, f32>,
    focal_subject_ids: BTreeSet<String>,
    rationale: String,
}

#[derive(Clone, Debug, Default, Serialize)]
struct RelationDemandSummary {
    edges: u64,
    total_strength: u64,
    max_strength: u8,
}

pub struct StrategicResolutionOutput {
    pub wave: ResolutionWaveCommit,
    pub stages: Vec<ModelStageOutput>,
    pub private_cell_traces: Vec<PrivateCellTrace>,
    pub aggregate_receipt_hash: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct PrivateCellTrace {
    pub cell_id: String,
    pub lived_stream: String,
    pub persona_output: String,
}

pub async fn propose_resolution_wave(
    model: Arc<dyn ModelPort>,
    permit: Arc<dyn ExecutionPermit>,
    campaign: &Campaign,
) -> Result<StrategicResolutionOutput> {
    let (demand, mut stages) = project_resolution_demand(model.as_ref(), campaign).await;
    let cover = plan_cover(campaign, demand)?;
    let receipt = plan_receipt(campaign, &cover);
    let engine = CellProjectionEngine {
        model,
        permit,
        projector_model: "deepseek-v4-flash".into(),
        persona_model: "deepseek-v4-flash".into(),
        interpreter_model: "deepseek-v4-flash".into(),
    };
    let semaphore = Arc::new(tokio::sync::Semaphore::new(usize::from(
        campaign.resolution_policy.provider_parallelism.max(1),
    )));
    let mut jobs = tokio::task::JoinSet::new();
    for cell in cover.cells.clone() {
        let engine = engine.clone();
        let semaphore = semaphore.clone();
        let slice = cell_slice(campaign, &cell)?;
        jobs.spawn(async move {
            let _permit = semaphore
                .acquire_owned()
                .await
                .map_err(|_| anyhow!("provider concurrency gate closed"))?;
            let terminal = engine.execute(slice).await?;
            Ok::<_, anyhow::Error>((cell.id, terminal))
        });
    }
    let mut terminals = Vec::new();
    while let Some(result) = jobs.join_next().await {
        terminals.push(result.map_err(|error| anyhow!("cell Persona task failed: {error}"))??);
    }
    terminals.sort_by(|left, right| left.0.cmp(&right.0));
    let appraisals = terminals
        .iter()
        .map(|(_, terminal)| terminal.appraisal.clone())
        .collect();
    let private_cell_traces = terminals
        .iter()
        .map(|(cell_id, terminal)| PrivateCellTrace {
            cell_id: cell_id.clone(),
            lived_stream: terminal.lived_stream.text.clone(),
            persona_output: terminal.persona_output.clone(),
        })
        .collect();
    for (_, terminal) in terminals {
        stages.extend(
            terminal
                .stage_receipts
                .into_iter()
                .map(|receipt| ModelStageOutput {
                    narrative: String::new(),
                    structured: None,
                    receipt,
                }),
        );
    }
    let model_receipt_hashes = stages
        .iter()
        .map(|stage| stage.receipt.storage_key().to_owned())
        .collect::<Vec<_>>();
    let aggregate_receipt_hash = format!(
        "sha256:{:x}",
        Sha256::digest(model_receipt_hashes.join("|").as_bytes())
    );
    let wave = ResolutionWaveCommit {
        schema: "ghostlight.resolution_wave_commit.v1".into(),
        world_revision: campaign.revision,
        resolution_epoch: campaign.resolution_policy.resolution_epoch,
        cover,
        plan_receipt: receipt,
        appraisals,
        model_receipt_hashes,
    };
    validate_and_resolve_wave(campaign, &wave)?;
    Ok(StrategicResolutionOutput {
        wave,
        stages,
        private_cell_traces,
        aggregate_receipt_hash,
    })
}

async fn project_resolution_demand(
    model: &dyn ModelPort,
    campaign: &Campaign,
) -> (ResolutionDemand, Vec<ModelStageOutput>) {
    let fallback = || {
        campaign
            .resolution_cover
            .as_ref()
            .map(|cover| {
                let mut demand = cover.demand.clone();
                demand.world_revision = campaign.revision;
                demand.resolution_epoch = campaign.resolution_policy.resolution_epoch;
                demand.rationale = format!("last accepted demand: {}", demand.rationale);
                demand
            })
            .unwrap_or_else(|| default_demand(campaign, "default geography/authority demand"))
    };
    let (focal_candidate_ids, agency_summary) = resolution_demand_context(campaign);
    let context = serde_json::json!({
        "campaign_id": campaign.id,
        "world_revision": campaign.revision,
        "resolution_epoch": campaign.resolution_policy.resolution_epoch,
        "horizon_minutes": campaign.tick_hours.saturating_mul(60),
        "clocks": campaign.clocks.values().map(|clock| serde_json::json!({
            "id":clock.id,
            "label":clock.label,
            "progress":clock.progress,
            "threshold":clock.threshold,
            "consequence":clock.consequence,
        })).collect::<Vec<_>>(),
        "agency_summary": agency_summary,
    });
    let mut schema = match serde_json::to_value(schema_for!(DemandProjection)) {
        Ok(value) => value,
        Err(_) => return (fallback(), vec![]),
    };
    if let Some(items) = schema.pointer_mut("/properties/focal_subject_ids/items") {
        *items = serde_json::json!({"type":"string","enum":focal_candidate_ids});
    }
    if let Some(focal) = schema
        .pointer_mut("/properties/focal_subject_ids")
        .and_then(serde_json::Value::as_object_mut)
    {
        focal.insert("uniqueItems".into(), true.into());
        focal.insert(
            "maxItems".into(),
            usize::from(campaign.resolution_policy.active_cell_budget)
                .min(focal_candidate_ids.len())
                .into(),
        );
    }
    let base_prompt = format!(
        "OUTPUT JSON SCHEMA (follow exactly):\n{}\n\nProject which boundaries best predict different strategic behavior for the next horizon. Return all six axis weights (geography, ideology, authority, economy_role, species_body, information), each from 0 to 1 and summing to 1. The focal-subject enum contains only candidates that committed state has locally distinguished. Choose only genuinely exceptional candidates and return an empty list when the enum is empty. Focal hints cannot force partition boundaries or budget overage. This stage proposes relevance and cannot change world state.\nSTATE:\n{}",
        serde_json::to_string(&schema).unwrap_or_default(),
        serde_json::to_string(&context).unwrap_or_default(),
    );
    let mut request = ModelStageRequest {
        stage: "resolution_demand".into(),
        model: "deepseek-v4-flash".into(),
        snapshot_binding: format!(
            "campaign:{}:revision:{}:resolution:{}",
            campaign.id, campaign.revision, campaign.resolution_policy.resolution_epoch
        ),
        lived_stream: base_prompt,
        output_schema: Some(schema),
        source_receipt_ids: campaign.branch_origin.evidence_receipt_ids.clone(),
        temperature: Some(0.0),
        max_output_tokens: Some(512),
    };
    let mut outputs = Vec::new();
    for attempt in 0..2 {
        let Ok(mut output) = run_validated_stage(model, &request).await else {
            return (fallback(), outputs);
        };
        let projection = output
            .structured
            .clone()
            .and_then(|value| serde_json::from_value::<DemandProjection>(value).ok());
        if let Some(projection) = projection {
            let demand = ResolutionDemand {
                schema: "ghostlight.resolution_demand.v1".into(),
                campaign_id: campaign.id,
                world_revision: campaign.revision,
                resolution_epoch: campaign.resolution_policy.resolution_epoch,
                axis_weights: projection.axis_weights,
                focal_subject_ids: projection.focal_subject_ids,
                horizon_minutes: campaign.tick_hours.saturating_mul(60),
                rationale: projection.rationale,
            };
            match validate_demand(campaign, &demand) {
                Ok(()) => {
                    outputs.push(output);
                    return (demand, outputs);
                }
                Err(error) => {
                    output.receipt.validation_result = "semantic_invalid".into();
                    output.receipt.local_validation_error =
                        Some(error.to_string().chars().take(1_000).collect());
                }
            }
        } else {
            output.receipt.validation_result = "semantic_invalid".into();
            output.receipt.local_validation_error =
                Some("resolution demand could not be decoded".into());
        }
        outputs.push(output);
        if attempt == 0 {
            request.lived_stream.push_str(
                "\n\nLOCAL VALIDATOR: The prior projection omitted an axis, used an unknown subject, had invalid weights, failed to sum to one, or had no rationale. Return one corrected object against the same snapshot.",
            );
        }
    }
    (fallback(), outputs)
}

fn resolution_demand_context(campaign: &Campaign) -> (Vec<String>, serde_json::Value) {
    let profiles = campaign
        .agency_profiles
        .values()
        .filter(|profile| profile.active_leaf && profile.simulation_eligible)
        .collect::<Vec<_>>();
    let active_ids = profiles
        .iter()
        .map(|profile| profile.subject_id.clone())
        .collect::<Vec<_>>();
    let active_id_set = active_ids.iter().cloned().collect::<BTreeSet<_>>();
    let axes = [
        AgencyAxis::Geography,
        AgencyAxis::Ideology,
        AgencyAxis::Authority,
        AgencyAxis::EconomyRole,
        AgencyAxis::SpeciesBody,
        AgencyAxis::Information,
    ];
    let mut facet_value_counts: BTreeMap<AgencyAxis, BTreeMap<String, u64>> = axes
        .iter()
        .cloned()
        .map(|axis| (axis, BTreeMap::new()))
        .collect();
    let mut collective_authority_counts = BTreeMap::<String, u64>::new();
    for profile in &profiles {
        for axis in &axes {
            let buckets = facet_value_counts
                .get_mut(axis)
                .expect("all six agency axes were initialized");
            match profile.facets.get(axis).filter(|values| !values.is_empty()) {
                Some(values) => {
                    for value in values {
                        *buckets.entry(value.clone()).or_default() += 1;
                    }
                }
                None => {
                    *buckets.entry("unknown".into()).or_default() += 1;
                }
            }
        }
        let authority = profile
            .collective_authority_id
            .clone()
            .unwrap_or_else(|| "none".into());
        *collective_authority_counts.entry(authority).or_default() += 1;
    }
    let mut relation_summary = BTreeMap::<String, RelationDemandSummary>::new();
    for relation in campaign
        .agency_relations
        .values()
        .filter(|relation| relation.active)
    {
        let kind = serde_json::to_value(&relation.kind)
            .ok()
            .and_then(|value| value.as_str().map(str::to_owned))
            .unwrap_or_else(|| "unknown".into());
        let summary = relation_summary.entry(kind).or_default();
        summary.edges += 1;
        summary.total_strength += u64::from(relation.strength);
        summary.max_strength = summary.max_strength.max(relation.strength);
    }
    let recent_events = campaign
        .events
        .iter()
        .rev()
        .take(12)
        .map(|event| {
            serde_json::json!({
                "id":event.id,
                "kind":event.kind,
                "summary":event.summary,
                "actor_ids":event.actor_ids,
                "institution_ids":event.institution_ids,
                "gestalt_ids":event.gestalt_ids,
                "location_ids":event.location_ids,
                "public_channels":event.public_channels,
            })
        })
        .collect::<Vec<_>>();
    let recent_participant_ids = campaign
        .events
        .iter()
        .rev()
        .take(12)
        .flat_map(|event| {
            event
                .actor_ids
                .iter()
                .chain(event.institution_ids.iter())
                .chain(event.gestalt_ids.iter())
        })
        .filter(|id| campaign.agency_profiles.contains_key(*id))
        .cloned()
        .collect::<BTreeSet<_>>();
    let mut highest_detail_debt = profiles
        .iter()
        .filter(|profile| profile.detail_debt > 0)
        .map(|profile| (profile.subject_id.clone(), profile.detail_debt))
        .collect::<Vec<_>>();
    highest_detail_debt
        .sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));
    highest_detail_debt
        .truncate(usize::from(campaign.resolution_policy.active_cell_budget).max(4) * 2);
    let mut focal_candidate_ids = highest_detail_debt
        .iter()
        .map(|(subject_id, _)| subject_id.clone())
        .collect::<BTreeSet<_>>();
    if !recent_participant_ids.is_empty() && recent_participant_ids.len() < active_id_set.len() {
        focal_candidate_ids.extend(recent_participant_ids.iter().cloned());
    }
    let focal_candidate_ids = focal_candidate_ids.into_iter().collect::<Vec<_>>();
    (
        focal_candidate_ids.clone(),
        serde_json::json!({
            "active_subject_count": profiles.len(),
            "facet_value_counts": facet_value_counts,
            "collective_authority_counts": collective_authority_counts,
            "relation_summary": relation_summary,
            "recent_events": recent_events,
            "recent_participant_ids": recent_participant_ids,
            "focal_candidate_ids": focal_candidate_ids,
            "highest_detail_debt": highest_detail_debt.iter().map(|(subject_id, detail_debt)| serde_json::json!({
                "subject_id":subject_id,
                "detail_debt":detail_debt,
            })).collect::<Vec<_>>(),
        }),
    )
}

fn cell_slice(campaign: &Campaign, cell: &SimulationCell) -> Result<PermittedCellSlice> {
    let mut member_exceptions = member_exceptions(campaign, cell)?;
    let selected_member_ids = member_exceptions
        .iter()
        .map(|member| member.subject_id.clone())
        .collect::<BTreeSet<_>>();
    let mut constituents = cell
        .subject_ids
        .iter()
        .map(|id| constituent_slice(campaign, id))
        .collect::<Result<Vec<_>>>()?;
    for subject in &mut constituents {
        subject.activity_target_ids.retain(|target| {
            !target.starts_with("member:") || selected_member_ids.contains(target)
        });
    }
    for member in &mut member_exceptions {
        member.activity_target_ids.retain(|target| {
            !target.starts_with("member:") || selected_member_ids.contains(target)
        });
    }
    let shared_knowledge = intersection(
        constituents
            .iter()
            .map(|subject| &subject.knowledge)
            .collect(),
    );
    let shared_capabilities = intersection(
        constituents
            .iter()
            .map(|subject| &subject.capabilities)
            .collect(),
    );
    let perceived_events = cell_perceived_events(campaign, &constituents, &member_exceptions);
    Ok(PermittedCellSlice {
        cell_id: cell.id.clone(),
        mode: cell.mode.clone(),
        world_revision: campaign.revision,
        resolution_epoch: campaign.resolution_policy.resolution_epoch,
        snapshot_binding: format!(
            "campaign:{}:revision:{}:resolution:{}:cell:{}",
            campaign.id, campaign.revision, campaign.resolution_policy.resolution_epoch, cell.id
        ),
        constituents,
        member_exceptions,
        shared_knowledge,
        shared_capabilities,
        perceived_events,
        world_clock_pressure: campaign
            .clocks
            .values()
            .map(project_world_clock_pressure)
            .collect(),
        detail_focus_subject_id: cell.detail_focus_subject_id.clone(),
        max_actions: cell_action_limit(cell),
        source_receipt_ids: campaign.branch_origin.evidence_receipt_ids.clone(),
    })
}

fn project_world_clock_pressure(clock: &crate::domain::WorldClock) -> String {
    if clock.progress >= clock.threshold {
        format!(
            "{}: threshold reached ({}/{}); declared consequence: {}",
            clock.label, clock.progress, clock.threshold, clock.consequence
        )
    } else {
        format!(
            "{}: progress {} of {}; {} step(s) remain before declared consequence: {}",
            clock.label,
            clock.progress,
            clock.threshold,
            clock.threshold.saturating_sub(clock.progress),
            clock.consequence
        )
    }
}

fn cell_perceived_events(
    campaign: &Campaign,
    constituents: &[CellConstituentSlice],
    member_exceptions: &[CellMemberSlice],
) -> Vec<CellPerceivedEventSlice> {
    campaign
        .events
        .iter()
        .rev()
        .take(12)
        .filter_map(|event| {
            let mut perceived_by_subject_ids = constituents
                .iter()
                .filter(|subject| {
                    event.actor_ids.contains(&subject.subject_id)
                        || event.institution_ids.contains(&subject.subject_id)
                        || event.gestalt_ids.contains(&subject.subject_id)
                        || !event
                            .location_ids
                            .iter()
                            .all(|location_id| !subject.location_ids.contains(location_id))
                        || !event
                            .public_channels
                            .iter()
                            .all(|channel| !subject.information_channels.contains(channel))
                })
                .map(|subject| subject.subject_id.clone())
                .collect::<BTreeSet<_>>();
            perceived_by_subject_ids.extend(
                member_exceptions
                    .iter()
                    .filter(|member| {
                        event.actor_ids.contains(&member.subject_id)
                            || event.location_ids.contains(&member.source_location_id)
                            || !event
                                .public_channels
                                .iter()
                                .all(|channel| !member.information_channels.contains(channel))
                    })
                    .map(|member| member.subject_id.clone()),
            );
            (!perceived_by_subject_ids.is_empty()).then(|| CellPerceivedEventSlice {
                event_id: event.id.clone(),
                summary: event.summary.clone(),
                perceived_by_subject_ids,
            })
        })
        .collect()
}

fn member_exceptions(campaign: &Campaign, cell: &SimulationCell) -> Result<Vec<CellMemberSlice>> {
    let player_id = &campaign.player_actor_id;
    let mut candidates = campaign
        .gestalt_members
        .values()
        .filter(|member| {
            member.materialized_actor_id.is_none() && cell.subject_ids.contains(&member.gestalt_id)
        })
        .filter_map(|member| {
            let source = campaign.gestalts.get(&member.gestalt_id)?;
            let origin = member
                .last_location_id
                .clone()
                .unwrap_or_else(|| source.home_location_id.clone());
            let destinations =
                gestalt_migration_destinations(campaign, &member.gestalt_id, &origin);
            let activity_targets =
                crate::resolution::member_activity_targets(campaign, &member.id).ok()?;
            if destinations.is_empty() && activity_targets.is_empty() {
                return None;
            }
            let player_relationship = member.relationships.contains_key(player_id) as u8;
            let personal_pressure = (!member.conditions.is_empty()
                || !member.obligations.is_empty()
                || !member.goals.is_empty()) as u8;
            Some((
                player_relationship,
                personal_pressure,
                member.last_relevant_revision,
                member.id.clone(),
                origin,
                destinations,
                activity_targets,
            ))
        })
        .collect::<Vec<_>>();
    candidates.sort_by(|left, right| {
        right
            .0
            .cmp(&left.0)
            .then_with(|| right.1.cmp(&left.1))
            .then_with(|| right.2.cmp(&left.2))
            .then_with(|| left.3.cmp(&right.3))
    });
    candidates
        .into_iter()
        .take(cell_action_limit(cell).min(4))
        .map(
            |(_, _, _, member_id, origin, destinations, activity_target_ids)| {
                let member = &campaign.gestalt_members[&member_id];
                let source = &campaign.gestalts[&member.gestalt_id];
                let capabilities = overlay(
                    &source.shared_capabilities,
                    &member.capability_additions,
                    &member.capability_removals,
                );
                let knowledge = overlay(
                    &source.shared_knowledge,
                    &member.knowledge_additions,
                    &member.knowledge_removals,
                );
                let information_channels =
                    crate::resolution::effective_member_information_channels(campaign, &member.id)?;
                let goals = if member.goals.is_empty() {
                    source.goals.clone()
                } else {
                    member.goals.clone()
                };
                let mut permitted_state_references = BTreeSet::from([
                    format!("member:{}", member.id),
                    format!("gestalt:{}", member.gestalt_id),
                    format!("location:{origin}"),
                ]);
                permitted_state_references.extend(
                    capabilities
                        .iter()
                        .map(|value| format!("capability:{value}")),
                );
                permitted_state_references
                    .extend(knowledge.iter().map(|value| format!("knowledge:{value}")));
                permitted_state_references.extend(
                    member
                        .equipment
                        .iter()
                        .map(|value| format!("resource:{value}")),
                );
                for (gestalt_id, location_id) in &destinations {
                    permitted_state_references.insert(format!("gestalt:{gestalt_id}"));
                    permitted_state_references.insert(format!("location:{location_id}"));
                }
                Ok(CellMemberSlice {
                    subject_id: format!("member:{}", member.id),
                    member_id: member.id.clone(),
                    name: member.name.clone(),
                    source_gestalt_id: member.gestalt_id.clone(),
                    source_location_id: origin,
                    knowledge: knowledge.clone(),
                    capabilities,
                    resources: member.equipment.clone(),
                    information_channels,
                    permitted_state_references,
                    migration_destinations: destinations,
                    activity_target_ids,
                    goals,
                    pressures: member
                        .conditions
                        .iter()
                        .chain(&member.obligations)
                        .cloned()
                        .collect(),
                    relationships: member.relationships.clone(),
                    memories: member.memories.clone(),
                })
            },
        )
        .collect()
}

fn gestalt_migration_destinations(
    campaign: &Campaign,
    source_gestalt_id: &str,
    origin_location_id: &str,
) -> BTreeMap<String, String> {
    campaign
        .agency_relations
        .values()
        .filter(|relation| {
            relation.active
                && relation.kind == crate::domain::AgencyRelationKind::Migration
                && relation.from_subject_id == source_gestalt_id
        })
        .filter_map(|relation| {
            let destination = campaign.gestalts.get(&relation.to_subject_id)?;
            let profile = campaign.agency_profiles.get(&destination.id)?;
            if !profile.active_leaf || !profile.simulation_eligible {
                return None;
            }
            let reachable = origin_location_id == destination.home_location_id
                || campaign
                    .locations
                    .get(origin_location_id)
                    .is_some_and(|location| {
                        location.routes.values().any(|route| {
                            route.destination_id == destination.home_location_id
                                && route.travel_minutes <= campaign.tick_hours.saturating_mul(60)
                        })
                    });
            reachable.then(|| (destination.id.clone(), destination.home_location_id.clone()))
        })
        .collect()
}

fn overlay(
    baseline: &BTreeSet<String>,
    additions: &BTreeSet<String>,
    removals: &BTreeSet<String>,
) -> BTreeSet<String> {
    baseline
        .union(additions)
        .filter(|value| !removals.contains(*value))
        .cloned()
        .collect()
}

fn constituent_slice(campaign: &Campaign, id: &str) -> Result<CellConstituentSlice> {
    let profile = campaign
        .agency_profiles
        .get(id)
        .ok_or_else(|| anyhow!("simulation cell subject lacks an agency profile"))?;
    let mut value = CellConstituentSlice {
        subject_id: id.into(),
        subject_kind: profile.subject_kind.clone(),
        name: id.into(),
        collective_authority_id: profile.collective_authority_id.clone(),
        location_ids: profile.location_ids.clone(),
        knowledge: BTreeSet::new(),
        capabilities: BTreeSet::new(),
        resources: BTreeSet::new(),
        information_channels: profile.information_channels.clone(),
        permitted_state_references: crate::resolution::subject_state_references(campaign, id)?,
        reachable_destination_ids: BTreeSet::new(),
        migration_destinations: BTreeMap::new(),
        activity_target_ids: crate::resolution::strategic_activity_targets(campaign, id),
        goals: vec![],
        current_posture: None,
        pressures: vec![],
    };
    match profile.subject_kind {
        AgencySubjectKind::Actor => {
            let actor = campaign.actors.get(id).context("cell actor vanished")?;
            value.name = actor.name.clone();
            value.knowledge = actor.knowledge.clone();
            value.capabilities = actor.capabilities.clone();
            value.resources = actor.equipment.clone();
            value.goals = actor.goals.clone();
            value.pressures = actor
                .conditions
                .iter()
                .chain(&actor.obligations)
                .cloned()
                .collect();
            value.reachable_destination_ids = campaign
                .locations
                .get(&actor.location_id)
                .into_iter()
                .flat_map(|location| location.routes.values())
                .filter(|route| route.travel_minutes <= campaign.tick_hours.saturating_mul(60))
                .map(|route| route.destination_id.clone())
                .collect();
        }
        AgencySubjectKind::Institution => {
            let institution = campaign
                .institutions
                .get(id)
                .context("cell institution vanished")?;
            value.name = institution.name.clone();
            value.resources = institution.resources.iter().cloned().collect();
            value.goals = institution.goals.clone();
            value.current_posture = Some(institution.posture.clone());
        }
        AgencySubjectKind::Gestalt => {
            let gestalt = campaign.gestalts.get(id).context("cell gestalt vanished")?;
            value.name = gestalt.name.clone();
            value.knowledge = gestalt.shared_knowledge.clone();
            value.capabilities = gestalt.shared_capabilities.clone();
            value.resources = gestalt.resources.clone();
            value.goals = gestalt.goals.clone();
            value.pressures = gestalt.pressures.clone();
            value.migration_destinations =
                gestalt_migration_destinations(campaign, id, &gestalt.home_location_id);
            value
                .migration_destinations
                .retain(|_, location_id| location_id != &gestalt.home_location_id);
            for (destination_id, location_id) in &value.migration_destinations {
                value
                    .permitted_state_references
                    .insert(format!("gestalt:{destination_id}"));
                value
                    .permitted_state_references
                    .insert(format!("location:{location_id}"));
            }
        }
    }
    Ok(value)
}

fn intersection(sets: Vec<&BTreeSet<String>>) -> BTreeSet<String> {
    let Some(first) = sets.first() else {
        return BTreeSet::new();
    };
    sets.iter().skip(1).fold((*first).clone(), |current, next| {
        current.intersection(next).cloned().collect()
    })
}

pub fn due_tick_target(now: DateTime<Utc>, last_player_activity: DateTime<Utc>) -> u8 {
    let idle = now - last_player_activity;
    if idle < chrono::Duration::minutes(15) {
        0
    } else {
        (idle.num_hours().max(0) as u8).min(8)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::ModelPort;
    use crate::persona::AllowAllPermit;
    use async_trait::async_trait;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct CellFixtureModel {
        active: AtomicUsize,
        maximum: AtomicUsize,
        malformed_cell: bool,
    }

    #[test]
    fn clock_projection_distinguishes_remaining_time_from_reached_consequence() {
        let mut clock = crate::domain::WorldClock {
            id: "ferry".into(),
            label: "Last protected ferry".into(),
            progress: 3,
            threshold: 4,
            consequence: "the camp is cut off by the storm".into(),
        };
        assert_eq!(
            project_world_clock_pressure(&clock),
            "Last protected ferry: progress 3 of 4; 1 step(s) remain before declared consequence: the camp is cut off by the storm"
        );
        clock.progress = 4;
        assert_eq!(
            project_world_clock_pressure(&clock),
            "Last protected ferry: threshold reached (4/4); declared consequence: the camp is cut off by the storm"
        );
    }

    impl CellFixtureModel {
        fn enter(&self) -> ActiveCall<'_> {
            let active = self.active.fetch_add(1, Ordering::SeqCst) + 1;
            self.maximum.fetch_max(active, Ordering::SeqCst);
            ActiveCall(self)
        }
    }

    struct ActiveCall<'a>(&'a CellFixtureModel);
    impl Drop for ActiveCall<'_> {
        fn drop(&mut self) {
            self.0.active.fetch_sub(1, Ordering::SeqCst);
        }
    }

    #[async_trait]
    impl ModelPort for CellFixtureModel {
        async fn run(&self, request: &ModelStageRequest) -> Result<String> {
            let _active = self.enter();
            tokio::time::sleep(std::time::Duration::from_millis(2)).await;
            match request.stage.as_str() {
                "resolution_demand" => Ok(serde_json::json!({
                    "axis_weights": {
                        "geography":0.25,
                        "ideology":0.20,
                        "authority":0.25,
                        "economy_role":0.15,
                        "species_body":0.05,
                        "information":0.10
                    },
                    "focal_subject_ids":[],
                    "rationale":"fixture pressure"
                })
                .to_string()),
                "cell_projector" => {
                    let subject_id = request
                        .output_schema
                        .as_ref()
                        .and_then(|schema| {
                            schema.pointer(
                                "/$defs/CellPerspectiveSegment/properties/subject_id/enum/0",
                            )
                        })
                        .and_then(serde_json::Value::as_str)
                        .ok_or_else(|| anyhow!("fixture projector lacks a bound subject"))?;
                    Ok(serde_json::json!({
                        "segments":[{
                            "subject_id":subject_id,
                            "narrative":"The horizon tightens around our own unresolved choice."
                        }]
                    })
                    .to_string())
                }
                "cell_persona" => Ok("Each constituent watches and deliberately holds.".into()),
                "cell_interpreter" if self.malformed_cell => Ok("not-json".into()),
                "cell_interpreter" => Ok(serde_json::json!({
                    "actions": [],
                    "inaction_reason": "No constituent has a justified move this horizon."
                })
                .to_string()),
                stage => Err(anyhow!("unexpected fixture stage {stage}")),
            }
        }

        fn provider(&self) -> &'static str {
            "fixture"
        }
    }

    #[test]
    fn away_budget_waits_and_caps_at_eight() {
        let now = Utc::now();
        assert_eq!(due_tick_target(now, now - chrono::Duration::minutes(14)), 0);
        assert_eq!(due_tick_target(now, now - chrono::Duration::minutes(59)), 0);
        assert_eq!(due_tick_target(now, now - chrono::Duration::hours(1)), 1);
        assert_eq!(due_tick_target(now, now - chrono::Duration::hours(30)), 8);
    }

    #[test]
    fn demand_projection_receives_aggregate_boundaries_not_full_subject_state() {
        let mut campaign = crate::resolution::tests::campaign(1_000, 8);
        let (focal_candidates, context) = resolution_demand_context(&campaign);
        let encoded = serde_json::to_string(&context).unwrap();
        assert_eq!(context["active_subject_count"], 1_000);
        assert!(focal_candidates.is_empty());
        assert_eq!(context["facet_value_counts"].as_object().unwrap().len(), 6);
        assert!(
            encoded.len() < 100_000,
            "aggregate context was {} chars",
            encoded.len()
        );
        assert!(!encoded.contains("permitted_state_references"));
        assert!(!encoded.contains("information_channels"));
        assert!(!encoded.contains("evidence_receipt_ids"));

        campaign.events.push(crate::domain::Event {
            id: "exceptional-pressure".into(),
            at: Utc::now(),
            kind: "test".into(),
            summary: "One faction receives an exceptional warning.".into(),
            actor_ids: vec![],
            institution_ids: vec!["faction-0007".into()],
            gestalt_ids: vec![],
            location_ids: vec![],
            public_channels: vec![],
        });
        let (focal_candidates, context) = resolution_demand_context(&campaign);
        assert_eq!(focal_candidates, vec!["faction-0007"]);
        assert_eq!(context["focal_candidate_ids"][0], "faction-0007");

        campaign.events[0].institution_ids = (0..1_000)
            .map(|index| format!("faction-{index:04}"))
            .collect();
        let (focal_candidates, context) = resolution_demand_context(&campaign);
        assert!(focal_candidates.is_empty());
        assert!(
            context["focal_candidate_ids"]
                .as_array()
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn cell_slice_projects_only_actionable_salient_member_exceptions() {
        use crate::domain::*;
        let mut campaign = crate::resolution::tests::campaign(0, 1);
        let gestalt = |id: &str, name: &str| GestaltPersonaState {
            schema: "ghostlight.gestalt_persona_state.v1".into(),
            id: id.into(),
            name: name.into(),
            version: 0,
            home_location_id: "center".into(),
            shared_capabilities: BTreeSet::new(),
            shared_knowledge: BTreeSet::new(),
            resources: BTreeSet::new(),
            goals: vec![],
            pressures: vec![],
        };
        campaign
            .gestalts
            .insert("refugees".into(), gestalt("refugees", "Refugees"));
        campaign
            .gestalts
            .insert("neighbors".into(), gestalt("neighbors", "Neighbors"));
        let member = |id: &str, player_relationship: bool| GestaltMemberDelta {
            schema: "ghostlight.gestalt_member_delta.v1".into(),
            id: id.into(),
            gestalt_id: "refugees".into(),
            version: 1,
            name: id.into(),
            capability_additions: BTreeSet::new(),
            capability_removals: BTreeSet::new(),
            knowledge_additions: BTreeSet::new(),
            knowledge_removals: BTreeSet::new(),
            equipment: BTreeSet::new(),
            conditions: BTreeSet::new(),
            obligations: BTreeSet::new(),
            relationships: if player_relationship {
                BTreeMap::from([("player".into(), "trusted rescuer".into())])
            } else {
                BTreeMap::new()
            },
            goals: vec!["find a home".into()],
            memories: vec![],
            last_location_id: Some("center".into()),
            materialized_actor_id: None,
            last_relevant_revision: 1,
            relevance_lease_until_revision: 0,
        };
        campaign
            .gestalt_members
            .insert("mira".into(), member("mira", true));
        campaign
            .gestalt_members
            .insert("other".into(), member("other", false));
        for member_id in ["third", "fourth", "fifth", "sixth", "seventh"] {
            campaign
                .gestalt_members
                .insert(member_id.into(), member(member_id, false));
        }
        crate::resolution::ensure_agency_profiles(&mut campaign);
        campaign.agency_relations.insert(
            "migration".into(),
            AgencyRelation {
                schema: "ghostlight.agency_relation.v1".into(),
                id: "migration".into(),
                from_subject_id: "refugees".into(),
                to_subject_id: "neighbors".into(),
                kind: AgencyRelationKind::Migration,
                strength: 90,
                active: true,
                evidence_receipt_ids: vec![],
            },
        );
        let cover = crate::resolution::plan_cover(
            &campaign,
            crate::resolution::default_demand(&campaign, "resettlement"),
        )
        .unwrap();
        let slice = cell_slice(&campaign, &cover.cells[0]).unwrap();
        assert_eq!(slice.member_exceptions.len(), 2);
        assert_eq!(slice.member_exceptions[0].member_id, "mira");
        assert_eq!(
            slice.member_exceptions[0]
                .migration_destinations
                .get("neighbors")
                .map(String::as_str),
            Some("center")
        );
        assert!(
            slice.member_exceptions[0]
                .permitted_state_references
                .contains("member:mira")
        );
        assert!(
            slice.member_exceptions[0]
                .activity_target_ids
                .contains("refugees")
        );
        let selected_member_ids = slice
            .member_exceptions
            .iter()
            .map(|member| member.subject_id.clone())
            .collect::<BTreeSet<_>>();
        assert!(selected_member_ids.contains("member:mira"));
        assert!(slice.constituents.iter().all(|constituent| {
            constituent
                .activity_target_ids
                .iter()
                .filter(|target| target.starts_with("member:"))
                .all(|target| selected_member_ids.contains(target))
        }));
        assert!(slice.member_exceptions.iter().all(|member| {
            member
                .activity_target_ids
                .iter()
                .filter(|target| target.starts_with("member:"))
                .all(|target| selected_member_ids.contains(target))
        }));
        assert!(
            !serde_json::to_string(&slice)
                .unwrap()
                .contains("member:seventh")
        );
        assert!(
            !serde_json::to_string(&slice)
                .unwrap()
                .contains("private dock code")
        );

        campaign.agency_relations.clear();
        let local_only_cover = crate::resolution::plan_cover(
            &campaign,
            crate::resolution::default_demand(&campaign, "ordinary local work"),
        )
        .unwrap();
        let local_only = cell_slice(&campaign, &local_only_cover.cells[0]).unwrap();
        assert!(
            local_only
                .member_exceptions
                .iter()
                .any(|member| member.member_id == "mira"
                    && member.migration_destinations.is_empty()
                    && member.activity_target_ids.contains("refugees"))
        );
    }

    #[test]
    fn institution_slice_does_not_disguise_committed_posture_as_pressure() {
        let campaign = crate::resolution::tests::campaign(1, 1);
        let institution = &campaign.institutions["faction-0000"];
        let slice = constituent_slice(&campaign, "faction-0000").unwrap();
        assert_eq!(
            slice.current_posture.as_deref(),
            Some(institution.posture.as_str())
        );
        assert!(slice.pressures.is_empty());
    }

    #[test]
    fn arena_event_projection_preserves_exact_viewers_including_named_members() {
        use crate::domain::*;
        let mut campaign = crate::resolution::tests::campaign(2, 1);
        campaign
            .agency_profiles
            .get_mut("faction-0000")
            .unwrap()
            .location_ids = BTreeSet::from(["east".into()]);
        let west = campaign.agency_profiles.get_mut("faction-0001").unwrap();
        west.location_ids = BTreeSet::from(["west".into()]);
        west.information_channels.insert("west-wire".into());
        campaign.events = vec![
            Event {
                id: "direct-east".into(),
                at: Utc::now(),
                kind: "test".into(),
                summary: "East receives a private order.".into(),
                actor_ids: vec![],
                institution_ids: vec!["faction-0000".into()],
                gestalt_ids: vec![],
                location_ids: vec![],
                public_channels: vec![],
            },
            Event {
                id: "west-wire".into(),
                at: Utc::now(),
                kind: "test".into(),
                summary: "A warning travels on the west wire.".into(),
                actor_ids: vec![],
                institution_ids: vec![],
                gestalt_ids: vec![],
                location_ids: vec![],
                public_channels: vec!["west-wire".into()],
            },
            Event {
                id: "mira-witnessed".into(),
                at: Utc::now(),
                kind: "test".into(),
                summary: "Mira witnesses the departure.".into(),
                actor_ids: vec!["member:mira".into()],
                institution_ids: vec![],
                gestalt_ids: vec![],
                location_ids: vec![],
                public_channels: vec![],
            },
            Event {
                id: "unseen".into(),
                at: Utc::now(),
                kind: "test".into(),
                summary: "Nobody in this arena can know this.".into(),
                actor_ids: vec![],
                institution_ids: vec![],
                gestalt_ids: vec![],
                location_ids: vec!["far-away".into()],
                public_channels: vec!["sealed-channel".into()],
            },
        ];
        let constituents = ["faction-0000", "faction-0001"]
            .iter()
            .map(|id| constituent_slice(&campaign, id).unwrap())
            .collect::<Vec<_>>();
        let members = vec![CellMemberSlice {
            subject_id: "member:mira".into(),
            member_id: "mira".into(),
            name: "Mira".into(),
            source_gestalt_id: "refugees".into(),
            source_location_id: "east".into(),
            knowledge: BTreeSet::new(),
            capabilities: BTreeSet::new(),
            resources: BTreeSet::new(),
            information_channels: BTreeSet::from(["refugee-wire".into()]),
            permitted_state_references: BTreeSet::new(),
            migration_destinations: BTreeMap::new(),
            activity_target_ids: BTreeSet::new(),
            goals: vec![],
            pressures: vec![],
            relationships: BTreeMap::new(),
            memories: vec![],
        }];

        let events = cell_perceived_events(&campaign, &constituents, &members)
            .into_iter()
            .map(|event| (event.event_id, event.perceived_by_subject_ids))
            .collect::<BTreeMap<_, _>>();
        assert_eq!(
            events["direct-east"],
            BTreeSet::from(["faction-0000".into()])
        );
        assert_eq!(events["west-wire"], BTreeSet::from(["faction-0001".into()]));
        assert_eq!(
            events["mira-witnessed"],
            BTreeSet::from(["member:mira".into()])
        );
        assert!(!events.contains_key("unseen"));
    }

    #[tokio::test]
    async fn every_selected_cell_runs_one_membrane_pipeline() {
        let mut campaign = crate::resolution::tests::campaign(6, 2);
        campaign.resolution_policy.provider_parallelism = 2;
        let model = Arc::new(CellFixtureModel {
            active: AtomicUsize::new(0),
            maximum: AtomicUsize::new(0),
            malformed_cell: false,
        });
        let output = propose_resolution_wave(model.clone(), Arc::new(AllowAllPermit), &campaign)
            .await
            .unwrap();
        assert_eq!(output.wave.cover.cells.len(), 2);
        assert_eq!(output.wave.appraisals.len(), 2);
        assert_eq!(output.stages.len(), 7);
        assert!(model.maximum.load(Ordering::SeqCst) <= 2);
        validate_and_resolve_wave(&campaign, &output.wave).unwrap();
    }

    #[tokio::test]
    async fn one_malformed_cell_aborts_the_whole_wave() {
        let campaign = crate::resolution::tests::campaign(6, 2);
        let before = campaign.clone();
        let model = Arc::new(CellFixtureModel {
            active: AtomicUsize::new(0),
            maximum: AtomicUsize::new(0),
            malformed_cell: true,
        });
        assert!(
            propose_resolution_wave(model, Arc::new(AllowAllPermit), &campaign)
                .await
                .is_err()
        );
        assert_eq!(campaign, before);
    }
}
