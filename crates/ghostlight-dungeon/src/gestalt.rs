use crate::{
    domain::{Campaign, GestaltPresencePlan},
    model::{ModelPort, ModelStageReceipt, ModelStageRequest, run_validated_stage},
};
use anyhow::{Result, anyhow};
use schemars::schema_for;
use std::{collections::BTreeSet, sync::Arc};

#[derive(Clone)]
pub struct GestaltPresencePlanner {
    pub model: Arc<dyn ModelPort>,
    pub model_name: String,
}

impl GestaltPresencePlanner {
    pub async fn plan(
        &self,
        campaign: &Campaign,
        event_summary: &str,
    ) -> Result<(GestaltPresencePlan, ModelStageReceipt)> {
        let player_location = &campaign.actors[&campaign.player_actor_id].location_id;
        let candidates = serde_json::json!({
            "player_location_id": player_location,
            "gestalts": campaign.gestalts,
            "members": campaign.gestalt_members,
            "materialized_member_actor_ids": campaign.gestalt_members.values()
                .filter_map(|member| member.materialized_actor_id.clone()).collect::<Vec<_>>(),
        });
        let schema = serde_json::to_value(schema_for!(GestaltPresencePlan))?;
        let output = run_validated_stage(
            self.model.as_ref(),
            &ModelStageRequest {
                stage: "gestalt_presence_planner".into(),
                model: self.model_name.clone(),
                snapshot_binding: format!("campaign:{}:revision:{}", campaign.id, campaign.revision),
                lived_stream: format!(
                    "Choose reversible Persona population presence after this event. Promote a member only when individualized interaction, perception, custody, promise, injury, possession, or relationship now matters. Demote a materialized member when they are no longer scene-relevant. Use only the supplied IDs and exact versions. Never place a promoted member outside the player location. Aggregate deltas must remain empty; population learning requires separate review. Emit the exact JSON schema.\nSCHEMA:\n{}\nCANDIDATES:\n{}\nEVENT:\n{}",
                    serde_json::to_string_pretty(&schema)?, candidates, event_summary
                ),
                output_schema: Some(schema),
                source_receipt_ids: campaign.branch_origin.evidence_receipt_ids.clone(),
            },
        )
        .await?;
        let plan: GestaltPresencePlan = serde_json::from_value(
            output
                .structured
                .ok_or_else(|| anyhow!("presence planner produced no plan"))?,
        )?;
        validate_plan(campaign, &plan, player_location)?;
        Ok((plan, output.receipt))
    }
}

fn validate_plan(
    campaign: &Campaign,
    plan: &GestaltPresencePlan,
    player_location: &str,
) -> Result<()> {
    let mut members = BTreeSet::new();
    for promotion in &plan.promotions {
        if !members.insert(promotion.member_id.clone()) {
            return Err(anyhow!("presence plan promotes one member twice"));
        }
        let member = campaign
            .gestalt_members
            .get(&promotion.member_id)
            .ok_or_else(|| anyhow!("presence plan invented a member"))?;
        let gestalt = campaign
            .gestalts
            .get(&promotion.gestalt_id)
            .ok_or_else(|| anyhow!("presence plan invented a gestalt"))?;
        if member.gestalt_id != promotion.gestalt_id
            || member.version != promotion.expected_member_version
            || gestalt.version != promotion.expected_gestalt_version
            || member.materialized_actor_id.is_some()
            || promotion.location_id != player_location
        {
            return Err(anyhow!("presence promotion does not match its snapshot"));
        }
    }
    let materialized: BTreeSet<_> = campaign
        .gestalt_members
        .values()
        .filter_map(|member| member.materialized_actor_id.clone())
        .collect();
    let mut actors = BTreeSet::new();
    for demotion in &plan.demotions {
        if !actors.insert(demotion.actor_id.clone()) || !materialized.contains(&demotion.actor_id) {
            return Err(anyhow!(
                "presence plan demotes an unknown or duplicate member"
            ));
        }
        if demotion.aggregate_delta != Default::default() {
            return Err(anyhow!(
                "automatic presence planning cannot rewrite gestalt knowledge"
            ));
        }
        let member = campaign
            .gestalt_members
            .values()
            .find(|member| {
                member.materialized_actor_id.as_deref() == Some(demotion.actor_id.as_str())
            })
            .expect("materialized member was validated");
        let actor = &campaign.actors[&demotion.actor_id];
        if actor.location_id == player_location
            || member.relevance_lease_until_revision > campaign.revision
        {
            return Err(anyhow!(
                "presence plan demotes a visible or recently relevant member"
            ));
        }
    }
    Ok(())
}
