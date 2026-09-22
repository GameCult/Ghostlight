//! A local OpenAI-compatible inference port, for a loopback model server this
//! process does not authenticate to and does not need a credential for.
//!
//! This module owns one authority: lowering a prepared request to one
//! `POST /v1/chat/completions` call and lifting the reply back into the same
//! `InferenceOutput` every other port produces. It runs no tool loop of its
//! own — a returned tool call is inert, exactly as the connector lane's is —
//! and it holds no mutex across a request, so several requests may be in
//! flight through the same port at once.

use super::controllers::{
    ControllerOpenError, InferenceEvent, InferenceFault, InferenceOutput, InferencePort,
    InferenceRequest, PreparedInference, REQUEST_EXPIRY, RESPONSE_TIMEOUT, call_id_is_valid,
    tool_name_is_valid, unix_ms,
};
use async_trait::async_trait;
use codex_connector::CodexInputItem;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::net::SocketAddr;
use std::sync::Arc;

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
    #[serde(default)]
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

#[derive(Debug, Deserialize)]
struct LocalChatUsage {
    #[serde(default)]
    prompt_tokens: u64,
    #[serde(default)]
    completion_tokens: u64,
}

/// The local OpenAI-compatible backend behind Ghostlight's inference seam.
/// Unlike the SDK port, it runs no query loop: one prepared request lowers to
/// one HTTP call, and any tool call in the reply comes back inert for Rust's
/// own evaluator to drive the next round, exactly as the connector lane
/// already does.
pub(super) struct LocalInferencePort {
    client: reqwest::Client,
    endpoint: SocketAddr,
    prefix: String,
    caller_runtime_id: String,
}

impl LocalInferencePort {
    /// The loopback check happens here, in the constructor, so a non-loopback
    /// endpoint cannot reach this port through any path that builds one —
    /// not only the one `open_inference` exercises today. The client is
    /// built with `.no_proxy()` so an environment proxy variable can never
    /// carry this request, or a `proxy-authorization` header derived from a
    /// proxy URL's userinfo, off this loopback endpoint; and with redirects
    /// disabled, so a 3xx reply cannot re-POST the request's contents to a
    /// second endpoint this port never opened.
    fn new(
        endpoint: SocketAddr,
        prefix: impl Into<String>,
        caller_runtime_id: impl Into<String>,
    ) -> Result<Self, ControllerOpenError> {
        if !endpoint.ip().is_loopback() {
            return Err(ControllerOpenError::LocalEndpointNotLoopback { endpoint });
        }
        Ok(Self {
            client: reqwest::Client::builder()
                .no_proxy()
                .redirect(reqwest::redirect::Policy::none())
                .timeout(RESPONSE_TIMEOUT)
                .build()
                .expect("the local inference HTTP client builds with no custom TLS material"),
            endpoint,
            prefix: prefix.into(),
            caller_runtime_id: caller_runtime_id.into(),
        })
    }

    async fn send(&self, body: Value) -> Result<LocalChatResponse, InferenceFault> {
        let response = self
            .client
            .post(format!("http://{}/v1/chat/completions", self.endpoint))
            .json(&body)
            .send()
            .await
            .map_err(|error| {
                if error.is_connect() {
                    InferenceFault::retryable(format!(
                        "the local inference endpoint refused the connection: {error}"
                    ))
                } else {
                    InferenceFault::new(format!("the local inference request failed: {error}"))
                }
            })?;
        let status = response.status();
        if !status.is_success() {
            let detail = format!("the local inference endpoint returned {status}");
            return Err(if status.as_u16() == 429 || status.as_u16() == 503 {
                InferenceFault::retryable(detail)
            } else {
                InferenceFault::new(detail)
            });
        }
        response.json::<LocalChatResponse>().await.map_err(|error| {
            InferenceFault::integrity_violation(format!(
                "the local inference endpoint's reply was not the declared shape: {error}"
            ))
        })
    }
}

/// Lowers one prepared request to the OpenAI chat-completions body: the
/// system message is the request's instructions, every input item maps to
/// one message in order, and every offered tool is declared `strict`. No
/// header but content type is ever set on the request this builds.
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
        let parameters: Value = serde_json::from_str(&tool.parameters_json).map_err(|error| {
            InferenceFault::integrity_violation(format!(
                "tool `{}` parameters are not valid JSON: {error}",
                tool.name
            ))
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
            .map(|usage| usage.prompt_tokens)
            .unwrap_or_default(),
        completion_tokens: response
            .usage
            .as_ref()
            .map(|usage| usage.completion_tokens)
            .unwrap_or_default(),
    };
    let receipt_bytes = rmp_serde::to_vec_named(&receipt)
        .map_err(|error| InferenceFault::new(error.to_string()))?;
    Ok(InferenceOutput::new(
        events,
        format!("sha256:{:x}", Sha256::digest(&receipt_bytes)),
    ))
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
    Ok(Arc::new(LocalInferencePort::new(
        binding.endpoint,
        binding.model_prefix,
        binding.caller_runtime_id,
    )?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::controllers::{InferencePurpose, RequestShape, open_inference, tool_request};
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
        LocalInferencePort::new(endpoint, DEFAULT_LOCAL_MODEL_PREFIX, TEST_RUNTIME)
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
            &[TEST_MODEL],
        )
        .err()
        .expect("a non-loopback local endpoint opened");
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
            Some(("claude-local/".to_owned(), Arc::clone(&local_port))),
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
}
