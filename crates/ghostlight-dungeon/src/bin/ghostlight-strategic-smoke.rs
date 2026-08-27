fn final_wave_field(
    wave_reports: &[serde_json::Value],
    field: &str,
) -> anyhow::Result<serde_json::Value> {
    wave_reports
        .last()
        .and_then(|wave| wave.get(field))
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("final strategic wave is missing {field}"))
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    use chrono::Utc;
    use ghostlight_dungeon::{
        domain::{TickSource, WorldCommand},
        kernel::{CommandResult, WorldKernel},
        model_runtime::ModelRuntimeSelection,
        persistence::CampaignStore,
        scheduler::{ResolutionWavePipelineFailure, propose_resolution_wave},
        turn::SnapshotPermit,
    };
    use std::{path::PathBuf, sync::Arc, time::Instant};

    let runtime_root = std::env::var_os("GHOSTLIGHT_DUNGEON_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(default_runtime_root);
    let model_selection = ModelRuntimeSelection::from_environment(&runtime_root)?;
    let scenario_id = std::env::var("GHOSTLIGHT_LIVE_FIRE_SCENARIO")
        .unwrap_or_else(|_| "strategic-default".into());
    let pressure = std::env::var("GHOSTLIGHT_STRATEGIC_PRESSURE").unwrap_or_else(|_| {
        "The sovereign deep-hold diverted the White Root aquifer. Two tithe caravans have vanished, the charcoal guilds threaten secession, and somebody pawned the regent's rain seal."
            .into()
    });
    let wave_count = bounded_environment_usize("GHOSTLIGHT_STRATEGIC_WAVES", 1, 1, 8)?;
    let max_rejected_pulses_per_wave =
        bounded_environment_usize("GHOSTLIGHT_STRATEGIC_MAX_REJECTED_PULSES_PER_WAVE", 2, 0, 4)?;
    let root = std::env::var_os("GHOSTLIGHT_LIVE_FIRE_RESULT_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(r"F:\GameCult\GhostlightDungeon\acceptance").join(format!(
                "strategic-{}-{}",
                Utc::now().format("%Y%m%d-%H%M%S"),
                uuid::Uuid::new_v4()
            ))
        });
    std::fs::create_dir_all(&root)?;
    let model = model_selection.open()?.ok_or_else(|| {
        anyhow::anyhow!(
            "{} credential is unavailable at {}",
            model_selection.provider,
            model_selection.credential_path.display()
        )
    })?;
    let public_channel = admitted_public_channel(
        &std::env::var("GHOSTLIGHT_STRATEGIC_PUBLIC_CHANNEL")
            .unwrap_or_else(|_| "root-wire broadsheet".into()),
    )?;
    let compiled = std::env::var("GHOSTLIGHT_WORLD_DESCRIPTION")
        .ok()
        .filter(|description| !description.trim().is_empty());
    let (mut campaign, seed_evidence_receipts, seed_model_receipts, mut world_compile) =
        if let Some(description) = compiled.as_deref() {
            std::fs::write(
                root.join("status.json"),
                serde_json::to_vec_pretty(&serde_json::json!({
                    "schema":"ghostlight.live_strategic_smoke_status.v1",
                    "state":"compiling_world",
                    "waves_completed":0,
                    "waves_requested":wave_count,
                    "updated_at":Utc::now(),
                }))?,
            )?;
            let (preview, receipts) = compile_strategic_campaign(
                model.clone(),
                &model_selection.fast_model,
                &model_selection.capable_model,
                description,
                &pressure,
                &public_channel,
            )
            .await?;
            std::fs::write(
                root.join("compiler-preview.json"),
                serde_json::to_vec_pretty(&serde_json::json!({
                    "description":description,
                    "preview":&preview,
                    "model_receipts":&receipts,
                }))?,
            )?;
            let evidence = preview.evidence_receipts.clone();
            let campaign = preview.campaign.clone();
            (
                campaign,
                evidence,
                receipts.clone(),
                Some(serde_json::json!({
                    "description":description,
                    "preview":preview,
                    "model_receipts":receipts,
                    "preview_path":root.join("compiler-preview.json"),
                })),
            )
        } else {
            (strategic_campaign(), vec![], vec![], None)
        };
    ghostlight_dungeon::compiler::validate_campaign_seed(&campaign)?;
    let pressure_event = ghostlight_dungeon::domain::Event {
        id: format!("pressure-{}", uuid::Uuid::new_v4()),
        at: campaign.world_time,
        kind: "strategic_pressure".into(),
        summary: pressure.clone(),
        actor_ids: campaign
            .actors
            .keys()
            .filter(|actor_id| **actor_id != campaign.player_actor_id)
            .cloned()
            .collect(),
        institution_ids: campaign.institutions.keys().cloned().collect(),
        gestalt_ids: campaign.gestalts.keys().cloned().collect(),
        location_ids: campaign.locations.keys().cloned().collect(),
        public_channels: vec![public_channel.clone()],
    };
    campaign.news.push(ghostlight_dungeon::domain::NewsIssue {
        id: format!(
            "news:{}:{}",
            pressure_event.id,
            public_channel.replace(' ', "-")
        ),
        at: pressure_event.at,
        channel: public_channel.clone(),
        headline: ghostlight_dungeon::domain::committed_news_headline(&pressure_event.summary),
        event_ids: vec![pressure_event.id.clone()],
        reliability: "committed public channel".into(),
    });
    campaign.events.push(pressure_event);
    let player_before = campaign.actors[&campaign.player_actor_id].clone();
    let store = CampaignStore::open(root.join("campaign.cc"))?;
    store.create_unadmitted_fixture_campaign(
        &campaign,
        &seed_evidence_receipts,
        &seed_model_receipts,
    )?;
    let kernel = WorldKernel::start(store.clone());
    let elaboration_passes = if compiled.is_some() {
        bounded_environment_usize("GHOSTLIGHT_WORLD_ELABORATION_PASSES", 0, 0, 8)?
    } else {
        0
    };
    let initial_location_ids = campaign
        .locations
        .keys()
        .take(elaboration_passes)
        .cloned()
        .collect::<Vec<_>>();
    let mut elaboration_reports = Vec::with_capacity(initial_location_ids.len());
    if let Some(description) = compiled.as_deref()
        && !initial_location_ids.is_empty()
    {
        let compiler = strategic_world_compiler(
            model.clone(),
            &model_selection.fast_model,
            &model_selection.capable_model,
            description,
            &strategic_world_when(),
        );
        for (index, location_id) in initial_location_ids.iter().enumerate() {
            let location_name = campaign.locations[location_id].name.clone();
            std::fs::write(
                root.join("status.json"),
                serde_json::to_vec_pretty(&serde_json::json!({
                    "schema":"ghostlight.live_strategic_smoke_status.v1",
                    "state":"elaborating_world",
                    "elaborations_completed":index,
                    "elaborations_requested":initial_location_ids.len(),
                    "current_location_id":location_id,
                    "waves_completed":0,
                    "waves_requested":wave_count,
                    "world_revision":campaign.revision,
                    "updated_at":Utc::now(),
                }))?,
            )?;
            let request = strategic_locality_request(&location_name, location_id, &pressure);
            let (preview, receipts) = compiler
                .compile_destination(&campaign, location_id, &request)
                .await?;
            let command = match &preview {
                ghostlight_dungeon::domain::DestinationCompilationPreview::LocalityElaboration(
                    preview,
                ) => WorldCommand::ElaborateLocality {
                    expected_revision: preview.expected_revision,
                    elaboration: preview.elaboration.clone(),
                    evidence_receipts: preview.evidence_receipts.clone(),
                    canon_candidates: preview.canon_candidates.clone(),
                    model_stage_receipts: receipts.clone(),
                },
                ghostlight_dungeon::domain::DestinationCompilationPreview::RegionExpansion(_) => {
                    anyhow::bail!(
                        "strategic elaboration resolved existing location {location_id} as a new destination"
                    )
                }
            };
            let preview_path = root.join(format!("elaboration-{:02}-preview.json", index + 1));
            std::fs::write(
                &preview_path,
                serde_json::to_vec_pretty(&serde_json::json!({
                    "location_id":location_id,
                    "location_name":location_name,
                    "request":request,
                    "preview":&preview,
                    "model_receipts":&receipts,
                }))?,
            )?;
            let committed = kernel.command(command).await?;
            let CommandResult::Committed {
                campaign: elaborated,
                ..
            } = committed
            else {
                anyhow::bail!("strategic locality elaboration did not commit")
            };
            campaign = elaborated;
            elaboration_reports.push(serde_json::json!({
                "location_id":location_id,
                "location_name":location_name,
                "world_revision":campaign.revision,
                "preview_path":preview_path,
                "model_receipts":receipts,
            }));
        }
    }
    if let Some(metadata) = world_compile
        .as_mut()
        .and_then(serde_json::Value::as_object_mut)
    {
        metadata.insert(
            "elaborations".into(),
            serde_json::Value::Array(elaboration_reports),
        );
    }
    let newspaper_title = std::env::var("GHOSTLIGHT_STRATEGIC_NEWSPAPER_TITLE")
        .unwrap_or_else(|_| "The Underdeep Clarion".into());
    let newspaper_voice = std::env::var("GHOSTLIGHT_STRATEGIC_NEWSPAPER_VOICE")
        .unwrap_or_else(|_| {
            "A sharp regional broadsheet for readers who already understand guild politics: skeptical of every throne, attentive to labor and material consequences, formally reported, and capable of one dry local barb without becoming satire."
                .into()
        });
    let started = Instant::now();
    let mut wave_reports = Vec::with_capacity(wave_count);
    for wave_index in 1..=wave_count {
        let previous_news_count = if wave_index == 1 {
            0
        } else {
            campaign.news.len()
        };
        let mut rejected_pulses = Vec::new();
        let output = loop {
            match propose_resolution_wave(
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
                Ok(output) => break output,
                Err(error) => {
                    let pulse = rejected_pulses.len() + 1;
                    let rejected_stage_receipt_hashes = error
                        .downcast_ref::<ResolutionWavePipelineFailure>()
                        .map(|failure| {
                            store.persist_model_stage_receipts(&failure.stage_receipts)?;
                            Ok::<_, anyhow::Error>(
                                failure
                                    .stage_receipts
                                    .iter()
                                    .map(|receipt| receipt.storage_key().to_owned())
                                    .collect::<Vec<_>>(),
                            )
                        })
                        .transpose()?
                        .unwrap_or_default();
                    std::fs::write(
                        root.join(format!(
                            "wave-{wave_index:02}-rejected-pulse-{pulse:02}.txt"
                        )),
                        error.to_string(),
                    )?;
                    let rejected_pulse = serde_json::json!({
                        "pulse":pulse,
                        "world_revision":campaign.revision,
                        "resolution_epoch":campaign.resolution_policy.resolution_epoch,
                        "error":error.to_string(),
                        "rejected_stage_receipt_hashes":rejected_stage_receipt_hashes,
                    });
                    if rejected_pulses.len() < max_rejected_pulses_per_wave {
                        rejected_pulses.push(rejected_pulse);
                    } else {
                        std::fs::write(
                            root.join(format!("wave-{wave_index:02}-terminal-failure.json")),
                            serde_json::to_vec_pretty(&serde_json::json!({
                                "rejected_pulses":rejected_pulses,
                                "terminal_failure":rejected_pulse,
                            }))?,
                        )?;
                        return Err(error);
                    }
                }
            }
        };
        std::fs::write(
            root.join(format!("wave-{wave_index:02}-preflight.json")),
            serde_json::to_vec_pretty(&serde_json::json!({
                "wave_index":wave_index,
                "world_revision":campaign.revision,
                "resolution_epoch":campaign.resolution_policy.resolution_epoch,
                "cover":&output.wave.cover,
                "appraisals":&output.wave.appraisals,
                "activity_outcomes":&output.wave.activity_outcomes,
                "private_cell_traces":&output.private_cell_traces,
                "model_stage_receipts":output.stages.iter().map(|stage|&stage.receipt).collect::<Vec<_>>(),
                "rejected_pulses":&rejected_pulses,
            }))?,
        )?;
        let plan =
            ghostlight_dungeon::resolution::validate_and_resolve_wave(&campaign, &output.wave)?;
        let material_activity_outcomes = plan
            .activity_outcomes
            .iter()
            .filter(|outcome| {
                !matches!(
                    outcome.effect,
                    ghostlight_dungeon::domain::StrategicOutcomeEffect::NoMaterialChange { .. }
                )
            })
            .count();
        if plan.institution_actions.is_empty()
            && plan.gestalt_actions.is_empty()
            && plan.gestalt_migrations.is_empty()
            && plan.actor_moves.is_empty()
            && plan.member_migrations.is_empty()
            && material_activity_outcomes == 0
        {
            anyhow::bail!(
                "strategic wave {wave_index} resolved no material offscreen change: direct transitions were empty and every selected activity outcome was no_material_change"
            );
        }
        store.persist_model_stage_receipts(
            &output
                .stages
                .iter()
                .map(|stage| stage.receipt.clone())
                .collect::<Vec<_>>(),
        )?;
        let committed = kernel
            .command(WorldCommand::AdvanceStrategicTick {
                expected_revision: campaign.revision,
                source: TickSource::Scheduler,
                plan: None,
                model_receipt_hash: Some(output.aggregate_receipt_hash.clone()),
                resolution_wave: Some(output.wave.clone()),
            })
            .await?;
        let CommandResult::Committed {
            campaign: advanced, ..
        } = &committed
        else {
            anyhow::bail!("strategic wave {wave_index} did not commit")
        };
        if advanced.actors[&advanced.player_actor_id] != player_before {
            anyhow::bail!("strategic wave {wave_index} puppeted the absent player")
        }
        let mut issue_campaign = advanced.clone();
        issue_campaign.news = advanced.news[previous_news_count..].to_vec();
        if issue_campaign.news.is_empty() {
            anyhow::bail!("strategic wave {wave_index} produced no gated news")
        }
        let issue_composition = compose_persisted_newspaper(
            model.as_ref(),
            &issue_campaign,
            format!("{newspaper_title} — Issue {wave_index}"),
            &newspaper_voice,
            5,
            &store,
        )
        .await;
        let (
            issue,
            newspaper_grounding,
            newspaper_model_receipts,
            issue_path,
            issue_audit_path,
            newspaper_error,
        ) = match issue_composition {
            Ok(composition) => {
                let issue_path = root.join(format!("newspaper-wave-{wave_index:02}.md"));
                let issue_audit_path =
                    root.join(format!("newspaper-wave-{wave_index:02}.audit.md"));
                std::fs::write(
                    &issue_path,
                    ghostlight_dungeon::newspaper::render_world_newspaper_markdown(
                        &composition.issue,
                    ),
                )?;
                std::fs::write(
                    &issue_audit_path,
                    ghostlight_dungeon::newspaper::render_world_newspaper_audit_markdown(
                        &composition.issue,
                    ),
                )?;
                (
                    Some(composition.issue),
                    Some(composition.grounding),
                    composition.model_receipts,
                    Some(issue_path),
                    Some(issue_audit_path),
                    None,
                )
            }
            Err(error) => {
                let Some(failure) = error.downcast_ref::<
                    ghostlight_dungeon::newspaper::WorldNewspaperCompositionFailure,
                >() else {
                    return Err(error);
                };
                (
                    None,
                    None,
                    failure.model_receipts.clone(),
                    None::<std::path::PathBuf>,
                    None::<std::path::PathBuf>,
                    Some(error.to_string()),
                )
            }
        };
        wave_reports.push(serde_json::json!({
            "wave_index":wave_index,
            "elapsed_seconds":started.elapsed().as_secs_f64(),
            "world_revision_before":campaign.revision,
            "world_revision_after":advanced.revision,
            "model_receipt_hash":output.aggregate_receipt_hash,
            "model_stage_receipts":output.stages.iter().map(|stage|&stage.receipt).collect::<Vec<_>>(),
            "rejected_pulses":rejected_pulses,
            "plan":plan,
            "commit":committed,
            "issue":issue,
            "newspaper_grounding":newspaper_grounding,
            "newspaper_model_receipts":newspaper_model_receipts,
            "newspaper_error":newspaper_error,
            "issue_path":issue_path,
            "issue_audit_path":issue_audit_path,
        }));
        campaign = advanced.clone();
        std::fs::write(
            root.join("status.json"),
            serde_json::to_vec_pretty(&serde_json::json!({
                "schema":"ghostlight.live_strategic_smoke_status.v1",
                "state":"running",
                "waves_completed":wave_index,
                "waves_requested":wave_count,
                "world_revision":campaign.revision,
                "event_count":campaign.events.len(),
                "news_count":campaign.news.len(),
                "updated_at":Utc::now(),
            }))?,
        )?;
    }
    let final_plan = final_wave_field(&wave_reports, "plan")?;
    let final_commit = final_wave_field(&wave_reports, "commit")?;
    let final_model_receipt_hash = final_wave_field(&wave_reports, "model_receipt_hash")?;
    let model_stage_receipts = wave_reports
        .iter()
        .flat_map(|wave| {
            wave["model_stage_receipts"]
                .as_array()
                .into_iter()
                .flatten()
                .cloned()
        })
        .collect::<Vec<_>>();
    let newspaper_composition = match compose_persisted_newspaper(
        model.as_ref(),
        &campaign,
        &newspaper_title,
        &newspaper_voice,
        6,
        &store,
    )
    .await
    {
        Ok(composition) => composition,
        Err(error) => {
            let final_newspaper_model_receipts = error
                .downcast_ref::<ghostlight_dungeon::newspaper::WorldNewspaperCompositionFailure>()
                .map(|failure| failure.model_receipts.clone())
                .unwrap_or_default();
            let failed_result = serde_json::json!({
                "schema":"ghostlight.live_strategic_smoke_failure.v1",
                "scenario_id":scenario_id,
                "pressure":pressure,
                "wave_count":wave_count,
                "campaign_id":campaign.id,
                "elapsed_seconds":started.elapsed().as_secs_f64(),
                "model_runtime":model_selection.status("configured"),
                "world_compile":&world_compile,
                "model_receipt_hash":&final_model_receipt_hash,
                "model_stage_receipts":&model_stage_receipts,
                "plan":&final_plan,
                "commit":&final_commit,
                "waves":&wave_reports,
                "event_count":campaign.events.len(),
                "news_count":campaign.news.len(),
                "final_newspaper_error":error.to_string(),
                "final_newspaper_model_receipts":final_newspaper_model_receipts,
                "player_location_unchanged":true,
                "player_state_unchanged":true,
                "store":root.join("campaign.cc")
            });
            let result_path = root.join("result.json");
            std::fs::write(&result_path, serde_json::to_vec_pretty(&failed_result)?)?;
            std::fs::write(
                root.join("status.json"),
                serde_json::to_vec_pretty(&serde_json::json!({
                    "schema":"ghostlight.live_strategic_smoke_status.v1",
                    "state":"failed",
                    "waves_completed":wave_count,
                    "waves_requested":wave_count,
                    "world_revision":campaign.revision,
                    "event_count":campaign.events.len(),
                    "news_count":campaign.news.len(),
                    "updated_at":Utc::now(),
                    "result_path":result_path,
                    "newspaper_error":error.to_string(),
                }))?,
            )?;
            return Err(error);
        }
    };
    let newspaper_path = root.join("newspaper.md");
    let newspaper_audit_path = root.join("newspaper.audit.md");
    std::fs::write(
        &newspaper_path,
        ghostlight_dungeon::newspaper::render_world_newspaper_markdown(
            &newspaper_composition.issue,
        ),
    )?;
    std::fs::write(
        &newspaper_audit_path,
        ghostlight_dungeon::newspaper::render_world_newspaper_audit_markdown(
            &newspaper_composition.issue,
        ),
    )?;
    let result = serde_json::json!({
        "schema":"ghostlight.live_strategic_smoke.v3",
        "scenario_id":scenario_id,
        "pressure":pressure,
        "wave_count":wave_count,
        "campaign_id":campaign.id,
        "elapsed_seconds":started.elapsed().as_secs_f64(),
        "model_runtime":model_selection.status("configured"),
        "world_compile":world_compile,
        "model_receipt_hash":final_model_receipt_hash,
        "model_stage_receipts":model_stage_receipts,
        "plan":final_plan,
        "commit":final_commit,
        "waves":wave_reports,
        "event_count":campaign.events.len(),
        "news_count":campaign.news.len(),
        "newspaper":newspaper_composition.issue,
        "newspaper_grounding":newspaper_composition.grounding,
        "newspaper_model_receipts":newspaper_composition.model_receipts,
        "newspaper_path":newspaper_path,
        "newspaper_audit_path":newspaper_audit_path,
        "player_location_unchanged":true,
        "player_state_unchanged":true,
        "store":root.join("campaign.cc")
    });
    std::fs::write(
        root.join("result.json"),
        serde_json::to_vec_pretty(&result)?,
    )?;
    std::fs::write(
        root.join("status.json"),
        serde_json::to_vec_pretty(&serde_json::json!({
            "schema":"ghostlight.live_strategic_smoke_status.v1",
            "state":"complete",
            "waves_completed":wave_count,
            "waves_requested":wave_count,
            "world_revision":campaign.revision,
            "event_count":campaign.events.len(),
            "news_count":campaign.news.len(),
            "updated_at":Utc::now(),
            "result_path":root.join("result.json"),
            "newspaper_path":&newspaper_path,
        }))?,
    )?;
    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}

async fn compose_persisted_newspaper(
    model: &dyn ghostlight_dungeon::model::ModelPort,
    campaign: &ghostlight_dungeon::domain::Campaign,
    title: impl Into<String>,
    editorial_voice: &str,
    max_articles: usize,
    store: &ghostlight_dungeon::persistence::CampaignStore,
) -> anyhow::Result<ghostlight_dungeon::newspaper::WorldNewspaperComposition> {
    let result = ghostlight_dungeon::newspaper::compose_world_newspaper(
        model,
        campaign,
        title,
        editorial_voice,
        max_articles,
    )
    .await;
    let receipts = match &result {
        Ok(composition) => Some(composition.model_receipts.as_slice()),
        Err(error) => error
            .downcast_ref::<ghostlight_dungeon::newspaper::WorldNewspaperCompositionFailure>()
            .map(|failure| failure.model_receipts.as_slice()),
    };
    if let Some(receipts) = receipts {
        if let Err(persistence_error) = store.persist_model_stage_receipts(receipts) {
            if let Err(error) = &result
                && let Some(failure) = error.downcast_ref::<
                    ghostlight_dungeon::newspaper::WorldNewspaperCompositionFailure,
                >()
            {
                return Err(anyhow::Error::new(
                    ghostlight_dungeon::newspaper::WorldNewspaperCompositionFailure {
                        message: format!(
                            "{}; model-receipt persistence failed: {persistence_error}",
                            failure.message
                        ),
                        model_receipts: failure.model_receipts.clone(),
                    },
                ));
            }
            return Err(persistence_error);
        }
    }
    result
}

async fn compile_strategic_campaign(
    model: std::sync::Arc<dyn ghostlight_dungeon::model::ModelPort>,
    retrieval_model: &str,
    compiler_model: &str,
    description: &str,
    pressure: &str,
    public_channel: &str,
) -> anyhow::Result<(
    ghostlight_dungeon::domain::WorldCompilePreview,
    Vec<ghostlight_dungeon::model::ModelStageReceipt>,
)> {
    use ghostlight_dungeon::compiler::{CustomStart, validate_campaign_seed};

    let description = description.trim();
    if description.chars().count() > 8_000 {
        anyhow::bail!("GHOSTLIGHT_WORLD_DESCRIPTION accepts at most 8,000 characters")
    }
    let world_name = std::env::var("GHOSTLIGHT_WORLD_NAME")
        .unwrap_or_else(|_| "The Elven Realms Beyond the Greathold".into());
    let who = std::env::var("GHOSTLIGHT_WORLD_PLAYER").unwrap_or_else(|_| {
        "The player-controlled Greathold, represented by a boundary observer; its sovereign choices remain external to the autonomous world simulation."
            .into()
    });
    let where_ = std::env::var("GHOSTLIGHT_WORLD_WHERE").unwrap_or_else(|_| {
        "the inhabited realms immediately beyond the Greathold boundary described by the supplied setting source"
            .into()
    });
    let when = strategic_world_when();
    let goal = format!(
        "Observe without ruling while the autonomous world responds to this new external pressure from the Greathold: {pressure}"
    );
    let compiler =
        strategic_world_compiler(model, retrieval_model, compiler_model, description, &when);
    let (mut preview, receipts) = compiler
        .compile_custom(CustomStart {
            campaign_name: world_name,
            who,
            where_,
            when,
            goal,
        })
        .await?;
    let campaign = &mut preview.campaign;
    campaign.resolution_policy.active_cell_budget = bounded_environment_usize(
        "GHOSTLIGHT_STRATEGIC_CELL_BUDGET",
        200,
        ghostlight_dungeon::resolution::MIN_ACTIVE_CELL_BUDGET as usize,
        ghostlight_dungeon::resolution::MAX_ACTIVE_CELL_BUDGET as usize,
    )? as u8;
    ghostlight_dungeon::resolution::ensure_agency_profiles(campaign);
    for (subject_id, profile) in &mut campaign.agency_profiles {
        profile.simulation_eligible = subject_id != &campaign.player_actor_id;
        profile.information_channels.insert(public_channel.into());
    }
    validate_campaign_seed(campaign)?;
    Ok((preview, receipts))
}

fn strategic_world_when() -> String {
    std::env::var("GHOSTLIGHT_WORLD_WHEN").unwrap_or_else(|_| {
        "a strained late age before any single realm has secured hegemony".into()
    })
}

fn strategic_world_compiler(
    model: std::sync::Arc<dyn ghostlight_dungeon::model::ModelPort>,
    retrieval_model: &str,
    compiler_model: &str,
    description: &str,
    temporal_scope: &str,
) -> ghostlight_dungeon::compiler::WorldCompiler {
    use ghostlight_dungeon::{domain::SourceWitness, vault::FixtureVault};
    use sha2::{Digest, Sha256};

    let witness = SourceWitness {
        source_id: "consumer-setting-description".into(),
        exact_locator: "consumer://setting-description".into(),
        content_hash: format!("sha256:{:x}", Sha256::digest(description.as_bytes())),
        excerpt: description.into(),
        authority_lane: "consumer.setting_description".into(),
        temporal_scope: temporal_scope.into(),
    };
    ghostlight_dungeon::compiler::WorldCompiler::new(
        std::sync::Arc::new(FixtureVault {
            witnesses: vec![witness],
        }),
        model,
        retrieval_model,
        compiler_model,
    )
}

fn strategic_locality_request(location_name: &str, location_id: &str, pressure: &str) -> String {
    let pressure = pressure.chars().take(140).collect::<String>();
    let request = format!(
        "Elaborate the existing canonical locality {location_name:?} (exact ID {location_id}) as a politically inhabited jurisdiction under this current crisis: {pressure}. Invent branch-local rival institutions and resident groups with authority, succession, revenue, redress, and concrete leverage. Give each a concrete public notice or report channel and enough opposed interests for autonomous conflict."
    );
    request.chars().take(500).collect()
}

fn admitted_public_channel(value: &str) -> anyhow::Result<String> {
    let channel = value.trim();
    if !ghostlight_dungeon::resolution::information_channel_is_concrete(channel) {
        anyhow::bail!("GHOSTLIGHT_STRATEGIC_PUBLIC_CHANNEL is not a concrete information route")
    }
    Ok(channel.into())
}

fn bounded_environment_usize(
    name: &str,
    default: usize,
    minimum: usize,
    maximum: usize,
) -> anyhow::Result<usize> {
    let value = std::env::var(name)
        .ok()
        .map(|value| value.parse::<usize>())
        .transpose()
        .map_err(|error| anyhow::anyhow!("{name} is not an integer: {error}"))?
        .unwrap_or(default);
    if !(minimum..=maximum).contains(&value) {
        anyhow::bail!("{name} must be between {minimum} and {maximum}")
    }
    Ok(value)
}

fn default_runtime_root() -> std::path::PathBuf {
    #[cfg(windows)]
    {
        std::path::PathBuf::from(r"F:\GameCult\GhostlightDungeon")
    }
    #[cfg(not(windows))]
    {
        std::path::PathBuf::from("/var/lib/gamecult/ghostlight-dungeon")
    }
}

fn strategic_campaign() -> ghostlight_dungeon::domain::Campaign {
    use chrono::{Duration, Utc};
    use ghostlight_dungeon::domain::*;
    use std::collections::{BTreeMap, BTreeSet};

    fn actor(id: &str, name: &str, location_id: &str, goal: &str) -> ActorState {
        ActorState {
            id: id.into(),
            name: name.into(),
            location_id: location_id.into(),
            capabilities: BTreeSet::from(["ordinary travel".into()]),
            knowledge: BTreeSet::new(),
            equipment: BTreeSet::new(),
            conditions: BTreeSet::new(),
            obligations: BTreeSet::new(),
            relationships: BTreeMap::new(),
            goals: vec![goal.into()],
            memories: vec![],
        }
    }
    let now = Utc::now();
    let player = actor(
        "player",
        "Deep-hold Envoy",
        "room",
        "observe without ruling",
    );
    let mut campaign = Campaign {
        schema: "ghostlight.campaign.v1".into(),
        id: uuid::Uuid::new_v4(),
        name: "The Rainless Marches".into(),
        revision: 0,
        branch_origin: BranchOrigin {
            canon_cutoff: "acceptance-fixture".into(),
            evidence_receipt_ids: vec![],
        },
        world_time: now,
        tick_hours: 6,
        player_actor_id: "player".into(),
        locations: BTreeMap::from([
            (
                "room".into(),
                Location {
                    id: "room".into(),
                    name: "Greathold Boundary Cairn".into(),
                    container_id: None,
                    routes: BTreeMap::new(),
                    persistent_features: vec![],
                },
            ),
            (
                "depot".into(),
                Location {
                    id: "depot".into(),
                    name: "Rootvault Granary".into(),
                    container_id: None,
                    routes: BTreeMap::from([(
                        "yard".into(),
                        Route {
                            destination_id: "yard".into(),
                            distance: "one hour".into(),
                            travel_minutes: 60,
                        },
                    )]),
                    persistent_features: vec![
                        "dry aquifer gauges".into(),
                        "a rain seal's empty reliquary".into(),
                    ],
                },
            ),
            (
                "yard".into(),
                Location {
                    id: "yard".into(),
                    name: "Thornweald Assembly".into(),
                    container_id: None,
                    routes: BTreeMap::from([(
                        "depot".into(),
                        Route {
                            destination_id: "depot".into(),
                            distance: "one hour".into(),
                            travel_minutes: 60,
                        },
                    )]),
                    persistent_features: vec![
                        "charcoal tithe scales".into(),
                        "three freshly painted secession banners".into(),
                    ],
                },
            ),
        ]),
        actors: BTreeMap::from([
            ("player".into(), player),
            (
                "runner".into(),
                actor(
                    "runner",
                    "Ilyra Quill",
                    "depot",
                    "find who sold the rain seal",
                ),
            ),
        ]),
        institutions: BTreeMap::from([
            (
                "board".into(),
                InstitutionState {
                    id: "board".into(),
                    name: "Mossglass Regency".into(),
                    resources: vec!["empty rain-seal reliquary".into()],
                    goals: vec!["survive the seal scandal without surrendering the throne".into()],
                    posture: "blaming unnamed caravan clerks".into(),
                },
            ),
            (
                "synod".into(),
                InstitutionState {
                    id: "synod".into(),
                    name: "Copper Synod".into(),
                    resources: vec!["tithe ledgers".into(), "three armed auditors".into()],
                    goals: vec!["make the regency pay for the vanished caravans".into()],
                    posture: "quietly pricing a replacement monarch".into(),
                },
            ),
        ]),
        clocks: BTreeMap::from([(
            "shortage".into(),
            WorldClock {
                id: "shortage".into(),
                label: "White Root succession crisis".into(),
                progress: 1,
                threshold: 4,
                consequence: "the charcoal guilds declare the regent ritually rainless".into(),
            },
        )]),
        facts: BTreeMap::new(),
        civic_systems: BTreeMap::new(),
        transcript: vec![],
        last_player_activity: now - Duration::hours(2),
        pending_ticks: 1,
        away_ticks_processed: 0,
        events: vec![],
        news: vec![],
        canon_candidates: BTreeMap::new(),
        gestalts: BTreeMap::from([(
            "workers".into(),
            GestaltPersonaState {
                schema: "ghostlight.gestalt_persona_state.v1".into(),
                id: "workers".into(),
                name: "Thornweald Charcoal Guilds".into(),
                version: 0,
                home_location_id: "yard".into(),
                shared_capabilities: BTreeSet::from(["close every forest kiln at once".into()]),
                shared_knowledge: BTreeSet::from([
                    "two tithe caravans vanished after the aquifer diversion".into(),
                    "the regent's rain seal is missing".into(),
                ]),
                resources: BTreeSet::from(["assembly grove".into()]),
                goals: vec!["replace tithe tribute with an elected water compact".into()],
                pressures: vec!["three guilds have already painted secession banners".into()],
            },
        )]),
        gestalt_members: BTreeMap::new(),
        pending_world_proposals: vec![],
        agency_profiles: BTreeMap::new(),
        agency_relations: BTreeMap::from([
            (
                "regency-synod-rivalry".into(),
                AgencyRelation {
                    schema: "ghostlight.agency_relation.v1".into(),
                    id: "regency-synod-rivalry".into(),
                    from_subject_id: "board".into(),
                    to_subject_id: "synod".into(),
                    kind: AgencyRelationKind::Rivalry,
                    strength: 86,
                    active: true,
                    evidence_receipt_ids: vec![],
                },
            ),
            (
                "synod-guild-command".into(),
                AgencyRelation {
                    schema: "ghostlight.agency_relation.v1".into(),
                    id: "synod-guild-command".into(),
                    from_subject_id: "synod".into(),
                    to_subject_id: "workers".into(),
                    kind: AgencyRelationKind::Command,
                    strength: 63,
                    active: true,
                    evidence_receipt_ids: vec![],
                },
            ),
        ]),
        gestalt_lineages: BTreeMap::new(),
        resolution_policy: Default::default(),
        resolution_pins: BTreeMap::new(),
        resolution_cover: None,
        strategic_tick_count: 0,
    };
    ghostlight_dungeon::resolution::ensure_agency_profiles(&mut campaign);
    for profile in campaign.agency_profiles.values_mut() {
        profile
            .information_channels
            .insert("root-wire broadsheet".into());
    }
    for institution_id in ["board", "synod"] {
        let profile = campaign
            .agency_profiles
            .get_mut(institution_id)
            .expect("fixture institution has a profile");
        profile.location_ids.extend(["depot".into(), "yard".into()]);
        profile
            .facets
            .entry(AgencyAxis::Geography)
            .or_default()
            .extend(["depot".into(), "yard".into()]);
    }
    campaign
}

#[cfg(test)]
mod tests {
    use super::{
        admitted_public_channel, final_wave_field, strategic_campaign, strategic_locality_request,
    };

    #[test]
    fn top_level_projection_uses_the_final_wave_head() {
        let waves = vec![
            serde_json::json!({"commit":{"campaign":{"revision":1}}}),
            serde_json::json!({"commit":{"campaign":{"revision":2}}}),
        ];

        let commit = final_wave_field(&waves, "commit").unwrap();
        assert_eq!(commit["campaign"]["revision"], 2);
    }

    #[test]
    fn public_channel_requires_one_concrete_information_route() {
        assert_eq!(
            admitted_public_channel("  root-wire broadsheet  ").unwrap(),
            "root-wire broadsheet"
        );
        for invalid in ["", "   ", "unknown"] {
            assert!(admitted_public_channel(invalid).is_err());
        }
        assert!(admitted_public_channel(&"x".repeat(161)).is_err());
    }

    #[test]
    fn locality_elaboration_request_names_the_existing_place_and_stays_bounded() {
        let request = strategic_locality_request(
            "Seed Vault",
            "loc-seed-vault",
            &"an intricately witnessed constitutional crisis ".repeat(40),
        );

        assert!(request.contains("Seed Vault"));
        assert!(request.contains("loc-seed-vault"));
        assert!(request.contains("authority, succession, revenue, redress"));
        assert!(request.contains("public notice or report channel"));
        assert!(request.chars().count() <= 500);
    }

    #[test]
    fn static_fixture_obeys_the_same_channel_and_knowledge_invariant() {
        let campaign = strategic_campaign();
        ghostlight_dungeon::compiler::validate_campaign_seed(&campaign).unwrap();
        let player = &campaign.actors[&campaign.player_actor_id];
        assert!(!player.knowledge.contains("root-wire broadsheet"));
        assert!(
            campaign.agency_profiles[&campaign.player_actor_id]
                .information_channels
                .contains("root-wire broadsheet")
        );
    }
}
