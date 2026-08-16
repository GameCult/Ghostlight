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
    WorldKernel,
    assessor::ActionAssessor,
    compiler::{CustomStart, OpeningRequest, OpeningSuggestion, SelectedStart, WorldCompiler},
    domain::{Campaign, RegionExpansionPreview, WorldCommand, WorldCompilePreview},
    model::{DeepSeekPort, ModelPort, ModelStageRequest, run_validated_stage},
    persistence::CampaignStore,
    persona::PersonaProjectionEngine,
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
};
use tokio::sync::Mutex;
use tower_http::services::ServeDir;
use tracing_subscriber::EnvFilter;

#[derive(Clone)]
struct AppState {
    kernel: WorldKernel,
    store: CampaignStore,
    auth: Arc<Mutex<AuthOwner>>,
    deepseek_status: String,
    compiler: Option<Arc<WorldCompiler>>,
    assessor: Option<Arc<ActionAssessor>>,
    model: Option<Arc<dyn ModelPort>>,
    compile_previews: Arc<Mutex<BTreeMap<String, WorldCompilePreview>>>,
    expansion_previews: Arc<Mutex<BTreeMap<String, RegionExpansionPreview>>>,
}

#[derive(Clone, Serialize, Deserialize)]
struct AuthState {
    schema: String,
    unused_invite_hashes: BTreeSet<String>,
    session_hashes: BTreeSet<String>,
}

struct AuthOwner {
    store: CampaignStore,
    row: cultcache_rs::CultCacheEnvelope,
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
    std::fs::create_dir_all(runtime_root.join("campaigns/default"))?;
    let store = CampaignStore::open(runtime_root.join("campaigns/default/campaign.cc"))?;
    let kernel = WorldKernel::start(store.clone());
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
    let state = AppState {
        kernel,
        store,
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
    };
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

async fn health(State(state): State<AppState>) -> Json<serde_json::Value> {
    Json(
        serde_json::json!({"schema":"ghostlight.service_health.v1","status":"ok","storeIdentity":state.store.identity(),"deepseek":state.deepseek_status}),
    )
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
    if !authorized(&headers, &state).await {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    match load_campaign(&state.store) {
        Ok(campaign) => Json(player_surface(&campaign)).into_response(),
        Err(_) => Json(serde_json::json!({"schema":"gamecult.eve.surface.v1","surface_id":"ghostlight.compiler","version":0,"title":"Compile a world","layout":{"kind":"stack","children":[]}})).into_response(),
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
    if !authorized(&headers, &state).await {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    if !state
        .store
        .keys("campaign.v1")
        .unwrap_or_default()
        .is_empty()
    {
        return (StatusCode::CONFLICT, "campaign already exists").into_response();
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
            state
                .compile_previews
                .lock()
                .await
                .insert(id.clone(), preview.clone());
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
    if !authorized(&headers, &state).await {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    if !state
        .store
        .keys("campaign.v1")
        .unwrap_or_default()
        .is_empty()
    {
        return (StatusCode::CONFLICT, "campaign already exists").into_response();
    }
    let Some(compiler) = &state.compiler else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            "DeepSeek credential is unavailable",
        )
            .into_response();
    };
    store_preview(&state, compiler.compile_selected(request).await).await
}

async fn store_preview(
    state: &AppState,
    result: anyhow::Result<(
        WorldCompilePreview,
        ghostlight_dungeon::model::ModelStageReceipt,
    )>,
) -> Response {
    match result {
        Ok((preview, receipt)) => {
            let id = uuid::Uuid::new_v4().to_string();
            state
                .compile_previews
                .lock()
                .await
                .insert(id.clone(), preview.clone());
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
    if !authorized(&headers, &state).await {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    let Some(preview) = state.compile_previews.lock().await.remove(&preview_id) else {
        return (StatusCode::NOT_FOUND, "preview missing or already consumed").into_response();
    };
    match state
        .kernel
        .command(WorldCommand::CreateCampaign {
            campaign: preview.campaign,
            evidence_receipts: preview.evidence_receipts,
        })
        .await
    {
        Ok(value) => Json(value).into_response(),
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
    if !authorized(&headers, &state).await {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    let campaign = match load_campaign(&state.store) {
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
            state
                .expansion_previews
                .lock()
                .await
                .insert(id.clone(), preview.clone());
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
    if !authorized(&headers, &state).await {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    let Some(preview) = state.expansion_previews.lock().await.remove(&preview_id) else {
        return (StatusCode::NOT_FOUND, "preview missing or already consumed").into_response();
    };
    match state
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

async fn command(
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(mut command): Json<WorldCommand>,
) -> Response {
    if !authorized(&headers, &state).await {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    if let Err(error) = process_due_ticks(&state).await {
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
            let campaign = match load_campaign(&state.store) {
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
                    let _ = state.store.insert(
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
    match state.kernel.command(command).await {
        Ok(result) => {
            if should_react {
                if let ghostlight_dungeon::kernel::CommandResult::Committed { campaign, .. } =
                    &result
                {
                    if campaign.actors.len() > 1 {
                        if let Some(model) = &state.model {
                            let summary = campaign
                                .transcript
                                .last()
                                .map(|turn| turn.text.clone())
                                .unwrap_or_else(|| "A consequential event occurred.".into());
                            let engine = PersonaProjectionEngine {
                                model: model.clone(),
                                permit: Arc::new(SnapshotPermit::new(
                                    state.store.clone(),
                                    campaign.id,
                                    campaign.revision,
                                )),
                                projector_model: "deepseek-v4-flash".into(),
                                persona_model: "deepseek-v4-pro".into(),
                                interpreter_model: "deepseek-v4-flash".into(),
                            };
                            match appraise_present(engine, campaign, &summary).await {
                                Ok(wave) if !wave.reactions.is_empty() => {
                                    for receipt in wave.receipts {
                                        let _ = state.store.insert(
                                            "persona_stage_receipt.v1",
                                            "ghostlight.persona_stage_receipt.v1",
                                            &receipt.output_hash,
                                            &receipt,
                                        );
                                    }
                                    match state.kernel.command(WorldCommand::ResolveReactionWave{expected_revision:campaign.revision,event_summary:summary,reactions:wave.reactions}).await{Ok(reaction)=>return Json(serde_json::json!({"primary":result,"reaction_wave":reaction})).into_response(),Err(error)=>return(StatusCode::CONFLICT,Json(ErrorBody{error:error.to_string()})).into_response()}
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
                    }
                }
            }
            Json(result).into_response()
        }
        Err(error) => (
            StatusCode::CONFLICT,
            Json(ErrorBody {
                error: error.to_string(),
            }),
        )
            .into_response(),
    }
}

async fn scheduler_loop(state: AppState) {
    let mut pulse = tokio::time::interval(std::time::Duration::from_secs(300));
    pulse.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    pulse.tick().await;
    loop {
        pulse.tick().await;
        if let Err(error) = process_due_ticks(&state).await {
            tracing::warn!(%error,"strategic scheduler pulse refused");
        }
    }
}

async fn process_due_ticks(state: &AppState) -> anyhow::Result<()> {
    loop {
        let campaign = match load_campaign(&state.store) {
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
        state
            .kernel
            .command(WorldCommand::AdvanceStrategicTick {
                expected_revision: campaign.revision,
                source: ghostlight_dungeon::domain::TickSource::ReturnCatchUp,
            })
            .await?;
    }
}

async fn authorized(headers: &HeaderMap, state: &AppState) -> bool {
    let session = headers
        .get(header::COOKIE)
        .and_then(|v| v.to_str().ok())
        .and_then(|cookies| {
            cookies
                .split(';')
                .map(str::trim)
                .find_map(|cookie| cookie.strip_prefix("ghostlight_session="))
        });
    match session {
        Some(id) => state
            .auth
            .lock()
            .await
            .state
            .session_hashes
            .contains(&secret_hash(id)),
        None => false,
    }
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

#[derive(Serialize)]
struct ErrorBody {
    error: String,
}

#[allow(dead_code)]
fn seed_campaign() -> Campaign {
    let id = uuid::Uuid::new_v4();
    let location = ghostlight_dungeon::domain::Location {
        id: "starfire-lab".into(),
        name: "Starfire Narrative Laboratory".into(),
        container_id: None,
        routes: BTreeMap::new(),
        persistent_features: vec![
            "A stable room waiting for an approved world compilation.".into(),
        ],
    };
    let actor = ghostlight_dungeon::domain::ActorState {
        id: "player".into(),
        name: "Tester".into(),
        location_id: location.id.clone(),
        capabilities: BTreeSet::new(),
        knowledge: BTreeSet::new(),
        equipment: BTreeSet::new(),
        conditions: BTreeSet::new(),
        obligations: BTreeSet::new(),
        relationships: BTreeMap::new(),
        goals: vec!["Compile a world worth entering.".into()],
        memories: vec![],
    };
    Campaign {
        schema: "ghostlight.campaign.v1".into(),
        id,
        name: "GhostlightDungeon MVP".into(),
        revision: 0,
        branch_origin: ghostlight_dungeon::domain::BranchOrigin {
            canon_cutoff: "fixture-development".into(),
            evidence_receipt_ids: vec![],
        },
        world_time: chrono::Utc::now(),
        tick_hours: 6,
        player_actor_id: actor.id.clone(),
        locations: BTreeMap::from([(location.id.clone(), location)]),
        actors: BTreeMap::from([(actor.id.clone(), actor)]),
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
