use crate::{
    domain::{
        ActorState, AgencyAxis, AgencyRelation, AgencyRelationKind, AgencySubjectKind,
        BranchOrigin, Campaign, FactScope, GestaltMemberDelta, GestaltPersonaState,
        InstitutionState, Location, VaultEvidenceReceipt, WorldClock, WorldCompilePreview,
        WorldFact,
    },
    model::{ModelPort, ModelStageReceipt, ModelStageRequest, run_validated_stage},
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
struct CompiledSeed {
    title: String,
    canon_cutoff: String,
    world_time: DateTime<Utc>,
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
    #[serde(default)]
    agency_profiles: Vec<CompiledAgencyProfile>,
    #[serde(default)]
    agency_relations: Vec<CompiledAgencyRelation>,
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
        let receipts = self.retrieve_all(&queries, &start.when, 18).await?;
        let base_prompt = format!(
            "Compile two deliberately different scales. First, compile a bounded playable region with stable topology, local actors, populations, and clocks. Second, compile a global agency skeleton covering every source-supported major power, coarse region, institution, important relation, information channel, and strategic pressure relevant to this era. Major remote powers belong in institutions and agency profiles; do not eagerly invent their settlements, routes, or people. Agency profile facets use exactly geography, ideology, authority, economy_role, species_body, and information. Agency relations use exact supplied subject IDs. Emit only supported canon facts; mark reversible texture provisional_local and list material gaps. The player location and every actor location must exist. Every route destination must exist, travel time must be positive, clocks need positive thresholds, and the player id must be unique. Represent populations that can act collectively (villages, crews, crowds, departments, corporations) as gestalt Personas. Seed a small roster of plausible durable member identities for people the player may encounter; member deltas contain only departures from their gestalt baseline and begin dematerialized. Do not duplicate a gestalt member in actors. Keep named plot-critical people as ordinary actors. Every gestalt home location and member gestalt reference must exist. Every non-player actor, institution, and gestalt must have exactly one agency profile; location IDs must already exist. Cross-faction relations never imply shared authority. START:\n{}\nEVIDENCE:\n{}",
            serde_json::to_string(&start)?,
            evidence_text(&receipts)
        );
        let schema = serde_json::to_value(schema_for!(CompiledSeed))?;
        let sources = receipt_ids(&receipts);
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
        let mut model_receipts = vec![retrieval_receipt];
        model_receipts.extend(compiler_receipts);
        Ok((
            WorldCompilePreview {
                schema: "ghostlight.world_compile_preview.v1".into(),
                title: seed.title,
                campaign,
                evidence_receipts: receipts,
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
            "Plan exactly {count} distinct source-search queries for the supplied subject. Each query must be a concise natural-language search string of 1 to 240 Unicode characters. Preserve proper nouns, era, place, institutions, mechanics, geography, and pressure when relevant. Do not answer the subject. SUBJECT:\n{subject}\n\nOUTPUT JSON SCHEMA (follow exactly):\n{}",
            serde_json::to_string_pretty(&schema)?
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

    async fn structured(
        &self,
        stage: &str,
        binding: &str,
        prompt: &str,
        schema: serde_json::Value,
        sources: Vec<String>,
    ) -> Result<(serde_json::Value, ModelStageReceipt)> {
        let prompt = format!(
            "{prompt}\n\nOUTPUT JSON SCHEMA (follow exactly):\n{}",
            serde_json::to_string_pretty(&schema)?
        );
        let out = run_validated_stage(
            self.model.as_ref(),
            &ModelStageRequest {
                stage: stage.into(),
                model: self.compiler_model.clone(),
                snapshot_binding: binding.into(),
                lived_stream: prompt,
                output_schema: Some(schema),
                source_receipt_ids: sources,
            },
        )
        .await?;
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
    receipts
        .iter()
        .flat_map(|r| r.witnesses.iter())
        .map(|w| {
            format!(
                "[{} | {} | {}] {}",
                w.source_id, w.exact_locator, w.content_hash, w.excerpt
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n")
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
    let id = Uuid::new_v4();
    let compiled_agency_profiles = seed.agency_profiles.clone();
    let compiled_agency_relations = seed.agency_relations.clone();
    let player_id = seed.player.id.clone();
    let now = Utc::now();
    let mut actors: BTreeMap<_, _> = seed.actors.into_iter().map(|x| (x.id.clone(), x)).collect();
    if actors.insert(player_id.clone(), seed.player).is_some() {
        return Err(anyhow!("player id duplicates an NPC"));
    }
    let evidence_receipt_ids = receipt_ids(receipts);
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
                if x.scope == FactScope::CanonBaseline && x.evidence_receipt_ids.is_empty() {
                    x.scope = FactScope::ProvisionalLocal
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
    apply_compiled_agency_skeleton(
        &mut campaign,
        compiled_agency_profiles,
        compiled_agency_relations,
    )?;
    Ok(campaign)
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
        return Err(anyhow!(
            "global agency skeleton must profile every non-player actor, institution, and gestalt exactly once"
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
        if profile.subject_kind != input.subject_kind
            || input_axes != axes
            || input
                .location_ids
                .iter()
                .any(|id| !campaign.locations.contains_key(id))
            || !authority_known
        {
            return Err(anyhow!("compiled agency profile is malformed"));
        }
        profile.collective_authority_id = input.collective_authority_id;
        profile.facets = input.facets;
        profile.location_ids = input.location_ids;
        profile.information_channels = input.information_channels;
        profile.evidence_receipt_ids = campaign.branch_origin.evidence_receipt_ids.clone();
    }
    let mut relation_ids = BTreeSet::new();
    for input in relations {
        if !relation_ids.insert(input.id.clone())
            || input.id.trim().is_empty()
            || input.from_subject_id == input.to_subject_id
            || !expected.contains(&input.from_subject_id)
            || !expected.contains(&input.to_subject_id)
            || input.strength == 0
            || input.strength > 100
        {
            return Err(anyhow!("compiled agency relation is malformed"));
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
            return Err(anyhow!("location {} has invalid container", location.id));
        }
        for route in location.routes.values() {
            if route.travel_minutes == 0 || !c.locations.contains_key(&route.destination_id) {
                return Err(anyhow!("location {} has invalid route", location.id));
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
                        "gaps":["Who owns the outer gate?"],"branch_assumptions":[],"opening_narration":"The yard persists.",
                        "agency_profiles":[{"subject_id":"yard-workers","subject_kind":"gestalt","collective_authority_id":"yard-workers","facets":{"geography":["yard"],"ideology":["mutual aid"],"authority":["yard-workers"],"economy_role":["maintenance"],"species_body":["human"],"information":["yard routines"]},"location_ids":["yard"],"information_channels":["yard routines"]}],
                        "agency_relations":[]
                    }).to_string()
                }
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
        let (preview, _) = compiler
            .compile_custom(CustomStart {
                campaign_name: "Test".into(),
                who: "worker".into(),
                where_: "yard".into(),
                when: "fixture".into(),
                goal: "learn".into(),
            })
            .await
            .unwrap();
        assert!(preview.requires_approval);
        assert_eq!(preview.campaign.revision, 0);
        assert_eq!(preview.campaign.locations.len(), 1);
        assert_eq!(preview.campaign.canon_candidates.len(), 1);
        assert_eq!(preview.campaign.gestalts.len(), 1);
        assert_eq!(preview.campaign.agency_profiles.len(), 2);
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
        assert!(result.unwrap_err().to_string().contains("invalid route"));
    }
}
