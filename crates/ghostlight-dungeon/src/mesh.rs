//! Derived CultMesh and Eve projection.
//!
//! The mesh publishes the world owner's current surface. It never pulls or
//! repairs world state and it has no command authority.

use crate::{
    eve,
    world::{CONSUMER_PATCH_SCHEMA, CONSUMER_RECEIPT_SCHEMA, STATE_SCHEMA, WorldSnapshot},
};
use anyhow::Result;
use chrono::{DateTime, Utc};
use cultcache_rs::DatabaseEntry;
use cultmesh_rs::{
    CultMesh, CultMeshNode, CultMeshNodeOptions, CultMeshRudpDocumentPublishOptions,
    publish_cultnet_messages_to_rudp_catalog,
};
use serde_json::{Value, json};
use std::{
    net::SocketAddr,
    path::Path,
    sync::{
        Arc, Condvar, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread,
};

pub(crate) const PROVIDER_ID: &str = "gamecult.ghostlight.dungeon";
pub(crate) const SURFACE_ID: &str = "ghostlight.play";
pub(crate) const COMMAND_BOUNDARY: &str = "ghostlight.eve.commands";
pub(crate) const COMMAND_RESULT_SCHEMA: &str = "gamecult.eve.command_result.v1";

const HEALTH_KEY: &str = "ghostlight:dungeon:health";
const ADVERTISEMENT_KEY: &str = "eve:provider:gamecult.ghostlight.dungeon";
const SURFACE_KEY: &str = "eve:surface:ghostlight.play";

#[derive(Clone, Debug, PartialEq, DatabaseEntry)]
#[cultcache(type = "gamecult.eve.surface", schema = "gamecult.eve.surface.v1")]
struct EveSurfaceRecord {
    #[cultcache(key = 0)]
    value: Value,
}

#[derive(Clone, Debug, PartialEq, DatabaseEntry)]
#[cultcache(
    type = "gamecult.eve.provider_advertisement",
    schema = "gamecult.eve.provider_advertisement.v1"
)]
struct EveProviderAdvertisementRecord {
    #[cultcache(key = 0)]
    value: Value,
}

#[derive(Clone, Debug, PartialEq, DatabaseEntry)]
#[cultcache(
    type = "ghostlight.service_health",
    schema = "ghostlight.service_health.v2"
)]
struct ServiceHealthRecord {
    #[cultcache(key = 0)]
    value: Value,
}

cultmesh_rs::cultmesh_documents!(GhostlightDocuments {
    EveSurfaceRecord => "gamecult.eve.surface.v1",
    EveProviderAdvertisementRecord => "gamecult.eve.provider_advertisement.v1",
    ServiceHealthRecord => "ghostlight.service_health.v2",
});

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct MeshRuntimeIdentity {
    pub(crate) runtime_id: String,
    pub(crate) service_id: String,
    pub(crate) located_service: String,
}

impl Default for MeshRuntimeIdentity {
    fn default() -> Self {
        Self {
            runtime_id: "ghostlight-dungeon".into(),
            service_id: "ghostlight-dungeon".into(),
            located_service: "local".into(),
        }
    }
}

#[derive(Clone)]
pub(crate) struct MeshPublisher {
    node: Arc<Mutex<CultMeshNode>>,
    remote: Option<RemoteReplication>,
    identity: MeshRuntimeIdentity,
}

#[derive(Clone)]
struct RemoteReplication {
    target: SocketAddr,
    runtime_id: String,
    pending: Arc<(Mutex<Option<Vec<cultnet_rs::CultNetMessage>>>, Condvar)>,
    alive: Arc<AtomicBool>,
}

impl MeshPublisher {
    pub(crate) fn open(
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
                // Ghostlight derives its own interface from the local owner.
                // Stale remote surfaces must never become startup authority.
                pull_on_start: false,
            },
        )?;
        let remote = rudp_target
            .map(|target| RemoteReplication::start(target, identity.runtime_id.clone()))
            .transpose()?;
        Ok(Self {
            node: Arc::new(Mutex::new(node)),
            remote,
            identity,
        })
    }

    pub(crate) fn provider_advertisement(&self, updated_at: &str) -> Value {
        provider_advertisement(&self.identity, updated_at)
    }

    pub(crate) fn publish(&self, snapshot: Option<&WorldSnapshot>) -> Result<Value> {
        let surface = eve::mesh_surface(snapshot);
        let world_state = eve::world_state(snapshot);
        let version = eve::surface_version(snapshot);
        let updated_at = Utc::now().to_rfc3339();
        let health = json!({
            "schema":"ghostlight.service_health.v2",
            "status":"ok",
            "worldState":world_state,
            "surfaceVersion":version,
            "updatedAtUtc":updated_at,
            "runtime":self.identity.runtime_id,
            "commit":option_env!("GHOSTLIGHT_BUILD_COMMIT").unwrap_or("development")
        });
        let advertisement = provider_advertisement(&self.identity, &updated_at);
        let mut node = self
            .node
            .lock()
            .map_err(|_| anyhow::anyhow!("mesh lock poisoned"))?;
        let mut remote_messages = Vec::new();
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
            SURFACE_KEY,
            &EveSurfaceRecord { value: surface },
            &mut remote_messages,
        )?;
        // Health is the completion marker for this derived projection. A
        // failed advertisement or surface write must leave the prior version
        // visibly stale rather than publishing green first.
        self.put_and_stage(
            &mut node,
            HEALTH_KEY,
            &ServiceHealthRecord {
                value: health.clone(),
            },
            &mut remote_messages,
        )?;
        drop(node);
        if let Some(remote) = &self.remote {
            remote.enqueue(remote_messages);
        }
        Ok(health)
    }

    pub(crate) fn health(&self) -> Result<Value> {
        if self
            .remote
            .as_ref()
            .is_some_and(|remote| !remote.is_alive())
        {
            anyhow::bail!("Odin projection worker stopped");
        }
        self.node
            .try_lock()
            .map_err(|_| anyhow::anyhow!("mesh projection is busy or its lock is poisoned"))?
            .get_required::<ServiceHealthRecord>(HEALTH_KEY)
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

pub(crate) fn provider_advertisement(identity: &MeshRuntimeIdentity, updated_at: &str) -> Value {
    json!({
        "schema":"gamecult.eve.provider_advertisement.v1",
        "providerId":PROVIDER_ID,
        "serviceId":identity.service_id,
        "verseId":"gamecult.private",
        "rootVerse":"gamecult",
        "canonicalService":"ghostlight-dungeon",
        "locatedService":identity.located_service,
        "cultMeshAddress":"cultmesh://gamecult.private/ghostlight/providers/dungeon",
        "title":"Ghostlight Dungeon",
        "kind":"narrative.simulation",
        "updatedAtUtc":updated_at,
        "freshness":{"state":"live","lastSeenAtUtc":updated_at,"maxAgeMs":360000},
        "schemas":[
            "gamecult.eve.surface.v1",
            "gamecult.eve.command_invocation.v1",
            COMMAND_RESULT_SCHEMA,
            "heimdall.access_gate_state.v1",
            STATE_SCHEMA,
            CONSUMER_PATCH_SCHEMA,
            CONSUMER_RECEIPT_SCHEMA
        ],
        "witnesses":[{
            "kind":"source-commit",
            "ref":option_env!("GHOSTLIGHT_BUILD_COMMIT").unwrap_or("development")
        }],
        "surfaces":[{
            "surfaceId":SURFACE_ID,
            "schema":"gamecult.eve.surface.v1",
            "url":"/api/eve/surfaces/ghostlight.play",
            "transport":"https-json",
            "status":"available",
            "surfaceKind":"subject-scoped-narrative",
            "interactionModel":"typed-command-receipts",
            "worldInteraction":{
                "projectionKind":"owner-derived",
                "stateSchemas":[STATE_SCHEMA],
                "commandBoundary":COMMAND_BOUNDARY,
                "receiptSchema":COMMAND_RESULT_SCHEMA,
                "loweringTargets":["browser","eve-native","tui"],
                "ownership":"One Ghostlight WorldMailbox owns state; Eve lowers derived projections and commands."
            },
            "requiresPlugins":[{
                "pluginId":"gamecult.heimdall.access",
                "versionRange":"^1.0.0",
                "availability":"required",
                "requiredCapabilities":["auth.gate","auth.begin","auth.complete","auth.logout"]
            }]
        }],
        "commands":[{
            "command":COMMAND_BOUNDARY,
            "transport":"https-json",
            "summary":"Canonical Ghostlight Eve command boundary."
        }]
    })
}

impl RemoteReplication {
    fn start(target: SocketAddr, runtime_id: String) -> Result<Self> {
        let pending = Arc::new((
            Mutex::new(None::<Vec<cultnet_rs::CultNetMessage>>),
            Condvar::new(),
        ));
        let worker_pending = pending.clone();
        let worker_runtime_id = runtime_id.clone();
        let alive = Arc::new(AtomicBool::new(true));
        let worker_alive = alive.clone();
        thread::Builder::new()
            .name("ghostlight-odin-rudp".into())
            .spawn(move || {
                struct Liveness(Arc<AtomicBool>);
                impl Drop for Liveness {
                    fn drop(&mut self) {
                        self.0.store(false, Ordering::Release);
                    }
                }
                let _liveness = Liveness(worker_alive);
                loop {
                let messages = {
                    let (lock, ready) = &*worker_pending;
                    let mut pending = lock
                        .lock()
                        .unwrap_or_else(|poisoned| poisoned.into_inner());
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
                if let Err(error) =
                    publish_cultnet_messages_to_rudp_catalog(&messages, options)
                {
                    tracing::warn!(
                        document_count = messages.len(),
                        %target,
                        %error,
                        "Odin publication failed; retaining the newest derived projection for retry"
                    );
                    let (lock, ready) = &*worker_pending;
                    let mut pending = lock
                        .lock()
                        .unwrap_or_else(|poisoned| poisoned.into_inner());
                    if pending.is_none() {
                        *pending = Some(messages);
                    }
                    let _ = ready
                        .wait_timeout(pending, std::time::Duration::from_secs(2))
                        .unwrap_or_else(|poisoned| poisoned.into_inner());
                }
                }
            })?;
        Ok(Self {
            target,
            runtime_id,
            pending,
            alive,
        })
    }

    fn is_alive(&self) -> bool {
        self.alive.load(Ordering::Acquire)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn publishes_only_owner_derived_surface_and_health() {
        let directory = tempfile::tempdir().unwrap();
        let publisher = MeshPublisher::open(
            directory.path().join("mesh-v2.cc"),
            None,
            MeshRuntimeIdentity::default(),
        )
        .unwrap();
        let health = publisher.publish(None).unwrap();
        assert_eq!(health["worldState"], "empty");
        assert_eq!(health["surfaceVersion"], 0);
        assert_eq!(publisher.health().unwrap(), health);
        let node = publisher.node.lock().unwrap();
        let surface = node
            .get_required::<EveSurfaceRecord>(SURFACE_KEY)
            .unwrap()
            .value;
        assert_eq!(surface["type"], "surface-state");
        assert_eq!(surface["schema"], "gamecult.eve.surface.v1");
        assert_eq!(surface["providerId"], PROVIDER_ID);
        assert_eq!(surface["version"], 0);
        assert!(
            surface["updatedAtUtc"]
                .as_str()
                .is_some_and(|value| value.parse::<DateTime<Utc>>().is_ok())
        );
        assert_eq!(surface["surface"], eve::mesh_surface(None)["surface"]);
        assert_eq!(surface["commands"], eve::mesh_surface(None)["commands"]);
        assert!(
            node.get_required::<EveProviderAdvertisementRecord>(ADVERTISEMENT_KEY)
                .is_ok()
        );
        assert!(node.get_required::<ServiceHealthRecord>(HEALTH_KEY).is_ok());
    }

    #[test]
    fn unavailable_odin_cannot_block_local_publication() {
        let directory = tempfile::tempdir().unwrap();
        let publisher = MeshPublisher::open(
            directory.path().join("mesh-v2.cc"),
            Some("127.0.0.1:1".parse().unwrap()),
            MeshRuntimeIdentity::default(),
        )
        .unwrap();
        let started = std::time::Instant::now();
        publisher.publish(None).unwrap();
        assert!(started.elapsed() < std::time::Duration::from_millis(500));
        assert_eq!(publisher.health().unwrap()["worldState"], "empty");
    }

    #[test]
    fn stopped_odin_worker_makes_runtime_health_non_green() {
        let directory = tempfile::tempdir().unwrap();
        let publisher = MeshPublisher::open(
            directory.path().join("mesh-v2.cc"),
            Some("127.0.0.1:1".parse().unwrap()),
            MeshRuntimeIdentity::default(),
        )
        .unwrap();
        publisher.publish(None).unwrap();
        publisher
            .remote
            .as_ref()
            .unwrap()
            .alive
            .store(false, Ordering::Release);
        assert!(publisher.health().is_err());
    }

    #[test]
    fn advertisement_has_one_surface_and_one_command_boundary() {
        let directory = tempfile::tempdir().unwrap();
        let publisher = MeshPublisher::open(
            directory.path().join("mesh-v2.cc"),
            None,
            MeshRuntimeIdentity::default(),
        )
        .unwrap();
        let advertisement = publisher.provider_advertisement("2026-09-01T00:00:00Z");
        assert_eq!(advertisement["surfaces"].as_array().unwrap().len(), 1);
        assert_eq!(advertisement["commands"].as_array().unwrap().len(), 1);
        assert_eq!(advertisement["surfaces"][0]["surfaceId"], SURFACE_ID);
    }
}
