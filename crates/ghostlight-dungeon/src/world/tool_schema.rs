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
            let mut properties = vec![(tag.to_owned(), json!({"type": "string", "const": name}))];
            properties.extend(content);
            object(properties)
        })
        .collect();
    json!({ "anyOf": branches })
}

pub(super) fn nullable(inner: Value) -> Value {
    json!({ "anyOf": [inner, {"type": "null"}] })
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
            let mut properties = vec![(tag.to_owned(), json!({"type": "string", "const": name}))];
            if let Some(inner) = payload {
                properties.push((content.to_owned(), inner));
            }
            object(properties)
        })
        .collect();
    json!({ "anyOf": branches })
}

/// An externally tagged sum: a bare string for a unit variant, or a
/// single-key object for one that carries a payload.
pub(super) fn external_variant(variants: Vec<(&str, Option<Value>)>) -> Value {
    let branches: Vec<Value> = variants
        .into_iter()
        .map(|(name, payload)| match payload {
            None => json!({"type": "string", "const": name}),
            Some(inner) => object(vec![(name.to_owned(), inner)]),
        })
        .collect();
    json!({ "anyOf": branches })
}

/// The provider's strict function-schema rules, enforced offline so the
/// catalog cannot be refused on the road one keyword at a time: every
/// node names a `type` or is an `anyOf` of nodes that do; every object is
/// closed and lists every property as required; `oneOf`, `allOf`, and
/// `not` never appear. The first live seeded run failed on exactly these.
#[cfg(test)]
pub(super) fn assert_strict(schema: &Value, at: &str) {
    for forbidden in ["oneOf", "allOf", "not"] {
        assert!(
            schema.get(forbidden).is_none(),
            "{at}: `{forbidden}` is not permitted in a strict schema"
        );
    }
    if let Some(branches) = schema.get("anyOf").and_then(Value::as_array) {
        assert!(!branches.is_empty(), "{at}: empty anyOf");
        for (index, branch) in branches.iter().enumerate() {
            assert_strict(branch, &format!("{at}/anyOf/{index}"));
        }
        return;
    }
    let kind = schema
        .get("type")
        .and_then(Value::as_str)
        .unwrap_or_else(|| panic!("{at}: schema must have a `type` key: {schema}"));
    match kind {
        "object" => {
            assert_eq!(
                schema.get("additionalProperties"),
                Some(&Value::Bool(false)),
                "{at}: object must be closed"
            );
            let properties = schema
                .get("properties")
                .and_then(Value::as_object)
                .unwrap_or_else(|| panic!("{at}: object without properties"));
            let mut names: Vec<&str> = properties.keys().map(String::as_str).collect();
            names.sort_unstable();
            let mut required: Vec<&str> = schema
                .get("required")
                .and_then(Value::as_array)
                .map(|list| list.iter().filter_map(Value::as_str).collect())
                .unwrap_or_default();
            required.sort_unstable();
            assert_eq!(names, required, "{at}: every property must be required");
            for (name, property) in properties {
                assert_strict(property, &format!("{at}/{name}"));
            }
        }
        "array" => {
            let items = schema
                .get("items")
                .unwrap_or_else(|| panic!("{at}: array without items"));
            assert_strict(items, &format!("{at}/items"));
        }
        "string" | "integer" | "number" | "boolean" | "null" => {}
        other => panic!("{at}: unexpected type {other}"),
    }
}
