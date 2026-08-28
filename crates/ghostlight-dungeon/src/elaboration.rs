use anyhow::{Result, anyhow};
use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::Arc;
use uuid::Uuid;

pub const MAX_ELABORATOR_WEIGHT: u16 = 100;

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
}

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
    async fn invoke(&self, invocation: ElaborationSubAgentInvocation) -> Result<Proposal>;
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
            }],
        });
    }
    let schedule = scheduler.schedule(eligible_titles, invocation_budget);
    let semaphore = Arc::new(tokio::sync::Semaphore::new(parallelism));
    let mut jobs = tokio::task::JoinSet::new();
    let mut task_dispatches = HashMap::new();
    for dispatch in schedule.dispatches.iter().cloned() {
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
                    })?;
            let request = ElaborationSubAgentInvocation {
                wave: wave.clone(),
                dispatch: dispatch.clone(),
            };
            let proposal =
                worker
                    .invoke(request)
                    .await
                    .map_err(|error| ElaborationInvocationFailure {
                        dispatch: Some(dispatch.clone()),
                        diagnostic: error.to_string().chars().take(2_000).collect(),
                    })?;
            Ok::<_, ElaborationInvocationFailure>(ElaborationInvocation {
                wave,
                dispatch,
                proposal,
            })
        });
        task_dispatches.insert(task.id(), task_dispatch);
    }

    let mut invocations = Vec::with_capacity(schedule.dispatches.len());
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
#[derive(Clone, Debug, Serialize, JsonSchema, PartialEq)]
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
    candidate: Option<crate::domain::LocalityElaboration>,
    candidate_diagnostic: Option<String>,
    digest: String,
}

/// The only elaboration value accepted by `WorldKernel::commit_elaboration`.
/// It binds the immutable admission result to an independently generated
/// semantic-verifier receipt without letting that verifier rewrite the draft.
#[derive(Clone, Debug, Serialize, JsonSchema, PartialEq)]
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
    for invocation in invocations {
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
        candidate,
        candidate_diagnostic,
        digest: String::new(),
    };
    admission.digest = world_elaboration_admission_digest(&admission)?;
    Ok(admission)
}

fn validate_successful_wave_run<Proposal>(run: &ElaborationWaveRun<Proposal>) -> Result<()> {
    let schedule = &run.schedule;
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
        || run.invocations.len() != schedule.dispatches.len()
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

pub fn finalize_world_elaboration(
    campaign: &crate::domain::Campaign,
    admission: WorldElaborationAdmission,
    semantic_verifier_receipt: crate::model::ModelStageReceipt,
) -> Result<FinalizedWorldElaboration> {
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

    pub fn candidate(&self) -> Option<&crate::domain::LocalityElaboration> {
        self.candidate.as_ref()
    }

    pub fn candidate_diagnostic(&self) -> Option<&str> {
        self.candidate_diagnostic.as_deref()
    }

    pub fn digest(&self) -> &str {
        &self.digest
    }

    fn valid_candidate(
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

    pub(crate) fn into_kernel_parts(
        self,
        campaign: &crate::domain::Campaign,
    ) -> Result<(
        u64,
        crate::domain::LocalityElaboration,
        crate::model::ModelStageReceipt,
    )> {
        if self.schema != "ghostlight.finalized_world_elaboration.v1"
            || self.digest != finalized_world_elaboration_digest(&self)?
        {
            return Err(anyhow!(
                "finalized world elaboration is malformed or tampered"
            ));
        }
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
        Ok((
            self.admission.expected_revision,
            elaboration,
            self.semantic_verifier_receipt,
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
            Ok(vec![format!("fact:{}", fact.id)])
        }
        AddPopulation {
            population,
            profile,
        } => {
            nonempty(&population.id, "population id")?;
            if profile.subject_id != population.id
                || profile.subject_kind != crate::domain::AgencySubjectKind::Gestalt
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
        async fn invoke(&self, invocation: ElaborationSubAgentInvocation) -> Result<String> {
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
            Ok(format!("proposal:{}", dispatch.ordinal))
        }
    }

    struct PartiallyFailingSubAgent;

    #[async_trait]
    impl ElaborationSubAgentPort<String> for PartiallyFailingSubAgent {
        async fn invoke(&self, invocation: ElaborationSubAgentInvocation) -> Result<String> {
            if invocation.dispatch.ordinal == 3 {
                return Err(anyhow!("fixture refusal"));
            }
            if invocation.dispatch.ordinal == 4 {
                panic!("fixture panic");
            }
            Ok(format!("proposal:{}", invocation.dispatch.ordinal))
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
}
