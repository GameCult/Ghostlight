#[cfg(not(windows))]
fn main() -> anyhow::Result<()> {
    anyhow::bail!("the live strategic smoke uses Starfire's DPAPI credential")
}

#[cfg(windows)]
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    use chrono::Utc;
    use ghostlight_dungeon::{
        domain::{TickSource, WorldCommand},
        kernel::{CommandResult, WorldKernel},
        model::{DeepSeekPort, ModelPort},
        persistence::CampaignStore,
        scheduler::propose_resolution_wave,
        turn::SnapshotPermit,
    };
    use std::{path::PathBuf, sync::Arc, time::Instant};

    let secret = std::env::var_os("GHOSTLIGHT_DEEPSEEK_BLOB")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(r"F:\GameCult\GhostlightDungeon\secrets\deepseek.dpapi"));
    let scenario_id = std::env::var("GHOSTLIGHT_LIVE_FIRE_SCENARIO")
        .unwrap_or_else(|_| "strategic-default".into());
    let pressure = std::env::var("GHOSTLIGHT_STRATEGIC_PRESSURE").unwrap_or_else(|_| {
        "Two reserve deliveries are missing and the next yard shift may refuse work.".into()
    });
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
    campaign.events.push(ghostlight_dungeon::domain::Event {
        id: format!("pressure-{}", uuid::Uuid::new_v4()),
        at: campaign.world_time,
        kind: "strategic_pressure".into(),
        summary: pressure.clone(),
        actor_ids: vec!["runner".into()],
        institution_ids: vec!["board".into()],
        location_ids: vec!["depot".into(), "yard".into()],
        public_channels: vec!["station radio".into()],
    });
    let player_location = campaign.actors[&campaign.player_actor_id]
        .location_id
        .clone();
    let store = CampaignStore::open(root.join("campaign.cc"))?;
    store.create_campaign(&campaign, &[], &[])?;
    let model: Arc<dyn ModelPort> = Arc::new(DeepSeekPort::from_machine_dpapi(secret)?);
    let started = Instant::now();
    let output = propose_resolution_wave(
        model,
        Arc::new(SnapshotPermit::new_resolution(
            store.clone(),
            campaign.id,
            campaign.revision,
            campaign.resolution_policy.resolution_epoch,
        )),
        &campaign,
    )
    .await?;
    let plan = ghostlight_dungeon::resolution::validate_and_resolve_wave(&campaign, &output.wave)?;
    if plan.institution_actions.is_empty()
        && plan.gestalt_actions.is_empty()
        && plan.actor_moves.is_empty()
    {
        anyhow::bail!(
            "strategic model proposed no material offscreen change: {}",
            "all cells explicitly chose inaction"
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
    let kernel = WorldKernel::start(store.clone());
    let committed = kernel
        .command(WorldCommand::AdvanceStrategicTick {
            expected_revision: 0,
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
        anyhow::bail!("strategic command did not commit")
    };
    if advanced.actors[&advanced.player_actor_id].location_id != player_location {
        anyhow::bail!("strategic tick puppeted the absent player")
    }
    if advanced.news.is_empty() {
        anyhow::bail!("accessible offscreen events produced no gated news")
    }
    let result = serde_json::json!({
        "schema":"ghostlight.live_strategic_smoke.v1",
        "scenario_id":scenario_id,
        "pressure":pressure,
        "campaign_id":campaign.id,
        "elapsed_seconds":started.elapsed().as_secs_f64(),
        "model_receipt_hash":output.aggregate_receipt_hash,
        "model_stage_receipts":output.stages.iter().map(|stage|&stage.receipt).collect::<Vec<_>>(),
        "plan":plan,
        "event_count":advanced.events.len(),
        "news_count":advanced.news.len(),
        "player_location_unchanged":true,
        "commit":committed,
        "store":root.join("campaign.cc")
    });
    std::fs::write(
        root.join("result.json"),
        serde_json::to_vec_pretty(&result)?,
    )?;
    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}

#[cfg(windows)]
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
    let mut player = actor("player", "Mediator", "room", "rest");
    player.knowledge.insert("station radio".into());
    let mut campaign = Campaign {
        schema: "ghostlight.campaign.v1".into(),
        id: uuid::Uuid::new_v4(),
        name: "Strategic live acceptance".into(),
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
                    name: "Player quarters".into(),
                    container_id: None,
                    routes: BTreeMap::new(),
                    persistent_features: vec![],
                },
            ),
            (
                "depot".into(),
                Location {
                    id: "depot".into(),
                    name: "Supply depot".into(),
                    container_id: None,
                    routes: BTreeMap::from([(
                        "yard".into(),
                        Route {
                            destination_id: "yard".into(),
                            distance: "one hour".into(),
                            travel_minutes: 60,
                        },
                    )]),
                    persistent_features: vec!["sealed reserve crates".into()],
                },
            ),
            (
                "yard".into(),
                Location {
                    id: "yard".into(),
                    name: "Workers' yard".into(),
                    container_id: None,
                    routes: BTreeMap::from([(
                        "depot".into(),
                        Route {
                            destination_id: "depot".into(),
                            distance: "one hour".into(),
                            travel_minutes: 60,
                        },
                    )]),
                    persistent_features: vec!["idle loading cranes".into()],
                },
            ),
        ]),
        actors: BTreeMap::from([
            ("player".into(), player),
            (
                "runner".into(),
                actor("runner", "Depot runner", "depot", "warn the workers"),
            ),
        ]),
        institutions: BTreeMap::from([(
            "board".into(),
            InstitutionState {
                id: "board".into(),
                name: "Station board".into(),
                resources: vec!["reserve shipment".into()],
                goals: vec!["contain the supply dispute".into()],
                posture: "deliberating".into(),
            },
        )]),
        clocks: BTreeMap::from([(
            "shortage".into(),
            WorldClock {
                id: "shortage".into(),
                label: "Supply shortage".into(),
                progress: 1,
                threshold: 4,
                consequence: "the yard stops work".into(),
            },
        )]),
        facts: BTreeMap::new(),
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
                name: "Yard workers".into(),
                version: 0,
                home_location_id: "yard".into(),
                shared_capabilities: BTreeSet::from(["operate loading cranes".into()]),
                shared_knowledge: BTreeSet::from(["two deliveries are missing".into()]),
                resources: BTreeSet::from(["union hall".into()]),
                goals: vec!["secure the missing supplies".into()],
                pressures: vec!["the next shift may refuse work".into()],
            },
        )]),
        gestalt_members: BTreeMap::new(),
        pending_world_proposals: vec![],
        agency_profiles: BTreeMap::new(),
        agency_relations: BTreeMap::new(),
        gestalt_lineages: BTreeMap::new(),
        resolution_policy: Default::default(),
        resolution_pins: BTreeMap::new(),
        resolution_cover: None,
        strategic_tick_count: 0,
    };
    ghostlight_dungeon::resolution::ensure_agency_profiles(&mut campaign);
    for profile in campaign.agency_profiles.values_mut() {
        profile.information_channels.insert("station radio".into());
    }
    if let Some(board) = campaign.agency_profiles.get_mut("board") {
        board.location_ids.extend(["depot".into(), "yard".into()]);
        board
            .facets
            .entry(AgencyAxis::Geography)
            .or_default()
            .extend(["depot".into(), "yard".into()]);
    }
    campaign
}
