use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const MEMBRANE_SCHEMA: &str = "ghostlight.persona_projection_membrane.v1";
pub const COGNITION_CONTROLLER_SCHEMA: &str = "ghostlight.decision_controller.v1";
pub const RECORD_GAP_TOOL_NAME: &str = "record_gap";
pub const RECORD_GAP_TOOL_CONTRACT: &str = "record_gap(kind: ambiguity | missing_reference | missing_affordance | missing_primitive, source_start_byte: integer, source_end_byte: integer, detail: string)";

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
        "<!-- membrane:{MEMBRANE_SCHEMA}:persona -->\nEverything below is prose from inside one person's life. Inhabit that person and respond naturally from what they perceive, remember, want, fear, and believe. Treat asserted perceptions and memories as known. Let anything beyond them appear only as suspicion, imagination, conjecture, or deliberate invention by the character. You may speak, remain silent, wonder, decide, or attempt something. Do not announce an outside consequence as already accomplished. Spend words in proportion to the moment.\n\nVoice and setting guidance:\n{guidance}\n\nWho you are:\n{identity}\n\nWhat this moment is like for you:\n{stream}\n\nUse at most {word_budget} words. Return only the character's natural response in prose.",
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
        "<!-- membrane:{MEMBRANE_SCHEMA}:interpreter -->\nYou are a private Interpreter. Translate a natural Persona turn into zero or more typed candidate proposals supported by the prose and permissioned context. The owning runtime validates and commits proposals; you never claim that a proposed consequence already happened. Do not invent knowledge, capability, custody, perception, identifiers, or state references.\n\nInterpretation is total: this turn cannot fail because some prose has no available translation. Preserve the Persona turn verbatim as speech. Capture every translation you can justify, citing its exact UTF-8 byte span in the Persona turn. If a meaningful passage cannot be represented safely, call `{gap_tool}` instead of guessing. A report containing only preserved speech, or preserved speech plus gaps, is valid. If the step budget ends, all proposals and gaps already captured remain part of the completed report.\n\nThe always-available gap tool is:\n{gap_contract}\nUse `ambiguity` when several translations remain live, `missing_reference` when the prose lacks an exact world reference, `missing_affordance` when the subject lacks a permitted way to attempt it, and `missing_primitive` when the ontology has no suitable proposal vocabulary.\n\n{proposal_contract}\n\nDomain guidance and exact permissions:\n{guidance}\n\nIdentity:\n{identity}\n\nPermissioned typed context:\n{context}\n\nLived stream:\n{stream}\n\nPersona turn (preserve verbatim in the report):\n{output}",
        identity = input.identity,
        guidance = input.domain_guidance,
        context = input.typed_context,
        stream = input.lived_stream,
        output = input.persona_output,
        gap_tool = RECORD_GAP_TOOL_NAME,
        gap_contract = RECORD_GAP_TOOL_CONTRACT,
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

/// An exact UTF-8 byte range in the preserved Persona response.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceSpan {
    pub start_byte: usize,
    pub end_byte: usize,
    pub verbatim: String,
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
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TypedProposalCapture<T> {
    /// The owning runtime's typed proposal enum. This membrane never converts
    /// an opaque string or JSON blob into world authority.
    pub proposal: T,
    pub source: SourceSpan,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TranslationGapKind {
    Ambiguity,
    MissingReference,
    MissingAffordance,
    MissingPrimitive,
}

/// The typed form of the always-available `record_gap` tool call.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecordGapToolCall {
    pub kind: TranslationGapKind,
    pub source_start_byte: usize,
    pub source_end_byte: usize,
    pub detail: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TranslationGap {
    pub kind: TranslationGapKind,
    pub source: SourceSpan,
    pub detail: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InterpretationFinalization {
    InterpreterFinished,
    StepBudgetExhausted,
}

/// A total interpretation result. Both finalization variants are completed
/// reports; the variant records how capture stopped, not success or failure.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct InterpretationReport<T> {
    pub preserved_speech: String,
    pub proposals: Vec<TypedProposalCapture<T>>,
    pub gaps: Vec<TranslationGap>,
    pub finalization: InterpretationFinalization,
}

/// Immediate feedback from a capture tool. Argument rejection is feedback to
/// the Interpreter's current step, not a failed interpretation and not a
/// semantic `TranslationGap`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CaptureToolFeedback {
    Accepted,
    RejectedArguments { detail: String },
}

impl<T> InterpretationReport<T> {
    pub fn spans_are_exact(&self) -> bool {
        self.proposals
            .iter()
            .all(|proposal| proposal.source.is_exact_in(&self.preserved_speech))
            && self
                .gaps
                .iter()
                .all(|gap| gap.source.is_exact_in(&self.preserved_speech))
    }
}

/// Accumulates tool captures without giving the Interpreter a terminal error
/// state. Invalid tool arguments receive local feedback and never discard
/// prior work.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InterpretationAccumulator<T> {
    preserved_speech: String,
    proposals: Vec<TypedProposalCapture<T>>,
    gaps: Vec<TranslationGap>,
}

impl<T> InterpretationAccumulator<T> {
    pub fn new(persona_output: impl Into<String>) -> Self {
        Self {
            preserved_speech: persona_output.into(),
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
        let Some(source) =
            SourceSpan::exact(&self.preserved_speech, source_start_byte, source_end_byte)
        else {
            return CaptureToolFeedback::RejectedArguments {
                detail: format!(
                    "proposal capture supplied a non-exact source range {source_start_byte}..{source_end_byte}"
                ),
            };
        };
        self.proposals
            .push(TypedProposalCapture { proposal, source });
        CaptureToolFeedback::Accepted
    }

    pub fn record_gap(&mut self, call: RecordGapToolCall) -> CaptureToolFeedback {
        let Some(source) = SourceSpan::exact(
            &self.preserved_speech,
            call.source_start_byte,
            call.source_end_byte,
        ) else {
            return CaptureToolFeedback::RejectedArguments {
                detail: format!(
                    "record_gap supplied a non-exact source range {}..{}",
                    call.source_start_byte, call.source_end_byte
                ),
            };
        };
        self.gaps.push(TranslationGap {
            kind: call.kind,
            source,
            detail: call.detail,
        });
        CaptureToolFeedback::Accepted
    }

    pub fn finalize(self, finalization: InterpretationFinalization) -> InterpretationReport<T> {
        InterpretationReport {
            preserved_speech: self.preserved_speech,
            proposals: self.proposals,
            gaps: self.gaps,
            finalization,
        }
    }
}

pub fn narrative_stream_is_clean(value: &str) -> bool {
    let trimmed = value.trim();
    !trimmed.is_empty()
        && !trimmed.starts_with('{')
        && !trimmed.starts_with('[')
        && !trimmed.contains("```json")
        && !trimmed.contains("STATE NOTE")
        && !trimmed.contains("SAY {")
}

pub fn sha256(value: &str) -> String {
    format!("sha256:{:x}", Sha256::digest(value.as_bytes()))
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

    #[test]
    fn persona_prompt_is_prose_only() {
        let prompt = build_persona_prompt(&PersonaPrompt {
            identity: "John, an exhausted village smith.",
            lived_stream: "The forge is hot and the traveler looks worried.",
            domain_guidance: "Speak as a villager with dry patience.",
            word_budget: 140,
        });
        let lower = prompt.to_lowercase();
        assert!(prompt.contains("The forge is hot"));
        assert!(!lower.contains("typed context"));
        assert!(!lower.contains("schema"));
        assert!(!lower.contains("json"));
        assert!(!lower.contains("tool"));
    }

    #[test]
    fn interpretation_is_total_at_step_exhaustion() {
        let speech = "I warn Mara, then try the rusted western gate.";
        let mut interpretation = InterpretationAccumulator::new(speech);
        let warning_start = speech.find("warn Mara").unwrap();
        assert_eq!(
            interpretation.capture_proposal(
                TestProposal::Speak {
                    recipient_id: "actor:mara",
                    utterance: "warning",
                },
                warning_start,
                warning_start + "warn Mara".len(),
            ),
            CaptureToolFeedback::Accepted
        );
        let gate_start = speech.find("try the rusted western gate").unwrap();
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
        assert_eq!(report.preserved_speech, speech);
        assert_eq!(report.proposals.len(), 1);
        assert_eq!(report.gaps.len(), 1);
        assert_eq!(
            report.finalization,
            InterpretationFinalization::StepBudgetExhausted
        );
        assert!(report.spans_are_exact());
    }

    #[test]
    fn invalid_gap_offsets_are_local_feedback_not_interpretation_failure() {
        let speech = "I appeal to whoever can hear me.";
        let mut interpretation = InterpretationAccumulator::<TestProposal>::new(speech);
        let feedback = interpretation.record_gap(RecordGapToolCall {
            kind: TranslationGapKind::MissingAffordance,
            source_start_byte: 900,
            source_end_byte: 940,
            detail: "No appeal channel is available.".into(),
        });

        assert!(matches!(
            feedback,
            CaptureToolFeedback::RejectedArguments { .. }
        ));
        let report = interpretation.finalize(InterpretationFinalization::StepBudgetExhausted);
        assert_eq!(report.proposals.len(), 0);
        assert_eq!(report.gaps.len(), 0);
        assert_eq!(report.preserved_speech, speech);
        assert!(report.spans_are_exact());
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

    #[test]
    fn projector_stream_rejects_schema_and_action_leaks() {
        assert!(narrative_stream_is_clean("The forge smells of coal."));
        assert!(!narrative_stream_is_clean(r#"{"conditions":[]}"#));
        assert!(!narrative_stream_is_clean("SAY { channel: room }"));
    }
}
