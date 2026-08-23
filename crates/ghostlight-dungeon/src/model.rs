use anyhow::{Result, anyhow};
use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::time::Instant;
use zeroize::Zeroizing;

pub const MODEL_FAST: &str = "ghostlight.fast.v1";
pub const MODEL_CAPABLE: &str = "ghostlight.capable.v1";

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ModelRuntimeStatus {
    pub provider: String,
    pub fast_model: String,
    pub capable_model: String,
    pub readiness: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ModelStageRequest {
    pub stage: String,
    pub model: String,
    pub snapshot_binding: String,
    pub lived_stream: String,
    pub output_schema: Option<serde_json::Value>,
    pub source_receipt_ids: Vec<String>,
    pub temperature: Option<f64>,
    pub max_output_tokens: Option<u32>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct ModelStageReceipt {
    pub schema: String,
    #[serde(default)]
    pub receipt_hash: String,
    pub provider: String,
    pub model: String,
    pub stage: String,
    pub snapshot_binding: String,
    pub request_hash: String,
    pub output_hash: String,
    pub source_receipt_ids: Vec<String>,
    pub latency_ms: u64,
    pub validation_result: String,
    #[serde(default)]
    pub local_validation_error: Option<String>,
    #[serde(default)]
    pub input_chars: usize,
    #[serde(default)]
    pub output_chars: usize,
    #[serde(default)]
    pub provider_attempts: Vec<ModelProviderAttemptReceipt>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct ModelTokenUsage {
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub total_tokens: u64,
    pub prompt_cache_hit_tokens: u64,
    pub prompt_cache_miss_tokens: u64,
    pub reasoning_tokens: u64,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct ModelProviderAttemptReceipt {
    pub provider_request_id: Option<String>,
    pub system_fingerprint: Option<String>,
    pub finish_reason: Option<String>,
    pub latency_ms: u64,
    pub token_usage: Option<ModelTokenUsage>,
    #[serde(default)]
    pub transport_features: Vec<String>,
    #[serde(default)]
    pub local_validation_result: String,
    #[serde(default)]
    pub local_validation_error: Option<String>,
}

impl ModelStageReceipt {
    pub fn storage_key(&self) -> &str {
        if self.receipt_hash.is_empty() {
            &self.output_hash
        } else {
            &self.receipt_hash
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ModelStageOutput {
    pub narrative: String,
    pub structured: Option<serde_json::Value>,
    pub receipt: ModelStageReceipt,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ModelProviderOutput {
    pub content: String,
    pub resolved_model: Option<String>,
    pub provider_request_id: Option<String>,
    pub system_fingerprint: Option<String>,
    pub finish_reason: Option<String>,
    pub token_usage: Option<ModelTokenUsage>,
    pub transport_features: Vec<String>,
}

#[async_trait]
pub trait ModelPort: Send + Sync {
    async fn run(&self, request: &ModelStageRequest) -> Result<String>;
    async fn run_observed(&self, request: &ModelStageRequest) -> Result<ModelProviderOutput> {
        Ok(ModelProviderOutput {
            content: self.run(request).await?,
            ..Default::default()
        })
    }
    fn provider(&self) -> &'static str;
    fn attempt_timeout(&self, _request: &ModelStageRequest) -> std::time::Duration {
        std::time::Duration::from_secs(45)
    }
}

pub async fn run_validated_stage(
    port: &dyn ModelPort,
    request: &ModelStageRequest,
) -> Result<ModelStageOutput> {
    run_validated_stage_with_timeout(port, request, port.attempt_timeout(request)).await
}

pub async fn run_validated_stage_with_timeout(
    port: &dyn ModelPort,
    request: &ModelStageRequest,
    timeout: std::time::Duration,
) -> Result<ModelStageOutput> {
    let validator = request
        .output_schema
        .as_ref()
        .map(jsonschema::validator_for)
        .transpose()
        .map_err(|error| {
            anyhow!(
                "model stage {} has an invalid local output schema: {error}",
                request.stage
            )
        })?;
    let mut attempt_request = request.clone();
    let stage_started = Instant::now();
    let mut provider_attempts = Vec::new();
    for attempt in 0..2 {
        let started = Instant::now();
        let provider_output = tokio::time::timeout(timeout, port.run_observed(&attempt_request))
            .await
            .map_err(|_| {
                anyhow!(
                    "model stage {} timed out after {} seconds with {} input characters",
                    request.stage,
                    timeout.as_secs(),
                    attempt_request.lived_stream.chars().count()
                )
            })??;
        let ModelProviderOutput {
            content: output,
            resolved_model,
            provider_request_id,
            system_fingerprint,
            finish_reason,
            token_usage,
            transport_features,
        } = provider_output;
        provider_attempts.push(ModelProviderAttemptReceipt {
            provider_request_id,
            system_fingerprint,
            finish_reason,
            latency_ms: started.elapsed().as_millis() as u64,
            token_usage,
            transport_features,
            local_validation_result: "pending".into(),
            local_validation_error: None,
        });
        if output.trim().is_empty() {
            provider_attempts
                .last_mut()
                .expect("attempt was just recorded")
                .local_validation_result = "empty".into();
            provider_attempts
                .last_mut()
                .expect("attempt was just recorded")
                .local_validation_error = Some("provider returned an empty response".into());
            if attempt == 0 {
                attempt_request.lived_stream.push_str(
                    "\n\nLOCAL VALIDATOR: The previous response was empty. Return one complete response against the same snapshot and output contract.",
                );
                continue;
            }
            return Err(anyhow!("model returned empty output twice"));
        }
        let structured = match &validator {
            Some(_) => match serde_json::from_str(&output) {
                Ok(value) => {
                    let validation = validator.as_ref().expect("structured validator");
                    if let Err(error) = validation.validate(&value) {
                        let diagnostic = schema_validation_diagnostic(&request.stage, &error);
                        provider_attempts
                            .last_mut()
                            .expect("attempt was just recorded")
                            .local_validation_result = "schema_invalid".into();
                        provider_attempts
                            .last_mut()
                            .expect("attempt was just recorded")
                            .local_validation_error = Some(bounded_validation_error(&diagnostic));
                        if attempt == 0 {
                            attempt_request.lived_stream.push_str(&format!(
                                "\n\nLOCAL VALIDATOR REJECTED THE PREVIOUS JSON: {diagnostic}\nReturn one corrected complete JSON object against the same snapshot and schema."
                            ));
                            continue;
                        }
                        return Err(anyhow!(
                            "model returned schema-invalid JSON twice: {diagnostic}"
                        ));
                    }
                    Some(value)
                }
                Err(error) if attempt == 0 => {
                    provider_attempts
                        .last_mut()
                        .expect("attempt was just recorded")
                        .local_validation_result = "malformed_json".into();
                    provider_attempts
                        .last_mut()
                        .expect("attempt was just recorded")
                        .local_validation_error = Some(bounded_validation_error(&error));
                    attempt_request.lived_stream.push_str(&format!(
                        "\n\nLOCAL VALIDATOR COULD NOT PARSE THE PREVIOUS RESPONSE AS JSON: {error}\nReturn one corrected complete JSON object against the same snapshot and schema."
                    ));
                    continue;
                }
                Err(error) => {
                    provider_attempts
                        .last_mut()
                        .expect("attempt was just recorded")
                        .local_validation_result = "malformed_json".into();
                    provider_attempts
                        .last_mut()
                        .expect("attempt was just recorded")
                        .local_validation_error = Some(bounded_validation_error(&error));
                    return Err(anyhow!("model returned malformed JSON twice: {error}"));
                }
            },
            None => None,
        };
        provider_attempts
            .last_mut()
            .expect("attempt was just recorded")
            .local_validation_result = "valid".into();
        let receipt_model = resolved_model.unwrap_or_else(|| attempt_request.model.clone());
        let mut receipted_request = attempt_request.clone();
        receipted_request.model.clone_from(&receipt_model);
        let request_bytes = serde_json::to_vec(&receipted_request)?;
        let provider = port.provider().to_owned();
        let request_hash = format!("sha256:{:x}", Sha256::digest(&request_bytes));
        let output_hash = format!("sha256:{:x}", Sha256::digest(output.as_bytes()));
        let receipt_hash = format!(
            "sha256:{:x}",
            Sha256::digest(
                format!(
                    "{}|{}|{}|{}|{}|{}",
                    provider,
                    receipt_model,
                    request.stage,
                    request.snapshot_binding,
                    request_hash,
                    output_hash
                )
                .as_bytes()
            )
        );
        return Ok(ModelStageOutput {
            narrative: output.clone(),
            structured,
            receipt: ModelStageReceipt {
                schema: "ghostlight.persona_stage_receipt.v1".into(),
                receipt_hash,
                provider,
                model: receipt_model,
                stage: request.stage.clone(),
                snapshot_binding: request.snapshot_binding.clone(),
                request_hash,
                output_hash,
                source_receipt_ids: request.source_receipt_ids.clone(),
                latency_ms: stage_started.elapsed().as_millis() as u64,
                validation_result: "valid".into(),
                local_validation_error: None,
                input_chars: attempt_request.lived_stream.chars().count(),
                output_chars: output.chars().count(),
                provider_attempts,
            },
        });
    }
    unreachable!()
}

fn bounded_validation_error(error: &impl std::fmt::Display) -> String {
    error.to_string().chars().take(1_000).collect()
}

fn schema_validation_diagnostic(stage: &str, error: &jsonschema::ValidationError<'_>) -> String {
    format!(
        "stage {stage}, instance {}, schema {}: {error}",
        error.instance_path(),
        error.schema_path()
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct InvalidThenValid {
        calls: AtomicUsize,
    }

    struct NeverReturns;
    struct ShortDeadlineNeverReturns;
    struct AlwaysSchemaInvalid;
    struct CorrectionAware {
        calls: AtomicUsize,
    }
    struct PhysicallyRoutedModel;

    #[async_trait]
    impl ModelPort for NeverReturns {
        async fn run(&self, _: &ModelStageRequest) -> Result<String> {
            std::future::pending().await
        }

        fn provider(&self) -> &'static str {
            "fixture"
        }
    }

    #[async_trait]
    impl ModelPort for ShortDeadlineNeverReturns {
        async fn run(&self, _: &ModelStageRequest) -> Result<String> {
            std::future::pending().await
        }

        fn provider(&self) -> &'static str {
            "slow-fixture"
        }

        fn attempt_timeout(&self, _: &ModelStageRequest) -> std::time::Duration {
            std::time::Duration::from_millis(5)
        }
    }

    #[async_trait]
    impl ModelPort for AlwaysSchemaInvalid {
        async fn run(&self, _: &ModelStageRequest) -> Result<String> {
            Ok(r#"{"answer":[]}"#.into())
        }

        fn provider(&self) -> &'static str {
            "invalid-fixture"
        }
    }

    #[async_trait]
    impl ModelPort for InvalidThenValid {
        async fn run(&self, _: &ModelStageRequest) -> Result<String> {
            Ok(if self.calls.fetch_add(1, Ordering::SeqCst) == 0 {
                r#"{"answer":7}"#.into()
            } else {
                r#"{"answer":"ready"}"#.into()
            })
        }

        fn provider(&self) -> &'static str {
            "fixture"
        }
    }

    #[async_trait]
    impl ModelPort for CorrectionAware {
        async fn run(&self, request: &ModelStageRequest) -> Result<String> {
            assert_eq!(request.snapshot_binding, "campaign:one:revision:4");
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(if request.lived_stream.contains("LOCAL VALIDATOR") {
                r#"{"answer":"corrected"}"#.into()
            } else {
                r#"{"wrong":"shape"}"#.into()
            })
        }

        fn provider(&self) -> &'static str {
            "fixture"
        }
    }

    #[async_trait]
    impl ModelPort for PhysicallyRoutedModel {
        async fn run(&self, _: &ModelStageRequest) -> Result<String> {
            unreachable!("run_observed owns this fixture")
        }

        async fn run_observed(&self, _: &ModelStageRequest) -> Result<ModelProviderOutput> {
            Ok(ModelProviderOutput {
                content: "ready".into(),
                resolved_model: Some("stealth/ox-alpha".into()),
                ..Default::default()
            })
        }

        fn provider(&self) -> &'static str {
            "openrouter"
        }
    }

    #[tokio::test]
    async fn schema_invalid_json_gets_one_same_snapshot_retry() {
        let port = InvalidThenValid {
            calls: AtomicUsize::new(0),
        };
        let request = ModelStageRequest {
            stage: "typed-stage".into(),
            model: "fixture".into(),
            snapshot_binding: "campaign:one:revision:4".into(),
            lived_stream: "fixture".into(),
            output_schema: Some(serde_json::json!({
                "$schema":"https://json-schema.org/draft/2020-12/schema",
                "type":"object",
                "required":["answer"],
                "properties":{"answer":{"type":"string"}}
            })),
            source_receipt_ids: vec![],
            temperature: None,
            max_output_tokens: None,
        };
        let output = run_validated_stage(&port, &request).await.unwrap();
        assert_eq!(port.calls.load(Ordering::SeqCst), 2);
        assert_eq!(output.structured.unwrap()["answer"], "ready");
        assert_eq!(output.receipt.provider_attempts.len(), 2);
        assert_eq!(output.receipt.snapshot_binding, request.snapshot_binding);
    }

    #[tokio::test]
    async fn structured_retry_carries_local_failure_without_rebinding_snapshot() {
        let port = CorrectionAware {
            calls: AtomicUsize::new(0),
        };
        let request = ModelStageRequest {
            stage: "typed-stage".into(),
            model: "fixture".into(),
            snapshot_binding: "campaign:one:revision:4".into(),
            lived_stream: "fixture".into(),
            output_schema: Some(serde_json::json!({
                "type":"object",
                "additionalProperties":false,
                "required":["answer"],
                "properties":{"answer":{"type":"string"}}
            })),
            source_receipt_ids: vec![],
            temperature: None,
            max_output_tokens: None,
        };
        let output = run_validated_stage(&port, &request).await.unwrap();
        assert_eq!(port.calls.load(Ordering::SeqCst), 2);
        assert_eq!(output.structured.unwrap()["answer"], "corrected");
        assert_eq!(output.receipt.snapshot_binding, request.snapshot_binding);
    }

    #[tokio::test]
    async fn repeated_schema_failure_names_stage_and_exact_instance_path() {
        let request = ModelStageRequest {
            stage: "outcome-fixture".into(),
            model: "fixture".into(),
            snapshot_binding: "campaign:one:revision:4".into(),
            lived_stream: "fixture".into(),
            output_schema: Some(serde_json::json!({
                "type":"object",
                "required":["answer"],
                "properties":{"answer":{"type":"array","minItems":1}}
            })),
            source_receipt_ids: vec![],
            temperature: None,
            max_output_tokens: None,
        };
        let error = run_validated_stage(&AlwaysSchemaInvalid, &request)
            .await
            .unwrap_err()
            .to_string();

        assert!(error.contains("stage outcome-fixture"));
        assert!(error.contains("instance /answer"));
        assert!(error.contains("minItems"));
    }

    #[tokio::test]
    async fn provider_timeout_returns_no_stage_output() {
        let request = ModelStageRequest {
            stage: "timeout-stage".into(),
            model: "fixture".into(),
            snapshot_binding: "campaign:one:revision:4".into(),
            lived_stream: "fixture".into(),
            output_schema: None,
            source_receipt_ids: vec![],
            temperature: None,
            max_output_tokens: None,
        };
        let error = run_validated_stage_with_timeout(
            &NeverReturns,
            &request,
            std::time::Duration::from_millis(5),
        )
        .await
        .unwrap_err();
        assert!(error.to_string().contains("timed out"));
    }

    #[tokio::test]
    async fn validated_stage_uses_the_transport_owned_attempt_deadline() {
        let request = ModelStageRequest {
            stage: "transport-timeout-stage".into(),
            model: "fixture".into(),
            snapshot_binding: "campaign:one:revision:4".into(),
            lived_stream: "fixture".into(),
            output_schema: None,
            source_receipt_ids: vec![],
            temperature: None,
            max_output_tokens: None,
        };
        let error = run_validated_stage(&ShortDeadlineNeverReturns, &request)
            .await
            .unwrap_err();
        assert!(error.to_string().contains("timed out"));
    }

    #[tokio::test]
    async fn receipt_hashes_the_physical_provider_model_not_the_logical_class() {
        let request = ModelStageRequest {
            stage: "startup_probe".into(),
            model: MODEL_FAST.into(),
            snapshot_binding: "service-startup".into(),
            lived_stream: "ready?".into(),
            output_schema: None,
            source_receipt_ids: vec![],
            temperature: Some(0.0),
            max_output_tokens: Some(16),
        };
        let output = run_validated_stage(&PhysicallyRoutedModel, &request)
            .await
            .unwrap();
        assert_eq!(output.receipt.provider, "openrouter");
        assert_eq!(output.receipt.model, "stealth/ox-alpha");
        let mut physical_request = request;
        physical_request.model = "stealth/ox-alpha".into();
        assert_eq!(
            output.receipt.request_hash,
            format!(
                "sha256:{:x}",
                Sha256::digest(serde_json::to_vec(&physical_request).unwrap())
            )
        );
    }

    #[test]
    fn deepseek_response_preserves_usage_metadata_without_reasoning_content() {
        let output = decode_deepseek_response(&serde_json::json!({
            "id":"request-7",
            "model":"deepseek-v4-flash",
            "system_fingerprint":"fp-live",
            "choices":[{
                "finish_reason":"stop",
                "message":{
                    "content":"ready",
                    "reasoning_content":"must never enter the receipt"
                }
            }],
            "usage":{
                "prompt_tokens":120,
                "completion_tokens":30,
                "total_tokens":150,
                "prompt_cache_hit_tokens":80,
                "prompt_cache_miss_tokens":40,
                "completion_tokens_details":{"reasoning_tokens":0}
            }
        }))
        .unwrap();
        assert_eq!(output.content, "ready");
        assert_eq!(output.resolved_model.as_deref(), Some("deepseek-v4-flash"));
        assert_eq!(output.provider_request_id.as_deref(), Some("request-7"));
        assert_eq!(output.finish_reason.as_deref(), Some("stop"));
        assert_eq!(output.token_usage.as_ref().unwrap().total_tokens, 150);
        let receipt_shape = serde_json::to_value(ModelProviderAttemptReceipt {
            provider_request_id: output.provider_request_id,
            system_fingerprint: output.system_fingerprint,
            finish_reason: output.finish_reason,
            latency_ms: 1,
            token_usage: output.token_usage,
            transport_features: output.transport_features,
            local_validation_result: "valid".into(),
            local_validation_error: None,
        })
        .unwrap();
        assert!(
            !serde_json::to_string(&receipt_shape)
                .unwrap()
                .contains("reasoning_content")
        );
    }

    #[test]
    fn deepseek_structured_requests_are_deterministic_but_narrative_requests_are_not_forced() {
        let mut request = ModelStageRequest {
            stage: "interpreter".into(),
            model: "deepseek-v4-flash".into(),
            snapshot_binding: "campaign:one:revision:4".into(),
            lived_stream: "fixture".into(),
            output_schema: Some(serde_json::json!({"type":"object"})),
            source_receipt_ids: vec![],
            temperature: None,
            max_output_tokens: Some(321),
        };
        let structured = deepseek_request_body(&request);
        assert_eq!(structured["temperature"].as_f64(), Some(0.0));
        assert_eq!(structured["response_format"]["type"], "json_object");
        assert_eq!(structured["max_tokens"], 321);

        request.output_schema = None;
        let narrative = deepseek_request_body(&request);
        assert!(narrative.get("temperature").is_none());
        assert!(narrative.get("response_format").is_none());
    }

    #[test]
    fn openrouter_requests_use_the_selected_physical_model_and_hide_reasoning() {
        let request = ModelStageRequest {
            stage: "interpreter".into(),
            model: MODEL_FAST.into(),
            snapshot_binding: "campaign:one:revision:4".into(),
            lived_stream: "fixture".into(),
            output_schema: Some(serde_json::json!({"type":"object"})),
            source_receipt_ids: vec![],
            temperature: None,
            max_output_tokens: Some(321),
        };
        let body = openrouter_request_body(&request, "stealth/ox-alpha");
        assert_eq!(body["model"], "stealth/ox-alpha");
        assert_eq!(body["reasoning"]["effort"], "low");
        assert_eq!(body["reasoning"]["exclude"], true);
        assert_eq!(body["response_format"]["type"], "json_object");
        assert_eq!(body["plugins"][0]["id"], "response-healing");
        assert_eq!(body["temperature"].as_f64(), Some(0.0));
        assert!(body.get("thinking").is_none());
        let port = OpenRouterPort::new("test-key".into(), "stealth/ox-alpha", "stealth/ox-alpha");
        assert_eq!(
            port.attempt_timeout(&request),
            std::time::Duration::from_secs(120)
        );

        let mut narrative = request.clone();
        narrative.output_schema = None;
        assert!(
            openrouter_request_body(&narrative, "stealth/ox-alpha")
                .get("plugins")
                .is_none()
        );

        let mut capable = request;
        capable.model = MODEL_CAPABLE.into();
        assert_eq!(
            openrouter_request_body(&capable, "stealth/ox-alpha")["reasoning"]["effort"],
            "medium"
        );
    }

    #[test]
    fn openrouter_response_preserves_cache_usage_without_reasoning_content() {
        let output = decode_openai_chat_response(
            &serde_json::json!({
                "id":"generation-9",
                "model":"stealth/ox-alpha",
                "choices":[{
                    "finish_reason":"stop",
                    "message":{"content":"ready", "reasoning":"private"}
                }],
                "usage":{
                    "prompt_tokens":120,
                    "completion_tokens":30,
                    "total_tokens":150,
                    "prompt_tokens_details":{"cached_tokens":80},
                    "completion_tokens_details":{"reasoning_tokens":0}
                }
            }),
            "OpenRouter",
        )
        .unwrap();
        assert_eq!(output.resolved_model.as_deref(), Some("stealth/ox-alpha"));
        let usage = output.token_usage.as_ref().unwrap();
        assert_eq!(usage.prompt_cache_hit_tokens, 80);
        assert_eq!(usage.prompt_cache_miss_tokens, 40);
        assert!(!format!("{output:?}").contains("private"));
    }
}

#[derive(Clone)]
pub struct FixtureModel;
#[async_trait]
impl ModelPort for FixtureModel {
    async fn run(&self, request: &ModelStageRequest) -> Result<String> {
        Ok(if request.output_schema.is_some() {
            r#"{"private_delta":{"memories_add":[],"conditions_add":[],"conditions_remove":[],"goals_add":[],"relationship_updates":{}},"speech":null,"reaction_priority":0,"world_actions":[]}"#.into()
        } else {
            request.lived_stream.to_string()
        })
    }
    fn provider(&self) -> &'static str {
        "fixture"
    }
}

pub struct DeepSeekPort {
    client: reqwest::Client,
    api_key: Zeroizing<String>,
    endpoint: String,
    fast_model: String,
    capable_model: String,
}
impl DeepSeekPort {
    pub fn new(api_key: String) -> Self {
        Self::with_models(api_key, "deepseek-v4-flash", "deepseek-v4-pro")
    }

    pub fn with_models(
        api_key: String,
        fast_model: impl Into<String>,
        capable_model: impl Into<String>,
    ) -> Self {
        Self {
            client: reqwest::Client::builder()
                .connect_timeout(std::time::Duration::from_secs(10))
                .build()
                .expect("static DeepSeek client configuration is valid"),
            api_key: Zeroizing::new(api_key),
            endpoint: "https://api.deepseek.com/chat/completions".into(),
            fast_model: fast_model.into(),
            capable_model: capable_model.into(),
        }
    }

    #[cfg(windows)]
    pub fn from_machine_dpapi(path: impl AsRef<std::path::Path>) -> Result<Self> {
        Ok(Self::new(crate::windows_secret::unprotect_machine_utf8(
            path,
        )?))
    }

    pub fn from_utf8_secret_file(path: impl AsRef<std::path::Path>) -> Result<Self> {
        let bytes = Zeroizing::new(std::fs::read(path.as_ref())?);
        let secret = std::str::from_utf8(bytes.as_slice())?.trim().to_owned();
        if secret.is_empty() {
            anyhow::bail!("DeepSeek credential file is empty");
        }
        Ok(Self::new(secret))
    }

    pub fn from_runtime_secret(path: impl AsRef<std::path::Path>) -> Result<Self> {
        Self::from_runtime_secret_with_models(path, "deepseek-v4-flash", "deepseek-v4-pro")
    }

    pub fn from_runtime_secret_with_models(
        path: impl AsRef<std::path::Path>,
        fast_model: impl Into<String>,
        capable_model: impl Into<String>,
    ) -> Result<Self> {
        #[cfg(windows)]
        if path
            .as_ref()
            .extension()
            .is_some_and(|value| value == "dpapi")
        {
            return Ok(Self::with_models(
                crate::windows_secret::unprotect_machine_utf8(path)?,
                fast_model,
                capable_model,
            ));
        }
        let bytes = Zeroizing::new(std::fs::read(path.as_ref())?);
        let secret = std::str::from_utf8(bytes.as_slice())?.trim().to_owned();
        if secret.is_empty() {
            anyhow::bail!("DeepSeek credential file is empty");
        }
        Ok(Self::with_models(secret, fast_model, capable_model))
    }

    async fn run_with_observation(
        &self,
        request: &ModelStageRequest,
    ) -> Result<ModelProviderOutput> {
        let resolved_model = resolve_model(request, &self.fast_model, &self.capable_model);
        let mut routed_request = request.clone();
        routed_request.model.clone_from(&resolved_model);
        let body = deepseek_request_body(&routed_request);
        let value: serde_json::Value = self
            .client
            .post(&self.endpoint)
            .bearer_auth(self.api_key.as_str())
            .json(&body)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        let mut output = decode_deepseek_response(&value)?;
        output.resolved_model = Some(resolved_model);
        Ok(output)
    }
}

fn deepseek_request_body(request: &ModelStageRequest) -> serde_json::Value {
    let mut body = serde_json::json!({
        "model": request.model,
        "messages": [{"role": "user", "content": request.lived_stream}],
        "stream": false,
        "thinking": {"type": "disabled"}
    });
    if request.output_schema.is_some() {
        body["response_format"] = serde_json::json!({"type":"json_object"});
        body["temperature"] = serde_json::json!(request.temperature.unwrap_or(0.0));
    } else if let Some(temperature) = request.temperature {
        body["temperature"] = serde_json::json!(temperature);
    }
    if let Some(max_tokens) = request.max_output_tokens {
        body["max_tokens"] = serde_json::json!(max_tokens);
    }
    body
}
#[async_trait]
impl ModelPort for DeepSeekPort {
    async fn run(&self, request: &ModelStageRequest) -> Result<String> {
        Ok(self.run_with_observation(request).await?.content)
    }
    async fn run_observed(&self, request: &ModelStageRequest) -> Result<ModelProviderOutput> {
        self.run_with_observation(request).await
    }
    fn provider(&self) -> &'static str {
        "deepseek"
    }
}

fn decode_deepseek_response(value: &serde_json::Value) -> Result<ModelProviderOutput> {
    decode_openai_chat_response(value, "DeepSeek")
}

pub struct OpenRouterPort {
    client: reqwest::Client,
    api_key: Zeroizing<String>,
    endpoint: String,
    fast_model: String,
    capable_model: String,
}

impl OpenRouterPort {
    pub fn new(
        api_key: String,
        fast_model: impl Into<String>,
        capable_model: impl Into<String>,
    ) -> Self {
        Self {
            client: reqwest::Client::builder()
                .connect_timeout(std::time::Duration::from_secs(10))
                .build()
                .expect("static OpenRouter client configuration is valid"),
            api_key: Zeroizing::new(api_key),
            endpoint: "https://openrouter.ai/api/v1/chat/completions".into(),
            fast_model: fast_model.into(),
            capable_model: capable_model.into(),
        }
    }

    pub fn from_utf8_secret_file(
        path: impl AsRef<std::path::Path>,
        fast_model: impl Into<String>,
        capable_model: impl Into<String>,
    ) -> Result<Self> {
        let bytes = Zeroizing::new(std::fs::read(path.as_ref())?);
        let secret = std::str::from_utf8(bytes.as_slice())?.trim().to_owned();
        if secret.is_empty() {
            anyhow::bail!("OpenRouter credential file is empty");
        }
        Ok(Self::new(secret, fast_model, capable_model))
    }

    pub fn from_runtime_secret(
        path: impl AsRef<std::path::Path>,
        fast_model: impl Into<String>,
        capable_model: impl Into<String>,
    ) -> Result<Self> {
        #[cfg(windows)]
        if path
            .as_ref()
            .extension()
            .is_some_and(|value| value == "dpapi")
        {
            return Ok(Self::new(
                crate::windows_secret::unprotect_machine_utf8(path)?,
                fast_model,
                capable_model,
            ));
        }
        Self::from_utf8_secret_file(path, fast_model, capable_model)
    }

    async fn run_with_observation(
        &self,
        request: &ModelStageRequest,
    ) -> Result<ModelProviderOutput> {
        let resolved_model = resolve_model(request, &self.fast_model, &self.capable_model);
        let body = openrouter_request_body(request, &resolved_model);
        let value: serde_json::Value = self
            .client
            .post(&self.endpoint)
            .bearer_auth(self.api_key.as_str())
            .header("HTTP-Referer", "https://ghostlight.gamecult.org")
            .header("X-Title", "Ghostlight Dungeon")
            .json(&body)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        let mut output = decode_openai_chat_response(&value, "OpenRouter")?;
        output.resolved_model = Some(resolved_model);
        if request.output_schema.is_some() {
            output
                .transport_features
                .push("openrouter.response-healing".into());
        }
        Ok(output)
    }
}

fn openrouter_request_body(request: &ModelStageRequest, resolved_model: &str) -> serde_json::Value {
    let mut body = serde_json::json!({
        "model": resolved_model,
        "messages": [{"role": "user", "content": request.lived_stream}],
        "stream": false,
        "reasoning": {
            "effort": if request.model == MODEL_CAPABLE { "medium" } else { "low" },
            "exclude": true
        }
    });
    if request.output_schema.is_some() {
        body["response_format"] = serde_json::json!({"type":"json_object"});
        body["plugins"] = serde_json::json!([{"id":"response-healing"}]);
        body["temperature"] = serde_json::json!(request.temperature.unwrap_or(0.0));
    } else if let Some(temperature) = request.temperature {
        body["temperature"] = serde_json::json!(temperature);
    }
    if let Some(max_tokens) = request.max_output_tokens {
        body["max_tokens"] = serde_json::json!(max_tokens);
    }
    body
}

#[async_trait]
impl ModelPort for OpenRouterPort {
    async fn run(&self, request: &ModelStageRequest) -> Result<String> {
        Ok(self.run_with_observation(request).await?.content)
    }

    async fn run_observed(&self, request: &ModelStageRequest) -> Result<ModelProviderOutput> {
        self.run_with_observation(request).await
    }

    fn provider(&self) -> &'static str {
        "openrouter"
    }

    fn attempt_timeout(&self, _request: &ModelStageRequest) -> std::time::Duration {
        std::time::Duration::from_secs(120)
    }
}

fn resolve_model(request: &ModelStageRequest, fast_model: &str, capable_model: &str) -> String {
    match request.model.as_str() {
        MODEL_FAST => fast_model.to_owned(),
        MODEL_CAPABLE => capable_model.to_owned(),
        explicit => explicit.to_owned(),
    }
}

fn decode_openai_chat_response(
    value: &serde_json::Value,
    provider_name: &str,
) -> Result<ModelProviderOutput> {
    let content = value
        .pointer("/choices/0/message/content")
        .and_then(|v| v.as_str())
        .map(str::to_owned)
        .ok_or_else(|| anyhow!("{provider_name} response contained no assistant content"))?;
    let token_usage = value
        .get("usage")
        .filter(|usage| !usage.is_null())
        .map(|usage| ModelTokenUsage {
            prompt_tokens: usage["prompt_tokens"].as_u64().unwrap_or_default(),
            completion_tokens: usage["completion_tokens"].as_u64().unwrap_or_default(),
            total_tokens: usage["total_tokens"].as_u64().unwrap_or_default(),
            prompt_cache_hit_tokens: usage["prompt_cache_hit_tokens"]
                .as_u64()
                .or_else(|| usage["prompt_tokens_details"]["cached_tokens"].as_u64())
                .unwrap_or_default(),
            prompt_cache_miss_tokens: usage["prompt_cache_miss_tokens"].as_u64().unwrap_or_else(
                || {
                    usage["prompt_tokens"]
                        .as_u64()
                        .unwrap_or_default()
                        .saturating_sub(
                            usage["prompt_tokens_details"]["cached_tokens"]
                                .as_u64()
                                .unwrap_or_default(),
                        )
                },
            ),
            reasoning_tokens: usage["completion_tokens_details"]["reasoning_tokens"]
                .as_u64()
                .unwrap_or_default(),
        });
    Ok(ModelProviderOutput {
        content,
        resolved_model: value["model"].as_str().map(str::to_owned),
        provider_request_id: value["id"].as_str().map(str::to_owned),
        system_fingerprint: value["system_fingerprint"].as_str().map(str::to_owned),
        finish_reason: value["choices"][0]["finish_reason"]
            .as_str()
            .map(str::to_owned),
        token_usage,
        transport_features: vec![],
    })
}
