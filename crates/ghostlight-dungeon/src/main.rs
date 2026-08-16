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
    compiler::{CustomStart, OpeningRequest, OpeningSuggestion, SelectedStart, WorldCompiler},
    domain::{
        ActionIntent, Campaign, NarrationProjection, RegionExpansionPreview,
        RejectedProposalReceipt, WorldCommand, WorldCompilePreview,
    },
    gestalt::GestaltPresencePlanner,
    kernel::CommandResult,
    mesh::MeshPublisher,
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
use tokio::sync::Mutex;
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
    live_turns: Arc<AtomicUsize>,
    mesh: MeshPublisher,
}

struct LiveTurnGuard(Arc<AtomicUsize>);
impl LiveTurnGuard {
    fn enter(counter: &Arc<AtomicUsize>) -> Self {
        counter.fetch_add(1, Ordering::SeqCst);
        Self(counter.clone())
    }
}
impl Drop for LiveTurnGuard {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::SeqCst);
    }
}

#[derive(Clone)]
struct OwnedPreview<T> {
    session_hash: String,
    value: T,
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
            },
        )
        .await?;
        (
            format!("ready:{}", probe.receipt.output_hash),
            Some(Arc::new(WorldCompiler::new(
                Arc::new(VoidBotMcpVault::starfire_loopback()),
                provider.clone(),
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
        live_turns: Arc::new(AtomicUsize::new(0)),
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
        .route("/api/command", post(command))
        .route("/api/campaigns", get(campaigns))
        .route(
            "/api/campaigns/select/{campaign_id}",
            post(select_campaign_route),
        )
        .route("/api/campaigns/fork", post(fork_campaign))
        .route("/api/campaigns/reset", post(reset_campaign))
        .route("/api/campaigns/export", get(export_campaign))
        .route("/api/operator", get(operator_inspector))
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
    let _live = LiveTurnGuard::enter(&state.live_turns);
    let Some(compiler) = &state.compiler else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            "DeepSeek credential is unavailable",
        )
            .into_response();
    };
    match compiler.suggest_openings(request).await {
        Ok(value) => Json(value).into_response(),
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
    let _live = LiveTurnGuard::enter(&state.live_turns);
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
        Ok((preview, receipt)) => {
            let id = uuid::Uuid::new_v4().to_string();
            state.compile_previews.lock().await.insert(
                id.clone(),
                OwnedPreview {
                    session_hash: session,
                    value: preview.clone(),
                },
            );
            Json(serde_json::json!({"preview_id":id,"preview":preview,"model_receipt":receipt}))
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
    let _live = LiveTurnGuard::enter(&state.live_turns);
    let Some(compiler) = &state.compiler else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            "DeepSeek credential is unavailable",
        )
            .into_response();
    };
    match compiler.suggest_roles(&opening).await {
        Ok(value) => Json(value).into_response(),
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
    let _live = LiveTurnGuard::enter(&state.live_turns);
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
        ghostlight_dungeon::model::ModelStageReceipt,
    )>,
) -> Response {
    match result {
        Ok((preview, receipt)) => {
            let id = uuid::Uuid::new_v4().to_string();
            state.compile_previews.lock().await.insert(
                id.clone(),
                OwnedPreview {
                    session_hash,
                    value: preview.clone(),
                },
            );
            Json(serde_json::json!({"preview_id":id,"preview":preview,"model_receipt":receipt}))
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
    let _live = LiveTurnGuard::enter(&state.live_turns);
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
    let campaign_id = preview.campaign.id;
    match state
        .registry
        .create(preview.campaign, preview.evidence_receipts)
        .await
    {
        Ok(runtime) => match select_campaign(&state, &session, campaign_id).await {
            Ok(()) => match load_campaign(&runtime.store) {
                Ok(campaign) => {
                    if let Err(error) = refresh_mesh(&state).await {
                        tracing::warn!(%error, "campaign approval CultMesh publication failed");
                    }
                    Json(CommandResult::Created { campaign }).into_response()
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
    let _live = LiveTurnGuard::enter(&state.live_turns);
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
        Ok((preview, receipt)) => {
            let id = uuid::Uuid::new_v4().to_string();
            state.expansion_previews.lock().await.insert(
                id.clone(),
                OwnedPreview {
                    session_hash: session,
                    value: preview.clone(),
                },
            );
            Json(serde_json::json!({"preview_id":id,"preview":preview,"model_receipt":receipt}))
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
        })
        .await
    {
        Ok(value) => Json(value).into_response(),
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
    let begun = runtime
        .kernel
        .command(WorldCommand::BeginNpcAction {
            expected_revision: campaign.revision,
            proposal: proposal.clone(),
        })
        .await?;
    let CommandResult::Committed {
        campaign: begun_campaign,
        ..
    } = &begun
    else {
        unreachable!()
    };
    let assessor = state
        .assessor
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("NPC initiative requires the action assessor"))?;
    let intent = ActionIntent {
        actor_id: proposal.actor_id,
        description: proposal.intent,
        intended_effect: proposal.intended_effect,
    };
    let (assessment, receipt) = assessor.assess(begun_campaign, intent.clone()).await?;
    let _ = runtime.store.insert(
        "persona_stage_receipt.v1",
        "ghostlight.persona_stage_receipt.v1",
        &receipt.output_hash,
        &receipt,
    );
    let assessed = runtime
        .kernel
        .command(WorldCommand::Assess {
            expected_revision: begun_campaign.revision,
            intent,
            proposal: Some(assessment.clone()),
        })
        .await?;
    if !assessment.admissible {
        return Ok(serde_json::json!({"begun":begun,"assessment":assessed,"attempt":null}));
    }
    let attempted = runtime
        .kernel
        .command(WorldCommand::Attempt {
            assessment_digest: assessment.digest,
        })
        .await?;
    Ok(serde_json::json!({"begun":begun,"assessment":assessed,"attempt":attempted}))
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
        &receipt.output_hash,
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
    let _live = LiveTurnGuard::enter(&state.live_turns);
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
    if let Err(error) = process_due_ticks(&runtime).await {
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
                        &receipt.output_hash,
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
                        let summary = campaign
                            .transcript
                            .last()
                            .map(|turn| turn.text.clone())
                            .unwrap_or_else(|| "A consequential event occurred.".into());
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
                                        &receipt.output_hash,
                                        &receipt,
                                    );
                                    if !plan.promotions.is_empty() || !plan.demotions.is_empty() {
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
                                            &receipt.output_hash,
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
                                            let initiative = match resolve_npc_initiative(
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
                                            return Json(serde_json::json!({"primary":result,"presence":presence_result,"reaction_wave":reaction,"npc_initiative":initiative,"narration":narration})).into_response();
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
                            return Json(
                                serde_json::json!({"primary":result,"presence":presence_result,"narration":narration}),
                            )
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
                Json(serde_json::json!({"result":result,"narration":narration})).into_response()
            } else {
                Json(result).into_response()
            }
        }
        Err(error) => {
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
                    if let Err(error) = process_due_ticks(&runtime).await {
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
        snapshots.push((campaign, narrations));
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
                    Json(campaign).into_response()
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
                    Json(campaign).into_response()
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
    let evidence = runtime
        .store
        .load_all::<ghostlight_dungeon::domain::VaultEvidenceReceipt>("vault_evidence_receipt.v1")
        .unwrap_or_default();
    let commits = runtime
        .store
        .load_all::<ghostlight_dungeon::domain::WorldCommitReceipt>("world_commit_receipt.v1")
        .unwrap_or_default();
    let stages = runtime
        .store
        .load_all::<ghostlight_dungeon::model::ModelStageReceipt>("persona_stage_receipt.v1")
        .unwrap_or_default();
    let rejected = runtime
        .store
        .load_all::<RejectedProposalReceipt>("rejected_proposal_receipt.v1")
        .unwrap_or_default();
    Json(serde_json::json!({"schema":"ghostlight.operator_inspector.v1","campaign":campaign,"evidence":evidence,"commit_receipts":commits,"model_stage_receipts":stages,"rejected_proposals":rejected,"scheduler":{"live_turn_pressure":state.live_turns.load(Ordering::SeqCst)}})).into_response()
}

async fn process_due_ticks(runtime: &CampaignRuntime) -> anyhow::Result<()> {
    loop {
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
        runtime
            .kernel
            .command(WorldCommand::AdvanceStrategicTick {
                expected_revision: campaign.revision,
                source: ghostlight_dungeon::domain::TickSource::ReturnCatchUp,
            })
            .await?;
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
        }
    }

    #[tokio::test]
    async fn sessions_resolve_only_their_selected_campaign_runtime() {
        let dir = tempfile::tempdir().unwrap();
        let registry = CampaignRegistry::new(dir.path().join("campaigns")).unwrap();
        let left = seed("Left");
        let right = seed("Right");
        registry.create(left.clone(), vec![]).await.unwrap();
        registry.create(right.clone(), vec![]).await.unwrap();
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
            live_turns: Arc::new(AtomicUsize::new(0)),
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
}
