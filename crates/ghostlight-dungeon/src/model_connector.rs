use std::collections::BTreeSet;
use std::net::SocketAddr;
use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, bail};
use async_trait::async_trait;
use codex_connector::{
    CodexConnectorClient, CodexInputItem, CodexProviderRequest, CodexToolChoice,
    CodexTransportDisposition, CodexTransportEventPayload, CodexTransportInvocation,
    CodexTransportOutcome,
};
use sha2::{Digest, Sha256};
use tokio::sync::Semaphore;
use zeroize::Zeroizing;

use crate::model::{
    MODEL_BALANCED, MODEL_CAPABLE, MODEL_FAST, ModelPort, ModelProviderOutput, ModelStageRequest,
    ModelTokenUsage,
};

const MAX_FRAME_BYTES: usize = 1_052_672;
const REQUEST_EXPIRY: Duration = Duration::from_secs(150);
const FAST_ATTEMPT_TIMEOUT: Duration = Duration::from_secs(120);
const BALANCED_ATTEMPT_TIMEOUT: Duration = Duration::from_secs(180);
const CAPABLE_ATTEMPT_TIMEOUT: Duration = Duration::from_secs(300);

#[derive(Clone)]
pub struct CodexConnectorModelPort {
    client: CodexConnectorClient,
    request_gate: std::sync::Arc<Semaphore>,
    caller_runtime_id: String,
    fast_model: String,
    balanced_model: String,
    capable_model: String,
}

impl CodexConnectorModelPort {
    pub fn new(
        endpoint: SocketAddr,
        connection_key: String,
        caller_runtime_id: impl Into<String>,
        fast_model: impl Into<String>,
        balanced_model: impl Into<String>,
        capable_model: impl Into<String>,
        max_concurrent_requests: usize,
    ) -> Result<Self> {
        let caller_runtime_id = caller_runtime_id.into();
        if caller_runtime_id.trim().is_empty() || caller_runtime_id.trim() != caller_runtime_id {
            bail!("CodexConnector caller runtime must be a non-empty exact identity")
        }
        if !(1..=128).contains(&max_concurrent_requests) {
            bail!("CodexConnector caller concurrency must be between 1 and 128")
        }
        let client = CodexConnectorClient::new(endpoint, connection_key, MAX_FRAME_BYTES, None)?;
        Ok(Self {
            client,
            request_gate: std::sync::Arc::new(Semaphore::new(max_concurrent_requests)),
            caller_runtime_id,
            fast_model: fast_model.into(),
            balanced_model: balanced_model.into(),
            capable_model: capable_model.into(),
        })
    }

    pub fn from_runtime_secret(
        endpoint: SocketAddr,
        path: impl AsRef<Path>,
        caller_runtime_id: impl Into<String>,
        fast_model: impl Into<String>,
        balanced_model: impl Into<String>,
        capable_model: impl Into<String>,
        max_concurrent_requests: usize,
    ) -> Result<Self> {
        let bytes = Zeroizing::new(std::fs::read(path.as_ref())?);
        let raw = std::str::from_utf8(bytes.as_slice())?;
        let connection_key = raw.trim_end_matches(['\r', '\n']);
        if connection_key.is_empty() || connection_key.len() != raw.trim().len() {
            bail!("CodexConnector key file is empty or contains surrounding whitespace")
        }
        Self::new(
            endpoint,
            connection_key.to_owned(),
            caller_runtime_id,
            fast_model,
            balanced_model,
            capable_model,
            max_concurrent_requests,
        )
    }

    fn invoke(&self, request: &ModelStageRequest) -> Result<ModelProviderOutput> {
        let request_id = format!("ghostlight-model-{}", uuid::Uuid::new_v4());
        let resolved_model = match request.model.as_str() {
            MODEL_FAST => self.fast_model.clone(),
            MODEL_BALANCED => self.balanced_model.clone(),
            MODEL_CAPABLE => self.capable_model.clone(),
            explicit => explicit.to_string(),
        };
        let mut provider_request = CodexProviderRequest::new(
            request_id.clone(),
            request_id,
            resolved_model,
            "Execute the supplied Ghostlight model stage. Treat it as projected context, obey its output contract, and return only the requested public answer.",
        );
        provider_request.input = vec![CodexInputItem::UserText {
            text: request.lived_stream.clone(),
        }];
        provider_request.reasoning_effort = Some(
            if matches!(request.model.as_str(), MODEL_BALANCED | MODEL_CAPABLE) {
                "medium"
            } else {
                "low"
            }
            .to_string(),
        );
        provider_request.tools = Vec::new();
        provider_request.tool_choice = CodexToolChoice::Auto;
        provider_request.parallel_tool_calls = false;
        if let Some(schema) = request.output_schema.as_ref() {
            let mut schema = schema.clone();
            project_strict_responses_schema(&mut schema)?;
            provider_request.output_format_name = Some(output_format_name(&request.stage));
            provider_request.output_schema_json = Some(serde_json::to_string(&schema)?);
        }
        provider_request.max_output_tokens = request.max_output_tokens;
        provider_request.prompt_cache_key = Some(prompt_cache_key(request)?);

        let native_request_sha256 = Sha256::digest(rmp_serde::to_vec(request)?).into();
        let invocation = CodexTransportInvocation::new(
            self.caller_runtime_id.clone(),
            unix_ms()?.saturating_add(REQUEST_EXPIRY.as_millis() as u64),
            native_request_sha256,
            provider_request,
        )?;
        observed_output(self.client.execute(&invocation)?)
    }
}

#[async_trait]
impl ModelPort for CodexConnectorModelPort {
    async fn run(&self, request: &ModelStageRequest) -> Result<String> {
        Ok(self.run_observed(request).await?.content)
    }

    async fn run_observed(&self, request: &ModelStageRequest) -> Result<ModelProviderOutput> {
        let permit = self
            .request_gate
            .clone()
            .acquire_owned()
            .await
            .map_err(|_| anyhow::anyhow!("CodexConnector request gate closed"))?;
        let port = self.clone();
        let request = request.clone();
        tokio::task::spawn_blocking(move || {
            let _permit = permit;
            port.invoke(&request)
        })
        .await?
    }

    fn provider(&self) -> &'static str {
        "codex-connector"
    }

    fn attempt_timeout(&self, request: &ModelStageRequest) -> Duration {
        match request.model.as_str() {
            MODEL_BALANCED => BALANCED_ATTEMPT_TIMEOUT,
            MODEL_CAPABLE => CAPABLE_ATTEMPT_TIMEOUT,
            _ => FAST_ATTEMPT_TIMEOUT,
        }
    }
}

fn observed_output(result: codex_connector::CodexTransportResult) -> Result<ModelProviderOutput> {
    let (events, receipt) = match result.disposition {
        CodexTransportDisposition::Refused(reason) => {
            bail!("CodexConnector refused request: {reason:?}")
        }
        CodexTransportDisposition::Transported { events, receipt } => (events, receipt),
    };
    let mut content = String::new();
    for event in events {
        match event.payload {
            CodexTransportEventPayload::TextDelta { text } => content.push_str(&text),
            CodexTransportEventPayload::ToolCall { .. } => {
                bail!("CodexConnector exposed an inadmissible tool call")
            }
        }
    }
    let (provider_response_id, prompt_tokens, completion_tokens, reasoning_tokens, cache_hits) =
        match receipt.outcome {
            CodexTransportOutcome::Completed {
                provider_response_id,
                input_tokens,
                output_tokens,
                reasoning_output_tokens,
                cached_input_tokens,
            } => (
                provider_response_id,
                input_tokens.unwrap_or_default(),
                output_tokens.unwrap_or_default(),
                reasoning_output_tokens.unwrap_or_default(),
                cached_input_tokens.unwrap_or_default(),
            ),
            CodexTransportOutcome::Failed {
                failure_kind,
                message,
            } => bail!("Codex provider failed ({failure_kind}): {message}"),
        };
    Ok(ModelProviderOutput {
        content,
        resolved_model: Some(receipt.model),
        provider_request_id: provider_response_id,
        system_fingerprint: None,
        finish_reason: Some("completed".to_string()),
        token_usage: Some(ModelTokenUsage {
            prompt_tokens,
            completion_tokens,
            total_tokens: prompt_tokens.saturating_add(completion_tokens),
            prompt_cache_hit_tokens: cache_hits,
            prompt_cache_miss_tokens: prompt_tokens.saturating_sub(cache_hits),
            reasoning_tokens,
        }),
        transport_features: vec![
            "cultnet.direct-pipe".to_string(),
            "gamecult.codex.transport.v2".to_string(),
            receipt.transport,
        ],
    })
}

fn output_format_name(stage: &str) -> String {
    let name = stage
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '_' | '-') {
                character
            } else {
                '_'
            }
        })
        .take(64)
        .collect::<String>();
    if name.is_empty() {
        "ghostlight_output".to_string()
    } else {
        name
    }
}

fn prompt_cache_key(request: &ModelStageRequest) -> Result<String> {
    let schema = request
        .output_schema
        .as_ref()
        .map(serde_json::to_vec)
        .transpose()?
        .unwrap_or_default();
    let digest = Sha256::digest(
        [
            request.stage.as_bytes(),
            &[0],
            request.model.as_bytes(),
            &[0],
            &schema,
        ]
        .concat(),
    );
    Ok(format!("ghostlight:{digest:x}"))
}

fn unix_ms() -> Result<u64> {
    Ok(SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("system clock predates Unix epoch")?
        .as_millis()
        .try_into()
        .context("system clock does not fit u64 milliseconds")?)
}

pub(crate) fn project_strict_responses_schema(schema: &mut serde_json::Value) -> Result<()> {
    lower_schema_for_responses_format(schema);
    require_closed_responses_objects(schema, "$")?;
    if !responses_schema_is_strict(schema) {
        bail!("projected Responses output schema is not strict")
    }
    Ok(())
}

fn responses_schema_is_strict(schema: &serde_json::Value) -> bool {
    match schema {
        serde_json::Value::Object(map) => {
            if map.contains_key("$ref") && map.len() != 1 {
                return false;
            }
            if (map.contains_key("const") || map.contains_key("enum")) && !map.contains_key("type")
            {
                return false;
            }
            if schema_map_describes_object(map) {
                if map.get("additionalProperties") != Some(&serde_json::Value::Bool(false)) {
                    return false;
                }
                let Some(properties) = map.get("properties").and_then(serde_json::Value::as_object)
                else {
                    return false;
                };
                let Some(required) = map.get("required").and_then(serde_json::Value::as_array)
                else {
                    return false;
                };
                if properties
                    .keys()
                    .any(|key| !required.iter().any(|item| item.as_str() == Some(key)))
                {
                    return false;
                }
            }
            map.values().all(responses_schema_is_strict)
        }
        serde_json::Value::Array(values) => values.iter().all(responses_schema_is_strict),
        _ => true,
    }
}

fn require_closed_responses_objects(schema: &mut serde_json::Value, path: &str) -> Result<()> {
    match schema {
        serde_json::Value::Object(map) => {
            if map.contains_key("$ref") && map.len() != 1 {
                bail!("Responses schema {path} places sibling keywords beside $ref")
            }
            let describes_object = schema_map_describes_object(map);
            if describes_object {
                map.insert("type".to_string(), serde_json::json!("object"));
                let properties = map
                    .get("properties")
                    .and_then(serde_json::Value::as_object)
                    .cloned()
                    .unwrap_or_default();
                let canonical_required = map
                    .get("required")
                    .and_then(serde_json::Value::as_array)
                    .cloned()
                    .unwrap_or_default();
                for required in &canonical_required {
                    let required = required.as_str().ok_or_else(|| {
                        anyhow::anyhow!("Responses schema {path} has a non-string required key")
                    })?;
                    if !properties.contains_key(required) {
                        bail!("Responses schema {path} requires undeclared property {required:?}")
                    }
                }
                let canonical_required = canonical_required
                    .into_iter()
                    .filter_map(|value| value.as_str().map(ToString::to_string))
                    .collect::<BTreeSet<_>>();
                let mut projected = serde_json::Map::new();
                for (name, mut property) in properties {
                    require_closed_responses_objects(
                        &mut property,
                        &format!("{path}.properties.{name}"),
                    )?;
                    if !canonical_required.contains(&name) {
                        property = nullable_responses_property(property);
                    }
                    projected.insert(name, property);
                }
                let required = projected
                    .keys()
                    .cloned()
                    .map(serde_json::Value::String)
                    .collect();
                map.insert("properties".to_string(), projected.into());
                map.insert("required".to_string(), serde_json::Value::Array(required));
                map.insert("additionalProperties".to_string(), serde_json::json!(false));
            }
            for (name, value) in map.iter_mut() {
                if name != "properties" || !describes_object {
                    require_closed_responses_objects(value, &format!("{path}.{name}"))?;
                }
            }
            Ok(())
        }
        serde_json::Value::Array(values) => {
            for (index, value) in values.iter_mut().enumerate() {
                require_closed_responses_objects(value, &format!("{path}[{index}]"))?;
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

fn schema_map_describes_object(map: &serde_json::Map<String, serde_json::Value>) -> bool {
    map.get("type").and_then(serde_json::Value::as_str) == Some("object")
        || [
            "properties",
            "required",
            "additionalProperties",
            "patternProperties",
            "propertyNames",
            "minProperties",
            "maxProperties",
        ]
        .iter()
        .any(|keyword| map.contains_key(*keyword))
}

fn parent_relative_object_alternatives(value: &serde_json::Value) -> bool {
    value.as_array().is_some_and(|alternatives| {
        !alternatives.is_empty()
            && alternatives.iter().all(|alternative| {
                alternative.as_object().is_some_and(|map| {
                    !map.contains_key("type")
                        && !map.contains_key("$ref")
                        && map
                            .keys()
                            .any(|key| matches!(key.as_str(), "properties" | "required"))
                        && map.keys().all(|key| {
                            matches!(
                                key.as_str(),
                                "properties" | "required" | "title" | "description" | "$comment"
                            )
                        })
                })
            })
    })
}

fn inferred_json_type(value: &serde_json::Value) -> &'static str {
    match value {
        serde_json::Value::Null => "null",
        serde_json::Value::Bool(_) => "boolean",
        serde_json::Value::Number(number) if number.is_i64() || number.is_u64() => "integer",
        serde_json::Value::Number(_) => "number",
        serde_json::Value::String(_) => "string",
        serde_json::Value::Array(_) => "array",
        serde_json::Value::Object(_) => "object",
    }
}

fn infer_responses_literal_type(map: &mut serde_json::Map<String, serde_json::Value>) {
    if map.contains_key("type") {
        return;
    }
    let mut types = BTreeSet::new();
    if let Some(value) = map.get("const") {
        types.insert(inferred_json_type(value));
    } else if let Some(values) = map.get("enum").and_then(serde_json::Value::as_array) {
        for value in values {
            types.insert(inferred_json_type(value));
        }
    }
    match types.len() {
        0 => {}
        1 => {
            map.insert(
                "type".to_string(),
                serde_json::json!(types.into_iter().next().expect("one literal type")),
            );
        }
        _ => {
            map.insert(
                "type".to_string(),
                types.into_iter().collect::<Vec<_>>().into(),
            );
        }
    }
}

fn nullable_responses_property(property: serde_json::Value) -> serde_json::Value {
    if property
        .get("anyOf")
        .and_then(serde_json::Value::as_array)
        .is_some_and(|variants| {
            variants.iter().any(|variant| {
                variant.get("type").and_then(serde_json::Value::as_str) == Some("null")
            })
        })
    {
        property
    } else {
        serde_json::json!({"anyOf":[property,{"type":"null"}]})
    }
}

const RESPONSES_UNSUPPORTED_SCHEMA_KEYWORDS: &[&str] = &[
    "allOf",
    "not",
    "dependentRequired",
    "dependentSchemas",
    "if",
    "then",
    "else",
    "patternProperties",
    "propertyNames",
    "minProperties",
    "maxProperties",
    "unevaluatedProperties",
    "uniqueItems",
    "contains",
    "minContains",
    "maxContains",
    "prefixItems",
    "unevaluatedItems",
    "default",
    "examples",
    "readOnly",
    "writeOnly",
    "$schema",
    "$id",
    "$anchor",
    "$dynamicAnchor",
    "$dynamicRef",
    "$vocabulary",
];

fn lower_schema_for_responses_format(schema: &mut serde_json::Value) {
    let serde_json::Value::Object(map) = schema else {
        return;
    };
    infer_responses_literal_type(map);
    if map.get("format").and_then(serde_json::Value::as_str) == Some("uuid") {
        map.remove("format");
        map.entry("pattern".to_string()).or_insert_with(|| {
            serde_json::json!(
                "^[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}$"
            )
        });
    }
    for unsupported in RESPONSES_UNSUPPORTED_SCHEMA_KEYWORDS {
        map.remove(*unsupported);
    }
    if let Some(one_of) = map.remove("oneOf") {
        map.insert("anyOf".to_string(), one_of);
    }
    if map
        .get("anyOf")
        .is_some_and(parent_relative_object_alternatives)
    {
        map.remove("anyOf");
    }
    for collection in ["properties", "$defs", "definitions"] {
        if let Some(serde_json::Value::Object(children)) = map.get_mut(collection) {
            for child in children.values_mut() {
                lower_schema_for_responses_format(child);
            }
        }
    }
    if let Some(items) = map.get_mut("items") {
        match items {
            serde_json::Value::Array(items) => {
                for item in items {
                    lower_schema_for_responses_format(item);
                }
            }
            item => lower_schema_for_responses_format(item),
        }
    }
    if let Some(serde_json::Value::Array(alternatives)) = map.get_mut("anyOf") {
        for alternative in alternatives {
            lower_schema_for_responses_format(alternative);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;

    use codex_connector::{
        CodexTransportEvent, CodexTransportKey, CodexTransportReceipt, decrypt_invocation,
        encrypt_result,
    };

    #[tokio::test]
    async fn connector_port_uses_the_shared_v2_transport_and_exact_provider_request() -> Result<()>
    {
        let listener = TcpListener::bind("127.0.0.1:0")?;
        let endpoint = listener.local_addr()?;
        let server = std::thread::spawn(move || -> Result<CodexTransportInvocation> {
            let (mut stream, _) = listener.accept()?;
            let mut length = [0_u8; 4];
            stream.read_exact(&mut length)?;
            let mut payload = vec![0_u8; u32::from_be_bytes(length) as usize];
            stream.read_exact(&mut payload)?;
            let envelope = rmp_serde::from_slice(&payload)?;
            let key = CodexTransportKey::from_connection_secret("connector-test-key")?;
            let invocation = decrypt_invocation(&envelope, &key)?;
            let receipt = CodexTransportReceipt {
                schema_id: codex_connector::RECEIPT_SCHEMA_ID.to_string(),
                request_id: invocation.request_id().to_string(),
                caller_runtime_id: invocation.caller_runtime_id.clone(),
                native_request_sha256: invocation.native_request_sha256,
                provider_request_sha256: invocation.provider_request_sha256,
                model: invocation.request.model.clone(),
                transport: "codex-connector-test".to_string(),
                outcome: CodexTransportOutcome::Completed {
                    provider_response_id: Some("response-1".to_string()),
                    input_tokens: Some(100),
                    output_tokens: Some(10),
                    reasoning_output_tokens: Some(3),
                    cached_input_tokens: Some(80),
                },
            };
            let result = codex_connector::CodexTransportResult::transported(
                &invocation,
                vec![CodexTransportEvent {
                    sequence: 0,
                    payload: CodexTransportEventPayload::TextDelta {
                        text: "{\"answer\":\"ready\"}".to_string(),
                    },
                }],
                receipt,
            );
            let response = rmp_serde::to_vec(&encrypt_result(&result, &key)?)?;
            stream.write_all(&(response.len() as u32).to_be_bytes())?;
            stream.write_all(&response)?;
            Ok(invocation)
        });

        let port = CodexConnectorModelPort::new(
            endpoint,
            "connector-test-key".to_string(),
            "ghostlight-dungeon-yggdrasil",
            "gpt-5.6-luna",
            "gpt-5.6-terra",
            "gpt-5.6-luna",
            1,
        )?;
        assert_eq!(port.request_gate.available_permits(), 1);
        let output = port
            .run_observed(&ModelStageRequest {
                stage: "test_interpreter".to_string(),
                model: MODEL_BALANCED.to_string(),
                snapshot_binding: "revision:7".to_string(),
                lived_stream: "Return the typed answer.".to_string(),
                output_schema: Some(serde_json::json!({
                    "type":"object",
                    "required":["answer"],
                    "properties":{"answer":{"type":"string"}},
                    "additionalProperties":false
                })),
                source_receipt_ids: Vec::new(),
                temperature: Some(0.0),
                max_output_tokens: Some(512),
            })
            .await?;
        let invocation = server.join().expect("server thread")?;
        assert_eq!(invocation.request.model, "gpt-5.6-terra");
        assert_eq!(invocation.request.max_output_tokens, Some(512));
        assert_eq!(
            invocation.request.output_format_name.as_deref(),
            Some("test_interpreter")
        );
        assert_eq!(output.content, "{\"answer\":\"ready\"}");
        assert_eq!(
            output.token_usage.expect("usage").prompt_cache_hit_tokens,
            80
        );
        Ok(())
    }

    #[test]
    fn strict_schema_projection_closes_objects_and_makes_optional_fields_nullable() -> Result<()> {
        let mut schema = serde_json::json!({
            "type":"object",
            "required":["answer"],
            "properties":{
                "answer":{"type":"string"},
                "note":{"type":"string","default":""},
                "kind":{"enum":["one","two"]}
            }
        });
        project_strict_responses_schema(&mut schema)?;
        assert_eq!(schema["additionalProperties"], false);
        assert_eq!(
            schema["required"],
            serde_json::json!(["answer", "kind", "note"])
        );
        let kind_variants = schema["properties"]["kind"]["anyOf"]
            .as_array()
            .context("optional enum was not projected as nullable")?;
        assert!(
            kind_variants
                .iter()
                .any(|variant| variant["type"] == "string")
        );
        assert!(
            kind_variants
                .iter()
                .any(|variant| variant["type"] == "null")
        );
        assert!(schema["properties"]["note"]["anyOf"].is_array());
        assert!(responses_schema_is_strict(&schema));
        Ok(())
    }

    #[test]
    fn strict_schema_projection_rejects_ref_siblings_before_provider_submission() {
        let mut schema = serde_json::json!({
            "$ref":"#/$defs/Answer",
            "description":"provider-incompatible sibling"
        });
        let error = project_strict_responses_schema(&mut schema)
            .unwrap_err()
            .to_string();
        assert!(error.contains("sibling keywords beside $ref"));
    }

    #[test]
    fn attempt_deadline_tracks_the_logical_model_class_not_the_stage_name() -> Result<()> {
        let port = CodexConnectorModelPort::new(
            "127.0.0.1:4103".parse()?,
            "bounded-test-key".to_string(),
            "ghostlight-test",
            "gpt-5.6-luna",
            "gpt-5.6-terra",
            "gpt-5.6-sol",
            1,
        )?;
        let mut request = ModelStageRequest {
            stage: "world_compile".to_string(),
            model: MODEL_CAPABLE.to_string(),
            snapshot_binding: "revision:0".to_string(),
            lived_stream: "fixture".to_string(),
            output_schema: None,
            source_receipt_ids: Vec::new(),
            temperature: None,
            max_output_tokens: None,
        };
        assert_eq!(port.attempt_timeout(&request), CAPABLE_ATTEMPT_TIMEOUT);

        request.stage = "destination_civic_reconciliation".to_string();
        request.model = MODEL_BALANCED.to_string();
        assert_eq!(port.attempt_timeout(&request), BALANCED_ATTEMPT_TIMEOUT);

        request.stage = "cell_interpreter".to_string();
        request.model = MODEL_FAST.to_string();
        assert_eq!(port.attempt_timeout(&request), FAST_ATTEMPT_TIMEOUT);
        Ok(())
    }

    #[test]
    fn cache_key_tracks_stable_stage_contract_not_dynamic_world_context() -> Result<()> {
        let request = ModelStageRequest {
            stage: "cell_interpreter".to_string(),
            model: MODEL_CAPABLE.to_string(),
            snapshot_binding: "revision:7".to_string(),
            lived_stream: "A changing world slice.".to_string(),
            output_schema: Some(serde_json::json!({
                "type":"object",
                "properties":{"action":{"type":"string"}}
            })),
            source_receipt_ids: vec!["receipt:one".to_string()],
            temperature: Some(0.3),
            max_output_tokens: Some(512),
        };
        let mut changed_context = request.clone();
        changed_context.snapshot_binding = "revision:8".to_string();
        changed_context.lived_stream = "A different world slice.".to_string();
        assert_eq!(
            prompt_cache_key(&request)?,
            prompt_cache_key(&changed_context)?
        );
        Ok(())
    }
}
