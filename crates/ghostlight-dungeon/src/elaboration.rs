use anyhow::{Result, anyhow};
use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::Arc;

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
    pub wave: ElaborationWaveBinding,
    pub schedule: ElaborationScheduleReceipt,
    pub invocations: Vec<ElaborationInvocation<Proposal>>,
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
