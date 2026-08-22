use anyhow::Context;
use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::{HeaderMap, HeaderValue, StatusCode, header},
    response::{
        IntoResponse, Response,
        sse::{Event as SseEvent, KeepAlive, Sse},
    },
    routing::{get, post},
};
use chrono::{DateTime, Utc};
#[cfg(test)]
use ghostlight_dungeon::domain::WorldCompilePreview;
use ghostlight_dungeon::{
    assessor::ActionAssessor,
    compiler::{GestaltFissionRequest, OpeningRequest, OpeningSuggestion, WorldCompiler},
    domain::{
        ActionIntent, Campaign, GestaltFissionPreview, NarrationProjection, RegionExpansionPreview,
        RejectedProposalReceipt, WorldCommand,
    },
    gestalt::GestaltPresencePlanner,
    idunn_health::{GHOSTLIGHT_IDUNN_HEALTH_CONTRACT, IdunnHealthPublisher},
    kernel::{CommandResult, KernelError},
    mesh::{
        COMMAND_BOUNDARY as EVE_COMMAND_BOUNDARY, COMMAND_RESULT_SCHEMA as EVE_RESULT_SCHEMA,
        CampaignMeshSnapshot, MeshPublisher, MeshRuntimeIdentity, PROVIDER_ID as EVE_PROVIDER_ID,
        SURFACE_ID as EVE_SURFACE_ID, SessionZeroMeshSnapshot,
    },
    model::{
        DeepSeekPort, MODEL_CAPABLE, MODEL_FAST, ModelPort, ModelRuntimeStatus, ModelStageRequest,
        OpenRouterPort, run_validated_stage,
    },
    narrator::Narrator,
    persistence::CampaignStore,
    persona::PersonaProjectionEngine,
    registry::{CampaignRegistry, CampaignRuntime},
    session_zero::{
        BoundaryLevel, EntitlementPort, FixtureEntitlementPort, SessionZeroCommand,
        SessionZeroDirector, SessionZeroRegistry, SessionZeroState, publication_from_session,
        session_zero_surface,
    },
    surface::player_surface_for_actor,
    turn::{SnapshotPermit, appraise_present},
    vault::VoidBotMcpVault,
};
use serde::Deserialize;
use serde::Serialize;
use std::{
    collections::{BTreeMap, BTreeSet},
    convert::Infallible,
    net::SocketAddr,
    path::PathBuf,
    sync::Arc,
    sync::atomic::{AtomicUsize, Ordering},
};
use tokio::sync::{Mutex, Notify, OwnedRwLockReadGuard, RwLock};
use tower_http::services::ServeDir;
use tracing_subscriber::EnvFilter;

mod app_session;
mod heimdall;
use app_session::{AppSessionOwner, CommandReservation, NewSession, RefreshedSession, secret_hash};
use heimdall::HeimdallClient;

#[derive(Clone)]
struct AppState {
    registry: CampaignRegistry,
    session_zeros: SessionZeroRegistry,
    session_zero_director: Option<Arc<SessionZeroDirector>>,
    entitlements: Arc<dyn EntitlementPort>,
    auth: Arc<Mutex<AppSessionOwner>>,
    heimdall: Arc<HeimdallClient>,
    model_status: ModelRuntimeStatus,
    compiler: Option<Arc<WorldCompiler>>,
    assessor: Option<Arc<ActionAssessor>>,
    model: Option<Arc<dyn ModelPort>>,
    expansion_previews: Arc<Mutex<BTreeMap<String, OwnedPreview<RegionExpansionPreview>>>>,
    fission_previews: Arc<Mutex<BTreeMap<String, OwnedFissionPreview>>>,
    live_turns: Arc<AtomicUsize>,
    live_turn_started: Arc<Notify>,
    live_commit_gate: Arc<RwLock<()>>,
    mesh: MeshPublisher,
}

struct LiveTurnGuard {
    counter: Arc<AtomicUsize>,
    _commit_read: OwnedRwLockReadGuard<()>,
}
impl LiveTurnGuard {
    async fn enter(state: &AppState) -> Self {
        let commit_read = state.live_commit_gate.clone().read_owned().await;
        state.live_turns.fetch_add(1, Ordering::SeqCst);
        state.live_turn_started.notify_waiters();
        Self {
            counter: state.live_turns.clone(),
            _commit_read: commit_read,
        }
    }
}
impl Drop for LiveTurnGuard {
    fn drop(&mut self) {
        self.counter.fetch_sub(1, Ordering::SeqCst);
    }
}

#[derive(Clone)]
struct OwnedPreview<T> {
    session_hash: String,
    value: T,
    model_receipts: Vec<ghostlight_dungeon::model::ModelStageReceipt>,
}

#[derive(Clone)]
struct OwnedFissionPreview {
    session_hash: String,
    value: GestaltFissionPreview,
    evidence_receipts: Vec<ghostlight_dungeon::domain::VaultEvidenceReceipt>,
    model_receipts: Vec<ghostlight_dungeon::model::ModelStageReceipt>,
}

#[derive(Deserialize)]
struct BeginSessionZeroRequest {
    name: String,
    vault_provider: String,
    display_name: String,
}

#[derive(Deserialize)]
struct SessionZeroInviteRequest {
    count: u8,
}

#[derive(Deserialize)]
struct JoinSessionZeroRequest {
    display_name: String,
}

#[derive(Deserialize)]
struct SessionZeroMessageRequest {
    expected_revision: u64,
    channel_id: String,
    text: String,
}

#[derive(Deserialize)]
struct SessionZeroBoundaryRequest {
    expected_revision: u64,
    boundary_id: Option<String>,
    topic: String,
    level: BoundaryLevel,
}

#[derive(Deserialize)]
struct SessionZeroDecisionRequest {
    expected_revision: u64,
    decision_id: String,
    accept: bool,
    counter: Option<String>,
}

#[derive(Deserialize)]
struct SessionZeroMemberRequest {
    expected_revision: u64,
    member_id: String,
}

#[derive(Deserialize)]
struct SessionZeroRevisionRequest {
    expected_revision: u64,
}

#[derive(Deserialize)]
struct TimeAdvanceRequest {
    expected_revision: u64,
    minutes: u32,
}

#[derive(Deserialize)]
struct GroupTravelRequest {
    expected_revision: u64,
    destination_location_id: String,
}

#[derive(Deserialize)]
struct CellBudgetRequest {
    expected_revision: u64,
    expected_resolution_epoch: u64,
    active_cell_budget: u8,
}

#[derive(Clone, Serialize, Deserialize)]
struct LegacyAuthState {
    schema: String,
    session_hashes: BTreeSet<String>,
    #[serde(default)]
    session_aliases: BTreeMap<String, String>,
    #[serde(default)]
    session_campaigns: BTreeMap<String, uuid::Uuid>,
    #[serde(default)]
    session_campaign_ids: BTreeMap<String, BTreeSet<uuid::Uuid>>,
    #[serde(default)]
    heimdall_attempts: BTreeMap<String, serde_json::Value>,
}

fn empty_legacy_auth_state() -> LegacyAuthState {
    LegacyAuthState {
        schema: "ghostlight.auth_state.v1".into(),
        session_hashes: BTreeSet::new(),
        session_aliases: BTreeMap::new(),
        session_campaigns: BTreeMap::new(),
        session_campaign_ids: BTreeMap::new(),
        heimdall_attempts: BTreeMap::new(),
    }
}

#[derive(Clone, Serialize, Deserialize)]
struct GovernanceMigrationReceipt {
    schema: String,
    campaign_id: uuid::Uuid,
    status: String,
    account_hashes: Vec<String>,
    actor_id: String,
    created_at: chrono::DateTime<chrono::Utc>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();
    let runtime_root = std::env::var_os("GHOSTLIGHT_DUNGEON_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(default_runtime_root);
    migrate_default_campaign(&runtime_root)?;
    let registry = CampaignRegistry::new(runtime_root.join("campaigns"))?;
    registry.load_existing().await?;
    let session_zeros = SessionZeroRegistry::new(runtime_root.join("session-zero"))?;
    session_zeros.load_existing().await?;
    std::fs::create_dir_all(runtime_root.join("service"))?;
    let legacy_auth_path = runtime_root.join("service/auth.cc");
    let legacy_auth_state = if legacy_auth_path.is_file() {
        CampaignStore::open(&legacy_auth_path)?
            .load::<LegacyAuthState>("auth_state.v1", "primary")?
            .map(|(_, state)| state)
            .unwrap_or_else(empty_legacy_auth_state)
    } else {
        empty_legacy_auth_state()
    };
    migrate_legacy_campaign_memberships(&registry, &legacy_auth_state).await?;
    let session_key_path = std::env::var_os("GHOSTLIGHT_SESSION_WRAPPING_KEY_FILE")
        .map(PathBuf::from)
        .unwrap_or_else(|| runtime_root.join("secrets/session-wrapping.key"));
    let mut app_sessions = AppSessionOwner::open(
        runtime_root.join("service/app-sessions.cc"),
        session_key_path,
    )?;
    for (account_hash, campaign_id) in &legacy_auth_state.session_campaigns {
        app_sessions.migrate_preference(account_hash, *campaign_id)?;
    }
    let provider_name = std::env::var("GHOSTLIGHT_MODEL_PROVIDER")
        .unwrap_or_else(|_| "deepseek".into())
        .to_ascii_lowercase();
    let (default_fast_model, default_capable_model, default_secret_name) =
        match provider_name.as_str() {
            "deepseek" => ("deepseek-v4-flash", "deepseek-v4-pro", "deepseek.dpapi"),
            "openrouter" => ("stealth/ox-alpha", "stealth/ox-alpha", "openrouter.key"),
            unsupported => anyhow::bail!("unsupported model provider {unsupported}"),
        };
    let fast_model =
        std::env::var("GHOSTLIGHT_MODEL_FAST").unwrap_or_else(|_| default_fast_model.into());
    let capable_model =
        std::env::var("GHOSTLIGHT_MODEL_CAPABLE").unwrap_or_else(|_| default_capable_model.into());
    let secret_path = std::env::var_os("GHOSTLIGHT_MODEL_CREDENTIAL")
        .map(PathBuf::from)
        .unwrap_or_else(|| runtime_root.join("secrets").join(default_secret_name));
    let configured_status = |readiness: String| ModelRuntimeStatus {
        provider: provider_name.clone(),
        fast_model: fast_model.clone(),
        capable_model: capable_model.clone(),
        readiness,
    };
    let (model_status, compiler, assessor, shared_model) = if secret_path.is_file() {
        let provider: Arc<dyn ModelPort> = match provider_name.as_str() {
            "deepseek" => Arc::new(DeepSeekPort::from_runtime_secret_with_models(
                &secret_path,
                fast_model.clone(),
                capable_model.clone(),
            )?),
            "openrouter" => Arc::new(OpenRouterPort::from_runtime_secret(
                &secret_path,
                fast_model.clone(),
                capable_model.clone(),
            )?),
            _ => unreachable!("provider name was validated above"),
        };
        let probe = run_validated_stage(
            provider.as_ref(),
            &ModelStageRequest {
                stage: "startup_probe".into(),
                model: MODEL_FAST.into(),
                snapshot_binding: "service-startup".into(),
                lived_stream: "Reply with the single word ready.".into(),
                output_schema: None,
                source_receipt_ids: vec![],
                temperature: Some(0.0),
                max_output_tokens: Some(128),
            },
        )
        .await;
        let readiness = match probe {
            Ok(probe) => format!("ready:{}", probe.receipt.output_hash),
            Err(error) => {
                tracing::warn!(provider = %provider_name, %error, "model startup probe failed; runtime remains available");
                "degraded:startup-probe-failed".into()
            }
        };
        let vault_endpoint = std::env::var("GHOSTLIGHT_VAULT_MCP_URL")
            .unwrap_or_else(|_| "http://127.0.0.1:17875/mcp".into());
        (
            configured_status(readiness),
            Some(Arc::new(WorldCompiler::new(
                Arc::new(VoidBotMcpVault::new(vault_endpoint)),
                provider.clone(),
                MODEL_FAST,
                MODEL_CAPABLE,
            ))),
            Some(Arc::new(ActionAssessor::new(
                provider.clone(),
                MODEL_CAPABLE,
            ))),
            Some(provider),
        )
    } else {
        (configured_status("missing-secret".into()), None, None, None)
    };
    let mesh_target = std::env::var("GHOSTLIGHT_ODIN_RUDP")
        .ok()
        .map(|value| value.parse())
        .transpose()?;
    let mesh_identity = MeshRuntimeIdentity {
        runtime_id: std::env::var("GHOSTLIGHT_RUNTIME_ID")
            .unwrap_or_else(|_| "ghostlight-dungeon-starfire".into()),
        service_id: std::env::var("GHOSTLIGHT_SERVICE_ID")
            .unwrap_or_else(|_| "ghostlight-dungeon-starfire".into()),
        located_service: std::env::var("GHOSTLIGHT_LOCATED_SERVICE")
            .unwrap_or_else(|_| "starfire".into()),
    };
    let mesh = MeshPublisher::open_with_identity(
        runtime_root.join("service/mesh.cc"),
        mesh_target,
        mesh_identity.clone(),
    )?;
    let idunn_health =
        std::env::var("GHOSTLIGHT_IDUNN_RUDP")
            .ok()
            .map(|endpoint| -> anyhow::Result<IdunnHealthPublisher> {
                let identity_store = std::env::var_os("GHOSTLIGHT_IDUNN_HEALTH_IDENTITY")
                .map(PathBuf::from)
                .ok_or_else(|| anyhow::anyhow!(
                    "GHOSTLIGHT_IDUNN_HEALTH_IDENTITY is required with GHOSTLIGHT_IDUNN_RUDP"
                ))?;
                Ok(IdunnHealthPublisher::open(
                    endpoint.parse()?,
                    std::env::var("GHOSTLIGHT_IDUNN_DAEMON")
                        .unwrap_or_else(|_| "yggdrasil-ghostlight".into()),
                    mesh_identity.runtime_id.clone(),
                    std::env::var("GHOSTLIGHT_IDUNN_HEALTH_CONTRACT")
                        .unwrap_or_else(|_| GHOSTLIGHT_IDUNN_HEALTH_CONTRACT.into()),
                    identity_store,
                )?)
            })
            .transpose()?;
    let session_zero_director = shared_model.as_ref().map(|model| {
        Arc::new(SessionZeroDirector::new(
            model.clone(),
            MODEL_FAST,
            MODEL_CAPABLE,
            MODEL_FAST,
        ))
    });
    let state = AppState {
        registry,
        session_zeros,
        session_zero_director,
        entitlements: Arc::new(FixtureEntitlementPort),
        auth: Arc::new(Mutex::new(app_sessions)),
        heimdall: Arc::new(HeimdallClient::from_env()?),
        model_status,
        compiler,
        assessor,
        model: shared_model,
        expansion_previews: Arc::new(Mutex::new(BTreeMap::new())),
        fission_previews: Arc::new(Mutex::new(BTreeMap::new())),
        live_turns: Arc::new(AtomicUsize::new(0)),
        live_turn_started: Arc::new(Notify::new()),
        live_commit_gate: Arc::new(RwLock::new(())),
        mesh,
    };
    refresh_mesh(&state).await?;
    tokio::spawn(scheduler_loop(state.clone()));
    tokio::spawn(app_session_refresh_loop(state.clone()));
    let release_web_root = std::env::current_exe()?
        .parent()
        .map(|parent| parent.join("web"));
    let web_root = release_web_root
        .filter(|path| path.join("index.html").is_file())
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../web/dist"));
    let app = app_router(state, web_root);
    let address: SocketAddr = std::env::var("GHOSTLIGHT_DUNGEON_BIND")
        .unwrap_or_else(|_| "0.0.0.0:8831".into())
        .parse()?;
    let listener = tokio::net::TcpListener::bind(address).await?;
    tracing::info!(%address, "GhostlightDungeon listening");
    if let Some(mut publisher) = idunn_health {
        tokio::task::spawn_blocking(move || {
            loop {
                if let Err(error) = publisher.publish("active", "world-kernel-serving") {
                    tracing::warn!(%error, "signed Idunn health publication failed");
                }
                std::thread::sleep(std::time::Duration::from_secs(10));
            }
        });
    }
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;
    Ok(())
}

fn default_runtime_root() -> PathBuf {
    #[cfg(windows)]
    {
        PathBuf::from(r"F:\GameCult\GhostlightDungeon")
    }
    #[cfg(not(windows))]
    {
        PathBuf::from("/var/lib/gamecult/ghostlight-dungeon")
    }
}

fn app_router(state: AppState, web_root: PathBuf) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/api/eve/provider", get(eve_provider))
        .route("/api/eve/surfaces/{surface_id}", get(eve_surface))
        .route("/api/eve/commands", post(eve_command))
        .route("/api/eve/events", get(revision_events))
        .fallback_service(ServeDir::new(web_root).append_index_html_on_directories(true))
        .with_state(state)
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct EveCommandInvocation {
    schema: String,
    #[serde(rename = "providerId")]
    provider_id: String,
    #[serde(rename = "surfaceId")]
    surface_id: String,
    operation: EveOperation,
    payload: serde_json::Value,
    #[serde(rename = "issuedAt")]
    issued_at: String,
    #[serde(rename = "clientId")]
    client_id: String,
    #[serde(rename = "commandBoundary")]
    command_boundary: String,
    #[serde(rename = "receiptSchema")]
    receipt_schema: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct EveOperation {
    operation_id: String,
    schema_id: Option<String>,
    idempotency_key: Option<String>,
    #[serde(default)]
    route_hint: EveRouteHint,
}

#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct EveRouteHint {
    source_version: Option<u64>,
    transport: Option<String>,
}

async fn eve_provider(State(state): State<AppState>) -> Response {
    let updated = chrono::Utc::now().to_rfc3339();
    Json(state.mesh.provider_advertisement(&updated)).into_response()
}

async fn eve_surface(
    Path(surface_id): Path<String>,
    Query(query): Query<EveSurfaceQuery>,
    headers: HeaderMap,
    State(state): State<AppState>,
) -> Response {
    if surface_id != EVE_SURFACE_ID {
        return StatusCode::NOT_FOUND.into_response();
    }
    if authenticated_session(&headers, &state).await.is_none() {
        return Json(anonymous_eve_surface()).into_response();
    }
    surface(headers, State(state), query.invite.as_deref()).await
}

#[derive(Default, Deserialize)]
struct EveSurfaceQuery {
    invite: Option<String>,
}

fn anonymous_eve_surface() -> serde_json::Value {
    serde_json::json!({
        "type":"surface-state",
        "schema":"gamecult.eve.surface.v1",
        "providerId":EVE_PROVIDER_ID,
        "providerKind":"narrative.simulation",
        "title":"Ghostlight Dungeon",
        "version":0,
        "updatedAtUtc":Utc::now().to_rfc3339(),
        "surface":{"id":EVE_SURFACE_ID,"root":{
            "id":"ghostlight.root","kind":"surface","props":{},"children":[
                {"id":"ghostlight.access","kind":"heimdall.access_gate","props":{
                    "state":"anonymous","title":"Enter Ghostlight","detail":"Sign in with Discord. Access currently requires the KLTST GameCult role."
                },"children":[]},
                {"id":"ghostlight.auth.begin","kind":"control.button","props":{
                    "label":"Continue with Discord","command":"heimdall.auth.begin"
                },"children":[]}
            ]
        },"styles":{"tokens":{"colorBackground":"#0c1110","colorPanel":"#17201d","colorText":"#e8e1cf","colorMuted":"#9aa69f","colorAccent":"#d49b58"}}},
        "commands":[{
            "schema":"gamecult.eve.command.v1","command":"heimdall.auth.begin",
            "payloadSchema":"heimdall.auth_begin_command.v1",
            "transport":"https-json","authority":"Heimdall"
        }]
    })
}

async fn eve_command(
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(invocation): Json<EveCommandInvocation>,
) -> Response {
    if let Err(error) = validate_eve_invocation(&invocation) {
        return Json(eve_result(
            &invocation,
            "denied",
            error.to_string(),
            None,
            None,
            None,
        ))
        .into_response();
    }
    let operation = invocation.operation.operation_id.as_str();
    if operation == "heimdall.auth.begin" {
        return match state.heimdall.begin(invocation.operation.idempotency_key.as_deref().unwrap()).await {
            Ok(receipt) if receipt.status == "pending"
                && !receipt.handle.is_empty()
                && receipt.expires_at.parse::<DateTime<Utc>>().is_ok_and(|expiry| expiry > Utc::now()) =>
            {
                Json(eve_result(
                    &invocation,
                    "accepted",
                    "Continue authentication with Heimdall.".into(),
                    None,
                    Some(serde_json::json!({
                        "pluginId":"gamecult.heimdall.access",
                        "schemaId":"heimdall.auth_navigation_receipt.v1",
                        "payload":{
                            "schema":"heimdall.auth_navigation_receipt.v1",
                            "handle":receipt.handle,
                            "navigation":{"url":receipt.navigation.url,"allowedOrigins":receipt.navigation.allowed_origins}
                        }
                    })),
                    None,
                )).into_response()
            }
            Ok(_) => Json(eve_result(&invocation, "denied", "Heimdall returned an invalid authentication attempt.".into(), None, None, None)).into_response(),
            Err(error) => Json(eve_result(&invocation, "denied", error.to_string(), None, None, None)).into_response(),
        };
    }
    if operation == "heimdall.auth.complete" {
        return complete_eve_authentication(invocation, state).await;
    }
    let Some(account_hash) = authenticated_session(&headers, &state).await else {
        return Json(eve_result(
            &invocation,
            "denied",
            "Authentication is required.".into(),
            None,
            None,
            None,
        ))
        .into_response();
    };
    if operation == "app.auth.logout" {
        let raw = cookie_value(&headers).unwrap_or_default();
        let logout_session = state
            .auth
            .lock()
            .await
            .session_for_logout(raw)
            .ok()
            .flatten();
        let _ = state.auth.lock().await.revoke_cookie(raw);
        if let Some(session) = logout_session {
            let idempotency_key = format!(
                "logout:{}:{}",
                session.heimdall_session_id, session.access_revision
            );
            let heimdall = state.heimdall.clone();
            tokio::spawn(async move {
                match heimdall
                    .logout(&session.refresh_claim, &idempotency_key)
                    .await
                {
                    Ok(receipt)
                        if receipt.status == "revoked"
                            && receipt.session_id == session.heimdall_session_id
                            && receipt.access_revision > session.access_revision
                            && receipt.revoked_at.parse::<DateTime<Utc>>().is_ok() => {}
                    Ok(_) => tracing::warn!(
                        session_id = %session.heimdall_session_id,
                        "Heimdall returned an invalid logout receipt; local session is already revoked"
                    ),
                    Err(error) => tracing::warn!(
                        %error,
                        session_id = %session.heimdall_session_id,
                        "Heimdall logout transport failed; local session is already revoked"
                    ),
                }
            });
        }
        let mut response = Json(eve_result(
            &invocation,
            "accepted",
            "Signed out.".into(),
            None,
            Some(serde_json::json!({
                "pluginId":"gamecult.heimdall.access",
                "schemaId":"heimdall.auth_completion_status.v1",
                "payload":{"schema":"heimdall.auth_completion_status.v1","status":"anonymous"}
            })),
            None,
        ))
        .into_response();
        response.headers_mut().insert(
            header::SET_COOKIE,
            HeaderValue::from_static(
                "ghostlight_session=; Max-Age=0; HttpOnly; Secure; SameSite=Lax; Path=/ghostlight/",
            ),
        );
        return response;
    }
    let current_version = match current_eve_surface_version(&state, &account_hash).await {
        Ok(value) => value,
        Err(error) => {
            return Json(eve_result(
                &invocation,
                "denied",
                error.to_string(),
                None,
                None,
                None,
            ))
            .into_response();
        }
    };
    if invocation.operation.route_hint.source_version != Some(current_version) {
        return Json(eve_result(
            &invocation,
            "denied",
            format!(
                "Stale Eve surface: expected version {current_version}. Refresh before retrying."
            ),
            None,
            None,
            None,
        ))
        .into_response();
    }
    let idempotency_key = invocation.operation.idempotency_key.clone().unwrap();
    match state.auth.lock().await.reserve_command(
        &account_hash,
        &idempotency_key,
        &invocation.operation.operation_id,
    ) {
        Ok(CommandReservation::Cached(result)) => return Json(result).into_response(),
        Ok(CommandReservation::Pending) => {
            return Json(eve_result(
                &invocation,
                "pending",
                "This exact operation is already reserved. Refresh authoritative state; Ghostlight will not execute it twice.".into(),
                None,
                None,
                None,
            )).into_response();
        }
        Ok(CommandReservation::Reserved) => {}
        Err(error) => {
            return Json(eve_result(
                &invocation,
                "denied",
                error.to_string(),
                None,
                None,
                None,
            ))
            .into_response();
        }
    }
    let operation_id = invocation.operation.operation_id.clone();
    let response = dispatch_eve_product_command(&headers, &state, &account_hash, invocation).await;
    persist_reserved_eve_result(
        &state,
        &account_hash,
        &idempotency_key,
        &operation_id,
        response,
    )
    .await
}

async fn persist_reserved_eve_result(
    state: &AppState,
    account_hash: &str,
    idempotency_key: &str,
    operation_id: &str,
    response: Response,
) -> Response {
    let status = response.status();
    let bytes = match axum::body::to_bytes(response.into_body(), 2 * 1024 * 1024).await {
        Ok(value) => value,
        Err(error) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()).into_response();
        }
    };
    let result = match serde_json::from_slice::<serde_json::Value>(&bytes) {
        Ok(value) => value,
        Err(error) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Eve command boundary returned a non-typed result: {error}"),
            )
                .into_response();
        }
    };
    if let Err(error) = state.auth.lock().await.record_command_result(
        account_hash,
        idempotency_key,
        operation_id,
        &result,
    ) {
        tracing::error!(%error, %operation_id, "Eve command committed but its reserved result could not be finalized; duplicate execution remains blocked");
    }
    (status, Json(result)).into_response()
}

async fn current_eve_surface_version(state: &AppState, account_hash: &str) -> anyhow::Result<u64> {
    if let Some(id) = state
        .session_zeros
        .active_contract_review_for_account(account_hash)
        .await?
    {
        return Ok(state.session_zeros.snapshot(id).await?.revision);
    }
    if let Some(runtime) = session_runtime(state, account_hash).await? {
        let campaign = load_campaign(&runtime.store)?;
        return Ok(campaign
            .revision
            .saturating_mul(1_000_000_000_000)
            .saturating_add(
                campaign
                    .resolution_policy
                    .resolution_epoch
                    .saturating_mul(1_000_000),
            )
            .saturating_add(campaign.resolution_policy.provider_configuration_epoch));
    }
    if let Some(id) = state
        .session_zeros
        .session_for_account(account_hash)
        .await?
    {
        return Ok(state.session_zeros.snapshot(id).await?.revision);
    }
    Ok(0)
}

async fn complete_eve_authentication(
    invocation: EveCommandInvocation,
    state: AppState,
) -> Response {
    let handle = invocation
        .payload
        .get("handle")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("");
    if handle.is_empty() {
        return Json(eve_result(
            &invocation,
            "denied",
            "Authentication completion omitted its opaque handle.".into(),
            None,
            None,
            None,
        ))
        .into_response();
    }
    let completion = match state
        .heimdall
        .complete(
            handle,
            invocation.operation.idempotency_key.as_deref().unwrap(),
        )
        .await
    {
        Ok(value) => value,
        Err(error) => {
            return Json(eve_result(
                &invocation,
                "denied",
                error.to_string(),
                None,
                None,
                None,
            ))
            .into_response();
        }
    };
    if completion.status == "pending" {
        return Json(eve_result(&invocation, "pending", "Heimdall is waiting for Discord.".into(), None, Some(serde_json::json!({
            "pluginId":"gamecult.heimdall.access","schemaId":"heimdall.auth_completion_status.v1",
            "payload":{"schema":"heimdall.auth_completion_status.v1","status":"pending"}
        })), None)).into_response();
    }
    if completion.status == "denied" {
        return Json(eve_result(
            &invocation,
            "denied",
            completion.error.clone().unwrap_or_else(|| "Heimdall denied access.".into()),
            None,
            Some(serde_json::json!({
                "pluginId":"gamecult.heimdall.access","schemaId":"heimdall.auth_completion_status.v1",
                "payload":{"schema":"heimdall.auth_completion_status.v1","status":"denied"}
            })),
            None,
        )).into_response();
    }
    if completion.handle.as_deref() != Some(handle) {
        return Json(eve_result(
            &invocation,
            "denied",
            "Heimdall completion handle did not match the browser-held attempt.".into(),
            None,
            None,
            None,
        ))
        .into_response();
    }
    let claims = match state.heimdall.verify_completion(&completion).await {
        Ok(value) => value,
        Err(error) => return Json(eve_result(&invocation, "denied", error.to_string(), None, Some(serde_json::json!({
            "pluginId":"gamecult.heimdall.access","schemaId":"heimdall.auth_completion_status.v1",
            "payload":{"schema":"heimdall.auth_completion_status.v1","status":"denied"}
        })), None)).into_response(),
    };
    let session = completion.session.as_ref().unwrap();
    if !completion
        .shared_capabilities
        .iter()
        .any(|value| value == "app_access")
    {
        return Json(eve_result(
            &invocation,
            "denied",
            "Heimdall completion did not expose Ghostlight app_access.".into(),
            None,
            None,
            None,
        ))
        .into_response();
    }
    let refresh = completion
        .refresh
        .as_ref()
        .map(|value| value.expires_at.as_str())
        .unwrap_or(&session.expires_at);
    let refresh_token = completion.refresh_token.as_deref().unwrap_or("");
    if refresh_token.is_empty() {
        return Json(eve_result(
            &invocation,
            "denied",
            "Heimdall omitted its refresh claim.".into(),
            None,
            None,
            None,
        ))
        .into_response();
    }
    let refresh_expires_at = refresh.parse().unwrap_or_else(|_| Utc::now());
    let raw_cookie = match state.auth.lock().await.create_session(NewSession {
        account_id: &claims.account_id,
        heimdall_session_id: &claims.sid,
        access_revision: claims.access_revision,
        capabilities: claims.capabilities.clone(),
        access_expires_at: DateTime::from_timestamp(claims.exp as i64, 0).unwrap_or_else(Utc::now),
        refresh_expires_at,
        refresh_claim: refresh_token,
    }) {
        Ok(value) => value,
        Err(error) => {
            return Json(eve_result(
                &invocation,
                "denied",
                error.to_string(),
                None,
                None,
                None,
            ))
            .into_response();
        }
    };
    let mut response = Json(eve_result(
        &invocation,
        "accepted",
        "Authenticated.".into(),
        None,
        Some(serde_json::json!({
            "pluginId":"gamecult.heimdall.access","schemaId":"heimdall.auth_completion_status.v1",
            "payload":{"schema":"heimdall.auth_completion_status.v1","status":"authenticated"}
        })),
        None,
    ))
    .into_response();
    response.headers_mut().insert(
        header::SET_COOKIE,
        app_session_cookie(&raw_cookie, refresh_expires_at, Utc::now()),
    );
    response
}

fn app_session_cookie(
    raw_cookie: &str,
    refresh_expires_at: DateTime<Utc>,
    now: DateTime<Utc>,
) -> HeaderValue {
    let max_age = (refresh_expires_at - now).num_seconds().max(0);
    HeaderValue::from_str(&format!(
        "ghostlight_session={raw_cookie}; Max-Age={max_age}; HttpOnly; Secure; SameSite=Lax; Path=/ghostlight/"
    ))
    .expect("Ghostlight generated an invalid app-session cookie")
}

fn validate_eve_invocation(invocation: &EveCommandInvocation) -> anyhow::Result<()> {
    if invocation.schema != "gamecult.eve.command_invocation.v1"
        || invocation.provider_id != EVE_PROVIDER_ID
        || invocation.surface_id != EVE_SURFACE_ID
        || invocation.command_boundary != EVE_COMMAND_BOUNDARY
        || invocation.receipt_schema != EVE_RESULT_SCHEMA
    {
        anyhow::bail!("Invocation does not match the Ghostlight Eve provider boundary");
    }
    if invocation.operation.operation_id.trim().is_empty()
        || invocation
            .operation
            .idempotency_key
            .as_deref()
            .is_none_or(str::is_empty)
        || invocation
            .operation
            .schema_id
            .as_deref()
            .is_none_or(str::is_empty)
        || invocation.client_id.trim().is_empty()
        || invocation.issued_at.parse::<DateTime<Utc>>().is_err()
    {
        anyhow::bail!("Invocation metadata is incomplete");
    }
    if contains_authority_field(&invocation.payload) {
        anyhow::bail!("Browser payloads may not supply actor, member, or account authority");
    }
    if invocation
        .operation
        .route_hint
        .transport
        .as_deref()
        .is_some_and(|value| value != "https-json")
    {
        anyhow::bail!(
            "Ghostlight's browser boundary accepts only its advertised HTTPS Eve transport"
        );
    }
    let expected_schema = eve_operation_schema(&invocation.operation.operation_id)
        .ok_or_else(|| anyhow::anyhow!("Eve operation is not advertised by Ghostlight"))?;
    if invocation.operation.schema_id.as_deref() != Some(expected_schema) {
        anyhow::bail!(
            "Eve operation payload schema does not match the advertised command descriptor"
        );
    }
    Ok(())
}

fn eve_operation_schema(operation: &str) -> Option<&'static str> {
    Some(match operation {
        "heimdall.auth.begin" => "heimdall.auth_begin_command.v1",
        "heimdall.auth.complete" => "heimdall.auth_complete_command.v1",
        "app.auth.logout" => "ghostlight.app_logout.v1",
        "session_zero.begin" => "ghostlight.session_zero_begin.v1",
        "session_zero.join" => "ghostlight.session_zero_join.v1",
        "session_zero.invites.create" => "ghostlight.session_zero_invites_create.v1",
        "session_zero.message.send" => "ghostlight.session_zero_message_send.v1",
        "session_zero.boundary.set" => "ghostlight.session_zero_boundary_set.v1",
        "session_zero.boundary.remove" => "ghostlight.session_zero_boundary_remove.v1",
        "session_zero.leave"
        | "session_zero.roster.lock"
        | "session_zero.compile"
        | "session_zero.approve"
        | "session_zero.publish" => "ghostlight.session_zero_revision_command.v1",
        "session_zero.member.remove" => "ghostlight.session_zero_member_remove.v1",
        "session_zero.decision.resolve" => "ghostlight.session_zero_decision_resolve.v1",
        "campaign.select" => "ghostlight.campaign_select.v1",
        "campaign.contract_review.begin" => "ghostlight.contract_review_begin.v1",
        "world.speak" => "ghostlight.world_speak.v1",
        "world.assess" => "ghostlight.player_action_assess.v1",
        "world.attempt" => "ghostlight.player_action_attempt.v1",
        "world.wait" => "ghostlight.world_wait.v1",
        "governance.time.propose" => "ghostlight.time_advance_proposal.v1",
        "governance.time.approve" => "ghostlight.time_advance_approval.v1",
        "governance.travel.propose" => "ghostlight.group_travel_proposal.v1",
        "governance.travel.approve" => "ghostlight.group_travel_approval.v1",
        "governance.cells.propose" => "ghostlight.cell_budget_proposal.v1",
        "governance.cells.approve" => "ghostlight.cell_budget_approval.v1",
        "world.destination.compile" => "ghostlight.destination_compile.v1",
        "world.destination.approve" => "ghostlight.destination_approval.v1",
        "world.gestalt.fission.compile" => "ghostlight.gestalt_fission_compile.v1",
        "world.gestalt.fission.approve" => "ghostlight.gestalt_fission_approval.v1",
        _ => return None,
    })
}

fn contains_authority_field(value: &serde_json::Value) -> bool {
    match value {
        serde_json::Value::Object(values) => values.iter().any(|(key, value)| {
            matches!(
                key.as_str(),
                "actor_id"
                    | "actorId"
                    | "member_id"
                    | "memberId"
                    | "account_hash"
                    | "accountHash"
                    | "viewer_actor_id"
            ) || contains_authority_field(value)
        }),
        serde_json::Value::Array(values) => values.iter().any(contains_authority_field),
        _ => false,
    }
}

fn eve_result(
    invocation: &EveCommandInvocation,
    state: &str,
    message: String,
    transient_projection: Option<serde_json::Value>,
    plugin_payload: Option<serde_json::Value>,
    clear_bindings: Option<Vec<String>>,
) -> serde_json::Value {
    let source_version = invocation.operation.route_hint.source_version.unwrap_or(0);
    let mut value = serde_json::json!({
        "schema":EVE_RESULT_SCHEMA,
        "receipt":{
            "schema":"gamecult.eve.command_receipt.v1",
            "receiptId":format!("eve-receipt:{}",uuid::Uuid::new_v4()),
            "commandId":invocation.operation.idempotency_key.as_deref().unwrap_or(""),
            "command":invocation.operation.operation_id,
            "state":state,
            "ownerRepo":"GameCult/Ghostlight",
            "authority":"SessionZeroKernel or WorldKernel after server-side membership resolution",
            "providerId":EVE_PROVIDER_ID,
            "surfaceId":EVE_SURFACE_ID,
            "message":message,
            "diagnostics":[],
            "issuedAtUtc":Utc::now().to_rfc3339(),
            "sourceVersion":source_version
        }
    });
    if let Some(transient) = transient_projection {
        value["transientProjection"] = transient;
    }
    if let Some(plugin) = plugin_payload {
        value["pluginPayload"] = plugin;
    }
    if let Some(names) = clear_bindings {
        value["draftDirective"] = serde_json::json!({"clear":true,"bindingNames":names});
    }
    value
}

async fn dispatch_eve_product_command(
    headers: &HeaderMap,
    state: &AppState,
    account_hash: &str,
    invocation: EveCommandInvocation,
) -> Response {
    let operation = invocation.operation.operation_id.clone();
    let payload = captured_payload(&invocation.payload);
    macro_rules! required_string {
        ($name:literal) => {
            match string_field(&payload, $name) {
                Some(value) => value,
                None => {
                    return invalid_eve_payload(
                        &invocation,
                        anyhow::anyhow!(concat!($name, " is required")),
                    )
                }
            }
        };
    }
    macro_rules! required_u64 {
        ($name:literal) => {
            match u64_field(&payload, $name) {
                Some(value) => value,
                None => {
                    return invalid_eve_payload(
                        &invocation,
                        anyhow::anyhow!(concat!($name, " is required")),
                    )
                }
            }
        };
    }
    let session_id = if operation.starts_with("session_zero.")
        && !matches!(
            operation.as_str(),
            "session_zero.begin" | "session_zero.join"
        ) {
        match state.session_zeros.session_for_account(account_hash).await {
            Ok(Some(value)) => Some(value),
            Ok(None) => {
                return Json(eve_result(
                    &invocation,
                    "denied",
                    "No active Session Zero belongs to this account.".into(),
                    None,
                    None,
                    None,
                ))
                .into_response();
            }
            Err(error) => {
                return Json(eve_result(
                    &invocation,
                    "denied",
                    error.to_string(),
                    None,
                    None,
                    None,
                ))
                .into_response();
            }
        }
    } else {
        None
    };
    let response = match operation.as_str() {
        "session_zero.begin" => match decode_payload::<BeginSessionZeroRequest>(&payload) {
            Ok(request) => {
                begin_session_zero(headers.clone(), State(state.clone()), Json(request)).await
            }
            Err(error) => return invalid_eve_payload(&invocation, error),
        },
        "session_zero.join" => {
            let token =
                string_field(&payload, "invite_token").or_else(|| string_field(&payload, "token"));
            let request = decode_payload::<JoinSessionZeroRequest>(&payload);
            match (token, request) {
                (Some(token), Ok(request)) => {
                    join_session_zero(
                        Path(token),
                        headers.clone(),
                        State(state.clone()),
                        Json(request),
                    )
                    .await
                }
                (None, _) => {
                    return invalid_eve_payload(
                        &invocation,
                        anyhow::anyhow!("invite_token is required"),
                    );
                }
                (_, Err(error)) => return invalid_eve_payload(&invocation, error),
            }
        }
        "session_zero.invites.create" => match decode_payload::<SessionZeroInviteRequest>(&payload)
        {
            Ok(request) => {
                create_session_zero_invites(
                    Path(session_id.unwrap()),
                    headers.clone(),
                    State(state.clone()),
                    Json(request),
                )
                .await
            }
            Err(error) => return invalid_eve_payload(&invocation, error),
        },
        "session_zero.message.send" => {
            match decode_payload::<SessionZeroMessageRequest>(&payload) {
                Ok(request) => {
                    post_session_zero_message(
                        Path(session_id.unwrap()),
                        headers.clone(),
                        State(state.clone()),
                        Json(request),
                    )
                    .await
                }
                Err(error) => return invalid_eve_payload(&invocation, error),
            }
        }
        "session_zero.boundary.set" => match decode_payload::<SessionZeroBoundaryRequest>(&payload)
        {
            Ok(request) => {
                set_session_zero_boundary(
                    Path(session_id.unwrap()),
                    headers.clone(),
                    State(state.clone()),
                    Json(request),
                )
                .await
            }
            Err(error) => return invalid_eve_payload(&invocation, error),
        },
        "session_zero.boundary.remove" => {
            let boundary_id = string_field(&payload, "target");
            let request = decode_payload::<SessionZeroRevisionRequest>(&payload);
            match (boundary_id, request) {
                (Some(id), Ok(request)) => {
                    remove_session_zero_boundary(
                        Path((session_id.unwrap(), id)),
                        headers.clone(),
                        State(state.clone()),
                        Json(request),
                    )
                    .await
                }
                (None, _) => {
                    return invalid_eve_payload(
                        &invocation,
                        anyhow::anyhow!("boundary target is required"),
                    );
                }
                (_, Err(error)) => return invalid_eve_payload(&invocation, error),
            }
        }
        "session_zero.leave" => match decode_payload::<SessionZeroRevisionRequest>(&payload) {
            Ok(request) => {
                leave_session_zero(
                    Path(session_id.unwrap()),
                    headers.clone(),
                    State(state.clone()),
                    Json(request),
                )
                .await
            }
            Err(error) => return invalid_eve_payload(&invocation, error),
        },
        "session_zero.member.remove" => {
            let target = string_field(&payload, "target");
            let revision = u64_field(&payload, "expected_revision");
            match (target, revision) {
                (Some(member_id), Some(expected_revision)) => {
                    remove_session_zero_member(
                        Path(session_id.unwrap()),
                        headers.clone(),
                        State(state.clone()),
                        Json(SessionZeroMemberRequest {
                            expected_revision,
                            member_id,
                        }),
                    )
                    .await
                }
                _ => {
                    return invalid_eve_payload(
                        &invocation,
                        anyhow::anyhow!("target and expected_revision are required"),
                    );
                }
            }
        }
        "session_zero.decision.resolve" => {
            match decode_payload::<SessionZeroDecisionRequest>(&payload) {
                Ok(request) => {
                    resolve_session_zero_decision(
                        Path(session_id.unwrap()),
                        headers.clone(),
                        State(state.clone()),
                        Json(request),
                    )
                    .await
                }
                Err(error) => return invalid_eve_payload(&invocation, error),
            }
        }
        "session_zero.roster.lock" => {
            match decode_payload::<SessionZeroRevisionRequest>(&payload) {
                Ok(request) => {
                    lock_session_zero_roster(
                        Path(session_id.unwrap()),
                        headers.clone(),
                        State(state.clone()),
                        Json(request),
                    )
                    .await
                }
                Err(error) => return invalid_eve_payload(&invocation, error),
            }
        }
        "session_zero.compile" => match decode_payload::<SessionZeroRevisionRequest>(&payload) {
            Ok(request) => {
                compile_session_zero(
                    Path(session_id.unwrap()),
                    headers.clone(),
                    State(state.clone()),
                    Json(request),
                )
                .await
            }
            Err(error) => return invalid_eve_payload(&invocation, error),
        },
        "session_zero.approve" => match decode_payload::<SessionZeroRevisionRequest>(&payload) {
            Ok(request) => {
                approve_session_zero(
                    Path(session_id.unwrap()),
                    headers.clone(),
                    State(state.clone()),
                    Json(request),
                )
                .await
            }
            Err(error) => return invalid_eve_payload(&invocation, error),
        },
        "session_zero.publish" => match decode_payload::<SessionZeroRevisionRequest>(&payload) {
            Ok(request) => {
                publish_session_zero(
                    Path(session_id.unwrap()),
                    headers.clone(),
                    State(state.clone()),
                    Json(request),
                )
                .await
            }
            Err(error) => return invalid_eve_payload(&invocation, error),
        },
        "campaign.contract_review.begin" => {
            begin_contract_review(headers.clone(), State(state.clone())).await
        }
        "campaign.select" => match string_field(&payload, "campaign_id")
            .and_then(|value| uuid::Uuid::parse_str(&value).ok())
        {
            Some(campaign_id) => {
                select_campaign_route(Path(campaign_id), headers.clone(), State(state.clone()))
                    .await
            }
            None => {
                return invalid_eve_payload(
                    &invocation,
                    anyhow::anyhow!("campaign_id is required"),
                );
            }
        },
        "world.speak" | "world.assess" | "world.attempt" | "world.wait" => {
            let (campaign, actor_id) = match current_player_context(state, account_hash).await {
                Ok(value) => value,
                Err(error) => return invalid_eve_payload(&invocation, error),
            };
            let world_command = match operation.as_str() {
                "world.speak" => WorldCommand::Speak {
                    expected_revision: required_u64!("expected_revision"),
                    actor_id,
                    text: required_string!("text"),
                    intended_effect: None,
                },
                "world.assess" => WorldCommand::Assess {
                    expected_revision: required_u64!("expected_revision"),
                    intent: ActionIntent {
                        actor_id,
                        description: required_string!("description"),
                        intended_effect: required_string!("intended_effect"),
                    },
                    proposal: None,
                },
                "world.attempt" => WorldCommand::Attempt {
                    actor_id,
                    assessment_digest: required_string!("assessment_digest"),
                },
                "world.wait" => WorldCommand::Wait {
                    expected_revision: required_u64!("expected_revision"),
                    minutes: required_u64!("minutes") as u32,
                },
                _ => unreachable!(),
            };
            let _ = campaign;
            command(headers.clone(), State(state.clone()), Json(world_command)).await
        }
        "governance.time.propose" => match (
            u64_field(&payload, "expected_revision"),
            u64_field(&payload, "time_advance_minutes"),
        ) {
            (Some(expected_revision), Some(minutes)) if u32::try_from(minutes).is_ok() => {
                propose_time_advance(
                    headers.clone(),
                    State(state.clone()),
                    Json(TimeAdvanceRequest {
                        expected_revision,
                        minutes: minutes as u32,
                    }),
                )
                .await
            }
            _ => {
                return invalid_eve_payload(
                    &invocation,
                    anyhow::anyhow!("expected_revision and time_advance_minutes are required"),
                );
            }
        },
        "governance.time.approve" => {
            let proposal = string_field(&payload, "proposal_id");
            let request = decode_payload::<SessionZeroRevisionRequest>(&payload);
            match (proposal, request) {
                (Some(id), Ok(request)) => {
                    approve_time_advance(
                        Path(id),
                        headers.clone(),
                        State(state.clone()),
                        Json(request),
                    )
                    .await
                }
                _ => {
                    return invalid_eve_payload(
                        &invocation,
                        anyhow::anyhow!("proposal_id and expected_revision are required"),
                    );
                }
            }
        }
        "governance.travel.propose" => match decode_payload::<GroupTravelRequest>(&payload) {
            Ok(request) => {
                propose_group_travel(headers.clone(), State(state.clone()), Json(request)).await
            }
            Err(error) => return invalid_eve_payload(&invocation, error),
        },
        "governance.travel.approve" => {
            let proposal = string_field(&payload, "proposal_id");
            let request = decode_payload::<SessionZeroRevisionRequest>(&payload);
            match (proposal, request) {
                (Some(id), Ok(request)) => {
                    approve_group_travel(
                        Path(id),
                        headers.clone(),
                        State(state.clone()),
                        Json(request),
                    )
                    .await
                }
                _ => {
                    return invalid_eve_payload(
                        &invocation,
                        anyhow::anyhow!("proposal_id and expected_revision are required"),
                    );
                }
            }
        }
        "governance.cells.propose" => match decode_payload::<CellBudgetRequest>(&payload) {
            Ok(request) => {
                propose_cell_budget(headers.clone(), State(state.clone()), Json(request)).await
            }
            Err(error) => return invalid_eve_payload(&invocation, error),
        },
        "governance.cells.approve" => {
            let proposal = string_field(&payload, "proposal_id");
            let request = decode_payload::<SessionZeroRevisionRequest>(&payload);
            match (proposal, request) {
                (Some(id), Ok(request)) => {
                    approve_cell_budget(
                        Path(id),
                        headers.clone(),
                        State(state.clone()),
                        Json(request),
                    )
                    .await
                }
                _ => {
                    return invalid_eve_payload(
                        &invocation,
                        anyhow::anyhow!("proposal_id and expected_revision are required"),
                    );
                }
            }
        }
        "world.destination.compile" => {
            let (campaign, actor_id) = match current_player_context(state, account_hash).await {
                Ok(value) => value,
                Err(error) => return invalid_eve_payload(&invocation, error),
            };
            let origin_location_id = match campaign.actors.get(&actor_id) {
                Some(actor) => actor.location_id.clone(),
                None => {
                    return invalid_eve_payload(
                        &invocation,
                        anyhow::anyhow!("Assigned actor is missing."),
                    );
                }
            };
            compile_destination(
                headers.clone(),
                State(state.clone()),
                Json(DestinationRequest {
                    origin_location_id,
                    destination: required_string!("destination"),
                }),
            )
            .await
        }
        "world.destination.approve" => match string_field(&payload, "preview_id") {
            Some(id) => approve_destination(Path(id), headers.clone(), State(state.clone())).await,
            None => {
                return invalid_eve_payload(&invocation, anyhow::anyhow!("preview_id is required"));
            }
        },
        "world.gestalt.fission.compile" => {
            match decode_payload::<GestaltFissionRequest>(&payload) {
                Ok(request) => {
                    compile_fission(headers.clone(), State(state.clone()), Json(request)).await
                }
                Err(error) => return invalid_eve_payload(&invocation, error),
            }
        }
        "world.gestalt.fission.approve" => match string_field(&payload, "preview_id") {
            Some(id) => approve_fission(Path(id), headers.clone(), State(state.clone())).await,
            None => {
                return invalid_eve_payload(&invocation, anyhow::anyhow!("preview_id is required"));
            }
        },
        _ => {
            return Json(eve_result(
                &invocation,
                "denied",
                format!("Unknown Eve operation {operation}."),
                None,
                None,
                None,
            ))
            .into_response();
        }
    };
    kernel_response_to_eve(invocation, response).await
}

fn captured_payload(payload: &serde_json::Value) -> serde_json::Value {
    let mut captured = payload.as_object().cloned().unwrap_or_default();
    if let Some(serde_json::Value::Object(bindings)) = captured.remove("bindings") {
        for (name, value) in bindings {
            captured.insert(name, value);
        }
    }
    serde_json::Value::Object(captured)
}

fn decode_payload<T: serde::de::DeserializeOwned>(
    payload: &serde_json::Value,
) -> anyhow::Result<T> {
    serde_json::from_value(payload.clone()).map_err(Into::into)
}

fn string_field(payload: &serde_json::Value, name: &str) -> Option<String> {
    payload
        .get(name)
        .and_then(serde_json::Value::as_str)
        .map(str::to_owned)
}

fn u64_field(payload: &serde_json::Value, name: &str) -> Option<u64> {
    payload.get(name).and_then(serde_json::Value::as_u64)
}

fn invalid_eve_payload(invocation: &EveCommandInvocation, error: anyhow::Error) -> Response {
    Json(eve_result(
        invocation,
        "denied",
        format!("Invalid operation payload: {error}"),
        None,
        None,
        None,
    ))
    .into_response()
}

async fn current_player_context(
    state: &AppState,
    account_hash: &str,
) -> anyhow::Result<(Campaign, String)> {
    let runtime = session_runtime(state, account_hash)
        .await?
        .ok_or_else(|| anyhow::anyhow!("No campaign is selected."))?;
    let campaign = load_campaign(&runtime.store)?;
    let member = campaign_member_for_account(&runtime.store, &campaign, account_hash)?;
    Ok((campaign, member.actor_id))
}

async fn kernel_response_to_eve(invocation: EveCommandInvocation, response: Response) -> Response {
    let status = response.status();
    let bytes = match axum::body::to_bytes(response.into_body(), 2 * 1024 * 1024).await {
        Ok(value) => value,
        Err(error) => {
            return Json(eve_result(
                &invocation,
                "denied",
                error.to_string(),
                None,
                None,
                None,
            ))
            .into_response();
        }
    };
    let value = serde_json::from_slice::<serde_json::Value>(&bytes).ok();
    let fallback = String::from_utf8_lossy(&bytes).trim().to_owned();
    let message = value
        .as_ref()
        .and_then(|value| value.get("error").and_then(serde_json::Value::as_str))
        .map(str::to_owned)
        .or_else(|| (!fallback.is_empty() && value.is_none()).then_some(fallback))
        .unwrap_or_else(|| {
            if status.is_success() {
                "Ghostlight committed the operation.".into()
            } else {
                format!("Ghostlight refused the operation ({status}).")
            }
        });
    let accepted = status.is_success();
    let transient = if accepted {
        value.as_ref().and_then(|value| {
            transient_result_projection(
                &invocation.operation.operation_id,
                value,
                invocation.operation.route_hint.source_version.unwrap_or(0),
            )
        })
    } else {
        None
    };
    let clear = accepted.then(|| {
        invocation
            .payload
            .get("bindings")
            .and_then(serde_json::Value::as_object)
            .map(|values| values.keys().cloned().collect::<Vec<_>>())
            .unwrap_or_default()
    });
    Json(eve_result(
        &invocation,
        if accepted { "accepted" } else { "denied" },
        message,
        transient,
        None,
        clear,
    ))
    .into_response()
}

fn transient_result_projection(
    operation: &str,
    value: &serde_json::Value,
    source_version: u64,
) -> Option<serde_json::Value> {
    let mut children = Vec::new();
    if operation == "session_zero.invites.create" {
        let links = value
            .get("invite_tokens")?
            .as_array()?
            .iter()
            .filter_map(serde_json::Value::as_str)
            .map(|token| format!("/ghostlight/?invite={token}"))
            .collect::<Vec<_>>();
        let text = (!links.is_empty())
            .then(|| format!("Single-use invitations:\n{}", links.join("\n")))?;
        children.push(serde_json::json!({"id":"ghostlight.command-result.text","kind":"text","props":{"value":text},"children":[]}));
    } else if operation == "world.assess" {
        let assessment = value.get("assessment")?;
        let digest = assessment.get("digest")?.as_str()?;
        let admissible = assessment
            .get("admissible")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false);
        let summary = format!(
            "{}\nDC {} · modifier {} · ceiling {}\nSuccess: {}\nMixed: {}\nFailure: {}{}",
            if admissible {
                "The attempt is admissible."
            } else {
                "The attempt is not currently admissible."
            },
            assessment
                .get("dc")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(0),
            assessment
                .get("modifier_total")
                .and_then(serde_json::Value::as_i64)
                .unwrap_or(0),
            assessment
                .get("effect_ceiling")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("bounded"),
            assessment
                .get("success_stake")
                .and_then(serde_json::Value::as_str)
                .unwrap_or(""),
            assessment
                .get("mixed_stake")
                .and_then(serde_json::Value::as_str)
                .unwrap_or(""),
            assessment
                .get("failure_stake")
                .and_then(serde_json::Value::as_str)
                .unwrap_or(""),
            assessment
                .get("bargains")
                .and_then(serde_json::Value::as_array)
                .filter(|values| !values.is_empty())
                .map(|values| format!(
                    "\nBargains: {}",
                    values
                        .iter()
                        .filter_map(serde_json::Value::as_str)
                        .collect::<Vec<_>>()
                        .join("; ")
                ))
                .unwrap_or_default(),
        );
        children.push(serde_json::json!({"id":"ghostlight.assessment.summary","kind":"text","props":{"value":summary},"children":[]}));
        if admissible {
            children.push(eve_button(
                "ghostlight.assessment.confirm",
                "Roll the d20",
                "world.attempt",
                serde_json::json!({"assessment_digest":digest}),
                &[],
            ));
        }
    } else if matches!(
        operation,
        "world.destination.compile" | "world.gestalt.fission.compile"
    ) {
        children.push(serde_json::json!({"id":"ghostlight.command-result.text","kind":"text","props":{"value":serde_json::to_string_pretty(value).ok()?},"children":[]}));
        let preview_id = value.get("preview_id")?.as_str()?;
        let approval_operation = if operation == "world.destination.compile" {
            "world.destination.approve"
        } else {
            "world.gestalt.fission.approve"
        };
        children.push(eve_button(
            "ghostlight.preview.approve",
            "Approve preview",
            approval_operation,
            serde_json::json!({"preview_id":preview_id}),
            &[],
        ));
    } else {
        return None;
    }
    Some(serde_json::json!({
        "type":"surface-state",
        "schema":"gamecult.eve.surface.v1",
        "providerId":EVE_PROVIDER_ID,
        "providerKind":"narrative.simulation.command-result",
        "title":"Ghostlight result",
        "version":source_version,
        "updatedAtUtc":Utc::now().to_rfc3339(),
        "surface":{"id":"ghostlight.command-result","root":{"id":"ghostlight.command-result.root","kind":"card","props":{"title":"Result"},"children":children}},
        "commands":[
            eve_command_descriptor("world.attempt","ghostlight.player_action_attempt.v1",&[],"WorldKernel"),
            eve_command_descriptor("world.destination.approve","ghostlight.destination_approval.v1",&[],"WorldKernel"),
            eve_command_descriptor("world.gestalt.fission.approve","ghostlight.gestalt_fission_approval.v1",&[],"WorldKernel")
        ]
    }))
}

async fn health(State(state): State<AppState>) -> Response {
    match state.mesh.health() {
        Ok(value) => Json(value).into_response(),
        Err(error) => (StatusCode::SERVICE_UNAVAILABLE, error.to_string()).into_response(),
    }
}

async fn begin_session_zero(
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(request): Json<BeginSessionZeroRequest>,
) -> Response {
    let account_hash = match authenticated_session(&headers, &state).await {
        Some(value) => value,
        None => return StatusCode::UNAUTHORIZED.into_response(),
    };
    let allowance = state.entitlements.persona_cell_allowance(&account_hash);
    match SessionZeroState::new_with_allowance(
        request.name,
        request.vault_provider,
        account_hash.clone(),
        request.display_name,
        allowance,
    ) {
        Ok(session_zero) => {
            let id = session_zero.id;
            match state.session_zeros.create(session_zero).await {
                Ok(runtime) => match state.session_zeros.snapshot(id).await {
                    Ok(snapshot) => match session_zero_surface(&snapshot, &account_hash) {
                        Ok(surface) => {
                            if let Some(compiler) = state.compiler.clone() {
                                let mesh_state = state.clone();
                                tokio::spawn(async move {
                                    if let Err(error) = populate_opening_suggestions(
                                        compiler,
                                        runtime,
                                        snapshot,
                                        mesh_state.clone(),
                                    )
                                    .await
                                    {
                                        tracing::warn!(%error, "inline Session Zero opening suggestions failed without draft mutation");
                                    }
                                });
                            }
                            schedule_mesh_refresh(&state);
                            (StatusCode::CREATED, Json(surface)).into_response()
                        }
                        Err(error) => {
                            (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()).into_response()
                        }
                    },
                    Err(error) => {
                        (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()).into_response()
                    }
                },
                Err(error) => (StatusCode::CONFLICT, error.to_string()).into_response(),
            }
        }
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, error.to_string()).into_response(),
    }
}

async fn populate_opening_suggestions(
    compiler: Arc<WorldCompiler>,
    runtime: ghostlight_dungeon::session_zero::SessionZeroRuntime,
    snapshot: SessionZeroState,
    state: AppState,
) -> anyhow::Result<()> {
    let suggestions = compiler
        .suggest_openings(OpeningRequest {
            setting: snapshot.contract.vault_provider.clone(),
            constraints: vec![format!(
                "Offer source-grounded possibilities for the Session Zero draft named {}. They are discussion proposals, not accepted world truth.",
                snapshot.name
            )],
        })
        .await?;
    for receipt in &suggestions.evidence_receipts {
        if runtime
            .store
            .load::<ghostlight_dungeon::domain::VaultEvidenceReceipt>(
                "vault_evidence_receipt.v1",
                &receipt.id,
            )?
            .is_none()
        {
            runtime.store.insert(
                "vault_evidence_receipt.v1",
                "ghostlight.vault_evidence_receipt.v1",
                &receipt.id,
                receipt,
            )?;
        }
    }
    let decisions = suggestions
        .openings
        .iter()
        .map(
            |opening| ghostlight_dungeon::session_zero::SessionZeroDecision {
                schema: "ghostlight.session_zero_decision.v1".into(),
                id: format!("opening:{}", opening.id),
                owner_member_id: None,
                prompt: format!(
                    "Use '{}' as the starting frame for further Session Zero discussion?",
                    opening.title
                ),
                proposed_resolution: format!(
                    "{} — {} at {}, under pressure from {}. Player hook: {}",
                    opening.title,
                    opening.era,
                    opening.place,
                    opening.pressure,
                    opening.player_hook
                ),
                proposed_extraordinary_permission: None,
                proposed_contract_patch: Some(
                    ghostlight_dungeon::session_zero::CampaignContractPatch {
                        premise: Some(opening.player_hook.clone()),
                        starting_where: Some(opening.place.clone()),
                        starting_when: Some(opening.era.clone()),
                        starting_pressure: Some(opening.pressure.clone()),
                        ..Default::default()
                    },
                ),
                proposed_character_patch: None,
                evidence_receipt_ids: opening.evidence_receipt_ids.clone(),
                pending_counter: None,
                material: false,
                resolved: false,
            },
        )
        .collect::<Vec<_>>();
    let dm_speech = format!(
        "I found three Vault-grounded starting frames. They are invitations, not decrees:\n{}\nChoose one to place it into our typed draft for discussion, counter it, or ignore all three and describe your own start.",
        suggestions
            .openings
            .iter()
            .map(|opening| format!(
                "• {} — {} · {} · {}",
                opening.title, opening.era, opening.place, opening.pressure
            ))
            .collect::<Vec<_>>()
            .join("\n")
    );
    let mut model_receipts = suggestions.model_receipts;
    model_receipts.push(suggestions.retrieval_receipt);
    runtime
        .kernel
        .command(SessionZeroCommand::ApplyDmTurn {
            expected_component_epoch: snapshot.shared_epoch,
            expected_channel_revision: snapshot.channels["shared:table"].revision,
            channel_id: "shared:table".into(),
            member_id: None,
            supersedes_countered_decision_id: None,
            delta: ghostlight_dungeon::session_zero::SessionZeroDelta {
                contract_patch: Default::default(),
                character_patch: None,
                decisions,
                dm_speech,
                suggested_replies: vec![
                    "Let's discuss the first opening.".into(),
                    "Show me what evidence supports these.".into(),
                    "I want a fully custom start.".into(),
                ],
            },
            model_receipts,
        })
        .await?;
    schedule_mesh_refresh(&state);
    Ok(())
}

fn accepted_opening_suggestion(
    state: &SessionZeroState,
    decision_id: &str,
) -> Option<OpeningSuggestion> {
    let decision = state.decisions.get(decision_id)?;
    if !decision.resolved || !decision_id.starts_with("opening:") {
        return None;
    }
    let patch = decision.proposed_contract_patch.as_ref()?;
    Some(OpeningSuggestion {
        id: decision_id.trim_start_matches("opening:").to_string(),
        title: decision
            .proposed_resolution
            .split(" — ")
            .next()
            .unwrap_or("Suggested opening")
            .to_string(),
        era: patch.starting_when.clone()?,
        place: patch.starting_where.clone()?,
        pressure: patch.starting_pressure.clone()?,
        player_hook: patch.premise.clone()?,
        evidence_receipt_ids: decision.evidence_receipt_ids.clone(),
    })
}

async fn populate_role_suggestions(
    compiler: Arc<WorldCompiler>,
    runtime: ghostlight_dungeon::session_zero::SessionZeroRuntime,
    snapshot: SessionZeroState,
    opening: OpeningSuggestion,
    state: AppState,
) -> anyhow::Result<()> {
    let suggestions = compiler.suggest_roles(&opening).await?;
    for receipt in &suggestions.evidence_receipts {
        if runtime
            .store
            .load::<ghostlight_dungeon::domain::VaultEvidenceReceipt>(
                "vault_evidence_receipt.v1",
                &receipt.id,
            )?
            .is_none()
        {
            runtime.store.insert(
                "vault_evidence_receipt.v1",
                "ghostlight.vault_evidence_receipt.v1",
                &receipt.id,
                receipt,
            )?;
        }
    }
    let mut receipts = suggestions.model_receipts;
    receipts.push(suggestions.retrieval_receipt);
    let active_members = snapshot
        .members
        .values()
        .filter(|member| member.active)
        .cloned()
        .collect::<Vec<_>>();
    for (index, member) in active_members.into_iter().enumerate() {
        let channel_id = format!("private:{}", member.id);
        let decisions = suggestions
            .roles
            .iter()
            .map(
                |role| ghostlight_dungeon::session_zero::SessionZeroDecision {
                    schema: "ghostlight.session_zero_decision.v1".into(),
                    id: format!("role:{}:{}", member.id, role.id),
                    owner_member_id: Some(member.id.clone()),
                    prompt: format!(
                        "Use '{}' as a starting character premise for further negotiation?",
                        role.name
                    ),
                    proposed_resolution: format!("{} — {}", role.name, role.premise),
                    proposed_extraordinary_permission: None,
                    proposed_contract_patch: None,
                    proposed_character_patch: Some(
                        ghostlight_dungeon::session_zero::CharacterDraftPatch {
                            name: Some(role.name.clone()),
                            public_premise: Some(role.premise.clone()),
                            capabilities_add: role.capabilities.clone(),
                            obligations_add: role.obligations.clone(),
                            ..Default::default()
                        },
                    ),
                    evidence_receipt_ids: role.evidence_receipt_ids.clone(),
                    pending_counter: None,
                    material: false,
                    resolved: false,
                },
            )
            .collect();
        runtime
            .kernel
            .command(SessionZeroCommand::ApplyDmTurn {
                expected_component_epoch: snapshot.character_epochs[&member.id],
                expected_channel_revision: snapshot.channels[&channel_id].revision,
                channel_id,
                member_id: Some(member.id.clone()),
                supersedes_countered_decision_id: None,
                delta: ghostlight_dungeon::session_zero::SessionZeroDelta {
                    contract_patch: Default::default(),
                    character_patch: None,
                    decisions,
                    dm_speech: format!(
                        "For {}, I found three source-grounded roles inside the chosen opening. Pick one to put it into your private draft for negotiation, counter it, or build your own.",
                        member.display_name
                    ),
                    suggested_replies: vec![
                        "Let's negotiate the first role.".into(),
                        "I want a custom character.".into(),
                    ],
                },
                model_receipts: if index == 0 { receipts.clone() } else { vec![] },
            })
            .await?;
    }
    schedule_mesh_refresh(&state);
    Ok(())
}

async fn create_session_zero_invites(
    Path(session_id): Path<uuid::Uuid>,
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(request): Json<SessionZeroInviteRequest>,
) -> Response {
    let account_hash = match authenticated_session(&headers, &state).await {
        Some(value) => value,
        None => return StatusCode::UNAUTHORIZED.into_response(),
    };
    let runtime = match state.session_zeros.runtime(session_id).await {
        Ok(value) => value,
        Err(error) => return (StatusCode::NOT_FOUND, error.to_string()).into_response(),
    };
    match runtime
        .kernel
        .command(SessionZeroCommand::CreateInvites {
            actor_account_hash: account_hash,
            count: request.count,
        })
        .await
    {
        Ok(result) => {
            schedule_mesh_refresh(&state);
            Json(serde_json::json!({
                "session_zero_id": session_id,
                "revision": result.state.revision,
                "invite_tokens": result.invite_tokens,
            }))
            .into_response()
        }
        Err(error) => (StatusCode::CONFLICT, error.to_string()).into_response(),
    }
}

async fn join_session_zero(
    Path(token): Path<String>,
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(request): Json<JoinSessionZeroRequest>,
) -> Response {
    let account_hash = match authenticated_session(&headers, &state).await {
        Some(value) => value,
        None => return StatusCode::UNAUTHORIZED.into_response(),
    };
    let session_id = match state.session_zeros.session_for_invite(&token).await {
        Ok(Some(value)) => value,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                "invite is invalid, expired, or consumed",
            )
                .into_response();
        }
        Err(error) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()).into_response();
        }
    };
    let runtime = match state.session_zeros.runtime(session_id).await {
        Ok(value) => value,
        Err(error) => return (StatusCode::NOT_FOUND, error.to_string()).into_response(),
    };
    match runtime
        .kernel
        .command(SessionZeroCommand::Join {
            token,
            account_hash: account_hash.clone(),
            display_name: request.display_name,
            cell_allowance: state.entitlements.persona_cell_allowance(&account_hash),
        })
        .await
    {
        Ok(result) => {
            schedule_mesh_refresh(&state);
            match session_zero_surface(&result.state, &account_hash) {
                Ok(surface) => (StatusCode::CREATED, Json(surface)).into_response(),
                Err(error) => {
                    (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()).into_response()
                }
            }
        }
        Err(error) => (StatusCode::CONFLICT, error.to_string()).into_response(),
    }
}

async fn post_session_zero_message(
    Path(session_id): Path<uuid::Uuid>,
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(request): Json<SessionZeroMessageRequest>,
) -> Response {
    let account_hash = match authenticated_session(&headers, &state).await {
        Some(value) => value,
        None => return StatusCode::UNAUTHORIZED.into_response(),
    };
    let runtime = match state.session_zeros.runtime(session_id).await {
        Ok(value) => value,
        Err(error) => return (StatusCode::NOT_FOUND, error.to_string()).into_response(),
    };
    let channel_id = request.channel_id.clone();
    let result = match runtime
        .kernel
        .command(SessionZeroCommand::PostPlayerMessage {
            actor_account_hash: account_hash.clone(),
            expected_revision: request.expected_revision,
            channel_id: channel_id.clone(),
            text: request.text,
        })
        .await
    {
        Ok(value) => value,
        Err(error) => return (StatusCode::CONFLICT, error.to_string()).into_response(),
    };
    let member_id = result
        .state
        .member_for_account(&account_hash)
        .map(|member| member.id.clone());
    let channel = result.state.channels.get(&channel_id).cloned();
    if let Some(channel) = channel {
        let private_member = if channel.kind
            == ghostlight_dungeon::session_zero::SessionZeroChannelKind::PrivateDm
        {
            member_id.clone()
        } else {
            None
        };
        schedule_session_zero_dm_response(
            &state,
            &runtime,
            result.state.clone(),
            channel_id.clone(),
            private_member,
            None,
        );
    }
    schedule_mesh_refresh(&state);
    (
        StatusCode::ACCEPTED,
        Json(serde_json::json!({
            "schema": "ghostlight.session_zero_progress.v1",
            "session_zero_id": session_id,
            "revision": result.state.revision,
            "status": "player_message_committed",
        })),
    )
        .into_response()
}

fn schedule_session_zero_dm_response(
    state: &AppState,
    runtime: &ghostlight_dungeon::session_zero::SessionZeroRuntime,
    snapshot: SessionZeroState,
    channel_id: String,
    member_id: Option<String>,
    supersedes_countered_decision_id: Option<String>,
) {
    let Some(director) = state.session_zero_director.clone() else {
        return;
    };
    let Some(channel) = snapshot.channels.get(&channel_id).cloned() else {
        return;
    };
    let component_epoch = member_id
        .as_ref()
        .and_then(|id| snapshot.character_epochs.get(id).copied())
        .unwrap_or(snapshot.shared_epoch);
    let kernel = runtime.kernel.clone();
    let mesh_state = state.clone();
    tokio::spawn(async move {
        match director
            .respond(&snapshot, &channel_id, member_id.as_deref())
            .await
        {
            Ok((delta, receipts)) => {
                let applied = kernel
                    .command(SessionZeroCommand::ApplyDmTurn {
                        expected_component_epoch: component_epoch,
                        expected_channel_revision: channel.revision,
                        channel_id: channel_id.clone(),
                        member_id: member_id.clone(),
                        supersedes_countered_decision_id: supersedes_countered_decision_id.clone(),
                        delta,
                        model_receipts: receipts,
                    })
                    .await;
                if let Err(error) = applied {
                    tracing::info!(%error, "stale or invalid Session Zero DM proposal discarded");
                    if supersedes_countered_decision_id.is_some() {
                        append_session_zero_dm_failure_notice(
                            &kernel,
                            &mesh_state,
                            component_epoch,
                            channel.revision,
                            channel_id,
                            member_id,
                            "I couldn't turn that counterproposal into a safe typed replacement. Your counter remains pending and the previous bargain cannot be accepted. Please discuss or counter it again.",
                        )
                        .await;
                    }
                } else {
                    schedule_mesh_refresh(&mesh_state);
                }
            }
            Err(error) => {
                tracing::warn!(%error, "Session Zero DM inference failed without draft mutation");
                append_session_zero_dm_failure_notice(
                    &kernel,
                    &mesh_state,
                    component_epoch,
                    channel.revision,
                    channel_id,
                    member_id,
                    "I couldn't finish that response. Your message is safely recorded and no draft state changed; please retry or rephrase when you're ready.",
                )
                .await;
            }
        }
    });
}

async fn append_session_zero_dm_failure_notice(
    kernel: &ghostlight_dungeon::session_zero::SessionZeroKernel,
    state: &AppState,
    expected_component_epoch: u64,
    expected_channel_revision: u64,
    channel_id: String,
    member_id: Option<String>,
    message: &str,
) {
    let failure = ghostlight_dungeon::session_zero::SessionZeroDelta {
        dm_speech: message.into(),
        ..Default::default()
    };
    if let Err(stale) = kernel
        .command(SessionZeroCommand::ApplyDmTurn {
            expected_component_epoch,
            expected_channel_revision,
            channel_id,
            member_id,
            supersedes_countered_decision_id: None,
            delta: failure,
            model_receipts: Vec::new(),
        })
        .await
    {
        tracing::info!(%stale, "stale Session Zero DM failure notice discarded");
    } else {
        schedule_mesh_refresh(state);
    }
}

async fn set_session_zero_boundary(
    Path(session_id): Path<uuid::Uuid>,
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(request): Json<SessionZeroBoundaryRequest>,
) -> Response {
    let account_hash = match authenticated_session(&headers, &state).await {
        Some(value) => value,
        None => return StatusCode::UNAUTHORIZED.into_response(),
    };
    let runtime = match state.session_zeros.runtime(session_id).await {
        Ok(value) => value,
        Err(error) => return (StatusCode::NOT_FOUND, error.to_string()).into_response(),
    };
    let normalized_topic = request.topic.trim().to_lowercase();
    match runtime
        .kernel
        .command(SessionZeroCommand::SetBoundary {
            actor_account_hash: account_hash,
            expected_revision: request.expected_revision,
            boundary_id: request.boundary_id,
            topic: request.topic,
            normalized_topic,
            level: request.level,
        })
        .await
    {
        Ok(result) => {
            if let Err(error) =
                mirror_contract_review_boundary_tightening(&state, &result.state).await
            {
                return (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()).into_response();
            }
            schedule_mesh_refresh(&state);
            Json(serde_json::json!({"revision":result.state.revision,"aggregate_boundaries":result.state.aggregate_boundaries})).into_response()
        }
        Err(error) => (StatusCode::CONFLICT, error.to_string()).into_response(),
    }
}

async fn mirror_contract_review_boundary_tightening(
    state: &AppState,
    review: &SessionZeroState,
) -> anyhow::Result<()> {
    let Some(campaign_id) = review.review_campaign_id else {
        return Ok(());
    };
    let runtime = state.registry.runtime(campaign_id).await?;
    let publication = runtime
        .store
        .load::<ghostlight_dungeon::session_zero::PublishedSessionZeroSeed>(
            "session_zero_publication.v1",
            &campaign_id.to_string(),
        )?
        .map(|(_, value)| value)
        .ok_or_else(|| anyhow::anyhow!("campaign publication state is missing"))?;
    let existing = runtime
        .store
        .load::<ghostlight_dungeon::session_zero::ActiveContractBoundaryPolicy>(
            "active_contract_boundary_policy.v1",
            &campaign_id.to_string(),
        )?;
    let mut strictest =
        BTreeMap::<String, ghostlight_dungeon::session_zero::AggregatedBoundary>::new();
    for boundary in publication
        .approved_brief
        .aggregate_boundaries
        .iter()
        .chain(
            existing
                .as_ref()
                .into_iter()
                .flat_map(|(_, policy)| policy.aggregate_boundaries.iter()),
        )
        .chain(review.aggregate_boundaries.iter())
    {
        let severity = |level: &BoundaryLevel| match level {
            BoundaryLevel::AskFirst => 1,
            BoundaryLevel::Veil => 2,
            BoundaryLevel::Line => 3,
        };
        let entry = strictest
            .entry(boundary.normalized_topic.clone())
            .or_insert_with(|| boundary.clone());
        if severity(&boundary.level) > severity(&entry.level) {
            *entry = boundary.clone();
        }
    }
    let policy = ghostlight_dungeon::session_zero::ActiveContractBoundaryPolicy {
        schema: "ghostlight.active_contract_boundary_policy.v1".into(),
        campaign_id,
        review_session_zero_id: review.id,
        aggregate_boundaries: strictest.into_values().collect(),
        updated_at: chrono::Utc::now(),
    };
    if let Some((row, _)) = existing {
        runtime.store.replace(
            &row,
            "ghostlight.active_contract_boundary_policy.v1",
            &policy,
        )?;
    } else {
        runtime.store.insert(
            "active_contract_boundary_policy.v1",
            "ghostlight.active_contract_boundary_policy.v1",
            &campaign_id.to_string(),
            &policy,
        )?;
    }
    Ok(())
}

async fn remove_session_zero_boundary(
    Path((session_id, boundary_id)): Path<(uuid::Uuid, String)>,
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(request): Json<SessionZeroRevisionRequest>,
) -> Response {
    session_zero_simple_command(&headers, &state, session_id, |account_hash| {
        SessionZeroCommand::RemoveBoundary {
            actor_account_hash: account_hash,
            expected_revision: request.expected_revision,
            boundary_id,
        }
    })
    .await
}

async fn leave_session_zero(
    Path(session_id): Path<uuid::Uuid>,
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(request): Json<SessionZeroRevisionRequest>,
) -> Response {
    session_zero_simple_command(&headers, &state, session_id, |account_hash| {
        SessionZeroCommand::Leave {
            actor_account_hash: account_hash,
            expected_revision: request.expected_revision,
        }
    })
    .await
}

async fn remove_session_zero_member(
    Path(session_id): Path<uuid::Uuid>,
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(request): Json<SessionZeroMemberRequest>,
) -> Response {
    session_zero_simple_command(&headers, &state, session_id, |account_hash| {
        SessionZeroCommand::RemoveMember {
            actor_account_hash: account_hash,
            expected_revision: request.expected_revision,
            member_id: request.member_id,
        }
    })
    .await
}

async fn resolve_session_zero_decision(
    Path(session_id): Path<uuid::Uuid>,
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(request): Json<SessionZeroDecisionRequest>,
) -> Response {
    let account_hash = match authenticated_session(&headers, &state).await {
        Some(value) => value,
        None => return StatusCode::UNAUTHORIZED.into_response(),
    };
    let runtime = match state.session_zeros.runtime(session_id).await {
        Ok(value) => value,
        Err(error) => return (StatusCode::NOT_FOUND, error.to_string()).into_response(),
    };
    let decision_id = request.decision_id.clone();
    let accepted = request.accept;
    match runtime
        .kernel
        .command(SessionZeroCommand::ResolveDecision {
            actor_account_hash: account_hash,
            expected_revision: request.expected_revision,
            decision_id: request.decision_id,
            accept: request.accept,
            counter: request.counter,
        })
        .await
    {
        Ok(result) => {
            if !accepted
                && let Some(decision) = result.state.decisions.get(&decision_id)
                && decision.pending_counter.is_some()
            {
                let member_id = decision.owner_member_id.clone();
                let channel_id = member_id
                    .as_ref()
                    .map(|owner| format!("private:{owner}"))
                    .unwrap_or_else(|| "shared:table".into());
                schedule_session_zero_dm_response(
                    &state,
                    &runtime,
                    result.state.clone(),
                    channel_id,
                    member_id,
                    Some(decision_id.clone()),
                );
            }
            if accepted
                && let Some(opening) = accepted_opening_suggestion(&result.state, &decision_id)
                && let Some(compiler) = state.compiler.clone()
            {
                let role_runtime = runtime.clone();
                let role_state = state.clone();
                let role_snapshot = result.state.clone();
                tokio::spawn(async move {
                    if let Err(error) = populate_role_suggestions(
                        compiler,
                        role_runtime,
                        role_snapshot,
                        opening,
                        role_state.clone(),
                    )
                    .await
                    {
                        tracing::warn!(%error, "inline Session Zero role suggestions failed without draft mutation");
                    }
                });
            }
            schedule_mesh_refresh(&state);
            Json(serde_json::json!({"revision":result.state.revision,"decisions":result.state.decisions})).into_response()
        }
        Err(error) => (StatusCode::CONFLICT, error.to_string()).into_response(),
    }
}

async fn lock_session_zero_roster(
    Path(session_id): Path<uuid::Uuid>,
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(request): Json<SessionZeroRevisionRequest>,
) -> Response {
    session_zero_simple_command(&headers, &state, session_id, |account_hash| {
        SessionZeroCommand::LockRoster {
            actor_account_hash: account_hash,
            expected_revision: request.expected_revision,
        }
    })
    .await
}

async fn compile_session_zero(
    Path(session_id): Path<uuid::Uuid>,
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(request): Json<SessionZeroRevisionRequest>,
) -> Response {
    let account_hash = match authenticated_session(&headers, &state).await {
        Some(value) => value,
        None => return StatusCode::UNAUTHORIZED.into_response(),
    };
    let runtime = match state.session_zeros.runtime(session_id).await {
        Ok(value) => value,
        Err(error) => return (StatusCode::NOT_FOUND, error.to_string()).into_response(),
    };
    let started = match runtime
        .kernel
        .command(SessionZeroCommand::BeginCompilation {
            actor_account_hash: account_hash,
            expected_revision: request.expected_revision,
        })
        .await
    {
        Ok(value) => value,
        Err(error) => return (StatusCode::CONFLICT, error.to_string()).into_response(),
    };
    let brief = match started.state.compilation_brief() {
        Ok(value) => value,
        Err(error) => return (StatusCode::UNPROCESSABLE_ENTITY, error.to_string()).into_response(),
    };
    let expected_revision = started.state.revision;
    if let Some(campaign_id) = started.state.review_campaign_id {
        let campaign_runtime = match state.registry.runtime(campaign_id).await {
            Ok(value) => value,
            Err(error) => return (StatusCode::NOT_FOUND, error.to_string()).into_response(),
        };
        let campaign = match load_campaign(&campaign_runtime.store) {
            Ok(value) => value,
            Err(error) => {
                return (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()).into_response();
            }
        };
        if Some(campaign.revision) != started.state.review_world_revision {
            let _ = runtime
                .kernel
                .command(SessionZeroCommand::CompilationFailed {
                    expected_revision,
                    message: "The world changed during Contract Review. Start a fresh review against the current revision.".into(),
                })
                .await;
            return (
                StatusCode::CONFLICT,
                "contract review world revision is stale",
            )
                .into_response();
        }
        let evidence_receipts = campaign_runtime
            .store
            .load_all::<ghostlight_dungeon::domain::VaultEvidenceReceipt>(
                "vault_evidence_receipt.v1",
            )
            .unwrap_or_default();
        let preview = ghostlight_dungeon::domain::WorldCompilePreview {
            schema: "ghostlight.world_compile_preview.v1".into(),
            title: format!("{} — Contract Review", campaign.name),
            campaign,
            evidence_receipts,
            evidence_coverage: vec![],
            gaps: vec![],
            branch_assumptions: vec![
                "Contract Review amends forward-looking governance and approved character projections; established events, canon, knowledge, and geometry remain unchanged.".into(),
            ],
            requires_approval: true,
        };
        return match runtime
            .kernel
            .command(SessionZeroCommand::InstallPreview {
                expected_revision,
                preview,
                model_receipts: vec![],
            })
            .await
        {
            Ok(result) => {
                schedule_mesh_refresh(&state);
                (
                    StatusCode::ACCEPTED,
                    Json(serde_json::json!({
                        "schema":"ghostlight.session_zero_progress.v1",
                        "session_zero_id":session_id,
                        "revision":result.state.revision,
                        "status":"review"
                    })),
                )
                    .into_response()
            }
            Err(error) => (StatusCode::CONFLICT, error.to_string()).into_response(),
        };
    }
    let Some(compiler) = state.compiler.clone() else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            "world compiler is unavailable",
        )
            .into_response();
    };
    let kernel = runtime.kernel.clone();
    let mesh_state = state.clone();
    tokio::spawn(async move {
        match compiler.compile_approved_brief(&brief).await {
            Ok((preview, receipts)) => {
                if let Err(error) = kernel
                    .command(SessionZeroCommand::InstallPreview {
                        expected_revision,
                        preview,
                        model_receipts: receipts,
                    })
                    .await
                {
                    tracing::info!(%error, "stale Session Zero compilation discarded");
                } else {
                    schedule_mesh_refresh(&mesh_state);
                }
            }
            Err(error) => {
                let message = error.to_string();
                if let Err(commit_error) = kernel
                    .command(SessionZeroCommand::CompilationFailed {
                        expected_revision,
                        message,
                    })
                    .await
                {
                    tracing::info!(%commit_error, "stale Session Zero compiler failure discarded");
                } else {
                    schedule_mesh_refresh(&mesh_state);
                }
            }
        }
    });
    (
        StatusCode::ACCEPTED,
        Json(serde_json::json!({
            "schema":"ghostlight.session_zero_progress.v1",
            "session_zero_id":session_id,
            "revision":expected_revision,
            "status":"compiling"
        })),
    )
        .into_response()
}

async fn approve_session_zero(
    Path(session_id): Path<uuid::Uuid>,
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(request): Json<SessionZeroRevisionRequest>,
) -> Response {
    session_zero_simple_command(&headers, &state, session_id, |account_hash| {
        SessionZeroCommand::Approve {
            actor_account_hash: account_hash,
            expected_revision: request.expected_revision,
        }
    })
    .await
}

async fn publish_session_zero(
    Path(session_id): Path<uuid::Uuid>,
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(request): Json<SessionZeroRevisionRequest>,
) -> Response {
    let account_hash = match authenticated_session(&headers, &state).await {
        Some(value) => value,
        None => return StatusCode::UNAUTHORIZED.into_response(),
    };
    let runtime = match state.session_zeros.runtime(session_id).await {
        Ok(value) => value,
        Err(error) => return (StatusCode::NOT_FOUND, error.to_string()).into_response(),
    };
    let snapshot = match state.session_zeros.snapshot(session_id).await {
        Ok(value) => value,
        Err(error) => return (StatusCode::NOT_FOUND, error.to_string()).into_response(),
    };
    if snapshot.revision != request.expected_revision {
        return (StatusCode::CONFLICT, "stale Session Zero revision").into_response();
    }
    if let Some(campaign_id) = snapshot.published_campaign_id {
        return Json(serde_json::json!({"campaign_id":campaign_id,"status":"published"}))
            .into_response();
    }
    let member = match snapshot.member_for_account(&account_hash) {
        Some(value) if value.is_host => value,
        _ => return StatusCode::FORBIDDEN.into_response(),
    };
    if let Some(campaign_id) = snapshot.review_campaign_id {
        let campaign_runtime = match state.registry.runtime(campaign_id).await {
            Ok(value) => value,
            Err(error) => return (StatusCode::NOT_FOUND, error.to_string()).into_response(),
        };
        let existing_publication = campaign_runtime
            .store
            .load::<ghostlight_dungeon::session_zero::PublishedSessionZeroSeed>(
                "session_zero_publication.v1",
                &campaign_id.to_string(),
            )
            .ok()
            .flatten()
            .map(|(_, value)| value);
        let review_result = if existing_publication
            .as_ref()
            .is_some_and(|publication| publication.session_zero_id == snapshot.id)
        {
            let publication = existing_publication.unwrap();
            Ok((
                publication.approved_seed_digest,
                publication
                    .membership
                    .members
                    .values()
                    .map(|member| member.account_hash.clone())
                    .collect::<Vec<_>>(),
            ))
        } else {
            commit_contract_review(&campaign_runtime, &snapshot)
        };
        let (seed_digest, member_accounts) = match review_result {
            Ok(value) => value,
            Err(error) => return (StatusCode::CONFLICT, error.to_string()).into_response(),
        };
        for member_account in member_accounts {
            if let Err(error) = select_campaign(&state, &member_account, campaign_id).await {
                tracing::warn!(%error, %campaign_id, "contract-review campaign selection failed");
            }
        }
        return match runtime
            .kernel
            .command(SessionZeroCommand::MarkPublished {
                actor_account_hash: member.account_hash.clone(),
                expected_revision: snapshot.revision,
                campaign_id,
                seed_digest,
            })
            .await
        {
            Ok(_) => {
                if let Err(error) = refresh_mesh(&state).await {
                    tracing::warn!(%error, "Contract Review mesh refresh failed");
                }
                Json(serde_json::json!({"campaign_id":campaign_id,"status":"contract_updated"}))
                    .into_response()
            }
            Err(error) => (StatusCode::CONFLICT, error.to_string()).into_response(),
        };
    }
    let Some(preview) = snapshot.preview.clone() else {
        return (StatusCode::CONFLICT, "final preview is missing").into_response();
    };
    let publication = match publication_from_session(&snapshot, preview.campaign.id) {
        Ok(value) => value,
        Err(error) => return (StatusCode::CONFLICT, error.to_string()).into_response(),
    };
    let seed_digest = publication.approved_seed_digest.clone();
    let member_accounts = publication
        .membership
        .members
        .values()
        .map(|value| value.account_hash.clone())
        .collect::<Vec<_>>();
    let campaign_id = preview.campaign.id;
    let published = state
        .registry
        .publish_session_zero(
            preview.campaign,
            preview.evidence_receipts,
            snapshot.preview_model_receipts.clone(),
            publication,
        )
        .await;
    if let Err(error) = published {
        return (StatusCode::CONFLICT, error.to_string()).into_response();
    }
    for member_account in member_accounts {
        if let Err(error) = select_campaign(&state, &member_account, campaign_id).await {
            tracing::warn!(%error, %campaign_id, "published member campaign selection failed");
        }
    }
    match runtime
        .kernel
        .command(SessionZeroCommand::MarkPublished {
            actor_account_hash: member.account_hash.clone(),
            expected_revision: snapshot.revision,
            campaign_id,
            seed_digest,
        })
        .await
    {
        Ok(_) => {
            if let Err(error) = refresh_mesh(&state).await {
                tracing::warn!(%error, "Session Zero publication mesh refresh failed");
            }
            Json(serde_json::json!({"campaign_id":campaign_id,"status":"published"}))
                .into_response()
        }
        Err(error) => (StatusCode::CONFLICT, error.to_string()).into_response(),
    }
}

fn commit_contract_review(
    runtime: &CampaignRuntime,
    snapshot: &SessionZeroState,
) -> anyhow::Result<(String, Vec<String>)> {
    let campaign_id = snapshot
        .review_campaign_id
        .ok_or_else(|| anyhow::anyhow!("not a Contract Review"))?;
    let (campaign_row, mut campaign) = runtime
        .store
        .load::<Campaign>("campaign.v1", &campaign_id.to_string())?
        .ok_or_else(|| anyhow::anyhow!("reviewed campaign vanished"))?;
    if Some(campaign.revision) != snapshot.review_world_revision {
        anyhow::bail!(
            "the world changed after Contract Review began; start a fresh review against revision {}",
            campaign.revision
        );
    }
    let (_, baseline_membership) = runtime
        .store
        .load::<ghostlight_dungeon::session_zero::CampaignMembership>(
            "campaign_membership.v1",
            &campaign_id.to_string(),
        )?
        .ok_or_else(|| anyhow::anyhow!("campaign membership vanished"))?;
    let (_, baseline_contract) = runtime
        .store
        .load::<ghostlight_dungeon::session_zero::CampaignContract>(
            "campaign_contract.v1",
            &campaign_id.to_string(),
        )?
        .ok_or_else(|| anyhow::anyhow!("campaign contract vanished"))?;
    if snapshot.contract.vault_provider != baseline_contract.vault_provider
        || snapshot.contract.canon_horizon != baseline_contract.canon_horizon
        || snapshot.contract.starting_where != baseline_contract.starting_where
        || snapshot.contract.starting_when != baseline_contract.starting_when
    {
        anyhow::bail!(
            "Contract Review cannot rewrite the Vault, canon horizon, starting time, or established geometry"
        );
    }
    let mut publication = publication_from_session(snapshot, campaign_id)?;
    let baseline_bindings = baseline_membership
        .members
        .iter()
        .map(|(id, member)| (id, (&member.account_hash, &member.actor_id, member.active)))
        .collect::<BTreeMap<_, _>>();
    let proposed_bindings = publication
        .membership
        .members
        .iter()
        .map(|(id, member)| (id, (&member.account_hash, &member.actor_id, member.active)))
        .collect::<BTreeMap<_, _>>();
    if baseline_bindings != proposed_bindings
        || baseline_membership.host_member_id != publication.membership.host_member_id
    {
        anyhow::bail!("Contract Review cannot change campaign membership or actor custody");
    }
    for draft in &publication.approved_brief.characters {
        if draft.relationships.keys().any(|target| {
            !campaign.actors.contains_key(target)
                && !campaign.institutions.contains_key(target)
                && !campaign.gestalts.contains_key(target)
        }) {
            anyhow::bail!("character amendment cites an unknown relationship target");
        }
        let actor = campaign
            .actors
            .get_mut(&draft.actor_id)
            .ok_or_else(|| anyhow::anyhow!("approved character actor vanished"))?;
        actor.name = draft.name.clone();
        actor.capabilities = draft.capabilities.iter().cloned().collect();
        actor.equipment = draft.equipment.iter().cloned().collect();
        actor.conditions = draft.vulnerabilities.iter().cloned().collect();
        actor.obligations = draft.obligations.iter().cloned().collect();
        actor.relationships = draft.relationships.clone();
        actor.goals = draft.goals.clone();
        // Location, knowledge, and memories are established world/history state.
        // The review draft may discuss them, but this command cannot rewrite them.
    }
    let next_governance_epoch = baseline_membership.governance_epoch.saturating_add(1);
    publication.membership.governance_epoch = next_governance_epoch;
    publication.governance.governance_epoch = next_governance_epoch;
    publication.approved_seed_digest = ghostlight_dungeon::session_zero::seed_digest(&(
        campaign_id,
        &publication.approved_brief,
        &publication.membership,
        &publication.governance,
        &publication.contract,
        &publication.dm_persona,
        &publication.boundaries,
    ))?;
    let previous_revision = campaign.revision;
    campaign.revision = campaign.revision.saturating_add(1);
    campaign.events.push(ghostlight_dungeon::domain::Event {
        id: format!("contract-review:{}", campaign.revision),
        at: campaign.world_time,
        kind: "contract_review".into(),
        summary: "The table unanimously adopts a revised campaign contract.".into(),
        actor_ids: vec![],
        institution_ids: vec![],
        gestalt_ids: vec![],
        location_ids: vec![],
        public_channels: vec![],
    });
    let receipt = ghostlight_dungeon::domain::WorldCommitReceipt {
        schema: "ghostlight.world_commit_receipt.v1".into(),
        campaign_id,
        previous_revision,
        revision: campaign.revision,
        command_kind: "unanimous_contract_review".into(),
        committed_at: chrono::Utc::now(),
        roll: None,
    };
    runtime
        .store
        .commit_contract_review(&campaign_row, &campaign, &publication, &receipt)?;
    Ok((
        publication.approved_seed_digest,
        publication
            .membership
            .members
            .values()
            .map(|member| member.account_hash.clone())
            .collect(),
    ))
}

async fn session_zero_simple_command(
    headers: &HeaderMap,
    state: &AppState,
    session_id: uuid::Uuid,
    make: impl FnOnce(String) -> SessionZeroCommand,
) -> Response {
    let account_hash = match authenticated_session(headers, state).await {
        Some(value) => value,
        None => return StatusCode::UNAUTHORIZED.into_response(),
    };
    let runtime = match state.session_zeros.runtime(session_id).await {
        Ok(value) => value,
        Err(error) => return (StatusCode::NOT_FOUND, error.to_string()).into_response(),
    };
    match runtime.kernel.command(make(account_hash.clone())).await {
        Ok(result) => {
            schedule_mesh_refresh(state);
            match session_zero_surface(&result.state, &account_hash) {
                Ok(surface) => Json(surface).into_response(),
                Err(error) => {
                    (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()).into_response()
                }
            }
        }
        Err(error) => (StatusCode::CONFLICT, error.to_string()).into_response(),
    }
}

async fn surface(
    headers: HeaderMap,
    State(state): State<AppState>,
    invite: Option<&str>,
) -> Response {
    let session = match authenticated_session(&headers, &state).await {
        Some(value) => value,
        None => return StatusCode::UNAUTHORIZED.into_response(),
    };
    match state
        .session_zeros
        .active_contract_review_for_account(&session)
        .await
    {
        Ok(Some(id)) => {
            return match state
                .session_zeros
                .snapshot(id)
                .await
                .and_then(|snapshot| session_zero_surface(&snapshot, &session))
            {
                Ok(surface) => Json(surface).into_response(),
                Err(error) => {
                    (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()).into_response()
                }
            };
        }
        Ok(None) => {}
        Err(error) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()).into_response();
        }
    }
    let runtime = match session_runtime(&state, &session).await {
        Ok(Some(value)) => value,
        Ok(None) => {
            return match state.session_zeros.session_for_account(&session).await {
                Ok(Some(id)) => match state
                    .session_zeros
                    .snapshot(id)
                    .await
                    .and_then(|snapshot| session_zero_surface(&snapshot, &session))
                {
                    Ok(surface) => Json(surface).into_response(),
                    Err(error) => {
                        (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()).into_response()
                    }
                },
                Ok(None) => match session_zero_entry_surface(&state, &session, invite).await {
                    Ok(surface) => Json(surface).into_response(),
                    Err(error) => {
                        (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()).into_response()
                    }
                },
                Err(error) => {
                    (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()).into_response()
                }
            };
        }
        Err(error) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()).into_response();
        }
    };
    match load_campaign(&runtime.store) {
        Ok(campaign) => {
            let viewer_actor_id =
                match campaign_member_for_account(&runtime.store, &campaign, &session) {
                    Ok(value) => value.actor_id,
                    Err(error) => {
                        return (StatusCode::FORBIDDEN, error.to_string()).into_response();
                    }
                };
            let mut narrations = runtime
                .store
                .keys("narration_projection.v1")
                .unwrap_or_default()
                .into_iter()
                .filter_map(|key| {
                    runtime
                        .store
                        .load::<NarrationProjection>("narration_projection.v1", &key)
                        .ok()
                        .flatten()
                        .map(|(_, value)| value)
                })
                .filter(|value| value.campaign_id == campaign.id)
                .collect::<Vec<_>>();
            narrations.sort_by_key(|value| value.source_revision);
            let mut projected = player_surface_for_actor(&campaign, &viewer_actor_id, &narrations);
            if let Ok(Some((_, membership))) = runtime
                .store
                .load::<ghostlight_dungeon::session_zero::CampaignMembership>(
                "campaign_membership.v1",
                &campaign.id.to_string(),
            ) {
                let viewer_member_id = membership
                    .member_for_account(&session)
                    .map(|member| member.member_id.clone());
                let time_proposals = runtime
                    .store
                    .load_all::<ghostlight_dungeon::session_zero::TimeAdvanceProposal>(
                        "time_advance_proposal.v1",
                    )
                    .unwrap_or_default()
                    .into_iter()
                    .filter(|proposal| {
                        !proposal.committed && proposal.expected_world_revision == campaign.revision
                    })
                    .collect::<Vec<_>>();
                let travel_proposals = runtime
                    .store
                    .load_all::<ghostlight_dungeon::session_zero::GroupTravelProposal>(
                        "group_travel_proposal.v1",
                    )
                    .unwrap_or_default()
                    .into_iter()
                    .filter(|proposal| {
                        !proposal.committed && proposal.expected_world_revision == campaign.revision
                    })
                    .collect::<Vec<_>>();
                let cell_budget_proposals = runtime
                    .store
                    .load_all::<ghostlight_dungeon::session_zero::CellBudgetProposal>(
                        "cell_budget_proposal.v1",
                    )
                    .unwrap_or_default()
                    .into_iter()
                    .filter(|proposal| {
                        !proposal.committed
                            && proposal.expected_world_revision == campaign.revision
                            && proposal.expected_resolution_epoch
                                == campaign.resolution_policy.resolution_epoch
                    })
                    .collect::<Vec<_>>();
                projected["governance"] = serde_json::json!({
                    "viewer_member_id":viewer_member_id,
                    "active_member_count":membership.members.values().filter(|member|member.active).count(),
                    "pooled_cell_allowance":membership.pooled_cell_allowance(),
                    "time_proposals":time_proposals,
                    "travel_proposals":travel_proposals,
                    "cell_budget_proposals":cell_budget_proposals,
                });
                if let Some(children) = projected
                    .pointer_mut("/surface/root/children")
                    .and_then(serde_json::Value::as_array_mut)
                {
                    for proposal in &time_proposals {
                        children.push(eve_button(
                            &format!("governance.time.approve.{}", proposal.id),
                            &format!("Approve time advance: {} minutes", proposal.minutes),
                            "governance.time.approve",
                            serde_json::json!({"expected_revision":campaign.revision,"proposal_id":proposal.id}),
                            &[],
                        ));
                    }
                    for proposal in &travel_proposals {
                        children.push(eve_button(
                            &format!("governance.travel.approve.{}", proposal.id),
                            "Approve group travel",
                            "governance.travel.approve",
                            serde_json::json!({"expected_revision":campaign.revision,"proposal_id":proposal.id}),
                            &[],
                        ));
                    }
                    for proposal in &cell_budget_proposals {
                        children.push(eve_button(
                            &format!("governance.cells.approve.{}", proposal.id),
                            &format!("Approve Persona-cell budget {}", proposal.active_cell_budget),
                            "governance.cells.approve",
                            serde_json::json!({"expected_revision":campaign.revision,"proposal_id":proposal.id}),
                            &[],
                        ));
                    }
                }
            }
            Json(projected).into_response()
        }
        Err(error) => (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()).into_response(),
    }
}

async fn session_zero_entry_surface(
    state: &AppState,
    account_hash: &str,
    invite: Option<&str>,
) -> anyhow::Result<serde_json::Value> {
    let mut campaign_choices = Vec::new();
    for id in state.registry.list().await {
        let runtime = state.registry.runtime(id).await?;
        let campaign = load_campaign(&runtime.store)?;
        if campaign_member_for_account(&runtime.store, &campaign, account_hash).is_ok() {
            campaign_choices.push(serde_json::json!({
                "id":format!("campaign-choice:{id}"),"kind":"control.button",
                "props":{"label":format!("Continue {}",campaign.name),"command":"campaign.select","action":{"command":"campaign.select","campaign_id":id}},"children":[]
            }));
        }
    }
    let mut children = vec![
        serde_json::json!({"id":"entry.intro","kind":"card","props":{"title":"Begin Session Zero"},"children":[
            {"id":"entry.intro.text","kind":"text","props":{"value":"Build the campaign with the DM before the world becomes canonical. Tone, boundaries, characters, evidence gaps, and opening pressure remain negotiable until everyone approves."},"children":[]}
        ]}),
        serde_json::json!({"id":"entry.name","kind":"control.input.text","props":{"label":"Draft name","placeholder":"The campaign you are about to regret caring about"},"stateBindings":[eve_local_draft("name","string")],"children":[]}),
        serde_json::json!({"id":"entry.vault","kind":"control.input.text","props":{"label":"Lore Vault","value":"Aetheria","placeholder":"Aetheria"},"stateBindings":[eve_local_draft("vault_provider","string")],"children":[]}),
        serde_json::json!({"id":"entry.display-name","kind":"control.input.text","props":{"label":"Your display name"},"stateBindings":[eve_local_draft("display_name","string")],"children":[]}),
        eve_button(
            "entry.begin",
            "Begin Session Zero",
            "session_zero.begin",
            serde_json::json!({}),
            &["name", "vault_provider", "display_name"],
        ),
        serde_json::json!({"id":"entry.join-heading","kind":"text.title","props":{"value":"Join a table"},"children":[]}),
        serde_json::json!({"id":"entry.invite","kind":"control.input.text","props":{"label":"Invitation token","value":invite.unwrap_or_default()},"stateBindings":[eve_local_draft("invite_token","string")],"children":[]}),
        serde_json::json!({"id":"entry.join-name","kind":"control.input.text","props":{"label":"Your display name"},"stateBindings":[eve_local_draft("display_name","string")],"children":[]}),
        eve_button(
            "entry.join",
            "Join Session Zero",
            "session_zero.join",
            serde_json::json!({}),
            &["invite_token", "display_name"],
        ),
    ];
    if !campaign_choices.is_empty() {
        children.push(serde_json::json!({"id":"entry.campaigns","kind":"card","props":{"title":"Your campaigns"},"children":campaign_choices}));
    }
    children.push(eve_button(
        "entry.logout",
        "Sign out",
        "app.auth.logout",
        serde_json::json!({}),
        &[],
    ));
    Ok(serde_json::json!({
        "type":"surface-state","schema":"gamecult.eve.surface.v1","providerId":EVE_PROVIDER_ID,"providerKind":"narrative.session-zero-entry",
        "title":"Ghostlight Dungeon","version":0,"updatedAtUtc":Utc::now().to_rfc3339(),
        "surface":{"id":EVE_SURFACE_ID,"root":{"id":"entry.root","kind":"surface","props":{},"children":children},"styles":{"tokens":{"colorBackground":"#0c1110","colorPanel":"#17201d","colorText":"#e8e1cf","colorMuted":"#9aa69f","colorAccent":"#d49b58"}}},
        "commands":[
            eve_command_descriptor("session_zero.begin","ghostlight.session_zero_begin.v1", &["name","vault_provider","display_name"], "SessionZeroKernel"),
            eve_command_descriptor("session_zero.join","ghostlight.session_zero_join.v1", &["invite_token","display_name"], "SessionZeroKernel"),
            eve_command_descriptor("campaign.select","ghostlight.campaign_select.v1", &[], "campaign_membership.v1"),
            eve_command_descriptor("app.auth.logout","ghostlight.app_logout.v1", &[], "ghostlight.app_session.v1")
        ]
    }))
}

fn eve_local_draft(name: &str, value_kind: &str) -> serde_json::Value {
    serde_json::json!({"targetProp":"value","pointerId":format!("draft:{name}"),"sourceId":"renderer","schemaId":"gamecult.eve.local_draft.v1","routeKind":"local","bindingName":name,"documentId":"ghostlight.play.drafts","fieldPath":name,"valueKind":value_kind,"accessMode":"local-draft","authority":"renderer-ephemeral"})
}

fn eve_button(
    id: &str,
    label: &str,
    command: &str,
    action: serde_json::Value,
    bindings: &[&str],
) -> serde_json::Value {
    let mut action = action.as_object().cloned().unwrap_or_default();
    action.insert("command".into(), serde_json::Value::String(command.into()));
    serde_json::json!({"id":id,"kind":"control.button","props":{"label":label,"command":command,"action":action,"captureBindings":bindings},"children":[]})
}

fn eve_command_descriptor(
    command: &str,
    payload_schema: &str,
    bindings: &[&str],
    authority: &str,
) -> serde_json::Value {
    serde_json::json!({"schema":"gamecult.eve.command.v1","command":command,"payloadSchema":payload_schema,"captureBindings":bindings,"transport":"https-json","authority":authority})
}

async fn revision_events(headers: HeaderMap, State(state): State<AppState>) -> Response {
    let account_hash = match authenticated_session(&headers, &state).await {
        Some(value) => value,
        None => return StatusCode::UNAUTHORIZED.into_response(),
    };
    let stream = async_stream::stream! {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(2));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        let mut previous = None::<String>;
        loop {
            interval.tick().await;
            let notice = match session_runtime(&state, &account_hash).await {
                Ok(Some(runtime)) => load_campaign(&runtime.store).ok().map(|campaign| {
                    serde_json::json!({
                        "kind":"campaign",
                        "id":campaign.id,
                        "revision":campaign.revision,
                        "resolution_epoch":campaign.resolution_policy.resolution_epoch,
                        "provider_configuration_epoch":campaign.resolution_policy.provider_configuration_epoch,
                    })
                }),
                Ok(None) => match state.session_zeros.session_for_account(&account_hash).await {
                    Ok(Some(id)) => state.session_zeros.snapshot(id).await.ok().map(|session| {
                        serde_json::json!({"kind":"session_zero","id":id,"revision":session.revision})
                    }),
                    _ => Some(serde_json::json!({"kind":"entry","revision":0})),
                },
                Err(_) => None,
            };
            let Some(notice) = notice else { continue };
            let encoded = notice.to_string();
            if previous.as_deref() == Some(encoded.as_str()) {
                continue;
            }
            previous = Some(encoded.clone());
            yield Ok::<SseEvent, Infallible>(SseEvent::default().event("revision").data(encoded));
        }
    };
    Sse::new(stream)
        .keep_alive(
            KeepAlive::new()
                .interval(std::time::Duration::from_secs(15))
                .text("revision-heartbeat"),
        )
        .into_response()
}

#[derive(Deserialize)]
struct DestinationRequest {
    origin_location_id: String,
    destination: String,
}

async fn compile_destination(
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(request): Json<DestinationRequest>,
) -> Response {
    let session = match authenticated_session(&headers, &state).await {
        Some(value) => value,
        None => return StatusCode::UNAUTHORIZED.into_response(),
    };
    let _live = LiveTurnGuard::enter(&state).await;
    let runtime = match session_runtime(&state, &session).await {
        Ok(Some(value)) => value,
        Ok(None) => {
            return (StatusCode::NOT_FOUND, "session has no selected campaign").into_response();
        }
        Err(error) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()).into_response();
        }
    };
    let campaign = match load_campaign(&runtime.store) {
        Ok(value) => value,
        Err(error) => return (StatusCode::NOT_FOUND, error.to_string()).into_response(),
    };
    let Some(compiler) = &state.compiler else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            "Model provider credential is unavailable",
        )
            .into_response();
    };
    match compiler
        .compile_destination(&campaign, &request.origin_location_id, &request.destination)
        .await
    {
        Ok((preview, model_receipts)) => {
            let id = uuid::Uuid::new_v4().to_string();
            let mut previews = state.expansion_previews.lock().await;
            previews.retain(|_, existing| existing.session_hash != session);
            previews.insert(
                id.clone(),
                OwnedPreview {
                    session_hash: session,
                    value: preview.clone(),
                    model_receipts: model_receipts.clone(),
                },
            );
            Json(serde_json::json!({
                "preview_id":id,
                "preview":region_expansion_preview_projection(&preview),
            }))
            .into_response()
        }
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, error.to_string()).into_response(),
    }
}

async fn approve_destination(
    Path(preview_id): Path<String>,
    headers: HeaderMap,
    State(state): State<AppState>,
) -> Response {
    let session = match authenticated_session(&headers, &state).await {
        Some(value) => value,
        None => return StatusCode::UNAUTHORIZED.into_response(),
    };
    let mut previews = state.expansion_previews.lock().await;
    let Some(owned) = previews.get(&preview_id).cloned() else {
        return (StatusCode::NOT_FOUND, "preview missing or already consumed").into_response();
    };
    if owned.session_hash != session {
        return StatusCode::FORBIDDEN.into_response();
    }
    previews.remove(&preview_id);
    drop(previews);
    let preview = owned.value;
    let model_receipts = owned.model_receipts;
    let runtime = match session_runtime(&state, &session).await {
        Ok(Some(value)) => value,
        Ok(None) => {
            return (StatusCode::NOT_FOUND, "session has no selected campaign").into_response();
        }
        Err(error) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()).into_response();
        }
    };
    match runtime
        .kernel
        .command(WorldCommand::ExpandRegion {
            expected_revision: preview.expected_revision,
            expansion: preview.expansion,
            evidence_receipts: preview.evidence_receipts,
            canon_candidates: preview.canon_candidates,
            model_stage_receipts: model_receipts,
        })
        .await
    {
        Ok(value) => Json(player_command_projection(&value, None)).into_response(),
        Err(error) => (StatusCode::CONFLICT, error.to_string()).into_response(),
    }
}

async fn compile_fission(
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(request): Json<GestaltFissionRequest>,
) -> Response {
    let session = match authenticated_session(&headers, &state).await {
        Some(value) => value,
        None => return StatusCode::UNAUTHORIZED.into_response(),
    };
    let _live = LiveTurnGuard::enter(&state).await;
    let runtime = match session_runtime(&state, &session).await {
        Ok(Some(value)) => value,
        Ok(None) => {
            return (StatusCode::NOT_FOUND, "session has no selected campaign").into_response();
        }
        Err(error) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()).into_response();
        }
    };
    let campaign = match load_campaign(&runtime.store) {
        Ok(value) => value,
        Err(error) => return (StatusCode::NOT_FOUND, error.to_string()).into_response(),
    };
    let Some(compiler) = &state.compiler else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            "Model provider credential is unavailable",
        )
            .into_response();
    };
    match compiler.compile_fission(&campaign, request).await {
        Ok((preview, evidence_receipts, model_receipts)) => {
            let id = uuid::Uuid::new_v4().to_string();
            let mut previews = state.fission_previews.lock().await;
            previews.retain(|_, existing| existing.session_hash != session);
            previews.insert(
                id.clone(),
                OwnedFissionPreview {
                    session_hash: session,
                    value: preview.clone(),
                    evidence_receipts,
                    model_receipts: model_receipts.clone(),
                },
            );
            Json(serde_json::json!({
                "preview_id":id,
                "preview":gestalt_fission_preview_projection(&preview),
            }))
            .into_response()
        }
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, error.to_string()).into_response(),
    }
}

async fn approve_fission(
    Path(preview_id): Path<String>,
    headers: HeaderMap,
    State(state): State<AppState>,
) -> Response {
    let session = match authenticated_session(&headers, &state).await {
        Some(value) => value,
        None => return StatusCode::UNAUTHORIZED.into_response(),
    };
    let _live = LiveTurnGuard::enter(&state).await;
    let mut previews = state.fission_previews.lock().await;
    let Some(owned) = previews.get(&preview_id).cloned() else {
        return (StatusCode::NOT_FOUND, "preview missing or already consumed").into_response();
    };
    if owned.session_hash != session {
        return StatusCode::FORBIDDEN.into_response();
    }
    previews.remove(&preview_id);
    drop(previews);
    let runtime = match session_runtime(&state, &session).await {
        Ok(Some(value)) => value,
        Ok(None) => {
            return (StatusCode::NOT_FOUND, "session has no selected campaign").into_response();
        }
        Err(error) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()).into_response();
        }
    };
    match runtime
        .kernel
        .command(WorldCommand::FissionGestalt {
            expected_revision: owned.value.expected_world_revision,
            preview: owned.value,
            evidence_receipts: owned.evidence_receipts,
            model_stage_receipts: owned.model_receipts,
        })
        .await
    {
        Ok(value) => {
            if let Err(error) = refresh_mesh(&state).await {
                tracing::warn!(%error, "gestalt fission CultMesh publication failed");
            }
            Json(player_command_projection(&value, None)).into_response()
        }
        Err(error) => (StatusCode::CONFLICT, error.to_string()).into_response(),
    }
}

async fn resolve_npc_initiative(
    state: &AppState,
    runtime: &CampaignRuntime,
    reaction: &CommandResult,
) -> anyhow::Result<serde_json::Value> {
    let CommandResult::Committed { campaign, .. } = reaction else {
        return Ok(serde_json::Value::Null);
    };
    let Some(proposal) = ghostlight_dungeon::initiative::winner(&campaign.pending_world_proposals)
    else {
        return Ok(serde_json::Value::Null);
    };
    let assessor = state
        .assessor
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("NPC initiative requires the action assessor"))?;
    let intent = ActionIntent {
        actor_id: proposal.actor_id.clone(),
        description: proposal.intent.clone(),
        intended_effect: proposal.intended_effect.clone(),
    };
    let (contract, boundaries) = campaign_model_policy(&runtime.store, campaign.id);
    let permissions = runtime
        .store
        .load::<ghostlight_dungeon::session_zero::CampaignMembership>(
            "campaign_membership.v1",
            &campaign.id.to_string(),
        )?
        .and_then(|(_, membership)| {
            membership
                .extraordinary_permissions
                .get(&intent.actor_id)
                .cloned()
        })
        .unwrap_or_default();
    let (assessment, receipt) = assessor
        .assess_with_context(
            campaign,
            intent,
            &permissions,
            contract.as_ref(),
            &boundaries,
        )
        .await?;
    let _ = runtime.store.insert(
        "persona_stage_receipt.v1",
        "ghostlight.persona_stage_receipt.v1",
        receipt.storage_key(),
        &receipt,
    );
    let resolved = runtime
        .kernel
        .command(WorldCommand::ResolveNpcAction {
            expected_revision: campaign.revision,
            proposal: proposal.clone(),
            assessment,
        })
        .await?;
    Ok(serde_json::to_value(resolved)?)
}

async fn publish_latest_narration(
    state: &AppState,
    runtime: &CampaignRuntime,
) -> anyhow::Result<Option<NarrationProjection>> {
    let campaign = load_campaign(&runtime.store)?;
    let key = format!("{}:{}", campaign.id, campaign.revision);
    if let Some((_, existing)) = runtime
        .store
        .load::<NarrationProjection>("narration_projection.v1", &key)?
    {
        return Ok(Some(existing));
    }
    let model = state
        .model
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("narration requires the model provider"))?;
    let narrator = Narrator {
        model: model.clone(),
        model_name: MODEL_CAPABLE.into(),
    };
    let (projection, receipt) = narrator.project(&runtime.store, &campaign).await?;
    runtime.store.insert(
        "narration_projection.v1",
        "ghostlight.narration_projection.v1",
        &projection.id,
        &projection,
    )?;
    let _ = runtime.store.insert(
        "persona_stage_receipt.v1",
        "ghostlight.persona_stage_receipt.v1",
        receipt.storage_key(),
        &receipt,
    );
    Ok(Some(projection))
}

async fn propose_time_advance(
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(request): Json<TimeAdvanceRequest>,
) -> Response {
    governed_campaign_command(&headers, &state, |member| {
        WorldCommand::ProposeTimeAdvance {
            expected_revision: request.expected_revision,
            member_id: member.member_id,
            minutes: request.minutes,
        }
    })
    .await
}

async fn approve_time_advance(
    Path(proposal_id): Path<String>,
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(request): Json<SessionZeroRevisionRequest>,
) -> Response {
    governed_campaign_command(&headers, &state, |member| {
        WorldCommand::ApproveTimeAdvance {
            expected_revision: request.expected_revision,
            proposal_id,
            member_id: member.member_id,
        }
    })
    .await
}

async fn propose_group_travel(
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(request): Json<GroupTravelRequest>,
) -> Response {
    governed_campaign_command(&headers, &state, |member| {
        WorldCommand::ProposeGroupTravel {
            expected_revision: request.expected_revision,
            member_id: member.member_id,
            destination_location_id: request.destination_location_id,
        }
    })
    .await
}

async fn approve_group_travel(
    Path(proposal_id): Path<String>,
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(request): Json<SessionZeroRevisionRequest>,
) -> Response {
    governed_campaign_command(&headers, &state, |member| {
        WorldCommand::ApproveGroupTravel {
            expected_revision: request.expected_revision,
            proposal_id,
            member_id: member.member_id,
        }
    })
    .await
}

async fn propose_cell_budget(
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(request): Json<CellBudgetRequest>,
) -> Response {
    governed_campaign_command(&headers, &state, |member| {
        WorldCommand::ProposeResolutionBudget {
            expected_revision: request.expected_revision,
            expected_resolution_epoch: request.expected_resolution_epoch,
            member_id: member.member_id,
            active_cell_budget: request.active_cell_budget,
        }
    })
    .await
}

async fn approve_cell_budget(
    Path(proposal_id): Path<String>,
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(request): Json<SessionZeroRevisionRequest>,
) -> Response {
    governed_campaign_command(&headers, &state, |member| {
        WorldCommand::ApproveResolutionBudget {
            expected_revision: request.expected_revision,
            proposal_id,
            member_id: member.member_id,
        }
    })
    .await
}

async fn governed_campaign_command(
    headers: &HeaderMap,
    state: &AppState,
    make: impl FnOnce(ghostlight_dungeon::session_zero::CampaignMember) -> WorldCommand,
) -> Response {
    let account_hash = match authenticated_session(headers, state).await {
        Some(value) => value,
        None => return StatusCode::UNAUTHORIZED.into_response(),
    };
    let runtime = match session_runtime(state, &account_hash).await {
        Ok(Some(value)) => value,
        Ok(None) => return (StatusCode::NOT_FOUND, "session has no campaign").into_response(),
        Err(error) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()).into_response();
        }
    };
    let campaign = match load_campaign(&runtime.store) {
        Ok(value) => value,
        Err(error) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()).into_response();
        }
    };
    let member = match campaign_member_for_account(&runtime.store, &campaign, &account_hash) {
        Ok(value) => value,
        Err(error) => return (StatusCode::FORBIDDEN, error.to_string()).into_response(),
    };
    match runtime.kernel.command(make(member)).await {
        Ok(result) => {
            if matches!(result, CommandResult::Committed { .. })
                && let Err(error) = refresh_mesh(state).await
            {
                tracing::warn!(%error, "governed time advance mesh refresh failed");
            }
            Json(player_command_projection(&result, None)).into_response()
        }
        Err(KernelError::Stale { expected, actual }) => (
            StatusCode::CONFLICT,
            Json(serde_json::json!({"error":"stale revision","expected":expected,"actual":actual})),
        )
            .into_response(),
        Err(error) => (StatusCode::CONFLICT, error.to_string()).into_response(),
    }
}

async fn command(
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(mut command): Json<WorldCommand>,
) -> Response {
    let session = match authenticated_session(&headers, &state).await {
        Some(value) => value,
        None => return StatusCode::UNAUTHORIZED.into_response(),
    };
    let _live = LiveTurnGuard::enter(&state).await;
    let runtime = match session_runtime(&state, &session).await {
        Ok(Some(value)) => value,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(ErrorBody {
                    error: "session has no selected campaign".into(),
                }),
            )
                .into_response();
        }
        Err(error) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorBody {
                    error: error.to_string(),
                }),
            )
                .into_response();
        }
    };
    let admission_campaign = match load_campaign(&runtime.store) {
        Ok(campaign) => campaign,
        Err(error) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorBody {
                    error: error.to_string(),
                }),
            )
                .into_response();
        }
    };
    let player_id = match campaign_member_for_account(&runtime.store, &admission_campaign, &session)
    {
        Ok(member) => member.actor_id,
        Err(error) => {
            return (
                StatusCode::FORBIDDEN,
                Json(ErrorBody {
                    error: error.to_string(),
                }),
            )
                .into_response();
        }
    };
    let campaign_membership = runtime
        .store
        .load::<ghostlight_dungeon::session_zero::CampaignMembership>(
            "campaign_membership.v1",
            &admission_campaign.id.to_string(),
        )
        .ok()
        .flatten()
        .map(|(_, membership)| membership);
    let cooperative_member_count = campaign_membership
        .as_ref()
        .map(|membership| {
            membership
                .members
                .values()
                .filter(|member| member.active)
                .count()
        })
        .unwrap_or(1);
    if cooperative_member_count > 1
        && matches!(
            command,
            WorldCommand::Wait { .. } | WorldCommand::SetResolutionBudget { .. }
        )
    {
        return (
            StatusCode::FORBIDDEN,
            Json(ErrorBody {
                error: "co-op time and Persona-cell budget changes require unanimous governance"
                    .into(),
            }),
        )
            .into_response();
    }
    if let WorldCommand::SetResolutionBudget {
        active_cell_budget, ..
    } = &command
        && campaign_membership
            .as_ref()
            .is_some_and(|membership| *active_cell_budget > membership.pooled_cell_allowance())
    {
        return (
            StatusCode::FORBIDDEN,
            Json(ErrorBody {
                error: "requested Persona-cell budget exceeds the campaign entitlement pool".into(),
            }),
        )
            .into_response();
    }
    if !player_http_command_allowed(&command, &player_id) {
        return (
            StatusCode::FORBIDDEN,
            Json(ErrorBody {
                error: "command is not admitted through the player HTTP boundary".into(),
            }),
        )
            .into_response();
    }
    if let Err(error) = validate_player_http_command(&command) {
        return (StatusCode::UNPROCESSABLE_ENTITY, Json(ErrorBody { error })).into_response();
    }
    if let Err(error) = process_due_ticks(
        &state,
        &runtime,
        ghostlight_dungeon::domain::TickSource::ReturnCatchUp,
        false,
    )
    .await
    {
        return (
            StatusCode::CONFLICT,
            Json(ErrorBody {
                error: error.to_string(),
            }),
        )
            .into_response();
    }
    if let WorldCommand::Assess {
        intent, proposal, ..
    } = &mut command
    {
        if proposal.is_none() {
            let Some(assessor) = &state.assessor else {
                return (
                    StatusCode::SERVICE_UNAVAILABLE,
                    Json(ErrorBody {
                        error: "Model assessor is unavailable".into(),
                    }),
                )
                    .into_response();
            };
            let campaign = match load_campaign(&runtime.store) {
                Ok(value) => value,
                Err(error) => {
                    return (
                        StatusCode::NOT_FOUND,
                        Json(ErrorBody {
                            error: error.to_string(),
                        }),
                    )
                        .into_response();
                }
            };
            let extraordinary_permissions = runtime
                .store
                .load::<ghostlight_dungeon::session_zero::CampaignMembership>(
                    "campaign_membership.v1",
                    &campaign.id.to_string(),
                )
                .ok()
                .flatten()
                .and_then(|(_, membership)| {
                    membership
                        .extraordinary_permissions
                        .get(&intent.actor_id)
                        .cloned()
                })
                .unwrap_or_default();
            let (contract, boundaries) = campaign_model_policy(&runtime.store, campaign.id);
            match assessor
                .assess_with_context(
                    &campaign,
                    intent.clone(),
                    &extraordinary_permissions,
                    contract.as_ref(),
                    &boundaries,
                )
                .await
            {
                Ok((assessment, receipt)) => {
                    if let Ok(Some((_, membership))) =
                        runtime
                            .store
                            .load::<ghostlight_dungeon::session_zero::CampaignMembership>(
                                "campaign_membership.v1",
                                &campaign.id.to_string(),
                            )
                        && membership
                            .members
                            .values()
                            .filter(|member| member.active)
                            .count()
                            > 1
                    {
                        let controlled = membership.controlled_actor_ids();
                        if assessment_effects(&assessment)
                            .any(|effect| !effect.actor_moves.is_empty())
                        {
                            return (
                                StatusCode::UNPROCESSABLE_ENTITY,
                                Json(ErrorBody {
                                    error:
                                        "group travel requires a unanimous revision-bound proposal"
                                            .into(),
                                }),
                            )
                                .into_response();
                        }
                        if assessment_effects(&assessment).any(|effect| {
                            effect
                                .actor_conditions
                                .keys()
                                .chain(effect.actor_knowledge_additions.keys())
                                .chain(effect.actor_relationship_updates.keys())
                                .any(|target| {
                                    target != &intent.actor_id && controlled.contains(target)
                                })
                        }) {
                            return (
                                StatusCode::UNPROCESSABLE_ENTITY,
                                Json(ErrorBody {
                                    error: "effects on another player character are unsupported"
                                        .into(),
                                }),
                            )
                                .into_response();
                        }
                    }
                    let _ = runtime.store.insert(
                        "persona_stage_receipt.v1",
                        "ghostlight.persona_stage_receipt.v1",
                        receipt.storage_key(),
                        &receipt,
                    );
                    *proposal = Some(assessment);
                }
                Err(error) => {
                    return (
                        StatusCode::UNPROCESSABLE_ENTITY,
                        Json(ErrorBody {
                            error: error.to_string(),
                        }),
                    )
                        .into_response();
                }
            }
        }
    }
    let should_react = matches!(
        &command,
        WorldCommand::Attempt { .. } | WorldCommand::Speak { .. }
    );
    let committed_reaction_stimulus = reaction_stimulus(&command);
    let command_kind = serde_json::to_value(&command)
        .ok()
        .and_then(|value| {
            value
                .get("type")
                .and_then(|value| value.as_str())
                .map(str::to_owned)
        })
        .unwrap_or_else(|| "unknown".into());
    match runtime.kernel.command(command).await {
        Ok(result) => {
            if should_react {
                if let ghostlight_dungeon::kernel::CommandResult::Committed { campaign, .. } =
                    &result
                {
                    if let Some(model) = &state.model {
                        let summary = committed_reaction_stimulus.clone().unwrap_or_else(|| {
                            campaign
                                .transcript
                                .last()
                                .map(|turn| turn.text.clone())
                                .unwrap_or_else(|| "A consequential event occurred.".into())
                        });
                        let mut reaction_campaign = campaign.clone();
                        let mut presence_result = None;
                        if !campaign.gestalts.is_empty() {
                            let planner = GestaltPresencePlanner {
                                model: model.clone(),
                                model_name: MODEL_FAST.into(),
                            };
                            match planner.plan(campaign, &summary).await {
                                Ok((plan, receipt)) => {
                                    let _ = runtime.store.insert(
                                        "persona_stage_receipt.v1",
                                        "ghostlight.persona_stage_receipt.v1",
                                        receipt.storage_key(),
                                        &receipt,
                                    );
                                    if !plan.individuations.is_empty()
                                        || !plan.promotions.is_empty()
                                        || !plan.demotions.is_empty()
                                    {
                                        match runtime.kernel.command(WorldCommand::ReconcileGestaltPresence {
                                            expected_revision: campaign.revision,
                                            reason: summary.clone(),
                                            plan,
                                        }).await {
                                            Ok(committed @ ghostlight_dungeon::kernel::CommandResult::Committed { .. }) => {
                                                if let ghostlight_dungeon::kernel::CommandResult::Committed { campaign, .. } = &committed {
                                                    reaction_campaign = campaign.clone();
                                                }
                                                presence_result = Some(committed);
                                            }
                                            Ok(_) => unreachable!(),
                                            Err(error) => return (StatusCode::CONFLICT, Json(ErrorBody { error: error.to_string() })).into_response(),
                                        }
                                    }
                                }
                                Err(error) => {
                                    return (
                                        StatusCode::BAD_GATEWAY,
                                        Json(ErrorBody {
                                            error: error.to_string(),
                                        }),
                                    )
                                        .into_response();
                                }
                            }
                        }
                        if reaction_campaign.actors.len() > 1 {
                            let engine = PersonaProjectionEngine {
                                model: model.clone(),
                                permit: Arc::new(SnapshotPermit::new(
                                    runtime.store.clone(),
                                    reaction_campaign.id,
                                    reaction_campaign.revision,
                                )),
                                projector_model: MODEL_FAST.into(),
                                persona_model: MODEL_CAPABLE.into(),
                                interpreter_model: MODEL_FAST.into(),
                            };
                            match appraise_present(engine, &reaction_campaign, &summary).await {
                                Ok(wave) if !wave.reactions.is_empty() => {
                                    for receipt in wave.receipts {
                                        let _ = runtime.store.insert(
                                            "persona_stage_receipt.v1",
                                            "ghostlight.persona_stage_receipt.v1",
                                            receipt.storage_key(),
                                            &receipt,
                                        );
                                    }
                                    match runtime
                                        .kernel
                                        .command(WorldCommand::ResolveReactionWave {
                                            expected_revision: reaction_campaign.revision,
                                            event_summary: summary,
                                            reactions: wave.reactions,
                                        })
                                        .await
                                    {
                                        Ok(reaction) => {
                                            let _initiative = match resolve_npc_initiative(
                                                &state, &runtime, &reaction,
                                            )
                                            .await
                                            {
                                                Ok(value) => value,
                                                Err(error) => {
                                                    return (
                                                        StatusCode::BAD_GATEWAY,
                                                        Json(ErrorBody {
                                                            error: error.to_string(),
                                                        }),
                                                    )
                                                        .into_response();
                                                }
                                            };
                                            let narration =
                                                publish_latest_narration(&state, &runtime)
                                                    .await
                                                    .ok()
                                                    .flatten();
                                            if let Err(error) = refresh_mesh(&state).await {
                                                tracing::warn!(%error, "post-command CultMesh publication failed");
                                            }
                                            return Json(player_command_projection(
                                                &result, narration,
                                            ))
                                            .into_response();
                                        }
                                        Err(error) => {
                                            return (
                                                StatusCode::CONFLICT,
                                                Json(ErrorBody {
                                                    error: error.to_string(),
                                                }),
                                            )
                                                .into_response();
                                        }
                                    }
                                }
                                Ok(_) => {}
                                Err(error) => {
                                    return (
                                        StatusCode::BAD_GATEWAY,
                                        Json(ErrorBody {
                                            error: error.to_string(),
                                        }),
                                    )
                                        .into_response();
                                }
                            }
                        }
                        if presence_result.is_some() {
                            let narration = publish_latest_narration(&state, &runtime)
                                .await
                                .ok()
                                .flatten();
                            if let Err(error) = refresh_mesh(&state).await {
                                tracing::warn!(%error, "post-command CultMesh publication failed");
                            }
                            return Json(player_command_projection(&result, narration))
                                .into_response();
                        }
                    }
                }
            }
            if matches!(
                &result,
                CommandResult::Committed { .. } | CommandResult::Created { .. }
            ) {
                let narration = publish_latest_narration(&state, &runtime)
                    .await
                    .ok()
                    .flatten();
                if let Err(error) = refresh_mesh(&state).await {
                    tracing::warn!(%error, "post-command CultMesh publication failed");
                }
                Json(player_command_projection(&result, narration)).into_response()
            } else {
                Json(player_command_projection(&result, None)).into_response()
            }
        }
        Err(error) => {
            if let KernelError::StaleAssessment { intent, .. } = &error {
                let Some(assessor) = &state.assessor else {
                    return (
                        StatusCode::SERVICE_UNAVAILABLE,
                        Json(ErrorBody {
                            error: "Model assessor is unavailable".into(),
                        }),
                    )
                        .into_response();
                };
                let campaign = match load_campaign(&runtime.store) {
                    Ok(value) => value,
                    Err(load_error) => {
                        return (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            Json(ErrorBody {
                                error: load_error.to_string(),
                            }),
                        )
                            .into_response();
                    }
                };
                let membership = runtime
                    .store
                    .load::<ghostlight_dungeon::session_zero::CampaignMembership>(
                        "campaign_membership.v1",
                        &campaign.id.to_string(),
                    )
                    .ok()
                    .flatten()
                    .map(|(_, value)| value);
                let permissions = membership
                    .as_ref()
                    .and_then(|membership| {
                        membership.extraordinary_permissions.get(&intent.actor_id)
                    })
                    .cloned()
                    .unwrap_or_default();
                let (contract, boundaries) = campaign_model_policy(&runtime.store, campaign.id);
                match assessor
                    .assess_with_context(
                        &campaign,
                        intent.clone(),
                        &permissions,
                        contract.as_ref(),
                        &boundaries,
                    )
                    .await
                {
                    Ok((assessment, receipt)) => {
                        let _ = runtime.store.insert(
                            "persona_stage_receipt.v1",
                            "ghostlight.persona_stage_receipt.v1",
                            receipt.storage_key(),
                            &receipt,
                        );
                        return match runtime
                            .kernel
                            .command(WorldCommand::Assess {
                                expected_revision: campaign.revision,
                                intent: intent.clone(),
                                proposal: Some(assessment),
                            })
                            .await
                        {
                            Ok(result) => {
                                Json(player_command_projection(&result, None)).into_response()
                            }
                            Err(recompile_error) => (
                                StatusCode::CONFLICT,
                                Json(ErrorBody {
                                    error: recompile_error.to_string(),
                                }),
                            )
                                .into_response(),
                        };
                    }
                    Err(recompile_error) => {
                        return (
                            StatusCode::BAD_GATEWAY,
                            Json(ErrorBody {
                                error: recompile_error.to_string(),
                            }),
                        )
                            .into_response();
                    }
                }
            }
            if let Ok(campaign) = load_campaign(&runtime.store) {
                let receipt = RejectedProposalReceipt {
                    schema: "ghostlight.rejected_proposal_receipt.v1".into(),
                    id: uuid::Uuid::new_v4().to_string(),
                    campaign_id: campaign.id,
                    revision: campaign.revision,
                    command_kind,
                    reason: error.to_string(),
                    rejected_at: chrono::Utc::now(),
                };
                let _ = runtime.store.insert(
                    "rejected_proposal_receipt.v1",
                    "ghostlight.rejected_proposal_receipt.v1",
                    &receipt.id,
                    &receipt,
                );
            }
            (
                StatusCode::CONFLICT,
                Json(ErrorBody {
                    error: error.to_string(),
                }),
            )
                .into_response()
        }
    }
}

fn assessment_effects(
    assessment: &ghostlight_dungeon::domain::ActionAssessment,
) -> impl Iterator<Item = &ghostlight_dungeon::domain::WorldEffectDelta> {
    [
        &assessment.strong_effect,
        &assessment.success_effect,
        &assessment.mixed_effect,
        &assessment.failure_effect,
    ]
    .into_iter()
}

#[cfg(test)]
fn world_compile_preview_projection(preview: &WorldCompilePreview) -> serde_json::Value {
    let campaign = &preview.campaign;
    let player = &campaign.actors[&campaign.player_actor_id];
    let locations = campaign
        .locations
        .values()
        .map(|location| {
            serde_json::json!({
                "id":location.id,
                "name":location.name,
                "container_id":location.container_id,
                "routes":location.routes,
                "persistent_features":location.persistent_features,
            })
        })
        .collect::<Vec<_>>();
    let cast = campaign
        .actors
        .values()
        .filter(|actor| actor.id != campaign.player_actor_id)
        .map(|actor| {
            serde_json::json!({
                "id":actor.id,
                "name":actor.name,
                "location_id":actor.location_id,
            })
        })
        .collect::<Vec<_>>();
    let institutions = campaign
        .institutions
        .values()
        .map(|institution| {
            serde_json::json!({
                "id":institution.id,
                "name":institution.name,
            })
        })
        .collect::<Vec<_>>();
    let populations = campaign
        .gestalts
        .values()
        .map(|gestalt| {
            serde_json::json!({
                "id":gestalt.id,
                "name":gestalt.name,
                "home_location_id":gestalt.home_location_id,
            })
        })
        .collect::<Vec<_>>();
    let clocks = campaign
        .clocks
        .values()
        .map(|clock| {
            serde_json::json!({
                "id":clock.id,
            "name":clock.label,
                "progress":clock.progress,
                "threshold":clock.threshold,
            "trigger":clock.consequence,
            })
        })
        .collect::<Vec<_>>();
    serde_json::json!({
        "schema":preview.schema,
        "title":preview.title,
        "opening":campaign.transcript.first().map(|turn| turn.text.as_str()),
        "locations":locations,
        "cast":cast,
        "institutions":institutions,
        "populations":populations,
        "clocks":clocks,
        "player_role":{
            "name":player.name,
            "location_id":player.location_id,
            "capabilities":player.capabilities,
            "equipment":player.equipment,
            "conditions":player.conditions,
            "obligations":player.obligations,
        },
        "evidence_coverage":preview.evidence_coverage,
        "gaps":preview.gaps,
        "branch_assumptions":preview.branch_assumptions,
        "requires_approval":preview.requires_approval,
    })
}

fn region_expansion_preview_projection(preview: &RegionExpansionPreview) -> serde_json::Value {
    serde_json::json!({
        "origin_location_id":preview.expansion.origin_location_id,
        "locations":preview.expansion.locations,
        "gaps":preview.gaps,
        "requires_approval":preview.requires_approval,
    })
}

fn gestalt_fission_preview_projection(preview: &GestaltFissionPreview) -> serde_json::Value {
    let children = preview
        .children
        .iter()
        .map(|child| {
            serde_json::json!({
                "id":child.id,
                "name":child.name,
                "home_location_id":child.home_location_id,
                "partition_value":preview.child_partition_values.get(&child.id),
            })
        })
        .collect::<Vec<_>>();
    serde_json::json!({
        "parent_gestalt_id":preview.parent_gestalt_id,
        "partition_axis":preview.partition_axis,
        "children":children,
        "gaps":preview.gaps,
        "requires_approval":preview.requires_approval,
    })
}

fn player_command_projection(
    result: &CommandResult,
    narration: Option<NarrationProjection>,
) -> serde_json::Value {
    match result {
        CommandResult::Assessed { assessment } => serde_json::json!({
            "kind":"assessed",
            "assessment":assessment,
        }),
        CommandResult::Committed { receipt, .. } => serde_json::json!({
            "kind":"committed",
            "revision":receipt.revision,
            "receipt":receipt,
            "narration":narration,
        }),
        CommandResult::ResolutionUpdated { receipt, .. } => serde_json::json!({
            "kind":"resolution_updated",
            "receipt":receipt,
        }),
        CommandResult::Created { .. } => serde_json::json!({
            "kind":"created",
        }),
        CommandResult::GovernancePending { proposal, .. } => serde_json::json!({
            "kind":"governance_pending",
            "proposal":proposal,
        }),
        CommandResult::TravelGovernancePending { proposal, .. } => serde_json::json!({
            "kind":"travel_governance_pending",
            "proposal":proposal,
        }),
        CommandResult::ResolutionGovernancePending { proposal, .. } => serde_json::json!({
            "kind":"resolution_governance_pending",
            "proposal":proposal,
        }),
    }
}

fn player_http_command_allowed(command: &WorldCommand, player_actor_id: &str) -> bool {
    match command {
        WorldCommand::Speak { actor_id, .. } => actor_id == player_actor_id,
        WorldCommand::Assess {
            intent, proposal, ..
        } => intent.actor_id == player_actor_id && proposal.is_none(),
        WorldCommand::Attempt { actor_id, .. } if actor_id == player_actor_id => true,
        WorldCommand::Attempt { .. } => false,
        WorldCommand::Wait { .. } | WorldCommand::SetResolutionBudget { .. } => true,
        WorldCommand::CreateCampaign { .. }
        | WorldCommand::ProposeTimeAdvance { .. }
        | WorldCommand::ApproveTimeAdvance { .. }
        | WorldCommand::ProposeGroupTravel { .. }
        | WorldCommand::ApproveGroupTravel { .. }
        | WorldCommand::ProposeResolutionBudget { .. }
        | WorldCommand::ApproveResolutionBudget { .. }
        | WorldCommand::AdvanceStrategicTick { .. }
        | WorldCommand::ExpandRegion { .. }
        | WorldCommand::MaterializeGestaltMember { .. }
        | WorldCommand::DematerializeGestaltMember { .. }
        | WorldCommand::IndividuateGestaltMember { .. }
        | WorldCommand::ReconcileGestaltPresence { .. }
        | WorldCommand::ResolveReactionWave { .. }
        | WorldCommand::ResolveNpcAction { .. }
        | WorldCommand::SetProviderParallelism { .. }
        | WorldCommand::ReplaceResolutionPins { .. }
        | WorldCommand::FissionGestalt { .. } => false,
    }
}

fn validate_player_http_command(command: &WorldCommand) -> Result<(), String> {
    let bounded = |label: &str, value: &str, max: usize| {
        if value.trim().is_empty() || value.chars().count() > max {
            Err(format!("{label} must contain 1 to {max} characters"))
        } else {
            Ok(())
        }
    };
    match command {
        WorldCommand::Speak {
            text,
            intended_effect,
            ..
        } => {
            bounded("speech", text, 4_000)?;
            if intended_effect.is_some() {
                return Err(
                    "speech and uncertain intended effects use separate player commands".into(),
                );
            }
            Ok(())
        }
        WorldCommand::Assess { intent, .. } => {
            bounded("action description", &intent.description, 4_000)?;
            bounded("intended effect", &intent.intended_effect, 1_000)
        }
        WorldCommand::Attempt {
            assessment_digest, ..
        } => bounded("assessment digest", assessment_digest, 160),
        WorldCommand::Wait { minutes, .. } if *minutes == 0 || *minutes > 1_440 => {
            Err("wait duration must be between 1 and 1440 minutes".into())
        }
        WorldCommand::SetResolutionBudget {
            active_cell_budget, ..
        } if !(1..=128).contains(active_cell_budget) => {
            Err("active Persona-cell budget must be between 1 and 128".into())
        }
        _ => Ok(()),
    }
}

fn reaction_stimulus(command: &WorldCommand) -> Option<String> {
    match command {
        WorldCommand::Speak { actor_id, text, .. } => {
            Some(format!("{actor_id} says: {}", text.trim()))
        }
        _ => None,
    }
}

async fn scheduler_loop(state: AppState) {
    let mut pulse = tokio::time::interval(std::time::Duration::from_secs(300));
    pulse.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    pulse.tick().await;
    loop {
        pulse.tick().await;
        if state.live_turns.load(Ordering::SeqCst) > 0 {
            continue;
        }
        for id in state.registry.list().await {
            match state.registry.runtime(id).await {
                Ok(runtime) => {
                    if let Err(error) = process_due_ticks(
                        &state,
                        &runtime,
                        ghostlight_dungeon::domain::TickSource::Scheduler,
                        true,
                    )
                    .await
                    {
                        tracing::warn!(%id,%error,"strategic scheduler pulse refused");
                    }
                }
                Err(error) => {
                    tracing::warn!(%id,%error,"campaign runtime vanished during scheduler pulse")
                }
            }
        }
        if let Err(error) = refresh_mesh(&state).await {
            tracing::warn!(%error, "CultMesh projection refresh failed");
        }
    }
}

async fn refresh_mesh(state: &AppState) -> anyhow::Result<serde_json::Value> {
    let mut snapshots = Vec::new();
    for id in state.registry.list().await {
        let runtime = state.registry.runtime(id).await?;
        let campaign = load_campaign(&runtime.store)?;
        let mut narrations = runtime
            .store
            .keys("narration_projection.v1")?
            .into_iter()
            .filter_map(|key| {
                runtime
                    .store
                    .load::<NarrationProjection>("narration_projection.v1", &key)
                    .ok()
                    .flatten()
                    .map(|(_, value)| value)
            })
            .filter(|value| value.campaign_id == campaign.id)
            .collect::<Vec<_>>();
        narrations.sort_by_key(|value| value.source_revision);
        snapshots.push(CampaignMeshSnapshot {
            membership: runtime
                .store
                .load::<ghostlight_dungeon::session_zero::CampaignMembership>(
                    "campaign_membership.v1",
                    &campaign.id.to_string(),
                )?
                .map(|(_, membership)| membership),
            campaign,
            narrations,
            evidence: runtime.store.load_all("vault_evidence_receipt.v1")?,
            commits: runtime.store.load_all("world_commit_receipt.v1")?,
            stages: runtime.store.load_all("persona_stage_receipt.v1")?,
            strategic_ticks: runtime.store.load_all("strategic_tick.v1")?,
            gestalt_receipts: runtime
                .store
                .load_all("gestalt_materialization_receipt.v1")?,
            rejected: runtime.store.load_all("rejected_proposal_receipt.v1")?,
            resolution_plans: runtime.store.load_all("resolution_plan_receipt.v1")?,
            cell_appraisals: runtime.store.load_all("cell_appraisal.v1")?,
            activity_outcomes: runtime.store.load_all("strategic_activity_outcome.v1")?,
            resolution_controls: runtime.store.load_all("resolution_control_receipt.v1")?,
        });
    }
    let mut session_zero_snapshots = Vec::new();
    for id in state.session_zeros.list().await {
        let session_zero = state.session_zeros.snapshot(id).await?;
        for member in session_zero.members.values().filter(|member| member.active) {
            session_zero_snapshots.push(SessionZeroMeshSnapshot {
                session_zero_id: id,
                member_id: member.id.clone(),
                surface: session_zero_surface(&session_zero, &member.account_hash)?,
            });
        }
    }
    let publisher = state.mesh.clone();
    let model_status = state.model_status.clone();
    let pressure = state.live_turns.load(Ordering::SeqCst);
    tokio::task::spawn_blocking(move || {
        publisher.publish_snapshot(&snapshots, &session_zero_snapshots, &model_status, pressure)
    })
    .await?
}

fn schedule_mesh_refresh(state: &AppState) {
    let state = state.clone();
    tokio::spawn(async move {
        if let Err(error) = refresh_mesh(&state).await {
            tracing::warn!(%error, "asynchronous CultMesh refresh failed");
        }
    });
}

async fn select_campaign_route(
    Path(campaign_id): Path<uuid::Uuid>,
    headers: HeaderMap,
    State(state): State<AppState>,
) -> Response {
    let account_hash = match authenticated_session(&headers, &state).await {
        Some(value) => value,
        None => return StatusCode::UNAUTHORIZED.into_response(),
    };
    let owned = match state.registry.runtime(campaign_id).await {
        Ok(runtime) => load_campaign(&runtime.store)
            .and_then(|campaign| {
                campaign_member_for_account(&runtime.store, &campaign, &account_hash)
            })
            .is_ok(),
        Err(_) => false,
    };
    if !owned {
        return StatusCode::FORBIDDEN.into_response();
    }
    match select_campaign(&state, &account_hash, campaign_id).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(error) => (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()).into_response(),
    }
}

async fn begin_contract_review(headers: HeaderMap, State(state): State<AppState>) -> Response {
    let account_hash = match authenticated_session(&headers, &state).await {
        Some(value) => value,
        None => return StatusCode::UNAUTHORIZED.into_response(),
    };
    if let Ok(Some(id)) = state
        .session_zeros
        .active_contract_review_for_account(&account_hash)
        .await
    {
        return match state
            .session_zeros
            .snapshot(id)
            .await
            .and_then(|snapshot| session_zero_surface(&snapshot, &account_hash))
        {
            Ok(surface) => Json(surface).into_response(),
            Err(error) => (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()).into_response(),
        };
    }
    let runtime = match session_runtime(&state, &account_hash).await {
        Ok(Some(value)) => value,
        Ok(None) => return (StatusCode::NOT_FOUND, "session has no campaign").into_response(),
        Err(error) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()).into_response();
        }
    };
    let campaign = match load_campaign(&runtime.store) {
        Ok(value) => value,
        Err(error) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()).into_response();
        }
    };
    let membership = match runtime
        .store
        .load::<ghostlight_dungeon::session_zero::CampaignMembership>(
            "campaign_membership.v1",
            &campaign.id.to_string(),
        ) {
        Ok(Some((_, value))) if value.member_for_account(&account_hash).is_some() => value,
        Ok(_) => return StatusCode::FORBIDDEN.into_response(),
        Err(error) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()).into_response();
        }
    };
    let contract = match runtime
        .store
        .load::<ghostlight_dungeon::session_zero::CampaignContract>(
            "campaign_contract.v1",
            &campaign.id.to_string(),
        ) {
        Ok(Some((_, value))) => value,
        Ok(None) => {
            return (
                StatusCode::CONFLICT,
                "campaign has no Session Zero contract",
            )
                .into_response();
        }
        Err(error) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()).into_response();
        }
    };
    let dm_persona = match runtime
        .store
        .load_all::<ghostlight_dungeon::session_zero::CampaignDmPersona>("campaign_dm_persona.v1")
    {
        Ok(values) if values.len() == 1 => values.into_iter().next().unwrap(),
        Ok(_) => return (StatusCode::CONFLICT, "campaign DM state is ambiguous").into_response(),
        Err(error) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()).into_response();
        }
    };
    let previous_brief = runtime
        .store
        .load::<ghostlight_dungeon::session_zero::ApprovedCampaignBrief>(
            "approved_campaign_brief.v1",
            &campaign.id.to_string(),
        )
        .ok()
        .flatten()
        .map(|(_, value)| value);
    let boundaries = runtime
        .store
        .load::<ghostlight_dungeon::session_zero::PublishedSessionZeroSeed>(
            "session_zero_publication.v1",
            &campaign.id.to_string(),
        )
        .ok()
        .flatten()
        .map(|(_, value)| value.boundaries)
        .unwrap_or_default();
    let review = match SessionZeroState::for_contract_review(
        &campaign,
        &membership,
        contract,
        dm_persona,
        previous_brief.as_ref(),
        boundaries,
    ) {
        Ok(value) => value,
        Err(error) => return (StatusCode::CONFLICT, error.to_string()).into_response(),
    };
    let id = review.id;
    match state.session_zeros.create(review).await {
        Ok(_) => match state
            .session_zeros
            .snapshot(id)
            .await
            .and_then(|snapshot| session_zero_surface(&snapshot, &account_hash))
        {
            Ok(surface) => {
                schedule_mesh_refresh(&state);
                (StatusCode::CREATED, Json(surface)).into_response()
            }
            Err(error) => (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()).into_response(),
        },
        Err(error) => (StatusCode::CONFLICT, error.to_string()).into_response(),
    }
}

async fn process_due_ticks(
    state: &AppState,
    runtime: &CampaignRuntime,
    source: ghostlight_dungeon::domain::TickSource,
    yield_to_live_turns: bool,
) -> anyhow::Result<()> {
    loop {
        if yield_to_live_turns && state.live_turns.load(Ordering::SeqCst) > 0 {
            return Ok(());
        }
        let campaign = match load_campaign(&runtime.store) {
            Ok(value) => value,
            Err(_) => return Ok(()),
        };
        let target = ghostlight_dungeon::scheduler::due_tick_target(
            chrono::Utc::now(),
            campaign.last_player_activity,
        );
        if campaign.away_ticks_processed >= target {
            return Ok(());
        }
        let has_simulatable_agency = campaign
            .agency_profiles
            .values()
            .any(|profile| profile.active_leaf && profile.simulation_eligible);
        if !has_simulatable_agency {
            if yield_to_live_turns && state.live_turns.load(Ordering::SeqCst) > 0 {
                return Ok(());
            }
            let _background_commit = if yield_to_live_turns {
                match state.live_commit_gate.clone().try_write_owned() {
                    Ok(guard) => Some(guard),
                    Err(_) => return Ok(()),
                }
            } else {
                None
            };
            if yield_to_live_turns && state.live_turns.load(Ordering::SeqCst) > 0 {
                return Ok(());
            }
            runtime
                .kernel
                .command(WorldCommand::AdvanceStrategicTick {
                    expected_revision: campaign.revision,
                    source: source.clone(),
                    plan: None,
                    model_receipt_hash: None,
                    resolution_wave: None,
                })
                .await?;
            continue;
        }
        let Some(model) = &state.model else {
            return Ok(());
        };
        let permit = Arc::new(SnapshotPermit::new_resolution(
            runtime.store.clone(),
            campaign.id,
            campaign.revision,
            campaign.resolution_policy.resolution_epoch,
        ));
        let (campaign_contract, aggregate_boundaries) =
            campaign_model_policy(&runtime.store, campaign.id);
        let Some(output) = await_background_work(
            state,
            yield_to_live_turns,
            ghostlight_dungeon::scheduler::propose_resolution_wave_with_policy(
                model.clone(),
                permit,
                &campaign,
                campaign_contract.as_ref(),
                &aggregate_boundaries,
            ),
        )
        .await?
        else {
            return Ok(());
        };
        if yield_to_live_turns && state.live_turns.load(Ordering::SeqCst) > 0 {
            return Ok(());
        }
        for stage in &output.stages {
            match runtime
                .store
                .load::<ghostlight_dungeon::model::ModelStageReceipt>(
                    "persona_stage_receipt.v1",
                    stage.receipt.storage_key(),
                )? {
                Some((_, existing)) if existing == stage.receipt => {}
                Some(_) => anyhow::bail!("strategic model receipt hash collision"),
                None => {
                    runtime.store.insert(
                        "persona_stage_receipt.v1",
                        "ghostlight.persona_stage_receipt.v1",
                        stage.receipt.storage_key(),
                        &stage.receipt,
                    )?;
                }
            }
        }
        if yield_to_live_turns && state.live_turns.load(Ordering::SeqCst) > 0 {
            return Ok(());
        }
        let _background_commit = if yield_to_live_turns {
            match state.live_commit_gate.clone().try_write_owned() {
                Ok(guard) => Some(guard),
                Err(_) => return Ok(()),
            }
        } else {
            None
        };
        if yield_to_live_turns && state.live_turns.load(Ordering::SeqCst) > 0 {
            return Ok(());
        }
        runtime
            .kernel
            .command(WorldCommand::AdvanceStrategicTick {
                expected_revision: campaign.revision,
                source: source.clone(),
                plan: None,
                model_receipt_hash: Some(output.aggregate_receipt_hash),
                resolution_wave: Some(output.wave),
            })
            .await?;
    }
}

async fn await_background_work<T>(
    state: &AppState,
    yield_to_live_turns: bool,
    work: impl std::future::Future<Output = anyhow::Result<T>>,
) -> anyhow::Result<Option<T>> {
    if !yield_to_live_turns {
        return work.await.map(Some);
    }
    let live_started = state.live_turn_started.notified();
    tokio::pin!(live_started);
    live_started.as_mut().enable();
    if state.live_turns.load(Ordering::SeqCst) > 0 {
        return Ok(None);
    }
    tokio::pin!(work);
    tokio::select! {
        biased;
        _ = &mut live_started => Ok(None),
        result = &mut work => result.map(Some),
    }
}

async fn app_session_refresh_loop(state: AppState) {
    let mut pulse = tokio::time::interval(std::time::Duration::from_secs(60));
    pulse.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    loop {
        pulse.tick().await;
        if let Err(error) = refresh_due_app_sessions(&state).await {
            tracing::warn!(%error, "scheduled Heimdall session refresh pass failed; unexpired local claims remain authoritative");
        }
    }
}

async fn refresh_due_app_sessions(state: &AppState) -> anyhow::Result<()> {
    let candidates = state
        .auth
        .lock()
        .await
        .sessions_due_for_refresh(Utc::now(), chrono::Duration::minutes(5))?;
    for candidate in candidates {
        let idempotency_key = refresh_idempotency_key(
            &candidate.heimdall_session_id,
            candidate.access_revision,
            &candidate.refresh_claim,
        );
        let completion = match state
            .heimdall
            .refresh(&candidate.refresh_claim, &idempotency_key)
            .await
        {
            Ok(value) => value,
            Err(error) => {
                tracing::warn!(%error, session_id=%candidate.heimdall_session_id, "Heimdall refresh transport unavailable");
                continue;
            }
        };
        if completion.status == "denied" {
            state
                .auth
                .lock()
                .await
                .revoke_cookie_hash(&candidate.cookie_hash)?;
            continue;
        }
        let claims = match state.heimdall.verify_completion(&completion).await {
            Ok(value) => value,
            Err(error) => {
                tracing::warn!(%error, session_id=%candidate.heimdall_session_id, "Heimdall refresh receipt failed verification");
                continue;
            }
        };
        if claims.sid != candidate.heimdall_session_id
            || secret_hash(&format!("heimdall-account:{}", claims.account_id))
                != candidate.account_subject_hash
        {
            tracing::warn!(session_id=%candidate.heimdall_session_id, "Heimdall refresh changed local session custody");
            continue;
        }
        let session = completion
            .session
            .as_ref()
            .context("Heimdall refresh omitted session summary")?;
        let refresh = completion
            .refresh
            .as_ref()
            .context("Heimdall refresh omitted refresh expiry")?;
        let refresh_claim = completion
            .refresh_token
            .as_deref()
            .context("Heimdall refresh omitted rotated refresh claim")?;
        state.auth.lock().await.apply_refresh(
            &candidate.cookie_hash,
            RefreshedSession {
                expected_access_revision: candidate.access_revision,
                access_revision: claims.access_revision,
                capabilities: claims.capabilities,
                access_expires_at: DateTime::from_timestamp(claims.exp as i64, 0)
                    .context("Heimdall refresh expiry is invalid")?,
                refresh_expires_at: refresh.expires_at.parse()?,
                refresh_claim,
            },
        )?;
        debug_assert_eq!(session.session_id, candidate.heimdall_session_id);
    }
    Ok(())
}

fn refresh_idempotency_key(
    heimdall_session_id: &str,
    access_revision: u64,
    refresh_claim: &str,
) -> String {
    let generation = secret_hash(&format!("heimdall-refresh-generation:{refresh_claim}"));
    format!("refresh:{heimdall_session_id}:{access_revision}:{generation}")
}

async fn authenticated_session(headers: &HeaderMap, state: &AppState) -> Option<String> {
    let raw_cookie = cookie_value(headers)?;
    state
        .auth
        .lock()
        .await
        .account_for_cookie(raw_cookie, Utc::now())
}

fn cookie_value(headers: &HeaderMap) -> Option<&str> {
    headers
        .get(header::COOKIE)
        .and_then(|v| v.to_str().ok())
        .and_then(|cookies| {
            cookies
                .split(';')
                .map(str::trim)
                .find_map(|cookie| cookie.strip_prefix("ghostlight_session="))
        })
}

async fn session_runtime(
    state: &AppState,
    account_hash: &str,
) -> anyhow::Result<Option<CampaignRuntime>> {
    let campaign_id = state.auth.lock().await.selected_campaign(account_hash);
    match campaign_id {
        Some(id) => {
            let runtime = state.registry.runtime(id).await?;
            let campaign = load_campaign(&runtime.store)?;
            campaign_member_for_account(&runtime.store, &campaign, account_hash)?;
            Ok(Some(runtime))
        }
        None => Ok(None),
    }
}

async fn select_campaign(
    state: &AppState,
    account_hash: &str,
    campaign_id: uuid::Uuid,
) -> anyhow::Result<()> {
    let runtime = state.registry.runtime(campaign_id).await?;
    let campaign = load_campaign(&runtime.store)?;
    campaign_member_for_account(&runtime.store, &campaign, account_hash)?;
    state
        .auth
        .lock()
        .await
        .select_campaign(account_hash, campaign_id)
}

fn load_campaign(store: &CampaignStore) -> anyhow::Result<Campaign> {
    let id = store
        .keys("campaign.v1")?
        .into_iter()
        .next()
        .ok_or_else(|| anyhow::anyhow!("campaign missing"))?;
    store
        .load("campaign.v1", &id)?
        .map(|(_, c)| c)
        .ok_or_else(|| anyhow::anyhow!("campaign missing"))
}

fn campaign_model_policy(
    store: &CampaignStore,
    campaign_id: uuid::Uuid,
) -> (
    Option<ghostlight_dungeon::session_zero::CampaignContract>,
    Vec<ghostlight_dungeon::session_zero::AggregatedBoundary>,
) {
    let publication = store
        .load::<ghostlight_dungeon::session_zero::PublishedSessionZeroSeed>(
            "session_zero_publication.v1",
            &campaign_id.to_string(),
        )
        .ok()
        .flatten()
        .map(|(_, publication)| publication);
    let Some(publication) = publication else {
        return (None, vec![]);
    };
    let mut boundaries = publication.approved_brief.aggregate_boundaries.clone();
    if let Some((_, active)) = store
        .load::<ghostlight_dungeon::session_zero::ActiveContractBoundaryPolicy>(
            "active_contract_boundary_policy.v1",
            &campaign_id.to_string(),
        )
        .ok()
        .flatten()
        && active.review_session_zero_id != publication.session_zero_id
    {
        boundaries = active.aggregate_boundaries;
    }
    (Some(publication.contract), boundaries)
}

async fn migrate_legacy_campaign_memberships(
    registry: &CampaignRegistry,
    auth: &LegacyAuthState,
) -> anyhow::Result<()> {
    let mut owners = BTreeMap::<uuid::Uuid, BTreeSet<String>>::new();
    for (account_hash, campaign_id) in &auth.session_campaigns {
        owners
            .entry(*campaign_id)
            .or_default()
            .insert(account_hash.clone());
    }
    for (account_hash, campaign_ids) in &auth.session_campaign_ids {
        for campaign_id in campaign_ids {
            owners
                .entry(*campaign_id)
                .or_default()
                .insert(account_hash.clone());
        }
    }
    for campaign_id in registry.list().await {
        let runtime = registry.runtime(campaign_id).await?;
        if runtime
            .store
            .load::<ghostlight_dungeon::session_zero::CampaignMembership>(
                "campaign_membership.v1",
                &campaign_id.to_string(),
            )?
            .is_some()
        {
            continue;
        }
        let campaign = load_campaign(&runtime.store)?;
        let account_hashes = owners
            .get(&campaign_id)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .collect::<Vec<_>>();
        let receipt = GovernanceMigrationReceipt {
            schema: "ghostlight.governance_migration_receipt.v1".into(),
            campaign_id,
            status: if account_hashes.len() == 1 {
                "migrated".into()
            } else {
                "quarantined_ambiguous_owner".into()
            },
            account_hashes: account_hashes.clone(),
            actor_id: campaign.player_actor_id.clone(),
            created_at: chrono::Utc::now(),
        };
        if account_hashes.len() == 1 {
            let member_id = format!("member:{}", uuid::Uuid::new_v4().simple());
            let membership = ghostlight_dungeon::session_zero::CampaignMembership {
                schema: "ghostlight.campaign_membership.v1".into(),
                campaign_id,
                governance_epoch: 0,
                host_member_id: member_id.clone(),
                members: BTreeMap::from([(
                    member_id.clone(),
                    ghostlight_dungeon::session_zero::CampaignMember {
                        member_id,
                        account_hash: account_hashes[0].clone(),
                        display_name: "Player".into(),
                        actor_id: campaign.player_actor_id.clone(),
                        is_host: true,
                        active: true,
                        cell_allowance: 8,
                    },
                )]),
                extraordinary_permissions: BTreeMap::new(),
            };
            runtime.store.insert(
                "campaign_membership.v1",
                "ghostlight.campaign_membership.v1",
                &campaign_id.to_string(),
                &membership,
            )?;
            runtime.store.insert(
                "campaign_governance.v1",
                "ghostlight.campaign_governance.v1",
                &campaign_id.to_string(),
                &ghostlight_dungeon::session_zero::CampaignGovernance {
                    schema: "ghostlight.campaign_governance.v1".into(),
                    campaign_id,
                    governance_epoch: 0,
                    time_advance_policy: "unanimous".into(),
                    pooled_cell_ceiling: 8,
                    cooperative_shared_scene_only: true,
                    pvp_enabled: false,
                },
            )?;
        }
        let _ = runtime.store.insert(
            "governance_migration_receipt.v1",
            "ghostlight.governance_migration_receipt.v1",
            &campaign_id.to_string(),
            &receipt,
        );
    }
    Ok(())
}

fn campaign_member_for_account(
    store: &CampaignStore,
    campaign: &Campaign,
    account_hash: &str,
) -> anyhow::Result<ghostlight_dungeon::session_zero::CampaignMember> {
    if let Some((_, membership)) = store
        .load::<ghostlight_dungeon::session_zero::CampaignMembership>(
            "campaign_membership.v1",
            &campaign.id.to_string(),
        )?
    {
        return membership
            .member_for_account(account_hash)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("account is not a campaign member"));
    }
    Err(anyhow::anyhow!(
        "campaign membership is missing or quarantined for operator review"
    ))
}

fn migrate_default_campaign(runtime_root: &std::path::Path) -> anyhow::Result<()> {
    let legacy_directory = runtime_root.join("campaigns/default");
    let legacy_store_path = legacy_directory.join("campaign.cc");
    if !legacy_store_path.is_file() {
        std::fs::create_dir_all(runtime_root.join("campaigns"))?;
        return Ok(());
    }
    let keys = CampaignStore::open(&legacy_store_path)?.keys("campaign.v1")?;
    if keys.is_empty() {
        return Ok(());
    }
    if keys.len() != 1 {
        return Err(anyhow::anyhow!(
            "legacy default store contains multiple campaigns"
        ));
    }
    let id = uuid::Uuid::parse_str(&keys[0])?;
    let target = runtime_root.join("campaigns").join(id.to_string());
    if target.exists() {
        return Err(anyhow::anyhow!(
            "legacy campaign migration target already exists"
        ));
    }
    std::fs::rename(legacy_directory, target)?;
    Ok(())
}

#[derive(Serialize)]
struct ErrorBody {
    error: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{body::Body, http::Request};
    use ghostlight_dungeon::domain::{ActorState, BranchOrigin, Location};
    use tower::ServiceExt;

    fn seed(name: &str) -> Campaign {
        let actor = ActorState {
            id: "player".into(),
            name: "Player".into(),
            location_id: "room".into(),
            capabilities: BTreeSet::new(),
            knowledge: BTreeSet::new(),
            equipment: BTreeSet::new(),
            conditions: BTreeSet::new(),
            obligations: BTreeSet::new(),
            relationships: BTreeMap::new(),
            goals: vec![],
            memories: vec![],
        };
        Campaign {
            schema: "ghostlight.campaign.v1".into(),
            id: uuid::Uuid::new_v4(),
            name: name.into(),
            revision: 0,
            branch_origin: BranchOrigin {
                canon_cutoff: "test".into(),
                evidence_receipt_ids: vec![],
            },
            world_time: chrono::Utc::now(),
            tick_hours: 6,
            player_actor_id: "player".into(),
            locations: BTreeMap::from([(
                "room".into(),
                Location {
                    id: "room".into(),
                    name: "Room".into(),
                    container_id: None,
                    routes: BTreeMap::new(),
                    persistent_features: vec![],
                },
            )]),
            actors: BTreeMap::from([("player".into(), actor)]),
            institutions: BTreeMap::new(),
            clocks: BTreeMap::new(),
            facts: BTreeMap::new(),
            transcript: vec![],
            last_player_activity: chrono::Utc::now(),
            pending_ticks: 0,
            away_ticks_processed: 0,
            events: vec![],
            news: vec![],
            canon_candidates: BTreeMap::new(),
            gestalts: BTreeMap::new(),
            gestalt_members: BTreeMap::new(),
            pending_world_proposals: vec![],
            agency_profiles: BTreeMap::new(),
            agency_relations: BTreeMap::new(),
            gestalt_lineages: BTreeMap::new(),
            resolution_policy: Default::default(),
            resolution_pins: BTreeMap::new(),
            resolution_cover: None,
            strategic_tick_count: 0,
        }
    }

    fn empty_app_state(root: &std::path::Path) -> AppState {
        let registry = CampaignRegistry::new(root.join("campaigns")).unwrap();
        let wrapping_key = root.join("session-wrapping.key");
        std::fs::write(&wrapping_key, [7_u8; 32]).unwrap();
        let auth = AppSessionOwner::open(root.join("app-sessions.cc"), &wrapping_key).unwrap();
        AppState {
            registry,
            session_zeros: SessionZeroRegistry::new(root.join("session-zero")).unwrap(),
            session_zero_director: None,
            entitlements: Arc::new(FixtureEntitlementPort),
            auth: Arc::new(Mutex::new(auth)),
            heimdall: Arc::new(HeimdallClient::fixture()),
            model_status: ModelRuntimeStatus {
                provider: "fixture".into(),
                fast_model: "fixture".into(),
                capable_model: "fixture".into(),
                readiness: "ready".into(),
            },
            compiler: None,
            assessor: None,
            model: None,
            expansion_previews: Arc::new(Mutex::new(BTreeMap::new())),
            fission_previews: Arc::new(Mutex::new(BTreeMap::new())),
            live_turns: Arc::new(AtomicUsize::new(0)),
            live_turn_started: Arc::new(Notify::new()),
            live_commit_gate: Arc::new(RwLock::new(())),
            mesh: MeshPublisher::open(root.join("mesh.cc"), None).unwrap(),
        }
    }

    async fn fixture_session(state: &AppState, account_id: &str) -> (String, String) {
        let cookie = state
            .auth
            .lock()
            .await
            .create_session(NewSession {
                account_id,
                heimdall_session_id: "fixture-heimdall-session",
                access_revision: 1,
                capabilities: vec!["app_access".into()],
                access_expires_at: Utc::now() + chrono::Duration::hours(1),
                refresh_expires_at: Utc::now() + chrono::Duration::days(7),
                refresh_claim: "fixture-refresh",
            })
            .unwrap();
        let account_hash = state
            .auth
            .lock()
            .await
            .account_for_cookie(&cookie, Utc::now())
            .unwrap();
        (cookie, account_hash)
    }

    #[tokio::test]
    async fn public_router_exposes_only_the_eve_product_boundary() {
        let dir = tempfile::tempdir().unwrap();
        let state = empty_app_state(dir.path());
        let app = app_router(state, dir.path().join("web"));
        for retired in [
            "/api/command",
            "/api/session-zero",
            "/api/auth/heimdall/start",
        ] {
            let response = app
                .clone()
                .oneshot(Request::builder().uri(retired).body(Body::empty()).unwrap())
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::NOT_FOUND);
        }
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/eve/surfaces/ghostlight.play")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn local_session_cookie_resolves_only_to_heimdall_account_subject_hash() {
        let dir = tempfile::tempdir().unwrap();
        let state = empty_app_state(dir.path());
        let (cookie, account_hash) = fixture_session(&state, "acct-1").await;
        let mut headers = HeaderMap::new();
        headers.insert(
            header::COOKIE,
            HeaderValue::from_str(&format!("ghostlight_session={cookie}")).unwrap(),
        );
        assert_eq!(
            authenticated_session(&headers, &state).await,
            Some(account_hash)
        );
    }

    #[test]
    fn refresh_idempotency_tracks_the_rotating_claim_generation() {
        let first = refresh_idempotency_key("heimdall-session", 7, "refresh-claim-one");
        let retry = refresh_idempotency_key("heimdall-session", 7, "refresh-claim-one");
        let rotated = refresh_idempotency_key("heimdall-session", 7, "refresh-claim-two");

        assert_eq!(first, retry);
        assert_ne!(first, rotated);
        assert!(!first.contains("refresh-claim-one"));
    }

    #[test]
    fn app_session_cookie_survives_browser_relaunch_until_refresh_expiry() {
        let now = Utc::now();
        let header = app_session_cookie("opaque-cookie", now + chrono::Duration::days(7), now);
        let value = header.to_str().unwrap();

        assert!(value.contains("Max-Age=604800"));
        assert!(value.contains("HttpOnly"));
        assert!(value.contains("Secure"));
        assert!(value.contains("SameSite=Lax"));
        assert!(value.contains("Path=/ghostlight/"));
    }

    #[tokio::test]
    async fn live_turn_cancels_background_inference_and_excludes_its_commit_gate() {
        let dir = tempfile::tempdir().unwrap();
        let state = empty_app_state(dir.path());
        let trigger_state = state.clone();
        let (entered_tx, entered_rx) = tokio::sync::oneshot::channel();
        let (release_tx, release_rx) = tokio::sync::oneshot::channel();
        let trigger = tokio::spawn(async move {
            let guard = LiveTurnGuard::enter(&trigger_state).await;
            let _ = entered_tx.send(());
            let _ = release_rx.await;
            drop(guard);
        });

        let result = tokio::time::timeout(
            std::time::Duration::from_millis(100),
            await_background_work(&state, true, std::future::pending::<anyhow::Result<()>>()),
        )
        .await
        .expect("background work did not yield")
        .unwrap();
        assert!(result.is_none());
        entered_rx.await.unwrap();
        assert_eq!(state.live_turns.load(Ordering::SeqCst), 1);
        assert!(state.live_commit_gate.clone().try_write_owned().is_err());

        let _ = release_tx.send(());
        trigger.await.unwrap();
        assert_eq!(state.live_turns.load(Ordering::SeqCst), 0);
        assert!(state.live_commit_gate.clone().try_write_owned().is_ok());
    }

    #[tokio::test]
    async fn due_ticks_without_simulatable_agency_advance_deterministically_without_a_model() {
        let dir = tempfile::tempdir().unwrap();
        let state = empty_app_state(dir.path());
        let mut campaign = seed("Quiet world");
        let original_player = campaign.actors["player"].clone();
        let original_time = campaign.world_time;
        campaign.last_player_activity = chrono::Utc::now() - chrono::Duration::hours(2);
        campaign.clocks.insert(
            "tide".into(),
            ghostlight_dungeon::domain::WorldClock {
                id: "tide".into(),
                label: "Tide turns".into(),
                progress: 0,
                threshold: 4,
                consequence: "the channel narrows".into(),
            },
        );
        let runtime = state
            .registry
            .create(campaign.clone(), vec![], vec![])
            .await
            .unwrap();

        process_due_ticks(
            &state,
            &runtime,
            ghostlight_dungeon::domain::TickSource::Scheduler,
            true,
        )
        .await
        .unwrap();

        let advanced = load_campaign(&runtime.store).unwrap();
        assert_eq!(advanced.away_ticks_processed, 2);
        assert_eq!(advanced.strategic_tick_count, 2);
        assert_eq!(advanced.clocks["tide"].progress, 2);
        assert_eq!(
            advanced.world_time,
            original_time + chrono::Duration::hours(12)
        );
        assert_eq!(advanced.actors["player"], original_player);
    }

    #[tokio::test]
    async fn campaign_selection_is_derived_from_exact_membership() {
        let dir = tempfile::tempdir().unwrap();
        let state = empty_app_state(dir.path());
        let (_cookie, account_hash) = fixture_session(&state, "owner").await;
        let campaign = seed("Membership-bound");
        let runtime = state
            .registry
            .create(campaign.clone(), vec![], vec![])
            .await
            .unwrap();
        runtime
            .store
            .insert(
                "campaign_membership.v1",
                "ghostlight.campaign_membership.v1",
                &campaign.id.to_string(),
                &ghostlight_dungeon::session_zero::CampaignMembership {
                    schema: "ghostlight.campaign_membership.v1".into(),
                    campaign_id: campaign.id,
                    governance_epoch: 0,
                    host_member_id: "member:owner".into(),
                    members: BTreeMap::from([(
                        "member:owner".into(),
                        ghostlight_dungeon::session_zero::CampaignMember {
                            member_id: "member:owner".into(),
                            account_hash: account_hash.clone(),
                            display_name: "Owner".into(),
                            actor_id: "player".into(),
                            is_host: true,
                            active: true,
                            cell_allowance: 8,
                        },
                    )]),
                    extraordinary_permissions: BTreeMap::new(),
                },
            )
            .unwrap();
        select_campaign(&state, &account_hash, campaign.id)
            .await
            .unwrap();
        assert_eq!(
            load_campaign(
                &session_runtime(&state, &account_hash)
                    .await
                    .unwrap()
                    .unwrap()
                    .store
            )
            .unwrap()
            .id,
            campaign.id
        );
        assert!(
            select_campaign(&state, "sha256:not-a-member", campaign.id)
                .await
                .is_err()
        );
    }

    #[test]
    fn authority_fields_are_rejected_recursively_at_eve_ingress() {
        assert!(contains_authority_field(
            &serde_json::json!({"bindings":{"actor_id":"npc"}})
        ));
        assert!(contains_authority_field(
            &serde_json::json!({"nested":[{"memberId":"member:other"}]})
        ));
        assert!(!contains_authority_field(
            &serde_json::json!({"target":"member:other","text":"hello"})
        ));
    }

    #[test]
    fn player_http_boundary_cannot_invoke_internal_world_commands_or_npcs() {
        assert!(player_http_command_allowed(
            &WorldCommand::Wait {
                expected_revision: 4,
                minutes: 10,
            },
            "player",
        ));
        assert!(player_http_command_allowed(
            &WorldCommand::Speak {
                expected_revision: 4,
                actor_id: "player".into(),
                text: "Hello.".into(),
                intended_effect: None,
            },
            "player",
        ));
        assert!(!player_http_command_allowed(
            &WorldCommand::Speak {
                expected_revision: 4,
                actor_id: "npc".into(),
                text: "I have been puppeted.".into(),
                intended_effect: None,
            },
            "player",
        ));
        assert!(!player_http_command_allowed(
            &WorldCommand::AdvanceStrategicTick {
                expected_revision: 4,
                source: ghostlight_dungeon::domain::TickSource::Scheduler,
                plan: None,
                model_receipt_hash: None,
                resolution_wave: None,
            },
            "player",
        ));
        assert!(!player_http_command_allowed(
            &WorldCommand::ReconcileGestaltPresence {
                expected_revision: 4,
                reason: "browser says so".into(),
                plan: Default::default(),
            },
            "player",
        ));
        assert!(player_http_command_allowed(
            &WorldCommand::SetResolutionBudget {
                expected_revision: 4,
                expected_resolution_epoch: 2,
                active_cell_budget: 8,
            },
            "player",
        ));
        assert!(!player_http_command_allowed(
            &WorldCommand::ReplaceResolutionPins {
                expected_revision: 4,
                expected_resolution_epoch: 2,
                pins: vec![],
            },
            "player",
        ));
    }

    #[test]
    fn player_http_command_admission_bounds_cost_and_fictional_time() {
        assert!(
            validate_player_http_command(&WorldCommand::Wait {
                expected_revision: 0,
                minutes: 0,
            })
            .is_err()
        );
        assert!(
            validate_player_http_command(&WorldCommand::Wait {
                expected_revision: 0,
                minutes: 1_441,
            })
            .is_err()
        );
        assert!(
            validate_player_http_command(&WorldCommand::SetResolutionBudget {
                expected_revision: 0,
                expected_resolution_epoch: 0,
                active_cell_budget: 129,
            })
            .is_err()
        );
        assert!(
            validate_player_http_command(&WorldCommand::Speak {
                expected_revision: 0,
                actor_id: "player".into(),
                text: "Hello".into(),
                intended_effect: Some("obey me".into()),
            })
            .is_err()
        );
        assert!(
            validate_player_http_command(&WorldCommand::Assess {
                expected_revision: 0,
                intent: ActionIntent {
                    actor_id: "player".into(),
                    description: "x".repeat(4_001),
                    intended_effect: "cross the room".into(),
                },
                proposal: None,
            })
            .is_err()
        );
    }

    #[test]
    fn npc_reactions_receive_player_speech_not_private_effect_scaffolding() {
        let command = WorldCommand::Speak {
            expected_revision: 4,
            actor_id: "player".into(),
            text: "Which record can I inspect without taking custody?".into(),
            intended_effect: Some("make the archivist disclose every secret".into()),
        };
        let stimulus = reaction_stimulus(&command).unwrap();
        assert_eq!(
            stimulus,
            "player says: Which record can I inspect without taking custody?"
        );
        assert!(!stimulus.contains("disclose every secret"));
        assert!(!stimulus.contains("requires assessment"));
    }

    #[test]
    fn player_command_projection_never_serializes_campaign_state() {
        let campaign = seed("Private state");
        let result = CommandResult::Committed {
            receipt: ghostlight_dungeon::domain::WorldCommitReceipt {
                schema: "ghostlight.world_commit_receipt.v1".into(),
                campaign_id: campaign.id,
                previous_revision: 0,
                revision: 1,
                command_kind: "attempt".into(),
                committed_at: chrono::Utc::now(),
                roll: None,
            },
            campaign,
        };
        let projection = player_command_projection(&result, None);
        assert_eq!(projection["kind"], "committed");
        assert_eq!(projection["revision"], 1);
        let encoded = serde_json::to_string(&projection).unwrap();
        assert!(!encoded.contains("Private state"));
        assert!(!encoded.contains("actors"));
        assert!(!encoded.contains("facts"));

        let created = player_command_projection(
            &CommandResult::Created {
                campaign: seed("Hidden seed"),
            },
            None,
        );
        assert_eq!(created, serde_json::json!({"kind":"created"}));
    }

    #[test]
    fn compiler_preview_projection_exposes_approval_shape_not_private_state() {
        let mut campaign = seed("Approval preview");
        let player = campaign.actors.get_mut("player").unwrap();
        player.goals = vec!["private goal".into()];
        player.memories = vec!["private memory".into()];
        player
            .relationships
            .insert("hidden".into(), "distrust".into());
        campaign.facts.insert(
            "branch-secret".into(),
            ghostlight_dungeon::domain::WorldFact {
                id: "branch-secret".into(),
                statement: "The apparent ally caused the disaster.".into(),
                scope: ghostlight_dungeon::domain::FactScope::BranchLocal,
                evidence_receipt_ids: vec![],
                discoverable_at_location_ids: BTreeSet::from(["room".into()]),
            },
        );
        let preview = WorldCompilePreview {
            schema: "ghostlight.world_compile_preview.v1".into(),
            title: "Approval preview".into(),
            campaign,
            evidence_receipts: vec![],
            evidence_coverage: vec![],
            gaps: vec!["a visible gap".into()],
            branch_assumptions: vec!["a visible assumption".into()],
            requires_approval: true,
        };

        let projection = world_compile_preview_projection(&preview);
        let encoded = serde_json::to_string(&projection).unwrap();
        assert!(encoded.contains("player_role"));
        assert!(encoded.contains("locations"));
        for private_key in [
            "\"campaign\":",
            "\"evidence_receipts\":",
            "\"goals\":",
            "\"memories\":",
            "\"relationships\":",
            "\"model_receipts\":",
            "\"branch_facts\":",
            "branch-secret",
            "The apparent ally caused the disaster.",
        ] {
            assert!(!encoded.contains(private_key), "leaked {private_key}");
        }
    }
}
