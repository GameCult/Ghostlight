use axum::{
    Json, Router,
    extract::{ConnectInfo, Path, Request, State},
    http::{HeaderMap, HeaderValue, StatusCode, header},
    middleware,
    middleware::Next,
    response::{
        IntoResponse, Response,
        sse::{Event as SseEvent, KeepAlive, Sse},
    },
    routing::{get, post},
};
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
    kernel::{CommandResult, KernelError},
    mesh::{CampaignMeshSnapshot, MeshPublisher, SessionZeroMeshSnapshot},
    model::{DeepSeekPort, ModelPort, ModelStageRequest, run_validated_stage},
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
use sha2::{Digest, Sha256};
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

mod heimdall;
use heimdall::{BackendCallback, HeimdallClient};

#[derive(Clone)]
struct AppState {
    registry: CampaignRegistry,
    session_zeros: SessionZeroRegistry,
    session_zero_director: Option<Arc<SessionZeroDirector>>,
    entitlements: Arc<dyn EntitlementPort>,
    runtime_root: PathBuf,
    auth: Arc<Mutex<AuthOwner>>,
    heimdall: Arc<HeimdallClient>,
    deepseek_status: String,
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
struct ProviderParallelismRequest {
    expected_provider_configuration_epoch: u64,
    provider_parallelism: u8,
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
struct AuthState {
    schema: String,
    session_hashes: BTreeSet<String>,
    #[serde(default)]
    session_aliases: BTreeMap<String, String>,
    #[serde(default)]
    session_campaigns: BTreeMap<String, uuid::Uuid>,
    #[serde(default)]
    session_campaign_ids: BTreeMap<String, BTreeSet<uuid::Uuid>>,
    #[serde(default)]
    heimdall_attempts: BTreeMap<String, HeimdallAuthAttempt>,
}

#[derive(Clone, Serialize, Deserialize)]
struct HeimdallAuthAttempt {
    expires_at_unix: u64,
    status: String,
    account_session_hash: Option<String>,
    error: Option<String>,
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

struct AuthOwner {
    store: CampaignStore,
    row: cultcache_legacy::CultCacheEnvelope,
    state: AuthState,
}

impl AuthOwner {
    fn commit(&mut self, next_state: AuthState) -> anyhow::Result<()> {
        let next_row = self
            .store
            .replace(&self.row, "ghostlight.auth_state.v1", &next_state)?;
        self.row = next_row;
        self.state = next_state;
        Ok(())
    }

    fn reload(&mut self) -> anyhow::Result<()> {
        let (row, state) = self
            .store
            .load::<AuthState>("auth_state.v1", "primary")?
            .ok_or_else(|| anyhow::anyhow!("canonical auth state is missing"))?;
        self.row = row;
        self.state = state;
        Ok(())
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();
    let runtime_root = std::env::var_os("GHOSTLIGHT_DUNGEON_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(r"F:\GameCult\GhostlightDungeon"));
    migrate_default_campaign(&runtime_root)?;
    let registry = CampaignRegistry::new(runtime_root.join("campaigns"))?;
    registry.load_existing().await?;
    let session_zeros = SessionZeroRegistry::new(runtime_root.join("session-zero"))?;
    session_zeros.load_existing().await?;
    std::fs::create_dir_all(runtime_root.join("service"))?;
    let auth_store = CampaignStore::open(runtime_root.join("service/auth.cc"))?;
    let (auth_row, auth_state) = match auth_store.load::<AuthState>("auth_state.v1", "primary")? {
        Some((row, state)) => (row, state),
        None => {
            let state = AuthState {
                schema: "ghostlight.auth_state.v1".into(),
                session_hashes: BTreeSet::new(),
                session_aliases: BTreeMap::new(),
                session_campaigns: BTreeMap::new(),
                session_campaign_ids: BTreeMap::new(),
                heimdall_attempts: BTreeMap::new(),
            };
            let row = auth_store.insert(
                "auth_state.v1",
                "ghostlight.auth_state.v1",
                "primary",
                &state,
            )?;
            (row, state)
        }
    };
    migrate_legacy_campaign_memberships(&registry, &auth_state).await?;
    let secret_path = runtime_root.join("secrets/deepseek.dpapi");
    let (deepseek_status, compiler, assessor, shared_model) = if secret_path.is_file() {
        let provider: Arc<dyn ModelPort> =
            Arc::new(DeepSeekPort::from_machine_dpapi(&secret_path)?);
        let probe = run_validated_stage(
            provider.as_ref(),
            &ModelStageRequest {
                stage: "startup_probe".into(),
                model: "deepseek-v4-flash".into(),
                snapshot_binding: "service-startup".into(),
                lived_stream: "Reply with the single word ready.".into(),
                output_schema: None,
                source_receipt_ids: vec![],
                temperature: Some(0.0),
                max_output_tokens: Some(16),
            },
        )
        .await?;
        (
            format!("ready:{}", probe.receipt.output_hash),
            Some(Arc::new(WorldCompiler::new(
                Arc::new(VoidBotMcpVault::starfire_loopback()),
                provider.clone(),
                "deepseek-v4-flash",
                "deepseek-v4-pro",
            ))),
            Some(Arc::new(ActionAssessor::new(
                provider.clone(),
                "deepseek-v4-pro",
            ))),
            Some(provider),
        )
    } else {
        ("missing-secret".into(), None, None, None)
    };
    let mesh_target = std::env::var("GHOSTLIGHT_ODIN_RUDP")
        .ok()
        .map(|value| value.parse())
        .transpose()?;
    let mesh = MeshPublisher::open(runtime_root.join("service/mesh.cc"), mesh_target)?;
    let session_zero_director = shared_model.as_ref().map(|model| {
        Arc::new(SessionZeroDirector::new(
            model.clone(),
            "deepseek-v4-flash",
            "deepseek-v4-pro",
            "deepseek-v4-flash",
        ))
    });
    let state = AppState {
        registry,
        session_zeros,
        session_zero_director,
        entitlements: Arc::new(FixtureEntitlementPort),
        runtime_root,
        auth: Arc::new(Mutex::new(AuthOwner {
            store: auth_store,
            row: auth_row,
            state: auth_state,
        })),
        heimdall: Arc::new(HeimdallClient::from_env()?),
        deepseek_status,
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
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;
    Ok(())
}

fn app_router(state: AppState, web_root: PathBuf) -> Router {
    let protected_api = Router::new()
        .route("/api/surface", get(surface))
        .route("/api/events", get(revision_events))
        .route("/api/session-zero", post(begin_session_zero))
        .route(
            "/api/session-zero/{session_id}/surface",
            get(session_zero_surface_route),
        )
        .route(
            "/api/session-zero/{session_id}/invites",
            post(create_session_zero_invites),
        )
        .route("/api/session-zero/join/{token}", post(join_session_zero))
        .route(
            "/api/session-zero/{session_id}/message",
            post(post_session_zero_message),
        )
        .route(
            "/api/session-zero/{session_id}/boundary",
            post(set_session_zero_boundary),
        )
        .route(
            "/api/session-zero/{session_id}/boundary/{boundary_id}/remove",
            post(remove_session_zero_boundary),
        )
        .route(
            "/api/session-zero/{session_id}/leave",
            post(leave_session_zero),
        )
        .route(
            "/api/session-zero/{session_id}/remove-member",
            post(remove_session_zero_member),
        )
        .route(
            "/api/session-zero/{session_id}/decision",
            post(resolve_session_zero_decision),
        )
        .route(
            "/api/session-zero/{session_id}/lock",
            post(lock_session_zero_roster),
        )
        .route(
            "/api/session-zero/{session_id}/compile",
            post(compile_session_zero),
        )
        .route(
            "/api/session-zero/{session_id}/approve",
            post(approve_session_zero),
        )
        .route(
            "/api/session-zero/{session_id}/publish",
            post(publish_session_zero),
        )
        .route("/api/compiler/destination", post(compile_destination))
        .route(
            "/api/compiler/destination/approve/{preview_id}",
            post(approve_destination),
        )
        .route("/api/compiler/gestalt/fission", post(compile_fission))
        .route(
            "/api/compiler/gestalt/fission/approve/{preview_id}",
            post(approve_fission),
        )
        .route("/api/command", post(command))
        .route("/api/governance/time", post(propose_time_advance))
        .route(
            "/api/governance/time/{proposal_id}/approve",
            post(approve_time_advance),
        )
        .route("/api/governance/travel", post(propose_group_travel))
        .route(
            "/api/governance/travel/{proposal_id}/approve",
            post(approve_group_travel),
        )
        .route("/api/governance/cell-budget", post(propose_cell_budget))
        .route(
            "/api/governance/cell-budget/{proposal_id}/approve",
            post(approve_cell_budget),
        )
        .route("/api/campaigns", get(campaigns))
        .route(
            "/api/campaigns/select/{campaign_id}",
            post(select_campaign_route),
        )
        .route("/api/campaigns/fork", post(fork_campaign))
        .route("/api/campaigns/reset", post(reset_campaign))
        .route("/api/campaigns/export", get(export_campaign))
        .route(
            "/api/campaigns/contract-review",
            post(begin_contract_review),
        )
        .route(
            "/api/campaigns/canon-candidates.md",
            get(export_canon_candidates_markdown),
        )
        .route("/api/operator", get(operator_inspector))
        .route(
            "/api/operator/provider-parallelism",
            post(set_provider_parallelism),
        )
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            require_api_authentication,
        ));

    Router::new()
        .route("/health", get(health))
        .route("/api/auth/heimdall/start", post(heimdall_start))
        .route("/api/auth/heimdall/callback", post(heimdall_callback))
        .route(
            "/api/auth/heimdall/attempt/{attempt_id}",
            get(heimdall_attempt),
        )
        .route(
            "/api/auth/heimdall/attempt/{attempt_id}/adopt",
            post(heimdall_adopt),
        )
        .merge(protected_api)
        .fallback_service(ServeDir::new(web_root).append_index_html_on_directories(true))
        .with_state(state)
}

async fn heimdall_start(State(state): State<AppState>) -> Response {
    let attempt_id = uuid::Uuid::new_v4().simple().to_string();
    let now = unix_time_seconds();
    {
        let mut auth = state.auth.lock().await;
        let mut next_state = auth.state.clone();
        prune_heimdall_attempts(&mut next_state, now);
        next_state.heimdall_attempts.insert(
            attempt_id.clone(),
            HeimdallAuthAttempt {
                expires_at_unix: now + 600,
                status: "pending".into(),
                account_session_hash: None,
                error: None,
            },
        );
        if let Err(error) = auth.commit(next_state) {
            return (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()).into_response();
        }
    }
    match state.heimdall.start(&attempt_id).await {
        Ok(start) => Json(serde_json::json!({
            "attempt_id": attempt_id,
            "authorization_url": start.authorization_url
        }))
        .into_response(),
        Err(error) => {
            let message = error.to_string();
            let mut auth = state.auth.lock().await;
            let mut next_state = auth.state.clone();
            if let Some(attempt) = next_state.heimdall_attempts.get_mut(&attempt_id) {
                attempt.status = "failed".into();
                attempt.error = Some("Heimdall could not start Discord sign-in".into());
            }
            if let Err(commit_error) = auth.commit(next_state) {
                return (StatusCode::INTERNAL_SERVER_ERROR, commit_error.to_string())
                    .into_response();
            }
            (StatusCode::BAD_GATEWAY, message).into_response()
        }
    }
}

async fn heimdall_callback(
    State(state): State<AppState>,
    Json(callback): Json<BackendCallback>,
) -> Response {
    if callback.attempt_id.len() > 128 || callback.attempt_id.trim().is_empty() {
        return (StatusCode::BAD_REQUEST, "invalid Heimdall attempt id").into_response();
    }
    if let Err(error) = state.heimdall.validate_callback_envelope(&callback) {
        return (StatusCode::UNAUTHORIZED, error.to_string()).into_response();
    }
    let now = unix_time_seconds();
    {
        let auth = state.auth.lock().await;
        let Some(attempt) = auth.state.heimdall_attempts.get(&callback.attempt_id) else {
            return (StatusCode::NOT_FOUND, "unknown Heimdall attempt").into_response();
        };
        if attempt.expires_at_unix < now || attempt.status != "pending" {
            return (StatusCode::CONFLICT, "Heimdall attempt is not pending").into_response();
        }
    }
    if callback.status != "success" {
        let mut auth = state.auth.lock().await;
        let mut next_state = auth.state.clone();
        if let Some(attempt) = next_state.heimdall_attempts.get_mut(&callback.attempt_id) {
            attempt.status = "failed".into();
            attempt.error = Some(
                callback
                    .error_description
                    .clone()
                    .or(callback.error.clone())
                    .unwrap_or_else(|| "Discord membership was not admitted".into()),
            );
        }
        return match auth.commit(next_state) {
            Ok(()) => StatusCode::NO_CONTENT.into_response(),
            Err(error) => (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()).into_response(),
        };
    }
    let claims = match state.heimdall.verify_callback(&callback).await {
        Ok(claims) => claims,
        Err(error) => return (StatusCode::UNAUTHORIZED, error.to_string()).into_response(),
    };
    let account_session = secret_hash(&format!("heimdall-account:{}", claims.account_id));
    let mut auth = state.auth.lock().await;
    let mut next_state = auth.state.clone();
    let Some(attempt) = next_state.heimdall_attempts.get_mut(&callback.attempt_id) else {
        return (StatusCode::NOT_FOUND, "unknown Heimdall attempt").into_response();
    };
    if attempt.expires_at_unix < now || attempt.status != "pending" {
        return (StatusCode::CONFLICT, "Heimdall attempt is not pending").into_response();
    }
    attempt.status = "succeeded".into();
    attempt.account_session_hash = Some(account_session);
    match auth.commit(next_state) {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(error) => (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()).into_response(),
    }
}

async fn heimdall_attempt(
    Path(attempt_id): Path<String>,
    State(state): State<AppState>,
) -> Response {
    let auth = state.auth.lock().await;
    let Some(attempt) = auth.state.heimdall_attempts.get(&attempt_id) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    if attempt.expires_at_unix < unix_time_seconds() {
        return StatusCode::GONE.into_response();
    }
    Json(serde_json::json!({
        "status": attempt.status,
        "error": attempt.error,
    }))
    .into_response()
}

async fn heimdall_adopt(Path(attempt_id): Path<String>, State(state): State<AppState>) -> Response {
    let now = unix_time_seconds();
    let raw_session = uuid::Uuid::new_v4().to_string();
    let alias_hash = secret_hash(&raw_session);
    let mut auth = state.auth.lock().await;
    let mut next_state = auth.state.clone();
    let Some(attempt) = next_state.heimdall_attempts.remove(&attempt_id) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    if attempt.expires_at_unix < now {
        return StatusCode::GONE.into_response();
    }
    if attempt.status != "succeeded" {
        return (StatusCode::CONFLICT, "Heimdall attempt has not succeeded").into_response();
    }
    let Some(account_session) = attempt.account_session_hash else {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Heimdall attempt lost account authority",
        )
            .into_response();
    };
    next_state.session_hashes.insert(account_session.clone());
    next_state
        .session_aliases
        .insert(alias_hash, account_session);
    if let Err(error) = auth.commit(next_state) {
        return (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()).into_response();
    }
    let mut response = Json(serde_json::json!({"status":"authenticated"})).into_response();
    response.headers_mut().insert(
        header::SET_COOKIE,
        HeaderValue::from_str(&format!(
            "ghostlight_session={raw_session}; HttpOnly; Secure; SameSite=Lax; Path=/"
        ))
        .unwrap(),
    );
    response
}

fn unix_time_seconds() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn prune_heimdall_attempts(state: &mut AuthState, now: u64) {
    state
        .heimdall_attempts
        .retain(|_, attempt| attempt.expires_at_unix >= now);
}

async fn require_api_authentication(
    State(state): State<AppState>,
    headers: HeaderMap,
    request: Request,
    next: Next,
) -> Response {
    if !authorized(&headers, &state).await {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    next.run(request).await
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
                material: true,
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
                    material: true,
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

async fn session_zero_surface_route(
    Path(session_id): Path<uuid::Uuid>,
    headers: HeaderMap,
    State(state): State<AppState>,
) -> Response {
    let account_hash = match authenticated_session(&headers, &state).await {
        Some(value) => value,
        None => return StatusCode::UNAUTHORIZED.into_response(),
    };
    match state.session_zeros.snapshot(session_id).await {
        Ok(snapshot) => match session_zero_surface(&snapshot, &account_hash) {
            Ok(surface) => Json(surface).into_response(),
            Err(error) => (StatusCode::FORBIDDEN, error.to_string()).into_response(),
        },
        Err(error) => (StatusCode::NOT_FOUND, error.to_string()).into_response(),
    }
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
    if let (Some(director), Some(channel)) = (state.session_zero_director.clone(), channel) {
        let private_member = if channel.kind
            == ghostlight_dungeon::session_zero::SessionZeroChannelKind::PrivateDm
        {
            member_id.clone()
        } else {
            None
        };
        let component_epoch = private_member
            .as_ref()
            .and_then(|id| result.state.character_epochs.get(id).copied())
            .unwrap_or(result.state.shared_epoch);
        let snapshot = result.state.clone();
        let kernel = runtime.kernel.clone();
        let mesh_state = state.clone();
        tokio::spawn(async move {
            match director
                .respond(&snapshot, &channel_id, private_member.as_deref())
                .await
            {
                Ok((delta, receipts)) => {
                    if let Err(error) = kernel
                        .command(SessionZeroCommand::ApplyDmTurn {
                            expected_component_epoch: component_epoch,
                            expected_channel_revision: channel.revision,
                            channel_id,
                            member_id: private_member,
                            delta,
                            model_receipts: receipts,
                        })
                        .await
                    {
                        tracing::info!(%error, "stale or invalid Session Zero DM proposal discarded");
                    } else {
                        schedule_mesh_refresh(&mesh_state);
                    }
                }
                Err(error) => {
                    tracing::warn!(%error, "Session Zero DM inference failed without draft mutation");
                    let failure = ghostlight_dungeon::session_zero::SessionZeroDelta {
                        dm_speech: "I couldn't finish that response. Your message is safely recorded and no draft state changed; please retry or rephrase when you're ready.".into(),
                        ..Default::default()
                    };
                    if let Err(stale) = kernel
                        .command(SessionZeroCommand::ApplyDmTurn {
                            expected_component_epoch: component_epoch,
                            expected_channel_revision: channel.revision,
                            channel_id,
                            member_id: private_member,
                            delta: failure,
                            model_receipts: Vec::new(),
                        })
                        .await
                    {
                        tracing::info!(%stale, "stale Session Zero DM failure notice discarded");
                    } else {
                        schedule_mesh_refresh(&mesh_state);
                    }
                }
            }
        });
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

async fn surface(headers: HeaderMap, State(state): State<AppState>) -> Response {
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
                Ok(Some(id)) => match state.session_zeros.snapshot(id).await.and_then(|snapshot| session_zero_surface(&snapshot, &session)) {
                    Ok(surface) => Json(surface).into_response(),
                    Err(error) => (StatusCode::INTERNAL_SERVER_ERROR,error.to_string()).into_response(),
                },
                Ok(None) => Json(serde_json::json!({"schema":"gamecult.eve.surface.v1","surface_id":"ghostlight.session-zero-entry","version":0,"title":"Begin Session Zero","layout":{"kind":"stack","children":[]}})).into_response(),
                Err(error) => (StatusCode::INTERNAL_SERVER_ERROR,error.to_string()).into_response(),
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
            }
            Json(projected).into_response()
        }
        Err(error) => (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()).into_response(),
    }
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
            "DeepSeek credential is unavailable",
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
            "DeepSeek credential is unavailable",
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
        model_name: "deepseek-v4-pro".into(),
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
                        error: "DeepSeek assessor is unavailable".into(),
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
                                model_name: "deepseek-v4-flash".into(),
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
                                projector_model: "deepseek-v4-flash".into(),
                                persona_model: "deepseek-v4-pro".into(),
                                interpreter_model: "deepseek-v4-flash".into(),
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
                            error: "DeepSeek assessor is unavailable".into(),
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

fn campaign_branch_projection(kind: &str, campaign: &Campaign) -> serde_json::Value {
    serde_json::json!({
        "kind":kind,
        "campaign_id":campaign.id,
        "name":campaign.name,
        "revision":campaign.revision,
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
    let deepseek = state.deepseek_status.clone();
    let pressure = state.live_turns.load(Ordering::SeqCst);
    tokio::task::spawn_blocking(move || {
        publisher.publish_snapshot(&snapshots, &session_zero_snapshots, &deepseek, pressure)
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

#[derive(Deserialize)]
struct CampaignBranchRequest {
    name: String,
}

async fn campaigns(headers: HeaderMap, State(state): State<AppState>) -> Response {
    let session = match authenticated_session(&headers, &state).await {
        Some(value) => value,
        None => return StatusCode::UNAUTHORIZED.into_response(),
    };
    let ids = state
        .auth
        .lock()
        .await
        .state
        .session_campaign_ids
        .get(&session)
        .cloned()
        .unwrap_or_default();
    let mut values = Vec::new();
    let selected = state
        .auth
        .lock()
        .await
        .state
        .session_campaigns
        .get(&session)
        .copied();
    for id in ids {
        if let Ok(runtime) = state.registry.runtime(id).await {
            if let Ok(campaign) = load_campaign(&runtime.store) {
                values.push(serde_json::json!({"id":id,"name":campaign.name,"revision":campaign.revision,"selected":selected==Some(id)}));
            }
        }
    }
    Json(serde_json::json!({"schema":"ghostlight.campaign_list.v1","campaigns":values}))
        .into_response()
}

async fn select_campaign_route(
    Path(campaign_id): Path<uuid::Uuid>,
    headers: HeaderMap,
    State(state): State<AppState>,
) -> Response {
    let session = match authenticated_session(&headers, &state).await {
        Some(value) => value,
        None => return StatusCode::UNAUTHORIZED.into_response(),
    };
    let owned = state
        .auth
        .lock()
        .await
        .state
        .session_campaign_ids
        .get(&session)
        .is_some_and(|ids| ids.contains(&campaign_id));
    if !owned {
        return StatusCode::FORBIDDEN.into_response();
    }
    match select_campaign(&state, &session, campaign_id).await {
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

async fn fork_campaign(
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(request): Json<CampaignBranchRequest>,
) -> Response {
    let session = match authenticated_session(&headers, &state).await {
        Some(value) => value,
        None => return StatusCode::UNAUTHORIZED.into_response(),
    };
    let source = match state
        .auth
        .lock()
        .await
        .state
        .session_campaigns
        .get(&session)
        .copied()
    {
        Some(value) => value,
        None => return (StatusCode::NOT_FOUND, "session has no selected campaign").into_response(),
    };
    let source_runtime = match state.registry.runtime(source).await {
        Ok(runtime) => runtime,
        Err(error) => return (StatusCode::NOT_FOUND, error.to_string()).into_response(),
    };
    if campaign_is_group(&source_runtime.store, source) {
        return (
            StatusCode::CONFLICT,
            "fork is disabled for co-op campaigns until departure and ownership governance exist",
        )
            .into_response();
    }
    match state.registry.fork(source, request.name).await {
        Ok(runtime) => {
            let campaign = match load_campaign(&runtime.store) {
                Ok(value) => value,
                Err(error) => {
                    return (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()).into_response();
                }
            };
            match select_campaign(&state, &session, campaign.id).await {
                Ok(()) => {
                    if let Err(error) = refresh_mesh(&state).await {
                        tracing::warn!(%error, "campaign fork CultMesh publication failed");
                    }
                    Json(campaign_branch_projection("forked", &campaign)).into_response()
                }
                Err(error) => {
                    (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()).into_response()
                }
            }
        }
        Err(error) => (StatusCode::CONFLICT, error.to_string()).into_response(),
    }
}

async fn reset_campaign(
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(request): Json<CampaignBranchRequest>,
) -> Response {
    let session = match authenticated_session(&headers, &state).await {
        Some(value) => value,
        None => return StatusCode::UNAUTHORIZED.into_response(),
    };
    let source = match state
        .auth
        .lock()
        .await
        .state
        .session_campaigns
        .get(&session)
        .copied()
    {
        Some(value) => value,
        None => return (StatusCode::NOT_FOUND, "session has no selected campaign").into_response(),
    };
    let source_runtime = match state.registry.runtime(source).await {
        Ok(runtime) => runtime,
        Err(error) => return (StatusCode::NOT_FOUND, error.to_string()).into_response(),
    };
    if campaign_is_group(&source_runtime.store, source) {
        return (
            StatusCode::CONFLICT,
            "reset is disabled for co-op campaigns until unanimous lifecycle governance exists",
        )
            .into_response();
    }
    match state.registry.reset(source, request.name).await {
        Ok(runtime) => {
            let campaign = match load_campaign(&runtime.store) {
                Ok(value) => value,
                Err(error) => {
                    return (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()).into_response();
                }
            };
            match select_campaign(&state, &session, campaign.id).await {
                Ok(()) => {
                    if let Err(error) = refresh_mesh(&state).await {
                        tracing::warn!(%error, "campaign reset CultMesh publication failed");
                    }
                    Json(campaign_branch_projection("reset", &campaign)).into_response()
                }
                Err(error) => {
                    (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()).into_response()
                }
            }
        }
        Err(error) => (StatusCode::CONFLICT, error.to_string()).into_response(),
    }
}

async fn export_campaign(headers: HeaderMap, State(state): State<AppState>) -> Response {
    let session = match authenticated_session(&headers, &state).await {
        Some(value) => value,
        None => return StatusCode::UNAUTHORIZED.into_response(),
    };
    let campaign_id = match state
        .auth
        .lock()
        .await
        .state
        .session_campaigns
        .get(&session)
        .copied()
    {
        Some(value) => value,
        None => return (StatusCode::NOT_FOUND, "session has no selected campaign").into_response(),
    };
    let runtime = match state.registry.runtime(campaign_id).await {
        Ok(runtime) => runtime,
        Err(error) => return (StatusCode::NOT_FOUND, error.to_string()).into_response(),
    };
    if campaign_is_group(&runtime.store, campaign_id) {
        return (
            StatusCode::CONFLICT,
            "export is disabled for co-op campaigns until private-state consent governance exists",
        )
            .into_response();
    }
    match state
        .registry
        .export(campaign_id, state.runtime_root.join("exports"))
        .await
    {
        Ok(path) => match std::fs::read(&path) {
            Ok(bytes) => {
                let filename = path
                    .file_name()
                    .and_then(|value| value.to_str())
                    .unwrap_or("campaign.cc");
                let mut response = Response::new(axum::body::Body::from(bytes));
                response.headers_mut().insert(
                    header::CONTENT_TYPE,
                    HeaderValue::from_static("application/octet-stream"),
                );
                response.headers_mut().insert(
                    header::CONTENT_DISPOSITION,
                    HeaderValue::from_str(&format!("attachment; filename=\"{filename}\"")).unwrap(),
                );
                response
            }
            Err(error) => (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()).into_response(),
        },
        Err(error) => (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()).into_response(),
    }
}

fn campaign_is_group(store: &CampaignStore, campaign_id: uuid::Uuid) -> bool {
    store
        .load::<ghostlight_dungeon::session_zero::CampaignMembership>(
            "campaign_membership.v1",
            &campaign_id.to_string(),
        )
        .ok()
        .flatten()
        .is_some_and(|(_, membership)| {
            membership
                .members
                .values()
                .filter(|member| member.active)
                .count()
                > 1
        })
}

async fn export_canon_candidates_markdown(
    headers: HeaderMap,
    State(state): State<AppState>,
) -> Response {
    let session = match authenticated_session(&headers, &state).await {
        Some(value) => value,
        None => return StatusCode::UNAUTHORIZED.into_response(),
    };
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
        Err(error) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()).into_response();
        }
    };
    let mut markdown = format!(
        "# Canon candidates — {}\n\nCampaign: `{}`  \nRevision: `{}`\n\n",
        campaign.name, campaign.id, campaign.revision
    );
    if campaign.canon_candidates.is_empty() {
        markdown.push_str("No canon candidates have been recorded.\n");
    }
    for candidate in campaign.canon_candidates.values() {
        markdown.push_str(&format!(
            "## {}\n\n- Status: `{}`\n- Gap: {}\n- Proposed wording: {}\n- Evidence receipts: {}\n- Affected Vault sources: {}\n- Conflicts: {}\n\n",
            candidate.id,
            candidate.status,
            candidate.gap,
            candidate.proposed_wording,
            candidate.evidence_receipt_ids.join(", "),
            candidate.affected_vault_sources.join(", "),
            candidate.conflicts.join("; "),
        ));
    }
    let mut response = Response::new(axum::body::Body::from(markdown));
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("text/markdown; charset=utf-8"),
    );
    response.headers_mut().insert(
        header::CONTENT_DISPOSITION,
        HeaderValue::from_static("attachment; filename=canon-candidates.md"),
    );
    response
}

async fn set_provider_parallelism(
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(request): Json<ProviderParallelismRequest>,
) -> Response {
    if !operator_peer_allowed(peer) {
        return StatusCode::FORBIDDEN.into_response();
    }
    let session = match authenticated_session(&headers, &state).await {
        Some(value) => value,
        None => return StatusCode::UNAUTHORIZED.into_response(),
    };
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
    match runtime
        .kernel
        .command(WorldCommand::SetProviderParallelism {
            expected_revision: campaign.revision,
            expected_provider_configuration_epoch: request.expected_provider_configuration_epoch,
            provider_parallelism: request.provider_parallelism,
        })
        .await
    {
        Ok(value) => {
            if let Err(error) = refresh_mesh(&state).await {
                tracing::warn!(%error, "provider concurrency CultMesh publication failed");
            }
            Json(value).into_response()
        }
        Err(error) => (StatusCode::CONFLICT, error.to_string()).into_response(),
    }
}

async fn operator_inspector(
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    State(state): State<AppState>,
) -> Response {
    if !operator_peer_allowed(peer) {
        return StatusCode::FORBIDDEN.into_response();
    }
    let session = match authenticated_session(&headers, &state).await {
        Some(value) => value,
        None => return StatusCode::UNAUTHORIZED.into_response(),
    };
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
        Err(error) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()).into_response();
        }
    };
    match state.mesh.operator_surface(campaign.id) {
        Ok(surface) => Json(surface).into_response(),
        Err(error) => (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()).into_response(),
    }
}

fn operator_peer_allowed(peer: SocketAddr) -> bool {
    peer.ip().is_loopback()
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

async fn authorized(headers: &HeaderMap, state: &AppState) -> bool {
    authenticated_session(headers, state).await.is_some()
}

async fn authenticated_session(headers: &HeaderMap, state: &AppState) -> Option<String> {
    let session = headers
        .get(header::COOKIE)
        .and_then(|v| v.to_str().ok())
        .and_then(|cookies| {
            cookies
                .split(';')
                .map(str::trim)
                .find_map(|cookie| cookie.strip_prefix("ghostlight_session="))
        });
    let hash = secret_hash(session?);
    let auth = state.auth.lock().await;
    if auth.state.session_hashes.contains(&hash) {
        return Some(hash);
    }
    auth.state.session_aliases.get(&hash).cloned()
}

async fn session_runtime(
    state: &AppState,
    session_hash: &str,
) -> anyhow::Result<Option<CampaignRuntime>> {
    let campaign_id = state
        .auth
        .lock()
        .await
        .state
        .session_campaigns
        .get(session_hash)
        .copied();
    match campaign_id {
        Some(id) => Ok(Some(state.registry.runtime(id).await?)),
        None => Ok(None),
    }
}

async fn select_campaign(
    state: &AppState,
    session_hash: &str,
    campaign_id: uuid::Uuid,
) -> anyhow::Result<()> {
    state.registry.runtime(campaign_id).await?;
    let mut auth = state.auth.lock().await;
    if !auth.state.session_hashes.contains(session_hash) {
        return Err(anyhow::anyhow!("session is no longer authorized"));
    }
    let selection = |current: &AuthState| {
        let mut next_state = current.clone();
        next_state
            .session_campaigns
            .insert(session_hash.to_owned(), campaign_id);
        next_state
            .session_campaign_ids
            .entry(session_hash.to_owned())
            .or_default()
            .insert(campaign_id);
        next_state
    };
    let current = auth.state.clone();
    if auth.commit(selection(&current)).is_ok() {
        return Ok(());
    }

    auth.reload()?;
    if !auth.state.session_hashes.contains(session_hash) {
        return Err(anyhow::anyhow!("session is no longer authorized"));
    }
    let current = auth.state.clone();
    auth.commit(selection(&current))
}

fn secret_hash(value: &str) -> String {
    format!("sha256:{:x}", Sha256::digest(value.as_bytes()))
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
    auth: &AuthState,
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
        let auth_store = CampaignStore::open(root.join("auth.cc")).unwrap();
        let auth_state = AuthState {
            schema: "ghostlight.auth_state.v1".into(),
            session_hashes: BTreeSet::new(),
            session_aliases: BTreeMap::new(),
            session_campaigns: BTreeMap::new(),
            session_campaign_ids: BTreeMap::new(),
            heimdall_attempts: BTreeMap::new(),
        };
        let row = auth_store
            .insert(
                "auth_state.v1",
                "ghostlight.auth_state.v1",
                "primary",
                &auth_state,
            )
            .unwrap();
        AppState {
            registry,
            session_zeros: SessionZeroRegistry::new(root.join("session-zero")).unwrap(),
            session_zero_director: None,
            entitlements: Arc::new(FixtureEntitlementPort),
            runtime_root: root.into(),
            auth: Arc::new(Mutex::new(AuthOwner {
                store: auth_store,
                row,
                state: auth_state,
            })),
            heimdall: Arc::new(HeimdallClient::fixture()),
            deepseek_status: "fixture".into(),
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

    #[tokio::test]
    async fn api_authentication_precedes_json_extraction() {
        let dir = tempfile::tempdir().unwrap();
        let state = empty_app_state(dir.path());
        let app = app_router(state, dir.path().join("web"));
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/command")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from("not-json"))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn authenticated_api_requests_reach_json_extraction() {
        let dir = tempfile::tempdir().unwrap();
        let state = empty_app_state(dir.path());
        state
            .auth
            .lock()
            .await
            .state
            .session_hashes
            .insert(secret_hash("valid-session"));
        let app = app_router(state, dir.path().join("web"));
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/command")
                    .header(header::CONTENT_TYPE, "application/json")
                    .header(header::COOKIE, "ghostlight_session=valid-session")
                    .body(Body::from("not-json"))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn heimdall_cookie_alias_resolves_to_stable_account_campaign_authority() {
        let dir = tempfile::tempdir().unwrap();
        let state = empty_app_state(dir.path());
        let account_session = secret_hash("heimdall-account:acct-1");
        let cookie_hash = secret_hash("browser-session");
        {
            let mut auth = state.auth.lock().await;
            auth.state.session_hashes.insert(account_session.clone());
            auth.state
                .session_aliases
                .insert(cookie_hash, account_session.clone());
        }
        let mut headers = HeaderMap::new();
        headers.insert(
            header::COOKIE,
            HeaderValue::from_static("ghostlight_session=browser-session"),
        );
        assert_eq!(
            authenticated_session(&headers, &state).await,
            Some(account_session)
        );
    }

    #[tokio::test]
    async fn completed_heimdall_attempt_is_single_use_session_authority() {
        let dir = tempfile::tempdir().unwrap();
        let state = empty_app_state(dir.path());
        {
            let mut auth = state.auth.lock().await;
            let mut next = auth.state.clone();
            next.heimdall_attempts.insert(
                "attempt-one".into(),
                HeimdallAuthAttempt {
                    expires_at_unix: unix_time_seconds() + 60,
                    status: "succeeded".into(),
                    account_session_hash: Some("account-authority".into()),
                    error: None,
                },
            );
            auth.commit(next).unwrap();
        }

        let router = app_router(state.clone(), dir.path().join("web"));
        let adopted = router
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/auth/heimdall/attempt/attempt-one/adopt")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(adopted.status(), StatusCode::OK);
        let cookie = adopted.headers()[header::SET_COOKIE].to_str().unwrap();
        assert!(cookie.contains("HttpOnly"));
        assert!(cookie.contains("Secure"));
        assert!(
            state
                .auth
                .lock()
                .await
                .state
                .session_hashes
                .contains("account-authority")
        );

        let replay = router
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/auth/heimdall/attempt/attempt-one/adopt")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(replay.status(), StatusCode::NOT_FOUND);

        let retired_invite = router
            .oneshot(
                Request::builder()
                    .uri("/invite/old-authority")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_ne!(retired_invite.status(), StatusCode::OK);
        assert!(retired_invite.headers().get(header::SET_COOKIE).is_none());
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
    async fn sessions_resolve_only_their_selected_campaign_runtime() {
        let dir = tempfile::tempdir().unwrap();
        let registry = CampaignRegistry::new(dir.path().join("campaigns")).unwrap();
        let left = seed("Left");
        let right = seed("Right");
        registry.create(left.clone(), vec![], vec![]).await.unwrap();
        registry
            .create(right.clone(), vec![], vec![])
            .await
            .unwrap();
        let auth_store = CampaignStore::open(dir.path().join("auth.cc")).unwrap();
        let auth_state = AuthState {
            schema: "ghostlight.auth_state.v1".into(),
            session_hashes: BTreeSet::from(["left".into(), "right".into()]),
            session_aliases: BTreeMap::new(),
            session_campaigns: BTreeMap::from([
                ("left".into(), left.id),
                ("right".into(), right.id),
            ]),
            session_campaign_ids: BTreeMap::from([
                ("left".into(), BTreeSet::from([left.id])),
                ("right".into(), BTreeSet::from([right.id])),
            ]),
            heimdall_attempts: BTreeMap::new(),
        };
        let row = auth_store
            .insert(
                "auth_state.v1",
                "ghostlight.auth_state.v1",
                "primary",
                &auth_state,
            )
            .unwrap();
        let state = AppState {
            registry,
            session_zeros: SessionZeroRegistry::new(dir.path().join("session-zero")).unwrap(),
            session_zero_director: None,
            entitlements: Arc::new(FixtureEntitlementPort),
            runtime_root: dir.path().into(),
            auth: Arc::new(Mutex::new(AuthOwner {
                store: auth_store,
                row,
                state: auth_state,
            })),
            heimdall: Arc::new(HeimdallClient::fixture()),
            deepseek_status: "fixture".into(),
            compiler: None,
            assessor: None,
            model: None,
            expansion_previews: Arc::new(Mutex::new(BTreeMap::new())),
            fission_previews: Arc::new(Mutex::new(BTreeMap::new())),
            live_turns: Arc::new(AtomicUsize::new(0)),
            live_turn_started: Arc::new(Notify::new()),
            live_commit_gate: Arc::new(RwLock::new(())),
            mesh: MeshPublisher::open(dir.path().join("mesh.cc"), None).unwrap(),
        };
        let left_runtime = session_runtime(&state, "left").await.unwrap().unwrap();
        let right_runtime = session_runtime(&state, "right").await.unwrap().unwrap();
        assert_eq!(load_campaign(&left_runtime.store).unwrap().id, left.id);
        assert_eq!(load_campaign(&right_runtime.store).unwrap().id, right.id);
        assert_ne!(
            left_runtime.store.identity(),
            right_runtime.store.identity()
        );
        assert!(!state.auth.lock().await.state.session_campaign_ids["left"].contains(&right.id));
    }

    #[test]
    fn failed_auth_commit_cannot_change_in_memory_authority() {
        let dir = tempfile::tempdir().unwrap();
        let store = CampaignStore::open(dir.path().join("auth.cc")).unwrap();
        let original = AuthState {
            schema: "ghostlight.auth_state.v1".into(),
            session_hashes: BTreeSet::new(),
            session_aliases: BTreeMap::new(),
            session_campaigns: BTreeMap::new(),
            session_campaign_ids: BTreeMap::new(),
            heimdall_attempts: BTreeMap::new(),
        };
        let row = store
            .insert(
                "auth_state.v1",
                "ghostlight.auth_state.v1",
                "primary",
                &original,
            )
            .unwrap();
        let mut owner = AuthOwner {
            store: store.clone(),
            row: row.clone(),
            state: original.clone(),
        };

        let mut externally_committed = original.clone();
        externally_committed.session_hashes.insert("other".into());
        store
            .replace(&row, "ghostlight.auth_state.v1", &externally_committed)
            .unwrap();

        let mut rejected = original.clone();
        rejected.session_hashes.insert("phantom".into());
        assert!(owner.commit(rejected).is_err());
        assert_eq!(owner.state.session_hashes, original.session_hashes);
        assert_eq!(owner.row, row);
    }

    #[tokio::test]
    async fn campaign_selection_reloads_stale_auth_without_losing_concurrent_state() {
        let dir = tempfile::tempdir().unwrap();
        let state = empty_app_state(dir.path());
        let campaign = seed("Fresh branch");
        state
            .registry
            .create(campaign.clone(), vec![], vec![])
            .await
            .unwrap();

        let (store, row, mut externally_committed) = {
            let mut auth = state.auth.lock().await;
            let mut authorized = auth.state.clone();
            authorized.session_hashes.insert("owner".into());
            auth.commit(authorized).unwrap();
            (auth.store.clone(), auth.row.clone(), auth.state.clone())
        };
        externally_committed
            .session_hashes
            .insert("concurrent".into());
        store
            .replace(&row, "ghostlight.auth_state.v1", &externally_committed)
            .unwrap();

        select_campaign(&state, "owner", campaign.id).await.unwrap();

        let auth = state.auth.lock().await;
        assert!(auth.state.session_hashes.contains("concurrent"));
        assert_eq!(auth.state.session_campaigns["owner"], campaign.id);
        assert!(auth.state.session_campaign_ids["owner"].contains(&campaign.id));
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
    fn operator_http_boundary_is_loopback_only() {
        assert!(operator_peer_allowed("127.0.0.1:8831".parse().unwrap()));
        assert!(operator_peer_allowed("[::1]:8831".parse().unwrap()));
        assert!(!operator_peer_allowed(
            "192.168.178.158:55000".parse().unwrap()
        ));
        assert!(!operator_peer_allowed("10.77.0.4:55000".parse().unwrap()));
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
