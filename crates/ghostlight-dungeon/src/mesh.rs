use crate::{
    domain::{
        Campaign, CellAppraisal, GestaltMaterializationReceipt, RejectedProposalReceipt,
        ResolutionControlReceipt, ResolutionPlanReceipt, StrategicActivityOutcome,
        StrategicTickReceipt, VaultEvidenceReceipt, WorldCommitReceipt,
    },
    model::{ModelRuntimeStatus, ModelStageReceipt},
    session_zero::{CampaignMembership, SessionZeroState},
    surface::{operator_surface, player_surface, player_surface_for_actor},
    vault::VaultSourceManifest,
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
pub const SURFACE_ID: &str = "ghostlight.play";
pub const COMMAND_BOUNDARY: &str = "ghostlight.eve.commands";
pub const COMMAND_RESULT_SCHEMA: &str = "gamecult.eve.command_result.v1";
pub const HEALTH_KEY: &str = "ghostlight:dungeon:health";
pub const ADVERTISEMENT_KEY: &str = "eve:provider:gamecult.ghostlight.dungeon";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MeshRuntimeIdentity {
    pub runtime_id: String,
    pub service_id: String,
    pub located_service: String,
}

impl Default for MeshRuntimeIdentity {
    fn default() -> Self {
        Self {
            runtime_id: "ghostlight-dungeon-starfire".into(),
            service_id: "ghostlight-dungeon-starfire".into(),
            located_service: "starfire".into(),
        }
    }
}

#[derive(Clone)]
pub struct CampaignMeshSnapshot {
    pub campaign: Campaign,
    pub membership: Option<CampaignMembership>,
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

#[derive(Clone)]
pub struct SessionZeroMeshSnapshot {
    pub session_zero_id: uuid::Uuid,
    pub member_id: String,
    pub surface: Value,
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
    identity: MeshRuntimeIdentity,
}

#[derive(Clone)]
struct RemoteReplication {
    target: SocketAddr,
    runtime_id: String,
    pending: Arc<(Mutex<Option<Vec<cultnet_rs::CultNetMessage>>>, Condvar)>,
}

impl MeshPublisher {
    pub fn open(store_path: impl AsRef<Path>, rudp_target: Option<SocketAddr>) -> Result<Self> {
        Self::open_with_identity(store_path, rudp_target, MeshRuntimeIdentity::default())
    }

    pub fn open_with_identity(
        store_path: impl AsRef<Path>,
        rudp_target: Option<SocketAddr>,
        identity: MeshRuntimeIdentity,
    ) -> Result<Self> {
        if let Some(parent) = store_path.as_ref().parent() {
            std::fs::create_dir_all(parent)?;
        }
        let node = CultMesh::create_node(
            store_path,
            GhostlightDocuments,
            CultMeshNodeOptions {
                runtime_id: identity.runtime_id.clone(),
                pull_on_start: true,
            },
        )?;
        let remote =
            rudp_target.map(|target| RemoteReplication::start(target, identity.runtime_id.clone()));
        Ok(Self {
            node: Arc::new(Mutex::new(node)),
            remote,
            identity,
        })
    }

    pub fn identity(&self) -> &MeshRuntimeIdentity {
        &self.identity
    }

    pub fn provider_advertisement(&self, updated_at: &str) -> Value {
        json!({
            "schema":"gamecult.eve.provider_advertisement.v1",
            "providerId":PROVIDER_ID,
            "serviceId":self.identity.service_id.as_str(),
            "verseId":"gamecult.private",
            "rootVerse":"gamecult",
            "canonicalService":"ghostlight-dungeon",
            "locatedService":self.identity.located_service.as_str(),
            "cultMeshAddress":"cultmesh://gamecult.private/ghostlight/providers/dungeon",
            "title":"Ghostlight Dungeon",
            "kind":"narrative.simulation",
            "updatedAtUtc":updated_at,
            "freshness":{"state":"live","lastSeenAtUtc":updated_at,"maxAgeMs":360000},
            "schemas":[
                "gamecult.eve.surface.v1",
                "gamecult.eve.command_invocation.v1",
                COMMAND_RESULT_SCHEMA,
                "gamecult.eve.command_receipt.v1",
                "heimdall.access_gate_state.v1"
            ],
            "witnesses":[{"kind":"source-commit","ref":option_env!("GHOSTLIGHT_BUILD_COMMIT").unwrap_or("development")}],
            "worldConsumerBoundary":{
                "serviceId":"ghostlight.world.consumer",
                "transport":"cultnet.transport.rudp.v0",
                "endpoint":"127.0.0.1:4102",
                "exposure":"loopback-only",
                "operations":[
                    "ghostlight.world.seed.admit",
                    "ghostlight.world.external.snapshot.apply",
                    "ghostlight.world.external.proposals.list",
                    "ghostlight.world.external.proposal.acknowledge",
                    "ghostlight.world.newspaper.compose"
                ],
                "ownership":"Consumers own declared external subjects; Ghostlight owns all simulated world subjects and emits proposals across that boundary."
            },
            "surfaces":[{
                "surfaceId":SURFACE_ID,
                "schema":"gamecult.eve.surface.v1",
                "url":"/api/eve/surfaces/ghostlight.play",
                "transport":"https-json",
                "status":"available",
                "surfaceKind":"subject-scoped-narrative",
                "interactionModel":"typed-command-receipts",
                "worldInteraction":{
                    "projectionKind":"actor-filtered-authoritative",
                    "stateSchemas":["ghostlight.session_zero.v1","ghostlight.campaign.v1"],
                    "commandBoundary":COMMAND_BOUNDARY,
                    "nativeCommandBoundary":{
                        "serviceId":"ghostlight.native.player",
                        "transport":"cultnet.transport.rudp.v0",
                        "endpoint":"127.0.0.1:4102",
                        "exposure":"loopback-only",
                        "authentication":"Heimdall-backed Ghostlight app session",
                        "operations":["ghostlight.auth.begin","ghostlight.auth.complete","ghostlight.surface.get","ghostlight.eve.invoke"]
                    },
                    "receiptSchema":COMMAND_RESULT_SCHEMA,
                    "loweringTargets":["browser","eve-native","tui"],
                    "ownership":"Ghostlight kernels own state; Eve lowers projections and commands."
                },
                "requiresPlugins":[{
                    "pluginId":"gamecult.heimdall.access",
                    "versionRange":"^1.0.0",
                    "availability":"required",
                    "requiredCapabilities":["auth.gate","auth.begin","auth.complete","auth.logout"]
                }]
            }],
            "commands":[
                {"command":COMMAND_BOUNDARY,"transport":"https-json","summary":"Canonical Ghostlight Eve command boundary."},
                {"command":"ghostlight.native.player","transport":"cultnet.transport.rudp.v0","summary":"Loopback native access to the same actor-filtered surface and Eve command authority."},
                {"command":"ghostlight.world.consumer","transport":"cultnet.transport.rudp.v0","summary":"Typed world-seed admission, external snapshots, and proposal exchange for game consumers."}
            ]
        })
    }

    pub fn publish_snapshot(
        &self,
        campaigns: &[CampaignMeshSnapshot],
        session_zeros: &[SessionZeroMeshSnapshot],
        model_status: &ModelRuntimeStatus,
        live_turn_pressure: usize,
    ) -> Result<Value> {
        let updated_at = Utc::now().to_rfc3339();
        let health = json!({
            "schema":"ghostlight.service_health.v1",
            "status":"ok",
            "campaigns":campaigns.len(),
            "sessionZeros":session_zeros.len(),
            "modelProvider":model_status,
            "scheduler":{"live_turn_pressure":live_turn_pressure},
            "updatedAtUtc":updated_at,
            "runtime":self.identity.runtime_id.as_str(),
            "commit":option_env!("GHOSTLIGHT_BUILD_COMMIT").unwrap_or("development")
        });
        let catalog = json!({
            "schema":"ghostlight.schema_catalog.v1",
            "providerId":PROVIDER_ID,
            "schemas":schema_catalog(),
            "updatedAtUtc":updated_at
        });
        let advertisement = self.provider_advertisement(&updated_at);

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
            ADVERTISEMENT_KEY,
            &EveProviderAdvertisementRecord {
                value: advertisement,
            },
            &mut remote_messages,
        )?;
        self.put_and_stage(
            &mut node,
            "ghostlight:schema-catalog",
            &SchemaCatalogRecord { value: catalog },
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
            if let Some(membership) = &snapshot.membership {
                for member in membership.members.values().filter(|member| member.active) {
                    let surface = player_surface_for_actor(campaign, &member.actor_id);
                    let key = format!(
                        "eve:surface:ghostlight.campaign.{}.{}",
                        campaign.id, member.member_id
                    );
                    self.put_and_stage(
                        &mut node,
                        &key,
                        &EveSurfaceRecord { value: surface },
                        &mut remote_messages,
                    )?;
                    self.put_and_stage(
                        &mut node,
                        format!(
                            "eve:surface-state:ghostlight.campaign.{}.{}",
                            campaign.id, member.member_id
                        ),
                        &EveSurfaceStateRecord {
                            provider_id: PROVIDER_ID.into(),
                            title: campaign.name.clone(),
                            version: interface_version,
                            updated_at: updated_at.clone(),
                            surface: player_surface_for_actor(campaign, &member.actor_id),
                        },
                        &mut remote_messages,
                    )?;
                }
            } else {
                let surface = player_surface(campaign);
                let key = format!("eve:surface:ghostlight.campaign.{}", campaign.id);
                self.put_and_stage(
                    &mut node,
                    &key,
                    &EveSurfaceRecord { value: surface },
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
                        surface: player_surface(campaign),
                    },
                    &mut remote_messages,
                )?;
            }
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
        }
        for snapshot in session_zeros {
            let key = format!(
                "eve:surface:ghostlight.session-zero.{}.{}",
                snapshot.session_zero_id, snapshot.member_id
            );
            self.put_and_stage(
                &mut node,
                &key,
                &EveSurfaceRecord {
                    value: snapshot.surface.clone(),
                },
                &mut remote_messages,
            )?;
            self.put_and_stage(
                &mut node,
                format!(
                    "eve:surface-state:ghostlight.session-zero.{}.{}",
                    snapshot.session_zero_id, snapshot.member_id
                ),
                &EveSurfaceStateRecord {
                    provider_id: PROVIDER_ID.into(),
                    title: snapshot.surface["title"]
                        .as_str()
                        .unwrap_or("Session Zero")
                        .into(),
                    version: snapshot.surface["version"].as_i64().unwrap_or_default(),
                    updated_at: updated_at.clone(),
                    surface: snapshot.surface.clone(),
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

    pub fn publish_live_turn_pressure(&self, live_turn_pressure: usize) -> Result<Value> {
        let mut node = self
            .node
            .lock()
            .map_err(|_| anyhow::anyhow!("mesh lock poisoned"))?;
        let mut health = node.get_required::<ServiceHealthRecord>(HEALTH_KEY)?.value;
        health["scheduler"]["live_turn_pressure"] = json!(live_turn_pressure);
        health["updatedAtUtc"] = json!(Utc::now().to_rfc3339());
        let mut remote_messages = Vec::new();
        self.put_and_stage(
            &mut node,
            HEALTH_KEY,
            &ServiceHealthRecord {
                value: health.clone(),
            },
            &mut remote_messages,
        )?;
        node.flush()?;
        drop(node);
        if let Some(remote) = &self.remote {
            remote.enqueue(remote_messages);
        }
        Ok(health)
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
    fn start(target: SocketAddr, runtime_id: String) -> Self {
        let pending = Arc::new((
            Mutex::new(None::<Vec<cultnet_rs::CultNetMessage>>),
            Condvar::new(),
        ));
        let worker_pending = pending.clone();
        let worker_runtime_id = runtime_id.clone();
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
                let options = remote_options(target, &worker_runtime_id);
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
        Self {
            target,
            runtime_id,
            pending,
        }
    }

    fn options(&self) -> CultMeshRudpDocumentPublishOptions {
        remote_options(self.target, &self.runtime_id)
    }

    fn enqueue(&self, messages: Vec<cultnet_rs::CultNetMessage>) {
        let (lock, ready) = &*self.pending;
        let mut pending = lock.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        *pending = Some(messages);
        ready.notify_one();
    }
}

fn remote_options(target: SocketAddr, runtime_id: &str) -> CultMeshRudpDocumentPublishOptions {
    let mut options = CultMeshRudpDocumentPublishOptions::odin(target, runtime_id);
    options.source_agent_id = Some(PROVIDER_ID.into());
    options.source_role = Some("narrative-simulation".into());
    options.tags = vec!["ghostlight".into(), "eve".into(), "private".into()];
    options
}

fn schema_catalog() -> Value {
    json!({
        "ghostlight.campaign.v1": schemars::schema_for!(Campaign),
        "ghostlight.session_zero.v1": schemars::schema_for!(SessionZeroState),
        "ghostlight.vault_source_manifest.v1": schemars::schema_for!(VaultSourceManifest),
        "ghostlight.world_seed.v1": schemars::schema_for!(crate::consumer::WorldSeed),
        "ghostlight.world_seed_admission_request.v1": schemars::schema_for!(crate::consumer::WorldSeedAdmissionRequest),
        "ghostlight.world_seed_admission_receipt.v1": schemars::schema_for!(crate::consumer::WorldSeedAdmissionReceipt),
        "ghostlight.external_subject_authority.v1": schemars::schema_for!(crate::consumer::ExternalSubjectAuthority),
        "ghostlight.external_subject_snapshot.v1": schemars::schema_for!(crate::consumer::ExternalSubjectSnapshot),
        "ghostlight.external_snapshot_receipt.v1": schemars::schema_for!(crate::consumer::ExternalSnapshotReceipt),
        "ghostlight.external_world_proposal.v1": schemars::schema_for!(crate::consumer::ExternalWorldProposal),
        "ghostlight.external_proposal_list_request.v1": schemars::schema_for!(crate::consumer::ExternalProposalListRequest),
        "ghostlight.external_proposal_list.v1": schemars::schema_for!(crate::consumer::ExternalProposalList),
        "ghostlight.external_proposal_acknowledgement.v1": schemars::schema_for!(crate::consumer::ExternalProposalAcknowledgement),
        "ghostlight.external_proposal_receipt.v1": schemars::schema_for!(crate::consumer::ExternalProposalReceipt),
        "ghostlight.world_newspaper_request.v2": schemars::schema_for!(crate::consumer::WorldNewspaperRequest),
        "ghostlight.world_newspaper_issue.v3": schemars::schema_for!(crate::newspaper::WorldNewspaperIssue)
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
        let model_status = fixture_model_status();
        let written = publisher
            .publish_snapshot(&[], &[], &model_status, 2)
            .unwrap();
        assert_eq!(publisher.health().unwrap(), written);
        assert_eq!(written["scheduler"]["live_turn_pressure"], 2);
        assert_eq!(written["modelProvider"]["provider"], "fixture");
        assert!(written.get("deepseek").is_none());
        let catalog = publisher
            .node
            .lock()
            .unwrap()
            .get_required::<SchemaCatalogRecord>("ghostlight:schema-catalog")
            .unwrap();
        let schemas = catalog.value["schemas"].as_object().unwrap();
        assert_eq!(schemas.len(), 16);
        assert!(schemas["ghostlight.campaign.v1"]["$schema"].is_string());
        assert!(schemas["ghostlight.session_zero.v1"]["$schema"].is_string());
        assert!(schemas["ghostlight.vault_source_manifest.v1"]["$schema"].is_string());
        assert!(schemas["ghostlight.world_newspaper_request.v2"]["$schema"].is_string());
        assert!(schemas["ghostlight.world_newspaper_issue.v3"]["$schema"].is_string());
        assert!(!schemas.contains_key("ghostlight.persona_stage_receipt.v1"));
        assert!(!schemas.contains_key("ghostlight.world_mutation.v1"));
        drop(catalog);
        drop(publisher);
        let reopened = MeshPublisher::open(temp.path().join("mesh.cc"), None).unwrap();
        assert_eq!(reopened.health().unwrap(), written);
    }

    #[test]
    fn live_turn_pressure_updates_the_canonical_health_document_immediately() {
        let temp = tempdir().unwrap();
        let publisher = MeshPublisher::open(temp.path().join("mesh.cc"), None).unwrap();
        publisher
            .publish_snapshot(&[], &[], &fixture_model_status(), 0)
            .unwrap();

        let pressured = publisher.publish_live_turn_pressure(3).unwrap();
        assert_eq!(pressured["scheduler"]["live_turn_pressure"], 3);
        assert_eq!(publisher.health().unwrap(), pressured);

        let idle = publisher.publish_live_turn_pressure(0).unwrap();
        assert_eq!(idle["scheduler"]["live_turn_pressure"], 0);
        assert_eq!(publisher.health().unwrap(), idle);
    }

    #[test]
    fn unavailable_odin_cannot_take_away_the_local_projection() {
        let temp = tempdir().unwrap();
        let unavailable = "127.0.0.1:1".parse().unwrap();
        let publisher =
            MeshPublisher::open(temp.path().join("mesh.cc"), Some(unavailable)).unwrap();

        let started = std::time::Instant::now();
        let model_status = fixture_model_status();
        let written = publisher
            .publish_snapshot(&[], &[], &model_status, 0)
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

    fn fixture_model_status() -> ModelRuntimeStatus {
        ModelRuntimeStatus {
            provider: "fixture".into(),
            fast_model: "fixture-fast".into(),
            balanced_model: "fixture-balanced".into(),
            capable_model: "fixture-capable".into(),
            readiness: "ready".into(),
        }
    }

    #[test]
    fn public_provider_advertises_one_subject_scoped_surface_and_heimdall_plugin() {
        let temp = tempdir().unwrap();
        let publisher = MeshPublisher::open_with_identity(
            temp.path().join("mesh.cc"),
            None,
            MeshRuntimeIdentity {
                runtime_id: "ghostlight-test".into(),
                service_id: "ghostlight-test".into(),
                located_service: "fixture".into(),
            },
        )
        .unwrap();

        let advertisement = publisher.provider_advertisement("2026-08-22T00:00:00Z");
        let surfaces = advertisement["surfaces"].as_array().unwrap();
        assert_eq!(surfaces.len(), 1);
        assert_eq!(surfaces[0]["surfaceId"], SURFACE_ID);
        assert_eq!(
            surfaces[0]["worldInteraction"]["commandBoundary"],
            COMMAND_BOUNDARY
        );
        assert_eq!(
            surfaces[0]["worldInteraction"]["receiptSchema"],
            COMMAND_RESULT_SCHEMA
        );
        assert_eq!(
            surfaces[0]["worldInteraction"]["nativeCommandBoundary"]["endpoint"],
            "127.0.0.1:4102"
        );
        assert_eq!(
            surfaces[0]["worldInteraction"]["nativeCommandBoundary"]["exposure"],
            "loopback-only"
        );
        assert_eq!(
            surfaces[0]["requiresPlugins"][0]["pluginId"],
            "gamecult.heimdall.access"
        );
        assert!(
            advertisement.to_string().find("account_hash").is_none(),
            "public advertisement must remain subject-agnostic"
        );
    }
}
