//! The Claude Agent SDK behind Ghostlight's inference seam, and the rule that
//! decides which lane reaches it.
//!
//! This module owns one authority: the protocol that drives a Node sidecar
//! through one SDK query and lowers the result back into the same
//! `InferenceOutput` the connector port produces. It builds no request shape of
//! its own, computes no tool result, and holds no credential.

use super::controllers::{
    InferenceEvent, InferenceFault, InferenceOutput, InferencePort, InferenceRequest,
    PreparedInference, REQUEST_EXPIRY, RESPONSE_TIMEOUT, ToolResultOracle, prepare_invocation,
    unix_ms,
};
use async_trait::async_trait;
use codex_connector::CodexInputItem;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWriteExt};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};
use tokio::sync::Mutex as AsyncMutex;

/// One frame on the sidecar pipe: a 4-byte big-endian length prefix followed by
/// `rmp_serde::to_vec_named` bytes, so every variant is a string-keyed map on
/// the TypeScript side. The cap is the connector's own frame cap, because the
/// largest request either transport carries is the same grouped cell.
pub(super) const MAX_SIDECAR_FRAME_BYTES: usize = 1_052_672;

/// The reasoning-effort values the SDK's own `EffortLevel` admits. A request
/// carrying anything else is refused rather than silently mis-mapped.
const SDK_EFFORT_LEVELS: [&str; 5] = ["low", "medium", "high", "xhigh", "max"];

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(super) enum SidecarFrame {
    Query {
        query_id: u64,
        model: String,
        instructions: String,
        prompt: String,
        transcript: Vec<String>,
        tools: Vec<SidecarTool>,
        effort: Option<String>,
        max_output_tokens: Option<u32>,
        turn_cap: u32,
    },
    ToolCall {
        query_id: u64,
        call_id: String,
        name: String,
        arguments: String,
    },
    ToolResult {
        query_id: u64,
        call_id: String,
        output: String,
    },
    Output {
        query_id: u64,
        events: Vec<SidecarEvent>,
        receipt: SdkResultMaterial,
    },
    Fault {
        query_id: u64,
        reason: SidecarFaultReason,
        detail: String,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct SidecarTool {
    pub(super) name: String,
    pub(super) description: String,
    pub(super) parameters_json: String,
}

/// One block of one assistant message, in order. `dispatched` is false only for
/// a `tool_use` block naming a tool the sidecar did not register — the harness
/// answered that one itself and the oracle never saw it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(super) enum SidecarEvent {
    Text {
        text: String,
    },
    ToolCall {
        call_id: String,
        name: String,
        arguments: String,
        dispatched: bool,
    },
}

/// What the sidecar reports about the SDK session that produced one output.
/// Every field here is the sidecar's; the identity half of the receipt is
/// filled in by this port from the invocation it already holds.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct SdkResultMaterial {
    pub(super) session_id: String,
    pub(super) result_uuid: String,
    pub(super) subtype: String,
    pub(super) stop_reason: Option<String>,
    pub(super) num_turns: u32,
    pub(super) assistant_message_uuids: Vec<String>,
    pub(super) assistant_request_ids: Vec<String>,
    pub(super) usage: Vec<SdkModelUsage>,
    /// A decimal string, because the receipt derives `Eq` and because this is a
    /// client-side estimate rather than a billing statement.
    pub(super) total_cost_usd_estimate: String,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub(super) struct SdkModelUsage {
    pub(super) model: String,
    pub(super) input_tokens: u64,
    pub(super) output_tokens: u64,
    pub(super) cache_read_input_tokens: u64,
    pub(super) cache_creation_input_tokens: u64,
}

/// The failures the sidecar is allowed to name. It reports a reason; this
/// module assigns the disposition, so TypeScript can never quarantine the
/// world's cognition by naming an integrity violation directly.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum SidecarFaultReason {
    RateLimited,
    Overloaded,
    ServerError,
    ApiTimeout,
    AuthenticationFailed,
    OrgNotAllowed,
    BillingError,
    InvalidRequest,
    ModelNotFound,
    MaxOutputTokens,
    MaxBudgetUsd,
    ExecutionError,
    Unknown,
    ProtocolViolation,
    ToolRegistrationFailed,
    TurnCapRefused,
}

impl SidecarFaultReason {
    fn into_fault(self, detail: String) -> InferenceFault {
        let detail = format!("SDK sidecar reported {self:?}: {detail}");
        match self {
            Self::RateLimited | Self::Overloaded | Self::ServerError | Self::ApiTimeout => {
                InferenceFault::retryable(detail)
            }
            Self::AuthenticationFailed
            | Self::OrgNotAllowed
            | Self::BillingError
            | Self::InvalidRequest
            | Self::ModelNotFound
            | Self::MaxOutputTokens
            | Self::MaxBudgetUsd
            | Self::ExecutionError
            | Self::Unknown => InferenceFault::new(detail),
            Self::ProtocolViolation | Self::ToolRegistrationFailed | Self::TurnCapRefused => {
                InferenceFault::integrity_violation(detail)
            }
        }
    }
}

#[derive(Debug)]
pub(super) enum SidecarLinkError {
    /// The pipe reached EOF or the child exited. Never a partial output.
    Closed,
    /// A length prefix over the cap, a truncated body, or an undecodable one.
    Codec(String),
    /// The child could not be started. The entry file existed at open, so this
    /// is an environment fault an operator must fix.
    Spawn(String),
}

/// The child-process seam, so the protocol driver is testable without Node, a
/// credential, or a process. `ChildProcessLink` is production; the tests drive a
/// scripted in-process link over the same trait.
#[async_trait]
pub(super) trait SidecarLink: Send + Sync {
    async fn send(&self, frame: SidecarFrame) -> Result<(), SidecarLinkError>;
    async fn recv(&self) -> Result<SidecarFrame, SidecarLinkError>;
    /// Kills and respawns the child, so no query inherits another query's
    /// half-read pipe. Called by the driver on every fault, and only there.
    async fn restart(&self) -> Result<(), SidecarLinkError>;
}

/// Reads one length-prefixed MessagePack frame. Free so the codec is testable
/// over a `tokio::io::duplex` pair without a process.
pub(super) async fn read_frame<R: AsyncRead + Unpin>(
    reader: &mut R,
) -> Result<SidecarFrame, SidecarLinkError> {
    let mut prefix = [0_u8; 4];
    reader
        .read_exact(&mut prefix)
        .await
        .map_err(|_| SidecarLinkError::Closed)?;
    let length = u32::from_be_bytes(prefix) as usize;
    if length == 0 || length > MAX_SIDECAR_FRAME_BYTES {
        return Err(SidecarLinkError::Codec(format!(
            "sidecar frame length {length} is outside 1..={MAX_SIDECAR_FRAME_BYTES}"
        )));
    }
    let mut body = vec![0_u8; length];
    reader
        .read_exact(&mut body)
        .await
        .map_err(|error| SidecarLinkError::Codec(error.to_string()))?;
    rmp_serde::from_slice(&body).map_err(|error| SidecarLinkError::Codec(error.to_string()))
}

fn encode_frame(frame: &SidecarFrame) -> Result<Vec<u8>, SidecarLinkError> {
    let body = rmp_serde::to_vec_named(frame)
        .map_err(|error| SidecarLinkError::Codec(error.to_string()))?;
    if body.len() > MAX_SIDECAR_FRAME_BYTES {
        return Err(SidecarLinkError::Codec(format!(
            "sidecar frame of {} bytes exceeds {MAX_SIDECAR_FRAME_BYTES}",
            body.len()
        )));
    }
    let mut framed = Vec::with_capacity(body.len() + 4);
    framed.extend_from_slice(&(body.len() as u32).to_be_bytes());
    framed.extend_from_slice(&body);
    Ok(framed)
}

struct SidecarChild {
    child: Child,
    stdin: ChildStdin,
    stdout: ChildStdout,
}

/// One persistent `node <entry>` child, respawned on fault. `stderr` is
/// inherited: the sidecar logs its own faults there and never a prompt, a tool
/// argument, or a tool result.
pub(super) struct ChildProcessLink {
    entry: PathBuf,
    child: AsyncMutex<Option<SidecarChild>>,
}

impl ChildProcessLink {
    pub(super) fn new(entry: PathBuf) -> Self {
        Self {
            entry,
            child: AsyncMutex::new(None),
        }
    }

    fn spawn(&self) -> Result<SidecarChild, SidecarLinkError> {
        let mut child = Command::new("node")
            .arg(&self.entry)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .kill_on_drop(true)
            .spawn()
            .map_err(|error| SidecarLinkError::Spawn(error.to_string()))?;
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| SidecarLinkError::Spawn("sidecar stdin was not piped".into()))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| SidecarLinkError::Spawn("sidecar stdout was not piped".into()))?;
        Ok(SidecarChild {
            child,
            stdin,
            stdout,
        })
    }
}

#[async_trait]
impl SidecarLink for ChildProcessLink {
    async fn send(&self, frame: SidecarFrame) -> Result<(), SidecarLinkError> {
        let bytes = encode_frame(&frame)?;
        let mut held = self.child.lock().await;
        if held.is_none() {
            *held = Some(self.spawn()?);
        }
        let child = held.as_mut().expect("the child was just ensured");
        child
            .stdin
            .write_all(&bytes)
            .await
            .map_err(|_| SidecarLinkError::Closed)?;
        child
            .stdin
            .flush()
            .await
            .map_err(|_| SidecarLinkError::Closed)
    }

    async fn recv(&self) -> Result<SidecarFrame, SidecarLinkError> {
        let mut held = self.child.lock().await;
        let Some(child) = held.as_mut() else {
            return Err(SidecarLinkError::Closed);
        };
        // The driver's only clock is the connector's own response timeout: a
        // seed patch at medium effort was measured past five minutes, so no
        // tighter deadline may be imposed here.
        match tokio::time::timeout(RESPONSE_TIMEOUT, read_frame(&mut child.stdout)).await {
            Ok(frame) => frame,
            Err(_) => Err(SidecarLinkError::Closed),
        }
    }

    async fn restart(&self) -> Result<(), SidecarLinkError> {
        let mut held = self.child.lock().await;
        if let Some(mut child) = held.take() {
            let _ = child.child.start_kill();
            let _ = child.child.wait().await;
        }
        Ok(())
    }
}

/// What the SDK reported, bound to the identity Ghostlight computed. The
/// identity half is filled here from the invocation, so the sidecar cannot
/// assert a request it was not given; the SDK half is the session and the
/// messages that produced this output.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct SdkInferenceReceipt {
    schema_id: String,
    request_id: String,
    conversation_id: String,
    caller_runtime_id: String,
    native_request_sha256: [u8; 32],
    provider_request_sha256: [u8; 32],
    model: String,
    session_id: String,
    result_uuid: String,
    subtype: String,
    stop_reason: Option<String>,
    num_turns: u32,
    assistant_message_uuids: Vec<String>,
    assistant_request_ids: Vec<String>,
    usage: Vec<SdkModelUsage>,
    total_cost_usd_estimate: String,
}

const SDK_RECEIPT_SCHEMA: &str = "ghostlight.sdk_inference_receipt.v1";

/// The model-name prefix that names the SDK transport when nothing configures
/// one. It holds even with no SDK port built, so a `claude-`prefixed lane on a
/// connector-only deployment is refused at open instead of quietly reaching the
/// wrong backend.
pub(super) const DEFAULT_SDK_MODEL_PREFIX: &str = "claude";

/// Everything the SDK sidecar needs to open, gathered so `open_inference` takes
/// a binding rather than reading the environment itself.
pub(crate) struct SdkBinding {
    pub(crate) sidecar_entry: PathBuf,
    pub(crate) caller_runtime_id: String,
    pub(crate) model_prefix: String,
}

/// The Claude Agent SDK behind Ghostlight's inference seam, through a Node
/// sidecar this port owns. It is a stopgap for one reason — it borrows the
/// Claude Code subscription credential, which Ghostlight never sees — and it
/// carries two liabilities that a Messages-API port would not:
///
/// The receipt is the SDK's message, not wire bytes. Its identity half is
/// computed here from the invocation, but its provenance half is a session id
/// and message uuids reported by a child process this port spawned. It attests
/// that this exact request produced this exact SDK session; it does not attest
/// against a party Ghostlight does not control, which is what the connector's
/// receipt does. `PersonaTurnBinding` keeps its meaning within that limit: the
/// two digests it binds still differ per round and still cannot be swapped.
///
/// A prior round's conversation reaches the model as prose. The SDK owns the
/// assistant side of its transcript, so typed `tool_call`/`tool_result` turns
/// cannot be replayed into it. `parallel_tool_calls` and `tool_choice` have no
/// SDK counterpart and are not sent; they still ride in the request and into
/// every digest.
///
/// When an `ANTHROPIC_API_KEY` and a budget exist, a Messages-API port is a
/// closer structural match to `InferencePort` than the SDK is — inert
/// `tool_use` blocks, a caller-appended `tool_result`, a real request id, real
/// concurrency, no subprocess — and this port is deleted rather than extended.
pub(super) struct SdkInferencePort {
    link: Arc<dyn SidecarLink>,
    caller_runtime_id: String,
    /// Holding this is the one-query-in-flight permit and the source of query
    /// ids. One child, one query: the pipe carries a single query's tool-call
    /// rendezvous, and every SDK query spawns its own Claude Code subprocess
    /// regardless, so a second in-flight query buys a second subprocess rather
    /// than a shared connection.
    gate: AsyncMutex<u64>,
    oracles: Mutex<BTreeMap<String, Box<dyn ToolResultOracle>>>,
}

impl SdkInferencePort {
    pub(super) fn new(link: Arc<dyn SidecarLink>, caller_runtime_id: impl Into<String>) -> Self {
        Self {
            link,
            caller_runtime_id: caller_runtime_id.into(),
            gate: AsyncMutex::new(0),
            oracles: Mutex::new(BTreeMap::new()),
        }
    }

    fn take_oracle(&self, request_id: &str) -> Option<Box<dyn ToolResultOracle>> {
        self.oracles
            .lock()
            .expect("the oracle map is never poisoned")
            .remove(request_id)
    }

    async fn run_query(
        &self,
        prepared: PreparedInference,
        mut oracle: Option<Box<dyn ToolResultOracle>>,
        turn_cap: u32,
    ) -> Result<InferenceOutput, InferenceFault> {
        let mut gate = self.gate.lock().await;
        *gate = gate.wrapping_add(1);
        let query_id = *gate;
        let frame = lower_query(query_id, &prepared, turn_cap)?;
        if let Err(error) = self.link.send(frame).await {
            let _ = self.link.restart().await;
            return Err(link_fault(error));
        }
        let mut answered: Vec<(String, String, String)> = Vec::new();
        loop {
            let frame = match self.link.recv().await {
                Ok(frame) => frame,
                Err(error) => {
                    let _ = self.link.restart().await;
                    return Err(link_fault(error));
                }
            };
            match frame {
                SidecarFrame::ToolCall {
                    query_id: seen,
                    call_id,
                    name,
                    arguments,
                } if seen == query_id => {
                    let Some(oracle) = oracle.as_mut() else {
                        let _ = self.link.restart().await;
                        return Err(InferenceFault::integrity_violation(
                            "the SDK sidecar issued a tool call for a request that carries no tools",
                        ));
                    };
                    match oracle.answer(&name, &arguments) {
                        Ok(output) => {
                            answered.push((call_id.clone(), name, arguments));
                            if let Err(error) = self
                                .link
                                .send(SidecarFrame::ToolResult {
                                    query_id,
                                    call_id,
                                    output,
                                })
                                .await
                            {
                                let _ = self.link.restart().await;
                                return Err(link_fault(error));
                            }
                        }
                        Err(error) => {
                            let _ = self.link.restart().await;
                            return Err(InferenceFault::new(error.to_string()));
                        }
                    }
                }
                SidecarFrame::Output {
                    query_id: seen,
                    events,
                    receipt,
                } if seen == query_id => {
                    return assemble_output(&prepared, events, receipt, &answered);
                }
                SidecarFrame::Fault {
                    query_id: seen,
                    reason,
                    detail,
                } if seen == query_id => {
                    let _ = self.link.restart().await;
                    return Err(reason.into_fault(detail));
                }
                _ => {
                    let _ = self.link.restart().await;
                    return Err(InferenceFault::integrity_violation(
                        "the SDK sidecar sent a frame this query did not ask for",
                    ));
                }
            }
        }
    }
}

fn link_fault(error: SidecarLinkError) -> InferenceFault {
    match error {
        SidecarLinkError::Closed => {
            InferenceFault::new("the SDK sidecar exited during a query".to_string())
        }
        SidecarLinkError::Spawn(detail) => {
            InferenceFault::new(format!("the SDK sidecar could not start: {detail}"))
        }
        SidecarLinkError::Codec(detail) => {
            InferenceFault::integrity_violation(format!("SDK sidecar framing: {detail}"))
        }
    }
}

/// Lowers one prepared request into the frame the sidecar reads. Deciding that
/// the first input item is the prompt and the rest are a prior-round transcript
/// is Ghostlight's claim about its own conversation type, so it is made here,
/// next to the evaluators that built those items — and no SDK option name
/// appears in this crate.
pub(super) fn lower_query(
    query_id: u64,
    prepared: &PreparedInference,
    turn_cap: u32,
) -> Result<SidecarFrame, InferenceFault> {
    let request = &prepared.invocation.request;
    if let Some(effort) = request.reasoning_effort.as_deref()
        && !SDK_EFFORT_LEVELS.contains(&effort)
    {
        return Err(InferenceFault::integrity_violation(format!(
            "reasoning effort `{effort}` has no SDK counterpart"
        )));
    }
    let mut items = request.input.iter();
    let Some(CodexInputItem::UserText { text: prompt }) = items.next() else {
        return Err(InferenceFault::integrity_violation(
            "an SDK request must open with exactly one user text item",
        ));
    };
    let mut transcript = Vec::new();
    for item in items {
        match item {
            CodexInputItem::AssistantText { text } => transcript.push(format!("assistant: {text}")),
            CodexInputItem::ToolCall {
                name, arguments, ..
            } => transcript.push(format!("tool call {name}: {arguments}")),
            CodexInputItem::ToolResult { output, .. } => {
                transcript.push(format!("tool result: {output}"))
            }
            CodexInputItem::UserText { .. } => {
                return Err(InferenceFault::integrity_violation(
                    "an SDK request carries one user text item and no other",
                ));
            }
        }
    }
    Ok(SidecarFrame::Query {
        query_id,
        model: request.model.clone(),
        instructions: request.instructions.clone(),
        prompt: prompt.clone(),
        transcript,
        tools: request
            .tools
            .iter()
            .map(|tool| SidecarTool {
                name: tool.name.clone(),
                description: tool.description.clone(),
                parameters_json: tool.parameters_json.clone(),
            })
            .collect(),
        effort: request.reasoning_effort.clone(),
        max_output_tokens: request.max_output_tokens,
        turn_cap,
    })
}

/// A call id the connector's own validators would refuse cannot be persisted:
/// the evaluator rebuilds these into `CodexInputItem`s that `validate()` will
/// see again.
fn call_id_is_valid(call_id: &str) -> bool {
    !call_id.is_empty() && call_id.len() <= 64 && call_id.is_ascii()
}

fn tool_name_is_valid(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 64
        && name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-')
}

fn assemble_output(
    prepared: &PreparedInference,
    events: Vec<SidecarEvent>,
    material: SdkResultMaterial,
    answered: &[(String, String, String)],
) -> Result<InferenceOutput, InferenceFault> {
    let request = &prepared.invocation.request;
    let mut lowered = Vec::with_capacity(events.len());
    let mut dispatched: Vec<(String, String, String)> = Vec::new();
    for event in events {
        match event {
            SidecarEvent::Text { text } => lowered.push(InferenceEvent::Text(text)),
            SidecarEvent::ToolCall {
                call_id,
                name,
                arguments,
                dispatched: was_dispatched,
            } => {
                if !call_id_is_valid(&call_id) || !tool_name_is_valid(&name) {
                    return Err(InferenceFault::integrity_violation(
                        "the SDK sidecar reported a call id or tool name the provider contract refuses",
                    ));
                }
                if was_dispatched {
                    dispatched.push((call_id.clone(), name.clone(), arguments.clone()));
                } else if request.tools.iter().any(|tool| tool.name == name) {
                    // A registered tool the sidecar did not dispatch is a
                    // dropped call, not an invented name.
                    return Err(InferenceFault::integrity_violation(
                        "the SDK sidecar left a registered tool call unanswered",
                    ));
                }
                lowered.push(InferenceEvent::ToolCall {
                    call_id,
                    name,
                    arguments,
                });
            }
        }
    }
    if dispatched != answered {
        return Err(InferenceFault::integrity_violation(
            "the SDK sidecar's dispatched calls do not match the calls this port answered",
        ));
    }
    let receipt = SdkInferenceReceipt {
        schema_id: SDK_RECEIPT_SCHEMA.to_owned(),
        request_id: request.request_id.clone(),
        conversation_id: request.conversation_id.clone(),
        caller_runtime_id: prepared.invocation.caller_runtime_id.clone(),
        native_request_sha256: prepared.invocation.native_request_sha256,
        provider_request_sha256: prepared.invocation.provider_request_sha256,
        model: request.model.clone(),
        session_id: material.session_id,
        result_uuid: material.result_uuid,
        subtype: material.subtype,
        stop_reason: material.stop_reason,
        num_turns: material.num_turns,
        assistant_message_uuids: material.assistant_message_uuids,
        assistant_request_ids: material.assistant_request_ids,
        usage: {
            let mut usage = material.usage;
            usage.sort();
            usage
        },
        total_cost_usd_estimate: material.total_cost_usd_estimate,
    };
    let receipt_bytes = rmp_serde::to_vec_named(&receipt)
        .map_err(|error| InferenceFault::new(error.to_string()))?;
    Ok(InferenceOutput {
        events: lowered,
        receipt_digest: format!("sha256:{:x}", Sha256::digest(&receipt_bytes)),
    })
}

#[async_trait]
impl InferencePort for SdkInferencePort {
    fn prepare(&self, request: InferenceRequest) -> Result<PreparedInference, InferenceFault> {
        prepare_invocation(
            &self.caller_runtime_id,
            unix_ms()?.saturating_add(REQUEST_EXPIRY.as_millis() as u64),
            request,
        )
    }

    async fn infer(&self, request: PreparedInference) -> Result<InferenceOutput, InferenceFault> {
        if request.invocation.caller_runtime_id != self.caller_runtime_id {
            return Err(InferenceFault::integrity_violation(
                "persisted inference caller does not match the configured runtime identity",
            ));
        }
        if request.invocation.expires_at_unix_ms <= unix_ms()? {
            return Err(InferenceFault::new(
                "persisted inference invocation expired before it reached the SDK sidecar",
            ));
        }
        let oracle = self.take_oracle(&request.invocation.request.request_id);
        let has_tools = !request.invocation.request.tools.is_empty();
        let turn_cap = match (&oracle, has_tools) {
            (Some(oracle), true) => oracle.remaining_rounds(),
            (None, false) => 1,
            _ => {
                return Err(InferenceFault::integrity_violation(
                    "a tool request reached the SDK port without its lane's tool-result owner",
                ));
            }
        };
        self.run_query(request, oracle, turn_cap).await
    }

    fn lend_tool_results(&self, prepared: &PreparedInference, oracle: Box<dyn ToolResultOracle>) {
        self.oracles
            .lock()
            .expect("the oracle map is never poisoned")
            .insert(prepared.invocation.request.request_id.clone(), oracle);
    }
}

/// Which port a lane uses is which model it is configured with. The config
/// already gates cognition by model name and deliberately refuses a mode flag,
/// so the transport is bound where the model is named and nowhere else.
pub(super) struct RoutedInferencePort {
    connector: Option<Arc<dyn InferencePort>>,
    sdk: Option<Arc<dyn InferencePort>>,
    sdk_model_prefix: String,
}

impl RoutedInferencePort {
    pub(super) fn new(
        connector: Option<Arc<dyn InferencePort>>,
        sdk: Option<Arc<dyn InferencePort>>,
        sdk_model_prefix: impl Into<String>,
    ) -> Self {
        Self {
            connector,
            sdk,
            sdk_model_prefix: sdk_model_prefix.into(),
        }
    }

    /// The model name carries the transport. A prefixed model that no SDK port
    /// claims routes nowhere rather than falling back: a fallback is what would
    /// make a typo in a model environment variable silent.
    pub(super) fn route(&self, model: &str) -> Option<&Arc<dyn InferencePort>> {
        if model.starts_with(&self.sdk_model_prefix) {
            self.sdk.as_ref()
        } else {
            self.connector.as_ref()
        }
    }
}

#[async_trait]
impl InferencePort for RoutedInferencePort {
    fn prepare(&self, request: InferenceRequest) -> Result<PreparedInference, InferenceFault> {
        let model = request.provider_model().to_owned();
        match self.route(&model) {
            Some(port) => port.prepare(request),
            None => Err(InferenceFault::integrity_violation(format!(
                "no configured inference backend claims the model `{model}`"
            ))),
        }
    }

    async fn infer(&self, request: PreparedInference) -> Result<InferenceOutput, InferenceFault> {
        let model = request.invocation.request.model.clone();
        match self.route(&model) {
            Some(port) => port.infer(request).await,
            None => Err(InferenceFault::integrity_violation(format!(
                "no configured inference backend claims the model `{model}`"
            ))),
        }
    }

    fn lend_tool_results(&self, prepared: &PreparedInference, oracle: Box<dyn ToolResultOracle>) {
        if let Some(port) = self.route(&prepared.invocation.request.model) {
            port.lend_tool_results(prepared, oracle);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::CommandId;
    use crate::world::controllers::{
        ConnectorBinding, ControllerModels, ControllerOpenError, InferencePurpose,
        OperationalOracle, RequestShape, catalog_tools, open_inference, tool_request,
    };
    use crate::world::elaboration::{
        ElaborationLoopEvaluation, ElaborationOracle, SEED_ROUND_BUDGET, evaluate_elaboration_loop,
    };
    use crate::world::patch::{
        RECORD_GAP_PATCH_TOOL, SUBMIT_PATCH_TOOL, kernel_speak_entry, patch_tools,
    };
    use crate::world::{AffordanceId, AffordanceSnapshot};
    use codex_connector::CodexToolDefinition;
    use std::collections::VecDeque;

    const TEST_RUNTIME: &str = "ghostlight-sdk-test";
    const TEST_MODEL: &str = "claude-opus-5";

    /// A `SidecarLink` with no child and no Node: it replays a scripted frame
    /// sequence and records every frame the port sends, so the protocol driver
    /// is exercised without a process or a credential.
    struct ScriptedLink {
        outbound: Mutex<VecDeque<SidecarFrame>>,
        sent: Mutex<Vec<SidecarFrame>>,
        restarts: Mutex<usize>,
    }

    impl ScriptedLink {
        fn new(outbound: Vec<SidecarFrame>) -> Arc<Self> {
            Arc::new(Self {
                outbound: Mutex::new(outbound.into()),
                sent: Mutex::new(Vec::new()),
                restarts: Mutex::new(0),
            })
        }

        fn sent(&self) -> Vec<SidecarFrame> {
            self.sent.lock().unwrap().clone()
        }

        fn restarts(&self) -> usize {
            *self.restarts.lock().unwrap()
        }
    }

    #[async_trait]
    impl SidecarLink for ScriptedLink {
        async fn send(&self, frame: SidecarFrame) -> Result<(), SidecarLinkError> {
            self.sent.lock().unwrap().push(frame);
            Ok(())
        }

        async fn recv(&self) -> Result<SidecarFrame, SidecarLinkError> {
            self.outbound
                .lock()
                .unwrap()
                .pop_front()
                .ok_or(SidecarLinkError::Closed)
        }

        async fn restart(&self) -> Result<(), SidecarLinkError> {
            *self.restarts.lock().unwrap() += 1;
            Ok(())
        }
    }

    fn material() -> SdkResultMaterial {
        SdkResultMaterial {
            session_id: "session-one".into(),
            result_uuid: "result-one".into(),
            subtype: "success".into(),
            stop_reason: Some("end_turn".into()),
            num_turns: 1,
            assistant_message_uuids: vec!["assistant-one".into()],
            assistant_request_ids: vec!["req_one".into()],
            usage: vec![SdkModelUsage {
                model: TEST_MODEL.into(),
                input_tokens: 12,
                output_tokens: 34,
                cache_read_input_tokens: 0,
                cache_creation_input_tokens: 0,
            }],
            total_cost_usd_estimate: "0.0123".into(),
        }
    }

    fn seed_prepared(port: &SdkInferencePort, prompt: &str) -> PreparedInference {
        let request = tool_request(
            CommandId::new(),
            0,
            InferencePurpose::Elaboration,
            TEST_MODEL,
            "Author the shortfall.",
            vec![CodexInputItem::UserText {
                text: prompt.into(),
            }],
            patch_tools(),
            RequestShape {
                max_output_tokens: 4_000,
                parallel_tool_calls: false,
            },
        )
        .expect("the seed request builds");
        port.prepare(request).expect("the SDK port prepares")
    }

    fn prose_request(prompt: &str) -> InferenceRequest {
        tool_request(
            CommandId::new(),
            0,
            InferencePurpose::Persona,
            TEST_MODEL,
            "Respond only in natural prose.",
            vec![CodexInputItem::UserText {
                text: prompt.into(),
            }],
            Vec::<CodexToolDefinition>::new(),
            RequestShape {
                max_output_tokens: 1_200,
                parallel_tool_calls: false,
            },
        )
        .expect("the prose request builds")
    }

    fn prose_prepared(port: &SdkInferencePort, prompt: &str) -> PreparedInference {
        port.prepare(prose_request(prompt))
            .expect("the SDK port prepares")
    }

    fn tool_call_frame(call_id: &str, name: &str, arguments: &str) -> SidecarFrame {
        SidecarFrame::ToolCall {
            query_id: 1,
            call_id: call_id.into(),
            name: name.into(),
            arguments: arguments.into(),
        }
    }

    fn dispatched_event(call_id: &str, name: &str, arguments: &str) -> SidecarEvent {
        SidecarEvent::ToolCall {
            call_id: call_id.into(),
            name: name.into(),
            arguments: arguments.into(),
            dispatched: true,
        }
    }

    /// Spec test 5.
    #[tokio::test]
    async fn a_persona_query_returns_one_prose_output() {
        let link = ScriptedLink::new(vec![SidecarFrame::Output {
            query_id: 1,
            events: vec![SidecarEvent::Text {
                text: "The rain has teeth tonight.".into(),
            }],
            receipt: material(),
        }]);
        let port = SdkInferencePort::new(link.clone(), TEST_RUNTIME);
        let prepared = prose_prepared(&port, "Say something true.");
        let output = port.infer(prepared).await.expect("the prose query returns");
        assert_eq!(
            output.events,
            vec![InferenceEvent::Text("The rain has teeth tonight.".into())]
        );
        let sent = link.sent();
        let [
            SidecarFrame::Query {
                tools, turn_cap, ..
            },
        ] = sent.as_slice()
        else {
            panic!("the port sent something other than one query frame")
        };
        assert!(tools.is_empty(), "a prose request carried tools");
        assert_eq!(*turn_cap, 1, "a prose request asked for more than one turn");
        assert!(output.receipt_digest.starts_with("sha256:"));
    }

    /// Spec test 6. The required end-to-end: every tool result the model saw
    /// came from the lane's own fold, and the finished output re-derives
    /// through the seed evaluator to the same capture.
    #[tokio::test]
    async fn a_multi_tool_query_re_derives_in_the_seed_evaluator() {
        let calls = [
            ("call-0", RECORD_GAP_PATCH_TOOL, r#"{"detail":"no route"}"#),
            ("call-1", RECORD_GAP_PATCH_TOOL, "not json at all"),
            ("call-3", SUBMIT_PATCH_TOOL, "{}"),
        ];
        let mut outbound: Vec<SidecarFrame> = calls
            .iter()
            .map(|(id, name, arguments)| tool_call_frame(id, name, arguments))
            .collect();
        let mut events: Vec<SidecarEvent> = calls
            .iter()
            .map(|(id, name, arguments)| dispatched_event(id, name, arguments))
            .collect();
        // A name the request never registered: the harness answered it, so it
        // is reported undispatched and the oracle never saw it.
        events.insert(
            2,
            SidecarEvent::ToolCall {
                call_id: "call-2".into(),
                name: "speek".into(),
                arguments: "{}".into(),
                dispatched: false,
            },
        );
        outbound.push(SidecarFrame::Output {
            query_id: 1,
            events,
            receipt: material(),
        });
        let link = ScriptedLink::new(outbound);
        let port = SdkInferencePort::new(link.clone(), TEST_RUNTIME);
        let prompt = "Author the shortfall.";
        let prepared = seed_prepared(&port, prompt);
        port.lend_tool_results(
            &prepared,
            Box::new(ElaborationOracle::new(&[], SEED_ROUND_BUDGET)),
        );
        let output = port.infer(prepared).await.expect("the seed query returns");

        // Every answer the port sent is the string the evaluator recomputes.
        let answers: Vec<String> = link
            .sent()
            .into_iter()
            .filter_map(|frame| match frame {
                SidecarFrame::ToolResult { output, .. } => Some(output),
                _ => None,
            })
            .collect();
        assert_eq!(
            answers,
            vec![
                "gap recorded".to_string(),
                "arguments recorded as a gap".to_string(),
                "patch submitted".to_string(),
            ]
        );

        let evaluation = evaluate_elaboration_loop(prompt, &[], &[output], SEED_ROUND_BUDGET)
            .expect("the seed evaluator re-derives");
        let ElaborationLoopEvaluation::Complete { capture } = evaluation else {
            panic!("one SDK query did not close the seed round")
        };
        assert!(capture.submitted, "the re-derived capture did not submit");
    }

    /// Spec test 7.
    #[tokio::test]
    async fn a_sidecar_crash_mid_query_is_recovery_required() {
        let link = ScriptedLink::new(vec![
            tool_call_frame("call-0", RECORD_GAP_PATCH_TOOL, r#"{"detail":"one"}"#),
            tool_call_frame("call-1", RECORD_GAP_PATCH_TOOL, r#"{"detail":"two"}"#),
        ]);
        let port = SdkInferencePort::new(link.clone(), TEST_RUNTIME);
        let prepared = seed_prepared(&port, "Author the shortfall.");
        port.lend_tool_results(
            &prepared,
            Box::new(ElaborationOracle::new(&[], SEED_ROUND_BUDGET)),
        );
        let fault = port
            .infer(prepared)
            .await
            .expect_err("a closed pipe produced an output");
        assert!(fault.recovery_required(), "{fault:?}");
        assert!(!fault.integrity_was_violated());
        assert_eq!(link.restarts(), 1);
    }

    /// Spec test 8.
    #[tokio::test]
    async fn a_malformed_frame_is_an_integrity_violation() {
        async fn refuse(body: Vec<u8>) -> InferenceFault {
            let (mut client, mut server) = tokio::io::duplex(1 << 16);
            client.write_all(&body).await.unwrap();
            drop(client);
            link_fault(read_frame(&mut server).await.expect_err("a frame decoded"))
        }
        let mut over_cap = ((MAX_SIDECAR_FRAME_BYTES + 1) as u32)
            .to_be_bytes()
            .to_vec();
        over_cap.extend_from_slice(b"body");
        assert!(refuse(over_cap).await.integrity_was_violated());

        let mut truncated = 64_u32.to_be_bytes().to_vec();
        truncated.extend_from_slice(b"short");
        assert!(refuse(truncated).await.integrity_was_violated());

        let mut garbage = 5_u32.to_be_bytes().to_vec();
        garbage.extend_from_slice(&[0xC1, 0xC1, 0xC1, 0xC1, 0xC1]);
        assert!(refuse(garbage).await.integrity_was_violated());
    }

    /// Spec test 9.
    #[tokio::test]
    async fn an_over_cap_or_misrouted_frame_is_refused() {
        for frame in [
            SidecarFrame::Output {
                query_id: 99,
                events: Vec::new(),
                receipt: material(),
            },
            SidecarFrame::Query {
                query_id: 1,
                model: TEST_MODEL.into(),
                instructions: "no".into(),
                prompt: "no".into(),
                transcript: Vec::new(),
                tools: Vec::new(),
                effort: None,
                max_output_tokens: None,
                turn_cap: 1,
            },
        ] {
            let link = ScriptedLink::new(vec![frame]);
            let port = SdkInferencePort::new(link.clone(), TEST_RUNTIME);
            let prepared = prose_prepared(&port, "Say something true.");
            let fault = port
                .infer(prepared)
                .await
                .expect_err("a misrouted frame produced an output");
            assert!(fault.integrity_was_violated(), "{fault:?}");
            assert_eq!(link.restarts(), 1);
        }
    }

    /// Spec test 10.
    #[tokio::test]
    async fn the_event_gates_catch_a_lying_sidecar() {
        let answered = ("call-0", RECORD_GAP_PATCH_TOOL, r#"{"detail":"one"}"#);
        let lies = [
            // A dispatched call the port never answered.
            vec![dispatched_event("call-9", RECORD_GAP_PATCH_TOOL, "{}")],
            // A registered tool reported as undispatched: a dropped call.
            vec![
                dispatched_event(answered.0, answered.1, answered.2),
                SidecarEvent::ToolCall {
                    call_id: "call-1".into(),
                    name: RECORD_GAP_PATCH_TOOL.into(),
                    arguments: "{}".into(),
                    dispatched: false,
                },
            ],
            // A call id the provider contract refuses.
            vec![dispatched_event(&"x".repeat(65), answered.1, answered.2)],
        ];
        for events in lies {
            let link = ScriptedLink::new(vec![
                tool_call_frame(answered.0, answered.1, answered.2),
                SidecarFrame::Output {
                    query_id: 1,
                    events,
                    receipt: material(),
                },
            ]);
            let port = SdkInferencePort::new(link.clone(), TEST_RUNTIME);
            let prepared = seed_prepared(&port, "Author the shortfall.");
            port.lend_tool_results(
                &prepared,
                Box::new(ElaborationOracle::new(&[], SEED_ROUND_BUDGET)),
            );
            let fault = port
                .infer(prepared)
                .await
                .expect_err("a lying sidecar produced an output");
            assert!(fault.integrity_was_violated(), "{fault:?}");
        }

        // An undispatched call naming a tool the request does not carry is
        // accepted, and its event survives into the output.
        let link = ScriptedLink::new(vec![
            tool_call_frame(answered.0, answered.1, answered.2),
            SidecarFrame::Output {
                query_id: 1,
                events: vec![
                    dispatched_event(answered.0, answered.1, answered.2),
                    SidecarEvent::ToolCall {
                        call_id: "call-1".into(),
                        name: "speek".into(),
                        arguments: "{}".into(),
                        dispatched: false,
                    },
                ],
                receipt: material(),
            },
        ]);
        let port = SdkInferencePort::new(link, TEST_RUNTIME);
        let prepared = seed_prepared(&port, "Author the shortfall.");
        port.lend_tool_results(
            &prepared,
            Box::new(ElaborationOracle::new(&[], SEED_ROUND_BUDGET)),
        );
        let output = port
            .infer(prepared)
            .await
            .expect("an invented name is a gap, not a violation");
        assert_eq!(output.events.len(), 2);
    }

    /// Spec test 11. The digest is a pure function of the receipt, so it is
    /// compared directly rather than through two whole queries.
    #[tokio::test]
    async fn a_receipt_digest_is_deterministic_and_bound_to_the_request() {
        let port = SdkInferencePort::new(ScriptedLink::new(Vec::new()), TEST_RUNTIME);
        let prepared = prose_prepared(&port, "Say something true.");
        let events = vec![SidecarEvent::Text { text: "ok".into() }];
        let first = assemble_output(&prepared, events.clone(), material(), &[]).unwrap();
        let again = assemble_output(&prepared, events.clone(), material(), &[]).unwrap();
        assert_eq!(first.receipt_digest, again.receipt_digest);
        assert_eq!(first.receipt_digest.len(), "sha256:".len() + 64);
        assert!(
            first.receipt_digest["sha256:".len()..]
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        );
        assert!(
            first
                .clone()
                .prose_only(InferencePurpose::Persona)
                .is_ok_and(|(prose, _)| prose == "ok")
        );

        let mut other_session = material();
        other_session.session_id = "session-two".into();
        assert_ne!(
            first.receipt_digest,
            assemble_output(&prepared, events.clone(), other_session, &[])
                .unwrap()
                .receipt_digest
        );
        let mut other_usage = material();
        other_usage.usage[0].output_tokens += 1;
        assert_ne!(
            first.receipt_digest,
            assemble_output(&prepared, events.clone(), other_usage, &[])
                .unwrap()
                .receipt_digest
        );
        let other_request = prose_prepared(&port, "Say something else.");
        assert_ne!(
            first.receipt_digest,
            assemble_output(&other_request, events, material(), &[])
                .unwrap()
                .receipt_digest
        );
    }

    /// Spec test 14.
    #[tokio::test]
    async fn an_expired_invocation_is_recovery_required() {
        let link = ScriptedLink::new(Vec::new());
        let port = SdkInferencePort::new(link.clone(), TEST_RUNTIME);
        let stale = prepare_invocation(TEST_RUNTIME, 1_000, prose_request("Say something true."))
            .expect("the stale invocation builds");
        let fault = port
            .infer(stale)
            .await
            .expect_err("a stale invocation re-ran");
        assert!(fault.recovery_required(), "{fault:?}");
        assert!(
            link.sent().is_empty(),
            "a stale invocation reached the pipe"
        );
    }

    /// Spec test 15.
    #[tokio::test]
    async fn the_oracle_error_aborts_the_query() {
        let granted = vec![AffordanceSnapshot {
            id: AffordanceId::issue(),
            entry: kernel_speak_entry(),
        }];
        let speak = granted[0].entry.kind.0.clone();
        let link = ScriptedLink::new(vec![
            tool_call_frame("call-0", &speak, r#"{"text":"Hold the bridge."}"#),
            tool_call_frame("call-1", "finish_without_proposal", "{}"),
            SidecarFrame::Output {
                query_id: 1,
                events: Vec::new(),
                receipt: material(),
            },
        ]);
        let port = SdkInferencePort::new(link.clone(), TEST_RUNTIME);
        let request = tool_request(
            CommandId::new(),
            0,
            InferencePurpose::OperationalAgent,
            TEST_MODEL,
            "Use only the supplied tools.",
            vec![CodexInputItem::UserText {
                text: "Hold the bridge.".into(),
            }],
            catalog_tools("", &granted),
            RequestShape {
                max_output_tokens: 1_200,
                parallel_tool_calls: false,
            },
        )
        .expect("the operational request builds");
        let prepared = port.prepare(request).expect("the SDK port prepares");
        port.lend_tool_results(&prepared, Box::new(OperationalOracle::new(&granted, &[])));
        let fault = port
            .infer(prepared)
            .await
            .expect_err("a contradicted terminal produced an output");
        assert!(fault.recovery_required(), "{fault:?}");
        assert_eq!(link.restarts(), 1);
        assert_eq!(
            link.sent()
                .into_iter()
                .filter(|frame| matches!(frame, SidecarFrame::ToolResult { .. }))
                .count(),
            1,
            "the port answered a call it had no string for"
        );
    }

    /// Spec test 12.
    #[test]
    fn a_model_prefix_routes_a_lane_to_its_port() {
        let sdk = Arc::new(SdkInferencePort::new(
            ScriptedLink::new(Vec::new()),
            TEST_RUNTIME,
        )) as Arc<dyn InferencePort>;
        let connector = Arc::new(SdkInferencePort::new(
            ScriptedLink::new(Vec::new()),
            "ghostlight-connector-stand-in",
        )) as Arc<dyn InferencePort>;
        let routed = RoutedInferencePort::new(
            Some(Arc::clone(&connector)),
            Some(Arc::clone(&sdk)),
            DEFAULT_SDK_MODEL_PREFIX,
        );
        assert!(Arc::ptr_eq(routed.route("claude-opus-5").unwrap(), &sdk));
        assert!(Arc::ptr_eq(
            routed.route("gpt-5.6-terra").unwrap(),
            &connector
        ));
        let moved =
            RoutedInferencePort::new(Some(Arc::clone(&connector)), Some(Arc::clone(&sdk)), "gpt-");
        assert!(Arc::ptr_eq(moved.route("gpt-5.6-terra").unwrap(), &sdk));
        assert!(Arc::ptr_eq(
            moved.route("claude-opus-5").unwrap(),
            &connector
        ));
    }

    fn models(model: &str) -> ControllerModels {
        ControllerModels {
            projector: model.into(),
            persona: model.into(),
            interpreter: model.into(),
            operational_agent: model.into(),
            elaborator: model.into(),
        }
    }

    /// Spec test 13.
    #[test]
    fn an_unroutable_model_fails_at_open() {
        let directory = tempfile::tempdir().unwrap();
        let entry = directory.path().join("main.js");
        std::fs::write(&entry, "// sidecar").unwrap();
        let key = directory.path().join("controller.key");
        std::fs::write(&key, "sdk-port-test-key").unwrap();
        let sdk = || SdkBinding {
            sidecar_entry: entry.clone(),
            caller_runtime_id: TEST_RUNTIME.into(),
            model_prefix: DEFAULT_SDK_MODEL_PREFIX.into(),
        };
        let connector = || ConnectorBinding {
            endpoint: "127.0.0.1:9".parse().unwrap(),
            key_path: key.clone(),
            caller_runtime_id: TEST_RUNTIME.into(),
        };

        assert!(matches!(
            open_inference(None, Some(sdk()), &models("gpt-5.6-terra")),
            Err(ControllerOpenError::UnroutableModel { .. })
        ));
        assert!(matches!(
            open_inference(Some(connector()), None, &models("claude-opus-5")),
            Err(ControllerOpenError::UnroutableModel { .. })
        ));
        assert!(matches!(
            open_inference(
                None,
                Some(SdkBinding {
                    sidecar_entry: directory.path().join("absent.js"),
                    caller_runtime_id: TEST_RUNTIME.into(),
                    model_prefix: DEFAULT_SDK_MODEL_PREFIX.into(),
                }),
                &models(TEST_MODEL),
            ),
            Err(ControllerOpenError::SdkSidecarMissing { .. })
        ));
        assert!(
            open_inference(Some(connector()), Some(sdk()), &models(TEST_MODEL)).is_ok(),
            "a configuration where every model routes did not open"
        );
    }

    /// Soul: the port's `prepare` is `prepare_invocation` and nothing else, so
    /// everything but the expiry stamp is identical to what any other port
    /// would have written for the same request.
    #[test]
    fn soul_the_sdk_port_prepares_the_identity_prepare_invocation_owns() {
        let port = SdkInferencePort::new(ScriptedLink::new(Vec::new()), TEST_RUNTIME);
        let request = prose_request("Say something true.");
        let through_port = port.prepare(request.clone()).expect("the port prepares");
        let through_owner = prepare_invocation(TEST_RUNTIME, 4_102_444_800_000, request.clone())
            .expect("the owner prepares");
        assert_eq!(through_port.purpose, through_owner.purpose);
        assert_eq!(
            through_port.invocation.request,
            through_owner.invocation.request
        );
        assert_eq!(
            through_port.invocation.native_request_sha256,
            through_owner.invocation.native_request_sha256
        );
        assert_eq!(
            through_port.invocation.provider_request_sha256,
            through_owner.invocation.provider_request_sha256
        );
        assert_eq!(
            through_port.invocation.caller_runtime_id,
            through_owner.invocation.caller_runtime_id
        );
        assert_eq!(
            through_port.invocation.request.request_id,
            through_owner.invocation.request.request_id
        );
    }

    /// Soul: a frame naming a kind or a fault reason the wire does not carry is
    /// refused by the codec, before the driver can act on a half-understood
    /// frame. The closed enums are the gate, and this proves they are closed.
    #[tokio::test]
    async fn soul_an_unknown_kind_or_fault_reason_is_refused_at_read_frame() {
        #[derive(Serialize)]
        struct Bogus<'a> {
            kind: &'a str,
            query_id: u64,
            reason: &'a str,
            detail: &'a str,
        }
        async fn refuse(body: Vec<u8>) -> InferenceFault {
            let mut framed = (body.len() as u32).to_be_bytes().to_vec();
            framed.extend_from_slice(&body);
            let mut reader = std::io::Cursor::new(framed);
            link_fault(
                read_frame(&mut reader)
                    .await
                    .expect_err("an unknown shape decoded"),
            )
        }
        // A kind that is not on the wire.
        let unknown_kind = rmp_serde::to_vec_named(&Bogus {
            kind: "hallucination",
            query_id: 1,
            reason: "rate_limited",
            detail: "",
        })
        .unwrap();
        assert!(refuse(unknown_kind).await.integrity_was_violated());
        // A fault whose reason the disposition table does not name. Nothing may
        // reach `into_fault` that the table has no row for.
        let unknown_reason = rmp_serde::to_vec_named(&Bogus {
            kind: "fault",
            query_id: 1,
            reason: "the_world_should_be_quarantined",
            detail: "",
        })
        .unwrap();
        assert!(refuse(unknown_reason).await.integrity_was_violated());
    }

    /// Soul: every reason the sidecar may name maps to the disposition the
    /// plan's table assigns, and no TypeScript-named reason can quarantine the
    /// world except the three the sidecar's own gates raise.
    #[test]
    fn soul_every_sidecar_reason_carries_its_named_disposition() {
        use SidecarFaultReason::*;
        for reason in [RateLimited, Overloaded, ServerError, ApiTimeout] {
            let fault = reason.into_fault("detail".into());
            assert!(
                !fault.recovery_required() && !fault.integrity_was_violated(),
                "{reason:?} was not retryable"
            );
        }
        for reason in [
            AuthenticationFailed,
            OrgNotAllowed,
            BillingError,
            InvalidRequest,
            ModelNotFound,
            MaxOutputTokens,
            MaxBudgetUsd,
            ExecutionError,
            Unknown,
        ] {
            assert!(
                reason.into_fault("detail".into()).recovery_required(),
                "{reason:?} was not recovery-required"
            );
        }
        for reason in [ProtocolViolation, ToolRegistrationFailed, TurnCapRefused] {
            assert!(
                reason.into_fault("detail".into()).integrity_was_violated(),
                "{reason:?} did not violate integrity"
            );
        }
    }

    /// Soul: the lend gate refuses both mismatches, in both directions.
    #[tokio::test]
    async fn soul_a_lend_that_does_not_match_the_request_is_refused() {
        // A tool request with no oracle lent.
        let port = SdkInferencePort::new(ScriptedLink::new(Vec::new()), TEST_RUNTIME);
        let prepared = seed_prepared(&port, "Author the shortfall.");
        let fault = port
            .infer(prepared)
            .await
            .expect_err("a tool request ran without its lane's owner");
        assert!(fault.integrity_was_violated(), "{fault:?}");

        // A prose request with an oracle lent.
        let port = SdkInferencePort::new(ScriptedLink::new(Vec::new()), TEST_RUNTIME);
        let prepared = prose_prepared(&port, "Say something true.");
        port.lend_tool_results(
            &prepared,
            Box::new(ElaborationOracle::new(&[], SEED_ROUND_BUDGET)),
        );
        let fault = port
            .infer(prepared)
            .await
            .expect_err("a prose request ran with a tool-result owner");
        assert!(fault.integrity_was_violated(), "{fault:?}");
    }

    /// Soul: a lend whose `infer` is refused before the oracle is taken leaves
    /// the entry in the map. The port's own doc says the entry is removed on
    /// every exit path of `infer`; the caller-identity and expiry gates run
    /// before the take, so it is not. Bounded by the request id's uniqueness
    /// and therefore not a leak that grows, but the claim as written is false.
    #[tokio::test]
    async fn soul_a_lend_refused_before_the_take_is_not_reclaimed() {
        let port = SdkInferencePort::new(ScriptedLink::new(Vec::new()), TEST_RUNTIME);
        let stale = prepare_invocation(TEST_RUNTIME, 1_000, prose_request("Say something true."))
            .expect("the stale invocation builds");
        port.lend_tool_results(
            &stale,
            Box::new(ElaborationOracle::new(&[], SEED_ROUND_BUDGET)),
        );
        assert!(port.infer(stale.clone()).await.is_err());
        assert_eq!(
            port.oracles.lock().unwrap().len(),
            1,
            "the expiry gate now reclaims the lend; update this test's claim"
        );

        // A caller-identity mismatch does the same.
        let other = SdkInferencePort::new(ScriptedLink::new(Vec::new()), "ghostlight-other");
        other.lend_tool_results(
            &stale,
            Box::new(ElaborationOracle::new(&[], SEED_ROUND_BUDGET)),
        );
        let fault = other
            .infer(stale)
            .await
            .expect_err("a foreign caller's invocation ran");
        assert!(fault.integrity_was_violated(), "{fault:?}");
        assert_eq!(other.oracles.lock().unwrap().len(), 1);

        // A completed query does reclaim its own lend.
        let link = ScriptedLink::new(vec![SidecarFrame::Output {
            query_id: 1,
            events: Vec::new(),
            receipt: material(),
        }]);
        let port = SdkInferencePort::new(link, TEST_RUNTIME);
        let prepared = seed_prepared(&port, "Author the shortfall.");
        port.lend_tool_results(
            &prepared,
            Box::new(ElaborationOracle::new(&[], SEED_ROUND_BUDGET)),
        );
        port.infer(prepared).await.expect("the empty query returns");
        assert!(port.oracles.lock().unwrap().is_empty());
    }

    /// Soul: the receipt is bound to the request without carrying a word of it.
    /// Nothing the model was shown, and nothing it wrote, may reach a digest
    /// that is persisted as provenance.
    #[test]
    fn soul_the_receipt_carries_no_prompt_or_tool_text() {
        let port = SdkInferencePort::new(ScriptedLink::new(Vec::new()), TEST_RUNTIME);
        let secret = "the rain has teeth tonight";
        let prepared = prose_prepared(&port, secret);
        let receipt = SdkInferenceReceipt {
            schema_id: SDK_RECEIPT_SCHEMA.to_owned(),
            request_id: prepared.invocation.request.request_id.clone(),
            conversation_id: prepared.invocation.request.conversation_id.clone(),
            caller_runtime_id: prepared.invocation.caller_runtime_id.clone(),
            native_request_sha256: prepared.invocation.native_request_sha256,
            provider_request_sha256: prepared.invocation.provider_request_sha256,
            model: prepared.invocation.request.model.clone(),
            session_id: material().session_id,
            result_uuid: material().result_uuid,
            subtype: material().subtype,
            stop_reason: material().stop_reason,
            num_turns: material().num_turns,
            assistant_message_uuids: material().assistant_message_uuids,
            assistant_request_ids: material().assistant_request_ids,
            usage: material().usage,
            total_cost_usd_estimate: material().total_cost_usd_estimate,
        };
        let bytes = rmp_serde::to_vec_named(&receipt).unwrap();
        let rendered = String::from_utf8_lossy(&bytes);
        assert!(!rendered.contains(secret), "the receipt carries the prompt");
        assert!(
            !rendered.contains(&prepared.invocation.request.instructions),
            "the receipt carries the instructions"
        );
        // And the digest the output carries is the digest of exactly this.
        let output = assemble_output(
            &prepared,
            vec![SidecarEvent::Text { text: "ok".into() }],
            material(),
            &[],
        )
        .unwrap();
        assert_eq!(
            output.receipt_digest,
            format!("sha256:{:x}", Sha256::digest(&bytes))
        );
        // A different stop reason is a different receipt.
        let mut stopped = material();
        stopped.stop_reason = Some("max_tokens".into());
        assert_ne!(
            output.receipt_digest,
            assemble_output(
                &prepared,
                vec![SidecarEvent::Text { text: "ok".into() }],
                stopped,
                &[]
            )
            .unwrap()
            .receipt_digest
        );
    }

    /// Soul: the order of the dispatched calls is part of the gate, not just
    /// the set. A sidecar that answered the same two calls in the other order
    /// put a different string in front of the model than the evaluator will
    /// recompute.
    #[tokio::test]
    async fn soul_dispatched_calls_out_of_order_are_refused() {
        let calls = [
            ("call-0", RECORD_GAP_PATCH_TOOL, r#"{"detail":"one"}"#),
            ("call-1", RECORD_GAP_PATCH_TOOL, r#"{"detail":"two"}"#),
        ];
        let mut outbound: Vec<SidecarFrame> = calls
            .iter()
            .map(|(id, name, arguments)| tool_call_frame(id, name, arguments))
            .collect();
        outbound.push(SidecarFrame::Output {
            query_id: 1,
            events: vec![
                dispatched_event(calls[1].0, calls[1].1, calls[1].2),
                dispatched_event(calls[0].0, calls[0].1, calls[0].2),
            ],
            receipt: material(),
        });
        let link = ScriptedLink::new(outbound);
        let port = SdkInferencePort::new(link, TEST_RUNTIME);
        let prepared = seed_prepared(&port, "Author the shortfall.");
        port.lend_tool_results(
            &prepared,
            Box::new(ElaborationOracle::new(&[], SEED_ROUND_BUDGET)),
        );
        let fault = port
            .infer(prepared)
            .await
            .expect_err("a reordered dispatch produced an output");
        assert!(fault.integrity_was_violated(), "{fault:?}");
    }

    /// Soul: the lowering's own gate table, every row.
    #[test]
    fn soul_lower_query_refuses_the_shapes_the_gate_table_names() {
        let port = SdkInferencePort::new(ScriptedLink::new(Vec::new()), TEST_RUNTIME);
        // `lower_query` reads the provider request off the prepared value, so
        // the shapes it must refuse are written there directly. Nothing here
        // asserts a digest; the identity tests own that.
        let build = |input: Vec<CodexInputItem>, effort: Option<&str>| {
            let mut prepared = prose_prepared(&port, "Say something true.");
            prepared.invocation.request.input = input;
            prepared.invocation.request.reasoning_effort = effort.map(str::to_owned);
            prepared
        };
        let user = |text: &str| CodexInputItem::UserText { text: text.into() };

        // Every effort the SDK admits passes; anything else is refused.
        for effort in SDK_EFFORT_LEVELS {
            assert!(lower_query(1, &build(vec![user("go")], Some(effort)), 1).is_ok());
        }
        let fault = lower_query(1, &build(vec![user("go")], Some("blistering")), 1)
            .expect_err("an unmapped effort lowered");
        assert!(fault.integrity_was_violated(), "{fault:?}");

        // The first item must be user text, and it must be the only one.
        for input in [
            vec![CodexInputItem::AssistantText { text: "no".into() }],
            Vec::new(),
            vec![user("go"), user("again")],
        ] {
            let fault = lower_query(1, &build(input, None), 1)
                .expect_err("a request that opens wrong lowered");
            assert!(fault.integrity_was_violated(), "{fault:?}");
        }

        // A prior round renders exactly as the sidecar's fixed header expects.
        let frame = lower_query(
            9,
            &build(
                vec![
                    user("go"),
                    CodexInputItem::AssistantText {
                        text: "thinking".into(),
                    },
                    CodexInputItem::ToolCall {
                        call_id: "call-0".into(),
                        name: "record_gap".into(),
                        arguments: "{}".into(),
                    },
                    CodexInputItem::ToolResult {
                        call_id: "call-0".into(),
                        output: "gap recorded".into(),
                    },
                ],
                Some("medium"),
            ),
            4,
        )
        .expect("a multi-item request lowers");
        let SidecarFrame::Query {
            query_id,
            prompt,
            transcript,
            turn_cap,
            effort,
            ..
        } = frame
        else {
            panic!("the lowering produced something other than a query")
        };
        assert_eq!(
            (query_id, turn_cap, effort.as_deref()),
            (9, 4, Some("medium"))
        );
        assert_eq!(prompt, "go");
        assert_eq!(
            transcript,
            vec![
                "assistant: thinking".to_string(),
                "tool call record_gap: {}".to_string(),
                "tool result: gap recorded".to_string(),
            ]
        );
    }

    /// Soul: an unroutable model is refused at `infer` as well as at open, and
    /// an SDK-only deployment needs no connector and therefore no credential.
    #[tokio::test]
    async fn soul_routing_refuses_rather_than_falling_back() {
        assert_eq!(DEFAULT_SDK_MODEL_PREFIX, "claude");
        let connector_only = RoutedInferencePort::new(
            Some(Arc::new(SdkInferencePort::new(
                ScriptedLink::new(Vec::new()),
                TEST_RUNTIME,
            )) as Arc<dyn InferencePort>),
            None,
            DEFAULT_SDK_MODEL_PREFIX,
        );
        assert!(connector_only.route(TEST_MODEL).is_none());
        let prepared =
            prepare_invocation(TEST_RUNTIME, 4_102_444_800_000, prose_request("Say it.")).unwrap();
        for fault in [
            connector_only
                .prepare(prose_request("Say it."))
                .expect_err("an unroutable model prepared"),
            connector_only
                .infer(prepared)
                .await
                .expect_err("an unroutable model inferred"),
        ] {
            assert!(fault.integrity_was_violated(), "{fault:?}");
            assert!(fault.to_string().contains(TEST_MODEL));
        }

        // An SDK-only deployment opens with no connector binding at all, so no
        // credential path is read.
        let directory = tempfile::tempdir().unwrap();
        let entry = directory.path().join("main.js");
        std::fs::write(&entry, "// sidecar").unwrap();
        assert!(
            open_inference(
                None,
                Some(SdkBinding {
                    sidecar_entry: entry,
                    caller_runtime_id: TEST_RUNTIME.into(),
                    model_prefix: DEFAULT_SDK_MODEL_PREFIX.into(),
                }),
                &models(TEST_MODEL),
            )
            .is_ok()
        );
    }

    /// Soul: Ghostlight holds no credential, so no credential name appears in
    /// the code that opens, routes, or drives this transport.
    #[test]
    fn soul_no_credential_name_appears_in_the_ports_own_source() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
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
        let mut sources = Vec::new();
        for file in [
            "world/sdk_inference.rs",
            "world/controllers.rs",
            "runtime.rs",
        ] {
            let text = std::fs::read_to_string(root.join(file))
                .expect("the source reads")
                .replace("\r\n", "\n");
            // Only the production half; a test may name what production must
            // not.
            let production = text
                .split_once("\n#[cfg(test)]\nmod tests {")
                .map(|(before, _)| before.to_owned())
                .unwrap_or(text);
            for needle in &needles {
                assert!(
                    !production.contains(needle.as_str()),
                    "{file} names {needle}"
                );
            }
            // `ANTHROPIC_API_KEY` is named once, in the port's doc comment, as
            // the condition under which this port is deleted. It must appear
            // nowhere that could read it.
            let anthropic = format!("ANTHROPIC{}", "_");
            for line in production.lines() {
                if line.contains(anthropic.as_str()) {
                    assert!(
                        line.trim_start().starts_with("///") || line.trim_start().starts_with("//"),
                        "{file} names an ANTHROPIC variable outside a comment: {line}"
                    );
                }
            }
            sources.push((file, production));
        }
        // The one credential path that does exist belongs to the connector and
        // is read only when a connector binding is built.
        assert_eq!(
            sources[1].1.matches("from_secret_file").count(),
            2,
            "the connector key is read somewhere new"
        );
        // The sidecar child inherits the ambient environment and is given none.
        assert!(
            !sources[0].1.contains(".env("),
            "the port sets a child environment"
        );
        assert!(!sources[0].1.contains("env_clear"));
    }

    /// The Rust half of the sidecar's schema-grammar pair (spec test 17):
    /// publishes every `parameters_json` the two catalogs actually emit, so the
    /// sidecar's closed converter is tested against real strings rather than
    /// hand-written ones.
    #[test]
    fn schema_fixtures_match_the_checked_in_sidecar_grammar() {
        let granted = vec![AffordanceSnapshot {
            id: AffordanceId::issue(),
            entry: kernel_speak_entry(),
        }];
        let mut emitted: Vec<(String, String)> = patch_tools()
            .into_iter()
            .chain(catalog_tools("", &granted))
            .chain(catalog_tools("c3__", &granted))
            .map(|tool| (tool.name, tool.parameters_json))
            .collect();
        emitted.sort();
        emitted.dedup();
        let rendered = serde_json::to_string_pretty(
            &emitted
                .into_iter()
                .map(
                    |(name, schema)| serde_json::json!({ "name": name, "parameters_json": schema }),
                )
                .collect::<Vec<_>>(),
        )
        .unwrap();
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../sidecar/claude-sdk/test/schemas.json");
        if std::env::var_os("GHOSTLIGHT_WRITE_SIDECAR_FIXTURES").is_some() {
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(&path, format!("{rendered}\n")).unwrap();
        }
        let on_disk = std::fs::read_to_string(&path).unwrap_or_else(|error| {
            panic!(
                "the sidecar schema fixture {} is missing ({error}); regenerate with GHOSTLIGHT_WRITE_SIDECAR_FIXTURES=1",
                path.display()
            )
        });
        assert_eq!(
            on_disk.replace("\r\n", "\n"),
            format!("{rendered}\n"),
            "the emitted tool schemas drifted from the sidecar's converter fixture"
        );
    }

    /// The Rust half of the sidecar's frame round-trip pair (spec test 18):
    /// pins the exact `rmp_serde::to_vec_named` bytes the TypeScript decoder is
    /// tested against, so the two halves cannot drift apart silently.
    #[tokio::test]
    async fn frame_fixtures_match_the_checked_in_sidecar_bytes() {
        let fixtures: Vec<(&str, SidecarFrame)> = vec![
            (
                "query",
                SidecarFrame::Query {
                    query_id: 7,
                    model: TEST_MODEL.into(),
                    instructions: "Author the shortfall.".into(),
                    prompt: "Answer the deficit.".into(),
                    transcript: vec!["assistant: thinking".into(), "tool result: ok".into()],
                    tools: vec![SidecarTool {
                        name: "submit".into(),
                        description: "Submit the draft.".into(),
                        parameters_json: r#"{"type":"object","additionalProperties":false,"required":[],"properties":{}}"#
                            .into(),
                    }],
                    effort: Some("medium".into()),
                    max_output_tokens: Some(4_000),
                    turn_cap: 24,
                },
            ),
            (
                "tool_call",
                SidecarFrame::ToolCall {
                    query_id: 7,
                    call_id: "call-0".into(),
                    name: "submit".into(),
                    arguments: "{}".into(),
                },
            ),
            (
                "tool_result",
                SidecarFrame::ToolResult {
                    query_id: 7,
                    call_id: "call-0".into(),
                    output: "patch submitted".into(),
                },
            ),
            (
                "output",
                SidecarFrame::Output {
                    query_id: 7,
                    events: vec![
                        SidecarEvent::Text {
                            text: "Done.".into(),
                        },
                        dispatched_event("call-0", "submit", "{}"),
                    ],
                    receipt: material(),
                },
            ),
            (
                "fault",
                SidecarFrame::Fault {
                    query_id: 7,
                    reason: SidecarFaultReason::RateLimited,
                    detail: "429".into(),
                },
            ),
        ];
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../sidecar/claude-sdk/test/frames");
        for (name, frame) in fixtures {
            let bytes = encode_frame(&frame).expect("the fixture frame encodes");
            let path = root.join(format!("{name}.bin"));
            if std::env::var_os("GHOSTLIGHT_WRITE_SIDECAR_FIXTURES").is_some() {
                std::fs::create_dir_all(&root).unwrap();
                std::fs::write(&path, &bytes).unwrap();
            }
            let on_disk = std::fs::read(&path).unwrap_or_else(|error| {
                panic!(
                    "the sidecar frame fixture {} is missing ({error}); regenerate with GHOSTLIGHT_WRITE_SIDECAR_FIXTURES=1",
                    path.display()
                )
            });
            assert_eq!(
                bytes, on_disk,
                "the sidecar frame fixture {name} drifted from the Rust encoder"
            );
            let mut reader = std::io::Cursor::new(on_disk);
            assert_eq!(
                read_frame(&mut reader).await.expect("the fixture decodes"),
                frame
            );
        }
    }
}
