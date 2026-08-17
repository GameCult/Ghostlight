#[cfg(not(windows))]
fn main() -> anyhow::Result<()> {
    anyhow::bail!("the live gestalt scale smoke uses Starfire's DPAPI credential")
}

#[cfg(windows)]
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    use chrono::Utc;
    use ghostlight_dungeon::{
        domain::{SimulationCellMode, TickSource, WorldCommand},
        kernel::{CommandResult, WorldKernel},
        model::{DeepSeekPort, ModelPort},
        persistence::CampaignStore,
        resolution::validate_and_resolve_wave,
        scheduler::propose_resolution_wave,
        turn::SnapshotPermit,
    };
    use std::{collections::BTreeSet, path::PathBuf, sync::Arc, time::Instant};

    let secret = std::env::var_os("GHOSTLIGHT_DEEPSEEK_BLOB")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(r"F:\GameCult\GhostlightDungeon\secrets\deepseek.dpapi"));
    let root = PathBuf::from(r"F:\GameCult\GhostlightDungeon\acceptance").join(format!(
        "gestalt-scale-{}",
        Utc::now().format("%Y%m%d-%H%M%S")
    ));
    std::fs::create_dir_all(&root)?;
    let campaign = scale_campaign();
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

    let preflight = serde_json::json!({
        "schema":"ghostlight.gestalt_scale_preflight.v1",
        "campaign_id":campaign.id,
        "elapsed_seconds":started.elapsed().as_secs_f64(),
        "cover":&output.wave.cover,
        "appraisals":&output.wave.appraisals,
        "model_stage_receipts":output.stages.iter().map(|stage|&stage.receipt).collect::<Vec<_>>()
    });
    std::fs::write(
        root.join("preflight.json"),
        serde_json::to_vec_pretty(&preflight)?,
    )?;

    let cover = &output.wave.cover;
    if cover.cells.len() != 4 || cover.effective_budget != 4 {
        anyhow::bail!(
            "budget-4 cover produced {} cells at effective budget {}",
            cover.cells.len(),
            cover.effective_budget
        )
    }
    let covered = cover
        .cells
        .iter()
        .flat_map(|cell| cell.subject_ids.iter().cloned())
        .collect::<Vec<_>>();
    if covered.len() != 24 || covered.iter().collect::<BTreeSet<_>>().len() != 24 {
        anyhow::bail!("scale cover omitted or duplicated canonical subjects")
    }
    let arena_count = cover
        .cells
        .iter()
        .filter(|cell| cell.mode == SimulationCellMode::Arena)
        .count();
    if arena_count == 0 {
        anyhow::bail!("cross-faction budget pressure produced no arena cells")
    }
    let plan = validate_and_resolve_wave(&campaign, &output.wave)?;
    if plan.institution_actions.is_empty() {
        anyhow::bail!("24-subject strategic wave proposed no material institution action")
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
        anyhow::bail!("scale wave did not commit")
    };
    if advanced.actors[&advanced.player_actor_id].location_id != player_location {
        anyhow::bail!("scale wave puppeted the absent player")
    }
    let result = serde_json::json!({
        "schema":"ghostlight.gestalt_scale_smoke.v1",
        "campaign_id":campaign.id,
        "subject_count":24,
        "configured_budget":4,
        "cell_count":cover.cells.len(),
        "arena_count":arena_count,
        "elapsed_seconds":started.elapsed().as_secs_f64(),
        "cover":cover,
        "appraisals":output.wave.appraisals,
        "plan":plan,
        "model_stage_receipts":output.stages.iter().map(|stage|&stage.receipt).collect::<Vec<_>>(),
        "commit":committed,
        "player_location_unchanged":true,
        "news_count":advanced.news.len(),
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
fn scale_campaign() -> ghostlight_dungeon::domain::Campaign {
    use chrono::{Duration, Utc};
    use ghostlight_dungeon::domain::*;
    use std::collections::{BTreeMap, BTreeSet};

    let now = Utc::now();
    let player = ActorState {
        id: "player".into(),
        name: "Observer".into(),
        location_id: "forum".into(),
        capabilities: BTreeSet::new(),
        knowledge: BTreeSet::from(["public bulletin".into()]),
        equipment: BTreeSet::new(),
        conditions: BTreeSet::new(),
        obligations: BTreeSet::new(),
        relationships: BTreeMap::new(),
        goals: vec!["observe the election without being puppeted".into()],
        memories: vec![],
    };
    let institutions = (0..24)
        .map(|index| {
            let id = format!("faction-{index:02}");
            (
                id.clone(),
                InstitutionState {
                    id,
                    name: format!("Faction {index:02}"),
                    resources: vec![
                        format!("private reserve {index:02}"),
                        format!("regional office {}", index % 4),
                        "public bulletin publishing access".into(),
                    ],
                    goals: vec![
                        format!("advance platform {}", index % 6),
                        "publish a concrete commitment before the final vote".into(),
                    ],
                    posture: format!(
                        "final vote in six hours; campaigning bloc {} must choose whether to commit its reserve",
                        index % 3
                    ),
                },
            )
        })
        .collect();
    let mut campaign = Campaign {
        schema: "ghostlight.campaign.v1".into(),
        id: uuid::Uuid::new_v4(),
        name: "Twenty-four faction gestalt acceptance".into(),
        revision: 0,
        branch_origin: BranchOrigin {
            canon_cutoff: "acceptance-fixture".into(),
            evidence_receipt_ids: vec![],
        },
        world_time: now,
        tick_hours: 6,
        player_actor_id: "player".into(),
        locations: BTreeMap::from([(
            "forum".into(),
            Location {
                id: "forum".into(),
                name: "Federation Forum".into(),
                container_id: None,
                routes: BTreeMap::new(),
                persistent_features: vec!["a public election bulletin".into()],
            },
        )]),
        actors: BTreeMap::from([("player".into(), player)]),
        institutions,
        clocks: BTreeMap::from([(
            "election".into(),
            WorldClock {
                id: "election".into(),
                label: "Federation election".into(),
                progress: 5,
                threshold: 6,
                consequence: "the governing coalition is chosen".into(),
            },
        )]),
        facts: BTreeMap::new(),
        transcript: vec![],
        last_player_activity: now - Duration::hours(2),
        pending_ticks: 1,
        away_ticks_processed: 0,
        events: vec![Event {
            id: "final-vote-notice".into(),
            at: now,
            kind: "public_notice".into(),
            summary: "The public bulletin announces that the final vote occurs in six hours; each faction has one last chance to publish a binding commitment.".into(),
            actor_ids: vec![],
            institution_ids: (0..24).map(|index| format!("faction-{index:02}")).collect(),
            location_ids: vec!["forum".into()],
            public_channels: vec!["public bulletin".into()],
        }],
        news: vec![],
        canon_candidates: BTreeMap::new(),
        gestalts: BTreeMap::new(),
        gestalt_members: BTreeMap::new(),
        pending_world_proposals: vec![],
        agency_profiles: BTreeMap::new(),
        agency_relations: BTreeMap::new(),
        gestalt_lineages: BTreeMap::new(),
        resolution_policy: ResolutionPolicy {
            active_cell_budget: 4,
            provider_parallelism: 4,
            ..ResolutionPolicy::default()
        },
        resolution_pins: BTreeMap::new(),
        resolution_cover: None,
        strategic_tick_count: 0,
    };
    ghostlight_dungeon::resolution::ensure_agency_profiles(&mut campaign);
    for (index, id) in (0..24).map(|index| (index, format!("faction-{index:02}"))) {
        let profile = campaign
            .agency_profiles
            .get_mut(&id)
            .expect("institution profile");
        profile.location_ids.insert("forum".into());
        profile
            .information_channels
            .insert("public bulletin".into());
        profile.facets.insert(
            AgencyAxis::Geography,
            BTreeSet::from([format!("region-{}", index % 4)]),
        );
        profile.facets.insert(
            AgencyAxis::Ideology,
            BTreeSet::from([format!("platform-{}", index % 6)]),
        );
        profile.facets.insert(
            AgencyAxis::Information,
            BTreeSet::from([format!("private-channel-{index:02}")]),
        );
        profile.facets.insert(
            AgencyAxis::SpeciesBody,
            BTreeSet::from([format!("body-{}", index % 2)]),
        );
        if index % 2 == 1 {
            let previous = format!("faction-{:02}", index - 1);
            let relation_id = format!("rivalry-{previous}-{id}");
            campaign.agency_relations.insert(
                relation_id.clone(),
                AgencyRelation {
                    schema: "ghostlight.agency_relation.v1".into(),
                    id: relation_id,
                    from_subject_id: previous,
                    to_subject_id: id,
                    kind: AgencyRelationKind::Rivalry,
                    strength: 90,
                    active: true,
                    evidence_receipt_ids: vec![],
                },
            );
        }
    }
    campaign
}
