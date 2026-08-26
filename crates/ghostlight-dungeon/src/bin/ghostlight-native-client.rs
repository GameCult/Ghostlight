use anyhow::{Context, Result, bail};
use base64::{Engine, engine::general_purpose::STANDARD};
use chrono::Utc;
use cultnet_rs::{
    CULTNET_OPERATION_CONNECTION_ID, CultNetMessage, CultNetRudpSocketTransportConnection,
    CultNetRudpSocketTransportOptions,
};
use ghostlight_dungeon::persistence::CampaignStore;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::json;
use std::{
    io::Read,
    net::{SocketAddr, UdpSocket},
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

const SERVICE_ID: &str = "ghostlight.native.player";
const TARGET_RUNTIME_ID: &str = "ghostlight-dungeon-yggdrasil";
const DEFAULT_ENDPOINT: &str = "127.0.0.1:4102";
const CONNECTION_TIMEOUT: Duration = Duration::from_secs(120);
const DEFAULT_OPERATION_TIMEOUT: Duration = Duration::from_secs(120);
const EVE_INVOCATION_TIMEOUT: Duration = Duration::from_secs(600);
const NATIVE_OPERATION_KEEPALIVE_INTERVAL: Duration = Duration::from_secs(5);

#[derive(Clone, Debug, Serialize, Deserialize)]
struct NativeClientState {
    schema: String,
    pending_handle: Option<String>,
    session_token: Option<String>,
    refresh_expires_at: Option<String>,
    updated_at: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AuthBeginReceipt {
    status: String,
    handle: String,
    expires_at: String,
    navigation: AuthNavigation,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AuthNavigation {
    url: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AuthCompletionReceipt {
    status: String,
    message: String,
    session_token: Option<String>,
    refresh_expires_at: Option<String>,
}

struct ClientStore {
    store: CampaignStore,
    row: Option<cultcache_legacy::CultCacheEnvelope>,
    state: NativeClientState,
    path: PathBuf,
}

impl ClientStore {
    fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let store = CampaignStore::open(&path)?;
        let loaded = store.load::<NativeClientState>("native_client_state.v1", "primary")?;
        let (row, state) = loaded.map_or_else(
            || {
                (
                    None,
                    NativeClientState {
                        schema: "ghostlight.native_client_state.v1".into(),
                        pending_handle: None,
                        session_token: None,
                        refresh_expires_at: None,
                        updated_at: Utc::now().to_rfc3339(),
                    },
                )
            },
            |(row, state)| (Some(row), state),
        );
        let client = Self {
            store,
            row,
            state,
            path,
        };
        client.restrict_permissions()?;
        Ok(client)
    }

    fn commit(&mut self) -> Result<()> {
        self.state.updated_at = Utc::now().to_rfc3339();
        self.row = Some(match &self.row {
            Some(row) => {
                self.store
                    .replace(row, "ghostlight.native_client_state.v1", &self.state)?
            }
            None => self.store.insert(
                "native_client_state.v1",
                "ghostlight.native_client_state.v1",
                "primary",
                &self.state,
            )?,
        });
        self.restrict_permissions()
    }

    fn restrict_permissions(&self) -> Result<()> {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            for path in [&self.path, &self.path.with_extension("cc.lock")] {
                if path.exists() {
                    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
                }
            }
        }
        Ok(())
    }
}

fn main() -> Result<()> {
    let (endpoint, state_path, command, rest) = parse_args()?;
    let mut state = ClientStore::open(state_path)?;
    match command.as_str() {
        "auth-begin" => {
            if state.state.pending_handle.is_some() {
                bail!(
                    "A native authentication attempt is already pending. Run auth-complete before starting another."
                );
            }
            let receipt: AuthBeginReceipt = invoke(
                endpoint,
                "ghostlight.auth.begin",
                "ghostlight.native_auth_begin.v1",
                &json!({"schema":"ghostlight.native_auth_begin.v1"}),
            )?;
            if receipt.status != "pending" || receipt.handle.is_empty() {
                bail!("Ghostlight returned an invalid native authentication attempt");
            }
            state.state.pending_handle = Some(receipt.handle);
            state.state.session_token = None;
            state.state.refresh_expires_at = None;
            state.commit()?;
            println!(
                "{}",
                serde_json::to_string_pretty(&json!({
                    "status":"pending",
                    "authorizationUrl":receipt.navigation.url,
                    "expiresAt":receipt.expires_at
                }))?
            );
        }
        "auth-complete" => {
            let handle = state
                .state
                .pending_handle
                .clone()
                .context("No pending native authentication attempt. Run auth-begin first.")?;
            complete_authentication(endpoint, &mut state, &handle)?;
        }
        "auth-complete-stdin" => {
            let mut handle = String::new();
            std::io::stdin().read_to_string(&mut handle)?;
            let handle = handle.trim();
            if handle.is_empty() || handle.len() > 8_192 {
                bail!("stdin did not contain one bounded opaque Heimdall attempt handle");
            }
            complete_authentication(endpoint, &mut state, handle)?;
        }
        "surface" => {
            let session = state
                .state
                .session_token
                .as_deref()
                .context("Native Ghostlight client is not authenticated")?;
            let value: serde_json::Value = invoke(
                endpoint,
                "ghostlight.surface.get",
                "ghostlight.native_surface_get.v1",
                &json!({"schema":"ghostlight.native_surface_get.v1","sessionToken":session,"invite":null}),
            )?;
            println!("{}", serde_json::to_string_pretty(&value)?);
        }
        "invoke" => {
            if rest.len() != 3 {
                bail!("invoke requires <operation> <payload-schema> <source-version>");
            }
            let session = state
                .state
                .session_token
                .as_deref()
                .context("Native Ghostlight client is not authenticated")?;
            let source_version = rest[2].parse::<u64>()?;
            let mut input = String::new();
            std::io::stdin().read_to_string(&mut input)?;
            let payload: serde_json::Value = if input.trim().is_empty() {
                json!({})
            } else {
                serde_json::from_str(&input).context("decode invocation payload from stdin")?
            };
            let invocation = json!({
                "schema":"gamecult.eve.command_invocation.v1",
                "providerId":"gamecult.ghostlight.dungeon",
                "surfaceId":"ghostlight.play",
                "operation":{
                    "operationId":rest[0],
                    "schemaId":rest[1],
                    "idempotencyKey":uuid::Uuid::new_v4().to_string(),
                    "routeHint":{"sourceVersion":source_version,"transport":"cultnet-rudp"}
                },
                "payload":payload,
                "issuedAt":Utc::now().to_rfc3339(),
                "clientId":"ghostlight-native-client",
                "commandBoundary":"ghostlight.eve.commands",
                "receiptSchema":"gamecult.eve.command_result.v1"
            });
            let value: serde_json::Value = invoke(
                endpoint,
                "ghostlight.eve.invoke",
                "ghostlight.native_eve_invocation.v1",
                &json!({
                    "schema":"ghostlight.native_eve_invocation.v1",
                    "sessionToken":session,
                    "invocation":invocation
                }),
            )?;
            if rest[0] == "app.auth.logout" && value["receipt"]["state"] == "accepted" {
                state.state.session_token = None;
                state.state.refresh_expires_at = None;
                state.commit()?;
            }
            println!("{}", serde_json::to_string_pretty(&value)?);
        }
        _ => bail!("unknown native client command {command}"),
    }
    Ok(())
}

fn parse_args() -> Result<(SocketAddr, PathBuf, String, Vec<String>)> {
    let mut args = std::env::args().skip(1).collect::<Vec<_>>();
    let mut endpoint = DEFAULT_ENDPOINT.parse()?;
    let mut state = None;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--endpoint" => {
                let value = args.get(index + 1).context("--endpoint requires a value")?;
                endpoint = value.parse()?;
                args.drain(index..=index + 1);
            }
            "--state" => {
                let value = args.get(index + 1).context("--state requires a value")?;
                state = Some(PathBuf::from(value));
                args.drain(index..=index + 1);
            }
            _ => index += 1,
        }
    }
    let state = state.context("--state <path.cc> is required")?;
    let command = args.first().cloned().context(
        "usage: ghostlight-native-client --state <path.cc> [--endpoint host:port] auth-begin|auth-complete|auth-complete-stdin|surface|invoke",
    )?;
    Ok((endpoint, state, command, args.into_iter().skip(1).collect()))
}

fn complete_authentication(
    endpoint: SocketAddr,
    state: &mut ClientStore,
    handle: &str,
) -> Result<()> {
    let receipt: AuthCompletionReceipt = invoke(
        endpoint,
        "ghostlight.auth.complete",
        "ghostlight.native_auth_complete.v1",
        &json!({"schema":"ghostlight.native_auth_complete.v1","handle":handle}),
    )?;
    if apply_auth_completion(&mut state.state, handle, &receipt)? {
        state.commit()?;
    }
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "status":receipt.status,
            "message":receipt.message,
            "refreshExpiresAt":state.state.refresh_expires_at
        }))?
    );
    Ok(())
}

fn apply_auth_completion(
    state: &mut NativeClientState,
    handle: &str,
    receipt: &AuthCompletionReceipt,
) -> Result<bool> {
    if receipt.status == "authenticated" {
        state.session_token = Some(
            receipt
                .session_token
                .clone()
                .context("Authenticated native receipt omitted its session token")?,
        );
        state.refresh_expires_at = receipt.refresh_expires_at.clone();
        state.pending_handle = None;
        return Ok(true);
    }
    if receipt.status != "pending" && state.pending_handle.as_deref() == Some(handle) {
        state.pending_handle = None;
        return Ok(true);
    }
    Ok(false)
}

fn invoke<T: DeserializeOwned>(
    endpoint: SocketAddr,
    operation: &str,
    payload_schema: &str,
    payload: &impl Serialize,
) -> Result<T> {
    let socket = UdpSocket::bind("127.0.0.1:0")?;
    socket.set_read_timeout(Some(Duration::from_millis(25)))?;
    let runtime_id = format!("ghostlight-native-client:{}", std::process::id());
    let mut options = CultNetRudpSocketTransportOptions::client(
        runtime_id.clone(),
        socket,
        endpoint,
        CULTNET_OPERATION_CONNECTION_ID,
    );
    options.max_fragment_bytes = Some(2_048);
    options.max_pending_reliable_packets = Some(4_096);
    let mut transport = CultNetRudpSocketTransportConnection::new(options)?;
    transport.connect(Vec::new())?;
    let connection_deadline = Instant::now() + CONNECTION_TIMEOUT;
    while !transport.connected() && Instant::now() < connection_deadline {
        let _ = transport.receive_once()?;
        transport.poll_resends()?;
    }
    if !transport.connected() {
        bail!("Ghostlight native CultNet connection timed out");
    }
    let message_id = uuid::Uuid::new_v4().to_string();
    transport.send_schema_message(&CultNetMessage::OperationRequest {
        message_id: message_id.clone(),
        service_id: SERVICE_ID.into(),
        operation: operation.into(),
        payload_schema: payload_schema.into(),
        payload_encoding: "messagepack-base64".into(),
        payload: STANDARD.encode(rmp_serde::to_vec_named(payload)?),
        source_runtime_id: Some(runtime_id),
        target_runtime_id: Some(TARGET_RUNTIME_ID.into()),
    })?;
    let operation_deadline = Instant::now() + operation_timeout(operation);
    let mut next_keepalive = Instant::now() + NATIVE_OPERATION_KEEPALIVE_INTERVAL;
    while Instant::now() < operation_deadline {
        if let Some(response) = transport.receive_schema_message_once()? {
            let CultNetMessage::OperationResponse {
                message_id: response_id,
                status,
                payload_encoding,
                payload,
                diagnostics,
                ..
            } = response
            else {
                continue;
            };
            if response_id != message_id {
                continue;
            }
            if status != "accepted" {
                bail!(
                    "Ghostlight denied the native operation: {}",
                    diagnostics.join("; ")
                );
            }
            if payload_encoding != "messagepack-base64" {
                bail!("Ghostlight returned an unsupported native payload encoding");
            }
            return Ok(rmp_serde::from_slice(&STANDARD.decode(payload)?)?);
        }
        if Instant::now() >= next_keepalive {
            transport.ping(message_id.as_bytes().to_vec())?;
            next_keepalive = Instant::now() + NATIVE_OPERATION_KEEPALIVE_INTERVAL;
        }
        transport.poll_resends()?;
    }
    let stats = transport.stats();
    bail!(
        "Ghostlight native operation timed out while connected={}; transport sent {} bytes/{} frames and received {} bytes/{} frames",
        transport.connected(),
        stats.bytes_sent,
        stats.frames_sent,
        stats.bytes_received,
        stats.frames_received,
    )
}

fn operation_timeout(operation: &str) -> Duration {
    if operation == "ghostlight.eve.invoke" {
        EVE_INVOCATION_TIMEOUT
    } else {
        DEFAULT_OPERATION_TIMEOUT
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state(handle: Option<&str>) -> NativeClientState {
        NativeClientState {
            schema: "ghostlight.native_client_state.v1".into(),
            pending_handle: handle.map(str::to_owned),
            session_token: None,
            refresh_expires_at: None,
            updated_at: "2026-08-23T00:00:00Z".into(),
        }
    }

    fn receipt(status: &str) -> AuthCompletionReceipt {
        AuthCompletionReceipt {
            status: status.into(),
            message: status.into(),
            session_token: (status == "authenticated").then(|| "session".into()),
            refresh_expires_at: (status == "authenticated").then(|| "2026-09-23T00:00:00Z".into()),
        }
    }

    #[test]
    fn pending_completion_preserves_the_attempt() {
        let mut state = state(Some("owned"));
        assert!(!apply_auth_completion(&mut state, "owned", &receipt("pending")).unwrap());
        assert_eq!(state.pending_handle.as_deref(), Some("owned"));
    }

    #[test]
    fn terminal_completion_clears_only_its_owned_attempt() {
        let mut owned = state(Some("owned"));
        assert!(apply_auth_completion(&mut owned, "owned", &receipt("denied")).unwrap());
        assert!(owned.pending_handle.is_none());

        let mut other = state(Some("current"));
        assert!(!apply_auth_completion(&mut other, "old", &receipt("denied")).unwrap());
        assert_eq!(other.pending_handle.as_deref(), Some("current"));
    }

    #[test]
    fn recovered_authenticated_attempt_supersedes_pending_state() {
        let mut state = state(Some("current"));
        assert!(
            apply_auth_completion(&mut state, "completed-old", &receipt("authenticated")).unwrap()
        );
        assert!(state.pending_handle.is_none());
        assert_eq!(state.session_token.as_deref(), Some("session"));
        assert_eq!(
            state.refresh_expires_at.as_deref(),
            Some("2026-09-23T00:00:00Z")
        );
    }

    #[test]
    fn eve_invocation_wait_budget_is_separate_from_connection_and_authentication() {
        assert_eq!(
            operation_timeout("ghostlight.eve.invoke"),
            Duration::from_secs(600)
        );
        assert_eq!(
            operation_timeout("ghostlight.surface.get"),
            Duration::from_secs(120)
        );
        assert_eq!(CONNECTION_TIMEOUT, Duration::from_secs(120));
        assert!(NATIVE_OPERATION_KEEPALIVE_INTERVAL < DEFAULT_OPERATION_TIMEOUT);
    }
}
