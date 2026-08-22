use aes_gcm::{
    Aes256Gcm, KeyInit,
    aead::{Aead, Payload},
};
use anyhow::{Context, bail};
use base64::{
    Engine,
    engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD},
};
use chrono::{DateTime, SecondsFormat, Utc};
use cultcache_rs::DatabaseEntry;
use cultmesh_rs::{CultMesh, CultMeshNodeOptions, CultMeshRudpSnapshotOptions};
use cultnet_rs::{
    CULTNET_OPERATION_CONNECTION_ID, CultNetMessage, CultNetRudpSocketTransportConnection,
    CultNetRudpSocketTransportOptions,
};
use hmac::{Hmac, Mac};
use jsonwebtoken::{Algorithm, DecodingKey, Validation, decode, decode_header, jwk::JwkSet};
use rand::RngCore;
use reqwest::Client;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use sha2::{Digest, Sha256};
use std::{
    net::{SocketAddr, UdpSocket},
    path::PathBuf,
    time::{Duration, Instant},
};

const APP_SLUG: &str = "ghostlight";
const PRIVATE_SERVICE: &str = "heimdall.private.commands";
const PRIVATE_ENVELOPE_SCHEMA: &str = "heimdall.private_command_envelope.v1";

#[derive(Clone)]
pub struct HeimdallClient {
    http: Client,
    issuer: String,
    public_app_url: String,
    discord_guild_id: String,
    discord_role_id: String,
    boundary_locator: HeimdallBoundaryLocator,
    runtime_id: String,
    shared_secret: String,
}

#[derive(Clone)]
enum HeimdallBoundaryLocator {
    Odin(SocketAddr),
    #[cfg(test)]
    Fixed(ResolvedHeimdallBoundary),
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ResolvedHeimdallBoundary {
    endpoint: SocketAddr,
    daemon_id: String,
    operations: Vec<String>,
}

#[derive(Clone, Debug, DatabaseEntry)]
#[cultcache(
    type = "heimdall.command_boundary",
    schema = "heimdall.command_boundary.v1"
)]
struct HeimdallCommandBoundaryCatalogEntry {
    #[cultcache(key = 0)]
    value: serde_json::Value,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct HeimdallCommandBoundaryRecord {
    schema: String,
    boundary_id: String,
    daemon_id: String,
    provider_id: String,
    updated_at: String,
    commands: Vec<HeimdallBoundaryCommand>,
    private_route: HeimdallPrivateRoute,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct HeimdallBoundaryCommand {
    operation: String,
    request_schema: String,
    response_schema: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct HeimdallPrivateRoute {
    endpoint: String,
    exposure: String,
    authentication: String,
    secret_bearing: bool,
}

cultmesh_rs::cultmesh_documents!(HeimdallDiscoveryDocuments {
    HeimdallCommandBoundaryCatalogEntry => "heimdall.command_boundary.v1",
});

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PrivateEnvelope {
    schema: String,
    app_slug: String,
    operation: String,
    content_schema: String,
    issued_at: String,
    expires_at: String,
    nonce: String,
    idempotency_key: String,
    iv: String,
    ciphertext: String,
    auth_tag: String,
    signature: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct BeginCommand<'a> {
    provider: &'a str,
    mode: &'a str,
    return_to: &'a str,
    entitlement_policy: DiscordRolePolicy<'a>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RefreshCommand<'a> {
    schema: &'static str,
    refresh_token: &'a str,
    entitlement_policy: DiscordRolePolicy<'a>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct LogoutCommand<'a> {
    schema: &'static str,
    refresh_token: &'a str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct DiscordRolePolicy<'a> {
    kind: &'a str,
    guild_id: &'a str,
    allowed_role_ids: [&'a str; 1],
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthBeginReceipt {
    pub status: String,
    pub handle: String,
    pub expires_at: String,
    pub navigation: AuthNavigation,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthNavigation {
    pub url: String,
    pub allowed_origins: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthCompletionReceipt {
    pub status: String,
    pub handle: Option<String>,
    pub error: Option<String>,
    pub account: Option<AccountSummary>,
    pub session: Option<HeimdallSession>,
    pub access_token: Option<String>,
    pub refresh_token: Option<String>,
    pub refresh: Option<RefreshSummary>,
    #[serde(default)]
    pub shared_capabilities: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct AccountSummary {
    pub id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HeimdallSession {
    pub account_id: String,
    pub session_id: String,
    pub app_slug: String,
    pub access_revision: u64,
    pub expires_at: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RefreshSummary {
    pub expires_at: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthLogoutReceipt {
    pub status: String,
    pub session_id: String,
    pub access_revision: u64,
    pub revoked_at: String,
}

#[derive(Debug, Deserialize)]
pub struct AccessClaims {
    pub iss: String,
    pub aud: String,
    pub sub: String,
    pub sid: String,
    pub exp: u64,
    pub typ: String,
    pub account_id: String,
    pub access_revision: u64,
    pub capabilities: Vec<String>,
    pub app: AppClaim,
}

#[derive(Debug, Deserialize)]
pub struct AppClaim {
    pub slug: String,
}

impl HeimdallClient {
    pub fn from_env() -> anyhow::Result<Self> {
        let odin_endpoint: SocketAddr = required_env("GHOSTLIGHT_ODIN_RUDP")?
            .parse()
            .context("GHOSTLIGHT_ODIN_RUDP is not a socket address")?;
        let secret_path = std::env::var_os("GHOSTLIGHT_HEIMDALL_APP_SECRET_FILE")
            .map(PathBuf::from)
            .context("GHOSTLIGHT_HEIMDALL_APP_SECRET_FILE is required")?;
        let shared_secret = std::fs::read_to_string(&secret_path)
            .with_context(|| format!("failed to read {}", secret_path.display()))?
            .trim()
            .to_owned();
        if shared_secret.is_empty() {
            bail!("Ghostlight's Heimdall app secret is empty");
        }
        Ok(Self {
            http: Client::builder().timeout(Duration::from_secs(10)).build()?,
            issuer: std::env::var("GHOSTLIGHT_HEIMDALL_BASE_URL")
                .unwrap_or_else(|_| "https://heimdall.gamecult.org".into())
                .trim_end_matches('/')
                .into(),
            public_app_url: std::env::var("GHOSTLIGHT_PUBLIC_APP_URL")
                .unwrap_or_else(|_| "https://yggdrasil.gamecult.org/ghostlight/".into()),
            discord_guild_id: required_env("GHOSTLIGHT_DISCORD_GUILD_ID")?,
            discord_role_id: required_env("GHOSTLIGHT_DISCORD_ROLE_ID")?,
            boundary_locator: HeimdallBoundaryLocator::Odin(odin_endpoint),
            runtime_id: std::env::var("GHOSTLIGHT_RUNTIME_ID")
                .unwrap_or_else(|_| "yggdrasil-ghostlight".into()),
            shared_secret,
        })
    }

    #[cfg(test)]
    pub fn fixture() -> Self {
        Self {
            http: Client::new(),
            issuer: "https://heimdall.invalid".into(),
            public_app_url: "https://ghostlight.invalid/".into(),
            discord_guild_id: "guild-kltst".into(),
            discord_role_id: "role-kltst".into(),
            boundary_locator: HeimdallBoundaryLocator::Fixed(ResolvedHeimdallBoundary {
                endpoint: "127.0.0.1:9".parse().unwrap(),
                daemon_id: "yggdrasil-heimdall".into(),
                operations: vec![
                    "heimdall.auth.begin".into(),
                    "heimdall.auth.complete".into(),
                    "heimdall.auth.refresh".into(),
                    "heimdall.auth.logout".into(),
                ],
            }),
            runtime_id: "yggdrasil-ghostlight".into(),
            shared_secret: "fixture-secret".into(),
        }
    }

    pub async fn begin(&self, idempotency_key: &str) -> anyhow::Result<AuthBeginReceipt> {
        let payload = BeginCommand {
            provider: "discord",
            mode: "sign_in",
            return_to: &self.public_app_url,
            entitlement_policy: DiscordRolePolicy {
                kind: "discord_role_access",
                guild_id: &self.discord_guild_id,
                allowed_role_ids: [&self.discord_role_id],
            },
        };
        self.invoke(
            "heimdall.auth.begin",
            "heimdall.auth_begin_command.v1",
            idempotency_key,
            &payload,
            "heimdall.auth_begin_receipt.v1",
        )
        .await
    }

    pub async fn complete(
        &self,
        handle: &str,
        idempotency_key: &str,
    ) -> anyhow::Result<AuthCompletionReceipt> {
        self.invoke(
            "heimdall.auth.complete",
            "heimdall.auth_complete_command.v1",
            idempotency_key,
            &serde_json::json!({"handle": handle}),
            "heimdall.auth_completion_receipt.v1",
        )
        .await
    }

    pub async fn refresh(
        &self,
        refresh_token: &str,
        idempotency_key: &str,
    ) -> anyhow::Result<AuthCompletionReceipt> {
        let payload = RefreshCommand {
            schema: "heimdall.auth_refresh_command.v1",
            refresh_token,
            entitlement_policy: DiscordRolePolicy {
                kind: "discord_role_access",
                guild_id: &self.discord_guild_id,
                allowed_role_ids: [&self.discord_role_id],
            },
        };
        self.invoke(
            "heimdall.auth.refresh",
            "heimdall.auth_refresh_command.v1",
            idempotency_key,
            &payload,
            "heimdall.auth_refresh_receipt.v1",
        )
        .await
    }

    pub async fn logout(
        &self,
        refresh_token: &str,
        idempotency_key: &str,
    ) -> anyhow::Result<AuthLogoutReceipt> {
        let payload = LogoutCommand {
            schema: "heimdall.auth_logout_command.v1",
            refresh_token,
        };
        self.invoke(
            "heimdall.auth.logout",
            "heimdall.auth_logout_command.v1",
            idempotency_key,
            &payload,
            "heimdall.auth_logout_receipt.v1",
        )
        .await
    }

    pub async fn verify_completion(
        &self,
        completion: &AuthCompletionReceipt,
    ) -> anyhow::Result<AccessClaims> {
        if completion.status != "authenticated" {
            bail!("Heimdall completion is not authenticated");
        }
        let token = completion
            .access_token
            .as_deref()
            .context("Heimdall omitted the access token")?;
        let claims = self.verify_access(token).await?;
        let account = completion
            .account
            .as_ref()
            .context("Heimdall omitted the account")?;
        let session = completion
            .session
            .as_ref()
            .context("Heimdall omitted the session")?;
        if account.id != claims.account_id
            || session.account_id != claims.account_id
            || session.session_id != claims.sid
            || session.access_revision != claims.access_revision
            || session.app_slug != APP_SLUG
        {
            bail!("Heimdall completion fields disagree with its signed claim");
        }
        Ok(claims)
    }

    async fn verify_access(&self, token: &str) -> anyhow::Result<AccessClaims> {
        let header = decode_header(token)?;
        if header.alg != Algorithm::EdDSA {
            bail!("Heimdall used an unsupported signing algorithm");
        }
        let kid = header.kid.context("Heimdall token omitted kid")?;
        let jwks: JwkSet = self
            .http
            .get(format!("{}/.well-known/jwks.json", self.issuer))
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        let jwk = jwks
            .find(&kid)
            .context("Heimdall signing key was not published")?;
        let mut validation = Validation::new(Algorithm::EdDSA);
        validation.set_audience(&[APP_SLUG]);
        validation.set_issuer(&[self.issuer.as_str()]);
        let claims =
            decode::<AccessClaims>(token, &DecodingKey::from_jwk(jwk)?, &validation)?.claims;
        if claims.typ != "heimdall_access"
            || claims.aud != APP_SLUG
            || claims.iss != self.issuer
            || claims.sub != claims.account_id
            || claims.app.slug != APP_SLUG
            || !claims
                .capabilities
                .iter()
                .any(|value| value == "app_access")
        {
            bail!("Heimdall claim did not grant Ghostlight app_access");
        }
        Ok(claims)
    }

    async fn invoke<T: DeserializeOwned>(
        &self,
        operation: &str,
        content_schema: &str,
        idempotency_key: &str,
        payload: &impl Serialize,
        expected_schema: &str,
    ) -> anyhow::Result<T> {
        let envelope = seal_envelope(
            operation,
            content_schema,
            idempotency_key,
            &self.shared_secret,
            payload,
        )?;
        let boundary_locator = self.boundary_locator.clone();
        let runtime_id = self.runtime_id.clone();
        let operation = operation.to_owned();
        let request_payload = STANDARD.encode(rmp_serde::to_vec_named(&envelope)?);
        let transport_operation = operation.clone();
        let response = tokio::task::spawn_blocking(move || {
            let boundary = boundary_locator.resolve(&runtime_id, &transport_operation)?;
            invoke_operation(
                boundary.endpoint,
                &boundary.daemon_id,
                &runtime_id,
                &transport_operation,
                request_payload,
            )
        })
        .await??;
        let CultNetMessage::OperationResponse {
            status,
            payload_schema,
            payload_encoding,
            payload,
            diagnostics,
            ..
        } = response
        else {
            bail!("Heimdall returned a non-operation response");
        };
        if status == "denied" {
            bail!(
                "Heimdall denied the private command: {}",
                diagnostics.join("; ")
            );
        }
        if payload_schema != PRIVATE_ENVELOPE_SCHEMA || payload_encoding != "messagepack-base64" {
            bail!("Heimdall returned the wrong private response contract");
        }
        let sealed: PrivateEnvelope = rmp_serde::from_slice(&STANDARD.decode(payload)?)?;
        if sealed.operation != operation || sealed.content_schema != expected_schema {
            bail!("Heimdall private response disagrees with the requested operation");
        }
        open_envelope(&sealed, &self.shared_secret)
    }
}

fn invoke_operation(
    endpoint: SocketAddr,
    target_runtime_id: &str,
    runtime_id: &str,
    operation: &str,
    payload: String,
) -> anyhow::Result<CultNetMessage> {
    let socket = UdpSocket::bind("127.0.0.1:0")?;
    socket.set_read_timeout(Some(Duration::from_millis(25)))?;
    let mut options = CultNetRudpSocketTransportOptions::client(
        runtime_id.to_owned(),
        socket,
        endpoint,
        CULTNET_OPERATION_CONNECTION_ID,
    );
    options.max_fragment_bytes = Some(2_048);
    let mut transport = CultNetRudpSocketTransportConnection::new(options)?;
    transport.connect(Vec::new())?;
    let deadline = Instant::now() + Duration::from_secs(10);
    while !transport.connected() && Instant::now() < deadline {
        let _ = transport.receive_once()?;
        transport.poll_resends()?;
    }
    if !transport.connected() {
        bail!("Heimdall private CultNet connection timed out");
    }
    let message_id = uuid::Uuid::new_v4().to_string();
    transport.send_schema_message(&CultNetMessage::OperationRequest {
        message_id: message_id.clone(),
        service_id: PRIVATE_SERVICE.into(),
        operation: operation.into(),
        payload_schema: PRIVATE_ENVELOPE_SCHEMA.into(),
        payload_encoding: "messagepack-base64".into(),
        payload,
        source_runtime_id: Some(runtime_id.into()),
        target_runtime_id: Some(target_runtime_id.into()),
    })?;
    while Instant::now() < deadline {
        if let Some(response) = transport.receive_schema_message_once()? {
            if matches!(&response, CultNetMessage::OperationResponse { message_id: value, .. } if value == &message_id)
            {
                return Ok(response);
            }
        }
        transport.poll_resends()?;
    }
    bail!("Heimdall private command timed out")
}

impl HeimdallBoundaryLocator {
    fn resolve(
        &self,
        runtime_id: &str,
        operation: &str,
    ) -> anyhow::Result<ResolvedHeimdallBoundary> {
        let boundary = match self {
            Self::Odin(endpoint) => discover_heimdall_boundary(*endpoint, runtime_id)?,
            #[cfg(test)]
            Self::Fixed(boundary) => boundary.clone(),
        };
        if !boundary
            .operations
            .iter()
            .any(|candidate| candidate == operation)
        {
            bail!("Odin's Heimdall boundary does not advertise {operation}");
        }
        Ok(boundary)
    }
}

fn discover_heimdall_boundary(
    odin_endpoint: SocketAddr,
    runtime_id: &str,
) -> anyhow::Result<ResolvedHeimdallBoundary> {
    let store_path = std::env::temp_dir().join(format!(
        "ghostlight-heimdall-discovery-{}.cc",
        uuid::Uuid::new_v4()
    ));
    let result = (|| {
        let mut node = CultMesh::create_node(
            &store_path,
            HeimdallDiscoveryDocuments,
            CultMeshNodeOptions {
                runtime_id: format!("{runtime_id}:heimdall-discovery"),
                pull_on_start: false,
            },
        )?;
        let mut options = CultMeshRudpSnapshotOptions::odin(
            odin_endpoint,
            format!("{runtime_id}:heimdall-discovery"),
        );
        options.schema_ids = Some(vec!["heimdall.command_boundary.v1".into()]);
        options.record_keys = Some(vec!["heimdall:command-boundary".into()]);
        let applied = node.pull_rudp_catalog_snapshot(options)?;
        if applied != 1 {
            bail!("Odin did not return exactly one Heimdall command boundary");
        }
        let envelope = node
            .cache()
            .snapshot()
            .into_iter()
            .find(|entry| {
                entry.key == "heimdall:command-boundary"
                    && entry.r#type == "heimdall.command_boundary"
                    && entry.schema_id.as_deref() == Some("heimdall.command_boundary.v1")
            })
            .context("Odin omitted the exact Heimdall command boundary envelope")?;
        validate_heimdall_boundary(rmp_serde::from_slice(&envelope.payload)?)
    })();
    let _ = std::fs::remove_file(&store_path);
    result
}

fn validate_heimdall_boundary(
    boundary: HeimdallCommandBoundaryRecord,
) -> anyhow::Result<ResolvedHeimdallBoundary> {
    if boundary.schema != "heimdall.command_boundary.v1"
        || boundary.boundary_id != "heimdall"
        || boundary.provider_id != "heimdall"
        || boundary.daemon_id.trim().is_empty()
        || boundary.updated_at.trim().is_empty()
    {
        bail!("Odin returned a foreign or malformed Heimdall command boundary");
    }
    if boundary.private_route.exposure != "loopback-only"
        || boundary.private_route.secret_bearing
        || boundary.private_route.authentication != "app-bound HMAC + AES-256-GCM envelope"
    {
        bail!("Odin returned an unsafe Heimdall private route");
    }
    let endpoint_text = boundary
        .private_route
        .endpoint
        .strip_prefix("rudp://")
        .context("Heimdall's discovered private route is not RUDP")?;
    let endpoint: SocketAddr = endpoint_text
        .parse()
        .context("Heimdall's discovered private route is not a socket address")?;
    if !endpoint.ip().is_loopback() {
        bail!("Heimdall's discovered private route is not loopback-only");
    }
    if boundary.commands.is_empty()
        || boundary.commands.iter().any(|command| {
            command.operation.trim().is_empty()
                || command.request_schema.trim().is_empty()
                || command.response_schema.trim().is_empty()
        })
    {
        bail!("Heimdall's discovered command catalog is malformed");
    }
    let operations = boundary
        .commands
        .into_iter()
        .map(|command| command.operation)
        .collect();
    Ok(ResolvedHeimdallBoundary {
        endpoint,
        daemon_id: boundary.daemon_id,
        operations,
    })
}

fn seal_envelope(
    operation: &str,
    content_schema: &str,
    idempotency_key: &str,
    secret: &str,
    payload: &impl Serialize,
) -> anyhow::Result<PrivateEnvelope> {
    let issued = Utc::now();
    let expires = issued + chrono::Duration::seconds(30);
    let mut nonce_bytes = [0_u8; 16];
    let mut iv = [0_u8; 12];
    rand::rng().fill_bytes(&mut nonce_bytes);
    rand::rng().fill_bytes(&mut iv);
    let mut envelope = PrivateEnvelope {
        schema: PRIVATE_ENVELOPE_SCHEMA.into(),
        app_slug: APP_SLUG.into(),
        operation: operation.into(),
        content_schema: content_schema.into(),
        issued_at: issued.to_rfc3339_opts(SecondsFormat::Millis, true),
        expires_at: expires.to_rfc3339_opts(SecondsFormat::Millis, true),
        nonce: URL_SAFE_NO_PAD.encode(nonce_bytes),
        idempotency_key: idempotency_key.into(),
        iv: URL_SAFE_NO_PAD.encode(iv),
        ciphertext: String::new(),
        auth_tag: String::new(),
        signature: String::new(),
    };
    let key = Sha256::digest(
        [
            b"heimdall.private-command.v1\0".as_slice(),
            secret.as_bytes(),
        ]
        .concat(),
    );
    let ciphertext_and_tag = Aes256Gcm::new_from_slice(&key)?
        .encrypt(
            (&iv).into(),
            Payload {
                msg: &rmp_serde::to_vec_named(payload)?,
                aad: envelope.aad().as_bytes(),
            },
        )
        .map_err(|_| anyhow::anyhow!("failed to encrypt Heimdall private command"))?;
    let split = ciphertext_and_tag
        .len()
        .checked_sub(16)
        .context("AES-GCM output omitted its tag")?;
    envelope.ciphertext = URL_SAFE_NO_PAD.encode(&ciphertext_and_tag[..split]);
    envelope.auth_tag = URL_SAFE_NO_PAD.encode(&ciphertext_and_tag[split..]);
    envelope.signature = sign(&envelope, secret)?;
    Ok(envelope)
}

fn open_envelope<T: DeserializeOwned>(
    envelope: &PrivateEnvelope,
    secret: &str,
) -> anyhow::Result<T> {
    if envelope.schema != PRIVATE_ENVELOPE_SCHEMA {
        bail!("unsupported Heimdall private envelope");
    }
    let now = Utc::now();
    let issued: DateTime<Utc> = envelope.issued_at.parse()?;
    let expires: DateTime<Utc> = envelope.expires_at.parse()?;
    if issued > now + chrono::Duration::seconds(30)
        || expires <= now
        || expires <= issued
        || expires - issued > chrono::Duration::seconds(120)
    {
        bail!("Heimdall private response is outside its accepted clock window");
    }
    let mut mac = <Hmac<Sha256> as Mac>::new_from_slice(secret.as_bytes())?;
    mac.update(envelope.authenticated_fields().as_bytes());
    mac.verify_slice(&URL_SAFE_NO_PAD.decode(&envelope.signature)?)
        .map_err(|_| anyhow::anyhow!("Heimdall private response signature is invalid"))?;
    let key = Sha256::digest(
        [
            b"heimdall.private-command.v1\0".as_slice(),
            secret.as_bytes(),
        ]
        .concat(),
    );
    let iv = URL_SAFE_NO_PAD.decode(&envelope.iv)?;
    let mut ciphertext = URL_SAFE_NO_PAD.decode(&envelope.ciphertext)?;
    ciphertext.extend(URL_SAFE_NO_PAD.decode(&envelope.auth_tag)?);
    let plaintext = Aes256Gcm::new_from_slice(&key)?
        .decrypt(
            iv.as_slice().into(),
            Payload {
                msg: &ciphertext,
                aad: envelope.aad().as_bytes(),
            },
        )
        .map_err(|_| anyhow::anyhow!("failed to decrypt Heimdall private response"))?;
    Ok(rmp_serde::from_slice(&plaintext)?)
}

fn sign(envelope: &PrivateEnvelope, secret: &str) -> anyhow::Result<String> {
    let mut mac = <Hmac<Sha256> as Mac>::new_from_slice(secret.as_bytes())?;
    mac.update(envelope.authenticated_fields().as_bytes());
    Ok(URL_SAFE_NO_PAD.encode(mac.finalize().into_bytes()))
}

impl PrivateEnvelope {
    fn aad(&self) -> String {
        [
            self.schema.as_str(),
            self.app_slug.as_str(),
            self.operation.as_str(),
            self.content_schema.as_str(),
            self.issued_at.as_str(),
            self.expires_at.as_str(),
            self.nonce.as_str(),
            self.idempotency_key.as_str(),
        ]
        .join("\n")
    }

    fn authenticated_fields(&self) -> String {
        [
            self.aad(),
            self.iv.clone(),
            self.ciphertext.clone(),
            self.auth_tag.clone(),
        ]
        .join("\n")
    }
}

fn required_env(name: &str) -> anyhow::Result<String> {
    let value = std::env::var(name).with_context(|| format!("{name} is required"))?;
    let value = value.trim();
    if value.is_empty() {
        bail!("{name} must not be empty");
    }
    Ok(value.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn boundary(endpoint: &str) -> HeimdallCommandBoundaryRecord {
        HeimdallCommandBoundaryRecord {
            schema: "heimdall.command_boundary.v1".into(),
            boundary_id: "heimdall".into(),
            daemon_id: "yggdrasil-heimdall".into(),
            provider_id: "heimdall".into(),
            updated_at: "2026-08-22T12:00:00.000Z".into(),
            commands: vec![HeimdallBoundaryCommand {
                operation: "heimdall.auth.begin".into(),
                request_schema: "heimdall.private_command_envelope.v1".into(),
                response_schema: "heimdall.auth_begin_receipt.v1".into(),
            }],
            private_route: HeimdallPrivateRoute {
                endpoint: endpoint.into(),
                exposure: "loopback-only".into(),
                authentication: "app-bound HMAC + AES-256-GCM envelope".into(),
                secret_bearing: false,
            },
        }
    }

    #[test]
    fn accepts_only_the_discovered_loopback_private_boundary() {
        let resolved = validate_heimdall_boundary(boundary("rudp://127.0.0.1:4101"))
            .expect("canonical boundary should resolve");
        assert_eq!(resolved.endpoint, "127.0.0.1:4101".parse().unwrap());
        assert_eq!(resolved.daemon_id, "yggdrasil-heimdall");
        assert_eq!(resolved.operations, vec!["heimdall.auth.begin"]);
    }

    #[test]
    fn rejects_non_loopback_or_secret_bearing_routes() {
        assert!(
            validate_heimdall_boundary(boundary("rudp://10.77.0.2:4101"))
                .unwrap_err()
                .to_string()
                .contains("not loopback-only")
        );
        let mut secret_bearing = boundary("rudp://127.0.0.1:4101");
        secret_bearing.private_route.secret_bearing = true;
        assert!(
            validate_heimdall_boundary(secret_bearing)
                .unwrap_err()
                .to_string()
                .contains("unsafe")
        );
    }

    #[test]
    fn refuses_operations_not_advertised_by_odin() {
        let locator = HeimdallBoundaryLocator::Fixed(
            validate_heimdall_boundary(boundary("rudp://127.0.0.1:4101")).unwrap(),
        );
        assert!(
            locator
                .resolve("ghostlight-test", "heimdall.auth.logout")
                .unwrap_err()
                .to_string()
                .contains("does not advertise")
        );
    }
}
