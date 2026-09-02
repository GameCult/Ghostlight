use anyhow::{Context, Result, anyhow, bail};
use cultcache_rs::{CultCacheEnvelope, SingleFileMessagePackBackingStore};
use cultnet_rs::{
    CultNetMessage, CultNetRawDocumentRecord, CultNetRawPayloadEncoding,
    CultNetRudpSocketTransportConnection, CultNetRudpSocketTransportOptions, CultNetWireContract,
    GameCultProviderHealthIdentity, IdunnSignedDaemonHealthPurpose, ServiceIdentitySigner,
    encode_cultnet_message_to_vec, open_service_identity_at,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
#[cfg(target_os = "linux")]
use std::os::unix::fs::MetadataExt;
use std::{
    net::{SocketAddr, UdpSocket},
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

pub const GHOSTLIGHT_IDUNN_HEALTH_CONTRACT: &str = "ghostlight.cultnet-rudp-service-health";
const CULTNET_RUDP_PROTOCOL_ID: &str = "cultnet.transport.rudp.v0";
const IDUNN_HEALTH_RUDP_CONNECTION_ID: u32 = 0x1d0d_0001;
const GHOSTLIGHT_ACTIVATION_SCHEMA: &str = "gamecult.ghostlight.activation.v1";
const GHOSTLIGHT_ACTIVATION_PATH: &str = "/etc/gamecult/ghostlight-dungeon/runtime/ACTIVATION";
const GHOSTLIGHT_UNIT_PATH: &str = "/etc/systemd/system/ghostlight-dungeon.service";
const GHOSTLIGHT_RUNTIME_ENVIRONMENT_PATH: &str = "/srv/ghostlight/deploy/runtime.env";
const TRAFFIC_ADMISSION_PATH: &str =
    "/etc/gamecult/ghostlight-dungeon/runtime/traffic-admission.cc";
const TRAFFIC_ADMISSION_TYPE: &str = "idunn.runtime_traffic_admission";
const TRAFFIC_ADMISSION_SCHEMA: &str = "idunn.runtime_traffic_admission.v2";
const TRAFFIC_ADMISSION_KEY: &str = "yggdrasil-ghostlight";

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct IdunnReleaseBinding {
    pub(crate) release_id: String,
    pub(crate) release_witness_sha256: String,
    pub(crate) source_commit: String,
    pub(crate) deployment_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct IdunnActivationBinding {
    activation_witness_sha256: String,
}

// Idunn owns and writes this CultCache schema. Ghostlight carries only the
// exact read-side projection required to consume that root traffic grant.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct TrafficAdmissionConsumerProjection {
    schema_version: String,
    daemon_id: String,
    release_id: String,
    release_witness_sha256: String,
    source_commit: String,
    deployment_id: String,
    activation_witness_sha256: String,
    signed_health_sha256: String,
    publisher_incarnation_id: String,
    publisher_sequence: u64,
    signer_identity_id: String,
    runtime_process_id: u32,
    runtime_process_starttime_ticks: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct RuntimeProcessInstanceBinding {
    process_id: u32,
    starttime_ticks: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ActivationWitness {
    activation_id: String,
    daemon_id: String,
    release_id: String,
    release_witness_sha256: String,
    source_commit: String,
    deployment_id: String,
    unit_path: String,
    unit_sha256: String,
    runtime_environment_path: String,
    runtime_environment_sha256: String,
}

#[derive(Clone)]
pub(crate) struct GhostlightTrafficAdmissionGate {
    path: PathBuf,
    expected: TrafficAdmissionConsumerProjection,
    ghostlight_gid: u32,
}

pub(crate) struct PublishedHealthStatementIdentity {
    daemon_id: String,
    signed_health_sha256: String,
    publisher_incarnation_id: String,
    publisher_sequence: u64,
    signer_identity_id: String,
}

impl IdunnReleaseBinding {
    fn validate(&self) -> Result<()> {
        require_id(&self.release_id, "release id")?;
        require_id(&self.deployment_id, "deployment id")?;
        if !self
            .release_witness_sha256
            .strip_prefix("sha256-")
            .is_some_and(|digest| is_lower_hex(digest, 64))
            || !is_lower_hex(&self.source_commit, 40)
        {
            bail!("release health binding is malformed");
        }
        Ok(())
    }
}

impl IdunnActivationBinding {
    fn validate(&self) -> Result<()> {
        require_sha256(&self.activation_witness_sha256, "activation witness digest")
    }
}

impl GhostlightTrafficAdmissionGate {
    pub(crate) fn from_environment(
        release: &IdunnReleaseBinding,
        activation: &IdunnActivationBinding,
        published: &PublishedHealthStatementIdentity,
    ) -> Result<Self> {
        #[cfg(not(target_os = "linux"))]
        bail!("signed Ghostlight health and traffic admission require Linux");

        let raw_path = std::env::var("GHOSTLIGHT_TRAFFIC_ADMISSION")
            .context("GHOSTLIGHT_TRAFFIC_ADMISSION is required for signed health")?;
        if raw_path != TRAFFIC_ADMISSION_PATH {
            bail!("Ghostlight traffic admission path is not the fixed root policy path");
        }
        Ok(Self {
            path: PathBuf::from(raw_path),
            expected: traffic_admission_expectation(release, activation, published)?,
            ghostlight_gid: local_group_id("ghostlight")?,
        })
    }

    pub(crate) async fn wait_until_granted(&self, timeout: Duration) -> Result<()> {
        let started = tokio::time::Instant::now();
        let mut last_invalid_observation = None;
        loop {
            match self.is_current() {
                Ok(true) => return Ok(()),
                Ok(false) => last_invalid_observation = None,
                Err(error) => last_invalid_observation = Some(format!("{error:#}")),
            }
            if started.elapsed() >= timeout {
                let detail = last_invalid_observation
                    .as_deref()
                    .unwrap_or("the fixed admission record remained absent");
                bail!(
                    "timed out waiting for exact sealed root traffic admission {}: {}",
                    self.path.display(),
                    detail,
                );
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }

    pub(crate) fn require_current(&self) -> Result<()> {
        if !self.is_current()? {
            bail!(
                "root traffic admission disappeared from {}",
                self.path.display()
            );
        }
        Ok(())
    }

    fn is_current(&self) -> Result<bool> {
        match std::fs::symlink_metadata(&self.path) {
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
            Err(error) => {
                return Err(error).with_context(|| {
                    format!(
                        "inspecting Ghostlight traffic admission {}",
                        self.path.display()
                    )
                });
            }
        }
        require_sealed_traffic_admission_files(&self.path, self.ghostlight_gid)?;
        SingleFileMessagePackBackingStore::new(&self.path).with_read_only_shared_snapshot(
            |entries| {
                require_sealed_traffic_admission_files(&self.path, self.ghostlight_gid)?;
                let admitted = decode_traffic_admission_entries(&entries)?;
                require_exact_traffic_admission(&admitted, &self.expected)
            },
        )?;
        Ok(true)
    }
}

pub(crate) fn active_activation_binding(
    release: &IdunnReleaseBinding,
) -> Result<IdunnActivationBinding> {
    #[cfg(not(target_os = "linux"))]
    bail!("signed Ghostlight health activation requires Linux");

    release.validate()?;
    let activation_path = Path::new(GHOSTLIGHT_ACTIVATION_PATH);
    let unit_path = Path::new(GHOSTLIGHT_UNIT_PATH);
    let runtime_environment_path = Path::new(GHOSTLIGHT_RUNTIME_ENVIRONMENT_PATH);
    let activation_bytes = read_root_controlled_file(activation_path, "Ghostlight activation")?;
    let unit_bytes = read_root_controlled_file(unit_path, "Ghostlight systemd unit")?;
    let runtime_environment_bytes =
        read_root_controlled_file(runtime_environment_path, "Ghostlight runtime environment")?;
    let witness = parse_activation_witness(&activation_bytes)?;
    let activation_id = std::env::var("GHOSTLIGHT_ACTIVATION_ID")
        .context("GHOSTLIGHT_ACTIVATION_ID is required for signed health")?;
    require_safe_token(&activation_id, "activation id")?;
    let expected = ActivationWitness {
        activation_id,
        daemon_id: TRAFFIC_ADMISSION_KEY.into(),
        release_id: release.release_id.clone(),
        release_witness_sha256: release.release_witness_sha256.clone(),
        source_commit: release.source_commit.clone(),
        deployment_id: release.deployment_id.clone(),
        unit_path: GHOSTLIGHT_UNIT_PATH.into(),
        unit_sha256: format!("sha256-{}", hex(&Sha256::digest(&unit_bytes))),
        runtime_environment_path: GHOSTLIGHT_RUNTIME_ENVIRONMENT_PATH.into(),
        runtime_environment_sha256: format!(
            "sha256-{}",
            hex(&Sha256::digest(&runtime_environment_bytes))
        ),
    };
    validate_activation_witness(&witness)?;
    if witness != expected {
        bail!("activation witness does not bind the exact Ghostlight launch inputs");
    }
    let binding = IdunnActivationBinding {
        activation_witness_sha256: format!("sha256-{}", hex(&Sha256::digest(&activation_bytes))),
    };
    binding.validate()?;
    Ok(binding)
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct IdunnSignedDaemonHealthRecord {
    schema_version: String,
    daemon_id: String,
    health_contract: String,
    source_runtime_id: String,
    state: String,
    detail: String,
    signer_identity_id: String,
    publisher_incarnation_id: String,
    publisher_sequence: u64,
    observed_at_unix_millis: u64,
    release_id: Option<String>,
    release_witness_sha256: Option<String>,
    source_commit: Option<String>,
    deployment_id: Option<String>,
    signature_algorithm: String,
    #[serde(with = "serde_bytes")]
    signature: Vec<u8>,
    private_state_exposed: bool,
    activation_witness_sha256: Option<String>,
}

impl IdunnSignedDaemonHealthRecord {
    const SCHEMA: &'static str = "idunn.signed_daemon_health.v1";

    fn validate(&self) -> Result<()> {
        for (value, label) in [
            (&self.daemon_id, "daemon id"),
            (&self.health_contract, "health contract"),
            (&self.source_runtime_id, "source runtime id"),
            (&self.signer_identity_id, "signer identity id"),
            (&self.publisher_incarnation_id, "publisher incarnation id"),
        ] {
            require_id(value, label)?;
        }
        if self.schema_version != Self::SCHEMA
            || !matches!(
                self.state.as_str(),
                "active" | "warming" | "degraded" | "failed"
            )
            || self.detail.len() > 512
            || self.detail.chars().any(char::is_control)
            || self.publisher_sequence == 0
            || self.observed_at_unix_millis == 0
            || self.signature_algorithm != "ed25519"
            || self.signature.len() != 64
            || self.private_state_exposed
        {
            bail!("signed daemon health shape is invalid");
        }
        IdunnReleaseBinding {
            release_id: self
                .release_id
                .clone()
                .ok_or_else(|| anyhow!("signed health release id is absent"))?,
            release_witness_sha256: self
                .release_witness_sha256
                .clone()
                .ok_or_else(|| anyhow!("signed health release witness is absent"))?,
            source_commit: self
                .source_commit
                .clone()
                .ok_or_else(|| anyhow!("signed health source commit is absent"))?,
            deployment_id: self
                .deployment_id
                .clone()
                .ok_or_else(|| anyhow!("signed health deployment id is absent"))?,
        }
        .validate()?;
        IdunnActivationBinding {
            activation_witness_sha256: self
                .activation_witness_sha256
                .clone()
                .ok_or_else(|| anyhow!("signed health activation witness is absent"))?,
        }
        .validate()?;
        Ok(())
    }
}

pub(crate) struct IdunnHealthPublisher {
    endpoint: SocketAddr,
    daemon_id: String,
    source_runtime_id: String,
    health_contract: String,
    signer: ServiceIdentitySigner<GameCultProviderHealthIdentity>,
    release: IdunnReleaseBinding,
    activation: IdunnActivationBinding,
    publisher_incarnation_id: String,
    publisher_sequence: u64,
}

impl IdunnHealthPublisher {
    pub(crate) fn open(
        endpoint: SocketAddr,
        daemon_id: impl Into<String>,
        source_runtime_id: impl Into<String>,
        health_contract: impl Into<String>,
        identity_store: impl AsRef<Path>,
        release: IdunnReleaseBinding,
        activation: IdunnActivationBinding,
    ) -> Result<Self> {
        let daemon_id = daemon_id.into();
        let source_runtime_id = source_runtime_id.into();
        let health_contract = health_contract.into();
        require_id(&daemon_id, "Idunn daemon id")?;
        require_id(&source_runtime_id, "health source runtime id")?;
        require_id(&health_contract, "Idunn health contract")?;
        release.validate()?;
        activation.validate()?;
        Ok(Self {
            endpoint,
            daemon_id,
            source_runtime_id,
            health_contract,
            signer: open_service_identity_at::<GameCultProviderHealthIdentity>(
                identity_store.as_ref(),
            )?,
            release,
            activation,
            publisher_incarnation_id: uuid::Uuid::new_v4().to_string(),
            publisher_sequence: 0,
        })
    }

    pub(crate) fn publish(
        &mut self,
        state: &str,
        detail: &str,
    ) -> Result<PublishedHealthStatementIdentity> {
        if !matches!(state, "active" | "warming" | "degraded" | "failed") {
            bail!("unsupported Idunn daemon health state");
        }
        if detail.len() > 512 || detail.chars().any(char::is_control) {
            bail!("Idunn daemon health detail is oversized or contains control characters");
        }
        self.publisher_sequence = self
            .publisher_sequence
            .checked_add(1)
            .ok_or_else(|| anyhow!("Idunn health publisher sequence overflow"))?;
        let mut record = IdunnSignedDaemonHealthRecord {
            schema_version: IdunnSignedDaemonHealthRecord::SCHEMA.into(),
            daemon_id: self.daemon_id.clone(),
            health_contract: self.health_contract.clone(),
            source_runtime_id: self.source_runtime_id.clone(),
            state: state.into(),
            detail: detail.into(),
            signer_identity_id: self.signer.entry().identity_id.clone(),
            publisher_incarnation_id: self.publisher_incarnation_id.clone(),
            publisher_sequence: self.publisher_sequence,
            observed_at_unix_millis: chrono::Utc::now().timestamp_millis().try_into()?,
            release_id: Some(self.release.release_id.clone()),
            release_witness_sha256: Some(self.release.release_witness_sha256.clone()),
            source_commit: Some(self.release.source_commit.clone()),
            deployment_id: Some(self.release.deployment_id.clone()),
            signature_algorithm: "ed25519".into(),
            signature: Vec::new(),
            private_state_exposed: false,
            activation_witness_sha256: Some(self.activation.activation_witness_sha256.clone()),
        };
        let unsigned = canonical_unsigned_record(&record)?;
        let proof = self
            .signer
            .sign::<IdunnSignedDaemonHealthPurpose>(&unsigned);
        if proof.identity_id != record.signer_identity_id {
            bail!("provider-health signer identity disagrees with record");
        }
        record.signature = proof.signature;
        record.validate()?;
        let signed_health_sha256 =
            publish_signed_health(self.endpoint, &self.source_runtime_id, &record)?;
        Ok(PublishedHealthStatementIdentity {
            daemon_id: record.daemon_id,
            signed_health_sha256,
            publisher_incarnation_id: record.publisher_incarnation_id,
            publisher_sequence: record.publisher_sequence,
            signer_identity_id: record.signer_identity_id,
        })
    }

    pub(crate) fn public_key(&self) -> &[u8] {
        &self.signer.entry().public_key
    }
}

fn publish_signed_health(
    endpoint: SocketAddr,
    source_runtime_id: &str,
    signed: &IdunnSignedDaemonHealthRecord,
) -> Result<String> {
    signed.validate()?;
    let payload = rmp_serde::to_vec(signed).context("encoding canonical signed Idunn health")?;
    let decoded: IdunnSignedDaemonHealthRecord = rmp_serde::from_slice(&payload)?;
    if decoded != *signed || rmp_serde::to_vec(&decoded)? != payload {
        bail!("signed Idunn health encoding is noncanonical");
    }
    let signed_health_sha256 = format!("sha256-{}", hex(&Sha256::digest(&payload)));
    let message = CultNetMessage::DocumentPutRaw {
        message_id: format!(
            "ghostlight-signed-health:{}:{}:{}",
            signed.daemon_id, signed.publisher_incarnation_id, signed.publisher_sequence
        ),
        document: CultNetRawDocumentRecord {
            schema_id: IdunnSignedDaemonHealthRecord::SCHEMA.into(),
            record_key: signed.daemon_id.clone(),
            stored_at: chrono::DateTime::from_timestamp_millis(
                signed.observed_at_unix_millis.try_into()?,
            )
            .context("signed health observation time is invalid")?
            .to_rfc3339(),
            payload_encoding: CultNetRawPayloadEncoding::Messagepack,
            payload,
            source_runtime_id: Some(source_runtime_id.into()),
            source_agent_id: Some(signed.signer_identity_id.clone()),
            source_role: Some("daemon-health-publisher".into()),
            tags: Some(vec![CULTNET_RUDP_PROTOCOL_ID.into()]),
        },
    };
    let bind = if endpoint.is_ipv4() {
        "0.0.0.0:0"
    } else {
        "[::]:0"
    };
    let socket = UdpSocket::bind(bind)
        .with_context(|| format!("binding Ghostlight Idunn RUDP sender at {bind}"))?;
    socket.set_read_timeout(Some(Duration::from_millis(100)))?;
    let mut transport =
        CultNetRudpSocketTransportConnection::new(CultNetRudpSocketTransportOptions::client(
            source_runtime_id,
            socket,
            endpoint,
            IDUNN_HEALTH_RUDP_CONNECTION_ID,
        ))?;
    transport.connect(Vec::new())?;
    let deadline = Instant::now() + Duration::from_millis(500);
    while !transport.connected() {
        let _ = transport.receive_once()?;
        transport.poll_resends()?;
        if Instant::now() >= deadline {
            return Err(anyhow!(
                "timed out connecting Ghostlight health publisher to {endpoint}"
            ));
        }
    }
    transport.send(
        "schema",
        encode_cultnet_message_to_vec(&message, CultNetWireContract::CultNetSchemaV0)?,
    )?;
    Ok(signed_health_sha256)
}

fn parse_activation_witness(bytes: &[u8]) -> Result<ActivationWitness> {
    let text = std::str::from_utf8(bytes).context("Ghostlight activation witness is not UTF-8")?;
    let body = text
        .strip_suffix('\n')
        .ok_or_else(|| anyhow!("Ghostlight activation witness has no canonical final newline"))?;
    if body.contains('\r') {
        bail!("Ghostlight activation witness has noncanonical line endings");
    }
    let lines = body.split('\n').collect::<Vec<_>>();
    let [
        schema,
        activation_id,
        daemon_id,
        release_id,
        release_witness_sha256,
        source_commit,
        deployment_id,
        unit_path,
        unit_sha256,
        runtime_environment_path,
        runtime_environment_sha256,
    ] = lines.as_slice()
    else {
        bail!("Ghostlight activation witness has an unexpected field set");
    };
    if exact_witness_value(schema, "schema_version")? != GHOSTLIGHT_ACTIVATION_SCHEMA {
        bail!("Ghostlight activation witness schema is not admitted");
    }
    let witness = ActivationWitness {
        activation_id: exact_witness_value(activation_id, "activation_id")?.into(),
        daemon_id: exact_witness_value(daemon_id, "daemon_id")?.into(),
        release_id: exact_witness_value(release_id, "release_id")?.into(),
        release_witness_sha256: exact_witness_value(
            release_witness_sha256,
            "release_witness_sha256",
        )?
        .into(),
        source_commit: exact_witness_value(source_commit, "source_commit")?.into(),
        deployment_id: exact_witness_value(deployment_id, "deployment_id")?.into(),
        unit_path: exact_witness_value(unit_path, "unit_path")?.into(),
        unit_sha256: exact_witness_value(unit_sha256, "unit_sha256")?.into(),
        runtime_environment_path: exact_witness_value(
            runtime_environment_path,
            "runtime_environment_path",
        )?
        .into(),
        runtime_environment_sha256: exact_witness_value(
            runtime_environment_sha256,
            "runtime_environment_sha256",
        )?
        .into(),
    };
    validate_activation_witness(&witness)?;
    Ok(witness)
}

fn exact_witness_value<'a>(line: &'a str, field: &str) -> Result<&'a str> {
    let prefix = format!("{field}=");
    let value = line
        .strip_prefix(&prefix)
        .with_context(|| format!("Ghostlight activation witness field {field} is out of order"))?;
    if value.is_empty() || value.trim() != value || value.chars().any(char::is_control) {
        bail!("Ghostlight activation witness field {field} is malformed");
    }
    Ok(value)
}

fn validate_activation_witness(witness: &ActivationWitness) -> Result<()> {
    require_safe_token(&witness.activation_id, "activation id")?;
    require_safe_token(&witness.deployment_id, "deployment id")?;
    require_id(&witness.release_id, "release id")?;
    if witness.daemon_id != TRAFFIC_ADMISSION_KEY
        || witness.unit_path != GHOSTLIGHT_UNIT_PATH
        || witness.runtime_environment_path != GHOSTLIGHT_RUNTIME_ENVIRONMENT_PATH
        || !is_lower_hex(&witness.source_commit, 40)
    {
        bail!("Ghostlight activation witness identity is malformed");
    }
    require_sha256(&witness.release_witness_sha256, "release witness digest")?;
    require_sha256(&witness.unit_sha256, "systemd unit digest")?;
    require_sha256(
        &witness.runtime_environment_sha256,
        "runtime environment digest",
    )?;
    Ok(())
}

fn traffic_admission_expectation(
    release: &IdunnReleaseBinding,
    activation: &IdunnActivationBinding,
    published: &PublishedHealthStatementIdentity,
) -> Result<TrafficAdmissionConsumerProjection> {
    release.validate()?;
    activation.validate()?;
    if published.daemon_id != TRAFFIC_ADMISSION_KEY || published.publisher_sequence != 1 {
        bail!("signed health identity for traffic admission is malformed");
    }
    require_sha256(
        &published.signed_health_sha256,
        "signed health statement digest",
    )?;
    require_id(
        &published.signer_identity_id,
        "traffic admission signer identity",
    )?;
    uuid::Uuid::parse_str(&published.publisher_incarnation_id)
        .context("traffic admission publisher incarnation is malformed")?;
    let process = current_runtime_process_instance()?;
    let expected = TrafficAdmissionConsumerProjection {
        schema_version: TRAFFIC_ADMISSION_SCHEMA.into(),
        daemon_id: published.daemon_id.clone(),
        release_id: release.release_id.clone(),
        release_witness_sha256: release.release_witness_sha256.clone(),
        source_commit: release.source_commit.clone(),
        deployment_id: release.deployment_id.clone(),
        activation_witness_sha256: activation.activation_witness_sha256.clone(),
        signed_health_sha256: published.signed_health_sha256.clone(),
        publisher_incarnation_id: published.publisher_incarnation_id.clone(),
        publisher_sequence: published.publisher_sequence,
        signer_identity_id: published.signer_identity_id.clone(),
        runtime_process_id: process.process_id,
        runtime_process_starttime_ticks: process.starttime_ticks,
    };
    validate_traffic_admission_projection(&expected)?;
    Ok(expected)
}

fn decode_traffic_admission_entries(
    entries: &[CultCacheEnvelope],
) -> Result<TrafficAdmissionConsumerProjection> {
    let [record] = entries else {
        bail!("Ghostlight traffic admission store must contain exactly one record");
    };
    if record.key != TRAFFIC_ADMISSION_KEY
        || record.r#type != TRAFFIC_ADMISSION_TYPE
        || record.schema_id.as_deref() != Some(TRAFFIC_ADMISSION_SCHEMA)
    {
        bail!("Ghostlight traffic admission store has the wrong typed envelope");
    }
    let admitted: TrafficAdmissionConsumerProjection = rmp_serde::from_slice(&record.payload)
        .context("decoding typed Ghostlight traffic admission record")?;
    if rmp_serde::to_vec(&admitted)? != record.payload {
        bail!("Ghostlight traffic admission payload is not canonical positional MessagePack");
    }
    validate_traffic_admission_projection(&admitted)?;
    Ok(admitted)
}

fn validate_traffic_admission_projection(
    admission: &TrafficAdmissionConsumerProjection,
) -> Result<()> {
    if admission.schema_version != TRAFFIC_ADMISSION_SCHEMA
        || admission.daemon_id != TRAFFIC_ADMISSION_KEY
        || admission.publisher_sequence == 0
        || admission.runtime_process_id == 0
        || admission.runtime_process_starttime_ticks == 0
        || !is_lower_hex(&admission.source_commit, 40)
    {
        bail!("Ghostlight traffic admission typed identity is invalid");
    }
    require_id(&admission.release_id, "traffic admission release id")?;
    require_safe_token(&admission.deployment_id, "traffic admission deployment id")?;
    require_id(
        &admission.signer_identity_id,
        "traffic admission signer identity",
    )?;
    uuid::Uuid::parse_str(&admission.publisher_incarnation_id)
        .context("traffic admission publisher incarnation is malformed")?;
    require_sha256(
        &admission.release_witness_sha256,
        "traffic admission release witness digest",
    )?;
    require_sha256(
        &admission.activation_witness_sha256,
        "traffic admission activation witness digest",
    )?;
    require_sha256(
        &admission.signed_health_sha256,
        "traffic admission signed health digest",
    )?;
    Ok(())
}

fn current_runtime_process_instance() -> Result<RuntimeProcessInstanceBinding> {
    let process_id = std::process::id();
    if process_id == 0 {
        bail!("current Ghostlight process id is invalid");
    }
    let stat = std::fs::read_to_string("/proc/self/stat")
        .context("reading current Ghostlight /proc starttime")?;
    Ok(RuntimeProcessInstanceBinding {
        process_id,
        starttime_ticks: parse_proc_stat_starttime(&stat, process_id)?,
    })
}

fn parse_proc_stat_starttime(stat: &str, expected_process_id: u32) -> Result<u64> {
    let stat = stat.strip_suffix('\n').unwrap_or(stat);
    if stat.contains('\r') || stat.contains('\n') {
        bail!("current Ghostlight /proc stat has noncanonical line endings");
    }
    let prefix = format!("{expected_process_id} (");
    if !stat.starts_with(&prefix) {
        bail!("current Ghostlight /proc stat has the wrong process id");
    }
    let command_end = stat
        .rfind(") ")
        .context("current Ghostlight /proc stat has no command terminator")?;
    let fields = stat[command_end + 2..]
        .split_ascii_whitespace()
        .collect::<Vec<_>>();
    if fields.len() <= 19 || fields[0].len() != 1 {
        bail!("current Ghostlight /proc stat is missing starttime");
    }
    let value = fields[19];
    let starttime_ticks = value
        .parse::<u64>()
        .context("current Ghostlight /proc starttime is malformed")?;
    if starttime_ticks == 0 || starttime_ticks.to_string() != value {
        bail!("current Ghostlight /proc starttime is noncanonical");
    }
    Ok(starttime_ticks)
}

fn require_exact_traffic_admission(
    admitted: &TrafficAdmissionConsumerProjection,
    expected: &TrafficAdmissionConsumerProjection,
) -> Result<()> {
    if admitted != expected {
        bail!("root traffic admission does not match the exact startup statement");
    }
    Ok(())
}

fn require_sealed_traffic_admission_files(path: &Path, ghostlight_gid: u32) -> Result<()> {
    #[cfg(not(target_os = "linux"))]
    bail!("Ghostlight traffic admission sealing requires Linux");

    #[cfg(target_os = "linux")]
    {
        if path != Path::new(TRAFFIC_ADMISSION_PATH) {
            bail!("Ghostlight traffic admission is not at its fixed path");
        }
        let parent = path
            .parent()
            .context("Ghostlight traffic admission has no parent")?;
        let expected_parent = Path::new("/etc/gamecult/ghostlight-dungeon/runtime");
        if parent != expected_parent {
            bail!("Ghostlight traffic admission parent is not canonical");
        }
        require_exact_root_group_mode(
            parent,
            "traffic admission directory",
            ghostlight_gid,
            0o750,
            true,
        )?;
        require_exact_root_group_mode(
            path,
            "traffic admission record",
            ghostlight_gid,
            0o640,
            false,
        )?;
        let lock_path = sibling_lock_path(path);
        require_exact_root_group_mode(
            &lock_path,
            "traffic admission shared lock",
            ghostlight_gid,
            0o640,
            false,
        )?;
        for ancestor in parent.ancestors().skip(1) {
            let metadata = std::fs::symlink_metadata(ancestor).with_context(|| {
                format!(
                    "inspecting traffic admission ancestor {}",
                    ancestor.display()
                )
            })?;
            if metadata.file_type().is_symlink()
                || metadata.uid() != 0
                || metadata.mode() & 0o022 != 0
            {
                bail!("traffic admission ancestor is indirect or mutable outside root");
            }
        }
        Ok(())
    }
}

#[cfg(target_os = "linux")]
fn require_exact_root_group_mode(
    path: &Path,
    label: &str,
    group_id: u32,
    mode: u32,
    directory: bool,
) -> Result<()> {
    let metadata = std::fs::symlink_metadata(path)
        .with_context(|| format!("inspecting {label} {}", path.display()))?;
    if metadata.file_type().is_symlink()
        || metadata.uid() != 0
        || metadata.gid() != group_id
        || metadata.mode() & 0o7777 != mode
        || (directory && !metadata.file_type().is_dir())
        || (!directory && !metadata.file_type().is_file())
    {
        bail!("{label} does not have the exact root:ghostlight sealed shape");
    }
    Ok(())
}

fn local_group_id(name: &str) -> Result<u32> {
    #[cfg(not(target_os = "linux"))]
    bail!("local group identity verification requires Linux");

    #[cfg(target_os = "linux")]
    {
        let bytes = read_root_controlled_file(Path::new("/etc/group"), "local group database")?;
        let text = std::str::from_utf8(&bytes).context("local group database is not UTF-8")?;
        let mut found = None;
        for line in text.lines() {
            let fields = line.split(':').collect::<Vec<_>>();
            if fields.first().copied() != Some(name) {
                continue;
            }
            let value = fields
                .get(2)
                .filter(|value| !value.is_empty())
                .context("local Ghostlight group has no numeric id")?;
            let group_id = value
                .parse::<u32>()
                .context("local Ghostlight group id is malformed")?;
            if group_id.to_string() != *value || found.replace(group_id).is_some() {
                bail!("local Ghostlight group identity is noncanonical or duplicate");
            }
        }
        found.context("local Ghostlight group is absent")
    }
}

fn sibling_lock_path(path: &Path) -> PathBuf {
    let mut lock_name = path
        .file_name()
        .map(|value| value.to_os_string())
        .unwrap_or_else(|| "cultcache.cc".into());
    lock_name.push(".lock");
    path.with_file_name(lock_name)
}

fn read_root_controlled_file(path: &Path, label: &str) -> Result<Vec<u8>> {
    #[cfg(not(target_os = "linux"))]
    bail!("{label} root-control verification requires Linux");

    #[cfg(target_os = "linux")]
    {
        if !path.is_absolute() {
            bail!("{label} path is not absolute");
        }
        for candidate in path.ancestors() {
            let metadata = std::fs::symlink_metadata(candidate)
                .with_context(|| format!("inspecting {label} path {}", candidate.display()))?;
            if metadata.file_type().is_symlink()
                || metadata.uid() != 0
                || metadata.mode() & 0o022 != 0
            {
                bail!("{label} path is indirect or mutable outside root");
            }
        }
        let metadata = std::fs::symlink_metadata(path)
            .with_context(|| format!("inspecting {label} {}", path.display()))?;
        if !metadata.file_type().is_file() {
            bail!("{label} is not a regular file");
        }
        std::fs::read(path).with_context(|| format!("reading {label} {}", path.display()))
    }
}

fn canonical_unsigned_record(record: &IdunnSignedDaemonHealthRecord) -> Result<Vec<u8>> {
    let mut unsigned = record.clone();
    unsigned.signature.clear();
    let mut signed_shape = unsigned.clone();
    signed_shape.signature = vec![0; 64];
    signed_shape.validate()?;
    uuid::Uuid::parse_str(&unsigned.publisher_incarnation_id)
        .context("publisher incarnation id must be UUID")?;
    rmp_serde::to_vec(&unsigned).context("encoding canonical unsigned Idunn health")
}

fn require_id(value: &str, label: &str) -> Result<()> {
    if value.trim().is_empty() || value.len() > 256 || value.chars().any(char::is_control) {
        bail!("{label} is empty, oversized, or contains control characters");
    }
    Ok(())
}

fn require_safe_token(value: &str, label: &str) -> Result<()> {
    if value.is_empty()
        || value.len() > 256
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b':' | b'-'))
    {
        bail!("{label} is malformed");
    }
    Ok(())
}

fn require_sha256(value: &str, label: &str) -> Result<()> {
    if !value
        .strip_prefix("sha256-")
        .is_some_and(|digest| is_lower_hex(digest, 64))
    {
        bail!("{label} is malformed");
    }
    Ok(())
}

fn is_lower_hex(value: &str, length: usize) -> bool {
    value.len() == length
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

#[cfg(test)]
mod tests {
    use super::*;
    use cultnet_rs::{
        GameCultProviderHealthIdentity, IdunnSignedDaemonHealthPurpose, enroll_service_identity_at,
        verify_service_identity_signature,
    };

    #[test]
    fn ghostlight_health_is_canonical_private_free_and_release_bound() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("provider.cc");
        let signer = enroll_service_identity_at::<GameCultProviderHealthIdentity>(&path).unwrap();
        let mut publisher = IdunnHealthPublisher::open(
            "127.0.0.1:17870".parse().unwrap(),
            "yggdrasil-ghostlight",
            "ghostlight-dungeon-yggdrasil",
            GHOSTLIGHT_IDUNN_HEALTH_CONTRACT,
            &path,
            IdunnReleaseBinding {
                release_id: "release-1".into(),
                release_witness_sha256: format!("sha256-{}", "a".repeat(64)),
                source_commit: "b".repeat(40),
                deployment_id: "deployment-1".into(),
            },
            IdunnActivationBinding {
                activation_witness_sha256: format!("sha256-{}", "c".repeat(64)),
            },
        )
        .unwrap();
        publisher.publisher_sequence = 7;
        let mut record = IdunnSignedDaemonHealthRecord {
            schema_version: IdunnSignedDaemonHealthRecord::SCHEMA.into(),
            daemon_id: publisher.daemon_id.clone(),
            health_contract: publisher.health_contract.clone(),
            source_runtime_id: publisher.source_runtime_id.clone(),
            state: "active".into(),
            detail: "world-kernel-serving".into(),
            signer_identity_id: publisher.signer.entry().identity_id.clone(),
            publisher_incarnation_id: publisher.publisher_incarnation_id.clone(),
            publisher_sequence: publisher.publisher_sequence,
            observed_at_unix_millis: 1_787_315_696_789,
            release_id: Some(publisher.release.release_id.clone()),
            release_witness_sha256: Some(publisher.release.release_witness_sha256.clone()),
            source_commit: Some(publisher.release.source_commit.clone()),
            deployment_id: Some(publisher.release.deployment_id.clone()),
            signature_algorithm: "ed25519".into(),
            signature: Vec::new(),
            private_state_exposed: false,
            activation_witness_sha256: Some(publisher.activation.activation_witness_sha256.clone()),
        };
        let unsigned = canonical_unsigned_record(&record).unwrap();
        let proof = publisher
            .signer
            .sign::<IdunnSignedDaemonHealthPurpose>(&unsigned);
        record.signature = proof.signature.clone();
        record.validate().unwrap();
        verify_service_identity_signature::<
            GameCultProviderHealthIdentity,
            IdunnSignedDaemonHealthPurpose,
        >(&signer.trust_anchor().unwrap(), &unsigned, &proof)
        .unwrap();
        assert_eq!(record.release_id.as_deref(), Some("release-1"));
        assert!(!record.private_state_exposed);
        let activation_digest = format!("sha256-{}", "c".repeat(64));
        assert_eq!(
            record.activation_witness_sha256.as_deref(),
            Some(activation_digest.as_str())
        );
        assert_eq!(rmp_serde::to_vec(&record).unwrap()[..3], [0xdc, 0, 18]);
    }

    #[test]
    fn activation_witness_is_strict_ordered_and_launch_bound() {
        let text = format!(
            "schema_version={GHOSTLIGHT_ACTIVATION_SCHEMA}\n\
             activation_id=activation-7\n\
             daemon_id={TRAFFIC_ADMISSION_KEY}\n\
             release_id={release}\n\
             release_witness_sha256=sha256-{release_witness}\n\
             source_commit={source}\n\
             deployment_id=deployment-9\n\
             unit_path={GHOSTLIGHT_UNIT_PATH}\n\
             unit_sha256=sha256-{unit}\n\
             runtime_environment_path={GHOSTLIGHT_RUNTIME_ENVIRONMENT_PATH}\n\
             runtime_environment_sha256=sha256-{runtime}\n",
            release = "a".repeat(40),
            release_witness = "b".repeat(64),
            source = "a".repeat(40),
            unit = "c".repeat(64),
            runtime = "d".repeat(64),
        );
        let parsed = parse_activation_witness(text.as_bytes()).unwrap();
        assert_eq!(parsed.activation_id, "activation-7");
        assert_eq!(parsed.daemon_id, TRAFFIC_ADMISSION_KEY);

        let reordered = text.replacen(
            "activation_id=activation-7\ndaemon_id=yggdrasil-ghostlight",
            "daemon_id=yggdrasil-ghostlight\nactivation_id=activation-7",
            1,
        );
        assert!(parse_activation_witness(reordered.as_bytes()).is_err());
        assert!(parse_activation_witness(text.trim_end().as_bytes()).is_err());
    }

    #[test]
    fn typed_traffic_admission_is_canonical_and_exact_startup_bound() {
        let release = IdunnReleaseBinding {
            release_id: "a".repeat(40),
            release_witness_sha256: format!("sha256-{}", "b".repeat(64)),
            source_commit: "a".repeat(40),
            deployment_id: "deployment-9".into(),
        };
        let activation = IdunnActivationBinding {
            activation_witness_sha256: format!("sha256-{}", "c".repeat(64)),
        };
        let published = PublishedHealthStatementIdentity {
            daemon_id: TRAFFIC_ADMISSION_KEY.into(),
            signed_health_sha256: format!("sha256-{}", "d".repeat(64)),
            publisher_incarnation_id: uuid::Uuid::new_v4().to_string(),
            publisher_sequence: 1,
            signer_identity_id: "signer-1".into(),
        };
        let expected = traffic_admission_expectation(&release, &activation, &published).unwrap();
        let payload = rmp_serde::to_vec(&expected).unwrap();
        assert_eq!(payload.first().copied(), Some(0x9d));
        assert_eq!(expected.runtime_process_id, std::process::id());
        assert!(expected.runtime_process_starttime_ticks > 0);
        let envelope = CultCacheEnvelope {
            key: TRAFFIC_ADMISSION_KEY.into(),
            r#type: TRAFFIC_ADMISSION_TYPE.into(),
            schema_id: Some(TRAFFIC_ADMISSION_SCHEMA.into()),
            stored_at: "2026-09-02T00:00:00Z".into(),
            payload,
        };
        assert_eq!(
            decode_traffic_admission_entries(std::slice::from_ref(&envelope)).unwrap(),
            expected
        );

        let mut substituted = expected.clone();
        substituted.publisher_sequence += 1;
        validate_traffic_admission_projection(&substituted).unwrap();
        assert!(require_exact_traffic_admission(&substituted, &expected).is_err());
        let mut substituted_process = expected.clone();
        substituted_process.runtime_process_id += 1;
        validate_traffic_admission_projection(&substituted_process).unwrap();
        assert!(require_exact_traffic_admission(&substituted_process, &expected).is_err());
        let mut substituted_starttime = expected.clone();
        substituted_starttime.runtime_process_starttime_ticks += 1;
        validate_traffic_admission_projection(&substituted_starttime).unwrap();
        assert!(require_exact_traffic_admission(&substituted_starttime, &expected).is_err());
        let mut stale_schema = expected.clone();
        stale_schema.schema_version = "idunn.runtime_traffic_admission.v1".into();
        assert!(validate_traffic_admission_projection(&stale_schema).is_err());
        let named_payload = rmp_serde::to_vec_named(&expected).unwrap();
        let named = CultCacheEnvelope {
            payload: named_payload,
            ..envelope.clone()
        };
        assert!(decode_traffic_admission_entries(&[named]).is_err());
        assert!(decode_traffic_admission_entries(&[envelope.clone(), envelope.clone()]).is_err());
    }

    #[test]
    fn proc_stat_starttime_parser_binds_pid_and_kernel_tick_identity() {
        let stat =
            "4242 (ghostlight ) worker) S 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 98765 20\n";
        assert_eq!(parse_proc_stat_starttime(stat, 4242).unwrap(), 98_765);
        assert!(parse_proc_stat_starttime(stat, 4243).is_err());
        assert!(
            parse_proc_stat_starttime(
                "4242 (ghostlight) S 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 0",
                4242,
            )
            .is_err()
        );
    }
}
