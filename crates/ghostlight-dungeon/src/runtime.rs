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
    play::{Admission, PlayRequest, PlayTable, PlayTurnState, PlayTurnView},
};
use ghostlight::{
    CONSUMER_BODY_LIMIT, CommandBody, CommandId, ConnectorBinding, ConsumerPort,
    ConsumerRegistry, ControllerError, ControllerModels, ControllerPort, ControllerRunner, ControllerWorkCustody,
    CreateJurisdictionIntent, CreateWorldIntent, DEFAULT_LOCAL_MODEL_PREFIX,
    DEFAULT_SDK_MODEL_PREFIX, KernelError, Lens,
    LensWeights, LocalBinding, MailboxError, PersonaLane, PrincipalCommandIntent, PrincipalId,
    SdkBinding, SeedOutcome, SeedPort, SubjectKind, SubmitReceipt, TickMinutes,
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
    /// quota. Cut 1 deleted Dungeon's own drivers, so the play table's own
    /// `execute_dispatch` and `close_turn` narration (Cut 8b) are this
    /// pool's one production consumer now — Persona dispatch *is* the
    /// controller work, not a second lane beside it. `runtime_readiness`'s
    /// `controllerStatus` "active" arm reads this pool for exactly that
    /// reason: live Persona cognition drawing a permit is what "active"
    /// reports, not only the test route's own forced exhaustion.
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
    #[serde(deserialize_with = "deserialize_json_capture")]
    targets: BTreeMap<SubjectKind, u32>,
    /// The jurisdiction roots, declared by genesis beside the commons because
    /// `resolve_patch` only resolves roots the same patch declares. A duplicate
    /// handle and a permille sum over 1000 are refused by the resolver, not
    /// pre-checked here: a pre-check would be a second reducer.
    #[serde(deserialize_with = "deserialize_json_capture")]
    jurisdictions: Vec<CreateJurisdiction>,
    /// Required, and must draw: a payload that omits it is refused, which is
    /// what `world_create.v4` means. An unknown lens name is refused by
    /// deserialization before any handler runs.
    #[serde(deserialize_with = "deserialize_json_capture")]
    lens_weights: LensWeights,
}

/// Cut 10 (PA.f149): the vendored Eve browser lowering's own
/// `createEveCommandIntent` (`vendor/eve/packages/eve-browser-lowering`)
/// sends every captured `captureBindings` value enveloped under
/// `payload.bindings` — the documented shape (Eve's own
/// `docs/surface-contract-v1.md`: "Operations declare captureBindings and
/// receive those values under payload.bindings"), confirmed by driving the
/// real lowering over a real Dungeon surface (`tools/eve_client_bridge.mjs`).
/// Dungeon's own payload structs are flat. Rather than teach every payload
/// struct the envelope, this unwraps it once, so a real client's payload
/// deserializes exactly like the flat, hand-built payloads the existing
/// tests already send. A payload with no `bindings` object (no
/// `captureBindings` advertised at all, e.g. `world.approve`/
/// `world.activate`, whose actions capture nothing) passes through
/// unchanged — `commandPayload` on the lowering's own side never sets
/// `bindings` when `captureBindings` is empty.
/// PA.f168's original rule refused any sibling next to the envelope
/// `bindings` object outright. PA.f170 changes what a real client's payload
/// carries: a button's own `props.action` fields (Cut 13's per-question
/// answer token, for `world.play`) ride the *same* top-level payload object
/// `commandPayload` builds — `{...action, bindings: {...captured}}` — so a
/// sibling next to `bindings` is now the shape the real lowering produces on
/// purpose, not a mismatch to surface. This merges the two into one flat
/// object instead: `bindings`' own fields plus every sibling field, so a
/// payload struct's ordinary `#[serde(deny_unknown_fields)]` still catches a
/// genuinely unknown field exactly as it did before. A field named by both
/// `bindings` and a sibling is refused outright — that shape nothing on the
/// real client's own side produces, and silently preferring one over the
/// other would hide the collision instead of surfacing it.
/// Field names that only ever legitimately arrive as an action sibling
/// (`props.action`'s own spread fields), never as a `captureBindings` value a
/// player's own control edited (PA.f180): `answerToken` is the one member
/// today — `eve::authenticated_surface` renders it DOM-invisibly on the
/// "Play" button's own action, never through any control a `bindings` entry
/// could come from. Nothing downstream of `unwrap_bindings` can tell the two
/// channels apart once merged, so invariant 8 (an answer's own token proves
/// nothing about the world, but must still prove it named a question the
/// player was actually shown) rested entirely on `eve.rs` never emitting it
/// as a control — a single mistake there, or a request built by hand rather
/// than through the real client, would otherwise be indistinguishable from
/// the real thing. This list is checked before the two objects merge, while
/// the channel each field arrived on is still known.
const ACTION_ONLY_PAYLOAD_FIELDS: &[&str] = &["answerToken"];

fn unwrap_bindings(payload: &Value) -> Result<Value, String> {
    let Some(object) = payload.as_object() else {
        return Ok(payload.clone());
    };
    let Some(bindings_object) = object.get("bindings").and_then(Value::as_object) else {
        return Ok(payload.clone());
    };
    for field in ACTION_ONLY_PAYLOAD_FIELDS {
        if bindings_object.contains_key(*field) {
            return Err(format!(
                "'{field}' arrived as a captured binding, not as an action field; it is refused \
                 rather than trusted as though it named the question the player was actually shown"
            ));
        }
    }
    let mut merged = serde_json::Map::with_capacity(object.len() + bindings_object.len());
    for (key, value) in object {
        if key == "bindings" {
            continue;
        }
        if bindings_object.contains_key(key) {
            return Err(format!(
                "payload's envelope 'bindings' object and its own sibling action field both \
                 name '{key}'; send it in exactly one place"
            ));
        }
        merged.insert(key.clone(), value.clone());
    }
    for (key, value) in bindings_object {
        merged.insert(key.clone(), value.clone());
    }
    Ok(Value::Object(merged))
}

/// Cut 10 (PA.f149): every one of these three fields is edited in a plain
/// `control.input.textarea` — the vendored Eve browser lowering has no
/// JSON-typed control — so a real client always captures the field as the
/// JS string the textarea holds (Dungeon's own `eve::local_draft` calls
/// these fields' `valueKind` `"string"` for exactly this reason), never a
/// parsed object or array. A hand-built payload (existing tests) may still
/// supply the already-parsed value directly. This accepts either: a JSON
/// string is parsed as JSON text; anything else is deserialized as the typed
/// value directly.
fn deserialize_json_capture<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: serde::Deserializer<'de>,
    T: serde::de::DeserializeOwned,
{
    match Value::deserialize(deserializer)? {
        Value::String(text) => serde_json::from_str(&text).map_err(serde::de::Error::custom),
        other => serde_json::from_value(other).map_err(serde::de::Error::custom),
    }
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

/// `world.play`'s own payload. Cut 10 (PA.f151) deleted the client-supplied
/// `answers` field: the vendored Eve browser lowering has no authored,
/// non-editable binding value (`hidden` is not a prop any renderer reads), so
/// the id-carrying control it used to name would have rendered as a plain,
/// visible, editable text box holding the raw `QuestionId` — a leak, and once
/// editable, a forgeable one. The server still resolves the currently open
/// question itself (`execute_world`'s `world.play` arm), but PA.f170 replaces
/// the version-only staleness check that once stood in for identity:
/// `answer_token` is the opaque per-question token `play::ASK_PLAYER_TOOL`
/// mints when the question opens, carried DOM-invisibly in the "Play"
/// button's own `props.action` (`eve::authenticated_surface`) rather than
/// through a `captureBindings` control — the real lowering spreads a button's
/// `action` fields straight into the payload alongside `bindings`
/// (`commandPayload`, `vendor/eve/packages/eve-browser-lowering/src/index.ts:2101`).
/// Absent or not naming the currently open question's own token, the answer
/// is refused as stale, never applied to whatever question happens to be
/// open when the request is processed.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct PlayPayload {
    text: String,
    #[serde(default)]
    answer_token: Option<String>,
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
    let play_view = current_play_view(&state).await;
    match current_world(&state).await.and_then(|snapshot| {
        eve::authenticated_surface(
            principal.account_subject_hash(),
            snapshot.as_ref(),
            play_view.as_ref(),
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
    let bindings = match unwrap_bindings(&invocation.payload) {
        Ok(bindings) => bindings,
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
    if let Err(error) = serde_json::from_value::<EmptyPayload>(bindings.clone()) {
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
    let bindings = match unwrap_bindings(&invocation.payload) {
        Ok(bindings) => bindings,
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
    let payload = match serde_json::from_value::<CompleteAuthPayload>(bindings.clone()) {
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
    let bindings = match unwrap_bindings(&invocation.payload) {
        Ok(bindings) => bindings,
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
    if let Err(error) = serde_json::from_value::<EmptyPayload>(bindings.clone()) {
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

/// The one door an answer's own `answerToken` is judged by (PA.f179): admits
/// only when both `given` (`payload.answer_token`) and `expected`
/// (`question.token`) are `Some` and equal. Extracted to its own function so
/// the absence case is a direct, cheap unit test rather than something only
/// reachable through the full `world.play` HTTP round trip — `expected`
/// being `None` (a question whose own token was, for whatever reason, never
/// minted) must refuse unconditionally, including against a `given` that is
/// also `None`, or against `Some("")`; it must never be treated as a
/// wildcard or degrade to a plain string comparison.
fn answer_token_admits(given: Option<&str>, expected: Option<&str>) -> bool {
    matches!((given, expected), (Some(given), Some(expected)) if given == expected)
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
        let payload: CreatePayload = serde_json::from_value(unwrap_bindings(&invocation.payload).map_err(RuntimeCommandError::Payload)?)
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
        // Cut 10 (PA.f152): owner-gated, like `world.advance_time` and
        // `world.seed` below — `eve::authenticated_surface` already hides
        // the card and the button from anyone else, but the route is the
        // actual authority boundary.
        let snapshot = current_world(state)
            .await
            .map_err(|error| RuntimeCommandError::Payload(error.to_string()))?
            .ok_or_else(|| RuntimeCommandError::Payload("world has not been created".into()))?;
        if snapshot.owner != PrincipalId::new(verified_principal.account_subject_hash()) {
            return Err(RuntimeCommandError::Payload(
                "only the world's owner may play".into(),
            ));
        }
        let payload: PlayPayload = serde_json::from_value(unwrap_bindings(&invocation.payload).map_err(RuntimeCommandError::Payload)?)
            .map_err(|error| RuntimeCommandError::Payload(error.to_string()))?;
        let table = state
            .play
            .clone()
            .ok_or_else(|| RuntimeCommandError::Payload("the play table is unavailable".into()))?;
        // The server resolves the open question itself — the client no
        // longer names it by `QuestionId`. No question open: this is a plain
        // continue/opening, `answers: None`. A question open: the caller
        // must be answering it, and PA.f170's guarantee (an answer cannot
        // land on a question the player never saw) is judged by the opaque
        // per-question token the question's own render carried in the "Play"
        // button's `props.action` (`eve::authenticated_surface`), not by
        // `routeHint.sourceVersion`: two questions opened in the same turn
        // carry the same surface version (asking is conversational and never
        // moves it), so a version-only comparison cannot tell them apart —
        // exactly the gap Soul proved (PA.f170). A token that is absent, or
        // does not name the currently open question's own token, is refused
        // as stale.
        let view = table.current_turn_view().await;
        // PA.f155: `PlayTurnView::state` is the actual rule this branches
        // on — the turn is currently waiting on the player — rather than
        // `question.is_some()` standing in for it; `question` is set
        // whenever the state is, so this and `question.is_some()` can never
        // disagree, but the state is the thing the rule is actually about.
        let is_awaiting_player = view
            .as_ref()
            .is_some_and(|view| view.state == PlayTurnState::AwaitingPlayer);
        // PA.f178: a stale tab — one that rendered a card for a question
        // that has since closed, answered, or been superseded by a fresh
        // turn — still carries the spent `answerToken` its own render was
        // built from. Before this check, `answer_token` was read only inside
        // the `Some(question)` arm below, so with no question currently
        // open this fell straight through to `answers: None`, and the
        // request's own `text` (the stale tab's stale answer prose) silently
        // opened a *new* turn instead: a real inference budget spent on an
        // answer to a question that no longer exists. A token naming no
        // currently open question is refused outright, the same family of
        // refusal as a mismatched token, and never reaches `admit` at all.
        if payload.answer_token.is_some() && !is_awaiting_player {
            return Err(RuntimeCommandError::Payload(
                "that question is closed".into(),
            ));
        }
        let answers = match view.as_ref().and_then(|view| view.question.as_ref()).filter(|_| is_awaiting_player) {
            None => None,
            Some(question) => {
                if !answer_token_admits(payload.answer_token.as_deref(), question.token.as_deref()) {
                    return Err(RuntimeCommandError::Payload(format!(
                        "the answer is stale: its token does not name the question \"{}\" \
                         currently open",
                        question.text
                    )));
                }
                Some(question.id.clone())
            }
        };
        // PA.f147 (reported, not fixed — see Hands' own report): the
        // unconditional `command_id = CommandId::parse_uuid(...)?` at the
        // top of this function already refuses a missing or non-uuid key
        // for every operation, `world.play` included, before this branch is
        // ever reached — verified by reverting a local guard here to this
        // exact `unwrap_or_default()` and re-running a test built to catch
        // it: it still passed. A local, explicit guard was tried and kept
        // only if it could be shown to matter; it could not, so it was not
        // kept as unreachable code.
        let key = invocation.operation.idempotency_key.clone().unwrap_or_default();
        // Cut 10 (PA.f153/PA.f154): `admit` runs synchronously, here, so a
        // replayed key, a stale/empty answer, a busy table, and a poisoned
        // table all answer `denied` with the reason, instead of being
        // decided inside a spawned task whose result nothing ever read. Only
        // the turn's own round loop — the part that can spend a real
        // inference budget — is spawned.
        let admission = table
            .admit(key, PlayRequest { text: payload.text, answers })
            .await
            .map_err(|error| RuntimeCommandError::Payload(error.to_string()))?;
        let admitted = match admission {
            Admission::Replayed { .. } => {
                return Ok(json!({"kind":"accepted","outcome":"replayed"}));
            }
            Admission::Spawn(admitted) => admitted,
        };
        let principal = verified_principal.clone();
        let spawned_state = state.clone();
        tokio::spawn(async move {
            if let Err(error) = table.execute(admitted, &principal).await {
                // PA.f155: `PlayTurnView::turn_id` names which turn faulted,
                // for an operator reading this log against the store — the
                // one production reader it had none of before.
                let turn_id = table.current_turn_view().await.map(|view| view.turn_id);
                tracing::warn!(%error, ?turn_id, "a play turn ended in error");
            }
            // Cut 10 (PA.f150): a play-row-only change (a fresh refusal, a
            // newly open question, a closed turn's narration) commits
            // nothing to the world, so nothing on the `eve_command` Ok arm's
            // own `publish_projection` call (`dispatch_world`) would ever
            // fire for it — that call only runs for this same command's own
            // immediate `accepted` response, sent before this turn's own
            // execution even started. This is the turn's own commit,
            // published on the same door.
            if let Err(error) = publish_projection(&spawned_state).await {
                tracing::warn!(%error, "play turn projection publish failed");
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
            serde_json::from_value::<EmptyPayload>(unwrap_bindings(&invocation.payload).map_err(RuntimeCommandError::Payload)?)
                .map_err(|error| RuntimeCommandError::Payload(error.to_string()))?;
            CommandBody::ApproveDraft
        }
        "world.activate" => {
            serde_json::from_value::<EmptyPayload>(unwrap_bindings(&invocation.payload).map_err(RuntimeCommandError::Payload)?)
                .map_err(|error| RuntimeCommandError::Payload(error.to_string()))?;
            CommandBody::ActivateWorld
        }
        "world.advance_time" => {
            let payload: AdvanceTimePayload = serde_json::from_value(unwrap_bindings(&invocation.payload).map_err(RuntimeCommandError::Payload)?)
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
            let payload: SeedPayload = serde_json::from_value(unwrap_bindings(&invocation.payload).map_err(RuntimeCommandError::Payload)?)
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

/// The current play turn's own player-facing state (Cut 9), read straight off
/// `PlayTable::current_turn_view` — `None` when the table is unavailable, and
/// `None` again when it is available but no turn has ever opened, exactly as
/// `current_turn_view` itself returns. `eve::authenticated_surface`'s own play
/// card degrades to its own empty shape either way.
async fn current_play_view(state: &AppState) -> Option<PlayTurnView> {
    state.play.as_ref()?.current_turn_view().await
}

async fn publish_projection(state: &AppState) -> anyhow::Result<u64> {
    let snapshot = current_world(state).await?;
    // This function's own return value — `command_result.sourceVersion` in
    // `dispatch_world`'s Ok arm — stays plain `surface_version` (PA.f148),
    // the only meaning a caller ever feeds back as `routeHint.sourceVersion`.
    // The SSE broadcast below is a different field with a different job: it
    // wakes a watching client, is never echoed back, and so carries
    // `authenticated_wake_version` instead, the value that also moves on a
    // play-row-only change (PA.f150) — every play commit, including a fresh
    // question, narration, or refusal, reaches this call.
    let version = eve::surface_version(snapshot.as_ref());
    let play_view = current_play_view(state).await;
    let wake_version = eve::authenticated_wake_version(snapshot.as_ref(), play_view.as_ref());
    let _ = state.revisions.send(wake_version);
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
                    // `controller_permits` has one production consumer since
                    // Cut 1 deleted Dungeon's own drivers: the play table's
                    // own dispatched-Persona turns and turn-close narration
                    // (Cut 8b). Every permit taken is therefore genuine
                    // Persona cognition in flight, so "active" here is
                    // accurate, not merely reachable through a test's own
                    // forced exhaustion.
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
    // PA.f172: the play table can fail to open independently of
    // `controllers` (a `PlayTurnStore` a prior session's row makes unreadable),
    // and before this the only trace was `tracing::warn!("the play table is
    // unavailable; world authority remains online")` at startup — visible in
    // a log an operator has to go looking for, never on the readiness
    // surface a client or operator actually watches. Same `Option` ->
    // "ok"/"unavailable" shape as `projectionStatus`/`controllerStatus`
    // above; this does not invent a new channel.
    //
    // PA.f186: `open` can also succeed while quietly retiring a row it could
    // not read (`play::retire_unreadable_row`, PA.f184/PA.f177) — before this,
    // `state.play.is_some()` alone decided this field, so a successful
    // retirement read as plain `"ok"` even though the player's own turn was
    // just destroyed and the only trace was one `tracing::warn!`. Reusing the
    // same `playStatus` channel (not a new one) rather than a bare enum token
    // when a retirement happened this process is what keeps this readable
    // without inventing a sibling field.
    let play_status = match &state.play {
        Some(table) => match table.retired_row_sidecar() {
            Some(sidecar) => format!("a play-turn row was retired to {}", sidecar.display()),
            None => "ok".into(),
        },
        None => "unavailable".into(),
    };
    health["projectionStatus"] = Value::String(projection_status.into());
    health["controllerStatus"] = Value::String(controller_status.into());
    health["playStatus"] = Value::String(play_status);
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
    // The refusal a player sees must stay a projection of the world and their
    // own act (invariant 8), never a server configuration name: naming
    // `SEED_VAULT_ROOT_ENVIRONMENT` to the player taught them a fact about
    // this process's own environment, not about the world. The detail is
    // still logged, for the operator who can actually act on it.
    let root = std::env::var(SEED_VAULT_ROOT_ENVIRONMENT).map_err(|_| {
        tracing::warn!(
            environment_variable = SEED_VAULT_ROOT_ENVIRONMENT,
            "seeding was requested but the vault root is not configured"
        );
        RuntimeCommandError::Payload("seeding is not available on this world".into())
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
        .map_err(|error| {
            // PA.f187: `error.to_string()` used to reach the player verbatim
            // through `dispatch_world`'s own `error.to_string()` rendering —
            // Soul measured the real string on a fresh workstation with no
            // inference endpoint running: "invalid command payload: <Purpose>
            // inference failed: the local inference endpoint refused the
            // connection: error sending request for url
            // (http://127.0.0.1:11434/v1/chat/completions)". Connection-
            // refused is the likeliest seed failure of all, so this handed
            // this process's own host and port to the player on the first
            // real failure. A player-facing refusal on this route names a
            // category instead; the full detail is logged here, once, at the
            // point this process actually knows it. `ControllerError`'s own
            // `Display` is unchanged — every other caller still sees the
            // detailed message — so this is scoped to the seed route alone.
            tracing::warn!(%error, "seeding a world could not complete");
            RuntimeCommandError::Payload(describe_seed_failure(&error))
        })
}

/// PA.f187: categorizes a seed sweep's own `ControllerError` for the player,
/// never its `Display`. `Inference`/`ProviderContract` are the connectivity
/// class Soul's probe hit (a closed or unreachable inference endpoint); every
/// other variant still names no internal detail, just a coarser category —
/// none of `ControllerError`'s other arms are seed-route-reachable today, but
/// this does not assume that stays true.
fn describe_seed_failure(error: &ControllerError) -> String {
    match error {
        ControllerError::Inference { .. } | ControllerError::ProviderContract { .. } => {
            "seeding could not reach the model".into()
        }
        _ => "seeding could not complete".into(),
    }
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
        // PA.f140: `open_play` writes the play table's own row here
        // (`service_root.join("play-turn-v1.cc")`), the same path this
        // pre-flight check must prove is a direct, non-symlinked, single-
        // linked regular file before anything is admitted to write it —
        // this list was never updated when Cut 8b added that store.
        service_root.join("play-turn-v1.cc"),
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
    use ghostlight::{InferenceFault, InferencePurpose};
    use tower::ServiceExt;

    #[cfg(target_os = "linux")]
    #[test]
    fn managed_linux_runtime_requires_the_admitted_state_root_binding() {
        assert!(admitted_runtime_root(None).is_err());
    }

    /// PA.f179's own direct proof, independent of the HTTP round trip: an
    /// absent expected token (the mint defeated, or any other path that
    /// leaves `question.token` `None`) must refuse every `given`, including
    /// `None` and `Some("")` — the exact values a naive fallback
    /// (`unwrap_or_default()` on either side, or a plain string compare)
    /// would treat as matching.
    ///
    /// Mutation: replace `answer_token_admits`'s body with
    /// `given.unwrap_or_default() == expected.unwrap_or_default()` — the
    /// `given: None, expected: None` and `given: Some(""), expected: None`
    /// cases below would then wrongly admit.
    #[test]
    fn answer_token_admits_only_two_present_and_equal_tokens() {
        assert!(answer_token_admits(Some("tok-a"), Some("tok-a")));
        assert!(!answer_token_admits(Some("tok-a"), Some("tok-b")));
        assert!(!answer_token_admits(None, Some("tok-a")));
        assert!(
            !answer_token_admits(None, None),
            "a question whose own token was never minted must refuse even an answer with no token at all"
        );
        assert!(
            !answer_token_admits(Some(""), None),
            "an empty-string answer must not be treated as matching an unminted (None) token"
        );
        assert!(
            !answer_token_admits(Some("tok-a"), None),
            "any given token must be refused when none was ever minted for the question"
        );
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

    /// PA.f140: `open_play` writes the play table's own row to
    /// `service/play-turn-v1.cc` (see its own call site), so the pre-flight
    /// layout check must cover that path exactly like `world.cc`,
    /// `app-sessions-v2.cc`, `controller-work.cc`, and `mesh-v2.cc` already
    /// do — a symlink standing in for it must be refused the same way.
    /// Mutation: dropping `play-turn-v1.cc` from the checked list.
    #[cfg(target_os = "linux")]
    #[test]
    fn post_lease_state_layout_rejects_a_symlinked_play_turn_store() {
        use std::os::unix::fs::symlink;

        let directory = tempfile::tempdir().unwrap();
        let root = fs::canonicalize(directory.path()).unwrap();
        fs::create_dir(root.join("service")).unwrap();
        let target = root.join("service").join("other.cc");
        fs::write(&target, b"not-play-turn").unwrap();
        symlink(&target, root.join("service").join("play-turn-v1.cc")).unwrap();
        assert!(prepare_admitted_state_layout(&root).is_err());
    }

    struct Fixture {
        _directory: tempfile::TempDir,
        state: AppState,
        cookie: String,
        /// Held only by tests that swap `state.play` for a table over a
        /// second store path (PA.f161's `play_world_with_a_scripted_question`):
        /// keeps that store's own temp directory alive for exactly as long as
        /// the swapped-in table is, never read.
        _play_directory: Option<tempfile::TempDir>,
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
            _play_directory: None,
        }
    }

    /// PA.f172: `state.play` being `None` — a play table that failed to
    /// open, for any reason, `PlayTurnStore::open` returning `Err` among
    /// them — must be visible on `/health`, the same operator-facing surface
    /// `projectionStatus`/`controllerStatus` already report through, not
    /// only in a `tracing::warn!` at startup.
    ///
    /// Mutation: delete the `health["playStatus"] = ...` line from
    /// `runtime_readiness` — `/health`'s own JSON would then carry no
    /// `playStatus` key at all, and both assertions below would fail.
    #[tokio::test]
    async fn health_reports_play_status_from_state_play() {
        let fixture = fixture().await;
        let available = get(&fixture.state, &fixture.cookie, "/health").await;
        assert_eq!(available["playStatus"], "ok", "{available}");

        let mut degraded_state = fixture.state.clone();
        degraded_state.play = None;
        let unavailable = get(&degraded_state, &fixture.cookie, "/health").await;
        assert_eq!(unavailable["playStatus"], "unavailable", "{unavailable}");
    }

    /// PA.f186: `state.play.is_some()` alone used to decide `playStatus`, so
    /// a `PlayTable::new` that succeeded by *retiring* an unreadable row
    /// (PA.f184/PA.f177's own `retire_unreadable_row`) read as plain `"ok"`
    /// on `/health` — the same surface `health_reports_play_status_from_state_play`
    /// above already covers for the *unavailable* case — even though the
    /// player's own turn had just been destroyed, with only a
    /// `tracing::warn!` as evidence. Builds a play table over a store this
    /// process's own `open` cannot read (`play::tests::seed_unreadable_row_for_test`,
    /// the exact shape `play::tests::a_row_with_the_current_schema_id_and_undecodable_bytes_is_retired_to_a_sidecar_and_play_starts_fresh`
    /// proves retires cleanly) and checks the field end to end, through the
    /// real `/health` route.
    ///
    /// Mutation: revert `play_status` to `match &state.play { Some(_) =>
    /// "ok", None => "unavailable" }` — this test's own `assert_ne!` and
    /// `contains("retired")` both then fail, since a successful retirement
    /// would read as plain `"ok"` again.
    #[tokio::test]
    async fn health_reports_a_retirement_not_plain_ok() {
        use crate::play::tests::{ScriptedPort, seed_unreadable_row_for_test};

        let mut fixture = fixture().await;
        let directory = tempfile::tempdir().unwrap();
        let store_path = directory.path().join("play-turn-v1.cc");
        seed_unreadable_row_for_test(&store_path);

        let personas = PersonaLane::new(
            ControllerPort::new(fixture.state.world.clone()),
            ScriptedPort::new(vec![]),
            "gpt-5.6-sol".into(),
            "gpt-5.6-sol".into(),
        )
        .unwrap();
        let table = PlayTable::new(
            fixture.state.world.clone(),
            personas,
            ScriptedPort::new(vec![]),
            "gpt-5.6-terra".into(),
            Arc::new(Semaphore::new(TEST_CONTROLLER_CONCURRENCY)),
            &store_path,
        )
        .expect("opening over an unreadable row must retire it, not fail outright (PA.f184)");
        fixture.state.play = Some(Arc::new(table));
        fixture._play_directory = Some(directory);

        let health = get(&fixture.state, &fixture.cookie, "/health").await;
        let play_status = health["playStatus"].as_str().unwrap();
        assert_ne!(play_status, "ok", "a retirement must not read as plain ok: {health}");
        assert!(
            play_status.contains("retired"),
            "the retirement must be nameable from the readiness surface itself: {health}"
        );
        assert!(
            play_status.contains("play-turn-v1.retired-unreadable-row"),
            "the reported status must name the sidecar path, not just that something happened: {health}"
        );
    }

    /// PA.f193-E: `playStatus` used to report a retirement only for the
    /// process that performed it — `PlayTable::retired_row_sidecar` cached a
    /// field set once at `open`, so any later restart's own `open` found a
    /// clean store (nothing left to retire) and reported plain `ok` again,
    /// even though the sidecar file a prior process wrote was still sitting
    /// on disk right beside the store with the only trace of what happened.
    /// This reopens a *second* `PlayTable` over the exact same `store_path`
    /// after the first retirement already ran (the shape a real daemon
    /// restart takes) and proves `/health` still names the retirement.
    ///
    /// Mutation: revert `PlayTable::retired_row_sidecar` to return a field
    /// cached at construction (`self.retired_sidecar.as_deref()`) instead of
    /// scanning `retired_row_sidecars_on_disk` fresh — the second table's own
    /// `open` retires nothing (the row is already fresh from the first
    /// table's retirement), so the cached field would read `None` and the
    /// assertion below fails.
    #[tokio::test]
    async fn health_reports_a_retirement_across_a_restart_not_only_the_process_that_performed_it() {
        use crate::play::tests::{ScriptedPort, seed_unreadable_row_for_test};

        let mut fixture = fixture().await;
        let directory = tempfile::tempdir().unwrap();
        let store_path = directory.path().join("play-turn-v1.cc");
        seed_unreadable_row_for_test(&store_path);

        // First "process": its own `open` performs the retirement.
        {
            let personas = PersonaLane::new(
                ControllerPort::new(fixture.state.world.clone()),
                ScriptedPort::new(vec![]),
                "gpt-5.6-sol".into(),
                "gpt-5.6-sol".into(),
            )
            .unwrap();
            let first_table = PlayTable::new(
                fixture.state.world.clone(),
                personas,
                ScriptedPort::new(vec![]),
                "gpt-5.6-terra".into(),
                Arc::new(Semaphore::new(TEST_CONTROLLER_CONCURRENCY)),
                &store_path,
            )
            .expect("opening over an unreadable row must retire it, not fail outright (PA.f184)");
            assert!(
                first_table.retired_row_sidecar().is_some(),
                "the table that actually performed the retirement must report it"
            );
            // Dropped here: the redb `Database`'s own single-owner lock must
            // release before a second `PlayTable` can open the same path.
        }

        // A fresh "process": opens the same, now-clean store. Nothing here
        // retires anything — the row `first_table` left behind already reads
        // fine — so this is exactly a restart, not a second corruption.
        let personas = PersonaLane::new(
            ControllerPort::new(fixture.state.world.clone()),
            ScriptedPort::new(vec![]),
            "gpt-5.6-sol".into(),
            "gpt-5.6-sol".into(),
        )
        .unwrap();
        let second_table = PlayTable::new(
            fixture.state.world.clone(),
            personas,
            ScriptedPort::new(vec![]),
            "gpt-5.6-terra".into(),
            Arc::new(Semaphore::new(TEST_CONTROLLER_CONCURRENCY)),
            &store_path,
        )
        .expect("reopening the already-retired store must succeed cleanly");
        fixture.state.play = Some(Arc::new(second_table));
        fixture._play_directory = Some(directory);

        let health = get(&fixture.state, &fixture.cookie, "/health").await;
        let play_status = health["playStatus"].as_str().unwrap();
        assert_ne!(
            play_status, "ok",
            "a restart must still surface a retirement a prior process performed, not read as plain ok: {health}"
        );
        assert!(
            play_status.contains("retired-unreadable-row"),
            "the restarted process must still name the sidecar evidence on disk: {health}"
        );
    }

    /// PA.f187: a seed-route refusal must never carry `ControllerError`'s own
    /// `Display` to the player — Soul measured the real string on a fresh
    /// workstation with no inference endpoint running: "invalid command
    /// payload: OperationalAgent inference failed: the local inference
    /// endpoint refused the connection: error sending request for url
    /// (http://127.0.0.1:11434/v1/chat/completions)", handing the player this
    /// process's own inference host and port. `describe_seed_failure` must
    /// name a category instead, for the connectivity class this actually
    /// happens on, without reproducing the endpoint anywhere in its output.
    ///
    /// Mutation: change `describe_seed_failure`'s `Inference { .. } |
    /// ProviderContract { .. }` arm to `_ => error.to_string()` (or any arm
    /// that forwards `error`'s own `Display`) — the assertions below, which
    /// specifically check the category string never contains the raw fault
    /// detail, then fail.
    #[test]
    fn describe_seed_failure_never_reproduces_the_inference_faults_own_detail() {
        let raw_detail = "the local inference endpoint refused the connection: error sending request \
                           for url (http://127.0.0.1:11434/v1/chat/completions)";
        let error = ControllerError::Inference {
            purpose: InferencePurpose::OperationalAgent,
            source: InferenceFault::retryable(raw_detail),
        };
        // The raw error's own Display really does carry the host and port —
        // pinning that here so this test fails loudly if `InferenceFault`'s
        // own `Display` ever stops reproducing its `detail` verbatim, rather
        // than silently testing nothing.
        assert!(error.to_string().contains("127.0.0.1:11434"), "{error}");

        let described = describe_seed_failure(&error);
        assert!(!described.contains("127.0.0.1"), "the category must not leak the endpoint: {described}");
        assert!(!described.contains("11434"), "the category must not leak the port: {described}");
        assert!(!described.contains("refused the connection"), "{described}");
        assert_eq!(described, "seeding could not reach the model");

        // A non-connectivity `ControllerError` still gets a category, not a
        // raw `Display`, even though Soul's own scenario was specifically
        // about the inference class.
        let other = describe_seed_failure(&ControllerError::AmbiguousOpportunity);
        assert_eq!(other, "seeding could not complete");
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

    async fn get(state: &AppState, cookie: &str, path: &str) -> Value {
        let response = api_router(state.clone())
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(path)
                    .header(header::COOKIE, format!("{COOKIE_NAME}={cookie}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let bytes = axum::body::to_bytes(response.into_body(), 1024 * 1024)
            .await
            .unwrap();
        serde_json::from_slice(&bytes).unwrap()
    }

    /// Cut 10 (PA.f149): drives the real vendored Eve browser lowering
    /// (`vendor/eve/packages/eve-browser-lowering`, submodule pin unmoved)
    /// over a surface document this process actually served, through
    /// `tools/eve_client_bridge.mjs`. `steps` is the bridge's own
    /// `{"set":{node_id:text}}`/`{"click":node_id}` step list; returns the
    /// command intent(s) the click(s) produced, in order, exactly as a real
    /// client would build and send them — never a hand-built payload
    /// asserting its own shape is correct.
    fn client_intents(surface: &Value, provider: &Value, steps: Value) -> Vec<Value> {
        let bridge = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("tools")
            .join("eve_client_bridge.mjs");
        let input = serde_json::to_vec(&json!({
            "surface": surface,
            "provider": provider,
            "steps": steps,
        }))
        .unwrap();
        let mut child = std::process::Command::new("node")
            .arg(&bridge)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .expect("node must be on PATH to run the client bridge");
        {
            use std::io::Write;
            child.stdin.take().unwrap().write_all(&input).unwrap();
        }
        let output = child.wait_with_output().unwrap();
        assert!(
            output.status.success(),
            "eve_client_bridge.mjs failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8(output.stdout)
            .unwrap()
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(|line| serde_json::from_str(line).unwrap())
            .collect()
    }

    /// PA.f149's own end-to-end proof: `world.create`, `world.approve`,
    /// `world.seed`, and `world.activate` intents, built by the real
    /// vendored lowering over surfaces this process actually served, and
    /// posted through `api_router` — every one of them accepted. Before
    /// this cut, every one of them submitted `{"bindings":{}}` (wrong
    /// binding names) and, even with the names fixed, `{"payload":
    /// {"bindings":{...}}}` (the envelope) against Dungeon's flat payload
    /// structs — either alone was enough to refuse every field as missing.
    ///
    /// `world.play` is included through "the route accepts it and the turn
    /// reaches the play table" (the same proof
    /// `world_play_reaches_the_play_table_and_is_accepted` already gives
    /// hand-built payloads, given here to the real client's own payload
    /// instead). This fixture's inference organ is an unreachable test
    /// address (`127.0.0.1:9`, see `fixture()`), so the turn it opens can
    /// never actually reach `AwaitingPlayer` here to prove the "answer" leg
    /// through this same harness — that would need a scripted inference
    /// port, which lives only inside `play.rs`'s own private test module
    /// (`ScriptedPort`) and is not reachable from here. PA.f151's own
    /// server-resolved-answer mechanism has its own direct proof instead, at
    /// the play-table layer:
    /// `play::tests::a_question_is_answered_through_the_cards_own_binding_and_the_turn_closes_with_narration`.
    /// (Soul's owed item 3): `#[ignore]`d, not skipped-with-a-print. Before
    /// this, an unavailable `node`/vendor-lowering environment made this
    /// test return early and count as an ordinary pass — cargo's own summary
    /// line reported the identical "178 passed" whether or not this actually
    /// drove the real Eve client bridge, and only `--nocapture` surfaced the
    /// difference (the `eprintln!` above was otherwise captured and
    /// discarded on a passing test). `#[ignore]` makes cargo's own default
    /// summary distinguish the two: an ordinary `cargo test` run reports this
    /// (and its three siblings gated the same way) under "X ignored", visible
    /// with no extra flags, and never executes the body at all. Run
    /// `cargo test -p ghostlight-dungeon --bin ghostlight-dungeon -- --ignored`
    /// where `node` is on PATH and both `npm install --prefix
    /// vendor/eve/packages/eve-browser-lowering` and `npm install --prefix
    /// vendor/eve/packages/eve-contracts` have been run, then `npm run build
    /// --prefix vendor/eve/packages/eve-browser-lowering`, to actually
    /// exercise it. The cost: this no longer auto-runs on a workstation that
    /// happens to have those dependencies installed — opting in is now
    /// explicit, and asking for it without the dependencies present now
    /// panics instead of silently returning, since a deliberate `--ignored`
    /// run that still cannot drive the real bridge is a real failure to
    /// report, not a skip to swallow quietly.
    #[tokio::test]
    #[ignore = "requires node + a built vendor/eve/eve-browser-lowering + eve-contracts; run with `cargo test -- --ignored`"]
    async fn world_create_seed_activate_and_play_round_trip_through_the_real_client() {
        if !eve_client_bridge_is_available() {
            panic!(
                "world_create_seed_activate_and_play_round_trip_through_the_real_client was run \
                 (via --ignored) but the real Eve client bridge is not available: `node` is \
                 missing, or the vendored lowering's built `dist/index.js`, its `jsdom` \
                 devDependency, or the sibling `eve-contracts` package's own `ajv` dependency does \
                 not resolve in this environment. Run `node` on PATH and both `npm install --prefix \
                 vendor/eve/packages/eve-browser-lowering` and `npm install --prefix \
                 vendor/eve/packages/eve-contracts`, then `npm run build --prefix \
                 vendor/eve/packages/eve-browser-lowering`, before running this test."
            );
        }
        let fixture = fixture().await;
        let provider = get(&fixture.state, &fixture.cookie, "/api/eve/provider").await;

        let empty_surface = get(&fixture.state, &fixture.cookie, "/api/eve/surfaces/ghostlight.play").await;
        let create_intents = client_intents(
            &empty_surface,
            &provider,
            json!([
                {"set": {
                    "world.create.title": "Bridge World",
                    "world.create.brief": "A world built by the real client bridge.",
                    "world.create.subject": "The Bridge Operator",
                    "world.create.targets": "{}",
                    "world.create.jurisdictions": "[]",
                    "world.create.lens_weights": "{\"patina\":1,\"charter\":1,\"ledger\":1,\"hearth\":1,\"tangle\":1,\"veil\":1,\"ember\":1,\"numen\":1}"
                }},
                {"click": "world.create"}
            ]),
        );
        assert_eq!(create_intents.len(), 1);
        let created = post(&fixture.state, &fixture.cookie, create_intents.into_iter().next().unwrap()).await;
        assert_eq!(created["receipt"]["state"], "accepted", "world.create via the real client: {created}");

        let draft_surface = get(&fixture.state, &fixture.cookie, "/api/eve/surfaces/ghostlight.play").await;
        let approve_intents = client_intents(&draft_surface, &provider, json!([{"click": "world.approve"}]));
        assert_eq!(approve_intents.len(), 1);
        let approved = post(&fixture.state, &fixture.cookie, approve_intents.into_iter().next().unwrap()).await;
        assert_eq!(approved["receipt"]["state"], "accepted", "world.approve via the real client: {approved}");

        let approved_surface = get(&fixture.state, &fixture.cookie, "/api/eve/surfaces/ghostlight.play").await;
        let seed_intents = client_intents(&approved_surface, &provider, json!([{"click": "world.seed"}]));
        assert_eq!(seed_intents.len(), 1);
        let seeded = post(&fixture.state, &fixture.cookie, seed_intents.into_iter().next().unwrap()).await;
        // No test fixture in this crate configures `GHOSTLIGHT_SEED_VAULT_ROOT`
        // (`world_seed_is_owner_only_and_draft_only_before_it_spends_anything`
        // asserts the same denial), so a real vault read is out of reach
        // here regardless of payload shape. The proof this leg carries is
        // narrower: the payload itself was accepted and reached the vault
        // lookup at all — `SeedPayload` has no required field, so
        // `captureBindings: []` (the click alone) is this operation's own
        // envelope-only proof — never refused as a payload/binding shape
        // problem the way every operation was before this cut.
        assert_eq!(
            seeded["receipt"]["message"], "invalid command payload: seeding is not available on this world",
            "world.seed via the real client must reach the vault lookup, not refuse the payload shape: {seeded}"
        );

        let seeded_surface = get(&fixture.state, &fixture.cookie, "/api/eve/surfaces/ghostlight.play").await;
        let activate_intents = client_intents(&seeded_surface, &provider, json!([{"click": "world.activate"}]));
        assert_eq!(activate_intents.len(), 1);
        let activated = post(&fixture.state, &fixture.cookie, activate_intents.into_iter().next().unwrap()).await;
        assert_eq!(activated["receipt"]["state"], "accepted", "world.activate via the real client: {activated}");

        let active_surface = get(&fixture.state, &fixture.cookie, "/api/eve/surfaces/ghostlight.play").await;
        let play_intents = client_intents(
            &active_surface,
            &provider,
            json!([
                {"set": {"world.play.text": "The new owner looks around."}},
                {"click": "world.play"}
            ]),
        );
        assert_eq!(play_intents.len(), 1);
        let played = post(&fixture.state, &fixture.cookie, play_intents.into_iter().next().unwrap()).await;
        assert_eq!(played["receipt"]["state"], "accepted", "world.play via the real client: {played}");

        let table = fixture.state.play.clone().unwrap();
        let observed = tokio::time::timeout(Duration::from_secs(5), async move {
            loop {
                if table.current_turn_view().await.is_some() {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await;
        assert!(
            observed.is_ok(),
            "the real client's own world.play intent must reach PlayTable::run and open a turn row"
        );
    }

    /// PA.f162: the real vendored lowering's own intents for the full
    /// create/approve/seed/activate/play round trip, captured once against
    /// the pinned `vendor/eve` submodule revision this document names, and
    /// committed so the gate never needs `node`, `jsdom`, or the built
    /// lowering to replay them. `#[serde(rename_all = "camelCase")]` matches
    /// the JSON exactly; no other transformation happens between capture and
    /// commit.
    const EVE_CLIENT_BRIDGE_FIXTURE: &str = include_str!("fixtures/eve_client_bridge_intents.json");

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct BridgeFixture {
        #[allow(dead_code)]
        schema: String,
        vendor_eve_submodule_revision: String,
        steps: Vec<BridgeFixtureStep>,
    }

    #[derive(Deserialize)]
    struct BridgeFixtureStep {
        label: String,
        intent: Value,
    }

    fn bridge_fixture() -> BridgeFixture {
        serde_json::from_str(EVE_CLIENT_BRIDGE_FIXTURE)
            .expect("the committed eve_client_bridge_intents.json fixture must parse")
    }

    /// PA.f162's own gate proof: replays the committed fixture's intents,
    /// in order, straight through `api_router` — no `node`, no bridge, no
    /// vendored `dist/index.js` anywhere in this call path. This is what
    /// `world_create_seed_activate_and_play_round_trip_through_the_real_client`
    /// (which drives the real bridge, and so needs `node`) proved once, at
    /// capture time; this test proves it again on every run, on the door
    /// Idunn's own `cargo test --locked -p ghostlight-dungeon --bin
    /// ghostlight-dungeon` gate actually opens.
    #[tokio::test]
    async fn the_committed_eve_client_bridge_fixture_round_trips_through_api_router_with_no_node() {
        let doc = bridge_fixture();
        let fixture = fixture().await;
        let mut steps = doc.steps.into_iter();

        let create = steps.next().unwrap();
        assert_eq!(create.label, "world.create");
        let created = post(&fixture.state, &fixture.cookie, create.intent).await;
        assert_eq!(created["receipt"]["state"], "accepted", "{created}");

        let approve = steps.next().unwrap();
        assert_eq!(approve.label, "world.approve");
        let approved = post(&fixture.state, &fixture.cookie, approve.intent).await;
        assert_eq!(approved["receipt"]["state"], "accepted", "{approved}");

        let seed = steps.next().unwrap();
        assert_eq!(seed.label, "world.seed");
        let seeded = post(&fixture.state, &fixture.cookie, seed.intent).await;
        assert_eq!(
            seeded["receipt"]["message"], "invalid command payload: seeding is not available on this world",
            "{seeded}"
        );

        let activate = steps.next().unwrap();
        assert_eq!(activate.label, "world.activate");
        let activated = post(&fixture.state, &fixture.cookie, activate.intent).await;
        assert_eq!(activated["receipt"]["state"], "accepted", "{activated}");

        let play = steps.next().unwrap();
        assert_eq!(play.label, "world.play");
        let played = post(&fixture.state, &fixture.cookie, play.intent).await;
        assert_eq!(played["receipt"]["state"], "accepted", "{played}");
        assert!(steps.next().is_none(), "the fixture must carry exactly these five steps");

        let table = fixture.state.play.clone().unwrap();
        let observed = tokio::time::timeout(Duration::from_secs(5), async move {
            loop {
                if table.current_turn_view().await.is_some() {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await;
        assert!(
            observed.is_ok(),
            "the fixture's own world.play intent must reach PlayTable::run and open a turn row"
        );
    }

    /// Whether `node` is on `PATH` *and* the real bridge's own module graph
    /// actually resolves in this environment — the things `client_intents`
    /// needs. Idunn's own Linux release container has neither (PA.f162), so
    /// this is `false` there.
    ///
    /// PA.f176: this used to check only `node --version` and that
    /// `vendor/eve/packages/eve-browser-lowering/dist/index.js` is a file —
    /// but `dist/` is committed, so the second half of that check is always
    /// true, whether or not `npm install` has ever run anywhere. Measured on
    /// a clean checkout with `node` on `PATH` and no `npm install` run: this
    /// old check reported "available", and the three gated tests then failed
    /// outright on `jsdom` (`tools/eve_client_bridge.mjs`'s own dependency,
    /// resolved through `eve-browser-lowering`'s own `package.json`) — not
    /// skipped, failed, exactly the "2 of 151 failed" shape PA.f169 already
    /// fixed once for a different pair of tests. Running the runbook's own
    /// `npm install --prefix vendor/eve/packages/eve-browser-lowering` step
    /// still left `ajv` unresolved: `eve-browser-lowering`'s own `dist/index.js`
    /// imports `@gamecult/eve-contracts` (a sibling package under
    /// `vendor/eve/packages`, wired in as a `file:` dependency), and that
    /// sibling's own `node_modules` — where its own `ajv` dependency
    /// lives — is never populated by an `npm install` run only inside
    /// `eve-browser-lowering`.
    ///
    /// So this now proves the dependencies the bridge actually needs by
    /// doing exactly what `tools/eve_client_bridge.mjs` does at import time —
    /// resolving `jsdom` through the vendored lowering's own `package.json`
    /// and dynamically importing the built `dist/index.js` itself, which is
    /// what actually pulls in `eve-contracts` and, through it, `ajv` — in a
    /// real `node` process (`eve_client_bridge_dependencies_resolve`), rather
    /// than asserting a fact about one committed file that was never in
    /// question. Cached: every call after the first reuses the one real
    /// probe's result rather than spawning `node` again per test.
    fn eve_client_bridge_is_available() -> bool {
        static AVAILABLE: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
        *AVAILABLE.get_or_init(|| {
            let node_on_path = std::process::Command::new("node")
                .arg("--version")
                .output()
                .is_ok_and(|output| output.status.success());
            if !node_on_path {
                return false;
            }
            let lowering_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("..")
                .join("..")
                .join("vendor")
                .join("eve")
                .join("packages")
                .join("eve-browser-lowering");
            if !lowering_root.join("dist").join("index.js").is_file() {
                return false;
            }
            if !eve_client_bridge_dependencies_resolve(&lowering_root) {
                return false;
            }
            // PA.f190: every gated test in this shape needs `node`, the
            // built lowering, *and* a real `vendor/eve` git checkout —
            // `eve_client_bridge_fixture_matches_what_the_pinned_lowering_produces_today`
            // asserts `git -C vendor/eve rev-parse HEAD` succeeds with
            // `.expect(...)`, not a skip. Before this, Soul hit that
            // `.expect` by accident with a `vendor/eve` that was a copied
            // directory rather than a checkout — this function's own two
            // checks above both passed (a copy still carries the committed
            // `dist/`), and the test *panicked* instead of skipping, the
            // third instance of this exact shape (see this function's own
            // doc comment above). Checked last, after the cheaper node/file
            // checks, since it is the least likely to fail in a normal dev
            // environment and this whole probe is already cached.
            vendor_eve_git_checkout_available(&repo_root())
        })
    }

    /// The workspace root two levels above this crate's own manifest —
    /// The browser validates every command result against
    /// `gamecult.eve.command_result.v1` before it reads a field of it, and
    /// that schema is `additionalProperties: false`. A flat result — state,
    /// message and ids at the top level rather than inside the receipt — is
    /// therefore not a lenient dialect: it is rejected whole, and the client
    /// reports "invalid Eve command result: data must not have additional
    /// properties" for every command, sign-in included.
    ///
    /// The assertion reads the contract off disk, from the pinned
    /// `vendor/eve` checkout, rather than restating it here. A test that
    /// spells the allowed keys itself agrees only with its own author; this
    /// one fails when the vendored contract moves under us.
    #[tokio::test]
    async fn a_command_result_satisfies_the_vendored_result_and_receipt_contracts() {
        let fixture = fixture().await;
        let denied = post(
            &fixture.state,
            &fixture.cookie,
            invocation(
                "world.approve",
                "ghostlight.world_approve.v0",
                0,
                json!({}),
                &uuid::Uuid::new_v4().to_string(),
            ),
        )
        .await;

        let schema_root = repo_root().join("vendor").join("eve").join("schemas");
        let result_schema: Value = serde_json::from_slice(
            &std::fs::read(schema_root.join("gamecult.eve.command_result.v1.schema.json"))
                .expect("the pinned vendor/eve checkout carries the result contract"),
        )
        .expect("the result contract is JSON");
        let receipt_schema: Value = serde_json::from_slice(
            &std::fs::read(schema_root.join("gamecult.eve.command_receipt.v1.schema.json"))
                .expect("the pinned vendor/eve checkout carries the receipt contract"),
        )
        .expect("the receipt contract is JSON");

        assert_eq!(
            result_schema["additionalProperties"],
            Value::Bool(false),
            "this test only means anything while the result contract is closed",
        );
        let allowed: Vec<&str> = result_schema["properties"]
            .as_object()
            .expect("the result contract names its properties")
            .keys()
            .map(String::as_str)
            .collect();
        for key in denied
            .as_object()
            .expect("a command result is an object")
            .keys()
        {
            assert!(
                allowed.contains(&key.as_str()),
                "`{key}` is not a property the result contract admits: {denied}",
            );
        }

        let receipt = &denied["receipt"];
        for required in receipt_schema["required"]
            .as_array()
            .expect("the receipt contract names its required fields")
        {
            let required = required.as_str().expect("a required field is named");
            assert!(
                !receipt[required].is_null(),
                "the receipt is missing its required `{required}`: {denied}",
            );
        }
        assert_eq!(receipt["state"], "denied", "{denied}");
        assert_eq!(
            receipt["schema"], "gamecult.eve.command_receipt.v1",
            "{denied}",
        );
    }

    /// `vendor/eve` lives here, not under `CARGO_MANIFEST_DIR` itself.
    fn repo_root() -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..")
    }

    /// PA.f190/PA.f193-C: every precondition
    /// `eve_client_bridge_fixture_matches_what_the_pinned_lowering_produces_today`
    /// itself asserts must be provable by `eve_client_bridge_is_available`
    /// before that test is allowed to run rather than skip — this is the one
    /// `eve_client_bridge_is_available` did not check.
    ///
    /// `git -C vendor/eve rev-parse HEAD` alone certifies nothing: when
    /// `vendor/eve` is a plain copied directory rather than a checkout, git
    /// does not fail — it walks up past the missing `.git` and answers with
    /// whichever *enclosing* repository it finds, which inside this checkout
    /// is Ghostlight's own HEAD. Soul measured this directly: the prior form
    /// of this check reported "available" for a copy sitting inside the
    /// Ghostlight repo, and
    /// `eve_client_bridge_fixture_matches_what_the_pinned_lowering_produces_today`
    /// then panicked comparing the fixture's stamped revision against the
    /// parent repo's HEAD instead of skipping.
    ///
    /// `--show-toplevel` names which repository git actually resolved the
    /// first command against. Comparing that, canonicalized, to `vendor/eve`'s
    /// own canonical path is what actually distinguishes "this directory is
    /// its own checkout" from "this directory sits inside someone else's". A
    /// bare tempdir outside any repository and a copy sitting inside one both
    /// now report unavailable, rather than only the former.
    /// `repo_root` is a parameter, not hard-coded, so a test can point this
    /// at a scratch directory shaped like a copied-not-checked-out
    /// `vendor/eve` without touching the real submodule.
    fn vendor_eve_git_checkout_available(repo_root: &std::path::Path) -> bool {
        let vendor_eve = repo_root.join("vendor").join("eve");
        let Ok(canonical_vendor_eve) = vendor_eve.canonicalize() else {
            return false;
        };
        let Ok(toplevel_output) = std::process::Command::new("git")
            .args(["-C", "vendor/eve", "rev-parse", "--show-toplevel"])
            .current_dir(repo_root)
            .output()
        else {
            return false;
        };
        if !toplevel_output.status.success() {
            return false;
        }
        let toplevel = String::from_utf8_lossy(&toplevel_output.stdout).trim().to_owned();
        let Ok(canonical_toplevel) = std::path::Path::new(&toplevel).canonicalize() else {
            return false;
        };
        if canonical_toplevel != canonical_vendor_eve {
            return false;
        }
        std::process::Command::new("git")
            .args(["-C", "vendor/eve", "rev-parse", "HEAD"])
            .current_dir(repo_root)
            .output()
            .is_ok_and(|output| output.status.success())
    }

    /// Runs a real `node` process that does exactly what
    /// `tools/eve_client_bridge.mjs` does at import time — resolve `jsdom`
    /// through the vendored lowering's own `package.json`, then dynamically
    /// import the built `dist/index.js` — and reports whether that succeeded.
    /// A missing `jsdom`, a missing `eve-contracts`/`ajv`, or any other
    /// unresolved module in that same import graph fails this the same way
    /// it would fail the real bridge script, which is the whole point: this
    /// is a probe built from the bridge's own actual dependency surface, not
    /// a second, independently-maintained guess at what it needs.
    fn eve_client_bridge_dependencies_resolve(lowering_root: &std::path::Path) -> bool {
        let lowering_root_js = lowering_root.to_string_lossy().replace('\\', "/");
        let probe = format!(
            "import {{ createRequire }} from \"node:module\";\n\
             import {{ join }} from \"node:path\";\n\
             const loweringRoot = \"{lowering_root_js}\";\n\
             const vendorRequire = createRequire(join(loweringRoot, \"package.json\"));\n\
             vendorRequire(\"jsdom\");\n\
             await import(\"file://\" + join(loweringRoot, \"dist\", \"index.js\").replace(/\\\\/g, \"/\"));\n"
        );
        let probe_path = std::env::temp_dir().join(format!(
            "ghostlight-eve-bridge-probe-{}-{}.mjs",
            std::process::id(),
            uuid::Uuid::new_v4()
        ));
        if std::fs::write(&probe_path, &probe).is_err() {
            return false;
        }
        let outcome = std::process::Command::new("node")
            .arg(&probe_path)
            .output();
        let _ = std::fs::remove_file(&probe_path);
        outcome.is_ok_and(|output| output.status.success())
    }

    /// PA.f190: `eve_client_bridge_is_available`'s own git precondition,
    /// isolated from the `node`/`dist` checks it also runs. Requires `git` on
    /// `PATH` (not `node`), so it is not itself gated by
    /// `eve_client_bridge_is_available`.
    ///
    /// Mutation: delete the `vendor_eve_git_checkout_available(&repo_root())`
    /// call from `eve_client_bridge_is_available` (or its early return) — a
    /// `vendor/eve` that is present but not a checkout (this test's own
    /// `copied` case) would then still report "available", exactly the shape
    /// Soul hit by accident and this test's own negative assertion catches.
    ///
    /// PA.f194-C: the positive case is gated on `vendor/eve/.git` actually
    /// existing, not merely on `git` being on `PATH`. Idunn's own frozen-
    /// source gate (`cargo test --locked -p ghostlight-dungeon --bin
    /// ghostlight-dungeon`, the exact command this repo's Yggdrasil runbook
    /// documents) materializes both the main tree and every Gitlink —
    /// `vendor/eve` included — byte for byte from the Git object store
    /// (`Idunn::drivers::materialize_tree_raw`/`materialize_gitlink_raw`),
    /// and that writer refuses to ever write `.git` metadata
    /// ("Git tree contains forbidden .git metadata", enforced on both the
    /// main tree and every Gitlink target). So inside Idunn's own gate,
    /// `vendor/eve` is unconditionally the "copied, not cloned" shape this
    /// test's negative case below proves unavailable — not a bug in the
    /// check, and not a shape this test's positive case should assert
    /// against. Checking `vendor/eve/.git` directly, rather than trusting
    /// `vendor_eve_git_checkout_available`'s own verdict, keeps this
    /// ground truth independent of the function under test.
    #[test]
    fn vendor_eve_git_checkout_available_distinguishes_a_checkout_from_a_copy() {
        // The real repo root: on a normal developer checkout `vendor/eve` is
        // a genuine git checkout (pinned submodule, `.git` is a gitlink file
        // pointing at the superproject's `.git/modules/...`), so this must
        // report true whenever `git` itself is on `PATH`. Inside Idunn's own
        // frozen-source gate `vendor/eve` carries no `.git` at all by design
        // (see the PA.f194-C doc comment above) — that is exactly the copy
        // shape this test's negative case proves unavailable, so the positive
        // assertion is skipped there instead of failing the gate.
        let vendor_eve_has_git_metadata = repo_root().join("vendor").join("eve").join(".git").exists();
        let git_on_path = std::process::Command::new("git")
            .arg("--version")
            .output()
            .is_ok_and(|output| output.status.success());
        if !vendor_eve_has_git_metadata {
            eprintln!(
                "SKIPPED vendor_eve_git_checkout_available_distinguishes_a_checkout_from_a_copy's own \
                 positive case: vendor/eve/.git does not exist here. Inside Idunn's own frozen-source \
                 gate this is expected (PA.f194-C, see the doc comment above) — the writer that \
                 materializes vendor/eve there deliberately never writes .git metadata. On a normal \
                 developer checkout this means the vendor/eve submodule was never initialized; run \
                 `git submodule update --init vendor/eve`."
            );
        } else if git_on_path {
            assert!(
                vendor_eve_git_checkout_available(&repo_root()),
                "vendor/eve carries .git metadata but still reports unavailable"
            );
        } else {
            eprintln!(
                "SKIPPED vendor_eve_git_checkout_available_distinguishes_a_checkout_from_a_copy's own \
                 positive case: `git` is not on PATH."
            );
        }

        // A `vendor/eve` that exists as a plain copied directory — carrying
        // real files (this test writes one) but no `.git` — is exactly the
        // shape Soul hit by accident: `dist/index.js` can still be present
        // (committed and copied along with everything else), so this is the
        // one check standing between that shape and a false "available".
        let scratch = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(scratch.path().join("vendor").join("eve")).unwrap();
        std::fs::write(scratch.path().join("vendor").join("eve").join("marker.txt"), b"copied, not cloned").unwrap();
        assert!(
            !vendor_eve_git_checkout_available(scratch.path()),
            "a copied vendor/eve directory with no .git must not report available"
        );
    }

    /// PA.f193-C: the bug Soul actually measured, built inside a real `git
    /// init` root rather than a bare tempdir. The prior test's own scratch
    /// directory sits outside any repository, so `git -C vendor/eve
    /// rev-parse HEAD` already failed there for an unrelated reason ("no such
    /// repository") and never exercised the actual defect: a naive `git -C
    /// vendor/eve rev-parse HEAD` walks up past the missing `.git` and
    /// answers with the *enclosing* repository's own HEAD instead of
    /// failing. This test proves that false positive exists (the sanity
    /// assertion) before proving the fix closes it.
    ///
    /// Mutation: revert `vendor_eve_git_checkout_available` to a bare `git -C
    /// vendor/eve rev-parse HEAD` success check (drop the `--show-toplevel`
    /// comparison) — the final assertion then fails, because the copy inside
    /// this enclosing repo reports available exactly like Soul found.
    #[test]
    fn vendor_eve_git_checkout_available_rejects_a_copy_inside_an_enclosing_repo() {
        let git_on_path = std::process::Command::new("git")
            .arg("--version")
            .output()
            .is_ok_and(|output| output.status.success());
        if !git_on_path {
            eprintln!(
                "SKIPPED vendor_eve_git_checkout_available_rejects_a_copy_inside_an_enclosing_repo: \
                 `git` is not on PATH."
            );
            return;
        }

        let scratch = tempfile::tempdir().unwrap();
        let init = std::process::Command::new("git")
            .args(["init", "-q"])
            .current_dir(scratch.path())
            .status();
        if !init.is_ok_and(|status| status.success()) {
            eprintln!(
                "SKIPPED vendor_eve_git_checkout_available_rejects_a_copy_inside_an_enclosing_repo: \
                 `git init` failed in the scratch directory."
            );
            return;
        }
        let commit = std::process::Command::new("git")
            .args([
                "-c",
                "user.email=ghostlight-test@example.com",
                "-c",
                "user.name=ghostlight-test",
                "commit",
                "--allow-empty",
                "-q",
                "-m",
                "scratch root for PA.f193-C",
            ])
            .current_dir(scratch.path())
            .status();
        if !commit.is_ok_and(|status| status.success()) {
            eprintln!(
                "SKIPPED vendor_eve_git_checkout_available_rejects_a_copy_inside_an_enclosing_repo: \
                 could not create the scratch repo's initial commit."
            );
            return;
        }

        // A copy, not a checkout: real files, no `.git`, sitting inside the
        // enclosing scratch repo just created above.
        std::fs::create_dir_all(scratch.path().join("vendor").join("eve")).unwrap();
        std::fs::write(scratch.path().join("vendor").join("eve").join("marker.txt"), b"copied, not cloned").unwrap();

        // Sanity: prove the false positive is real before proving the fix
        // closes it. If this fails, the scratch fixture stopped reproducing
        // Soul's bug and the test below would certify nothing either.
        let naive = std::process::Command::new("git")
            .args(["-C", "vendor/eve", "rev-parse", "HEAD"])
            .current_dir(scratch.path())
            .output();
        assert!(
            naive.is_ok_and(|output| output.status.success()),
            "the naive `git -C vendor/eve rev-parse HEAD` must succeed here — this is exactly the \
             false positive PA.f193-C found; if it no longer succeeds, this fixture stopped \
             reproducing the bug"
        );

        assert!(
            !vendor_eve_git_checkout_available(scratch.path()),
            "a vendor/eve copy sitting inside an enclosing repository must not report available"
        );
    }

    /// Strips the fields that legitimately vary between two otherwise
    /// identical intents — `idempotencyKey` (`randomIdempotencyKey`) and
    /// `issuedAt` (`new Date().toISOString()`), both minted fresh by
    /// `createEveCommandIntent` on every call — so a structural comparison
    /// judges the payload and routing shape the lowering actually produced,
    /// not two different timestamps.
    fn normalize_intent_for_diff(mut intent: Value) -> Value {
        if let Some(operation) = intent.get_mut("operation").and_then(Value::as_object_mut) {
            operation.remove("idempotencyKey");
        }
        if let Some(object) = intent.as_object_mut() {
            object.remove("issuedAt");
        }
        intent
    }

    /// PA.f162's own drift check: regenerates the same round trip through the
    /// real vendored lowering (`tools/eve_client_bridge.mjs`, submodule pin
    /// unmoved) and fails if it no longer matches the committed fixture —
    /// the fixture alone, without this, would be a frozen copy of the
    /// client's past behavior pretending to still be the client's current
    /// one. Runs only when `node` and the built lowering are actually
    /// available (`eve_client_bridge_is_available`). `#[ignore]`d for the
    /// same reason `world_create_seed_activate_and_play_round_trip_through_the_real_client`
    /// is (Soul's owed item 3, above): a normal `cargo test` run must
    /// distinguish "not exercised" from "passed" in its own summary line
    /// without `--nocapture`, and only `--ignored` opts in.
    #[tokio::test]
    #[ignore = "requires node + a built vendor/eve/eve-browser-lowering + eve-contracts; run with `cargo test -- --ignored`"]
    async fn eve_client_bridge_fixture_matches_what_the_pinned_lowering_produces_today() {
        if !eve_client_bridge_is_available() {
            panic!(
                "eve_client_bridge_fixture_matches_what_the_pinned_lowering_produces_today was run \
                 (via --ignored) but the real Eve client bridge is not available, so the committed \
                 fixture (src/fixtures/eve_client_bridge_intents.json) cannot be regenerated or \
                 diffed against it. Run `node` on PATH and both `npm install --prefix \
                 vendor/eve/packages/eve-browser-lowering` and `npm install --prefix \
                 vendor/eve/packages/eve-contracts`, then `npm run build --prefix \
                 vendor/eve/packages/eve-browser-lowering`, before running this test."
            );
        }
        let doc = bridge_fixture();
        assert_eq!(
            doc.vendor_eve_submodule_revision,
            std::process::Command::new("git")
                .args(["-C", "vendor/eve", "rev-parse", "HEAD"])
                .current_dir(repo_root())
                .output()
                .ok()
                .filter(|output| output.status.success())
                .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_owned())
                .expect(
                    "`git -C vendor/eve rev-parse HEAD` must succeed when the bridge is available \
                     (eve_client_bridge_is_available's own vendor_eve_git_checkout_available check \
                     should have skipped this test otherwise, PA.f190)",
                ),
            "the fixture's own stamped submodule revision no longer matches vendor/eve's checked-out \
             commit; regenerate the fixture (and update this stamp) against the revision actually \
             pinned, or the diff below is comparing against the wrong baseline"
        );

        let fixture = fixture().await;
        let provider = get(&fixture.state, &fixture.cookie, "/api/eve/provider").await;

        let empty_surface = get(&fixture.state, &fixture.cookie, "/api/eve/surfaces/ghostlight.play").await;
        let create_intent = client_intents(
            &empty_surface,
            &provider,
            json!([
                {"set": {
                    "world.create.title": "Bridge Fixture World",
                    "world.create.brief": "A world captured for the committed Eve client bridge fixture.",
                    "world.create.subject": "The Fixture Operator",
                    "world.create.targets": "{}",
                    "world.create.jurisdictions": "[]",
                    "world.create.lens_weights": "{\"patina\":1,\"charter\":1,\"ledger\":1,\"hearth\":1,\"tangle\":1,\"veil\":1,\"ember\":1,\"numen\":1}"
                }},
                {"click": "world.create"}
            ]),
        )
        .into_iter()
        .next()
        .unwrap();
        post(&fixture.state, &fixture.cookie, create_intent.clone()).await;

        let draft_surface = get(&fixture.state, &fixture.cookie, "/api/eve/surfaces/ghostlight.play").await;
        let approve_intent = client_intents(&draft_surface, &provider, json!([{"click": "world.approve"}]))
            .into_iter()
            .next()
            .unwrap();
        post(&fixture.state, &fixture.cookie, approve_intent.clone()).await;

        let approved_surface = get(&fixture.state, &fixture.cookie, "/api/eve/surfaces/ghostlight.play").await;
        let seed_intent = client_intents(&approved_surface, &provider, json!([{"click": "world.seed"}]))
            .into_iter()
            .next()
            .unwrap();
        post(&fixture.state, &fixture.cookie, seed_intent.clone()).await;

        let seeded_surface = get(&fixture.state, &fixture.cookie, "/api/eve/surfaces/ghostlight.play").await;
        let activate_intent = client_intents(&seeded_surface, &provider, json!([{"click": "world.activate"}]))
            .into_iter()
            .next()
            .unwrap();
        post(&fixture.state, &fixture.cookie, activate_intent.clone()).await;

        let active_surface = get(&fixture.state, &fixture.cookie, "/api/eve/surfaces/ghostlight.play").await;
        let play_intent = client_intents(
            &active_surface,
            &provider,
            json!([
                {"set": {"world.play.text": "The new owner looks around."}},
                {"click": "world.play"}
            ]),
        )
        .into_iter()
        .next()
        .unwrap();
        post(&fixture.state, &fixture.cookie, play_intent.clone()).await;

        let regenerated = [create_intent, approve_intent, seed_intent, activate_intent, play_intent];
        let committed = bridge_fixture().steps;
        assert_eq!(regenerated.len(), committed.len(), "step count drifted");
        for (regenerated_intent, committed_step) in regenerated.into_iter().zip(committed) {
            assert_eq!(
                normalize_intent_for_diff(regenerated_intent),
                normalize_intent_for_diff(committed_step.intent),
                "the real vendored lowering's own `{}` intent no longer matches the committed fixture; \
                 regenerate src/fixtures/eve_client_bridge_intents.json from the pinned submodule \
                 revision and commit the difference deliberately",
                committed_step.label
            );
        }
    }

    /// PA.f170's own round trip through the real client: with a question
    /// open, the "Play" button's own `props.action` (the answer token) and
    /// the bound `text` field must arrive together in one payload —
    /// `{"answerToken":..., "bindings":{"text":...}}` — exactly the shape
    /// `commandPayload` produces by spreading a button's own action fields
    /// into the payload beside `bindings`
    /// (`vendor/eve/packages/eve-browser-lowering/src/index.ts:2101`). This
    /// is not in the committed fixture above (that walkthrough's own
    /// `world.play` click never has an open question, so its action carries
    /// no token to prove); this test builds its own scripted question so the
    /// merge actually has something in `props.action` to carry. Skips, with
    /// `#[ignore]`d, same reason and opt-in as this file's other three
    /// bridge-dependent tests (Soul's owed item 3, above).
    #[tokio::test]
    #[ignore = "requires node + a built vendor/eve/eve-browser-lowering + eve-contracts; run with `cargo test -- --ignored`"]
    async fn the_real_client_carries_the_answer_token_and_the_bound_text_together() {
        if !eve_client_bridge_is_available() {
            panic!(
                "the_real_client_carries_the_answer_token_and_the_bound_text_together was run (via \
                 --ignored) but the real Eve client bridge is not available: `node` is missing, or \
                 the vendored lowering's built `dist/index.js`, its `jsdom` devDependency, or the \
                 sibling `eve-contracts` package's own `ajv` dependency does not resolve in this \
                 environment."
            );
        }
        let (fixture, served_version) = play_world_with_a_scripted_question().await;
        let table = fixture.state.play.clone().unwrap();

        let played = post(
            &fixture.state,
            &fixture.cookie,
            invocation(
                "world.play",
                "ghostlight.world_play.v0",
                served_version,
                json!({"text":"I stand at a crossroads."}),
                &uuid::Uuid::new_v4().to_string(),
            ),
        )
        .await;
        assert_eq!(played["receipt"]["state"], "accepted");

        tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                if let Some(view) = table.current_turn_view().await
                    && view.state == PlayTurnState::AwaitingPlayer
                {
                    return;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("the scripted ask_player call must open a question");

        let provider = get(&fixture.state, &fixture.cookie, "/api/eve/provider").await;
        let surface = get(&fixture.state, &fixture.cookie, "/api/eve/surfaces/ghostlight.play").await;
        let play_intent = client_intents(
            &surface,
            &provider,
            json!([
                {"set": {"world.play.text": "I go left."}},
                {"click": "world.play"}
            ]),
        )
        .into_iter()
        .next()
        .unwrap();

        let payload = play_intent["payload"].clone();
        assert!(
            payload
                .get("answerToken")
                .and_then(Value::as_str)
                .is_some_and(|token| !token.is_empty()),
            "the real client's own button action must carry a non-empty answerToken: {payload}"
        );
        assert_eq!(
            payload["bindings"]["text"], "I go left.",
            "the real client's own captured binding must still carry the bound text beside the \
             action's own answerToken: {payload}"
        );

        let answered = post(&fixture.state, &fixture.cookie, play_intent).await;
        assert_eq!(
            answered["receipt"]["state"], "accepted",
            "the real client's own combined payload must be accepted: {answered}"
        );

        let closed = tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                if let Some(view) = table.current_turn_view().await {
                    if view.state == PlayTurnState::Closed {
                        return view;
                    }
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("the scripted end_turn call must close the turn");
        assert_eq!(closed.narration.as_deref(), Some("The hall falls quiet."));
    }

    /// PA.f164: `targets`, `jurisdictions`, and `brief` are all documented
    /// "required, may be empty" (`CreatePayload`) — a legitimate deliberate
    /// choice, not a value nobody supplied. Before this fix, only `lens_weights`
    /// carried an authored `value` the real lowering's own
    /// `findAuthoredBindingValue` could capture without the field being typed
    /// into; the other three carried only a `placeholder`, which the lowering
    /// never captures at all (`src/index.ts`'s own `findAuthoredBindingValue`
    /// reads `props.value`, never `props.placeholder`). A form filled exactly
    /// as its labels suggest — the identity fields a real owner actually types,
    /// `title`/`subject`, and nothing else — was refused with "missing field
    /// targets". This test drives the real vendored lowering the same way,
    /// typing only `title` and `subject`, and the command must still be
    /// accepted.
    /// `#[ignore]`d, same reason and opt-in as this file's other three
    /// bridge-dependent tests (Soul's owed item 3, above).
    #[tokio::test]
    #[ignore = "requires node + a built vendor/eve/eve-browser-lowering + eve-contracts; run with `cargo test -- --ignored`"]
    async fn a_create_form_filled_only_by_its_labelled_identity_fields_is_still_accepted() {
        if !eve_client_bridge_is_available() {
            panic!(
                "a_create_form_filled_only_by_its_labelled_identity_fields_is_still_accepted was run \
                 (via --ignored) but the real Eve client bridge is not available: `node` is missing, \
                 or the vendored lowering's built `dist/index.js`, its `jsdom` devDependency, or the \
                 sibling `eve-contracts` package's own `ajv` dependency does not resolve in this \
                 environment. Run `node` on PATH and both `npm install --prefix \
                 vendor/eve/packages/eve-browser-lowering` and `npm install --prefix \
                 vendor/eve/packages/eve-contracts`, then `npm run build --prefix \
                 vendor/eve/packages/eve-browser-lowering`, before running this test."
            );
        }
        let fixture = fixture().await;
        let provider = get(&fixture.state, &fixture.cookie, "/api/eve/provider").await;
        let empty_surface = get(&fixture.state, &fixture.cookie, "/api/eve/surfaces/ghostlight.play").await;

        let create_intents = client_intents(
            &empty_surface,
            &provider,
            json!([
                {"set": {
                    "world.create.title": "Untouched Fields World",
                    "world.create.subject": "The Owner"
                }},
                {"click": "world.create"}
            ]),
        );
        assert_eq!(create_intents.len(), 1);
        let intent = create_intents.into_iter().next().unwrap();
        // The real lowering's own captured payload — not a hand-built one —
        // must have fallen back to the authored `value`s this cut seeded
        // rather than omitting the fields outright.
        let bindings = &intent["payload"]["bindings"];
        assert_eq!(bindings["brief"], "");
        assert_eq!(bindings["targets"], "{}");
        assert_eq!(bindings["jurisdictions"], "[]");

        let created = post(&fixture.state, &fixture.cookie, intent).await;
        assert_eq!(
            created["receipt"]["state"], "accepted",
            "a form filled only by its labelled identity fields must still be accepted: {created}"
        );
    }

    /// PA.f168 originally refused any sibling next to the envelope
    /// `bindings` object outright. PA.f170 changes what a real client's
    /// payload carries — a button's own `props.action` fields (the answer
    /// token, for `world.play`) ride beside `bindings` on purpose — so
    /// `unwrap_bindings` now merges the two instead. An unrecognized sibling
    /// is still refused, just one door downstream: the merged object still
    /// has to satisfy the operation's own payload struct, and
    /// `world.approve`'s `EmptyPayload` (`#[serde(deny_unknown_fields)]`)
    /// declares none at all, so `"rogue"` is still an unknown field there.
    ///
    /// Mutation: have `unwrap_bindings` silently drop an unrecognized sibling
    /// instead of merging it in — this command would then be accepted, with
    /// `"rogue"` discarded rather than refused.
    #[tokio::test]
    async fn a_payload_with_an_unrecognized_sibling_field_beside_bindings_is_refused() {
        let fixture = fixture().await;
        let created = post(
            &fixture.state,
            &fixture.cookie,
            invocation(
                "world.create",
                "ghostlight.world_create.v4",
                0,
                json!({
                    "title":"Sibling Field World",
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
        assert_eq!(created["receipt"]["state"], "accepted");

        let denied = post(
            &fixture.state,
            &fixture.cookie,
            invocation(
                "world.approve",
                "ghostlight.world_approve.v0",
                1,
                json!({"bindings":{}, "rogue":"a field no real client ever sends beside bindings"}),
                &uuid::Uuid::new_v4().to_string(),
            ),
        )
        .await;
        assert_eq!(denied["receipt"]["state"], "denied");
        assert!(
            denied["receipt"]["message"].as_str().unwrap_or_default().contains("rogue"),
            "an unrecognized sibling field beside the envelope must still be refused, not silently \
             discarded: {denied}"
        );
    }

    /// PA.f170's own negative on the merge itself: a payload naming the same
    /// field both inside `bindings` and as a sibling of it — a shape nothing
    /// on the real lowering's own side produces (`commandPayload` builds
    /// `bindings` and the action's own fields from disjoint sources) — is
    /// refused as an outright collision, never resolved by silently
    /// preferring one side over the other.
    ///
    /// Mutation: have `unwrap_bindings` prefer the sibling (or the `bindings`
    /// entry) on a name collision instead of refusing it — this command
    /// would then be accepted with one of the two conflicting `"minutes"`
    /// values silently discarded.
    #[tokio::test]
    async fn a_payload_naming_the_same_field_in_bindings_and_as_a_sibling_is_refused() {
        let fixture = fixture().await;
        let created = post(
            &fixture.state,
            &fixture.cookie,
            invocation(
                "world.create",
                "ghostlight.world_create.v4",
                0,
                json!({
                    "title":"Colliding Field World",
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
        assert_eq!(created["receipt"]["state"], "accepted");
        let approved = post(
            &fixture.state,
            &fixture.cookie,
            invocation("world.approve", "ghostlight.world_approve.v0", 1, json!({}), &uuid::Uuid::new_v4().to_string()),
        )
        .await;
        assert_eq!(approved["receipt"]["state"], "accepted");
        let activated = post(
            &fixture.state,
            &fixture.cookie,
            invocation("world.activate", "ghostlight.world_activate.v0", 2, json!({}), &uuid::Uuid::new_v4().to_string()),
        )
        .await;
        assert_eq!(activated["receipt"]["state"], "accepted");

        let denied = post(
            &fixture.state,
            &fixture.cookie,
            invocation(
                "world.advance_time",
                "ghostlight.world_advance_time.v0",
                3,
                json!({"bindings":{"minutes":5}, "minutes":9}),
                &uuid::Uuid::new_v4().to_string(),
            ),
        )
        .await;
        assert_eq!(denied["receipt"]["state"], "denied");
        assert!(
            denied["receipt"]["message"].as_str().unwrap_or_default().contains("minutes"),
            "a field named by both bindings and a sibling must be refused as a collision: {denied}"
        );
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
        assert_eq!(created["receipt"]["state"], "accepted");
        assert_eq!(created["receipt"]["sourceVersion"], 1);

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
        assert_eq!(approved["receipt"]["sourceVersion"], 2);
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
        assert_eq!(activated["receipt"]["sourceVersion"], 3);

        let played = post(
            &fixture.state,
            &fixture.cookie,
            invocation(
                "world.play",
                "ghostlight.world_play.v0",
                3,
                json!({"text":"The new owner looks around."}),
                &uuid::Uuid::new_v4().to_string(),
            ),
        )
        .await;
        assert_eq!(played["receipt"]["state"], "accepted");

        // PA.f138: this leg observes the play table itself, not merely the
        // accepted response — a route that spawned the task and handed back
        // the same JSON without ever calling `table.run` would still pass
        // the assertion above, so `world_play_reaches_the_play_table_and_is_
        // accepted`'s own poll is what actually proves the turn opened.
        let table = fixture.state.play.clone().unwrap();
        let observed = tokio::time::timeout(Duration::from_secs(5), async move {
            loop {
                if table.current_turn_view().await.is_some() {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await;
        assert!(
            observed.is_ok(),
            "world.play must actually reach PlayTable::run, recording a turn row, not just return \
             the same accepted JSON on its own"
        );
    }

    /// Cut 8b's own routing test: `world.play` reaches the play table and
    /// returns `accepted` — the request routes and returns before the turn
    /// itself resolves, exactly as the spawned-task shape calls for. The
    /// fixture's own play table shares the fixture's unreachable test
    /// inference organ (`127.0.0.1:9`), so the spawned turn will itself fail
    /// once it actually tries to infer; that failure is expected and is not
    /// this test's concern; only the HTTP response this route hands back
    /// before that happens is.
    ///
    /// PA.f138: `played["receipt"]["state"] == "accepted"` alone is reachable by a
    /// mutation that spawns the task and returns the same JSON without ever
    /// calling `table.run` — this route builds `{"kind":"accepted"}` from
    /// nothing on the table itself. This test also polls
    /// `PlayTable::current_turn_view` (PA.f134's own consumer-facing read)
    /// until it reports a turn, bounded by a timeout: `table.run` genuinely
    /// ran only if a turn row ever appears, whatever it did with it.
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
        assert_eq!(created["receipt"]["state"], "accepted");

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
        assert_eq!(played["receipt"]["state"], "accepted");

        let table = fixture.state.play.clone().unwrap();
        let observed = tokio::time::timeout(Duration::from_secs(5), async move {
            loop {
                if table.current_turn_view().await.is_some() {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await;
        assert!(
            observed.is_ok(),
            "the spawned task must actually call table.run, recording a turn row, not just return \
             the same accepted JSON on its own"
        );
    }

    /// PA.f148/PA.f160: Soul's own probe — the served document's `version`
    /// used to fold the play row's own revision in, so a play commit alone
    /// (no kernel patch) could move `version` to a number `world.revision`
    /// never reached, and every later command's compare-and-swap derived
    /// `expected_revision` from it and was permanently denied. This drives
    /// the exact same shape: open a play turn (it commits nothing to the
    /// world here — the fixture's own inference organ is unreachable, so it
    /// faults immediately, which still exercises the row's own revision
    /// counter the same way a real committed refusal or question would),
    /// wait for the row to move, then read `sourceVersion` from the actually
    /// served surface document — as a client does — and submit
    /// `world.advance_time` against it. It must still be accepted.
    #[tokio::test]
    async fn a_play_commit_does_not_poison_the_next_world_commands_compare_and_swap() {
        let fixture = fixture().await;
        let created = post(
            &fixture.state,
            &fixture.cookie,
            invocation(
                "world.create",
                "ghostlight.world_create.v4",
                0,
                json!({
                    "title":"CAS World",
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
        assert_eq!(created["receipt"]["state"], "accepted");
        let approved = post(
            &fixture.state,
            &fixture.cookie,
            invocation("world.approve", "ghostlight.world_approve.v0", 1, json!({}), &uuid::Uuid::new_v4().to_string()),
        )
        .await;
        assert_eq!(approved["receipt"]["state"], "accepted");
        let activated = post(
            &fixture.state,
            &fixture.cookie,
            invocation("world.activate", "ghostlight.world_activate.v0", 2, json!({}), &uuid::Uuid::new_v4().to_string()),
        )
        .await;
        assert_eq!(activated["receipt"]["state"], "accepted");
        assert_eq!(activated["receipt"]["sourceVersion"], 3, "world.revision is 3, and nothing has folded anything else into it yet");

        let played = post(
            &fixture.state,
            &fixture.cookie,
            invocation("world.play", "ghostlight.world_play.v0", 3, json!({"text":"I look around."}), &uuid::Uuid::new_v4().to_string()),
        )
        .await;
        assert_eq!(played["receipt"]["state"], "accepted");

        let table = fixture.state.play.clone().unwrap();
        tokio::time::timeout(Duration::from_secs(5), async move {
            loop {
                if let Some(view) = table.current_turn_view().await {
                    if view.revision > 0 {
                        break;
                    }
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("the play row's own revision must move once the turn commits its own row");

        // As a client does: read `version` off the actually served document,
        // never re-derive it from `world.revision` directly.
        let surface = get(&fixture.state, &fixture.cookie, "/api/eve/surfaces/ghostlight.play").await;
        let served_version = surface["version"].as_u64().expect("the surface document names a version");
        assert_eq!(
            served_version, 3,
            "PA.f148: the document's own version must still name world.revision alone, not the play row's \
             revision folded in — Soul's own probe was a version that did not"
        );

        let advanced = post(
            &fixture.state,
            &fixture.cookie,
            invocation(
                "world.advance_time",
                "ghostlight.world_advance_time.v0",
                served_version,
                json!({"minutes": 5}),
                &uuid::Uuid::new_v4().to_string(),
            ),
        )
        .await;
        assert_eq!(
            advanced["receipt"]["state"], "accepted",
            "a play commit must never poison the next world command's compare-and-swap: {advanced}"
        );
    }

    /// The create/approve/activate boilerplate every scripted-question
    /// harness below needs, factored out so `play_world_with_a_scripted_question`
    /// and `play_world_with_two_scripted_questions` (Cut 13) do not each
    /// repeat it. Returns the fixture and the world's `sourceVersion` after
    /// activation.
    async fn activated_fixture(title: &str) -> (Fixture, u64) {
        let fixture = fixture().await;
        let created = post(
            &fixture.state,
            &fixture.cookie,
            invocation(
                "world.create",
                "ghostlight.world_create.v4",
                0,
                json!({
                    "title":title,
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
        assert_eq!(created["receipt"]["state"], "accepted");
        let approved = post(
            &fixture.state,
            &fixture.cookie,
            invocation("world.approve", "ghostlight.world_approve.v0", 1, json!({}), &uuid::Uuid::new_v4().to_string()),
        )
        .await;
        assert_eq!(approved["receipt"]["state"], "accepted");
        let activated = post(
            &fixture.state,
            &fixture.cookie,
            invocation("world.activate", "ghostlight.world_activate.v0", 2, json!({}), &uuid::Uuid::new_v4().to_string()),
        )
        .await;
        assert_eq!(activated["receipt"]["state"], "accepted");
        let served_version = activated["receipt"]["sourceVersion"].as_u64().unwrap();
        (fixture, served_version)
    }

    /// PA.f161's own harness: builds a play world through the real HTTP
    /// route, then swaps `state.play` for a table over `crate::play::tests`'
    /// own `ScriptedPort`, scripted to call `ask_player` and then `end_turn`
    /// — a deterministic question this test can actually answer through the
    /// route, which the fixture's own unreachable test inference organ
    /// cannot produce. Returns the fixture and the world's `sourceVersion`
    /// after activation (still a required `routeHint.sourceVersion` field on
    /// every command envelope, even though `world.play` no longer reads it
    /// for staleness — PA.f170 judges an answer by its own token instead).
    async fn play_world_with_a_scripted_question() -> (Fixture, u64) {
        use crate::play::tests::{ScriptedPort, call_event, output, text_event};
        let (fixture, served_version) = activated_fixture("Scripted Question World").await;

        let ask = output(
            "r0",
            vec![call_event(
                "c0",
                "ask_player",
                serde_json::json!({"question": "Which way?"}),
            )],
        );
        let end = output("r1", vec![call_event("c1", "end_turn", serde_json::json!({}))]);
        let personas = PersonaLane::new(
            ControllerPort::new(fixture.state.world.clone()),
            ScriptedPort::new(vec![output("proj-0", vec![text_event("The hall falls quiet.")])]),
            "gpt-5.6-sol".into(),
            "gpt-5.6-sol".into(),
        )
        .unwrap();
        let directory = tempfile::tempdir().unwrap();
        let table = PlayTable::new(
            fixture.state.world.clone(),
            personas,
            ScriptedPort::new(vec![ask, end]),
            "gpt-5.6-terra".into(),
            Arc::new(Semaphore::new(TEST_CONTROLLER_CONCURRENCY)),
            directory.path().join("play-turn-v1.cc"),
        )
        .unwrap();
        let mut fixture = fixture;
        fixture.state.play = Some(Arc::new(table));
        fixture._play_directory = Some(directory);
        (fixture, served_version)
    }

    /// PA.f170's own harness: two scripted `ask_player` calls, one per round,
    /// with no `end_turn` — so answering Q1 leaves the turn `AwaitingPlayer`
    /// on Q2 rather than closed, the exact shape Soul's own probe needed to
    /// show an answer composed for Q1 landing on Q2.
    async fn play_world_with_two_scripted_questions() -> (Fixture, u64) {
        use crate::play::tests::{ScriptedPort, call_event, output};
        let (fixture, served_version) = activated_fixture("Two Scripted Questions World").await;

        let ask_one = output(
            "r0",
            vec![call_event("c0", "ask_player", serde_json::json!({"question": "Which way?"}))],
        );
        let ask_two = output(
            "r1",
            vec![call_event("c1", "ask_player", serde_json::json!({"question": "Are you sure?"}))],
        );
        let personas = PersonaLane::new(
            ControllerPort::new(fixture.state.world.clone()),
            ScriptedPort::new(vec![]),
            "gpt-5.6-sol".into(),
            "gpt-5.6-sol".into(),
        )
        .unwrap();
        let directory = tempfile::tempdir().unwrap();
        let table = PlayTable::new(
            fixture.state.world.clone(),
            personas,
            ScriptedPort::new(vec![ask_one, ask_two]),
            "gpt-5.6-terra".into(),
            Arc::new(Semaphore::new(TEST_CONTROLLER_CONCURRENCY)),
            directory.path().join("play-turn-v1.cc"),
        )
        .unwrap();
        let mut fixture = fixture;
        fixture.state.play = Some(Arc::new(table));
        fixture._play_directory = Some(directory);
        (fixture, served_version)
    }

    /// Finds the surface node named `id`, depth-first, the same way
    /// `crate::play::tests::find_surface_node` does for `play.rs`'s own
    /// tests — duplicated locally rather than exposed `pub(crate)` across a
    /// module boundary for one small helper.
    fn find_surface_node<'a>(node: &'a Value, id: &str) -> Option<&'a Value> {
        if node.get("id").and_then(Value::as_str) == Some(id) {
            return Some(node);
        }
        for child in node.get("children").and_then(Value::as_array).into_iter().flatten() {
            if let Some(found) = find_surface_node(child, id) {
                return Some(found);
            }
        }
        None
    }

    /// Reads the open question's own answer token the same way a real
    /// client does (PA.f170): off the "Play" button's own served
    /// `props.action.answerToken`, from the actually served surface document
    /// — never from `PlayTurnView`/`OpenQuestion` directly, which a test
    /// could read but a browser never does.
    async fn play_button_token(state: &AppState, cookie: &str) -> Option<String> {
        let surface = get(state, cookie, "/api/eve/surfaces/ghostlight.play").await;
        let root = surface.get("surface")?.get("root")?;
        let button = find_surface_node(root, "world.play")?;
        button
            .get("props")?
            .get("action")?
            .get("answerToken")?
            .as_str()
            .map(str::to_owned)
    }

    /// Drives `play_world_with_a_scripted_question` through `world.play` and
    /// waits for the scripted `ask_player` call to open the turn's own
    /// question, returning the fixture, table, and served version so each
    /// caller only writes its own answer's own assertions.
    async fn played_and_awaiting_question() -> (Fixture, Arc<PlayTable>, u64) {
        let (fixture, served_version) = play_world_with_a_scripted_question().await;
        let table = fixture.state.play.clone().unwrap();

        let played = post(
            &fixture.state,
            &fixture.cookie,
            invocation(
                "world.play",
                "ghostlight.world_play.v0",
                served_version,
                json!({"text":"I stand at a crossroads."}),
                &uuid::Uuid::new_v4().to_string(),
            ),
        )
        .await;
        assert_eq!(played["receipt"]["state"], "accepted");

        tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                if let Some(view) = table.current_turn_view().await
                    && view.state == PlayTurnState::AwaitingPlayer
                {
                    return;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("the scripted ask_player call must open a question");
        (fixture, table, served_version)
    }

    /// PA.f170's own end-to-end proof: a question asked, the card served,
    /// its own answer token read out of the served surface document — never
    /// out of `PlayTurnView` directly, which a real client never sees — the
    /// answer accepted, and the turn closes with the scripted narration.
    ///
    /// Mutation: change the `matches!((payload.answer_token.as_deref(),
    /// question.token.as_deref()), (Some(given), Some(expected)) if given ==
    /// expected)` gate at `runtime.rs`'s `world.play` arm to always pass (or
    /// always fail) — this test's own answer, echoing the exact token the
    /// surface served, would then be wrongly refused (or a broken-but-lenient
    /// comparison would let a later mismatch test wrongly pass).
    #[tokio::test]
    async fn an_answer_naming_the_open_questions_own_token_read_from_the_served_surface_is_accepted() {
        let (fixture, table, served_version) = played_and_awaiting_question().await;

        let token = play_button_token(&fixture.state, &fixture.cookie)
            .await
            .expect("the served surface's own \"Play\" button must carry the open question's answer token");

        let answered = post(
            &fixture.state,
            &fixture.cookie,
            invocation(
                "world.play",
                "ghostlight.world_play.v0",
                served_version,
                json!({"text":"I go left.","answerToken":token}),
                &uuid::Uuid::new_v4().to_string(),
            ),
        )
        .await;
        assert_eq!(
            answered["receipt"]["state"], "accepted",
            "an answer naming the open question's own token, read off the served surface, must be accepted: {answered}"
        );

        let closed = tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                if let Some(view) = table.current_turn_view().await {
                    if view.state == PlayTurnState::Closed {
                        return view;
                    }
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("the scripted end_turn call must close the turn");
        assert_eq!(closed.narration.as_deref(), Some("The hall falls quiet."));
    }

    /// PA.f170's own refusal proof: an answer that carries no token at all
    /// while a question is open is refused as stale, and the turn stays open
    /// rather than being resumed by an answer that could not have named the
    /// question the player was actually shown.
    ///
    /// Mutation: change the `matches!` gate at `runtime.rs`'s `world.play`
    /// arm to treat a missing token (`None`) as a match — this test's
    /// answer, carrying no `answerToken` field at all, would then be
    /// wrongly accepted.
    #[tokio::test]
    async fn an_answer_missing_the_open_questions_own_token_is_refused_as_stale() {
        let (fixture, table, served_version) = played_and_awaiting_question().await;

        let stale_answered = post(
            &fixture.state,
            &fixture.cookie,
            invocation(
                "world.play",
                "ghostlight.world_play.v0",
                served_version,
                json!({"text":"I go left."}),
                &uuid::Uuid::new_v4().to_string(),
            ),
        )
        .await;
        assert_eq!(stale_answered["receipt"]["state"], "denied");
        assert!(
            stale_answered["receipt"]["message"]
                .as_str()
                .unwrap_or_default()
                .contains("stale"),
            "an answer with no token at all must be refused as stale: {stale_answered}"
        );

        let still_open = table.current_turn_view().await.unwrap();
        assert_eq!(
            still_open.state,
            PlayTurnState::AwaitingPlayer,
            "a stale answer must leave the question open, not resume the turn"
        );
    }

    /// PA.f170: an answer naming a token that does not match the currently
    /// open question's own — forged, mistyped, or simply wrong — is refused
    /// as stale exactly like a missing one, never applied to the open
    /// question.
    ///
    /// Mutation: compare only a prefix or a hash of the token instead of the
    /// exact string — this test's near-miss token (the real one with its
    /// last character flipped) would then be wrongly accepted for at least
    /// some mutations of that weakened comparison.
    #[tokio::test]
    async fn an_answer_naming_a_mismatched_token_is_refused_as_stale() {
        let (fixture, table, served_version) = played_and_awaiting_question().await;

        let real_token = play_button_token(&fixture.state, &fixture.cookie)
            .await
            .expect("the served surface must carry the open question's own answer token");
        let mut forged = real_token.clone();
        forged.push('x');

        let stale_answered = post(
            &fixture.state,
            &fixture.cookie,
            invocation(
                "world.play",
                "ghostlight.world_play.v0",
                served_version,
                json!({"text":"I go left.","answerToken":forged}),
                &uuid::Uuid::new_v4().to_string(),
            ),
        )
        .await;
        assert_eq!(stale_answered["receipt"]["state"], "denied");
        assert!(
            stale_answered["receipt"]["message"]
                .as_str()
                .unwrap_or_default()
                .contains("stale"),
            "a mismatched token must be refused as stale: {stale_answered}"
        );

        let still_open = table.current_turn_view().await.unwrap();
        assert_eq!(
            still_open.state,
            PlayTurnState::AwaitingPlayer,
            "a stale answer must leave the question open, not resume the turn"
        );
    }

    /// PA.f178: a stale tab — one still holding a render of a question that
    /// has since been answered and closed — resubmits its own spent
    /// `answerToken` alongside new prose. Before this cut, with no question
    /// currently open the token was simply never read, and the prose opened
    /// a *fresh* turn instead of being refused: a real inference budget spent
    /// answering a question that no longer exists. The token must instead be
    /// refused outright, and no new turn opened.
    ///
    /// Mutation: delete the `payload.answer_token.is_some() &&
    /// !is_awaiting_player` guard from `execute_world`'s `world.play` arm —
    /// this test's stale-tab resubmission would then be accepted, opening a
    /// second turn, and both assertions below would fail (the response
    /// would read `"accepted"`, and `current_turn_view`'s own `turn_id`
    /// would move on from the first, closed turn).
    #[tokio::test]
    async fn a_stale_tabs_spent_token_is_refused_and_never_opens_a_new_turn() {
        let (fixture, table, served_version) = played_and_awaiting_question().await;

        let spent_token = play_button_token(&fixture.state, &fixture.cookie)
            .await
            .expect("the served surface must carry the open question's own answer token");

        let answered = post(
            &fixture.state,
            &fixture.cookie,
            invocation(
                "world.play",
                "ghostlight.world_play.v0",
                served_version,
                json!({"text":"I go left.","answerToken":spent_token}),
                &uuid::Uuid::new_v4().to_string(),
            ),
        )
        .await;
        assert_eq!(answered["receipt"]["state"], "accepted");

        let closed = tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                if let Some(view) = table.current_turn_view().await {
                    if view.state == PlayTurnState::Closed {
                        return view;
                    }
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("the scripted end_turn call must close the turn");
        assert_eq!(closed.narration.as_deref(), Some("The hall falls quiet."));
        let closed_turn_id = closed.turn_id.clone();

        // The stale tab, still holding its own now-spent token, resubmits —
        // with a fresh idempotency key, so a key-replay refusal cannot be
        // mistaken for this one.
        let stale_resubmission = post(
            &fixture.state,
            &fixture.cookie,
            invocation(
                "world.play",
                "ghostlight.world_play.v0",
                served_version,
                json!({"text":"a followup the player never actually meant to send here","answerToken":spent_token}),
                &uuid::Uuid::new_v4().to_string(),
            ),
        )
        .await;
        assert_eq!(
            stale_resubmission["receipt"]["state"], "denied",
            "a spent token, resubmitted once no question is open, must be refused: {stale_resubmission}"
        );
        assert!(
            stale_resubmission["receipt"]["message"]
                .as_str()
                .unwrap_or_default()
                .contains("closed"),
            "the refusal must name the actual defect (the question is closed), not a generic decode error: \
             {stale_resubmission}"
        );

        let still_closed = table.current_turn_view().await.unwrap();
        assert_eq!(
            still_closed.turn_id, closed_turn_id,
            "a refused stale resubmission must never open a fresh turn"
        );
        assert_eq!(still_closed.state, PlayTurnState::Closed);
    }

    /// PA.f180: the real client never emits `answerToken` as a captured
    /// binding — `eve.rs` renders it DOM-invisibly on the "Play" button's own
    /// `props.action`, never through any control a `captureBindings` value
    /// could come from — but nothing before this cut refused a request built
    /// by hand (or by a compromised/buggy client) that sent the *correct*
    /// token through the `bindings` channel instead of the action channel.
    /// `unwrap_bindings` used to flatten both into one map with no memory of
    /// which channel a field arrived on, so this would have been
    /// indistinguishable from a real answer and accepted.
    ///
    /// Mutation: remove the `ACTION_ONLY_PAYLOAD_FIELDS` check from
    /// `unwrap_bindings` — this test's answer, sending the real token through
    /// `bindings` instead of as a sibling, would then be wrongly accepted
    /// and the turn would close.
    #[tokio::test]
    async fn an_answer_token_sent_as_a_captured_binding_is_refused_not_admitted() {
        let (fixture, table, served_version) = played_and_awaiting_question().await;

        let real_token = play_button_token(&fixture.state, &fixture.cookie)
            .await
            .expect("the served surface must carry the open question's own answer token");

        let smuggled = post(
            &fixture.state,
            &fixture.cookie,
            invocation(
                "world.play",
                "ghostlight.world_play.v0",
                served_version,
                json!({"bindings":{"text":"I go left.","answerToken":real_token}}),
                &uuid::Uuid::new_v4().to_string(),
            ),
        )
        .await;
        assert_eq!(
            smuggled["receipt"]["state"], "denied",
            "the real token, sent through the wrong channel, must still be refused: {smuggled}"
        );
        assert!(
            smuggled["receipt"]["message"]
                .as_str()
                .unwrap_or_default()
                .contains("captured binding"),
            "the refusal must name the actual defect (wrong channel), not a generic decode error: {smuggled}"
        );

        let still_open = table.current_turn_view().await.unwrap();
        assert_eq!(
            still_open.state,
            PlayTurnState::AwaitingPlayer,
            "a smuggled-channel token must leave the question open, not resume the turn"
        );
    }

    /// PA.f180's negative: every other command's own round trip — the action
    /// spread and the envelope `bindings` arriving together, as the real
    /// lowering actually sends them — must keep working; the new check
    /// refuses only a reserved action-only field found inside `bindings`,
    /// never an ordinary field arriving in its usual place.
    ///
    /// Mutation: widen `ACTION_ONLY_PAYLOAD_FIELDS` to also cover `"text"` —
    /// this test's ordinary answer, whose `text` legitimately arrives via
    /// `bindings`, would then be wrongly refused.
    #[tokio::test]
    async fn an_ordinary_answer_with_text_via_bindings_and_token_as_a_sibling_is_still_accepted() {
        let (fixture, table, served_version) = played_and_awaiting_question().await;

        let real_token = play_button_token(&fixture.state, &fixture.cookie)
            .await
            .expect("the served surface must carry the open question's own answer token");

        let answered = post(
            &fixture.state,
            &fixture.cookie,
            invocation(
                "world.play",
                "ghostlight.world_play.v0",
                served_version,
                json!({"answerToken":real_token,"bindings":{"text":"I go left."}}),
                &uuid::Uuid::new_v4().to_string(),
            ),
        )
        .await;
        assert_eq!(
            answered["receipt"]["state"], "accepted",
            "the real client's own shape — action fields as siblings, captured values inside \
             bindings — must still be accepted: {answered}"
        );

        let closed = tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                if let Some(view) = table.current_turn_view().await {
                    if view.state == PlayTurnState::Closed {
                        return view;
                    }
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("the scripted end_turn call must close the turn");
        assert_eq!(closed.narration.as_deref(), Some("The hall falls quiet."));
    }

    /// PA.f170's own core proof — the defect Soul demonstrated (an answer
    /// composed for Q1 lands on Q2, the player never saw): two questions open
    /// in the same turn, an answer carries Q1's own token, and by the time it
    /// is submitted the turn has moved on to Q2. `question_surface_version`
    /// could not refuse this: asking is conversational and never moves the
    /// world's own revision, so Q1 and Q2 would have carried the same
    /// version, and a version-only comparison could not tell them apart. The
    /// per-question token can, and must.
    ///
    /// Mutation: have `runtime.rs`'s `world.play` arm resolve `answers` from
    /// `payload.text` alone, ignoring `answer_token` entirely (reverting to
    /// "any request while a question is open answers whatever is open now")
    /// — this test's Q1-token answer would then be wrongly accepted against
    /// Q2.
    #[tokio::test]
    async fn soul_an_answer_composed_for_q1_lands_on_q2_the_player_never_saw() {
        let (fixture, served_version) = play_world_with_two_scripted_questions().await;
        let table = fixture.state.play.clone().unwrap();

        let played = post(
            &fixture.state,
            &fixture.cookie,
            invocation(
                "world.play",
                "ghostlight.world_play.v0",
                served_version,
                json!({"text":"I stand at a crossroads."}),
                &uuid::Uuid::new_v4().to_string(),
            ),
        )
        .await;
        assert_eq!(played["receipt"]["state"], "accepted");

        tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                if let Some(view) = table.current_turn_view().await
                    && view.question.as_ref().is_some_and(|question| question.text == "Which way?")
                {
                    return;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("the first scripted ask_player call must open Q1");

        // The player reads Q1's own token off the served surface — exactly
        // what a real client's "Play" button would carry — but does not
        // answer it before the table moves on to Q2.
        let q1_token = play_button_token(&fixture.state, &fixture.cookie)
            .await
            .expect("the served surface must carry Q1's own answer token");

        let answered_q1 = post(
            &fixture.state,
            &fixture.cookie,
            invocation(
                "world.play",
                "ghostlight.world_play.v0",
                served_version,
                json!({"text":"left","answerToken":q1_token}),
                &uuid::Uuid::new_v4().to_string(),
            ),
        )
        .await;
        assert_eq!(
            answered_q1["receipt"]["state"], "accepted",
            "Q1's own token must still answer Q1: {answered_q1}"
        );

        tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                if let Some(view) = table.current_turn_view().await
                    && view.question.as_ref().is_some_and(|question| question.text == "Are you sure?")
                {
                    return;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("answering Q1 must open Q2");

        // A late request still carrying Q1's own token — the exact shape a
        // slow retry or a second tab would produce — must never be applied
        // to Q2, which the player has not been shown yet.
        let stale_on_q2 = post(
            &fixture.state,
            &fixture.cookie,
            invocation(
                "world.play",
                "ghostlight.world_play.v0",
                served_version,
                json!({"text":"yes","answerToken":q1_token}),
                &uuid::Uuid::new_v4().to_string(),
            ),
        )
        .await;
        assert_eq!(
            stale_on_q2["receipt"]["state"], "denied",
            "an answer composed for Q1 must never land on Q2: {stale_on_q2}"
        );
        assert!(
            stale_on_q2["receipt"]["message"].as_str().unwrap_or_default().contains("stale"),
            "the refusal must name it stale: {stale_on_q2}"
        );

        let still_on_q2 = table.current_turn_view().await.unwrap();
        assert_eq!(
            still_on_q2.question.as_ref().map(|question| question.text.as_str()),
            Some("Are you sure?"),
            "the wrongly-refused answer must leave Q2 open, not resolve it"
        );
    }

    /// PA.f170: a token is single-use across distinct questions — reusing
    /// the token a *closed* turn's own answered question carried, replayed
    /// against a fresh question with a fresh token, is refused exactly like
    /// any other mismatch (the two-questions probe above already proves
    /// reuse within one still-open turn is refused; this proves reuse is
    /// never grandfathered in just because the token was once genuine).
    #[tokio::test]
    async fn a_token_from_an_already_answered_question_is_refused_against_the_next_one() {
        let (fixture, served_version) = play_world_with_two_scripted_questions().await;
        let table = fixture.state.play.clone().unwrap();

        let played = post(
            &fixture.state,
            &fixture.cookie,
            invocation(
                "world.play",
                "ghostlight.world_play.v0",
                served_version,
                json!({"text":"I stand at a crossroads."}),
                &uuid::Uuid::new_v4().to_string(),
            ),
        )
        .await;
        assert_eq!(played["receipt"]["state"], "accepted");

        tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                if let Some(view) = table.current_turn_view().await
                    && view.question.as_ref().is_some_and(|question| question.text == "Which way?")
                {
                    return;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("the first scripted ask_player call must open Q1");
        let q1_token = play_button_token(&fixture.state, &fixture.cookie)
            .await
            .expect("the served surface must carry Q1's own answer token");

        let answered_q1 = post(
            &fixture.state,
            &fixture.cookie,
            invocation(
                "world.play",
                "ghostlight.world_play.v0",
                served_version,
                json!({"text":"left","answerToken":q1_token.clone()}),
                &uuid::Uuid::new_v4().to_string(),
            ),
        )
        .await;
        assert_eq!(answered_q1["receipt"]["state"], "accepted");

        tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                if let Some(view) = table.current_turn_view().await
                    && view.question.as_ref().is_some_and(|question| question.text == "Are you sure?")
                {
                    return;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("answering Q1 must open Q2");
        let q2_token = play_button_token(&fixture.state, &fixture.cookie)
            .await
            .expect("the served surface must carry Q2's own answer token");
        assert_ne!(q1_token, q2_token, "Q1's own spent token must never be reissued for Q2");

        let replayed_q1_token = post(
            &fixture.state,
            &fixture.cookie,
            invocation(
                "world.play",
                "ghostlight.world_play.v0",
                served_version,
                json!({"text":"yes","answerToken":q1_token}),
                &uuid::Uuid::new_v4().to_string(),
            ),
        )
        .await;
        assert_eq!(
            replayed_q1_token["receipt"]["state"], "denied",
            "Q1's own already-spent token must never answer Q2: {replayed_q1_token}"
        );
    }

    /// PA.f150: `publish_projection` used to be called only at startup and
    /// from `eve_command`'s own Ok arm — which answers a `world.play`
    /// request before its own turn ever executes — so a play-row-only
    /// change (a fresh refusal, a newly open question, a closed turn's
    /// narration, none of which move `world.revision`) never republished at
    /// all, and a watching client's SSE subscription never woke for it.
    /// Drives a real play turn to a fault (the fixture's own inference organ
    /// is unreachable, so this always happens quickly) and asserts the
    /// `state.revisions` channel — what `/api/eve/events` relays — receives
    /// a fresh value for it, on the same door `eve_command`'s own commits use.
    #[tokio::test]
    async fn a_play_row_only_change_wakes_the_revisions_channel() {
        let fixture = fixture().await;
        let created = post(
            &fixture.state,
            &fixture.cookie,
            invocation(
                "world.create",
                "ghostlight.world_create.v4",
                0,
                json!({
                    "title":"Wake World",
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
        assert_eq!(created["receipt"]["state"], "accepted");
        let approved = post(
            &fixture.state,
            &fixture.cookie,
            invocation("world.approve", "ghostlight.world_approve.v0", 1, json!({}), &uuid::Uuid::new_v4().to_string()),
        )
        .await;
        assert_eq!(approved["receipt"]["state"], "accepted");
        let activated = post(
            &fixture.state,
            &fixture.cookie,
            invocation("world.activate", "ghostlight.world_activate.v0", 2, json!({}), &uuid::Uuid::new_v4().to_string()),
        )
        .await;
        assert_eq!(activated["receipt"]["state"], "accepted");

        // The fixture's own inference connector points at an unreachable
        // address (127.0.0.1:9), so every round faults immediately; only the
        // *retry backoff* is real time. Zeroed here so the turn actually
        // closes inside this test's own timeout, rather than spending it on
        // `backoff_delay`'s production delay between retries.
        fixture.state.play.clone().unwrap().set_retry_delay_base_ms(0);

        // Subscribed after every world-commit publish above has already
        // happened, so only the play turn's own publish (or a lagged
        // catch-up of it) can satisfy this recv.
        let mut revisions = fixture.state.revisions.subscribe();

        let played = post(
            &fixture.state,
            &fixture.cookie,
            invocation("world.play", "ghostlight.world_play.v0", 3, json!({"text":"I look around."}), &uuid::Uuid::new_v4().to_string()),
        )
        .await;
        assert_eq!(played["receipt"]["state"], "accepted");

        // PA.f163: `dispatch_world`'s own Ok arm publishes a revision for
        // *every* accepted command, including this one's immediate
        // `{"kind":"accepted"}` — before the turn's own round has executed
        // at all, since admission is synchronous and only the round loop is
        // spawned. Draining that message here is what keeps the recv below
        // from being satisfiable by it: before this fix, deleting the
        // spawned task's own `publish_projection` call left this test green
        // regardless, because this same immediate publish alone was already
        // enough to satisfy an undrained `recv()`.
        let immediate = tokio::time::timeout(Duration::from_secs(5), revisions.recv()).await;
        assert!(
            immediate.is_ok(),
            "world.play's own immediate accepted response must publish a revision"
        );

        // Wait for the turn's own commit to actually land. The fixture's
        // play table shares its unreachable test inference organ
        // (127.0.0.1:9), so the round faults immediately and the turn closes
        // on that fault (`close_with_fault`) rather than reaching
        // `AwaitingPlayer` — still a play-row-only commit, and the one this
        // test means to observe.
        let table = fixture.state.play.clone().unwrap();
        // A generous bound: `set_retry_delay_base_ms(0)` above removes the
        // backoff *sleep* between retries, but each of `ROUND_RETRY_BUDGET`
        // attempts still pays its own connector-level connect cost against
        // the unreachable address, so this can take several seconds even
        // with the delay zeroed.
        let closed = tokio::time::timeout(Duration::from_secs(30), async move {
            loop {
                if table
                    .current_turn_view()
                    .await
                    .is_some_and(|view| view.state == PlayTurnState::Closed)
                {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await;
        assert!(
            closed.is_ok(),
            "the turn must actually close (on a fault, given the unreachable test inference organ) \
             before this test can observe its own commit's wake"
        );

        // The turn's own commit — its close — must publish a *second*,
        // distinct revision: the spawned task's own `publish_projection`
        // call, the one PA.f150 added and PA.f163's mutation deletes.
        // Deleting it leaves exactly the one message already drained above,
        // so this recv would time out.
        let woke = tokio::time::timeout(Duration::from_secs(5), revisions.recv()).await;
        assert!(
            woke.is_ok(),
            "the play turn's own commit must publish a fresh revision to the SSE channel, not only \
             eve_command's own already-drained immediate accepted response"
        );
    }

    /// PA.f153/PA.f154: before this cut, `world.play`'s route spawned
    /// `PlayTable::run` in a task and answered `{"kind":"accepted"}`
    /// unconditionally, discarding whatever `run` returned into a log line
    /// nothing read — a replayed key, a stale/empty answer, a busy table, and
    /// a poisoned table all looked exactly like success to the caller.
    /// `admit` now runs synchronously in the route, so an admission refusal
    /// — here, an empty/whitespace-only opening with no turn currently open
    /// to continue — reaches the caller as `denied`, with the reason,
    /// instead.
    #[tokio::test]
    async fn an_admission_refusal_reaches_the_caller_as_denied_not_accepted() {
        let fixture = fixture().await;
        let created = post(
            &fixture.state,
            &fixture.cookie,
            invocation(
                "world.create",
                "ghostlight.world_create.v4",
                0,
                json!({
                    "title":"Admission World",
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
        assert_eq!(created["receipt"]["state"], "accepted");
        let approved = post(
            &fixture.state,
            &fixture.cookie,
            invocation("world.approve", "ghostlight.world_approve.v0", 1, json!({}), &uuid::Uuid::new_v4().to_string()),
        )
        .await;
        assert_eq!(approved["receipt"]["state"], "accepted");
        let activated = post(
            &fixture.state,
            &fixture.cookie,
            invocation("world.activate", "ghostlight.world_activate.v0", 2, json!({}), &uuid::Uuid::new_v4().to_string()),
        )
        .await;
        assert_eq!(activated["receipt"]["state"], "accepted");

        let opened_empty = post(
            &fixture.state,
            &fixture.cookie,
            invocation("world.play", "ghostlight.world_play.v0", 3, json!({"text":"   "}), &uuid::Uuid::new_v4().to_string()),
        )
        .await;
        assert_eq!(
            opened_empty["receipt"]["state"], "denied",
            "an empty/whitespace-only opening with no turn open must be denied, not accepted: {opened_empty}"
        );
        assert!(
            opened_empty["receipt"]["message"].as_str().unwrap().contains("empty"),
            "the denial must carry the admission refusal's own reason: {opened_empty}"
        );

        // No turn was ever admitted, so nothing was ever spawned either.
        let table = fixture.state.play.clone().unwrap();
        assert!(
            table.current_turn_view().await.is_none(),
            "a denied admission must never open a turn row"
        );
    }

    /// PA.f152: `world.play` is owner-gated like `world.advance_time` and
    /// `world.seed` — both the surface (no card, no controls, no descriptor
    /// for anyone else) and the route itself, since the surface is only a
    /// hint a hostile client can ignore.
    #[tokio::test]
    async fn world_play_is_owner_gated_on_the_surface_and_the_route() {
        let fixture = fixture().await;
        let created = post(
            &fixture.state,
            &fixture.cookie,
            invocation(
                "world.create",
                "ghostlight.world_create.v4",
                0,
                json!({
                    "title":"Owner-Gated World",
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
        assert_eq!(created["receipt"]["state"], "accepted");
        let approved = post(
            &fixture.state,
            &fixture.cookie,
            invocation("world.approve", "ghostlight.world_approve.v0", 1, json!({}), &uuid::Uuid::new_v4().to_string()),
        )
        .await;
        assert_eq!(approved["receipt"]["state"], "accepted");
        let activated = post(
            &fixture.state,
            &fixture.cookie,
            invocation("world.activate", "ghostlight.world_activate.v0", 2, json!({}), &uuid::Uuid::new_v4().to_string()),
        )
        .await;
        assert_eq!(activated["receipt"]["state"], "accepted");

        // A stranger's own authenticated session — a different account, so a
        // different `PrincipalId`, never the world's own owner.
        let mut sessions = fixture.state.sessions.lock().await;
        let stranger_cookie = sessions
            .create_session(heimdall::VerifiedSessionAdmission::fixture(
                "a-stranger-account",
                "heimdall-session-stranger",
                2,
                Utc::now() + chrono::Duration::hours(1),
                Utc::now() + chrono::Duration::days(1),
                "fixture-refresh-stranger",
            ))
            .unwrap();
        drop(sessions);

        let stranger_surface = get(&fixture.state, &stranger_cookie, "/api/eve/surfaces/ghostlight.play").await;
        let encoded = serde_json::to_string(&stranger_surface).unwrap();
        assert!(
            !encoded.contains("world.play"),
            "a non-owner's own surface must carry no play card, control, or command descriptor: {encoded}"
        );

        let played = post(
            &fixture.state,
            &stranger_cookie,
            invocation("world.play", "ghostlight.world_play.v0", 3, json!({"text":"I look around."}), &uuid::Uuid::new_v4().to_string()),
        )
        .await;
        assert_eq!(
            played["receipt"]["state"], "denied",
            "the route itself must refuse a non-owner's world.play, not only hide the affordance: {played}"
        );

        let table = fixture.state.play.clone().unwrap();
        assert!(
            table.current_turn_view().await.is_none(),
            "a non-owner's own denied world.play must never open a turn row"
        );
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
        assert_eq!(created["receipt"]["state"], "accepted");

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
        assert_eq!(played["receipt"]["state"], "denied");
    }

    /// PA.f147, reported rather than fixed (see Hands' own report):
    /// `world.play`'s own arm takes `idempotency_key.unwrap_or_default()`
    /// with no local guard, which reads like a hole — a caller that skipped
    /// `eve::validate_invocation` (the only door that normally requires a
    /// non-empty key) would land on key `""`, and `PlayTable::run`'s own
    /// `applied_keys`/`KeyLedger` would record and later replay it as a
    /// no-op. A local, explicit guard was tried and reverted: mutation-tested
    /// against a fresh key-less call through `dispatch_world` (the one
    /// caller that skips HTTP validation), it made no difference, because
    /// `execute_world`'s own `command_id = CommandId::parse_uuid(...)?`,
    /// computed unconditionally before *every* operation's own branch
    /// (`world.play`'s included), already refuses a missing or non-uuid key
    /// first. This test pins that guard's own reach over `world.play`
    /// specifically, since nothing else in this suite calls `dispatch_world`
    /// with a key-less invocation. Mutation: `unwrap_or("")` →
    /// `unwrap_or("00000000-0000-0000-0000-000000000000")` on
    /// `command_id`'s own parse.
    #[tokio::test]
    async fn world_play_without_an_idempotency_key_is_refused_not_defaulted() {
        let fixture = fixture().await;
        let created = post(
            &fixture.state,
            &fixture.cookie,
            invocation(
                "world.create",
                "ghostlight.world_create.v4",
                0,
                json!({
                    "title":"No Key World",
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
        assert_eq!(created["receipt"]["state"], "accepted");

        let mut keyless: EveCommandInvocation = serde_json::from_value(invocation(
            "world.play",
            "ghostlight.world_play.v0",
            1,
            json!({"text":"I look around."}),
            &uuid::Uuid::new_v4().to_string(),
        ))
        .unwrap();
        keyless.operation.idempotency_key = None;

        let principal =
            VerifiedPrincipalEvidence::new("operator-account", Utc::now() + chrono::Duration::hours(1));
        let response = dispatch_world(&fixture.state, &principal, keyless).await;
        let bytes = axum::body::to_bytes(response.into_body(), 1024 * 1024)
            .await
            .unwrap();
        let played: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(played["receipt"]["state"], "denied");
        assert!(
            fixture.state.play.as_ref().unwrap().current_turn_view().await.is_none(),
            "a missing key must never reach table.run at all, not just fail inside it"
        );
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
        assert_eq!(first["receipt"]["state"], "accepted");
        assert_eq!(retry["receipt"]["state"], "accepted");
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
            assert_eq!(result["receipt"]["state"], "denied", "{operation}");
            assert!(
                result["receipt"]["message"]
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
        assert_eq!(result["receipt"]["state"], "denied");
        assert!(
            result["receipt"]["message"]
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
        assert_eq!(stale["receipt"]["state"], "denied");
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
        assert_eq!(unweighted["receipt"]["state"], "denied");
        // Refused as a payload, before genesis: a defaulted empty set would
        // also be denied, by the resolver, and must not pass for this.
        let message = unweighted["receipt"]["message"].as_str().unwrap_or_default();
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
        assert_eq!(partial["receipt"]["state"], "denied");
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
        assert_eq!(result["receipt"]["state"], "denied");
        let message = result["receipt"]["message"].as_str().unwrap_or_default();
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
        assert_eq!(result["receipt"]["state"], "denied");
        let message = result["receipt"]["message"].as_str().unwrap_or_default();
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
        let mut surfaces = vec![eve::authenticated_surface(&owner, None, None).unwrap()];
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
            surfaces.push(eve::authenticated_surface(account, Some(&draft), None).unwrap());
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
        let play_view = current_play_view(&fixture.state).await;
        for account in [owner.as_str(), stranger] {
            surfaces.push(
                eve::authenticated_surface(account, Some(&active), play_view.as_ref()).unwrap(),
            );
        }
        surfaces.push(eve::anonymous_surface());

        let mut seen = 0usize;
        let mut saw_world_play = false;
        for surface in &surfaces {
            let mut buttons = Vec::new();
            surface_buttons(&surface["surface"]["root"], &mut buttons);
            assert!(!buttons.is_empty());
            for command in &buttons {
                assert!(
                    eve::operation_schema(command).is_some(),
                    "the panel emits {command}, which Ghostlight does not advertise"
                );
                saw_world_play |= command == "world.play";
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
        // Cut 9: the play card's own button is now part of this walk — it
        // was not, before Cut 9 published its descriptor (PA.f146).
        assert!(saw_world_play, "the panel must emit world.play in Active");
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
        assert_eq!(denied["receipt"]["state"], "denied");
        assert!(
            denied["receipt"]["message"].as_str().unwrap().contains("owner"),
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
        assert_eq!(refused["receipt"]["state"], "accepted");
        assert_eq!(refused["receipt"]["outcome"], "not_draft");
        assert_eq!(
            fixture.state.world.snapshot().await.unwrap().revision,
            active.revision,
            "a refused seed still moved the world"
        );
    }

    /// The seed refusal must be a projection of the world and the player's
    /// own act (invariant 8), never a server configuration name: an
    /// unconfigured vault root is this daemon's own operational problem, not
    /// a fact about the world worth teaching the player. Names the actual
    /// former leak by string, so a regression is caught by name and not only
    /// by a vague "does not equal the old message" diff.
    ///
    /// Mutation: revert `seed_once`'s vault-root refusal back to
    /// `format!("{SEED_VAULT_ROOT_ENVIRONMENT} is not configured")` — this
    /// test's `assert!(!... .contains("GHOSTLIGHT_SEED_VAULT_ROOT"))` would
    /// then fail.
    #[tokio::test]
    async fn the_unconfigured_seed_vault_refusal_does_not_name_the_environment_variable() {
        let fixture = fixture().await;
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

        unsafe { std::env::remove_var(SEED_VAULT_ROOT_ENVIRONMENT) };
        let denied = post(
            &fixture.state,
            &fixture.cookie,
            invocation(
                "world.seed",
                "ghostlight.world_seed.v1",
                draft.revision + 1,
                json!({}),
                &uuid::Uuid::new_v4().to_string(),
            ),
        )
        .await;
        assert_eq!(denied["receipt"]["state"], "denied");
        let message = denied["receipt"]["message"].as_str().unwrap_or_default();
        assert!(
            !message.contains(SEED_VAULT_ROOT_ENVIRONMENT),
            "the player-facing refusal must not name the server's own environment variable: {denied}"
        );
        assert_eq!(
            message, "invalid command payload: seeding is not available on this world",
            "{denied}"
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
        let surface = eve::authenticated_surface(&owner, Some(&draft), None).unwrap();
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
