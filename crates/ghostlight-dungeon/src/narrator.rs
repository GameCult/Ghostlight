use crate::{
    domain::{Campaign, NarrationProjection},
    model::{ModelPort, ModelStageReceipt, ModelStageRequest, run_validated_stage},
    persistence::CampaignStore,
};
use anyhow::{Result, anyhow};
use chrono::Utc;
use std::sync::Arc;

#[derive(Clone)]
pub struct Narrator {
    pub model: Arc<dyn ModelPort>,
    pub model_name: String,
}

impl Narrator {
    pub async fn project(
        &self,
        store: &CampaignStore,
        campaign: &Campaign,
    ) -> Result<(NarrationProjection, ModelStageReceipt)> {
        let player = &campaign.actors[&campaign.player_actor_id];
        let location = &campaign.locations[&player.location_id];
        let recent_events: Vec<_> = campaign.events.iter().rev().take(8).cloned().collect();
        let recent_turns: Vec<_> = campaign.transcript.iter().rev().take(12).cloned().collect();
        let visible_actors: Vec<_> = campaign
            .actors
            .values()
            .filter(|actor| actor.location_id == player.location_id)
            .map(|actor| {
                serde_json::json!({
                    "id": actor.id,
                    "name": actor.name,
                    "conditions": actor.conditions,
                })
            })
            .collect();
        let public_slice = serde_json::json!({
            "world_time": campaign.world_time,
            "location": location,
            "visible_actors": visible_actors,
            "recent_events": recent_events,
            "recent_explicit_speech_and_stakes": recent_turns,
        });
        let output = run_validated_stage(
            self.model.as_ref(),
            &ModelStageRequest {
                stage: "narrator".into(),
                model: self.model_name.clone(),
                snapshot_binding: format!(
                    "campaign:{}:revision:{}",
                    campaign.id, campaign.revision
                ),
                lived_stream: format!(
                    "Narrate the latest committed change in vivid, concise second-person interactive-fiction prose. Describe only the supplied committed state, speech, and accessible sensory consequences. Do not add actions, facts, private thoughts, expertise, geography, outcomes, or dialogue. Emit prose only.\n\n{}",
                    serde_json::to_string(&public_slice)?
                ),
                output_schema: None,
                source_receipt_ids: campaign.branch_origin.evidence_receipt_ids.clone(),
            },
        )
        .await?;
        let text = output.narrative.trim();
        if text.is_empty() || text.starts_with('{') || text.contains("```json") {
            return Err(anyhow!("narrator violated prose projection membrane"));
        }
        let current = store
            .load::<Campaign>("campaign.v1", &campaign.id.to_string())?
            .map(|(_, campaign)| campaign)
            .ok_or_else(|| anyhow!("campaign vanished during narration"))?;
        if current.revision != campaign.revision {
            return Err(anyhow!("narration snapshot became stale"));
        }
        let projection = NarrationProjection {
            schema: "ghostlight.narration_projection.v1".into(),
            id: format!("{}:{}", campaign.id, campaign.revision),
            campaign_id: campaign.id,
            source_revision: campaign.revision,
            text: text.into(),
            event_ids: recent_events.into_iter().map(|event| event.id).collect(),
            model_receipt_hash: output.receipt.storage_key().to_owned(),
            published_at: Utc::now(),
        };
        Ok((projection, output.receipt))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        domain::{ActorState, BranchOrigin, Campaign, Location},
        model::FixtureModel,
    };
    use std::collections::{BTreeMap, BTreeSet};

    fn campaign() -> Campaign {
        let actor = ActorState {
            id: "player".into(),
            name: "Player".into(),
            location_id: "room".into(),
            capabilities: BTreeSet::new(),
            knowledge: BTreeSet::new(),
            equipment: BTreeSet::new(),
            conditions: BTreeSet::new(),
            obligations: BTreeSet::new(),
            relationships: BTreeMap::new(),
            goals: vec![],
            memories: vec![],
        };
        Campaign {
            schema: "ghostlight.campaign.v1".into(),
            id: uuid::Uuid::new_v4(),
            name: "Narrator test".into(),
            revision: 0,
            branch_origin: BranchOrigin {
                canon_cutoff: "test".into(),
                evidence_receipt_ids: vec![],
            },
            world_time: Utc::now(),
            tick_hours: 6,
            player_actor_id: "player".into(),
            locations: BTreeMap::from([(
                "room".into(),
                Location {
                    id: "room".into(),
                    name: "Room".into(),
                    container_id: None,
                    routes: BTreeMap::new(),
                    persistent_features: vec![],
                },
            )]),
            actors: BTreeMap::from([("player".into(), actor)]),
            institutions: BTreeMap::new(),
            clocks: BTreeMap::new(),
            facts: BTreeMap::new(),
            transcript: vec![],
            last_player_activity: Utc::now(),
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

    #[tokio::test]
    async fn narration_is_revision_bound_and_does_not_mutate_campaign() {
        let dir = tempfile::tempdir().unwrap();
        let store = CampaignStore::open(dir.path().join("campaign.cc")).unwrap();
        let seed = campaign();
        store.create_campaign(&seed, &[], &[]).unwrap();
        let narrator = Narrator {
            model: Arc::new(FixtureModel),
            model_name: "fixture".into(),
        };
        let (projection, _) = narrator.project(&store, &seed).await.unwrap();
        assert_eq!(projection.source_revision, 0);
        let persisted = store
            .load::<Campaign>("campaign.v1", &seed.id.to_string())
            .unwrap()
            .unwrap()
            .1;
        assert_eq!(persisted, seed);
        assert!(store.keys("narration_projection.v1").unwrap().is_empty());
    }
}
