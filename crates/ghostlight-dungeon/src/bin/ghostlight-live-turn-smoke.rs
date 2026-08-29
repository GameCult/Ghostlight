#[cfg(not(windows))]
fn main() -> anyhow::Result<()> {
    anyhow::bail!("the live turn smoke uses Starfire's DPAPI credential")
}

#[cfg(windows)]
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    use chrono::Utc;
    use ghostlight_dungeon::{
        domain::{NarrativeTurn, WorldCommand},
        kernel::WorldKernel,
        model::{DeepSeekPort, ModelPort},
        persistence::CampaignStore,
        persona::PersonaProjectionEngine,
        turn::{SnapshotPermit, appraise_present},
    };
    use std::{path::PathBuf, sync::Arc, time::Instant};

    let secret = std::env::var_os("GHOSTLIGHT_DEEPSEEK_BLOB")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(r"F:\GameCult\GhostlightDungeon\secrets\deepseek.dpapi"));
    let scenario_id = std::env::var("GHOSTLIGHT_LIVE_FIRE_SCENARIO")
        .unwrap_or_else(|_| "live-turn-default".into());
    let event_summary = std::env::var("GHOSTLIGHT_LIVE_EVENT").unwrap_or_else(|_| {
        "The player asks all three witnesses what they believe is at stake.".into()
    });
    let root = std::env::var_os("GHOSTLIGHT_LIVE_FIRE_RESULT_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(r"F:\GameCult\GhostlightDungeon\acceptance").join(format!(
                "four-actor-{}-{}",
                Utc::now().format("%Y%m%d-%H%M%S"),
                uuid::Uuid::new_v4()
            ))
        });
    std::fs::create_dir_all(&root)?;
    let mut campaign = four_actor_campaign();
    campaign.transcript.push(NarrativeTurn {
        revision: campaign.revision,
        at: campaign.world_time,
        speaker: "world".into(),
        text: event_summary.clone(),
        persona_response_actor_ids: Default::default(),
    });
    let store = CampaignStore::open(root.join("campaign.cc"))?;
    store.create_unadmitted_fixture_campaign(&campaign, &[], &[])?;
    let model: Arc<dyn ModelPort> = Arc::new(DeepSeekPort::from_runtime_secret(secret)?);
    let engine = PersonaProjectionEngine {
        model,
        permit: Arc::new(SnapshotPermit::new(store.clone(), campaign.id, 0)),
        projector_model: "deepseek-v4-flash".into(),
        persona_model: "deepseek-v4-pro".into(),
        interpreter_model: "deepseek-v4-flash".into(),
    };
    let started = Instant::now();
    let wave = appraise_present(engine, &campaign, &event_summary).await?;
    let inference_seconds = started.elapsed().as_secs_f64();
    if wave.reactions.len() != 3 || wave.receipts.len() != 9 {
        anyhow::bail!(
            "four-actor wave returned {} reactions and {} receipts",
            wave.reactions.len(),
            wave.receipts.len()
        );
    }
    for receipt in &wave.receipts {
        store.insert(
            "persona_stage_receipt.v1",
            "ghostlight.persona_stage_receipt.v1",
            receipt.storage_key(),
            receipt,
        )?;
    }
    let kernel = WorldKernel::start(store.clone());
    let committed = kernel
        .command(WorldCommand::ResolveReactionWave {
            expected_revision: 0,
            event_summary: event_summary.clone(),
            reactions: wave.reactions.clone(),
            gestalt_reactions: wave.gestalt_reactions.clone(),
        })
        .await?;
    let total_seconds = started.elapsed().as_secs_f64();
    let result = serde_json::json!({
        "schema":"ghostlight.live_turn_smoke.v1",
        "scenario_id":scenario_id,
        "event_summary":event_summary,
        "campaign_id":campaign.id,
        "actor_count":campaign.actors.len(),
        "reaction_count":3,
        "stage_receipt_count":wave.receipts.len(),
        "inference_seconds":inference_seconds,
        "total_seconds":total_seconds,
        "target_seconds":20,
        "within_target":total_seconds <= 20.0,
        "reactions":wave.reactions,
        "model_stage_receipts":wave.receipts,
        "commit":committed,
        "store":root.join("campaign.cc")
    });
    std::fs::write(
        root.join("result.json"),
        serde_json::to_vec_pretty(&result)?,
    )?;
    println!("{}", serde_json::to_string_pretty(&result)?);
    if total_seconds > 20.0 {
        anyhow::bail!("four-actor live turn exceeded 20 second target");
    }
    Ok(())
}

#[cfg(windows)]
fn four_actor_campaign() -> ghostlight_dungeon::domain::Campaign {
    use chrono::Utc;
    use ghostlight_dungeon::domain::{ActorState, BranchOrigin, Campaign, Location};
    use std::collections::{BTreeMap, BTreeSet};

    fn actor(id: &str, name: &str, goal: &str, knowledge: &str) -> ActorState {
        ActorState {
            id: id.into(),
            name: name.into(),
            location_id: "hall".into(),
            capabilities: BTreeSet::from(["ordinary conversation".into()]),
            knowledge: BTreeSet::from([knowledge.into()]),
            equipment: BTreeSet::new(),
            conditions: BTreeSet::new(),
            obligations: BTreeSet::new(),
            relationships: BTreeMap::new(),
            goals: vec![goal.into()],
            memories: vec![],
        }
    }
    let id = uuid::Uuid::new_v4();
    let now = Utc::now();
    Campaign {
        schema: "ghostlight.campaign.v1".into(),
        id,
        name: "Four actor live acceptance".into(),
        revision: 0,
        branch_origin: BranchOrigin {
            canon_cutoff: "acceptance-fixture".into(),
            evidence_receipt_ids: vec![],
        },
        world_time: now,
        tick_hours: 6,
        player_actor_id: "player".into(),
        locations: BTreeMap::from([(
            "hall".into(),
            Location {
                id: "hall".into(),
                name: "Dispute hall".into(),
                container_id: None,
                routes: BTreeMap::new(),
                persistent_features: vec!["four people can see and hear one another".into()],
            },
        )]),
        actors: BTreeMap::from([
            (
                "player".into(),
                actor(
                    "player",
                    "Mediator",
                    "understand the dispute",
                    "the question just asked",
                ),
            ),
            (
                "witness-a".into(),
                actor(
                    "witness-a",
                    "Asha",
                    "protect the workers",
                    "the workers missed two supply deliveries",
                ),
            ),
            (
                "witness-b".into(),
                actor(
                    "witness-b",
                    "Beren",
                    "protect the station",
                    "the station has one reserve shipment",
                ),
            ),
            (
                "witness-c".into(),
                actor(
                    "witness-c",
                    "Cira",
                    "prevent violence",
                    "both factions have begun recruiting guards",
                ),
            ),
        ]),
        institutions: BTreeMap::new(),
        clocks: BTreeMap::new(),
        facts: BTreeMap::new(),
        civic_systems: BTreeMap::new(),
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
        nemesis_attention_history: Vec::new(),
        strategic_tick_count: 0,
    }
}
