use crate::{
    domain::{
        Campaign, CellActionProposal, StrategicActivityKind, StrategicActivityOutcome,
        StrategicCellEffect, StrategicOutcomeBand, StrategicOutcomeEffect, StrategicTickPlan,
    },
    model::{MODEL_FAST, ModelPort, ModelStageOutput, ModelStageRequest, run_validated_stage},
    resolution::{cell_action_digest, effective_member_knowledge, subject_state_references},
};
use anyhow::{Result, anyhow};
use schemars::{JsonSchema, schema_for};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};
use tokio::{sync::Semaphore, task::JoinSet};

const OUTCOME_PROPOSAL_OUTPUT_CONTRACT: &str = r#"The top-level object has exactly one field named outcomes—never action_resolutions, results, or resolutions. outcomes is an array with one item per supplied action_digest. Every item requires action_digest, band (success, mixed, or failure), effect_kind, and supporting_state_references. Fields are conditionally required, not optional suggestions: no_material_change requires reason; resource_created and resource_consumed require owner_subject_id and resource; resource_transferred requires owner_subject_id, other_subject_id, and resource; gestalt_pressure requires owner_subject_id plus both pressure arrays; agency_relation_shift requires relation_id and strength_delta; member_memory requires member_id and memory; member_obligation requires member_id and obligation; member_relationship requires member_id, other_subject_id, and relationship_description; knowledge_learned requires owner_subject_id and fact_id; knowledge_communicated requires owner_subject_id, recipient_subject_ids, and fact_id. Every scalar field not named for the chosen effect_kind is omitted or null; irrelevant arrays are omitted, null, or empty and normalize to empty before semantic binding. A non-neutral irrelevant field is invalid. For gestalt_pressure, pressure_additions and pressure_resolutions must be arrays of plain strings, never null or objects. For knowledge_communicated, recipient_subject_ids is an array of exact supplied recipients, never null. no_material_change uses an empty supporting_state_references array; every material effect cites the smallest causally decisive set of one to eight values copied literally and only from that action's allowed_state_references. A source_subject_id, target_subject_id, target_state subject_id, member_state_owner_id, owner_subject_id, other_subject_id, member_id, relation_id, fact_id, or recipient_subject_id is not a supporting state reference unless that exact string also appears in allowed_state_references. For member_relationship and knowledge_communicated, the target field already identifies the target; do not repeat it as provenance. Do not emit summary; Ghostlight derives it from the validated typed effect. When resource_created is admissible and concrete capability-backed making or repair establishes a durable result, owner_subject_id copies one exact resource_creation_owner_ids value: use the source for portable stock or equipment it retains, and use an exact activity location for a durable installed, placed, or repaired feature there. The resource field names the resulting object, stock, repair, or usable arrangement. It never restates an action or an unnamed recipient's response. Example no-op shape: {"outcomes":[{"action_digest":"sha256:<copy an exact supplied digest>","band":"mixed","effect_kind":"no_material_change","supporting_state_references":[],"reason":"No durable state changed."}]}"#;
const OUTCOME_VERIFIER_OUTPUT_CONTRACT: &str = r#"The top-level object has exactly one field named verdicts—never verifications, outcomes, results, or resolutions. verdicts is an array with one item per supplied action_digest. Every item has exactly action_digest, result, and repair_guidance. Example: {"verdicts":[{"action_digest":"sha256:<copy exact supplied digest>","result":"match","repair_guidance":null}]}"#;

#[derive(Clone, Debug, Serialize)]
struct OutcomeContext {
    world_revision: u64,
    resolution_epoch: u64,
    actions: Vec<ActionOutcomeContext>,
}

#[derive(Clone, Debug, Serialize)]
struct ActionOutcomeContext {
    action_digest: String,
    source_subject_id: String,
    source_name: String,
    intent: String,
    intended_effect: String,
    activities: Vec<StrategicActivityKind>,
    target_subject_ids: Vec<String>,
    location_ids: Vec<String>,
    source_state: serde_json::Value,
    target_state: Vec<serde_json::Value>,
    active_relations: Vec<serde_json::Value>,
    pressure_owners: Vec<PressureOwnerContext>,
    resource_owner_id: String,
    resource_creation_owner_ids: Vec<String>,
    resource_recipient_ids: Vec<String>,
    discoverable_facts: Vec<serde_json::Value>,
    communicable_facts: Vec<CommunicableFactContext>,
    member_state_owner_id: Option<String>,
    admissible_effect_kinds: Vec<OutcomeEffectKind>,
    allowed_state_references: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
struct PressureOwnerContext {
    owner_subject_id: String,
    current_pressures: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
struct CommunicableFactContext {
    fact_id: String,
    statement: String,
    recipient_subject_ids: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct OutcomeProposalBundle {
    outcomes: Vec<OutcomeProposal>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum OutcomeEffectKind {
    NoMaterialChange,
    ResourceCreated,
    ResourceConsumed,
    ResourceTransferred,
    GestaltPressure,
    AgencyRelationShift,
    MemberMemory,
    MemberObligation,
    MemberRelationship,
    KnowledgeLearned,
    KnowledgeCommunicated,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct OutcomeProposal {
    action_digest: String,
    band: StrategicOutcomeBand,
    effect_kind: OutcomeEffectKind,
    #[serde(default)]
    supporting_state_references: Vec<String>,
    #[serde(default)]
    owner_subject_id: Option<String>,
    #[serde(default)]
    other_subject_id: Option<String>,
    #[serde(default)]
    relation_id: Option<String>,
    #[serde(default)]
    strength_delta: Option<i16>,
    #[serde(default)]
    resource: Option<String>,
    #[serde(default, deserialize_with = "null_as_empty_vec")]
    pressure_additions: Vec<String>,
    #[serde(default, deserialize_with = "null_as_empty_vec")]
    pressure_resolutions: Vec<String>,
    #[serde(default)]
    member_id: Option<String>,
    #[serde(default)]
    memory: Option<String>,
    #[serde(default)]
    obligation: Option<String>,
    #[serde(default)]
    relationship_description: Option<String>,
    #[serde(default)]
    fact_id: Option<String>,
    #[serde(default, deserialize_with = "null_as_empty_vec")]
    recipient_subject_ids: Vec<String>,
    #[serde(default)]
    reason: Option<String>,
}

fn null_as_empty_vec<'de, D>(deserializer: D) -> std::result::Result<Vec<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Ok(Option::<Vec<String>>::deserialize(deserializer)?.unwrap_or_default())
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct OutcomeVerifierBundle {
    verdicts: Vec<OutcomeVerifierVerdict>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum OutcomeVerifierResult {
    Match,
    Mismatch,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct OutcomeVerifierVerdict {
    action_digest: String,
    result: OutcomeVerifierResult,
    #[serde(default)]
    repair_guidance: Option<String>,
}

pub async fn resolve_activity_outcomes(
    model: Arc<dyn ModelPort>,
    campaign: &Campaign,
    proposals: &[CellActionProposal],
) -> Result<(Vec<StrategicActivityOutcome>, Vec<ModelStageOutput>)> {
    if proposals.is_empty() {
        return Ok((Vec::new(), Vec::new()));
    }
    let parallelism = usize::from(campaign.resolution_policy.provider_parallelism.max(1));
    let permits = Arc::new(Semaphore::new(parallelism));
    let campaign = Arc::new(campaign.clone());
    let mut tasks = JoinSet::new();
    for (index, proposal) in proposals.iter().cloned().enumerate() {
        let model = model.clone();
        let campaign = campaign.clone();
        let permits = permits.clone();
        tasks.spawn(async move {
            let _permit = permits
                .acquire_owned()
                .await
                .map_err(|_| anyhow!("strategic outcome concurrency gate closed"))?;
            let resolved = resolve_activity_outcome(model.as_ref(), &campaign, &proposal).await?;
            Ok::<_, anyhow::Error>((index, resolved))
        });
    }
    let mut ordered = BTreeMap::new();
    while let Some(task) = tasks.join_next().await {
        let (index, resolved) =
            task.map_err(|error| anyhow!("strategic outcome worker failed: {error}"))??;
        if ordered.insert(index, resolved).is_some() {
            return Err(anyhow!(
                "strategic outcome worker returned a duplicate slot"
            ));
        }
    }
    if ordered.len() != proposals.len() {
        return Err(anyhow!(
            "strategic outcome workers returned {} slots for {} selected actions",
            ordered.len(),
            proposals.len()
        ));
    }
    let mut outcomes = Vec::with_capacity(proposals.len());
    let mut stages = Vec::new();
    for (_, (mut action_outcomes, mut action_stages)) in ordered {
        outcomes.append(&mut action_outcomes);
        stages.append(&mut action_stages);
    }
    Ok((outcomes, stages))
}

async fn resolve_activity_outcome(
    model: &dyn ModelPort,
    campaign: &Campaign,
    proposal: &CellActionProposal,
) -> Result<(Vec<StrategicActivityOutcome>, Vec<ModelStageOutput>)> {
    let proposals = std::slice::from_ref(proposal);
    let context = build_context(campaign, proposals)?;
    let digests = proposals
        .iter()
        .map(cell_action_digest)
        .collect::<Result<Vec<_>>>()?;
    let binding = activity_outcome_binding(
        campaign.id,
        campaign.revision,
        campaign.resolution_policy.resolution_epoch,
        &digests,
    );
    let mut schema = serde_json::to_value(schema_for!(OutcomeProposalBundle))?;
    constrain_outcome_schema(&mut schema, &context.actions)?;
    let source_receipt_ids: Vec<String> = proposals
        .iter()
        .flat_map(|proposal| outcome_source_receipts(campaign, proposal))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    let static_contract = format!(
        "You are Ghostlight's private strategic outcome resolver. The Interpreter already established each exact constituent's selected attempt; you alone assess opposition and choose its bounded durable result. Resolve every supplied action_digest exactly once. Never add or remove an action. For each action, effect_kind must come from that action's admissible_effect_kinds; this is the runtime's exact projection of locally valid consequence handles. Use only IDs, resources, pressure resolutions, relations, facts, member owners, targets, and state references supplied for that same action. Never mutate the player. Never treat an arena as an actor or union constituents' private state. Prefer the most specific causally supported durable effect when the attempt and its band establish one. A mixed result should preserve bounded progress, cost, or a new unresolved pressure when a supplied handle supports it; do not collapse concrete partial work into no change merely because it is incomplete. Use no_material_change when none of the other supplied handles honestly represents a durable result; success or mixed success does not itself authorize inventing a response, fact, relationship, or resource. A failure may create a pressure or spend a committed resource when causally supported. Every material effect must actually change the supplied state; do not repeat an existing resource, pressure, memory, obligation, relationship description, or known fact. Choose exactly one effect_kind. Populate only its fields and omit every irrelevant optional field. no_material_change requires reason. resource_created creates one bounded branch-local resource and requires a capability reference. Its owner_subject_id must copy one exact resource_creation_owner_ids value: choose the source for portable stock or equipment it retains, or an exact activity location for a durable installed, placed, or repaired feature there. resource_consumed spends one exact existing source resource. resource_transferred gives one exact existing source resource to one recipient: copy one exact resource_recipient_ids value into the output field other_subject_id; it cannot take from a target. For resource_consumed and resource_transferred, owner_subject_id must copy that action's exact resource_owner_id, including any member: prefix. gestalt_pressure copies one exact pressure_owners.owner_subject_id value adjacent to that owner's current_pressures into owner_subject_id; resolutions must copy exact current pressure text. agency_relation_shift uses one supplied active relation and a nonzero delta from -10 through 10. Member memory, obligation, or relationship may change only the supplied member_state_owner_id; member_id omits the member: prefix. A relationship's other_subject_id must be one exact action target. knowledge_learned uses one supplied discoverable fact and teaches only the source. knowledge_communicated copies one communicable_facts fact_id and a nonempty subset of its exact recipient_subject_ids; owner_subject_id is the source speaker. Every material effect needs at least one supporting_state_reference copied literally and only from that action's allowed_state_references. IDs shown in source_state, target_state, target_subject_ids, or effect-specific owner and target fields are not provenance handles unless repeated in allowed_state_references. Return one JSON object and no prose outside JSON.\n\nOUTPUT CONTRACT:\n{OUTCOME_PROPOSAL_OUTPUT_CONTRACT}"
    );
    let mut request = ModelStageRequest {
        stage: "strategic_outcome_resolver".into(),
        model: MODEL_FAST.into(),
        snapshot_binding: binding,
        lived_stream: format!(
            "{static_contract}\n\nOUTCOME_CONTEXT:\n{}",
            serde_json::to_string(&context)?
        ),
        output_schema: Some(schema),
        source_receipt_ids: source_receipt_ids.clone(),
        temperature: Some(0.0),
        max_output_tokens: Some(2_400),
    };
    let mut stages = Vec::new();
    for semantic_attempt in 0..2 {
        let mut stage = run_validated_stage(model, &request).await?;
        let proposal_bundle: OutcomeProposalBundle = serde_json::from_value(
            stage
                .structured
                .clone()
                .ok_or_else(|| anyhow!("strategic outcome resolver produced no typed output"))?,
        )?;
        match bind_outcomes(campaign, proposals, proposal_bundle) {
            Ok(outcomes) => {
                stages.push(stage);
                let semantic_outcomes = outcomes
                    .iter()
                    .filter(|outcome| is_material_outcome(&outcome.effect))
                    .cloned()
                    .collect::<Vec<_>>();
                if semantic_outcomes.is_empty() {
                    return Ok((outcomes, stages));
                }
                let semantic_digests = semantic_outcomes
                    .iter()
                    .map(|outcome| outcome.action_digest.clone())
                    .collect::<Vec<_>>();
                let semantic_digest_set = semantic_digests.iter().cloned().collect::<BTreeSet<_>>();
                let semantic_proposals = proposals
                    .iter()
                    .filter(|proposal| {
                        cell_action_digest(proposal)
                            .is_ok_and(|digest| semantic_digest_set.contains(&digest))
                    })
                    .cloned()
                    .collect::<Vec<_>>();
                let semantic_context = build_context(campaign, &semantic_proposals)?;
                let (verifier, mismatches) = verify_outcomes(
                    model,
                    campaign,
                    &semantic_context,
                    &semantic_outcomes,
                    &semantic_digests,
                    &source_receipt_ids,
                )
                .await?;
                stages.push(verifier);
                if mismatches.is_empty() {
                    return Ok((outcomes, stages));
                }
                if semantic_attempt == 0 {
                    request.lived_stream.push_str(&format!(
                        "\n\nCORRECTION TASK—THE INDEPENDENT OUTCOME VERIFIER REJECTED THE PREVIOUS BUNDLE.\nREPAIRS:\n{}\nPREVIOUS_REJECTED_OUTCOMES:\n{}\nReturn one complete corrected bundle against the exact same snapshot and action digests. Do not preserve a rejected effect_kind unless its repair explicitly says that kind remains valid. When no exact supplied handle can express the repair, use no_material_change with a concrete reason.",
                        mismatches.join("\n"),
                        serde_json::to_string(&outcomes)?
                    ));
                } else {
                    return Err(anyhow!(
                        "strategic outcome verifier rejected the corrected bundle: {}",
                        mismatches.join("; ")
                    ));
                }
            }
            Err(error) if semantic_attempt == 0 => {
                let rejected = stage.narrative.clone();
                stage.receipt.validation_result = "semantic_invalid".into();
                stage.receipt.local_validation_error =
                    Some(error.to_string().chars().take(1_000).collect());
                stages.push(stage);
                request.lived_stream.push_str(&format!(
                    "\n\nCORRECTION TASK—THE PREVIOUS OUTCOME BUNDLE WAS REJECTED.\nREJECTION: {error}\nPREVIOUS_REJECTED_BUNDLE:\n{rejected}\nReturn one complete corrected bundle against the exact same snapshot and action digests. Do not preserve an effect that violates the stated handle set."
                ));
            }
            Err(error) => {
                return Err(anyhow!(
                    "strategic outcome resolver failed semantic validation after one correction: {error}"
                ));
            }
        }
    }
    unreachable!()
}

fn is_material_outcome(effect: &StrategicOutcomeEffect) -> bool {
    !matches!(effect, StrategicOutcomeEffect::NoMaterialChange { .. })
}

async fn verify_outcomes(
    model: &dyn ModelPort,
    campaign: &Campaign,
    context: &OutcomeContext,
    outcomes: &[StrategicActivityOutcome],
    digests: &[String],
    source_receipt_ids: &[String],
) -> Result<(ModelStageOutput, Vec<String>)> {
    let mut schema = serde_json::to_value(schema_for!(OutcomeVerifierBundle))?;
    constrain_verifier_schema(&mut schema, digests)?;
    let outcome_hash = format!(
        "sha256:{:x}",
        Sha256::digest(rmp_serde::to_vec_named(outcomes)?)
    );
    let request = ModelStageRequest {
        stage: "strategic_outcome_verifier".into(),
        model: MODEL_FAST.into(),
        snapshot_binding: format!(
            "{}:verifier:{outcome_hash}",
            activity_outcome_binding(
                campaign.id,
                campaign.revision,
                campaign.resolution_policy.resolution_epoch,
                digests,
            )
        ),
        lived_stream: format!(
            "You are Ghostlight's independent semantic verifier for material strategic outcomes. The local validator has already proved IDs, custody, scope, and bounds. Judge only whether each proposed durable mutation is causally entailed by that exact subject's attempt and supplied state. Return one verdict per action_digest in supplied order. Resource creation requires concrete making, repair, or preparation that establishes the named durable result. Its custodian must match the result: portable stock or equipment stays with the source, while an installed, placed, or repaired feature belongs to the exact activity location where it was established. A resource_consumed must be an exact resource the attempt actually uses, spends, gives up, damages, or transforms; reject unrelated inventory charges. A resource_transferred requires the attempt to give that exact resource to that exact recipient. A pressure addition or resolution must follow from the attempted activity rather than merely naming a current pressure. A relation shift requires an interaction capable of changing that relationship, not merely a message, proximity, or unrelated work. Member memory, obligation, and relationship effects require an event in the attempt that could create that exact personal delta. Knowledge learned requires an investigation, observation, or communication that can actually teach the source that exact fact; the fact merely appearing in supplied state or references is not enough. Knowledge communicated requires the attempt to convey that exact source-known fact to those exact recipients; addressing them without stating the fact is insufficient. result is match or mismatch. match requires null repair_guidance; mismatch requires one concrete correction sentence of at most 240 characters. Return JSON only.\n\nOUTPUT CONTRACT:\n{OUTCOME_VERIFIER_OUTPUT_CONTRACT}\n\nACTION_CONTEXT:\n{}\n\nPROPOSED_OUTCOMES:\n{}",
            serde_json::to_string(context)?,
            serde_json::to_string(outcomes)?,
        ),
        output_schema: Some(schema),
        source_receipt_ids: source_receipt_ids.to_vec(),
        temperature: Some(0.0),
        max_output_tokens: Some(1_200),
    };
    let stage = run_validated_stage(model, &request).await?;
    let bundle: OutcomeVerifierBundle = serde_json::from_value(
        stage
            .structured
            .clone()
            .ok_or_else(|| anyhow!("strategic outcome verifier produced no typed output"))?,
    )?;
    let expected = digests.iter().cloned().collect::<BTreeSet<_>>();
    let mut seen = BTreeSet::new();
    let mut mismatches = Vec::new();
    for verdict in bundle.verdicts {
        if !expected.contains(&verdict.action_digest) || !seen.insert(verdict.action_digest.clone())
        {
            return Err(anyhow!(
                "strategic outcome verifier changed or duplicated an action digest"
            ));
        }
        match (verdict.result, verdict.repair_guidance) {
            (OutcomeVerifierResult::Match, None) => {}
            (OutcomeVerifierResult::Mismatch, Some(guidance)) if !guidance.trim().is_empty() => {
                mismatches.push(format!(
                    "{}: {}",
                    verdict.action_digest,
                    guidance.chars().take(240).collect::<String>()
                ));
            }
            _ => {
                return Err(anyhow!(
                    "strategic outcome verifier returned incoherent result or repair guidance"
                ));
            }
        }
    }
    if seen != expected {
        return Err(anyhow!(
            "strategic outcome verifier omitted an action digest"
        ));
    }
    Ok((stage, mismatches))
}

pub fn activity_outcome_binding(
    campaign_id: uuid::Uuid,
    world_revision: u64,
    resolution_epoch: u64,
    digests: &[String],
) -> String {
    let mut exact = digests.to_vec();
    exact.sort();
    let digest = format!("sha256:{:x}", Sha256::digest(exact.join("|").as_bytes()));
    format!(
        "campaign:{campaign_id}:revision:{world_revision}:resolution:{resolution_epoch}:strategic-outcomes:{digest}"
    )
}

pub fn plan_activity_digests(plan: &StrategicTickPlan) -> Vec<String> {
    plan.gestalt_activities
        .iter()
        .map(|activity| activity.action_digest.clone())
        .chain(
            plan.actor_activities
                .iter()
                .map(|activity| activity.action_digest.clone()),
        )
        .chain(
            plan.member_activities
                .iter()
                .map(|activity| activity.action_digest.clone()),
        )
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

pub fn validate_activity_outcomes(
    campaign: &Campaign,
    proposals: &[CellActionProposal],
    outcomes: &[StrategicActivityOutcome],
) -> Result<()> {
    let expected = proposals
        .iter()
        .map(|proposal| Ok((cell_action_digest(proposal)?, proposal)))
        .collect::<Result<BTreeMap<_, _>>>()?;
    validate_outcomes_against_expected(campaign, &expected, outcomes)
}

pub fn validate_plan_activity_outcomes(
    campaign: &Campaign,
    plan: &StrategicTickPlan,
) -> Result<()> {
    if !plan.selected_actions.is_empty() {
        let selected = plan
            .selected_actions
            .iter()
            .filter(|proposal| proposal.effects.iter().any(is_activity_effect))
            .map(|proposal| Ok((cell_action_digest(proposal)?, proposal)))
            .collect::<Result<BTreeMap<_, _>>>()?;
        if selected.len()
            != plan
                .selected_actions
                .iter()
                .filter(|proposal| proposal.effects.iter().any(is_activity_effect))
                .count()
        {
            return Err(anyhow!(
                "selected strategic activities contain duplicate action digests"
            ));
        }
        return validate_outcomes_against_expected(campaign, &selected, &plan.activity_outcomes);
    }
    let synthetic = plan
        .gestalt_activities
        .iter()
        .map(|activity| {
            (
                activity.action_digest.clone(),
                CellActionProposal {
                    subject_id: activity.gestalt_id.clone(),
                    intent: "committed strategic activity".into(),
                    intended_effect: "resolve the committed strategic activity".into(),
                    priority: 0,
                    state_references: vec![],
                    public_channels: activity.public_channels.clone(),
                    effects: vec![StrategicCellEffect::GestaltActivity {
                        gestalt_id: activity.gestalt_id.clone(),
                        activity: activity.activity.clone(),
                        target_subject_ids: activity.target_subject_ids.clone(),
                        location_ids: activity.location_ids.clone(),
                    }],
                },
            )
        })
        .chain(plan.actor_activities.iter().map(|activity| {
            (
                activity.action_digest.clone(),
                CellActionProposal {
                    subject_id: activity.actor_id.clone(),
                    intent: "committed strategic activity".into(),
                    intended_effect: "resolve the committed strategic activity".into(),
                    priority: 0,
                    state_references: vec![],
                    public_channels: activity.public_channels.clone(),
                    effects: vec![StrategicCellEffect::ActorActivity {
                        actor_id: activity.actor_id.clone(),
                        activity: activity.activity.clone(),
                        target_subject_ids: activity.target_subject_ids.clone(),
                        location_ids: activity.location_ids.clone(),
                    }],
                },
            )
        }))
        .chain(plan.member_activities.iter().map(|activity| {
            (
                activity.action_digest.clone(),
                CellActionProposal {
                    subject_id: crate::domain::gestalt_member_subject_id(&activity.member_id),
                    intent: "committed strategic activity".into(),
                    intended_effect: "resolve the committed strategic activity".into(),
                    priority: 0,
                    state_references: vec![],
                    public_channels: activity.public_channels.clone(),
                    effects: vec![StrategicCellEffect::MemberActivity {
                        member_id: activity.member_id.clone(),
                        activity: activity.activity.clone(),
                        target_subject_ids: activity.target_subject_ids.clone(),
                        location_ids: activity.location_ids.clone(),
                    }],
                },
            )
        }))
        .collect::<BTreeMap<_, _>>();
    if synthetic.len()
        != plan.gestalt_activities.len()
            + plan.actor_activities.len()
            + plan.member_activities.len()
    {
        return Err(anyhow!(
            "strategic activities contain duplicate action digests"
        ));
    }
    let expected = synthetic
        .iter()
        .map(|(digest, proposal)| (digest.clone(), proposal))
        .collect();
    validate_outcomes_against_expected(campaign, &expected, &plan.activity_outcomes)
}

fn validate_outcomes_against_expected(
    campaign: &Campaign,
    expected: &BTreeMap<String, &CellActionProposal>,
    outcomes: &[StrategicActivityOutcome],
) -> Result<()> {
    if outcomes.len() != expected.len() {
        return Err(anyhow!(
            "every selected strategic activity requires exactly one outcome"
        ));
    }
    let mut seen = BTreeSet::new();
    let mut exclusive_effects = BTreeSet::new();
    let mut relation_totals = BTreeMap::<String, i16>::new();
    for outcome in outcomes {
        let proposal = expected
            .get(&outcome.action_digest)
            .ok_or_else(|| anyhow!("strategic outcome is not bound to a selected activity"))?;
        if !seen.insert(outcome.action_digest.clone())
            || outcome.schema != "ghostlight.strategic_activity_outcome.v1"
            || outcome.source_subject_id != proposal.subject_id
            || !valid_sha256(&outcome.action_digest)
            || !bounded_text(&outcome.summary, 240)
        {
            return Err(anyhow!(
                "strategic outcome identity or summary is malformed"
            ));
        }
        let allowed_references = allowed_state_references(campaign, proposal)?;
        let unique_references = outcome
            .supporting_state_references
            .iter()
            .collect::<BTreeSet<_>>();
        if unique_references.len() != outcome.supporting_state_references.len() {
            return Err(anyhow!(
                "outcome {} repeats a supporting_state_reference; each exact handle may appear once",
                outcome.action_digest
            ));
        }
        if outcome.supporting_state_references.len() > 8 {
            return Err(anyhow!(
                "outcome {} cites {} supporting_state_references; the exact limit is 8",
                outcome.action_digest,
                outcome.supporting_state_references.len()
            ));
        }
        let unavailable_references = outcome
            .supporting_state_references
            .iter()
            .filter(|reference| !allowed_references.contains(*reference))
            .cloned()
            .collect::<Vec<_>>();
        if !unavailable_references.is_empty() {
            return Err(anyhow!(
                "outcome {} cites unavailable supporting_state_references {}; copy only from this action's exact allowed_state_references {}",
                outcome.action_digest,
                serde_json::to_string(&unavailable_references)?,
                serde_json::to_string(&allowed_references)?,
            ));
        }
        if !matches!(
            outcome.effect,
            StrategicOutcomeEffect::NoMaterialChange { .. }
        ) && outcome.supporting_state_references.is_empty()
        {
            return Err(anyhow!(
                "material outcome {} requires at least one supporting_state_reference copied from its exact allowed_state_references {}",
                outcome.action_digest,
                serde_json::to_string(&allowed_references)?,
            ));
        }
        validate_effect(campaign, proposal, outcome, &mut exclusive_effects).map_err(|error| {
            let admissible = admissible_effect_kinds(campaign, proposal)
                .and_then(|kinds| serde_json::to_string(&kinds).map_err(Into::into))
                .unwrap_or_else(|_| "[unavailable]".into());
            anyhow!(
                "outcome {} is invalid: {error}; exact admissible_effect_kinds are {admissible}",
                outcome.action_digest
            )
        })?;
        if let StrategicOutcomeEffect::AgencyRelationShift {
            relation_id,
            strength_delta,
        } = &outcome.effect
        {
            *relation_totals.entry(relation_id.clone()).or_default() += *strength_delta;
        }
    }
    if seen.len() != expected.len() {
        return Err(anyhow!("strategic outcome bundle omitted an activity"));
    }
    for (relation_id, total) in relation_totals {
        let relation = &campaign.agency_relations[&relation_id];
        if total.abs() > 10 || !(1..=100).contains(&(i16::from(relation.strength) + total)) {
            return Err(anyhow!(
                "strategic outcome relation shifts exceed one bounded wave"
            ));
        }
    }
    Ok(())
}

fn bind_outcomes(
    campaign: &Campaign,
    proposals: &[CellActionProposal],
    bundle: OutcomeProposalBundle,
) -> Result<Vec<StrategicActivityOutcome>> {
    let sources = proposals
        .iter()
        .map(|proposal| Ok((cell_action_digest(proposal)?, proposal)))
        .collect::<Result<BTreeMap<_, _>>>()?;
    let outcomes = bundle
        .outcomes
        .into_iter()
        .map(|proposal| {
            let action = sources
                .get(&proposal.action_digest)
                .copied()
                .ok_or_else(|| anyhow!("outcome used an unknown action digest"))?;
            let source_subject_id = action.subject_id.clone();
            let effect = bind_effect(&proposal)?;
            let summary = resolved_outcome_summary(campaign, action, &effect)?;
            Ok(StrategicActivityOutcome {
                schema: "ghostlight.strategic_activity_outcome.v1".into(),
                action_digest: proposal.action_digest,
                source_subject_id,
                band: proposal.band,
                summary,
                supporting_state_references: proposal.supporting_state_references,
                effect,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    validate_activity_outcomes(campaign, proposals, &outcomes)?;
    Ok(outcomes)
}

fn bind_effect(proposal: &OutcomeProposal) -> Result<StrategicOutcomeEffect> {
    let populated = proposal.populated_effect_fields();
    let require_exact = |allowed: &[&str]| -> Result<()> {
        let allowed = allowed.iter().copied().collect::<BTreeSet<_>>();
        let irrelevant = populated.difference(&allowed).copied().collect::<Vec<_>>();
        if !irrelevant.is_empty() {
            return Err(anyhow!(
                "outcome populated irrelevant fields [{}]; the chosen effect_kind permits only [{}]",
                irrelevant.join(", "),
                allowed.into_iter().collect::<Vec<_>>().join(", ")
            ));
        }
        Ok(())
    };
    Ok(match proposal.effect_kind {
        OutcomeEffectKind::NoMaterialChange => {
            require_exact(&["reason"])?;
            StrategicOutcomeEffect::NoMaterialChange {
                reason: required(&proposal.reason, "reason")?,
            }
        }
        OutcomeEffectKind::ResourceCreated => {
            require_exact(&["owner_subject_id", "resource"])?;
            StrategicOutcomeEffect::ResourceCreated {
                owner_subject_id: required(&proposal.owner_subject_id, "owner_subject_id")?,
                resource: required(&proposal.resource, "resource")?,
            }
        }
        OutcomeEffectKind::ResourceConsumed => {
            require_exact(&["owner_subject_id", "resource"])?;
            StrategicOutcomeEffect::ResourceConsumed {
                owner_subject_id: required(&proposal.owner_subject_id, "owner_subject_id")?,
                resource: required(&proposal.resource, "resource")?,
            }
        }
        OutcomeEffectKind::ResourceTransferred => {
            require_exact(&["owner_subject_id", "other_subject_id", "resource"])?;
            StrategicOutcomeEffect::ResourceTransferred {
                from_subject_id: required(&proposal.owner_subject_id, "owner_subject_id")?,
                to_subject_id: required(&proposal.other_subject_id, "other_subject_id")?,
                resource: required(&proposal.resource, "resource")?,
            }
        }
        OutcomeEffectKind::GestaltPressure => {
            require_exact(&[
                "owner_subject_id",
                "pressure_additions",
                "pressure_resolutions",
            ])?;
            StrategicOutcomeEffect::GestaltPressure {
                gestalt_id: required(&proposal.owner_subject_id, "owner_subject_id")?,
                pressure_additions: proposal.pressure_additions.clone(),
                pressure_resolutions: proposal.pressure_resolutions.clone(),
            }
        }
        OutcomeEffectKind::AgencyRelationShift => {
            require_exact(&["relation_id", "strength_delta"])?;
            StrategicOutcomeEffect::AgencyRelationShift {
                relation_id: required(&proposal.relation_id, "relation_id")?,
                strength_delta: proposal
                    .strength_delta
                    .ok_or_else(|| anyhow!("outcome omitted strength_delta"))?,
            }
        }
        OutcomeEffectKind::MemberMemory => {
            require_exact(&["member_id", "memory"])?;
            StrategicOutcomeEffect::MemberMemory {
                member_id: required(&proposal.member_id, "member_id")?,
                memory: required(&proposal.memory, "memory")?,
            }
        }
        OutcomeEffectKind::MemberObligation => {
            require_exact(&["member_id", "obligation"])?;
            StrategicOutcomeEffect::MemberObligation {
                member_id: required(&proposal.member_id, "member_id")?,
                obligation: required(&proposal.obligation, "obligation")?,
            }
        }
        OutcomeEffectKind::MemberRelationship => {
            require_exact(&["member_id", "other_subject_id", "relationship_description"])?;
            StrategicOutcomeEffect::MemberRelationship {
                member_id: required(&proposal.member_id, "member_id")?,
                other_subject_id: required(&proposal.other_subject_id, "other_subject_id")?,
                description: required(
                    &proposal.relationship_description,
                    "relationship_description",
                )?,
            }
        }
        OutcomeEffectKind::KnowledgeLearned => {
            require_exact(&["owner_subject_id", "fact_id"])?;
            StrategicOutcomeEffect::KnowledgeLearned {
                owner_subject_id: required(&proposal.owner_subject_id, "owner_subject_id")?,
                fact_id: required(&proposal.fact_id, "fact_id")?,
            }
        }
        OutcomeEffectKind::KnowledgeCommunicated => {
            require_exact(&["owner_subject_id", "recipient_subject_ids", "fact_id"])?;
            StrategicOutcomeEffect::KnowledgeCommunicated {
                from_subject_id: required(&proposal.owner_subject_id, "owner_subject_id")?,
                to_subject_ids: proposal.recipient_subject_ids.clone(),
                fact_id: required(&proposal.fact_id, "fact_id")?,
            }
        }
    })
}

fn resolved_outcome_summary(
    campaign: &Campaign,
    action: &CellActionProposal,
    effect: &StrategicOutcomeEffect,
) -> Result<String> {
    let source_subject_id = &action.subject_id;
    let source_name = subject_name(campaign, source_subject_id)?;
    let summary = match effect {
        StrategicOutcomeEffect::NoMaterialChange { .. } => {
            format!("{source_name}'s attempt produces no durable state change.")
        }
        StrategicOutcomeEffect::ResourceCreated {
            owner_subject_id,
            resource,
        } if owner_subject_id == source_subject_id => {
            format!("{source_name} creates and retains {resource}.")
        }
        StrategicOutcomeEffect::ResourceCreated {
            owner_subject_id,
            resource,
        } => format!(
            "At {}, {source_name} establishes {resource}.",
            subject_name(campaign, owner_subject_id)?
        ),
        StrategicOutcomeEffect::ResourceConsumed { resource, .. } => {
            format!("{source_name} expends {resource}.")
        }
        StrategicOutcomeEffect::ResourceTransferred {
            to_subject_id,
            resource,
            ..
        } => format!(
            "{source_name} transfers {resource} to {}.",
            subject_name(campaign, to_subject_id)?
        ),
        StrategicOutcomeEffect::GestaltPressure {
            gestalt_id,
            pressure_additions,
            pressure_resolutions,
        } => {
            let owner = subject_name(campaign, gestalt_id)?;
            if let Some(pressure) = pressure_additions.first() {
                format!("New pressure on {owner}: {pressure}.")
            } else if let Some(pressure) = pressure_resolutions.first() {
                format!("A live dispute involving {owner} is settled: {pressure}.")
            } else {
                return Err(anyhow!("pressure outcome has no state transition"));
            }
        }
        StrategicOutcomeEffect::AgencyRelationShift {
            relation_id,
            strength_delta,
        } => {
            let relation = campaign
                .agency_relations
                .get(relation_id)
                .ok_or_else(|| anyhow!("relation outcome relation vanished"))?;
            let direction = match (&relation.kind, *strength_delta > 0) {
                (crate::domain::AgencyRelationKind::Rivalry, true) => "deepens",
                (crate::domain::AgencyRelationKind::Rivalry, false) => "eases",
                (_, true) => "strengthens",
                (_, false) => "weakens",
            };
            let action = activity_label(action);
            format!(
                "{action} by {source_name} {direction} the {} relationship between {} and {}.",
                agency_relation_label(&relation.kind),
                subject_name(campaign, &relation.from_subject_id)?,
                subject_name(campaign, &relation.to_subject_id)?,
            )
        }
        StrategicOutcomeEffect::MemberMemory { member_id, memory } => format!(
            "{} retains a new memory: {memory}.",
            subject_name(
                campaign,
                &crate::domain::gestalt_member_subject_id(member_id)
            )?
        ),
        StrategicOutcomeEffect::MemberObligation {
            member_id,
            obligation,
        } => format!(
            "{} accepts an obligation: {obligation}.",
            subject_name(
                campaign,
                &crate::domain::gestalt_member_subject_id(member_id)
            )?
        ),
        StrategicOutcomeEffect::MemberRelationship {
            member_id,
            other_subject_id,
            description,
        } => format!(
            "{}'s relationship with {} becomes: {description}.",
            subject_name(
                campaign,
                &crate::domain::gestalt_member_subject_id(member_id)
            )?,
            subject_name(campaign, other_subject_id)?
        ),
        StrategicOutcomeEffect::KnowledgeLearned {
            owner_subject_id,
            fact_id,
        } => format!(
            "{} learns: {}.",
            subject_name(campaign, owner_subject_id)?,
            campaign
                .facts
                .get(fact_id)
                .ok_or_else(|| anyhow!("knowledge outcome fact vanished"))?
                .statement
        ),
        StrategicOutcomeEffect::KnowledgeCommunicated {
            to_subject_ids,
            fact_id,
            ..
        } => {
            let statement = campaign
                .facts
                .get(fact_id)
                .ok_or_else(|| anyhow!("knowledge outcome fact vanished"))?
                .statement
                .trim()
                .trim_end_matches(['.', '!', '?']);
            format!(
                "{source_name} shares this with {}: {statement}.",
                to_subject_ids
                    .iter()
                    .map(|subject_id| subject_name(campaign, subject_id))
                    .collect::<Result<Vec<_>>>()?
                    .join(", ")
            )
        }
    };
    let summary = summary.chars().take(240).collect::<String>();
    if !bounded_text(&summary, 240) {
        return Err(anyhow!("derived strategic outcome summary is empty"));
    }
    Ok(summary)
}

fn activity_label(action: &CellActionProposal) -> &'static str {
    let mut activities = action.effects.iter().filter_map(|effect| match effect {
        StrategicCellEffect::GestaltActivity { activity, .. }
        | StrategicCellEffect::ActorActivity { activity, .. }
        | StrategicCellEffect::MemberActivity { activity, .. } => Some(activity),
        _ => None,
    });
    let Some(activity) = activities.next() else {
        return "A concerted campaign";
    };
    if activities.any(|candidate| candidate != activity) {
        return "A concerted campaign";
    }
    match activity {
        StrategicActivityKind::Prepare => "Preparations",
        StrategicActivityKind::Coordinate => "An organizing effort",
        StrategicActivityKind::Investigate => "An inquiry",
        StrategicActivityKind::Recruit => "A recruitment drive",
        StrategicActivityKind::Obstruct => "An obstruction",
        StrategicActivityKind::Trade => "A trade offer",
        StrategicActivityKind::Communicate if action.public_channels.is_empty() => "A message",
        StrategicActivityKind::Communicate => "A public appeal",
    }
}

fn agency_relation_label(kind: &crate::domain::AgencyRelationKind) -> &'static str {
    use crate::domain::AgencyRelationKind;
    match kind {
        AgencyRelationKind::Containment => "territorial",
        AgencyRelationKind::Command => "command",
        AgencyRelationKind::Membership => "membership",
        AgencyRelationKind::Alliance => "alliance",
        AgencyRelationKind::Rivalry => "rivalry",
        AgencyRelationKind::Trade => "trade",
        AgencyRelationKind::Migration => "migration",
        AgencyRelationKind::Communication => "communication",
        AgencyRelationKind::Coercion => "coercive",
        AgencyRelationKind::SharedLocation => "local",
    }
}

impl OutcomeProposal {
    fn populated_effect_fields(&self) -> BTreeSet<&'static str> {
        let mut fields = BTreeSet::new();
        macro_rules! present {
            ($field:ident) => {
                if self.$field.is_some() {
                    fields.insert(stringify!($field));
                }
            };
        }
        present!(owner_subject_id);
        present!(other_subject_id);
        present!(relation_id);
        present!(strength_delta);
        present!(resource);
        present!(member_id);
        present!(memory);
        present!(obligation);
        present!(relationship_description);
        present!(fact_id);
        if !self.recipient_subject_ids.is_empty() {
            fields.insert("recipient_subject_ids");
        }
        present!(reason);
        if !self.pressure_additions.is_empty() {
            fields.insert("pressure_additions");
        }
        if !self.pressure_resolutions.is_empty() {
            fields.insert("pressure_resolutions");
        }
        fields
    }
}

fn validate_effect(
    campaign: &Campaign,
    proposal: &CellActionProposal,
    outcome: &StrategicActivityOutcome,
    exclusive_effects: &mut BTreeSet<String>,
) -> Result<()> {
    let source = proposal.subject_id.as_str();
    let (activities, targets, locations) = activity_parts(proposal)?;
    let target_set = targets.iter().cloned().collect::<BTreeSet<_>>();
    match &outcome.effect {
        StrategicOutcomeEffect::NoMaterialChange { reason } => {
            if !bounded_text(reason, 240) {
                return Err(anyhow!("no-change outcome requires a bounded reason"));
            }
        }
        StrategicOutcomeEffect::ResourceCreated {
            owner_subject_id,
            resource,
        } => {
            if owner_subject_id != source && !locations.contains(owner_subject_id) {
                return Err(anyhow!(
                    "outcome {} resource_created owner_subject_id must be the source or one exact activity location",
                    outcome.action_digest
                ));
            }
            if !activities.contains(&StrategicActivityKind::Prepare) {
                return Err(anyhow!(
                    "outcome {} resource_created requires a prepare activity",
                    outcome.action_digest
                ));
            }
            if !bounded_text(resource, 160) {
                return Err(anyhow!(
                    "outcome {} resource_created requires one bounded resource description",
                    outcome.action_digest
                ));
            }
            let existing_resources = subject_resources(campaign, owner_subject_id)?;
            if contains_normalized(&existing_resources, resource) {
                return Err(anyhow!(
                    "outcome {} resource_created repeats an existing custodian resource",
                    outcome.action_digest
                ));
            }
            if existing_resources.len() >= 64 || !can_hold_resources(campaign, owner_subject_id) {
                return Err(anyhow!(
                    "outcome {} resource_created exceeds custodian resource capacity",
                    outcome.action_digest
                ));
            }
            if !outcome
                .supporting_state_references
                .iter()
                .any(|reference| reference.starts_with("capability:"))
            {
                return Err(anyhow!(
                    "outcome {} resource_created requires one supplied capability reference",
                    outcome.action_digest
                ));
            }
            if !exclusive_effects.insert(format!("resource:{owner_subject_id}:{resource}")) {
                return Err(anyhow!(
                    "outcome {} duplicates a resource creation in this wave",
                    outcome.action_digest
                ));
            }
        }
        StrategicOutcomeEffect::ResourceConsumed {
            owner_subject_id,
            resource,
        } => {
            if owner_subject_id != source
                || !subject_resources(campaign, source)?.contains(resource)
                || !exclusive_effects.insert(format!("resource:{source}:{resource}"))
            {
                return Err(anyhow!("resource consumption lacks exact source custody"));
            }
        }
        StrategicOutcomeEffect::ResourceTransferred {
            from_subject_id,
            to_subject_id,
            resource,
        } => {
            if from_subject_id != source
                || !target_set.contains(to_subject_id)
                || is_human_controlled_actor(campaign, to_subject_id)
                || !can_hold_resources(campaign, to_subject_id)
                || !subject_resources(campaign, source)?.contains(resource)
                || contains_normalized(&subject_resources(campaign, to_subject_id)?, resource)
                || subject_resources(campaign, to_subject_id)?.len() >= 64
                || !activities.iter().any(|activity| {
                    matches!(
                        activity,
                        StrategicActivityKind::Trade
                            | StrategicActivityKind::Communicate
                            | StrategicActivityKind::Recruit
                            | StrategicActivityKind::Coordinate
                    )
                })
                || !exclusive_effects.insert(format!("resource:{source}:{resource}"))
            {
                return Err(anyhow!(
                    "resource transfer exceeds exact custody or target scope"
                ));
            }
        }
        StrategicOutcomeEffect::GestaltPressure {
            gestalt_id,
            pressure_additions,
            pressure_resolutions,
        } => {
            let allowed = pressure_owner_ids(campaign, proposal)?;
            let gestalt = campaign
                .gestalts
                .get(gestalt_id)
                .ok_or_else(|| anyhow!("outcome pressure owner is not a gestalt"))?;
            crate::resolution::validate_gestalt_pressure_transition(
                &gestalt.pressures,
                pressure_additions,
                pressure_resolutions,
            )?;
            if !allowed.contains(gestalt_id)
                || gestalt.pressures.len() + pressure_additions.len()
                    > 64 + pressure_resolutions.len()
                || !exclusive_effects.insert(format!("pressure:{gestalt_id}"))
            {
                return Err(anyhow!(
                    "outcome pressure transition exceeds exact target scope"
                ));
            }
        }
        StrategicOutcomeEffect::AgencyRelationShift {
            relation_id,
            strength_delta,
        } => {
            let relation = campaign
                .agency_relations
                .get(relation_id)
                .filter(|relation| relation.active)
                .ok_or_else(|| anyhow!("outcome relation is not active"))?;
            let other = if relation.from_subject_id == source {
                &relation.to_subject_id
            } else if relation.to_subject_id == source {
                &relation.from_subject_id
            } else {
                return Err(anyhow!("outcome relation is not incident to its source"));
            };
            if source.starts_with("member:")
                || !target_set.contains(other)
                || *strength_delta == 0
                || strength_delta.abs() > 10
                || !activities.iter().any(|activity| {
                    matches!(
                        activity,
                        StrategicActivityKind::Coordinate
                            | StrategicActivityKind::Recruit
                            | StrategicActivityKind::Obstruct
                            | StrategicActivityKind::Trade
                            | StrategicActivityKind::Communicate
                    )
                })
            {
                return Err(anyhow!(
                    "outcome relation shift exceeds exact source or activity"
                ));
            }
        }
        StrategicOutcomeEffect::MemberMemory { member_id, memory } => {
            validate_member_owner(source, member_id, memory, "memory", exclusive_effects)?;
            let member = &campaign.gestalt_members[member_id];
            if member.memories.len() >= 64 || contains_normalized_slice(&member.memories, memory) {
                return Err(anyhow!(
                    "member memory outcome is duplicate or over capacity"
                ));
            }
        }
        StrategicOutcomeEffect::MemberObligation {
            member_id,
            obligation,
        } => {
            validate_member_owner(
                source,
                member_id,
                obligation,
                "obligation",
                exclusive_effects,
            )?;
            let member = &campaign.gestalt_members[member_id];
            if member.obligations.len() >= 64
                || contains_normalized(&member.obligations, obligation)
            {
                return Err(anyhow!(
                    "member obligation outcome is duplicate or over capacity"
                ));
            }
        }
        StrategicOutcomeEffect::MemberRelationship {
            member_id,
            other_subject_id,
            description,
        } => {
            validate_member_owner(
                source,
                member_id,
                description,
                &format!("relationship:{other_subject_id}"),
                exclusive_effects,
            )?;
            if !target_set.contains(other_subject_id) {
                return Err(anyhow!(
                    "member relationship target was not part of the attempt"
                ));
            }
            let member = &campaign.gestalt_members[member_id];
            if (member.relationships.len() >= 64
                && !member.relationships.contains_key(other_subject_id))
                || member
                    .relationships
                    .get(other_subject_id)
                    .is_some_and(|current| {
                        !crate::resolution::substantive_text_change(current, description)
                    })
            {
                return Err(anyhow!(
                    "member relationship outcome is unchanged or over capacity"
                ));
            }
        }
        StrategicOutcomeEffect::KnowledgeLearned {
            owner_subject_id,
            fact_id,
        } => {
            if owner_subject_id != source
                || !activities.iter().any(|activity| {
                    matches!(
                        activity,
                        StrategicActivityKind::Investigate | StrategicActivityKind::Communicate
                    )
                })
                || !discoverable_fact_ids(campaign, proposal)?.contains(fact_id)
                || source_knowledge(campaign, source)?.contains(&campaign.facts[fact_id].statement)
                || !exclusive_effects.insert(format!("knowledge:{source}:{fact_id}"))
            {
                return Err(anyhow!("knowledge outcome exceeds exact fact access"));
            }
        }
        StrategicOutcomeEffect::KnowledgeCommunicated {
            from_subject_id,
            to_subject_ids,
            fact_id,
        } => {
            let permitted = communicable_facts(campaign, proposal)?
                .into_iter()
                .find(|fact| fact.fact_id == *fact_id)
                .ok_or_else(|| anyhow!("knowledge communication fact is not source-known"))?;
            let recipients = to_subject_ids.iter().cloned().collect::<BTreeSet<_>>();
            let permitted_recipients = permitted
                .recipient_subject_ids
                .into_iter()
                .collect::<BTreeSet<_>>();
            if from_subject_id != source
                || !activities.contains(&StrategicActivityKind::Communicate)
                || recipients.is_empty()
                || recipients.len() != to_subject_ids.len()
                || recipients.len() > 4
                || !recipients.is_subset(&target_set)
                || !recipients.is_subset(&permitted_recipients)
            {
                return Err(anyhow!(
                    "knowledge communication exceeds exact speaker, fact, or recipients"
                ));
            }
            for recipient in recipients {
                if !exclusive_effects.insert(format!("knowledge:{recipient}:{fact_id}")) {
                    return Err(anyhow!(
                        "knowledge communication duplicates a recipient fact in this wave"
                    ));
                }
            }
        }
    }
    if locations
        .iter()
        .any(|location| !campaign.locations.contains_key(location))
    {
        return Err(anyhow!("outcome activity location vanished"));
    }
    Ok(())
}

fn validate_member_owner(
    source: &str,
    member_id: &str,
    text: &str,
    field: &str,
    exclusive_effects: &mut BTreeSet<String>,
) -> Result<()> {
    if source.strip_prefix("member:") != Some(member_id)
        || !bounded_text(text, 240)
        || !exclusive_effects.insert(format!("member:{member_id}:{field}"))
    {
        return Err(anyhow!(
            "member outcome does not belong to its exact person"
        ));
    }
    Ok(())
}

fn build_context(campaign: &Campaign, proposals: &[CellActionProposal]) -> Result<OutcomeContext> {
    Ok(OutcomeContext {
        world_revision: campaign.revision,
        resolution_epoch: campaign.resolution_policy.resolution_epoch,
        actions: proposals
            .iter()
            .map(|proposal| action_context(campaign, proposal))
            .collect::<Result<_>>()?,
    })
}

fn action_context(
    campaign: &Campaign,
    proposal: &CellActionProposal,
) -> Result<ActionOutcomeContext> {
    let (activities, targets, locations) = activity_parts(proposal)?;
    let source_name = subject_name(campaign, &proposal.subject_id)?;
    let source_state = subject_summary(campaign, &proposal.subject_id)?;
    let target_state = targets
        .iter()
        .map(|target| subject_summary(campaign, target))
        .collect::<Result<_>>()?;
    let target_set = targets.iter().collect::<BTreeSet<_>>();
    let active_relations = campaign
        .agency_relations
        .values()
        .filter(|relation| {
            relation.active
                && ((relation.from_subject_id == proposal.subject_id
                    && target_set.contains(&relation.to_subject_id))
                    || (relation.to_subject_id == proposal.subject_id
                        && target_set.contains(&relation.from_subject_id)))
        })
        .map(|relation| {
            serde_json::json!({
                "relation_id":relation.id,
                "from_subject_id":relation.from_subject_id,
                "to_subject_id":relation.to_subject_id,
                "kind":relation.kind,
                "strength":relation.strength,
            })
        })
        .collect();
    let resource_recipient_ids = targets
        .iter()
        .filter(|target| {
            !is_human_controlled_actor(campaign, target) && can_hold_resources(campaign, target)
        })
        .cloned()
        .collect();
    let discoverable_facts = discoverable_fact_ids(campaign, proposal)?
        .into_iter()
        .map(|fact_id| {
            let fact = &campaign.facts[&fact_id];
            serde_json::json!({"fact_id":fact.id,"statement":fact.statement})
        })
        .collect();
    let communicable_facts = communicable_facts(campaign, proposal)?;
    let resource_creation_owner_ids = std::iter::once(proposal.subject_id.clone())
        .chain(locations.iter().cloned())
        .filter(|owner| can_hold_resources(campaign, owner))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    Ok(ActionOutcomeContext {
        action_digest: cell_action_digest(proposal)?,
        source_subject_id: proposal.subject_id.clone(),
        source_name,
        intent: proposal.intent.clone(),
        intended_effect: proposal.intended_effect.clone(),
        activities,
        target_subject_ids: targets,
        location_ids: locations,
        source_state,
        target_state,
        active_relations,
        pressure_owners: pressure_owner_ids(campaign, proposal)?
            .into_iter()
            .map(|owner_subject_id| PressureOwnerContext {
                current_pressures: campaign.gestalts[&owner_subject_id]
                    .pressures
                    .iter()
                    .cloned()
                    .collect(),
                owner_subject_id,
            })
            .collect(),
        resource_owner_id: proposal.subject_id.clone(),
        resource_creation_owner_ids,
        resource_recipient_ids,
        discoverable_facts,
        communicable_facts,
        member_state_owner_id: crate::resolution::dormant_member_id_for_subject(
            campaign,
            &proposal.subject_id,
        )
        .map(str::to_owned),
        admissible_effect_kinds: admissible_effect_kinds(campaign, proposal)?,
        allowed_state_references: allowed_state_references(campaign, proposal)?
            .into_iter()
            .collect(),
    })
}

fn admissible_effect_kinds(
    campaign: &Campaign,
    proposal: &CellActionProposal,
) -> Result<Vec<OutcomeEffectKind>> {
    let (activities, targets, _) = activity_parts(proposal)?;
    let source = proposal.subject_id.as_str();
    let resources = subject_resources(campaign, source)?;
    let references = allowed_state_references(campaign, proposal)?;
    let mut kinds = vec![OutcomeEffectKind::NoMaterialChange];

    if activities.contains(&StrategicActivityKind::Prepare)
        && resources.len() < 64
        && can_hold_resources(campaign, source)
        && references
            .iter()
            .any(|reference| reference.starts_with("capability:"))
    {
        kinds.push(OutcomeEffectKind::ResourceCreated);
    }
    if !resources.is_empty() {
        kinds.push(OutcomeEffectKind::ResourceConsumed);
    }
    if !resources.is_empty()
        && activities.iter().any(|activity| {
            matches!(
                activity,
                StrategicActivityKind::Trade
                    | StrategicActivityKind::Communicate
                    | StrategicActivityKind::Recruit
                    | StrategicActivityKind::Coordinate
            )
        })
        && targets.iter().any(|target| {
            !is_human_controlled_actor(campaign, target) && can_hold_resources(campaign, target)
        })
    {
        kinds.push(OutcomeEffectKind::ResourceTransferred);
    }
    if !pressure_owner_ids(campaign, proposal)?.is_empty() {
        kinds.push(OutcomeEffectKind::GestaltPressure);
    }
    if crate::resolution::dormant_member_id_for_subject(campaign, source).is_none()
        && activities.iter().any(|activity| {
            matches!(
                activity,
                StrategicActivityKind::Coordinate
                    | StrategicActivityKind::Recruit
                    | StrategicActivityKind::Obstruct
                    | StrategicActivityKind::Trade
                    | StrategicActivityKind::Communicate
            )
        })
        && campaign.agency_relations.values().any(|relation| {
            relation.active
                && ((relation.from_subject_id == source
                    && targets.contains(&relation.to_subject_id))
                    || (relation.to_subject_id == source
                        && targets.contains(&relation.from_subject_id)))
        })
    {
        kinds.push(OutcomeEffectKind::AgencyRelationShift);
    }
    if let Some(member_id) = crate::resolution::dormant_member_id_for_subject(campaign, source) {
        let member = &campaign.gestalt_members[member_id];
        if member.memories.len() < 64 {
            kinds.push(OutcomeEffectKind::MemberMemory);
        }
        if member.obligations.len() < 64
            && activities.iter().any(|activity| {
                matches!(
                    activity,
                    StrategicActivityKind::Communicate
                        | StrategicActivityKind::Coordinate
                        | StrategicActivityKind::Recruit
                        | StrategicActivityKind::Trade
                )
            })
        {
            kinds.push(OutcomeEffectKind::MemberObligation);
        }
        if !targets.is_empty() {
            kinds.push(OutcomeEffectKind::MemberRelationship);
        }
    }
    if activities.iter().any(|activity| {
        matches!(
            activity,
            StrategicActivityKind::Investigate | StrategicActivityKind::Communicate
        )
    }) && !discoverable_fact_ids(campaign, proposal)?.is_empty()
    {
        kinds.push(OutcomeEffectKind::KnowledgeLearned);
    }
    if !communicable_facts(campaign, proposal)?.is_empty() {
        kinds.push(OutcomeEffectKind::KnowledgeCommunicated);
    }
    Ok(kinds)
}

fn subject_summary(campaign: &Campaign, subject_id: &str) -> Result<serde_json::Value> {
    if let Some(actor) = campaign.actors.get(subject_id) {
        return Ok(serde_json::json!({
            "subject_id":subject_id,"name":actor.name,"kind":"actor",
            "capabilities":actor.capabilities,"knowledge":actor.knowledge,
            "resources":actor.equipment,"conditions":actor.conditions,
            "obligations":actor.obligations,"relationships":actor.relationships,"goals":actor.goals,
        }));
    }
    if let Some(member_id) = crate::resolution::dormant_member_id_for_subject(campaign, subject_id)
    {
        let member = campaign
            .gestalt_members
            .get(member_id)
            .ok_or_else(|| anyhow!("outcome context member vanished"))?;
        return Ok(serde_json::json!({
            "subject_id":subject_id,
            "name":member.name,
            "kind":"member",
            "gestalt_id":member.gestalt_id,
            "capabilities":crate::resolution::effective_member_capabilities(campaign, member_id)?,
            "knowledge":effective_member_knowledge(campaign, member_id)?,
            "resources":member.equipment,
            "conditions":member.conditions,
            "obligations":member.obligations,
            "relationships":member.relationships,
            "goals":member.goals,
        }));
    }
    if let Some(gestalt) = campaign.gestalts.get(subject_id) {
        return Ok(serde_json::json!({
            "subject_id":subject_id,"name":gestalt.name,"kind":"gestalt",
            "capabilities":gestalt.shared_capabilities,"knowledge":gestalt.shared_knowledge,
            "resources":gestalt.resources,"goals":gestalt.goals,"pressures":gestalt.pressures,
        }));
    }
    if let Some(institution) = campaign.institutions.get(subject_id) {
        return Ok(serde_json::json!({
            "subject_id":subject_id,"name":institution.name,"kind":"institution",
            "resources":institution.resources,"goals":institution.goals,"posture":institution.posture,
        }));
    }
    Err(anyhow!("outcome context subject vanished"))
}

fn allowed_state_references(
    campaign: &Campaign,
    proposal: &CellActionProposal,
) -> Result<BTreeSet<String>> {
    let mut references = if let Some(member_id) =
        crate::resolution::dormant_member_id_for_subject(campaign, &proposal.subject_id)
    {
        crate::resolution::member_state_references(campaign, member_id)?
    } else {
        subject_state_references(campaign, &proposal.subject_id)?
    };
    for relation in campaign
        .agency_relations
        .values()
        .filter(|relation| relation.active)
    {
        if relation.from_subject_id == proposal.subject_id
            || relation.to_subject_id == proposal.subject_id
        {
            references.insert(format!("relation:{}", relation.id));
        }
    }
    for owner_id in pressure_owner_ids(campaign, proposal)? {
        if let Some(gestalt) = campaign.gestalts.get(&owner_id) {
            references.extend(
                gestalt
                    .pressures
                    .iter()
                    .map(|pressure| format!("pressure:{owner_id}:{pressure}")),
            );
        }
    }
    for fact_id in discoverable_fact_ids(campaign, proposal)? {
        references.insert(format!("fact:{fact_id}"));
    }
    for fact in communicable_facts(campaign, proposal)? {
        references.insert(format!("fact:{}", fact.fact_id));
    }
    Ok(references)
}

fn pressure_owner_ids(
    campaign: &Campaign,
    proposal: &CellActionProposal,
) -> Result<BTreeSet<String>> {
    let (_, targets, _) = activity_parts(proposal)?;
    let mut ids = targets
        .into_iter()
        .filter(|target| campaign.gestalts.contains_key(target))
        .collect::<BTreeSet<_>>();
    if campaign.gestalts.contains_key(&proposal.subject_id) {
        ids.insert(proposal.subject_id.clone());
    }
    Ok(ids)
}

fn discoverable_fact_ids(
    campaign: &Campaign,
    proposal: &CellActionProposal,
) -> Result<BTreeSet<String>> {
    let (activities, targets, locations) = activity_parts(proposal)?;
    let mut facts = campaign
        .facts
        .values()
        .filter(|fact| {
            !fact.discoverable_at_location_ids.is_empty()
                && locations
                    .iter()
                    .any(|location| fact.discoverable_at_location_ids.contains(location))
        })
        .map(|fact| fact.id.clone())
        .collect::<BTreeSet<_>>();
    if activities.contains(&StrategicActivityKind::Communicate) {
        for target in targets {
            if is_human_controlled_actor(campaign, &target) {
                continue;
            }
            let known = source_knowledge(campaign, &target)?;
            facts.extend(
                campaign
                    .facts
                    .values()
                    .filter(|fact| known.contains(&fact.id) || known.contains(&fact.statement))
                    .map(|fact| fact.id.clone()),
            );
        }
    }
    let source_known = source_knowledge(campaign, &proposal.subject_id)?;
    facts.retain(|fact_id| {
        let fact = &campaign.facts[fact_id];
        !source_known.contains(fact_id) && !source_known.contains(&fact.statement)
    });
    Ok(facts)
}

fn communicable_facts(
    campaign: &Campaign,
    proposal: &CellActionProposal,
) -> Result<Vec<CommunicableFactContext>> {
    let (activities, targets, _) = activity_parts(proposal)?;
    if !activities.contains(&StrategicActivityKind::Communicate) {
        return Ok(Vec::new());
    }
    let source_known = source_knowledge(campaign, &proposal.subject_id)?;
    let mut facts = Vec::new();
    for fact in campaign
        .facts
        .values()
        .filter(|fact| source_known.contains(&fact.id) || source_known.contains(&fact.statement))
    {
        let mut recipient_subject_ids = Vec::new();
        for target in &targets {
            if is_human_controlled_actor(campaign, target) || !can_hold_knowledge(campaign, target)
            {
                continue;
            }
            let target_known = source_knowledge(campaign, target)?;
            if !target_known.contains(&fact.id) && !target_known.contains(&fact.statement) {
                recipient_subject_ids.push(target.clone());
            }
        }
        if !recipient_subject_ids.is_empty() {
            facts.push(CommunicableFactContext {
                fact_id: fact.id.clone(),
                statement: fact.statement.clone(),
                recipient_subject_ids,
            });
        }
    }
    Ok(facts)
}

fn can_hold_knowledge(campaign: &Campaign, subject_id: &str) -> bool {
    campaign.actors.contains_key(subject_id)
        || campaign.gestalts.contains_key(subject_id)
        || crate::resolution::dormant_member_id_for_subject(campaign, subject_id).is_some()
}

fn source_knowledge(campaign: &Campaign, subject_id: &str) -> Result<BTreeSet<String>> {
    if let Some(gestalt) = campaign.gestalts.get(subject_id) {
        return Ok(gestalt.shared_knowledge.clone());
    }
    if let Some(actor) = campaign.actors.get(subject_id) {
        return Ok(actor.knowledge.clone());
    }
    if let Some(member_id) = crate::resolution::dormant_member_id_for_subject(campaign, subject_id)
    {
        return effective_member_knowledge(campaign, member_id);
    }
    Ok(BTreeSet::new())
}

pub fn subject_resources(campaign: &Campaign, subject_id: &str) -> Result<BTreeSet<String>> {
    if let Some(actor) = campaign.actors.get(subject_id) {
        return Ok(actor.equipment.clone());
    }
    if let Some(member_id) = crate::resolution::dormant_member_id_for_subject(campaign, subject_id)
    {
        return campaign
            .gestalt_members
            .get(member_id)
            .map(|member| member.equipment.clone())
            .ok_or_else(|| anyhow!("resource owner member is unknown"));
    }
    if let Some(gestalt) = campaign.gestalts.get(subject_id) {
        return Ok(gestalt.resources.clone());
    }
    if let Some(institution) = campaign.institutions.get(subject_id) {
        return Ok(institution.resources.iter().cloned().collect());
    }
    if let Some(location) = campaign.locations.get(subject_id) {
        return Ok(location.persistent_features.iter().cloned().collect());
    }
    Err(anyhow!("resource owner is unknown"))
}

pub fn can_hold_resources(campaign: &Campaign, subject_id: &str) -> bool {
    subject_id
        .strip_prefix("member:")
        .is_some_and(|member_id| campaign.gestalt_members.contains_key(member_id))
        || campaign.gestalts.contains_key(subject_id)
        || campaign.institutions.contains_key(subject_id)
        || campaign.actors.contains_key(subject_id)
        || campaign.locations.contains_key(subject_id)
}

fn is_human_controlled_actor(campaign: &Campaign, subject_id: &str) -> bool {
    subject_id == campaign.player_actor_id
        || campaign
            .agency_profiles
            .get(subject_id)
            .is_some_and(|profile| !profile.simulation_eligible)
}

fn activity_parts(
    proposal: &CellActionProposal,
) -> Result<(Vec<StrategicActivityKind>, Vec<String>, Vec<String>)> {
    let activity_effects = proposal
        .effects
        .iter()
        .filter(|effect| is_activity_effect(effect))
        .collect::<Vec<_>>();
    if activity_effects.is_empty() {
        return Err(anyhow!(
            "strategic outcome was requested for a non-activity"
        ));
    }
    let mut activities = Vec::with_capacity(activity_effects.len());
    let mut targets = BTreeSet::new();
    let mut locations = BTreeSet::new();
    for effect in activity_effects {
        let (activity, effect_targets, effect_locations) = match effect {
            StrategicCellEffect::GestaltActivity {
                activity,
                target_subject_ids,
                location_ids,
                ..
            }
            | StrategicCellEffect::ActorActivity {
                activity,
                target_subject_ids,
                location_ids,
                ..
            }
            | StrategicCellEffect::MemberActivity {
                activity,
                target_subject_ids,
                location_ids,
                ..
            } => (activity, target_subject_ids, location_ids),
            _ => unreachable!("activity filter admitted a non-activity"),
        };
        if !activities.contains(activity) {
            activities.push(activity.clone());
        }
        targets.extend(effect_targets.iter().cloned());
        locations.extend(effect_locations.iter().cloned());
    }
    Ok((
        activities,
        targets.into_iter().collect(),
        locations.into_iter().collect(),
    ))
}

fn is_activity_effect(effect: &StrategicCellEffect) -> bool {
    matches!(
        effect,
        StrategicCellEffect::GestaltActivity { .. }
            | StrategicCellEffect::ActorActivity { .. }
            | StrategicCellEffect::MemberActivity { .. }
    )
}

fn subject_name(campaign: &Campaign, subject_id: &str) -> Result<String> {
    campaign
        .actors
        .get(subject_id)
        .map(|value| value.name.clone())
        .or_else(|| {
            campaign
                .institutions
                .get(subject_id)
                .map(|value| value.name.clone())
        })
        .or_else(|| {
            campaign
                .gestalts
                .get(subject_id)
                .map(|value| value.name.clone())
        })
        .or_else(|| {
            crate::resolution::dormant_member_id_for_subject(campaign, subject_id).and_then(
                |member_id| {
                    campaign
                        .gestalt_members
                        .get(member_id)
                        .map(|value| value.name.clone())
                },
            )
        })
        .or_else(|| {
            campaign
                .locations
                .get(subject_id)
                .map(|value| value.name.clone())
        })
        .ok_or_else(|| anyhow!("outcome source vanished"))
}

fn outcome_source_receipts(campaign: &Campaign, proposal: &CellActionProposal) -> Vec<String> {
    let mut ids = Vec::new();
    let mut subjects = vec![proposal.subject_id.clone()];
    if let Ok((_, targets, _)) = activity_parts(proposal) {
        subjects.extend(targets);
    }
    for subject in subjects {
        let profile_id = crate::resolution::dormant_member_id_for_subject(campaign, &subject)
            .and_then(|member_id| campaign.gestalt_members.get(member_id))
            .map(|member| member.gestalt_id.as_str())
            .unwrap_or(&subject);
        if let Some(profile) = campaign.agency_profiles.get(profile_id) {
            ids.extend(profile.evidence_receipt_ids.clone());
        }
    }
    ids
}

fn constrain_outcome_schema(
    schema: &mut serde_json::Value,
    actions: &[ActionOutcomeContext],
) -> Result<()> {
    {
        let outcomes = schema
            .pointer_mut("/properties/outcomes")
            .ok_or_else(|| anyhow!("strategic outcome schema has no outcomes property"))?;
        outcomes["minItems"] = serde_json::json!(actions.len());
        outcomes["maxItems"] = serde_json::json!(actions.len());
    }
    let proposal = schema
        .pointer_mut("/$defs/OutcomeProposal")
        .ok_or_else(|| anyhow!("strategic outcome schema has no proposal definition"))?;
    for field in [
        "pressure_additions",
        "pressure_resolutions",
        "recipient_subject_ids",
    ] {
        let array_schema = proposal["properties"][field].take();
        proposal["properties"][field] = serde_json::json!({
            "anyOf":[array_schema,{"type":"null"}]
        });
    }
    proposal["required"] = serde_json::json!([
        "action_digest",
        "band",
        "effect_kind",
        "supporting_state_references"
    ]);
    proposal["properties"]["action_digest"] = exact_string_value_schema(
        &actions
            .iter()
            .map(|action| action.action_digest.clone())
            .collect::<Vec<_>>(),
    );
    let mut constraints = vec![serde_json::json!({
        "oneOf":outcome_effect_shape_schemas()
    })];
    constraints.extend(actions.iter().map(|action| {
        serde_json::json!({
            "if":{
                "properties":{"action_digest":{"const":action.action_digest}},
                "required":["action_digest"]
            },
            "then":outcome_action_scope_schema(action)
        })
    }));
    proposal["allOf"] = serde_json::json!(constraints);
    Ok(())
}

fn outcome_effect_shape_schemas() -> Vec<serde_json::Value> {
    vec![
        outcome_effect_shape(OutcomeEffectKind::NoMaterialChange, &["reason"], false),
        outcome_effect_shape(
            OutcomeEffectKind::ResourceCreated,
            &["owner_subject_id", "resource"],
            true,
        ),
        outcome_effect_shape(
            OutcomeEffectKind::ResourceConsumed,
            &["owner_subject_id", "resource"],
            true,
        ),
        outcome_effect_shape(
            OutcomeEffectKind::ResourceTransferred,
            &["owner_subject_id", "other_subject_id", "resource"],
            true,
        ),
        outcome_effect_shape(
            OutcomeEffectKind::GestaltPressure,
            &[
                "owner_subject_id",
                "pressure_additions",
                "pressure_resolutions",
            ],
            true,
        ),
        outcome_effect_shape(
            OutcomeEffectKind::AgencyRelationShift,
            &["relation_id", "strength_delta"],
            true,
        ),
        outcome_effect_shape(
            OutcomeEffectKind::MemberMemory,
            &["member_id", "memory"],
            true,
        ),
        outcome_effect_shape(
            OutcomeEffectKind::MemberObligation,
            &["member_id", "obligation"],
            true,
        ),
        outcome_effect_shape(
            OutcomeEffectKind::MemberRelationship,
            &["member_id", "other_subject_id", "relationship_description"],
            true,
        ),
        outcome_effect_shape(
            OutcomeEffectKind::KnowledgeLearned,
            &["owner_subject_id", "fact_id"],
            true,
        ),
        outcome_effect_shape(
            OutcomeEffectKind::KnowledgeCommunicated,
            &["owner_subject_id", "recipient_subject_ids", "fact_id"],
            true,
        ),
    ]
}

fn outcome_effect_shape(
    effect_kind: OutcomeEffectKind,
    required_effect_fields: &[&str],
    material: bool,
) -> serde_json::Value {
    const EFFECT_FIELDS: [&str; 14] = [
        "owner_subject_id",
        "other_subject_id",
        "relation_id",
        "strength_delta",
        "resource",
        "pressure_additions",
        "pressure_resolutions",
        "member_id",
        "memory",
        "obligation",
        "relationship_description",
        "fact_id",
        "recipient_subject_ids",
        "reason",
    ];
    let forbidden = EFFECT_FIELDS
        .iter()
        .filter(|field| !required_effect_fields.contains(field))
        .map(|field| {
            if matches!(
                *field,
                "pressure_additions" | "pressure_resolutions" | "recipient_subject_ids"
            ) {
                serde_json::json!({
                    "required":[field],
                    "properties":{(*field):{"type":"array","minItems":1}}
                })
            } else {
                serde_json::json!({
                    "required":[field],
                    "properties":{(*field):{"not":{"type":"null"}}}
                })
            }
        })
        .collect::<Vec<_>>();
    let mut required = vec!["effect_kind"];
    required.extend_from_slice(required_effect_fields);
    let mut shape = serde_json::json!({
        "properties":{
            "effect_kind":{"const":effect_kind},
            "supporting_state_references":if material {
                serde_json::json!({"minItems":1,"maxItems":8})
            } else {
                serde_json::json!({"maxItems":0})
            }
        },
        "required":required
    });
    for field in required_effect_fields.iter() {
        shape["properties"][*field] = if matches!(
            *field,
            "pressure_additions" | "pressure_resolutions" | "recipient_subject_ids"
        ) {
            serde_json::json!({"type":"array"})
        } else {
            serde_json::json!({"not":{"type":"null"}})
        };
    }
    if !forbidden.is_empty() {
        shape["not"] = serde_json::json!({"anyOf":forbidden});
    }
    shape
}

fn outcome_action_scope_schema(action: &ActionOutcomeContext) -> serde_json::Value {
    let mut constraints = Vec::new();
    let admits = |kind: &OutcomeEffectKind| action.admissible_effect_kinds.contains(kind);
    let source_resources = action
        .source_state
        .get("resources")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(serde_json::Value::as_str)
        .map(str::to_owned)
        .collect::<Vec<_>>();
    let admitted_existing_resource_kinds = [
        OutcomeEffectKind::ResourceConsumed,
        OutcomeEffectKind::ResourceTransferred,
    ]
    .into_iter()
    .filter(|kind| admits(kind))
    .collect::<Vec<_>>();
    if !admitted_existing_resource_kinds.is_empty() {
        constraints.push(effect_scope_condition(
            &admitted_existing_resource_kinds,
            serde_json::json!({
                "properties":{"owner_subject_id":{"const":action.resource_owner_id}}
            }),
        ));
    }
    if admits(&OutcomeEffectKind::ResourceCreated) {
        constraints.push(effect_scope_condition(
            &[OutcomeEffectKind::ResourceCreated],
            serde_json::json!({
                "properties":{
                    "owner_subject_id":exact_string_value_schema(&action.resource_creation_owner_ids),
                    "resource":{"type":"string","minLength":1,"maxLength":160}
                }
            }),
        ));
    }
    if !admitted_existing_resource_kinds.is_empty() {
        constraints.push(effect_scope_condition(
            &admitted_existing_resource_kinds,
            serde_json::json!({
                "properties":{"resource":exact_string_value_schema(&source_resources)}
            }),
        ));
    }
    if admits(&OutcomeEffectKind::ResourceTransferred) {
        constraints.push(effect_scope_condition(
            &[OutcomeEffectKind::ResourceTransferred],
            serde_json::json!({
                "properties":{"other_subject_id":exact_string_value_schema(&action.resource_recipient_ids)}
            }),
        ));
    }
    if admits(&OutcomeEffectKind::GestaltPressure) {
        let pressure_owner_constraints = action
            .pressure_owners
            .iter()
            .map(|owner| {
                serde_json::json!({
                    "if":{
                        "properties":{"owner_subject_id":{"const":owner.owner_subject_id}},
                        "required":["owner_subject_id"]
                    },
                    "then":{
                        "properties":{
                            "pressure_additions":{
                                "type":"array","uniqueItems":true,"maxItems":4,
                                "items":{
                                    "type":"string","minLength":1,"maxLength":240
                                }
                            },
                            "pressure_resolutions":exact_string_array_value_schema(
                                &owner.current_pressures,
                                0,
                                owner.current_pressures.len().min(4)
                            )
                        },
                        "anyOf":[
                            {"properties":{"pressure_additions":{"minItems":1}}},
                            {"properties":{"pressure_resolutions":{"minItems":1}}}
                        ]
                    }
                })
            })
            .collect::<Vec<_>>();
        let pressure_owner_ids = action
            .pressure_owners
            .iter()
            .map(|owner| owner.owner_subject_id.clone())
            .collect::<Vec<_>>();
        constraints.push(effect_scope_condition(
            &[OutcomeEffectKind::GestaltPressure],
            serde_json::json!({
                "properties":{
                    "owner_subject_id":exact_string_value_schema(&pressure_owner_ids)
                },
                "allOf":pressure_owner_constraints
            }),
        ));
    }
    let relation_ids = action
        .active_relations
        .iter()
        .filter_map(|relation| relation.get("relation_id"))
        .filter_map(serde_json::Value::as_str)
        .map(str::to_owned)
        .collect::<Vec<_>>();
    if admits(&OutcomeEffectKind::AgencyRelationShift) {
        constraints.push(effect_scope_condition(
            &[OutcomeEffectKind::AgencyRelationShift],
            serde_json::json!({
                "properties":{
                    "relation_id":exact_string_value_schema(&relation_ids),
                    "strength_delta":{"type":"integer","enum":[-10,-9,-8,-7,-6,-5,-4,-3,-2,-1,1,2,3,4,5,6,7,8,9,10]}
                }
            }),
        ));
    }
    let admitted_member_kinds = [
        OutcomeEffectKind::MemberMemory,
        OutcomeEffectKind::MemberObligation,
        OutcomeEffectKind::MemberRelationship,
    ]
    .into_iter()
    .filter(|kind| admits(kind))
    .collect::<Vec<_>>();
    if !admitted_member_kinds.is_empty() {
        if let Some(member_id) = &action.member_state_owner_id {
            constraints.push(effect_scope_condition(
                &admitted_member_kinds,
                serde_json::json!({
                    "properties":{"member_id":{"const":member_id}}
                }),
            ));
        }
    }
    if admits(&OutcomeEffectKind::MemberMemory) {
        constraints.push(effect_scope_condition(
            &[OutcomeEffectKind::MemberMemory],
            serde_json::json!({"properties":{"memory":{"type":"string","minLength":1,"maxLength":240}}}),
        ));
    }
    if admits(&OutcomeEffectKind::MemberObligation) {
        constraints.push(effect_scope_condition(
            &[OutcomeEffectKind::MemberObligation],
            serde_json::json!({"properties":{"obligation":{"type":"string","minLength":1,"maxLength":240}}}),
        ));
    }
    if admits(&OutcomeEffectKind::MemberRelationship) {
        constraints.push(effect_scope_condition(
            &[OutcomeEffectKind::MemberRelationship],
            serde_json::json!({
                "properties":{
                    "other_subject_id":exact_string_value_schema(&action.target_subject_ids),
                    "relationship_description":{"type":"string","minLength":1,"maxLength":240}
                }
            }),
        ));
    }
    let fact_ids = action
        .discoverable_facts
        .iter()
        .filter_map(|fact| fact.get("fact_id"))
        .filter_map(serde_json::Value::as_str)
        .map(str::to_owned)
        .collect::<Vec<_>>();
    if admits(&OutcomeEffectKind::KnowledgeLearned) {
        constraints.push(effect_scope_condition(
            &[OutcomeEffectKind::KnowledgeLearned],
            serde_json::json!({
                "properties":{
                    "owner_subject_id":{"const":action.source_subject_id},
                    "fact_id":exact_string_value_schema(&fact_ids)
                }
            }),
        ));
    }
    if admits(&OutcomeEffectKind::KnowledgeCommunicated) {
        let fact_constraints = action
            .communicable_facts
            .iter()
            .map(|fact| {
                serde_json::json!({
                    "if":{
                        "properties":{"fact_id":{"const":fact.fact_id}},
                        "required":["fact_id"]
                    },
                    "then":{
                        "properties":{
                            "recipient_subject_ids":exact_string_array_value_schema(
                                &fact.recipient_subject_ids,
                                1,
                                fact.recipient_subject_ids.len().min(4)
                            )
                        }
                    }
                })
            })
            .collect::<Vec<_>>();
        let fact_ids = action
            .communicable_facts
            .iter()
            .map(|fact| fact.fact_id.clone())
            .collect::<Vec<_>>();
        constraints.push(effect_scope_condition(
            &[OutcomeEffectKind::KnowledgeCommunicated],
            serde_json::json!({
                "properties":{
                    "owner_subject_id":{"const":action.source_subject_id},
                    "fact_id":exact_string_value_schema(&fact_ids)
                },
                "allOf":fact_constraints
            }),
        ));
    }
    constraints.push(effect_scope_condition(
        &[OutcomeEffectKind::NoMaterialChange],
        serde_json::json!({"properties":{"reason":{"type":"string","minLength":1,"maxLength":240}}}),
    ));
    serde_json::json!({
        "properties":{
            "action_digest":{"const":action.action_digest},
            "effect_kind":{"enum":action.admissible_effect_kinds},
            "supporting_state_references":exact_string_array_value_schema(
                &action.allowed_state_references,
                0,
                action.allowed_state_references.len().min(8)
            )
        },
        "required":["action_digest"],
        "allOf":constraints
    })
}

fn effect_scope_condition(
    effect_kinds: &[OutcomeEffectKind],
    consequence: serde_json::Value,
) -> serde_json::Value {
    serde_json::json!({
        "if":{
            "properties":{"effect_kind":{"enum":effect_kinds}},
            "required":["effect_kind"]
        },
        "then":consequence
    })
}

fn exact_string_value_schema(values: &[String]) -> serde_json::Value {
    serde_json::json!({"type":"string","enum":values})
}

fn exact_string_array_value_schema(
    values: &[String],
    min_items: usize,
    max_items: usize,
) -> serde_json::Value {
    if values.is_empty() {
        return serde_json::json!({
            "type":"array",
            "minItems":min_items,
            "maxItems":0
        });
    }
    serde_json::json!({
        "type":"array",
        "uniqueItems":true,
        "minItems":min_items,
        "maxItems":max_items,
        "items":exact_string_value_schema(values)
    })
}

fn constrain_verifier_schema(schema: &mut serde_json::Value, digests: &[String]) -> Result<()> {
    let verdicts = schema
        .pointer_mut("/properties/verdicts")
        .ok_or_else(|| anyhow!("strategic outcome verifier schema has no verdicts property"))?;
    verdicts["minItems"] = serde_json::json!(digests.len());
    verdicts["maxItems"] = serde_json::json!(digests.len());
    let verdict = schema
        .pointer_mut("/$defs/OutcomeVerifierVerdict")
        .ok_or_else(|| anyhow!("strategic outcome verifier schema has no verdict definition"))?;
    *verdict = serde_json::json!({
        "oneOf":[
            {
                "type":"object",
                "additionalProperties":false,
                "required":["action_digest", "result", "repair_guidance"],
                "properties":{
                    "action_digest":{"enum":digests},
                    "result":{"const":"match"},
                    "repair_guidance":{"type":"null"}
                }
            },
            {
                "type":"object",
                "additionalProperties":false,
                "required":["action_digest", "result", "repair_guidance"],
                "properties":{
                    "action_digest":{"enum":digests},
                    "result":{"const":"mismatch"},
                    "repair_guidance":{
                        "type":"string",
                        "minLength":1
                    }
                }
            }
        ]
    });
    Ok(())
}

fn required(value: &Option<String>, field: &str) -> Result<String> {
    value
        .clone()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| anyhow!("outcome omitted {field}"))
}

fn bounded_text(value: &str, max: usize) -> bool {
    !value.trim().is_empty() && value.chars().count() <= max
}

fn contains_normalized(values: &BTreeSet<String>, candidate: &str) -> bool {
    values
        .iter()
        .any(|value| value.eq_ignore_ascii_case(candidate))
}

fn contains_normalized_slice(values: &[String], candidate: &str) -> bool {
    values
        .iter()
        .any(|value| value.eq_ignore_ascii_case(candidate))
}

fn valid_sha256(value: &str) -> bool {
    value
        .strip_prefix("sha256:")
        .is_some_and(|hex| hex.len() == 64 && hex.bytes().all(|byte| byte.is_ascii_hexdigit()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        domain::{
            AgencyProfile, AgencyRelation, AgencyRelationKind, AgencySubjectKind, BranchOrigin,
            GestaltPersonaState, Location, ResolutionPolicy, WorldFact,
        },
        resolution::ensure_agency_profiles,
    };
    use chrono::Utc;
    use std::sync::Mutex;
    use uuid::Uuid;

    struct OutcomeFixtureModel {
        action_digest: String,
        requests: Mutex<Vec<ModelStageRequest>>,
    }

    struct RejectingVerifierModel {
        action_digest: String,
    }

    struct CorrectingOutcomeModel {
        action_digest: String,
        resolver_calls: Mutex<usize>,
    }

    struct CorrectingKnowledgeModel {
        action_digest: String,
        resolver_calls: Mutex<usize>,
        requests: Mutex<Vec<ModelStageRequest>>,
    }

    struct RepeatingPressureModel {
        action_digest: String,
        requests: Mutex<Vec<ModelStageRequest>>,
    }

    struct IsolatingOutcomeModel {
        requests: Mutex<Vec<ModelStageRequest>>,
    }

    #[async_trait::async_trait]
    impl ModelPort for IsolatingOutcomeModel {
        async fn run(&self, request: &ModelStageRequest) -> Result<String> {
            assert_eq!(request.stage, "strategic_outcome_resolver");
            self.requests.lock().unwrap().push(request.clone());
            let (_, payload) = request
                .lived_stream
                .rsplit_once("OUTCOME_CONTEXT:\n")
                .ok_or_else(|| anyhow!("fixture request omitted outcome context"))?;
            let context: serde_json::Value = serde_json::from_str(payload)?;
            let actions = context["actions"]
                .as_array()
                .ok_or_else(|| anyhow!("fixture context omitted actions"))?;
            if actions.len() != 1 {
                return Err(anyhow!(
                    "unrelated strategic actions shared one resolver context"
                ));
            }
            let digest = actions[0]["action_digest"]
                .as_str()
                .ok_or_else(|| anyhow!("fixture action omitted digest"))?;
            Ok(serde_json::json!({
                "outcomes":[{
                    "action_digest":digest,
                    "band":"mixed",
                    "effect_kind":"no_material_change",
                    "supporting_state_references":[],
                    "reason":"This exact attempt establishes no durable change."
                }]
            })
            .to_string())
        }

        fn provider(&self) -> &'static str {
            "isolating-outcome-model"
        }
    }

    #[async_trait::async_trait]
    impl ModelPort for RepeatingPressureModel {
        async fn run(&self, request: &ModelStageRequest) -> Result<String> {
            let mut requests = self.requests.lock().unwrap();
            requests.push(request.clone());
            if requests.len() == 1 {
                return Ok(serde_json::json!({
                    "outcomes":[{
                        "action_digest":self.action_digest,
                        "band":"mixed",
                        "effect_kind":"gestalt_pressure",
                        "supporting_state_references":["pressure:dockers:storm damage"],
                        "owner_subject_id":"dockers",
                        "pressure_additions":["storm damage"],
                        "pressure_resolutions":[]
                    }]
                })
                .to_string());
            }
            Ok(serde_json::json!({
                "outcomes":[{
                    "action_digest":self.action_digest,
                    "band":"mixed",
                    "effect_kind":"no_material_change",
                    "supporting_state_references":[],
                    "reason":"The preparation does not yet establish a new durable pressure or resolve the existing storm damage."
                }]
            })
            .to_string())
        }

        fn provider(&self) -> &'static str {
            "repeating-pressure-model"
        }
    }

    #[async_trait::async_trait]
    impl ModelPort for CorrectingOutcomeModel {
        async fn run(&self, request: &ModelStageRequest) -> Result<String> {
            if request.stage == "strategic_outcome_verifier" {
                let rejects_consumption = request
                    .lived_stream
                    .contains("\"type\":\"resource_consumed\"");
                return Ok(serde_json::json!({
                    "verdicts":[{
                        "action_digest":self.action_digest,
                        "result":if rejects_consumption {"mismatch"} else {"match"},
                        "repair_guidance":if rejects_consumption {
                            Some("The inspection does not use spare rope; choose no_material_change.")
                        } else {
                            None
                        }
                    }]
                })
                .to_string());
            }
            let mut calls = self.resolver_calls.lock().unwrap();
            *calls += 1;
            if *calls == 1 {
                return Ok(serde_json::json!({
                    "outcomes":[{
                        "action_digest":self.action_digest,
                        "band":"mixed",
                        "effect_kind":"resource_consumed",
                        "supporting_state_references":["resource:spare rope"],
                        "owner_subject_id":"dockers",
                        "resource":"spare rope"
                    }]
                })
                .to_string());
            }
            Ok(serde_json::json!({
                "outcomes":[{
                    "action_digest":self.action_digest,
                    "band":"mixed",
                    "effect_kind":"no_material_change",
                    "supporting_state_references":[],
                    "reason":"The inspection does not use or alter any durable resource."
                }]
            })
            .to_string())
        }

        fn provider(&self) -> &'static str {
            "correcting-outcome-model"
        }
    }

    #[async_trait::async_trait]
    impl ModelPort for CorrectingKnowledgeModel {
        async fn run(&self, request: &ModelStageRequest) -> Result<String> {
            self.requests.lock().unwrap().push(request.clone());
            if request.stage == "strategic_outcome_verifier" {
                let rejects_uncommunicated_fact = request
                    .lived_stream
                    .contains("\"type\":\"knowledge_learned\"");
                return Ok(serde_json::json!({
                    "verdicts":[{
                        "action_digest":self.action_digest,
                        "result":if rejects_uncommunicated_fact {"mismatch"} else {"match"},
                        "repair_guidance":if rejects_uncommunicated_fact {
                            Some("The attempt does not observe, investigate, or communicate the proposed fact; choose no_material_change.")
                        } else {
                            None
                        }
                    }]
                })
                .to_string());
            }
            let mut calls = self.resolver_calls.lock().unwrap();
            *calls += 1;
            if *calls == 1 {
                return Ok(serde_json::json!({
                    "outcomes":[{
                        "action_digest":self.action_digest,
                        "band":"mixed",
                        "effect_kind":"knowledge_learned",
                        "supporting_state_references":["fact:fact:safe-route"],
                        "owner_subject_id":"dockers",
                        "fact_id":"fact:safe-route"
                    }]
                })
                .to_string());
            }
            Ok(serde_json::json!({
                "outcomes":[{
                    "action_digest":self.action_digest,
                    "band":"mixed",
                    "effect_kind":"no_material_change",
                    "supporting_state_references":[],
                    "reason":"The coordination attempt does not establish or communicate a durable fact."
                }]
            })
            .to_string())
        }

        fn provider(&self) -> &'static str {
            "correcting-knowledge-model"
        }
    }

    #[async_trait::async_trait]
    impl ModelPort for RejectingVerifierModel {
        async fn run(&self, request: &ModelStageRequest) -> Result<String> {
            assert_eq!(request.stage, "strategic_outcome_verifier");
            Ok(serde_json::json!({
                "verdicts":[{
                    "action_digest":self.action_digest,
                    "result":"mismatch",
                    "repair_guidance":"The inspection never uses or expends spare rope; choose no_material_change."
                }]
            })
            .to_string())
        }

        fn provider(&self) -> &'static str {
            "rejecting-outcome-verifier"
        }
    }

    #[async_trait::async_trait]
    impl ModelPort for OutcomeFixtureModel {
        async fn run(&self, request: &ModelStageRequest) -> Result<String> {
            self.requests.lock().unwrap().push(request.clone());
            if request.stage == "strategic_outcome_verifier" {
                return Ok(serde_json::json!({
                    "verdicts":[{
                        "action_digest":self.action_digest,
                        "result":"match",
                        "repair_guidance":null
                    }]
                })
                .to_string());
            }
            Ok(serde_json::json!({
                "outcomes":[{
                    "action_digest":self.action_digest,
                    "band":"success",
                    "effect_kind":"knowledge_learned",
                    "supporting_state_references":["fact:fact:safe-route"],
                    "owner_subject_id":"dockers",
                    "fact_id":"fact:safe-route"
                }]
            })
            .to_string())
        }

        fn provider(&self) -> &'static str {
            "outcome-fixture"
        }
    }

    #[test]
    fn member_outcome_text_retains_its_field_specific_bound() {
        let mut exact = BTreeSet::new();
        validate_member_owner(
            "member:mira",
            "mira",
            &"x".repeat(240),
            "memory",
            &mut exact,
        )
        .unwrap();

        let mut oversized = BTreeSet::new();
        assert!(
            validate_member_owner(
                "member:mira",
                "mira",
                &"x".repeat(241),
                "memory",
                &mut oversized,
            )
            .is_err()
        );
    }

    #[test]
    fn outcome_verifier_schema_makes_result_guidance_coherence_structural() {
        let digest = format!("sha256:{}", "a".repeat(64));
        let mut schema = serde_json::to_value(schema_for!(OutcomeVerifierBundle)).unwrap();
        constrain_verifier_schema(&mut schema, std::slice::from_ref(&digest)).unwrap();
        let validator = jsonschema::validator_for(&schema).unwrap();

        assert!(validator.is_valid(&serde_json::json!({
            "verdicts":[{
                "action_digest":digest,
                "result":"match",
                "repair_guidance":null
            }]
        })));
        assert!(validator.is_valid(&serde_json::json!({
            "verdicts":[{
                "action_digest":digest,
                "result":"mismatch",
                "repair_guidance":"The proposed transfer is not causally supported by the exact attempt. Preserve the source subject, exact resource custody, and recipient boundary; if no supplied outcome can express the actual result, replace it with no_material_change instead of inventing an exchange or acceptance that never occurred."
            }]
        })));
        assert!(!validator.is_valid(&serde_json::json!({
            "verdicts":[{
                "action_digest":digest,
                "result":"match",
                "repair_guidance":"This cannot accompany a match."
            }]
        })));
        assert!(!validator.is_valid(&serde_json::json!({
            "verdicts":[{
                "action_digest":digest,
                "result":"mismatch",
                "repair_guidance":null
            }]
        })));
    }

    fn campaign() -> Campaign {
        let mut value = Campaign {
            schema: "ghostlight.campaign.v1".into(),
            id: Uuid::new_v4(),
            name: "outcome-test".into(),
            revision: 4,
            branch_origin: BranchOrigin {
                canon_cutoff: "test".into(),
                evidence_receipt_ids: vec![],
            },
            world_time: Utc::now(),
            tick_hours: 6,
            player_actor_id: "player".into(),
            locations: BTreeMap::from([(
                "dock".into(),
                Location {
                    id: "dock".into(),
                    name: "Dock".into(),
                    container_id: None,
                    routes: BTreeMap::new(),
                    persistent_features: vec![],
                },
            )]),
            actors: BTreeMap::new(),
            institutions: BTreeMap::new(),
            clocks: BTreeMap::new(),
            facts: BTreeMap::from([(
                "fact:safe-route".into(),
                WorldFact {
                    id: "fact:safe-route".into(),
                    statement: "the western causeway remains open".into(),
                    scope: crate::domain::FactScope::BranchLocal,
                    evidence_receipt_ids: vec![],
                    discoverable_at_location_ids: BTreeSet::from(["dock".into()]),
                },
            )]),
            civic_systems: BTreeMap::new(),
            transcript: vec![],
            last_player_activity: Utc::now(),
            pending_ticks: 0,
            away_ticks_processed: 0,
            events: vec![],
            news: vec![],
            canon_candidates: BTreeMap::new(),
            gestalts: BTreeMap::from([(
                "dockers".into(),
                GestaltPersonaState {
                    schema: "ghostlight.gestalt_persona_state.v1".into(),
                    id: "dockers".into(),
                    name: "Dockers".into(),
                    version: 0,
                    home_location_id: "dock".into(),
                    shared_capabilities: BTreeSet::from(["repair nets".into()]),
                    shared_knowledge: BTreeSet::new(),
                    resources: BTreeSet::from(["spare rope".into()]),
                    goals: vec!["keep the harbor working".into()],
                    pressures: vec!["storm damage".into()],
                },
            )]),
            gestalt_members: BTreeMap::new(),
            pending_world_proposals: vec![],
            agency_profiles: BTreeMap::new(),
            agency_relations: BTreeMap::new(),
            gestalt_lineages: BTreeMap::new(),
            resolution_policy: ResolutionPolicy::default(),
            resolution_pins: BTreeMap::new(),
            resolution_cover: None,
            strategic_tick_count: 0,
        };
        value.actors.insert(
            "player".into(),
            crate::domain::ActorState {
                id: "player".into(),
                name: "Player".into(),
                location_id: "dock".into(),
                capabilities: BTreeSet::new(),
                knowledge: BTreeSet::new(),
                equipment: BTreeSet::new(),
                conditions: BTreeSet::new(),
                obligations: BTreeSet::new(),
                relationships: BTreeMap::new(),
                goals: vec![],
                memories: vec![],
            },
        );
        ensure_agency_profiles(&mut value);
        value.agency_profiles.insert(
            "dockers".into(),
            AgencyProfile {
                schema: "ghostlight.agency_profile.v1".into(),
                id: "profile:dockers".into(),
                subject_id: "dockers".into(),
                subject_kind: AgencySubjectKind::Gestalt,
                profile_version: 0,
                collective_authority_id: Some("dockers".into()),
                parent_subject_id: None,
                active_leaf: true,
                simulation_eligible: true,
                facets: BTreeMap::new(),
                location_ids: BTreeSet::from(["dock".into()]),
                information_channels: BTreeSet::new(),
                detail_debt: 0,
                last_detail_tick: 0,
                evidence_receipt_ids: vec![],
            },
        );
        value
    }

    fn proposal() -> CellActionProposal {
        CellActionProposal {
            subject_id: "dockers".into(),
            intent: "inspect the damaged quay".into(),
            intended_effect: "learn whether the western route remains usable".into(),
            priority: 50,
            state_references: vec!["capability:repair nets".into(), "location:dock".into()],
            public_channels: vec![],
            effects: vec![StrategicCellEffect::GestaltActivity {
                gestalt_id: "dockers".into(),
                activity: StrategicActivityKind::Investigate,
                target_subject_ids: vec![],
                location_ids: vec!["dock".into()],
            }],
        }
    }

    #[test]
    fn compound_strategic_means_share_one_digest_and_union_outcome_authority() {
        let value = campaign();
        let mut action = proposal();
        action.effects.push(StrategicCellEffect::GestaltActivity {
            gestalt_id: "dockers".into(),
            activity: StrategicActivityKind::Prepare,
            target_subject_ids: vec![],
            location_ids: vec!["dock".into()],
        });

        let context = action_context(&value, &action).unwrap();
        assert_eq!(
            context.activities,
            vec![
                StrategicActivityKind::Investigate,
                StrategicActivityKind::Prepare,
            ]
        );
        assert_eq!(context.location_ids, ["dock"]);
        let kinds = admissible_effect_kinds(&value, &action).unwrap();
        assert!(kinds.contains(&OutcomeEffectKind::KnowledgeLearned));
        assert!(kinds.contains(&OutcomeEffectKind::ResourceCreated));
        assert_eq!(
            plan_activity_digests(
                &crate::resolution::project_selected_actions(&value, vec![action.clone()],)
                    .unwrap()
            ),
            vec![cell_action_digest(&action).unwrap()]
        );
    }

    #[test]
    fn materialized_member_outcome_context_uses_actor_state_and_effects() {
        let mut value = campaign();
        value.gestalt_members.insert(
            "sable".into(),
            crate::domain::GestaltMemberDelta {
                schema: "ghostlight.gestalt_member_delta.v1".into(),
                id: "sable".into(),
                gestalt_id: "dockers".into(),
                version: 1,
                name: "Sable".into(),
                capability_additions: BTreeSet::new(),
                capability_removals: BTreeSet::new(),
                knowledge_additions: BTreeSet::new(),
                knowledge_removals: BTreeSet::new(),
                equipment: BTreeSet::from(["folded kit".into()]),
                conditions: BTreeSet::new(),
                obligations: BTreeSet::new(),
                relationships: BTreeMap::new(),
                goals: vec![],
                memories: vec![],
                last_location_id: Some("dock".into()),
                materialized_actor_id: Some("member:sable".into()),
                last_relevant_revision: 4,
                relevance_lease_until_revision: 9,
            },
        );
        value.actors.insert(
            "member:sable".into(),
            crate::domain::ActorState {
                id: "member:sable".into(),
                name: "Sable".into(),
                location_id: "dock".into(),
                capabilities: BTreeSet::from(["route scouting".into()]),
                knowledge: BTreeSet::from(["the east gate is watched".into()]),
                equipment: BTreeSet::from(["materialized kit".into()]),
                conditions: BTreeSet::new(),
                obligations: BTreeSet::new(),
                relationships: BTreeMap::new(),
                goals: vec![],
                memories: vec![],
            },
        );
        ensure_agency_profiles(&mut value);
        let action = CellActionProposal {
            subject_id: "member:sable".into(),
            intent: "inspect the eastern trail".into(),
            intended_effect: "identify an immediate route hazard".into(),
            priority: 50,
            state_references: vec![
                "subject:member:sable".into(),
                "location:dock".into(),
                "capability:route scouting".into(),
            ],
            public_channels: vec![],
            effects: vec![StrategicCellEffect::ActorActivity {
                actor_id: "member:sable".into(),
                activity: StrategicActivityKind::Investigate,
                target_subject_ids: vec![],
                location_ids: vec!["dock".into()],
            }],
        };

        let context = build_context(&value, &[action]).unwrap();
        let action = &context.actions[0];
        assert_eq!(action.source_state["kind"], "actor");
        assert_eq!(action.source_state["resources"][0], "materialized kit");
        assert_eq!(action.member_state_owner_id, None);
        assert!(
            action
                .allowed_state_references
                .contains(&"subject:member:sable".into())
        );
        assert!(
            !action
                .admissible_effect_kinds
                .contains(&OutcomeEffectKind::MemberMemory)
        );
        assert!(
            !action
                .admissible_effect_kinds
                .contains(&OutcomeEffectKind::MemberObligation)
        );
        assert!(
            !action
                .admissible_effect_kinds
                .contains(&OutcomeEffectKind::MemberRelationship)
        );
    }

    #[test]
    fn knowledge_outcome_requires_exact_discoverable_fact() {
        let value = campaign();
        let action = proposal();
        let digest = cell_action_digest(&action).unwrap();
        let outcome = StrategicActivityOutcome {
            schema: "ghostlight.strategic_activity_outcome.v1".into(),
            action_digest: digest,
            source_subject_id: "dockers".into(),
            band: StrategicOutcomeBand::Success,
            summary: "The inspection confirms the causeway is open.".into(),
            supporting_state_references: vec!["fact:fact:safe-route".into()],
            effect: StrategicOutcomeEffect::KnowledgeLearned {
                owner_subject_id: "dockers".into(),
                fact_id: "fact:safe-route".into(),
            },
        };
        validate_activity_outcomes(&value, &[action], &[outcome]).unwrap();
    }

    #[test]
    fn communication_projects_and_commits_only_source_known_facts_to_exact_nonhuman_targets() {
        let mut value = campaign();
        let statement = value.facts["fact:safe-route"].statement.clone();
        value.facts.insert(
            "fact:player-secret".into(),
            WorldFact {
                id: "fact:player-secret".into(),
                statement: "the player alone knows the sealed passphrase".into(),
                scope: crate::domain::FactScope::BranchLocal,
                evidence_receipt_ids: vec![],
                discoverable_at_location_ids: BTreeSet::new(),
            },
        );
        value
            .actors
            .get_mut("player")
            .unwrap()
            .knowledge
            .insert("the player alone knows the sealed passphrase".into());
        value
            .gestalts
            .get_mut("dockers")
            .unwrap()
            .shared_knowledge
            .insert(statement.clone());
        value.actors.insert(
            "reed".into(),
            crate::domain::ActorState {
                id: "reed".into(),
                name: "Reed".into(),
                location_id: "dock".into(),
                capabilities: BTreeSet::new(),
                knowledge: BTreeSet::new(),
                equipment: BTreeSet::new(),
                conditions: BTreeSet::new(),
                obligations: BTreeSet::new(),
                relationships: BTreeMap::new(),
                goals: vec![],
                memories: vec![],
            },
        );
        ensure_agency_profiles(&mut value);
        let action = CellActionProposal {
            subject_id: "dockers".into(),
            intent: "warn Reed that the western causeway remains open".into(),
            intended_effect: "tell Reed the exact route fact".into(),
            priority: 80,
            state_references: vec![format!("knowledge:{statement}"), "location:dock".into()],
            public_channels: vec![],
            effects: vec![StrategicCellEffect::GestaltActivity {
                gestalt_id: "dockers".into(),
                activity: StrategicActivityKind::Communicate,
                target_subject_ids: vec!["reed".into(), "player".into()],
                location_ids: vec!["dock".into()],
            }],
        };
        let context = action_context(&value, &action).unwrap();
        assert!(
            context
                .admissible_effect_kinds
                .contains(&OutcomeEffectKind::KnowledgeCommunicated)
        );
        assert_eq!(context.communicable_facts.len(), 1);
        assert!(
            context
                .discoverable_facts
                .iter()
                .all(|fact| fact["fact_id"] != "fact:player-secret")
        );
        assert_eq!(
            context.communicable_facts[0].recipient_subject_ids,
            vec!["reed".to_string()]
        );
        let mut schema = serde_json::to_value(schema_for!(OutcomeProposalBundle)).unwrap();
        constrain_outcome_schema(&mut schema, std::slice::from_ref(&context)).unwrap();
        let validator = jsonschema::validator_for(&schema).unwrap();
        let proposed = |recipient: &str| {
            serde_json::json!({
                "outcomes":[{
                    "action_digest":context.action_digest,
                    "band":"success",
                    "effect_kind":"knowledge_communicated",
                    "supporting_state_references":["fact:fact:safe-route"],
                    "owner_subject_id":"dockers",
                    "recipient_subject_ids":[recipient],
                    "fact_id":"fact:safe-route"
                }]
            })
        };
        assert!(validator.is_valid(&proposed("reed")));
        assert!(!validator.is_valid(&proposed("player")));
        let outcome = StrategicActivityOutcome {
            schema: "ghostlight.strategic_activity_outcome.v1".into(),
            action_digest: cell_action_digest(&action).unwrap(),
            source_subject_id: "dockers".into(),
            band: StrategicOutcomeBand::Success,
            summary: "Dockers shares this with Reed: the western causeway remains open.".into(),
            supporting_state_references: vec!["fact:fact:safe-route".into()],
            effect: StrategicOutcomeEffect::KnowledgeCommunicated {
                from_subject_id: "dockers".into(),
                to_subject_ids: vec!["reed".into()],
                fact_id: "fact:safe-route".into(),
            },
        };
        validate_activity_outcomes(
            &value,
            std::slice::from_ref(&action),
            std::slice::from_ref(&outcome),
        )
        .unwrap();
        let mut plan = crate::resolution::project_selected_actions(&value, vec![action]).unwrap();
        plan.activity_outcomes = vec![outcome];
        let lowered = crate::legacy_transition::lower_strategic_wave(
            &value,
            &plan,
            "test:knowledge-communication",
            Utc::now() + chrono::Duration::minutes(5),
        )
        .unwrap()
        .unwrap();
        assert!(lowered.batch.mutations.iter().any(|permitted| {
            matches!(
                &permitted.mutation,
                crate::transition::WorldMutation::ChangeKnowledge {
                    operation: crate::transition::KnowledgeMutationOperation::Communicate,
                    ..
                }
            )
        }));
        crate::legacy_transition::apply_lowered_transition(&mut value, &lowered, Utc::now())
            .unwrap();
        assert!(value.actors["reed"].knowledge.contains(&statement));
        assert!(!value.actors["player"].knowledge.contains(&statement));
    }

    #[test]
    fn missing_outcome_is_rejected() {
        let value = campaign();
        assert!(validate_activity_outcomes(&value, &[proposal()], &[]).is_err());
    }

    #[test]
    fn outcome_schema_binds_required_fields_and_exact_action_authority() {
        let value = campaign();
        let action = proposal();
        let context = build_context(&value, std::slice::from_ref(&action)).unwrap();
        let digest = context.actions[0].action_digest.clone();
        let mut schema = serde_json::to_value(schema_for!(OutcomeProposalBundle)).unwrap();
        constrain_outcome_schema(&mut schema, &context.actions).unwrap();
        assert!(
            schema
                .pointer("/properties/outcomes/items/properties")
                .is_none(),
            "runtime constraints belong to the referenced proposal definition"
        );
        assert!(
            schema
                .pointer("/$defs/OutcomeProposal/properties/band")
                .is_some(),
            "every required field must remain declared on the constrained definition"
        );
        let validator = jsonschema::validator_for(&schema).unwrap();
        let outcome = |owner: Option<&str>, extra: Option<(&str, serde_json::Value)>| {
            let mut item = serde_json::json!({
                "action_digest":digest,
                "band":"success",
                "effect_kind":"knowledge_learned",
                "supporting_state_references":["fact:fact:safe-route"],
                "fact_id":"fact:safe-route"
            });
            if let Some(owner) = owner {
                item["owner_subject_id"] = serde_json::json!(owner);
            }
            if let Some((field, value)) = extra {
                item[field] = value;
            }
            serde_json::json!({"outcomes":[item]})
        };

        assert!(validator.is_valid(&outcome(Some("dockers"), None)));
        assert!(validator.is_valid(&serde_json::json!({
            "outcomes":[{
                "action_digest":digest,
                "band":"mixed",
                "effect_kind":"no_material_change",
                "supporting_state_references":[],
                "owner_subject_id":null,
                "other_subject_id":null,
                "relation_id":null,
                "strength_delta":null,
                "resource":null,
                "pressure_additions":[],
                "pressure_resolutions":[],
                "member_id":null,
                "memory":null,
                "obligation":null,
                "relationship_description":null,
                "fact_id":null,
                "reason":"No durable state changes."
            }]
        })));
        let nullable_inactive_arrays = serde_json::json!({
            "outcomes":[{
                "action_digest":digest,
                "band":"mixed",
                "effect_kind":"no_material_change",
                "supporting_state_references":[],
                "pressure_additions":null,
                "pressure_resolutions":null,
                "recipient_subject_ids":null,
                "reason":"No durable state changes."
            }]
        });
        assert!(validator.is_valid(&nullable_inactive_arrays));
        let normalized: OutcomeProposalBundle =
            serde_json::from_value(nullable_inactive_arrays).unwrap();
        assert!(normalized.outcomes[0].pressure_additions.is_empty());
        assert!(normalized.outcomes[0].pressure_resolutions.is_empty());
        assert!(normalized.outcomes[0].recipient_subject_ids.is_empty());
        assert!(!validator.is_valid(&outcome(None, None)));
        assert!(!validator.is_valid(&outcome(Some("invented-owner"), None)));
        assert!(!validator.is_valid(&outcome(
            Some("dockers"),
            Some(("resource", serde_json::json!("spare rope")))
        )));
        assert!(!validator.is_valid(&serde_json::json!({
            "outcomes":[{
                "action_digest":format!("sha256:{}", "f".repeat(64)),
                "band":"mixed",
                "effect_kind":"no_material_change",
                "supporting_state_references":[],
                "reason":"No durable state changes."
            }]
        })));
    }

    #[test]
    fn outcome_schema_omits_unavailable_empty_effect_lanes() {
        let value = campaign();
        let action = proposal();
        let mut contexts = build_context(&value, &[action]).unwrap().actions;
        let mut context = contexts.remove(0);
        context.source_state["resources"] = serde_json::json!([]);
        context.active_relations.clear();
        context.pressure_owners.clear();
        context.resource_recipient_ids.clear();
        context.discoverable_facts.clear();
        context.member_state_owner_id = None;
        context.admissible_effect_kinds = vec![OutcomeEffectKind::NoMaterialChange];
        context.allowed_state_references.clear();
        let mut schema = serde_json::to_value(schema_for!(OutcomeProposalBundle)).unwrap();
        constrain_outcome_schema(&mut schema, std::slice::from_ref(&context)).unwrap();
        let validator = jsonschema::validator_for(&schema).unwrap();

        assert!(validator.is_valid(&serde_json::json!({
            "outcomes":[{
                "action_digest":context.action_digest,
                "band":"mixed",
                "effect_kind":"no_material_change",
                "supporting_state_references":[],
                "reason":"The attempt changes no durable state."
            }]
        })));
    }

    #[test]
    fn pressure_context_and_schema_expose_only_real_state_changes() {
        let value = campaign();
        let action = proposal();
        let context = action_context(&value, &action).unwrap();
        assert_eq!(context.pressure_owners.len(), 1);
        assert_eq!(context.pressure_owners[0].owner_subject_id, "dockers");
        assert_eq!(
            context.pressure_owners[0].current_pressures,
            vec!["storm damage"]
        );

        let mut schema = serde_json::to_value(schema_for!(OutcomeProposalBundle)).unwrap();
        constrain_outcome_schema(&mut schema, std::slice::from_ref(&context)).unwrap();
        let validator = jsonschema::validator_for(&schema).unwrap();
        let pressure = |additions: Vec<&str>, resolutions: Vec<&str>| {
            serde_json::json!({
                "outcomes":[{
                    "action_digest":context.action_digest,
                    "band":"mixed",
                    "effect_kind":"gestalt_pressure",
                    "supporting_state_references":["pressure:dockers:storm damage"],
                    "owner_subject_id":"dockers",
                    "pressure_additions":additions,
                    "pressure_resolutions":resolutions
                }]
            })
        };

        assert!(validator.is_valid(&pressure(vec!["crew exhaustion"], vec![])));
        assert!(validator.is_valid(&pressure(vec![], vec!["storm damage"])));
        assert!(!validator.is_valid(&pressure(vec![], vec![])));
        assert!(!validator.is_valid(&serde_json::json!({
            "outcomes":[{
                "action_digest":context.action_digest,
                "band":"mixed",
                "effect_kind":"gestalt_pressure",
                "supporting_state_references":["pressure:dockers:storm damage"],
                "owner_subject_id":"dockers",
                "pressure_additions":null,
                "pressure_resolutions":["storm damage"]
            }]
        })));
        let repeated_pressure = pressure(vec!["storm damage"], vec![]);
        assert!(validator.is_valid(&repeated_pressure));
        let repeated_bundle: OutcomeProposalBundle =
            serde_json::from_value(repeated_pressure).unwrap();
        let repeated_error = bind_outcomes(&value, &[action], repeated_bundle)
            .unwrap_err()
            .to_string();
        assert!(repeated_error.contains("gestalt pressure additions already exist: storm damage"));
        assert!(!validator.is_valid(&pressure(vec![], vec!["invented resolution"])));
    }

    #[tokio::test]
    async fn repeated_current_pressure_uses_the_outcome_semantic_correction_owner() {
        let value = campaign();
        let mut action = proposal();
        action.effects = vec![StrategicCellEffect::GestaltActivity {
            gestalt_id: "dockers".into(),
            activity: StrategicActivityKind::Prepare,
            target_subject_ids: vec![],
            location_ids: vec!["dock".into()],
        }];
        let digest = cell_action_digest(&action).unwrap();
        let model = Arc::new(RepeatingPressureModel {
            action_digest: digest,
            requests: Mutex::new(Vec::new()),
        });

        let (outcomes, stages) = resolve_activity_outcomes(model.clone(), &value, &[action])
            .await
            .unwrap();

        assert_eq!(stages.len(), 2);
        assert_eq!(stages[0].receipt.provider_attempts.len(), 1);
        assert_eq!(stages[0].receipt.validation_result, "semantic_invalid");
        assert_eq!(stages[1].receipt.provider_attempts.len(), 1);
        assert!(matches!(
            outcomes[0].effect,
            StrategicOutcomeEffect::NoMaterialChange { .. }
        ));
        let requests = model.requests.lock().unwrap();
        assert_eq!(requests.len(), 2);
        assert!(
            requests[1]
                .lived_stream
                .contains("gestalt pressure additions already exist: storm damage")
        );
        assert!(
            requests[1]
                .lived_stream
                .contains("PREVIOUS_REJECTED_BUNDLE")
        );
        assert!(requests[1].lived_stream.contains("pressure_additions"));
    }

    #[test]
    fn reference_rejection_names_the_action_offender_and_allowed_handles() {
        let value = campaign();
        let action = proposal();
        let digest = cell_action_digest(&action).unwrap();
        let outcome = StrategicActivityOutcome {
            schema: "ghostlight.strategic_activity_outcome.v1".into(),
            action_digest: digest.clone(),
            source_subject_id: "dockers".into(),
            band: StrategicOutcomeBand::Success,
            summary: "The inspection confirms the causeway is open.".into(),
            supporting_state_references: vec!["knowledge:borrowed from another action".into()],
            effect: StrategicOutcomeEffect::KnowledgeLearned {
                owner_subject_id: "dockers".into(),
                fact_id: "fact:safe-route".into(),
            },
        };

        let error = validate_activity_outcomes(&value, &[action], &[outcome])
            .unwrap_err()
            .to_string();

        assert!(error.contains(&digest));
        assert!(error.contains("knowledge:borrowed from another action"));
        assert!(error.contains("fact:fact:safe-route"));
        assert!(error.contains("exact allowed_state_references"));
    }

    #[test]
    fn named_actor_possessions_use_the_resource_reference_ontology() {
        let mut value = campaign();
        value.actors.insert(
            "reed".into(),
            crate::domain::ActorState {
                id: "reed".into(),
                name: "Reed".into(),
                location_id: "dock".into(),
                capabilities: BTreeSet::from(["field repair".into()]),
                knowledge: BTreeSet::new(),
                equipment: BTreeSet::from(["medical satchel".into()]),
                conditions: BTreeSet::new(),
                obligations: BTreeSet::new(),
                relationships: BTreeMap::new(),
                goals: vec!["keep the patients together".into()],
                memories: vec![],
            },
        );
        ensure_agency_profiles(&mut value);

        let references = allowed_state_references(
            &value,
            &CellActionProposal {
                subject_id: "reed".into(),
                intent: "use the satchel to stabilize a patient".into(),
                intended_effect: "spend its remaining supplies".into(),
                priority: 90,
                state_references: vec!["resource:medical satchel".into()],
                public_channels: vec![],
                effects: vec![StrategicCellEffect::ActorActivity {
                    actor_id: "reed".into(),
                    activity: StrategicActivityKind::Prepare,
                    target_subject_ids: vec![],
                    location_ids: vec!["dock".into()],
                }],
            },
        )
        .unwrap();

        assert!(references.contains("resource:medical satchel"));
        assert!(!references.contains("equipment:medical satchel"));
    }

    #[test]
    fn resource_outcome_reports_the_exact_required_owner() {
        let value = campaign();
        let mut action = proposal();
        action.effects = vec![StrategicCellEffect::GestaltActivity {
            gestalt_id: "dockers".into(),
            activity: StrategicActivityKind::Prepare,
            target_subject_ids: vec![],
            location_ids: vec!["dock".into()],
        }];
        let outcome = StrategicActivityOutcome {
            schema: "ghostlight.strategic_activity_outcome.v1".into(),
            action_digest: cell_action_digest(&action).unwrap(),
            source_subject_id: "dockers".into(),
            band: StrategicOutcomeBand::Success,
            summary: "The dockers finish a new net.".into(),
            supporting_state_references: vec!["capability:repair nets".into()],
            effect: StrategicOutcomeEffect::ResourceCreated {
                owner_subject_id: "member:dockers".into(),
                resource: "finished storm net".into(),
            },
        };
        let error = validate_activity_outcomes(&value, &[action], &[outcome]).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("owner_subject_id must be the source or one exact activity location")
        );
        assert!(error.to_string().contains("exact admissible_effect_kinds"));
    }

    #[test]
    fn resource_creation_can_belong_to_the_exact_activity_place() {
        let value = campaign();
        let mut action = proposal();
        action.intent = "brace a permanent public warning at the damaged quay".into();
        action.intended_effect = "leave a durable warning where travelers will see it".into();
        action.effects = vec![StrategicCellEffect::GestaltActivity {
            gestalt_id: "dockers".into(),
            activity: StrategicActivityKind::Prepare,
            target_subject_ids: vec![],
            location_ids: vec!["dock".into()],
        }];
        let outcome = StrategicActivityOutcome {
            schema: "ghostlight.strategic_activity_outcome.v1".into(),
            action_digest: cell_action_digest(&action).unwrap(),
            source_subject_id: "dockers".into(),
            band: StrategicOutcomeBand::Success,
            summary: "ignored model summary".into(),
            supporting_state_references: vec!["capability:repair nets".into()],
            effect: StrategicOutcomeEffect::ResourceCreated {
                owner_subject_id: "dock".into(),
                resource: "reinforced public hazard marker".into(),
            },
        };

        validate_activity_outcomes(
            &value,
            std::slice::from_ref(&action),
            std::slice::from_ref(&outcome),
        )
        .unwrap();
        assert_eq!(
            resolved_outcome_summary(&value, &action, &outcome.effect).unwrap(),
            "At Dock, Dockers establishes reinforced public hazard marker."
        );
    }

    #[test]
    fn relation_outcome_summary_names_people_and_consequence_without_state_coordinates() {
        let mut value = campaign();
        value.agency_relations.insert(
            "dock-command".into(),
            AgencyRelation {
                schema: "ghostlight.agency_relation.v1".into(),
                id: "dock-command".into(),
                from_subject_id: "dockers".into(),
                to_subject_id: "player".into(),
                kind: AgencyRelationKind::Command,
                strength: 50,
                active: true,
                evidence_receipt_ids: vec![],
            },
        );

        let mut action = proposal();
        action.public_channels = vec!["dock broadsheet".into()];
        action.effects = vec![StrategicCellEffect::GestaltActivity {
            gestalt_id: "dockers".into(),
            activity: StrategicActivityKind::Communicate,
            target_subject_ids: vec!["player".into()],
            location_ids: vec!["dock".into()],
        }];

        let summary = resolved_outcome_summary(
            &value,
            &action,
            &StrategicOutcomeEffect::AgencyRelationShift {
                relation_id: "dock-command".into(),
                strength_delta: -5,
            },
        )
        .unwrap();

        assert_eq!(
            summary,
            "A public appeal by Dockers weakens the command relationship between Dockers and Player."
        );
        assert!(!summary.contains("dock-command"));
        assert!(!summary.contains("-5"));

        value.agency_relations.insert(
            "dock-rivalry".into(),
            AgencyRelation {
                schema: "ghostlight.agency_relation.v1".into(),
                id: "dock-rivalry".into(),
                from_subject_id: "dockers".into(),
                to_subject_id: "player".into(),
                kind: AgencyRelationKind::Rivalry,
                strength: 50,
                active: true,
                evidence_receipt_ids: vec![],
            },
        );
        assert_eq!(
            resolved_outcome_summary(
                &value,
                &action,
                &StrategicOutcomeEffect::AgencyRelationShift {
                    relation_id: "dock-rivalry".into(),
                    strength_delta: -5,
                },
            )
            .unwrap(),
            "A public appeal by Dockers eases the rivalry relationship between Dockers and Player."
        );
    }

    #[tokio::test]
    async fn resolver_keeps_each_selected_attempt_in_one_digest_bound_stage() {
        let value = campaign();
        let action = proposal();
        let digest = cell_action_digest(&action).unwrap();
        let model = Arc::new(OutcomeFixtureModel {
            action_digest: digest.clone(),
            requests: Mutex::new(Vec::new()),
        });

        let (outcomes, stages) = resolve_activity_outcomes(model.clone(), &value, &[action])
            .await
            .unwrap();

        assert_eq!(outcomes.len(), 1);
        assert!(matches!(
            outcomes[0].effect,
            StrategicOutcomeEffect::KnowledgeLearned { .. }
        ));
        assert_eq!(
            outcomes[0].summary,
            "Dockers learns: the western causeway remains open."
        );
        assert_eq!(stages.len(), 2);
        let requests = model.requests.lock().unwrap();
        assert_eq!(requests.len(), 2);
        assert_eq!(requests[0].stage, "strategic_outcome_resolver");
        assert_eq!(requests[1].stage, "strategic_outcome_verifier");
        assert!(
            requests[0]
                .lived_stream
                .contains("exactly one field named outcomes")
        );
        assert!(
            requests[0]
                .lived_stream
                .contains("never action_resolutions")
        );
        assert!(requests[0].lived_stream.contains("Do not emit summary"));
        assert!(requests[0].lived_stream.contains("admissible_effect_kinds"));
        assert!(requests[0].lived_stream.contains("knowledge_learned"));
        assert!(
            requests[0]
                .lived_stream
                .contains("copied literally and only from that action's allowed_state_references")
        );
        assert!(
            requests[0]
                .lived_stream
                .contains("the target field already identifies the target")
        );
        assert!(
            requests[0]
                .lived_stream
                .contains("resource_recipient_ids value into the output field other_subject_id")
        );
        assert_eq!(
            requests[0].snapshot_binding,
            activity_outcome_binding(
                value.id,
                value.revision,
                value.resolution_policy.resolution_epoch,
                &[digest]
            )
        );
        assert!(requests[1].lived_stream.contains(
            "Knowledge learned requires an investigation, observation, or communication"
        ));
    }

    #[tokio::test]
    async fn resolver_never_shares_authority_context_between_selected_attempts() {
        let value = campaign();
        let first = proposal();
        let mut second = proposal();
        second.intent = "inspect the eastern causeway supports".into();
        second.intended_effect = "establish whether the eastern supports remain sound".into();
        let expected = vec![
            cell_action_digest(&first).unwrap(),
            cell_action_digest(&second).unwrap(),
        ];
        let model = Arc::new(IsolatingOutcomeModel {
            requests: Mutex::new(Vec::new()),
        });

        let (outcomes, stages) = resolve_activity_outcomes(model.clone(), &value, &[first, second])
            .await
            .unwrap();

        assert_eq!(
            outcomes
                .iter()
                .map(|outcome| outcome.action_digest.clone())
                .collect::<Vec<_>>(),
            expected
        );
        assert_eq!(stages.len(), 2);
        assert_eq!(model.requests.lock().unwrap().len(), 2);
    }

    #[test]
    fn prepare_context_projects_only_locally_admissible_durable_handles() {
        let value = campaign();
        let mut action = proposal();
        action.effects = vec![StrategicCellEffect::GestaltActivity {
            gestalt_id: "dockers".into(),
            activity: StrategicActivityKind::Prepare,
            target_subject_ids: vec![],
            location_ids: vec!["dock".into()],
        }];

        let context = action_context(&value, &action).unwrap();

        assert!(
            context
                .admissible_effect_kinds
                .contains(&OutcomeEffectKind::NoMaterialChange)
        );
        assert!(
            context
                .admissible_effect_kinds
                .contains(&OutcomeEffectKind::ResourceCreated)
        );
        assert!(
            context
                .admissible_effect_kinds
                .contains(&OutcomeEffectKind::ResourceConsumed)
        );
        assert!(
            context
                .admissible_effect_kinds
                .contains(&OutcomeEffectKind::GestaltPressure)
        );
        assert!(
            !context
                .admissible_effect_kinds
                .contains(&OutcomeEffectKind::KnowledgeLearned)
        );
        assert_eq!(context.resource_owner_id, "dockers");
        assert_eq!(context.resource_creation_owner_ids, ["dock", "dockers"]);

        let mut schema = serde_json::to_value(schema_for!(OutcomeProposalBundle)).unwrap();
        constrain_outcome_schema(&mut schema, std::slice::from_ref(&context)).unwrap();
        let validator = jsonschema::validator_for(&schema).unwrap();
        let proposed = |owner: &str| {
            serde_json::json!({
                "outcomes":[{
                    "action_digest":context.action_digest,
                    "band":"success",
                    "effect_kind":"resource_created",
                    "supporting_state_references":["capability:repair nets"],
                    "owner_subject_id":owner,
                    "resource":"reinforced public hazard marker"
                }]
            })
        };
        assert!(validator.is_valid(&proposed("dock")));
        assert!(validator.is_valid(&proposed("dockers")));
        assert!(!validator.is_valid(&proposed("somewhere-else")));
    }

    #[test]
    fn physical_member_activity_cannot_mint_an_unrelated_obligation() {
        let mut value = campaign();
        value.gestalt_members.insert(
            "mira".into(),
            crate::domain::GestaltMemberDelta {
                schema: "ghostlight.gestalt_member_delta.v1".into(),
                id: "mira".into(),
                gestalt_id: "dockers".into(),
                version: 0,
                name: "Mira".into(),
                capability_additions: BTreeSet::from(["pull relays".into()]),
                capability_removals: BTreeSet::new(),
                knowledge_additions: BTreeSet::new(),
                knowledge_removals: BTreeSet::new(),
                equipment: BTreeSet::new(),
                conditions: BTreeSet::new(),
                obligations: BTreeSet::new(),
                relationships: BTreeMap::new(),
                goals: vec!["keep the harbor working".into()],
                memories: vec![],
                last_location_id: Some("dock".into()),
                materialized_actor_id: None,
                last_relevant_revision: 0,
                relevance_lease_until_revision: 0,
            },
        );
        ensure_agency_profiles(&mut value);
        let mut action = CellActionProposal {
            subject_id: "member:mira".into(),
            intent: "pull the local relay".into(),
            intended_effect: "interrupt the local signal".into(),
            priority: 70,
            state_references: vec!["capability:pull relays".into(), "location:dock".into()],
            public_channels: vec![],
            effects: vec![StrategicCellEffect::MemberActivity {
                member_id: "mira".into(),
                activity: StrategicActivityKind::Obstruct,
                target_subject_ids: vec![],
                location_ids: vec!["dock".into()],
            }],
        };

        let physical = action_context(&value, &action).unwrap();
        assert!(
            !physical
                .admissible_effect_kinds
                .contains(&OutcomeEffectKind::MemberObligation)
        );

        if let StrategicCellEffect::MemberActivity { activity, .. } = &mut action.effects[0] {
            *activity = StrategicActivityKind::Communicate;
        }
        let social = action_context(&value, &action).unwrap();
        assert!(
            social
                .admissible_effect_kinds
                .contains(&OutcomeEffectKind::MemberObligation)
        );
    }

    #[test]
    fn outcome_shape_error_names_the_exact_irrelevant_fields() {
        let proposal = OutcomeProposal {
            action_digest: "sha256:ignored".into(),
            band: StrategicOutcomeBand::Success,
            effect_kind: OutcomeEffectKind::AgencyRelationShift,
            supporting_state_references: vec![],
            owner_subject_id: Some("dockers".into()),
            other_subject_id: Some("rivals".into()),
            relation_id: Some("dock-rivalry".into()),
            strength_delta: Some(-2),
            resource: None,
            pressure_additions: vec![],
            pressure_resolutions: vec![],
            member_id: None,
            memory: None,
            obligation: None,
            relationship_description: None,
            fact_id: None,
            recipient_subject_ids: vec![],
            reason: None,
        };

        let error = bind_effect(&proposal).unwrap_err().to_string();

        assert!(error.contains("other_subject_id, owner_subject_id"));
        assert!(error.contains("relation_id, strength_delta"));
    }

    #[tokio::test]
    async fn independent_verifier_can_reject_a_structurally_legal_unrelated_resource_cost() {
        let value = campaign();
        let action = proposal();
        let digest = cell_action_digest(&action).unwrap();
        let outcome = StrategicActivityOutcome {
            schema: "ghostlight.strategic_activity_outcome.v1".into(),
            action_digest: digest.clone(),
            source_subject_id: "dockers".into(),
            band: StrategicOutcomeBand::Mixed,
            summary: "Dockers expends spare rope.".into(),
            supporting_state_references: vec!["resource:spare rope".into()],
            effect: StrategicOutcomeEffect::ResourceConsumed {
                owner_subject_id: "dockers".into(),
                resource: "spare rope".into(),
            },
        };
        validate_activity_outcomes(
            &value,
            std::slice::from_ref(&action),
            std::slice::from_ref(&outcome),
        )
        .unwrap();
        let context = build_context(&value, std::slice::from_ref(&action)).unwrap();

        let (_, mismatches) = verify_outcomes(
            &RejectingVerifierModel {
                action_digest: digest.clone(),
            },
            &value,
            &context,
            &[outcome],
            &[digest],
            &[],
        )
        .await
        .unwrap();

        assert_eq!(mismatches.len(), 1);
        assert!(mismatches[0].contains("never uses or expends spare rope"));
    }

    #[tokio::test]
    async fn verifier_rejection_returns_to_resolver_without_mutating_the_effect() {
        let value = campaign();
        let action = proposal();
        let digest = cell_action_digest(&action).unwrap();
        let model = Arc::new(CorrectingOutcomeModel {
            action_digest: digest,
            resolver_calls: Mutex::new(0),
        });

        let (outcomes, stages) = resolve_activity_outcomes(model.clone(), &value, &[action])
            .await
            .unwrap();

        assert_eq!(stages.len(), 3);
        assert!(matches!(
            outcomes[0].effect,
            StrategicOutcomeEffect::NoMaterialChange { .. }
        ));
        assert_eq!(*model.resolver_calls.lock().unwrap(), 2);
    }

    #[tokio::test]
    async fn verifier_rejects_a_fact_that_the_attempt_did_not_learn_or_communicate() {
        let value = campaign();
        let mut action = proposal();
        action.intent =
            "ask the dock watch to confirm the next rotation without discussing the western route"
                .into();
        action.intended_effect = "receive the watchkeeper's answer about the next rotation".into();
        action.effects = vec![StrategicCellEffect::GestaltActivity {
            gestalt_id: "dockers".into(),
            activity: StrategicActivityKind::Communicate,
            target_subject_ids: vec![],
            location_ids: vec!["dock".into()],
        }];
        let digest = cell_action_digest(&action).unwrap();
        let model = Arc::new(CorrectingKnowledgeModel {
            action_digest: digest,
            resolver_calls: Mutex::new(0),
            requests: Mutex::new(Vec::new()),
        });

        let (outcomes, stages) = resolve_activity_outcomes(model.clone(), &value, &[action])
            .await
            .unwrap();

        assert_eq!(stages.len(), 3);
        assert!(matches!(
            outcomes[0].effect,
            StrategicOutcomeEffect::NoMaterialChange { .. }
        ));
        assert_eq!(*model.resolver_calls.lock().unwrap(), 2);
        let requests = model.requests.lock().unwrap();
        assert_eq!(requests[1].stage, "strategic_outcome_verifier");
        assert!(
            requests[2]
                .lived_stream
                .contains("does not observe, investigate, or communicate the proposed fact")
        );
    }
}
