use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const MEMBRANE_SCHEMA: &str = "ghostlight.persona_projection_membrane.v1";

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
    pub output_schema: &'a str,
    pub domain_guidance: &'a str,
}

pub fn build_projector_prompt(input: &ProjectorPrompt<'_>) -> String {
    format!(
        "<!-- membrane:{MEMBRANE_SCHEMA}:projector -->\nYou are a private Projector. Convert only the permitted typed context and visible stimulus into one compact lived narrative stream. Project what this person perceives, remembers, wants, fears, knows, and explicitly does not know. Anything absent from the permitted context is unavailable: render imagination, suspicion, or speculation as uncertainty, never as remembered or perceived fact. Do not choose actions, emit schemas, expose field names, or claim world effects. Use at most {word_budget} words; omit decorative recap that does not change the decision.\n\nDomain guidance:\n{guidance}\n\nIdentity:\n{identity}\n\nPermitted typed context:\n{context}\n\nVisible stimulus:\n{stimulus}\n\nReturn only the lived narrative stream.",
        identity = input.identity,
        guidance = input.domain_guidance,
        context = input.typed_context,
        stimulus = input.visible_stimulus,
        word_budget = input.word_budget,
    )
}

pub fn build_persona_prompt(input: &PersonaPrompt<'_>) -> String {
    format!(
        "<!-- membrane:{MEMBRANE_SCHEMA}:persona -->\nThe text below is the character's complete lived stream for this turn. Respond naturally from inside it. Treat only asserted perceptions and memories in that stream as known. New external details may appear only as explicit conjecture, imagination, or deliberate invention by the character. You may speak, remain silent, wonder, decide, or attempt something, but do not emit JSON, schemas, action DSL, tool calls, state patches, or claims that an external consequence already occurred. Use at most {word_budget} words and spend them in proportion to the moment.\n\nDomain guidance:\n{guidance}\n\nIdentity:\n{identity}\n\nLived stream:\n{stream}\n\nReturn only the natural Persona turn.",
        identity = input.identity,
        guidance = input.domain_guidance,
        stream = input.lived_stream,
        word_budget = input.word_budget,
    )
}

pub fn build_interpreter_prompt(input: &InterpreterPrompt<'_>) -> String {
    format!(
        "<!-- membrane:{MEMBRANE_SCHEMA}:interpreter -->\nYou are a private Interpreter. Convert a natural Persona turn into typed candidate effects supported by the lived stream and permissioned typed context. Candidates are proposals only; the owning runtime validates and commits them. Do not invent knowledge, capability, custody, perception, identifiers, state references, or completed consequences.\n\nReturn exactly one JSON object matching this stable shape:\n{schema}\n\nDomain guidance and exact permissions:\n{guidance}\n\nIdentity:\n{identity}\n\nPermissioned typed context:\n{context}\n\nLived stream:\n{stream}\n\nPersona turn:\n{output}",
        identity = input.identity,
        guidance = input.domain_guidance,
        context = input.typed_context,
        stream = input.lived_stream,
        output = input.persona_output,
        schema = input.output_schema,
    )
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

    #[test]
    fn persona_prompt_has_only_one_context_authority() {
        let prompt = build_persona_prompt(&PersonaPrompt {
            identity: "John",
            lived_stream: "The forge is hot and the traveler looks worried.",
            domain_guidance: "Speak as a villager.",
            word_budget: 140,
        });
        assert!(prompt.contains("The forge is hot"));
        assert!(!prompt.contains("typed context"));
        assert!(!prompt.contains("output schema"));
    }

    #[test]
    fn projector_stream_rejects_schema_and_action_leaks() {
        assert!(narrative_stream_is_clean("The forge smells of coal."));
        assert!(!narrative_stream_is_clean(r#"{"conditions":[]}"#));
        assert!(!narrative_stream_is_clean("SAY { channel: room }"));
    }
}
