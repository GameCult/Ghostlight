use anyhow::{Result, anyhow};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::time::Instant;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ModelStageRequest {
    pub stage: String,
    pub model: String,
    pub snapshot_binding: String,
    pub lived_stream: String,
    pub output_schema: Option<serde_json::Value>,
    pub source_receipt_ids: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ModelStageReceipt {
    pub schema: String,
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
    let request_bytes = serde_json::to_vec(request)?;
    for attempt in 0..2 {
        let started = Instant::now();
        let output = port.run(request).await?;
        if output.trim().is_empty() {
            if attempt == 0 {
                continue;
            }
            return Err(anyhow!("model returned empty output twice"));
        }
        let structured = match &request.output_schema {
            Some(_) => match serde_json::from_str(&output) {
                Ok(value) => Some(value),
                Err(_) if attempt == 0 => continue,
                Err(error) => return Err(anyhow!("model returned malformed JSON twice: {error}")),
            },
            None => None,
        };
        return Ok(ModelStageOutput {
            narrative: output.clone(),
            structured,
            receipt: ModelStageReceipt {
                schema: "ghostlight.persona_stage_receipt.v1".into(),
                provider: port.provider().into(),
                model: request.model.clone(),
                stage: request.stage.clone(),
                snapshot_binding: request.snapshot_binding.clone(),
                request_hash: format!("sha256:{:x}", Sha256::digest(&request_bytes)),
                output_hash: format!("sha256:{:x}", Sha256::digest(output.as_bytes())),
                source_receipt_ids: request.source_receipt_ids.clone(),
                latency_ms: started.elapsed().as_millis() as u64,
                validation_result: "valid".into(),
            },
        });
    }
    unreachable!()
}

#[derive(Clone)]
pub struct FixtureModel;
#[async_trait]
impl ModelPort for FixtureModel {
    async fn run(&self, request: &ModelStageRequest) -> Result<String> {
        Ok(if request.output_schema.is_some() {
            r#"{"private_deltas":[],"speech":null,"reaction_priority":0,"world_actions":[]}"#.into()
        } else {
            format!("{}", request.lived_stream)
        })
    }
    fn provider(&self) -> &'static str {
        "fixture"
    }
}

#[derive(Clone)]
pub struct DeepSeekPort {
    client: reqwest::Client,
    api_key: String,
    endpoint: String,
}
impl DeepSeekPort {
    pub fn new(api_key: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key,
            endpoint: "https://api.deepseek.com/chat/completions".into(),
        }
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
            .bearer_auth(&self.api_key)
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
