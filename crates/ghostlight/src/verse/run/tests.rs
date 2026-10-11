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

    /// One run over `inner`, recorded under the next sequence.
    async fn run(
        &self,
        inner: Arc<dyn InferencePort>,
        cap: u64,
        ticks: u32,
        observe: impl FnMut(&TickReport) -> ControlFlow<()>,
    ) -> RunOutcome {
        let world = self.mailbox.snapshot().await.unwrap();
        let sequence = self.runs.next_sequence(world.world_id).unwrap();
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
            &config(ticks),
            sequence,
            observe,
        )
        .await
        .unwrap()
    }

    async fn run_quietly(&self, inner: Arc<dyn InferencePort>, cap: u64, ticks: u32) -> RunOutcome {
        self.run(inner, cap, ticks, |_| ControlFlow::Continue(()))
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
    let opportunities = verse.mailbox.opportunities().await.unwrap();
    let graph = verse.mailbox.agency_graph().await.unwrap();
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
    assert_eq!(verse.now().await, 3 * u64::from(SPAN));
    // Both factions acted every tick, the second on a scope the first moved.
    assert_eq!(verse.acts().await, 6);
    assert_eq!(port.calls(), 6);

    // Every committed command id is the one the cover derives for that cell,
    // in cell order.
    for checkpoint in &outcome.record.checkpoints {
        let cover = derive_cover(
            world.world_id,
            FictionalMinutes(checkpoint.tick * u64::from(SPAN)),
            SPAN,
            &opportunities,
            &graph,
            two_cells(),
        );
        let expected: Vec<CommandId> = cover
            .cells
            .iter()
            .map(|cell| match cell {
                Cell::Singleton { id, tick, member } => {
                    CommandId::for_cell_constituent(world.world_id, *id, member.subject, *tick)
                }
                Cell::Group { .. } => panic!("two factions under two cells are singletons"),
            })
            .collect();
        assert_eq!(expected.len(), 2);
        assert_eq!(checkpoint.committed, expected);
        assert_eq!(checkpoint.cells, 2);
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
        assert!(report.closed);
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
    assert!(ended.record.checkpoints.is_empty());
    assert_eq!(verse.now().await, 0, "the open tick's clock did not advance");
    assert_eq!(first.calls(), 1, "the second call never reached the provider");
    assert_eq!(verse.acts().await, 1);
    // The open tick is reported, unclosed, with what it committed.
    assert_eq!(reports.len(), 1);
    assert!(!reports[0].closed);
    assert_eq!(reports[0].acts.len(), 1);
    assert_eq!(reports[0].checkpoint.committed.len(), 1);
    assert!(reports[0].remaining < first_bound);
    assert_eq!(
        verse.runs.runs(world.world_id).unwrap(),
        vec![ended.record.clone()]
    );

    // A later run, under its own cap, finishes the tick: the first faction's
    // act is answered from the kernel's ledger and costs no call.
    let second = ScriptedPort::speaking();
    let resumed = verse.run_quietly(second.clone(), u64::MAX, 2).await;
    assert_eq!(resumed.record.status, RunStatus::Completed);
    assert_eq!(resumed.record.sequence, 2);
    let ticks: Vec<u64> = resumed.record.checkpoints.iter().map(|c| c.tick).collect();
    assert_eq!(ticks, vec![0, 1], "the open tick is the first this run closes");
    assert_eq!(verse.now().await, 2 * u64::from(SPAN));
    assert_eq!(second.calls(), 3, "one call finished tick 0, two ran tick 1");
    assert_eq!(verse.acts().await, 4, "no act was committed twice");
    // The tick that was open committed both of its cells' commands.
    assert_eq!(resumed.record.checkpoints[0].committed.len(), 2);
    assert_eq!(
        resumed.record.checkpoints[0].committed[0],
        reports[0].checkpoint.committed[0]
    );
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
    assert!(ended.record.checkpoints.is_empty());
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
    assert!(outcome.record.checkpoints.is_empty());
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
            ],
            tokens: 7,
        },
        acts: Vec::new(),
        spent: 40,
        remaining: 60,
        closed: false,
    };
    assert_eq!(
        report.to_string(),
        "tick 4 open: 3 cells, 0 acts, 3 refused
  refused: cell 1 world_rejected
  refused: cell 2 opportunity_gone
  refused: cell 0 interrupted
  tokens: 7 this tick, 40 spent, 60 remaining
"
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
    assert!(text.starts_with("tick 0 closed: 2 cells, 2 acts, 0 refused
"), "{text}");
    assert!(text.contains(&format!(
        "
  Adrasteia attack [{}] target=Brannoch says \"Adrasteia strikes.\"
",
        name("attack")
    )), "{text}");
    assert!(text.contains(&format!(
        "
  Brannoch speak [{}] says \"Brannoch answers.\"
",
        name("speak")
    )), "{text}");
    assert!(text.ends_with(&format!(
        "  tokens: {} this tick, {} spent, {} remaining
",
        checkpoint.tokens, reports[0].spent, reports[0].remaining
    )), "{text}");
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
        FictionalMinutes(0),
        SPAN,
        &opportunities,
        &graph,
        two_cells(),
    );
    let group = derive_cover(
        world,
        FictionalMinutes(0),
        SPAN,
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
        let Some(Cell::Singleton { member, .. }) = refreshed(cell, &moved) else {
            panic!("a subject that still has an opportunity keeps its cell")
        };
        assert_ne!(member.opportunity, held.opportunity);
        assert!(moved.contains(&member.opportunity));
    }
    let Some(Cell::Group { members: now, .. }) = refreshed(grouped, &moved) else {
        panic!("a group keeps its members")
    };
    assert_eq!(now.len(), 2);
    assert!(now.iter().all(|member| moved.contains(&member.opportunity)));

    // A subject with no opportunity leaves its cell, and an empty cell is gone.
    let one = &opportunities[..1];
    let Some(Cell::Group { members: left, .. }) = refreshed(grouped, one) else {
        panic!("one member remains")
    };
    assert_eq!(left.len(), 1);
    assert_eq!(left[0].opportunity.scope, one[0].scope);
    let gone = singles
        .cells
        .iter()
        .filter(|cell| refreshed(cell, one).is_none())
        .count();
    assert_eq!(gone, 1);
    assert!(refreshed(grouped, &[]).is_none());
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
        ControllerError::OpportunityMismatch,
    ] {
        assert!(matches!(
            classify(&gone),
            CellEnd::Refused(RefusalReason::OpportunityGone)
        ));
    }
    assert!(matches!(
        classify(&ControllerError::CommandMismatch),
        CellEnd::Fatal("controller_error")
    ));
}

#[tokio::test]
async fn a_port_opened_for_another_run_is_refused_before_anything_is_written() {
    let (_dir, verse) = Harness::fresh().await;
    let world = verse.mailbox.snapshot().await.unwrap();
    let port = Arc::new(
        RecordingInferencePort::open(
            ScriptedPort::speaking(),
            verse.calls.clone(),
            call_run_id(7),
            1_000,
            &RunStart::of(&world),
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
    let error = run_manual(
        &verse.mailbox,
        &runner,
        &port,
        &verse.runs,
        &config(1),
        1,
        |_| ControlFlow::Continue(()),
    )
    .await
    .unwrap_err();
    assert!(matches!(error, VerseRunError::PortNotThisRun));
    assert!(verse.runs.runs(world.world_id).unwrap().is_empty());

    // A cover budget with no cells is refused before the record exists.
    let mut bad = config(1);
    bad.cover.cells = 0;
    let own = Arc::new(
        RecordingInferencePort::open(
            ScriptedPort::speaking(),
            verse.calls.clone(),
            call_run_id(1),
            1_000,
            &RunStart::of(&world),
        )
        .unwrap(),
    );
    let error = run_manual(&verse.mailbox, &runner, &own, &verse.runs, &bad, 1, |_| {
        ControlFlow::Continue(())
    })
    .await
    .unwrap_err();
    assert!(matches!(
        error,
        VerseRunError::Budget(CoverBudgetError::NoCells)
    ));
    assert!(verse.runs.runs(world.world_id).unwrap().is_empty());
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

#[test]
fn the_run_store_numbers_runs_per_world_and_replaces_a_run_with_its_later_state() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("runs.cc");
    let store = VerseRunStore::open(&path).unwrap();
    let one = WorldId::nil_for_test();
    let other = WorldId::issue();
    assert_eq!(store.next_sequence(one).unwrap(), 1);

    // A run can only begin as the next in its world.
    assert!(matches!(
        store.save(&record(one, 2)),
        Err(VerseRunError::OutOfSequence)
    ));
    assert!(matches!(
        store.save(&record(one, 0)),
        Err(VerseRunError::OutOfSequence)
    ));
    store.save(&record(one, 1)).unwrap();
    assert_eq!(store.next_sequence(one).unwrap(), 2);
    assert_eq!(
        store.next_sequence(other).unwrap(),
        1,
        "sequences are per world"
    );

    // Saving a run again replaces it, and the file reopens to the same rows.
    let mut finished = record(one, 1);
    finished.status = RunStatus::Completed;
    store.save(&finished).unwrap();
    store.save(&record(one, 2)).unwrap();
    store.save(&record(other, 1)).unwrap();
    drop(store);
    let store = VerseRunStore::open(&path).unwrap();
    assert_eq!(store.runs(one).unwrap(), vec![finished, record(one, 2)]);
    assert_eq!(store.runs(other).unwrap(), vec![record(other, 1)]);
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
