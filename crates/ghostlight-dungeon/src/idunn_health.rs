//! Service-owned runtime presence at the Idunn/Odin authority boundary.
//!
//! Idunn supplies one immutable Expected/activation bundle, an activation-only
//! signing credential, the candidate bind, and (after Odin observes Warming)
//! one process write lease. Ghostlight reports what this process actually is;
//! it never promotes its route or manufactures Ready.

use crate::world::{STATE_SCHEMA, state_schema_compatibility_tag};
use anyhow::{Context, Result, anyhow, bail, ensure};
#[cfg(test)]
use cultcache_rs::CacheBackingStore;
use cultcache_rs::{CultCacheEnvelope, DatabaseEntry, SingleFileMessagePackBackingStore};
use cultnet_rs::{
    CultNetMessage, CultNetRawDocumentRecord, CultNetRawPayloadEncoding,
    CultNetRudpReliableSendStatus, CultNetRudpSocketTransportConnection,
    CultNetRudpSocketTransportOptions, CultNetWireContract,
    GAMECULT_RUNTIME_PRESENCE_HEALTH_SCHEMA, GameCultProviderHealthIdentity,
    GameCultRuntimeCapability, GameCultRuntimePresenceHealthPurpose,
    GameCultRuntimePresenceHealthRecord, IDUNN_EXPECTED_INCARNATION_SCHEMA,
    IDUNN_PROCESS_WRITE_LEASE_SCHEMA, IDUNN_RUNTIME_ACTIVATION_CREDENTIAL_NAME,
    IDUNN_RUNTIME_ACTIVATION_SCHEMA, IdunnExpectedDependency, IdunnExpectedIncarnationRecord,
    IdunnProcessWriteLeaseRecord, IdunnRuntimeActivationRecord, IdunnRuntimeActivationSigner,
    ServiceIdentitySigner, encode_cultnet_message_to_vec, open_service_identity_credential_reader,
};
use std::{
    collections::VecDeque,
    fs::{self, File},
    net::{SocketAddr, UdpSocket},
    path::{Path, PathBuf},
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};
#[cfg(target_os = "linux")]
use std::{
    os::fd::{AsRawFd, FromRawFd, RawFd},
    os::unix::fs::{MetadataExt, PermissionsExt},
};

pub const GHOSTLIGHT_RUNTIME_PRESENCE_HEALTH_CONTRACT: &str =
    "ghostlight.cultnet-rudp-service-health";
pub const IDUNN_RUNTIME_BUNDLE_ENVIRONMENT: &str = "GAMECULT_IDUNN_RUNTIME_BUNDLE";
pub const IDUNN_RUNTIME_CANDIDATE_BIND_ENVIRONMENT: &str = "GAMECULT_IDUNN_CANDIDATE_BIND";
pub const IDUNN_PROCESS_WRITE_LEASE_ENVIRONMENT: &str = "GAMECULT_IDUNN_PROCESS_WRITE_LEASE";

pub(crate) const TARGET: &str = "ghostlight";
const STATE_SCHEMA_GENERATION: &str = "world-v2";
const STATE_CONTRACT_SHA256: &str =
    "sha256-bf6ec06d885a59ddb237c6224d0abb4ccceac8c7ba23761d1326d7f562a4c21e";
const CULTNET_RUDP_PROTOCOL_ID: &str = "cultnet.transport.rudp.v0";
const ODIN_CULTMESH_CATALOG_CONNECTION_ID: u32 = 0x0d1d_0002;
const RUNTIME_PRESENCE_IDENTITY_FD_NAME: &str = "gamecult-runtime-presence-identity";
const WARMING_HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);
const MAX_RECENT_WARMING_PROOFS: usize = 64;
const ODIN_CAPABILITY: (&str, &str, &str) =
    ("odin.verse-rendezvous", "odin.verse-topology.v1", "v1");
const CONNECTOR_CAPABILITY: (&str, &str, &str) = (
    "gamecult.codex.subscription-inference",
    "gamecult.codex.transport_envelope.v2",
    "v2",
);
const HEIMDALL_CAPABILITY: (&str, &str, &str) = (
    "heimdall.command-boundary",
    "heimdall.command_boundary.v1",
    "v1",
);
#[cfg(target_os = "linux")]
const SYSTEMD_LISTEN_FDS_START: RawFd = 3;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct HeimdallDependencyBinding {
    pub(crate) provider_id: String,
    pub(crate) endpoint: SocketAddr,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RuntimeDependencyBindings {
    pub(crate) odin_rudp: SocketAddr,
    pub(crate) connector: SocketAddr,
    pub(crate) heimdall: HeimdallDependencyBinding,
}

pub(crate) struct PublishedRuntimePresence {
    canonical_sha256: String,
}

impl PublishedRuntimePresence {
    pub(crate) fn canonical_sha256(&self) -> &str {
        &self.canonical_sha256
    }
}

pub(crate) struct RuntimePresencePublisher {
    endpoint: SocketAddr,
    bound_endpoint: String,
    expected: IdunnExpectedIncarnationRecord,
    activation: IdunnRuntimeActivationRecord,
    provider_signer: ServiceIdentitySigner<GameCultProviderHealthIdentity>,
    activation_signer: IdunnRuntimeActivationSigner,
    capabilities: Vec<GameCultRuntimeCapability>,
    write_lease_path: PathBuf,
    dependency_bindings: RuntimeDependencyBindings,
    publisher_sequence: u64,
    active_write_lease_sha256: Option<String>,
}

impl RuntimePresencePublisher {
    pub(crate) fn from_environment(observed_bound_endpoint: SocketAddr) -> Result<Option<Self>> {
        let Some(bundle) = std::env::var_os(IDUNN_RUNTIME_BUNDLE_ENVIRONMENT) else {
            #[cfg(target_os = "linux")]
            bail!("{IDUNN_RUNTIME_BUNDLE_ENVIRONMENT} is mandatory on Linux");
            #[cfg(not(target_os = "linux"))]
            return Ok(None);
        };
        let bundle = PathBuf::from(bundle);
        require_runtime_bundle(&bundle)?;
        let expected = read_expected(&bundle.join("expected.cc"))?;
        let activation = read_activation(&bundle.join("activation.cc"), &expected.target)?;

        let odin_endpoint: SocketAddr = std::env::var("GHOSTLIGHT_ODIN_RUDP")
            .context("GHOSTLIGHT_ODIN_RUDP is required for runtime presence")?
            .parse()
            .context("GHOSTLIGHT_ODIN_RUDP is not a socket address")?;
        let connector_endpoint: SocketAddr = std::env::var("GHOSTLIGHT_CONTROLLER_CONNECTOR")
            .context("GHOSTLIGHT_CONTROLLER_CONNECTOR is required for managed runtime")?
            .parse()
            .context("GHOSTLIGHT_CONTROLLER_CONNECTOR is not a socket address")?;
        let write_lease_path = PathBuf::from(
            std::env::var_os(IDUNN_PROCESS_WRITE_LEASE_ENVIRONMENT)
                .with_context(|| format!("{IDUNN_PROCESS_WRITE_LEASE_ENVIRONMENT} is required"))?,
        );
        require_process_write_lease_parent(&write_lease_path)?;
        let (activation_credential, provider_credential) = inherited_authority_files()?;
        let provider_signer = open_service_identity_credential_reader::<
            GameCultProviderHealthIdentity,
        >(provider_credential)?;
        let activation_signer =
            IdunnRuntimeActivationSigner::from_credential_reader(activation_credential)?;
        Ok(Some(Self::new(
            odin_endpoint,
            connector_endpoint,
            observed_bound_endpoint,
            expected,
            activation,
            provider_signer,
            activation_signer,
            write_lease_path,
        )?))
    }

    #[allow(clippy::too_many_arguments)]
    fn new(
        odin_endpoint: SocketAddr,
        connector_endpoint: SocketAddr,
        observed_bound_endpoint: SocketAddr,
        expected: IdunnExpectedIncarnationRecord,
        activation: IdunnRuntimeActivationRecord,
        provider_signer: ServiceIdentitySigner<GameCultProviderHealthIdentity>,
        activation_signer: IdunnRuntimeActivationSigner,
        write_lease_path: PathBuf,
    ) -> Result<Self> {
        expected.validate()?;
        activation.validate()?;
        ensure!(
            expected.target == TARGET,
            "Expected belongs to another target"
        );
        ensure!(
            expected.health_contract == GHOSTLIGHT_RUNTIME_PRESENCE_HEALTH_CONTRACT,
            "Expected names another Ghostlight health contract"
        );
        ensure!(
            expected.state_schema_generation.as_deref() == Some(STATE_SCHEMA_GENERATION)
                && expected.state_contract_sha256.as_deref() == Some(STATE_CONTRACT_SHA256)
                && expected.write_lease_required,
            "Expected does not name Ghostlight's writable world-v2 state"
        );
        ensure!(
            activation.expected_projection_sha256 == expected.canonical_sha256()?
                && activation.runtime_id == expected.runtime_id,
            "activation does not bind the exact Expected incarnation"
        );
        ensure!(
            activation.activation_signer_identity_id == activation_signer.identity_id(),
            "activation credential does not belong to this launch"
        );
        ensure!(
            expected.expected_signer_identity_id == provider_signer.entry().identity_id,
            "provider credential is not Idunn's Expected signer"
        );
        ensure!(
            write_lease_path.is_absolute(),
            "write-lease path is not absolute"
        );

        let route = expected
            .route
            .as_ref()
            .context("Ghostlight Expected has no candidate route")?;
        ensure!(
            route.transport == "http",
            "Ghostlight candidate route is not HTTP"
        );
        let expected_bind: SocketAddr = route
            .candidate_endpoint
            .strip_prefix("http://")
            .context("Ghostlight candidate endpoint is not plain HTTP")?
            .parse()
            .context("Ghostlight candidate endpoint is not a socket address")?;
        ensure!(
            observed_bound_endpoint == expected_bind
                && route.candidate_endpoint == format!("http://{observed_bound_endpoint}"),
            "observed candidate bind differs from canonical Expected"
        );

        let capabilities = actual_capabilities();
        for requirement in &expected.capabilities {
            ensure!(
                capabilities.iter().any(|actual| {
                    actual.capability == requirement.capability
                        && actual.schema == requirement.schema
                        && actual.compatibility == requirement.compatibility
                        && actual.capacity >= requirement.minimum_capacity
                }),
                "Expected requires a capability this Ghostlight binary does not provide"
            );
        }
        let dependency_bindings =
            runtime_dependency_bindings(&expected, odin_endpoint, connector_endpoint)?;

        Ok(Self {
            endpoint: odin_endpoint,
            bound_endpoint: format!("http://{observed_bound_endpoint}"),
            expected,
            activation,
            provider_signer,
            activation_signer,
            capabilities,
            write_lease_path,
            dependency_bindings,
            publisher_sequence: 0,
            active_write_lease_sha256: None,
        })
    }

    pub(crate) fn runtime_id(&self) -> &str {
        &self.expected.runtime_id
    }

    pub(crate) fn dependency_bindings(&self) -> &RuntimeDependencyBindings {
        &self.dependency_bindings
    }

    pub(crate) fn publish_warming(&mut self) -> Result<PublishedRuntimePresence> {
        ensure!(
            self.active_write_lease_sha256.is_none(),
            "an active runtime cannot return to Warming"
        );
        self.publish("warming", "process-write-lease-pending", None)
    }

    pub(crate) async fn wait_for_write_lease(
        &mut self,
        warming: &PublishedRuntimePresence,
        timeout: Duration,
    ) -> Result<ProcessWriteLeaseGuard> {
        let deadline = Instant::now() + timeout;
        let mut next_heartbeat = Instant::now() + WARMING_HEARTBEAT_INTERVAL;
        let mut recent_warming_proofs = VecDeque::new();
        remember_warming_proof(&mut recent_warming_proofs, warming.canonical_sha256.clone());
        loop {
            if self.write_lease_path.exists() && sibling_lock_path(&self.write_lease_path).exists()
            {
                return ProcessWriteLeaseGuard::acquire_recent(
                    self.write_lease_path.clone(),
                    self.expected.clone(),
                    self.activation.clone(),
                    recent_warming_proofs.iter().cloned().collect(),
                );
            }
            let now = Instant::now();
            if now >= deadline {
                bail!(
                    "timed out waiting for process write lease {}",
                    self.write_lease_path.display()
                );
            }
            if now >= next_heartbeat {
                let heartbeat = self
                    .publish_warming()
                    .context("publishing Warming heartbeat while awaiting process lease")?;
                remember_warming_proof(&mut recent_warming_proofs, heartbeat.canonical_sha256);
                next_heartbeat = now + WARMING_HEARTBEAT_INTERVAL;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }

    pub(crate) fn publish_active(
        &mut self,
        lease: &ProcessWriteLeaseGuard,
    ) -> Result<PublishedRuntimePresence> {
        lease.require_current()?;
        let lease_sha256 = lease.canonical_sha256().to_owned();
        match &self.active_write_lease_sha256 {
            Some(current) => ensure!(
                current == &lease_sha256,
                "runtime attempted to replace its active write lease"
            ),
            None => self.active_write_lease_sha256 = Some(lease_sha256.clone()),
        }
        self.publish("active", "world-owner-serving", Some(lease_sha256))
    }

    pub(crate) fn republish_active(&mut self) -> Result<PublishedRuntimePresence> {
        let lease_sha256 = self
            .active_write_lease_sha256
            .clone()
            .context("runtime has not entered Active")?;
        self.publish("active", "world-owner-serving", Some(lease_sha256))
    }

    pub(crate) fn route_observation(
        &mut self,
        message_id: &str,
        ready: bool,
        lease: &ProcessWriteLeaseGuard,
    ) -> Result<CultNetMessage> {
        ensure!(
            !message_id.is_empty(),
            "route-observation message id is empty"
        );
        lease.require_current()?;
        let lease_sha256 = lease.canonical_sha256();
        ensure!(
            self.active_write_lease_sha256.as_deref() == Some(lease_sha256),
            "route observation does not carry the active process write lease"
        );
        let state = if ready { "active" } else { "degraded" };
        let detail = format!("route-observation:{message_id}");
        let record = self.signed_record(state, &detail, Some(lease_sha256.into()))?;
        let payload = canonical_presence_payload(&record)?;
        Ok(CultNetMessage::SnapshotResponseRaw {
            message_id: message_id.into(),
            documents: vec![runtime_presence_document(&record, payload, None)?],
        })
    }

    fn publish(
        &mut self,
        state: &str,
        detail: &str,
        write_lease_sha256: Option<String>,
    ) -> Result<PublishedRuntimePresence> {
        let record = self.signed_record(state, detail, write_lease_sha256)?;
        let canonical_sha256 = publish_runtime_presence(self.endpoint, &record)?;
        Ok(PublishedRuntimePresence { canonical_sha256 })
    }

    fn signed_record(
        &mut self,
        state: &str,
        detail: &str,
        write_lease_sha256: Option<String>,
    ) -> Result<GameCultRuntimePresenceHealthRecord> {
        self.publisher_sequence = self
            .publisher_sequence
            .checked_add(1)
            .ok_or_else(|| anyhow!("runtime presence sequence overflow"))?;
        let observed_at_unix_millis = u64::try_from(chrono::Utc::now().timestamp_millis())?;
        let mut record = GameCultRuntimePresenceHealthRecord {
            schema_version: GAMECULT_RUNTIME_PRESENCE_HEALTH_SCHEMA.into(),
            target: self.expected.target.clone(),
            expected_projection_sha256: self.expected.canonical_sha256()?,
            plan_id: self.expected.plan_id.clone(),
            incarnation_id: self.expected.incarnation_id.clone(),
            sealed_release_id: self.expected.sealed_release_id.clone(),
            activation_witness_sha256: self.activation.canonical_sha256()?,
            state_schema_generation: self.expected.state_schema_generation.clone(),
            state_contract_sha256: self.expected.state_contract_sha256.clone(),
            runtime_id: self.expected.runtime_id.clone(),
            runtime_instance_id: self.activation.runtime_instance_id.clone(),
            bound_endpoint: Some(self.bound_endpoint.clone()),
            capabilities: self.capabilities.clone(),
            health_contract: GHOSTLIGHT_RUNTIME_PRESENCE_HEALTH_CONTRACT.into(),
            state: state.into(),
            detail: detail.into(),
            write_lease_sha256,
            signer_identity_id: self.provider_signer.entry().identity_id.clone(),
            publisher_sequence: self.publisher_sequence,
            observed_at_unix_millis,
            signature_algorithm: "ed25519".into(),
            signature: Vec::new(),
            activation_signer_identity_id: self.activation_signer.identity_id(),
            activation_signature: Vec::new(),
        };
        let payload = record.canonical_proof_payload()?;
        let provider_proof = self
            .provider_signer
            .sign::<GameCultRuntimePresenceHealthPurpose>(&payload);
        ensure!(
            provider_proof.identity_id == record.signer_identity_id,
            "provider signer identity changed while publishing"
        );
        record.signature = provider_proof.signature;
        record.activation_signature = self.activation_signer.sign_presence_proof(&record)?;
        record.validate()?;
        Ok(record)
    }
}

fn runtime_dependency_bindings(
    expected: &IdunnExpectedIncarnationRecord,
    configured_odin: SocketAddr,
    configured_connector: SocketAddr,
) -> Result<RuntimeDependencyBindings> {
    ensure!(
        expected.dependencies.len() == 3,
        "Ghostlight Expected does not carry its exact three dependencies"
    );
    let odin = required_managed_dependency(expected, "shared-infrastructure", ODIN_CAPABILITY)?;
    let connector = required_managed_dependency(expected, "private", CONNECTOR_CAPABILITY)?;
    let heimdall = required_managed_dependency(expected, "required", HEIMDALL_CAPABILITY)?;

    let odin_endpoint = parse_dependency_socket_endpoint(
        odin.provider_endpoint
            .as_deref()
            .context("Expected Odin dependency has no endpoint")?,
        &["rudp://", "udp://"],
        "Odin dependency",
    )?;
    ensure!(
        odin_endpoint == configured_odin,
        "GHOSTLIGHT_ODIN_RUDP differs from Expected Odin"
    );
    let connector_endpoint = parse_dependency_socket_endpoint(
        connector
            .provider_endpoint
            .as_deref()
            .context("Expected Connector dependency has no endpoint")?,
        &["tcp://"],
        "Connector dependency",
    )?;
    ensure!(
        connector_endpoint == configured_connector,
        "GHOSTLIGHT_CONTROLLER_CONNECTOR differs from Expected Connector"
    );
    let heimdall_endpoint = parse_dependency_socket_endpoint(
        heimdall
            .provider_endpoint
            .as_deref()
            .context("Expected Heimdall dependency has no endpoint")?,
        &["rudp://", "udp://"],
        "Heimdall dependency",
    )?;

    Ok(RuntimeDependencyBindings {
        odin_rudp: odin_endpoint,
        connector: connector_endpoint,
        heimdall: HeimdallDependencyBinding {
            provider_id: heimdall
                .provider_id
                .clone()
                .context("Expected Heimdall dependency has no provider")?,
            endpoint: heimdall_endpoint,
        },
    })
}

fn required_managed_dependency<'a>(
    expected: &'a IdunnExpectedIncarnationRecord,
    kind: &str,
    identity: (&str, &str, &str),
) -> Result<&'a IdunnExpectedDependency> {
    let dependency = expected
        .dependencies
        .iter()
        .find(|dependency| {
            dependency.capability == identity.0
                && dependency.schema == identity.1
                && dependency.compatibility == identity.2
        })
        .with_context(|| format!("Expected omits dependency {}", identity.0))?;
    ensure!(
        dependency.kind == kind
            && dependency.startup == "before-promotion"
            && dependency.minimum_capacity > 0
            && dependency.provider_id.is_some()
            && dependency.provider_authority.as_deref() == Some("managed-incarnation")
            && dependency.provider_expected_projection_sha256.is_some(),
        "Expected dependency {} is unresolved or authority-incoherent",
        identity.0
    );
    Ok(dependency)
}

fn parse_dependency_socket_endpoint(
    endpoint: &str,
    allowed_schemes: &[&str],
    label: &str,
) -> Result<SocketAddr> {
    let matched_scheme = allowed_schemes
        .iter()
        .copied()
        .find(|scheme| endpoint.starts_with(scheme));
    let socket_text = match matched_scheme {
        Some(scheme) => &endpoint[scheme.len()..],
        None if !endpoint.contains("://") => endpoint,
        None => bail!("{label} uses an unsupported endpoint scheme"),
    };
    let socket: SocketAddr = socket_text
        .parse()
        .with_context(|| format!("{label} endpoint is not a socket address"))?;
    let canonical = matched_scheme
        .map(|scheme| format!("{scheme}{socket}"))
        .unwrap_or_else(|| socket.to_string());
    ensure!(endpoint == canonical, "{label} endpoint is not canonical");
    Ok(socket)
}

fn require_runtime_bundle(bundle: &Path) -> Result<()> {
    ensure!(
        bundle.is_absolute(),
        "Idunn runtime bundle path is not absolute"
    );
    #[cfg(target_os = "linux")]
    {
        let metadata = fs::symlink_metadata(bundle)
            .with_context(|| format!("inspecting Idunn runtime bundle {}", bundle.display()))?;
        ensure!(
            metadata.is_dir()
                && !metadata.file_type().is_symlink()
                && metadata.uid() == 0
                && metadata.gid() == 0
                && metadata.permissions().mode() & 0o022 == 0,
            "Idunn runtime bundle is not a root-owned service-read-only directory"
        );
        require_root_controlled_directory_chain(
            bundle
                .parent()
                .context("Idunn runtime bundle has no parent")?,
            "Idunn runtime bundle parent",
        )?;
        for path in [
            bundle.join("expected.cc"),
            sibling_lock_path(&bundle.join("expected.cc")),
            bundle.join("activation.cc"),
            sibling_lock_path(&bundle.join("activation.cc")),
        ] {
            require_root_read_only_file(&path, "Idunn runtime bundle document")?;
        }
    }
    Ok(())
}

fn require_process_write_lease_parent(path: &Path) -> Result<()> {
    ensure!(path.is_absolute(), "write-lease path is not absolute");
    #[cfg(target_os = "linux")]
    {
        let parent = path
            .parent()
            .context("process write-lease path has no parent")?;
        let metadata = fs::symlink_metadata(parent).with_context(|| {
            format!("inspecting process write-lease parent {}", parent.display())
        })?;
        ensure!(
            metadata.is_dir()
                && !metadata.file_type().is_symlink()
                && metadata.uid() == 0
                && metadata.permissions().mode() & 0o022 == 0,
            "process write-lease parent is not root-owned and service-nonwritable"
        );
        require_root_controlled_directory_chain(
            parent
                .parent()
                .context("process write-lease parent has no parent")?,
            "process write-lease ancestor",
        )?;
    }
    Ok(())
}

#[cfg(target_os = "linux")]
fn require_process_write_lease_custody(path: &Path) -> Result<()> {
    require_process_write_lease_parent(path)?;
    let parent = path
        .parent()
        .context("process write-lease path has no parent")?;
    let parent_metadata = fs::symlink_metadata(parent)?;
    for authority_file in [path.to_owned(), sibling_lock_path(path)] {
        let metadata = fs::symlink_metadata(&authority_file).with_context(|| {
            format!(
                "inspecting process write-lease authority {}",
                authority_file.display()
            )
        })?;
        ensure!(
            metadata.is_file()
                && !metadata.file_type().is_symlink()
                && metadata.uid() == 0
                && metadata.gid() == parent_metadata.gid()
                && metadata.permissions().mode() & 0o022 == 0
                && metadata.nlink() == 1,
            "process write-lease authority {} is not one root-owned service-nonwritable file",
            authority_file.display()
        );
    }
    Ok(())
}

#[cfg(not(target_os = "linux"))]
fn require_process_write_lease_custody(_path: &Path) -> Result<()> {
    Ok(())
}

#[cfg(target_os = "linux")]
fn require_root_read_only_file(path: &Path, label: &str) -> Result<()> {
    let metadata = fs::symlink_metadata(path)
        .with_context(|| format!("inspecting {label} {}", path.display()))?;
    ensure!(
        metadata.is_file()
            && !metadata.file_type().is_symlink()
            && metadata.uid() == 0
            && metadata.gid() == 0
            && metadata.permissions().mode() & 0o222 == 0
            && metadata.nlink() == 1,
        "{label} is not one root-owned read-only regular file"
    );
    Ok(())
}

#[cfg(target_os = "linux")]
fn require_root_controlled_directory_chain(path: &Path, label: &str) -> Result<()> {
    for directory in path.ancestors() {
        let metadata = fs::symlink_metadata(directory)
            .with_context(|| format!("inspecting {label} {}", directory.display()))?;
        ensure!(
            metadata.is_dir()
                && !metadata.file_type().is_symlink()
                && metadata.uid() == 0
                && metadata.permissions().mode() & 0o022 == 0,
            "{label} contains a non-root-controlled directory"
        );
    }
    Ok(())
}

#[cfg(target_os = "linux")]
fn inherited_authority_files() -> Result<(File, File)> {
    let listen_pid = std::env::var("LISTEN_PID").context("systemd did not bind authority FDs")?;
    let listen_fds = std::env::var("LISTEN_FDS").context("systemd did not bind authority FDs")?;
    let listen_fd_names =
        std::env::var("LISTEN_FDNAMES").context("systemd did not name authority FDs")?;
    authority_fd_layout(
        std::process::id(),
        &listen_pid,
        &listen_fds,
        &listen_fd_names,
    )?;
    ensure_descriptor_is_open(SYSTEMD_LISTEN_FDS_START, "runtime activation signer")?;
    ensure_descriptor_is_open(
        SYSTEMD_LISTEN_FDS_START + 1,
        "stable runtime-presence signer",
    )?;
    // SAFETY: the exact systemd LISTEN_* contract above assigns these two
    // distinct, verified-open descriptors to this PID. This function is their
    // first consumer and takes sole ownership of both.
    let activation = unsafe { File::from_raw_fd(SYSTEMD_LISTEN_FDS_START) };
    // SAFETY: same ownership proof; fd4 is distinct from fd3.
    let provider = unsafe { File::from_raw_fd(SYSTEMD_LISTEN_FDS_START + 1) };
    // OpenFile descriptors deliberately arrive without close-on-exec. Seal
    // both before inspecting either so even a rejected first credential
    // cannot leave the second signer inheritable on the error path.
    mark_descriptor_close_on_exec(&activation, "runtime activation signer")?;
    mark_descriptor_close_on_exec(&provider, "stable runtime-presence signer")?;
    protect_inherited_authority_file(&activation, Some(32), "runtime activation signer")?;
    protect_inherited_authority_file(&provider, None, "stable runtime-presence signer")?;
    Ok((activation, provider))
}

#[cfg(not(target_os = "linux"))]
fn inherited_authority_files() -> Result<(File, File)> {
    bail!("Idunn authority FDs require the managed Linux runtime")
}

#[cfg(target_os = "linux")]
fn authority_fd_layout(
    process_id: u32,
    listen_pid: &str,
    listen_fds: &str,
    listen_fd_names: &str,
) -> Result<()> {
    ensure!(
        listen_pid.parse::<u32>()? == process_id && listen_pid == process_id.to_string(),
        "systemd authority FDs belong to another process"
    );
    ensure!(
        listen_fds.parse::<usize>()? == 2 && listen_fds == "2",
        "systemd authority FD count is not exact"
    );
    let names = listen_fd_names.split(':').collect::<Vec<_>>();
    ensure!(
        names
            == [
                IDUNN_RUNTIME_ACTIVATION_CREDENTIAL_NAME,
                RUNTIME_PRESENCE_IDENTITY_FD_NAME,
            ],
        "systemd authority FD names or order differ from the launch contract"
    );
    Ok(())
}

#[cfg(target_os = "linux")]
fn ensure_descriptor_is_open(fd: RawFd, label: &str) -> Result<()> {
    // SAFETY: F_GETFD observes one integer descriptor and transfers no
    // ownership. Failure leaves the descriptor untouched.
    if unsafe { libc::fcntl(fd, libc::F_GETFD) } == -1 {
        return Err(std::io::Error::last_os_error())
            .with_context(|| format!("opening inherited {label} descriptor"));
    }
    Ok(())
}

#[cfg(target_os = "linux")]
fn protect_inherited_authority_file(
    file: &File,
    exact_size: Option<u64>,
    label: &str,
) -> Result<()> {
    let metadata = file.metadata()?;
    ensure!(
        metadata.is_file()
            && metadata.uid() == 0
            && metadata.gid() == 0
            && metadata.permissions().mode() & 0o777 == 0o400
            && metadata.nlink() == 1
            && exact_size.map_or(metadata.len() > 0, |size| metadata.len() == size),
        "inherited {label} descriptor is not one exact root-owned 0400 file"
    );
    // SAFETY: F_GETFL observes integer flags on the live descriptor owned by
    // `file` and transfers no ownership.
    let status_flags = unsafe { libc::fcntl(file.as_raw_fd(), libc::F_GETFL) };
    if status_flags == -1 {
        return Err(std::io::Error::last_os_error())
            .with_context(|| format!("reading inherited {label} access mode"));
    }
    ensure!(
        status_flags & libc::O_ACCMODE == libc::O_RDONLY,
        "inherited {label} descriptor is not read-only"
    );
    Ok(())
}

#[cfg(target_os = "linux")]
fn mark_descriptor_close_on_exec(file: &File, label: &str) -> Result<()> {
    let descriptor_flags = unsafe { libc::fcntl(file.as_raw_fd(), libc::F_GETFD) };
    if descriptor_flags == -1 {
        return Err(std::io::Error::last_os_error())
            .with_context(|| format!("reading inherited {label} descriptor flags"));
    }
    // SAFETY: F_SETFD mutates flags on the descriptor owned by `file`; it
    // transfers no ownership and dereferences no pointer.
    if unsafe {
        libc::fcntl(
            file.as_raw_fd(),
            libc::F_SETFD,
            descriptor_flags | libc::FD_CLOEXEC,
        )
    } == -1
    {
        return Err(std::io::Error::last_os_error())
            .with_context(|| format!("protecting inherited {label} from child inheritance"));
    }
    Ok(())
}

pub(crate) struct ProcessWriteLeaseGuard {
    path: PathBuf,
    record: IdunnProcessWriteLeaseRecord,
    canonical_sha256: String,
    enforce_custody: bool,
    release: Option<mpsc::Sender<()>>,
    worker: Option<thread::JoinHandle<()>>,
}

impl ProcessWriteLeaseGuard {
    fn acquire_recent(
        path: PathBuf,
        expected: IdunnExpectedIncarnationRecord,
        activation: IdunnRuntimeActivationRecord,
        warming_presence_sha256es: Vec<String>,
    ) -> Result<Self> {
        Self::acquire_inner(path, expected, activation, warming_presence_sha256es, true)
    }

    #[cfg(test)]
    fn acquire_fixture(
        path: PathBuf,
        expected: IdunnExpectedIncarnationRecord,
        activation: IdunnRuntimeActivationRecord,
        warming_presence_sha256: String,
    ) -> Result<Self> {
        Self::acquire_inner(
            path,
            expected,
            activation,
            vec![warming_presence_sha256],
            false,
        )
    }

    #[cfg(test)]
    fn acquire_recent_fixture(
        path: PathBuf,
        expected: IdunnExpectedIncarnationRecord,
        activation: IdunnRuntimeActivationRecord,
        warming_presence_sha256es: Vec<String>,
    ) -> Result<Self> {
        Self::acquire_inner(path, expected, activation, warming_presence_sha256es, false)
    }

    fn acquire_inner(
        path: PathBuf,
        expected: IdunnExpectedIncarnationRecord,
        activation: IdunnRuntimeActivationRecord,
        warming_presence_sha256es: Vec<String>,
        enforce_custody: bool,
    ) -> Result<Self> {
        ensure!(
            !warming_presence_sha256es.is_empty(),
            "process write lease has no provider-owned Warming proof"
        );
        if enforce_custody {
            require_process_write_lease_custody(&path)?;
        }
        let worker_path = path.clone();
        let (ready_tx, ready_rx) = mpsc::sync_channel(1);
        let (release_tx, release_rx) = mpsc::channel();
        let worker = thread::Builder::new()
            .name("ghostlight-write-lease".into())
            .spawn(move || {
                let store = SingleFileMessagePackBackingStore::new(&worker_path);
                let result = store.with_read_only_shared_snapshot(|entries| {
                    if enforce_custody {
                        require_process_write_lease_custody(&worker_path)?;
                    }
                    let record = decode_write_lease(
                        &entries,
                        &expected,
                        &activation,
                        &warming_presence_sha256es,
                    )?;
                    ready_tx
                        .send(Ok(record))
                        .map_err(|_| anyhow!("write-lease receiver stopped"))?;
                    let _ = release_rx.recv();
                    Ok(())
                });
                if let Err(error) = result {
                    let _ = ready_tx.send(Err(format!("{error:#}")));
                }
            })?;
        match ready_rx
            .recv()
            .context("write-lease custodian stopped before admission")?
        {
            Ok(record) => Ok(Self {
                canonical_sha256: record.canonical_sha256()?,
                path,
                record,
                enforce_custody,
                release: Some(release_tx),
                worker: Some(worker),
            }),
            Err(error) => {
                let _ = worker.join();
                bail!("process write lease was not admitted: {error}")
            }
        }
    }

    pub(crate) fn canonical_sha256(&self) -> &str {
        &self.canonical_sha256
    }

    pub(crate) fn require_current(&self) -> Result<()> {
        if self.enforce_custody {
            require_process_write_lease_custody(&self.path)?;
        }
        let current =
            SingleFileMessagePackBackingStore::new(&self.path).pull_all_read_only_snapshot()?;
        let [envelope] = current.as_slice() else {
            bail!("process write-lease store is absent or ambiguous");
        };
        ensure!(
            envelope.key == self.record.target
                && envelope.r#type == IdunnProcessWriteLeaseRecord::TYPE
                && envelope.schema_id.as_deref() == Some(IDUNN_PROCESS_WRITE_LEASE_SCHEMA)
                && IdunnProcessWriteLeaseRecord::decode_canonical(&envelope.payload)?
                    == self.record,
            "process write lease changed after admission"
        );
        Ok(())
    }
}

impl Drop for ProcessWriteLeaseGuard {
    fn drop(&mut self) {
        if let Some(release) = self.release.take() {
            let _ = release.send(());
        }
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

fn actual_capabilities() -> Vec<GameCultRuntimeCapability> {
    vec![
        GameCultRuntimeCapability {
            capability: "gamecult.eve.surface-provider".into(),
            schema: "gamecult.eve.surface.v1".into(),
            compatibility: "v1".into(),
            capacity: 1,
        },
        GameCultRuntimeCapability {
            capability: "ghostlight.world-service".into(),
            schema: STATE_SCHEMA.into(),
            compatibility: state_schema_compatibility_tag(),
            capacity: 1,
        },
    ]
}

fn read_expected(path: &Path) -> Result<IdunnExpectedIncarnationRecord> {
    let envelope = read_single_envelope(path)?;
    ensure!(
        envelope.r#type == IdunnExpectedIncarnationRecord::TYPE
            && envelope.schema_id.as_deref() == Some(IDUNN_EXPECTED_INCARNATION_SCHEMA),
        "runtime bundle Expected envelope has the wrong schema"
    );
    let expected = IdunnExpectedIncarnationRecord::decode_canonical(&envelope.payload)?;
    ensure!(
        envelope.key == expected.target,
        "Expected envelope key differs from target"
    );
    Ok(expected)
}

fn read_activation(path: &Path, target: &str) -> Result<IdunnRuntimeActivationRecord> {
    let envelope = read_single_envelope(path)?;
    ensure!(
        envelope.key == target
            && envelope.r#type == IdunnRuntimeActivationRecord::TYPE
            && envelope.schema_id.as_deref() == Some(IDUNN_RUNTIME_ACTIVATION_SCHEMA),
        "runtime bundle activation envelope has the wrong identity"
    );
    IdunnRuntimeActivationRecord::decode_canonical(&envelope.payload)
}

fn read_single_envelope(path: &Path) -> Result<CultCacheEnvelope> {
    let entries = SingleFileMessagePackBackingStore::new(path)
        .pull_all_read_only_snapshot()
        .with_context(|| format!("reading immutable runtime document {}", path.display()))?;
    let [envelope] = entries.as_slice() else {
        bail!("runtime document {} is absent or ambiguous", path.display());
    };
    Ok(envelope.clone())
}

fn decode_write_lease(
    entries: &[CultCacheEnvelope],
    expected: &IdunnExpectedIncarnationRecord,
    activation: &IdunnRuntimeActivationRecord,
    warming_presence_sha256es: &[String],
) -> Result<IdunnProcessWriteLeaseRecord> {
    let [envelope] = entries else {
        bail!("process write-lease store is absent or ambiguous");
    };
    ensure!(
        envelope.key == expected.target
            && envelope.r#type == IdunnProcessWriteLeaseRecord::TYPE
            && envelope.schema_id.as_deref() == Some(IDUNN_PROCESS_WRITE_LEASE_SCHEMA),
        "process write-lease envelope has the wrong identity"
    );
    let lease = IdunnProcessWriteLeaseRecord::decode_canonical(&envelope.payload)?;
    ensure!(
        lease.target == expected.target
            && lease.expected_projection_sha256 == expected.canonical_sha256()?
            && lease.plan_id == expected.plan_id
            && lease.incarnation_id == expected.incarnation_id
            && lease.sealed_release_id == expected.sealed_release_id
            && lease.activation_witness_sha256 == activation.canonical_sha256()?
            && lease.state_schema_generation
                == expected
                    .state_schema_generation
                    .as_deref()
                    .context("Expected has no state schema generation")?
            && lease.state_contract_sha256
                == expected
                    .state_contract_sha256
                    .as_deref()
                    .context("Expected has no state contract")?
            && lease.runtime_id == expected.runtime_id
            && lease.runtime_instance_id == activation.runtime_instance_id
            && warming_presence_sha256es
                .iter()
                .any(|sha256| sha256 == &lease.warming_presence_sha256),
        "process write lease does not bind a provider-owned Warming proof for this incarnation"
    );
    Ok(lease)
}

fn remember_warming_proof(proofs: &mut VecDeque<String>, sha256: String) {
    proofs.push_back(sha256);
    while proofs.len() > MAX_RECENT_WARMING_PROOFS {
        proofs.pop_front();
    }
}

fn publish_runtime_presence(
    endpoint: SocketAddr,
    record: &GameCultRuntimePresenceHealthRecord,
) -> Result<String> {
    record.validate()?;
    let payload = canonical_presence_payload(record)?;
    let canonical_sha256 = record.canonical_sha256()?;
    let message = CultNetMessage::DocumentPutRaw {
        message_id: format!(
            "runtime-presence:{}:{}:{}",
            record.target, record.runtime_instance_id, record.publisher_sequence
        ),
        document: runtime_presence_document(
            record,
            payload,
            Some(vec![
                CULTNET_RUDP_PROTOCOL_ID.into(),
                "runtime-presence".into(),
            ]),
        )?,
    };
    let bind = if endpoint.is_ipv4() {
        "0.0.0.0:0"
    } else {
        "[::]:0"
    };
    let socket = UdpSocket::bind(bind)
        .with_context(|| format!("binding Ghostlight CultNet sender at {bind}"))?;
    socket.set_read_timeout(Some(Duration::from_millis(100)))?;
    let mut transport =
        CultNetRudpSocketTransportConnection::new(CultNetRudpSocketTransportOptions::client(
            &record.runtime_id,
            socket,
            endpoint,
            ODIN_CULTMESH_CATALOG_CONNECTION_ID,
        ))?;
    transport.connect(Vec::new())?;
    let deadline = Instant::now() + Duration::from_millis(500);
    while !transport.connected() {
        let _ = transport.receive_once()?;
        transport.poll_resends()?;
        if Instant::now() >= deadline {
            return Err(anyhow!(
                "timed out connecting runtime-presence publisher to {endpoint}"
            ));
        }
    }
    let receipt = transport.send_reliable(
        "schema",
        encode_cultnet_message_to_vec(&message, CultNetWireContract::CultNetSchemaV0)?,
    )?;
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        match transport.reliable_send_status(&receipt) {
            CultNetRudpReliableSendStatus::Acknowledged => break,
            CultNetRudpReliableSendStatus::Invalidated => {
                bail!("runtime-presence acknowledgement was invalidated")
            }
            CultNetRudpReliableSendStatus::Pending => {}
        }
        let _ = transport.receive_once()?;
        transport.poll_resends()?;
        if Instant::now() >= deadline {
            bail!("timed out awaiting exact runtime-presence acknowledgement from {endpoint}");
        }
    }
    Ok(canonical_sha256)
}

fn canonical_presence_payload(record: &GameCultRuntimePresenceHealthRecord) -> Result<Vec<u8>> {
    let payload = rmp_serde::to_vec(record).context("encoding canonical runtime presence")?;
    let decoded: GameCultRuntimePresenceHealthRecord = rmp_serde::from_slice(&payload)?;
    ensure!(
        decoded == *record && rmp_serde::to_vec(&decoded)? == payload,
        "runtime presence encoding is noncanonical"
    );
    Ok(payload)
}

fn runtime_presence_document(
    record: &GameCultRuntimePresenceHealthRecord,
    payload: Vec<u8>,
    tags: Option<Vec<String>>,
) -> Result<CultNetRawDocumentRecord> {
    Ok(CultNetRawDocumentRecord {
        schema_id: GAMECULT_RUNTIME_PRESENCE_HEALTH_SCHEMA.into(),
        record_key: record.target.clone(),
        stored_at: chrono::DateTime::from_timestamp_millis(
            record.observed_at_unix_millis.try_into()?,
        )
        .context("runtime presence timestamp is invalid")?
        .to_rfc3339(),
        payload_encoding: CultNetRawPayloadEncoding::Messagepack,
        payload,
        source_runtime_id: Some(record.runtime_id.clone()),
        source_agent_id: Some(record.signer_identity_id.clone()),
        source_role: Some("runtime-presence-publisher".into()),
        tags,
    })
}

fn sibling_lock_path(path: &Path) -> PathBuf {
    let mut name = path
        .file_name()
        .map(|value| value.to_os_string())
        .unwrap_or_else(|| "authority.cc".into());
    name.push(".lock");
    path.with_file_name(name)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use cultnet_rs::{
        GameCultRuntimePresenceHealthPurpose, IdunnExpectedCapability, IdunnExpectedRoute,
        IdunnRuntimeActivationLaunch, IdunnServiceIdentity, RuntimePresenceAuthenticationContext,
        authenticate_runtime_presence_claim, enroll_service_identity_at, verify_runtime_authority,
    };

    fn digest(byte: char) -> String {
        format!("sha256-{}", byte.to_string().repeat(64))
    }

    struct Fixture {
        publisher: RuntimePresencePublisher,
        authority: cultnet_rs::VerifiedRuntimeAuthority,
        expected: IdunnExpectedIncarnationRecord,
        activation: IdunnRuntimeActivationRecord,
    }

    pub(crate) struct RouteObservationFixture {
        pub(crate) publisher: RuntimePresencePublisher,
        pub(crate) write_lease: ProcessWriteLeaseGuard,
        pub(crate) authority: cultnet_rs::VerifiedRuntimeAuthority,
    }

    fn managed_dependency(
        kind: &str,
        capability: &str,
        schema: &str,
        compatibility: &str,
        provider_id: &str,
        endpoint: &str,
    ) -> IdunnExpectedDependency {
        IdunnExpectedDependency {
            kind: kind.into(),
            capability: capability.into(),
            schema: schema.into(),
            compatibility: compatibility.into(),
            minimum_capacity: 1,
            startup: "before-promotion".into(),
            provider_id: Some(provider_id.into()),
            provider_authority: Some("managed-incarnation".into()),
            provider_expected_projection_sha256: Some(digest('a')),
            provider_endpoint: Some(endpoint.into()),
        }
    }

    fn fixture_with_state_contract(root: &Path, state_contract_sha256: &str) -> Result<Fixture> {
        let provider_path = root.join("provider.cc");
        let provider =
            enroll_service_identity_at::<GameCultProviderHealthIdentity>(&provider_path)?;
        let provider_public_key = provider.entry().public_key.clone();
        let idunn = enroll_service_identity_at::<IdunnServiceIdentity>(&root.join("idunn.cc"))?;
        let expected = IdunnExpectedIncarnationRecord {
            schema_version: IDUNN_EXPECTED_INCARNATION_SCHEMA.into(),
            target: TARGET.into(),
            plan_id: digest('1'),
            incarnation_id: "ghostlight-incarnation-1".into(),
            sealed_release_id: digest('2'),
            source_repository: "github.com/GameCult/Ghostlight".into(),
            source_revision: "3".repeat(40),
            recipe_sha256: digest('4'),
            runtime_id: "ghostlight-yggdrasil".into(),
            expected_signer_identity_id: provider.entry().identity_id.clone(),
            health_contract: GHOSTLIGHT_RUNTIME_PRESENCE_HEALTH_CONTRACT.into(),
            artifact_sha256: digest('5'),
            state_schema_generation: Some(STATE_SCHEMA_GENERATION.into()),
            state_contract_sha256: Some(state_contract_sha256.into()),
            write_lease_required: true,
            route: Some(IdunnExpectedRoute {
                route_id: "ghostlight-http".into(),
                transport: "http".into(),
                stable_endpoint: "https://ghostlight.gamecult.org/".into(),
                candidate_endpoint: "http://127.0.0.1:18831".into(),
            }),
            capabilities: vec![
                IdunnExpectedCapability {
                    capability: "gamecult.eve.surface-provider".into(),
                    schema: "gamecult.eve.surface.v1".into(),
                    compatibility: "v1".into(),
                    minimum_capacity: 1,
                },
                IdunnExpectedCapability {
                    capability: "ghostlight.world-service".into(),
                    schema: STATE_SCHEMA.into(),
                    compatibility: state_schema_compatibility_tag(),
                    minimum_capacity: 1,
                },
            ],
            dependencies: vec![
                managed_dependency(
                    "private",
                    CONNECTOR_CAPABILITY.0,
                    CONNECTOR_CAPABILITY.1,
                    CONNECTOR_CAPABILITY.2,
                    "connector-yggdrasil",
                    "tcp://127.0.0.1:4103",
                ),
                managed_dependency(
                    "required",
                    HEIMDALL_CAPABILITY.0,
                    HEIMDALL_CAPABILITY.1,
                    HEIMDALL_CAPABILITY.2,
                    "heimdall-yggdrasil",
                    "rudp://127.0.0.1:4101",
                ),
                managed_dependency(
                    "shared-infrastructure",
                    ODIN_CAPABILITY.0,
                    ODIN_CAPABILITY.1,
                    ODIN_CAPABILITY.2,
                    "odin-yggdrasil",
                    "rudp://127.0.0.1:9",
                ),
            ],
        };
        let launch = IdunnRuntimeActivationLaunch::issue(&expected, digest('7'), 100, &idunn)?;
        let activation = launch.activation().clone();
        let mut activation_credential = Vec::new();
        launch.write_credential(&mut activation_credential)?;
        let activation_signer =
            IdunnRuntimeActivationSigner::from_credential_reader(&activation_credential[..])?;
        let provider_signer = open_service_identity_credential_reader::<
            GameCultProviderHealthIdentity,
        >(File::open(&provider_path)?)?;
        let authority = verify_runtime_authority(
            &expected,
            &activation,
            &idunn.trust_anchor()?,
            &provider_public_key,
        )?;
        let publisher = RuntimePresencePublisher::new(
            "127.0.0.1:9".parse()?,
            "127.0.0.1:4103".parse()?,
            "127.0.0.1:18831".parse()?,
            expected.clone(),
            activation.clone(),
            provider_signer,
            activation_signer,
            root.join("write-lease.cc"),
        )?;
        Ok(Fixture {
            publisher,
            authority,
            expected,
            activation,
        })
    }

    fn fixture(root: &Path) -> Result<Fixture> {
        fixture_with_state_contract(root, STATE_CONTRACT_SHA256)
    }

    pub(crate) fn route_observation_fixture(root: &Path) -> Result<RouteObservationFixture> {
        let mut fixture = fixture(root)?;
        let warming =
            fixture
                .publisher
                .signed_record("warming", "process-write-lease-pending", None)?;
        let warming_sha256 = warming.canonical_sha256()?;
        let lease = IdunnProcessWriteLeaseRecord {
            schema_version: IDUNN_PROCESS_WRITE_LEASE_SCHEMA.into(),
            target: fixture.expected.target.clone(),
            expected_projection_sha256: fixture.expected.canonical_sha256()?,
            plan_id: fixture.expected.plan_id.clone(),
            incarnation_id: fixture.expected.incarnation_id.clone(),
            sealed_release_id: fixture.expected.sealed_release_id.clone(),
            activation_witness_sha256: fixture.activation.canonical_sha256()?,
            state_schema_generation: STATE_SCHEMA_GENERATION.into(),
            state_contract_sha256: fixture.expected.state_contract_sha256.clone().unwrap(),
            runtime_id: fixture.expected.runtime_id.clone(),
            runtime_instance_id: fixture.activation.runtime_instance_id.clone(),
            warming_presence_sha256: warming_sha256.clone(),
            lease_epoch: 1,
            issued_at_unix_millis: 200,
        };
        let path = root.join("write-lease.cc");
        SingleFileMessagePackBackingStore::new(&path).push(&CultCacheEnvelope {
            key: lease.target.clone(),
            r#type: IdunnProcessWriteLeaseRecord::TYPE.into(),
            payload: lease.canonical_bytes()?,
            stored_at: "2026-09-03T00:00:00Z".into(),
            schema_id: Some(IDUNN_PROCESS_WRITE_LEASE_SCHEMA.into()),
        })?;
        let write_lease = ProcessWriteLeaseGuard::acquire_fixture(
            path,
            fixture.expected,
            fixture.activation,
            warming_sha256,
        )?;
        fixture.publisher.active_write_lease_sha256 = Some(write_lease.canonical_sha256().into());
        Ok(RouteObservationFixture {
            publisher: fixture.publisher,
            write_lease,
            authority: fixture.authority,
        })
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn inherited_authority_fd_layout_rejects_substitution_and_extras() -> Result<()> {
        let pid = std::process::id();
        let names = format!(
            "{IDUNN_RUNTIME_ACTIVATION_CREDENTIAL_NAME}:{RUNTIME_PRESENCE_IDENTITY_FD_NAME}"
        );
        authority_fd_layout(pid, &pid.to_string(), "2", &names)?;
        assert!(authority_fd_layout(pid, &(pid + 1).to_string(), "2", &names).is_err());
        assert!(authority_fd_layout(pid, &format!("0{pid}"), "2", &names).is_err());
        assert!(authority_fd_layout(pid, &pid.to_string(), "02", &names).is_err());
        assert!(authority_fd_layout(pid, &pid.to_string(), "3", &names).is_err());
        assert!(
            authority_fd_layout(
                pid,
                &pid.to_string(),
                "2",
                &format!(
                    "{RUNTIME_PRESENCE_IDENTITY_FD_NAME}:{IDUNN_RUNTIME_ACTIVATION_CREDENTIAL_NAME}"
                ),
            )
            .is_err()
        );
        assert!(
            authority_fd_layout(
                pid,
                &pid.to_string(),
                "2",
                &format!(
                    "{IDUNN_RUNTIME_ACTIVATION_CREDENTIAL_NAME}:{IDUNN_RUNTIME_ACTIVATION_CREDENTIAL_NAME}"
                ),
            )
            .is_err()
        );
        Ok(())
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn close_on_exec_authority_descriptor_is_not_visible_to_a_child() -> Result<()> {
        use std::{os::fd::AsRawFd, process::Command};

        let file = tempfile::tempfile()?;
        mark_descriptor_close_on_exec(&file, "test signer")?;
        let descriptor = file.as_raw_fd();
        let status = Command::new("/bin/sh")
            .arg("-c")
            .arg(format!("test ! -e /proc/self/fd/{descriptor}"))
            .status()?;
        ensure!(status.success(), "child inherited the signer descriptor");
        Ok(())
    }

    #[test]
    fn runtime_rejects_an_expected_projection_for_another_state_contract() {
        let root = tempfile::tempdir().unwrap();
        let error = match fixture_with_state_contract(root.path(), &digest('6')) {
            Ok(_) => panic!("another state contract was admitted"),
            Err(error) => error,
        };
        assert!(
            error
                .to_string()
                .contains("does not name Ghostlight's writable world-v2 state")
        );
    }

    #[test]
    fn warming_presence_is_dual_signed_and_reports_compiled_capabilities() -> Result<()> {
        let root = tempfile::tempdir()?;
        let mut fixture = fixture(root.path())?;
        let record =
            fixture
                .publisher
                .signed_record("warming", "process-write-lease-pending", None)?;
        let bytes = rmp_serde::to_vec(&record)?;
        let claim = authenticate_runtime_presence_claim(
            &bytes,
            &fixture.authority,
            RuntimePresenceAuthenticationContext {
                trusted_received_at_unix_millis: record.observed_at_unix_millis,
                maximum_age_millis: 1_000,
                maximum_future_skew_millis: 10,
            },
        )?;
        assert_eq!(claim.record(), &record);
        assert_eq!(record.capabilities, actual_capabilities());
        assert_eq!(
            record.bound_endpoint.as_deref(),
            Some("http://127.0.0.1:18831")
        );
        assert!(record.write_lease_sha256.is_none());

        let wrong_purpose = fixture
            .publisher
            .provider_signer
            .sign::<GameCultRuntimePresenceHealthPurpose>(b"another-record");
        assert_ne!(wrong_purpose.signature, record.signature);
        Ok(())
    }

    #[test]
    fn dependency_endpoints_fail_closed_without_scheme_coercion() -> Result<()> {
        assert_eq!(
            parse_dependency_socket_endpoint(
                "rudp://127.0.0.1:4100",
                &["rudp://", "udp://"],
                "Odin dependency",
            )?,
            "127.0.0.1:4100".parse::<SocketAddr>()?
        );
        assert!(
            parse_dependency_socket_endpoint(
                "tcp://127.0.0.1:4100",
                &["rudp://", "udp://"],
                "Odin dependency",
            )
            .is_err()
        );
        assert!(
            parse_dependency_socket_endpoint(
                "http://127.0.0.1:4101",
                &["rudp://", "udp://"],
                "Heimdall dependency",
            )
            .is_err()
        );
        Ok(())
    }

    #[test]
    fn process_write_lease_guard_holds_and_rechecks_the_exact_incarnation() -> Result<()> {
        let root = tempfile::tempdir()?;
        let mut fixture = fixture(root.path())?;
        let first_warming =
            fixture
                .publisher
                .signed_record("warming", "process-write-lease-pending", None)?;
        let first_warming_sha256 = first_warming.canonical_sha256()?;
        let latest_warming =
            fixture
                .publisher
                .signed_record("warming", "process-write-lease-pending", None)?;
        let latest_warming_sha256 = latest_warming.canonical_sha256()?;
        assert!(latest_warming.publisher_sequence > first_warming.publisher_sequence);
        assert_ne!(latest_warming_sha256, first_warming_sha256);
        let lease = IdunnProcessWriteLeaseRecord {
            schema_version: IDUNN_PROCESS_WRITE_LEASE_SCHEMA.into(),
            target: fixture.expected.target.clone(),
            expected_projection_sha256: fixture.expected.canonical_sha256()?,
            plan_id: fixture.expected.plan_id.clone(),
            incarnation_id: fixture.expected.incarnation_id.clone(),
            sealed_release_id: fixture.expected.sealed_release_id.clone(),
            activation_witness_sha256: fixture.activation.canonical_sha256()?,
            state_schema_generation: STATE_SCHEMA_GENERATION.into(),
            state_contract_sha256: fixture.expected.state_contract_sha256.clone().unwrap(),
            runtime_id: fixture.expected.runtime_id.clone(),
            runtime_instance_id: fixture.activation.runtime_instance_id.clone(),
            warming_presence_sha256: first_warming_sha256.clone(),
            lease_epoch: 1,
            issued_at_unix_millis: 200,
        };
        let path = root.path().join("write-lease.cc");
        SingleFileMessagePackBackingStore::new(&path).push(&CultCacheEnvelope {
            key: lease.target.clone(),
            r#type: IdunnProcessWriteLeaseRecord::TYPE.into(),
            payload: lease.canonical_bytes()?,
            stored_at: "2026-09-03T00:00:00Z".into(),
            schema_id: Some(IDUNN_PROCESS_WRITE_LEASE_SCHEMA.into()),
        })?;

        let guard = ProcessWriteLeaseGuard::acquire_recent_fixture(
            path.clone(),
            fixture.expected.clone(),
            fixture.activation.clone(),
            vec![first_warming_sha256, latest_warming_sha256],
        )?;
        guard.require_current()?;
        assert_eq!(guard.canonical_sha256(), lease.canonical_sha256()?);

        let lease_sha256 = lease.canonical_sha256()?;
        fixture.publisher.active_write_lease_sha256 = Some(lease_sha256.clone());
        let response = fixture
            .publisher
            .route_observation("challenge-31", true, &guard)?;
        let CultNetMessage::SnapshotResponseRaw {
            message_id,
            documents,
        } = response
        else {
            panic!("route observation was not a raw snapshot response");
        };
        assert_eq!(message_id, "challenge-31");
        let [document] = documents.as_slice() else {
            panic!("route observation did not contain exactly one document");
        };
        assert_eq!(document.schema_id, GAMECULT_RUNTIME_PRESENCE_HEALTH_SCHEMA);
        assert_eq!(document.record_key, TARGET);
        assert_eq!(document.tags, None);
        let presence: GameCultRuntimePresenceHealthRecord =
            rmp_serde::from_slice(&document.payload)?;
        assert_eq!(presence.state, "active");
        assert_eq!(presence.detail, "route-observation:challenge-31");
        assert_eq!(
            presence.write_lease_sha256.as_deref(),
            Some(lease_sha256.as_str())
        );
        let claim = authenticate_runtime_presence_claim(
            &document.payload,
            &fixture.authority,
            RuntimePresenceAuthenticationContext {
                trusted_received_at_unix_millis: presence.observed_at_unix_millis,
                maximum_age_millis: 1_000,
                maximum_future_skew_millis: 10,
            },
        )?;
        assert_eq!(claim.record(), &presence);

        let degraded = fixture
            .publisher
            .route_observation("challenge-32", false, &guard)?;
        let CultNetMessage::SnapshotResponseRaw { documents, .. } = degraded else {
            panic!("degraded route observation was not a raw snapshot response");
        };
        let degraded: GameCultRuntimePresenceHealthRecord =
            rmp_serde::from_slice(&documents[0].payload)?;
        assert_eq!(degraded.state, "degraded");
        assert_eq!(degraded.detail, "route-observation:challenge-32");
        drop(guard);

        let mut wrong = lease;
        wrong.runtime_instance_id = digest('8');
        let entries = SingleFileMessagePackBackingStore::new(&path).pull_all()?;
        let store = SingleFileMessagePackBackingStore::new(&path);
        ensure!(
            store.replace_and_append_if_snapshot_unchanged(
                &entries,
                vec![CultCacheEnvelope {
                    key: wrong.target.clone(),
                    r#type: IdunnProcessWriteLeaseRecord::TYPE.into(),
                    payload: wrong.canonical_bytes()?,
                    stored_at: "2026-09-03T00:00:01Z".into(),
                    schema_id: Some(IDUNN_PROCESS_WRITE_LEASE_SCHEMA.into()),
                }],
            )?,
            "test lease replacement lost CAS"
        );
        assert!(
            ProcessWriteLeaseGuard::acquire_fixture(
                path,
                fixture.expected,
                fixture.activation,
                digest('9'),
            )
            .is_err()
        );
        Ok(())
    }
}
