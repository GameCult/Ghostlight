use super::*;
use crate::CommandId;
use crate::controllers::{InferenceFaultDisposition, RequestShape, tool_request};
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

fn recording(
    inner: &Arc<ScriptedPort>,
    store: &Arc<CallRecordStore>,
    run: &str,
    cap: u64,
) -> RecordingInferencePort {
    RecordingInferencePort::open(inner.clone(), store.clone(), RunId::new(run).unwrap(), cap)
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
        let digest = hex(&prepared.invocation.provider_request_sha256);
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
        assert_eq!(record.request_sha256(), digest);
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

    // Exactly the cap is allowed.
    let exact = recording(&inner, &store, "run-cap", bound);
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
    while inner.calls() == 0 {
        tokio::task::yield_now().await;
    }
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
async fn a_fault_is_charged_its_bound_unless_it_never_connected() {
    let directory = tempfile::tempdir().unwrap();
    let (store, _) = store_in(&directory);
    let prepared = prepared_for("fault", 1_200);
    let bound = bound_of(&prepared);
    let inner = ScriptedPort::new(vec![
        Err(InferenceFault::retryable("down").classed(InferenceFaultClass::Connect)),
        Err(InferenceFault::retryable("slow").classed(InferenceFaultClass::Timeout)),
    ]);
    let port = recording(&inner, &store, "run-fault", u64::MAX);
    port.infer(prepared.clone())
        .await
        .expect_err("connect fault");
    assert_eq!(port.spent(), 0, "nothing was sent");
    let fault = port.infer(prepared).await.expect_err("timeout fault");
    assert_eq!(
        fault.class(),
        InferenceFaultClass::Timeout,
        "the fault passes through"
    );
    assert_eq!(
        port.spent(),
        bound,
        "a request that may have run is charged its bound"
    );
    assert!(
        store
            .records(&RunId::new("run-fault").unwrap())
            .unwrap()
            .is_empty()
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
    let first = recording(&inner, &store, "run-resume", bound * 2);
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

    let stored = store.records(&run).unwrap();
    let replay = ReplayInferencePort::new("verse-replay", stored).unwrap();
    // A request the run never made is a fault and consumes nothing.
    let stranger = replay
        .prepare(request_for(CommandId::new(), "a", 1_200))
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
    replay
        .infer(second_first)
        .await
        .expect_err("out of order");
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
        request_sha256: "ab".into(),
        events: vec![InferenceEvent::Text("t".into())],
        receipt_digest: "r".into(),
        usage: None,
        charged: 3,
    }
}

#[test]
fn replay_needs_one_runs_dense_records() {
    assert!(
        ReplayInferencePort::new("x", vec![sample_record("a", 1), sample_record("a", 2)]).is_ok()
    );
    for records in [
        vec![sample_record("a", 2)],
        vec![sample_record("a", 1), sample_record("a", 3)],
        vec![sample_record("a", 1), sample_record("b", 2)],
    ] {
        assert_eq!(
            ReplayInferencePort::new("x", records).err(),
            Some(CallRecordError::NotReplayable)
        );
    }
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
    assert_eq!(raw.len(), 1);
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
    // The same row, well formed, opens.
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("calls.redb");
    OwnedRedbMessagePackBackingStore::new(&path)
        .unwrap()
        .push(&v1(&key, payload))
        .unwrap();
    let store = CallRecordStore::open(&path).unwrap();
    let run = RunId::new("run-x").unwrap();
    assert_eq!(store.records(&run).unwrap(), vec![good]);
    assert_eq!(store.charged(&run).unwrap(), 3);
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
