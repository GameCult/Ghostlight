use crate::{
    domain::{
        AgencyAxis, AgencySubjectKind, Campaign, ResolutionDemand, ResolutionWaveCommit,
        SimulationCell,
    },
    model::{ModelPort, ModelStageOutput, ModelStageRequest, run_validated_stage},
    persona::{CellConstituentSlice, CellProjectionEngine, ExecutionPermit, PermittedCellSlice},
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

pub struct StrategicResolutionOutput {
    pub wave: ResolutionWaveCommit,
    pub stages: Vec<ModelStageOutput>,
    pub aggregate_receipt_hash: String,
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
    let active_ids: Vec<_> = campaign
        .agency_profiles
        .values()
        .filter(|profile| profile.active_leaf && profile.simulation_eligible)
        .map(|profile| profile.subject_id.clone())
        .collect();
    let context = serde_json::json!({
        "campaign_id": campaign.id,
        "world_revision": campaign.revision,
        "resolution_epoch": campaign.resolution_policy.resolution_epoch,
        "horizon_minutes": campaign.tick_hours.saturating_mul(60),
        "allowed_subject_ids": active_ids,
        "clocks": campaign.clocks,
        "recent_events": campaign.events.iter().rev().take(12).collect::<Vec<_>>(),
        "relations": campaign.agency_relations.values().filter(|relation| relation.active).collect::<Vec<_>>(),
        "profiles": campaign.agency_profiles.values().filter(|profile| profile.active_leaf && profile.simulation_eligible).collect::<Vec<_>>()
    });
    let mut schema = match serde_json::to_value(schema_for!(DemandProjection)) {
        Ok(value) => value,
        Err(_) => return (fallback(), vec![]),
    };
    if let Some(items) = schema.pointer_mut("/properties/focal_subject_ids/items") {
        *items = serde_json::json!({"type":"string","enum":active_ids});
    }
    let base_prompt = format!(
        "Project which boundaries best predict different strategic behavior for the next horizon. Return all six axis weights (geography, ideology, authority, economy_role, species_body, information), each from 0 to 1 and summing to 1. Focal subjects may only use supplied IDs. This stage proposes relevance and cannot change world state. Return one JSON object matching the schema.\nSTATE:\n{}\nOUTPUT JSON SCHEMA:\n{}",
        serde_json::to_string(&context).unwrap_or_default(),
        serde_json::to_string_pretty(&schema).unwrap_or_default(),
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
            if validate_demand(campaign, &demand).is_ok() {
                outputs.push(output);
                return (demand, outputs);
            }
        }
        output.receipt.validation_result = "semantic_invalid".into();
        outputs.push(output);
        if attempt == 0 {
            request.lived_stream.push_str(
                "\n\nLOCAL VALIDATOR: The prior projection omitted an axis, used an unknown subject, had invalid weights, failed to sum to one, or had no rationale. Return one corrected object against the same snapshot.",
            );
        }
    }
    (fallback(), outputs)
}

fn cell_slice(campaign: &Campaign, cell: &SimulationCell) -> Result<PermittedCellSlice> {
    let constituents = cell
        .subject_ids
        .iter()
        .map(|id| constituent_slice(campaign, id))
        .collect::<Result<Vec<_>>>()?;
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
        shared_knowledge,
        shared_capabilities,
        perceived_events: campaign
            .events
            .iter()
            .rev()
            .take(12)
            .map(|event| event.summary.clone())
            .collect(),
        world_clock_pressure: campaign
            .clocks
            .values()
            .map(|clock| {
                format!(
                    "{}: {}/{}; consequence {}",
                    clock.label, clock.progress, clock.threshold, clock.consequence
                )
            })
            .collect(),
        detail_focus_subject_id: cell.detail_focus_subject_id.clone(),
        max_actions: cell_action_limit(cell),
        source_receipt_ids: campaign.branch_origin.evidence_receipt_ids.clone(),
    })
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
        goals: vec![],
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
            value.pressures = vec![institution.posture.clone()];
        }
        AgencySubjectKind::Gestalt => {
            let gestalt = campaign.gestalts.get(id).context("cell gestalt vanished")?;
            value.name = gestalt.name.clone();
            value.knowledge = gestalt.shared_knowledge.clone();
            value.capabilities = gestalt.shared_capabilities.clone();
            value.resources = gestalt.resources.clone();
            value.goals = gestalt.goals.clone();
            value.pressures = gestalt.pressures.clone();
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
                "cell_projector" => Ok("Several powers feel the horizon tightening.".into()),
                "cell_persona" => Ok("Each constituent watches and deliberately holds.".into()),
                "cell_interpreter" if self.malformed_cell => Ok("not-json".into()),
                "cell_interpreter" => {
                    let context = request
                        .lived_stream
                        .split("Permissioned typed context:\n")
                        .nth(1)
                        .and_then(|value| value.split("\n\nLived stream:").next())
                        .context("fixture could not locate typed cell context")?;
                    let slice: PermittedCellSlice = serde_json::from_str(context)?;
                    Ok(serde_json::to_string(&crate::domain::CellAppraisal {
                        schema: "ghostlight.cell_appraisal.v1".into(),
                        cell_id: slice.cell_id,
                        world_revision: slice.world_revision,
                        resolution_epoch: slice.resolution_epoch,
                        considered_subject_ids: slice
                            .constituents
                            .into_iter()
                            .map(|value| value.subject_id)
                            .collect(),
                        actions: vec![],
                        inaction_reason: Some(
                            "No constituent has a justified move this horizon.".into(),
                        ),
                    })?)
                }
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
