use crate::{
    domain::{
        ActorState, AgencyAxis, AgencyRelation, AgencyRelationKind, AgencySubjectKind,
        BranchOrigin, Campaign, EvidenceCoverage, EvidenceUseLane, FactScope, GestaltMemberDelta,
        GestaltPersonaState, InstitutionState, Location, VaultEvidenceReceipt, WorldClock,
        WorldCompilePreview, WorldFact,
    },
    model::{
        ModelPort, ModelStageReceipt, ModelStageRequest, run_validated_stage,
        run_validated_stage_with_timeout,
    },
    vault::{VaultProvider, VaultQuery},
};
use anyhow::{Result, anyhow};
use chrono::{DateTime, Utc};
use schemars::{JsonSchema, schema_for};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct OpeningRequest {
    pub setting: String,
    pub constraints: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct OpeningSuggestion {
    pub id: String,
    pub title: String,
    pub era: String,
    pub place: String,
    pub pressure: String,
    pub player_hook: String,
    pub evidence_receipt_ids: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct RoleSuggestion {
    pub id: String,
    pub name: String,
    pub premise: String,
    pub capabilities: Vec<String>,
    pub obligations: Vec<String>,
    pub evidence_receipt_ids: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct CustomStart {
    pub campaign_name: String,
    pub who: String,
    pub where_: String,
    pub when: String,
    pub goal: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct SelectedStart {
    pub campaign_name: String,
    pub opening: OpeningSuggestion,
    pub role: RoleSuggestion,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct GestaltFissionRequest {
    pub parent_gestalt_id: String,
    pub partition_axis: AgencyAxis,
    pub requested_partition_values: Vec<String>,
    pub reason: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SuggestedOpenings {
    pub openings: Vec<OpeningSuggestion>,
    pub evidence_receipts: Vec<VaultEvidenceReceipt>,
    pub model_receipts: Vec<ModelStageReceipt>,
    pub retrieval_receipt: ModelStageReceipt,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SuggestedRoles {
    pub roles: Vec<RoleSuggestion>,
    pub evidence_receipts: Vec<VaultEvidenceReceipt>,
    pub model_receipts: Vec<ModelStageReceipt>,
    pub retrieval_receipt: ModelStageReceipt,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
struct OpeningSet {
    openings: Vec<OpeningSuggestion>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
struct RoleSet {
    roles: Vec<RoleSuggestion>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
struct RetrievalQueryPlan {
    queries: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
struct OpeningRetrievalPlan {
    early_frame_query: String,
    transition_frame_query: String,
    late_frame_query: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
struct EvidenceUsePlan {
    coverage: Vec<EvidenceCoverage>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
struct CompiledSeed {
    title: String,
    canon_cutoff: String,
    world_time: DateTime<Utc>,
    #[schemars(range(min = 1))]
    tick_hours: u32,
    player: ActorState,
    locations: Vec<Location>,
    actors: Vec<ActorState>,
    #[serde(default)]
    gestalts: Vec<GestaltPersonaState>,
    #[serde(default)]
    gestalt_members: Vec<GestaltMemberDelta>,
    institutions: Vec<InstitutionState>,
    clocks: Vec<WorldClock>,
    facts: Vec<WorldFact>,
    gaps: Vec<String>,
    branch_assumptions: Vec<String>,
    opening_narration: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
struct CompiledAgencySkeleton {
    agency_profiles: Vec<CompiledAgencyProfile>,
    agency_relations: Vec<CompiledAgencyRelation>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
struct CompiledRemoteInstitution {
    name: String,
    mandate: String,
    #[serde(skip)]
    #[schemars(skip)]
    evidence_receipt_ids: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
struct CompiledGlobalAgencyCatalog {
    // The model proposes a bounded candidate pool. Local grounding owns the
    // actual 32-cell admission limit because unsupported index fragments must
    // not consume canonical agency capacity.
    #[schemars(length(max = 64))]
    institutions: Vec<CompiledRemoteInstitution>,
    gaps: Vec<String>,
}

#[derive(Clone, Debug, Serialize, JsonSchema, PartialEq)]
struct AgencySubjectBrief {
    subject_id: String,
    subject_kind: AgencySubjectKind,
    name: String,
    location_ids: BTreeSet<String>,
    capabilities_or_resources: Vec<String>,
    knowledge_or_posture: Vec<String>,
    goals: Vec<String>,
    pressures_or_obligations: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
struct CompiledAgencyProfile {
    subject_id: String,
    subject_kind: AgencySubjectKind,
    collective_authority_id: Option<String>,
    facets: BTreeMap<AgencyAxis, BTreeSet<String>>,
    location_ids: BTreeSet<String>,
    information_channels: BTreeSet<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
struct CompiledAgencyRelation {
    id: String,
    from_subject_id: String,
    to_subject_id: String,
    kind: AgencyRelationKind,
    #[schemars(range(min = 1, max = 100))]
    strength: u8,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
struct CompiledExpansionSeed {
    locations: Vec<Location>,
    facts: Vec<WorldFact>,
    gaps: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
struct CompiledFissionSeed {
    children: Vec<GestaltPersonaState>,
    child_partition_values: BTreeMap<String, String>,
    #[serde(default)]
    member_child_assignments: BTreeMap<String, String>,
    gaps: Vec<String>,
}

pub struct WorldCompiler {
    vault: Arc<dyn VaultProvider>,
    model: Arc<dyn ModelPort>,
    retrieval_model: String,
    compiler_model: String,
}

impl WorldCompiler {
    pub fn new(
        vault: Arc<dyn VaultProvider>,
        model: Arc<dyn ModelPort>,
        retrieval_model: impl Into<String>,
        compiler_model: impl Into<String>,
    ) -> Self {
        Self {
            vault,
            model,
            retrieval_model: retrieval_model.into(),
            compiler_model: compiler_model.into(),
        }
    }

    pub async fn suggest_openings(&self, request: OpeningRequest) -> Result<SuggestedOpenings> {
        validate_user_text("setting", &request.setting, 120)?;
        if request.constraints.len() > 8 {
            return Err(anyhow!("opening request accepts at most 8 constraints"));
        }
        for constraint in &request.constraints {
            validate_user_text("opening constraint", constraint, 240)?;
        }
        let (queries, retrieval_receipt) = self.plan_opening_queries(&request).await?;
        let receipts = self.retrieve_all(&queries, "all", 8).await?;
        let evidence = opening_evidence_text(&queries, &receipts);
        let base_prompt = format!(
            "Generate exactly three source-grounded openings, taking one from each labeled historical-frame evidence group when that group contains adequate support. The three literal `era` values must name specific, genuinely different historical periods and be pairwise distinct after trimming and case-folding. An umbrella label such as `Post-Elysium` is insufficient when used twice: qualify each with its distinct source-supported event, phase, or date. The three `place` values and three `pressure` values must independently be pairwise distinct. Do not return aliases for the same period or place merely to satisfy spelling-level diversity. Do not fill material evidence gaps with invention. Before returning, verify the nine axis values yourself. REQUEST:\n{}\nEVIDENCE GROUPS:\n{}",
            serde_json::to_string(&request)?,
            evidence
        );
        let schema = serde_json::to_value(schema_for!(OpeningSet))?;
        let source_receipts = receipt_ids(&receipts);
        let mut correction = String::new();
        let mut model_receipts = Vec::new();
        for attempt in 0..2 {
            let (value, stage) = self
                .structured(
                    "world_openings",
                    "opening-suggestions",
                    &format!("{base_prompt}{correction}"),
                    schema.clone(),
                    source_receipts.clone(),
                )
                .await?;
            model_receipts.push(stage);
            let parsed: OpeningSet = serde_json::from_value(value.clone())?;
            let validation = if parsed.openings.len() != 3 {
                Err(anyhow!("world compiler must return exactly three openings"))
            } else {
                validate_opening_suggestions(&parsed.openings, &source_receipts)
            };
            match validation {
                Ok(()) => {
                    return Ok(SuggestedOpenings {
                        openings: parsed.openings,
                        evidence_receipts: receipts,
                        model_receipts,
                        retrieval_receipt,
                    });
                }
                Err(error) if attempt == 0 => {
                    mark_semantic_invalid(
                        model_receipts
                            .last_mut()
                            .expect("opening receipt was just stored"),
                        &error,
                    );
                    correction = format!(
                        "\n\nLOCAL VALIDATOR REJECTED THE PREVIOUS OPENINGS: {error}\nPREVIOUS_REJECTED_OPENINGS:\n{}\nReturn one complete corrected set. Replace the specifically collided values named by the validator with different source-supported values. All three literal values on each axis must be pairwise distinct after trimming and case-folding. Preserve source grounding and use only supplied evidence.",
                        serde_json::to_string(&value)?
                    );
                }
                Err(error) => {
                    return Err(anyhow!(
                        "world opening compiler failed local validation after one correction: {error}"
                    ));
                }
            }
        }
        unreachable!()
    }

    pub async fn suggest_roles(&self, opening: &OpeningSuggestion) -> Result<SuggestedRoles> {
        let (queries, retrieval_receipt) = self
            .plan_queries(
                "role_retrieval_plan",
                &format!("roles:{}", opening.id),
                &serde_json::to_string(opening)?,
                2,
            )
            .await?;
        let receipts = self.retrieve_all(&queries, &opening.era, 8).await?;
        let base_prompt = format!(
            "Generate exactly three materially distinct player roles grounded in this opening and evidence. Names and premises must each be pairwise distinct after trimming and case-folding. The roles must differ in social position, capabilities, and obligations rather than being cosmetic aliases. OPENING:\n{}\nEVIDENCE:\n{}",
            serde_json::to_string(opening)?,
            evidence_text(&receipts)
        );
        let schema = serde_json::to_value(schema_for!(RoleSet))?;
        let source_receipts = receipt_ids(&receipts);
        let mut correction = String::new();
        let mut model_receipts = Vec::new();
        for attempt in 0..2 {
            let (value, stage) = self
                .structured(
                    "world_roles",
                    &format!("roles:{}", opening.id),
                    &format!("{base_prompt}{correction}"),
                    schema.clone(),
                    source_receipts.clone(),
                )
                .await?;
            model_receipts.push(stage);
            let parsed: RoleSet = serde_json::from_value(value.clone())?;
            let validation = if parsed.roles.len() != 3 {
                Err(anyhow!("world compiler must return exactly three roles"))
            } else {
                validate_role_suggestions(&parsed.roles, &source_receipts)
            };
            match validation {
                Ok(()) => {
                    return Ok(SuggestedRoles {
                        roles: parsed.roles,
                        evidence_receipts: receipts,
                        model_receipts,
                        retrieval_receipt,
                    });
                }
                Err(error) if attempt == 0 => {
                    mark_semantic_invalid(
                        model_receipts
                            .last_mut()
                            .expect("role receipt was just stored"),
                        &error,
                    );
                    correction = format!(
                        "\n\nLOCAL VALIDATOR REJECTED THE PREVIOUS ROLES: {error}\nPREVIOUS_REJECTED_ROLES:\n{}\nReturn one complete corrected set. Replace the specifically collided names or premises named by the validator and make the roles materially different while preserving source grounding.",
                        serde_json::to_string(&value)?
                    );
                }
                Err(error) => {
                    return Err(anyhow!(
                        "world role compiler failed local validation after one correction: {error}"
                    ));
                }
            }
        }
        unreachable!()
    }

    pub async fn compile_custom(
        &self,
        start: CustomStart,
    ) -> Result<(WorldCompilePreview, Vec<ModelStageReceipt>)> {
        validate_user_text("campaign name", &start.campaign_name, 80)?;
        validate_user_text("player identity", &start.who, 500)?;
        validate_user_text("starting location", &start.where_, 500)?;
        validate_user_text("starting time", &start.when, 500)?;
        validate_user_text("player goal", &start.goal, 1_000)?;
        let (queries, retrieval_receipt) = self
            .plan_queries(
                "custom_retrieval_plan",
                "custom-start",
                &serde_json::to_string(&start)?,
                3,
            )
            .await?;
        let global_queries = global_agency_queries(&start);
        let (local_evidence, global_evidence) = tokio::join!(
            self.retrieve_all(&queries, &start.when, 8),
            self.retrieve_all(&global_queries, &start.when, 12),
        );
        let receipts = local_evidence?;
        let global_receipts = global_evidence?;
        let (classified, global_catalog) = tokio::join!(
            self.classify_evidence(&start, &receipts),
            self.compile_global_agency_catalog(&start, &global_receipts),
        );
        let (evidence_coverage, relevance_receipts) = classified?;
        let (global_catalog, global_catalog_receipts) = global_catalog?;
        let scoped_evidence = direct_seed_evidence_text(&receipts, &evidence_coverage);
        let shared_prefix = format!(
            "SOURCE-GROUNDED WORLD COMPILATION\nSTART:\n{}\nSCOPED EVIDENCE:\n{}\n\n",
            serde_json::to_string(&start)?,
            scoped_evidence
        );
        let base_prompt = format!(
            "{shared_prefix}Compile a bounded playable region with stable topology, local actors, populations, clocks, and only those remote institutions that have a direct causal relationship to this requested start. SCOPED EVIDENCE contains direct_seed witnesses only. Setting-background and excluded witnesses remain visible in the approval coverage but are deliberately absent here: they cannot donate cast, incidents, clocks, location state, goals, or institutional posture to this branch. When direct evidence cannot ground a requested local detail, keep the local cast sparse, mark reversible texture provisional_local, and list the material gap instead of borrowing a nearby story. Do not eagerly invent remote settlements, routes, or people. Emit only supported canon facts. A canon_baseline fact must cite one or more exact receipt_id values printed in SCOPED EVIDENCE whose witnesses directly support the whole statement. Never label an invented proper noun canon. Facts that an actor can uncover through an admitted local observation must exist before play and list the exact discoverable_at_location_ids where that observation is possible. Seed enough branch_local or provisional_local discoverable facts to make the requested opening pressure and immediate goal actionable; at least one such non-canon fact must be discoverable at the player's exact starting location. The later action assessor can reveal an existing fact but cannot invent one. Facts that are private history or not directly observable have an empty discovery-location set. The player location and every actor location must exist. Every route destination and fact discovery location must exist, travel time must be positive, clocks need positive thresholds, and the player id must be unique. Actor relationship map keys must copy exact actor or institution IDs declared in this candidate, never display names, roles, groups, or location IDs. Represent populations that can act collectively (villages, crews, crowds, departments, corporations) as gestalt Personas. Seed a small roster of plausible durable member identities for people the player may encounter; member deltas contain only departures from their gestalt baseline and begin dematerialized. Do not duplicate a gestalt member in actors. Keep named plot-critical people as ordinary actors. Every gestalt home location and member gestalt reference must exist. Do not emit agency profiles or relations; those are compiled from the exact validated subject roster in the next stage."
        );
        let schema = serde_json::to_value(schema_for!(CompiledSeed))?;
        let sources = receipt_ids_for_coverage(&receipts, &evidence_coverage);
        let mut compiler_receipts = Vec::new();
        let mut correction = String::new();
        let mut seed = loop {
            let output = self
                .structured(
                    "world_compile",
                    "custom-start",
                    &format!("{base_prompt}{correction}"),
                    schema.clone(),
                    sources.clone(),
                )
                .await?;
            compiler_receipts.push(output.1);
            let seed: CompiledSeed = serde_json::from_value(output.0)?;
            match seed_to_campaign(seed.clone(), &receipts).and_then(|campaign| {
                validate_campaign_seed(&campaign)?;
                validate_opening_playability(&campaign)?;
                Ok(campaign)
            }) {
                Ok(_) => break seed,
                Err(error) if compiler_receipts.len() == 1 => {
                    mark_semantic_invalid(
                        compiler_receipts
                            .last_mut()
                            .expect("receipt was just stored"),
                        &error,
                    );
                    let previous_structure =
                        serde_json::to_string(&compiled_seed_structure(&seed))?;
                    correction = format!(
                        "\n\nLOCAL VALIDATOR REJECTED THE PREVIOUS CANDIDATE: {error}\nPREVIOUS_CANDIDATE_STRUCTURE:\n{previous_structure}\nReturn a corrected complete candidate against the same START and EVIDENCE. Preserve valid detail, but make every reference use an ID declared by the corrected candidate."
                    );
                }
                Err(error) => {
                    return Err(anyhow!(
                        "world compiler failed local validation after one correction: {error}"
                    ));
                }
            }
        };
        let (remote_institution_evidence, global_agency_gaps) =
            merge_global_agency_catalog(&mut seed, global_catalog)?;
        let remote_institution_ids = remote_institution_evidence
            .keys()
            .cloned()
            .collect::<BTreeSet<_>>();
        let all_receipts = merge_evidence_receipts(&receipts, &global_receipts);
        let evidence_coverage = merge_global_evidence_coverage(evidence_coverage, &global_receipts);
        let mut campaign = seed_to_campaign(seed.clone(), &all_receipts)?;
        apply_coarse_remote_agency_profiles(&mut campaign, &remote_institution_evidence)?;
        validate_campaign_seed(&campaign)?;
        validate_opening_playability(&campaign)?;
        let subject_briefs = agency_subject_briefs(&campaign, &remote_institution_ids);
        let modeled_subject_ids = subject_briefs
            .iter()
            .map(|brief| brief.subject_id.clone())
            .collect::<BTreeSet<_>>();
        let agency_prompt = format!(
            "MULTIRESOLUTION AGENCY SKELETON\nCompile only this exact, already validated subject roster:\n{}\n\nReturn exactly one agency profile for every supplied subject and no other subject. Copy every subject_id, subject_kind, and location_ids exactly. Every profile must contain exactly the six facet axes geography, ideology, authority, economy_role, species_body, and information. Derive facets only from the supplied roster fields; use an explicit unknown value when they do not support a sharper claim. collective_authority_id must be null or one supplied subject ID; it denotes real shared authority, never mere alliance or proximity. Relations may use only supplied subject IDs and strength must be an integer from 1 through 100. Cross-faction relations never imply shared speech, knowledge, or authority. Preserve geographic, ideological, institutional, economic, biological, and information boundaries that predict different behavior under pressure.",
            serde_json::to_string(&subject_briefs)?
        );
        let agency_schema = serde_json::to_value(schema_for!(CompiledAgencySkeleton))?;
        let mut agency_correction = String::new();
        let agency_sources = receipt_ids(&all_receipts);
        while !subject_briefs.is_empty() {
            let output = self
                .structured(
                    "agency_compile",
                    "custom-start",
                    &format!("{agency_prompt}{agency_correction}"),
                    agency_schema.clone(),
                    agency_sources.clone(),
                )
                .await?;
            compiler_receipts.push(output.1);
            let skeleton: CompiledAgencySkeleton = serde_json::from_value(output.0)?;
            let mut candidate = campaign.clone();
            match apply_compiled_agency_skeleton(
                &mut candidate,
                &modeled_subject_ids,
                skeleton.agency_profiles,
                skeleton.agency_relations,
            )
            .and_then(|_| validate_campaign_seed(&candidate))
            {
                Ok(()) => {
                    campaign = candidate;
                    break;
                }
                Err(error) if agency_correction.is_empty() => {
                    mark_semantic_invalid(
                        compiler_receipts
                            .last_mut()
                            .expect("receipt was just stored"),
                        &error,
                    );
                    agency_correction = format!(
                        "\n\nLOCAL VALIDATOR REJECTED THE PREVIOUS AGENCY SKELETON: {error}\nReturn one corrected complete agency skeleton for the same exact roster."
                    );
                }
                Err(error) => {
                    return Err(anyhow!(
                        "agency compiler failed local validation after one correction: {error}"
                    ));
                }
            }
        }
        let mut model_receipts = vec![retrieval_receipt];
        model_receipts.extend(relevance_receipts);
        model_receipts.extend(global_catalog_receipts);
        model_receipts.extend(compiler_receipts);
        Ok((
            WorldCompilePreview {
                schema: "ghostlight.world_compile_preview.v1".into(),
                title: seed.title,
                campaign,
                evidence_receipts: all_receipts,
                evidence_coverage,
                gaps: seed.gaps.into_iter().chain(global_agency_gaps).collect(),
                branch_assumptions: seed.branch_assumptions,
                requires_approval: true,
            },
            model_receipts,
        ))
    }

    pub async fn compile_selected(
        &self,
        start: SelectedStart,
    ) -> Result<(WorldCompilePreview, Vec<ModelStageReceipt>)> {
        let role = start.role.clone();
        let (mut preview, receipts) = self
            .compile_custom(CustomStart {
                campaign_name: start.campaign_name,
                who: format!("{} — {}", start.role.name, start.role.premise),
                where_: start.opening.place,
                when: start.opening.era,
                goal: format!("{}; {}", start.opening.player_hook, start.opening.pressure),
            })
            .await?;
        let player_id = preview.campaign.player_actor_id.clone();
        let player = preview
            .campaign
            .actors
            .get_mut(&player_id)
            .ok_or_else(|| anyhow!("compiled campaign lost its player actor"))?;
        player.capabilities.extend(role.capabilities.clone());
        player.obligations.extend(role.obligations.clone());
        preview.branch_assumptions.push(format!(
            "The approved generated role '{}' grants the player capabilities [{}] and obligations [{}].",
            role.name,
            role.capabilities.join(", "),
            role.obligations.join(", ")
        ));
        validate_campaign_seed(&preview.campaign)?;
        Ok((preview, receipts))
    }

    pub async fn compile_fission(
        &self,
        campaign: &Campaign,
        request: GestaltFissionRequest,
    ) -> Result<(
        crate::domain::GestaltFissionPreview,
        Vec<VaultEvidenceReceipt>,
        Vec<ModelStageReceipt>,
    )> {
        let requested = validate_fission_request(&request)?;
        let parent = campaign
            .gestalts
            .get(&request.parent_gestalt_id)
            .ok_or_else(|| anyhow!("fission parent is unknown"))?;
        let subject = serde_json::json!({
            "request":request,
            "parent":parent,
            "member_deltas":campaign.gestalt_members.values().filter(|member| member.gestalt_id == parent.id).collect::<Vec<_>>(),
            "campaign_time":campaign.world_time,
            "canon_cutoff":campaign.branch_origin.canon_cutoff
        });
        let (queries, retrieval_receipt) = self
            .plan_queries(
                "gestalt_fission_retrieval_plan",
                &format!("fission:{}:{}", campaign.id, parent.id),
                &serde_json::to_string(&subject)?,
                3,
            )
            .await?;
        let receipts = self
            .retrieve_all(&queries, &campaign.branch_origin.canon_cutoff, 12)
            .await?;
        let base_prompt = format!(
            "Refine one canonical leaf gestalt along exactly the requested facet. Produce one child per requested value plus one mandatory child whose value is exactly 'other/unknown'. Children inherit the parent baseline and contain only justified refinements; every child starts at version 0 and uses an existing campaign location. Do not erase or rewrite member deltas. Assign a member only when evidence or durable existing delta supports the cut; unassigned members will remain in other/unknown. List every material lore gap. This is an approval preview, not a commit. SUBJECT:\n{}\nEVIDENCE:\n{}",
            serde_json::to_string(&subject)?,
            evidence_text(&receipts),
        );
        let schema = serde_json::to_value(schema_for!(CompiledFissionSeed))?;
        let mut stages = vec![retrieval_receipt];
        let mut correction = String::new();
        for attempt in 0..2 {
            let (value, stage) = self
                .structured(
                    "gestalt_fission_compile",
                    &format!(
                        "campaign:{}:revision:{}:fission:{}",
                        campaign.id, campaign.revision, parent.id
                    ),
                    &format!("{base_prompt}{correction}"),
                    schema.clone(),
                    receipt_ids(&receipts),
                )
                .await?;
            stages.push(stage);
            let compiled: CompiledFissionSeed = serde_json::from_value(value)?;
            let residual_child_id = compiled
                .child_partition_values
                .iter()
                .find(|(_, value)| value.trim().eq_ignore_ascii_case("other/unknown"))
                .map(|(id, _)| id.clone());
            let gaps = compiled.gaps.clone();
            let evidence_receipt_ids = receipt_ids(&receipts);
            let affected_sources: Vec<_> = receipts
                .iter()
                .flat_map(|receipt| {
                    receipt
                        .witnesses
                        .iter()
                        .map(|witness| witness.source_id.clone())
                })
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect();
            let canon_candidates = gaps
                .iter()
                .enumerate()
                .map(|(index, gap)| crate::domain::CanonCandidate {
                    schema: "ghostlight.canon_candidate.v1".into(),
                    id: format!(
                        "canon-candidate:{}:fission:{}:{}",
                        campaign.id,
                        parent.id,
                        index + 1
                    ),
                    originating_campaign_id: campaign.id,
                    gap: gap.clone(),
                    evidence_receipt_ids: evidence_receipt_ids.clone(),
                    conflicts: vec![],
                    proposed_wording: format!(
                        "Clarify population division for {}: {gap}",
                        parent.name
                    ),
                    affected_vault_sources: affected_sources.clone(),
                    status: "review".into(),
                })
                .collect();
            let preview = crate::domain::GestaltFissionPreview {
                schema: "ghostlight.gestalt_fission_preview.v1".into(),
                campaign_id: campaign.id,
                expected_world_revision: campaign.revision,
                parent_gestalt_id: parent.id.clone(),
                partition_axis: request.partition_axis.clone(),
                children: compiled.children,
                child_partition_values: compiled.child_partition_values,
                residual_child_id: residual_child_id.unwrap_or_default(),
                member_child_assignments: compiled.member_child_assignments,
                evidence_receipt_ids,
                gaps,
                canon_candidates,
                requires_approval: true,
            };
            let returned_values: BTreeSet<_> = preview
                .child_partition_values
                .values()
                .map(|value| value.trim().to_ascii_lowercase())
                .filter(|value| value != "other/unknown")
                .collect();
            match crate::resolution::validate_fission(campaign, &preview).and_then(|_| {
                if returned_values == requested {
                    Ok(())
                } else {
                    Err(anyhow!(
                        "fission did not preserve the requested enumerated cut"
                    ))
                }
            }) {
                Ok(()) => return Ok((preview, receipts, stages)),
                Err(error) if attempt == 0 => {
                    mark_semantic_invalid(
                        stages.last_mut().expect("receipt was just stored"),
                        &error,
                    );
                    correction = format!(
                        "\n\nLOCAL VALIDATOR REJECTED THE PREVIOUS FISSION: {error}\nReturn one corrected complete preview against the same subject and evidence."
                    );
                }
                Err(error) => {
                    return Err(anyhow!(
                        "gestalt fission failed local validation after one correction: {error}"
                    ));
                }
            }
        }
        unreachable!()
    }

    pub async fn compile_destination(
        &self,
        campaign: &Campaign,
        origin_location_id: &str,
        destination_request: &str,
    ) -> Result<(
        crate::domain::RegionExpansionPreview,
        Vec<ModelStageReceipt>,
    )> {
        validate_user_text("destination request", destination_request, 500)?;
        let origin = campaign
            .locations
            .get(origin_location_id)
            .ok_or_else(|| anyhow!("origin location is unknown"))?;
        let (queries, retrieval_receipt) = self
            .plan_queries(
                "destination_retrieval_plan",
                &format!("campaign:{}:revision:{}", campaign.id, campaign.revision),
                &serde_json::to_string(&serde_json::json!({
                    "origin": origin,
                    "destination": destination_request,
                    "canon_cutoff": campaign.branch_origin.canon_cutoff,
                }))?,
                2,
            )
            .await?;
        let receipts = self
            .retrieve_all(&queries, &campaign.branch_origin.canon_cutoff, 10)
            .await?;
        let snapshot = format!("campaign:{}:revision:{}", campaign.id, campaign.revision);
        let base_prompt = format!(
            "Compile only the requested bounded destination region. Every new location id must be new. At least one new location must route back to origin id {} with a positive travel time. Do not rewrite existing geography. Any locally observable clue must already exist as a fact and list exact discoverable_at_location_ids from the combined existing and new topology; later action assessment can reveal facts but cannot invent them. CAMPAIGN LOCATIONS:\n{}\nREQUEST:\n{}\nEVIDENCE:\n{}",
            origin_location_id,
            serde_json::to_string(&campaign.locations)?,
            destination_request,
            evidence_text(&receipts)
        );
        let schema = serde_json::to_value(schema_for!(CompiledExpansionSeed))?;
        let sources = receipt_ids(&receipts);
        let mut compiler_receipts = Vec::new();
        let mut correction = String::new();
        let (seed, expansion) = loop {
            let output = self
                .structured(
                    "destination_compile",
                    &snapshot,
                    &format!("{base_prompt}{correction}"),
                    schema.clone(),
                    sources.clone(),
                )
                .await?;
            compiler_receipts.push(output.1);
            let seed: CompiledExpansionSeed = serde_json::from_value(output.0)?;
            let expansion = crate::domain::RegionExpansion {
                origin_location_id: origin_location_id.into(),
                locations: seed.locations.clone(),
                facts: seed.facts.clone(),
            };
            match validate_region_expansion(campaign, &expansion) {
                Ok(()) => break (seed, expansion),
                Err(error) if compiler_receipts.len() == 1 => {
                    mark_semantic_invalid(
                        compiler_receipts
                            .last_mut()
                            .expect("receipt was just stored"),
                        &error,
                    );
                    correction = format!(
                        "\n\nLOCAL VALIDATOR REJECTED THE PREVIOUS CANDIDATE: {error}\nReturn a corrected complete candidate against the same CAMPAIGN, REQUEST, and EVIDENCE."
                    );
                }
                Err(error) => {
                    return Err(anyhow!(
                        "destination compiler failed local validation after one correction: {error}"
                    ));
                }
            }
        };
        let evidence_ids = receipt_ids(&receipts);
        let affected_sources: Vec<String> = receipts
            .iter()
            .flat_map(|r| r.witnesses.iter().map(|w| w.source_id.clone()))
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        let candidates = seed
            .gaps
            .iter()
            .enumerate()
            .map(|(index, gap)| crate::domain::CanonCandidate {
                schema: "ghostlight.canon_candidate.v1".into(),
                id: format!(
                    "canon-candidate:{}:r{}:{}",
                    campaign.id,
                    campaign.revision,
                    index + 1
                ),
                originating_campaign_id: campaign.id,
                gap: gap.clone(),
                evidence_receipt_ids: evidence_ids.clone(),
                conflicts: vec![],
                proposed_wording: format!("Clarify the documented answer to: {gap}"),
                affected_vault_sources: affected_sources.clone(),
                status: "review".into(),
            })
            .collect();
        Ok((
            crate::domain::RegionExpansionPreview {
                schema: "ghostlight.region_expansion_preview.v1".into(),
                campaign_id: campaign.id,
                expected_revision: campaign.revision,
                expansion,
                evidence_receipts: receipts,
                gaps: seed.gaps,
                canon_candidates: candidates,
                requires_approval: true,
            },
            std::iter::once(retrieval_receipt)
                .chain(compiler_receipts)
                .collect(),
        ))
    }

    async fn retrieve_all(
        &self,
        queries: &[String],
        temporal_scope: &str,
        limit: u8,
    ) -> Result<Vec<VaultEvidenceReceipt>> {
        let mut receipts = Vec::new();
        for query in queries {
            receipts.push(
                self.vault
                    .search(&VaultQuery {
                        query: query.clone(),
                        authority_lanes: vec!["Aetheria".into(), "AetheriaLore".into()],
                        temporal_scope: temporal_scope.into(),
                        limit,
                    })
                    .await?,
            );
        }
        if receipts.iter().all(|r| r.witnesses.is_empty()) {
            return Err(anyhow!("Vault returned no evidence; compilation refused"));
        }
        let source_ids = receipts
            .iter()
            .flat_map(|receipt| {
                receipt
                    .witnesses
                    .iter()
                    .map(|witness| witness.source_id.clone())
            })
            .collect::<BTreeSet<_>>();
        let mut tasks = tokio::task::JoinSet::new();
        for source_id in source_ids {
            let vault = self.vault.clone();
            tasks.spawn(async move {
                let exact = vault.exact_document(&source_id).await?;
                Ok::<_, anyhow::Error>((source_id, exact))
            });
        }
        let mut exact_documents = BTreeMap::new();
        while let Some(result) = tasks.join_next().await {
            let (source_id, exact) = result??;
            exact_documents.insert(source_id, exact);
        }
        for witness in receipts
            .iter_mut()
            .flat_map(|receipt| receipt.witnesses.iter_mut())
        {
            let exact = exact_documents
                .get(&witness.source_id)
                .ok_or_else(|| anyhow!("Vault omitted exact document for {}", witness.source_id))?;
            if !normalized_contains(&exact.excerpt, &witness.excerpt) {
                return Err(anyhow!(
                    "retrieval excerpt is not witnessed by exact document {}",
                    witness.source_id
                ));
            }
            witness.content_hash = exact.content_hash.clone();
        }
        Ok(receipts)
    }

    async fn plan_queries(
        &self,
        stage: &str,
        binding: &str,
        subject: &str,
        count: usize,
    ) -> Result<(Vec<String>, ModelStageReceipt)> {
        let schema = serde_json::to_value(schema_for!(RetrievalQueryPlan))?;
        let prompt = format!(
            "OUTPUT JSON SCHEMA (follow exactly):\n{}\n\nPlan exactly {count} distinct source-search queries for the supplied subject. Each query must be a concise natural-language search string of 1 to 240 Unicode characters. Preserve proper nouns, era, place, institutions, mechanics, geography, and pressure when relevant. Do not answer the subject. SUBJECT:\n{subject}",
            serde_json::to_string(&schema)?
        );
        let output = run_validated_stage(
            self.model.as_ref(),
            &ModelStageRequest {
                stage: stage.into(),
                model: self.retrieval_model.clone(),
                snapshot_binding: binding.into(),
                lived_stream: prompt,
                output_schema: Some(schema),
                source_receipt_ids: vec![],
                temperature: Some(0.0),
                max_output_tokens: Some(512),
            },
        )
        .await?;
        let plan: RetrievalQueryPlan = serde_json::from_value(
            output
                .structured
                .ok_or_else(|| anyhow!("retrieval planner returned no structured output"))?,
        )?;
        let normalized = plan
            .queries
            .into_iter()
            .map(|query| query.trim().to_owned())
            .collect::<Vec<_>>();
        let unique = normalized.iter().collect::<BTreeSet<_>>();
        if normalized.len() != count || unique.len() != count {
            return Err(anyhow!(
                "retrieval planner must return exactly {count} distinct queries"
            ));
        }
        if normalized
            .iter()
            .any(|query| query.is_empty() || query.chars().count() > 240)
        {
            return Err(anyhow!(
                "retrieval planner query must contain 1 to 240 characters"
            ));
        }
        Ok((normalized, output.receipt))
    }

    async fn plan_opening_queries(
        &self,
        request: &OpeningRequest,
    ) -> Result<(Vec<String>, ModelStageReceipt)> {
        let schema = serde_json::to_value(schema_for!(OpeningRetrievalPlan))?;
        let prompt = format!(
            "OUTPUT JSON SCHEMA (follow exactly):\n{}\n\nPlan three source-search queries for distinct historical frames in the requested setting. `early_frame_query` must seek the earliest well-documented playable period and its geography and pressure. `transition_frame_query` must seek a materially later transition, shunt, collapse, migration, or institutional realignment. `late_frame_query` must seek the latest well-documented playable period and a different geography and pressure. Use setting-specific terms from the request where available. Each value is only a concise natural-language search query of 1 to 240 Unicode characters; do not answer the request. REQUEST:\n{}",
            serde_json::to_string(&schema)?,
            serde_json::to_string(request)?
        );
        let output = run_validated_stage(
            self.model.as_ref(),
            &ModelStageRequest {
                stage: "opening_retrieval_plan".into(),
                model: self.retrieval_model.clone(),
                snapshot_binding: "opening-suggestions".into(),
                lived_stream: prompt,
                output_schema: Some(schema),
                source_receipt_ids: vec![],
                temperature: Some(0.0),
                max_output_tokens: Some(512),
            },
        )
        .await?;
        let plan: OpeningRetrievalPlan =
            serde_json::from_value(output.structured.ok_or_else(|| {
                anyhow!("opening retrieval planner returned no structured output")
            })?)?;
        let queries = vec![
            plan.early_frame_query.trim().to_owned(),
            plan.transition_frame_query.trim().to_owned(),
            plan.late_frame_query.trim().to_owned(),
        ];
        let unique = queries.iter().collect::<BTreeSet<_>>();
        if unique.len() != 3 {
            return Err(anyhow!(
                "opening retrieval planner must return three distinct historical-frame queries"
            ));
        }
        if queries
            .iter()
            .any(|query| query.is_empty() || query.chars().count() > 240)
        {
            return Err(anyhow!(
                "opening retrieval query must contain 1 to 240 characters"
            ));
        }
        Ok((queries, output.receipt))
    }

    async fn classify_evidence(
        &self,
        start: &CustomStart,
        receipts: &[VaultEvidenceReceipt],
    ) -> Result<(Vec<EvidenceCoverage>, Vec<ModelStageReceipt>)> {
        let mut source_briefs = BTreeMap::new();
        for witness in receipts.iter().flat_map(|receipt| &receipt.witnesses) {
            source_briefs
                .entry(witness.source_id.clone())
                .or_insert_with(|| {
                    serde_json::json!({
                        "source_id":witness.source_id,
                        "authority_lane":witness.authority_lane,
                        "temporal_scope":witness.temporal_scope,
                        "excerpt":witness.excerpt.chars().take(1_200).collect::<String>(),
                    })
                });
        }
        let expected: BTreeSet<_> = source_briefs.keys().cloned().collect();
        let authority_by_source = receipts
            .iter()
            .flat_map(|receipt| receipt.witnesses.iter())
            .map(|witness| (witness.source_id.clone(), witness.authority_lane.clone()))
            .collect::<BTreeMap<_, _>>();
        // Keep the provider schema stable across campaigns for prefix-cache reuse.
        // Exact membership and cardinality belong to the local validator below.
        let schema = serde_json::to_value(schema_for!(EvidenceUsePlan))?;
        let base_prompt = format!(
            "OUTPUT JSON SCHEMA (follow exactly):\n{}\n\nClassify every supplied source exactly once for this requested custom start. direct_seed means the source directly supports this specific local place, era, role, goal, pressure, or a causal actor/institution that should actually be present. setting_background means the source supports general setting history, mechanics, geography, or institution identity, but its story-specific cast, incident, clocks, goals, and postures must not be imported into the new branch. excluded means it is merely nearby in search space. A shared place name or era alone does not make another story episode current. Keep each rationale to one short sentence.\nSTART:\n{}\nSOURCES:\n{}",
            serde_json::to_string(&schema)?,
            serde_json::to_string(start)?,
            serde_json::to_string(&source_briefs.values().collect::<Vec<_>>())?,
        );
        let source_receipt_ids = receipt_ids(receipts);
        let mut stage_receipts = Vec::new();
        let mut correction = String::new();
        for attempt in 0..2 {
            let output = run_validated_stage(
                self.model.as_ref(),
                &ModelStageRequest {
                    stage: "evidence_relevance".into(),
                    model: self.retrieval_model.clone(),
                    snapshot_binding: "custom-start".into(),
                    lived_stream: format!("{base_prompt}{correction}"),
                    output_schema: Some(schema.clone()),
                    source_receipt_ids: source_receipt_ids.clone(),
                    temperature: Some(0.0),
                    max_output_tokens: Some(2_500),
                },
            )
            .await?;
            let candidate = output
                .structured
                .clone()
                .ok_or_else(|| anyhow!("evidence classifier returned no structured output"))
                .and_then(|value| serde_json::from_value::<EvidenceUsePlan>(value).map_err(Into::into))
                .and_then(|plan| {
                    let actual = plan
                        .coverage
                        .iter()
                        .map(|item| item.source_id.clone())
                        .collect::<BTreeSet<_>>();
                    if plan.coverage.len() != expected.len()
                        || actual != expected
                        || plan.coverage.iter().any(|item| item.rationale.trim().is_empty())
                    {
                        return Err(anyhow!(
                            "evidence classifier must cover every exact source once with a rationale"
                        ));
                    }
                    if let Some(item) = plan.coverage.iter().find(|item| {
                        item.lane == EvidenceUseLane::DirectSeed
                            && authority_by_source
                                .get(&item.source_id)
                                .is_some_and(|lane| !authority_allows_direct_seed(lane))
                    }) {
                        return Err(anyhow!(
                            "source {} belongs to authority lane {} and cannot seed a new branch directly",
                            item.source_id,
                            authority_by_source
                                .get(&item.source_id)
                                .expect("source coverage was validated")
                        ));
                    }
                    Ok(plan.coverage)
                });
            let mut receipt = output.receipt;
            match candidate {
                Ok(coverage) => {
                    stage_receipts.push(receipt);
                    return Ok((coverage, stage_receipts));
                }
                Err(error) if attempt == 0 => {
                    mark_semantic_invalid(&mut receipt, &error);
                    stage_receipts.push(receipt);
                    correction = format!(
                        "\n\nLOCAL VALIDATOR REJECTED THE PREVIOUS CLASSIFICATION: {error}\nReturn one corrected complete classification against the same START and SOURCES."
                    );
                }
                Err(error) => {
                    return Err(anyhow!(
                        "evidence classifier failed local validation after one correction: {error}"
                    ));
                }
            }
        }
        unreachable!()
    }

    async fn compile_global_agency_catalog(
        &self,
        start: &CustomStart,
        receipts: &[VaultEvidenceReceipt],
    ) -> Result<(CompiledGlobalAgencyCatalog, Vec<ModelStageReceipt>)> {
        let receipts = canonical_worldbuilding_receipts(receipts);
        let schema = serde_json::to_value(schema_for!(CompiledGlobalAgencyCatalog))?;
        let base_prompt = format!(
            "OUTPUT JSON SCHEMA (follow exactly):\n{}\n\nBuild the coarse remote strategic agency catalog for the requested historical horizon. This is a different authority lane from local world compilation: it may establish durable institutions, movements, governments, corporations, or other collective powers, but it must never import a story-specific cast, incident, clock, location state, capability inventory, or current branch posture. Include every major power and strategically distinct movement explicitly supported as relevant to this horizon by the supplied witnesses, up to 32 institutions. Do not emit an institution from a mere index link: omit it unless one supplied witness both names it and contains a durable mandate. For each admitted institution, copy its exact displayed name. mandate must be one short contiguous quotation, at most 320 characters, from that witness establishing a durable purpose, interest, or pressure it can act on. Copy the quotation exactly; do not paraphrase or identify its source because deterministic code binds it to the actual witness. Summarize classes of omitted institutions in gaps rather than emitting one gap per name. Return no narrative analysis. Fine resources and capabilities compile on demand only when the institution becomes causally relevant.\nHORIZON:\n{}\nREQUESTED PLACE (relevance only; not local authority):\n{}\nEVIDENCE:\n{}",
            serde_json::to_string(&schema)?,
            start.when,
            start.where_,
            bounded_evidence_text(&receipts, 1_200),
        );
        let source_receipt_ids = receipt_ids(&receipts);
        let mut stage_receipts = Vec::new();
        let mut correction = String::new();
        for attempt in 0..2 {
            let output = run_validated_stage(
                self.model.as_ref(),
                &ModelStageRequest {
                    stage: "global_agency_compile".into(),
                    model: self.retrieval_model.clone(),
                    snapshot_binding: format!("global-agency:{}", start.when),
                    lived_stream: format!("{base_prompt}{correction}"),
                    output_schema: Some(schema.clone()),
                    source_receipt_ids: source_receipt_ids.clone(),
                    temperature: Some(0.0),
                    max_output_tokens: Some(5_000),
                },
            )
            .await?;
            let candidate = output
                .structured
                .clone()
                .ok_or_else(|| anyhow!("global agency compiler returned no structured output"))
                .and_then(|value| {
                    serde_json::from_value::<CompiledGlobalAgencyCatalog>(value).map_err(Into::into)
                })
                .and_then(|catalog| ground_global_agency_catalog(catalog, &receipts));
            let mut receipt = output.receipt;
            match candidate {
                Ok((catalog, grounding_gaps)) => {
                    if !grounding_gaps.is_empty() {
                        receipt.validation_result = "valid_with_grounding_gaps".into();
                        receipt.local_validation_error =
                            Some(grounding_gaps.join("; ").chars().take(1_000).collect());
                    }
                    stage_receipts.push(receipt);
                    return Ok((catalog, stage_receipts));
                }
                Err(error) if attempt == 0 => {
                    mark_semantic_invalid(&mut receipt, &error);
                    stage_receipts.push(receipt);
                    correction = format!(
                        "\n\nLOCAL VALIDATOR REJECTED THE PREVIOUS GLOBAL AGENCY CATALOG: {error}\nReturn one corrected complete catalog against the same HORIZON and EVIDENCE."
                    );
                }
                Err(error) => {
                    return Err(anyhow!(
                        "global agency compiler failed local validation after one correction: {error}"
                    ));
                }
            }
        }
        unreachable!()
    }

    async fn structured(
        &self,
        stage: &str,
        binding: &str,
        prompt: &str,
        schema: serde_json::Value,
        sources: Vec<String>,
    ) -> Result<(serde_json::Value, ModelStageReceipt)> {
        let prompt = format!(
            "OUTPUT JSON SCHEMA (follow exactly):\n{}\n\nTASK CONTEXT:\n{prompt}",
            serde_json::to_string(&schema)?
        );
        let request = ModelStageRequest {
            stage: stage.into(),
            model: self.compiler_model.clone(),
            snapshot_binding: binding.into(),
            lived_stream: prompt,
            output_schema: Some(schema),
            source_receipt_ids: sources,
            temperature: Some(0.0),
            max_output_tokens: Some(match stage {
                "world_compile" => 6_000,
                "agency_compile" => 3_500,
                "world_openings" => 1_800,
                "world_roles" => 1_200,
                "destination_compile" => 3_000,
                "gestalt_fission" => 2_500,
                _ => 2_500,
            }),
        };
        let out = if stage == "world_compile" {
            run_validated_stage_with_timeout(
                self.model.as_ref(),
                &request,
                std::time::Duration::from_secs(120),
            )
            .await?
        } else {
            run_validated_stage(self.model.as_ref(), &request).await?
        };
        Ok((
            out.structured
                .ok_or_else(|| anyhow!("compiler returned no structured output"))?,
            out.receipt,
        ))
    }
}

fn validate_user_text(label: &str, value: &str, max_chars: usize) -> Result<()> {
    let value = value.trim();
    if value.is_empty()
        || value.chars().count() > max_chars
        || value
            .chars()
            .any(|character| character.is_control() && !matches!(character, '\n' | '\r' | '\t'))
    {
        return Err(anyhow!(
            "{label} must contain 1 to {max_chars} readable characters"
        ));
    }
    Ok(())
}

fn validate_fission_request(request: &GestaltFissionRequest) -> Result<BTreeSet<String>> {
    validate_user_text("fission reason", &request.reason, 500)?;
    if request.requested_partition_values.is_empty()
        || request.requested_partition_values.len() > 16
    {
        return Err(anyhow!("fission request needs between 1 and 16 named cuts"));
    }
    for value in &request.requested_partition_values {
        validate_user_text("fission cut", value, 160)?;
    }
    let requested: BTreeSet<_> = request
        .requested_partition_values
        .iter()
        .map(|value| value.trim().to_ascii_lowercase())
        .collect();
    if requested.len() != request.requested_partition_values.len()
        || requested.contains("other/unknown")
    {
        return Err(anyhow!(
            "fission request needs distinct named cuts and reserves other/unknown for the compiler"
        ));
    }
    Ok(requested)
}

fn global_agency_queries(start: &CustomStart) -> Vec<String> {
    let horizon = start.when.chars().take(120).collect::<String>();
    vec![
        format!(
            "major powers factions institutions and movements active during {horizon} overview index"
        ),
        format!(
            "strategic specialist organizations populations regions and information channels during {horizon}"
        ),
    ]
}

fn authority_allows_direct_seed(authority_lane: &str) -> bool {
    matches!(
        authority_lane,
        "aetheria.canon_worldbuilding" | "aetheria.vault_document" | "AetheriaLore"
    )
}

fn canonical_worldbuilding_receipts(
    receipts: &[VaultEvidenceReceipt],
) -> Vec<VaultEvidenceReceipt> {
    receipts
        .iter()
        .filter_map(|receipt| {
            let mut filtered = receipt.clone();
            filtered.witnesses.retain(|witness| {
                matches!(
                    witness.authority_lane.as_str(),
                    "aetheria.canon_worldbuilding" | "aetheria.vault_document" | "AetheriaLore"
                )
            });
            (!filtered.witnesses.is_empty()).then_some(filtered)
        })
        .collect()
}

fn bounded_evidence_text(receipts: &[VaultEvidenceReceipt], max_chars: usize) -> String {
    let mut seen = BTreeSet::new();
    receipts
        .iter()
        .flat_map(|receipt| {
            receipt
                .witnesses
                .iter()
                .map(move |witness| (receipt.id.as_str(), witness))
        })
        .filter(|(_, witness)| {
            seen.insert((
                witness.source_id.clone(),
                witness.exact_locator.clone(),
                witness.content_hash.clone(),
            ))
        })
        .map(|(receipt_id, witness)| {
            let excerpt = witness.excerpt.chars().take(max_chars).collect::<String>();
            format!(
                "[receipt_id={} | source={} | locator={} | content_hash={}] {}",
                receipt_id, witness.source_id, witness.exact_locator, witness.content_hash, excerpt,
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}

fn ground_global_agency_catalog(
    mut catalog: CompiledGlobalAgencyCatalog,
    receipts: &[VaultEvidenceReceipt],
) -> Result<(CompiledGlobalAgencyCatalog, Vec<String>)> {
    if catalog.institutions.len() > 64 {
        return Err(anyhow!(
            "global agency candidate pool exceeds 64 institutions"
        ));
    }
    if catalog.institutions.is_empty() && catalog.gaps.is_empty() {
        return Err(anyhow!(
            "global agency catalog must contain witnessed institutions or an explicit evidence gap"
        ));
    }
    if catalog
        .gaps
        .iter()
        .any(|gap| gap.trim().is_empty() || gap.chars().count() > 500)
    {
        return Err(anyhow!("global agency catalog contains a malformed gap"));
    }
    let by_source = receipts
        .iter()
        .flat_map(|receipt| receipt.witnesses.iter())
        .fold(BTreeMap::<&str, Vec<&str>>::new(), |mut map, witness| {
            map.entry(witness.source_id.as_str())
                .or_default()
                .push(witness.excerpt.as_str());
            map
        });
    let receipt_ids_by_source = receipts.iter().fold(
        BTreeMap::<&str, BTreeSet<String>>::new(),
        |mut map, receipt| {
            for witness in &receipt.witnesses {
                map.entry(witness.source_id.as_str())
                    .or_default()
                    .insert(receipt.id.clone());
            }
            map
        },
    );
    let mut names = BTreeSet::new();
    for institution in &catalog.institutions {
        let normalized_name = institution
            .name
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .to_lowercase();
        if normalized_name.is_empty()
            || institution.name.chars().count() > 160
            || !names.insert(normalized_name.clone())
        {
            return Err(anyhow!(
                "global agency institution names must be non-empty, bounded, and unique"
            ));
        }
    }
    let mut admitted = Vec::new();
    let mut grounding_gaps = Vec::new();
    let mut omitted_names = Vec::new();
    for mut institution in std::mem::take(&mut catalog.institutions) {
        let normalized_name = institution
            .name
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .to_lowercase();
        let mandate_sources = matching_agency_claim_sources(&institution.mandate, &by_source)?;
        if mandate_sources.is_empty() {
            grounding_gaps.push(format!(
                "{} had no exact mandate quotation in the supplied witnesses",
                institution.name
            ));
            omitted_names.push(institution.name);
            continue;
        }
        let mandate_source_names_institution = mandate_sources.iter().any(|source_id| {
            by_source
                .get(source_id)
                .into_iter()
                .flatten()
                .any(|witness| witness.to_lowercase().contains(&normalized_name))
        });
        if !mandate_source_names_institution {
            grounding_gaps.push(format!(
                "{} mandate witnesses did not name that institution",
                institution.name
            ));
            omitted_names.push(institution.name);
            continue;
        }
        institution.evidence_receipt_ids = mandate_sources
            .into_iter()
            .flat_map(|source_id| {
                receipt_ids_by_source
                    .get(source_id)
                    .into_iter()
                    .flatten()
                    .cloned()
            })
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        if institution.evidence_receipt_ids.is_empty() {
            return Err(anyhow!(
                "grounded remote institution has no exact evidence receipt"
            ));
        }
        admitted.push(institution);
    }
    if admitted.len() > 32 {
        let overflow = admitted.len() - 32;
        admitted.truncate(32);
        grounding_gaps.push(format!(
            "{overflow} grounded remote agency candidates exceeded the 32-institution simulation catalog capacity"
        ));
        catalog.gaps.push(format!(
            "{overflow} additional source-grounded institutions were omitted at this horizon because the remote agency catalog is capped at 32; they remain available for on-demand compilation."
        ));
    }
    catalog.institutions = admitted;
    if !omitted_names.is_empty() {
        catalog.gaps.push(format!(
            "{} remote agency candidates were omitted because their mandates could not be bound to witnesses naming them; exact rejection details remain in the private model-stage receipt.",
            omitted_names.len()
        ));
    }
    Ok((catalog, grounding_gaps))
}

fn matching_agency_claim_sources<'a>(
    claim: &str,
    by_source: &'a BTreeMap<&'a str, Vec<&'a str>>,
) -> Result<Vec<&'a str>> {
    if claim.trim().is_empty() || claim.chars().count() > 320 {
        return Err(anyhow!(
            "global agency claim must contain 1 to 320 characters"
        ));
    }
    let matches = by_source
        .iter()
        .filter(|(_, witnesses)| {
            witnesses
                .iter()
                .any(|witness| normalized_contains(witness, claim))
        })
        .map(|(source_id, _)| *source_id)
        .collect::<Vec<_>>();
    Ok(matches)
}

fn merge_global_agency_catalog(
    seed: &mut CompiledSeed,
    catalog: CompiledGlobalAgencyCatalog,
) -> Result<(BTreeMap<String, Vec<String>>, Vec<String>)> {
    let mut known_names = seed
        .institutions
        .iter()
        .map(|institution| institution.name.to_lowercase())
        .collect::<BTreeSet<_>>();
    let mut known_ids = seed
        .institutions
        .iter()
        .map(|institution| institution.id.clone())
        .collect::<BTreeSet<_>>();
    let mut remote_evidence = BTreeMap::new();
    for institution in catalog.institutions {
        if !known_names.insert(institution.name.to_lowercase()) {
            continue;
        }
        let digest = format!("{:x}", Sha256::digest(institution.name.as_bytes()));
        let id = format!("remote-institution:{}", &digest[..16]);
        if !known_ids.insert(id.clone()) {
            return Err(anyhow!("global agency institution ID collision"));
        }
        seed.institutions.push(InstitutionState {
            id: id.clone(),
            name: institution.name,
            resources: vec![],
            goals: vec![institution.mandate],
            posture: "No branch-local posture has been established.".into(),
        });
        remote_evidence.insert(id, institution.evidence_receipt_ids);
    }
    let gaps = catalog
        .gaps
        .into_iter()
        .map(|gap| format!("Global agency evidence gap: {gap}"))
        .collect();
    Ok((remote_evidence, gaps))
}

fn apply_coarse_remote_agency_profiles(
    campaign: &mut Campaign,
    remote_institution_evidence: &BTreeMap<String, Vec<String>>,
) -> Result<()> {
    let axes = [
        (AgencyAxis::Geography, "remote/unknown"),
        (AgencyAxis::Ideology, "unknown"),
        (AgencyAxis::Authority, "self-governing institution"),
        (AgencyAxis::EconomyRole, "unknown"),
        (AgencyAxis::SpeciesBody, "institutional collective"),
        (AgencyAxis::Information, "unknown"),
    ];
    for (institution_id, evidence_receipt_ids) in remote_institution_evidence {
        let profile = campaign
            .agency_profiles
            .get_mut(institution_id)
            .ok_or_else(|| anyhow!("remote agency profile has no canonical institution"))?;
        if profile.subject_kind != AgencySubjectKind::Institution {
            return Err(anyhow!("remote agency profile has the wrong subject kind"));
        }
        profile.collective_authority_id = Some(institution_id.clone());
        profile.facets = axes
            .iter()
            .map(|(axis, value)| (axis.clone(), BTreeSet::from([(*value).into()])))
            .collect();
        profile.facets.insert(
            AgencyAxis::Authority,
            BTreeSet::from([institution_id.clone()]),
        );
        profile.information_channels.clear();
        profile.evidence_receipt_ids = evidence_receipt_ids.clone();
    }
    Ok(())
}

fn merge_evidence_receipts(
    local: &[VaultEvidenceReceipt],
    global: &[VaultEvidenceReceipt],
) -> Vec<VaultEvidenceReceipt> {
    let mut seen = BTreeSet::new();
    local
        .iter()
        .chain(global)
        .filter(|receipt| seen.insert(receipt.id.clone()))
        .cloned()
        .collect()
}

fn merge_global_evidence_coverage(
    local: Vec<EvidenceCoverage>,
    global: &[VaultEvidenceReceipt],
) -> Vec<EvidenceCoverage> {
    let mut coverage = local
        .into_iter()
        .map(|item| (item.source_id.clone(), item))
        .collect::<BTreeMap<_, _>>();
    for source_id in global
        .iter()
        .flat_map(|receipt| receipt.witnesses.iter())
        .map(|witness| witness.source_id.clone())
        .collect::<BTreeSet<_>>()
    {
        coverage
            .entry(source_id.clone())
            .and_modify(|item| {
                if item.lane == EvidenceUseLane::Excluded {
                    item.lane = EvidenceUseLane::SettingBackground;
                    item.rationale =
                        "Supports the remote agency catalog, not the local seed.".into();
                }
            })
            .or_insert(EvidenceCoverage {
                source_id,
                lane: EvidenceUseLane::SettingBackground,
                rationale: "Supports the remote agency catalog, not the local seed.".into(),
            });
    }
    coverage.into_values().collect()
}

fn normalized_contains(document: &str, excerpt: &str) -> bool {
    let document = document.split_whitespace().collect::<Vec<_>>().join(" ");
    let excerpt = excerpt.split_whitespace().collect::<Vec<_>>().join(" ");
    !excerpt.is_empty() && document.contains(&excerpt)
}

fn mark_semantic_invalid(receipt: &mut ModelStageReceipt, error: &impl std::fmt::Display) {
    receipt.validation_result = "semantic_invalid".into();
    receipt.local_validation_error = Some(error.to_string().chars().take(1_000).collect());
}

pub fn validate_region_expansion(
    campaign: &Campaign,
    expansion: &crate::domain::RegionExpansion,
) -> Result<()> {
    if !campaign
        .locations
        .contains_key(&expansion.origin_location_id)
    {
        return Err(anyhow!("destination expansion origin is unknown"));
    }
    let new_ids: BTreeSet<_> = expansion.locations.iter().map(|x| x.id.as_str()).collect();
    if expansion.locations.is_empty() || new_ids.len() != expansion.locations.len() {
        return Err(anyhow!("destination expansion has no unique locations"));
    }
    if new_ids
        .iter()
        .any(|id| campaign.locations.contains_key(*id))
    {
        return Err(anyhow!(
            "destination expansion collides with stable topology"
        ));
    }
    let known = |id: &str| campaign.locations.contains_key(id) || new_ids.contains(id);
    let mut attached = false;
    for location in &expansion.locations {
        for route in location.routes.values() {
            if route.travel_minutes == 0 || !known(&route.destination_id) {
                return Err(anyhow!("destination expansion has a dangling route"));
            }
            if route.destination_id == expansion.origin_location_id {
                attached = true;
            }
        }
    }
    if !attached {
        return Err(anyhow!("destination expansion is not attached to origin"));
    }
    let existing_fact_ids = campaign.facts.keys().collect::<BTreeSet<_>>();
    let mut new_fact_ids = BTreeSet::new();
    let mut fact_statements = campaign
        .facts
        .values()
        .map(|fact| fact.statement.clone())
        .collect::<BTreeSet<_>>();
    for fact in &expansion.facts {
        if fact.id.trim().is_empty()
            || existing_fact_ids.contains(&fact.id)
            || !new_fact_ids.insert(fact.id.clone())
            || fact.statement.trim().is_empty()
            || !fact_statements.insert(fact.statement.clone())
        {
            return Err(anyhow!(
                "destination expansion facts must have new IDs and non-empty unique statements"
            ));
        }
        if fact
            .discoverable_at_location_ids
            .iter()
            .any(|id| !known(id))
        {
            return Err(anyhow!(
                "destination expansion fact {} has an unknown discovery location",
                fact.id
            ));
        }
    }
    Ok(())
}

fn evidence_text(receipts: &[VaultEvidenceReceipt]) -> String {
    let mut seen = BTreeSet::new();
    receipts
        .iter()
        .flat_map(|receipt| {
            receipt
                .witnesses
                .iter()
                .map(move |witness| (receipt.id.as_str(), witness))
        })
        .filter(|(_, witness)| {
            seen.insert((
                witness.source_id.clone(),
                witness.exact_locator.clone(),
                witness.content_hash.clone(),
            ))
        })
        .map(|(receipt_id, witness)| {
            format!(
                "[receipt_id={} | source={} | locator={} | content_hash={}] {}",
                receipt_id,
                witness.source_id,
                witness.exact_locator,
                witness.content_hash,
                witness.excerpt
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}

fn opening_evidence_text(queries: &[String], receipts: &[VaultEvidenceReceipt]) -> String {
    const FRAME_LABELS: [&str; 3] = ["early", "transition", "late"];
    queries
        .iter()
        .zip(receipts)
        .zip(FRAME_LABELS)
        .map(|((query, receipt), frame)| {
            let witnesses = evidence_text(std::slice::from_ref(receipt));
            format!("[historical_frame={frame} | retrieval_query={query}]\n{witnesses}")
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}

fn direct_seed_evidence_text(
    receipts: &[VaultEvidenceReceipt],
    coverage: &[EvidenceCoverage],
) -> String {
    let coverage = coverage
        .iter()
        .map(|item| (item.source_id.as_str(), item))
        .collect::<BTreeMap<_, _>>();
    let mut seen = BTreeSet::new();
    receipts
        .iter()
        .flat_map(|receipt| {
            receipt
                .witnesses
                .iter()
                .map(move |witness| (receipt.id.as_str(), witness))
        })
        .filter_map(|(receipt_id, witness)| {
            let use_plan = coverage.get(witness.source_id.as_str())?;
            if use_plan.lane != EvidenceUseLane::DirectSeed
                || !authority_allows_direct_seed(&witness.authority_lane)
                || !seen.insert((
                    witness.source_id.clone(),
                    witness.exact_locator.clone(),
                    witness.content_hash.clone(),
                ))
            {
                return None;
            }
            Some(format!(
                "[usage_lane=direct_seed | rationale={} | receipt_id={} | source={} | locator={} | content_hash={}] {}",
                use_plan.rationale,
                receipt_id,
                witness.source_id,
                witness.exact_locator,
                witness.content_hash,
                witness.excerpt
            ))
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}

fn receipt_ids_for_coverage(
    receipts: &[VaultEvidenceReceipt],
    coverage: &[EvidenceCoverage],
) -> Vec<String> {
    let included_sources = coverage
        .iter()
        .filter(|item| item.lane == EvidenceUseLane::DirectSeed)
        .map(|item| item.source_id.as_str())
        .collect::<BTreeSet<_>>();
    receipts
        .iter()
        .filter(|receipt| {
            receipt.witnesses.iter().any(|witness| {
                included_sources.contains(witness.source_id.as_str())
                    && authority_allows_direct_seed(&witness.authority_lane)
            })
        })
        .map(|receipt| receipt.id.clone())
        .collect()
}

fn receipt_ids(receipts: &[VaultEvidenceReceipt]) -> Vec<String> {
    receipts.iter().map(|r| r.id.clone()).collect()
}
fn ensure_distinct_openings(items: &[OpeningSuggestion]) -> Result<()> {
    ensure_distinct_fields(
        "openings",
        [
            ("era", items.iter().map(|x| x.era.as_str()).collect()),
            ("place", items.iter().map(|x| x.place.as_str()).collect()),
            (
                "pressure",
                items.iter().map(|x| x.pressure.as_str()).collect(),
            ),
        ],
    )
}

fn ensure_distinct_roles(items: &[RoleSuggestion]) -> Result<()> {
    ensure_distinct_fields(
        "roles",
        [
            ("name", items.iter().map(|x| x.name.as_str()).collect()),
            (
                "premise",
                items.iter().map(|x| x.premise.as_str()).collect(),
            ),
        ],
    )
}

fn validate_opening_suggestions(items: &[OpeningSuggestion], receipts: &[String]) -> Result<()> {
    ensure_distinct_openings(items)?;
    let mut ids = BTreeSet::new();
    for item in items {
        validate_user_text("opening id", &item.id, 160)?;
        validate_user_text("opening title", &item.title, 160)?;
        validate_user_text("opening era", &item.era, 160)?;
        validate_user_text("opening place", &item.place, 240)?;
        validate_user_text("opening pressure", &item.pressure, 500)?;
        validate_user_text("opening player hook", &item.player_hook, 500)?;
        if !ids.insert(item.id.trim().to_owned()) {
            return Err(anyhow!("opening ids must be unique"));
        }
        validate_suggestion_evidence("opening", &item.evidence_receipt_ids, receipts)?;
    }
    Ok(())
}

fn validate_role_suggestions(items: &[RoleSuggestion], receipts: &[String]) -> Result<()> {
    ensure_distinct_roles(items)?;
    let mut ids = BTreeSet::new();
    for item in items {
        validate_user_text("role id", &item.id, 160)?;
        validate_user_text("role name", &item.name, 160)?;
        validate_user_text("role premise", &item.premise, 500)?;
        if !ids.insert(item.id.trim().to_owned()) {
            return Err(anyhow!("role ids must be unique"));
        }
        if item.capabilities.is_empty()
            || item.capabilities.len() > 8
            || item.obligations.is_empty()
            || item.obligations.len() > 8
        {
            return Err(anyhow!(
                "each role needs between 1 and 8 capabilities and obligations"
            ));
        }
        for capability in &item.capabilities {
            validate_user_text("role capability", capability, 160)?;
        }
        for obligation in &item.obligations {
            validate_user_text("role obligation", obligation, 160)?;
        }
        validate_suggestion_evidence("role", &item.evidence_receipt_ids, receipts)?;
    }
    Ok(())
}

fn validate_suggestion_evidence(
    label: &str,
    supplied: &[String],
    allowed: &[String],
) -> Result<()> {
    let unique = supplied.iter().collect::<BTreeSet<_>>();
    let allowed = allowed.iter().collect::<BTreeSet<_>>();
    if supplied.len() > 8 || unique.len() != supplied.len() || !unique.is_subset(&allowed) {
        return Err(anyhow!(
            "{label} evidence may contain at most 8 unique supplied receipt ids"
        ));
    }
    Ok(())
}

fn ensure_distinct_fields<const N: usize>(
    subject: &str,
    axes: [(&str, Vec<&str>); N],
) -> Result<()> {
    let mut collisions = Vec::new();
    for (axis, values) in axes {
        let mut counts = BTreeMap::new();
        for value in values {
            *counts.entry(value.trim().to_lowercase()).or_insert(0usize) += 1;
        }
        for (value, count) in counts {
            if count > 1 {
                collisions.push(format!("{axis}={value:?} repeated {count} times"));
            }
        }
    }
    if collisions.is_empty() {
        Ok(())
    } else {
        Err(anyhow!(
            "{subject} contain duplicate axes: {}",
            collisions.join("; ")
        ))
    }
}

fn seed_to_campaign(seed: CompiledSeed, receipts: &[VaultEvidenceReceipt]) -> Result<Campaign> {
    require_unique_ids(
        "location",
        seed.locations.iter().map(|item| item.id.as_str()),
    )?;
    require_unique_ids("actor", seed.actors.iter().map(|item| item.id.as_str()))?;
    require_unique_ids(
        "institution",
        seed.institutions.iter().map(|item| item.id.as_str()),
    )?;
    require_unique_ids("clock", seed.clocks.iter().map(|item| item.id.as_str()))?;
    require_unique_ids("fact", seed.facts.iter().map(|item| item.id.as_str()))?;
    require_unique_ids("gestalt", seed.gestalts.iter().map(|item| item.id.as_str()))?;
    require_unique_ids(
        "gestalt member",
        seed.gestalt_members.iter().map(|item| item.id.as_str()),
    )?;
    require_unique_ids(
        "canonical subject",
        std::iter::once(seed.player.id.as_str())
            .chain(seed.actors.iter().map(|item| item.id.as_str()))
            .chain(seed.institutions.iter().map(|item| item.id.as_str()))
            .chain(seed.gestalts.iter().map(|item| item.id.as_str())),
    )?;
    require_unique_ids(
        "actor or gestalt member",
        std::iter::once(seed.player.id.as_str())
            .chain(seed.actors.iter().map(|item| item.id.as_str()))
            .chain(seed.gestalt_members.iter().map(|item| item.id.as_str())),
    )?;
    let id = Uuid::new_v4();
    let player_id = seed.player.id.clone();
    let now = Utc::now();
    let mut actors: BTreeMap<_, _> = seed.actors.into_iter().map(|x| (x.id.clone(), x)).collect();
    if actors.insert(player_id.clone(), seed.player).is_some() {
        return Err(anyhow!("player id duplicates an NPC"));
    }
    let evidence_receipt_ids = receipt_ids(receipts);
    let valid_evidence_receipt_ids = evidence_receipt_ids
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    let affected_sources: Vec<String> = receipts
        .iter()
        .flat_map(|r| r.witnesses.iter().map(|w| w.source_id.clone()))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    let canon_candidates = seed
        .gaps
        .iter()
        .enumerate()
        .map(|(index, gap)| {
            let candidate = crate::domain::CanonCandidate {
                schema: "ghostlight.canon_candidate.v1".into(),
                id: format!("canon-candidate:{}:{}", id, index + 1),
                originating_campaign_id: id,
                gap: gap.clone(),
                evidence_receipt_ids: evidence_receipt_ids.clone(),
                conflicts: vec![],
                proposed_wording: format!("Clarify the documented answer to: {gap}"),
                affected_vault_sources: affected_sources.clone(),
                status: "review".into(),
            };
            (candidate.id.clone(), candidate)
        })
        .collect();
    let mut campaign = Campaign {
        schema: "ghostlight.campaign.v1".into(),
        id,
        name: seed.title,
        revision: 0,
        branch_origin: BranchOrigin {
            canon_cutoff: seed.canon_cutoff,
            evidence_receipt_ids,
        },
        world_time: seed.world_time,
        tick_hours: seed.tick_hours,
        player_actor_id: player_id,
        locations: seed
            .locations
            .into_iter()
            .map(|x| (x.id.clone(), x))
            .collect(),
        actors,
        institutions: seed
            .institutions
            .into_iter()
            .map(|x| (x.id.clone(), x))
            .collect(),
        clocks: seed.clocks.into_iter().map(|x| (x.id.clone(), x)).collect(),
        facts: seed
            .facts
            .into_iter()
            .map(|mut x| {
                let supplied_reference_count = x.evidence_receipt_ids.len();
                x.evidence_receipt_ids
                    .retain(|id| valid_evidence_receipt_ids.contains(id));
                if x.scope == FactScope::CanonBaseline
                    && (supplied_reference_count == 0
                        || x.evidence_receipt_ids.len() != supplied_reference_count)
                {
                    x.scope = FactScope::ProvisionalLocal;
                };
                (x.id.clone(), x)
            })
            .collect(),
        transcript: vec![crate::domain::NarrativeTurn {
            revision: 0,
            at: now,
            speaker: "world".into(),
            text: seed.opening_narration,
        }],
        last_player_activity: now,
        pending_ticks: 0,
        away_ticks_processed: 0,
        events: vec![],
        news: vec![],
        canon_candidates,
        gestalts: seed
            .gestalts
            .into_iter()
            .map(|x| (x.id.clone(), x))
            .collect(),
        gestalt_members: seed
            .gestalt_members
            .into_iter()
            .map(|x| (x.id.clone(), x))
            .collect(),
        pending_world_proposals: vec![],
        agency_profiles: BTreeMap::new(),
        agency_relations: BTreeMap::new(),
        gestalt_lineages: BTreeMap::new(),
        resolution_policy: Default::default(),
        resolution_pins: BTreeMap::new(),
        resolution_cover: None,
        strategic_tick_count: 0,
    };
    crate::resolution::ensure_agency_profiles(&mut campaign);
    Ok(campaign)
}

fn compiled_seed_structure(seed: &CompiledSeed) -> serde_json::Value {
    serde_json::json!({
        "tick_hours": seed.tick_hours,
        "player": {"id": seed.player.id, "location_id": seed.player.location_id},
        "locations": seed.locations.iter().map(|location| serde_json::json!({
            "id": location.id,
            "container_id": location.container_id,
            "routes": location.routes,
        })).collect::<Vec<_>>(),
        "actors": seed.actors.iter().map(|actor| serde_json::json!({
            "id": actor.id,
            "location_id": actor.location_id,
        })).collect::<Vec<_>>(),
        "institution_ids": seed.institutions.iter().map(|institution| institution.id.as_str()).collect::<Vec<_>>(),
        "gestalts": seed.gestalts.iter().map(|gestalt| serde_json::json!({
            "id": gestalt.id,
            "home_location_id": gestalt.home_location_id,
        })).collect::<Vec<_>>(),
        "gestalt_members": seed.gestalt_members.iter().map(|member| serde_json::json!({
            "id": member.id,
            "gestalt_id": member.gestalt_id,
            "materialized_actor_id": member.materialized_actor_id,
        })).collect::<Vec<_>>(),
        "clocks": seed.clocks.iter().map(|clock| serde_json::json!({
            "id": clock.id,
            "progress": clock.progress,
            "threshold": clock.threshold,
        })).collect::<Vec<_>>(),
        "fact_ids": seed.facts.iter().map(|fact| fact.id.as_str()).collect::<Vec<_>>(),
    })
}

fn require_unique_ids<'a>(label: &str, ids: impl IntoIterator<Item = &'a str>) -> Result<()> {
    let mut seen = BTreeSet::new();
    let mut duplicates = BTreeSet::new();
    for id in ids {
        if id.trim().is_empty() || !seen.insert(id.to_owned()) {
            duplicates.insert(id.to_owned());
        }
    }
    if duplicates.is_empty() {
        Ok(())
    } else {
        Err(anyhow!(
            "{label} IDs must be non-empty and unique; rejected IDs={duplicates:?}"
        ))
    }
}

fn agency_subject_briefs(
    campaign: &Campaign,
    excluded_subject_ids: &BTreeSet<String>,
) -> Vec<AgencySubjectBrief> {
    let mut briefs = Vec::new();
    for actor in campaign.actors.values().filter(|actor| {
        actor.id != campaign.player_actor_id && !excluded_subject_ids.contains(&actor.id)
    }) {
        briefs.push(AgencySubjectBrief {
            subject_id: actor.id.clone(),
            subject_kind: AgencySubjectKind::Actor,
            name: actor.name.clone(),
            location_ids: BTreeSet::from([actor.location_id.clone()]),
            capabilities_or_resources: actor
                .capabilities
                .iter()
                .chain(actor.equipment.iter())
                .cloned()
                .collect(),
            knowledge_or_posture: actor.knowledge.iter().cloned().collect(),
            goals: actor.goals.clone(),
            pressures_or_obligations: actor
                .obligations
                .iter()
                .chain(actor.conditions.iter())
                .cloned()
                .collect(),
        });
    }
    for institution in campaign
        .institutions
        .values()
        .filter(|institution| !excluded_subject_ids.contains(&institution.id))
    {
        briefs.push(AgencySubjectBrief {
            subject_id: institution.id.clone(),
            subject_kind: AgencySubjectKind::Institution,
            name: institution.name.clone(),
            location_ids: BTreeSet::new(),
            capabilities_or_resources: institution.resources.clone(),
            knowledge_or_posture: vec![institution.posture.clone()],
            goals: institution.goals.clone(),
            pressures_or_obligations: Vec::new(),
        });
    }
    for gestalt in campaign
        .gestalts
        .values()
        .filter(|gestalt| !excluded_subject_ids.contains(&gestalt.id))
    {
        briefs.push(AgencySubjectBrief {
            subject_id: gestalt.id.clone(),
            subject_kind: AgencySubjectKind::Gestalt,
            name: gestalt.name.clone(),
            location_ids: BTreeSet::from([gestalt.home_location_id.clone()]),
            capabilities_or_resources: gestalt
                .shared_capabilities
                .iter()
                .chain(gestalt.resources.iter())
                .cloned()
                .collect(),
            knowledge_or_posture: gestalt.shared_knowledge.iter().cloned().collect(),
            goals: gestalt.goals.clone(),
            pressures_or_obligations: gestalt.pressures.clone(),
        });
    }
    briefs.sort_by(|left, right| left.subject_id.cmp(&right.subject_id));
    briefs
}

fn apply_compiled_agency_skeleton(
    campaign: &mut Campaign,
    expected: &BTreeSet<String>,
    profiles: Vec<CompiledAgencyProfile>,
    relations: Vec<CompiledAgencyRelation>,
) -> Result<()> {
    if expected.is_empty() && profiles.is_empty() && relations.is_empty() {
        return Ok(());
    }
    let supplied: BTreeSet<_> = profiles
        .iter()
        .map(|profile| profile.subject_id.clone())
        .collect();
    let axes = BTreeSet::from([
        AgencyAxis::Geography,
        AgencyAxis::Ideology,
        AgencyAxis::Authority,
        AgencyAxis::EconomyRole,
        AgencyAxis::SpeciesBody,
        AgencyAxis::Information,
    ]);
    if &supplied != expected || supplied.len() != profiles.len() {
        let missing = expected.difference(&supplied).cloned().collect::<Vec<_>>();
        let unexpected = supplied.difference(expected).cloned().collect::<Vec<_>>();
        let duplicate_count = profiles.len().saturating_sub(supplied.len());
        return Err(anyhow!(
            "global agency skeleton coverage mismatch: missing={missing:?}; unexpected={unexpected:?}; duplicate_profile_count={duplicate_count}; expected_subject_ids={:?}",
            expected
        ));
    }
    for input in profiles {
        let authority_known = input
            .collective_authority_id
            .as_ref()
            .is_none_or(|id| expected.contains(id));
        let profile = campaign
            .agency_profiles
            .get_mut(&input.subject_id)
            .ok_or_else(|| anyhow!("agency profile references an unknown subject"))?;
        let input_axes: BTreeSet<_> = input.facets.keys().cloned().collect();
        let unknown_locations = input
            .location_ids
            .iter()
            .filter(|id| !campaign.locations.contains_key(*id))
            .cloned()
            .collect::<Vec<_>>();
        if profile.subject_kind != input.subject_kind
            || input_axes != axes
            || input.location_ids != profile.location_ids
            || !unknown_locations.is_empty()
            || !authority_known
        {
            let missing_axes = axes.difference(&input_axes).cloned().collect::<Vec<_>>();
            let unexpected_axes = input_axes.difference(&axes).cloned().collect::<Vec<_>>();
            return Err(anyhow!(
                "agency profile {} malformed: expected_kind={:?}; supplied_kind={:?}; expected_location_ids={:?}; supplied_location_ids={:?}; missing_axes={missing_axes:?}; unexpected_axes={unexpected_axes:?}; unknown_locations={unknown_locations:?}; unknown_collective_authority={:?}",
                input.subject_id,
                profile.subject_kind,
                input.subject_kind,
                profile.location_ids,
                input.location_ids,
                input.collective_authority_id.filter(|_| !authority_known)
            ));
        }
        profile.collective_authority_id = input.collective_authority_id;
        profile.facets = input.facets;
        profile.location_ids = input.location_ids;
        profile.information_channels = input.information_channels;
        profile.evidence_receipt_ids = campaign.branch_origin.evidence_receipt_ids.clone();
    }
    let mut relation_ids = BTreeSet::new();
    for input in relations {
        let duplicate_id = !relation_ids.insert(input.id.clone());
        let empty_id = input.id.trim().is_empty();
        let self_edge = input.from_subject_id == input.to_subject_id;
        let unknown_from = !expected.contains(&input.from_subject_id);
        let unknown_to = !expected.contains(&input.to_subject_id);
        let invalid_strength = input.strength == 0 || input.strength > 100;
        if duplicate_id || empty_id || self_edge || unknown_from || unknown_to || invalid_strength {
            return Err(anyhow!(
                "agency relation {:?} malformed: duplicate_id={duplicate_id}; empty_id={empty_id}; self_edge={self_edge}; unknown_from_subject={unknown_from} ({:?}); unknown_to_subject={unknown_to} ({:?}); invalid_strength={invalid_strength} ({}) ; supplied_subject_ids={:?}",
                input.id,
                input.from_subject_id,
                input.to_subject_id,
                input.strength,
                expected
            ));
        }
        campaign.agency_relations.insert(
            input.id.clone(),
            AgencyRelation {
                schema: "ghostlight.agency_relation.v1".into(),
                id: input.id,
                from_subject_id: input.from_subject_id,
                to_subject_id: input.to_subject_id,
                kind: input.kind,
                strength: input.strength,
                active: true,
                evidence_receipt_ids: campaign.branch_origin.evidence_receipt_ids.clone(),
            },
        );
    }
    Ok(())
}

pub fn validate_campaign_seed(c: &Campaign) -> Result<()> {
    if c.tick_hours == 0 {
        return Err(anyhow!("strategic tick duration must be positive"));
    }
    if !c.actors.contains_key(&c.player_actor_id) {
        return Err(anyhow!("player actor is missing"));
    }
    crate::resolution::validate_policy(&c.resolution_policy)?;
    crate::resolution::validate_pins(c, &c.resolution_pins)?;
    let expected_profiles: BTreeSet<_> = c
        .actors
        .keys()
        .filter(|id| *id != &c.player_actor_id)
        .chain(c.institutions.keys())
        .chain(c.gestalts.keys())
        .cloned()
        .collect();
    let actual_profiles: BTreeSet<_> = c
        .agency_profiles
        .values()
        .filter(|profile| profile.active_leaf && profile.simulation_eligible)
        .map(|profile| profile.subject_id.clone())
        .collect();
    if expected_profiles != actual_profiles {
        return Err(anyhow!(
            "campaign agency skeleton has incomplete subject coverage"
        ));
    }
    let relationship_targets = c
        .actors
        .keys()
        .chain(c.institutions.keys())
        .cloned()
        .collect::<BTreeSet<_>>();
    for actor in c.actors.values() {
        if !c.locations.contains_key(&actor.location_id) {
            return Err(anyhow!(
                "actor {} occupies unknown location {}",
                actor.id,
                actor.location_id
            ));
        }
        let invalid_relationships = actor
            .relationships
            .iter()
            .filter(|(target_id, description)| {
                !relationship_targets.contains(*target_id) || description.trim().is_empty()
            })
            .map(|(target_id, _)| format!("{}->{target_id}", actor.id))
            .collect::<Vec<_>>();
        if !invalid_relationships.is_empty() {
            return Err(anyhow!(
                "actor relationships must use exact declared actor or institution IDs with non-empty descriptions; rejected relationships={invalid_relationships:?}; valid target IDs={relationship_targets:?}"
            ));
        }
    }
    for gestalt in c.gestalts.values() {
        if !c.locations.contains_key(&gestalt.home_location_id) {
            return Err(anyhow!(
                "gestalt {} occupies unknown home location {}",
                gestalt.id,
                gestalt.home_location_id
            ));
        }
    }
    for member in c.gestalt_members.values() {
        if !c.gestalts.contains_key(&member.gestalt_id) {
            return Err(anyhow!(
                "gestalt member {} references unknown gestalt {}",
                member.id,
                member.gestalt_id
            ));
        }
        if member.materialized_actor_id.is_some() {
            return Err(anyhow!(
                "compiled gestalt member {} must begin dematerialized",
                member.id
            ));
        }
    }
    for location in c.locations.values() {
        if let Some(parent) = &location.container_id
            && (parent == &location.id || !c.locations.contains_key(parent))
        {
            return Err(anyhow!(
                "location {} has invalid container_id {:?}; it must name a different supplied location or be null",
                location.id,
                location.container_id
            ));
        }
        for (route_id, route) in &location.routes {
            if route.travel_minutes == 0 {
                return Err(anyhow!(
                    "location {} route {} to {} has zero travel_minutes",
                    location.id,
                    route_id,
                    route.destination_id
                ));
            }
            if !c.locations.contains_key(&route.destination_id) {
                return Err(anyhow!(
                    "location {} route {} references missing destination_id {}; supplied location IDs={:?}",
                    location.id,
                    route_id,
                    route.destination_id,
                    c.locations.keys().collect::<Vec<_>>()
                ));
            }
        }
    }
    let mut fact_statements = BTreeSet::new();
    for fact in c.facts.values() {
        if fact.statement.trim().is_empty() || !fact_statements.insert(fact.statement.clone()) {
            return Err(anyhow!(
                "world facts must have non-empty unique statements; rejected fact {}",
                fact.id
            ));
        }
        let invalid_locations = fact
            .discoverable_at_location_ids
            .iter()
            .filter(|id| !c.locations.contains_key(*id))
            .cloned()
            .collect::<Vec<_>>();
        if !invalid_locations.is_empty() {
            return Err(anyhow!(
                "fact {} is discoverable at unknown locations {:?}; valid location IDs={:?}",
                fact.id,
                invalid_locations,
                c.locations.keys().collect::<Vec<_>>()
            ));
        }
    }
    for clock in c.clocks.values() {
        if clock.threshold == 0 || clock.progress > clock.threshold {
            return Err(anyhow!("clock {} is invalid", clock.id));
        }
    }
    Ok(())
}

fn validate_opening_playability(campaign: &Campaign) -> Result<()> {
    let player_location = &campaign.actors[&campaign.player_actor_id].location_id;
    if campaign.facts.values().any(|fact| {
        fact.scope != FactScope::CanonBaseline
            && fact.discoverable_at_location_ids.contains(player_location)
    }) {
        Ok(())
    } else {
        Err(anyhow!(
            "the opening location must contain at least one branch_local or provisional_local discoverable fact; player location={player_location}"
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_text_admission_rejects_empty_oversized_and_binary_control_input() {
        assert!(validate_user_text("field", "", 8).is_err());
        assert!(validate_user_text("field", "   ", 8).is_err());
        assert!(validate_user_text("field", "123456789", 8).is_err());
        assert!(validate_user_text("field", "hello\0world", 20).is_err());
        assert!(validate_user_text("field", "hello\nworld", 20).is_ok());
    }

    #[test]
    fn fission_text_is_bounded_before_retrieval_or_inference() {
        let mut request = GestaltFissionRequest {
            parent_gestalt_id: "population".into(),
            partition_axis: AgencyAxis::Geography,
            requested_partition_values: vec!["harbor".into(), "inland".into()],
            reason: "The population is dispersing along established routes.".into(),
        };
        assert!(validate_fission_request(&request).is_ok());

        request.requested_partition_values = (0..17).map(|index| format!("cut-{index}")).collect();
        assert!(validate_fission_request(&request).is_err());
        request.requested_partition_values = vec!["x".repeat(161)];
        assert!(validate_fission_request(&request).is_err());
        request.requested_partition_values = vec!["Harbor".into(), "harbor".into()];
        assert!(validate_fission_request(&request).is_err());
        request.requested_partition_values = vec!["other/unknown".into()];
        assert!(validate_fission_request(&request).is_err());
        request.requested_partition_values = vec!["harbor".into()];
        request.reason = "x".repeat(501);
        assert!(validate_fission_request(&request).is_err());
    }
    use crate::{domain::SourceWitness, model::ModelPort, vault::FixtureVault};
    use async_trait::async_trait;
    use sha2::Digest;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

    struct CompilerModel {
        invalid_route: bool,
    }

    struct OversizedQueryModel;

    struct CorrectionAwareCompilerModel {
        world_calls: AtomicUsize,
        saw_previous_structure: AtomicBool,
    }

    struct CorrectionAwareOpeningModel {
        opening_calls: AtomicUsize,
        saw_exact_correction: AtomicBool,
    }

    struct CorrectionAwareRoleModel {
        role_calls: AtomicUsize,
        saw_exact_correction: AtomicBool,
    }

    #[async_trait]
    impl ModelPort for CorrectionAwareOpeningModel {
        async fn run(&self, request: &ModelStageRequest) -> Result<String> {
            let output = CompilerModel {
                invalid_route: false,
            }
            .run(request)
            .await?;
            if request.stage != "world_openings" {
                return Ok(output);
            }
            let call = self.opening_calls.fetch_add(1, Ordering::SeqCst);
            if call == 0 {
                let mut candidate: serde_json::Value = serde_json::from_str(&output)?;
                candidate["openings"][1]["era"] = serde_json::json!("early");
                return Ok(candidate.to_string());
            }
            self.saw_exact_correction.store(
                request
                    .lived_stream
                    .contains("LOCAL VALIDATOR REJECTED THE PREVIOUS OPENINGS")
                    && request
                        .lived_stream
                        .contains("era=\"early\" repeated 2 times")
                    && request.lived_stream.contains("\"era\":\"early\""),
                Ordering::SeqCst,
            );
            Ok(output)
        }

        fn provider(&self) -> &'static str {
            "correction-aware-opening-fixture"
        }
    }

    #[async_trait]
    impl ModelPort for CorrectionAwareRoleModel {
        async fn run(&self, request: &ModelStageRequest) -> Result<String> {
            let output = CompilerModel {
                invalid_route: false,
            }
            .run(request)
            .await?;
            if request.stage != "world_roles" {
                return Ok(output);
            }
            let call = self.role_calls.fetch_add(1, Ordering::SeqCst);
            if call == 0 {
                let mut candidate: serde_json::Value = serde_json::from_str(&output)?;
                candidate["roles"][1]["name"] = serde_json::json!("Courier");
                return Ok(candidate.to_string());
            }
            self.saw_exact_correction.store(
                request
                    .lived_stream
                    .contains("LOCAL VALIDATOR REJECTED THE PREVIOUS ROLES")
                    && request
                        .lived_stream
                        .contains("name=\"courier\" repeated 2 times")
                    && request.lived_stream.contains("\"name\":\"Courier\""),
                Ordering::SeqCst,
            );
            Ok(output)
        }

        fn provider(&self) -> &'static str {
            "correction-aware-role-fixture"
        }
    }

    #[async_trait]
    impl ModelPort for CorrectionAwareCompilerModel {
        async fn run(&self, request: &ModelStageRequest) -> Result<String> {
            let output = CompilerModel {
                invalid_route: false,
            }
            .run(request)
            .await?;
            if request.stage != "world_compile" {
                return Ok(output);
            }
            let call = self.world_calls.fetch_add(1, Ordering::SeqCst);
            if call == 0 {
                let mut candidate: serde_json::Value = serde_json::from_str(&output)?;
                candidate["locations"][0]["routes"]["out"]["destination_id"] =
                    serde_json::Value::String("missing".into());
                return Ok(candidate.to_string());
            }
            self.saw_previous_structure.store(
                request
                    .lived_stream
                    .contains("PREVIOUS_CANDIDATE_STRUCTURE")
                    && request
                        .lived_stream
                        .contains("\"destination_id\":\"missing\"")
                    && request.lived_stream.contains("\"id\":\"yard\""),
                Ordering::SeqCst,
            );
            Ok(output)
        }

        fn provider(&self) -> &'static str {
            "correction-aware-compiler-fixture"
        }
    }

    #[async_trait]
    impl ModelPort for OversizedQueryModel {
        async fn run(&self, _: &ModelStageRequest) -> Result<String> {
            Ok(serde_json::json!({"queries":["x".repeat(241)]}).to_string())
        }
        fn provider(&self) -> &'static str {
            "oversized-query-fixture"
        }
    }
    #[async_trait]
    impl ModelPort for CompilerModel {
        async fn run(&self, request: &ModelStageRequest) -> Result<String> {
            Ok(match request.stage.as_str() {
                "opening_retrieval_plan" => serde_json::json!({
                    "early_frame_query":"fixture earliest period ring strike",
                    "transition_frame_query":"fixture transition period moon siege",
                    "late_frame_query":"fixture latest period station election"
                }).to_string(),
                stage if stage.ends_with("_retrieval_plan") => {
                    let count = if stage == "role_retrieval_plan"
                        || stage == "destination_retrieval_plan"
                    {
                        2
                    } else {
                        3
                    };
                    serde_json::json!({"queries":(1..=count).map(|index|format!("fixture grounded query {index}")).collect::<Vec<_>>()}).to_string()
                }
                "evidence_relevance" => serde_json::json!({
                    "coverage":[{
                        "source_id":"AetheriaLore:test.md",
                        "lane":"direct_seed",
                        "rationale":"The fixture source directly grounds the requested place."
                    }]
                }).to_string(),
                "global_agency_compile" => serde_json::json!({
                    "institutions":[{
                        "name":"Fixture Council",
                        "mandate":"The Fixture Council maintains the shared route."
                    }],
                    "gaps":[]
                }).to_string(),
                "world_openings" => serde_json::json!({"openings":[
                    {"id":"a","title":"Ash","era":"early","place":"ring","pressure":"strike","player_hook":"work","evidence_receipt_ids":[]},
                    {"id":"b","title":"Glass","era":"middle","place":"moon","pressure":"siege","player_hook":"survive","evidence_receipt_ids":[]},
                    {"id":"c","title":"Rain","era":"late","place":"station","pressure":"election","player_hook":"choose","evidence_receipt_ids":[]}
                ]}).to_string(),
                "world_roles" => serde_json::json!({"roles":[
                    {"id":"courier","name":"Courier","premise":"Carry a disputed manifest through the blockade.","capabilities":["route knowledge"],"obligations":["deliver the manifest"],"evidence_receipt_ids":[]},
                    {"id":"organizer","name":"Dock Organizer","premise":"Keep the strike coalition together under pressure.","capabilities":["labor trust"],"obligations":["protect the picket"],"evidence_receipt_ids":[]},
                    {"id":"auditor","name":"Contract Auditor","premise":"Trace the institution hiding the missing supplies.","capabilities":["ledger access"],"obligations":["report material fraud"],"evidence_receipt_ids":[]}
                ]}).to_string(),
                "world_compile" => {
                    let destination = if self.invalid_route { "missing" } else { "yard" };
                    serde_json::json!({
                        "title":"Grounded test", "canon_cutoff":"fixture", "world_time":"2026-01-01T00:00:00Z", "tick_hours":6,
                        "player":{"id":"player","name":"Tester","location_id":"yard","capabilities":[],"knowledge":[],"equipment":[],"conditions":[],"obligations":[],"relationships":{},"goals":["learn"]},
                        "locations":[{"id":"yard","name":"Yard","container_id":null,"routes":{"out":{"destination_id":destination,"distance":"near","travel_minutes":5}},"persistent_features":["same yard"]}],
                        "actors":[],
                        "gestalts":[{"schema":"ghostlight.gestalt_persona_state.v1","id":"yard-workers","name":"Yard workers","version":0,"home_location_id":"yard","shared_capabilities":["maintain machinery"],"shared_knowledge":["yard routines"],"resources":["tool shed"],"goals":["finish the shift"],"pressures":["the gate is failing"]}],
                        "gestalt_members":[{"schema":"ghostlight.gestalt_member_delta.v1","id":"john","gestalt_id":"yard-workers","version":0,"name":"John the smith","capability_additions":["forge hinges"],"capability_removals":[],"knowledge_additions":[],"knowledge_removals":[],"equipment":["hammer"],"conditions":[],"obligations":[],"relationships":{},"goals":[],"memories":[],"last_location_id":"yard","materialized_actor_id":null}],
                        "institutions":[],"clocks":[{"id":"shift","label":"Shift ends","progress":0,"threshold":4,"consequence":"night"}],
                        "facts":[
                            {"id":"f","statement":"A witnessed fact","scope":"canon_baseline","evidence_receipt_ids":["fixture"]},
                            {"id":"local","statement":"The outer gate indicator is dark.","scope":"branch_local","evidence_receipt_ids":[],"discoverable_at_location_ids":["yard"]}
                        ],
                        "gaps":["Who owns the outer gate?"],"branch_assumptions":[],"opening_narration":"The yard persists."
                    }).to_string()
                }
                "agency_compile" => serde_json::json!({
                        "agency_profiles":[{"subject_id":"yard-workers","subject_kind":"gestalt","collective_authority_id":"yard-workers","facets":{"geography":["yard"],"ideology":["mutual aid"],"authority":["yard-workers"],"economy_role":["maintenance"],"species_body":["human"],"information":["yard routines"]},"location_ids":["yard"],"information_channels":["yard routines"]}],
                        "agency_relations":[]
                    }).to_string(),
                _ => return Err(anyhow!("unexpected stage")),
            })
        }
        fn provider(&self) -> &'static str {
            "compiler-fixture"
        }
    }

    fn vault() -> Arc<FixtureVault> {
        Arc::new(FixtureVault {
            witnesses: vec![SourceWitness {
                source_id: "AetheriaLore:test.md".into(),
                exact_locator: "test.md:1-2".into(),
                content_hash: "sha256:test".into(),
                excerpt:
                    "A stable witnessed place. The Fixture Council maintains the shared route."
                        .into(),
                authority_lane: "AetheriaLore".into(),
                temporal_scope: "fixture".into(),
            }],
        })
    }

    struct ExactWitnessVault;
    #[async_trait]
    impl VaultProvider for ExactWitnessVault {
        async fn search(&self, query: &VaultQuery) -> Result<VaultEvidenceReceipt> {
            Ok(VaultEvidenceReceipt {
                schema: "ghostlight.vault_evidence_receipt.v1".into(),
                id: "search-receipt".into(),
                provider: "fixture".into(),
                query_hash: "sha256:query".into(),
                witnesses: vec![SourceWitness {
                    source_id: "AetheriaLore:route.md".into(),
                    exact_locator: "route.md:2-2".into(),
                    content_hash: "sha256:excerpt-only".into(),
                    excerpt: "The route takes six hours.".into(),
                    authority_lane: query.authority_lanes.join(","),
                    temporal_scope: query.temporal_scope.clone(),
                }],
                retrieved_at: Utc::now(),
            })
        }

        async fn surrounding_context(&self, _: &str, _: u32) -> Result<SourceWitness> {
            unreachable!()
        }

        async fn exact_document(&self, source_id: &str) -> Result<SourceWitness> {
            let content =
                "The forge opens at dawn.\nThe route takes six hours.\nThe gate closes at dusk.";
            Ok(SourceWitness {
                source_id: source_id.into(),
                exact_locator: "route.md".into(),
                content_hash: format!("sha256:{:x}", sha2::Sha256::digest(content.as_bytes())),
                excerpt: content.into(),
                authority_lane: "AetheriaLore".into(),
                temporal_scope: "fixture".into(),
            })
        }

        fn provider_id(&self) -> &'static str {
            "fixture"
        }
    }

    #[test]
    fn campaign_relationships_bind_to_canonical_subject_ids() {
        let mut campaign = crate::resolution::tests::campaign(2, 1);
        campaign
            .actors
            .get_mut("player")
            .unwrap()
            .relationships
            .insert("faction-0000".into(), "cautious contact".into());
        validate_campaign_seed(&campaign).unwrap();

        campaign
            .actors
            .get_mut("player")
            .unwrap()
            .relationships
            .insert("Faction Zero".into(), "display name, not identity".into());
        let error = validate_campaign_seed(&campaign).unwrap_err().to_string();
        assert!(error.contains("player->Faction Zero"));
        assert!(error.contains("faction-0000"));
    }

    #[tokio::test]
    async fn opening_stage_requires_three_distinct_axes() {
        let compiler = WorldCompiler::new(
            vault(),
            Arc::new(CompilerModel {
                invalid_route: false,
            }),
            "flash",
            "pro",
        );
        let output = compiler
            .suggest_openings(OpeningRequest {
                setting: "Aetheria".into(),
                constraints: vec![],
            })
            .await
            .unwrap();
        assert_eq!(output.openings.len(), 3);
        assert_eq!(output.evidence_receipts.len(), 3);
        assert_eq!(output.model_receipts.len(), 1);
    }

    #[tokio::test]
    async fn opening_stage_corrects_a_semantically_duplicate_axis_once() {
        let model = Arc::new(CorrectionAwareOpeningModel {
            opening_calls: AtomicUsize::new(0),
            saw_exact_correction: AtomicBool::new(false),
        });
        let compiler = WorldCompiler::new(vault(), model.clone(), "flash", "pro");

        let output = compiler
            .suggest_openings(OpeningRequest {
                setting: "Aetheria".into(),
                constraints: vec![],
            })
            .await
            .unwrap();

        assert_eq!(output.openings.len(), 3);
        assert_eq!(output.model_receipts.len(), 2);
        assert!(output.model_receipts[0].local_validation_error.is_some());
        assert!(output.model_receipts[1].local_validation_error.is_none());
        assert!(model.saw_exact_correction.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn role_stage_corrects_a_semantically_duplicate_axis_once() {
        let model = Arc::new(CorrectionAwareRoleModel {
            role_calls: AtomicUsize::new(0),
            saw_exact_correction: AtomicBool::new(false),
        });
        let compiler = WorldCompiler::new(vault(), model.clone(), "flash", "pro");

        let output = compiler
            .suggest_roles(&OpeningSuggestion {
                id: "blockade".into(),
                title: "The Blockade".into(),
                era: "late".into(),
                place: "ring".into(),
                pressure: "blockade".into(),
                player_hook: "choose a route".into(),
                evidence_receipt_ids: vec![],
            })
            .await
            .unwrap();

        assert_eq!(output.roles.len(), 3);
        assert_eq!(output.model_receipts.len(), 2);
        assert!(output.model_receipts[0].local_validation_error.is_some());
        assert!(output.model_receipts[1].local_validation_error.is_none());
        assert!(model.saw_exact_correction.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn retrieval_planner_refuses_provider_oversized_queries() {
        let compiler = WorldCompiler::new(vault(), Arc::new(OversizedQueryModel), "flash", "pro");
        let error = compiler
            .plan_queries("custom_retrieval_plan", "test", "subject", 1)
            .await
            .unwrap_err();
        assert!(error.to_string().contains("1 to 240"));
    }

    #[tokio::test]
    async fn retrieval_receipts_bind_excerpts_to_exact_archive_hashes() {
        let compiler = WorldCompiler::new(
            Arc::new(ExactWitnessVault),
            Arc::new(CompilerModel {
                invalid_route: false,
            }),
            "flash",
            "pro",
        );
        let receipts = compiler
            .retrieve_all(&["route".into()], "fixture", 3)
            .await
            .unwrap();
        let witness = &receipts[0].witnesses[0];
        assert_ne!(witness.content_hash, "sha256:excerpt-only");
        assert_eq!(witness.exact_locator, "route.md:2-2");
    }

    #[tokio::test]
    async fn compile_returns_approval_preview_without_committing() {
        let compiler = WorldCompiler::new(
            vault(),
            Arc::new(CompilerModel {
                invalid_route: false,
            }),
            "flash",
            "pro",
        );
        let (preview, receipts) = compiler
            .compile_custom(CustomStart {
                campaign_name: "Test".into(),
                who: "worker".into(),
                where_: "yard".into(),
                when: "fixture".into(),
                goal: "learn".into(),
            })
            .await
            .unwrap();
        assert_eq!(
            receipts
                .iter()
                .map(|receipt| receipt.stage.as_str())
                .collect::<Vec<_>>(),
            vec![
                "custom_retrieval_plan",
                "evidence_relevance",
                "global_agency_compile",
                "world_compile",
                "agency_compile"
            ]
        );
        assert!(preview.requires_approval);
        assert_eq!(preview.evidence_coverage.len(), 1);
        assert_eq!(
            preview.evidence_coverage[0].lane,
            EvidenceUseLane::DirectSeed
        );
        assert_eq!(preview.campaign.revision, 0);
        assert_eq!(preview.campaign.locations.len(), 1);
        assert_eq!(preview.campaign.canon_candidates.len(), 1);
        assert_eq!(preview.campaign.gestalts.len(), 1);
        assert_eq!(preview.campaign.institutions.len(), 1);
        assert_eq!(preview.campaign.agency_profiles.len(), 3);
        let remote = preview
            .campaign
            .agency_profiles
            .values()
            .find(|profile| profile.subject_id.starts_with("remote-institution:"))
            .unwrap();
        assert_eq!(remote.facets.len(), 6);
        assert!(!remote.evidence_receipt_ids.is_empty());
        assert_eq!(
            remote.facets[&AgencyAxis::Authority],
            BTreeSet::from([remote.subject_id.clone()])
        );
        assert!(remote.information_channels.is_empty());
        assert_eq!(
            preview.campaign.facts["f"].scope,
            FactScope::ProvisionalLocal
        );
        assert!(preview.campaign.facts["f"].evidence_receipt_ids.is_empty());
        assert_eq!(
            preview.campaign.agency_profiles["yard-workers"].facets[&AgencyAxis::Ideology],
            BTreeSet::from(["mutual aid".into()])
        );
        assert_eq!(
            preview.campaign.gestalt_members["john"].name,
            "John the smith"
        );
        assert!(
            preview.campaign.gestalt_members["john"]
                .materialized_actor_id
                .is_none()
        );
    }

    #[tokio::test]
    async fn selected_role_capabilities_and_obligations_survive_world_compilation() {
        let compiler = WorldCompiler::new(
            vault(),
            Arc::new(CompilerModel {
                invalid_route: false,
            }),
            "flash",
            "pro",
        );
        let (preview, _) = compiler
            .compile_selected(SelectedStart {
                campaign_name: "Selected role".into(),
                opening: OpeningSuggestion {
                    id: "opening".into(),
                    title: "Opening".into(),
                    era: "fixture".into(),
                    place: "yard".into(),
                    pressure: "A gate is closed".into(),
                    player_hook: "Learn why".into(),
                    evidence_receipt_ids: vec![],
                },
                role: RoleSuggestion {
                    id: "courier".into(),
                    name: "Courier".into(),
                    premise: "Carry a disputed manifest.".into(),
                    capabilities: vec!["route knowledge".into()],
                    obligations: vec!["deliver the manifest".into()],
                    evidence_receipt_ids: vec![],
                },
            })
            .await
            .unwrap();
        let player = &preview.campaign.actors[&preview.campaign.player_actor_id];
        assert!(player.capabilities.contains("route knowledge"));
        assert!(player.obligations.contains("deliver the manifest"));
        assert!(
            preview
                .branch_assumptions
                .iter()
                .any(|assumption| assumption.contains("Courier"))
        );
    }

    #[test]
    fn evidence_projection_carries_exact_receipt_ids_and_deduplicates_witnesses() {
        let witness = SourceWitness {
            source_id: "AetheriaLore:test.md".into(),
            exact_locator: "test.md:1-2".into(),
            content_hash: "sha256:test".into(),
            excerpt: "A stable witnessed place.".into(),
            authority_lane: "AetheriaLore".into(),
            temporal_scope: "fixture".into(),
        };
        let text = evidence_text(&[
            VaultEvidenceReceipt {
                schema: "ghostlight.vault_evidence_receipt.v1".into(),
                id: "vault:receipt-one".into(),
                provider: "fixture".into(),
                query_hash: "sha256:one".into(),
                witnesses: vec![witness.clone()],
                retrieved_at: Utc::now(),
            },
            VaultEvidenceReceipt {
                schema: "ghostlight.vault_evidence_receipt.v1".into(),
                id: "vault:receipt-two".into(),
                provider: "fixture".into(),
                query_hash: "sha256:two".into(),
                witnesses: vec![witness],
                retrieved_at: Utc::now(),
            },
        ]);
        assert_eq!(text.matches("A stable witnessed place.").count(), 1);
        assert!(text.contains("receipt_id=vault:receipt-one"));
        assert!(!text.contains("receipt_id=vault:receipt-two"));
    }

    #[test]
    fn world_seed_context_contains_only_direct_evidence() {
        let direct = VaultEvidenceReceipt {
            schema: "ghostlight.vault_evidence_receipt.v1".into(),
            id: "vault:direct".into(),
            provider: "fixture".into(),
            query_hash: "sha256:direct".into(),
            witnesses: vec![SourceWitness {
                source_id: "AetheriaLore:place.md".into(),
                exact_locator: "place.md:1".into(),
                content_hash: "sha256:place".into(),
                excerpt: "The requested station exists.".into(),
                authority_lane: "AetheriaLore".into(),
                temporal_scope: "fixture".into(),
            }],
            retrieved_at: Utc::now(),
        };
        let nearby_story = VaultEvidenceReceipt {
            schema: "ghostlight.vault_evidence_receipt.v1".into(),
            id: "vault:story".into(),
            provider: "fixture".into(),
            query_hash: "sha256:story".into(),
            witnesses: vec![SourceWitness {
                source_id: "AetheriaLore:unrelated-story.md".into(),
                exact_locator: "unrelated-story.md:1".into(),
                content_hash: "sha256:story".into(),
                excerpt: "An unrelated named cast has a crisis nearby.".into(),
                authority_lane: "AetheriaLore".into(),
                temporal_scope: "fixture".into(),
            }],
            retrieved_at: Utc::now(),
        };
        let receipts = vec![direct, nearby_story];
        let coverage = vec![
            EvidenceCoverage {
                source_id: "AetheriaLore:place.md".into(),
                lane: EvidenceUseLane::DirectSeed,
                rationale: "Directly grounds the requested station.".into(),
            },
            EvidenceCoverage {
                source_id: "AetheriaLore:unrelated-story.md".into(),
                lane: EvidenceUseLane::SettingBackground,
                rationale: "The incident offers setting color but is not current.".into(),
            },
        ];
        let text = direct_seed_evidence_text(&receipts, &coverage);
        assert!(text.contains("The requested station exists."));
        assert!(!text.contains("unrelated named cast"));
        assert_eq!(
            receipt_ids_for_coverage(&receipts, &coverage),
            vec!["vault:direct"]
        );
    }

    #[test]
    fn narrative_and_fixture_documents_cannot_seed_a_new_branch() {
        let make_receipt = |id: &str, lane: &str, excerpt: &str| VaultEvidenceReceipt {
            schema: "ghostlight.vault_evidence_receipt.v1".into(),
            id: format!("vault:{id}"),
            provider: "fixture".into(),
            query_hash: format!("sha256:{id}"),
            witnesses: vec![SourceWitness {
                source_id: format!("AetheriaLore:{id}"),
                exact_locator: id.into(),
                content_hash: format!("sha256:{id}"),
                excerpt: excerpt.into(),
                authority_lane: lane.into(),
                temporal_scope: "fixture".into(),
            }],
            retrieved_at: Utc::now(),
        };
        let receipts = vec![
            make_receipt(
                "mars.md",
                "aetheria.canon_worldbuilding",
                "Zhestokost holds fortified nodes on Mars.",
            ),
            make_receipt(
                "first-exodus.md",
                "aetheria.legacy_story",
                "Blackbox Aviary 3C contains Kesh and Dr. Maela Voss.",
            ),
            make_receipt(
                "corvid.branch.json",
                "aetheria.fixture_artifact",
                "The interactive fixture repeats Blackbox Aviary 3C.",
            ),
        ];
        let coverage = receipts
            .iter()
            .map(|receipt| EvidenceCoverage {
                source_id: receipt.witnesses[0].source_id.clone(),
                lane: EvidenceUseLane::DirectSeed,
                rationale: "classifier proposed direct use".into(),
            })
            .collect::<Vec<_>>();

        let text = direct_seed_evidence_text(&receipts, &coverage);
        assert!(text.contains("Zhestokost holds fortified nodes on Mars"));
        assert!(!text.contains("Blackbox Aviary 3C"));
        assert_eq!(
            receipt_ids_for_coverage(&receipts, &coverage),
            vec!["vault:mars.md"]
        );

        let global = canonical_worldbuilding_receipts(&receipts);
        assert_eq!(global.len(), 1);
        assert_eq!(global[0].id, "vault:mars.md");
    }

    #[test]
    fn agency_compile_schema_exposes_relation_strength_domain() {
        let schema = serde_json::to_value(schema_for!(CompiledAgencySkeleton)).unwrap();
        let serialized = serde_json::to_string(&schema).unwrap();
        assert!(serialized.contains("\"minimum\":1"));
        assert!(serialized.contains("\"maximum\":100"));
    }

    #[test]
    fn global_agency_claims_must_be_short_exact_source_witnesses() {
        let receipts = vec![VaultEvidenceReceipt {
            schema: "ghostlight.vault_evidence_receipt.v1".into(),
            id: "vault:power".into(),
            provider: "fixture".into(),
            query_hash: "sha256:power".into(),
            witnesses: vec![SourceWitness {
                source_id: "AetheriaLore:powers.md".into(),
                exact_locator: "powers.md:1".into(),
                content_hash: "sha256:power".into(),
                excerpt: "Pan-Solar Consortium coordinates interplanetary logistics.".into(),
                authority_lane: "AetheriaLore".into(),
                temporal_scope: "fixture".into(),
            }],
            retrieved_at: Utc::now(),
        }];
        let valid = CompiledGlobalAgencyCatalog {
            institutions: vec![CompiledRemoteInstitution {
                name: "Pan-Solar Consortium".into(),
                mandate: "Pan-Solar Consortium coordinates interplanetary logistics.".into(),
                evidence_receipt_ids: vec![],
            }],
            gaps: vec![],
        };
        let (valid, gaps) = ground_global_agency_catalog(valid, &receipts).unwrap();
        assert_eq!(valid.institutions.len(), 1);
        assert_eq!(
            valid.institutions[0].evidence_receipt_ids,
            vec!["vault:power"]
        );
        assert!(gaps.is_empty());

        let mut invented = valid;
        invented.institutions[0].mandate =
            "Pan-Solar Consortium secretly controls every government.".into();
        let (grounded, gaps) = ground_global_agency_catalog(invented, &receipts).unwrap();
        assert!(grounded.institutions.is_empty());
        assert_eq!(gaps.len(), 1);
        assert!(grounded.gaps[0].contains("1 remote agency candidates"));
    }

    #[test]
    fn global_agency_capacity_applies_after_grounding() {
        let excerpt = (0..33)
            .map(|index| format!("Institution {index} protects route {index}."))
            .collect::<Vec<_>>()
            .join("\n");
        let receipts = vec![VaultEvidenceReceipt {
            schema: "ghostlight.vault_evidence_receipt.v1".into(),
            id: "vault:many-powers".into(),
            provider: "fixture".into(),
            query_hash: "sha256:many-powers".into(),
            witnesses: vec![SourceWitness {
                source_id: "AetheriaLore:many-powers.md".into(),
                exact_locator: "many-powers.md:1-33".into(),
                content_hash: "sha256:many-powers".into(),
                excerpt,
                authority_lane: "aetheria.canon_worldbuilding".into(),
                temporal_scope: "fixture".into(),
            }],
            retrieved_at: Utc::now(),
        }];
        let catalog = CompiledGlobalAgencyCatalog {
            institutions: (0..33)
                .map(|index| CompiledRemoteInstitution {
                    name: format!("Institution {index}"),
                    mandate: format!("Institution {index} protects route {index}."),
                    evidence_receipt_ids: vec![],
                })
                .collect(),
            gaps: vec![],
        };

        let (grounded, private_gaps) = ground_global_agency_catalog(catalog, &receipts).unwrap();
        assert_eq!(grounded.institutions.len(), 32);
        assert_eq!(grounded.institutions[0].name, "Institution 0");
        assert_eq!(grounded.institutions[31].name, "Institution 31");
        assert!(grounded.gaps.iter().any(|gap| gap.contains("capped at 32")));
        assert!(
            private_gaps
                .iter()
                .any(|gap| gap.contains("exceeded the 32-institution"))
        );
    }

    #[test]
    fn global_agency_schema_allows_bounded_pre_grounding_candidates() {
        let schema = serde_json::to_value(schema_for!(CompiledGlobalAgencyCatalog)).unwrap();
        assert_eq!(schema["properties"]["institutions"]["maxItems"], 64);
    }

    #[test]
    fn global_evidence_never_demotes_direct_local_authority() {
        let global = vec![VaultEvidenceReceipt {
            schema: "ghostlight.vault_evidence_receipt.v1".into(),
            id: "vault:global".into(),
            provider: "fixture".into(),
            query_hash: "sha256:global".into(),
            witnesses: vec![SourceWitness {
                source_id: "AetheriaLore:shared.md".into(),
                exact_locator: "shared.md:1".into(),
                content_hash: "sha256:shared".into(),
                excerpt: "Shared source.".into(),
                authority_lane: "AetheriaLore".into(),
                temporal_scope: "fixture".into(),
            }],
            retrieved_at: Utc::now(),
        }];
        let merged = merge_global_evidence_coverage(
            vec![EvidenceCoverage {
                source_id: "AetheriaLore:shared.md".into(),
                lane: EvidenceUseLane::DirectSeed,
                rationale: "Directly establishes the requested place.".into(),
            }],
            &global,
        );
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].lane, EvidenceUseLane::DirectSeed);
    }

    #[tokio::test]
    async fn compiler_refuses_dream_route_to_unknown_location() {
        let compiler = WorldCompiler::new(
            vault(),
            Arc::new(CompilerModel {
                invalid_route: true,
            }),
            "flash",
            "pro",
        );
        let result = compiler
            .compile_custom(CustomStart {
                campaign_name: "Test".into(),
                who: "worker".into(),
                where_: "yard".into(),
                when: "fixture".into(),
                goal: "learn".into(),
            })
            .await;
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("references missing destination_id missing")
        );
    }

    #[tokio::test]
    async fn semantic_retry_receives_the_previous_candidates_structural_ids() {
        let model = Arc::new(CorrectionAwareCompilerModel {
            world_calls: AtomicUsize::new(0),
            saw_previous_structure: AtomicBool::new(false),
        });
        let compiler = WorldCompiler::new(vault(), model.clone(), "flash", "pro");
        let (_, receipts) = compiler
            .compile_custom(CustomStart {
                campaign_name: "Correction context".into(),
                who: "worker".into(),
                where_: "yard".into(),
                when: "fixture".into(),
                goal: "learn".into(),
            })
            .await
            .unwrap();
        assert!(model.saw_previous_structure.load(Ordering::SeqCst));
        let world_receipts = receipts
            .iter()
            .filter(|receipt| receipt.stage == "world_compile")
            .collect::<Vec<_>>();
        assert_eq!(world_receipts.len(), 2);
        assert_eq!(world_receipts[0].validation_result, "semantic_invalid");
        assert_eq!(world_receipts[1].validation_result, "valid");
    }
}
