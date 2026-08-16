use crate::{
    domain::{Campaign, StrategicTickPlan},
    model::{ModelPort, ModelStageOutput, ModelStageRequest, run_validated_stage},
};
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use schemars::schema_for;

pub async fn propose_strategic_tick(
    model: &dyn ModelPort,
    campaign: &Campaign,
) -> Result<(StrategicTickPlan, ModelStageOutput)> {
    let player_id = &campaign.player_actor_id;
    let actors = campaign
        .actors
        .iter()
        .filter(|(id, _)| *id != player_id)
        .map(|(id, actor)| {
            serde_json::json!({
                "id":id,
                "location_id":actor.location_id,
                "goals":actor.goals,
                "conditions":actor.conditions,
                "obligations":actor.obligations
            })
        })
        .collect::<Vec<_>>();
    let context = serde_json::json!({
        "campaign_id":campaign.id,
        "revision":campaign.revision,
        "world_time":campaign.world_time,
        "tick_hours":campaign.tick_hours,
        "locations":campaign.locations,
        "remote_actors":actors,
        "institutions":campaign.institutions,
        "gestalts":campaign.gestalts,
        "clocks":campaign.clocks,
        "recent_events":campaign.events.iter().rev().take(12).collect::<Vec<_>>()
    });
    let output = run_validated_stage(
        model,
        &ModelStageRequest {
            stage: "strategic_tick".into(),
            model: "deepseek-v4-flash".into(),
            snapshot_binding: format!("campaign:{}:revision:{}", campaign.id, campaign.revision),
            lived_stream: format!(
                "Propose one bounded offscreen strategic tick. Institutions, remote actors, and gestalt populations act in their own interests. Do not control, move, injure, or directly target the absent player. Use only supplied IDs. Actor movement must use a direct listed route whose travel time fits inside tick_hours. Keep summaries concrete. Pressure additions are branch-local population pressure, not new canon knowledge. Empty arrays are valid when nobody has a credible action. Return only the schema JSON.\nSTATE:\n{}",
                serde_json::to_string(&context)?
            ),
            output_schema: Some(serde_json::to_value(schema_for!(StrategicTickPlan))?),
            source_receipt_ids: campaign.branch_origin.evidence_receipt_ids.clone(),
        },
    )
    .await?;
    let plan = serde_json::from_value(
        output
            .structured
            .clone()
            .context("strategic tick stage returned no structured plan")?,
    )?;
    Ok((plan, output))
}

pub fn due_tick_target(now: DateTime<Utc>, last_player_activity: DateTime<Utc>) -> u8 {
    let idle = now - last_player_activity;
    if idle < chrono::Duration::minutes(15) {
        0
    } else {
        (idle.num_hours().max(0) as u8).min(8)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn away_budget_waits_and_caps_at_eight() {
        let now = Utc::now();
        assert_eq!(due_tick_target(now, now - chrono::Duration::minutes(14)), 0);
        assert_eq!(due_tick_target(now, now - chrono::Duration::minutes(59)), 0);
        assert_eq!(due_tick_target(now, now - chrono::Duration::hours(1)), 1);
        assert_eq!(due_tick_target(now, now - chrono::Duration::hours(30)), 8);
    }
}
