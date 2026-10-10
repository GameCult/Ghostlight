use super::*;
use crate::CommandId;
use crate::controllers::{
    InferenceFaultClass, InferenceFaultDisposition, RequestShape, tool_request,
};
use codex_connector::{CodexInputItem, CodexToolDefinition};
use std::sync::atomic::{AtomicUsize, Ordering};

const MODEL: &str = "hosted/test-model";

type Script = Vec<Result<InferenceOutput, InferenceFault>>;

/// A provider stand-in: answers from a script, counts the calls that
/// reached it, and can hold a call open until released.
struct ScriptedPort {
    replies: Mutex<VecDeque<Result<InferenceOutput, InferenceFault>>>,
    calls: AtomicUsize,
    lent: AtomicUsize,
    gate: Option<Arc<tokio::sync::Semaphore>>,
}

impl ScriptedPort {
    fn new(replies: Script) -> Arc<Self> {
        Arc::new(Self {
            replies: Mutex::new(replies.into()),
            calls: AtomicUsize::new(0),
            lent: AtomicUsize::new(0),
            gate: None,
        })
    }

    fn gated(replies: Script, gate: Arc<tokio::sync::Semaphore>) -> Arc<Self> {
        Arc::new(Self {
            replies: Mutex::new(replies.into()),
            calls: AtomicUsize::new(0),
            lent: AtomicUsize::new(0),
            gate: Some(gate),
        })
    }

    fn calls(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }
}

#[async_trait]
impl InferencePort for ScriptedPort {
    fn prepare(&self, request: InferenceRequest) -> Result<PreparedInference, InferenceFault> {
        PreparedInference::prepare("verse-test", 4_102_444_800_000, request)
    }

    fn lend_tool_results(&self, _: &PreparedInference, _: Box<dyn ToolResultOracle>) {
        self.lent.fetch_add(1, Ordering::SeqCst);
    }

    async fn infer(&self, _: PreparedInference) -> Result<InferenceOutput, InferenceFault> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        if let Some(gate) = &self.gate {
            gate.acquire().await.expect("the gate stays open").forget();
        }
        self.replies
            .lock()
            .unwrap()
            .pop_front()
            .expect("the script has a reply for every call that reaches it")
    }
}

fn request_for(command: CommandId, prompt: &str, max_output_tokens: u32) -> InferenceRequest {
    tool_request(
        command,
        0,
        InferencePurpose::Persona,
        MODEL,
        "Respond only in prose.",
        vec![CodexInputItem::UserText {
            text: prompt.into(),
        }],
        Vec::<CodexToolDefinition>::new(),
        RequestShape {
            max_output_tokens,
            parallel_tool_calls: false,
        },
    )
    .expect("the request builds")
}

fn prepared_for(prompt: &str, max_output_tokens: u32) -> PreparedInference {
    PreparedInference::prepare(
        "verse-test",
        4_102_444_800_000,
        request_for(CommandId::new(), prompt, max_output_tokens),
    )
    .expect("the request prepares")
}

/// What the spec defines the bound as, computed from the prepared request.
fn bound_of(prepared: &PreparedInference) -> u64 {
    let payload = serde_json::to_vec(&prepared.invocation.request)
        .expect("a provider request serializes")
        .len() as u64;
    payload
        + u64::from(
            prepared
                .invocation
                .request
                .max_output_tokens
                .expect("an allowance"),
        )
}

fn reply(text: &str, receipt: &str) -> InferenceOutput {
    InferenceOutput::new(vec![InferenceEvent::Text(text.into())], receipt)
}

fn usage(prompt: u64, completion: u64, cached_prompt: Option<u64>) -> TokenUsage {
    TokenUsage {
        prompt,
        completion,
        cached_prompt,
    }
}

fn store_in(directory: &tempfile::TempDir) -> (Arc<CallRecordStore>, std::path::PathBuf) {
    let path = directory.path().join("calls.redb");
    (
        Arc::new(CallRecordStore::open(&path).expect("the store opens")),
        path,
    )
}

/// A world identity and digest for tests that need a run to start somewhere.
fn start() -> RunStart {
    RunStart {
        world_id: "world-a".into(),
        state_digest: "sha256:a".into(),
    }
}

fn recording(
    inner: &Arc<ScriptedPort>,
    store: &Arc<CallRecordStore>,
    run: &str,
    cap: u64,
) -> RecordingInferencePort {
    RecordingInferencePort::open(
        inner.clone(),
        store.clone(),
        RunId::new(run).unwrap(),
        cap,
        &start(),
    )
    .expect("the port opens")
}

fn json(output: &InferenceOutput) -> Vec<u8> {
    serde_json::to_vec(output).expect("an output serializes")
}

fn ok_all(outputs: &[InferenceOutput]) -> Script {
    outputs.iter().cloned().map(Ok).collect()
}

#[tokio::test]
async fn recording_port_stores_each_call_before_returning() {
    let directory = tempfile::tempdir().unwrap();
    let (store, path) = store_in(&directory);
    let run = RunId::new("run-a").unwrap();
    let outputs = vec![
        reply("  first\nreply \u{e9}\u{1f9ff} ", "receipt-1").with_usage(usage(100, 20, Some(40))),
        InferenceOutput::new(
            vec![
                InferenceEvent::Text("a".into()),
                InferenceEvent::ToolCall {
                    call_id: "c1".into(),
                    name: "speak".into(),
                    arguments: "{\"b\":1, \"a\":[1,2]}".into(),
                },
            ],
            "receipt-2",
        ),
        InferenceOutput::new(Vec::new(), "receipt-3").with_usage(usage(7, 0, None)),
    ];
    let inner = ScriptedPort::new(ok_all(&outputs));
    let port = recording(&inner, &store, "run-a", u64::MAX);
    for (index, expected) in outputs.iter().enumerate() {
        let prepared = port
            .prepare(request_for(CommandId::new(), "tell", 1_200))
            .unwrap();
        let digest = content_digest(&prepared.invocation.request).unwrap();
        let returned = port.infer(prepared).await.unwrap();
        assert_eq!(json(&returned), json(expected));
        let stored = store.records(&run).unwrap();
        assert_eq!(
            stored.len(),
            index + 1,
            "the call was stored before it returned"
        );
        let record = &stored[index];
        assert_eq!(record.sequence(), index as u64 + 1);
        assert_eq!(record.run_id(), "run-a");
        assert_eq!(record.content_sha256(), digest);
        assert_eq!(record.model(), MODEL);
        assert_eq!(record.purpose(), InferencePurpose::Persona);
        assert_eq!(record.usage(), expected.usage);
        assert_eq!(json(&record.output()), json(expected), "byte for byte");
    }
    // Prompt plus completion; the cached figure is part of the prompt.
    let charges: Vec<u64> = store
        .records(&run)
        .unwrap()
        .iter()
        .map(|record| record.charged())
        .collect();
    assert_eq!(charges[0], 120);
    assert_eq!(charges[2], 7);
    assert_eq!(port.spent(), charges.iter().sum::<u64>());

    // Another run keys its own sequence from 1.
    let other = recording(&inner, &store, "run-b", u64::MAX);
    inner
        .replies
        .lock()
        .unwrap()
        .push_back(Ok(reply("b", "receipt-b")));
    other
        .infer(
            other
                .prepare(request_for(CommandId::new(), "tell", 1_200))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        store.records(&RunId::new("run-b").unwrap()).unwrap()[0].sequence(),
        1
    );

    // The records are in the file, not in the port.
    let before = store.records(&run).unwrap();
    drop((port, other, store));
    let reopened = CallRecordStore::open(&path).unwrap();
    assert_eq!(reopened.records(&run).unwrap(), before);
}

#[tokio::test]
async fn cap_refuses_before_the_provider_is_called() {
    let directory = tempfile::tempdir().unwrap();
    let (store, _) = store_in(&directory);
    let probe = prepared_for("probe", 1_200);
    let bound = bound_of(&probe);
    assert!(bound > 1_200, "the bound includes the request's bytes");

    let inner = ScriptedPort::new(vec![Ok(reply("never", "r"))]);
    let port = recording(&inner, &store, "run-cap", bound - 1);
    let fault = port.infer(probe.clone()).await.expect_err("over the cap");
    assert_eq!(fault.class(), InferenceFaultClass::BudgetExhausted);
    assert_eq!(
        fault.disposition(),
        InferenceFaultDisposition::IntegrityViolation
    );
    assert_eq!(inner.calls(), 0, "the provider was never invoked");
    assert!(
        store
            .records(&RunId::new("run-cap").unwrap())
            .unwrap()
            .is_empty()
    );
    assert_eq!(port.spent(), 0);
    drop(port);

    // Exactly the cap is allowed.
    let exact = recording(&inner, &store, "run-exact-cap", bound);
    exact
        .infer(probe)
        .await
        .expect("a call that fits the cap is made");
    assert_eq!(inner.calls(), 1);

    // A request with no output allowance has no bound and is refused.
    let mut open_ended = prepared_for("open ended", 1_200);
    open_ended.invocation.request.max_output_tokens = None;
    let unbounded = recording(&inner, &store, "run-open", u64::MAX);
    let fault = unbounded
        .infer(open_ended)
        .await
        .expect_err("no allowance, no bound");
    assert_eq!(fault.class(), InferenceFaultClass::BudgetExhausted);
    assert_eq!(inner.calls(), 1);
}

#[tokio::test]
async fn spend_never_exceeds_cap() {
    let directory = tempfile::tempdir().unwrap();
    let (store, _) = store_in(&directory);
    let run = RunId::new("run-spend").unwrap();
    let prompt = "a moderately long prompt for the sample call";
    let bound = bound_of(&prepared_for(prompt, 600));
    let cap = bound * 5 / 2;
    let script: Script = (0..40)
        .map(|n| Ok(reply("ok", &format!("r{n}")).with_usage(usage(bound / 4, bound / 8, None))))
        .collect();
    let inner = ScriptedPort::new(script);
    let port = recording(&inner, &store, "run-spend", cap);
    let mut made = 0;
    loop {
        match port.infer(prepared_for(prompt, 600)).await {
            Ok(_) => made += 1,
            Err(fault) => {
                assert_eq!(fault.class(), InferenceFaultClass::BudgetExhausted);
                break;
            }
        }
        assert!(made < 40, "the cap never refused");
    }
    assert!(made >= 2, "the cap let some calls through: {made}");
    assert_eq!(
        inner.calls(),
        made,
        "a refused call never reached the provider"
    );
    let total: u64 = store
        .records(&run)
        .unwrap()
        .iter()
        .map(|record| record.charged())
        .sum();
    assert!(total <= cap, "{total} > {cap}");
    assert_eq!(total, port.spent());
    // The refusal is the cap's doing: another call would not have fit.
    assert!(total + bound > cap);
}

#[tokio::test]
async fn calls_on_the_wire_hold_their_bound() {
    let directory = tempfile::tempdir().unwrap();
    let (store, _) = store_in(&directory);
    let bound = bound_of(&prepared_for("held", 1_200));
    let gate = Arc::new(tokio::sync::Semaphore::new(0));
    let inner = ScriptedPort::gated(vec![Ok(reply("one", "r1"))], gate.clone());
    // Room for one call's bound and not two.
    let port = Arc::new(recording(&inner, &store, "run-flight", bound + bound / 2));
    let first = {
        let port = port.clone();
        tokio::spawn(async move { port.infer(prepared_for("held", 1_200)).await })
    };
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        while inner.calls() == 0 {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("the first call reached the provider");
    // A second call that was let through would wait on the closed gate.
    let second = tokio::time::timeout(
        std::time::Duration::from_secs(5),
        port.infer(prepared_for("held", 1_200)),
    )
    .await
    .expect("the second call was refused, not left waiting on the provider");
    let fault = second.expect_err("the first call's bound is still held");
    assert_eq!(fault.class(), InferenceFaultClass::BudgetExhausted);
    assert_eq!(inner.calls(), 1);
    gate.add_permits(1);
    first.await.unwrap().expect("the first call completes");
}

struct NoTools;

impl ToolResultOracle for NoTools {
    fn remaining_rounds(&self) -> u32 {
        0
    }

    fn answer(&mut self, _: &str, _: &str) -> Result<String, crate::ControllerError> {
        Ok(String::new())
    }
}

#[test]
fn tool_results_are_lent_to_the_port_underneath() {
    let directory = tempfile::tempdir().unwrap();
    let (store, _) = store_in(&directory);
    let inner = ScriptedPort::new(Vec::new());
    let port = recording(&inner, &store, "run-lend", u64::MAX);
    port.lend_tool_results(&prepared_for("lend", 1_200), Box::new(NoTools));
    assert_eq!(inner.lent.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn a_reply_with_no_usage_is_charged_its_bound() {
    let directory = tempfile::tempdir().unwrap();
    let (store, _) = store_in(&directory);
    let prepared = prepared_for("no usage reported", 1_200);
    let bound = bound_of(&prepared);
    let inner = ScriptedPort::new(vec![Ok(reply("silent", "r"))]);
    let port = recording(&inner, &store, "run-silent", u64::MAX);
    port.infer(prepared).await.unwrap();
    let record = store
        .records(&RunId::new("run-silent").unwrap())
        .unwrap()
        .remove(0);
    assert_eq!(record.usage(), None);
    assert_eq!(record.charged(), bound);
    assert_eq!(port.spent(), bound);
}

#[tokio::test]
async fn a_fault_keeps_its_reservation_charged() {
    let directory = tempfile::tempdir().unwrap();
    let (store, _) = store_in(&directory);
    let prepared = prepared_for("fault", 1_200);
    let bound = bound_of(&prepared);
    let inner = ScriptedPort::new(vec![
        Err(InferenceFault::retryable("down").classed(InferenceFaultClass::Connect)),
        Err(InferenceFault::retryable("slow").classed(InferenceFaultClass::Timeout)),
        Err(InferenceFault::new("junk").classed(InferenceFaultClass::BadReply)),
    ]);
    let port = recording(&inner, &store, "run-fault", u64::MAX);
    for (index, class) in [
        InferenceFaultClass::Connect,
        InferenceFaultClass::Timeout,
        InferenceFaultClass::BadReply,
    ]
    .into_iter()
    .enumerate()
    {
        let fault = port.infer(prepared.clone()).await.expect_err("a fault");
        assert_eq!(fault.class(), class, "the fault passes through");
        assert_eq!(port.spent(), bound * (index as u64 + 1));
    }
    assert!(
        store
            .records(&RunId::new("run-fault").unwrap())
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn faulting_calls_across_restarts_never_pass_the_cap() {
    let directory = tempfile::tempdir().unwrap();
    let (store, path) = store_in(&directory);
    let prepared = prepared_for("loop", 1_200);
    let bound = bound_of(&prepared);
    let cap = bound * 3 + bound / 2;
    drop(store);
    let mut reached_the_provider = 0;
    for attempt in 0..12 {
        let store = Arc::new(CallRecordStore::open(&path).unwrap());
        let class = if attempt % 2 == 0 {
            InferenceFaultClass::BadReply
        } else {
            InferenceFaultClass::Timeout
        };
        let inner = ScriptedPort::new(vec![Err(InferenceFault::new("down").classed(class))]);
        let port = recording(&inner, &store, "run-loop", cap);
        let fault = port.infer(prepared.clone()).await.expect_err("a fault");
        reached_the_provider += inner.calls();
        if inner.calls() == 0 {
            assert_eq!(fault.class(), InferenceFaultClass::BudgetExhausted);
        }
        assert!(port.spent() <= cap, "{} > {cap}", port.spent());
    }
    assert_eq!(reached_the_provider, 3, "a restart returns no budget");
}

#[tokio::test]
async fn a_call_abandoned_on_the_wire_stays_charged_after_a_restart() {
    let directory = tempfile::tempdir().unwrap();
    let (store, path) = store_in(&directory);
    let prepared = prepared_for("crash", 1_200);
    let bound = bound_of(&prepared);
    let gate = Arc::new(tokio::sync::Semaphore::new(0));
    let inner = ScriptedPort::gated(vec![Ok(reply("never", "r"))], gate);
    let port = Arc::new(recording(&inner, &store, "run-crash", bound));
    let call = {
        let port = port.clone();
        let prepared = prepared.clone();
        tokio::spawn(async move { port.infer(prepared).await })
    };
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        while inner.calls() == 0 {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("the call reached the provider");
    call.abort();
    let _ = call.await;
    drop((port, store));

    let store = Arc::new(CallRecordStore::open(&path).unwrap());
    let inner = ScriptedPort::new(vec![Ok(reply("again", "r2"))]);
    let reopened = recording(&inner, &store, "run-crash", bound);
    assert_eq!(reopened.spent(), bound);
    let fault = reopened
        .infer(prepared)
        .await
        .expect_err("the cap is spent");
    assert_eq!(fault.class(), InferenceFaultClass::BudgetExhausted);
    assert_eq!(inner.calls(), 0);
}

#[tokio::test]
async fn one_run_has_one_ledger_and_one_owner() {
    let directory = tempfile::tempdir().unwrap();
    let (store, _) = store_in(&directory);
    let prepared = prepared_for("owner", 1_200);
    let bound = bound_of(&prepared);
    let cap = bound + bound / 2;
    let inner = ScriptedPort::new(vec![Ok(reply("a", "r1")), Ok(reply("b", "r2"))]);
    let first = recording(&inner, &store, "run-own", cap);
    let second = RecordingInferencePort::open(
        inner.clone(),
        store.clone(),
        RunId::new("run-own").unwrap(),
        cap,
        &start(),
    );
    assert_eq!(second.err(), Some(CallRecordError::RunInUse));
    // Another run is another ledger.
    drop(recording(&inner, &store, "run-other", cap));
    first.infer(prepared.clone()).await.unwrap();
    assert_eq!(first.spent(), bound);
    // The owner going away frees the run, and its spend carries over.
    drop(first);
    let next = recording(&inner, &store, "run-own", cap);
    assert_eq!(next.spent(), bound);
    next.infer(prepared)
        .await
        .expect_err("the cap holds across owners");
    assert_eq!(inner.calls(), 1);
}

#[tokio::test]
async fn usage_over_the_bound_is_charged_in_full_and_faults_the_call() {
    let directory = tempfile::tempdir().unwrap();
    let (store, _) = store_in(&directory);
    let prepared = prepared_for("liar", 1_200);
    let bound = bound_of(&prepared);
    let cap = bound + 200;
    let run = RunId::new("run-over").unwrap();
    let inner = ScriptedPort::new(vec![
        Ok(reply("big", "r1").with_usage(usage(bound, bound, None))),
        Ok(reply("never", "r2")),
    ]);
    let port = recording(&inner, &store, "run-over", cap);
    let fault = port.infer(prepared.clone()).await.expect_err("over bound");
    assert_eq!(fault.class(), InferenceFaultClass::BudgetExhausted);
    assert_eq!(
        fault.disposition(),
        InferenceFaultDisposition::IntegrityViolation
    );
    assert_eq!(port.spent(), bound * 2, "charged in full");
    assert!(store.records(&run).unwrap().is_empty());
    // The run is stopped: the next call is refused before the provider.
    let fault = port.infer(prepared).await.expect_err("the cap is passed");
    assert_eq!(fault.class(), InferenceFaultClass::BudgetExhausted);
    assert_eq!(inner.calls(), 1);
}

#[tokio::test]
async fn usage_exactly_at_the_bound_is_a_normal_reply() {
    let directory = tempfile::tempdir().unwrap();
    let (store, _) = store_in(&directory);
    let prepared = prepared_for("exact", 1_200);
    let bound = bound_of(&prepared);
    let inner = ScriptedPort::new(vec![Ok(reply("fits", "r1").with_usage(usage(
        bound - 10,
        10,
        None,
    )))]);
    let port = recording(&inner, &store, "run-exact", bound);
    port.infer(prepared).await.expect("a reply at its bound");
    assert_eq!(port.spent(), bound);
    assert_eq!(
        store
            .records(&RunId::new("run-exact").unwrap())
            .unwrap()
            .len(),
        1
    );
}

#[tokio::test]
async fn a_reopened_run_resumes_its_spend() {
    let directory = tempfile::tempdir().unwrap();
    let (store, path) = store_in(&directory);
    let probe = prepared_for("resume", 1_200);
    let bound = bound_of(&probe);
    let inner = ScriptedPort::new(vec![Ok(
        reply("one", "r1").with_usage(usage(bound, 0, None))
    )]);
    let first = recording(&inner, &store, "run-resume", bound * 2 - 1);
    first.infer(probe.clone()).await.unwrap();
    drop((first, store));

    let store = Arc::new(CallRecordStore::open(&path).unwrap());
    let inner = ScriptedPort::new(vec![Ok(reply("two", "r2"))]);
    let resumed = recording(&inner, &store, "run-resume", bound * 2 - 1);
    assert_eq!(resumed.spent(), bound);
    let fault = resumed
        .infer(probe)
        .await
        .expect_err("the earlier spend counts");
    assert_eq!(fault.class(), InferenceFaultClass::BudgetExhausted);
    assert_eq!(inner.calls(), 0);
}

#[tokio::test]
async fn replay_serves_stored_responses_and_refuses_strangers() {
    let directory = tempfile::tempdir().unwrap();
    let (store, _) = store_in(&directory);
    let run = RunId::new("run-replay").unwrap();
    let commands = [CommandId::new(), CommandId::new(), CommandId::new()];
    let outputs = [
        reply("one \u{e9}", "r1").with_usage(usage(5, 6, Some(1))),
        InferenceOutput::new(
            vec![InferenceEvent::ToolCall {
                call_id: "c".into(),
                name: "speak".into(),
                arguments: "{ \"x\" : 1 }".into(),
            }],
            "r2",
        ),
        reply("three", "r3"),
    ];
    let inner = ScriptedPort::new(ok_all(&outputs));
    let live = recording(&inner, &store, "run-replay", u64::MAX);
    for (command, prompt) in commands.iter().zip(["a", "b", "c"]) {
        let prepared = live.prepare(request_for(*command, prompt, 1_200)).unwrap();
        live.infer(prepared).await.unwrap();
    }
    let calls_made = inner.calls();

    let replay = ReplayInferencePort::new("verse-replay", &store, &run, &start()).unwrap();
    // A request the run never made is a fault and consumes nothing.
    let stranger = replay
        .prepare(request_for(CommandId::new(), "z", 1_200))
        .unwrap();
    let fault = replay.infer(stranger).await.expect_err("a stranger");
    assert_eq!(
        fault.disposition(),
        InferenceFaultDisposition::IntegrityViolation
    );
    // Out of order is a stranger too.
    let second_first = replay
        .prepare(request_for(commands[1], "b", 1_200))
        .unwrap();
    replay.infer(second_first).await.expect_err("out of order");
    // In order, each request gets exactly what the run received.
    for ((command, prompt), expected) in commands.iter().zip(["a", "b", "c"]).zip(&outputs) {
        let prepared = replay
            .prepare(request_for(*command, prompt, 1_200))
            .unwrap();
        let served = replay.infer(prepared).await.unwrap();
        assert_eq!(json(&served), json(expected));
    }
    let again = replay
        .prepare(request_for(commands[0], "a", 1_200))
        .unwrap();
    replay.infer(again).await.expect_err("the record is spent");
    assert_eq!(inner.calls(), calls_made, "replay made no provider call");
}

fn sample_record(run: &str, sequence: u64) -> ProviderCallRecord {
    ProviderCallRecord {
        run_id: run.into(),
        sequence,
        purpose: InferencePurpose::Persona,
        model: MODEL.into(),
        content_sha256: "ab".into(),
        events: vec![InferenceEvent::Text("t".into())],
        receipt_digest: "r".into(),
        usage: None,
        charged: 3,
    }
}

#[test]
fn replay_needs_a_run_the_store_began() {
    let directory = tempfile::tempdir().unwrap();
    let (store, _) = store_in(&directory);
    let run = RunId::new("never-begun").unwrap();
    assert_eq!(
        ReplayInferencePort::new("x", &store, &run, &start()).err(),
        Some(CallRecordError::NotReplayable)
    );
}

#[tokio::test]
async fn a_record_holds_no_request_text() {
    let directory = tempfile::tempdir().unwrap();
    let (store, path) = store_in(&directory);
    let canary = "REQUEST-CANARY-91c4";
    let inner = ScriptedPort::new(vec![Ok(reply("fine", "r"))]);
    let port = recording(&inner, &store, "run-secret", u64::MAX);
    port.infer(prepared_for(canary, 1_200)).await.unwrap();
    drop((port, store));
    let raw = OwnedRedbMessagePackBackingStore::new(&path)
        .unwrap()
        .pull_all()
        .unwrap();
    assert_eq!(raw.len(), 2, "one record and the run's ledger");
    for row in raw {
        let bytes = [row.key.as_bytes(), row.payload.as_slice()].concat();
        assert!(
            !bytes
                .windows(canary.len())
                .any(|window| window == canary.as_bytes()),
            "the request text reached the store"
        );
    }
    let refusal = InferenceFault::budget_exhausted();
    assert!(!format!("{refusal} {refusal:?}").contains(canary));
}

fn spend_row(run: &str, spent: u64) -> CultCacheEnvelope {
    envelope(
        RUN_LEDGER_ROW,
        Some(RUN_LEDGER_SCHEMA),
        run,
        rmp_serde::to_vec_named(&RunLedger {
            run_id: run.into(),
            cap: u64::MAX,
            start: start(),
            spent,
        })
        .unwrap(),
    )
}

fn envelope(
    row_type: &str,
    schema: Option<&str>,
    key: &str,
    payload: Vec<u8>,
) -> CultCacheEnvelope {
    CultCacheEnvelope {
        key: key.into(),
        r#type: row_type.into(),
        payload,
        stored_at: Utc::now().to_rfc3339(),
        schema_id: schema.map(str::to_owned),
    }
}

#[test]
fn the_store_refuses_any_row_that_is_not_a_canonical_v1_record() {
    let good = sample_record("run-x", 1);
    let payload = rmp_serde::to_vec_named(&good).unwrap();
    let key = row_key("run-x", 1);
    let mut loose = payload.clone();
    loose.push(0xc0);
    let v1 = |key: &str, payload: Vec<u8>| {
        envelope(CALL_RECORD_ROW, Some(CALL_RECORD_SCHEMA), key, payload)
    };
    let record_at = |sequence: u64| {
        v1(
            &row_key("run-x", sequence),
            rmp_serde::to_vec_named(&sample_record("run-x", sequence)).unwrap(),
        )
    };
    let cases = [
        (
            "earlier version",
            envelope(
                "verse_provider_call.v0",
                Some("ghostlight.verse_provider_call.v0"),
                &key,
                payload.clone(),
            ),
        ),
        (
            "later version",
            envelope(
                "verse_provider_call.v2",
                Some("ghostlight.verse_provider_call.v2"),
                &key,
                payload.clone(),
            ),
        ),
        (
            "another document",
            envelope(
                "controller_work.v18",
                Some("ghostlight.controller_work.v18"),
                &key,
                payload.clone(),
            ),
        ),
        (
            "no schema",
            envelope(CALL_RECORD_ROW, None, &key, payload.clone()),
        ),
        ("wrong key", v1("run-x/2", payload.clone())),
        ("not canonical", v1(&key, loose)),
        ("not a record", v1(&key, vec![1, 2, 3])),
        (
            "a run id that is not a key segment",
            v1(
                &row_key("bad/run", 1),
                rmp_serde::to_vec_named(&sample_record("bad/run", 1)).unwrap(),
            ),
        ),
        ("sequence zero", record_at(0)),
        ("a hole in the run", record_at(2)),
        ("a record with no ledger", record_at(1)),
        (
            "a ledger at the wrong key",
            envelope(
                RUN_LEDGER_ROW,
                Some(RUN_LEDGER_SCHEMA),
                "other",
                rmp_serde::to_vec_named(&RunLedger {
                    run_id: "run-x".into(),
                    cap: u64::MAX,
                    start: start(),
                    spent: 1,
                })
                .unwrap(),
            ),
        ),
        (
            "a ledger that is not canonical",
            envelope(
                RUN_LEDGER_ROW,
                Some(RUN_LEDGER_SCHEMA),
                "run-x",
                [
                    rmp_serde::to_vec_named(&RunLedger {
                        run_id: "run-x".into(),
                        cap: u64::MAX,
                        start: start(),
                        spent: 1,
                    })
                    .unwrap(),
                    vec![0xc0],
                ]
                .concat(),
            ),
        ),
    ];
    for (label, row) in cases {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("calls.redb");
        OwnedRedbMessagePackBackingStore::new(&path)
            .unwrap()
            .push(&row)
            .unwrap();
        let error = CallRecordStore::open(&path).err();
        assert_eq!(error, Some(CallRecordError::Corrupt), "{label}");
    }
    // A ledger under the charges of its records is refused too.
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("calls.redb");
    let mut backing = OwnedRedbMessagePackBackingStore::new(&path).unwrap();
    backing.push(&v1(&key, payload.clone())).unwrap();
    backing.push(&spend_row("run-x", 2)).unwrap();
    drop(backing);
    assert_eq!(
        CallRecordStore::open(&path).err(),
        Some(CallRecordError::Corrupt)
    );
    // The same rows, well formed and covered by the ledger, open.
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("calls.redb");
    let mut backing = OwnedRedbMessagePackBackingStore::new(&path).unwrap();
    backing.push(&v1(&key, payload)).unwrap();
    backing.push(&spend_row("run-x", 3)).unwrap();
    drop(backing);
    let store = CallRecordStore::open(&path).unwrap();
    let run = RunId::new("run-x").unwrap();
    assert_eq!(store.records(&run).unwrap(), vec![good]);
    assert_eq!(store.spent(&run).unwrap(), 3);
}

#[test]
fn a_record_is_appended_only_at_the_next_sequence() {
    let directory = tempfile::tempdir().unwrap();
    let (store, _) = store_in(&directory);
    let run = RunId::new("run-seq").unwrap();
    assert_eq!(
        store.append(&sample_record("run-seq", 2)),
        Err(CallRecordError::OutOfSequence)
    );
    store.append(&sample_record("run-seq", 1)).unwrap();
    assert_eq!(
        store.append(&sample_record("run-seq", 1)),
        Err(CallRecordError::OutOfSequence)
    );
    assert_eq!(
        store.append(&sample_record("run-seq", 3)),
        Err(CallRecordError::OutOfSequence)
    );
    store.append(&sample_record("run-seq", 2)).unwrap();
    assert_eq!(store.next_sequence(&run), Ok(3));
}

#[test]
fn a_run_id_is_a_key_safe_segment() {
    assert!(RunId::new("run_1-A").is_ok());
    assert!(RunId::new(&"a".repeat(64)).is_ok());
    for bad in ["", "has/slash", "has space", "caf\u{e9}", &"a".repeat(65)] {
        assert_eq!(RunId::new(bad), Err(CallRecordError::InvalidRunId));
    }
}

#[tokio::test]
async fn replay_keys_the_request_content_not_the_command_identity() {
    let directory = tempfile::tempdir().unwrap();
    let (store, _) = store_in(&directory);
    let run = RunId::new("run-world").unwrap();
    let inner = ScriptedPort::new(ok_all(&[reply("one", "r1"), reply("two", "r2")]));
    let live = recording(&inner, &store, "run-world", u64::MAX);
    for prompt in ["first", "second"] {
        live.infer(
            live.prepare(request_for(CommandId::new(), prompt, 1_200))
                .unwrap(),
        )
        .await
        .unwrap();
    }
    // A fresh genesis names every command anew; the content is what repeats.
    let replay = ReplayInferencePort::new("verse-replay", &store, &run, &start()).unwrap();
    let under_new_names = |prompt: &str, max: u32| {
        replay
            .prepare(request_for(CommandId::new(), prompt, max))
            .unwrap()
    };
    // The same words under a different output allowance are another request.
    replay
        .infer(under_new_names("first", 1_201))
        .await
        .expect_err("a different shape");
    let one = replay.infer(under_new_names("first", 1_200)).await.unwrap();
    let two = replay
        .infer(under_new_names("second", 1_200))
        .await
        .unwrap();
    assert_eq!(json(&one), json(&reply("one", "r1")));
    assert_eq!(json(&two), json(&reply("two", "r2")));
}

#[tokio::test]
async fn usage_one_token_over_the_bound_faults_and_stores_no_record() {
    let directory = tempfile::tempdir().unwrap();
    let (store, _) = store_in(&directory);
    let prepared = prepared_for("one over", 1_200);
    let bound = bound_of(&prepared);
    let inner = ScriptedPort::new(vec![Ok(
        reply("edge", "r1").with_usage(usage(bound, 1, None))
    )]);
    let port = recording(&inner, &store, "run-edge", bound + 200);
    let fault = port.infer(prepared).await.expect_err("one over");
    assert_eq!(fault.class(), InferenceFaultClass::BudgetExhausted);
    assert_eq!(port.spent(), bound + 1, "charged in full");
    assert!(
        store
            .records(&RunId::new("run-edge").unwrap())
            .unwrap()
            .is_empty()
    );
}

/// Every field of the provider request is in the replay key except the two
/// ids that name the call.
#[test]
fn the_replay_key_covers_what_the_request_asks_and_not_what_names_it() {
    let base = prepared_for("key", 1_200).invocation.request.clone();
    let digest = |edit: &dyn Fn(&mut CodexProviderRequest)| {
        let mut request = base.clone();
        edit(&mut request);
        content_digest(&request).unwrap()
    };
    let same = digest(&|_| {});
    assert_eq!(same, content_digest(&base).unwrap());
    let changes: Vec<(&str, Box<dyn Fn(&mut CodexProviderRequest)>)> = vec![
        ("model", Box::new(|r| r.model.push('x'))),
        ("instructions", Box::new(|r| r.instructions.push('x'))),
        (
            "tools",
            Box::new(|r| {
                r.tools.push(CodexToolDefinition {
                    name: "t".into(),
                    description: "d".into(),
                    parameters_json: "{}".into(),
                })
            }),
        ),
        (
            "input",
            Box::new(|r| r.input.push(CodexInputItem::UserText { text: "x".into() })),
        ),
        (
            "max_output_tokens",
            Box::new(|r| r.max_output_tokens = Some(1)),
        ),
        (
            "reasoning_effort",
            Box::new(|r| r.reasoning_effort = Some("low".into())),
        ),
        (
            "service_tier",
            Box::new(|r| r.service_tier = Some("flex".into())),
        ),
        (
            "output_schema_json",
            Box::new(|r| r.output_schema_json = Some("{}".into())),
        ),
        (
            "previous_response_id",
            Box::new(|r| r.previous_response_id = Some("p".into())),
        ),
        (
            "parallel_tool_calls",
            Box::new(|r| r.parallel_tool_calls = !r.parallel_tool_calls),
        ),
        ("schema_id", Box::new(|r| r.schema_id.push_str(".v2"))),
        (
            "reasoning_summary",
            Box::new(|r| r.reasoning_summary = Some("auto".into())),
        ),
        (
            "output_format_name",
            Box::new(|r| r.output_format_name = Some("persona_reply".into())),
        ),
        (
            "tool_choice",
            Box::new(|r| {
                r.tool_choice = match r.tool_choice {
                    codex_connector::CodexToolChoice::Auto => {
                        codex_connector::CodexToolChoice::Required
                    }
                    codex_connector::CodexToolChoice::Required => {
                        codex_connector::CodexToolChoice::Auto
                    }
                }
            }),
        ),
        (
            "prompt_cache_key",
            Box::new(|r| r.prompt_cache_key = Some("ghostlight-persona-1".into())),
        ),
    ];
    for (field, change) in changes {
        assert_ne!(digest(&*change), same, "{field} is not in the replay key");
    }
    assert_eq!(digest(&|r| r.request_id.push('x')), same);
    assert_eq!(digest(&|r| r.conversation_id.push('x')), same);
}

fn open_with(
    rows: Vec<CultCacheEnvelope>,
) -> (tempfile::TempDir, Result<CallRecordStore, CallRecordError>) {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("calls.redb");
    let mut backing = OwnedRedbMessagePackBackingStore::new(&path).unwrap();
    for row in &rows {
        backing.push(row).unwrap();
    }
    drop(backing);
    let opened = CallRecordStore::open(&path);
    (directory, opened)
}

fn record_row(record: &ProviderCallRecord) -> CultCacheEnvelope {
    envelope(
        CALL_RECORD_ROW,
        Some(CALL_RECORD_SCHEMA),
        &row_key(&record.run_id, record.sequence),
        rmp_serde::to_vec_named(record).unwrap(),
    )
}

#[test]
fn a_run_with_sequence_zero_beside_a_hole_is_refused() {
    let (_directory, opened) = open_with(vec![
        record_row(&sample_record("run-x", 0)),
        record_row(&sample_record("run-x", 2)),
        spend_row("run-x", 6),
    ]);
    assert_eq!(opened.err(), Some(CallRecordError::Corrupt));
}

#[test]
fn a_record_charged_nothing_still_needs_its_ledger() {
    let mut record = sample_record("run-y", 1);
    record.charged = 0;
    let (_directory, opened) = open_with(vec![record_row(&record)]);
    assert_eq!(opened.err(), Some(CallRecordError::Corrupt));
}

#[test]
fn a_record_whose_run_id_is_no_whole_key_segment_is_refused() {
    let mut stray = sample_record("run-z/b", 1);
    stray.charged = 0;
    let (_directory, opened) = open_with(vec![
        record_row(&stray),
        record_row(&sample_record("run-z", 1)),
        spend_row("run-z", 3),
    ]);
    assert_eq!(opened.err(), Some(CallRecordError::Corrupt));
    // A neighbouring run id that merely shares the prefix keeps its own records.
    let (_directory, opened) = open_with(vec![
        record_row(&sample_record("run-z", 1)),
        spend_row("run-z", 3),
        spend_row("run-z-b", 0),
    ]);
    let store = opened.unwrap();
    assert_eq!(
        store.records(&RunId::new("run-z").unwrap()).unwrap().len(),
        1
    );
}

#[tokio::test]
async fn a_run_reopens_only_under_the_cap_it_began_with() {
    let directory = tempfile::tempdir().unwrap();
    let (store, path) = store_in(&directory);
    let inner = ScriptedPort::new(Vec::new());
    drop(recording(&inner, &store, "run-cap", 5_000));
    let run = RunId::new("run-cap").unwrap();
    let open = |store: &Arc<CallRecordStore>, cap: u64| {
        RecordingInferencePort::open(inner.clone(), store.clone(), run.clone(), cap, &start())
    };
    for other in [4_999, 5_001, u64::MAX] {
        assert_eq!(
            open(&store, other).err(),
            Some(CallRecordError::RunMismatch(RunField::Cap))
        );
    }
    // A refused reopen does not hold the run, and the cap outlives the process.
    drop(open(&store, 5_000).expect("the same cap reopens"));
    drop(store);
    let store = Arc::new(CallRecordStore::open(&path).unwrap());
    assert_eq!(
        open(&store, 5_001).err(),
        Some(CallRecordError::RunMismatch(RunField::Cap))
    );
    open(&store, 5_000).expect("the same cap reopens after a restart");
    let message = format!("{}", CallRecordError::RunMismatch(RunField::Cap));
    assert!(message.contains("token cap") && !message.contains("5000"));
}

/// A world made by the production path: one genesis command in a fresh world
/// file, its snapshot, and the file's path.
async fn genesis(
    directory: &std::path::Path,
    command: CommandId,
) -> (crate::WorldSnapshot, std::path::PathBuf) {
    let path = directory.join("world.cc");
    let (mailbox, task) = crate::WorldMailbox::open(&path).expect("an empty world");
    mailbox
        .create_fixture(
            crate::tests::creation(command, "Replayed"),
            &crate::tests::auth_principal(crate::tests::owner()),
        )
        .await
        .expect("a created world");
    let snapshot = mailbox.snapshot().await.unwrap();
    drop(mailbox);
    task.await.unwrap();
    (snapshot, path)
}

async fn reopened(path: &std::path::Path) -> crate::WorldSnapshot {
    let (mailbox, task) = crate::WorldMailbox::open(path).expect("a restored world");
    let snapshot = mailbox.snapshot().await.unwrap();
    drop(mailbox);
    task.await.unwrap();
    snapshot
}

#[tokio::test]
async fn replay_runs_against_a_restored_copy_and_refuses_any_other_world_up_front() {
    let command = CommandId::new();
    let first = tempfile::tempdir().unwrap();
    let (world, world_file) = genesis(first.path(), command).await;
    let began = RunStart::of(&world);

    let directory = tempfile::tempdir().unwrap();
    let (store, _) = store_in(&directory);
    let run = RunId::new("run-world").unwrap();
    let inner = ScriptedPort::new(ok_all(&[reply("one", "r1")]));
    let live =
        RecordingInferencePort::open(inner.clone(), store.clone(), run.clone(), u64::MAX, &began)
            .unwrap();
    live.infer(live.prepare(request_for(command, "go", 1_200)).unwrap())
        .await
        .unwrap();
    drop(live);

    // A byte copy of the starting world, restored, replays the run.
    let restored_dir = tempfile::tempdir().unwrap();
    let restored_file = restored_dir.path().join("world.cc");
    std::fs::copy(&world_file, &restored_file).unwrap();
    let restored = RunStart::of(&reopened(&restored_file).await);
    assert_eq!(restored, began);
    let replay = ReplayInferencePort::new("verse-replay", &store, &run, &restored)
        .expect("a restored copy replays");
    let served = replay
        .infer(replay.prepare(request_for(command, "go", 1_200)).unwrap())
        .await
        .unwrap();
    assert_eq!(json(&served), json(&reply("one", "r1")));

    // The same genesis created again is another world and is refused before
    // any call, naming the field and nothing it held.
    let again = tempfile::tempdir().unwrap();
    let (recreated, _) = genesis(again.path(), command).await;
    let refusal = ReplayInferencePort::new("verse-replay", &store, &run, &RunStart::of(&recreated))
        .err()
        .expect("a re-created world");
    assert_eq!(refusal, CallRecordError::RunMismatch(RunField::WorldId));
    let text = format!("{refusal} {refusal:?}");
    assert!(!text.contains(&began.world_id) && !text.contains(&began.state_digest));

    // The same id with another digest is refused too.
    let drifted = RunStart {
        world_id: began.world_id.clone(),
        state_digest: format!("{}x", began.state_digest),
    };
    assert_eq!(
        ReplayInferencePort::new("verse-replay", &store, &run, &drifted).err(),
        Some(CallRecordError::RunMismatch(RunField::StateDigest))
    );
    assert_eq!(inner.calls(), 1, "replay never reached the provider");
}
