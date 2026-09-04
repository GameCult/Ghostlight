//! The one emitter for every model-facing tool schema in the tree.
//!
//! Two catalogs sit above it and share nothing else: the action catalog
//! (`controllers::catalog_tools`) projects a world-state snapshot filtered by
//! grant, and the patch catalog (`patch::patch_tools`) projects the reducer's
//! own vocabulary. Both spell a property the same way because both spell it
//! here. Nothing in this module reads world state, a snapshot, or a config.

use codex_connector::CodexToolDefinition;
use serde_json::{Value, json};

pub(super) fn tool(name: &str, description: &str, schema: Value) -> CodexToolDefinition {
    CodexToolDefinition {
        name: name.into(),
        description: description.into(),
        parameters_json: serde_json::to_string(&schema)
            .expect("static tool schemas must serialize"),
    }
}

pub(super) fn empty_schema() -> Value {
    json!({"type":"object","additionalProperties":false,"properties":{}})
}

/// Every generated object is closed and total: `additionalProperties` is false
/// and every property is required. An optional argument is expressed as a
/// nullable property, never as an absent one, so the decoder and the schema
/// agree without a second rule about omission.
pub(super) fn object(properties: Vec<(String, Value)>) -> Value {
    let required: Vec<Value> = properties
        .iter()
        .map(|(name, _)| Value::String(name.clone()))
        .collect();
    json!({
        "type": "object",
        "additionalProperties": false,
        "required": required,
        "properties": properties.into_iter().collect::<serde_json::Map<_, _>>(),
    })
}

pub(super) fn canonical_string(description: &str) -> Value {
    json!({"type": "string", "description": description})
}

pub(super) fn bounded_integer(min: u64, max: u64) -> Value {
    json!({"type": "integer", "minimum": min, "maximum": max})
}

pub(super) fn name_enum(values: &[&str]) -> Value {
    json!({"type": "string", "enum": values})
}

/// A closed sum spelled as a tagged object. `tag` is the discriminator field
/// name; each variant carries its own closed object of content.
pub(super) fn variant(tag: &str, variants: Vec<(&str, Vec<(String, Value)>)>) -> Value {
    let branches: Vec<Value> = variants
        .into_iter()
        .map(|(name, content)| {
            let mut properties = vec![(tag.to_owned(), json!({"const": name}))];
            properties.extend(content);
            object(properties)
        })
        .collect();
    json!({ "oneOf": branches })
}

pub(super) fn nullable(inner: Value) -> Value {
    json!({ "oneOf": [inner, {"type": "null"}] })
}

pub(super) fn list(inner: Value) -> Value {
    json!({"type": "array", "items": inner})
}

/// `{"ref": "draft"|"existing", "value": <handle text|canonical uuid>}` —
/// Invariant 1 rendered as a schema, and the only reference shape either
/// catalog emits.
pub(super) fn reference(referent: &str) -> Value {
    json!({
        "type": "object",
        "additionalProperties": false,
        "required": ["ref", "value"],
        "properties": {
            "ref": {"type": "string", "enum": ["draft", "existing"]},
            "value": {
                "type": "string",
                "description": format!(
                    "a draft handle declared in this same patch, or the canonical id of an existing {referent}"
                ),
            },
        },
    })
}

/// An adjacently tagged sum: `{"<tag>": "<variant>", "<content>": <payload>}`.
/// A variant with no payload (`None`) omits the content field entirely, which
/// is what serde emits for a unit variant in an adjacently tagged enum.
pub(super) fn variant_content(
    tag: &str,
    content: &str,
    variants: Vec<(&str, Option<Value>)>,
) -> Value {
    let branches: Vec<Value> = variants
        .into_iter()
        .map(|(name, payload)| {
            let mut properties = vec![(tag.to_owned(), json!({"const": name}))];
            if let Some(inner) = payload {
                properties.push((content.to_owned(), inner));
            }
            object(properties)
        })
        .collect();
    json!({ "oneOf": branches })
}

/// An externally tagged sum: a bare string for a unit variant, or a
/// single-key object for one that carries a payload.
pub(super) fn external_variant(variants: Vec<(&str, Option<Value>)>) -> Value {
    let branches: Vec<Value> = variants
        .into_iter()
        .map(|(name, payload)| match payload {
            None => json!({"const": name}),
            Some(inner) => object(vec![(name.to_owned(), inner)]),
        })
        .collect();
    json!({ "oneOf": branches })
}
