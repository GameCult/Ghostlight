use anyhow::{Result, anyhow};
use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::time::Instant;
use zeroize::Zeroizing;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ModelStageRequest {
    pub stage: String,
    pub model: String,
    pub snapshot_binding: String,
    pub lived_stream: String,
    pub output_schema: Option<serde_json::Value>,
    pub source_receipt_ids: Vec<String>,
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

#[async_trait]
pub trait ModelPort: Send + Sync {
    async fn run(&self, request: &ModelStageRequest) -> Result<String>;
    fn provider(&self) -> &'static str;
}

pub async fn run_validated_stage(
    port: &dyn ModelPort,
    request: &ModelStageRequest,
) -> Result<ModelStageOutput> {
    run_validated_stage_with_timeout(port, request, std::time::Duration::from_secs(45)).await
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
        .map_err(|error| anyhow!("invalid local output schema: {error}"))?;
    let mut attempt_request = request.clone();
    for attempt in 0..2 {
        let started = Instant::now();
        let output = tokio::time::timeout(timeout, port.run(&attempt_request))
            .await
            .map_err(|_| anyhow!("model stage {} timed out", request.stage))??;
        if output.trim().is_empty() {
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
                        if attempt == 0 {
                            attempt_request.lived_stream.push_str(&format!(
                                "\n\nLOCAL VALIDATOR REJECTED THE PREVIOUS JSON: {error}\nReturn one corrected complete JSON object against the same snapshot and schema."
                            ));
                            continue;
                        }
                        return Err(anyhow!("model returned schema-invalid JSON twice: {error}"));
                    }
                    Some(value)
                }
                Err(error) if attempt == 0 => {
                    attempt_request.lived_stream.push_str(&format!(
                        "\n\nLOCAL VALIDATOR COULD NOT PARSE THE PREVIOUS RESPONSE AS JSON: {error}\nReturn one corrected complete JSON object against the same snapshot and schema."
                    ));
                    continue;
                }
                Err(error) => return Err(anyhow!("model returned malformed JSON twice: {error}")),
            },
            None => None,
        };
        let request_bytes = serde_json::to_vec(&attempt_request)?;
        let provider = port.provider().to_owned();
        let request_hash = format!("sha256:{:x}", Sha256::digest(&request_bytes));
        let output_hash = format!("sha256:{:x}", Sha256::digest(output.as_bytes()));
        let receipt_hash = format!(
            "sha256:{:x}",
            Sha256::digest(
                format!(
                    "{}|{}|{}|{}|{}|{}",
                    provider,
                    request.model,
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
                model: request.model.clone(),
                stage: request.stage.clone(),
                snapshot_binding: request.snapshot_binding.clone(),
                request_hash,
                output_hash,
                source_receipt_ids: request.source_receipt_ids.clone(),
                latency_ms: started.elapsed().as_millis() as u64,
                validation_result: "valid".into(),
            },
        });
    }
    unreachable!()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct InvalidThenValid {
        calls: AtomicUsize,
    }

    struct NeverReturns;
    struct CorrectionAware {
        calls: AtomicUsize,
    }

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
        };
        let output = run_validated_stage(&port, &request).await.unwrap();
        assert_eq!(port.calls.load(Ordering::SeqCst), 2);
        assert_eq!(output.structured.unwrap()["answer"], "ready");
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
        };
        let output = run_validated_stage(&port, &request).await.unwrap();
        assert_eq!(port.calls.load(Ordering::SeqCst), 2);
        assert_eq!(output.structured.unwrap()["answer"], "corrected");
        assert_eq!(output.receipt.snapshot_binding, request.snapshot_binding);
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
}
impl DeepSeekPort {
    pub fn new(api_key: String) -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(45))
                .build()
                .expect("static DeepSeek client configuration is valid"),
            api_key: Zeroizing::new(api_key),
            endpoint: "https://api.deepseek.com/chat/completions".into(),
        }
    }

    #[cfg(windows)]
    pub fn from_machine_dpapi(path: impl AsRef<std::path::Path>) -> Result<Self> {
        Ok(Self::new(crate::windows_secret::unprotect_machine_utf8(
            path,
        )?))
    }
}
#[async_trait]
impl ModelPort for DeepSeekPort {
    async fn run(&self, request: &ModelStageRequest) -> Result<String> {
        let mut body = serde_json::json!({"model":request.model,"messages":[{"role":"user","content":request.lived_stream}],"stream":false,"thinking":{"type":"disabled"}});
        if request.output_schema.is_some() {
            body["response_format"] = serde_json::json!({"type":"json_object"});
        }
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
        value
            .pointer("/choices/0/message/content")
            .and_then(|v| v.as_str())
            .map(str::to_owned)
            .ok_or_else(|| anyhow!("DeepSeek response contained no assistant content"))
    }
    fn provider(&self) -> &'static str {
        "deepseek"
    }
}
