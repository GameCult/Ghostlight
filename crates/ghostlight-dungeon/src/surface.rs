use crate::{
    domain::{
        Campaign, GestaltMaterializationReceipt, NarrationProjection, RejectedProposalReceipt,
        StrategicTickReceipt, VaultEvidenceReceipt, WorldCommitReceipt,
    },
    model::ModelStageReceipt,
};
use serde_json::{Value, json};

pub fn player_surface(campaign: &Campaign, narrations: &[NarrationProjection]) -> Value {
    let interface_version = campaign
        .revision
        .saturating_mul(1_000_000_000_000)
        .saturating_add(
            campaign
                .resolution_policy
                .resolution_epoch
                .saturating_mul(1_000_000),
        )
        .saturating_add(campaign.resolution_policy.provider_configuration_epoch);
    let mut story = narrations.iter().map(|n| json!({"id":format!("narration-{}",n.source_revision),"kind":"text","props":{"value":n.text},"children":[]})).collect::<Vec<_>>();
    story.extend(campaign.transcript.iter().map(|t| json!({"id":format!("turn-{}-{}",t.revision,t.speaker),"kind":"text","props":{"value":format!("{}: {}",t.speaker,t.text)},"children":[]})));
    let player = &campaign.actors[&campaign.player_actor_id];
    let location = &campaign.locations[&player.location_id];
    let ledger = format!(
        "Capabilities: {}\nEquipment: {}\nConditions: {}\nObligations: {}\nKnown facts: {}",
        join(&player.capabilities),
        join(&player.equipment),
        join(&player.conditions),
        join(&player.obligations),
        join(&player.knowledge)
    );
    let pressures = campaign
        .clocks
        .values()
        .map(|clock| {
            format!(
                "{} {}/{} — {}",
                clock.label, clock.progress, clock.threshold, clock.consequence
            )
        })
        .chain(
            campaign
                .institutions
                .values()
                .map(|x| format!("{} — {}", x.name, x.posture)),
        )
        .collect::<Vec<_>>()
        .join("\n");
    let news=campaign.news.iter().map(|item|json!({"id":item.id,"kind":"text","props":{"value":format!("[{}] {}",item.channel,item.headline)},"children":[]})).collect::<Vec<_>>();
    let effective_budget = campaign
        .resolution_cover
        .as_ref()
        .map(|cover| cover.effective_budget)
        .unwrap_or(campaign.resolution_policy.active_cell_budget);
    json!({
      "type":"surface-state", "schema":"gamecult.eve.surface.v1", "providerId":"gamecult.ghostlight.dungeon",
      "providerKind":"narrative.simulation", "title":campaign.name, "version":interface_version,
      "world_revision":campaign.revision,
      "resolution":{
        "policy":campaign.resolution_policy,
        "effective_budget":effective_budget,
        "mandatory_overage":campaign.resolution_cover.as_ref().map(|cover| cover.mandatory_overage).unwrap_or(0),
        "pins":campaign.resolution_pins.values().collect::<Vec<_>>(),
        "fission_targets":campaign.agency_profiles.values().filter(|profile| profile.active_leaf && profile.subject_kind == crate::domain::AgencySubjectKind::Gestalt).filter_map(|profile| campaign.gestalts.get(&profile.subject_id).map(|gestalt| json!({"id":gestalt.id,"name":gestalt.name}))).collect::<Vec<_>>()
      },
      "surface":{"id":format!("ghostlight.campaign.{}",campaign.id),"root":{"id":"dungeon.root","kind":"surface","props":{},"children":[
        {"id":"dungeon.status","kind":"card","props":{"title":format!("{} · revision {} · {}",campaign.name,campaign.revision,campaign.world_time)},"children":[]},
        {"id":"dungeon.location","kind":"card","props":{"title":format!("{} · {}",location.name,player.name)},"children":[{"id":"dungeon.pressures","kind":"text","props":{"value":pressures},"children":[]}]},
        {"id":"dungeon.ledger","kind":"card","props":{"title":"Character ledger"},"children":[{"id":"dungeon.ledger.text","kind":"text","props":{"value":ledger},"children":[]}]},
        {"id":"dungeon.resolution","kind":"card","props":{"title":format!("World resolution · {} configured / {} effective",campaign.resolution_policy.active_cell_budget,effective_budget)},"children":[{"id":"dungeon.resolution.text","kind":"text","props":{"value":format!("Resolution epoch {} · {} pins · {} temporary overage",campaign.resolution_policy.resolution_epoch,campaign.resolution_pins.len(),campaign.resolution_cover.as_ref().map(|cover| cover.mandatory_overage).unwrap_or(0))},"children":[]}]},
        {"id":"dungeon.news","kind":"card","props":{"title":"Accessible news and rumors"},"children":news},
        {"id":"dungeon.transcript","kind":"card","props":{"title":"Story"},"children":story},
        {"id":"dungeon.composer","kind":"text-input","props":{"label":"What do you attempt?","commandId":"attempt.assess"},"children":[]}
      ]},"styles":{"tokens":{"colorBackground":"#0c1110","colorPanel":"#17201d","colorText":"#e8e1cf","colorMuted":"#9aa69f","colorAccent":"#d49b58"}}},
      "commands":[{"id":"attempt.assess","schema":"gamecult.eve.command.v1","receiptSchema":"ghostlight.player_action_assessment.v1"}]
    })
}

pub fn operator_surface(
    campaign: &Campaign,
    evidence: &[VaultEvidenceReceipt],
    commits: &[WorldCommitReceipt],
    stages: &[ModelStageReceipt],
    strategic_ticks: &[StrategicTickReceipt],
    gestalt_receipts: &[GestaltMaterializationReceipt],
    rejected: &[RejectedProposalReceipt],
    resolution_plans: &[crate::domain::ResolutionPlanReceipt],
    cell_appraisals: &[crate::domain::CellAppraisal],
    resolution_controls: &[crate::domain::ResolutionControlReceipt],
    live_turn_pressure: usize,
) -> Value {
    let interface_version = campaign
        .revision
        .saturating_mul(1_000_000_000_000)
        .saturating_add(
            campaign
                .resolution_policy
                .resolution_epoch
                .saturating_mul(1_000_000),
        )
        .saturating_add(campaign.resolution_policy.provider_configuration_epoch);
    let cover_text = campaign
        .resolution_cover
        .as_ref()
        .map(|cover| {
            cover
                .cells
                .iter()
                .map(|cell| {
                    format!(
                        "{} {:?} [{}] loss {:.3} debt focus {} — {}",
                        cell.id,
                        cell.mode,
                        cell.subject_ids
                            .iter()
                            .cloned()
                            .collect::<Vec<_>>()
                            .join(", "),
                        cell.merge_loss.total,
                        cell.detail_focus_subject_id.as_deref().unwrap_or("none"),
                        cell.rationale
                    )
                })
                .collect::<Vec<_>>()
                .join("\n")
        })
        .unwrap_or_else(|| "No cover has committed yet.".into());
    let graph_text = campaign
        .agency_relations
        .values()
        .filter(|relation| relation.active)
        .map(|relation| {
            format!(
                "{} --{:?}/{}--> {}",
                relation.from_subject_id, relation.kind, relation.strength, relation.to_subject_id
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    let typed = json!({
        "campaign": campaign,
        "evidence": evidence,
        "commit_receipts": commits,
        "model_stage_receipts": stages,
        "strategic_ticks": strategic_ticks,
        "gestalt_materialization_receipts": gestalt_receipts,
        "rejected_proposals": rejected,
        "resolution_plan_receipts": resolution_plans,
        "cell_appraisals": cell_appraisals,
        "resolution_control_receipts": resolution_controls,
        "scheduler": {"live_turn_pressure": live_turn_pressure}
    });
    json!({
      "type":"surface-state", "schema":"gamecult.eve.surface.v1", "providerId":"gamecult.ghostlight.dungeon",
      "providerKind":"narrative.simulation.operator", "title":format!("{} operator",campaign.name), "version":interface_version,
      "surface":{"id":format!("ghostlight.operator.{}",campaign.id),"root":{"id":"dungeon.operator.root","kind":"surface","props":{},"children":[
        {"id":"dungeon.operator.status","kind":"card","props":{"title":format!("Revision {} · {} model stages · {} rejected proposals",campaign.revision,stages.len(),rejected.len())},"children":[]},
        {"id":"dungeon.operator.cover","kind":"card","props":{"title":format!("Agency cover · epoch {} · budget {}",campaign.resolution_policy.resolution_epoch,campaign.resolution_policy.active_cell_budget)},"children":[{"id":"dungeon.operator.cover.text","kind":"text","props":{"value":cover_text},"children":[]}]},
        {"id":"dungeon.operator.graph","kind":"card","props":{"title":format!("Agency graph · {} profiles · {} relations",campaign.agency_profiles.len(),campaign.agency_relations.len())},"children":[{"id":"dungeon.operator.graph.text","kind":"text","props":{"value":graph_text},"children":[]}]},
        {"id":"dungeon.operator.typed","kind":"code","props":{"language":"json","value":serde_json::to_string_pretty(&typed).unwrap_or_else(|_| "operator projection failed".into())},"children":[]}
      ]},"styles":{"tokens":{"colorBackground":"#0c1110","colorPanel":"#17201d","colorText":"#e8e1cf","colorMuted":"#9aa69f","colorAccent":"#d49b58"}}},
      "commands":[]
    })
}

fn join(values: &std::collections::BTreeSet<String>) -> String {
    if values.is_empty() {
        "none".into()
    } else {
        values.iter().cloned().collect::<Vec<_>>().join(", ")
    }
}
