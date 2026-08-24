use std::net::{SocketAddr, TcpStream};
use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, bail};
use async_trait::async_trait;
use cultnet_rs::{
    CultNetClientSecurityOptions, CultNetMessage, CultNetRawDocumentRecord,
    CultNetRawPayloadEncoding, CultNetSecret, CultNetWireContract, TcpFramedTransportConnection,
    TcpFramedTransportProfileOptions, create_tcp_framed_transport_profile,
    decode_cultnet_message_from_slice, encode_cultnet_message_to_vec,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use zeroize::Zeroizing;

use crate::model::{
    MODEL_CAPABLE, ModelPort, ModelProviderOutput, ModelStageRequest, ModelTokenUsage,
};

const CONNECTOR_ENVELOPE_SCHEMA: &str = "epiphany.model_connector_envelope.v1";
const CONNECTOR_INVOCATION_SCHEMA: &str = "epiphany.model_connector_invocation.v1";
const CONNECTOR_RESULT_SCHEMA: &str = "epiphany.model_connector_result.v1";
const MODEL_REQUEST_SCHEMA: &str = "epiphany.model_request.v0";
const MODEL_EVENT_SCHEMA: &str = "epiphany.model_stream_event.v0";
const MODEL_RECEIPT_SCHEMA: &str = "epiphany.model_receipt.v0";
const REQUEST_KIND: &str = "model_request";
const RESULT_KIND: &str = "model_result";
const MAX_PAYLOAD_BYTES: u32 = 1_048_576;
const REQUEST_EXPIRY: Duration = Duration::from_secs(150);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct ConnectorEnvelope {
    schema_id: String,
    request_id: String,
    message_kind: String,
    nonce: Vec<u8>,
    ciphertext: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct ConnectorInvocation {
    schema_id: String,
    request_id: String,
    caller_runtime_id: String,
    expires_at_unix_ms: u64,
    request: ConnectorModelRequest,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct ConnectorResult {
    schema_id: String,
    request_id: String,
    accepted: bool,
    #[serde(default)]
    events: Vec<ConnectorModelEvent>,
    #[serde(default)]
    error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct ConnectorModelRequest {
    schema_id: String,
    request_id: String,
    conversation_id: String,
    provider: String,
    model: String,
    instructions: String,
    input: Vec<ConnectorModelInput>,
    reasoning_effort: Option<String>,
    reasoning_summary: Option<String>,
    service_tier: Option<String>,
    output_contract_id: Option<String>,
    previous_response_id: Option<String>,
    tools: Vec<ConnectorModelTool>,
    output_schema_json: Option<String>,
    source_worker_job_id: Option<String>,
    reasoning_basis_id: Option<String>,
    max_output_tokens: Option<u32>,
    prompt_cache_key: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
enum ConnectorModelInput {
    UserText {
        text: String,
    },
    AssistantText {
        text: String,
    },
    ToolCall {
        call_id: String,
        name: String,
        arguments: String,
    },
    ToolResult {
        call_id: String,
        output: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct ConnectorModelTool {
    name: String,
    description: String,
    parameters_json: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct ConnectorModelEvent {
    schema_id: String,
    request_id: String,
    provider: String,
    sequence: u64,
    payload: ConnectorModelPayload,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
enum ConnectorModelPayload {
    TextDelta {
        text: String,
    },
    ReasoningDelta {
        text: String,
    },
    ToolCall {
        call_id: String,
        name: String,
        arguments: String,
    },
    Completed {
        receipt: ConnectorModelReceipt,
    },
    Failed {
        message: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct ConnectorModelReceipt {
    schema_id: String,
    request_id: String,
    provider: String,
    model: String,
    provider_response_id: Option<String>,
    input_tokens: Option<u64>,
    output_tokens: Option<u64>,
    reasoning_output_tokens: Option<u64>,
    transport: Option<String>,
    cached_input_tokens: Option<u64>,
}

pub struct CultMeshModelPort {
    endpoint: SocketAddr,
    security: CultNetClientSecurityOptions,
    caller_runtime_id: String,
    fast_model: String,
    capable_model: String,
}

impl CultMeshModelPort {
    pub fn new(
        endpoint: SocketAddr,
        connection_key: String,
        caller_runtime_id: impl Into<String>,
        fast_model: impl Into<String>,
        capable_model: impl Into<String>,
    ) -> Result<Self> {
        let caller_runtime_id = caller_runtime_id.into();
        if caller_runtime_id.trim().is_empty() {
            bail!("model connector caller runtime must be non-empty")
        }
        if !endpoint.ip().is_loopback() {
            bail!("model connector endpoint must be loopback-only")
        }
        Ok(Self {
            endpoint,
            security: CultNetClientSecurityOptions::new(connection_key)?,
            caller_runtime_id,
            fast_model: fast_model.into(),
            capable_model: capable_model.into(),
        })
    }

    pub fn from_runtime_secret(
        endpoint: SocketAddr,
        path: impl AsRef<Path>,
        caller_runtime_id: impl Into<String>,
        fast_model: impl Into<String>,
        capable_model: impl Into<String>,
    ) -> Result<Self> {
        let bytes = Zeroizing::new(std::fs::read(path.as_ref())?);
        let connection_key = std::str::from_utf8(bytes.as_slice())?.trim().to_owned();
        if connection_key.is_empty()
            || std::str::from_utf8(bytes.as_slice())?.trim_matches(['\r', '\n']) != connection_key
        {
            bail!("model connector key file is empty or contains surrounding whitespace")
        }
        Self::new(
            endpoint,
            connection_key,
            caller_runtime_id,
            fast_model,
            capable_model,
        )
    }

    fn invoke(&self, request: &ModelStageRequest) -> Result<ModelProviderOutput> {
        let request_id = format!("ghostlight-model-{}", uuid::Uuid::new_v4());
        let resolved_model = match request.model.as_str() {
            crate::model::MODEL_FAST => self.fast_model.clone(),
            MODEL_CAPABLE => self.capable_model.clone(),
            explicit => explicit.to_string(),
        };
        let model_request = ConnectorModelRequest {
            schema_id: MODEL_REQUEST_SCHEMA.to_string(),
            request_id: request_id.clone(),
            conversation_id: request_id.clone(),
            provider: "openai-codex".to_string(),
            model: resolved_model,
            instructions: "Execute the supplied Ghostlight model stage. Treat it as projected context, obey its output contract, and return only the requested public answer.".to_string(),
            input: vec![ConnectorModelInput::UserText {
                text: request.lived_stream.clone(),
            }],
            reasoning_effort: Some(
                if request.model == MODEL_CAPABLE {
                    "medium"
                } else {
                    "low"
                }
                .to_string(),
            ),
            reasoning_summary: None,
            service_tier: None,
            output_contract_id: request.output_schema.as_ref().map(|_| request.stage.clone()),
            previous_response_id: None,
            tools: Vec::new(),
            output_schema_json: request
                .output_schema
                .as_ref()
                .map(serde_json::to_string)
                .transpose()?,
            source_worker_job_id: None,
            reasoning_basis_id: Some(request.snapshot_binding.clone()),
            max_output_tokens: request.max_output_tokens,
            prompt_cache_key: Some(prompt_cache_key(request)?),
        };
        let invocation = ConnectorInvocation {
            schema_id: CONNECTOR_INVOCATION_SCHEMA.to_string(),
            request_id: request_id.clone(),
            caller_runtime_id: self.caller_runtime_id.clone(),
            expires_at_unix_ms: unix_ms()?.saturating_add(REQUEST_EXPIRY.as_millis() as u64),
            request: model_request,
        };
        let envelope = encrypt_invocation(&invocation, &self.security)?;
        let document = CultNetRawDocumentRecord {
            schema_id: CONNECTOR_ENVELOPE_SCHEMA.to_string(),
            record_key: request_id.clone(),
            stored_at: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
            payload_encoding: CultNetRawPayloadEncoding::Messagepack,
            payload: rmp_serde::to_vec_named(&envelope)?,
            source_runtime_id: Some(self.caller_runtime_id.clone()),
            source_agent_id: None,
            source_role: Some("model-consumer".to_string()),
            tags: Some(vec!["model.generate.structured".to_string()]),
        };
        let message = CultNetMessage::DocumentPutRaw {
            message_id: request_id.clone(),
            document,
        };
        let payload =
            encode_cultnet_message_to_vec(&message, CultNetWireContract::CultNetSchemaV0)?;
        if payload.len() > MAX_PAYLOAD_BYTES as usize {
            bail!("model connector invocation exceeds the transport payload bound")
        }

        let stream = TcpStream::connect_timeout(&self.endpoint, Duration::from_secs(5))?;
        stream.set_nodelay(true)?;
        stream.set_read_timeout(Some(REQUEST_EXPIRY))?;
        stream.set_write_timeout(Some(Duration::from_secs(10)))?;
        let profile = create_tcp_framed_transport_profile(
            &self.caller_runtime_id,
            TcpFramedTransportProfileOptions {
                host: Some(self.endpoint.ip().to_string()),
                port: Some(self.endpoint.port()),
                max_payload_bytes: Some(MAX_PAYLOAD_BYTES),
                ..TcpFramedTransportProfileOptions::default()
            },
        );
        let mut connection = TcpFramedTransportConnection::new(stream, profile);
        connection.send("schema", &payload)?;
        let frame = connection.receive()?;
        let response = decode_cultnet_message_from_slice(
            &frame.payload,
            CultNetWireContract::CultNetSchemaV0,
        )?;
        let response_document = match response {
            CultNetMessage::SnapshotResponseRaw {
                message_id,
                mut documents,
            } if message_id == request_id && documents.len() == 1 => documents.remove(0),
            CultNetMessage::Error { error } => bail!("model connector refused request: {error}"),
            _ => bail!("model connector returned an unexpected CultNet response"),
        };
        if response_document.schema_id != CONNECTOR_ENVELOPE_SCHEMA
            || response_document.record_key != request_id
            || response_document.payload_encoding != CultNetRawPayloadEncoding::Messagepack
        {
            bail!("model connector response substituted its document identity")
        }
        let response_envelope: ConnectorEnvelope =
            rmp_serde::from_slice(&response_document.payload)?;
        let result = decrypt_result(&response_envelope, &self.security)?;
        if result.request_id != request_id {
            bail!("model connector result substituted its request identity")
        }
        if !result.accepted {
            bail!(
                "model connector rejected request: {}",
                result
                    .error
                    .unwrap_or_else(|| "unspecified refusal".to_string())
            )
        }
        observed_output(&request_id, result.events)
    }
}

#[async_trait]
impl ModelPort for CultMeshModelPort {
    async fn run(&self, request: &ModelStageRequest) -> Result<String> {
        Ok(self.run_observed(request).await?.content)
    }

    async fn run_observed(&self, request: &ModelStageRequest) -> Result<ModelProviderOutput> {
        let endpoint = self.endpoint;
        let security = self.security.clone();
        let caller_runtime_id = self.caller_runtime_id.clone();
        let fast_model = self.fast_model.clone();
        let capable_model = self.capable_model.clone();
        let request = request.clone();
        tokio::task::spawn_blocking(move || {
            Self {
                endpoint,
                security,
                caller_runtime_id,
                fast_model,
                capable_model,
            }
            .invoke(&request)
        })
        .await?
    }

    fn provider(&self) -> &'static str {
        "epiphany-codex"
    }

    fn attempt_timeout(&self, _request: &ModelStageRequest) -> Duration {
        Duration::from_secs(120)
    }
}

fn encrypt_invocation(
    invocation: &ConnectorInvocation,
    security: &CultNetClientSecurityOptions,
) -> Result<ConnectorEnvelope> {
    let nonce = CultNetSecret::new_nonce();
    let plaintext = rmp_serde::to_vec_named(invocation)?;
    Ok(ConnectorEnvelope {
        schema_id: CONNECTOR_ENVELOPE_SCHEMA.to_string(),
        request_id: invocation.request_id.clone(),
        message_kind: REQUEST_KIND.to_string(),
        ciphertext: CultNetSecret::encrypt_bytes(&plaintext, &nonce, security)?,
        nonce: nonce.to_vec(),
    })
}

fn decrypt_result(
    envelope: &ConnectorEnvelope,
    security: &CultNetClientSecurityOptions,
) -> Result<ConnectorResult> {
    if envelope.schema_id != CONNECTOR_ENVELOPE_SCHEMA || envelope.message_kind != RESULT_KIND {
        bail!("model connector returned an unexpected encrypted envelope")
    }
    let plaintext = CultNetSecret::decrypt_bytes(&envelope.ciphertext, &envelope.nonce, security)?;
    let result: ConnectorResult = rmp_serde::from_slice(&plaintext)?;
    if result.schema_id != CONNECTOR_RESULT_SCHEMA || result.request_id != envelope.request_id {
        bail!("model connector encrypted result substituted request identity")
    }
    Ok(result)
}

fn observed_output(
    request_id: &str,
    events: Vec<ConnectorModelEvent>,
) -> Result<ModelProviderOutput> {
    let mut content = String::new();
    let mut receipt = None;
    for (index, event) in events.into_iter().enumerate() {
        if event.schema_id != MODEL_EVENT_SCHEMA
            || event.request_id != request_id
            || event.provider != "openai-codex"
            || event.sequence != index as u64
            || receipt.is_some()
        {
            bail!("model connector returned an invalid event sequence")
        }
        match event.payload {
            ConnectorModelPayload::TextDelta { text } => content.push_str(&text),
            ConnectorModelPayload::Completed { receipt: completed } => receipt = Some(completed),
            ConnectorModelPayload::Failed { message } => {
                bail!("model connector provider failed: {message}")
            }
            ConnectorModelPayload::ReasoningDelta { .. }
            | ConnectorModelPayload::ToolCall { .. } => {
                bail!("model connector exposed a private or unsupported event")
            }
        }
    }
    let receipt = receipt.context("model connector response had no completion receipt")?;
    if receipt.schema_id != MODEL_RECEIPT_SCHEMA
        || receipt.request_id != request_id
        || receipt.provider != "openai-codex"
    {
        bail!("model connector completion receipt substituted identity")
    }
    let prompt_tokens = receipt.input_tokens.unwrap_or_default();
    let completion_tokens = receipt.output_tokens.unwrap_or_default();
    let cache_hits = receipt.cached_input_tokens.unwrap_or_default();
    Ok(ModelProviderOutput {
        content,
        resolved_model: Some(receipt.model),
        provider_request_id: receipt.provider_response_id,
        system_fingerprint: None,
        finish_reason: Some("completed".to_string()),
        token_usage: Some(ModelTokenUsage {
            prompt_tokens,
            completion_tokens,
            total_tokens: prompt_tokens.saturating_add(completion_tokens),
            prompt_cache_hit_tokens: cache_hits,
            prompt_cache_miss_tokens: prompt_tokens.saturating_sub(cache_hits),
            reasoning_tokens: receipt.reasoning_output_tokens.unwrap_or_default(),
        }),
        transport_features: vec![
            "cultnet.tcp-framed".to_string(),
            "cultmesh.provider:epiphany.codex-model".to_string(),
            receipt
                .transport
                .unwrap_or_else(|| "epiphany-codex-transport".to_string()),
        ],
    })
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
    Ok(format!("ghostlight:{:x}", digest))
}

fn unix_ms() -> Result<u64> {
    Ok(SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("system clock predates Unix epoch")?
        .as_millis()
        .try_into()
        .context("system clock does not fit u64 milliseconds")?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;

    #[tokio::test]
    async fn cultmesh_model_port_round_trips_typed_encrypted_cultnet_cargo() -> Result<()> {
        let listener = TcpListener::bind("127.0.0.1:0")?;
        let endpoint = listener.local_addr()?;
        let security = CultNetClientSecurityOptions::new("connector-test-key")?;
        let server_security = security.clone();
        let server = std::thread::spawn(move || -> Result<ConnectorInvocation> {
            let (stream, _) = listener.accept()?;
            let profile = create_tcp_framed_transport_profile(
                "test-connector",
                TcpFramedTransportProfileOptions {
                    max_payload_bytes: Some(MAX_PAYLOAD_BYTES),
                    ..TcpFramedTransportProfileOptions::default()
                },
            );
            let mut connection = TcpFramedTransportConnection::new(stream, profile);
            let frame = connection.receive()?;
            let message = decode_cultnet_message_from_slice(
                &frame.payload,
                CultNetWireContract::CultNetSchemaV0,
            )?;
            let CultNetMessage::DocumentPutRaw { document, .. } = message else {
                bail!("expected raw invocation")
            };
            let envelope: ConnectorEnvelope = rmp_serde::from_slice(&document.payload)?;
            let plaintext = CultNetSecret::decrypt_bytes(
                &envelope.ciphertext,
                &envelope.nonce,
                &server_security,
            )?;
            let invocation: ConnectorInvocation = rmp_serde::from_slice(&plaintext)?;
            let result = ConnectorResult {
                schema_id: CONNECTOR_RESULT_SCHEMA.to_string(),
                request_id: invocation.request_id.clone(),
                accepted: true,
                events: vec![
                    ConnectorModelEvent {
                        schema_id: MODEL_EVENT_SCHEMA.to_string(),
                        request_id: invocation.request_id.clone(),
                        provider: "openai-codex".to_string(),
                        sequence: 0,
                        payload: ConnectorModelPayload::TextDelta {
                            text: "{\"answer\":\"ready\"}".to_string(),
                        },
                    },
                    ConnectorModelEvent {
                        schema_id: MODEL_EVENT_SCHEMA.to_string(),
                        request_id: invocation.request_id.clone(),
                        provider: "openai-codex".to_string(),
                        sequence: 1,
                        payload: ConnectorModelPayload::Completed {
                            receipt: ConnectorModelReceipt {
                                schema_id: MODEL_RECEIPT_SCHEMA.to_string(),
                                request_id: invocation.request_id.clone(),
                                provider: "openai-codex".to_string(),
                                model: "gpt-5.4".to_string(),
                                provider_response_id: Some("response-1".to_string()),
                                input_tokens: Some(100),
                                output_tokens: Some(10),
                                reasoning_output_tokens: Some(3),
                                transport: Some("epiphany_direct_responses_http".to_string()),
                                cached_input_tokens: Some(80),
                            },
                        },
                    },
                ],
                error: None,
            };
            let nonce = CultNetSecret::new_nonce();
            let plaintext = rmp_serde::to_vec_named(&result)?;
            let envelope = ConnectorEnvelope {
                schema_id: CONNECTOR_ENVELOPE_SCHEMA.to_string(),
                request_id: invocation.request_id.clone(),
                message_kind: RESULT_KIND.to_string(),
                ciphertext: CultNetSecret::encrypt_bytes(&plaintext, &nonce, &server_security)?,
                nonce: nonce.to_vec(),
            };
            let document = CultNetRawDocumentRecord {
                schema_id: CONNECTOR_ENVELOPE_SCHEMA.to_string(),
                record_key: invocation.request_id.clone(),
                stored_at: "2026-08-24T00:00:00Z".to_string(),
                payload_encoding: CultNetRawPayloadEncoding::Messagepack,
                payload: rmp_serde::to_vec_named(&envelope)?,
                source_runtime_id: Some("test-connector".to_string()),
                source_agent_id: None,
                source_role: Some("model-provider-connector".to_string()),
                tags: None,
            };
            let payload = encode_cultnet_message_to_vec(
                &CultNetMessage::SnapshotResponseRaw {
                    message_id: invocation.request_id.clone(),
                    documents: vec![document],
                },
                CultNetWireContract::CultNetSchemaV0,
            )?;
            connection.send("schema", &payload)?;
            Ok(invocation)
        });

        let port = CultMeshModelPort::new(
            endpoint,
            "connector-test-key".to_string(),
            "ghostlight-dungeon-yggdrasil",
            "gpt-5.4",
            "gpt-5.4",
        )?;
        let output = port
            .run_observed(&ModelStageRequest {
                stage: "test_interpreter".to_string(),
                model: MODEL_CAPABLE.to_string(),
                snapshot_binding: "revision:7".to_string(),
                lived_stream: "Return the typed answer.".to_string(),
                output_schema: Some(serde_json::json!({
                    "type": "object",
                    "required": ["answer"],
                    "properties": {"answer": {"type": "string"}},
                    "additionalProperties": false
                })),
                source_receipt_ids: Vec::new(),
                temperature: Some(0.0),
                max_output_tokens: Some(512),
            })
            .await?;
        let invocation = server.join().expect("server thread")?;
        assert_eq!(invocation.caller_runtime_id, "ghostlight-dungeon-yggdrasil");
        assert_eq!(invocation.request.model, "gpt-5.4");
        assert_eq!(invocation.request.max_output_tokens, Some(512));
        assert!(invocation.request.prompt_cache_key.is_some());
        assert_eq!(output.content, "{\"answer\":\"ready\"}");
        assert_eq!(
            output.token_usage.expect("usage").prompt_cache_hit_tokens,
            80
        );
        Ok(())
    }

    #[test]
    fn event_sequence_refuses_reasoning_and_missing_terminal_receipts() {
        let reasoning = ConnectorModelEvent {
            schema_id: MODEL_EVENT_SCHEMA.to_string(),
            request_id: "request-1".to_string(),
            provider: "openai-codex".to_string(),
            sequence: 0,
            payload: ConnectorModelPayload::ReasoningDelta {
                text: "private".to_string(),
            },
        };
        assert!(observed_output("request-1", vec![reasoning]).is_err());
        assert!(observed_output("request-1", Vec::new()).is_err());
    }

    #[test]
    fn cache_key_tracks_stable_stage_contract_not_dynamic_world_context() -> Result<()> {
        let request = ModelStageRequest {
            stage: "cell_interpreter".to_string(),
            model: MODEL_CAPABLE.to_string(),
            snapshot_binding: "revision:7".to_string(),
            lived_stream: "A changing world slice.".to_string(),
            output_schema: Some(serde_json::json!({
                "type": "object",
                "properties": {"action": {"type": "string"}}
            })),
            source_receipt_ids: vec!["receipt:one".to_string()],
            temperature: Some(0.3),
            max_output_tokens: Some(512),
        };
        let mut changed_context = request.clone();
        changed_context.snapshot_binding = "revision:8".to_string();
        changed_context.lived_stream = "A different world slice.".to_string();
        changed_context.source_receipt_ids = vec!["receipt:two".to_string()];
        assert_eq!(
            prompt_cache_key(&request)?,
            prompt_cache_key(&changed_context)?
        );

        let mut changed_contract = request.clone();
        changed_contract.stage = "persona".to_string();
        assert_ne!(
            prompt_cache_key(&request)?,
            prompt_cache_key(&changed_contract)?
        );
        Ok(())
    }
}
