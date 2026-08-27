use crate::{
    domain::{
        ActorState, AgencyAxis, AgencyProfile, AgencyRelation, AgencyRelationKind,
        AgencySubjectKind, BranchOrigin, Campaign, CivicSystemManifest,
        DestinationCompilationPreview, EvidenceCoverage, EvidenceUseLane, FactScope,
        GestaltMemberDelta, GestaltPersonaState, InstitutionState, LocalityElaboration,
        LocalityElaborationPreview, Location, MAX_POSTURE_CHARS, Route, VaultEvidenceReceipt,
        WorldClock, WorldCompilePreview, WorldFact,
    },
    model::{
        ModelPort, ModelStageReceipt, ModelStageRequest, run_validated_stage,
        run_validated_stage_with_timeout,
    },
    session_zero::{
        ApprovedCampaignBrief, CampaignContract, MAX_SESSION_ZERO_MEMBERS, actor_from_character,
    },
    vault::{DEFAULT_VAULT_ID, VaultProvider, VaultQuery, canonical_vault_id},
};
use anyhow::{Result, anyhow};
use chrono::{DateTime, Utc};
use schemars::{JsonSchema, schema_for};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    cmp::Reverse,
    collections::{BTreeMap, BTreeSet, BinaryHeap},
    sync::Arc,
};
use uuid::Uuid;

const MAX_PARTY_IDENTITY_CHARS: usize = MAX_SESSION_ZERO_MEMBERS * 1_000;

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

#[derive(Clone, Debug, Serialize, PartialEq)]
struct RequiredRelationshipActor {
    id: String,
    name: String,
    approved_relationship_descriptions: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
struct ApprovedRelationshipPlan {
    anchors: Vec<RequiredRelationshipActor>,
    targets: BTreeMap<(String, String), String>,
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

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum DestinationIdentityDecision {
    Existing,
    New,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
struct DestinationIdentityResolution {
    decision: DestinationIdentityDecision,
    existing_location_id: Option<String>,
    rationale: String,
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
struct CompiledRelationship {
    subject_id: String,
    description: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
struct CompiledActorState {
    id: String,
    name: String,
    location_id: String,
    capabilities: BTreeSet<String>,
    knowledge: BTreeSet<String>,
    equipment: BTreeSet<String>,
    conditions: BTreeSet<String>,
    obligations: BTreeSet<String>,
    relationships: Vec<CompiledRelationship>,
    goals: Vec<String>,
    #[serde(default)]
    memories: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
struct CompiledRoute {
    route_id: String,
    destination_id: String,
    distance: String,
    #[schemars(range(min = 1))]
    travel_minutes: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
struct CompiledLocation {
    id: String,
    name: String,
    #[schemars(
        description = "Geometric containment only. This does not create an implicit movement edge."
    )]
    container_id: Option<String>,
    #[schemars(
        description = "Explicit directed movement edges. A playable opening must provide route chains from the player location to every supplied location and back."
    )]
    routes: Vec<CompiledRoute>,
    persistent_features: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
struct CompiledGestaltMemberDelta {
    schema: String,
    #[schemars(
        description = "Local stable member ID without the world-subject `member:` namespace prefix."
    )]
    id: String,
    gestalt_id: String,
    version: u64,
    name: String,
    capability_additions: BTreeSet<String>,
    capability_removals: BTreeSet<String>,
    knowledge_additions: BTreeSet<String>,
    knowledge_removals: BTreeSet<String>,
    equipment: BTreeSet<String>,
    conditions: BTreeSet<String>,
    obligations: BTreeSet<String>,
    relationships: Vec<CompiledRelationship>,
    goals: Vec<String>,
    memories: Vec<String>,
    last_location_id: Option<String>,
    materialized_actor_id: Option<String>,
    #[serde(default)]
    last_relevant_revision: u64,
    #[serde(default)]
    relevance_lease_until_revision: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
struct CompiledSeed {
    title: String,
    canon_cutoff: String,
    world_time: DateTime<Utc>,
    #[schemars(range(min = 1))]
    tick_hours: u32,
    player: CompiledActorState,
    locations: Vec<CompiledLocation>,
    actors: Vec<CompiledActorState>,
    #[serde(default)]
    gestalts: Vec<GestaltPersonaState>,
    #[serde(default)]
    gestalt_members: Vec<CompiledGestaltMemberDelta>,
    institutions: Vec<InstitutionState>,
    clocks: Vec<WorldClock>,
    facts: Vec<WorldFact>,
    #[schemars(
        description = "Only premise-blocking gaps. Ordinary missing game-scale geometry, routes, people, procedures, daily texture, intentionally unresolved population detail, and safely omitted remote candidates are branch-local elaboration or coverage limits, never material gaps."
    )]
    gaps: Vec<CompiledMaterialGap>,
    branch_assumptions: Vec<String>,
    opening_narration: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
enum CompiledMaterialGapKind {
    ContradictoryCanonBaselines,
    UnanchoredRequestedBaseline,
    ApprovedCapabilityConflict,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
struct CompiledMaterialGap {
    kind: CompiledMaterialGapKind,
    #[schemars(
        description = "Concise description of why no compatible branch-local elaboration can preserve the approved premise."
    )]
    summary: String,
    #[schemars(
        description = "Exact approved request or Session Zero premise clause that cannot be preserved."
    )]
    premise_clause: String,
    #[schemars(
        description = "The mutually exclusive canon baseline, premise change, or capability bargain the table must choose."
    )]
    blocked_choice: String,
    #[schemars(
        description = "Exact supplied Vault receipt IDs supporting the contradiction, when the gap is grounded in conflicting canon."
    )]
    evidence_receipt_ids: Vec<String>,
}

impl From<ActorState> for CompiledActorState {
    fn from(actor: ActorState) -> Self {
        Self {
            id: actor.id,
            name: actor.name,
            location_id: actor.location_id,
            capabilities: actor.capabilities,
            knowledge: actor.knowledge,
            equipment: actor.equipment,
            conditions: actor.conditions,
            obligations: actor.obligations,
            relationships: actor
                .relationships
                .into_iter()
                .map(|(subject_id, description)| CompiledRelationship {
                    subject_id,
                    description,
                })
                .collect(),
            goals: actor.goals,
            memories: actor.memories,
        }
    }
}

impl CompiledActorState {
    fn into_actor(self) -> Result<ActorState> {
        Ok(ActorState {
            id: self.id,
            name: self.name,
            location_id: self.location_id,
            capabilities: self.capabilities,
            knowledge: self.knowledge,
            equipment: self.equipment,
            conditions: self.conditions,
            obligations: self.obligations,
            relationships: compiled_relationship_map(self.relationships)?,
            goals: self.goals,
            memories: self.memories,
        })
    }
}

impl CompiledLocation {
    fn into_location(self) -> Result<Location> {
        let routes = compiled_route_map(self.routes, &self.id)?;
        Ok(Location {
            id: self.id,
            name: self.name,
            container_id: self.container_id,
            routes,
            persistent_features: self.persistent_features,
        })
    }
}

fn compiled_route_map(
    compiled_routes: Vec<CompiledRoute>,
    origin_id: &str,
) -> Result<BTreeMap<String, Route>> {
    let mut routes = BTreeMap::new();
    for route in compiled_routes {
        if route.route_id.trim().is_empty()
            || routes
                .insert(
                    route.route_id.clone(),
                    Route {
                        destination_id: route.destination_id,
                        distance: route.distance,
                        travel_minutes: route.travel_minutes,
                    },
                )
                .is_some()
        {
            return Err(anyhow!(
                "compiled location {origin_id} has an empty or duplicate route ID {:?}",
                route.route_id
            ));
        }
    }
    Ok(routes)
}

impl CompiledGestaltMemberDelta {
    fn into_member(self) -> Result<GestaltMemberDelta> {
        Ok(GestaltMemberDelta {
            schema: self.schema,
            id: self.id,
            gestalt_id: self.gestalt_id,
            version: self.version,
            name: self.name,
            capability_additions: self.capability_additions,
            capability_removals: self.capability_removals,
            knowledge_additions: self.knowledge_additions,
            knowledge_removals: self.knowledge_removals,
            equipment: self.equipment,
            conditions: self.conditions,
            obligations: self.obligations,
            relationships: compiled_relationship_map(self.relationships)?,
            goals: self.goals,
            memories: self.memories,
            last_location_id: self.last_location_id,
            materialized_actor_id: self.materialized_actor_id,
            last_relevant_revision: self.last_relevant_revision,
            relevance_lease_until_revision: self.relevance_lease_until_revision,
        })
    }
}

fn compiled_relationship_map(
    relationships: Vec<CompiledRelationship>,
) -> Result<BTreeMap<String, String>> {
    let mut mapped = BTreeMap::new();
    for relationship in relationships {
        if relationship.subject_id.trim().is_empty()
            || relationship.description.trim().is_empty()
            || mapped
                .insert(relationship.subject_id.clone(), relationship.description)
                .is_some()
        {
            return Err(anyhow!(
                "compiled relationships require unique non-empty subject IDs and descriptions; rejected subject {:?}",
                relationship.subject_id
            ));
        }
    }
    Ok(mapped)
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
struct PrivateRelationshipActorCandidate {
    name: String,
    location_id: String,
    capabilities: BTreeSet<String>,
    knowledge: BTreeSet<String>,
    equipment: BTreeSet<String>,
    conditions: BTreeSet<String>,
    obligations: BTreeSet<String>,
    relationships: Vec<CompiledRelationship>,
    goals: Vec<String>,
    #[serde(default)]
    memories: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
struct PrivateRelationshipActorSet {
    #[schemars(length(max = 64))]
    actors: Vec<PrivateRelationshipActorCandidate>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
struct CompiledAgencySkeleton {
    agency_profiles: Vec<CompiledAgencyProfile>,
    agency_relations: Vec<CompiledAgencyRelation>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
struct ExtractedRemoteInstitution {
    name: String,
    #[schemars(length(min = 1, max = 3))]
    supporting_claims: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
struct ExtractedGlobalAgencyCatalog {
    #[schemars(length(max = 64))]
    institutions: Vec<ExtractedRemoteInstitution>,
    gaps: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
struct GroundedRemoteInstitution {
    name: String,
    supporting_claims: Vec<String>,
    evidence_receipt_ids: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
struct GroundedGlobalAgencyCatalog {
    institutions: Vec<GroundedRemoteInstitution>,
    gaps: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
struct SynthesizedRemoteInstitution {
    name: String,
    strategic_doctrine: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
struct StrategicDoctrineCatalog {
    institutions: Vec<SynthesizedRemoteInstitution>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
struct StrategicDoctrineVerdict {
    name: String,
    compatible_with_canon: bool,
    rationale: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
struct StrategicDoctrineVerification {
    verdicts: Vec<StrategicDoctrineVerdict>,
}

#[derive(Clone, Debug, PartialEq)]
struct CompiledRemoteInstitution {
    name: String,
    strategic_doctrine: String,
    evidence_receipt_ids: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
struct CompiledGlobalAgencyCatalog {
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
    facets: CompiledAgencyFacets,
    location_ids: BTreeSet<String>,
    information_channels: BTreeSet<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
struct CompiledAgencyFacets {
    geography: BTreeSet<String>,
    ideology: BTreeSet<String>,
    authority: BTreeSet<String>,
    economy_role: BTreeSet<String>,
    species_body: BTreeSet<String>,
    information: BTreeSet<String>,
}

impl CompiledAgencyFacets {
    fn into_map(self) -> BTreeMap<AgencyAxis, BTreeSet<String>> {
        BTreeMap::from([
            (AgencyAxis::Geography, self.geography),
            (AgencyAxis::Ideology, self.ideology),
            (AgencyAxis::Authority, self.authority),
            (AgencyAxis::EconomyRole, self.economy_role),
            (AgencyAxis::SpeciesBody, self.species_body),
            (AgencyAxis::Information, self.information),
        ])
    }
}

impl From<BTreeMap<AgencyAxis, BTreeSet<String>>> for CompiledAgencyFacets {
    fn from(mut facets: BTreeMap<AgencyAxis, BTreeSet<String>>) -> Self {
        Self {
            geography: facets.remove(&AgencyAxis::Geography).unwrap_or_default(),
            ideology: facets.remove(&AgencyAxis::Ideology).unwrap_or_default(),
            authority: facets.remove(&AgencyAxis::Authority).unwrap_or_default(),
            economy_role: facets.remove(&AgencyAxis::EconomyRole).unwrap_or_default(),
            species_body: facets.remove(&AgencyAxis::SpeciesBody).unwrap_or_default(),
            information: facets.remove(&AgencyAxis::Information).unwrap_or_default(),
        }
    }
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
    origin_routes: Vec<CompiledRoute>,
    locations: Vec<CompiledLocation>,
    facts: Vec<WorldFact>,
    #[serde(default)]
    #[schemars(length(max = 8))]
    populations: Vec<CompiledDestinationPopulation>,
    #[serde(default)]
    #[schemars(length(max = 12))]
    institutions: Vec<CompiledDestinationInstitution>,
    #[serde(default)]
    #[schemars(length(max = 48))]
    local_relations: Vec<CompiledAgencyRelation>,
    #[serde(default)]
    civic_system: Option<CivicSystemManifest>,
    #[serde(default)]
    #[schemars(length(max = 32))]
    migration_relations: Vec<CompiledDestinationMigrationRelation>,
    #[serde(default)]
    #[schemars(
        length(max = 32),
        description = "Consequential compatible game-scale inventions admitted only for this campaign. Missing routes, geometry, ordinary procedures, supplies, local responsibilities, and operating doctrine belong here rather than in gaps."
    )]
    branch_assumptions: Vec<String>,
    #[serde(default)]
    #[schemars(
        description = "Only premise-blocking material gaps for which no compatible branch-local elaboration can preserve the exact requested destination."
    )]
    gaps: Vec<CompiledMaterialGap>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
struct CompiledDestinationPopulation {
    id: String,
    name: String,
    home_location_id: String,
    shared_capabilities: BTreeSet<String>,
    /// Exact IDs from this expansion's `facts`. The compiler resolves them to
    /// statements after validation so a population cannot acquire unsupported
    /// free-text knowledge during admission.
    shared_fact_ids: BTreeSet<String>,
    resources: BTreeSet<String>,
    goals: Vec<String>,
    pressures: Vec<String>,
    collective_authority_id: Option<String>,
    facets: CompiledAgencyFacets,
    information_channels: BTreeSet<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
struct CompiledDestinationInstitution {
    id: String,
    name: String,
    resources: Vec<String>,
    goals: Vec<String>,
    posture: String,
    location_ids: BTreeSet<String>,
    facets: CompiledAgencyFacets,
    information_channels: BTreeSet<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
struct CivicSystemVerification {
    authority_legible: bool,
    selection_or_succession_legible: bool,
    public_resources_legible: bool,
    redress_legible: bool,
    institutional_relations_coherent: bool,
    resident_answer_grounded: bool,
    rationale: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
struct CompiledDestinationMigrationRelation {
    id: String,
    from_gestalt_id: String,
    to_gestalt_id: String,
    #[schemars(range(min = 1, max = 100))]
    strength: u8,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
struct CompiledFissionSeed {
    children: Vec<GestaltPersonaState>,
    child_partition_values: Vec<CompiledChildPartitionValue>,
    #[serde(default)]
    member_child_assignments: Vec<CompiledMemberChildAssignment>,
    resource_child_assignments: Vec<CompiledResourceChildAssignment>,
    gaps: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
struct CompiledChildPartitionValue {
    child_id: String,
    value: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
struct CompiledMemberChildAssignment {
    member_id: String,
    child_id: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
struct CompiledResourceChildAssignment {
    resource_id: String,
    child_id: String,
}

fn compiled_assignment_map(
    label: &str,
    assignments: impl IntoIterator<Item = (String, String)>,
) -> Result<BTreeMap<String, String>> {
    let mut mapped = BTreeMap::new();
    for (subject_id, child_id) in assignments {
        if subject_id.trim().is_empty()
            || child_id.trim().is_empty()
            || mapped.insert(subject_id.clone(), child_id).is_some()
        {
            return Err(anyhow!(
                "compiled {label} require unique non-empty subject IDs and child IDs; rejected subject {subject_id:?}"
            ));
        }
    }
    Ok(mapped)
}

pub struct WorldCompiler {
    vault: Arc<dyn VaultProvider>,
    model: Arc<dyn ModelPort>,
    retrieval_model: String,
    compiler_model: String,
    vault_id: String,
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
            vault_id: DEFAULT_VAULT_ID.into(),
        }
    }

    pub fn for_vault(&self, vault_id: &str) -> Result<Self> {
        let vault_id = canonical_vault_id(vault_id)
            .ok_or_else(|| anyhow!("unknown or unavailable lore Vault {vault_id:?}"))?;
        Ok(Self {
            vault: self.vault.clone(),
            model: self.model.clone(),
            retrieval_model: self.retrieval_model.clone(),
            compiler_model: self.compiler_model.clone(),
            vault_id: vault_id.into(),
        })
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
        let receipts = self.retrieve_all_player_visible(&queries, "all", 8).await?;
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
            let mut parsed: OpeningSet = serde_json::from_value(value.clone())?;
            for opening in &mut parsed.openings {
                deduplicate_ids(&mut opening.evidence_receipt_ids);
            }
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
        let receipts = self
            .retrieve_all_player_visible(&queries, &opening.era, 8)
            .await?;
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
            let mut parsed: RoleSet = serde_json::from_value(value.clone())?;
            for role in &mut parsed.roles {
                deduplicate_ids(&mut role.evidence_receipt_ids);
            }
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
        self.compile_custom_with_owned_subjects(start, &[], &[], None, None)
            .await
    }

    async fn compile_custom_with_owned_subjects(
        &self,
        start: CustomStart,
        required_relationship_actors: &[RequiredRelationshipActor],
        player_names: &[String],
        private_operational_context: Option<&serde_json::Value>,
        approved_contract: Option<&CampaignContract>,
    ) -> Result<(WorldCompilePreview, Vec<ModelStageReceipt>)> {
        validate_user_text("campaign name", &start.campaign_name, 80)?;
        validate_user_text("player identity", &start.who, MAX_PARTY_IDENTITY_CHARS)?;
        validate_user_text("starting location", &start.where_, 500)?;
        validate_user_text("starting time", &start.when, 500)?;
        validate_user_text("player goal", &start.goal, 1_000)?;
        validate_required_relationship_actor_inputs(required_relationship_actors)?;
        let retrieval_subject = serde_json::to_string(&serde_json::json!({
            "start": &start,
            "approved_contract": approved_contract,
        }))?;
        let (planned_queries, retrieval_receipt) = self
            .plan_queries(
                "custom_retrieval_plan",
                "custom-start",
                &retrieval_subject,
                3,
            )
            .await?;
        let queries = planned_queries;
        let global_queries = global_agency_queries(&start);
        let (local_evidence, global_evidence) = tokio::join!(
            self.retrieve_all(&queries, &start.when, 8),
            self.retrieve_all(&global_queries, &start.when, 12),
        );
        let receipts = local_evidence?;
        let global_receipts = global_evidence?;
        let (classified, global_catalog) = tokio::join!(
            self.classify_evidence(&start, approved_contract, &receipts),
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
        let approved_contract_context = approved_contract
            .map(|contract| -> Result<String> {
                Ok(format!(
                    "\nAPPROVED SESSION ZERO CONTRACT:\n{}\nThis public table-approved contract is causal compilation input. Preserve its premise, opening pressure, tone, pacing, consequences, narrative focus, internal tension, and DM style together; do not reconstruct or substitute them from the narrower legacy START fields. Materialize exact public branch-local opening requirements named by the contract unless they contradict Vault evidence, in which case report an explicit gap.\n",
                    serde_json::to_string(contract)?
                ))
            })
            .transpose()?
            .unwrap_or_default();
        let player_identity_context = if player_names.is_empty() {
            String::new()
        } else {
            format!(
                "\nHUMAN-CONTROLLED PLAYER NAMES:\n{}\nThese people are input context, not world-cast outputs. Do not emit them in actors or gestalt_members; Session Zero materializes their canonical actors after world compilation. The singular player field is only a provisional starting-position marker and must not also appear in actors or gestalt_members.\n",
                serde_json::to_string(player_names)?
            )
        };
        let operational_playability_context = private_operational_context
            .map(|context| -> Result<String> {
                Ok(format!(
                    "\nPRIVATE OPERATIONAL PLAYABILITY INPUT:\n{}\nThis input exists only to make approved player capabilities, equipment, obligations, goals, and extraordinary-permission ceilings actionable in the starting world. It is not public narration, NPC knowledge, or world-cast material. Never copy it into actors, gestalts, institutions, dialogue, or the opening transcript. Use it only to seed bounded branch_local or provisional_local environmental facts at exact relevant locations, or report an explicit evidence gap. For every concrete opening problem that an approved capability or permission could investigate, seed at least one discoverable fact precise enough to produce a meaningful result within that capability's effect ceiling. A generic restatement of the overall crisis is not sufficient. Do not encode private history, secrets, relationships, or pre-existing character knowledge; none are supplied here.\n",
                    serde_json::to_string(context)?
                ))
            })
            .transpose()?
            .unwrap_or_default();
        let base_prompt = format!(
            "{shared_prefix}{approved_contract_context}{player_identity_context}{operational_playability_context}Compile a bounded playable region with stable topology, local actors, populations, clocks, and only those remote institutions that have a direct causal relationship to this requested start. SCOPED EVIDENCE contains direct_seed witnesses only. Setting-background and excluded witnesses remain visible in the approval coverage but are deliberately absent here: they cannot donate cast, incidents, clocks, location state, goals, or institutional posture to this branch. Use evidence as canon constraints, not as an exhaustive game map. A source marked with a `.gm_canon` authority lane may constrain hidden canonical state, but it must not be quoted or paraphrased into opening narration or granted as player or NPC knowledge merely because this compiler received it. When the Vault omits game-scale geometry, routes, local people, procedures, or daily texture, invent the smallest coherent playable elaboration, mark facts branch_local or provisional_local, and disclose consequential choices in branch_assumptions. An unevidenced route needed to connect the bounded region is branch-local geometry, not an evidence gap. Intentionally unresolved identities or population detail remain unresolved branch assumptions, not material gaps. Safely omitted remote candidates are global coverage limits, not material gaps. The `gaps` array is legal only when no compatible elaboration can preserve an exact approved premise clause without choosing between contradictory canon baselines, inventing an unanchored baseline explicitly required by the premise, or exceeding an approved capability. Every gap must name that exact premise clause and the exact choice requiring table approval. `The Vault does not specify X` is never sufficient. Use an empty gaps array when branch-local invention preserves the premise. Never borrow a nearby story to fill a gap. Do not eagerly materialize remote settlements or people outside the bounded playable region. Private character history, secrets, relationships, and relationship subjects are deliberately absent and compile in a separate private stage; do not assume or reconstruct them. Emit only supported canon facts. A canon_baseline fact must cite one or more exact receipt_id values printed in SCOPED EVIDENCE whose witnesses directly support the whole statement. Never label an invented proper noun canon. Facts that an actor can uncover through an admitted local observation must exist before play and list the exact discoverable_at_location_ids where that observation is possible. Seed enough branch_local or provisional_local discoverable facts to make the requested opening pressure and immediate goal actionable; at least one such non-canon fact must be discoverable at the player's exact starting location. The later action assessor can reveal an existing fact but cannot invent one. Facts that are private history or not directly observable have an empty discovery-location set. The player location and every actor location must exist. Containment describes nested geometry; it never creates implicit movement. Every supplied location is a playable occupancy node. When the region contains more than one location, explicit route chains must let the player reach every supplied location from the starting location and return. Model inaccessible scenery as a persistent feature instead of an unreachable location. Every route record needs a stable route_id within its origin, an exact supplied destination_id, a distance, and positive travel_minutes. Every fact discovery location must exist, clocks need positive thresholds, and the player id must be unique. Actor relationship records must use subject_id values copied from exact actor, institution, gestalt, or named-member subject IDs declared in this candidate, never display names, roles, undeclared groups, or location IDs. A relationship to a collective population names its exact gestalt; it does not union that population's knowledge or turn the actor into its authority. Represent populations that can act collectively (villages, crews, crowds, departments, corporations) as gestalt Personas. Seed a small roster of plausible durable member identities for people the player may encounter; member deltas contain only departures from their gestalt baseline and begin dematerialized. Do not duplicate a gestalt member in actors. Keep named plot-critical people as ordinary actors. Every gestalt home location and member gestalt reference must exist. Do not emit agency profiles or relations; those are compiled from the exact validated subject roster in the next stage."
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
            let validation = validate_compiled_material_gaps(&seed.gaps, &receipts)
                .and_then(|()| {
                    validate_shared_seed_excludes_locally_owned_subjects(
                        &seed,
                        required_relationship_actors,
                        player_names,
                    )
                })
                .and_then(|()| seed_to_campaign(seed.clone(), &receipts))
                .and_then(|campaign| {
                    validate_campaign_seed(&campaign)?;
                    validate_opening_playability(&campaign)?;
                    Ok(campaign)
                });
            match validation {
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
                        "\n\nLOCAL VALIDATOR REJECTED THE PREVIOUS CANDIDATE: {error}\nPREVIOUS_CANDIDATE_STRUCTURE:\n{previous_structure}\nReturn a corrected complete candidate against the same START and EVIDENCE. Preserve valid detail, but make every reference use an ID declared by the corrected candidate. ROUTE REPAIR REQUIREMENT: routes are directed movement authority and container_id is geometry only. The corrected candidate must contain an explicit bidirectional spanning route tree rooted at the player's location: every supplied location must have a directed route chain from the player and a directed route chain back. A physically navigable contained location should normally connect to its container in both directions; otherwise connect it through the nearest legitimate occupancy node. Each route record needs a stable route_id within its origin, an exact supplied destination_id, a distance, and positive travel_minutes. Before returning, internally trace player-to-location and location-to-player paths for every location. Do not emit the trace separately."
                    );
                }
                Err(error) => {
                    return Err(anyhow!(
                        "world compiler failed local validation after one correction: {error}"
                    ));
                }
            }
        };
        let (private_relationship_actors, private_actor_receipts) = self
            .compile_private_relationship_actors(
                &start,
                &seed,
                required_relationship_actors,
                &receipts,
            )
            .await?;
        seed.actors.extend(
            private_relationship_actors
                .into_iter()
                .map(CompiledActorState::from),
        );
        compiler_receipts.extend(private_actor_receipts);
        let campaign_with_private_actors = seed_to_campaign(seed.clone(), &receipts)?;
        validate_campaign_seed(&campaign_with_private_actors)?;
        validate_opening_playability(&campaign_with_private_actors)?;
        validate_required_relationship_actors(
            &campaign_with_private_actors,
            required_relationship_actors,
        )?;
        let (remote_institution_evidence, global_agency_assumptions) =
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
        validate_required_relationship_actors(&campaign, required_relationship_actors)?;
        let subject_briefs = agency_subject_briefs(&campaign, &remote_institution_ids);
        let modeled_subject_ids = subject_briefs
            .iter()
            .map(|brief| brief.subject_id.clone())
            .collect::<BTreeSet<_>>();
        let agency_prompt = format!(
            "MULTIRESOLUTION AGENCY SKELETON\nCompile only this exact, already validated subject roster:\n{}\n\nReturn exactly one agency profile for every supplied subject and no other subject. Copy every subject_id, subject_kind, and location_ids exactly. Every profile must contain exactly the six facet axes geography, ideology, authority, economy_role, species_body, and information. Derive facets only from the supplied roster fields; use an explicit unknown value when they do not support a sharper facet claim. information_channels are concrete routes through which the subject can publish or receive reports (for example a courier wire, bulletin, newspaper, radio net, or word of mouth), never facts the subject knows; do not copy knowledge_or_posture values into information_channels, and use an empty set rather than an unknown placeholder when no route is supported. collective_authority_id must be null or one supplied subject ID; it denotes real shared authority, never mere alliance or proximity. Relations may use only supplied subject IDs and strength must be an integer from 1 through 100. Cross-faction relations never imply shared speech, knowledge, or authority. Preserve geographic, ideological, institutional, economic, biological, and information boundaries that predict different behavior under pressure.",
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
                gaps: seed.gaps.iter().map(material_gap_text).collect(),
                branch_assumptions: seed
                    .branch_assumptions
                    .into_iter()
                    .chain(global_agency_assumptions)
                    .collect(),
                requires_approval: true,
            },
            model_receipts,
        ))
    }

    async fn compile_private_relationship_actors(
        &self,
        start: &CustomStart,
        seed: &CompiledSeed,
        anchors: &[RequiredRelationshipActor],
        evidence_receipts: &[VaultEvidenceReceipt],
    ) -> Result<(Vec<ActorState>, Vec<ModelStageReceipt>)> {
        if anchors.is_empty() {
            return Ok((Vec::new(), Vec::new()));
        }
        validate_required_relationship_actor_inputs(anchors)?;
        let private_context = serde_json::json!({
            "start": start,
            "approved_private_subjects": anchors.iter().map(|anchor| serde_json::json!({
                "name":anchor.name,
                "approved_relationship_descriptions":anchor.approved_relationship_descriptions,
            })).collect::<Vec<_>>(),
            "locations": seed.locations.iter().map(|location| serde_json::json!({
                "id": location.id,
                "name": location.name,
                "container_id": location.container_id,
                "persistent_features": location.persistent_features,
            })).collect::<Vec<_>>(),
            "public_actors": seed.actors.iter().map(|actor| serde_json::json!({
                "id": actor.id,
                "name": actor.name,
                "location_id": actor.location_id,
            })).collect::<Vec<_>>(),
            "public_institutions": seed.institutions.iter().map(|institution| serde_json::json!({
                "id": institution.id,
                "name": institution.name,
                "posture": institution.posture,
            })).collect::<Vec<_>>(),
            "public_populations": seed.gestalts.iter().map(|gestalt| serde_json::json!({
                "id": gestalt.id,
                "name": gestalt.name,
                "home_location_id": gestalt.home_location_id,
                "pressures": gestalt.pressures,
            })).collect::<Vec<_>>(),
        });
        let base_prompt = format!(
            "PRIVATE RELATIONSHIP ACTOR COMPILATION\nThis is a private branch-local stage. Synthesize exactly one ordinary actor candidate for every approved_private_subject in the supplied order. Copy each name exactly. Choose one exact location id from the supplied topology. The approved relationship descriptions are private, player-approved context: use them only to preserve facts explicitly owned by the counterpart, such as that person's role, affiliation, capabilities, knowledge, goals, obligations, or stated feelings. Do not infer that the counterpart knows the player's secrets or private history, and do not convert player-only interpretation into counterpart knowledge. When ownership is ambiguous, omit the fact and let the private approval preview expose the gap. Candidate relationships may name only exact actor, institution, or population IDs supplied in the public world context and should preserve an explicit affiliation or obligation; never invent a player ID or relationship-anchor ID. Return actor candidates only: no canonical IDs, narration, facts, evidence gaps, branch assumptions, agency profiles, or changes to the public world.\nCONTEXT:\n{}",
            serde_json::to_string(&private_context)?
        );
        let mut schema = serde_json::to_value(schema_for!(PrivateRelationshipActorSet))?;
        constrain_private_relationship_actor_schema(&mut schema, anchors, seed)?;
        let sources = receipt_ids(evidence_receipts);
        let mut receipts = Vec::new();
        let mut correction = String::new();
        loop {
            let output = self
                .structured(
                    "private_relationship_actor_compile",
                    "custom-start-private-relationships",
                    &format!("{base_prompt}{correction}"),
                    schema.clone(),
                    sources.clone(),
                )
                .await
                .map_err(|_| {
                    anyhow!(
                        "private relationship actor model stage failed safely; no private diagnostic was published"
                    )
                })?;
            receipts.push(output.1);
            let candidates: PrivateRelationshipActorSet = serde_json::from_value(output.0)
                .map_err(|_| {
                    anyhow!(
                        "private relationship actor model stage failed safely; no private diagnostic was published"
                    )
                })?;
            match materialize_private_relationship_actors(seed, anchors, candidates) {
                Ok(actors) => return Ok((actors, receipts)),
                Err(error) if receipts.len() == 1 => {
                    mark_semantic_invalid(
                        receipts.last_mut().expect("receipt was just stored"),
                        &error,
                    );
                    correction = format!(
                        "\n\nLOCAL VALIDATOR REJECTED THE PREVIOUS PRIVATE ACTOR CANDIDATES: {error}\nReturn one corrected complete candidate set against the same private context."
                    );
                }
                Err(_) => {
                    return Err(anyhow!(
                        "private relationship actor compiler failed local validation after one correction; exact diagnostics remain private"
                    ));
                }
            }
        }
    }

    pub async fn compile_approved_brief(
        &self,
        brief: &ApprovedCampaignBrief,
    ) -> Result<(WorldCompilePreview, Vec<ModelStageReceipt>)> {
        if brief.characters.is_empty() || brief.characters.len() > 8 {
            return Err(anyhow!(
                "approved campaign brief must contain one to eight characters"
            ));
        }
        let host_actor_id = brief
            .member_actor_ids
            .get(&brief.host_member_id)
            .ok_or_else(|| anyhow!("approved campaign brief has no host actor binding"))?
            .clone();
        let public_party = brief
            .characters
            .iter()
            .map(|character| format!("{} — {}", character.name, character.public_premise))
            .collect::<Vec<_>>()
            .join("; ");
        let relationship_plan = approved_relationship_plan(brief)?;
        let player_names = brief
            .characters
            .iter()
            .map(|character| character.name.clone())
            .collect::<Vec<_>>();
        let operational_playability_context = serde_json::json!({
            "characters":brief.characters.iter().map(|character| serde_json::json!({
                "actor_id":character.actor_id,
                "name":character.name,
                "public_premise":character.public_premise,
                "capabilities":character.capabilities,
                "equipment":character.equipment,
                "obligations":character.obligations,
                "goals":character.goals,
                "extraordinary_permissions":character.extraordinary_permissions.iter().map(|permission| serde_json::json!({
                    "name":permission.name,
                    "reliable_scope":permission.reliable_scope,
                    "prerequisites":permission.prerequisites,
                    "costs":permission.costs,
                    "limits":permission.limits,
                    "opposition":permission.opposition,
                    "exposure":permission.exposure,
                    "effect_ceiling":permission.effect_ceiling,
                })).collect::<Vec<_>>(),
            })).collect::<Vec<_>>()
        });
        let mut compiled = self
            .compile_custom_with_owned_subjects(
                CustomStart {
                    campaign_name: brief.contract.campaign_name.clone(),
                    who: format!(
                        "A cooperative party whose public starting identities are: {public_party}. Private histories, secrets, and individual knowledge are deliberately withheld from world generation."
                    ),
                    where_: brief.contract.starting_where.clone(),
                    when: brief.contract.starting_when.clone(),
                    goal: brief.contract.desired_goal.clone(),
                },
                &relationship_plan.anchors,
                &player_names,
                Some(&operational_playability_context),
                Some(&brief.contract),
            )
            .await?;
        let campaign = &mut compiled.0.campaign;
        let generated_player_id = campaign.player_actor_id.clone();
        let starting_location = campaign
            .actors
            .get(&generated_player_id)
            .ok_or_else(|| anyhow!("compiled campaign lost its provisional player"))?
            .location_id
            .clone();
        campaign.actors.remove(&generated_player_id);
        campaign.agency_profiles.remove(&generated_player_id);
        campaign.agency_relations.retain(|_, relation| {
            relation.from_subject_id != generated_player_id
                && relation.to_subject_id != generated_player_id
        });
        for actor in campaign.actors.values_mut() {
            actor.relationships.remove(&generated_player_id);
        }
        let member_actor_ids = brief
            .member_actor_ids
            .values()
            .cloned()
            .collect::<BTreeSet<_>>();
        if member_actor_ids.len() != brief.characters.len() {
            return Err(anyhow!("approved characters must have unique actor IDs"));
        }
        let mut approved_relationship_targets = canonical_relationship_subject_ids(campaign);
        approved_relationship_targets.extend(member_actor_ids.iter().cloned());
        for character in &brief.characters {
            if brief.member_actor_ids.get(&character.member_id) != Some(&character.actor_id) {
                return Err(anyhow!("character and membership actor binding disagree"));
            }
            if campaign.actors.contains_key(&character.actor_id) {
                return Err(anyhow!("approved actor ID collides with compiled cast"));
            }
            let mut actor = actor_from_character(character, starting_location.clone());
            actor.relationships = actor
                .relationships
                .into_iter()
                .map(|(subject, relationship)| {
                    let resolved = relationship_plan
                        .targets
                        .get(&(character.member_id.clone(), subject.clone()))
                        .cloned()
                        .ok_or_else(|| {
                            anyhow!("approved character relationship lost its compiled subject")
                        })?;
                    Ok((resolved, relationship))
                })
                .collect::<Result<BTreeMap<_, _>>>()?;
            if actor
                .relationships
                .keys()
                .any(|id| !approved_relationship_targets.contains(id))
            {
                return Err(anyhow!(
                    "character relationship refers to an unknown canonical subject"
                ));
            }
            campaign.actors.insert(actor.id.clone(), actor);
        }
        campaign.player_actor_id = host_actor_id;
        campaign.name = brief.contract.campaign_name.clone();
        campaign.resolution_policy.active_cell_budget =
            (brief.characters.len() as u8).saturating_mul(8).min(128);
        campaign.resolution_cover = None;
        crate::resolution::ensure_agency_profiles(campaign);
        for actor_id in member_actor_ids {
            let profile = campaign
                .agency_profiles
                .get_mut(&actor_id)
                .ok_or_else(|| anyhow!("approved actor has no agency profile"))?;
            profile.simulation_eligible = false;
        }
        validate_campaign_seed(campaign)?;
        validate_opening_playability(campaign)?;
        compiled.0.branch_assumptions.push(format!(
            "Campaign contract approved in Session Zero {} at shared digest {}.",
            brief.session_zero_id, brief.shared_digest
        ));
        if !relationship_plan.anchors.is_empty() {
            compiled.0.branch_assumptions.push(format!(
                "{} private player-approved relationship subject(s) were materialized without exposing their relationship details to the shared opening.",
                relationship_plan.anchors.len()
            ));
        }
        Ok(compiled)
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
            "Refine one canonical leaf gestalt along exactly the requested facet. Produce one child per requested value plus one mandatory child whose value is exactly 'other/unknown'. Every child starts at version 0 and uses an existing campaign location. Each child must copy the parent's shared capabilities, shared knowledge, goals, and pressures exactly; the partition facet belongs to the agency profile and does not silently rewrite the population baseline. Emit one child_partition_values record per child with child_id and value. Exact scarce resources are not inheritable traits: assign every parent resource to exactly one child through a resource_id/child_id record in resource_child_assignments, and make each child's resources set equal exactly the resources assigned to that child. Do not create, duplicate, omit, or rename a resource. Do not erase or rewrite member deltas. Assign a member only when evidence or durable existing delta supports the cut, using a member_id/child_id record; unassigned members will remain in other/unknown. List every material lore gap. This is an approval preview, not a commit. SUBJECT:\n{}\nEVIDENCE:\n{}",
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
            let child_partition_values = compiled_assignment_map(
                "child partition values",
                compiled
                    .child_partition_values
                    .iter()
                    .map(|entry| (entry.child_id.clone(), entry.value.clone())),
            )?;
            let member_child_assignments = compiled_assignment_map(
                "member child assignments",
                compiled
                    .member_child_assignments
                    .iter()
                    .map(|entry| (entry.member_id.clone(), entry.child_id.clone())),
            )?;
            let resource_child_assignments = compiled_assignment_map(
                "resource child assignments",
                compiled
                    .resource_child_assignments
                    .iter()
                    .map(|entry| (entry.resource_id.clone(), entry.child_id.clone())),
            )?;
            let residual_child_id = child_partition_values
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
                child_partition_values,
                residual_child_id: residual_child_id.unwrap_or_default(),
                member_child_assignments,
                resource_child_assignments,
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

    async fn resolve_destination_identity(
        &self,
        campaign: &Campaign,
        destination_request: &str,
        snapshot_binding: &str,
    ) -> Result<(Option<String>, Vec<ModelStageReceipt>)> {
        let mut schema = serde_json::to_value(schema_for!(DestinationIdentityResolution))?;
        schema["properties"]["existing_location_id"] = serde_json::json!({
            "anyOf":[
                {
                    "type":"string",
                    "enum":campaign.locations.keys().collect::<Vec<_>>()
                },
                {"type":"null"}
            ]
        });
        let locations = campaign
            .locations
            .values()
            .map(|location| {
                serde_json::json!({
                    "id":location.id,
                    "name":location.name,
                    "container_id":location.container_id,
                })
            })
            .collect::<Vec<_>>();
        let base_prompt = format!(
            "OUTPUT JSON SCHEMA (follow exactly):\n{}\n\nResolve only the identity of the player's primary requested destination. `existing` means the place they ultimately ask to reach, revisit, inspect, or materialize is one exact canonical location in KNOWN LOCATIONS, even when they also ask for missing route detail or a playable approach. Copy that exact ID. `new` means the requested primary destination is a genuinely new place; a new room, district, waystation, refuge, route feature, or site may be new even when its description mentions an existing location as context. Do not treat an existing place as new merely because the request asks the compiler to elaborate it. Do not infer aliases without strong support from the request and supplied names. This stage identifies only; it does not plan routes, compile facts, or change state. Keep rationale to one sentence.\n\nKNOWN LOCATIONS:\n{}\n\nREQUEST:\n{}",
            serde_json::to_string(&schema)?,
            serde_json::to_string(&locations)?,
            destination_request,
        );
        let mut correction = String::new();
        let mut receipts = Vec::new();
        for attempt in 0..2 {
            let output = run_validated_stage(
                self.model.as_ref(),
                &ModelStageRequest {
                    stage: "destination_identity_resolution".into(),
                    model: self.retrieval_model.clone(),
                    snapshot_binding: snapshot_binding.into(),
                    lived_stream: format!("{base_prompt}{correction}"),
                    output_schema: Some(schema.clone()),
                    source_receipt_ids: vec![],
                    temperature: Some(0.0),
                    max_output_tokens: Some(384),
                },
            )
            .await?;
            let mut receipt = output.receipt;
            let resolution = output
                .structured
                .ok_or_else(|| {
                    anyhow!("destination identity resolver returned no structured output")
                })
                .and_then(|value| {
                    serde_json::from_value::<DestinationIdentityResolution>(value)
                        .map_err(Into::into)
                });
            let validated = resolution.and_then(|resolution| {
                let rationale_chars = resolution.rationale.trim().chars().count();
                if rationale_chars == 0 || rationale_chars > 500 {
                    return Err(anyhow!(
                        "destination identity rationale must contain 1 to 500 characters"
                    ));
                }
                match (&resolution.decision, &resolution.existing_location_id) {
                    (DestinationIdentityDecision::Existing, Some(location_id))
                        if campaign.locations.contains_key(location_id) =>
                    {
                        Ok(Some(location_id.clone()))
                    }
                    (DestinationIdentityDecision::New, None) => Ok(None),
                    (DestinationIdentityDecision::Existing, _) => Err(anyhow!(
                        "existing destination resolution must name one exact known location"
                    )),
                    (DestinationIdentityDecision::New, Some(_)) => Err(anyhow!(
                        "new destination resolution must not name an existing location"
                    )),
                }
            });
            match validated {
                Ok(location_id) => {
                    receipts.push(receipt);
                    return Ok((location_id, receipts));
                }
                Err(error) if attempt == 0 => {
                    mark_semantic_invalid(&mut receipt, &error);
                    receipts.push(receipt);
                    correction = format!(
                        "\n\nLOCAL VALIDATOR REJECTED THE PREVIOUS IDENTITY RESOLUTION: {error}\nReturn one corrected complete resolution against the same KNOWN LOCATIONS and REQUEST."
                    );
                }
                Err(error) => {
                    mark_semantic_invalid(&mut receipt, &error);
                    return Err(anyhow!(
                        "destination identity resolution failed local validation after one correction: {error}"
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
    ) -> Result<(DestinationCompilationPreview, Vec<ModelStageReceipt>)> {
        validate_user_text("destination request", destination_request, 500)?;
        let origin = campaign
            .locations
            .get(origin_location_id)
            .ok_or_else(|| anyhow!("origin location is unknown"))?;
        let snapshot = format!("campaign:{}:revision:{}", campaign.id, campaign.revision);
        let (existing_destination_id, identity_receipts) = self
            .resolve_destination_identity(campaign, destination_request, &snapshot)
            .await?;
        if let Some(existing_destination_id) = existing_destination_id.as_deref() {
            let destination = campaign
                .locations
                .get(existing_destination_id)
                .expect("destination identity resolver was locally validated");
            let Some((_path, _travel_minutes)) =
                shortest_location_path(campaign, origin_location_id, existing_destination_id)
            else {
                return Err(anyhow!(
                    "{} already exists in canonical campaign topology, but no committed route currently reaches it from {}. No substitute location or route preview was created; request a genuinely new intermediary place or resolve the missing topology explicitly.",
                    destination.name,
                    origin.name,
                ));
            };
        }
        let expansion_origin_location_id = existing_destination_id
            .as_deref()
            .unwrap_or(origin_location_id);
        let expansion_origin = campaign
            .locations
            .get(expansion_origin_location_id)
            .expect("expansion origin is either the validated origin or destination");
        let (queries, retrieval_receipt) = self
            .plan_queries(
                "destination_retrieval_plan",
                &format!("campaign:{}:revision:{}", campaign.id, campaign.revision),
                &serde_json::to_string(&serde_json::json!({
                    "origin": origin,
                    "elaboration_anchor": expansion_origin,
                    "existing_destination_id": existing_destination_id,
                    "destination": destination_request,
                    "canon_cutoff": campaign.branch_origin.canon_cutoff,
                }))?,
                2,
            )
            .await?;
        let receipts = self
            .retrieve_all(&queries, &campaign.branch_origin.canon_cutoff, 10)
            .await?;
        let source_population_ids = campaign
            .gestalts
            .values()
            .filter(|gestalt| {
                gestalt.home_location_id == expansion_origin_location_id
                    && campaign
                        .agency_profiles
                        .get(&gestalt.id)
                        .is_some_and(|profile| profile.active_leaf)
            })
            .map(|gestalt| gestalt.id.clone())
            .collect::<BTreeSet<_>>();
        let origin_population_context = campaign
            .gestalts
            .values()
            .filter(|gestalt| source_population_ids.contains(&gestalt.id))
            .map(|gestalt| {
                serde_json::json!({
                    "id":gestalt.id,
                    "name":gestalt.name,
                    "home_location_id":gestalt.home_location_id,
                    "goals":gestalt.goals,
                    "pressures":gestalt.pressures,
                    "named_members":campaign.gestalt_members.values()
                        .filter(|member| member.gestalt_id == gestalt.id
                            && member.materialized_actor_id.is_none()
                            && member.last_location_id.as_deref().unwrap_or(&gestalt.home_location_id) == expansion_origin_location_id)
                        .map(|member| serde_json::json!({
                            "id":member.id,
                            "name":member.name,
                            "goals":member.goals,
                            "obligations":member.obligations,
                            "memories":member.memories,
                        }))
                        .collect::<Vec<_>>(),
                })
            })
            .collect::<Vec<_>>();
        let current_civic_context = campaign
            .civic_systems
            .get(expansion_origin_location_id)
            .map(|system| {
                serde_json::json!({
                    "manifest":system,
                    "institutions":system.governing_institution_ids.iter().filter_map(|id|campaign.institutions.get(id)).collect::<Vec<_>>(),
                    "institution_profiles":system.governing_institution_ids.iter().filter_map(|id|campaign.agency_profiles.get(id)).collect::<Vec<_>>(),
                    "residents":system.resident_population_ids.iter().filter_map(|id|campaign.gestalts.get(id)).collect::<Vec<_>>(),
                    "resident_profiles":system.resident_population_ids.iter().filter_map(|id|campaign.agency_profiles.get(id)).collect::<Vec<_>>(),
                    "public_facts":system.public_authority_fact_ids.iter()
                        .chain(system.public_selection_fact_ids.iter())
                        .chain(system.public_resource_fact_ids.iter())
                        .chain(system.public_redress_fact_ids.iter())
                        .filter_map(|id|campaign.facts.get(id)).collect::<Vec<_>>(),
                    "political_relations":system.political_relation_ids.iter().filter_map(|id|campaign.agency_relations.get(id)).collect::<Vec<_>>(),
                })
            });
        let scope_instruction = if let Some(target_id) = existing_destination_id.as_deref() {
            format!(
                "The primary destination already exists as exact canonical location {target_id}. Elaborate it in place. Never emit that location again, rename it, replace it, or alter its existing routes or container. Every new location must be a bounded child whose containment chain reaches {target_id}; origin_routes are new local routes owned by {target_id}."
            )
        } else {
            "The primary destination is genuinely new. Admit it and its bounded child detail from the supplied origin without rewriting any existing place.".into()
        };
        let base_prompt = format!(
            "Compile only the requested bounded destination region. {scope_instruction} Every new location id must be new. Return explicit origin_routes records owned by expansion anchor id {} into the new region or child locality, and give every such destination a reciprocal route record back to the anchor with the same positive travel time. Every route record needs a stable route_id local to its exact origin, an exact destination_id, a distance, and positive travel_minutes; the same local route_id may exist under another origin without naming the same route. Do not rewrite existing geography. Every place has a non-empty name, valid container, and concrete persistent features. Any locally observable clue must already exist as a fact and list exact discoverable_at_location_ids from the combined existing and new topology; later action assessment can reveal facts but cannot invent them.\n\nUse evidence as canon constraints, not as an exhaustive game map. Missing game-scale routes, geometry, people, ordinary procedures, supplies, local responsibilities, capacity choices, and operating doctrine require the smallest coherent playable elaboration. Mark the resulting facts branch_local or provisional_local and disclose consequential inventions in branch_assumptions. Variation between campaigns is permitted; if a detail must not vary, it belongs in the Vault. Never put a compatible elaboration in gaps merely because the Vault is silent.\n\nWhen CURRENT CIVIC APPARATUS is null, a playable inhabited destination needs one to eight non-overlapping population leaves and two to twelve distinct institutions. When it is present, preserve it and add only genuinely new detail needed by the request; population and institution arrays may be empty. New local relations may join new subjects to exact subjects in the current apparatus. Never duplicate a resident body, office, fact, or relation under a fresh name. Each new population home_location_id must be one new location. Each new institution location_ids set may name the jurisdiction or its new children. shared_fact_ids may contain exact fact IDs from the current apparatus or this candidate, never free-text knowledge. collective_authority_id may be null or the exact ID of one new population and denotes real shared authority.\n\nReturn one complete civic_system manifest for every inhabited candidate. On an existing apparatus it is the next version and must retain every existing governing institution, resident population, public fact, and political relation while adding any new IDs. It must identify the exact jurisdiction, its governing institutions and resident populations, at least one committed public fact in each of four domains—current authority, selection or succession, public resources or revenue, and redress or appeal—and the political relations that make implementation, hierarchy, or contestation legible. Every named resident population must share those public civic facts. If REQUEST presupposes a mayor, election, throne, council, or other office that this locality does not use, commit facts that let a resident correct the premise; never manufacture the requested institution merely to agree with the question. The question selects the missing domain, not its answer.\n\nThe gaps array is legal only when no compatible elaboration can preserve an exact clause of REQUEST without choosing between contradictory canon baselines, inventing an unanchored canon baseline explicitly required by the request, or exceeding an approved capability. Every gap must name that exact premise clause and the exact table choice blocking compilation. `The Vault does not specify X` is never sufficient. Use an empty gaps array when branch-local invention preserves the request.\n\nA migration relation is a directed available path for a later voluntary strategic choice; it does not move anyone, establish that admission occurred, or erase destination-community agency. It may originate only from one exact co-located active population ID supplied below and may target only one new population ID. Emit a relation only when the request and supplied source population/member goals support that migration possibility. Never invent a source population or named member. The approval preview must make all branch-local assumptions explicit without misclassifying them as canon gaps.\n\nCAMPAIGN LOCATIONS:\n{}\nCURRENT CIVIC APPARATUS:\n{}\nCO-LOCATED SOURCE POPULATIONS AND NAMED MEMBER DELTAS:\n{}\nREQUEST:\n{}\nEVIDENCE:\n{}",
            expansion_origin_location_id,
            serde_json::to_string(&campaign.locations)?,
            serde_json::to_string(&current_civic_context)?,
            serde_json::to_string(&origin_population_context)?,
            destination_request,
            evidence_text(&receipts)
        );
        let mut schema = serde_json::to_value(schema_for!(CompiledExpansionSeed))?;
        constrain_destination_expansion_schema(&mut schema, &source_population_ids)?;
        let sources = receipt_ids(&receipts);
        let mut compiler_receipts = Vec::new();
        let mut correction = String::new();
        let (seed, mut expansion) = loop {
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
            if let Err(error) = validate_compiled_material_gaps(&seed.gaps, &receipts)
                .and_then(|_| validate_branch_assumptions(&seed.branch_assumptions))
            {
                mark_semantic_invalid(
                    compiler_receipts
                        .last_mut()
                        .expect("receipt was just stored"),
                    &error,
                );
                if compiler_receipts.len() == 1 {
                    correction = format!(
                        "\n\nLOCAL VALIDATOR REJECTED THE PREVIOUS CANDIDATE: {error}\nReturn a corrected complete candidate against the same CAMPAIGN, REQUEST, and EVIDENCE. Compatible game-scale invention belongs in branch_assumptions, not gaps."
                    );
                    continue;
                }
                return Err(anyhow!(
                    "destination compiler failed local validation after one correction: {error}"
                ));
            }
            let fact_statements = campaign
                .facts
                .values()
                .chain(seed.facts.iter())
                .map(|fact| (fact.id.as_str(), fact.statement.as_str()))
                .collect::<BTreeMap<_, _>>();
            let populations = seed
                .populations
                .iter()
                .map(|population| {
                    let shared_knowledge = population
                        .shared_fact_ids
                        .iter()
                        .map(|fact_id| {
                            fact_statements.get(fact_id.as_str()).map(|value| (*value).to_owned())
                                .ok_or_else(|| anyhow!("destination population {} references unknown shared fact {fact_id}", population.id))
                        })
                        .collect::<Result<BTreeSet<_>>>()?;
                    Ok(GestaltPersonaState {
                        schema: "ghostlight.gestalt_persona_state.v1".into(),
                        id: population.id.clone(),
                        name: population.name.clone(),
                        version: 0,
                        home_location_id: population.home_location_id.clone(),
                        shared_capabilities: population.shared_capabilities.clone(),
                        shared_knowledge,
                        resources: population.resources.clone(),
                        goals: population.goals.clone(),
                        pressures: population.pressures.clone(),
                    })
                })
                .collect::<Result<Vec<_>>>()?;
            let population_profiles = seed
                .populations
                .iter()
                .map(|population| AgencyProfile {
                    schema: "ghostlight.agency_profile.v1".into(),
                    id: format!("agency:{}", population.id),
                    subject_id: population.id.clone(),
                    subject_kind: AgencySubjectKind::Gestalt,
                    profile_version: 0,
                    collective_authority_id: population.collective_authority_id.clone(),
                    parent_subject_id: None,
                    active_leaf: true,
                    simulation_eligible: true,
                    facets: population.facets.clone().into_map(),
                    location_ids: BTreeSet::from([population.home_location_id.clone()]),
                    information_channels: population.information_channels.clone(),
                    detail_debt: 0,
                    last_detail_tick: campaign.strategic_tick_count,
                    evidence_receipt_ids: sources.clone(),
                })
                .collect::<Vec<_>>();
            let institutions = seed
                .institutions
                .iter()
                .map(|institution| InstitutionState {
                    id: institution.id.clone(),
                    name: institution.name.clone(),
                    resources: institution.resources.clone(),
                    goals: institution.goals.clone(),
                    posture: institution.posture.clone(),
                })
                .collect::<Vec<_>>();
            let institution_profiles = seed
                .institutions
                .iter()
                .map(|institution| AgencyProfile {
                    schema: "ghostlight.agency_profile.v1".into(),
                    id: format!("agency:{}", institution.id),
                    subject_id: institution.id.clone(),
                    subject_kind: AgencySubjectKind::Institution,
                    profile_version: 0,
                    collective_authority_id: None,
                    parent_subject_id: None,
                    active_leaf: true,
                    simulation_eligible: true,
                    facets: institution.facets.clone().into_map(),
                    location_ids: institution.location_ids.clone(),
                    information_channels: institution.information_channels.clone(),
                    detail_debt: 0,
                    last_detail_tick: campaign.strategic_tick_count,
                    evidence_receipt_ids: sources.clone(),
                })
                .collect::<Vec<_>>();
            let local_relations = seed
                .local_relations
                .iter()
                .map(|relation| AgencyRelation {
                    schema: "ghostlight.agency_relation.v1".into(),
                    id: relation.id.clone(),
                    from_subject_id: relation.from_subject_id.clone(),
                    to_subject_id: relation.to_subject_id.clone(),
                    kind: relation.kind.clone(),
                    strength: relation.strength,
                    active: true,
                    evidence_receipt_ids: sources.clone(),
                })
                .collect::<Vec<_>>();
            let migration_relations = seed
                .migration_relations
                .iter()
                .map(|relation| AgencyRelation {
                    schema: "ghostlight.agency_relation.v1".into(),
                    id: relation.id.clone(),
                    from_subject_id: relation.from_gestalt_id.clone(),
                    to_subject_id: relation.to_gestalt_id.clone(),
                    kind: AgencyRelationKind::Migration,
                    strength: relation.strength,
                    active: true,
                    evidence_receipt_ids: sources.clone(),
                })
                .collect::<Vec<_>>();
            let mut civic_system = seed.civic_system.clone();
            if let Some(system) = &mut civic_system {
                system.version = campaign
                    .civic_systems
                    .get(&system.jurisdiction_location_id)
                    .map(|current| current.version.saturating_add(1))
                    .unwrap_or(0);
                system.semantic_verification_receipt_id.clear();
            }
            let expansion = crate::domain::RegionExpansion {
                origin_location_id: expansion_origin_location_id.into(),
                origin_routes: compiled_route_map(
                    seed.origin_routes.clone(),
                    expansion_origin_location_id,
                )?,
                locations: seed
                    .locations
                    .clone()
                    .into_iter()
                    .map(CompiledLocation::into_location)
                    .collect::<Result<Vec<_>>>()?,
                facts: seed.facts.clone(),
                populations,
                population_profiles,
                migration_relations,
                institutions,
                institution_profiles,
                local_relations,
                civic_system,
            };
            let validation =
                if !expansion.populations.is_empty() && expansion.civic_system.is_none() {
                    Err(anyhow!(
                        "compiled inhabited destination omitted its civic system manifest"
                    ))
                } else if let Some(target_location_id) = existing_destination_id.as_deref() {
                    validate_locality_elaboration(
                        campaign,
                        &LocalityElaboration {
                            target_location_id: target_location_id.into(),
                            expansion: expansion.clone(),
                        },
                    )
                } else {
                    validate_new_destination_expansion(campaign, &expansion)
                };
            match validation {
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
        if let Some(civic_system) = &expansion.civic_system {
            let verification_prompt = format!(
                "Independently verify the admitted civic apparatus. Judge meaning, not JSON shape. The public facts must actually explain current authority, selection or succession, public resources or revenue, and redress or appeal. The institutions and political relations must form a coherent local apparatus, and every resident population's exact shared_fact_ids must ground an ordinary answer about local government. A question may select a civic domain but must not have forced its presupposed office, election, or answer into the candidate. Do not rewrite or complete the candidate; return verdicts only.\n\nREQUEST:\n{}\nEVIDENCE:\n{}\nCANDIDATE:\n{}",
                destination_request,
                evidence_text(&receipts),
                serde_json::to_string(&serde_json::json!({
                    "previous_civic_apparatus":&current_civic_context,
                    "civic_system":civic_system,
                    "new_facts":&seed.facts,
                    "new_institutions":&seed.institutions,
                    "new_local_relations":&seed.local_relations,
                    "new_resident_populations":seed.populations.iter().map(|population| serde_json::json!({
                        "id":population.id,
                        "shared_fact_ids":population.shared_fact_ids,
                    })).collect::<Vec<_>>(),
                }))?,
            );
            let (value, mut verification_receipt) = self
                .structured(
                    "destination_civic_verification",
                    &snapshot,
                    &verification_prompt,
                    serde_json::to_value(schema_for!(CivicSystemVerification))?,
                    sources.clone(),
                )
                .await?;
            let verdict = serde_json::from_value::<CivicSystemVerification>(value)?;
            let rationale_chars = verdict.rationale.trim().chars().count();
            if rationale_chars == 0
                || rationale_chars > 1_000
                || !verdict.authority_legible
                || !verdict.selection_or_succession_legible
                || !verdict.public_resources_legible
                || !verdict.redress_legible
                || !verdict.institutional_relations_coherent
                || !verdict.resident_answer_grounded
            {
                let error = anyhow!(
                    "destination civic verifier rejected the candidate: {}",
                    verdict.rationale.trim()
                );
                mark_semantic_invalid(&mut verification_receipt, &error);
                return Err(error);
            }
            let candidate_digest = civic_candidate_digest(&expansion)?;
            verification_receipt
                .rebind_snapshot(civic_verifier_binding(campaign, &candidate_digest));
            expansion
                .civic_system
                .as_mut()
                .expect("verified civic system remains present")
                .semantic_verification_receipt_id = verification_receipt.storage_key().to_owned();
            compiler_receipts.push(verification_receipt);
        }
        let evidence_ids = receipt_ids(&receipts);
        let affected_sources: Vec<String> = receipts
            .iter()
            .flat_map(|r| r.witnesses.iter().map(|w| w.source_id.clone()))
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        let gap_texts = seed.gaps.iter().map(material_gap_text).collect::<Vec<_>>();
        let candidates = gap_texts
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
        let preview = if let Some(target_location_id) = existing_destination_id {
            DestinationCompilationPreview::LocalityElaboration(LocalityElaborationPreview {
                schema: "ghostlight.locality_elaboration_preview.v1".into(),
                campaign_id: campaign.id,
                expected_revision: campaign.revision,
                elaboration: LocalityElaboration {
                    target_location_id,
                    expansion,
                },
                evidence_receipts: receipts,
                branch_assumptions: seed.branch_assumptions,
                gaps: gap_texts,
                canon_candidates: candidates,
                requires_approval: true,
            })
        } else {
            DestinationCompilationPreview::RegionExpansion(crate::domain::RegionExpansionPreview {
                schema: "ghostlight.region_expansion_preview.v1".into(),
                campaign_id: campaign.id,
                expected_revision: campaign.revision,
                expansion,
                evidence_receipts: receipts,
                branch_assumptions: seed.branch_assumptions,
                gaps: gap_texts,
                canon_candidates: candidates,
                requires_approval: true,
            })
        };
        Ok((
            preview,
            identity_receipts
                .into_iter()
                .chain(std::iter::once(retrieval_receipt))
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
        self.retrieve_all_with_visibility(queries, temporal_scope, limit, false)
            .await
    }

    async fn retrieve_all_player_visible(
        &self,
        queries: &[String],
        temporal_scope: &str,
        limit: u8,
    ) -> Result<Vec<VaultEvidenceReceipt>> {
        self.retrieve_all_with_visibility(queries, temporal_scope, limit, true)
            .await
    }

    async fn retrieve_all_with_visibility(
        &self,
        queries: &[String],
        temporal_scope: &str,
        limit: u8,
        player_visible_only: bool,
    ) -> Result<Vec<VaultEvidenceReceipt>> {
        let mut receipts = Vec::new();
        for query in queries {
            let mut authority_lanes = vec![self.vault_id.clone()];
            if player_visible_only {
                authority_lanes.push("visibility.player".into());
            }
            receipts.push(
                self.vault
                    .search(&VaultQuery {
                        query: query.clone(),
                        authority_lanes,
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
            "OUTPUT JSON SCHEMA (follow exactly):\n{}\n\nPlan exactly {count} distinct source-search queries for the supplied subject. Each query must be a concise natural-language search string of 1 to 240 Unicode characters. Preserve proper nouns, era, place, institutions, mechanics, geography, and pressure when relevant. When the subject contains an approved_contract, at least one query must directly target its named canon_horizon; synthesize a bounded search query instead of copying an overlong paragraph verbatim. Do not answer the subject. SUBJECT:\n{subject}",
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
        approved_contract: Option<&CampaignContract>,
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
        let approved_contract_context = approved_contract
            .map(serde_json::to_string)
            .transpose()?
            .unwrap_or_else(|| "null".into());
        let base_prompt = format!(
            "OUTPUT JSON SCHEMA (follow exactly):\n{}\n\nClassify every supplied source exactly once for this requested custom start. When APPROVED CONTRACT is present, its premise and canon_horizon are retrieval authority: a source that directly witnesses a named canon anchor belongs in direct_seed even when the requested opening geometry is branch-local. direct_seed means the source directly supports this specific local place, era, role, goal, pressure, canon anchor, or a causal actor/institution that should actually be present. setting_background means the source supports general setting history, mechanics, geography, or institution identity, but its story-specific cast, incident, clocks, goals, and postures must not be imported into the new branch. excluded means it is merely nearby in search space. A shared place name or era alone does not make another story episode current. Keep each rationale to one short sentence.\nSTART:\n{}\nAPPROVED CONTRACT:\n{}\nSOURCES:\n{}",
            serde_json::to_string(&schema)?,
            serde_json::to_string(start)?,
            approved_contract_context,
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
                    let mut counts = BTreeMap::<String, usize>::new();
                    for item in &plan.coverage {
                        *counts.entry(item.source_id.clone()).or_default() += 1;
                    }
                    let actual = counts.keys().cloned().collect::<BTreeSet<_>>();
                    let missing = expected.difference(&actual).cloned().collect::<Vec<_>>();
                    let unexpected = actual.difference(&expected).cloned().collect::<Vec<_>>();
                    let duplicates = counts
                        .iter()
                        .filter(|(_, count)| **count > 1)
                        .map(|(source_id, count)| format!("{source_id} ({count} times)"))
                        .collect::<Vec<_>>();
                    let empty_rationales = plan
                        .coverage
                        .iter()
                        .filter(|item| item.rationale.trim().is_empty())
                        .map(|item| item.source_id.clone())
                        .collect::<Vec<_>>();
                    if !missing.is_empty()
                        || !unexpected.is_empty()
                        || !duplicates.is_empty()
                        || !empty_rationales.is_empty()
                    {
                        return Err(anyhow!(
                            "evidence classifier must cover every exact source once with a rationale; missing={missing:?}; unexpected={unexpected:?}; duplicates={duplicates:?}; empty_rationales={empty_rationales:?}"
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
                        "\n\nLOCAL VALIDATOR REJECTED THE PREVIOUS CLASSIFICATION: {error}\nPREVIOUS_REJECTED_CLASSIFICATION:\n{}\nReturn one corrected complete classification against the same START and SOURCES. Preserve valid records, add every missing literal source_id, remove unexpected or duplicate records, and supply every missing rationale.",
                        output
                            .structured
                            .as_ref()
                            .map(serde_json::to_string)
                            .transpose()?
                            .unwrap_or_else(|| "null".into())
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
        let schema = serde_json::to_value(schema_for!(ExtractedGlobalAgencyCatalog))?;
        let base_prompt = format!(
            "OUTPUT JSON SCHEMA (follow exactly):\n{}\n\nExtract evidence for the coarse remote strategic agency catalog at the requested historical horizon. This stage extracts; it does not write simulation doctrine. Include major powers and strategically distinct movements supported by the supplied witnesses. For each candidate, copy its exact displayed name and 1-3 contiguous supporting_claims verbatim from institution-specific source prose; every claim must contain 1 to 320 Unicode characters. A claim may establish the institution's existence or identity, or an explicit role, interest, method, constraint, refusal, or pressure. Exact institution-specific evidence of existence is enough to admit a major power; missing operational detail will be compiled later as branch-local doctrine. Do not use mere index links, shared headings, category descriptions, movement lists, or story-specific incidents. Do not infer current posture, territory, capability inventory, or branch facts. Report a material evidence gap only when the witnesses cannot anchor the institution or historical horizon at all, not merely because they omit game-scale doctrine, routes, or daily operations. Return no narrative analysis.\nHORIZON:\n{}\nREQUESTED PLACE (relevance only; not local authority):\n{}\nEVIDENCE:\n{}",
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
                    serde_json::from_value::<ExtractedGlobalAgencyCatalog>(value)
                        .map_err(Into::into)
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
                    let (catalog, mut doctrine_receipts) = self
                        .synthesize_global_agency_doctrine(start, catalog)
                        .await?;
                    stage_receipts.append(&mut doctrine_receipts);
                    return Ok((catalog, stage_receipts));
                }
                Err(error) if attempt == 0 => {
                    mark_semantic_invalid(&mut receipt, &error);
                    stage_receipts.push(receipt);
                    correction = format!(
                        "\n\nLOCAL VALIDATOR REJECTED THE PREVIOUS GLOBAL AGENCY CATALOG: {error}\nPREVIOUS_REJECTED_CATALOG:\n{}\nReturn one corrected complete catalog against the same HORIZON and EVIDENCE. Shorten or replace the exact invalid claim while preserving verbatim source grounding.",
                        output
                            .structured
                            .as_ref()
                            .map(serde_json::to_string)
                            .transpose()?
                            .unwrap_or_else(|| "null".into())
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

    async fn synthesize_global_agency_doctrine(
        &self,
        start: &CustomStart,
        grounded: GroundedGlobalAgencyCatalog,
    ) -> Result<(CompiledGlobalAgencyCatalog, Vec<ModelStageReceipt>)> {
        if grounded.institutions.is_empty() {
            return Ok((
                CompiledGlobalAgencyCatalog {
                    institutions: vec![],
                    gaps: grounded.gaps,
                },
                vec![],
            ));
        }
        let evidence = grounded.institutions.iter().map(|institution| {
            serde_json::json!({"name": institution.name, "supporting_claims": institution.supporting_claims})
        }).collect::<Vec<_>>();
        let synthesis_schema = serde_json::to_value(schema_for!(StrategicDoctrineCatalog))?;
        let synthesis_prompt = format!(
            "Synthesize one concise strategic_doctrine for every supplied institution. Doctrine is durable branch-local simulation state, not a claim that the Vault exhaustively specified policy. Preserve every canon anchor in the supplied exact claims, then fill missing operational detail with the smallest coherent interests, characteristic methods, constraints, or refusals needed for this institution to make meaningful strategic decisions at the requested horizon. Compatible elaboration is required and may vary between campaigns; absence from the claims is not itself a gap. Do not contradict or erase a supplied claim, merge institutions, borrow story-specific incidents, assert that an invented detail is sourced canon, or grant setting-breaking power without an anchor. Do not invent a current branch event or posture; those belong to later simulation state. Return the same names exactly once and no others. Keep each doctrine under 600 characters.\nHORIZON:\n{}\nCANON ANCHORS:\n{}",
            start.when,
            serde_json::to_string(&evidence)?
        );
        let verification_schema = serde_json::to_value(schema_for!(StrategicDoctrineVerification))?;
        let source_receipt_ids = grounded
            .institutions
            .iter()
            .flat_map(|i| i.evidence_receipt_ids.clone())
            .collect::<Vec<_>>();
        let mut stage_receipts = Vec::new();
        let mut correction = String::new();
        for attempt in 0..=1 {
            let (value, mut synthesis_receipt) = self
                .structured(
                    "global_agency_doctrine_synthesis",
                    &format!("global-agency-doctrine:{}", start.when),
                    &format!("{synthesis_prompt}{correction}"),
                    synthesis_schema.clone(),
                    source_receipt_ids.clone(),
                )
                .await?;
            let synthesized: StrategicDoctrineCatalog = serde_json::from_value(value)?;
            if let Err(error) = validate_doctrine_catalog(&grounded.institutions, &synthesized) {
                mark_semantic_invalid(&mut synthesis_receipt, &error);
                stage_receipts.push(synthesis_receipt);
                if attempt == 0 {
                    correction = format!(
                        "\n\nLOCAL VALIDATOR REJECTED THE PREVIOUS DOCTRINE CATALOG: {error}\nReturn one corrected complete catalog against the same CANON ANCHORS."
                    );
                    continue;
                }
                return Err(anyhow!(
                    "strategic doctrine synthesis failed local validation after one correction: {error}"
                ));
            }
            stage_receipts.push(synthesis_receipt);

            let verification_prompt = format!(
                "Verify each strategic doctrine as canon-constrained branch elaboration. compatible_with_canon is true when the doctrine preserves every supplied canon anchor, does not contradict or erase one, does not merge institutions or borrow a story-specific incident, and remains a plausible bounded policy at the requested horizon. The doctrine is intentionally allowed to invent missing operational interests, methods, constraints, refusals, and reasons as branch-local state. Do not reject a clause merely because the anchors are silent about it, and do not pretend an elaboration was stated by the source. Reject actual contradiction, canon erasure, identity conflation, an invented current branch event, or unanchored setting-breaking power. Return one verdict for each name exactly once.\nHORIZON:\n{}\nCANON ANCHORS:\n{}\nBRANCH DOCTRINES:\n{}",
                start.when,
                serde_json::to_string(&evidence)?,
                serde_json::to_string(&synthesized)?
            );
            let (value, mut verification_receipt) = self
                .structured(
                    "global_agency_doctrine_verification",
                    &format!("global-agency-doctrine-verification:{}", start.when),
                    &verification_prompt,
                    verification_schema.clone(),
                    source_receipt_ids.clone(),
                )
                .await?;
            let verification: StrategicDoctrineVerification = serde_json::from_value(value)?;
            match validate_doctrine_verification(&grounded.institutions, &verification) {
                Ok(incompatible) if incompatible.is_empty() => {
                    stage_receipts.push(verification_receipt);
                    let catalog = lower_compatible_doctrine_catalog(grounded, synthesized);
                    return Ok((catalog, stage_receipts));
                }
                Ok(incompatible) if attempt == 0 => {
                    let error = doctrine_incompatibility_error(&incompatible);
                    mark_semantic_invalid(&mut verification_receipt, &error);
                    stage_receipts.push(verification_receipt);
                    correction = format!(
                        "\n\nTHE COMPATIBILITY VERIFIER REJECTED THE PREVIOUS BRANCH DOCTRINES: {error}\nRewrite the complete catalog against the same CANON ANCHORS. Remove the contradiction, canon erasure, identity conflation, branch event, or unanchored setting-breaking power identified by the verifier while retaining useful compatible branch elaboration. Do not collapse back to quotation when a playable doctrine can be generated.\nPREVIOUS DOCTRINES:\n{}",
                        serde_json::to_string(&synthesized)?
                    );
                }
                Ok(incompatible) => {
                    let error = doctrine_incompatibility_error(&incompatible);
                    mark_semantic_invalid(&mut verification_receipt, &error);
                    stage_receipts.push(verification_receipt);
                    return Err(anyhow!(
                        "strategic doctrine contradicted canon after one correction: {error}"
                    ));
                }
                Err(error) if attempt == 0 => {
                    mark_semantic_invalid(&mut verification_receipt, &error);
                    stage_receipts.push(verification_receipt);
                    correction = format!(
                        "\n\nLOCAL VALIDATOR REJECTED THE PREVIOUS DOCTRINE VERIFICATION: {error}\nReturn one corrected verdict for every grounded institution exactly once against the same CANON ANCHORS and BRANCH DOCTRINES."
                    );
                }
                Err(error) => {
                    mark_semantic_invalid(&mut verification_receipt, &error);
                    return Err(anyhow!(
                        "strategic doctrine verification failed local validation after one correction: {error}"
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
                "private_relationship_actor_compile" => 4_000,
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

fn shortest_location_path(
    campaign: &Campaign,
    origin_location_id: &str,
    destination_location_id: &str,
) -> Option<(Vec<String>, u32)> {
    if !campaign.locations.contains_key(origin_location_id)
        || !campaign.locations.contains_key(destination_location_id)
    {
        return None;
    }
    let mut frontier = BinaryHeap::from([Reverse((0_u32, origin_location_id.to_owned()))]);
    let mut distances = BTreeMap::from([(origin_location_id.to_owned(), 0_u32)]);
    let mut previous = BTreeMap::<String, String>::new();
    while let Some(Reverse((elapsed, location_id))) = frontier.pop() {
        if location_id == destination_location_id {
            let mut path = vec![location_id.clone()];
            let mut cursor = location_id;
            while let Some(parent) = previous.get(&cursor) {
                path.push(parent.clone());
                cursor = parent.clone();
            }
            path.reverse();
            return Some((path, elapsed));
        }
        if distances
            .get(&location_id)
            .is_some_and(|best| elapsed > *best)
        {
            continue;
        }
        let location = campaign.locations.get(&location_id)?;
        for route in location.routes.values() {
            if !campaign.locations.contains_key(&route.destination_id) {
                continue;
            }
            let Some(next_elapsed) = elapsed.checked_add(route.travel_minutes) else {
                continue;
            };
            if distances
                .get(&route.destination_id)
                .is_some_and(|best| *best <= next_elapsed)
            {
                continue;
            }
            distances.insert(route.destination_id.clone(), next_elapsed);
            previous.insert(route.destination_id.clone(), location_id.clone());
            frontier.push(Reverse((next_elapsed, route.destination_id.clone())));
        }
    }
    None
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

fn validate_compiled_material_gaps(
    gaps: &[CompiledMaterialGap],
    receipts: &[VaultEvidenceReceipt],
) -> Result<()> {
    let known_receipt_ids = receipt_ids(receipts).into_iter().collect::<BTreeSet<_>>();
    let mut seen = BTreeSet::new();
    for gap in gaps {
        for (label, value) in [
            ("summary", gap.summary.as_str()),
            ("premise_clause", gap.premise_clause.as_str()),
            ("blocked_choice", gap.blocked_choice.as_str()),
        ] {
            validate_user_text(&format!("compiled material gap {label}"), value, 2_000)?;
        }
        if !seen.insert((
            gap.kind.clone(),
            gap.summary.trim().to_lowercase(),
            gap.premise_clause.trim().to_lowercase(),
        )) {
            return Err(anyhow!("compiled material gaps must be unique"));
        }
        let supplied_receipts = gap
            .evidence_receipt_ids
            .iter()
            .cloned()
            .collect::<BTreeSet<_>>();
        if supplied_receipts.len() != gap.evidence_receipt_ids.len()
            || !supplied_receipts.is_subset(&known_receipt_ids)
        {
            return Err(anyhow!(
                "compiled material gap cites duplicate or unknown evidence receipt IDs"
            ));
        }
        match gap.kind {
            CompiledMaterialGapKind::ContradictoryCanonBaselines
                if supplied_receipts.is_empty() =>
            {
                return Err(anyhow!(
                    "a contradictory canon-baselines gap must cite supplied evidence"
                ));
            }
            CompiledMaterialGapKind::UnanchoredRequestedBaseline
            | CompiledMaterialGapKind::ApprovedCapabilityConflict
                if !supplied_receipts.is_empty() =>
            {
                return Err(anyhow!(
                    "only contradictory canon-baselines gaps may cite Vault evidence receipts"
                ));
            }
            _ => {}
        }
    }
    Ok(())
}

fn validate_branch_assumptions(assumptions: &[String]) -> Result<()> {
    if assumptions.len() > 32 {
        return Err(anyhow!("branch assumptions may contain at most 32 entries"));
    }
    let mut seen = BTreeSet::new();
    for assumption in assumptions {
        validate_user_text("branch assumption", assumption, 2_000)?;
        if !seen.insert(assumption.trim().to_lowercase()) {
            return Err(anyhow!("branch assumptions must be unique"));
        }
    }
    Ok(())
}

fn material_gap_text(gap: &CompiledMaterialGap) -> String {
    format!(
        "{} Affected approved premise: {} Approval must decide: {}",
        gap.summary.trim(),
        gap.premise_clause.trim(),
        gap.blocked_choice.trim()
    )
}

fn approved_relationship_plan(brief: &ApprovedCampaignBrief) -> Result<ApprovedRelationshipPlan> {
    let mut character_names = BTreeMap::<String, Vec<String>>::new();
    for character in &brief.characters {
        character_names
            .entry(normalized_identity(&character.name))
            .or_default()
            .push(character.actor_id.clone());
    }
    let member_actor_ids = brief
        .member_actor_ids
        .values()
        .cloned()
        .collect::<BTreeSet<_>>();
    let mut anchors = BTreeMap::<String, RequiredRelationshipActor>::new();
    let mut anchor_norms = BTreeMap::<String, String>::new();
    let mut targets = BTreeMap::new();
    for character in &brief.characters {
        for (subject, description) in &character.relationships {
            validate_user_text("relationship subject", subject, 160)?;
            validate_user_text("relationship description", description, 1_000)?;
            let normalized = normalized_identity(subject);
            let target = if let Some(actor_id) = brief.member_actor_ids.get(subject) {
                actor_id.clone()
            } else if member_actor_ids.contains(subject) {
                subject.clone()
            } else if let Some(matches) = character_names.get(&normalized).filter(|v| v.len() == 1)
            {
                matches[0].clone()
            } else {
                let digest = format!(
                    "{:x}",
                    Sha256::digest(format!("private-relationship-actor\0{normalized}").as_bytes())
                );
                let id = format!("relationship-anchor:{}", &digest[..20]);
                if let Some(previous) = anchor_norms.insert(id.clone(), normalized.clone())
                    && previous != normalized
                {
                    return Err(anyhow!("relationship anchor ID collision"));
                }
                let anchor =
                    anchors
                        .entry(id.clone())
                        .or_insert_with(|| RequiredRelationshipActor {
                            id: id.clone(),
                            name: subject.trim().to_owned(),
                            approved_relationship_descriptions: Vec::new(),
                        });
                if !anchor
                    .approved_relationship_descriptions
                    .contains(description)
                {
                    anchor
                        .approved_relationship_descriptions
                        .push(description.clone());
                }
                id
            };
            targets.insert((character.member_id.clone(), subject.clone()), target);
        }
    }
    Ok(ApprovedRelationshipPlan {
        anchors: anchors.into_values().collect(),
        targets,
    })
}

fn validate_required_relationship_actor_inputs(
    anchors: &[RequiredRelationshipActor],
) -> Result<()> {
    if anchors.len() > 64 {
        return Err(anyhow!(
            "approved campaign brief cannot require more than 64 relationship actors"
        ));
    }
    let mut ids = BTreeSet::new();
    let mut names = BTreeSet::new();
    for anchor in anchors {
        validate_user_text("relationship actor name", &anchor.name, 160)?;
        if anchor.approved_relationship_descriptions.is_empty()
            || anchor.approved_relationship_descriptions.len() > 8
        {
            return Err(anyhow!(
                "relationship actors require one to eight approved descriptions"
            ));
        }
        for description in &anchor.approved_relationship_descriptions {
            validate_user_text("relationship description", description, 1_000)?;
        }
        if !anchor.id.starts_with("relationship-anchor:")
            || anchor.id.chars().count() > 80
            || !ids.insert(anchor.id.clone())
            || !names.insert(normalized_identity(&anchor.name))
        {
            return Err(anyhow!(
                "relationship actors require unique server-generated IDs and names"
            ));
        }
    }
    Ok(())
}

fn validate_shared_seed_excludes_locally_owned_subjects(
    seed: &CompiledSeed,
    anchors: &[RequiredRelationshipActor],
    player_names: &[String],
) -> Result<()> {
    let locally_owned_names = anchors
        .iter()
        .map(|anchor| normalized_identity(&anchor.name))
        .chain(player_names.iter().map(|name| normalized_identity(name)))
        .collect::<BTreeSet<_>>();
    let collisions = seed
        .actors
        .iter()
        .filter(|actor| locally_owned_names.contains(&normalized_identity(&actor.name)))
        .map(|actor| actor.id.clone())
        .chain(
            seed.gestalt_members
                .iter()
                .filter(|member| locally_owned_names.contains(&normalized_identity(&member.name)))
                .map(|member| crate::domain::gestalt_member_subject_id(&member.id)),
        )
        .collect::<Vec<_>>();
    if collisions.is_empty() {
        Ok(())
    } else {
        Err(anyhow!(
            "shared world candidate materialized subject IDs owned outside world-cast compilation; omit those subjects and their derived public references: {collisions:?}"
        ))
    }
}

fn materialize_private_relationship_actors(
    seed: &CompiledSeed,
    anchors: &[RequiredRelationshipActor],
    candidates: PrivateRelationshipActorSet,
) -> Result<Vec<ActorState>> {
    validate_required_relationship_actor_inputs(anchors)?;
    if candidates.actors.len() != anchors.len() {
        return Err(anyhow!(
            "private relationship actor compiler returned {} candidates for {} approved subjects",
            candidates.actors.len(),
            anchors.len()
        ));
    }
    let reserved_ids = std::iter::once(seed.player.id.as_str())
        .chain(seed.actors.iter().map(|item| item.id.as_str()))
        .chain(seed.institutions.iter().map(|item| item.id.as_str()))
        .chain(seed.gestalts.iter().map(|item| item.id.as_str()))
        .chain(seed.gestalt_members.iter().map(|item| item.id.as_str()))
        .collect::<BTreeSet<_>>();
    let allowed_relationship_subject_ids = seed
        .actors
        .iter()
        .map(|item| item.id.as_str())
        .chain(seed.institutions.iter().map(|item| item.id.as_str()))
        .chain(seed.gestalts.iter().map(|item| item.id.as_str()))
        .collect::<BTreeSet<_>>();
    let mut used_candidates = BTreeSet::new();
    let mut actors = Vec::with_capacity(anchors.len());
    for (anchor_index, anchor) in anchors.iter().enumerate() {
        if reserved_ids.contains(anchor.id.as_str()) {
            return Err(anyhow!(
                "private relationship actor identity {} collides with an existing canonical subject",
                anchor.id
            ));
        }
        let normalized_name = normalized_identity(&anchor.name);
        let matches = candidates
            .actors
            .iter()
            .enumerate()
            .filter(|(index, actor)| {
                !used_candidates.contains(index)
                    && normalized_identity(&actor.name) == normalized_name
            })
            .map(|(index, _)| index)
            .collect::<Vec<_>>();
        let index = match matches.as_slice() {
            [index] => *index,
            [] => {
                return Err(anyhow!(
                    "private actor stage omitted approved subject slot {anchor_index}"
                ));
            }
            _ => {
                return Err(anyhow!(
                    "private actor stage returned {} ambiguous candidates for approved subject slot {anchor_index}",
                    matches.len(),
                ));
            }
        };
        used_candidates.insert(index);
        let candidate = &candidates.actors[index];
        if !seed
            .locations
            .iter()
            .any(|location| location.id == candidate.location_id)
        {
            return Err(anyhow!(
                "private actor candidate slot {anchor_index} occupies unknown location {}",
                candidate.location_id
            ));
        }
        let relationships = compiled_relationship_map(candidate.relationships.clone())?;
        if relationships
            .keys()
            .any(|subject_id| !allowed_relationship_subject_ids.contains(subject_id.as_str()))
        {
            return Err(anyhow!(
                "private actor candidate slot {anchor_index} relates to a subject outside the exact public actor, institution, and population allowlist"
            ));
        }
        actors.push(ActorState {
            id: anchor.id.clone(),
            name: anchor.name.clone(),
            location_id: candidate.location_id.clone(),
            capabilities: candidate.capabilities.clone(),
            knowledge: candidate.knowledge.clone(),
            equipment: candidate.equipment.clone(),
            conditions: candidate.conditions.clone(),
            obligations: candidate.obligations.clone(),
            relationships,
            goals: candidate.goals.clone(),
            memories: candidate.memories.clone(),
        });
    }
    Ok(actors)
}

fn constrain_private_relationship_actor_schema(
    schema: &mut serde_json::Value,
    anchors: &[RequiredRelationshipActor],
    seed: &CompiledSeed,
) -> Result<()> {
    let actors = schema
        .pointer_mut("/properties/actors")
        .ok_or_else(|| anyhow!("private relationship actor schema has no actors property"))?;
    actors["minItems"] = serde_json::json!(anchors.len());
    actors["maxItems"] = serde_json::json!(anchors.len());

    let candidate = schema
        .pointer_mut("/$defs/PrivateRelationshipActorCandidate")
        .ok_or_else(|| anyhow!("private relationship actor schema has no candidate definition"))?;
    candidate["properties"]["name"] = serde_json::json!({
        "type":"string",
        "enum":anchors.iter().map(|anchor| anchor.name.clone()).collect::<Vec<_>>()
    });
    candidate["properties"]["location_id"] = serde_json::json!({
        "type":"string",
        "enum":seed.locations.iter().map(|location| location.id.clone()).collect::<Vec<_>>()
    });

    let allowed_relationship_subject_ids = seed
        .actors
        .iter()
        .map(|actor| actor.id.clone())
        .chain(
            seed.institutions
                .iter()
                .map(|institution| institution.id.clone()),
        )
        .chain(seed.gestalts.iter().map(|gestalt| gestalt.id.clone()))
        .collect::<Vec<_>>();
    if allowed_relationship_subject_ids.is_empty() {
        candidate["properties"]["relationships"] = serde_json::json!({
            "type":"array",
            "maxItems":0
        });
    } else {
        let relationship = schema
            .pointer_mut("/$defs/CompiledRelationship")
            .ok_or_else(|| {
                anyhow!("private relationship actor schema has no relationship definition")
            })?;
        relationship["properties"]["subject_id"] = serde_json::json!({
            "type":"string",
            "enum":allowed_relationship_subject_ids
        });
    }
    Ok(())
}

fn constrain_destination_expansion_schema(
    schema: &mut serde_json::Value,
    source_population_ids: &BTreeSet<String>,
) -> Result<()> {
    if source_population_ids.is_empty() {
        let relations = schema
            .pointer_mut("/properties/migration_relations")
            .ok_or_else(|| anyhow!("destination schema has no migration_relations property"))?;
        relations["maxItems"] = serde_json::json!(0);
        return Ok(());
    }
    let relation = schema
        .pointer_mut("/$defs/CompiledDestinationMigrationRelation")
        .ok_or_else(|| anyhow!("destination schema has no migration relation definition"))?;
    relation["properties"]["from_gestalt_id"] = serde_json::json!({
        "type":"string",
        "enum":source_population_ids.iter().cloned().collect::<Vec<_>>()
    });
    Ok(())
}

fn validate_required_relationship_actors(
    campaign: &Campaign,
    anchors: &[RequiredRelationshipActor],
) -> Result<()> {
    let rejected_count = anchors
        .iter()
        .filter(|anchor| {
            campaign
                .actors
                .get(&anchor.id)
                .is_none_or(|actor| actor.name != anchor.name)
        })
        .count();
    if rejected_count > 0 {
        return Err(anyhow!(
            "world seed failed private relationship actor binding for {} subject(s)",
            rejected_count
        ));
    }
    Ok(())
}

fn canonical_relationship_subject_ids(campaign: &Campaign) -> BTreeSet<String> {
    let mut targets = campaign
        .actors
        .keys()
        .chain(campaign.institutions.keys())
        .chain(campaign.gestalts.keys())
        .cloned()
        .collect::<BTreeSet<_>>();
    targets.extend(
        campaign
            .gestalt_members
            .keys()
            .map(|member_id| crate::domain::gestalt_member_subject_id(member_id)),
    );
    targets
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
        "aetheria.canon_worldbuilding"
            | "aetheria.vault_document"
            | "AetheriaLore"
            | "kalsa.public"
            | "kalsa.gm_canon"
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
                    "aetheria.canon_worldbuilding"
                        | "aetheria.vault_document"
                        | "AetheriaLore"
                        | "kalsa.public"
                        | "kalsa.gm_canon"
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
    mut catalog: ExtractedGlobalAgencyCatalog,
    receipts: &[VaultEvidenceReceipt],
) -> Result<(GroundedGlobalAgencyCatalog, Vec<String>)> {
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
    let candidate_names = catalog
        .institutions
        .iter()
        .map(|institution| normalized_identity(&institution.name))
        .collect::<Vec<_>>();
    for institution in std::mem::take(&mut catalog.institutions) {
        let mut valid_claims = Vec::new();
        let mut claim_sources = BTreeSet::new();
        for claim in institution.supporting_claims {
            let sources = matching_agency_claim_sources(&claim, &by_source)?;
            let normalized_claim = normalized_identity(&claim);
            let named_candidate_count = candidate_names
                .iter()
                .filter(|name| normalized_identity_contains(&normalized_claim, name))
                .count();
            let specific = sources
                .iter()
                .any(|source_id| source_document_names_institution(source_id, &institution.name))
                || (normalized_identity_contains(
                    &normalized_claim,
                    &normalized_identity(&institution.name),
                ) && named_candidate_count == 1);
            if !sources.is_empty() && specific {
                valid_claims.push(claim);
                claim_sources.extend(sources);
            } else {
                grounding_gaps.push(format!("{} supplied a supporting claim that was not exact institution-specific evidence", institution.name));
            }
        }
        if valid_claims.is_empty() {
            omitted_names.push(institution.name);
            continue;
        }
        let evidence_receipt_ids: Vec<String> = claim_sources
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
        if evidence_receipt_ids.is_empty() {
            return Err(anyhow!(
                "grounded remote institution has no exact evidence receipt"
            ));
        }
        admitted.push(GroundedRemoteInstitution {
            name: institution.name,
            supporting_claims: valid_claims,
            evidence_receipt_ids,
        });
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
    if !omitted_names.is_empty() {
        catalog.gaps.push(format!(
            "{} remote agency candidates were omitted because no supporting claim could be bound to institution-specific evidence; exact rejection details remain in the private model-stage receipt.",
            omitted_names.len()
        ));
    }
    Ok((
        GroundedGlobalAgencyCatalog {
            institutions: admitted,
            gaps: catalog.gaps,
        },
        grounding_gaps,
    ))
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

fn normalized_identity(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_alphanumeric() {
                character.to_ascii_lowercase()
            } else {
                ' '
            }
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn normalized_identity_contains(normalized_text: &str, normalized_name: &str) -> bool {
    !normalized_name.is_empty()
        && format!(" {normalized_text} ").contains(&format!(" {normalized_name} "))
}

fn source_document_names_institution(source_id: &str, institution_name: &str) -> bool {
    let path = source_id
        .split_once(':')
        .map_or(source_id, |(_, path)| path);
    let file_name = path.rsplit(['/', '\\']).next().unwrap_or(path);
    let stem = file_name
        .rsplit_once('.')
        .map_or(file_name, |(stem, _)| stem);
    normalized_identity(stem) == normalized_identity(institution_name)
}

fn validate_doctrine_catalog(
    grounded: &[GroundedRemoteInstitution],
    synthesized: &StrategicDoctrineCatalog,
) -> Result<()> {
    let expected = grounded
        .iter()
        .map(|i| i.name.as_str())
        .collect::<BTreeSet<_>>();
    let actual = synthesized
        .institutions
        .iter()
        .map(|i| i.name.as_str())
        .collect::<BTreeSet<_>>();
    if synthesized.institutions.len() != grounded.len() || actual != expected {
        return Err(anyhow!(
            "strategic doctrine synthesis must cover every grounded institution exactly once"
        ));
    }
    if synthesized.institutions.iter().any(|i| {
        i.strategic_doctrine.trim().is_empty() || i.strategic_doctrine.chars().count() > 600
    }) {
        return Err(anyhow!(
            "strategic doctrine must contain 1 to 600 characters"
        ));
    }
    Ok(())
}

fn validate_doctrine_verification(
    grounded: &[GroundedRemoteInstitution],
    verification: &StrategicDoctrineVerification,
) -> Result<Vec<StrategicDoctrineVerdict>> {
    let expected = grounded
        .iter()
        .map(|i| i.name.as_str())
        .collect::<BTreeSet<_>>();
    let actual = verification
        .verdicts
        .iter()
        .map(|i| i.name.as_str())
        .collect::<BTreeSet<_>>();
    if verification.verdicts.len() != grounded.len() || actual != expected {
        return Err(anyhow!(
            "strategic doctrine verification must cover every grounded institution exactly once"
        ));
    }
    if verification.verdicts.iter().any(|verdict| {
        verdict.rationale.trim().is_empty() || verdict.rationale.chars().count() > 500
    }) {
        return Err(anyhow!(
            "strategic doctrine verification rationale must contain 1 to 500 characters"
        ));
    }
    Ok(verification
        .verdicts
        .iter()
        .filter(|v| !v.compatible_with_canon)
        .cloned()
        .collect())
}

fn doctrine_incompatibility_error(incompatible: &[StrategicDoctrineVerdict]) -> anyhow::Error {
    anyhow!(
        "strategic doctrine contradicted its canon anchors: {}",
        incompatible
            .iter()
            .map(|verdict| format!("{}: {}", verdict.name, verdict.rationale))
            .collect::<Vec<_>>()
            .join("; ")
    )
}

fn lower_compatible_doctrine_catalog(
    grounded: GroundedGlobalAgencyCatalog,
    synthesized: StrategicDoctrineCatalog,
) -> CompiledGlobalAgencyCatalog {
    let doctrines = synthesized
        .institutions
        .into_iter()
        .map(|institution| (institution.name, institution.strategic_doctrine))
        .collect::<BTreeMap<_, _>>();
    let institutions = grounded
        .institutions
        .into_iter()
        .map(|institution| CompiledRemoteInstitution {
            strategic_doctrine: doctrines[&institution.name].clone(),
            name: institution.name,
            evidence_receipt_ids: institution.evidence_receipt_ids,
        })
        .collect();
    CompiledGlobalAgencyCatalog {
        institutions,
        gaps: grounded.gaps,
    }
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
    let mut branch_assumptions = Vec::new();
    for institution in catalog.institutions {
        if !known_names.insert(institution.name.to_lowercase()) {
            continue;
        }
        let digest = format!("{:x}", Sha256::digest(institution.name.as_bytes()));
        let id = format!("remote-institution:{}", &digest[..16]);
        if !known_ids.insert(id.clone()) {
            return Err(anyhow!("global agency institution ID collision"));
        }
        branch_assumptions.push(format!(
            "Campaign-local operational doctrine for {}: {}",
            institution.name, institution.strategic_doctrine
        ));
        seed.institutions.push(InstitutionState {
            id: id.clone(),
            name: institution.name,
            resources: vec![],
            goals: vec![institution.strategic_doctrine],
            posture: "No branch-local posture has been established.".into(),
        });
        remote_evidence.insert(id, institution.evidence_receipt_ids);
    }
    branch_assumptions.extend(
        catalog
            .gaps
            .into_iter()
            .map(|gap| format!("Global agency coverage limit: {gap}")),
    );
    Ok((remote_evidence, branch_assumptions))
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

pub(crate) fn civic_candidate_digest(expansion: &crate::domain::RegionExpansion) -> Result<String> {
    let mut candidate = expansion.clone();
    if let Some(system) = &mut candidate.civic_system {
        system.semantic_verification_receipt_id.clear();
    }
    Ok(format!(
        "sha256:{:x}",
        Sha256::digest(rmp_serde::to_vec_named(&candidate)?)
    ))
}

pub(crate) fn civic_verifier_binding(campaign: &Campaign, candidate_digest: &str) -> String {
    format!(
        "campaign:{}:revision:{}:destination_civic:{}",
        campaign.id, campaign.revision, candidate_digest
    )
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
    if expansion.locations.is_empty()
        || new_ids.len() != expansion.locations.len()
        || new_ids.iter().any(|id| id.trim().is_empty())
    {
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
    let new_locations = expansion
        .locations
        .iter()
        .map(|location| (location.id.as_str(), location))
        .collect::<BTreeMap<_, _>>();
    let existing_origin_route_ids = campaign
        .locations
        .get(&expansion.origin_location_id)
        .map(|location| {
            location
                .routes
                .keys()
                .map(String::as_str)
                .collect::<BTreeSet<_>>()
        })
        .unwrap_or_default();
    if expansion.origin_routes.is_empty() {
        return Err(anyhow!(
            "destination expansion has no explicit route from the origin"
        ));
    }
    for (route_id, route) in &expansion.origin_routes {
        if route_id.trim().is_empty()
            || existing_origin_route_ids.contains(route_id.as_str())
            || route.travel_minutes == 0
            || route.distance.trim().is_empty()
            || !new_ids.contains(route.destination_id.as_str())
        {
            return Err(anyhow!("destination expansion has an invalid origin route"));
        }
        let destination = new_locations
            .get(route.destination_id.as_str())
            .expect("origin route destination was admitted above");
        if !destination.routes.values().any(|reverse| {
            reverse.destination_id == expansion.origin_location_id
                && reverse.travel_minutes == route.travel_minutes
        }) {
            return Err(anyhow!(
                "destination expansion origin route lacks an exact reciprocal return route"
            ));
        }
    }
    let mut attached = false;
    for location in &expansion.locations {
        if location.name.trim().is_empty()
            || location
                .persistent_features
                .iter()
                .any(|feature| feature.trim().is_empty())
            || location
                .persistent_features
                .iter()
                .collect::<BTreeSet<_>>()
                .len()
                != location.persistent_features.len()
        {
            return Err(anyhow!(
                "destination expansion has a malformed place profile"
            ));
        }
        if let Some(container_id) = &location.container_id
            && (!known(container_id) || container_id == &location.id)
        {
            return Err(anyhow!("destination expansion has an invalid container"));
        }
        for (route_id, route) in &location.routes {
            if route_id.trim().is_empty()
                || route.travel_minutes == 0
                || route.distance.trim().is_empty()
                || !known(&route.destination_id)
                || route.destination_id == location.id
            {
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
    for location in &expansion.locations {
        let mut seen = BTreeSet::from([location.id.as_str()]);
        let mut cursor = location.container_id.as_deref();
        while let Some(container_id) = cursor {
            if !seen.insert(container_id) {
                return Err(anyhow!(
                    "destination expansion containment contains a cycle"
                ));
            }
            cursor = new_locations
                .get(container_id)
                .and_then(|container| container.container_id.as_deref())
                .or_else(|| {
                    campaign
                        .locations
                        .get(container_id)
                        .and_then(|container| container.container_id.as_deref())
                });
        }
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
            || fact
                .evidence_receipt_ids
                .iter()
                .any(|receipt_id| receipt_id.trim().is_empty())
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
    let population_ids = expansion
        .populations
        .iter()
        .map(|population| population.id.as_str())
        .collect::<BTreeSet<_>>();
    if expansion.populations.len() > 8
        || population_ids.len() != expansion.populations.len()
        || population_ids.iter().any(|id| {
            id.trim().is_empty()
                || campaign.gestalts.contains_key(*id)
                || campaign.actors.contains_key(*id)
                || campaign.institutions.contains_key(*id)
        })
    {
        return Err(anyhow!(
            "destination populations need at most eight unique new canonical subject IDs"
        ));
    }
    let profile_ids = expansion
        .population_profiles
        .iter()
        .map(|profile| profile.subject_id.as_str())
        .collect::<BTreeSet<_>>();
    if profile_ids != population_ids
        || expansion.population_profiles.len() != expansion.populations.len()
    {
        return Err(anyhow!(
            "destination population profiles must cover every new population exactly once"
        ));
    }
    let axes = BTreeSet::from([
        AgencyAxis::Geography,
        AgencyAxis::Ideology,
        AgencyAxis::Authority,
        AgencyAxis::EconomyRole,
        AgencyAxis::SpeciesBody,
        AgencyAxis::Information,
    ]);
    for population in &expansion.populations {
        if population.schema != "ghostlight.gestalt_persona_state.v1"
            || population.version != 0
            || population.name.trim().is_empty()
            || !new_ids.contains(population.home_location_id.as_str())
            || population.goals.is_empty()
            || population
                .shared_capabilities
                .iter()
                .chain(population.shared_knowledge.iter())
                .chain(population.resources.iter())
                .chain(population.goals.iter())
                .chain(population.pressures.iter())
                .any(|value| value.trim().is_empty())
            || population
                .shared_knowledge
                .iter()
                .any(|statement| !fact_statements.contains(statement))
        {
            return Err(anyhow!(
                "destination population {} has malformed or unsupported canonical state",
                population.id
            ));
        }
        let profile = expansion
            .population_profiles
            .iter()
            .find(|profile| profile.subject_id == population.id)
            .expect("profile coverage was checked above");
        let profile_axes = profile.facets.keys().cloned().collect::<BTreeSet<_>>();
        if profile.schema != "ghostlight.agency_profile.v1"
            || profile.id != format!("agency:{}", population.id)
            || profile.subject_kind != AgencySubjectKind::Gestalt
            || profile.profile_version != 0
            || profile.parent_subject_id.is_some()
            || !profile.active_leaf
            || !profile.simulation_eligible
            || profile.location_ids != BTreeSet::from([population.home_location_id.clone()])
            || profile_axes != axes
            || profile
                .collective_authority_id
                .as_ref()
                .is_some_and(|authority| !population_ids.contains(authority.as_str()))
            || profile
                .information_channels
                .iter()
                .any(|channel| !crate::resolution::information_channel_is_concrete(channel))
            || profile
                .information_channels
                .intersection(&population.shared_knowledge)
                .next()
                .is_some()
        {
            return Err(anyhow!(
                "destination population {} has a malformed agency profile",
                population.id
            ));
        }
    }
    let institution_ids = expansion
        .institutions
        .iter()
        .map(|institution| institution.id.as_str())
        .collect::<BTreeSet<_>>();
    if expansion.institutions.len() > 12
        || institution_ids.len() != expansion.institutions.len()
        || institution_ids.iter().any(|id| {
            id.trim().is_empty()
                || population_ids.contains(id)
                || campaign.gestalts.contains_key(*id)
                || campaign.actors.contains_key(*id)
                || campaign.institutions.contains_key(*id)
        })
    {
        return Err(anyhow!(
            "destination institutions need at most twelve unique new canonical subject IDs"
        ));
    }
    let institution_profile_ids = expansion
        .institution_profiles
        .iter()
        .map(|profile| profile.subject_id.as_str())
        .collect::<BTreeSet<_>>();
    if institution_profile_ids != institution_ids
        || expansion.institution_profiles.len() != expansion.institutions.len()
    {
        return Err(anyhow!(
            "destination institution profiles must cover every new institution exactly once"
        ));
    }
    for institution in &expansion.institutions {
        if institution.name.trim().is_empty()
            || institution.posture.trim().is_empty()
            || institution.posture.chars().count() > MAX_POSTURE_CHARS
            || institution.goals.is_empty()
            || institution
                .resources
                .iter()
                .chain(institution.goals.iter())
                .any(|value| value.trim().is_empty())
        {
            return Err(anyhow!(
                "destination institution {} has malformed canonical state",
                institution.id
            ));
        }
        let profile = expansion
            .institution_profiles
            .iter()
            .find(|profile| profile.subject_id == institution.id)
            .expect("institution profile coverage was checked above");
        let profile_axes = profile.facets.keys().cloned().collect::<BTreeSet<_>>();
        if profile.schema != "ghostlight.agency_profile.v1"
            || profile.id != format!("agency:{}", institution.id)
            || profile.subject_kind != AgencySubjectKind::Institution
            || profile.profile_version != 0
            || profile.collective_authority_id.is_some()
            || profile.parent_subject_id.is_some()
            || !profile.active_leaf
            || !profile.simulation_eligible
            || profile.location_ids.is_empty()
            || profile.location_ids.iter().any(|id| !known(id))
            || profile_axes != axes
            || profile
                .information_channels
                .iter()
                .any(|channel| !crate::resolution::information_channel_is_concrete(channel))
        {
            return Err(anyhow!(
                "destination institution {} has a malformed agency profile",
                institution.id
            ));
        }
    }
    let existing_civic = expansion
        .civic_system
        .as_ref()
        .and_then(|system| campaign.civic_systems.get(&system.jurisdiction_location_id));
    let existing_civic_subject_ids = existing_civic
        .into_iter()
        .flat_map(|system| {
            system
                .governing_institution_ids
                .iter()
                .chain(system.resident_population_ids.iter())
        })
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    let local_subject_ids = population_ids
        .iter()
        .copied()
        .chain(institution_ids.iter().copied())
        .chain(existing_civic_subject_ids.iter().copied())
        .collect::<BTreeSet<_>>();
    let mut local_relation_ids = BTreeSet::new();
    for relation in &expansion.local_relations {
        if expansion.local_relations.len() > 48
            || relation.schema != "ghostlight.agency_relation.v1"
            || relation.id.trim().is_empty()
            || !local_relation_ids.insert(relation.id.as_str())
            || campaign.agency_relations.contains_key(&relation.id)
            || relation.kind == AgencyRelationKind::Migration
            || !relation.active
            || relation.strength == 0
            || relation.strength > 100
            || relation.from_subject_id == relation.to_subject_id
            || !local_subject_ids.contains(relation.from_subject_id.as_str())
            || !local_subject_ids.contains(relation.to_subject_id.as_str())
        {
            return Err(anyhow!(
                "destination local relation {} is not an exact new local-subject edge",
                relation.id
            ));
        }
    }
    if let Some(civic) = expansion.civic_system.as_ref() {
        let civic_fact_ids = civic
            .public_authority_fact_ids
            .iter()
            .chain(civic.public_selection_fact_ids.iter())
            .chain(civic.public_resource_fact_ids.iter())
            .chain(civic.public_redress_fact_ids.iter())
            .collect::<BTreeSet<_>>();
        let previous = campaign.civic_systems.get(&civic.jurisdiction_location_id);
        let expected_version = previous
            .map(|system| system.version.saturating_add(1))
            .unwrap_or(0);
        let previous_institution_ids = previous
            .into_iter()
            .flat_map(|system| system.governing_institution_ids.iter())
            .collect::<BTreeSet<_>>();
        let previous_resident_ids = previous
            .into_iter()
            .flat_map(|system| system.resident_population_ids.iter())
            .collect::<BTreeSet<_>>();
        let previous_relation_ids = previous
            .into_iter()
            .flat_map(|system| system.political_relation_ids.iter())
            .collect::<BTreeSet<_>>();
        let allowed_institution_ids = institution_ids
            .iter()
            .map(|id| (*id).to_owned())
            .chain(previous_institution_ids.iter().map(|id| (*id).clone()))
            .collect::<BTreeSet<_>>();
        let expected_resident_ids = population_ids
            .iter()
            .map(|id| (*id).to_owned())
            .chain(previous_resident_ids.iter().map(|id| (*id).clone()))
            .collect::<BTreeSet<_>>();
        let allowed_relation_ids = local_relation_ids
            .iter()
            .map(|id| (*id).to_owned())
            .chain(previous_relation_ids.iter().map(|id| (*id).clone()))
            .collect::<BTreeSet<_>>();
        let allowed_fact_ids = new_fact_ids
            .iter()
            .cloned()
            .chain(previous.into_iter().flat_map(|system| {
                system
                    .public_authority_fact_ids
                    .iter()
                    .chain(system.public_selection_fact_ids.iter())
                    .chain(system.public_resource_fact_ids.iter())
                    .chain(system.public_redress_fact_ids.iter())
                    .cloned()
            }))
            .collect::<BTreeSet<_>>();
        if civic.schema != "ghostlight.civic_system_manifest.v1"
            || civic.version != expected_version
            || !known(&civic.jurisdiction_location_id)
            || allowed_institution_ids.len() < 2
            || expected_resident_ids.is_empty()
            || civic.governing_institution_ids.is_empty()
            || civic
                .governing_institution_ids
                .iter()
                .any(|id| !allowed_institution_ids.contains(id))
            || previous_institution_ids
                .iter()
                .any(|id| !civic.governing_institution_ids.contains(*id))
            || civic.resident_population_ids != expected_resident_ids
            || civic.public_authority_fact_ids.is_empty()
            || civic.public_selection_fact_ids.is_empty()
            || civic.public_resource_fact_ids.is_empty()
            || civic.public_redress_fact_ids.is_empty()
            || civic_fact_ids
                .iter()
                .any(|id| !allowed_fact_ids.contains(*id))
            || civic.political_relation_ids.is_empty()
            || civic
                .political_relation_ids
                .iter()
                .any(|id| !allowed_relation_ids.contains(id))
            || previous_relation_ids
                .iter()
                .any(|id| !civic.political_relation_ids.contains(*id))
            || previous.is_some_and(|previous| {
                !previous
                    .public_authority_fact_ids
                    .is_subset(&civic.public_authority_fact_ids)
                    || !previous
                        .public_selection_fact_ids
                        .is_subset(&civic.public_selection_fact_ids)
                    || !previous
                        .public_resource_fact_ids
                        .is_subset(&civic.public_resource_fact_ids)
                    || !previous
                        .public_redress_fact_ids
                        .is_subset(&civic.public_redress_fact_ids)
            })
        {
            return Err(anyhow!(
                "inhabited destination civic manifest does not close authority, selection, resources, redress, populations, and political relations"
            ));
        }
        let public_statements = civic_fact_ids
            .iter()
            .map(|fact_id| {
                expansion
                    .facts
                    .iter()
                    .find(|fact| fact.id.as_str() == fact_id.as_str())
                    .or_else(|| campaign.facts.get(*fact_id))
                    .map(|fact| fact.statement.as_str())
                    .ok_or_else(|| anyhow!("civic manifest references a missing public fact"))
            })
            .collect::<Result<BTreeSet<_>>>()?;
        if expansion.populations.iter().any(|population| {
            public_statements
                .iter()
                .any(|statement| !population.shared_knowledge.contains(*statement))
        }) {
            return Err(anyhow!(
                "every resident population must know the committed public civic facts"
            ));
        }
        if expansion.facts.iter().any(|fact| {
            civic_fact_ids.contains(&fact.id)
                && fact.scope == FactScope::CanonBaseline
                && fact.evidence_receipt_ids.is_empty()
        }) {
            return Err(anyhow!(
                "an invented public civic fact cannot claim canon-baseline scope without evidence"
            ));
        }
        if !civic.political_relation_ids.iter().any(|id| {
            expansion
                .local_relations
                .iter()
                .find(|relation| relation.id == *id)
                .or_else(|| campaign.agency_relations.get(id))
                .is_some_and(|relation| {
                    matches!(
                        relation.kind,
                        AgencyRelationKind::Command
                            | AgencyRelationKind::Rivalry
                            | AgencyRelationKind::Coercion
                    ) && (civic
                        .governing_institution_ids
                        .contains(&relation.from_subject_id)
                        || civic
                            .governing_institution_ids
                            .contains(&relation.to_subject_id))
                })
        }) {
            return Err(anyhow!(
                "civic system needs a command, rivalry, or coercion edge that makes political authority or contestation legible"
            ));
        }
    }
    let mut relation_ids = local_relation_ids;
    for relation in &expansion.migration_relations {
        let source_is_exact_origin_leaf = campaign
            .gestalts
            .get(&relation.from_subject_id)
            .is_some_and(|source| source.home_location_id == expansion.origin_location_id)
            && campaign
                .agency_profiles
                .get(&relation.from_subject_id)
                .is_some_and(|profile| {
                    profile.subject_kind == AgencySubjectKind::Gestalt
                        && profile.active_leaf
                        && profile.simulation_eligible
                });
        if expansion.migration_relations.len() > 32
            || relation.schema != "ghostlight.agency_relation.v1"
            || relation.id.trim().is_empty()
            || !relation_ids.insert(relation.id.as_str())
            || campaign.agency_relations.contains_key(&relation.id)
            || relation.kind != AgencyRelationKind::Migration
            || !relation.active
            || relation.strength == 0
            || relation.strength > 100
            || relation.from_subject_id == relation.to_subject_id
            || !source_is_exact_origin_leaf
            || !population_ids.contains(relation.to_subject_id.as_str())
        {
            return Err(anyhow!(
                "destination migration relation {} is not an exact origin-leaf to new-population edge",
                relation.id
            ));
        }
    }
    if !expansion.migration_relations.is_empty() {
        let mut candidate = campaign.clone();
        for location in &expansion.locations {
            candidate
                .locations
                .insert(location.id.clone(), location.clone());
        }
        candidate
            .locations
            .get_mut(&expansion.origin_location_id)
            .expect("origin was checked above")
            .routes
            .extend(expansion.origin_routes.clone());
        for population in &expansion.populations {
            candidate
                .gestalts
                .insert(population.id.clone(), population.clone());
        }
        for profile in &expansion.population_profiles {
            candidate
                .agency_profiles
                .insert(profile.subject_id.clone(), profile.clone());
        }
        for relation in &expansion.migration_relations {
            candidate
                .agency_relations
                .insert(relation.id.clone(), relation.clone());
            let destination = candidate
                .gestalts
                .get(&relation.to_subject_id)
                .expect("migration target was checked above");
            if crate::resolution::route_travel_minutes_within(
                &candidate,
                &expansion.origin_location_id,
                &destination.home_location_id,
                campaign.tick_hours.saturating_mul(60),
            )
            .is_none()
            {
                return Err(anyhow!(
                    "destination migration relation {} exceeds the strategic travel horizon",
                    relation.id
                ));
            }
        }
    }
    Ok(())
}

pub fn validate_new_destination_expansion(
    campaign: &Campaign,
    expansion: &crate::domain::RegionExpansion,
) -> Result<()> {
    validate_region_expansion(campaign, expansion)?;
    if let Some(civic) = &expansion.civic_system {
        if !expansion
            .locations
            .iter()
            .any(|location| location.id == civic.jurisdiction_location_id)
        {
            return Err(anyhow!(
                "new destination civic jurisdiction must be one newly admitted location"
            ));
        }
        validate_civic_locality_scope(campaign, expansion, &civic.jurisdiction_location_id)?;
    }
    Ok(())
}

pub fn validate_locality_elaboration(
    campaign: &Campaign,
    elaboration: &LocalityElaboration,
) -> Result<()> {
    validate_region_expansion(campaign, &elaboration.expansion)?;
    if !campaign
        .locations
        .contains_key(&elaboration.target_location_id)
        || elaboration.expansion.origin_location_id != elaboration.target_location_id
    {
        return Err(anyhow!(
            "locality elaboration must preserve one exact canonical target as its expansion anchor"
        ));
    }
    if elaboration.expansion.locations.iter().any(|location| {
        !expansion_location_is_within(
            campaign,
            &elaboration.expansion,
            &location.id,
            &elaboration.target_location_id,
        )
    }) {
        return Err(anyhow!(
            "locality elaboration may admit only child places beneath its exact target"
        ));
    }
    let civic = elaboration
        .expansion
        .civic_system
        .as_ref()
        .ok_or_else(|| anyhow!("locality elaboration requires a civic system manifest"))?;
    if civic.jurisdiction_location_id != elaboration.target_location_id {
        return Err(anyhow!(
            "locality civic jurisdiction must remain the exact canonical target"
        ));
    }
    validate_civic_locality_scope(
        campaign,
        &elaboration.expansion,
        &elaboration.target_location_id,
    )?;
    Ok(())
}

fn validate_civic_locality_scope(
    campaign: &Campaign,
    expansion: &crate::domain::RegionExpansion,
    jurisdiction_location_id: &str,
) -> Result<()> {
    if expansion.populations.iter().any(|population| {
        !expansion_location_is_within(
            campaign,
            expansion,
            &population.home_location_id,
            jurisdiction_location_id,
        )
    }) || expansion.institution_profiles.iter().any(|profile| {
        profile.location_ids.iter().any(|location_id| {
            !expansion_location_is_within(
                campaign,
                expansion,
                location_id,
                jurisdiction_location_id,
            )
        })
    }) {
        return Err(anyhow!(
            "civic populations and institutions must be located within their exact jurisdiction"
        ));
    }
    let civic = expansion
        .civic_system
        .as_ref()
        .expect("civic scope validation follows civic manifest validation");
    let public_fact_ids = civic
        .public_authority_fact_ids
        .iter()
        .chain(civic.public_selection_fact_ids.iter())
        .chain(civic.public_resource_fact_ids.iter())
        .chain(civic.public_redress_fact_ids.iter())
        .collect::<BTreeSet<_>>();
    if expansion.facts.iter().any(|fact| {
        public_fact_ids.contains(&fact.id)
            && (fact.discoverable_at_location_ids.is_empty()
                || fact.discoverable_at_location_ids.iter().any(|location_id| {
                    !expansion_location_is_within(
                        campaign,
                        expansion,
                        location_id,
                        jurisdiction_location_id,
                    )
                }))
    }) {
        return Err(anyhow!(
            "public civic facts must be discoverable only within their exact jurisdiction"
        ));
    }
    Ok(())
}

fn expansion_location_is_within(
    campaign: &Campaign,
    expansion: &crate::domain::RegionExpansion,
    location_id: &str,
    ancestor_id: &str,
) -> bool {
    let new_locations = expansion
        .locations
        .iter()
        .map(|location| (location.id.as_str(), location))
        .collect::<BTreeMap<_, _>>();
    let mut cursor = Some(location_id);
    let mut seen = BTreeSet::new();
    while let Some(id) = cursor {
        if id == ancestor_id {
            return true;
        }
        if !seen.insert(id) {
            return false;
        }
        cursor = new_locations
            .get(id)
            .and_then(|location| location.container_id.as_deref())
            .or_else(|| {
                campaign
                    .locations
                    .get(id)
                    .and_then(|location| location.container_id.as_deref())
            });
    }
    false
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
                "[receipt_id={} | source={} | authority_lane={} | locator={} | content_hash={}] {}",
                receipt_id,
                witness.source_id,
                witness.authority_lane,
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
                "[usage_lane=direct_seed | rationale={} | receipt_id={} | source={} | authority_lane={} | locator={} | content_hash={}] {}",
                use_plan.rationale,
                receipt_id,
                witness.source_id,
                witness.authority_lane,
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
    let mut ids = receipts
        .iter()
        .map(|receipt| receipt.id.clone())
        .collect::<Vec<_>>();
    deduplicate_ids(&mut ids);
    ids
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
    if supplied.len() > 8 {
        return Err(anyhow!(
            "{label} evidence may contain at most 8 receipt ids"
        ));
    }
    if unique.len() != supplied.len() {
        return Err(anyhow!("{label} evidence repeats a receipt id"));
    }
    let unknown = unique.difference(&allowed).copied().collect::<Vec<_>>();
    if !unknown.is_empty() {
        return Err(anyhow!(
            "{label} evidence names receipt ids absent from the supplied Vault evidence: {unknown:?}"
        ));
    }
    Ok(())
}

fn deduplicate_ids(ids: &mut Vec<String>) {
    let mut seen = BTreeSet::new();
    ids.retain(|id| seen.insert(id.clone()));
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

fn seed_to_campaign(mut seed: CompiledSeed, receipts: &[VaultEvidenceReceipt]) -> Result<Campaign> {
    for member in &mut seed.gestalt_members {
        member.id = crate::domain::canonical_gestalt_member_local_id(&member.id);
        if member.id.is_empty() {
            return Err(anyhow!("compiled gestalt member ID is empty"));
        }
    }
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
    let mut actors: BTreeMap<_, _> = seed
        .actors
        .into_iter()
        .map(|actor| {
            let actor = actor.into_actor()?;
            Ok((actor.id.clone(), actor))
        })
        .collect::<Result<_>>()?;
    if actors
        .insert(player_id.clone(), seed.player.into_actor()?)
        .is_some()
    {
        return Err(anyhow!("player id duplicates an NPC"));
    }
    let locations = seed
        .locations
        .into_iter()
        .map(|location| {
            let location = location.into_location()?;
            Ok((location.id.clone(), location))
        })
        .collect::<Result<_>>()?;
    let gestalt_members = seed
        .gestalt_members
        .into_iter()
        .map(|member| {
            let member = member.into_member()?;
            Ok((member.id.clone(), member))
        })
        .collect::<Result<_>>()?;
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
            let gap_text = material_gap_text(gap);
            let candidate = crate::domain::CanonCandidate {
                schema: "ghostlight.canon_candidate.v1".into(),
                id: format!("canon-candidate:{}:{}", id, index + 1),
                originating_campaign_id: id,
                gap: gap_text.clone(),
                evidence_receipt_ids: gap.evidence_receipt_ids.clone(),
                conflicts: vec![],
                proposed_wording: format!("Clarify the documented answer to: {gap_text}"),
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
        locations,
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
        civic_systems: BTreeMap::new(),
        transcript: vec![crate::domain::NarrativeTurn {
            revision: 0,
            at: now,
            speaker: "world".into(),
            text: seed.opening_narration,
            persona_response_actor_ids: BTreeSet::new(),
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
        gestalt_members,
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
        let input_facets = input.facets.into_map();
        let input_axes: BTreeSet<_> = input_facets.keys().cloned().collect();
        let unknown_locations = input
            .location_ids
            .iter()
            .filter(|id| !campaign.locations.contains_key(*id))
            .cloned()
            .collect::<Vec<_>>();
        let knowledge_channel_overlap = match input.subject_kind {
            AgencySubjectKind::Actor => campaign
                .actors
                .get(&input.subject_id)
                .map(|actor| {
                    input
                        .information_channels
                        .intersection(&actor.knowledge)
                        .cloned()
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default(),
            AgencySubjectKind::Gestalt => campaign
                .gestalts
                .get(&input.subject_id)
                .map(|gestalt| {
                    input
                        .information_channels
                        .intersection(&gestalt.shared_knowledge)
                        .cloned()
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default(),
            AgencySubjectKind::Institution => Vec::new(),
        };
        let invalid_information_channels = input
            .information_channels
            .iter()
            .filter(|channel| !crate::resolution::information_channel_is_concrete(channel))
            .cloned()
            .collect::<Vec<_>>();
        if profile.subject_kind != input.subject_kind
            || input_axes != axes
            || input.location_ids != profile.location_ids
            || !unknown_locations.is_empty()
            || !authority_known
            || !knowledge_channel_overlap.is_empty()
            || !invalid_information_channels.is_empty()
        {
            let missing_axes = axes.difference(&input_axes).cloned().collect::<Vec<_>>();
            let unexpected_axes = input_axes.difference(&axes).cloned().collect::<Vec<_>>();
            return Err(anyhow!(
                "agency profile {} malformed: expected_kind={:?}; supplied_kind={:?}; expected_location_ids={:?}; supplied_location_ids={:?}; missing_axes={missing_axes:?}; unexpected_axes={unexpected_axes:?}; unknown_locations={unknown_locations:?}; unknown_collective_authority={:?}; knowledge_channel_overlap={knowledge_channel_overlap:?}; invalid_information_channels={invalid_information_channels:?}",
                input.subject_id,
                profile.subject_kind,
                input.subject_kind,
                profile.location_ids,
                input.location_ids,
                input.collective_authority_id.filter(|_| !authority_known)
            ));
        }
        profile.collective_authority_id = input.collective_authority_id;
        profile.facets = input_facets;
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
    validate_campaign(c, true)
}

pub(crate) fn validate_campaign_runtime(c: &Campaign) -> Result<()> {
    validate_campaign(c, false)
}

fn validate_campaign(c: &Campaign, require_dematerialized_members: bool) -> Result<()> {
    if c.tick_hours == 0 {
        return Err(anyhow!("strategic tick duration must be positive"));
    }
    if !c.actors.contains_key(&c.player_actor_id)
        && !c.institutions.contains_key(&c.player_actor_id)
        && !c.gestalts.contains_key(&c.player_actor_id)
    {
        return Err(anyhow!("primary controlled subject is missing"));
    }
    crate::resolution::validate_policy(&c.resolution_policy)?;
    crate::resolution::validate_pins(c, &c.resolution_pins)?;
    if let Some(institution) = c.institutions.values().find(|institution| {
        institution.posture.trim().is_empty()
            || institution.posture.chars().count() > MAX_POSTURE_CHARS
    }) {
        return Err(anyhow!(
            "institution {} posture must contain one to {MAX_POSTURE_CHARS} characters",
            institution.id
        ));
    }
    let canonical_subjects = c
        .actors
        .keys()
        .map(|id| (id, AgencySubjectKind::Actor))
        .chain(
            c.institutions
                .keys()
                .map(|id| (id, AgencySubjectKind::Institution)),
        )
        .chain(c.gestalts.keys().map(|id| (id, AgencySubjectKind::Gestalt)));
    for (subject_id, expected_kind) in canonical_subjects {
        let Some(profile) = c.agency_profiles.get(subject_id) else {
            return Err(anyhow!(
                "campaign agency skeleton has incomplete subject coverage: {subject_id}"
            ));
        };
        if profile.subject_id != *subject_id || profile.subject_kind != expected_kind {
            return Err(anyhow!(
                "campaign agency profile does not match canonical subject {subject_id}"
            ));
        }
    }
    if let Some(profile) = c.agency_profiles.values().find(|profile| {
        profile.active_leaf
            && profile.simulation_eligible
            && !c.actors.contains_key(&profile.subject_id)
            && !c.institutions.contains_key(&profile.subject_id)
            && !c.gestalts.contains_key(&profile.subject_id)
    }) {
        return Err(anyhow!(
            "active agency profile refers to unknown canonical subject {}",
            profile.subject_id
        ));
    }
    let relationship_targets = canonical_relationship_subject_ids(c);
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
                "actor relationships must use exact declared actor, institution, gestalt, or named-member subject IDs with non-empty descriptions; rejected relationships={invalid_relationships:?}; valid target IDs={relationship_targets:?}"
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
    for (member_key, member) in &c.gestalt_members {
        if member_key != &member.id
            || member.id.is_empty()
            || crate::domain::canonical_gestalt_member_local_id(&member.id) != member.id
        {
            return Err(anyhow!(
                "gestalt member {} must use one canonical local ID without a member: prefix",
                member.id
            ));
        }
        if !c.gestalts.contains_key(&member.gestalt_id) {
            return Err(anyhow!(
                "gestalt member {} references unknown gestalt {}",
                member.id,
                member.gestalt_id
            ));
        }
        if require_dematerialized_members && member.materialized_actor_id.is_some() {
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
    validate_campaign_civic_systems(c)?;
    Ok(())
}

fn validate_campaign_civic_systems(campaign: &Campaign) -> Result<()> {
    for (jurisdiction_id, system) in &campaign.civic_systems {
        let public_fact_ids = system
            .public_authority_fact_ids
            .iter()
            .chain(system.public_selection_fact_ids.iter())
            .chain(system.public_resource_fact_ids.iter())
            .chain(system.public_redress_fact_ids.iter())
            .collect::<BTreeSet<_>>();
        if system.schema != "ghostlight.civic_system_manifest.v1"
            || system.jurisdiction_location_id != *jurisdiction_id
            || !campaign.locations.contains_key(jurisdiction_id)
            || system.semantic_verification_receipt_id.trim().is_empty()
            || system.governing_institution_ids.len() < 2
            || system
                .governing_institution_ids
                .iter()
                .any(|id| !campaign.institutions.contains_key(id))
            || system.resident_population_ids.is_empty()
            || system
                .resident_population_ids
                .iter()
                .any(|id| !campaign.gestalts.contains_key(id))
            || system.public_authority_fact_ids.is_empty()
            || system.public_selection_fact_ids.is_empty()
            || system.public_resource_fact_ids.is_empty()
            || system.public_redress_fact_ids.is_empty()
            || public_fact_ids
                .iter()
                .any(|id| !campaign.facts.contains_key(*id))
            || system.political_relation_ids.is_empty()
            || system.political_relation_ids.iter().any(|id| {
                !campaign
                    .agency_relations
                    .get(id)
                    .is_some_and(|edge| edge.active)
            })
        {
            return Err(anyhow!(
                "campaign civic system for {jurisdiction_id} has broken canonical references"
            ));
        }
        for resident_id in &system.resident_population_ids {
            let resident = &campaign.gestalts[resident_id];
            if public_fact_ids.iter().any(|fact_id| {
                !resident
                    .shared_knowledge
                    .contains(&campaign.facts[*fact_id].statement)
            }) {
                return Err(anyhow!(
                    "campaign civic resident {resident_id} lost a public civic fact"
                ));
            }
        }
    }
    Ok(())
}

fn validate_opening_playability(campaign: &Campaign) -> Result<()> {
    validate_opening_topology(campaign)?;
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

fn validate_opening_topology(campaign: &Campaign) -> Result<()> {
    if campaign.locations.len() <= 1 {
        return Ok(());
    }
    let player_location = campaign.actors[&campaign.player_actor_id]
        .location_id
        .as_str();
    let reachable = |start: &str, reverse: bool| {
        let mut visited = BTreeSet::new();
        let mut pending = vec![start.to_owned()];
        while let Some(current) = pending.pop() {
            if !visited.insert(current.clone()) {
                continue;
            }
            if reverse {
                for (origin_id, location) in &campaign.locations {
                    if location
                        .routes
                        .values()
                        .any(|route| route.destination_id == current)
                    {
                        pending.push(origin_id.clone());
                    }
                }
            } else if let Some(location) = campaign.locations.get(&current) {
                pending.extend(
                    location
                        .routes
                        .values()
                        .map(|route| route.destination_id.clone()),
                );
            }
        }
        visited
    };
    let outward = reachable(player_location, false);
    let unreachable = campaign
        .locations
        .keys()
        .filter(|id| !outward.contains(*id))
        .cloned()
        .collect::<Vec<_>>();
    if !unreachable.is_empty() {
        return Err(anyhow!(
            "opening topology has locations unreachable from player location {player_location}: {unreachable:?}; containment does not create implicit movement"
        ));
    }
    let returning = reachable(player_location, true);
    let trapping = campaign
        .locations
        .keys()
        .filter(|id| !returning.contains(*id))
        .cloned()
        .collect::<Vec<_>>();
    if !trapping.is_empty() {
        return Err(anyhow!(
            "opening topology has locations with no route chain back to player location {player_location}: {trapping:?}"
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compiler_responses_schemas_preserve_dynamic_semantics_as_records() {
        let mut schema = serde_json::to_value(schema_for!(CompiledSeed)).unwrap();
        crate::model_connector::project_strict_responses_schema(&mut schema).unwrap();

        assert_eq!(
            schema["$defs"]["CompiledLocation"]["properties"]["routes"]["type"],
            "array"
        );
        assert_eq!(
            schema["$defs"]["CompiledActorState"]["properties"]["relationships"]["type"],
            "array"
        );
        assert_eq!(
            schema["$defs"]["CompiledGestaltMemberDelta"]["properties"]["relationships"]["type"],
            "array"
        );
        let serialized = serde_json::to_string(&schema).unwrap();
        assert!(serialized.contains("\"route_id\""));
        assert!(serialized.contains("\"subject_id\""));

        let mut agency = serde_json::to_value(schema_for!(CompiledAgencySkeleton)).unwrap();
        crate::model_connector::project_strict_responses_schema(&mut agency).unwrap();
        assert_eq!(
            agency["$defs"]["CompiledAgencyFacets"]["properties"]["geography"]["type"],
            "array"
        );
        assert_eq!(
            agency["$defs"]["CompiledAgencyFacets"]["properties"]["information"]["type"],
            "array"
        );

        let mut expansion = serde_json::to_value(schema_for!(CompiledExpansionSeed)).unwrap();
        crate::model_connector::project_strict_responses_schema(&mut expansion).unwrap();
        assert_eq!(expansion["properties"]["origin_routes"]["type"], "array");
        assert_eq!(expansion["properties"]["locations"]["type"], "array");

        let mut fission = serde_json::to_value(schema_for!(CompiledFissionSeed)).unwrap();
        crate::model_connector::project_strict_responses_schema(&mut fission).unwrap();
        assert_eq!(
            fission["properties"]["child_partition_values"]["type"],
            "array"
        );
        assert!(
            fission["properties"]["member_child_assignments"]["anyOf"]
                .as_array()
                .unwrap()
                .iter()
                .any(|variant| variant["type"] == "array")
        );
        assert_eq!(
            fission["properties"]["resource_child_assignments"]["type"],
            "array"
        );
    }

    #[test]
    fn compiler_boundary_rejects_duplicate_route_and_relationship_ids() {
        let location = CompiledLocation {
            id: "yard".into(),
            name: "Yard".into(),
            container_id: None,
            routes: vec![
                CompiledRoute {
                    route_id: "gate".into(),
                    destination_id: "yard".into(),
                    distance: "near".into(),
                    travel_minutes: 1,
                },
                CompiledRoute {
                    route_id: "gate".into(),
                    destination_id: "yard".into(),
                    distance: "near".into(),
                    travel_minutes: 1,
                },
            ],
            persistent_features: vec![],
        };
        assert!(location.into_location().is_err());
        assert!(
            compiled_relationship_map(vec![
                CompiledRelationship {
                    subject_id: "workers".into(),
                    description: "trusts".into(),
                },
                CompiledRelationship {
                    subject_id: "workers".into(),
                    description: "owes".into(),
                },
            ])
            .is_err()
        );
    }

    #[test]
    fn party_identity_bound_covers_long_approved_names_without_truncation() {
        let name = "N".repeat(144);
        let premise = "P".repeat(308);
        let identity = format!(
            "A cooperative party whose public starting identities are: {name} — {premise}. Private histories, secrets, and individual knowledge are deliberately withheld from world generation."
        );
        assert_eq!(identity.chars().count(), 616);
        assert!(validate_user_text("player identity", &identity, 500).is_err());
        validate_user_text("player identity", &identity, MAX_PARTY_IDENTITY_CHARS).unwrap();
        assert!(
            validate_user_text(
                "player identity",
                &"x".repeat(MAX_PARTY_IDENTITY_CHARS + 1),
                MAX_PARTY_IDENTITY_CHARS,
            )
            .is_err()
        );
    }

    #[test]
    fn agency_compiler_cannot_disguise_private_knowledge_as_a_channel() {
        let mut campaign = crate::resolution::tests::campaign(0, 1);
        campaign
            .actors
            .get_mut("player")
            .unwrap()
            .knowledge
            .insert("convoy vulnerabilities".into());
        let facets = BTreeMap::from([
            (AgencyAxis::Geography, BTreeSet::from(["center".into()])),
            (AgencyAxis::Ideology, BTreeSet::from(["unknown".into()])),
            (AgencyAxis::Authority, BTreeSet::from(["unknown".into()])),
            (AgencyAxis::EconomyRole, BTreeSet::from(["unknown".into()])),
            (AgencyAxis::SpeciesBody, BTreeSet::from(["unknown".into()])),
            (
                AgencyAxis::Information,
                BTreeSet::from(["convoy vulnerabilities".into()]),
            ),
        ]);
        let error = apply_compiled_agency_skeleton(
            &mut campaign,
            &BTreeSet::from(["player".into()]),
            vec![CompiledAgencyProfile {
                subject_id: "player".into(),
                subject_kind: AgencySubjectKind::Actor,
                collective_authority_id: None,
                facets: facets.clone().into(),
                location_ids: BTreeSet::from(["center".into()]),
                information_channels: BTreeSet::from(["convoy vulnerabilities".into()]),
            }],
            vec![],
        )
        .unwrap_err();
        assert!(error.to_string().contains("knowledge_channel_overlap"));

        campaign.actors.get_mut("player").unwrap().knowledge.clear();
        let error = apply_compiled_agency_skeleton(
            &mut campaign,
            &BTreeSet::from(["player".into()]),
            vec![CompiledAgencyProfile {
                subject_id: "player".into(),
                subject_kind: AgencySubjectKind::Actor,
                collective_authority_id: None,
                facets: facets.into(),
                location_ids: BTreeSet::from(["center".into()]),
                information_channels: BTreeSet::from(["unknown".into()]),
            }],
            vec![],
        )
        .unwrap_err();
        assert!(error.to_string().contains("invalid_information_channels"));
    }

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

    use crate::{
        domain::SourceWitness,
        model::ModelPort,
        session_zero::{CampaignContract, CharacterDraft},
        vault::FixtureVault,
    };
    use async_trait::async_trait;
    use sha2::Digest;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

    struct CompilerModel {
        invalid_route: bool,
    }

    struct DestinationElaborationModel {
        saw_branch_assumption_boundary: AtomicBool,
        reject_civic_verification: bool,
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

    struct CorrectionAwareDoctrineModel {
        synthesis_calls: AtomicUsize,
        saw_branch_elaboration_boundary: AtomicBool,
        saw_verifier_correction: AtomicBool,
        stay_incompatible: bool,
    }

    struct PrivateBoundaryCompilerModel {
        retrieval_stage_received_approved_contract: AtomicBool,
        shared_stage_was_private_free: AtomicBool,
        shared_stage_received_operational_playability: AtomicBool,
        shared_stage_received_approved_contract: AtomicBool,
        evidence_stage_received_approved_contract: AtomicBool,
        private_stage_was_minimal: AtomicBool,
    }

    struct PlayerOwnershipCompilerModel {
        world_calls: AtomicUsize,
        saw_player_ownership_boundary: AtomicBool,
        saw_exact_collision_correction: AtomicBool,
    }

    #[async_trait]
    impl ModelPort for PlayerOwnershipCompilerModel {
        async fn run(&self, request: &ModelStageRequest) -> Result<String> {
            let output = CompilerModel {
                invalid_route: false,
            }
            .run(request)
            .await?;
            if request.stage != "world_compile" {
                return Ok(output);
            }
            self.saw_player_ownership_boundary.store(
                request
                    .lived_stream
                    .contains("HUMAN-CONTROLLED PLAYER NAMES")
                    && request.lived_stream.contains("\"Sable\"")
                    && request
                        .lived_stream
                        .contains("Do not emit them in actors or gestalt_members"),
                Ordering::SeqCst,
            );
            let call = self.world_calls.fetch_add(1, Ordering::SeqCst);
            if call == 0 {
                let mut candidate: serde_json::Value = serde_json::from_str(&output)?;
                candidate["actors"] = serde_json::json!([{
                    "id":"actor_sable",
                    "name":"Sable",
                    "location_id":"yard",
                    "capabilities":[],
                    "knowledge":[],
                    "equipment":[],
                    "conditions":[],
                    "obligations":[],
                    "relationships":[],
                    "goals":[],
                    "memories":[]
                }]);
                return Ok(candidate.to_string());
            }
            self.saw_exact_collision_correction.store(
                request
                    .lived_stream
                    .contains("LOCAL VALIDATOR REJECTED THE PREVIOUS CANDIDATE")
                    && request.lived_stream.contains("actor_sable")
                    && request
                        .lived_stream
                        .contains("owned outside world-cast compilation"),
                Ordering::SeqCst,
            );
            Ok(output)
        }

        fn provider(&self) -> &'static str {
            "player-ownership-fixture"
        }
    }

    #[async_trait]
    impl ModelPort for PrivateBoundaryCompilerModel {
        async fn run(&self, request: &ModelStageRequest) -> Result<String> {
            match request.stage.as_str() {
                "custom_retrieval_plan" => {
                    self.retrieval_stage_received_approved_contract.store(
                        request.lived_stream.contains("approved_contract")
                            && request
                                .lived_stream
                                .contains("A convoy has reached a strained logistics yard")
                            && request.lived_stream.contains("canon_horizon")
                            && request.lived_stream.contains("fixture")
                            && request.lived_stream.contains(
                                "synthesize a bounded search query instead of copying an overlong paragraph verbatim",
                            ),
                        Ordering::SeqCst,
                    );
                }
                "evidence_relevance" => {
                    self.evidence_stage_received_approved_contract.store(
                        request.lived_stream.contains("APPROVED CONTRACT")
                            && request
                                .lived_stream
                                .contains("A convoy has reached a strained logistics yard")
                            && request.lived_stream.contains("canon_horizon")
                            && request.lived_stream.contains("fixture"),
                        Ordering::SeqCst,
                    );
                }
                "world_compile" => {
                    self.shared_stage_was_private_free.store(
                        !request.lived_stream.contains("convoy quartermaster")
                            && !request.lived_stream.contains("relationship-anchor:")
                            && !request.lived_stream.contains("life-debt")
                            && !request.lived_stream.contains("SECRET_COOLANT_RECORD")
                            && !request.lived_stream.contains("PRIVATE_ROUTE_CODE"),
                        Ordering::SeqCst,
                    );
                    self.shared_stage_received_operational_playability.store(
                        request
                            .lived_stream
                            .contains("PRIVATE OPERATIONAL PLAYABILITY INPUT")
                            && request.lived_stream.contains("route planning")
                            && request.lived_stream.contains("keep the convoy supplied")
                            && request.lived_stream.contains(
                                "A generic restatement of the overall crisis is not sufficient",
                            ),
                        Ordering::SeqCst,
                    );
                    self.shared_stage_received_approved_contract.store(
                        request
                            .lived_stream
                            .contains("APPROVED SESSION ZERO CONTRACT")
                            && request
                                .lived_stream
                                .contains("A convoy has reached a strained logistics yard")
                            && request.lived_stream.contains("The convoy needs supplies"),
                        Ordering::SeqCst,
                    );
                }
                "private_relationship_actor_compile" => {
                    self.private_stage_was_minimal.store(
                        request.lived_stream.contains("convoy quartermaster")
                            && !request.lived_stream.contains("relationship-anchor:")
                            && request.lived_stream.contains("life-debt")
                            && !request.lived_stream.contains("SECRET_COOLANT_RECORD")
                            && !request.lived_stream.contains("PRIVATE_ROUTE_CODE"),
                        Ordering::SeqCst,
                    );
                    return Ok(serde_json::json!({
                        "actors":[{
                            "name":"convoy quartermaster",
                            "location_id":"yard",
                            "capabilities":["convoy logistics"],
                            "knowledge":["current manifest"],
                            "equipment":["manifest terminal"],
                            "conditions":[],
                            "obligations":["account for the convoy"],
                            "relationships":[{
                                "subject_id":"yard-workers",
                                "description":"convoy clerk coordinating with the yard workers"
                            }],
                            "goals":["supply the convoy"],
                            "memories":[]
                        }]
                    })
                    .to_string());
                }
                "agency_compile" => {
                    let roster = request
                        .lived_stream
                        .split_once("subject roster:\n")
                        .and_then(|(_, tail)| tail.split_once("\n\nReturn exactly"))
                        .map(|(json, _)| json)
                        .ok_or_else(|| anyhow!("agency prompt lost its exact subject roster"))?;
                    let briefs: Vec<serde_json::Value> = serde_json::from_str(roster)?;
                    let profiles = briefs
                        .into_iter()
                        .map(|brief| {
                            let subject_id = brief["subject_id"].clone();
                            let subject_kind = brief["subject_kind"].clone();
                            serde_json::json!({
                                "subject_id":subject_id,
                                "subject_kind":subject_kind,
                                "collective_authority_id":if subject_kind == "gestalt" { subject_id } else { serde_json::Value::Null },
                                "facets":{
                                    "geography":["unknown"],
                                    "ideology":["unknown"],
                                    "authority":["unknown"],
                                    "economy_role":["unknown"],
                                    "species_body":["unknown"],
                                    "information":["unknown"]
                                },
                                "location_ids":brief["location_ids"],
                                "information_channels":[]
                            })
                        })
                        .collect::<Vec<_>>();
                    return Ok(serde_json::json!({
                        "agency_profiles":profiles,
                        "agency_relations":[]
                    })
                    .to_string());
                }
                _ => {}
            }
            CompilerModel {
                invalid_route: false,
            }
            .run(request)
            .await
        }

        fn provider(&self) -> &'static str {
            "private-boundary-fixture"
        }
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
    impl ModelPort for CorrectionAwareDoctrineModel {
        async fn run(&self, request: &ModelStageRequest) -> Result<String> {
            match request.stage.as_str() {
                "global_agency_doctrine_synthesis" => {
                    self.saw_branch_elaboration_boundary.store(
                        request.lived_stream.contains(
                            "Compatible elaboration is required and may vary between campaigns",
                        ),
                        Ordering::SeqCst,
                    );
                    let call = self.synthesis_calls.fetch_add(1, Ordering::SeqCst);
                    if call == 0 {
                        return Ok(serde_json::json!({"institutions":[{
                            "name":"Fixture Council",
                            "strategic_doctrine":"Abandon the shared route and destroy every crossing."
                        }]}).to_string());
                    }
                    self.saw_verifier_correction.store(
                        request.lived_stream.contains(
                            "THE COMPATIBILITY VERIFIER REJECTED THE PREVIOUS BRANCH DOCTRINES",
                        ) && request.lived_stream.contains("Abandon the shared route")
                            && request
                                .lived_stream
                                .contains("retaining useful compatible branch elaboration"),
                        Ordering::SeqCst,
                    );
                    let doctrine = if self.stay_incompatible {
                        "Abandon the shared route and destroy every crossing."
                    } else {
                        "Maintain the shared route through rotating repair crews, convoy permits, and temporary rationing during breakdowns."
                    };
                    Ok(serde_json::json!({"institutions":[{
                        "name":"Fixture Council",
                        "strategic_doctrine":doctrine
                    }]})
                    .to_string())
                }
                "global_agency_doctrine_verification" => {
                    let rejected = request.lived_stream.contains("Abandon the shared route");
                    Ok(serde_json::json!({"verdicts":[{
                        "name":"Fixture Council",
                        "compatible_with_canon":!rejected,
                        "rationale":if rejected {
                            "Abandoning the route contradicts the supplied maintenance anchor."
                        } else {
                            "The route remains maintained; operational methods are compatible branch elaboration."
                        }
                    }]})
                    .to_string())
                }
                _ => Err(anyhow!("unexpected doctrine correction stage")),
            }
        }

        fn provider(&self) -> &'static str {
            "correction-aware-doctrine-fixture"
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
                candidate["locations"][0]["routes"][0]["destination_id"] =
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
                    && request.lived_stream.contains("\"id\":\"yard\"")
                    && request
                        .lived_stream
                        .contains("explicit bidirectional spanning route tree")
                    && request
                        .lived_stream
                        .contains("container_id is geometry only"),
                Ordering::SeqCst,
            );
            Ok(output)
        }

        fn provider(&self) -> &'static str {
            "correction-aware-compiler-fixture"
        }
    }

    #[async_trait]
    impl ModelPort for DestinationElaborationModel {
        async fn run(&self, request: &ModelStageRequest) -> Result<String> {
            match request.stage.as_str() {
                "destination_identity_resolution" => Ok(serde_json::json!({
                    "decision":"new",
                    "existing_location_id":null,
                    "rationale":"The request asks for a new storm refuge."
                })
                .to_string()),
                "destination_retrieval_plan" => Ok(serde_json::json!({
                    "queries":["fixture storm refuge","fixture relief route"]
                })
                .to_string()),
                "destination_compile" => {
                    self.saw_branch_assumption_boundary.store(
                        request
                            .lived_stream
                            .contains("Use evidence as canon constraints, not as an exhaustive game map")
                            && request.lived_stream.contains(
                                "Never put a compatible elaboration in gaps merely because the Vault is silent",
                            )
                            && request
                                .lived_stream
                                .contains("if a detail must not vary, it belongs in the Vault"),
                        Ordering::SeqCst,
                    );
                    Ok(serde_json::json!({
                        "origin_routes":[{
                            "route_id":"route:yard_to_refuge",
                            "destination_id":"refuge",
                            "distance":"up the marked storm path",
                            "travel_minutes":15
                        }],
                        "locations":[{
                            "id":"refuge",
                            "name":"Storm Refuge",
                            "container_id":null,
                            "routes":[{
                                "route_id":"route:refuge_to_yard",
                                "destination_id":"convoy-staging",
                                "distance":"down the marked storm path",
                                "travel_minutes":15
                            }],
                            "persistent_features":["braced roof","witnessed stores ledger"]
                        }],
                        "facts":[
                            {
                                "id":"fact:refuge_authority",
                                "statement":"The refuge duty council currently authorizes admissions and storm closure.",
                                "scope":"branch_local",
                                "evidence_receipt_ids":[],
                                "discoverable_at_location_ids":["refuge"]
                            },
                            {
                                "id":"fact:refuge_selection",
                                "statement":"Wardens select two duty councillors by witnessed lot after each storm season.",
                                "scope":"branch_local",
                                "evidence_receipt_ids":[],
                                "discoverable_at_location_ids":["refuge"]
                            },
                            {
                                "id":"fact:refuge_resources",
                                "statement":"The stores office receives repair boards through disclosed convoy levies.",
                                "scope":"branch_local",
                                "evidence_receipt_ids":[],
                                "discoverable_at_location_ids":["refuge"]
                            },
                            {
                                "id":"fact:refuge_redress",
                                "statement":"A warden may appeal a duty ruling before the next witnessed stores count.",
                                "scope":"branch_local",
                                "evidence_receipt_ids":[],
                                "discoverable_at_location_ids":["refuge"]
                            }
                        ],
                        "populations":[{
                            "id":"refuge-wardens",
                            "name":"Refuge wardens",
                            "home_location_id":"refuge",
                            "shared_capabilities":["brace ordinary storm damage"],
                            "shared_fact_ids":["fact:refuge_authority","fact:refuge_selection","fact:refuge_resources","fact:refuge_redress"],
                            "resources":["repair boards"],
                            "goals":["keep the refuge usable"],
                            "pressures":["capacity is finite"],
                            "collective_authority_id":"refuge-wardens",
                            "facets":{
                                "geography":["storm path"],
                                "ideology":["preserve consent"],
                                "authority":["witnessed duty"],
                                "economy_role":["repair labor"],
                                "species_body":["mixed households"],
                                "information":["operating leaf"]
                            },
                            "information_channels":["witnessed operating leaf"]
                        }],
                        "institutions":[
                            {
                                "id":"refuge-duty-council",
                                "name":"Refuge Duty Council",
                                "resources":["closure seal"],
                                "goals":["keep admissions within storm capacity"],
                                "posture":"admit by witnessed capacity ruling",
                                "location_ids":["refuge"],
                                "facets":{
                                    "geography":["storm refuge"],
                                    "ideology":["witnessed duty"],
                                    "authority":["admission rulings"],
                                    "economy_role":["capacity allocation"],
                                    "species_body":["mixed households"],
                                    "information":["public duty leaf"]
                                },
                                "information_channels":["public duty leaf"]
                            },
                            {
                                "id":"refuge-stores-office",
                                "name":"Refuge Stores Office",
                                "resources":["stores ledger"],
                                "goals":["preserve disclosed emergency stores"],
                                "posture":"publish every levy and count",
                                "location_ids":["refuge"],
                                "facets":{
                                    "geography":["storm refuge"],
                                    "ideology":["disclosed accounting"],
                                    "authority":["stores custody"],
                                    "economy_role":["repair supply"],
                                    "species_body":["mixed households"],
                                    "information":["witnessed stores count"]
                                },
                                "information_channels":["witnessed stores count"]
                            }
                        ],
                        "local_relations":[{
                            "id":"relation:refuge-council-stores",
                            "from_subject_id":"refuge-duty-council",
                            "to_subject_id":"refuge-stores-office",
                            "kind":"command",
                            "strength":60
                        }],
                        "civic_system":{
                            "schema":"ghostlight.civic_system_manifest.v1",
                            "jurisdiction_location_id":"refuge",
                            "governing_institution_ids":["refuge-duty-council"],
                            "resident_population_ids":["refuge-wardens"],
                            "public_authority_fact_ids":["fact:refuge_authority"],
                            "public_selection_fact_ids":["fact:refuge_selection"],
                            "public_resource_fact_ids":["fact:refuge_resources"],
                            "public_redress_fact_ids":["fact:refuge_redress"],
                            "political_relation_ids":["relation:refuge-council-stores"]
                        },
                        "migration_relations":[],
                        "branch_assumptions":[
                            "The storm-path geometry and witnessed repair procedure are compatible campaign-local elaboration."
                        ],
                        "gaps":[]
                    })
                    .to_string())
                }
                "destination_civic_verification" => {
                    let accepted = !self.reject_civic_verification;
                    Ok(serde_json::json!({
                        "authority_legible":accepted,
                        "selection_or_succession_legible":accepted,
                        "public_resources_legible":accepted,
                        "redress_legible":accepted,
                        "institutional_relations_coherent":accepted,
                        "resident_answer_grounded":accepted,
                        "rationale":if accepted {
                            "The refuge facts and institutions form a legible civic apparatus grounded in every resident population."
                        } else {
                            "The candidate's labels do not form a meaningful civic apparatus."
                        }
                    })
                    .to_string())
                }
                _ => Err(anyhow!("unexpected destination elaboration stage")),
            }
        }

        fn provider(&self) -> &'static str {
            "destination-elaboration-fixture"
        }
    }

    struct ExistingDestinationModel {
        calls: AtomicUsize,
        saw_current_civic_context: AtomicBool,
    }

    #[async_trait]
    impl ModelPort for ExistingDestinationModel {
        async fn run(&self, request: &ModelStageRequest) -> Result<String> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            match request.stage.as_str() {
                "destination_identity_resolution" => {
                    assert!(request.lived_stream.contains("Harrow Station"));
                    assert!(request.lived_stream.contains("loc:harrow_station"));
                    Ok(serde_json::json!({
                        "decision":"existing",
                        "existing_location_id":"loc:harrow_station",
                        "rationale":"The request's primary destination is the supplied Harrow Station."
                    })
                    .to_string())
                }
                "destination_retrieval_plan" => Ok(serde_json::json!({
                    "queries":["Harrow Station civic institutions","Harrow Station public offices"]
                })
                .to_string()),
                "destination_compile" => {
                    assert!(
                        request
                            .lived_stream
                            .contains("Elaborate it in place. Never emit that location again")
                    );
                    assert!(
                        request
                            .lived_stream
                            .contains("The question selects the missing domain, not its answer")
                    );
                    if !request
                        .lived_stream
                        .contains("CURRENT CIVIC APPARATUS:\nnull")
                    {
                        self.saw_current_civic_context.store(true, Ordering::SeqCst);
                        return Ok(serde_json::json!({
                            "origin_routes":[{
                                "route_id":"route:station_to_audit_chamber",
                                "destination_id":"loc:harrow_audit_chamber",
                                "distance":"behind the petitions bench",
                                "travel_minutes":4
                            }],
                            "locations":[{
                                "id":"loc:harrow_audit_chamber",
                                "name":"Harrow Audit Chamber",
                                "container_id":"loc:harrow_station",
                                "routes":[{
                                    "route_id":"route:audit_chamber_to_station",
                                    "destination_id":"loc:harrow_station",
                                    "distance":"back through the petitions bench",
                                    "travel_minutes":4
                                }],
                                "persistent_features":["sealed berth-dues duplicate ledger"]
                            }],
                            "facts":[{
                                "id":"fact:harrow_hidden_levy",
                                "statement":"The audit chamber found that Mayor Selka Vey diverted one season of published berth dues into an unvoted emergency courier fund.",
                                "scope":"branch_local",
                                "evidence_receipt_ids":[],
                                "discoverable_at_location_ids":["loc:harrow_station","loc:harrow_audit_chamber"]
                            }],
                            "populations":[],
                            "institutions":[{
                                "id":"harrow-audit-chamber",
                                "name":"Harrow Audit Chamber",
                                "resources":["duplicate berth-dues ledger"],
                                "goals":["force a public vote on the diverted dues"],
                                "posture":"prepare contempt findings against the mayoral office",
                                "location_ids":["loc:harrow_audit_chamber"],
                                "facets":{
                                    "geography":["Harrow civic quarter"],
                                    "ideology":["adversarial public accounting"],
                                    "authority":["compulsory civic audit"],
                                    "economy_role":["treasury inspection"],
                                    "species_body":["mixed civil service"],
                                    "information":["open audit findings"]
                                },
                                "information_channels":["open audit findings"]
                            }],
                            "local_relations":[{
                                "id":"relation:harrow-audit-mayor",
                                "from_subject_id":"harrow-audit-chamber",
                                "to_subject_id":"harrow-mayoral-office",
                                "kind":"rivalry",
                                "strength":81
                            }],
                            "civic_system":{
                                "schema":"ghostlight.civic_system_manifest.v1",
                                "jurisdiction_location_id":"loc:harrow_station",
                                "governing_institution_ids":["harrow-mayoral-office","harrow-ward-assembly","harrow-audit-chamber"],
                                "resident_population_ids":["harrow-residents"],
                                "public_authority_fact_ids":["fact:harrow_authority"],
                                "public_selection_fact_ids":["fact:harrow_selection"],
                                "public_resource_fact_ids":["fact:harrow_resources","fact:harrow_hidden_levy"],
                                "public_redress_fact_ids":["fact:harrow_redress"],
                                "political_relation_ids":["relation:harrow-mayor-assembly","relation:harrow-audit-mayor"]
                            },
                            "migration_relations":[],
                            "branch_assumptions":["The audit chamber and diverted-dues finding are a second bounded campaign-local elaboration of Harrow's persisted civic apparatus."],
                            "gaps":[]
                        })
                        .to_string());
                    }
                    Ok(serde_json::json!({
                        "origin_routes":[{
                            "route_id":"route:station_to_civic_quarter",
                            "destination_id":"loc:harrow_civic_quarter",
                            "distance":"through the public arcade",
                            "travel_minutes":6
                        }],
                        "locations":[{
                            "id":"loc:harrow_civic_quarter",
                            "name":"Harrow Civic Quarter",
                            "container_id":"loc:harrow_station",
                            "routes":[{
                                "route_id":"route:civic_quarter_to_station",
                                "destination_id":"loc:harrow_station",
                                "distance":"back through the public arcade",
                                "travel_minutes":6
                            }],
                            "persistent_features":["ward notice boards","sealed ballot archive"]
                        }],
                        "facts":[
                            {
                                "id":"fact:harrow_authority",
                                "statement":"Mayor Selka Vey currently holds Harrow Station's civic seal while the ward assembly controls appropriations.",
                                "scope":"branch_local",
                                "evidence_receipt_ids":[],
                                "discoverable_at_location_ids":["loc:harrow_station","loc:harrow_civic_quarter"]
                            },
                            {
                                "id":"fact:harrow_selection",
                                "statement":"Harrow residents elected Selka Vey over Oren Vale at the last five-year mayoral ballot.",
                                "scope":"branch_local",
                                "evidence_receipt_ids":[],
                                "discoverable_at_location_ids":["loc:harrow_station","loc:harrow_civic_quarter"]
                            },
                            {
                                "id":"fact:harrow_resources",
                                "statement":"The civic treasury is funded by berth dues published before each ward appropriation session.",
                                "scope":"branch_local",
                                "evidence_receipt_ids":[],
                                "discoverable_at_location_ids":["loc:harrow_station","loc:harrow_civic_quarter"]
                            },
                            {
                                "id":"fact:harrow_redress",
                                "statement":"Residents may contest a mayoral order before the ward assembly's open petitions bench.",
                                "scope":"branch_local",
                                "evidence_receipt_ids":[],
                                "discoverable_at_location_ids":["loc:harrow_station","loc:harrow_civic_quarter"]
                            }
                        ],
                        "populations":[{
                            "id":"harrow-residents",
                            "name":"Harrow residents",
                            "home_location_id":"loc:harrow_civic_quarter",
                            "shared_capabilities":["participate in ward ballots"],
                            "shared_fact_ids":["fact:harrow_authority","fact:harrow_selection","fact:harrow_resources","fact:harrow_redress"],
                            "resources":["ward meeting rooms"],
                            "goals":["keep berth dues answerable to residents"],
                            "pressures":["the mayor and assembly dispute emergency appropriations"],
                            "collective_authority_id":"harrow-residents",
                            "facets":{
                                "geography":["Harrow Station"],
                                "ideology":["ward representation"],
                                "authority":["resident franchise"],
                                "economy_role":["station households and berth workers"],
                                "species_body":["mixed residents"],
                                "information":["ward notice boards"]
                            },
                            "information_channels":["ward notice boards"]
                        }],
                        "institutions":[
                            {
                                "id":"harrow-mayoral-office",
                                "name":"Harrow Mayoral Office",
                                "resources":["civic seal","emergency clerks"],
                                "goals":["retain emergency spending discretion"],
                                "posture":"press the ward assembly for immediate appropriation",
                                "location_ids":["loc:harrow_station"],
                                "facets":{
                                    "geography":["Harrow Station"],
                                    "ideology":["executive dispatch"],
                                    "authority":["mayoral orders"],
                                    "economy_role":["emergency administration"],
                                    "species_body":["mixed civil service"],
                                    "information":["sealed civic notices"]
                                },
                                "information_channels":["sealed civic notices"]
                            },
                            {
                                "id":"harrow-ward-assembly",
                                "name":"Harrow Ward Assembly",
                                "resources":["appropriations ledger","petitions bench"],
                                "goals":["bind emergency spending to public accounts"],
                                "posture":"withhold appropriation pending an open audit",
                                "location_ids":["loc:harrow_civic_quarter"],
                                "facets":{
                                    "geography":["Harrow wards"],
                                    "ideology":["public accounting"],
                                    "authority":["appropriations and petitions"],
                                    "economy_role":["budget oversight"],
                                    "species_body":["mixed ward delegates"],
                                    "information":["open session record"]
                                },
                                "information_channels":["open session record"]
                            }
                        ],
                        "local_relations":[{
                            "id":"relation:harrow-mayor-assembly",
                            "from_subject_id":"harrow-mayoral-office",
                            "to_subject_id":"harrow-ward-assembly",
                            "kind":"rivalry",
                            "strength":72
                        }],
                        "civic_system":{
                            "schema":"ghostlight.civic_system_manifest.v1",
                            "jurisdiction_location_id":"loc:harrow_station",
                            "governing_institution_ids":["harrow-mayoral-office","harrow-ward-assembly"],
                            "resident_population_ids":["harrow-residents"],
                            "public_authority_fact_ids":["fact:harrow_authority"],
                            "public_selection_fact_ids":["fact:harrow_selection"],
                            "public_resource_fact_ids":["fact:harrow_resources"],
                            "public_redress_fact_ids":["fact:harrow_redress"],
                            "political_relation_ids":["relation:harrow-mayor-assembly"]
                        },
                        "migration_relations":[],
                        "branch_assumptions":["Harrow's mayoral ballot, ward appropriations split, officeholders, and current dispute are bounded campaign-local elaboration."],
                        "gaps":[]
                    })
                    .to_string())
                }
                "destination_civic_verification" => Ok(serde_json::json!({
                    "authority_legible":true,
                    "selection_or_succession_legible":true,
                    "public_resources_legible":true,
                    "redress_legible":true,
                    "institutional_relations_coherent":true,
                    "resident_answer_grounded":true,
                    "rationale":"Harrow's exact public facts explain its divided authority, ballot, revenue, and appeal path to every resident population."
                })
                .to_string()),
                _ => Err(anyhow!(
                    "unexpected existing destination stage {}",
                    request.stage
                )),
            }
        }

        fn provider(&self) -> &'static str {
            "existing-destination-fixture"
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
                "destination_identity_resolution" => serde_json::json!({
                    "decision":"new",
                    "existing_location_id":null,
                    "rationale":"The requested destination is not one of the supplied locations."
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
                        "supporting_claims":["The Fixture Council maintains the shared route."]
                    }],
                    "gaps":[]
                }).to_string(),
                "global_agency_doctrine_synthesis" => serde_json::json!({
                    "institutions":[{
                        "name":"Fixture Council",
                        "strategic_doctrine":"Maintain the shared route as a durable civic responsibility."
                    }]
                }).to_string(),
                "global_agency_doctrine_verification" => serde_json::json!({
                    "verdicts":[{
                        "name":"Fixture Council",
                        "compatible_with_canon":true,
                        "rationale":"The doctrine preserves the maintenance anchor."
                    }]
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
                        "player":{"id":"player","name":"Tester","location_id":"yard","capabilities":[],"knowledge":[],"equipment":[],"conditions":[],"obligations":[],"relationships":[],"goals":["learn"]},
                        "locations":[{"id":"yard","name":"Yard","container_id":null,"routes":[{"route_id":"out","destination_id":destination,"distance":"near","travel_minutes":5}],"persistent_features":["same yard"]}],
                        "actors":[],
                        "gestalts":[{"schema":"ghostlight.gestalt_persona_state.v1","id":"yard-workers","name":"Yard workers","version":0,"home_location_id":"yard","shared_capabilities":["maintain machinery"],"shared_knowledge":["yard routines"],"resources":["tool shed"],"goals":["finish the shift"],"pressures":["the gate is failing"]}],
                        "gestalt_members":[{"schema":"ghostlight.gestalt_member_delta.v1","id":"member:john","gestalt_id":"yard-workers","version":0,"name":"John the smith","capability_additions":["forge hinges"],"capability_removals":[],"knowledge_additions":[],"knowledge_removals":[],"equipment":["hammer"],"conditions":[],"obligations":[],"relationships":[],"goals":[],"memories":[],"last_location_id":"yard","materialized_actor_id":null}],
                        "institutions":[],"clocks":[{"id":"shift","label":"Shift ends","progress":0,"threshold":4,"consequence":"night"}],
                        "facts":[
                            {"id":"f","statement":"A witnessed fact","scope":"canon_baseline","evidence_receipt_ids":["fixture"]},
                            {"id":"local","statement":"The outer gate indicator is dark.","scope":"branch_local","evidence_receipt_ids":[],"discoverable_at_location_ids":["yard"]}
                        ],
                        "gaps":[{
                            "kind":"unanchored_requested_baseline",
                            "summary":"The approved premise requires a canon owner for the outer gate, but no compatible baseline is anchored.",
                            "premise_clause":"The outer gate must be owned by a canon institution.",
                            "blocked_choice":"Choose which institution canonically owns the outer gate or remove that canon-specific requirement.",
                            "evidence_receipt_ids":[]
                        }],"branch_assumptions":[],"opening_narration":"The yard persists."
                    }).to_string()
                }
                "agency_compile" => serde_json::json!({
                        "agency_profiles":[{"subject_id":"yard-workers","subject_kind":"gestalt","collective_authority_id":"yard-workers","facets":{"geography":["yard"],"ideology":["mutual aid"],"authority":["yard-workers"],"economy_role":["maintenance"],"species_body":["human"],"information":["yard routines"]},"location_ids":["yard"],"information_channels":["yard bulletin"]}],
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
        campaign.gestalts.insert(
            "clinic-staff".into(),
            GestaltPersonaState {
                schema: "ghostlight.gestalt_persona_state.v1".into(),
                id: "clinic-staff".into(),
                name: "Clinic staff".into(),
                version: 0,
                home_location_id: "center".into(),
                shared_capabilities: BTreeSet::new(),
                shared_knowledge: BTreeSet::new(),
                resources: BTreeSet::new(),
                goals: vec![],
                pressures: vec![],
            },
        );
        crate::resolution::ensure_agency_profiles(&mut campaign);
        campaign
            .actors
            .get_mut("player")
            .unwrap()
            .relationships
            .insert("faction-0000".into(), "cautious contact".into());
        campaign
            .actors
            .get_mut("player")
            .unwrap()
            .relationships
            .insert(
                "clinic-staff".into(),
                "trusted by the clinic collective".into(),
            );
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

    #[test]
    fn campaign_seed_rejects_an_oversized_institution_posture() {
        let mut campaign = crate::resolution::tests::campaign(2, 1);
        let institution = campaign.institutions.values_mut().next().unwrap();
        institution.posture = "x".repeat(MAX_POSTURE_CHARS + 1);

        let error = validate_campaign_seed(&campaign).unwrap_err();
        assert!(error.to_string().contains("one to 460 characters"));
    }

    #[test]
    fn approved_relationships_resolve_party_members_and_materialize_stable_private_anchors() {
        let mut sable = CharacterDraft {
            member_id: "member-sable".into(),
            actor_id: "actor-sable".into(),
            name: "Sable".into(),
            ..Default::default()
        };
        sable.relationships.insert(
            "convoy quartermaster".into(),
            "Sable owes them a life-debt".into(),
        );
        let mut mara = CharacterDraft {
            member_id: "member-mara".into(),
            actor_id: "actor-mara".into(),
            name: "Mara".into(),
            ..Default::default()
        };
        mara.relationships
            .insert("Sable".into(), "trusted under pressure".into());
        let brief = ApprovedCampaignBrief {
            schema: "ghostlight.approved_campaign_brief.v1".into(),
            session_zero_id: Uuid::new_v4(),
            host_member_id: "member-sable".into(),
            contract: CampaignContract::default(),
            aggregate_boundaries: vec![],
            characters: vec![sable, mara],
            member_actor_ids: BTreeMap::from([
                ("member-sable".into(), "actor-sable".into()),
                ("member-mara".into(), "actor-mara".into()),
            ]),
            shared_digest: "sha256:shared".into(),
            character_digests: BTreeMap::new(),
        };

        let first = approved_relationship_plan(&brief).unwrap();
        let second = approved_relationship_plan(&brief).unwrap();
        assert_eq!(first, second);
        assert_eq!(first.anchors.len(), 1);
        assert_eq!(first.anchors[0].name, "convoy quartermaster");
        assert!(first.anchors[0].id.starts_with("relationship-anchor:"));
        assert_eq!(
            first.targets[&("member-mara".into(), "Sable".into())],
            "actor-sable"
        );
        assert_eq!(
            first.targets[&("member-sable".into(), "convoy quartermaster".into())],
            first.anchors[0].id
        );
        assert_eq!(
            first.anchors[0].approved_relationship_descriptions,
            vec!["Sable owes them a life-debt"]
        );
    }

    #[test]
    fn relationship_anchor_validation_requires_the_exact_actor_identity() {
        let mut campaign = crate::resolution::tests::campaign(2, 1);
        let anchor = RequiredRelationshipActor {
            id: "relationship-anchor:quartermaster".into(),
            name: "convoy quartermaster".into(),
            approved_relationship_descriptions: vec!["trusted convoy contact".into()],
        };
        let mut actor = campaign.actors["player"].clone();
        actor.id = anchor.id.clone();
        actor.name = anchor.name.clone();
        campaign.actors.insert(actor.id.clone(), actor);
        validate_required_relationship_actors(&campaign, std::slice::from_ref(&anchor)).unwrap();

        campaign.actors.get_mut(&anchor.id).unwrap().name = "Somebody else".into();
        assert!(
            validate_required_relationship_actors(&campaign, &[anchor])
                .unwrap_err()
                .to_string()
                .contains("failed private relationship actor binding")
        );
    }

    #[tokio::test]
    async fn approved_private_relationship_identity_never_enters_shared_world_compilation() {
        let model = Arc::new(PrivateBoundaryCompilerModel {
            retrieval_stage_received_approved_contract: AtomicBool::new(false),
            shared_stage_was_private_free: AtomicBool::new(false),
            shared_stage_received_operational_playability: AtomicBool::new(false),
            shared_stage_received_approved_contract: AtomicBool::new(false),
            evidence_stage_received_approved_contract: AtomicBool::new(false),
            private_stage_was_minimal: AtomicBool::new(false),
        });
        let compiler = WorldCompiler::new(vault(), model.clone(), "flash", "pro");
        let mut character = CharacterDraft {
            schema: "ghostlight.character_draft.v1".into(),
            member_id: "member-sable".into(),
            actor_id: "actor-sable".into(),
            name: "Sable".into(),
            public_premise: "A logistics mediator".into(),
            capabilities: vec!["route planning".into()],
            secrets: vec!["SECRET_COOLANT_RECORD".into()],
            knowledge: vec!["PRIVATE_ROUTE_CODE".into()],
            goals: vec!["keep the convoy supplied".into()],
            ..CharacterDraft::default()
        };
        character.relationships.insert(
            "convoy quartermaster".into(),
            "Sable owes them a life-debt; they are the current convoy quartermaster and can explain the manifest but cannot allocate cargo alone".into(),
        );
        let brief = ApprovedCampaignBrief {
            schema: "ghostlight.approved_campaign_brief.v1".into(),
            session_zero_id: Uuid::new_v4(),
            host_member_id: "member-sable".into(),
            contract: CampaignContract {
                campaign_name: "Private boundary".into(),
                premise: format!(
                    "A convoy has reached a strained logistics yard. {}",
                    "Public table-approved opening detail. ".repeat(40)
                ),
                canon_horizon: "fixture".into(),
                starting_where: "yard".into(),
                starting_when: "now".into(),
                starting_pressure: "The convoy needs supplies.".into(),
                desired_goal: "Keep the convoy supplied.".into(),
                ..CampaignContract::default()
            },
            aggregate_boundaries: vec![],
            characters: vec![character],
            member_actor_ids: BTreeMap::from([("member-sable".into(), "actor-sable".into())]),
            shared_digest: "sha256:shared".into(),
            character_digests: BTreeMap::new(),
        };

        let (preview, _) = compiler.compile_approved_brief(&brief).await.unwrap();

        assert!(
            model
                .retrieval_stage_received_approved_contract
                .load(Ordering::SeqCst)
        );
        assert!(model.shared_stage_was_private_free.load(Ordering::SeqCst));
        assert!(
            model
                .shared_stage_received_operational_playability
                .load(Ordering::SeqCst)
        );
        assert!(
            model
                .shared_stage_received_approved_contract
                .load(Ordering::SeqCst)
        );
        assert!(
            model
                .evidence_stage_received_approved_contract
                .load(Ordering::SeqCst)
        );
        assert!(model.private_stage_was_minimal.load(Ordering::SeqCst));
        assert!(
            !preview.campaign.transcript[0]
                .text
                .contains("convoy quartermaster")
        );
        assert!(
            !preview
                .branch_assumptions
                .iter()
                .any(|assumption| assumption.contains("convoy quartermaster"))
        );
        let anchor = preview
            .campaign
            .actors
            .values()
            .find(|actor| actor.id.starts_with("relationship-anchor:"))
            .unwrap();
        assert_eq!(anchor.name, "convoy quartermaster");
        assert_eq!(anchor.location_id, "yard");
        assert!(anchor.capabilities.contains("convoy logistics"));
        assert!(anchor.knowledge.contains("current manifest"));
        assert_eq!(
            anchor.relationships.get("yard-workers"),
            Some(&"convoy clerk coordinating with the yard workers".into())
        );
        assert_eq!(
            preview.campaign.actors["actor-sable"]
                .relationships
                .get(&anchor.id),
            Some(&"Sable owes them a life-debt; they are the current convoy quartermaster and can explain the manifest but cannot allocate cargo alone".into())
        );
    }

    #[tokio::test]
    async fn approved_player_identity_cannot_be_materialized_as_world_cast() {
        let model = Arc::new(PlayerOwnershipCompilerModel {
            world_calls: AtomicUsize::new(0),
            saw_player_ownership_boundary: AtomicBool::new(false),
            saw_exact_collision_correction: AtomicBool::new(false),
        });
        let compiler = WorldCompiler::new(vault(), model.clone(), "flash", "pro");
        let brief = ApprovedCampaignBrief {
            schema: "ghostlight.approved_campaign_brief.v1".into(),
            session_zero_id: Uuid::new_v4(),
            host_member_id: "member-sable".into(),
            contract: CampaignContract {
                campaign_name: "Player ownership".into(),
                premise: "A convoy has reached a strained logistics yard.".into(),
                canon_horizon: "fixture".into(),
                starting_where: "yard".into(),
                starting_when: "now".into(),
                starting_pressure: "The convoy needs supplies.".into(),
                desired_goal: "Keep the convoy supplied.".into(),
                ..CampaignContract::default()
            },
            aggregate_boundaries: vec![],
            characters: vec![CharacterDraft {
                schema: "ghostlight.character_draft.v1".into(),
                member_id: "member-sable".into(),
                actor_id: "actor-sable".into(),
                name: "Sable".into(),
                public_premise: "A logistics mediator".into(),
                capabilities: vec!["route planning".into()],
                goals: vec!["keep the convoy supplied".into()],
                ..CharacterDraft::default()
            }],
            member_actor_ids: BTreeMap::from([("member-sable".into(), "actor-sable".into())]),
            shared_digest: "sha256:shared".into(),
            character_digests: BTreeMap::new(),
        };

        let (preview, _) = compiler.compile_approved_brief(&brief).await.unwrap();

        assert_eq!(model.world_calls.load(Ordering::SeqCst), 2);
        assert!(model.saw_player_ownership_boundary.load(Ordering::SeqCst));
        assert!(model.saw_exact_collision_correction.load(Ordering::SeqCst));
        assert_eq!(
            preview
                .campaign
                .actors
                .values()
                .filter(|actor| normalized_identity(&actor.name) == "sable")
                .count(),
            1
        );
        assert_eq!(preview.campaign.player_actor_id, "actor-sable");
    }

    #[test]
    fn approved_player_identity_cannot_be_materialized_as_a_gestalt_member() {
        let mut seed = private_actor_test_seed();
        seed.gestalts.push(GestaltPersonaState {
            schema: "ghostlight.gestalt_persona_state.v1".into(),
            id: "corvid-collective".into(),
            name: "Corvid collective".into(),
            version: 0,
            home_location_id: "convoy-staging".into(),
            shared_capabilities: BTreeSet::new(),
            shared_knowledge: BTreeSet::new(),
            resources: BTreeSet::new(),
            goals: vec![],
            pressures: vec![],
        });
        seed.gestalt_members.push(CompiledGestaltMemberDelta {
            schema: "ghostlight.gestalt_member_delta.v1".into(),
            id: "member-corvid-sable".into(),
            gestalt_id: "corvid-collective".into(),
            version: 0,
            name: "Sable".into(),
            capability_additions: BTreeSet::new(),
            capability_removals: BTreeSet::new(),
            knowledge_additions: BTreeSet::new(),
            knowledge_removals: BTreeSet::new(),
            equipment: BTreeSet::new(),
            conditions: BTreeSet::new(),
            obligations: BTreeSet::new(),
            relationships: vec![],
            goals: vec![],
            memories: vec![],
            last_location_id: Some("convoy-staging".into()),
            materialized_actor_id: None,
            last_relevant_revision: 0,
            relevance_lease_until_revision: 0,
        });

        let error =
            validate_shared_seed_excludes_locally_owned_subjects(&seed, &[], &["Sable".into()])
                .unwrap_err()
                .to_string();
        assert!(error.contains("member:member-corvid-sable"));
    }

    fn private_actor_test_actor(id: &str, name: &str) -> ActorState {
        ActorState {
            id: id.into(),
            name: name.into(),
            location_id: "yard".into(),
            capabilities: BTreeSet::new(),
            knowledge: BTreeSet::new(),
            equipment: BTreeSet::new(),
            conditions: BTreeSet::new(),
            obligations: BTreeSet::new(),
            relationships: BTreeMap::new(),
            goals: vec![],
            memories: vec![],
        }
    }

    fn private_actor_test_seed() -> CompiledSeed {
        CompiledSeed {
            title: "Private actor compilation".into(),
            canon_cutoff: "fixture".into(),
            world_time: Utc::now(),
            tick_hours: 6,
            player: private_actor_test_actor("player", "Sable").into(),
            locations: vec![CompiledLocation {
                id: "convoy-staging".into(),
                name: "Convoy Staging".into(),
                container_id: None,
                routes: vec![],
                persistent_features: vec!["temporary shelters".into()],
            }],
            actors: vec![],
            gestalts: vec![],
            gestalt_members: vec![],
            institutions: vec![],
            clocks: vec![],
            facts: vec![],
            gaps: vec![],
            branch_assumptions: vec![],
            opening_narration: String::new(),
        }
    }

    fn private_actor_candidate(name: &str, location_id: &str) -> PrivateRelationshipActorCandidate {
        PrivateRelationshipActorCandidate {
            name: name.into(),
            location_id: location_id.into(),
            capabilities: BTreeSet::from(["convoy logistics".into()]),
            knowledge: BTreeSet::from(["current convoy manifest".into()]),
            equipment: BTreeSet::from(["manifest terminal".into()]),
            conditions: BTreeSet::new(),
            obligations: BTreeSet::from(["account for the convoy".into()]),
            relationships: vec![],
            goals: vec!["get the convoy supplied".into()],
            memories: vec![],
        }
    }

    #[test]
    fn private_actor_schema_projects_exact_current_binding_authority() {
        let seed = private_actor_test_seed();
        let anchors = vec![RequiredRelationshipActor {
            id: "relationship-anchor:quartermaster".into(),
            name: "convoy quartermaster".into(),
            approved_relationship_descriptions: vec!["trusted convoy contact".into()],
        }];
        let mut schema = serde_json::to_value(schema_for!(PrivateRelationshipActorSet)).unwrap();
        constrain_private_relationship_actor_schema(&mut schema, &anchors, &seed).unwrap();
        let validator = jsonschema::validator_for(&schema).unwrap();
        let candidate = |name: &str, location_id: &str, relationships: serde_json::Value| {
            serde_json::json!({
                "actors":[{
                    "name":name,
                    "location_id":location_id,
                    "capabilities":[],
                    "knowledge":[],
                    "equipment":[],
                    "conditions":[],
                    "obligations":[],
                    "relationships":relationships,
                    "goals":[],
                    "memories":[]
                }]
            })
        };

        assert!(validator.is_valid(&candidate(
            "convoy quartermaster",
            "convoy-staging",
            serde_json::json!([])
        )));
        assert!(!validator.is_valid(&candidate(
            "renamed quartermaster",
            "convoy-staging",
            serde_json::json!([])
        )));
        assert!(!validator.is_valid(&candidate(
            "convoy quartermaster",
            "invented-location",
            serde_json::json!([])
        )));
        assert!(!validator.is_valid(&candidate(
            "convoy quartermaster",
            "convoy-staging",
            serde_json::json!([{
                "subject_id":"invented-office",
                "description":"invented affiliation"
            }])
        )));
    }

    #[test]
    fn private_actor_materialization_attaches_server_identity() {
        let anchor = RequiredRelationshipActor {
            id: "relationship-anchor:quartermaster".into(),
            name: "convoy quartermaster".into(),
            approved_relationship_descriptions: vec!["trusted convoy contact".into()],
        };
        let actors = materialize_private_relationship_actors(
            &private_actor_test_seed(),
            std::slice::from_ref(&anchor),
            PrivateRelationshipActorSet {
                actors: vec![private_actor_candidate(
                    "Convoy Quartermaster",
                    "convoy-staging",
                )],
            },
        )
        .unwrap();

        assert_eq!(actors.len(), 1);
        assert_eq!(actors[0].id, anchor.id);
        assert_eq!(actors[0].name, anchor.name);
        assert_eq!(actors[0].location_id, "convoy-staging");
        assert!(actors[0].capabilities.contains("convoy logistics"));
        assert!(actors[0].relationships.is_empty());
    }

    #[test]
    fn private_actor_materialization_rejects_missing_ambiguous_unknown_and_colliding_state() {
        let anchor = RequiredRelationshipActor {
            id: "relationship-anchor:quartermaster".into(),
            name: "convoy quartermaster".into(),
            approved_relationship_descriptions: vec!["trusted convoy contact".into()],
        };
        let seed = private_actor_test_seed();
        assert!(
            materialize_private_relationship_actors(
                &seed,
                std::slice::from_ref(&anchor),
                PrivateRelationshipActorSet { actors: vec![] }
            )
            .unwrap_err()
            .to_string()
            .contains("returned 0 candidates")
        );

        assert!(
            materialize_private_relationship_actors(
                &seed,
                std::slice::from_ref(&anchor),
                PrivateRelationshipActorSet {
                    actors: vec![
                        private_actor_candidate("Convoy Quartermaster", "convoy-staging"),
                        private_actor_candidate("convoy-quartermaster", "convoy-staging"),
                    ]
                }
            )
            .unwrap_err()
            .to_string()
            .contains("returned 2 candidates")
        );

        let medic = RequiredRelationshipActor {
            id: "relationship-anchor:medic".into(),
            name: "convoy medic".into(),
            approved_relationship_descriptions: vec!["known convoy medic".into()],
        };
        assert!(
            materialize_private_relationship_actors(
                &seed,
                &[anchor.clone(), medic],
                PrivateRelationshipActorSet {
                    actors: vec![
                        private_actor_candidate("Convoy Quartermaster", "convoy-staging"),
                        private_actor_candidate("convoy-quartermaster", "convoy-staging"),
                    ]
                }
            )
            .unwrap_err()
            .to_string()
            .contains("ambiguous candidates")
        );

        let mut unknown = private_actor_candidate("convoy quartermaster", "missing");
        assert!(
            materialize_private_relationship_actors(
                &seed,
                std::slice::from_ref(&anchor),
                PrivateRelationshipActorSet {
                    actors: vec![unknown.clone()]
                }
            )
            .unwrap_err()
            .to_string()
            .contains("occupies unknown location")
        );

        unknown.location_id = "convoy-staging".into();
        unknown.relationships = vec![CompiledRelationship {
            subject_id: "player".into(),
            description: "guessed player relationship".into(),
        }];
        assert!(
            materialize_private_relationship_actors(
                &seed,
                std::slice::from_ref(&anchor),
                PrivateRelationshipActorSet {
                    actors: vec![unknown.clone()]
                }
            )
            .unwrap_err()
            .to_string()
            .contains("outside the exact public actor, institution, and population allowlist")
        );

        unknown.relationships.clear();
        let mut collision = seed;
        collision.player.id.clone_from(&anchor.id);
        assert!(
            materialize_private_relationship_actors(
                &collision,
                &[anchor],
                PrivateRelationshipActorSet {
                    actors: vec![unknown]
                }
            )
            .unwrap_err()
            .to_string()
            .contains("collides with an existing canonical subject")
        );
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

    #[test]
    fn suggestion_evidence_uses_set_semantics_and_rejects_unknown_receipts_precisely() {
        let mut ids = vec![
            "receipt:one".into(),
            "receipt:one".into(),
            "receipt:two".into(),
        ];
        deduplicate_ids(&mut ids);
        assert_eq!(ids, ["receipt:one", "receipt:two"]);
        validate_suggestion_evidence(
            "opening",
            &ids,
            &["receipt:one".into(), "receipt:two".into()],
        )
        .unwrap();

        let error = validate_suggestion_evidence(
            "opening",
            &["receipt:invented".into()],
            &["receipt:one".into()],
        )
        .unwrap_err()
        .to_string();
        assert!(error.contains("absent from the supplied Vault evidence"));
        assert!(error.contains("receipt:invented"));
    }

    #[tokio::test]
    async fn doctrine_stage_rewrites_canon_contradiction_but_keeps_branch_elaboration() {
        let model = Arc::new(CorrectionAwareDoctrineModel {
            synthesis_calls: AtomicUsize::new(0),
            saw_branch_elaboration_boundary: AtomicBool::new(false),
            saw_verifier_correction: AtomicBool::new(false),
            stay_incompatible: false,
        });
        let compiler = WorldCompiler::new(vault(), model.clone(), "flash", "pro");
        let start = CustomStart {
            campaign_name: "Doctrine correction".into(),
            who: "worker".into(),
            where_: "yard".into(),
            when: "fixture".into(),
            goal: "keep the route open".into(),
        };
        let grounded = GroundedGlobalAgencyCatalog {
            institutions: vec![GroundedRemoteInstitution {
                name: "Fixture Council".into(),
                supporting_claims: vec!["The Fixture Council maintains the shared route.".into()],
                evidence_receipt_ids: vec!["receipt:fixture".into()],
            }],
            gaps: vec![],
        };

        let (catalog, receipts) = compiler
            .synthesize_global_agency_doctrine(&start, grounded)
            .await
            .unwrap();

        assert_eq!(catalog.institutions.len(), 1);
        assert_eq!(
            catalog.institutions[0].strategic_doctrine,
            "Maintain the shared route through rotating repair crews, convoy permits, and temporary rationing during breakdowns."
        );
        assert_eq!(
            receipts
                .iter()
                .map(|receipt| receipt.stage.as_str())
                .collect::<Vec<_>>(),
            vec![
                "global_agency_doctrine_synthesis",
                "global_agency_doctrine_verification",
                "global_agency_doctrine_synthesis",
                "global_agency_doctrine_verification"
            ]
        );
        assert!(receipts[1].local_validation_error.is_some());
        assert!(receipts[3].local_validation_error.is_none());
        assert!(model.saw_branch_elaboration_boundary.load(Ordering::SeqCst));
        assert!(model.saw_verifier_correction.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn doctrine_stage_aborts_instead_of_omitting_an_incompatible_institution() {
        let model = Arc::new(CorrectionAwareDoctrineModel {
            synthesis_calls: AtomicUsize::new(0),
            saw_branch_elaboration_boundary: AtomicBool::new(false),
            saw_verifier_correction: AtomicBool::new(false),
            stay_incompatible: true,
        });
        let compiler = WorldCompiler::new(vault(), model, "flash", "pro");
        let start = CustomStart {
            campaign_name: "Doctrine refusal".into(),
            who: "worker".into(),
            where_: "yard".into(),
            when: "fixture".into(),
            goal: "keep the route open".into(),
        };
        let grounded = GroundedGlobalAgencyCatalog {
            institutions: vec![GroundedRemoteInstitution {
                name: "Fixture Council".into(),
                supporting_claims: vec!["The Fixture Council maintains the shared route.".into()],
                evidence_receipt_ids: vec!["receipt:fixture".into()],
            }],
            gaps: vec![],
        };

        let error = compiler
            .synthesize_global_agency_doctrine(&start, grounded)
            .await
            .unwrap_err();
        assert!(
            error
                .to_string()
                .contains("contradicted canon after one correction")
        );
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
                "global_agency_doctrine_synthesis",
                "global_agency_doctrine_verification",
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
    async fn destination_compiler_surfaces_compatible_playability_inventions_as_branch_assumptions()
    {
        let model = Arc::new(DestinationElaborationModel {
            saw_branch_assumption_boundary: AtomicBool::new(false),
            reject_civic_verification: false,
        });
        let compiler = WorldCompiler::new(vault(), model.clone(), "flash", "pro");
        let mut seed = private_actor_test_seed();
        seed.player.location_id = "convoy-staging".into();
        seed.opening_narration = "The convoy waits in the rain.".into();
        let campaign = seed_to_campaign(seed, &[]).unwrap();

        let (preview, receipts) = compiler
            .compile_destination(
                &campaign,
                "convoy-staging",
                "a playable storm refuge with ordinary repair and admission procedure",
            )
            .await
            .unwrap();
        let DestinationCompilationPreview::RegionExpansion(preview) = preview else {
            panic!("new destination must produce a region expansion preview")
        };

        assert!(model.saw_branch_assumption_boundary.load(Ordering::SeqCst));
        assert!(preview.requires_approval);
        assert!(preview.gaps.is_empty());
        assert!(preview.canon_candidates.is_empty());
        assert_eq!(preview.branch_assumptions.len(), 1);
        assert!(preview.branch_assumptions[0].contains("campaign-local elaboration"));
        assert_eq!(preview.expansion.locations[0].id, "refuge");
        assert_eq!(preview.expansion.populations[0].id, "refuge-wardens");
        assert_eq!(preview.expansion.facts[0].scope, FactScope::BranchLocal);
        assert_eq!(
            receipts
                .iter()
                .map(|receipt| receipt.stage.as_str())
                .collect::<Vec<_>>(),
            vec![
                "destination_identity_resolution",
                "destination_retrieval_plan",
                "destination_compile",
                "destination_civic_verification"
            ]
        );
    }

    #[tokio::test]
    async fn destination_compiler_rejects_semantically_empty_civic_machinery() {
        let model = Arc::new(DestinationElaborationModel {
            saw_branch_assumption_boundary: AtomicBool::new(false),
            reject_civic_verification: true,
        });
        let compiler = WorldCompiler::new(vault(), model, "flash", "pro");
        let mut seed = private_actor_test_seed();
        seed.player.location_id = "convoy-staging".into();
        seed.opening_narration = "The convoy waits in the rain.".into();
        let campaign = seed_to_campaign(seed, &[]).unwrap();

        let error = compiler
            .compile_destination(
                &campaign,
                "convoy-staging",
                "a playable storm refuge with ordinary repair and admission procedure",
            )
            .await
            .unwrap_err();

        assert!(
            error
                .to_string()
                .contains("destination civic verifier rejected the candidate")
        );
    }

    fn harrow_campaign() -> Campaign {
        let mut seed = private_actor_test_seed();
        seed.player.location_id = "convoy-staging".into();
        seed.locations[0].routes.push(CompiledRoute {
            route_id: "route:staging_to_run".into(),
            destination_id: "loc:veyr_run".into(),
            distance: "down the drainage bypass".into(),
            travel_minutes: 35,
        });
        seed.locations.push(CompiledLocation {
            id: "loc:veyr_run".into(),
            name: "Lower Veyr Run".into(),
            container_id: None,
            routes: vec![
                CompiledRoute {
                    route_id: "route:run_to_staging".into(),
                    destination_id: "convoy-staging".into(),
                    distance: "up the drainage bypass".into(),
                    travel_minutes: 35,
                },
                CompiledRoute {
                    route_id: "route:run_to_station".into(),
                    destination_id: "loc:harrow_station".into(),
                    distance: "along the anchored ascent".into(),
                    travel_minutes: 75,
                },
            ],
            persistent_features: vec!["disturbed iron anchors".into()],
        });
        seed.locations.push(CompiledLocation {
            id: "loc:harrow_station".into(),
            name: "Harrow Station".into(),
            container_id: None,
            routes: vec![CompiledRoute {
                route_id: "route:station_to_run".into(),
                destination_id: "loc:veyr_run".into(),
                distance: "back down the anchored ascent".into(),
                travel_minutes: 75,
            }],
            persistent_features: vec!["abandoned road office".into()],
        });
        seed_to_campaign(seed, &[]).unwrap()
    }

    #[tokio::test]
    async fn destination_compiler_elaborates_a_reachable_canonical_city_in_place() {
        let model = Arc::new(ExistingDestinationModel {
            calls: AtomicUsize::new(0),
            saw_current_civic_context: AtomicBool::new(false),
        });
        let compiler = WorldCompiler::new(vault(), model.clone(), "flash", "pro");
        let campaign = harrow_campaign();

        let (preview, receipts) = compiler
            .compile_destination(
                &campaign,
                "convoy-staging",
                "Visit Harrow Station and ask a local who they voted for Mayor.",
            )
            .await
            .unwrap();
        let DestinationCompilationPreview::LocalityElaboration(preview) = preview else {
            panic!("existing destination must produce a locality elaboration preview")
        };
        let expansion = &preview.elaboration.expansion;
        assert_eq!(preview.elaboration.target_location_id, "loc:harrow_station");
        assert_eq!(expansion.origin_location_id, "loc:harrow_station");
        assert!(
            expansion
                .locations
                .iter()
                .all(|location| location.id != "loc:harrow_station")
        );
        assert_eq!(expansion.institutions.len(), 2);
        assert_eq!(expansion.civic_system.as_ref().unwrap().version, 0);
        assert!(
            !expansion
                .civic_system
                .as_ref()
                .unwrap()
                .semantic_verification_receipt_id
                .is_empty()
        );
        assert_eq!(
            expansion.local_relations[0].kind,
            AgencyRelationKind::Rivalry
        );
        let selection = &expansion
            .facts
            .iter()
            .find(|fact| fact.id == "fact:harrow_selection")
            .unwrap()
            .statement;
        assert!(selection.contains("Selka Vey"));
        assert!(selection.contains("Oren Vale"));
        assert!(
            expansion.populations[0]
                .shared_knowledge
                .contains(selection)
        );
        assert_eq!(
            receipts
                .iter()
                .map(|receipt| receipt.stage.as_str())
                .collect::<Vec<_>>(),
            vec![
                "destination_identity_resolution",
                "destination_retrieval_plan",
                "destination_compile",
                "destination_civic_verification"
            ]
        );
        assert_eq!(model.calls.load(Ordering::SeqCst), 4);
    }

    #[tokio::test]
    async fn locality_elaboration_reuses_and_deepens_its_persisted_civic_apparatus() {
        let model = Arc::new(ExistingDestinationModel {
            calls: AtomicUsize::new(0),
            saw_current_civic_context: AtomicBool::new(false),
        });
        let compiler = WorldCompiler::new(vault(), model.clone(), "flash", "pro");
        let initial = harrow_campaign();
        let (first, first_receipts) = compiler
            .compile_destination(
                &initial,
                "convoy-staging",
                "Visit Harrow Station and ask a local who they voted for Mayor.",
            )
            .await
            .unwrap();
        let DestinationCompilationPreview::LocalityElaboration(first) = first else {
            panic!("first pass must elaborate Harrow in place")
        };
        let first_evidence_queries = first
            .evidence_receipts
            .iter()
            .map(|receipt| receipt.query_hash.clone())
            .collect::<BTreeSet<_>>();
        let dir = tempfile::tempdir().unwrap();
        let store =
            crate::persistence::CampaignStore::open(dir.path().join("campaign.cc")).unwrap();
        let kernel = crate::kernel::WorldKernel::start(store);
        kernel
            .command(crate::domain::WorldCommand::CreateCampaign {
                campaign: initial,
                evidence_receipts: vec![],
                model_stage_receipts: vec![],
            })
            .await
            .unwrap();
        let first_commit = kernel
            .command(crate::domain::WorldCommand::ElaborateLocality {
                expected_revision: first.expected_revision,
                elaboration: first.elaboration,
                evidence_receipts: first.evidence_receipts,
                canon_candidates: first.canon_candidates,
                model_stage_receipts: first_receipts,
            })
            .await
            .unwrap();
        let crate::kernel::CommandResult::Committed {
            campaign: first_campaign,
            ..
        } = first_commit
        else {
            panic!("first civic pass must commit")
        };

        let (second, second_receipts) = compiler
            .compile_destination(
                &first_campaign,
                "convoy-staging",
                "Investigate the fight over Harrow's berth dues and who is hiding the ledger.",
            )
            .await
            .unwrap();
        let DestinationCompilationPreview::LocalityElaboration(second) = second else {
            panic!("second pass must deepen Harrow in place")
        };
        assert!(
            second
                .evidence_receipts
                .iter()
                .any(|receipt| first_evidence_queries.contains(&receipt.query_hash))
        );
        assert_eq!(second.elaboration.expansion.populations.len(), 0);
        assert_eq!(second.elaboration.expansion.institutions.len(), 1);
        let manifest = second.elaboration.expansion.civic_system.as_ref().unwrap();
        assert_eq!(manifest.version, 1);
        assert!(
            manifest
                .governing_institution_ids
                .contains("harrow-mayoral-office")
        );
        assert!(
            manifest
                .governing_institution_ids
                .contains("harrow-audit-chamber")
        );
        assert!(
            manifest
                .public_resource_fact_ids
                .contains("fact:harrow_resources")
        );
        assert!(
            manifest
                .public_resource_fact_ids
                .contains("fact:harrow_hidden_levy")
        );

        let second_commit = kernel
            .command(crate::domain::WorldCommand::ElaborateLocality {
                expected_revision: second.expected_revision,
                elaboration: second.elaboration,
                evidence_receipts: second.evidence_receipts,
                canon_candidates: second.canon_candidates,
                model_stage_receipts: second_receipts,
            })
            .await
            .unwrap();
        let crate::kernel::CommandResult::Committed {
            campaign: second_campaign,
            ..
        } = second_commit
        else {
            panic!("second civic pass must commit")
        };
        assert_eq!(
            second_campaign.civic_systems["loc:harrow_station"].version,
            1
        );
        assert_eq!(second_campaign.institutions.len(), 3);
        assert!(
            second_campaign.gestalts["harrow-residents"]
                .shared_knowledge
                .iter()
                .any(|fact| fact.contains("unvoted emergency courier fund"))
        );
        assert!(model.saw_current_civic_context.load(Ordering::SeqCst));
        assert_eq!(model.calls.load(Ordering::SeqCst), 8);
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
        let valid = ExtractedGlobalAgencyCatalog {
            institutions: vec![ExtractedRemoteInstitution {
                name: "Pan-Solar Consortium".into(),
                supporting_claims: vec![
                    "Pan-Solar Consortium coordinates interplanetary logistics.".into(),
                ],
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

        let invented = ExtractedGlobalAgencyCatalog {
            institutions: vec![ExtractedRemoteInstitution {
                name: "Pan-Solar Consortium".into(),
                supporting_claims: vec![
                    "Pan-Solar Consortium secretly controls every government.".into(),
                ],
            }],
            gaps: vec![],
        };
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
        let catalog = ExtractedGlobalAgencyCatalog {
            institutions: (0..33)
                .map(|index| ExtractedRemoteInstitution {
                    name: format!("Institution {index}"),
                    supporting_claims: vec![format!("Institution {index} protects route {index}.")],
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
        let schema = serde_json::to_value(schema_for!(ExtractedGlobalAgencyCatalog)).unwrap();
        assert_eq!(schema["properties"]["institutions"]["maxItems"], 64);
    }

    #[test]
    fn strategic_doctrine_requires_exact_coverage_and_classifies_canon_compatibility() {
        let grounded = vec![GroundedRemoteInstitution {
            name: "Fixture Council".into(),
            supporting_claims: vec!["The Fixture Council maintains the route.".into()],
            evidence_receipt_ids: vec!["vault:council".into()],
        }];
        let doctrine = StrategicDoctrineCatalog {
            institutions: vec![SynthesizedRemoteInstitution {
                name: "Fixture Council".into(),
                strategic_doctrine: "Maintain the route as a durable responsibility.".into(),
            }],
        };
        validate_doctrine_catalog(&grounded, &doctrine).unwrap();
        let incompatible = StrategicDoctrineVerification {
            verdicts: vec![StrategicDoctrineVerdict {
                name: "Fixture Council".into(),
                compatible_with_canon: false,
                rationale: "Abandoning the route contradicts its maintenance anchor.".into(),
            }],
        };
        let incompatible = validate_doctrine_verification(&grounded, &incompatible).unwrap();
        assert_eq!(incompatible.len(), 1);
        assert_eq!(incompatible[0].name, "Fixture Council");

        let compatible = StrategicDoctrineVerification {
            verdicts: vec![StrategicDoctrineVerdict {
                name: "Fixture Council".into(),
                compatible_with_canon: true,
                rationale: "Convoy permits are compatible branch-local operating detail.".into(),
            }],
        };
        assert!(
            validate_doctrine_verification(&grounded, &compatible)
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn compatible_remote_doctrines_retain_every_institution_and_surface_branch_assumptions() {
        let grounded = GroundedGlobalAgencyCatalog {
            institutions: vec![
                GroundedRemoteInstitution {
                    name: "Fixture Council".into(),
                    supporting_claims: vec!["The Fixture Council maintains the route.".into()],
                    evidence_receipt_ids: vec!["vault:council".into()],
                },
                GroundedRemoteInstitution {
                    name: "Witness Guild".into(),
                    supporting_claims: vec!["The Witness Guild records public crossings.".into()],
                    evidence_receipt_ids: vec!["vault:guild".into()],
                },
            ],
            gaps: vec!["The exact horizon remains uncertain.".into()],
        };
        let synthesized = StrategicDoctrineCatalog {
            institutions: vec![
                SynthesizedRemoteInstitution {
                    name: "Fixture Council".into(),
                    strategic_doctrine:
                        "Maintain the route through repair crews and convoy permits.".into(),
                },
                SynthesizedRemoteInstitution {
                    name: "Witness Guild".into(),
                    strategic_doctrine: "Record public crossings.".into(),
                },
            ],
        };
        let compiled = lower_compatible_doctrine_catalog(grounded, synthesized);
        assert_eq!(compiled.institutions.len(), 2);
        assert_eq!(compiled.institutions[0].name, "Fixture Council");
        assert_eq!(
            compiled.institutions[0].evidence_receipt_ids,
            vec!["vault:council"]
        );
        let mut seed = private_actor_test_seed();
        let (evidence, assumptions) = merge_global_agency_catalog(&mut seed, compiled).unwrap();
        assert_eq!(evidence.len(), 2);
        assert_eq!(assumptions.len(), 3);
        assert!(assumptions.iter().any(|assumption| {
            assumption.contains("Campaign-local operational doctrine for Fixture Council")
                && assumption.contains("convoy permits")
        }));
        assert!(assumptions.iter().any(|assumption| {
            assumption == "Global agency coverage limit: The exact horizon remains uncertain."
        }));
        assert_eq!(seed.institutions.len(), 2);
    }

    #[test]
    fn material_gap_contract_requires_an_exact_blocked_premise_and_evidence_ownership() {
        let mut gap = CompiledMaterialGap {
            kind: CompiledMaterialGapKind::UnanchoredRequestedBaseline,
            summary: "The requested canon office has no anchored identity.".into(),
            premise_clause: "The opening must use the canonical Winter Office.".into(),
            blocked_choice: "Name the intended canon office or admit a branch-local office.".into(),
            evidence_receipt_ids: vec![],
        };
        validate_compiled_material_gaps(&[gap.clone()], &[]).unwrap();

        gap.kind = CompiledMaterialGapKind::ContradictoryCanonBaselines;
        assert!(
            validate_compiled_material_gaps(&[gap.clone()], &[])
                .unwrap_err()
                .to_string()
                .contains("must cite supplied evidence")
        );

        gap.evidence_receipt_ids = vec!["unknown-receipt".into()];
        assert!(
            validate_compiled_material_gaps(&[gap], &[])
                .unwrap_err()
                .to_string()
                .contains("unknown evidence receipt")
        );
    }

    #[test]
    fn shared_index_language_cannot_masquerade_as_an_institution_mandate() {
        let receipts = vec![VaultEvidenceReceipt {
            schema: "ghostlight.vault_evidence_receipt.v1".into(),
            id: "vault:movement-index".into(),
            provider: "fixture".into(),
            query_hash: "sha256:movement-index".into(),
            witnesses: vec![SourceWitness {
                source_id: "AetheriaLore:Aetheria/Worldbuilding/Movements/index.md".into(),
                exact_locator: "Movements/index.md:1-4".into(),
                content_hash: "sha256:movement-index".into(),
                excerpt:
                    "Characteristic movements: Disciplinists, Pragmatists, selected Bio-Purists."
                        .into(),
                authority_lane: "aetheria.canon_worldbuilding".into(),
                temporal_scope: "fixture".into(),
            }],
            retrieved_at: Utc::now(),
        }];
        let catalog = ExtractedGlobalAgencyCatalog {
            institutions: ["Disciplinists", "Pragmatists", "Bio-Purists"]
                .into_iter()
                .map(|name| ExtractedRemoteInstitution {
                    name: name.into(),
                    supporting_claims: vec!["Characteristic movements: Disciplinists, Pragmatists, selected Bio-Purists."
                        .into()],
                })
                .collect(),
            gaps: vec![],
        };

        let (grounded, private_gaps) = ground_global_agency_catalog(catalog, &receipts).unwrap();
        assert!(grounded.institutions.is_empty());
        assert!(private_gaps.iter().any(|gap| gap.contains("Pragmatists")));
    }

    #[test]
    fn dedicated_institution_document_may_supply_a_non_self_naming_tagline() {
        assert!(source_document_names_institution(
            "AetheriaLore:Aetheria/Worldbuilding/Factions/Powers/Major/Sol Dominion.md",
            "Sol Dominion"
        ));
        assert!(!source_document_names_institution(
            "AetheriaLore:Aetheria/Worldbuilding/Factions/Powers/Major/index.md",
            "Sol Dominion"
        ));
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

    #[test]
    fn opening_topology_requires_explicit_outbound_and_return_routes() {
        let mut campaign = crate::resolution::tests::campaign(0, 1);
        let player_location = campaign.actors[&campaign.player_actor_id]
            .location_id
            .clone();
        campaign.locations.insert(
            "annex".into(),
            Location {
                id: "annex".into(),
                name: "Annex".into(),
                container_id: Some(player_location.clone()),
                routes: BTreeMap::new(),
                persistent_features: vec!["A visible annex".into()],
            },
        );

        let disconnected = validate_opening_topology(&campaign).unwrap_err();
        assert!(disconnected.to_string().contains("unreachable"));
        assert!(
            disconnected
                .to_string()
                .contains("containment does not create implicit movement")
        );

        campaign
            .locations
            .get_mut(&player_location)
            .unwrap()
            .routes
            .insert(
                "route:annex".into(),
                crate::domain::Route {
                    destination_id: "annex".into(),
                    distance: "near".into(),
                    travel_minutes: 5,
                },
            );
        assert!(
            validate_opening_topology(&campaign)
                .unwrap_err()
                .to_string()
                .contains("no route chain back")
        );

        campaign.locations.get_mut("annex").unwrap().routes.insert(
            "route:return".into(),
            crate::domain::Route {
                destination_id: player_location,
                distance: "near".into(),
                travel_minutes: 5,
            },
        );
        validate_opening_topology(&campaign).unwrap();
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

    fn valid_region_expansion() -> crate::domain::RegionExpansion {
        crate::domain::RegionExpansion {
            origin_location_id: "center".into(),
            origin_routes: BTreeMap::from([(
                "route:center-annex".into(),
                crate::domain::Route {
                    destination_id: "annex".into(),
                    distance: "near".into(),
                    travel_minutes: 10,
                },
            )]),
            locations: vec![Location {
                id: "annex".into(),
                name: "Annex".into(),
                container_id: None,
                routes: BTreeMap::from([(
                    "route:annex-center".into(),
                    crate::domain::Route {
                        destination_id: "center".into(),
                        distance: "near".into(),
                        travel_minutes: 10,
                    },
                )]),
                persistent_features: vec!["sealed gate".into()],
            }],
            facts: vec![],
            populations: vec![],
            population_profiles: vec![],
            migration_relations: vec![],
            institutions: vec![],
            institution_profiles: vec![],
            local_relations: vec![],
            civic_system: None,
        }
    }

    #[test]
    fn region_expansion_requires_an_approved_round_trip() {
        let campaign = crate::resolution::tests::campaign(0, 1);
        let mut expansion = valid_region_expansion();
        validate_region_expansion(&campaign, &expansion).unwrap();
        expansion.origin_routes.clear();
        assert!(
            validate_region_expansion(&campaign, &expansion)
                .unwrap_err()
                .to_string()
                .contains("explicit route from the origin")
        );
    }

    #[test]
    fn region_expansion_route_keys_are_local_to_their_origin() {
        let campaign = crate::resolution::tests::campaign(0, 1);
        let mut expansion = valid_region_expansion();
        let outward = expansion
            .origin_routes
            .remove("route:center-annex")
            .unwrap();
        expansion.origin_routes.insert("road".into(), outward);
        let returning = expansion.locations[0]
            .routes
            .remove("route:annex-center")
            .unwrap();
        expansion.locations[0]
            .routes
            .insert("road".into(), returning);

        validate_region_expansion(&campaign, &expansion).unwrap();
    }

    #[test]
    fn inhabited_expansion_admits_only_reachable_population_migration_edges() {
        let mut campaign = crate::resolution::tests::campaign(0, 1);
        campaign.gestalts.insert(
            "refugees".into(),
            GestaltPersonaState {
                schema: "ghostlight.gestalt_persona_state.v1".into(),
                id: "refugees".into(),
                name: "Refugees".into(),
                version: 0,
                home_location_id: "center".into(),
                shared_capabilities: BTreeSet::new(),
                shared_knowledge: BTreeSet::new(),
                resources: BTreeSet::new(),
                goals: vec!["find voluntary settlement".into()],
                pressures: vec![],
            },
        );
        crate::resolution::ensure_agency_profiles(&mut campaign);
        let mut expansion = valid_region_expansion();
        expansion.locations[0].id = "pass".into();
        expansion.locations[0].name = "Ridge Pass".into();
        expansion.locations[0].routes = BTreeMap::from([
            (
                "back".into(),
                Route {
                    destination_id: "center".into(),
                    distance: "near".into(),
                    travel_minutes: 10,
                },
            ),
            (
                "onward".into(),
                Route {
                    destination_id: "village".into(),
                    distance: "near".into(),
                    travel_minutes: 10,
                },
            ),
        ]);
        expansion
            .origin_routes
            .values_mut()
            .next()
            .unwrap()
            .destination_id = "pass".into();
        expansion.locations.push(Location {
            id: "village".into(),
            name: "Ridge Village".into(),
            container_id: None,
            routes: BTreeMap::from([(
                "return".into(),
                Route {
                    destination_id: "pass".into(),
                    distance: "near".into(),
                    travel_minutes: 10,
                },
            )]),
            persistent_features: vec!["assembly hall".into()],
        });
        let statement = "The ridge assembly governs voluntary admission.".to_owned();
        expansion.facts.push(WorldFact {
            id: "ridge-admission".into(),
            statement: statement.clone(),
            scope: FactScope::ProvisionalLocal,
            evidence_receipt_ids: vec![],
            discoverable_at_location_ids: BTreeSet::from(["village".into()]),
        });
        expansion.populations.push(GestaltPersonaState {
            schema: "ghostlight.gestalt_persona_state.v1".into(),
            id: "ridge-households".into(),
            name: "Ridge Households".into(),
            version: 0,
            home_location_id: "village".into(),
            shared_capabilities: BTreeSet::from(["communal agriculture".into()]),
            shared_knowledge: BTreeSet::from([statement]),
            resources: BTreeSet::from(["shared kitchen".into()]),
            goals: vec!["admit newcomers without surrendering local consent".into()],
            pressures: vec!["winter capacity is finite".into()],
        });
        expansion.population_profiles.push(AgencyProfile {
            schema: "ghostlight.agency_profile.v1".into(),
            id: "agency:ridge-households".into(),
            subject_id: "ridge-households".into(),
            subject_kind: AgencySubjectKind::Gestalt,
            profile_version: 0,
            collective_authority_id: Some("ridge-households".into()),
            parent_subject_id: None,
            active_leaf: true,
            simulation_eligible: true,
            facets: BTreeMap::from([
                (AgencyAxis::Geography, BTreeSet::from(["ridge".into()])),
                (AgencyAxis::Ideology, BTreeSet::from(["consent".into()])),
                (AgencyAxis::Authority, BTreeSet::from(["assembly".into()])),
                (
                    AgencyAxis::EconomyRole,
                    BTreeSet::from(["agriculture".into()]),
                ),
                (AgencyAxis::SpeciesBody, BTreeSet::from(["mixed".into()])),
                (AgencyAxis::Information, BTreeSet::from(["local".into()])),
            ]),
            location_ids: BTreeSet::from(["village".into()]),
            information_channels: BTreeSet::from(["village assembly bulletin".into()]),
            detail_debt: 0,
            last_detail_tick: 0,
            evidence_receipt_ids: vec![],
        });
        expansion.migration_relations.push(AgencyRelation {
            schema: "ghostlight.agency_relation.v1".into(),
            id: "migration:refugees:ridge".into(),
            from_subject_id: "refugees".into(),
            to_subject_id: "ridge-households".into(),
            kind: AgencyRelationKind::Migration,
            strength: 50,
            active: true,
            evidence_receipt_ids: vec![],
        });

        validate_region_expansion(&campaign, &expansion).unwrap();
        expansion.locations[0].routes.remove("onward");
        assert!(
            validate_region_expansion(&campaign, &expansion)
                .unwrap_err()
                .to_string()
                .contains("strategic travel horizon")
        );
    }

    #[test]
    fn region_expansion_rejects_containment_cycles() {
        let campaign = crate::resolution::tests::campaign(0, 1);
        let mut expansion = valid_region_expansion();
        expansion.locations[0].container_id = Some("service-ring".into());
        expansion.locations.push(Location {
            id: "service-ring".into(),
            name: "Service Ring".into(),
            container_id: Some("annex".into()),
            routes: BTreeMap::new(),
            persistent_features: vec!["fixed conduits".into()],
        });
        assert!(
            validate_region_expansion(&campaign, &expansion)
                .unwrap_err()
                .to_string()
                .contains("containment contains a cycle")
        );
    }
}
