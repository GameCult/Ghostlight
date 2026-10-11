use super::*;
use crate::aetheria::read_aetheria_catalog;
use crate::controllers::{
    ControllerModels, ControllerWorkStore, InferenceEvent, InferenceFault, InferenceOutput,
    InferencePort, InferencePurpose, InferenceRequest, PreparedInference, TokenUsage,
    open_controller_work,
};
use crate::cover::derive_cover;
use crate::tests::stock_weights;
use crate::verse::{
    CallRecordStore, CreateVerseWorldIntent, RecordingInferencePort, ReplayInferencePort,
    RunStart,
};
use crate::{
    BoundRole, CommandBody, ControllerMode, FictionalMinutes, KernelError,
    PrincipalCommandIntent, RenderPath, VerifiedPrincipalEvidence, id_text,
};
use async_trait::async_trait;
use chrono::Duration;
use serde_json::json;
use std::collections::{BTreeMap, VecDeque};
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::task::JoinHandle;

const COMMIT: &str = "92b82e9aa5bb422c568c4bfc85a8f641426a41fc";
const FIXTURE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/aetheria-verse-catalog.cc"
);
const SPAN: u32 = 60;

/// Creates the verse world at `dir/world.cc` from the fixture catalog and
/// activates it, then lets go of the file so a copy of it is a restored world.
async fn create_world(dir: &Path) {
    let (mailbox, task) = WorldMailbox::open(dir.join("world.cc")).unwrap();
    let principal =
        VerifiedPrincipalEvidence::fixture("verse-owner", Utc::now() + Duration::hours(1));
    let receipt = mailbox
        .create_verse(
            CreateVerseWorldIntent {
                id: CommandId::new(),
                title: "The Verse".into(),
                brief: "Aetheria.".into(),
                catalog: read_aetheria_catalog(FIXTURE).unwrap(),
                aetheria_commit: COMMIT.into(),
                lens_weights: stock_weights(),
                targets: BTreeMap::new(),
            },
            &principal,
        )
        .await
        .unwrap();
    for body in [CommandBody::ApproveDraft, CommandBody::ActivateWorld] {
        let revision = mailbox.snapshot().await.unwrap().revision;
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
            .unwrap();
    }
    drop(mailbox);
    task.await.unwrap();
}

/// A scripted provider. `reply` sees the serialized request and answers with
/// the events a model would. It reports usage as a fixed share of the bound the
/// recording port holds the call to, so the charge is a function of the
/// request.
struct ScriptedPort {
    reply: Box<dyn Fn(&str) -> Vec<InferenceEvent> + Send + Sync>,
    faults: std::sync::Mutex<VecDeque<InferenceFault>>,
    bounds: std::sync::Mutex<Vec<u64>>,
    calls: AtomicUsize,
}

impl ScriptedPort {
    fn new(reply: impl Fn(&str) -> Vec<InferenceEvent> + Send + Sync + 'static) -> Arc<Self> {
        Arc::new(Self {
            reply: Box::new(reply),
            faults: std::sync::Mutex::new(VecDeque::new()),
            bounds: std::sync::Mutex::new(Vec::new()),
            calls: AtomicUsize::new(0),
        })
    }

    /// Every faction speaks its own line.
    fn speaking() -> Arc<Self> {
        Self::new(|request| {
            let name = who(request);
            vec![speak_call(&format!("{name} holds the line."))]
        })
    }

    fn fail_next(&self, fault: InferenceFault) {
        self.faults.lock().unwrap().push_back(fault);
    }

    fn calls(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }

    fn bounds(&self) -> Vec<u64> {
        self.bounds.lock().unwrap().clone()
    }
}

/// Which faction a request is for: the prompt names its decision owner.
fn who(request: &str) -> String {
    for name in ["Adrasteia", "Brannoch"] {
        if request.contains(&format!("Decision owner:\\n{name}\\n")) {
            return name.to_owned();
        }
    }
    panic!("a request names no faction");
}

fn speak_call(text: &str) -> InferenceEvent {
    InferenceEvent::ToolCall {
        call_id: "act".into(),
        name: "speak".into(),
        arguments: json!({ "text": text }).to_string(),
    }
}

#[async_trait]
impl InferencePort for ScriptedPort {
    fn prepare(&self, request: InferenceRequest) -> Result<PreparedInference, InferenceFault> {
        PreparedInference::prepare("verse-run-test", 4_102_444_800_000, request)
    }

    async fn infer(&self, request: PreparedInference) -> Result<InferenceOutput, InferenceFault> {
        let sequence = self.calls.fetch_add(1, Ordering::SeqCst);
        let wire = &request.invocation.request;
        let bound = serde_json::to_vec(wire).unwrap().len() as u64
            + u64::from(wire.max_output_tokens.unwrap());
        self.bounds.lock().unwrap().push(bound);
        if let Some(fault) = self.faults.lock().unwrap().pop_front() {
            return Err(fault);
        }
        let text = serde_json::to_string(wire).unwrap();
        Ok(
            InferenceOutput::new((self.reply)(&text), format!("sha256:scripted-{sequence}"))
                .with_usage(TokenUsage {
                    prompt: bound / 2,
                    completion: bound / 4,
                    cached_prompt: None,
                }),
        )
    }
}

fn models() -> ControllerModels {
    ControllerModels {
        projector: "scripted".into(),
        persona: "scripted".into(),
        interpreter: "scripted".into(),
        operational_agent: "scripted".into(),
        elaborator: "scripted".into(),
    }
}

fn two_cells() -> CoverBudget {
    CoverBudget {
        cells: 2,
        constituent_cap: 8,
        urgency_slots: 0,
    }
}

fn config(ticks: u32) -> VerseRunConfig {
    VerseRunConfig {
        ticks,
        tick_span: TickMinutes::new(SPAN).unwrap(),
        cover: two_cells(),
    }
}

/// An opened verse state root: the world, its call records, its run records
/// and its controller work, as the binary holds them.
struct Harness {
    mailbox: WorldMailbox,
    _task: JoinHandle<()>,
    calls: Arc<CallRecordStore>,
    runs: VerseRunStore,
    work: Arc<dyn ControllerWorkStore>,
}

impl Harness {
    async fn open(dir: &Path) -> Self {
        let (mailbox, task) = WorldMailbox::open(dir.join("world.cc")).unwrap();
        Self {
            mailbox,
            _task: task,
            calls: Arc::new(CallRecordStore::open(dir.join("calls.cc")).unwrap()),
            runs: VerseRunStore::open(dir.join("runs.cc")).unwrap(),
            work: open_controller_work(dir.join("controller-work.cc")).unwrap(),
        }
    }

    async fn fresh() -> (tempfile::TempDir, Self) {
        let dir = tempfile::tempdir().unwrap();
        create_world(dir.path()).await;
        let harness = Self::open(dir.path()).await;
        (dir, harness)
    }

    /// One run over `inner`, claimed and recorded under the next sequence.
    async fn run(
        &self,
        inner: Arc<dyn InferencePort>,
        cap: u64,
        ticks: u32,
        observe: impl FnMut(&TickReport) -> ControlFlow<()>,
    ) -> RunOutcome {
        self.run_under(inner, cap, config(ticks), observe).await
    }

    /// One run under the given config, claimed and recorded under the next
    /// sequence.
    async fn run_under(
        &self,
        inner: Arc<dyn InferencePort>,
        cap: u64,
        config: VerseRunConfig,
        observe: impl FnMut(&TickReport) -> ControlFlow<()>,
    ) -> RunOutcome {
        let world = self.mailbox.snapshot().await.unwrap();
        let claim = self.runs.claim(&world, cap, &config).unwrap();
        let sequence = claim.sequence();
        let port = Arc::new(
            RecordingInferencePort::open(
                inner,
                self.calls.clone(),
                call_run_id(sequence),
                cap,
                &RunStart::of(&world),
            )
            .unwrap(),
        );
        let runner =
            ControllerRunner::open(self.mailbox.clone(), port.clone(), self.work.clone(), models())
                .unwrap();
        run_manual(
            &self.mailbox,
            &runner,
            &port,
            &self.runs,
            claim,
            &config,
            observe,
        )
        .await
        .unwrap()
    }

    async fn run_quietly(&self, inner: Arc<dyn InferencePort>, cap: u64, ticks: u32) -> RunOutcome {
        self.run(inner, cap, ticks, |_| ControlFlow::Continue(()))
            .await
    }

    async fn run_quietly_under(
        &self,
        inner: Arc<dyn InferencePort>,
        config: VerseRunConfig,
    ) -> RunOutcome {
        self.run_under(inner, u64::MAX, config, |_| ControlFlow::Continue(()))
            .await
    }

    async fn now(&self) -> u64 {
        self.mailbox.snapshot().await.unwrap().now.0
    }

    async fn acts(&self) -> usize {
        self.mailbox.operator_log().await.unwrap().len()
    }

    async fn subject(&self, label: &str) -> crate::SubjectId {
        self.mailbox
            .snapshot()
            .await
            .unwrap()
            .subjects
            .iter()
            .find(|subject| subject.label == label)
            .unwrap()
            .id
    }
}

#[tokio::test]
async fn scripted_run_commits_and_checkpoints_every_tick() {
    let (_dir, verse) = Harness::fresh().await;
    let world = verse.mailbox.snapshot().await.unwrap();
    let port = ScriptedPort::speaking();
    let mut reports = Vec::new();
    let outcome = verse
        .run(port.clone(), 10_000_000, 3, |report| {
            reports.push(report.clone());
            ControlFlow::Continue(())
        })
        .await;

    // One checkpoint per tick, tick indexes 0 to 2, the clock moved three spans.
    assert_eq!(outcome.record.status, RunStatus::Completed);
    let ticks: Vec<u64> = outcome.record.checkpoints.iter().map(|c| c.tick).collect();
    assert_eq!(ticks, vec![0, 1, 2]);
    assert_eq!(outcome.record.ticks_closed(), 3);
    assert_eq!(verse.now().await, 3 * u64::from(SPAN));
    // Both factions acted every tick, the second on a scope the first moved.
    assert_eq!(verse.acts().await, 6);
    assert_eq!(port.calls(), 6);

    // Every committed command id is the one the tick document's frozen cells
    // derive, in cell order, and the document is closed by this run's clock.
    for checkpoint in &outcome.record.checkpoints {
        let tick = verse
            .runs
            .tick(world.world_id, checkpoint.tick)
            .unwrap()
            .unwrap();
        assert_eq!(tick.closed, Some(TickClose::Clock { run: 1 }));
        assert_eq!(tick.opened_by_run, 1);
        assert_eq!(tick.span_minutes, SPAN);
        let expected: Vec<CommandId> = tick
            .cells
            .iter()
            .map(|cell| {
                assert!(!cell.group, "two factions under two cells are singletons");
                CommandId::for_cell_constituent(
                    world.world_id,
                    cell.id,
                    cell.members[0],
                    TickIndex(checkpoint.tick),
                )
            })
            .collect();
        assert_eq!(expected.len(), 2);
        assert_eq!(checkpoint.committed, expected);
        assert_eq!(checkpoint.cells, 2);
        assert!(checkpoint.closed);
        assert!(checkpoint.refusals.is_empty());
    }

    // The record in the store is the record the run returned, and it says what
    // the run was asked.
    assert_eq!(
        verse.runs.runs(world.world_id).unwrap(),
        vec![outcome.record.clone()]
    );
    assert_eq!(outcome.record.world_id, world.world_id);
    assert_eq!(outcome.record.sequence, 1);
    assert_eq!(outcome.record.token_cap, 10_000_000);
    assert_eq!(outcome.record.ticks_requested, 3);
    assert_eq!(outcome.record.tick_span_minutes, SPAN);
    assert_eq!(outcome.record.start_revision, world.revision);
    assert_eq!(
        outcome.record.cover_budget,
        RecordedCoverBudget {
            cells: 2,
            constituent_cap: 8,
            urgency_slots: 0
        }
    );

    // A tick's tokens are what the call store charged for its two calls.
    let records = verse.calls.records(&call_run_id(1)).unwrap();
    assert_eq!(records.len(), 6);
    for (tick, checkpoint) in outcome.record.checkpoints.iter().enumerate() {
        let charged = records[2 * tick].charged() + records[2 * tick + 1].charged();
        assert!(charged > 0);
        assert_eq!(checkpoint.tokens, charged);
    }
    let total: u64 = outcome.record.checkpoints.iter().map(|c| c.tokens).sum();
    assert_eq!(total, verse.calls.spent(&call_run_id(1)).unwrap());

    // The operator sees each tick as it ends: its acts, and the spend so far.
    assert_eq!(reports.len(), 3);
    let mut spent = 0;
    for (tick, report) in reports.iter().enumerate() {
        spent += report.checkpoint.tokens;
        assert!(report.checkpoint.closed);
        assert_eq!(report.acts.len(), 2);
        assert_eq!(report.spent, spent);
        assert_eq!(report.remaining, 10_000_000 - spent);
        assert_eq!(report.checkpoint, outcome.record.checkpoints[tick]);
        for act in &report.acts {
            assert_eq!(act.verb, "speak");
            assert_eq!(
                act.speech.as_ref().unwrap().as_str(),
                format!("{} holds the line.", act.speaker_label)
            );
        }
    }
    // A tick reports only its own acts.
    let revisions: Vec<u64> = reports
        .iter()
        .flat_map(|report| report.acts.iter().map(|act| act.revision))
        .collect();
    assert_eq!(revisions.len(), 6);
    assert!(revisions.windows(2).all(|pair| pair[0] < pair[1]));
}

#[tokio::test]
async fn budget_exhausted_stops_before_the_clock_and_resumes() {
    // What the first call is held to, measured on a world of its own.
    let (_probe_dir, probe) = Harness::fresh().await;
    let measuring = ScriptedPort::speaking();
    probe.run_quietly(measuring.clone(), u64::MAX, 1).await;
    let first_bound = measuring.bounds()[0];

    let (_dir, verse) = Harness::fresh().await;
    let world = verse.mailbox.snapshot().await.unwrap();
    // Room for the first call, and not for a second after the first's charge.
    let first = ScriptedPort::speaking();
    let mut reports = Vec::new();
    let ended = verse
        .run(first.clone(), first_bound + 64, 2, |report| {
            reports.push(report.clone());
            ControlFlow::Continue(())
        })
        .await;
    assert_eq!(ended.record.status, RunStatus::BudgetExhausted);
    // The tick it ended inside is recorded, unclosed, with the act it committed.
    let [open] = ended.record.checkpoints.as_slice() else {
        panic!("the open tick is the run's one checkpoint")
    };
    assert!(!open.closed);
    assert_eq!(open.tick, 0);
    assert_eq!(open.committed.len(), 1);
    assert_eq!(ended.record.ticks_closed(), 0);
    assert_eq!(verse.now().await, 0, "the open tick's clock did not advance");
    assert_eq!(first.calls(), 1, "the second call never reached the provider");
    assert_eq!(verse.acts().await, 1);
    // The open tick is reported, unclosed, with what it committed.
    assert_eq!(reports.len(), 1);
    assert!(!reports[0].checkpoint.closed);
    assert_eq!(reports[0].acts.len(), 1);
    assert_eq!(&reports[0].checkpoint, open);
    assert!(reports[0].remaining < first_bound);
    assert_eq!(
        verse.runs.runs(world.world_id).unwrap(),
        vec![ended.record.clone()]
    );

    // A later run, under its own cap, finishes the tick and costs no call for
    // the act already committed.
    let second = ScriptedPort::speaking();
    let mut resumed_reports = Vec::new();
    let resumed = verse
        .run(second.clone(), u64::MAX, 2, |report| {
            resumed_reports.push(report.clone());
            ControlFlow::Continue(())
        })
        .await;
    assert_eq!(resumed.record.status, RunStatus::Completed);
    assert_eq!(resumed.record.sequence, 2);
    // The resumed tick is the document run 1 opened, closed by run 2.
    let opened = verse.runs.tick(world.world_id, 0).unwrap().unwrap();
    assert_eq!(opened.opened_by_run, 1);
    assert_eq!(opened.closed, Some(TickClose::Clock { run: 2 }));
    let ticks: Vec<u64> = resumed.record.checkpoints.iter().map(|c| c.tick).collect();
    assert_eq!(ticks, vec![0, 1], "the open tick is the first this run closes");
    assert_eq!(verse.now().await, 2 * u64::from(SPAN));
    assert_eq!(second.calls(), 3, "one call finished tick 0, two ran tick 1");
    assert_eq!(verse.acts().await, 4, "no act was committed twice");
    // The frozen cells run in their frozen order: the cell that already acted
    // answers with its original receipt and costs no call, and the one the
    // first run left in flight finishes.
    let closing = &resumed.record.checkpoints[0];
    assert!(closing.closed);
    assert_eq!(closing.cells, 2);
    assert_eq!(closing.committed.len(), 2, "{closing:?}");
    assert_eq!(closing.committed[0], open.committed[0], "{closing:?}");
    assert_ne!(closing.committed[1], open.committed[0], "{closing:?}");
    assert!(closing.refusals.is_empty(), "{closing:?}");
    assert_eq!(resumed.record.ticks_closed(), 2);
    // The tick reports the act this run committed and not the one it found.
    assert_eq!(resumed_reports[0].acts.len(), 1);
    assert!(resumed_reports[0].acts[0].revision > reports[0].acts[0].revision);
    assert_ne!(
        resumed_reports[0].acts[0].speaker_label,
        reports[0].acts[0].speaker_label
    );
    assert_eq!(resumed.record.checkpoints[1].committed.len(), 2);
}

/// Both cells of a tick committed through another driver before any run opened
/// it, one after the other, so each one's act has since moved the other's
/// scope. The run that opens the tick reaches both committed rows, in whatever
/// order it froze, and answers each with its receipt: no inference, no refusal,
/// no second act. `reverse` flips which cell the other driver committed first,
/// so the answer is shown not to depend on the order the cells are reached in.
async fn resumed_after_both_cells_committed_elsewhere(reverse: bool) {
    let (_dir, verse) = Harness::fresh().await;
    let snapshot = verse.mailbox.snapshot().await.unwrap();
    let graph = verse.mailbox.agency_graph().await.unwrap();
    let cover = derive_cover(
        snapshot.world_id,
        TickIndex(0),
        &snapshot.opportunities,
        &graph,
        CoverBudget {
            cells: 2,
            constituent_cap: 8,
            urgency_slots: 0,
        },
    );
    assert_eq!(cover.cells.len(), 2);
    let elsewhere = ScriptedPort::speaking();
    let runner = ControllerRunner::open(
        verse.mailbox.clone(),
        elsewhere.clone(),
        verse.work.clone(),
        models(),
    )
    .unwrap();
    let order: Vec<&Cell> = if reverse {
        cover.cells.iter().rev().collect()
    } else {
        cover.cells.iter().collect()
    };
    for cell in order {
        let fresh = verse.mailbox.opportunities().await.unwrap();
        let bound = refreshed(&FrozenCell::of(cell), TickIndex(0), &fresh).unwrap();
        runner.run_cell(&bound).await.unwrap();
    }
    assert_eq!(verse.acts().await, 2);

    let resuming = ScriptedPort::speaking();
    let ran = verse
        .run_quietly_under(resuming.clone(), config_of(1, 60, 2))
        .await;
    assert_eq!(ran.record.status, RunStatus::Completed);
    let [closing] = ran.record.checkpoints.as_slice() else {
        panic!("the run closed one tick")
    };
    assert!(closing.closed);
    assert!(closing.refusals.is_empty(), "{closing:?}");
    assert_eq!(closing.committed.len(), 2, "{closing:?}");
    assert_eq!(resuming.calls(), 0, "a committed cell was asked again");
    assert_eq!(verse.acts().await, 2, "no act was committed twice");
}

#[tokio::test]
async fn committed_cells_are_answered_in_the_order_the_tick_froze() {
    resumed_after_both_cells_committed_elsewhere(false).await;
}

#[tokio::test]
async fn committed_cells_are_answered_against_the_order_the_tick_froze() {
    resumed_after_both_cells_committed_elsewhere(true).await;
}

#[tokio::test]
async fn recorded_run_replays_to_the_same_journal() {
    let live_dir = tempfile::tempdir().unwrap();
    create_world(live_dir.path()).await;
    let replay_dir = tempfile::tempdir().unwrap();
    std::fs::copy(
        live_dir.path().join("world.cc"),
        replay_dir.path().join("world.cc"),
    )
    .unwrap();
    let live = Harness::open(live_dir.path()).await;
    let replayed = Harness::open(replay_dir.path()).await;

    let provider = ScriptedPort::speaking();
    let recorded = live.run_quietly(provider.clone(), u64::MAX, 3).await;
    assert_eq!(recorded.record.status, RunStatus::Completed);

    // The same run again, from the recorded replies and no provider.
    let start = RunStart::of(&replayed.mailbox.snapshot().await.unwrap());
    let replay = Arc::new(
        ReplayInferencePort::new("verse-replay", &live.calls, &call_run_id(1), &start).unwrap(),
    );
    let again = replayed.run_quietly(replay, u64::MAX, 3).await;
    assert_eq!(again.record.status, RunStatus::Completed);
    assert_eq!(provider.calls(), 6, "replay made no provider call");

    assert_eq!(again.record.checkpoints, recorded.record.checkpoints);
    let live_world = live.mailbox.snapshot().await.unwrap();
    let replayed_world = replayed.mailbox.snapshot().await.unwrap();
    assert_eq!(replayed_world.revision, live_world.revision);
    assert_eq!(replayed_world.state_digest, live_world.state_digest);
    assert_eq!(
        replayed.mailbox.operator_log().await.unwrap(),
        live.mailbox.operator_log().await.unwrap()
    );
    // Both stores hold the same calls, charged the same.
    assert_eq!(
        replayed.calls.records(&call_run_id(1)).unwrap(),
        live.calls.records(&call_run_id(1)).unwrap()
    );
}

#[tokio::test]
async fn a_run_the_caller_stops_ends_after_a_finished_tick() {
    let (_dir, verse) = Harness::fresh().await;
    let stopped = verse
        .run(ScriptedPort::speaking(), u64::MAX, 3, |_| {
            ControlFlow::Break(())
        })
        .await;
    assert_eq!(stopped.record.status, RunStatus::Stopped);
    assert_eq!(stopped.record.checkpoints.len(), 1);
    assert_eq!(verse.now().await, u64::from(SPAN));

    // A stop asked on the last tick is not a stop: the run was done.
    let done = verse
        .run(ScriptedPort::speaking(), u64::MAX, 1, |_| {
            ControlFlow::Break(())
        })
        .await;
    assert_eq!(done.record.status, RunStatus::Completed);
}

#[tokio::test]
async fn a_pending_cell_ends_the_run_without_the_clock_and_a_later_run_finishes_it() {
    let (_dir, verse) = Harness::fresh().await;
    let flaky = ScriptedPort::speaking();
    flaky.fail_next(InferenceFault::retryable("the provider is busy"));
    let ended = verse.run_quietly(flaky.clone(), u64::MAX, 2).await;
    assert_eq!(
        ended.record.status,
        RunStatus::Failed {
            reason: "inference_retryable".into()
        }
    );
    assert_eq!(ended.record.ticks_closed(), 0);
    let [open] = ended.record.checkpoints.as_slice() else {
        panic!("the open tick is recorded")
    };
    assert!(!open.closed && open.committed.is_empty());
    assert_eq!(verse.now().await, 0);
    assert_eq!(verse.acts().await, 0);

    let finished = verse
        .run_quietly(ScriptedPort::speaking(), u64::MAX, 1)
        .await;
    assert_eq!(finished.record.status, RunStatus::Completed);
    assert_eq!(verse.now().await, u64::from(SPAN));
    assert_eq!(verse.acts().await, 2);
}

#[tokio::test]
async fn a_provider_answer_against_the_tool_contract_ends_the_run_without_the_clock() {
    let (_dir, verse) = Harness::fresh().await;
    // The faction acts and then calls finish_without_proposal, which
    // contradicts its own choice.
    let port = ScriptedPort::new(|request| {
        let name = who(request);
        vec![
            speak_call(&format!("{name} speaks.")),
            InferenceEvent::ToolCall {
                call_id: "finish".into(),
                name: "finish_without_proposal".into(),
                arguments: "{}".into(),
            },
        ]
    });
    let outcome = verse.run_quietly(port, u64::MAX, 1).await;
    assert_eq!(
        outcome.record.status,
        RunStatus::Failed {
            reason: "inference_recovery_required".into()
        }
    );
    assert_eq!(outcome.record.ticks_closed(), 0);
    assert_eq!(verse.now().await, 0);
}

#[tokio::test]
async fn a_cell_the_world_refuses_is_a_refusal_and_the_tick_closes() {
    let (_dir, verse) = Harness::fresh().await;
    // Adrasteia attacks something that is not in the world; Brannoch speaks.
    let nothing = id_text(CommandId::new());
    let port = ScriptedPort::new(move |request| {
        let name = who(request);
        if name == "Adrasteia" {
            vec![InferenceEvent::ToolCall {
                call_id: "act".into(),
                name: "attack".into(),
                arguments: json!({ "target": nothing, "text": "Adrasteia strikes." }).to_string(),
            }]
        } else {
            vec![speak_call("Brannoch speaks.")]
        }
    });
    let outcome = verse.run_quietly(port, u64::MAX, 1).await;
    assert_eq!(outcome.record.status, RunStatus::Completed);
    let [checkpoint] = outcome.record.checkpoints.as_slice() else {
        panic!("one tick, one checkpoint")
    };
    assert_eq!(checkpoint.cells, 2);
    assert_eq!(checkpoint.committed.len(), 1);
    assert_eq!(checkpoint.refusals.len(), 1);
    assert_eq!(checkpoint.refusals[0].reason, RefusalReason::WorldRejected);
    assert!(checkpoint.refusals[0].cell < 2);
    assert_eq!(verse.now().await, u64::from(SPAN));
    assert_eq!(verse.acts().await, 1);
}

#[test]
fn the_operator_reads_refusals_and_how_a_run_ended() {
    let report = TickReport {
        checkpoint: TickCheckpoint {
            tick: 4,
            closed: false,
            cells: 3,
            committed: Vec::new(),
            refusals: vec![
                Refusal {
                    cell: 1,
                    reason: RefusalReason::WorldRejected,
                },
                Refusal {
                    cell: 2,
                    reason: RefusalReason::OpportunityGone,
                },
                Refusal {
                    cell: 0,
                    reason: RefusalReason::Interrupted,
                },
                Refusal {
                    cell: 5,
                    reason: RefusalReason::ScopeMoved,
                },
            ],
            tokens: 7,
        },
        acts: Vec::new(),
        spent: 40,
        remaining: 60,
    };
    assert_eq!(
        report.to_string(),
        concat!(
            "tick 4 open: 3 cells, 0 acts, 4 refused\n",
            "  refused: cell 1 world_rejected\n",
            "  refused: cell 2 opportunity_gone\n",
            "  refused: cell 0 interrupted\n",
            "  refused: cell 5 scope_moved\n",
            "  tokens: 7 this tick, 40 spent, 60 remaining\n",
        )
    );
    let ended: Vec<String> = [
        RunStatus::Running,
        RunStatus::Completed,
        RunStatus::BudgetExhausted,
        RunStatus::Failed {
            reason: "clock_refused".into(),
        },
        RunStatus::Stopped,
    ]
    .iter()
    .map(ToString::to_string)
    .collect();
    assert_eq!(
        ended,
        ["Running", "Completed", "BudgetExhausted", "Failed (clock_refused)", "Stopped"]
    );
}

#[tokio::test]
async fn an_act_names_its_verb_render_path_and_bound_roles() {
    let (_dir, verse) = Harness::fresh().await;
    let target = id_text(verse.subject("Brannoch").await);
    let port = ScriptedPort::new(move |request| {
        if who(request) == "Adrasteia" {
            vec![InferenceEvent::ToolCall {
                call_id: "act".into(),
                name: "attack".into(),
                arguments: json!({ "target": target, "text": "Adrasteia strikes." }).to_string(),
            }]
        } else {
            vec![speak_call("Brannoch answers.")]
        }
    });
    let mut reports = Vec::new();
    let outcome = verse
        .run(port, u64::MAX, 1, |report| {
            reports.push(report.clone());
            ControlFlow::Continue(())
        })
        .await;
    assert_eq!(outcome.record.status, RunStatus::Completed);
    let catalog = read_aetheria_catalog(FIXTURE).unwrap();
    let acts = &reports[0].acts;
    let attack = acts.iter().find(|act| act.verb == "attack").unwrap();
    assert_eq!(attack.speaker_label, "Adrasteia");
    assert_eq!(
        attack.render_path,
        Some(catalog.grammar.verb("attack").unwrap().render_path)
    );
    assert_eq!(
        attack.roles,
        vec![BoundRole {
            role: "target".into(),
            target: "Brannoch".into()
        }]
    );

    // And the operator reads them so.
    let name = |verb: &str| match catalog.grammar.verb(verb).unwrap().render_path {
        RenderPath::ShipAction => "ship_action",
        RenderPath::Conversation => "conversation",
    };
    let text = reports[0].to_string();
    let checkpoint = &reports[0].checkpoint;
    assert!(
        text.starts_with("tick 0 closed: 2 cells, 2 acts, 0 refused\n"),
        "{text}"
    );
    let attack_line = format!(
        "\n  Adrasteia attack [{}] target=Brannoch says \"Adrasteia strikes.\"\n",
        name("attack")
    );
    assert!(text.contains(&attack_line), "{text}");
    let speak_line = format!(
        "\n  Brannoch speak [{}] says \"Brannoch answers.\"\n",
        name("speak")
    );
    assert!(text.contains(&speak_line), "{text}");
    let tokens_line = format!(
        "  tokens: {} this tick, {} spent, {} remaining\n",
        checkpoint.tokens, reports[0].spent, reports[0].remaining
    );
    assert!(text.ends_with(&tokens_line), "{text}");
    let speak = acts.iter().find(|act| act.verb == "speak").unwrap();
    assert_eq!(speak.speaker_label, "Brannoch");
    assert_eq!(
        speak.render_path,
        Some(catalog.grammar.verb("speak").unwrap().render_path)
    );
    assert!(speak.roles.is_empty());
}

#[tokio::test]
async fn the_opportunity_door_answers_what_a_snapshot_carries() {
    let (_dir, verse) = Harness::fresh().await;
    let snapshot = verse.mailbox.snapshot().await.unwrap();
    assert_eq!(snapshot.opportunities.len(), 2);
    assert_eq!(
        verse.mailbox.opportunities().await.unwrap(),
        snapshot.opportunities
    );
}

#[tokio::test]
async fn a_cell_is_run_on_what_the_world_holds_for_its_members_now() {
    let (_dir, verse) = Harness::fresh().await;
    let opportunities = verse.mailbox.opportunities().await.unwrap();
    let graph = verse.mailbox.agency_graph().await.unwrap();
    let world = verse.mailbox.snapshot().await.unwrap().world_id;
    let singles = derive_cover(
        world,
        TickIndex(0),
        &opportunities,
        &graph,
        two_cells(),
    );
    let group = derive_cover(
        world,
        TickIndex(0),
        &opportunities,
        &graph,
        CoverBudget {
            cells: 1,
            constituent_cap: 8,
            urgency_slots: 0,
        },
    );
    let [grouped @ Cell::Group { members, .. }] = group.cells.as_slice() else {
        panic!("one cell for two subjects is a group")
    };
    assert_eq!(members.len(), 2);

    // Changed opportunities replace the ones the cover held.
    let mut moved = opportunities.clone();
    for opportunity in &mut moved {
        opportunity.revision += 1;
    }
    for cell in &singles.cells {
        let Cell::Singleton { member: held, .. } = cell else {
            panic!("two subjects under two cells are singletons")
        };
        let frozen = FrozenCell::of(cell);
        let Some(Cell::Singleton { member, .. }) = refreshed(&frozen, TickIndex(0), &moved) else {
            panic!("a subject that still has an opportunity keeps its cell")
        };
        assert_ne!(member.opportunity, held.opportunity);
        assert!(moved.contains(&member.opportunity));
        // The member is bound to its own subject's opportunity, not a neighbour's.
        assert_eq!(member.subject, held.subject);
        assert_eq!(member.opportunity.scope.subject_id, member.subject);
    }
    let frozen_group = FrozenCell::of(grouped);
    let Some(Cell::Group { members: now, .. }) = refreshed(&frozen_group, TickIndex(0), &moved)
    else {
        panic!("a group keeps its members")
    };
    assert_eq!(now.len(), 2);
    assert!(now.iter().all(|member| moved.contains(&member.opportunity)));
    assert!(
        now.iter()
            .all(|member| member.opportunity.scope.subject_id == member.subject)
    );

    // A subject with no opportunity leaves its cell, and an empty cell is gone.
    let one = &opportunities[..1];
    let Some(Cell::Group { members: left, .. }) = refreshed(&frozen_group, TickIndex(0), one)
    else {
        panic!("one member remains")
    };
    assert_eq!(left.len(), 1);
    assert_eq!(left[0].opportunity.scope, one[0].scope);
    let gone = singles
        .cells
        .iter()
        .filter(|cell| refreshed(&FrozenCell::of(cell), TickIndex(0), one).is_none())
        .count();
    assert_eq!(gone, 1);
    assert!(refreshed(&frozen_group, TickIndex(0), &[]).is_none());
}

#[test]
fn an_error_ends_a_cell_as_the_run_must_treat_it() {
    let budget = ControllerError::Inference {
        purpose: InferencePurpose::OperationalAgent,
        source: InferenceFault::budget_exhausted(),
    };
    assert!(matches!(classify(&budget), CellEnd::Budget));
    // Any other fault that stops the cognition organ stops the run.
    let corrupted = ControllerError::Inference {
        purpose: InferencePurpose::OperationalAgent,
        source: InferenceFault::integrity_violation("the reply cannot be trusted"),
    };
    assert!(matches!(
        classify(&corrupted),
        CellEnd::Fatal("controller_quarantined")
    ));
    assert!(matches!(
        classify(&ControllerError::Serialization("x".into())),
        CellEnd::Fatal("controller_quarantined")
    ));
    // What the world or the subject's standing refused stays with its cell.
    assert!(matches!(
        classify(&ControllerError::World(KernelError::AffordanceDenied)),
        CellEnd::Refused(RefusalReason::WorldRejected)
    ));
    for gone in [
        ControllerError::NoOpportunity {
            expected: ControllerMode::OperationalAgent,
        },
        ControllerError::AmbiguousOpportunity,
    ] {
        assert!(matches!(
            classify(&gone),
            CellEnd::Refused(RefusalReason::OpportunityGone)
        ));
    }
    assert!(matches!(
        classify(&ControllerError::OpportunityMismatch),
        CellEnd::Refused(RefusalReason::ScopeMoved)
    ));
    assert!(matches!(
        classify(&ControllerError::CommandMismatch),
        CellEnd::Fatal("controller_error")
    ));
}

/// One fact, one label: the world refusing a body whose scope moved is
/// `ScopeMoved`, the same label the controller's own refusal of in-flight work
/// carries, and not the catch-all `WorldRejected`.
#[tokio::test]
async fn a_world_refusal_for_a_moved_scope_is_scope_moved() {
    let (_dir, verse) = Harness::fresh().await;
    let opportunity = verse.mailbox.snapshot().await.unwrap().opportunities[0].clone();
    let moved = ControllerError::World(KernelError::ScopeChanged {
        scope: opportunity.scope.clone(),
        expected: opportunity.scope_digest.clone(),
        actual: opportunity.scope_digest.clone(),
    });
    assert!(matches!(
        classify(&moved),
        CellEnd::Refused(RefusalReason::ScopeMoved)
    ));
    assert!(matches!(
        classify(&ControllerError::World(KernelError::AffordanceDenied)),
        CellEnd::Refused(RefusalReason::WorldRejected)
    ));
}

/// A recording port over the harness's call store for `sequence` under `cap`,
/// and a runner on it.
fn port_and_runner(
    verse: &Harness,
    world: &WorldSnapshot,
    sequence: u64,
    cap: u64,
) -> (Arc<RecordingInferencePort>, ControllerRunner) {
    let port = Arc::new(
        RecordingInferencePort::open(
            ScriptedPort::speaking(),
            verse.calls.clone(),
            call_run_id(sequence),
            cap,
            &RunStart::of(world),
        )
        .unwrap(),
    );
    let runner = ControllerRunner::open(
        verse.mailbox.clone(),
        port.clone(),
        verse.work.clone(),
        models(),
    )
    .unwrap();
    (port, runner)
}

/// Runs `claim` on the given port and runner, which must refuse it.
async fn refused(
    verse: &Harness,
    port: &RecordingInferencePort,
    runner: &ControllerRunner,
    claim: RunClaim,
    config: &VerseRunConfig,
) -> VerseRunError {
    run_manual(
        &verse.mailbox,
        runner,
        port,
        &verse.runs,
        claim,
        config,
        |_| ControlFlow::Continue(()),
    )
    .await
    .unwrap_err()
}

#[tokio::test]
async fn a_run_started_on_the_wrong_port_world_or_config_is_refused_and_its_claim_ends_failed() {
    let (_dir, verse) = Harness::fresh().await;
    let world = verse.mailbox.snapshot().await.unwrap();
    let (_other_dir, elsewhere) = Harness::fresh().await;
    let strange = elsewhere.mailbox.snapshot().await.unwrap();

    // A cover budget with no cells cannot be claimed: no record exists.
    let mut bad = config(1);
    bad.cover.cells = 0;
    assert!(matches!(
        verse.runs.claim(&world, 1_000, &bad),
        Err(VerseRunError::Budget(CoverBudgetError::NoCells))
    ));
    assert!(verse.runs.runs(world.world_id).unwrap().is_empty());

    // Run 1: a port opened for another run's number.
    let claim = verse.runs.claim(&world, 1_000, &config(1)).unwrap();
    let (port, runner) = port_and_runner(&verse, &world, 7, 1_000);
    let error = refused(&verse, &port, &runner, claim, &config(1)).await;
    assert!(matches!(error, VerseRunError::PortNotThisRun));

    // Run 2: a port under another cap than the claim recorded.
    let claim = verse.runs.claim(&world, 1_000, &config(1)).unwrap();
    let (port, runner) = port_and_runner(&verse, &world, claim.sequence(), 5_000);
    let error = refused(&verse, &port, &runner, claim, &config(1)).await;
    assert!(matches!(error, VerseRunError::PortNotThisRun));

    // Run 3: a config other than the one the claim was made with.
    let claim = verse.runs.claim(&world, 1_000, &config(1)).unwrap();
    let (port, runner) = port_and_runner(&verse, &world, claim.sequence(), 1_000);
    let error = refused(&verse, &port, &runner, claim, &config(2)).await;
    assert!(matches!(error, VerseRunError::ConfigNotThisClaim));

    // Another world's claim 1, on this world's mailbox and a port that fits it.
    let claim = verse.runs.claim(&strange, 1_000, &config(1)).unwrap();
    let (port, runner) = port_and_runner(&verse, &world, claim.sequence(), 1_000);
    let error = refused(&verse, &port, &runner, claim, &config(1)).await;
    assert!(matches!(error, VerseRunError::PortNotThisRun));

    let statuses = |world: &WorldSnapshot| -> Vec<(u64, RunStatus)> {
        verse
            .runs
            .runs(world.world_id)
            .unwrap()
            .into_iter()
            .map(|record| (record.sequence, record.status))
            .collect()
    };
    assert_eq!(
        statuses(&world),
        vec![
            (1, failed("port_not_this_run")),
            (2, failed("port_not_this_run")),
            (3, failed("config_not_this_claim")),
        ]
    );
    assert_eq!(statuses(&strange), vec![(1, failed("port_not_this_run"))]);
    assert!(verse.runs.tick(world.world_id, 0).unwrap().is_none());
    assert!(verse.runs.tick(strange.world_id, 0).unwrap().is_none());
    assert_eq!(verse.mailbox.operator_log().await.unwrap().len(), 0);
}

fn record(world: WorldId, sequence: u64) -> VerseRunRecord {
    VerseRunRecord {
        world_id: world,
        sequence,
        token_cap: 10,
        ticks_requested: 1,
        tick_span_minutes: SPAN,
        cover_budget: RecordedCoverBudget {
            cells: 2,
            constituent_cap: 8,
            urgency_slots: 0,
        },
        start_revision: 0,
        checkpoints: Vec::new(),
        status: RunStatus::Running,
    }
}

#[tokio::test]
async fn the_run_store_claims_runs_per_world_and_replaces_a_claimed_run_with_its_later_state() {
    let (_one_dir, one) = Harness::fresh().await;
    let (_other_dir, other) = Harness::fresh().await;
    let one = one.mailbox.snapshot().await.unwrap();
    let other = other.mailbox.snapshot().await.unwrap();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("runs.cc");
    let store = VerseRunStore::open(&path).unwrap();

    // Claims number runs from one, per world, and write the Running record.
    let first = store.claim(&one, 10, &config(3)).unwrap();
    let second = store.claim(&one, 20, &config(1)).unwrap();
    let elsewhere = store.claim(&other, 10, &config(1)).unwrap();
    assert_eq!(
        (first.sequence(), second.sequence(), elsewhere.sequence()),
        (1, 2, 1)
    );
    let claimed = store.runs(one.world_id).unwrap();
    assert_eq!(claimed.len(), 2);
    assert_eq!(claimed[0].token_cap, 10);
    assert_eq!(claimed[0].ticks_requested, 3);
    assert_eq!(claimed[1].token_cap, 20);
    assert_eq!(claimed[0].start_revision, one.revision);
    assert_eq!(claimed[0].status, RunStatus::Running);

    // A record is only ever a claimed one: a sequence with no record is refused.
    assert!(matches!(
        store.save(&record(one.world_id, 3)),
        Err(VerseRunError::OutOfSequence)
    ));
    assert!(matches!(
        store.save(&record(one.world_id, 0)),
        Err(VerseRunError::OutOfSequence)
    ));
    assert_eq!(store.runs(one.world_id).unwrap().len(), 2);

    // Saving a claimed run again replaces it, and the file reopens to the
    // same rows.
    let mut finished = claimed[0].clone();
    finished.status = RunStatus::Completed;
    store.save(&finished).unwrap();
    drop(store);
    let store = VerseRunStore::open(&path).unwrap();
    assert_eq!(
        store.runs(one.world_id).unwrap(),
        vec![finished, claimed[1].clone()]
    );
    assert_eq!(store.runs(other.world_id).unwrap().len(), 1);
}

#[tokio::test]
async fn a_claim_is_never_reissued_even_when_its_run_never_started() {
    let (_dir, verse) = Harness::fresh().await;
    let world = verse.mailbox.snapshot().await.unwrap();

    // Run 1 is claimed, then nothing happens: the process died.
    let lost = verse.runs.claim(&world, 1_000_000_000, &config(1)).unwrap();
    assert_eq!(lost.sequence(), 1);
    let before = verse.runs.runs(world.world_id).unwrap();
    drop(lost);

    // The next run is run 2 and leaves run 1's record exactly as it was.
    let outcome = verse
        .run_quietly(ScriptedPort::speaking(), 1_000_000_000, 1)
        .await;
    assert_eq!(outcome.record.sequence, 2);
    assert_eq!(outcome.record.status, RunStatus::Completed);
    let after = verse.runs.runs(world.world_id).unwrap();
    assert_eq!(after.len(), 2);
    assert_eq!(after[0], before[0], "run 1's record was overwritten");
    assert_eq!(after[0].status, RunStatus::Running);
    assert_eq!(after[1], outcome.record);
}

#[tokio::test]
async fn a_start_refused_after_the_claim_leaves_the_next_run_free_to_choose_its_cap() {
    let (_dir, verse) = Harness::fresh().await;
    let world = verse.mailbox.snapshot().await.unwrap();

    // Run 1 is claimed and its call ledger opens under a cap, then the start
    // is refused before the run begins.
    let claim = verse.runs.claim(&world, 1_000, &config(1)).unwrap();
    let ledger = RecordingInferencePort::open(
        ScriptedPort::speaking(),
        verse.calls.clone(),
        call_run_id(claim.sequence()),
        1_000,
        &RunStart::of(&world),
    )
    .unwrap();
    drop(ledger);
    verse.runs.abandon(claim, "start_refused").unwrap();

    // The operator tries again under another cap: it is a new run, and runs.
    let outcome = verse
        .run_quietly(ScriptedPort::speaking(), 50_000, 1)
        .await;
    assert_eq!(outcome.record.sequence, 2);
    assert_eq!(outcome.record.token_cap, 50_000);
    assert_eq!(outcome.record.status, RunStatus::Completed);
    let runs = verse.runs.runs(world.world_id).unwrap();
    assert_eq!(runs[0].status, failed("start_refused"));
    assert_eq!(runs[0].token_cap, 1_000);
}

#[tokio::test]
async fn an_unreadable_operator_log_is_no_tick_report() {
    let (_dir, verse) = Harness::fresh().await;
    verse.run_quietly(ScriptedPort::speaking(), u64::MAX, 2).await;
    let log = verse.mailbox.operator_log().await.unwrap();
    assert!(log.len() >= 4, "two ticks of two cells acted");

    // A log that cannot be read is no tick report, whatever the revision.
    assert!(acts_since(Err(MailboxError::Unavailable), 0).is_none());
    assert!(acts_since(Err(MailboxError::Unavailable), u64::MAX).is_none());

    // A log that can is cut at the tick's opening revision.
    assert_eq!(acts_since(Ok(log.clone()), 0), Some(log.clone()));
    let middle = log[1].revision;
    let later: Vec<_> = log.iter().filter(|e| e.revision > middle).cloned().collect();
    assert!(!later.is_empty() && later.len() < log.len());
    assert_eq!(acts_since(Ok(log.clone()), middle), Some(later));
    assert_eq!(
        acts_since(Ok(log), u64::MAX),
        Some(Vec::new()),
        "an empty tick is a readable one"
    );
}

#[test]
fn the_run_store_refuses_rows_that_are_not_canonical_run_records() {
    let dir = tempfile::tempdir().unwrap();
    let world = WorldId::nil_for_test();
    let good = record(world, 1);
    let envelope = |key: String, schema: &str, payload: Vec<u8>| CultCacheEnvelope {
        key,
        r#type: RUN_ROW.into(),
        payload,
        stored_at: Utc::now().to_rfc3339(),
        schema_id: Some(schema.into()),
    };
    let named = rmp_serde::to_vec_named(&good).unwrap();
    let cases = [
        // Another schema version is not migrated.
        envelope(row_key(world, 1), "ghostlight.verse_run.v2", named.clone()),
        // A row under a key that is not its own.
        envelope(row_key(world, 9), RUN_RECORD_SCHEMA, named.clone()),
        // A payload that is not the record.
        envelope(row_key(world, 1), RUN_RECORD_SCHEMA, vec![0xc0]),
        // The record in a non-canonical encoding.
        envelope(
            row_key(world, 1),
            RUN_RECORD_SCHEMA,
            rmp_serde::to_vec(&good).unwrap(),
        ),
        // A sequence of zero.
        envelope(
            row_key(world, 0),
            RUN_RECORD_SCHEMA,
            rmp_serde::to_vec_named(&record(world, 0)).unwrap(),
        ),
    ];
    for (index, row) in cases.into_iter().enumerate() {
        let file = dir.path().join(format!("case-{index}.cc"));
        {
            let mut raw = OwnedRedbMessagePackBackingStore::new(&file).unwrap();
            raw.push(&row).unwrap();
        }
        assert!(
            matches!(VerseRunStore::open(&file), Err(VerseRunError::Corrupt)),
            "case {index} was accepted"
        );
    }
}

/// A port that answers like `ScriptedPort::speaking` and fails the `at`th
/// provider call it sees, retryably.
struct FaultAt {
    inner: Arc<ScriptedPort>,
    at: usize,
    seen: AtomicUsize,
}

#[async_trait]
impl InferencePort for FaultAt {
    fn prepare(&self, request: InferenceRequest) -> Result<PreparedInference, InferenceFault> {
        self.inner.prepare(request)
    }

    async fn infer(&self, request: PreparedInference) -> Result<InferenceOutput, InferenceFault> {
        if self.seen.fetch_add(1, Ordering::SeqCst) == self.at {
            return Err(InferenceFault::retryable("the provider is busy"));
        }
        self.inner.infer(request).await
    }
}

fn fault_at(at: usize) -> Arc<FaultAt> {
    Arc::new(FaultAt {
        inner: ScriptedPort::speaking(),
        at,
        seen: AtomicUsize::new(0),
    })
}

fn config_of(ticks: u32, span: u32, cells: u16) -> VerseRunConfig {
    VerseRunConfig {
        ticks,
        tick_span: TickMinutes::new(span).unwrap(),
        cover: CoverBudget {
            cells,
            constituent_cap: 8,
            urgency_slots: 0,
        },
    }
}

/// Run 1 closes tick 0 (two cells, 60 minutes). Run 2 commits one faction's act
/// in tick 1 and ends pending. Run 3 resumes under `resume`. Asserts that each
/// faction acted exactly once between the end of run 1 and the end of run 3.
async fn resumed_after_a_pending_cell(
    resume: VerseRunConfig,
) -> (tempfile::TempDir, Harness, RunOutcome) {
    let (dir, verse) = Harness::fresh().await;
    let first = verse
        .run_quietly_under(ScriptedPort::speaking(), config_of(1, 60, 2))
        .await;
    assert_eq!(first.record.status, RunStatus::Completed);
    let mark = verse.mailbox.snapshot().await.unwrap().revision;

    let ended = verse
        .run_quietly_under(fault_at(1), config_of(1, 60, 2))
        .await;
    assert_eq!(
        ended.record.status,
        RunStatus::Failed {
            reason: "inference_retryable".into()
        }
    );
    let opened: Vec<_> = verse
        .mailbox
        .operator_log()
        .await
        .unwrap()
        .into_iter()
        .filter(|event| event.revision > mark)
        .map(|event| event.speaker)
        .collect();
    assert_eq!(opened.len(), 1, "one faction acted in the open tick");

    let resumed = verse
        .run_quietly_under(ScriptedPort::speaking(), resume)
        .await;
    assert_eq!(resumed.record.status, RunStatus::Completed);
    let since: Vec<_> = verse
        .mailbox
        .operator_log()
        .await
        .unwrap()
        .into_iter()
        .filter(|event| event.revision > mark)
        .map(|event| event.speaker)
        .collect();
    assert_eq!(since.len(), 2, "each faction acted once in the open tick");
    assert_eq!(
        since.iter().filter(|speaker| **speaker == opened[0]).count(),
        1,
        "the faction that acted before the pause did not act again"
    );
    (dir, verse, resumed)
}

#[tokio::test]
async fn a_resume_under_another_span_finishes_the_open_tick_under_its_own() {
    let (_dir, verse, resumed) = resumed_after_a_pending_cell(config_of(1, 30, 2)).await;
    // Tick 1's clock advance is the frozen 60 minutes, not the resuming 30.
    assert_eq!(verse.now().await, 120);
    let world = verse.mailbox.snapshot().await.unwrap().world_id;
    let tick = verse.runs.tick(world, 1).unwrap().unwrap();
    assert_eq!(tick.span_minutes, 60);
    assert_eq!(tick.opened_by_run, 2);
    assert_eq!(tick.closed, Some(TickClose::Clock { run: 3 }));
    let [closing] = resumed.record.checkpoints.as_slice() else {
        panic!("the resuming run closed one tick")
    };
    assert_eq!(closing.tick, 1);
    assert!(closing.closed);
}

#[tokio::test]
async fn a_resume_under_another_cover_budget_keeps_the_frozen_cells() {
    // One cell would be a group of both factions; the open tick keeps its two
    // singletons.
    let (_dir, verse, resumed) = resumed_after_a_pending_cell(config_of(1, 60, 1)).await;
    let world = verse.mailbox.snapshot().await.unwrap().world_id;
    let tick = verse.runs.tick(world, 1).unwrap().unwrap();
    assert_eq!(tick.cover_budget.cells, 2);
    assert_eq!(tick.cells.len(), 2);
    assert!(tick.cells.iter().all(|cell| !cell.group));
    assert_eq!(resumed.record.checkpoints[0].cells, 2);
    assert_eq!(verse.now().await, 120);
}

#[tokio::test]
async fn a_new_span_opens_a_new_tick_and_never_reuses_a_closed_one() {
    let (_dir, verse) = Harness::fresh().await;
    let first = verse
        .run_quietly_under(ScriptedPort::speaking(), config_of(2, 60, 2))
        .await;
    assert_eq!(first.record.status, RunStatus::Completed);
    assert_eq!(verse.now().await, 120);
    let acts = verse.acts().await;

    let port = ScriptedPort::speaking();
    let second = verse
        .run_quietly_under(port.clone(), config_of(2, 120, 2))
        .await;
    assert_eq!(second.record.status, RunStatus::Completed);
    let ordinals: Vec<u64> = second.record.checkpoints.iter().map(|c| c.tick).collect();
    assert_eq!(ordinals, vec![2, 3]);
    assert_eq!(verse.now().await, 120 + 240);
    assert_eq!(verse.acts().await, acts + 4);
    assert_eq!(port.calls(), 4);
    let before: Vec<CommandId> = first
        .record
        .checkpoints
        .iter()
        .flat_map(|c| c.committed.clone())
        .collect();
    for checkpoint in &second.record.checkpoints {
        assert!(checkpoint.committed.iter().all(|id| !before.contains(id)));
    }
}

#[tokio::test]
async fn a_closed_ticks_clock_is_answered_from_the_kernels_ledger() {
    let (_dir, verse) = Harness::fresh().await;
    verse.run_quietly(ScriptedPort::speaking(), u64::MAX, 2).await;
    let world = verse.mailbox.snapshot().await.unwrap().world_id;
    assert_eq!(verse.now().await, 120);
    for ordinal in 0..2 {
        let receipt = verse
            .mailbox
            .submit_clock(
                clock_command_id(world, ordinal),
                TickMinutes::new(SPAN).unwrap(),
            )
            .await
            .unwrap();
        assert!(
            matches!(receipt, SubmitReceipt::AlreadyApplied(_)),
            "the clock of tick {ordinal} was not the id the run submitted"
        );
    }
    assert_eq!(verse.now().await, 120);
}

#[tokio::test]
async fn an_open_tick_the_world_moved_past_is_closed_by_the_world() {
    let (_probe_dir, probe) = Harness::fresh().await;
    let measuring = ScriptedPort::speaking();
    probe.run_quietly(measuring.clone(), u64::MAX, 1).await;
    let first_bound = measuring.bounds()[0];

    let (_dir, verse) = Harness::fresh().await;
    let world = verse.mailbox.snapshot().await.unwrap().world_id;
    let ended = verse
        .run_under(
            ScriptedPort::speaking(),
            first_bound + 64,
            config(1),
            |_| ControlFlow::Continue(()),
        )
        .await;
    assert_eq!(ended.record.status, RunStatus::BudgetExhausted);
    assert_eq!(verse.acts().await, 1);
    // Another writer moves time without closing the tick.
    verse
        .mailbox
        .submit_clock(CommandId::new(), TickMinutes::new(SPAN).unwrap())
        .await
        .unwrap();
    assert_eq!(verse.now().await, 60);

    let resumed = verse
        .run_quietly(ScriptedPort::speaking(), u64::MAX, 1)
        .await;
    assert_eq!(resumed.record.status, RunStatus::Completed);
    let abandoned = verse.runs.tick(world, 0).unwrap().unwrap();
    assert_eq!(abandoned.closed, Some(TickClose::World));
    let next = verse.runs.tick(world, 1).unwrap().unwrap();
    assert_eq!(next.opened_by_run, 2);
    assert_eq!(next.closed, Some(TickClose::Clock { run: 2 }));
    let [checkpoint] = resumed.record.checkpoints.as_slice() else {
        panic!("one tick, one checkpoint")
    };
    assert_eq!(checkpoint.tick, 1);
    assert_eq!(checkpoint.committed.len(), 2, "a fresh cover ran both factions");
    assert_eq!(verse.acts().await, 3);
    assert_eq!(verse.now().await, 120);
    // No faction has two acts in one ordinal: ordinal 1 opened after run 1's
    // act, so its acts are the ones past the tick's opening revision.
    let log = verse.mailbox.operator_log().await.unwrap();
    for speaker in log.iter().map(|event| event.speaker) {
        let in_first = log
            .iter()
            .filter(|event| event.speaker == speaker && event.revision <= next.opened_revision)
            .count();
        let in_second = log
            .iter()
            .filter(|event| event.speaker == speaker && event.revision > next.opened_revision)
            .count();
        assert!(in_first <= 1 && in_second <= 1);
    }
}

fn tick_doc(world: WorldId, ordinal: u64) -> VerseTick {
    VerseTick {
        world_id: world,
        ordinal,
        opened_now: 0,
        opened_revision: 3,
        span_minutes: SPAN,
        cover_budget: RecordedCoverBudget {
            cells: 2,
            constituent_cap: 8,
            urgency_slots: 0,
        },
        oversubscribed: false,
        cells: Vec::new(),
        opened_by_run: 1,
        closed: None,
    }
}

#[test]
fn the_tick_store_opens_once_closes_once_and_refuses_non_canonical_rows() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("runs.cc");
    let store = VerseRunStore::open(&path).unwrap();
    let world = WorldId::nil_for_test();
    let other = WorldId::issue();

    store.open_tick(&tick_doc(world, 0)).unwrap();
    assert!(matches!(
        store.open_tick(&tick_doc(world, 0)),
        Err(VerseRunError::OutOfSequence)
    ));
    // A tick opened already closed is not an open.
    let mut born_closed = tick_doc(world, 5);
    born_closed.closed = Some(TickClose::World);
    assert!(matches!(
        store.open_tick(&born_closed),
        Err(VerseRunError::OutOfSequence)
    ));
    assert!(matches!(
        store.close_tick(world, 9, TickClose::World),
        Err(VerseRunError::OutOfSequence)
    ));
    store
        .close_tick(world, 0, TickClose::Clock { run: 4 })
        .unwrap();
    assert!(matches!(
        store.close_tick(world, 0, TickClose::World),
        Err(VerseRunError::OutOfSequence)
    ));

    // The world closes the open ticks below an ordinal and no others.
    for ordinal in 1..4 {
        store.open_tick(&tick_doc(world, ordinal)).unwrap();
    }
    store.open_tick(&tick_doc(other, 1)).unwrap();
    store.close_ticks_before(world, 3).unwrap();
    let closed_by = |store: &VerseRunStore, world, ordinal| {
        store.tick(world, ordinal).unwrap().unwrap().closed
    };
    assert_eq!(closed_by(&store, world, 0), Some(TickClose::Clock { run: 4 }));
    assert_eq!(closed_by(&store, world, 1), Some(TickClose::World));
    assert_eq!(closed_by(&store, world, 2), Some(TickClose::World));
    assert_eq!(closed_by(&store, world, 3), None);
    assert_eq!(closed_by(&store, other, 1), None);
    assert!(store.tick(world, 7).unwrap().is_none());

    // Tick documents and run records share the file and reopen intact.
    store
        .store
        .lock()
        .unwrap()
        .push(&run_row(&record(world, 1)).unwrap())
        .unwrap();
    drop(store);
    let store = VerseRunStore::open(&path).unwrap();
    assert_eq!(store.runs(world).unwrap(), vec![record(world, 1)]);
    assert_eq!(closed_by(&store, world, 2), Some(TickClose::World));

    let good = tick_doc(world, 0);
    let envelope = |key: String, row_type: &str, schema: &str, payload: Vec<u8>| CultCacheEnvelope {
        key,
        r#type: row_type.into(),
        payload,
        stored_at: Utc::now().to_rfc3339(),
        schema_id: Some(schema.into()),
    };
    let named = rmp_serde::to_vec_named(&good).unwrap();
    let cases = [
        envelope(tick_key(world, 9), TICK_ROW, TICK_RECORD_SCHEMA, named.clone()),
        envelope(tick_key(world, 0), TICK_ROW, "ghostlight.verse_tick.v2", named.clone()),
        envelope(tick_key(world, 0), TICK_ROW, TICK_RECORD_SCHEMA, vec![0xc0]),
        envelope(
            tick_key(world, 0),
            TICK_ROW,
            TICK_RECORD_SCHEMA,
            rmp_serde::to_vec(&good).unwrap(),
        ),
        envelope(tick_key(world, 0), "something_else.v1", TICK_RECORD_SCHEMA, named),
    ];
    for (index, row) in cases.into_iter().enumerate() {
        let file = dir.path().join(format!("tick-case-{index}.cc"));
        {
            let mut raw = OwnedRedbMessagePackBackingStore::new(&file).unwrap();
            raw.push(&row).unwrap();
        }
        assert!(
            matches!(VerseRunStore::open(&file), Err(VerseRunError::Corrupt)),
            "tick case {index} was accepted"
        );
    }
}

/// A restored world keeps its run store: the closed tick document at the
/// world's own ordinal is refused before any provider call or act.
#[tokio::test]
async fn a_closed_tick_at_the_worlds_current_ordinal_is_refused_before_any_cell() {
    let live = tempfile::tempdir().unwrap();
    create_world(live.path()).await;
    let backup = live.path().join("world.backup");
    std::fs::copy(live.path().join("world.cc"), &backup).unwrap();
    {
        let verse = Harness::open(live.path()).await;
        let ran = verse
            .run_quietly_under(ScriptedPort::speaking(), config_of(2, 60, 2))
            .await;
        assert_eq!(ran.record.status, RunStatus::Completed);
        let Harness {
            mailbox,
            _task,
            calls,
            runs,
            work,
        } = verse;
        drop((calls, runs, work, mailbox));
        _task.await.unwrap();
    }
    let restored = tempfile::tempdir().unwrap();
    std::fs::copy(&backup, restored.path().join("world.cc")).unwrap();
    std::fs::copy(live.path().join("runs.cc"), restored.path().join("runs.cc")).unwrap();
    let verse = Harness::open(restored.path()).await;
    let world = verse.mailbox.snapshot().await.unwrap().world_id;
    assert_eq!(verse.mailbox.clock_advances().await.unwrap(), 0);
    assert!(verse.runs.tick(world, 0).unwrap().unwrap().closed.is_some());
    let acts = verse.acts().await;

    let port = ScriptedPort::speaking();
    let refused = verse.run_quietly_under(port.clone(), config_of(1, 60, 2)).await;
    assert_eq!(
        refused.record.status,
        RunStatus::Failed {
            reason: "run_store".into()
        }
    );
    assert_eq!(port.calls(), 0, "no provider call before the refusal");
    assert_eq!(verse.acts().await, acts);
    assert_eq!(verse.now().await, 0);
}

/// A port that moves the world's clock through another writer on its first
/// call, then answers like the wrapped port.
struct ClockMovedMid {
    inner: Arc<ScriptedPort>,
    mailbox: WorldMailbox,
    seen: AtomicUsize,
    minutes: u32,
}

#[async_trait]
impl InferencePort for ClockMovedMid {
    fn prepare(&self, request: InferenceRequest) -> Result<PreparedInference, InferenceFault> {
        self.inner.prepare(request)
    }

    async fn infer(&self, request: PreparedInference) -> Result<InferenceOutput, InferenceFault> {
        if self.seen.fetch_add(1, Ordering::SeqCst) == 0 {
            self.mailbox
                .submit_clock(CommandId::new(), TickMinutes::new(self.minutes).unwrap())
                .await
                .unwrap();
        }
        self.inner.infer(request).await
    }
}

/// The ordinal counts every clock advance the world committed, whatever its
/// length: a 45-minute advance by another writer inside an open tick moves
/// it, so the next tick opens at ordinal 2 and ordinal 1 never has a document.
#[tokio::test]
async fn every_clock_advance_moves_the_ordinal_whatever_its_length() {
    let (_dir, verse) = Harness::fresh().await;
    let world = verse.mailbox.snapshot().await.unwrap().world_id;
    let port = Arc::new(ClockMovedMid {
        inner: ScriptedPort::speaking(),
        mailbox: verse.mailbox.clone(),
        seen: AtomicUsize::new(0),
        minutes: 45,
    });
    let first = verse.run_quietly_under(port, config_of(1, 60, 2)).await;
    assert_eq!(first.record.status, RunStatus::Completed);
    assert_eq!(verse.mailbox.clock_advances().await.unwrap(), 2);
    assert_eq!(
        verse.runs.tick(world, 0).unwrap().unwrap().closed,
        Some(TickClose::Clock { run: 1 })
    );

    let second = verse
        .run_quietly_under(ScriptedPort::speaking(), config_of(1, 60, 2))
        .await;
    assert_eq!(second.record.status, RunStatus::Completed);
    assert_eq!(second.record.checkpoints[0].tick, 2);
    assert!(verse.runs.tick(world, 1).unwrap().is_none());
    assert_eq!(verse.now().await, 45 + 60 + 60);
}

/// Two opens of one ordinal cannot both succeed, even raced.
#[test]
fn two_racing_opens_of_one_tick_cannot_both_succeed() {
    let dir = tempfile::tempdir().unwrap();
    let store = VerseRunStore::open(dir.path().join("runs.cc")).unwrap();
    let world = WorldId::nil_for_test();
    for ordinal in 0..100u64 {
        let barrier = std::sync::Barrier::new(2);
        let opened = std::thread::scope(|scope| {
            let handles: Vec<_> = (1..=2u64)
                .map(|run| {
                    let (store, barrier) = (&store, &barrier);
                    scope.spawn(move || {
                        let mut doc = tick_doc(world, ordinal);
                        doc.opened_by_run = run;
                        barrier.wait();
                        store.open_tick(&doc).is_ok()
                    })
                })
                .collect();
            handles
                .into_iter()
                .map(|handle| handle.join().unwrap())
                .filter(|ok| *ok)
                .count()
        });
        assert_eq!(opened, 1, "ordinal {ordinal} was opened {opened} times");
    }
}
