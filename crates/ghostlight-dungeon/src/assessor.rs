use crate::{
    domain::{ActionAssessment, ActionIntent, Campaign, ContextModifier, WorldEffectDelta},
    model::{
        ModelPort, ModelProviderAttemptReceipt, ModelStageReceipt, ModelStageRequest,
        ModelTokenUsage, run_validated_stage,
    },
    persistence::CampaignStore,
    session_zero::{AggregatedBoundary, CampaignContract, ExtraordinaryPermission},
};
use anyhow::{Result, anyhow};
use chrono::{Duration, Utc};
use schemars::{JsonSchema, schema_for};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, sync::Arc};

const ASSESSMENT_PROPOSAL_CACHE_KIND: &str = "assessment_proposal_cache.v1";
const ASSESSMENT_PROPOSAL_CACHE_SCHEMA: &str = "ghostlight.private.assessment_proposal_cache.v1";
const ASSESSMENT_SEMANTICS_VERSION: &str = "ghostlight.action_assessment.v2";

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
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

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
struct AssessmentProposalCacheEntry {
    schema: String,
    basis_digest: String,
    proposal: AssessmentProposal,
    source_provider: String,
    source_model: String,
    source_receipt_hash: String,
    created_at: chrono::DateTime<Utc>,
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
        self.assess_with_permissions(campaign, intent, &[]).await
    }

    pub async fn assess_with_permissions(
        &self,
        campaign: &Campaign,
        intent: ActionIntent,
        extraordinary_permissions: &[ExtraordinaryPermission],
    ) -> Result<(ActionAssessment, ModelStageReceipt)> {
        self.assess_with_context(campaign, intent, extraordinary_permissions, None, &[])
            .await
    }

    pub async fn assess_with_context(
        &self,
        campaign: &Campaign,
        intent: ActionIntent,
        extraordinary_permissions: &[ExtraordinaryPermission],
        campaign_contract: Option<&CampaignContract>,
        aggregate_boundaries: &[AggregatedBoundary],
    ) -> Result<(ActionAssessment, ModelStageReceipt)> {
        self.assess_with_context_inner(
            None,
            campaign,
            intent,
            extraordinary_permissions,
            campaign_contract,
            aggregate_boundaries,
        )
        .await
    }

    pub async fn assess_with_context_cached(
        &self,
        store: &CampaignStore,
        campaign: &Campaign,
        intent: ActionIntent,
        extraordinary_permissions: &[ExtraordinaryPermission],
        campaign_contract: Option<&CampaignContract>,
        aggregate_boundaries: &[AggregatedBoundary],
    ) -> Result<(ActionAssessment, ModelStageReceipt)> {
        self.assess_with_context_inner(
            Some(store),
            campaign,
            intent,
            extraordinary_permissions,
            campaign_contract,
            aggregate_boundaries,
        )
        .await
    }

    async fn assess_with_context_inner(
        &self,
        store: Option<&CampaignStore>,
        campaign: &Campaign,
        intent: ActionIntent,
        extraordinary_permissions: &[ExtraordinaryPermission],
        campaign_contract: Option<&CampaignContract>,
        aggregate_boundaries: &[AggregatedBoundary],
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
        let mut allowed_references = allowed_references(campaign, actor);
        allowed_references.extend(present_actor_references(campaign, actor));
        allowed_references.extend(
            extraordinary_permissions
                .iter()
                .map(|permission| format!("extraordinary_permission:{}", permission.id)),
        );
        let agency_guidance = action_agency_guidance(
            campaign
                .agency_profiles
                .get(&intent.actor_id)
                .is_some_and(|profile| !profile.simulation_eligible)
                || intent.actor_id == campaign.player_actor_id,
        );
        let mut schema = serde_json::to_value(schema_for!(AssessmentProposal))?;
        constrain_assessment_schema(&mut schema, &allowed_references, campaign, actor)?;
        let base_prompt = format!(
            "OUTPUT JSON SCHEMA (follow exactly):\n{}\n\nAssess an attempted effect, not whether words can be spoken. Impossible actions are inadmissible and receive bargains, not a roll. Choose DC only from 5,10,15,20,25,30. Every modifier reference must be copied exactly from ALLOWED REFERENCES. Modifier total is capped at +/-10. Never grant capability, custody, access, knowledge, or spatial reach absent from state. Accepted extraordinary permissions are binding: preserve their prerequisites, costs, limits, exposure, and effect ceiling exactly; they admit only effects within that scope. The campaign contract governs tone, pacing, focus, consequence style, and DM style. Obey every aggregate content boundary: line excludes the topic, veil keeps it off-screen, ask_first admits no new depiction without a current explicit acceptance. Never reveal attribution. State concrete success, mixed, and failure consequences and a bounded effect ceiling. Outcome deltas may use only the mutation lanes present in the supplied schema; omit an unavailable or unused lane. A missing mutation map means no mutation in that lane. Supplied lanes may only name actor IDs copied exactly from PRESENT ACTORS, change their conditions or relationships, move only the acting actor along an existing route, advance or reduce existing clocks by a positive amount, or change existing institution posture. Use clock_advances when an outcome moves a pressure toward its consequence. Use clock_reductions when repair, relief, delay, or obstruction removes established progress. Never name the same clock in both maps for one outcome. Informational outcomes may reveal only an exact statement copied from AVAILABLE INFORMATION FACTS; they never create a new fact. Choose the fact that most directly answers the intended effect, preferring a relevant branch_local or provisional_local fact over generic canon background. A location-discoverable fact may be added only to the acting actor. A fact already known by the acting actor may instead be communicated to another present actor. actor_knowledge_additions contains the player-readable statement, never a fact ID, key, slug, or label. Strong and ordinary success share one visible stake, so give them identical knowledge additions. The runtime binds each exact finding into the player-visible stake; do not spend prose repeating it solely for formatting. If no supplied fact supports the intended discovery or disclosure, omit the knowledge lane and make the limitation explicit in the stakes or mark the attempt inadmissible. Never invent remote events, hidden actors, unsupported proper nouns, or conclusions beyond the effect ceiling. Keep an effect empty only when the outcome truly has no canonical state change.\nCAMPAIGN CONTRACT:\n{}\nAGGREGATE CONTENT BOUNDARIES:\n{}\nAGENCY BOUNDARY:\n{}\nLEGACY HOST ACTOR ID (not an authority):\n{}\nINTENT:\n{}\nACTOR:\n{}\nACCEPTED EXTRAORDINARY PERMISSIONS:\n{}\nLOCATION:\n{}\nPRESENT ACTORS:\n{}\nVISIBLE INSTITUTIONS:\n{}\nAVAILABLE INFORMATION FACTS:\n{}\nALLOWED REFERENCES:\n{}",
            serde_json::to_string(&schema)?,
            serde_json::to_string(&campaign_contract)?,
            serde_json::to_string(aggregate_boundaries)?,
            agency_guidance,
            campaign.player_actor_id,
            serde_json::to_string(&intent)?,
            serde_json::to_string(actor)?,
            serde_json::to_string(extraordinary_permissions)?,
            serde_json::to_string(location)?,
            serde_json::to_string(&present_actors)?,
            serde_json::to_string(&visible_institutions)?,
            serde_json::to_string(&information_facts)?,
            serde_json::to_string(&allowed_references)?
        );
        let snapshot_binding = format!("campaign:{}:revision:{}", campaign.id, campaign.revision);
        let basis_digest = assessment_basis_digest(&self.model_id, &base_prompt);
        if let Some(store) = store
            && let Some((_, cached)) = store.load::<AssessmentProposalCacheEntry>(
                ASSESSMENT_PROPOSAL_CACHE_KIND,
                &basis_digest,
            )?
        {
            if cached.schema != ASSESSMENT_PROPOSAL_CACHE_SCHEMA
                || cached.basis_digest != basis_digest
            {
                return Err(anyhow!("assessment proposal cache identity mismatch"));
            }
            let proposal = validate_and_bind_proposal(
                cached.proposal.clone(),
                campaign,
                actor,
                &allowed_references,
            )?;
            let assessment = build_assessment(campaign, intent, proposal.clone())?;
            let receipt = cache_hit_receipt(
                &cached,
                &proposal,
                &snapshot_binding,
                &base_prompt,
                &campaign.branch_origin.evidence_receipt_ids,
            )?;
            return Ok((assessment, receipt));
        }
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
                    max_output_tokens: Some(2_800),
                },
            )
            .await?;
            let candidate = (|| -> Result<AssessmentProposal> {
                let proposal: AssessmentProposal = serde_json::from_value(
                    out.structured
                        .clone()
                        .ok_or_else(|| anyhow!("assessor returned no typed proposal"))?,
                )?;
                validate_and_bind_proposal(proposal, campaign, actor, &allowed_references)
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
        let mut selected_proposal = proposal;
        let mut selected_receipt = out.receipt;
        if let Some(store) = store {
            let entry = AssessmentProposalCacheEntry {
                schema: ASSESSMENT_PROPOSAL_CACHE_SCHEMA.into(),
                basis_digest: basis_digest.clone(),
                proposal: selected_proposal.clone(),
                source_provider: selected_receipt.provider.clone(),
                source_model: selected_receipt.model.clone(),
                source_receipt_hash: selected_receipt.receipt_hash.clone(),
                created_at: Utc::now(),
            };
            if let Err(insert_error) = store.insert(
                ASSESSMENT_PROPOSAL_CACHE_KIND,
                ASSESSMENT_PROPOSAL_CACHE_SCHEMA,
                &basis_digest,
                &entry,
            ) {
                let (_, winner) = store
                    .load::<AssessmentProposalCacheEntry>(
                        ASSESSMENT_PROPOSAL_CACHE_KIND,
                        &basis_digest,
                    )?
                    .ok_or_else(|| {
                        anyhow!(
                            "assessment proposal cache insert failed without an existing winner: {insert_error}"
                        )
                    })?;
                if winner.schema != ASSESSMENT_PROPOSAL_CACHE_SCHEMA
                    || winner.basis_digest != basis_digest
                {
                    return Err(anyhow!(
                        "assessment proposal cache winner identity mismatch"
                    ));
                }
                selected_proposal = validate_and_bind_proposal(
                    winner.proposal.clone(),
                    campaign,
                    actor,
                    &allowed_references,
                )?;
                selected_receipt = cache_hit_receipt(
                    &winner,
                    &selected_proposal,
                    &snapshot_binding,
                    &base_prompt,
                    &campaign.branch_origin.evidence_receipt_ids,
                )?;
            }
        }
        let assessment = build_assessment(campaign, intent, selected_proposal)?;
        Ok((assessment, selected_receipt))
    }
}

fn validate_and_bind_proposal(
    mut proposal: AssessmentProposal,
    campaign: &Campaign,
    actor: &crate::domain::ActorState,
    allowed_references: &BTreeSet<String>,
) -> Result<AssessmentProposal> {
    bind_visible_knowledge(&mut proposal)?;
    validate_proposal(&proposal, allowed_references)?;
    for (effect, stake) in [
        (&proposal.strong_effect, &proposal.success_stake),
        (&proposal.success_effect, &proposal.success_stake),
        (&proposal.mixed_effect, &proposal.mixed_stake),
        (&proposal.failure_effect, &proposal.failure_stake),
    ] {
        validate_effect(campaign, actor, effect, stake)?;
    }
    Ok(proposal)
}

fn build_assessment(
    campaign: &Campaign,
    intent: ActionIntent,
    proposal: AssessmentProposal,
) -> Result<ActionAssessment> {
    let modifier_total =
        crate::d20::capped_modifier(proposal.modifiers.iter().map(|modifier| modifier.value));
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
    Ok(assessment)
}

fn assessment_basis_digest(model_id: &str, base_prompt: &str) -> String {
    format!(
        "sha256:{:x}",
        Sha256::digest(
            format!("{ASSESSMENT_SEMANTICS_VERSION}|{model_id}|{base_prompt}").as_bytes()
        )
    )
}

fn cache_hit_receipt(
    cached: &AssessmentProposalCacheEntry,
    proposal: &AssessmentProposal,
    snapshot_binding: &str,
    base_prompt: &str,
    source_receipt_ids: &[String],
) -> Result<ModelStageReceipt> {
    let output = serde_json::to_string(proposal)?;
    let output_hash = format!("sha256:{:x}", Sha256::digest(output.as_bytes()));
    let request_hash = format!(
        "sha256:{:x}",
        Sha256::digest(
            format!(
                "{}|{}|{}|{}",
                cached.basis_digest,
                snapshot_binding,
                cached.source_model,
                source_receipt_ids.join("|")
            )
            .as_bytes()
        )
    );
    let receipt_hash = format!(
        "sha256:{:x}",
        Sha256::digest(
            format!(
                "{}|{}|action_assessment|{}|{}|{}|{}",
                cached.source_provider,
                cached.source_model,
                snapshot_binding,
                request_hash,
                output_hash,
                cached.source_receipt_hash
            )
            .as_bytes()
        )
    );
    Ok(ModelStageReceipt {
        schema: "ghostlight.persona_stage_receipt.v1".into(),
        receipt_hash,
        provider: cached.source_provider.clone(),
        model: cached.source_model.clone(),
        stage: "action_assessment".into(),
        snapshot_binding: snapshot_binding.into(),
        request_hash,
        output_hash,
        source_receipt_ids: source_receipt_ids.to_vec(),
        latency_ms: 0,
        validation_result: "valid_cache_hit".into(),
        local_validation_error: None,
        input_chars: base_prompt.chars().count(),
        output_chars: output.chars().count(),
        provider_attempts: vec![ModelProviderAttemptReceipt {
            provider_request_id: None,
            system_fingerprint: None,
            finish_reason: Some("cache_hit".into()),
            latency_ms: 0,
            token_usage: Some(ModelTokenUsage::default()),
            transport_features: vec![
                "cultcache.output-cache".into(),
                format!("source-receipt:{}", cached.source_receipt_hash),
            ],
            local_validation_result: "valid_cache_hit".into(),
            local_validation_error: None,
        }],
    })
}

fn constrain_assessment_schema(
    schema: &mut serde_json::Value,
    allowed_references: &BTreeSet<String>,
    campaign: &Campaign,
    acting_actor: &crate::domain::ActorState,
) -> Result<()> {
    let properties = schema
        .pointer_mut("/properties")
        .and_then(serde_json::Value::as_object_mut)
        .ok_or_else(|| anyhow!("assessment schema has no properties"))?;
    properties.insert(
        "dc".into(),
        serde_json::json!({
            "type":"integer",
            "enum":[5,10,15,20,25,30]
        }),
    );
    let modifier_properties = schema
        .pointer_mut("/$defs/ContextModifier/properties")
        .and_then(serde_json::Value::as_object_mut)
        .ok_or_else(|| anyhow!("assessment schema has no context modifier properties"))?;
    modifier_properties.insert(
        "value".into(),
        serde_json::json!({
            "type":"integer",
            "minimum":-10,
            "maximum":10
        }),
    );
    modifier_properties.insert(
        "references".into(),
        serde_json::json!({
            "type":"array",
            "items":{
                "type":"string",
                "enum":allowed_references.iter().collect::<Vec<_>>()
            }
        }),
    );
    constrain_effect_schema(schema, campaign, acting_actor)?;
    Ok(())
}

fn constrain_effect_schema(
    schema: &mut serde_json::Value,
    campaign: &Campaign,
    acting_actor: &crate::domain::ActorState,
) -> Result<()> {
    schema["$defs"]["WorldEffectDelta"]["additionalProperties"] = serde_json::json!(false);
    let present_actor_ids = campaign
        .actors
        .values()
        .filter(|candidate| candidate.location_id == acting_actor.location_id)
        .map(|candidate| candidate.id.clone())
        .collect::<BTreeSet<_>>();
    let institution_ids = campaign
        .institutions
        .keys()
        .cloned()
        .collect::<BTreeSet<_>>();
    let relationship_target_ids = present_actor_ids
        .iter()
        .chain(institution_ids.iter())
        .cloned()
        .collect::<BTreeSet<_>>();
    let destination_ids = campaign
        .locations
        .get(&acting_actor.location_id)
        .ok_or_else(|| anyhow!("acting actor location vanished while binding effect schema"))?
        .routes
        .keys()
        .filter(|destination| campaign.locations.contains_key(*destination))
        .cloned()
        .collect::<BTreeSet<_>>();
    let clock_ids = campaign.clocks.keys().cloned().collect::<BTreeSet<_>>();
    for field in ["add", "remove"] {
        schema["$defs"]["ConditionDelta"]["properties"][field]["items"] =
            serde_json::json!({"type":"string","minLength":1});
    }
    let effect_properties = schema
        .pointer_mut("/$defs/WorldEffectDelta/properties")
        .and_then(serde_json::Value::as_object_mut)
        .ok_or_else(|| anyhow!("assessment schema has no world effect properties"))?;

    for field in ["actor_conditions", "actor_relationship_updates"] {
        constrain_map_keys(
            effect_properties
                .get_mut(field)
                .ok_or_else(|| anyhow!("assessment effect schema omitted {field}"))?,
            &present_actor_ids,
        )?;
    }
    let knowledge_available = constrain_knowledge_map(
        effect_properties
            .get_mut("actor_knowledge_additions")
            .ok_or_else(|| anyhow!("assessment effect schema omitted actor_knowledge_additions"))?,
        campaign,
        acting_actor,
        &present_actor_ids,
    )?;
    let relationship_targets = effect_properties
        .get_mut("actor_relationship_updates")
        .and_then(|value| value.get_mut("additionalProperties"))
        .ok_or_else(|| anyhow!("assessment relationship schema has no target map"))?;
    constrain_map_keys(relationship_targets, &relationship_target_ids)?;
    relationship_targets["additionalProperties"] =
        serde_json::json!({"type":"string","minLength":1});

    let actor_moves = effect_properties
        .get_mut("actor_moves")
        .ok_or_else(|| anyhow!("assessment effect schema omitted actor_moves"))?;
    constrain_map_keys(actor_moves, &BTreeSet::from([acting_actor.id.clone()]))?;
    if destination_ids.is_empty() {
        actor_moves["maxProperties"] = serde_json::json!(0);
    } else {
        actor_moves["additionalProperties"] = serde_json::json!({
            "type":"string",
            "enum":destination_ids
        });
    }

    let clock_advances = effect_properties
        .get_mut("clock_advances")
        .ok_or_else(|| anyhow!("assessment effect schema omitted clock_advances"))?;
    constrain_map_keys(clock_advances, &clock_ids)?;
    clock_advances["additionalProperties"] =
        serde_json::json!({"type":"integer","minimum":1,"maximum":255});
    let clock_reductions = effect_properties
        .get_mut("clock_reductions")
        .ok_or_else(|| anyhow!("assessment effect schema omitted clock_reductions"))?;
    constrain_map_keys(clock_reductions, &clock_ids)?;
    clock_reductions["additionalProperties"] =
        serde_json::json!({"type":"integer","minimum":1,"maximum":255});
    let institution_postures = effect_properties
        .get_mut("institution_postures")
        .ok_or_else(|| anyhow!("assessment effect schema omitted institution_postures"))?;
    constrain_map_keys(institution_postures, &institution_ids)?;
    institution_postures["additionalProperties"] =
        serde_json::json!({"type":"string","minLength":1});
    let mut unavailable_lanes = Vec::new();
    if !knowledge_available {
        unavailable_lanes.push("actor_knowledge_additions");
    }
    if destination_ids.is_empty() {
        unavailable_lanes.push("actor_moves");
    }
    if clock_ids.is_empty() {
        unavailable_lanes.extend(["clock_advances", "clock_reductions"]);
    }
    if institution_ids.is_empty() {
        unavailable_lanes.push("institution_postures");
    }
    for field in unavailable_lanes {
        remove_effect_lane(schema, field)?;
    }
    Ok(())
}

fn constrain_knowledge_map(
    schema: &mut serde_json::Value,
    campaign: &Campaign,
    acting_actor: &crate::domain::ActorState,
    present_actor_ids: &BTreeSet<String>,
) -> Result<bool> {
    let mut properties = serde_json::Map::new();
    for actor_id in present_actor_ids {
        let target = campaign
            .actors
            .get(actor_id)
            .ok_or_else(|| anyhow!("present actor vanished while binding knowledge schema"))?;
        let allowed = campaign
            .facts
            .values()
            .filter(|fact| {
                let accessible = if actor_id == &acting_actor.id {
                    fact.discoverable_at_location_ids
                        .contains(&acting_actor.location_id)
                } else {
                    acting_actor.knowledge.contains(&fact.statement)
                };
                accessible && !target.knowledge.contains(&fact.statement)
            })
            .map(|fact| fact.statement.clone())
            .collect::<BTreeSet<_>>();
        if allowed.is_empty() {
            continue;
        }
        let max_items = usize::min(4, allowed.len());
        properties.insert(
            actor_id.clone(),
            serde_json::json!({
                "type":"array",
                "items":{"type":"string","enum":allowed},
                "uniqueItems":true,
                "minItems":1,
                "maxItems":max_items
            }),
        );
    }
    let max_properties = properties.len();
    *schema = serde_json::json!({
        "type":"object",
        "properties":properties,
        "additionalProperties":false,
        "maxProperties":max_properties
    });
    Ok(max_properties > 0)
}

fn remove_effect_lane(schema: &mut serde_json::Value, field: &str) -> Result<()> {
    let definition = schema
        .pointer_mut("/$defs/WorldEffectDelta")
        .and_then(serde_json::Value::as_object_mut)
        .ok_or_else(|| anyhow!("assessment schema has no world effect definition"))?;
    definition
        .get_mut("properties")
        .and_then(serde_json::Value::as_object_mut)
        .ok_or_else(|| anyhow!("assessment schema has no world effect properties"))?
        .remove(field);
    if let Some(required) = definition
        .get_mut("required")
        .and_then(serde_json::Value::as_array_mut)
    {
        required.retain(|value| value.as_str() != Some(field));
    }
    Ok(())
}

fn constrain_map_keys(schema: &mut serde_json::Value, allowed: &BTreeSet<String>) -> Result<()> {
    let object = schema
        .as_object_mut()
        .ok_or_else(|| anyhow!("assessment effect map schema is not an object"))?;
    if allowed.is_empty() {
        object.insert("maxProperties".into(), serde_json::json!(0));
        object.remove("propertyNames");
    } else {
        object.insert("propertyNames".into(), serde_json::json!({"enum":allowed}));
    }
    Ok(())
}

fn action_agency_guidance(human_controlled: bool) -> &'static str {
    if human_controlled {
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

fn present_actor_references(
    campaign: &Campaign,
    actor: &crate::domain::ActorState,
) -> BTreeSet<String> {
    campaign
        .actors
        .values()
        .filter(|candidate| candidate.location_id == actor.location_id)
        .map(|candidate| format!("actor:{}", candidate.id))
        .collect()
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
                "scope": fact.scope,
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
            (!campaign
                .actors
                .get(id)
                .is_some_and(|target| target.location_id == acting_actor.location_id)
                && !campaign.institutions.contains_key(id))
                || value.trim().is_empty()
        }) {
            return Err(anyhow!(
                "outcome relationship delta cited an unavailable target"
            ));
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
    if let Some((id, amount)) = effect
        .clock_advances
        .iter()
        .find(|(id, amount)| **amount == 0 || !campaign.clocks.contains_key(*id))
    {
        return Err(anyhow!(
            "outcome clock advance must name an existing clock and advance it by at least one: {id}={amount}"
        ));
    }
    if let Some((id, amount)) = effect
        .clock_reductions
        .iter()
        .find(|(id, amount)| **amount == 0 || !campaign.clocks.contains_key(*id))
    {
        return Err(anyhow!(
            "outcome clock reduction must name an existing clock and reduce it by at least one: {id}={amount}"
        ));
    }
    if let Some(id) = effect
        .clock_advances
        .keys()
        .find(|id| effect.clock_reductions.contains_key(*id))
    {
        return Err(anyhow!(
            "one outcome cannot both advance and reduce the same clock: {id}"
        ));
    }
    if let Some((id, posture)) = effect
        .institution_postures
        .iter()
        .find(|(id, posture)| !campaign.institutions.contains_key(*id) || posture.trim().is_empty())
    {
        return Err(anyhow!(
            "outcome institution posture must name an existing institution and a non-empty posture: {id}={posture:?}"
        ));
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
    use async_trait::async_trait;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct DriftingAssessmentModel {
        calls: AtomicUsize,
    }

    #[async_trait]
    impl ModelPort for DriftingAssessmentModel {
        async fn run(&self, request: &ModelStageRequest) -> Result<String> {
            let call = self.calls.fetch_add(1, Ordering::SeqCst);
            let mut value = serde_json::to_value(proposal("actor:player"))?;
            value["modifiers"][0]["value"] = serde_json::json!(if call == 0 { 2 } else { 6 });
            let allowed_effect_fields = request
                .output_schema
                .as_ref()
                .and_then(|schema| schema.pointer("/$defs/WorldEffectDelta/properties"))
                .and_then(serde_json::Value::as_object)
                .map(|properties| properties.keys().cloned().collect::<BTreeSet<_>>())
                .unwrap_or_default();
            for field in [
                "strong_effect",
                "success_effect",
                "mixed_effect",
                "failure_effect",
            ] {
                value[field]
                    .as_object_mut()
                    .expect("fixture effect is an object")
                    .retain(|key, _| allowed_effect_fields.contains(key));
            }
            Ok(serde_json::to_string(&value)?)
        }

        fn provider(&self) -> &'static str {
            "drifting-fixture"
        }
    }

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

    #[tokio::test]
    async fn unchanged_semantic_packet_reuses_exact_validated_assessment_proposal() {
        let model = Arc::new(DriftingAssessmentModel {
            calls: AtomicUsize::new(0),
        });
        let assessor = ActionAssessor::new(model.clone(), "fixture-model");
        let dir = tempfile::tempdir().unwrap();
        let store = CampaignStore::open(dir.path().join("campaign.cc")).unwrap();
        let mut campaign = crate::resolution::tests::campaign(0, 1);
        let intent = ActionIntent {
            actor_id: "player".into(),
            description: "inspect the gate".into(),
            intended_effect: "learn whether it is open".into(),
        };

        let (first, first_receipt) = assessor
            .assess_with_context_cached(&store, &campaign, intent.clone(), &[], None, &[])
            .await
            .unwrap();
        campaign.revision += 1;
        let (second, second_receipt) = assessor
            .assess_with_context_cached(&store, &campaign, intent, &[], None, &[])
            .await
            .unwrap();

        assert_eq!(model.calls.load(Ordering::SeqCst), 1);
        assert_eq!(first.modifiers, second.modifiers);
        assert_eq!(first.modifier_total, 2);
        assert_eq!(second.modifier_total, 2);
        assert_eq!(first.revision, 0);
        assert_eq!(second.revision, 1);
        assert_ne!(first.digest, second.digest);
        assert_eq!(first_receipt.validation_result, "valid");
        assert_eq!(second_receipt.validation_result, "valid_cache_hit");
        assert!(
            second_receipt.provider_attempts[0]
                .transport_features
                .contains(&"cultcache.output-cache".to_string())
        );
        assert_eq!(store.keys(ASSESSMENT_PROPOSAL_CACHE_KIND).unwrap().len(), 1);
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
    fn assessment_reference_schema_enumerates_exact_allowed_values() {
        let campaign = crate::resolution::tests::campaign(0, 1);
        let acting = &campaign.actors["player"];
        let allowed = BTreeSet::from([
            "actor:player".into(),
            "actor:clinic-director".into(),
            "equipment:audit-seal".into(),
        ]);
        let mut schema = serde_json::to_value(schema_for!(AssessmentProposal)).unwrap();
        constrain_assessment_schema(&mut schema, &allowed, &campaign, acting).unwrap();
        let values = schema
            .pointer("/$defs/ContextModifier/properties/references/items/enum")
            .and_then(serde_json::Value::as_array)
            .unwrap()
            .iter()
            .filter_map(serde_json::Value::as_str)
            .collect::<BTreeSet<_>>();
        assert_eq!(
            values,
            BTreeSet::from([
                "actor:player",
                "actor:clinic-director",
                "equipment:audit-seal"
            ])
        );
    }

    #[test]
    fn assessment_effect_schema_binds_exact_visible_and_spatial_scope() {
        let mut campaign = crate::resolution::tests::campaign(0, 1);
        let acting = campaign.actors["player"].clone();
        campaign.clocks.insert(
            "clinic-failure".into(),
            crate::domain::WorldClock {
                id: "clinic-failure".into(),
                label: "Clinic failure".into(),
                progress: 1,
                threshold: 4,
                consequence: "The clinic fails.".into(),
            },
        );
        campaign.institutions.insert(
            "clinic".into(),
            crate::domain::InstitutionState {
                id: "clinic".into(),
                name: "Clinic".into(),
                resources: vec![],
                goals: vec!["remain operational".into()],
                posture: "strained".into(),
            },
        );
        let mut nearby = acting.clone();
        nearby.id = "clinic-director".into();
        nearby.name = "Clinic Director".into();
        campaign.actors.insert(nearby.id.clone(), nearby);
        campaign.locations.insert(
            "adjacent".into(),
            crate::domain::Location {
                id: "adjacent".into(),
                name: "Adjacent".into(),
                container_id: None,
                routes: Default::default(),
                persistent_features: vec![],
            },
        );
        campaign
            .locations
            .get_mut(&acting.location_id)
            .unwrap()
            .routes
            .insert(
                "adjacent".into(),
                crate::domain::Route {
                    destination_id: "adjacent".into(),
                    distance: "5 km".into(),
                    travel_minutes: 20,
                },
            );
        campaign.locations.insert(
            "remote".into(),
            crate::domain::Location {
                id: "remote".into(),
                name: "Remote".into(),
                container_id: None,
                routes: Default::default(),
                persistent_features: vec![],
            },
        );
        let mut remote = acting.clone();
        remote.id = "remote-commander".into();
        remote.name = "Remote Commander".into();
        remote.location_id = "remote".into();
        campaign.actors.insert(remote.id.clone(), remote);

        let mut schema = serde_json::to_value(schema_for!(AssessmentProposal)).unwrap();
        constrain_assessment_schema(&mut schema, &BTreeSet::new(), &campaign, &acting).unwrap();
        let effect = schema
            .pointer("/$defs/WorldEffectDelta/properties")
            .unwrap();
        let actor_targets = effect["actor_conditions"]["propertyNames"]["enum"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(serde_json::Value::as_str)
            .collect::<BTreeSet<_>>();
        assert_eq!(actor_targets, BTreeSet::from(["clinic-director", "player"]));
        let relationship_targets =
            effect["actor_relationship_updates"]["additionalProperties"]["propertyNames"]["enum"]
                .as_array()
                .unwrap()
                .iter()
                .filter_map(serde_json::Value::as_str)
                .collect::<BTreeSet<_>>();
        assert!(relationship_targets.contains("clinic-director"));
        assert!(relationship_targets.contains("player"));
        assert!(!relationship_targets.contains("remote-commander"));
        assert_eq!(
            effect["actor_moves"]["propertyNames"]["enum"],
            serde_json::json!(["player"])
        );
        assert_eq!(
            effect["actor_moves"]["additionalProperties"]["enum"],
            serde_json::json!(["adjacent"])
        );
        assert_eq!(
            effect["clock_advances"]["additionalProperties"]["minimum"],
            1
        );
        assert_eq!(
            effect["clock_reductions"]["additionalProperties"]["minimum"],
            1
        );
        assert_eq!(
            effect["institution_postures"]["additionalProperties"]["minLength"],
            1
        );
        assert_eq!(
            effect["actor_relationship_updates"]["additionalProperties"]["additionalProperties"]["minLength"],
            1
        );
        assert_eq!(
            schema["$defs"]["ConditionDelta"]["properties"]["add"]["items"]["minLength"],
            1
        );
    }

    #[test]
    fn assessment_effect_schema_omits_unavailable_mutation_lanes() {
        let mut campaign = crate::resolution::tests::campaign(0, 1);
        campaign.clocks.clear();
        campaign.institutions.clear();
        campaign.facts.clear();
        let actor_id = campaign.player_actor_id.clone();
        let location_id = campaign.actors[&actor_id].location_id.clone();
        campaign
            .locations
            .get_mut(&location_id)
            .unwrap()
            .routes
            .clear();
        campaign
            .actors
            .get_mut(&actor_id)
            .unwrap()
            .knowledge
            .clear();
        let acting = campaign.actors[&actor_id].clone();

        let mut schema = serde_json::to_value(schema_for!(AssessmentProposal)).unwrap();
        constrain_assessment_schema(&mut schema, &BTreeSet::new(), &campaign, &acting).unwrap();
        let effect = schema.pointer("/$defs/WorldEffectDelta").unwrap();
        let properties = effect["properties"].as_object().unwrap();

        assert!(properties.contains_key("actor_conditions"));
        assert!(properties.contains_key("actor_relationship_updates"));
        for unavailable in [
            "actor_knowledge_additions",
            "actor_moves",
            "clock_advances",
            "clock_reductions",
            "institution_postures",
        ] {
            assert!(!properties.contains_key(unavailable));
            assert!(
                effect["required"]
                    .as_array()
                    .is_none_or(|required| required.iter().all(|value| value != unavailable))
            );
        }
        let effect_contract = serde_json::json!({
            "$ref":"#/$defs/WorldEffectDelta",
            "$defs":schema["$defs"].clone(),
        });
        let validator = jsonschema::validator_for(&effect_contract).unwrap();
        assert!(validator.is_valid(&serde_json::json!({})));
        assert!(!validator.is_valid(&serde_json::json!({
            "actor_knowledge_additions":null
        })));
        let decoded: WorldEffectDelta = serde_json::from_value(serde_json::json!({})).unwrap();
        assert_eq!(decoded, WorldEffectDelta::default());
    }

    #[test]
    fn assessment_knowledge_schema_binds_exact_facts_per_recipient() {
        let mut campaign = crate::resolution::tests::campaign(0, 1);
        campaign
            .actors
            .get_mut("player")
            .unwrap()
            .knowledge
            .insert("The clinic director already knows the convoy is delayed.".into());
        let acting = campaign.actors["player"].clone();
        let mut nearby = acting.clone();
        nearby.id = "clinic-director".into();
        nearby.name = "Clinic Director".into();
        nearby.knowledge.clear();
        campaign.actors.insert(nearby.id.clone(), nearby);
        campaign.facts.insert(
            "cache".into(),
            crate::domain::WorldFact {
                id: "cache".into(),
                statement: "The emergency cache is behind the north clinic wall.".into(),
                scope: crate::domain::FactScope::BranchLocal,
                evidence_receipt_ids: vec![],
                discoverable_at_location_ids: BTreeSet::from([acting.location_id.clone()]),
            },
        );
        campaign.facts.insert(
            "delay".into(),
            crate::domain::WorldFact {
                id: "delay".into(),
                statement: "The clinic director already knows the convoy is delayed.".into(),
                scope: crate::domain::FactScope::BranchLocal,
                evidence_receipt_ids: vec![],
                discoverable_at_location_ids: BTreeSet::new(),
            },
        );

        let mut schema = serde_json::to_value(schema_for!(AssessmentProposal)).unwrap();
        constrain_assessment_schema(&mut schema, &BTreeSet::new(), &campaign, &acting).unwrap();
        let knowledge = schema
            .pointer("/$defs/WorldEffectDelta/properties/actor_knowledge_additions")
            .unwrap();
        let validator = jsonschema::validator_for(knowledge).unwrap();

        assert!(validator.is_valid(&serde_json::json!({
            "player":["The emergency cache is behind the north clinic wall."]
        })));
        assert!(validator.is_valid(&serde_json::json!({
            "clinic-director":["The clinic director already knows the convoy is delayed."]
        })));
        assert!(!validator.is_valid(&serde_json::json!({
            "clinic-director":["The emergency cache is behind the north clinic wall."]
        })));
        assert!(!validator.is_valid(&serde_json::json!({
            "player":["The clinic director already knows the convoy is delayed."]
        })));
    }

    #[test]
    fn assessment_references_include_present_actors_but_not_remote_actors() {
        let mut campaign = crate::resolution::tests::campaign(0, 1);
        let acting = campaign.actors["player"].clone();
        let mut nearby = acting.clone();
        nearby.id = "clinic-director".into();
        nearby.name = "Clinic Director".into();
        campaign.actors.insert(nearby.id.clone(), nearby);
        campaign.locations.insert(
            "remote".into(),
            crate::domain::Location {
                id: "remote".into(),
                name: "Remote".into(),
                container_id: None,
                routes: Default::default(),
                persistent_features: vec![],
            },
        );
        let mut remote = acting.clone();
        remote.id = "remote-commander".into();
        remote.name = "Remote Commander".into();
        remote.location_id = "remote".into();
        campaign.actors.insert(remote.id.clone(), remote);

        let references = present_actor_references(&campaign, &acting);
        assert!(references.contains("actor:player"));
        assert!(references.contains("actor:clinic-director"));
        assert!(!references.contains("actor:remote-commander"));
    }

    #[test]
    fn assessment_effect_rejects_a_relationship_to_an_unseen_remote_actor() {
        let mut campaign = crate::resolution::tests::campaign(0, 1);
        let acting = campaign.actors["player"].clone();
        campaign.locations.insert(
            "remote".into(),
            crate::domain::Location {
                id: "remote".into(),
                name: "Remote".into(),
                container_id: None,
                routes: Default::default(),
                persistent_features: vec![],
            },
        );
        let mut remote = acting.clone();
        remote.id = "remote-commander".into();
        remote.name = "Remote Commander".into();
        remote.location_id = "remote".into();
        campaign.actors.insert(remote.id.clone(), remote);
        let effect = WorldEffectDelta {
            actor_relationship_updates: std::collections::BTreeMap::from([(
                acting.id.clone(),
                std::collections::BTreeMap::from([(
                    "remote-commander".into(),
                    "unexpected trust".into(),
                )]),
            )]),
            ..WorldEffectDelta::default()
        };

        let error = validate_effect(&campaign, &acting, &effect, "nothing changes")
            .unwrap_err()
            .to_string();
        assert!(error.contains("unavailable target"));
    }

    #[test]
    fn assessment_effect_names_an_invalid_zero_clock_advance() {
        let campaign = crate::resolution::tests::campaign(0, 1);
        let acting = campaign.actors["player"].clone();
        let effect = WorldEffectDelta {
            clock_advances: std::collections::BTreeMap::from([("missing-clock".into(), 0)]),
            ..WorldEffectDelta::default()
        };

        let error = validate_effect(&campaign, &acting, &effect, "nothing changes")
            .unwrap_err()
            .to_string();
        assert!(error.contains("missing-clock=0"));
        assert!(error.contains("at least one"));
    }

    #[test]
    fn assessment_effect_rejects_invalid_or_conflicting_clock_reduction() {
        let mut campaign = crate::resolution::tests::campaign(0, 1);
        campaign.clocks.insert(
            "clinic-failure".into(),
            crate::domain::WorldClock {
                id: "clinic-failure".into(),
                label: "Clinic failure".into(),
                progress: 3,
                threshold: 4,
                consequence: "The regulator fails.".into(),
            },
        );
        let acting = campaign.actors["player"].clone();
        let invalid = WorldEffectDelta {
            clock_reductions: std::collections::BTreeMap::from([("missing-clock".into(), 0)]),
            ..WorldEffectDelta::default()
        };
        let error = validate_effect(&campaign, &acting, &invalid, "nothing changes")
            .unwrap_err()
            .to_string();
        assert!(error.contains("missing-clock=0"));
        assert!(error.contains("at least one"));

        let conflicting = WorldEffectDelta {
            clock_advances: std::collections::BTreeMap::from([("clinic-failure".into(), 1)]),
            clock_reductions: std::collections::BTreeMap::from([("clinic-failure".into(), 1)]),
            ..WorldEffectDelta::default()
        };
        let error = validate_effect(&campaign, &acting, &conflicting, "ambiguous change")
            .unwrap_err()
            .to_string();
        assert!(error.contains("both advance and reduce"));
        assert!(error.contains("clinic-failure"));
    }

    #[test]
    fn assessment_rejects_noncanonical_dc() {
        let mut value = proposal("equipment:key");
        value.dc = 17;
        assert!(validate_proposal(&value, &BTreeSet::from(["equipment:key".into()])).is_err());
    }

    #[test]
    fn npc_assessment_guidance_preserves_player_decision_authority() {
        let guidance = action_agency_guidance(false);
        assert!(guidance.contains("player retains authority"));
        assert!(guidance.contains("response remains the player's next decision"));
        assert!(!action_agency_guidance(true).contains("acting actor is an NPC"));
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
