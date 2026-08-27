use crate::{
    domain::{
        ActionAssessment, ActionIntent, Campaign, ContextModifier, MAX_POSTURE_CHARS,
        WorldEffectDelta,
    },
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
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

const ASSESSMENT_PROPOSAL_CACHE_KIND: &str = "assessment_proposal_cache.v1";
const ASSESSMENT_PROPOSAL_CACHE_SCHEMA: &str = "ghostlight.private.assessment_proposal_cache.v6";
const ASSESSMENT_SCOPE_CACHE_KIND: &str = "assessment_mutation_scope_cache.v1";
const ASSESSMENT_SCOPE_CACHE_SCHEMA: &str = "ghostlight.private.assessment_mutation_scope_cache.v5";
const ASSESSMENT_SEMANTICS_VERSION: &str = "ghostlight.action_assessment.v12";

const ASSESSMENT_SCOPE_INSTRUCTIONS: &str = "You own the compact admission and mutation-scope preflight for one fiction-first attempt. Decide 'deny' only when no d20 outcome can realize the exact intended effect from the supplied authority: the attempt lacks required capability, custody, access, spatial reach, extraordinary permission, an exact typed mutation path, or demands guaranteed control over another subject's independent future choices. A bounded attempt to influence a present subject's voluntary response is not guaranteed control: when the means can plausibly reach that subject, assess it and represent a successful voluntary promise, agreement, plan, goal, or obligation through actor_commitments. Difficulty, opposition, danger, or a costly but bounded effect are not reasons to deny; use 'assess' and let the full assessor set the DC and ceiling. For 'assess', select the smallest causally plausible typed mutation vocabulary. Availability is never relevance. Put every selected lane in lanes. Also put in required_success_lanes every lane whose non-empty mutation is necessary for strong and ordinary success to realize the intended effect; direct costs or incidental consequences are allowed lanes but are not required success lanes. actor_conditions changes bodily or situational conditions. actor_commitments adds or retires an exact present actor's goal or obligation; it records that actor's voluntary commitment and never grants custody of them or guarantees behavior beyond it. actor_knowledge_additions acquires or communicates an exact existing fact to an exact present actor allowed by the supplied lane. A subject receiving, recognizing, learning, remembering, or becoming able to act on information is a canonical knowledge transition even when narration is the only player-visible effect. If the intended recipient is an institution, population, place, remote actor, absent actor, unspecified office, or other subject for which no exact knowledge-recipient lane is structurally available, deny the intended effect and name the missing recipient/channel mutation path. Do not assess with empty information lanes while promising receipt or recognition in the stakes. actor_observations admits a new branch-local finding only when the acting actor's exact means directly perceives, measures, inspects, or tests something within current spatial reach; it cannot invent a remote event, hidden motive, unsupported identity, or conclusion beyond the intended effect and effect ceiling. Do not select either information lane for ordinary speech, promises, persuasion, trust, or scene texture. actor_relationship_updates changes durable trust, regard, leverage, or another exact relationship. actor_moves relocates the acting character along an admitted route. clock_advances and clock_reductions change an existing pressure. institution_postures changes an institution's durable policy or stance; posture is not a substitute for knowledge receipt. For 'deny', both lane sets must be empty and denial must state the exact missing permission or mutation path, the maximum effect declaration alone can have, one concise refusal stake, and one to four actionable bargains that could admit a narrower future assessment. For 'assess', denial must be null. Never infer a new lane, target, route, clock, institution, possession, or permission. New propositions are permitted only through actor_observations. Return only the typed JSON.";

const ASSESSMENT_EFFECT_VERIFIER_INSTRUCTIONS: &str = "You are the private semantic verifier between the fiction-first action assessor and the world kernel. Structural authority, reach, knowledge access, and mutation shape were already checked. Judge the complete four-band typed effect bundle against the player's exact means and intended effect. Every non-empty mutation must be a direct realization of the intended effect or a concrete, previewed consequence of the attempted means in that exact outcome band. Every visible stake must also be backed by the typed mutations that would make its canonical claims true. A subject receiving, recognizing, learning, remembering, or becoming able to act on information is a knowledge transition. If a stake claims such a transition without an exact typed knowledge mutation for that exact recipient, return mismatch_kind 'effect_omission'; ordinary speech, narration, an institution posture change, or a sender's observation is not a substitute. Acquiring or communicating an existing fact requires a causally related information action. Admitting a new actor_observations finding requires exact local means that directly perceive, measure, inspect, or test it, and the statement must remain within the intended effect and effect ceiling. An actor_observations entry becomes canonical knowledge if its outcome is committed. It must therefore be a concrete, truth-apt proposition about what the inspection actually finds. Reject an entry that merely says the actor determines, establishes, learns, or discovers whether something is true; repeats the intended question; preserves an 'if one exists' placeholder; or otherwise reports completion of an inquiry without resolving it. A bounded concrete negative finding is valid. When supplied canon does not determine harmless local texture, the assessment may preview the smallest reversible branch-local detail consistent with the location and evidence; it must state that detail rather than leave a template for a later resolver. A plausible general reaction does not justify changing a commitment, relationship, condition, clock, posture, movement, or knowledge record that the attempted means and stakes do not cause. A target actor's new commitment must be a plausible voluntary response to the attempted influence, never disguised custody or guaranteed obedience. Failure and mixed effects may impose direct costs or complications, but not arbitrary available state changes. The effect ceiling and visible stakes must describe the same bounded consequences as the typed effects. Do not reassess admissibility, DC, or modifiers, and do not choose replacement effects. Return one JSON object. If every typed mutation is causally faithful and epistemically complete, use result 'match' with null mismatch_kind and null repair_guidance. Otherwise use result 'mismatch', one mismatch_kind, and one concrete repair sentence of at most 240 characters naming what must be removed or aligned. Shape: {\"result\":\"match\",\"mismatch_kind\":null,\"repair_guidance\":null}.";

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

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum ConditionMutationOperation {
    Add,
    Remove,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct ActorConditionMutation {
    actor_id: String,
    operation: ConditionMutationOperation,
    condition: String,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum ActorCommitmentMutationOperation {
    AddGoal,
    RetireGoal,
    AddObligation,
    RetireObligation,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct ActorCommitmentMutation {
    actor_id: String,
    operation: ActorCommitmentMutationOperation,
    description: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct ActorKnowledgeMutation {
    actor_id: String,
    statement: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct ActorObservationMutation {
    actor_id: String,
    statement: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct ActorRelationshipMutation {
    actor_id: String,
    target_id: String,
    relationship: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct ActorMoveMutation {
    actor_id: String,
    destination_id: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct ClockMutation {
    clock_id: String,
    amount: u8,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct InstitutionPostureMutation {
    institution_id: String,
    posture: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
struct AssessmentProposalCacheEntry {
    schema: String,
    basis_digest: String,
    proposal: AssessmentProposal,
    source_provider: String,
    source_model: String,
    source_scope_receipt_hash: String,
    source_receipt_hash: String,
    #[serde(default)]
    source_effect_verifier_receipt_hash: Option<String>,
    created_at: chrono::DateTime<Utc>,
}

#[derive(
    Clone, Copy, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq, PartialOrd, Ord,
)]
#[serde(rename_all = "snake_case")]
enum AssessmentMutationLane {
    ActorConditions,
    ActorCommitments,
    ActorKnowledgeAdditions,
    ActorObservations,
    ActorRelationshipUpdates,
    ActorMoves,
    ClockAdvances,
    ClockReductions,
    InstitutionPostures,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum AssessmentScopeDecision {
    Assess,
    Deny,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
struct AssessmentDenial {
    normalized_intent: String,
    missing_permission: String,
    effect_ceiling: String,
    refusal_stake: String,
    bargains: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
struct AssessmentMutationScope {
    decision: AssessmentScopeDecision,
    lanes: BTreeSet<AssessmentMutationLane>,
    required_success_lanes: BTreeSet<AssessmentMutationLane>,
    denial: Option<AssessmentDenial>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
struct AssessmentMutationScopeCacheEntry {
    schema: String,
    basis_digest: String,
    scope: AssessmentMutationScope,
    source_provider: String,
    source_model: String,
    source_receipt_hash: String,
    created_at: chrono::DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum AssessmentEffectMatchResult {
    Match,
    Mismatch,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum AssessmentEffectMismatchKind {
    UnrelatedMutation,
    EffectOmission,
    EffectReversal,
    TargetSubstitution,
    InventedOutcome,
    WrongEffectKind,
    StakeMutationMismatch,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
struct AssessmentEffectVerification {
    result: AssessmentEffectMatchResult,
    mismatch_kind: Option<AssessmentEffectMismatchKind>,
    repair_guidance: Option<String>,
}

pub struct ActionAssessor {
    model: Arc<dyn ModelPort>,
    verifier_model_id: String,
    model_id: String,
}
impl ActionAssessor {
    pub fn new(model: Arc<dyn ModelPort>, model_id: impl Into<String>) -> Self {
        let model_id = model_id.into();
        Self {
            model,
            verifier_model_id: model_id.clone(),
            model_id,
        }
    }

    pub fn with_models(
        model: Arc<dyn ModelPort>,
        verifier_model_id: impl Into<String>,
        model_id: impl Into<String>,
    ) -> Self {
        Self {
            model,
            verifier_model_id: verifier_model_id.into(),
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
        let (mutation_scope, scope_receipt) = self
            .select_mutation_scope(
                store,
                campaign,
                &intent,
                actor,
                extraordinary_permissions,
                campaign_contract,
                aggregate_boundaries,
            )
            .await?;
        let information_facts = if mutation_scope
            .lanes
            .contains(&AssessmentMutationLane::ActorKnowledgeAdditions)
        {
            available_information_facts(campaign, actor)
        } else {
            Vec::new()
        };
        let mut allowed_references = allowed_references(campaign, actor);
        allowed_references.extend(present_actor_references(campaign, actor));
        allowed_references.extend(
            extraordinary_permissions
                .iter()
                .map(|permission| format!("extraordinary_permission:{}", permission.id)),
        );
        if matches!(mutation_scope.decision, AssessmentScopeDecision::Deny) {
            let proposal = denied_assessment_proposal(&mutation_scope)?;
            let proposal = validate_and_bind_proposal(
                proposal,
                campaign,
                actor,
                &allowed_references,
                &mutation_scope,
            )?;
            return Ok((build_assessment(campaign, intent, proposal)?, scope_receipt));
        }
        let source_scope_receipt_hash = scope_receipt.receipt_hash.clone();
        let agency_guidance = action_agency_guidance(
            campaign
                .agency_profiles
                .get(&intent.actor_id)
                .is_some_and(|profile| !profile.simulation_eligible)
                || intent.actor_id == campaign.player_actor_id,
        );
        let mut schema = serde_json::to_value(schema_for!(AssessmentProposal))?;
        constrain_assessment_schema(&mut schema, &allowed_references, campaign, actor)?;
        project_effect_schema_to_mutation_entries(&mut schema, campaign)?;
        constrain_effect_schema_to_scope(&mut schema, &mutation_scope.lanes)?;
        require_success_scope(&mut schema, &mutation_scope.required_success_lanes)?;
        let base_prompt = format!(
            "OUTPUT JSON SCHEMA (follow exactly):\n{}\n\nAssess an attempted effect, not whether words can be spoken. Impossible actions are inadmissible and receive bargains, not a roll. Choose DC only from 5,10,15,20,25,30. Every modifier reference must be copied exactly from ALLOWED REFERENCES. Modifier total is capped at +/-10. Never grant capability, custody, access, knowledge, or spatial reach absent from state. Accepted extraordinary permissions are binding: preserve their prerequisites, costs, limits, exposure, and effect ceiling exactly; they admit only effects within that scope. The campaign contract governs tone, pacing, focus, consequence style, and DM style. Obey every aggregate content boundary: line excludes the topic, veil keeps it off-screen, ask_first admits no new depiction without a current explicit acceptance. Never reveal attribution. State concrete success, mixed, and failure consequences and a bounded effect ceiling. The private scope projection has already removed mutation lanes that are not causally plausible for this exact attempt. The remaining lanes are an upper bound, not a request to use all of them. When admissible, every required_success_lane in MUTATION SCOPE must contain at least one mutation entry in both strong_effect and success_effect; a success stake may not claim an intended canonical change that its typed effect omits. Every non-empty mutation must be directly caused by the exact attempted means or realize the exact intended effect in that outcome band. A fact, relationship, clock, posture, or route being true, nearby, discoverable, or useful does not make changing it a consequence of an unrelated attempt. Do not append scene context as an observed finding unless the attempted means actually communicates it or the intended effect actually investigates or discloses it. Each supplied mutation lane is a bounded array of exact entries. Use an empty array when that lane does not change in an outcome; never add a duplicate or contradictory entry. Supplied entries may only name IDs copied exactly from PRESENT ACTORS and the enumerated schema, add or retire their exact goals or obligations, change their conditions or relationships, move only the acting actor along an existing route, advance or reduce existing clocks by a positive amount, or change existing institution posture. An actor_commitments entry records a present actor voluntarily adopting or retiring a bounded goal or obligation as an outcome; it does not make that actor a puppet, transfer authority, or guarantee unrelated future conduct. Strong and ordinary success share one visible stake, so give them identical commitment changes; the runtime binds each exact commitment into that stake. Use clock_advances when an outcome moves a pressure toward its consequence. Use clock_reductions when repair, relief, delay, or obstruction removes established progress. Never name the same clock in both arrays for one outcome. Existing informational outcomes may copy an exact statement from AVAILABLE INFORMATION FACTS through actor_knowledge_additions. A direct observation, measurement, inspection, or test may instead state one new bounded result through actor_observations. A subject receiving, recognizing, learning, remembering, or becoming able to act on information is a canonical knowledge transition and must have an exact actor_knowledge_additions entry for that exact present recipient. Never promise remote, institutional, population, place, office, or absent-actor receipt in a stake when no exact recipient knowledge lane exists; posture, narration, speech, and sender observation are not substitutes. Choose the fact that most directly answers the intended effect, preferring a relevant branch_local or provisional_local fact over generic canon background. A location-discoverable fact may be added only to the acting actor. A fact already known by the acting actor may instead be communicated to another present actor. Each actor_knowledge_additions or actor_observations entry contains the player-readable statement, never a fact ID, key, or label. An actor_observations statement is itself the canonical branch-local proposition committed on that outcome: state what is actually true. Do not say that the actor determines or establishes whether something is true, do not repeat the intended question, and do not leave 'if one exists' or another unresolved placeholder. A concrete negative result is valid. If canon is silent about harmless local texture needed to answer a direct inspection, improvise the smallest reversible detail consistent with the supplied location and evidence and preview it exactly; material geography, mechanics, institutions, or extraordinary capabilities remain outside this lane. actor_observations may target only the acting actor and must state only what the exact attempted means could establish at the current location; never use it for remote events, hidden motives, unsupported identities, or conclusions beyond the effect ceiling. Strong and ordinary success share one visible stake, so give them identical knowledge additions and observations. The runtime binds each exact finding into the player-visible stake; do not spend prose repeating it solely for formatting. If no supplied fact supports an intended disclosure, leave actor_knowledge_additions empty. If an intended local investigation can directly establish a bounded result, use actor_observations; otherwise make the limitation explicit in the stakes or mark the attempt inadmissible. Never invent remote events, hidden actors, unsupported proper nouns, or conclusions beyond the effect ceiling. Keep an effect empty only when the outcome truly has no canonical state change.\nMUTATION SCOPE:\n{}\nCAMPAIGN CONTRACT:\n{}\nAGGREGATE CONTENT BOUNDARIES:\n{}\nAGENCY BOUNDARY:\n{}\nLEGACY HOST ACTOR ID (not an authority):\n{}\nINTENT:\n{}\nACTOR:\n{}\nACCEPTED EXTRAORDINARY PERMISSIONS:\n{}\nLOCATION:\n{}\nPRESENT ACTORS:\n{}\nVISIBLE INSTITUTIONS:\n{}\nAVAILABLE INFORMATION FACTS:\n{}\nALLOWED REFERENCES:\n{}",
            serde_json::to_string(&schema)?,
            serde_json::to_string(&mutation_scope)?,
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
        let basis_digest =
            assessment_basis_digest(&self.verifier_model_id, &self.model_id, &base_prompt);
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
                &mutation_scope,
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
        let mut attempts = 0_u8;
        let (proposal, out, effect_verifier_receipt) = loop {
            attempts += 1;
            let mut out = run_validated_stage(
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
                let proposal = decode_assessment_proposal(
                    out.structured
                        .clone()
                        .ok_or_else(|| anyhow!("assessor returned no typed proposal"))?,
                )?;
                validate_and_bind_proposal(
                    proposal,
                    campaign,
                    actor,
                    &allowed_references,
                    &mutation_scope,
                )
            })();
            match candidate {
                Ok(proposal) => {
                    if !proposal.admissible {
                        if let Some(store) = store {
                            persist_private_stage_receipt(store, &out.receipt)?;
                        }
                        break (proposal, out, None);
                    }
                    let (verification, verifier_receipt) = verify_assessment_effects(
                        self.model.as_ref(),
                        &self.verifier_model_id,
                        campaign,
                        &intent,
                        &proposal,
                        &snapshot_binding,
                        attempts,
                    )
                    .await?;
                    if matches!(verification.result, AssessmentEffectMatchResult::Mismatch) {
                        out.receipt.validation_result = "semantic_invalid".into();
                        out.receipt.local_validation_error = verification
                            .repair_guidance
                            .as_deref()
                            .map(|guidance| guidance.chars().take(1_000).collect());
                    }
                    if let Some(store) = store {
                        persist_private_stage_receipt(store, &out.receipt)?;
                        persist_private_stage_receipt(store, &verifier_receipt)?;
                    }
                    match verification.result {
                        AssessmentEffectMatchResult::Match => {
                            validate_effect_verification(&verification)?;
                            break (proposal, out, Some(verifier_receipt));
                        }
                        AssessmentEffectMatchResult::Mismatch if attempts == 1 => {
                            validate_effect_verification(&verification)?;
                            let rejected = serde_json::to_string(&proposal)?;
                            let guidance = verification
                                .repair_guidance
                                .as_deref()
                                .expect("validated mismatch requires repair guidance");
                            correction = format!(
                                "\n\nSEMANTIC EFFECT VERIFIER REJECTED THE PREVIOUS ASSESSMENT.\nREPAIR GUIDANCE: {guidance}\nPREVIOUS ASSESSMENT:\n{rejected}\nReturn one corrected complete assessment against the same snapshot. Preserve the exact means, intended effect, admissibility, and all causally faithful stakes and mutations. Remove or align the rejected mutation; do not replace it with another merely available fact or unrelated side effect."
                            );
                        }
                        AssessmentEffectMatchResult::Mismatch => {
                            validate_effect_verification(&verification)?;
                            return Err(anyhow!(
                                "assessment failed semantic effect verification after one correction: {}",
                                verification
                                    .repair_guidance
                                    .as_deref()
                                    .unwrap_or("effect mismatch")
                            ));
                        }
                    }
                }
                Err(error) if attempts == 1 => {
                    out.receipt.validation_result = "semantic_invalid".into();
                    out.receipt.local_validation_error =
                        Some(error.to_string().chars().take(1_000).collect());
                    if let Some(store) = store {
                        persist_private_stage_receipt(store, &out.receipt)?;
                    }
                    let rejected = out
                        .structured
                        .as_ref()
                        .and_then(|value| serde_json::to_string(value).ok())
                        .unwrap_or_else(|| "unavailable".into());
                    correction = format!(
                        "\n\nLOCAL VALIDATOR REJECTED THE PREVIOUS ASSESSMENT: {error}\nPREVIOUS ASSESSMENT:\n{rejected}\nReturn a corrected complete assessment against the same snapshot. Copy every modifier reference from ALLOWED REFERENCES exactly; omit a modifier rather than paraphrasing or inventing its reference. Copy every actor and destination ID exactly from the supplied state. Every knowledge addition must copy one exact statement from AVAILABLE INFORMATION FACTS and obey its access mode; strong and ordinary success must use identical knowledge additions. Otherwise leave the corresponding mutation array empty."
                    );
                }
                Err(error) => {
                    out.receipt.validation_result = "semantic_invalid".into();
                    out.receipt.local_validation_error =
                        Some(error.to_string().chars().take(1_000).collect());
                    if let Some(store) = store {
                        persist_private_stage_receipt(store, &out.receipt)?;
                    }
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
                source_scope_receipt_hash: source_scope_receipt_hash.clone(),
                source_receipt_hash: selected_receipt.receipt_hash.clone(),
                source_effect_verifier_receipt_hash: effect_verifier_receipt
                    .map(|receipt| receipt.receipt_hash),
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
                    &mutation_scope,
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

    async fn select_mutation_scope(
        &self,
        store: Option<&CampaignStore>,
        campaign: &Campaign,
        intent: &ActionIntent,
        actor: &crate::domain::ActorState,
        extraordinary_permissions: &[ExtraordinaryPermission],
        campaign_contract: Option<&CampaignContract>,
        aggregate_boundaries: &[AggregatedBoundary],
    ) -> Result<(AssessmentMutationScope, ModelStageReceipt)> {
        let available_lanes = available_mutation_lanes(campaign, actor);
        let mut schema = serde_json::to_value(schema_for!(AssessmentMutationScope))?;
        constrain_mutation_scope_schema(&mut schema, &available_lanes)?;
        let authority = assessment_admission_authority(campaign, actor);
        let scope_prompt = format!(
            "{ASSESSMENT_SCOPE_INSTRUCTIONS}\nOUTPUT JSON SCHEMA (follow exactly):\n{}\nEXACT ATTEMPT:\n{}\nEXACT CURRENT AUTHORITY:\n{}\nACCEPTED EXTRAORDINARY PERMISSIONS:\n{}\nCAMPAIGN CONTRACT:\n{}\nAGGREGATE CONTENT BOUNDARIES:\n{}\nSTRUCTURALLY AVAILABLE LANES:\n{}",
            serde_json::to_string(&schema)?,
            serde_json::to_string(intent)?,
            serde_json::to_string(&authority)?,
            serde_json::to_string(extraordinary_permissions)?,
            serde_json::to_string(&campaign_contract)?,
            serde_json::to_string(aggregate_boundaries)?,
            serde_json::to_string(&available_lanes)?,
        );
        let basis_digest = format!(
            "sha256:{:x}",
            Sha256::digest(
                format!(
                    "{}|assessment_mutation_scope|{}|{}",
                    ASSESSMENT_SEMANTICS_VERSION, self.verifier_model_id, scope_prompt
                )
                .as_bytes()
            )
        );
        if let Some(store) = store
            && let Some((_, cached)) = store.load::<AssessmentMutationScopeCacheEntry>(
                ASSESSMENT_SCOPE_CACHE_KIND,
                &basis_digest,
            )?
        {
            if cached.schema != ASSESSMENT_SCOPE_CACHE_SCHEMA || cached.basis_digest != basis_digest
            {
                return Err(anyhow!("assessment mutation scope cache identity mismatch"));
            }
            validate_mutation_scope(&cached.scope, &available_lanes)?;
            let receipt = scope_cache_hit_receipt(
                &cached,
                &snapshot_binding_for_scope(campaign, &basis_digest),
                &scope_prompt,
            )?;
            return Ok((cached.scope, receipt));
        }
        let snapshot_binding = snapshot_binding_for_scope(campaign, &basis_digest);
        let mut correction = String::new();
        let mut attempt = 0_u8;
        let (scope, out) = loop {
            attempt += 1;
            let mut out = run_validated_stage(
                self.model.as_ref(),
                &ModelStageRequest {
                    stage: "assessment_mutation_scope".into(),
                    model: self.verifier_model_id.clone(),
                    snapshot_binding: snapshot_binding.clone(),
                    lived_stream: format!("{scope_prompt}{correction}"),
                    output_schema: Some(schema.clone()),
                    source_receipt_ids: Vec::new(),
                    temperature: Some(0.0),
                    max_output_tokens: Some(900),
                },
            )
            .await?;
            let candidate = (|| -> Result<AssessmentMutationScope> {
                let scope = serde_json::from_value(out.structured.clone().ok_or_else(|| {
                    anyhow!("assessment mutation scope returned no typed output")
                })?)?;
                validate_mutation_scope(&scope, &available_lanes)?;
                Ok(scope)
            })();
            match candidate {
                Ok(scope) => break (scope, out),
                Err(error) if attempt == 1 => {
                    out.receipt.validation_result = "semantic_invalid".into();
                    out.receipt.local_validation_error =
                        Some(error.to_string().chars().take(1_000).collect());
                    if let Some(store) = store {
                        persist_private_stage_receipt(store, &out.receipt)?;
                    }
                    correction = format!(
                        "\n\nLOCAL VALIDATOR REJECTED THE PREVIOUS ADMISSION SCOPE: {error}\nReturn one corrected complete object against the same snapshot. An assessable scope has no denial. A denied scope has a denial, one to four bargains, and no lanes."
                    );
                }
                Err(error) => {
                    out.receipt.validation_result = "semantic_invalid".into();
                    out.receipt.local_validation_error =
                        Some(error.to_string().chars().take(1_000).collect());
                    if let Some(store) = store {
                        persist_private_stage_receipt(store, &out.receipt)?;
                    }
                    return Err(anyhow!(
                        "assessment mutation scope failed local validation after one correction: {error}"
                    ));
                }
            }
        };
        if let Some(store) = store {
            persist_private_stage_receipt(store, &out.receipt)?;
            let entry = AssessmentMutationScopeCacheEntry {
                schema: ASSESSMENT_SCOPE_CACHE_SCHEMA.into(),
                basis_digest: basis_digest.clone(),
                scope: scope.clone(),
                source_provider: out.receipt.provider.clone(),
                source_model: out.receipt.model.clone(),
                source_receipt_hash: out.receipt.receipt_hash.clone(),
                created_at: Utc::now(),
            };
            if let Err(insert_error) = store.insert(
                ASSESSMENT_SCOPE_CACHE_KIND,
                ASSESSMENT_SCOPE_CACHE_SCHEMA,
                &basis_digest,
                &entry,
            ) {
                let (_, winner) = store
                    .load::<AssessmentMutationScopeCacheEntry>(
                        ASSESSMENT_SCOPE_CACHE_KIND,
                        &basis_digest,
                    )?
                    .ok_or_else(|| {
                        anyhow!(
                            "assessment mutation scope cache insert failed without an existing winner: {insert_error}"
                        )
                    })?;
                if winner.schema != ASSESSMENT_SCOPE_CACHE_SCHEMA
                    || winner.basis_digest != basis_digest
                {
                    return Err(anyhow!(
                        "assessment mutation scope cache winner identity mismatch"
                    ));
                }
                validate_mutation_scope(&winner.scope, &available_lanes)?;
                let receipt = scope_cache_hit_receipt(&winner, &snapshot_binding, &scope_prompt)?;
                return Ok((winner.scope, receipt));
            }
        }
        Ok((scope, out.receipt))
    }
}

fn decode_assessment_proposal(value: serde_json::Value) -> Result<AssessmentProposal> {
    let mut wrapper = value
        .as_object()
        .cloned()
        .ok_or_else(|| anyhow!("assessor output is not an object"))?;
    if wrapper.len() != 1 || !wrapper.contains_key("proposal") {
        return Err(anyhow!(
            "assessor output must contain exactly one admission-bound proposal"
        ));
    }
    let mut value = wrapper
        .remove("proposal")
        .ok_or_else(|| anyhow!("assessor output omitted proposal"))?;
    let proposal = value
        .as_object_mut()
        .ok_or_else(|| anyhow!("assessor proposal is not an object"))?;
    for field in [
        "strong_effect",
        "success_effect",
        "mixed_effect",
        "failure_effect",
    ] {
        let effect = proposal
            .get(field)
            .ok_or_else(|| anyhow!("assessor output omitted {field}"))?;
        proposal.insert(
            field.into(),
            serde_json::to_value(decode_effect_entries(effect)?)?,
        );
    }
    serde_json::from_value(value).map_err(Into::into)
}

fn decode_effect_entries(value: &serde_json::Value) -> Result<WorldEffectDelta> {
    let fields = value
        .as_object()
        .ok_or_else(|| anyhow!("assessor effect is not an object"))?;
    let allowed_fields = BTreeSet::from([
        "actor_conditions",
        "actor_commitments",
        "actor_knowledge_additions",
        "actor_observations",
        "actor_relationship_updates",
        "actor_moves",
        "clock_advances",
        "clock_reductions",
        "institution_postures",
    ]);
    if let Some(field) = fields
        .keys()
        .find(|field| !allowed_fields.contains(field.as_str()))
    {
        return Err(anyhow!(
            "assessor effect contains unknown mutation lane {field}"
        ));
    }

    let mut effect = WorldEffectDelta::default();
    for entry in decode_mutation_entries::<ActorConditionMutation>(fields, "actor_conditions")? {
        let delta = effect
            .actor_conditions
            .entry(entry.actor_id.clone())
            .or_default();
        let (selected, opposite) = match entry.operation {
            ConditionMutationOperation::Add => (&mut delta.add, &delta.remove),
            ConditionMutationOperation::Remove => (&mut delta.remove, &delta.add),
        };
        if opposite.contains(&entry.condition) {
            return Err(anyhow!(
                "condition mutation both adds and removes {:?} for {}",
                entry.condition,
                entry.actor_id
            ));
        }
        if !selected.insert(entry.condition.clone()) {
            return Err(anyhow!(
                "duplicate condition mutation {:?} for {}",
                entry.condition,
                entry.actor_id
            ));
        }
    }
    for entry in decode_mutation_entries::<ActorCommitmentMutation>(fields, "actor_commitments")? {
        let delta = effect
            .actor_commitments
            .entry(entry.actor_id.clone())
            .or_default();
        let (selected, opposite) = match entry.operation {
            ActorCommitmentMutationOperation::AddGoal => {
                (&mut delta.goals_add, &delta.goals_retire)
            }
            ActorCommitmentMutationOperation::RetireGoal => {
                (&mut delta.goals_retire, &delta.goals_add)
            }
            ActorCommitmentMutationOperation::AddObligation => {
                (&mut delta.obligations_add, &delta.obligations_retire)
            }
            ActorCommitmentMutationOperation::RetireObligation => {
                (&mut delta.obligations_retire, &delta.obligations_add)
            }
        };
        if opposite.contains(&entry.description) {
            return Err(anyhow!(
                "commitment mutation both adds and retires {:?} for {}",
                entry.description,
                entry.actor_id
            ));
        }
        if !selected.insert(entry.description.clone()) {
            return Err(anyhow!(
                "duplicate commitment mutation {:?} for {}",
                entry.description,
                entry.actor_id
            ));
        }
    }
    for entry in
        decode_mutation_entries::<ActorKnowledgeMutation>(fields, "actor_knowledge_additions")?
    {
        if !effect
            .actor_knowledge_additions
            .entry(entry.actor_id.clone())
            .or_default()
            .insert(entry.statement.clone())
        {
            return Err(anyhow!(
                "duplicate knowledge mutation {:?} for {}",
                entry.statement,
                entry.actor_id
            ));
        }
    }
    for entry in decode_mutation_entries::<ActorObservationMutation>(fields, "actor_observations")?
    {
        if !effect
            .actor_observations
            .entry(entry.actor_id.clone())
            .or_default()
            .insert(entry.statement.clone())
        {
            return Err(anyhow!(
                "duplicate observation mutation {:?} for {}",
                entry.statement,
                entry.actor_id
            ));
        }
    }
    for entry in
        decode_mutation_entries::<ActorRelationshipMutation>(fields, "actor_relationship_updates")?
    {
        if effect
            .actor_relationship_updates
            .entry(entry.actor_id.clone())
            .or_default()
            .insert(entry.target_id.clone(), entry.relationship)
            .is_some()
        {
            return Err(anyhow!(
                "duplicate relationship mutation from {} to {}",
                entry.actor_id,
                entry.target_id
            ));
        }
    }
    for entry in decode_mutation_entries::<ActorMoveMutation>(fields, "actor_moves")? {
        if effect
            .actor_moves
            .insert(entry.actor_id.clone(), entry.destination_id)
            .is_some()
        {
            return Err(anyhow!(
                "duplicate movement mutation for {}",
                entry.actor_id
            ));
        }
    }
    for entry in decode_mutation_entries::<ClockMutation>(fields, "clock_advances")? {
        if effect
            .clock_advances
            .insert(entry.clock_id.clone(), entry.amount)
            .is_some()
        {
            return Err(anyhow!("duplicate clock advance for {}", entry.clock_id));
        }
    }
    for entry in decode_mutation_entries::<ClockMutation>(fields, "clock_reductions")? {
        if effect
            .clock_reductions
            .insert(entry.clock_id.clone(), entry.amount)
            .is_some()
        {
            return Err(anyhow!("duplicate clock reduction for {}", entry.clock_id));
        }
    }
    for entry in
        decode_mutation_entries::<InstitutionPostureMutation>(fields, "institution_postures")?
    {
        if effect
            .institution_postures
            .insert(entry.institution_id.clone(), entry.posture)
            .is_some()
        {
            return Err(anyhow!(
                "duplicate institution posture mutation for {}",
                entry.institution_id
            ));
        }
    }
    Ok(effect)
}

fn decode_mutation_entries<T>(
    fields: &serde_json::Map<String, serde_json::Value>,
    lane: &str,
) -> Result<Vec<T>>
where
    T: serde::de::DeserializeOwned,
{
    match fields.get(lane) {
        None | Some(serde_json::Value::Null) => Ok(Vec::new()),
        Some(value @ serde_json::Value::Array(_)) => {
            serde_json::from_value(value.clone()).map_err(Into::into)
        }
        Some(_) => Err(anyhow!("assessor mutation lane {lane} is not an array")),
    }
}

fn validate_and_bind_proposal(
    mut proposal: AssessmentProposal,
    campaign: &Campaign,
    actor: &crate::domain::ActorState,
    allowed_references: &BTreeSet<String>,
    mutation_scope: &AssessmentMutationScope,
) -> Result<AssessmentProposal> {
    bind_visible_effects(&mut proposal)?;
    validate_proposal(&proposal, allowed_references)?;
    for (effect, stake) in [
        (&proposal.strong_effect, &proposal.success_stake),
        (&proposal.success_effect, &proposal.success_stake),
        (&proposal.mixed_effect, &proposal.mixed_stake),
        (&proposal.failure_effect, &proposal.failure_stake),
    ] {
        validate_effect(campaign, actor, effect, stake)?;
    }
    validate_required_success_lanes(&proposal, mutation_scope)?;
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

fn assessment_basis_digest(verifier_model_id: &str, model_id: &str, base_prompt: &str) -> String {
    format!(
        "sha256:{:x}",
        Sha256::digest(
            format!(
                "{ASSESSMENT_SEMANTICS_VERSION}|{verifier_model_id}|{ASSESSMENT_EFFECT_VERIFIER_INSTRUCTIONS}|{model_id}|{base_prompt}"
            )
            .as_bytes()
        )
    )
}

async fn verify_assessment_effects(
    model: &dyn ModelPort,
    verifier_model_id: &str,
    campaign: &Campaign,
    intent: &ActionIntent,
    proposal: &AssessmentProposal,
    snapshot_binding: &str,
    assessment_attempt: u8,
) -> Result<(AssessmentEffectVerification, ModelStageReceipt)> {
    let referenced_state = assessment_effect_reference_context(campaign, proposal);
    let context = serde_json::json!({
        "means":intent.description,
        "intended_effect":intent.intended_effect,
        "effect_ceiling":proposal.effect_ceiling,
        "success_stake":proposal.success_stake,
        "mixed_stake":proposal.mixed_stake,
        "failure_stake":proposal.failure_stake,
        "typed_effects":{
            "strong_success":proposal.strong_effect,
            "success":proposal.success_effect,
            "mixed":proposal.mixed_effect,
            "failure":proposal.failure_effect,
        },
        "referenced_state":referenced_state,
    });
    let request = ModelStageRequest {
        stage: "assessment_effect_verifier".into(),
        model: verifier_model_id.into(),
        snapshot_binding: format!(
            "{}:assessment-attempt:{}:assessment-effect:{}",
            snapshot_binding,
            assessment_attempt,
            format!("{:x}", Sha256::digest(serde_json::to_vec(&context)?))
        ),
        lived_stream: format!(
            "{ASSESSMENT_EFFECT_VERIFIER_INSTRUCTIONS}\n\nCANDIDATE:\n{}",
            serde_json::to_string(&context)?
        ),
        output_schema: Some(serde_json::to_value(schema_for!(
            AssessmentEffectVerification
        ))?),
        source_receipt_ids: campaign.branch_origin.evidence_receipt_ids.clone(),
        temperature: Some(0.0),
        max_output_tokens: Some(192),
    };
    let output = run_validated_stage(model, &request).await?;
    let verification: AssessmentEffectVerification = serde_json::from_value(
        output
            .structured
            .clone()
            .ok_or_else(|| anyhow!("assessment effect verifier produced no typed verdict"))?,
    )?;
    validate_effect_verification(&verification)?;
    let mut receipt = output.receipt;
    if matches!(verification.result, AssessmentEffectMatchResult::Mismatch) {
        receipt.validation_result = "semantic_invalid".into();
        receipt.local_validation_error = verification
            .repair_guidance
            .as_deref()
            .map(|guidance| guidance.chars().take(1_000).collect());
    }
    Ok((verification, receipt))
}

fn assessment_effect_reference_context(
    campaign: &Campaign,
    proposal: &AssessmentProposal,
) -> serde_json::Value {
    let mut actor_ids = BTreeSet::new();
    let mut institution_ids = BTreeSet::new();
    let mut clock_ids = BTreeSet::new();
    let mut location_ids = BTreeSet::new();
    for effect in [
        &proposal.strong_effect,
        &proposal.success_effect,
        &proposal.mixed_effect,
        &proposal.failure_effect,
    ] {
        actor_ids.extend(effect.actor_conditions.keys().cloned());
        actor_ids.extend(effect.actor_commitments.keys().cloned());
        actor_ids.extend(effect.actor_knowledge_additions.keys().cloned());
        actor_ids.extend(effect.actor_observations.keys().cloned());
        actor_ids.extend(effect.actor_relationship_updates.keys().cloned());
        actor_ids.extend(effect.actor_moves.keys().cloned());
        for target_id in effect
            .actor_relationship_updates
            .values()
            .flat_map(|relationships| relationships.keys())
        {
            if campaign.actors.contains_key(target_id) {
                actor_ids.insert(target_id.clone());
            } else if campaign.institutions.contains_key(target_id) {
                institution_ids.insert(target_id.clone());
            }
        }
        location_ids.extend(effect.actor_moves.values().cloned());
        clock_ids.extend(effect.clock_advances.keys().cloned());
        clock_ids.extend(effect.clock_reductions.keys().cloned());
        institution_ids.extend(effect.institution_postures.keys().cloned());
    }
    let actors = actor_ids
        .into_iter()
        .filter_map(|id| {
            campaign.actors.get(&id).map(|actor| {
                (
                    id,
                    serde_json::json!({
                        "name":&actor.name,
                        "goals":&actor.goals,
                        "obligations":&actor.obligations,
                        "relationships":&actor.relationships,
                    }),
                )
            })
        })
        .collect::<BTreeMap<_, _>>();
    let institutions = institution_ids
        .into_iter()
        .filter_map(|id| {
            campaign
                .institutions
                .get(&id)
                .map(|institution| (id, institution.name.clone()))
        })
        .collect::<BTreeMap<_, _>>();
    let clocks = clock_ids
        .into_iter()
        .filter_map(|id| {
            campaign
                .clocks
                .get(&id)
                .map(|clock| (id, clock.label.clone()))
        })
        .collect::<BTreeMap<_, _>>();
    let locations = location_ids
        .into_iter()
        .filter_map(|id| {
            campaign
                .locations
                .get(&id)
                .map(|location| (id, location.name.clone()))
        })
        .collect::<BTreeMap<_, _>>();
    serde_json::json!({
        "actors":actors,
        "institutions":institutions,
        "clocks":clocks,
        "locations":locations,
    })
}

fn validate_effect_verification(verification: &AssessmentEffectVerification) -> Result<()> {
    match (
        &verification.result,
        &verification.mismatch_kind,
        &verification.repair_guidance,
    ) {
        (AssessmentEffectMatchResult::Match, None, None) => Ok(()),
        (AssessmentEffectMatchResult::Mismatch, Some(_), Some(guidance))
            if !guidance.trim().is_empty() && guidance.chars().count() <= 240 =>
        {
            Ok(())
        }
        _ => Err(anyhow!(
            "assessment effect verifier returned an incoherent verdict"
        )),
    }
}

fn persist_private_stage_receipt(store: &CampaignStore, receipt: &ModelStageReceipt) -> Result<()> {
    match store.insert(
        "persona_stage_receipt.v1",
        "ghostlight.persona_stage_receipt.v1",
        receipt.storage_key(),
        receipt,
    ) {
        Ok(_) => Ok(()),
        Err(_error)
            if store
                .load::<ModelStageReceipt>("persona_stage_receipt.v1", receipt.storage_key())?
                .is_some() =>
        {
            Ok(())
        }
        Err(error) => Err(error),
    }
}

fn snapshot_binding_for_scope(campaign: &Campaign, basis_digest: &str) -> String {
    format!(
        "campaign:{}:revision:{}:assessment-mutation-scope:{}",
        campaign.id, campaign.revision, basis_digest
    )
}

fn scope_cache_hit_receipt(
    cached: &AssessmentMutationScopeCacheEntry,
    snapshot_binding: &str,
    scope_prompt: &str,
) -> Result<ModelStageReceipt> {
    let output = serde_json::to_string(&cached.scope)?;
    let output_hash = format!("sha256:{:x}", Sha256::digest(output.as_bytes()));
    let request_hash = format!(
        "sha256:{:x}",
        Sha256::digest(
            format!(
                "{}|{}|{}|{}",
                cached.basis_digest, snapshot_binding, cached.source_model, scope_prompt
            )
            .as_bytes()
        )
    );
    let receipt_hash = format!(
        "sha256:{:x}",
        Sha256::digest(
            format!(
                "{}|{}|assessment_mutation_scope|{}|{}|{}|{}",
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
        stage: "assessment_mutation_scope".into(),
        snapshot_binding: snapshot_binding.into(),
        request_hash,
        output_hash,
        source_receipt_ids: Vec::new(),
        latency_ms: 0,
        validation_result: "valid_cache_hit".into(),
        local_validation_error: None,
        input_chars: scope_prompt.chars().count(),
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
                "{}|{}|action_assessment|{}|{}|{}|{}|{}",
                cached.source_provider,
                cached.source_model,
                snapshot_binding,
                request_hash,
                output_hash,
                cached.source_scope_receipt_hash,
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
            transport_features: {
                let mut features = vec![
                    "cultcache.output-cache".into(),
                    format!("source-mutation-scope:{}", cached.source_scope_receipt_hash),
                    format!("source-receipt:{}", cached.source_receipt_hash),
                ];
                if let Some(hash) = &cached.source_effect_verifier_receipt_hash {
                    features.push(format!("source-effect-verifier:{hash}"));
                }
                features
            },
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
        .values()
        .map(|route| route.destination_id.clone())
        .filter(|destination| campaign.locations.contains_key(destination))
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

    for field in [
        "actor_conditions",
        "actor_commitments",
        "actor_relationship_updates",
    ] {
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
    constrain_map_keys(
        effect_properties
            .get_mut("actor_observations")
            .ok_or_else(|| anyhow!("assessment effect schema omitted actor_observations"))?,
        &BTreeSet::from([acting_actor.id.clone()]),
    )?;
    effect_properties["actor_observations"]["additionalProperties"] = serde_json::json!({
        "type":"array",
        "items":{"type":"string","minLength":1,"maxLength":500},
        "uniqueItems":true,
        "minItems":1,
        "maxItems":2
    });
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
    institution_postures["additionalProperties"] = serde_json::json!({
        "type":"string",
        "minLength":1,
        "maxLength":MAX_POSTURE_CHARS
    });
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

fn project_effect_schema_to_mutation_entries(
    schema: &mut serde_json::Value,
    campaign: &Campaign,
) -> Result<()> {
    let properties = schema
        .pointer_mut("/$defs/WorldEffectDelta/properties")
        .and_then(serde_json::Value::as_object_mut)
        .ok_or_else(|| anyhow!("assessment schema has no world effect properties"))?;

    if let Some(map) = properties.get("actor_conditions").cloned() {
        let actor_ids = constrained_map_keys(&map, "actor_conditions")?;
        properties.insert(
            "actor_conditions".into(),
            mutation_entry_array(
                serde_json::json!({
                    "type":"object",
                    "properties":{
                        "actor_id":typed_string_enum(&actor_ids),
                        "operation":{"type":"string","enum":["add","remove"]},
                        "condition":{"type":"string","minLength":1}
                    },
                    "required":["actor_id","operation","condition"],
                    "additionalProperties":false
                }),
                bounded_entry_count(actor_ids.len().saturating_mul(4)),
            ),
        );
    }

    if let Some(map) = properties.get("actor_commitments").cloned() {
        let actor_ids = constrained_map_keys(&map, "actor_commitments")?;
        let mut alternatives = Vec::new();
        for actor_id in &actor_ids {
            let actor = campaign
                .actors
                .get(actor_id)
                .ok_or_else(|| anyhow!("assessment commitment actor vanished"))?;
            for operation in ["add_goal", "add_obligation"] {
                alternatives.push(serde_json::json!({
                    "type":"object",
                    "properties":{
                        "actor_id":{"type":"string","enum":[actor_id]},
                        "operation":{"type":"string","enum":[operation]},
                        "description":{"type":"string","minLength":1,"maxLength":600}
                    },
                    "required":["actor_id","operation","description"],
                    "additionalProperties":false
                }));
            }
            if !actor.goals.is_empty() {
                alternatives.push(serde_json::json!({
                    "type":"object",
                    "properties":{
                        "actor_id":{"type":"string","enum":[actor_id]},
                        "operation":{"type":"string","enum":["retire_goal"]},
                        "description":{"type":"string","enum":&actor.goals}
                    },
                    "required":["actor_id","operation","description"],
                    "additionalProperties":false
                }));
            }
            if !actor.obligations.is_empty() {
                alternatives.push(serde_json::json!({
                    "type":"object",
                    "properties":{
                        "actor_id":{"type":"string","enum":[actor_id]},
                        "operation":{"type":"string","enum":["retire_obligation"]},
                        "description":{"type":"string","enum":&actor.obligations}
                    },
                    "required":["actor_id","operation","description"],
                    "additionalProperties":false
                }));
            }
        }
        let items = if alternatives.len() == 1 {
            alternatives.remove(0)
        } else {
            serde_json::json!({"anyOf":alternatives})
        };
        properties.insert(
            "actor_commitments".into(),
            mutation_entry_array(
                items,
                bounded_entry_count(actor_ids.len().saturating_mul(4)),
            ),
        );
    }

    if let Some(map) = properties.get("actor_knowledge_additions").cloned() {
        let recipient_schemas = map
            .get("properties")
            .and_then(serde_json::Value::as_object)
            .ok_or_else(|| anyhow!("assessment knowledge lane has no recipient properties"))?;
        let mut alternatives = Vec::new();
        let mut maximum = 0_usize;
        for (actor_id, statements) in recipient_schemas {
            let statement_schema = statements
                .get("items")
                .cloned()
                .ok_or_else(|| anyhow!("assessment knowledge recipient has no statement schema"))?;
            maximum = maximum.saturating_add(
                statements
                    .get("maxItems")
                    .and_then(serde_json::Value::as_u64)
                    .unwrap_or(1) as usize,
            );
            alternatives.push(serde_json::json!({
                "type":"object",
                "properties":{
                    "actor_id":{"type":"string","enum":[actor_id]},
                    "statement":statement_schema
                },
                "required":["actor_id","statement"],
                "additionalProperties":false
            }));
        }
        if alternatives.is_empty() {
            return Err(anyhow!(
                "assessment knowledge lane survived without an authorized recipient"
            ));
        }
        let items = if alternatives.len() == 1 {
            alternatives.remove(0)
        } else {
            serde_json::json!({"anyOf":alternatives})
        };
        properties.insert(
            "actor_knowledge_additions".into(),
            mutation_entry_array(items, bounded_entry_count(maximum)),
        );
    }

    if let Some(map) = properties.get("actor_relationship_updates").cloned() {
        let actor_ids = constrained_map_keys(&map, "actor_relationship_updates")?;
        let target_map = map
            .get("additionalProperties")
            .ok_or_else(|| anyhow!("assessment relationship lane has no target map"))?;
        let target_ids = constrained_map_keys(target_map, "actor_relationship_updates targets")?;
        properties.insert(
            "actor_relationship_updates".into(),
            mutation_entry_array(
                serde_json::json!({
                    "type":"object",
                    "properties":{
                        "actor_id":typed_string_enum(&actor_ids),
                        "target_id":typed_string_enum(&target_ids),
                        "relationship":{"type":"string","minLength":1}
                    },
                    "required":["actor_id","target_id","relationship"],
                    "additionalProperties":false
                }),
                bounded_entry_count(actor_ids.len().saturating_mul(target_ids.len())),
            ),
        );
    }

    if let Some(map) = properties.get("actor_observations").cloned() {
        let actor_ids = constrained_map_keys(&map, "actor_observations")?;
        properties.insert(
            "actor_observations".into(),
            mutation_entry_array(
                serde_json::json!({
                    "type":"object",
                    "properties":{
                        "actor_id":typed_string_enum(&actor_ids),
                        "statement":{"type":"string","minLength":1,"maxLength":500}
                    },
                    "required":["actor_id","statement"],
                    "additionalProperties":false
                }),
                2,
            ),
        );
    }

    if let Some(map) = properties.get("actor_moves").cloned() {
        let actor_ids = constrained_map_keys(&map, "actor_moves")?;
        let destination_ids = constrained_value_enum(&map, "actor_moves destinations")?;
        properties.insert(
            "actor_moves".into(),
            mutation_entry_array(
                serde_json::json!({
                    "type":"object",
                    "properties":{
                        "actor_id":typed_string_enum(&actor_ids),
                        "destination_id":typed_string_enum(&destination_ids)
                    },
                    "required":["actor_id","destination_id"],
                    "additionalProperties":false
                }),
                bounded_entry_count(actor_ids.len()),
            ),
        );
    }

    for field in ["clock_advances", "clock_reductions"] {
        if let Some(map) = properties.get(field).cloned() {
            let clock_ids = constrained_map_keys(&map, field)?;
            properties.insert(
                field.into(),
                mutation_entry_array(
                    serde_json::json!({
                        "type":"object",
                        "properties":{
                            "clock_id":typed_string_enum(&clock_ids),
                            "amount":{"type":"integer","minimum":1,"maximum":255}
                        },
                        "required":["clock_id","amount"],
                        "additionalProperties":false
                    }),
                    bounded_entry_count(clock_ids.len()),
                ),
            );
        }
    }

    if let Some(map) = properties.get("institution_postures").cloned() {
        let institution_ids = constrained_map_keys(&map, "institution_postures")?;
        properties.insert(
            "institution_postures".into(),
            mutation_entry_array(
                serde_json::json!({
                    "type":"object",
                    "properties":{
                        "institution_id":typed_string_enum(&institution_ids),
                        "posture":{"type":"string","minLength":1,"maxLength":MAX_POSTURE_CHARS}
                    },
                    "required":["institution_id","posture"],
                    "additionalProperties":false
                }),
                bounded_entry_count(institution_ids.len()),
            ),
        );
    }
    Ok(())
}

fn bounded_entry_count(available: usize) -> usize {
    available.clamp(1, 16)
}

fn mutation_entry_array(items: serde_json::Value, max_items: usize) -> serde_json::Value {
    serde_json::json!({
        "type":"array",
        "items":items,
        "maxItems":max_items
    })
}

fn constrained_map_keys(schema: &serde_json::Value, lane: &str) -> Result<Vec<String>> {
    schema
        .pointer("/propertyNames/enum")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| anyhow!("assessment {lane} lane has no authorized keys"))?
        .iter()
        .map(|value| {
            value
                .as_str()
                .map(ToOwned::to_owned)
                .ok_or_else(|| anyhow!("assessment {lane} lane has a non-string key"))
        })
        .collect()
}

fn constrained_value_enum(schema: &serde_json::Value, lane: &str) -> Result<Vec<String>> {
    schema
        .pointer("/additionalProperties/enum")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| anyhow!("assessment {lane} lane has no authorized values"))?
        .iter()
        .map(|value| {
            value
                .as_str()
                .map(ToOwned::to_owned)
                .ok_or_else(|| anyhow!("assessment {lane} lane has a non-string value"))
        })
        .collect()
}

fn typed_string_enum(values: &[String]) -> serde_json::Value {
    serde_json::json!({"type":"string","enum":values})
}

fn assessment_admission_authority(
    campaign: &Campaign,
    acting_actor: &crate::domain::ActorState,
) -> serde_json::Value {
    let location = campaign.locations.get(&acting_actor.location_id);
    let routes = location
        .into_iter()
        .flat_map(|location| {
            location.routes.iter().map(|(route_id, route)| {
                serde_json::json!({
                    "route_id":route_id,
                    "destination_id":route.destination_id,
                    "destination_name":campaign.locations.get(&route.destination_id).map(|value| value.name.as_str()),
                    "distance":route.distance,
                    "travel_minutes":route.travel_minutes,
                })
            })
        })
        .collect::<Vec<_>>();
    let present_actors = campaign
        .actors
        .values()
        .filter(|candidate| candidate.location_id == acting_actor.location_id)
        .map(|candidate| {
            serde_json::json!({
                "id":candidate.id,
                "name":candidate.name,
                "conditions":candidate.conditions,
                "relationship_to_actor":candidate.relationships.get(&acting_actor.id),
            })
        })
        .collect::<Vec<_>>();
    let institutions = campaign
        .institutions
        .values()
        .map(|institution| {
            let profile = campaign.agency_profiles.get(&institution.id);
            serde_json::json!({
                "id":institution.id,
                "name":institution.name,
                "posture":institution.posture,
                "location_ids":profile.map(|value| &value.location_ids),
                "collective_authority_id":profile.and_then(|value| value.collective_authority_id.as_deref()),
                "information_channels":profile.map(|value| &value.information_channels),
            })
        })
        .collect::<Vec<_>>();
    let clocks = campaign
        .clocks
        .values()
        .map(|clock| {
            serde_json::json!({
                "id":clock.id,
                "label":clock.label,
                "progress":clock.progress,
                "threshold":clock.threshold,
                "consequence":clock.consequence,
            })
        })
        .collect::<Vec<_>>();
    serde_json::json!({
        "actor":{
            "id":acting_actor.id,
            "location_id":acting_actor.location_id,
            "capabilities":acting_actor.capabilities,
            "knowledge":acting_actor.knowledge,
            "equipment":acting_actor.equipment,
            "conditions":acting_actor.conditions,
            "obligations":acting_actor.obligations,
            "relationships":acting_actor.relationships,
        },
        "location":location.map(|value| serde_json::json!({
            "id":value.id,
            "name":value.name,
            "persistent_features":value.persistent_features,
        })),
        "routes":routes,
        "present_actors":present_actors,
        "institutions":institutions,
        "clocks":clocks,
        "accessible_information_facts":available_information_facts(campaign, acting_actor),
    })
}

fn denied_assessment_proposal(scope: &AssessmentMutationScope) -> Result<AssessmentProposal> {
    let denial = scope
        .denial
        .as_ref()
        .ok_or_else(|| anyhow!("denied assessment scope omitted its denial"))?;
    Ok(AssessmentProposal {
        normalized_intent: denial.normalized_intent.clone(),
        admissible: false,
        missing_permission: Some(denial.missing_permission.clone()),
        dc: 30,
        modifiers: Vec::new(),
        effect_ceiling: denial.effect_ceiling.clone(),
        success_stake: denial.refusal_stake.clone(),
        mixed_stake: denial.refusal_stake.clone(),
        failure_stake: denial.refusal_stake.clone(),
        strong_effect: WorldEffectDelta::default(),
        success_effect: WorldEffectDelta::default(),
        mixed_effect: WorldEffectDelta::default(),
        failure_effect: WorldEffectDelta::default(),
        bargains: denial.bargains.clone(),
    })
}

fn available_mutation_lanes(
    campaign: &Campaign,
    acting_actor: &crate::domain::ActorState,
) -> BTreeSet<AssessmentMutationLane> {
    let mut lanes = BTreeSet::from([
        AssessmentMutationLane::ActorConditions,
        AssessmentMutationLane::ActorCommitments,
        AssessmentMutationLane::ActorObservations,
        AssessmentMutationLane::ActorRelationshipUpdates,
    ]);
    let has_knowledge_target = campaign.actors.values().any(|target| {
        target.location_id == acting_actor.location_id
            && campaign.facts.values().any(|fact| {
                let accessible = if target.id == acting_actor.id {
                    fact.discoverable_at_location_ids
                        .contains(&acting_actor.location_id)
                } else {
                    acting_actor.knowledge.contains(&fact.statement)
                };
                accessible && !target.knowledge.contains(&fact.statement)
            })
    });
    if has_knowledge_target {
        lanes.insert(AssessmentMutationLane::ActorKnowledgeAdditions);
    }
    let has_destination = campaign
        .locations
        .get(&acting_actor.location_id)
        .is_some_and(|location| {
            location
                .routes
                .values()
                .any(|route| campaign.locations.contains_key(&route.destination_id))
        });
    if has_destination {
        lanes.insert(AssessmentMutationLane::ActorMoves);
    }
    if !campaign.clocks.is_empty() {
        lanes.insert(AssessmentMutationLane::ClockAdvances);
        lanes.insert(AssessmentMutationLane::ClockReductions);
    }
    if !campaign.institutions.is_empty() {
        lanes.insert(AssessmentMutationLane::InstitutionPostures);
    }
    lanes
}

fn constrain_mutation_scope_schema(
    schema: &mut serde_json::Value,
    available_lanes: &BTreeSet<AssessmentMutationLane>,
) -> Result<()> {
    for field in ["lanes", "required_success_lanes"] {
        let lane_items = schema
            .pointer_mut(&format!("/properties/{field}/items"))
            .ok_or_else(|| anyhow!("assessment mutation scope schema has no {field} items"))?;
        *lane_items = serde_json::json!({
            "type":"string",
            "enum":available_lanes,
        });
    }
    Ok(())
}

fn validate_mutation_scope(
    scope: &AssessmentMutationScope,
    available_lanes: &BTreeSet<AssessmentMutationLane>,
) -> Result<()> {
    if !scope.lanes.is_subset(available_lanes) {
        return Err(anyhow!(
            "assessment mutation scope selected a structurally unavailable lane"
        ));
    }
    if !scope.required_success_lanes.is_subset(&scope.lanes) {
        return Err(anyhow!(
            "assessment mutation scope required a success lane it did not select"
        ));
    }
    match (&scope.decision, &scope.denial) {
        (AssessmentScopeDecision::Assess, None) => {}
        (AssessmentScopeDecision::Deny, Some(denial)) => {
            if !scope.lanes.is_empty() || !scope.required_success_lanes.is_empty() {
                return Err(anyhow!("denied assessment scope selected a mutation lane"));
            }
            for (field, value) in [
                ("normalized_intent", denial.normalized_intent.as_str()),
                ("missing_permission", denial.missing_permission.as_str()),
                ("effect_ceiling", denial.effect_ceiling.as_str()),
                ("refusal_stake", denial.refusal_stake.as_str()),
            ] {
                let count = value.trim().chars().count();
                if count == 0 || count > 600 {
                    return Err(anyhow!(
                        "assessment denial {field} must contain 1 to 600 characters"
                    ));
                }
            }
            if denial.bargains.is_empty()
                || denial.bargains.len() > 4
                || denial
                    .bargains
                    .iter()
                    .any(|value| value.trim().is_empty() || value.chars().count() > 600)
            {
                return Err(anyhow!(
                    "assessment denial must contain one to four bounded bargains"
                ));
            }
        }
        (AssessmentScopeDecision::Assess, Some(_)) => {
            return Err(anyhow!("assessable mutation scope included a denial"));
        }
        (AssessmentScopeDecision::Deny, None) => {
            return Err(anyhow!("denied assessment scope omitted its denial"));
        }
    }
    Ok(())
}

fn constrain_effect_schema_to_scope(
    schema: &mut serde_json::Value,
    selected_lanes: &BTreeSet<AssessmentMutationLane>,
) -> Result<()> {
    for (lane, field) in [
        (AssessmentMutationLane::ActorConditions, "actor_conditions"),
        (
            AssessmentMutationLane::ActorCommitments,
            "actor_commitments",
        ),
        (
            AssessmentMutationLane::ActorKnowledgeAdditions,
            "actor_knowledge_additions",
        ),
        (
            AssessmentMutationLane::ActorObservations,
            "actor_observations",
        ),
        (
            AssessmentMutationLane::ActorRelationshipUpdates,
            "actor_relationship_updates",
        ),
        (AssessmentMutationLane::ActorMoves, "actor_moves"),
        (AssessmentMutationLane::ClockAdvances, "clock_advances"),
        (AssessmentMutationLane::ClockReductions, "clock_reductions"),
        (
            AssessmentMutationLane::InstitutionPostures,
            "institution_postures",
        ),
    ] {
        if !selected_lanes.contains(&lane) {
            remove_effect_lane(schema, field)?;
        }
    }
    Ok(())
}

fn require_success_scope(
    schema: &mut serde_json::Value,
    required_lanes: &BTreeSet<AssessmentMutationLane>,
) -> Result<()> {
    let base_effect = schema
        .pointer("/$defs/WorldEffectDelta")
        .cloned()
        .ok_or_else(|| anyhow!("assessment schema has no scoped world effect definition"))?;
    let mut required_success_effect = base_effect.clone();
    let required_properties = required_success_effect
        .get_mut("properties")
        .and_then(serde_json::Value::as_object_mut)
        .ok_or_else(|| anyhow!("scoped success effect has no properties"))?;
    for lane in required_lanes {
        let field = mutation_lane_field(*lane);
        let property = required_properties.get_mut(field).ok_or_else(|| {
            anyhow!("required success lane {field} was removed from the scoped schema")
        })?;
        if property.get("type").and_then(serde_json::Value::as_str) != Some("array") {
            return Err(anyhow!(
                "required success lane {field} is not a model mutation-entry array"
            ));
        }
        property["minItems"] = serde_json::json!(1);
    }

    let mut no_mutation_effect = base_effect;
    let no_mutation_properties = no_mutation_effect
        .get_mut("properties")
        .and_then(serde_json::Value::as_object_mut)
        .ok_or_else(|| anyhow!("scoped no-mutation effect has no properties"))?;
    for property in no_mutation_properties.values_mut() {
        if property.get("type").and_then(serde_json::Value::as_str) != Some("array") {
            return Err(anyhow!(
                "scoped no-mutation effect contains a non-array mutation lane"
            ));
        }
        property["maxItems"] = serde_json::json!(0);
    }

    let defs = schema
        .get_mut("$defs")
        .and_then(serde_json::Value::as_object_mut)
        .ok_or_else(|| anyhow!("assessment schema has no definitions"))?;
    defs.insert(
        "RequiredSuccessWorldEffectDelta".into(),
        required_success_effect,
    );
    defs.insert("NoMutationWorldEffectDelta".into(), no_mutation_effect);

    let proposal_properties = schema
        .get("properties")
        .and_then(serde_json::Value::as_object)
        .cloned()
        .ok_or_else(|| anyhow!("assessment schema has no proposal properties"))?;
    let required = proposal_properties
        .keys()
        .cloned()
        .map(serde_json::Value::String)
        .collect::<Vec<_>>();
    let mut admitted_properties = proposal_properties.clone();
    admitted_properties.insert(
        "admissible".into(),
        serde_json::json!({"type":"boolean","enum":[true]}),
    );
    admitted_properties.insert(
        "missing_permission".into(),
        serde_json::json!({"type":"null"}),
    );
    for field in ["strong_effect", "success_effect"] {
        admitted_properties.insert(
            field.into(),
            serde_json::json!({"$ref":"#/$defs/RequiredSuccessWorldEffectDelta"}),
        );
    }
    let admitted = serde_json::json!({
        "type":"object",
        "properties":admitted_properties,
        "required":required.clone(),
        "additionalProperties":false
    });

    let mut denied_properties = proposal_properties;
    denied_properties.insert(
        "admissible".into(),
        serde_json::json!({"type":"boolean","enum":[false]}),
    );
    denied_properties.insert(
        "missing_permission".into(),
        serde_json::json!({"type":"string","minLength":1}),
    );
    for field in [
        "strong_effect",
        "success_effect",
        "mixed_effect",
        "failure_effect",
    ] {
        denied_properties.insert(
            field.into(),
            serde_json::json!({"$ref":"#/$defs/NoMutationWorldEffectDelta"}),
        );
    }
    let denied = serde_json::json!({
        "type":"object",
        "properties":denied_properties,
        "required":required,
        "additionalProperties":false
    });

    let defs = schema
        .get("$defs")
        .cloned()
        .ok_or_else(|| anyhow!("assessment schema lost its definitions"))?;
    *schema = serde_json::json!({
        "type":"object",
        "properties":{
            "proposal":{"anyOf":[admitted,denied]}
        },
        "required":["proposal"],
        "additionalProperties":false,
        "$defs":defs
    });
    Ok(())
}

fn validate_required_success_lanes(
    proposal: &AssessmentProposal,
    scope: &AssessmentMutationScope,
) -> Result<()> {
    if !proposal.admissible {
        return Ok(());
    }
    for lane in &scope.required_success_lanes {
        if !effect_uses_lane(&proposal.strong_effect, *lane)
            || !effect_uses_lane(&proposal.success_effect, *lane)
        {
            return Err(anyhow!(
                "admissible assessment omitted required success mutation lane {}",
                mutation_lane_field(*lane)
            ));
        }
    }
    Ok(())
}

fn mutation_lane_field(lane: AssessmentMutationLane) -> &'static str {
    match lane {
        AssessmentMutationLane::ActorConditions => "actor_conditions",
        AssessmentMutationLane::ActorCommitments => "actor_commitments",
        AssessmentMutationLane::ActorKnowledgeAdditions => "actor_knowledge_additions",
        AssessmentMutationLane::ActorObservations => "actor_observations",
        AssessmentMutationLane::ActorRelationshipUpdates => "actor_relationship_updates",
        AssessmentMutationLane::ActorMoves => "actor_moves",
        AssessmentMutationLane::ClockAdvances => "clock_advances",
        AssessmentMutationLane::ClockReductions => "clock_reductions",
        AssessmentMutationLane::InstitutionPostures => "institution_postures",
    }
}

fn effect_uses_lane(effect: &WorldEffectDelta, lane: AssessmentMutationLane) -> bool {
    match lane {
        AssessmentMutationLane::ActorConditions => effect
            .actor_conditions
            .values()
            .any(|delta| !delta.add.is_empty() || !delta.remove.is_empty()),
        AssessmentMutationLane::ActorCommitments => {
            effect.actor_commitments.values().any(|delta| {
                !delta.goals_add.is_empty()
                    || !delta.goals_retire.is_empty()
                    || !delta.obligations_add.is_empty()
                    || !delta.obligations_retire.is_empty()
            })
        }
        AssessmentMutationLane::ActorKnowledgeAdditions => effect
            .actor_knowledge_additions
            .values()
            .any(|additions| !additions.is_empty()),
        AssessmentMutationLane::ActorObservations => effect
            .actor_observations
            .values()
            .any(|observations| !observations.is_empty()),
        AssessmentMutationLane::ActorRelationshipUpdates => effect
            .actor_relationship_updates
            .values()
            .any(|updates| !updates.is_empty()),
        AssessmentMutationLane::ActorMoves => !effect.actor_moves.is_empty(),
        AssessmentMutationLane::ClockAdvances => !effect.clock_advances.is_empty(),
        AssessmentMutationLane::ClockReductions => !effect.clock_reductions.is_empty(),
        AssessmentMutationLane::InstitutionPostures => !effect.institution_postures.is_empty(),
    }
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

fn bind_visible_effects(proposal: &mut AssessmentProposal) -> Result<()> {
    if proposal.strong_effect.actor_knowledge_additions
        != proposal.success_effect.actor_knowledge_additions
    {
        return Err(anyhow!(
            "strong and ordinary success must expose identical knowledge because they share one visible stake"
        ));
    }
    if proposal.strong_effect.actor_observations != proposal.success_effect.actor_observations {
        return Err(anyhow!(
            "strong and ordinary success must expose identical observations because they share one visible stake"
        ));
    }
    if proposal.strong_effect.actor_commitments != proposal.success_effect.actor_commitments {
        return Err(anyhow!(
            "strong and ordinary success must expose identical commitments because they share one visible stake"
        ));
    }
    append_visible_commitments(&mut proposal.success_stake, &proposal.success_effect);
    append_visible_commitments(&mut proposal.mixed_stake, &proposal.mixed_effect);
    append_visible_commitments(&mut proposal.failure_stake, &proposal.failure_effect);
    append_visible_findings(&mut proposal.success_stake, &proposal.success_effect);
    append_visible_findings(&mut proposal.mixed_stake, &proposal.mixed_effect);
    append_visible_findings(&mut proposal.failure_stake, &proposal.failure_effect);
    Ok(())
}

fn append_visible_commitments(stake: &mut String, effect: &WorldEffectDelta) {
    for delta in effect.actor_commitments.values() {
        for (label, values) in [
            ("Goal adopted", &delta.goals_add),
            ("Goal retired", &delta.goals_retire),
            ("Obligation adopted", &delta.obligations_add),
            ("Obligation retired", &delta.obligations_retire),
        ] {
            for value in values {
                if !stake.contains(value) {
                    if !stake.trim_end().is_empty() {
                        stake.push(' ');
                    }
                    stake.push_str(label);
                    stake.push_str(": ");
                    stake.push_str(value);
                }
            }
        }
    }
}

fn append_visible_findings(stake: &mut String, effect: &WorldEffectDelta) {
    for finding in effect
        .actor_knowledge_additions
        .values()
        .chain(effect.actor_observations.values())
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
    if !p.admissible
        && [
            &p.strong_effect,
            &p.success_effect,
            &p.mixed_effect,
            &p.failure_effect,
        ]
        .into_iter()
        .any(|effect| effect != &WorldEffectDelta::default())
    {
        return Err(anyhow!("inadmissible assessment proposed a world mutation"));
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
        .chain(effect.actor_commitments.keys())
        .chain(effect.actor_knowledge_additions.keys())
        .chain(effect.actor_observations.keys())
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
    for (actor_id, delta) in &effect.actor_commitments {
        let target = campaign
            .actors
            .get(actor_id)
            .ok_or_else(|| anyhow!("outcome commitment actor vanished"))?;
        let additions = delta
            .goals_add
            .iter()
            .chain(delta.obligations_add.iter())
            .collect::<Vec<_>>();
        let retirements = delta
            .goals_retire
            .iter()
            .chain(delta.obligations_retire.iter())
            .collect::<Vec<_>>();
        if additions.len().saturating_add(retirements.len()) > 4
            || additions.iter().chain(retirements.iter()).any(|value| {
                value.trim().is_empty()
                    || value.chars().count() > 600
                    || !stake.contains(value.as_str())
            })
            || !delta.goals_add.is_disjoint(&delta.goals_retire)
            || !delta.obligations_add.is_disjoint(&delta.obligations_retire)
            || delta
                .goals_add
                .iter()
                .any(|value| target.goals.contains(value))
            || delta
                .obligations_add
                .iter()
                .any(|value| target.obligations.contains(value))
            || delta
                .goals_retire
                .iter()
                .any(|value| !target.goals.contains(value))
            || delta
                .obligations_retire
                .iter()
                .any(|value| !target.obligations.contains(value))
        {
            return Err(anyhow!(
                "outcome commitment delta must add a new or retire an exact existing goal or obligation for a present actor and expose the exact commitment in its stake"
            ));
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
    for (actor_id, observations) in &effect.actor_observations {
        if actor_id != &acting_actor.id
            || observations.is_empty()
            || observations.len() > 2
            || observations.iter().any(|observation| {
                observation.trim().is_empty()
                    || observation.chars().count() > 500
                    || looks_like_identifier(observation)
                    || !stake.contains(observation)
                    || acting_actor.knowledge.contains(observation)
                    || campaign
                        .facts
                        .values()
                        .any(|fact| fact.statement == observation.as_str())
                    || effect
                        .actor_knowledge_additions
                        .values()
                        .any(|additions| additions.contains(observation))
            })
        {
            return Err(anyhow!(
                "outcome observations must contain one or two new player-readable branch-local findings for only the acting actor, visible verbatim in the stake and distinct from existing propositions"
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
                .values()
                .any(|route| &route.destination_id == destination)
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
    if let Some((id, posture)) = effect.institution_postures.iter().find(|(id, posture)| {
        !campaign.institutions.contains_key(*id)
            || posture.trim().is_empty()
            || posture.chars().count() > MAX_POSTURE_CHARS
    }) {
        return Err(anyhow!(
            "outcome institution posture must name an existing institution and contain one to {MAX_POSTURE_CHARS} characters: {id}={posture:?}"
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

    struct DenyingScopeModel {
        calls: AtomicUsize,
    }

    struct DenyingUnsupportedRecipientModel {
        calls: AtomicUsize,
    }

    #[async_trait]
    impl ModelPort for DenyingScopeModel {
        async fn run(&self, request: &ModelStageRequest) -> Result<String> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            assert_eq!(request.stage, "assessment_mutation_scope");
            assert!(request.lived_stream.contains("EXACT CURRENT AUTHORITY"));
            assert!(request.lived_stream.contains("present_actors"));
            Ok(serde_json::json!({
                "decision":"deny",
                "lanes":[],
                "required_success_lanes":[],
                "denial":{
                    "normalized_intent":"Take permanent command of an independent institution by declaration.",
                    "missing_permission":"The actor has no authority, leverage, custody, or extraordinary permission that can compel the institution's surrender.",
                    "effect_ceiling":"The actor may make the demand; the declaration cannot transfer institutional authority or custody.",
                    "refusal_stake":"No roll occurs and no institutional authority, custody, or obedience changes.",
                    "bargains":[
                        "Seek a specific local concession from a present official.",
                        "Acquire leverage or recognized authority that the institution must answer."
                    ]
                }
            })
            .to_string())
        }

        fn provider(&self) -> &'static str {
            "denying-scope-fixture"
        }
    }

    #[async_trait]
    impl ModelPort for DenyingUnsupportedRecipientModel {
        async fn run(&self, request: &ModelStageRequest) -> Result<String> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            assert_eq!(request.stage, "assessment_mutation_scope");
            assert!(request.lived_stream.contains(
                "A subject receiving, recognizing, learning, remembering, or becoming able to act on information is a canonical knowledge transition"
            ));
            assert!(request.lived_stream.contains("Harrow Station office"));
            Ok(serde_json::json!({
                "decision":"deny",
                "lanes":[],
                "required_success_lanes":[],
                "denial":{
                    "normalized_intent":"Deliver Vesa Orn's exact message to the Harrow Station office and establish its receipt.",
                    "missing_permission":"No exact foreground knowledge-recipient and channel mutation path is available for the remote Harrow Station office.",
                    "effect_ceiling":"Asha may speak into or deposit the message in the apparatus, but cannot establish institutional receipt or recognition.",
                    "refusal_stake":"No roll occurs and no remote or institutional subject acquires the message.",
                    "bargains":[
                        "Reach an exact present Harrow representative who can receive the message.",
                        "Establish an admitted bidirectional channel bound to an exact recipient."
                    ]
                }
            })
            .to_string())
        }

        fn provider(&self) -> &'static str {
            "unsupported-recipient-scope-fixture"
        }
    }

    #[tokio::test]
    async fn impossible_overreach_terminates_after_one_compact_admission_stage() {
        let model = Arc::new(DenyingScopeModel {
            calls: AtomicUsize::new(0),
        });
        let assessor = ActionAssessor::with_models(model.clone(), "fast", "capable");
        let dir = tempfile::tempdir().unwrap();
        let store = CampaignStore::open(dir.path().join("campaign.cc")).unwrap();
        let mut campaign = crate::resolution::tests::campaign(0, 1);
        let intent = ActionIntent {
            actor_id: "player".into(),
            description: "I declare myself the ruler of every institution.".into(),
            intended_effect: "Every institution surrenders its authority and obeys forever.".into(),
        };

        let (first, first_receipt) = assessor
            .assess_with_context_cached(&store, &campaign, intent.clone(), &[], None, &[])
            .await
            .unwrap();
        assert!(!first.admissible);
        assert!(first.missing_permission.is_some());
        assert_eq!(first.bargains.len(), 2);
        assert_eq!(first.strong_effect, WorldEffectDelta::default());
        assert_eq!(first.success_effect, WorldEffectDelta::default());
        assert_eq!(first_receipt.stage, "assessment_mutation_scope");
        assert_eq!(model.calls.load(Ordering::SeqCst), 1);
        assert!(
            store
                .keys(ASSESSMENT_PROPOSAL_CACHE_KIND)
                .unwrap()
                .is_empty()
        );
        assert_eq!(store.keys(ASSESSMENT_SCOPE_CACHE_KIND).unwrap().len(), 1);
        let stages = store
            .load_all::<ModelStageReceipt>("persona_stage_receipt.v1")
            .unwrap()
            .into_iter()
            .map(|receipt| receipt.stage)
            .collect::<BTreeSet<_>>();
        assert_eq!(stages, BTreeSet::from(["assessment_mutation_scope".into()]));

        campaign.revision += 1;
        let (second, second_receipt) = assessor
            .assess_with_context_cached(&store, &campaign, intent, &[], None, &[])
            .await
            .unwrap();
        assert_eq!(model.calls.load(Ordering::SeqCst), 1);
        assert!(!second.admissible);
        assert_eq!(second.revision, 1);
        assert_ne!(first.digest, second.digest);
        assert_eq!(second_receipt.validation_result, "valid_cache_hit");
    }

    #[tokio::test]
    async fn unsupported_remote_or_institution_recipient_is_denied_before_resolution() {
        let model = Arc::new(DenyingUnsupportedRecipientModel {
            calls: AtomicUsize::new(0),
        });
        let assessor = ActionAssessor::with_models(model.clone(), "fast", "capable");
        let mut campaign = crate::resolution::tests::campaign(0, 1);
        campaign.institutions.insert(
            "harrow-station-office".into(),
            crate::domain::InstitutionState {
                id: "harrow-station-office".into(),
                name: "Harrow Station office".into(),
                resources: vec![],
                goals: vec!["maintain the station's message service".into()],
                posture: "unverified".into(),
            },
        );
        let intent = ActionIntent {
            actor_id: "player".into(),
            description:
                "Use the marked message tube to recite Vesa Orn's exact message unchanged.".into(),
            intended_effect:
                "The Harrow Station office receives and recognizes Vesa Orn's exact words.".into(),
        };

        let (assessment, receipt) = assessor
            .assess_with_context(&campaign, intent, &[], None, &[])
            .await
            .unwrap();

        assert!(!assessment.admissible);
        assert!(
            assessment
                .missing_permission
                .as_deref()
                .is_some_and(|value| value.contains("knowledge-recipient"))
        );
        assert_eq!(assessment.strong_effect, WorldEffectDelta::default());
        assert_eq!(assessment.success_effect, WorldEffectDelta::default());
        assert_eq!(receipt.stage, "assessment_mutation_scope");
        assert_eq!(model.calls.load(Ordering::SeqCst), 1);
    }

    fn proposal_value_for_request(
        request: &ModelStageRequest,
        proposal: AssessmentProposal,
    ) -> Result<serde_json::Value> {
        let mut value = serde_json::to_value(proposal)?;
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
            let canonical: WorldEffectDelta = serde_json::from_value(value[field].clone())?;
            value[field] = encode_effect_entries(&canonical, &allowed_effect_fields);
        }
        Ok(serde_json::json!({"proposal":value}))
    }

    fn encode_effect_entries(
        effect: &WorldEffectDelta,
        allowed_fields: &BTreeSet<String>,
    ) -> serde_json::Value {
        let mut fields = serde_json::Map::new();
        if allowed_fields.contains("actor_conditions") {
            let mut entries = Vec::new();
            for (actor_id, delta) in &effect.actor_conditions {
                entries.extend(delta.add.iter().map(|condition| {
                    serde_json::json!({
                        "actor_id":actor_id,
                        "operation":"add",
                        "condition":condition
                    })
                }));
                entries.extend(delta.remove.iter().map(|condition| {
                    serde_json::json!({
                        "actor_id":actor_id,
                        "operation":"remove",
                        "condition":condition
                    })
                }));
            }
            fields.insert("actor_conditions".into(), serde_json::Value::Array(entries));
        }
        if allowed_fields.contains("actor_commitments") {
            let mut entries = Vec::new();
            for (actor_id, delta) in &effect.actor_commitments {
                entries.extend(delta.goals_add.iter().map(|description| {
                    serde_json::json!({
                        "actor_id":actor_id,
                        "operation":"add_goal",
                        "description":description
                    })
                }));
                entries.extend(delta.goals_retire.iter().map(|description| {
                    serde_json::json!({
                        "actor_id":actor_id,
                        "operation":"retire_goal",
                        "description":description
                    })
                }));
                entries.extend(delta.obligations_add.iter().map(|description| {
                    serde_json::json!({
                        "actor_id":actor_id,
                        "operation":"add_obligation",
                        "description":description
                    })
                }));
                entries.extend(delta.obligations_retire.iter().map(|description| {
                    serde_json::json!({
                        "actor_id":actor_id,
                        "operation":"retire_obligation",
                        "description":description
                    })
                }));
            }
            fields.insert(
                "actor_commitments".into(),
                serde_json::Value::Array(entries),
            );
        }
        if allowed_fields.contains("actor_knowledge_additions") {
            let entries = effect
                .actor_knowledge_additions
                .iter()
                .flat_map(|(actor_id, statements)| {
                    statements.iter().map(move
                        |statement| serde_json::json!({"actor_id":actor_id,"statement":statement}),
                    )
                })
                .collect();
            fields.insert(
                "actor_knowledge_additions".into(),
                serde_json::Value::Array(entries),
            );
        }
        if allowed_fields.contains("actor_observations") {
            let entries = effect
                .actor_observations
                .iter()
                .flat_map(|(actor_id, statements)| {
                    statements.iter().map(move |statement| {
                        serde_json::json!({"actor_id":actor_id,"statement":statement})
                    })
                })
                .collect();
            fields.insert(
                "actor_observations".into(),
                serde_json::Value::Array(entries),
            );
        }
        if allowed_fields.contains("actor_relationship_updates") {
            let entries = effect
                .actor_relationship_updates
                .iter()
                .flat_map(|(actor_id, targets)| {
                    targets.iter().map(move |(target_id, relationship)| {
                        serde_json::json!({
                            "actor_id":actor_id,
                            "target_id":target_id,
                            "relationship":relationship
                        })
                    })
                })
                .collect();
            fields.insert(
                "actor_relationship_updates".into(),
                serde_json::Value::Array(entries),
            );
        }
        if allowed_fields.contains("actor_moves") {
            let entries = effect
                .actor_moves
                .iter()
                .map(|(actor_id, destination_id)| {
                    serde_json::json!({"actor_id":actor_id,"destination_id":destination_id})
                })
                .collect();
            fields.insert("actor_moves".into(), serde_json::Value::Array(entries));
        }
        for (field, clocks) in [
            ("clock_advances", &effect.clock_advances),
            ("clock_reductions", &effect.clock_reductions),
        ] {
            if allowed_fields.contains(field) {
                let entries = clocks
                    .iter()
                    .map(|(clock_id, amount)| {
                        serde_json::json!({"clock_id":clock_id,"amount":amount})
                    })
                    .collect();
                fields.insert(field.into(), serde_json::Value::Array(entries));
            }
        }
        if allowed_fields.contains("institution_postures") {
            let entries = effect
                .institution_postures
                .iter()
                .map(|(institution_id, posture)| {
                    serde_json::json!({"institution_id":institution_id,"posture":posture})
                })
                .collect();
            fields.insert(
                "institution_postures".into(),
                serde_json::Value::Array(entries),
            );
        }
        serde_json::Value::Object(fields)
    }

    #[async_trait]
    impl ModelPort for DriftingAssessmentModel {
        async fn run(&self, request: &ModelStageRequest) -> Result<String> {
            if request.stage == "assessment_mutation_scope" {
                return Ok(
                    r#"{"decision":"assess","lanes":[],"required_success_lanes":[],"denial":null}"#
                        .into(),
                );
            }
            if request.stage == "assessment_effect_verifier" {
                return Ok(
                    r#"{"result":"match","mismatch_kind":null,"repair_guidance":null}"#.into(),
                );
            }
            assert!(
                request
                    .lived_stream
                    .contains("remaining lanes are an upper bound")
            );
            let call = self.calls.fetch_add(1, Ordering::SeqCst);
            let mut value = proposal_value_for_request(request, proposal("actor:player"))?;
            value["proposal"]["modifiers"][0]["value"] =
                serde_json::json!(if call == 0 { 2 } else { 6 });
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
        assert!(
            second_receipt.provider_attempts[0]
                .transport_features
                .iter()
                .any(|feature| feature.starts_with("source-mutation-scope:sha256:"))
        );
        assert!(
            second_receipt.provider_attempts[0]
                .transport_features
                .iter()
                .any(|feature| feature.starts_with("source-effect-verifier:sha256:"))
        );
        assert_eq!(store.keys(ASSESSMENT_PROPOSAL_CACHE_KIND).unwrap().len(), 1);
        assert_eq!(store.keys(ASSESSMENT_SCOPE_CACHE_KIND).unwrap().len(), 1);
        assert_eq!(store.keys("persona_stage_receipt.v1").unwrap().len(), 3);
    }

    struct ConcretizingObservationModel {
        assessment_calls: AtomicUsize,
        verifier_calls: AtomicUsize,
    }

    struct OmittedRecipientEffectModel {
        assessment_calls: AtomicUsize,
        verifier_calls: AtomicUsize,
    }

    #[async_trait]
    impl ModelPort for OmittedRecipientEffectModel {
        async fn run(&self, request: &ModelStageRequest) -> Result<String> {
            match request.stage.as_str() {
                "assessment_mutation_scope" => Ok(serde_json::json!({
                    "decision":"assess",
                    "lanes":[],
                    "required_success_lanes":[],
                    "denial":null
                })
                .to_string()),
                "action_assessment" => {
                    self.assessment_calls.fetch_add(1, Ordering::SeqCst);
                    assert!(request.lived_stream.contains(
                        "Never promise remote, institutional, population, place, office, or absent-actor receipt"
                    ));
                    let mut value = proposal_value_for_request(request, proposal("actor:player"))?;
                    value["proposal"]["normalized_intent"] =
                        serde_json::json!("send Vesa Orn's exact message to Harrow Station");
                    value["proposal"]["effect_ceiling"] = serde_json::json!(
                        "The station office may receive and recognize the exact message."
                    );
                    value["proposal"]["success_stake"] = serde_json::json!(
                        "The Harrow Station office receives and recognizes Vesa Orn's exact words."
                    );
                    value["proposal"]["mixed_stake"] = serde_json::json!(
                        "The office receives only part of the message and cannot authenticate it."
                    );
                    value["proposal"]["failure_stake"] =
                        serde_json::json!("The message does not reach the office.");
                    Ok(serde_json::to_string(&value)?)
                }
                "assessment_effect_verifier" => {
                    self.verifier_calls.fetch_add(1, Ordering::SeqCst);
                    assert!(request.lived_stream.contains(
                        "If a stake claims such a transition without an exact typed knowledge mutation for that exact recipient"
                    ));
                    Ok(serde_json::json!({
                        "result":"mismatch",
                        "mismatch_kind":"effect_omission",
                        "repair_guidance":"The stake claims Harrow Station learned the message without an exact recipient knowledge mutation; deny the intended delivery rather than narrating receipt."
                    })
                    .to_string())
                }
                stage => Err(anyhow!("unexpected fixture stage {stage}")),
            }
        }

        fn provider(&self) -> &'static str {
            "omitted-recipient-effect-fixture"
        }
    }

    #[async_trait]
    impl ModelPort for ConcretizingObservationModel {
        async fn run(&self, request: &ModelStageRequest) -> Result<String> {
            match request.stage.as_str() {
                "assessment_mutation_scope" => Ok(serde_json::json!({
                    "decision":"assess",
                    "lanes":["actor_observations"],
                    "required_success_lanes":["actor_observations"],
                    "denial":null
                })
                .to_string()),
                "action_assessment" => {
                    self.assessment_calls.fetch_add(1, Ordering::SeqCst);
                    assert!(request.lived_stream.contains(
                        "An actor_observations statement is itself the canonical branch-local proposition"
                    ));
                    let correction = request
                        .lived_stream
                        .contains("SEMANTIC EFFECT VERIFIER REJECTED");
                    if correction {
                        assert!(
                            request.lived_stream.contains(
                                "Replace the inquiry-status observation with one concrete"
                            )
                        );
                    }
                    let statement = if correction {
                        "The station is visibly unoccupied, but a weatherproof platform dispatch box is powered and accepts written messages for the Harrow watch."
                    } else {
                        "The inspection establishes whether the station is occupied and identifies a usable message channel if one exists."
                    };
                    let mut value = proposal_value_for_request(request, proposal("actor:player"))?;
                    value["proposal"]["normalized_intent"] =
                        serde_json::json!("inspect the station for a message channel");
                    value["proposal"]["effect_ceiling"] = serde_json::json!(
                        "Direct local evidence may establish station occupancy and one usable message channel."
                    );
                    value["proposal"]["success_stake"] =
                        serde_json::json!("The inspection answers the bounded local question.");
                    value["proposal"]["mixed_stake"] =
                        serde_json::json!("Only part of the station can be assessed.");
                    value["proposal"]["failure_stake"] =
                        serde_json::json!("The station remains unreadable from the platform.");
                    for effect in ["strong_effect", "success_effect"] {
                        value["proposal"][effect]["actor_observations"] = serde_json::json!([{
                            "actor_id":"player",
                            "statement":statement
                        }]);
                    }
                    Ok(serde_json::to_string(&value)?)
                }
                "assessment_effect_verifier" => {
                    self.verifier_calls.fetch_add(1, Ordering::SeqCst);
                    assert!(
                        request
                            .lived_stream
                            .contains("must therefore be a concrete, truth-apt proposition")
                    );
                    let unresolved = request
                        .lived_stream
                        .contains("identifies a usable message channel if one exists");
                    Ok(if unresolved {
                        serde_json::json!({
                            "result":"mismatch",
                            "mismatch_kind":"invented_outcome",
                            "repair_guidance":"Replace the inquiry-status observation with one concrete, truth-apt local finding that resolves occupancy or the message channel within the previewed ceiling."
                        })
                    } else {
                        serde_json::json!({
                            "result":"match",
                            "mismatch_kind":null,
                            "repair_guidance":null
                        })
                    }
                    .to_string())
                }
                stage => Err(anyhow!("unexpected fixture stage {stage}")),
            }
        }

        fn provider(&self) -> &'static str {
            "concretizing-observation-fixture"
        }
    }

    #[tokio::test]
    async fn semantic_verifier_rejects_inquiry_status_as_canonical_observation() {
        let model = Arc::new(ConcretizingObservationModel {
            assessment_calls: AtomicUsize::new(0),
            verifier_calls: AtomicUsize::new(0),
        });
        let assessor = ActionAssessor::with_models(model.clone(), "flash", "capable");
        let campaign = crate::resolution::tests::campaign(0, 1);
        let intent = ActionIntent {
            actor_id: "player".into(),
            description: "inspect the station platform and signal equipment".into(),
            intended_effect: "learn whether anyone is present and find a message channel".into(),
        };

        let (assessment, _) = assessor
            .assess_with_context(&campaign, intent, &[], None, &[])
            .await
            .unwrap();

        assert_eq!(model.assessment_calls.load(Ordering::SeqCst), 2);
        assert_eq!(model.verifier_calls.load(Ordering::SeqCst), 2);
        assert_eq!(
            assessment.success_effect.actor_observations["player"],
            BTreeSet::from(["The station is visibly unoccupied, but a weatherproof platform dispatch box is powered and accepts written messages for the Harrow watch.".into()])
        );
        assert!(!assessment.success_stake.contains("if one exists"));
    }

    #[tokio::test]
    async fn semantic_verifier_aborts_narrated_receipt_without_a_recipient_mutation() {
        let model = Arc::new(OmittedRecipientEffectModel {
            assessment_calls: AtomicUsize::new(0),
            verifier_calls: AtomicUsize::new(0),
        });
        let assessor = ActionAssessor::with_models(model.clone(), "flash", "capable");
        let campaign = crate::resolution::tests::campaign(0, 1);
        let intent = ActionIntent {
            actor_id: "player".into(),
            description: "Recite Vesa Orn's exact message into the station tube.".into(),
            intended_effect: "The Harrow Station office receives and recognizes it.".into(),
        };

        let error = assessor
            .assess_with_context(&campaign, intent, &[], None, &[])
            .await
            .unwrap_err()
            .to_string();

        assert!(error.contains("failed semantic effect verification after one correction"));
        assert!(error.contains("without an exact recipient knowledge mutation"));
        assert_eq!(model.assessment_calls.load(Ordering::SeqCst), 2);
        assert_eq!(model.verifier_calls.load(Ordering::SeqCst), 2);
    }

    struct CorrectingEffectModel {
        assessment_calls: AtomicUsize,
        verifier_calls: AtomicUsize,
        corrects: bool,
    }

    #[async_trait]
    impl ModelPort for CorrectingEffectModel {
        async fn run(&self, request: &ModelStageRequest) -> Result<String> {
            match request.stage.as_str() {
                "assessment_mutation_scope" => Ok(serde_json::json!({
                    "decision":"assess",
                    "lanes":["actor_knowledge_additions","actor_relationship_updates"],
                    "required_success_lanes":["actor_relationship_updates"],
                    "denial":null
                })
                .to_string()),
                "action_assessment" => {
                    self.assessment_calls.fetch_add(1, Ordering::SeqCst);
                    let correction = request
                        .lived_stream
                        .contains("SEMANTIC EFFECT VERIFIER REJECTED");
                    if correction {
                        assert!(
                            request
                                .lived_stream
                                .contains("Remove the ration knowledge transfer")
                        );
                    }
                    let mut value = proposal_value_for_request(request, proposal("actor:target"))?;
                    value["proposal"]["normalized_intent"] =
                        serde_json::json!("honor the target's consent boundary");
                    value["proposal"]["effect_ceiling"] = serde_json::json!(
                        "The target may trust the player more while retaining control of their identity."
                    );
                    value["proposal"]["success_stake"] =
                        serde_json::json!("The target's trust deepens.");
                    value["proposal"]["mixed_stake"] =
                        serde_json::json!("The target remains cautious.");
                    value["proposal"]["failure_stake"] =
                        serde_json::json!("The promise sounds hollow.");
                    for effect in ["strong_effect", "success_effect"] {
                        value["proposal"][effect]["actor_relationship_updates"] = serde_json::json!([{
                            "actor_id":"target",
                            "target_id":"player",
                            "relationship":"trusts the player to respect their consent boundary"
                        }]);
                    }
                    if !correction || !self.corrects {
                        for effect in ["strong_effect", "success_effect"] {
                            value["proposal"][effect]["actor_knowledge_additions"] = serde_json::json!([{
                                "actor_id":"target",
                                "statement":"Rations are restricted."
                            }]);
                        }
                    }
                    Ok(serde_json::to_string(&value)?)
                }
                "assessment_effect_verifier" => {
                    assert!(request.lived_stream.contains("\"name\":\"Target\""));
                    self.verifier_calls.fetch_add(1, Ordering::SeqCst);
                    let mismatch = request.lived_stream.contains("Rations are restricted.");
                    Ok(if mismatch {
                        serde_json::json!({
                            "result":"mismatch",
                            "mismatch_kind":"unrelated_mutation",
                            "repair_guidance":"Remove the ration knowledge transfer; the trust promise does not communicate ration policy."
                        })
                    } else {
                        serde_json::json!({
                            "result":"match",
                            "mismatch_kind":null,
                            "repair_guidance":null
                        })
                    }
                    .to_string())
                }
                stage => Err(anyhow!("unexpected fixture stage {stage}")),
            }
        }

        fn provider(&self) -> &'static str {
            "correcting-effect-fixture"
        }
    }

    #[tokio::test]
    async fn semantic_verifier_removes_a_legal_but_unrelated_effect_before_caching() {
        let model = Arc::new(CorrectingEffectModel {
            assessment_calls: AtomicUsize::new(0),
            verifier_calls: AtomicUsize::new(0),
            corrects: true,
        });
        let assessor = ActionAssessor::with_models(model.clone(), "flash", "capable");
        let dir = tempfile::tempdir().unwrap();
        let store = CampaignStore::open(dir.path().join("campaign.cc")).unwrap();
        let mut campaign = crate::resolution::tests::campaign(0, 1);
        let acting = campaign.actors["player"].clone();
        campaign
            .actors
            .get_mut("player")
            .unwrap()
            .knowledge
            .insert("Rations are restricted.".into());
        let mut target = acting.clone();
        target.id = "target".into();
        target.name = "Target".into();
        target.knowledge.clear();
        campaign.actors.insert(target.id.clone(), target);
        campaign.facts.insert(
            "ration-policy".into(),
            crate::domain::WorldFact {
                id: "ration-policy".into(),
                statement: "Rations are restricted.".into(),
                scope: crate::domain::FactScope::BranchLocal,
                evidence_receipt_ids: vec![],
                discoverable_at_location_ids: BTreeSet::new(),
            },
        );

        let (assessment, _) =
            assessor
                .assess_with_context_cached(
                    &store,
                    &campaign,
                    ActionIntent {
                        actor_id: "player".into(),
                        description:
                            "Promise not to record the target's route role without consent.".into(),
                        intended_effect:
                            "The target trusts the player more while retaining identity control."
                                .into(),
                    },
                    &[],
                    None,
                    &[],
                )
                .await
                .unwrap();

        assert_eq!(model.assessment_calls.load(Ordering::SeqCst), 2);
        assert_eq!(model.verifier_calls.load(Ordering::SeqCst), 2);
        assert!(
            assessment
                .success_effect
                .actor_knowledge_additions
                .is_empty()
        );
        assert_eq!(
            assessment.success_effect.actor_relationship_updates["target"]["player"],
            "trusts the player to respect their consent boundary"
        );
        let stage_receipts = store
            .load_all::<ModelStageReceipt>("persona_stage_receipt.v1")
            .unwrap();
        assert_eq!(stage_receipts.len(), 5);
        assert_eq!(
            stage_receipts
                .iter()
                .filter(|receipt| receipt.validation_result == "semantic_invalid")
                .count(),
            2
        );
        assert_eq!(
            stage_receipts
                .iter()
                .filter(|receipt| receipt.validation_result == "valid")
                .count(),
            3
        );
        assert_eq!(
            stage_receipts
                .iter()
                .filter(|receipt| receipt.stage == "assessment_mutation_scope")
                .count(),
            1
        );
        assert_eq!(
            stage_receipts
                .iter()
                .filter(|receipt| receipt.stage == "action_assessment")
                .count(),
            2
        );
        assert_eq!(
            stage_receipts
                .iter()
                .filter(|receipt| receipt.stage == "assessment_effect_verifier")
                .count(),
            2
        );
        assert_eq!(store.keys(ASSESSMENT_PROPOSAL_CACHE_KIND).unwrap().len(), 1);
        assert_eq!(store.keys(ASSESSMENT_SCOPE_CACHE_KIND).unwrap().len(), 1);
    }

    #[tokio::test]
    async fn repeated_semantic_mismatch_aborts_without_a_cache_entry() {
        let model = Arc::new(CorrectingEffectModel {
            assessment_calls: AtomicUsize::new(0),
            verifier_calls: AtomicUsize::new(0),
            corrects: false,
        });
        let assessor = ActionAssessor::with_models(model.clone(), "flash", "capable");
        let dir = tempfile::tempdir().unwrap();
        let store = CampaignStore::open(dir.path().join("campaign.cc")).unwrap();
        let mut campaign = crate::resolution::tests::campaign(0, 1);
        let acting = campaign.actors["player"].clone();
        campaign
            .actors
            .get_mut("player")
            .unwrap()
            .knowledge
            .insert("Rations are restricted.".into());
        let mut target = acting;
        target.id = "target".into();
        target.name = "Target".into();
        target.knowledge.clear();
        campaign.actors.insert(target.id.clone(), target);
        campaign.facts.insert(
            "ration-policy".into(),
            crate::domain::WorldFact {
                id: "ration-policy".into(),
                statement: "Rations are restricted.".into(),
                scope: crate::domain::FactScope::BranchLocal,
                evidence_receipt_ids: vec![],
                discoverable_at_location_ids: BTreeSet::new(),
            },
        );

        let error = assessor
            .assess_with_context_cached(
                &store,
                &campaign,
                ActionIntent {
                    actor_id: "player".into(),
                    description: "Promise not to record the target without consent.".into(),
                    intended_effect: "The target trusts the player more.".into(),
                },
                &[],
                None,
                &[],
            )
            .await
            .unwrap_err()
            .to_string();

        assert!(error.contains("failed semantic effect verification after one correction"));
        assert_eq!(model.assessment_calls.load(Ordering::SeqCst), 2);
        assert_eq!(model.verifier_calls.load(Ordering::SeqCst), 2);
        assert!(
            store
                .keys(ASSESSMENT_PROPOSAL_CACHE_KIND)
                .unwrap()
                .is_empty()
        );
        let receipts = store
            .load_all::<ModelStageReceipt>("persona_stage_receipt.v1")
            .unwrap();
        assert_eq!(receipts.len(), 5);
        assert_eq!(
            receipts
                .iter()
                .filter(|receipt| receipt.validation_result == "semantic_invalid")
                .count(),
            4
        );
        assert_eq!(
            receipts
                .iter()
                .filter(|receipt| receipt.stage == "assessment_mutation_scope")
                .count(),
            1
        );
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
    fn mutation_scope_removes_every_causally_unselected_effect_lane() {
        let campaign = crate::resolution::tests::campaign(0, 1);
        let acting = &campaign.actors["player"];
        let mut schema = serde_json::to_value(schema_for!(AssessmentProposal)).unwrap();
        constrain_assessment_schema(
            &mut schema,
            &BTreeSet::from(["actor:player".into()]),
            &campaign,
            acting,
        )
        .unwrap();
        project_effect_schema_to_mutation_entries(&mut schema, &campaign).unwrap();
        constrain_effect_schema_to_scope(
            &mut schema,
            &BTreeSet::from([AssessmentMutationLane::ActorRelationshipUpdates]),
        )
        .unwrap();

        let fields = schema
            .pointer("/$defs/WorldEffectDelta/properties")
            .and_then(serde_json::Value::as_object)
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect::<BTreeSet<_>>();
        assert_eq!(fields, BTreeSet::from(["actor_relationship_updates"]));
    }

    #[test]
    fn mutation_scope_schema_inlines_the_provider_enum_without_ref_siblings() {
        let mut schema = serde_json::to_value(schema_for!(AssessmentMutationScope)).unwrap();
        constrain_mutation_scope_schema(
            &mut schema,
            &BTreeSet::from([
                AssessmentMutationLane::ActorConditions,
                AssessmentMutationLane::ActorRelationshipUpdates,
            ]),
        )
        .unwrap();

        for field in ["lanes", "required_success_lanes"] {
            let items = schema
                .pointer(&format!("/properties/{field}/items"))
                .unwrap();
            assert_eq!(items["type"], "string");
            assert!(items.get("$ref").is_none());
            assert_eq!(
                items["enum"],
                serde_json::json!(["actor_conditions", "actor_relationship_updates"])
            );
        }
    }

    #[test]
    fn required_success_lane_is_structural_and_locally_rechecked() {
        let campaign = crate::resolution::tests::campaign(0, 1);
        let acting = &campaign.actors["player"];
        let required = BTreeSet::from([AssessmentMutationLane::ActorRelationshipUpdates]);
        let mut schema = serde_json::to_value(schema_for!(AssessmentProposal)).unwrap();
        constrain_assessment_schema(
            &mut schema,
            &BTreeSet::from(["actor:player".into()]),
            &campaign,
            acting,
        )
        .unwrap();
        project_effect_schema_to_mutation_entries(&mut schema, &campaign).unwrap();
        constrain_effect_schema_to_scope(&mut schema, &required).unwrap();
        require_success_scope(&mut schema, &required).unwrap();
        assert!(schema.get("anyOf").is_none());
        assert_eq!(
            schema["properties"]["proposal"]["anyOf"][0]["properties"]["success_effect"]["$ref"],
            "#/$defs/RequiredSuccessWorldEffectDelta"
        );
        assert_eq!(
            schema["properties"]["proposal"]["anyOf"][0]["properties"]["admissible"]["enum"],
            serde_json::json!([true])
        );
        assert_eq!(
            schema["properties"]["proposal"]["anyOf"][1]["properties"]["admissible"]["enum"],
            serde_json::json!([false])
        );
        assert_eq!(
            schema["$defs"]["RequiredSuccessWorldEffectDelta"]["properties"]["actor_relationship_updates"]
                ["minItems"],
            1
        );
        assert_eq!(
            schema["$defs"]["NoMutationWorldEffectDelta"]["properties"]["actor_relationship_updates"]
                ["maxItems"],
            0
        );

        let scope = AssessmentMutationScope {
            decision: AssessmentScopeDecision::Assess,
            lanes: required.clone(),
            required_success_lanes: required,
            denial: None,
        };
        let mut candidate = proposal("actor:player");
        let error = validate_required_success_lanes(&candidate, &scope)
            .unwrap_err()
            .to_string();
        assert!(error.contains("actor_relationship_updates"));
        for effect in [&mut candidate.strong_effect, &mut candidate.success_effect] {
            effect.actor_relationship_updates.insert(
                "player".into(),
                BTreeMap::from([("player".into(), "trust deepens".into())]),
            );
        }
        validate_required_success_lanes(&candidate, &scope).unwrap();
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
        project_effect_schema_to_mutation_entries(&mut schema, &campaign).unwrap();
        let effect = schema
            .pointer("/$defs/WorldEffectDelta/properties")
            .unwrap();
        let actor_targets = effect["actor_conditions"]["items"]["properties"]["actor_id"]["enum"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(serde_json::Value::as_str)
            .collect::<BTreeSet<_>>();
        assert_eq!(actor_targets, BTreeSet::from(["clinic-director", "player"]));
        assert_eq!(
            effect["actor_observations"]["items"]["properties"]["actor_id"]["enum"],
            serde_json::json!(["player"])
        );
        let observation_validator =
            jsonschema::validator_for(&effect["actor_observations"]).unwrap();
        assert!(observation_validator.is_valid(&serde_json::json!([{
            "actor_id":"player",
            "statement":"The left coupling carries the highest current thermal stress."
        }])));
        assert!(!observation_validator.is_valid(&serde_json::json!([{
            "actor_id":"clinic-director",
            "statement":"The left coupling carries the highest current thermal stress."
        }])));
        let commitments = &effect["actor_commitments"];
        let commitment_validator = jsonschema::validator_for(commitments).unwrap();
        assert!(commitment_validator.is_valid(&serde_json::json!([{
            "actor_id":"clinic-director",
            "operation":"add_obligation",
            "description":"state concrete supervised-inspection terms"
        }])));
        assert!(!commitment_validator.is_valid(&serde_json::json!([{
            "actor_id":"remote-commander",
            "operation":"add_obligation",
            "description":"state concrete supervised-inspection terms"
        }])));
        let relationship_targets =
            effect["actor_relationship_updates"]["items"]["properties"]["target_id"]["enum"]
                .as_array()
                .unwrap()
                .iter()
                .filter_map(serde_json::Value::as_str)
                .collect::<BTreeSet<_>>();
        assert!(relationship_targets.contains("clinic-director"));
        assert!(relationship_targets.contains("player"));
        assert!(!relationship_targets.contains("remote-commander"));
        assert_eq!(
            effect["actor_moves"]["items"]["properties"]["actor_id"]["enum"],
            serde_json::json!(["player"])
        );
        assert_eq!(
            effect["actor_moves"]["items"]["properties"]["destination_id"]["enum"],
            serde_json::json!(["adjacent"])
        );
        assert_eq!(
            effect["clock_advances"]["items"]["properties"]["amount"]["minimum"],
            1
        );
        assert_eq!(
            effect["clock_reductions"]["items"]["properties"]["amount"]["minimum"],
            1
        );
        assert_eq!(
            effect["institution_postures"]["items"]["properties"]["posture"]["minLength"],
            1
        );
        assert_eq!(
            effect["institution_postures"]["items"]["properties"]["posture"]["maxLength"],
            MAX_POSTURE_CHARS
        );
        assert_eq!(
            effect["actor_relationship_updates"]["items"]["properties"]["relationship"]["minLength"],
            1
        );
        assert_eq!(
            effect["actor_conditions"]["items"]["properties"]["condition"]["minLength"],
            1
        );
        let effect_text = serde_json::to_string(effect).unwrap();
        for erased_dynamic_map_keyword in ["propertyNames", "minProperties", "maxProperties"] {
            assert!(!effect_text.contains(erased_dynamic_map_keyword));
        }
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
        project_effect_schema_to_mutation_entries(&mut schema, &campaign).unwrap();
        let effect = schema.pointer("/$defs/WorldEffectDelta").unwrap();
        let properties = effect["properties"].as_object().unwrap();

        assert!(properties.contains_key("actor_conditions"));
        assert!(properties.contains_key("actor_commitments"));
        assert!(properties.contains_key("actor_observations"));
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
    fn actor_commitment_entries_are_typed_and_contradictions_fail_closed() {
        let decoded = decode_effect_entries(&serde_json::json!({
            "actor_commitments":[
                {
                    "actor_id":"target",
                    "operation":"add_obligation",
                    "description":"state concrete supervised-inspection terms"
                },
                {
                    "actor_id":"target",
                    "operation":"add_goal",
                    "description":"review the seizure ledger"
                }
            ]
        }))
        .unwrap();
        assert!(
            decoded.actor_commitments["target"]
                .obligations_add
                .contains("state concrete supervised-inspection terms")
        );
        assert!(
            decoded.actor_commitments["target"]
                .goals_add
                .contains("review the seizure ledger")
        );

        let error = decode_effect_entries(&serde_json::json!({
            "actor_commitments":[
                {
                    "actor_id":"target",
                    "operation":"add_obligation",
                    "description":"state concrete supervised-inspection terms"
                },
                {
                    "actor_id":"target",
                    "operation":"retire_obligation",
                    "description":"state concrete supervised-inspection terms"
                }
            ]
        }))
        .unwrap_err()
        .to_string();
        assert!(error.contains("both adds and retires"));
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
        project_effect_schema_to_mutation_entries(&mut schema, &campaign).unwrap();
        let knowledge = schema
            .pointer("/$defs/WorldEffectDelta/properties/actor_knowledge_additions")
            .unwrap();
        let validator = jsonschema::validator_for(knowledge).unwrap();

        assert!(validator.is_valid(&serde_json::json!([{
            "actor_id":"player",
            "statement":"The emergency cache is behind the north clinic wall."
        }])));
        assert!(validator.is_valid(&serde_json::json!([{
            "actor_id":"clinic-director",
            "statement":"The clinic director already knows the convoy is delayed."
        }])));
        assert!(!validator.is_valid(&serde_json::json!([{
            "actor_id":"clinic-director",
            "statement":"The emergency cache is behind the north clinic wall."
        }])));
        assert!(!validator.is_valid(&serde_json::json!([{
            "actor_id":"player",
            "statement":"The clinic director already knows the convoy is delayed."
        }])));
    }

    #[test]
    fn bounded_observation_must_be_new_actor_local_and_visible() {
        let mut campaign = crate::resolution::tests::campaign(0, 1);
        let acting = campaign.actors["player"].clone();
        let statement = "The regulator's left coupling carries the highest current thermal stress.";
        let stake = format!("Observed finding: {statement}");
        let effect = decode_effect_entries(&serde_json::json!({
            "actor_observations":[{
                "actor_id":acting.id,
                "statement":statement
            }]
        }))
        .unwrap();

        validate_effect(&campaign, &acting, &effect, &stake).unwrap();

        campaign.facts.insert(
            "already-known".into(),
            crate::domain::WorldFact {
                id: "already-known".into(),
                statement: statement.into(),
                scope: crate::domain::FactScope::BranchLocal,
                evidence_receipt_ids: vec![],
                discoverable_at_location_ids: BTreeSet::from([acting.location_id.clone()]),
            },
        );
        assert!(
            validate_effect(&campaign, &acting, &effect, &stake)
                .unwrap_err()
                .to_string()
                .contains("branch-local findings")
        );
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
    fn assessment_effect_rejects_an_oversized_posture() {
        let campaign = crate::resolution::tests::campaign(2, 1);
        let acting = campaign.actors["player"].clone();
        let institution_id = campaign.institutions.keys().next().unwrap().clone();
        let effect = WorldEffectDelta {
            institution_postures: std::collections::BTreeMap::from([(
                institution_id,
                "x".repeat(MAX_POSTURE_CHARS + 1),
            )]),
            ..WorldEffectDelta::default()
        };

        let error = validate_effect(&campaign, &acting, &effect, "nothing changes").unwrap_err();
        assert!(error.to_string().contains("one to 460 characters"));
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
    fn inadmissible_assessment_cannot_smuggle_a_world_mutation() {
        let mut value = proposal("equipment:key");
        value.admissible = false;
        value.missing_permission = Some("No admitted route reaches that destination.".into());
        value
            .success_effect
            .actor_moves
            .insert("player".into(), "elsewhere".into());

        let error = validate_proposal(&value, &BTreeSet::from(["equipment:key".into()]))
            .unwrap_err()
            .to_string();
        assert!(error.contains("inadmissible assessment proposed a world mutation"));
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

        bind_visible_effects(&mut value).unwrap();

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
