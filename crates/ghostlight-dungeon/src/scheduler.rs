use crate::{
    domain::{
        AgencyAxis, AgencySubjectKind, Campaign, GestaltIndividuation, GestaltMemberDelta,
        ResolutionDemand, ResolutionWaveCommit, SimulationCell, StrategicGestaltIndividuation,
    },
    model::{MODEL_FAST, ModelPort, ModelStageOutput, ModelStageRequest, run_validated_stage},
    outcome::{activity_outcome_binding, resolve_activity_outcomes},
    persona::{
        CellActivityTargetSlice, CellConstituentSlice, CellMemberSlice,
        CellMigrationDestinationSlice, CellPerceivedEventSlice, CellProjectionEngine,
        ExecutionPermit, PermittedCellSlice,
    },
    resolution::{
        cell_action_digest, cell_action_limit, default_demand, plan_cover, plan_receipt,
        select_resolution_wave, validate_and_resolve_wave, validate_demand,
    },
    session_zero::{AggregatedBoundary, CampaignContract},
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
    axis_weights: DemandAxisWeights,
    focal_subject_ids: BTreeSet<String>,
    rationale: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
struct DemandAxisWeights {
    geography: f32,
    ideology: f32,
    authority: f32,
    economy_role: f32,
    species_body: f32,
    information: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
struct StrategicPersonDraft {
    action_digest: String,
    gestalt_id: String,
    member_id: String,
    name: String,
    goals: Vec<String>,
    obligations: BTreeSet<String>,
    relationships: BTreeMap<String, String>,
    memories: Vec<String>,
    rationale: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
struct StrategicPersonSelection {
    proposals: Vec<StrategicPersonDraft>,
}

impl DemandAxisWeights {
    fn into_map(self) -> BTreeMap<AgencyAxis, f32> {
        BTreeMap::from([
            (AgencyAxis::Geography, self.geography),
            (AgencyAxis::Ideology, self.ideology),
            (AgencyAxis::Authority, self.authority),
            (AgencyAxis::EconomyRole, self.economy_role),
            (AgencyAxis::SpeciesBody, self.species_body),
            (AgencyAxis::Information, self.information),
        ])
    }
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
    propose_resolution_wave_with_policy(model, permit, campaign, None, &[]).await
}

pub async fn propose_resolution_wave_with_policy(
    model: Arc<dyn ModelPort>,
    permit: Arc<dyn ExecutionPermit>,
    campaign: &Campaign,
    campaign_contract: Option<&CampaignContract>,
    aggregate_boundaries: &[AggregatedBoundary],
) -> Result<StrategicResolutionOutput> {
    let (demand, mut stages) = project_resolution_demand(
        model.as_ref(),
        campaign,
        campaign_contract,
        aggregate_boundaries,
    )
    .await;
    let cover = plan_cover(campaign, demand)?;
    let receipt = plan_receipt(campaign, &cover);
    let outcome_model = model.clone();
    let outcome_permit = permit.clone();
    let engine = CellProjectionEngine {
        model,
        permit,
        projector_model: MODEL_FAST.into(),
        persona_model: MODEL_FAST.into(),
        interpreter_model: MODEL_FAST.into(),
        campaign_contract: campaign_contract.cloned(),
        aggregate_boundaries: aggregate_boundaries.to_vec(),
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
            let cell_id = cell.id;
            let subject_ids = cell.subject_ids;
            let _permit = semaphore
                .acquire_owned()
                .await
                .map_err(|_| anyhow!("provider concurrency gate closed"))?;
            let terminal = engine.execute(slice).await.with_context(|| {
                format!(
                    "simulation cell {cell_id} subjects {} pipeline failed",
                    serde_json::to_string(&subject_ids).unwrap_or_else(|_| "[unavailable]".into())
                )
            })?;
            Ok::<_, anyhow::Error>((cell_id, terminal))
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
    let cell_model_receipt_hashes = distinct_model_receipt_hashes(&stages);
    let mut wave = ResolutionWaveCommit {
        schema: "ghostlight.resolution_wave_commit.v1".into(),
        world_revision: campaign.revision,
        resolution_epoch: campaign.resolution_policy.resolution_epoch,
        cover,
        plan_receipt: receipt,
        appraisals,
        activity_outcomes: Vec::new(),
        strategic_individuations: Vec::new(),
        model_receipt_hashes: cell_model_receipt_hashes,
    };
    let selection = select_resolution_wave(campaign, &wave)?;
    let outcome_digests = selection
        .activity_proposals
        .iter()
        .map(cell_action_digest)
        .collect::<Result<Vec<_>>>()?;
    let outcome_binding = activity_outcome_binding(
        campaign.id,
        campaign.revision,
        campaign.resolution_policy.resolution_epoch,
        &outcome_digests,
    );
    if !outcome_digests.is_empty() {
        outcome_permit
            .require(
                "strategic-outcomes",
                &outcome_binding,
                "strategic_outcome_resolver",
            )
            .await?;
    }
    let (activity_outcomes, outcome_stages) = resolve_activity_outcomes(
        outcome_model.clone(),
        campaign,
        &selection.activity_proposals,
    )
    .await?;
    if !outcome_digests.is_empty() {
        outcome_permit
            .require(
                "strategic-outcomes",
                &outcome_binding,
                "strategic_outcome_terminal",
            )
            .await?;
    }
    stages.extend(outcome_stages);
    wave.activity_outcomes = activity_outcomes;
    let individuation_candidate_digests =
        strategic_individuation_candidate_digests(campaign, &selection.plan.selected_actions);
    if !individuation_candidate_digests.is_empty() {
        outcome_permit
            .require(
                "strategic-individuation",
                &strategic_individuation_binding(campaign, &individuation_candidate_digests, None),
                "strategic_individuation_selector",
            )
            .await?;
    }
    let (strategic_individuations, individuation_stages) = propose_strategic_individuation(
        outcome_model.as_ref(),
        campaign,
        &selection.plan.selected_actions,
    )
    .await;
    if !individuation_candidate_digests.is_empty() {
        let proposal_digest = strategic_individuations
            .first()
            .map(strategic_individuation_proposal_digest)
            .transpose()?;
        outcome_permit
            .require(
                "strategic-individuation",
                &strategic_individuation_binding(
                    campaign,
                    &individuation_candidate_digests,
                    proposal_digest.as_deref(),
                ),
                "strategic_individuation_terminal",
            )
            .await?;
    }
    stages.extend(individuation_stages);
    wave.strategic_individuations = strategic_individuations;
    wave.model_receipt_hashes = distinct_model_receipt_hashes(&stages);
    validate_and_resolve_wave(campaign, &wave)?;
    let aggregate_receipt_hash = format!(
        "sha256:{:x}",
        Sha256::digest(wave.model_receipt_hashes.join("|").as_bytes())
    );
    Ok(StrategicResolutionOutput {
        wave,
        stages,
        private_cell_traces,
        aggregate_receipt_hash,
    })
}

pub fn strategic_individuation_binding(
    campaign: &Campaign,
    action_digests: &[String],
    proposal_digest: Option<&str>,
) -> String {
    format!(
        "campaign:{}:revision:{}:resolution:{}:strategic-individuation:{}:proposal:{}",
        campaign.id,
        campaign.revision,
        campaign.resolution_policy.resolution_epoch,
        action_digests.join(","),
        proposal_digest.unwrap_or("none")
    )
}

pub fn strategic_individuation_candidate_digests(
    campaign: &Campaign,
    selected_actions: &[crate::domain::CellActionProposal],
) -> Vec<String> {
    selected_actions
        .iter()
        .filter(|action| {
            campaign.gestalts.contains_key(&action.subject_id)
                && campaign
                    .agency_profiles
                    .get(&action.subject_id)
                    .is_some_and(|profile| {
                        profile.active_leaf
                            && profile.simulation_eligible
                            && profile.location_ids.len() == 1
                    })
        })
        .filter_map(|action| cell_action_digest(action).ok())
        .collect()
}

pub fn strategic_individuation_proposal_digest(
    proposal: &StrategicGestaltIndividuation,
) -> Result<String> {
    Ok(format!(
        "sha256:{:x}",
        Sha256::digest(rmp_serde::to_vec_named(proposal)?)
    ))
}

async fn propose_strategic_individuation(
    model: &dyn ModelPort,
    campaign: &Campaign,
    selected_actions: &[crate::domain::CellActionProposal],
) -> (Vec<StrategicGestaltIndividuation>, Vec<ModelStageOutput>) {
    let candidates = selected_actions
        .iter()
        .filter_map(|action| {
            let gestalt = campaign.gestalts.get(&action.subject_id)?;
            let profile = campaign.agency_profiles.get(&action.subject_id)?;
            (profile.active_leaf && profile.simulation_eligible && profile.location_ids.len() == 1)
                .then(|| {
                    (
                        action,
                        gestalt,
                        profile.location_ids.iter().next().unwrap().clone(),
                    )
                })
        })
        .filter_map(|(action, gestalt, location_id)| {
            cell_action_digest(action)
                .ok()
                .map(|digest| (digest, action, gestalt, location_id))
        })
        .collect::<Vec<_>>();
    if candidates.is_empty() {
        return (Vec::new(), Vec::new());
    }
    let digests = candidates
        .iter()
        .map(|(digest, ..)| digest.clone())
        .collect::<Vec<_>>();
    let context = candidates
        .iter()
        .map(|(digest, action, gestalt, location_id)| {
            serde_json::json!({
                "action_digest":digest,
                "gestalt_id":gestalt.id,
                "gestalt_name":gestalt.name,
                "location_id":location_id,
                "goals":gestalt.goals,
                "pressures":gestalt.pressures,
                "selected_action":action,
            })
        })
        .collect::<Vec<_>>();
    let mut schema = match serde_json::to_value(schema_for!(StrategicPersonSelection)) {
        Ok(schema) => schema,
        Err(_) => return (Vec::new(), Vec::new()),
    };
    if let Some(value) = schema.pointer_mut("/$defs/StrategicPersonDraft/properties/action_digest")
    {
        *value = serde_json::json!({"type":"string","enum":digests});
    }
    if let Some(value) = schema.pointer_mut("/$defs/StrategicPersonDraft/properties/gestalt_id") {
        *value = serde_json::json!({"type":"string","enum":candidates.iter().map(|(_, _, gestalt, _)| gestalt.id.clone()).collect::<Vec<_>>()});
    }
    if let Some(proposals) = schema
        .pointer_mut("/properties/proposals")
        .and_then(serde_json::Value::as_object_mut)
    {
        proposals.insert("maxItems".into(), 1.into());
    }
    let request = ModelStageRequest {
        stage: "strategic_individuation_selector".into(),
        model: MODEL_FAST.into(),
        snapshot_binding: strategic_individuation_binding(campaign, &digests, None),
        lived_stream: format!(
            "OUTPUT JSON SCHEMA (follow exactly):\n{}\n\nReturn zero or one proposal. Choose one person only when a selected Gestalt action has created concrete political work that cannot remain anonymous: an envoy, organizer, claimant, conspirator, commander, broker, or dissident. Identity content is a proposal only. Use a short stable lowercase member_id without a member: prefix. Do not invent authority, location, or state beyond the supplied Gestalt. Return an empty proposals list when nobody needs to emerge.\nCANDIDATES:\n{}",
            serde_json::to_string(&schema).unwrap_or_default(),
            serde_json::to_string(&context).unwrap_or_default(),
        ),
        output_schema: Some(schema),
        source_receipt_ids: campaign.branch_origin.evidence_receipt_ids.clone(),
        temperature: Some(0.2),
        max_output_tokens: Some(768),
    };
    let Ok(mut output) = run_validated_stage(model, &request).await else {
        return (Vec::new(), Vec::new());
    };
    let Some(mut selection) = output
        .structured
        .clone()
        .and_then(|value| serde_json::from_value::<StrategicPersonSelection>(value).ok())
    else {
        return (Vec::new(), vec![output]);
    };
    if selection.proposals.is_empty() {
        return (Vec::new(), vec![output]);
    }
    if selection.proposals.len() != 1 {
        output.receipt.validation_result = "semantic_invalid".into();
        output.receipt.local_validation_error =
            Some("strategic selector exceeded its one-person budget".into());
        return (Vec::new(), vec![output]);
    }
    let draft = selection.proposals.remove(0);
    let Some((_, _, gestalt, location_id)) = candidates.iter().find(|(digest, _, gestalt, _)| {
        digest == &draft.action_digest && gestalt.id == draft.gestalt_id
    }) else {
        output.receipt.validation_result = "semantic_invalid".into();
        output.receipt.local_validation_error =
            Some("proposed person crossed the selected Gestalt action boundary".into());
        return (Vec::new(), vec![output]);
    };
    let member_id = crate::domain::canonical_gestalt_member_local_id(&draft.member_id);
    if member_id.is_empty()
        || draft.name.trim().is_empty()
        || campaign.gestalt_members.contains_key(&member_id)
    {
        output.receipt.validation_result = "semantic_invalid".into();
        output.receipt.local_validation_error =
            Some("proposed person has an empty or occupied identity".into());
        return (Vec::new(), vec![output]);
    }
    let member = GestaltMemberDelta {
        schema: "ghostlight.gestalt_member_delta.v1".into(),
        id: member_id,
        gestalt_id: gestalt.id.clone(),
        version: 0,
        name: draft.name,
        capability_additions: BTreeSet::new(),
        capability_removals: BTreeSet::new(),
        knowledge_additions: BTreeSet::new(),
        knowledge_removals: BTreeSet::new(),
        equipment: BTreeSet::new(),
        conditions: BTreeSet::new(),
        obligations: draft.obligations,
        relationships: draft.relationships,
        goals: draft.goals,
        memories: draft.memories,
        last_location_id: Some(location_id.clone()),
        materialized_actor_id: None,
        last_relevant_revision: campaign.revision,
        relevance_lease_until_revision: campaign.revision.saturating_add(4),
    };
    let proposal = StrategicGestaltIndividuation {
        schema: "ghostlight.strategic_gestalt_individuation.v1".into(),
        action_digest: draft.action_digest,
        rationale: draft.rationale,
        individuation: GestaltIndividuation {
            gestalt_id: gestalt.id.clone(),
            expected_gestalt_version: gestalt.version,
            member,
            location_id: location_id.clone(),
        },
    };
    let proposal_digest = match strategic_individuation_proposal_digest(&proposal) {
        Ok(digest) => digest,
        Err(_) => return (Vec::new(), vec![output]),
    };
    output
        .receipt
        .rebind_snapshot(strategic_individuation_binding(
            campaign,
            &digests,
            Some(&proposal_digest),
        ));
    (vec![proposal], vec![output])
}

fn distinct_model_receipt_hashes(stages: &[ModelStageOutput]) -> Vec<String> {
    let mut seen = BTreeSet::new();
    stages
        .iter()
        .filter_map(|stage| {
            let hash = stage.receipt.storage_key().to_owned();
            seen.insert(hash.clone()).then_some(hash)
        })
        .collect()
}

async fn project_resolution_demand(
    model: &dyn ModelPort,
    campaign: &Campaign,
    campaign_contract: Option<&CampaignContract>,
    aggregate_boundaries: &[AggregatedBoundary],
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
        "campaign_contract":campaign_contract,
        "aggregate_content_boundaries":aggregate_boundaries,
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
        model: MODEL_FAST.into(),
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
                axis_weights: projection.axis_weights.into_map(),
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
    let member_candidates = member_exceptions(campaign, cell)?;
    let decision_owner_ids = select_cell_decision_owners(campaign, cell, &member_candidates)?;
    let mut member_exceptions = member_candidates
        .into_iter()
        .filter(|member| decision_owner_ids.contains(&member.subject_id))
        .collect::<Vec<_>>();
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
        subject.activity_targets.retain(|target, _| {
            !target.starts_with("member:") || selected_member_ids.contains(target)
        });
    }
    for member in &mut member_exceptions {
        member.activity_targets.retain(|target, _| {
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
    let canonical_locations = constituents
        .iter()
        .flat_map(|subject| subject.location_ids.iter())
        .chain(
            member_exceptions
                .iter()
                .map(|member| &member.source_location_id),
        )
        .map(|location_id| {
            let location = campaign
                .locations
                .get(location_id)
                .ok_or_else(|| anyhow!("simulation cell location vanished from campaign"))?;
            Ok((location_id.clone(), location.name.clone()))
        })
        .collect::<Result<BTreeMap<_, _>>>()?;
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
        canonical_locations,
        detail_focus_subject_id: cell.detail_focus_subject_id.clone(),
        decision_owner_ids,
        max_actions: cell_action_limit(cell),
        source_receipt_ids: campaign.branch_origin.evidence_receipt_ids.clone(),
    })
}

fn select_cell_decision_owners(
    campaign: &Campaign,
    cell: &SimulationCell,
    member_candidates: &[CellMemberSlice],
) -> Result<BTreeSet<String>> {
    let quota = cell_action_limit(cell);
    if quota == 0 || cell.subject_ids.is_empty() {
        return Err(anyhow!("simulation cell has no bounded decision capacity"));
    }
    if let Some(focus) = cell.detail_focus_subject_id.as_deref()
        && !cell.subject_ids.contains(focus)
    {
        return Err(anyhow!("simulation cell detail focus is outside the cell"));
    }

    let canonical = cell.subject_ids.iter().cloned().collect::<Vec<_>>();
    let mut selected = BTreeSet::new();
    if let Some(focus) = cell.detail_focus_subject_id.as_ref() {
        selected.insert(focus.clone());
    }
    let available = canonical
        .iter()
        .filter(|subject_id| !selected.contains(*subject_id))
        .cloned()
        .collect::<Vec<_>>();
    let remaining = quota.saturating_sub(selected.len()).min(available.len());
    if remaining == available.len() {
        selected.extend(available);
    } else if remaining > 0 {
        let start =
            (campaign.strategic_tick_count as usize).saturating_mul(remaining) % available.len();
        for offset in 0..remaining {
            selected.insert(available[(start + offset) % available.len()].clone());
        }
    }

    let mut owners = BTreeSet::new();
    for subject_id in selected {
        if cell.detail_focus_subject_id.as_deref() == Some(subject_id.as_str())
            || !campaign.gestalts.contains_key(&subject_id)
        {
            owners.insert(subject_id);
            continue;
        }
        let mut alternatives = std::iter::once(subject_id.clone())
            .chain(
                member_candidates
                    .iter()
                    .filter(|member| member.source_gestalt_id == subject_id)
                    .map(|member| member.subject_id.clone()),
            )
            .collect::<Vec<_>>();
        alternatives.dedup();
        let index = campaign.strategic_tick_count as usize % alternatives.len();
        owners.insert(alternatives.swap_remove(index));
    }
    if owners.is_empty() || owners.len() > quota {
        return Err(anyhow!(
            "resolution produced an invalid exact decision-owner set"
        ));
    }
    Ok(owners)
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
                    subject_perceives_event(
                        &subject.subject_id,
                        &subject.location_ids,
                        &subject.information_channels,
                        event,
                    )
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

pub(crate) fn subject_perceives_event(
    subject_id: &str,
    location_ids: &BTreeSet<String>,
    information_channels: &BTreeSet<String>,
    event: &crate::domain::Event,
) -> bool {
    event.actor_ids.iter().any(|id| id == subject_id)
        || event.institution_ids.iter().any(|id| id == subject_id)
        || event.gestalt_ids.iter().any(|id| id == subject_id)
        || event
            .location_ids
            .iter()
            .any(|location_id| location_ids.contains(location_id))
        || event
            .public_channels
            .iter()
            .any(|channel| information_channels.contains(channel))
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
            let destinations = crate::resolution::gestalt_migration_destinations(
                campaign,
                &member.gestalt_id,
                &origin,
            );
            let activity_targets =
                crate::resolution::member_activity_targets(campaign, &member.id).ok()?;
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
                    crate::domain::gestalt_member_subject_id(&member.id),
                    crate::domain::gestalt_state_reference(&member.gestalt_id),
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
                    permitted_state_references
                        .insert(crate::domain::gestalt_state_reference(gestalt_id));
                    permitted_state_references.insert(format!("location:{location_id}"));
                }
                let migration_destinations = migration_destination_slices(campaign, &destinations)?;
                let activity_targets = activity_target_slices(campaign, &activity_target_ids)?;
                Ok(CellMemberSlice {
                    subject_id: crate::domain::gestalt_member_subject_id(&member.id),
                    member_id: member.id.clone(),
                    name: member.name.clone(),
                    source_gestalt_id: member.gestalt_id.clone(),
                    source_location_id: origin,
                    knowledge: knowledge.clone(),
                    capabilities,
                    resources: member.equipment.clone(),
                    information_channels,
                    permitted_state_references,
                    migration_destinations,
                    activity_targets,
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
    let activity_target_ids = crate::resolution::strategic_activity_targets(campaign, id);
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
        reachable_destinations: BTreeMap::new(),
        migration_destinations: BTreeMap::new(),
        activity_targets: activity_target_slices(campaign, &activity_target_ids)?,
        goals: vec![],
        relationships: BTreeMap::new(),
        memories: vec![],
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
            value.relationships = actor.relationships.clone();
            value.memories = actor.memories.clone();
            value.pressures = actor
                .conditions
                .iter()
                .chain(&actor.obligations)
                .cloned()
                .collect();
            value.reachable_destinations = campaign
                .locations
                .get(&actor.location_id)
                .into_iter()
                .flat_map(|location| location.routes.values())
                .filter(|route| route.travel_minutes <= campaign.tick_hours.saturating_mul(60))
                .map(|route| {
                    let destination = campaign
                        .locations
                        .get(&route.destination_id)
                        .ok_or_else(|| anyhow!("reachable destination vanished from topology"))?;
                    Ok((route.destination_id.clone(), destination.name.clone()))
                })
                .collect::<Result<BTreeMap<_, _>>>()?;
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
            let mut destinations = crate::resolution::gestalt_migration_destinations(
                campaign,
                id,
                &gestalt.home_location_id,
            );
            destinations.retain(|_, location_id| location_id != &gestalt.home_location_id);
            for (destination_id, location_id) in &destinations {
                value
                    .permitted_state_references
                    .insert(crate::domain::gestalt_state_reference(destination_id));
                value
                    .permitted_state_references
                    .insert(format!("location:{location_id}"));
            }
            value.migration_destinations = migration_destination_slices(campaign, &destinations)?;
        }
    }
    Ok(value)
}

fn activity_target_slices(
    campaign: &Campaign,
    target_ids: &BTreeSet<String>,
) -> Result<BTreeMap<String, CellActivityTargetSlice>> {
    target_ids
        .iter()
        .map(|target_id| {
            let (name, location_ids) =
                if let Some(profile) = campaign.agency_profiles.get(target_id) {
                    let name = campaign
                        .actors
                        .get(target_id)
                        .map(|actor| actor.name.clone())
                        .or_else(|| {
                            campaign
                                .institutions
                                .get(target_id)
                                .map(|institution| institution.name.clone())
                        })
                        .or_else(|| {
                            campaign
                                .gestalts
                                .get(target_id)
                                .map(|gestalt| gestalt.name.clone())
                        })
                        .ok_or_else(|| {
                            anyhow!("activity target {target_id} has no canonical named subject")
                        })?;
                    (name, profile.location_ids.clone())
                } else if let Some(member_id) =
                    crate::resolution::dormant_member_id_for_subject(campaign, target_id)
                {
                    let member = campaign.gestalt_members.get(member_id).ok_or_else(|| {
                        anyhow!("activity target {target_id} has no member state")
                    })?;
                    (
                        member.name.clone(),
                        BTreeSet::from([crate::resolution::dormant_member_location(
                            campaign, member_id,
                        )?]),
                    )
                } else {
                    return Err(anyhow!("activity target {target_id} has no agency profile"));
                };
            let locations = location_ids
                .into_iter()
                .map(|location_id| {
                    let location = campaign.locations.get(&location_id).ok_or_else(|| {
                        anyhow!("activity target location {location_id} vanished")
                    })?;
                    Ok((location_id, location.name.clone()))
                })
                .collect::<Result<BTreeMap<_, _>>>()?;
            Ok((
                target_id.clone(),
                CellActivityTargetSlice { name, locations },
            ))
        })
        .collect()
}

fn migration_destination_slices(
    campaign: &Campaign,
    destinations: &BTreeMap<String, String>,
) -> Result<BTreeMap<String, CellMigrationDestinationSlice>> {
    destinations
        .iter()
        .map(|(gestalt_id, location_id)| {
            let population = campaign
                .gestalts
                .get(gestalt_id)
                .ok_or_else(|| anyhow!("migration destination {gestalt_id} vanished"))?;
            let location = campaign
                .locations
                .get(location_id)
                .ok_or_else(|| anyhow!("migration destination location {location_id} vanished"))?;
            Ok((
                gestalt_id.clone(),
                CellMigrationDestinationSlice {
                    population_name: population.name.clone(),
                    location_id: location_id.clone(),
                    location_name: location.name.clone(),
                },
            ))
        })
        .collect()
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

    struct PersonFixtureModel;

    #[test]
    fn gestalt_state_references_qualify_canonical_ids_exactly_once() {
        assert_eq!(
            crate::domain::gestalt_state_reference("raincross_households"),
            "gestalt:raincross_households"
        );
        assert_eq!(
            crate::domain::gestalt_state_reference("gestalt:raincross_households"),
            "gestalt:raincross_households"
        );
    }

    #[test]
    fn demand_schema_preserves_all_six_weights_for_strict_providers() {
        let mut schema = serde_json::to_value(schema_for!(DemandProjection)).unwrap();
        crate::model_connector::project_strict_responses_schema(&mut schema).unwrap();
        let properties = schema["$defs"]["DemandAxisWeights"]["properties"]
            .as_object()
            .unwrap();
        assert_eq!(
            properties.keys().cloned().collect::<BTreeSet<_>>(),
            BTreeSet::from([
                "authority".into(),
                "economy_role".into(),
                "geography".into(),
                "ideology".into(),
                "information".into(),
                "species_body".into(),
            ])
        );
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
                    let subject_ids = request
                        .output_schema
                        .as_ref()
                        .and_then(|schema| {
                            schema
                                .pointer("/$defs/CellPerspectiveSegment/properties/subject_id/enum")
                        })
                        .and_then(serde_json::Value::as_array)
                        .map(|values| {
                            values
                                .iter()
                                .filter_map(serde_json::Value::as_str)
                                .collect::<Vec<_>>()
                        })
                        .filter(|values| !values.is_empty())
                        .ok_or_else(|| anyhow!("fixture projector lacks bound subjects"))?;
                    Ok(serde_json::json!({
                        "segments":subject_ids.into_iter().map(|subject_id| serde_json::json!({
                            "subject_id":subject_id,
                            "narrative":"The horizon tightens around this subject's own unresolved choice."
                        })).collect::<Vec<_>>()
                    })
                    .to_string())
                }
                "cell_persona" => Ok("Each constituent watches and deliberately holds.".into()),
                "cell_interpreter" if self.malformed_cell => Ok("not-json".into()),
                "cell_interpreter" => {
                    let subject_ids = request
                        .output_schema
                        .as_ref()
                        .and_then(|schema| {
                            schema
                                .pointer("/properties/decisions/properties")
                                .and_then(serde_json::Value::as_object)
                        })
                        .map(|properties| properties.keys().cloned().collect::<Vec<_>>())
                        .filter(|subject_ids| !subject_ids.is_empty())
                        .ok_or_else(|| anyhow!("fixture Interpreter lacks a bound subject"))?;
                    let decisions = subject_ids
                        .into_iter()
                        .map(|subject_id| {
                            (
                                subject_id.clone(),
                                serde_json::json!({"inaction":{
                                    "subject_id":subject_id,
                                    "reason":"No justified move this horizon."
                                }}),
                            )
                        })
                        .collect::<serde_json::Map<_, _>>();
                    Ok(serde_json::json!({
                        "decisions":decisions
                    })
                    .to_string())
                }
                stage => Err(anyhow!("unexpected fixture stage {stage}")),
            }
        }

        fn provider(&self) -> &'static str {
            "fixture"
        }
    }

    #[async_trait]
    impl ModelPort for PersonFixtureModel {
        async fn run(&self, request: &ModelStageRequest) -> Result<String> {
            let digest = request
                .output_schema
                .as_ref()
                .and_then(|schema| {
                    schema.pointer("/$defs/StrategicPersonDraft/properties/action_digest/enum/0")
                })
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| anyhow!("person fixture lacks action digest"))?;
            let gestalt_id = request
                .output_schema
                .as_ref()
                .and_then(|schema| {
                    schema.pointer("/$defs/StrategicPersonDraft/properties/gestalt_id/enum/0")
                })
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| anyhow!("person fixture lacks Gestalt"))?;
            Ok(serde_json::json!({"proposals":[{
                "action_digest":digest,
                "gestalt_id":gestalt_id,
                "member_id":"veska-rill",
                "name":"Veska Rill",
                "goals":["control the grain delegation"],
                "obligations":["answer to the river wards"],
                "relationships":{},
                "memories":["the lower road vanished after the dwarven excavation"],
                "rationale":"The delegation needs one accountable broker."
            }]})
            .to_string())
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
    fn canonical_actor_cell_slice_preserves_memory_and_relationship_context() {
        let mut campaign = crate::resolution::tests::campaign(0, 1);
        let expected_goals = vec!["keep the twelve patients together".into()];
        let expected_memories = vec!["Promised Ash to stay with the twelve.".into()];
        let expected_relationships =
            BTreeMap::from([("ash".into(), "trusted with a responsibility".into())]);
        {
            let actor = campaign.actors.get_mut("player").unwrap();
            actor.goals = expected_goals.clone();
            actor.memories = expected_memories.clone();
            actor.relationships = expected_relationships.clone();
        }

        let slice = constituent_slice(&campaign, "player").unwrap();
        assert_eq!(slice.goals, expected_goals);
        assert_eq!(slice.memories, expected_memories);
        assert_eq!(slice.relationships, expected_relationships);
    }

    #[test]
    fn actor_slice_names_reachable_places_and_a_target_already_at_the_current_location() {
        use crate::domain::{ActorState, Location, Route};

        let mut campaign = crate::resolution::tests::campaign(0, 1);
        campaign.actors.insert(
            "reed".into(),
            ActorState {
                id: "reed".into(),
                name: "Reed".into(),
                location_id: "center".into(),
                capabilities: BTreeSet::new(),
                knowledge: BTreeSet::new(),
                equipment: BTreeSet::new(),
                conditions: BTreeSet::new(),
                obligations: BTreeSet::new(),
                relationships: BTreeMap::new(),
                goals: vec!["keep the twelve together".into()],
                memories: vec![],
            },
        );
        campaign.locations.insert(
            "garrison".into(),
            Location {
                id: "garrison".into(),
                name: "Garrison Outpost".into(),
                container_id: None,
                routes: BTreeMap::new(),
                persistent_features: vec![],
            },
        );
        campaign.locations.get_mut("center").unwrap().routes.insert(
            "garrison".into(),
            Route {
                destination_id: "garrison".into(),
                distance: "3 km".into(),
                travel_minutes: 20,
            },
        );
        crate::resolution::ensure_agency_profiles(&mut campaign);

        let slice = constituent_slice(&campaign, "player").unwrap();
        assert_eq!(
            slice
                .reachable_destinations
                .get("garrison")
                .map(String::as_str),
            Some("Garrison Outpost")
        );
        let reed = slice.activity_targets.get("reed").unwrap();
        assert_eq!(reed.name, "Reed");
        assert_eq!(
            reed.locations.get("center").map(String::as_str),
            Some("Center")
        );
        assert!(!reed.locations.contains_key("garrison"));
    }

    #[test]
    fn materialized_gestalt_member_is_projected_as_an_actor_target() {
        use crate::domain::{ActorState, GestaltMemberDelta, GestaltPersonaState};

        let mut campaign = crate::resolution::tests::campaign(0, 2);
        campaign.gestalts.insert(
            "refugees".into(),
            GestaltPersonaState {
                schema: "ghostlight.gestalt_persona_state.v1".into(),
                id: "refugees".into(),
                name: "Refugees".into(),
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
            "sable".into(),
            GestaltMemberDelta {
                schema: "ghostlight.gestalt_member_delta.v1".into(),
                id: "sable".into(),
                gestalt_id: "refugees".into(),
                version: 1,
                name: "Sable".into(),
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
                materialized_actor_id: Some("member:sable".into()),
                last_relevant_revision: 0,
                relevance_lease_until_revision: 5,
            },
        );
        campaign.actors.insert(
            "member:sable".into(),
            ActorState {
                id: "member:sable".into(),
                name: "Sable".into(),
                location_id: "center".into(),
                capabilities: BTreeSet::from(["route scouting".into()]),
                knowledge: BTreeSet::new(),
                equipment: BTreeSet::new(),
                conditions: BTreeSet::new(),
                obligations: BTreeSet::new(),
                relationships: BTreeMap::new(),
                goals: vec![],
                memories: vec![],
            },
        );
        crate::resolution::ensure_agency_profiles(&mut campaign);

        let slice = constituent_slice(&campaign, "player").unwrap();
        let sable = slice.activity_targets.get("member:sable").unwrap();
        assert_eq!(sable.name, "Sable");
        assert_eq!(
            sable.locations.get("center").map(String::as_str),
            Some("Center")
        );
    }

    #[test]
    fn cell_slice_rotates_exact_member_decision_owners_without_hiding_the_cell() {
        use crate::domain::*;
        let mut campaign = crate::resolution::tests::campaign(0, 1);
        campaign.locations.insert(
            "unsliced".into(),
            Location {
                id: "unsliced".into(),
                name: "Remote Unsliced Court".into(),
                container_id: None,
                routes: BTreeMap::new(),
                persistent_features: vec![],
            },
        );
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
        campaign.strategic_tick_count = 1;
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
        assert_eq!(
            slice.canonical_locations.get("center").map(String::as_str),
            Some("Center")
        );
        assert!(!slice.canonical_locations.contains_key("unsliced"));
        assert_eq!(slice.member_exceptions.len(), 1);
        assert_eq!(slice.member_exceptions[0].member_id, "mira");
        assert!(slice.decision_owner_ids.contains("member:mira"));
        assert!(slice.decision_owner_ids.contains("neighbors"));
        assert!(!slice.decision_owner_ids.contains("refugees"));
        assert_eq!(
            slice.member_exceptions[0]
                .migration_destinations
                .get("neighbors")
                .map(|destination| destination.location_id.as_str()),
            Some("center")
        );
        assert_eq!(
            slice.member_exceptions[0]
                .migration_destinations
                .get("neighbors")
                .map(|destination| destination.population_name.as_str()),
            Some("Neighbors")
        );
        assert!(
            slice.member_exceptions[0]
                .permitted_state_references
                .contains("member:mira")
        );
        assert!(
            slice.member_exceptions[0]
                .activity_targets
                .contains_key("refugees")
        );
        assert_eq!(
            slice.member_exceptions[0]
                .activity_targets
                .get("refugees")
                .map(|target| target.name.as_str()),
            Some("Refugees")
        );
        assert_eq!(
            slice.member_exceptions[0]
                .activity_targets
                .get("refugees")
                .and_then(|target| target.locations.get("center"))
                .map(String::as_str),
            Some("Center")
        );
        let selected_member_ids = slice
            .member_exceptions
            .iter()
            .map(|member| member.subject_id.clone())
            .collect::<BTreeSet<_>>();
        assert!(selected_member_ids.contains("member:mira"));
        assert!(slice.constituents.iter().all(|constituent| {
            constituent
                .activity_targets
                .keys()
                .filter(|target| target.starts_with("member:"))
                .all(|target| selected_member_ids.contains(target))
        }));
        assert!(slice.member_exceptions.iter().all(|member| {
            member
                .activity_targets
                .keys()
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

        let mut focused_cell = cover.cells[0].clone();
        focused_cell.detail_focus_subject_id = Some("refugees".into());
        let focused = cell_slice(&campaign, &focused_cell).unwrap();
        assert!(focused.decision_owner_ids.contains("refugees"));
        assert!(!focused.decision_owner_ids.contains("member:mira"));

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
                    && member.activity_targets.contains_key("refugees"))
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
            activity_targets: BTreeMap::new(),
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
        assert_eq!(
            output.wave.model_receipt_hashes.len(),
            output
                .wave
                .model_receipt_hashes
                .iter()
                .collect::<BTreeSet<_>>()
                .len()
        );
        let mut repeated_stage = output.stages.clone();
        repeated_stage.push(output.stages[0].clone());
        assert_eq!(
            distinct_model_receipt_hashes(&repeated_stage),
            output.wave.model_receipt_hashes
        );
        assert!(model.maximum.load(Ordering::SeqCst) <= 2);
        validate_and_resolve_wave(&campaign, &output.wave).unwrap();
    }

    #[tokio::test]
    async fn two_hundred_cell_wave_dispatches_in_parallel_under_one_provider_gate() {
        let mut campaign = crate::resolution::tests::campaign(1_000, 200);
        campaign.resolution_policy.provider_parallelism = 7;
        let model = Arc::new(CellFixtureModel {
            active: AtomicUsize::new(0),
            maximum: AtomicUsize::new(0),
            malformed_cell: false,
        });
        let output = propose_resolution_wave(model.clone(), Arc::new(AllowAllPermit), &campaign)
            .await
            .unwrap();
        assert_eq!(output.wave.cover.cells.len(), 200);
        assert_eq!(output.wave.appraisals.len(), 200);
        assert_eq!(output.stages.len(), 601);
        let maximum = model.maximum.load(Ordering::SeqCst);
        assert!(maximum > 1, "the wave never dispatched concurrently");
        assert!(
            maximum <= 7,
            "provider concurrency escaped its gate: {maximum}"
        );
        validate_and_resolve_wave(&campaign, &output.wave).unwrap();
    }

    #[tokio::test]
    async fn strategic_gestalt_pressure_can_propose_one_action_bound_named_person() {
        use crate::domain::{AgencySubjectKind, GestaltPersonaState, StrategicCellEffect};
        let mut campaign = crate::resolution::tests::campaign(1, 1);
        let mut profile = campaign.agency_profiles["faction-0000"].clone();
        profile.subject_id = "river-wards".into();
        profile.subject_kind = AgencySubjectKind::Gestalt;
        profile.location_ids = BTreeSet::from(["center".into()]);
        let location_id = profile.location_ids.iter().next().unwrap().clone();
        campaign
            .agency_profiles
            .insert(profile.subject_id.clone(), profile);
        campaign.gestalts.insert(
            "river-wards".into(),
            GestaltPersonaState {
                schema: "ghostlight.gestalt_persona_state.v1".into(),
                id: "river-wards".into(),
                name: "River Wards".into(),
                version: 0,
                home_location_id: location_id.clone(),
                shared_capabilities: BTreeSet::new(),
                shared_knowledge: BTreeSet::new(),
                resources: BTreeSet::from(["grain barges".into()]),
                goals: vec!["keep the river wards fed".into()],
                pressures: vec!["dwarven excavation diverted the lower road".into()],
            },
        );
        let action = crate::domain::CellActionProposal {
            subject_id: "river-wards".into(),
            intent: "Send a grain delegation around the broken lower road.".into(),
            intended_effect: "Negotiate a politically accountable detour.".into(),
            priority: 80,
            state_references: vec![],
            public_channels: vec![],
            effects: vec![StrategicCellEffect::Gestalt {
                gestalt_id: "river-wards".into(),
                pressure_additions: vec!["the delegation needs an accountable broker".into()],
                pressure_resolutions: vec![],
            }],
        };
        let digest = cell_action_digest(&action).unwrap();
        let institution_action = crate::domain::CellActionProposal {
            subject_id: "faction-0000".into(),
            intent: "Publish the existing ration posture.".into(),
            intended_effect: "Keep the institution legible.".into(),
            priority: 20,
            state_references: vec![],
            public_channels: vec![],
            effects: vec![StrategicCellEffect::Institution {
                institution_id: "faction-0000".into(),
                posture: "publishing ration posture".into(),
                location_ids: vec![],
            }],
        };
        let selected_actions = vec![action, institution_action];
        let (proposals, stages) =
            propose_strategic_individuation(&PersonFixtureModel, &campaign, &selected_actions)
                .await;
        assert_eq!(proposals.len(), 1);
        assert_eq!(stages.len(), 1);
        assert_eq!(proposals[0].action_digest, digest);
        assert_eq!(proposals[0].individuation.gestalt_id, "river-wards");
        assert_eq!(proposals[0].individuation.location_id, location_id);
        assert_eq!(proposals[0].individuation.member.name, "Veska Rill");
        assert_eq!(proposals[0].individuation.expected_gestalt_version, 0);
        let candidate_digests =
            strategic_individuation_candidate_digests(&campaign, &selected_actions);
        assert_eq!(candidate_digests, vec![digest]);
        let proposal_digest = strategic_individuation_proposal_digest(&proposals[0]).unwrap();
        assert_eq!(
            stages[0].receipt.snapshot_binding,
            strategic_individuation_binding(&campaign, &candidate_digests, Some(&proposal_digest),)
        );
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
