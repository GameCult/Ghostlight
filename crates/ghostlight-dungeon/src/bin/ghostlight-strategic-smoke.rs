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
        scheduler::propose_resolution_wave,
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
    let mut campaign = strategic_campaign();
    let pressure_event = ghostlight_dungeon::domain::Event {
        id: format!("pressure-{}", uuid::Uuid::new_v4()),
        at: campaign.world_time,
        kind: "strategic_pressure".into(),
        summary: pressure.clone(),
        actor_ids: vec!["runner".into()],
        institution_ids: vec!["board".into(), "synod".into()],
        gestalt_ids: vec![],
        location_ids: vec!["depot".into(), "yard".into()],
        public_channels: vec!["root-wire broadsheet".into()],
    };
    campaign.news.push(ghostlight_dungeon::domain::NewsIssue {
        id: format!("news:{}:root-wire-broadsheet", pressure_event.id),
        at: pressure_event.at,
        channel: "root-wire broadsheet".into(),
        headline: ghostlight_dungeon::domain::committed_news_headline(&pressure_event.summary),
        event_ids: vec![pressure_event.id.clone()],
        reliability: "committed public channel".into(),
    });
    campaign.events.push(pressure_event);
    let player_before = campaign.actors[&campaign.player_actor_id].clone();
    let store = CampaignStore::open(root.join("campaign.cc"))?;
    store.create_unadmitted_fixture_campaign(&campaign, &[], &[])?;
    let model = model_selection.open()?.ok_or_else(|| {
        anyhow::anyhow!(
            "{} credential is unavailable at {}",
            model_selection.provider,
            model_selection.credential_path.display()
        )
    })?;
    let newspaper_title = std::env::var("GHOSTLIGHT_STRATEGIC_NEWSPAPER_TITLE")
        .unwrap_or_else(|_| "The Underdeep Clarion".into());
    let newspaper_voice = std::env::var("GHOSTLIGHT_STRATEGIC_NEWSPAPER_VOICE")
        .unwrap_or_else(|_| {
            "A sharp regional broadsheet for readers who already understand guild politics: skeptical of every throne, attentive to labor and material consequences, formally reported, and capable of one dry local barb without becoming satire."
                .into()
        });
    let started = Instant::now();
    let kernel = WorldKernel::start(store.clone());
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
                Err(error) if rejected_pulses.len() < max_rejected_pulses_per_wave => {
                    let pulse = rejected_pulses.len() + 1;
                    std::fs::write(
                        root.join(format!(
                            "wave-{wave_index:02}-rejected-pulse-{pulse:02}.txt"
                        )),
                        error.to_string(),
                    )?;
                    rejected_pulses.push(serde_json::json!({
                        "pulse":pulse,
                        "world_revision":campaign.revision,
                        "resolution_epoch":campaign.resolution_policy.resolution_epoch,
                        "error":error.to_string(),
                    }));
                }
                Err(error) => return Err(error),
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
    let mut player = actor(
        "player",
        "Deep-hold Envoy",
        "room",
        "observe without ruling",
    );
    player.knowledge.insert("root-wire broadsheet".into());
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
    use super::final_wave_field;

    #[test]
    fn top_level_projection_uses_the_final_wave_head() {
        let waves = vec![
            serde_json::json!({"commit":{"campaign":{"revision":1}}}),
            serde_json::json!({"commit":{"campaign":{"revision":2}}}),
        ];

        let commit = final_wave_field(&waves, "commit").unwrap();
        assert_eq!(commit["campaign"]["revision"], 2);
    }
}
