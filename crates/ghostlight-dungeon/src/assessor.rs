use crate::{
    domain::{ActionAssessment, ActionIntent, Campaign, ContextModifier, WorldEffectDelta},
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
    strong_effect: WorldEffectDelta,
    success_effect: WorldEffectDelta,
    mixed_effect: WorldEffectDelta,
    failure_effect: WorldEffectDelta,
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
        let present_actors: Vec<_> = campaign
            .actors
            .values()
            .filter(|candidate| candidate.location_id == actor.location_id)
            .map(|candidate| {
                serde_json::json!({
                    "id": candidate.id,
                    "name": candidate.name,
                    "conditions": candidate.conditions,
                    "relationships": candidate.relationships,
                })
            })
            .collect();
        let information_facts = available_information_facts(campaign, actor);
        let allowed_references = allowed_references(campaign, actor);
        let agency_guidance = action_agency_guidance(&campaign.player_actor_id, &intent.actor_id);
        let mut schema = serde_json::to_value(schema_for!(AssessmentProposal))?;
        schema["properties"]["dc"] = serde_json::json!({
            "type":"integer",
            "enum":[5,10,15,20,25,30]
        });
        schema["$defs"]["ContextModifier"]["properties"]["value"] = serde_json::json!({
            "type":"integer",
            "minimum":-10,
            "maximum":10
        });
        let base_prompt = format!(
            "OUTPUT JSON SCHEMA (follow exactly):\n{}\n\nAssess an attempted effect, not whether words can be spoken. Impossible actions are inadmissible and receive bargains, not a roll. Choose DC only from 5,10,15,20,25,30. Every modifier reference must be copied exactly from ALLOWED REFERENCES. Modifier total is capped at +/-10. Never grant capability, custody, access, knowledge, or spatial reach absent from state. State concrete success, mixed, and failure consequences and a bounded effect ceiling. Outcome deltas may only name actor IDs copied exactly from PRESENT ACTORS, change their conditions or relationships, move only the acting actor along an existing route, advance existing clocks, or change existing institution posture. Informational outcomes may reveal only an exact statement copied from AVAILABLE INFORMATION FACTS; they never create a new fact. A location-discoverable fact may be added only to the acting actor. A fact already known by the acting actor may instead be communicated to another present actor. actor_knowledge_additions contains the player-readable statement, never a fact ID, key, slug, or label. Strong and ordinary success share one visible stake, so give them identical knowledge additions. The runtime binds each exact finding into the player-visible stake; do not spend prose repeating it solely for formatting. If no supplied fact supports the intended discovery or disclosure, leave knowledge deltas empty and make the limitation explicit in the stakes or mark the attempt inadmissible. Never invent remote events, hidden actors, unsupported proper nouns, or conclusions beyond the effect ceiling. Keep a delta empty only when the outcome truly has no canonical state change.\nAGENCY BOUNDARY:\n{}\nPLAYER ACTOR ID:\n{}\nINTENT:\n{}\nACTOR:\n{}\nLOCATION:\n{}\nPRESENT ACTORS:\n{}\nVISIBLE INSTITUTIONS:\n{}\nAVAILABLE INFORMATION FACTS:\n{}\nALLOWED REFERENCES:\n{}",
            serde_json::to_string(&schema)?,
            agency_guidance,
            campaign.player_actor_id,
            serde_json::to_string(&intent)?,
            serde_json::to_string(actor)?,
            serde_json::to_string(location)?,
            serde_json::to_string(&present_actors)?,
            serde_json::to_string(&visible_institutions)?,
            serde_json::to_string(&information_facts)?,
            serde_json::to_string(&allowed_references)?
        );
        let snapshot_binding = format!("campaign:{}:revision:{}", campaign.id, campaign.revision);
        let mut correction = String::new();
        let mut attempts = 0;
        let (proposal, out) = loop {
            attempts += 1;
            let out = run_validated_stage(
                self.model.as_ref(),
                &ModelStageRequest {
                    stage: "action_assessment".into(),
                    model: self.model_id.clone(),
                    snapshot_binding: snapshot_binding.clone(),
                    lived_stream: format!("{base_prompt}{correction}"),
                    output_schema: Some(schema.clone()),
                    source_receipt_ids: campaign.branch_origin.evidence_receipt_ids.clone(),
                    temperature: Some(0.0),
                    max_output_tokens: Some(1_800),
                },
            )
            .await?;
            let candidate = (|| -> Result<AssessmentProposal> {
                let mut proposal: AssessmentProposal = serde_json::from_value(
                    out.structured
                        .clone()
                        .ok_or_else(|| anyhow!("assessor returned no typed proposal"))?,
                )?;
                bind_visible_knowledge(&mut proposal)?;
                validate_proposal(&proposal, &allowed_references)?;
                for (effect, stake) in [
                    (&proposal.strong_effect, &proposal.success_stake),
                    (&proposal.success_effect, &proposal.success_stake),
                    (&proposal.mixed_effect, &proposal.mixed_stake),
                    (&proposal.failure_effect, &proposal.failure_stake),
                ] {
                    validate_effect(campaign, actor, effect, stake)?;
                }
                Ok(proposal)
            })();
            match candidate {
                Ok(proposal) => break (proposal, out),
                Err(error) if attempts == 1 => {
                    let rejected = out
                        .structured
                        .as_ref()
                        .and_then(|value| serde_json::to_string(value).ok())
                        .unwrap_or_else(|| "unavailable".into());
                    correction = format!(
                        "\n\nLOCAL VALIDATOR REJECTED THE PREVIOUS ASSESSMENT: {error}\nPREVIOUS ASSESSMENT:\n{rejected}\nReturn a corrected complete assessment against the same snapshot. Copy every modifier reference from ALLOWED REFERENCES exactly; omit a modifier rather than paraphrasing or inventing its reference. Copy every actor and destination ID exactly from the supplied state. Every knowledge addition must copy one exact statement from AVAILABLE INFORMATION FACTS and obey its access mode; strong and ordinary success must use identical knowledge additions. Otherwise leave the typed delta empty."
                    );
                }
                Err(error) => {
                    return Err(anyhow!(
                        "assessor failed local validation after one correction: {error}"
                    ));
                }
            }
        };
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
            strong_effect: proposal.strong_effect,
            success_effect: proposal.success_effect,
            mixed_effect: proposal.mixed_effect,
            failure_effect: proposal.failure_effect,
            bargains: proposal.bargains,
            expires_at: Utc::now() + Duration::minutes(10),
            digest: String::new(),
        };
        assessment.digest = assessment_digest(&assessment)?;
        Ok((assessment, out.receipt))
    }
}

fn action_agency_guidance(player_actor_id: &str, acting_actor_id: &str) -> &'static str {
    if acting_actor_id == player_actor_id {
        "The acting actor is player-controlled; assess the player's attempted effect without upgrading it into a completed fact."
    } else {
        "The acting actor is an NPC. The player retains authority over the player's own speech, choices, consent, beliefs, disclosures, feelings, and voluntary actions. An NPC may create pressure, make an offer or threat, reveal information, oppose the player, or change independently owned world state, but no outcome stake may assert that the player answered, chose, consented, believed, disclosed, felt, or obeyed. If the intended effect is only to obtain such a player response, mark it inadmissible because the response remains the player's next decision."
    }
}

fn bind_visible_knowledge(proposal: &mut AssessmentProposal) -> Result<()> {
    if proposal.strong_effect.actor_knowledge_additions
        != proposal.success_effect.actor_knowledge_additions
    {
        return Err(anyhow!(
            "strong and ordinary success must expose identical knowledge because they share one visible stake"
        ));
    }
    append_visible_findings(&mut proposal.success_stake, &proposal.success_effect);
    append_visible_findings(&mut proposal.mixed_stake, &proposal.mixed_effect);
    append_visible_findings(&mut proposal.failure_stake, &proposal.failure_effect);
    Ok(())
}

fn append_visible_findings(stake: &mut String, effect: &WorldEffectDelta) {
    for finding in effect
        .actor_knowledge_additions
        .values()
        .flat_map(|findings| findings.iter())
    {
        if !stake.contains(finding) {
            if !stake.trim_end().is_empty() {
                stake.push(' ');
            }
            stake.push_str("Observed finding: ");
            stake.push_str(finding);
        }
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
    for fact in available_information_facts(campaign, actor) {
        if let Some(id) = fact.get("id").and_then(serde_json::Value::as_str) {
            refs.insert(format!("fact:{id}"));
        }
    }
    for id in &campaign.branch_origin.evidence_receipt_ids {
        refs.insert(id.clone());
    }
    refs
}

fn available_information_facts(
    campaign: &Campaign,
    actor: &crate::domain::ActorState,
) -> Vec<serde_json::Value> {
    campaign
        .facts
        .values()
        .filter_map(|fact| {
            let access = if actor.knowledge.contains(&fact.statement) {
                "known_by_actor"
            } else if fact
                .discoverable_at_location_ids
                .contains(&actor.location_id)
            {
                "discoverable_here"
            } else {
                return None;
            };
            Some(serde_json::json!({
                "id": fact.id,
                "statement": fact.statement,
                "access": access,
            }))
        })
        .collect()
}
fn validate_proposal(p: &AssessmentProposal, allowed: &BTreeSet<String>) -> Result<()> {
    if ![5, 10, 15, 20, 25, 30].contains(&p.dc) {
        return Err(anyhow!("assessor chose invalid DC"));
    }
    let invalid_values = p
        .modifiers
        .iter()
        .filter(|modifier| modifier.value < -10 || modifier.value > 10)
        .map(|modifier| format!("{}={}", modifier.label, modifier.value))
        .collect::<Vec<_>>();
    let invalid_references = p
        .modifiers
        .iter()
        .flat_map(|modifier| modifier.references.iter())
        .filter(|reference| !allowed.contains(*reference))
        .cloned()
        .collect::<BTreeSet<_>>();
    if !invalid_values.is_empty() || !invalid_references.is_empty() {
        return Err(anyhow!(
            "assessor modifier validation failed; out-of-range values [{}]; references absent from ALLOWED REFERENCES [{}]",
            invalid_values.join(", "),
            invalid_references
                .into_iter()
                .collect::<Vec<_>>()
                .join(", ")
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

pub(crate) fn validate_effect(
    campaign: &Campaign,
    acting_actor: &crate::domain::ActorState,
    effect: &WorldEffectDelta,
    stake: &str,
) -> Result<()> {
    let affected = effect
        .actor_conditions
        .keys()
        .chain(effect.actor_knowledge_additions.keys())
        .chain(effect.actor_relationship_updates.keys());
    for id in affected {
        let target = campaign
            .actors
            .get(id)
            .ok_or_else(|| anyhow!("outcome delta invented an actor"))?;
        if target.location_id != acting_actor.location_id {
            return Err(anyhow!("outcome delta exceeds spatial reach"));
        }
    }
    for delta in effect.actor_conditions.values() {
        if delta.add.iter().any(|value| value.trim().is_empty())
            || delta.remove.iter().any(|value| value.trim().is_empty())
            || !delta.add.is_disjoint(&delta.remove)
        {
            return Err(anyhow!("outcome condition delta is contradictory"));
        }
    }
    for (actor_id, additions) in &effect.actor_knowledge_additions {
        let target = campaign
            .actors
            .get(actor_id)
            .ok_or_else(|| anyhow!("outcome knowledge target vanished"))?;
        let invalid_finding = additions.iter().find(|finding| {
            let Some(fact) = campaign
                .facts
                .values()
                .find(|fact| fact.statement == finding.as_str())
            else {
                return true;
            };
            let accessible = if actor_id == &acting_actor.id {
                fact.discoverable_at_location_ids
                    .contains(&acting_actor.location_id)
            } else {
                acting_actor.knowledge.contains(*finding)
            };
            !accessible || target.knowledge.contains(*finding)
        });
        if additions.is_empty()
            || additions.len() > 4
            || additions.iter().any(|finding| {
                finding.trim().is_empty()
                    || finding.chars().count() > 500
                    || looks_like_identifier(finding)
                    || !stake.contains(finding)
            })
            || invalid_finding.is_some()
        {
            return Err(anyhow!(
                "outcome knowledge must copy an existing accessible WorldFact statement: a location-discoverable fact may go only to the acting actor, while another present actor may receive only a fact already known by the acting actor; every finding must be new to its recipient and visible verbatim in its stake"
            ));
        }
    }
    for relationships in effect.actor_relationship_updates.values() {
        if relationships.iter().any(|(id, value)| {
            (!campaign.actors.contains_key(id) && !campaign.institutions.contains_key(id))
                || value.trim().is_empty()
        }) {
            return Err(anyhow!("outcome relationship delta invented a target"));
        }
    }
    for (actor_id, destination) in &effect.actor_moves {
        if actor_id != &acting_actor.id
            || !campaign.locations.contains_key(destination)
            || !campaign.locations[&acting_actor.location_id]
                .routes
                .contains_key(destination)
        {
            return Err(anyhow!("outcome movement exceeds spatial reach"));
        }
    }
    if effect
        .clock_advances
        .iter()
        .any(|(id, amount)| *amount == 0 || !campaign.clocks.contains_key(id))
        || effect.institution_postures.iter().any(|(id, posture)| {
            !campaign.institutions.contains_key(id) || posture.trim().is_empty()
        })
    {
        return Err(anyhow!("outcome delta cites unknown world state"));
    }
    Ok(())
}

fn looks_like_identifier(value: &str) -> bool {
    let value = value.trim();
    !value.chars().any(char::is_whitespace)
        && (value.contains('_')
            || value.contains("::")
            || value.starts_with("fact:")
            || value.starts_with("fact-"))
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
            strong_effect: WorldEffectDelta::default(),
            success_effect: WorldEffectDelta::default(),
            mixed_effect: WorldEffectDelta::default(),
            failure_effect: WorldEffectDelta::default(),
            bargains: vec![],
        }
    }

    #[test]
    fn assessment_rejects_unearned_state_reference() {
        let allowed = BTreeSet::from(["equipment:key".into()]);
        let error = validate_proposal(&proposal("capability:telepathy"), &allowed)
            .unwrap_err()
            .to_string();
        assert!(error.contains("capability:telepathy"));
        assert!(error.contains("ALLOWED REFERENCES"));
        assert!(validate_proposal(&proposal("equipment:key"), &allowed).is_ok());
    }

    #[test]
    fn assessment_rejects_noncanonical_dc() {
        let mut value = proposal("equipment:key");
        value.dc = 17;
        assert!(validate_proposal(&value, &BTreeSet::from(["equipment:key".into()])).is_err());
    }

    #[test]
    fn npc_assessment_guidance_preserves_player_decision_authority() {
        let guidance = action_agency_guidance("player", "archivist");
        assert!(guidance.contains("player retains authority"));
        assert!(guidance.contains("response remains the player's next decision"));
        assert!(!action_agency_guidance("player", "player").contains("acting actor is an NPC"));
    }

    #[test]
    fn exact_knowledge_is_bound_into_visible_stakes_without_model_duplication() {
        let finding = "The relay's backup cell is depleted.".to_string();
        let mut value = proposal("equipment:key");
        let additions = std::collections::BTreeMap::from([(
            "player".into(),
            BTreeSet::from([finding.clone()]),
        )]);
        value.strong_effect.actor_knowledge_additions = additions.clone();
        value.success_effect.actor_knowledge_additions = additions;

        bind_visible_knowledge(&mut value).unwrap();

        assert!(value.success_stake.contains(&finding));
        assert_eq!(value.success_stake.matches(&finding).count(), 1);
    }

    #[test]
    fn fact_identifier_cannot_masquerade_as_character_knowledge() {
        assert!(looks_like_identifier("fact_distress_relay_antenna_damaged"));
        assert!(!looks_like_identifier(
            "The distress relay's antenna array is damaged."
        ));
    }
}
