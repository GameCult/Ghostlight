use crate::{
    domain::{ActionAssessment, ActionIntent, Campaign, ContextModifier},
    model::{ModelPort, ModelStageReceipt, ModelStageRequest, run_validated_stage},
};
use anyhow::{Result, anyhow};
use chrono::{Duration, Utc};
use schemars::{JsonSchema, schema_for};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, sync::Arc};

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
struct AssessmentProposal {
    normalized_intent: String,
    admissible: bool,
    missing_permission: Option<String>,
    dc: u8,
    modifiers: Vec<ContextModifier>,
    effect_ceiling: String,
    success_stake: String,
    mixed_stake: String,
    failure_stake: String,
    bargains: Vec<String>,
}

pub struct ActionAssessor {
    model: Arc<dyn ModelPort>,
    model_id: String,
}
impl ActionAssessor {
    pub fn new(model: Arc<dyn ModelPort>, model_id: impl Into<String>) -> Self {
        Self {
            model,
            model_id: model_id.into(),
        }
    }
    pub async fn assess(
        &self,
        campaign: &Campaign,
        intent: ActionIntent,
    ) -> Result<(ActionAssessment, ModelStageReceipt)> {
        let actor = campaign
            .actors
            .get(&intent.actor_id)
            .ok_or_else(|| anyhow!("actor does not exist in this branch"))?;
        let location = campaign
            .locations
            .get(&actor.location_id)
            .ok_or_else(|| anyhow!("actor location is invalid"))?;
        let visible_institutions: Vec<_> = campaign
            .institutions
            .values()
            .map(|x| serde_json::json!({"id":x.id,"name":x.name,"posture":x.posture}))
            .collect();
        let allowed_references = allowed_references(campaign, actor);
        let schema = serde_json::to_value(schema_for!(AssessmentProposal))?;
        let prompt = format!(
            "Assess an attempted effect, not whether words can be spoken. Impossible actions are inadmissible and receive bargains, not a roll. Choose DC only from 5,10,15,20,25,30. Every modifier reference must be copied exactly from ALLOWED REFERENCES. Modifier total is capped at +/-10. Never grant capability, custody, access, knowledge, or spatial reach absent from state. State concrete success, mixed, and failure consequences and a bounded effect ceiling.\nINTENT:\n{}\nACTOR:\n{}\nLOCATION:\n{}\nVISIBLE INSTITUTIONS:\n{}\nALLOWED REFERENCES:\n{}\nOUTPUT JSON SCHEMA:\n{}",
            serde_json::to_string(&intent)?,
            serde_json::to_string(actor)?,
            serde_json::to_string(location)?,
            serde_json::to_string(&visible_institutions)?,
            serde_json::to_string(&allowed_references)?,
            serde_json::to_string_pretty(&schema)?
        );
        let out = run_validated_stage(
            self.model.as_ref(),
            &ModelStageRequest {
                stage: "action_assessment".into(),
                model: self.model_id.clone(),
                snapshot_binding: format!(
                    "campaign:{}:revision:{}",
                    campaign.id, campaign.revision
                ),
                lived_stream: prompt,
                output_schema: Some(schema),
                source_receipt_ids: campaign.branch_origin.evidence_receipt_ids.clone(),
            },
        )
        .await?;
        let proposal: AssessmentProposal = serde_json::from_value(
            out.structured
                .ok_or_else(|| anyhow!("assessor returned no typed proposal"))?,
        )?;
        validate_proposal(&proposal, &allowed_references)?;
        let modifier_total =
            crate::d20::capped_modifier(proposal.modifiers.iter().map(|m| m.value));
        let mut assessment = ActionAssessment {
            schema: "ghostlight.player_action_assessment.v1".into(),
            campaign_id: campaign.id,
            revision: campaign.revision,
            intent,
            admissible: proposal.admissible,
            missing_permission: proposal.missing_permission,
            dc: proposal.dc,
            modifiers: proposal.modifiers,
            modifier_total,
            effect_ceiling: proposal.effect_ceiling,
            success_stake: proposal.success_stake,
            mixed_stake: proposal.mixed_stake,
            failure_stake: proposal.failure_stake,
            bargains: proposal.bargains,
            expires_at: Utc::now() + Duration::minutes(10),
            digest: String::new(),
        };
        assessment.digest = assessment_digest(&assessment)?;
        Ok((assessment, out.receipt))
    }
}

fn allowed_references(campaign: &Campaign, actor: &crate::domain::ActorState) -> BTreeSet<String> {
    let mut refs = BTreeSet::from([
        format!("actor:{}", actor.id),
        format!("location:{}", actor.location_id),
    ]);
    for value in actor
        .capabilities
        .iter()
        .map(|x| format!("capability:{x}"))
        .chain(actor.knowledge.iter().map(|x| format!("knowledge:{x}")))
        .chain(actor.equipment.iter().map(|x| format!("equipment:{x}")))
        .chain(actor.conditions.iter().map(|x| format!("condition:{x}")))
        .chain(actor.obligations.iter().map(|x| format!("obligation:{x}")))
    {
        refs.insert(value);
    }
    for id in campaign.institutions.keys() {
        refs.insert(format!("institution:{id}"));
    }
    for id in campaign.facts.keys() {
        refs.insert(format!("fact:{id}"));
    }
    for id in &campaign.branch_origin.evidence_receipt_ids {
        refs.insert(id.clone());
    }
    refs
}
fn validate_proposal(p: &AssessmentProposal, allowed: &BTreeSet<String>) -> Result<()> {
    if ![5, 10, 15, 20, 25, 30].contains(&p.dc) {
        return Err(anyhow!("assessor chose invalid DC"));
    }
    if p.modifiers
        .iter()
        .any(|m| m.value < -10 || m.value > 10 || m.references.iter().any(|r| !allowed.contains(r)))
    {
        return Err(anyhow!(
            "assessor used an invalid modifier or unearned reference"
        ));
    }
    if p.admissible && p.missing_permission.is_some() {
        return Err(anyhow!("admissible assessment claims missing permission"));
    }
    if !p.admissible && p.missing_permission.is_none() {
        return Err(anyhow!(
            "inadmissible assessment omitted missing permission"
        ));
    }
    Ok(())
}
pub fn assessment_digest(assessment: &ActionAssessment) -> Result<String> {
    let mut value = assessment.clone();
    value.digest.clear();
    Ok(format!(
        "sha256:{:x}",
        Sha256::digest(rmp_serde::to_vec_named(&value)?)
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn proposal(reference: &str) -> AssessmentProposal {
        AssessmentProposal {
            normalized_intent: "open the gate".into(),
            admissible: true,
            missing_permission: None,
            dc: 15,
            modifiers: vec![ContextModifier {
                label: "tool".into(),
                value: 2,
                references: vec![reference.into()],
            }],
            effect_ceiling: "open this gate".into(),
            success_stake: "gate opens".into(),
            mixed_stake: "gate opens noisily".into(),
            failure_stake: "lock jams".into(),
            bargains: vec![],
        }
    }

    #[test]
    fn assessment_rejects_unearned_state_reference() {
        let allowed = BTreeSet::from(["equipment:key".into()]);
        assert!(validate_proposal(&proposal("capability:telepathy"), &allowed).is_err());
        assert!(validate_proposal(&proposal("equipment:key"), &allowed).is_ok());
    }

    #[test]
    fn assessment_rejects_noncanonical_dc() {
        let mut value = proposal("equipment:key");
        value.dc = 17;
        assert!(validate_proposal(&value, &BTreeSet::from(["equipment:key".into()])).is_err());
    }
}
