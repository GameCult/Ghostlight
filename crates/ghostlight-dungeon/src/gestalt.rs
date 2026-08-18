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
        let nearby_gestalts = campaign
            .gestalts
            .values()
            .filter(|gestalt| {
                crate::resolution::validate_active_gestalt_presence_location(
                    campaign,
                    &gestalt.id,
                    player_location,
                )
                .is_ok()
            })
            .collect::<Vec<_>>();
        let nearby_gestalt_ids = nearby_gestalts
            .iter()
            .map(|gestalt| gestalt.id.as_str())
            .collect::<BTreeSet<_>>();
        let player = &campaign.actors[&campaign.player_actor_id];
        let nearby_dormant_members = campaign
            .gestalt_members
            .values()
            .filter(|member| {
                nearby_gestalt_ids.contains(member.gestalt_id.as_str())
                    && crate::resolution::dormant_member_location(campaign, &member.id)
                        .is_ok_and(|location| location == *player_location)
            })
            .map(|member| {
                serde_json::json!({
                    "member": member,
                    "player_relationship_to_member": player
                        .relationships
                        .get(&format!("member:{}", member.id)),
                })
            })
            .collect::<Vec<_>>();
        let materialized_members = campaign
            .gestalt_members
            .values()
            .filter_map(|member| {
                let actor_id = member.materialized_actor_id.as_ref()?;
                let actor = campaign.actors.get(actor_id)?;
                Some(serde_json::json!({
                    "member_id":member.id,
                    "member_version":member.version,
                    "actor_id":actor_id,
                    "actor_location_id":actor.location_id,
                    "relevance_lease_until_revision":member.relevance_lease_until_revision,
                }))
            })
            .collect::<Vec<_>>();
        let candidates = serde_json::json!({
            "player_location_id": player_location,
            "nearby_active_leaf_gestalts": nearby_gestalts,
            "nearby_dormant_members": nearby_dormant_members,
            "materialized_members": materialized_members,
        });
        let schema = serde_json::to_value(schema_for!(GestaltPresencePlan))?;
        let output = run_validated_stage(
            self.model.as_ref(),
            &ModelStageRequest {
                stage: "gestalt_presence_planner".into(),
                model: self.model_name.clone(),
                snapshot_binding: format!("campaign:{}:revision:{}", campaign.id, campaign.revision),
                lived_stream: format!(
                    "Cast reversible Persona population presence for the next scene after this event. The purpose is to make causal continuity visible without crowding the scene or inventing coincidence. Promote an existing member when their exact durable history makes them individually relevant. When a nearby dormant member has a reciprocal player relationship and an unresolved callback signal such as an obligation, memory, or goal, promote the single strongest earned callback unless the event makes their presence implausible or dramatically harmful. An ordinary shared-location event is enough opportunity for an earned callback; do not require the player to ask for that person. Prefer an existing person over anonymous individuation whenever their exact delta supports the scene. Return no promotion when there is no earned callback or the current event conflicts with it. If the event makes an anonymous population member individually relevant and no supplied member fits, individuate exactly one durable member delta from the gestalt baseline; use a new stable lowercase id, version 0, the exact gestalt id/version, no materialized actor id, and record only personal departures from the shared baseline. Demote a materialized member when they are no longer scene-relevant. Never place a promoted or individuated member outside the player location. Aggregate deltas must remain empty; population learning requires separate review. Emit the exact JSON schema.\nSCHEMA:\n{}\nCANDIDATES:\n{}\nEVENT:\n{}",
                    serde_json::to_string_pretty(&schema)?, candidates, event_summary
                ),
                output_schema: Some(schema),
                source_receipt_ids: campaign.branch_origin.evidence_receipt_ids.clone(),
                temperature: Some(0.0),
                max_output_tokens: Some(1_500),
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
    for individuation in &plan.individuations {
        let member = &individuation.member;
        let gestalt = campaign
            .gestalts
            .get(&individuation.gestalt_id)
            .ok_or_else(|| anyhow!("presence plan invented a gestalt"))?;
        crate::resolution::validate_active_gestalt_presence_location(
            campaign,
            &individuation.gestalt_id,
            player_location,
        )?;
        if individuation.expected_gestalt_version != gestalt.version
            || individuation.location_id != player_location
            || member.gestalt_id != individuation.gestalt_id
            || member.version != 0
            || member.materialized_actor_id.is_some()
            || member.id.trim().is_empty()
            || member.name.trim().is_empty()
            || campaign.gestalt_members.contains_key(&member.id)
            || !members.insert(member.id.clone())
        {
            return Err(anyhow!(
                "presence individuation does not match its snapshot"
            ));
        }
    }
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
        let member_location = crate::resolution::dormant_member_location(campaign, &member.id)?;
        if member.gestalt_id != promotion.gestalt_id
            || member.version != promotion.expected_member_version
            || gestalt.version != promotion.expected_gestalt_version
            || member.materialized_actor_id.is_some()
            || promotion.location_id != player_location
            || member_location != promotion.location_id
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        domain::{GestaltMemberDelta, GestaltPersonaState, Location},
        model::ModelStageRequest,
    };
    use async_trait::async_trait;
    use std::{
        collections::{BTreeMap, BTreeSet},
        sync::Mutex,
    };

    struct CapturePresenceModel {
        prompt: Arc<Mutex<String>>,
    }

    #[async_trait]
    impl ModelPort for CapturePresenceModel {
        async fn run(&self, request: &ModelStageRequest) -> Result<String> {
            *self.prompt.lock().unwrap() = request.lived_stream.clone();
            Ok(serde_json::json!({
                "individuations":[],
                "promotions":[],
                "demotions":[]
            })
            .to_string())
        }

        fn provider(&self) -> &'static str {
            "presence-capture"
        }
    }

    #[tokio::test]
    async fn presence_projection_contains_only_nearby_active_leaves_and_relevant_members() {
        let mut campaign = crate::resolution::tests::campaign(0, 8);
        campaign.locations.insert(
            "far".into(),
            Location {
                id: "far".into(),
                name: "Far".into(),
                container_id: None,
                routes: BTreeMap::new(),
                persistent_features: vec![],
            },
        );
        let gestalt = |id: &str, name: &str, location: &str| GestaltPersonaState {
            schema: "ghostlight.gestalt_persona_state.v1".into(),
            id: id.into(),
            name: name.into(),
            version: 0,
            home_location_id: location.into(),
            shared_capabilities: BTreeSet::new(),
            shared_knowledge: BTreeSet::new(),
            resources: BTreeSet::new(),
            goals: vec![],
            pressures: vec![],
        };
        campaign.gestalts.insert(
            "nearby-leaf".into(),
            gestalt("nearby-leaf", "Nearby leaf", "center"),
        );
        campaign
            .gestalts
            .insert("far-leaf".into(), gestalt("far-leaf", "Far leaf", "far"));
        campaign.gestalts.insert(
            "inactive-parent".into(),
            gestalt("inactive-parent", "Inactive parent", "center"),
        );
        let member = |id: &str, name: &str, gestalt_id: &str, location: &str| GestaltMemberDelta {
            schema: "ghostlight.gestalt_member_delta.v1".into(),
            id: id.into(),
            gestalt_id: gestalt_id.into(),
            version: 0,
            name: name.into(),
            capability_additions: BTreeSet::new(),
            capability_removals: BTreeSet::new(),
            knowledge_additions: BTreeSet::new(),
            knowledge_removals: BTreeSet::new(),
            equipment: BTreeSet::new(),
            conditions: BTreeSet::new(),
            obligations: BTreeSet::new(),
            relationships: BTreeMap::new(),
            goals: vec![],
            memories: vec![],
            last_location_id: Some(location.into()),
            materialized_actor_id: None,
            last_relevant_revision: 0,
            relevance_lease_until_revision: 0,
        };
        campaign.gestalt_members.insert(
            "mira".into(),
            member("mira", "Mira Nearby", "nearby-leaf", "center"),
        );
        let player_id = campaign.player_actor_id.clone();
        campaign
            .actors
            .get_mut(&player_id)
            .unwrap()
            .relationships
            .insert("member:mira".into(), "helped her cross the flood".into());
        campaign
            .gestalt_members
            .get_mut("mira")
            .unwrap()
            .relationships
            .insert(player_id, "trusts them for helping".into());
        campaign
            .gestalt_members
            .get_mut("mira")
            .unwrap()
            .obligations
            .insert("thank the player if they meet again".into());
        campaign.gestalt_members.insert(
            "far-person".into(),
            member("far-person", "Far Person", "far-leaf", "far"),
        );
        crate::resolution::ensure_agency_profiles(&mut campaign);
        let parent = campaign.agency_profiles.get_mut("inactive-parent").unwrap();
        parent.active_leaf = false;
        parent.simulation_eligible = false;

        let prompt = Arc::new(Mutex::new(String::new()));
        GestaltPresencePlanner {
            model: Arc::new(CapturePresenceModel {
                prompt: prompt.clone(),
            }),
            model_name: "fixture".into(),
        }
        .plan(&campaign, "The player works in the square.")
        .await
        .unwrap();
        let prompt = prompt.lock().unwrap();
        assert!(prompt.contains("Nearby leaf"));
        assert!(prompt.contains("Mira Nearby"));
        assert!(prompt.contains("helped her cross the flood"));
        assert!(prompt.contains("thank the player if they meet again"));
        assert!(prompt.contains("do not require the player to ask for that person"));
        assert!(!prompt.contains("Far leaf"));
        assert!(!prompt.contains("Far Person"));
        assert!(!prompt.contains("Inactive parent"));
    }
}
