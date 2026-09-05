//! The two cognition modes behind one exact world opportunity.
//!
//! This organ may ask models to think. It cannot mint authority: it reads one
//! `WorldMailbox` snapshot, binds one controller-owned opportunity, and submits
//! one typed exercise or decline through that same mailbox. The model-facing
//! tools never carry caller, controller, world, opportunity, revision, or
//! affordance fields.

use super::elaboration::{
    ElaborationCheckpoint, ElaborationRunner, EvidenceSource, NullEvidenceSource, SeedCheckpoint,
    SeedRunner, valid_elaboration_progression, valid_seed_progression,
};
use super::sdk_inference::{
    ChildProcessLink, DEFAULT_SDK_MODEL_PREFIX, RoutedInferencePort, SdkBinding, SdkInferencePort,
};
use super::tool_schema;
use crate::world::{
    AffordanceId, AffordanceSnapshot, AuthorityGrant, Bounds, Cell, CellId, CommandId,
    CommitReceipt, Confidence, Constituent, ControllerMode, ControllerPort, Cost,
    DecisionInvocation, DecisionOpportunity, DependencyTarget, EdgeId, ElaborationPort, EntityId,
    EntityKind, FactStandingView, KernelError, KnowledgeSnapshot, KnowledgeSource, Magnitude,
    MailboxError, OfficeSnapshot, ProposedEffect, Quantity, RefKind, Resolution, RoleBinding,
    ScopeComponents, ScopeDigest, SeedPort, Statement, SubjectId, SubjectSnapshot, SubmitReceipt,
    Target, TickIndex, WorldMailbox, WorldSnapshot,
};
use async_trait::async_trait;
use chrono::Utc;
use codex_connector::{
    CodexConnectorClient, CodexConnectorClientError, CodexInputItem, CodexProviderRequest,
    CodexRefusal, CodexToolChoice, CodexToolDefinition, CodexTransportDisposition,
    CodexTransportEventPayload, CodexTransportInvocation, CodexTransportOutcome,
    provider_request_sha256,
};
use cultcache_rs::{CacheBackingStore, CultCacheEnvelope, OwnedRedbMessagePackBackingStore};
use ghostlight_persona_projection::{
    CaptureToolFeedback, GroupedAgentPrompt, InterpretationAccumulator, InterpretationFinalization,
    InterpretationReport, InterpreterPrompt, LabeledView, OperationalAgentPrompt, PersonaPrompt,
    PersonaTurn, PersonaTurnBinding, ProjectorPrompt, RecordGapToolCall, TranslationGapKind,
    build_grouped_agent_prompt, build_interpreter_prompt, build_operational_agent_prompt,
    build_persona_prompt, build_projector_prompt, sha256,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    net::SocketAddr,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use thiserror::Error;
use zeroize::Zeroizing;

const MAX_CONNECTOR_FRAME_BYTES: usize = 1_052_672;
/// How far ahead an invocation may claim to be valid. The connector refuses
/// an expiry beyond its own skew bound as `Expired`, so this is the daemon's
/// number, not ours; it is admission validity, not generation time.
pub(super) const REQUEST_EXPIRY: Duration = Duration::from_secs(300);
/// One provider round's whole lifetime as seen from this side of the socket:
/// the connector replies once at the end, so the read timeout bounds an
/// entire generation, and a seed patch at medium effort was measured past
/// five minutes.
pub(super) const RESPONSE_TIMEOUT: Duration = Duration::from_secs(900);
pub(super) const TOOL_STEP_BUDGET: usize = 4;
/// The grouped protocol asks for every call in one round, so this budget buys
/// exactly one repair round after decode gaps are reported back. It deliberately
/// does not scale with the cell: a cell that needs three rounds of repair is a
/// cell that should have been smaller.
pub(super) const CELL_TOOL_STEP_BUDGET: usize = 2;
/// Separates a constituent handle from the tool it names. Attribution is
/// carried by tool identity, never by a model-written argument.
const HANDLE_SEPARATOR: &str = "__";
const PERSONA_WORD_BUDGET: usize = 180;
const CONTROLLER_WORK_ROW: &str = "controller_work.v11";
const CONTROLLER_WORK_SCHEMA: &str = "ghostlight.controller_work.v11";

/// The Interpreter's byte-span capture tool. It is not the generated `speak`
/// affordance tool: one captures an utterance out of preserved prose, the other
/// is a projection of a catalog entry.
const INTERPRETER_SPEAK_TOOL: &str = "speak";

/// The kind name the narrative lane looks for among its granted entries. The
/// kernel carries the name and branches on it nowhere; matching on it here is a
/// consumer reading data, which is where genre belongs.
const SPEAK_KIND: &str = "speak";
const INTERPRETER_RECORD_GAP_TOOL: &str = "record_gap";
const RECORD_NEED_TOOL: &str = "record_need";
const FINISH_INTERPRETATION_TOOL: &str = "finish_interpretation";
const FINISH_WITHOUT_PROPOSAL_TOOL: &str = "finish_without_proposal";
const PERSONA_PROVIDER_INSTRUCTIONS: &str =
    "Respond only in natural prose to the lived moment in the user message.";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum InferencePurpose {
    Projector,
    Persona,
    Interpreter,
    OperationalAgent,
    /// One inference over a cell's labeled constituent views. Distinct from
    /// `OperationalAgent` so a grouped request's derived ids can never collide
    /// with a detail turn's, and so the output budget and the parallel-call
    /// shape a cell needs do not leak into the singleton path.
    GroupedAgent,
    Elaboration,
}

/// One exact provider request. Keeping the native request visible at this seam
/// makes the prose-only Persona boundary structurally inspectable.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct InferenceRequest {
    purpose: InferencePurpose,
    provider: CodexProviderRequest,
}

impl InferenceRequest {
    /// The model this request names, read before it is prepared. Routing is the
    /// one decision that has to be made on an unprepared request.
    pub(super) fn provider_model(&self) -> &str {
        &self.provider.model
    }
}

/// The exact connector invocation, including expiry and native provenance.
/// Replaying this value may recover a completed connector response; rebuilding
/// it under the same request ID would be a replay conflict.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct PreparedInference {
    pub(crate) purpose: InferencePurpose,
    pub(super) invocation: CodexTransportInvocation,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum InferenceEvent {
    Text(String),
    ToolCall {
        call_id: String,
        name: String,
        arguments: String,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct InferenceOutput {
    pub(super) events: Vec<InferenceEvent>,
    pub(super) receipt_digest: String,
}

impl InferenceOutput {
    pub(super) fn prose_only(
        self,
        purpose: InferencePurpose,
    ) -> Result<(String, String), ControllerError> {
        let mut prose = String::new();
        for event in self.events {
            match event {
                InferenceEvent::Text(text) => prose.push_str(&text),
                InferenceEvent::ToolCall { .. } => {
                    return Err(ControllerError::ProviderContract {
                        purpose,
                        detail: "a prose-only request returned a tool call".into(),
                    });
                }
            }
        }
        if self.receipt_digest.is_empty() || self.receipt_digest.trim() != self.receipt_digest {
            return Err(ControllerError::ProviderContract {
                purpose,
                detail: "provider output has no receipt digest".into(),
            });
        }
        Ok((prose, self.receipt_digest))
    }
}

#[derive(Clone, Debug, Error, PartialEq, Eq)]
#[error("{detail}")]
pub(crate) struct InferenceFault {
    disposition: InferenceFaultDisposition,
    detail: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum InferenceFaultDisposition {
    Retryable,
    RecoveryRequired,
    IntegrityViolation,
}

impl InferenceFault {
    pub(super) fn new(detail: impl Into<String>) -> Self {
        Self {
            disposition: InferenceFaultDisposition::RecoveryRequired,
            detail: detail.into(),
        }
    }

    pub(super) fn retryable(detail: impl Into<String>) -> Self {
        Self {
            disposition: InferenceFaultDisposition::Retryable,
            detail: detail.into(),
        }
    }

    pub(super) fn integrity_violation(detail: impl Into<String>) -> Self {
        Self {
            disposition: InferenceFaultDisposition::IntegrityViolation,
            detail: detail.into(),
        }
    }

    pub(super) fn recovery_required(&self) -> bool {
        self.disposition == InferenceFaultDisposition::RecoveryRequired
    }

    pub(super) fn integrity_was_violated(&self) -> bool {
        self.disposition == InferenceFaultDisposition::IntegrityViolation
    }

    /// The one disposition that makes `ControllerError::requires_quarantine`
    /// true for an `Inference` fault. A test port outside this module needs to
    /// raise exactly this to exercise the tick driver's quarantine edge, and
    /// this is that one legal way in.
    #[cfg(test)]
    pub(crate) fn fixture_integrity_violation(detail: impl Into<String>) -> Self {
        Self::integrity_violation(detail)
    }

    /// The non-quarantining counterpart: a test port outside this module that
    /// needs one purpose to fail without raising
    /// `ControllerError::requires_quarantine` (so a sibling cell in the same
    /// tick is unaffected) has no other legal way to build one.
    #[cfg(test)]
    pub(crate) fn fixture_recovery_required(detail: impl Into<String>) -> Self {
        Self::new(detail)
    }
}

/// One lane's own answer to "what does this tool call return", lent to the
/// inference port for exactly one query. A port that runs the tool loop needs a
/// real result before the model will take another turn, and the only correct
/// result is the one the lane's evaluator will recompute when it re-derives the
/// finished round from `completed`. So this is not a second implementation: it
/// is the same fold, over the calls made so far, into fresh state. A port that
/// computed its own answer would put a string in front of the model that no
/// durable evidence records and no check can catch.
pub(crate) trait ToolResultOracle: Send {
    /// The lane's remaining round budget, lowered to a transport's turn cap so
    /// one query cannot exceed what the evaluator would have allowed.
    fn remaining_rounds(&self) -> u32;

    /// The next call in this query. Errors are the lane's own hard contract
    /// errors; the port aborts the query rather than inventing a string.
    fn answer(&mut self, name: &str, arguments: &str) -> Result<String, ControllerError>;
}

#[async_trait]
pub(crate) trait InferencePort: Send + Sync {
    fn prepare(&self, request: InferenceRequest) -> Result<PreparedInference, InferenceFault>;

    async fn infer(&self, request: PreparedInference) -> Result<InferenceOutput, InferenceFault>;

    /// Lends a lane's tool-result owner to the port for one query, keyed by the
    /// prepared request's own id. The default ignores it: the connector returns
    /// tool calls inert and the evaluator computes every result at
    /// re-derivation, so only a port that runs the tool loop needs one.
    fn lend_tool_results(&self, _prepared: &PreparedInference, _oracle: Box<dyn ToolResultOracle>) {
    }
}

/// Builds a `PreparedInference` outside the real CodexConnector wiring. Exists
/// so a test port defined outside this module (`runtime`'s own spec tests, for
/// the tick driver's concurrency and quarantine behaviour) has one legal way to
/// answer `InferencePort::prepare` without reaching into `PreparedInference`'s
/// private fields or standing up a real connector.
#[cfg(test)]
pub(crate) fn fixture_prepared_inference(
    request: InferenceRequest,
) -> Result<PreparedInference, InferenceFault> {
    prepare_invocation("ghostlight-controller-test", 4_102_444_800_000, request)
}

/// The one place a `PreparedInference` is built. Every port calls it, so a
/// request prepared by one transport and a request prepared by another are the
/// same value under the same digests, and every `integrity_is_valid` variant
/// validates one identity scheme rather than two.
pub(super) fn prepare_invocation(
    caller_runtime_id: &str,
    expires_at_unix_ms: u64,
    request: InferenceRequest,
) -> Result<PreparedInference, InferenceFault> {
    let request_bytes =
        serde_json::to_vec(&request).map_err(|error| InferenceFault::new(error.to_string()))?;
    let purpose = request.purpose;
    let invocation = CodexTransportInvocation::new(
        caller_runtime_id,
        expires_at_unix_ms,
        Sha256::digest(request_bytes).into(),
        request.provider,
    )
    .map_err(|error| InferenceFault::new(error.to_string()))?;
    Ok(PreparedInference {
        purpose,
        invocation,
    })
}

/// The `infer` half of the same seam: a canned prose output a test port can
/// return without naming `InferenceOutput`'s or `InferenceEvent`'s private
/// fields.
#[cfg(test)]
pub(crate) fn fixture_inference_output(text: impl Into<String>, receipt: &str) -> InferenceOutput {
    InferenceOutput {
        events: vec![InferenceEvent::Text(text.into())],
        receipt_digest: format!("sha256:{receipt}"),
    }
}

/// The same seam as `fixture_inference_output`, for a test port outside this
/// module that needs to answer with something other than plain prose — an
/// Interpreter's `speak`/`finish_interpretation` tool calls, in particular.
#[cfg(test)]
pub(crate) fn fixture_inference_events(
    events: Vec<InferenceEvent>,
    receipt: &str,
) -> InferenceOutput {
    InferenceOutput {
        events,
        receipt_digest: format!("sha256:{receipt}"),
    }
}

/// Production lowering to CodexConnector. There is deliberately no retry,
/// fallback model, output repair, or stage registry here; transport failure is
/// returned to the owning controller flow with its pending evidence intact.
#[derive(Clone)]
struct CodexConnectorInferencePort {
    client: CodexConnectorClient,
    caller_runtime_id: String,
}

impl CodexConnectorInferencePort {
    fn new(
        endpoint: SocketAddr,
        connection_key: String,
        caller_runtime_id: impl Into<String>,
    ) -> Result<Self, InferenceFault> {
        let caller_runtime_id = caller_runtime_id.into();
        if caller_runtime_id.trim().is_empty() || caller_runtime_id.trim() != caller_runtime_id {
            return Err(InferenceFault::new(
                "CodexConnector caller runtime must be one exact nonempty identity",
            ));
        }
        let client = CodexConnectorClient::new(
            endpoint,
            connection_key,
            MAX_CONNECTOR_FRAME_BYTES,
            Some(RESPONSE_TIMEOUT),
        )
        .map_err(|error| InferenceFault::new(error.to_string()))?;
        Ok(Self {
            client,
            caller_runtime_id,
        })
    }

    fn from_secret_file(
        endpoint: SocketAddr,
        path: impl AsRef<Path>,
        caller_runtime_id: impl Into<String>,
    ) -> Result<Self, InferenceFault> {
        let bytes = Zeroizing::new(
            std::fs::read(path.as_ref()).map_err(|error| InferenceFault::new(error.to_string()))?,
        );
        let raw = std::str::from_utf8(bytes.as_slice())
            .map_err(|error| InferenceFault::new(error.to_string()))?;
        let key = raw.trim_end_matches(['\r', '\n']);
        if key.is_empty() || key.len() != raw.trim().len() {
            return Err(InferenceFault::new(
                "CodexConnector key file is empty or has surrounding whitespace",
            ));
        }
        Self::new(endpoint, key.to_owned(), caller_runtime_id)
    }

    fn execute(&self, request: PreparedInference) -> Result<InferenceOutput, InferenceFault> {
        if request.invocation.caller_runtime_id != self.caller_runtime_id {
            return Err(InferenceFault::integrity_violation(
                "persisted inference caller does not match the configured runtime identity",
            ));
        }
        let result = self
            .client
            .execute(&request.invocation)
            .map_err(|error| match error {
                CodexConnectorClientError::Connection(_) => {
                    InferenceFault::retryable(error.to_string())
                }
                CodexConnectorClientError::InvalidConfig
                | CodexConnectorClientError::FrameSize
                | CodexConnectorClientError::Encoding
                | CodexConnectorClientError::Transport(_) => {
                    InferenceFault::integrity_violation(error.to_string())
                }
            })?;
        let (events, receipt) = match result.disposition {
            CodexTransportDisposition::Refused(reason) => {
                let detail = format!("CodexConnector refused request: {reason:?}");
                return Err(match reason {
                    CodexRefusal::InFlight | CodexRefusal::Capacity => {
                        InferenceFault::retryable(detail)
                    }
                    CodexRefusal::Expired | CodexRefusal::Indeterminate => {
                        InferenceFault::new(detail)
                    }
                    CodexRefusal::IdentitySubstitution
                    | CodexRefusal::ProviderDigestSubstitution
                    | CodexRefusal::Policy
                    | CodexRefusal::ReplayConflict
                    | CodexRefusal::Malformed => InferenceFault::integrity_violation(detail),
                });
            }
            CodexTransportDisposition::Transported { events, receipt } => (events, receipt),
        };
        if let CodexTransportOutcome::Failed {
            failure_kind,
            message,
        } = &receipt.outcome
        {
            return Err(InferenceFault::new(format!(
                "Codex provider failed ({failure_kind}): {message}"
            )));
        }
        let events = events
            .into_iter()
            .map(|event| match event.payload {
                CodexTransportEventPayload::TextDelta { text } => InferenceEvent::Text(text),
                CodexTransportEventPayload::ToolCall {
                    call_id,
                    name,
                    arguments,
                } => InferenceEvent::ToolCall {
                    call_id,
                    name,
                    arguments,
                },
            })
            .collect();
        let receipt_bytes = rmp_serde::to_vec_named(&receipt)
            .map_err(|error| InferenceFault::new(error.to_string()))?;
        Ok(InferenceOutput {
            events,
            receipt_digest: format!("sha256:{:x}", Sha256::digest(&receipt_bytes)),
        })
    }
}

#[async_trait]
impl InferencePort for CodexConnectorInferencePort {
    fn prepare(&self, request: InferenceRequest) -> Result<PreparedInference, InferenceFault> {
        prepare_invocation(
            &self.caller_runtime_id,
            unix_ms()?.saturating_add(REQUEST_EXPIRY.as_millis() as u64),
            request,
        )
    }

    async fn infer(&self, request: PreparedInference) -> Result<InferenceOutput, InferenceFault> {
        let port = self.clone();
        tokio::task::spawn_blocking(move || port.execute(request))
            .await
            .map_err(|error| InferenceFault::new(error.to_string()))?
    }
}

pub(super) fn unix_ms() -> Result<u64, InferenceFault> {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| InferenceFault::new(error.to_string()))?
        .as_millis();
    u64::try_from(millis).map_err(|_| InferenceFault::new("system time exceeds u64 milliseconds"))
}

/// One constituent of a grouped cell, frozen at selection. It carries its own
/// opportunity, so its controller, scope, and authority are exactly what the
/// detail path would have submitted: grouping changes representation, never
/// authority. There is no `controller_mode` beside the opportunity's, because a
/// second copy of a digest-bound value is a value that can disagree.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ConstituentWork {
    subject: SubjectId,
    opportunity: DecisionOpportunity,
    granted: Vec<AffordanceSnapshot>,
    /// Derived from `(world, cell, subject, tick)`, so a resumed cell re-submits
    /// the same id and the kernel answers from its idempotency ledger.
    command_id: CommandId,
}

/// One cell, one inference, one row. A third variant rather than a widened
/// pair: making the twelve singleton progression comparisons vector-wise for a
/// path that is always length one would buy nothing, and it would put the
/// batched prompt in the same type as the byte-identical singleton one.
///
/// There is no persisted per-constituent outcome. The submission loop is
/// idempotent by construction — every constituent's command id is derived, and
/// `submit_controller_world` probes the kernel's receipt before committing — so
/// a resumed `Submitting` re-runs the loop and the ledger answers with the
/// original receipts. A stored outcome would be a second, weaker record of what
/// the ledger already owns.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "stage", rename_all = "snake_case")]
enum GroupedCheckpoint {
    AgentInFlight {
        command_id: CommandId,
        cell: CellId,
        tick: TickIndex,
        agent_prompt: String,
        constituents: Vec<ConstituentWork>,
        completed: Vec<InferenceOutput>,
        invocation: PreparedInference,
    },
    Submitting {
        command_id: CommandId,
        cell: CellId,
        tick: TickIndex,
        agent_prompt: String,
        constituents: Vec<ConstituentWork>,
        completed: Vec<InferenceOutput>,
    },
}

/// One command has one durable tagged cognition checkpoint. Every in-flight
/// variant owns the exact connector invocation before transport; terminal
/// variants retain completed provider outputs so their capture is derived,
/// never separately persisted.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "mode", content = "checkpoint", rename_all = "snake_case")]
pub(crate) enum ControllerWork {
    Narrative(NarrativeCheckpoint),
    Operational(OperationalCheckpoint),
    /// One coarse cell's cognition. The only durable trace a cover leaves, and
    /// it lives in the controller-work store, whose custody is separate from
    /// world custody: representation is cognition evidence, not world truth.
    Grouped(GroupedCheckpoint),
    /// The authoring lane. One store, one custody probe, one progression check:
    /// forking them would buy nothing and split one authority in three.
    Elaboration(ElaborationCheckpoint),
    /// Draft's authoring lane. A separate variant rather than a reused
    /// `Elaboration` row, because an elaboration session carries a `PatchAnswer`
    /// and a seed answers nothing: sharing the row would mean either widening a
    /// kernel enum read by `require_answer` and `ground_covers`, or a session
    /// whose answer field is never submitted.
    Seed(SeedCheckpoint),
}

/// Which lane a stored checkpoint belongs to. Distinct from `ControllerMode`,
/// which says how a *subject* is controlled: the elaborator is not a subject
/// and has no mode.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum WorkLane {
    Narrative,
    Operational,
    Grouped,
    Elaboration,
    Seed,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "stage", rename_all = "snake_case")]
enum NarrativeCheckpoint {
    Projector {
        command_id: CommandId,
        identity: String,
        typed_view: String,
        /// The acting subject's own components at turn time, frozen for the
        /// life of the row exactly as `opportunity` is. Not a cache of the
        /// digest: it is the only copy of a value the digest hashes and
        /// discards, and it is the "before" an interruption is measured
        /// against. Pre-turn variants carry it because progression demands
        /// equality and the value must reach `ReadyToSubmit`; nothing reads it
        /// before submit.
        components: ScopeComponents,
        persona_model: String,
        interpreter_model: String,
        opportunity: DecisionOpportunity,
        granted: Vec<AffordanceSnapshot>,
        invocation: PreparedInference,
    },
    Persona {
        command_id: CommandId,
        identity: String,
        typed_view: String,
        components: ScopeComponents,
        interpreter_model: String,
        opportunity: DecisionOpportunity,
        granted: Vec<AffordanceSnapshot>,
        projector_output: InferenceOutput,
        invocation: PreparedInference,
    },
    InterpreterInFlight {
        command_id: CommandId,
        turn: PersonaTurn,
        interpreter_prompt: String,
        components: ScopeComponents,
        /// `Some` exactly when this row's turn was re-lowered, and then equal to
        /// the turn's `interrupted_from`: the two records of the same fact may
        /// not disagree.
        interruption: Option<Interruption>,
        opportunity: DecisionOpportunity,
        granted: Vec<AffordanceSnapshot>,
        completed: Vec<InferenceOutput>,
        invocation: PreparedInference,
    },
    ReadyToSubmit {
        command_id: CommandId,
        turn: PersonaTurn,
        interpreter_prompt: String,
        components: ScopeComponents,
        interruption: Option<Interruption>,
        opportunity: DecisionOpportunity,
        granted: Vec<AffordanceSnapshot>,
        completed: Vec<InferenceOutput>,
    },
    NoProposal {
        command_id: CommandId,
        turn: PersonaTurn,
        interpreter_prompt: String,
        components: ScopeComponents,
        interruption: Option<Interruption>,
        opportunity: DecisionOpportunity,
        completed: Vec<InferenceOutput>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "stage", rename_all = "snake_case")]
enum OperationalCheckpoint {
    AgentInFlight {
        command_id: CommandId,
        agent_prompt: String,
        opportunity: DecisionOpportunity,
        granted: Vec<AffordanceSnapshot>,
        completed: Vec<InferenceOutput>,
        invocation: PreparedInference,
    },
    ReadyToSubmit {
        command_id: CommandId,
        agent_prompt: String,
        opportunity: DecisionOpportunity,
        granted: Vec<AffordanceSnapshot>,
        completed: Vec<InferenceOutput>,
    },
    NoProposal {
        command_id: CommandId,
        agent_prompt: String,
        opportunity: DecisionOpportunity,
        completed: Vec<InferenceOutput>,
    },
}

impl ControllerWork {
    pub(super) fn command_id(&self) -> CommandId {
        match self {
            Self::Narrative(checkpoint) => checkpoint.command_id(),
            Self::Operational(checkpoint) => checkpoint.command_id(),
            Self::Grouped(checkpoint) => checkpoint.command_id(),
            Self::Elaboration(checkpoint) => checkpoint.command_id(),
            Self::Seed(checkpoint) => checkpoint.command_id(),
        }
    }

    fn lane(&self) -> WorkLane {
        match self {
            Self::Narrative(_) => WorkLane::Narrative,
            Self::Operational(_) => WorkLane::Operational,
            Self::Grouped(_) => WorkLane::Grouped,
            Self::Elaboration(_) => WorkLane::Elaboration,
            Self::Seed(_) => WorkLane::Seed,
        }
    }

    /// How the subjects in this row were represented. Derived from the row's
    /// own shape rather than stored beside it: a persisted copy would be a
    /// second spelling of `constituents.len()` that could disagree with it.
    #[cfg_attr(not(test), expect(dead_code, reason = "read by the resolution test"))]
    fn resolution(&self) -> Resolution {
        match self {
            Self::Narrative(_) | Self::Operational(_) | Self::Elaboration(_) | Self::Seed(_) => {
                Resolution::Detail
            }
            Self::Grouped(checkpoint) => Resolution::Coarse {
                constituents: checkpoint.constituents().len(),
            },
        }
    }

    fn is_initial(&self) -> bool {
        match self {
            Self::Narrative(NarrativeCheckpoint::Projector { .. }) => true,
            Self::Operational(OperationalCheckpoint::AgentInFlight { completed, .. })
            | Self::Grouped(GroupedCheckpoint::AgentInFlight { completed, .. }) => {
                completed.is_empty()
            }
            Self::Elaboration(ElaborationCheckpoint::ElaboratorInFlight {
                completed,
                last_mismatches,
                ..
            }) => completed.is_empty() && last_mismatches.is_empty(),
            Self::Seed(checkpoint) => checkpoint.is_initial(),
            _ => false,
        }
    }

    fn integrity_is_valid(&self) -> bool {
        match self {
            Self::Narrative(checkpoint) => checkpoint.integrity_is_valid(),
            Self::Operational(checkpoint) => checkpoint.integrity_is_valid(),
            Self::Grouped(checkpoint) => checkpoint.integrity_is_valid(),
            Self::Elaboration(checkpoint) => checkpoint.integrity_is_valid(),
            Self::Seed(checkpoint) => checkpoint.integrity_is_valid(),
        }
    }
}

impl NarrativeCheckpoint {
    fn command_id(&self) -> CommandId {
        match self {
            Self::Projector { command_id, .. }
            | Self::Persona { command_id, .. }
            | Self::InterpreterInFlight { command_id, .. }
            | Self::ReadyToSubmit { command_id, .. }
            | Self::NoProposal { command_id, .. } => *command_id,
        }
    }

    fn opportunity(&self) -> &DecisionOpportunity {
        match self {
            Self::Projector { opportunity, .. }
            | Self::Persona { opportunity, .. }
            | Self::InterpreterInFlight { opportunity, .. }
            | Self::ReadyToSubmit { opportunity, .. }
            | Self::NoProposal { opportunity, .. } => opportunity,
        }
    }

    fn persona_prose(&self) -> Option<&str> {
        match self {
            Self::InterpreterInFlight { turn, .. }
            | Self::ReadyToSubmit { turn, .. }
            | Self::NoProposal { turn, .. } => Some(turn.source_prose()),
            Self::Projector { .. } | Self::Persona { .. } => None,
        }
    }

    fn integrity_is_valid(&self) -> bool {
        match self {
            Self::Projector {
                command_id,
                identity,
                typed_view,
                components: _,
                persona_model,
                interpreter_model,
                opportunity,
                granted,
                invocation,
            } => {
                base_checkpoint_is_valid(
                    identity,
                    typed_view,
                    opportunity,
                    granted,
                    ControllerMode::NarrativePersona,
                ) && canonical_model(persona_model)
                    && canonical_model(interpreter_model)
                    && exact_request_identity(
                        invocation,
                        *command_id,
                        InferencePurpose::Projector,
                        0,
                    )
                    && prose_request_shape_is_valid(invocation)
            }
            Self::Persona {
                command_id,
                identity,
                typed_view,
                components: _,
                interpreter_model,
                opportunity,
                granted,
                projector_output,
                invocation,
            } => {
                let Ok((lived_stream, _)) = projector_output
                    .clone()
                    .prose_only(InferencePurpose::Projector)
                else {
                    return false;
                };
                let prompt = build_persona_prompt(&PersonaPrompt {
                    identity,
                    lived_stream: &lived_stream,
                    domain_guidance: "",
                    word_budget: PERSONA_WORD_BUDGET,
                });
                base_checkpoint_is_valid(
                    identity,
                    typed_view,
                    opportunity,
                    granted,
                    ControllerMode::NarrativePersona,
                ) && canonical_model(interpreter_model)
                    && canonical_model(&invocation.invocation.request.model)
                    && persona_request(*command_id, &invocation.invocation.request.model, prompt)
                        .is_ok_and(|expected| {
                            prepared_matches_request(invocation, &expected, *command_id, 0)
                        })
                    && persona_request_shape_is_valid(invocation)
            }
            Self::InterpreterInFlight {
                command_id,
                turn,
                interpreter_prompt,
                interruption,
                opportunity,
                granted,
                completed,
                invocation,
                ..
            } => {
                let round = interpreter_round(interruption, completed);
                turn_matches_opportunity(turn, opportunity)
                    && interruption_matches_turn(interruption, turn)
                    && opportunity.controller_mode == ControllerMode::NarrativePersona
                    && granted_matches_opportunity(granted, opportunity)
                    && !interpreter_prompt.is_empty()
                    && canonical_model(&invocation.invocation.request.model)
                    && match evaluate_interpreter_loop(turn, interpreter_prompt, completed) {
                        Ok(InterpreterLoopEvaluation::Continue { conversation }) => {
                            interpreter_request(
                                *command_id,
                                round,
                                &invocation.invocation.request.model,
                                conversation,
                            )
                            .is_ok_and(|expected| {
                                prepared_matches_request(invocation, &expected, *command_id, round)
                            })
                        }
                        Ok(InterpreterLoopEvaluation::Complete { .. }) | Err(_) => false,
                    }
            }
            Self::ReadyToSubmit {
                turn,
                interpreter_prompt,
                interruption,
                opportunity,
                granted,
                completed,
                ..
            } => {
                turn_matches_opportunity(turn, opportunity)
                    && interruption_matches_turn(interruption, turn)
                    && opportunity.controller_mode == ControllerMode::NarrativePersona
                    && granted_matches_opportunity(granted, opportunity)
                    && derive_narrative_capture(turn, interpreter_prompt, completed)
                        .is_ok_and(|capture| capture.proposal.is_some())
            }
            Self::NoProposal {
                turn,
                interpreter_prompt,
                interruption,
                opportunity,
                completed,
                ..
            } => {
                turn_matches_opportunity(turn, opportunity)
                    && interruption_matches_turn(interruption, turn)
                    && opportunity.controller_mode == ControllerMode::NarrativePersona
                    && derive_narrative_capture(turn, interpreter_prompt, completed)
                        .is_ok_and(|capture| capture.proposal.is_none())
            }
        }
    }
}

impl OperationalCheckpoint {
    fn command_id(&self) -> CommandId {
        match self {
            Self::AgentInFlight { command_id, .. }
            | Self::ReadyToSubmit { command_id, .. }
            | Self::NoProposal { command_id, .. } => *command_id,
        }
    }

    fn opportunity(&self) -> &DecisionOpportunity {
        match self {
            Self::AgentInFlight { opportunity, .. }
            | Self::ReadyToSubmit { opportunity, .. }
            | Self::NoProposal { opportunity, .. } => opportunity,
        }
    }

    fn integrity_is_valid(&self) -> bool {
        match self {
            Self::AgentInFlight {
                command_id,
                agent_prompt,
                opportunity,
                granted,
                completed,
                invocation,
            } => {
                opportunity.controller_mode == ControllerMode::OperationalAgent
                    && granted_matches_opportunity(granted, opportunity)
                    && !agent_prompt.is_empty()
                    && canonical_model(&invocation.invocation.request.model)
                    && match evaluate_operational_loop(agent_prompt, granted, completed) {
                        Ok(OperationalLoopEvaluation::Continue { conversation }) => {
                            operational_request(
                                *command_id,
                                completed.len(),
                                &invocation.invocation.request.model,
                                granted,
                                conversation,
                            )
                            .is_ok_and(|expected| {
                                prepared_matches_request(
                                    invocation,
                                    &expected,
                                    *command_id,
                                    completed.len(),
                                )
                            })
                        }
                        Ok(OperationalLoopEvaluation::Complete { .. }) | Err(_) => false,
                    }
            }
            Self::ReadyToSubmit {
                agent_prompt,
                opportunity,
                granted,
                completed,
                ..
            } => {
                opportunity.controller_mode == ControllerMode::OperationalAgent
                    && granted_matches_opportunity(granted, opportunity)
                    && derive_operational_capture(agent_prompt, granted, completed)
                        .is_ok_and(|capture| capture.proposal.is_some())
            }
            Self::NoProposal {
                agent_prompt,
                opportunity,
                completed,
                ..
            } => {
                opportunity.controller_mode == ControllerMode::OperationalAgent
                    && derive_operational_capture(agent_prompt, &[], completed)
                        .is_ok_and(|capture| capture.proposal.is_none())
            }
        }
    }
}

impl GroupedCheckpoint {
    fn command_id(&self) -> CommandId {
        match self {
            Self::AgentInFlight { command_id, .. } | Self::Submitting { command_id, .. } => {
                *command_id
            }
        }
    }

    fn cell(&self) -> CellId {
        match self {
            Self::AgentInFlight { cell, .. } | Self::Submitting { cell, .. } => *cell,
        }
    }

    fn constituents(&self) -> &[ConstituentWork] {
        match self {
            Self::AgentInFlight { constituents, .. } | Self::Submitting { constituents, .. } => {
                constituents
            }
        }
    }

    fn integrity_is_valid(&self) -> bool {
        let (agent_prompt, constituents, completed) = match self {
            Self::AgentInFlight {
                agent_prompt,
                constituents,
                completed,
                ..
            }
            | Self::Submitting {
                agent_prompt,
                constituents,
                completed,
                ..
            } => (agent_prompt, constituents, completed),
        };
        if !cell_constituents_are_valid(constituents) || agent_prompt.is_empty() {
            return false;
        }
        match self {
            Self::AgentInFlight {
                command_id,
                invocation,
                ..
            } => {
                canonical_model(&invocation.invocation.request.model)
                    && match evaluate_grouped_loop(agent_prompt, constituents, completed) {
                        Ok(GroupedLoopEvaluation::Continue { conversation }) => grouped_request(
                            *command_id,
                            completed.len(),
                            &invocation.invocation.request.model,
                            constituents,
                            conversation,
                        )
                        .is_ok_and(|expected| {
                            prepared_matches_request(
                                invocation,
                                &expected,
                                *command_id,
                                completed.len(),
                            )
                        }),
                        Ok(GroupedLoopEvaluation::Complete { .. }) | Err(_) => false,
                    }
            }
            Self::Submitting { .. } => {
                derive_grouped_capture(agent_prompt, constituents, completed).is_ok()
            }
        }
    }
}

/// A cell's constituents are ascending, distinct, non-empty, each holding
/// exactly what its own opportunity grants, and each an inference rather than a
/// human turn. A duplicate subject would make one handle able to submit twice.
fn cell_constituents_are_valid(constituents: &[ConstituentWork]) -> bool {
    !constituents.is_empty()
        && constituents
            .windows(2)
            .all(|pair| pair[0].subject < pair[1].subject)
        && constituents.iter().all(|entry| {
            entry.subject == entry.opportunity.scope.subject_id
                && entry.opportunity.controller_mode != ControllerMode::Human
                && granted_matches_opportunity(&entry.granted, &entry.opportunity)
        })
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct NarrativeCapture {
    pub(crate) proposal: Option<SourceRange>,
    pub(crate) gaps: Vec<TranslationGapSummary>,
    pub(crate) finalization: InterpretationFinalization,
    pub(crate) inference_receipts: Vec<String>,
}

/// One thing said to this subject after its turn was formed. Speech is the only
/// interruption a subject may perceive as having an author, and only through a
/// `Told { by }` row for a fact `fan_out` actually gave it. The label is
/// resolved here because the snapshot that resolves it is gone by the time the
/// row is re-read, and it is resolved by `speaker_label` and nothing else.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Overheard {
    /// `None` for a fact this subject holds without having been told it. It
    /// knows the thing, not the telling.
    pub(crate) speaker: Option<String>,
    pub(crate) statement: Statement,
    pub(crate) confidence: Confidence,
}

/// What the runner showed the Interpreter when it re-lowered, and the
/// conversation it replaced. Persisted so a resumed row rebuilds the same
/// request byte for byte without a snapshot.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Interruption {
    /// The subject's own components at re-lowering. The "after"; the
    /// checkpoint's `components` is the "before".
    pub(crate) components: ScopeComponents,
    /// Rows whose `spoken_at` is later than the turn's bound revision, in the
    /// snapshot's order.
    pub(crate) overheard: Vec<Overheard>,
    /// The first lowering's evidence. Nothing is discarded merely because the
    /// world moved elsewhere; it also fixes the round index so the second
    /// conversation cannot collide with the first on `conversation_id`.
    pub(crate) discarded: Vec<InferenceOutput>,
}

/// The one owner of the interpreter round index. A re-lowering continues the
/// row's numbering rather than restarting it, so two conversations under one
/// command id can never share a `conversation_id`.
fn interpreter_round(interruption: &Option<Interruption>, completed: &[InferenceOutput]) -> usize {
    interruption
        .as_ref()
        .map_or(0, |interruption| interruption.discarded.len())
        + completed.len()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct SourceRange {
    pub(crate) start_byte: usize,
    pub(crate) end_byte: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct TranslationGapSummary {
    pub(crate) kind: TranslationGapKind,
    pub(crate) source: SourceRange,
    pub(crate) detail: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct OperationalCapture {
    pub(crate) proposal: Option<DecisionInvocation>,
    pub(crate) needs: Vec<ControllerNeed>,
    pub(crate) inference_receipts: Vec<String>,
}

/// One opportunity's selection against one snapshot.
///
/// The expected controller mode is read from the opportunity rather than
/// supplied by the caller. Representation and controller mode used to be the
/// same value here, which made a coarsely represented `NarrativePersona`
/// unrepresentable; the lane gate that still matters — whether this turn may
/// enter the Persona membrane — lives at the lane's own entry, where entering
/// the membrane is decided.
fn select_one(
    snapshot: &WorldSnapshot,
    exact_opportunity: &DecisionOpportunity,
) -> Result<SelectedDecision, ControllerError> {
    let selected = select_scope(snapshot, exact_opportunity, true)?;
    Ok(SelectedDecision {
        // The run's own bound value, not the fresh snapshot's: one run binds
        // one opportunity, persists it, and submits it unchanged.
        opportunity: exact_opportunity.clone(),
        ..selected
    })
}

/// What replaced the scope this run bound. Same subject, same controller, same
/// single-match requirement, same grant intersection — only the digest clause
/// differs, so an interruption can never be selected under looser rules than a
/// turn. The returned opportunity and granted set are the *fresh* ones.
fn select_fresh(
    snapshot: &WorldSnapshot,
    exact_opportunity: &DecisionOpportunity,
) -> Result<SelectedDecision, ControllerError> {
    select_scope(snapshot, exact_opportunity, false)
}

/// The shared matcher. `require_digest` is the only difference between binding a
/// run and finding what replaced it.
fn select_scope(
    snapshot: &WorldSnapshot,
    exact_opportunity: &DecisionOpportunity,
    require_digest: bool,
) -> Result<SelectedDecision, ControllerError> {
    let expected = exact_opportunity.controller_mode;
    let subject_id = exact_opportunity.scope.subject_id;
    let Some(subject) = snapshot
        .subjects
        .iter()
        .find(|subject| subject.id == subject_id)
        .cloned()
    else {
        return Err(ControllerError::NoOpportunity { expected });
    };
    // Fail closed on a subject with no controller: a mirror has neither a mode
    // nor a controller id, so neither comparison can accidentally hold.
    if subject.controller_mode != Some(expected) {
        return Err(ControllerError::NoOpportunity { expected });
    }
    if Some(exact_opportunity.controller_id) != subject.controller_id {
        return Err(ControllerError::OpportunityMismatch);
    }
    let matches = snapshot
        .opportunities
        .iter()
        .filter(|opportunity| {
            opportunity.scope == exact_opportunity.scope
                && (!require_digest || opportunity.scope_digest == exact_opportunity.scope_digest)
        })
        .cloned()
        .collect::<Vec<_>>();
    let [opportunity] = matches.as_slice() else {
        return if matches.is_empty() {
            Err(ControllerError::NoOpportunity { expected })
        } else {
            Err(ControllerError::AmbiguousOpportunity)
        };
    };
    let granted: Vec<AffordanceSnapshot> = snapshot
        .affordances
        .iter()
        .filter(|entry| {
            subject.affordances.contains(&entry.id)
                && opportunity.affordance_ids.contains(&entry.id)
        })
        .cloned()
        .collect();
    if granted.is_empty() {
        return Err(ControllerError::NoGrantedAffordance);
    }
    Ok(SelectedDecision {
        snapshot: snapshot.clone(),
        subject,
        opportunity: opportunity.clone(),
        granted,
    })
}

fn base_checkpoint_is_valid(
    identity: &str,
    typed_view: &str,
    opportunity: &DecisionOpportunity,
    granted: &[AffordanceSnapshot],
    mode: ControllerMode,
) -> bool {
    !identity.is_empty()
        && identity.trim() == identity
        && !typed_view.is_empty()
        && opportunity.controller_mode == mode
        && granted_matches_opportunity(granted, opportunity)
}

/// A resumed run rebuilds byte-identical tool schemas only if it still holds the
/// same entries the opportunity grants.
fn granted_matches_opportunity(
    granted: &[AffordanceSnapshot],
    opportunity: &DecisionOpportunity,
) -> bool {
    !granted.is_empty()
        && granted
            .iter()
            .all(|entry| opportunity.affordance_ids.contains(&entry.id))
}

/// The row and the turn are two records of one fact — that this prose was
/// lowered a second time — and they may not disagree.
fn interruption_matches_turn(interruption: &Option<Interruption>, turn: &PersonaTurn) -> bool {
    interruption.is_some() == turn.binding().interrupted_from.is_some()
}

fn turn_matches_opportunity(turn: &PersonaTurn, opportunity: &DecisionOpportunity) -> bool {
    turn.receipt_is_valid()
        && encoded_id(&opportunity.world_id)
            .is_ok_and(|world_id| world_id == turn.binding().world_id)
        && encoded_id(&opportunity.controller_id)
            .is_ok_and(|controller_id| controller_id == turn.binding().controller_id)
        && opportunity
            .digest()
            .is_ok_and(|digest| digest == turn.binding().opportunity_digest)
        && opportunity.revision == turn.binding().world_revision
        && opportunity.scope_digest.as_str() == turn.binding().scope_digest
}

/// Two opportunity values that bind the same proposal. The revision is a receipt
/// and an ordering value; the scope digest is the binding.
fn binds_same_scope(left: &DecisionOpportunity, right: &DecisionOpportunity) -> bool {
    left.world_id == right.world_id
        && left.scope == right.scope
        && left.scope_digest == right.scope_digest
}

fn derive_narrative_capture(
    turn: &PersonaTurn,
    interpreter_prompt: &str,
    completed: &[InferenceOutput],
) -> Result<NarrativeCapture, ControllerError> {
    match evaluate_interpreter_loop(turn, interpreter_prompt, completed)? {
        InterpreterLoopEvaluation::Complete { capture } => Ok(capture),
        InterpreterLoopEvaluation::Continue { .. } => Err(ControllerError::Serialization(
            "terminal Interpreter checkpoint has unfinished evidence".into(),
        )),
    }
}

fn derive_operational_capture(
    agent_prompt: &str,
    granted: &[AffordanceSnapshot],
    completed: &[InferenceOutput],
) -> Result<OperationalCapture, ControllerError> {
    match evaluate_operational_loop(agent_prompt, granted, completed)? {
        OperationalLoopEvaluation::Complete { capture } => Ok(capture),
        OperationalLoopEvaluation::Continue { .. } => Err(ControllerError::Serialization(
            "terminal operational checkpoint has unfinished evidence".into(),
        )),
    }
}

pub(super) fn canonical_model(model: &str) -> bool {
    !model.is_empty() && model.chars().all(|character| !character.is_whitespace())
}

fn exact_request_identity(
    request: &PreparedInference,
    command_id: CommandId,
    purpose: InferencePurpose,
    round: usize,
) -> bool {
    let provider = &request.invocation.request;
    let native = InferenceRequest {
        purpose: request.purpose,
        provider: provider.clone(),
    };
    request.purpose == purpose
        && provider.validate().is_ok()
        && !request.invocation.caller_runtime_id.is_empty()
        && request.invocation.caller_runtime_id.trim() == request.invocation.caller_runtime_id
        && request.invocation.expires_at_unix_ms > 0
        && serde_json::to_vec(&native)
            .ok()
            .map(|bytes| <[u8; 32]>::from(Sha256::digest(bytes)))
            .is_some_and(|digest| digest == request.invocation.native_request_sha256)
        && provider_request_sha256(provider)
            .is_ok_and(|digest| digest == request.invocation.provider_request_sha256)
        && provider_request_id(
            command_id,
            purpose,
            round,
            &provider.instructions,
            &provider.input,
        )
        .is_ok_and(|expected| provider.request_id == expected)
        && conversation_id(command_id, purpose, round)
            .is_ok_and(|expected| provider.conversation_id == expected)
}

pub(super) fn prepared_matches_request(
    prepared: &PreparedInference,
    expected: &InferenceRequest,
    command_id: CommandId,
    round: usize,
) -> bool {
    prepared.invocation.request == expected.provider
        && exact_request_identity(prepared, command_id, expected.purpose, round)
}

fn prose_request_shape_is_valid(request: &PreparedInference) -> bool {
    let provider = &request.invocation.request;
    provider.input.len() == 1
        && matches!(
            provider.input.first(),
            Some(CodexInputItem::UserText { .. })
        )
        && provider.tools.is_empty()
        && provider.tool_choice == CodexToolChoice::Auto
        && !provider.parallel_tool_calls
        && provider.output_format_name.is_none()
        && provider.output_schema_json.is_none()
        && provider.previous_response_id.is_none()
}

fn persona_request_shape_is_valid(request: &PreparedInference) -> bool {
    prose_request_shape_is_valid(request)
        && request.invocation.request.instructions == PERSONA_PROVIDER_INSTRUCTIONS
}

#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub(crate) enum ControllerWorkStoreError {
    #[error("Eve command ID already belongs to the other controller mode")]
    CommandModeConflict,
    #[error("{detail}")]
    Fault { detail: String },
}

impl ControllerWorkStoreError {
    fn new(detail: impl Into<String>) -> Self {
        Self::Fault {
            detail: detail.into(),
        }
    }
}

#[async_trait]
pub(crate) trait ControllerWorkStore: Send + Sync {
    async fn lookup(
        &self,
        command_id: CommandId,
    ) -> Result<ControllerWorkLookup, ControllerWorkStoreError>;

    /// Owns the only legal checkpoint transition for any lane.
    async fn persist(
        &self,
        work: &ControllerWork,
    ) -> Result<ControllerWorkWrite, ControllerWorkStoreError>;

    /// Proves custody of the backing path without interpreting ordinary model
    /// or world pending states as store failure.
    async fn custody_probe(&self) -> Result<ControllerWorkCustody, ControllerWorkStoreError>;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ControllerWorkLookup {
    Missing,
    Confirmed(ControllerWork),
    CustodyUncertain(ControllerWork),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ControllerWorkWrite {
    Applied,
    AlreadyPresent,
    CustodyUncertain,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ControllerWorkCustody {
    Owned {
        narrative_commands: usize,
        operational_commands: usize,
        elaboration_commands: usize,
        seed_commands: usize,
        // `Grouped` still has no count here. That asymmetry is the grouped
        // lane's debt, named rather than fixed: widening it in this pass would
        // be work with no consumer.
    },
    /// A compare-and-swap or post-write ownership check did not confirm. The
    /// process must reopen the journal before doing any other controller work.
    Uncertain {
        command_id: CommandId,
        lane: WorkLane,
    },
}

struct ControllerWorkJournal {
    store: OwnedRedbMessagePackBackingStore,
    rows: Vec<CultCacheEnvelope>,
    work: BTreeMap<CommandId, ControllerWork>,
    uncertain: Option<ControllerWork>,
}

/// The one controller-work journal. It owns durable cognition evidence and
/// exact handoff progress for both modes, but never owns a world decision.
struct CultCacheControllerWorkStore {
    journal: Mutex<ControllerWorkJournal>,
}

/// The production controller-work owner, opened for the caller that names its
/// path. The store type stays private; only the trait object leaves here.
pub(crate) fn open_controller_work(
    path: impl AsRef<Path>,
) -> Result<Arc<dyn ControllerWorkStore>, ControllerOpenError> {
    Ok(Arc::new(CultCacheControllerWorkStore::open(path)?))
}

impl CultCacheControllerWorkStore {
    fn open(path: impl AsRef<Path>) -> Result<Self, ControllerWorkStoreError> {
        let store = OwnedRedbMessagePackBackingStore::new(path.as_ref())
            .map_err(|error| ControllerWorkStoreError::new(error.to_string()))?;
        store
            .validate_path_identity()
            .map_err(|error| ControllerWorkStoreError::new(error.to_string()))?;
        let rows = store
            .pull_all()
            .map_err(|error| ControllerWorkStoreError::new(error.to_string()))?;
        let mut work_by_command = BTreeMap::new();
        for row in &rows {
            if row.r#type != CONTROLLER_WORK_ROW
                || row.schema_id.as_deref() != Some(CONTROLLER_WORK_SCHEMA)
            {
                return Err(ControllerWorkStoreError::new(format!(
                    "unexpected row {}/{} in controller work store",
                    row.r#type, row.key
                )));
            }
            let work = decode_controller_work(row)?;
            let command_id = work.command_id();
            let key = store_key(command_id)?;
            if !work.integrity_is_valid() || row.key != key {
                return Err(ControllerWorkStoreError::new(format!(
                    "controller work row {} failed its checkpoint binding",
                    row.key
                )));
            }
            if work_by_command.insert(command_id, work).is_some() {
                return Err(ControllerWorkStoreError::new(format!(
                    "duplicate controller command row {}",
                    row.key
                )));
            }
        }
        Ok(Self {
            journal: Mutex::new(ControllerWorkJournal {
                store,
                rows,
                work: work_by_command,
                uncertain: None,
            }),
        })
    }
}

fn decode_controller_work(
    row: &CultCacheEnvelope,
) -> Result<ControllerWork, ControllerWorkStoreError> {
    let value: ControllerWork = rmp_serde::from_slice(&row.payload)
        .map_err(|error| ControllerWorkStoreError::new(error.to_string()))?;
    let canonical = rmp_serde::to_vec_named(&value)
        .map_err(|error| ControllerWorkStoreError::new(error.to_string()))?;
    if canonical != row.payload {
        return Err(ControllerWorkStoreError::new(format!(
            "controller work row {} is not canonical",
            row.key
        )));
    }
    Ok(value)
}

#[async_trait]
impl ControllerWorkStore for CultCacheControllerWorkStore {
    async fn lookup(
        &self,
        command_id: CommandId,
    ) -> Result<ControllerWorkLookup, ControllerWorkStoreError> {
        let journal = self.journal.lock().map_err(|_| {
            ControllerWorkStoreError::new("controller work store lock was poisoned")
        })?;
        if let Some(uncertain) = &journal.uncertain {
            return if uncertain.command_id() == command_id {
                Ok(ControllerWorkLookup::CustodyUncertain(uncertain.clone()))
            } else {
                Err(ControllerWorkStoreError::new(
                    "controller work store ownership is uncertain; reopen before another command",
                ))
            };
        }
        verify_journal_custody(&journal)?;
        Ok(journal
            .work
            .get(&command_id)
            .cloned()
            .map(ControllerWorkLookup::Confirmed)
            .unwrap_or(ControllerWorkLookup::Missing))
    }

    async fn persist(
        &self,
        work: &ControllerWork,
    ) -> Result<ControllerWorkWrite, ControllerWorkStoreError> {
        if !work.integrity_is_valid() {
            return Err(ControllerWorkStoreError::new(
                "refused controller work with an invalid checkpoint",
            ));
        }
        let command_id = work.command_id();
        let key = store_key(command_id)?;
        let mut journal = self.journal.lock().map_err(|_| {
            ControllerWorkStoreError::new("controller work store lock was poisoned")
        })?;
        if let Some(uncertain) = &journal.uncertain {
            return if uncertain == work {
                Ok(ControllerWorkWrite::CustodyUncertain)
            } else {
                Err(ControllerWorkStoreError::new(
                    "controller work store ownership is uncertain; reopen before mutation",
                ))
            };
        }
        verify_journal_custody(&journal)?;
        if let Some(existing) = journal.work.get(&command_id) {
            if existing == work {
                return Ok(ControllerWorkWrite::AlreadyPresent);
            }
            if existing.lane() != work.lane() {
                return Err(ControllerWorkStoreError::CommandModeConflict);
            }
            if !valid_controller_work_progression(existing, work) {
                return Err(ControllerWorkStoreError::new(format!(
                    "controller command {key} attempted an illegal checkpoint transition"
                )));
            }
        } else if !work.is_initial() {
            return Err(ControllerWorkStoreError::new(format!(
                "controller command {key} did not begin before its provider boundary"
            )));
        }
        let row = CultCacheEnvelope {
            key: key.clone(),
            r#type: CONTROLLER_WORK_ROW.into(),
            payload: rmp_serde::to_vec_named(work)
                .map_err(|error| ControllerWorkStoreError::new(error.to_string()))?,
            stored_at: Utc::now().to_rfc3339(),
            schema_id: Some(CONTROLLER_WORK_SCHEMA.into()),
        };
        let mut next_rows = journal
            .rows
            .iter()
            .filter(|existing| !(existing.r#type == CONTROLLER_WORK_ROW && existing.key == key))
            .cloned()
            .collect::<Vec<_>>();
        next_rows.push(row);
        next_rows
            .sort_by(|left, right| (&left.r#type, &left.key).cmp(&(&right.r#type, &right.key)));
        if journal.store.validate_path_identity().is_err() {
            journal.uncertain = Some(work.clone());
            return Ok(ControllerWorkWrite::CustodyUncertain);
        }
        let written = journal
            .store
            .replace_and_append_if_snapshot_unchanged(&journal.rows, next_rows.clone());
        if !matches!(written, Ok(true)) || journal.store.validate_path_identity().is_err() {
            journal.uncertain = Some(work.clone());
            return Ok(ControllerWorkWrite::CustodyUncertain);
        }
        journal.rows = next_rows;
        journal.work.insert(command_id, work.clone());
        Ok(ControllerWorkWrite::Applied)
    }

    async fn custody_probe(&self) -> Result<ControllerWorkCustody, ControllerWorkStoreError> {
        let journal = self.journal.lock().map_err(|_| {
            ControllerWorkStoreError::new("controller work store lock was poisoned")
        })?;
        if let Some(uncertain) = &journal.uncertain {
            return Ok(ControllerWorkCustody::Uncertain {
                command_id: uncertain.command_id(),
                lane: uncertain.lane(),
            });
        }
        verify_journal_custody(&journal)?;
        let count = |lane: WorkLane| {
            journal
                .work
                .values()
                .filter(|work| work.lane() == lane)
                .count()
        };
        Ok(ControllerWorkCustody::Owned {
            narrative_commands: count(WorkLane::Narrative),
            operational_commands: count(WorkLane::Operational),
            elaboration_commands: count(WorkLane::Elaboration),
            seed_commands: count(WorkLane::Seed),
        })
    }
}

fn verify_journal_custody(journal: &ControllerWorkJournal) -> Result<(), ControllerWorkStoreError> {
    journal
        .store
        .validate_path_identity()
        .map_err(|error| ControllerWorkStoreError::new(error.to_string()))?;
    let current_rows = journal
        .store
        .pull_all()
        .map_err(|error| ControllerWorkStoreError::new(error.to_string()))?;
    if current_rows != journal.rows {
        return Err(ControllerWorkStoreError::new(
            "controller work snapshot changed outside its owning store",
        ));
    }
    Ok(())
}

fn valid_controller_work_progression(existing: &ControllerWork, next: &ControllerWork) -> bool {
    if !existing.integrity_is_valid() || !next.integrity_is_valid() {
        return false;
    }
    match (existing, next) {
        (ControllerWork::Narrative(existing), ControllerWork::Narrative(next)) => {
            valid_narrative_progression(existing, next)
        }
        (ControllerWork::Operational(existing), ControllerWork::Operational(next)) => {
            valid_operational_progression(existing, next)
        }
        (ControllerWork::Grouped(existing), ControllerWork::Grouped(next)) => {
            valid_grouped_progression(existing, next)
        }
        (ControllerWork::Elaboration(existing), ControllerWork::Elaboration(next)) => {
            valid_elaboration_progression(existing, next)
        }
        (ControllerWork::Seed(existing), ControllerWork::Seed(next)) => {
            valid_seed_progression(existing, next)
        }
        _ => false,
    }
}

/// The identity of a grouped row is frozen and its evidence only appends. The
/// constituent vector is part of that identity: a resumed cell may not gain,
/// lose, reorder, or re-permission a handle, because a handle is a position in
/// a prompt that has already been sent.
fn valid_grouped_progression(existing: &GroupedCheckpoint, next: &GroupedCheckpoint) -> bool {
    let frozen = |left: &GroupedCheckpoint, right: &GroupedCheckpoint| {
        let identity = |work: &GroupedCheckpoint| match work {
            GroupedCheckpoint::AgentInFlight {
                command_id,
                cell,
                tick,
                agent_prompt,
                constituents,
                ..
            }
            | GroupedCheckpoint::Submitting {
                command_id,
                cell,
                tick,
                agent_prompt,
                constituents,
                ..
            } => (
                *command_id,
                *cell,
                *tick,
                agent_prompt.clone(),
                constituents.clone(),
            ),
        };
        identity(left) == identity(right)
    };
    fn completed(work: &GroupedCheckpoint) -> &[InferenceOutput] {
        match work {
            GroupedCheckpoint::AgentInFlight { completed, .. }
            | GroupedCheckpoint::Submitting { completed, .. } => completed.as_slice(),
        }
    }
    if !frozen(existing, next) {
        return false;
    }
    match (existing, next) {
        (
            GroupedCheckpoint::AgentInFlight { .. },
            GroupedCheckpoint::AgentInFlight { .. } | GroupedCheckpoint::Submitting { .. },
        ) => completed_advances(completed(existing), completed(next)),
        // Submitting is terminal: the submission loop replays against the
        // kernel's ledger rather than advancing a persisted outcome, so the row
        // never changes again and never regresses to another inference.
        (GroupedCheckpoint::Submitting { .. }, GroupedCheckpoint::Submitting { .. }) => {
            completed(existing) == completed(next)
        }
        (GroupedCheckpoint::Submitting { .. }, GroupedCheckpoint::AgentInFlight { .. }) => false,
    }
}

fn valid_narrative_progression(existing: &NarrativeCheckpoint, next: &NarrativeCheckpoint) -> bool {
    match (existing, next) {
        (
            NarrativeCheckpoint::Projector {
                command_id,
                identity,
                typed_view,
                components,
                persona_model,
                interpreter_model,
                opportunity,
                granted,
                invocation: existing_invocation,
            },
            NarrativeCheckpoint::Persona {
                command_id: next_command_id,
                identity: next_identity,
                typed_view: next_typed_view,
                components: next_components,
                interpreter_model: next_interpreter_model,
                opportunity: next_opportunity,
                granted: next_granted,
                invocation,
                ..
            },
        ) => {
            command_id == next_command_id
                && identity == next_identity
                && typed_view == next_typed_view
                && components == next_components
                && interpreter_model == next_interpreter_model
                && opportunity == next_opportunity
                && granted == next_granted
                && &invocation.invocation.request.model == persona_model
                && invocation.invocation.caller_runtime_id
                    == existing_invocation.invocation.caller_runtime_id
        }
        (
            NarrativeCheckpoint::Persona {
                command_id,
                identity,
                typed_view,
                components,
                interpreter_model,
                opportunity,
                granted,
                projector_output,
                invocation: existing_invocation,
            },
            NarrativeCheckpoint::InterpreterInFlight {
                command_id: next_command_id,
                turn,
                interpreter_prompt,
                components: next_components,
                interruption: next_interruption,
                opportunity: next_opportunity,
                granted: next_granted,
                completed,
                invocation,
            },
        ) => {
            let Ok((lived_stream, projector_receipt)) = projector_output
                .clone()
                .prose_only(InferencePurpose::Projector)
            else {
                return false;
            };
            let expected_prompt = build_interpreter_prompt(&InterpreterPrompt {
                identity,
                typed_context: typed_view,
                lived_stream: &lived_stream,
                persona_output: turn.source_prose(),
                output_schema: None,
                domain_guidance: "",
            });
            command_id == next_command_id
                && components == next_components
                && opportunity == next_opportunity
                && granted == next_granted
                && completed.is_empty()
                // A first lowering can never be born interrupted.
                && next_interruption.is_none()
                && turn.binding().interrupted_from.is_none()
                && interpreter_prompt == &expected_prompt
                && turn.binding().projector_receipt_digest == projector_receipt
                && &invocation.invocation.request.model == interpreter_model
                && invocation.invocation.caller_runtime_id
                    == existing_invocation.invocation.caller_runtime_id
        }
        (
            NarrativeCheckpoint::InterpreterInFlight {
                command_id,
                turn,
                interpreter_prompt,
                components,
                interruption,
                opportunity,
                granted,
                completed,
                invocation: existing_invocation,
            },
            NarrativeCheckpoint::InterpreterInFlight {
                command_id: next_command_id,
                turn: next_turn,
                interpreter_prompt: next_interpreter_prompt,
                components: next_components,
                interruption: next_interruption,
                opportunity: next_opportunity,
                granted: next_granted,
                completed: next_completed,
                invocation: next_invocation,
            },
        ) => {
            command_id == next_command_id
                && turn == next_turn
                && interpreter_prompt == next_interpreter_prompt
                && components == next_components
                && interruption == next_interruption
                && opportunity == next_opportunity
                && granted == next_granted
                && completed_advances(completed, next_completed)
                && existing_invocation.invocation.request.model
                    == next_invocation.invocation.request.model
                && existing_invocation.invocation.caller_runtime_id
                    == next_invocation.invocation.caller_runtime_id
        }
        (
            NarrativeCheckpoint::InterpreterInFlight {
                command_id,
                turn,
                interpreter_prompt,
                components,
                interruption,
                opportunity,
                granted,
                completed,
                ..
            },
            NarrativeCheckpoint::ReadyToSubmit {
                command_id: next_command_id,
                turn: next_turn,
                interpreter_prompt: next_interpreter_prompt,
                components: next_components,
                interruption: next_interruption,
                opportunity: next_opportunity,
                granted: next_granted,
                completed: next_completed,
            },
        ) => {
            command_id == next_command_id
                && turn == next_turn
                && interpreter_prompt == next_interpreter_prompt
                && components == next_components
                && interruption == next_interruption
                && opportunity == next_opportunity
                && granted == next_granted
                && completed_advances(completed, next_completed)
        }
        (
            NarrativeCheckpoint::InterpreterInFlight {
                command_id,
                turn,
                interpreter_prompt,
                components,
                interruption,
                opportunity,
                completed,
                ..
            },
            NarrativeCheckpoint::NoProposal {
                command_id: next_command_id,
                turn: next_turn,
                interpreter_prompt: next_interpreter_prompt,
                components: next_components,
                interruption: next_interruption,
                opportunity: next_opportunity,
                completed: next_completed,
            },
        ) => {
            command_id == next_command_id
                && turn == next_turn
                && interpreter_prompt == next_interpreter_prompt
                && components == next_components
                && interruption == next_interruption
                && opportunity == next_opportunity
                && completed_advances(completed, next_completed)
        }
        // The one re-lowering. This arm is the entire bound: a checkpoint whose
        // turn already carries `interrupted_from` fails the clause below and has
        // no other successor, so a second re-lowering cannot be persisted even
        // if the runner were changed to attempt one. There is no counter and no
        // tick. It is also what refuses a forged ancestry — `is_initial` admits
        // only `Projector` for this lane, so an `InterpreterInFlight` row can
        // only arrive through a progression, and this arm requires the claimed
        // prior binding to equal the existing row's own turn binding.
        (
            NarrativeCheckpoint::ReadyToSubmit {
                command_id,
                turn,
                interpreter_prompt,
                components,
                opportunity,
                completed,
                ..
            }
            | NarrativeCheckpoint::NoProposal {
                command_id,
                turn,
                interpreter_prompt,
                components,
                opportunity,
                completed,
                ..
            },
            NarrativeCheckpoint::InterpreterInFlight {
                command_id: next_command_id,
                turn: next_turn,
                interpreter_prompt: next_interpreter_prompt,
                components: next_components,
                interruption: next_interruption,
                opportunity: next_opportunity,
                granted: next_granted,
                completed: next_completed,
                invocation: next_invocation,
            },
        ) => {
            command_id == next_command_id
                // The before must survive the rebinding unchanged.
                && components == next_components
                && turn.binding().interrupted_from.is_none()
                && next_turn.binding().interrupted_from.as_deref() == Some(turn.binding())
                && next_turn.source_prose() == turn.source_prose()
                && next_turn.binding().projector_receipt_digest
                    == turn.binding().projector_receipt_digest
                && next_turn.binding().persona_inference_receipt_digest
                    == turn.binding().persona_inference_receipt_digest
                && opportunity.world_id == next_opportunity.world_id
                && opportunity.scope == next_opportunity.scope
                && opportunity.scope_digest != next_opportunity.scope_digest
                && granted_matches_opportunity(next_granted, next_opportunity)
                && next_completed.is_empty()
                && next_interruption.as_ref().is_some_and(|interruption| {
                    interruption.discarded == *completed
                        && next_interpreter_prompt
                            == &format!(
                                "{interpreter_prompt}{}",
                                interruption_section(
                                    components,
                                    opportunity,
                                    interruption,
                                    next_opportunity,
                                )
                            )
                })
                && canonical_model(&next_invocation.invocation.request.model)
        }
        _ => false,
    }
}

fn valid_operational_progression(
    existing: &OperationalCheckpoint,
    next: &OperationalCheckpoint,
) -> bool {
    match (existing, next) {
        (
            OperationalCheckpoint::AgentInFlight {
                command_id,
                agent_prompt,
                opportunity,
                granted,
                completed,
                invocation: existing_invocation,
            },
            OperationalCheckpoint::AgentInFlight {
                command_id: next_command_id,
                agent_prompt: next_agent_prompt,
                opportunity: next_opportunity,
                granted: next_granted,
                completed: next_completed,
                invocation: next_invocation,
            },
        ) => {
            command_id == next_command_id
                && agent_prompt == next_agent_prompt
                && opportunity == next_opportunity
                && granted == next_granted
                && completed_advances(completed, next_completed)
                && existing_invocation.invocation.request.model
                    == next_invocation.invocation.request.model
                && existing_invocation.invocation.caller_runtime_id
                    == next_invocation.invocation.caller_runtime_id
        }
        (
            OperationalCheckpoint::AgentInFlight {
                command_id,
                agent_prompt,
                opportunity,
                granted,
                completed,
                ..
            },
            OperationalCheckpoint::ReadyToSubmit {
                command_id: next_command_id,
                agent_prompt: next_agent_prompt,
                opportunity: next_opportunity,
                granted: next_granted,
                completed: next_completed,
            },
        ) => {
            command_id == next_command_id
                && agent_prompt == next_agent_prompt
                && opportunity == next_opportunity
                && granted == next_granted
                && completed_advances(completed, next_completed)
        }
        (
            OperationalCheckpoint::AgentInFlight {
                command_id,
                agent_prompt,
                opportunity,
                completed,
                ..
            },
            OperationalCheckpoint::NoProposal {
                command_id: next_command_id,
                agent_prompt: next_agent_prompt,
                opportunity: next_opportunity,
                completed: next_completed,
            },
        ) => {
            command_id == next_command_id
                && agent_prompt == next_agent_prompt
                && opportunity == next_opportunity
                && completed_advances(completed, next_completed)
        }
        _ => false,
    }
}

fn completed_advances(existing: &[InferenceOutput], next: &[InferenceOutput]) -> bool {
    next.len() == existing.len() + 1 && next.starts_with(existing)
}

fn store_key(command_id: CommandId) -> Result<String, ControllerWorkStoreError> {
    serde_json::to_value(command_id)
        .map_err(|error| ControllerWorkStoreError::new(error.to_string()))?
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| ControllerWorkStoreError::new("command ID did not encode as a string"))
}

#[derive(Clone, Debug)]
pub(crate) struct ControllerModels {
    pub(crate) projector: String,
    pub(crate) persona: String,
    pub(crate) interpreter: String,
    pub(crate) operational_agent: String,
    /// The authoring lane's model. The existing config shape already gates
    /// cognition by model name, so the driver starts on this and no mode flag
    /// is added. The model name also selects the transport: a
    /// `GHOSTLIGHT_SDK_MODEL_PREFIX`-prefixed model reaches the SDK sidecar and
    /// anything else reaches the connector, which is why there is still no mode
    /// flag.
    pub(crate) elaborator: String,
}

impl ControllerModels {
    /// Every model a configured backend must claim, in one place, so the
    /// open-time routing check and the canonical-identifier gate read the same
    /// list.
    pub(crate) fn each(&self) -> [&String; 5] {
        [
            &self.projector,
            &self.persona,
            &self.interpreter,
            &self.operational_agent,
            &self.elaborator,
        ]
    }

    fn are_canonical(&self) -> bool {
        self.each().into_iter().all(|model| canonical_model(model))
    }
}

#[derive(Debug, Error)]
pub(crate) enum ControllerOpenError {
    #[error("controller model IDs must be exact nonempty identifiers without whitespace")]
    InvalidModels,
    #[error("CodexConnector controller transport could not open: {0}")]
    Connector(#[from] InferenceFault),
    #[error("controller work journal could not open: {0}")]
    WorkStore(String),
    #[error("no configured inference backend claims the controller model `{model}`")]
    UnroutableModel { model: String },
    #[error("the SDK sidecar entry `{path}` is not a file")]
    SdkSidecarMissing { path: String },
}

/// Everything the CodexConnector transport needs to open, gathered so
/// `ControllerRunner::open` takes a port rather than building one.
pub(crate) struct ConnectorBinding {
    pub(crate) endpoint: SocketAddr,
    pub(crate) key_path: PathBuf,
    pub(crate) caller_runtime_id: String,
}

/// Builds the one port every lane shares. A lane whose model no configured
/// backend claims fails here, at open, rather than at its first tick.
pub(crate) fn open_inference(
    connector: Option<ConnectorBinding>,
    sdk: Option<SdkBinding>,
    models: &ControllerModels,
) -> Result<Arc<dyn InferencePort>, ControllerOpenError> {
    let connector: Option<Arc<dyn InferencePort>> = match connector {
        Some(binding) => Some(Arc::new(CodexConnectorInferencePort::from_secret_file(
            binding.endpoint,
            binding.key_path,
            binding.caller_runtime_id,
        )?)),
        None => None,
    };
    let mut sdk_model_prefix = DEFAULT_SDK_MODEL_PREFIX.to_owned();
    let sdk: Option<Arc<dyn InferencePort>> = match sdk {
        Some(binding) => {
            if !binding.sidecar_entry.is_file() {
                return Err(ControllerOpenError::SdkSidecarMissing {
                    path: binding.sidecar_entry.display().to_string(),
                });
            }
            sdk_model_prefix = binding.model_prefix;
            Some(Arc::new(SdkInferencePort::new(
                Arc::new(ChildProcessLink::new(binding.sidecar_entry)),
                binding.caller_runtime_id,
            )))
        }
        None => None,
    };
    let routed = RoutedInferencePort::new(connector, sdk, sdk_model_prefix);
    for model in models.each() {
        if routed.route(model).is_none() {
            return Err(ControllerOpenError::UnroutableModel {
                model: model.clone(),
            });
        }
    }
    Ok(Arc::new(routed))
}

impl From<ControllerWorkStoreError> for ControllerOpenError {
    fn from(error: ControllerWorkStoreError) -> Self {
        Self::WorkStore(error.to_string())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub(crate) struct SpeakProposal {
    text: String,
}

impl SpeakProposal {
    pub(crate) fn text(&self) -> &str {
        &self.text
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct ControllerNeed {
    pub(crate) detail: String,
}

#[derive(Debug)]
pub(crate) enum SubmissionDisposition {
    NoProposal(SubmitReceipt),
    Completed(SubmitReceipt),
    /// Derived on demand from the WorldMailbox journal. Controller work never
    /// persists a second opinion about the canonical commit.
    PreviouslyConfirmed(CommitReceipt),
}

#[derive(Debug)]
pub(crate) struct NarrativeDecision {
    turn: PersonaTurn,
    capture: NarrativeCapture,
    submission: SubmissionDisposition,
}

impl NarrativeDecision {
    pub(crate) fn persona_turn(&self) -> &PersonaTurn {
        &self.turn
    }

    pub(crate) fn capture(&self) -> &NarrativeCapture {
        &self.capture
    }

    pub(crate) fn submission(&self) -> &SubmissionDisposition {
        &self.submission
    }

    pub(crate) fn into_parts(self) -> (PersonaTurn, NarrativeCapture, SubmissionDisposition) {
        (self.turn, self.capture, self.submission)
    }
}

#[derive(Debug)]
pub(crate) enum NarrativeRun {
    Completed(NarrativeDecision),
    Pending(NarrativePending),
    /// The turn was interrupted and could not be lowered again: its one
    /// re-lowering was already spent, or the fresh opportunity no longer grants
    /// speech. Nothing was submitted to the world, and this is not a pending
    /// state — there is nothing to retry.
    Interrupted(NarrativeInterruption),
}

#[derive(Debug)]
pub(crate) struct NarrativeInterruption {
    turn: PersonaTurn,
    subject: SubjectId,
    bound_scope_digest: String,
    /// Absent when the bound re-lowering was already spent, because the runner
    /// refuses that before it looks for what replaced the scope.
    fresh_scope_digest: Option<ScopeDigest>,
    /// The runner's own statement, in the existing vocabulary, that the world
    /// refused the act. A report field: never persisted into a checkpoint and
    /// never entered into a `NarrativeCapture`, so `evaluate_interpreter_loop`
    /// remains the only producer of capture gaps.
    gap: TranslationGapSummary,
}

impl NarrativeInterruption {
    pub(crate) fn persona_turn(&self) -> &PersonaTurn {
        &self.turn
    }

    pub(crate) fn subject(&self) -> SubjectId {
        self.subject
    }

    pub(crate) fn bound_scope_digest(&self) -> &str {
        &self.bound_scope_digest
    }

    pub(crate) fn fresh_scope_digest(&self) -> Option<&str> {
        self.fresh_scope_digest.as_ref().map(ScopeDigest::as_str)
    }

    pub(crate) fn gap(&self) -> &TranslationGapSummary {
        &self.gap
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ControllerPendingReason {
    /// The exact persisted connector invocation can be presented again. The
    /// connector decides whether this is an admitted completed replay.
    InferenceRetryable,
    /// The connector reported an indeterminate, expired, or otherwise
    /// non-replayable provider outcome. Integrity violations are errors and
    /// quarantine the controller organ instead of entering this state.
    InferenceRecoveryRequired,
    WorldUnavailable,
    WorldOutcomeUnknown,
    /// The exact work remains attached for reporting, but the journal owner
    /// must be reopened before this process may continue.
    StoreReopenRequired,
}

fn inference_pending_reason(error: &ControllerError) -> Option<ControllerPendingReason> {
    // The pending reason is a classification; the fault text is the only
    // record of what the provider or connector actually said, so it is logged
    // here where every lane's fault passes.
    tracing::info!(%error, "inference fault classified for pending");
    match error {
        ControllerError::Inference { source, .. } if source.integrity_was_violated() => None,
        ControllerError::Inference { source, .. } if source.recovery_required() => {
            Some(ControllerPendingReason::InferenceRecoveryRequired)
        }
        ControllerError::Inference { .. } => Some(ControllerPendingReason::InferenceRetryable),
        ControllerError::ProviderContract { .. } => {
            Some(ControllerPendingReason::InferenceRecoveryRequired)
        }
        _ => None,
    }
}

#[derive(Debug)]
pub(crate) struct NarrativePending {
    work: NarrativeCheckpoint,
    reason: ControllerPendingReason,
}

impl NarrativePending {
    pub(crate) fn mode(&self) -> ControllerMode {
        ControllerMode::NarrativePersona
    }

    pub(crate) fn reason(&self) -> ControllerPendingReason {
        self.reason
    }

    pub(crate) fn persona_prose(&self) -> Option<&str> {
        self.work.persona_prose()
    }
}

#[derive(Debug)]
pub(crate) struct OperationalDecision {
    capture: OperationalCapture,
    submission: SubmissionDisposition,
}

impl OperationalDecision {
    pub(crate) fn capture(&self) -> &OperationalCapture {
        &self.capture
    }

    pub(crate) fn submission(&self) -> &SubmissionDisposition {
        &self.submission
    }

    pub(crate) fn into_parts(self) -> (OperationalCapture, SubmissionDisposition) {
        (self.capture, self.submission)
    }
}

#[derive(Debug)]
pub(crate) enum OperationalRun {
    Completed(OperationalDecision),
    Pending(OperationalPending),
}

/// What one cell's run produced. A singleton delegates to the lane its
/// subject's controller mode names, and its outcome is that lane's own: the
/// detail path is not wrapped, re-shaped, or summarised.
#[derive(Debug)]
pub(crate) enum CellRun {
    Narrative(NarrativeRun),
    Operational(OperationalRun),
    Grouped(GroupedRun),
}

#[derive(Debug)]
pub(crate) struct ConstituentSubmission {
    pub(crate) subject: SubjectId,
    pub(crate) submission: SubmissionDisposition,
}

/// One coarse cell's outcome: one inference, N ordinary one-opportunity
/// submissions, and whatever the decode could not attribute.
#[derive(Debug)]
pub(crate) struct GroupedRun {
    pub(crate) cell: CellId,
    pub(crate) resolution: Resolution,
    pub(crate) submissions: Vec<ConstituentSubmission>,
    pub(crate) needs: Vec<ControllerNeed>,
    /// Set when the run stopped before every constituent was submitted. The
    /// constituents already in `submissions` are committed; the rest resume from
    /// the persisted row against the kernel's idempotency ledger.
    pub(crate) pending: Option<ControllerPendingReason>,
}

#[derive(Debug)]
pub(crate) struct OperationalPending {
    work: OperationalCheckpoint,
    reason: ControllerPendingReason,
}

impl OperationalPending {
    pub(crate) fn mode(&self) -> ControllerMode {
        ControllerMode::OperationalAgent
    }

    pub(crate) fn reason(&self) -> ControllerPendingReason {
        self.reason
    }
}

#[derive(Debug, Error)]
pub(crate) enum ControllerError {
    #[error("world snapshot failed: {0}")]
    Snapshot(MailboxError),
    #[error("subject has no exact {expected:?} opportunity")]
    NoOpportunity { expected: ControllerMode },
    #[error("subject has more than one current decision opportunity")]
    AmbiguousOpportunity,
    #[error("Eve opportunity does not match this exact controller command")]
    OpportunityMismatch,
    #[error("controller opportunity has no Speak affordance")]
    SpeakUnavailable,
    #[error("controller opportunity grants no affordance")]
    NoGrantedAffordance,
    #[error("Eve command ID does not match persisted controller work")]
    CommandMismatch,
    #[error("no persisted controller work exists for this Eve command")]
    MissingControllerWork,
    #[error("{purpose:?} inference failed: {source}")]
    Inference {
        purpose: InferencePurpose,
        #[source]
        source: InferenceFault,
    },
    #[error("{purpose:?} provider contract failed: {detail}")]
    ProviderContract {
        purpose: InferencePurpose,
        detail: String,
    },
    #[error("controller work was not persisted: {0}")]
    WorkPersistence(String),
    #[error("world command was rejected: {0}")]
    World(#[from] KernelError),
    #[error("controller view serialization failed: {0}")]
    Serialization(String),
}

impl ControllerError {
    /// Quarantine only the cognition organ. WorldMailbox and AppSession remain
    /// authoritative and available; this error must never become a daemon-wide
    /// fatal signal.
    pub(crate) fn requires_quarantine(&self) -> bool {
        match self {
            Self::WorkPersistence(_) | Self::Serialization(_) => true,
            Self::Inference { source, .. } => source.integrity_was_violated(),
            _ => false,
        }
    }
}

impl From<ControllerWorkStoreError> for ControllerError {
    fn from(error: ControllerWorkStoreError) -> Self {
        match error {
            ControllerWorkStoreError::CommandModeConflict => Self::CommandMismatch,
            error @ ControllerWorkStoreError::Fault { .. } => {
                Self::WorkPersistence(error.to_string())
            }
        }
    }
}

enum ControllerWorldCommand {
    Exercise(DecisionInvocation),
    Decline,
}

enum ControllerWorldSubmission {
    Completed(SubmissionDisposition),
    Pending(ControllerPendingReason),
}

pub(crate) struct ControllerRunner {
    mailbox: ControllerPort,
    /// The authoring lane's own narrowing of the same mailbox. It is opened
    /// here because this constructor is where a whole `WorldMailbox` is
    /// narrowed, and nowhere else in the world subtree holds one.
    elaboration: ElaborationPort,
    inference: Arc<dyn InferencePort>,
    work: Arc<dyn ControllerWorkStore>,
    models: ControllerModels,
}

impl ControllerRunner {
    /// Opens the complete controller organ around ports the caller supplies.
    /// Runtime binds the inference transport and the durable controller-work
    /// owner, because runtime is where the deployment configuration that names
    /// them lives. The caller still hands over a whole `WorldMailbox` — that
    /// stays the one owner-facing type — but this constructor is where it
    /// narrows to a `ControllerPort` before the runner ever sees it, so nothing
    /// inside this module can reach past the five requests a controller lane
    /// makes.
    pub(crate) fn open(
        mailbox: WorldMailbox,
        inference: Arc<dyn InferencePort>,
        work: Arc<dyn ControllerWorkStore>,
        models: ControllerModels,
    ) -> Result<Self, ControllerOpenError> {
        if !models.are_canonical() {
            return Err(ControllerOpenError::InvalidModels);
        }
        Ok(Self {
            mailbox: ControllerPort::new(mailbox.clone()),
            elaboration: ElaborationPort::new(mailbox),
            inference,
            work,
            models,
        })
    }

    /// The authoring lane, built from the same ports the decision lanes use.
    /// `NullEvidenceSource` is what production supplies until a retrieval organ
    /// lands; the bound that buys is stated where the source is defined.
    pub(crate) fn elaborator(&self) -> ElaborationRunner {
        ElaborationRunner::new(
            self.elaboration.clone(),
            Arc::clone(&self.inference),
            Arc::new(NullEvidenceSource),
            Arc::clone(&self.work),
            self.models.elaborator.clone(),
        )
    }

    /// Draft's authoring lane, built from the same inference transport and the
    /// same work store. The port is not one this runner holds: it carries the
    /// owner's verified evidence and therefore belongs to the request that
    /// asked for the work, and the evidence source is supplied by that request
    /// too. `NullEvidenceSource` stays the elaborator's; there is no mode flag
    /// choosing between them.
    pub(crate) fn seeder(
        &self,
        port: SeedPort,
        evidence: Arc<dyn EvidenceSource>,
        brief: Option<String>,
    ) -> SeedRunner {
        SeedRunner::new(
            port,
            Arc::clone(&self.inference),
            evidence,
            Arc::clone(&self.work),
            self.models.elaborator.clone(),
            brief,
        )
    }

    pub(crate) async fn custody_probe(&self) -> Result<ControllerWorkCustody, ControllerError> {
        Ok(self.work.custody_probe().await?)
    }

    pub(crate) async fn run_narrative(
        &self,
        command_id: CommandId,
        opportunity: &DecisionOpportunity,
    ) -> Result<NarrativeRun, ControllerError> {
        // The lane gate, where entering the Persona membrane is decided. It is
        // not `select`'s job: `select` proves the opportunity is live, and this
        // proves this lane may run it. Grouping never reaches here.
        if opportunity.controller_mode != ControllerMode::NarrativePersona {
            return Err(ControllerError::NoOpportunity {
                expected: ControllerMode::NarrativePersona,
            });
        }
        match self.work.lookup(command_id).await? {
            ControllerWorkLookup::Confirmed(ControllerWork::Narrative(checkpoint)) => {
                if !binds_same_scope(checkpoint.opportunity(), opportunity) {
                    return Err(ControllerError::OpportunityMismatch);
                }
                return self
                    .resume_persisted_narrative(command_id, checkpoint)
                    .await;
            }
            ControllerWorkLookup::CustodyUncertain(ControllerWork::Narrative(checkpoint)) => {
                if !binds_same_scope(checkpoint.opportunity(), opportunity) {
                    return Err(ControllerError::OpportunityMismatch);
                }
                return Ok(narrative_pending(
                    checkpoint,
                    ControllerPendingReason::StoreReopenRequired,
                ));
            }
            ControllerWorkLookup::Confirmed(
                ControllerWork::Operational(_)
                | ControllerWork::Grouped(_)
                | ControllerWork::Elaboration(_)
                | ControllerWork::Seed(_),
            )
            | ControllerWorkLookup::CustodyUncertain(
                ControllerWork::Operational(_)
                | ControllerWork::Grouped(_)
                | ControllerWork::Elaboration(_)
                | ControllerWork::Seed(_),
            ) => {
                return Err(ControllerError::CommandMismatch);
            }
            ControllerWorkLookup::Missing => {}
        }
        let selected = self.select(opportunity).await?;
        let identity = selected.subject.label.clone();
        let typed_view = selected.typed_view()?;
        let projector_context = selected.projector_context()?;
        let visible_stimulus = selected.visible_stimulus()?;
        let projector_prompt = build_projector_prompt(&ProjectorPrompt {
            identity: &identity,
            typed_context: &projector_context,
            visible_stimulus: &visible_stimulus,
            domain_guidance: "",
            word_budget: PERSONA_WORD_BUDGET,
        });
        let checkpoint = NarrativeCheckpoint::Projector {
            command_id,
            identity,
            typed_view,
            components: selected.subject.components.clone(),
            persona_model: self.models.persona.clone(),
            interpreter_model: self.models.interpreter.clone(),
            opportunity: selected.opportunity,
            granted: selected.granted.clone(),
            invocation: self.prepare(projector_request(
                command_id,
                &self.models.projector,
                projector_prompt,
            )?)?,
        };
        if self
            .persist(ControllerWork::Narrative(checkpoint.clone()))
            .await?
            == ControllerWorkWrite::CustodyUncertain
        {
            return Ok(narrative_pending(
                checkpoint,
                ControllerPendingReason::StoreReopenRequired,
            ));
        }
        self.resume_persisted_narrative(command_id, checkpoint)
            .await
    }

    pub(crate) async fn run_operational(
        &self,
        command_id: CommandId,
        opportunity: &DecisionOpportunity,
    ) -> Result<OperationalRun, ControllerError> {
        // The detail operational lane runs a subject the world assigned to an
        // operational agent. A coarsely represented `NarrativePersona` goes
        // through the grouped lane, never through this one.
        if opportunity.controller_mode != ControllerMode::OperationalAgent {
            return Err(ControllerError::NoOpportunity {
                expected: ControllerMode::OperationalAgent,
            });
        }
        match self.work.lookup(command_id).await? {
            ControllerWorkLookup::Confirmed(ControllerWork::Operational(checkpoint)) => {
                if !binds_same_scope(checkpoint.opportunity(), opportunity) {
                    return Err(ControllerError::OpportunityMismatch);
                }
                return self
                    .resume_persisted_operational(command_id, checkpoint)
                    .await;
            }
            ControllerWorkLookup::CustodyUncertain(ControllerWork::Operational(checkpoint)) => {
                if !binds_same_scope(checkpoint.opportunity(), opportunity) {
                    return Err(ControllerError::OpportunityMismatch);
                }
                return Ok(operational_pending(
                    checkpoint,
                    ControllerPendingReason::StoreReopenRequired,
                ));
            }
            ControllerWorkLookup::Confirmed(
                ControllerWork::Narrative(_)
                | ControllerWork::Grouped(_)
                | ControllerWork::Elaboration(_)
                | ControllerWork::Seed(_),
            )
            | ControllerWorkLookup::CustodyUncertain(
                ControllerWork::Narrative(_)
                | ControllerWork::Grouped(_)
                | ControllerWork::Elaboration(_)
                | ControllerWork::Seed(_),
            ) => {
                return Err(ControllerError::CommandMismatch);
            }
            ControllerWorkLookup::Missing => {}
        }
        let selected = self.select(opportunity).await?;
        let identity = selected.subject.label.clone();
        let typed_view = selected.typed_view()?;
        let agent_prompt = build_operational_agent_prompt(&OperationalAgentPrompt {
            identity: &identity,
            typed_view: &typed_view,
            available_tools: &catalog_signatures("", &selected.granted),
            decision_pressure: "Choose whether this decision owner should speak now.",
            domain_guidance: "",
            step_budget: TOOL_STEP_BUDGET,
        });
        let initial_conversation =
            match evaluate_operational_loop(&agent_prompt, &selected.granted, &[])? {
                OperationalLoopEvaluation::Continue { conversation } => conversation,
                OperationalLoopEvaluation::Complete { .. } => {
                    return Err(ControllerError::Serialization(
                        "empty operational evidence unexpectedly finalized".into(),
                    ));
                }
            };
        let checkpoint = OperationalCheckpoint::AgentInFlight {
            command_id,
            agent_prompt,
            opportunity: selected.opportunity,
            granted: selected.granted.clone(),
            completed: Vec::new(),
            invocation: self.prepare(operational_request(
                command_id,
                0,
                &self.models.operational_agent,
                &selected.granted,
                initial_conversation,
            )?)?,
        };
        if self
            .persist(ControllerWork::Operational(checkpoint.clone()))
            .await?
            == ControllerWorkWrite::CustodyUncertain
        {
            return Ok(operational_pending(
                checkpoint,
                ControllerPendingReason::StoreReopenRequired,
            ));
        }
        self.resume_persisted_operational(command_id, checkpoint)
            .await
    }

    async fn resume_persisted_operational(
        &self,
        command_id: CommandId,
        checkpoint: OperationalCheckpoint,
    ) -> Result<OperationalRun, ControllerError> {
        if checkpoint.command_id() != command_id {
            return Err(ControllerError::CommandMismatch);
        }
        match checkpoint {
            checkpoint @ OperationalCheckpoint::AgentInFlight { .. } => {
                self.run_operational_pending(checkpoint).await
            }
            checkpoint @ OperationalCheckpoint::ReadyToSubmit { .. } => {
                self.submit_operational(checkpoint).await
            }
            checkpoint @ OperationalCheckpoint::NoProposal { .. } => {
                self.submit_operational(checkpoint).await
            }
        }
    }

    async fn run_operational_pending(
        &self,
        mut checkpoint: OperationalCheckpoint,
    ) -> Result<OperationalRun, ControllerError> {
        loop {
            let OperationalCheckpoint::AgentInFlight {
                command_id,
                agent_prompt,
                opportunity,
                granted,
                mut completed,
                invocation,
            } = checkpoint.clone()
            else {
                return Err(ControllerError::Serialization(
                    "operational runner received a terminal checkpoint".into(),
                ));
            };
            self.ensure_scope_unchanged(&opportunity).await?;
            let model = invocation.invocation.request.model.clone();
            self.inference.lend_tool_results(
                &invocation,
                Box::new(OperationalOracle::new(&granted, &completed)),
            );
            let output = match self.infer(invocation).await {
                Ok(output) => output,
                Err(error) => match inference_pending_reason(&error) {
                    Some(reason) => return Ok(operational_pending(checkpoint, reason)),
                    None => return Err(error),
                },
            };
            completed.push(output);
            match evaluate_operational_loop(&agent_prompt, &granted, &completed) {
                Ok(OperationalLoopEvaluation::Complete { capture }) => {
                    let next = if capture.proposal.is_some() {
                        OperationalCheckpoint::ReadyToSubmit {
                            command_id,
                            agent_prompt,
                            opportunity,
                            granted,
                            completed,
                        }
                    } else {
                        OperationalCheckpoint::NoProposal {
                            command_id,
                            agent_prompt,
                            opportunity,
                            completed,
                        }
                    };
                    if self
                        .persist(ControllerWork::Operational(next.clone()))
                        .await?
                        == ControllerWorkWrite::CustodyUncertain
                    {
                        return Ok(operational_pending(
                            next,
                            ControllerPendingReason::StoreReopenRequired,
                        ));
                    }
                    return self.submit_operational(next).await;
                }
                Ok(OperationalLoopEvaluation::Continue { conversation }) => {
                    let round = completed.len();
                    let next = OperationalCheckpoint::AgentInFlight {
                        command_id,
                        agent_prompt,
                        opportunity,
                        completed,
                        invocation: self.prepare(operational_request(
                            command_id,
                            round,
                            &model,
                            &granted,
                            conversation,
                        )?)?,
                        granted,
                    };
                    match self
                        .persist(ControllerWork::Operational(next.clone()))
                        .await?
                    {
                        ControllerWorkWrite::Applied | ControllerWorkWrite::AlreadyPresent => {
                            checkpoint = next;
                        }
                        ControllerWorkWrite::CustodyUncertain => {
                            return Ok(operational_pending(
                                next,
                                ControllerPendingReason::StoreReopenRequired,
                            ));
                        }
                    }
                }
                Err(error) => match inference_pending_reason(&error) {
                    Some(reason) => return Ok(operational_pending(checkpoint, reason)),
                    None => return Err(error),
                },
            }
        }
    }
    /// One cell's cognition. It receives a `&Cell` and never a `Cover`: it
    /// cannot see other cells, the budget, or the agency graph, and the tick
    /// index reaches it only as an opaque value threaded into id derivation.
    pub(crate) async fn run_cell(&self, cell: &Cell) -> Result<CellRun, ControllerError> {
        match cell {
            Cell::Singleton { id, tick, member } => {
                // The driver's singleton turn takes a derived id, so it is as
                // replayable as its grouped ones. The operator's manual button
                // keeps issuing a fresh id, and that is correct: a manual turn
                // is not part of a replayable tick.
                let command_id = CommandId::for_cell_constituent(
                    member.opportunity.world_id,
                    *id,
                    member.subject,
                    *tick,
                );
                match member.opportunity.controller_mode {
                    ControllerMode::NarrativePersona => self
                        .run_narrative(command_id, &member.opportunity)
                        .await
                        .map(CellRun::Narrative),
                    ControllerMode::OperationalAgent => self
                        .run_operational(command_id, &member.opportunity)
                        .await
                        .map(CellRun::Operational),
                    ControllerMode::Human => Err(ControllerError::NoOpportunity {
                        expected: ControllerMode::OperationalAgent,
                    }),
                }
            }
            Cell::Group { id, tick, members } => self
                .run_group(*id, *tick, members)
                .await
                .map(CellRun::Grouped),
        }
    }

    async fn run_group(
        &self,
        cell: CellId,
        tick: TickIndex,
        members: &[Constituent],
    ) -> Result<GroupedRun, ControllerError> {
        let Some(first) = members.first() else {
            return Err(ControllerError::NoOpportunity {
                expected: ControllerMode::OperationalAgent,
            });
        };
        let world_id = first.opportunity.world_id;
        let command_id = CommandId::for_cell(world_id, cell, tick);
        match self.work.lookup(command_id).await? {
            ControllerWorkLookup::Confirmed(ControllerWork::Grouped(checkpoint)) => {
                if checkpoint.cell() != cell {
                    return Err(ControllerError::OpportunityMismatch);
                }
                return self.resume_persisted_group(command_id, checkpoint).await;
            }
            ControllerWorkLookup::CustodyUncertain(ControllerWork::Grouped(checkpoint)) => {
                if checkpoint.cell() != cell {
                    return Err(ControllerError::OpportunityMismatch);
                }
                return Ok(grouped_pending(
                    checkpoint,
                    ControllerPendingReason::StoreReopenRequired,
                ));
            }
            ControllerWorkLookup::Confirmed(
                ControllerWork::Narrative(_)
                | ControllerWork::Operational(_)
                | ControllerWork::Elaboration(_)
                | ControllerWork::Seed(_),
            )
            | ControllerWorkLookup::CustodyUncertain(
                ControllerWork::Narrative(_)
                | ControllerWork::Operational(_)
                | ControllerWork::Elaboration(_)
                | ControllerWork::Seed(_),
            ) => {
                return Err(ControllerError::CommandMismatch);
            }
            ControllerWorkLookup::Missing => {}
        }

        // One snapshot for the whole cell, and no mid-stage rechecks. The detail
        // lanes' per-stage recheck is an early exit, not a correctness gate: the
        // scope digest is re-derived at admission, so a stale grouped proposal is
        // refused with an honest `ScopeChanged` rather than committed.
        let snapshot = self
            .mailbox
            .snapshot()
            .await
            .map_err(ControllerError::Snapshot)?;
        let mut selected = Vec::new();
        let mut constituents = Vec::new();
        let mut needs = Vec::new();
        for member in members {
            match select_one(&snapshot, &member.opportunity) {
                Ok(decision) => {
                    constituents.push(ConstituentWork {
                        subject: member.subject,
                        opportunity: decision.opportunity.clone(),
                        granted: decision.granted.clone(),
                        command_id: CommandId::for_cell_constituent(
                            world_id,
                            cell,
                            member.subject,
                            tick,
                        ),
                    });
                    selected.push(decision);
                }
                // A subject whose opportunity moved between the cover and the
                // cell was not active when the cell ran. It is dropped rather
                // than declined: declining a stale opportunity would be refused
                // anyway, and the drop is recorded where a reader can see it.
                Err(error) => needs.push(ControllerNeed {
                    detail: format!("a constituent left the cell before selection: {error}"),
                }),
            }
        }
        if constituents.is_empty() {
            return Ok(GroupedRun {
                cell,
                resolution: Resolution::Coarse { constituents: 0 },
                submissions: Vec::new(),
                needs,
                pending: None,
            });
        }

        let views = partitioned_views(&selected)?;
        let labeled: Vec<LabeledView<'_>> = views
            .iter()
            .map(|view| LabeledView {
                handle: &view.handle,
                identity: &view.identity,
                typed_view: &view.typed_view,
                tool_signatures: &view.tool_signatures,
            })
            .collect();
        let agent_prompt = build_grouped_agent_prompt(&GroupedAgentPrompt {
            views: &labeled,
            decision_pressure: "Choose whether each decision owner should act now.",
            domain_guidance: "",
            step_budget: CELL_TOOL_STEP_BUDGET,
        });
        let initial_conversation = match evaluate_grouped_loop(&agent_prompt, &constituents, &[])? {
            GroupedLoopEvaluation::Continue { conversation } => conversation,
            GroupedLoopEvaluation::Complete { .. } => {
                return Err(ControllerError::Serialization(
                    "empty grouped evidence unexpectedly finalized".into(),
                ));
            }
        };
        let checkpoint = GroupedCheckpoint::AgentInFlight {
            command_id,
            cell,
            tick,
            agent_prompt,
            completed: Vec::new(),
            invocation: self.prepare(grouped_request(
                command_id,
                0,
                &self.models.operational_agent,
                &constituents,
                initial_conversation,
            )?)?,
            constituents,
        };
        let mut run = if self
            .persist(ControllerWork::Grouped(checkpoint.clone()))
            .await?
            == ControllerWorkWrite::CustodyUncertain
        {
            grouped_pending(checkpoint, ControllerPendingReason::StoreReopenRequired)
        } else {
            self.resume_persisted_group(command_id, checkpoint).await?
        };
        // The constituents that left the cell are reported beside the run they
        // were dropped from, not written into the row: they produced no
        // cognition, so they are not cognition evidence.
        run.needs.splice(0..0, needs);
        Ok(run)
    }

    async fn resume_persisted_group(
        &self,
        command_id: CommandId,
        checkpoint: GroupedCheckpoint,
    ) -> Result<GroupedRun, ControllerError> {
        if checkpoint.command_id() != command_id {
            return Err(ControllerError::CommandMismatch);
        }
        match checkpoint {
            checkpoint @ GroupedCheckpoint::AgentInFlight { .. } => {
                self.run_group_pending(checkpoint).await
            }
            checkpoint @ GroupedCheckpoint::Submitting { .. } => {
                self.submit_group(checkpoint).await
            }
        }
    }

    async fn run_group_pending(
        &self,
        mut checkpoint: GroupedCheckpoint,
    ) -> Result<GroupedRun, ControllerError> {
        loop {
            let GroupedCheckpoint::AgentInFlight {
                command_id,
                cell,
                tick,
                agent_prompt,
                constituents,
                mut completed,
                invocation,
            } = checkpoint.clone()
            else {
                return Err(ControllerError::Serialization(
                    "grouped runner received a terminal checkpoint".into(),
                ));
            };
            let model = invocation.invocation.request.model.clone();
            self.inference.lend_tool_results(
                &invocation,
                Box::new(GroupedOracle::new(&constituents, &completed)),
            );
            let output = match self.infer(invocation).await {
                Ok(output) => output,
                Err(error) => match inference_pending_reason(&error) {
                    Some(reason) => return Ok(grouped_pending(checkpoint, reason)),
                    None => return Err(error),
                },
            };
            completed.push(output);
            match evaluate_grouped_loop(&agent_prompt, &constituents, &completed) {
                Ok(GroupedLoopEvaluation::Complete { .. }) => {
                    let next = GroupedCheckpoint::Submitting {
                        command_id,
                        cell,
                        tick,
                        agent_prompt,
                        constituents,
                        completed,
                    };
                    if self.persist(ControllerWork::Grouped(next.clone())).await?
                        == ControllerWorkWrite::CustodyUncertain
                    {
                        return Ok(grouped_pending(
                            next,
                            ControllerPendingReason::StoreReopenRequired,
                        ));
                    }
                    return self.submit_group(next).await;
                }
                Ok(GroupedLoopEvaluation::Continue { conversation }) => {
                    let round = completed.len();
                    let next = GroupedCheckpoint::AgentInFlight {
                        command_id,
                        cell,
                        tick,
                        agent_prompt,
                        invocation: self.prepare(grouped_request(
                            command_id,
                            round,
                            &model,
                            &constituents,
                            conversation,
                        )?)?,
                        completed,
                        constituents,
                    };
                    match self.persist(ControllerWork::Grouped(next.clone())).await? {
                        ControllerWorkWrite::Applied | ControllerWorkWrite::AlreadyPresent => {
                            checkpoint = next;
                        }
                        ControllerWorkWrite::CustodyUncertain => {
                            return Ok(grouped_pending(
                                next,
                                ControllerPendingReason::StoreReopenRequired,
                            ));
                        }
                    }
                }
                Err(error) => match inference_pending_reason(&error) {
                    Some(reason) => return Ok(grouped_pending(checkpoint, reason)),
                    None => return Err(error),
                },
            }
        }
    }

    /// One constituent at a time, in handle order, each through the same
    /// `submit_controller_world` a detail turn uses. There is no batch command
    /// body and no aggregate receipt: the only way a cell reaches the kernel is
    /// one opportunity at a time, which is what keeps a cell-owned proposal
    /// unrepresentable.
    ///
    /// A constituent that proposed nothing declines. Otherwise the world cannot
    /// tell "was attended and stayed silent" from "was never attended", and the
    /// resume path loses its only record that the turn finished.
    async fn submit_group(
        &self,
        checkpoint: GroupedCheckpoint,
    ) -> Result<GroupedRun, ControllerError> {
        let GroupedCheckpoint::Submitting {
            cell,
            agent_prompt,
            constituents,
            completed,
            ..
        } = &checkpoint
        else {
            return Err(ControllerError::Serialization(
                "grouped submission requires terminal controller work".into(),
            ));
        };
        if matches!(
            self.work.custody_probe().await?,
            ControllerWorkCustody::Uncertain { .. }
        ) {
            return Ok(grouped_pending(
                checkpoint.clone(),
                ControllerPendingReason::StoreReopenRequired,
            ));
        }
        let capture = derive_grouped_capture(agent_prompt, constituents, completed)?;
        let resolution = Resolution::Coarse {
            constituents: constituents.len(),
        };
        let mut needs = capture.needs;
        let mut submissions = Vec::new();
        // Declines first. A decline changes no other subject's scope, so every
        // silent constituent's turn is consumed. An exercise can change what its
        // neighbours bound to, and co-located subjects are exactly
        // the ones a connected cover groups. The kernel refuses a proposal
        // reasoned from a scope that no longer holds. That refusal is the honest
        // outcome; ordering the non-mutating submissions ahead of it loses no
        // turns and costs nothing.
        let mut ordered: Vec<(usize, &ConstituentWork)> = constituents.iter().enumerate().collect();
        ordered.sort_by_key(|(handle, _)| capture.proposals.contains_key(handle));
        for (handle, constituent) in ordered {
            let command = match capture.proposals.get(&handle) {
                Some(invocation) => ControllerWorldCommand::Exercise(invocation.clone()),
                None => ControllerWorldCommand::Decline,
            };
            match self
                .submit_controller_world(constituent.command_id, &constituent.opportunity, command)
                .await
            {
                Ok(ControllerWorldSubmission::Completed(submission)) => {
                    submissions.push(ConstituentSubmission {
                        subject: constituent.subject,
                        submission,
                    });
                }
                Ok(ControllerWorldSubmission::Pending(reason)) => {
                    return Ok(GroupedRun {
                        cell: *cell,
                        resolution,
                        submissions,
                        needs,
                        pending: Some(reason),
                    });
                }
                // One constituent's refusal is that constituent's outcome. The
                // cell is not a transaction: every other handle's proposal was
                // bound to its own opportunity and is unaffected.
                Err(ControllerError::World(error)) => needs.push(ControllerNeed {
                    detail: format!("constituent c{handle} was refused by the world: {error}"),
                }),
                Err(error) => return Err(error),
            }
        }
        submissions.sort_by_key(|entry| {
            constituents
                .iter()
                .position(|constituent| constituent.subject == entry.subject)
                .unwrap_or(usize::MAX)
        });
        Ok(GroupedRun {
            cell: *cell,
            resolution,
            submissions,
            needs,
            pending: None,
        })
    }

    fn prepare(&self, request: InferenceRequest) -> Result<PreparedInference, ControllerError> {
        let purpose = request.purpose;
        self.inference
            .prepare(request)
            .map_err(|source| ControllerError::Inference { purpose, source })
    }

    async fn infer(&self, request: PreparedInference) -> Result<InferenceOutput, ControllerError> {
        let purpose = request.purpose;
        self.inference
            .infer(request)
            .await
            .map_err(|source| ControllerError::Inference { purpose, source })
    }

    async fn select(
        &self,
        exact_opportunity: &DecisionOpportunity,
    ) -> Result<SelectedDecision, ControllerError> {
        let snapshot = self
            .mailbox
            .snapshot()
            .await
            .map_err(ControllerError::Snapshot)?;
        select_one(&snapshot, exact_opportunity)
    }

    /// The scope this run bound still derives the same digest. The operational
    /// lane's early abort, and only its: a lane holding nothing to preserve
    /// stops as soon as the scope moves, while the narrative lane runs to the
    /// kernel's refusal so its prose and receipts survive to be lowered again.
    async fn ensure_scope_unchanged(
        &self,
        opportunity: &DecisionOpportunity,
    ) -> Result<(), ControllerError> {
        self.select(opportunity).await.map(|_| ())
    }

    async fn resume_persisted_narrative(
        &self,
        command_id: CommandId,
        checkpoint: NarrativeCheckpoint,
    ) -> Result<NarrativeRun, ControllerError> {
        if checkpoint.command_id() != command_id {
            return Err(ControllerError::CommandMismatch);
        }
        match checkpoint {
            checkpoint @ NarrativeCheckpoint::Projector { .. } => {
                self.run_projector(checkpoint).await
            }
            checkpoint @ NarrativeCheckpoint::Persona { .. } => self.run_persona(checkpoint).await,
            checkpoint @ NarrativeCheckpoint::InterpreterInFlight { .. } => {
                self.interpret_pending(checkpoint).await
            }
            checkpoint @ NarrativeCheckpoint::ReadyToSubmit { .. } => {
                self.submit_narrative(checkpoint).await
            }
            checkpoint @ NarrativeCheckpoint::NoProposal { .. } => {
                self.submit_narrative(checkpoint).await
            }
        }
    }

    async fn run_projector(
        &self,
        checkpoint: NarrativeCheckpoint,
    ) -> Result<NarrativeRun, ControllerError> {
        let NarrativeCheckpoint::Projector {
            command_id,
            identity,
            typed_view,
            components,
            persona_model,
            interpreter_model,
            opportunity,
            granted,
            invocation,
        } = checkpoint.clone()
        else {
            return Err(ControllerError::Serialization(
                "Projector runner received another checkpoint".into(),
            ));
        };
        let projector_output = match self.infer(invocation).await {
            Ok(output) => output,
            Err(error) => match inference_pending_reason(&error) {
                Some(reason) => return Ok(narrative_pending(checkpoint, reason)),
                None => return Err(error),
            },
        };
        let lived_stream = match projector_output
            .clone()
            .prose_only(InferencePurpose::Projector)
        {
            Ok((prose, _)) => prose,
            Err(error) => match inference_pending_reason(&error) {
                Some(reason) => return Ok(narrative_pending(checkpoint, reason)),
                None => return Err(error),
            },
        };
        let persona_prompt = build_persona_prompt(&PersonaPrompt {
            identity: &identity,
            lived_stream: &lived_stream,
            domain_guidance: "",
            word_budget: PERSONA_WORD_BUDGET,
        });
        let next = NarrativeCheckpoint::Persona {
            command_id,
            identity,
            typed_view,
            components,
            interpreter_model,
            opportunity,
            granted,
            projector_output,
            invocation: self.prepare(persona_request(
                command_id,
                &persona_model,
                persona_prompt,
            )?)?,
        };
        match self
            .persist(ControllerWork::Narrative(next.clone()))
            .await?
        {
            ControllerWorkWrite::CustodyUncertain => Ok(narrative_pending(
                next,
                ControllerPendingReason::StoreReopenRequired,
            )),
            ControllerWorkWrite::Applied | ControllerWorkWrite::AlreadyPresent => {
                self.run_persona(next).await
            }
        }
    }

    async fn run_persona(
        &self,
        checkpoint: NarrativeCheckpoint,
    ) -> Result<NarrativeRun, ControllerError> {
        let NarrativeCheckpoint::Persona {
            command_id,
            identity,
            typed_view,
            components,
            interpreter_model,
            opportunity,
            granted,
            projector_output,
            invocation,
        } = checkpoint.clone()
        else {
            return Err(ControllerError::Serialization(
                "Persona runner received another checkpoint".into(),
            ));
        };
        let (lived_stream, projector_receipt) = projector_output
            .clone()
            .prose_only(InferencePurpose::Projector)?;
        let persona = match self
            .infer(invocation)
            .await
            .and_then(|output| output.prose_only(InferencePurpose::Persona))
        {
            Ok(persona) => persona,
            Err(error) => match inference_pending_reason(&error) {
                Some(reason) => return Ok(narrative_pending(checkpoint, reason)),
                None => return Err(error),
            },
        };
        let turn = PersonaTurn::record(
            PersonaTurnBinding {
                world_id: encoded_id(&opportunity.world_id)?,
                controller_id: encoded_id(&opportunity.controller_id)?,
                opportunity_digest: opportunity.digest()?,
                world_revision: opportunity.revision,
                scope_digest: opportunity.scope_digest.as_str().to_owned(),
                projector_receipt_digest: projector_receipt,
                persona_inference_receipt_digest: persona.1,
                interrupted_from: None,
            },
            persona.0,
        );
        let interpreter_prompt = build_interpreter_prompt(&InterpreterPrompt {
            identity: &identity,
            typed_context: &typed_view,
            lived_stream: &lived_stream,
            persona_output: turn.source_prose(),
            output_schema: None,
            domain_guidance: "",
        });
        let initial_conversation = match evaluate_interpreter_loop(&turn, &interpreter_prompt, &[])?
        {
            InterpreterLoopEvaluation::Continue { conversation } => conversation,
            InterpreterLoopEvaluation::Complete { .. } => {
                return Err(ControllerError::Serialization(
                    "empty Interpreter evidence unexpectedly finalized".into(),
                ));
            }
        };
        let next = NarrativeCheckpoint::InterpreterInFlight {
            command_id,
            turn,
            interpreter_prompt,
            components,
            interruption: None,
            opportunity,
            granted,
            completed: Vec::new(),
            invocation: self.prepare(interpreter_request(
                command_id,
                0,
                &interpreter_model,
                initial_conversation,
            )?)?,
        };
        match self
            .persist(ControllerWork::Narrative(next.clone()))
            .await?
        {
            ControllerWorkWrite::CustodyUncertain => Ok(narrative_pending(
                next,
                ControllerPendingReason::StoreReopenRequired,
            )),
            ControllerWorkWrite::Applied | ControllerWorkWrite::AlreadyPresent => {
                self.interpret_pending(next).await
            }
        }
    }

    async fn interpret_pending(
        &self,
        mut checkpoint: NarrativeCheckpoint,
    ) -> Result<NarrativeRun, ControllerError> {
        loop {
            let NarrativeCheckpoint::InterpreterInFlight {
                command_id,
                turn,
                interpreter_prompt,
                components,
                interruption,
                opportunity,
                granted,
                mut completed,
                invocation,
            } = checkpoint.clone()
            else {
                return Err(ControllerError::Serialization(
                    "Interpreter runner received a terminal checkpoint".into(),
                ));
            };
            let model = invocation.invocation.request.model.clone();
            self.inference.lend_tool_results(
                &invocation,
                Box::new(InterpreterOracle::new(&turn, &completed)),
            );
            let output = match self.infer(invocation).await {
                Ok(output) => output,
                Err(error) => match inference_pending_reason(&error) {
                    Some(reason) => return Ok(narrative_pending(checkpoint, reason)),
                    None => return Err(error),
                },
            };
            completed.push(output);
            match evaluate_interpreter_loop(&turn, &interpreter_prompt, &completed) {
                Ok(InterpreterLoopEvaluation::Complete { capture }) => {
                    let next = if capture.proposal.is_some() {
                        NarrativeCheckpoint::ReadyToSubmit {
                            command_id,
                            turn,
                            interpreter_prompt,
                            components,
                            interruption,
                            opportunity,
                            granted,
                            completed,
                        }
                    } else {
                        NarrativeCheckpoint::NoProposal {
                            command_id,
                            turn,
                            interpreter_prompt,
                            components,
                            interruption,
                            opportunity,
                            completed,
                        }
                    };
                    if self
                        .persist(ControllerWork::Narrative(next.clone()))
                        .await?
                        == ControllerWorkWrite::CustodyUncertain
                    {
                        return Ok(narrative_pending(
                            next,
                            ControllerPendingReason::StoreReopenRequired,
                        ));
                    }
                    return self.submit_narrative(next).await;
                }
                Ok(InterpreterLoopEvaluation::Continue { conversation }) => {
                    let round = interpreter_round(&interruption, &completed);
                    let next = NarrativeCheckpoint::InterpreterInFlight {
                        command_id,
                        turn,
                        interpreter_prompt,
                        components,
                        interruption,
                        opportunity,
                        granted,
                        completed,
                        invocation: self.prepare(interpreter_request(
                            command_id,
                            round,
                            &model,
                            conversation,
                        )?)?,
                    };
                    match self
                        .persist(ControllerWork::Narrative(next.clone()))
                        .await?
                    {
                        ControllerWorkWrite::Applied | ControllerWorkWrite::AlreadyPresent => {
                            checkpoint = next;
                        }
                        ControllerWorkWrite::CustodyUncertain => {
                            return Ok(narrative_pending(
                                next,
                                ControllerPendingReason::StoreReopenRequired,
                            ));
                        }
                    }
                }
                Err(error) => match inference_pending_reason(&error) {
                    Some(reason) => return Ok(narrative_pending(checkpoint, reason)),
                    None => return Err(error),
                },
            }
        }
    }

    async fn submit_narrative(
        &self,
        checkpoint: NarrativeCheckpoint,
    ) -> Result<NarrativeRun, ControllerError> {
        let (command_id, opportunity) = match &checkpoint {
            NarrativeCheckpoint::ReadyToSubmit {
                command_id,
                opportunity,
                ..
            }
            | NarrativeCheckpoint::NoProposal {
                command_id,
                opportunity,
                ..
            } => (*command_id, opportunity.clone()),
            _ => {
                return Err(ControllerError::Serialization(
                    "narrative submission requires terminal controller work".into(),
                ));
            }
        };
        if matches!(
            self.work.custody_probe().await?,
            ControllerWorkCustody::Uncertain { .. }
        ) {
            return Ok(narrative_pending(
                checkpoint,
                ControllerPendingReason::StoreReopenRequired,
            ));
        }
        let command = match &checkpoint {
            NarrativeCheckpoint::ReadyToSubmit { .. } => {
                ControllerWorldCommand::Exercise(narrative_invocation(&checkpoint)?)
            }
            NarrativeCheckpoint::NoProposal { .. } => ControllerWorldCommand::Decline,
            _ => unreachable!("terminal checkpoint was established above"),
        };
        match self
            .submit_controller_world(command_id, &opportunity, command)
            .await
        {
            Ok(ControllerWorldSubmission::Completed(submission)) => {
                completed_narrative(&checkpoint, submission)
            }
            Ok(ControllerWorldSubmission::Pending(reason)) => {
                Ok(narrative_pending(checkpoint, reason))
            }
            // Both of `submit_controller_world`'s kernel-error mappings funnel
            // here; they are the same fact and are not distinguished.
            Err(ControllerError::World(KernelError::ScopeChanged { .. })) => {
                self.interrupted(checkpoint).await
            }
            Err(error) => Err(error),
        }
    }

    /// One place, one event. A scope digest that moved between the turn and the
    /// commit is an interruption whatever moved it: a neighbour's speech, a
    /// transfer, a route closing, a revocation, a vacated office, a routine
    /// rolling on a tick, an elaborator patch, a consumer document. The Persona
    /// is never re-run; only the binding is renewed, and the same prose is
    /// lowered a second time against an anonymous account of what moved.
    async fn interrupted(
        &self,
        checkpoint: NarrativeCheckpoint,
    ) -> Result<NarrativeRun, ControllerError> {
        let (command_id, turn, interpreter_prompt, opportunity, completed, components) =
            match checkpoint.clone() {
                NarrativeCheckpoint::ReadyToSubmit {
                    command_id,
                    turn,
                    interpreter_prompt,
                    components,
                    opportunity,
                    completed,
                    ..
                }
                | NarrativeCheckpoint::NoProposal {
                    command_id,
                    turn,
                    interpreter_prompt,
                    components,
                    opportunity,
                    completed,
                    ..
                } => (
                    command_id,
                    turn,
                    interpreter_prompt,
                    opportunity,
                    completed,
                    components,
                ),
                _ => {
                    return Err(ControllerError::Serialization(
                        "an interruption requires terminal controller work".into(),
                    ));
                }
            };
        // The bound of one re-lowering, read before any inference or selection
        // is spent on it.
        if turn.binding().interrupted_from.is_some() {
            return overtaken(&checkpoint, None);
        }
        let snapshot = self
            .mailbox
            .snapshot()
            .await
            .map_err(ControllerError::Snapshot)?;
        let fresh = select_fresh(&snapshot, &opportunity)?;
        if fresh.opportunity.scope_digest == opportunity.scope_digest {
            // The refusal was not this scope's move, and the runner does not
            // invent a second explanation for it.
            return Err(ControllerError::World(KernelError::ScopeChanged {
                scope: opportunity.scope,
                expected: opportunity.scope_digest.clone(),
                actual: fresh.opportunity.scope_digest,
            }));
        }
        // No branch here checks the fresh grant set for `SPEAK_KIND`: grants
        // are recorded in an insert-only ledger (`affordance_grants` never
        // removes an entry), and this checkpoint could only reach
        // `ReadyToSubmit`/`NoProposal` by having speech granted for this
        // scope already, so a fresh opportunity on the same scope always
        // carries it too.
        let interruption = Interruption {
            components: fresh.subject.components.clone(),
            overheard: fresh.overheard_since(turn.binding().world_revision),
            discarded: completed,
        };
        let next_turn = PersonaTurn::record(
            PersonaTurnBinding {
                world_id: encoded_id(&fresh.opportunity.world_id)?,
                controller_id: encoded_id(&fresh.opportunity.controller_id)?,
                opportunity_digest: fresh.opportunity.digest()?,
                world_revision: fresh.opportunity.revision,
                scope_digest: fresh.opportunity.scope_digest.as_str().to_owned(),
                projector_receipt_digest: turn.binding().projector_receipt_digest.clone(),
                persona_inference_receipt_digest: turn
                    .binding()
                    .persona_inference_receipt_digest
                    .clone(),
                interrupted_from: Some(Box::new(turn.binding().clone())),
            },
            turn.source_prose(),
        );
        let next_prompt = format!(
            "{interpreter_prompt}{}",
            interruption_section(&components, &opportunity, &interruption, &fresh.opportunity,)
        );
        let conversation = match evaluate_interpreter_loop(&next_turn, &next_prompt, &[])? {
            InterpreterLoopEvaluation::Continue { conversation } => conversation,
            InterpreterLoopEvaluation::Complete { .. } => {
                return Err(ControllerError::Serialization(
                    "empty Interpreter evidence unexpectedly finalized".into(),
                ));
            }
        };
        let round = interpreter_round(&Some(interruption.clone()), &[]);
        let next = NarrativeCheckpoint::InterpreterInFlight {
            command_id,
            turn: next_turn,
            interpreter_prompt: next_prompt,
            components,
            interruption: Some(interruption),
            granted: fresh.granted,
            opportunity: fresh.opportunity,
            completed: Vec::new(),
            invocation: self.prepare(interpreter_request(
                command_id,
                round,
                &self.models.interpreter,
                conversation,
            )?)?,
        };
        if self
            .persist(ControllerWork::Narrative(next.clone()))
            .await?
            == ControllerWorkWrite::CustodyUncertain
        {
            return Ok(narrative_pending(
                next,
                ControllerPendingReason::StoreReopenRequired,
            ));
        }
        // The re-lowering rejoins the ordinary loop, which is what closes the
        // cycle `interpret_pending -> submit_narrative -> interrupted`. One
        // boxed edge is all the compiler needs to size it.
        Box::pin(self.interpret_pending(next)).await
    }

    async fn persist(&self, work: ControllerWork) -> Result<ControllerWorkWrite, ControllerError> {
        Ok(self.work.persist(&work).await?)
    }

    async fn submit_operational(
        &self,
        checkpoint: OperationalCheckpoint,
    ) -> Result<OperationalRun, ControllerError> {
        let (command_id, opportunity) = match &checkpoint {
            OperationalCheckpoint::ReadyToSubmit {
                command_id,
                opportunity,
                ..
            }
            | OperationalCheckpoint::NoProposal {
                command_id,
                opportunity,
                ..
            } => (*command_id, opportunity.clone()),
            OperationalCheckpoint::AgentInFlight { .. } => {
                return Err(ControllerError::Serialization(
                    "operational submission requires terminal controller work".into(),
                ));
            }
        };
        if matches!(
            self.work.custody_probe().await?,
            ControllerWorkCustody::Uncertain { .. }
        ) {
            return Ok(operational_pending(
                checkpoint,
                ControllerPendingReason::StoreReopenRequired,
            ));
        }
        let command = match &checkpoint {
            OperationalCheckpoint::ReadyToSubmit { .. } => {
                ControllerWorldCommand::Exercise(operational_invocation(&checkpoint)?)
            }
            OperationalCheckpoint::NoProposal { .. } => ControllerWorldCommand::Decline,
            OperationalCheckpoint::AgentInFlight { .. } => {
                unreachable!("terminal checkpoint was established above")
            }
        };
        match self
            .submit_controller_world(command_id, &opportunity, command)
            .await?
        {
            ControllerWorldSubmission::Completed(submission) => {
                completed_operational(&checkpoint, submission)
            }
            ControllerWorldSubmission::Pending(reason) => {
                Ok(operational_pending(checkpoint, reason))
            }
        }
    }

    async fn submit_controller_world(
        &self,
        command_id: CommandId,
        opportunity: &DecisionOpportunity,
        command: ControllerWorldCommand,
    ) -> Result<ControllerWorldSubmission, ControllerError> {
        let committed = match &command {
            ControllerWorldCommand::Exercise(invocation) => {
                self.mailbox
                    .controller_receipt(command_id, opportunity, invocation)
                    .await
            }
            ControllerWorldCommand::Decline => {
                self.mailbox
                    .controller_decline_receipt(command_id, opportunity)
                    .await
            }
        };
        match committed {
            Ok(Some(receipt)) => {
                let submission = match command {
                    ControllerWorldCommand::Exercise(_) => {
                        SubmissionDisposition::PreviouslyConfirmed(receipt)
                    }
                    ControllerWorldCommand::Decline => {
                        SubmissionDisposition::NoProposal(SubmitReceipt::AlreadyApplied(receipt))
                    }
                };
                return Ok(ControllerWorldSubmission::Completed(submission));
            }
            Ok(None) => {}
            Err(MailboxError::Unavailable) => {
                return Ok(ControllerWorldSubmission::Pending(
                    ControllerPendingReason::WorldUnavailable,
                ));
            }
            Err(MailboxError::OutcomeUnknown { .. }) => {
                return Ok(ControllerWorldSubmission::Pending(
                    ControllerPendingReason::WorldOutcomeUnknown,
                ));
            }
            Err(MailboxError::Kernel(error)) => return Err(ControllerError::World(error)),
        }

        let submitted = match command {
            ControllerWorldCommand::Exercise(invocation) => self
                .mailbox
                .submit_controller(command_id, opportunity, invocation)
                .await
                .map(SubmissionDisposition::Completed),
            ControllerWorldCommand::Decline => self
                .mailbox
                .submit_controller_decline(command_id, opportunity)
                .await
                .map(SubmissionDisposition::NoProposal),
        };
        match submitted {
            Ok(submission) => Ok(ControllerWorldSubmission::Completed(submission)),
            Err(MailboxError::OutcomeUnknown { .. }) => Ok(ControllerWorldSubmission::Pending(
                ControllerPendingReason::WorldOutcomeUnknown,
            )),
            Err(MailboxError::Unavailable) => Ok(ControllerWorldSubmission::Pending(
                ControllerPendingReason::WorldUnavailable,
            )),
            Err(MailboxError::Kernel(error)) => Err(ControllerError::World(error)),
        }
    }
}

#[derive(Clone)]
struct SelectedDecision {
    snapshot: WorldSnapshot,
    subject: SubjectSnapshot,
    opportunity: DecisionOpportunity,
    /// The entries this opportunity grants, in `AffordanceId` order. Every
    /// model-facing surface is a projection of exactly this list.
    granted: Vec<AffordanceSnapshot>,
}

impl SelectedDecision {
    fn projector_context(&self) -> Result<String, ControllerError> {
        serde_json::to_string_pretty(&json!({
            "world": {
                "title": self.snapshot.title,
            },
            "subject": {
                "label": self.subject.label,
                "kind": self.subject.kind,
            },
            "now": self.snapshot.now,
            "permission": catalog_permissions(&self.granted),
        }))
        .map_err(|error| ControllerError::Serialization(error.to_string()))
    }

    fn typed_view(&self) -> Result<String, ControllerError> {
        serde_json::to_string_pretty(&json!({
            "world": {
                "title": self.snapshot.title,
                "revision": self.snapshot.revision,
                "state_digest": self.snapshot.state_digest,
            },
            "subject": {
                "id": self.subject.id,
                "label": self.subject.label,
                "kind": self.subject.kind,
                "place": self.typed_place(),
            },
            "permission": catalog_permissions(&self.granted),
            "routes": self.typed_routes(),
            "holdings": self.typed_holdings(),
            "dependencies": self.typed_dependencies(),
            "authority": self.typed_authority(),
            "offices_held": Self::typed_offices(&self.subject.offices_held),
            "offices_granted": Self::typed_offices(&self.subject.offices_granted),
            "redress": self.typed_redress(),
            "knowledge": self.typed_knowledge(),
            "channels": self.typed_channels(),
            "now": self.snapshot.now,
            "commitments": self.typed_commitments(),
            "pressures": self.typed_pressures(),
        }))
        .map_err(|error| ControllerError::Serialization(error.to_string()))
    }

    /// The acting subject's own promises. IDs, not labels: resolving a
    /// counterparty's name is a `Knowledge` question.
    fn typed_commitments(&self) -> Vec<Value> {
        self.subject
            .commitments
            .iter()
            .map(|commitment| {
                json!({
                    "key": commitment.key,
                    "kind": commitment.kind,
                    "counterparty": commitment.counterparty,
                    "due": commitment.due,
                    "period": commitment.period,
                    "past_due": commitment.past_due,
                })
            })
            .collect()
    }

    /// Pressure on self. Pressure this subject sources is another subject's
    /// state and never enters: the actor already sees the commitment that
    /// produced it.
    fn typed_pressures(&self) -> Vec<Value> {
        self.subject
            .pressures
            .iter()
            .map(|pressure| json!({"source": pressure.source, "magnitude": pressure.magnitude}))
            .collect()
    }

    /// The acting subject's own jurisdictions, with no label resolved for
    /// anything inside a target: a jurisdiction may name places and subjects
    /// this subject has never heard of, and label resolution is a `Knowledge`
    /// question. Nothing here marks whether a jurisdiction is occupied.
    fn typed_authority(&self) -> Vec<Value> {
        self.subject
            .components
            .authority
            .iter()
            .map(Self::typed_grant)
            .collect()
    }

    fn typed_grant(grant: &AuthorityGrant) -> Value {
        json!({"kind": grant.kind, "over": grant.over})
    }

    /// The offices this subject occupies, and — for an institution — the
    /// offices it grants. No global office gazetteer, and no other subject's
    /// grants.
    fn typed_offices(offices: &[OfficeSnapshot]) -> Vec<Value> {
        offices
            .iter()
            .map(|office| {
                json!({
                    "institution": office.institution,
                    "office": office.office,
                    "incumbent": office.incumbent,
                    "authority": office.authority.iter().map(Self::typed_grant).collect::<Vec<_>>(),
                })
            })
            .collect()
    }

    /// The forums this subject may petition. Standing is not carried: a subject
    /// learns that it may bring a grievance, not the boundary of everyone
    /// else's standing.
    fn typed_redress(&self) -> Vec<Value> {
        self.subject
            .redress
            .iter()
            .map(|forum| json!({"grievance": forum.grievance, "forum": forum.forum}))
            .collect()
    }

    /// Only the acting subject's own place, so the typed surface carries what a
    /// precondition reads and not a world gazetteer.
    fn typed_place(&self) -> Value {
        self.subject
            .position
            .and_then(|place| {
                self.snapshot
                    .places
                    .iter()
                    .find(|candidate| candidate.id == place)
            })
            .map_or(
                Value::Null,
                |place| json!({"id": place.id, "label": place.label}),
            )
    }

    /// The routes incident to that place, named by the kernel rather than
    /// recomputed here: the same set the scope digest reads, because both come
    /// from the one `scope_components` derivation.
    fn typed_routes(&self) -> Vec<Value> {
        self.subject
            .components
            .routes
            .keys()
            .filter_map(|edge_id| {
                self.snapshot
                    .routes
                    .iter()
                    .find(|route| route.id == *edge_id)
            })
            .map(|route| {
                json!({
                    "id": route.id,
                    "label": route.label,
                    "from": route.from,
                    "to": route.to,
                    "access": route.access,
                    "cost": route.cost,
                    "open": route.open,
                })
            })
            .collect()
    }

    /// Only the acting subject's own holdings. No other subject's, and no
    /// resource gazetteer: labels are resolved for what this subject holds and
    /// for nothing else.
    fn typed_holdings(&self) -> Vec<Value> {
        self.subject
            .components
            .holdings
            .iter()
            .map(|(resource, quantity)| {
                let label = self
                    .snapshot
                    .resources
                    .iter()
                    .find(|candidate| candidate.id == *resource)
                    .map(|candidate| candidate.label.clone());
                json!({"id": resource, "label": label, "quantity": quantity})
            })
            .collect()
    }

    /// The acting subject's dependencies, with no label and no target state. A
    /// target's label would name places, routes, or subjects this subject may
    /// not know, and a dependency on a closed route appears here unmarked: the
    /// crisis is the world's to derive, not this view's to editorialize.
    fn typed_dependencies(&self) -> Vec<Value> {
        self.subject
            .components
            .dependencies
            .iter()
            .map(|target| match target {
                DependencyTarget::Resource(id) => {
                    json!({"target_kind": "resource", "target_id": id})
                }
                DependencyTarget::Route(id) => json!({"target_kind": "route", "target_id": id}),
                DependencyTarget::Subject(id) => {
                    json!({"target_kind": "subject", "target_id": id})
                }
            })
            .collect()
    }

    fn visible_stimulus(&self) -> Result<String, ControllerError> {
        serde_json::to_string_pretty(&self.projector_knowledge())
            .map_err(|error| ControllerError::Serialization(error.to_string()))
    }

    /// The acting subject's own knowledge, and nothing else. A subject perceives
    /// a speech act if and only if it holds `Knowledge` of that act's fact, so
    /// this is a renderer over one kernel-derived field rather than a walk that
    /// decides reach a second time. Confidence reaches the Projector as prose
    /// uncertainty, which makes the prompt's instruction a description of the
    /// surface instead of its enforcement.
    fn projector_knowledge(&self) -> Vec<Value> {
        self.subject
            .knowledge
            .iter()
            .map(|entry| {
                json!({
                    "speaker": self.speaker_label(entry).map_or(Value::Null, Value::String),
                    "certainty": entry.confidence,
                    "text": entry.statement.as_str(),
                })
            })
            .collect()
    }

    /// Attribution comes from the listener's own knowledge, never from an event
    /// scope: a subject that holds a fact it was never told sees the statement
    /// with no speaker, because it knows the thing and not the telling. A label
    /// is resolved only for a subject that spoke to this one.
    fn speaker_label(&self, entry: &KnowledgeSnapshot) -> Option<String> {
        match entry.source {
            KnowledgeSource::Told { by, .. } => self
                .snapshot
                .subjects
                .iter()
                .find(|subject| subject.id == by)
                .map(|subject| subject.label.clone()),
            KnowledgeSource::Witnessed | KnowledgeSource::Evidenced => None,
        }
    }

    /// Everything said to *this* subject since the turn's bound revision. Read
    /// from `self.subject` and never from `self.snapshot.subjects`: a delta
    /// rendered from the snapshot's subject list would leak a neighbour's state
    /// and no type would stop it.
    fn overheard_since(&self, revision: u64) -> Vec<Overheard> {
        self.subject
            .knowledge
            .iter()
            .filter(|row| row.spoken_at.is_some_and(|at| at > revision))
            .map(|row| Overheard {
                speaker: self.speaker_label(row),
                statement: row.statement.clone(),
                confidence: row.confidence,
            })
            .collect()
    }

    fn typed_knowledge(&self) -> Vec<Value> {
        self.subject
            .knowledge
            .iter()
            .map(|entry| {
                json!({
                    "fact": entry.fact,
                    "statement": entry.statement.as_str(),
                    "standing": match entry.standing {
                        FactStandingView::Canonical => json!({"standing": "canonical"}),
                        FactStandingView::Claimed { by } => {
                            json!({"standing": "claimed", "by": by})
                        }
                    },
                    "confidence": entry.confidence,
                    "source": entry.source,
                    "spoken_at": entry.spoken_at,
                })
            })
            .collect()
    }

    /// Only the channels this subject controls, by id. Who else is in reach is a
    /// question about other subjects, so no reach set is carried.
    fn typed_channels(&self) -> Vec<Value> {
        self.subject
            .components
            .controls
            .keys()
            .map(|channel| json!({"id": channel}))
            .collect()
    }
}

const OVERTAKEN_DETAIL: &str =
    "the intent this prose carried was overtaken before it could take effect";

/// The interruption section, appended verbatim to the interpreter prompt the
/// first lowering used.
///
/// It takes values, not a snapshot, so a neighbour's state is not reachable from
/// here: the whole-snapshot leak is a compile error rather than a review note.
/// What a subject may be shown is exactly (1) that one of its *own* scope
/// components differs, by the name of the field's meaning and with no value, id,
/// count, or actor; (2) that the set of affordances its opportunity grants
/// differs, in the same terms; and (3) every statement told to it since its
/// turn's bound revision, with a speaker only where the row's source is a
/// telling. (1) and (2) are anonymous by construction — no component of a scope
/// names an actor — and (3) is attributable because `fan_out` already decided
/// the subject may hold that row.
fn interruption_section(
    before: &ScopeComponents,
    bound: &DecisionOpportunity,
    interruption: &Interruption,
    fresh: &DecisionOpportunity,
) -> String {
    let after = &interruption.components;
    let mut section = String::from(
        "\n\nInterrupted: the world moved after this turn's prose was written. The prose stands; what it can still mean may not.\n\nWhat changed in this person's own reach, with no author it could perceive:\n",
    );
    let moved = [
        (
            before.position != after.position,
            "- where this person stands changed\n",
        ),
        (
            before.routes != after.routes,
            "- a way out of here changed\n",
        ),
        (
            before.holdings != after.holdings,
            "- what this person holds changed\n",
        ),
        (
            before.dependencies != after.dependencies,
            "- what this person depends on changed\n",
        ),
        (
            before.authority != after.authority,
            "- what this person is authorized over changed\n",
        ),
        (
            before.delegated != after.delegated,
            "- an office this person holds changed\n",
        ),
        (
            before.knows != after.knows,
            "- what this person knows changed\n",
        ),
        (
            before.controls != after.controls,
            "- a channel this person controls changed\n",
        ),
        (
            before.commitments != after.commitments,
            "- what this person owes changed\n",
        ),
        (
            bound.affordance_ids != fresh.affordance_ids,
            "- what this person may attempt changed\n",
        ),
    ];
    // The block is never empty: the digest also binds each granted entry's
    // definition, so every named field can compare equal while the scope has
    // still moved.
    if moved.iter().any(|(differs, _)| *differs) {
        for (_, line) in moved.iter().filter(|(differs, _)| *differs) {
            section.push_str(line);
        }
    } else {
        section.push_str("- something in this person's reach changed that it cannot name\n");
    }
    if !interruption.overheard.is_empty() {
        section.push_str("\nWhat was said to this person since:\n");
        for row in &interruption.overheard {
            let confidence = confidence_name(row.confidence);
            let statement = row.statement.as_str();
            match &row.speaker {
                Some(speaker) => section
                    .push_str(&format!("- {speaker} said: \"{statement}\" ({confidence})\n")),
                None => section.push_str(&format!(
                    "- this person came to know, without being told: \"{statement}\" ({confidence})\n"
                )),
            }
        }
    }
    section
}

/// Confidence through its own serde name, so the prompt and the typed surface
/// spell it the same way.
fn confidence_name(confidence: Confidence) -> String {
    serde_json::to_value(confidence)
        .ok()
        .and_then(|value| value.as_str().map(str::to_owned))
        .unwrap_or_default()
}

/// The narrative lane speaks and does nothing else, so it finds its entry by
/// kind name among the entries the kernel granted this opportunity.
fn speak_invocation(
    granted: &[AffordanceSnapshot],
    text: String,
) -> Result<DecisionInvocation, ControllerError> {
    let entry = granted
        .iter()
        .find(|entry| entry.entry.kind.0 == SPEAK_KIND)
        .ok_or(ControllerError::SpeakUnavailable)?;
    let speech = Statement::new(text).ok_or_else(|| {
        ControllerError::Serialization("Persona proposal is not canonical utterance text".into())
    })?;
    Ok(DecisionInvocation {
        affordance: entry.id,
        bindings: Vec::new(),
        proposed: Vec::new(),
        speech: Some(speech),
    })
}

fn narrative_invocation(
    checkpoint: &NarrativeCheckpoint,
) -> Result<DecisionInvocation, ControllerError> {
    let NarrativeCheckpoint::ReadyToSubmit {
        turn,
        interpreter_prompt,
        granted,
        completed,
        ..
    } = checkpoint
    else {
        return Err(ControllerError::Serialization(
            "Persona work is not ready to submit".into(),
        ));
    };
    let span = derive_narrative_capture(turn, interpreter_prompt, completed)?
        .proposal
        .ok_or_else(|| {
            ControllerError::Serialization("Persona work has no exact proposal span".into())
        })?;
    let text = turn
        .source_prose()
        .get(span.start_byte..span.end_byte)
        .ok_or_else(|| ControllerError::Serialization("Persona proposal span is not exact".into()))?
        .to_owned();
    speak_invocation(granted, text)
}

fn operational_invocation(
    checkpoint: &OperationalCheckpoint,
) -> Result<DecisionInvocation, ControllerError> {
    let OperationalCheckpoint::ReadyToSubmit {
        agent_prompt,
        granted,
        completed,
        ..
    } = checkpoint
    else {
        return Err(ControllerError::Serialization(
            "operational work is not ready to submit".into(),
        ));
    };
    derive_operational_capture(agent_prompt, granted, completed)?
        .proposal
        .ok_or_else(|| {
            ControllerError::Serialization("operational work has no exact proposal".into())
        })
}

fn narrative_capture(
    report: &InterpretationReport<SpeakProposal>,
    inference_receipts: Vec<String>,
) -> NarrativeCapture {
    NarrativeCapture {
        proposal: report.proposals().first().map(|proposal| SourceRange {
            start_byte: proposal.source().start_byte(),
            end_byte: proposal.source().end_byte(),
        }),
        gaps: report
            .gaps()
            .iter()
            .map(|gap| TranslationGapSummary {
                kind: gap.kind(),
                source: SourceRange {
                    start_byte: gap.source().start_byte(),
                    end_byte: gap.source().end_byte(),
                },
                detail: gap.detail().to_owned(),
            })
            .collect(),
        finalization: report.finalization(),
        inference_receipts,
    }
}

fn completed_narrative(
    checkpoint: &NarrativeCheckpoint,
    submission: SubmissionDisposition,
) -> Result<NarrativeRun, ControllerError> {
    let (turn, interpreter_prompt, completed) = match checkpoint {
        NarrativeCheckpoint::ReadyToSubmit {
            turn,
            interpreter_prompt,
            completed,
            ..
        }
        | NarrativeCheckpoint::NoProposal {
            turn,
            interpreter_prompt,
            completed,
            ..
        } => (turn, interpreter_prompt, completed),
        _ => {
            return Err(ControllerError::Serialization(
                "completed Persona work is not terminal".into(),
            ));
        }
    };
    Ok(NarrativeRun::Completed(NarrativeDecision {
        turn: turn.clone(),
        capture: derive_narrative_capture(turn, interpreter_prompt, completed)?,
        submission,
    }))
}

fn narrative_pending(work: NarrativeCheckpoint, reason: ControllerPendingReason) -> NarrativeRun {
    NarrativeRun::Pending(NarrativePending { work, reason })
}

/// The terminal interruption. One rule for every un-re-lowerable interruption:
/// it ends the turn, it never becomes a world submission, and it is reported in
/// the gap vocabulary the model already owns rather than a new word.
fn overtaken(
    checkpoint: &NarrativeCheckpoint,
    fresh_scope_digest: Option<ScopeDigest>,
) -> Result<NarrativeRun, ControllerError> {
    let (turn, interpreter_prompt, completed, opportunity) = match checkpoint {
        NarrativeCheckpoint::ReadyToSubmit {
            turn,
            interpreter_prompt,
            completed,
            opportunity,
            ..
        }
        | NarrativeCheckpoint::NoProposal {
            turn,
            interpreter_prompt,
            completed,
            opportunity,
            ..
        } => (turn, interpreter_prompt, completed, opportunity),
        _ => {
            return Err(ControllerError::Serialization(
                "interrupted Persona work is not terminal".into(),
            ));
        }
    };
    // The gap points at the proposal span the interpretation captured, or at
    // the whole prose when it captured none.
    let source = derive_narrative_capture(turn, interpreter_prompt, completed)?
        .proposal
        .unwrap_or(SourceRange {
            start_byte: 0,
            end_byte: turn.source_prose().len(),
        });
    Ok(NarrativeRun::Interrupted(NarrativeInterruption {
        subject: opportunity.scope.subject_id,
        bound_scope_digest: opportunity.scope_digest.as_str().to_owned(),
        fresh_scope_digest,
        gap: TranslationGapSummary {
            kind: TranslationGapKind::Unresolved,
            source,
            detail: OVERTAKEN_DETAIL.into(),
        },
        turn: turn.clone(),
    }))
}

fn completed_operational(
    checkpoint: &OperationalCheckpoint,
    submission: SubmissionDisposition,
) -> Result<OperationalRun, ControllerError> {
    // A `NoProposal` checkpoint holds no granted catalog: it reached the end of
    // its turn without one, so the empty set is what its capture re-derives
    // against.
    let (agent_prompt, granted, completed) = match checkpoint {
        OperationalCheckpoint::ReadyToSubmit {
            agent_prompt,
            granted,
            completed,
            ..
        } => (agent_prompt, granted.as_slice(), completed),
        OperationalCheckpoint::NoProposal {
            agent_prompt,
            completed,
            ..
        } => (agent_prompt, [].as_slice(), completed),
        OperationalCheckpoint::AgentInFlight { .. } => {
            return Err(ControllerError::Serialization(
                "completed operational work is not terminal".into(),
            ));
        }
    };
    Ok(OperationalRun::Completed(OperationalDecision {
        capture: derive_operational_capture(agent_prompt, granted, completed)?,
        submission,
    }))
}

fn operational_pending(
    work: OperationalCheckpoint,
    reason: ControllerPendingReason,
) -> OperationalRun {
    OperationalRun::Pending(OperationalPending { work, reason })
}

fn encoded_id<T: Serialize>(id: &T) -> Result<String, ControllerError> {
    let value = serde_json::to_value(id)
        .map_err(|error| ControllerError::Serialization(error.to_string()))?;
    value
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| ControllerError::Serialization("opaque ID was not a string".into()))
}

pub(super) fn tool_decode_need(name: &str, arguments: &str, detail: &str) -> ControllerNeed {
    ControllerNeed {
        detail: format!(
            "tool `{name}` arguments were not usable ({detail}); raw_argument_digest={}",
            sha256(arguments)
        ),
    }
}

enum InterpreterLoopEvaluation {
    Continue { conversation: Vec<CodexInputItem> },
    Complete { capture: NarrativeCapture },
}

/// The interpreter lane's cross-round accumulator, held apart from the loop so
/// one function computes every tool result string and both the batch evaluator
/// and the tool-result oracle call it. `finished` rides along because the call
/// that sets it is the same call that names it; it is read by the evaluator's
/// terminality check and never by a result string.
pub(super) struct InterpreterFold {
    accumulator: InterpretationAccumulator<SpeakProposal>,
    captured_speech: bool,
    finished: bool,
}

impl InterpreterFold {
    pub(super) fn new(source: PersonaTurn) -> Self {
        Self {
            accumulator: InterpretationAccumulator::new(source),
            captured_speech: false,
            finished: false,
        }
    }
}

/// One interpreter tool call folded into `fold`, returning exactly the string
/// the model is owed for it.
pub(super) fn interpreter_tool_result(
    source: &PersonaTurn,
    fold: &mut InterpreterFold,
    name: &str,
    arguments: &str,
) -> String {
    match name {
        INTERPRETER_SPEAK_TOOL => match serde_json::from_str::<InterpreterSpeakCall>(arguments) {
            Ok(call) if !fold.captured_speech => {
                let derived_text = source
                    .source_prose()
                    .get(call.source_start_byte..call.source_end_byte)
                    .unwrap_or_default()
                    .to_owned();
                let feedback = fold.accumulator.capture_proposal(
                    SpeakProposal { text: derived_text },
                    call.source_start_byte,
                    call.source_end_byte,
                );
                fold.captured_speech = feedback == CaptureToolFeedback::Accepted;
                format!("{feedback:?}")
            }
            Ok(call) => {
                let feedback = fold.accumulator.record_gap(RecordGapToolCall {
                    kind: TranslationGapKind::Ambiguity,
                    source_start_byte: call.source_start_byte,
                    source_end_byte: call.source_end_byte,
                    detail: "More than one speech proposal was offered; this runner permits one decision invocation per opportunity.".into(),
                });
                format!("{feedback:?}")
            }
            Err(error) => format!(
                "{:?}",
                fold.accumulator
                    .record_tool_decode_failure(name, arguments, &error.to_string())
            ),
        },
        INTERPRETER_RECORD_GAP_TOOL => match serde_json::from_str::<RecordGapToolCall>(arguments) {
            Ok(call) => format!("{:?}", fold.accumulator.record_gap(call)),
            Err(error) => format!(
                "{:?}",
                fold.accumulator
                    .record_tool_decode_failure(name, arguments, &error.to_string())
            ),
        },
        FINISH_INTERPRETATION_TOOL => match serde_json::from_str::<EmptyToolCall>(arguments) {
            Ok(_) => {
                fold.finished = true;
                "interpretation finished".into()
            }
            Err(error) => format!(
                "{:?}",
                fold.accumulator
                    .record_tool_decode_failure(name, arguments, &error.to_string())
            ),
        },
        _ => format!(
            "{:?}",
            fold.accumulator.record_tool_decode_failure(
                name,
                arguments,
                "tool is not available for this exact opportunity",
            )
        ),
    }
}

/// Replays every completed round's calls into fresh state, then answers this
/// query's calls in order. Construction is the replay: after it, `answer` is
/// exactly the evaluator's next step.
pub(super) struct InterpreterOracle {
    source: PersonaTurn,
    fold: InterpreterFold,
    remaining: u32,
}

impl InterpreterOracle {
    pub(super) fn new(source: &PersonaTurn, completed: &[InferenceOutput]) -> Self {
        let mut fold = InterpreterFold::new(source.clone());
        for output in completed {
            for event in &output.events {
                if let InferenceEvent::ToolCall {
                    name, arguments, ..
                } = event
                {
                    let _ = interpreter_tool_result(source, &mut fold, name, arguments);
                }
            }
        }
        Self {
            source: source.clone(),
            fold,
            remaining: TOOL_STEP_BUDGET.saturating_sub(completed.len()) as u32,
        }
    }
}

impl ToolResultOracle for InterpreterOracle {
    fn remaining_rounds(&self) -> u32 {
        self.remaining
    }

    fn answer(&mut self, name: &str, arguments: &str) -> Result<String, ControllerError> {
        Ok(interpreter_tool_result(
            &self.source,
            &mut self.fold,
            name,
            arguments,
        ))
    }
}

fn evaluate_interpreter_loop(
    source: &PersonaTurn,
    prompt: &str,
    completed: &[InferenceOutput],
) -> Result<InterpreterLoopEvaluation, ControllerError> {
    let mut conversation = vec![CodexInputItem::UserText {
        text: prompt.to_owned(),
    }];
    let mut fold = InterpreterFold::new(source.clone());
    let mut receipts = Vec::new();

    for (round, output) in completed.iter().enumerate() {
        if output.receipt_digest.is_empty() || output.receipt_digest.trim() != output.receipt_digest
        {
            return Err(ControllerError::ProviderContract {
                purpose: InferencePurpose::Interpreter,
                detail: "provider output has no canonical receipt digest".into(),
            });
        }
        receipts.push(output.receipt_digest.clone());
        let mut called_tool = false;
        for event in &output.events {
            match event {
                InferenceEvent::Text(text) => {
                    if !text.is_empty() {
                        conversation.push(CodexInputItem::AssistantText { text: text.clone() });
                    }
                }
                InferenceEvent::ToolCall {
                    call_id,
                    name,
                    arguments,
                } => {
                    called_tool = true;
                    conversation.push(CodexInputItem::ToolCall {
                        call_id: call_id.clone(),
                        name: name.clone(),
                        arguments: arguments.clone(),
                    });
                    let result = interpreter_tool_result(source, &mut fold, name, arguments);
                    conversation.push(CodexInputItem::ToolResult {
                        call_id: call_id.clone(),
                        output: result,
                    });
                }
            }
        }

        let finalization = if fold.finished || !called_tool {
            Some(InterpretationFinalization::InterpreterFinished)
        } else if round + 1 == TOOL_STEP_BUDGET {
            Some(InterpretationFinalization::StepBudgetExhausted)
        } else {
            None
        };
        if let Some(finalization) = finalization {
            if round + 1 != completed.len() {
                return Err(ControllerError::Serialization(
                    "interpreter evidence continued after total finalization".into(),
                ));
            }
            let report = fold.accumulator.finalize(finalization);
            return Ok(InterpreterLoopEvaluation::Complete {
                capture: narrative_capture(&report, receipts),
            });
        }
    }

    if completed.len() >= TOOL_STEP_BUDGET {
        return Err(ControllerError::Serialization(
            "interpreter evidence exceeded its step budget".into(),
        ));
    }
    Ok(InterpreterLoopEvaluation::Continue { conversation })
}

/// One constituent's prompt block, owned so the borrow of the selection ends
/// before the prompt is built.
struct PartitionedView {
    handle: String,
    identity: String,
    typed_view: String,
    tool_signatures: String,
}

/// Each block is the verbatim output of the same per-subject `typed_view()` the
/// detail path renders, under its own handle. There is no cross-resolution, no
/// merge, no dedup, and no shared header: a fact held under two handles appears
/// twice, and that duplication is the invariant.
fn partitioned_views(
    selected: &[SelectedDecision],
) -> Result<Vec<PartitionedView>, ControllerError> {
    selected
        .iter()
        .enumerate()
        .map(|(handle, decision)| {
            Ok(PartitionedView {
                handle: format!("c{handle}"),
                identity: decision.subject.label.clone(),
                typed_view: decision.typed_view()?,
                tool_signatures: catalog_signatures(&handle_prefix(handle), &decision.granted),
            })
        })
        .collect()
}

fn handle_prefix(handle: usize) -> String {
    format!("c{handle}{HANDLE_SEPARATOR}")
}

/// One tool set per constituent, name-namespaced, so an attributed proposal is
/// decided by which tool was called rather than by an argument the model writes.
/// A shared tool with a `subject` argument is the forgeable shape: it would let
/// one constituent's turn propose for another.
fn cell_catalog_tools(constituents: &[ConstituentWork]) -> Vec<CodexToolDefinition> {
    constituents
        .iter()
        .enumerate()
        .flat_map(|(handle, entry)| catalog_tools(&handle_prefix(handle), &entry.granted))
        .collect()
}

/// `c<index>__<tool>`, with one spelling per handle: a padded, signed, or
/// non-numeric index is not a handle, and a handle outside the cell does not
/// exist.
fn split_handle(name: &str, constituents: usize) -> Option<(usize, &str)> {
    let (prefix, tool) = name.split_once(HANDLE_SEPARATOR)?;
    let digits = prefix.strip_prefix('c')?;
    if digits.is_empty() || (digits.len() > 1 && digits.starts_with('0')) {
        return None;
    }
    if !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    let handle: usize = digits.parse().ok()?;
    (handle < constituents).then_some((handle, tool))
}

/// The batched decode. Vector-valued by construction: the singleton evaluator
/// keeps its scalar proposal and is not touched.
struct GroupedCapture {
    /// Handle to proposal. A handle absent here proposed nothing and declines.
    proposals: BTreeMap<usize, DecisionInvocation>,
    needs: Vec<ControllerNeed>,
}

enum GroupedLoopEvaluation {
    Continue { conversation: Vec<CodexInputItem> },
    Complete { capture: GroupedCapture },
}

/// The grouped lane's cross-round accumulator. Every field outlives the round
/// that wrote it, so the fold has no per-round reset.
pub(super) struct GroupedFold {
    proposals: BTreeMap<usize, DecisionInvocation>,
    terminal: BTreeSet<usize>,
    needs: Vec<ControllerNeed>,
}

impl GroupedFold {
    pub(super) fn new() -> Self {
        Self {
            proposals: BTreeMap::new(),
            terminal: BTreeSet::new(),
            needs: Vec::new(),
        }
    }
}

/// One grouped tool call folded into `fold`. The `Err` arm is the lane's own
/// hard contract error, not a result string: there is no answer for a
/// `finish_without_proposal` that contradicts a terminal choice already made.
pub(super) fn grouped_tool_result(
    constituents: &[ConstituentWork],
    fold: &mut GroupedFold,
    name: &str,
    arguments: &str,
) -> Result<String, ControllerError> {
    Ok(match split_handle(name, constituents.len()) {
        // A model that writes `c99__carry` has not proposed anything. It has
        // produced a gap.
        None => {
            fold.needs.push(tool_decode_need(
                name,
                arguments,
                "tool names no handle in this cell",
            ));
            "unattributable tool recorded as a need".into()
        }
        Some((handle, tool)) => {
            let granted = &constituents[handle].granted;
            let entry = granted.iter().find(|entry| entry.entry.kind.0 == tool);
            match tool {
                _ if entry.is_some() => {
                    if fold.terminal.contains(&handle) {
                        fold.needs.push(ControllerNeed {
                            detail: format!(
                                "Handle c{handle} was offered more than one terminal choice for one opportunity."
                            ),
                        });
                        "one terminal choice is already captured for this handle".into()
                    } else {
                        match decode_catalog_call(
                            entry.expect("the entry matched above"),
                            arguments,
                        ) {
                            Ok(invocation) => {
                                fold.proposals.insert(handle, invocation);
                                fold.terminal.insert(handle);
                                "invocation captured".into()
                            }
                            Err(detail) => {
                                fold.needs.push(tool_decode_need(name, arguments, &detail));
                                "arguments recorded as a need".into()
                            }
                        }
                    }
                }
                RECORD_NEED_TOOL => match serde_json::from_str::<RecordNeedCall>(arguments) {
                    Ok(call) => {
                        fold.needs.push(ControllerNeed {
                            detail: format!("c{handle}: {}", call.detail),
                        });
                        "need recorded".into()
                    }
                    Err(error) => {
                        fold.needs
                            .push(tool_decode_need(name, arguments, &error.to_string()));
                        "arguments recorded as a need".into()
                    }
                },
                FINISH_WITHOUT_PROPOSAL_TOOL => {
                    match serde_json::from_str::<EmptyToolCall>(arguments) {
                        Ok(_) if fold.terminal.contains(&handle) => {
                            return Err(ControllerError::ProviderContract {
                                purpose: InferencePurpose::GroupedAgent,
                                detail:
                                    "finish_without_proposal contradicted an existing terminal choice"
                                        .into(),
                            });
                        }
                        Ok(_) => {
                            fold.terminal.insert(handle);
                            "decision finished without a proposal".into()
                        }
                        Err(error) => {
                            fold.needs
                                .push(tool_decode_need(name, arguments, &error.to_string()));
                            "arguments recorded as a need".into()
                        }
                    }
                }
                _ => {
                    fold.needs.push(tool_decode_need(
                        name,
                        arguments,
                        "tool is not available to this handle",
                    ));
                    "unavailable tool recorded as a need".into()
                }
            }
        }
    })
}

/// The grouped lane's replay-then-answer oracle. Same shape as the
/// interpreter's, over `CELL_TOOL_STEP_BUDGET`.
pub(super) struct GroupedOracle {
    constituents: Vec<ConstituentWork>,
    fold: GroupedFold,
    remaining: u32,
}

impl GroupedOracle {
    pub(super) fn new(constituents: &[ConstituentWork], completed: &[InferenceOutput]) -> Self {
        let mut fold = GroupedFold::new();
        for output in completed {
            for event in &output.events {
                if let InferenceEvent::ToolCall {
                    name, arguments, ..
                } = event
                {
                    let _ = grouped_tool_result(constituents, &mut fold, name, arguments);
                }
            }
        }
        Self {
            constituents: constituents.to_vec(),
            fold,
            remaining: CELL_TOOL_STEP_BUDGET.saturating_sub(completed.len()) as u32,
        }
    }
}

impl ToolResultOracle for GroupedOracle {
    fn remaining_rounds(&self) -> u32 {
        self.remaining
    }

    fn answer(&mut self, name: &str, arguments: &str) -> Result<String, ControllerError> {
        grouped_tool_result(&self.constituents, &mut self.fold, name, arguments)
    }
}

fn evaluate_grouped_loop(
    prompt: &str,
    constituents: &[ConstituentWork],
    completed: &[InferenceOutput],
) -> Result<GroupedLoopEvaluation, ControllerError> {
    let mut conversation = vec![CodexInputItem::UserText {
        text: prompt.to_owned(),
    }];
    let mut fold = GroupedFold::new();

    for (round, output) in completed.iter().enumerate() {
        if output.receipt_digest.is_empty() || output.receipt_digest.trim() != output.receipt_digest
        {
            return Err(ControllerError::ProviderContract {
                purpose: InferencePurpose::GroupedAgent,
                detail: "provider output has no canonical receipt digest".into(),
            });
        }
        let mut called_tool = false;
        for event in &output.events {
            match event {
                InferenceEvent::Text(text) => {
                    if !text.is_empty() {
                        conversation.push(CodexInputItem::AssistantText { text: text.clone() });
                    }
                }
                InferenceEvent::ToolCall {
                    call_id,
                    name,
                    arguments,
                } => {
                    called_tool = true;
                    conversation.push(CodexInputItem::ToolCall {
                        call_id: call_id.clone(),
                        name: name.clone(),
                        arguments: arguments.clone(),
                    });
                    let result = grouped_tool_result(constituents, &mut fold, name, arguments)?;
                    conversation.push(CodexInputItem::ToolResult {
                        call_id: call_id.clone(),
                        output: result,
                    });
                }
            }
        }

        let is_complete = fold.terminal.len() == constituents.len()
            || !called_tool
            || round + 1 == CELL_TOOL_STEP_BUDGET;
        if is_complete {
            if round + 1 != completed.len() {
                return Err(ControllerError::Serialization(
                    "grouped evidence continued after total finalization".into(),
                ));
            }
            if round + 1 == CELL_TOOL_STEP_BUDGET
                && fold.terminal.len() < constituents.len()
                && called_tool
            {
                fold.needs.push(ControllerNeed {
                    detail: "The grouped step budget ended before every handle finished.".into(),
                });
            }
            return Ok(GroupedLoopEvaluation::Complete {
                capture: GroupedCapture {
                    proposals: fold.proposals,
                    needs: fold.needs,
                },
            });
        }
    }

    if completed.len() >= CELL_TOOL_STEP_BUDGET {
        return Err(ControllerError::Serialization(
            "grouped evidence exceeded its step budget".into(),
        ));
    }
    Ok(GroupedLoopEvaluation::Continue { conversation })
}

fn derive_grouped_capture(
    prompt: &str,
    constituents: &[ConstituentWork],
    completed: &[InferenceOutput],
) -> Result<GroupedCapture, ControllerError> {
    match evaluate_grouped_loop(prompt, constituents, completed)? {
        GroupedLoopEvaluation::Complete { capture } => Ok(capture),
        GroupedLoopEvaluation::Continue { .. } => Err(ControllerError::Serialization(
            "terminal grouped checkpoint has unfinished evidence".into(),
        )),
    }
}

fn grouped_request(
    command_id: CommandId,
    round: usize,
    model: &str,
    constituents: &[ConstituentWork],
    input: Vec<CodexInputItem>,
) -> Result<InferenceRequest, ControllerError> {
    tool_request(
        command_id,
        round,
        InferencePurpose::GroupedAgent,
        model,
        "Use only the supplied tools. Every tool belongs to exactly one handle, and calling one is how a proposal is attributed. Returning no proposal for a handle is valid.",
        input,
        cell_catalog_tools(constituents),
        RequestShape {
            // Output tokens, not the connector frame, are the real bound on a
            // cell: one call per handle, plus its arguments.
            max_output_tokens: 8_000
                .min(600 + 200 * u32::try_from(constituents.len()).unwrap_or(u32::MAX)),
            // The grouped protocol asks for every call in one round, so the
            // provider must be allowed to emit them together.
            parallel_tool_calls: true,
        },
    )
}

fn grouped_pending(checkpoint: GroupedCheckpoint, reason: ControllerPendingReason) -> GroupedRun {
    GroupedRun {
        cell: checkpoint.cell(),
        resolution: Resolution::Coarse {
            constituents: checkpoint.constituents().len(),
        },
        submissions: Vec::new(),
        needs: Vec::new(),
        pending: Some(reason),
    }
}
enum OperationalLoopEvaluation {
    Continue { conversation: Vec<CodexInputItem> },
    Complete { capture: OperationalCapture },
}

/// The operational lane's accumulator. `proposal` and `needs` outlive the round
/// that wrote them; `terminal_choice` is the one local a result string depends
/// on that a round boundary clears, which is what `begin_round` is for.
pub(super) struct OperationalFold {
    proposal: Option<DecisionInvocation>,
    needs: Vec<ControllerNeed>,
    terminal_choice: Option<String>,
}

impl OperationalFold {
    pub(super) fn new() -> Self {
        Self {
            proposal: None,
            needs: Vec::new(),
            terminal_choice: None,
        }
    }

    pub(super) fn begin_round(&mut self) {
        self.terminal_choice = None;
    }
}

/// One operational tool call folded into `fold`. The `Err` arm is the lane's
/// own hard contract error; there is no result string for a
/// `finish_without_proposal` that contradicts a terminal choice already made.
pub(super) fn operational_tool_result(
    granted: &[AffordanceSnapshot],
    fold: &mut OperationalFold,
    name: &str,
    arguments: &str,
) -> Result<String, ControllerError> {
    let entry = granted.iter().find(|entry| entry.entry.kind.0 == *name);
    Ok(match name {
        _ if entry.is_some() => {
            match decode_catalog_call(entry.expect("the entry matched above"), arguments) {
                Ok(invocation) if fold.terminal_choice.is_none() => {
                    fold.proposal = Some(invocation);
                    fold.terminal_choice = Some(name.to_owned());
                    "invocation captured".into()
                }
                Ok(_) => {
                    fold.needs.push(ControllerNeed {
                        detail:
                            "The agent offered more than one terminal choice for one opportunity."
                                .into(),
                    });
                    "one terminal choice is already captured".into()
                }
                Err(detail) => {
                    fold.needs.push(tool_decode_need(name, arguments, &detail));
                    "arguments recorded as a need".into()
                }
            }
        }
        RECORD_NEED_TOOL => match serde_json::from_str::<RecordNeedCall>(arguments) {
            Ok(call) => {
                fold.needs.push(ControllerNeed {
                    detail: call.detail,
                });
                "need recorded".into()
            }
            Err(error) => {
                fold.needs
                    .push(tool_decode_need(name, arguments, &error.to_string()));
                "arguments recorded as a need".into()
            }
        },
        FINISH_WITHOUT_PROPOSAL_TOOL => match serde_json::from_str::<EmptyToolCall>(arguments) {
            Ok(_) => {
                if fold.terminal_choice.is_some() {
                    return Err(ControllerError::ProviderContract {
                        purpose: InferencePurpose::OperationalAgent,
                        detail: "finish_without_proposal contradicted an existing terminal choice"
                            .into(),
                    });
                }
                fold.terminal_choice = Some(FINISH_WITHOUT_PROPOSAL_TOOL.to_owned());
                "decision finished without a proposal".into()
            }
            Err(error) => {
                fold.needs
                    .push(tool_decode_need(name, arguments, &error.to_string()));
                "arguments recorded as a need".into()
            }
        },
        _ => {
            fold.needs.push(tool_decode_need(
                name,
                arguments,
                "tool is not available for this exact opportunity",
            ));
            "unavailable tool recorded as a need".into()
        }
    })
}

/// The operational lane's replay-then-answer oracle. `begin_round` runs at
/// every replayed round boundary and once more after the replay, so the live
/// query's first call sees `terminal_choice: None` exactly as round *n* does.
pub(super) struct OperationalOracle {
    granted: Vec<AffordanceSnapshot>,
    fold: OperationalFold,
    remaining: u32,
}

impl OperationalOracle {
    pub(super) fn new(granted: &[AffordanceSnapshot], completed: &[InferenceOutput]) -> Self {
        let mut fold = OperationalFold::new();
        for output in completed {
            fold.begin_round();
            for event in &output.events {
                if let InferenceEvent::ToolCall {
                    name, arguments, ..
                } = event
                {
                    let _ = operational_tool_result(granted, &mut fold, name, arguments);
                }
            }
        }
        fold.begin_round();
        Self {
            granted: granted.to_vec(),
            fold,
            remaining: TOOL_STEP_BUDGET.saturating_sub(completed.len()) as u32,
        }
    }
}

impl ToolResultOracle for OperationalOracle {
    fn remaining_rounds(&self) -> u32 {
        self.remaining
    }

    fn answer(&mut self, name: &str, arguments: &str) -> Result<String, ControllerError> {
        operational_tool_result(&self.granted, &mut self.fold, name, arguments)
    }
}

fn evaluate_operational_loop(
    prompt: &str,
    granted: &[AffordanceSnapshot],
    completed: &[InferenceOutput],
) -> Result<OperationalLoopEvaluation, ControllerError> {
    let mut conversation = vec![CodexInputItem::UserText {
        text: prompt.to_owned(),
    }];
    let mut fold = OperationalFold::new();
    let mut receipts = Vec::new();

    for (round, output) in completed.iter().enumerate() {
        if output.receipt_digest.is_empty() || output.receipt_digest.trim() != output.receipt_digest
        {
            return Err(ControllerError::ProviderContract {
                purpose: InferencePurpose::OperationalAgent,
                detail: "provider output has no canonical receipt digest".into(),
            });
        }
        receipts.push(output.receipt_digest.clone());
        let mut called_tool = false;
        fold.begin_round();
        for event in &output.events {
            match event {
                InferenceEvent::Text(text) => {
                    if !text.is_empty() {
                        conversation.push(CodexInputItem::AssistantText { text: text.clone() });
                    }
                }
                InferenceEvent::ToolCall {
                    call_id,
                    name,
                    arguments,
                } => {
                    called_tool = true;
                    conversation.push(CodexInputItem::ToolCall {
                        call_id: call_id.clone(),
                        name: name.clone(),
                        arguments: arguments.clone(),
                    });
                    let result = operational_tool_result(granted, &mut fold, name, arguments)?;
                    conversation.push(CodexInputItem::ToolResult {
                        call_id: call_id.clone(),
                        output: result,
                    });
                }
            }
        }

        let is_complete =
            fold.terminal_choice.is_some() || !called_tool || round + 1 == TOOL_STEP_BUDGET;
        if is_complete {
            if round + 1 != completed.len() {
                return Err(ControllerError::Serialization(
                    "operational evidence continued after total finalization".into(),
                ));
            }
            if round + 1 == TOOL_STEP_BUDGET && fold.terminal_choice.is_none() && called_tool {
                fold.needs.push(ControllerNeed {
                    detail: "The operational-agent step budget ended before explicit completion."
                        .into(),
                });
            }
            return Ok(OperationalLoopEvaluation::Complete {
                capture: OperationalCapture {
                    proposal: fold.proposal,
                    needs: fold.needs,
                    inference_receipts: receipts,
                },
            });
        }
    }

    if completed.len() >= TOOL_STEP_BUDGET {
        return Err(ControllerError::Serialization(
            "operational evidence exceeded its step budget".into(),
        ));
    }
    Ok(OperationalLoopEvaluation::Continue { conversation })
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct InterpreterSpeakCall {
    source_start_byte: usize,
    source_end_byte: usize,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct OperationalSpeakCall {
    text: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RecordNeedCall {
    detail: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct EmptyToolCall {}

fn projector_request(
    command_id: CommandId,
    model: &str,
    prompt: String,
) -> Result<InferenceRequest, ControllerError> {
    text_request(
        command_id,
        InferencePurpose::Projector,
        model,
        "Render the private view in the user message as one lived narrative stream. Return only that prose.",
        prompt,
        1_200,
    )
}

fn persona_request(
    command_id: CommandId,
    model: &str,
    prompt: String,
) -> Result<InferenceRequest, ControllerError> {
    // This boundary is deliberately not lowered through a generic stage
    // constructor. Its only model-visible input is the dedicated natural-prose
    // prompt plus the smallest nonempty instruction required by the provider
    // transport contract.
    let input = vec![CodexInputItem::UserText { text: prompt }];
    let mut provider = CodexProviderRequest::new(
        provider_request_id(
            command_id,
            InferencePurpose::Persona,
            0,
            PERSONA_PROVIDER_INSTRUCTIONS,
            &input,
        )?,
        conversation_id(command_id, InferencePurpose::Persona, 0)?,
        model,
        PERSONA_PROVIDER_INSTRUCTIONS,
    );
    provider.input = input;
    provider.reasoning_effort = Some("low".into());
    provider.tools = Vec::new();
    provider.tool_choice = CodexToolChoice::Auto;
    provider.parallel_tool_calls = false;
    provider.output_format_name = None;
    provider.output_schema_json = None;
    provider.max_output_tokens = Some(1_200);
    Ok(InferenceRequest {
        purpose: InferencePurpose::Persona,
        provider,
    })
}

fn text_request(
    command_id: CommandId,
    purpose: InferencePurpose,
    model: &str,
    instructions: &str,
    prompt: String,
    max_output_tokens: u32,
) -> Result<InferenceRequest, ControllerError> {
    let input = vec![CodexInputItem::UserText { text: prompt }];
    let request = provider_request_id(command_id, purpose, 0, instructions, &input)?;
    let conversation = conversation_id(command_id, purpose, 0)?;
    let mut provider = CodexProviderRequest::new(request, conversation, model, instructions);
    provider.input = input;
    provider.reasoning_effort = Some("low".into());
    provider.tools = Vec::new();
    provider.tool_choice = CodexToolChoice::Auto;
    provider.parallel_tool_calls = false;
    provider.output_format_name = None;
    provider.output_schema_json = None;
    provider.max_output_tokens = Some(max_output_tokens);
    Ok(InferenceRequest { purpose, provider })
}

fn interpreter_request(
    command_id: CommandId,
    round: usize,
    model: &str,
    input: Vec<CodexInputItem>,
) -> Result<InferenceRequest, ControllerError> {
    tool_request(
        command_id,
        round,
        InferencePurpose::Interpreter,
        model,
        "Translate the preserved Persona prose using only the supplied capture tools. Untranslatable meaning is recorded, never repaired or rejected.",
        input,
        interpreter_tools(),
        DECISION_REQUEST_SHAPE,
    )
}

fn operational_request(
    command_id: CommandId,
    round: usize,
    model: &str,
    granted: &[AffordanceSnapshot],
    input: Vec<CodexInputItem>,
) -> Result<InferenceRequest, ControllerError> {
    tool_request(
        command_id,
        round,
        InferencePurpose::OperationalAgent,
        model,
        "Use only the supplied tools for this permissioned decision. Returning no proposal is valid.",
        input,
        catalog_tools("", granted),
        DECISION_REQUEST_SHAPE,
    )
}

/// The two knobs a request's *shape* needs beyond its tools: how much the
/// provider may write, and whether it may write more than one call at once. A
/// patch turn emits many calls where a decision turn emits one, and a cell emits
/// one per handle, so these belong to the caller rather than to one constant
/// shared by every catalog.
pub(super) struct RequestShape {
    pub(super) max_output_tokens: u32,
    pub(super) parallel_tool_calls: bool,
}

/// One decision, one call: the shape both detail lanes have always had.
const DECISION_REQUEST_SHAPE: RequestShape = RequestShape {
    max_output_tokens: 1_200,
    parallel_tool_calls: false,
};

pub(super) fn tool_request(
    command_id: CommandId,
    round: usize,
    purpose: InferencePurpose,
    model: &str,
    instructions: &str,
    input: Vec<CodexInputItem>,
    tools: Vec<CodexToolDefinition>,
    shape: RequestShape,
) -> Result<InferenceRequest, ControllerError> {
    let mut provider = CodexProviderRequest::new(
        provider_request_id(command_id, purpose, round, instructions, &input)?,
        conversation_id(command_id, purpose, round)?,
        model,
        instructions,
    );
    provider.input = input;
    provider.reasoning_effort = Some("medium".into());
    provider.tools = tools;
    provider.tool_choice = CodexToolChoice::Auto;
    provider.parallel_tool_calls = shape.parallel_tool_calls;
    provider.output_format_name = None;
    provider.output_schema_json = None;
    provider.max_output_tokens = Some(shape.max_output_tokens);
    Ok(InferenceRequest { purpose, provider })
}

fn interpreter_tools() -> Vec<CodexToolDefinition> {
    vec![
        tool_schema::tool(
            INTERPRETER_SPEAK_TOOL,
            "Capture one exact spoken utterance by byte span in the preserved Persona prose; the harness derives the utterance verbatim.",
            tool_schema::object(vec![
                (
                    "source_start_byte".into(),
                    tool_schema::bounded_integer(0, u64::from(u32::MAX)),
                ),
                (
                    "source_end_byte".into(),
                    tool_schema::bounded_integer(1, u64::from(u32::MAX)),
                ),
            ]),
        ),
        tool_schema::tool(
            INTERPRETER_RECORD_GAP_TOOL,
            "Preserve meaningful source prose that has no safe current typed translation.",
            tool_schema::object(vec![
                (
                    "kind".into(),
                    tool_schema::name_enum(&[
                        "ambiguity",
                        "missing_reference",
                        "missing_affordance",
                        "missing_primitive",
                        "unresolved",
                    ]),
                ),
                (
                    "source_start_byte".into(),
                    tool_schema::bounded_integer(0, u64::from(u32::MAX)),
                ),
                (
                    "source_end_byte".into(),
                    tool_schema::bounded_integer(1, u64::from(u32::MAX)),
                ),
                (
                    "detail".into(),
                    tool_schema::canonical_string(
                        "what the prose says that no typed capture can carry",
                    ),
                ),
            ]),
        ),
        tool_schema::tool(
            FINISH_INTERPRETATION_TOOL,
            "Finish the total interpretation after all supported captures and gaps are recorded.",
            tool_schema::empty_schema(),
        ),
    ]
}

/// One generated tool per granted entry, plus the two turn-enders. There is no
/// hand-written claim about what a subject may do anywhere: the model-facing
/// catalog, its prose signature line, and the typed view's permission block are
/// three projections of `granted` and cannot drift.
///
/// A schema carries the affordance *name*, its role parameters, and its bounded
/// slots — never an affordance id, an opportunity, a revision, a world, a
/// controller, or a caller. `record_need` and `finish_without_proposal` survive
/// because they are not world operations: they end a turn without proposing.
/// `prefix` is empty on the detail path, so its schemas stay byte-identical,
/// and `c<handle>__` on the grouped path, where the tool name is what attributes
/// a proposal to one constituent. One owner, two spellings of the same catalog.
pub(super) fn catalog_tools(
    prefix: &str,
    granted: &[AffordanceSnapshot],
) -> Vec<CodexToolDefinition> {
    let mut tools: Vec<CodexToolDefinition> = granted
        .iter()
        .map(|entry| {
            let mut properties: Vec<(String, Value)> = Vec::new();
            for spec in &entry.entry.roles {
                properties.push((
                    spec.role.0.clone(),
                    tool_schema::canonical_string(role_description(spec.kind)),
                ));
            }
            for (index, slot) in entry.entry.effect_slots.iter().enumerate() {
                // The ceiling is in the schema, so the model is told the bound
                // rather than discovering it through a rejection.
                let (name, ceiling) = match slot.bounds {
                    Bounds::None => continue,
                    Bounds::Quantity(max) => (slot_property(index, "qty"), max.0),
                    Bounds::Cost(max) => (slot_property(index, "cost"), u64::from(max.0)),
                };
                properties.push((name, tool_schema::bounded_integer(1, ceiling)));
            }
            if entry.entry.carries_speech {
                properties.push((
                    "text".into(),
                    tool_schema::canonical_string("the utterance this invocation carries"),
                ));
            }
            tool_schema::tool(
                &format!("{prefix}{}", entry.entry.kind.0),
                &if prefix.is_empty() {
                    format!("Invoke the {} affordance.", entry.entry.kind.0)
                } else {
                    format!(
                        "Invoke the {} affordance for {}.",
                        entry.entry.kind.0,
                        prefix.trim_end_matches(HANDLE_SEPARATOR)
                    )
                },
                tool_schema::object(properties),
            )
        })
        .collect();
    tools.push(tool_schema::tool(
        &format!("{prefix}{RECORD_NEED_TOOL}"),
        "Record information or capability the agent would need but does not currently have.",
        tool_schema::object(vec![(
            "detail".into(),
            tool_schema::canonical_string("what the agent would need and does not have"),
        )]),
    ));
    tools.push(tool_schema::tool(
        &format!("{prefix}{FINISH_WITHOUT_PROPOSAL_TOOL}"),
        "Finish this opportunity without proposing an action.",
        tool_schema::empty_schema(),
    ));
    tools
}

/// The same iteration rendered as one prose line, so the prompt's tool list and
/// the schemas have one owner.
fn catalog_signatures(prefix: &str, granted: &[AffordanceSnapshot]) -> String {
    let mut signatures: Vec<String> = granted
        .iter()
        .map(|entry| {
            let mut parameters: Vec<String> = entry
                .entry
                .roles
                .iter()
                .map(|spec| spec.role.0.clone())
                .collect();
            for (index, slot) in entry.entry.effect_slots.iter().enumerate() {
                match slot.bounds {
                    Bounds::None => {}
                    Bounds::Quantity(_) => parameters.push(slot_property(index, "qty")),
                    Bounds::Cost(_) => parameters.push(slot_property(index, "cost")),
                }
            }
            if entry.entry.carries_speech {
                parameters.push("text".into());
            }
            format!("{prefix}{}({})", entry.entry.kind.0, parameters.join(", "))
        })
        .collect();
    signatures.push(format!("{prefix}record_need(detail)"));
    signatures.push(format!("{prefix}finish_without_proposal()"));
    signatures.join(", ")
}

/// The typed view's permission block: the granted entries by name, derived from
/// the same list the schemas come from.
fn catalog_permissions(granted: &[AffordanceSnapshot]) -> Value {
    Value::Object(
        granted
            .iter()
            .map(|entry| (entry.entry.kind.0.clone(), Value::Bool(true)))
            .collect(),
    )
}

fn slot_property(index: usize, dimension: &str) -> String {
    format!("slot_{index}_{dimension}")
}

fn role_description(kind: RefKind) -> &'static str {
    match kind {
        RefKind::Subject(_) => "a subject id",
        RefKind::Entity(EntityKind::Place) => "a place id",
        RefKind::Entity(EntityKind::Resource) => "a resource id",
        RefKind::Entity(EntityKind::Fact) => "a fact id",
        RefKind::Entity(EntityKind::Channel) => "a channel id",
        RefKind::Edge(_) => "a route id",
        RefKind::Affordance => "an affordance name",
    }
}

/// A catalog tool call lowered to an invocation. A property that will not parse
/// becomes a `ControllerNeed` through the existing accumulator, never a kernel
/// round trip.
fn decode_catalog_call(
    entry: &AffordanceSnapshot,
    arguments: &str,
) -> Result<DecisionInvocation, String> {
    let Ok(Value::Object(fields)) = serde_json::from_str::<Value>(arguments) else {
        return Err("arguments are not a JSON object".into());
    };
    let mut bindings = Vec::new();
    for spec in &entry.entry.roles {
        let raw = fields
            .get(&spec.role.0)
            .and_then(Value::as_str)
            .ok_or_else(|| format!("role `{}` is missing or not a string", spec.role.0))?;
        // The id newtypes are serde-transparent over `Uuid`, so the one
        // deserializer parses them and no caller reaches inside them.
        let id = Value::String(raw.to_owned());
        let unparsed = || format!("role `{}` is not a UUID", spec.role.0);
        let target = match spec.kind {
            RefKind::Subject(_) => {
                Target::Subject(serde_json::from_value::<SubjectId>(id).map_err(|_| unparsed())?)
            }
            RefKind::Entity(_) => {
                Target::Entity(serde_json::from_value::<EntityId>(id).map_err(|_| unparsed())?)
            }
            RefKind::Edge(_) => {
                Target::Edge(serde_json::from_value::<EdgeId>(id).map_err(|_| unparsed())?)
            }
            RefKind::Affordance => {
                return Err(format!(
                    "role `{}` names no bindable namespace",
                    spec.role.0
                ));
            }
        };
        bindings.push(RoleBinding {
            role: spec.role.clone(),
            target,
        });
    }
    let mut proposed = Vec::new();
    for (index, slot) in entry.entry.effect_slots.iter().enumerate() {
        let magnitude = match slot.bounds {
            Bounds::None => Magnitude::None,
            Bounds::Quantity(_) => {
                let name = slot_property(index, "qty");
                let value = fields
                    .get(&name)
                    .and_then(Value::as_u64)
                    .ok_or_else(|| format!("`{name}` is missing or not a positive integer"))?;
                Magnitude::Quantity(Quantity(value))
            }
            Bounds::Cost(_) => {
                let name = slot_property(index, "cost");
                let value = fields
                    .get(&name)
                    .and_then(Value::as_u64)
                    .and_then(|value| u32::try_from(value).ok())
                    .ok_or_else(|| format!("`{name}` is missing or out of range"))?;
                Magnitude::Cost(Cost(value))
            }
        };
        proposed.push(ProposedEffect {
            slot: index,
            magnitude,
        });
    }
    let speech = if entry.entry.carries_speech {
        let text = fields
            .get("text")
            .and_then(Value::as_str)
            .ok_or_else(|| "`text` is missing or not a string".to_owned())?;
        Some(Statement::new(text).ok_or_else(|| "`text` is not canonical".to_owned())?)
    } else {
        None
    };
    Ok(DecisionInvocation {
        affordance: entry.id,
        bindings,
        proposed,
        speech,
    })
}

/// The provider request identity is content-addressed: the same command,
/// purpose, round, and bytes name the same request, so a resumed round replays
/// its completed response, while a repair round that re-prompts under the
/// same command and round names a different request instead of colliding
/// with the connector's replay record for the first attempt.
pub(super) fn provider_request_id(
    command_id: CommandId,
    purpose: InferencePurpose,
    round: usize,
    instructions: &str,
    input: &[CodexInputItem],
) -> Result<String, ControllerError> {
    let content = request_content_digest(instructions, input)?;
    Ok(format!(
        "ghostlight-request-{}-{}-{round}-{content}",
        encoded_id(&command_id)?,
        purpose_name(purpose)
    ))
}

fn request_content_digest(
    instructions: &str,
    input: &[CodexInputItem],
) -> Result<String, ControllerError> {
    let bytes = serde_json::to_vec(&(instructions, input))
        .map_err(|error| ControllerError::Serialization(error.to_string()))?;
    let digest = Sha256::digest(bytes);
    Ok(digest[..12]
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}

fn conversation_id(
    command_id: CommandId,
    purpose: InferencePurpose,
    round: usize,
) -> Result<String, ControllerError> {
    Ok(format!(
        "ghostlight-conversation-{}-{}-{round}",
        encoded_id(&command_id)?,
        purpose_name(purpose)
    ))
}

fn purpose_name(purpose: InferencePurpose) -> &'static str {
    match purpose {
        InferencePurpose::Projector => "projector",
        InferencePurpose::Persona => "persona",
        InferencePurpose::Interpreter => "interpreter",
        InferencePurpose::OperationalAgent => "operational",
        InferencePurpose::GroupedAgent => "grouped",
        InferencePurpose::Elaboration => "elaboration",
    }
}

#[cfg(test)]
mod tests {

    /// The decision lanes' tools go to the same provider under the same
    /// strict rules as the patch catalog; the interpreter's fixed tools and a
    /// granted catalog entry are checked offline here for the same reason.
    #[test]
    fn every_decision_lane_tool_schema_is_strict() {
        for tool in interpreter_tools() {
            let schema: serde_json::Value = serde_json::from_str(&tool.parameters_json).unwrap();
            super::super::tool_schema::assert_strict(&schema, &tool.name);
        }
    }
    use super::*;
    use crate::world::elaboration::{EvidenceError, EvidenceQuery, EvidenceReceipt};
    use crate::world::patch::{RECORD_GAP_PATCH_TOOL, kernel_speak_entry, kernel_speak_grant};
    use crate::world::{
        CommitmentKind, CoverBudget, CreateJurisdictionIntent, CreateWorldIntent,
        EntityDeclaration, EntityId, EvidenceRef, JurisdictionKey, Ref, SeedOutcome, SeedPort,
        TickMinutes, WorldScaleIntentRef, derive_cover,
    };

    /// The granted catalog every controller fixture works against: the
    /// kernel-built Speak entry under a fixture id.
    fn speak_snapshot(id: AffordanceId) -> AffordanceSnapshot {
        AffordanceSnapshot {
            id,
            entry: kernel_speak_entry(),
        }
    }
    use crate::world::{
        AuthenticatedCaller, CallerId, CommandBody, CommandEnvelope, ControllerId, CreateWorld,
        DecisionScope, Declaration, DraftHandle, NewController, PrincipalId, ScopeDigest,
        SubjectDeclaration, SubjectKind, WorldId, WorldPatch, WorldPhase,
    };
    use std::{
        collections::BTreeSet,
        sync::atomic::{AtomicBool, Ordering},
    };

    #[test]
    fn controller_work_decode_rejects_an_alternate_messagepack_shape() {
        let command_id = CommandId::new();
        let opportunity = fixture_opportunity(ControllerMode::OperationalAgent);
        let work = operational_in_flight(
            command_id,
            &opportunity,
            "Hold the bridge.",
            "operational-model",
            vec![],
        );
        let payload = rmp_serde::to_vec(&work).unwrap();
        assert_eq!(
            rmp_serde::from_slice::<ControllerWork>(&payload).unwrap(),
            work
        );
        let row = CultCacheEnvelope {
            key: store_key(command_id).unwrap(),
            r#type: CONTROLLER_WORK_ROW.into(),
            payload,
            stored_at: Utc::now().to_rfc3339(),
            schema_id: Some(CONTROLLER_WORK_SCHEMA.into()),
        };
        assert!(decode_controller_work(&row).is_err());
    }

    /// The schema bump refuses a prior store rather than migrating it silently:
    /// `open` walks every row and demands both the row type and the schema id
    /// match the current constants, so a row left over from before the bump is
    /// an open-time error, not a quietly dropped or reinterpreted checkpoint.
    /// v8 is the version that predates the pass-10 `session_command_id`
    /// derivation change, so a v8 row is exactly the checkpoint this bump
    /// exists to refuse.
    #[test]
    fn a_controller_work_row_from_schema_v8_is_refused_at_open() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("controller-work.cc");
        let command_id = CommandId::new();
        let opportunity = fixture_opportunity(ControllerMode::OperationalAgent);
        let work = operational_in_flight(
            command_id,
            &opportunity,
            "Hold the bridge.",
            "operational-model",
            vec![],
        );
        {
            let mut store = OwnedRedbMessagePackBackingStore::new(&path).unwrap();
            let row = CultCacheEnvelope {
                key: store_key(command_id).unwrap(),
                r#type: "controller_work.v8".into(),
                payload: rmp_serde::to_vec_named(&work).unwrap(),
                stored_at: Utc::now().to_rfc3339(),
                schema_id: Some("ghostlight.controller_work.v8".into()),
            };
            store.push(&row).unwrap();
        }
        let Err(error) = CultCacheControllerWorkStore::open(&path) else {
            panic!("a v8 row was accepted by the v9 store");
        };
        assert!(matches!(error, ControllerWorkStoreError::Fault { .. }));
    }

    #[test]
    fn in_flight_progression_cannot_substitute_a_model() {
        let operational_command = CommandId::new();
        let operational_opportunity = fixture_opportunity(ControllerMode::OperationalAgent);
        let existing_operational = operational_in_flight(
            operational_command,
            &operational_opportunity,
            "Hold the bridge.",
            "operational-model-a",
            vec![],
        );
        let completed_operational = vec![InferenceOutput {
            events: vec![InferenceEvent::ToolCall {
                call_id: "need".into(),
                name: RECORD_NEED_TOOL.into(),
                arguments: json!({"detail":"The wind reading is missing."}).to_string(),
            }],
            receipt_digest: "sha256:operational-round-zero".into(),
        }];
        let substituted_operational = operational_in_flight(
            operational_command,
            &operational_opportunity,
            "Hold the bridge.",
            "operational-model-b",
            completed_operational,
        );
        assert!(existing_operational.integrity_is_valid());
        assert!(substituted_operational.integrity_is_valid());
        assert!(!valid_controller_work_progression(
            &existing_operational,
            &substituted_operational
        ));

        let narrative_command = CommandId::new();
        let narrative_opportunity = fixture_opportunity(ControllerMode::NarrativePersona);
        let turn = fixture_persona_turn(&narrative_opportunity, "I wait.");
        let existing_narrative = narrative_interpreter_in_flight(
            narrative_command,
            &narrative_opportunity,
            &turn,
            "Interpret the prose.",
            "interpreter-model-a",
            vec![],
        );
        let completed_narrative = vec![InferenceOutput {
            events: vec![InferenceEvent::ToolCall {
                call_id: "gap".into(),
                name: INTERPRETER_RECORD_GAP_TOOL.into(),
                arguments: json!({
                    "kind":"unresolved",
                    "source_start_byte":0,
                    "source_end_byte":1,
                    "detail":"The intended action is unclear."
                })
                .to_string(),
            }],
            receipt_digest: "sha256:narrative-round-zero".into(),
        }];
        let substituted_narrative = narrative_interpreter_in_flight(
            narrative_command,
            &narrative_opportunity,
            &turn,
            "Interpret the prose.",
            "interpreter-model-b",
            completed_narrative,
        );
        assert!(existing_narrative.integrity_is_valid());
        assert!(substituted_narrative.integrity_is_valid());
        assert!(!valid_controller_work_progression(
            &existing_narrative,
            &substituted_narrative
        ));
    }

    #[test]
    fn persona_provider_request_is_structurally_prose_only() {
        let command_id = CommandId::new();
        let request = persona_request(
            command_id,
            "persona-model",
            build_persona_prompt(&PersonaPrompt {
                identity: "An old bridge keeper with sore hands.",
                lived_stream: "Rain drums on the tollhouse roof.",
                domain_guidance: "Answer with dry patience.",
                word_budget: 80,
            }),
        )
        .unwrap();
        assert_eq!(request.purpose, InferencePurpose::Persona);
        assert!(request.provider.validate().is_ok());
        assert_eq!(request.provider.instructions, PERSONA_PROVIDER_INSTRUCTIONS);
        assert!(request.provider.tools.is_empty());
        assert_eq!(request.provider.tool_choice, CodexToolChoice::Auto);
        assert!(!request.provider.parallel_tool_calls);
        assert!(request.provider.output_format_name.is_none());
        assert!(request.provider.output_schema_json.is_none());
        assert!(request.provider.previous_response_id.is_none());
        assert_eq!(request.provider.input.len(), 1);
        let encoded = serde_json::to_string(&request.provider)
            .unwrap()
            .to_lowercase();
        for leak in [
            "world_id",
            "controller_id",
            "opportunity",
            "affordance_id",
            "execute the supplied",
            "output contract",
        ] {
            assert!(!encoded.contains(leak), "Persona request leaked `{leak}`");
        }
    }

    #[test]
    fn projector_view_contains_lived_labels_but_no_authority_metadata() {
        let actor_id = SubjectId::issue();
        let speaker_id = SubjectId::issue();
        let actor_controller = ControllerId::issue();
        let speaker_controller = ControllerId::issue();
        let speak_affordance = AffordanceId::issue();
        let opportunity = DecisionOpportunity {
            world_id: WorldId::issue(),
            revision: 41,
            scope_digest: ScopeDigest::fixture("sha256:projector-must-not-see-this-digest"),
            scope: DecisionScope {
                subject_id: actor_id,
            },
            controller_id: actor_controller,
            controller_mode: ControllerMode::NarrativePersona,
            affordance_ids: vec![speak_affordance],
        };
        // Mara was told. Iris also holds a fact of her own that Mara was never
        // told, and no surface of Mara's may carry it.
        let heard = EntityId::issue();
        let unheard = EntityId::issue();
        let actor = SubjectSnapshot {
            id: actor_id,
            label: "Mara at the rain gate".into(),
            kind: SubjectKind::Person,
            controller_id: Some(actor_controller),
            controller_mode: Some(ControllerMode::NarrativePersona),
            human_controller: None,
            affordances: BTreeSet::from([speak_affordance]),
            position: None,
            components: fixture_components(),
            offices_held: Vec::new(),
            offices_granted: Vec::new(),
            redress: Vec::new(),
            knowledge: vec![KnowledgeSnapshot {
                fact: heard,
                statement: Statement::new("The lower hinge is flooding.").unwrap(),
                standing: FactStandingView::Claimed { by: speaker_id },
                confidence: Confidence::Believed,
                source: KnowledgeSource::Told {
                    by: speaker_id,
                    via: None,
                },
                spoken_at: Some(40),
            }],
            commitments: Vec::new(),
            pressures: Vec::new(),
            qualified: false,
        };
        let speaker = SubjectSnapshot {
            id: speaker_id,
            label: "Iris in the tollhouse".into(),
            kind: SubjectKind::Person,
            controller_id: Some(speaker_controller),
            controller_mode: Some(ControllerMode::OperationalAgent),
            human_controller: None,
            affordances: BTreeSet::new(),
            position: None,
            components: fixture_components(),
            offices_held: Vec::new(),
            offices_granted: Vec::new(),
            redress: Vec::new(),
            knowledge: vec![KnowledgeSnapshot {
                fact: unheard,
                statement: Statement::new("The tollhouse ledger is short.").unwrap(),
                standing: FactStandingView::Canonical,
                confidence: Confidence::Certain,
                source: KnowledgeSource::Witnessed,
                spoken_at: None,
            }],
            commitments: Vec::new(),
            pressures: Vec::new(),
            qualified: false,
        };
        let snapshot = WorldSnapshot {
            world_id: opportunity.world_id,
            revision: opportunity.revision,
            phase: WorldPhase::Active,
            owner: PrincipalId::new("projector-fixture-owner"),
            title: "The Rain Gate".into(),
            draft_approvals: BTreeSet::new(),
            required_approvers: BTreeSet::new(),
            subjects: vec![actor.clone(), speaker],
            affordances: vec![speak_snapshot(speak_affordance)],
            places: Vec::new(),
            resources: Vec::new(),
            routes: Vec::new(),
            opportunities: vec![opportunity.clone()],
            state_digest: "sha256:projector-must-not-see-this-digest".into(),
            last_commit_digest: Some("sha256:projector-must-not-see-the-commit".into()),
            now: crate::world::FictionalMinutes::default(),
            boundaries: Vec::new(),
            scale_deficit: Vec::new(),
        };
        let selected = SelectedDecision {
            snapshot,
            subject: actor,
            opportunity,
            granted: vec![speak_snapshot(speak_affordance)],
        };

        let context = selected.projector_context().unwrap();
        let stimulus = selected.visible_stimulus().unwrap();
        let projector_surface = format!("{context}\n{stimulus}");
        let actor_id = encoded_id(&actor_id).unwrap();
        let speaker_id = encoded_id(&speaker_id).unwrap();
        for forbidden in [
            "revision",
            "state_digest",
            "speaker_subject_id",
            "projector-must-not-see",
            actor_id.as_str(),
            speaker_id.as_str(),
        ] {
            assert!(
                !projector_surface.contains(forbidden),
                "Projector surface leaked `{forbidden}`"
            );
        }
        assert!(projector_surface.contains("Mara at the rain gate"));
        // The speaker's label is resolved because Mara was told by her, and the
        // statement appears because Mara holds it. Iris's own unheard fact does
        // not: a subject's view carries its own knowledge and no one else's.
        assert!(projector_surface.contains("Iris in the tollhouse"));
        assert!(projector_surface.contains("The lower hinge is flooding."));
        assert!(!projector_surface.contains("The tollhouse ledger is short."));

        let operational_surface = selected.typed_view().unwrap();
        assert!(operational_surface.contains("state_digest"));
        assert!(operational_surface.contains("The lower hinge is flooding."));
        assert!(!operational_surface.contains("The tollhouse ledger is short."));
    }

    /// Verification 5, second half, over real committed state: a subject
    /// perceives a speech act if and only if it holds `Knowledge` of that act's
    /// fact. The speaker speaks in the hall; the listener perceives it, the
    /// bystander one containment level down does not, the placeless stranger
    /// does not, and no surface of any of the three carries the utterance bytes
    /// except the listener's.
    #[test]
    fn a_subject_does_not_perceive_speech_it_was_not_in_reach_of() {
        use crate::world::tests::{auth_principal, command, opportunity_for, owner, speech_world};
        use crate::world::{
            AuthenticatedCaller, DecisionInvocation, Role, RoleBinding, Statement, SubmitReceipt,
            Target, WorldKernel,
        };

        let directory = tempfile::tempdir().unwrap();
        let mut kernel = WorldKernel::create(
            directory.path().join("world.cc"),
            crate::world::tests::creation(CommandId::new(), "Leakage"),
            &auth_principal(owner()),
        )
        .expect("a created world")
        .0;
        let (speech, active) = speech_world(&mut kernel);
        let utterance = "The lower hinge is flooding tonight.";
        let opportunity = opportunity_for(&active, speech.speaker);
        let caller = CallerId::Controller(opportunity.controller_id);
        let receipt = kernel
            .submit(
                command(
                    &active,
                    CommandId::new(),
                    caller.clone(),
                    CommandBody::ExerciseDecision {
                        opportunity,
                        invocation: DecisionInvocation {
                            affordance: speech.whisper,
                            bindings: vec![RoleBinding {
                                role: Role("target".into()),
                                target: Target::Subject(speech.listener),
                            }],
                            proposed: Vec::new(),
                            speech: Some(Statement::new(utterance).unwrap()),
                        },
                    },
                ),
                &AuthenticatedCaller::fixture(caller),
            )
            .expect("the whisper commits");
        assert!(matches!(receipt, SubmitReceipt::Applied(_)));

        let snapshot = kernel.snapshot().unwrap();
        let surfaces = |subject_id| {
            let subject = snapshot
                .subjects
                .iter()
                .find(|subject| subject.id == subject_id)
                .expect("the subject")
                .clone();
            let opportunity = opportunity_for(&snapshot, subject_id);
            let granted: Vec<AffordanceSnapshot> = snapshot
                .affordances
                .iter()
                .filter(|entry| subject.affordances.contains(&entry.id))
                .cloned()
                .collect();
            let selected = SelectedDecision {
                snapshot: snapshot.clone(),
                subject,
                opportunity,
                granted,
            };
            format!(
                "{}\n{}\n{}",
                selected.projector_context().unwrap(),
                selected.visible_stimulus().unwrap(),
                selected.typed_view().unwrap(),
            )
        };

        assert!(surfaces(speech.listener).contains(utterance));
        for deaf in [speech.bystander, speech.stranger, speech.speaker] {
            assert!(
                !surfaces(deaf).contains(utterance),
                "a subject perceived speech it holds no knowledge of"
            );
        }
        // The listener sees who told it, and nothing about anyone else's
        // knowledge, holdings, or position.
        let heard = surfaces(speech.listener);
        assert!(heard.contains("The Hall Speaker"));
        assert!(!heard.contains("The Yard Bystander"));
        assert!(!heard.contains("The Placeless Stranger"));
    }
    /// Soul falsification: the typed view carries the acting subject's own place
    /// and the routes incident to it, and no more. Another subject's position
    /// and a route that touches neither endpoint stay out of the surface.
    #[test]
    fn soul_the_typed_view_exposes_only_the_actors_place_and_incident_routes() {
        use crate::world::{
            AccessKind, Cost, EdgeId, EntityId, PlaceSnapshot, Quantity, ResourceSnapshot,
            RouteSnapshot,
        };

        let actor_id = SubjectId::issue();
        let other_id = SubjectId::issue();
        let actor_controller = ControllerId::issue();
        let other_controller = ControllerId::issue();
        let speak_affordance = AffordanceId::issue();
        // Ordered so the snapshot vectors stay in ID order, as `snapshot` builds
        // them.
        let mut place_ids = [EntityId::issue(), EntityId::issue(), EntityId::issue()];
        place_ids.sort();
        let [yard, road, vault] = place_ids;
        // `EdgeId` has no test allocator; it is transparent over a UUID, so two
        // literals in byte order do the job without touching production source.
        let edge = |value: &str| {
            serde_json::from_value::<EdgeId>(Value::String(value.into())).expect("an edge ID")
        };
        let first_edge = edge("11111111-1111-4111-8111-111111111111");
        let second_edge = edge("22222222-2222-4222-8222-222222222222");

        let opportunity = DecisionOpportunity {
            world_id: WorldId::issue(),
            revision: 12,
            scope_digest: ScopeDigest::fixture("sha256:scope"),
            scope: DecisionScope {
                subject_id: actor_id,
            },
            controller_id: actor_controller,
            controller_mode: ControllerMode::OperationalAgent,
            affordance_ids: vec![speak_affordance],
        };
        let actor = SubjectSnapshot {
            id: actor_id,
            label: "The Walker".into(),
            kind: SubjectKind::Person,
            controller_id: Some(actor_controller),
            controller_mode: Some(ControllerMode::OperationalAgent),
            human_controller: None,
            affordances: BTreeSet::from([speak_affordance]),
            position: Some(yard),
            components: ScopeComponents {
                routes: BTreeMap::from([(first_edge, fixture_route(yard, road))]),
                ..fixture_components()
            },
            offices_held: Vec::new(),
            offices_granted: Vec::new(),
            redress: Vec::new(),
            knowledge: Vec::new(),
            commitments: Vec::new(),
            pressures: Vec::new(),
            qualified: false,
        };
        let other = SubjectSnapshot {
            id: other_id,
            label: "The Vault Keeper".into(),
            kind: SubjectKind::Person,
            controller_id: Some(other_controller),
            controller_mode: Some(ControllerMode::OperationalAgent),
            human_controller: None,
            affordances: BTreeSet::new(),
            position: Some(vault),
            components: ScopeComponents {
                routes: BTreeMap::from([(second_edge, fixture_route(road, vault))]),
                ..fixture_components()
            },
            offices_held: Vec::new(),
            offices_granted: Vec::new(),
            redress: Vec::new(),
            knowledge: Vec::new(),
            commitments: Vec::new(),
            pressures: Vec::new(),
            qualified: false,
        };
        let named_place = |id, label: &str| PlaceSnapshot {
            id,
            label: label.into(),
            container: None,
        };
        let named_route = |id, label: &str, from, to| RouteSnapshot {
            id,
            label: label.into(),
            from,
            to,
            access: AccessKind::Public,
            cost: Cost(6),
            open: true,
        };
        let snapshot = WorldSnapshot {
            world_id: opportunity.world_id,
            revision: opportunity.revision,
            phase: WorldPhase::Active,
            affordances: vec![speak_snapshot(opportunity.affordance_ids[0])],
            owner: PrincipalId::new("scope-fixture-owner"),
            title: "Kharad".into(),
            draft_approvals: BTreeSet::new(),
            required_approvers: BTreeSet::new(),
            subjects: vec![actor.clone(), other],
            places: vec![
                named_place(yard, "The Cavity Yard"),
                named_place(road, "The Rhythm Road"),
                named_place(vault, "The Sealed Vault"),
            ],
            resources: Vec::new(),
            routes: vec![
                named_route(first_edge, "The Yard Ramp", yard, road),
                named_route(second_edge, "The Vault Stair", road, vault),
            ],
            opportunities: vec![opportunity.clone()],
            state_digest: "sha256:state".into(),
            last_commit_digest: None,
            now: crate::world::FictionalMinutes::default(),
            boundaries: Vec::new(),
            scale_deficit: Vec::new(),
        };
        let selected = SelectedDecision {
            snapshot,
            subject: actor,
            opportunity,
            granted: vec![speak_snapshot(speak_affordance)],
        };

        let view = selected.typed_view().unwrap();
        assert!(view.contains("The Cavity Yard"));
        assert!(view.contains("The Yard Ramp"));
        for leaked in [
            "The Sealed Vault",
            "The Vault Stair",
            "The Vault Keeper",
            encoded_id(&vault).unwrap().as_str(),
        ] {
            assert!(!view.contains(leaked), "the typed view leaked `{leaked}`");
        }

        // A subject standing nowhere gets a null place and no routes at all.
        // `scope_components` derives both together, so the fixture clears both.
        let mut unplaced = selected;
        unplaced.subject.position = None;
        unplaced.subject.components.routes.clear();
        let view = unplaced.typed_view().unwrap();
        assert!(!view.contains("The Cavity Yard"));
        assert!(!view.contains("The Yard Ramp"));

        // Holdings are the acting subject's own, with labels resolved only for
        // what it holds; a dependency carries its target kind and ID and no
        // label at all, because a target's name may be something this subject
        // does not know. A dependency on a closed route appears unmarked.
        let mut custodial = unplaced;
        let tithe = EntityId::issue();
        let hoard = EntityId::issue();
        custodial.snapshot.resources = vec![
            ResourceSnapshot {
                id: tithe,
                label: "The Rhythm Tithe".into(),
            },
            ResourceSnapshot {
                id: hoard,
                label: "The Vault Hoard".into(),
            },
        ];
        custodial.subject.components.holdings = BTreeMap::from([(tithe, Quantity(7))]);
        custodial.subject.components.dependencies =
            BTreeSet::from([DependencyTarget::Route(second_edge)]);
        let view = custodial.typed_view().unwrap();
        assert!(view.contains("The Rhythm Tithe"));
        assert!(view.contains(r#""quantity": 7"#));
        assert!(view.contains(r#""target_kind": "route""#));
        assert!(view.contains(encoded_id(&second_edge).unwrap().as_str()));
        for leaked in ["The Vault Hoard", "The Vault Stair"] {
            assert!(!view.contains(leaked), "the typed view leaked `{leaked}`");
        }

        // The civic blocks are scoped the same way: the subject's own grants,
        // the offices it occupies and the offices it grants, and the forums it
        // may petition. No standing boundary, no label for anything inside a
        // target, and no other subject's jurisdiction.
        let mut civic = custodial;
        let hall = EntityId::issue();
        civic.subject.components.authority = BTreeSet::from([crate::world::AuthorityGrant {
            kind: crate::world::AuthorityKindName("levy".into()),
            over: crate::world::AuthorityTarget::PlaceSubtree(hall),
        }]);
        civic.subject.offices_held = vec![OfficeSnapshot {
            institution: other_id,
            office: crate::world::OfficeName("warden".into()),
            incumbent: Some(actor_id),
            authority: civic.subject.components.authority.clone(),
        }];
        civic.subject.redress = vec![crate::world::ForumSnapshot {
            grievance: crate::world::GrievanceKindName("seizure".into()),
            forum: other_id,
        }];
        let view = civic.typed_view().unwrap();
        assert!(view.contains(r#""kind": "levy""#));
        assert!(view.contains(r#""office": "warden""#));
        assert!(view.contains(r#""grievance": "seizure""#));
        assert!(view.contains(encoded_id(&hall).unwrap().as_str()));
        assert!(!view.contains("standing"));
        assert!(!view.contains("The Sealed Vault"));
    }

    #[test]
    fn controller_tool_schemas_cannot_claim_authority_or_envelopes() {
        let granted = vec![speak_snapshot(AffordanceId::issue())];
        for definition in interpreter_tools()
            .into_iter()
            .chain(catalog_tools("", &granted))
        {
            let schema: Value = serde_json::from_str(&definition.parameters_json).unwrap();
            assert_eq!(schema["additionalProperties"], false);
            let properties = schema["properties"].as_object().unwrap();
            for forbidden in [
                "caller",
                "caller_id",
                "controller_id",
                "world_id",
                "opportunity",
                "revision",
                "expected_revision",
                "affordance_id",
                "command_id",
            ] {
                assert!(!properties.contains_key(forbidden));
            }
        }
        assert_eq!(
            catalog_tools("", &granted)
                .iter()
                .map(|tool| tool.name.as_str())
                .collect::<Vec<_>>(),
            vec![SPEAK_KIND, RECORD_NEED_TOOL, FINISH_WITHOUT_PROPOSAL_TOOL]
        );
        let interpreter_speak = interpreter_tools()
            .into_iter()
            .find(|tool| tool.name == INTERPRETER_SPEAK_TOOL)
            .unwrap();
        let schema: Value = serde_json::from_str(&interpreter_speak.parameters_json).unwrap();
        assert!(
            !schema["properties"]
                .as_object()
                .unwrap()
                .contains_key("text")
        );
        assert!(
            serde_json::from_str::<InterpreterSpeakCall>(
                r#"{"source_start_byte":0,"source_end_byte":4,"text":"invented"}"#
            )
            .is_err(),
            "Interpreter speech cannot carry text independent of its source span"
        );
    }

    #[test]
    fn the_generated_tool_catalog_equals_the_granted_catalog() {
        use crate::world::{
            Affordance, AffordanceKindName, Bounds, ComponentOpKind, EffectSlot, OutcomeBand,
            Quantity, RefKind, Role, RoleSpec,
        };

        let carry = AffordanceSnapshot {
            id: AffordanceId::issue(),
            entry: Affordance {
                kind: AffordanceKindName("carry".into()),
                roles: vec![
                    RoleSpec {
                        role: Role("recipient".into()),
                        kind: RefKind::Subject(None),
                    },
                    RoleSpec {
                        role: Role("resource".into()),
                        kind: RefKind::Entity(crate::world::EntityKind::Resource),
                    },
                ],
                preconditions: Vec::new(),
                effect_slots: vec![EffectSlot {
                    op_kind: ComponentOpKind::Transfer,
                    roles: vec![
                        Role("recipient".into()),
                        Role("recipient".into()),
                        Role("resource".into()),
                    ],
                    bounds: Bounds::Quantity(Quantity(3)),
                }],
                outcome_bands: vec![OutcomeBand {
                    weight: 1,
                    effects: vec![0],
                }],
                carries_speech: false,
            },
        };
        let speak = speak_snapshot(AffordanceId::issue());
        let granted = vec![carry.clone(), speak.clone()];

        // The tool names are exactly the granted entries' kind names plus the
        // two turn-enders, in that order.
        assert_eq!(
            catalog_tools("", &granted)
                .iter()
                .map(|tool| tool.name.as_str())
                .collect::<Vec<_>>(),
            vec![
                "carry",
                SPEAK_KIND,
                RECORD_NEED_TOOL,
                FINISH_WITHOUT_PROPOSAL_TOOL
            ]
        );

        // Each tool requires exactly its roles, its bounded slots, and `text`
        // where the entry carries speech, and each bounded slot states its own
        // ceiling.
        let tools = catalog_tools("", &granted);
        let schema_of = |name: &str| -> Value {
            serde_json::from_str(
                &tools
                    .iter()
                    .find(|tool| tool.name == name)
                    .expect("a generated tool")
                    .parameters_json,
            )
            .unwrap()
        };
        let carry_schema = schema_of("carry");
        assert_eq!(
            carry_schema["required"],
            json!(["recipient", "resource", "slot_0_qty"])
        );
        assert_eq!(
            carry_schema["properties"]["slot_0_qty"]["maximum"],
            json!(3)
        );
        assert_eq!(
            carry_schema["properties"]["slot_0_qty"]["minimum"],
            json!(1)
        );
        assert_eq!(schema_of(SPEAK_KIND)["required"], json!(["text"]));

        // The three model-facing surfaces name the same entries in the same
        // order, so none of them can drift from the kernel's grant.
        assert_eq!(
            catalog_signatures("", &granted),
            "carry(recipient, resource, slot_0_qty), speak(text), record_need(detail), finish_without_proposal()"
        );
        assert_eq!(
            catalog_permissions(&granted),
            json!({"carry": true, "speak": true})
        );

        // A subject granted nothing gets no world tools, only the turn-enders.
        assert_eq!(
            catalog_tools("", &[])
                .iter()
                .map(|tool| tool.name.as_str())
                .collect::<Vec<_>>(),
            vec![RECORD_NEED_TOOL, FINISH_WITHOUT_PROPOSAL_TOOL]
        );

        // No generated schema carries an affordance id or an authority envelope.
        for tool in catalog_tools("", &granted) {
            let schema: Value = serde_json::from_str(&tool.parameters_json).unwrap();
            let properties = schema["properties"].as_object().unwrap();
            for forbidden in [
                "affordance",
                "affordance_id",
                "opportunity",
                "revision",
                "world_id",
                "controller_id",
                "caller",
                "command_id",
            ] {
                assert!(!properties.contains_key(forbidden));
            }
            assert!(
                !tool
                    .parameters_json
                    .contains(&encoded_id(&carry.id).unwrap())
            );
            assert!(
                !tool
                    .parameters_json
                    .contains(&encoded_id(&speak.id).unwrap())
            );
        }
    }

    #[test]
    fn provider_request_and_conversation_ids_are_deterministic_from_the_eve_command() {
        let command_id = CommandId::new();
        let first = persona_request(command_id, "persona-model", "prompt".into()).unwrap();
        let replay = persona_request(command_id, "persona-model", "prompt".into()).unwrap();
        assert_eq!(first, replay);
        assert_eq!(
            conversation_id(command_id, InferencePurpose::Interpreter, 0).unwrap(),
            conversation_id(command_id, InferencePurpose::Interpreter, 0).unwrap()
        );
        assert_ne!(
            conversation_id(command_id, InferencePurpose::Interpreter, 0).unwrap(),
            conversation_id(command_id, InferencePurpose::Interpreter, 1).unwrap()
        );
        let round_zero = interpreter_request(
            command_id,
            0,
            "interpreter-model",
            vec![CodexInputItem::UserText {
                text: "prompt".into(),
            }],
        )
        .unwrap();
        let round_one = interpreter_request(
            command_id,
            1,
            "interpreter-model",
            vec![CodexInputItem::UserText {
                text: "prompt".into(),
            }],
        )
        .unwrap();
        assert!(round_zero.provider.validate().is_ok());
        assert!(round_one.provider.validate().is_ok());
        assert_ne!(round_zero, round_one);
    }

    struct RecordingPort {
        outputs: Mutex<Vec<Result<InferenceOutput, InferenceFault>>>,
        persisted_before_interpreter: Arc<AtomicBool>,
    }

    #[async_trait]
    impl InferencePort for RecordingPort {
        fn prepare(&self, request: InferenceRequest) -> Result<PreparedInference, InferenceFault> {
            fixture_prepared(request)
        }

        async fn infer(
            &self,
            _request: PreparedInference,
        ) -> Result<InferenceOutput, InferenceFault> {
            assert!(self.persisted_before_interpreter.load(Ordering::SeqCst));
            self.outputs.lock().unwrap().remove(0)
        }
    }

    struct ExactReplayPort {
        expected: PreparedInference,
        output: Mutex<Option<Result<InferenceOutput, InferenceFault>>>,
        seen: AtomicBool,
    }

    #[async_trait]
    impl InferencePort for ExactReplayPort {
        fn prepare(&self, _request: InferenceRequest) -> Result<PreparedInference, InferenceFault> {
            Err(InferenceFault::new(
                "exact replay unexpectedly attempted to prepare a new invocation",
            ))
        }

        async fn infer(
            &self,
            request: PreparedInference,
        ) -> Result<InferenceOutput, InferenceFault> {
            assert_eq!(request, self.expected);
            assert!(!self.seen.swap(true, Ordering::SeqCst));
            self.output.lock().unwrap().take().unwrap()
        }
    }

    fn fixture_prepared(request: InferenceRequest) -> Result<PreparedInference, InferenceFault> {
        let native_request_sha256 = Sha256::digest(
            serde_json::to_vec(&request).map_err(|error| InferenceFault::new(error.to_string()))?,
        )
        .into();
        let purpose = request.purpose;
        let invocation = CodexTransportInvocation::new(
            "ghostlight-controller-test",
            4_102_444_800_000,
            native_request_sha256,
            request.provider,
        )
        .map_err(|error| InferenceFault::new(error.to_string()))?;
        Ok(PreparedInference {
            purpose,
            invocation,
        })
    }

    /// The digest-bound components a fixture subject carries. A literal rather
    /// than a constructor on `ScopeComponents`: the kernel derives these from
    /// state and owes no test builder.
    fn fixture_components() -> ScopeComponents {
        ScopeComponents {
            position: None,
            routes: BTreeMap::new(),
            holdings: BTreeMap::new(),
            dependencies: BTreeSet::new(),
            authority: BTreeSet::new(),
            delegated: BTreeMap::new(),
            knows: BTreeSet::new(),
            controls: BTreeMap::new(),
            commitments: BTreeMap::new(),
        }
    }

    fn fixture_route(from: EntityId, to: EntityId) -> crate::world::patch::EdgeRecord {
        crate::world::patch::EdgeRecord::Route {
            label: "a fixture route".into(),
            from,
            to,
            access: crate::world::AccessKind::Public,
            cost: crate::world::Cost(6),
            open: true,
        }
    }

    fn fixture_opportunity(mode: ControllerMode) -> DecisionOpportunity {
        DecisionOpportunity {
            world_id: WorldId::issue(),
            revision: 7,
            scope_digest: ScopeDigest::fixture("sha256:fixture-state"),
            scope: DecisionScope {
                subject_id: SubjectId::issue(),
            },
            controller_id: ControllerId::issue(),
            controller_mode: mode,
            affordance_ids: vec![AffordanceId::issue()],
        }
    }

    fn operational_in_flight(
        command_id: CommandId,
        opportunity: &DecisionOpportunity,
        agent_prompt: &str,
        model: &str,
        completed: Vec<InferenceOutput>,
    ) -> ControllerWork {
        let granted = vec![speak_snapshot(opportunity.affordance_ids[0])];
        let OperationalLoopEvaluation::Continue { conversation } =
            evaluate_operational_loop(agent_prompt, &granted, &completed).unwrap()
        else {
            panic!("operational fixture unexpectedly finalized")
        };
        let request =
            operational_request(command_id, completed.len(), model, &granted, conversation)
                .unwrap();
        ControllerWork::Operational(OperationalCheckpoint::AgentInFlight {
            command_id,
            agent_prompt: agent_prompt.into(),
            opportunity: opportunity.clone(),
            granted,
            completed,
            invocation: fixture_prepared(request).unwrap(),
        })
    }

    fn fixture_persona_turn(opportunity: &DecisionOpportunity, source_prose: &str) -> PersonaTurn {
        PersonaTurn::record(
            PersonaTurnBinding {
                world_id: encoded_id(&opportunity.world_id).unwrap(),
                controller_id: encoded_id(&opportunity.controller_id).unwrap(),
                opportunity_digest: opportunity.digest().unwrap(),
                world_revision: opportunity.revision,
                scope_digest: opportunity.scope_digest.as_str().to_owned(),
                projector_receipt_digest: "sha256:projector".into(),
                persona_inference_receipt_digest: "sha256:persona".into(),
                interrupted_from: None,
            },
            source_prose,
        )
    }

    /// A tool-call event as the SDK port would report one.
    fn oracle_call(call_id: &str, name: &str, arguments: &str) -> InferenceEvent {
        InferenceEvent::ToolCall {
            call_id: call_id.into(),
            name: name.into(),
            arguments: arguments.into(),
        }
    }

    fn oracle_round(calls: &[(&str, &str, &str)], receipt: &str) -> InferenceOutput {
        InferenceOutput {
            events: calls
                .iter()
                .map(|(id, name, arguments)| oracle_call(id, name, arguments))
                .collect(),
            receipt_digest: format!("sha256:{receipt}"),
        }
    }

    fn tool_results(conversation: &[CodexInputItem]) -> Vec<String> {
        conversation
            .iter()
            .filter_map(|item| match item {
                CodexInputItem::ToolResult { output, .. } => Some(output.clone()),
                _ => None,
            })
            .collect()
    }

    /// Spec test 1, interpreter lane. The string the oracle hands the model is
    /// the string the evaluator recomputes for the same call, in order.
    #[test]
    fn the_interpreter_oracle_answers_what_its_evaluator_recomputes() {
        let opportunity = fixture_opportunity(ControllerMode::NarrativePersona);
        let source = "I say, \"The rain has teeth tonight.\"";
        let turn = fixture_persona_turn(&opportunity, source);
        let speech = "The rain has teeth tonight.";
        let start = source.find(speech).unwrap();
        let span = json!({
            "source_start_byte": start,
            "source_end_byte": start + speech.len(),
        })
        .to_string();
        let calls: Vec<(&str, &str, &str)> = vec![
            ("call-0", INTERPRETER_SPEAK_TOOL, span.as_str()),
            // A second speak, so `captured_speech` is exercised.
            ("call-1", INTERPRETER_SPEAK_TOOL, span.as_str()),
            ("call-2", INTERPRETER_RECORD_GAP_TOOL, "not json at all"),
            ("call-3", "speek", "{}"),
        ];
        let mut oracle = InterpreterOracle::new(&turn, &[]);
        let answers: Vec<String> = calls
            .iter()
            .map(|(_, name, arguments)| oracle.answer(name, arguments).unwrap())
            .collect();
        let output = oracle_round(&calls, "interpreter-oracle");
        let InterpreterLoopEvaluation::Continue { conversation } =
            evaluate_interpreter_loop(&turn, "Translate the prose.", &[output]).unwrap()
        else {
            panic!("a non-terminal interpreter round finalized")
        };
        assert_eq!(answers, tool_results(&conversation));
    }

    /// Spec test 1, operational lane, plus spec test 2's replay: an oracle
    /// seeded from a completed round continues the same fold, and its
    /// `terminal_choice` resets at the round boundary while `needs` does not.
    #[test]
    fn the_operational_oracle_answers_what_its_evaluator_recomputes() {
        let opportunity = fixture_opportunity(ControllerMode::OperationalAgent);
        let granted = vec![speak_snapshot(opportunity.affordance_ids[0])];
        let speak = granted[0].entry.kind.0.clone();
        let first: Vec<(&str, &str, &str)> = vec![
            ("call-0", RECORD_NEED_TOOL, r#"{"detail":"no route"}"#),
            ("call-1", RECORD_NEED_TOOL, "not json at all"),
            // A granted tool whose arguments do not decode: a need, not a
            // terminal choice, so the round stays open.
            ("call-2", speak.as_str(), "not json at all"),
            ("call-3", "speek", "{}"),
        ];
        let mut oracle = OperationalOracle::new(&granted, &[]);
        let answers: Vec<String> = first
            .iter()
            .map(|(_, name, arguments)| oracle.answer(name, arguments).unwrap())
            .collect();
        let round_zero = oracle_round(&first, "operational-oracle");
        let OperationalLoopEvaluation::Continue { conversation } =
            evaluate_operational_loop("Hold the bridge.", &granted, &[round_zero.clone()]).unwrap()
        else {
            panic!("a non-terminal operational round finalized")
        };
        assert_eq!(answers, tool_results(&conversation));

        // Round one, from an oracle constructed over round zero.
        let second: Vec<(&str, &str, &str)> = vec![
            ("call-4", RECORD_NEED_TOOL, r#"{"detail":"still no route"}"#),
            ("call-5", "speek", "{}"),
        ];
        let mut resumed = OperationalOracle::new(&granted, &[round_zero.clone()]);
        let resumed_answers: Vec<String> = second
            .iter()
            .map(|(_, name, arguments)| resumed.answer(name, arguments).unwrap())
            .collect();
        let round_one = oracle_round(&second, "operational-oracle-one");
        let OperationalLoopEvaluation::Continue { conversation } =
            evaluate_operational_loop("Hold the bridge.", &granted, &[round_zero, round_one])
                .unwrap()
        else {
            panic!("a non-terminal operational round finalized")
        };
        assert_eq!(
            resumed_answers,
            tool_results(&conversation)[first.len()..].to_vec()
        );

        // The one string a terminal choice owns, and the one the lane has no
        // string for at all.
        let mut terminal = OperationalOracle::new(&granted, &[]);
        let speech = json!({ "text": "Hold the bridge." }).to_string();
        assert_eq!(
            terminal.answer(&speak, &speech).unwrap(),
            "invocation captured"
        );
        assert_eq!(
            terminal.answer(&speak, &speech).unwrap(),
            "one terminal choice is already captured"
        );
        assert!(matches!(
            terminal.answer(FINISH_WITHOUT_PROPOSAL_TOOL, "{}"),
            Err(ControllerError::ProviderContract { .. })
        ));
    }

    /// Spec test 1, grouped lane. Two handles, and a tool that names none.
    #[test]
    fn the_grouped_oracle_answers_what_its_evaluator_recomputes() {
        let opportunity = fixture_opportunity(ControllerMode::OperationalAgent);
        let constituents = vec![
            ConstituentWork {
                subject: opportunity.scope.subject_id,
                opportunity: opportunity.clone(),
                granted: vec![speak_snapshot(opportunity.affordance_ids[0])],
                command_id: CommandId::new(),
            },
            ConstituentWork {
                subject: opportunity.scope.subject_id,
                opportunity: opportunity.clone(),
                granted: vec![speak_snapshot(opportunity.affordance_ids[0])],
                command_id: CommandId::new(),
            },
        ];
        let speak = constituents[0].granted[0].entry.kind.0.clone();
        let handle_one = format!("c0__{speak}");
        let calls: Vec<(&str, &str, &str)> = vec![
            ("call-0", "c99__carry", "{}"),
            ("call-1", "c0__record_need", r#"{"detail":"no route"}"#),
            ("call-2", "c1__record_need", "not json at all"),
            ("call-3", handle_one.as_str(), "not json at all"),
            ("call-4", "c1__notgranted", "{}"),
        ];
        let mut oracle = GroupedOracle::new(&constituents, &[]);
        let answers: Vec<String> = calls
            .iter()
            .map(|(_, name, arguments)| oracle.answer(name, arguments).unwrap())
            .collect();
        let output = oracle_round(&calls, "grouped-oracle");
        let GroupedLoopEvaluation::Continue { conversation } =
            evaluate_grouped_loop("Decide together.", &constituents, &[output]).unwrap()
        else {
            panic!("a non-terminal grouped round finalized")
        };
        assert_eq!(answers, tool_results(&conversation));
    }

    /// Spec test 4, and the reachable half of spec test 3: two ports that
    /// prepare under the same identity produce the same `PreparedInference`,
    /// and it satisfies the checkpoint's own identity check.
    #[test]
    fn both_ports_prepare_the_same_invocation() {
        let opportunity = fixture_opportunity(ControllerMode::OperationalAgent);
        let granted = vec![speak_snapshot(opportunity.affordance_ids[0])];
        let command_id = CommandId::new();
        let build = || {
            operational_request(
                command_id,
                0,
                "claude-opus-5",
                &granted,
                vec![CodexInputItem::UserText {
                    text: "Hold the bridge.".into(),
                }],
            )
            .unwrap()
        };
        let expiry = 4_102_444_800_000;
        let connector_side = prepare_invocation("ghostlight-runtime", expiry, build()).unwrap();
        let sdk_side = prepare_invocation("ghostlight-runtime", expiry, build()).unwrap();
        assert_eq!(connector_side, sdk_side);
        assert!(prepared_matches_request(&sdk_side, &build(), command_id, 0));
    }

    fn narrative_interpreter_in_flight(
        command_id: CommandId,
        opportunity: &DecisionOpportunity,
        turn: &PersonaTurn,
        interpreter_prompt: &str,
        model: &str,
        completed: Vec<InferenceOutput>,
    ) -> ControllerWork {
        let InterpreterLoopEvaluation::Continue { conversation } =
            evaluate_interpreter_loop(turn, interpreter_prompt, &completed).unwrap()
        else {
            panic!("Interpreter fixture unexpectedly finalized")
        };
        let request =
            interpreter_request(command_id, completed.len(), model, conversation).unwrap();
        ControllerWork::Narrative(NarrativeCheckpoint::InterpreterInFlight {
            command_id,
            turn: turn.clone(),
            interpreter_prompt: interpreter_prompt.into(),
            components: fixture_components(),
            interruption: None,
            opportunity: opportunity.clone(),
            granted: vec![speak_snapshot(opportunity.affordance_ids[0])],
            completed,
            invocation: fixture_prepared(request).unwrap(),
        })
    }

    struct RecordingWorkStore {
        persisted: Arc<AtomicBool>,
        work: Mutex<BTreeMap<CommandId, ControllerWork>>,
    }

    #[async_trait]
    impl ControllerWorkStore for RecordingWorkStore {
        async fn lookup(
            &self,
            command_id: CommandId,
        ) -> Result<ControllerWorkLookup, ControllerWorkStoreError> {
            Ok(self
                .work
                .lock()
                .unwrap()
                .get(&command_id)
                .cloned()
                .map(ControllerWorkLookup::Confirmed)
                .unwrap_or(ControllerWorkLookup::Missing))
        }

        async fn persist(
            &self,
            work: &ControllerWork,
        ) -> Result<ControllerWorkWrite, ControllerWorkStoreError> {
            if !work.integrity_is_valid() {
                return Err(ControllerWorkStoreError::new("invalid controller work"));
            }
            let command_id = work.command_id();
            let mut stored = self.work.lock().unwrap();
            if let Some(existing) = stored.get(&command_id) {
                if existing == work {
                    return Ok(ControllerWorkWrite::AlreadyPresent);
                }
                if existing.lane() != work.lane() {
                    return Err(ControllerWorkStoreError::CommandModeConflict);
                }
                if !valid_controller_work_progression(existing, work) {
                    return Err(ControllerWorkStoreError::new(
                        "controller checkpoint progression conflict",
                    ));
                }
            } else if !work.is_initial() {
                return Err(ControllerWorkStoreError::new(
                    "controller work skipped its durable pre-inference boundary",
                ));
            }
            stored.insert(command_id, work.clone());
            self.persisted.store(true, Ordering::SeqCst);
            Ok(ControllerWorkWrite::Applied)
        }

        async fn custody_probe(&self) -> Result<ControllerWorkCustody, ControllerWorkStoreError> {
            let stored = self.work.lock().unwrap();
            let count = |lane: WorkLane| stored.values().filter(|work| work.lane() == lane).count();
            Ok(ControllerWorkCustody::Owned {
                narrative_commands: count(WorkLane::Narrative),
                operational_commands: count(WorkLane::Operational),
                elaboration_commands: count(WorkLane::Elaboration),
                seed_commands: count(WorkLane::Seed),
            })
        }
    }

    async fn active_mailbox(
        controller: NewController,
    ) -> (
        tempfile::TempDir,
        WorldMailbox,
        tokio::task::JoinHandle<()>,
        SubjectSnapshot,
    ) {
        let directory = tempfile::tempdir().unwrap();
        let (mailbox, task) = WorldMailbox::open(directory.path().join("world.cc")).unwrap();
        let owner = PrincipalId::new("owner");
        let authenticated = AuthenticatedCaller::fixture(CallerId::Principal(owner.clone()));
        let creation = mailbox
            .create_fixture(
                CreateWorld {
                    id: CommandId::new(),
                    owner: owner.clone(),
                    title: "Controller Fixture".into(),
                    patch: WorldPatch {
                        declarations: vec![
                            Declaration::Entity(EntityDeclaration {
                                handle: DraftHandle::new("commons"),
                                label: "The Commons".into(),
                                kind: EntityKind::Place,
                                container: None,
                            }),
                            Declaration::Subject(SubjectDeclaration {
                                handle: DraftHandle::new("subject"),
                                label: "Subject".into(),
                                kind: SubjectKind::Person,
                                controller,
                                affordances: kernel_speak_grant(),
                                // Speech needs a room to fill.
                                position: Some(Ref::Draft(DraftHandle::new("commons"))),
                            }),
                        ],
                        operations: Vec::new(),
                        evidence: Vec::new(),
                    },
                    scale_intent: WorldScaleIntentRef::default(),
                },
                &authenticated,
            )
            .await
            .unwrap();
        let mut snapshot = mailbox.snapshot().await.unwrap();
        for body in [CommandBody::ApproveDraft, CommandBody::ActivateWorld] {
            mailbox
                .submit_fixture(
                    CommandEnvelope {
                        id: CommandId::new(),
                        world_id: creation.world_id,
                        expected_revision: snapshot.revision,
                        caller: CallerId::Principal(owner.clone()),
                        body,
                    },
                    &authenticated,
                )
                .await
                .unwrap();
            snapshot = mailbox.snapshot().await.unwrap();
        }
        assert_eq!(snapshot.phase, WorldPhase::Active);
        let subject = snapshot.subjects[0].clone();
        (directory, mailbox, task, subject)
    }

    fn output(
        events: Vec<InferenceEvent>,
        receipt: &str,
    ) -> Result<InferenceOutput, InferenceFault> {
        Ok(InferenceOutput {
            events,
            receipt_digest: format!("sha256:{receipt}"),
        })
    }

    fn models() -> ControllerModels {
        ControllerModels {
            projector: "projector".into(),
            persona: "persona".into(),
            interpreter: "interpreter".into(),
            operational_agent: "operator".into(),
            elaborator: "elaborator".into(),
        }
    }

    #[tokio::test]
    async fn narrative_turn_is_persisted_before_total_interpretation_and_shared_submit() {
        let (_directory, mailbox, task, _subject) =
            active_mailbox(NewController::NarrativePersona).await;
        let persisted = Arc::new(AtomicBool::new(false));
        let source = "I say, \"The rain has teeth tonight.\"";
        let speech = "The rain has teeth tonight.";
        let start = source.find(speech).unwrap();
        let port = Arc::new(RecordingPort {
            outputs: Mutex::new(vec![
                output(
                    vec![InferenceEvent::Text("Cold rain needles the bridge.".into())],
                    "projector",
                ),
                output(vec![InferenceEvent::Text(source.into())], "persona"),
                output(
                    vec![
                        InferenceEvent::ToolCall {
                            call_id: "call_bad_span".into(),
                            name: INTERPRETER_SPEAK_TOOL.into(),
                            arguments: json!({
                                "source_start_byte":source.len() + 10,
                                "source_end_byte":source.len() + 20
                            })
                            .to_string(),
                        },
                        InferenceEvent::ToolCall {
                            call_id: "call_speak".into(),
                            name: INTERPRETER_SPEAK_TOOL.into(),
                            arguments: json!({
                                "source_start_byte":start,
                                "source_end_byte":start + speech.len()
                            })
                            .to_string(),
                        },
                        InferenceEvent::ToolCall {
                            call_id: "call_finish".into(),
                            name: FINISH_INTERPRETATION_TOOL.into(),
                            arguments: "{}".into(),
                        },
                    ],
                    "interpreter",
                ),
            ]),
            persisted_before_interpreter: persisted.clone(),
        });
        let store = Arc::new(RecordingWorkStore {
            persisted,
            work: Mutex::new(BTreeMap::new()),
        });
        let runner = ControllerRunner::open(mailbox.clone(), port, store, models())
            .expect("the fixture ports open");
        let opportunity = mailbox.snapshot().await.unwrap().opportunities[0].clone();
        let command_id = CommandId::new();
        let run = runner
            .run_narrative(command_id, &opportunity)
            .await
            .unwrap();
        let NarrativeRun::Completed(decision) = run else {
            panic!("turn remained pending")
        };
        assert_eq!(
            decision.capture.proposal,
            Some(SourceRange {
                start_byte: start,
                end_byte: start + speech.len(),
            })
        );
        assert_eq!(decision.capture.gaps.len(), 1);
        assert_eq!(decision.turn.source_prose(), source);
        assert!(matches!(
            decision.submission,
            SubmissionDisposition::Completed(_)
        ));
        // The same scope at a later revision is the same binding, so a caller
        // holding a fresher snapshot resumes this run instead of failing.
        let later_opportunity = mailbox.snapshot().await.unwrap().opportunities[0].clone();
        assert_ne!(later_opportunity.revision, opportunity.revision);
        let NarrativeRun::Completed(resumed) = runner
            .run_narrative(command_id, &later_opportunity)
            .await
            .unwrap()
        else {
            panic!("a later view of the same scope did not resume the run")
        };
        assert!(matches!(
            resumed.submission,
            SubmissionDisposition::PreviouslyConfirmed(_)
        ));
        let NarrativeRun::Completed(replayed) = runner
            .run_narrative(command_id, &opportunity)
            .await
            .unwrap()
        else {
            panic!("terminal narrative work was not lookup-first")
        };
        assert!(matches!(
            replayed.submission,
            SubmissionDisposition::PreviouslyConfirmed(_)
        ));
        assert_eq!(mailbox.operator_log().await.unwrap().len(), 1);
        drop(runner);
        drop(mailbox);
        task.await.unwrap();
    }

    #[tokio::test]
    async fn operational_agent_uses_the_same_world_submission_without_exposed_authority() {
        let (_directory, mailbox, task, _subject) =
            active_mailbox(NewController::OperationalAgent).await;
        let persisted = Arc::new(AtomicBool::new(false));
        let port = Arc::new(RecordingPort {
            outputs: Mutex::new(vec![output(
                vec![
                    InferenceEvent::ToolCall {
                        call_id: "need".into(),
                        name: RECORD_NEED_TOOL.into(),
                        arguments: json!({"detail":"No wind reading is available."}).to_string(),
                    },
                    InferenceEvent::ToolCall {
                        call_id: "speak".into(),
                        name: INTERPRETER_SPEAK_TOOL.into(),
                        arguments: json!({"text":"Close the western span."}).to_string(),
                    },
                ],
                "operational",
            )]),
            persisted_before_interpreter: persisted.clone(),
        });
        let store = Arc::new(RecordingWorkStore {
            persisted,
            work: Mutex::new(BTreeMap::new()),
        });
        let runner = ControllerRunner::open(mailbox.clone(), port, store, models())
            .expect("the fixture ports open");
        let opportunity = mailbox.snapshot().await.unwrap().opportunities[0].clone();
        let command_id = CommandId::new();
        let run = runner
            .run_operational(command_id, &opportunity)
            .await
            .unwrap();
        let OperationalRun::Completed(decision) = run else {
            panic!("operational work remained pending")
        };
        assert_eq!(decision.capture.needs.len(), 1);
        assert!(matches!(
            decision.submission,
            SubmissionDisposition::Completed(_)
        ));
        let OperationalRun::Completed(replayed) = runner
            .run_operational(command_id, &opportunity)
            .await
            .unwrap()
        else {
            panic!("terminal operational work was not lookup-first")
        };
        assert!(matches!(
            replayed.submission,
            SubmissionDisposition::PreviouslyConfirmed(_)
        ));
        assert_eq!(mailbox.operator_log().await.unwrap().len(), 1);
        drop(runner);
        drop(mailbox);
        task.await.unwrap();
    }

    #[tokio::test]
    async fn narrative_no_proposal_commits_an_exact_world_decline_and_replays() {
        let (_directory, mailbox, task, _subject) =
            active_mailbox(NewController::NarrativePersona).await;
        let persisted = Arc::new(AtomicBool::new(false));
        let source = "I listen to the rain and let the question pass.";
        let port = Arc::new(RecordingPort {
            outputs: Mutex::new(vec![
                output(
                    vec![InferenceEvent::Text(
                        "Rain ticks against the bridge rail.".into(),
                    )],
                    "decline-projector",
                ),
                output(vec![InferenceEvent::Text(source.into())], "decline-persona"),
                output(
                    vec![
                        InferenceEvent::ToolCall {
                            call_id: "gap".into(),
                            name: INTERPRETER_RECORD_GAP_TOOL.into(),
                            arguments: json!({
                                "kind":"unresolved",
                                "source_start_byte":0,
                                "source_end_byte":source.len(),
                                "detail":"The prose expresses no supported world action."
                            })
                            .to_string(),
                        },
                        InferenceEvent::ToolCall {
                            call_id: "finish".into(),
                            name: FINISH_INTERPRETATION_TOOL.into(),
                            arguments: "{}".into(),
                        },
                    ],
                    "decline-interpreter",
                ),
            ]),
            persisted_before_interpreter: persisted.clone(),
        });
        let store = Arc::new(RecordingWorkStore {
            persisted,
            work: Mutex::new(BTreeMap::new()),
        });
        let runner = ControllerRunner::open(mailbox.clone(), port, store, models())
            .expect("the fixture ports open");
        let opportunity = mailbox.snapshot().await.unwrap().opportunities[0].clone();
        let command_id = CommandId::new();
        let NarrativeRun::Completed(decision) = runner
            .run_narrative(command_id, &opportunity)
            .await
            .unwrap()
        else {
            panic!("narrative decline remained pending")
        };
        assert!(decision.capture.proposal.is_none());
        assert_eq!(decision.capture.gaps.len(), 1);
        let applied = match decision.submission {
            SubmissionDisposition::NoProposal(SubmitReceipt::Applied(receipt)) => receipt,
            other => panic!("narrative decline was not canonically applied: {other:?}"),
        };
        assert_eq!(applied.resulting_revision, opportunity.revision + 1);
        let after = mailbox.snapshot().await.unwrap();
        assert_eq!(after.revision, opportunity.revision + 1);
        assert!(mailbox.operator_log().await.unwrap().is_empty());
        let refreshed = after.opportunities[0].clone();
        assert_ne!(refreshed, opportunity);
        assert_eq!(refreshed.scope_digest, opportunity.scope_digest);
        // A digest the world does not derive selects nothing.
        let mut forged = opportunity.clone();
        forged.scope_digest = ScopeDigest::fixture("sha256:not-this-scope");
        assert!(matches!(
            runner.run_narrative(CommandId::new(), &forged).await,
            Err(ControllerError::NoOpportunity {
                expected: ControllerMode::NarrativePersona
            })
        ));
        let NarrativeRun::Completed(resumed) =
            runner.run_narrative(command_id, &refreshed).await.unwrap()
        else {
            panic!("a later view of the same scope did not resume the run")
        };
        assert!(matches!(
            resumed.submission,
            SubmissionDisposition::NoProposal(SubmitReceipt::AlreadyApplied(ref receipt))
                if receipt == &applied
        ));

        let NarrativeRun::Completed(replayed) = runner
            .run_narrative(command_id, &opportunity)
            .await
            .unwrap()
        else {
            panic!("persisted narrative decline did not replay")
        };
        assert!(matches!(
            replayed.submission,
            SubmissionDisposition::NoProposal(SubmitReceipt::AlreadyApplied(ref receipt))
                if receipt == &applied
        ));
        assert_eq!(mailbox.snapshot().await.unwrap(), after);
        drop(runner);
        drop(mailbox);
        task.await.unwrap();
    }

    #[tokio::test]
    async fn operational_no_proposal_commits_an_exact_world_decline_and_replays() {
        let (_directory, mailbox, task, _subject) =
            active_mailbox(NewController::OperationalAgent).await;
        let persisted = Arc::new(AtomicBool::new(false));
        let port = Arc::new(RecordingPort {
            outputs: Mutex::new(vec![output(
                vec![
                    InferenceEvent::ToolCall {
                        call_id: "need".into(),
                        name: RECORD_NEED_TOOL.into(),
                        arguments: json!({"detail":"No current intervention is warranted."})
                            .to_string(),
                    },
                    InferenceEvent::ToolCall {
                        call_id: "finish".into(),
                        name: FINISH_WITHOUT_PROPOSAL_TOOL.into(),
                        arguments: "{}".into(),
                    },
                ],
                "decline-operational",
            )]),
            persisted_before_interpreter: persisted.clone(),
        });
        let store = Arc::new(RecordingWorkStore {
            persisted,
            work: Mutex::new(BTreeMap::new()),
        });
        let runner = ControllerRunner::open(mailbox.clone(), port, store, models())
            .expect("the fixture ports open");
        let opportunity = mailbox.snapshot().await.unwrap().opportunities[0].clone();
        let command_id = CommandId::new();
        let OperationalRun::Completed(decision) = runner
            .run_operational(command_id, &opportunity)
            .await
            .unwrap()
        else {
            panic!("operational decline remained pending")
        };
        assert!(decision.capture.proposal.is_none());
        assert_eq!(decision.capture.needs.len(), 1);
        let applied = match decision.submission {
            SubmissionDisposition::NoProposal(SubmitReceipt::Applied(receipt)) => receipt,
            other => panic!("operational decline was not canonically applied: {other:?}"),
        };
        assert_eq!(applied.resulting_revision, opportunity.revision + 1);
        let after = mailbox.snapshot().await.unwrap();
        assert_eq!(after.revision, opportunity.revision + 1);
        assert!(mailbox.operator_log().await.unwrap().is_empty());

        let OperationalRun::Completed(replayed) = runner
            .run_operational(command_id, &opportunity)
            .await
            .unwrap()
        else {
            panic!("persisted operational decline did not replay")
        };
        assert!(matches!(
            replayed.submission,
            SubmissionDisposition::NoProposal(SubmitReceipt::AlreadyApplied(ref receipt))
                if receipt == &applied
        ));
        assert_eq!(mailbox.snapshot().await.unwrap(), after);
        drop(runner);
        drop(mailbox);
        task.await.unwrap();
    }

    #[tokio::test]
    async fn controller_work_reopens_and_replays_the_exact_connector_invocation() {
        let (directory, mailbox, task, _subject) =
            active_mailbox(NewController::NarrativePersona).await;
        let path = directory.path().join("controller-work.cc");
        let store = Arc::new(CultCacheControllerWorkStore::open(&path).unwrap());
        let source = "I say, \"The rain has teeth tonight.\"";
        let speech = "The rain has teeth tonight.";
        let start = source.find(speech).unwrap();
        let persisted = Arc::new(AtomicBool::new(true));
        let runner = ControllerRunner::open(
            mailbox.clone(),
            Arc::new(RecordingPort {
                outputs: Mutex::new(vec![
                    output(
                        vec![InferenceEvent::Text("Cold rain needles the bridge.".into())],
                        "reopen-projector",
                    ),
                    output(vec![InferenceEvent::Text(source.into())], "reopen-persona"),
                    Err(InferenceFault::retryable("connector transport interrupted")),
                ]),
                persisted_before_interpreter: persisted.clone(),
            }),
            store.clone(),
            models(),
        )
        .expect("the fixture ports open");
        let opportunity = mailbox.snapshot().await.unwrap().opportunities[0].clone();
        let command_id = CommandId::new();
        let NarrativeRun::Pending(first_pending) = runner
            .run_narrative(command_id, &opportunity)
            .await
            .unwrap()
        else {
            panic!("interrupted Interpreter unexpectedly completed")
        };
        assert_eq!(
            first_pending.reason(),
            ControllerPendingReason::InferenceRetryable
        );
        assert_eq!(first_pending.mode(), ControllerMode::NarrativePersona);
        assert_eq!(first_pending.persona_prose(), Some(source));
        let pending = first_pending.work;
        let NarrativeCheckpoint::InterpreterInFlight {
            invocation: exact_invocation,
            ..
        } = &pending
        else {
            panic!("interrupted Interpreter lost its persisted Persona turn")
        };
        let exact_invocation = exact_invocation.clone();
        assert!(pending.integrity_is_valid());

        drop(runner);
        drop(store);
        let reopened = Arc::new(CultCacheControllerWorkStore::open(&path).unwrap());
        let ControllerWorkLookup::Confirmed(ControllerWork::Narrative(reopened_pending)) =
            reopened.lookup(command_id).await.unwrap()
        else {
            panic!("exact command was not recovered")
        };
        assert_eq!(reopened_pending, pending);
        let NarrativeCheckpoint::InterpreterInFlight {
            invocation: reopened_invocation,
            ..
        } = &reopened_pending
        else {
            panic!("reopened command changed checkpoint")
        };
        assert_eq!(reopened_invocation, &exact_invocation);

        let replay_port = Arc::new(ExactReplayPort {
            expected: exact_invocation,
            output: Mutex::new(Some(output(
                vec![
                    InferenceEvent::ToolCall {
                        call_id: "bad-span".into(),
                        name: INTERPRETER_SPEAK_TOOL.into(),
                        arguments: json!({
                            "source_start_byte": source.len() + 1,
                            "source_end_byte": source.len() + 2
                        })
                        .to_string(),
                    },
                    InferenceEvent::ToolCall {
                        call_id: "exact-span".into(),
                        name: INTERPRETER_SPEAK_TOOL.into(),
                        arguments: json!({
                            "source_start_byte": start,
                            "source_end_byte": start + speech.len()
                        })
                        .to_string(),
                    },
                    InferenceEvent::ToolCall {
                        call_id: "finish".into(),
                        name: FINISH_INTERPRETATION_TOOL.into(),
                        arguments: "{}".into(),
                    },
                ],
                "replayed-interpreter",
            ))),
            seen: AtomicBool::new(false),
        });
        let recovery_runner = ControllerRunner::open(
            mailbox.clone(),
            replay_port.clone(),
            reopened.clone(),
            models(),
        )
        .expect("the fixture ports open");
        let NarrativeRun::Completed(decision) = recovery_runner
            .run_narrative(command_id, &opportunity)
            .await
            .unwrap()
        else {
            panic!("completed connector replay did not continue")
        };
        assert!(replay_port.seen.load(Ordering::SeqCst));
        assert_eq!(decision.turn.source_prose(), source);
        assert_eq!(
            decision.capture.proposal,
            Some(SourceRange {
                start_byte: start,
                end_byte: start + speech.len(),
            })
        );
        assert_eq!(decision.capture.gaps.len(), 1);
        assert!(matches!(
            decision.submission,
            SubmissionDisposition::Completed(_)
        ));
        let captured = decision.capture.clone();

        drop(recovery_runner);
        drop(reopened);
        let reopened_terminal = CultCacheControllerWorkStore::open(&path).unwrap();
        let ControllerWorkLookup::Confirmed(ControllerWork::Narrative(terminal)) =
            reopened_terminal.lookup(command_id).await.unwrap()
        else {
            panic!("terminal command was not recovered")
        };
        let NarrativeCheckpoint::ReadyToSubmit {
            turn,
            interpreter_prompt,
            completed,
            ..
        } = &terminal
        else {
            panic!("terminal command changed checkpoint")
        };
        assert_eq!(
            derive_narrative_capture(turn, interpreter_prompt, completed).unwrap(),
            captured
        );
        let log = mailbox.operator_log().await.unwrap();
        assert_eq!(log.len(), 1);
        assert_eq!(log[0].speech.as_ref().map(Statement::as_str), Some(speech));

        drop(reopened_terminal);
        drop(mailbox);
        task.await.unwrap();
    }

    #[tokio::test]
    #[ignore = "requires the Idunn-sealed production CodexConnector"]
    async fn real_codex_connector_cognition_modes_commit_speech() {
        let connector_endpoint: SocketAddr = std::env::var("GHOSTLIGHT_CONTROLLER_CONNECTOR")
            .expect("GHOSTLIGHT_CONTROLLER_CONNECTOR is required")
            .parse()
            .expect("GHOSTLIGHT_CONTROLLER_CONNECTOR must be a socket address");
        let connector_credential = std::env::var_os("GHOSTLIGHT_CONTROLLER_CREDENTIAL")
            .expect("GHOSTLIGHT_CONTROLLER_CREDENTIAL is required");
        let runtime_id = std::env::var("GHOSTLIGHT_ACCEPTANCE_RUNTIME_ID")
            .expect("GHOSTLIGHT_ACCEPTANCE_RUNTIME_ID is required");
        let models = ControllerModels {
            projector: std::env::var("GHOSTLIGHT_CONTROLLER_PROJECTOR_MODEL")
                .expect("GHOSTLIGHT_CONTROLLER_PROJECTOR_MODEL is required"),
            persona: std::env::var("GHOSTLIGHT_CONTROLLER_PERSONA_MODEL")
                .expect("GHOSTLIGHT_CONTROLLER_PERSONA_MODEL is required"),
            interpreter: std::env::var("GHOSTLIGHT_CONTROLLER_INTERPRETER_MODEL")
                .expect("GHOSTLIGHT_CONTROLLER_INTERPRETER_MODEL is required"),
            operational_agent: std::env::var("GHOSTLIGHT_CONTROLLER_OPERATIONAL_MODEL")
                .expect("GHOSTLIGHT_CONTROLLER_OPERATIONAL_MODEL is required"),
            elaborator: std::env::var("GHOSTLIGHT_CONTROLLER_ELABORATOR_MODEL")
                .expect("GHOSTLIGHT_CONTROLLER_ELABORATOR_MODEL is required"),
        };
        let persona_log = std::env::var_os("GHOSTLIGHT_ACCEPTANCE_PERSONA_PROSE_LOG")
            .expect("GHOSTLIGHT_ACCEPTANCE_PERSONA_PROSE_LOG is required");

        let directory = tempfile::tempdir().unwrap();
        let (mailbox, task) = WorldMailbox::open(directory.path().join("world.cc")).unwrap();
        let owner = PrincipalId::new("acceptance-owner");
        let human = PrincipalId::new("acceptance-roll-caller");
        let owner_caller = AuthenticatedCaller::fixture(CallerId::Principal(owner.clone()));
        let human_caller = AuthenticatedCaller::fixture(CallerId::Principal(human.clone()));
        let creation = mailbox
            .create_fixture(
                CreateWorld {
                    id: CommandId::new(),
                    owner: owner.clone(),
                    title: "Midnight Roll Call".into(),
                    patch: WorldPatch {
                        declarations: vec![
                            Declaration::Subject(SubjectDeclaration {
                                handle: DraftHandle::new("roll-caller"),
                                label: "Iris, the midnight roll caller".into(),
                                kind: SubjectKind::Person,
                                controller: NewController::Human {
                                    principal: human.clone(),
                                },
                                affordances: kernel_speak_grant(),
                                position: None,
                            }),
                            Declaration::Subject(SubjectDeclaration {
                                handle: DraftHandle::new("watch-officer"),
                                label: "Mara, a watch officer who answers direct roll calls aloud in one short sentence".into(),
                                kind: SubjectKind::Person,
                                controller: NewController::NarrativePersona,
                                affordances: kernel_speak_grant(),
                                position: None,
                            }),
                            Declaration::Subject(SubjectDeclaration {
                                handle: DraftHandle::new("signal-council"),
                                label: "The Signal Council, which answers direct roll calls aloud with one short operational sentence".into(),
                                kind: SubjectKind::Institution,
                                controller: NewController::OperationalAgent,
                                affordances: kernel_speak_grant(),
                                position: None,
                            }),
                        ],
                        operations: Vec::new(),
                        evidence: Vec::new(),
                    },
                    scale_intent: WorldScaleIntentRef::default(),
                },
                &owner_caller,
            )
            .await
            .unwrap();

        let mut snapshot = mailbox.snapshot().await.unwrap();
        mailbox
            .submit_fixture(
                CommandEnvelope {
                    id: CommandId::new(),
                    world_id: creation.world_id,
                    expected_revision: snapshot.revision,
                    caller: CallerId::Principal(owner.clone()),
                    body: CommandBody::ApproveDraft,
                },
                &owner_caller,
            )
            .await
            .unwrap();
        snapshot = mailbox.snapshot().await.unwrap();
        mailbox
            .submit_fixture(
                CommandEnvelope {
                    id: CommandId::new(),
                    world_id: creation.world_id,
                    expected_revision: snapshot.revision,
                    caller: CallerId::Principal(human.clone()),
                    body: CommandBody::ApproveDraft,
                },
                &human_caller,
            )
            .await
            .unwrap();
        snapshot = mailbox.snapshot().await.unwrap();
        mailbox
            .submit_fixture(
                CommandEnvelope {
                    id: CommandId::new(),
                    world_id: creation.world_id,
                    expected_revision: snapshot.revision,
                    caller: CallerId::Principal(owner.clone()),
                    body: CommandBody::ActivateWorld,
                },
                &owner_caller,
            )
            .await
            .unwrap();

        snapshot = mailbox.snapshot().await.unwrap();
        assert!(snapshot.phase == WorldPhase::Active);
        let human_subject = snapshot
            .subjects
            .iter()
            .find(|subject| subject.human_controller.as_ref() == Some(&human))
            .cloned()
            .unwrap();
        let human_opportunity = snapshot
            .opportunities
            .iter()
            .find(|opportunity| Some(opportunity.controller_id) == human_subject.controller_id)
            .cloned()
            .unwrap();
        let human_speak = *snapshot
            .affordances
            .iter()
            .find(|entry| {
                entry.entry.kind.0 == SPEAK_KIND && human_subject.affordances.contains(&entry.id)
            })
            .map(|entry| &entry.id)
            .unwrap();
        let seed = "Mara and the Signal Council, answer the midnight roll call aloud now. Each of you, say in one short sentence that you hear me.";
        let seed_command = CommandId::new();
        let seed_receipt = mailbox
            .submit_fixture(
                CommandEnvelope {
                    id: seed_command,
                    world_id: creation.world_id,
                    expected_revision: snapshot.revision,
                    caller: CallerId::Principal(human.clone()),
                    body: CommandBody::ExerciseDecision {
                        opportunity: human_opportunity,
                        invocation: DecisionInvocation {
                            affordance: human_speak,
                            bindings: Vec::new(),
                            proposed: Vec::new(),
                            speech: Some(Statement::new(seed).unwrap()),
                        },
                    },
                },
                &human_caller,
            )
            .await
            .unwrap();
        assert!(matches!(seed_receipt, SubmitReceipt::Applied(_)));

        let inference = open_inference(
            Some(ConnectorBinding {
                endpoint: connector_endpoint,
                key_path: connector_credential.into(),
                caller_runtime_id: runtime_id,
            }),
            None,
            &models,
        )
        .unwrap();
        let work = open_controller_work(directory.path().join("controller-work.cc")).unwrap();
        let runner = ControllerRunner::open(mailbox.clone(), inference, work, models).unwrap();

        snapshot = mailbox.snapshot().await.unwrap();
        let log = mailbox.operator_log().await.unwrap();
        assert!(log.len() == 1);
        let seeded_text = log[0].speech.as_ref().unwrap();
        assert!(seeded_text.as_str().as_bytes() == seed.as_bytes());
        let narrative_subject = snapshot
            .subjects
            .iter()
            .find(|subject| subject.controller_mode == Some(ControllerMode::NarrativePersona))
            .cloned()
            .unwrap();
        let narrative_opportunity = snapshot
            .opportunities
            .iter()
            .find(|opportunity| Some(opportunity.controller_id) == narrative_subject.controller_id)
            .cloned()
            .unwrap();
        let narrative_command = CommandId::new();
        let narrative_run = match runner
            .run_narrative(narrative_command, &narrative_opportunity)
            .await
        {
            Ok(run) => run,
            Err(_) => panic!("NarrativePersona acceptance returned an error"),
        };
        let NarrativeRun::Completed(narrative_decision) = narrative_run else {
            panic!("NarrativePersona acceptance did not complete")
        };
        let narrative_receipt = match narrative_decision.submission() {
            SubmissionDisposition::Completed(SubmitReceipt::Applied(receipt)) => receipt,
            _ => panic!("NarrativePersona acceptance did not apply one world command"),
        };
        assert!(narrative_receipt.command_id == narrative_command);
        assert!(narrative_decision.persona_turn().receipt_is_valid());
        assert!(
            !narrative_decision
                .persona_turn()
                .binding()
                .projector_receipt_digest
                .is_empty()
        );
        assert!(
            !narrative_decision
                .persona_turn()
                .binding()
                .persona_inference_receipt_digest
                .is_empty()
        );
        assert!(
            narrative_decision
                .persona_turn()
                .binding()
                .opportunity_digest
                == narrative_opportunity.digest().unwrap()
        );
        assert!(
            narrative_decision.persona_turn().binding().world_revision
                == narrative_opportunity.revision
        );
        assert!(
            narrative_decision.persona_turn().binding().scope_digest
                == narrative_opportunity.scope_digest.as_str()
        );
        let persona_prose = narrative_decision.persona_turn().source_prose().to_owned();
        assert!(!persona_prose.trim().is_empty());
        assert!(persona_prose.len() <= 65_536);
        assert!(!narrative_decision.capture().inference_receipts.is_empty());
        let narrative_span = narrative_decision.capture().proposal.unwrap();
        let narrative_speech = persona_prose
            .get(narrative_span.start_byte..narrative_span.end_byte)
            .unwrap()
            .to_owned();
        assert!(!narrative_speech.trim().is_empty());
        let ControllerWorkLookup::Confirmed(ControllerWork::Narrative(
            NarrativeCheckpoint::ReadyToSubmit {
                completed: persisted_interpreter_outputs,
                ..
            },
        )) = runner.work.lookup(narrative_command).await.unwrap()
        else {
            panic!("NarrativePersona acceptance evidence was not durable")
        };
        let interpreter_speak = persisted_interpreter_outputs
            .iter()
            .flat_map(|output| output.events.iter())
            .filter_map(|event| match event {
                InferenceEvent::ToolCall {
                    name, arguments, ..
                } if name == INTERPRETER_SPEAK_TOOL => {
                    serde_json::from_str::<InterpreterSpeakCall>(arguments).ok()
                }
                InferenceEvent::Text(_) | InferenceEvent::ToolCall { .. } => None,
            })
            .find(|call| {
                call.source_start_byte == narrative_span.start_byte
                    && call.source_end_byte == narrative_span.end_byte
            })
            .unwrap();
        assert!(interpreter_speak.source_start_byte == narrative_span.start_byte);
        assert!(interpreter_speak.source_end_byte == narrative_span.end_byte);

        snapshot = mailbox.snapshot().await.unwrap();
        let log = mailbox.operator_log().await.unwrap();
        assert!(log.len() == 2);
        assert!(snapshot.revision == narrative_opportunity.revision + 1);
        let narrative_event = log
            .iter()
            .find(|event| event.speaker == narrative_subject.id)
            .unwrap();
        let committed_narrative_speech = narrative_event.speech.as_ref().unwrap();
        assert!(committed_narrative_speech.as_str().as_bytes() == narrative_speech.as_bytes());

        let operational_subject = snapshot
            .subjects
            .iter()
            .find(|subject| subject.controller_mode == Some(ControllerMode::OperationalAgent))
            .cloned()
            .unwrap();
        let operational_opportunity = snapshot
            .opportunities
            .iter()
            .find(|opportunity| {
                Some(opportunity.controller_id) == operational_subject.controller_id
            })
            .cloned()
            .unwrap();
        let operational_command = CommandId::new();
        let operational_run = match runner
            .run_operational(operational_command, &operational_opportunity)
            .await
        {
            Ok(run) => run,
            Err(_) => panic!("OperationalAgent acceptance returned an error"),
        };
        let OperationalRun::Completed(operational_decision) = operational_run else {
            panic!("OperationalAgent acceptance did not complete")
        };
        let operational_receipt = match operational_decision.submission() {
            SubmissionDisposition::Completed(SubmitReceipt::Applied(receipt)) => receipt,
            _ => panic!("OperationalAgent acceptance did not apply one world command"),
        };
        assert!(operational_receipt.command_id == operational_command);
        assert!(!operational_decision.capture().inference_receipts.is_empty());
        let operational_speech = operational_decision
            .capture()
            .proposal
            .as_ref()
            .and_then(|invocation| invocation.speech.as_ref())
            .unwrap()
            .as_str()
            .to_owned();
        assert!(!operational_speech.trim().is_empty());
        let ControllerWorkLookup::Confirmed(ControllerWork::Operational(
            OperationalCheckpoint::ReadyToSubmit {
                completed: persisted_agent_outputs,
                ..
            },
        )) = runner.work.lookup(operational_command).await.unwrap()
        else {
            panic!("OperationalAgent acceptance evidence was not durable")
        };
        let operational_speak = persisted_agent_outputs
            .iter()
            .flat_map(|output| output.events.iter())
            .find_map(|event| match event {
                InferenceEvent::ToolCall {
                    name, arguments, ..
                } if name == SPEAK_KIND => {
                    serde_json::from_str::<OperationalSpeakCall>(arguments).ok()
                }
                InferenceEvent::Text(_) | InferenceEvent::ToolCall { .. } => None,
            })
            .unwrap();
        assert!(operational_speak.text.as_bytes() == operational_speech.as_bytes());

        snapshot = mailbox.snapshot().await.unwrap();
        let log = mailbox.operator_log().await.unwrap();
        assert!(log.len() == 3);
        assert!(snapshot.revision == operational_opportunity.revision + 1);
        let operational_event = log
            .iter()
            .find(|event| event.speaker == operational_subject.id)
            .unwrap();
        let committed_operational_speech = operational_event.speech.as_ref().unwrap();
        assert!(committed_operational_speech.as_str().as_bytes() == operational_speech.as_bytes());

        drop(runner);
        drop(mailbox);
        task.await.unwrap();

        let mut persona_file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(persona_log)
            .unwrap();
        std::io::Write::write_all(&mut persona_file, persona_prose.as_bytes()).unwrap();
        persona_file.sync_all().unwrap();
    }

    // ---- Soul: the elaboration runner, which nothing drove -------------

    /// A scripted provider for the authoring lane. It records the exact
    /// prepared invocations it was asked to run, so the repaired round's prompt
    /// can be read back out of the wire rather than out of the checkpoint.
    struct ElaborationScript {
        outputs: Mutex<Vec<Result<InferenceOutput, InferenceFault>>>,
        seen: Mutex<Vec<PreparedInference>>,
    }

    #[async_trait]
    impl InferencePort for ElaborationScript {
        fn prepare(&self, request: InferenceRequest) -> Result<PreparedInference, InferenceFault> {
            fixture_prepared(request)
        }

        async fn infer(
            &self,
            request: PreparedInference,
        ) -> Result<InferenceOutput, InferenceFault> {
            self.seen.lock().unwrap().push(request);
            let mut outputs = self.outputs.lock().unwrap();
            assert!(!outputs.is_empty(), "the script ran out of rounds");
            outputs.remove(0)
        }
    }

    fn tool_round(
        calls: Vec<(&str, Value)>,
        receipt: &str,
    ) -> Result<InferenceOutput, InferenceFault> {
        output(
            calls
                .into_iter()
                .enumerate()
                .map(|(index, (name, arguments))| InferenceEvent::ToolCall {
                    call_id: format!("call-{index}"),
                    name: name.to_owned(),
                    arguments: arguments.to_string(),
                })
                .collect(),
            receipt,
        )
    }

    /// An Active world with one dead end: a road nothing has grown into, one
    /// route away from an inhabited commons.
    async fn elaboration_mailbox() -> (
        tempfile::TempDir,
        WorldMailbox,
        tokio::task::JoinHandle<()>,
        EntityId,
        EntityId,
    ) {
        let directory = tempfile::tempdir().unwrap();
        let (mailbox, task) = WorldMailbox::open(directory.path().join("world.cc")).unwrap();
        let owner = PrincipalId::new("owner");
        let authenticated = AuthenticatedCaller::fixture(CallerId::Principal(owner.clone()));
        let creation = mailbox
            .create_fixture(
                CreateWorld {
                    id: CommandId::new(),
                    owner: owner.clone(),
                    title: "Elaboration Fixture".into(),
                    patch: WorldPatch {
                        declarations: vec![
                            Declaration::Entity(EntityDeclaration {
                                handle: DraftHandle::new("commons"),
                                label: "The Commons".into(),
                                kind: EntityKind::Place,
                                container: None,
                            }),
                            Declaration::Entity(EntityDeclaration {
                                handle: DraftHandle::new("road"),
                                label: "The Unwalked Road".into(),
                                kind: EntityKind::Place,
                                container: Some(Ref::Draft(DraftHandle::new("commons"))),
                            }),
                            Declaration::Route(crate::world::RouteDeclaration {
                                handle: DraftHandle::new("lane"),
                                label: "The Long Lane".into(),
                                from: Ref::Draft(DraftHandle::new("commons")),
                                to: Ref::Draft(DraftHandle::new("road")),
                                access: crate::world::AccessKind::Public,
                                cost: Cost(1),
                            }),
                            Declaration::Subject(SubjectDeclaration {
                                handle: DraftHandle::new("subject"),
                                label: "Subject".into(),
                                kind: SubjectKind::Person,
                                controller: NewController::NarrativePersona,
                                affordances: kernel_speak_grant(),
                                position: Some(Ref::Draft(DraftHandle::new("commons"))),
                            }),
                        ],
                        operations: Vec::new(),
                        evidence: Vec::new(),
                    },
                    scale_intent: WorldScaleIntentRef::default(),
                },
                &authenticated,
            )
            .await
            .unwrap();
        let mut snapshot = mailbox.snapshot().await.unwrap();
        for body in [CommandBody::ApproveDraft, CommandBody::ActivateWorld] {
            mailbox
                .submit_fixture(
                    CommandEnvelope {
                        id: CommandId::new(),
                        world_id: creation.world_id,
                        expected_revision: snapshot.revision,
                        caller: CallerId::Principal(owner.clone()),
                        body,
                    },
                    &authenticated,
                )
                .await
                .unwrap();
            snapshot = mailbox.snapshot().await.unwrap();
        }
        assert_eq!(snapshot.phase, WorldPhase::Active);
        let place = |label: &str| {
            snapshot
                .places
                .iter()
                .find(|entry| entry.label == label)
                .expect("the declared place")
                .id
        };
        let (commons, road) = (place("The Commons"), place("The Unwalked Road"));
        assert_eq!(snapshot.boundaries.len(), 1, "one dead end, and only one");
        (directory, mailbox, task, commons, road)
    }

    /// Spec test 20, which the pass wired and did not write. Round one submits a
    /// draft the kernel refuses; the complete mismatch set is persisted against
    /// the same command id; a **fresh** runner over the same store resumes,
    /// renders those mismatches into the round-two prompt, and the repaired
    /// draft commits under the same identity.
    #[tokio::test]
    async fn soul_an_elaboration_session_resumes_and_repairs_from_its_mismatch_set() {
        let (_directory, mailbox, task, commons, road) = elaboration_mailbox().await;
        let jurisdiction = JurisdictionKey::PlaceSubtree(commons);
        let road_id = serde_json::to_value(road).unwrap();
        let store = Arc::new(RecordingWorkStore {
            persisted: Arc::new(AtomicBool::new(false)),
            work: Mutex::new(BTreeMap::new()),
        });
        let script = Arc::new(ElaborationScript {
            outputs: Mutex::new(vec![
                // Round one: a shed whose container is a handle nothing
                // declares, then a submit.
                tool_round(
                    vec![
                        (
                            "declare_place",
                            json!({
                                "handle": "shed",
                                "label": "The Roadside Shed",
                                "container": {"ref": "draft", "value": "nowhere"},
                            }),
                        ),
                        ("submit", json!({})),
                    ],
                    "elaboration-round-zero",
                ),
                // Round two: the same shed, repaired onto the answered place.
                tool_round(
                    vec![
                        (
                            "declare_place",
                            json!({
                                "handle": "shed",
                                "label": "The Roadside Shed",
                                "container": {"ref": "existing", "value": road_id},
                            }),
                        ),
                        ("submit", json!({})),
                    ],
                    "elaboration-round-one",
                ),
            ]),
            seen: Mutex::new(Vec::new()),
        });

        let first = ElaborationRunner::new(
            ElaborationPort::new(mailbox.clone()),
            script.clone(),
            Arc::new(NullEvidenceSource),
            store.clone(),
            models().elaborator,
        );
        let outcome = first.step(jurisdiction).await.unwrap();
        assert_eq!(
            outcome,
            crate::world::elaboration::ElaborationOutcome::Rejected
        );
        drop(first);

        // The kernel's complete set is what the checkpoint carries, under the
        // same command id, and it is not empty.
        let (command_id, mismatches, resumed_prompt) = {
            let stored = store.work.lock().unwrap();
            assert_eq!(stored.len(), 1, "one session, one row");
            let ControllerWork::Elaboration(ElaborationCheckpoint::ElaboratorInFlight {
                command_id,
                last_mismatches,
                agent_prompt,
                completed,
                ..
            }) = stored.values().next().unwrap().clone()
            else {
                panic!("the rejection did not reopen the session for repair");
            };
            assert!(!last_mismatches.is_empty(), "the repair set is empty");
            assert!(completed.is_empty(), "a rejected round kept its evidence");
            (command_id, last_mismatches, agent_prompt)
        };
        assert!(
            resumed_prompt.contains("Your previous patch was refused"),
            "{resumed_prompt}"
        );
        for mismatch in &mismatches {
            let rendered = serde_json::to_string(mismatch).unwrap();
            assert!(
                resumed_prompt.contains(&rendered),
                "the round-two prompt dropped {rendered}"
            );
        }

        // A fresh runner over the same store, holding nothing from the first.
        let second = ElaborationRunner::new(
            ElaborationPort::new(mailbox.clone()),
            script.clone(),
            Arc::new(NullEvidenceSource),
            store.clone(),
            models().elaborator,
        );
        let outcome = second.step(jurisdiction).await.unwrap();
        assert_eq!(
            outcome,
            crate::world::elaboration::ElaborationOutcome::Committed
        );

        // One identity across the whole session, and the answered boundary is
        // no longer derived because the commit made the predicate stop holding.
        {
            let stored = store.work.lock().unwrap();
            assert_eq!(stored.len(), 1);
            assert_eq!(stored.values().next().unwrap().command_id(), command_id);
        }
        let snapshot = mailbox.snapshot().await.unwrap();
        assert!(snapshot.boundaries.is_empty(), "the boundary survived");
        assert!(
            snapshot
                .places
                .iter()
                .any(|place| place.label == "The Roadside Shed")
        );

        // The wire agrees with the checkpoint: the second invocation carried
        // the repaired prompt.
        let seen = script.seen.lock().unwrap();
        assert_eq!(seen.len(), 2, "two rounds, two invocations");
        let second_wire = serde_json::to_string(&seen[1]).unwrap();
        assert!(second_wire.contains("previous patch was refused"));

        drop(second);
        drop(mailbox);
        task.await.unwrap();
    }

    /// A jurisdiction with no boundary and no deficit is the terminating
    /// condition, and it spends no inference.
    #[tokio::test]
    async fn soul_a_clean_jurisdiction_ends_the_loop_without_an_inference_call() {
        let (_directory, mailbox, task, commons, _road) = elaboration_mailbox().await;
        let store = Arc::new(RecordingWorkStore {
            persisted: Arc::new(AtomicBool::new(false)),
            work: Mutex::new(BTreeMap::new()),
        });
        let script = Arc::new(ElaborationScript {
            outputs: Mutex::new(Vec::new()),
            seen: Mutex::new(Vec::new()),
        });
        let runner = ElaborationRunner::new(
            ElaborationPort::new(mailbox.clone()),
            script.clone(),
            Arc::new(NullEvidenceSource),
            store.clone(),
            models().elaborator,
        );
        // The road's own subtree holds the road's boundary; a leaf with nothing
        // under it and no deficit row is clean.
        let outcome = runner.step(JurisdictionKey::Uncovered).await.unwrap();
        assert_eq!(
            outcome,
            crate::world::elaboration::ElaborationOutcome::Clean
        );
        assert!(script.seen.lock().unwrap().is_empty());
        assert!(store.work.lock().unwrap().is_empty());
        let _ = commons;

        drop(runner);
        drop(mailbox);
        task.await.unwrap();
    }

    /// The store refuses the version immediately before it, not merely some
    /// older one: `v7` is the bump this pass made and `v6` is what a store

    /// An active world with several non-human subjects standing in one room, so
    /// a cover has something to group.
    async fn active_cell_mailbox(
        controllers: Vec<NewController>,
    ) -> (tempfile::TempDir, WorldMailbox, tokio::task::JoinHandle<()>) {
        let directory = tempfile::tempdir().unwrap();
        let (mailbox, task) = WorldMailbox::open(directory.path().join("world.cc")).unwrap();
        let owner = PrincipalId::new("owner");
        let authenticated = AuthenticatedCaller::fixture(CallerId::Principal(owner.clone()));
        let mut declarations = vec![Declaration::Entity(EntityDeclaration {
            handle: DraftHandle::new("commons"),
            label: "The Commons".into(),
            kind: EntityKind::Place,
            container: None,
        })];
        for (index, controller) in controllers.into_iter().enumerate() {
            declarations.push(Declaration::Subject(SubjectDeclaration {
                handle: DraftHandle::new(&format!("subject{index}")),
                label: format!("Subject {index}"),
                kind: SubjectKind::Person,
                controller,
                affordances: kernel_speak_grant(),
                position: Some(Ref::Draft(DraftHandle::new("commons"))),
            }));
        }
        let creation = mailbox
            .create_fixture(
                CreateWorld {
                    id: CommandId::new(),
                    owner: owner.clone(),
                    title: "Cover Fixture".into(),
                    patch: WorldPatch {
                        declarations,
                        operations: Vec::new(),
                        evidence: Vec::new(),
                    },
                    scale_intent: WorldScaleIntentRef::default(),
                },
                &authenticated,
            )
            .await
            .unwrap();
        let mut snapshot = mailbox.snapshot().await.unwrap();
        for body in [CommandBody::ApproveDraft, CommandBody::ActivateWorld] {
            mailbox
                .submit_fixture(
                    CommandEnvelope {
                        id: CommandId::new(),
                        world_id: creation.world_id,
                        expected_revision: snapshot.revision,
                        caller: CallerId::Principal(owner.clone()),
                        body,
                    },
                    &authenticated,
                )
                .await
                .unwrap();
            snapshot = mailbox.snapshot().await.unwrap();
        }
        assert_eq!(snapshot.phase, WorldPhase::Active);
        (directory, mailbox, task)
    }

    /// One cell holding every active subject, derived by the real scheduler
    /// rather than hand-built: a test that mints its own cell would prove the
    /// runner works on a partition the cover cannot produce.
    async fn one_group(mailbox: &WorldMailbox) -> Cell {
        let snapshot = mailbox.snapshot().await.unwrap();
        let graph = mailbox.agency_graph().await.unwrap();
        let cover = derive_cover(
            snapshot.world_id,
            snapshot.now,
            60,
            &snapshot.opportunities,
            &graph,
            CoverBudget {
                cells: 1,
                constituent_cap: 8,
                urgency_slots: 0,
            },
        );
        let [cell] = cover.cells.as_slice() else {
            panic!("a one-cell budget derived {} cells", cover.cells.len());
        };
        assert!(matches!(cell, Cell::Group { .. }), "the cell is not coarse");
        cell.clone()
    }

    fn grouped_runner(
        mailbox: &WorldMailbox,
        outputs: Vec<Result<InferenceOutput, InferenceFault>>,
    ) -> (ControllerRunner, Arc<RecordingWorkStore>) {
        let persisted = Arc::new(AtomicBool::new(true));
        let port = Arc::new(RecordingPort {
            outputs: Mutex::new(outputs),
            persisted_before_interpreter: persisted.clone(),
        });
        let store = Arc::new(RecordingWorkStore {
            persisted,
            work: Mutex::new(BTreeMap::new()),
        });
        (
            ControllerRunner::open(mailbox.clone(), port, store.clone(), models())
                .expect("the fixture ports open"),
            store,
        )
    }

    fn speak_call(call_id: &str, name: &str, text: &str) -> InferenceEvent {
        InferenceEvent::ToolCall {
            call_id: call_id.into(),
            name: name.into(),
            arguments: json!({ "text": text }).to_string(),
        }
    }

    /// Verification 20. One inference proposes for `c0`, names a handle outside
    /// the cell, and says nothing for the rest. The valid call commits under its
    /// own opportunity; the out-of-cell name produces a gap and reaches no
    /// mailbox; every silent handle declines, so "attended and stayed silent" is
    /// distinguishable from "was never attended".
    #[tokio::test]
    async fn soul_a_batched_turn_attributes_by_tool_identity_and_refuses_an_outside_handle() {
        let (_directory, mailbox, task) = active_cell_mailbox(vec![
            NewController::OperationalAgent,
            NewController::OperationalAgent,
            NewController::OperationalAgent,
        ])
        .await;
        let cell = one_group(&mailbox).await;
        assert_eq!(cell.members().len(), 3);

        let (runner, _store) = grouped_runner(
            &mailbox,
            vec![
                output(
                    vec![
                        speak_call("c0", "c0__speak", "Close the western span."),
                        speak_call("outside", "c7__speak", "I speak for someone else."),
                    ],
                    "grouped-one",
                ),
                output(
                    vec![InferenceEvent::Text("Nothing further.".into())],
                    "grouped-two",
                ),
            ],
        );
        let CellRun::Grouped(run) = runner.run_cell(&cell).await.unwrap() else {
            panic!("a coarse cell did not run the grouped lane")
        };
        assert!(run.pending.is_none());
        assert_eq!(run.resolution, Resolution::Coarse { constituents: 3 });
        assert_eq!(run.submissions.len(), 3, "every constituent finished");
        let first = cell.members()[0].subject;
        assert!(matches!(
            run.submissions
                .iter()
                .find(|entry| entry.subject == first)
                .map(|entry| &entry.submission),
            Some(SubmissionDisposition::Completed(_))
        ));
        for entry in run
            .submissions
            .iter()
            .filter(|entry| entry.subject != first)
        {
            assert!(
                matches!(entry.submission, SubmissionDisposition::NoProposal(_)),
                "a silent constituent did not decline"
            );
        }
        assert!(
            run.needs
                .iter()
                .any(|need| need.detail.contains("c7__speak")),
            "the out-of-cell call left no gap: {:?}",
            run.needs
        );
        // Exactly one act was committed, by exactly one subject.
        let log = mailbox.operator_log().await.unwrap();
        assert_eq!(log.len(), 1);
        assert_eq!(log[0].speaker, first);

        drop(runner);
        drop(mailbox);
        task.await.unwrap();
    }

    /// Verification 20b. No spelling of a handle outside the cell decodes, and
    /// the index-to-subject mapping is scheduler-owned rather than
    /// model-supplied: a shared tool with a `subject` argument would be the
    /// forgeable shape.
    #[test]
    fn soul_no_out_of_cell_handle_has_a_decodable_spelling() {
        for name in [
            "c9__speak",
            "speak",
            "c__speak",
            "cx__speak",
            "c01__speak",
            "c-1__speak",
            "c 0__speak",
            "__speak",
        ] {
            assert_eq!(
                split_handle(name, 3),
                None,
                "`{name}` decoded to a handle inside a three-constituent cell"
            );
        }
        assert_eq!(split_handle("c0__speak", 3), Some((0, "speak")));
        assert_eq!(split_handle("c2__record_need", 3), Some((2, "record_need")));
        // In-cell but not granted: a handle, and still no proposal.
        assert_eq!(split_handle("c0__notgranted", 3), Some((0, "notgranted")));
    }

    /// A handle that names a tool it was not granted produces a gap, never a
    /// proposal.
    #[test]
    fn an_ungranted_tool_under_a_live_handle_is_a_gap() {
        let opportunity = fixture_opportunity(ControllerMode::OperationalAgent);
        let constituents = vec![ConstituentWork {
            subject: opportunity.scope.subject_id,
            opportunity: opportunity.clone(),
            granted: vec![speak_snapshot(opportunity.affordance_ids[0])],
            command_id: CommandId::new(),
        }];
        let completed = vec![
            InferenceOutput {
                events: vec![speak_call("call", "c0__notgranted", "Anything.")],
                receipt_digest: "sha256:grouped".into(),
            },
            // The repair round the grouped budget buys. It calls nothing, so the
            // handle finishes silent.
            InferenceOutput {
                events: vec![InferenceEvent::Text("Nothing further.".into())],
                receipt_digest: "sha256:grouped-repair".into(),
            },
        ];
        let GroupedLoopEvaluation::Complete { capture } =
            evaluate_grouped_loop("prompt", &constituents, &completed).unwrap()
        else {
            panic!("the round did not finalize")
        };
        assert!(capture.proposals.is_empty());
        assert_eq!(capture.needs.len(), 1);
    }

    /// Each constituent's view stays under its own handle. Nothing is unioned,
    /// nothing is deduplicated, and the detail path's prompt is untouched: a
    /// shared builder is how the singleton prompt drifts by one byte and every
    /// persisted checkpoint fails its request-shape check on resume.
    #[test]
    fn soul_partitioned_views_do_not_cross_labels() {
        let mine = "ONLY-MINE-TAG";
        let yours = "ONLY-YOURS-TAG";
        let views = [
            LabeledView {
                handle: "c0",
                identity: "Mara",
                typed_view: mine,
                tool_signatures: "c0__speak(text)",
            },
            LabeledView {
                handle: "c1",
                identity: "Iris",
                typed_view: yours,
                tool_signatures: "c1__speak(text)",
            },
        ];
        let prompt = build_grouped_agent_prompt(&GroupedAgentPrompt {
            views: &views,
            decision_pressure: "pressure",
            domain_guidance: "",
            step_budget: CELL_TOOL_STEP_BUDGET,
        });
        assert_eq!(prompt.matches(mine).count(), 1);
        assert_eq!(prompt.matches(yours).count(), 1);
        let blocks: Vec<&str> = prompt.split("### ").skip(1).collect();
        assert_eq!(blocks.len(), 2);
        assert!(blocks[0].contains(mine) && !blocks[0].contains(yours));
        assert!(blocks[1].contains(yours) && !blocks[1].contains(mine));

        // The detail path's tool names and signatures are unprefixed, and its
        // prompt has no handle blocks at all.
        let granted = vec![speak_snapshot(AffordanceId::issue())];
        assert_eq!(
            catalog_signatures("", &granted),
            "speak(text), record_need(detail), finish_without_proposal()"
        );
        let singleton = build_operational_agent_prompt(&OperationalAgentPrompt {
            identity: "Mara",
            typed_view: mine,
            available_tools: &catalog_signatures("", &granted),
            decision_pressure: "Choose whether this decision owner should speak now.",
            domain_guidance: "",
            step_budget: TOOL_STEP_BUDGET,
        });
        assert!(!singleton.contains("### c0"));
        assert!(!singleton.contains("c0__"));
        assert_eq!(
            catalog_signatures("c1__", &granted),
            "c1__speak(text), c1__record_need(detail), c1__finish_without_proposal()"
        );
    }

    /// A `NarrativePersona` grouped this tick is represented operationally at
    /// coarse resolution. The membrane is not weakened; it is not entered: one
    /// inference, no Projector, no Persona, no Interpreter, and no Persona turn
    /// receipt. Its controller, scope, and authority do not change, and the
    /// kernel cannot tell the difference.
    #[tokio::test]
    async fn soul_a_coarse_narrative_persona_keeps_its_controller_and_enters_no_membrane() {
        let (_directory, mailbox, task) = active_cell_mailbox(vec![
            NewController::NarrativePersona,
            NewController::NarrativePersona,
        ])
        .await;
        let cell = one_group(&mailbox).await;
        let before = mailbox.snapshot().await.unwrap();
        for member in cell.members() {
            assert_eq!(
                member.opportunity.controller_mode,
                ControllerMode::NarrativePersona,
                "grouping changed a controller mode"
            );
        }

        // Exactly one output is supplied. A membrane turn would need three, and
        // the port panics when it runs dry.
        let (runner, store) = grouped_runner(
            &mailbox,
            vec![output(
                vec![
                    speak_call("c0", "c0__speak", "The hinge is flooding."),
                    InferenceEvent::ToolCall {
                        call_id: "c1".into(),
                        name: "c1__finish_without_proposal".into(),
                        arguments: "{}".into(),
                    },
                ],
                "coarse-narrative",
            )],
        );
        let CellRun::Grouped(run) = runner.run_cell(&cell).await.unwrap() else {
            panic!("a coarse cell did not run the grouped lane")
        };
        assert!(run.pending.is_none());
        assert_eq!(run.submissions.len(), 2);

        // No Persona turn was minted, and the stored row is a grouped one.
        let rows = store.work.lock().unwrap().clone();
        assert_eq!(rows.len(), 1);
        let row = rows.values().next().unwrap().clone();
        assert!(matches!(row, ControllerWork::Grouped(_)));
        assert_eq!(row.resolution(), Resolution::Coarse { constituents: 2 });
        let encoded = serde_json::to_string(&row).unwrap();
        assert!(
            !encoded.contains("persona_turn"),
            "a coarse turn minted a receipt"
        );
        assert!(!encoded.contains("projector"));
        assert!(!encoded.contains("interpreter"));

        // Controller, mode, and scope are exactly what they were.
        let after = mailbox.snapshot().await.unwrap();
        for subject in &after.subjects {
            let was = before
                .subjects
                .iter()
                .find(|entry| entry.id == subject.id)
                .unwrap();
            assert_eq!(subject.controller_id, was.controller_id);
            assert_eq!(subject.controller_mode, was.controller_mode);
        }
        assert_eq!(mailbox.operator_log().await.unwrap().len(), 1);

        drop(runner);
        drop(mailbox);
        task.await.unwrap();
    }

    /// A resumed cell re-derives the same command ids, so the kernel answers
    /// from its idempotency ledger instead of committing a second time. There is
    /// no persisted per-constituent outcome doing this job: the ledger already
    /// owns it.
    #[tokio::test]
    async fn soul_a_resumed_cell_re_derives_its_ids_and_commits_nothing_twice() {
        let (_directory, mailbox, task) = active_cell_mailbox(vec![
            NewController::OperationalAgent,
            NewController::OperationalAgent,
        ])
        .await;
        let cell = one_group(&mailbox).await;
        let (runner, _store) = grouped_runner(
            &mailbox,
            vec![output(
                vec![
                    speak_call("c0", "c0__speak", "Hold the bridge."),
                    InferenceEvent::ToolCall {
                        call_id: "c1".into(),
                        name: "c1__finish_without_proposal".into(),
                        arguments: "{}".into(),
                    },
                ],
                "grouped",
            )],
        );
        let CellRun::Grouped(first) = runner.run_cell(&cell).await.unwrap() else {
            panic!("a coarse cell did not run the grouped lane")
        };
        assert_eq!(first.submissions.len(), 2);
        assert!(matches!(
            first.submissions[0].submission,
            SubmissionDisposition::Completed(_)
        ));
        assert!(matches!(
            first.submissions[1].submission,
            SubmissionDisposition::NoProposal(_)
        ));
        let committed = mailbox.operator_log().await.unwrap().len();
        assert_eq!(committed, 1);

        // The port has no outputs left: a second inference would panic. The
        // resumed row goes straight to submission, and every constituent's
        // derived id answers from the ledger.
        let CellRun::Grouped(second) = runner.run_cell(&cell).await.unwrap() else {
            panic!("a resumed coarse cell did not run the grouped lane")
        };
        assert!(
            matches!(
                second.submissions[0].submission,
                SubmissionDisposition::PreviouslyConfirmed(_)
            ),
            "a resumed cell committed again: {:?}",
            second.submissions
        );
        assert!(matches!(
            second.submissions[1].submission,
            SubmissionDisposition::NoProposal(SubmitReceipt::AlreadyApplied(_))
        ));
        assert_eq!(mailbox.operator_log().await.unwrap().len(), committed);

        drop(runner);
        drop(mailbox);
        task.await.unwrap();
    }

    /// Adjacency is a scheduler projection, not a subject view. Two subjects in
    /// one room are related through the kernel's own containment closure, which
    /// no union of `SubjectSnapshot`s could produce for a channel, and the
    /// result reaches `derive_cover` and nothing else: `run_cell` takes a
    /// `&Cell`, which holds no graph, so a controller organ that read adjacency
    /// would fail to compile.
    #[tokio::test]
    async fn soul_the_agency_graph_relates_co_located_subjects_and_reaches_no_prompt() {
        let (_directory, mailbox, task) = active_cell_mailbox(vec![
            NewController::OperationalAgent,
            NewController::OperationalAgent,
            NewController::NarrativePersona,
        ])
        .await;
        let graph = mailbox.agency_graph().await.unwrap();
        assert_eq!(graph.subjects.len(), 3);
        assert_eq!(
            graph.edges.len(),
            3,
            "three subjects in one room are a clique"
        );
        for (one, other) in &graph.edges {
            assert!(one < other, "an edge is not canonicalised");
        }
        // Nothing about adjacency reaches a subject-facing surface: the typed
        // view a constituent is shown carries its own state and no neighbour's.
        let snapshot = mailbox.snapshot().await.unwrap();
        let selected = select_one(&snapshot, &snapshot.opportunities[0]).unwrap();
        let view = selected.typed_view().unwrap();
        for other in snapshot
            .subjects
            .iter()
            .filter(|subject| subject.id != selected.subject.id)
        {
            assert!(
                !view.contains(&other.label),
                "a typed view named a neighbour"
            );
        }

        drop(mailbox);
        task.await.unwrap();
    }

    /// A cell is not a transaction, and one inference does not buy N commits.
    ///
    /// Two co-located constituents both propose. The first commits; the second
    /// was reasoned from a scope its neighbour has since changed, and the kernel
    /// refuses it with `ScopeChanged` rather than committing a proposal whose
    /// binding no longer holds. The batch buys one inference, never one
    /// admission rule.
    ///
    /// This is the live cost of a connected cover: the cover groups subjects
    /// precisely because they are causally coupled, and coupled subjects
    /// contend for the same scope.
    #[tokio::test]
    async fn soul_a_second_act_in_one_cell_is_refused_rather_than_committed_on_a_stale_scope() {
        let (_directory, mailbox, task) = active_cell_mailbox(vec![
            NewController::OperationalAgent,
            NewController::OperationalAgent,
        ])
        .await;
        let cell = one_group(&mailbox).await;
        let (runner, _store) = grouped_runner(
            &mailbox,
            vec![output(
                vec![
                    speak_call("c0", "c0__speak", "The hinge is flooding."),
                    speak_call("c1", "c1__speak", "Then we close the span."),
                ],
                "two-acts",
            )],
        );
        let CellRun::Grouped(run) = runner.run_cell(&cell).await.unwrap() else {
            panic!("a coarse cell did not run the grouped lane")
        };
        assert_eq!(run.submissions.len(), 1, "two acts committed from one cell");
        assert!(matches!(
            run.submissions[0].submission,
            SubmissionDisposition::Completed(_)
        ));
        assert!(
            run.needs
                .iter()
                .any(|need| need.detail.contains("scope changed")),
            "the refused act was not recorded: {:?}",
            run.needs
        );
        assert_eq!(mailbox.operator_log().await.unwrap().len(), 1);

        drop(runner);
        drop(mailbox);
        task.await.unwrap();
    }

    /// The same fixture, with every subject standing in its own room. The
    /// agency graph relates none of them, so the cell that packs them together
    /// holds constituents that cannot change each other's scope.
    async fn apart_cell_mailbox(
        controllers: Vec<NewController>,
    ) -> (tempfile::TempDir, WorldMailbox, tokio::task::JoinHandle<()>) {
        let directory = tempfile::tempdir().unwrap();
        let (mailbox, task) = WorldMailbox::open(directory.path().join("world.cc")).unwrap();
        let owner = PrincipalId::new("owner");
        let authenticated = AuthenticatedCaller::fixture(CallerId::Principal(owner.clone()));
        let mut declarations = Vec::new();
        for (index, controller) in controllers.into_iter().enumerate() {
            declarations.push(Declaration::Entity(EntityDeclaration {
                handle: DraftHandle::new(&format!("room{index}")),
                label: format!("Room {index}"),
                kind: EntityKind::Place,
                container: None,
            }));
            declarations.push(Declaration::Subject(SubjectDeclaration {
                handle: DraftHandle::new(&format!("subject{index}")),
                label: format!("Subject {index}"),
                kind: SubjectKind::Person,
                controller,
                affordances: kernel_speak_grant(),
                position: Some(Ref::Draft(DraftHandle::new(&format!("room{index}")))),
            }));
        }
        let creation = mailbox
            .create_fixture(
                CreateWorld {
                    id: CommandId::new(),
                    owner: owner.clone(),
                    title: "Apart Fixture".into(),
                    patch: WorldPatch {
                        declarations,
                        operations: Vec::new(),
                        evidence: Vec::new(),
                    },
                    scale_intent: WorldScaleIntentRef::default(),
                },
                &authenticated,
            )
            .await
            .unwrap();
        let mut snapshot = mailbox.snapshot().await.unwrap();
        for body in [CommandBody::ApproveDraft, CommandBody::ActivateWorld] {
            mailbox
                .submit_fixture(
                    CommandEnvelope {
                        id: CommandId::new(),
                        world_id: creation.world_id,
                        expected_revision: snapshot.revision,
                        caller: CallerId::Principal(owner.clone()),
                        body,
                    },
                    &authenticated,
                )
                .await
                .unwrap();
            snapshot = mailbox.snapshot().await.unwrap();
        }
        assert_eq!(snapshot.phase, WorldPhase::Active);
        (directory, mailbox, task)
    }

    /// The contention finding is a fact about coupling, not about grouping. Two
    /// constituents in separate rooms both propose from one inference and both
    /// commit: the refusal a coupled cell earns is not a per-cell quota, and it
    /// is not the world revision moving underneath the second submission.
    #[tokio::test]
    async fn soul_b_an_uncoupled_grouped_cell_commits_every_constituent() {
        let (_directory, mailbox, task) = apart_cell_mailbox(vec![
            NewController::OperationalAgent,
            NewController::OperationalAgent,
        ])
        .await;
        let graph = mailbox.agency_graph().await.unwrap();
        assert!(
            graph.edges.is_empty(),
            "subjects in separate rooms were related: {:?}",
            graph.edges
        );
        let cell = one_group(&mailbox).await;
        assert_eq!(cell.members().len(), 2);

        let (runner, _store) = grouped_runner(
            &mailbox,
            vec![output(
                vec![
                    speak_call("c0", "c0__speak", "The east hinge is dry."),
                    speak_call("c1", "c1__speak", "The west hinge is dry."),
                ],
                "two-uncoupled-acts",
            )],
        );
        let CellRun::Grouped(run) = runner.run_cell(&cell).await.unwrap() else {
            panic!("a coarse cell did not run the grouped lane")
        };
        assert!(run.pending.is_none());
        assert_eq!(
            run.submissions.len(),
            2,
            "an uncoupled constituent was refused: {:?}",
            run.needs
        );
        for entry in &run.submissions {
            assert!(matches!(
                entry.submission,
                SubmissionDisposition::Completed(_)
            ));
        }
        assert!(run.needs.is_empty(), "{:?}", run.needs);
        assert_eq!(mailbox.operator_log().await.unwrap().len(), 2);

        drop(runner);
        drop(mailbox);
        task.await.unwrap();
    }

    /// The refusal a coupled cell earns is honest: the second constituent's
    /// scope digest really is different after its neighbour acted. A refusal
    /// that fired on a moved world revision rather than on a moved scope would
    /// leave this digest untouched.
    #[tokio::test]
    async fn soul_b_the_refused_constituent_bound_a_scope_that_really_changed() {
        let (_directory, mailbox, task) = active_cell_mailbox(vec![
            NewController::OperationalAgent,
            NewController::OperationalAgent,
        ])
        .await;
        let cell = one_group(&mailbox).await;
        let second = cell.members()[1].subject;
        let digest_of = |snapshot: &WorldSnapshot, subject: SubjectId| {
            snapshot
                .opportunities
                .iter()
                .find(|entry| entry.scope.subject_id == subject)
                .map(|entry| entry.scope_digest.clone())
        };
        let before = digest_of(&mailbox.snapshot().await.unwrap(), second)
            .expect("the second constituent has an opportunity before the cell runs");

        let (runner, _store) = grouped_runner(
            &mailbox,
            vec![output(
                vec![
                    speak_call("c0", "c0__speak", "The hinge is flooding."),
                    speak_call("c1", "c1__speak", "Then we close the span."),
                ],
                "coupled-acts",
            )],
        );
        let CellRun::Grouped(run) = runner.run_cell(&cell).await.unwrap() else {
            panic!("a coarse cell did not run the grouped lane")
        };
        assert_eq!(run.submissions.len(), 1);
        assert!(
            run.needs
                .iter()
                .any(|need| need.detail.contains("scope changed")),
            "{:?}",
            run.needs
        );

        let after = digest_of(&mailbox.snapshot().await.unwrap(), second)
            .expect("the refused constituent still has an opportunity");
        assert_ne!(
            before, after,
            "the refusal named a scope change that did not happen"
        );

        drop(runner);
        drop(mailbox);
        task.await.unwrap();
    }

    /// The other checkpoint's resume. A retryable fault leaves the row at
    /// `AgentInFlight` with nothing committed; the resumed run re-derives the
    /// same constituent ids from the persisted row, infers once, and commits
    /// once. Both grouped checkpoints therefore resume without a second commit.
    #[tokio::test]
    async fn soul_b_a_cell_resumed_from_agent_in_flight_commits_nothing_twice() {
        let (_directory, mailbox, task) = active_cell_mailbox(vec![
            NewController::OperationalAgent,
            NewController::OperationalAgent,
        ])
        .await;
        let cell = one_group(&mailbox).await;
        let (runner, store) = grouped_runner(
            &mailbox,
            vec![
                Err(InferenceFault::retryable("the provider is busy")),
                output(
                    vec![
                        speak_call("c0", "c0__speak", "Hold the bridge."),
                        InferenceEvent::ToolCall {
                            call_id: "c1".into(),
                            name: "c1__finish_without_proposal".into(),
                            arguments: "{}".into(),
                        },
                    ],
                    "grouped-after-retry",
                ),
            ],
        );

        let CellRun::Grouped(first) = runner.run_cell(&cell).await.unwrap() else {
            panic!("a coarse cell did not run the grouped lane")
        };
        assert!(
            first.pending.is_some(),
            "a retryable fault did not park the cell"
        );
        assert!(first.submissions.is_empty());
        assert_eq!(mailbox.operator_log().await.unwrap().len(), 0);

        let parked = store.work.lock().unwrap().values().next().unwrap().clone();
        let ControllerWork::Grouped(GroupedCheckpoint::AgentInFlight { constituents, .. }) =
            &parked
        else {
            panic!("the parked row is not an in-flight grouped checkpoint: {parked:?}")
        };
        let parked_ids: Vec<CommandId> = constituents
            .iter()
            .map(|constituent| constituent.command_id)
            .collect();

        let CellRun::Grouped(second) = runner.run_cell(&cell).await.unwrap() else {
            panic!("a resumed coarse cell did not run the grouped lane")
        };
        assert!(second.pending.is_none());
        assert_eq!(second.submissions.len(), 2);
        assert_eq!(mailbox.operator_log().await.unwrap().len(), 1);

        let resumed = store.work.lock().unwrap().values().next().unwrap().clone();
        let ControllerWork::Grouped(GroupedCheckpoint::Submitting { constituents, .. }) = &resumed
        else {
            panic!("the resumed row did not reach submission: {resumed:?}")
        };
        assert_eq!(
            parked_ids,
            constituents
                .iter()
                .map(|constituent| constituent.command_id)
                .collect::<Vec<_>>(),
            "a resumed cell derived different constituent ids"
        );

        // A third pass has no inference left: it must answer from the ledger.
        let CellRun::Grouped(third) = runner.run_cell(&cell).await.unwrap() else {
            panic!("a twice-resumed coarse cell did not run the grouped lane")
        };
        assert!(matches!(
            third.submissions[0].submission,
            SubmissionDisposition::PreviouslyConfirmed(_)
        ));
        assert_eq!(mailbox.operator_log().await.unwrap().len(), 1);

        drop(runner);
        drop(mailbox);
        task.await.unwrap();
    }

    /// The replay half of the grouped-tick verification. A coarse cell reaches
    /// the kernel as ordinary one-opportunity submissions, so the store it
    /// leaves must replay from genesis to the same digest and the same events.
    /// If a cell were a command shape, this is where it would show.
    #[tokio::test]
    async fn soul_b_a_grouped_tick_replays_to_an_identical_state_digest() {
        let (directory, mailbox, task) = active_cell_mailbox(vec![
            NewController::OperationalAgent,
            NewController::OperationalAgent,
            NewController::NarrativePersona,
        ])
        .await;
        let path = directory.path().join("world.cc");
        let cell = one_group(&mailbox).await;
        assert_eq!(cell.members().len(), 3);
        let (runner, _store) = grouped_runner(
            &mailbox,
            vec![
                output(
                    vec![
                        speak_call("c0", "c0__speak", "The hinge is flooding."),
                        speak_call("outside", "c9__speak", "I speak for someone else."),
                    ],
                    "replay-one",
                ),
                output(
                    vec![InferenceEvent::Text("Nothing further.".into())],
                    "replay-two",
                ),
            ],
        );
        let CellRun::Grouped(run) = runner.run_cell(&cell).await.unwrap() else {
            panic!("a coarse cell did not run the grouped lane")
        };
        assert!(run.pending.is_none());
        assert_eq!(run.submissions.len(), 3);

        let live = mailbox.snapshot().await.unwrap();
        assert_eq!(mailbox.operator_log().await.unwrap().len(), 1);
        drop(runner);
        drop(mailbox);
        task.await.unwrap();

        let replayed =
            crate::world::WorldKernel::open(&path, live.world_id).expect("the world store replays");
        assert_eq!(
            replayed.state.state_digest, live.state_digest,
            "a grouped tick did not replay to the same digest"
        );
        assert_eq!(replayed.state.revision, live.revision);
        // The event sequence, not just its digest: a replay that agreed on a
        // digest while disagreeing on what happened would be a worse bug.
        let events = serde_json::to_string(&replayed.state.events).unwrap();
        assert!(events.contains("The hinge is flooding."));
        for forbidden in ["cell-act", "cell-work", "c0__", "constituent", "resolution"] {
            assert!(
                !events.contains(forbidden),
                "the replayed event sequence names `{forbidden}`"
            );
        }
        // A second open is the same open: replay is a function of the store.
        let expected = replayed.state.clone();
        drop(replayed);
        let again = crate::world::WorldKernel::open(&path, live.world_id)
            .expect("the world store replays twice");
        assert_eq!(again.state, expected);
    }

    /// The cover is derived and disposable. Nothing the kernel persists or
    /// hands out names a cell, a handle, or a resolution: the only durable
    /// trace is the controller-work row, whose custody is separate from world
    /// custody.
    #[tokio::test]
    async fn soul_a_cell_leaves_no_trace_in_world_state() {
        let (_directory, mailbox, task) = active_cell_mailbox(vec![
            NewController::OperationalAgent,
            NewController::OperationalAgent,
        ])
        .await;
        let cell = one_group(&mailbox).await;
        let (runner, _store) = grouped_runner(
            &mailbox,
            vec![output(
                vec![
                    speak_call("c0", "c0__speak", "Hold the bridge."),
                    InferenceEvent::ToolCall {
                        call_id: "c1".into(),
                        name: "c1__finish_without_proposal".into(),
                        arguments: "{}".into(),
                    },
                ],
                "grouped",
            )],
        );
        runner.run_cell(&cell).await.unwrap();

        let snapshot = mailbox.snapshot().await.unwrap();
        for opportunity in &snapshot.opportunities {
            let encoded = serde_json::to_string(opportunity).unwrap();
            for forbidden in ["cell", "resolution", "constituent", "handle"] {
                assert!(
                    !encoded.contains(forbidden),
                    "an opportunity carries `{forbidden}`"
                );
            }
        }
        assert!(
            !serde_json::to_string(&snapshot.state_digest)
                .unwrap()
                .is_empty()
        );

        drop(runner);
        drop(mailbox);
        task.await.unwrap();
    }
    /// written before it holds.
    #[test]
    fn soul_a_controller_work_row_from_v7_is_refused_at_open() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("controller-work.cc");
        let command_id = CommandId::new();
        let opportunity = fixture_opportunity(ControllerMode::OperationalAgent);
        let work = operational_in_flight(
            command_id,
            &opportunity,
            "Hold the bridge.",
            "operational-model",
            vec![],
        );
        {
            let mut store = OwnedRedbMessagePackBackingStore::new(&path).unwrap();
            store
                .push(&CultCacheEnvelope {
                    key: store_key(command_id).unwrap(),
                    r#type: "controller_work.v7".into(),
                    payload: rmp_serde::to_vec_named(&work).unwrap(),
                    stored_at: Utc::now().to_rfc3339(),
                    schema_id: Some("ghostlight.controller_work.v7".into()),
                })
                .unwrap();
        }
        let Err(error) = CultCacheControllerWorkStore::open(&path) else {
            panic!("a v7 row was accepted by the v9 store");
        };
        assert!(matches!(error, ControllerWorkStoreError::Fault { .. }));
    }

    /// The custody discriminator names the lane, and `Uncertain` round-trips
    /// through the probe with the authoring lane's own name rather than a
    /// subject's control mode.
    #[test]
    fn soul_controller_work_custody_names_the_authoring_lane() {
        let command_id = CommandId::new();
        let uncertain = ControllerWorkCustody::Uncertain {
            command_id,
            lane: WorkLane::Elaboration,
        };
        assert_eq!(
            uncertain,
            ControllerWorkCustody::Uncertain {
                command_id,
                lane: WorkLane::Elaboration,
            }
        );
        assert_ne!(
            uncertain,
            ControllerWorkCustody::Uncertain {
                command_id,
                lane: WorkLane::Operational,
            }
        );
        assert_ne!(
            ControllerWorkCustody::Owned {
                narrative_commands: 0,
                operational_commands: 0,
                elaboration_commands: 1,
                seed_commands: 0,
            },
            ControllerWorkCustody::Owned {
                narrative_commands: 0,
                operational_commands: 1,
                elaboration_commands: 0,
                seed_commands: 0,
            }
        );
    }

    // ---- The seed lane ---------------------------------------------------

    struct SeedFixture {
        _directory: tempfile::TempDir,
        mailbox: WorldMailbox,
        task: tokio::task::JoinHandle<()>,
        owner: PrincipalId,
        principal: crate::app_session::VerifiedPrincipalEvidence,
        sere: EntityId,
        speak: AffordanceId,
    }

    /// A Draft world with an authored scale intent: persons wanted in the Low
    /// Sere, none alive yet, plus the owner's own subject standing in the
    /// commons, which no declared root covers.
    async fn seed_mailbox(target: u32) -> SeedFixture {
        let directory = tempfile::tempdir().unwrap();
        let (mailbox, task) = WorldMailbox::open(directory.path().join("world.cc")).unwrap();
        let owner = PrincipalId::new("seed-owner");
        let authenticated = AuthenticatedCaller::fixture(CallerId::Principal(owner.clone()));
        mailbox
            .create_fixture(
                CreateWorld {
                    id: CommandId::new(),
                    owner: owner.clone(),
                    title: "Seed Fixture".into(),
                    patch: WorldPatch {
                        declarations: vec![
                            Declaration::Entity(EntityDeclaration {
                                handle: DraftHandle::new("commons"),
                                label: "The Commons".into(),
                                kind: EntityKind::Place,
                                container: None,
                            }),
                            Declaration::Entity(EntityDeclaration {
                                handle: DraftHandle::new("sere"),
                                label: "The Low Sere".into(),
                                kind: EntityKind::Place,
                                container: None,
                            }),
                            Declaration::Subject(SubjectDeclaration {
                                handle: DraftHandle::new("first-person"),
                                label: "The Owner".into(),
                                kind: SubjectKind::Person,
                                controller: NewController::Human {
                                    principal: owner.clone(),
                                },
                                affordances: kernel_speak_grant(),
                                position: Some(Ref::Draft(DraftHandle::new("commons"))),
                            }),
                        ],
                        operations: Vec::new(),
                        evidence: Vec::new(),
                    },
                    scale_intent: WorldScaleIntentRef {
                        targets: BTreeMap::from([(SubjectKind::Person, target)]),
                        jurisdictions: BTreeMap::from([(DraftHandle::new("sere"), 1000)]),
                    },
                },
                &authenticated,
            )
            .await
            .unwrap();
        let snapshot = mailbox.snapshot().await.unwrap();
        assert_eq!(snapshot.phase, WorldPhase::Draft);
        let sere = snapshot
            .places
            .iter()
            .find(|place| place.label == "The Low Sere")
            .expect("the declared root")
            .id;
        let speak = snapshot
            .affordances
            .iter()
            .find(|entry| entry.entry.kind.0 == "speak")
            .expect("the kernel Speak entry")
            .id;
        SeedFixture {
            _directory: directory,
            mailbox,
            task,
            owner,
            principal: crate::app_session::VerifiedPrincipalEvidence::fixture(
                "seed-owner",
                Utc::now() + chrono::Duration::hours(1),
            ),
            sere,
            speak,
        }
    }

    fn seed_port(fixture: &SeedFixture) -> SeedPort {
        SeedPort::new(fixture.mailbox.clone(), fixture.principal.clone())
    }

    /// One session that declares `handles.len()` persons who qualify: a
    /// controller, the granted Speak entry, a position inside the root, and a
    /// personal goal each.
    fn author_persons(
        receipt: &str,
        root: EntityId,
        speak: AffordanceId,
        handles: &[&str],
    ) -> Result<InferenceOutput, InferenceFault> {
        author_across(receipt, speak, &[(root, handles)])
    }

    /// The same, over more than one jurisdiction root in one patch.
    fn author_across(
        receipt: &str,
        speak: AffordanceId,
        groups: &[(EntityId, &[&str])],
    ) -> Result<InferenceOutput, InferenceFault> {
        let entry = serde_json::to_value(speak).unwrap();
        let mut calls: Vec<(&str, Value)> = Vec::new();
        let first = groups
            .first()
            .and_then(|(_, handles)| handles.first().copied());
        for (root, handles) in groups {
            author_group(&mut calls, *root, &entry, handles, first);
        }
        calls.push(("submit", json!({})));
        tool_round(calls, receipt)
    }

    fn author_group(
        calls: &mut Vec<(&'static str, Value)>,
        root: EntityId,
        entry: &Value,
        handles: &[&str],
        owed: Option<&str>,
    ) {
        let place = serde_json::to_value(root).unwrap();
        for (index, handle) in handles.iter().enumerate() {
            calls.push((
                "declare_subject",
                json!({
                    "handle": handle,
                    "label": format!("Sere {handle}"),
                    "kind": "person",
                    "controller": {"type": "narrative_persona"},
                    "affordances": [{"ref": "existing", "value": entry}],
                    "position": {"ref": "existing", "value": place},
                }),
            ));
            calls.push((
                "create_commitment",
                json!({
                    "subject": {"ref": "draft", "value": handle},
                    "counterparty": null,
                    "kind": "goal",
                    "due": 600,
                    "period": null,
                    "checks": [],
                }),
            ));
            // Everyone after the first owes the first something. That is what
            // leaves the Active elaborator a `MissingStructure` boundary to
            // answer; a goal cannot, because a goal carrying a counterparty is
            // refused with `GoalWithCounterparty`.
            if let Some(owed) = owed.filter(|owed| owed != handle) {
                let _ = index;
                calls.push((
                    "create_commitment",
                    json!({
                        "subject": {"ref": "draft", "value": handle},
                        "counterparty": {"ref": "draft", "value": owed},
                        "kind": "obligation",
                        "due": 900,
                        "period": null,
                        "checks": [],
                    }),
                ));
            }
        }
    }

    fn fresh_store() -> Arc<RecordingWorkStore> {
        Arc::new(RecordingWorkStore {
            persisted: Arc::new(AtomicBool::new(false)),
            work: Mutex::new(BTreeMap::new()),
        })
    }

    fn seed_script(rounds: Vec<Result<InferenceOutput, InferenceFault>>) -> Arc<ElaborationScript> {
        Arc::new(ElaborationScript {
            outputs: Mutex::new(rounds),
            seen: Mutex::new(Vec::new()),
        })
    }

    /// A fixture Vault that hands back exactly what it was built with, so a
    /// citation test can name a reference inside and outside the retrieved set
    /// without a filesystem.
    struct FixtureVault {
        receipts: Vec<EvidenceReceipt>,
    }

    #[async_trait]
    impl EvidenceSource for FixtureVault {
        async fn retrieve(
            &self,
            _query: &EvidenceQuery,
        ) -> Result<Vec<EvidenceReceipt>, EvidenceError> {
            Ok(self.receipts.clone())
        }
    }

    fn seed_runner(
        fixture: &SeedFixture,
        script: Arc<ElaborationScript>,
        store: Arc<RecordingWorkStore>,
        evidence: Arc<dyn EvidenceSource>,
    ) -> SeedRunner {
        SeedRunner::new(
            seed_port(fixture),
            script,
            evidence,
            store,
            models().elaborator,
            None,
        )
    }

    fn root_deficit(snapshot: &WorldSnapshot, root: EntityId) -> u32 {
        snapshot
            .scale_deficit
            .iter()
            .find(|row| row.jurisdiction == JurisdictionKey::PlaceSubtree(root))
            .map_or(0, |row| row.deficit)
    }

    /// Spec test 4. One session commits one Draft patch as the owner's own act,
    /// and every subject it declared qualifies.
    #[tokio::test]
    async fn a_seed_session_commits_a_draft_patch_as_the_owner() {
        let fixture = seed_mailbox(6).await;
        let before = fixture.mailbox.snapshot().await.unwrap();
        let runner = seed_runner(
            &fixture,
            seed_script(vec![author_persons(
                "seed-round-zero",
                fixture.sere,
                fixture.speak,
                &["digger", "warden"],
            )]),
            fresh_store(),
            Arc::new(NullEvidenceSource),
        );
        assert_eq!(runner.step().await.unwrap(), SeedOutcome::Committed);

        let after = fixture.mailbox.snapshot().await.unwrap();
        assert_eq!(after.revision, before.revision + 1);
        assert_eq!(after.phase, WorldPhase::Draft, "a seed never activates");
        for label in ["Sere digger", "Sere warden"] {
            let subject = after
                .subjects
                .iter()
                .find(|subject| subject.label == label)
                .expect("the declared subject");
            assert!(subject.controller_id.is_some());
            assert!(!subject.affordances.is_empty());
            assert_eq!(subject.position, Some(fixture.sere));
            assert!(
                subject
                    .commitments
                    .iter()
                    .any(|held| held.kind == CommitmentKind::Goal)
            );
            assert!(subject.qualified, "{label} does not reduce the deficit");
        }
        assert_eq!(root_deficit(&after, fixture.sere), 4);

        // The commit is the owner's own act, and that is not a convention: the
        // only unconfined Draft author is `Principal(owner)`, so a port built
        // on anyone else's evidence is refused by the reducer.
        let stranger = SeedPort::new(
            fixture.mailbox.clone(),
            crate::app_session::VerifiedPrincipalEvidence::fixture(
                "not-the-owner",
                Utc::now() + chrono::Duration::hours(1),
            ),
        );
        let refused = stranger
            .submit_seed(
                CommandId::new(),
                after.world_id,
                WorldPatch {
                    declarations: vec![Declaration::Entity(EntityDeclaration {
                        handle: DraftHandle::new("shed"),
                        label: "A Shed".into(),
                        kind: EntityKind::Place,
                        container: None,
                    })],
                    operations: Vec::new(),
                    evidence: Vec::new(),
                },
            )
            .await;
        assert!(
            matches!(
                refused,
                Err(MailboxError::Kernel(KernelError::Unauthorized))
            ),
            "{refused:?}"
        );
        drop(stranger);

        drop(runner);
        drop(fixture.mailbox);
        fixture.task.await.unwrap();
    }

    /// Spec test 5. Termination is derived: the deficit falls as patches land
    /// and the sweep stops at zero without spending a further inference call.
    #[tokio::test]
    async fn the_deficit_falls_as_patches_land_and_the_sweep_stops_at_zero() {
        let fixture = seed_mailbox(6).await;
        assert_eq!(
            root_deficit(&fixture.mailbox.snapshot().await.unwrap(), fixture.sere),
            6
        );
        let scripted = seed_script(vec![
            author_persons("r0", fixture.sere, fixture.speak, &["a1", "a2"]),
            author_persons("r1", fixture.sere, fixture.speak, &["b1", "b2"]),
            author_persons("r2", fixture.sere, fixture.speak, &["c1", "c2"]),
        ]);
        let runner = seed_runner(
            &fixture,
            scripted.clone(),
            fresh_store(),
            Arc::new(NullEvidenceSource),
        );
        assert_eq!(runner.sweep(8).await.unwrap(), SeedOutcome::Clean);
        assert_eq!(
            root_deficit(&fixture.mailbox.snapshot().await.unwrap(), fixture.sere),
            0
        );
        assert_eq!(
            scripted.seen.lock().unwrap().len(),
            3,
            "the fourth step is clean and must spend nothing"
        );

        drop(runner);
        drop(fixture.mailbox);
        fixture.task.await.unwrap();
    }

    /// Spec test 6. A commit that does not strictly lower its row is a fixed
    /// point, not a retry.
    #[tokio::test]
    async fn a_committed_patch_that_does_not_move_the_deficit_stops_the_row() {
        let fixture = seed_mailbox(6).await;
        let scripted = seed_script(vec![tool_round(
            vec![
                (
                    "declare_place",
                    json!({"handle": "shed", "label": "A Shed", "container": null}),
                ),
                ("submit", json!({})),
            ],
            "r0",
        )]);
        let runner = seed_runner(
            &fixture,
            scripted.clone(),
            fresh_store(),
            Arc::new(NullEvidenceSource),
        );
        assert_eq!(runner.sweep(4).await.unwrap(), SeedOutcome::NoProgress);
        assert_eq!(
            scripted.seen.lock().unwrap().len(),
            1,
            "a fixed point spun against the endpoint"
        );

        drop(runner);
        drop(fixture.mailbox);
        fixture.task.await.unwrap();
    }

    /// Spec test 7. Seeding is Draft's lane, refused twice: the runner's own
    /// phase gate spends nothing, and `SeedPort::submit_seed` refuses the
    /// submission directly, because it takes its own snapshot and refuses
    /// outright once the phase has moved off Draft — before it ever asks what
    /// the patch declares.
    #[tokio::test]
    async fn a_seed_patch_cannot_be_admitted_in_active_by_the_seed_lane() {
        let fixture = seed_mailbox(6).await;
        let mut snapshot = fixture.mailbox.snapshot().await.unwrap();
        let authenticated =
            AuthenticatedCaller::fixture(CallerId::Principal(fixture.owner.clone()));
        for body in [CommandBody::ApproveDraft, CommandBody::ActivateWorld] {
            fixture
                .mailbox
                .submit_fixture(
                    CommandEnvelope {
                        id: CommandId::new(),
                        world_id: snapshot.world_id,
                        expected_revision: snapshot.revision,
                        caller: CallerId::Principal(fixture.owner.clone()),
                        body,
                    },
                    &authenticated,
                )
                .await
                .unwrap();
            snapshot = fixture.mailbox.snapshot().await.unwrap();
        }
        assert_eq!(snapshot.phase, WorldPhase::Active);

        let store = fresh_store();
        let scripted = seed_script(vec![author_persons(
            "r0",
            fixture.sere,
            fixture.speak,
            &["late"],
        )]);
        let runner = seed_runner(
            &fixture,
            scripted.clone(),
            store.clone(),
            Arc::new(NullEvidenceSource),
        );
        assert_eq!(runner.step().await.unwrap(), SeedOutcome::NotDraft);
        assert!(scripted.seen.lock().unwrap().is_empty());
        assert!(store.work.lock().unwrap().is_empty());

        let refused = seed_port(&fixture)
            .submit_seed(
                CommandId::new(),
                snapshot.world_id,
                WorldPatch {
                    declarations: vec![Declaration::Entity(EntityDeclaration {
                        handle: DraftHandle::new("shed"),
                        label: "A Shed".into(),
                        kind: EntityKind::Place,
                        container: None,
                    })],
                    operations: Vec::new(),
                    evidence: Vec::new(),
                },
            )
            .await;
        assert!(
            matches!(
                refused,
                Err(MailboxError::Kernel(KernelError::WrongPhase {
                    expected: WorldPhase::Draft,
                    actual: WorldPhase::Active,
                }))
            ),
            "{refused:?}"
        );

        drop(runner);
        drop(fixture.mailbox);
        fixture.task.await.unwrap();
    }

    /// Spec test 8. `SeedPort` has two methods. Approve, activate, the clock,
    /// the operator log, and the agency graph are not among them, so reaching
    /// for one fails to compile rather than failing a test; what runs here is
    /// the other half — the body is hardcoded, so the lane cannot express any
    /// command but an unanswered `AdmitPatch`, and a commit through it moves
    /// neither the phase, nor the approvals, nor the clock.
    #[tokio::test]
    async fn the_seed_lane_cannot_approve_or_activate() {
        let fixture = seed_mailbox(6).await;
        let port = seed_port(&fixture);
        let snapshot = port.snapshot().await.unwrap();
        assert_eq!(snapshot.phase, WorldPhase::Draft);
        assert!(snapshot.draft_approvals.is_empty());

        port.submit_seed(
            CommandId::new(),
            snapshot.world_id,
            WorldPatch {
                declarations: vec![Declaration::Entity(EntityDeclaration {
                    handle: DraftHandle::new("shed"),
                    label: "A Shed".into(),
                    kind: EntityKind::Place,
                    container: None,
                })],
                operations: Vec::new(),
                evidence: Vec::new(),
            },
        )
        .await
        .unwrap();
        let after = port.snapshot().await.unwrap();
        assert_eq!(after.revision, snapshot.revision + 1);
        assert_eq!(after.phase, WorldPhase::Draft);
        assert!(after.draft_approvals.is_empty());
        assert_eq!(after.now, snapshot.now, "the lane moved the clock");

        drop(port);
        drop(fixture.mailbox);
        fixture.task.await.unwrap();
    }

    /// Spec test 9. Provenance's owner is the runner. A citation outside the
    /// round's retrieved set is dropped before the kernel sees it, so the
    /// canonical fact that named it is refused; the same fact citing a
    /// reference the Vault did return commits.
    #[tokio::test]
    async fn a_citation_outside_the_retrieved_set_is_dropped_and_an_uncited_fact_is_refused() {
        let retrieved = EvidenceReceipt {
            reference: EvidenceRef::new("Public/Places/Low Sere.md"),
            excerpt: "The sere runs dry nine months a year.".into(),
            source: "Low Sere".into(),
        };
        let fact = |reference: &str| {
            tool_round(
                vec![
                    (
                        "declare_fact",
                        json!({
                            "handle": "dry",
                            "label": "The Dry Season",
                            "statement": "The sere runs dry nine months a year.",
                            "standing": {"standing": "canonical", "evidence": reference},
                        }),
                    ),
                    ("submit", json!({})),
                ],
                "r0",
            )
        };

        let forged = seed_mailbox(6).await;
        let before = forged.mailbox.snapshot().await.unwrap().revision;
        let refused = seed_runner(
            &forged,
            seed_script(vec![fact("Public/Places/Nowhere.md")]),
            fresh_store(),
            Arc::new(FixtureVault {
                receipts: vec![retrieved.clone()],
            }),
        );
        assert_eq!(refused.step().await.unwrap(), SeedOutcome::Rejected);
        assert_eq!(
            forged.mailbox.snapshot().await.unwrap().revision,
            before,
            "a forged citation minted canon"
        );
        drop(refused);
        drop(forged.mailbox);
        forged.task.await.unwrap();

        let cited = seed_mailbox(6).await;
        let before = cited.mailbox.snapshot().await.unwrap().revision;
        let admitted = seed_runner(
            &cited,
            seed_script(vec![fact("Public/Places/Low Sere.md")]),
            fresh_store(),
            Arc::new(FixtureVault {
                receipts: vec![retrieved],
            }),
        );
        // It commits; it declares nobody alive, so the row does not move and
        // the outcome is the honest fixed point rather than a rejection.
        assert_eq!(admitted.step().await.unwrap(), SeedOutcome::NoProgress);
        assert_eq!(
            cited.mailbox.snapshot().await.unwrap().revision,
            before + 1,
            "the cited fact was refused"
        );
        drop(admitted);
        drop(cited.mailbox);
        cited.task.await.unwrap();
    }

    /// Spec test 10. A rejection reopens the same derived id with the kernel's
    /// complete mismatch set, and a fresh runner over the same store resumes it
    /// rather than opening a second session.
    #[tokio::test]
    async fn a_seed_session_resumes_its_checkpoint_under_the_same_derived_id() {
        let fixture = seed_mailbox(6).await;
        let store = fresh_store();
        let entry = serde_json::to_value(fixture.speak).unwrap();
        let scripted = seed_script(vec![
            // A subject standing in a handle nothing declares.
            tool_round(
                vec![
                    (
                        "declare_subject",
                        json!({
                            "handle": "digger",
                            "label": "Sere digger",
                            "kind": "person",
                            "controller": {"type": "narrative_persona"},
                            "affordances": [{"ref": "existing", "value": entry}],
                            "position": {"ref": "draft", "value": "nowhere"},
                        }),
                    ),
                    ("submit", json!({})),
                ],
                "r0",
            ),
            author_persons("r1", fixture.sere, fixture.speak, &["digger"]),
        ]);
        let first = seed_runner(
            &fixture,
            scripted.clone(),
            store.clone(),
            Arc::new(NullEvidenceSource),
        );
        assert_eq!(first.step().await.unwrap(), SeedOutcome::Rejected);
        drop(first);

        let (command_id, prompt) = {
            let stored = store.work.lock().unwrap();
            assert_eq!(stored.len(), 1, "one session, one row");
            let ControllerWork::Seed(SeedCheckpoint::SeedInFlight {
                command_id,
                last_mismatches,
                agent_prompt,
                completed,
                ..
            }) = stored.values().next().unwrap().clone()
            else {
                panic!("the rejection did not reopen the session for repair");
            };
            assert!(!last_mismatches.is_empty(), "the repair set is empty");
            assert!(completed.is_empty(), "a rejected round kept its evidence");
            (command_id, agent_prompt)
        };
        assert!(
            prompt.contains("Your previous patch was refused"),
            "{prompt}"
        );
        assert!(prompt.contains("You are seeding a world before it opens"));
        assert!(prompt.contains("holds a goal commitment"));

        let second = seed_runner(
            &fixture,
            scripted,
            store.clone(),
            Arc::new(NullEvidenceSource),
        );
        assert_eq!(second.step().await.unwrap(), SeedOutcome::Committed);
        {
            let stored = store.work.lock().unwrap();
            assert_eq!(stored.len(), 1, "the resume opened a second row");
            assert_eq!(stored.values().next().unwrap().command_id(), command_id);
        }

        drop(second);
        drop(fixture.mailbox);
        fixture.task.await.unwrap();
    }

    /// Spec test 16. A v9 row is refused at open beside the pinned v8 and v7
    /// refusals; a seed checkpoint written over another lane's row under one
    /// command id is a mode conflict; cross-lane progression is false; and a
    /// first write that is not the lane's initial stage is refused.
    #[tokio::test]
    async fn a_controller_work_row_written_before_this_pass_is_refused() {
        let directory = tempfile::tempdir().unwrap();
        let command_id = CommandId::new();
        let opportunity = fixture_opportunity(ControllerMode::OperationalAgent);
        let work = operational_in_flight(
            command_id,
            &opportunity,
            "Hold the bridge.",
            "operational-model",
            vec![],
        );
        // The version immediately before this pass, and the one before that.
        // Each is refused at open; none is migrated and none is dual-read.
        for version in ["v10", "v9"] {
            let path = directory
                .path()
                .join(format!("controller-work-{version}.cc"));
            {
                let mut store = OwnedRedbMessagePackBackingStore::new(&path).unwrap();
                store
                    .push(&CultCacheEnvelope {
                        key: store_key(command_id).unwrap(),
                        r#type: format!("controller_work.{version}"),
                        payload: rmp_serde::to_vec_named(&work).unwrap(),
                        stored_at: Utc::now().to_rfc3339(),
                        schema_id: Some(format!("ghostlight.controller_work.{version}")),
                    })
                    .unwrap();
            }
            let Err(error) = CultCacheControllerWorkStore::open(&path) else {
                panic!("a {version} row was accepted by the {CONTROLLER_WORK_ROW} store");
            };
            assert!(matches!(error, ControllerWorkStoreError::Fault { .. }));
        }

        // A real seed row, taken from a session that committed.
        let fixture = seed_mailbox(6).await;
        let store = fresh_store();
        let runner = seed_runner(
            &fixture,
            seed_script(vec![author_persons(
                "r0",
                fixture.sere,
                fixture.speak,
                &["digger"],
            )]),
            store.clone(),
            Arc::new(NullEvidenceSource),
        );
        assert_eq!(runner.step().await.unwrap(), SeedOutcome::Committed);
        drop(runner);
        let seed_row = store.work.lock().unwrap().values().next().unwrap().clone();
        let seed_command = seed_row.command_id();

        // Another lane's row under the same command id. The store is seeded
        // directly, because what is under test is the conflict on write and not
        // the other lane's own entry path.
        let other = operational_in_flight(
            seed_command,
            &opportunity,
            "Hold the bridge.",
            "operational-model",
            vec![],
        );
        let conflicted = fresh_store();
        conflicted
            .work
            .lock()
            .unwrap()
            .insert(seed_command, other.clone());
        assert!(matches!(
            conflicted.persist(&seed_row).await,
            Err(ControllerWorkStoreError::CommandModeConflict)
        ));
        assert!(!valid_controller_work_progression(&other, &seed_row));
        assert!(!valid_controller_work_progression(&seed_row, &other));

        // The row the committing session left is `ReadyToSubmit`, which is not
        // the lane's initial stage, so a store that has never seen the session
        // refuses it.
        assert!(matches!(
            seed_row,
            ControllerWork::Seed(SeedCheckpoint::ReadyToSubmit { .. })
        ));
        assert!(fresh_store().persist(&seed_row).await.is_err());

        drop(fixture.mailbox);
        fixture.task.await.unwrap();
    }

    /// Spec test 17. The whole road: a world is created with a two-root scale
    /// intent, seeded to zero deficit, approved, activated, elaborated, ticked,
    /// and then replayed from its own journal to the same state digest.
    #[tokio::test]
    async fn a_world_is_created_seeded_approved_activated_and_replays() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("world.cc");
        let owner = PrincipalId::new("seed-owner");
        let principal = crate::app_session::VerifiedPrincipalEvidence::fixture(
            "seed-owner",
            Utc::now() + chrono::Duration::hours(1),
        );
        let authenticated = AuthenticatedCaller::fixture(CallerId::Principal(owner.clone()));
        let (mailbox, task) = WorldMailbox::open(&path).unwrap();
        mailbox
            .create(
                CreateWorldIntent {
                    id: CommandId::new(),
                    title: "The Whole Road".into(),
                    human_subject_label: "The Owner".into(),
                    narrative_persona_label: None,
                    operational_agent_label: None,
                    targets: BTreeMap::from([(SubjectKind::Person, 4)]),
                    jurisdictions: vec![
                        CreateJurisdictionIntent {
                            handle: "sere".into(),
                            label: "The Low Sere".into(),
                            permille: 500,
                        },
                        CreateJurisdictionIntent {
                            handle: "gate".into(),
                            label: "The Rain Gate".into(),
                            permille: 500,
                        },
                    ],
                },
                &principal,
            )
            .await
            .unwrap();
        let genesis = mailbox.snapshot().await.unwrap();
        assert_eq!(genesis.scale_deficit.len(), 2, "one row per declared root");
        assert!(
            genesis
                .scale_deficit
                .iter()
                .all(|row| row.target == 2 && row.deficit == 2)
        );
        let root = |label: &str| {
            genesis
                .places
                .iter()
                .find(|place| place.label == label)
                .expect("the declared root")
                .id
        };
        let (sere, gate) = (root("The Low Sere"), root("The Rain Gate"));
        let speak = genesis
            .affordances
            .iter()
            .find(|entry| entry.entry.kind.0 == "speak")
            .unwrap()
            .id;

        let runner = SeedRunner::new(
            SeedPort::new(mailbox.clone(), principal.clone()),
            // One patch fills both roots. Which row the runner selects first is
            // the snapshot's order and not this test's business; a session that
            // answers both cannot pick wrong.
            seed_script(vec![author_across(
                "r0",
                speak,
                &[(sere, &["s1", "s2"]), (gate, &["g1", "g2"])],
            )]),
            Arc::new(NullEvidenceSource),
            fresh_store(),
            models().elaborator,
            None,
        );
        assert_eq!(runner.sweep(3).await.unwrap(), SeedOutcome::Clean);
        let seeded = mailbox.snapshot().await.unwrap();
        assert!(
            seeded.scale_deficit.iter().all(|row| row.deficit == 0),
            "{:?}",
            seeded.scale_deficit
        );
        for label in ["Sere s1", "Sere s2", "Sere g1", "Sere g2"] {
            assert!(
                seeded
                    .subjects
                    .iter()
                    .any(|subject| subject.label == label && subject.qualified)
            );
        }
        drop(runner);

        let mut snapshot = seeded;
        for body in [CommandBody::ApproveDraft, CommandBody::ActivateWorld] {
            mailbox
                .submit_fixture(
                    CommandEnvelope {
                        id: CommandId::new(),
                        world_id: snapshot.world_id,
                        expected_revision: snapshot.revision,
                        caller: CallerId::Principal(owner.clone()),
                        body,
                    },
                    &authenticated,
                )
                .await
                .unwrap();
            snapshot = mailbox.snapshot().await.unwrap();
        }
        assert_eq!(snapshot.phase, WorldPhase::Active);
        // The seeded goals have no counterparty who can command or litigate
        // them, which is exactly the boundary the elaborator exists to answer.
        assert!(
            !snapshot.boundaries.is_empty(),
            "the seed manufactured nothing for the elaborator"
        );

        let elaborator = ElaborationRunner::new(
            ElaborationPort::new(mailbox.clone()),
            // One text-only round per jurisdiction: the sweep reaches every
            // root the intent named plus the uncovered residual, and each ends
            // in `NoPatch` without a tool call. What is under test here is that
            // the elaborator has something to be given, not what it does with
            // it.
            seed_script(
                (0..4)
                    .map(|round| {
                        output(
                            vec![InferenceEvent::Text("nothing to add".into())],
                            &format!("e{round}"),
                        )
                    })
                    .collect(),
            ),
            Arc::new(NullEvidenceSource),
            fresh_store(),
            models().elaborator,
        );
        elaborator.sweep().await.unwrap();
        drop(elaborator);

        mailbox
            .submit_clock(CommandId::new(), TickMinutes::new(60).unwrap())
            .await
            .unwrap();
        let final_snapshot = mailbox.snapshot().await.unwrap();
        assert!(final_snapshot.now.0 >= 60);
        drop(mailbox);
        task.await.unwrap();

        // Replay: the journal alone reproduces the state, digest for digest.
        let (reopened, replay_task) = WorldMailbox::open(&path).unwrap();
        let replayed = reopened.snapshot().await.unwrap();
        assert_eq!(replayed.state_digest, final_snapshot.state_digest);
        assert_eq!(replayed.revision, final_snapshot.revision);
        assert_eq!(replayed.subjects.len(), final_snapshot.subjects.len());
        assert_eq!(replayed.scale_deficit, final_snapshot.scale_deficit);
        drop(reopened);
        replay_task.await.unwrap();
    }

    /// Spec test 2. A create naming two roots yields a world whose places
    /// include both, whose deficit carries one row per `(kind, root)`, and
    /// whose weights divide the target. The two ways to author a bad intent are
    /// refused by the resolver, in its complete mismatch set, rather than
    /// pre-checked at ingress.
    #[tokio::test]
    async fn the_intents_roots_are_declared_and_covered() {
        let directory = tempfile::tempdir().unwrap();
        let owner = PrincipalId::new("seed-owner");
        let authenticated = AuthenticatedCaller::fixture(CallerId::Principal(owner.clone()));
        let genesis = |intent: WorldScaleIntentRef, roots: &[&str]| CreateWorld {
            id: CommandId::new(),
            owner: owner.clone(),
            title: "Two Roots".into(),
            patch: WorldPatch {
                declarations: std::iter::once(Declaration::Entity(EntityDeclaration {
                    handle: DraftHandle::new("commons"),
                    label: "The Commons".into(),
                    kind: EntityKind::Place,
                    container: None,
                }))
                .chain(roots.iter().map(|handle| {
                    Declaration::Entity(EntityDeclaration {
                        handle: DraftHandle::new(*handle),
                        label: format!("The {handle}"),
                        kind: EntityKind::Place,
                        container: None,
                    })
                }))
                .chain(std::iter::once(Declaration::Subject(SubjectDeclaration {
                    handle: DraftHandle::new("first-person"),
                    label: "The Owner".into(),
                    kind: SubjectKind::Person,
                    controller: NewController::Human {
                        principal: owner.clone(),
                    },
                    affordances: kernel_speak_grant(),
                    position: Some(Ref::Draft(DraftHandle::new("commons"))),
                })))
                .collect(),
                operations: Vec::new(),
                evidence: Vec::new(),
            },
            scale_intent: intent,
        };

        let (mailbox, task) = WorldMailbox::open(directory.path().join("two-roots.cc")).unwrap();
        mailbox
            .create_fixture(
                genesis(
                    WorldScaleIntentRef {
                        targets: BTreeMap::from([(SubjectKind::Person, 10)]),
                        jurisdictions: BTreeMap::from([
                            (DraftHandle::new("sere"), 700),
                            (DraftHandle::new("gate"), 300),
                        ]),
                    },
                    &["sere", "gate"],
                ),
                &authenticated,
            )
            .await
            .unwrap();
        let snapshot = mailbox.snapshot().await.unwrap();
        for label in ["The sere", "The gate"] {
            assert!(snapshot.places.iter().any(|place| place.label == label));
        }
        assert_eq!(snapshot.scale_deficit.len(), 2);
        let mut targets: Vec<u32> = snapshot
            .scale_deficit
            .iter()
            .map(|row| {
                assert_eq!(row.kind, SubjectKind::Person);
                assert_eq!(row.deficit, row.target, "nothing is alive at genesis");
                row.target
            })
            .collect();
        targets.sort_unstable();
        assert_eq!(targets, vec![3, 7], "the weights did not divide the target");
        drop(mailbox);
        task.await.unwrap();

        // A root the genesis patch does not declare.
        let (mailbox, task) = WorldMailbox::open(directory.path().join("unknown.cc")).unwrap();
        let refused = mailbox
            .create_fixture(
                genesis(
                    WorldScaleIntentRef {
                        targets: BTreeMap::from([(SubjectKind::Person, 10)]),
                        jurisdictions: BTreeMap::from([(DraftHandle::new("nowhere"), 1000)]),
                    },
                    &["sere"],
                ),
                &authenticated,
            )
            .await;
        let Err(MailboxError::Kernel(KernelError::PatchRejected(mismatches))) = refused else {
            panic!("an undeclared jurisdiction root was admitted");
        };
        assert!(
            mismatches.iter().any(|mismatch| matches!(
                mismatch,
                crate::world::Mismatch::UnknownJurisdictionRoot { .. }
            )),
            "{mismatches:?}"
        );
        drop(mailbox);
        task.await.unwrap();

        // Weights distribute the target and never raise it.
        let (mailbox, task) = WorldMailbox::open(directory.path().join("heavy.cc")).unwrap();
        let refused = mailbox
            .create_fixture(
                genesis(
                    WorldScaleIntentRef {
                        targets: BTreeMap::from([(SubjectKind::Person, 10)]),
                        jurisdictions: BTreeMap::from([
                            (DraftHandle::new("sere"), 700),
                            (DraftHandle::new("gate"), 700),
                        ]),
                    },
                    &["sere", "gate"],
                ),
                &authenticated,
            )
            .await;
        let Err(MailboxError::Kernel(KernelError::PatchRejected(mismatches))) = refused else {
            panic!("permille weights over the whole were admitted");
        };
        assert!(
            mismatches.iter().any(|mismatch| matches!(
                mismatch,
                crate::world::Mismatch::ScaleWeightsExceedWhole
            )),
            "{mismatches:?}"
        );
        drop(mailbox);
        task.await.unwrap();
    }

    /// Spec test 3. The world's first person belongs to no jurisdiction's
    /// population target: it stands in the commons, which no declared root
    /// covers, so it counts in `Uncovered` and reduces nothing.
    #[tokio::test]
    async fn the_owner_subject_lands_uncovered() {
        let fixture = seed_mailbox(6).await;
        let before = fixture.mailbox.snapshot().await.unwrap();
        let owner_subject = before
            .subjects
            .iter()
            .find(|subject| subject.label == "The Owner")
            .expect("the first person");
        assert!(
            !owner_subject.qualified,
            "the first person holds no goal and cannot count yet"
        );
        assert_eq!(before.scale_deficit.len(), 1, "one root, one row");

        // Give it a goal through the same lane a seed uses, and it appears in
        // the residual rather than in the root's row.
        let runner = seed_runner(
            &fixture,
            seed_script(vec![tool_round(
                vec![
                    (
                        "create_commitment",
                        json!({
                            "subject": {"ref": "existing", "value": serde_json::to_value(owner_subject.id).unwrap()},
                            "counterparty": null,
                            "kind": "goal",
                            "due": 600,
                            "period": null,
                            "checks": [],
                        }),
                    ),
                    ("submit", json!({})),
                ],
                "r0",
            )]),
            fresh_store(),
            Arc::new(NullEvidenceSource),
        );
        assert_eq!(runner.step().await.unwrap(), SeedOutcome::NoProgress);
        let after = fixture.mailbox.snapshot().await.unwrap();
        let residual = after
            .scale_deficit
            .iter()
            .find(|row| row.jurisdiction == JurisdictionKey::Uncovered)
            .expect("the uncovered residual");
        assert_eq!(residual.qualified, 1);
        assert_eq!(residual.target, 0, "the first person raised a target");
        assert_eq!(
            root_deficit(&after, fixture.sere),
            6,
            "the first person reduced a root's shortfall"
        );

        drop(runner);
        drop(fixture.mailbox);
        fixture.task.await.unwrap();
    }

    // ---- Soul: the seed lane under falsification ------------------------

    /// The lane's claim is that Active is refused twice: once by the runner's
    /// phase gate and once by `SeedPort::submit_seed` itself, "if that check is
    /// ever removed [from the runner]". An operations-only patch is the case
    /// that used to falsify the second gate: `require_answer`'s
    /// `(Active, None) if declares` arm never fires for it, because `declares`
    /// is `!(declarations.is_empty() && evidence.is_empty())` and operations
    /// are not in it — that is correct, not a bug, because the owner's hand is
    /// legitimate in Active. `submit_seed` now owns its own Draft-only phase
    /// check, taken from a fresh snapshot at submission time, so the second
    /// gate no longer depends on what the patch happens to declare.
    ///
    /// This is not an authority escalation: the owner may write in Active. It
    /// falsifies the old defence-in-depth claim about `require_answer`, not
    /// the ownership one.
    #[tokio::test]
    async fn soul_the_seed_lane_admits_an_operations_only_patch_in_active() {
        let fixture = seed_mailbox(6).await;
        let authenticated =
            AuthenticatedCaller::fixture(CallerId::Principal(fixture.owner.clone()));
        let mut snapshot = fixture.mailbox.snapshot().await.unwrap();
        for body in [CommandBody::ApproveDraft, CommandBody::ActivateWorld] {
            fixture
                .mailbox
                .submit_fixture(
                    CommandEnvelope {
                        id: CommandId::new(),
                        world_id: snapshot.world_id,
                        expected_revision: snapshot.revision,
                        caller: CallerId::Principal(fixture.owner.clone()),
                        body,
                    },
                    &authenticated,
                )
                .await
                .unwrap();
            snapshot = fixture.mailbox.snapshot().await.unwrap();
        }
        assert_eq!(snapshot.phase, WorldPhase::Active);
        let owner_subject = snapshot
            .subjects
            .iter()
            .find(|subject| subject.label == "The Owner")
            .expect("the first person")
            .id;

        let admitted = seed_port(&fixture)
            .submit_seed(
                CommandId::new(),
                snapshot.world_id,
                WorldPatch {
                    declarations: Vec::new(),
                    operations: vec![crate::world::patch::ComponentOp::CreateCommitment {
                        subject: Ref::Existing(owner_subject),
                        counterparty: None,
                        kind: CommitmentKind::Goal,
                        due: crate::world::FictionalMinutes(600),
                        period: None,
                        checks: Vec::new(),
                    }],
                    evidence: Vec::new(),
                },
            )
            .await;
        let after = fixture.mailbox.snapshot().await.unwrap();
        assert!(
            admitted.is_err(),
            "an Active seed submission committed: revision {} -> {}, owner qualified {}",
            snapshot.revision,
            after.revision,
            after
                .subjects
                .iter()
                .any(|subject| subject.id == owner_subject && subject.qualified)
        );

        drop(fixture.mailbox);
        fixture.task.await.unwrap();
    }

    /// Soul. What the deleted phase clause used to be blamed for, checked
    /// directly: `require_answer` still refuses every Draft answer, and still
    /// demands one from every Active patch that declares — that is exercised
    /// here against the general owner path, `submit_fixture`, which carries no
    /// Draft-only gate of its own. `SeedPort::submit_seed` meets a stricter
    /// bar than `require_answer` alone: it never reaches Active at all, because
    /// it takes its own snapshot and refuses outright once the phase has moved,
    /// whatever the patch would or would not have declared.
    #[tokio::test]
    async fn soul_require_answer_still_owns_the_phase_rule_the_clause_did_not() {
        let fixture = seed_mailbox(6).await;
        let authenticated =
            AuthenticatedCaller::fixture(CallerId::Principal(fixture.owner.clone()));
        let mut snapshot = fixture.mailbox.snapshot().await.unwrap();
        let shed = || WorldPatch {
            declarations: vec![Declaration::Entity(EntityDeclaration {
                handle: DraftHandle::new("shed"),
                label: "A Shed".into(),
                kind: EntityKind::Place,
                container: None,
            })],
            operations: Vec::new(),
            evidence: Vec::new(),
        };

        // Draft refuses an answer outright, whatever the answer names.
        let refused = fixture
            .mailbox
            .submit_fixture(
                CommandEnvelope {
                    id: CommandId::new(),
                    world_id: snapshot.world_id,
                    expected_revision: snapshot.revision,
                    caller: CallerId::Principal(fixture.owner.clone()),
                    body: CommandBody::AdmitPatch {
                        answers: Some(crate::world::PatchAnswer::Deficit(
                            JurisdictionKey::PlaceSubtree(fixture.sere),
                        )),
                        patch: shed(),
                    },
                },
                &authenticated,
            )
            .await;
        assert!(
            matches!(
                refused,
                Err(MailboxError::Kernel(KernelError::AnswerNotDerived))
            ),
            "{refused:?}"
        );
        assert_eq!(
            fixture.mailbox.snapshot().await.unwrap().revision,
            snapshot.revision
        );

        for body in [CommandBody::ApproveDraft, CommandBody::ActivateWorld] {
            fixture
                .mailbox
                .submit_fixture(
                    CommandEnvelope {
                        id: CommandId::new(),
                        world_id: snapshot.world_id,
                        expected_revision: snapshot.revision,
                        caller: CallerId::Principal(fixture.owner.clone()),
                        body,
                    },
                    &authenticated,
                )
                .await
                .unwrap();
            snapshot = fixture.mailbox.snapshot().await.unwrap();
        }

        // The general owner path still meets `require_answer`'s Active gate: a
        // declaring patch with no answer is refused whether or not a phase
        // clause exists to name the phase.
        let refused = fixture
            .mailbox
            .submit_fixture(
                CommandEnvelope {
                    id: CommandId::new(),
                    world_id: snapshot.world_id,
                    expected_revision: snapshot.revision,
                    caller: CallerId::Principal(fixture.owner.clone()),
                    body: CommandBody::AdmitPatch {
                        answers: None,
                        patch: shed(),
                    },
                },
                &authenticated,
            )
            .await;
        assert!(
            matches!(
                refused,
                Err(MailboxError::Kernel(KernelError::AnswerRequired))
            ),
            "{refused:?}"
        );
        assert_eq!(
            fixture.mailbox.snapshot().await.unwrap().revision,
            snapshot.revision
        );

        // The seed lane's own port refuses the same submission earlier still:
        // it never gets far enough to ask what the patch declares.
        let refused = seed_port(&fixture)
            .submit_seed(CommandId::new(), snapshot.world_id, shed())
            .await;
        assert!(
            matches!(
                refused,
                Err(MailboxError::Kernel(KernelError::WrongPhase {
                    expected: WorldPhase::Draft,
                    actual: WorldPhase::Active,
                }))
            ),
            "{refused:?}"
        );

        drop(fixture.mailbox);
        fixture.task.await.unwrap();
    }

    /// Soul, revised on the road. The seed lane's round loop is bounded by
    /// `SEED_ROUND_BUDGET`; a model that keeps declaring and never submits
    /// stops there and the draft as authored is submitted rather than
    /// discarded, because what it authored is a patch the resolver decides.
    #[tokio::test]
    async fn soul_a_session_that_never_submits_is_bounded_and_its_draft_is_submitted() {
        // `elaboration::SEED_ROUND_BUDGET`, file-private to the lane that owns
        // it, restated so raising it fails this bound loudly.
        const SEED_ROUND_BUDGET: usize = 24;

        let fixture = seed_mailbox(6).await;
        let genesis = fixture.mailbox.snapshot().await.unwrap().revision;
        let store = fresh_store();
        let scripted = seed_script(
            (0..SEED_ROUND_BUDGET + 4)
                .map(|round| {
                    tool_round(
                        vec![(
                            "declare_place",
                            json!({
                                "handle": format!("shed{round}"),
                                "label": "A Shed",
                                "container": null
                            }),
                        )],
                        &format!("r{round}"),
                    )
                })
                .collect(),
        );
        let runner = seed_runner(
            &fixture,
            scripted.clone(),
            store.clone(),
            Arc::new(NullEvidenceSource),
        );
        // Sheds qualify nobody, so the outcome may report no progress on the
        // row; the revision and the places prove the draft was submitted.
        let outcome = runner.step().await.unwrap();
        assert!(
            matches!(outcome, SeedOutcome::Committed | SeedOutcome::NoProgress),
            "the draft as authored was not submitted: {outcome:?}"
        );
        assert_eq!(
            scripted.seen.lock().unwrap().len(),
            SEED_ROUND_BUDGET,
            "the round loop is not bounded by the seed budget"
        );
        let after = fixture.mailbox.snapshot().await.unwrap();
        assert_eq!(
            after.revision,
            genesis + 1,
            "the authored draft was not submitted once"
        );
        assert_eq!(
            after
                .places
                .iter()
                .filter(|place| place.label == "A Shed")
                .count(),
            SEED_ROUND_BUDGET,
            "every declaration authored before the budget ended must land"
        );

        drop(runner);
        drop(fixture.mailbox);
        fixture.task.await.unwrap();
    }

    /// Soul. A session that only records gaps has authored nothing; the budget
    /// ends it as `NoPatch`, the world does not move, and a second sweep over
    /// the same store finds that row and spends no round.
    #[tokio::test]
    async fn soul_a_session_that_authors_nothing_is_a_fixed_point() {
        const SEED_ROUND_BUDGET: usize = 24;

        let fixture = seed_mailbox(6).await;
        let genesis = fixture.mailbox.snapshot().await.unwrap().revision;
        let store = fresh_store();
        let scripted = seed_script(
            (0..SEED_ROUND_BUDGET + 4)
                .map(|round| {
                    tool_round(
                        vec![(
                            "record_gap",
                            json!({"detail": format!("nothing yet {round}")}),
                        )],
                        &format!("r{round}"),
                    )
                })
                .collect(),
        );
        let runner = seed_runner(
            &fixture,
            scripted.clone(),
            store.clone(),
            Arc::new(NullEvidenceSource),
        );
        assert_eq!(runner.step().await.unwrap(), SeedOutcome::NoPatch);
        assert_eq!(scripted.seen.lock().unwrap().len(), SEED_ROUND_BUDGET);
        assert_eq!(fixture.mailbox.snapshot().await.unwrap().revision, genesis);
        let resumed = seed_runner(
            &fixture,
            scripted.clone(),
            store.clone(),
            Arc::new(NullEvidenceSource),
        );
        assert_eq!(resumed.sweep(4).await.unwrap(), SeedOutcome::NoPatch);
        assert_eq!(
            scripted.seen.lock().unwrap().len(),
            SEED_ROUND_BUDGET,
            "a NoPatch row was reopened against the endpoint"
        );
        assert_eq!(store.work.lock().unwrap().len(), 1, "one row per session");

        drop(runner);
        drop(resumed);
        drop(fixture.mailbox);
        fixture.task.await.unwrap();
    }

    /// Soul. The derived id is the whole idempotency story, so it has to hold
    /// at the ledger and not only at the store: the same id submitted twice
    /// commits once.
    #[tokio::test]
    async fn soul_one_derived_seed_id_commits_once() {
        let fixture = seed_mailbox(6).await;
        let port = seed_port(&fixture);
        let snapshot = port.snapshot().await.unwrap();
        let command_id = CommandId::new();
        let shed = || WorldPatch {
            declarations: vec![Declaration::Entity(EntityDeclaration {
                handle: DraftHandle::new("shed"),
                label: "A Shed".into(),
                kind: EntityKind::Place,
                container: None,
            })],
            operations: Vec::new(),
            evidence: Vec::new(),
        };
        let first = port
            .submit_seed(command_id, snapshot.world_id, shed())
            .await
            .unwrap();
        let after = port.snapshot().await.unwrap();
        assert_eq!(after.revision, snapshot.revision + 1);
        let again = port
            .submit_seed(command_id, snapshot.world_id, shed())
            .await
            .unwrap();
        let (SubmitReceipt::Applied(landed), SubmitReceipt::AlreadyApplied(replayed)) =
            (&first, &again)
        else {
            panic!("a replayed seed id was not recognised: {first:?} then {again:?}");
        };
        assert_eq!(
            landed, replayed,
            "a replayed seed id returned a different commit"
        );
        assert_eq!(
            port.snapshot().await.unwrap().revision,
            after.revision,
            "one derived id committed twice"
        );

        drop(port);
        drop(fixture.mailbox);
        fixture.task.await.unwrap();
    }

    /// Soul. Two rows in one Draft world, answered in sequence, leave two store
    /// rows under two ids. The ancestry in the key is what makes each landed
    /// patch open a fresh session rather than resubmitting one id.
    #[tokio::test]
    async fn soul_two_seed_sessions_in_one_world_hold_two_ids() {
        let fixture = seed_mailbox(2).await;
        let store = fresh_store();
        let scripted = seed_script(vec![
            author_persons("r0", fixture.sere, fixture.speak, &["a1"]),
            author_persons("r1", fixture.sere, fixture.speak, &["a2"]),
        ]);
        let runner = seed_runner(
            &fixture,
            scripted,
            store.clone(),
            Arc::new(NullEvidenceSource),
        );
        assert_eq!(runner.sweep(4).await.unwrap(), SeedOutcome::Clean);
        let ids: BTreeSet<CommandId> = store.work.lock().unwrap().keys().copied().collect();
        assert_eq!(ids.len(), 2, "two landed sessions shared one derived id");

        drop(runner);
        drop(fixture.mailbox);
        fixture.task.await.unwrap();
    }

    /// Soul. `valid_seed_progression` is what stops a resumed session from
    /// rewriting its own history. The forward extension is allowed; a repair
    /// with an empty mismatch set, a rewritten row, and every transition out of
    /// the terminal stage are not.
    #[tokio::test]
    async fn soul_valid_seed_progression_refuses_every_backward_transition() {
        let fixture = seed_mailbox(6).await;
        let store = fresh_store();
        let runner = seed_runner(
            &fixture,
            seed_script(vec![tool_round(
                vec![
                    (
                        "declare_subject",
                        json!({
                            "handle": "digger",
                            "label": "Sere digger",
                            "kind": "person",
                            "controller": {"type": "narrative_persona"},
                            "affordances": [{
                                "ref": "existing",
                                "value": serde_json::to_value(fixture.speak).unwrap()
                            }],
                            "position": {"ref": "draft", "value": "nowhere"},
                        }),
                    ),
                    ("submit", json!({})),
                ],
                "r0",
            )]),
            store.clone(),
            Arc::new(NullEvidenceSource),
        );
        assert_eq!(runner.step().await.unwrap(), SeedOutcome::Rejected);
        drop(runner);

        let stored = store.work.lock().unwrap().values().next().unwrap().clone();
        let ControllerWork::Seed(repair) = stored else {
            panic!("the rejection left no seed row");
        };
        let SeedCheckpoint::SeedInFlight {
            command_id,
            session,
            agent_prompt,
            last_mismatches,
            completed,
            invocation,
        } = repair.clone()
        else {
            panic!("a rejection must reopen the session in flight");
        };
        assert!(!last_mismatches.is_empty());
        assert!(completed.is_empty());

        let ready = SeedCheckpoint::ReadyToSubmit {
            command_id,
            session: session.clone(),
            agent_prompt: agent_prompt.clone(),
            last_mismatches: last_mismatches.clone(),
            completed: completed.clone(),
        };
        let unrepaired = SeedCheckpoint::SeedInFlight {
            command_id,
            session: session.clone(),
            agent_prompt: agent_prompt.clone(),
            last_mismatches: Vec::new(),
            completed: completed.clone(),
            invocation: invocation.clone(),
        };
        let no_patch = SeedCheckpoint::NoPatch {
            command_id,
            session: session.clone(),
            agent_prompt: agent_prompt.clone(),
            completed: completed.clone(),
            gaps: Vec::new(),
        };
        let other_row = SeedCheckpoint::SeedInFlight {
            command_id,
            session: {
                let mut moved = session.clone();
                moved.target += 1;
                moved
            },
            agent_prompt,
            last_mismatches,
            completed,
            invocation,
        };

        // A submitted session reopens only with a repair set.
        assert!(!valid_seed_progression(&ready, &unrepaired));
        // Nothing follows the terminal stage.
        assert!(!valid_seed_progression(&no_patch, &repair));
        assert!(!valid_seed_progression(&no_patch, &ready));
        // The row a session bound to is not rewritable.
        assert!(!valid_seed_progression(&repair, &other_row));
        // And the forward move the lane actually needs is still allowed.
        assert!(valid_seed_progression(&repair, &ready));

        drop(fixture.mailbox);
        fixture.task.await.unwrap();
    }

    /// It is the obligations that hand the Active elaborator a boundary, never
    /// the goals: `derive_boundaries` skips a commitment with no counterparty,
    /// and a goal cannot carry one (`patch::resolve_patch` refuses a `Goal`
    /// declared with one). A world seeded to full strength out of goals alone
    /// therefore activates with nothing for the elaborator to answer — a
    /// structural fact this session's script proves directly, by declaring
    /// only goals and no obligations. `SEED_INSTRUCTIONS` now requires an
    /// obligation with a counterparty from every subject a compliant session
    /// authors; this test's point is that nothing at the kernel layer stops a
    /// session that ignores that instruction from producing exactly this
    /// boundary-less world, so activation itself must not be the thing
    /// guaranteeing a boundary exists.
    #[tokio::test]
    async fn soul_a_world_seeded_out_of_goals_alone_hands_the_elaborator_nothing() {
        let fixture = seed_mailbox(2).await;
        let entry = serde_json::to_value(fixture.speak).unwrap();
        let place = serde_json::to_value(fixture.sere).unwrap();
        let mut calls: Vec<(&str, Value)> = Vec::new();
        for handle in ["a1", "a2"] {
            calls.push((
                "declare_subject",
                json!({
                    "handle": handle,
                    "label": format!("Sere {handle}"),
                    "kind": "person",
                    "controller": {"type": "narrative_persona"},
                    "affordances": [{"ref": "existing", "value": entry}],
                    "position": {"ref": "existing", "value": place},
                }),
            ));
            calls.push((
                "create_commitment",
                json!({
                    "subject": {"ref": "draft", "value": handle},
                    "counterparty": null,
                    "kind": "goal",
                    "due": 600,
                    "period": null,
                    "checks": [],
                }),
            ));
        }
        calls.push(("submit", json!({})));
        let runner = seed_runner(
            &fixture,
            seed_script(vec![tool_round(calls, "r0")]),
            fresh_store(),
            Arc::new(NullEvidenceSource),
        );
        assert_eq!(runner.sweep(4).await.unwrap(), SeedOutcome::Clean);
        drop(runner);

        let authenticated =
            AuthenticatedCaller::fixture(CallerId::Principal(fixture.owner.clone()));
        let mut snapshot = fixture.mailbox.snapshot().await.unwrap();
        assert!(snapshot.scale_deficit.iter().all(|row| row.deficit == 0));
        for body in [CommandBody::ApproveDraft, CommandBody::ActivateWorld] {
            fixture
                .mailbox
                .submit_fixture(
                    CommandEnvelope {
                        id: CommandId::new(),
                        world_id: snapshot.world_id,
                        expected_revision: snapshot.revision,
                        caller: CallerId::Principal(fixture.owner.clone()),
                        body,
                    },
                    &authenticated,
                )
                .await
                .unwrap();
            snapshot = fixture.mailbox.snapshot().await.unwrap();
        }
        assert_eq!(snapshot.phase, WorldPhase::Active);
        assert!(
            snapshot.boundaries.is_empty(),
            "a goals-only seed handed the elaborator a boundary it derives no boundary from: {:?}",
            snapshot.boundaries
        );

        drop(fixture.mailbox);
        fixture.task.await.unwrap();
    }

    /// Soul. The custody discriminator has to see the new lane, or a seed row
    /// is a row nobody counts.
    #[tokio::test]
    async fn soul_the_custody_probe_counts_a_seed_row() {
        let fixture = seed_mailbox(6).await;
        let store = fresh_store();
        assert_eq!(
            store.custody_probe().await.unwrap(),
            ControllerWorkCustody::Owned {
                narrative_commands: 0,
                operational_commands: 0,
                elaboration_commands: 0,
                seed_commands: 0,
            }
        );
        let runner = seed_runner(
            &fixture,
            seed_script(vec![author_persons(
                "r0",
                fixture.sere,
                fixture.speak,
                &["digger"],
            )]),
            store.clone(),
            Arc::new(NullEvidenceSource),
        );
        assert_eq!(runner.step().await.unwrap(), SeedOutcome::Committed);
        assert_eq!(
            store.custody_probe().await.unwrap(),
            ControllerWorkCustody::Owned {
                narrative_commands: 0,
                operational_commands: 0,
                elaboration_commands: 0,
                seed_commands: 1,
            }
        );

        drop(runner);
        drop(fixture.mailbox);
        fixture.task.await.unwrap();
    }

    // ---------------------------------------------------------------- step 9
    use crate::world::{ControllerAssignment, ScopePreimage, digest as world_digest};
    use ghostlight_persona_projection::PersonaTurnIntegrityError;
    use std::sync::atomic::AtomicUsize;

    // The interruption pass. A scope digest that moved between a subject's
    // prose and its commit is one event with one detector — the kernel — and
    // one handler, which renews the turn's binding without re-running the
    // Persona and lowers the same prose a second time.

    /// What lands on the world in the window between a turn's prose and its
    /// commit. Both shapes move the acting subject's own components; only the
    /// first is something it may perceive as having an author.
    enum MidTurnCommit {
        /// A co-located neighbour speaks. `fan_out` gives the fact to the actor,
        /// so its `knows` grows and its digest moves.
        Speech { speaker: SubjectId, text: String },
        /// An owner patch creates a commitment on the actor. Its `commitments`
        /// move and nothing names a mover anywhere in the components.
        Commitment { subject: SubjectId },
        /// An owner patch carrying arbitrary component operations. One arm for
        /// every remaining un-authored cause: a transfer out of a neighbour's
        /// custody, a route closing under the actor's feet, a grant revoked out
        /// from under it, a commitment that names a counterparty.
        Ops(Vec<crate::world::patch::ComponentOp>),
        /// An owner patch witnessing a pre-declared fact over `place`. Unlike a
        /// neighbour's speech, this names no speaker and writes no `spoken_at`
        /// row: `KnowledgeSource::Witnessed` is not `Told`, so
        /// `overheard_since` finds nothing and the section renders only the
        /// anonymous `knows` line.
        Witness { fact: EntityId, place: EntityId },
    }

    async fn apply_mid_turn(mailbox: &WorldMailbox, commit: &MidTurnCommit) {
        let snapshot = mailbox.snapshot().await.unwrap();
        match commit {
            MidTurnCommit::Speech { speaker, text } => {
                let opportunity = snapshot
                    .opportunities
                    .iter()
                    .find(|entry| entry.scope.subject_id == *speaker)
                    .expect("the speaker has a live opportunity")
                    .clone();
                let entry = snapshot
                    .affordances
                    .iter()
                    .find(|entry| {
                        entry.entry.kind.0 == SPEAK_KIND
                            && opportunity.affordance_ids.contains(&entry.id)
                    })
                    .expect("the speaker was granted speech");
                mailbox
                    .submit_controller(
                        CommandId::new(),
                        &opportunity,
                        DecisionInvocation {
                            affordance: entry.id,
                            bindings: Vec::new(),
                            proposed: Vec::new(),
                            speech: Some(Statement::new(text.clone()).unwrap()),
                        },
                    )
                    .await
                    .expect("the mid-turn speech committed");
            }
            MidTurnCommit::Commitment { subject } => {
                let owner = PrincipalId::new("owner");
                let authenticated =
                    AuthenticatedCaller::fixture(CallerId::Principal(owner.clone()));
                mailbox
                    .submit_fixture(
                        CommandEnvelope {
                            id: CommandId::new(),
                            world_id: snapshot.world_id,
                            expected_revision: snapshot.revision,
                            caller: CallerId::Principal(owner),
                            body: CommandBody::AdmitPatch {
                                answers: None,
                                patch: WorldPatch {
                                    declarations: Vec::new(),
                                    operations: vec![
                                        crate::world::patch::ComponentOp::CreateCommitment {
                                            subject: Ref::Existing(*subject),
                                            counterparty: None,
                                            kind: CommitmentKind::Goal,
                                            due: crate::world::FictionalMinutes(600),
                                            period: None,
                                            checks: Vec::new(),
                                        },
                                    ],
                                    evidence: Vec::new(),
                                },
                            },
                        },
                        &authenticated,
                    )
                    .await
                    .expect("the mid-turn patch committed");
            }
            MidTurnCommit::Ops(operations) => {
                let owner = PrincipalId::new("owner");
                let authenticated =
                    AuthenticatedCaller::fixture(CallerId::Principal(owner.clone()));
                mailbox
                    .submit_fixture(
                        CommandEnvelope {
                            id: CommandId::new(),
                            world_id: snapshot.world_id,
                            expected_revision: snapshot.revision,
                            caller: CallerId::Principal(owner),
                            body: CommandBody::AdmitPatch {
                                answers: None,
                                patch: WorldPatch {
                                    declarations: Vec::new(),
                                    operations: operations.clone(),
                                    evidence: Vec::new(),
                                },
                            },
                        },
                        &authenticated,
                    )
                    .await
                    .expect("the mid-turn operations committed");
            }
            MidTurnCommit::Witness { fact, place } => {
                let owner = PrincipalId::new("owner");
                let authenticated =
                    AuthenticatedCaller::fixture(CallerId::Principal(owner.clone()));
                mailbox
                    .submit_fixture(
                        CommandEnvelope {
                            id: CommandId::new(),
                            world_id: snapshot.world_id,
                            expected_revision: snapshot.revision,
                            caller: CallerId::Principal(owner),
                            body: CommandBody::AdmitPatch {
                                answers: None,
                                patch: WorldPatch {
                                    declarations: Vec::new(),
                                    operations: vec![crate::world::patch::ComponentOp::Witness {
                                        fact: Ref::Existing(*fact),
                                        place: Ref::Existing(*place),
                                        confidence: Confidence::Certain,
                                    }],
                                    evidence: Vec::new(),
                                },
                            },
                        },
                        &authenticated,
                    )
                    .await
                    .expect("the mid-turn witness committed");
            }
        }
    }

    /// An inference port that lands a real commit on the world at chosen call
    /// indexes. The interruption a test produces is therefore the kernel's own
    /// refusal over real committed state, not a hand-built digest.
    struct InterruptingPort {
        mailbox: WorldMailbox,
        outputs: Mutex<Vec<Result<InferenceOutput, InferenceFault>>>,
        calls: Arc<AtomicUsize>,
        /// `(call index, what to commit)`, applied just before that call
        /// returns.
        commits: Vec<(usize, MidTurnCommit)>,
        /// Every prompt this port was asked to run, in order.
        seen: Arc<Mutex<Vec<String>>>,
    }

    #[async_trait]
    impl InferencePort for InterruptingPort {
        fn prepare(&self, request: InferenceRequest) -> Result<PreparedInference, InferenceFault> {
            fixture_prepared(request)
        }

        async fn infer(
            &self,
            request: PreparedInference,
        ) -> Result<InferenceOutput, InferenceFault> {
            let index = self.calls.fetch_add(1, Ordering::SeqCst);
            self.seen
                .lock()
                .unwrap()
                .push(serde_json::to_string(&request.invocation.request.input).unwrap());
            for (at, commit) in &self.commits {
                if *at == index {
                    apply_mid_turn(&self.mailbox, commit).await;
                }
            }
            let mut outputs = self.outputs.lock().unwrap();
            if outputs.is_empty() {
                // A round the fixture did not script leaves the run pending
                // against its persisted row, which is what a test that inspects
                // that row wants.
                return Err(InferenceFault::retryable("the fixture script ran out"));
            }
            outputs.remove(0)
        }
    }

    /// The first line of the interruption section. `build_interpreter_prompt`
    /// names `Interrupted:` unconditionally in its instructions, so a test that
    /// split on the bare word would find the instruction and not the section.
    const SECTION_HEADER: &str =
        "Interrupted: the world moved after this turn's prose was written.";

    /// The interruption section of a prompt, or `None` when it carries none.
    fn interruption_section_of(prompt: &str) -> Option<String> {
        prompt.split(SECTION_HEADER).nth(1).map(str::to_owned)
    }

    /// The Interpreter's whole conversation for one round: capture the quoted
    /// span, then finish.
    fn speak_span(
        source: &str,
        speech: &str,
        receipt: &str,
    ) -> Result<InferenceOutput, InferenceFault> {
        let start = source
            .find(speech)
            .expect("the fixture prose quotes itself");
        output(
            vec![
                InferenceEvent::ToolCall {
                    call_id: format!("call_speak_{receipt}"),
                    name: INTERPRETER_SPEAK_TOOL.into(),
                    arguments: json!({
                        "source_start_byte": start,
                        "source_end_byte": start + speech.len(),
                    })
                    .to_string(),
                },
                InferenceEvent::ToolCall {
                    call_id: format!("call_finish_{receipt}"),
                    name: FINISH_INTERPRETATION_TOOL.into(),
                    arguments: "{}".into(),
                },
            ],
            receipt,
        )
    }

    struct InterruptedTurn {
        runner: ControllerRunner,
        store: Arc<RecordingWorkStore>,
        calls: Arc<AtomicUsize>,
        seen: Arc<Mutex<Vec<String>>>,
    }

    /// One narrative turn whose prose is `source` and whose Interpreter quotes
    /// `speech` on every round, with `commits` landing on the world at the given
    /// inference indexes: 0 is the Projector, 1 the Persona, 2 the first
    /// Interpreter round, 3 the re-lowered one.
    fn interrupted_turn(
        mailbox: &WorldMailbox,
        source: &str,
        speech: &str,
        rounds: usize,
        commits: Vec<(usize, MidTurnCommit)>,
    ) -> InterruptedTurn {
        let mut outputs = vec![
            output(
                vec![InferenceEvent::Text("The room holds its breath.".into())],
                "projector",
            ),
            output(vec![InferenceEvent::Text(source.into())], "persona"),
        ];
        for round in 0..rounds {
            outputs.push(speak_span(source, speech, &format!("interpreter-{round}")));
        }
        let calls = Arc::new(AtomicUsize::new(0));
        let seen = Arc::new(Mutex::new(Vec::new()));
        let port = Arc::new(InterruptingPort {
            mailbox: mailbox.clone(),
            outputs: Mutex::new(outputs),
            calls: calls.clone(),
            commits,
            seen: seen.clone(),
        });
        let store = Arc::new(RecordingWorkStore {
            persisted: Arc::new(AtomicBool::new(true)),
            work: Mutex::new(BTreeMap::new()),
        });
        InterruptedTurn {
            runner: ControllerRunner::open(mailbox.clone(), port, store.clone(), models())
                .expect("the fixture ports open"),
            store,
            calls,
            seen,
        }
    }

    fn narrative_row(store: &RecordingWorkStore) -> NarrativeCheckpoint {
        let stored = store.work.lock().unwrap();
        assert_eq!(stored.len(), 1, "one turn, one row");
        let ControllerWork::Narrative(checkpoint) = stored.values().next().unwrap().clone() else {
            panic!("the narrative turn left another lane's row")
        };
        checkpoint
    }

    fn row_scope_digest(store: &RecordingWorkStore) -> String {
        match narrative_row(store) {
            NarrativeCheckpoint::ReadyToSubmit { opportunity, .. }
            | NarrativeCheckpoint::NoProposal { opportunity, .. }
            | NarrativeCheckpoint::InterpreterInFlight { opportunity, .. } => {
                opportunity.scope_digest.as_str().to_owned()
            }
            _ => panic!("the turn's row never reached the Interpreter"),
        }
    }

    /// The acting subject and the opportunity the driver would bind for it.
    async fn narrative_opportunity(mailbox: &WorldMailbox) -> (SubjectId, DecisionOpportunity) {
        let snapshot = mailbox.snapshot().await.unwrap();
        let subject = snapshot
            .subjects
            .iter()
            .find(|subject| subject.controller_mode == Some(ControllerMode::NarrativePersona))
            .expect("a narrative subject")
            .id;
        let opportunity = snapshot
            .opportunities
            .iter()
            .find(|entry| entry.scope.subject_id == subject)
            .expect("a live opportunity")
            .clone();
        (subject, opportunity)
    }

    /// Tests 1 and 4. A co-located speaker commits after the Persona turn is
    /// recorded and before the subject's submit. The prose is lowered exactly
    /// once more, the section names the speaker and the statement, the renewed
    /// binding carries the one it replaced, and the act commits against the
    /// fresh digest.
    #[tokio::test]
    async fn a_neighbours_speech_between_the_turn_and_submit_is_re_lowered_once() {
        let (_directory, mailbox, task) = active_cell_mailbox(vec![
            NewController::NarrativePersona,
            NewController::OperationalAgent,
        ])
        .await;
        let (actor, opportunity) = narrative_opportunity(&mailbox).await;
        let neighbour = mailbox
            .snapshot()
            .await
            .unwrap()
            .subjects
            .iter()
            .find(|subject| subject.id != actor)
            .expect("a neighbour")
            .id;

        let source = "I say, \"The western brace is giving way.\"";
        let speech = "The western brace is giving way.";
        let turn = interrupted_turn(
            &mailbox,
            source,
            speech,
            2,
            vec![(
                2,
                MidTurnCommit::Speech {
                    speaker: neighbour,
                    text: "The tollhouse ledger is short.".into(),
                },
            )],
        );
        let run = turn
            .runner
            .run_narrative(CommandId::new(), &opportunity)
            .await
            .unwrap();
        let NarrativeRun::Completed(decision) = run else {
            panic!("the interrupted turn did not commit")
        };

        // Exactly one extra Interpreter inference: Projector, Persona, and two
        // Interpreter rounds.
        assert_eq!(turn.calls.load(Ordering::SeqCst), 4);

        // The Persona is never re-run: the prose and both inference receipts are
        // carried, and the binding is the only thing renewed.
        let renewed = decision.persona_turn().binding().clone();
        let prior = renewed
            .interrupted_from
            .as_deref()
            .expect("the committed turn does not record its interruption")
            .clone();
        assert_eq!(decision.persona_turn().source_prose(), source);
        assert_eq!(
            renewed.projector_receipt_digest,
            prior.projector_receipt_digest
        );
        assert_eq!(
            renewed.persona_inference_receipt_digest,
            prior.persona_inference_receipt_digest
        );
        assert_eq!(prior.scope_digest, opportunity.scope_digest.as_str());
        assert_ne!(renewed.scope_digest, prior.scope_digest);
        assert!(prior.interrupted_from.is_none());
        assert!(decision.persona_turn().receipt_is_valid());

        // The commit is bound to the fresh digest the row holds, and the world
        // took the act exactly once.
        assert_eq!(renewed.scope_digest, row_scope_digest(&turn.store));
        let log = mailbox.operator_log().await.unwrap();
        assert_eq!(log.len(), 2, "the neighbour spoke once and the actor once");
        assert_eq!(log[1].speaker, actor);

        // The second prompt is the first plus the interruption section, and it
        // names the speaker and the statement and nothing else.
        let seen = turn.seen.lock().unwrap();
        assert_eq!(seen.len(), 4);
        let first_interpreter = seen[2].clone();
        let second_interpreter = seen[3].clone();
        drop(seen);
        assert!(
            interruption_section_of(&first_interpreter).is_none(),
            "the first lowering was born interrupted"
        );
        let section = interruption_section_of(&second_interpreter)
            .expect("the re-lowered prompt has no section");
        for expected in [
            "- what this person knows changed",
            "What was said to this person since:",
            "Subject 1 said:",
            "The tollhouse ledger is short.",
        ] {
            assert!(
                section.contains(expected),
                "the interruption section dropped `{expected}`: {section}"
            );
        }
        // No fact id, no revision, no digest, and no `spoken_at`.
        for leaked in ["spoken_at", "sha256:", "revision", "scope_digest"] {
            assert!(
                !section.contains(leaked),
                "the interruption section leaked `{leaked}`"
            );
        }

        drop(turn.runner);
        drop(mailbox);
        task.await.unwrap();
    }

    /// Tests 2 and 3. A patch landing mid-turn moves a component that names no
    /// actor anywhere. The section carries the anonymous line for the moved
    /// field and no `What was said to this person since:` block at all.
    #[tokio::test]
    async fn a_change_with_no_author_is_re_lowered_with_no_overheard_block() {
        let (_directory, mailbox, task) = active_cell_mailbox(vec![
            NewController::NarrativePersona,
            NewController::OperationalAgent,
        ])
        .await;
        let (actor, opportunity) = narrative_opportunity(&mailbox).await;

        let source = "I say, \"Then we go by the lower stair.\"";
        let speech = "Then we go by the lower stair.";
        let turn = interrupted_turn(
            &mailbox,
            source,
            speech,
            2,
            vec![(2, MidTurnCommit::Commitment { subject: actor })],
        );
        let run = turn
            .runner
            .run_narrative(CommandId::new(), &opportunity)
            .await
            .unwrap();
        assert!(matches!(run, NarrativeRun::Completed(_)));
        assert_eq!(turn.calls.load(Ordering::SeqCst), 4);

        let seen = turn.seen.lock().unwrap();
        let second_interpreter = seen[3].clone();
        drop(seen);
        let section = interruption_section_of(&second_interpreter)
            .expect("the re-lowered prompt has no section");
        assert!(section.contains("- what this person owes changed"));
        assert!(
            !section.contains("What was said to this person since:"),
            "an anonymous change produced an overheard block: {section}"
        );
        for leaked in ["Subject 1", " said:", "came to know"] {
            assert!(
                !section.contains(leaked),
                "the anonymous section leaked `{leaked}`"
            );
        }

        drop(turn.runner);
        drop(mailbox);
        task.await.unwrap();
    }

    /// Test 5. The digest moves again during the re-lowered submit. The bound of
    /// one re-lowering is already spent, so the turn ends carrying the overtaken
    /// gap with zero further inference and nothing submitted.
    #[tokio::test]
    async fn a_second_scope_change_after_the_re_lowering_spends_nothing() {
        let (_directory, mailbox, task) = active_cell_mailbox(vec![
            NewController::NarrativePersona,
            NewController::OperationalAgent,
        ])
        .await;
        let (actor, opportunity) = narrative_opportunity(&mailbox).await;

        let source = "I say, \"Hold the line at the gate.\"";
        let speech = "Hold the line at the gate.";
        let turn = interrupted_turn(
            &mailbox,
            source,
            speech,
            2,
            vec![
                (2, MidTurnCommit::Commitment { subject: actor }),
                (3, MidTurnCommit::Commitment { subject: actor }),
            ],
        );
        let run = turn
            .runner
            .run_narrative(CommandId::new(), &opportunity)
            .await
            .unwrap();
        let NarrativeRun::Interrupted(interruption) = run else {
            panic!("a spent re-lowering did not end the turn")
        };
        assert_eq!(interruption.gap().kind, TranslationGapKind::Unresolved);
        assert_eq!(interruption.gap().detail, OVERTAKEN_DETAIL);
        assert_eq!(interruption.subject(), actor);
        assert_eq!(
            interruption.bound_scope_digest(),
            row_scope_digest(&turn.store),
            "the report names a digest the row does not hold"
        );
        // The bound was read before any selection, so no fresh digest was taken.
        assert_eq!(interruption.fresh_scope_digest(), None);
        assert_eq!(interruption.persona_turn().source_prose(), source);
        // Four inferences, exactly as the single re-lowering allows.
        assert_eq!(turn.calls.load(Ordering::SeqCst), 4);
        // Two patches landed; the actor never reached the world.
        let log = mailbox.operator_log().await.unwrap();
        assert!(log.is_empty(), "an overtaken turn reached the world");

        drop(turn.runner);
        drop(mailbox);
        task.await.unwrap();
    }

    /// The same genesis as `active_cell_mailbox`, plus one canonical fact
    /// declared and evidenced at genesis (so it needs no post-genesis
    /// `PatchAnswer`) that nobody yet knows: a live `EntityId` a mid-turn
    /// `Witness` can land over the commons without declaring anything itself.
    async fn witness_cell_mailbox() -> (
        tempfile::TempDir,
        WorldMailbox,
        tokio::task::JoinHandle<()>,
        EntityId,
        EntityId,
    ) {
        let directory = tempfile::tempdir().unwrap();
        let (mailbox, task) = WorldMailbox::open(directory.path().join("world.cc")).unwrap();
        let owner = PrincipalId::new("owner");
        let authenticated = AuthenticatedCaller::fixture(CallerId::Principal(owner.clone()));
        let ledger = EvidenceRef::new("the fixture's own witnessed bell");
        let declarations = vec![
            Declaration::Entity(EntityDeclaration {
                handle: DraftHandle::new("commons"),
                label: "The Commons".into(),
                kind: EntityKind::Place,
                container: None,
            }),
            Declaration::Fact(crate::world::patch::FactDeclaration {
                handle: DraftHandle::new("bell"),
                label: "A bell rings over the yard".into(),
                statement: Statement::new("A bell rings over the yard.").unwrap(),
                standing: crate::world::patch::FactStandingRef::Canonical {
                    evidence: ledger.clone(),
                },
            }),
            Declaration::Subject(SubjectDeclaration {
                handle: DraftHandle::new("subject0"),
                label: "Subject 0".into(),
                kind: SubjectKind::Person,
                controller: NewController::NarrativePersona,
                affordances: kernel_speak_grant(),
                position: Some(Ref::Draft(DraftHandle::new("commons"))),
            }),
            Declaration::Subject(SubjectDeclaration {
                handle: DraftHandle::new("subject1"),
                label: "Subject 1".into(),
                kind: SubjectKind::Person,
                controller: NewController::OperationalAgent,
                affordances: kernel_speak_grant(),
                position: Some(Ref::Draft(DraftHandle::new("commons"))),
            }),
        ];
        let creation = mailbox
            .create_fixture(
                CreateWorld {
                    id: CommandId::new(),
                    owner: owner.clone(),
                    title: "Witness Fixture".into(),
                    patch: WorldPatch {
                        declarations,
                        // Genesis is Draft phase, exempt from the post-genesis
                        // `PatchAnswer` a declaring `AdmitPatch` would need, so
                        // Subject 1 (never the narrative actor) is handed the
                        // fact here rather than through a second patch — a
                        // live `EntityId` to read back, not a fact this test's
                        // actor is meant to already know.
                        operations: vec![crate::world::patch::ComponentOp::AcquireKnowledge {
                            subject: Ref::Draft(DraftHandle::new("subject1")),
                            fact: Ref::Draft(DraftHandle::new("bell")),
                            source: crate::world::patch::AuthoredSource::Witnessed,
                            confidence: Confidence::Certain,
                        }],
                        evidence: vec![ledger],
                    },
                    scale_intent: WorldScaleIntentRef::default(),
                },
                &authenticated,
            )
            .await
            .unwrap();
        let mut snapshot = mailbox.snapshot().await.unwrap();
        for body in [CommandBody::ApproveDraft, CommandBody::ActivateWorld] {
            mailbox
                .submit_fixture(
                    CommandEnvelope {
                        id: CommandId::new(),
                        world_id: creation.world_id,
                        expected_revision: snapshot.revision,
                        caller: CallerId::Principal(owner.clone()),
                        body,
                    },
                    &authenticated,
                )
                .await
                .unwrap();
            snapshot = mailbox.snapshot().await.unwrap();
        }
        assert_eq!(snapshot.phase, WorldPhase::Active);
        let commons = snapshot.places[0].id;
        let bell = snapshot
            .subjects
            .iter()
            .find(|subject| subject.label == "Subject 1")
            .and_then(|subject| subject.knowledge.first())
            .map(|entry| entry.fact)
            .expect("Subject 1 was handed the bell at genesis");
        (directory, mailbox, task, bell, commons)
    }

    /// Soul's follow-up on the witness pass (step 10): a `Witness` landing
    /// between the actor's bound narrative turn and its submit re-lowers like
    /// every other un-authored cause, over the same anonymous `knows` line as
    /// `every_un_authored_cause_is_re_lowered_with_no_actor_and_no_value`'s own
    /// `AcquireKnowledge`-shaped causes. Unlike a neighbour's speech, the row a
    /// witness writes carries no `spoken_at` (`KnowledgeSource::Witnessed` is
    /// not `Told`), so `overheard_since` finds nothing this turn's binding
    /// postdates, and the section must carry no `Overheard` block at all — not
    /// even an empty one.
    #[tokio::test]
    async fn a_witness_between_the_turn_and_submit_carries_no_overheard_row() {
        let (_directory, mailbox, task, bell, commons) = witness_cell_mailbox().await;
        let (_actor, opportunity) = narrative_opportunity(&mailbox).await;

        let source = "I say, \"The western brace is giving way.\"";
        let speech = "The western brace is giving way.";
        let turn = interrupted_turn(
            &mailbox,
            source,
            speech,
            2,
            vec![(
                2,
                MidTurnCommit::Witness {
                    fact: bell,
                    place: commons,
                },
            )],
        );
        let run = turn
            .runner
            .run_narrative(CommandId::new(), &opportunity)
            .await
            .unwrap();
        assert!(
            matches!(run, NarrativeRun::Completed(_)),
            "a witness did not re-lower to a commit: {run:?}"
        );
        // Exactly one extra Interpreter inference, the one re-lowering owed.
        assert_eq!(turn.calls.load(Ordering::SeqCst), 4);

        let seen = turn.seen.lock().unwrap();
        let second_interpreter = seen[3].clone();
        drop(seen);
        let section = interruption_section_of(&second_interpreter)
            .expect("the re-lowered prompt has no section");
        assert!(
            section.contains("- what this person knows changed"),
            "the witness cause did not report the anonymous knows line: {section}"
        );
        assert!(
            !section.contains("What was said to this person since:"),
            "a witness produced an overheard block: {section}"
        );
        for leaked in [
            "Subject 0",
            "Subject 1",
            " said:",
            "came to know",
            "spoken_at",
        ] {
            assert!(
                !section.contains(leaked),
                "the witness section leaked `{leaked}`: {section}"
            );
        }

        drop(turn.runner);
        drop(mailbox);
        task.await.unwrap();
    }

    /// Test 6. The row a re-lowering persists rebuilds its own request byte for
    /// byte from `(turn, prompt, completed)` at the derived round and takes no
    /// snapshot to do it — which is exactly what a resumed runner does with it.
    #[tokio::test]
    async fn a_checkpoint_resumed_mid_re_lowering_rebuilds_the_same_request() {
        let (_directory, mailbox, task) = active_cell_mailbox(vec![
            NewController::NarrativePersona,
            NewController::OperationalAgent,
        ])
        .await;
        let (actor, opportunity) = narrative_opportunity(&mailbox).await;
        let source = "I say, \"Wait for the bell.\"";
        let speech = "Wait for the bell.";
        // One scripted Interpreter round only, so the run stops with the
        // re-lowered row persisted and unanswered.
        let turn = interrupted_turn(
            &mailbox,
            source,
            speech,
            1,
            vec![(2, MidTurnCommit::Commitment { subject: actor })],
        );
        let outcome = turn
            .runner
            .run_narrative(CommandId::new(), &opportunity)
            .await;
        assert!(
            matches!(outcome, Ok(NarrativeRun::Pending(_))),
            "the re-lowered round did not leave its row pending"
        );

        let checkpoint = narrative_row(&turn.store);
        let NarrativeCheckpoint::InterpreterInFlight {
            turn: renewed,
            interruption,
            completed,
            invocation,
            ..
        } = checkpoint.clone()
        else {
            panic!("the re-lowering did not persist an in-flight row")
        };
        let interruption = interruption.expect("the re-lowered row records no interruption");
        assert!(
            completed.is_empty(),
            "the re-lowering restarted carrying evidence"
        );
        assert_eq!(
            interruption.discarded.len(),
            1,
            "the first round was dropped"
        );
        assert!(renewed.binding().interrupted_from.is_some());
        // The round continues the row's numbering, so two conversations under
        // one command id cannot collide.
        assert_eq!(interpreter_round(&Some(interruption), &completed), 1);
        assert!(
            invocation
                .invocation
                .request
                .conversation_id
                .ends_with("-interpreter-1"),
            "the re-lowering reused the first conversation: {}",
            invocation.invocation.request.conversation_id
        );
        // The whole rebuild, byte for byte.
        assert!(ControllerWork::Narrative(checkpoint).integrity_is_valid());

        drop(turn.runner);
        drop(mailbox);
        task.await.unwrap();
    }

    /// Test 7. Three forgeries, each refused at the gate that owns it: an
    /// ancestry no row ever held, a chained re-lowering, and a row whose
    /// interruption and turn disagree about the same fact.
    #[test]
    fn a_forged_interruption_ancestry_is_refused() {
        let opportunity = fixture_opportunity(ControllerMode::NarrativePersona);
        let mut moved = opportunity.clone();
        moved.scope_digest = ScopeDigest::fixture("sha256:fixture-state-moved");
        let source = "I say, \"The brace is giving way.\"";
        let command_id = CommandId::new();
        let first = fixture_persona_turn(&opportunity, source);

        let ready = NarrativeCheckpoint::ReadyToSubmit {
            command_id,
            turn: first.clone(),
            interpreter_prompt: "prompt".into(),
            components: fixture_components(),
            interruption: None,
            opportunity: opportunity.clone(),
            granted: vec![speak_snapshot(opportunity.affordance_ids[0])],
            completed: Vec::new(),
        };
        let renewed_binding = |prior: &PersonaTurnBinding| PersonaTurnBinding {
            scope_digest: moved.scope_digest.as_str().to_owned(),
            opportunity_digest: moved.digest().unwrap(),
            interrupted_from: Some(Box::new(prior.clone())),
            ..prior.clone()
        };
        let interruption = Interruption {
            components: fixture_components(),
            overheard: Vec::new(),
            discarded: Vec::new(),
        };
        let in_flight = |turn: PersonaTurn, interruption: Option<Interruption>| {
            NarrativeCheckpoint::InterpreterInFlight {
                command_id,
                turn,
                interpreter_prompt: "prompt".into(),
                components: fixture_components(),
                interruption,
                opportunity: moved.clone(),
                granted: vec![speak_snapshot(moved.affordance_ids[0])],
                completed: Vec::new(),
                invocation: fixture_prepared(
                    interpreter_request(command_id, 1, "interpreter", Vec::new()).unwrap(),
                )
                .unwrap(),
            }
        };

        // An ancestry naming a binding this row's turn never held.
        let stranger = fixture_persona_turn(&moved, source);
        let forged = PersonaTurn::record(renewed_binding(stranger.binding()), source);
        assert!(
            !valid_narrative_progression(&ready, &in_flight(forged, Some(interruption.clone()))),
            "a fabricated ancestry was admitted"
        );

        // A re-lowered turn whose predecessor was itself re-lowered.
        let once = renewed_binding(first.binding());
        let twice = PersonaTurn::record(renewed_binding(&once), source);
        assert!(!twice.receipt_is_valid());
        assert_eq!(
            PersonaTurn::rehydrate(
                twice.binding().clone(),
                twice.source_prose(),
                twice.source_digest(),
                twice.receipt_digest(),
            ),
            Err(PersonaTurnIntegrityError::InterruptionChained)
        );

        // The two records of one fact may not disagree, in either direction.
        let honest = PersonaTurn::record(once, source);
        assert!(
            !in_flight(first, Some(interruption)).integrity_is_valid(),
            "a row claimed an interruption its turn does not carry"
        );
        assert!(
            !in_flight(honest, None).integrity_is_valid(),
            "a re-lowered turn's row dropped its interruption"
        );
    }

    /// Test 8. The leak proof over the delta surface, in the idiom of
    /// `a_subject_does_not_perceive_speech_it_was_not_in_reach_of` and over real
    /// committed state. An actor out of the listener's reach speaks; the
    /// listener is interrupted by something else entirely, and neither that
    /// actor's label nor its utterance may appear on the listener's surface.
    #[tokio::test]
    async fn the_interruption_section_names_no_actor_it_was_not_told_by() {
        let (_directory, mailbox, task) = apart_cell_mailbox(vec![
            NewController::NarrativePersona,
            NewController::OperationalAgent,
        ])
        .await;
        let (listener, opportunity) = narrative_opportunity(&mailbox).await;
        let far = mailbox
            .snapshot()
            .await
            .unwrap()
            .subjects
            .iter()
            .find(|subject| subject.id != listener)
            .expect("a subject out of reach")
            .id;

        let source = "I say, \"The stair is sound.\"";
        let speech = "The stair is sound.";
        let turn = interrupted_turn(
            &mailbox,
            source,
            speech,
            2,
            vec![
                (
                    1,
                    MidTurnCommit::Speech {
                        speaker: far,
                        text: "The vault seal is broken.".into(),
                    },
                ),
                (2, MidTurnCommit::Commitment { subject: listener }),
            ],
        );
        let run = turn
            .runner
            .run_narrative(CommandId::new(), &opportunity)
            .await
            .unwrap();
        assert!(matches!(run, NarrativeRun::Completed(_)));

        let seen = turn.seen.lock().unwrap();
        let second_interpreter = seen[3].clone();
        drop(seen);
        let section = interruption_section_of(&second_interpreter)
            .expect("the re-lowered prompt has no section");
        assert!(section.contains("- what this person owes changed"));
        assert!(!section.contains("What was said to this person since:"));
        for leaked in [
            "The vault seal is broken.",
            "Subject 1",
            encoded_id(&far).unwrap().as_str(),
        ] {
            assert!(
                !section.contains(leaked),
                "the interruption section leaked `{leaked}`"
            );
        }
        // Nor does the out-of-reach utterance reach any surface of this turn.
        assert!(!second_interpreter.contains("The vault seal is broken."));

        drop(turn.runner);
        drop(mailbox);
        task.await.unwrap();
    }

    /// Test 11, the negative check the deleted stage detectors owe. A digest
    /// that moves before the Persona inference is lowered rather than aborted:
    /// the run proceeds and the interruption is reported once at submit, not
    /// three times and not as `NoOpportunity`.
    #[tokio::test]
    async fn a_scope_that_moves_before_the_persona_inference_is_lowered_not_aborted() {
        let (_directory, mailbox, task) = active_cell_mailbox(vec![
            NewController::NarrativePersona,
            NewController::OperationalAgent,
        ])
        .await;
        let (actor, opportunity) = narrative_opportunity(&mailbox).await;
        let source = "I say, \"We leave before the bell.\"";
        let speech = "We leave before the bell.";
        // The commit lands during the Projector round, before the Persona has
        // been asked anything at all.
        let turn = interrupted_turn(
            &mailbox,
            source,
            speech,
            2,
            vec![(0, MidTurnCommit::Commitment { subject: actor })],
        );
        let run = turn
            .runner
            .run_narrative(CommandId::new(), &opportunity)
            .await
            .unwrap();
        let NarrativeRun::Completed(decision) = run else {
            panic!("an early scope change aborted the turn")
        };
        assert_eq!(
            turn.calls.load(Ordering::SeqCst),
            4,
            "one re-lowering, no more"
        );
        assert!(
            decision.persona_turn().binding().interrupted_from.is_some(),
            "the turn committed without recording the interruption"
        );
        let log = mailbox.operator_log().await.unwrap();
        assert_eq!(log.len(), 1);
        assert_eq!(log[0].speaker, actor);

        drop(turn.runner);
        drop(mailbox);
        task.await.unwrap();
    }

    /// Test 13. This pass adds `Deserialize` to `ScopeComponents` and no field,
    /// so `ScopePreimage`'s serialization is unchanged and every scope digest is
    /// byte-identical. The pin is over a hand-built preimage with fixed ids, so
    /// any future field, order, or serde-attribute change on either type fails
    /// here rather than silently reissuing every bound proposal in every live
    /// world.
    #[test]
    fn the_scope_digest_is_unchanged_by_this_pass() {
        let fixed = |value: &str| Value::String(value.into());
        let world: WorldId =
            serde_json::from_value(fixed("33333333-3333-4333-8333-333333333333")).unwrap();
        let subject: SubjectId =
            serde_json::from_value(fixed("44444444-4444-4444-8444-444444444444")).unwrap();
        let controller: ControllerId =
            serde_json::from_value(fixed("55555555-5555-4555-8555-555555555555")).unwrap();
        let affordance: AffordanceId =
            serde_json::from_value(fixed("66666666-6666-4666-8666-666666666666")).unwrap();
        let assignment = ControllerAssignment::NarrativePersona {
            controller_id: controller,
        };
        let entry = kernel_speak_entry();
        let components = fixture_components();
        let digest = world_digest(&ScopePreimage {
            world_id: world,
            subject_id: subject,
            controller: &assignment,
            affordances: BTreeMap::from([(affordance, &entry)]),
            components: &components,
        })
        .unwrap();
        assert_eq!(digest, PINNED_SCOPE_DIGEST);
    }

    /// The value `the_scope_digest_is_unchanged_by_this_pass` guards.
    const PINNED_SCOPE_DIGEST: &str =
        "sha256:39c881f8052be4e1fb278e78d20dcf9e081750affebafa33bd244de11ae18c9e";

    // ------------------------------------------- Soul falsification, step 9
    //
    // The pass proves the leak invariant over two causes: a speech the subject
    // was told, and an owner patch moving its commitments. These carry it over
    // the rest of the causes the handler claims to serve, over the values the
    // persisted row actually holds, and over the asymmetry the deleted stage
    // detectors left between the two detail lanes.

    use crate::world::patch::{AuthorityGrantRef, AuthorityTargetRef, RouteDeclaration};
    use crate::world::{AccessKind, AuthorityKindName, AuthorityTarget};

    /// An Active world carrying the furniture every un-authored cause needs: a
    /// route the actor stands on, a resource its neighbour holds, and a
    /// jurisdiction the actor is granted. `active_cell_mailbox` has none of
    /// these, so its subjects can only ever be interrupted through `knows` and
    /// `commitments`.
    struct InterruptionWorld {
        actor: SubjectId,
        neighbour: SubjectId,
        ramp: EdgeId,
        tithe: EntityId,
        hall: EntityId,
    }

    async fn interruption_world() -> (
        tempfile::TempDir,
        WorldMailbox,
        tokio::task::JoinHandle<()>,
        InterruptionWorld,
    ) {
        let directory = tempfile::tempdir().unwrap();
        let (mailbox, task) = WorldMailbox::open(directory.path().join("world.cc")).unwrap();
        let owner = PrincipalId::new("owner");
        let authenticated = AuthenticatedCaller::fixture(CallerId::Principal(owner.clone()));
        let place = |handle: &str, label: &str, container: Option<&str>| {
            Declaration::Entity(EntityDeclaration {
                handle: DraftHandle::new(handle),
                label: label.into(),
                kind: EntityKind::Place,
                container: container.map(|inside| Ref::Draft(DraftHandle::new(inside))),
            })
        };
        let inhabitant = |handle: &str, label: &str, controller: NewController| {
            Declaration::Subject(SubjectDeclaration {
                handle: DraftHandle::new(handle),
                label: label.into(),
                kind: SubjectKind::Person,
                controller,
                affordances: kernel_speak_grant(),
                position: Some(Ref::Draft(DraftHandle::new("commons"))),
            })
        };
        let ledger = EvidenceRef::new("the fixture ledger");
        let creation = mailbox
            .create_fixture(
                CreateWorld {
                    id: CommandId::new(),
                    owner: owner.clone(),
                    title: "Interruption Fixture".into(),
                    patch: WorldPatch {
                        declarations: vec![
                            place("commons", "The Commons", None),
                            // Inside the commons, and empty: the deficit that
                            // gives an elaborator session something to author
                            // under `PlaceSubtree(commons)`.
                            place("road", "The Road", Some("commons")),
                            place("hall", "The Hall", None),
                            Declaration::Entity(EntityDeclaration {
                                handle: DraftHandle::new("tithe"),
                                label: "The Rhythm Tithe".into(),
                                kind: EntityKind::Resource,
                                container: None,
                            }),
                            Declaration::Route(RouteDeclaration {
                                handle: DraftHandle::new("ramp"),
                                label: "The Yard Ramp".into(),
                                from: Ref::Draft(DraftHandle::new("commons")),
                                to: Ref::Draft(DraftHandle::new("road")),
                                access: AccessKind::Public,
                                cost: Cost(4),
                            }),
                            inhabitant("actor", "Subject 0", NewController::NarrativePersona),
                            inhabitant("neighbour", "Subject 1", NewController::OperationalAgent),
                        ],
                        operations: vec![
                            crate::world::patch::ComponentOp::Admit {
                                holder: Ref::Draft(DraftHandle::new("neighbour")),
                                resource: Ref::Draft(DraftHandle::new("tithe")),
                                qty: Quantity(5),
                                evidence: ledger.clone(),
                            },
                            crate::world::patch::ComponentOp::GrantAuthority {
                                holder: Ref::Draft(DraftHandle::new("actor")),
                                grant: AuthorityGrantRef {
                                    kind: AuthorityKindName("levy".into()),
                                    over: AuthorityTargetRef::PlaceSubtree(Ref::Draft(
                                        DraftHandle::new("hall"),
                                    )),
                                },
                            },
                        ],
                        evidence: vec![ledger],
                    },
                    scale_intent: WorldScaleIntentRef::default(),
                },
                &authenticated,
            )
            .await
            .unwrap();
        let mut snapshot = mailbox.snapshot().await.unwrap();
        for body in [CommandBody::ApproveDraft, CommandBody::ActivateWorld] {
            mailbox
                .submit_fixture(
                    CommandEnvelope {
                        id: CommandId::new(),
                        world_id: creation.world_id,
                        expected_revision: snapshot.revision,
                        caller: CallerId::Principal(owner.clone()),
                        body,
                    },
                    &authenticated,
                )
                .await
                .unwrap();
            snapshot = mailbox.snapshot().await.unwrap();
        }
        assert_eq!(snapshot.phase, WorldPhase::Active);
        let of = |label: &str| {
            snapshot
                .subjects
                .iter()
                .find(|subject| subject.label == label)
                .unwrap_or_else(|| panic!("the fixture declares {label}"))
        };
        let actor = of("Subject 0");
        let neighbour = of("Subject 1");
        let furniture = InterruptionWorld {
            actor: actor.id,
            neighbour: neighbour.id,
            ramp: *actor
                .components
                .routes
                .keys()
                .next()
                .expect("the actor stands on a route"),
            tithe: *neighbour
                .components
                .holdings
                .keys()
                .next()
                .expect("the neighbour holds the tithe"),
            hall: match actor
                .components
                .authority
                .iter()
                .next()
                .expect("the actor is granted a jurisdiction")
                .over
            {
                AuthorityTarget::PlaceSubtree(place) => place,
                AuthorityTarget::Subject(_) => panic!("the fixture grants over a place"),
            },
        };
        (directory, mailbox, task, furniture)
    }

    /// One interrupted narrative turn over `interruption_world`, driven to the
    /// re-lowering and stopped there with its row persisted: `rounds` of 1 means
    /// the re-lowered Interpreter call finds no script and leaves the row for
    /// the test to read. Returns the turn and the prompt bytes the port saw.
    async fn interrupted_by(
        mailbox: &WorldMailbox,
        source: &str,
        speech: &str,
        rounds: usize,
        commit: MidTurnCommit,
    ) -> (
        InterruptedTurn,
        Vec<String>,
        Result<NarrativeRun, ControllerError>,
    ) {
        let (_, opportunity) = narrative_opportunity(mailbox).await;
        let turn = interrupted_turn(mailbox, source, speech, rounds, vec![(2, commit)]);
        let run = turn
            .runner
            .run_narrative(CommandId::new(), &opportunity)
            .await;
        let seen = turn.seen.lock().unwrap().clone();
        (turn, seen, run)
    }

    /// Priority one, the leak invariant, over the causes the pass leaves
    /// untested: a transfer out of a neighbour's custody, a route closed under
    /// the actor's feet, the actor's own grant revoked, and an elaborator patch
    /// admitted under its root. Each moves a different component; each is
    /// something no subject may perceive as having an author. The section must
    /// carry the anonymous line for the field that moved, no `What was said`
    /// block at all, and no label, id, or value belonging to anyone.
    #[tokio::test]
    async fn every_un_authored_cause_is_re_lowered_with_no_actor_and_no_value() {
        type Cause = fn(&InterruptionWorld) -> MidTurnCommit;
        let causes: [(&str, &str, Cause); 3] = [
            (
                "a transfer",
                "- what this person holds changed",
                (|world| {
                    MidTurnCommit::Ops(vec![crate::world::patch::ComponentOp::Transfer {
                        from: Ref::Existing(world.neighbour),
                        to: Ref::Existing(world.actor),
                        resource: Ref::Existing(world.tithe),
                        qty: Quantity(2),
                    }])
                }) as Cause,
            ),
            ("a closed route", "- a way out of here changed", |world| {
                MidTurnCommit::Ops(vec![crate::world::patch::ComponentOp::CloseRoute {
                    route: Ref::Existing(world.ramp),
                }])
            }),
            (
                "a revoked grant",
                "- what this person is authorized over changed",
                |world| {
                    MidTurnCommit::Ops(vec![crate::world::patch::ComponentOp::RevokeAuthority {
                        holder: Ref::Existing(world.actor),
                        grant: AuthorityGrantRef {
                            kind: AuthorityKindName("levy".into()),
                            over: AuthorityTargetRef::PlaceSubtree(Ref::Existing(world.hall)),
                        },
                    }])
                },
            ),
        ];

        for (name, expected_line, cause) in causes {
            let (_directory, mailbox, task, world) = interruption_world().await;
            let source = "I say, \"The western brace is giving way.\"";
            let (turn, seen, run) = interrupted_by(
                &mailbox,
                source,
                "The western brace is giving way.",
                2,
                cause(&world),
            )
            .await;
            assert!(
                matches!(run, Ok(NarrativeRun::Completed(_))),
                "{name} did not re-lower to a commit: {run:?}"
            );
            assert_eq!(
                turn.calls.load(Ordering::SeqCst),
                4,
                "{name} spent something other than one re-lowering"
            );
            let section = interruption_section_of(&seen[3])
                .unwrap_or_else(|| panic!("{name} produced no interruption section"));
            assert!(
                section.contains(expected_line),
                "{name} did not report `{expected_line}`: {section}"
            );
            assert!(
                !section.contains("What was said to this person since:"),
                "{name} produced an overheard block: {section}"
            );
            // Nothing that could name a mover, a value, or a thing: the section
            // renders fixed English per changed field and nothing else.
            for leaked in [
                "Subject 0",
                "Subject 1",
                "The Rhythm Tithe",
                "The Yard Ramp",
                "The Roadside Shed",
                "The Hall",
                "levy",
                encoded_id(&world.neighbour).unwrap().as_str(),
                encoded_id(&world.actor).unwrap().as_str(),
                encoded_id(&world.tithe).unwrap().as_str(),
                encoded_id(&world.ramp).unwrap().as_str(),
                encoded_id(&world.hall).unwrap().as_str(),
            ] {
                assert!(
                    !section.contains(leaked),
                    "{name} leaked `{leaked}` into the interruption section: {section}"
                );
            }
            drop(turn.runner);
            drop(mailbox);
            task.await.unwrap();
        }
    }

    /// The membrane's exact boundary, and the sharpest thing to falsify here:
    /// the persisted `Interruption` carries a whole `ScopeComponents`, and a
    /// commitment inside it names its counterparty by id. That value is the
    /// subject's own digest-bound state and belongs in the row. It must reach
    /// no prompt byte, because the renderer takes values and emits only the
    /// name of the field that moved.
    #[tokio::test]
    async fn the_row_carries_a_counterparty_the_prompt_never_names() {
        let (_directory, mailbox, task, world) = interruption_world().await;
        let source = "I say, \"Then it is agreed.\"";
        // One scripted round only, so the re-lowered row is left persisted.
        let (turn, seen, run) = interrupted_by(
            &mailbox,
            source,
            "Then it is agreed.",
            1,
            MidTurnCommit::Ops(vec![crate::world::patch::ComponentOp::CreateCommitment {
                subject: Ref::Existing(world.actor),
                counterparty: Some(Ref::Existing(world.neighbour)),
                kind: CommitmentKind::Obligation,
                due: crate::world::FictionalMinutes(900),
                period: None,
                checks: Vec::new(),
            }]),
        )
        .await;
        assert!(
            matches!(run, Ok(NarrativeRun::Pending(_))),
            "the re-lowered round did not leave its row pending: {run:?}"
        );

        let NarrativeCheckpoint::InterpreterInFlight { interruption, .. } =
            narrative_row(&turn.store)
        else {
            panic!("the re-lowering did not persist an in-flight row")
        };
        let interruption = interruption.expect("the re-lowered row records no interruption");
        assert!(
            interruption
                .components
                .commitments
                .values()
                .any(|commitment| commitment.counterparty == Some(world.neighbour)),
            "the row does not carry the counterparty this test is about"
        );

        let section =
            interruption_section_of(&seen[3]).expect("the re-lowered prompt has no section");
        assert!(section.contains("- what this person owes changed"));
        for leaked in [
            "Subject 1",
            encoded_id(&world.neighbour).unwrap().as_str(),
            "obligation",
            "900",
        ] {
            assert!(
                !section.contains(leaked),
                "the section rendered `{leaked}` out of the components it diffs: {section}"
            );
        }

        drop(turn.runner);
        drop(mailbox);
        task.await.unwrap();
    }

    /// Fork B's claim, checked against the digest rather than against itself.
    /// `the_scope_digest_is_unchanged_by_this_pass` pins the serialization over
    /// a hand-built preimage; this asserts the other half — that the value the
    /// snapshot now hands the lane *is* what the kernel hashed for that subject
    /// at that revision. Rebuilding the preimage from the view alone and
    /// reproducing the live opportunity's digest is what "view and digest cannot
    /// drift" has to mean, and it is the property the five deleted projections
    /// used to spread across five fields.
    #[tokio::test]
    async fn the_snapshot_component_field_reproduces_every_live_scope_digest() {
        let (_directory, mailbox, task, world) = interruption_world().await;
        // Checked once on the fixture as declared, then again after a transfer
        // and a route closure have moved two more component kinds.
        for cause in [
            MidTurnCommit::Ops(vec![crate::world::patch::ComponentOp::Transfer {
                from: Ref::Existing(world.neighbour),
                to: Ref::Existing(world.actor),
                resource: Ref::Existing(world.tithe),
                qty: Quantity(1),
            }]),
            MidTurnCommit::Ops(vec![crate::world::patch::ComponentOp::CloseRoute {
                route: Ref::Existing(world.ramp),
            }]),
        ] {
            let snapshot = mailbox.snapshot().await.unwrap();
            assert_eq!(snapshot.opportunities.len(), 2, "two subjects hold a turn");
            for opportunity in &snapshot.opportunities {
                let subject = snapshot
                    .subjects
                    .iter()
                    .find(|subject| subject.id == opportunity.scope.subject_id)
                    .expect("every opportunity names a subject in the view");
                let controller_id = opportunity.controller_id;
                let assignment = match opportunity.controller_mode {
                    ControllerMode::NarrativePersona => {
                        ControllerAssignment::NarrativePersona { controller_id }
                    }
                    ControllerMode::OperationalAgent => {
                        ControllerAssignment::OperationalAgent { controller_id }
                    }
                    ControllerMode::Human => continue,
                };
                let entries = snapshot
                    .affordances
                    .iter()
                    .filter(|entry| opportunity.affordance_ids.contains(&entry.id))
                    .map(|entry| (entry.id, &entry.entry))
                    .collect::<BTreeMap<_, _>>();
                assert_eq!(
                    world_digest(&ScopePreimage {
                        world_id: snapshot.world_id,
                        subject_id: subject.id,
                        controller: &assignment,
                        affordances: entries,
                        components: &subject.components,
                    })
                    .unwrap(),
                    opportunity.scope_digest.as_str(),
                    "the view's components do not hash to the digest the kernel published"
                );
            }
            apply_mid_turn(&mailbox, &cause).await;
        }

        drop(mailbox);
        task.await.unwrap();
    }

    /// Fork A's cut, taken at the exact call sites it deleted, and the
    /// asymmetry it leaves behind. Both halves resume a persisted in-flight row
    /// after the world moved under it, which is where `ensure_scope_unchanged`
    /// used to stand in each lane. The operational lane still aborts there and
    /// spends nothing. The narrative lane no longer does: it spends its
    /// Interpreter round on a binding already known to be stale, is refused at
    /// submit, re-lowers, and commits. That extra round is the price the fork
    /// accepts, and it is bounded at one.
    #[tokio::test]
    async fn a_resumed_row_aborts_in_one_lane_and_is_lowered_in_the_other() {
        let (_directory, mailbox, task, world) = interruption_world().await;
        let (_, narrative) = narrative_opportunity(&mailbox).await;
        let operational = mailbox
            .snapshot()
            .await
            .unwrap()
            .opportunities
            .iter()
            .find(|entry| entry.scope.subject_id == world.neighbour)
            .expect("the neighbour has a live opportunity")
            .clone();
        let source = "I say, \"We leave by the upper stair.\"";
        let speech = "We leave by the upper stair.";

        // Each lane is driven to a persisted in-flight row and left there by a
        // script that runs out.
        let narrative_command = CommandId::new();
        let first = interrupted_turn(&mailbox, source, speech, 0, Vec::new());
        assert!(matches!(
            first
                .runner
                .run_narrative(narrative_command, &narrative)
                .await,
            Ok(NarrativeRun::Pending(_))
        ));
        assert!(matches!(
            narrative_row(&first.store),
            NarrativeCheckpoint::InterpreterInFlight { .. }
        ));
        let operational_command = CommandId::new();
        let opening_store = Arc::new(RecordingWorkStore {
            persisted: Arc::new(AtomicBool::new(true)),
            work: Mutex::new(BTreeMap::new()),
        });
        let opening_calls = Arc::new(AtomicUsize::new(0));
        let opening = ControllerRunner::open(
            mailbox.clone(),
            // No scripted round at all: the opening call faults retryably, so
            // the lane leaves its `AgentInFlight` row persisted and unanswered.
            Arc::new(InterruptingPort {
                mailbox: mailbox.clone(),
                outputs: Mutex::new(Vec::new()),
                calls: opening_calls.clone(),
                commits: Vec::new(),
                seen: Arc::new(Mutex::new(Vec::new())),
            }),
            opening_store.clone(),
            models(),
        )
        .expect("the fixture ports open");
        let opened = opening
            .run_operational(operational_command, &operational)
            .await;
        assert!(
            matches!(opened, Ok(OperationalRun::Pending(_))),
            "the operational opening round did not leave a row pending: {opened:?}"
        );
        assert_eq!(
            opening_calls.load(Ordering::SeqCst),
            1,
            "the operational opening round never reached the port"
        );
        drop(first.runner);
        drop(opening);

        // Then the world moves under both rows.
        apply_mid_turn(
            &mailbox,
            &MidTurnCommit::Ops(vec![crate::world::patch::ComponentOp::CloseRoute {
                route: Ref::Existing(world.ramp),
            }]),
        )
        .await;

        // The operational lane checks the digest at the top of its loop and
        // stops without asking the provider anything.
        let operational_calls = Arc::new(AtomicUsize::new(0));
        let refused = ControllerRunner::open(
            mailbox.clone(),
            Arc::new(InterruptingPort {
                mailbox: mailbox.clone(),
                outputs: Mutex::new(vec![output(
                    vec![InferenceEvent::Text("Nothing further.".into())],
                    "operational-resume",
                )]),
                calls: operational_calls.clone(),
                commits: Vec::new(),
                seen: Arc::new(Mutex::new(Vec::new())),
            }),
            opening_store.clone(),
            models(),
        )
        .expect("the fixture ports open")
        .run_operational(operational_command, &operational)
        .await;
        assert!(
            matches!(refused, Err(ControllerError::NoOpportunity { .. })),
            "the operational lane stopped holding its early abort: {refused:?}"
        );
        assert_eq!(
            operational_calls.load(Ordering::SeqCst),
            0,
            "a lane with nothing to preserve spent an inference on a doomed turn"
        );

        // The narrative lane has prose to preserve, so it runs on.
        let narrative_calls = Arc::new(AtomicUsize::new(0));
        let run = ControllerRunner::open(
            mailbox.clone(),
            Arc::new(InterruptingPort {
                mailbox: mailbox.clone(),
                outputs: Mutex::new(vec![
                    speak_span(source, speech, "resume-zero"),
                    speak_span(source, speech, "resume-one"),
                ]),
                calls: narrative_calls.clone(),
                commits: Vec::new(),
                seen: Arc::new(Mutex::new(Vec::new())),
            }),
            first.store.clone(),
            models(),
        )
        .expect("the fixture ports open")
        .run_narrative(narrative_command, &narrative)
        .await
        .expect("a resumed narrative row on a moved scope is no longer an error");
        let NarrativeRun::Completed(decision) = run else {
            panic!("the resumed narrative row did not re-lower to a commit")
        };
        assert!(
            decision.persona_turn().binding().interrupted_from.is_some(),
            "the turn committed without recording the interruption"
        );
        assert_eq!(
            narrative_calls.load(Ordering::SeqCst),
            2,
            "fork A's accepted cost is one extra Interpreter round, no more"
        );

        drop(mailbox);
        task.await.unwrap();
    }
}
