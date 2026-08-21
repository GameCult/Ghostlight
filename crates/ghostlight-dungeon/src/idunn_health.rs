use anyhow::{Context, Result, anyhow, bail};
use cultnet_rs::{
    CultNetMessage, CultNetRawDocumentRecord, CultNetRawPayloadEncoding,
    CultNetRudpSocketTransportConnection, CultNetRudpSocketTransportOptions, CultNetWireContract,
    GameCultProviderHealthIdentity, IdunnSignedDaemonHealthPurpose, ServiceIdentitySigner,
    encode_cultnet_message_to_vec, open_service_identity_at,
};
use serde::{Deserialize, Serialize};
use std::{
    net::{SocketAddr, UdpSocket},
    path::Path,
    time::{Duration, Instant},
};

pub const GHOSTLIGHT_IDUNN_HEALTH_CONTRACT: &str = "ghostlight.cultnet-rudp-service-health";
const CULTNET_RUDP_PROTOCOL_ID: &str = "cultnet.transport.rudp.v0";
const IDUNN_HEALTH_RUDP_CONNECTION_ID: u32 = 0x1d0d_0001;

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
            || self.release_id.is_some()
            || self.release_witness_sha256.is_some()
            || self.source_commit.is_some()
            || self.deployment_id.is_some()
            || self.signature_algorithm != "ed25519"
            || self.signature.len() != 64
            || self.private_state_exposed
        {
            bail!("signed daemon health shape is invalid");
        }
        Ok(())
    }
}

pub struct IdunnHealthPublisher {
    endpoint: SocketAddr,
    daemon_id: String,
    source_runtime_id: String,
    health_contract: String,
    signer: ServiceIdentitySigner<GameCultProviderHealthIdentity>,
    publisher_incarnation_id: String,
    publisher_sequence: u64,
}

impl IdunnHealthPublisher {
    pub fn open(
        endpoint: SocketAddr,
        daemon_id: impl Into<String>,
        source_runtime_id: impl Into<String>,
        health_contract: impl Into<String>,
        identity_store: impl AsRef<Path>,
    ) -> Result<Self> {
        let daemon_id = daemon_id.into();
        let source_runtime_id = source_runtime_id.into();
        let health_contract = health_contract.into();
        require_id(&daemon_id, "Idunn daemon id")?;
        require_id(&source_runtime_id, "health source runtime id")?;
        require_id(&health_contract, "Idunn health contract")?;
        Ok(Self {
            endpoint,
            daemon_id,
            source_runtime_id,
            health_contract,
            signer: open_service_identity_at::<GameCultProviderHealthIdentity>(
                identity_store.as_ref(),
            )?,
            publisher_incarnation_id: uuid::Uuid::new_v4().to_string(),
            publisher_sequence: 0,
        })
    }

    pub fn publish(&mut self, state: &str, detail: &str) -> Result<()> {
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
            release_id: None,
            release_witness_sha256: None,
            source_commit: None,
            deployment_id: None,
            signature_algorithm: "ed25519".into(),
            signature: Vec::new(),
            private_state_exposed: false,
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
        publish_signed_health(self.endpoint, &self.source_runtime_id, &record)
    }

    pub fn public_key(&self) -> &[u8] {
        &self.signer.entry().public_key
    }
}

fn publish_signed_health(
    endpoint: SocketAddr,
    source_runtime_id: &str,
    signed: &IdunnSignedDaemonHealthRecord,
) -> Result<()> {
    signed.validate()?;
    let payload = rmp_serde::to_vec(signed).context("encoding canonical signed Idunn health")?;
    let decoded: IdunnSignedDaemonHealthRecord = rmp_serde::from_slice(&payload)?;
    if decoded != *signed || rmp_serde::to_vec(&decoded)? != payload {
        bail!("signed Idunn health encoding is noncanonical");
    }
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
    Ok(())
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

#[cfg(test)]
mod tests {
    use super::*;
    use cultnet_rs::{
        GameCultProviderHealthIdentity, IdunnSignedDaemonHealthPurpose, enroll_service_identity_at,
        verify_service_identity_signature,
    };

    #[test]
    fn ghostlight_health_is_canonical_private_free_and_release_unbound() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("provider.cc");
        let signer = enroll_service_identity_at::<GameCultProviderHealthIdentity>(&path).unwrap();
        let mut publisher = IdunnHealthPublisher::open(
            "127.0.0.1:17870".parse().unwrap(),
            "yggdrasil-ghostlight",
            "ghostlight-dungeon-yggdrasil",
            GHOSTLIGHT_IDUNN_HEALTH_CONTRACT,
            &path,
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
            release_id: None,
            release_witness_sha256: None,
            source_commit: None,
            deployment_id: None,
            signature_algorithm: "ed25519".into(),
            signature: Vec::new(),
            private_state_exposed: false,
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
        assert!(record.release_id.is_none());
        assert!(!record.private_state_exposed);
        assert_eq!(rmp_serde::to_vec(&record).unwrap()[..3], [0xdc, 0, 17]);
    }
}
