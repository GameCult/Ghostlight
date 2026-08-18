use crate::{
    domain::{
        ActionAssessment, ActorState, ActorStateDelta, AgencyProfile, AgencyRelation, Campaign,
        CampaignLifecycleReceipt, CanonCandidate, CellActionProposal, CellAppraisal, Event,
        GestaltFissionPreview, GestaltLineage, GestaltMaterializationReceipt, GestaltMemberDelta,
        GestaltPersonaState, InstitutionState, Location, NarrationProjection, NewsIssue,
        RegionExpansionPreview, RejectedProposalReceipt, RelationshipState,
        ResolutionControlReceipt, ResolutionCover, ResolutionDemand, ResolutionPin,
        ResolutionPlanReceipt, ResolutionPolicy, RollReceipt, SimulationCell,
        StrategicActivityOutcome, StrategicGestaltMigration, StrategicMemberMigration,
        StrategicTickPlan, StrategicTickReceipt, VaultEvidenceReceipt, VaultManifest,
        WorldActionProposal, WorldClock, WorldCommitReceipt, WorldCompilePreview, WorldFact,
    },
    model::ModelStageReceipt,
    surface::{operator_surface, player_surface},
};
use anyhow::Result;
use chrono::Utc;
use cultcache_rs::DatabaseEntry;
use cultmesh_rs::{
    CultMesh, CultMeshNode, CultMeshNodeOptions, CultMeshRudpDocumentPublishOptions,
    publish_cultnet_messages_to_rudp_catalog,
};
use serde_json::{Value, json};
use std::{
    net::SocketAddr,
    path::Path,
    sync::{Arc, Condvar, Mutex},
    thread,
};

pub const PROVIDER_ID: &str = "gamecult.ghostlight.dungeon";
pub const HEALTH_KEY: &str = "ghostlight:dungeon:health";
pub const ADVERTISEMENT_KEY: &str = "eve:provider:gamecult.ghostlight.dungeon";

#[derive(Clone)]
pub struct CampaignMeshSnapshot {
    pub campaign: Campaign,
    pub narrations: Vec<NarrationProjection>,
    pub evidence: Vec<VaultEvidenceReceipt>,
    pub commits: Vec<WorldCommitReceipt>,
    pub stages: Vec<ModelStageReceipt>,
    pub strategic_ticks: Vec<StrategicTickReceipt>,
    pub gestalt_receipts: Vec<GestaltMaterializationReceipt>,
    pub rejected: Vec<RejectedProposalReceipt>,
    pub resolution_plans: Vec<ResolutionPlanReceipt>,
    pub cell_appraisals: Vec<CellAppraisal>,
    pub activity_outcomes: Vec<StrategicActivityOutcome>,
    pub resolution_controls: Vec<ResolutionControlReceipt>,
}

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
    remote: Option<RemoteReplication>,
}

#[derive(Clone)]
struct RemoteReplication {
    target: SocketAddr,
    pending: Arc<(Mutex<Option<Vec<cultnet_rs::CultNetMessage>>>, Condvar)>,
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
        let remote = rudp_target.map(RemoteReplication::start);
        Ok(Self {
            node: Arc::new(Mutex::new(node)),
            remote,
        })
    }

    pub fn publish_snapshot(
        &self,
        campaigns: &[CampaignMeshSnapshot],
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
            "ghostlight.gestalt_migration.v1",
            "ghostlight.member_migration.v1",
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
            .flat_map(|snapshot| {
                let campaign = &snapshot.campaign;
                [
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
                    }),
                    json!({
                        "schema":"gamecult.eve.surface.v1",
                        "surfaceId":format!("ghostlight.operator.{}",campaign.id),
                        "key":format!("eve:operator:ghostlight.campaign.{}",campaign.id),
                        "transport":"cultmesh-record",
                        "status":"available",
                        "audience":"authenticated-operator",
                        "mode":"inspect",
                        "surfaceKind":"operator-inspector",
                        "interactionModel":"read-only-receipt-inspection"
                    }),
                ]
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
        let mut remote_messages = Vec::new();
        self.put_and_stage(
            &mut node,
            HEALTH_KEY,
            &ServiceHealthRecord {
                value: health.clone(),
            },
            &mut remote_messages,
        )?;
        self.put_and_stage(
            &mut node,
            "ghostlight:schema-catalog",
            &SchemaCatalogRecord { value: catalog },
            &mut remote_messages,
        )?;
        self.put_and_stage(
            &mut node,
            ADVERTISEMENT_KEY,
            &EveProviderAdvertisementRecord {
                value: advertisement,
            },
            &mut remote_messages,
        )?;
        for snapshot in campaigns {
            let campaign = &snapshot.campaign;
            let interface_version = campaign
                .revision
                .saturating_mul(1_000_000_000_000)
                .saturating_add(
                    campaign
                        .resolution_policy
                        .resolution_epoch
                        .saturating_mul(1_000_000),
                )
                .saturating_add(campaign.resolution_policy.provider_configuration_epoch)
                as i64;
            let surface = player_surface(campaign, &snapshot.narrations);
            let key = format!("eve:surface:ghostlight.campaign.{}", campaign.id);
            self.put_and_stage(
                &mut node,
                &key,
                &EveSurfaceRecord {
                    value: surface.clone(),
                },
                &mut remote_messages,
            )?;
            let operator = operator_surface(
                campaign,
                &snapshot.evidence,
                &snapshot.commits,
                &snapshot.stages,
                &snapshot.strategic_ticks,
                &snapshot.gestalt_receipts,
                &snapshot.rejected,
                &snapshot.resolution_plans,
                &snapshot.cell_appraisals,
                &snapshot.activity_outcomes,
                &snapshot.resolution_controls,
                live_turn_pressure,
            );
            self.put_and_stage(
                &mut node,
                format!("eve:operator:ghostlight.campaign.{}", campaign.id),
                &EveSurfaceRecord {
                    value: operator.clone(),
                },
                &mut remote_messages,
            )?;
            self.put_and_stage(
                &mut node,
                format!("eve:operator-state:ghostlight.campaign.{}", campaign.id),
                &EveSurfaceStateRecord {
                    provider_id: PROVIDER_ID.into(),
                    title: format!("{} operator", campaign.name),
                    version: interface_version,
                    updated_at: updated_at.clone(),
                    surface: operator,
                },
                &mut remote_messages,
            )?;
            self.put_and_stage(
                &mut node,
                format!("eve:surface-state:ghostlight.campaign.{}", campaign.id),
                &EveSurfaceStateRecord {
                    provider_id: PROVIDER_ID.into(),
                    title: campaign.name.clone(),
                    version: interface_version,
                    updated_at: updated_at.clone(),
                    surface,
                },
                &mut remote_messages,
            )?;
        }
        node.flush()?;
        drop(node);
        if let Some(remote) = &self.remote {
            remote.enqueue(remote_messages);
        }
        Ok(health)
    }

    pub fn health(&self) -> Result<Value> {
        self.node
            .lock()
            .map_err(|_| anyhow::anyhow!("mesh lock poisoned"))?
            .get_required::<ServiceHealthRecord>(HEALTH_KEY)
            .map(|record| record.value)
    }

    pub fn operator_surface(&self, campaign_id: uuid::Uuid) -> Result<Value> {
        self.node
            .lock()
            .map_err(|_| anyhow::anyhow!("mesh lock poisoned"))?
            .get_required::<EveSurfaceRecord>(&format!(
                "eve:operator:ghostlight.campaign.{campaign_id}"
            ))
            .map(|record| record.value)
    }

    fn put_and_stage<T>(
        &self,
        node: &mut CultMeshNode,
        key: impl Into<String>,
        value: &T,
        remote_messages: &mut Vec<cultnet_rs::CultNetMessage>,
    ) -> Result<()>
    where
        T: DatabaseEntry + serde::Serialize,
    {
        let key = key.into();
        node.put(&key, value)?;
        if let Some(remote) = &self.remote {
            remote_messages.push(node.create_rudp_document_message(
                &key,
                value,
                &remote.options(),
            )?);
        }
        Ok(())
    }
}

impl RemoteReplication {
    fn start(target: SocketAddr) -> Self {
        let pending = Arc::new((
            Mutex::new(None::<Vec<cultnet_rs::CultNetMessage>>),
            Condvar::new(),
        ));
        let worker_pending = pending.clone();
        thread::Builder::new()
            .name("ghostlight-odin-rudp".into())
            .spawn(move || loop {
                let messages = {
                    let (lock, ready) = &*worker_pending;
                    let mut pending = lock.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
                    while pending.is_none() {
                        pending = ready
                            .wait(pending)
                            .unwrap_or_else(|poisoned| poisoned.into_inner());
                    }
                    pending.take().unwrap_or_default()
                };
                if messages.is_empty() {
                    continue;
                }
                let options = remote_options(target);
                if let Err(error) = publish_cultnet_messages_to_rudp_catalog(&messages, options) {
                    tracing::warn!(
                        document_count = messages.len(),
                        %target,
                        %error,
                        "Odin RUDP batch publication failed; retained canonical local CultMesh projection"
                    );
                }
            })
            .expect("Ghostlight RUDP replication worker must start");
        Self { target, pending }
    }

    fn options(&self) -> CultMeshRudpDocumentPublishOptions {
        remote_options(self.target)
    }

    fn enqueue(&self, messages: Vec<cultnet_rs::CultNetMessage>) {
        let (lock, ready) = &*self.pending;
        let mut pending = lock.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        *pending = Some(messages);
        ready.notify_one();
    }
}

fn remote_options(target: SocketAddr) -> CultMeshRudpDocumentPublishOptions {
    let mut options =
        CultMeshRudpDocumentPublishOptions::odin(target, "ghostlight-dungeon-starfire");
    options.source_agent_id = Some(PROVIDER_ID.into());
    options.source_role = Some("narrative-simulation".into());
    options.tags = vec!["ghostlight".into(), "eve".into(), "private".into()];
    options
}

fn schema_catalog() -> Value {
    let mut catalog = json!({
        "ghostlight.vault_manifest.v1": schemars::schema_for!(VaultManifest),
        "ghostlight.vault_evidence_receipt.v1": schemars::schema_for!(VaultEvidenceReceipt),
        "ghostlight.world_compile_preview.v1": schemars::schema_for!(WorldCompilePreview),
        "ghostlight.campaign.v1": schemars::schema_for!(Campaign),
        "ghostlight.world_fact.v1": schemars::schema_for!(WorldFact),
        "ghostlight.location.v1": schemars::schema_for!(Location),
        "ghostlight.actor_state.v1": schemars::schema_for!(ActorState),
        "ghostlight.relationship_state.v1": schemars::schema_for!(RelationshipState),
        "ghostlight.institution_state.v1": schemars::schema_for!(InstitutionState),
        "ghostlight.world_clock.v1": schemars::schema_for!(WorldClock),
        "ghostlight.event.v1": schemars::schema_for!(Event),
        "ghostlight.player_action_assessment.v1": schemars::schema_for!(ActionAssessment),
        "ghostlight.roll_receipt.v1": schemars::schema_for!(RollReceipt),
        "ghostlight.world_commit_receipt.v1": schemars::schema_for!(WorldCommitReceipt),
        "ghostlight.persona_stage_receipt.v1": schemars::schema_for!(ModelStageReceipt),
        "ghostlight.actor_state_delta.v1": schemars::schema_for!(ActorStateDelta),
        "ghostlight.world_action_proposal.v1": schemars::schema_for!(WorldActionProposal),
        "ghostlight.narration_projection.v1": schemars::schema_for!(NarrationProjection),
        "ghostlight.region_expansion_preview.v1": schemars::schema_for!(RegionExpansionPreview),
        "ghostlight.rejected_proposal_receipt.v1": schemars::schema_for!(RejectedProposalReceipt),
        "ghostlight.campaign_lifecycle_receipt.v1": schemars::schema_for!(CampaignLifecycleReceipt),
        "ghostlight.strategic_tick_plan.v1": schemars::schema_for!(StrategicTickPlan),
        "ghostlight.strategic_tick.v1": schemars::schema_for!(StrategicTickReceipt),
        "ghostlight.news_issue.v1": schemars::schema_for!(NewsIssue),
        "ghostlight.canon_candidate.v1": schemars::schema_for!(CanonCandidate),
        "ghostlight.gestalt_persona_state.v1": schemars::schema_for!(GestaltPersonaState),
        "ghostlight.gestalt_member_delta.v1": schemars::schema_for!(GestaltMemberDelta)
        ,"ghostlight.gestalt_materialization_receipt.v1": schemars::schema_for!(GestaltMaterializationReceipt),
        "ghostlight.agency_profile.v1": schemars::schema_for!(AgencyProfile),
        "ghostlight.agency_relation.v1": schemars::schema_for!(AgencyRelation),
        "ghostlight.gestalt_lineage.v1": schemars::schema_for!(GestaltLineage),
        "ghostlight.resolution_policy.v1": schemars::schema_for!(ResolutionPolicy),
        "ghostlight.resolution_pin.v1": schemars::schema_for!(ResolutionPin),
        "ghostlight.resolution_demand.v1": schemars::schema_for!(ResolutionDemand),
        "ghostlight.simulation_cell.v1": schemars::schema_for!(SimulationCell),
        "ghostlight.resolution_cover.v1": schemars::schema_for!(ResolutionCover),
        "ghostlight.resolution_plan_receipt.v1": schemars::schema_for!(ResolutionPlanReceipt),
        "ghostlight.resolution_control_receipt.v1": schemars::schema_for!(ResolutionControlReceipt),
        "ghostlight.cell_appraisal.v1": schemars::schema_for!(CellAppraisal),
        "ghostlight.cell_action_proposal.v1": schemars::schema_for!(CellActionProposal),
        "ghostlight.gestalt_fission_preview.v1": schemars::schema_for!(GestaltFissionPreview)
    });
    let schemas = catalog
        .as_object_mut()
        .expect("schema catalog literal is an object");
    schemas.insert(
        "ghostlight.strategic_activity_outcome.v1".into(),
        serde_json::to_value(schemars::schema_for!(StrategicActivityOutcome))
            .expect("strategic activity outcome schema serializes"),
    );
    schemas.insert(
        "ghostlight.gestalt_migration.v1".into(),
        serde_json::to_value(schemars::schema_for!(StrategicGestaltMigration))
            .expect("gestalt migration schema serializes"),
    );
    schemas.insert(
        "ghostlight.member_migration.v1".into(),
        serde_json::to_value(schemars::schema_for!(StrategicMemberMigration))
            .expect("member migration schema serializes"),
    );
    catalog
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
        for schema in [
            "ghostlight.vault_manifest.v1",
            "ghostlight.relationship_state.v1",
            "ghostlight.strategic_tick.v1",
            "ghostlight.gestalt_materialization_receipt.v1",
            "ghostlight.agency_profile.v1",
            "ghostlight.resolution_cover.v1",
            "ghostlight.cell_appraisal.v1",
            "ghostlight.gestalt_fission_preview.v1",
        ] {
            assert!(catalog.value["schemas"][schema]["$schema"].is_string());
        }
        drop(catalog);
        drop(publisher);
        let reopened = MeshPublisher::open(temp.path().join("mesh.cc"), None).unwrap();
        assert_eq!(reopened.health().unwrap(), written);
    }

    #[test]
    fn unavailable_odin_cannot_take_away_the_local_projection() {
        let temp = tempdir().unwrap();
        let unavailable = "127.0.0.1:1".parse().unwrap();
        let publisher =
            MeshPublisher::open(temp.path().join("mesh.cc"), Some(unavailable)).unwrap();

        let started = std::time::Instant::now();
        let written = publisher
            .publish_snapshot(&[], "fixture-ready", 0)
            .expect("local projection remains writable while rendezvous is unavailable");

        assert!(
            started.elapsed() < std::time::Duration::from_millis(500),
            "optional remote replication blocked local publication"
        );
        assert_eq!(publisher.health().unwrap(), written);
        assert!(
            publisher
                .node
                .lock()
                .unwrap()
                .get_required::<SchemaCatalogRecord>("ghostlight:schema-catalog")
                .is_ok()
        );
    }
}
