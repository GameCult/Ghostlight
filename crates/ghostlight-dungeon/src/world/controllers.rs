//! The two cognition modes behind one exact world opportunity.
//!
//! This organ may ask models to think. It cannot mint authority: it reads one
//! `WorldMailbox` snapshot, binds one controller-owned opportunity, and submits
//! one typed exercise or decline through that same mailbox. The model-facing
//! tools never carry caller, controller, world, opportunity, revision, or
//! affordance fields.

use super::elaboration::{
    ElaborationCheckpoint, ElaborationRunner, NullEvidenceSource, valid_elaboration_progression,
};
use super::tool_schema;
use crate::world::{
    AffordanceId, AffordanceSnapshot, AuthorityGrant, Bounds, Cell, CellId, CommandId,
    CommitReceipt, Confidence, Constituent, ControllerMode, ControllerPort, Cost,
    DecisionInvocation, DecisionOpportunity, DependencyTarget, EdgeId, ElaborationPort, EntityId,
    EntityKind, FactStandingView, KernelError, KnowledgeSnapshot, KnowledgeSource, Magnitude,
    MailboxError, OfficeSnapshot, ProposedEffect, Quantity, RefKind, Resolution, RoleBinding,
    Statement, SubjectId, SubjectSnapshot, SubmitReceipt, Target, TickIndex, WorldMailbox,
    WorldSnapshot,
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
    path::Path,
    sync::{Arc, Mutex},
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use thiserror::Error;
use zeroize::Zeroizing;

const MAX_CONNECTOR_FRAME_BYTES: usize = 1_052_672;
const REQUEST_EXPIRY: Duration = Duration::from_secs(300);
const TOOL_STEP_BUDGET: usize = 4;
/// The grouped protocol asks for every call in one round, so this budget buys
/// exactly one repair round after decode gaps are reported back. It deliberately
/// does not scale with the cell: a cell that needs three rounds of repair is a
/// cell that should have been smaller.
const CELL_TOOL_STEP_BUDGET: usize = 2;
/// Separates a constituent handle from the tool it names. Attribution is
/// carried by tool identity, never by a model-written argument.
const HANDLE_SEPARATOR: &str = "__";
const PERSONA_WORD_BUDGET: usize = 180;
const CONTROLLER_WORK_ROW: &str = "controller_work.v9";
const CONTROLLER_WORK_SCHEMA: &str = "ghostlight.controller_work.v9";

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

/// The exact connector invocation, including expiry and native provenance.
/// Replaying this value may recover a completed connector response; rebuilding
/// it under the same request ID would be a replay conflict.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct PreparedInference {
    purpose: InferencePurpose,
    pub(super) invocation: CodexTransportInvocation,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(super) enum InferenceEvent {
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
    fn prose_only(self, purpose: InferencePurpose) -> Result<(String, String), ControllerError> {
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
    fn new(detail: impl Into<String>) -> Self {
        Self {
            disposition: InferenceFaultDisposition::RecoveryRequired,
            detail: detail.into(),
        }
    }

    fn retryable(detail: impl Into<String>) -> Self {
        Self {
            disposition: InferenceFaultDisposition::Retryable,
            detail: detail.into(),
        }
    }

    fn integrity_violation(detail: impl Into<String>) -> Self {
        Self {
            disposition: InferenceFaultDisposition::IntegrityViolation,
            detail: detail.into(),
        }
    }

    fn recovery_required(&self) -> bool {
        self.disposition == InferenceFaultDisposition::RecoveryRequired
    }

    fn integrity_was_violated(&self) -> bool {
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
}

#[async_trait]
pub(crate) trait InferencePort: Send + Sync {
    fn prepare(&self, request: InferenceRequest) -> Result<PreparedInference, InferenceFault>;

    async fn infer(&self, request: PreparedInference) -> Result<InferenceOutput, InferenceFault>;
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
            Some(REQUEST_EXPIRY),
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

    fn prepare_request(
        &self,
        request: InferenceRequest,
    ) -> Result<PreparedInference, InferenceFault> {
        let request_bytes =
            serde_json::to_vec(&request).map_err(|error| InferenceFault::new(error.to_string()))?;
        let invocation = CodexTransportInvocation::new(
            self.caller_runtime_id.clone(),
            unix_ms()?.saturating_add(REQUEST_EXPIRY.as_millis() as u64),
            Sha256::digest(request_bytes).into(),
            request.provider,
        )
        .map_err(|error| InferenceFault::new(error.to_string()))?;
        Ok(PreparedInference {
            purpose: request.purpose,
            invocation,
        })
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
        self.prepare_request(request)
    }

    async fn infer(&self, request: PreparedInference) -> Result<InferenceOutput, InferenceFault> {
        let port = self.clone();
        tokio::task::spawn_blocking(move || port.execute(request))
            .await
            .map_err(|error| InferenceFault::new(error.to_string()))?
    }
}

fn unix_ms() -> Result<u64, InferenceFault> {
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
struct ConstituentWork {
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
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "stage", rename_all = "snake_case")]
enum NarrativeCheckpoint {
    Projector {
        command_id: CommandId,
        identity: String,
        typed_view: String,
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
        opportunity: DecisionOpportunity,
        granted: Vec<AffordanceSnapshot>,
        completed: Vec<InferenceOutput>,
        invocation: PreparedInference,
    },
    ReadyToSubmit {
        command_id: CommandId,
        turn: PersonaTurn,
        interpreter_prompt: String,
        opportunity: DecisionOpportunity,
        granted: Vec<AffordanceSnapshot>,
        completed: Vec<InferenceOutput>,
    },
    NoProposal {
        command_id: CommandId,
        turn: PersonaTurn,
        interpreter_prompt: String,
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
        }
    }

    fn lane(&self) -> WorkLane {
        match self {
            Self::Narrative(_) => WorkLane::Narrative,
            Self::Operational(_) => WorkLane::Operational,
            Self::Grouped(_) => WorkLane::Grouped,
            Self::Elaboration(_) => WorkLane::Elaboration,
        }
    }

    /// How the subjects in this row were represented. Derived from the row's
    /// own shape rather than stored beside it: a persisted copy would be a
    /// second spelling of `constituents.len()` that could disagree with it.
    #[cfg_attr(not(test), expect(dead_code, reason = "read by the resolution test"))]
    fn resolution(&self) -> Resolution {
        match self {
            Self::Narrative(_) | Self::Operational(_) | Self::Elaboration(_) => Resolution::Detail,
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
            _ => false,
        }
    }

    fn integrity_is_valid(&self) -> bool {
        match self {
            Self::Narrative(checkpoint) => checkpoint.integrity_is_valid(),
            Self::Operational(checkpoint) => checkpoint.integrity_is_valid(),
            Self::Grouped(checkpoint) => checkpoint.integrity_is_valid(),
            Self::Elaboration(checkpoint) => checkpoint.integrity_is_valid(),
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
                opportunity,
                granted,
                completed,
                invocation,
            } => {
                turn_matches_opportunity(turn, opportunity)
                    && opportunity.controller_mode == ControllerMode::NarrativePersona
                    && granted_matches_opportunity(granted, opportunity)
                    && !interpreter_prompt.is_empty()
                    && canonical_model(&invocation.invocation.request.model)
                    && match evaluate_interpreter_loop(turn, interpreter_prompt, completed) {
                        Ok(InterpreterLoopEvaluation::Continue { conversation }) => {
                            interpreter_request(
                                *command_id,
                                completed.len(),
                                &invocation.invocation.request.model,
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
                        Ok(InterpreterLoopEvaluation::Complete { .. }) | Err(_) => false,
                    }
            }
            Self::ReadyToSubmit {
                turn,
                interpreter_prompt,
                opportunity,
                granted,
                completed,
                ..
            } => {
                turn_matches_opportunity(turn, opportunity)
                    && opportunity.controller_mode == ControllerMode::NarrativePersona
                    && granted_matches_opportunity(granted, opportunity)
                    && derive_narrative_capture(turn, interpreter_prompt, completed)
                        .is_ok_and(|capture| capture.proposal.is_some())
            }
            Self::NoProposal {
                turn,
                interpreter_prompt,
                opportunity,
                completed,
                ..
            } => {
                turn_matches_opportunity(turn, opportunity)
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
                && opportunity.scope_digest == exact_opportunity.scope_digest
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
        // The run's own bound value, not the fresh snapshot's: one run binds
        // one opportunity, persists it, and submits it unchanged.
        opportunity: exact_opportunity.clone(),
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
        && provider_request_id(command_id, purpose, round)
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
                && opportunity == next_opportunity
                && granted == next_granted
                && completed.is_empty()
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
                opportunity,
                granted,
                completed,
                invocation: existing_invocation,
            },
            NarrativeCheckpoint::InterpreterInFlight {
                command_id: next_command_id,
                turn: next_turn,
                interpreter_prompt: next_interpreter_prompt,
                opportunity: next_opportunity,
                granted: next_granted,
                completed: next_completed,
                invocation: next_invocation,
            },
        ) => {
            command_id == next_command_id
                && turn == next_turn
                && interpreter_prompt == next_interpreter_prompt
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
                opportunity,
                granted,
                completed,
                ..
            },
            NarrativeCheckpoint::ReadyToSubmit {
                command_id: next_command_id,
                turn: next_turn,
                interpreter_prompt: next_interpreter_prompt,
                opportunity: next_opportunity,
                granted: next_granted,
                completed: next_completed,
            },
        ) => {
            command_id == next_command_id
                && turn == next_turn
                && interpreter_prompt == next_interpreter_prompt
                && opportunity == next_opportunity
                && granted == next_granted
                && completed_advances(completed, next_completed)
        }
        (
            NarrativeCheckpoint::InterpreterInFlight {
                command_id,
                turn,
                interpreter_prompt,
                opportunity,
                completed,
                ..
            },
            NarrativeCheckpoint::NoProposal {
                command_id: next_command_id,
                turn: next_turn,
                interpreter_prompt: next_interpreter_prompt,
                opportunity: next_opportunity,
                completed: next_completed,
            },
        ) => {
            command_id == next_command_id
                && turn == next_turn
                && interpreter_prompt == next_interpreter_prompt
                && opportunity == next_opportunity
                && completed_advances(completed, next_completed)
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
    /// is added.
    pub(crate) elaborator: String,
}

impl ControllerModels {
    fn are_canonical(&self) -> bool {
        [
            &self.projector,
            &self.persona,
            &self.interpreter,
            &self.operational_agent,
            &self.elaborator,
        ]
        .into_iter()
        .all(|model| canonical_model(model))
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
    /// Opens the complete production controller organ. Runtime supplies
    /// deployment configuration, but cannot replace either the inference
    /// transport or the durable controller-work owner. The caller still hands
    /// over a whole `WorldMailbox` — that stays the one owner-facing type —
    /// but this constructor is where it narrows to a `ControllerPort` before
    /// the runner ever sees it, so nothing inside this module can reach past
    /// the five requests a controller lane makes.
    pub(crate) fn open(
        mailbox: WorldMailbox,
        connector_endpoint: SocketAddr,
        connector_key_path: impl AsRef<Path>,
        caller_runtime_id: impl Into<String>,
        controller_work_path: impl AsRef<Path>,
        models: ControllerModels,
    ) -> Result<Self, ControllerOpenError> {
        if !models.are_canonical() {
            return Err(ControllerOpenError::InvalidModels);
        }
        let inference = CodexConnectorInferencePort::from_secret_file(
            connector_endpoint,
            connector_key_path,
            caller_runtime_id,
        )?;
        let work = CultCacheControllerWorkStore::open(controller_work_path)?;
        Ok(Self {
            mailbox: ControllerPort::new(mailbox.clone()),
            elaboration: ElaborationPort::new(mailbox),
            inference: Arc::new(inference),
            work: Arc::new(work),
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

    #[cfg(test)]
    pub(crate) fn with_test_ports(
        mailbox: WorldMailbox,
        inference: Arc<dyn InferencePort>,
        work: Arc<dyn ControllerWorkStore>,
        models: ControllerModels,
    ) -> Self {
        Self {
            mailbox: ControllerPort::new(mailbox.clone()),
            elaboration: ElaborationPort::new(mailbox),
            inference,
            work,
            models,
        }
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
                | ControllerWork::Elaboration(_),
            )
            | ControllerWorkLookup::CustodyUncertain(
                ControllerWork::Operational(_)
                | ControllerWork::Grouped(_)
                | ControllerWork::Elaboration(_),
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
                | ControllerWork::Elaboration(_),
            )
            | ControllerWorkLookup::CustodyUncertain(
                ControllerWork::Narrative(_)
                | ControllerWork::Grouped(_)
                | ControllerWork::Elaboration(_),
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
                | ControllerWork::Elaboration(_),
            )
            | ControllerWorkLookup::CustodyUncertain(
                ControllerWork::Narrative(_)
                | ControllerWork::Operational(_)
                | ControllerWork::Elaboration(_),
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

    /// The scope this run bound still derives the same digest. A commit
    /// elsewhere in the world no longer kills an in-flight turn.
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
        self.ensure_scope_unchanged(&opportunity).await?;
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
        self.ensure_scope_unchanged(&opportunity).await?;
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
            self.ensure_scope_unchanged(&opportunity).await?;
            let model = invocation.invocation.request.model.clone();
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
                            opportunity,
                            granted,
                            completed,
                        }
                    } else {
                        NarrativeCheckpoint::NoProposal {
                            command_id,
                            turn,
                            interpreter_prompt,
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
                    let round = completed.len();
                    let next = NarrativeCheckpoint::InterpreterInFlight {
                        command_id,
                        turn,
                        interpreter_prompt,
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
            .await?
        {
            ControllerWorldSubmission::Completed(submission) => {
                completed_narrative(&checkpoint, submission)
            }
            ControllerWorldSubmission::Pending(reason) => Ok(narrative_pending(checkpoint, reason)),
        }
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
            .incident_routes
            .iter()
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
                    "speaker": self.speaker_label(entry),
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
    fn speaker_label(&self, entry: &KnowledgeSnapshot) -> Value {
        match entry.source {
            KnowledgeSource::Told { by, .. } => self
                .snapshot
                .subjects
                .iter()
                .find(|subject| subject.id == by)
                .map_or(Value::Null, |subject| Value::String(subject.label.clone())),
            KnowledgeSource::Witnessed | KnowledgeSource::Evidenced => Value::Null,
        }
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
            .controls
            .iter()
            .map(|channel| json!({"id": channel}))
            .collect()
    }
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

fn evaluate_interpreter_loop(
    source: &PersonaTurn,
    prompt: &str,
    completed: &[InferenceOutput],
) -> Result<InterpreterLoopEvaluation, ControllerError> {
    let mut conversation = vec![CodexInputItem::UserText {
        text: prompt.to_owned(),
    }];
    let mut accumulator = InterpretationAccumulator::new(source.clone());
    let mut captured_speech = false;
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
        let mut finished = false;
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
                    let result = match name.as_str() {
                        INTERPRETER_SPEAK_TOOL => {
                            match serde_json::from_str::<InterpreterSpeakCall>(arguments) {
                                Ok(call) if !captured_speech => {
                                    let derived_text = source
                                        .source_prose()
                                        .get(call.source_start_byte..call.source_end_byte)
                                        .unwrap_or_default()
                                        .to_owned();
                                    let feedback = accumulator.capture_proposal(
                                        SpeakProposal { text: derived_text },
                                        call.source_start_byte,
                                        call.source_end_byte,
                                    );
                                    captured_speech = feedback == CaptureToolFeedback::Accepted;
                                    format!("{feedback:?}")
                                }
                                Ok(call) => {
                                    let feedback = accumulator.record_gap(RecordGapToolCall {
                                    kind: TranslationGapKind::Ambiguity,
                                    source_start_byte: call.source_start_byte,
                                    source_end_byte: call.source_end_byte,
                                    detail: "More than one speech proposal was offered; this runner permits one decision invocation per opportunity.".into(),
                                });
                                    format!("{feedback:?}")
                                }
                                Err(error) => format!(
                                    "{:?}",
                                    accumulator.record_tool_decode_failure(
                                        name,
                                        arguments,
                                        &error.to_string(),
                                    )
                                ),
                            }
                        }
                        INTERPRETER_RECORD_GAP_TOOL => {
                            match serde_json::from_str::<RecordGapToolCall>(arguments) {
                                Ok(call) => format!("{:?}", accumulator.record_gap(call)),
                                Err(error) => format!(
                                    "{:?}",
                                    accumulator.record_tool_decode_failure(
                                        name,
                                        arguments,
                                        &error.to_string(),
                                    )
                                ),
                            }
                        }
                        FINISH_INTERPRETATION_TOOL => {
                            match serde_json::from_str::<EmptyToolCall>(arguments) {
                                Ok(_) => {
                                    finished = true;
                                    "interpretation finished".into()
                                }
                                Err(error) => format!(
                                    "{:?}",
                                    accumulator.record_tool_decode_failure(
                                        name,
                                        arguments,
                                        &error.to_string(),
                                    )
                                ),
                            }
                        }
                        _ => format!(
                            "{:?}",
                            accumulator.record_tool_decode_failure(
                                name,
                                arguments,
                                "tool is not available for this exact opportunity",
                            )
                        ),
                    };
                    conversation.push(CodexInputItem::ToolResult {
                        call_id: call_id.clone(),
                        output: result,
                    });
                }
            }
        }

        let finalization = if finished || !called_tool {
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
            let report = accumulator.finalize(finalization);
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

fn evaluate_grouped_loop(
    prompt: &str,
    constituents: &[ConstituentWork],
    completed: &[InferenceOutput],
) -> Result<GroupedLoopEvaluation, ControllerError> {
    let mut conversation = vec![CodexInputItem::UserText {
        text: prompt.to_owned(),
    }];
    let mut proposals: BTreeMap<usize, DecisionInvocation> = BTreeMap::new();
    let mut terminal: BTreeSet<usize> = BTreeSet::new();
    let mut needs = Vec::new();

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
                    let result: String = match split_handle(name, constituents.len()) {
                        // A model that writes `c99__carry` has not proposed
                        // anything. It has produced a gap.
                        None => {
                            needs.push(tool_decode_need(
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
                                    if terminal.contains(&handle) {
                                        needs.push(ControllerNeed {
                                            detail: format!(
                                                "Handle c{handle} was offered more than one terminal choice for one opportunity."
                                            ),
                                        });
                                        "one terminal choice is already captured for this handle"
                                            .into()
                                    } else {
                                        match decode_catalog_call(
                                            entry.expect("the entry matched above"),
                                            arguments,
                                        ) {
                                            Ok(invocation) => {
                                                proposals.insert(handle, invocation);
                                                terminal.insert(handle);
                                                "invocation captured".into()
                                            }
                                            Err(detail) => {
                                                needs.push(tool_decode_need(
                                                    name, arguments, &detail,
                                                ));
                                                "arguments recorded as a need".into()
                                            }
                                        }
                                    }
                                }
                                RECORD_NEED_TOOL => {
                                    match serde_json::from_str::<RecordNeedCall>(arguments) {
                                        Ok(call) => {
                                            needs.push(ControllerNeed {
                                                detail: format!("c{handle}: {}", call.detail),
                                            });
                                            "need recorded".into()
                                        }
                                        Err(error) => {
                                            needs.push(tool_decode_need(
                                                name,
                                                arguments,
                                                &error.to_string(),
                                            ));
                                            "arguments recorded as a need".into()
                                        }
                                    }
                                }
                                FINISH_WITHOUT_PROPOSAL_TOOL => {
                                    match serde_json::from_str::<EmptyToolCall>(arguments) {
                                        Ok(_) if terminal.contains(&handle) => {
                                            return Err(ControllerError::ProviderContract {
                                                purpose: InferencePurpose::GroupedAgent,
                                                detail: "finish_without_proposal contradicted an existing terminal choice".into(),
                                            });
                                        }
                                        Ok(_) => {
                                            terminal.insert(handle);
                                            "decision finished without a proposal".into()
                                        }
                                        Err(error) => {
                                            needs.push(tool_decode_need(
                                                name,
                                                arguments,
                                                &error.to_string(),
                                            ));
                                            "arguments recorded as a need".into()
                                        }
                                    }
                                }
                                _ => {
                                    needs.push(tool_decode_need(
                                        name,
                                        arguments,
                                        "tool is not available to this handle",
                                    ));
                                    "unavailable tool recorded as a need".into()
                                }
                            }
                        }
                    };
                    conversation.push(CodexInputItem::ToolResult {
                        call_id: call_id.clone(),
                        output: result,
                    });
                }
            }
        }

        let is_complete = terminal.len() == constituents.len()
            || !called_tool
            || round + 1 == CELL_TOOL_STEP_BUDGET;
        if is_complete {
            if round + 1 != completed.len() {
                return Err(ControllerError::Serialization(
                    "grouped evidence continued after total finalization".into(),
                ));
            }
            if round + 1 == CELL_TOOL_STEP_BUDGET
                && terminal.len() < constituents.len()
                && called_tool
            {
                needs.push(ControllerNeed {
                    detail: "The grouped step budget ended before every handle finished.".into(),
                });
            }
            return Ok(GroupedLoopEvaluation::Complete {
                capture: GroupedCapture { proposals, needs },
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

fn evaluate_operational_loop(
    prompt: &str,
    granted: &[AffordanceSnapshot],
    completed: &[InferenceOutput],
) -> Result<OperationalLoopEvaluation, ControllerError> {
    let mut conversation = vec![CodexInputItem::UserText {
        text: prompt.to_owned(),
    }];
    let mut proposal = None;
    let mut needs = Vec::new();
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
        let mut terminal_choice = None;
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
                    let entry = granted.iter().find(|entry| entry.entry.kind.0 == *name);
                    let result = match name.as_str() {
                        _ if entry.is_some() => match decode_catalog_call(
                            entry.expect("the entry matched above"),
                            arguments,
                        ) {
                            Ok(invocation) if terminal_choice.is_none() => {
                                proposal = Some(invocation);
                                terminal_choice = Some(name.clone());
                                "invocation captured".into()
                            }
                            Ok(_) => {
                                needs.push(ControllerNeed {
                                    detail: "The agent offered more than one terminal choice for one opportunity.".into(),
                                });
                                "one terminal choice is already captured".into()
                            }
                            Err(detail) => {
                                needs.push(tool_decode_need(name, arguments, &detail));
                                "arguments recorded as a need".into()
                            }
                        },
                        RECORD_NEED_TOOL => match serde_json::from_str::<RecordNeedCall>(arguments)
                        {
                            Ok(call) => {
                                needs.push(ControllerNeed {
                                    detail: call.detail,
                                });
                                "need recorded".into()
                            }
                            Err(error) => {
                                needs.push(tool_decode_need(name, arguments, &error.to_string()));
                                "arguments recorded as a need".into()
                            }
                        },
                        FINISH_WITHOUT_PROPOSAL_TOOL => {
                            match serde_json::from_str::<EmptyToolCall>(arguments) {
                                Ok(_) => {
                                    if terminal_choice.is_some() {
                                        return Err(ControllerError::ProviderContract {
                                            purpose: InferencePurpose::OperationalAgent,
                                            detail: "finish_without_proposal contradicted an existing terminal choice".into(),
                                        });
                                    }
                                    terminal_choice = Some(FINISH_WITHOUT_PROPOSAL_TOOL.to_owned());
                                    "decision finished without a proposal".into()
                                }
                                Err(error) => {
                                    needs.push(tool_decode_need(
                                        name,
                                        arguments,
                                        &error.to_string(),
                                    ));
                                    "arguments recorded as a need".into()
                                }
                            }
                        }
                        _ => {
                            needs.push(tool_decode_need(
                                name,
                                arguments,
                                "tool is not available for this exact opportunity",
                            ));
                            "unavailable tool recorded as a need".into()
                        }
                    };
                    conversation.push(CodexInputItem::ToolResult {
                        call_id: call_id.clone(),
                        output: result,
                    });
                }
            }
        }

        let is_complete =
            terminal_choice.is_some() || !called_tool || round + 1 == TOOL_STEP_BUDGET;
        if is_complete {
            if round + 1 != completed.len() {
                return Err(ControllerError::Serialization(
                    "operational evidence continued after total finalization".into(),
                ));
            }
            if round + 1 == TOOL_STEP_BUDGET && terminal_choice.is_none() && called_tool {
                needs.push(ControllerNeed {
                    detail: "The operational-agent step budget ended before explicit completion."
                        .into(),
                });
            }
            return Ok(OperationalLoopEvaluation::Complete {
                capture: OperationalCapture {
                    proposal,
                    needs,
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
    let mut provider = CodexProviderRequest::new(
        provider_request_id(command_id, InferencePurpose::Persona, 0)?,
        conversation_id(command_id, InferencePurpose::Persona, 0)?,
        model,
        PERSONA_PROVIDER_INSTRUCTIONS,
    );
    provider.input = vec![CodexInputItem::UserText { text: prompt }];
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
    let request = provider_request_id(command_id, purpose, 0)?;
    let conversation = conversation_id(command_id, purpose, 0)?;
    let mut provider = CodexProviderRequest::new(request, conversation, model, instructions);
    provider.input = vec![CodexInputItem::UserText { text: prompt }];
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
        provider_request_id(command_id, purpose, round)?,
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
fn catalog_tools(prefix: &str, granted: &[AffordanceSnapshot]) -> Vec<CodexToolDefinition> {
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

pub(super) fn provider_request_id(
    command_id: CommandId,
    purpose: InferencePurpose,
    round: usize,
) -> Result<String, ControllerError> {
    Ok(format!(
        "ghostlight-request-{}-{}-{round}",
        encoded_id(&command_id)?,
        purpose_name(purpose)
    ))
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
    use super::*;
    use crate::world::patch::{kernel_speak_entry, kernel_speak_grant};
    use crate::world::{CoverBudget, WorldScaleIntentRef, derive_cover};

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
            holdings: BTreeMap::new(),
            dependencies: BTreeSet::new(),
            incident_routes: Vec::new(),
            authority: BTreeSet::new(),
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
            controls: BTreeSet::new(),
            commitments: Vec::new(),
            pressures: Vec::new(),
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
            holdings: BTreeMap::new(),
            dependencies: BTreeSet::new(),
            incident_routes: Vec::new(),
            authority: BTreeSet::new(),
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
            controls: BTreeSet::new(),
            commitments: Vec::new(),
            pressures: Vec::new(),
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
            holdings: BTreeMap::new(),
            dependencies: BTreeSet::new(),
            incident_routes: vec![first_edge],
            authority: BTreeSet::new(),
            offices_held: Vec::new(),
            offices_granted: Vec::new(),
            redress: Vec::new(),
            knowledge: Vec::new(),
            controls: BTreeSet::new(),
            commitments: Vec::new(),
            pressures: Vec::new(),
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
            holdings: BTreeMap::new(),
            dependencies: BTreeSet::new(),
            incident_routes: vec![second_edge],
            authority: BTreeSet::new(),
            offices_held: Vec::new(),
            offices_granted: Vec::new(),
            redress: Vec::new(),
            knowledge: Vec::new(),
            controls: BTreeSet::new(),
            commitments: Vec::new(),
            pressures: Vec::new(),
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
        unplaced.subject.incident_routes.clear();
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
        custodial.subject.holdings = BTreeMap::from([(tithe, Quantity(7))]);
        custodial.subject.dependencies = BTreeSet::from([DependencyTarget::Route(second_edge)]);
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
        civic.subject.authority = BTreeSet::from([crate::world::AuthorityGrant {
            kind: crate::world::AuthorityKindName("levy".into()),
            over: crate::world::AuthorityTarget::PlaceSubtree(hall),
        }]);
        civic.subject.offices_held = vec![OfficeSnapshot {
            institution: other_id,
            office: crate::world::OfficeName("warden".into()),
            incumbent: Some(actor_id),
            authority: civic.subject.authority.clone(),
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
            },
            source_prose,
        )
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
                            Declaration::Entity(crate::world::EntityDeclaration {
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
                                position: Some(crate::world::Ref::Draft(DraftHandle::new(
                                    "commons",
                                ))),
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
        let runner = ControllerRunner::with_test_ports(mailbox.clone(), port, store, models());
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
        let runner = ControllerRunner::with_test_ports(mailbox.clone(), port, store, models());
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
        let runner = ControllerRunner::with_test_ports(mailbox.clone(), port, store, models());
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
        let runner = ControllerRunner::with_test_ports(mailbox.clone(), port, store, models());
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
        let runner = ControllerRunner::with_test_ports(
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
        );
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
        let recovery_runner = ControllerRunner::with_test_ports(
            mailbox.clone(),
            replay_port.clone(),
            reopened.clone(),
            models(),
        );
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

        let runner = ControllerRunner::open(
            mailbox.clone(),
            connector_endpoint,
            connector_credential,
            runtime_id,
            directory.path().join("controller-work.cc"),
            models,
        )
        .unwrap();

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
        crate::world::EntityId,
        crate::world::EntityId,
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
                            Declaration::Entity(crate::world::EntityDeclaration {
                                handle: DraftHandle::new("commons"),
                                label: "The Commons".into(),
                                kind: EntityKind::Place,
                                container: None,
                            }),
                            Declaration::Entity(crate::world::EntityDeclaration {
                                handle: DraftHandle::new("road"),
                                label: "The Unwalked Road".into(),
                                kind: EntityKind::Place,
                                container: Some(crate::world::Ref::Draft(DraftHandle::new(
                                    "commons",
                                ))),
                            }),
                            Declaration::Route(crate::world::RouteDeclaration {
                                handle: DraftHandle::new("lane"),
                                label: "The Long Lane".into(),
                                from: crate::world::Ref::Draft(DraftHandle::new("commons")),
                                to: crate::world::Ref::Draft(DraftHandle::new("road")),
                                access: crate::world::AccessKind::Public,
                                cost: Cost(1),
                            }),
                            Declaration::Subject(SubjectDeclaration {
                                handle: DraftHandle::new("subject"),
                                label: "Subject".into(),
                                kind: SubjectKind::Person,
                                controller: NewController::NarrativePersona,
                                affordances: kernel_speak_grant(),
                                position: Some(crate::world::Ref::Draft(DraftHandle::new(
                                    "commons",
                                ))),
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
        let jurisdiction = crate::world::JurisdictionKey::PlaceSubtree(commons);
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
        let outcome = runner
            .step(crate::world::JurisdictionKey::Uncovered)
            .await
            .unwrap();
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
        let mut declarations = vec![Declaration::Entity(crate::world::EntityDeclaration {
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
                position: Some(crate::world::Ref::Draft(DraftHandle::new("commons"))),
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
            ControllerRunner::with_test_ports(mailbox.clone(), port, store.clone(), models()),
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
            declarations.push(Declaration::Entity(crate::world::EntityDeclaration {
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
                position: Some(crate::world::Ref::Draft(DraftHandle::new(&format!(
                    "room{index}"
                )))),
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
            },
            ControllerWorkCustody::Owned {
                narrative_commands: 0,
                operational_commands: 1,
                elaboration_commands: 0,
            }
        );
    }
}
