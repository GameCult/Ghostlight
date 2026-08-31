use anyhow::{Context, Result};
use ghostlight_dungeon::{
    domain::{
        Campaign, CellAppraisal, GestaltMaterializationReceipt, RejectedProposalReceipt,
        ResolutionCover, RollReceipt, StrategicActivityOutcome, StrategicTickReceipt,
        WorldCommitReceipt,
    },
    model::ModelStageReceipt,
    persistence::CampaignStore,
};
use serde_json::json;
use std::collections::{BTreeMap, BTreeSet};

fn main() -> Result<()> {
    let store_path = std::env::args()
        .nth(1)
        .context("usage: ghostlight-campaign-inspect <campaign.cc>")?;
    let store = CampaignStore::open(store_path)?;
    let campaign_key = store
        .keys("campaign.v1")?
        .into_iter()
        .next()
        .context("store contains no campaign.v1 state")?;
    let (_, campaign) = store
        .load::<Campaign>("campaign.v1", &campaign_key)?
        .context("campaign.v1 row disappeared during inspection")?;
    let mut ticks = store.load_all::<StrategicTickReceipt>("strategic_tick.v1")?;
    ticks.sort_by_key(|tick| tick.revision);
    let latest_tick = ticks.last();
    let resolution_epoch = latest_tick.and_then(|tick| tick.resolution_epoch);
    let snapshot_revision = latest_tick.map(|tick| tick.previous_revision);

    let cover = match (snapshot_revision, resolution_epoch) {
        (Some(revision), Some(epoch)) => store
            .load_all::<ResolutionCover>("resolution_cover.v1")?
            .into_iter()
            .find(|cover| cover.world_revision == revision && cover.resolution_epoch == epoch),
        _ => None,
    };
    let appraisals = match (snapshot_revision, resolution_epoch) {
        (Some(revision), Some(epoch)) => store
            .load_all::<CellAppraisal>("cell_appraisal.v1")?
            .into_iter()
            .filter(|appraisal| {
                appraisal.world_revision == revision && appraisal.resolution_epoch == epoch
            })
            .collect::<Vec<_>>(),
        _ => Vec::new(),
    };
    let outcomes = match (latest_tick, resolution_epoch) {
        (Some(tick), Some(epoch)) => {
            let prefix = format!("{}:{epoch}:", tick.revision);
            let mut outcomes = Vec::new();
            for key in store.keys("strategic_activity_outcome.v1")? {
                if key.starts_with(&prefix)
                    && let Some((_, outcome)) = store
                        .load::<StrategicActivityOutcome>("strategic_activity_outcome.v1", &key)?
                {
                    outcomes.push(outcome);
                }
            }
            outcomes.sort_by(|left, right| left.action_digest.cmp(&right.action_digest));
            outcomes
        }
        _ => Vec::new(),
    };
    let mut rejected = store.load_all::<RejectedProposalReceipt>("rejected_proposal_receipt.v1")?;
    rejected.sort_by_key(|receipt| (receipt.revision, receipt.rejected_at));
    let rejected = rejected
        .into_iter()
        .rev()
        .take(16)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<Vec<_>>();
    let mut presence =
        store.load_all::<GestaltMaterializationReceipt>("gestalt_materialization_receipt.v1")?;
    presence.sort_by_key(|receipt| receipt.revision);
    let presence = presence.into_iter().rev().take(16).collect::<Vec<_>>();

    let receipt_hashes = latest_tick
        .map(|tick| tick.model_receipt_hashes.iter().cloned().collect())
        .unwrap_or_else(BTreeSet::new);
    let mut model_receipts = Vec::new();
    for hash in &receipt_hashes {
        if let Some((_, receipt)) =
            store.load::<ModelStageReceipt>("persona_stage_receipt.v1", hash)?
        {
            model_receipts.push(json!({
                "receiptHash": receipt.storage_key(),
                "stage": receipt.stage,
                "provider": receipt.provider,
                "model": receipt.model,
                "snapshotBinding": receipt.snapshot_binding,
                "latencyMs": receipt.latency_ms,
                "validationResult": receipt.validation_result,
                "inputChars": receipt.input_chars,
                "outputChars": receipt.output_chars,
                "attempts": receipt.provider_attempts.iter().map(|attempt| json!({
                    "finishReason": attempt.finish_reason,
                    "latencyMs": attempt.latency_ms,
                    "tokenUsage": attempt.token_usage,
                    "localValidationResult": attempt.local_validation_result,
                })).collect::<Vec<_>>(),
            }));
        }
    }
    let recent_revision_bindings = (campaign.revision.saturating_sub(3)..=campaign.revision)
        .map(|revision| format!("campaign:{}:revision:{revision}", campaign.id))
        .collect::<Vec<_>>();
    let mut recent_model_receipts = store
        .load_all::<ModelStageReceipt>("persona_stage_receipt.v1")?
        .into_iter()
        .filter(|receipt| {
            recent_revision_bindings
                .iter()
                .any(|binding| receipt.snapshot_binding.contains(binding))
        })
        .collect::<Vec<_>>();
    recent_model_receipts.sort_by(|left, right| left.storage_key().cmp(right.storage_key()));
    let mut newspaper_model_receipts = store
        .load_all::<ModelStageReceipt>("persona_stage_receipt.v1")?
        .into_iter()
        .filter(|receipt| receipt.stage.starts_with("newspaper_"))
        .collect::<Vec<_>>();
    newspaper_model_receipts.sort_by(|left, right| left.storage_key().cmp(right.storage_key()));
    let assessment_mutation_scopes =
        store.load_all::<serde_json::Value>("assessment_mutation_scope_cache.v1")?;
    let mut roll_receipts = store.load_all::<RollReceipt>("roll_receipt.v1")?;
    roll_receipts.reverse();
    roll_receipts.truncate(16);
    let mut commit_receipts = store.load_all::<WorldCommitReceipt>("world_commit_receipt.v1")?;
    commit_receipts.sort_by_key(|receipt| receipt.revision);
    let commit_receipts = commit_receipts
        .into_iter()
        .rev()
        .take(16)
        .collect::<Vec<_>>();

    let mut subjects = BTreeMap::new();
    for actor in campaign.actors.values() {
        let profile = campaign.agency_profiles.get(&actor.id);
        subjects.insert(
            actor.id.clone(),
            json!({
                "kind": "actor",
                "name": actor.name,
                "locationId": actor.location_id,
                "simulationEligible": profile.is_none_or(|profile| profile.simulation_eligible),
                "goals": actor.goals,
                "obligations": actor.obligations,
                "relationships": actor.relationships,
                "memories": actor.memories,
                "knowledge": actor.knowledge,
            }),
        );
    }
    for institution in campaign.institutions.values() {
        subjects.insert(
            institution.id.clone(),
            json!({ "kind": "institution", "name": institution.name }),
        );
    }
    for gestalt in campaign.gestalts.values() {
        subjects.insert(
            gestalt.id.clone(),
            json!({
                "kind": "gestalt",
                "name": gestalt.name,
                "goals": gestalt.goals,
                "pressures": gestalt.pressures,
                "knowledge": gestalt.shared_knowledge,
            }),
        );
    }
    for member in campaign.gestalt_members.values() {
        subjects.insert(
            member.id.clone(),
            json!({
                "kind": "gestalt_member",
                "name": member.name,
                "gestaltId": member.gestalt_id,
                "goals": member.goals,
                "obligations": member.obligations,
                "relationships": member.relationships,
                "memories": member.memories,
                "knowledge": ghostlight_dungeon::resolution::effective_member_knowledge(
                    &campaign,
                    &member.id,
                )?,
            }),
        );
    }

    let event_ids = latest_tick
        .map(|tick| tick.event_ids.iter().cloned().collect::<BTreeSet<_>>())
        .unwrap_or_default();
    let events = campaign
        .events
        .iter()
        .filter(|event| event_ids.contains(&event.id))
        .collect::<Vec<_>>();
    let news = campaign
        .news
        .iter()
        .filter(|issue| issue.event_ids.iter().any(|id| event_ids.contains(id)))
        .collect::<Vec<_>>();

    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "schema": "ghostlight.campaign_inspection.v1",
            "campaign": {
                "id": campaign.id,
                "name": campaign.name,
                "revision": campaign.revision,
                "worldTime": campaign.world_time,
                "strategicTickCount": campaign.strategic_tick_count,
                "resolutionEpoch": campaign.resolution_policy.resolution_epoch,
                "configuredBudget": campaign.resolution_policy.active_cell_budget,
                "subjectCounts": {
                    "actors": campaign.actors.len(),
                    "institutions": campaign.institutions.len(),
                    "gestalts": campaign.gestalts.len(),
                    "gestaltMembers": campaign.gestalt_members.len(),
                },
            },
            "subjectDirectory": subjects,
            "locations": campaign.locations,
            "civicSystems": campaign.civic_systems,
            "agencyRelations": campaign.agency_relations,
            "latestStrategicTick": latest_tick,
            "cover": cover,
            "appraisals": appraisals,
            "activityOutcomes": outcomes,
            "rejectedProposals": rejected,
            "recentPresenceReceipts": presence,
            "recentTranscript": campaign.transcript.iter().rev().take(24).collect::<Vec<_>>(),
            "recentWorldEvents": campaign.events.iter().rev().take(24).collect::<Vec<_>>(),
            "pendingWorldProposals": campaign.pending_world_proposals,
            "recentModelReceipts": recent_model_receipts,
            "newspaperModelReceipts": newspaper_model_receipts,
            "assessmentMutationScopes": assessment_mutation_scopes,
            "recentRollReceipts": roll_receipts,
            "recentCommitReceipts": commit_receipts,
            "events": events,
            "news": news,
            "modelReceipts": model_receipts,
            "missingModelReceiptHashes": receipt_hashes.into_iter().filter(|hash| {
                !model_receipts.iter().any(|receipt| receipt["receiptHash"] == *hash)
            }).collect::<Vec<_>>(),
        }))?
    );
    Ok(())
}
