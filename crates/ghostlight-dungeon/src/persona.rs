use crate::model::{ModelPort, ModelStageRequest, run_validated_stage};
use anyhow::{Result, anyhow};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct PermittedActorSlice {
    pub actor_id: String,
    pub snapshot_binding: String,
    pub identity_experience: Vec<String>,
    pub memories: Vec<String>,
    pub perceived_events: Vec<String>,
    pub relationships: Vec<String>,
    pub goals: Vec<String>,
    pub knowledge: Vec<String>,
    pub capabilities: Vec<String>,
    pub pressures: Vec<String>,
    pub affordances: Vec<String>,
    pub source_receipt_ids: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct LivedNarrativeStream {
    pub text: String,
    pub snapshot_binding: String,
    pub projector_receipt: crate::model::ModelStageReceipt,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct PersonaProposalBundle {
    pub private_deltas: Vec<serde_json::Value>,
    pub speech: Option<String>,
    pub reaction_priority: i16,
    pub world_actions: Vec<serde_json::Value>,
}

#[derive(Clone, Debug)]
pub struct PersonaTerminalBundle {
    pub lived_stream: LivedNarrativeStream,
    pub persona_output: String,
    pub proposals: PersonaProposalBundle,
    pub stage_receipts: Vec<crate::model::ModelStageReceipt>,
}

#[async_trait]
pub trait ExecutionPermit: Send + Sync {
    async fn require(&self, actor_id: &str, snapshot_binding: &str, stage: &str) -> Result<()>;
}

#[derive(Clone)]
pub struct AllowAllPermit;
#[async_trait]
impl ExecutionPermit for AllowAllPermit {
    async fn require(&self, _: &str, _: &str, _: &str) -> Result<()> {
        Ok(())
    }
}

pub struct PersonaProjectionEngine {
    pub model: Arc<dyn ModelPort>,
    pub permit: Arc<dyn ExecutionPermit>,
    pub projector_model: String,
    pub persona_model: String,
    pub interpreter_model: String,
}

impl PersonaProjectionEngine {
    pub async fn execute(&self, slice: PermittedActorSlice) -> Result<PersonaTerminalBundle> {
        self.permit
            .require(&slice.actor_id, &slice.snapshot_binding, "projector")
            .await?;
        let projector_prompt = format!(
            "Project this permitted typed experience into one private lived narrative stream. Emit narrative only; no JSON, action syntax, schemas, or raw field labels.\n\n{}",
            serde_json::to_string(&slice)?
        );
        let projected = run_validated_stage(
            self.model.as_ref(),
            &ModelStageRequest {
                stage: "projector".into(),
                model: self.projector_model.clone(),
                snapshot_binding: slice.snapshot_binding.clone(),
                lived_stream: projector_prompt,
                output_schema: None,
                source_receipt_ids: slice.source_receipt_ids.clone(),
            },
        )
        .await?;
        require_narrative_membrane(&projected.narrative)?;
        let lived = LivedNarrativeStream {
            text: projected.narrative.clone(),
            snapshot_binding: slice.snapshot_binding.clone(),
            projector_receipt: projected.receipt.clone(),
        };

        self.permit
            .require(&slice.actor_id, &slice.snapshot_binding, "persona")
            .await?;
        // This is the membrane: the Persona's domain context is exactly one lived stream.
        let persona = run_validated_stage(
            self.model.as_ref(),
            &ModelStageRequest {
                stage: "persona".into(),
                model: self.persona_model.clone(),
                snapshot_binding: slice.snapshot_binding.clone(),
                lived_stream: lived.text.clone(),
                output_schema: None,
                source_receipt_ids: vec![],
            },
        )
        .await?;

        self.permit
            .require(&slice.actor_id, &slice.snapshot_binding, "interpreter")
            .await?;
        let schema = serde_json::json!({"type":"object","required":["private_deltas","speech","reaction_priority","world_actions"]});
        let interpreted = run_validated_stage(
            self.model.as_ref(),
            &ModelStageRequest {
                stage: "interpreter".into(),
                model: self.interpreter_model.clone(),
                snapshot_binding: slice.snapshot_binding.clone(),
                lived_stream: format!(
                    "LIVED STREAM:\n{}\n\nPERSONA OUTPUT:\n{}",
                    lived.text, persona.narrative
                ),
                output_schema: Some(schema),
                source_receipt_ids: slice.source_receipt_ids,
            },
        )
        .await?;
        self.permit
            .require(&slice.actor_id, &slice.snapshot_binding, "terminal")
            .await?;
        let proposals: PersonaProposalBundle = serde_json::from_value(
            interpreted
                .structured
                .clone()
                .ok_or_else(|| anyhow!("interpreter produced no typed proposal"))?,
        )?;
        Ok(PersonaTerminalBundle {
            lived_stream: lived,
            persona_output: persona.narrative,
            proposals,
            stage_receipts: vec![projected.receipt, persona.receipt, interpreted.receipt],
        })
    }
}

fn require_narrative_membrane(text: &str) -> Result<()> {
    let trimmed = text.trim();
    if trimmed.is_empty() || trimmed.starts_with('{') || trimmed.contains("```json") {
        return Err(anyhow!("projector violated lived-narrative membrane"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::FixtureModel;
    #[tokio::test]
    async fn persona_receives_only_projected_stream() {
        let engine = PersonaProjectionEngine {
            model: Arc::new(FixtureModel),
            permit: Arc::new(AllowAllPermit),
            projector_model: "flash".into(),
            persona_model: "pro".into(),
            interpreter_model: "flash".into(),
        };
        let slice = PermittedActorSlice {
            actor_id: "npc".into(),
            snapshot_binding: "campaign:1".into(),
            identity_experience: vec!["A tired navigator".into()],
            memories: vec![],
            perceived_events: vec![],
            relationships: vec![],
            goals: vec![],
            knowledge: vec![],
            capabilities: vec![],
            pressures: vec![],
            affordances: vec![],
            source_receipt_ids: vec![],
        };
        let result = engine.execute(slice).await.unwrap();
        assert_eq!(result.stage_receipts.len(), 3);
        assert_eq!(result.proposals.reaction_priority, 0);
    }
}
