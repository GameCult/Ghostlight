use axum::{
    Json, Router,
    extract::{Path, State},
    http::{HeaderMap, HeaderValue, StatusCode, header},
    response::{IntoResponse, Redirect, Response},
    routing::{get, post},
};
#[cfg(windows)]
use ghostlight_dungeon::windows_secret::unprotect_machine_utf8;
use ghostlight_dungeon::{
    assessor::ActionAssessor,
    compiler::{
        CustomStart, GestaltFissionRequest, OpeningRequest, OpeningSuggestion, SelectedStart,
        SuggestedOpenings, SuggestedRoles, WorldCompiler,
    },
    domain::{
        ActionIntent, Campaign, FactScope, GestaltFissionPreview, NarrationProjection,
        RegionExpansionPreview, RejectedProposalReceipt, WorldCommand, WorldCompilePreview,
    },
    gestalt::GestaltPresencePlanner,
    kernel::{CommandResult, KernelError},
    mesh::{CampaignMeshSnapshot, MeshPublisher},
    model::{DeepSeekPort, ModelPort, ModelStageRequest, run_validated_stage},
    narrator::Narrator,
    persistence::CampaignStore,
    persona::PersonaProjectionEngine,
    registry::{CampaignRegistry, CampaignRuntime},
    surface::player_surface,
    turn::{SnapshotPermit, appraise_present},
    vault::VoidBotMcpVault,
};
use serde::Deserialize;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    net::SocketAddr,
    path::PathBuf,
    sync::Arc,
    sync::atomic::{AtomicUsize, Ordering},
};
use tokio::sync::{Mutex, Notify, OwnedRwLockReadGuard, RwLock};
use tower_http::services::ServeDir;
use tracing_subscriber::EnvFilter;

#[derive(Clone)]
struct AppState {
    registry: CampaignRegistry,
    runtime_root: PathBuf,
    auth: Arc<Mutex<AuthOwner>>,
    deepseek_status: String,
    compiler: Option<Arc<WorldCompiler>>,
    assessor: Option<Arc<ActionAssessor>>,
    model: Option<Arc<dyn ModelPort>>,
    compile_previews: Arc<Mutex<BTreeMap<String, OwnedPreview<WorldCompilePreview>>>>,
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

#[derive(Clone, Serialize, Deserialize)]
struct AuthState {
    schema: String,
    unused_invite_hashes: BTreeSet<String>,
    session_hashes: BTreeSet<String>,
    #[serde(default)]
    session_campaigns: BTreeMap<String, uuid::Uuid>,
    #[serde(default)]
    session_campaign_ids: BTreeMap<String, BTreeSet<uuid::Uuid>>,
}

struct AuthOwner {
    store: CampaignStore,
    row: cultcache_legacy::CultCacheEnvelope,
    state: AuthState,
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
    let invite_blob = runtime_root.join("secrets/invites.dpapi");
    #[cfg(windows)]
    let invite_material = if invite_blob.is_file() {
        unprotect_machine_utf8(&invite_blob)?
    } else {
        std::env::var("GHOSTLIGHT_INVITES").map_err(|_| {
            anyhow::anyhow!(
                "protected invite blob is missing; run the privileged GhostlightDungeon setup"
            )
        })?
    };
    #[cfg(not(windows))]
    let invite_material = std::env::var("GHOSTLIGHT_INVITES")
        .map_err(|_| anyhow::anyhow!("GHOSTLIGHT_INVITES is required outside Windows"))?;
    let invite_tokens: BTreeSet<String> = invite_material
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
        .collect();
    if invite_tokens.len() != 2 {
        return Err(anyhow::anyhow!(
            "GhostlightDungeon requires exactly two unused invite tokens"
        ));
    }
    std::fs::create_dir_all(runtime_root.join("service"))?;
    let auth_store = CampaignStore::open(runtime_root.join("service/auth.cc"))?;
    let (auth_row, auth_state) = match auth_store.load::<AuthState>("auth_state.v1", "primary")? {
        Some(existing) => existing,
        None => {
            let state = AuthState {
                schema: "ghostlight.auth_state.v1".into(),
                unused_invite_hashes: invite_tokens
                    .iter()
                    .map(|token| secret_hash(token))
                    .collect(),
                session_hashes: BTreeSet::new(),
                session_campaigns: BTreeMap::new(),
                session_campaign_ids: BTreeMap::new(),
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
    let state = AppState {
        registry,
        runtime_root,
        auth: Arc::new(Mutex::new(AuthOwner {
            store: auth_store,
            row: auth_row,
            state: auth_state,
        })),
        deepseek_status,
        compiler,
        assessor,
        model: shared_model,
        compile_previews: Arc::new(Mutex::new(BTreeMap::new())),
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
    let app = Router::new()
        .route("/health", get(health))
        .route("/invite/{token}", get(invite))
        .route("/api/surface", get(surface))
        .route("/api/compiler/openings", post(compile_openings))
        .route("/api/compiler/roles", post(compile_roles))
        .route("/api/compiler/selected", post(compile_selected))
        .route("/api/compiler/custom", post(compile_custom))
        .route("/api/compiler/approve/{preview_id}", post(approve_preview))
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
        .route("/api/campaigns", get(campaigns))
        .route(
            "/api/campaigns/select/{campaign_id}",
            post(select_campaign_route),
        )
        .route("/api/campaigns/fork", post(fork_campaign))
        .route("/api/campaigns/reset", post(reset_campaign))
        .route("/api/campaigns/export", get(export_campaign))
        .route(
            "/api/campaigns/canon-candidates.md",
            get(export_canon_candidates_markdown),
        )
        .route("/api/operator", get(operator_inspector))
        .route(
            "/api/operator/provider-parallelism",
            post(set_provider_parallelism),
        )
        .fallback_service(ServeDir::new(web_root).append_index_html_on_directories(true))
        .with_state(state);
    let address: SocketAddr = std::env::var("GHOSTLIGHT_DUNGEON_BIND")
        .unwrap_or_else(|_| "0.0.0.0:8831".into())
        .parse()?;
    let listener = tokio::net::TcpListener::bind(address).await?;
    tracing::info!(%address, "GhostlightDungeon listening");
    axum::serve(listener, app).await?;
    Ok(())
}

async fn health(State(state): State<AppState>) -> Response {
    match state.mesh.health() {
        Ok(value) => Json(value).into_response(),
        Err(error) => (StatusCode::SERVICE_UNAVAILABLE, error.to_string()).into_response(),
    }
}

async fn invite(Path(token): Path<String>, State(state): State<AppState>) -> Response {
    let mut auth = state.auth.lock().await;
    if !auth.state.unused_invite_hashes.remove(&secret_hash(&token)) {
        return (StatusCode::UNAUTHORIZED, "invalid or consumed invite").into_response();
    }
    let session = uuid::Uuid::new_v4().to_string();
    auth.state.session_hashes.insert(secret_hash(&session));
    let next = match auth
        .store
        .replace(&auth.row, "ghostlight.auth_state.v1", &auth.state)
    {
        Ok(row) => row,
        Err(error) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()).into_response();
        }
    };
    auth.row = next;
    drop(auth);
    let mut response = Redirect::to("/").into_response();
    response.headers_mut().insert(
        header::SET_COOKIE,
        HeaderValue::from_str(&format!(
            "ghostlight_session={session}; HttpOnly; SameSite=Strict; Path=/"
        ))
        .unwrap(),
    );
    response
}

async fn surface(headers: HeaderMap, State(state): State<AppState>) -> Response {
    let session = match authenticated_session(&headers, &state).await {
        Some(value) => value,
        None => return StatusCode::UNAUTHORIZED.into_response(),
    };
    let runtime = match session_runtime(&state, &session).await {
        Ok(Some(value)) => value,
        Ok(None) => return Json(serde_json::json!({"schema":"gamecult.eve.surface.v1","surface_id":"ghostlight.compiler","version":0,"title":"Compile a world","layout":{"kind":"stack","children":[]}})).into_response(),
        Err(error) => return (StatusCode::INTERNAL_SERVER_ERROR,error.to_string()).into_response(),
    };
    match load_campaign(&runtime.store) {
        Ok(campaign) => {
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
            Json(player_surface(&campaign, &narrations)).into_response()
        }
        Err(error) => (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()).into_response(),
    }
}

async fn compile_openings(
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(request): Json<OpeningRequest>,
) -> Response {
    if !authorized(&headers, &state).await {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    let _live = LiveTurnGuard::enter(&state).await;
    let Some(compiler) = &state.compiler else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            "DeepSeek credential is unavailable",
        )
            .into_response();
    };
    match compiler.suggest_openings(request).await {
        Ok(value) => Json(opening_suggestions_projection(&value)).into_response(),
        Err(error) => (StatusCode::BAD_GATEWAY, error.to_string()).into_response(),
    }
}

async fn compile_custom(
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(request): Json<CustomStart>,
) -> Response {
    let session = match authenticated_session(&headers, &state).await {
        Some(value) => value,
        None => return StatusCode::UNAUTHORIZED.into_response(),
    };
    let _live = LiveTurnGuard::enter(&state).await;
    if session_runtime(&state, &session)
        .await
        .ok()
        .flatten()
        .is_some()
    {
        return (
            StatusCode::CONFLICT,
            "session already has a selected campaign",
        )
            .into_response();
    }
    let Some(compiler) = &state.compiler else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            "DeepSeek credential is unavailable",
        )
            .into_response();
    };
    match compiler.compile_custom(request).await {
        Ok((preview, model_receipts)) => {
            let id = uuid::Uuid::new_v4().to_string();
            state.compile_previews.lock().await.insert(
                id.clone(),
                OwnedPreview {
                    session_hash: session,
                    value: preview.clone(),
                    model_receipts: model_receipts.clone(),
                },
            );
            Json(serde_json::json!({
                "preview_id":id,
                "preview":world_compile_preview_projection(&preview),
            }))
            .into_response()
        }
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, error.to_string()).into_response(),
    }
}

async fn compile_roles(
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(opening): Json<OpeningSuggestion>,
) -> Response {
    if !authorized(&headers, &state).await {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    let _live = LiveTurnGuard::enter(&state).await;
    let Some(compiler) = &state.compiler else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            "DeepSeek credential is unavailable",
        )
            .into_response();
    };
    match compiler.suggest_roles(&opening).await {
        Ok(value) => Json(role_suggestions_projection(&value)).into_response(),
        Err(error) => (StatusCode::BAD_GATEWAY, error.to_string()).into_response(),
    }
}

async fn compile_selected(
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(request): Json<SelectedStart>,
) -> Response {
    let session = match authenticated_session(&headers, &state).await {
        Some(value) => value,
        None => return StatusCode::UNAUTHORIZED.into_response(),
    };
    let _live = LiveTurnGuard::enter(&state).await;
    if session_runtime(&state, &session)
        .await
        .ok()
        .flatten()
        .is_some()
    {
        return (
            StatusCode::CONFLICT,
            "session already has a selected campaign",
        )
            .into_response();
    }
    let Some(compiler) = &state.compiler else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            "DeepSeek credential is unavailable",
        )
            .into_response();
    };
    store_preview(&state, session, compiler.compile_selected(request).await).await
}

async fn store_preview(
    state: &AppState,
    session_hash: String,
    result: anyhow::Result<(
        WorldCompilePreview,
        Vec<ghostlight_dungeon::model::ModelStageReceipt>,
    )>,
) -> Response {
    match result {
        Ok((preview, model_receipts)) => {
            let id = uuid::Uuid::new_v4().to_string();
            state.compile_previews.lock().await.insert(
                id.clone(),
                OwnedPreview {
                    session_hash,
                    value: preview.clone(),
                    model_receipts: model_receipts.clone(),
                },
            );
            Json(serde_json::json!({
                "preview_id":id,
                "preview":world_compile_preview_projection(&preview),
            }))
            .into_response()
        }
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, error.to_string()).into_response(),
    }
}

async fn approve_preview(
    Path(preview_id): Path<String>,
    headers: HeaderMap,
    State(state): State<AppState>,
) -> Response {
    let session = match authenticated_session(&headers, &state).await {
        Some(value) => value,
        None => return StatusCode::UNAUTHORIZED.into_response(),
    };
    let _live = LiveTurnGuard::enter(&state).await;
    let mut previews = state.compile_previews.lock().await;
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
    let campaign_id = preview.campaign.id;
    match state
        .registry
        .create(preview.campaign, preview.evidence_receipts, model_receipts)
        .await
    {
        Ok(runtime) => match select_campaign(&state, &session, campaign_id).await {
            Ok(()) => match load_campaign(&runtime.store) {
                Ok(campaign) => {
                    if let Err(error) = refresh_mesh(&state).await {
                        tracing::warn!(%error, "campaign approval CultMesh publication failed");
                    }
                    Json(player_command_projection(
                        &CommandResult::Created { campaign },
                        None,
                    ))
                    .into_response()
                }
                Err(error) => {
                    (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()).into_response()
                }
            },
            Err(error) => (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()).into_response(),
        },
        Err(error) => (StatusCode::CONFLICT, error.to_string()).into_response(),
    }
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
            state.expansion_previews.lock().await.insert(
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
            state.fission_previews.lock().await.insert(
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
    let (assessment, receipt) = assessor.assess(campaign, intent).await?;
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
    let player_id = match load_campaign(&runtime.store) {
        Ok(campaign) => campaign.player_actor_id,
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
    if !player_http_command_allowed(&command, &player_id) {
        return (
            StatusCode::FORBIDDEN,
            Json(ErrorBody {
                error: "command is not admitted through the player HTTP boundary".into(),
            }),
        )
            .into_response();
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
            match assessor.assess(&campaign, intent.clone()).await {
                Ok((assessment, receipt)) => {
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
                match assessor.assess(&campaign, intent.clone()).await {
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

fn opening_suggestions_projection(value: &SuggestedOpenings) -> serde_json::Value {
    serde_json::json!({"openings":value.openings})
}

fn role_suggestions_projection(value: &SuggestedRoles) -> serde_json::Value {
    serde_json::json!({"roles":value.roles})
}

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
    let branch_facts = campaign
        .facts
        .values()
        .filter(|fact| fact.scope != FactScope::CanonBaseline)
        .map(|fact| {
            serde_json::json!({
                "id":fact.id,
                "scope":fact.scope,
                "statement":fact.statement,
                "discoverable_at_location_ids":fact.discoverable_at_location_ids,
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
        "branch_facts":branch_facts,
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
    }
}

fn player_http_command_allowed(command: &WorldCommand, player_actor_id: &str) -> bool {
    match command {
        WorldCommand::Speak { actor_id, .. } => actor_id == player_actor_id,
        WorldCommand::Assess {
            intent, proposal, ..
        } => intent.actor_id == player_actor_id && proposal.is_none(),
        WorldCommand::Attempt { .. }
        | WorldCommand::Wait { .. }
        | WorldCommand::SetResolutionBudget { .. }
        | WorldCommand::ReplaceResolutionPins { .. } => true,
        WorldCommand::CreateCampaign { .. }
        | WorldCommand::AdvanceStrategicTick { .. }
        | WorldCommand::ExpandRegion { .. }
        | WorldCommand::MaterializeGestaltMember { .. }
        | WorldCommand::DematerializeGestaltMember { .. }
        | WorldCommand::IndividuateGestaltMember { .. }
        | WorldCommand::ReconcileGestaltPresence { .. }
        | WorldCommand::ResolveReactionWave { .. }
        | WorldCommand::ResolveNpcAction { .. }
        | WorldCommand::SetProviderParallelism { .. }
        | WorldCommand::FissionGestalt { .. } => false,
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
            resolution_controls: runtime.store.load_all("resolution_control_receipt.v1")?,
        });
    }
    let publisher = state.mesh.clone();
    let deepseek = state.deepseek_status.clone();
    let pressure = state.live_turns.load(Ordering::SeqCst);
    tokio::task::spawn_blocking(move || publisher.publish_snapshot(&snapshots, &deepseek, pressure))
        .await?
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
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(request): Json<ProviderParallelismRequest>,
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

async fn operator_inspector(headers: HeaderMap, State(state): State<AppState>) -> Response {
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
        let Some(model) = &state.model else {
            return Ok(());
        };
        let permit = Arc::new(SnapshotPermit::new_resolution(
            runtime.store.clone(),
            campaign.id,
            campaign.revision,
            campaign.resolution_policy.resolution_epoch,
        ));
        let Some(output) = await_background_work(
            state,
            yield_to_live_turns,
            ghostlight_dungeon::scheduler::propose_resolution_wave(
                model.clone(),
                permit,
                &campaign,
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
    state
        .auth
        .lock()
        .await
        .state
        .session_hashes
        .contains(&hash)
        .then_some(hash)
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
    auth.state
        .session_campaigns
        .insert(session_hash.to_owned(), campaign_id);
    auth.state
        .session_campaign_ids
        .entry(session_hash.to_owned())
        .or_default()
        .insert(campaign_id);
    auth.row = auth
        .store
        .replace(&auth.row, "ghostlight.auth_state.v1", &auth.state)?;
    Ok(())
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
    use ghostlight_dungeon::domain::{ActorState, BranchOrigin, Location};

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
            unused_invite_hashes: BTreeSet::new(),
            session_hashes: BTreeSet::new(),
            session_campaigns: BTreeMap::new(),
            session_campaign_ids: BTreeMap::new(),
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
            runtime_root: root.into(),
            auth: Arc::new(Mutex::new(AuthOwner {
                store: auth_store,
                row,
                state: auth_state,
            })),
            deepseek_status: "fixture".into(),
            compiler: None,
            assessor: None,
            model: None,
            compile_previews: Arc::new(Mutex::new(BTreeMap::new())),
            expansion_previews: Arc::new(Mutex::new(BTreeMap::new())),
            fission_previews: Arc::new(Mutex::new(BTreeMap::new())),
            live_turns: Arc::new(AtomicUsize::new(0)),
            live_turn_started: Arc::new(Notify::new()),
            live_commit_gate: Arc::new(RwLock::new(())),
            mesh: MeshPublisher::open(root.join("mesh.cc"), None).unwrap(),
        }
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
            unused_invite_hashes: BTreeSet::new(),
            session_hashes: BTreeSet::from(["left".into(), "right".into()]),
            session_campaigns: BTreeMap::from([
                ("left".into(), left.id),
                ("right".into(), right.id),
            ]),
            session_campaign_ids: BTreeMap::from([
                ("left".into(), BTreeSet::from([left.id])),
                ("right".into(), BTreeSet::from([right.id])),
            ]),
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
            runtime_root: dir.path().into(),
            auth: Arc::new(Mutex::new(AuthOwner {
                store: auth_store,
                row,
                state: auth_state,
            })),
            deepseek_status: "fixture".into(),
            compiler: None,
            assessor: None,
            model: None,
            compile_previews: Arc::new(Mutex::new(BTreeMap::new())),
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
        ] {
            assert!(!encoded.contains(private_key), "leaked {private_key}");
        }
    }
}
