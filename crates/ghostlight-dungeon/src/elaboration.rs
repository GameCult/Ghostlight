use anyhow::{Result, anyhow};
use async_trait::async_trait;
use schemars::{JsonSchema, schema_for};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::Arc;
use uuid::Uuid;

pub const MAX_ELABORATOR_WEIGHT: u16 = 100;
pub const COVER_BASIS_POINTS: u32 = 10_000;

/// Consumer-owned scale intent for world elaboration. This states how much of
/// the potentially acting world may receive simultaneous cell attention; it
/// does not change the campaign's active-cell entitlement or admit subjects.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct WorldScaleIntent {
    pub schema: String,
    #[schemars(range(min = 1, max = 10000))]
    pub target_active_cover_basis_points: u16,
}

impl WorldScaleIntent {
    pub fn ten_percent() -> Self {
        Self {
            schema: "ghostlight.world_scale_intent.v1".into(),
            target_active_cover_basis_points: 1_000,
        }
    }

    pub fn twenty_percent() -> Self {
        Self {
            schema: "ghostlight.world_scale_intent.v1".into(),
            target_active_cover_basis_points: 2_000,
        }
    }
}

/// Deterministic elaboration pressure derived from scale intent and current
/// canonical state. The scheduler may spend this budget on proposals; only
/// existing admission and WorldKernel paths can turn them into subjects.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct WorldElaborationDemand {
    pub schema: String,
    pub active_cell_budget: u16,
    pub target_active_cover_basis_points: u16,
    pub target_actionable_subjects: u32,
    pub current_actionable_subjects: u32,
    pub actionable_subject_deficit: u32,
    /// Bounded work to attempt before remeasuring canonical complexity. This
    /// is pressure, not an assumed subject yield or model-call count.
    pub round_mutation_budget: u32,
    pub realm_complexity_weights: BTreeMap<String, u32>,
    pub realm_subject_targets: BTreeMap<String, u32>,
}

pub fn residual_gestalt_ids(campaign: &crate::domain::Campaign) -> BTreeSet<String> {
    campaign
        .gestalt_lineages
        .values()
        .map(|lineage| lineage.residual_child_id.clone())
        .collect()
}

fn semantically_qualified_fission_child(
    campaign: &crate::domain::Campaign,
    child_id: &str,
    parent_id: &str,
) -> bool {
    let Some(child) = campaign.gestalts.get(child_id) else {
        return false;
    };
    let Some(parent) = campaign.gestalts.get(parent_id) else {
        return false;
    };
    if !child
        .shared_capabilities
        .is_superset(&parent.shared_capabilities)
        || !child.shared_knowledge.is_superset(&parent.shared_knowledge)
        || !child.goals.starts_with(&parent.goals)
        || !child.pressures.starts_with(&parent.pressures)
    {
        return false;
    }
    [
        child.shared_capabilities.len() > parent.shared_capabilities.len(),
        child.shared_knowledge.len() > parent.shared_knowledge.len(),
        child.goals.len() > parent.goals.len(),
        child.pressures.len() > parent.pressures.len(),
    ]
    .into_iter()
    .filter(|present| *present)
    .count()
        >= 2
}

/// Returns active action owners whose current state can justify complexity
/// credit. Compiler-admitted roots and individuated people are admitted by
/// their owning gates. A fission child must additionally retain two or more
/// semantic dimensions beyond its exact parent; a merely relabelled ordinary
/// split remains simulatable but cannot pay the world-size bill.
pub fn canonical_actionable_subject_ids(campaign: &crate::domain::Campaign) -> BTreeSet<String> {
    let residual_ids = residual_gestalt_ids(campaign);
    let fission_parents = campaign
        .gestalt_lineages
        .values()
        .flat_map(|lineage| {
            lineage
                .child_gestalt_ids
                .iter()
                .map(move |child_id| (child_id.clone(), lineage.parent_gestalt_id.clone()))
        })
        .collect::<BTreeMap<_, _>>();
    campaign
        .agency_profiles
        .values()
        .filter(|profile| {
            profile.active_leaf
                && profile.simulation_eligible
                && !residual_ids.contains(&profile.subject_id)
                && fission_parents
                    .get(&profile.subject_id)
                    .is_none_or(|parent_id| {
                        semantically_qualified_fission_child(
                            campaign,
                            &profile.subject_id,
                            parent_id,
                        )
                    })
        })
        .map(|profile| profile.subject_id.clone())
        .collect()
}

/// Counts qualified active agency leaves that could own an action. Dormant
/// member records, census texture, and unresolved residual populations do not
/// satisfy world-complexity demand. Residuals preserve unresolved lineage and
/// membership state, but are not simulation-eligible action owners and cannot
/// be recursively split for quota.
pub fn canonical_actionable_subject_count(campaign: &crate::domain::Campaign) -> u32 {
    u32::try_from(canonical_actionable_subject_ids(campaign).len()).unwrap_or(u32::MAX)
}

/// Resolve exact operating places through canonical containment to one
/// consumer-declared jurisdiction. Presence remains owned by the profile;
/// this read-only derivation never promotes a container into occupancy.
pub fn unique_containing_jurisdiction(
    campaign: &crate::domain::Campaign,
    location_ids: &BTreeSet<String>,
    jurisdiction_ids: &BTreeSet<String>,
) -> Option<String> {
    let mut jurisdictions = BTreeSet::new();
    for location_id in location_ids {
        let mut current = Some(location_id.as_str());
        let mut visited = BTreeSet::new();
        while let Some(candidate) = current {
            if !visited.insert(candidate) {
                break;
            }
            if jurisdiction_ids.contains(candidate) {
                jurisdictions.insert(candidate.to_owned());
                break;
            }
            current = campaign
                .locations
                .get(candidate)
                .and_then(|location| location.container_id.as_deref());
        }
    }
    (jurisdictions.len() == 1).then(|| {
        jurisdictions
            .into_iter()
            .next()
            .expect("one jurisdiction was resolved")
    })
}

pub fn derive_world_elaboration_demand(
    active_cell_budget: u16,
    current_actionable_subjects: u32,
    intent: &WorldScaleIntent,
    realm_complexity_weights: BTreeMap<String, u32>,
) -> Result<WorldElaborationDemand> {
    if intent.schema != "ghostlight.world_scale_intent.v1"
        || intent.target_active_cover_basis_points == 0
        || u32::from(intent.target_active_cover_basis_points) > COVER_BASIS_POINTS
    {
        return Err(anyhow!("world scale intent is unsupported or out of range"));
    }
    if active_cell_budget == 0 {
        return Err(anyhow!(
            "world scale intent requires a nonzero active-cell budget"
        ));
    }
    if realm_complexity_weights.is_empty()
        || realm_complexity_weights
            .iter()
            .any(|(realm, weight)| realm.trim().is_empty() || *weight == 0)
    {
        return Err(anyhow!(
            "world elaboration demand requires nonempty realms with positive complexity weights"
        ));
    }
    let cover = u32::from(intent.target_active_cover_basis_points);
    let target_actionable_subjects = u32::from(active_cell_budget)
        .saturating_mul(COVER_BASIS_POINTS)
        .div_ceil(cover);
    let actionable_subject_deficit =
        target_actionable_subjects.saturating_sub(current_actionable_subjects);
    let round_mutation_budget = if actionable_subject_deficit == 0 {
        0
    } else {
        u32::from(active_cell_budget)
            .saturating_mul(actionable_subject_deficit)
            .div_ceil(target_actionable_subjects)
            .max(1)
    };
    let total_weight = realm_complexity_weights
        .values()
        .try_fold(0_u32, |total, weight| total.checked_add(*weight))
        .ok_or_else(|| anyhow!("realm complexity weights overflow"))?;
    let mut realm_subject_targets = BTreeMap::new();
    let mut assigned = 0_u32;
    let last_realm = realm_complexity_weights.keys().next_back().cloned();
    for (realm, weight) in &realm_complexity_weights {
        let share = if Some(realm) == last_realm.as_ref() {
            target_actionable_subjects.saturating_sub(assigned)
        } else {
            target_actionable_subjects.saturating_mul(*weight) / total_weight
        };
        assigned = assigned.saturating_add(share);
        realm_subject_targets.insert(realm.clone(), share);
    }
    Ok(WorldElaborationDemand {
        schema: "ghostlight.world_elaboration_demand.v1".into(),
        active_cell_budget,
        target_active_cover_basis_points: intent.target_active_cover_basis_points,
        target_actionable_subjects,
        current_actionable_subjects,
        actionable_subject_deficit,
        round_mutation_budget,
        realm_complexity_weights,
        realm_subject_targets,
    })
}

#[derive(
    Clone, Copy, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq, PartialOrd, Ord, Hash,
)]
#[serde(rename_all = "snake_case")]
pub enum ElaboratorTitle {
    /// Ordinary objects, customs, nicknames, local jokes, and other reusable
    /// low-stakes texture.
    Patina,
    /// Government, law, offices, selection, succession, and redress.
    Charter,
    /// Resources, labor, trade, infrastructure, scarcity, and class pressure.
    Ledger,
    /// Kinship, daily life, care, obligation, belonging, and neighborhood ties.
    Hearth,
    /// Factions, alliances, rivalries, leverage, and political maneuvering.
    Tangle,
    /// Secrets, rumors, misinformation, taboos, and unevenly held knowledge.
    Veil,
    /// Active disputes, hazards, instability, escalation, and urgent pressure.
    Ember,
    /// Religion, magic, ritual, awe, cosmology, and the genuinely strange.
    Numen,
}

impl ElaboratorTitle {
    pub const ALL: [Self; 8] = [
        Self::Patina,
        Self::Charter,
        Self::Ledger,
        Self::Hearth,
        Self::Tangle,
        Self::Veil,
        Self::Ember,
        Self::Numen,
    ];

    pub fn mandate(self) -> &'static str {
        match self {
            Self::Patina => {
                "Add durable, locally grounded, low-stakes texture that later events may acquire meaning from."
            }
            Self::Charter => {
                "Elaborate legible institutions, offices, procedures, succession, public authority, and redress."
            }
            Self::Ledger => {
                "Elaborate material dependencies: work, resources, infrastructure, exchange, scarcity, and class pressure."
            }
            Self::Hearth => {
                "Elaborate ordinary social life: kinship, care, neighborhood, obligation, belonging, and private stakes."
            }
            Self::Tangle => {
                "Elaborate political relationships: factions, alliances, rivalries, leverage, constituencies, and plots."
            }
            Self::Veil => {
                "Elaborate uneven knowledge: secrets, rumors, taboos, misinformation, mysteries, and disclosure paths."
            }
            Self::Ember => {
                "Elaborate active pressure: disputes, hazards, instability, escalation paths, and urgent consequences."
            }
            Self::Numen => {
                "Elaborate numinous meaning: religion, ritual, magic, cosmology, wonder, and bounded strangeness."
            }
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            Self::Patina => "Patina",
            Self::Charter => "Charter",
            Self::Ledger => "Ledger",
            Self::Hearth => "Hearth",
            Self::Tangle => "Tangle",
            Self::Veil => "Veil",
            Self::Ember => "Ember",
            Self::Numen => "Numen",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ElaboratorDescriptor {
    pub title: ElaboratorTitle,
    pub display_name: String,
    pub mandate: String,
}

pub fn elaborator_catalog() -> Vec<ElaboratorDescriptor> {
    ElaboratorTitle::ALL
        .into_iter()
        .map(|title| ElaboratorDescriptor {
            title,
            display_name: title.display_name().into(),
            mandate: title.mandate().into(),
        })
        .collect()
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ElaboratorControl {
    pub title: ElaboratorTitle,
    /// Relative scheduler weight. Zero disables this elaborator. Positive
    /// values are normalized across the titles eligible for each dispatch.
    #[schemars(range(min = 0, max = 100))]
    pub weight: u16,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct WorldElaborationProfile {
    pub schema: String,
    pub controls: Vec<ElaboratorControl>,
}

impl Default for WorldElaborationProfile {
    fn default() -> Self {
        Self {
            schema: "ghostlight.world_elaboration_profile.v1".into(),
            controls: ElaboratorTitle::ALL
                .into_iter()
                .map(|title| ElaboratorControl { title, weight: 50 })
                .collect(),
        }
    }
}

impl WorldElaborationProfile {
    pub fn weights(&self) -> Result<BTreeMap<ElaboratorTitle, u16>> {
        if self.schema != "ghostlight.world_elaboration_profile.v1" {
            return Err(anyhow!("world elaboration profile schema is unsupported"));
        }
        let mut weights = BTreeMap::new();
        for control in &self.controls {
            if control.weight > MAX_ELABORATOR_WEIGHT
                || weights.insert(control.title, control.weight).is_some()
            {
                return Err(anyhow!(
                    "world elaboration profile requires unique titles with weights from zero to {MAX_ELABORATOR_WEIGHT}"
                ));
            }
        }
        if weights.values().all(|weight| *weight == 0) {
            return Err(anyhow!(
                "world elaboration profile must enable at least one titled elaborator"
            ));
        }
        Ok(weights)
    }
}

/// Compacted working memory for one titled elaborator. Narrative fields steer
/// later proposals but own no world truth; exact commit receipts and the
/// canonical campaign remain authoritative.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ElaboratorSessionCheckpoint {
    pub schema: String,
    pub session_id: String,
    pub title: ElaboratorTitle,
    pub generation: u32,
    pub campaign_id: Uuid,
    pub through_world_revision: u64,
    pub target_location_id: String,
    pub frontier_summary: String,
    pub unresolved_leads: Vec<String>,
    pub recent_commit_receipt_ids: Vec<String>,
    pub recent_rejection_findings: Vec<String>,
    pub prior_checkpoint_digest: Option<String>,
    pub digest: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ElaboratorSessionCompactionDraft {
    pub schema: String,
    pub frontier_summary: String,
    pub unresolved_leads: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ElaboratorSessionJournalEntry {
    pub world_revision: u64,
    pub commit_receipt_id: String,
    pub mutation_kind: String,
    pub affected_subject_ids: Vec<String>,
    pub summary: String,
}

/// Stable logical-session routing key. A title keeps a distinct bounded mind
/// in each world jurisdiction instead of carrying one global transcript across
/// unrelated realms.
pub fn elaborator_session_id(title: ElaboratorTitle, jurisdiction_id: &str) -> String {
    format!(
        "{}:{}",
        title.display_name().to_ascii_lowercase(),
        jurisdiction_id
    )
}

impl ElaboratorSessionCheckpoint {
    #[allow(clippy::too_many_arguments)]
    pub fn bind_compaction(
        session_id: impl Into<String>,
        title: ElaboratorTitle,
        generation: u32,
        campaign_id: Uuid,
        through_world_revision: u64,
        target_location_id: impl Into<String>,
        draft: ElaboratorSessionCompactionDraft,
        recent_commit_receipt_ids: Vec<String>,
        recent_rejection_findings: Vec<String>,
        prior_checkpoint_digest: Option<String>,
    ) -> Result<Self> {
        if draft.schema != "ghostlight.elaborator_session_compaction_draft.v1" {
            return Err(anyhow!(
                "elaborator session compaction draft schema is unsupported"
            ));
        }
        let mut checkpoint = Self {
            schema: "ghostlight.elaborator_session_checkpoint.v1".into(),
            session_id: session_id.into(),
            title,
            generation,
            campaign_id,
            through_world_revision,
            target_location_id: target_location_id.into(),
            frontier_summary: draft.frontier_summary,
            unresolved_leads: draft.unresolved_leads,
            recent_commit_receipt_ids,
            recent_rejection_findings,
            prior_checkpoint_digest,
            digest: String::new(),
        };
        checkpoint.validate_shape()?;
        checkpoint.digest = checkpoint.recompute_digest()?;
        Ok(checkpoint)
    }

    pub fn validate_for(
        &self,
        campaign: &crate::domain::Campaign,
        target_location_id: &str,
        title: ElaboratorTitle,
    ) -> Result<()> {
        self.validate_shape()?;
        if self.campaign_id != campaign.id
            || self.through_world_revision > campaign.revision
            || self.target_location_id != target_location_id
            || self.title != title
            || self.digest != self.recompute_digest()?
        {
            return Err(anyhow!(
                "elaborator session checkpoint is stale, cross-session, or malformed"
            ));
        }
        Ok(())
    }

    fn validate_shape(&self) -> Result<()> {
        let bounded = |value: &str, maximum: usize| {
            !value.trim().is_empty() && value.chars().count() <= maximum
        };
        if self.schema != "ghostlight.elaborator_session_checkpoint.v1"
            || !bounded(&self.session_id, 160)
            || !bounded(&self.target_location_id, 240)
            || !bounded(&self.frontier_summary, 4_000)
            || self.unresolved_leads.len() > 32
            || self.unresolved_leads.iter().any(|lead| !bounded(lead, 600))
            || self.recent_commit_receipt_ids.len() > 64
            || self
                .recent_commit_receipt_ids
                .iter()
                .any(|id| !bounded(id, 240))
            || self.recent_rejection_findings.len() > 32
            || self
                .recent_rejection_findings
                .iter()
                .any(|finding| !bounded(finding, 800))
            || self
                .prior_checkpoint_digest
                .as_deref()
                .is_some_and(|digest| !bounded(digest, 160))
        {
            return Err(anyhow!(
                "elaborator session checkpoint exceeds its bounded memory contract"
            ));
        }
        Ok(())
    }

    fn recompute_digest(&self) -> Result<String> {
        crate::legacy_transition::digest_serializable(&(
            "ghostlight.elaborator_session_checkpoint.v1",
            &self.session_id,
            self.title,
            self.generation,
            self.campaign_id,
            self.through_world_revision,
            &self.target_location_id,
            &self.frontier_summary,
            &self.unresolved_leads,
            &self.recent_commit_receipt_ids,
            &self.recent_rejection_findings,
            &self.prior_checkpoint_digest,
        ))
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(deny_unknown_fields)]
struct ElaboratorSessionCompactionFinding {
    diagnostic: String,
}

struct ElaboratorSessionCompactionTool {
    workbench: serde_json::Value,
}

#[async_trait]
impl crate::agent::ModelAgentTool for ElaboratorSessionCompactionTool {
    type Action = ElaboratorSessionCompactionDraft;
    type Output = ElaboratorSessionCompactionDraft;
    type Finding = ElaboratorSessionCompactionFinding;

    fn action_schema(&self) -> std::result::Result<serde_json::Value, String> {
        let mut schema = serde_json::to_value(schema_for!(ElaboratorSessionCompactionDraft))
            .map_err(|error| error.to_string())?;
        schema["properties"]["schema"] =
            exact_schema("ghostlight.elaborator_session_compaction_draft.v1");
        schema["properties"]["frontier_summary"]["minLength"] = serde_json::json!(1);
        schema["properties"]["frontier_summary"]["maxLength"] = serde_json::json!(4000);
        schema["properties"]["unresolved_leads"]["maxItems"] = serde_json::json!(32);
        schema["properties"]["unresolved_leads"]["items"]["minLength"] = serde_json::json!(1);
        schema["properties"]["unresolved_leads"]["items"]["maxLength"] = serde_json::json!(600);
        Ok(schema)
    }

    fn initial_context_snapshot(&self) -> Option<serde_json::Value> {
        Some(self.workbench.clone())
    }

    async fn invoke(
        &mut self,
        action: Self::Action,
        _context: &crate::agent::ModelAgentToolContext,
    ) -> crate::agent::ModelAgentToolOutcome<Self::Output, Self::Finding> {
        let bounded = |value: &str, maximum: usize| {
            !value.trim().is_empty() && value.chars().count() <= maximum
        };
        let mut findings = Vec::new();
        if action.schema != "ghostlight.elaborator_session_compaction_draft.v1" {
            findings.push("compaction schema is unsupported".to_owned());
        }
        if !bounded(&action.frontier_summary, 4_000) {
            findings.push("frontier summary must contain 1 through 4000 characters".to_owned());
        }
        if action.unresolved_leads.len() > 32 {
            findings.push("compaction contains more than 32 unresolved leads".to_owned());
        }
        for (index, lead) in action.unresolved_leads.iter().enumerate() {
            if !bounded(lead, 600) {
                findings.push(format!(
                    "unresolved lead {index} must contain 1 through 600 characters"
                ));
            }
        }
        if !findings.is_empty() {
            return crate::agent::ModelAgentToolOutcome::Rejected {
                finding: ElaboratorSessionCompactionFinding {
                    diagnostic: findings.join("; "),
                },
                receipts: Vec::new(),
            };
        }
        crate::agent::ModelAgentToolOutcome::Accepted {
            output: action,
            receipts: Vec::new(),
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub async fn compact_elaborator_session(
    model: &dyn crate::model::ModelPort,
    campaign: &crate::domain::Campaign,
    target_location_id: &str,
    title: ElaboratorTitle,
    session_id: &str,
    prior: Option<&ElaboratorSessionCheckpoint>,
    journal: &[ElaboratorSessionJournalEntry],
    recent_rejection_findings: Vec<String>,
) -> Result<(
    ElaboratorSessionCheckpoint,
    Vec<crate::model::ModelStageReceipt>,
)> {
    let mut findings = Vec::new();
    if !campaign.locations.contains_key(target_location_id) {
        findings.push(format!(
            "target location {target_location_id:?} is absent from the canonical campaign"
        ));
    }
    if journal.is_empty() && recent_rejection_findings.is_empty() {
        findings.push(
            "compaction requires an admitted mutation or a semantic rejection finding".to_owned(),
        );
    }
    if journal.len() > 64 {
        findings.push(format!(
            "journal contains {} entries; the maximum is 64",
            journal.len()
        ));
    }
    for (index, entry) in journal.iter().enumerate() {
        if entry.world_revision > campaign.revision {
            findings.push(format!(
                "journal[{index}].world_revision {} exceeds canonical revision {}",
                entry.world_revision, campaign.revision
            ));
        }
        if entry.commit_receipt_id.trim().is_empty() {
            findings.push(format!("journal[{index}].commit_receipt_id is empty"));
        }
        if entry.mutation_kind.trim().is_empty() {
            findings.push(format!("journal[{index}].mutation_kind is empty"));
        }
        if entry.summary.trim().is_empty() {
            findings.push(format!("journal[{index}].summary is empty"));
        }
        let summary_length = entry.summary.chars().count();
        if summary_length > 1_000 {
            findings.push(format!(
                "journal[{index}].summary contains {summary_length} characters; the maximum is 1000"
            ));
        }
        if entry.affected_subject_ids.len() > 32 {
            findings.push(format!(
                "journal[{index}].affected_subject_ids contains {} subjects; the maximum is 32",
                entry.affected_subject_ids.len()
            ));
        }
    }
    if recent_rejection_findings.len() > 32 {
        findings.push(format!(
            "recent_rejection_findings contains {} entries; the maximum is 32",
            recent_rejection_findings.len()
        ));
    }
    for (index, finding) in recent_rejection_findings.iter().enumerate() {
        let length = finding.trim().chars().count();
        if length == 0 || length > 800 {
            findings.push(format!(
                "recent_rejection_findings[{index}] contains {length} characters; the permitted range is 1 through 800"
            ));
        }
    }
    if !findings.is_empty() {
        return Err(anyhow!(
            "elaborator session compaction preflight failed: {}",
            findings.join("; ")
        ));
    }
    if let Some(prior) = prior {
        prior.validate_for(campaign, target_location_id, title)?;
    }
    let generation = prior.map_or(0, |checkpoint| checkpoint.generation.saturating_add(1));
    let snapshot_binding = crate::legacy_transition::digest_serializable(&(
        "ghostlight.elaborator_session_compaction_snapshot.v1",
        campaign.id,
        campaign.revision,
        target_location_id,
        title,
        session_id,
        prior.map(|checkpoint| checkpoint.digest.as_str()),
        journal,
        &recent_rejection_findings,
    ))?;
    let instructions = format!(
        "You compact working memory for the {} elaborator. {} Preserve the causal frontier, unfinished structural leads, and useful rejected paths. Do not invent commits, subjects, facts, or completed work. The canonical world and exact journal remain authoritative; your summary only steers the next turn.",
        title.display_name(),
        title.mandate(),
    );
    let spec = crate::agent::ModelAgentSpec {
        stage: format!(
            "world-elaboration-{}-session-compaction",
            title.display_name().to_ascii_lowercase()
        ),
        model: crate::model::MODEL_FAST.into(),
        snapshot_binding,
        instructions,
        source_receipt_ids: Vec::new(),
        temperature: Some(0.1),
        max_output_tokens: Some(1_400),
        max_steps: 2,
    };
    let mut tool = ElaboratorSessionCompactionTool {
        workbench: serde_json::json!({
            "schema":"ghostlight.elaborator_session_compaction_workbench.v1",
            "prior_checkpoint":prior,
            "recent_admitted_journal":journal,
            "recent_rejection_findings":recent_rejection_findings,
            "through_world_revision":campaign.revision,
            "target_location_id":target_location_id,
        }),
    };
    let run = crate::agent::run_model_agent(model, &spec, &mut tool)
        .await
        .map_err(|failure| anyhow!(failure.message))?;
    let checkpoint = ElaboratorSessionCheckpoint::bind_compaction(
        session_id,
        title,
        generation,
        campaign.id,
        campaign.revision,
        target_location_id,
        run.output,
        journal
            .iter()
            .map(|entry| entry.commit_receipt_id.clone())
            .collect(),
        recent_rejection_findings,
        prior.map(|checkpoint| checkpoint.digest.clone()),
    )?;
    Ok((checkpoint, run.receipts))
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ElaborationDispatchState {
    pub schema: String,
    pub profile_digest: String,
    /// Smooth weighted-round-robin credit across every enabled title. Blocked
    /// selections consume their configured slots without invoking an agent,
    /// so their share is never donated to another elaborator.
    pub current_scores: BTreeMap<ElaboratorTitle, i64>,
    pub dispatch_counts: BTreeMap<ElaboratorTitle, u64>,
    pub total_budget_slots: u64,
    pub total_dispatches: u64,
}

impl Default for ElaborationDispatchState {
    fn default() -> Self {
        Self {
            schema: "ghostlight.elaboration_dispatch_state.v1".into(),
            profile_digest: String::new(),
            current_scores: BTreeMap::new(),
            dispatch_counts: BTreeMap::new(),
            total_budget_slots: 0,
            total_dispatches: 0,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ElaborationDispatch {
    pub schema: String,
    pub budget_ordinal: u64,
    pub ordinal: u64,
    pub title: ElaboratorTitle,
    pub title_weight: u16,
    pub total_enabled_weight: u32,
    /// Requested share of the complete configured invocation budget.
    pub requested_share_millionths: u32,
    pub title_dispatch_count: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ElaborationScheduleReceipt {
    pub schema: String,
    pub requested_invocations: u32,
    pub unused_invocations: u32,
    pub eligible_titles: BTreeSet<ElaboratorTitle>,
    /// Actual invocations allocated inside this receipt's budget window.
    pub dispatch_counts: BTreeMap<ElaboratorTitle, u32>,
    /// Configured allocations that could not invoke their titled agent. These
    /// remain unused instead of being redistributed to another slider.
    pub unused_counts: BTreeMap<ElaboratorTitle, u32>,
    pub dispatches: Vec<ElaborationDispatch>,
    pub final_state: ElaborationDispatchState,
}

enum ElaborationBudgetAllocation {
    Dispatch(ElaborationDispatch),
    Unused(ElaboratorTitle),
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ElaborationWaveBinding {
    pub schema: String,
    /// Content-addressed or revision-addressed identity for the immutable
    /// world projection shared by every invocation in this wave.
    pub snapshot_binding: String,
}

impl ElaborationWaveBinding {
    pub fn validate(&self) -> Result<()> {
        if self.schema != "ghostlight.elaboration_wave_binding.v1" {
            return Err(anyhow!("elaboration wave binding schema is unsupported"));
        }
        if self.snapshot_binding.trim().is_empty() {
            return Err(anyhow!("elaboration wave snapshot binding cannot be empty"));
        }
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct ElaborationSubAgentInvocation {
    pub wave: ElaborationWaveBinding,
    pub dispatch: ElaborationDispatch,
}

#[derive(Debug)]
pub struct ElaborationInvocation<Proposal> {
    pub wave: ElaborationWaveBinding,
    pub dispatch: ElaborationDispatch,
    /// A proposal against the frozen elaboration snapshot. The invocation
    /// cannot commit canonical world state; admission and conflict handling
    /// remain downstream owners.
    pub proposal: Proposal,
    /// Exact model/tool ancestry for this proposal. These receipts are audit
    /// evidence only; they never become source evidence or mutation authority.
    pub model_stage_receipts: Vec<crate::model::ModelStageReceipt>,
}

#[derive(Debug)]
pub struct ElaborationSubAgentOutput<Proposal> {
    pub proposal: Proposal,
    pub model_stage_receipts: Vec<crate::model::ModelStageReceipt>,
}

impl<Proposal> ElaborationSubAgentOutput<Proposal> {
    pub fn deterministic(proposal: Proposal) -> Self {
        Self {
            proposal,
            model_stage_receipts: Vec::new(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct ElaborationSubAgentFailure {
    pub diagnostic: String,
    pub model_stage_receipts: Vec<crate::model::ModelStageReceipt>,
}

impl std::fmt::Display for ElaborationSubAgentFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.diagnostic)
    }
}

impl std::error::Error for ElaborationSubAgentFailure {}

#[derive(Debug)]
pub struct ElaborationWaveRun<Proposal> {
    wave: ElaborationWaveBinding,
    schedule: ElaborationScheduleReceipt,
    invocations: Vec<ElaborationInvocation<Proposal>>,
}

impl<Proposal> ElaborationWaveRun<Proposal> {
    pub fn wave(&self) -> &ElaborationWaveBinding {
        &self.wave
    }

    pub fn schedule(&self) -> &ElaborationScheduleReceipt {
        &self.schedule
    }

    pub fn invocations(&self) -> &[ElaborationInvocation<Proposal>] {
        &self.invocations
    }
}

#[derive(Clone, Debug)]
pub struct ElaborationInvocationFailure {
    pub dispatch: Option<ElaborationDispatch>,
    pub diagnostic: String,
    pub model_stage_receipts: Vec<crate::model::ModelStageReceipt>,
}

#[derive(Debug)]
pub struct ElaborationWaveFailure<Proposal> {
    pub wave: Option<ElaborationWaveBinding>,
    pub schedule: Option<ElaborationScheduleReceipt>,
    pub completed_invocations: Vec<ElaborationInvocation<Proposal>>,
    pub invocation_failures: Vec<ElaborationInvocationFailure>,
}

#[async_trait]
pub trait ElaborationSubAgentPort<Proposal>: Send + Sync {
    async fn invoke(
        &self,
        invocation: ElaborationSubAgentInvocation,
    ) -> std::result::Result<ElaborationSubAgentOutput<Proposal>, ElaborationSubAgentFailure>;
}

pub struct ElaborationScheduler {
    weights: BTreeMap<ElaboratorTitle, u16>,
    state: ElaborationDispatchState,
}

impl ElaborationScheduler {
    pub fn new(profile: &WorldElaborationProfile) -> Result<Self> {
        Self::from_state(profile, ElaborationDispatchState::default())
    }

    pub fn from_state(
        profile: &WorldElaborationProfile,
        mut state: ElaborationDispatchState,
    ) -> Result<Self> {
        let weights = profile.weights()?;
        let profile_digest = crate::legacy_transition::digest_serializable(&(
            "ghostlight.world_elaboration_profile.v1",
            &weights,
        ))?;
        if state.schema != "ghostlight.elaboration_dispatch_state.v1" {
            return Err(anyhow!("elaboration dispatch state schema is unsupported"));
        }
        if state.profile_digest.is_empty() {
            state.profile_digest = profile_digest;
        } else if state.profile_digest != profile_digest {
            return Err(anyhow!(
                "elaboration dispatch state belongs to a different slider profile"
            ));
        }
        state
            .current_scores
            .retain(|title, _| weights.contains_key(title));
        state
            .dispatch_counts
            .retain(|title, _| weights.contains_key(title));
        Ok(Self { weights, state })
    }

    pub fn state(&self) -> &ElaborationDispatchState {
        &self.state
    }

    fn next_allocation(
        &mut self,
        eligible_titles: &BTreeSet<ElaboratorTitle>,
    ) -> ElaborationBudgetAllocation {
        let enabled = self
            .weights
            .iter()
            .filter(|(_, weight)| **weight > 0)
            .map(|(title, weight)| (*title, *weight))
            .collect::<Vec<_>>();
        let total_enabled_weight = enabled
            .iter()
            .map(|(_, weight)| u32::from(*weight))
            .sum::<u32>();
        debug_assert!(total_enabled_weight > 0);

        let mut selected = None;
        for (title, weight) in &enabled {
            let score = self.state.current_scores.entry(*title).or_default();
            *score = score.saturating_add(i64::from(*weight));
            let candidate = (*title, *score);
            if selected.is_none_or(|best: (ElaboratorTitle, i64)| candidate.1 > best.1) {
                selected = Some(candidate);
            }
        }
        let (title, _) = selected.expect("a valid profile requires one enabled title");
        *self.state.current_scores.entry(title).or_default() -= i64::from(total_enabled_weight);
        self.state.total_budget_slots = self.state.total_budget_slots.saturating_add(1);
        if !eligible_titles.contains(&title) {
            return ElaborationBudgetAllocation::Unused(title);
        }

        self.state.total_dispatches = self.state.total_dispatches.saturating_add(1);
        let title_dispatch_count = self
            .state
            .dispatch_counts
            .entry(title)
            .and_modify(|count| *count = count.saturating_add(1))
            .or_insert(1);
        let title_weight = self.weights[&title];
        ElaborationBudgetAllocation::Dispatch(ElaborationDispatch {
            schema: "ghostlight.elaboration_dispatch.v1".into(),
            budget_ordinal: self.state.total_budget_slots,
            ordinal: self.state.total_dispatches,
            title,
            title_weight,
            total_enabled_weight,
            requested_share_millionths: u32::from(title_weight).saturating_mul(1_000_000)
                / total_enabled_weight,
            title_dispatch_count: *title_dispatch_count,
        })
    }

    pub fn schedule(
        &mut self,
        eligible_titles: &BTreeSet<ElaboratorTitle>,
        invocation_budget: u32,
    ) -> ElaborationScheduleReceipt {
        let mut dispatches = Vec::with_capacity(invocation_budget as usize);
        let mut unused_counts = BTreeMap::new();
        for _ in 0..invocation_budget {
            match self.next_allocation(eligible_titles) {
                ElaborationBudgetAllocation::Dispatch(dispatch) => dispatches.push(dispatch),
                ElaborationBudgetAllocation::Unused(title) => {
                    *unused_counts.entry(title).or_default() += 1;
                }
            }
        }
        ElaborationScheduleReceipt {
            schema: "ghostlight.elaboration_schedule_receipt.v1".into(),
            requested_invocations: invocation_budget,
            unused_invocations: unused_counts.values().copied().sum(),
            eligible_titles: eligible_titles.clone(),
            dispatch_counts: dispatches
                .iter()
                .fold(BTreeMap::new(), |mut counts, dispatch| {
                    *counts.entry(dispatch.title).or_default() += 1;
                    counts
                }),
            unused_counts,
            dispatches,
            final_state: self.state.clone(),
        }
    }
}

/// Dispatches one immutable-snapshot elaboration wave. Weight determines the
/// number of actual sub-agent invocations, not prompt emphasis. Proposals are
/// returned in deterministic dispatch order after parallel execution; this
/// function has no canonical world-state writer.
pub async fn dispatch_elaboration_wave<Worker, Proposal>(
    scheduler: &mut ElaborationScheduler,
    wave: ElaborationWaveBinding,
    eligible_titles: &BTreeSet<ElaboratorTitle>,
    invocation_budget: u32,
    parallelism: usize,
    worker: Arc<Worker>,
) -> std::result::Result<ElaborationWaveRun<Proposal>, ElaborationWaveFailure<Proposal>>
where
    Worker: ElaborationSubAgentPort<Proposal> + 'static,
    Proposal: Send + 'static,
{
    if let Err(error) = wave.validate() {
        return Err(ElaborationWaveFailure {
            wave: Some(wave),
            schedule: None,
            completed_invocations: Vec::new(),
            invocation_failures: vec![ElaborationInvocationFailure {
                dispatch: None,
                diagnostic: error.to_string(),
                model_stage_receipts: Vec::new(),
            }],
        });
    }
    if parallelism == 0 {
        return Err(ElaborationWaveFailure {
            wave: Some(wave),
            schedule: None,
            completed_invocations: Vec::new(),
            invocation_failures: vec![ElaborationInvocationFailure {
                dispatch: None,
                diagnostic: "elaboration wave parallelism must be greater than zero".into(),
                model_stage_receipts: Vec::new(),
            }],
        });
    }
    let schedule = scheduler.schedule(eligible_titles, invocation_budget);
    let (invocations, invocation_failures) = invoke_elaboration_dispatches(
        wave.clone(),
        schedule.dispatches.iter().cloned().collect(),
        parallelism,
        worker,
    )
    .await;
    if !invocation_failures.is_empty() {
        return Err(ElaborationWaveFailure {
            wave: Some(wave),
            schedule: Some(schedule),
            completed_invocations: invocations,
            invocation_failures,
        });
    }
    Ok(ElaborationWaveRun {
        wave,
        schedule,
        invocations,
    })
}

/// Resumes only the failed dispatches from one complete, immutable wave
/// checkpoint. The original schedule remains authoritative: successful
/// invocations are retained, scheduler state is not advanced, and every retry
/// keeps its exact title, ordinal, and frozen snapshot binding.
pub async fn resume_elaboration_wave<Worker, Proposal>(
    failure: ElaborationWaveFailure<Proposal>,
    parallelism: usize,
    worker: Arc<Worker>,
) -> std::result::Result<ElaborationWaveRun<Proposal>, ElaborationWaveFailure<Proposal>>
where
    Worker: ElaborationSubAgentPort<Proposal> + 'static,
    Proposal: Send + 'static,
{
    let ElaborationWaveFailure {
        wave,
        schedule,
        mut completed_invocations,
        invocation_failures,
    } = failure;
    let Some(wave) = wave else {
        return Err(ElaborationWaveFailure {
            wave: None,
            schedule,
            completed_invocations,
            invocation_failures,
        });
    };
    let Some(schedule) = schedule else {
        return Err(ElaborationWaveFailure {
            wave: Some(wave),
            schedule: None,
            completed_invocations,
            invocation_failures,
        });
    };
    let checkpoint_error = validate_partial_wave_checkpoint(
        &wave,
        &schedule,
        &completed_invocations,
        &invocation_failures,
    )
    .err()
    .map(|error| error.to_string())
    .or_else(|| {
        (parallelism == 0).then(|| "elaboration wave parallelism must be greater than zero".into())
    });
    if let Some(diagnostic) = checkpoint_error {
        return Err(ElaborationWaveFailure {
            wave: Some(wave),
            schedule: Some(schedule),
            completed_invocations,
            invocation_failures: vec![ElaborationInvocationFailure {
                dispatch: None,
                diagnostic,
                model_stage_receipts: Vec::new(),
            }],
        });
    }
    let failed_dispatches = invocation_failures
        .into_iter()
        .map(|failure| failure.dispatch.expect("validated checkpoint dispatch"))
        .collect::<Vec<_>>();
    let (mut resumed_invocations, invocation_failures) =
        invoke_elaboration_dispatches(wave.clone(), failed_dispatches, parallelism, worker).await;
    completed_invocations.append(&mut resumed_invocations);
    completed_invocations.sort_by_key(|invocation| invocation.dispatch.ordinal);
    if !invocation_failures.is_empty() {
        return Err(ElaborationWaveFailure {
            wave: Some(wave),
            schedule: Some(schedule),
            completed_invocations,
            invocation_failures,
        });
    }
    let run = ElaborationWaveRun {
        wave,
        schedule,
        invocations: completed_invocations,
    };
    if let Err(error) = validate_successful_wave_run(&run) {
        return Err(ElaborationWaveFailure {
            wave: Some(run.wave),
            schedule: Some(run.schedule),
            completed_invocations: run.invocations,
            invocation_failures: vec![ElaborationInvocationFailure {
                dispatch: None,
                diagnostic: error.to_string(),
                model_stage_receipts: Vec::new(),
            }],
        });
    }
    Ok(run)
}

async fn invoke_elaboration_dispatches<Worker, Proposal>(
    wave: ElaborationWaveBinding,
    dispatches: Vec<ElaborationDispatch>,
    parallelism: usize,
    worker: Arc<Worker>,
) -> (
    Vec<ElaborationInvocation<Proposal>>,
    Vec<ElaborationInvocationFailure>,
)
where
    Worker: ElaborationSubAgentPort<Proposal> + 'static,
    Proposal: Send + 'static,
{
    let invocation_count = dispatches.len();
    let semaphore = Arc::new(tokio::sync::Semaphore::new(parallelism));
    let mut jobs = tokio::task::JoinSet::new();
    let mut task_dispatches = HashMap::new();
    for dispatch in dispatches {
        let worker = worker.clone();
        let semaphore = semaphore.clone();
        let wave = wave.clone();
        let task_dispatch = dispatch.clone();
        let task = jobs.spawn(async move {
            let _slot =
                semaphore
                    .acquire_owned()
                    .await
                    .map_err(|_| ElaborationInvocationFailure {
                        dispatch: Some(dispatch.clone()),
                        diagnostic: "elaboration invocation gate closed".into(),
                        model_stage_receipts: Vec::new(),
                    })?;
            let request = ElaborationSubAgentInvocation {
                wave: wave.clone(),
                dispatch: dispatch.clone(),
            };
            let output =
                worker
                    .invoke(request)
                    .await
                    .map_err(|error| ElaborationInvocationFailure {
                        dispatch: Some(dispatch.clone()),
                        diagnostic: error.diagnostic.chars().take(2_000).collect(),
                        model_stage_receipts: error.model_stage_receipts,
                    })?;
            Ok::<_, ElaborationInvocationFailure>(ElaborationInvocation {
                wave,
                dispatch,
                proposal: output.proposal,
                model_stage_receipts: output.model_stage_receipts,
            })
        });
        task_dispatches.insert(task.id(), task_dispatch);
    }

    let mut invocations = Vec::with_capacity(invocation_count);
    let mut invocation_failures = Vec::new();
    while let Some(result) = jobs.join_next_with_id().await {
        match result {
            Ok((task_id, Ok(invocation))) => {
                task_dispatches.remove(&task_id);
                invocations.push(invocation);
            }
            Ok((task_id, Err(failure))) => {
                task_dispatches.remove(&task_id);
                invocation_failures.push(failure);
            }
            Err(error) => invocation_failures.push(ElaborationInvocationFailure {
                dispatch: task_dispatches.remove(&error.id()),
                diagnostic: format!("elaboration sub-agent task failed: {error}"),
                model_stage_receipts: Vec::new(),
            }),
        }
    }
    invocations.sort_by_key(|invocation| invocation.dispatch.ordinal);
    invocation_failures.sort_by_key(|failure| {
        failure
            .dispatch
            .as_ref()
            .map(|dispatch| dispatch.ordinal)
            .unwrap_or(u64::MAX)
    });
    (invocations, invocation_failures)
}

/// Provider-backed titled worker for one additive locality operation. The
/// model chooses content through the generic agent harness; a deterministic
/// tool checks the exact task assignment and frozen campaign namespace before
/// returning a proposal to wave admission.
pub struct ModelWorldElaborationWorker {
    model: Arc<dyn crate::model::ModelPort>,
    campaign: Arc<crate::domain::Campaign>,
    target_location_id: String,
    request: String,
    session_checkpoints: BTreeMap<ElaboratorTitle, ElaboratorSessionCheckpoint>,
}

impl ModelWorldElaborationWorker {
    pub fn new(
        model: Arc<dyn crate::model::ModelPort>,
        campaign: Arc<crate::domain::Campaign>,
        target_location_id: impl Into<String>,
        request: impl Into<String>,
    ) -> Result<Self> {
        let target_location_id = target_location_id.into();
        if !campaign.locations.contains_key(&target_location_id) {
            return Err(anyhow!("world elaboration worker target is unknown"));
        }
        if !campaign.civic_systems.contains_key(&target_location_id) {
            return Err(anyhow!(
                "provider-backed titled elaboration currently requires an admitted civic foundation"
            ));
        }
        Ok(Self {
            model,
            campaign,
            target_location_id,
            request: request.into(),
            session_checkpoints: BTreeMap::new(),
        })
    }

    pub fn with_session_checkpoints(
        mut self,
        checkpoints: BTreeMap<ElaboratorTitle, ElaboratorSessionCheckpoint>,
    ) -> Result<Self> {
        for (title, checkpoint) in &checkpoints {
            checkpoint.validate_for(&self.campaign, &self.target_location_id, *title)?;
        }
        self.session_checkpoints = checkpoints;
        Ok(self)
    }

    /// The exact semantic task shared by the worker wave, its artifacts, and
    /// the independent verifier. Callers should read this value from the
    /// prepared worker instead of retaining a second request owner.
    pub fn task_request(&self) -> &str {
        &self.request
    }

    fn projection(&self) -> Result<String> {
        let civic = &self.campaign.civic_systems[&self.target_location_id];
        let subject_ids = civic
            .governing_institution_ids
            .iter()
            .chain(civic.resident_population_ids.iter())
            .cloned()
            .collect::<BTreeSet<_>>();
        let projection = serde_json::json!({
            "campaign_id":self.campaign.id,
            "world_revision":self.campaign.revision,
            "campaign_name":self.campaign.name,
            "target_location":self.campaign.locations.get(&self.target_location_id),
            "current_civic_system":civic,
            "civic_institutions":subject_ids.iter().filter_map(|id|self.campaign.institutions.get(id)).collect::<Vec<_>>(),
            "civic_populations":subject_ids.iter().filter_map(|id|self.campaign.gestalts.get(id)).collect::<Vec<_>>(),
            "political_relations":civic.political_relation_ids.iter().filter_map(|id|self.campaign.agency_relations.get(id)).collect::<Vec<_>>(),
            "public_facts":civic.public_authority_fact_ids.iter()
                .chain(civic.public_selection_fact_ids.iter())
                .chain(civic.public_resource_fact_ids.iter())
                .chain(civic.public_redress_fact_ids.iter())
                .filter_map(|id|self.campaign.facts.get(id)).collect::<Vec<_>>(),
            "existing_fact_namespace":self.campaign.facts.values().map(|fact|serde_json::json!({
                "id":fact.id,
                "statement":fact.statement,
            })).collect::<Vec<_>>(),
            "request":self.request,
        });
        Ok(serde_json::to_string(&projection)?)
    }
}

/// Provider-backed worker for one causally meaningful Gestalt subdivision.
/// A round assigns distinct active parents against one frozen campaign. The
/// worker proposes only; callers must revalidate and commit each preview
/// through `WorldKernel::ElaborateGestaltFission`.
pub struct ModelWorldComplexityWorker {
    model: Arc<dyn crate::model::ModelPort>,
    campaign: Arc<crate::domain::Campaign>,
    wave: ElaborationWaveBinding,
    first_dispatch_ordinal: u64,
    parent_gestalt_ids: Vec<String>,
    parent_jurisdiction_ids: BTreeMap<String, String>,
    target_actionable_gains: BTreeMap<u64, u8>,
    session_checkpoints: BTreeMap<String, ElaboratorSessionCheckpoint>,
}

impl ModelWorldComplexityWorker {
    pub fn new(
        model: Arc<dyn crate::model::ModelPort>,
        campaign: Arc<crate::domain::Campaign>,
        first_dispatch_ordinal: u64,
        parent_gestalt_ids: Vec<String>,
        parent_jurisdiction_ids: BTreeMap<String, String>,
        actionable_subject_deficit: u32,
        session_checkpoints: BTreeMap<String, ElaboratorSessionCheckpoint>,
    ) -> Result<Self> {
        if parent_gestalt_ids.is_empty()
            || parent_gestalt_ids.iter().collect::<BTreeSet<_>>().len() != parent_gestalt_ids.len()
        {
            return Err(anyhow!(
                "complexity round requires distinct assigned parent Gestalts"
            ));
        }
        if actionable_subject_deficit == 0
            || u32::try_from(parent_gestalt_ids.len()).unwrap_or(u32::MAX)
                > actionable_subject_deficit
        {
            return Err(anyhow!(
                "complexity round cannot dispatch more parents than its remaining subject deficit"
            ));
        }
        for parent_id in &parent_gestalt_ids {
            let profile = campaign.agency_profiles.get(parent_id);
            if !campaign.gestalts.contains_key(parent_id)
                || profile.is_none_or(|profile| {
                    !profile.active_leaf
                        || !profile.simulation_eligible
                        || profile.subject_kind != crate::domain::AgencySubjectKind::Gestalt
                })
            {
                return Err(anyhow!(
                    "complexity round parent is not an unresolved active Gestalt"
                ));
            }
            let Some(jurisdiction_id) = parent_jurisdiction_ids.get(parent_id) else {
                return Err(anyhow!(
                    "complexity round parent has no session jurisdiction"
                ));
            };
            let permitted_jurisdictions = BTreeSet::from([jurisdiction_id.clone()]);
            if unique_containing_jurisdiction(
                &campaign,
                &campaign.agency_profiles[parent_id].location_ids,
                &permitted_jurisdictions,
            )
            .as_deref()
                != Some(jurisdiction_id.as_str())
            {
                return Err(anyhow!(
                    "complexity round session jurisdiction does not contain the parent location"
                ));
            }
        }
        if parent_jurisdiction_ids.len() != parent_gestalt_ids.len() {
            return Err(anyhow!(
                "complexity round session jurisdictions do not match assigned parents"
            ));
        }
        let planned_gain = actionable_subject_deficit.min(
            u32::try_from(parent_gestalt_ids.len())
                .unwrap_or(u32::MAX)
                .saturating_mul(6),
        );
        let parent_count = u32::try_from(parent_gestalt_ids.len()).unwrap_or(u32::MAX);
        let base_gain = planned_gain / parent_count;
        let remainder = planned_gain % parent_count;
        let target_actionable_gains = parent_gestalt_ids
            .iter()
            .enumerate()
            .map(|(index, _)| {
                let index = u32::try_from(index).unwrap_or(u32::MAX);
                let gain = base_gain.saturating_add(u32::from(index < remainder));
                Ok((
                    first_dispatch_ordinal.saturating_add(u64::from(index)),
                    u8::try_from(gain.clamp(1, 6))?,
                ))
            })
            .collect::<std::result::Result<BTreeMap<_, _>, std::num::TryFromIntError>>()?;
        for (session_id, checkpoint) in &session_checkpoints {
            if checkpoint.session_id != *session_id
                || elaborator_session_id(checkpoint.title, &checkpoint.target_location_id)
                    != *session_id
            {
                return Err(anyhow!(
                    "complexity worker received a stale or misrouted session checkpoint"
                ));
            }
            checkpoint.validate_for(&campaign, &checkpoint.target_location_id, checkpoint.title)?;
        }
        let wave = ElaborationWaveBinding {
            schema: "ghostlight.elaboration_wave_binding.v1".into(),
            snapshot_binding: crate::legacy_transition::digest_serializable(&(
                "ghostlight.world_complexity_round.v1",
                campaign.id,
                campaign.revision,
                first_dispatch_ordinal,
                &parent_gestalt_ids,
            ))?,
        };
        Ok(Self {
            model,
            campaign,
            wave,
            first_dispatch_ordinal,
            parent_gestalt_ids,
            parent_jurisdiction_ids,
            target_actionable_gains,
            session_checkpoints,
        })
    }

    pub fn wave(&self) -> &ElaborationWaveBinding {
        &self.wave
    }

    fn parent_for(&self, dispatch: &ElaborationDispatch) -> Result<&str> {
        let offset = dispatch
            .ordinal
            .checked_sub(self.first_dispatch_ordinal)
            .ok_or_else(|| anyhow!("complexity dispatch predates its assigned round"))?;
        self.parent_gestalt_ids
            .get(usize::try_from(offset).unwrap_or(usize::MAX))
            .map(String::as_str)
            .ok_or_else(|| anyhow!("complexity dispatch exceeds its assigned parent set"))
    }
}

fn complexity_partition_axis(title: ElaboratorTitle) -> crate::domain::AgencyAxis {
    use crate::domain::AgencyAxis;
    match title {
        ElaboratorTitle::Patina | ElaboratorTitle::Hearth | ElaboratorTitle::Ember => {
            AgencyAxis::Geography
        }
        ElaboratorTitle::Charter => AgencyAxis::Authority,
        ElaboratorTitle::Ledger => AgencyAxis::EconomyRole,
        ElaboratorTitle::Tangle | ElaboratorTitle::Numen => AgencyAxis::Ideology,
        ElaboratorTitle::Veil => AgencyAxis::Information,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum WorldComplexityOperation {
    Fission,
    Individuate,
}

fn complexity_operation(title: ElaboratorTitle) -> WorldComplexityOperation {
    match title {
        ElaboratorTitle::Hearth | ElaboratorTitle::Tangle | ElaboratorTitle::Veil => {
            WorldComplexityOperation::Individuate
        }
        _ => WorldComplexityOperation::Fission,
    }
}

/// One proposed increase in causally meaningful world resolution. This is
/// deliberately the same command algebra the kernel already owns: elaborators
/// may either split a population into distinct active leaves or promote one
/// consequential person from it. They do not own a parallel subject store.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(tag = "operation", rename_all = "snake_case", deny_unknown_fields)]
pub enum WorldComplexityProposal {
    Fission {
        preview: crate::domain::GestaltFissionPreview,
        qualification: WorldComplexityFissionQualification,
    },
    Individuate {
        individuation: crate::domain::GestaltIndividuation,
        qualification: WorldComplexityIndividuationQualification,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq, Default)]
#[serde(deny_unknown_fields)]
pub struct WorldComplexitySemanticQualification {
    pub schema: String,
    pub identity_namespace: String,
    pub frozen_campaign_id: Uuid,
    pub frozen_world_revision: u64,
    pub established_names_digest: String,
    pub semantic_context_digest: String,
    pub owner_lane_digest: String,
    pub candidate_digest: String,
    pub semantic_verification_binding: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct WorldComplexityFissionQualification {
    pub schema: String,
    pub title: ElaboratorTitle,
    pub jurisdiction_location_id: String,
    pub target_actionable_gain: u8,
    #[serde(default)]
    pub semantic: WorldComplexitySemanticQualification,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct WorldComplexityIndividuationQualification {
    pub schema: String,
    pub title: ElaboratorTitle,
    pub jurisdiction_location_id: String,
    #[serde(default)]
    pub semantic: WorldComplexitySemanticQualification,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
struct WorldComplexityAction {
    schema: String,
    mutation: WorldComplexityMutationDraft,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(tag = "operation", rename_all = "snake_case", deny_unknown_fields)]
enum WorldComplexityMutationDraft {
    Fission {
        children: Vec<WorldComplexityChildDraft>,
        residual_child_id: String,
        member_child_assignments: BTreeMap<String, String>,
        resource_child_assignments: BTreeMap<String, String>,
    },
    Individuate {
        member: WorldComplexityMemberDraft,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
struct WorldComplexityChildDraft {
    id: String,
    name: String,
    partition_value: String,
    home_location_id: String,
    capability_additions: BTreeSet<String>,
    knowledge_additions: BTreeSet<String>,
    goals_add: Vec<String>,
    pressures_add: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
struct WorldComplexityMemberDraft {
    id: String,
    name: String,
    capability_additions: BTreeSet<String>,
    capability_removals: BTreeSet<String>,
    knowledge_additions: BTreeSet<String>,
    knowledge_removals: BTreeSet<String>,
    equipment: BTreeSet<String>,
    conditions: BTreeSet<String>,
    obligations: BTreeSet<String>,
    relationships: BTreeMap<String, String>,
    goals: Vec<String>,
    memories: Vec<String>,
}

impl From<crate::domain::GestaltMemberDelta> for WorldComplexityMemberDraft {
    fn from(member: crate::domain::GestaltMemberDelta) -> Self {
        Self {
            id: member.id,
            name: member.name,
            capability_additions: member.capability_additions,
            capability_removals: member.capability_removals,
            knowledge_additions: member.knowledge_additions,
            knowledge_removals: member.knowledge_removals,
            equipment: member.equipment,
            conditions: member.conditions,
            obligations: member.obligations,
            relationships: member.relationships,
            goals: member.goals,
            memories: member.memories,
        }
    }
}

impl WorldComplexityProposal {
    pub fn parent_gestalt_id(&self) -> &str {
        match self {
            Self::Fission { preview, .. } => &preview.parent_gestalt_id,
            Self::Individuate { individuation, .. } => &individuation.gestalt_id,
        }
    }

    pub fn mutation_kind(&self) -> &'static str {
        match self {
            Self::Fission { .. } => "elaborate_gestalt_fission",
            Self::Individuate { .. } => "elaborate_gestalt_individuation",
        }
    }

    pub fn expected_world_revision(&self) -> u64 {
        match self {
            Self::Fission { preview, .. } => preview.expected_world_revision,
            Self::Individuate { qualification, .. } => qualification.semantic.frozen_world_revision,
        }
    }
}

/// Rebinds only the optimistic revision of a proposal produced in a frozen
/// parallel round. The assigned parent, its active profile, and all of its
/// member state must remain byte-identical; unrelated earlier commits may not
/// force an expensive proposal replay.
pub fn world_complexity_parent_binding(
    campaign: &crate::domain::Campaign,
    parent_gestalt_id: &str,
) -> Result<String> {
    let members = campaign
        .gestalt_members
        .values()
        .filter(|member| member.gestalt_id == parent_gestalt_id)
        .collect::<Vec<_>>();
    crate::legacy_transition::digest_serializable(&(
        "ghostlight.world_complexity_parent.v1",
        campaign.id,
        campaign.gestalts.get(parent_gestalt_id),
        campaign.agency_profiles.get(parent_gestalt_id),
        members,
    ))
}

pub fn rebase_world_complexity_proposal(
    frozen_parent_binding: &str,
    current: &crate::domain::Campaign,
    mut proposal: WorldComplexityProposal,
) -> Result<WorldComplexityProposal> {
    let parent_id = proposal.parent_gestalt_id().to_owned();
    if world_complexity_parent_binding(current, &parent_id)? != frozen_parent_binding {
        return Err(anyhow!(
            "complexity mutation parent changed after its frozen proposal"
        ));
    }
    match &mut proposal {
        WorldComplexityProposal::Fission {
            preview,
            qualification,
        } => {
            if qualification.schema != "ghostlight.world_complexity_fission_qualification.v1"
                || !(1..=6).contains(&qualification.target_actionable_gain)
            {
                return Err(anyhow!(
                    "complexity fission qualification is unsupported or out of range"
                ));
            }
            if preview.campaign_id != current.id {
                return Err(anyhow!("complexity fission belongs to another campaign"));
            }
            preview.expected_world_revision = current.revision;
            reconcile_fission_child_id_collisions(current, preview)?;
            crate::resolution::validate_elaborated_fission(current, preview)?;
            validate_complexity_fission_semantics(
                current,
                &current.gestalts[&preview.parent_gestalt_id],
                preview,
                qualification.title,
                &qualification.jurisdiction_location_id,
                qualification.target_actionable_gain,
            )?;
        }
        WorldComplexityProposal::Individuate { individuation, .. } => {
            crate::resolution::validate_gestalt_individuation(current, individuation)?;
        }
    }
    Ok(proposal)
}

fn reconcile_fission_child_id_collisions(
    current: &crate::domain::Campaign,
    preview: &mut crate::domain::GestaltFissionPreview,
) -> Result<()> {
    let mut replacements = BTreeMap::new();
    for child in &preview.children {
        if crate::resolution::canonical_subject_id_is_occupied(current, &child.id) {
            let digest = crate::legacy_transition::digest_serializable(&(
                "ghostlight.fission_child_id.v1",
                current.id,
                &preview.parent_gestalt_id,
                &child.id,
            ))?;
            let suffix = digest
                .strip_prefix("sha256:")
                .unwrap_or(&digest)
                .chars()
                .take(16)
                .collect::<String>();
            let replacement = format!("{}:fission:{suffix}", preview.parent_gestalt_id);
            if crate::resolution::canonical_subject_id_is_occupied(current, &replacement)
                || preview
                    .children
                    .iter()
                    .any(|other| other.id == replacement && other.id != child.id)
                || replacements.values().any(|other| other == &replacement)
            {
                return Err(anyhow!(
                    "deterministic fission child id reconciliation collided"
                ));
            }
            replacements.insert(child.id.clone(), replacement);
        }
    }
    if replacements.is_empty() {
        return Ok(());
    }
    for child in &mut preview.children {
        if let Some(replacement) = replacements.get(&child.id) {
            child.id = replacement.clone();
        }
    }
    preview.child_partition_values = preview
        .child_partition_values
        .iter()
        .map(|(child_id, value)| {
            (
                replacements
                    .get(child_id)
                    .cloned()
                    .unwrap_or_else(|| child_id.clone()),
                value.clone(),
            )
        })
        .collect();
    if let Some(replacement) = replacements.get(&preview.residual_child_id) {
        preview.residual_child_id = replacement.clone();
    }
    for child_id in preview.member_child_assignments.values_mut() {
        if let Some(replacement) = replacements.get(child_id) {
            *child_id = replacement.clone();
        }
    }
    for child_id in preview.resource_child_assignments.values_mut() {
        if let Some(replacement) = replacements.get(child_id) {
            *child_id = replacement.clone();
        }
    }
    Ok(())
}

#[derive(Clone, Debug, Serialize)]
#[serde(deny_unknown_fields)]
struct WorldComplexityFinding {
    diagnostic: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct WorldComplexitySemanticVerification {
    pub public_names_are_legible_identifiers: bool,
    pub names_do_not_repeat_an_overused_template: bool,
    pub cultural_resemblance_is_grounded_not_quota_cloning: bool,
    pub causal_additions_are_materially_distinct: bool,
    pub causal_additions_do_not_repeat_an_overused_procedural_template: bool,
    #[schemars(length(min = 1, max = 1_000))]
    pub rationale: String,
}

struct WorldComplexityTool<'a> {
    campaign: &'a crate::domain::Campaign,
    parent_gestalt_id: &'a str,
    title: ElaboratorTitle,
    operation: WorldComplexityOperation,
    partition_axis: crate::domain::AgencyAxis,
    jurisdiction_location_id: &'a str,
    target_actionable_gain: u8,
    workbench: serde_json::Value,
}

pub fn locations_in_jurisdiction(
    campaign: &crate::domain::Campaign,
    jurisdiction_location_id: &str,
) -> Vec<String> {
    let jurisdictions = BTreeSet::from([jurisdiction_location_id.to_owned()]);
    campaign
        .locations
        .keys()
        .filter(|location_id| {
            unique_containing_jurisdiction(
                campaign,
                &BTreeSet::from([(*location_id).clone()]),
                &jurisdictions,
            )
            .as_deref()
                == Some(jurisdiction_location_id)
        })
        .cloned()
        .collect()
}

fn semantic_text_key(value: &str) -> String {
    value
        .split_whitespace()
        .flat_map(|part| part.chars())
        .filter(|character| character.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

fn world_complexity_semantic_candidate_digest(
    proposal: &WorldComplexityProposal,
) -> Result<String> {
    let candidate = match proposal {
        WorldComplexityProposal::Fission { preview, .. } => serde_json::json!({
            "operation":"fission",
            "parent_gestalt_id":preview.parent_gestalt_id,
            "partition_axis":preview.partition_axis,
            "specific_children":preview.children.iter()
                .filter(|child|child.id != preview.residual_child_id)
                .map(|child|serde_json::json!({
                    "name":child.name,
                    "partition_value":preview.child_partition_values.get(&child.id),
                    "home_location_id":child.home_location_id,
                    "capabilities":child.shared_capabilities,
                    "knowledge":child.shared_knowledge,
                    "goals":child.goals,
                    "pressures":child.pressures,
                })).collect::<Vec<_>>(),
        }),
        WorldComplexityProposal::Individuate { individuation, .. } => serde_json::json!({
            "operation":"individuate",
            "parent_gestalt_id":individuation.gestalt_id,
            "location_id":individuation.location_id,
            "member":individuation.member,
        }),
    };
    crate::legacy_transition::digest_serializable(&(
        "ghostlight.world_complexity_semantic_candidate.v1",
        candidate,
    ))
}

fn world_complexity_semantic_verification_binding(
    semantic: &WorldComplexitySemanticQualification,
) -> Result<String> {
    crate::legacy_transition::digest_serializable(&(
        "ghostlight.world_complexity_semantic_verification.v1",
        semantic.frozen_campaign_id,
        semantic.frozen_world_revision,
        &semantic.identity_namespace,
        &semantic.established_names_digest,
        &semantic.semantic_context_digest,
        &semantic.owner_lane_digest,
        &semantic.candidate_digest,
    ))
}

fn world_complexity_owner_lane_digest(proposal: &WorldComplexityProposal) -> Result<String> {
    let lane = match proposal {
        WorldComplexityProposal::Fission { qualification, .. } => serde_json::json!({
            "operation":"fission",
            "title":qualification.title,
            "jurisdiction_location_id":qualification.jurisdiction_location_id,
            "target_actionable_gain":qualification.target_actionable_gain,
        }),
        WorldComplexityProposal::Individuate { qualification, .. } => serde_json::json!({
            "operation":"individuate",
            "title":qualification.title,
            "jurisdiction_location_id":qualification.jurisdiction_location_id,
        }),
    };
    crate::legacy_transition::digest_serializable(&(
        "ghostlight.world_complexity_owner_lane.v1",
        lane,
    ))
}

pub fn validate_world_complexity_semantic_qualification_shape(
    proposal: &WorldComplexityProposal,
    semantic: &WorldComplexitySemanticQualification,
) -> Result<()> {
    let expected_namespace = match proposal {
        WorldComplexityProposal::Fission { .. } => "population",
        WorldComplexityProposal::Individuate { .. } => "person",
    };
    if semantic.schema != "ghostlight.world_complexity_semantic_qualification.v1"
        || semantic.identity_namespace != expected_namespace
        || semantic.candidate_digest != world_complexity_semantic_candidate_digest(proposal)?
        || semantic.established_names_digest.trim().is_empty()
        || semantic.semantic_context_digest.trim().is_empty()
        || semantic.owner_lane_digest != world_complexity_owner_lane_digest(proposal)?
        || semantic.semantic_verification_binding
            != world_complexity_semantic_verification_binding(semantic)?
    {
        return Err(anyhow!(
            "world-complexity semantic qualification is malformed or bound to another candidate"
        ));
    }
    Ok(())
}

pub fn validate_world_complexity_semantic_qualification(
    campaign: &crate::domain::Campaign,
    proposal: &WorldComplexityProposal,
    semantic: &WorldComplexitySemanticQualification,
) -> Result<()> {
    validate_world_complexity_semantic_qualification_shape(proposal, semantic)?;
    let expected_namespace = match proposal {
        WorldComplexityProposal::Fission { .. } => "population",
        WorldComplexityProposal::Individuate { .. } => "person",
    };
    let established_names = match expected_namespace {
        "population" => {
            let residual_ids = residual_gestalt_ids(campaign);
            campaign
                .gestalts
                .values()
                .filter(|gestalt| !residual_ids.contains(&gestalt.id))
                .map(|gestalt| gestalt.name.as_str())
                .collect::<Vec<_>>()
        }
        "person" => campaign
            .actors
            .values()
            .map(|actor| actor.name.as_str())
            .chain(campaign.gestalt_members.values().filter_map(|member| {
                member
                    .materialized_actor_id
                    .is_none()
                    .then_some(member.name.as_str())
            }))
            .collect::<Vec<_>>(),
        _ => unreachable!("semantic namespace is fixed above"),
    };
    let expected_names_digest = crate::legacy_transition::digest_serializable(&(
        "ghostlight.world_complexity_established_names.v1",
        expected_namespace,
        established_names,
    ))?;
    if semantic.frozen_campaign_id != campaign.id
        || semantic.frozen_world_revision != campaign.revision
        || semantic.established_names_digest != expected_names_digest
    {
        return Err(anyhow!(
            "world-complexity semantic qualification is stale, malformed, or bound to another candidate"
        ));
    }
    Ok(())
}

pub fn validate_world_complexity_semantic_receipt_provenance(
    proposal: &WorldComplexityProposal,
    semantic: &WorldComplexitySemanticQualification,
    receipts: &[crate::model::ModelStageReceipt],
) -> Result<()> {
    validate_world_complexity_semantic_qualification_shape(proposal, semantic)?;
    let receipt_keys = receipts
        .iter()
        .map(|receipt| receipt.storage_key().to_owned())
        .collect::<BTreeSet<_>>();
    if receipt_keys.len() != receipts.len() {
        return Err(anyhow!(
            "world-complexity semantic provenance contains duplicate model receipts"
        ));
    }
    let verifier_receipts = receipts
        .iter()
        .filter(|receipt| receipt.stage == "world-complexity-semantic-verification")
        .collect::<Vec<_>>();
    if verifier_receipts.len() != 1 {
        return Err(anyhow!(
            "world-complexity semantic qualification needs exactly one valid bound verifier receipt"
        ));
    }
    let verifier = verifier_receipts[0];
    let expected_sources = receipts
        .iter()
        .filter(|receipt| receipt.stage != "world-complexity-semantic-verification")
        .map(|receipt| receipt.storage_key().to_owned())
        .collect::<BTreeSet<_>>();
    let actual_sources = verifier
        .source_receipt_ids
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    if verifier.model != crate::model::MODEL_BALANCED
        || verifier.validation_result != "valid"
        || verifier.local_validation_error.is_some()
        || verifier.snapshot_binding != semantic.semantic_verification_binding
        || expected_sources.is_empty()
        || actual_sources.len() != verifier.source_receipt_ids.len()
        || actual_sources != expected_sources
    {
        return Err(anyhow!(
            "world-complexity semantic verifier ancestry does not exactly cover its generation receipts"
        ));
    }
    Ok(())
}

pub(crate) fn validate_world_complexity_semantic_provenance(
    campaign: &crate::domain::Campaign,
    proposal: &WorldComplexityProposal,
    semantic: &WorldComplexitySemanticQualification,
    receipts: &[crate::model::ModelStageReceipt],
) -> Result<()> {
    validate_world_complexity_semantic_qualification(campaign, proposal, semantic)?;
    validate_world_complexity_semantic_receipt_provenance(proposal, semantic, receipts)
}

impl WorldComplexitySemanticVerification {
    pub fn accepted(&self) -> bool {
        self.public_names_are_legible_identifiers
            && self.names_do_not_repeat_an_overused_template
            && self.cultural_resemblance_is_grounded_not_quota_cloning
            && self.causal_additions_are_materially_distinct
            && self.causal_additions_do_not_repeat_an_overused_procedural_template
            && !self.rationale.trim().is_empty()
    }
}

pub async fn qualify_world_complexity_proposal_semantics(
    model: &dyn crate::model::ModelPort,
    campaign: &crate::domain::Campaign,
    semantic_comparison_context: &serde_json::Value,
    mut proposal: WorldComplexityProposal,
    source_receipt_ids: Vec<String>,
) -> Result<(
    WorldComplexityProposal,
    WorldComplexitySemanticVerification,
    crate::model::ModelStageReceipt,
)> {
    if source_receipt_ids.is_empty() {
        return Err(anyhow!(
            "world-complexity semantic verification has no source generation receipt"
        ));
    }
    let (identity_namespace, established_names) = match &proposal {
        WorldComplexityProposal::Fission { .. } => {
            let residual_ids = residual_gestalt_ids(campaign);
            (
                "population",
                campaign
                    .gestalts
                    .values()
                    .filter(|gestalt| !residual_ids.contains(&gestalt.id))
                    .map(|gestalt| gestalt.name.as_str())
                    .collect::<Vec<_>>(),
            )
        }
        WorldComplexityProposal::Individuate { .. } => (
            "person",
            campaign
                .actors
                .values()
                .map(|actor| actor.name.as_str())
                .chain(campaign.gestalt_members.values().filter_map(|member| {
                    member
                        .materialized_actor_id
                        .is_none()
                        .then_some(member.name.as_str())
                }))
                .collect::<Vec<_>>(),
        ),
    };
    let established_names_digest = crate::legacy_transition::digest_serializable(&(
        "ghostlight.world_complexity_established_names.v1",
        identity_namespace,
        &established_names,
    ))?;
    let semantic_context_digest = crate::legacy_transition::digest_serializable(&(
        "ghostlight.world_complexity_semantic_context.v1",
        semantic_comparison_context,
    ))?;
    let candidate_digest = world_complexity_semantic_candidate_digest(&proposal)?;
    let mut qualification = WorldComplexitySemanticQualification {
        schema: "ghostlight.world_complexity_semantic_qualification.v1".into(),
        identity_namespace: identity_namespace.into(),
        frozen_campaign_id: campaign.id,
        frozen_world_revision: campaign.revision,
        established_names_digest,
        semantic_context_digest,
        owner_lane_digest: world_complexity_owner_lane_digest(&proposal)?,
        candidate_digest,
        semantic_verification_binding: String::new(),
    };
    qualification.semantic_verification_binding =
        world_complexity_semantic_verification_binding(&qualification)?;
    let parent = campaign
        .gestalts
        .get(proposal.parent_gestalt_id())
        .ok_or_else(|| anyhow!("world-complexity semantic proposal lost its parent"))?;
    let prompt = format!(
        "Independently judge one proposed world-complexity mutation against the current canonical world and the supplied recent peer context. This is semantic review, not a keyword or spelling rule. Every public name must be a legible person or population identifier, not a role label, serial placeholder, or lightly varied quota template. Reject an overused surname, epithet, grammatical mold, bureaucratic construction, or cultureless fantasy-name pattern unless actual kinship or local culture grounds the resemblance. Also reject goals, pressures, capabilities, knowledge, obligations, relationships, or memories that merely paraphrase an overused procedural mold (for example everybody filing, registering, petitioning, or serving on another council) without adding materially different means, stakes, affordances, or causal behavior. Repeated vocabulary is allowed when the concrete mechanism and consequence differ. Judge the proposal as a whole and return only the verdict object.\n\nIDENTITY NAMESPACE:\n{}\nCOMPLETE ESTABLISHED NAMES:\n{}\nPARENT POPULATION:\n{}\nRECENT SEMANTIC COMPARISON CONTEXT:\n{}\nCANDIDATE PROPOSAL:\n{}",
        identity_namespace,
        serde_json::to_string(&established_names)?,
        serde_json::to_string(parent)?,
        serde_json::to_string(semantic_comparison_context)?,
        serde_json::to_string(&proposal)?,
    );
    let request = crate::model::ModelStageRequest {
        stage: "world-complexity-semantic-verification".into(),
        model: crate::model::MODEL_BALANCED.into(),
        snapshot_binding: qualification.semantic_verification_binding.clone(),
        lived_stream: prompt,
        output_schema: Some(serde_json::to_value(schema_for!(
            WorldComplexitySemanticVerification
        ))?),
        source_receipt_ids,
        temperature: Some(0.0),
        max_output_tokens: Some(1_000),
    };
    let output = crate::model::run_validated_stage(model, &request).await?;
    let verdict = serde_json::from_value::<WorldComplexitySemanticVerification>(
        output
            .structured
            .ok_or_else(|| anyhow!("semantic verifier returned no structured verdict"))?,
    )?;
    let mut receipt = output.receipt;
    if !verdict.accepted() {
        crate::model::mark_model_receipt_semantic_invalid(
            &mut receipt,
            &format!(
                "world-complexity semantic verifier rejected the candidate: {}",
                serde_json::to_string(&verdict).unwrap_or_else(|_| verdict.rationale.clone())
            ),
        );
    }
    match &mut proposal {
        WorldComplexityProposal::Fission {
            qualification: owner,
            ..
        } => owner.semantic = qualification,
        WorldComplexityProposal::Individuate {
            qualification: owner,
            ..
        } => owner.semantic = qualification,
    }
    Ok((proposal, verdict, receipt))
}

fn validate_new_semantic_texts<'a>(
    label: &str,
    additions: impl IntoIterator<Item = &'a String>,
    inherited: impl IntoIterator<Item = &'a String>,
    established: impl IntoIterator<Item = &'a String>,
) -> Result<BTreeSet<String>> {
    let inherited = inherited
        .into_iter()
        .map(|value| semantic_text_key(value))
        .collect::<BTreeSet<_>>();
    let established = established
        .into_iter()
        .map(|value| semantic_text_key(value))
        .collect::<BTreeSet<_>>();
    let mut keys = BTreeSet::new();
    for value in additions {
        let key = semantic_text_key(value);
        if value.trim().is_empty() || value.chars().count() > 300 || key.is_empty() {
            return Err(anyhow!(
                "{label} must contain 1 through 300 readable characters"
            ));
        }
        if inherited.contains(&key) || established.contains(&key) || !keys.insert(key) {
            return Err(anyhow!(
                "{label} must add text distinct from its parent, siblings, and established world"
            ));
        }
    }
    Ok(keys)
}

pub(crate) fn validate_complexity_fission_semantics(
    campaign: &crate::domain::Campaign,
    parent: &crate::domain::GestaltPersonaState,
    preview: &crate::domain::GestaltFissionPreview,
    title: ElaboratorTitle,
    jurisdiction_location_id: &str,
    target_actionable_gain: u8,
) -> Result<()> {
    if complexity_operation(title) != WorldComplexityOperation::Fission {
        return Err(anyhow!(
            "complexity fission qualification names a non-fission elaborator"
        ));
    }
    if preview.partition_axis != complexity_partition_axis(title) {
        return Err(anyhow!(
            "complexity fission partition axis does not belong to its elaborator title"
        ));
    }
    let actual_gain = preview.children.len().saturating_sub(2);
    if actual_gain != usize::from(target_actionable_gain) {
        return Err(anyhow!(
            "complexity fission must add exactly {target_actionable_gain} qualified subjects"
        ));
    }
    let residual = preview
        .children
        .iter()
        .find(|child| child.id == preview.residual_child_id)
        .ok_or_else(|| anyhow!("complexity fission lost its residual child"))?;
    if residual.home_location_id != parent.home_location_id
        || residual.shared_capabilities != parent.shared_capabilities
        || residual.shared_knowledge != parent.shared_knowledge
        || residual.goals != parent.goals
        || residual.pressures != parent.pressures
    {
        return Err(anyhow!(
            "the residual child is unresolved background and cannot claim new semantic detail"
        ));
    }

    let residual_ids = residual_gestalt_ids(campaign);
    let established_names = campaign
        .gestalts
        .values()
        .filter(|gestalt| !residual_ids.contains(&gestalt.id))
        .map(|gestalt| crate::resolution::public_identity_key(&gestalt.name))
        .collect::<BTreeSet<_>>();
    let allowed_locations = locations_in_jurisdiction(campaign, jurisdiction_location_id)
        .into_iter()
        .collect::<BTreeSet<_>>();
    let mut child_names = BTreeSet::new();
    let mut sibling_capabilities = BTreeSet::new();
    let mut sibling_knowledge = BTreeSet::new();
    let mut sibling_goals = BTreeSet::new();
    let mut sibling_pressures = BTreeSet::new();
    let mut specific_homes = BTreeSet::new();
    for child in preview
        .children
        .iter()
        .filter(|child| child.id != preview.residual_child_id)
    {
        let name_key = crate::resolution::public_identity_key(&child.name);
        if name_key.is_empty()
            || established_names.contains(&name_key)
            || !child_names.insert(name_key)
        {
            return Err(anyhow!(
                "every fission child needs a distinct public population identity"
            ));
        }
        if !allowed_locations.contains(&child.home_location_id) {
            return Err(anyhow!(
                "fission child {} is outside its assigned jurisdiction",
                child.id
            ));
        }
        if preview.partition_axis != crate::domain::AgencyAxis::Geography
            && child.home_location_id != parent.home_location_id
        {
            return Err(anyhow!(
                "only a geography fission may distribute children to other admitted places"
            ));
        }
        specific_homes.insert(child.home_location_id.clone());
        let capability_additions = child
            .shared_capabilities
            .difference(&parent.shared_capabilities)
            .cloned()
            .collect::<Vec<_>>();
        let knowledge_additions = child
            .shared_knowledge
            .difference(&parent.shared_knowledge)
            .cloned()
            .collect::<Vec<_>>();
        let goal_additions = child
            .goals
            .iter()
            .filter(|goal| !parent.goals.contains(goal))
            .cloned()
            .collect::<Vec<_>>();
        let pressure_additions = child
            .pressures
            .iter()
            .filter(|pressure| !parent.pressures.contains(pressure))
            .cloned()
            .collect::<Vec<_>>();
        let semantic_dimensions = [
            !capability_additions.is_empty(),
            !knowledge_additions.is_empty(),
            !goal_additions.is_empty(),
            !pressure_additions.is_empty(),
        ]
        .into_iter()
        .filter(|present| *present)
        .count();
        if semantic_dimensions < 2
            || goal_additions.len() > 1
            || pressure_additions.len() > 1
            || capability_additions.len() > 2
            || knowledge_additions.len() > 2
        {
            return Err(anyhow!(
                "each specific child needs two or more bounded causal dimensions without template padding"
            ));
        }
        let title_shape_is_valid = match title {
            ElaboratorTitle::Patina => {
                !goal_additions.is_empty() && !capability_additions.is_empty()
            }
            ElaboratorTitle::Charter | ElaboratorTitle::Ledger => {
                !goal_additions.is_empty()
                    && !pressure_additions.is_empty()
                    && (!capability_additions.is_empty() || !knowledge_additions.is_empty())
            }
            ElaboratorTitle::Ember => {
                !pressure_additions.is_empty()
                    && (!capability_additions.is_empty() || !knowledge_additions.is_empty())
            }
            ElaboratorTitle::Numen => {
                !goal_additions.is_empty()
                    && !pressure_additions.is_empty()
                    && !knowledge_additions.is_empty()
            }
            ElaboratorTitle::Hearth | ElaboratorTitle::Tangle | ElaboratorTitle::Veil => false,
        };
        if !title_shape_is_valid {
            return Err(anyhow!(
                "specific child additions do not satisfy the assigned elaborator's causal lane"
            ));
        }
        let capability_keys = validate_new_semantic_texts(
            "child capability",
            capability_additions.iter(),
            parent.shared_capabilities.iter(),
            campaign
                .gestalts
                .values()
                .flat_map(|gestalt| gestalt.shared_capabilities.iter()),
        )?;
        let knowledge_keys = validate_new_semantic_texts(
            "child knowledge",
            knowledge_additions.iter(),
            parent.shared_knowledge.iter(),
            campaign
                .gestalts
                .values()
                .flat_map(|gestalt| gestalt.shared_knowledge.iter()),
        )?;
        let goal_keys = validate_new_semantic_texts(
            "child goal",
            goal_additions.iter(),
            parent.goals.iter(),
            campaign
                .gestalts
                .values()
                .flat_map(|gestalt| gestalt.goals.iter()),
        )?;
        let pressure_keys = validate_new_semantic_texts(
            "child pressure",
            pressure_additions.iter(),
            parent.pressures.iter(),
            campaign
                .gestalts
                .values()
                .flat_map(|gestalt| gestalt.pressures.iter()),
        )?;
        if !capability_keys.is_disjoint(&sibling_capabilities)
            || !knowledge_keys.is_disjoint(&sibling_knowledge)
            || !goal_keys.is_disjoint(&sibling_goals)
            || !pressure_keys.is_disjoint(&sibling_pressures)
        {
            return Err(anyhow!(
                "specific fission children must not repeat one another's causal additions"
            ));
        }
        sibling_capabilities.extend(capability_keys);
        sibling_knowledge.extend(knowledge_keys);
        sibling_goals.extend(goal_keys);
        sibling_pressures.extend(pressure_keys);
    }
    if preview.partition_axis == crate::domain::AgencyAxis::Geography
        && allowed_locations.len() > 1
        && specific_homes.len() < 2
    {
        return Err(anyhow!(
            "geography fission must distribute specific children across at least two admitted places"
        ));
    }
    Ok(())
}

pub(crate) fn validate_complexity_individuation_semantics(
    campaign: &crate::domain::Campaign,
    title: ElaboratorTitle,
    individuation: &crate::domain::GestaltIndividuation,
) -> Result<()> {
    if !matches!(
        title,
        ElaboratorTitle::Hearth | ElaboratorTitle::Tangle | ElaboratorTitle::Veil
    ) {
        return Err(anyhow!(
            "only Hearth, Tangle, or Veil may own a complexity individuation"
        ));
    }
    let member = &individuation.member;
    let parent = &campaign.gestalts[&individuation.gestalt_id];
    let dimensions = [
        !member.capability_additions.is_empty(),
        !member.knowledge_additions.is_empty(),
        !member.equipment.is_empty(),
        !member.obligations.is_empty(),
        !member.relationships.is_empty(),
        !member.goals.is_empty(),
        !member.memories.is_empty(),
    ]
    .into_iter()
    .filter(|present| *present)
    .count();
    if dimensions < 3 || member.goals.is_empty() || member.memories.is_empty() {
        return Err(anyhow!(
            "complexity individuation needs a goal, a concrete memory, and at least one additional identity-bearing difference"
        ));
    }
    validate_new_semantic_texts(
        "member goal",
        member.goals.iter(),
        parent.goals.iter(),
        campaign
            .gestalt_members
            .values()
            .flat_map(|existing| existing.goals.iter()),
    )?;
    validate_new_semantic_texts(
        "member memory",
        member.memories.iter(),
        std::iter::empty(),
        campaign
            .gestalt_members
            .values()
            .flat_map(|existing| existing.memories.iter()),
    )?;
    match title {
        ElaboratorTitle::Hearth if member.obligations.is_empty() => Err(anyhow!(
            "Hearth individuation requires one concrete care or belonging obligation"
        )),
        ElaboratorTitle::Tangle if member.relationships.is_empty() => Err(anyhow!(
            "Tangle individuation requires a relationship to an existing canonical subject"
        )),
        ElaboratorTitle::Veil if member.knowledge_additions.is_empty() => Err(anyhow!(
            "Veil individuation requires uneven knowledge not held by the parent population"
        )),
        _ => Ok(()),
    }
}

impl WorldComplexityTool<'_> {
    fn materialize_proposal(
        &self,
        mutation: WorldComplexityMutationDraft,
    ) -> Result<WorldComplexityProposal> {
        let parent = &self.campaign.gestalts[self.parent_gestalt_id];
        match mutation {
            WorldComplexityMutationDraft::Fission {
                children,
                residual_child_id,
                member_child_assignments,
                resource_child_assignments,
            } => {
                if self.operation != WorldComplexityOperation::Fission {
                    return Err(anyhow!("complexity action changed its assigned operation"));
                }
                let child_ids = children
                    .iter()
                    .map(|child| child.id.clone())
                    .collect::<BTreeSet<_>>();
                let partition_values = children
                    .iter()
                    .map(|child| child.partition_value.trim().to_ascii_lowercase())
                    .collect::<BTreeSet<_>>();
                let expected_members = self
                    .campaign
                    .gestalt_members
                    .values()
                    .filter(|member| member.gestalt_id == self.parent_gestalt_id)
                    .map(|member| member.id.clone())
                    .collect::<BTreeSet<_>>();
                let assigned_members = member_child_assignments
                    .keys()
                    .cloned()
                    .collect::<BTreeSet<_>>();
                if child_ids.len() != children.len()
                    || partition_values.len() != children.len()
                    || assigned_members != expected_members
                {
                    return Err(anyhow!(
                        "fission draft must use unique children and partitions and assign every exact member"
                    ));
                }
                let child_states = children
                    .iter()
                    .map(|child| {
                        let mut state = parent.clone();
                        state.id = child.id.clone();
                        state.name = child.name.clone();
                        state.version = 0;
                        state.home_location_id = child.home_location_id.clone();
                        state
                            .shared_capabilities
                            .extend(child.capability_additions.iter().cloned());
                        state
                            .shared_knowledge
                            .extend(child.knowledge_additions.iter().cloned());
                        state.goals.extend(child.goals_add.iter().cloned());
                        state.pressures.extend(child.pressures_add.iter().cloned());
                        state.resources = resource_child_assignments
                            .iter()
                            .filter(|(_, child_id)| *child_id == &child.id)
                            .map(|(resource, _)| resource.clone())
                            .collect();
                        state
                    })
                    .collect::<Vec<_>>();
                let preview = crate::domain::GestaltFissionPreview {
                    schema: "ghostlight.gestalt_fission_preview.v1".into(),
                    campaign_id: self.campaign.id,
                    expected_world_revision: self.campaign.revision,
                    parent_gestalt_id: self.parent_gestalt_id.into(),
                    partition_axis: self.partition_axis.clone(),
                    child_partition_values: children
                        .into_iter()
                        .map(|child| (child.id, child.partition_value))
                        .collect(),
                    children: child_states,
                    residual_child_id,
                    member_child_assignments,
                    resource_child_assignments,
                    evidence_receipt_ids: Vec::new(),
                    gaps: Vec::new(),
                    canon_candidates: Vec::new(),
                    requires_approval: true,
                };
                crate::resolution::validate_elaborated_fission(self.campaign, &preview)?;
                validate_complexity_fission_semantics(
                    self.campaign,
                    parent,
                    &preview,
                    self.title,
                    self.jurisdiction_location_id,
                    self.target_actionable_gain,
                )?;
                Ok(WorldComplexityProposal::Fission {
                    preview,
                    qualification: WorldComplexityFissionQualification {
                        schema: "ghostlight.world_complexity_fission_qualification.v1".into(),
                        title: self.title,
                        jurisdiction_location_id: self.jurisdiction_location_id.into(),
                        target_actionable_gain: self.target_actionable_gain,
                        semantic: WorldComplexitySemanticQualification::default(),
                    },
                })
            }
            WorldComplexityMutationDraft::Individuate { member } => {
                if self.operation != WorldComplexityOperation::Individuate {
                    return Err(anyhow!("complexity action changed its assigned operation"));
                }
                let location_id = self.campaign.agency_profiles[self.parent_gestalt_id]
                    .location_ids
                    .iter()
                    .next()
                    .ok_or_else(|| anyhow!("complexity parent has no exact location"))?
                    .clone();
                let individuation = crate::domain::GestaltIndividuation {
                    gestalt_id: self.parent_gestalt_id.into(),
                    expected_gestalt_version: parent.version,
                    member: crate::domain::GestaltMemberDelta {
                        schema: "ghostlight.gestalt_member_delta.v1".into(),
                        id: member.id,
                        gestalt_id: self.parent_gestalt_id.into(),
                        version: 0,
                        name: member.name,
                        capability_additions: member.capability_additions,
                        capability_removals: member.capability_removals,
                        knowledge_additions: member.knowledge_additions,
                        knowledge_removals: member.knowledge_removals,
                        equipment: member.equipment,
                        conditions: member.conditions,
                        obligations: member.obligations,
                        relationships: member.relationships,
                        goals: member.goals,
                        memories: member.memories,
                        last_location_id: Some(location_id.clone()),
                        materialized_actor_id: None,
                        last_relevant_revision: 0,
                        relevance_lease_until_revision: 0,
                    },
                    location_id,
                };
                crate::resolution::validate_gestalt_individuation(self.campaign, &individuation)?;
                validate_complexity_individuation_semantics(
                    self.campaign,
                    self.title,
                    &individuation,
                )?;
                Ok(WorldComplexityProposal::Individuate {
                    individuation,
                    qualification: WorldComplexityIndividuationQualification {
                        schema: "ghostlight.world_complexity_individuation_qualification.v1".into(),
                        title: self.title,
                        jurisdiction_location_id: self.jurisdiction_location_id.into(),
                        semantic: WorldComplexitySemanticQualification::default(),
                    },
                })
            }
        }
    }
}

#[async_trait]
impl crate::agent::ModelAgentTool for WorldComplexityTool<'_> {
    type Action = WorldComplexityAction;
    type Output = WorldComplexityProposal;
    type Finding = WorldComplexityFinding;

    fn action_schema(&self) -> std::result::Result<serde_json::Value, String> {
        let mut schema = serde_json::to_value(schema_for!(WorldComplexityAction))
            .map_err(|error| error.to_string())?;
        schema["properties"]["schema"] = exact_schema("ghostlight.world_complexity_action.v1");
        if self.operation == WorldComplexityOperation::Fission {
            let allowed_locations =
                locations_in_jurisdiction(self.campaign, self.jurisdiction_location_id);
            let child = &mut schema["$defs"]["WorldComplexityChildDraft"];
            child["properties"]["home_location_id"] = serde_json::json!({
                "type":"string",
                "enum":allowed_locations,
            });
            child["properties"]["id"]["minLength"] = serde_json::json!(1);
            child["properties"]["id"]["maxLength"] = serde_json::json!(160);
            child["properties"]["name"]["minLength"] = serde_json::json!(1);
            child["properties"]["name"]["maxLength"] = serde_json::json!(160);
            for field in ["capability_additions", "knowledge_additions"] {
                child["properties"][field]["maxItems"] = serde_json::json!(2);
                child["properties"][field]["items"]["minLength"] = serde_json::json!(1);
                child["properties"][field]["items"]["maxLength"] = serde_json::json!(300);
            }
            for field in ["goals_add", "pressures_add"] {
                child["properties"][field]["maxItems"] = serde_json::json!(1);
                child["properties"][field]["items"]["minLength"] = serde_json::json!(1);
                child["properties"][field]["items"]["maxLength"] = serde_json::json!(300);
            }
        } else {
            let member = &mut schema["$defs"]["WorldComplexityMemberDraft"];
            member["properties"]["name"]["minLength"] = serde_json::json!(1);
            member["properties"]["name"]["maxLength"] = serde_json::json!(160);
            for field in ["goals", "memories"] {
                member["properties"][field]["minItems"] = serde_json::json!(1);
                member["properties"][field]["maxItems"] = serde_json::json!(8);
            }
            match self.title {
                ElaboratorTitle::Hearth => {
                    member["properties"]["obligations"]["minItems"] = serde_json::json!(1);
                }
                ElaboratorTitle::Tangle => {
                    member["properties"]["relationships"]["minProperties"] = serde_json::json!(1);
                }
                ElaboratorTitle::Veil => {
                    member["properties"]["knowledge_additions"]["minItems"] = serde_json::json!(1);
                }
                _ => {}
            }
        }
        let variants = schema
            .pointer_mut("/$defs/WorldComplexityMutationDraft/oneOf")
            .and_then(serde_json::Value::as_array_mut)
            .ok_or_else(|| "complexity action schema has no mutation variants".to_owned())?;
        let expected_operation = match self.operation {
            WorldComplexityOperation::Fission => "fission",
            WorldComplexityOperation::Individuate => "individuate",
        };
        let selected = variants
            .iter()
            .position(|variant| {
                variant
                    .pointer("/properties/operation/const")
                    .and_then(serde_json::Value::as_str)
                    == Some(expected_operation)
            })
            .ok_or_else(|| {
                format!("complexity action schema has no {expected_operation} variant")
            })?;
        let mut selected = variants.remove(selected);
        if self.operation == WorldComplexityOperation::Fission {
            let children = selected
                .pointer_mut("/properties/children")
                .and_then(serde_json::Value::as_object_mut)
                .ok_or_else(|| "complexity fission schema has no children".to_owned())?;
            let child_count = usize::from(self.target_actionable_gain).saturating_add(2);
            children.insert("minItems".into(), serde_json::json!(child_count));
            children.insert("maxItems".into(), serde_json::json!(child_count));
            let parent = &self.campaign.gestalts[self.parent_gestalt_id];
            selected["properties"]["resource_child_assignments"] =
                exact_keyed_string_map_schema(parent.resources.iter().cloned());
            selected["properties"]["member_child_assignments"] = exact_keyed_string_map_schema(
                self.campaign
                    .gestalt_members
                    .values()
                    .filter(|member| member.gestalt_id == self.parent_gestalt_id)
                    .map(|member| member.id.clone()),
            );
        }
        *variants = vec![selected];
        Ok(schema)
    }

    fn initial_context_snapshot(&self) -> Option<serde_json::Value> {
        Some(self.workbench.clone())
    }

    async fn invoke(
        &mut self,
        action: Self::Action,
        _context: &crate::agent::ModelAgentToolContext,
    ) -> crate::agent::ModelAgentToolOutcome<Self::Output, Self::Finding> {
        if action.schema != "ghostlight.world_complexity_action.v1" {
            return crate::agent::ModelAgentToolOutcome::Rejected {
                finding: WorldComplexityFinding {
                    diagnostic: "complexity action schema is unsupported".into(),
                },
                receipts: Vec::new(),
            };
        }
        match self.materialize_proposal(action.mutation) {
            Ok(proposal) => crate::agent::ModelAgentToolOutcome::Accepted {
                output: proposal,
                receipts: Vec::new(),
            },
            Err(error) => crate::agent::ModelAgentToolOutcome::Rejected {
                finding: WorldComplexityFinding {
                    diagnostic: error.to_string(),
                },
                receipts: Vec::new(),
            },
        }
    }
}

#[async_trait]
impl ElaborationSubAgentPort<WorldComplexityProposal> for ModelWorldComplexityWorker {
    async fn invoke(
        &self,
        invocation: ElaborationSubAgentInvocation,
    ) -> std::result::Result<
        ElaborationSubAgentOutput<WorldComplexityProposal>,
        ElaborationSubAgentFailure,
    > {
        if invocation.wave != self.wave {
            return Err(ElaborationSubAgentFailure {
                diagnostic: "complexity invocation does not match its frozen round".into(),
                model_stage_receipts: Vec::new(),
            });
        }
        let parent_id =
            self.parent_for(&invocation.dispatch)
                .map_err(|error| ElaborationSubAgentFailure {
                    diagnostic: error.to_string(),
                    model_stage_receipts: Vec::new(),
                })?;
        let parent = &self.campaign.gestalts[parent_id];
        let profile = &self.campaign.agency_profiles[parent_id];
        let axis = complexity_partition_axis(invocation.dispatch.title);
        let operation = complexity_operation(invocation.dispatch.title);
        let location_id =
            profile
                .location_ids
                .iter()
                .next()
                .ok_or_else(|| ElaborationSubAgentFailure {
                    diagnostic: "complexity parent has no exact location".into(),
                    model_stage_receipts: Vec::new(),
                })?;
        let jurisdiction_id = self.parent_jurisdiction_ids.get(parent_id).ok_or_else(|| {
            ElaborationSubAgentFailure {
                diagnostic: "complexity parent lost its session jurisdiction".into(),
                model_stage_receipts: Vec::new(),
            }
        })?;
        let target_actionable_gain = *self
            .target_actionable_gains
            .get(&invocation.dispatch.ordinal)
            .ok_or_else(|| ElaborationSubAgentFailure {
                diagnostic: "complexity dispatch has no bounded actionable gain".into(),
                model_stage_receipts: Vec::new(),
            })?;
        let session_id = elaborator_session_id(invocation.dispatch.title, jurisdiction_id);
        let assignment = match operation {
            WorldComplexityOperation::Fission => serde_json::json!({
                "operation":"fission_gestalt",
                "parent_gestalt_id":parent_id,
                "partition_axis":axis,
                "target_actionable_gain":target_actionable_gain,
                "exact_child_count":usize::from(target_actionable_gain) + 2,
                "requirements":[
                    "Return a ghostlight.world_complexity_action.v1 envelope whose mutation has operation=fission.",
                    "Create the exact assigned number of non-overlapping children with distinct partition values.",
                    "Exactly one residual child must have partition value other/unknown, remain at the parent home, and provide empty capability_additions, knowledge_additions, goals_add, and pressures_add.",
                    "Every specific child needs a unique public name and title-appropriate causal additions. Patina adds a concrete practice plus a goal; Charter and Ledger add a goal, pressure, and operational capability or knowledge; Ember adds pressure plus an operational capability or knowledge; Numen adds a goal, pressure, and uneven numinous knowledge.",
                    "A geography fission distributes specific children among admitted places in this jurisdiction. Other axes keep the parent home.",
                    "Assign every scarce resource and named member exactly once; materialized people retain their Actor identity while changing population affiliation.",
                    "Do not restate inherited capabilities, knowledge, goals, pressures, evidence, or approval fields in the additive fields."
                ]
            }),
            WorldComplexityOperation::Individuate => serde_json::json!({
                "operation":"individuate_gestalt_member",
                "parent_gestalt_id":parent_id,
                "location_id":location_id,
                "requirements":[
                    "Return a ghostlight.world_complexity_action.v1 envelope whose mutation has operation=individuate and one compact member draft.",
                    "The deterministic tool owns schema, exact parent id, parent version, assigned location, materialization state, and relevance revisions; those fields are absent from your draft.",
                    "Create one consequential named person with a distinct full public name, at least one new goal, one situated memory, and one additional identity-bearing difference grounded in the parent state.",
                    "Hearth requires a concrete care or belonging obligation. Tangle requires leverage expressed as a relationship to one supplied canonical target. Veil requires uneven knowledge not held by the parent population.",
                    "Use a new local member id without the member: prefix.",
                    "Do not create quota names, unsupported relationships, or decorative biography."
                ]
            }),
        };
        let instructions = format!(
            "You are {}, one titled elaborator in an iterative world-complexity session. {} Perform exactly the assigned structural mutation where the frozen world supports consequential agency. Prefer one strong change over quota padding. Your proposal cannot commit, approve itself, or invent evidence.",
            invocation.dispatch.title.display_name(),
            invocation.dispatch.title.mandate(),
        );
        let residual_ids = residual_gestalt_ids(&self.campaign);
        let (public_identity_namespace, established_public_names) = match operation {
            WorldComplexityOperation::Fission => (
                "population",
                self.campaign
                    .gestalts
                    .values()
                    .filter(|gestalt| !residual_ids.contains(&gestalt.id))
                    .map(|gestalt| gestalt.name.as_str())
                    .collect::<Vec<_>>(),
            ),
            WorldComplexityOperation::Individuate => (
                "person",
                self.campaign
                    .actors
                    .values()
                    .map(|actor| actor.name.as_str())
                    .chain(
                        self.campaign
                            .gestalt_members
                            .values()
                            .map(|member| member.name.as_str()),
                    )
                    .collect::<Vec<_>>(),
            ),
        };
        let workbench = serde_json::json!({
            "schema":"ghostlight.world_complexity_workbench.v1",
            "frozen_world":{
                "campaign_id":self.campaign.id,
                "world_revision":self.campaign.revision,
            },
            "assignment":assignment,
            "session_jurisdiction_id":jurisdiction_id,
            "admitted_home_location_ids":locations_in_jurisdiction(&self.campaign, jurisdiction_id),
            "canonical_relationship_target_ids":self.campaign.civic_systems.get(jurisdiction_id)
                .into_iter()
                .flat_map(|civic| civic.governing_institution_ids.iter().chain(civic.resident_population_ids.iter()))
                .filter(|id|id.as_str()!=parent_id)
                .take(64).cloned().collect::<Vec<_>>(),
            "public_identity_namespace":public_identity_namespace,
            "established_names_in_this_identity_namespace":established_public_names,
            "required_resource_assignment_ids":parent.resources,
            "required_member_assignment_ids":self.campaign.gestalt_members.values()
                .filter(|member|member.gestalt_id == parent_id)
                .map(|member|member.id.clone()).collect::<Vec<_>>(),
            "parent":parent,
            "parent_profile":profile,
            "parent_members":self.campaign.gestalt_members.values()
                .filter(|member|member.gestalt_id == parent_id).collect::<Vec<_>>(),
            "local_relations":self.campaign.agency_relations.values()
                .filter(|relation|relation.from_subject_id == parent_id || relation.to_subject_id == parent_id)
                .collect::<Vec<_>>(),
            "compacted_session":self.session_checkpoints.get(&session_id),
        });
        let mut tool = WorldComplexityTool {
            campaign: &self.campaign,
            parent_gestalt_id: parent_id,
            title: invocation.dispatch.title,
            operation,
            partition_axis: axis,
            jurisdiction_location_id: jurisdiction_id,
            target_actionable_gain,
            workbench,
        };
        let spec = crate::agent::ModelAgentSpec {
            stage: format!(
                "world-complexity-{}-{}",
                invocation
                    .dispatch
                    .title
                    .display_name()
                    .to_ascii_lowercase(),
                match operation {
                    WorldComplexityOperation::Fission => "fission",
                    WorldComplexityOperation::Individuate => "individuate",
                }
            ),
            model: match invocation.dispatch.title {
                ElaboratorTitle::Charter | ElaboratorTitle::Tangle => {
                    crate::model::MODEL_BALANCED.into()
                }
                ElaboratorTitle::Numen => crate::model::MODEL_CAPABLE.into(),
                _ => crate::model::MODEL_FAST.into(),
            },
            snapshot_binding: crate::legacy_transition::digest_serializable(&(
                "ghostlight.world_complexity_invocation.v1",
                &self.wave,
                &invocation.dispatch,
                parent_id,
            ))
            .map_err(|error| ElaborationSubAgentFailure {
                diagnostic: error.to_string(),
                model_stage_receipts: Vec::new(),
            })?,
            instructions,
            source_receipt_ids: Vec::new(),
            temperature: Some(0.35),
            max_output_tokens: Some(4_000),
            max_steps: 3,
        };
        match crate::agent::run_model_agent(self.model.as_ref(), &spec, &mut tool).await {
            Ok(run) => Ok(ElaborationSubAgentOutput {
                proposal: run.output,
                model_stage_receipts: run.receipts,
            }),
            Err(error) => Err(ElaborationSubAgentFailure {
                diagnostic: error.message,
                model_stage_receipts: error.receipts,
            }),
        }
    }
}

#[derive(Clone, Debug)]
enum WorldElaborationAssignment {
    PatinaPlace {
        child_location_id: String,
    },
    PatinaOriginRoute {
        child_location_id: String,
        route_id: String,
    },
    PatinaReturnRoute {
        child_location_id: String,
        route_id: String,
    },
    PreserveCivicSystem {
        system: crate::domain::CivicSystemManifest,
    },
    AddFact {
        fact_id: String,
    },
    AddPoliticalRelation {
        relation_id: String,
        allowed_subject_ids: BTreeSet<String>,
    },
}

impl WorldElaborationAssignment {
    fn for_dispatch(
        campaign: &crate::domain::Campaign,
        target_location_id: &str,
        dispatch: &ElaborationDispatch,
    ) -> Result<Self> {
        let title_count = dispatch.title_dispatch_count;
        let title_name = dispatch.title.display_name().to_ascii_lowercase();
        match dispatch.title {
            ElaboratorTitle::Patina => {
                let cycle = (title_count.saturating_sub(1) / 3).saturating_add(1);
                let child_location_id = format!("elab:{target_location_id}:patina-place:{cycle}");
                match title_count.saturating_sub(1) % 3 {
                    0 => Ok(Self::PatinaPlace { child_location_id }),
                    1 => Ok(Self::PatinaOriginRoute {
                        child_location_id,
                        route_id: format!("elab:{target_location_id}:patina-route:{cycle}:out"),
                    }),
                    _ => Ok(Self::PatinaReturnRoute {
                        child_location_id,
                        route_id: format!("elab:{target_location_id}:patina-route:{cycle}:back"),
                    }),
                }
            }
            ElaboratorTitle::Charter if title_count % 2 == 1 => {
                let mut system = campaign
                    .civic_systems
                    .get(target_location_id)
                    .cloned()
                    .ok_or_else(|| anyhow!("Charter requires an admitted civic system"))?;
                system.version = system.version.saturating_add(1);
                system.semantic_verification_receipt_id.clear();
                Ok(Self::PreserveCivicSystem { system })
            }
            ElaboratorTitle::Tangle if title_count % 2 == 1 => {
                let civic = campaign
                    .civic_systems
                    .get(target_location_id)
                    .ok_or_else(|| anyhow!("Tangle requires an admitted civic system"))?;
                let allowed_subject_ids = civic
                    .governing_institution_ids
                    .iter()
                    .chain(civic.resident_population_ids.iter())
                    .cloned()
                    .collect::<BTreeSet<_>>();
                if allowed_subject_ids.len() < 2 {
                    return Err(anyhow!("Tangle requires at least two exact civic subjects"));
                }
                Ok(Self::AddPoliticalRelation {
                    relation_id: format!(
                        "elab:{target_location_id}:{title_name}-relation:{title_count}"
                    ),
                    allowed_subject_ids,
                })
            }
            _ => Ok(Self::AddFact {
                fact_id: format!("elab:{target_location_id}:{title_name}-fact:{title_count}"),
            }),
        }
    }

    fn instruction(&self, target_location_id: &str) -> Result<String> {
        let instruction = match self {
            Self::PatinaPlace { child_location_id } => format!(
                "Submit exactly one add_place operation. Its id must be {child_location_id:?}, container_id must be {target_location_id:?}, name must be concrete, and persistent_features must contain one or more durable visible local details. This is Patina's texture-bearing place; a later assigned operation supplies its routes."
            ),
            Self::PatinaOriginRoute {
                child_location_id,
                route_id,
            } => format!(
                "Submit exactly one add_route operation from {target_location_id:?}. route_id must be {route_id:?}; destination_id must be {child_location_id:?}; distance must be \"a short internal path\"; travel_minutes must be 3."
            ),
            Self::PatinaReturnRoute {
                child_location_id,
                route_id,
            } => format!(
                "Submit exactly one add_route operation from {child_location_id:?}. route_id must be {route_id:?}; destination_id must be {target_location_id:?}; distance must be \"a short internal path\"; travel_minutes must be 3."
            ),
            Self::PreserveCivicSystem { system } => format!(
                "Submit exactly one set_civic_system operation containing this exact next-version manifest, byte-for-byte in meaning and with an empty semantic_verification_receipt_id: {}. Do not add or omit an ID; the independent verifier owns semantic acceptance.",
                serde_json::to_string(system)?
            ),
            Self::AddFact { fact_id } => format!(
                "Submit exactly one add_fact operation. The fact id must be {fact_id:?}; scope must be branch_local; evidence_receipt_ids must be empty; discoverable_at_location_ids must contain exactly {target_location_id:?}. Invent one concrete, consequential, title-appropriate statement that does not duplicate an existing fact."
            ),
            Self::AddPoliticalRelation {
                relation_id,
                allowed_subject_ids,
            } => format!(
                "Submit exactly one add_local_relation operation. Its schema must be {relation_schema:?}; id must be {relation_id:?}; both distinct endpoints must come from {}; kind must be alliance, rivalry, trade, communication, command, or coercion; strength must be 1 through 100; active must be true; evidence_receipt_ids must be empty.",
                serde_json::to_string(allowed_subject_ids)?,
                relation_schema = crate::domain::AgencyRelation::SCHEMA,
            ),
        };
        Ok(instruction)
    }

    fn action_schema(&self, target_location_id: &str) -> Result<serde_json::Value> {
        let mut schema = serde_json::to_value(schema_for!(WorldElaborationProposal))?;
        schema["properties"]["schema"] = serde_json::json!({
            "type":"string",
            "const":"ghostlight.world_elaboration_proposal.v1",
        });
        let operation = schema
            .pointer_mut("/$defs/WorldElaborationOperation")
            .ok_or_else(|| anyhow!("world elaboration schema has no operation definition"))?;
        let variants = operation
            .get_mut("oneOf")
            .and_then(serde_json::Value::as_array_mut)
            .ok_or_else(|| anyhow!("world elaboration operation schema has no typed variants"))?;
        let expected_type = self.operation_type();
        let index = variants
            .iter()
            .position(|variant| {
                variant
                    .pointer("/properties/type/const")
                    .and_then(serde_json::Value::as_str)
                    == Some(expected_type)
            })
            .ok_or_else(|| {
                anyhow!("world elaboration schema has no {expected_type} operation variant")
            })?;
        let selected = variants.remove(index);
        *variants = vec![selected];

        match self {
            Self::PatinaPlace { child_location_id } => {
                let branch = selected_operation_branch(&mut schema)?;
                branch["properties"]["id"] = exact_schema(child_location_id);
                branch["properties"]["container_id"] = exact_schema(target_location_id);
                branch["properties"]["name"]["minLength"] = serde_json::json!(1);
                branch["properties"]["persistent_features"]["minItems"] = serde_json::json!(1);
                branch["required"] = serde_json::json!([
                    "type",
                    "id",
                    "name",
                    "container_id",
                    "persistent_features"
                ]);
            }
            Self::PatinaOriginRoute {
                child_location_id,
                route_id,
            } => {
                let branch = selected_operation_branch(&mut schema)?;
                branch["properties"]["origin_location_id"] = exact_schema(target_location_id);
                branch["properties"]["route_id"] = exact_schema(route_id);
                constrain_exact_object(
                    schema.pointer_mut("/$defs/Route").ok_or_else(|| {
                        anyhow!("world elaboration schema has no route definition")
                    })?,
                    &serde_json::json!({
                        "destination_id":child_location_id,
                        "distance":"a short internal path",
                        "travel_minutes":3,
                    }),
                )?;
            }
            Self::PatinaReturnRoute {
                child_location_id,
                route_id,
            } => {
                let branch = selected_operation_branch(&mut schema)?;
                branch["properties"]["origin_location_id"] = exact_schema(child_location_id);
                branch["properties"]["route_id"] = exact_schema(route_id);
                constrain_exact_object(
                    schema.pointer_mut("/$defs/Route").ok_or_else(|| {
                        anyhow!("world elaboration schema has no route definition")
                    })?,
                    &serde_json::json!({
                        "destination_id":target_location_id,
                        "distance":"a short internal path",
                        "travel_minutes":3,
                    }),
                )?;
            }
            Self::PreserveCivicSystem { system } => constrain_exact_object(
                schema
                    .pointer_mut("/$defs/CivicSystemManifest")
                    .ok_or_else(|| anyhow!("world elaboration schema has no civic definition"))?,
                &serde_json::to_value(system)?,
            )?,
            Self::AddFact { fact_id } => {
                let fact = schema
                    .pointer_mut("/$defs/WorldFact")
                    .ok_or_else(|| anyhow!("world elaboration schema has no fact definition"))?;
                fact["properties"]["id"] = exact_schema(fact_id);
                fact["properties"]["scope"] = exact_schema("branch_local");
                fact["properties"]["evidence_receipt_ids"] = exact_schema(&Vec::<String>::new());
                fact["properties"]["discoverable_at_location_ids"] =
                    exact_schema(&vec![target_location_id]);
                fact["properties"]["statement"]["minLength"] = serde_json::json!(1);
                fact["properties"]["statement"]["maxLength"] = serde_json::json!(500);
                fact["required"] = serde_json::json!([
                    "id",
                    "statement",
                    "scope",
                    "evidence_receipt_ids",
                    "discoverable_at_location_ids"
                ]);
                fact["additionalProperties"] = serde_json::json!(false);
            }
            Self::AddPoliticalRelation {
                relation_id,
                allowed_subject_ids,
            } => {
                let relation = schema.pointer_mut("/$defs/AgencyRelation").ok_or_else(|| {
                    anyhow!("world elaboration schema has no relation definition")
                })?;
                relation["properties"]["schema"] =
                    exact_schema(crate::domain::AgencyRelation::SCHEMA);
                relation["properties"]["id"] = exact_schema(relation_id);
                let allowed = allowed_subject_ids.iter().cloned().collect::<Vec<_>>();
                relation["properties"]["from_subject_id"] = serde_json::json!({
                    "type":"string",
                    "enum":allowed,
                });
                relation["properties"]["to_subject_id"] = serde_json::json!({
                    "type":"string",
                    "enum":allowed_subject_ids.iter().cloned().collect::<Vec<_>>(),
                });
                relation["properties"]["kind"] = serde_json::json!({
                    "type":"string",
                    "enum":["alliance", "rivalry", "trade", "communication", "command", "coercion"],
                });
                relation["properties"]["active"] = exact_schema(&true);
                relation["properties"]["evidence_receipt_ids"] =
                    exact_schema(&Vec::<String>::new());
                relation["additionalProperties"] = serde_json::json!(false);
            }
        }
        Ok(schema)
    }

    fn operation_type(&self) -> &'static str {
        match self {
            Self::PatinaPlace { .. } => "add_place",
            Self::PatinaOriginRoute { .. } | Self::PatinaReturnRoute { .. } => "add_route",
            Self::PreserveCivicSystem { .. } => "set_civic_system",
            Self::AddFact { .. } => "add_fact",
            Self::AddPoliticalRelation { .. } => "add_local_relation",
        }
    }

    fn validate(
        &self,
        campaign: &crate::domain::Campaign,
        target_location_id: &str,
        proposal: &WorldElaborationProposal,
    ) -> Result<()> {
        if proposal.schema != "ghostlight.world_elaboration_proposal.v1" {
            return Err(anyhow!("world elaboration proposal schema is unsupported"));
        }
        operation_claims(&proposal.operation)?;
        use WorldElaborationOperation::*;
        match (self, &proposal.operation) {
            (
                Self::PatinaPlace { child_location_id },
                AddPlace {
                    id,
                    name,
                    container_id,
                    persistent_features,
                },
            ) if id == child_location_id
                && !name.trim().is_empty()
                && container_id.as_deref() == Some(target_location_id)
                && !persistent_features.is_empty() => {}
            (
                Self::PatinaOriginRoute {
                    child_location_id,
                    route_id: expected_route_id,
                },
                AddRoute {
                    origin_location_id,
                    route_id,
                    route,
                },
            ) if origin_location_id == target_location_id
                && route_id == expected_route_id
                && route.destination_id == *child_location_id
                && route.distance == "a short internal path"
                && route.travel_minutes == 3 => {}
            (
                Self::PatinaReturnRoute {
                    child_location_id,
                    route_id: expected_route_id,
                },
                AddRoute {
                    origin_location_id,
                    route_id,
                    route,
                },
            ) if origin_location_id == child_location_id
                && route_id == expected_route_id
                && route.destination_id == target_location_id
                && route.distance == "a short internal path"
                && route.travel_minutes == 3 => {}
            (Self::PreserveCivicSystem { system }, SetCivicSystem { system: proposed })
                if proposed == system => {}
            (Self::AddFact { fact_id }, AddFact { fact })
                if fact.id == *fact_id
                    && fact.scope == crate::domain::FactScope::BranchLocal
                    && fact.evidence_receipt_ids.is_empty()
                    && fact.discoverable_at_location_ids
                        == BTreeSet::from([target_location_id.to_owned()])
                    && !fact.statement.trim().is_empty()
                    && fact.statement.chars().count() <= 500
                    && campaign
                        .facts
                        .values()
                        .all(|existing| existing.statement != fact.statement) => {}
            (
                Self::AddPoliticalRelation {
                    relation_id,
                    allowed_subject_ids,
                },
                AddLocalRelation { relation },
            ) if relation.id == *relation_id
                && relation.schema == crate::domain::AgencyRelation::SCHEMA
                && allowed_subject_ids.contains(&relation.from_subject_id)
                && allowed_subject_ids.contains(&relation.to_subject_id)
                && relation.from_subject_id != relation.to_subject_id
                && matches!(
                    relation.kind,
                    crate::domain::AgencyRelationKind::Alliance
                        | crate::domain::AgencyRelationKind::Rivalry
                        | crate::domain::AgencyRelationKind::Trade
                        | crate::domain::AgencyRelationKind::Communication
                        | crate::domain::AgencyRelationKind::Command
                        | crate::domain::AgencyRelationKind::Coercion
                )
                && relation.active
                && (1..=100).contains(&relation.strength)
                && relation.evidence_receipt_ids.is_empty() => {}
            _ => {
                return Err(anyhow!(
                    "proposal does not satisfy the exact titled elaboration task assignment; correct it against this exact contract: {}",
                    self.instruction(target_location_id)?
                ));
            }
        }
        Ok(())
    }
}

fn selected_operation_branch(schema: &mut serde_json::Value) -> Result<&mut serde_json::Value> {
    schema
        .pointer_mut("/$defs/WorldElaborationOperation/oneOf/0")
        .ok_or_else(|| anyhow!("world elaboration schema lost its selected operation variant"))
}

fn exact_schema(value: impl Serialize) -> serde_json::Value {
    let value = serde_json::to_value(value).expect("serializable assignment value");
    let value_type = match &value {
        serde_json::Value::Null => "null",
        serde_json::Value::Bool(_) => "boolean",
        serde_json::Value::Number(number) if number.is_i64() || number.is_u64() => "integer",
        serde_json::Value::Number(_) => "number",
        serde_json::Value::String(_) => "string",
        serde_json::Value::Array(_) => "array",
        serde_json::Value::Object(_) => "object",
    };
    serde_json::json!({
        "type":value_type,
        "const":value,
    })
}

fn exact_keyed_string_map_schema(keys: impl IntoIterator<Item = String>) -> serde_json::Value {
    let keys = keys.into_iter().collect::<Vec<_>>();
    let properties = keys
        .iter()
        .cloned()
        .map(|key| (key, serde_json::json!({"type":"string","minLength":1})))
        .collect::<serde_json::Map<_, _>>();
    serde_json::json!({
        "type":"object",
        "properties":properties,
        "required":keys,
        "additionalProperties":false,
    })
}

fn constrain_exact_object(schema: &mut serde_json::Value, exact: &serde_json::Value) -> Result<()> {
    let exact = exact
        .as_object()
        .ok_or_else(|| anyhow!("exact assignment value is not an object"))?;
    let properties = schema
        .get_mut("properties")
        .and_then(serde_json::Value::as_object_mut)
        .ok_or_else(|| anyhow!("generated assignment object schema has no properties"))?;
    for (name, value) in exact {
        if !properties.contains_key(name) {
            return Err(anyhow!(
                "generated assignment object schema has no {name} property"
            ));
        }
        properties.insert(name.clone(), exact_schema(value));
    }
    schema["required"] = serde_json::json!(exact.keys().collect::<Vec<_>>());
    schema["additionalProperties"] = serde_json::json!(false);
    Ok(())
}

#[derive(Clone, Debug, Serialize)]
#[serde(deny_unknown_fields)]
struct WorldElaborationAgentFinding {
    diagnostic: String,
}

struct WorldElaborationAgentTool<'a> {
    campaign: &'a crate::domain::Campaign,
    target_location_id: &'a str,
    assignment: &'a WorldElaborationAssignment,
    workbench: serde_json::Value,
}

#[async_trait]
impl crate::agent::ModelAgentTool for WorldElaborationAgentTool<'_> {
    type Action = WorldElaborationProposal;
    type Output = WorldElaborationProposal;
    type Finding = WorldElaborationAgentFinding;

    fn action_schema(&self) -> std::result::Result<serde_json::Value, String> {
        self.assignment
            .action_schema(self.target_location_id)
            .map_err(|error| error.to_string())
    }

    fn initial_context_snapshot(&self) -> Option<serde_json::Value> {
        Some(self.workbench.clone())
    }

    async fn invoke(
        &mut self,
        action: Self::Action,
        _context: &crate::agent::ModelAgentToolContext,
    ) -> crate::agent::ModelAgentToolOutcome<Self::Output, Self::Finding> {
        match self
            .assignment
            .validate(self.campaign, self.target_location_id, &action)
        {
            Ok(()) => crate::agent::ModelAgentToolOutcome::Accepted {
                output: action,
                receipts: Vec::new(),
            },
            Err(error) => crate::agent::ModelAgentToolOutcome::Rejected {
                finding: WorldElaborationAgentFinding {
                    diagnostic: error.to_string(),
                },
                receipts: Vec::new(),
            },
        }
    }
}

#[async_trait]
impl ElaborationSubAgentPort<WorldElaborationProposal> for ModelWorldElaborationWorker {
    async fn invoke(
        &self,
        invocation: ElaborationSubAgentInvocation,
    ) -> std::result::Result<
        ElaborationSubAgentOutput<WorldElaborationProposal>,
        ElaborationSubAgentFailure,
    > {
        let expected_wave =
            world_elaboration_wave_binding(&self.campaign, &self.target_location_id).map_err(
                |error| ElaborationSubAgentFailure {
                    diagnostic: error.to_string(),
                    model_stage_receipts: Vec::new(),
                },
            )?;
        if invocation.wave != expected_wave {
            return Err(ElaborationSubAgentFailure {
                diagnostic:
                    "world elaboration worker invocation does not match its frozen campaign".into(),
                model_stage_receipts: Vec::new(),
            });
        }
        let assignment = WorldElaborationAssignment::for_dispatch(
            &self.campaign,
            &self.target_location_id,
            &invocation.dispatch,
        )
        .map_err(|error| ElaborationSubAgentFailure {
            diagnostic: error.to_string(),
            model_stage_receipts: Vec::new(),
        })?;
        let snapshot_binding =
            world_elaboration_invocation_binding(&invocation.wave, &invocation.dispatch).map_err(
                |error| ElaborationSubAgentFailure {
                    diagnostic: error.to_string(),
                    model_stage_receipts: Vec::new(),
                },
            )?;
        // Keep identity and mandate as the stable cacheable prefix. Exact
        // assignment, mutable world projection, and (later) compacted working
        // memory belong to the bounded workbench snapshot.
        let instructions = format!(
            "You are {}, one titled elaborator in an iterative worldbuilding session. {} Your authority is one proposal at a time. Use the typed submit tool to negotiate with the deterministic validator; never claim canonical state, invent evidence receipts, or alter another assignment. Preserve admitted structure and make each contribution specific enough that later events can use it.",
            invocation.dispatch.title.display_name(),
            invocation.dispatch.title.mandate(),
        );
        let assignment_instruction =
            assignment
                .instruction(&self.target_location_id)
                .map_err(|error| ElaborationSubAgentFailure {
                    diagnostic: error.to_string(),
                    model_stage_receipts: Vec::new(),
                })?;
        let projection = self
            .projection()
            .map_err(|error| ElaborationSubAgentFailure {
                diagnostic: error.to_string(),
                model_stage_receipts: Vec::new(),
            })?;
        let model = match invocation.dispatch.title {
            ElaboratorTitle::Charter | ElaboratorTitle::Tangle => crate::model::MODEL_BALANCED,
            ElaboratorTitle::Numen => crate::model::MODEL_CAPABLE,
            _ => crate::model::MODEL_FAST,
        };
        let spec = crate::agent::ModelAgentSpec {
            stage: elaborator_stage(invocation.dispatch.title),
            model: model.into(),
            snapshot_binding,
            instructions,
            source_receipt_ids: Vec::new(),
            temperature: Some(0.4),
            max_output_tokens: Some(1_800),
            max_steps: 2,
        };
        let mut tool = WorldElaborationAgentTool {
            campaign: &self.campaign,
            target_location_id: &self.target_location_id,
            assignment: &assignment,
            workbench: serde_json::json!({
                "schema":"ghostlight.elaborator_workbench.v1",
                "assignment":assignment_instruction,
                "compacted_session":self.session_checkpoints.get(&invocation.dispatch.title),
                "frozen_public_world_projection":serde_json::from_str::<serde_json::Value>(&projection)
                    .unwrap_or_else(|_| serde_json::Value::String(projection)),
            }),
        };
        match crate::agent::run_model_agent(self.model.as_ref(), &spec, &mut tool).await {
            Ok(run) => Ok(ElaborationSubAgentOutput {
                proposal: run.output,
                model_stage_receipts: run.receipts,
            }),
            Err(error) => Err(ElaborationSubAgentFailure {
                diagnostic: error.message,
                model_stage_receipts: error.receipts,
            }),
        }
    }
}

/// One additive world operation proposed by one titled elaborator. These are
/// deliberately compiler inputs, not kernel commands or mutation permits.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WorldElaborationOperation {
    AddPlace {
        id: String,
        name: String,
        container_id: Option<String>,
        #[serde(default)]
        persistent_features: Vec<String>,
    },
    AddRoute {
        origin_location_id: String,
        route_id: String,
        route: crate::domain::Route,
    },
    AddFact {
        fact: crate::domain::WorldFact,
    },
    AddPopulation {
        population: crate::domain::GestaltPersonaState,
        profile: crate::domain::AgencyProfile,
    },
    AddInstitution {
        institution: crate::domain::InstitutionState,
        profile: crate::domain::AgencyProfile,
    },
    AddLocalRelation {
        relation: crate::domain::AgencyRelation,
    },
    AddMigrationRelation {
        relation: crate::domain::AgencyRelation,
    },
    SetCivicSystem {
        system: crate::domain::CivicSystemManifest,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct WorldElaborationProposal {
    pub schema: String,
    pub operation: WorldElaborationOperation,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct AdmittedWorldElaborationOperation {
    pub dispatch: ElaborationDispatch,
    pub operation: WorldElaborationOperation,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorldElaborationRejectionKind {
    InvalidProposal,
    WriteConflict,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct WorldElaborationRejection {
    pub dispatch: ElaborationDispatch,
    pub proposal: WorldElaborationProposal,
    pub kind: WorldElaborationRejectionKind,
    pub conflicting_dispatch_ordinal: Option<u64>,
    pub diagnostic: String,
}

/// Deterministic result of admitting one successful parallel elaboration wave.
/// The candidate remains non-canonical and carries an empty civic verifier
/// binding until an independent semantic verifier finalizes it.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct WorldElaborationAdmission {
    schema: String,
    wave: ElaborationWaveBinding,
    campaign_id: Uuid,
    expected_revision: u64,
    target_location_id: String,
    schedule: ElaborationScheduleReceipt,
    accepted_operations: Vec<AdmittedWorldElaborationOperation>,
    rejections: Vec<WorldElaborationRejection>,
    model_stage_receipts: Vec<crate::model::ModelStageReceipt>,
    candidate: Option<crate::domain::LocalityElaboration>,
    candidate_diagnostic: Option<String>,
    digest: String,
}

/// The only elaboration value accepted by `WorldKernel::commit_elaboration`.
/// It binds the immutable admission result to an independently generated
/// semantic-verifier receipt without letting that verifier rewrite the draft.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct FinalizedWorldElaboration {
    schema: String,
    admission: WorldElaborationAdmission,
    semantic_verifier_receipt: crate::model::ModelStageReceipt,
    digest: String,
}

pub fn world_elaboration_wave_binding(
    campaign: &crate::domain::Campaign,
    target_location_id: &str,
) -> Result<ElaborationWaveBinding> {
    if !campaign.locations.contains_key(target_location_id) {
        return Err(anyhow!("world elaboration target location is unknown"));
    }
    Ok(ElaborationWaveBinding {
        schema: "ghostlight.elaboration_wave_binding.v1".into(),
        snapshot_binding: crate::legacy_transition::digest_serializable(&(
            "ghostlight.world_elaboration_snapshot.v1",
            campaign.id,
            campaign.revision,
            target_location_id,
        ))?,
    })
}

pub fn world_elaboration_invocation_binding(
    wave: &ElaborationWaveBinding,
    dispatch: &ElaborationDispatch,
) -> Result<String> {
    crate::legacy_transition::digest_serializable(&(
        "ghostlight.world_elaboration_agent_snapshot.v1",
        wave,
        dispatch,
    ))
}

/// Deterministically validates, conflict-selects, and merges every proposal in
/// a successful wave. First writer in scheduler dispatch order owns a write
/// claim; later colliding proposals are retained as exact rejections.
pub fn admit_world_elaboration_wave(
    campaign: &crate::domain::Campaign,
    target_location_id: &str,
    run: ElaborationWaveRun<WorldElaborationProposal>,
) -> Result<WorldElaborationAdmission> {
    let expected_wave = world_elaboration_wave_binding(campaign, target_location_id)?;
    if run.wave != expected_wave {
        return Err(anyhow!(
            "world elaboration wave does not bind the current target snapshot"
        ));
    }
    validate_successful_wave_run(&run)?;
    validate_invocation_model_receipts(&run)?;
    let mut invocations = run.invocations;
    invocations.sort_by_key(|invocation| invocation.dispatch.ordinal);
    if invocations
        .iter()
        .zip(run.schedule.dispatches.iter())
        .any(|(invocation, dispatch)| {
            invocation.wave != run.wave || invocation.dispatch != *dispatch
        })
    {
        return Err(anyhow!(
            "world elaboration invocation provenance does not match its schedule"
        ));
    }

    let mut claim_owners = BTreeMap::<String, u64>::new();
    let mut accepted_operations = Vec::new();
    let mut rejections = Vec::new();
    let mut model_stage_receipts = Vec::new();
    for invocation in invocations {
        model_stage_receipts.extend(invocation.model_stage_receipts);
        let proposal = invocation.proposal;
        let claims = if proposal.schema == "ghostlight.world_elaboration_proposal.v1" {
            operation_claims(&proposal.operation)
        } else {
            Err(anyhow!("world elaboration proposal schema is unsupported"))
        };
        let claims = match claims {
            Ok(claims) => claims,
            Err(error) => {
                rejections.push(WorldElaborationRejection {
                    dispatch: invocation.dispatch,
                    proposal,
                    kind: WorldElaborationRejectionKind::InvalidProposal,
                    conflicting_dispatch_ordinal: None,
                    diagnostic: bounded_diagnostic(error.to_string()),
                });
                continue;
            }
        };
        if let Some(conflicting_dispatch_ordinal) = claims
            .iter()
            .filter_map(|claim| claim_owners.get(claim).copied())
            .min()
        {
            rejections.push(WorldElaborationRejection {
                dispatch: invocation.dispatch,
                proposal,
                kind: WorldElaborationRejectionKind::WriteConflict,
                conflicting_dispatch_ordinal: Some(conflicting_dispatch_ordinal),
                diagnostic: format!(
                    "world elaboration write conflicts with dispatch {conflicting_dispatch_ordinal}"
                ),
            });
            continue;
        }
        for claim in claims {
            claim_owners.insert(claim, invocation.dispatch.ordinal);
        }
        accepted_operations.push(AdmittedWorldElaborationOperation {
            dispatch: invocation.dispatch,
            operation: proposal.operation,
        });
    }

    let (candidate, candidate_diagnostic) =
        candidate_from_operations(campaign, target_location_id, &accepted_operations);
    let mut admission = WorldElaborationAdmission {
        schema: "ghostlight.world_elaboration_admission.v1".into(),
        wave: run.wave,
        campaign_id: campaign.id,
        expected_revision: campaign.revision,
        target_location_id: target_location_id.into(),
        schedule: run.schedule,
        accepted_operations,
        rejections,
        model_stage_receipts,
        candidate,
        candidate_diagnostic,
        digest: String::new(),
    };
    admission.digest = world_elaboration_admission_digest(&admission)?;
    Ok(admission)
}

fn validate_successful_wave_run<Proposal>(run: &ElaborationWaveRun<Proposal>) -> Result<()> {
    let schedule = &run.schedule;
    validate_elaboration_schedule(schedule)?;
    if run.invocations.len() != schedule.dispatches.len() {
        return Err(anyhow!(
            "world elaboration schedule receipt is not internally coherent"
        ));
    }
    Ok(())
}

fn validate_elaboration_schedule(schedule: &ElaborationScheduleReceipt) -> Result<()> {
    let recomputed_dispatch_counts = schedule.dispatches.iter().fold(
        BTreeMap::<ElaboratorTitle, u32>::new(),
        |mut counts, dispatch| {
            *counts.entry(dispatch.title).or_default() += 1;
            counts
        },
    );
    if schedule.schema != "ghostlight.elaboration_schedule_receipt.v1"
        || schedule.final_state.schema != "ghostlight.elaboration_dispatch_state.v1"
        || schedule.final_state.profile_digest.trim().is_empty()
        || schedule.requested_invocations
            != schedule.dispatches.len() as u32 + schedule.unused_invocations
        || schedule.unused_invocations != schedule.unused_counts.values().copied().sum::<u32>()
        || schedule.dispatch_counts != recomputed_dispatch_counts
        || schedule
            .dispatches
            .iter()
            .any(|dispatch| !schedule.eligible_titles.contains(&dispatch.title))
    {
        return Err(anyhow!(
            "world elaboration schedule receipt is not internally coherent"
        ));
    }
    let initial_budget_slots = schedule
        .final_state
        .total_budget_slots
        .checked_sub(u64::from(schedule.requested_invocations))
        .ok_or_else(|| anyhow!("world elaboration schedule budget regressed"))?;
    let initial_dispatches = schedule
        .final_state
        .total_dispatches
        .checked_sub(schedule.dispatches.len() as u64)
        .ok_or_else(|| anyhow!("world elaboration dispatch count regressed"))?;
    let mut seen_budget_ordinals = BTreeSet::new();
    let mut next_title_counts = BTreeMap::new();
    for (index, dispatch) in schedule.dispatches.iter().enumerate() {
        let expected_ordinal = initial_dispatches + index as u64 + 1;
        let final_title_count = schedule
            .final_state
            .dispatch_counts
            .get(&dispatch.title)
            .copied()
            .unwrap_or_default();
        let wave_title_count = u64::from(
            schedule
                .dispatch_counts
                .get(&dispatch.title)
                .copied()
                .unwrap_or_default(),
        );
        let initial_title_count = final_title_count
            .checked_sub(wave_title_count)
            .ok_or_else(|| anyhow!("world elaboration title dispatch count regressed"))?;
        let expected_title_count = next_title_counts
            .entry(dispatch.title)
            .and_modify(|count| *count += 1)
            .or_insert(initial_title_count + 1);
        if dispatch.schema != "ghostlight.elaboration_dispatch.v1"
            || dispatch.ordinal != expected_ordinal
            || dispatch.title_dispatch_count != *expected_title_count
            || dispatch.title_weight == 0
            || dispatch.total_enabled_weight == 0
            || dispatch.budget_ordinal <= initial_budget_slots
            || dispatch.budget_ordinal > schedule.final_state.total_budget_slots
            || !seen_budget_ordinals.insert(dispatch.budget_ordinal)
        {
            return Err(anyhow!(
                "world elaboration dispatch provenance is not derived from its final scheduler state"
            ));
        }
        let expected_share = u32::from(dispatch.title_weight).saturating_mul(1_000_000)
            / dispatch.total_enabled_weight;
        if dispatch.requested_share_millionths != expected_share {
            return Err(anyhow!(
                "world elaboration dispatch share is not derived from its configured weight"
            ));
        }
    }
    if next_title_counts.into_iter().any(|(title, count)| {
        schedule.final_state.dispatch_counts.get(&title).copied() != Some(count)
    }) {
        return Err(anyhow!(
            "world elaboration title dispatch totals do not reach final scheduler state"
        ));
    }
    Ok(())
}

fn validate_partial_wave_checkpoint<Proposal>(
    wave: &ElaborationWaveBinding,
    schedule: &ElaborationScheduleReceipt,
    completed_invocations: &[ElaborationInvocation<Proposal>],
    invocation_failures: &[ElaborationInvocationFailure],
) -> Result<()> {
    wave.validate()?;
    validate_elaboration_schedule(schedule)?;
    if invocation_failures.is_empty() {
        return Err(anyhow!(
            "elaboration resume checkpoint contains no failed dispatch"
        ));
    }
    let scheduled = schedule
        .dispatches
        .iter()
        .map(|dispatch| (dispatch.ordinal, dispatch))
        .collect::<BTreeMap<_, _>>();
    let mut partition = BTreeSet::new();
    for invocation in completed_invocations {
        if &invocation.wave != wave
            || scheduled.get(&invocation.dispatch.ordinal).copied() != Some(&invocation.dispatch)
            || !partition.insert(invocation.dispatch.ordinal)
        {
            return Err(anyhow!(
                "elaboration resume checkpoint completed work does not match its exact schedule"
            ));
        }
    }
    for failure in invocation_failures {
        let Some(dispatch) = failure.dispatch.as_ref() else {
            return Err(anyhow!(
                "elaboration resume checkpoint has an unbound failed dispatch"
            ));
        };
        if scheduled.get(&dispatch.ordinal).copied() != Some(dispatch)
            || !partition.insert(dispatch.ordinal)
        {
            return Err(anyhow!(
                "elaboration resume checkpoint failed work does not match its exact schedule"
            ));
        }
    }
    if partition.len() != scheduled.len() {
        return Err(anyhow!(
            "elaboration resume checkpoint does not partition its exact schedule"
        ));
    }
    Ok(())
}

fn validate_invocation_model_receipts<Proposal>(run: &ElaborationWaveRun<Proposal>) -> Result<()> {
    let mut receipt_ids = BTreeSet::new();
    for invocation in &run.invocations {
        if invocation.model_stage_receipts.is_empty() {
            return Err(anyhow!(
                "world elaboration invocation lacks model receipt custody"
            ));
        }
        let expected_binding =
            world_elaboration_invocation_binding(&invocation.wave, &invocation.dispatch)?;
        let expected_stage = elaborator_stage(invocation.dispatch.title);
        for receipt in &invocation.model_stage_receipts {
            let mut rebound = receipt.clone();
            rebound.rebind_snapshot(receipt.snapshot_binding.clone());
            if receipt.schema != "ghostlight.persona_stage_receipt.v1"
                || receipt.stage != expected_stage
                || receipt.snapshot_binding != expected_binding
                || !matches!(
                    receipt.validation_result.as_str(),
                    "valid" | "semantic_invalid"
                )
                || rebound.storage_key() != receipt.storage_key()
                || !receipt_ids.insert(receipt.storage_key().to_owned())
            {
                return Err(anyhow!(
                    "world elaboration model receipt provenance is invalid or duplicated"
                ));
            }
        }
        if invocation
            .model_stage_receipts
            .last()
            .is_none_or(|receipt| {
                receipt.validation_result != "valid" || receipt.local_validation_error.is_some()
            })
        {
            return Err(anyhow!(
                "world elaboration invocation lacks a terminal accepted model receipt"
            ));
        }
    }
    Ok(())
}

fn elaborator_stage(title: ElaboratorTitle) -> String {
    format!(
        "world_elaboration_{}",
        title.display_name().to_ascii_lowercase()
    )
}

fn validate_semantic_verifier_ancestry(
    admission: &WorldElaborationAdmission,
    semantic_verifier_receipt: &crate::model::ModelStageReceipt,
) -> Result<()> {
    let expected = admission
        .model_stage_receipts
        .iter()
        .map(|receipt| receipt.storage_key().to_owned())
        .collect::<BTreeSet<_>>();
    let actual = semantic_verifier_receipt
        .source_receipt_ids
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    if actual.len() != semantic_verifier_receipt.source_receipt_ids.len() || actual != expected {
        return Err(anyhow!(
            "world elaboration semantic verifier ancestry does not exactly cover admitted model receipts"
        ));
    }
    Ok(())
}

pub fn finalize_world_elaboration(
    campaign: &crate::domain::Campaign,
    admission: WorldElaborationAdmission,
    semantic_verifier_receipt: crate::model::ModelStageReceipt,
) -> Result<FinalizedWorldElaboration> {
    validate_semantic_verifier_ancestry(&admission, &semantic_verifier_receipt)?;
    let mut elaboration = admission.valid_candidate(campaign)?;
    elaboration
        .expansion
        .civic_system
        .as_mut()
        .expect("valid locality elaboration has a civic system")
        .semantic_verification_receipt_id = semantic_verifier_receipt.storage_key().into();
    crate::compiler::validate_civic_admission_receipts(
        campaign,
        &elaboration.expansion,
        std::slice::from_ref(&semantic_verifier_receipt),
    )?;
    let mut finalized = FinalizedWorldElaboration {
        schema: "ghostlight.finalized_world_elaboration.v1".into(),
        admission,
        semantic_verifier_receipt,
        digest: String::new(),
    };
    finalized.digest = finalized_world_elaboration_digest(&finalized)?;
    Ok(finalized)
}

impl WorldElaborationAdmission {
    pub fn wave(&self) -> &ElaborationWaveBinding {
        &self.wave
    }

    pub fn schedule(&self) -> &ElaborationScheduleReceipt {
        &self.schedule
    }

    pub fn accepted_operations(&self) -> &[AdmittedWorldElaborationOperation] {
        &self.accepted_operations
    }

    pub fn rejections(&self) -> &[WorldElaborationRejection] {
        &self.rejections
    }

    pub fn model_stage_receipts(&self) -> &[crate::model::ModelStageReceipt] {
        &self.model_stage_receipts
    }

    pub fn candidate(&self) -> Option<&crate::domain::LocalityElaboration> {
        self.candidate.as_ref()
    }

    pub fn candidate_diagnostic(&self) -> Option<&str> {
        self.candidate_diagnostic.as_deref()
    }

    pub fn digest(&self) -> &str {
        &self.digest
    }

    pub fn expected_revision(&self) -> u64 {
        self.expected_revision
    }

    pub fn target_location_id(&self) -> &str {
        &self.target_location_id
    }

    /// Revalidates the complete admission against the current campaign and
    /// derives its candidate from the admitted operations. Recovery code uses
    /// this instead of trusting a serialized candidate projection.
    pub fn valid_candidate(
        &self,
        campaign: &crate::domain::Campaign,
    ) -> Result<crate::domain::LocalityElaboration> {
        if self.schema != "ghostlight.world_elaboration_admission.v1"
            || self.campaign_id != campaign.id
            || self.expected_revision != campaign.revision
            || self.wave != world_elaboration_wave_binding(campaign, &self.target_location_id)?
            || self.digest != world_elaboration_admission_digest(self)?
        {
            return Err(anyhow!(
                "world elaboration admission is stale, malformed, or tampered"
            ));
        }
        validate_admission_dispatch_partition(self)?;
        let (candidate, candidate_diagnostic) = candidate_from_operations(
            campaign,
            &self.target_location_id,
            &self.accepted_operations,
        );
        if candidate != self.candidate || candidate_diagnostic != self.candidate_diagnostic {
            return Err(anyhow!(
                "world elaboration admission candidate is not derived from its accepted operations"
            ));
        }
        if let Some(diagnostic) = &self.candidate_diagnostic {
            return Err(anyhow!(
                "world elaboration candidate requires reconciliation: {diagnostic}"
            ));
        }
        let candidate = self
            .candidate
            .clone()
            .ok_or_else(|| anyhow!("world elaboration admission has no candidate"))?;
        let system = candidate
            .expansion
            .civic_system
            .as_ref()
            .ok_or_else(|| anyhow!("world elaboration candidate has no civic system"))?;
        if !system.semantic_verification_receipt_id.is_empty() {
            return Err(anyhow!(
                "titled elaborators cannot supply the civic semantic-verifier receipt"
            ));
        }
        Ok(candidate)
    }
}

impl FinalizedWorldElaboration {
    pub fn admission(&self) -> &WorldElaborationAdmission {
        &self.admission
    }

    pub fn digest(&self) -> &str {
        &self.digest
    }

    pub fn semantic_verifier_receipt(&self) -> &crate::model::ModelStageReceipt {
        &self.semantic_verifier_receipt
    }

    pub(crate) fn into_kernel_parts(
        self,
        campaign: &crate::domain::Campaign,
    ) -> Result<(
        u64,
        crate::domain::LocalityElaboration,
        Vec<crate::model::ModelStageReceipt>,
    )> {
        if self.schema != "ghostlight.finalized_world_elaboration.v1"
            || self.digest != finalized_world_elaboration_digest(&self)?
        {
            return Err(anyhow!(
                "finalized world elaboration is malformed or tampered"
            ));
        }
        validate_semantic_verifier_ancestry(&self.admission, &self.semantic_verifier_receipt)?;
        let mut elaboration = self.admission.valid_candidate(campaign)?;
        elaboration
            .expansion
            .civic_system
            .as_mut()
            .expect("valid locality elaboration has a civic system")
            .semantic_verification_receipt_id = self.semantic_verifier_receipt.storage_key().into();
        crate::compiler::validate_civic_admission_receipts(
            campaign,
            &elaboration.expansion,
            std::slice::from_ref(&self.semantic_verifier_receipt),
        )?;
        let mut model_stage_receipts = self.admission.model_stage_receipts.clone();
        model_stage_receipts.push(self.semantic_verifier_receipt);
        Ok((
            self.admission.expected_revision,
            elaboration,
            model_stage_receipts,
        ))
    }
}

fn operation_claims(operation: &WorldElaborationOperation) -> Result<Vec<String>> {
    use WorldElaborationOperation::*;
    let nonempty = |value: &str, field: &str| {
        if value.trim().is_empty() {
            Err(anyhow!("world elaboration {field} cannot be empty"))
        } else {
            Ok(())
        }
    };
    match operation {
        AddPlace {
            id,
            name,
            persistent_features,
            ..
        } => {
            nonempty(id, "place id")?;
            nonempty(name, "place name")?;
            if persistent_features
                .iter()
                .any(|value| value.trim().is_empty())
                || persistent_features.iter().collect::<BTreeSet<_>>().len()
                    != persistent_features.len()
            {
                return Err(anyhow!("world elaboration place features are malformed"));
            }
            Ok(vec![format!("subject:{id}")])
        }
        AddRoute {
            origin_location_id,
            route_id,
            route,
        } => {
            nonempty(origin_location_id, "route origin")?;
            nonempty(route_id, "route id")?;
            nonempty(&route.destination_id, "route destination")?;
            nonempty(&route.distance, "route distance")?;
            if route.travel_minutes == 0 || route.destination_id == *origin_location_id {
                return Err(anyhow!("world elaboration route is malformed"));
            }
            Ok(vec![format!("route:{origin_location_id}:{route_id}")])
        }
        AddFact { fact } => {
            nonempty(&fact.id, "fact id")?;
            nonempty(&fact.statement, "fact statement")?;
            if fact.scope == crate::domain::FactScope::CanonBaseline {
                return Err(anyhow!(
                    "titled elaborators cannot assert canon-baseline facts"
                ));
            }
            if !fact.evidence_receipt_ids.is_empty() {
                return Err(anyhow!(
                    "titled elaborators cannot attach source-evidence receipts"
                ));
            }
            Ok(vec![format!("fact:{}", fact.id)])
        }
        AddPopulation {
            population,
            profile,
        } => {
            nonempty(&population.id, "population id")?;
            if profile.subject_id != population.id
                || profile.subject_kind != crate::domain::AgencySubjectKind::Gestalt
                || !profile.evidence_receipt_ids.is_empty()
            {
                return Err(anyhow!(
                    "world elaboration population profile does not bind its population"
                ));
            }
            Ok(vec![
                format!("subject:{}", population.id),
                format!("agency_profile:{}", population.id),
            ])
        }
        AddInstitution {
            institution,
            profile,
        } => {
            nonempty(&institution.id, "institution id")?;
            if profile.subject_id != institution.id
                || profile.subject_kind != crate::domain::AgencySubjectKind::Institution
                || !profile.evidence_receipt_ids.is_empty()
            {
                return Err(anyhow!(
                    "world elaboration institution profile does not bind its institution"
                ));
            }
            Ok(vec![
                format!("subject:{}", institution.id),
                format!("agency_profile:{}", institution.id),
            ])
        }
        AddLocalRelation { relation } | AddMigrationRelation { relation } => {
            nonempty(&relation.id, "relation id")?;
            if !relation.evidence_receipt_ids.is_empty() {
                return Err(anyhow!(
                    "titled elaborators cannot attach source-evidence receipts"
                ));
            }
            Ok(vec![format!("relation:{}", relation.id)])
        }
        SetCivicSystem { system } => {
            nonempty(&system.jurisdiction_location_id, "civic jurisdiction")?;
            if !system.semantic_verification_receipt_id.is_empty() {
                return Err(anyhow!(
                    "titled elaborators cannot supply the civic semantic-verifier receipt"
                ));
            }
            Ok(vec!["civic_system".into()])
        }
    }
}

fn candidate_from_operations(
    campaign: &crate::domain::Campaign,
    target_location_id: &str,
    accepted: &[AdmittedWorldElaborationOperation],
) -> (Option<crate::domain::LocalityElaboration>, Option<String>) {
    match build_candidate(target_location_id, accepted).and_then(|candidate| {
        crate::compiler::validate_locality_elaboration(campaign, &candidate)?;
        Ok(candidate)
    }) {
        Ok(candidate) => (Some(candidate), None),
        Err(error) => {
            let candidate = build_candidate(target_location_id, accepted).ok();
            (candidate, Some(bounded_diagnostic(error.to_string())))
        }
    }
}

fn build_candidate(
    target_location_id: &str,
    accepted: &[AdmittedWorldElaborationOperation],
) -> Result<crate::domain::LocalityElaboration> {
    use WorldElaborationOperation::*;
    let mut expansion = crate::domain::RegionExpansion {
        origin_location_id: target_location_id.into(),
        origin_routes: BTreeMap::new(),
        locations: Vec::new(),
        facts: Vec::new(),
        populations: Vec::new(),
        population_profiles: Vec::new(),
        migration_relations: Vec::new(),
        institutions: Vec::new(),
        institution_profiles: Vec::new(),
        local_relations: Vec::new(),
        civic_system: None,
    };
    for accepted in accepted {
        match &accepted.operation {
            AddPlace {
                id,
                name,
                container_id,
                persistent_features,
            } => expansion.locations.push(crate::domain::Location {
                id: id.clone(),
                name: name.clone(),
                container_id: container_id.clone(),
                routes: BTreeMap::new(),
                persistent_features: persistent_features.clone(),
            }),
            AddFact { fact } => expansion.facts.push(fact.clone()),
            AddPopulation {
                population,
                profile,
            } => {
                expansion.populations.push(population.clone());
                expansion.population_profiles.push(profile.clone());
            }
            AddInstitution {
                institution,
                profile,
            } => {
                expansion.institutions.push(institution.clone());
                expansion.institution_profiles.push(profile.clone());
            }
            AddLocalRelation { relation } => expansion.local_relations.push(relation.clone()),
            AddMigrationRelation { relation } => {
                expansion.migration_relations.push(relation.clone())
            }
            SetCivicSystem { system } => expansion.civic_system = Some(system.clone()),
            AddRoute { .. } => {}
        }
    }
    for accepted in accepted {
        let AddRoute {
            origin_location_id,
            route_id,
            route,
        } = &accepted.operation
        else {
            continue;
        };
        if origin_location_id == target_location_id {
            expansion
                .origin_routes
                .insert(route_id.clone(), route.clone());
        } else {
            let location = expansion
                .locations
                .iter_mut()
                .find(|location| location.id == *origin_location_id)
                .ok_or_else(|| {
                    anyhow!(
                        "world elaboration route origin {} was not proposed",
                        origin_location_id
                    )
                })?;
            location.routes.insert(route_id.clone(), route.clone());
        }
    }
    Ok(crate::domain::LocalityElaboration {
        target_location_id: target_location_id.into(),
        expansion,
    })
}

fn validate_admission_dispatch_partition(admission: &WorldElaborationAdmission) -> Result<()> {
    let scheduled = admission
        .schedule
        .dispatches
        .iter()
        .map(|dispatch| (dispatch.ordinal, dispatch))
        .collect::<BTreeMap<_, _>>();
    let admitted = admission
        .accepted_operations
        .iter()
        .map(|accepted| (accepted.dispatch.ordinal, &accepted.dispatch))
        .chain(
            admission
                .rejections
                .iter()
                .map(|rejection| (rejection.dispatch.ordinal, &rejection.dispatch)),
        )
        .collect::<BTreeMap<_, _>>();
    if scheduled.len() != admission.schedule.dispatches.len()
        || admitted.len() != admission.accepted_operations.len() + admission.rejections.len()
        || scheduled != admitted
    {
        return Err(anyhow!(
            "world elaboration admission does not partition its exact schedule"
        ));
    }
    Ok(())
}

fn world_elaboration_admission_digest(admission: &WorldElaborationAdmission) -> Result<String> {
    crate::legacy_transition::digest_serializable(&(
        "ghostlight.world_elaboration_admission.v1",
        &admission.wave,
        admission.campaign_id,
        admission.expected_revision,
        &admission.target_location_id,
        &admission.schedule,
        &admission.accepted_operations,
        &admission.rejections,
        &admission.model_stage_receipts,
        &admission.candidate,
        &admission.candidate_diagnostic,
    ))
}

fn finalized_world_elaboration_digest(finalized: &FinalizedWorldElaboration) -> Result<String> {
    crate::legacy_transition::digest_serializable(&(
        "ghostlight.finalized_world_elaboration.v1",
        &finalized.admission.digest,
        &finalized.semantic_verifier_receipt,
    ))
}

fn bounded_diagnostic(diagnostic: String) -> String {
    diagnostic.chars().take(2_000).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ten_percent_cover_derives_twenty_four_hundred_actionable_subjects() {
        let demand = derive_world_elaboration_demand(
            240,
            80,
            &WorldScaleIntent::ten_percent(),
            BTreeMap::from([
                ("forest".into(), 3),
                ("grain".into(), 1),
                ("rail".into(), 2),
            ]),
        )
        .unwrap();
        assert_eq!(demand.target_actionable_subjects, 2_400);
        assert_eq!(demand.actionable_subject_deficit, 2_320);
        assert_eq!(demand.round_mutation_budget, 232);
        assert_eq!(demand.realm_subject_targets.values().sum::<u32>(), 2_400);
        assert_eq!(demand.realm_subject_targets["forest"], 1_200);
        assert_eq!(demand.realm_subject_targets["grain"], 400);
        assert_eq!(demand.realm_subject_targets["rail"], 800);
    }

    #[test]
    fn twenty_percent_cover_derives_twelve_hundred_qualified_subjects() {
        let demand = derive_world_elaboration_demand(
            240,
            80,
            &WorldScaleIntent::twenty_percent(),
            BTreeMap::from([("delvehold".into(), 1)]),
        )
        .unwrap();
        assert_eq!(demand.target_actionable_subjects, 1_200);
        assert_eq!(demand.actionable_subject_deficit, 1_120);
        assert_eq!(demand.round_mutation_budget, 224);
    }

    #[test]
    fn elaboration_demand_never_reopens_a_satisfied_subject_target() {
        let demand = derive_world_elaboration_demand(
            8,
            100,
            &WorldScaleIntent::ten_percent(),
            BTreeMap::from([("locality".into(), 1)]),
        )
        .unwrap();
        assert_eq!(demand.target_actionable_subjects, 80);
        assert_eq!(demand.actionable_subject_deficit, 0);
        assert_eq!(demand.round_mutation_budget, 0);
    }

    #[test]
    fn elaboration_demand_counts_only_canonical_simulation_leaves() {
        let mut campaign = campaign_with_civic_room();
        crate::resolution::ensure_agency_profiles(&mut campaign);
        let mut eligible = campaign.agency_profiles[&campaign.player_actor_id].clone();
        eligible.id = "eligible-profile".into();
        eligible.subject_id = "eligible-subject".into();
        eligible.subject_kind = crate::domain::AgencySubjectKind::Gestalt;
        eligible.simulation_eligible = true;
        let eligible_id = eligible.subject_id.clone();
        campaign.gestalts.insert(
            eligible_id.clone(),
            crate::domain::GestaltPersonaState {
                schema: "ghostlight.gestalt_persona_state.v1".into(),
                id: eligible_id.clone(),
                name: "Eligible Population".into(),
                version: 1,
                home_location_id: "room".into(),
                shared_capabilities: BTreeSet::new(),
                shared_knowledge: BTreeSet::new(),
                resources: BTreeSet::new(),
                goals: Vec::new(),
                pressures: Vec::new(),
            },
        );
        campaign
            .agency_profiles
            .insert(eligible_id.clone(), eligible);
        campaign.gestalt_lineages.insert(
            "source-parent".into(),
            crate::domain::GestaltLineage {
                schema: "ghostlight.gestalt_lineage.v1".into(),
                parent_gestalt_id: "source-parent".into(),
                child_gestalt_ids: vec![eligible_id.clone()],
                partition_axis: crate::domain::AgencyAxis::Authority,
                partition_values: BTreeMap::from([(eligible_id.clone(), "other/unknown".into())]),
                residual_child_id: eligible_id.clone(),
                source_revision: 1,
            },
        );
        let mut retired = campaign.agency_profiles[&eligible_id].clone();
        retired.id = "retired-profile".into();
        retired.subject_id = "retired-subject".into();
        retired.active_leaf = false;
        campaign
            .agency_profiles
            .insert(retired.subject_id.clone(), retired);

        let expected = campaign
            .agency_profiles
            .values()
            .filter(|profile| {
                profile.active_leaf
                    && profile.simulation_eligible
                    && profile.subject_id != eligible_id
            })
            .count() as u32;
        assert_eq!(canonical_actionable_subject_count(&campaign), expected);
        assert!(campaign.agency_profiles[&eligible_id].simulation_eligible);
        assert!(!campaign.agency_profiles[&campaign.player_actor_id].simulation_eligible);
    }

    #[test]
    fn ordinary_fission_children_cannot_pay_the_qualified_complexity_bill() {
        let mut campaign = campaign_with_civic_room();
        crate::resolution::ensure_agency_profiles(&mut campaign);
        let parent_id = "source-population".to_owned();
        let parent = crate::domain::GestaltPersonaState {
            schema: "ghostlight.gestalt_persona_state.v1".into(),
            id: parent_id.clone(),
            name: "Source Population".into(),
            version: 0,
            home_location_id: "room".into(),
            shared_capabilities: BTreeSet::new(),
            shared_knowledge: BTreeSet::new(),
            resources: BTreeSet::new(),
            goals: Vec::new(),
            pressures: Vec::new(),
        };
        campaign.gestalts.insert(parent_id.clone(), parent.clone());
        let mut parent_profile = campaign.agency_profiles[&campaign.player_actor_id].clone();
        parent_profile.id = format!("profile:{parent_id}");
        parent_profile.subject_id = parent_id.clone();
        parent_profile.subject_kind = crate::domain::AgencySubjectKind::Gestalt;
        parent_profile.active_leaf = true;
        parent_profile.simulation_eligible = true;
        campaign
            .agency_profiles
            .insert(parent_id.clone(), parent_profile.clone());
        let baseline = canonical_actionable_subject_count(&campaign);

        let mut ordinary = parent.clone();
        ordinary.id = "ordinary-child".into();
        ordinary.name = "Ordinary Child".into();
        let mut qualified = parent.clone();
        qualified.id = "qualified-child".into();
        qualified.name = "Qualified Child".into();
        qualified
            .shared_capabilities
            .insert("operate the west sluice".into());
        qualified
            .goals
            .push("secure an independent flood rota".into());
        let mut residual = parent.clone();
        residual.id = "residual-child".into();
        residual.name = "Residual Child".into();
        for child in [&ordinary, &qualified, &residual] {
            campaign.gestalts.insert(child.id.clone(), child.clone());
            let mut profile = parent_profile.clone();
            profile.id = format!("profile:{}", child.id);
            profile.subject_id = child.id.clone();
            profile.parent_subject_id = Some(parent_id.clone());
            campaign.agency_profiles.insert(child.id.clone(), profile);
        }
        campaign.gestalt_lineages.insert(
            parent_id.clone(),
            crate::domain::GestaltLineage {
                schema: "ghostlight.gestalt_lineage.v1".into(),
                parent_gestalt_id: parent_id,
                child_gestalt_ids: vec![
                    ordinary.id.clone(),
                    qualified.id.clone(),
                    residual.id.clone(),
                ],
                partition_axis: crate::domain::AgencyAxis::Authority,
                partition_values: BTreeMap::from([
                    (ordinary.id.clone(), "ordinary".into()),
                    (qualified.id.clone(), "qualified".into()),
                    (residual.id.clone(), "other/unknown".into()),
                ]),
                residual_child_id: residual.id.clone(),
                source_revision: campaign.revision,
            },
        );

        let ids = canonical_actionable_subject_ids(&campaign);
        assert!(!ids.contains(&ordinary.id));
        assert!(ids.contains(&qualified.id));
        assert!(!ids.contains(&residual.id));
        assert_eq!(canonical_actionable_subject_count(&campaign), baseline + 1);
    }

    #[test]
    fn elaborator_session_compaction_binds_narrative_memory_to_exact_commits() {
        let campaign = campaign_with_civic_room();
        let checkpoint = ElaboratorSessionCheckpoint::bind_compaction(
            "charter:room",
            ElaboratorTitle::Charter,
            2,
            campaign.id,
            campaign.revision,
            "room",
            ElaboratorSessionCompactionDraft {
                schema: "ghostlight.elaborator_session_compaction_draft.v1".into(),
                frontier_summary:
                    "The threshold witness office lacks an acknowledged succession rule.".into(),
                unresolved_leads: vec![
                    "Split the resident body by who can challenge a witness count.".into(),
                ],
            },
            vec!["world-commit:12".into()],
            vec!["A proposed office duplicated an admitted institution.".into()],
            Some("sha256:prior".into()),
        )
        .unwrap();
        checkpoint
            .validate_for(&campaign, "room", ElaboratorTitle::Charter)
            .unwrap();

        let mut tampered = checkpoint.clone();
        tampered.frontier_summary = "Invent an unrelated monarchy.".into();
        assert!(
            tampered
                .validate_for(&campaign, "room", ElaboratorTitle::Charter)
                .is_err()
        );
        assert!(
            checkpoint
                .validate_for(&campaign, "room", ElaboratorTitle::Patina)
                .is_err()
        );
    }

    #[tokio::test]
    async fn session_compaction_schema_and_tool_name_every_invalid_lead() {
        let mut tool = ElaboratorSessionCompactionTool {
            workbench: serde_json::json!({"schema":"test.compaction_workbench.v1"}),
        };
        let schema = crate::agent::ModelAgentTool::action_schema(&tool).unwrap();
        assert_eq!(
            schema["properties"]["unresolved_leads"]["items"]["minLength"],
            1
        );
        assert_eq!(
            schema["properties"]["unresolved_leads"]["items"]["maxLength"],
            600
        );
        let rejected = crate::agent::ModelAgentTool::invoke(
            &mut tool,
            ElaboratorSessionCompactionDraft {
                schema: "ghostlight.elaborator_session_compaction_draft.v1".into(),
                frontier_summary: "A bounded frontier remains live.".into(),
                unresolved_leads: vec![String::new(), "x".repeat(601)],
            },
            &crate::agent::ModelAgentToolContext {
                source_receipt_ids: Vec::new(),
                current_model_receipt: None,
            },
        )
        .await;
        let crate::agent::ModelAgentToolOutcome::Rejected { finding, .. } = rejected else {
            panic!("invalid compaction was admitted")
        };
        assert!(finding.diagnostic.contains("unresolved lead 0"));
        assert!(finding.diagnostic.contains("unresolved lead 1"));
    }

    fn campaign_with_civic_room() -> crate::domain::Campaign {
        let mut campaign = crate::kernel::tests::campaign();
        campaign.civic_systems.insert(
            "room".into(),
            crate::domain::CivicSystemManifest {
                schema: "ghostlight.civic_system_manifest.v1".into(),
                version: 0,
                jurisdiction_location_id: "room".into(),
                governing_institution_ids: BTreeSet::new(),
                resident_population_ids: BTreeSet::new(),
                public_authority_fact_ids: BTreeSet::new(),
                public_selection_fact_ids: BTreeSet::new(),
                public_resource_fact_ids: BTreeSet::new(),
                public_redress_fact_ids: BTreeSet::new(),
                political_relation_ids: BTreeSet::new(),
                semantic_verification_receipt_id: String::new(),
            },
        );
        campaign
    }

    fn campaign_with_fission_parent() -> crate::domain::Campaign {
        let mut campaign = campaign_with_civic_room();
        campaign.gestalts.insert(
            "river-households".into(),
            crate::domain::GestaltPersonaState {
                schema: "ghostlight.gestalt_persona_state.v1".into(),
                id: "river-households".into(),
                name: "River Households".into(),
                version: 0,
                home_location_id: "room".into(),
                shared_capabilities: BTreeSet::from(["river tending".into()]),
                shared_knowledge: BTreeSet::from(["seasonal ford marks".into()]),
                resources: BTreeSet::from(["ferry charter".into()]),
                goals: vec!["keep the river habitable".into()],
                pressures: vec!["the banks are narrowing".into()],
            },
        );
        crate::resolution::ensure_agency_profiles(&mut campaign);
        campaign
            .agency_profiles
            .get_mut("river-households")
            .unwrap()
            .location_ids = BTreeSet::from(["room".into()]);
        campaign
    }

    fn valid_river_fission(
        campaign: &crate::domain::Campaign,
    ) -> crate::domain::GestaltFissionPreview {
        let parent = &campaign.gestalts["river-households"];
        let child = |id: &str, name: &str| crate::domain::GestaltPersonaState {
            schema: parent.schema.clone(),
            id: id.into(),
            name: name.into(),
            version: 0,
            home_location_id: parent.home_location_id.clone(),
            shared_capabilities: parent.shared_capabilities.clone(),
            shared_knowledge: parent.shared_knowledge.clone(),
            resources: BTreeSet::new(),
            goals: parent.goals.clone(),
            pressures: parent.pressures.clone(),
        };
        let mut licensed = child("licensed-river-households", "Licensed River Households");
        licensed.resources.insert("ferry charter".into());
        licensed
            .shared_capabilities
            .insert("withhold licensed ferry service".into());
        licensed
            .goals
            .push("keep licensed crossings under household control".into());
        licensed
            .pressures
            .push("the charter renewal is being challenged".into());
        let mut unlicensed = child("unlicensed-river-households", "Unlicensed River Households");
        unlicensed
            .shared_knowledge
            .insert("unmarked low-water crossings".into());
        unlicensed
            .goals
            .push("open one crossing beyond charter custody".into());
        unlicensed
            .pressures
            .push("licensed patrols impound unmarked ferries".into());
        crate::domain::GestaltFissionPreview {
            schema: "ghostlight.gestalt_fission_preview.v1".into(),
            campaign_id: campaign.id,
            expected_world_revision: campaign.revision,
            parent_gestalt_id: parent.id.clone(),
            partition_axis: crate::domain::AgencyAxis::Authority,
            children: vec![
                licensed,
                unlicensed,
                child("unrecorded-river-households", "Unrecorded River Households"),
            ],
            child_partition_values: BTreeMap::from([
                (
                    "licensed-river-households".into(),
                    "licensed tenders".into(),
                ),
                (
                    "unlicensed-river-households".into(),
                    "unlicensed tenders".into(),
                ),
                ("unrecorded-river-households".into(), "other/unknown".into()),
            ]),
            residual_child_id: "unrecorded-river-households".into(),
            member_child_assignments: BTreeMap::new(),
            resource_child_assignments: BTreeMap::from([(
                "ferry charter".into(),
                "licensed-river-households".into(),
            )]),
            evidence_receipt_ids: Vec::new(),
            gaps: Vec::new(),
            canon_candidates: Vec::new(),
            requires_approval: true,
        }
    }

    fn river_fission_qualification() -> WorldComplexityFissionQualification {
        WorldComplexityFissionQualification {
            schema: "ghostlight.world_complexity_fission_qualification.v1".into(),
            title: ElaboratorTitle::Charter,
            jurisdiction_location_id: "room".into(),
            target_actionable_gain: 1,
            semantic: WorldComplexitySemanticQualification::default(),
        }
    }

    #[tokio::test]
    async fn complexity_tool_admits_only_the_assigned_meaningful_fission() {
        let campaign = campaign_with_fission_parent();
        let preview = valid_river_fission(&campaign);
        let mut tool = WorldComplexityTool {
            campaign: &campaign,
            parent_gestalt_id: "river-households",
            title: ElaboratorTitle::Charter,
            operation: WorldComplexityOperation::Fission,
            partition_axis: crate::domain::AgencyAxis::Authority,
            jurisdiction_location_id: "room",
            target_actionable_gain: 1,
            workbench: serde_json::json!({"schema":"test.workbench.v1"}),
        };
        let context = crate::agent::ModelAgentToolContext {
            source_receipt_ids: Vec::new(),
            current_model_receipt: None,
        };
        let schema = crate::agent::ModelAgentTool::action_schema(&tool).unwrap();
        assert_eq!(schema["type"], "object");
        assert_eq!(
            schema["properties"]["schema"]["const"],
            "ghostlight.world_complexity_action.v1"
        );
        assert_eq!(
            schema["$defs"]["WorldComplexityMutationDraft"]["oneOf"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
        assert!(schema["$defs"].get("GestaltFissionPreview").is_none());
        assert_eq!(
            schema["$defs"]["WorldComplexityMutationDraft"]["oneOf"][0]["properties"]["resource_child_assignments"]
                ["required"],
            serde_json::json!(["ferry charter"])
        );
        assert_eq!(
            schema["$defs"]["WorldComplexityMutationDraft"]["oneOf"][0]["properties"]["member_child_assignments"]
                ["required"],
            serde_json::json!([])
        );
        let draft = WorldComplexityMutationDraft::Fission {
            children: preview
                .children
                .iter()
                .map(|child| WorldComplexityChildDraft {
                    id: child.id.clone(),
                    name: child.name.clone(),
                    partition_value: preview.child_partition_values[&child.id].clone(),
                    home_location_id: child.home_location_id.clone(),
                    capability_additions: child
                        .shared_capabilities
                        .difference(&campaign.gestalts["river-households"].shared_capabilities)
                        .cloned()
                        .collect(),
                    knowledge_additions: child
                        .shared_knowledge
                        .difference(&campaign.gestalts["river-households"].shared_knowledge)
                        .cloned()
                        .collect(),
                    goals_add: child
                        .goals
                        .iter()
                        .filter(|goal| !campaign.gestalts["river-households"].goals.contains(goal))
                        .cloned()
                        .collect(),
                    pressures_add: child
                        .pressures
                        .iter()
                        .filter(|pressure| {
                            !campaign.gestalts["river-households"]
                                .pressures
                                .contains(pressure)
                        })
                        .cloned()
                        .collect(),
                })
                .collect(),
            residual_child_id: preview.residual_child_id.clone(),
            member_child_assignments: preview.member_child_assignments.clone(),
            resource_child_assignments: preview.resource_child_assignments.clone(),
        };

        let admitted = crate::agent::ModelAgentTool::invoke(
            &mut tool,
            WorldComplexityAction {
                schema: "ghostlight.world_complexity_action.v1".into(),
                mutation: draft,
            },
            &context,
        )
        .await;
        let crate::agent::ModelAgentToolOutcome::Accepted {
            output:
                WorldComplexityProposal::Fission {
                    preview: admitted, ..
                },
            ..
        } = admitted
        else {
            panic!("compact fission draft was not admitted")
        };
        assert_eq!(admitted.children.len(), 3);
        assert!(
            admitted
                .children
                .iter()
                .filter(|child| child.id != admitted.residual_child_id)
                .all(|child| {
                    child.goals.len() > campaign.gestalts["river-households"].goals.len()
                        && child.pressures.len()
                            > campaign.gestalts["river-households"].pressures.len()
                })
        );

        let invalid = WorldComplexityMutationDraft::Fission {
            children: preview
                .children
                .iter()
                .map(|child| WorldComplexityChildDraft {
                    id: child.id.clone(),
                    name: child.name.clone(),
                    partition_value: "same partition".into(),
                    home_location_id: child.home_location_id.clone(),
                    capability_additions: BTreeSet::new(),
                    knowledge_additions: BTreeSet::new(),
                    goals_add: Vec::new(),
                    pressures_add: Vec::new(),
                })
                .collect(),
            residual_child_id: preview.residual_child_id,
            member_child_assignments: preview.member_child_assignments,
            resource_child_assignments: preview.resource_child_assignments,
        };
        let rejected = crate::agent::ModelAgentTool::invoke(
            &mut tool,
            WorldComplexityAction {
                schema: "ghostlight.world_complexity_action.v1".into(),
                mutation: invalid,
            },
            &context,
        )
        .await;
        assert!(matches!(
            rejected,
            crate::agent::ModelAgentToolOutcome::Rejected { .. }
        ));
    }

    #[tokio::test]
    async fn complexity_tool_promotes_one_grounded_person_without_a_roster() {
        let campaign = campaign_with_fission_parent();
        let individuation = crate::domain::GestaltIndividuation {
            gestalt_id: "river-households".into(),
            expected_gestalt_version: campaign.gestalts["river-households"].version,
            location_id: "room".into(),
            member: crate::domain::GestaltMemberDelta {
                schema: "ghostlight.gestalt_member_delta.v1".into(),
                id: "orin-weirkeeper".into(),
                gestalt_id: "river-households".into(),
                version: 0,
                name: "Orin Weirkeeper".into(),
                capability_additions: BTreeSet::from(["read narrowing banks".into()]),
                capability_removals: BTreeSet::new(),
                knowledge_additions: BTreeSet::from(["which ford marks were moved".into()]),
                knowledge_removals: BTreeSet::new(),
                equipment: BTreeSet::from(["notched sounding pole".into()]),
                conditions: BTreeSet::new(),
                obligations: BTreeSet::from(["keep the river habitable".into()]),
                relationships: BTreeMap::new(),
                goals: vec!["force a public reckoning over the narrowing banks".into()],
                memories: vec!["found three seasonal ford marks reset upstream".into()],
                last_location_id: Some("room".into()),
                materialized_actor_id: None,
                last_relevant_revision: 0,
                relevance_lease_until_revision: 0,
            },
        };
        let mut punctuation_only = individuation.clone();
        punctuation_only.member.name = "---".into();
        assert!(
            crate::resolution::validate_gestalt_individuation(&campaign, &punctuation_only)
                .unwrap_err()
                .to_string()
                .contains("no public identity characters")
        );
        let mut tool = WorldComplexityTool {
            campaign: &campaign,
            parent_gestalt_id: "river-households",
            title: ElaboratorTitle::Veil,
            operation: WorldComplexityOperation::Individuate,
            partition_axis: crate::domain::AgencyAxis::Information,
            jurisdiction_location_id: "room",
            target_actionable_gain: 1,
            workbench: serde_json::json!({"schema":"test.workbench.v1"}),
        };
        let context = crate::agent::ModelAgentToolContext {
            source_receipt_ids: Vec::new(),
            current_model_receipt: None,
        };
        let schema = crate::agent::ModelAgentTool::action_schema(&tool).unwrap();
        assert_eq!(
            schema["$defs"]["WorldComplexityMutationDraft"]["oneOf"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
        assert!(schema["$defs"].get("GestaltIndividuation").is_none());
        let member_properties = schema["$defs"]["WorldComplexityMemberDraft"]["properties"]
            .as_object()
            .unwrap();
        for deterministic_field in [
            "schema",
            "gestalt_id",
            "version",
            "last_location_id",
            "materialized_actor_id",
            "last_relevant_revision",
            "relevance_lease_until_revision",
        ] {
            assert!(!member_properties.contains_key(deterministic_field));
        }

        let admitted = crate::agent::ModelAgentTool::invoke(
            &mut tool,
            WorldComplexityAction {
                schema: "ghostlight.world_complexity_action.v1".into(),
                mutation: WorldComplexityMutationDraft::Individuate {
                    member: individuation.member.into(),
                },
            },
            &context,
        )
        .await;
        assert!(matches!(
            admitted,
            crate::agent::ModelAgentToolOutcome::Accepted { .. }
        ));
    }

    #[tokio::test]
    async fn complexity_tool_reports_every_individuation_mismatch_for_agent_repair() {
        let mut campaign = campaign_with_fission_parent();
        campaign.actors.insert(
            "member:new-weirkeeper".into(),
            crate::domain::ActorState {
                id: "member:new-weirkeeper".into(),
                name: "Orin Weirkeeper".into(),
                location_id: "room".into(),
                capabilities: BTreeSet::new(),
                knowledge: BTreeSet::new(),
                equipment: BTreeSet::new(),
                conditions: BTreeSet::new(),
                obligations: BTreeSet::new(),
                relationships: BTreeMap::new(),
                goals: Vec::new(),
                memories: Vec::new(),
            },
        );
        let mut tool = WorldComplexityTool {
            campaign: &campaign,
            parent_gestalt_id: "river-households",
            title: ElaboratorTitle::Veil,
            operation: WorldComplexityOperation::Individuate,
            partition_axis: crate::domain::AgencyAxis::Information,
            jurisdiction_location_id: "room",
            target_actionable_gain: 1,
            workbench: serde_json::json!({"schema":"test.workbench.v1"}),
        };
        let context = crate::agent::ModelAgentToolContext {
            source_receipt_ids: Vec::new(),
            current_model_receipt: None,
        };
        let valid_same_name = crate::domain::GestaltMemberDelta {
            schema: "ghostlight.gestalt_member_delta.v1".into(),
            id: "other-weirkeeper".into(),
            gestalt_id: "river-households".into(),
            version: 0,
            name: "Orin Weirkeeper".into(),
            capability_additions: BTreeSet::new(),
            capability_removals: BTreeSet::new(),
            knowledge_additions: BTreeSet::new(),
            knowledge_removals: BTreeSet::new(),
            equipment: BTreeSet::new(),
            conditions: BTreeSet::new(),
            obligations: BTreeSet::new(),
            relationships: BTreeMap::new(),
            goals: Vec::new(),
            memories: Vec::new(),
            last_location_id: Some("room".into()),
            materialized_actor_id: None,
            last_relevant_revision: 0,
            relevance_lease_until_revision: 0,
        };
        let same_name = crate::agent::ModelAgentTool::invoke(
            &mut tool,
            WorldComplexityAction {
                schema: "ghostlight.world_complexity_action.v1".into(),
                mutation: WorldComplexityMutationDraft::Individuate {
                    member: valid_same_name.clone().into(),
                },
            },
            &context,
        )
        .await;
        let crate::agent::ModelAgentToolOutcome::Rejected { finding, .. } = same_name else {
            panic!("duplicate public name was admitted")
        };
        assert!(finding.diagnostic.contains("public Actor identity"));
        let mut invalid = valid_same_name;
        invalid.id = "new-weirkeeper".into();
        invalid.goals = (0..9).map(|index| format!("goal {index}")).collect();
        invalid
            .relationships
            .insert("actor:invented-stranger".into(), "owes a favor".into());
        let rejected = crate::agent::ModelAgentTool::invoke(
            &mut tool,
            WorldComplexityAction {
                schema: "ghostlight.world_complexity_action.v1".into(),
                mutation: WorldComplexityMutationDraft::Individuate {
                    member: invalid.into(),
                },
            },
            &context,
        )
        .await;
        let crate::agent::ModelAgentToolOutcome::Rejected { finding, .. } = rejected else {
            panic!("invalid individuation was admitted")
        };
        assert!(finding.diagnostic.contains("more than eight goals"));
        assert!(
            finding
                .diagnostic
                .contains("duplicates an established canonical subject identity")
        );
        assert!(
            finding
                .diagnostic
                .contains("unsupported subjects: actor:invented-stranger")
        );
        assert!(finding.diagnostic.contains("public Actor identity"));
    }

    #[test]
    fn complexity_fission_rebases_only_across_unchanged_parent_state() {
        let frozen = campaign_with_fission_parent();
        let preview = valid_river_fission(&frozen);
        let mut current = frozen.clone();
        current.revision = current.revision.saturating_add(1);

        let binding = world_complexity_parent_binding(&frozen, "river-households").unwrap();
        let rebased = rebase_world_complexity_proposal(
            &binding,
            &current,
            WorldComplexityProposal::Fission {
                preview: preview.clone(),
                qualification: river_fission_qualification(),
            },
        )
        .unwrap();
        let WorldComplexityProposal::Fission {
            preview: rebased, ..
        } = rebased
        else {
            panic!("rebased mutation changed kind")
        };
        assert_eq!(rebased.expected_world_revision, current.revision);

        current
            .gestalts
            .get_mut("river-households")
            .unwrap()
            .pressures
            .push("an intervening flood changed the parent".into());
        assert!(
            rebase_world_complexity_proposal(
                &binding,
                &current,
                WorldComplexityProposal::Fission {
                    preview,
                    qualification: river_fission_qualification(),
                },
            )
            .is_err()
        );
    }

    #[test]
    fn complexity_fission_rebase_namespaces_only_colliding_child_ids() {
        let frozen = campaign_with_fission_parent();
        let preview = valid_river_fission(&frozen);
        let mut current = frozen.clone();
        current.revision = current.revision.saturating_add(1);
        let mut collision = preview.children[0].clone();
        collision.name = "Preexisting Licensed Register".into();
        let parent = &frozen.gestalts["river-households"];
        collision.shared_capabilities = parent.shared_capabilities.clone();
        collision
            .shared_capabilities
            .insert("maintain tidal sluice clocks".into());
        collision.shared_knowledge = parent.shared_knowledge.clone();
        collision
            .shared_knowledge
            .insert("salt-rot marks on the old quay".into());
        collision.goals = parent.goals.clone();
        collision
            .goals
            .push("restore the abandoned east-quay sluice".into());
        collision.pressures = parent.pressures.clone();
        collision
            .pressures
            .push("spring tides are cracking the quay gates".into());
        current
            .gestalts
            .insert("licensed-river-households".into(), collision);

        let binding = world_complexity_parent_binding(&frozen, "river-households").unwrap();
        let rebased = rebase_world_complexity_proposal(
            &binding,
            &current,
            WorldComplexityProposal::Fission {
                preview,
                qualification: river_fission_qualification(),
            },
        )
        .unwrap();
        let WorldComplexityProposal::Fission {
            preview: rebased, ..
        } = rebased
        else {
            panic!("rebased mutation changed kind")
        };
        let renamed = rebased
            .children
            .iter()
            .find(|child| child.name == "Licensed River Households")
            .unwrap();
        assert!(renamed.id.starts_with("river-households:fission:"));
        assert!(rebased.child_partition_values.contains_key(&renamed.id));
        assert_eq!(
            rebased.resource_child_assignments["ferry charter"],
            renamed.id
        );
        assert_eq!(rebased.residual_child_id, "unrecorded-river-households");
    }

    #[test]
    fn complexity_fission_rebase_sees_agency_profile_id_collisions() {
        let frozen = campaign_with_fission_parent();
        let preview = valid_river_fission(&frozen);
        let colliding_id = preview.children[0].id.clone();
        let mut current = frozen.clone();
        current.revision = current.revision.saturating_add(1);
        let mut reservation = current.agency_profiles["river-households"].clone();
        reservation.id = colliding_id;
        reservation.subject_id = "detached-profile-subject".into();
        reservation.active_leaf = false;
        reservation.simulation_eligible = false;
        current
            .agency_profiles
            .insert("detached-profile-key".into(), reservation);

        let binding = world_complexity_parent_binding(&frozen, "river-households").unwrap();
        let rebase = || {
            rebase_world_complexity_proposal(
                &binding,
                &current,
                WorldComplexityProposal::Fission {
                    preview: preview.clone(),
                    qualification: river_fission_qualification(),
                },
            )
            .unwrap()
        };
        let renamed_id = |proposal: WorldComplexityProposal| {
            let WorldComplexityProposal::Fission { preview, .. } = proposal else {
                panic!("rebased mutation changed kind")
            };
            preview
                .children
                .iter()
                .find(|child| child.name == "Licensed River Households")
                .unwrap()
                .id
                .clone()
        };

        let first = renamed_id(rebase());
        let second = renamed_id(rebase());
        assert!(first.starts_with("river-households:fission:"));
        assert_eq!(first, second);
    }

    fn profile(weights: &[(ElaboratorTitle, u16)]) -> WorldElaborationProfile {
        WorldElaborationProfile {
            schema: "ghostlight.world_elaboration_profile.v1".into(),
            controls: weights
                .iter()
                .map(|(title, weight)| ElaboratorControl {
                    title: *title,
                    weight: *weight,
                })
                .collect(),
        }
    }

    fn wave_binding() -> ElaborationWaveBinding {
        ElaborationWaveBinding {
            schema: "ghostlight.elaboration_wave_binding.v1".into(),
            snapshot_binding: "world:7:seed-sha256".into(),
        }
    }

    struct CountingSubAgent {
        counts: std::sync::Mutex<BTreeMap<ElaboratorTitle, u32>>,
        active: std::sync::atomic::AtomicUsize,
        maximum: std::sync::atomic::AtomicUsize,
    }

    #[async_trait]
    impl ElaborationSubAgentPort<String> for CountingSubAgent {
        async fn invoke(
            &self,
            invocation: ElaborationSubAgentInvocation,
        ) -> std::result::Result<ElaborationSubAgentOutput<String>, ElaborationSubAgentFailure>
        {
            use std::sync::atomic::Ordering;
            assert_eq!(invocation.wave, wave_binding());
            let dispatch = invocation.dispatch;
            let active = self.active.fetch_add(1, Ordering::SeqCst) + 1;
            self.maximum.fetch_max(active, Ordering::SeqCst);
            tokio::time::sleep(std::time::Duration::from_millis(2)).await;
            *self
                .counts
                .lock()
                .unwrap()
                .entry(dispatch.title)
                .or_default() += 1;
            self.active.fetch_sub(1, Ordering::SeqCst);
            Ok(ElaborationSubAgentOutput::deterministic(format!(
                "proposal:{}",
                dispatch.ordinal
            )))
        }
    }

    struct PartiallyFailingSubAgent;

    struct RecordingResumeSubAgent {
        ordinals: std::sync::Mutex<Vec<u64>>,
    }

    #[async_trait]
    impl ElaborationSubAgentPort<String> for PartiallyFailingSubAgent {
        async fn invoke(
            &self,
            invocation: ElaborationSubAgentInvocation,
        ) -> std::result::Result<ElaborationSubAgentOutput<String>, ElaborationSubAgentFailure>
        {
            if invocation.dispatch.ordinal == 3 {
                return Err(ElaborationSubAgentFailure {
                    diagnostic: "fixture refusal".into(),
                    model_stage_receipts: Vec::new(),
                });
            }
            if invocation.dispatch.ordinal == 4 {
                panic!("fixture panic");
            }
            Ok(ElaborationSubAgentOutput::deterministic(format!(
                "proposal:{}",
                invocation.dispatch.ordinal
            )))
        }
    }

    #[async_trait]
    impl ElaborationSubAgentPort<String> for RecordingResumeSubAgent {
        async fn invoke(
            &self,
            invocation: ElaborationSubAgentInvocation,
        ) -> std::result::Result<ElaborationSubAgentOutput<String>, ElaborationSubAgentFailure>
        {
            self.ordinals
                .lock()
                .unwrap()
                .push(invocation.dispatch.ordinal);
            Ok(ElaborationSubAgentOutput::deterministic(format!(
                "resumed:{}",
                invocation.dispatch.ordinal
            )))
        }
    }

    #[test]
    fn weighted_schedule_dispatches_in_proportion_to_slider_share() {
        let profile = profile(&[(ElaboratorTitle::Patina, 20), (ElaboratorTitle::Tangle, 80)]);
        let mut scheduler = ElaborationScheduler::new(&profile).unwrap();
        let eligible = BTreeSet::from([ElaboratorTitle::Patina, ElaboratorTitle::Tangle]);

        let receipt = scheduler.schedule(&eligible, 100);

        assert_eq!(receipt.unused_invocations, 0);
        assert_eq!(receipt.dispatch_counts[&ElaboratorTitle::Patina], 20);
        assert_eq!(receipt.dispatch_counts[&ElaboratorTitle::Tangle], 80);
        assert_eq!(
            receipt.final_state.dispatch_counts[&ElaboratorTitle::Patina],
            20
        );
        assert_eq!(
            receipt.final_state.dispatch_counts[&ElaboratorTitle::Tangle],
            80
        );
        assert!(receipt.dispatches.iter().all(|dispatch| {
            dispatch.requested_share_millionths
                == match dispatch.title {
                    ElaboratorTitle::Patina => 200_000,
                    ElaboratorTitle::Tangle => 800_000,
                    _ => unreachable!(),
                }
        }));
    }

    #[tokio::test]
    async fn weighted_share_controls_actual_parallel_sub_agent_invocations() {
        use std::sync::atomic::Ordering;
        let profile = profile(&[(ElaboratorTitle::Patina, 30), (ElaboratorTitle::Tangle, 70)]);
        let mut scheduler = ElaborationScheduler::new(&profile).unwrap();
        let eligible = BTreeSet::from([ElaboratorTitle::Patina, ElaboratorTitle::Tangle]);
        let worker = Arc::new(CountingSubAgent {
            counts: std::sync::Mutex::new(BTreeMap::new()),
            active: std::sync::atomic::AtomicUsize::new(0),
            maximum: std::sync::atomic::AtomicUsize::new(0),
        });

        let run = dispatch_elaboration_wave(
            &mut scheduler,
            wave_binding(),
            &eligible,
            100,
            6,
            worker.clone(),
        )
        .await
        .unwrap();

        assert_eq!(run.wave, wave_binding());
        assert_eq!(run.invocations.len(), 100);
        assert!(
            run.invocations
                .iter()
                .all(|invocation| invocation.wave == wave_binding())
        );
        assert_eq!(run.invocations[0].dispatch.ordinal, 1);
        assert_eq!(run.invocations[99].dispatch.ordinal, 100);
        let counts = worker.counts.lock().unwrap();
        assert_eq!(counts[&ElaboratorTitle::Patina], 30);
        assert_eq!(counts[&ElaboratorTitle::Tangle], 70);
        assert!(worker.maximum.load(Ordering::SeqCst) > 1);
        assert!(worker.maximum.load(Ordering::SeqCst) <= 6);
    }

    #[tokio::test]
    async fn failed_wave_preserves_consumed_schedule_and_completed_evidence() {
        let profile = profile(&[(ElaboratorTitle::Patina, 100)]);
        let mut scheduler = ElaborationScheduler::new(&profile).unwrap();
        let failure = dispatch_elaboration_wave(
            &mut scheduler,
            wave_binding(),
            &BTreeSet::from([ElaboratorTitle::Patina]),
            4,
            2,
            Arc::new(PartiallyFailingSubAgent),
        )
        .await
        .unwrap_err();

        assert_eq!(failure.wave, Some(wave_binding()));
        assert_eq!(failure.schedule.as_ref().unwrap().dispatches.len(), 4);
        assert_eq!(failure.completed_invocations.len(), 2);
        assert_eq!(failure.invocation_failures.len(), 2);
        assert_eq!(
            failure.invocation_failures[0]
                .dispatch
                .as_ref()
                .unwrap()
                .ordinal,
            3
        );
        assert_eq!(
            failure.invocation_failures[1]
                .dispatch
                .as_ref()
                .unwrap()
                .ordinal,
            4
        );
        assert_eq!(scheduler.state().total_budget_slots, 4);
        assert!(failure.completed_invocations.iter().all(|invocation| {
            invocation.wave == wave_binding() && invocation.dispatch.ordinal != 3
        }));
    }

    #[tokio::test]
    async fn partial_wave_resume_invokes_only_failed_original_dispatches() {
        let profile = profile(&[(ElaboratorTitle::Patina, 100)]);
        let mut scheduler = ElaborationScheduler::new(&profile).unwrap();
        let failure = dispatch_elaboration_wave(
            &mut scheduler,
            wave_binding(),
            &BTreeSet::from([ElaboratorTitle::Patina]),
            4,
            2,
            Arc::new(PartiallyFailingSubAgent),
        )
        .await
        .unwrap_err();
        let original_schedule = failure.schedule.clone().unwrap();
        let completed_before = failure
            .completed_invocations
            .iter()
            .map(|invocation| invocation.dispatch.ordinal)
            .collect::<Vec<_>>();
        let worker = Arc::new(RecordingResumeSubAgent {
            ordinals: std::sync::Mutex::new(Vec::new()),
        });

        let run = resume_elaboration_wave(failure, 2, worker.clone())
            .await
            .unwrap();

        assert_eq!(worker.ordinals.lock().unwrap().as_slice(), &[3, 4]);
        assert_eq!(completed_before, vec![1, 2]);
        assert_eq!(run.schedule, original_schedule);
        assert_eq!(
            run.invocations
                .iter()
                .map(|invocation| invocation.dispatch.ordinal)
                .collect::<Vec<_>>(),
            vec![1, 2, 3, 4]
        );
        assert_eq!(scheduler.state().total_dispatches, 4);
    }

    #[tokio::test]
    async fn partial_wave_resume_refuses_a_checkpoint_with_duplicate_authority() {
        let profile = profile(&[(ElaboratorTitle::Patina, 100)]);
        let mut scheduler = ElaborationScheduler::new(&profile).unwrap();
        let mut failure = dispatch_elaboration_wave(
            &mut scheduler,
            wave_binding(),
            &BTreeSet::from([ElaboratorTitle::Patina]),
            4,
            2,
            Arc::new(PartiallyFailingSubAgent),
        )
        .await
        .unwrap_err();
        failure.invocation_failures[0].dispatch =
            Some(failure.completed_invocations[0].dispatch.clone());
        let worker = Arc::new(RecordingResumeSubAgent {
            ordinals: std::sync::Mutex::new(Vec::new()),
        });

        let resumed = resume_elaboration_wave(failure, 2, worker.clone()).await;

        assert!(resumed.is_err());
        assert!(worker.ordinals.lock().unwrap().is_empty());
        assert!(
            resumed.unwrap_err().invocation_failures[0]
                .diagnostic
                .contains("does not match its exact schedule")
        );
    }

    #[test]
    fn ineligible_titles_consume_their_share_without_catchup_debt() {
        let profile = profile(&[(ElaboratorTitle::Patina, 20), (ElaboratorTitle::Tangle, 80)]);
        let mut scheduler = ElaborationScheduler::new(&profile).unwrap();

        let patina_only = BTreeSet::from([ElaboratorTitle::Patina]);
        let first = scheduler.schedule(&patina_only, 5);
        assert_eq!(
            first.final_state.dispatch_counts[&ElaboratorTitle::Patina],
            1
        );
        assert_eq!(first.unused_invocations, 4);
        assert_eq!(first.unused_counts[&ElaboratorTitle::Tangle], 4);

        let both = BTreeSet::from([ElaboratorTitle::Patina, ElaboratorTitle::Tangle]);
        let second = scheduler.schedule(&both, 100);
        assert_eq!(second.dispatch_counts[&ElaboratorTitle::Patina], 20);
        assert_eq!(second.dispatch_counts[&ElaboratorTitle::Tangle], 80);
        assert_eq!(
            second.final_state.dispatch_counts[&ElaboratorTitle::Patina],
            21
        );
        assert_eq!(
            second.final_state.dispatch_counts[&ElaboratorTitle::Tangle],
            80
        );
    }

    #[test]
    fn disabled_or_blocked_elaborators_leave_budget_visibly_unused() {
        let profile = profile(&[(ElaboratorTitle::Patina, 100)]);
        let mut scheduler = ElaborationScheduler::new(&profile).unwrap();
        let receipt = scheduler.schedule(&BTreeSet::from([ElaboratorTitle::Veil]), 7);

        assert!(receipt.dispatches.is_empty());
        assert_eq!(receipt.unused_invocations, 7);
        assert_eq!(receipt.unused_counts[&ElaboratorTitle::Patina], 7);
        assert_eq!(receipt.final_state.total_budget_slots, 7);
        assert_eq!(receipt.final_state.total_dispatches, 0);
    }

    #[test]
    fn a_partially_blocked_profile_does_not_donate_its_share() {
        let profile = profile(&[(ElaboratorTitle::Patina, 20), (ElaboratorTitle::Tangle, 80)]);
        let mut scheduler = ElaborationScheduler::new(&profile).unwrap();
        let receipt = scheduler.schedule(&BTreeSet::from([ElaboratorTitle::Patina]), 100);

        assert_eq!(receipt.dispatch_counts[&ElaboratorTitle::Patina], 20);
        assert_eq!(receipt.unused_counts[&ElaboratorTitle::Tangle], 80);
        assert_eq!(receipt.unused_invocations, 80);
        assert_eq!(receipt.final_state.total_budget_slots, 100);
        assert_eq!(receipt.final_state.total_dispatches, 20);
    }

    #[test]
    fn profile_rejects_duplicate_titles_and_an_all_zero_budget() {
        assert!(
            profile(&[(ElaboratorTitle::Patina, 10), (ElaboratorTitle::Patina, 20),])
                .weights()
                .is_err()
        );
        assert!(profile(&[(ElaboratorTitle::Patina, 0)]).weights().is_err());
    }

    #[test]
    fn persisted_scheduler_state_binds_semantic_weights_not_control_order() {
        let first = profile(&[(ElaboratorTitle::Patina, 25), (ElaboratorTitle::Tangle, 75)]);
        let scheduler = ElaborationScheduler::new(&first).unwrap();
        let state = scheduler.state().clone();
        let reordered = profile(&[(ElaboratorTitle::Tangle, 75), (ElaboratorTitle::Patina, 25)]);
        assert!(ElaborationScheduler::from_state(&reordered, state.clone()).is_ok());

        let changed = profile(&[(ElaboratorTitle::Patina, 75), (ElaboratorTitle::Tangle, 25)]);
        assert!(ElaborationScheduler::from_state(&changed, state).is_err());
    }

    #[test]
    fn catalog_exposes_every_titled_slider_with_a_distinct_mandate() {
        let catalog = elaborator_catalog();
        assert_eq!(catalog.len(), ElaboratorTitle::ALL.len());
        assert_eq!(
            catalog
                .iter()
                .map(|entry| entry.title)
                .collect::<BTreeSet<_>>()
                .len(),
            ElaboratorTitle::ALL.len()
        );
        assert!(catalog.iter().all(|entry| !entry.mandate.trim().is_empty()));
        assert!(catalog.iter().any(|entry| {
            entry.title == ElaboratorTitle::Patina && entry.mandate.contains("low-stakes texture")
        }));
    }

    #[test]
    fn titled_operations_cannot_mint_source_evidence_authority() {
        let operation = WorldElaborationOperation::AddFact {
            fact: crate::domain::WorldFact {
                id: "branch-fact".into(),
                statement: "The gate duck is called Harold.".into(),
                scope: crate::domain::FactScope::BranchLocal,
                evidence_receipt_ids: vec!["invented-receipt".into()],
                discoverable_at_location_ids: BTreeSet::from(["room".into()]),
            },
        };

        let error = operation_claims(&operation).unwrap_err();

        assert!(error.to_string().contains("cannot attach source-evidence"));
    }

    struct PanicIfInvokedModel;

    struct ExactAssignmentSchemaModel;

    struct ExactSessionCompactionModel;

    struct RejectionOnlySessionCompactionModel;

    struct ReceiptlessWorldWorker;

    #[async_trait]
    impl ElaborationSubAgentPort<WorldElaborationProposal> for ReceiptlessWorldWorker {
        async fn invoke(
            &self,
            _invocation: ElaborationSubAgentInvocation,
        ) -> std::result::Result<
            ElaborationSubAgentOutput<WorldElaborationProposal>,
            ElaborationSubAgentFailure,
        > {
            Ok(ElaborationSubAgentOutput::deterministic(
                WorldElaborationProposal {
                    schema: "ghostlight.world_elaboration_proposal.v1".into(),
                    operation: WorldElaborationOperation::AddFact {
                        fact: crate::domain::WorldFact {
                            id: "receiptless-fact".into(),
                            statement: "This proposal has no provider custody.".into(),
                            scope: crate::domain::FactScope::BranchLocal,
                            evidence_receipt_ids: vec![],
                            discoverable_at_location_ids: BTreeSet::from(["room".into()]),
                        },
                    },
                },
            ))
        }
    }

    #[async_trait]
    impl crate::model::ModelPort for PanicIfInvokedModel {
        async fn run(&self, _request: &crate::model::ModelStageRequest) -> Result<String> {
            panic!("stale world worker reached the model")
        }

        fn provider(&self) -> &'static str {
            "panic-fixture"
        }
    }

    #[async_trait]
    impl crate::model::ModelPort for ExactAssignmentSchemaModel {
        async fn run(&self, request: &crate::model::ModelStageRequest) -> Result<String> {
            let schema = request
                .output_schema
                .as_ref()
                .ok_or_else(|| anyhow!("titled agent lost its action schema"))?;
            assert_eq!(
                schema.pointer("/$defs/WorldElaborationOperation/oneOf/0/properties/type/const"),
                Some(&serde_json::json!("add_fact"))
            );
            assert_eq!(
                schema
                    .pointer("/$defs/WorldElaborationOperation/oneOf")
                    .and_then(serde_json::Value::as_array)
                    .map(Vec::len),
                Some(1)
            );
            assert_eq!(
                schema.pointer("/$defs/WorldFact/properties/id/const"),
                Some(&serde_json::json!("elab:room:charter-fact:2"))
            );
            let output = serde_json::json!({
                "schema":"ghostlight.world_elaboration_proposal.v1",
                "operation":{
                    "type":"add_fact",
                    "fact":{
                        "id":"elab:room:charter-fact:2",
                        "statement":"The room elects a threshold witness after every public repair count.",
                        "scope":"branch_local",
                        "evidence_receipt_ids":[],
                        "discoverable_at_location_ids":["room"]
                    }
                }
            });
            let validator = jsonschema::validator_for(schema)?;
            assert!(validator.is_valid(&output));
            let mut wrong_id = output.clone();
            wrong_id["operation"]["fact"]["id"] = serde_json::json!("wrong-id");
            assert!(!validator.is_valid(&wrong_id));
            assert!(!validator.is_valid(&serde_json::json!({
                "schema":"ghostlight.world_elaboration_proposal.v1",
                "operation":{
                    "type":"set_civic_system",
                    "system":{}
                }
            })));
            let mut provider_schema = schema.clone();
            crate::model_connector::project_strict_responses_schema(&mut provider_schema)?;
            assert!(jsonschema::validator_for(&provider_schema)?.is_valid(&output));
            Ok(output.to_string())
        }

        fn provider(&self) -> &'static str {
            "exact-assignment-schema-fixture"
        }
    }

    #[async_trait]
    impl crate::model::ModelPort for ExactSessionCompactionModel {
        async fn run(&self, request: &crate::model::ModelStageRequest) -> Result<String> {
            assert!(request.stage.ends_with("session-compaction"));
            assert!(request.lived_stream.contains("world-commit:12"));
            assert!(request.lived_stream.contains("threshold witness"));
            Ok(serde_json::json!({
                "schema":"ghostlight.elaborator_session_compaction_draft.v1",
                "frontier_summary":"The threshold witness office still lacks a succession rule.",
                "unresolved_leads":["Fission the electorate by access to public count challenges."]
            })
            .to_string())
        }

        fn provider(&self) -> &'static str {
            "session-compaction-fixture"
        }
    }

    #[async_trait]
    impl crate::model::ModelPort for RejectionOnlySessionCompactionModel {
        async fn run(&self, request: &crate::model::ModelStageRequest) -> Result<String> {
            assert!(request.stage.ends_with("session-compaction"));
            assert!(
                request
                    .lived_stream
                    .contains("causal_additions_do_not_repeat_an_overused_procedural_template")
            );
            Ok(serde_json::json!({
                "schema":"ghostlight.elaborator_session_compaction_draft.v1",
                "frontier_summary":"The rejected filing mold remains a live design constraint.",
                "unresolved_leads":["Replace procedure with a materially executable conflict."]
            })
            .to_string())
        }

        fn provider(&self) -> &'static str {
            "rejection-session-compaction-fixture"
        }
    }

    #[tokio::test]
    async fn session_compactor_preserves_exact_journal_ancestry() {
        let campaign = campaign_with_civic_room();
        let (checkpoint, receipts) = compact_elaborator_session(
            &ExactSessionCompactionModel,
            &campaign,
            "room",
            ElaboratorTitle::Charter,
            "charter:room",
            None,
            &[ElaboratorSessionJournalEntry {
                world_revision: campaign.revision,
                commit_receipt_id: "world-commit:12".into(),
                mutation_kind: "fission_gestalt".into(),
                affected_subject_ids: vec!["threshold-witnesses".into()],
                summary: "The threshold witness electorate split over public challenges.".into(),
            }],
            vec!["A duplicate office proposal was rejected.".into()],
        )
        .await
        .unwrap();

        assert_eq!(checkpoint.recent_commit_receipt_ids, ["world-commit:12"]);
        assert_eq!(checkpoint.generation, 0);
        assert_eq!(receipts.len(), 1);
        checkpoint
            .validate_for(&campaign, "room", ElaboratorTitle::Charter)
            .unwrap();
    }

    #[tokio::test]
    async fn session_compactor_carries_a_semantic_rejection_without_a_commit() {
        let campaign = campaign_with_civic_room();
        let finding = "{\"causal_additions_do_not_repeat_an_overused_procedural_template\":false}";
        let (checkpoint, receipts) = compact_elaborator_session(
            &RejectionOnlySessionCompactionModel,
            &campaign,
            "room",
            ElaboratorTitle::Charter,
            "charter:room",
            None,
            &[],
            vec![finding.into()],
        )
        .await
        .unwrap();

        assert!(checkpoint.recent_commit_receipt_ids.is_empty());
        assert_eq!(checkpoint.recent_rejection_findings, [finding]);
        assert_eq!(receipts.len(), 1);
        checkpoint
            .validate_for(&campaign, "room", ElaboratorTitle::Charter)
            .unwrap();
    }

    #[tokio::test]
    async fn session_compactor_reports_every_deterministic_preflight_mismatch() {
        let campaign = campaign_with_civic_room();
        let error = compact_elaborator_session(
            &PanicIfInvokedModel,
            &campaign,
            "missing-realm",
            ElaboratorTitle::Charter,
            "charter:missing-realm",
            None,
            &[
                ElaboratorSessionJournalEntry {
                    world_revision: campaign.revision.saturating_add(1),
                    commit_receipt_id: " ".into(),
                    mutation_kind: "".into(),
                    affected_subject_ids: (0..33).map(|index| format!("subject-{index}")).collect(),
                    summary: " ".into(),
                },
                ElaboratorSessionJournalEntry {
                    world_revision: campaign.revision,
                    commit_receipt_id: "world-commit:13".into(),
                    mutation_kind: "fission_gestalt".into(),
                    affected_subject_ids: Vec::new(),
                    summary: "x".repeat(1_001),
                },
            ],
            Vec::new(),
        )
        .await
        .unwrap_err()
        .to_string();

        assert!(error.contains("target location \"missing-realm\" is absent"));
        assert!(error.contains("journal[0].world_revision"));
        assert!(error.contains("journal[0].commit_receipt_id is empty"));
        assert!(error.contains("journal[0].mutation_kind is empty"));
        assert!(error.contains("journal[0].summary is empty"));
        assert!(error.contains("journal[0].affected_subject_ids contains 33"));
        assert!(error.contains("journal[1].summary contains 1001 characters"));
    }

    #[test]
    fn elaborator_session_routing_separates_titles_and_realm_jurisdictions() {
        assert_ne!(
            elaborator_session_id(ElaboratorTitle::Patina, "elven-realms"),
            elaborator_session_id(ElaboratorTitle::Patina, "kharad-hearths")
        );
        assert_ne!(
            elaborator_session_id(ElaboratorTitle::Patina, "elven-realms"),
            elaborator_session_id(ElaboratorTitle::Charter, "elven-realms")
        );
        assert_eq!(
            elaborator_session_id(ElaboratorTitle::Patina, "elven-realms"),
            "patina:elven-realms"
        );
    }

    #[test]
    fn complexity_worker_rejects_a_cross_bound_session_identity() {
        let campaign = campaign_with_fission_parent();
        let checkpoint = ElaboratorSessionCheckpoint::bind_compaction(
            "patina:room",
            ElaboratorTitle::Charter,
            0,
            campaign.id,
            campaign.revision,
            "room",
            ElaboratorSessionCompactionDraft {
                schema: "ghostlight.elaborator_session_compaction_draft.v1".into(),
                frontier_summary: "A valid checkpoint parked under another title's key.".into(),
                unresolved_leads: Vec::new(),
            },
            vec!["world-commit:1".into()],
            Vec::new(),
            None,
        )
        .unwrap();

        let result = ModelWorldComplexityWorker::new(
            Arc::new(PanicIfInvokedModel),
            Arc::new(campaign),
            1,
            vec!["river-households".into()],
            BTreeMap::from([("river-households".into(), "room".into())]),
            1,
            BTreeMap::from([("patina:room".into(), checkpoint)]),
        );

        let error = match result {
            Ok(_) => panic!("cross-bound elaborator session reached the worker"),
            Err(error) => error,
        };
        assert!(error.to_string().contains("misrouted session checkpoint"));
    }

    #[test]
    fn complexity_worker_accepts_a_canonical_containing_jurisdiction() {
        let mut campaign = campaign_with_fission_parent();
        campaign.locations.get_mut("room").unwrap().container_id = Some("realm".into());
        campaign.locations.insert(
            "realm".into(),
            crate::domain::Location {
                id: "realm".into(),
                name: "Realm".into(),
                container_id: None,
                routes: BTreeMap::new(),
                persistent_features: Vec::new(),
            },
        );

        let worker = ModelWorldComplexityWorker::new(
            Arc::new(PanicIfInvokedModel),
            Arc::new(campaign),
            1,
            vec!["river-households".into()],
            BTreeMap::from([("river-households".into(), "realm".into())]),
            1,
            BTreeMap::new(),
        );

        assert!(worker.is_ok());
    }

    #[tokio::test]
    async fn provider_worker_rejects_a_wave_from_a_newer_world_before_model_inference() {
        let frozen = campaign_with_civic_room();
        let worker = ModelWorldElaborationWorker::new(
            Arc::new(PanicIfInvokedModel),
            Arc::new(frozen.clone()),
            "room",
            "add texture",
        )
        .unwrap();
        assert_eq!(worker.task_request(), "add texture");
        let mut current = frozen;
        current.revision = 1;
        let invocation = ElaborationSubAgentInvocation {
            wave: world_elaboration_wave_binding(&current, "room").unwrap(),
            dispatch: ElaborationDispatch {
                schema: "ghostlight.elaboration_dispatch.v1".into(),
                budget_ordinal: 1,
                ordinal: 1,
                title: ElaboratorTitle::Patina,
                title_weight: 1,
                total_enabled_weight: 1,
                requested_share_millionths: 1_000_000,
                title_dispatch_count: 1,
            },
        };

        let error = worker.invoke(invocation).await.unwrap_err();

        assert!(error.diagnostic.contains("frozen campaign"));
        assert!(error.model_stage_receipts.is_empty());
    }

    #[tokio::test]
    async fn provider_worker_publishes_the_exact_assignment_schema() {
        let campaign = campaign_with_civic_room();
        let worker = ModelWorldElaborationWorker::new(
            Arc::new(ExactAssignmentSchemaModel),
            Arc::new(campaign.clone()),
            "room",
            "add political texture",
        )
        .unwrap();
        let output = worker
            .invoke(ElaborationSubAgentInvocation {
                wave: world_elaboration_wave_binding(&campaign, "room").unwrap(),
                dispatch: ElaborationDispatch {
                    schema: "ghostlight.elaboration_dispatch.v1".into(),
                    budget_ordinal: 2,
                    ordinal: 2,
                    title: ElaboratorTitle::Charter,
                    title_weight: 1,
                    total_enabled_weight: 1,
                    requested_share_millionths: 1_000_000,
                    title_dispatch_count: 2,
                },
            })
            .await
            .unwrap();

        assert_eq!(output.model_stage_receipts.len(), 1);
        assert!(matches!(
            output.proposal.operation,
            WorldElaborationOperation::AddFact { ref fact }
                if fact.id == "elab:room:charter-fact:2"
        ));
    }

    #[test]
    fn every_assignment_schema_exposes_one_owned_operation_variant() {
        let mut campaign = campaign_with_civic_room();
        campaign
            .civic_systems
            .get_mut("room")
            .unwrap()
            .governing_institution_ids =
            BTreeSet::from(["first-council".into(), "second-council".into()]);
        let cases = [
            (ElaboratorTitle::Patina, 1, "add_place"),
            (ElaboratorTitle::Patina, 2, "add_route"),
            (ElaboratorTitle::Patina, 3, "add_route"),
            (ElaboratorTitle::Charter, 1, "set_civic_system"),
            (ElaboratorTitle::Charter, 2, "add_fact"),
            (ElaboratorTitle::Tangle, 1, "add_local_relation"),
            (ElaboratorTitle::Tangle, 2, "add_fact"),
            (ElaboratorTitle::Ledger, 1, "add_fact"),
        ];

        for (ordinal, (title, title_dispatch_count, expected_type)) in cases.into_iter().enumerate()
        {
            let assignment = WorldElaborationAssignment::for_dispatch(
                &campaign,
                "room",
                &ElaborationDispatch {
                    schema: "ghostlight.elaboration_dispatch.v1".into(),
                    budget_ordinal: ordinal as u64 + 1,
                    ordinal: ordinal as u64 + 1,
                    title,
                    title_weight: 1,
                    total_enabled_weight: 1,
                    requested_share_millionths: 1_000_000,
                    title_dispatch_count,
                },
            )
            .unwrap();
            let schema = assignment.action_schema("room").unwrap();
            let branches = schema
                .pointer("/$defs/WorldElaborationOperation/oneOf")
                .and_then(serde_json::Value::as_array)
                .unwrap();

            assert_eq!(branches.len(), 1);
            assert_eq!(
                branches[0]
                    .pointer("/properties/type/const")
                    .and_then(serde_json::Value::as_str),
                Some(expected_type)
            );
            let mut provider_schema = schema;
            crate::model_connector::project_strict_responses_schema(&mut provider_schema).unwrap();
            jsonschema::validator_for(&provider_schema).unwrap();
        }
    }

    #[test]
    fn assignment_rejection_returns_the_exact_correction_contract() {
        let campaign = campaign_with_civic_room();
        let assignment = WorldElaborationAssignment::for_dispatch(
            &campaign,
            "room",
            &ElaborationDispatch {
                schema: "ghostlight.elaboration_dispatch.v1".into(),
                budget_ordinal: 2,
                ordinal: 2,
                title: ElaboratorTitle::Charter,
                title_weight: 1,
                total_enabled_weight: 1,
                requested_share_millionths: 1_000_000,
                title_dispatch_count: 2,
            },
        )
        .unwrap();
        let wrong = WorldElaborationProposal {
            schema: "ghostlight.world_elaboration_proposal.v1".into(),
            operation: WorldElaborationOperation::AddFact {
                fact: crate::domain::WorldFact {
                    id: "wrong-id".into(),
                    statement: "A valid-shaped fact with the wrong assignment identity.".into(),
                    scope: crate::domain::FactScope::BranchLocal,
                    evidence_receipt_ids: vec![],
                    discoverable_at_location_ids: BTreeSet::from(["room".into()]),
                },
            },
        };

        let error = assignment.validate(&campaign, "room", &wrong).unwrap_err();

        assert!(
            error
                .to_string()
                .contains("correct it against this exact contract")
        );
        assert!(error.to_string().contains("elab:room:charter-fact:2"));
        assert!(error.to_string().contains("exactly one add_fact"));
    }

    #[tokio::test]
    async fn assignment_tool_rejects_a_relation_schema_bypass_then_accepts_the_correction() {
        let mut campaign = campaign_with_civic_room();
        campaign
            .civic_systems
            .get_mut("room")
            .unwrap()
            .governing_institution_ids =
            BTreeSet::from(["first-council".into(), "second-council".into()]);
        let assignment = WorldElaborationAssignment::for_dispatch(
            &campaign,
            "room",
            &ElaborationDispatch {
                schema: "ghostlight.elaboration_dispatch.v1".into(),
                budget_ordinal: 1,
                ordinal: 1,
                title: ElaboratorTitle::Tangle,
                title_weight: 1,
                total_enabled_weight: 1,
                requested_share_millionths: 1_000_000,
                title_dispatch_count: 1,
            },
        )
        .unwrap();
        let wrong = WorldElaborationProposal {
            schema: "ghostlight.world_elaboration_proposal.v1".into(),
            operation: WorldElaborationOperation::AddLocalRelation {
                relation: crate::domain::AgencyRelation {
                    schema: "wrong.relation.schema".into(),
                    id: "elab:room:tangle-relation:1".into(),
                    from_subject_id: "first-council".into(),
                    to_subject_id: "second-council".into(),
                    kind: crate::domain::AgencyRelationKind::Rivalry,
                    strength: 50,
                    active: true,
                    evidence_receipt_ids: vec![],
                },
            },
        };
        let mut tool = WorldElaborationAgentTool {
            campaign: &campaign,
            target_location_id: "room",
            assignment: &assignment,
            workbench: serde_json::json!({"schema":"test.workbench.v1"}),
        };
        let context = crate::agent::ModelAgentToolContext {
            source_receipt_ids: Vec::new(),
            current_model_receipt: None,
        };

        let rejected = crate::agent::ModelAgentTool::invoke(&mut tool, wrong, &context).await;
        match rejected {
            crate::agent::ModelAgentToolOutcome::Rejected { finding, .. } => {
                assert!(
                    finding
                        .diagnostic
                        .contains(crate::domain::AgencyRelation::SCHEMA)
                );
            }
            _ => panic!("wrong relation schema bypassed the assignment tool"),
        }

        let corrected = WorldElaborationProposal {
            schema: "ghostlight.world_elaboration_proposal.v1".into(),
            operation: WorldElaborationOperation::AddLocalRelation {
                relation: crate::domain::AgencyRelation {
                    schema: crate::domain::AgencyRelation::SCHEMA.into(),
                    id: "elab:room:tangle-relation:1".into(),
                    from_subject_id: "first-council".into(),
                    to_subject_id: "second-council".into(),
                    kind: crate::domain::AgencyRelationKind::Rivalry,
                    strength: 50,
                    active: true,
                    evidence_receipt_ids: vec![],
                },
            },
        };

        let accepted = crate::agent::ModelAgentTool::invoke(&mut tool, corrected, &context).await;
        assert!(matches!(
            accepted,
            crate::agent::ModelAgentToolOutcome::Accepted { .. }
        ));
    }

    #[tokio::test]
    async fn world_admission_rejects_a_successful_receiptless_agent_proposal() {
        let campaign = crate::kernel::tests::campaign();
        let mut scheduler =
            ElaborationScheduler::new(&profile(&[(ElaboratorTitle::Patina, 1)])).unwrap();
        let run = dispatch_elaboration_wave(
            &mut scheduler,
            world_elaboration_wave_binding(&campaign, "room").unwrap(),
            &BTreeSet::from([ElaboratorTitle::Patina]),
            1,
            1,
            Arc::new(ReceiptlessWorldWorker),
        )
        .await
        .unwrap();

        let error = admit_world_elaboration_wave(&campaign, "room", run).unwrap_err();

        assert!(error.to_string().contains("lacks model receipt custody"));
    }
}
