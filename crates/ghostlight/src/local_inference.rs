//! An OpenAI-compatible inference port with two bindings. The local binding
//! reaches a loopback model server this process does not authenticate to and
//! needs no credential for. The hosted binding reaches an https endpoint
//! (a cheap hosted model for the Aetheria verse) with a bearer key read once
//! from a file.
//!
//! This module owns one authority: lowering a prepared request to one
//! `POST /v1/chat/completions` call and lifting the reply back into the same
//! `InferenceOutput` every other port produces. It runs no tool loop of its
//! own — a returned tool call is inert, exactly as the connector lane's is —
//! and it holds no mutex across a request, so several requests may be in
//! flight through the same port at once.

use super::controllers::{
    ControllerOpenError, InferenceEvent, InferenceFault, InferenceFaultClass, InferenceOutput, InferencePort,
    InferenceRequest, PreparedInference, REQUEST_EXPIRY, RESPONSE_TIMEOUT, TokenUsage,
    call_id_is_valid, tool_name_is_valid, unix_ms,
};
use async_trait::async_trait;
use codex_connector::CodexInputItem;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use zeroize::Zeroizing;

/// The model-name prefix that names the local transport when nothing
/// configures one.
pub const DEFAULT_LOCAL_MODEL_PREFIX: &str = "local/";

/// Everything the local port needs to open, gathered so `open_inference`
/// takes a binding rather than reading the environment itself.
pub struct LocalBinding {
    pub endpoint: SocketAddr,
    pub model_prefix: String,
    pub caller_runtime_id: String,
}

/// The model-name prefix that names the hosted transport when nothing
/// configures one.
pub const DEFAULT_HOSTED_MODEL_PREFIX: &str = "hosted/";

/// Everything the hosted binding needs to open. `base_url` is the provider's
/// origin (`https://api.deepseek.com`); the port appends
/// `/v1/chat/completions`. The key is read from `key_path` once, at open.
pub struct HostedBinding {
    pub base_url: String,
    pub key_path: PathBuf,
    pub model_prefix: String,
    pub caller_runtime_id: String,
}

/// Where the port sends its one request. The credential lives only in the
/// `Hosted` variant, so the loopback binding cannot carry one.
enum Endpoint {
    Loopback(SocketAddr),
    Hosted {
        base_url: String,
        key: Zeroizing<String>,
    },
}

impl Endpoint {
    fn url(&self) -> String {
        match self {
            Self::Loopback(address) => format!("http://{address}/v1/chat/completions"),
            Self::Hosted { base_url, .. } => {
                format!("{}/v1/chat/completions", base_url.trim_end_matches('/'))
            }
        }
    }
}

const LOCAL_RECEIPT_SCHEMA: &str = "ghostlight.local_inference_receipt.v1";

/// The identity half is computed here from the invocation, exactly as the SDK
/// receipt's is; the provenance half is the endpoint's own reply, carried
/// verbatim.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct LocalInferenceReceipt {
    schema_id: String,
    request_id: String,
    conversation_id: String,
    caller_runtime_id: String,
    native_request_sha256: [u8; 32],
    provider_request_sha256: [u8; 32],
    model: String,
    response_id: String,
    finish_reason: Option<String>,
    prompt_tokens: u64,
    completion_tokens: u64,
}

/// One OpenAI-compatible chat-completions reply, only as much of the shape as
/// this port needs. A reply that does not decode into this shape is an
/// integrity violation, not a best-effort read.
#[derive(Debug, Deserialize)]
struct LocalChatResponse {
    /// Used only for receipt bookkeeping (`response_id`), so an absent id is
    /// tolerated rather than an integrity violation; a missing or duplicate
    /// tool call id, which the tool loop must correlate against, still is.
    /// `null_as_default` covers an explicit `"id": null` the same way it
    /// does for `tool_calls`: `#[serde(default)]` alone only fires when the
    /// key is missing, not when it is present and null.
    #[serde(default, deserialize_with = "null_as_default")]
    id: String,
    choices: Vec<LocalChatChoice>,
    #[serde(default)]
    usage: Option<LocalChatUsage>,
}

#[derive(Debug, Deserialize)]
struct LocalChatChoice {
    message: LocalChatMessage,
    #[serde(default)]
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct LocalChatMessage {
    #[serde(default)]
    content: Option<String>,
    /// An absent `tool_calls` field already decodes to the empty vec through
    /// `#[serde(default)]`; `null_as_default` additionally covers an explicit
    /// `"tool_calls": null`, which `#[serde(default)]` alone does not, since
    /// `default` only fires when the key is missing, not when it is present
    /// and null.
    #[serde(default, deserialize_with = "null_as_default")]
    tool_calls: Vec<LocalChatToolCall>,
}

/// Treats a present-but-null field the same as an absent one. Paired with
/// `#[serde(default)]`, which covers only the absent case on its own.
fn null_as_default<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Default + Deserialize<'de>,
{
    Ok(Option::deserialize(deserializer)?.unwrap_or_default())
}

#[derive(Debug, Deserialize)]
struct LocalChatToolCall {
    id: String,
    function: LocalChatFunctionCall,
}

#[derive(Debug, Deserialize)]
struct LocalChatFunctionCall {
    name: String,
    arguments: String,
}

/// Every count is optional on the wire: a reply missing the prompt or
/// completion count reports no usage rather than a zero.
#[derive(Debug, Deserialize)]
struct LocalChatUsage {
    #[serde(default)]
    prompt_tokens: Option<u64>,
    #[serde(default)]
    completion_tokens: Option<u64>,
    /// DeepSeek's cache-hit count, part of `prompt_tokens`.
    #[serde(default)]
    prompt_cache_hit_tokens: Option<u64>,
}

impl LocalChatUsage {
    fn reported(&self) -> Option<TokenUsage> {
        Some(TokenUsage {
            prompt: self.prompt_tokens?,
            completion: self.completion_tokens?,
            cached_prompt: self.prompt_cache_hit_tokens,
        })
    }
}

/// The OpenAI-compatible backend behind Ghostlight's inference seam.
/// Unlike the SDK port, it runs no query loop: one prepared request lowers to
/// one HTTP call, and any tool call in the reply comes back inert for Rust's
/// own evaluator to drive the next round, exactly as the connector lane
/// already does.
pub(super) struct LocalInferencePort {
    client: reqwest::Client,
    endpoint: Endpoint,
    prefix: String,
    caller_runtime_id: String,
    /// Read only by the tests that pin which timeout a constructor resolves
    /// to; the live timeout is the client's.
    #[cfg(test)]
    response_timeout: std::time::Duration,
}

impl LocalInferencePort {
    /// The endpoint checks happen here, in the constructor, so neither a
    /// non-loopback local endpoint nor a non-https hosted one can reach this
    /// port through any path that builds one — not only the one
    /// `open_inference` exercises today. The client is built with
    /// `.no_proxy()` so an environment proxy variable can never carry this
    /// request, or a `proxy-authorization` header derived from a proxy URL's
    /// userinfo, off the endpoint; and with redirects disabled, so a 3xx
    /// reply cannot re-POST the request's contents (or the hosted bearer) to
    /// a second endpoint this port never opened.
    fn new(
        endpoint: Endpoint,
        prefix: impl Into<String>,
        caller_runtime_id: impl Into<String>,
    ) -> Result<Self, ControllerOpenError> {
        Self::with_timeout(endpoint, prefix, caller_runtime_id, RESPONSE_TIMEOUT)
    }

    /// `new` with the response timeout named. `new` is the only production
    /// caller and passes `RESPONSE_TIMEOUT`, so production has one timeout
    /// owner; the other callers are tests that reach the timeout path without
    /// waiting `RESPONSE_TIMEOUT` out.
    fn with_timeout(
        endpoint: Endpoint,
        prefix: impl Into<String>,
        caller_runtime_id: impl Into<String>,
        timeout: std::time::Duration,
    ) -> Result<Self, ControllerOpenError> {
        Self::build(endpoint, prefix, caller_runtime_id, timeout, false)
    }

    /// The one builder. `allow_plain_http` is false on every production path;
    /// the tests' hosted seam passes true so a hosted endpoint can be a
    /// loopback responder with no TLS.
    fn build(
        endpoint: Endpoint,
        prefix: impl Into<String>,
        caller_runtime_id: impl Into<String>,
        timeout: std::time::Duration,
        allow_plain_http: bool,
    ) -> Result<Self, ControllerOpenError> {
        match &endpoint {
            Endpoint::Loopback(address) if !address.ip().is_loopback() => {
                return Err(ControllerOpenError::LocalEndpointNotLoopback { endpoint: *address });
            }
            Endpoint::Hosted { base_url, .. }
                if !allow_plain_http && !base_url.starts_with("https://") =>
            {
                return Err(ControllerOpenError::HostedEndpointNotHttps);
            }
            _ => {}
        }
        Ok(Self {
            client: reqwest::Client::builder()
                .no_proxy()
                .redirect(reqwest::redirect::Policy::none())
                .timeout(timeout)
                .build()
                .expect("the local inference HTTP client builds with no custom TLS material"),
            endpoint,
            prefix: prefix.into(),
            caller_runtime_id: caller_runtime_id.into(),
            #[cfg(test)]
            response_timeout: timeout,
        })
    }

    async fn send(&self, body: Value) -> Result<LocalChatResponse, InferenceFault> {
        let mut request = self.client.post(self.endpoint.url()).json(&body);
        if let Endpoint::Hosted { key, .. } = &self.endpoint {
            request = request.bearer_auth(key.as_str());
        }
        let response = request.send().await.map_err(send_fault)?;
        let status = response.status();
        if !status.is_success() {
            let detail = "the local inference endpoint returned a non-success status";
            let class = InferenceFaultClass::Status(status.as_u16());
            return Err(if status.as_u16() == 429 || status.as_u16() == 503 {
                InferenceFault::retryable(detail)
            } else {
                InferenceFault::new(detail)
            }
            .classed(class));
        }
        let body = response.bytes().await.map_err(send_fault)?;
        serde_json::from_slice(&body).map_err(|_| bad_reply())
    }
}

/// The classifier of every transport failure of a send: one that failed before
/// a status arrived, or while the reply body was read. A timeout is not
/// retried, because the endpoint may still be generating. Anything else means
/// no complete reply came (a refused or closed connection, a body cut off
/// mid-stream), so retrying is safe. A body that arrived whole but is not the
/// declared shape is not a transport failure: `bad_reply` below classes it.
/// The fault carries the kind and never the error's own text:
/// `reqwest::Error`'s `Display` ends with the request URL, so formatting it
/// would put the endpoint into every log line and card that renders the
/// detail. Tests pin this for each class.
fn send_fault(error: reqwest::Error) -> InferenceFault {
    if error.is_timeout() {
        InferenceFault::new("the local inference request timed out")
            .classed(InferenceFaultClass::Timeout)
    } else if error.is_connect() {
        InferenceFault::retryable("the local inference endpoint refused the connection")
            .classed(InferenceFaultClass::Connect)
    } else {
        InferenceFault::retryable("the local inference endpoint closed the connection before a complete reply")
            .classed(InferenceFaultClass::NoResponse)
    }
}

/// A complete reply body that is not the declared shape: an integrity
/// violation, never retried. The decode error's text is not carried.
fn bad_reply() -> InferenceFault {
    InferenceFault::integrity_violation(
        "the local inference endpoint's reply was not the declared shape",
    )
    .classed(InferenceFaultClass::BadReply)
}

/// Lowers one prepared request to the OpenAI chat-completions body: the
/// system message is the request's instructions, every input item maps to
/// one message in order, and every offered tool is declared `strict`. The
/// body carries no credential; the hosted binding's bearer is the one header
/// `send` adds.
fn lower_request(prepared: &PreparedInference, prefix: &str) -> Result<Value, InferenceFault> {
    let request = &prepared.invocation.request;
    let model = request.model.strip_prefix(prefix).unwrap_or(&request.model);
    let mut messages = vec![json!({"role": "system", "content": request.instructions})];
    for item in &request.input {
        messages.push(match item {
            CodexInputItem::UserText { text } => json!({"role": "user", "content": text}),
            CodexInputItem::AssistantText { text } => json!({"role": "assistant", "content": text}),
            CodexInputItem::ToolCall {
                call_id,
                name,
                arguments,
            } => json!({
                "role": "assistant",
                "content": Value::Null,
                "tool_calls": [{
                    "id": call_id,
                    "type": "function",
                    "function": {"name": name, "arguments": arguments},
                }],
            }),
            CodexInputItem::ToolResult { call_id, output } => json!({
                "role": "tool",
                "tool_call_id": call_id,
                "content": output,
            }),
        });
    }
    let mut tools = Vec::with_capacity(request.tools.len());
    for tool in &request.tools {
        let parameters: Value = serde_json::from_str(&tool.parameters_json).map_err(|_| {
            InferenceFault::integrity_violation("a tool's parameters are not valid JSON")
        })?;
        tools.push(json!({
            "type": "function",
            "function": {
                "name": tool.name,
                "description": tool.description,
                "parameters": parameters,
                "strict": true,
            },
        }));
    }
    let mut body = json!({
        "model": model,
        "messages": messages,
    });
    if !tools.is_empty() {
        body["tools"] = json!(tools);
        body["parallel_tool_calls"] = json!(request.parallel_tool_calls);
    }
    if let Some(max_tokens) = request.max_output_tokens {
        body["max_tokens"] = json!(max_tokens);
    }
    Ok(body)
}

/// Lifts one chat-completions reply into the port's output. A tool call
/// naming a tool this request never offered, or carrying a call id or tool
/// name the provider contract refuses, is the endpoint lying about the
/// request it was given, not a gap — so it is an integrity violation, not a
/// dropped call.
fn assemble_output(
    prepared: &PreparedInference,
    response: LocalChatResponse,
) -> Result<InferenceOutput, InferenceFault> {
    let request = &prepared.invocation.request;
    let mut choices = response.choices;
    if choices.is_empty() {
        return Err(InferenceFault::integrity_violation(
            "the local inference endpoint's reply carried no choices",
        ));
    }
    let choice = choices.remove(0);
    let mut events = Vec::new();
    if let Some(content) = choice.message.content
        && !content.is_empty()
    {
        events.push(InferenceEvent::Text(content));
    }
    let mut seen_call_ids = std::collections::HashSet::new();
    for call in choice.message.tool_calls {
        if !call_id_is_valid(&call.id) || !tool_name_is_valid(&call.function.name) {
            return Err(InferenceFault::integrity_violation(
                "the local inference endpoint reported a call id or tool name the provider contract refuses",
            ));
        }
        if !request.tools.iter().any(|tool| tool.name == call.function.name) {
            return Err(InferenceFault::integrity_violation(
                "the local inference endpoint called a tool this request did not offer",
            ));
        }
        if !seen_call_ids.insert(call.id.clone()) {
            return Err(InferenceFault::integrity_violation(
                "the local inference endpoint reported the same call id twice",
            ));
        }
        events.push(InferenceEvent::ToolCall {
            call_id: call.id,
            name: call.function.name,
            arguments: call.function.arguments,
        });
    }
    let usage = response.usage.as_ref().and_then(LocalChatUsage::reported);
    let receipt = LocalInferenceReceipt {
        schema_id: LOCAL_RECEIPT_SCHEMA.to_owned(),
        request_id: request.request_id.clone(),
        conversation_id: request.conversation_id.clone(),
        caller_runtime_id: prepared.invocation.caller_runtime_id.clone(),
        native_request_sha256: prepared.invocation.native_request_sha256,
        provider_request_sha256: prepared.invocation.provider_request_sha256,
        model: request.model.clone(),
        response_id: response.id,
        finish_reason: choice.finish_reason,
        prompt_tokens: response
            .usage
            .as_ref()
            .and_then(|usage| usage.prompt_tokens)
            .unwrap_or_default(),
        completion_tokens: response
            .usage
            .as_ref()
            .and_then(|usage| usage.completion_tokens)
            .unwrap_or_default(),
    };
    let receipt_bytes = rmp_serde::to_vec_named(&receipt)
        .map_err(|_| InferenceFault::new("the local inference receipt could not be encoded"))?;
    let output = InferenceOutput::new(events, format!("sha256:{:x}", Sha256::digest(&receipt_bytes)));
    Ok(match usage {
        Some(usage) => output.with_usage(usage),
        None => output,
    })
}

#[async_trait]
impl InferencePort for LocalInferencePort {
    fn prepare(&self, request: InferenceRequest) -> Result<PreparedInference, InferenceFault> {
        PreparedInference::prepare(
            &self.caller_runtime_id,
            unix_ms()?.saturating_add(REQUEST_EXPIRY.as_millis() as u64),
            request,
        )
    }

    async fn infer(&self, request: PreparedInference) -> Result<InferenceOutput, InferenceFault> {
        if request.invocation.caller_runtime_id != self.caller_runtime_id {
            return Err(InferenceFault::integrity_violation(
                "persisted inference caller does not match the configured runtime identity",
            ));
        }
        if request.invocation.expires_at_unix_ms <= unix_ms()? {
            return Err(InferenceFault::new(
                "persisted inference invocation expired before it reached the local endpoint",
            ));
        }
        let body = lower_request(&request, &self.prefix)?;
        let response = self.send(body).await?;
        assemble_output(&request, response)
    }
}

/// Builds the local port from its binding. The loopback check lives in
/// `LocalInferencePort::new` itself, so it holds by construction for every
/// caller of this function, not only `open_inference`.
pub(super) fn open_local_port(
    binding: LocalBinding,
) -> Result<Arc<dyn InferencePort>, ControllerOpenError> {
    Ok(Arc::new(local_port(binding)?))
}

/// The same port with a named response timeout, for a consumer's tests that
/// reach the timeout path without waiting `RESPONSE_TIMEOUT` out. It exists
/// only under the `test-support` feature, which production builds never
/// enable, so `LocalBinding` carries no timeout and a release has exactly
/// one: `RESPONSE_TIMEOUT`.
#[cfg(feature = "test-support")]
pub fn open_local_port_with_timeout(
    binding: LocalBinding,
    timeout: std::time::Duration,
) -> Result<Arc<dyn InferencePort>, ControllerOpenError> {
    Ok(Arc::new(LocalInferencePort::with_timeout(
        Endpoint::Loopback(binding.endpoint),
        binding.model_prefix,
        binding.caller_runtime_id,
        timeout,
    )?))
}

fn local_port(binding: LocalBinding) -> Result<LocalInferencePort, ControllerOpenError> {
    LocalInferencePort::new(
        Endpoint::Loopback(binding.endpoint),
        binding.model_prefix,
        binding.caller_runtime_id,
    )
}

/// Builds the hosted port from its binding: reads the key file once, then the
/// constructor checks the URL is https. Every failure names the field's
/// error and never the path, the URL or the key.
pub(super) fn open_hosted_port(
    binding: HostedBinding,
) -> Result<Arc<dyn InferencePort>, ControllerOpenError> {
    Ok(Arc::new(hosted_port(binding)?))
}

fn hosted_port(binding: HostedBinding) -> Result<LocalInferencePort, ControllerOpenError> {
    let key = read_key_file(&binding.key_path)?;
    LocalInferencePort::new(
        Endpoint::Hosted {
            base_url: binding.base_url,
            key,
        },
        binding.model_prefix,
        binding.caller_runtime_id,
    )
}

/// The key file's whole content, minus one trailing newline run. Empty, not
/// UTF-8, or padded with whitespace is unreadable.
fn read_key_file(path: &std::path::Path) -> Result<Zeroizing<String>, ControllerOpenError> {
    let bytes = Zeroizing::new(std::fs::read(path).map_err(|_| ControllerOpenError::HostedKeyUnreadable)?);
    let raw = std::str::from_utf8(bytes.as_slice()).map_err(|_| ControllerOpenError::HostedKeyUnreadable)?;
    let key = raw.trim_end_matches(['\r', '\n']);
    if key.is_empty() || key.len() != key.trim().len() {
        return Err(ControllerOpenError::HostedKeyUnreadable);
    }
    Ok(Zeroizing::new(key.to_owned()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::controllers::{
        InferenceFaultDisposition, InferencePurpose, RequestShape, open_inference, tool_request,
    };
    use crate::elaboration::{ElaborationLoopEvaluation, SEED_ROUND_BUDGET, evaluate_elaboration_loop};
    use crate::patch::{RECORD_GAP_PATCH_TOOL, SUBMIT_PATCH_TOOL, patch_tools};
    use crate::sdk_inference::{DEFAULT_SDK_MODEL_PREFIX, RoutedInferencePort, SdkBinding};
    use crate::CommandId;
    use codex_connector::CodexToolDefinition;
    use std::collections::VecDeque;
    use std::sync::{Mutex, OnceLock};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::{TcpListener, TcpStream};

    const TEST_RUNTIME: &str = "ghostlight-local-test";
    const TEST_MODEL: &str = "local/llama-8b";

    /// A raw HTTP/1.1 responder over a loopback `TcpListener`, so the driver
    /// is exercised against real sockets without a mock HTTP crate. It plays
    /// back one scripted `(status, body)` reply per accepted connection and
    /// records each request's header block and body for inspection.
    struct ScriptedResponder {
        addr: SocketAddr,
        state: Arc<Mutex<ScriptedResponderState>>,
    }

    #[derive(Default)]
    struct ScriptedResponderState {
        replies: VecDeque<(u16, String)>,
        headers: Vec<String>,
        bodies: Vec<String>,
    }

    impl ScriptedResponder {
        async fn start(replies: Vec<(u16, String)>) -> Self {
            let listener = TcpListener::bind("127.0.0.1:0").await.expect("loopback binds");
            let addr = listener.local_addr().expect("a bound listener has a local address");
            let state = Arc::new(Mutex::new(ScriptedResponderState {
                replies: replies.into(),
                ..Default::default()
            }));
            let worker_state = Arc::clone(&state);
            tokio::spawn(async move {
                loop {
                    let Ok((stream, _)) = listener.accept().await else {
                        return;
                    };
                    serve_one(stream, &worker_state).await;
                }
            });
            Self { addr, state }
        }

        fn endpoint(&self) -> SocketAddr {
            self.addr
        }

        fn headers(&self) -> Vec<String> {
            self.state.lock().expect("the responder mutex is never poisoned").headers.clone()
        }

        fn bodies(&self) -> Vec<String> {
            self.state.lock().expect("the responder mutex is never poisoned").bodies.clone()
        }
    }

    async fn serve_one(mut stream: TcpStream, state: &Arc<Mutex<ScriptedResponderState>>) {
        let mut buffer = Vec::new();
        let mut chunk = [0_u8; 4096];
        let header_end = loop {
            let read = stream.read(&mut chunk).await.expect("the test socket reads");
            if read == 0 {
                return;
            }
            buffer.extend_from_slice(&chunk[..read]);
            if let Some(pos) = find_double_crlf(&buffer) {
                break pos;
            }
        };
        let header_block = String::from_utf8_lossy(&buffer[..header_end]).into_owned();
        let content_length: usize = header_block
            .lines()
            .find_map(|line| {
                let (name, value) = line.split_once(':')?;
                name.eq_ignore_ascii_case("content-length")
                    .then(|| value.trim().parse().ok())
                    .flatten()
            })
            .unwrap_or(0);
        let body_start = header_end + 4;
        while buffer.len() < body_start + content_length {
            let read = stream.read(&mut chunk).await.expect("the test socket reads");
            if read == 0 {
                break;
            }
            buffer.extend_from_slice(&chunk[..read]);
        }
        let body = String::from_utf8_lossy(&buffer[body_start..body_start + content_length]).into_owned();

        let (status, reply_body) = {
            let mut guard = state.lock().expect("the responder mutex is never poisoned");
            guard.headers.push(header_block);
            guard.bodies.push(body);
            guard.replies.pop_front().unwrap_or((500, "{}".to_owned()))
        };
        let reason = match status {
            200 => "OK",
            400 => "Bad Request",
            429 => "Too Many Requests",
            503 => "Service Unavailable",
            _ => "Error",
        };
        let response = format!(
            "HTTP/1.1 {status} {reason}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{reply_body}",
            reply_body.len()
        );
        let _ = stream.write_all(response.as_bytes()).await;
        let _ = stream.shutdown().await;
    }

    fn find_double_crlf(buffer: &[u8]) -> Option<usize> {
        buffer.windows(4).position(|window| window == b"\r\n\r\n")
    }

    /// PA.f17's redirect responder: one accepted connection, answered with a
    /// 307 pointing at `to`, never the scripted JSON replies `ScriptedResponder`
    /// plays back. A disabled redirect policy means `reqwest` must never open
    /// a second connection to `to` to follow it.
    async fn start_redirecting_to(to: SocketAddr) -> SocketAddr {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("loopback binds");
        let addr = listener.local_addr().expect("a bound listener has a local address");
        tokio::spawn(async move {
            let Ok((mut stream, _)) = listener.accept().await else {
                return;
            };
            let mut buffer = Vec::new();
            let mut chunk = [0_u8; 4096];
            loop {
                let read = stream.read(&mut chunk).await.unwrap_or(0);
                if read == 0 {
                    return;
                }
                buffer.extend_from_slice(&chunk[..read]);
                if find_double_crlf(&buffer).is_some() {
                    break;
                }
            }
            let body = format!(
                "HTTP/1.1 307 Temporary Redirect\r\nlocation: http://{to}/v1/chat/completions\r\ncontent-length: 0\r\nconnection: close\r\n\r\n"
            );
            let _ = stream.write_all(body.as_bytes()).await;
            let _ = stream.shutdown().await;
        });
        addr
    }

    /// One or more tool calls in one reply. Folded from the former
    /// single-call and two-call variants so there is one shape to keep
    /// consistent (PA.f20).
    fn tool_call_reply(calls: &[(&str, &str, &str)]) -> String {
        let id = if calls.len() == 1 {
            format!("resp-{}", calls[0].0)
        } else {
            "resp-double".to_owned()
        };
        let tool_calls: Vec<Value> = calls
            .iter()
            .map(|(call_id, name, arguments)| {
                json!({
                    "id": call_id,
                    "type": "function",
                    "function": {"name": name, "arguments": arguments},
                })
            })
            .collect();
        json!({
            "id": id,
            "choices": [{
                "message": {
                    "role": "assistant",
                    "content": Value::Null,
                    "tool_calls": tool_calls,
                },
                "finish_reason": "tool_calls",
            }],
            "usage": {"prompt_tokens": 10, "completion_tokens": 5},
        })
        .to_string()
    }

    fn text_reply(text: &str) -> String {
        text_reply_with_id("resp-text", text)
    }

    fn text_reply_with_id(id: &str, text: &str) -> String {
        json!({
            "id": id,
            "choices": [{
                "message": {"role": "assistant", "content": text, "tool_calls": []},
                "finish_reason": "stop",
            }],
            "usage": {"prompt_tokens": 3, "completion_tokens": 2},
        })
        .to_string()
    }

    fn seed_shape() -> RequestShape {
        RequestShape {
            max_output_tokens: 4_000,
            parallel_tool_calls: false,
        }
    }

    /// PA.f16's proxy test mutates the process-global proxy environment
    /// around one client build. `HTTP_PROXY` is read by `reqwest` only at
    /// `Client::build` time, and the only place this module builds a client
    /// is `LocalInferencePort::new`, so routing every test's construction
    /// through this one lock is enough to serialize against that test: no
    /// other client build in this module can land inside the window where
    /// the proxy variable is set.
    fn client_build_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    fn port(endpoint: SocketAddr) -> LocalInferencePort {
        let _guard = client_build_lock()
            .lock()
            .expect("the client-build lock is never poisoned");
        LocalInferencePort::new(Endpoint::Loopback(endpoint), DEFAULT_LOCAL_MODEL_PREFIX, TEST_RUNTIME)
            .expect("a loopback endpoint opens")
    }

    /// Spec test: pins invariant 7. Two rounds through the real seed
    /// evaluator: round 0 carries two non-terminal tool calls, round 1
    /// submits, and the second HTTP request literally carries the first
    /// round's tool calls and their derived results.
    #[tokio::test]
    async fn a_scripted_backend_drives_an_elaboration_round_through_the_existing_evaluator() {
        let prompt = "Author the shortfall.";
        let responder = ScriptedResponder::start(vec![
            (
                200,
                tool_call_reply(&[
                    ("call-0", RECORD_GAP_PATCH_TOOL, r#"{"detail":"no route"}"#),
                    ("call-1", RECORD_GAP_PATCH_TOOL, r#"{"detail":"still missing"}"#),
                ]),
            ),
            (200, tool_call_reply(&[("call-2", SUBMIT_PATCH_TOOL, "{}")])),
        ])
        .await;
        let port = port(responder.endpoint());
        let command_id = CommandId::new();

        let round0 = tool_request(
            command_id,
            0,
            InferencePurpose::Elaboration,
            TEST_MODEL,
            "Author the shortfall.",
            vec![CodexInputItem::UserText {
                text: prompt.into(),
            }],
            patch_tools(),
            seed_shape(),
        )
        .expect("round 0 builds");
        let prepared0 = port.prepare(round0).expect("round 0 prepares");
        let output0 = port.infer(prepared0).await.expect("round 0 infers");
        assert_eq!(
            output0.events.len(),
            2,
            "both round-0 tool calls came back inert"
        );

        let ElaborationLoopEvaluation::Continue { conversation } =
            evaluate_elaboration_loop(prompt, &[], std::slice::from_ref(&output0), SEED_ROUND_BUDGET)
                .expect("round 0 re-derives")
        else {
            panic!("round 0 completed early");
        };

        let round1 = tool_request(
            command_id,
            1,
            InferencePurpose::Elaboration,
            TEST_MODEL,
            "Author the shortfall.",
            conversation,
            patch_tools(),
            seed_shape(),
        )
        .expect("round 1 builds");
        let prepared1 = port.prepare(round1).expect("round 1 prepares");
        let output1 = port.infer(prepared1).await.expect("round 1 infers");

        let evaluation =
            evaluate_elaboration_loop(prompt, &[], &[output0, output1], SEED_ROUND_BUDGET)
                .expect("the seed evaluator re-derives");
        let ElaborationLoopEvaluation::Complete { capture } = evaluation else {
            panic!("two local inference rounds did not close the seed round")
        };
        assert!(capture.submitted, "the re-derived capture did not submit");

        let bodies = responder.bodies();
        assert_eq!(bodies.len(), 2, "the port made anything but two requests");
        // The second request is Rust's own loop replaying the first round's
        // tool calls and their fold-derived results, not the port's.
        assert!(bodies[1].contains("no route"));
        assert!(bodies[1].contains("still missing"));
        assert!(bodies[1].contains("call-0"));
        assert!(bodies[1].contains("call-1"));
        assert!(bodies[1].contains("gap recorded"));
    }

    /// Spec test: pins the message mapping item by item.
    #[tokio::test]
    async fn the_request_lowers_every_input_item() {
        let responder = ScriptedResponder::start(vec![(200, text_reply("acknowledged"))]).await;
        let port = port(responder.endpoint());
        let request = tool_request(
            CommandId::new(),
            0,
            InferencePurpose::Elaboration,
            TEST_MODEL,
            "Only reply once every item is accounted for.",
            vec![
                CodexInputItem::UserText {
                    text: "Open the gate.".into(),
                },
                CodexInputItem::AssistantText {
                    text: "Checking the ledger.".into(),
                },
                CodexInputItem::ToolCall {
                    call_id: "call-9".into(),
                    name: RECORD_GAP_PATCH_TOOL.into(),
                    arguments: r#"{"detail":"no ledger entry"}"#.into(),
                },
                CodexInputItem::ToolResult {
                    call_id: "call-9".into(),
                    output: "gap recorded".into(),
                },
            ],
            patch_tools(),
            seed_shape(),
        )
        .expect("the request builds");
        let prepared = port.prepare(request).expect("the port prepares");
        let output = port.infer(prepared).await.expect("the port infers");
        assert_eq!(
            output.events,
            vec![InferenceEvent::Text("acknowledged".into())]
        );

        let sent: Value = serde_json::from_str(&responder.bodies()[0]).expect("the body is JSON");
        let messages = sent["messages"].as_array().expect("messages is an array");
        assert_eq!(messages.len(), 5, "instructions plus four input items");
        assert_eq!(messages[0]["role"], "system");
        assert_eq!(messages[1]["role"], "user");
        assert_eq!(messages[1]["content"], "Open the gate.");
        assert_eq!(messages[2]["role"], "assistant");
        assert_eq!(messages[2]["content"], "Checking the ledger.");
        assert_eq!(messages[3]["role"], "assistant");
        assert_eq!(
            messages[3]["tool_calls"][0]["function"]["name"],
            RECORD_GAP_PATCH_TOOL
        );
        assert_eq!(messages[4]["role"], "tool");
        assert_eq!(messages[4]["tool_call_id"], "call-9");
        assert_eq!(messages[4]["content"], "gap recorded");
    }

    /// Spec test.
    #[test]
    fn a_non_loopback_endpoint_is_refused_at_open() {
        let directory = tempfile::tempdir().unwrap();
        let sdk_entry = directory.path().join("main.js");
        std::fs::write(&sdk_entry, "// sidecar").unwrap();
        let error = open_inference(
            None,
            Some(SdkBinding {
                sidecar_entry: sdk_entry,
                caller_runtime_id: TEST_RUNTIME.into(),
                model_prefix: DEFAULT_SDK_MODEL_PREFIX.into(),
            }),
            Some(LocalBinding {
                endpoint: "93.184.216.34:443".parse().unwrap(),
                model_prefix: DEFAULT_LOCAL_MODEL_PREFIX.into(),
                caller_runtime_id: TEST_RUNTIME.into(),
            }),
            None,
            &[TEST_MODEL],
        )
        .err()
        .expect("a non-loopback local endpoint opened");
        assert!(matches!(
            error,
            ControllerOpenError::LocalEndpointNotLoopback { .. }
        ));
    }

    /// PA.f24: the loopback check is IP-address loopback, not "any IPv6
    /// address" or "any address that parses." `[::1]` — IPv6 loopback —
    /// opens, and a routable IPv6 address such as `[2001:db8::1]:8080` (a
    /// documentation-range address; never loopback) is refused the same
    /// way a non-loopback IPv4 address is.
    #[test]
    fn ipv6_loopback_opens_and_a_non_loopback_ipv6_address_is_refused() {
        let _guard = client_build_lock()
            .lock()
            .expect("the client-build lock is never poisoned");
        LocalInferencePort::new(
            "[::1]:1".parse().unwrap(),
            DEFAULT_LOCAL_MODEL_PREFIX,
            TEST_RUNTIME,
        )
        .expect("the IPv6 loopback address opens");

        let error = LocalInferencePort::new(
            "[2001:db8::1]:8080".parse().unwrap(),
            DEFAULT_LOCAL_MODEL_PREFIX,
            TEST_RUNTIME,
        )
        .err()
        .expect("a non-loopback IPv6 endpoint opened");
        assert!(matches!(
            error,
            ControllerOpenError::LocalEndpointNotLoopback { .. }
        ));
    }

    /// Spec test.
    #[tokio::test]
    async fn a_call_to_an_unoffered_tool_is_an_integrity_violation() {
        let responder =
            ScriptedResponder::start(vec![(200, tool_call_reply(&[("call-0", "speek", "{}")]))]).await;
        let port = port(responder.endpoint());
        let request = tool_request(
            CommandId::new(),
            0,
            InferencePurpose::Elaboration,
            TEST_MODEL,
            "Author the shortfall.",
            vec![CodexInputItem::UserText {
                text: "Author the shortfall.".into(),
            }],
            patch_tools(),
            seed_shape(),
        )
        .expect("the request builds");
        let prepared = port.prepare(request).expect("the port prepares");
        let fault = port
            .infer(prepared)
            .await
            .expect_err("an unoffered tool call produced an output");
        assert!(fault.integrity_was_violated(), "{fault:?}");
    }

    /// Spec test. `RoutedInferencePort` prefers the longest matching claim
    /// among overlapping prefixes, and `open_inference` refuses two slots
    /// that claim the identical prefix outright.
    #[test]
    fn routing_prefers_the_longest_claim_and_refuses_a_shared_prefix() {
        let local_port = Arc::new(port("127.0.0.1:1".parse().unwrap())) as Arc<dyn InferencePort>;
        let sdk_port = Arc::new(port("127.0.0.1:1".parse().unwrap())) as Arc<dyn InferencePort>;
        let connector_port = Arc::new(port("127.0.0.1:1".parse().unwrap())) as Arc<dyn InferencePort>;
        let routed = RoutedInferencePort::new(
            Some(Arc::clone(&connector_port)),
            Some(Arc::clone(&sdk_port)),
            "claude",
            vec![("claude-local/".to_owned(), Arc::clone(&local_port))],
        );
        assert!(Arc::ptr_eq(
            routed.route("claude-local/llama-8b").unwrap(),
            &local_port
        ));
        assert!(Arc::ptr_eq(routed.route("claude-opus-5").unwrap(), &sdk_port));
        assert!(Arc::ptr_eq(
            routed.route("gpt-5.6-terra").unwrap(),
            &connector_port
        ));

        let directory = tempfile::tempdir().unwrap();
        let sdk_entry = directory.path().join("main.js");
        std::fs::write(&sdk_entry, "// sidecar").unwrap();
        let error = open_inference(
            None,
            Some(SdkBinding {
                sidecar_entry: sdk_entry,
                caller_runtime_id: TEST_RUNTIME.into(),
                model_prefix: "claude".into(),
            }),
            Some(LocalBinding {
                endpoint: "127.0.0.1:1".parse().unwrap(),
                model_prefix: "claude".into(),
                caller_runtime_id: TEST_RUNTIME.into(),
            }),
            None,
            &["claude-opus-5"],
        )
        .err()
        .expect("two ports sharing one prefix opened");
        assert!(matches!(
            error,
            ControllerOpenError::SharedModelPrefix { .. }
        ));
    }

    /// Spec test. The responder asserts on the raw header block, not the
    /// decoded request, so a header this port never intended to send cannot
    /// hide behind a friendly wrapper's own parsing.
    #[tokio::test]
    async fn no_request_carries_an_authorization_header() {
        let responder = ScriptedResponder::start(vec![(200, text_reply("ok"))]).await;
        let port = port(responder.endpoint());
        let request = tool_request(
            CommandId::new(),
            0,
            InferencePurpose::Persona,
            TEST_MODEL,
            "Respond only in natural prose.",
            vec![CodexInputItem::UserText {
                text: "Say something true.".into(),
            }],
            Vec::<CodexToolDefinition>::new(),
            RequestShape {
                max_output_tokens: 1_200,
                parallel_tool_calls: false,
            },
        )
        .expect("the request builds");
        let prepared = port.prepare(request).expect("the port prepares");
        let output = port.infer(prepared).await.expect("the port infers");
        assert_eq!(output.events, vec![InferenceEvent::Text("ok".into())]);

        let header_block = responder.headers()[0].to_ascii_lowercase();
        assert!(!header_block.contains("authorization"));
        assert!(!header_block.contains("bearer"));
        assert!(!header_block.contains("api-key"));
        assert!(!header_block.contains("api_key"));
    }

    /// Spec test (mutation M2.3). Connection refusal and the two status codes
    /// the map names are retryable; any other unsuccessful status is
    /// recovery-required, not a special case.
    #[tokio::test]
    async fn connection_refusal_and_429_503_are_retryable_other_statuses_are_recovery_required() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let refused_endpoint = listener.local_addr().unwrap();
        drop(listener);
        let refused = port(refused_endpoint);
        let request = tool_request(
            CommandId::new(),
            0,
            InferencePurpose::Persona,
            TEST_MODEL,
            "Respond only in natural prose.",
            vec![CodexInputItem::UserText {
                text: "Say something true.".into(),
            }],
            Vec::<CodexToolDefinition>::new(),
            RequestShape {
                max_output_tokens: 1_200,
                parallel_tool_calls: false,
            },
        )
        .expect("the request builds");
        let prepared = refused.prepare(request.clone()).expect("the port prepares");
        let fault = refused
            .infer(prepared)
            .await
            .expect_err("a closed port accepted a connection");
        assert!(
            !fault.requires_recovery(),
            "a refused connection was not retryable: {fault:?}"
        );
        assert!(!fault.integrity_was_violated());

        for (status, retryable) in [(429_u16, true), (503, true), (400, false)] {
            let responder = ScriptedResponder::start(vec![(status, "{}".to_owned())]).await;
            let statused = port(responder.endpoint());
            let prepared = statused.prepare(request.clone()).expect("the port prepares");
            let fault = statused
                .infer(prepared)
                .await
                .expect_err("a non-2xx status produced an output");
            assert!(!fault.integrity_was_violated(), "{fault:?}");
            assert_eq!(
                !fault.requires_recovery(),
                retryable,
                "status {status} disposition mismatch: {fault:?}"
            );
        }
    }

    /// Spec test (PA.f16). `HTTP_PROXY` is process-global and `reqwest`
    /// resolves it only at `Client::build` time, so this holds
    /// `client_build_lock` for the whole window from setting the variable to
    /// building the client to clearing it again — the same lock `port` takes
    /// for every other client this module builds — rather than adding a new
    /// dev-dependency to serialize tests.
    #[tokio::test]
    async fn the_client_never_honors_an_environment_proxy() {
        let target = ScriptedResponder::start(vec![(200, text_reply("direct"))]).await;
        let proxy = ScriptedResponder::start(vec![(200, text_reply("via-proxy"))]).await;
        let proxy_url = format!("http://{}", proxy.endpoint());

        let port = {
            let _guard = client_build_lock()
                .lock()
                .expect("the client-build lock is never poisoned");
            // SAFETY: guarded by `client_build_lock` for the entire window
            // the variable is set, and every other reqwest client build in
            // this module (`port`) takes the same lock before it builds.
            unsafe {
                std::env::set_var("HTTP_PROXY", &proxy_url);
                std::env::set_var("http_proxy", &proxy_url);
            }
            let built = LocalInferencePort::new(target.endpoint(), DEFAULT_LOCAL_MODEL_PREFIX, TEST_RUNTIME);
            unsafe {
                std::env::remove_var("HTTP_PROXY");
                std::env::remove_var("http_proxy");
            }
            built.expect("a loopback endpoint opens")
        };

        let request = tool_request(
            CommandId::new(),
            0,
            InferencePurpose::Persona,
            TEST_MODEL,
            "Respond only in natural prose.",
            vec![CodexInputItem::UserText {
                text: "Say something true.".into(),
            }],
            Vec::<CodexToolDefinition>::new(),
            RequestShape {
                max_output_tokens: 1_200,
                parallel_tool_calls: false,
            },
        )
        .expect("the request builds");
        let prepared = port.prepare(request).expect("the port prepares");
        let output = port.infer(prepared).await.expect("the port infers");
        assert_eq!(output.events, vec![InferenceEvent::Text("direct".into())]);
        assert_eq!(
            target.bodies().len(),
            1,
            "the target endpoint was not hit directly"
        );
        assert!(
            proxy.bodies().is_empty(),
            "the proxy endpoint received a request"
        );
    }

    /// Spec test (PA.f17). A 307 is a fault the redirect target never sees:
    /// `Policy::none()` means `reqwest` reports the 3xx as this port's own
    /// response instead of opening a second connection to follow it.
    #[tokio::test]
    async fn a_redirect_is_a_fault_and_the_redirect_target_is_never_reached() {
        let redirect_target = ScriptedResponder::start(vec![(200, text_reply("should never run"))]).await;
        let redirecting_endpoint = start_redirecting_to(redirect_target.endpoint()).await;
        let port = port(redirecting_endpoint);
        let request = tool_request(
            CommandId::new(),
            0,
            InferencePurpose::Persona,
            TEST_MODEL,
            "Respond only in natural prose.",
            vec![CodexInputItem::UserText {
                text: "Say something true.".into(),
            }],
            Vec::<CodexToolDefinition>::new(),
            RequestShape {
                max_output_tokens: 1_200,
                parallel_tool_calls: false,
            },
        )
        .expect("the request builds");
        let prepared = port.prepare(request).expect("the port prepares");
        let fault = port
            .infer(prepared)
            .await
            .expect_err("a redirect produced an output");
        assert!(!fault.integrity_was_violated(), "{fault:?}");
        assert!(
            fault.requires_recovery(),
            "a followed redirect should have been recovery-required, same as any \
             other unsuccessful status outside 429/503: {fault:?}"
        );
        assert!(
            redirect_target.bodies().is_empty(),
            "the redirect target received a request"
        );
    }

    /// Spec test (PA.f18). An explicit `"tool_calls": null` decodes as no
    /// calls, the same as an absent field, instead of failing to decode into
    /// an integrity violation.
    #[test]
    fn a_null_tool_calls_field_decodes_as_no_calls() {
        let raw = json!({
            "id": "resp-null",
            "choices": [{
                "message": {"role": "assistant", "content": "ok", "tool_calls": Value::Null},
                "finish_reason": "stop",
            }],
        })
        .to_string();
        let response: LocalChatResponse =
            serde_json::from_str(&raw).expect("a null tool_calls field decodes");
        assert!(response.choices[0].message.tool_calls.is_empty());
    }

    /// Spec test (PA.f18). An absent `tool_calls` field decodes the same way
    /// as an explicit null.
    #[test]
    fn an_absent_tool_calls_field_decodes_as_no_calls() {
        let raw = json!({
            "id": "resp-absent",
            "choices": [{
                "message": {"role": "assistant", "content": "ok"},
                "finish_reason": "stop",
            }],
        })
        .to_string();
        let response: LocalChatResponse =
            serde_json::from_str(&raw).expect("an absent tool_calls field decodes");
        assert!(response.choices[0].message.tool_calls.is_empty());
    }

    /// Spec test (PA.f18). An absent top-level `id` decodes to the empty
    /// string. It is used only for receipt bookkeeping, so a local server
    /// that omits it should not quarantine the lane.
    #[test]
    fn an_absent_response_id_decodes_as_empty() {
        let raw = json!({
            "choices": [{
                "message": {"role": "assistant", "content": "ok", "tool_calls": []},
                "finish_reason": "stop",
            }],
        })
        .to_string();
        let response: LocalChatResponse =
            serde_json::from_str(&raw).expect("an absent id decodes");
        assert_eq!(response.id, "");
    }

    /// PA.f23: an explicit `"id": null` decodes to the empty string the
    /// same way an absent `id` does, instead of failing to decode. Mirrors
    /// `a_null_tool_calls_field_decodes_as_no_calls`.
    #[test]
    fn a_null_response_id_decodes_as_empty() {
        let raw = json!({
            "id": Value::Null,
            "choices": [{
                "message": {"role": "assistant", "content": "ok", "tool_calls": []},
                "finish_reason": "stop",
            }],
        })
        .to_string();
        let response: LocalChatResponse =
            serde_json::from_str(&raw).expect("a null id decodes");
        assert_eq!(response.id, "");
    }

    /// Spec test (PA.f19). Open refuses an empty local model prefix: an empty
    /// prefix matches every model and would silently take the local port's
    /// lanes.
    #[test]
    fn an_empty_local_model_prefix_is_refused_at_open() {
        let error = open_inference(
            None,
            None,
            Some(LocalBinding {
                endpoint: "127.0.0.1:1".parse().unwrap(),
                model_prefix: String::new(),
                caller_runtime_id: TEST_RUNTIME.into(),
            }),
            None,
            &[TEST_MODEL],
        )
        .err()
        .expect("an empty local model prefix opened");
        assert!(matches!(
            error,
            ControllerOpenError::EmptyModelPrefix { transport: "local" }
        ));
    }

    /// Spec test (PA.f19). Open refuses an empty SDK model prefix for the
    /// same reason.
    #[test]
    fn an_empty_sdk_model_prefix_is_refused_at_open() {
        let directory = tempfile::tempdir().unwrap();
        let sdk_entry = directory.path().join("main.js");
        std::fs::write(&sdk_entry, "// sidecar").unwrap();
        let error = open_inference(
            None,
            Some(SdkBinding {
                sidecar_entry: sdk_entry,
                caller_runtime_id: TEST_RUNTIME.into(),
                model_prefix: String::new(),
            }),
            None,
            None,
            &[TEST_MODEL],
        )
        .err()
        .expect("an empty SDK model prefix opened");
        assert!(matches!(
            error,
            ControllerOpenError::EmptyModelPrefix { transport: "SDK" }
        ));
    }

    /// Spec test (PA.f19). Duplicate call ids in one reply pass neither the
    /// port's own validators nor the evaluator's; the port refuses them
    /// itself, as an integrity violation, rather than letting the evaluator
    /// see two calls it cannot tell apart.
    #[tokio::test]
    async fn duplicate_call_ids_in_one_reply_are_an_integrity_violation() {
        let responder = ScriptedResponder::start(vec![(
            200,
            tool_call_reply(&[
                ("call-0", RECORD_GAP_PATCH_TOOL, r#"{"detail":"first"}"#),
                ("call-0", RECORD_GAP_PATCH_TOOL, r#"{"detail":"second"}"#),
            ]),
        )])
        .await;
        let port = port(responder.endpoint());
        let request = tool_request(
            CommandId::new(),
            0,
            InferencePurpose::Elaboration,
            TEST_MODEL,
            "Author the shortfall.",
            vec![CodexInputItem::UserText {
                text: "Author the shortfall.".into(),
            }],
            patch_tools(),
            seed_shape(),
        )
        .expect("the request builds");
        let prepared = port.prepare(request).expect("the port prepares");
        let fault = port
            .infer(prepared)
            .await
            .expect_err("a duplicate call id produced an output");
        assert!(fault.integrity_was_violated(), "{fault:?}");
    }

    /// Spec test (PA.f20). A request offering no tools omits `tools` and
    /// `parallel_tool_calls` from the lowered body instead of sending an
    /// empty array.
    #[tokio::test]
    async fn a_request_with_no_tools_omits_tools_and_parallel_tool_calls() {
        let responder = ScriptedResponder::start(vec![(200, text_reply("ok"))]).await;
        let port = port(responder.endpoint());
        let request = tool_request(
            CommandId::new(),
            0,
            InferencePurpose::Persona,
            TEST_MODEL,
            "Respond only in natural prose.",
            vec![CodexInputItem::UserText {
                text: "Say something true.".into(),
            }],
            Vec::<CodexToolDefinition>::new(),
            RequestShape {
                max_output_tokens: 1_200,
                parallel_tool_calls: false,
            },
        )
        .expect("the request builds");
        let prepared = port.prepare(request).expect("the port prepares");
        let _ = port.infer(prepared).await.expect("the port infers");

        let sent: Value = serde_json::from_str(&responder.bodies()[0]).expect("the body is JSON");
        let object = sent.as_object().expect("the body is a JSON object");
        assert!(!object.contains_key("tools"), "{sent}");
        assert!(!object.contains_key("parallel_tool_calls"), "{sent}");
    }

    /// Spec test (PA.f22, Soul's S2). The tool declarations offered actually
    /// reach the request body; a lowering that always sent an empty `tools`
    /// array would pass every other test in this module.
    #[tokio::test]
    async fn offered_tools_are_declared_in_the_request_body() {
        let responder = ScriptedResponder::start(vec![(200, text_reply("ok"))]).await;
        let port = port(responder.endpoint());
        let request = tool_request(
            CommandId::new(),
            0,
            InferencePurpose::Elaboration,
            TEST_MODEL,
            "Author the shortfall.",
            vec![CodexInputItem::UserText {
                text: "Author the shortfall.".into(),
            }],
            patch_tools(),
            seed_shape(),
        )
        .expect("the request builds");
        let prepared = port.prepare(request).expect("the port prepares");
        let _ = port.infer(prepared).await.expect("the port infers");

        let sent: Value = serde_json::from_str(&responder.bodies()[0]).expect("the body is JSON");
        let tools = sent["tools"].as_array().expect("tools is an array");
        assert_eq!(tools.len(), patch_tools().len(), "{sent}");
        let names: Vec<&str> = tools
            .iter()
            .map(|tool| tool["function"]["name"].as_str().expect("a tool name"))
            .collect();
        assert!(names.contains(&RECORD_GAP_PATCH_TOOL), "{names:?}");
        assert!(names.contains(&SUBMIT_PATCH_TOOL), "{names:?}");
    }

    /// Spec test (PA.f22, Soul's S3). The model prefix is stripped before the
    /// request is sent.
    #[tokio::test]
    async fn the_model_prefix_is_stripped_in_the_request_body() {
        let responder = ScriptedResponder::start(vec![(200, text_reply("ok"))]).await;
        let port = port(responder.endpoint());
        let request = tool_request(
            CommandId::new(),
            0,
            InferencePurpose::Persona,
            TEST_MODEL,
            "Respond only in natural prose.",
            vec![CodexInputItem::UserText {
                text: "Say something true.".into(),
            }],
            Vec::<CodexToolDefinition>::new(),
            RequestShape {
                max_output_tokens: 1_200,
                parallel_tool_calls: false,
            },
        )
        .expect("the request builds");
        let prepared = port.prepare(request).expect("the port prepares");
        let _ = port.infer(prepared).await.expect("the port infers");

        let sent: Value = serde_json::from_str(&responder.bodies()[0]).expect("the body is JSON");
        assert_eq!(
            sent["model"],
            TEST_MODEL.strip_prefix(DEFAULT_LOCAL_MODEL_PREFIX).unwrap()
        );
    }

    /// Spec test (PA.f22, Soul's S4). The foreign-caller gate in `infer`
    /// refuses a persisted invocation stamped with a different runtime
    /// identity than the one this port was configured with.
    #[tokio::test]
    async fn infer_refuses_a_foreign_caller() {
        let responder = ScriptedResponder::start(vec![(200, text_reply("ok"))]).await;
        let port = port(responder.endpoint());
        let request = tool_request(
            CommandId::new(),
            0,
            InferencePurpose::Persona,
            TEST_MODEL,
            "Respond only in natural prose.",
            vec![CodexInputItem::UserText {
                text: "Say something true.".into(),
            }],
            Vec::<CodexToolDefinition>::new(),
            RequestShape {
                max_output_tokens: 1_200,
                parallel_tool_calls: false,
            },
        )
        .expect("the request builds");
        let mut prepared = port.prepare(request).expect("the port prepares");
        prepared.invocation.caller_runtime_id = "someone-elses-runtime".into();
        let fault = port
            .infer(prepared)
            .await
            .expect_err("a foreign caller's invocation produced an output");
        assert!(fault.integrity_was_violated(), "{fault:?}");
        assert!(responder.bodies().is_empty(), "a foreign invocation reached the endpoint");
    }

    /// Spec test (PA.f22, Soul's S4). The expiry gate in `infer` refuses a
    /// persisted invocation whose expiry has already passed.
    #[tokio::test]
    async fn infer_refuses_an_expired_invocation() {
        let responder = ScriptedResponder::start(vec![(200, text_reply("ok"))]).await;
        let port = port(responder.endpoint());
        let request = tool_request(
            CommandId::new(),
            0,
            InferencePurpose::Persona,
            TEST_MODEL,
            "Respond only in natural prose.",
            vec![CodexInputItem::UserText {
                text: "Say something true.".into(),
            }],
            Vec::<CodexToolDefinition>::new(),
            RequestShape {
                max_output_tokens: 1_200,
                parallel_tool_calls: false,
            },
        )
        .expect("the request builds");
        let mut prepared = port.prepare(request).expect("the port prepares");
        prepared.invocation.expires_at_unix_ms = 1;
        let fault = port
            .infer(prepared)
            .await
            .expect_err("an expired invocation produced an output");
        assert!(!fault.integrity_was_violated(), "{fault:?}");
        assert!(responder.bodies().is_empty(), "an expired invocation reached the endpoint");
    }

    /// Spec test (PA.f22, Soul's S6). The receipt digest changes when the
    /// reply's `response_id` changes and nothing else does.
    #[tokio::test]
    async fn the_receipt_digest_changes_with_the_response_id() {
        let request = tool_request(
            CommandId::new(),
            0,
            InferencePurpose::Persona,
            TEST_MODEL,
            "Respond only in natural prose.",
            vec![CodexInputItem::UserText {
                text: "Say something true.".into(),
            }],
            Vec::<CodexToolDefinition>::new(),
            RequestShape {
                max_output_tokens: 1_200,
                parallel_tool_calls: false,
            },
        )
        .expect("the request builds");

        let first_responder =
            ScriptedResponder::start(vec![(200, text_reply_with_id("resp-aaa", "ok"))]).await;
        let first_port = port(first_responder.endpoint());
        let first_prepared = first_port.prepare(request.clone()).expect("the port prepares");
        let first_output = first_port.infer(first_prepared).await.expect("the port infers");

        let second_responder =
            ScriptedResponder::start(vec![(200, text_reply_with_id("resp-bbb", "ok"))]).await;
        let second_port = port(second_responder.endpoint());
        let second_prepared = second_port.prepare(request).expect("the port prepares");
        let second_output = second_port.infer(second_prepared).await.expect("the port infers");

        assert_ne!(
            first_output.receipt_digest, second_output.receipt_digest,
            "a changed response_id did not change the receipt digest"
        );
    }

    fn plain_request() -> InferenceRequest {
        tool_request(
            CommandId::new(),
            0,
            InferencePurpose::Persona,
            TEST_MODEL,
            "Respond only in natural prose.",
            vec![CodexInputItem::UserText {
                text: "Say something true.".into(),
            }],
            Vec::<CodexToolDefinition>::new(),
            RequestShape {
                max_output_tokens: 1_200,
                parallel_tool_calls: false,
            },
        )
        .expect("the request builds")
    }

    async fn infer_once(port: &LocalInferencePort) -> Result<InferenceOutput, InferenceFault> {
        let prepared = port.prepare(plain_request()).expect("the port prepares");
        port.infer(prepared).await
    }

    /// A loopback server that accepts the first `drops` connections and
    /// closes each without a byte (the shape of a model server reloading),
    /// then answers every later connection with `reply`.
    async fn closing_then_serving(drops: usize, reply: String) -> SocketAddr {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("loopback binds");
        let addr = listener.local_addr().expect("a bound listener has an address");
        let state = Arc::new(Mutex::new(ScriptedResponderState {
            replies: VecDeque::from(vec![(200, reply)]),
            ..Default::default()
        }));
        tokio::spawn(async move {
            let mut seen = 0;
            loop {
                let Ok((mut stream, _)) = listener.accept().await else {
                    return;
                };
                if seen < drops {
                    seen += 1;
                    let _ = stream.shutdown().await;
                    continue;
                }
                serve_one(stream, &state).await;
            }
        });
        addr
    }

    /// R3. A send that dies after the connection was made but before any
    /// status arrives is `Retryable` and classed `NoResponse`; the recovered
    /// endpoint then answers the very next call from the same port. Mutation:
    /// classify only `is_connect()` errors as retryable, everything else as
    /// recovery-required (the pre-cut shape) and the first assertion fails.
    #[tokio::test]
    async fn an_empty_reply_is_retryable_and_the_recovered_endpoint_answers() {
        let addr = closing_then_serving(1, text_reply("back")).await;
        let port = port(addr);
        let fault = infer_once(&port).await.expect_err("a closed connection produced an output");
        assert_eq!(fault.disposition(), InferenceFaultDisposition::Retryable, "{fault:?}");
        assert_eq!(fault.class(), InferenceFaultClass::NoResponse, "{fault:?}");
        infer_once(&port).await.expect("the recovered endpoint answers the retry");
    }

    /// R3's other half: a timeout is not retried, because the endpoint may
    /// still be generating. The listener accepts and never answers.
    #[tokio::test]
    async fn a_timed_out_send_is_recovery_required_not_retried() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            let mut held = Vec::new();
            while let Ok((stream, _)) = listener.accept().await {
                held.push(stream);
            }
        });
        let port = {
            let _guard = client_build_lock().lock().expect("the client-build lock is never poisoned");
            LocalInferencePort::with_timeout(
                addr,
                DEFAULT_LOCAL_MODEL_PREFIX,
                TEST_RUNTIME,
                std::time::Duration::from_millis(150),
            )
            .expect("a loopback endpoint opens")
        };
        let fault = infer_once(&port).await.expect_err("a silent endpoint produced an output");
        assert_eq!(fault.disposition(), InferenceFaultDisposition::RecoveryRequired, "{fault:?}");
        assert_eq!(fault.class(), InferenceFaultClass::Timeout, "{fault:?}");
    }

    /// The class is a typed value for every way a send can fail, and the
    /// endpoint appears in none of the fault's renderings: the canary is the
    /// endpoint this test bound, so a `Display` that carries the request URL
    /// (reqwest's does) fails here.
    #[tokio::test]
    async fn every_send_failure_is_classed_and_names_no_endpoint() {
        let closed = {
            let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
            let addr = listener.local_addr().unwrap();
            drop(listener);
            addr
        };
        let dropping = closing_then_serving(usize::MAX, String::new()).await;
        let status = ScriptedResponder::start(vec![(400, "{}".to_owned())]).await.endpoint();
        let malformed = ScriptedResponder::start(vec![(200, "not json".to_owned())]).await.endpoint();
        let cases = [
            (closed, InferenceFaultClass::Connect),
            (dropping, InferenceFaultClass::NoResponse),
            (status, InferenceFaultClass::Status(400)),
            (malformed, InferenceFaultClass::BadReply),
        ];
        for (addr, expected) in cases {
            let fault = infer_once(&port(addr)).await.expect_err("the send was expected to fail");
            assert_eq!(fault.class(), expected, "{fault:?}");
            let rendered = format!("{fault} | {fault:?}");
            for canary in [addr.to_string(), addr.ip().to_string(), "chat/completions".to_owned()] {
                assert!(!rendered.contains(&canary), "{canary} leaked into {rendered}");
            }
        }
    }

    fn port_with_timeout(endpoint: SocketAddr, timeout: std::time::Duration) -> LocalInferencePort {
        let _guard = client_build_lock()
            .lock()
            .expect("the client-build lock is never poisoned");
        LocalInferencePort::with_timeout(Endpoint::Loopback(endpoint), DEFAULT_LOCAL_MODEL_PREFIX, TEST_RUNTIME, timeout)
            .expect("a loopback endpoint opens")
    }

    /// A loopback server that answers every request with a `200` whose
    /// header promises 1000 body bytes and whose body is a single `{`, then
    /// either holds the connection open (`stall`) or closes it (a cut body).
    async fn truncated_body_endpoint(stall: bool) -> SocketAddr {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("loopback binds");
        let addr = listener.local_addr().expect("a bound listener has an address");
        tokio::spawn(async move {
            let mut held = Vec::new();
            while let Ok((mut stream, _)) = listener.accept().await {
                let mut request = vec![0u8; 65536];
                let _ = stream.read(&mut request).await;
                let _ = stream
                    .write_all(b"HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: 1000\r\n\r\n{")
                    .await;
                if stall {
                    held.push(stream);
                } else {
                    let _ = stream.shutdown().await;
                }
            }
        });
        addr
    }

    /// One classifier: a timeout while the body streams is `Timeout`, not
    /// retried. Mutants: classify the body-read error as `BadReply` again, or
    /// treat a stall as retryable.
    #[tokio::test]
    async fn a_body_that_stalls_is_a_timeout_not_retried() {
        let addr = truncated_body_endpoint(true).await;
        // Bounded, so a client that ignores the timeout it was given fails
        // here instead of waiting out a longer one.
        let fault = tokio::time::timeout(
            std::time::Duration::from_secs(10),
            infer_once(&port_with_timeout(addr, std::time::Duration::from_millis(300))),
        )
        .await
        .expect("the client honours the timeout it was given")
        .expect_err("a stalled body produced an output");
        assert_eq!(fault.class(), InferenceFaultClass::Timeout, "{fault:?}");
        assert_eq!(fault.disposition(), InferenceFaultDisposition::RecoveryRequired, "{fault:?}");
    }

    /// A body cut off mid-stream (a reload dropping the connection during the
    /// reply) is a reply that never completed: `NoResponse`, retryable.
    #[tokio::test]
    async fn a_body_cut_off_mid_stream_is_retryable() {
        let addr = truncated_body_endpoint(false).await;
        let fault = infer_once(&port(addr)).await.expect_err("a cut body produced an output");
        assert_eq!(fault.class(), InferenceFaultClass::NoResponse, "{fault:?}");
        assert_eq!(fault.disposition(), InferenceFaultDisposition::Retryable, "{fault:?}");
    }

    /// A body that arrived whole but is not the declared shape stays `BadReply`
    /// and an integrity violation, never retried.
    #[tokio::test]
    async fn a_complete_malformed_body_is_a_bad_reply() {
        let addr = ScriptedResponder::start(vec![(200, "not json".to_owned())]).await.endpoint();
        let fault = infer_once(&port(addr)).await.expect_err("a malformed body produced an output");
        assert_eq!(fault.class(), InferenceFaultClass::BadReply, "{fault:?}");
        assert_eq!(fault.disposition(), InferenceFaultDisposition::IntegrityViolation, "{fault:?}");
    }

    /// The malformed-body leg of the no-input-text rule: serde's `Display`
    /// for a type mismatch echoes the offending value, so a detail built from
    /// it carries the reply's own text. The canary is a value inside a whole
    /// body that fails to deserialize.
    #[tokio::test]
    async fn a_malformed_body_carries_neither_its_own_text_nor_the_endpoint() {
        const CANARY: &str = "CANARY-reply-7f3e91";
        let addr = ScriptedResponder::start(vec![(200, format!("\"{CANARY}\""))]).await.endpoint();
        let fault = infer_once(&port(addr)).await.expect_err("a malformed body produced an output");
        assert_eq!(fault.class(), InferenceFaultClass::BadReply, "{fault:?}");
        let rendered = format!("{fault} | {fault:?}");
        for canary in [CANARY.to_owned(), addr.to_string(), addr.ip().to_string()] {
            assert!(!rendered.contains(&canary), "{canary} leaked into {rendered}");
        }
    }

    /// The production default is `RESPONSE_TIMEOUT` (a cold model load lives
    /// inside one request) at the constructor and at the port a binding
    /// opens: a shorter default turns every cold start into a Timeout fault
    /// that is never retried.
    #[test]
    fn the_production_default_timeout_is_response_timeout_at_both_resolving_sites() {
        assert_eq!(RESPONSE_TIMEOUT, std::time::Duration::from_secs(900));
        let endpoint: SocketAddr = "127.0.0.1:9".parse().unwrap();
        let constructed = {
            let _guard = client_build_lock().lock().expect("the client-build lock is never poisoned");
            LocalInferencePort::new(Endpoint::Loopback(endpoint), DEFAULT_LOCAL_MODEL_PREFIX, TEST_RUNTIME)
                .expect("a loopback endpoint opens")
        };
        assert_eq!(constructed.response_timeout, RESPONSE_TIMEOUT);
        let opened = {
            let _guard = client_build_lock().lock().expect("the client-build lock is never poisoned");
            local_port(LocalBinding {
                endpoint,
                model_prefix: DEFAULT_LOCAL_MODEL_PREFIX.into(),
                caller_runtime_id: TEST_RUNTIME.into(),
            })
            .expect("a loopback binding opens")
        };
        assert_eq!(opened.response_timeout, RESPONSE_TIMEOUT);
    }

    /// The timeout leg of the no-endpoint rule: reqwest's `Display` for a
    /// timeout ends with the request URL, so a detail built from it names the
    /// canary endpoint this test bound. Covers both a stalled send and a
    /// stalled body.
    #[tokio::test]
    async fn a_timeout_names_no_endpoint() {
        let silent = {
            let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
            let addr = listener.local_addr().unwrap();
            tokio::spawn(async move {
                let mut held = Vec::new();
                while let Ok((stream, _)) = listener.accept().await {
                    held.push(stream);
                }
            });
            addr
        };
        let stalled_body = truncated_body_endpoint(true).await;
        for addr in [silent, stalled_body] {
            let fault = infer_once(&port_with_timeout(addr, std::time::Duration::from_millis(200)))
                .await
                .expect_err("a stalled endpoint produced an output");
            assert_eq!(fault.class(), InferenceFaultClass::Timeout, "{fault:?}");
            let rendered = format!("{fault} | {fault:?}");
            for canary in [addr.to_string(), addr.ip().to_string(), addr.port().to_string(), "chat/completions".to_owned()] {
                assert!(!rendered.contains(&canary), "{canary} leaked into {rendered}");
            }
        }
    }

    // ---- The hosted binding and token usage (cut hosted-lane) ----

    /// A synthetic bearer, generated for the test and written to a temp key
    /// file, so the production reader is the path under test.
    const SYNTHETIC_KEY: &str = "synthetic-bearer-0123456789abcdef";

    fn write_key_file(directory: &tempfile::TempDir, content: &[u8]) -> PathBuf {
        let path = directory.path().join("provider.key");
        std::fs::write(&path, content).unwrap();
        path
    }

    fn hosted_binding(base_url: &str, key_path: PathBuf) -> HostedBinding {
        HostedBinding {
            base_url: base_url.into(),
            key_path,
            model_prefix: DEFAULT_HOSTED_MODEL_PREFIX.into(),
            caller_runtime_id: TEST_RUNTIME.into(),
        }
    }

    /// The test-only seam: the production key reader and the production
    /// builder, with the https requirement lifted so the endpoint can be a
    /// loopback responder with no TLS.
    fn hosted_port_over_plain_http(base_url: String, key_path: &std::path::Path) -> LocalInferencePort {
        let _guard = client_build_lock()
            .lock()
            .expect("the client-build lock is never poisoned");
        LocalInferencePort::build(
            Endpoint::Hosted {
                base_url,
                key: read_key_file(key_path).expect("the synthetic key file reads"),
            },
            DEFAULT_HOSTED_MODEL_PREFIX,
            TEST_RUNTIME,
            RESPONSE_TIMEOUT,
            true,
        )
        .expect("the hosted seam opens")
    }

    fn usage_reply(usage: Value) -> String {
        json!({
            "id": "resp-usage",
            "choices": [{
                "message": {"role": "assistant", "content": "ok", "tool_calls": []},
                "finish_reason": "stop",
            }],
            "usage": usage,
        })
        .to_string()
    }

    #[test]
    fn hosted_binding_refuses_plain_http() {
        let directory = tempfile::tempdir().unwrap();
        let key_path = write_key_file(&directory, SYNTHETIC_KEY.as_bytes());
        for url in ["http://api.example.test", "ftp://api.example.test", "api.example.test", ""] {
            let error = open_hosted_port(hosted_binding(url, key_path.clone()))
                .err()
                .expect("a non-https hosted endpoint opened");
            assert!(matches!(error, ControllerOpenError::HostedEndpointNotHttps), "{url}");
            assert!(!error.to_string().contains("example"), "the refusal echoed the URL");
        }
        let through_open_inference = open_inference(
            None,
            None,
            None,
            Some(hosted_binding("http://api.example.test", key_path.clone())),
            &["hosted/deepseek-chat"],
        )
        .err()
        .expect("open_inference opened a plain-http hosted endpoint");
        assert!(matches!(through_open_inference, ControllerOpenError::HostedEndpointNotHttps));
        assert!(open_hosted_port(hosted_binding("https://api.example.test", key_path)).is_ok());
    }

    #[test]
    fn hosted_binding_refuses_an_empty_key_file() {
        let directory = tempfile::tempdir().unwrap();
        let absent = directory.path().join("absent.key");
        let cases: Vec<(&str, PathBuf)> = vec![
            ("empty", write_key_file(&directory, b"")),
            ("only a newline", write_key_file(&directory, b"\n")),
            ("padded", write_key_file(&directory, b" key-with-padding \n")),
            ("not utf-8", write_key_file(&directory, &[0xff, 0xfe, 0xfd])),
            ("absent", absent),
        ];
        for (label, path) in cases {
            let canary = path.display().to_string();
            let error = open_hosted_port(hosted_binding("https://api.example.test", path))
                .err()
                .unwrap_or_else(|| panic!("{label}: a bad key file opened"));
            assert!(matches!(error, ControllerOpenError::HostedKeyUnreadable), "{label}");
            let rendered = format!("{error} | {error:?}");
            assert!(!rendered.contains(&canary), "{label}: the path leaked");
            assert!(!rendered.contains("provider.key"), "{label}: the file name leaked");
        }
        // A key file that ends in the usual newline is the key without it.
        let ok = write_key_file(&directory, b"fine-key\r\n");
        assert_eq!(read_key_file(&ok).unwrap().as_str(), "fine-key");
    }

    #[tokio::test]
    async fn hosted_request_carries_the_bearer_and_loopback_carries_none() {
        let directory = tempfile::tempdir().unwrap();
        let key_path = write_key_file(&directory, format!("{SYNTHETIC_KEY}\n").as_bytes());

        let hosted_responder = ScriptedResponder::start(vec![(200, text_reply("ok"))]).await;
        let hosted = hosted_port_over_plain_http(format!("http://{}", hosted_responder.endpoint()), &key_path);
        let output = infer_once(&hosted).await.expect("the hosted port infers");
        assert_eq!(output.events, vec![InferenceEvent::Text("ok".into())]);
        let header_block = hosted_responder.headers()[0].to_ascii_lowercase();
        assert!(
            header_block.contains(&format!("authorization: bearer {}", SYNTHETIC_KEY.to_ascii_lowercase())),
            "{header_block}"
        );

        let local_responder = ScriptedResponder::start(vec![(200, text_reply("ok"))]).await;
        infer_once(&port(local_responder.endpoint())).await.expect("the local port infers");
        assert!(!local_responder.headers()[0].to_ascii_lowercase().contains("authorization"));
    }

    #[tokio::test]
    async fn the_hosted_request_strips_its_prefix_and_names_the_v1_path() {
        let directory = tempfile::tempdir().unwrap();
        let key_path = write_key_file(&directory, SYNTHETIC_KEY.as_bytes());
        let responder = ScriptedResponder::start(vec![(200, text_reply("ok"))]).await;
        // A trailing slash on the origin must not double the path separator.
        let hosted = hosted_port_over_plain_http(format!("http://{}/", responder.endpoint()), &key_path);
        let request = tool_request(
            CommandId::new(),
            0,
            InferencePurpose::Persona,
            "hosted/deepseek-chat",
            "Respond only in natural prose.",
            vec![CodexInputItem::UserText { text: "Say something true.".into() }],
            Vec::<CodexToolDefinition>::new(),
            RequestShape { max_output_tokens: 1_200, parallel_tool_calls: false },
        )
        .expect("the request builds");
        let prepared = hosted.prepare(request).unwrap();
        hosted.infer(prepared).await.expect("the hosted port infers");
        assert!(responder.headers()[0].starts_with("POST /v1/chat/completions HTTP/1.1"));
        let body: Value = serde_json::from_str(&responder.bodies()[0]).unwrap();
        assert_eq!(body["model"], "deepseek-chat");
    }

    /// The bearer must not follow a redirect, and no fault, log line or
    /// receipt may carry the key or the endpoint.
    #[tokio::test]
    async fn the_hosted_bearer_is_never_sent_to_a_redirect_target_or_echoed() {
        let directory = tempfile::tempdir().unwrap();
        let key_path = write_key_file(&directory, SYNTHETIC_KEY.as_bytes());
        let target = ScriptedResponder::start(vec![(200, text_reply("reached"))]).await;
        let redirecting = start_redirecting_to(target.endpoint()).await;
        let hosted = hosted_port_over_plain_http(format!("http://{redirecting}"), &key_path);
        let fault = infer_once(&hosted).await.expect_err("a redirect produced an output");
        assert!(target.headers().is_empty(), "the redirect target was reached");

        let refusing = ScriptedResponder::start(vec![(401, "{}".to_owned())]).await;
        let hosted = hosted_port_over_plain_http(format!("http://{}", refusing.endpoint()), &key_path);
        let unauthorised = infer_once(&hosted).await.expect_err("a 401 produced an output");
        let ok_responder = ScriptedResponder::start(vec![(200, text_reply("ok"))]).await;
        let hosted = hosted_port_over_plain_http(format!("http://{}", ok_responder.endpoint()), &key_path);
        let output = infer_once(&hosted).await.expect("the hosted port infers");

        let rendered = format!(
            "{fault} | {fault:?} | {unauthorised} | {unauthorised:?} | {output:?}"
        );
        assert!(!rendered.contains(SYNTHETIC_KEY), "the key leaked: {rendered}");
        for addr in [redirecting, refusing.endpoint(), ok_responder.endpoint()] {
            assert!(!rendered.contains(&addr.port().to_string()), "an endpoint leaked: {rendered}");
        }
    }

    #[tokio::test]
    async fn usage_reaches_inference_output() {
        let directory = tempfile::tempdir().unwrap();
        let key_path = write_key_file(&directory, SYNTHETIC_KEY.as_bytes());
        let responder = ScriptedResponder::start(vec![
            (
                200,
                usage_reply(json!({
                    "prompt_tokens": 120,
                    "completion_tokens": 7,
                    "prompt_cache_hit_tokens": 100,
                    "prompt_cache_miss_tokens": 20,
                })),
            ),
            (200, usage_reply(json!({"prompt_tokens": 31, "completion_tokens": 4}))),
        ])
        .await;
        for (hosted, expected) in [
            (
                true,
                TokenUsage { prompt: 120, completion: 7, cached_prompt: Some(100) },
            ),
            (
                false,
                TokenUsage { prompt: 31, completion: 4, cached_prompt: None },
            ),
        ] {
            let output = if hosted {
                let port =
                    hosted_port_over_plain_http(format!("http://{}", responder.endpoint()), &key_path);
                infer_once(&port).await.unwrap()
            } else {
                infer_once(&port(responder.endpoint())).await.unwrap()
            };
            assert_eq!(output.usage(), Some(expected));
        }
    }

    /// A reply without usage is a typed absence. The receipt keeps the zero
    /// counts it always carried (the local lane's receipt is unchanged), so
    /// the digest equals the digest of a reply that reported zeroes, while the
    /// output tells the two apart.
    #[tokio::test]
    async fn a_reply_without_usage_is_none_and_the_local_receipt_is_unchanged() {
        let absent = json!({
            "id": "resp-usage",
            "choices": [{
                "message": {"role": "assistant", "content": "ok", "tool_calls": []},
                "finish_reason": "stop",
            }],
        })
        .to_string();
        let responder = ScriptedResponder::start(vec![
            (200, absent),
            (200, usage_reply(json!({"prompt_tokens": 0, "completion_tokens": 0}))),
            (200, usage_reply(json!({"prompt_tokens": 9}))),
            (200, usage_reply(json!({"prompt_tokens": null, "completion_tokens": 3}))),
        ])
        .await;
        let port = port(responder.endpoint());
        let none = infer_once(&port).await.unwrap();
        let zeroes = infer_once(&port).await.unwrap();
        let half = infer_once(&port).await.unwrap();
        let null = infer_once(&port).await.unwrap();
        assert_eq!(none.usage(), None);
        assert_eq!(zeroes.usage(), Some(TokenUsage { prompt: 0, completion: 0, cached_prompt: None }));
        assert_eq!(half.usage(), None, "a reply missing a count reported a zero");
        assert_eq!(null.usage(), None);
        assert_eq!(none.receipt_digest(), zeroes.receipt_digest());
    }

    #[test]
    fn a_hosted_prefix_routes_to_the_hosted_port_and_shares_no_prefix() {
        let directory = tempfile::tempdir().unwrap();
        let key_path = write_key_file(&directory, SYNTHETIC_KEY.as_bytes());
        let local_binding = || LocalBinding {
            endpoint: "127.0.0.1:1".parse().unwrap(),
            model_prefix: DEFAULT_LOCAL_MODEL_PREFIX.into(),
            caller_runtime_id: TEST_RUNTIME.into(),
        };
        let hosted = || hosted_binding("https://api.example.test", key_path.clone());
        let models = ["local/llama-8b", "hosted/deepseek-chat"];
        assert!(open_inference(None, None, Some(local_binding()), Some(hosted()), &models).is_ok());
        // Without the hosted binding its model has no owner: loud, not a fallback.
        assert!(matches!(
            open_inference(None, None, Some(local_binding()), None, &models),
            Err(ControllerOpenError::UnroutableModel { .. })
        ));
        let mut clashing = hosted();
        clashing.model_prefix = DEFAULT_LOCAL_MODEL_PREFIX.into();
        assert!(matches!(
            open_inference(None, None, Some(local_binding()), Some(clashing), &["local/x"]),
            Err(ControllerOpenError::SharedModelPrefix { .. })
        ));
        let mut empty = hosted();
        empty.model_prefix = String::new();
        assert!(matches!(
            open_inference(None, None, None, Some(empty), &["x"]),
            Err(ControllerOpenError::EmptyModelPrefix { transport: "hosted" })
        ));

        // Routing itself: each prefix reaches its own port.
        let local_port = Arc::new(port("127.0.0.1:1".parse().unwrap())) as Arc<dyn InferencePort>;
        let hosted_port = Arc::new(port("127.0.0.1:1".parse().unwrap())) as Arc<dyn InferencePort>;
        let routed = RoutedInferencePort::new(
            None,
            None,
            "claude",
            vec![
                ("local/".to_owned(), Arc::clone(&local_port)),
                ("hosted/".to_owned(), Arc::clone(&hosted_port)),
            ],
        );
        assert!(Arc::ptr_eq(routed.route("local/a").unwrap(), &local_port));
        assert!(Arc::ptr_eq(routed.route("hosted/a").unwrap(), &hosted_port));
        assert!(routed.route("other").is_none());
    }
}
