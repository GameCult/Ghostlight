use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const MEMBRANE_SCHEMA: &str = "ghostlight.persona_projection_membrane.v1";
pub const COGNITION_CONTROLLER_SCHEMA: &str = "ghostlight.decision_controller.v1";
pub const PERSONA_TURN_RECEIPT_SCHEMA: &str = "ghostlight.persona_turn_receipt.v3";
pub const RECORD_GAP_TOOL_NAME: &str = "record_gap";
pub const RECORD_GAP_TOOL_CONTRACT: &str = "record_gap(kind: ambiguity | missing_reference | missing_affordance | missing_primitive | unresolved, source_start_byte: integer, source_end_byte: integer, detail: string)";

/// Selects how a decision owner thinks without making any claim about what
/// kind of subject it is or which scope it controls.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DecisionControllerMode {
    /// A Projector supplies lived prose, a prose-only Persona responds, and a
    /// private Interpreter translates that response into typed proposals.
    NarrativePersona,
    /// One non-Persona agent reads a typed view and acts through typed tools.
    OperationalAgent,
}

/// A scope-neutral reference to a decision controller. Subject kind, political
/// scale, and jurisdiction are bindings owned by the world ontology, not by
/// this descriptor.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DecisionControllerDescriptor {
    pub controller_id: String,
    pub mode: DecisionControllerMode,
}

impl DecisionControllerDescriptor {
    pub fn narrative_persona(controller_id: impl Into<String>) -> Self {
        Self {
            controller_id: controller_id.into(),
            mode: DecisionControllerMode::NarrativePersona,
        }
    }

    pub fn operational_agent(controller_id: impl Into<String>) -> Self {
        Self {
            controller_id: controller_id.into(),
            mode: DecisionControllerMode::OperationalAgent,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectorPrompt<'a> {
    pub identity: &'a str,
    pub typed_context: &'a str,
    pub visible_stimulus: &'a str,
    pub domain_guidance: &'a str,
    pub word_budget: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersonaPrompt<'a> {
    pub identity: &'a str,
    pub lived_stream: &'a str,
    pub domain_guidance: &'a str,
    pub word_budget: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct InterpreterPrompt<'a> {
    pub identity: &'a str,
    pub typed_context: &'a str,
    pub lived_stream: &'a str,
    pub persona_output: &'a str,
    pub output_schema: Option<&'a str>,
    pub domain_guidance: &'a str,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperationalAgentPrompt<'a> {
    pub identity: &'a str,
    pub typed_view: &'a str,
    pub available_tools: &'a str,
    pub decision_pressure: &'a str,
    pub domain_guidance: &'a str,
    pub step_budget: usize,
}

pub fn build_projector_prompt(input: &ProjectorPrompt<'_>) -> String {
    format!(
        "<!-- membrane:{MEMBRANE_SCHEMA}:projector -->\nYou are a private Projector. Convert only the permitted typed context and visible stimulus into one compact lived narrative stream. Project what this person perceives, remembers, wants, fears, knows, and explicitly does not know. Anything absent from the permitted context is unavailable: render imagination, suspicion, or speculation as uncertainty, never as remembered or perceived fact. Do not choose actions, emit schemas, expose field names, or claim world effects. Omit decorative recap that does not change the decision.\n\nDomain guidance:\n{guidance}\n\nIdentity:\n{identity}\n\nPermitted typed context:\n{context}\n\nVisible stimulus:\n{stimulus}\n\nUse at most {word_budget} words. Return only the lived narrative stream.",
        identity = input.identity,
        guidance = input.domain_guidance,
        context = input.typed_context,
        stimulus = input.visible_stimulus,
        word_budget = input.word_budget,
    )
}

pub fn build_persona_prompt(input: &PersonaPrompt<'_>) -> String {
    format!(
        "Everything below is prose from inside one person's life. Inhabit that person and respond naturally from what they perceive, remember, want, fear, and believe. Treat asserted perceptions and memories as known. Let anything beyond them appear only as suspicion, imagination, conjecture, or deliberate invention by the character. You may speak, remain silent, wonder, decide, or attempt something. Do not announce an outside consequence as already accomplished. Spend words in proportion to the moment.\n\nVoice and setting guidance:\n{guidance}\n\nWho you are:\n{identity}\n\nWhat this moment is like for you:\n{stream}\n\nUse at most {word_budget} words. Return only the character's natural response in prose.",
        identity = input.identity,
        guidance = input.domain_guidance,
        stream = input.lived_stream,
        word_budget = input.word_budget,
    )
}

pub fn build_interpreter_prompt(input: &InterpreterPrompt<'_>) -> String {
    let proposal_contract = match input.output_schema {
        Some(schema) => format!(
            "Typed proposal payloads must match this current contract:\n{schema}"
        ),
        None => "The harness supplies the current legal typed proposal tools immediately before each step. Use only those current contracts; never reuse a contract remembered from an earlier step.".into(),
    };
    format!(
        "<!-- membrane:{MEMBRANE_SCHEMA}:interpreter -->\nYou are a private Interpreter. Translate a natural Persona turn into zero or more typed candidate proposals supported by the prose and permissioned context. The owning runtime validates and commits proposals; you never claim that a proposed consequence already happened. Do not invent knowledge, capability, custody, perception, identifiers, or state references.\n\nInterpretation is total: this turn cannot fail because some prose has no available translation. The harness has already preserved the Persona turn verbatim as noncanonical source prose. Capture every translation you can justify, citing its exact UTF-8 byte span in that source. Spoken words become a typed speech proposal; wondering, deciding, attempting, and narration are not automatically speech. If a meaningful passage cannot be represented safely, call `{gap_tool}` instead of guessing. A report containing only source prose, or source prose plus gaps, is valid. If the step budget ends, the harness completes the report and records the unresolved source instead of failing.\n\nThe always-available gap tool is:\n{gap_contract}\nUse `ambiguity` when several translations remain live, `missing_reference` when the prose lacks an exact world reference, `missing_affordance` when the subject lacks a permitted way to attempt it, `missing_primitive` when the ontology has no suitable proposal vocabulary, and `unresolved` only when no narrower account fits.\n\nIf a section headed `Interrupted:` follows the Persona turn, the world moved after that prose was written and before it could take effect. The prose is not re-written and not re-authored; translate it again against what that section reports. You may capture a shorter or different span of the same prose, a different permitted action the prose supports, or call `{gap_tool}` with `unresolved` when the intent the prose carried has been overtaken. The reported change is not something this person said, did, or caused.\n\n{proposal_contract}\n\nDomain guidance and exact permissions:\n{guidance}\n\nIdentity:\n{identity}\n\nPermissioned typed context:\n{context}\n\nLived stream:\n{stream}\n\nPersona turn (already preserved verbatim as source evidence):\n{output}",
        identity = input.identity,
        guidance = input.domain_guidance,
        context = input.typed_context,
        stream = input.lived_stream,
        output = input.persona_output,
        gap_tool = RECORD_GAP_TOOL_NAME,
        gap_contract = RECORD_GAP_TOOL_CONTRACT,
    )
}

/// One constituent's block in a batched prompt. Built by calling the caller's
/// existing per-subject view builder once per constituent: there is no union
/// step anywhere in this type's construction, and a fact held by two
/// constituents appears twice, under each of their handles. That duplication is
/// the invariant, not waste.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LabeledView<'a> {
    /// `c0`, `c1`, … — the constituent's index in its cell.
    pub handle: &'a str,
    pub identity: &'a str,
    /// Verbatim per-subject typed view, for this subject alone.
    pub typed_view: &'a str,
    /// This constituent's tool signatures, already handle-prefixed.
    pub tool_signatures: &'a str,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct GroupedAgentPrompt<'a> {
    pub views: &'a [LabeledView<'a>],
    pub decision_pressure: &'a str,
    pub domain_guidance: &'a str,
    pub step_budget: usize,
}

/// A distinct builder, not the singleton builder widened. The singleton prompt
/// must stay byte-identical or every persisted checkpoint's request-shape check
/// fails on resume, and a shared builder with a vector arity is how that drifts
/// by one byte.
///
/// The only shared text is the instruction and the attribution contract. There
/// is no shared header, no cell summary, no "who is present", and no
/// cross-reference between handles: each of those is a union with a friendly
/// name.
pub fn build_grouped_agent_prompt(input: &GroupedAgentPrompt<'_>) -> String {
    let blocks = input
        .views
        .iter()
        .map(|view| {
            format!(
                "### {handle} — {identity}\n{body}\nTools for {handle}: {tools}",
                handle = view.handle,
                identity = view.identity,
                body = view.typed_view,
                tools = view.tool_signatures,
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n");
    format!(
        "<!-- membrane:{MEMBRANE_SCHEMA}:grouped-agent -->\nYou are an operational decision agent deciding for several subjects at once, not a Persona and not a prose roleplay surface. Each subject's view appears under its own handle. A subject knows only what appears under its own handle. Call at most one tool per handle. A handle you were not given does not exist. Do not invent identifiers, permissions, observations, resources, or completed consequences, and never propose for one handle out of another handle's view.\n\nDomain guidance and exact permissions:\n{guidance}\n\nCurrent decision pressure:\n{pressure}\n\n{blocks}\n\nUse at most {step_budget} tool steps, and emit every call you intend to make in the first step. Returning no proposal for a handle is valid.",
        guidance = input.domain_guidance,
        pressure = input.decision_pressure,
        step_budget = input.step_budget,
    )
}

pub fn build_operational_agent_prompt(input: &OperationalAgentPrompt<'_>) -> String {
    format!(
        "<!-- membrane:{MEMBRANE_SCHEMA}:operational-agent -->\nYou are an operational decision agent, not a Persona and not a prose roleplay surface. Read the permissioned typed view directly and use only the supplied typed tools. Choose proposals for this decision owner; the owning runtime validates and commits them. Do not invent identifiers, permissions, observations, resources, or completed consequences. Efficient structured reasoning is appropriate here.\n\nDomain guidance and exact permissions:\n{guidance}\n\nDecision owner:\n{identity}\n\nCurrent decision pressure:\n{pressure}\n\nPermissioned typed view:\n{view}\n\nAvailable typed tools:\n{tools}\n\nUse at most {step_budget} tool steps. Returning no proposal is valid.",
        identity = input.identity,
        guidance = input.domain_guidance,
        pressure = input.decision_pressure,
        view = input.typed_view,
        tools = input.available_tools,
        step_budget = input.step_budget,
    )
}

/// Exact authority references supplied by the NarrativePersona runner when it
/// persists a Persona turn. The world kernel later checks these claims against
/// its own decision opportunity; this projection membrane cannot grant them.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersonaTurnBinding {
    pub world_id: String,
    pub controller_id: String,
    /// Digest of the complete canonical decision opportunity supplied to the
    /// Projector. It binds scope, controller mode, affordances, revision, and
    /// the scope digest as one value instead of inventing a second opportunity
    /// ID.
    pub opportunity_digest: String,
    pub world_revision: u64,
    pub scope_digest: String,
    pub projector_receipt_digest: String,
    pub persona_inference_receipt_digest: String,
    /// The binding this same prose was first receipted under. Present exactly
    /// on a re-lowered turn: the world moved between the prose and its commit,
    /// so the turn's binding was renewed against the fresh opportunity while the
    /// prose and both inference receipts were carried across unchanged. Its own
    /// `interrupted_from` is always `None` — prose is lowered at most twice, so
    /// a chain is not a shape this type admits.
    pub interrupted_from: Option<Box<PersonaTurnBinding>>,
}

/// Immutable source evidence persisted by the NarrativePersona runner before
/// interpretation begins. Its self-derived receipt digest binds every typed
/// reference above to the exact prose; callers cannot pair an unrelated digest
/// and string after construction.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct PersonaTurn {
    binding: PersonaTurnBinding,
    source_prose: String,
    source_digest: String,
    receipt_digest: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PersonaTurnIntegrityError {
    SourceDigestMismatch,
    ReceiptDigestMismatch,
    /// A re-lowered turn whose predecessor was itself re-lowered. One
    /// re-lowering per turn is a property of the type, not a counter.
    InterruptionChained,
}

impl std::fmt::Display for PersonaTurnIntegrityError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SourceDigestMismatch => {
                formatter.write_str("Persona source prose does not match its persisted digest")
            }
            Self::ReceiptDigestMismatch => formatter
                .write_str("Persona turn binding or source does not match its persisted receipt"),
            Self::InterruptionChained => formatter
                .write_str("Persona prose was already re-lowered once and admits no second"),
        }
    }
}

impl std::error::Error for PersonaTurnIntegrityError {}

#[derive(Deserialize)]
struct PersistedPersonaTurn {
    binding: PersonaTurnBinding,
    source_prose: String,
    source_digest: String,
    receipt_digest: String,
}

impl<'de> Deserialize<'de> for PersonaTurn {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let persisted = PersistedPersonaTurn::deserialize(deserializer)?;
        Self::rehydrate(
            persisted.binding,
            persisted.source_prose,
            persisted.source_digest,
            persisted.receipt_digest,
        )
        .map_err(serde::de::Error::custom)
    }
}

impl PersonaTurn {
    pub fn record(binding: PersonaTurnBinding, source_prose: impl Into<String>) -> Self {
        let source_prose = source_prose.into();
        let source_digest = sha256(&source_prose);
        let receipt_digest = persona_turn_receipt_digest(&binding, &source_digest);
        Self {
            binding,
            source_prose,
            source_digest,
            receipt_digest,
        }
    }

    /// Rehydrates an exact persisted turn while refusing to manufacture a
    /// trusted receipt from altered prose or altered authority bindings.
    pub fn rehydrate(
        binding: PersonaTurnBinding,
        source_prose: impl Into<String>,
        source_digest: impl Into<String>,
        receipt_digest: impl Into<String>,
    ) -> Result<Self, PersonaTurnIntegrityError> {
        let source_prose = source_prose.into();
        let source_digest = source_digest.into();
        let receipt_digest = receipt_digest.into();
        if !interruption_is_unchained(&binding) {
            return Err(PersonaTurnIntegrityError::InterruptionChained);
        }
        if sha256(&source_prose) != source_digest {
            return Err(PersonaTurnIntegrityError::SourceDigestMismatch);
        }
        if persona_turn_receipt_digest(&binding, &source_digest) != receipt_digest {
            return Err(PersonaTurnIntegrityError::ReceiptDigestMismatch);
        }
        Ok(Self {
            binding,
            source_prose,
            source_digest,
            receipt_digest,
        })
    }

    pub fn receipt_is_valid(&self) -> bool {
        interruption_is_unchained(&self.binding)
            && sha256(&self.source_prose) == self.source_digest
            && persona_turn_receipt_digest(&self.binding, &self.source_digest)
                == self.receipt_digest
    }

    pub fn binding(&self) -> &PersonaTurnBinding {
        &self.binding
    }

    pub fn source_prose(&self) -> &str {
        &self.source_prose
    }

    pub fn source_digest(&self) -> &str {
        &self.source_digest
    }

    pub fn receipt_digest(&self) -> &str {
        &self.receipt_digest
    }
}

/// An exact UTF-8 byte range in the preserved Persona source prose.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct SourceSpan {
    start_byte: usize,
    end_byte: usize,
    verbatim: String,
}

impl SourceSpan {
    pub fn exact(source: &str, start_byte: usize, end_byte: usize) -> Option<Self> {
        if start_byte >= end_byte {
            return None;
        }
        source.get(start_byte..end_byte).map(|verbatim| Self {
            start_byte,
            end_byte,
            verbatim: verbatim.to_owned(),
        })
    }

    pub fn whole(source: &str) -> Self {
        Self {
            start_byte: 0,
            end_byte: source.len(),
            verbatim: source.to_owned(),
        }
    }

    pub fn is_exact_in(&self, source: &str) -> bool {
        source.get(self.start_byte..self.end_byte) == Some(self.verbatim.as_str())
    }

    pub fn start_byte(&self) -> usize {
        self.start_byte
    }

    pub fn end_byte(&self) -> usize {
        self.end_byte
    }

    pub fn verbatim(&self) -> &str {
        &self.verbatim
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct TypedProposalCapture<T> {
    /// The owning runtime's typed proposal enum. This membrane never converts
    /// an opaque string or JSON blob into world authority.
    proposal: T,
    source: SourceSpan,
}

impl<T> TypedProposalCapture<T> {
    pub fn proposal(&self) -> &T {
        &self.proposal
    }

    pub fn source(&self) -> &SourceSpan {
        &self.source
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TranslationGapKind {
    Ambiguity,
    MissingReference,
    MissingAffordance,
    MissingPrimitive,
    Unresolved,
}

/// The typed form of the always-available `record_gap` tool call.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecordGapToolCall {
    pub kind: TranslationGapKind,
    pub source_start_byte: usize,
    pub source_end_byte: usize,
    pub detail: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct TranslationGap {
    kind: TranslationGapKind,
    source: SourceSpan,
    detail: String,
}

impl TranslationGap {
    pub fn kind(&self) -> TranslationGapKind {
        self.kind
    }

    pub fn source(&self) -> &SourceSpan {
        &self.source
    }

    pub fn detail(&self) -> &str {
        &self.detail
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InterpretationFinalization {
    InterpreterFinished,
    StepBudgetExhausted,
}

/// A total interpretation result. Both finalization variants are completed
/// reports; the variant records how capture stopped, not success or failure.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct InterpretationReport<T> {
    source: PersonaTurn,
    proposals: Vec<TypedProposalCapture<T>>,
    gaps: Vec<TranslationGap>,
    finalization: InterpretationFinalization,
}

/// Immediate feedback from a capture tool. Malformed evidence never rejects
/// the turn: the harness records an exact whole-source gap and reports that
/// fallback to the current Interpreter step.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CaptureToolFeedback {
    Accepted,
    RecordedAsGap { detail: String },
}

impl<T> InterpretationReport<T> {
    pub fn source(&self) -> &PersonaTurn {
        &self.source
    }

    pub fn proposals(&self) -> &[TypedProposalCapture<T>] {
        &self.proposals
    }

    pub fn gaps(&self) -> &[TranslationGap] {
        &self.gaps
    }

    pub fn finalization(&self) -> InterpretationFinalization {
        self.finalization
    }

    pub fn spans_are_exact(&self) -> bool {
        self.proposals
            .iter()
            .all(|proposal| proposal.source.is_exact_in(self.source.source_prose()))
            && self
                .gaps
                .iter()
                .all(|gap| gap.source.is_exact_in(self.source.source_prose()))
    }
}

/// Accumulates tool captures without giving the Interpreter a terminal error
/// state. Invalid tool arguments become exact gaps and never discard prior
/// work or source prose.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InterpretationAccumulator<T> {
    source: PersonaTurn,
    proposals: Vec<TypedProposalCapture<T>>,
    gaps: Vec<TranslationGap>,
}

impl<T> InterpretationAccumulator<T> {
    pub fn new(source: PersonaTurn) -> Self {
        Self {
            source,
            proposals: Vec::new(),
            gaps: Vec::new(),
        }
    }

    pub fn capture_proposal(
        &mut self,
        proposal: T,
        source_start_byte: usize,
        source_end_byte: usize,
    ) -> CaptureToolFeedback {
        let Some(source) = SourceSpan::exact(
            self.source.source_prose(),
            source_start_byte,
            source_end_byte,
        ) else {
            let detail = format!(
                "typed proposal could not be bound to source range {source_start_byte}..{source_end_byte}; its meaning remains untranslated"
            );
            self.record_whole_source_gap(TranslationGapKind::Unresolved, detail.clone());
            return CaptureToolFeedback::RecordedAsGap { detail };
        };
        self.proposals
            .push(TypedProposalCapture { proposal, source });
        CaptureToolFeedback::Accepted
    }

    pub fn record_gap(&mut self, call: RecordGapToolCall) -> CaptureToolFeedback {
        let Some(source) = SourceSpan::exact(
            self.source.source_prose(),
            call.source_start_byte,
            call.source_end_byte,
        ) else {
            let detail = format!(
                "{} [requested source range {}..{} was not exact, so the harness bound this gap to the complete source prose]",
                call.detail, call.source_start_byte, call.source_end_byte
            );
            self.record_whole_source_gap(call.kind, detail.clone());
            return CaptureToolFeedback::RecordedAsGap { detail };
        };
        self.gaps.push(TranslationGap {
            kind: call.kind,
            source,
            detail: call.detail,
        });
        CaptureToolFeedback::Accepted
    }

    /// Total ingress for a model action that could not be decoded into the
    /// current typed proposal or `record_gap` contract. The raw payload stays
    /// noncanonical attempt evidence; its digest makes the fallback traceable
    /// without asking a second model to repair it.
    pub fn record_tool_decode_failure(
        &mut self,
        tool_name: &str,
        raw_arguments: &str,
        decode_detail: &str,
    ) -> CaptureToolFeedback {
        let detail = format!(
            "tool `{tool_name}` arguments could not be decoded ({decode_detail}); raw_argument_digest={}",
            sha256(raw_arguments)
        );
        self.record_whole_source_gap(TranslationGapKind::Unresolved, detail.clone());
        CaptureToolFeedback::RecordedAsGap { detail }
    }

    fn record_whole_source_gap(&mut self, kind: TranslationGapKind, detail: String) {
        self.gaps.push(TranslationGap {
            kind,
            source: SourceSpan::whole(self.source.source_prose()),
            detail,
        });
    }

    pub fn finalize(mut self, finalization: InterpretationFinalization) -> InterpretationReport<T> {
        if finalization == InterpretationFinalization::StepBudgetExhausted {
            self.record_whole_source_gap(
                TranslationGapKind::Unresolved,
                "Interpreter step budget ended before explicit completion; the complete source prose remains unresolved evidence.".into(),
            );
        }
        InterpretationReport {
            source: self.source,
            proposals: self.proposals,
            gaps: self.gaps,
            finalization,
        }
    }

    /// Discards partial captures after an infrastructure interruption and
    /// yields the immutable source for a fresh attempt.
    pub fn into_pending_source(self) -> PersonaTurn {
        self.source
    }
}

pub fn sha256(value: &str) -> String {
    format!("sha256:{:x}", Sha256::digest(value.as_bytes()))
}

/// One re-lowering, expressed as a shape rather than a count: the binding a
/// re-lowered turn replaced must itself be an original.
fn interruption_is_unchained(binding: &PersonaTurnBinding) -> bool {
    binding
        .interrupted_from
        .as_ref()
        .is_none_or(|prior| prior.interrupted_from.is_none())
}

fn persona_turn_receipt_digest(binding: &PersonaTurnBinding, source_digest: &str) -> String {
    // The prior binding's receipt over the *same* source digest is exactly the
    // receipt the first turn published, so a re-lowered turn's receipt commits
    // to the turn it replaced by value.
    let interrupted_from = binding
        .interrupted_from
        .as_ref()
        .map(|prior| persona_turn_receipt_digest(prior, source_digest))
        .unwrap_or_default();
    let mut digest = Sha256::new();
    for value in [
        PERSONA_TURN_RECEIPT_SCHEMA.as_bytes(),
        binding.world_id.as_bytes(),
        binding.controller_id.as_bytes(),
        binding.opportunity_digest.as_bytes(),
        binding.scope_digest.as_bytes(),
        binding.projector_receipt_digest.as_bytes(),
        binding.persona_inference_receipt_digest.as_bytes(),
        interrupted_from.as_bytes(),
        source_digest.as_bytes(),
    ] {
        digest.update((value.len() as u64).to_le_bytes());
        digest.update(value);
    }
    digest.update(binding.world_revision.to_le_bytes());
    format!("sha256:{:x}", digest.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Debug, PartialEq, Eq)]
    enum TestProposal {
        Speak {
            recipient_id: &'static str,
            utterance: &'static str,
        },
    }

    fn turn(source_prose: &str) -> PersonaTurn {
        let binding = PersonaTurnBinding {
            world_id: "world:test".into(),
            controller_id: "controller:mara".into(),
            opportunity_digest: "sha256:complete-opportunity".into(),
            world_revision: 12,
            scope_digest: "sha256:world-state".into(),
            projector_receipt_digest: "sha256:projector-receipt".into(),
            persona_inference_receipt_digest: "sha256:persona-inference-receipt".into(),
            interrupted_from: None,
        };
        PersonaTurn::record(binding, source_prose)
    }

    #[test]
    fn persona_prompt_is_prose_only() {
        let prompt = build_persona_prompt(&PersonaPrompt {
            identity: "John, an exhausted village smith.",
            lived_stream: "The forge is hot and the traveler looks worried.",
            domain_guidance: "Speak as a villager with dry patience.",
            word_budget: 140,
        });
        let lower = prompt.to_lowercase();
        let words: Vec<_> = lower
            .split(|character: char| !character.is_alphanumeric())
            .filter(|word| !word.is_empty())
            .collect();
        assert!(prompt.contains("The forge is hot"));
        for machine_word in [
            "id",
            "ids",
            "identifier",
            "json",
            "schema",
            "tool",
            "tools",
            "stage",
            "membrane",
            "typed",
            "contract",
        ] {
            assert!(
                !words.contains(&machine_word),
                "Persona-visible prompt leaked machine word `{machine_word}`"
            );
        }
        assert!(!prompt.contains("<!--"));
        assert!(!prompt.contains("world:test"));
        assert!(!prompt.contains("controller:mara"));
        assert!(!prompt.contains("sha256:"));
    }

    #[test]
    fn persisted_exact_turn_rehydrates_with_the_same_receipt() {
        let recorded = turn("I tell Mara, \"The western brace is giving way.\"");
        let restored = PersonaTurn::rehydrate(
            recorded.binding().clone(),
            recorded.source_prose(),
            recorded.source_digest(),
            recorded.receipt_digest(),
        )
        .expect("exact persisted turn must rehydrate");

        assert_eq!(restored, recorded);
        assert_eq!(restored.receipt_digest(), recorded.receipt_digest());
        assert!(restored.receipt_is_valid());
    }

    #[test]
    fn rehydration_rejects_altered_prose_or_opportunity_binding() {
        let recorded = turn("I tell Mara the western brace is giving way.");

        assert_eq!(
            PersonaTurn::rehydrate(
                recorded.binding().clone(),
                "I tell Mara the bridge is perfectly safe.",
                recorded.source_digest(),
                recorded.receipt_digest(),
            ),
            Err(PersonaTurnIntegrityError::SourceDigestMismatch)
        );

        let mut altered_binding = recorded.binding().clone();
        altered_binding.opportunity_digest = "sha256:another-complete-opportunity".into();
        assert_eq!(
            PersonaTurn::rehydrate(
                altered_binding,
                recorded.source_prose(),
                recorded.source_digest(),
                recorded.receipt_digest(),
            ),
            Err(PersonaTurnIntegrityError::ReceiptDigestMismatch)
        );
    }

    #[test]
    fn a_re_lowered_turn_commits_to_the_receipt_it_replaced_and_admits_no_chain() {
        let source = "I say, \"The western brace is giving way.\"";
        let first = turn(source);
        let mut renewed = first.binding().clone();
        renewed.scope_digest = "sha256:world-state-moved".into();
        renewed.interrupted_from = Some(Box::new(first.binding().clone()));
        let second = PersonaTurn::record(renewed.clone(), source);

        // The prose and both inference receipts are carried, never recomputed.
        assert_eq!(second.source_prose(), first.source_prose());
        assert_eq!(second.source_digest(), first.source_digest());
        assert_eq!(
            second.binding().projector_receipt_digest,
            first.binding().projector_receipt_digest
        );
        assert_eq!(
            second.binding().persona_inference_receipt_digest,
            first.binding().persona_inference_receipt_digest
        );
        // The second receipt commits to the first by value.
        assert_ne!(second.receipt_digest(), first.receipt_digest());
        assert_eq!(
            second.binding().interrupted_from.as_deref(),
            Some(first.binding())
        );
        assert!(second.receipt_is_valid());

        let mut chained = renewed.clone();
        chained.interrupted_from = Some(Box::new(renewed));
        let third = PersonaTurn::record(chained.clone(), source);
        assert!(!third.receipt_is_valid());
        assert_eq!(
            PersonaTurn::rehydrate(
                chained,
                third.source_prose(),
                third.source_digest(),
                third.receipt_digest(),
            ),
            Err(PersonaTurnIntegrityError::InterruptionChained)
        );
    }

    /// The receipt schema bumped because the binding's fields changed, and the
    /// pass keeps no compatibility path. A v2 receipt is rebuilt here exactly as
    /// the previous version computed it — the old schema string, and no
    /// `interrupted_from` value in the preimage — and it must not rehydrate.
    /// Written out rather than derived from the live function, so a future edit
    /// to the digest cannot quietly move both sides of the comparison together.
    #[test]
    fn a_receipt_written_before_this_pass_does_not_rehydrate() {
        let source = "I say, \"The western brace is giving way.\"";
        let live = turn(source);
        let binding = live.binding();
        let mut digest = Sha256::new();
        for value in [
            "ghostlight.persona_turn_receipt.v2".as_bytes(),
            binding.world_id.as_bytes(),
            binding.controller_id.as_bytes(),
            binding.opportunity_digest.as_bytes(),
            binding.scope_digest.as_bytes(),
            binding.projector_receipt_digest.as_bytes(),
            binding.persona_inference_receipt_digest.as_bytes(),
            live.source_digest().as_bytes(),
        ] {
            digest.update((value.len() as u64).to_le_bytes());
            digest.update(value);
        }
        let v2 = format!("sha256:{:x}", digest.finalize());

        assert_eq!(
            PERSONA_TURN_RECEIPT_SCHEMA,
            "ghostlight.persona_turn_receipt.v3"
        );
        assert_ne!(v2, live.receipt_digest(), "the bump moved no bytes");
        assert_eq!(
            PersonaTurn::rehydrate(
                binding.clone(),
                live.source_prose(),
                live.source_digest(),
                &v2,
            ),
            Err(PersonaTurnIntegrityError::ReceiptDigestMismatch)
        );
    }

    #[test]
    fn interpretation_is_total_at_step_exhaustion() {
        let source = "I say, \"Mara, the bridge is unsafe.\" Then I try the rusted western gate.";
        let mut interpretation = InterpretationAccumulator::new(turn(source));
        let warning = "Mara, the bridge is unsafe.";
        let warning_start = source.find(warning).unwrap();
        assert_eq!(
            interpretation.capture_proposal(
                TestProposal::Speak {
                    recipient_id: "actor:mara",
                    utterance: "Mara, the bridge is unsafe.",
                },
                warning_start,
                warning_start + warning.len(),
            ),
            CaptureToolFeedback::Accepted
        );
        let gate_start = source.find("try the rusted western gate").unwrap();
        assert_eq!(
            interpretation.record_gap(RecordGapToolCall {
                kind: TranslationGapKind::MissingReference,
                source_start_byte: gate_start,
                source_end_byte: gate_start + "try the rusted western gate".len(),
                detail: "No exact gate id is visible in the permissioned context.".into(),
            }),
            CaptureToolFeedback::Accepted
        );

        let report = interpretation.finalize(InterpretationFinalization::StepBudgetExhausted);
        assert_eq!(report.source().source_prose(), source);
        assert_eq!(report.proposals().len(), 1);
        assert_eq!(report.gaps().len(), 2);
        assert_eq!(
            report.finalization(),
            InterpretationFinalization::StepBudgetExhausted
        );
        assert_eq!(
            report.gaps()[0].kind(),
            TranslationGapKind::MissingReference
        );
        assert_eq!(report.gaps()[1].kind(), TranslationGapKind::Unresolved);
        assert!(report.spans_are_exact());
    }

    #[test]
    fn invalid_gap_offsets_become_exact_gaps_instead_of_losing_meaning() {
        let speech = "I appeal to whoever can hear me.";
        let mut interpretation = InterpretationAccumulator::<TestProposal>::new(turn(speech));
        let feedback = interpretation.record_gap(RecordGapToolCall {
            kind: TranslationGapKind::MissingAffordance,
            source_start_byte: 900,
            source_end_byte: 940,
            detail: "No appeal channel is available.".into(),
        });

        assert!(matches!(
            feedback,
            CaptureToolFeedback::RecordedAsGap { .. }
        ));
        let report = interpretation.finalize(InterpretationFinalization::StepBudgetExhausted);
        assert_eq!(report.proposals().len(), 0);
        assert_eq!(report.gaps().len(), 2);
        assert_eq!(report.gaps()[0].source().verbatim(), speech);
        assert_eq!(report.gaps()[1].kind(), TranslationGapKind::Unresolved);
        assert_eq!(report.source().source_prose(), speech);
        assert!(report.spans_are_exact());
    }

    #[test]
    fn invalid_proposal_evidence_is_preserved_as_an_unresolved_gap() {
        let source = "I tell Mara the bridge is unsafe.";
        let mut interpretation = InterpretationAccumulator::new(turn(source));
        assert!(matches!(
            interpretation.capture_proposal(
                TestProposal::Speak {
                    recipient_id: "actor:mara",
                    utterance: "warning",
                },
                700,
                740,
            ),
            CaptureToolFeedback::RecordedAsGap { .. }
        ));

        let report = interpretation.finalize(InterpretationFinalization::InterpreterFinished);
        assert!(report.proposals().is_empty());
        assert_eq!(report.gaps().len(), 1);
        assert_eq!(report.gaps()[0].kind(), TranslationGapKind::Unresolved);
        assert_eq!(report.gaps()[0].source().verbatim(), source);
        assert!(report.spans_are_exact());
    }

    #[test]
    fn source_only_interpretation_is_a_completed_value() {
        let source = "The rain sounds different against copper.";
        let report = InterpretationAccumulator::<TestProposal>::new(turn(source))
            .finalize(InterpretationFinalization::InterpreterFinished);

        assert_eq!(report.source().source_prose(), source);
        assert_eq!(report.source().source_digest(), sha256(source));
        assert!(report.source().receipt_digest().starts_with("sha256:"));
        assert_eq!(report.source().binding().world_revision, 12);
        assert!(report.proposals().is_empty());
        assert!(report.gaps().is_empty());
        assert_eq!(
            report.finalization(),
            InterpretationFinalization::InterpreterFinished
        );
    }

    #[test]
    fn raw_tool_decode_failure_becomes_an_exact_gap() {
        let source = "I reach for the unfamiliar latch.";
        let mut interpretation = InterpretationAccumulator::<TestProposal>::new(turn(source));
        assert!(matches!(
            interpretation.record_tool_decode_failure(
                "attempt_action",
                r#"{"target":17,"oops"}"#,
                "unexpected end of input",
            ),
            CaptureToolFeedback::RecordedAsGap { .. }
        ));

        let report = interpretation.finalize(InterpretationFinalization::InterpreterFinished);
        assert_eq!(report.gaps().len(), 1);
        assert_eq!(report.gaps()[0].kind(), TranslationGapKind::Unresolved);
        assert_eq!(report.gaps()[0].source().verbatim(), source);
        assert!(
            report.gaps()[0]
                .detail()
                .contains("raw_argument_digest=sha256:")
        );
    }

    #[test]
    fn infrastructure_interruption_leaves_the_turn_pending() {
        let source = "I say, \"Wait here.\"";
        let mut interpretation = InterpretationAccumulator::new(turn(source));
        let utterance = "Wait here.";
        let start = source.find(utterance).unwrap();
        assert_eq!(
            interpretation.capture_proposal(
                TestProposal::Speak {
                    recipient_id: "actor:mara",
                    utterance: "Wait here.",
                },
                start,
                start + utterance.len(),
            ),
            CaptureToolFeedback::Accepted
        );

        let pending = interpretation.into_pending_source();
        assert_eq!(pending.source_prose(), source);
    }

    #[test]
    fn controller_modes_are_explicit_and_scope_neutral() {
        let persona = DecisionControllerDescriptor::narrative_persona("controller:john");
        let gestalt = DecisionControllerDescriptor::operational_agent("controller:rail-council");
        assert_eq!(persona.mode, DecisionControllerMode::NarrativePersona);
        assert_eq!(gestalt.mode, DecisionControllerMode::OperationalAgent);
        assert_ne!(persona, gestalt);
    }

    #[test]
    fn operational_agent_is_a_distinct_typed_surface() {
        let prompt = build_operational_agent_prompt(&OperationalAgentPrompt {
            identity: "The Rail Council gestalt",
            typed_view: "jurisdiction=region:capital; pressure=shortage",
            available_tools: "allocate(resource_id, destination_id, amount)",
            decision_pressure: "Restore grain flow without breaking custody.",
            domain_guidance: "Prefer reversible allocations.",
            step_budget: 4,
        });
        assert!(prompt.contains("not a Persona"));
        assert!(prompt.contains("Permissioned typed view"));
        assert!(prompt.contains("Available typed tools"));
    }
}
