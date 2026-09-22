//! One-process runtime for the sealed world owner.

use crate::{
    app_session::AppSessionOwner,
    eve::{self, EveCommandInvocation},
    heimdall::{self, HeimdallClient},
    idunn_health::{
        IDUNN_RUNTIME_CANDIDATE_BIND_ENVIRONMENT, ProcessWriteLeaseGuard, RuntimePresencePublisher,
        TARGET as GHOSTLIGHT_TARGET,
    },
    mesh::{self, MeshPublisher, MeshRuntimeIdentity},
    play::{PlayRequest, PlayTable, QuestionId},
};
use ghostlight::{
    AffordanceId, CONSUMER_BODY_LIMIT, CommandBody, CommandId, ConnectorBinding, ConsumerPort,
    ConsumerRegistry, ControllerModels, ControllerPort, ControllerRunner, ControllerWorkCustody,
    CreateJurisdictionIntent, CreateWorldIntent, DEFAULT_LOCAL_MODEL_PREFIX,
    DEFAULT_SDK_MODEL_PREFIX, DecisionInvocation, DecisionOpportunity, KernelError, Lens,
    LensWeights, LocalBinding, MailboxError, PersonaLane, PrincipalCommandIntent, PrincipalId,
    SdkBinding, SeedOutcome, SeedPort, Statement, SubjectKind, SubmitReceipt, TickMinutes,
    VaultEvidenceSource, VerifiedPrincipalEvidence, WorldMailbox, WorldPhase, WorldSnapshot,
    open_controller_work, open_inference,
};
use anyhow::{Context, bail, ensure};
use axum::{
    Json, Router,
    body::Bytes,
    extract::{ConnectInfo, DefaultBodyLimit, Path, State},
    http::{HeaderMap, HeaderValue, StatusCode, header},
    response::{
        IntoResponse, Response,
        sse::{Event as SseEvent, KeepAlive, Sse},
    },
    routing::{get, post},
};
use chrono::{DateTime, Utc};
use cultnet_rs::{
    CultNetMessage, CultNetWireContract, GAMECULT_RUNTIME_PRESENCE_HEALTH_SCHEMA,
    decode_cultnet_message_from_slice, encode_cultnet_message_to_vec,
};
use serde::Deserialize;
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    convert::Infallible,
    fs,
    net::SocketAddr,
    path::PathBuf,
    sync::Arc,
    time::Duration,
};
use tokio::sync::{Mutex, Semaphore, broadcast, mpsc};
use tower_http::services::ServeDir;
use tracing_subscriber::EnvFilter;

const COOKIE_NAME: &str = "ghostlight_session";
const CULTNET_SNAPSHOT_BODY_LIMIT: usize = 16 * 1024;

#[derive(Clone)]
struct RuntimeHealthOwner {
    publisher: Arc<Mutex<RuntimePresencePublisher>>,
    write_lease: Arc<ProcessWriteLeaseGuard>,
}

#[derive(Clone)]
struct AppState {
    world: WorldMailbox,
    /// The consumer ingress's narrow view of the same owner, and the configured
    /// consumers it authenticates against. The registry is read once at startup
    /// and holds only digests; a missing file means no consumers.
    consumer: ConsumerPort,
    consumers: Arc<ConsumerRegistry>,
    /// The one cognition organ, shared. The owner's seeding and the readiness
    /// custody probe use it; nothing in this process runs it on its own.
    controllers: Option<Arc<ControllerRunner>>,
    /// The play table (Cut 8b): one operational model running a role-playing
    /// game over this process's own world, minted alongside `controllers`
    /// from the same opened inference organ. `None` exactly when
    /// `controllers` is `None` — the play lane has no cognition of its own to
    /// fall back to.
    play: Option<Arc<PlayTable>>,
    /// The controller/Persona concurrency pool, sized to the connector's
    /// quota. `execute_dispatch` and `close_turn`'s own narration (Cut 8b)
    /// draw permits from this exact pool, so `runtime_readiness`'s
    /// `controllerStatus` "active" arm — unreachable in production before
    /// this cut, since nothing drew from the pool — now reflects live
    /// Persona cognition, not only the test route's own forced exhaustion.
    controller_permits: Arc<Semaphore>,
    sessions: Arc<Mutex<AppSessionOwner>>,
    heimdall: Arc<HeimdallClient>,
    mesh: Option<MeshPublisher>,
    mesh_identity: MeshRuntimeIdentity,
    runtime_health: Option<RuntimeHealthOwner>,
    revisions: broadcast::Sender<u64>,
    fatal: mpsc::UnboundedSender<String>,
}

struct ProductionAdmission {
    health: RuntimePresencePublisher,
    write_lease: ProcessWriteLeaseGuard,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CreatePayload {
    title: String,
    /// The world's premise in the owner's words, projected to every lane as
    /// guidance. Required, may be empty; a payload that omits it is refused,
    /// which is what `world_create.v4` means.
    brief: String,
    subject_label: String,
    #[serde(default)]
    narrative_persona_label: Option<String>,
    #[serde(default)]
    operational_agent_label: Option<String>,
    /// World-wide target of goal-bearing subjects per kind. Required, and may
    /// be empty: a world with no target is a deliberate choice, not a default
    /// that arrives because nobody said anything. A payload that omits it is
    /// refused, which is what `world_create.v4` means.
    targets: BTreeMap<SubjectKind, u32>,
    /// The jurisdiction roots, declared by genesis beside the commons because
    /// `resolve_patch` only resolves roots the same patch declares. A duplicate
    /// handle and a permille sum over 1000 are refused by the resolver, not
    /// pre-checked here: a pre-check would be a second reducer.
    jurisdictions: Vec<CreateJurisdiction>,
    /// Required, and must draw: a payload that omits it is refused, which is
    /// what `world_create.v4` means. An unknown lens name is refused by
    /// deserialization before any handler runs.
    lens_weights: LensWeights,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CreateJurisdiction {
    /// The draft handle genesis declares the root under, and the intent's key.
    handle: String,
    label: String,
    permille: u32,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SeedPayload {
    /// A subdirectory of the configured vault root, or empty for the whole
    /// vault. Relative and `..`-free, refused at `VaultEvidenceSource::open`.
    /// The root itself is configuration: a payload carrying an absolute path
    /// would turn an authenticated Eve button into a read primitive over the
    /// server's filesystem.
    #[serde(default)]
    vault_scope: String,
    /// One sentence of the owner's own intent, carried into the brief verbatim
    /// and into the Vault query as a referent.
    #[serde(default)]
    brief: Option<String>,
}

/// The shape a jurisdiction handle may take. Checked at ingress because the
/// handle becomes a draft-index key the model reads back in mismatches, and a
/// handle carrying whitespace or case would make that text unusable.
fn is_handle_shape(value: &str) -> bool {
    let mut chars = value.chars();
    value.len() <= 48
        && chars.next().is_some_and(|first| first.is_ascii_lowercase())
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-')
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SpeakPayload {
    text: String,
    opportunity: DecisionOpportunity,
    affordance_id: AffordanceId,
}

/// `world.play`'s own payload (Cut 8b): the player's prose, plus `answers`
/// naming the question this request answers (PA.f84) — `None` for the plain
/// opening/continue shape. Follows `ghostlight.world_speak.v0`'s own field
/// pattern above.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct PlayPayload {
    text: String,
    #[serde(default)]
    answers: Option<QuestionId>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct AdvanceTimePayload {
    minutes: u32,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct EmptyPayload {}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CompleteAuthPayload {
    handle: String,
}

pub(crate) async fn run(state_root_binding: Option<PathBuf>) -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();
    let runtime_root = admitted_runtime_root(state_root_binding)?;
    let service_root = runtime_root.join("service");
    // The bound socket is service observation, not configuration. Retain it
    // unopened to application traffic while Warming waits for Idunn's lease.
    let requested_address = candidate_bind()?;
    let listener = tokio::net::TcpListener::bind(requested_address).await?;
    let bound_address = listener.local_addr()?;
    let (fatal, mut fatal_events) = mpsc::unbounded_channel();
    let production_admission = initialize_production_admission(bound_address).await?;
    let dependency_bindings = production_admission
        .as_ref()
        .map(|admission| admission.health.dependency_bindings().clone());
    let runtime_id = match &production_admission {
        Some(admission) => admission.health.runtime_id().to_owned(),
        None => std::env::var("GHOSTLIGHT_RUNTIME_ID")
            .context("GHOSTLIGHT_RUNTIME_ID is required outside managed runtime")?,
    };
    if runtime_id.is_empty() || runtime_id.trim() != runtime_id {
        bail!("GHOSTLIGHT_RUNTIME_ID must be a canonical non-empty identity");
    }
    let mesh_identity = MeshRuntimeIdentity {
        runtime_id: runtime_id.clone(),
        service_id: std::env::var("GHOSTLIGHT_SERVICE_ID")
            .unwrap_or_else(|_| "ghostlight-dungeon".into()),
        located_service: std::env::var("GHOSTLIGHT_LOCATED_SERVICE")
            .unwrap_or_else(|_| "local".into()),
    };

    // Production may bind but not serve its candidate socket, inspect immutable
    // root activation inputs and its provider identity, then publish Warming.
    // World, session, controller, replay, and CultMesh state remain unopened
    // until Idunn grants the lease bound to that exact observed process.
    let mut write_lease_guard = None;
    let mut idunn_health = match production_admission {
        Some(admission) => {
            let ProductionAdmission {
                health,
                write_lease,
            } = admission;
            write_lease.require_current()?;
            write_lease_guard = Some(write_lease);
            Some(health)
        }
        None => None,
    };

    tokio::task::yield_now().await;
    require_no_runtime_custody_failure(&mut fatal_events)?;
    require_current_write_lease(write_lease_guard.as_ref())?;
    if write_lease_guard.is_some() {
        prepare_admitted_state_layout(&runtime_root)?;
    } else {
        fs::create_dir_all(&service_root)?;
    }
    require_current_write_lease(write_lease_guard.as_ref())?;
    let (world, world_owner) = WorldMailbox::open(runtime_root.join("world.cc"))?;
    require_no_runtime_custody_failure(&mut fatal_events)?;
    let connector_endpoint = dependency_bindings
        .as_ref()
        .map(|bindings| bindings.connector)
        .map(Ok)
        .unwrap_or_else(configured_connector_endpoint)?;
    let controller_permits = Arc::new(Semaphore::new(configured_controller_concurrency()));
    let (controllers, play) =
        match open_controller(&world, &service_root, &runtime_id, connector_endpoint) {
            Ok(OpenedControllers {
                runner,
                inference,
                models,
                play_model,
            }) => {
                let play = match open_play(
                    &world,
                    &service_root,
                    inference,
                    &models,
                    play_model,
                    controller_permits.clone(),
                ) {
                    Ok(table) => Some(Arc::new(table)),
                    Err(error) => {
                        tracing::warn!(%error, "the play table is unavailable; world authority remains online");
                        None
                    }
                };
                (Some(runner), play)
            }
            Err(error) => {
                tracing::warn!(%error, "controller cognition is unavailable; world authority remains online");
                (None, None)
            }
        };
    require_no_runtime_custody_failure(&mut fatal_events)?;
    let wrapping_key = std::env::var_os("GHOSTLIGHT_SESSION_WRAPPING_KEY_FILE")
        .map(PathBuf::from)
        .unwrap_or_else(|| runtime_root.join("secrets/session-wrapping.key"));
    let sessions = AppSessionOwner::open(service_root.join("app-sessions-v2.cc"), wrapping_key)?;
    let odin_endpoint = dependency_bindings
        .as_ref()
        .map(|bindings| bindings.odin_rudp)
        .map(Ok)
        .unwrap_or_else(configured_odin_endpoint)?;
    let expected_heimdall = dependency_bindings
        .as_ref()
        .map(|bindings| bindings.heimdall.clone());
    let heimdall = Arc::new(HeimdallClient::from_env(
        &runtime_id,
        odin_endpoint,
        expected_heimdall,
    )?);
    require_no_runtime_custody_failure(&mut fatal_events)?;
    let mesh = match open_mesh(&service_root, &mesh_identity, Some(odin_endpoint)) {
        Ok(mesh) => Some(mesh),
        Err(error) => {
            tracing::warn!(%error, "derived CultMesh projection is unavailable; world authority remains online");
            None
        }
    };
    require_no_runtime_custody_failure(&mut fatal_events)?;
    let (revisions, _) = broadcast::channel(32);
    let consumers = Arc::new(open_consumer_registry()?);
    let mut state = AppState {
        consumer: ConsumerPort::new(world.clone()),
        consumers,
        world,
        controllers: controllers.map(Arc::new),
        play,
        controller_permits,
        sessions: Arc::new(Mutex::new(sessions)),
        heimdall,
        mesh,
        mesh_identity: mesh_identity.clone(),
        revisions,
        fatal,
        runtime_health: None,
    };
    require_no_runtime_custody_failure(&mut fatal_events)?;
    publish_projection(&state).await?;

    let release_web_root = std::env::current_exe()?
        .parent()
        .map(|parent| parent.join("web"));
    let web_root = release_web_root
        .filter(|path| path.join("index.html").is_file())
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../web/dist"));
    require_no_runtime_custody_failure(&mut fatal_events)?;
    require_current_write_lease(write_lease_guard.as_ref())?;
    canonical_readiness(&state)
        .await
        .context("runtime is not ready for active signed health")?;
    if let Some(publisher) = idunn_health.as_mut() {
        let write_lease = write_lease_guard
            .as_ref()
            .context("production runtime lost its process write lease")?;
        publisher
            .publish_active(write_lease)
            .context("publishing initial active runtime presence")?;
    }
    require_current_write_lease(write_lease_guard.as_ref())?;
    state.runtime_health = match (idunn_health, write_lease_guard) {
        (Some(publisher), Some(write_lease)) => Some(RuntimeHealthOwner {
            publisher: Arc::new(Mutex::new(publisher)),
            write_lease: Arc::new(write_lease),
        }),
        (None, None) => None,
        _ => bail!("managed runtime health authority is partial"),
    };
    tokio::spawn(maintain_mesh_projection(state.clone()));
    let app = app_router(state.clone(), web_root);
    tracing::info!(address = %bound_address, "Ghostlight Dungeon world owner serving");
    let server = axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    );
    tokio::select! {
        result = server => result.context("Ghostlight HTTP server stopped"),
        result = world_owner => {
            result.context("world-owner task panicked")?;
            bail!("world-owner task stopped while the daemon was serving")
        }
        result = maintain_runtime_health(state.clone()) => {
            result?;
            bail!("runtime readiness owner stopped while the daemon was serving")
        }
        result = maintain_session_refresh(state.clone()) => {
            result?;
            bail!("app-session refresh owner stopped while the daemon was serving")
        }
        detail = fatal_events.recv() => {
            bail!(
                "runtime custody failed: {}",
                detail.unwrap_or_else(|| "fatal signal channel closed".into())
            )
        }
    }
}

/// What `open_controller` opens: the controller organ itself, plus the same
/// opened inference organ and model set the play table (Cut 8b) shares
/// rather than opening its own second connection to the same transports.
struct OpenedControllers {
    runner: ControllerRunner,
    inference: Arc<dyn ghostlight::InferencePort>,
    models: ControllerModels,
    play_model: String,
}

fn open_controller(
    world: &WorldMailbox,
    service_root: &std::path::Path,
    runtime_id: &str,
    endpoint: SocketAddr,
) -> anyhow::Result<OpenedControllers> {
    let models = ControllerModels {
        projector: std::env::var("GHOSTLIGHT_CONTROLLER_PROJECTOR_MODEL")
            .unwrap_or_else(|_| "gpt-5.6-luna".into()),
        persona: std::env::var("GHOSTLIGHT_CONTROLLER_PERSONA_MODEL")
            .unwrap_or_else(|_| "gpt-5.6-sol".into()),
        interpreter: std::env::var("GHOSTLIGHT_CONTROLLER_INTERPRETER_MODEL")
            .unwrap_or_else(|_| "gpt-5.6-terra".into()),
        operational_agent: std::env::var("GHOSTLIGHT_CONTROLLER_OPERATIONAL_MODEL")
            .unwrap_or_else(|_| "gpt-5.6-terra".into()),
        elaborator: std::env::var("GHOSTLIGHT_CONTROLLER_ELABORATOR_MODEL")
            .unwrap_or_else(|_| "gpt-5.6-terra".into()),
    };
    // The play table's own model (Cut 8b), read beside its four siblings and
    // handed into the same `open_inference` call below — exactly as the
    // sibling models are handled — so the one opened organ validates and
    // routes it too, rather than a second `open_inference` connecting to the
    // same transports again for the play lane alone.
    let play_model =
        std::env::var("GHOSTLIGHT_PLAY_MODEL").unwrap_or_else(|_| "gpt-5.6-terra".into());
    let connector = std::env::var_os("GHOSTLIGHT_CONTROLLER_CREDENTIAL")
        .map(PathBuf::from)
        .map(|key_path| ConnectorBinding {
            endpoint,
            key_path,
            caller_runtime_id: runtime_id.to_owned(),
        });
    // Ghostlight reads no credential for this transport. The sidecar inherits
    // the ambient Claude Code login; this process never names, copies, or logs
    // it.
    let sdk = std::env::var_os("GHOSTLIGHT_SDK_SIDECAR")
        .map(PathBuf::from)
        .map(|sidecar_entry| SdkBinding {
            sidecar_entry,
            caller_runtime_id: runtime_id.to_owned(),
            model_prefix: std::env::var("GHOSTLIGHT_SDK_MODEL_PREFIX")
                .unwrap_or_else(|_| DEFAULT_SDK_MODEL_PREFIX.into()),
        });
    // Ghostlight reads no credential for this transport either: a loopback
    // local model server needs none, and `open_inference` refuses anything
    // else at open.
    let local = match std::env::var("GHOSTLIGHT_LOCAL_ENDPOINT") {
        Ok(value) => Some(LocalBinding {
            endpoint: value.parse().with_context(|| {
                format!("GHOSTLIGHT_LOCAL_ENDPOINT `{value}` is not a socket address")
            })?,
            model_prefix: std::env::var("GHOSTLIGHT_LOCAL_MODEL_PREFIX")
                .unwrap_or_else(|_| DEFAULT_LOCAL_MODEL_PREFIX.into()),
            caller_runtime_id: runtime_id.to_owned(),
        }),
        Err(_) => None,
    };
    let mut all_models: Vec<&str> = models.each().to_vec();
    all_models.push(&play_model);
    let inference = open_inference(connector, sdk, local, &all_models)?;
    let work = open_controller_work(service_root.join("controller-work.cc"))?;
    let runner = ControllerRunner::open(world.clone(), inference.clone(), work, models.clone())?;
    Ok(OpenedControllers {
        runner,
        inference,
        models,
        play_model,
    })
}

/// Opens the play table (Cut 8b) around the same inference organ and world
/// owner `open_controller` already opened, plus its own store row and
/// concurrency pool. `permits` is the exact `Arc<Semaphore>` `AppState` hands
/// out as `controller_permits`: `execute_dispatch` and `close_turn`'s own
/// narration draw from it, so the one pool `runtime_readiness` reads is the
/// one play cognition actually contends for (PA.f13, P8.5).
fn open_play(
    world: &WorldMailbox,
    service_root: &std::path::Path,
    inference: Arc<dyn ghostlight::InferencePort>,
    models: &ControllerModels,
    play_model: String,
    permits: Arc<Semaphore>,
) -> anyhow::Result<PlayTable> {
    let personas = PersonaLane::new(
        ControllerPort::new(world.clone()),
        inference.clone(),
        models.projector.clone(),
        models.persona.clone(),
    )?;
    PlayTable::new(
        world.clone(),
        personas,
        inference,
        play_model,
        permits,
        service_root.join("play-turn-v1.cc"),
    )
}

fn open_mesh(
    service_root: &std::path::Path,
    identity: &MeshRuntimeIdentity,
    target: Option<SocketAddr>,
) -> anyhow::Result<MeshPublisher> {
    MeshPublisher::open(service_root.join("mesh-v2.cc"), target, identity.clone())
}

fn app_router(state: AppState, web_root: PathBuf) -> Router {
    api_router(state)
        .fallback_service(ServeDir::new(web_root).append_index_html_on_directories(true))
}

fn api_router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route(
            "/cultnet/snapshot",
            post(cultnet_snapshot).layer(DefaultBodyLimit::max(CULTNET_SNAPSHOT_BODY_LIMIT)),
        )
        .route(
            "/cultnet/world-patch",
            post(cultnet_world_patch).layer(DefaultBodyLimit::max(CONSUMER_BODY_LIMIT)),
        )
        .route("/api/eve/provider", get(eve_provider))
        .route("/api/eve/surfaces/{surface_id}", get(eve_surface))
        .route("/api/eve/commands", post(eve_command))
        .route("/api/eve/events", get(revision_events))
        .with_state(state)
}

/// The consumer ingress's door. Loopback and content type are the two gates
/// `/cultnet/snapshot` already established; everything past them belongs to
/// `ghostlight`'s `consumer`, which owns decode, bounds, authentication, and the one
/// receipt. This handler holds no opinion about a patch.
async fn cultnet_world_patch(
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    if !peer.ip().is_loopback() {
        return StatusCode::FORBIDDEN.into_response();
    }
    if headers.get(header::CONTENT_TYPE) != Some(&HeaderValue::from_static("application/msgpack")) {
        return StatusCode::UNSUPPORTED_MEDIA_TYPE.into_response();
    }
    let receipt = ghostlight::admit_document(&state.consumer, &state.consumers, &body).await;
    match ghostlight::encode_receipt(&receipt) {
        Ok(bytes) => (
            StatusCode::OK,
            [(header::CONTENT_TYPE, "application/msgpack")],
            bytes,
        )
            .into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

/// Read once at startup. A missing or unset path means no configured
/// consumers, which is the fail-closed default; a malformed file fails
/// startup outright, so a mistyped credential cannot silently read as "no
/// consumers configured".
fn open_consumer_registry() -> anyhow::Result<ConsumerRegistry> {
    let Ok(path) = std::env::var(ghostlight::CONSUMER_CREDENTIALS_ENVIRONMENT) else {
        return Ok(ConsumerRegistry::empty());
    };
    ConsumerRegistry::from_secret_file(&path)
        .map_err(|error| anyhow::anyhow!("consumer credentials at {path} are unreadable: {error}"))
}

async fn cultnet_snapshot(
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    if !peer.ip().is_loopback() {
        return StatusCode::FORBIDDEN.into_response();
    }
    if headers.get(header::CONTENT_TYPE) != Some(&HeaderValue::from_static("application/msgpack")) {
        return StatusCode::UNSUPPORTED_MEDIA_TYPE.into_response();
    }
    let message_id = match exact_route_observation_message_id(&body) {
        Ok(message_id) => message_id,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };
    let Some(owner) = state.runtime_health.clone() else {
        return StatusCode::SERVICE_UNAVAILABLE.into_response();
    };

    // Hold the one publisher across readiness observation and signing so a
    // periodic publication cannot age the checked state before this challenge
    // is bound into the signed record.
    let write_lease = owner.write_lease.clone();
    let mut publisher = owner.publisher.clone().lock_owned().await;
    let ready = canonical_readiness(&state).await.is_ok();
    let response = tokio::task::spawn_blocking(move || {
        let response = publisher.route_observation(&message_id, ready, &write_lease)?;
        encode_cultnet_message_to_vec(&response, CultNetWireContract::CultNetSchemaV0)
    })
    .await;
    let Ok(Ok(response)) = response else {
        return StatusCode::SERVICE_UNAVAILABLE.into_response();
    };
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "application/msgpack")],
        response,
    )
        .into_response()
}

fn exact_route_observation_message_id(body: &[u8]) -> anyhow::Result<String> {
    let request = decode_cultnet_message_from_slice(body, CultNetWireContract::CultNetSchemaV0)
        .context("decoding route-observation request")?;
    ensure!(
        encode_cultnet_message_to_vec(&request, CultNetWireContract::CultNetSchemaV0)? == body,
        "route-observation request is not canonical MessagePack"
    );
    let CultNetMessage::SnapshotRequest {
        message_id,
        schema_ids,
        record_keys,
    } = request
    else {
        bail!("route-observation request is not a snapshot request");
    };
    ensure!(
        !message_id.is_empty(),
        "route-observation message id is empty"
    );
    ensure!(
        matches!(
            schema_ids.as_deref(),
            Some([schema_id]) if schema_id == GAMECULT_RUNTIME_PRESENCE_HEALTH_SCHEMA
        ) && matches!(
            record_keys.as_deref(),
            Some([record_key]) if record_key == GHOSTLIGHT_TARGET
        ),
        "route-observation request is not the exact Ghostlight presence record"
    );
    let detail = format!("route-observation:{message_id}");
    ensure!(
        detail.len() <= 512 && !detail.chars().any(char::is_control),
        "route-observation message id cannot be signed canonically"
    );
    Ok(message_id)
}

async fn health(State(state): State<AppState>) -> Response {
    match runtime_readiness(&state).await {
        Ok(health) => Json(health).into_response(),
        Err(error) => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({"status":"failed","message":error.to_string()})),
        )
            .into_response(),
    }
}

async fn eve_provider(State(state): State<AppState>) -> Response {
    Json(mesh::provider_advertisement(
        &state.mesh_identity,
        &Utc::now().to_rfc3339(),
    ))
    .into_response()
}

async fn eve_surface(
    Path(surface_id): Path<String>,
    headers: HeaderMap,
    State(state): State<AppState>,
) -> Response {
    if surface_id != mesh::SURFACE_ID {
        return StatusCode::NOT_FOUND.into_response();
    }
    let Some(principal) = authenticated_principal(&headers, &state).await else {
        return Json(eve::anonymous_surface()).into_response();
    };
    match current_operator_view(&state)
        .await
        .and_then(|(snapshot, log)| {
            eve::authenticated_surface(
                principal.account_subject_hash(),
                snapshot.as_ref(),
                &log,
            )
        }) {
        Ok(surface) => Json(surface).into_response(),
        Err(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"message":error.to_string()})),
        )
            .into_response(),
    }
}

async fn eve_command(
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(invocation): Json<EveCommandInvocation>,
) -> Response {
    if let Err(error) = eve::validate_invocation(&invocation, "https-json") {
        return Json(eve::command_result(
            &invocation,
            "denied",
            error.to_string(),
            None,
            None,
            None,
        ))
        .into_response();
    }
    match invocation.operation.operation_id.as_str() {
        "heimdall.auth.begin" => begin_authentication(&state, invocation).await,
        "heimdall.auth.complete" => complete_authentication(&state, invocation).await,
        "app.auth.logout" => logout(&headers, &state, invocation).await,
        _ => {
            let Some(principal) = authenticated_principal(&headers, &state).await else {
                return Json(eve::command_result(
                    &invocation,
                    "denied",
                    "Authentication is required.",
                    None,
                    None,
                    None,
                ))
                .into_response();
            };
            dispatch_world(&state, &principal, invocation).await
        }
    }
}

async fn begin_authentication(state: &AppState, invocation: EveCommandInvocation) -> Response {
    if let Err(error) = serde_json::from_value::<EmptyPayload>(invocation.payload.clone()) {
        return Json(eve::command_result(
            &invocation,
            "denied",
            format!("invalid command payload: {error}"),
            None,
            None,
            None,
        ))
        .into_response();
    }
    let idempotency = invocation
        .operation
        .idempotency_key
        .as_deref()
        .unwrap_or("");
    match state.heimdall.begin(idempotency).await {
        Ok(receipt)
            if receipt.status == "pending"
                && !receipt.handle.is_empty()
                && receipt
                    .expires_at
                    .parse::<DateTime<Utc>>()
                    .is_ok_and(|expiry| expiry > Utc::now()) =>
        {
            Json(eve::command_result(
                &invocation,
                "accepted",
                "Continue authentication with Heimdall.",
                None,
                Some(json!({
                    "pluginId":"gamecult.heimdall.access",
                    "schemaId":"heimdall.auth_navigation_receipt.v1",
                    "payload":{
                        "schema":"heimdall.auth_navigation_receipt.v1",
                        "handle":receipt.handle,
                        "navigation":{
                            "url":receipt.navigation.url,
                            "allowedOrigins":receipt.navigation.allowed_origins
                        }
                    }
                })),
                None,
            ))
            .into_response()
        }
        Ok(_) => Json(eve::command_result(
            &invocation,
            "denied",
            "Heimdall returned an invalid authentication attempt.",
            None,
            None,
            None,
        ))
        .into_response(),
        Err(error) => Json(eve::command_result(
            &invocation,
            "denied",
            error.to_string(),
            None,
            None,
            None,
        ))
        .into_response(),
    }
}

async fn complete_authentication(state: &AppState, invocation: EveCommandInvocation) -> Response {
    let payload = match serde_json::from_value::<CompleteAuthPayload>(invocation.payload.clone()) {
        Ok(payload) => payload,
        Err(error) => {
            return Json(eve::command_result(
                &invocation,
                "denied",
                format!("invalid command payload: {error}"),
                None,
                None,
                None,
            ))
            .into_response();
        }
    };
    let handle = payload.handle.as_str();
    if handle.is_empty() {
        return Json(eve::command_result(
            &invocation,
            "denied",
            "Authentication completion omitted its opaque handle.",
            None,
            None,
            None,
        ))
        .into_response();
    }
    let idempotency = invocation
        .operation
        .idempotency_key
        .as_deref()
        .unwrap_or("");
    let completion = match state.heimdall.complete(handle, idempotency).await {
        Ok(value) => value,
        Err(error) => {
            return Json(eve::command_result(
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
        return Json(auth_completion_result(
            &invocation,
            "pending",
            "pending",
            "Heimdall is waiting for Discord.",
        ))
        .into_response();
    }
    if completion.status != "authenticated" || completion.handle.as_deref() != Some(handle) {
        let message = completion
            .error
            .clone()
            .unwrap_or_else(|| "Heimdall denied access.".into());
        return Json(auth_completion_result(
            &invocation,
            "denied",
            "denied",
            &message,
        ))
        .into_response();
    }
    let adopted = match adopt_heimdall_completion(state, completion).await {
        Ok(value) => value,
        Err(error) => {
            return Json(auth_completion_result(
                &invocation,
                "denied",
                "denied",
                &error.to_string(),
            ))
            .into_response();
        }
    };
    let mut response = Json(auth_completion_result(
        &invocation,
        "accepted",
        "authenticated",
        "Authenticated.",
    ))
    .into_response();
    response.headers_mut().insert(
        header::SET_COOKIE,
        session_cookie(&adopted.token, adopted.expires_at, Utc::now()),
    );
    response
}

struct AdoptedSession {
    token: String,
    expires_at: DateTime<Utc>,
}

async fn adopt_heimdall_completion(
    state: &AppState,
    completion: heimdall::AuthCompletionReceipt,
) -> anyhow::Result<AdoptedSession> {
    let admission = state.heimdall.verify_completion(completion).await?;
    let expires_at = admission.refresh_expires_at();
    let token = {
        let mut sessions = state.sessions.lock().await;
        let result = sessions.create_session(admission);
        if result.is_err() && !sessions.is_healthy() {
            signal_fatal(
                state,
                "app-session authentication adoption",
                result.as_ref().unwrap_err(),
            );
        }
        result?
    };
    Ok(AdoptedSession { token, expires_at })
}

fn auth_completion_result(
    invocation: &EveCommandInvocation,
    command_state: &str,
    auth_state: &str,
    message: &str,
) -> Value {
    eve::command_result(
        invocation,
        command_state,
        message,
        None,
        Some(json!({
            "pluginId":"gamecult.heimdall.access",
            "schemaId":"heimdall.auth_completion_status.v1",
            "payload":{"schema":"heimdall.auth_completion_status.v1","status":auth_state}
        })),
        None,
    )
}

async fn logout(
    headers: &HeaderMap,
    state: &AppState,
    invocation: EveCommandInvocation,
) -> Response {
    if let Err(error) = serde_json::from_value::<EmptyPayload>(invocation.payload.clone()) {
        return Json(eve::command_result(
            &invocation,
            "denied",
            format!("invalid command payload: {error}"),
            None,
            None,
            None,
        ))
        .into_response();
    }
    let raw_cookie = cookie_value(headers).unwrap_or_default();
    let local_logout = {
        let mut sessions = state.sessions.lock().await;
        match sessions.session_for_logout(raw_cookie) {
            Ok(remote) => match sessions.revoke_cookie(raw_cookie) {
                Ok(true) => Ok(remote),
                Ok(false) => Ok(remote),
                Err(error) => Err(error),
            },
            Err(error) => Err(error),
        }
    };
    let remote = match local_logout {
        Ok(remote) => remote,
        Err(error) => {
            signal_fatal(state, "app-session logout", &error);
            return Json(auth_completion_result(
                &invocation,
                "unknown",
                "unknown",
                "Local session revocation was not confirmed.",
            ))
            .into_response();
        }
    };
    if let Some(session) = remote {
        let heimdall = state.heimdall.clone();
        tokio::spawn(async move {
            let idempotency = format!(
                "logout:{}:{}",
                session.heimdall_session_id, session.access_revision
            );
            if let Err(error) = heimdall.logout(&session.refresh_claim, &idempotency).await {
                tracing::warn!(%error, "local logout succeeded but Heimdall logout was unavailable");
            }
        });
    }
    let mut response = Json(auth_completion_result(
        &invocation,
        "accepted",
        "anonymous",
        "Signed out.",
    ))
    .into_response();
    response.headers_mut().insert(
        header::SET_COOKIE,
        HeaderValue::from_static(
            "ghostlight_session=; Max-Age=0; HttpOnly; Secure; SameSite=Lax; Path=/ghostlight/",
        ),
    );
    response
}

async fn dispatch_world(
    state: &AppState,
    principal: &VerifiedPrincipalEvidence,
    invocation: EveCommandInvocation,
) -> Response {
    let result = execute_world(state, principal, &invocation).await;
    match result {
        Ok(receipt) => {
            let version = match publish_projection(state).await {
                Ok(version) => Some(version),
                Err(error) => {
                    tracing::error!(%error, "world revision could not be read after commit");
                    current_world(state)
                        .await
                        .ok()
                        .map(|snapshot| eve::surface_version(snapshot.as_ref()))
                }
            };
            Json(eve::command_result(
                &invocation,
                "accepted",
                "World owner accepted the command.",
                version,
                None,
                Some(receipt),
            ))
            .into_response()
        }
        Err(error) => {
            let state_name = if matches!(error, RuntimeCommandError::OutcomeUnknown(_)) {
                "unknown"
            } else {
                "denied"
            };
            Json(eve::command_result(
                &invocation,
                state_name,
                error.to_string(),
                current_world(state)
                    .await
                    .ok()
                    .map(|snapshot| eve::surface_version(snapshot.as_ref())),
                None,
                None,
            ))
            .into_response()
        }
    }
}

#[derive(Debug, thiserror::Error)]
enum RuntimeCommandError {
    #[error("invalid command payload: {0}")]
    Payload(String),
    #[error("world command outcome is unknown after durable submission: {0}")]
    OutcomeUnknown(String),
    #[error(transparent)]
    Mailbox(#[from] MailboxError),
    #[error(transparent)]
    Kernel(#[from] KernelError),
}

async fn execute_world(
    state: &AppState,
    verified_principal: &VerifiedPrincipalEvidence,
    invocation: &EveCommandInvocation,
) -> Result<Value, RuntimeCommandError> {
    let command_id = CommandId::parse_uuid(
        invocation
            .operation
            .idempotency_key
            .as_deref()
            .unwrap_or(""),
    )?;
    if invocation.operation.operation_id == "world.create" {
        let payload: CreatePayload = serde_json::from_value(invocation.payload.clone())
            .map_err(|error| RuntimeCommandError::Payload(error.to_string()))?;
        if let Some(root) = payload
            .jurisdictions
            .iter()
            .find(|root| !is_handle_shape(&root.handle))
        {
            return Err(RuntimeCommandError::Payload(format!(
                "jurisdiction handle {} is not a draft handle",
                root.handle
            )));
        }
        let receipt = state
            .world
            .create(
                CreateWorldIntent {
                    id: command_id,
                    title: payload.title,
                    brief: payload.brief,
                    human_subject_label: payload.subject_label,
                    narrative_persona_label: payload.narrative_persona_label,
                    operational_agent_label: payload.operational_agent_label,
                    targets: payload.targets,
                    jurisdictions: payload
                        .jurisdictions
                        .into_iter()
                        .map(|root| CreateJurisdictionIntent {
                            handle: root.handle,
                            label: root.label,
                            permille: root.permille,
                        })
                        .collect(),
                    lens_weights: payload.lens_weights,
                },
                verified_principal,
            )
            .await
            .map_err(map_mailbox)?;
        return Ok(json!({
            "kind":"created",
            "commandId":serde_json::to_value(receipt.command_id).unwrap_or(Value::Null),
            "worldId":serde_json::to_value(receipt.world_id).unwrap_or(Value::Null),
            "stateDigest":receipt.resulting_state_digest,
            "commitDigest":receipt.commit_digest
        }));
    }
    if invocation.operation.operation_id == "world.play" {
        // A play turn is not one kernel command: `PlayTable::run` submits as
        // many as its own round loop decides, and a round can spend a real
        // inference budget before any of them commit. Routed to the table in
        // a spawned task, exactly as the map calls for, rather than held
        // open behind this request — the caller polls the world/story
        // surface for what the turn actually did, the same way it already
        // observes any other committed consequence.
        let payload: PlayPayload = serde_json::from_value(invocation.payload.clone())
            .map_err(|error| RuntimeCommandError::Payload(error.to_string()))?;
        let table = state
            .play
            .clone()
            .ok_or_else(|| RuntimeCommandError::Payload("the play table is unavailable".into()))?;
        let key = invocation
            .operation
            .idempotency_key
            .clone()
            .unwrap_or_default();
        let principal = verified_principal.clone();
        tokio::spawn(async move {
            if let Err(error) = table
                .run(
                    &principal,
                    key,
                    PlayRequest {
                        text: payload.text,
                        answers: payload.answers,
                    },
                )
                .await
            {
                tracing::warn!(%error, "a play turn ended in error");
            }
        });
        return Ok(json!({"kind":"accepted"}));
    }

    let snapshot = current_world(state)
        .await
        .map_err(|error| RuntimeCommandError::Payload(error.to_string()))?
        .ok_or_else(|| RuntimeCommandError::Payload("world has not been created".into()))?;
    let source_version = invocation
        .operation
        .route_hint
        .source_version
        .ok_or_else(|| RuntimeCommandError::Payload("source version is required".into()))?;
    let expected_revision = source_version.checked_sub(1).ok_or_else(|| {
        RuntimeCommandError::Payload("source version does not name a world revision".into())
    })?;
    let body = match invocation.operation.operation_id.as_str() {
        "world.approve" => {
            serde_json::from_value::<EmptyPayload>(invocation.payload.clone())
                .map_err(|error| RuntimeCommandError::Payload(error.to_string()))?;
            CommandBody::ApproveDraft
        }
        "world.activate" => {
            serde_json::from_value::<EmptyPayload>(invocation.payload.clone())
                .map_err(|error| RuntimeCommandError::Payload(error.to_string()))?;
            CommandBody::ActivateWorld
        }
        "world.speak" => {
            let payload: SpeakPayload = serde_json::from_value(invocation.payload.clone())
                .map_err(|error| RuntimeCommandError::Payload(error.to_string()))?;
            // `world.speak` stays an ingress for the Speak entry specifically.
            // A generic `world.act` is only useful beside per-affordance Eve
            // controls derived from the catalog, and that is a projection pass.
            CommandBody::ExerciseDecision {
                opportunity: payload.opportunity,
                invocation: DecisionInvocation {
                    affordance: payload.affordance_id,
                    bindings: Vec::new(),
                    proposed: Vec::new(),
                    speech: Some(Statement::new(payload.text).ok_or_else(|| {
                        RuntimeCommandError::Payload("spoken text is not canonical".into())
                    })?),
                    display: None,
                },
            }
        }
        "world.advance_time" => {
            let payload: AdvanceTimePayload = serde_json::from_value(invocation.payload.clone())
                .map_err(|error| RuntimeCommandError::Payload(error.to_string()))?;
            CommandBody::AdvanceTime {
                minutes: TickMinutes::new(payload.minutes).ok_or_else(|| {
                    RuntimeCommandError::Payload("time span is outside one year of minutes".into())
                })?,
            }
        }
        // The one arm that builds no `CommandBody`: the seed runner submits
        // through its own port, as the owner, and the receipt reports what one
        // session did rather than what one command committed.
        "world.seed" => {
            let payload: SeedPayload = serde_json::from_value(invocation.payload.clone())
                .map_err(|error| RuntimeCommandError::Payload(error.to_string()))?;
            let outcome = seed_once(state, verified_principal, &snapshot, payload).await?;
            let after = current_world(state)
                .await
                .map_err(|error| RuntimeCommandError::Payload(error.to_string()))?;
            return Ok(json!({
                "kind":"seeded",
                "outcome":outcome.name(),
                "revision":after.as_ref().map(|world| world.revision),
                "deficit":after
                    .as_ref()
                    .map(|world| world.scale_deficit.iter().map(|row| u64::from(row.deficit)).sum::<u64>()),
            }));
        }
        operation => {
            return Err(RuntimeCommandError::Payload(format!(
                "unknown world operation {operation}"
            )));
        }
    };
    let receipt = state
        .world
        .submit_principal(
            PrincipalCommandIntent {
                id: command_id,
                world_id: snapshot.world_id,
                expected_revision,
                body,
            },
            verified_principal,
        )
        .await
        .map_err(map_mailbox)?;
    Ok(submit_receipt(receipt))
}

fn submit_receipt(receipt: SubmitReceipt) -> Value {
    match receipt {
        SubmitReceipt::Applied(receipt) => json!({
            "kind":"applied",
            "commandId":serde_json::to_value(receipt.command_id).unwrap_or(Value::Null),
            "revision":receipt.resulting_revision,
            "stateDigest":receipt.resulting_state_digest,
            "commitDigest":receipt.commit_digest
        }),
        SubmitReceipt::AlreadyApplied(receipt) => json!({
            "kind":"already_applied",
            "commandId":serde_json::to_value(receipt.command_id).unwrap_or(Value::Null),
            "revision":receipt.resulting_revision,
            "stateDigest":receipt.resulting_state_digest,
            "commitDigest":receipt.commit_digest
        }),
    }
}

fn map_mailbox(error: MailboxError) -> RuntimeCommandError {
    match error {
        MailboxError::OutcomeUnknown { command_id } => {
            RuntimeCommandError::OutcomeUnknown(format!("{command_id:?}"))
        }
        other => RuntimeCommandError::Mailbox(other),
    }
}

async fn current_world(state: &AppState) -> anyhow::Result<Option<WorldSnapshot>> {
    match state.world.snapshot().await {
        Ok(snapshot) => Ok(Some(snapshot)),
        Err(MailboxError::Kernel(KernelError::WorldNotCreated)) => Ok(None),
        Err(error) => Err(error.into()),
    }
}

/// The two halves of the operator surface, fetched together: the projection of
/// world state, and the story feed the human reads. The feed is deliberately not
/// a snapshot field, so no controller lane can reach it.
async fn current_operator_view(
    state: &AppState,
) -> anyhow::Result<(Option<WorldSnapshot>, Vec<ghostlight::OperatorEvent>)> {
    let snapshot = current_world(state).await?;
    let log = match state.world.operator_log().await {
        Ok(log) => log,
        Err(MailboxError::Kernel(KernelError::WorldNotCreated)) => Vec::new(),
        Err(error) => return Err(error.into()),
    };
    Ok((snapshot, log))
}

async fn publish_projection(state: &AppState) -> anyhow::Result<u64> {
    let snapshot = current_world(state).await?;
    let version = eve::surface_version(snapshot.as_ref());
    let _ = state.revisions.send(version);
    if let Some(mesh) = state.mesh.clone() {
        let _projection = tokio::task::spawn_blocking(move || {
            if let Err(error) = mesh.publish(snapshot.as_ref()) {
                tracing::warn!(%error, "derived CultMesh projection update failed");
            }
        });
    }
    Ok(version)
}

async fn refresh_mesh_projection(state: &AppState) -> anyhow::Result<u64> {
    let snapshot = current_world(state).await?;
    let version = eve::surface_version(snapshot.as_ref());
    let Some(mesh) = state.mesh.clone() else {
        bail!("CultMesh projection is unavailable");
    };
    tokio::task::spawn_blocking(move || mesh.publish(snapshot.as_ref()))
        .await
        .context("CultMesh projection worker panicked")??;
    Ok(version)
}

async fn canonical_readiness(state: &AppState) -> anyhow::Result<Value> {
    let snapshot = current_world(state)
        .await
        .context("world owner is not ready")?;
    state
        .sessions
        .lock()
        .await
        .validate_custody()
        .context("app-session owner is not ready")?;
    let expected_version = eve::surface_version(snapshot.as_ref());
    let expected_world_state = eve::world_state(snapshot.as_ref());
    Ok(json!({
        "schema":"ghostlight.service_health.v2",
        "status":"ok",
        "worldState":expected_world_state,
        "surfaceVersion":expected_version,
        "updatedAtUtc":Utc::now().to_rfc3339(),
        "runtime":state.mesh_identity.runtime_id,
        "commit":option_env!("GHOSTLIGHT_BUILD_COMMIT").unwrap_or("development"),
    }))
}

async fn runtime_readiness(state: &AppState) -> anyhow::Result<Value> {
    let mut health = canonical_readiness(state).await?;
    let expected_version = health["surfaceVersion"].as_u64().unwrap_or_default();
    let expected_world_state = health["worldState"].as_str().unwrap_or("unknown");
    let projection_status = match &state.mesh {
        Some(mesh) => match mesh.health() {
            Ok(health)
                if health.get("status").and_then(Value::as_str) == Some("ok")
                    && health.get("surfaceVersion").and_then(Value::as_u64)
                        == Some(expected_version)
                    && health.get("worldState").and_then(Value::as_str)
                        == Some(expected_world_state) =>
            {
                "ok"
            }
            Ok(_) | Err(_) => "degraded",
        },
        None => "unavailable",
    };
    let controller_status = match state.controllers.as_deref() {
        Some(controller) => {
            match tokio::time::timeout(Duration::from_millis(100), controller.custody_probe()).await
            {
                Ok(Ok(ControllerWorkCustody::Owned { .. })) => {
                    if state.controller_permits.available_permits() == 0 {
                        "active"
                    } else {
                        "ok"
                    }
                }
                Ok(Ok(ControllerWorkCustody::Uncertain { .. })) | Ok(Err(_)) => "degraded",
                Err(_) => "busy",
            }
        }
        None => "unavailable",
    };
    health["projectionStatus"] = Value::String(projection_status.into());
    health["controllerStatus"] = Value::String(controller_status.into());
    Ok(health)
}

async fn maintain_mesh_projection(state: AppState) {
    let mut interval = tokio::time::interval(Duration::from_secs(120));
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    // Startup publishes once before serving. Do not wake clients with a duplicate
    // revision event; this loop only renews derived CultMesh freshness.
    interval.tick().await;
    loop {
        interval.tick().await;
        if let Err(error) = refresh_mesh_projection(&state).await {
            tracing::warn!(%error, "derived CultMesh projection remains degraded");
        }
    }
}

/// Where the seed lane's read-only markdown vault lives. Configuration, read at
/// open exactly as every other runtime path is: the payload names a relative
/// scope inside it and never the root.
const SEED_VAULT_ROOT_ENVIRONMENT: &str = "GHOSTLIGHT_SEED_VAULT_ROOT";

/// One seeding session, inside the request that asked for it. There is no
/// background task and no stop channel: a spawned sweep would need the owner's
/// verified evidence to outlive the request that carried it, and then two new
/// authorities in `AppState` to be stoppable. The checkpoint discipline already
/// makes a long request safe — a transport timeout loses the response, not the
/// work — so the owner's repetition is the sweep and not pressing the button
/// again is the stop.
///
/// Every refusal here happens before anything is spent. The reducer refuses a
/// non-owner too, at `require_patch_author`, and the runner refuses an Active
/// world again at its own phase gate; this is the gate that keeps a paid
/// session from running first.
async fn seed_once(
    state: &AppState,
    verified_principal: &VerifiedPrincipalEvidence,
    snapshot: &WorldSnapshot,
    payload: SeedPayload,
) -> Result<SeedOutcome, RuntimeCommandError> {
    if snapshot.owner != PrincipalId::new(verified_principal.account_subject_hash()) {
        return Err(RuntimeCommandError::Payload(
            "seeding a world is its owner's lane".into(),
        ));
    }
    if snapshot.phase != WorldPhase::Draft {
        return Ok(SeedOutcome::NotDraft);
    }
    let Some(controllers) = state.controllers.as_deref() else {
        return Err(RuntimeCommandError::Payload(
            "the cognition organ is closed".into(),
        ));
    };
    let root = std::env::var(SEED_VAULT_ROOT_ENVIRONMENT).map_err(|_| {
        RuntimeCommandError::Payload(format!("{SEED_VAULT_ROOT_ENVIRONMENT} is not configured"))
    })?;
    let vault = VaultEvidenceSource::open(std::path::Path::new(&root), &payload.vault_scope)
        .map_err(|error| RuntimeCommandError::Payload(error.to_string()))?;
    let runner = controllers.seeder(
        SeedPort::new(state.world.clone(), verified_principal.clone()),
        Arc::new(vault),
        payload.brief,
    );
    runner
        .sweep(1)
        .await
        .map_err(|error| RuntimeCommandError::Payload(error.to_string()))
}

/// Dungeon policy for a world created before Session Zero's sliders exist
/// (D2). Not a library default; the library requires the weights to be stated.
pub(crate) fn uniform_lens_weights() -> LensWeights {
    LensWeights::new(Lens::ALL.into_iter().map(|lens| (lens, 1)).collect())
}

/// Sized to the connector's per-caller quota. `Capacity` and `InFlight`
/// refusals are already retryable faults, so a pool above that quota degrades
/// to retry rather than to corruption.
fn configured_controller_concurrency() -> usize {
    std::env::var("GHOSTLIGHT_CONTROLLER_MAX_CONCURRENT")
        .ok()
        .and_then(|value| value.trim().parse::<usize>().ok())
        .filter(|value| *value >= 1)
        .unwrap_or(4)
}

async fn revision_events(State(state): State<AppState>) -> impl IntoResponse {
    let mut revisions = state.revisions.subscribe();
    let stream = async_stream::stream! {
        loop {
            match revisions.recv().await {
                Ok(version) => yield Ok::<SseEvent, Infallible>(SseEvent::default().event("revision").data(version.to_string())),
                Err(broadcast::error::RecvError::Lagged(_)) => continue,
                Err(broadcast::error::RecvError::Closed) => break,
            }
        }
    };
    Sse::new(stream).keep_alive(KeepAlive::default())
}

async fn authenticated_principal(
    headers: &HeaderMap,
    state: &AppState,
) -> Option<VerifiedPrincipalEvidence> {
    let raw = cookie_value(headers)?;
    match state
        .sessions
        .lock()
        .await
        .account_for_cookie(raw, Utc::now())
    {
        Ok(account) => account,
        Err(error) => {
            signal_fatal(state, "app-session custody", &error);
            None
        }
    }
}

fn signal_fatal(state: &AppState, organ: &str, error: &impl std::fmt::Display) {
    let detail = format!("{organ}: {error}");
    tracing::error!(%detail, "runtime owner can no longer uphold its invariant");
    let _ = state.fatal.send(detail);
}

fn cookie_value(headers: &HeaderMap) -> Option<&str> {
    headers
        .get(header::COOKIE)?
        .to_str()
        .ok()?
        .split(';')
        .map(str::trim)
        .find_map(|cookie| cookie.strip_prefix(&format!("{COOKIE_NAME}=")))
}

fn session_cookie(raw: &str, expires_at: DateTime<Utc>, now: DateTime<Utc>) -> HeaderValue {
    HeaderValue::from_str(&format!(
        "{COOKIE_NAME}={raw}; Max-Age={}; HttpOnly; Secure; SameSite=Lax; Path=/ghostlight/",
        (expires_at - now).num_seconds().max(0)
    ))
    .expect("generated session cookie must be valid")
}

async fn maintain_session_refresh(state: AppState) -> anyhow::Result<()> {
    let mut interval = tokio::time::interval(Duration::from_secs(60));
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    loop {
        interval.tick().await;
        let candidates = state
            .sessions
            .lock()
            .await
            .sessions_due_for_refresh(Utc::now(), chrono::Duration::minutes(2))
            .context("app-session refresh scan lost custody")?;
        for candidate in candidates {
            let idempotency = format!(
                "refresh:{}:{}",
                candidate.heimdall_session_id, candidate.access_revision
            );
            let completion = match state
                .heimdall
                .refresh(&candidate.refresh_claim, &idempotency)
                .await
            {
                Ok(value) => value,
                Err(error) => {
                    tracing::warn!(%error, "Heimdall refresh transport unavailable");
                    continue;
                }
            };
            let verified = match state.heimdall.verify_refresh(completion).await {
                Ok(value) => value,
                Err(error) => {
                    tracing::warn!(%error, "Heimdall refresh receipt was invalid");
                    let mut sessions = state.sessions.lock().await;
                    if let Err(revoke_error) = sessions.revoke_cookie_hash(&candidate.cookie_hash) {
                        return Err(revoke_error)
                            .context("invalid Heimdall refresh could not revoke local custody");
                    }
                    continue;
                }
            };
            if verified.heimdall_session_id() != candidate.heimdall_session_id
                || crate::app_session::secret_hash(&format!(
                    "heimdall-account:{}",
                    verified.account_id()
                )) != candidate.account_subject_hash
            {
                tracing::warn!("Heimdall refresh changed local session custody");
                let mut sessions = state.sessions.lock().await;
                if let Err(revoke_error) = sessions.revoke_cookie_hash(&candidate.cookie_hash) {
                    return Err(revoke_error)
                        .context("changed Heimdall custody could not revoke local session");
                }
                continue;
            }
            let mut sessions = state.sessions.lock().await;
            if let Err(error) =
                sessions.apply_refresh(&candidate.cookie_hash, candidate.access_revision, verified)
            {
                if sessions.is_healthy() {
                    tracing::warn!(%error, "local app session rejected Heimdall refresh");
                } else {
                    return Err(error).context("app-session refresh commit lost custody");
                }
            }
        }
    }
}

async fn initialize_production_admission(
    bound_endpoint: SocketAddr,
) -> anyhow::Result<Option<ProductionAdmission>> {
    let Some(mut publisher) = RuntimePresencePublisher::from_environment(bound_endpoint)? else {
        return Ok(None);
    };
    let warming = publisher
        .publish_warming()
        .context("publishing initial Warming runtime presence")?;
    let write_lease = publisher
        .wait_for_write_lease(&warming, Duration::from_secs(120))
        .await
        .with_context(|| {
            format!(
                "waiting for process lease bound to Warming {}",
                warming.canonical_sha256()
            )
        })?;
    Ok(Some(ProductionAdmission {
        health: publisher,
        write_lease,
    }))
}

async fn maintain_runtime_health(state: AppState) -> anyhow::Result<()> {
    let mut interval = tokio::time::interval(Duration::from_secs(10));
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    loop {
        interval.tick().await;
        canonical_readiness(&state)
            .await
            .context("runtime is not ready for signed health")?;
        let Some(owner) = state.runtime_health.clone() else {
            continue;
        };
        owner
            .write_lease
            .require_current()
            .context("runtime process write lease is not current")?;
        let mut publisher = owner.publisher.clone().lock_owned().await;
        let published = tokio::task::spawn_blocking(move || {
            publisher.republish_active()?;
            Ok::<(), anyhow::Error>(())
        })
        .await;
        let Ok(result) = published else {
            tracing::warn!("runtime-presence worker panicked; publication will retry");
            continue;
        };
        if let Err(error) = result {
            tracing::warn!(%error, "signed runtime-presence publication failed");
        }
    }
}

fn candidate_bind() -> anyhow::Result<SocketAddr> {
    match std::env::var(IDUNN_RUNTIME_CANDIDATE_BIND_ENVIRONMENT) {
        Ok(value) => value
            .parse()
            .context("Idunn candidate bind is not a socket address"),
        Err(_) => {
            #[cfg(target_os = "linux")]
            bail!("{IDUNN_RUNTIME_CANDIDATE_BIND_ENVIRONMENT} is mandatory on Linux");
            #[cfg(not(target_os = "linux"))]
            return Ok("127.0.0.1:8831".parse()?);
        }
    }
}

fn configured_connector_endpoint() -> anyhow::Result<SocketAddr> {
    std::env::var("GHOSTLIGHT_CONTROLLER_CONNECTOR")
        .unwrap_or_else(|_| "127.0.0.1:4103".into())
        .parse()
        .context("GHOSTLIGHT_CONTROLLER_CONNECTOR is not a socket address")
}

fn configured_odin_endpoint() -> anyhow::Result<SocketAddr> {
    std::env::var("GHOSTLIGHT_ODIN_RUDP")
        .context("GHOSTLIGHT_ODIN_RUDP is required")?
        .parse()
        .context("GHOSTLIGHT_ODIN_RUDP is not a socket address")
}

fn require_current_write_lease(lease: Option<&ProcessWriteLeaseGuard>) -> anyhow::Result<()> {
    match lease {
        Some(lease) => lease.require_current(),
        None => Ok(()),
    }
}

fn require_no_runtime_custody_failure(
    failures: &mut mpsc::UnboundedReceiver<String>,
) -> anyhow::Result<()> {
    match failures.try_recv() {
        Ok(detail) => bail!("runtime custody failed before serving: {detail}"),
        Err(mpsc::error::TryRecvError::Empty) => Ok(()),
        Err(mpsc::error::TryRecvError::Disconnected) => {
            bail!("runtime custody signal channel closed before serving")
        }
    }
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

fn admitted_runtime_root(binding: Option<PathBuf>) -> anyhow::Result<PathBuf> {
    let Some(binding) = binding else {
        #[cfg(target_os = "linux")]
        bail!("--state-root is mandatory for the managed Linux runtime");
        #[cfg(not(target_os = "linux"))]
        return Ok(default_runtime_root());
    };
    ensure!(binding.is_absolute(), "state-root binding is not absolute");
    let metadata = fs::symlink_metadata(&binding)
        .with_context(|| format!("inspecting state-root binding {}", binding.display()))?;
    ensure!(
        metadata.is_dir() && !metadata.file_type().is_symlink(),
        "state-root binding is not a direct directory"
    );
    let canonical = fs::canonicalize(&binding)
        .with_context(|| format!("canonicalizing state-root binding {}", binding.display()))?;
    ensure!(
        canonical == binding,
        "state-root binding is indirect or non-canonical"
    );
    Ok(binding)
}

fn prepare_admitted_state_layout(runtime_root: &std::path::Path) -> anyhow::Result<()> {
    let rebound = admitted_runtime_root(Some(runtime_root.to_owned()))?;
    ensure!(
        rebound == runtime_root,
        "state-root binding changed after process-write-lease admission"
    );
    let service_root = runtime_root.join("service");
    match fs::symlink_metadata(&service_root) {
        Ok(metadata) => ensure!(
            metadata.is_dir() && !metadata.file_type().is_symlink(),
            "service state directory is indirect or not a directory"
        ),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            fs::create_dir(&service_root).with_context(|| {
                format!(
                    "creating direct service state directory {}",
                    service_root.display()
                )
            })?;
        }
        Err(error) => return Err(error).context("inspecting service state directory"),
    }
    ensure_direct_state_directory(&service_root, "service state directory")?;
    for path in [
        runtime_root.join("world.cc"),
        service_root.join("app-sessions-v2.cc"),
        service_root.join("controller-work.cc"),
        service_root.join("mesh-v2.cc"),
    ] {
        require_direct_state_file_or_absent(&path)?;
        require_direct_state_file_or_absent(&sibling_state_lock_path(&path))?;
    }
    Ok(())
}

fn ensure_direct_state_directory(path: &std::path::Path, label: &str) -> anyhow::Result<()> {
    let metadata = fs::symlink_metadata(path)
        .with_context(|| format!("inspecting {label} {}", path.display()))?;
    ensure!(
        metadata.is_dir() && !metadata.file_type().is_symlink(),
        "{label} is indirect or not a directory"
    );
    Ok(())
}

fn require_direct_state_file_or_absent(path: &std::path::Path) -> anyhow::Result<()> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => {
            ensure!(
                metadata.is_file() && !metadata.file_type().is_symlink(),
                "declared state path {} is indirect or not a regular file",
                path.display()
            );
            #[cfg(target_os = "linux")]
            ensure!(
                std::os::unix::fs::MetadataExt::nlink(&metadata) == 1,
                "declared state path {} is multiply linked",
                path.display()
            );
            Ok(())
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => {
            Err(error).with_context(|| format!("inspecting declared state path {}", path.display()))
        }
    }
}

fn sibling_state_lock_path(path: &std::path::Path) -> PathBuf {
    let mut name = path
        .file_name()
        .map(|value| value.to_os_string())
        .unwrap_or_else(|| "state.cc".into());
    name.push(".lock");
    path.with_file_name(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Small enough that a test can hold every permit and prove a route does
    /// not cross the provider boundary.
    const TEST_CONTROLLER_CONCURRENCY: usize = 2;
    use crate::idunn_health::tests::route_observation_fixture;
    use axum::{
        body::{Body, to_bytes},
        http::Request,
    };
    use cultnet_rs::{
        GameCultRuntimePresenceHealthRecord, RuntimePresenceAuthenticationContext,
        authenticate_runtime_presence_claim,
    };
    use tower::ServiceExt;

    #[cfg(target_os = "linux")]
    #[test]
    fn managed_linux_runtime_requires_the_admitted_state_root_binding() {
        assert!(admitted_runtime_root(None).is_err());
    }

    #[test]
    fn admitted_state_root_must_name_the_direct_canonical_directory() {
        let directory = tempfile::tempdir().unwrap();
        let root = fs::canonicalize(directory.path()).unwrap();
        assert_eq!(admitted_runtime_root(Some(root.clone())).unwrap(), root);
        assert!(admitted_runtime_root(Some(directory.path().join("missing"))).is_err());
    }

    #[test]
    fn post_lease_state_layout_creates_only_the_direct_declared_directory() {
        let directory = tempfile::tempdir().unwrap();
        let root = fs::canonicalize(directory.path()).unwrap();
        prepare_admitted_state_layout(&root).unwrap();
        let service = fs::symlink_metadata(root.join("service")).unwrap();
        assert!(service.is_dir());
        assert!(!service.file_type().is_symlink());
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn post_lease_state_layout_rejects_symlinks_and_hardlinks() {
        use std::os::unix::fs::symlink;

        let symlink_directory = tempfile::tempdir().unwrap();
        let symlink_root = fs::canonicalize(symlink_directory.path()).unwrap();
        let target = symlink_root.join("other.cc");
        fs::write(&target, b"not-world").unwrap();
        symlink(&target, symlink_root.join("world.cc")).unwrap();
        assert!(prepare_admitted_state_layout(&symlink_root).is_err());

        let hardlink_directory = tempfile::tempdir().unwrap();
        let hardlink_root = fs::canonicalize(hardlink_directory.path()).unwrap();
        let target = hardlink_root.join("other.cc");
        fs::write(&target, b"not-world").unwrap();
        fs::hard_link(&target, hardlink_root.join("world.cc")).unwrap();
        assert!(prepare_admitted_state_layout(&hardlink_root).is_err());
    }

    struct Fixture {
        _directory: tempfile::TempDir,
        state: AppState,
        cookie: String,
    }

    async fn fixture() -> Fixture {
        let directory = tempfile::tempdir().unwrap();
        let key = directory.path().join("session.key");
        std::fs::write(&key, [17_u8; 32]).unwrap();
        let mut sessions =
            AppSessionOwner::open(directory.path().join("sessions.cc"), &key).unwrap();
        let cookie = sessions
            .create_session(heimdall::VerifiedSessionAdmission::fixture(
                "operator-account",
                "heimdall-session",
                1,
                Utc::now() + chrono::Duration::hours(1),
                Utc::now() + chrono::Duration::days(1),
                "fixture-refresh",
            ))
            .unwrap();
        let (world, _owner) = WorldMailbox::open(directory.path().join("world.cc")).unwrap();
        let controller_key = directory.path().join("controller.key");
        std::fs::write(&controller_key, "runtime-test-controller-key").unwrap();
        let models = ControllerModels {
            projector: "gpt-5.6-luna".into(),
            persona: "gpt-5.6-sol".into(),
            interpreter: "gpt-5.6-terra".into(),
            operational_agent: "gpt-5.6-terra".into(),
            elaborator: "gpt-5.6-terra".into(),
        };
        let connector = ConnectorBinding {
            endpoint: "127.0.0.1:9".parse().unwrap(),
            key_path: controller_key,
            caller_runtime_id: "ghostlight-runtime-test".into(),
        };
        let inference = open_inference(Some(connector), None, None, &models.each()).unwrap();
        let work = open_controller_work(directory.path().join("controller-work.cc")).unwrap();
        let controllers =
            ControllerRunner::open(world.clone(), inference.clone(), work, models.clone()).unwrap();
        let personas = PersonaLane::new(
            ControllerPort::new(world.clone()),
            inference.clone(),
            models.projector.clone(),
            models.persona.clone(),
        )
        .unwrap();
        let play = PlayTable::new(
            world.clone(),
            personas,
            inference,
            models.operational_agent.clone(),
            Arc::new(Semaphore::new(TEST_CONTROLLER_CONCURRENCY)),
            directory.path().join("play-turn-v1.cc"),
        )
        .unwrap();
        let mesh = MeshPublisher::open(
            directory.path().join("mesh.cc"),
            None,
            MeshRuntimeIdentity::default(),
        )
        .unwrap();
        let mesh_identity = MeshRuntimeIdentity::default();
        let (revisions, _) = broadcast::channel(8);
        let (fatal, _fatal_events) = mpsc::unbounded_channel();
        let state = AppState {
            consumer: ConsumerPort::new(world.clone()),
            consumers: Arc::new(ConsumerRegistry::empty()),
            world,
            controllers: Some(Arc::new(controllers)),
            play: Some(Arc::new(play)),
            controller_permits: Arc::new(Semaphore::new(TEST_CONTROLLER_CONCURRENCY)),
            sessions: Arc::new(Mutex::new(sessions)),
            heimdall: Arc::new(HeimdallClient::fixture()),
            mesh: Some(mesh),
            mesh_identity,
            runtime_health: None,
            revisions,
            fatal,
        };
        publish_projection(&state).await.unwrap();
        Fixture {
            _directory: directory,
            state,
            cookie,
        }
    }

    fn route_snapshot_request(
        message_id: &str,
        schema_ids: Option<Vec<String>>,
        record_keys: Option<Vec<String>>,
    ) -> Vec<u8> {
        encode_cultnet_message_to_vec(
            &CultNetMessage::SnapshotRequest {
                message_id: message_id.into(),
                schema_ids,
                record_keys,
            },
            CultNetWireContract::CultNetSchemaV0,
        )
        .unwrap()
    }

    #[test]
    fn route_observation_request_is_one_exact_canonical_record() {
        let exact = route_snapshot_request(
            "route-challenge-41",
            Some(vec![GAMECULT_RUNTIME_PRESENCE_HEALTH_SCHEMA.into()]),
            Some(vec![GHOSTLIGHT_TARGET.into()]),
        );
        assert_eq!(
            exact_route_observation_message_id(&exact).unwrap(),
            "route-challenge-41"
        );

        for refused in [
            route_snapshot_request("broad", None, None),
            route_snapshot_request(
                "foreign-schema",
                Some(vec!["gamecult.other.v1".into()]),
                Some(vec![GHOSTLIGHT_TARGET.into()]),
            ),
            route_snapshot_request(
                "foreign-record",
                Some(vec![GAMECULT_RUNTIME_PRESENCE_HEALTH_SCHEMA.into()]),
                Some(vec!["other-target".into()]),
            ),
        ] {
            assert!(exact_route_observation_message_id(&refused).is_err());
        }
    }

    /// The consumer door's two transport gates, the same two
    /// `/cultnet/snapshot` established. Everything past them belongs to
    /// `ghostlight`'s `consumer`, which is tested there.
    #[tokio::test]
    async fn a_non_loopback_peer_is_forbidden_and_a_wrong_content_type_is_unsupported() {
        let fixture = fixture().await;
        let request = |peer: &str, content_type: &str| {
            Request::builder()
                .method("POST")
                .uri("/cultnet/world-patch")
                .extension(ConnectInfo(peer.parse::<SocketAddr>().unwrap()))
                .header(header::CONTENT_TYPE, content_type)
                .body(Body::from(Vec::new()))
                .unwrap()
        };

        let remote = api_router(fixture.state.clone())
            .oneshot(request("192.0.2.9:39001", "application/msgpack"))
            .await
            .unwrap();
        assert_eq!(remote.status(), StatusCode::FORBIDDEN);

        let wrong_media = api_router(fixture.state.clone())
            .oneshot(request("127.0.0.1:39001", "application/json"))
            .await
            .unwrap();
        assert_eq!(wrong_media.status(), StatusCode::UNSUPPORTED_MEDIA_TYPE);

        // A loopback peer with the right media type reaches the ingress, which
        // refuses an empty frame with a receipt rather than a status.
        let admitted = api_router(fixture.state.clone())
            .oneshot(request("127.0.0.1:39001", "application/msgpack"))
            .await
            .unwrap();
        assert_eq!(admitted.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn http_route_probe_bypasses_provider_and_enforces_exact_admission() {
        let mut fixture = fixture().await;
        let body = route_snapshot_request(
            "route-challenge-42",
            Some(vec![GAMECULT_RUNTIME_PRESENCE_HEALTH_SCHEMA.into()]),
            Some(vec![GHOSTLIGHT_TARGET.into()]),
        );
        let request = |peer: &str, content_type: Option<&str>, body: Vec<u8>| {
            let mut builder = Request::builder()
                .method("POST")
                .uri("/cultnet/snapshot")
                .extension(ConnectInfo(peer.parse::<SocketAddr>().unwrap()));
            if let Some(content_type) = content_type {
                builder = builder.header(header::CONTENT_TYPE, content_type);
            }
            builder.body(Body::from(body)).unwrap()
        };

        let unmanaged = api_router(fixture.state.clone())
            .oneshot(request(
                "127.0.0.1:39001",
                Some("application/msgpack"),
                body.clone(),
            ))
            .await
            .unwrap();
        assert_eq!(unmanaged.status(), StatusCode::SERVICE_UNAVAILABLE);

        let remote = api_router(fixture.state.clone())
            .oneshot(request(
                "192.0.2.9:39001",
                Some("application/msgpack"),
                body.clone(),
            ))
            .await
            .unwrap();
        assert_eq!(remote.status(), StatusCode::FORBIDDEN);

        let wrong_media = api_router(fixture.state.clone())
            .oneshot(request(
                "127.0.0.1:39001",
                Some("application/json"),
                body.clone(),
            ))
            .await
            .unwrap();
        assert_eq!(wrong_media.status(), StatusCode::UNSUPPORTED_MEDIA_TYPE);

        let broad = api_router(fixture.state.clone())
            .oneshot(request(
                "127.0.0.1:39001",
                Some("application/msgpack"),
                route_snapshot_request("broad", None, None),
            ))
            .await
            .unwrap();
        assert_eq!(broad.status(), StatusCode::BAD_REQUEST);

        let health = route_observation_fixture(fixture._directory.path()).unwrap();
        let authority = health.authority;
        let write_lease_sha256 = health.write_lease.canonical_sha256().to_owned();
        fixture.state.runtime_health = Some(RuntimeHealthOwner {
            publisher: Arc::new(Mutex::new(health.publisher)),
            write_lease: Arc::new(health.write_lease),
        });

        // If this route enters the controller/provider boundary, the exhausted
        // permit pool makes the request time out. Route admission and signing
        // need only the canonical health owner and managed runtime authority.
        let permits = fixture.state.controller_permits.clone();
        let _provider_execution_barrier = permits
            .acquire_many(u32::try_from(TEST_CONTROLLER_CONCURRENCY).expect("a small pool"))
            .await
            .expect("the controller permit pool is open");
        let response = tokio::time::timeout(
            Duration::from_secs(2),
            api_router(fixture.state.clone()).oneshot(request(
                "127.0.0.1:39001",
                Some("application/msgpack"),
                body,
            )),
        )
        .await
        .expect("route observation entered provider execution")
        .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers().get(header::CONTENT_TYPE),
            Some(&HeaderValue::from_static("application/msgpack"))
        );
        let bytes = to_bytes(response.into_body(), 64 * 1024).await.unwrap();
        let message =
            decode_cultnet_message_from_slice(&bytes, CultNetWireContract::CultNetSchemaV0)
                .unwrap();
        assert_eq!(
            encode_cultnet_message_to_vec(&message, CultNetWireContract::CultNetSchemaV0).unwrap(),
            bytes
        );
        let CultNetMessage::SnapshotResponseRaw {
            message_id,
            documents,
        } = message
        else {
            panic!("route observation was not a raw snapshot response");
        };
        assert_eq!(message_id, "route-challenge-42");
        let [document] = documents.as_slice() else {
            panic!("route observation did not contain exactly one document");
        };
        assert_eq!(document.schema_id, GAMECULT_RUNTIME_PRESENCE_HEALTH_SCHEMA);
        assert_eq!(document.record_key, GHOSTLIGHT_TARGET);
        let presence: GameCultRuntimePresenceHealthRecord =
            rmp_serde::from_slice(&document.payload).unwrap();
        assert_eq!(presence.state, "active");
        assert_eq!(presence.detail, "route-observation:route-challenge-42");
        assert_eq!(
            presence.write_lease_sha256.as_deref(),
            Some(write_lease_sha256.as_str())
        );
        authenticate_runtime_presence_claim(
            &document.payload,
            &authority,
            RuntimePresenceAuthenticationContext {
                trusted_received_at_unix_millis: presence.observed_at_unix_millis,
                maximum_age_millis: 1_000,
                maximum_future_skew_millis: 10,
            },
        )
        .unwrap();
    }

    fn invocation(operation: &str, schema: &str, version: u64, payload: Value, id: &str) -> Value {
        json!({
            "schema":"gamecult.eve.command_invocation.v1",
            "providerId":mesh::PROVIDER_ID,
            "surfaceId":mesh::SURFACE_ID,
            "operation":{
                "operationId":operation,
                "schemaId":schema,
                "idempotencyKey":id,
                "routeHint":{"sourceVersion":version,"transport":"https-json"}
            },
            "payload":payload,
            "issuedAt":Utc::now().to_rfc3339(),
            "clientId":"runtime-test",
            "commandBoundary":mesh::COMMAND_BOUNDARY,
            "receiptSchema":mesh::COMMAND_RESULT_SCHEMA
        })
    }

    async fn post(state: &AppState, cookie: &str, body: Value) -> Value {
        let response = api_router(state.clone())
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/eve/commands")
                    .header(header::CONTENT_TYPE, "application/json")
                    .header(header::COOKIE, format!("{COOKIE_NAME}={cookie}"))
                    .body(Body::from(serde_json::to_vec(&body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        let bytes = axum::body::to_bytes(response.into_body(), 1024 * 1024)
            .await
            .unwrap();
        serde_json::from_slice(&bytes).unwrap()
    }

    #[tokio::test]
    async fn http_eve_journey_uses_one_world_owner() {
        let fixture = fixture().await;
        let created = post(
            &fixture.state,
            &fixture.cookie,
            invocation(
                "world.create",
                "ghostlight.world_create.v4",
                0,
                json!({
                    "title":"Cutover World",
                    "brief":"",
                    "subject_label":"Operator",
                    "targets":{},
                    "jurisdictions":[],
                    "lens_weights":{"patina":1,"charter":1,"ledger":1,"hearth":1,"tangle":1,"veil":1,"ember":1,"numen":1}
                }),
                &uuid::Uuid::new_v4().to_string(),
            ),
        )
        .await;
        assert_eq!(created["state"], "accepted");
        assert_eq!(created["sourceVersion"], 1);

        let approved = post(
            &fixture.state,
            &fixture.cookie,
            invocation(
                "world.approve",
                "ghostlight.world_approve.v0",
                1,
                json!({}),
                &uuid::Uuid::new_v4().to_string(),
            ),
        )
        .await;
        assert_eq!(approved["sourceVersion"], 2);
        let activated = post(
            &fixture.state,
            &fixture.cookie,
            invocation(
                "world.activate",
                "ghostlight.world_activate.v0",
                2,
                json!({}),
                &uuid::Uuid::new_v4().to_string(),
            ),
        )
        .await;
        assert_eq!(activated["sourceVersion"], 3);

        let world = current_world(&fixture.state).await.unwrap().unwrap();
        let opportunity = world.opportunities[0].clone();
        let affordance = *world
            .affordances
            .iter()
            .find(|entry| {
                entry.entry.kind.0 == "speak" && world.subjects[0].affordances.contains(&entry.id)
            })
            .map(|entry| &entry.id)
            .unwrap();
        let spoken = post(
            &fixture.state,
            &fixture.cookie,
            invocation(
                "world.speak",
                "ghostlight.world_speak.v0",
                3,
                json!({
                    "text":"The new owner speaks.",
                    "opportunity":opportunity,
                    "affordance_id":affordance
                }),
                &uuid::Uuid::new_v4().to_string(),
            ),
        )
        .await;
        assert_eq!(spoken["sourceVersion"], 4);
        let (world, log) = current_operator_view(&fixture.state).await.unwrap();
        assert!(world.is_some());
        assert_eq!(log.len(), 1);
    }

    /// Cut 8b's own routing test: `world.play` reaches the play table and
    /// returns `accepted` — the request routes and returns before the turn
    /// itself resolves, exactly as the spawned-task shape calls for. The
    /// fixture's own play table shares the fixture's unreachable test
    /// inference organ (`127.0.0.1:9`), so the spawned turn will itself fail
    /// once it actually tries to infer; that failure is expected and is not
    /// this test's concern; only the HTTP response this route hands back
    /// before that happens is.
    #[tokio::test]
    async fn world_play_reaches_the_play_table_and_is_accepted() {
        let fixture = fixture().await;
        let created = post(
            &fixture.state,
            &fixture.cookie,
            invocation(
                "world.create",
                "ghostlight.world_create.v4",
                0,
                json!({
                    "title":"Play Routing World",
                    "brief":"",
                    "subject_label":"Operator",
                    "targets":{},
                    "jurisdictions":[],
                    "lens_weights":{"patina":1,"charter":1,"ledger":1,"hearth":1,"tangle":1,"veil":1,"ember":1,"numen":1}
                }),
                &uuid::Uuid::new_v4().to_string(),
            ),
        )
        .await;
        assert_eq!(created["state"], "accepted");

        let played = post(
            &fixture.state,
            &fixture.cookie,
            invocation(
                "world.play",
                "ghostlight.world_play.v0",
                1,
                json!({"text":"I look around."}),
                &uuid::Uuid::new_v4().to_string(),
            ),
        )
        .await;
        assert_eq!(played["state"], "accepted");
    }

    /// Same route, with `state.play` unavailable (mirrors `controllers:
    /// None` elsewhere in this suite): refused, not a panic or a hang.
    #[tokio::test]
    async fn world_play_without_a_play_table_is_denied() {
        let mut fixture = fixture().await;
        fixture.state.play = None;
        let created = post(
            &fixture.state,
            &fixture.cookie,
            invocation(
                "world.create",
                "ghostlight.world_create.v4",
                0,
                json!({
                    "title":"No Play World",
                    "brief":"",
                    "subject_label":"Operator",
                    "targets":{},
                    "jurisdictions":[],
                    "lens_weights":{"patina":1,"charter":1,"ledger":1,"hearth":1,"tangle":1,"veil":1,"ember":1,"numen":1}
                }),
                &uuid::Uuid::new_v4().to_string(),
            ),
        )
        .await;
        assert_eq!(created["state"], "accepted");

        let played = post(
            &fixture.state,
            &fixture.cookie,
            invocation(
                "world.play",
                "ghostlight.world_play.v0",
                1,
                json!({"text":"I look around."}),
                &uuid::Uuid::new_v4().to_string(),
            ),
        )
        .await;
        assert_eq!(played["state"], "denied");
    }

    #[tokio::test]
    async fn exact_stale_retry_returns_journal_receipt_without_app_cache() {
        let fixture = fixture().await;
        let id = uuid::Uuid::new_v4().to_string();
        let command = invocation(
            "world.create",
            "ghostlight.world_create.v4",
            0,
            json!({
                "title":"Retry World",
                "brief":"",
                "subject_label":"Operator",
                "targets":{},
                "jurisdictions":[],
                "lens_weights":{"patina":1,"charter":1,"ledger":1,"hearth":1,"tangle":1,"veil":1,"ember":1,"numen":1}
            }),
            &id,
        );
        let first = post(&fixture.state, &fixture.cookie, command.clone()).await;
        let second = post(&fixture.state, &fixture.cookie, command).await;
        assert_eq!(
            first["receipt"]["commitDigest"],
            second["receipt"]["commitDigest"]
        );
        assert_eq!(
            current_world(&fixture.state)
                .await
                .unwrap()
                .unwrap()
                .revision,
            0
        );
    }

    /// PA.f12: the dropped `no_proposal_projection_carries_the_canonical_world_commit`
    /// was the only test of `submit_receipt`'s `applied` / `already_applied`
    /// kinds and revision. This is the fix, through the real Dungeon path
    /// rather than calling `submit_receipt` directly: one principal command,
    /// submitted twice under the same idempotency key, must come back
    /// `applied` and then `already_applied`, both naming the same revision.
    #[tokio::test]
    async fn a_principal_command_resubmitted_under_its_own_key_returns_applied_then_already_applied()
     {
        let fixture = fixture().await;
        post(
            &fixture.state,
            &fixture.cookie,
            invocation(
                "world.create",
                "ghostlight.world_create.v4",
                0,
                json!({
                    "title":"Retry Receipt World",
                    "brief":"",
                    "subject_label":"Operator",
                    "targets":{},
                    "jurisdictions":[],
                    "lens_weights":{"patina":1,"charter":1,"ledger":1,"hearth":1,"tangle":1,"veil":1,"ember":1,"numen":1}
                }),
                &uuid::Uuid::new_v4().to_string(),
            ),
        )
        .await;
        post(
            &fixture.state,
            &fixture.cookie,
            invocation(
                "world.approve",
                "ghostlight.world_approve.v0",
                1,
                json!({}),
                &uuid::Uuid::new_v4().to_string(),
            ),
        )
        .await;
        post(
            &fixture.state,
            &fixture.cookie,
            invocation(
                "world.activate",
                "ghostlight.world_activate.v0",
                2,
                json!({}),
                &uuid::Uuid::new_v4().to_string(),
            ),
        )
        .await;
        let id = uuid::Uuid::new_v4().to_string();
        let command = invocation(
            "world.advance_time",
            "ghostlight.world_advance_time.v0",
            3,
            json!({"minutes": 5}),
            &id,
        );
        let first = post(&fixture.state, &fixture.cookie, command.clone()).await;
        let second = post(&fixture.state, &fixture.cookie, command).await;
        assert_eq!(first["receipt"]["kind"], "applied");
        assert_eq!(second["receipt"]["kind"], "already_applied");
        assert_eq!(first["receipt"]["revision"], second["receipt"]["revision"]);
    }

    #[tokio::test]
    async fn logout_retry_clears_a_revoked_cookie_without_reauthentication() {
        let fixture = fixture().await;
        let id = uuid::Uuid::new_v4().to_string();
        let command = invocation(
            "app.auth.logout",
            "ghostlight.app_logout.v2",
            0,
            json!({}),
            &id,
        );
        let first = post(&fixture.state, &fixture.cookie, command.clone()).await;
        let retry = post(&fixture.state, &fixture.cookie, command).await;
        assert_eq!(first["state"], "accepted");
        assert_eq!(retry["state"], "accepted");
        assert_eq!(retry["pluginPayload"]["payload"]["status"], "anonymous");
    }

    #[tokio::test]
    async fn removed_legacy_operation_is_denied_before_dispatch() {
        let fixture = fixture().await;
        for (operation, schema) in [
            ("session_zero.begin", "ghostlight.session_zero_begin.v1"),
            ("world.controller.act", "ghostlight.world_controller_act.v0"),
        ] {
            let result = post(
                &fixture.state,
                &fixture.cookie,
                invocation(
                    operation,
                    schema,
                    0,
                    json!({}),
                    &uuid::Uuid::new_v4().to_string(),
                ),
            )
            .await;
            assert_eq!(result["state"], "denied", "{operation}");
            assert!(
                result["message"]
                    .as_str()
                    .unwrap()
                    .contains("not advertised"),
                "{operation}"
            );
        }
    }

    #[tokio::test]
    async fn authentication_commands_reject_ignored_authority_payloads() {
        let fixture = fixture().await;
        let result = post(
            &fixture.state,
            &fixture.cookie,
            invocation(
                "heimdall.auth.begin",
                "heimdall.auth_begin_command.v1",
                0,
                json!({"caller":{"principal":"legacy-owner"}}),
                &uuid::Uuid::new_v4().to_string(),
            ),
        )
        .await;
        assert_eq!(result["state"], "denied");
        assert!(
            result["message"]
                .as_str()
                .unwrap()
                .contains("payload may not supply caller authority")
        );
    }

    /// A Draft world from `world.create`'s own intent: the owner's Human
    /// subject beside a narrative persona and an operational agent, with the
    /// given scale intent and jurisdictions, so a test can look at the Draft
    /// world the seed lane actually runs against.
    async fn two_cell_world(
        state: &AppState,
        cookie: &str,
        targets: BTreeMap<SubjectKind, u32>,
        jurisdictions: Vec<CreateJurisdictionIntent>,
    ) {
        let principal = state
            .sessions
            .lock()
            .await
            .account_for_cookie(cookie, Utc::now())
            .unwrap()
            .expect("the fixture cookie names a live session");
        state
            .world
            .create(
                CreateWorldIntent {
                    id: CommandId::new(),
                    title: "Draft Fixture".into(),
                    brief: String::new(),
                    human_subject_label: "Operator".into(),
                    narrative_persona_label: Some("Persona".into()),
                    operational_agent_label: Some("Operational Agent".into()),
                    targets,
                    jurisdictions,
                    lens_weights: uniform_lens_weights(),
                },
                &principal,
            )
            .await
            .unwrap();
    }

    /// The library half of this rule is `sdk_inference`'s own
    /// `soul_no_credential_name_appears_in_the_ports_own_source`, which can
    /// only read its own crate's source. This is the Dungeon half, on the same
    /// needles: nothing in the daemon's production source may name a
    /// credential path or token variable the SDK port refuses to read.
    #[test]
    fn soul_no_credential_name_appears_in_the_runtimes_own_source() {
        let source = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/runtime.rs"),
        )
        .expect("the source reads")
        .replace("\r\n", "\n");
        // Assembled from halves so this test's own source is not a match.
        let needles: [String; 8] = [
            format!(".{}", "credentials.json"),
            format!("CLAUDE_CODE{}", "_OAUTH_TOKEN"),
            format!(".{}", "claude.json"),
            format!("apiKey{}", "Helper"),
            format!("USER{}", "PROFILE"),
            format!("home{}", "_dir"),
            format!("GHOSTLIGHT_SDK{}", "_TOKEN"),
            format!("GHOSTLIGHT_SDK{}", "_CREDENTIAL"),
        ];
        // Only the production half; a test may name what production must not.
        let production = source
            .split_once("\n#[cfg(test)]\nmod tests {")
            .map(|(before, _)| before.to_owned())
            .unwrap_or(source);
        for needle in &needles {
            assert!(
                !production.contains(needle.as_str()),
                "runtime.rs names {needle}"
            );
        }
        // `ANTHROPIC_API_KEY` may be named in a comment, never anywhere that
        // could read it.
        let anthropic = format!("ANTHROPIC{}", "_");
        for line in production.lines() {
            if line.contains(anthropic.as_str()) {
                assert!(
                    line.trim_start().starts_with("///") || line.trim_start().starts_with("//"),
                    "runtime.rs names an ANTHROPIC variable outside a comment: {line}"
                );
            }
        }
    }

    /// Every `.rs` file under this crate's `src`, with each file's own
    /// `#[cfg(test)] mod tests { .. }` block excluded the way
    /// `soul_no_credential_name_appears_in_the_runtimes_own_source` excludes
    /// its own: a test module may name what production must not. Unlike that
    /// scan, PA.f11 must reach every file in the crate, not just this one,
    /// because a forbidden writer could return in `eve.rs`, `mesh.rs`, or
    /// anywhere else Dungeon owns.
    fn dungeon_non_test_source() -> Vec<(PathBuf, String)> {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut files = Vec::new();
        let mut pending = vec![root];
        while let Some(directory) = pending.pop() {
            for entry in std::fs::read_dir(&directory).expect("dungeon src directory reads") {
                let entry = entry.expect("dungeon src directory entry reads");
                let path = entry.path();
                if path.is_dir() {
                    pending.push(path);
                    continue;
                }
                if path.extension().and_then(|extension| extension.to_str()) == Some("rs") {
                    files.push(path);
                }
            }
        }
        files
            .into_iter()
            .map(|path| {
                let source = std::fs::read_to_string(&path)
                    .expect("dungeon source file reads")
                    .replace("\r\n", "\n");
                // The earliest test-module marker in the file, mirroring the
                // one runtime.rs's own scan splits on; a file with neither
                // marker (for example main.rs) has no test module, so its
                // whole source is production.
                let production = ["\n#[cfg(test)]\nmod tests {", "\n#[cfg(test)]\npub(crate) mod tests {"]
                    .into_iter()
                    .filter_map(|marker| source.split_once(marker).map(|(before, _)| before.len()))
                    .min()
                    .map(|len| source[..len].to_owned())
                    .unwrap_or(source);
                (path, production)
            })
            .collect()
    }

    /// PA.f11: P1.1 and P1.2 promise that no Dungeon code path submits a clock
    /// tick or runs a Persona, operational or elaboration lane outside a
    /// turn. Neither promise had a test; Soul re-added a 30-second
    /// `submit_clock` loop and a spawned `sweep` and both passed 46/46. This
    /// is the fix: a source-scan test on the precedent below, banning the
    /// deleted forbidden writers' own names from every file Dungeon owns.
    /// `PersonaLane` (Cut 8) is deliberately not named here: dispatching a
    /// Persona through it is the owner this cut clears the ground for, not a
    /// forbidden writer.
    #[test]
    fn soul_no_forbidden_writer_name_appears_in_dungeons_own_source() {
        let needles = [
            "submit_clock(",
            ".elaborator(",
            "run_narrative(",
            "run_operational(",
            "run_cell(",
            "ElaborationRunner",
        ];
        for (path, production) in dungeon_non_test_source() {
            for needle in needles {
                assert!(
                    !production.contains(needle),
                    "{} names {needle}",
                    path.display()
                );
            }
        }
    }

    /// A missing credentials path is the fail-closed default: no consumer is
    /// configured, and startup proceeds. A path that names a file that exists
    /// but does not decode is a different situation entirely -- a mistyped or
    /// corrupted credential -- and must not be swallowed into the same empty
    /// registry. It fails startup instead.
    #[test]
    fn a_malformed_consumer_credentials_file_refuses_the_registry_rather_than_starting_empty() {
        // Serialize against any other test in this binary that touches the
        // same process-wide environment variable.
        static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
        let _guard = ENV_LOCK.lock().unwrap_or_else(|poison| poison.into_inner());

        let previous = std::env::var(ghostlight::CONSUMER_CREDENTIALS_ENVIRONMENT).ok();
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("consumers.cc");
        std::fs::write(&path, b"not a consumer credentials file").unwrap();
        // SAFETY: serialized by ENV_LOCK above; no other thread in this test
        // binary reads or writes this variable concurrently.
        unsafe {
            std::env::set_var(
                ghostlight::CONSUMER_CREDENTIALS_ENVIRONMENT,
                path.as_os_str(),
            );
        }
        let result = open_consumer_registry();
        // SAFETY: same lock, restoring (or clearing) the prior value.
        unsafe {
            match &previous {
                Some(value) => {
                    std::env::set_var(ghostlight::CONSUMER_CREDENTIALS_ENVIRONMENT, value)
                }
                None => std::env::remove_var(ghostlight::CONSUMER_CREDENTIALS_ENVIRONMENT),
            }
        }
        assert!(
            result.is_err(),
            "a malformed credentials file must refuse the registry, not start empty"
        );
    }

    // ---- The seed command ------------------------------------------------

    /// Spec test 1. The previous create schema is not kept alive beside
    /// `world_create.v4`: a complete payload announcing it dies at validation before any handler runs,
    /// and a payload that announces v4 but omits the lens weights or the brief
    /// is a payload error rather than a defaulted intent. None creates a world.
    #[tokio::test]
    async fn a_stale_create_payload_is_refused() {
        let fixture = fixture().await;
        let stale = post(
            &fixture.state,
            &fixture.cookie,
            invocation(
                "world.create",
                "ghostlight.world_create.v3",
                0,
                json!({"title":"Stale World","brief":"","subject_label":"Operator","targets":{},"jurisdictions":[]}),
                &uuid::Uuid::new_v4().to_string(),
            ),
        )
        .await;
        assert_eq!(stale["state"], "denied");
        assert!(current_world(&fixture.state).await.unwrap().is_none());

        let unweighted = post(
            &fixture.state,
            &fixture.cookie,
            invocation(
                "world.create",
                "ghostlight.world_create.v4",
                0,
                json!({"title":"Unweighted World","brief":"","subject_label":"Operator","targets":{},"jurisdictions":[]}),
                &uuid::Uuid::new_v4().to_string(),
            ),
        )
        .await;
        assert_eq!(unweighted["state"], "denied");
        // Refused as a payload, before genesis: a defaulted empty set would
        // also be denied, by the resolver, and must not pass for this.
        let message = unweighted["message"].as_str().unwrap_or_default();
        assert!(
            message.contains("missing field `lens_weights`"),
            "the omission was not refused at the payload: {message}"
        );
        assert!(
            current_world(&fixture.state).await.unwrap().is_none(),
            "a v4 payload with no lens weights created a world anyway"
        );

        let partial = post(
            &fixture.state,
            &fixture.cookie,
            invocation(
                "world.create",
                "ghostlight.world_create.v4",
                0,
                json!({"title":"Half World","subject_label":"Operator","targets":{},"jurisdictions":[],"lens_weights":{"patina":1,"charter":1,"ledger":1,"hearth":1,"tangle":1,"veil":1,"ember":1,"numen":1}}),
                &uuid::Uuid::new_v4().to_string(),
            ),
        )
        .await;
        assert_eq!(partial["state"], "denied");
        assert!(
            current_world(&fixture.state).await.unwrap().is_none(),
            "a v4 payload with no brief created a world anyway"
        );
    }

    /// A lens name the library does not know, beside one it does, is refused
    /// by deserialization before any handler runs. No world is created.
    #[tokio::test]
    async fn a_create_payload_naming_an_unknown_lens_is_refused() {
        let fixture = fixture().await;
        let result = post(
            &fixture.state,
            &fixture.cookie,
            invocation(
                "world.create",
                "ghostlight.world_create.v4",
                0,
                json!({"title":"Tribunal World","brief":"","subject_label":"Operator","targets":{},"jurisdictions":[],"lens_weights":{"patina":1,"tribunal":1}}),
                &uuid::Uuid::new_v4().to_string(),
            ),
        )
        .await;
        assert_eq!(result["state"], "denied");
        let message = result["message"].as_str().unwrap_or_default();
        assert!(
            message.contains("tribunal"),
            "the unknown lens was not refused by name: {message}"
        );
        assert!(current_world(&fixture.state).await.unwrap().is_none());
    }

    /// Every lens named, every weight zero: the payload decodes, and the
    /// library's genesis refuses it with `LensWeightsNeverDraw`. No world.
    #[tokio::test]
    async fn a_create_payload_whose_lenses_never_draw_is_refused() {
        let fixture = fixture().await;
        let result = post(
            &fixture.state,
            &fixture.cookie,
            invocation(
                "world.create",
                "ghostlight.world_create.v4",
                0,
                json!({"title":"Still World","brief":"","subject_label":"Operator","targets":{},"jurisdictions":[],"lens_weights":{"patina":0,"charter":0,"ledger":0,"hearth":0,"tangle":0,"veil":0,"ember":0,"numen":0}}),
                &uuid::Uuid::new_v4().to_string(),
            ),
        )
        .await;
        assert_eq!(result["state"], "denied");
        let message = result["message"].as_str().unwrap_or_default();
        assert!(
            message.contains("LensWeightsNeverDraw"),
            "the refusal did not name the draw rule: {message}"
        );
        assert!(current_world(&fixture.state).await.unwrap().is_none());
    }

    /// Every `control.button` in a surface, at whatever depth.
    fn surface_buttons(node: &Value, into: &mut Vec<String>) {
        if node["kind"] == "control.button"
            && let Some(command) = node["props"]["command"].as_str()
        {
            into.push(command.to_owned());
        }
        for child in node["children"].as_array().into_iter().flatten() {
            surface_buttons(child, into);
        }
    }

    /// Spec test 11. An operation can be emitted by the panel, handled by
    /// `execute_world`, and still be dead because `operation_schema` does not
    /// name it — which is exactly what `world.advance_time` was. Every button
    /// the panel emits and every descriptor it advertises must resolve, and the
    /// descriptor's schema must be the one validation will demand.
    #[tokio::test]
    async fn every_operation_the_panel_emits_has_a_schema() {
        let fixture = fixture().await;
        let owner = fixture
            .state
            .sessions
            .lock()
            .await
            .account_for_cookie(&fixture.cookie, Utc::now())
            .unwrap()
            .unwrap()
            .account_subject_hash()
            .to_owned();
        let stranger = "someone-else";
        let mut surfaces = vec![eve::authenticated_surface(&owner, None, &[]).unwrap()];
        two_cell_world(
            &fixture.state,
            &fixture.cookie,
            BTreeMap::from([(SubjectKind::Person, 4)]),
            vec![CreateJurisdictionIntent {
                handle: "sere".into(),
                label: "The Low Sere".into(),
                permille: 1000,
            }],
        )
        .await;
        let draft = fixture.state.world.snapshot().await.unwrap();
        assert_eq!(draft.phase, WorldPhase::Draft);
        for account in [owner.as_str(), stranger] {
            surfaces.push(eve::authenticated_surface(account, Some(&draft), &[]).unwrap());
        }
        for body in [CommandBody::ApproveDraft, CommandBody::ActivateWorld] {
            let snapshot = fixture.state.world.snapshot().await.unwrap();
            let principal = fixture
                .state
                .sessions
                .lock()
                .await
                .account_for_cookie(&fixture.cookie, Utc::now())
                .unwrap()
                .unwrap();
            fixture
                .state
                .world
                .submit_principal(
                    PrincipalCommandIntent {
                        id: CommandId::new(),
                        world_id: snapshot.world_id,
                        expected_revision: snapshot.revision,
                        body,
                    },
                    &principal,
                )
                .await
                .unwrap();
        }
        let active = fixture.state.world.snapshot().await.unwrap();
        assert_eq!(active.phase, WorldPhase::Active);
        for account in [owner.as_str(), stranger] {
            surfaces.push(eve::authenticated_surface(account, Some(&active), &[]).unwrap());
        }
        surfaces.push(eve::anonymous_surface());

        let mut seen = 0usize;
        for surface in &surfaces {
            let mut buttons = Vec::new();
            surface_buttons(&surface["surface"]["root"], &mut buttons);
            assert!(!buttons.is_empty());
            for command in &buttons {
                assert!(
                    eve::operation_schema(command).is_some(),
                    "the panel emits {command}, which Ghostlight does not advertise"
                );
                seen += 1;
            }
            for descriptor in surface["commands"].as_array().unwrap() {
                let command = descriptor["command"].as_str().unwrap();
                assert_eq!(
                    eve::operation_schema(command),
                    descriptor["payloadSchema"].as_str(),
                    "the descriptor for {command} names a schema validation will refuse"
                );
            }
        }
        assert!(seen > 5, "the walk found almost nothing to check");
    }

    /// Spec test 12. Seeding is the owner's lane and Draft's lane, and both
    /// refusals land before the request reaches a paid endpoint or a vault.
    #[tokio::test]
    async fn world_seed_is_owner_only_and_draft_only_before_it_spends_anything() {
        let fixture = fixture().await;
        let stranger = fixture
            .state
            .sessions
            .lock()
            .await
            .create_session(heimdall::VerifiedSessionAdmission::fixture(
                "stranger-account",
                "heimdall-stranger",
                1,
                Utc::now() + chrono::Duration::hours(1),
                Utc::now() + chrono::Duration::days(1),
                "stranger-refresh",
            ))
            .unwrap();
        two_cell_world(
            &fixture.state,
            &fixture.cookie,
            BTreeMap::from([(SubjectKind::Person, 4)]),
            vec![CreateJurisdictionIntent {
                handle: "sere".into(),
                label: "The Low Sere".into(),
                permille: 1000,
            }],
        )
        .await;
        let draft = fixture.state.world.snapshot().await.unwrap();

        // The vault root is deliberately unset: neither refusal may reach it.
        unsafe { std::env::remove_var(SEED_VAULT_ROOT_ENVIRONMENT) };
        let denied = post(
            &fixture.state,
            &stranger,
            invocation(
                "world.seed",
                "ghostlight.world_seed.v1",
                draft.revision + 1,
                json!({}),
                &uuid::Uuid::new_v4().to_string(),
            ),
        )
        .await;
        assert_eq!(denied["state"], "denied");
        assert!(
            denied["message"].as_str().unwrap().contains("owner"),
            "{denied}"
        );

        for body in [CommandBody::ApproveDraft, CommandBody::ActivateWorld] {
            let snapshot = fixture.state.world.snapshot().await.unwrap();
            let principal = fixture
                .state
                .sessions
                .lock()
                .await
                .account_for_cookie(&fixture.cookie, Utc::now())
                .unwrap()
                .unwrap();
            fixture
                .state
                .world
                .submit_principal(
                    PrincipalCommandIntent {
                        id: CommandId::new(),
                        world_id: snapshot.world_id,
                        expected_revision: snapshot.revision,
                        body,
                    },
                    &principal,
                )
                .await
                .unwrap();
        }
        let active = fixture.state.world.snapshot().await.unwrap();
        let refused = post(
            &fixture.state,
            &fixture.cookie,
            invocation(
                "world.seed",
                "ghostlight.world_seed.v1",
                active.revision + 1,
                json!({}),
                &uuid::Uuid::new_v4().to_string(),
            ),
        )
        .await;
        assert_eq!(refused["state"], "accepted");
        assert_eq!(refused["receipt"]["outcome"], "not_draft");
        assert_eq!(
            fixture.state.world.snapshot().await.unwrap().revision,
            active.revision,
            "a refused seed still moved the world"
        );
    }

    /// Spec test 13. The card's rows are the deficit rows, and the shortfall it
    /// names is the row the runner will actually select.
    #[tokio::test]
    async fn the_seed_card_projects_the_deficit_it_will_answer() {
        let fixture = fixture().await;
        let owner = fixture
            .state
            .sessions
            .lock()
            .await
            .account_for_cookie(&fixture.cookie, Utc::now())
            .unwrap()
            .unwrap()
            .account_subject_hash()
            .to_owned();
        two_cell_world(
            &fixture.state,
            &fixture.cookie,
            BTreeMap::from([(SubjectKind::Person, 9)]),
            vec![CreateJurisdictionIntent {
                handle: "sere".into(),
                label: "The Low Sere".into(),
                permille: 1000,
            }],
        )
        .await;
        let draft = fixture.state.world.snapshot().await.unwrap();
        let surface = eve::authenticated_surface(
            &owner,
            Some(&draft),
            &[],
        )
        .unwrap();
        let encoded = serde_json::to_string(&surface).unwrap();
        let card = surface["surface"]["root"]["children"]
            .as_array()
            .unwrap()
            .iter()
            .find(|child| child["id"] == "world.seed.card")
            .expect("the seed card");
        assert_eq!(
            card["children"].as_array().unwrap().len(),
            draft.scale_deficit.len()
        );
        let selected = ghostlight::select_row(&draft).expect("a shortfall to answer");
        assert_eq!(selected.target, 9);
        assert!(
            card["props"]["nextShortfall"]
                .as_str()
                .unwrap()
                .contains(&format!("short {}", selected.deficit)),
            "{card}"
        );
        assert!(
            card["props"]["detail"]
                .as_str()
                .unwrap()
                .contains(&format!("{}", draft.revision.saturating_sub(1))),
        );
        assert!(encoded.contains("world.seed"), "the button is missing");
    }

    /// `SeedPort` holds one `VerifiedPrincipalEvidence` for the whole session,
    /// and a multi-round session can run long past the moment the evidence was
    /// minted. `VerifiedPrincipalEvidence` now carries `valid_until`, minted
    /// from the session's own `access_expires_at`, and `submit_principal`
    /// reads the wall clock at ingress and refuses anything presented after
    /// that moment — so evidence cannot outlive the session that vouched for
    /// it, whatever the port itself still believes.
    #[tokio::test]
    async fn soul_verified_evidence_outlives_the_session_that_minted_it() {
        let fixture = fixture().await;
        two_cell_world(
            &fixture.state,
            &fixture.cookie,
            BTreeMap::new(),
            Vec::new(),
        )
        .await;
        let live = fixture
            .state
            .sessions
            .lock()
            .await
            .account_for_cookie(&fixture.cookie, Utc::now())
            .unwrap()
            .expect("the fixture cookie names a live session");
        // Same account as the live evidence, but minted with an expiry that
        // has already passed.
        let expired = VerifiedPrincipalEvidence::new(
            live.account_subject_hash().to_owned(),
            Utc::now() - chrono::Duration::seconds(1),
        );

        let before = fixture.state.world.snapshot().await.unwrap();
        let committed = fixture
            .state
            .world
            .submit_principal(
                PrincipalCommandIntent {
                    id: CommandId::new(),
                    world_id: before.world_id,
                    expected_revision: before.revision,
                    // `ApproveDraft`, because it is refused for anyone but a
                    // required approver and its effect is visible in the
                    // snapshot. What is under test is whether an expired
                    // claim is spent at all.
                    body: CommandBody::ApproveDraft,
                },
                &expired,
            )
            .await;
        assert!(
            committed.is_err(),
            "evidence minted with a past valid_until still committed: {committed:?}"
        );
        let after = fixture.state.world.snapshot().await.unwrap();
        assert_eq!(
            after.revision, before.revision,
            "an expired submission moved the world"
        );
        assert!(after.draft_approvals.is_empty());
    }

    /// Sibling to the above: evidence still inside its own window is not
    /// touched by the expiry gate, so the same account submitting the same
    /// command before `valid_until` commits normally.
    #[tokio::test]
    async fn soul_verified_evidence_within_its_window_still_commits() {
        let fixture = fixture().await;
        two_cell_world(
            &fixture.state,
            &fixture.cookie,
            BTreeMap::new(),
            Vec::new(),
        )
        .await;
        let live = fixture
            .state
            .sessions
            .lock()
            .await
            .account_for_cookie(&fixture.cookie, Utc::now())
            .unwrap()
            .expect("the fixture cookie names a live session");

        let before = fixture.state.world.snapshot().await.unwrap();
        let committed = fixture
            .state
            .world
            .submit_principal(
                PrincipalCommandIntent {
                    id: CommandId::new(),
                    world_id: before.world_id,
                    expected_revision: before.revision,
                    body: CommandBody::ApproveDraft,
                },
                &live,
            )
            .await;
        assert!(committed.is_ok(), "{committed:?}");
        let after = fixture.state.world.snapshot().await.unwrap();
        assert_eq!(after.revision, before.revision + 1);
    }
}
