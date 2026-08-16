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
    domain::{Campaign, WorldCommand},
    model::{DeepSeekPort, ModelStageRequest, run_validated_stage},
    persistence::CampaignStore,
    surface::player_surface,
};
use serde::Serialize;
use std::{
    collections::{BTreeMap, BTreeSet},
    ffi::OsString,
    net::SocketAddr,
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};
use tokio::sync::Mutex;
use tower_http::services::ServeDir;
use tracing_subscriber::EnvFilter;

#[derive(Clone)]
struct AppState {
    kernel: WorldKernel,
    store: CampaignStore,
    invites: Arc<Mutex<BTreeSet<String>>>,
    sessions: Arc<Mutex<BTreeSet<String>>>,
    deepseek_status: String,
}

fn main() -> anyhow::Result<()> {
    #[cfg(windows)]
    if std::env::args().any(|argument| argument == "--service") {
        windows_service_host::run()?;
        return Ok(());
    }
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?
        .block_on(run_daemon(None))
}

async fn run_daemon(shutdown: Option<Arc<AtomicBool>>) -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();
    let runtime_root = std::env::var_os("GHOSTLIGHT_DUNGEON_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(r"F:\GameCult\GhostlightDungeon"));
    std::fs::create_dir_all(runtime_root.join("campaigns/default"))?;
    let store = CampaignStore::open(runtime_root.join("campaigns/default/campaign.cc"))?;
    let kernel = WorldKernel::start(store.clone());
    if store.keys("campaign.v1")?.is_empty() {
        kernel
            .command(WorldCommand::CreateCampaign {
                campaign: seed_campaign(),
            })
            .await?;
    }
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
    let secret_path = runtime_root.join("secrets/deepseek.dpapi");
    let deepseek_status = if secret_path.is_file() {
        let provider = DeepSeekPort::from_machine_dpapi(&secret_path)?;
        let probe = run_validated_stage(
            &provider,
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
        format!("ready:{}", probe.receipt.output_hash)
    } else {
        "missing-secret".into()
    };
    let state = AppState {
        kernel,
        store,
        invites: Arc::new(Mutex::new(invite_tokens)),
        sessions: Arc::new(Mutex::new(BTreeSet::new())),
        deepseek_status,
    };
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
        .route("/api/command", post(command))
        .fallback_service(ServeDir::new(web_root).append_index_html_on_directories(true))
        .with_state(state);
    let address: SocketAddr = "0.0.0.0:8831".parse()?;
    let listener = tokio::net::TcpListener::bind(address).await?;
    tracing::info!(%address, "GhostlightDungeon listening");
    let server = axum::serve(listener, app);
    if let Some(shutdown) = shutdown {
        server
            .with_graceful_shutdown(async move {
                while !shutdown.load(Ordering::Acquire) {
                    tokio::time::sleep(Duration::from_millis(200)).await;
                }
            })
            .await?;
    } else {
        server.await?;
    }
    Ok(())
}

#[cfg(windows)]
mod windows_service_host {
    use super::*;
    use windows_service::{
        define_windows_service,
        service::{
            ServiceControl, ServiceControlAccept, ServiceExitCode, ServiceState, ServiceStatus,
            ServiceType,
        },
        service_control_handler::{self, ServiceControlHandlerResult},
        service_dispatcher,
    };

    const NAME: &str = "GhostlightDungeon";
    const TYPE: ServiceType = ServiceType::OWN_PROCESS;

    pub fn run() -> windows_service::Result<()> {
        service_dispatcher::start(NAME, ffi_service_main)
    }
    define_windows_service!(ffi_service_main, service_main);

    fn service_main(_arguments: Vec<OsString>) {
        let _ = run_service();
    }

    fn run_service() -> windows_service::Result<()> {
        let shutdown = Arc::new(AtomicBool::new(false));
        let handler_flag = shutdown.clone();
        let handler = move |event| match event {
            ServiceControl::Stop => {
                handler_flag.store(true, Ordering::Release);
                ServiceControlHandlerResult::NoError
            }
            ServiceControl::Interrogate => ServiceControlHandlerResult::NoError,
            _ => ServiceControlHandlerResult::NotImplemented,
        };
        let status = service_control_handler::register(NAME, handler)?;
        status.set_service_status(ServiceStatus {
            service_type: TYPE,
            current_state: ServiceState::StartPending,
            controls_accepted: ServiceControlAccept::empty(),
            exit_code: ServiceExitCode::Win32(0),
            checkpoint: 1,
            wait_hint: Duration::from_secs(30),
            process_id: None,
        })?;
        status.set_service_status(ServiceStatus {
            service_type: TYPE,
            current_state: ServiceState::Running,
            controls_accepted: ServiceControlAccept::STOP,
            exit_code: ServiceExitCode::Win32(0),
            checkpoint: 0,
            wait_hint: Duration::default(),
            process_id: None,
        })?;
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .map_err(|_| windows_service::Error::Winapi(std::io::Error::last_os_error()))?;
        let result = runtime.block_on(run_daemon(Some(shutdown)));
        status.set_service_status(ServiceStatus {
            service_type: TYPE,
            current_state: ServiceState::Stopped,
            controls_accepted: ServiceControlAccept::empty(),
            exit_code: if result.is_ok() {
                ServiceExitCode::Win32(0)
            } else {
                ServiceExitCode::Win32(1)
            },
            checkpoint: 0,
            wait_hint: Duration::default(),
            process_id: None,
        })?;
        Ok(())
    }
}

async fn health(State(state): State<AppState>) -> Json<serde_json::Value> {
    Json(
        serde_json::json!({"schema":"ghostlight.service_health.v1","status":"ok","storeIdentity":state.store.identity(),"deepseek":state.deepseek_status}),
    )
}

async fn invite(Path(token): Path<String>, State(state): State<AppState>) -> Response {
    if !state.invites.lock().await.remove(&token) {
        return (StatusCode::UNAUTHORIZED, "invalid or consumed invite").into_response();
    }
    let session = uuid::Uuid::new_v4().to_string();
    state.sessions.lock().await.insert(session.clone());
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
        Err(error) => (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()).into_response(),
    }
}

async fn command(
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(command): Json<WorldCommand>,
) -> Response {
    if !authorized(&headers, &state).await {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    match state.kernel.command(command).await {
        Ok(result) => Json(result).into_response(),
        Err(error) => (
            StatusCode::CONFLICT,
            Json(ErrorBody {
                error: error.to_string(),
            }),
        )
            .into_response(),
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
        Some(id) => state.sessions.lock().await.contains(id),
        None => false,
    }
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
    }
}
