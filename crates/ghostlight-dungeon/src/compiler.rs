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
    pub model_receipt: ModelStageReceipt,
    pub retrieval_receipt: ModelStageReceipt,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SuggestedRoles {
    pub roles: Vec<RoleSuggestion>,
    pub evidence_receipts: Vec<VaultEvidenceReceipt>,
    pub model_receipt: ModelStageReceipt,
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
        let (queries, retrieval_receipt) = self
            .plan_queries(
                "opening_retrieval_plan",
                "opening-suggestions",
                &serde_json::to_string(&request)?,
                3,
            )
            .await?;
        let receipts = self.retrieve_all(&queries, "all", 8).await?;
        let evidence = evidence_text(&receipts);
        let output = self.structured("world_openings", "opening-suggestions", &format!("Generate exactly three source-grounded openings. They must use distinct eras, places, and pressures. Do not fill material evidence gaps with invention. REQUEST:\n{}\nEVIDENCE:\n{}", serde_json::to_string(&request)?, evidence), serde_json::to_value(schema_for!(OpeningSet))?, receipt_ids(&receipts)).await?;
        let parsed: OpeningSet = serde_json::from_value(output.0)?;
        if parsed.openings.len() != 3 {
            return Err(anyhow!("world compiler must return exactly three openings"));
        }
        ensure_distinct_openings(&parsed.openings)?;
        Ok(SuggestedOpenings {
            openings: parsed.openings,
            evidence_receipts: receipts,
            model_receipt: output.1,
            retrieval_receipt,
        })
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
        let output = self.structured("world_roles", &format!("roles:{}", opening.id), &format!("Generate exactly three materially distinct player roles grounded in this opening and evidence. OPENING:\n{}\nEVIDENCE:\n{}", serde_json::to_string(opening)?, evidence_text(&receipts)), serde_json::to_value(schema_for!(RoleSet))?, receipt_ids(&receipts)).await?;
        let parsed: RoleSet = serde_json::from_value(output.0)?;
        if parsed.roles.len() != 3 {
            return Err(anyhow!("world compiler must return exactly three roles"));
        }
        Ok(SuggestedRoles {
            roles: parsed.roles,
            evidence_receipts: receipts,
            model_receipt: output.1,
            retrieval_receipt,
        })
    }

    pub async fn compile_custom(
        &self,
        start: CustomStart,
    ) -> Result<(WorldCompilePreview, Vec<ModelStageReceipt>)> {
        let (queries, retrieval_receipt) = self
            .plan_queries(
                "custom_retrieval_plan",
                "custom-start",
                &serde_json::to_string(&start)?,
                3,
            )
            .await?;
        let receipts = self.retrieve_all(&queries, &start.when, 8).await?;
        let (evidence_coverage, relevance_receipts) =
            self.classify_evidence(&start, &receipts).await?;
        let scoped_evidence = scoped_evidence_text(&receipts, &evidence_coverage);
        let shared_prefix = format!(
            "SOURCE-GROUNDED WORLD COMPILATION\nSTART:\n{}\nSCOPED EVIDENCE:\n{}\n\n",
            serde_json::to_string(&start)?,
            scoped_evidence
        );
        let base_prompt = format!(
            "{shared_prefix}Compile a bounded playable region with stable topology, local actors, populations, clocks, and only those remote institutions that have a direct causal relationship to this requested start. Evidence marked direct_seed may shape the local situation. Evidence marked setting_background may establish general history, mechanics, or institutions, but must not import its story-specific cast, incident, clock, location state, goals, or institutional posture into the current branch. A matching place name or era alone does not make another source episode current. When the evidence cannot ground a requested local detail, keep the local cast sparse, mark reversible texture provisional_local, and list the material gap instead of borrowing a nearby story. Do not eagerly invent remote settlements, routes, or people. Emit only supported canon facts. A canon_baseline fact must cite one or more exact receipt_id values printed in SCOPED EVIDENCE whose witnesses directly support the whole statement. Never label an invented proper noun canon. The player location and every actor location must exist. Every route destination must exist, travel time must be positive, clocks need positive thresholds, and the player id must be unique. Represent populations that can act collectively (villages, crews, crowds, departments, corporations) as gestalt Personas. Seed a small roster of plausible durable member identities for people the player may encounter; member deltas contain only departures from their gestalt baseline and begin dematerialized. Do not duplicate a gestalt member in actors. Keep named plot-critical people as ordinary actors. Every gestalt home location and member gestalt reference must exist. Do not emit agency profiles or relations; those are compiled from the exact validated subject roster in the next stage."
        );
        let schema = serde_json::to_value(schema_for!(CompiledSeed))?;
        let sources = receipt_ids_for_coverage(&receipts, &evidence_coverage);
        let mut compiler_receipts = Vec::new();
        let mut correction = String::new();
        let (seed, campaign) = loop {
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
            match seed_to_campaign(seed.clone(), &receipts)
                .and_then(|campaign| validate_campaign_seed(&campaign).map(|_| campaign))
            {
                Ok(campaign) => break (seed, campaign),
                Err(error) if compiler_receipts.len() == 1 => {
                    mark_semantic_invalid(
                        compiler_receipts
                            .last_mut()
                            .expect("receipt was just stored"),
                        &error,
                    );
                    correction = format!(
                        "\n\nLOCAL VALIDATOR REJECTED THE PREVIOUS CANDIDATE: {error}\nReturn a corrected complete candidate against the same START and EVIDENCE."
                    );
                }
                Err(error) => {
                    return Err(anyhow!(
                        "world compiler failed local validation after one correction: {error}"
                    ));
                }
            }
        };
        let subject_briefs = agency_subject_briefs(&campaign);
        let agency_prompt = format!(
            "MULTIRESOLUTION AGENCY SKELETON\nCompile only this exact, already validated subject roster:\n{}\n\nReturn exactly one agency profile for every supplied subject and no other subject. Copy every subject_id, subject_kind, and location_ids exactly. Every profile must contain exactly the six facet axes geography, ideology, authority, economy_role, species_body, and information. Derive facets only from the supplied roster fields; use an explicit unknown value when they do not support a sharper claim. collective_authority_id must be null or one supplied subject ID; it denotes real shared authority, never mere alliance or proximity. Relations may use only supplied subject IDs and strength must be an integer from 1 through 100. Cross-faction relations never imply shared speech, knowledge, or authority. Preserve geographic, ideological, institutional, economic, biological, and information boundaries that predict different behavior under pressure.",
            serde_json::to_string(&subject_briefs)?
        );
        let agency_schema = serde_json::to_value(schema_for!(CompiledAgencySkeleton))?;
        let mut agency_correction = String::new();
        let mut campaign = campaign;
        loop {
            let output = self
                .structured(
                    "agency_compile",
                    "custom-start",
                    &format!("{agency_prompt}{agency_correction}"),
                    agency_schema.clone(),
                    sources.clone(),
                )
                .await?;
            compiler_receipts.push(output.1);
            let skeleton: CompiledAgencySkeleton = serde_json::from_value(output.0)?;
            let mut candidate = campaign.clone();
            match apply_compiled_agency_skeleton(
                &mut candidate,
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
        model_receipts.extend(compiler_receipts);
        Ok((
            WorldCompilePreview {
                schema: "ghostlight.world_compile_preview.v1".into(),
                title: seed.title,
                campaign,
                evidence_receipts: receipts,
                evidence_coverage,
                gaps: seed.gaps,
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
        self.compile_custom(CustomStart {
            campaign_name: start.campaign_name,
            who: format!("{} — {}", start.role.name, start.role.premise),
            where_: start.opening.place,
            when: start.opening.era,
            goal: format!("{}; {}", start.opening.player_hook, start.opening.pressure),
        })
        .await
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
        let parent = campaign
            .gestalts
            .get(&request.parent_gestalt_id)
            .ok_or_else(|| anyhow!("fission parent is unknown"))?;
        let requested: BTreeSet<_> = request
            .requested_partition_values
            .iter()
            .map(|value| value.trim().to_ascii_lowercase())
            .collect();
        if request.reason.trim().is_empty()
            || requested.is_empty()
            || requested.len() != request.requested_partition_values.len()
            || requested.contains("other/unknown")
            || request
                .requested_partition_values
                .iter()
                .any(|value| value.trim().is_empty())
        {
            return Err(anyhow!(
                "fission request needs distinct named cuts and a reason"
            ));
        }
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
            "Compile only the requested bounded destination region. Every new location id must be new. At least one new location must route back to origin id {} with a positive travel time. Do not rewrite existing geography. CAMPAIGN LOCATIONS:\n{}\nREQUEST:\n{}\nEVIDENCE:\n{}",
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

fn scoped_evidence_text(
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
            if use_plan.lane == EvidenceUseLane::Excluded
                || !seen.insert((
                    witness.source_id.clone(),
                    witness.exact_locator.clone(),
                    witness.content_hash.clone(),
                ))
            {
                return None;
            }
            let lane = match use_plan.lane {
                EvidenceUseLane::DirectSeed => "direct_seed",
                EvidenceUseLane::SettingBackground => "setting_background",
                EvidenceUseLane::Excluded => unreachable!(),
            };
            Some(format!(
                "[usage_lane={} | rationale={} | receipt_id={} | source={} | locator={} | content_hash={}] {}",
                lane,
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
        .filter(|item| item.lane != EvidenceUseLane::Excluded)
        .map(|item| item.source_id.as_str())
        .collect::<BTreeSet<_>>();
    receipts
        .iter()
        .filter(|receipt| {
            receipt
                .witnesses
                .iter()
                .any(|witness| included_sources.contains(witness.source_id.as_str()))
        })
        .map(|receipt| receipt.id.clone())
        .collect()
}

fn receipt_ids(receipts: &[VaultEvidenceReceipt]) -> Vec<String> {
    receipts.iter().map(|r| r.id.clone()).collect()
}
fn ensure_distinct_openings(items: &[OpeningSuggestion]) -> Result<()> {
    let eras: BTreeSet<_> = items.iter().map(|x| x.era.trim().to_lowercase()).collect();
    let places: BTreeSet<_> = items
        .iter()
        .map(|x| x.place.trim().to_lowercase())
        .collect();
    let pressures: BTreeSet<_> = items
        .iter()
        .map(|x| x.pressure.trim().to_lowercase())
        .collect();
    if eras.len() != 3 || places.len() != 3 || pressures.len() != 3 {
        Err(anyhow!(
            "openings are not distinct across era, place, and pressure"
        ))
    } else {
        Ok(())
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

fn agency_subject_briefs(campaign: &Campaign) -> Vec<AgencySubjectBrief> {
    let mut briefs = Vec::new();
    for actor in campaign
        .actors
        .values()
        .filter(|actor| actor.id != campaign.player_actor_id)
    {
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
    for institution in campaign.institutions.values() {
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
    for gestalt in campaign.gestalts.values() {
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
    profiles: Vec<CompiledAgencyProfile>,
    relations: Vec<CompiledAgencyRelation>,
) -> Result<()> {
    let expected: BTreeSet<_> = campaign
        .agency_profiles
        .values()
        .filter(|profile| profile.active_leaf && profile.simulation_eligible)
        .map(|profile| profile.subject_id.clone())
        .collect();
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
    if supplied != expected || supplied.len() != profiles.len() {
        let missing = expected.difference(&supplied).cloned().collect::<Vec<_>>();
        let unexpected = supplied.difference(&expected).cloned().collect::<Vec<_>>();
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
            .is_none_or(|id| campaign.agency_profiles.contains_key(id));
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
    for actor in c.actors.values() {
        if !c.locations.contains_key(&actor.location_id) {
            return Err(anyhow!(
                "actor {} occupies unknown location {}",
                actor.id,
                actor.location_id
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
    for clock in c.clocks.values() {
        if clock.threshold == 0 || clock.progress > clock.threshold {
            return Err(anyhow!("clock {} is invalid", clock.id));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{domain::SourceWitness, model::ModelPort, vault::FixtureVault};
    use async_trait::async_trait;
    use sha2::Digest;

    struct CompilerModel {
        invalid_route: bool,
    }

    struct OversizedQueryModel;
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
                "world_openings" => serde_json::json!({"openings":[
                    {"id":"a","title":"Ash","era":"early","place":"ring","pressure":"strike","player_hook":"work","evidence_receipt_ids":[]},
                    {"id":"b","title":"Glass","era":"middle","place":"moon","pressure":"siege","player_hook":"survive","evidence_receipt_ids":[]},
                    {"id":"c","title":"Rain","era":"late","place":"station","pressure":"election","player_hook":"choose","evidence_receipt_ids":[]}
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
                        "facts":[{"id":"f","statement":"A witnessed fact","scope":"canon_baseline","evidence_receipt_ids":["fixture"]}],
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
                excerpt: "A stable witnessed place.".into(),
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
        assert_eq!(preview.campaign.agency_profiles.len(), 2);
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
    fn scoped_evidence_excludes_nearby_story_incidents_from_world_context() {
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
                lane: EvidenceUseLane::Excluded,
                rationale: "The incident is unrelated to the requested start.".into(),
            },
        ];
        let text = scoped_evidence_text(&receipts, &coverage);
        assert!(text.contains("The requested station exists."));
        assert!(!text.contains("unrelated named cast"));
        assert_eq!(
            receipt_ids_for_coverage(&receipts, &coverage),
            vec!["vault:direct"]
        );
    }

    #[test]
    fn agency_compile_schema_exposes_relation_strength_domain() {
        let schema = serde_json::to_value(schema_for!(CompiledAgencySkeleton)).unwrap();
        let serialized = serde_json::to_string(&schema).unwrap();
        assert!(serialized.contains("\"minimum\":1"));
        assert!(serialized.contains("\"maximum\":100"));
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
}
