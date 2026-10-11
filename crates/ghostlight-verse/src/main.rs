//! The verse runner's operator command line.
//!
//! `genesis` creates the canonical Aetheria verse world in a state root, and
//! `run` advances it a given number of ticks under a token cap, printing per
//! tick what each faction did and what it cost. A run starts only when this
//! command is invoked, and ends when it has done what it was asked.
//!
//! The state root holds `world.cc`, `controller-work.cc`, `runs.cc` and
//! `calls.cc`. The provider key is read from a file at run time and appears
//! nowhere else.

use anyhow::{Context, Result, anyhow, bail};
use chrono::{Duration, Utc};
use ghostlight::{
    CommandBody, CommandId, ControllerModels, ControllerOpenError, ControllerRunner, DEFAULT_HOSTED_MODEL_PREFIX,
    DEFAULT_LOCAL_MODEL_PREFIX, CallRecordStore, CoverBudget, CreateVerseWorldIntent,
    HostedBinding, Lens, LensWeights, LocalBinding, PrincipalCommandIntent, RecordingInferencePort,
    RunStart, RunStatus, TickMinutes, VerifiedPrincipalEvidence, VerseRunConfig, VerseRunStore,
    WorldMailbox, call_run_id, id_text, open_controller_work, open_inference,
    read_aetheria_catalog, run_manual,
};
use std::collections::BTreeMap;
use std::net::SocketAddr;
use std::ops::ControlFlow;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

/// What a manual run spends per tick on cognition cells. The operator's lever
/// on cost is the token cap; the partition always covers every faction.
const COVER: CoverBudget = CoverBudget {
    cells: 8,
    constituent_cap: 8,
    urgency_slots: 2,
};

const RUNTIME_ID: &str = "ghostlight-verse";

#[tokio::main]
async fn main() -> ExitCode {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    match command(arguments).await {
        Ok(code) => code,
        Err(error) => {
            eprintln!("ghostlight-verse: {error:#}");
            ExitCode::FAILURE
        }
    }
}

async fn command(arguments: Vec<String>) -> Result<ExitCode> {
    let Some((name, rest)) = arguments.split_first() else {
        bail!("usage: ghostlight-verse genesis|run [flags]");
    };
    match name.as_str() {
        "genesis" => genesis(Flags::parse(rest, &GENESIS_FLAGS)?).await,
        "run" => run(Flags::parse(rest, &RUN_FLAGS)?).await,
        _ => bail!("unknown command, expected genesis or run"),
    }
}

const GENESIS_FLAGS: [&str; 4] = ["catalog", "aetheria-commit", "state", "owner-hash"];
const RUN_FLAGS: [&str; 7] = [
    "state",
    "ticks",
    "token-cap",
    "tick-span-minutes",
    "model",
    "base-url",
    "key-file",
];
/// Flags a command may omit.
const OPTIONAL: [&str; 1] = ["key-file"];

/// `--name value` pairs, each named once and from the command's own list.
struct Flags(BTreeMap<&'static str, String>);

impl Flags {
    fn parse(arguments: &[String], allowed: &[&'static str]) -> Result<Self> {
        let mut seen = BTreeMap::new();
        let mut rest = arguments.iter();
        while let Some(flag) = rest.next() {
            let Some(name) = flag.strip_prefix("--") else {
                bail!("expected a --flag, found a bare argument");
            };
            let Some(&known) = allowed.iter().find(|candidate| **candidate == name) else {
                bail!("a flag this command does not take was given");
            };
            let Some(value) = rest.next() else {
                bail!("--{known} needs a value");
            };
            if seen.insert(known, value.clone()).is_some() {
                bail!("--{known} was given twice");
            }
        }
        for &required in allowed {
            if !OPTIONAL.contains(&required) && !seen.contains_key(required) {
                bail!("--{required} is required");
            }
        }
        Ok(Self(seen))
    }

    fn text(&self, name: &str) -> &str {
        self.0.get(name).map_or("", String::as_str)
    }

    fn path(&self, name: &str) -> PathBuf {
        PathBuf::from(self.text(name))
    }

    fn number<T: std::str::FromStr + PartialOrd + Default>(&self, name: &str) -> Result<T> {
        match self.text(name).parse::<T>() {
            Ok(value) if value > T::default() => Ok(value),
            _ => bail!("--{name} is not a positive integer"),
        }
    }
}

async fn genesis(flags: Flags) -> Result<ExitCode> {
    let state = flags.path("state");
    std::fs::create_dir_all(&state).context("the state directory cannot be created")?;
    let catalog = read_aetheria_catalog(flags.path("catalog"))
        .context("the Aetheria catalog cannot be read")?;
    let factions = catalog.factions.len();
    let principal = VerifiedPrincipalEvidence::new(flags.text("owner-hash"), Utc::now() + Duration::hours(1));

    let (mailbox, task) = WorldMailbox::open(state.join("world.cc"))
        .map_err(store_fault("the world file in --state cannot be opened"))?;
    let receipt = mailbox
        .create_verse(
            CreateVerseWorldIntent {
                id: CommandId::new(),
                title: "The Aetheria Verse".into(),
                brief: "The Aetheria verse, simulated from its catalog.".into(),
                catalog,
                aetheria_commit: flags.text("aetheria-commit").into(),
                lens_weights: LensWeights::new(Lens::ALL.into_iter().map(|lens| (lens, 1)).collect()),
                targets: BTreeMap::new(),
            },
            &principal,
        )
        .await
        .context("the verse world cannot be created")?;
    // A created world is a draft: the owner approves and activates it, so the
    // factions hold opportunities.
    for body in [CommandBody::ApproveDraft, CommandBody::ActivateWorld] {
        let revision = mailbox.snapshot().await.context("the world cannot be read")?.revision;
        mailbox
            .submit_principal(
                PrincipalCommandIntent {
                    id: CommandId::new(),
                    world_id: receipt.world_id,
                    expected_revision: revision,
                    body,
                },
                &principal,
            )
            .await
            .context("the world cannot be activated")?;
    }
    drop(mailbox);
    task.await.context("the world file did not close cleanly")?;
    println!("world {}", id_text(receipt.world_id));
    println!("factions {factions}");
    println!("phase Active");
    Ok(ExitCode::SUCCESS)
}

async fn run(flags: Flags) -> Result<ExitCode> {
    let state = flags.path("state");
    let ticks: u32 = flags.number("ticks")?;
    let cap: u64 = flags.number("token-cap")?;
    let span: u32 = flags.number("tick-span-minutes")?;
    let tick_span = TickMinutes::new(span).context("--tick-span-minutes is outside the clock's span")?;
    if !state.join("world.cc").is_file() {
        bail!("the state directory holds no world; run genesis first");
    }

    let model = flags.text("model").to_owned();
    let models = ControllerModels {
        projector: model.clone(),
        persona: model.clone(),
        interpreter: model.clone(),
        operational_agent: model.clone(),
        elaborator: model.clone(),
    };
    let inner = open_lane(&flags, &model, &models)?;

    let (mailbox, task) = WorldMailbox::open(state.join("world.cc"))
        .map_err(store_fault("the world file in --state cannot be opened"))?;
    let world = mailbox.snapshot().await.context("the world cannot be read")?;
    let calls = Arc::new(CallRecordStore::open(state.join("calls.cc")).map_err(store_fault("the call records in --state cannot be opened"))?);
    let runs = VerseRunStore::open(state.join("runs.cc")).map_err(store_fault("the run records in --state cannot be opened"))?;
    let work = open_controller_work(state.join("controller-work.cc"))
        .map_err(store_fault("the controller work store in --state cannot be opened"))?;
    let config = VerseRunConfig {
        ticks,
        tick_span,
        cover: COVER,
    };
    let claim = runs
        .claim(&world, cap, &config)
        .context("the run cannot be claimed")?;
    let sequence = claim.sequence();
    // A start that fails after the claim ends the claimed run, so it binds
    // nothing: the next claim takes the next number.
    let started = RecordingInferencePort::open(
        inner,
        calls,
        call_run_id(sequence),
        cap,
        &RunStart::of(&world),
    )
    .context("the run's call records cannot be opened")
    .and_then(|port| {
        let port = Arc::new(port);
        ControllerRunner::open(mailbox.clone(), port.clone(), work, models)
            .map(|runner| (port, runner))
            .context("the controller organ cannot open")
    });
    let (port, runner) = match started {
        Ok(started) => started,
        Err(error) => {
            return Err(match runs.abandon(claim, "start_refused") {
                Ok(()) => error,
                Err(_) => error.context("the refused run's record could not be ended"),
            });
        }
    };

    println!(
        "run {sequence}: {ticks} ticks of {span} minutes, cap {cap} tokens, model {model}"
    );
    let interrupted = Arc::new(AtomicBool::new(false));
    {
        let interrupted = interrupted.clone();
        tokio::spawn(async move {
            if tokio::signal::ctrl_c().await.is_ok() {
                interrupted.store(true, Ordering::SeqCst);
            }
        });
    }
    let outcome = run_manual(
        &mailbox,
        &runner,
        &port,
        &runs,
        claim,
        &config,
        |report| {
            print!("{report}");
            if interrupted.load(Ordering::SeqCst) {
                ControlFlow::Break(())
            } else {
                ControlFlow::Continue(())
            }
        },
    )
    .await
    .context("the run could not be recorded")?;
    println!(
        "status: {}; {} of {} ticks closed",
        outcome.record.status,
        outcome.record.ticks_closed(),
        outcome.record.ticks_requested
    );
    drop(runner);
    drop(port);
    drop(mailbox);
    task.await.context("the world file did not close cleanly")?;
    Ok(match outcome.record.status {
        RunStatus::Failed { .. } => ExitCode::FAILURE,
        _ => ExitCode::SUCCESS,
    })
}

/// The inference lane the model's prefix names. A hosted model needs an https
/// origin and a key file; a local one reaches a loopback server and needs
/// neither, so a key file beside it is refused.
fn open_lane(
    flags: &Flags,
    model: &str,
    models: &ControllerModels,
) -> Result<Arc<dyn ghostlight::InferencePort>> {
    let (mut hosted, mut local) = (None, None);
    if model.starts_with(DEFAULT_HOSTED_MODEL_PREFIX) {
        let Some(key) = flags.0.get("key-file") else {
            bail!("a hosted model needs --key-file");
        };
        hosted = Some(HostedBinding {
            base_url: flags.text("base-url").to_owned(),
            key_path: Path::new(key).to_owned(),
            model_prefix: DEFAULT_HOSTED_MODEL_PREFIX.into(),
            caller_runtime_id: RUNTIME_ID.into(),
        });
    } else if model.starts_with(DEFAULT_LOCAL_MODEL_PREFIX) {
        if flags.0.contains_key("key-file") {
            bail!("a local model takes no --key-file");
        }
        let endpoint: SocketAddr = flags
            .text("base-url")
            .strip_prefix("http://")
            .and_then(|address| address.trim_end_matches('/').parse().ok())
            .context("--base-url for a local model is not http://<address>:<port>")?;
        local = Some(LocalBinding {
            endpoint,
            model_prefix: DEFAULT_LOCAL_MODEL_PREFIX.into(),
            caller_runtime_id: RUNTIME_ID.into(),
        });
    } else {
        bail!("--model names no inference lane (expected a hosted/ or local/ prefix)");
    }
    open_inference(None, None, local, hosted, &models.each())
        .map_err(|error| anyhow!(lane_fault(&error)))
}

/// A store that cannot open says which one and that it is under `--state`,
/// never the path or the store's own text, which carries the path.
fn store_fault<E>(message: &'static str) -> impl FnOnce(E) -> anyhow::Error {
    move |_| anyhow!(message)
}

/// What a lane that cannot open says: the flag or the kind of fault, never a
/// value the operator passed.
fn lane_fault(error: &ControllerOpenError) -> &'static str {
    match error {
        ControllerOpenError::InvalidModels => "--model is not an exact nonempty identifier",
        ControllerOpenError::Connector(_) => "the connector lane cannot open",
        ControllerOpenError::WorkStore(_) => "the controller work journal cannot open",
        ControllerOpenError::UnroutableModel { .. } => "--model is claimed by no inference lane",
        ControllerOpenError::SdkSidecarMissing { .. } => "the SDK sidecar entry is not a file",
        ControllerOpenError::LocalEndpointNotLoopback { .. } => {
            "--base-url for a local model must be a loopback address"
        }
        ControllerOpenError::HostedEndpointNotHttps => {
            "--base-url for a hosted model must be an https URL"
        }
        ControllerOpenError::HostedKeyUnreadable => {
            "--key-file is unreadable, empty, or not one run of visible ASCII"
        }
        ControllerOpenError::SharedModelPrefix { .. } => "two inference lanes claim one model prefix",
        ControllerOpenError::EmptyModelPrefix { .. } => "an inference lane's model prefix is empty",
    }
}
