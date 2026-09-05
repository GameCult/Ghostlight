//! One-process runtime for the sealed world owner.

use crate::{
    app_session::{AppSessionOwner, VerifiedPrincipalEvidence},
    eve::{self, EveCommandInvocation},
    heimdall::{self, HeimdallClient},
    idunn_health::{
        IDUNN_RUNTIME_CANDIDATE_BIND_ENVIRONMENT, ProcessWriteLeaseGuard, RuntimePresencePublisher,
        TARGET as GHOSTLIGHT_TARGET,
    },
    mesh::{self, MeshPublisher, MeshRuntimeIdentity},
    world::{
        AffordanceId, CONSUMER_BODY_LIMIT, CellRun, CommandBody, CommandId, ConsumerPort,
        ConsumerRegistry, ControllerError, ControllerModels, ControllerPendingReason,
        ControllerRunner, ControllerWorkCustody, Cover, CoverBudget, CreateJurisdictionIntent,
        CreateWorldIntent, DecisionInvocation, DecisionOpportunity, KernelError, MailboxError,
        NarrativeRun, OperationalRun, PrincipalCommandIntent, PrincipalId, SeedOutcome, SeedPort,
        Statement, SubjectKind, SubmissionDisposition, SubmitReceipt, TickMinutes,
        VaultEvidenceSource, WorldMailbox, WorldPhase, WorldSnapshot, derive_cover,
    },
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
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
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
    /// The one cognition organ, shared. Concurrency is owned by
    /// `controller_permits` and quarantine by `controller_quarantined`: an
    /// exclusive lock here would be a second owner of both, and a tick spends a
    /// budget of inferences rather than one.
    controllers: Option<Arc<ControllerRunner>>,
    controller_permits: Arc<Semaphore>,
    /// Set once a turn loses its local invariant. No further permit is granted;
    /// in-flight turns finish or fail on their own bindings.
    controller_quarantined: Arc<AtomicBool>,
    cover_budget: CoverBudget,
    /// Display-only, written by the tick driver and read by Eve and the mesh
    /// projection. It decides nothing: the cover it summarises was derived,
    /// used, and dropped before this was written.
    cover: Arc<Mutex<Option<CoverSummary>>>,
    sessions: Arc<Mutex<AppSessionOwner>>,
    heimdall: Arc<HeimdallClient>,
    mesh: Option<MeshPublisher>,
    mesh_identity: MeshRuntimeIdentity,
    runtime_health: Option<RuntimeHealthOwner>,
    revisions: broadcast::Sender<u64>,
    fatal: mpsc::UnboundedSender<String>,
}

/// What one tick's cover looked like, for the operator surfaces. A projection
/// of a derived value, never an input to the next tick.
#[derive(Clone, Copy, Debug)]
struct CoverSummary {
    tick: u64,
    cells: usize,
    singletons: usize,
    groups: usize,
    oversubscribed: bool,
}

/// The read-only projection of the budget and the last tick, for Eve.
async fn cover_panel(state: &AppState) -> eve::CoverPanel {
    eve::CoverPanel {
        cells: state.cover_budget.cells,
        constituent_cap: state.cover_budget.constituent_cap,
        urgency_slots: state.cover_budget.urgency_slots,
        last: state.cover.lock().await.map(|summary| eve::CoverPanelTick {
            tick: summary.tick,
            cells: summary.cells,
            singletons: summary.singletons,
            groups: summary.groups,
            oversubscribed: summary.oversubscribed,
        }),
    }
}

impl CoverSummary {
    fn of(cover: &Cover) -> Self {
        Self {
            tick: cover.tick.0,
            cells: cover.cells.len(),
            singletons: cover.singletons(),
            groups: cover.groups(),
            oversubscribed: cover.oversubscribed,
        }
    }
}

struct ProductionAdmission {
    health: RuntimePresencePublisher,
    write_lease: ProcessWriteLeaseGuard,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CreatePayload {
    title: String,
    subject_label: String,
    #[serde(default)]
    narrative_persona_label: Option<String>,
    #[serde(default)]
    operational_agent_label: Option<String>,
    /// World-wide target of goal-bearing subjects per kind. Required, and may
    /// be empty: a world with no target is a deliberate choice, not a default
    /// that arrives because nobody said anything. A payload that omits it is
    /// refused, which is what `world_create.v2` means.
    targets: BTreeMap<SubjectKind, u32>,
    /// The jurisdiction roots, declared by genesis beside the commons because
    /// `resolve_patch` only resolves roots the same patch declares. A duplicate
    /// handle and a permille sum over 1000 are refused by the resolver, not
    /// pre-checked here: a pre-check would be a second reducer.
    jurisdictions: Vec<CreateJurisdiction>,
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

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ControllerActPayload {
    opportunity: DecisionOpportunity,
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
    let controllers = match open_controller(&world, &service_root, &runtime_id, connector_endpoint)
    {
        Ok(runner) => Some(runner),
        Err(error) => {
            tracing::warn!(%error, "controller cognition is unavailable; world authority remains online");
            None
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
    let cover_budget = configured_cover_budget()?;
    let consumers = Arc::new(open_consumer_registry()?);
    let mut state = AppState {
        consumer: ConsumerPort::new(world.clone()),
        consumers,
        world,
        controllers: controllers.map(Arc::new),
        controller_permits: Arc::new(Semaphore::new(configured_controller_concurrency())),
        controller_quarantined: Arc::new(AtomicBool::new(false)),
        cover_budget,
        cover: Arc::new(Mutex::new(None)),
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
    tokio::spawn(drive_cover_tick(state.clone(), configured_tick_interval()));
    tokio::spawn(elaborate_world(state.clone()));
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

fn open_controller(
    world: &WorldMailbox,
    service_root: &std::path::Path,
    runtime_id: &str,
    endpoint: SocketAddr,
) -> anyhow::Result<ControllerRunner> {
    let credential = std::env::var_os("GHOSTLIGHT_CONTROLLER_CREDENTIAL")
        .map(PathBuf::from)
        .context("GHOSTLIGHT_CONTROLLER_CREDENTIAL is required")?;
    ControllerRunner::open(
        world.clone(),
        endpoint,
        credential,
        runtime_id.to_owned(),
        service_root.join("controller-work.cc"),
        ControllerModels {
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
        },
    )
    .map_err(Into::into)
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
/// `world::consumer`, which owns decode, bounds, authentication, and the one
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
    let receipt = crate::world::admit_document(&state.consumer, &state.consumers, &body).await;
    match crate::world::encode_receipt(&receipt) {
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
    let Ok(path) = std::env::var(crate::world::CONSUMER_CREDENTIALS_ENVIRONMENT) else {
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
    let panel = cover_panel(&state).await;
    match current_operator_view(&state)
        .await
        .and_then(|(snapshot, log)| {
            eve::authenticated_surface(
                principal.account_subject_hash(),
                snapshot.as_ref(),
                &log,
                &panel,
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
    if invocation.operation.operation_id == "world.controller.act" {
        return dispatch_controller(state, principal, invocation).await;
    }
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

async fn dispatch_controller(
    state: &AppState,
    principal: &VerifiedPrincipalEvidence,
    invocation: EveCommandInvocation,
) -> Response {
    let admitted = admit_controller_command(state, principal, &invocation).await;
    let (command_id, opportunity) = match admitted {
        Ok(admitted) => admitted,
        Err(error) => {
            return Json(eve::command_result(
                &invocation,
                "denied",
                error.to_string(),
                current_world(state)
                    .await
                    .ok()
                    .map(|snapshot| eve::surface_version(snapshot.as_ref())),
                None,
                None,
            ))
            .into_response();
        }
    };

    let available = state
        .controllers
        .as_ref()
        .filter(|_| !state.controller_quarantined.load(Ordering::SeqCst));
    let Some(controller) = available else {
        return Json(eve::command_result(
            &invocation,
            "unknown",
            "Controller cognition is unavailable; no world outcome is claimed.",
            current_world(state)
                .await
                .ok()
                .map(|snapshot| eve::surface_version(snapshot.as_ref())),
            None,
            None,
        ))
        .into_response();
    };
    // One permit for the whole turn, from the same pool the tick driver spends.
    // There is no second concurrency owner.
    let permit = state.controller_permits.clone().acquire_owned().await;
    let result = match opportunity.controller_mode {
        crate::world::ControllerMode::NarrativePersona => controller
            .run_narrative(command_id, &opportunity)
            .await
            .map(controller_narrative_result),
        crate::world::ControllerMode::OperationalAgent => controller
            .run_operational(command_id, &opportunity)
            .await
            .map(controller_operational_result),
        crate::world::ControllerMode::Human => unreachable!("human opportunity was not admitted"),
    };
    drop(permit);
    let quarantine = match &result {
        Ok(ControllerHttpResult::Pending { quarantine, .. }) => *quarantine,
        Ok(ControllerHttpResult::Completed { .. }) => false,
        Err(error) => error.requires_quarantine(),
    };
    if quarantine {
        tracing::error!("controller cognition quarantined after losing its local invariant");
        state.controller_quarantined.store(true, Ordering::SeqCst);
    }

    match result {
        Ok(ControllerHttpResult::Completed {
            message,
            receipt,
            world_changed,
        }) => {
            let version = if world_changed {
                match publish_projection(state).await {
                    Ok(version) => Some(version),
                    Err(error) => {
                        tracing::error!(%error, "world revision could not be read after controller commit");
                        current_world(state)
                            .await
                            .ok()
                            .map(|snapshot| eve::surface_version(snapshot.as_ref()))
                    }
                }
            } else {
                current_world(state)
                    .await
                    .ok()
                    .map(|snapshot| eve::surface_version(snapshot.as_ref()))
            };
            Json(eve::command_result(
                &invocation,
                "accepted",
                message,
                version,
                None,
                Some(receipt),
            ))
            .into_response()
        }
        Ok(ControllerHttpResult::Pending {
            state_name,
            message,
            receipt,
            ..
        }) => Json(eve::command_result(
            &invocation,
            state_name,
            message,
            current_world(state)
                .await
                .ok()
                .map(|snapshot| eve::surface_version(snapshot.as_ref())),
            None,
            Some(receipt),
        ))
        .into_response(),
        Err(error) => {
            let state_name = controller_error_disposition(&error);
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

async fn admit_controller_command(
    state: &AppState,
    principal: &VerifiedPrincipalEvidence,
    invocation: &EveCommandInvocation,
) -> Result<(CommandId, DecisionOpportunity), RuntimeCommandError> {
    let command_id = CommandId::parse_uuid(
        invocation
            .operation
            .idempotency_key
            .as_deref()
            .unwrap_or(""),
    )?;
    let payload: ControllerActPayload = serde_json::from_value(invocation.payload.clone())
        .map_err(|error| RuntimeCommandError::Payload(error.to_string()))?;
    if payload.opportunity.controller_mode == crate::world::ControllerMode::Human {
        return Err(RuntimeCommandError::Payload(
            "human decisions cannot enter through the controller runner".into(),
        ));
    }
    let snapshot = current_world(state)
        .await
        .map_err(|error| RuntimeCommandError::Payload(error.to_string()))?
        .ok_or_else(|| RuntimeCommandError::Payload("world has not been created".into()))?;
    if snapshot.owner != PrincipalId::new(principal.account_subject_hash()) {
        return Err(RuntimeCommandError::Payload(
            "only the world owner may ask a controller to act".into(),
        ));
    }
    if payload.opportunity.world_id != snapshot.world_id {
        return Err(RuntimeCommandError::Payload(
            "controller opportunity belongs to another world".into(),
        ));
    }
    Ok((command_id, payload.opportunity))
}

enum ControllerHttpResult {
    Completed {
        message: &'static str,
        receipt: Value,
        world_changed: bool,
    },
    Pending {
        state_name: &'static str,
        message: &'static str,
        receipt: Value,
        quarantine: bool,
    },
}

fn controller_narrative_result(run: NarrativeRun) -> ControllerHttpResult {
    match run {
        NarrativeRun::Completed(decision) => {
            let (turn, capture, disposition) = decision.into_parts();
            let (submission, world_changed) = controller_submission(disposition);
            ControllerHttpResult::Completed {
                message: "Narrative Persona completed its decision.",
                receipt: json!({
                    "kind":"narrative_persona",
                    "personaProse":turn.source_prose(),
                    "personaSourceDigest":turn.source_digest(),
                    "personaReceiptDigest":turn.receipt_digest(),
                    "interpretation":{
                        "proposal":capture.proposal.as_ref().map(|source| json!({
                            "startByte":source.start_byte,
                            "endByte":source.end_byte,
                        })),
                        "gaps":capture.gaps.iter().map(|gap| json!({
                            "kind":gap.kind,
                            "startByte":gap.source.start_byte,
                            "endByte":gap.source.end_byte,
                            "detail":gap.detail,
                        })).collect::<Vec<_>>(),
                        "finalization":capture.finalization,
                        "inferenceReceipts":capture.inference_receipts,
                    },
                    "submission":submission,
                }),
                world_changed,
            }
        }
        NarrativeRun::Pending(pending) => {
            controller_pending_result(pending.mode(), pending.reason(), pending.persona_prose())
        }
    }
}

fn controller_operational_result(run: OperationalRun) -> ControllerHttpResult {
    match run {
        OperationalRun::Completed(decision) => {
            let (capture, disposition) = decision.into_parts();
            let (submission, world_changed) = controller_submission(disposition);
            ControllerHttpResult::Completed {
                message: "Operational agent completed its decision.",
                receipt: json!({
                    "kind":"operational_agent",
                    "proposal":capture.proposal,
                    "needs":capture.needs,
                    "inferenceReceipts":capture.inference_receipts,
                    "submission":submission,
                }),
                world_changed,
            }
        }
        OperationalRun::Pending(pending) => {
            controller_pending_result(pending.mode(), pending.reason(), None)
        }
    }
}

fn controller_pending_result(
    mode: crate::world::ControllerMode,
    reason: ControllerPendingReason,
    persona_prose: Option<&str>,
) -> ControllerHttpResult {
    let (state_name, message, quarantine) = match reason {
        ControllerPendingReason::InferenceRetryable => (
            "pending",
            "The exact controller inference remains available for connector replay.",
            false,
        ),
        ControllerPendingReason::InferenceRecoveryRequired => (
            "unknown",
            "The connector cannot establish an exact outcome for this inference.",
            false,
        ),
        ControllerPendingReason::WorldUnavailable => (
            "unknown",
            "The world owner is unavailable; no controller outcome is claimed.",
            false,
        ),
        ControllerPendingReason::WorldOutcomeUnknown => (
            "unknown",
            "World commit custody is uncertain; no controller outcome is claimed.",
            false,
        ),
        ControllerPendingReason::StoreReopenRequired => (
            "unknown",
            "Controller work-store custody is uncertain; controller cognition has been quarantined.",
            true,
        ),
    };
    ControllerHttpResult::Pending {
        state_name,
        message,
        receipt: json!({
            "kind":"controller_pending",
            "mode":mode,
            "reason":format!("{reason:?}"),
            "personaProse":persona_prose,
        }),
        quarantine,
    }
}

fn controller_submission(disposition: SubmissionDisposition) -> (Value, bool) {
    match disposition {
        SubmissionDisposition::NoProposal(receipt) => {
            let changed = matches!(&receipt, SubmitReceipt::Applied(_));
            (
                json!({
                    "kind":"no_proposal",
                    "commit":submit_receipt(receipt),
                }),
                changed,
            )
        }
        SubmissionDisposition::Completed(receipt) => {
            let changed = matches!(&receipt, SubmitReceipt::Applied(_));
            (submit_receipt(receipt), changed)
        }
        SubmissionDisposition::PreviouslyConfirmed(confirmation) => (
            json!({
                "kind":"previously_confirmed",
                "commandId":confirmation.command_id,
                "revision":confirmation.resulting_revision,
                "stateDigest":confirmation.resulting_state_digest,
                "commitDigest":confirmation.commit_digest,
            }),
            false,
        ),
    }
}

fn controller_error_disposition(error: &ControllerError) -> &'static str {
    match error {
        ControllerError::WorkPersistence(_) | ControllerError::Serialization(_) => "unknown",
        ControllerError::Inference { .. } | ControllerError::ProviderContract { .. } => "unknown",
        ControllerError::Snapshot(
            MailboxError::Unavailable | MailboxError::OutcomeUnknown { .. },
        ) => "unknown",
        ControllerError::Snapshot(_)
        | ControllerError::NoOpportunity { .. }
        | ControllerError::AmbiguousOpportunity
        | ControllerError::OpportunityMismatch
        | ControllerError::SpeakUnavailable
        | ControllerError::NoGrantedAffordance
        | ControllerError::CommandMismatch
        | ControllerError::MissingControllerWork
        | ControllerError::World(_) => "denied",
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
) -> anyhow::Result<(Option<WorldSnapshot>, Vec<crate::world::OperatorEvent>)> {
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
        _ if state.controller_quarantined.load(Ordering::SeqCst) => "unavailable",
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

/// How often the tick task submits, and how many fictional minutes each
/// submission names. The wall clock decides *when* to submit and this constant
/// decides *how many minutes* the command carries; neither reaches `reduce`.
const CLOCK_TICK_INTERVAL: Duration = Duration::from_secs(60);
const CLOCK_TICK_MINUTES: u32 = 60;

/// One tick's work, apart from the wall clock that decides when to run it: ask
/// `submit` for the outcome of one `minutes`-wide `AdvanceTime`, and log
/// (never propagate) a refusal — no world yet, a Draft world, a stale
/// revision. `submit` is a narrow port over [`WorldMailbox::submit_clock`], so
/// a test can capture what one tick submits without driving tokio's timer.
async fn submit_clock_tick<F, Fut>(minutes: TickMinutes, submit: F)
where
    F: FnOnce(CommandId, TickMinutes) -> Fut,
    Fut: std::future::Future<Output = Result<SubmitReceipt, MailboxError>>,
{
    match submit(CommandId::new(), minutes).await {
        Ok(_) => {}
        Err(error) => tracing::debug!(%error, "world clock tick was not admitted"),
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
    let Some(controllers) = state
        .controllers
        .as_deref()
        .filter(|_| !state.controller_quarantined.load(Ordering::SeqCst))
    else {
        return Err(RuntimeCommandError::Payload(
            "the cognition organ is closed or quarantined".into(),
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

/// How often the authoring lane takes one sweep. Slow, and skipping: a sweep
/// that admits nothing is a fixed point, not a reason to spin against a paid
/// inference endpoint.
const ELABORATION_SWEEP_INTERVAL: Duration = Duration::from_secs(300);

/// The authoring lane's only driver. It runs when the cognition organ opened at
/// all, which is the config gate the runtime already has: no mode flag joins it.
/// One sequential sweep per wake, because a boundary binds to its own digest and
/// the loops are logically independent without being separate tasks.
async fn elaborate_world(state: AppState) {
    let mut interval = tokio::time::interval(ELABORATION_SWEEP_INTERVAL);
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    interval.tick().await;
    loop {
        interval.tick().await;
        let Some(runner) = state
            .controllers
            .as_deref()
            .filter(|_| !state.controller_quarantined.load(Ordering::SeqCst))
            .map(ControllerRunner::elaborator)
        else {
            continue;
        };
        if let Err(error) = runner.sweep().await {
            tracing::debug!(%error, "elaboration sweep did not complete");
        }
    }
}

/// The compute budget, read at open exactly as the model names are. It is not
/// world data and does not live beside `WorldScaleIntent`, which is the authored
/// target count of goal-bearing subjects: the two are numerically related and
/// have different owners. Changing either number is a restart.
fn configured_cover_budget() -> anyhow::Result<CoverBudget> {
    let read = |name: &str, default: u16| -> u16 {
        std::env::var(name)
            .ok()
            .and_then(|value| value.trim().parse().ok())
            .unwrap_or(default)
    };
    CoverBudget {
        cells: read("GHOSTLIGHT_COVER_CELL_BUDGET", 240),
        constituent_cap: read("GHOSTLIGHT_COVER_CONSTITUENT_CAP", 24),
        urgency_slots: read("GHOSTLIGHT_COVER_URGENCY_SLOTS", 36),
    }
    .validated()
    .context("cover budget configuration is not usable")
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

fn configured_tick_interval() -> Duration {
    std::env::var("GHOSTLIGHT_TICK_INTERVAL_SECONDS")
        .ok()
        .and_then(|value| value.trim().parse::<u64>().ok())
        .filter(|value| *value >= 1)
        .map_or(CLOCK_TICK_INTERVAL, Duration::from_secs)
}

/// The world's only driver: one owner of the tick's cadence, of the cover, and
/// of the clock. It derives a cover from one snapshot, runs each cell under a
/// permit, and then advances the clock — after the cells, so every cell in a
/// tick sees the same `now` and therefore the same tick index, which is what
/// makes the derived command ids stable and the rotation phase well defined.
///
/// The kernel learns one `AdvanceTime` and `0..N` ordinary one-opportunity
/// submissions. It never learns a tick happened.
async fn drive_cover_tick(state: AppState, interval: Duration) {
    let Some(minutes) = TickMinutes::new(CLOCK_TICK_MINUTES) else {
        tracing::error!("the configured clock tick is outside one year of minutes");
        return;
    };
    let mut ticker = tokio::time::interval(interval);
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    ticker.tick().await;
    loop {
        ticker.tick().await;
        drive_one_tick(
            minutes,
            || run_cover_tick(&state),
            |id, minutes| state.world.submit_clock(id, minutes),
        )
        .await;
        if let Err(error) = publish_projection(&state).await {
            tracing::debug!(%error, "world revision could not be read after a tick");
        }
    }
}

/// One tick's ordering, narrowed the same way `submit_clock_tick` narrows the
/// clock port: `run_cover` is opaque here, never `run_cover_tick` by name, so a
/// test can substitute a recording fake for the whole cognition organ and
/// observe call order without a controller runner, a mailbox, or tokio's
/// timer. Sequencing, not concurrency, is the invariant this buys: the cover
/// always finishes before the clock is asked to advance, which is what makes
/// every cell's derived tick index agree with the `AdvanceTime` that follows
/// it.
async fn drive_one_tick<RunCover, RunCoverFut, SubmitClock, SubmitClockFut>(
    minutes: TickMinutes,
    run_cover: RunCover,
    submit_clock: SubmitClock,
) where
    RunCover: FnOnce() -> RunCoverFut,
    RunCoverFut: std::future::Future<Output = ()>,
    SubmitClock: FnOnce(CommandId, TickMinutes) -> SubmitClockFut,
    SubmitClockFut: std::future::Future<Output = Result<SubmitReceipt, MailboxError>>,
{
    run_cover().await;
    submit_clock_tick(minutes, submit_clock).await;
}

/// One tick's cognition, apart from the clock and the wall clock that decides
/// when to run it.
async fn run_cover_tick(state: &AppState) {
    let Some(runner) = state.controllers.clone() else {
        return;
    };
    if state.controller_quarantined.load(Ordering::SeqCst) {
        return;
    }
    let (Ok(snapshot), Ok(graph)) = (
        state.world.snapshot().await,
        state.world.agency_graph().await,
    ) else {
        return;
    };
    let cover = derive_cover(
        snapshot.world_id,
        snapshot.now,
        CLOCK_TICK_MINUTES,
        &snapshot.opportunities,
        &graph,
        state.cover_budget,
    );
    *state.cover.lock().await = Some(CoverSummary::of(&cover));

    let mut running = tokio::task::JoinSet::new();
    for cell in cover.cells {
        let runner = runner.clone();
        let permits = state.controller_permits.clone();
        let quarantined = state.controller_quarantined.clone();
        running.spawn(async move {
            // Checked once up front, as a fast path that skips contending for
            // a permit at all, and once more after acquiring one: a cell
            // already parked behind the pool when a sibling's fault raises
            // the flag must not proceed just because it queued before the
            // flag flipped. A permit already held when the flag is set still
            // finishes on its own binding — quarantine stops the *next* cell
            // to reach either check, not one already mid-turn.
            if quarantined.load(Ordering::SeqCst) {
                return;
            }
            let Ok(_permit) = permits.acquire().await else {
                return;
            };
            if quarantined.load(Ordering::SeqCst) {
                return;
            }
            match runner.run_cell(&cell).await {
                // A tick that reports nothing is a tick nobody can debug. One
                // line per coarse cell, naming what it consumed and what it
                // could not: a batch buys one inference, never one admission
                // rule, and the gap between turns offered and turns committed
                // is the number that says so.
                Ok(CellRun::Grouped(grouped)) => {
                    let committed = grouped
                        .submissions
                        .iter()
                        .filter(|entry| {
                            matches!(
                                entry.submission,
                                SubmissionDisposition::Completed(_)
                                    | SubmissionDisposition::PreviouslyConfirmed(_)
                            )
                        })
                        .count();
                    tracing::debug!(
                        cell = %grouped.cell,
                        resolution = ?grouped.resolution,
                        submitted = grouped.submissions.len(),
                        committed,
                        needs = grouped.needs.len(),
                        pending = ?grouped.pending,
                        "a coarse cell finished"
                    );
                }
                Ok(CellRun::Narrative(NarrativeRun::Pending(pending))) => {
                    tracing::debug!(
                        reason = ?pending.reason(),
                        "a detail narrative cell is pending"
                    );
                }
                Ok(CellRun::Operational(OperationalRun::Pending(pending))) => {
                    tracing::debug!(
                        reason = ?pending.reason(),
                        "a detail operational cell is pending"
                    );
                }
                Ok(CellRun::Narrative(_) | CellRun::Operational(_)) => {}
                Err(error) => {
                    // A cell that faults does not abort the tick. Custody loss
                    // does: the work journal is one lock and one custody claim,
                    // so it is a tick-level outcome rather than a per-cell one.
                    if error.requires_quarantine() {
                        tracing::error!(%error, "controller cognition quarantined mid-tick");
                        quarantined.store(true, Ordering::SeqCst);
                    } else {
                        tracing::debug!(%error, "a cell did not complete this tick");
                    }
                }
            }
        });
    }
    while running.join_next().await.is_some() {}
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

    fn controller_commit() -> crate::world::CommitReceipt {
        crate::world::CommitReceipt {
            command_id: CommandId::new(),
            resulting_revision: 9,
            resulting_state_digest: "sha256:declined-state".into(),
            commit_digest: "sha256:decline-commit".into(),
        }
    }

    #[test]
    fn no_proposal_projection_carries_the_canonical_world_commit() {
        let commit = controller_commit();
        let (applied, changed) = controller_submission(SubmissionDisposition::NoProposal(
            SubmitReceipt::Applied(commit.clone()),
        ));
        assert!(changed);
        assert_eq!(applied["kind"], "no_proposal");
        assert_eq!(applied["commit"]["kind"], "applied");
        assert_eq!(applied["commit"]["revision"], 9);

        let (replayed, changed) = controller_submission(SubmissionDisposition::NoProposal(
            SubmitReceipt::AlreadyApplied(commit),
        ));
        assert!(!changed);
        assert_eq!(replayed["kind"], "no_proposal");
        assert_eq!(replayed["commit"]["kind"], "already_applied");
    }

    struct Fixture {
        _directory: tempfile::TempDir,
        state: AppState,
        cookie: String,
    }

    /// A real CodexConnector behind the fixture, read from the environment
    /// the production runtime reads. Only the ignored live smoke uses it.
    struct LiveController {
        endpoint: SocketAddr,
        credential: PathBuf,
        runtime_id: String,
        models: ControllerModels,
    }

    async fn fixture() -> Fixture {
        fixture_with(None).await
    }

    async fn fixture_with(live: Option<LiveController>) -> Fixture {
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
        let live = live.unwrap_or_else(|| LiveController {
            endpoint: "127.0.0.1:9".parse().unwrap(),
            credential: controller_key.clone(),
            runtime_id: "ghostlight-runtime-test".into(),
            models: ControllerModels {
                projector: "gpt-5.6-luna".into(),
                persona: "gpt-5.6-sol".into(),
                interpreter: "gpt-5.6-terra".into(),
                operational_agent: "gpt-5.6-terra".into(),
                elaborator: "gpt-5.6-terra".into(),
            },
        });
        let controllers = ControllerRunner::open(
            world.clone(),
            live.endpoint,
            &live.credential,
            live.runtime_id,
            directory.path().join("controller-work.cc"),
            live.models,
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
            controller_permits: Arc::new(Semaphore::new(TEST_CONTROLLER_CONCURRENCY)),
            controller_quarantined: Arc::new(AtomicBool::new(false)),
            cover_budget: CoverBudget {
                cells: 240,
                constituent_cap: 24,
                urgency_slots: 36,
            },
            cover: Arc::new(Mutex::new(None)),
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
    /// `world::consumer`, which is tested there.
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
                "ghostlight.world_create.v2",
                0,
                json!({
                    "title":"Cutover World",
                    "subject_label":"Operator",
                    "targets":{},
                    "jurisdictions":[]
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

    #[tokio::test]
    async fn exact_stale_retry_returns_journal_receipt_without_app_cache() {
        let fixture = fixture().await;
        let id = uuid::Uuid::new_v4().to_string();
        let command = invocation(
            "world.create",
            "ghostlight.world_create.v2",
            0,
            json!({
                "title":"Retry World",
                "subject_label":"Operator",
                "targets":{},
                "jurisdictions":[]
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
        let result = post(
            &fixture.state,
            &fixture.cookie,
            invocation(
                "session_zero.begin",
                "ghostlight.session_zero_begin.v1",
                0,
                json!({}),
                &uuid::Uuid::new_v4().to_string(),
            ),
        )
        .await;
        assert_eq!(result["state"], "denied");
        assert!(
            result["message"]
                .as_str()
                .unwrap()
                .contains("not advertised")
        );
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

    /// One tick submits exactly the configured span, and nothing else: the
    /// captured argument is the `TickMinutes` `submit_clock_tick` was given,
    /// with no measured elapsed duration — no `Duration`, no `Instant` — ever
    /// constructed along the way. Driven through the narrow port rather than
    /// `drive_cover_tick` itself, so the assertion holds without waiting on
    /// tokio's timer or on a cognition organ.
    #[tokio::test]
    async fn a_clock_tick_submits_the_configured_span_and_nothing_measured() {
        let minutes = TickMinutes::new(CLOCK_TICK_MINUTES).expect("a valid configured tick");
        let captured: std::sync::Arc<std::sync::Mutex<Option<TickMinutes>>> =
            std::sync::Arc::new(std::sync::Mutex::new(None));
        let sink = captured.clone();
        submit_clock_tick(minutes, move |_id, submitted| {
            *sink.lock().unwrap() = Some(submitted);
            std::future::ready(Ok(SubmitReceipt::AlreadyApplied(controller_commit())))
        })
        .await;
        let submitted = captured.lock().unwrap().expect("the tick submitted a span");
        assert_eq!(submitted.minutes(), CLOCK_TICK_MINUTES);
    }

    use crate::world::{
        ControllerWork, ControllerWorkLookup, ControllerWorkStore, ControllerWorkStoreError,
        ControllerWorkWrite, InferenceFault, InferenceOutput, InferencePort, InferenceRequest,
        PreparedInference, fixture_inference_output, fixture_prepared_inference,
    };
    use std::sync::atomic::AtomicUsize;

    /// Builds an active world with exactly two controller-bearing subjects — a
    /// narrative persona and an operational agent — beside its one Human
    /// subject. `derive_cover` skips the Human opportunity, so this cover is
    /// always exactly two singleton cells: the most `world.create`'s own
    /// intent can produce without reaching past the production ingress
    /// surface into the kernel's private command types the rest of this
    /// module deliberately does not name.
    async fn active_two_cell_world(state: &AppState, cookie: &str) {
        two_cell_world(state, cookie, BTreeMap::new(), Vec::new(), true).await;
    }

    /// The same genesis, with an authored scale intent and without the
    /// approve/activate pair, so a test can look at the Draft world the seed
    /// lane actually runs against.
    async fn two_cell_world(
        state: &AppState,
        cookie: &str,
        targets: BTreeMap<SubjectKind, u32>,
        jurisdictions: Vec<CreateJurisdictionIntent>,
        activate: bool,
    ) {
        let principal = state
            .sessions
            .lock()
            .await
            .account_for_cookie(cookie, Utc::now())
            .unwrap()
            .expect("the fixture cookie names a live session");
        let receipt = state
            .world
            .create(
                CreateWorldIntent {
                    id: CommandId::new(),
                    title: "Cover Tick Fixture".into(),
                    human_subject_label: "Operator".into(),
                    narrative_persona_label: Some("Persona".into()),
                    operational_agent_label: Some("Operational Agent".into()),
                    targets,
                    jurisdictions,
                },
                &principal,
            )
            .await
            .unwrap();
        let mut snapshot = state.world.snapshot().await.unwrap();
        if !activate {
            return;
        }
        for body in [CommandBody::ApproveDraft, CommandBody::ActivateWorld] {
            state
                .world
                .submit_principal(
                    PrincipalCommandIntent {
                        id: CommandId::new(),
                        world_id: receipt.world_id,
                        expected_revision: snapshot.revision,
                        body,
                    },
                    &principal,
                )
                .await
                .unwrap();
            snapshot = state.world.snapshot().await.unwrap();
        }
    }

    fn test_controller_models() -> ControllerModels {
        ControllerModels {
            projector: "projector".into(),
            persona: "persona".into(),
            interpreter: "interpreter".into(),
            operational_agent: "operator".into(),
            elaborator: "elaborator".into(),
        }
    }

    /// A store that never remembers anything: every command looks unwritten
    /// and every write lands clean. Sufficient for tests whose subject is the
    /// tick driver's permit and quarantine handling rather than checkpoint
    /// resumption.
    struct AlwaysFreshWorkStore;

    #[async_trait::async_trait]
    impl ControllerWorkStore for AlwaysFreshWorkStore {
        async fn lookup(
            &self,
            _command_id: CommandId,
        ) -> Result<ControllerWorkLookup, ControllerWorkStoreError> {
            Ok(ControllerWorkLookup::Missing)
        }

        async fn persist(
            &self,
            _work: &ControllerWork,
        ) -> Result<ControllerWorkWrite, ControllerWorkStoreError> {
            Ok(ControllerWorkWrite::Applied)
        }

        async fn custody_probe(&self) -> Result<ControllerWorkCustody, ControllerWorkStoreError> {
            Ok(ControllerWorkCustody::Owned {
                narrative_commands: 0,
                operational_commands: 0,
                elaboration_commands: 0,
                seed_commands: 0,
            })
        }
    }

    /// Counts concurrent `infer` calls. Each call increments an in-flight
    /// counter, records the running high-water mark, yields once so a
    /// concurrently spawned cell gets a chance to run, then decrements. If
    /// `run_cover_tick` ever let two cells' inference calls overlap, this
    /// would observe it.
    struct CountingInferencePort {
        in_flight: AtomicUsize,
        high_water: AtomicUsize,
        calls: AtomicUsize,
    }

    impl CountingInferencePort {
        fn new() -> Self {
            Self {
                in_flight: AtomicUsize::new(0),
                high_water: AtomicUsize::new(0),
                calls: AtomicUsize::new(0),
            }
        }
    }

    #[async_trait::async_trait]
    impl InferencePort for CountingInferencePort {
        fn prepare(&self, request: InferenceRequest) -> Result<PreparedInference, InferenceFault> {
            fixture_prepared_inference(request)
        }

        async fn infer(
            &self,
            _request: PreparedInference,
        ) -> Result<InferenceOutput, InferenceFault> {
            let in_flight = self.in_flight.fetch_add(1, Ordering::SeqCst) + 1;
            self.calls.fetch_add(1, Ordering::SeqCst);
            self.high_water.fetch_max(in_flight, Ordering::SeqCst);
            tokio::task::yield_now().await;
            self.in_flight.fetch_sub(1, Ordering::SeqCst);
            Ok(fixture_inference_output(
                "The cover tick fixture speaks.",
                "counting-port",
            ))
        }
    }

    /// The tick driver spawns one task per cell into one `JoinSet` up front,
    /// each gated by `state.controller_permits` before it may call the port.
    /// This proves that gate is load-bearing: with the pool sized to one and
    /// a cover of two singleton cells (the most a genesis world's two
    /// controller-bearing subjects can produce), the counting port must never
    /// observe a second concurrent `infer` call while the first is still
    /// in flight, even though both cells were spawned before either ran.
    #[tokio::test]
    async fn the_tick_driver_never_exceeds_its_controller_permit_pool() {
        let fixture = fixture().await;
        active_two_cell_world(&fixture.state, &fixture.cookie).await;

        let port = Arc::new(CountingInferencePort::new());
        let mut state = fixture.state.clone();
        state.controllers = Some(Arc::new(ControllerRunner::with_test_ports(
            state.world.clone(),
            port.clone(),
            Arc::new(AlwaysFreshWorkStore),
            test_controller_models(),
        )));
        state.controller_permits = Arc::new(Semaphore::new(1));

        run_cover_tick(&state).await;

        assert!(
            port.calls.load(Ordering::SeqCst) >= 2,
            "both singleton cells should have reached the port at least once"
        );
        assert_eq!(
            port.high_water.load(Ordering::SeqCst),
            1,
            "a permit pool of one must never admit a second concurrent call"
        );
    }

    /// Raises `ControllerError::requires_quarantine` on every call. Used to
    /// prove the tick driver's quarantine edge rather than any cognition
    /// outcome.
    struct QuarantiningInferencePort {
        calls: AtomicUsize,
    }

    #[async_trait::async_trait]
    impl InferencePort for QuarantiningInferencePort {
        fn prepare(&self, request: InferenceRequest) -> Result<PreparedInference, InferenceFault> {
            fixture_prepared_inference(request)
        }

        async fn infer(
            &self,
            _request: PreparedInference,
        ) -> Result<InferenceOutput, InferenceFault> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Err(InferenceFault::fixture_integrity_violation(
                "the fixture port disputes every receipt",
            ))
        }
    }

    /// `run_cover_tick`'s per-cell task checks `controller_quarantined` twice:
    /// once up front, before contending for a permit at all, and once more
    /// right after acquiring one. With the pool sized to one, only one of
    /// this tick's two cells can ever hold the permit at a time, and the
    /// second cannot be granted it until the first has fully returned —
    /// which, for a faulting cell, means the flag is already set. So the
    /// second cell's post-acquire check always sees it and returns without
    /// ever reaching the port, deterministically, regardless of which cell
    /// happened to acquire first. The flag's cross-tick teeth are separate:
    /// `run_cover_tick`'s own top-of-function guard means the *next* tick
    /// never derives a cover at all once quarantined, so no cell of any later
    /// tick reaches the port either. Both edges are asserted here.
    #[tokio::test]
    async fn quarantine_raised_mid_tick_stops_the_sibling_cell_and_every_later_tick() {
        let fixture = fixture().await;
        active_two_cell_world(&fixture.state, &fixture.cookie).await;

        let port = Arc::new(QuarantiningInferencePort {
            calls: AtomicUsize::new(0),
        });
        let mut state = fixture.state.clone();
        state.controllers = Some(Arc::new(ControllerRunner::with_test_ports(
            state.world.clone(),
            port.clone(),
            Arc::new(AlwaysFreshWorkStore),
            test_controller_models(),
        )));
        // One permit, two cells: the second cell cannot even attempt the
        // port until the first has returned, which is what makes the
        // post-acquire recheck deterministic rather than a race.
        state.controller_permits = Arc::new(Semaphore::new(1));

        run_cover_tick(&state).await;
        assert!(
            state.controller_quarantined.load(Ordering::SeqCst),
            "an integrity-violating fault must quarantine the cognition organ"
        );
        let calls_after_first_tick = port.calls.load(Ordering::SeqCst);
        assert_eq!(
            calls_after_first_tick, 1,
            "the sibling cell waiting on the permit must not reach the port \
             once the first cell's fault raised the flag"
        );

        run_cover_tick(&state).await;
        assert_eq!(
            port.calls.load(Ordering::SeqCst),
            calls_after_first_tick,
            "a quarantined organ must not let a later tick's cells reach the port"
        );
    }

    /// `drive_one_tick` never names `run_cover_tick`: `run_cover` is opaque to
    /// it, so this test substitutes a fake that walks a real `Cover` (three
    /// singleton cells, from three fabricated opportunities) and records each
    /// cell's tick index, alongside a fake clock submitter that records its
    /// own call. Asserts the ordering invariant `drive_one_tick` exists to
    /// buy — every cell recorded before the clock — and that every recorded
    /// cell carries the one tick index `derive_cover` stamped on the whole
    /// cover, matching `drive_cover_tick`'s own doc comment.
    #[tokio::test]
    async fn drive_one_tick_runs_every_cell_before_the_clock_and_all_share_one_tick() {
        use crate::world::{
            AgencyGraph, Cell, ControllerMode, FictionalMinutes, TickIndex,
            fixture_controller_opportunities,
        };

        let opportunities = fixture_controller_opportunities(&[
            ControllerMode::NarrativePersona,
            ControllerMode::OperationalAgent,
            ControllerMode::OperationalAgent,
        ]);
        let world_id = opportunities[0].world_id;
        let now = FictionalMinutes(u64::from(CLOCK_TICK_MINUTES) * 7);
        let cover = derive_cover(
            world_id,
            now,
            CLOCK_TICK_MINUTES,
            &opportunities,
            &AgencyGraph::default(),
            CoverBudget {
                cells: 240,
                constituent_cap: 24,
                urgency_slots: 36,
            },
        );
        assert_eq!(
            cover.cells.len(),
            3,
            "three distinct subjects should derive three singleton cells"
        );

        #[derive(Debug, Clone, PartialEq, Eq)]
        enum Event {
            Cell(TickIndex),
            Clock,
        }
        let order: Arc<Mutex<Vec<Event>>> = Arc::new(Mutex::new(Vec::new()));

        let cell_order = order.clone();
        let cells = cover.cells.clone();
        let run_cover = move || {
            let order = cell_order.clone();
            let cells = cells.clone();
            async move {
                for cell in cells {
                    let tick = match cell {
                        Cell::Singleton { tick, .. } | Cell::Group { tick, .. } => tick,
                    };
                    order.lock().await.push(Event::Cell(tick));
                }
            }
        };
        let clock_order = order.clone();
        let minutes = TickMinutes::new(CLOCK_TICK_MINUTES).expect("a valid configured tick");
        drive_one_tick(minutes, run_cover, move |_id, _minutes| {
            let order = clock_order.clone();
            async move {
                order.lock().await.push(Event::Clock);
                Ok(SubmitReceipt::AlreadyApplied(controller_commit()))
            }
        })
        .await;

        let recorded = order.lock().await.clone();
        let (cell_events, clock_events) = recorded.split_at(recorded.len() - 1);
        assert_eq!(
            clock_events,
            [Event::Clock],
            "the clock must be the last thing recorded"
        );
        assert_eq!(cell_events.len(), 3, "every cell must have run");
        assert!(
            cell_events
                .iter()
                .all(|event| matches!(event, Event::Cell(tick) if *tick == cover.tick)),
            "every cell in the tick must carry the same tick index"
        );
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

        let previous = std::env::var(crate::world::CONSUMER_CREDENTIALS_ENVIRONMENT).ok();
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("consumers.cc");
        std::fs::write(&path, b"not a consumer credentials file").unwrap();
        // SAFETY: serialized by ENV_LOCK above; no other thread in this test
        // binary reads or writes this variable concurrently.
        unsafe {
            std::env::set_var(
                crate::world::CONSUMER_CREDENTIALS_ENVIRONMENT,
                path.as_os_str(),
            );
        }
        let result = open_consumer_registry();
        // SAFETY: same lock, restoring (or clearing) the prior value.
        unsafe {
            match &previous {
                Some(value) => {
                    std::env::set_var(crate::world::CONSUMER_CREDENTIALS_ENVIRONMENT, value)
                }
                None => std::env::remove_var(crate::world::CONSUMER_CREDENTIALS_ENVIRONMENT),
            }
        }
        assert!(
            result.is_err(),
            "a malformed credentials file must refuse the registry, not start empty"
        );
    }

    /// Spec test 18. The local live smoke: a world created with a real scale
    /// intent, seeded from a real Vault until its deficit is zero or the
    /// session budget is spent, then approved, activated, and ticked by the
    /// production tick driver, elaboration sweep, and clock against a real
    /// CodexConnector. Everything else in this module proves the machine under
    /// fixture ports; this is the one place the road is tested. It asserts only
    /// that the loop ran; the log is the deliverable — patches committed,
    /// deficit per round, subjects qualified, and the prose a tick produces
    /// from a world that now has people in it.
    #[tokio::test]
    #[ignore = "requires a running CodexConnector and a vault; see GHOSTLIGHT_SMOKE_* environment"]
    async fn live_smoke_seeds_then_ticks_a_world_against_the_connector() {
        let env = |name: &str| std::env::var(name).unwrap_or_else(|_| panic!("{name} is required"));
        let live = LiveController {
            endpoint: env("GHOSTLIGHT_CONTROLLER_CONNECTOR").parse().unwrap(),
            credential: PathBuf::from(env("GHOSTLIGHT_CONTROLLER_CREDENTIAL")),
            runtime_id: env("GHOSTLIGHT_ACCEPTANCE_RUNTIME_ID"),
            models: ControllerModels {
                projector: env("GHOSTLIGHT_CONTROLLER_PROJECTOR_MODEL"),
                persona: env("GHOSTLIGHT_CONTROLLER_PERSONA_MODEL"),
                interpreter: env("GHOSTLIGHT_CONTROLLER_INTERPRETER_MODEL"),
                operational_agent: env("GHOSTLIGHT_CONTROLLER_OPERATIONAL_MODEL"),
                elaborator: env("GHOSTLIGHT_CONTROLLER_ELABORATOR_MODEL"),
            },
        };
        let ticks: u32 = env("GHOSTLIGHT_SMOKE_TICKS").parse().unwrap();
        let log_path = PathBuf::from(env("GHOSTLIGHT_SMOKE_LOG"));
        let mut log = std::fs::File::create(&log_path).unwrap();
        use std::io::Write as _;
        let mut line = |text: String| {
            writeln!(log, "{} {text}", Utc::now().to_rfc3339()).unwrap();
            log.flush().unwrap();
        };

        let seed_sessions: usize = env("GHOSTLIGHT_SMOKE_SEED_SESSIONS").parse().unwrap();
        let seed_target: u32 = env("GHOSTLIGHT_SMOKE_SEED_TARGET").parse().unwrap();
        let vault_scope = std::env::var("GHOSTLIGHT_SMOKE_VAULT_SCOPE").unwrap_or_default();
        let brief = std::env::var("GHOSTLIGHT_SMOKE_SEED_BRIEF").ok();
        // Read here rather than only inside `seed_once`, so a misconfigured run
        // fails at the top instead of after the first paid session.
        let _ = env(SEED_VAULT_ROOT_ENVIRONMENT);

        let fixture = fixture_with(Some(live)).await;
        let state = &fixture.state;
        two_cell_world(
            state,
            &fixture.cookie,
            BTreeMap::from([(SubjectKind::Person, seed_target)]),
            vec![CreateJurisdictionIntent {
                handle: "seed_root".into(),
                label: env("GHOSTLIGHT_SMOKE_SEED_ROOT_LABEL"),
                permille: 1000,
            }],
            false,
        )
        .await;

        let principal = state
            .sessions
            .lock()
            .await
            .account_for_cookie(&fixture.cookie, Utc::now())
            .unwrap()
            .expect("the fixture cookie names a live session");
        let draft = state.world.snapshot().await.unwrap();
        line(format!(
            "draft world={:?} revision={} deficit_rows={} shortfall={:?}",
            draft.world_id,
            draft.revision,
            draft.scale_deficit.len(),
            crate::world::select_row(&draft)
        ));
        for round in 1..=seed_sessions {
            let before = state.world.snapshot().await.unwrap();
            if crate::world::select_row(&before).is_none() {
                line(format!("seed round {round} skipped: no shortfall left"));
                break;
            }
            let started = std::time::Instant::now();
            let outcome = seed_once(
                state,
                &principal,
                &before,
                SeedPayload {
                    vault_scope: vault_scope.clone(),
                    brief: brief.clone(),
                },
            )
            .await;
            let after = state.world.snapshot().await.unwrap();
            line(format!(
                "seed round {round} took={:?} outcome={:?} revision {}->{} patches={} qualified={} rows={:?}",
                started.elapsed(),
                outcome.as_ref().map(|value| value.name()),
                before.revision,
                after.revision,
                after.revision.saturating_sub(1),
                after
                    .subjects
                    .iter()
                    .filter(|subject| subject.qualified)
                    .count(),
                after
                    .scale_deficit
                    .iter()
                    .map(|row| (row.kind, row.target, row.qualified, row.deficit))
                    .collect::<Vec<_>>()
            ));
            if !matches!(outcome, Ok(SeedOutcome::Committed)) {
                break;
            }
        }
        for subject in &state.world.snapshot().await.unwrap().subjects {
            line(format!(
                "  seeded {} kind={:?} mode={:?} grants={} qualified={}",
                subject.label,
                subject.kind,
                subject.controller_mode,
                subject.affordances.len(),
                subject.qualified
            ));
        }

        for body in [CommandBody::ApproveDraft, CommandBody::ActivateWorld] {
            let snapshot = state.world.snapshot().await.unwrap();
            state
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
        let snapshot = state.world.snapshot().await.unwrap();
        assert_eq!(snapshot.phase, WorldPhase::Active);
        line(format!(
            "activated world={:?} revision={} phase={:?} subjects={} opportunities={} now={:?} boundaries={} deficit_rows={}",
            snapshot.world_id,
            snapshot.revision,
            snapshot.phase,
            snapshot.subjects.len(),
            snapshot.opportunities.len(),
            snapshot.now,
            snapshot.boundaries.len(),
            snapshot.scale_deficit.len()
        ));
        for subject in &snapshot.subjects {
            line(format!(
                "  subject {} kind={:?} mode={:?} affordances={}",
                subject.label,
                subject.kind,
                subject.controller_mode,
                subject.affordances.len()
            ));
        }
        let runner = state.controllers.clone().expect("a live controller runner");
        let mut logged_events = 0usize;
        for tick in 1..=ticks {
            let started = std::time::Instant::now();
            let before = state.world.snapshot().await.unwrap().revision;
            drive_one_tick(
                TickMinutes::new(CLOCK_TICK_MINUTES).unwrap(),
                || run_cover_tick(state),
                |id, minutes| state.world.submit_clock(id, minutes),
            )
            .await;
            let cover = *state.cover.lock().await;
            let elaboration = runner.elaborator().sweep().await;
            let after = state.world.snapshot().await.unwrap();
            line(format!(
                "tick {tick} took={:?} revision {before}->{} now={:?} cover={cover:?} quarantined={} elaboration={elaboration:?} boundaries={} deficit_rows={} subjects={}",
                started.elapsed(),
                after.revision,
                after.now,
                state.controller_quarantined.load(Ordering::SeqCst),
                after.boundaries.len(),
                after.scale_deficit.len(),
                after.subjects.len()
            ));
            let events = state.world.operator_log().await.unwrap();
            for event in events.iter().skip(logged_events) {
                line(format!(
                    "  r{} {}: {}",
                    event.revision,
                    event.speaker_label,
                    event
                        .speech
                        .as_ref()
                        .map(Statement::as_str)
                        .unwrap_or("<no speech>")
                ));
            }
            logged_events = events.len();
            if let Ok(custody) = runner.custody_probe().await {
                line(format!("  custody {custody:?}"));
            }
        }
        let final_snapshot = state.world.snapshot().await.unwrap();
        line(format!(
            "done revision={} state_digest={} events={}",
            final_snapshot.revision, final_snapshot.state_digest, logged_events
        ));
        assert!(
            final_snapshot.revision > snapshot.revision,
            "the clock alone must move the revision"
        );
    }

    // ---- The seed command ------------------------------------------------

    /// Spec test 1. `world_create.v1` is not kept alive beside v2: an
    /// invocation announcing it dies at validation before any handler runs, and
    /// a payload that announces v2 but omits the scale target is a payload
    /// error rather than a defaulted empty intent. Neither creates a world.
    #[tokio::test]
    async fn a_v1_create_payload_is_refused() {
        let fixture = fixture().await;
        let stale = post(
            &fixture.state,
            &fixture.cookie,
            invocation(
                "world.create",
                "ghostlight.world_create.v1",
                0,
                json!({"title":"Stale World","subject_label":"Operator"}),
                &uuid::Uuid::new_v4().to_string(),
            ),
        )
        .await;
        assert_eq!(stale["state"], "denied");
        assert!(current_world(&fixture.state).await.unwrap().is_none());

        let partial = post(
            &fixture.state,
            &fixture.cookie,
            invocation(
                "world.create",
                "ghostlight.world_create.v2",
                0,
                json!({"title":"Half World","subject_label":"Operator"}),
                &uuid::Uuid::new_v4().to_string(),
            ),
        )
        .await;
        assert_eq!(partial["state"], "denied");
        assert!(
            current_world(&fixture.state).await.unwrap().is_none(),
            "a v2 payload with no scale target created a world anyway"
        );
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
        let cover = eve::CoverPanel {
            cells: 240,
            constituent_cap: 24,
            urgency_slots: 36,
            last: None,
        };

        let mut surfaces = vec![eve::authenticated_surface(&owner, None, &[], &cover).unwrap()];
        two_cell_world(
            &fixture.state,
            &fixture.cookie,
            BTreeMap::from([(SubjectKind::Person, 4)]),
            vec![CreateJurisdictionIntent {
                handle: "sere".into(),
                label: "The Low Sere".into(),
                permille: 1000,
            }],
            false,
        )
        .await;
        let draft = fixture.state.world.snapshot().await.unwrap();
        assert_eq!(draft.phase, WorldPhase::Draft);
        for account in [owner.as_str(), stranger] {
            surfaces.push(eve::authenticated_surface(account, Some(&draft), &[], &cover).unwrap());
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
            surfaces.push(eve::authenticated_surface(account, Some(&active), &[], &cover).unwrap());
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
            false,
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
            false,
        )
        .await;
        let draft = fixture.state.world.snapshot().await.unwrap();
        let surface = eve::authenticated_surface(
            &owner,
            Some(&draft),
            &[],
            &eve::CoverPanel {
                cells: 240,
                constituent_cap: 24,
                urgency_slots: 36,
                last: None,
            },
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
        let selected = crate::world::select_row(&draft).expect("a shortfall to answer");
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
}
