#[cfg(not(windows))]
fn main() -> anyhow::Result<()> {
    anyhow::bail!("the live gestalt dynamics smoke uses Starfire's DPAPI credential")
}

#[cfg(windows)]
struct LiveFireModelRecorder {
    inner: ghostlight_dungeon::model::DeepSeekPort,
    calls: std::sync::Arc<std::sync::Mutex<Vec<serde_json::Value>>>,
}

#[cfg(windows)]
#[async_trait::async_trait]
impl ghostlight_dungeon::model::ModelPort for LiveFireModelRecorder {
    async fn run(
        &self,
        request: &ghostlight_dungeon::model::ModelStageRequest,
    ) -> anyhow::Result<String> {
        Ok(self.run_observed(request).await?.content)
    }

    async fn run_observed(
        &self,
        request: &ghostlight_dungeon::model::ModelStageRequest,
    ) -> anyhow::Result<ghostlight_dungeon::model::ModelProviderOutput> {
        let started = std::time::Instant::now();
        let result = self.inner.run_observed(request).await;
        let trace = match &result {
            Ok(output) => serde_json::json!({
                "stage":request.stage,
                "model":request.model,
                "snapshot_binding":request.snapshot_binding,
                "input":request.lived_stream,
                "output":output.content,
                "provider_request_id":output.provider_request_id,
                "system_fingerprint":output.system_fingerprint,
                "finish_reason":output.finish_reason,
                "token_usage":output.token_usage,
                "latency_ms":started.elapsed().as_millis(),
            }),
            Err(error) => serde_json::json!({
                "stage":request.stage,
                "model":request.model,
                "snapshot_binding":request.snapshot_binding,
                "input":request.lived_stream,
                "error":error.to_string(),
                "latency_ms":started.elapsed().as_millis(),
            }),
        };
        self.calls.lock().expect("live-fire trace lock").push(trace);
        result
    }

    fn provider(&self) -> &'static str {
        self.inner.provider()
    }
}

#[cfg(windows)]
fn write_wave_failure(
    root: &std::path::Path,
    wave_index: usize,
    pulse_attempt: usize,
    budget: u8,
    error: &anyhow::Error,
    calls: &std::sync::Arc<std::sync::Mutex<Vec<serde_json::Value>>>,
    trace_start: usize,
) -> anyhow::Result<()> {
    let calls = calls.lock().expect("live-fire trace lock");
    std::fs::write(
        root.join(format!(
            "sustained-wave-{wave_index:02}-pulse-{pulse_attempt:02}-failure.json"
        )),
        serde_json::to_vec_pretty(&serde_json::json!({
            "schema":"ghostlight.live_fire_model_failure.v1",
            "wave_index":wave_index,
            "pulse_attempt":pulse_attempt,
            "budget":budget,
            "error":error.to_string(),
            "private_model_calls":&calls[trace_start..],
        }))?,
    )?;
    Ok(())
}

#[cfg(windows)]
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    use chrono::Utc;
    use ghostlight_dungeon::{
        domain::{SimulationCellMode, TickSource, WorldCommand},
        gestalt::GestaltPresencePlanner,
        kernel::{CommandResult, WorldKernel},
        model::{DeepSeekPort, ModelPort},
        persistence::CampaignStore,
        resolution::{
            effective_member_capabilities, effective_member_knowledge, validate_and_resolve_wave,
        },
        scheduler::propose_resolution_wave,
        turn::SnapshotPermit,
    };
    use sha2::{Digest, Sha256};
    use std::{collections::BTreeSet, path::PathBuf, sync::Arc, time::Instant};

    let secret = std::env::var_os("GHOSTLIGHT_DEEPSEEK_BLOB")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(r"F:\GameCult\GhostlightDungeon\secrets\deepseek.dpapi"));
    let scenario_id = std::env::var("GHOSTLIGHT_LIVE_FIRE_SCENARIO")
        .unwrap_or_else(|_| "gestalt-dynamics-refugee-return".into());
    let require_migration = std::env::var("GHOSTLIGHT_LIVE_FIRE_REQUIRE_MIGRATION")
        .is_ok_and(|value| value == "1" || value.eq_ignore_ascii_case("true"));
    let baseline_result_path =
        std::env::var_os("GHOSTLIGHT_LIVE_FIRE_BASELINE_RESULT").map(PathBuf::from);
    if require_migration && baseline_result_path.is_some() {
        anyhow::bail!("strict live migration and a committed migration baseline are exclusive")
    }
    let fairness_stress_waves = std::env::var("GHOSTLIGHT_LIVE_FIRE_FAIRNESS_WAVES")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|value| (1..=31).contains(value))
        .unwrap_or_default();
    let presence_only = std::env::var("GHOSTLIGHT_LIVE_FIRE_PRESENCE_ONLY")
        .is_ok_and(|value| value == "1" || value.eq_ignore_ascii_case("true"));
    if presence_only && baseline_result_path.is_none() {
        anyhow::bail!("presence-only live fire requires a committed migration baseline")
    }
    let fairness_stress_budget = std::env::var("GHOSTLIGHT_LIVE_FIRE_STRESS_BUDGET")
        .ok()
        .and_then(|value| value.parse::<u8>().ok())
        .filter(|value| (1..=32).contains(value))
        .unwrap_or(1);
    let stress_provider_parallelism = std::env::var("GHOSTLIGHT_LIVE_FIRE_PROVIDER_PARALLELISM")
        .ok()
        .and_then(|value| value.parse::<u8>().ok())
        .filter(|value| (1..=32).contains(value));
    let max_rejected_pulses_per_wave =
        std::env::var("GHOSTLIGHT_LIVE_FIRE_MAX_REJECTED_PULSES_PER_WAVE")
            .ok()
            .and_then(|value| value.parse::<usize>().ok())
            .filter(|value| *value <= 8)
            .unwrap_or_default();
    let root = std::env::var_os("GHOSTLIGHT_LIVE_FIRE_RESULT_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(r"F:\GameCult\GhostlightDungeon\acceptance").join(format!(
                "gestalt-dynamics-{}-{}",
                Utc::now().format("%Y%m%d-%H%M%S"),
                uuid::Uuid::new_v4()
            ))
        });
    std::fs::create_dir_all(&root)?;
    let base_campaign = dynamics_campaign();
    let source_lineage_depth = lineage_depth(&base_campaign, "refugees-east")?;
    let destination_lineage_depth = lineage_depth(&base_campaign, "harbor-neighbors")?;
    if source_lineage_depth < 2 || destination_lineage_depth < 2 {
        anyhow::bail!("refugee callback fixture lost its nested lineage depth")
    }
    let player_before = base_campaign.actors[&base_campaign.player_actor_id].clone();
    let member_before = base_campaign.gestalt_members["mira-venn"].clone();
    let capabilities_before = effective_member_capabilities(&base_campaign, "mira-venn")?;
    let knowledge_before = effective_member_knowledge(&base_campaign, "mira-venn")?;
    let (campaign, baseline_receipt) = if let Some(path) = baseline_result_path {
        let bytes = std::fs::read(&path)?;
        let result: serde_json::Value = serde_json::from_slice(&bytes)?;
        let campaign: ghostlight_dungeon::domain::Campaign =
            serde_json::from_value(result.pointer("/commit/campaign").cloned().ok_or_else(
                || anyhow::anyhow!("baseline result lacks committed campaign state"),
            )?)?;
        let member = campaign
            .gestalt_members
            .get("mira-venn")
            .ok_or_else(|| anyhow::anyhow!("baseline result lost Mira"))?;
        if member.gestalt_id != "harbor-neighbors"
            || member.last_location_id.as_deref() != Some("south-harbor")
            || member.materialized_actor_id.is_some()
            || member.name != member_before.name
            || member.relationships != member_before.relationships
            || member.memories != member_before.memories
            || effective_member_capabilities(&campaign, "mira-venn")? != capabilities_before
            || effective_member_knowledge(&campaign, "mira-venn")? != knowledge_before
            || campaign.actors[&campaign.player_actor_id] != player_before
            || campaign.resolution_cover.as_ref().is_none_or(|cover| {
                cover.cells.len() != 1
                    || cover.cells[0].mode != SimulationCellMode::Arena
                    || cover.cells[0].subject_ids.len() != 24
            })
        {
            anyhow::bail!("baseline result is not the committed strict migration golden")
        }
        (
            campaign,
            Some(serde_json::json!({
                "path":path,
                "sha256":format!("sha256:{:x}", Sha256::digest(&bytes)),
                "scenario_id":result.get("scenario_id"),
                "campaign_revision":result.pointer("/commit/campaign/revision"),
            })),
        )
    } else {
        (base_campaign, None)
    };
    let store = CampaignStore::open(root.join("campaign.cc"))?;
    store.create_campaign(&campaign, &[], &[])?;
    let kernel = WorldKernel::start(store.clone());
    let model_calls = Arc::new(std::sync::Mutex::new(Vec::new()));
    let model: Arc<dyn ModelPort> = Arc::new(LiveFireModelRecorder {
        inner: DeepSeekPort::from_machine_dpapi(secret)?,
        calls: model_calls.clone(),
    });
    let started = Instant::now();
    let output = match propose_resolution_wave(
        model.clone(),
        Arc::new(SnapshotPermit::new_resolution(
            store.clone(),
            campaign.id,
            campaign.revision,
            campaign.resolution_policy.resolution_epoch,
        )),
        &campaign,
    )
    .await
    {
        Ok(output) => output,
        Err(error) => {
            write_wave_failure(&root, 0, 1, 1, &error, &model_calls, 0)?;
            return Err(error);
        }
    };
    let preflight = serde_json::json!({
        "schema":"ghostlight.gestalt_dynamics_preflight.v1",
        "scenario_id":scenario_id,
        "elapsed_seconds":started.elapsed().as_secs_f64(),
        "cover":&output.wave.cover,
        "appraisals":&output.wave.appraisals,
        "activity_outcomes":&output.wave.activity_outcomes,
        "model_stage_receipts":output.stages.iter().map(|stage|&stage.receipt).collect::<Vec<_>>(),
        "private_cell_traces":&output.private_cell_traces,
        "private_model_stage_outputs":output.stages.iter().map(|stage|serde_json::json!({
            "stage":stage.receipt.stage,
            "validation_result":stage.receipt.validation_result,
            "local_validation_error":stage.receipt.local_validation_error,
            "narrative":stage.narrative,
            "structured":stage.structured,
        })).collect::<Vec<_>>()
    });
    std::fs::write(
        root.join("preflight.json"),
        serde_json::to_vec_pretty(&preflight)?,
    )?;

    if output.wave.cover.cells.len() != 1
        || output.wave.cover.cells[0].mode != SimulationCellMode::Arena
    {
        anyhow::bail!("budget-one whole-setting cover was not one arena")
    }
    let root_cell = &output.wave.cover.cells[0];
    if !root_cell.subject_ids.contains("refugees-east")
        || !root_cell.subject_ids.contains("harbor-neighbors")
        || root_cell.subject_ids.len() != 24
    {
        anyhow::bail!("whole-setting arena lost a rival population or canonical subject")
    }
    if root_cell.subject_ids.iter().any(|subject_id| {
        campaign
            .agency_profiles
            .get(subject_id)
            .is_none_or(|profile| !profile.active_leaf || !profile.simulation_eligible)
    }) {
        anyhow::bail!("whole-setting arena simulated an inactive lineage parent")
    }
    let migration_proposal = output
        .wave
        .appraisals
        .iter()
        .flat_map(|appraisal| &appraisal.actions)
        .find(|proposal| {
            proposal.subject_id == "member:mira-venn"
                && matches!(
                    &proposal.effect,
                    ghostlight_dungeon::domain::StrategicCellEffect::MemberMigration {
                        destination_gestalt_id,
                    } if destination_gestalt_id == "harbor-neighbors"
                )
        })
        .cloned();
    if require_migration && migration_proposal.is_none() {
        anyhow::bail!("strict migration golden did not receive Mira's attributed migration choice")
    }
    if output.wave.appraisals.iter().any(|appraisal| {
        appraisal
            .actions
            .iter()
            .any(|proposal| proposal.subject_id == root_cell.id)
    }) {
        anyhow::bail!("arena emitted an action as if it were a person")
    }
    let plan = validate_and_resolve_wave(&campaign, &output.wave)?;
    if plan.member_migrations.len() != usize::from(migration_proposal.is_some()) {
        anyhow::bail!("validated wave changed the attributed member choice")
    }
    let background_action_count = plan.institution_actions.len()
        + plan.gestalt_actions.len()
        + plan.gestalt_activities.len()
        + plan.gestalt_migrations.len()
        + plan.actor_moves.len()
        + plan.member_activities.len();
    for stage in &output.stages {
        store.insert(
            "persona_stage_receipt.v1",
            "ghostlight.persona_stage_receipt.v1",
            stage.receipt.storage_key(),
            &stage.receipt,
        )?;
    }
    let committed = kernel
        .command(WorldCommand::AdvanceStrategicTick {
            expected_revision: campaign.revision,
            source: TickSource::Scheduler,
            plan: None,
            model_receipt_hash: Some(output.aggregate_receipt_hash.clone()),
            resolution_wave: Some(output.wave.clone()),
        })
        .await?;
    let mut advanced = match &committed {
        CommandResult::Committed { campaign, .. } => campaign.clone(),
        _ => anyhow::bail!("gestalt dynamics wave did not commit"),
    };
    if advanced.actors[&advanced.player_actor_id] != player_before {
        anyhow::bail!("background simulation puppeted the player")
    }
    if migration_proposal.is_none() && baseline_receipt.is_none() {
        if advanced.gestalt_members["mira-venn"] != member_before {
            anyhow::bail!("explicit inaction changed Mira's dormant identity")
        }
        let result = serde_json::json!({
            "schema":"ghostlight.gestalt_dynamics_smoke.v1",
            "scenario_id":scenario_id,
            "elapsed_seconds":started.elapsed().as_secs_f64(),
            "subject_count":root_cell.subject_ids.len(),
            "configured_budget":campaign.resolution_policy.active_cell_budget,
            "effective_budget":output.wave.cover.effective_budget,
            "cell_mode":root_cell.mode,
            "rivals_share_arena":true,
            "source_lineage_depth":source_lineage_depth,
            "destination_lineage_depth":destination_lineage_depth,
            "choice":"explicit_inaction",
            "migration_proposal":serde_json::Value::Null,
            "initial_background_action_count":background_action_count,
            "plan":plan,
            "commit":committed,
            "identity_preserved":true,
            "player_unchanged":true,
            "private_cell_traces":output.private_cell_traces,
            "model_stage_receipts":output.stages.iter().map(|stage|&stage.receipt).collect::<Vec<_>>(),
            "store":root.join("campaign.cc"),
            "preflight_path":root.join("preflight.json"),
            "result_path":root.join("result.json")
        });
        std::fs::write(
            root.join("result.json"),
            serde_json::to_vec_pretty(&result)?,
        )?;
        println!("{}", serde_json::to_string_pretty(&result)?);
        return Ok(());
    }
    let member_after = &advanced.gestalt_members["mira-venn"];
    if member_after.gestalt_id != "harbor-neighbors"
        || member_after.last_location_id.as_deref() != Some("south-harbor")
        || member_after.name != member_before.name
        || member_after.relationships != member_before.relationships
        || member_after.memories != member_before.memories
        || member_after.equipment != member_before.equipment
        || member_after.conditions != member_before.conditions
        || member_after.obligations != member_before.obligations
        || effective_member_capabilities(&advanced, "mira-venn")? != capabilities_before
        || effective_member_knowledge(&advanced, "mira-venn")? != knowledge_before
    {
        anyhow::bail!("migration changed Mira's identity or effective personal state")
    }
    let mut sustained_waves = Vec::new();
    let mut rejected_wave_pulses = Vec::new();
    let mut background_subject_ids = BTreeSet::new();
    let mut detail_focus_subject_ids = BTreeSet::new();
    let mut sustained_material_consequence_count = 0usize;
    let mut sustained_material_outcome_count = 0usize;
    let mut mira_outcome_kinds = BTreeSet::new();
    detail_focus_subject_ids.extend(direct_resolution_subject_ids(&output.wave.cover.cells));
    let sustained_budgets = if presence_only {
        vec![]
    } else if fairness_stress_waves == 0 {
        vec![4_u8, 8, 4]
    } else {
        vec![fairness_stress_budget; fairness_stress_waves]
    };
    if let Some(provider_parallelism) = stress_provider_parallelism
        && advanced.resolution_policy.provider_parallelism != provider_parallelism
    {
        let control = kernel
            .command(WorldCommand::SetProviderParallelism {
                expected_revision: advanced.revision,
                expected_provider_configuration_epoch: advanced
                    .resolution_policy
                    .provider_configuration_epoch,
                provider_parallelism,
            })
            .await?;
        advanced = match control {
            CommandResult::ResolutionUpdated { campaign, .. } => campaign,
            _ => anyhow::bail!("provider parallelism change used the world commit path"),
        };
    }
    for (wave_index, budget) in sustained_budgets.into_iter().enumerate() {
        if advanced.resolution_policy.active_cell_budget != budget {
            let control = kernel
                .command(WorldCommand::SetResolutionBudget {
                    expected_revision: advanced.revision,
                    expected_resolution_epoch: advanced.resolution_policy.resolution_epoch,
                    active_cell_budget: budget,
                })
                .await?;
            advanced = match control {
                CommandResult::ResolutionUpdated { campaign, .. } => campaign,
                _ => anyhow::bail!("resolution budget change did not commit at a safe boundary"),
            };
        }
        let mut rejected_pulses_for_wave = 0;
        let sustained_output = loop {
            let trace_start = model_calls.lock().expect("live-fire trace lock").len();
            match propose_resolution_wave(
                model.clone(),
                Arc::new(SnapshotPermit::new_resolution(
                    store.clone(),
                    advanced.id,
                    advanced.revision,
                    advanced.resolution_policy.resolution_epoch,
                )),
                &advanced,
            )
            .await
            {
                Ok(output) => break output,
                Err(error) => {
                    rejected_pulses_for_wave += 1;
                    write_wave_failure(
                        &root,
                        wave_index + 1,
                        rejected_pulses_for_wave,
                        budget,
                        &error,
                        &model_calls,
                        trace_start,
                    )?;
                    rejected_wave_pulses.push(serde_json::json!({
                        "wave_index":wave_index + 1,
                        "pulse_attempt":rejected_pulses_for_wave,
                        "world_revision":advanced.revision,
                        "resolution_epoch":advanced.resolution_policy.resolution_epoch,
                        "error":error.to_string(),
                    }));
                    if rejected_pulses_for_wave > max_rejected_pulses_per_wave {
                        return Err(error);
                    }
                }
            }
        };
        std::fs::write(
            root.join(format!(
                "sustained-wave-{:02}-preflight.json",
                wave_index + 1
            )),
            serde_json::to_vec_pretty(&serde_json::json!({
                "wave_index":wave_index + 1,
                "budget":budget,
                "cover":&sustained_output.wave.cover,
                "appraisals":&sustained_output.wave.appraisals,
                "activity_outcomes":&sustained_output.wave.activity_outcomes,
                "private_cell_traces":&sustained_output.private_cell_traces,
                "model_stage_receipts":sustained_output.stages.iter().map(|stage|&stage.receipt).collect::<Vec<_>>(),
            }))?,
        )?;
        let sustained_plan = validate_and_resolve_wave(&advanced, &sustained_output.wave)?;
        let direct_material_count = sustained_plan.institution_actions.len()
            + sustained_plan.gestalt_actions.len()
            + sustained_plan.gestalt_migrations.len()
            + sustained_plan.actor_moves.len()
            + sustained_plan.member_migrations.len();
        let material_outcome_count = sustained_plan
            .activity_outcomes
            .iter()
            .filter(|outcome| {
                !matches!(
                    outcome.effect,
                    ghostlight_dungeon::domain::StrategicOutcomeEffect::NoMaterialChange { .. }
                )
            })
            .count();
        sustained_material_consequence_count += direct_material_count + material_outcome_count;
        sustained_material_outcome_count += material_outcome_count;
        for outcome in &sustained_plan.activity_outcomes {
            if let Some(kind) = member_outcome_kind(&outcome.effect, "mira-venn") {
                mira_outcome_kinds.insert(kind.to_owned());
            }
        }
        for appraisal in &sustained_output.wave.appraisals {
            for proposal in &appraisal.actions {
                if proposal.subject_id != "member:mira-venn" {
                    background_subject_ids.insert(proposal.subject_id.clone());
                }
            }
        }
        detail_focus_subject_ids.extend(direct_resolution_subject_ids(
            &sustained_output.wave.cover.cells,
        ));
        for stage in &sustained_output.stages {
            store.insert(
                "persona_stage_receipt.v1",
                "ghostlight.persona_stage_receipt.v1",
                stage.receipt.storage_key(),
                &stage.receipt,
            )?;
        }
        let sustained_commit = kernel
            .command(WorldCommand::AdvanceStrategicTick {
                expected_revision: advanced.revision,
                source: TickSource::Scheduler,
                plan: None,
                model_receipt_hash: Some(sustained_output.aggregate_receipt_hash.clone()),
                resolution_wave: Some(sustained_output.wave.clone()),
            })
            .await?;
        advanced = match &sustained_commit {
            CommandResult::Committed { campaign, .. } => campaign.clone(),
            _ => anyhow::bail!("sustained background wave did not commit"),
        };
        if advanced.actors[&advanced.player_actor_id] != player_before {
            anyhow::bail!("sustained background simulation puppeted the player")
        }
        sustained_waves.push(serde_json::json!({
            "wave_index":wave_index + 1,
            "budget":budget,
            "cover":sustained_output.wave.cover,
            "appraisals":sustained_output.wave.appraisals,
            "plan":sustained_plan,
            "commit":sustained_commit,
            "private_cell_traces":sustained_output.private_cell_traces,
            "model_stage_receipts":sustained_output.stages.into_iter().map(|stage|stage.receipt).collect::<Vec<_>>(),
        }));
        std::fs::write(
            root.join("sustained-preflight.json"),
            serde_json::to_vec_pretty(&sustained_waves)?,
        )?;
    }
    if !presence_only && background_subject_ids.len() < 3 {
        anyhow::bail!(
            "sustained multiresolution waves produced only {} distinct background actors",
            background_subject_ids.len()
        )
    }
    if !presence_only && sustained_material_consequence_count == 0 {
        anyhow::bail!("sustained multiresolution waves resolved no material background consequence")
    }
    let fairness_missing_subject_ids = if fairness_stress_waves == 0 {
        BTreeSet::new()
    } else {
        root_cell
            .subject_ids
            .difference(&detail_focus_subject_ids)
            .cloned()
            .collect::<BTreeSet<_>>()
    };
    if !fairness_missing_subject_ids.is_empty() {
        anyhow::bail!(
            "budget-one fairness stress omitted debt focus for {:?}",
            fairness_missing_subject_ids
        )
    }
    let member_after_sustained = &advanced.gestalt_members["mira-venn"];
    if member_after_sustained.gestalt_id != "harbor-neighbors"
        || member_after_sustained.name != member_before.name
        || member_after_sustained.conditions != member_before.conditions
        || effective_member_capabilities(&advanced, "mira-venn")? != capabilities_before
    {
        anyhow::bail!("sustained population simulation damaged Mira's dormant identity")
    }
    if (member_after_sustained.relationships != member_before.relationships
        && !mira_outcome_kinds.contains("relationship"))
        || (member_after_sustained.memories != member_before.memories
            && !mira_outcome_kinds.contains("memory"))
        || (member_after_sustained.obligations != member_before.obligations
            && !mira_outcome_kinds.contains("obligation"))
        || (member_after_sustained.equipment != member_before.equipment
            && !mira_outcome_kinds.contains("resource"))
        || (effective_member_knowledge(&advanced, "mira-venn")? != knowledge_before
            && !mira_outcome_kinds.contains("knowledge"))
    {
        anyhow::bail!("Mira's dormant delta changed without an exact member-bound outcome")
    }

    let return_event = "The player spends a quiet afternoon repairing the South Harbor ferry steps after the resettlement. The settled harbor populations move through their ordinary routines nearby.";
    let presence_planner = GestaltPresencePlanner {
        model: model.clone(),
        model_name: "deepseek-v4-flash".into(),
    };
    let (presence_plan, presence_receipt) = presence_planner.plan(&advanced, return_event).await?;
    std::fs::write(
        root.join("presence-preflight.json"),
        serde_json::to_vec_pretty(&serde_json::json!({
            "schema":"ghostlight.live_fire_presence_preflight.v1",
            "campaign_revision":advanced.revision,
            "return_event":return_event,
            "plan":presence_plan,
            "receipt":presence_receipt,
        }))?,
    )?;
    if presence_plan.individuations.len() != 0
        || presence_plan.demotions.len() != 0
        || presence_plan.promotions.len() != 1
        || presence_plan.promotions[0].member_id != "mira-venn"
        || presence_plan.promotions[0].gestalt_id != "harbor-neighbors"
    {
        anyhow::bail!(
            "automatic presence planning did not surface the existing refugee callback: {}",
            serde_json::to_string(&presence_plan)?
        )
    }
    store.insert(
        "persona_stage_receipt.v1",
        "ghostlight.persona_stage_receipt.v1",
        presence_receipt.storage_key(),
        &presence_receipt,
    )?;
    let materialized = kernel
        .command(WorldCommand::ReconcileGestaltPresence {
            expected_revision: advanced.revision,
            reason: return_event.into(),
            plan: presence_plan.clone(),
        })
        .await?;
    let CommandResult::Committed {
        campaign: returned, ..
    } = &materialized
    else {
        anyhow::bail!("Mira did not rematerialize at her destination")
    };
    let actor = &returned.actors["member:mira-venn"];
    if actor.location_id != returned.actors[&returned.player_actor_id].location_id
        || actor.name != member_before.name
        || actor.relationships != member_before.relationships
        || actor.memories != member_before.memories
        || actor.capabilities != capabilities_before
        || actor.knowledge != knowledge_before
    {
        anyhow::bail!("return encounter did not rematerialize the same person")
    }
    let event_kinds = returned
        .events
        .iter()
        .map(|event| event.kind.clone())
        .collect::<Vec<_>>();
    let stage_receipts = output
        .stages
        .iter()
        .map(|stage| &stage.receipt)
        .collect::<Vec<_>>();
    let result = serde_json::json!({
        "schema":"ghostlight.gestalt_dynamics_smoke.v1",
        "scenario_id":scenario_id,
        "elapsed_seconds":started.elapsed().as_secs_f64(),
        "subject_count":root_cell.subject_ids.len(),
        "configured_budget":campaign.resolution_policy.active_cell_budget,
        "effective_budget":output.wave.cover.effective_budget,
        "cell_mode":root_cell.mode,
        "rivals_share_arena":true,
        "migration_proposal":migration_proposal,
        "migration_baseline":baseline_receipt,
        "initial_background_action_count":background_action_count,
        "sustained_background_subject_ids":background_subject_ids,
        "sustained_detail_focus_subject_ids":detail_focus_subject_ids,
        "sustained_material_consequence_count":sustained_material_consequence_count,
        "sustained_material_outcome_count":sustained_material_outcome_count,
        "mira_outcome_kinds":mira_outcome_kinds,
        "fairness_stress_waves":fairness_stress_waves,
        "presence_only":presence_only,
        "fairness_stress_budget":fairness_stress_budget,
        "stress_provider_parallelism":stress_provider_parallelism,
        "max_rejected_pulses_per_wave":max_rejected_pulses_per_wave,
        "rejected_wave_pulses":rejected_wave_pulses,
        "fairness_missing_subject_ids":fairness_missing_subject_ids,
        "source_lineage_depth":source_lineage_depth,
        "destination_lineage_depth":destination_lineage_depth,
        "sustained_waves":sustained_waves,
        "plan":plan,
        "commit":committed,
        "materialization":materialized,
        "automatic_presence_plan":presence_plan,
        "automatic_presence_receipt":presence_receipt,
        "identity_preserved":true,
        "member_delta_changes_bound_to_outcomes":true,
        "player_unchanged":true,
        "return_encounter_same_person":true,
        "event_kinds":event_kinds,
        "model_stage_receipts":stage_receipts,
        "store":root.join("campaign.cc"),
        "preflight_path":root.join("preflight.json"),
        "result_path":root.join("result.json")
    });
    std::fs::write(
        root.join("result.json"),
        serde_json::to_vec_pretty(&result)?,
    )?;
    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}

#[cfg(windows)]
fn member_outcome_kind(
    effect: &ghostlight_dungeon::domain::StrategicOutcomeEffect,
    member_id: &str,
) -> Option<&'static str> {
    use ghostlight_dungeon::domain::StrategicOutcomeEffect;

    let member_subject_id = format!("member:{member_id}");
    match effect {
        StrategicOutcomeEffect::MemberMemory {
            member_id: owner, ..
        } if owner == member_id => Some("memory"),
        StrategicOutcomeEffect::MemberObligation {
            member_id: owner, ..
        } if owner == member_id => Some("obligation"),
        StrategicOutcomeEffect::MemberRelationship {
            member_id: owner, ..
        } if owner == member_id => Some("relationship"),
        StrategicOutcomeEffect::KnowledgeLearned {
            owner_subject_id, ..
        } if owner_subject_id == &member_subject_id => Some("knowledge"),
        StrategicOutcomeEffect::ResourceCreated {
            owner_subject_id, ..
        }
        | StrategicOutcomeEffect::ResourceConsumed {
            owner_subject_id, ..
        } if owner_subject_id == &member_subject_id => Some("resource"),
        StrategicOutcomeEffect::ResourceTransferred {
            from_subject_id,
            to_subject_id,
            ..
        } if from_subject_id == &member_subject_id || to_subject_id == &member_subject_id => {
            Some("resource")
        }
        _ => None,
    }
}

#[cfg(windows)]
fn lineage_depth(
    campaign: &ghostlight_dungeon::domain::Campaign,
    leaf_id: &str,
) -> anyhow::Result<usize> {
    use std::collections::BTreeSet;

    let mut current = leaf_id;
    let mut depth = 0;
    let mut seen = BTreeSet::new();
    while let Some(lineage) = campaign.gestalt_lineages.values().find(|lineage| {
        lineage
            .child_gestalt_ids
            .iter()
            .any(|child| child == current)
    }) {
        if !seen.insert(lineage.parent_gestalt_id.as_str()) {
            anyhow::bail!("gestalt lineage contains a cycle at {current}")
        }
        depth += 1;
        current = &lineage.parent_gestalt_id;
    }
    Ok(depth)
}

#[cfg(windows)]
fn direct_resolution_subject_ids(
    cells: &[ghostlight_dungeon::domain::SimulationCell],
) -> std::collections::BTreeSet<String> {
    cells
        .iter()
        .filter_map(|cell| cell.detail_focus_subject_id.clone())
        .chain(
            cells
                .iter()
                .filter(|cell| cell.subject_ids.len() == 1)
                .flat_map(|cell| cell.subject_ids.iter().cloned()),
        )
        .collect()
}

#[cfg(windows)]
fn dynamics_campaign() -> ghostlight_dungeon::domain::Campaign {
    use chrono::{Duration, Utc};
    use ghostlight_dungeon::domain::*;
    use std::collections::{BTreeMap, BTreeSet};

    let now = Utc::now();
    let locations = BTreeMap::from([
        (
            "south-harbor".into(),
            Location {
                id: "south-harbor".into(),
                name: "South Harbor".into(),
                container_id: None,
                routes: BTreeMap::new(),
                persistent_features: vec!["ferry steps", "weathered net lofts"]
                    .into_iter()
                    .map(str::to_owned)
                    .collect(),
            },
        ),
        (
            "transit-camp".into(),
            Location {
                id: "transit-camp".into(),
                name: "Eastern Transit Camp".into(),
                container_id: None,
                routes: BTreeMap::from([(
                    "resettlement-ferry".into(),
                    Route {
                        destination_id: "south-harbor".into(),
                        distance: "across the protected bay".into(),
                        travel_minutes: 90,
                    },
                )]),
                persistent_features: vec!["departure board", "storm-damaged shelter rows"]
                    .into_iter()
                    .map(str::to_owned)
                    .collect(),
            },
        ),
    ]);
    let player = ActorState {
        id: "player".into(),
        name: "The gate runner".into(),
        location_id: "south-harbor".into(),
        capabilities: BTreeSet::from(["open emergency access routes".into()]),
        knowledge: BTreeSet::from(["public harbor bulletin".into()]),
        equipment: BTreeSet::new(),
        conditions: BTreeSet::new(),
        obligations: BTreeSet::new(),
        relationships: BTreeMap::from([(
            "member:mira-venn".into(),
            "helped her family escape the eastern fire".into(),
        )]),
        goals: vec!["help rebuild the harbor without commanding its people".into()],
        memories: vec!["Mira disappeared into the evacuation crowd weeks ago.".into()],
    };
    let institutions = (0..16)
        .map(|index| {
            let id = match index {
                0 => "port-authority".into(),
                1 => "relief-union".into(),
                _ => format!("regional-power-{index:02}"),
            };
            (
                id.clone(),
                InstitutionState {
                    id,
                    name: match index {
                        0 => "Port Authority".into(),
                        1 => "Relief Union".into(),
                        _ => format!("Regional Power {index:02}"),
                    },
                    resources: vec![
                        format!("regional reserve {index:02}"),
                        "public harbor bulletin".into(),
                    ],
                    goals: vec![format!("shape resettlement policy bloc {}", index % 5)],
                    posture: format!(
                        "Bloc {} weighs shelter, labor, and political costs before the storm.",
                        index % 4
                    ),
                },
            )
        })
        .collect();
    let gestalt = |id: &str,
                   name: &str,
                   home: &str,
                   capabilities: &[&str],
                   knowledge: &[&str],
                   goals: &[&str],
                   pressures: &[&str]| GestaltPersonaState {
        schema: "ghostlight.gestalt_persona_state.v1".into(),
        id: id.into(),
        name: name.into(),
        version: 0,
        home_location_id: home.into(),
        shared_capabilities: capabilities.iter().map(|value| (*value).into()).collect(),
        shared_knowledge: knowledge.iter().map(|value| (*value).into()).collect(),
        resources: BTreeSet::new(),
        goals: goals.iter().map(|value| (*value).into()).collect(),
        pressures: pressures.iter().map(|value| (*value).into()).collect(),
    };
    let mut gestalts = BTreeMap::from([
        (
            "refugees-east".into(),
            gestalt(
                "refugees-east",
                "Eastern fire refugees",
                "transit-camp",
                &["survive transit", "organize shelter rows"],
                &["camp departure board", "public harbor bulletin"],
                &["find durable homes before the storm"],
                &["the camp closes after this ferry"],
            ),
        ),
        (
            "harbor-neighbors".into(),
            gestalt(
                "harbor-neighbors",
                "South Harbor neighbors",
                "south-harbor",
                &["repair nets", "organize harbor kitchens"],
                &["harbor routines", "public harbor bulletin"],
                &["keep the harbor alive through the storm"],
                &["some residents resent the resettlement order"],
            ),
        ),
        (
            "relief-crews".into(),
            gestalt(
                "relief-crews",
                "Mutual-aid relief crews",
                "south-harbor",
                &["distribute shelter supplies"],
                &["public harbor bulletin"],
                &["keep arrivals alive"],
                &["supplies cover only one more night"],
            ),
        ),
        (
            "dock-guild".into(),
            gestalt(
                "dock-guild",
                "Dock labor guild",
                "south-harbor",
                &["load ferries", "repair cranes"],
                &["dock shift board", "public harbor bulletin"],
                &["protect shifts and keep ferries moving"],
                &["the storm deadline compresses every shift"],
            ),
        ),
    ]);
    for (id, name, home) in [
        ("displaced-root", "All displaced people", "transit-camp"),
        (
            "crisis-refugees",
            "Crisis refugee populations",
            "transit-camp",
        ),
        ("displaced-other", "Other displaced people", "transit-camp"),
        ("crisis-other", "Other crisis refugees", "transit-camp"),
        (
            "southport-root",
            "All Southport populations",
            "south-harbor",
        ),
        ("harbor-populations", "Harbor populations", "south-harbor"),
        (
            "inland-other",
            "Other Southport populations",
            "south-harbor",
        ),
        ("harbor-other", "Other harbor populations", "south-harbor"),
    ] {
        gestalts.insert(id.into(), gestalt(id, name, home, &[], &[], &[], &[]));
    }
    let member = GestaltMemberDelta {
        schema: "ghostlight.gestalt_member_delta.v1".into(),
        id: "mira-venn".into(),
        gestalt_id: "refugees-east".into(),
        version: 3,
        name: "Mira Venn".into(),
        capability_additions: BTreeSet::from(["weave signal cord".into()]),
        capability_removals: BTreeSet::from(["organize shelter rows".into()]),
        knowledge_additions: BTreeSet::from(["the player kept the evacuation gate open".into()]),
        knowledge_removals: BTreeSet::new(),
        equipment: BTreeSet::from(["patched blue satchel".into()]),
        conditions: BTreeSet::from(["healed smoke burn".into()]),
        obligations: BTreeSet::from(["thank the player if their paths cross again".into()]),
        relationships: BTreeMap::from([(
            "player".into(),
            "trusts them for opening the evacuation gate without demanding obedience".into(),
        )]),
        goals: vec!["take the verified South Harbor berth and build a quiet life".into()],
        memories: vec![
            "The player held the eastern gate while Mira carried her brother through the smoke."
                .into(),
        ],
        last_location_id: Some("transit-camp".into()),
        materialized_actor_id: None,
        last_relevant_revision: 7,
        relevance_lease_until_revision: 0,
    };
    let pressure = "The last protected ferry leaves the Eastern Transit Camp before the storm. Mira Venn has one verified berth in South Harbor, where the harbor neighbors dispute the resettlement order. The port authority, relief union, dock guild, and regional powers must also act on shelter, labor, and supply pressures; none may decide for Mira or speak as the arena.";
    let mut campaign = Campaign {
        schema: "ghostlight.campaign.v1".into(),
        id: uuid::Uuid::new_v4(),
        name: "Longitudinal refugee return acceptance".into(),
        revision: 0,
        branch_origin: BranchOrigin {
            canon_cutoff: "acceptance-fixture".into(),
            evidence_receipt_ids: vec![],
        },
        world_time: now,
        tick_hours: 6,
        player_actor_id: "player".into(),
        locations,
        actors: BTreeMap::from([("player".into(), player)]),
        institutions,
        clocks: BTreeMap::from([(
            "storm-ferry".into(),
            WorldClock {
                id: "storm-ferry".into(),
                label: "Last protected ferry".into(),
                progress: 3,
                threshold: 4,
                consequence: "the camp is cut off by the storm".into(),
            },
        )]),
        facts: BTreeMap::new(),
        transcript: vec![],
        last_player_activity: now - Duration::hours(2),
        pending_ticks: 1,
        away_ticks_processed: 0,
        events: vec![Event {
            id: "resettlement-deadline".into(),
            at: now,
            kind: "public_notice".into(),
            summary: pressure.into(),
            actor_ids: vec![],
            institution_ids: vec![],
            gestalt_ids: vec!["refugees-east".into(), "harbor-neighbors".into()],
            location_ids: vec!["transit-camp".into(), "south-harbor".into()],
            public_channels: vec!["public harbor bulletin".into()],
        }],
        news: vec![],
        canon_candidates: BTreeMap::new(),
        gestalts,
        gestalt_members: BTreeMap::from([("mira-venn".into(), member)]),
        pending_world_proposals: vec![],
        agency_profiles: BTreeMap::new(),
        agency_relations: BTreeMap::new(),
        gestalt_lineages: BTreeMap::new(),
        resolution_policy: ResolutionPolicy {
            active_cell_budget: 1,
            provider_parallelism: 4,
            ..ResolutionPolicy::default()
        },
        resolution_pins: BTreeMap::new(),
        resolution_cover: None,
        strategic_tick_count: 0,
    };
    ghostlight_dungeon::resolution::ensure_agency_profiles(&mut campaign);
    for id in [
        "displaced-root",
        "crisis-refugees",
        "southport-root",
        "harbor-populations",
    ] {
        let profile = campaign.agency_profiles.get_mut(id).unwrap();
        profile.active_leaf = false;
        profile.simulation_eligible = false;
    }
    campaign
        .agency_profiles
        .get_mut("crisis-refugees")
        .unwrap()
        .parent_subject_id = Some("displaced-root".into());
    campaign
        .agency_profiles
        .get_mut("displaced-other")
        .unwrap()
        .parent_subject_id = Some("displaced-root".into());
    campaign
        .agency_profiles
        .get_mut("refugees-east")
        .unwrap()
        .parent_subject_id = Some("crisis-refugees".into());
    campaign
        .agency_profiles
        .get_mut("crisis-other")
        .unwrap()
        .parent_subject_id = Some("crisis-refugees".into());
    campaign
        .agency_profiles
        .get_mut("harbor-populations")
        .unwrap()
        .parent_subject_id = Some("southport-root".into());
    campaign
        .agency_profiles
        .get_mut("inland-other")
        .unwrap()
        .parent_subject_id = Some("southport-root".into());
    campaign
        .agency_profiles
        .get_mut("harbor-neighbors")
        .unwrap()
        .parent_subject_id = Some("harbor-populations".into());
    campaign
        .agency_profiles
        .get_mut("harbor-other")
        .unwrap()
        .parent_subject_id = Some("harbor-populations".into());
    for (index, profile) in campaign
        .agency_profiles
        .values_mut()
        .filter(|profile| profile.active_leaf && profile.simulation_eligible)
        .enumerate()
    {
        profile.facets.insert(
            AgencyAxis::Geography,
            BTreeSet::from([format!("region-{}", index % 4)]),
        );
        profile.facets.insert(
            AgencyAxis::Ideology,
            BTreeSet::from([format!("bloc-{}", index % 5)]),
        );
        profile.facets.insert(
            AgencyAxis::Information,
            BTreeSet::from([format!("channel-{}", index % 6)]),
        );
    }
    campaign.agency_relations.insert(
        "resettlement-route".into(),
        AgencyRelation {
            schema: "ghostlight.agency_relation.v1".into(),
            id: "resettlement-route".into(),
            from_subject_id: "refugees-east".into(),
            to_subject_id: "harbor-neighbors".into(),
            kind: AgencyRelationKind::Migration,
            strength: 95,
            active: true,
            evidence_receipt_ids: vec![],
        },
    );
    campaign.agency_relations.insert(
        "resettlement-rivalry".into(),
        AgencyRelation {
            schema: "ghostlight.agency_relation.v1".into(),
            id: "resettlement-rivalry".into(),
            from_subject_id: "refugees-east".into(),
            to_subject_id: "harbor-neighbors".into(),
            kind: AgencyRelationKind::Rivalry,
            strength: 80,
            active: true,
            evidence_receipt_ids: vec![],
        },
    );
    campaign.gestalt_lineages.insert(
        "displaced-root".into(),
        GestaltLineage {
            schema: "ghostlight.gestalt_lineage.v1".into(),
            parent_gestalt_id: "displaced-root".into(),
            child_gestalt_ids: vec!["crisis-refugees".into(), "displaced-other".into()],
            partition_axis: AgencyAxis::Ideology,
            partition_values: BTreeMap::from([
                ("crisis-refugees".into(), "crisis".into()),
                ("displaced-other".into(), "other/unknown".into()),
            ]),
            residual_child_id: "displaced-other".into(),
            source_revision: 0,
        },
    );
    campaign.gestalt_lineages.insert(
        "crisis-refugees".into(),
        GestaltLineage {
            schema: "ghostlight.gestalt_lineage.v1".into(),
            parent_gestalt_id: "crisis-refugees".into(),
            child_gestalt_ids: vec!["refugees-east".into(), "crisis-other".into()],
            partition_axis: AgencyAxis::Geography,
            partition_values: BTreeMap::from([
                ("refugees-east".into(), "east".into()),
                ("crisis-other".into(), "other/unknown".into()),
            ]),
            residual_child_id: "crisis-other".into(),
            source_revision: 0,
        },
    );
    campaign.gestalt_lineages.insert(
        "southport-root".into(),
        GestaltLineage {
            schema: "ghostlight.gestalt_lineage.v1".into(),
            parent_gestalt_id: "southport-root".into(),
            child_gestalt_ids: vec!["harbor-populations".into(), "inland-other".into()],
            partition_axis: AgencyAxis::Geography,
            partition_values: BTreeMap::from([
                ("harbor-populations".into(), "harbor".into()),
                ("inland-other".into(), "other/unknown".into()),
            ]),
            residual_child_id: "inland-other".into(),
            source_revision: 0,
        },
    );
    campaign.gestalt_lineages.insert(
        "harbor-populations".into(),
        GestaltLineage {
            schema: "ghostlight.gestalt_lineage.v1".into(),
            parent_gestalt_id: "harbor-populations".into(),
            child_gestalt_ids: vec!["harbor-neighbors".into(), "harbor-other".into()],
            partition_axis: AgencyAxis::EconomyRole,
            partition_values: BTreeMap::from([
                ("harbor-neighbors".into(), "neighbors".into()),
                ("harbor-other".into(), "other/unknown".into()),
            ]),
            residual_child_id: "harbor-other".into(),
            source_revision: 0,
        },
    );
    campaign
}
