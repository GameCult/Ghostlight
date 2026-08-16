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
        scheduler::propose_strategic_tick,
    };
    use std::{path::PathBuf, sync::Arc, time::Instant};

    let secret = std::env::var_os("GHOSTLIGHT_DEEPSEEK_BLOB")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(r"F:\GameCult\GhostlightDungeon\secrets\deepseek.dpapi"));
    let root = PathBuf::from(r"F:\GameCult\GhostlightDungeon\acceptance")
        .join(format!("strategic-{}", Utc::now().format("%Y%m%d-%H%M%S")));
    std::fs::create_dir_all(&root)?;
    let campaign = strategic_campaign();
    let player_location = campaign.actors[&campaign.player_actor_id]
        .location_id
        .clone();
    let store = CampaignStore::open(root.join("campaign.cc"))?;
    store.create_campaign(&campaign, &[], &[])?;
    let model: Arc<dyn ModelPort> = Arc::new(DeepSeekPort::from_machine_dpapi(secret)?);
    let started = Instant::now();
    let (plan, output) = propose_strategic_tick(model.as_ref(), &campaign).await?;
    if plan.institution_actions.is_empty()
        && plan.gestalt_actions.is_empty()
        && plan.actor_moves.is_empty()
    {
        anyhow::bail!(
            "strategic model proposed no material offscreen change: {}",
            output.narrative
        );
    }
    store.insert(
        "persona_stage_receipt.v1",
        "ghostlight.persona_stage_receipt.v1",
        &output.receipt.output_hash,
        &output.receipt,
    )?;
    let kernel = WorldKernel::start(store.clone());
    let committed = kernel
        .command(WorldCommand::AdvanceStrategicTick {
            expected_revision: 0,
            source: TickSource::Scheduler,
            plan: Some(plan.clone()),
            model_receipt_hash: Some(output.receipt.output_hash.clone()),
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
        "campaign_id":campaign.id,
        "elapsed_seconds":started.elapsed().as_secs_f64(),
        "model_receipt_hash":output.receipt.output_hash,
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
    Campaign {
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
    }
}
