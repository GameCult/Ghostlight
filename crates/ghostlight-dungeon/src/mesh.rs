use crate::{
    domain::{
        ActionAssessment, ActorState, ActorStateDelta, Campaign, CanonCandidate, Event,
        GestaltMemberDelta, GestaltPersonaState, InstitutionState, Location, NarrationProjection,
        NewsIssue, RollReceipt, StrategicTickPlan, VaultEvidenceReceipt, WorldActionProposal,
        WorldClock, WorldCommitReceipt, WorldCompilePreview, WorldFact,
    },
    model::ModelStageReceipt,
    surface::player_surface,
};
use anyhow::{Context, Result};
use chrono::Utc;
use cultcache_rs::DatabaseEntry;
use cultmesh_rs::{
    CultMesh, CultMeshNode, CultMeshNodeOptions, CultMeshRudpDocumentPublishOptions,
};
use serde_json::{Value, json};
use std::{
    net::SocketAddr,
    path::Path,
    sync::{Arc, Mutex},
};

pub const PROVIDER_ID: &str = "gamecult.ghostlight.dungeon";
pub const HEALTH_KEY: &str = "ghostlight:dungeon:health";
pub const ADVERTISEMENT_KEY: &str = "eve:provider:gamecult.ghostlight.dungeon";

#[derive(Clone, Debug, PartialEq, DatabaseEntry)]
#[cultcache(type = "gamecult.eve.surface", schema = "gamecult.eve.surface.v1")]
pub struct EveSurfaceRecord {
    #[cultcache(key = 0)]
    pub value: Value,
}

#[derive(Clone, Debug, PartialEq, DatabaseEntry)]
#[cultcache(
    type = "gamecult.eve.surface_state",
    schema = "gamecult.eve.surface_state.v1"
)]
pub struct EveSurfaceStateRecord {
    #[cultcache(key = 0)]
    pub provider_id: String,
    #[cultcache(key = 1)]
    pub title: String,
    #[cultcache(key = 2)]
    pub version: i64,
    #[cultcache(key = 3)]
    pub updated_at: String,
    #[cultcache(key = 4)]
    pub surface: Value,
}

#[derive(Clone, Debug, PartialEq, DatabaseEntry)]
#[cultcache(
    type = "gamecult.eve.provider_advertisement",
    schema = "gamecult.eve.provider_advertisement.v1"
)]
pub struct EveProviderAdvertisementRecord {
    #[cultcache(key = 0)]
    pub value: Value,
}

#[derive(Clone, Debug, PartialEq, DatabaseEntry)]
#[cultcache(
    type = "ghostlight.service_health",
    schema = "ghostlight.service_health.v1"
)]
pub struct ServiceHealthRecord {
    #[cultcache(key = 0)]
    pub value: Value,
}

#[derive(Clone, Debug, PartialEq, DatabaseEntry)]
#[cultcache(
    type = "ghostlight.schema_catalog",
    schema = "ghostlight.schema_catalog.v1"
)]
pub struct SchemaCatalogRecord {
    #[cultcache(key = 0)]
    pub value: Value,
}

cultmesh_rs::cultmesh_documents!(GhostlightDocuments {
    EveSurfaceRecord => "gamecult.eve.surface.v1",
    EveSurfaceStateRecord => "gamecult.eve.surface_state.v1",
    EveProviderAdvertisementRecord => "gamecult.eve.provider_advertisement.v1",
    ServiceHealthRecord => "ghostlight.service_health.v1",
    SchemaCatalogRecord => "ghostlight.schema_catalog.v1",
});

#[derive(Clone)]
pub struct MeshPublisher {
    node: Arc<Mutex<CultMeshNode>>,
    rudp_target: Option<SocketAddr>,
}

impl MeshPublisher {
    pub fn open(store_path: impl AsRef<Path>, rudp_target: Option<SocketAddr>) -> Result<Self> {
        if let Some(parent) = store_path.as_ref().parent() {
            std::fs::create_dir_all(parent)?;
        }
        let node = CultMesh::create_node(
            store_path,
            GhostlightDocuments,
            CultMeshNodeOptions {
                runtime_id: "ghostlight-dungeon-starfire".into(),
                pull_on_start: true,
            },
        )?;
        Ok(Self {
            node: Arc::new(Mutex::new(node)),
            rudp_target,
        })
    }

    pub fn publish_snapshot(
        &self,
        campaigns: &[(Campaign, Vec<NarrationProjection>)],
        deepseek_status: &str,
        live_turn_pressure: usize,
    ) -> Result<Value> {
        let updated_at = Utc::now().to_rfc3339();
        let health = json!({
            "schema":"ghostlight.service_health.v1",
            "status":"ok",
            "campaigns":campaigns.len(),
            "deepseek":deepseek_status,
            "scheduler":{"live_turn_pressure":live_turn_pressure},
            "updatedAtUtc":updated_at,
            "runtime":"ghostlight-dungeon-starfire",
            "commit":option_env!("GHOSTLIGHT_BUILD_COMMIT").unwrap_or("development")
        });
        let schema_ids = vec![
            "gamecult.eve.surface.v1",
            "gamecult.eve.surface_state.v1",
            "gamecult.eve.provider_advertisement.v1",
            "ghostlight.service_health.v1",
            "ghostlight.schema_catalog.v1",
            "ghostlight.campaign.v1",
            "ghostlight.player_action_assessment.v1",
            "ghostlight.roll_receipt.v1",
            "ghostlight.world_commit_receipt.v1",
            "ghostlight.persona_stage_receipt.v1",
            "ghostlight.actor_state_delta.v1",
            "ghostlight.world_action_proposal.v1",
            "ghostlight.strategic_tick.v1",
            "ghostlight.news_issue.v1",
            "ghostlight.canon_candidate.v1",
        ];
        let catalog = json!({
            "schema":"ghostlight.schema_catalog.v1",
            "providerId":PROVIDER_ID,
            "schemas":schema_catalog(),
            "updatedAtUtc":updated_at
        });
        let surfaces = campaigns
            .iter()
            .map(|(campaign, _)| {
                json!({
                    "schema":"gamecult.eve.surface.v1",
                    "surfaceId":format!("ghostlight.campaign.{}",campaign.id),
                    "key":format!("eve:surface:ghostlight.campaign.{}",campaign.id),
                    "transport":"cultmesh-record",
                    "status":"available",
                    "audience":"authenticated-player",
                    "mode":"interactive",
                    "surfaceKind":"interactive-world",
                    "interactionModel":"provider-command-receipts"
                })
            })
            .collect::<Vec<_>>();
        let advertisement = json!({
            "schema":"gamecult.eve.provider_advertisement.v1",
            "providerId":PROVIDER_ID,
            "serviceId":"ghostlight-dungeon-starfire",
            "verseId":"gamecult.private",
            "rootVerse":"gamecult",
            "canonicalService":"ghostlight-dungeon",
            "locatedService":"starfire",
            "cultMeshAddress":"cultmesh://gamecult.private/ghostlight/providers/dungeon",
            "title":"GhostlightDungeon",
            "kind":"narrative.simulation",
            "updatedAtUtc":updated_at,
            "freshness":{"state":"live","lastSeenAtUtc":updated_at,"maxAgeMs":360000},
            "schemas":schema_ids,
            "surfaces":surfaces,
            "commands":[{"command":"ghostlight.world.commands","schema":"gamecult.eve.command.v1","transport":"cultmesh","summary":"Revision-bound world command intents; WorldKernel owns receipts."}]
        });

        let mut node = self
            .node
            .lock()
            .map_err(|_| anyhow::anyhow!("mesh lock poisoned"))?;
        self.put_and_publish(
            &mut node,
            HEALTH_KEY,
            &ServiceHealthRecord {
                value: health.clone(),
            },
        )?;
        self.put_and_publish(
            &mut node,
            "ghostlight:schema-catalog",
            &SchemaCatalogRecord { value: catalog },
        )?;
        self.put_and_publish(
            &mut node,
            ADVERTISEMENT_KEY,
            &EveProviderAdvertisementRecord {
                value: advertisement,
            },
        )?;
        for (campaign, narrations) in campaigns {
            let surface = player_surface(campaign, narrations);
            let key = format!("eve:surface:ghostlight.campaign.{}", campaign.id);
            self.put_and_publish(
                &mut node,
                &key,
                &EveSurfaceRecord {
                    value: surface.clone(),
                },
            )?;
            self.put_and_publish(
                &mut node,
                format!("eve:surface-state:ghostlight.campaign.{}", campaign.id),
                &EveSurfaceStateRecord {
                    provider_id: PROVIDER_ID.into(),
                    title: campaign.name.clone(),
                    version: campaign.revision as i64,
                    updated_at: updated_at.clone(),
                    surface,
                },
            )?;
        }
        node.flush()?;
        Ok(health)
    }

    pub fn health(&self) -> Result<Value> {
        self.node
            .lock()
            .map_err(|_| anyhow::anyhow!("mesh lock poisoned"))?
            .get_required::<ServiceHealthRecord>(HEALTH_KEY)
            .map(|record| record.value)
    }

    fn put_and_publish<T>(
        &self,
        node: &mut CultMeshNode,
        key: impl Into<String>,
        value: &T,
    ) -> Result<()>
    where
        T: DatabaseEntry + serde::Serialize,
    {
        let key = key.into();
        node.put(&key, value)?;
        if let Some(target) = self.rudp_target {
            let mut options =
                CultMeshRudpDocumentPublishOptions::odin(target, "ghostlight-dungeon-starfire");
            options.source_agent_id = Some(PROVIDER_ID.into());
            options.source_role = Some("narrative-simulation".into());
            options.tags = vec!["ghostlight".into(), "eve".into(), "private".into()];
            node.publish_document_to_rudp_catalog(&key, value, options)
                .with_context(|| format!("publish {key} to Odin RUDP catalog"))?;
        }
        Ok(())
    }
}

fn schema_catalog() -> Value {
    json!({
        "ghostlight.vault_evidence_receipt.v1": schemars::schema_for!(VaultEvidenceReceipt),
        "ghostlight.world_compile_preview.v1": schemars::schema_for!(WorldCompilePreview),
        "ghostlight.campaign.v1": schemars::schema_for!(Campaign),
        "ghostlight.world_fact.v1": schemars::schema_for!(WorldFact),
        "ghostlight.location.v1": schemars::schema_for!(Location),
        "ghostlight.actor_state.v1": schemars::schema_for!(ActorState),
        "ghostlight.institution_state.v1": schemars::schema_for!(InstitutionState),
        "ghostlight.world_clock.v1": schemars::schema_for!(WorldClock),
        "ghostlight.event.v1": schemars::schema_for!(Event),
        "ghostlight.player_action_assessment.v1": schemars::schema_for!(ActionAssessment),
        "ghostlight.roll_receipt.v1": schemars::schema_for!(RollReceipt),
        "ghostlight.world_commit_receipt.v1": schemars::schema_for!(WorldCommitReceipt),
        "ghostlight.persona_stage_receipt.v1": schemars::schema_for!(ModelStageReceipt),
        "ghostlight.actor_state_delta.v1": schemars::schema_for!(ActorStateDelta),
        "ghostlight.world_action_proposal.v1": schemars::schema_for!(WorldActionProposal),
        "ghostlight.strategic_tick_plan.v1": schemars::schema_for!(StrategicTickPlan),
        "ghostlight.news_issue.v1": schemars::schema_for!(NewsIssue),
        "ghostlight.canon_candidate.v1": schemars::schema_for!(CanonCandidate),
        "ghostlight.gestalt_persona_state.v1": schemars::schema_for!(GestaltPersonaState),
        "ghostlight.gestalt_member_delta.v1": schemars::schema_for!(GestaltMemberDelta)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn health_is_read_from_the_same_typed_mesh_record_that_is_published() {
        let temp = tempdir().unwrap();
        let publisher = MeshPublisher::open(temp.path().join("mesh.cc"), None).unwrap();
        let written = publisher.publish_snapshot(&[], "fixture-ready", 2).unwrap();
        assert_eq!(publisher.health().unwrap(), written);
        assert_eq!(written["scheduler"]["live_turn_pressure"], 2);
        let catalog = publisher
            .node
            .lock()
            .unwrap()
            .get_required::<SchemaCatalogRecord>("ghostlight:schema-catalog")
            .unwrap();
        assert!(catalog.value["schemas"]["ghostlight.campaign.v1"]["$schema"].is_string());
        drop(catalog);
        drop(publisher);
        let reopened = MeshPublisher::open(temp.path().join("mesh.cc"), None).unwrap();
        assert_eq!(reopened.health().unwrap(), written);
    }
}
