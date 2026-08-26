use super::{
    AppState, EveCommandInvocation, complete_native_authentication, eve_command_for_transport,
    surface,
};
use anyhow::{Context, Result, bail};
use axum::{
    extract::State,
    http::{HeaderMap, HeaderValue, header},
    response::Response,
};
use base64::{Engine, engine::general_purpose::STANDARD};
use cultnet_rs::{
    CULTNET_OPERATION_CONNECTION_ID, CultNetMessage, CultNetRudpServerEvent, CultNetRudpServerHub,
    CultNetRudpServerHubOptions, CultNetRudpServerSessionContext, CultNetWireContract,
    decode_cultnet_message_from_slice,
};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::{
    net::{SocketAddr, UdpSocket},
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
        mpsc,
    },
    time::Duration,
};

pub const NATIVE_ENDPOINT: &str = "127.0.0.1:4102";
pub const NATIVE_SERVICE_ID: &str = "ghostlight.native.player";
pub const NATIVE_RUNTIME_ID: &str = "ghostlight-dungeon-yggdrasil";
pub const NATIVE_AUTH_BEGIN: &str = "ghostlight.auth.begin";
pub const NATIVE_AUTH_COMPLETE: &str = "ghostlight.auth.complete";
pub const NATIVE_SURFACE_GET: &str = "ghostlight.surface.get";
pub const NATIVE_EVE_INVOKE: &str = "ghostlight.eve.invoke";
const MAX_IN_FLIGHT: usize = 64;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NativeAuthBeginCommand {
    pub schema: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NativeAuthCompleteCommand {
    pub schema: String,
    pub handle: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NativeSurfaceGetCommand {
    pub schema: String,
    pub session_token: String,
    pub invite: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NativeEveInvokeCommand {
    pub schema: String,
    pub session_token: String,
    pub invocation: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeAuthCompletionReceipt {
    pub schema: String,
    pub status: String,
    pub message: String,
    pub session_token: Option<String>,
    pub refresh_expires_at: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct NativeFailure {
    schema: String,
    code: String,
    message: String,
}

struct CompletedResponse {
    session: CultNetRudpServerSessionContext,
    response: CultNetMessage,
}

pub fn start(state: AppState) -> Result<()> {
    let address: SocketAddr = NATIVE_ENDPOINT.parse()?;
    let socket = UdpSocket::bind(address)
        .with_context(|| format!("bind Ghostlight native CultNet boundary at {address}"))?;
    socket.set_nonblocking(true)?;
    let mut options = CultNetRudpServerHubOptions::new(
        state.mesh.identity().runtime_id.clone(),
        socket,
        CULTNET_OPERATION_CONNECTION_ID,
    );
    options.max_fragment_bytes = Some(2_048);
    options.max_pending_reliable_packets = Some(4_096);
    let hub = CultNetRudpServerHub::new(options)?;
    let runtime = tokio::runtime::Handle::current();
    std::thread::Builder::new()
        .name("ghostlight-native-cultmesh".into())
        .spawn(move || {
            if let Err(error) = run_server(hub, state, runtime) {
                tracing::error!(%error, "Ghostlight native CultMesh boundary stopped");
                std::process::exit(1);
            }
        })?;
    Ok(())
}

fn run_server(
    mut hub: CultNetRudpServerHub,
    state: AppState,
    runtime: tokio::runtime::Handle,
) -> Result<()> {
    let (completed_tx, completed_rx) = mpsc::channel::<CompletedResponse>();
    let in_flight = Arc::new(AtomicUsize::new(0));
    loop {
        while let Ok(completed) = completed_rx.try_recv() {
            in_flight.fetch_sub(1, Ordering::SeqCst);
            let (message_id, payload_bytes) = response_diagnostics(&completed.response);
            match hub.session(completed.session.remote_addr) {
                Some(active)
                    if active.session_generation == completed.session.session_generation =>
                {
                    hub.send_schema_message(&completed.session, &completed.response)?;
                    tracing::info!(
                        %message_id,
                        payload_bytes,
                        session_generation = completed.session.session_generation,
                        remote_addr = %completed.session.remote_addr,
                        "Ghostlight native boundary queued completed operation response"
                    );
                }
                Some(active) => tracing::warn!(
                    %message_id,
                    completed_session_generation = completed.session.session_generation,
                    active_session_generation = active.session_generation,
                    remote_addr = %completed.session.remote_addr,
                    "Ghostlight native boundary fenced a completed response from a replaced session"
                ),
                None => tracing::warn!(
                    %message_id,
                    session_generation = completed.session.session_generation,
                    remote_addr = %completed.session.remote_addr,
                    "Ghostlight native boundary could not deliver a completed response because its session vanished"
                ),
            }
        }
        if let Some(event) = hub.receive_event_once()? {
            let CultNetRudpServerEvent::Frame { session, frame } = event else {
                hub.poll_resends()?;
                continue;
            };
            if frame.channel_id != "schema" {
                continue;
            }
            let request = match decode_cultnet_message_from_slice(
                &frame.payload,
                CultNetWireContract::CultNetSchemaV0,
            ) {
                Ok(request @ CultNetMessage::OperationRequest { .. }) => request,
                Ok(_) => continue,
                Err(error) => {
                    tracing::warn!(%error, "Ghostlight native boundary rejected malformed CultNet payload");
                    continue;
                }
            };
            if in_flight.load(Ordering::SeqCst) >= MAX_IN_FLIGHT {
                let response = failure_response(
                    &request,
                    "busy",
                    "Ghostlight is already processing the maximum native command pressure.",
                )?;
                hub.send_schema_message(&session, &response)?;
                continue;
            }
            in_flight.fetch_add(1, Ordering::SeqCst);
            let tx = completed_tx.clone();
            let request_state = state.clone();
            runtime.spawn(async move {
                let response = handle_operation(request_state, request).await;
                let _ = tx.send(CompletedResponse { session, response });
            });
        }
        hub.poll_resends()?;
        std::thread::sleep(Duration::from_millis(5));
    }
}

fn response_diagnostics(response: &CultNetMessage) -> (String, usize) {
    match response {
        CultNetMessage::OperationResponse {
            message_id,
            payload,
            ..
        } => (message_id.clone(), payload.len()),
        _ => ("non-operation-response".into(), 0),
    }
}

pub(super) async fn handle_operation(state: AppState, request: CultNetMessage) -> CultNetMessage {
    match handle_operation_inner(state, &request).await {
        Ok(response) => response,
        Err(error) => {
            tracing::warn!(%error, "Ghostlight denied a native CultMesh operation");
            failure_response(
                &request,
                "command-denied",
                "Ghostlight denied the typed native operation without changing world state.",
            )
            .unwrap_or_else(|_| bare_failure_response(&request))
        }
    }
}

async fn handle_operation_inner(
    state: AppState,
    request: &CultNetMessage,
) -> Result<CultNetMessage> {
    let CultNetMessage::OperationRequest {
        service_id,
        operation,
        payload_schema,
        payload_encoding,
        target_runtime_id,
        ..
    } = request
    else {
        bail!("native boundary accepts only operation requests");
    };
    if payload_encoding != "messagepack-base64"
        || (service_id != NATIVE_SERVICE_ID
            && service_id != ghostlight_dungeon::consumer::WORLD_CONSUMER_SERVICE_ID)
    {
        bail!("native request does not match the advertised service contract");
    }
    if target_runtime_id
        .as_deref()
        .is_some_and(|target| target != state.mesh.identity().runtime_id)
    {
        bail!("native request targets another runtime");
    }
    if service_id == ghostlight_dungeon::consumer::WORLD_CONSUMER_SERVICE_ID {
        return match operation.as_str() {
            ghostlight_dungeon::consumer::ADMIT_WORLD_OPERATION => {
                require_schema(payload_schema, "ghostlight.world_seed_admission_request.v1")?;
                let command: ghostlight_dungeon::consumer::WorldSeedAdmissionRequest =
                    decode_request(request)?;
                require_schema(
                    &command.schema,
                    "ghostlight.world_seed_admission_request.v1",
                )?;
                let (_, receipt) = state.registry.admit_world_seed(command).await?;
                success_response(
                    request,
                    "ghostlight.world_seed_admission_receipt.v1",
                    &receipt,
                )
            }
            ghostlight_dungeon::consumer::APPLY_EXTERNAL_SNAPSHOT_OPERATION => {
                require_schema(payload_schema, "ghostlight.external_subject_snapshot.v1")?;
                let command: ghostlight_dungeon::consumer::ExternalSubjectSnapshot =
                    decode_request(request)?;
                require_schema(&command.schema, "ghostlight.external_subject_snapshot.v1")?;
                let receipt = state
                    .registry
                    .apply_external_subject_snapshot(command)
                    .await?;
                success_response(request, "ghostlight.external_snapshot_receipt.v1", &receipt)
            }
            ghostlight_dungeon::consumer::LIST_EXTERNAL_PROPOSALS_OPERATION => {
                require_schema(
                    payload_schema,
                    "ghostlight.external_proposal_list_request.v1",
                )?;
                let command: ghostlight_dungeon::consumer::ExternalProposalListRequest =
                    decode_request(request)?;
                require_schema(
                    &command.schema,
                    "ghostlight.external_proposal_list_request.v1",
                )?;
                let proposals = state.registry.list_external_proposals(command).await?;
                success_response(request, "ghostlight.external_proposal_list.v1", &proposals)
            }
            ghostlight_dungeon::consumer::ACKNOWLEDGE_EXTERNAL_PROPOSAL_OPERATION => {
                require_schema(
                    payload_schema,
                    "ghostlight.external_proposal_acknowledgement.v1",
                )?;
                let command: ghostlight_dungeon::consumer::ExternalProposalAcknowledgement =
                    decode_request(request)?;
                require_schema(
                    &command.schema,
                    "ghostlight.external_proposal_acknowledgement.v1",
                )?;
                let receipt = state
                    .registry
                    .acknowledge_external_proposal(command)
                    .await?;
                success_response(request, "ghostlight.external_proposal_receipt.v1", &receipt)
            }
            _ => bail!("world consumer operation is not advertised by Ghostlight"),
        };
    }
    match operation.as_str() {
        NATIVE_AUTH_BEGIN => {
            require_schema(payload_schema, "ghostlight.native_auth_begin.v1")?;
            let command: NativeAuthBeginCommand = decode_request(request)?;
            require_schema(&command.schema, "ghostlight.native_auth_begin.v1")?;
            let receipt = state.heimdall.begin(message_id(request)?).await?;
            success_response(request, "heimdall.auth_begin_receipt.v1", &receipt)
        }
        NATIVE_AUTH_COMPLETE => {
            require_schema(payload_schema, "ghostlight.native_auth_complete.v1")?;
            let command: NativeAuthCompleteCommand = decode_request(request)?;
            require_schema(&command.schema, "ghostlight.native_auth_complete.v1")?;
            if command.handle.trim().is_empty() {
                bail!("native auth completion omitted its opaque handle");
            }
            let receipt =
                complete_native_authentication(&state, &command.handle, message_id(request)?)
                    .await?;
            success_response(
                request,
                "ghostlight.native_auth_completion_receipt.v1",
                &receipt,
            )
        }
        NATIVE_SURFACE_GET => {
            require_schema(payload_schema, "ghostlight.native_surface_get.v1")?;
            let command: NativeSurfaceGetCommand = decode_request(request)?;
            require_schema(&command.schema, "ghostlight.native_surface_get.v1")?;
            let headers = session_headers(&command.session_token)?;
            let response = surface(headers, State(state), command.invite.as_deref()).await;
            let value = response_json(response).await?;
            success_response(request, "gamecult.eve.surface.v1", &value)
        }
        NATIVE_EVE_INVOKE => {
            require_schema(payload_schema, "ghostlight.native_eve_invocation.v1")?;
            let command: NativeEveInvokeCommand = decode_request(request)?;
            require_schema(&command.schema, "ghostlight.native_eve_invocation.v1")?;
            let headers = session_headers(&command.session_token)?;
            let invocation: EveCommandInvocation = serde_json::from_value(command.invocation)?;
            let response =
                eve_command_for_transport(headers, state, invocation, "cultnet-rudp").await;
            let value = response_json(response).await?;
            success_response(request, "gamecult.eve.command_result.v1", &value)
        }
        _ => bail!("native operation is not advertised by Ghostlight"),
    }
}

fn require_schema(actual: &str, expected: &str) -> Result<()> {
    if actual != expected {
        bail!("native payload schema does not match the operation");
    }
    Ok(())
}

fn session_headers(session_token: &str) -> Result<HeaderMap> {
    if session_token.trim().is_empty() || session_token.len() > 256 {
        bail!("native session token is malformed");
    }
    let mut headers = HeaderMap::new();
    headers.insert(
        header::COOKIE,
        HeaderValue::from_str(&format!("ghostlight_session={session_token}"))?,
    );
    Ok(headers)
}

async fn response_json(response: Response) -> Result<serde_json::Value> {
    if !response.status().is_success() {
        bail!("Ghostlight product boundary rejected the authenticated request");
    }
    let bytes = axum::body::to_bytes(response.into_body(), 4 * 1024 * 1024).await?;
    Ok(serde_json::from_slice(&bytes)?)
}

fn decode_request<T: DeserializeOwned>(request: &CultNetMessage) -> Result<T> {
    let CultNetMessage::OperationRequest { payload, .. } = request else {
        bail!("expected operation request");
    };
    Ok(rmp_serde::from_slice(&STANDARD.decode(payload)?)?)
}

fn success_response(
    request: &CultNetMessage,
    payload_schema: &str,
    payload: &impl Serialize,
) -> Result<CultNetMessage> {
    operation_response(request, "accepted", payload_schema, payload, Vec::new())
}

fn failure_response(request: &CultNetMessage, code: &str, message: &str) -> Result<CultNetMessage> {
    operation_response(
        request,
        "denied",
        "ghostlight.native_failure.v1",
        &NativeFailure {
            schema: "ghostlight.native_failure.v1".into(),
            code: code.into(),
            message: message.into(),
        },
        vec![message.into()],
    )
}

fn bare_failure_response(request: &CultNetMessage) -> CultNetMessage {
    let (message_id, service_id, operation) = match request {
        CultNetMessage::OperationRequest {
            message_id,
            service_id,
            operation,
            ..
        } => (message_id.clone(), service_id.clone(), operation.clone()),
        _ => (
            uuid::Uuid::new_v4().to_string(),
            NATIVE_SERVICE_ID.into(),
            "unknown".into(),
        ),
    };
    CultNetMessage::OperationResponse {
        message_id,
        service_id,
        operation,
        status: "denied".into(),
        payload_schema: "ghostlight.native_failure.v1".into(),
        payload_encoding: "messagepack-base64".into(),
        payload: String::new(),
        diagnostics: vec!["Ghostlight denied the malformed native request.".into()],
        source_runtime_id: Some(NATIVE_RUNTIME_ID.into()),
    }
}

fn operation_response(
    request: &CultNetMessage,
    status: &str,
    payload_schema: &str,
    payload: &impl Serialize,
    diagnostics: Vec<String>,
) -> Result<CultNetMessage> {
    let CultNetMessage::OperationRequest {
        message_id,
        service_id,
        operation,
        ..
    } = request
    else {
        bail!("expected operation request");
    };
    Ok(CultNetMessage::OperationResponse {
        message_id: message_id.clone(),
        service_id: service_id.clone(),
        operation: operation.clone(),
        status: status.into(),
        payload_schema: payload_schema.into(),
        payload_encoding: "messagepack-base64".into(),
        payload: STANDARD.encode(rmp_serde::to_vec_named(payload)?),
        diagnostics,
        source_runtime_id: Some(NATIVE_RUNTIME_ID.into()),
    })
}

fn message_id(request: &CultNetMessage) -> Result<&str> {
    let CultNetMessage::OperationRequest { message_id, .. } = request else {
        bail!("expected operation request");
    };
    Ok(message_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cultnet_rs::{
        CULTNET_OPERATION_CONNECTION_ID, CultNetRudpSocketTransportConnection,
        CultNetRudpSocketTransportOptions,
    };
    use std::time::Duration;

    #[test]
    fn native_session_payload_rejects_authority_fields() {
        let payload = serde_json::json!({
            "schema":"ghostlight.native_eve_invocation.v1",
            "sessionToken":"opaque",
            "invocation":{
                "schema":"gamecult.eve.command_invocation.v1",
                "providerId":"gamecult.ghostlight.dungeon",
                "surfaceId":"ghostlight.play",
                "operation":{"operationId":"world.speak","schemaId":"ghostlight.world_speak.v1","idempotencyKey":"one","routeHint":{"sourceVersion":1,"transport":"cultnet-rudp"}},
                "payload":{"actor_id":"npc"},
                "issuedAt":"2026-08-23T00:00:00Z",
                "clientId":"native-test",
                "commandBoundary":"ghostlight.eve.commands",
                "receiptSchema":"gamecult.eve.command_result.v1"
            }
        });
        let command = serde_json::from_value::<NativeEveInvokeCommand>(payload).unwrap();
        assert!(super::super::contains_authority_field(
            &command.invocation["payload"]
        ));
    }

    #[test]
    fn native_transport_delivers_large_response_after_long_keepalive_only_operation() {
        let server_socket = UdpSocket::bind("127.0.0.1:0").unwrap();
        server_socket
            .set_read_timeout(Some(Duration::from_millis(5)))
            .unwrap();
        let server_addr = server_socket.local_addr().unwrap();
        let mut hub_options = CultNetRudpServerHubOptions::new(
            NATIVE_RUNTIME_ID,
            server_socket,
            CULTNET_OPERATION_CONNECTION_ID,
        );
        hub_options.max_fragment_bytes = Some(2_048);
        hub_options.max_pending_reliable_packets = Some(4_096);
        let mut hub = CultNetRudpServerHub::new(hub_options).unwrap();

        let client_socket = UdpSocket::bind("127.0.0.1:0").unwrap();
        client_socket
            .set_read_timeout(Some(Duration::from_millis(5)))
            .unwrap();
        let mut client_options = CultNetRudpSocketTransportOptions::client(
            "ghostlight-native-test",
            client_socket,
            server_addr,
            CULTNET_OPERATION_CONNECTION_ID,
        );
        client_options.max_fragment_bytes = Some(2_048);
        client_options.max_pending_reliable_packets = Some(4_096);
        let mut client = CultNetRudpSocketTransportConnection::new(client_options).unwrap();
        client.connect(b"long-operation-session".to_vec()).unwrap();
        let session = (0..20)
            .find_map(|_| match hub.receive_event_once().unwrap() {
                Some(CultNetRudpServerEvent::Connected { session }) => Some(session),
                _ => None,
            })
            .expect("server did not accept native test connection");
        client.receive_once().unwrap();
        assert!(client.connected());

        for heartbeat in 0..64_u32 {
            client.ping(heartbeat.to_be_bytes().to_vec()).unwrap();
            assert!(hub.receive_event_once().unwrap().is_none());
            assert!(client.receive_once().unwrap().is_none());
        }
        assert_eq!(hub.session(session.remote_addr), Some(&session));

        let response = CultNetMessage::OperationResponse {
            message_id: "long-operation".into(),
            service_id: NATIVE_SERVICE_ID.into(),
            operation: NATIVE_EVE_INVOKE.into(),
            status: "accepted".into(),
            payload_schema: "gamecult.eve.command_result.v1".into(),
            payload_encoding: "messagepack-base64".into(),
            payload: "x".repeat(256_000),
            diagnostics: vec![],
            source_runtime_id: Some(NATIVE_RUNTIME_ID.into()),
        };
        hub.send_schema_message(&session, &response).unwrap();

        let received = (0..4_000).find_map(|_| {
            let message = client.receive_schema_message_once().unwrap();
            let _ = hub.receive_event_once().unwrap();
            hub.poll_resends().unwrap();
            message
        });
        assert_eq!(received, Some(response));
    }
}
