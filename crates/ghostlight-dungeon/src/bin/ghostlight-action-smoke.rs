#[cfg(not(windows))]
fn main() -> anyhow::Result<()> {
    anyhow::bail!("the live action smoke uses Starfire's DPAPI credential")
}

#[cfg(windows)]
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    use chrono::Utc;
    use ghostlight_dungeon::{
        assessor::ActionAssessor,
        domain::{ActionIntent, Campaign, WorldCommand},
        kernel::{CommandResult, KernelError, WorldKernel},
        model::{DeepSeekPort, ModelPort},
        narrator::Narrator,
        persistence::CampaignStore,
    };
    use std::{path::PathBuf, sync::Arc, time::Instant};

    let secret = std::env::var_os("GHOSTLIGHT_DEEPSEEK_BLOB")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(r"F:\GameCult\GhostlightDungeon\secrets\deepseek.dpapi"));
    let scenario_id =
        std::env::var("GHOSTLIGHT_LIVE_FIRE_SCENARIO").unwrap_or_else(|_| "action-default".into());
    let root = std::env::var_os("GHOSTLIGHT_LIVE_FIRE_RESULT_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(r"F:\GameCult\GhostlightDungeon\acceptance").join(format!(
                "action-{}-{}",
                Utc::now().format("%Y%m%d-%H%M%S"),
                uuid::Uuid::new_v4()
            ))
        });
    std::fs::create_dir_all(&root)?;
    let campaign = action_campaign();
    let store = CampaignStore::open(root.join("campaign.cc"))?;
    store.create_campaign(&campaign, &[], &[])?;
    let model: Arc<dyn ModelPort> = Arc::new(DeepSeekPort::from_runtime_secret(secret)?);
    let assessor = ActionAssessor::new(model.clone(), "deepseek-v4-pro");
    let kernel = WorldKernel::start(store.clone());
    let started = Instant::now();

    let impossible_intent = ActionIntent {
        actor_id: "player".into(),
        description: std::env::var("GHOSTLIGHT_IMPOSSIBLE_DESCRIPTION").unwrap_or_else(|_| {
            "I teleport the entire station into the sun by force of will.".into()
        }),
        intended_effect: std::env::var("GHOSTLIGHT_IMPOSSIBLE_EFFECT")
            .unwrap_or_else(|_| "destroy the station instantly".into()),
    };
    let (impossible, impossible_receipt) = assessor
        .assess(&campaign, impossible_intent.clone())
        .await?;
    if impossible.admissible || impossible.missing_permission.is_none() {
        anyhow::bail!("impossible action was offered a roll")
    }
    let CommandResult::Assessed {
        assessment: admitted_impossible,
    } = kernel
        .command(WorldCommand::Assess {
            expected_revision: 0,
            intent: impossible_intent,
            proposal: Some(impossible.clone()),
        })
        .await?
    else {
        anyhow::bail!("kernel did not return the impossible assessment")
    };
    let impossible_error = kernel
        .command(WorldCommand::Attempt {
            actor_id: "player".into(),
            assessment_digest: admitted_impossible.digest,
        })
        .await
        .expect_err("impossible attempt must not roll");
    if !matches!(impossible_error, KernelError::Impossible(_)) {
        anyhow::bail!("impossible attempt failed for the wrong reason: {impossible_error}")
    }
    let after_impossible = store
        .load::<Campaign>("campaign.v1", &campaign.id.to_string())?
        .map(|(_, value)| value)
        .ok_or_else(|| anyhow::anyhow!("campaign disappeared"))?;
    if after_impossible.revision != 0 {
        anyhow::bail!("impossible attempt mutated the campaign")
    }

    let feasible_intent = ActionIntent {
        actor_id: "player".into(),
        description: std::env::var("GHOSTLIGHT_FEASIBLE_DESCRIPTION").unwrap_or_else(|_| {
            "I connect my multimeter to the accessible coolant panel and inspect its readings."
                .into()
        }),
        intended_effect: std::env::var("GHOSTLIGHT_FEASIBLE_EFFECT").unwrap_or_else(|_| {
            "identify whether the coolant fault is electrical without changing the machinery".into()
        }),
    };
    let (feasible, feasible_receipt) = assessor
        .assess(&after_impossible, feasible_intent.clone())
        .await?;
    if !feasible.admissible {
        anyhow::bail!(
            "grounded maintenance attempt was rejected: {:?}",
            feasible.missing_permission
        )
    }
    let CommandResult::Assessed {
        assessment: admitted_feasible,
    } = kernel
        .command(WorldCommand::Assess {
            expected_revision: 0,
            intent: feasible_intent,
            proposal: Some(feasible.clone()),
        })
        .await?
    else {
        anyhow::bail!("kernel did not return the feasible assessment")
    };
    let attempt = kernel
        .command(WorldCommand::Attempt {
            actor_id: "player".into(),
            assessment_digest: admitted_feasible.digest,
        })
        .await?;
    let post = store
        .load::<Campaign>("campaign.v1", &campaign.id.to_string())?
        .map(|(_, value)| value)
        .ok_or_else(|| anyhow::anyhow!("campaign disappeared after attempt"))?;
    if post.revision != 1 {
        anyhow::bail!("confirmed attempt did not commit exactly one revision")
    }

    for receipt in [&impossible_receipt, &feasible_receipt] {
        store.insert(
            "persona_stage_receipt.v1",
            "ghostlight.persona_stage_receipt.v1",
            receipt.storage_key(),
            receipt,
        )?;
    }
    let narrator = Narrator {
        model,
        model_name: "deepseek-v4-pro".into(),
    };
    let (narration, narration_receipt) = narrator.project(&store, &post).await?;
    store.insert(
        "persona_stage_receipt.v1",
        "ghostlight.persona_stage_receipt.v1",
        narration_receipt.storage_key(),
        &narration_receipt,
    )?;

    let result = serde_json::json!({
        "schema":"ghostlight.action_smoke.v1",
        "scenario_id":scenario_id,
        "elapsed_seconds":started.elapsed().as_secs_f64(),
        "impossible_assessment":impossible,
        "impossible_attempt_error":impossible_error.to_string(),
        "feasible_assessment":feasible,
        "attempt":attempt,
        "narration":narration,
        "model_stage_receipts":[impossible_receipt, feasible_receipt, narration_receipt],
        "campaign_revision":post.revision,
        "store":root.join("campaign.cc"),
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
fn action_campaign() -> ghostlight_dungeon::domain::Campaign {
    use chrono::Utc;
    use ghostlight_dungeon::domain::{
        ActorState, BranchOrigin, Campaign, InstitutionState, Location, WorldClock,
    };
    use std::collections::{BTreeMap, BTreeSet};

    let id = uuid::Uuid::new_v4();
    let now = Utc::now();
    let player = ActorState {
        id: "player".into(),
        name: "Dana Voss".into(),
        location_id: "maintenance-bay".into(),
        capabilities: BTreeSet::from([
            "basic electrical diagnosis".into(),
            "safe multimeter use".into(),
        ]),
        knowledge: BTreeSet::from([
            "the coolant alarm began this shift".into(),
            "the accessible panel exposes diagnostic test points".into(),
        ]),
        equipment: BTreeSet::from(["calibrated multimeter".into()]),
        conditions: BTreeSet::from(["tired".into()]),
        obligations: BTreeSet::from(["report hazards to the shift foreman".into()]),
        relationships: BTreeMap::new(),
        goals: vec!["prevent a coolant failure without damaging the system".into()],
        memories: vec![],
    };
    let foreman = ActorState {
        id: "foreman".into(),
        name: "Ilyan Kesh".into(),
        location_id: "maintenance-bay".into(),
        capabilities: BTreeSet::from(["authorize invasive maintenance".into()]),
        knowledge: BTreeSet::from(["the work-order backlog".into()]),
        equipment: BTreeSet::new(),
        conditions: BTreeSet::new(),
        obligations: BTreeSet::from(["keep the shift on schedule".into()]),
        relationships: BTreeMap::new(),
        goals: vec!["avoid an unscheduled shutdown".into()],
        memories: vec![],
    };
    Campaign {
        schema: "ghostlight.campaign.v1".into(),
        id,
        name: "Action resolution acceptance".into(),
        revision: 0,
        branch_origin: BranchOrigin {
            canon_cutoff: "acceptance-fixture".into(),
            evidence_receipt_ids: vec![],
        },
        world_time: now,
        tick_hours: 6,
        player_actor_id: "player".into(),
        locations: BTreeMap::from([(
            "maintenance-bay".into(),
            Location {
                id: "maintenance-bay".into(),
                name: "Coolant Maintenance Bay".into(),
                container_id: None,
                routes: BTreeMap::new(),
                persistent_features: vec![
                    "an accessible coolant diagnostic panel".into(),
                    "a sealed valve housing requiring foreman authorization".into(),
                ],
            },
        )]),
        actors: BTreeMap::from([("player".into(), player), ("foreman".into(), foreman)]),
        institutions: BTreeMap::from([(
            "station-operations".into(),
            InstitutionState {
                id: "station-operations".into(),
                name: "Station Operations".into(),
                resources: vec!["maintenance authority".into()],
                goals: vec!["avoid shutdown".into()],
                posture: "dismissive of unscheduled work".into(),
            },
        )]),
        clocks: BTreeMap::from([(
            "coolant-failure".into(),
            WorldClock {
                id: "coolant-failure".into(),
                label: "Coolant failure".into(),
                progress: 2,
                threshold: 5,
                consequence: "the coolant loop trips offline".into(),
            },
        )]),
        facts: BTreeMap::new(),
        transcript: vec![],
        last_player_activity: now,
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
