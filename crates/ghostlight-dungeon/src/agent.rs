use crate::model::{
    ModelPort, ModelStageReceipt, ModelStageRequest, mark_model_receipt_semantic_invalid,
    run_validated_stage,
};
use async_trait::async_trait;
use serde::{Serialize, de::DeserializeOwned};
use std::collections::BTreeSet;

pub struct ModelAgentSpec {
    pub stage: String,
    pub model: String,
    pub snapshot_binding: String,
    pub instructions: String,
    pub source_receipt_ids: Vec<String>,
    pub temperature: Option<f64>,
    pub max_output_tokens: Option<u32>,
    pub max_steps: usize,
}

#[derive(Debug)]
pub struct ModelAgentRun<Output> {
    pub output: Output,
    pub receipts: Vec<ModelStageReceipt>,
}

#[derive(Debug)]
pub struct ModelAgentFailure {
    pub message: String,
    pub receipts: Vec<ModelStageReceipt>,
}

impl std::fmt::Display for ModelAgentFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for ModelAgentFailure {}

pub struct ModelAgentToolContext {
    /// Causal model/tool receipt ancestry for this action. These IDs prove how
    /// the action was produced; they are not source evidence for canonical
    /// world facts or profiles.
    pub source_receipt_ids: Vec<String>,
}

pub enum ModelAgentToolOutcome<Output, Finding> {
    /// The typed action was applied to the tool's private workbench, but the
    /// agent has not yet produced a terminal result. Canonical state remains
    /// untouched; the observation describes the updated draft or next need.
    Continue {
        observation: Finding,
        receipts: Vec<ModelStageReceipt>,
    },
    Accepted {
        output: Output,
        receipts: Vec<ModelStageReceipt>,
    },
    Rejected {
        finding: Finding,
        receipts: Vec<ModelStageReceipt>,
    },
    Failed {
        message: String,
        receipts: Vec<ModelStageReceipt>,
    },
}

#[async_trait]
pub trait ModelAgentTool: Send {
    type Action: DeserializeOwned + Serialize + Send;
    type Output: Send;
    type Finding: Serialize + Send;

    /// The tool owns the action space that is legal at its current private
    /// workbench state. The agent loop asks again before every model step so a
    /// rejected action can narrow the next call to the exact repair surface.
    fn action_schema(&self) -> std::result::Result<serde_json::Value, String>;

    async fn invoke(
        &mut self,
        action: Self::Action,
        context: &ModelAgentToolContext,
    ) -> ModelAgentToolOutcome<Self::Output, Self::Finding>;
}

pub async fn run_model_agent<Tool: ModelAgentTool>(
    port: &dyn ModelPort,
    spec: &ModelAgentSpec,
    tool: &mut Tool,
) -> std::result::Result<ModelAgentRun<Tool::Output>, ModelAgentFailure> {
    if spec.max_steps == 0 {
        return Err(ModelAgentFailure {
            message: format!("model agent {} has no semantic step budget", spec.stage),
            receipts: Vec::new(),
        });
    }

    let mut receipts = Vec::new();
    let mut transcript = String::new();
    let mut last_observation = None;
    for step in 0..spec.max_steps {
        let action_schema = match tool.action_schema() {
            Ok(schema) => schema,
            Err(message) => {
                return Err(ModelAgentFailure {
                    message: format!(
                        "model agent {} tool could not publish its current action schema: {message}",
                        spec.stage
                    ),
                    receipts,
                });
            }
        };
        let action_schema_text = match serde_json::to_string(&action_schema) {
            Ok(schema) => schema,
            Err(error) => {
                return Err(ModelAgentFailure {
                    message: format!(
                        "model agent {} could not serialize its current action schema: {error}",
                        spec.stage
                    ),
                    receipts,
                });
            }
        };
        let source_receipt_ids = causal_source_ids(&spec.source_receipt_ids, &receipts);
        let request = ModelStageRequest {
            stage: spec.stage.clone(),
            model: spec.model.clone(),
            snapshot_binding: spec.snapshot_binding.clone(),
            lived_stream: format!(
                "CURRENT LEGAL TOOL ACTION SCHEMA:\n{}\n\n{}{}\n\nAGENT STEP: {} of {}. Choose one typed tool action admitted by the current schema. The harness will execute it against the frozen state and return the real tool observation. Do not claim success yourself; only an accepted tool result can end the task.",
                action_schema_text,
                spec.instructions,
                transcript,
                step + 1,
                spec.max_steps,
            ),
            output_schema: Some(action_schema),
            source_receipt_ids,
            temperature: spec.temperature,
            max_output_tokens: spec.max_output_tokens,
        };
        let mut stage = match run_validated_stage(port, &request).await {
            Ok(stage) => stage,
            Err(error) => {
                return Err(ModelAgentFailure {
                    message: format!(
                        "model agent {} transport or schema stage failed: {error}",
                        spec.stage
                    ),
                    receipts,
                });
            }
        };
        let action_value = match stage.structured.take() {
            Some(value) => value,
            None => {
                let finding = serde_json::json!({
                    "kind":"action_decode_error",
                    "message":"agent returned no typed action",
                });
                mark_model_receipt_semantic_invalid(&mut stage.receipt, &finding);
                transcript.push_str(&tool_observation(
                    step,
                    "rejected",
                    &serde_json::Value::Null,
                    &finding,
                ));
                last_observation = Some(finding);
                receipts.push(stage.receipt);
                continue;
            }
        };
        let action = match serde_json::from_value::<Tool::Action>(action_value.clone()) {
            Ok(action) => action,
            Err(error) => {
                let finding = serde_json::json!({
                    "kind":"action_decode_error",
                    "message":error.to_string(),
                });
                mark_model_receipt_semantic_invalid(&mut stage.receipt, &finding);
                transcript.push_str(&tool_observation(step, "rejected", &action_value, &finding));
                last_observation = Some(finding);
                receipts.push(stage.receipt);
                continue;
            }
        };
        let tool_context = ModelAgentToolContext {
            source_receipt_ids: causal_source_ids(
                &spec.source_receipt_ids,
                &receipts
                    .iter()
                    .cloned()
                    .chain(std::iter::once(stage.receipt.clone()))
                    .collect::<Vec<_>>(),
            ),
        };
        match tool.invoke(action, &tool_context).await {
            ModelAgentToolOutcome::Continue {
                observation,
                receipts: tool_receipts,
            } => {
                let observation = match serde_json::to_value(observation) {
                    Ok(observation) => observation,
                    Err(error) => {
                        let message = format!(
                            "model agent {} could not serialize its typed tool observation: {error}",
                            spec.stage
                        );
                        mark_model_receipt_semantic_invalid(&mut stage.receipt, &message);
                        receipts.push(stage.receipt);
                        receipts.extend(tool_receipts);
                        return Err(ModelAgentFailure { message, receipts });
                    }
                };
                transcript.push_str(&tool_observation(
                    step,
                    "continued",
                    &action_value,
                    &observation,
                ));
                last_observation = Some(observation);
                receipts.push(stage.receipt);
                receipts.extend(tool_receipts);
            }
            ModelAgentToolOutcome::Accepted {
                output,
                receipts: tool_receipts,
            } => {
                receipts.push(stage.receipt);
                receipts.extend(tool_receipts);
                return Ok(ModelAgentRun { output, receipts });
            }
            ModelAgentToolOutcome::Rejected {
                finding,
                receipts: tool_receipts,
            } => {
                let finding = match serde_json::to_value(finding) {
                    Ok(finding) => finding,
                    Err(error) => {
                        let message = format!(
                            "model agent {} could not serialize its typed tool finding: {error}",
                            spec.stage
                        );
                        mark_model_receipt_semantic_invalid(&mut stage.receipt, &message);
                        receipts.push(stage.receipt);
                        receipts.extend(tool_receipts);
                        return Err(ModelAgentFailure { message, receipts });
                    }
                };
                mark_model_receipt_semantic_invalid(&mut stage.receipt, &finding);
                transcript.push_str(&tool_observation(step, "rejected", &action_value, &finding));
                last_observation = Some(finding);
                receipts.push(stage.receipt);
                receipts.extend(tool_receipts);
            }
            ModelAgentToolOutcome::Failed {
                message,
                receipts: tool_receipts,
            } => {
                mark_model_receipt_semantic_invalid(&mut stage.receipt, &message);
                receipts.push(stage.receipt);
                receipts.extend(tool_receipts);
                return Err(ModelAgentFailure { message, receipts });
            }
        }
    }

    Err(ModelAgentFailure {
        message: format!(
            "model agent {} exhausted {} semantic steps; final tool observation: {}",
            spec.stage,
            spec.max_steps,
            last_observation
                .map(|observation| observation.to_string())
                .unwrap_or_else(|| "none".into()),
        ),
        receipts,
    })
}

/// Builds one deterministic causal ancestry set for every model stage. This is
/// model/tool provenance only; callers must not reuse it as canonical world
/// evidence.
pub(crate) fn causal_source_ids(base: &[String], receipts: &[ModelStageReceipt]) -> Vec<String> {
    base.iter()
        .cloned()
        .chain(
            receipts
                .iter()
                .map(|receipt| receipt.storage_key().to_owned()),
        )
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn tool_observation(
    step: usize,
    status: &str,
    action: &serde_json::Value,
    observation: &serde_json::Value,
) -> String {
    format!(
        "\n\nTOOL OBSERVATION AFTER AGENT STEP {} ({status}):\n{}\nPREVIOUS TOOL ACTION:\n{}\nReturn the next complete typed tool action. Preserve frozen identity and useful valid draft work; change only what the observation actually rejects or requests.",
        step + 1,
        serde_json::to_string(observation).unwrap_or_else(|_| "null".into()),
        serde_json::to_string(action).unwrap_or_else(|_| "null".into()),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{MODEL_BALANCED, ModelProviderOutput};
    use anyhow::Result;
    use async_trait::async_trait;
    use schemars::{JsonSchema, schema_for};
    use serde::{Deserialize, Serialize};
    use std::sync::Mutex;

    #[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
    struct CandidateAction {
        value: String,
    }

    #[derive(Clone, Debug, Serialize)]
    #[serde(tag = "kind", rename_all = "snake_case")]
    enum ExactFinding {
        DraftStored { value: String },
        WrongValue { received: String },
    }

    struct ScriptedModel {
        outputs: Mutex<Vec<String>>,
        requests: Mutex<Vec<ModelStageRequest>>,
    }

    #[async_trait]
    impl ModelPort for ScriptedModel {
        async fn run(&self, request: &ModelStageRequest) -> Result<String> {
            self.requests.lock().unwrap().push(request.clone());
            Ok(self.outputs.lock().unwrap().remove(0))
        }

        async fn run_observed(&self, request: &ModelStageRequest) -> Result<ModelProviderOutput> {
            Ok(ModelProviderOutput {
                content: self.run(request).await?,
                resolved_model: Some("gpt-5.6-terra".into()),
                ..Default::default()
            })
        }

        fn provider(&self) -> &'static str {
            "fixture"
        }
    }

    struct ExactTool;

    #[async_trait]
    impl ModelAgentTool for ExactTool {
        type Action = CandidateAction;
        type Output = String;
        type Finding = ExactFinding;

        fn action_schema(&self) -> std::result::Result<serde_json::Value, String> {
            serde_json::to_value(schema_for!(CandidateAction)).map_err(|error| error.to_string())
        }

        async fn invoke(
            &mut self,
            action: Self::Action,
            _context: &ModelAgentToolContext,
        ) -> ModelAgentToolOutcome<Self::Output, Self::Finding> {
            if action.value == "accepted" {
                ModelAgentToolOutcome::Accepted {
                    output: action.value,
                    receipts: Vec::new(),
                }
            } else if action.value == "drafted" {
                ModelAgentToolOutcome::Continue {
                    observation: ExactFinding::DraftStored {
                        value: action.value,
                    },
                    receipts: Vec::new(),
                }
            } else {
                ModelAgentToolOutcome::Rejected {
                    finding: ExactFinding::WrongValue {
                        received: action.value.clone(),
                    },
                    receipts: vec![fixture_tool_receipt(&action.value)],
                }
            }
        }
    }

    struct NarrowingTool {
        draft_stored: bool,
    }

    #[async_trait]
    impl ModelAgentTool for NarrowingTool {
        type Action = CandidateAction;
        type Output = String;
        type Finding = ExactFinding;

        fn action_schema(&self) -> std::result::Result<serde_json::Value, String> {
            let value = if self.draft_stored {
                "accepted"
            } else {
                "drafted"
            };
            Ok(serde_json::json!({
                "type":"object",
                "additionalProperties":false,
                "required":["value"],
                "properties":{"value":{"const":value}}
            }))
        }

        async fn invoke(
            &mut self,
            action: Self::Action,
            _context: &ModelAgentToolContext,
        ) -> ModelAgentToolOutcome<Self::Output, Self::Finding> {
            if self.draft_stored {
                assert_eq!(action.value, "accepted");
                ModelAgentToolOutcome::Accepted {
                    output: action.value,
                    receipts: Vec::new(),
                }
            } else {
                assert_eq!(action.value, "drafted");
                self.draft_stored = true;
                ModelAgentToolOutcome::Continue {
                    observation: ExactFinding::DraftStored {
                        value: action.value,
                    },
                    receipts: Vec::new(),
                }
            }
        }
    }

    fn fixture_tool_receipt(value: &str) -> ModelStageReceipt {
        ModelStageReceipt {
            schema: "ghostlight.model_stage_receipt.v1".into(),
            receipt_hash: String::new(),
            provider: "fixture-tool".into(),
            model: "deterministic".into(),
            stage: "fixture_tool_validation".into(),
            snapshot_binding: "snapshot:1".into(),
            request_hash: format!("request:{value}"),
            output_hash: format!("tool:{value}"),
            source_receipt_ids: Vec::new(),
            latency_ms: 0,
            validation_result: "semantic_invalid".into(),
            local_validation_error: Some(format!("rejected {value}")),
            input_chars: 0,
            output_chars: 0,
            provider_attempts: Vec::new(),
        }
    }

    #[tokio::test]
    async fn semantic_tool_observation_drives_a_causally_receipted_second_action() {
        let model = ScriptedModel {
            outputs: Mutex::new(vec![
                r#"{"value":"rejected"}"#.into(),
                r#"{"value":"accepted"}"#.into(),
            ]),
            requests: Mutex::new(Vec::new()),
        };
        let spec = ModelAgentSpec {
            stage: "fixture_agent".into(),
            model: MODEL_BALANCED.into(),
            snapshot_binding: "snapshot:1".into(),
            instructions: "Use the tool.".into(),
            source_receipt_ids: vec!["source:one".into()],
            temperature: Some(0.0),
            max_output_tokens: Some(128),
            max_steps: 2,
        };

        let run = run_model_agent(&model, &spec, &mut ExactTool)
            .await
            .unwrap();

        assert_eq!(run.output, "accepted");
        assert_eq!(run.receipts.len(), 3);
        assert_eq!(run.receipts[0].validation_result, "semantic_invalid");
        assert_eq!(run.receipts[1].stage, "fixture_tool_validation");
        assert_eq!(run.receipts[2].validation_result, "valid");
        assert!(
            run.receipts[2]
                .source_receipt_ids
                .contains(&run.receipts[0].storage_key().to_owned())
        );
        assert!(
            run.receipts[2]
                .source_receipt_ids
                .contains(&run.receipts[1].storage_key().to_owned())
        );
        let requests = model.requests.lock().unwrap();
        assert!(requests[1].lived_stream.contains("wrong_value"));
        assert!(requests[1].lived_stream.contains("PREVIOUS TOOL ACTION"));
    }

    #[tokio::test]
    async fn successful_nonterminal_tool_action_preserves_draft_progress() {
        let model = ScriptedModel {
            outputs: Mutex::new(vec![
                r#"{"value":"drafted"}"#.into(),
                r#"{"value":"accepted"}"#.into(),
            ]),
            requests: Mutex::new(Vec::new()),
        };
        let spec = ModelAgentSpec {
            stage: "fixture_agent".into(),
            model: MODEL_BALANCED.into(),
            snapshot_binding: "snapshot:1".into(),
            instructions: "Use the tool.".into(),
            source_receipt_ids: vec!["source:one".into()],
            temperature: Some(0.0),
            max_output_tokens: Some(128),
            max_steps: 2,
        };

        let run = run_model_agent(&model, &spec, &mut ExactTool)
            .await
            .unwrap();

        assert_eq!(run.output, "accepted");
        assert_eq!(run.receipts.len(), 2);
        assert_eq!(run.receipts[0].validation_result, "valid");
        assert_eq!(run.receipts[1].validation_result, "valid");
        let requests = model.requests.lock().unwrap();
        assert!(requests[1].lived_stream.contains("continued"));
        assert!(requests[1].lived_stream.contains("draft_stored"));
        assert!(
            requests[1]
                .source_receipt_ids
                .contains(&run.receipts[0].storage_key().to_owned())
        );
    }

    #[tokio::test]
    async fn tool_state_owns_each_steps_exact_action_schema() {
        let model = ScriptedModel {
            outputs: Mutex::new(vec![
                r#"{"value":"drafted"}"#.into(),
                r#"{"value":"accepted"}"#.into(),
            ]),
            requests: Mutex::new(Vec::new()),
        };
        let spec = ModelAgentSpec {
            stage: "fixture_agent".into(),
            model: MODEL_BALANCED.into(),
            snapshot_binding: "snapshot:1".into(),
            instructions: "Use the current tool contract.".into(),
            source_receipt_ids: vec!["source:one".into()],
            temperature: Some(0.0),
            max_output_tokens: Some(128),
            max_steps: 2,
        };
        let mut tool = NarrowingTool {
            draft_stored: false,
        };

        let run = run_model_agent(&model, &spec, &mut tool).await.unwrap();

        assert_eq!(run.output, "accepted");
        let requests = model.requests.lock().unwrap();
        assert_eq!(
            requests[0].output_schema.as_ref().unwrap()["properties"]["value"]["const"],
            "drafted"
        );
        assert_eq!(
            requests[1].output_schema.as_ref().unwrap()["properties"]["value"]["const"],
            "accepted"
        );
        assert!(
            requests[1]
                .lived_stream
                .contains("CURRENT LEGAL TOOL ACTION SCHEMA")
        );
        assert!(requests[1].lived_stream.contains(r#""const":"accepted""#));
    }

    #[tokio::test]
    async fn exhausted_agent_preserves_every_action_and_tool_receipt() {
        let model = ScriptedModel {
            outputs: Mutex::new(vec![
                r#"{"value":"first"}"#.into(),
                r#"{"value":"second"}"#.into(),
            ]),
            requests: Mutex::new(Vec::new()),
        };
        let spec = ModelAgentSpec {
            stage: "fixture_agent".into(),
            model: MODEL_BALANCED.into(),
            snapshot_binding: "snapshot:1".into(),
            instructions: "Use the tool.".into(),
            source_receipt_ids: vec!["source:one".into()],
            temperature: Some(0.0),
            max_output_tokens: Some(128),
            max_steps: 2,
        };

        let failure = run_model_agent(&model, &spec, &mut ExactTool)
            .await
            .unwrap_err();

        assert!(failure.message.contains("exhausted 2 semantic steps"));
        assert_eq!(failure.receipts.len(), 4);
        assert_eq!(failure.receipts[0].validation_result, "semantic_invalid");
        assert_eq!(failure.receipts[1].stage, "fixture_tool_validation");
        assert_eq!(failure.receipts[2].validation_result, "semantic_invalid");
        assert_eq!(failure.receipts[3].stage, "fixture_tool_validation");
        assert!(
            failure.receipts[2]
                .source_receipt_ids
                .contains(&failure.receipts[0].storage_key().to_owned())
        );
        assert!(
            failure.receipts[2]
                .source_receipt_ids
                .contains(&failure.receipts[1].storage_key().to_owned())
        );
    }
}
