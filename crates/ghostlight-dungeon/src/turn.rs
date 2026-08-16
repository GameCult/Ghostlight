use crate::{
    domain::{ActorReaction, Campaign},
    model::ModelStageReceipt,
    persistence::CampaignStore,
    persona::{ExecutionPermit, PermittedActorSlice, PersonaProjectionEngine},
};
use anyhow::{Result, anyhow};
use async_trait::async_trait;

pub struct SnapshotPermit {
    store: CampaignStore,
    campaign_id: uuid::Uuid,
    revision: u64,
}
impl SnapshotPermit {
    pub fn new(store: CampaignStore, campaign_id: uuid::Uuid, revision: u64) -> Self {
        Self {
            store,
            campaign_id,
            revision,
        }
    }
}
#[async_trait]
impl ExecutionPermit for SnapshotPermit {
    async fn require(&self, _: &str, _: &str, _: &str) -> Result<()> {
        let value = self
            .store
            .load::<Campaign>("campaign.v1", &self.campaign_id.to_string())?
            .map(|(_, c)| c)
            .ok_or_else(|| anyhow!("campaign vanished during Persona wave"))?;
        if value.revision != self.revision {
            return Err(anyhow!("Persona projection snapshot is stale"));
        }
        Ok(())
    }
}

pub struct ReactionWaveOutput {
    pub reactions: Vec<ActorReaction>,
    pub receipts: Vec<ModelStageReceipt>,
}

pub async fn appraise_present(
    engine: PersonaProjectionEngine,
    campaign: &Campaign,
    event_summary: &str,
) -> Result<ReactionWaveOutput> {
    let player = campaign
        .actors
        .get(&campaign.player_actor_id)
        .ok_or_else(|| anyhow!("player actor missing"))?;
    let actors: Vec<_> = campaign
        .actors
        .values()
        .filter(|actor| {
            actor.id != campaign.player_actor_id && actor.location_id == player.location_id
        })
        .cloned()
        .collect();
    let perceived_actors = campaign
        .actors
        .values()
        .filter(|actor| actor.location_id == player.location_id)
        .map(|actor| (actor.id.clone(), actor.name.clone()))
        .collect::<std::collections::BTreeMap<_, _>>();
    let mut jobs = tokio::task::JoinSet::new();
    for actor in actors {
        let engine = engine.clone();
        let snapshot = format!("campaign:{}:revision:{}", campaign.id, campaign.revision);
        let event = event_summary.to_owned();
        let receipts = campaign.branch_origin.evidence_receipt_ids.clone();
        let perceived_actors = perceived_actors.clone();
        jobs.spawn(async move {
            let slice = PermittedActorSlice {
                actor_id: actor.id.clone(),
                location_id: actor.location_id.clone(),
                snapshot_binding: snapshot,
                identity_experience: vec![format!("You are {}.", actor.name)],
                memories: actor.memories,
                perceived_events: vec![event],
                perceived_actors,
                relationships: actor
                    .relationships
                    .into_iter()
                    .map(|(id, value)| format!("{id}: {value}"))
                    .collect(),
                goals: actor.goals,
                knowledge: actor.knowledge.into_iter().collect(),
                capabilities: actor.capabilities.into_iter().collect(),
                pressures: actor
                    .conditions
                    .into_iter()
                    .chain(actor.obligations)
                    .collect(),
                affordances: actor.equipment.into_iter().collect(),
                source_receipt_ids: receipts,
            };
            let terminal = engine.execute(slice).await?;
            Ok::<_, anyhow::Error>((actor.id, terminal))
        });
    }
    let mut reactions = Vec::new();
    let mut receipts = Vec::new();
    while let Some(result) = jobs.join_next().await {
        let (actor_id, terminal) =
            result.map_err(|e| anyhow!("Persona appraisal task failed: {e}"))??;
        receipts.extend(terminal.stage_receipts);
        reactions.push(ActorReaction {
            actor_id,
            speech: terminal.proposals.speech,
            private_delta: terminal.proposals.private_delta,
            action_proposals: terminal.proposals.world_actions,
        });
    }
    reactions.sort_by(|a, b| a.actor_id.cmp(&b.actor_id));
    Ok(ReactionWaveOutput {
        reactions,
        receipts,
    })
}
