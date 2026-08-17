use crate::domain::*;
use anyhow::{Result, anyhow};
use sha2::{Digest, Sha256};
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet, BinaryHeap, HashMap, VecDeque};

pub const MIN_ACTIVE_CELL_BUDGET: u8 = 1;
pub const MAX_ACTIVE_CELL_BUDGET: u8 = 32;
pub const MAX_PROVIDER_PARALLELISM: u8 = 32;

#[derive(Clone, Debug)]
struct Candidate {
    left: String,
    right: String,
    loss: MergeLoss,
}

impl PartialEq for Candidate {
    fn eq(&self, other: &Self) -> bool {
        self.loss.total.to_bits() == other.loss.total.to_bits()
            && self.left == other.left
            && self.right == other.right
    }
}
impl Eq for Candidate {}
impl PartialOrd for Candidate {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for Candidate {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .loss
            .total
            .total_cmp(&self.loss.total)
            .then_with(|| other.left.cmp(&self.left))
            .then_with(|| other.right.cmp(&self.right))
    }
}

pub fn default_demand(campaign: &Campaign, rationale: impl Into<String>) -> ResolutionDemand {
    ResolutionDemand {
        schema: "ghostlight.resolution_demand.v1".into(),
        campaign_id: campaign.id,
        world_revision: campaign.revision,
        resolution_epoch: campaign.resolution_policy.resolution_epoch,
        axis_weights: BTreeMap::from([
            (AgencyAxis::Geography, 0.25),
            (AgencyAxis::Ideology, 0.20),
            (AgencyAxis::Authority, 0.25),
            (AgencyAxis::EconomyRole, 0.15),
            (AgencyAxis::SpeciesBody, 0.05),
            (AgencyAxis::Information, 0.10),
        ]),
        focal_subject_ids: BTreeSet::new(),
        horizon_minutes: campaign.tick_hours.saturating_mul(60),
        rationale: rationale.into(),
    }
}

pub fn ensure_agency_profiles(campaign: &mut Campaign) {
    let evidence = campaign.branch_origin.evidence_receipt_ids.clone();
    for actor in campaign.actors.values() {
        campaign
            .agency_profiles
            .entry(actor.id.clone())
            .or_insert_with(|| profile_for_actor(actor, &evidence));
    }
    for institution in campaign.institutions.values() {
        campaign
            .agency_profiles
            .entry(institution.id.clone())
            .or_insert_with(|| profile_for_institution(institution, &evidence));
    }
    for gestalt in campaign.gestalts.values() {
        campaign
            .agency_profiles
            .entry(gestalt.id.clone())
            .or_insert_with(|| profile_for_gestalt(gestalt, &evidence));
    }
    for actor in campaign.actors.values() {
        if let Some(profile) = campaign.agency_profiles.get_mut(&actor.id) {
            profile.location_ids = BTreeSet::from([actor.location_id.clone()]);
            profile
                .information_channels
                .extend(actor.knowledge.iter().cloned());
        }
    }
    let live: BTreeSet<_> = campaign
        .actors
        .keys()
        .chain(campaign.institutions.keys())
        .chain(campaign.gestalts.keys())
        .cloned()
        .collect();
    campaign
        .agency_profiles
        .retain(|id, profile| live.contains(id) || !profile.active_leaf);
    if let Some(player) = campaign.agency_profiles.get_mut(&campaign.player_actor_id) {
        player.simulation_eligible = false;
    }
    ensure_structural_relations(campaign);
}

fn base_profile(
    subject_id: &str,
    kind: AgencySubjectKind,
    authority: Option<String>,
    evidence: &[String],
) -> AgencyProfile {
    AgencyProfile {
        schema: "ghostlight.agency_profile.v1".into(),
        id: format!("agency-profile:{subject_id}"),
        subject_id: subject_id.into(),
        subject_kind: kind,
        profile_version: 0,
        collective_authority_id: authority,
        parent_subject_id: None,
        active_leaf: true,
        simulation_eligible: true,
        facets: BTreeMap::new(),
        location_ids: BTreeSet::new(),
        information_channels: BTreeSet::new(),
        detail_debt: 0,
        last_detail_tick: 0,
        evidence_receipt_ids: evidence.to_vec(),
    }
}

fn profile_for_actor(actor: &ActorState, evidence: &[String]) -> AgencyProfile {
    let mut profile = base_profile(&actor.id, AgencySubjectKind::Actor, None, evidence);
    profile.location_ids.insert(actor.location_id.clone());
    profile.facets.insert(
        AgencyAxis::Geography,
        BTreeSet::from([actor.location_id.clone()]),
    );
    profile.facets.insert(
        AgencyAxis::EconomyRole,
        actor.capabilities.iter().cloned().collect(),
    );
    profile.facets.insert(
        AgencyAxis::Information,
        actor.knowledge.iter().cloned().collect(),
    );
    profile.information_channels = actor.knowledge.iter().cloned().collect();
    profile
}

fn profile_for_institution(value: &InstitutionState, evidence: &[String]) -> AgencyProfile {
    let mut profile = base_profile(
        &value.id,
        AgencySubjectKind::Institution,
        Some(value.id.clone()),
        evidence,
    );
    profile.facets.insert(
        AgencyAxis::Authority,
        BTreeSet::from([value.id.clone(), value.posture.clone()]),
    );
    profile.facets.insert(
        AgencyAxis::EconomyRole,
        value.resources.iter().cloned().collect(),
    );
    profile
}

fn profile_for_gestalt(value: &GestaltPersonaState, evidence: &[String]) -> AgencyProfile {
    let mut profile = base_profile(
        &value.id,
        AgencySubjectKind::Gestalt,
        Some(value.id.clone()),
        evidence,
    );
    profile.location_ids.insert(value.home_location_id.clone());
    profile.facets.insert(
        AgencyAxis::Geography,
        BTreeSet::from([value.home_location_id.clone()]),
    );
    profile.facets.insert(
        AgencyAxis::EconomyRole,
        value.shared_capabilities.iter().cloned().collect(),
    );
    profile.facets.insert(
        AgencyAxis::Information,
        value.shared_knowledge.iter().cloned().collect(),
    );
    profile.information_channels = value.shared_knowledge.iter().cloned().collect();
    profile
}

fn ensure_structural_relations(campaign: &mut Campaign) {
    let live: BTreeSet<_> = campaign
        .agency_profiles
        .values()
        .filter(|profile| profile.active_leaf)
        .map(|profile| profile.subject_id.as_str())
        .collect();
    campaign.agency_relations.retain(|_, relation| {
        live.contains(relation.from_subject_id.as_str())
            && live.contains(relation.to_subject_id.as_str())
    });
}

#[derive(Clone, Debug)]
struct DerivedAgencyGraph {
    neighbors: BTreeMap<String, BTreeSet<String>>,
}

#[derive(Debug)]
struct ScoringCache {
    pressure_tokens: BTreeMap<String, BTreeSet<String>>,
    salience: BTreeMap<String, f32>,
}

impl ScoringCache {
    fn new(campaign: &Campaign, profiles: &BTreeMap<String, AgencyProfile>) -> Self {
        Self {
            pressure_tokens: profiles
                .keys()
                .map(|id| (id.clone(), pressure_tokens(campaign, id)))
                .collect(),
            salience: profiles
                .iter()
                .map(|(id, profile)| (id.clone(), subject_salience(campaign, profile)))
                .collect(),
        }
    }
}

impl DerivedAgencyGraph {
    fn build(campaign: &Campaign, subjects: BTreeSet<String>) -> Self {
        let mut neighbors: BTreeMap<_, _> = subjects
            .iter()
            .map(|id| (id.clone(), BTreeSet::new()))
            .collect();
        {
            let mut connect = |left: &str, right: &str| {
                if left != right && subjects.contains(left) && subjects.contains(right) {
                    neighbors
                        .entry(left.to_owned())
                        .or_default()
                        .insert(right.to_owned());
                    neighbors
                        .entry(right.to_owned())
                        .or_default()
                        .insert(left.to_owned());
                }
            };
            for relation in campaign
                .agency_relations
                .values()
                .filter(|relation| relation.active)
            {
                connect(&relation.from_subject_id, &relation.to_subject_id);
            }
            let mut by_location: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
            for profile in campaign
                .agency_profiles
                .values()
                .filter(|profile| subjects.contains(&profile.subject_id))
            {
                for location in &profile.location_ids {
                    by_location
                        .entry(location.as_str())
                        .or_default()
                        .push(profile.subject_id.as_str());
                }
            }
            for values in by_location.values_mut() {
                values.sort_unstable();
                values.dedup();
                for pair in values.windows(2) {
                    connect(pair[0], pair[1]);
                }
            }
        }

        // A campaign root is a resolution-only Steiner vertex. Linking component
        // representatives through it keeps the cover coarsenable without claiming
        // that distant or opposed subjects share containment, knowledge, or authority.
        let mut seen = BTreeSet::new();
        let mut representatives = Vec::new();
        for start in &subjects {
            if !seen.insert(start.clone()) {
                continue;
            }
            representatives.push(start.clone());
            let mut queue = VecDeque::from([start.clone()]);
            while let Some(current) = queue.pop_front() {
                for next in neighbors.get(&current).into_iter().flatten() {
                    if seen.insert(next.clone()) {
                        queue.push_back(next.clone());
                    }
                }
            }
        }
        for pair in representatives.windows(2) {
            neighbors
                .entry(pair[0].clone())
                .or_default()
                .insert(pair[1].clone());
            neighbors
                .entry(pair[1].clone())
                .or_default()
                .insert(pair[0].clone());
        }
        Self { neighbors }
    }
}

pub fn validate_policy(policy: &ResolutionPolicy) -> Result<()> {
    if !(MIN_ACTIVE_CELL_BUDGET..=MAX_ACTIVE_CELL_BUDGET).contains(&policy.active_cell_budget)
        || policy.pending_active_cell_budget.is_some_and(|value| {
            !(MIN_ACTIVE_CELL_BUDGET..=MAX_ACTIVE_CELL_BUDGET).contains(&value)
        })
        || !(1..=MAX_PROVIDER_PARALLELISM).contains(&policy.provider_parallelism)
    {
        return Err(anyhow!("resolution policy is outside supported bounds"));
    }
    Ok(())
}

pub fn validate_pins(campaign: &Campaign, pins: &BTreeMap<String, ResolutionPin>) -> Result<()> {
    let subjects: BTreeSet<_> = campaign
        .agency_profiles
        .values()
        .filter(|profile| profile.active_leaf && profile.simulation_eligible)
        .map(|profile| profile.subject_id.as_str())
        .collect();
    for pin in pins.values() {
        if pin.id.trim().is_empty()
            || pin.reason.trim().is_empty()
            || pin.subject_ids.is_empty()
            || pin
                .subject_ids
                .iter()
                .any(|id| !subjects.contains(id.as_str()))
        {
            return Err(anyhow!(
                "resolution pin is malformed or references an unknown subject"
            ));
        }
        match pin.kind {
            ResolutionPinKind::MinimumIndividualDetail if pin.subject_ids.len() != 1 => {
                return Err(anyhow!(
                    "minimum-individual pin must name exactly one subject"
                ));
            }
            ResolutionPinKind::KeepTogether | ResolutionPinKind::KeepSeparate
                if pin.subject_ids.len() < 2 =>
            {
                return Err(anyhow!(
                    "group resolution pin must name at least two subjects"
                ));
            }
            _ => {}
        }
    }
    for together in pins
        .values()
        .filter(|pin| pin.kind == ResolutionPinKind::KeepTogether)
    {
        if pins
            .values()
            .filter(|pin| pin.kind == ResolutionPinKind::KeepSeparate)
            .any(|separate| {
                together
                    .subject_ids
                    .intersection(&separate.subject_ids)
                    .count()
                    >= 2
            })
            || pins
                .values()
                .filter(|pin| pin.kind == ResolutionPinKind::MinimumIndividualDetail)
                .any(|individual| !individual.subject_ids.is_disjoint(&together.subject_ids))
        {
            return Err(anyhow!(
                "resolution pins require the same subjects together and apart"
            ));
        }
    }
    Ok(())
}

pub fn plan_cover(campaign: &Campaign, demand: ResolutionDemand) -> Result<ResolutionCover> {
    let trace = std::env::var_os("GHOSTLIGHT_PARTITION_TRACE").is_some();
    let trace_started = std::time::Instant::now();
    validate_policy(&campaign.resolution_policy)?;
    validate_pins(campaign, &campaign.resolution_pins)?;
    validate_demand(campaign, &demand)?;
    let profiles: BTreeMap<_, _> = campaign
        .agency_profiles
        .iter()
        .filter(|(_, profile)| profile.active_leaf && profile.simulation_eligible)
        .map(|(id, profile)| (id.clone(), profile.clone()))
        .collect();
    if profiles.is_empty() {
        return Err(anyhow!("agency graph has no active subjects"));
    }
    let scoring = ScoringCache::new(campaign, &profiles);
    let mandatory = mandatory_subjects(campaign);
    let graph = DerivedAgencyGraph::build(campaign, profiles.keys().cloned().collect());
    if trace {
        eprintln!("partition graph: {:?}", trace_started.elapsed());
    }
    let mut groups: HashMap<String, BTreeSet<String>> = profiles
        .keys()
        .map(|id| (id.clone(), BTreeSet::from([id.clone()])))
        .collect();
    contract_together_pins(&mut groups, &campaign.resolution_pins, &mandatory);
    let mut group_samples: HashMap<String, Vec<String>> = groups
        .iter()
        .map(|(id, subjects)| (id.clone(), representative_sample(subjects.iter())))
        .collect();
    let mandatory_cells = groups
        .values()
        .filter(|subjects| !subjects.is_disjoint(&mandatory))
        .count();
    let configured = campaign.resolution_policy.active_cell_budget as usize;
    let target = configured.max(mandatory_cells);
    let active_subjects: BTreeSet<_> = profiles.keys().cloned().collect();
    let separate_floor = campaign
        .resolution_pins
        .values()
        .filter(|pin| pin.kind == ResolutionPinKind::KeepSeparate)
        .map(|pin| pin.subject_ids.intersection(&active_subjects).count())
        .max()
        .unwrap_or(0);
    let target = target.max(separate_floor);
    let mut neighbors = group_neighbors(&groups, &graph);
    let mut heap = candidates(
        campaign,
        &profiles,
        &groups,
        &group_samples,
        &neighbors,
        &scoring,
        &demand,
        &mandatory,
    );
    if trace {
        eprintln!("partition heap: {:?}", trace_started.elapsed());
    }
    let mut merge_serial = 0u64;
    let mut candidate_elapsed = std::time::Duration::ZERO;
    let mut heap_pops = 0usize;
    while groups.len() > target {
        let Some(best) = heap.pop() else { break };
        heap_pops += 1;
        if !groups.contains_key(&best.left)
            || !groups.contains_key(&best.right)
            || !neighbors
                .get(&best.left)
                .is_some_and(|ids| ids.contains(&best.right))
        {
            continue;
        }
        let left_group = groups
            .remove(&best.left)
            .ok_or_else(|| anyhow!("partition candidate vanished"))?;
        let right_group = groups
            .remove(&best.right)
            .ok_or_else(|| anyhow!("partition candidate vanished"))?;
        let (mut joined, smaller) = if left_group.len() >= right_group.len() {
            (left_group, right_group)
        } else {
            (right_group, left_group)
        };
        joined.extend(smaller);
        let left_sample = group_samples.remove(&best.left).unwrap_or_default();
        let right_sample = group_samples.remove(&best.right).unwrap_or_default();
        let joined_sample = representative_sample(left_sample.iter().chain(right_sample.iter()));
        let new_id = format!("merge-group:{merge_serial:016}");
        merge_serial = merge_serial.saturating_add(1);
        let joined_neighbors: BTreeSet<_> = neighbors
            .remove(&best.left)
            .unwrap_or_default()
            .union(&neighbors.remove(&best.right).unwrap_or_default())
            .filter(|id| *id != &best.left && *id != &best.right && groups.contains_key(*id))
            .cloned()
            .collect();
        for other in &joined_neighbors {
            if let Some(values) = neighbors.get_mut(other) {
                values.remove(&best.left);
                values.remove(&best.right);
                values.insert(new_id.clone());
            }
        }
        groups.insert(new_id.clone(), joined);
        group_samples.insert(new_id.clone(), joined_sample);
        neighbors.insert(new_id.clone(), joined_neighbors.clone());
        for other in joined_neighbors {
            let candidate = if trace {
                let candidate_started = std::time::Instant::now();
                let candidate = candidate_for(
                    campaign,
                    &profiles,
                    &groups,
                    &group_samples,
                    &new_id,
                    &other,
                    &scoring,
                    &demand,
                    &mandatory,
                );
                candidate_elapsed += candidate_started.elapsed();
                candidate
            } else {
                candidate_for(
                    campaign,
                    &profiles,
                    &groups,
                    &group_samples,
                    &new_id,
                    &other,
                    &scoring,
                    &demand,
                    &mandatory,
                )
            };
            if let Some(candidate) = candidate {
                heap.push(candidate);
            }
        }
    }
    if trace {
        eprintln!(
            "partition merge: {:?}; candidates {:?}; heap pops {}",
            trace_started.elapsed(),
            candidate_elapsed,
            heap_pops
        );
    }
    refine_boundaries(
        campaign,
        &profiles,
        &graph,
        &scoring,
        &demand,
        &mandatory,
        &mut groups,
    );
    if trace {
        eprintln!("partition refine: {:?}", trace_started.elapsed());
    }
    let highest_debt = profiles
        .values()
        .max_by(|left, right| {
            left.detail_debt
                .cmp(&right.detail_debt)
                .then_with(|| right.subject_id.cmp(&left.subject_id))
        })
        .map(|profile| profile.subject_id.clone());
    let mut cells = Vec::new();
    for subjects in groups.values() {
        let (mode, loss) = classify_and_score(campaign, &profiles, &scoring, subjects, &demand);
        cells.push(SimulationCell {
            schema: "ghostlight.simulation_cell.v1".into(),
            id: cell_id(subjects, &mode),
            mode,
            subject_ids: subjects.clone(),
            merge_loss: loss,
            rationale: cut_rationale(subjects, &demand),
            lease_until_world_revision: campaign.revision.saturating_add(5),
            lease_until_strategic_tick: campaign.strategic_tick_count.saturating_add(2),
            detail_focus_subject_id: highest_debt
                .as_ref()
                .filter(|id| subjects.contains(*id))
                .cloned(),
        });
    }
    cells.sort_by(|left, right| left.id.cmp(&right.id));
    if trace {
        eprintln!("partition cells: {:?}", trace_started.elapsed());
    }
    cells = preserve_previous_cover(
        campaign, &profiles, &scoring, &demand, &mandatory, target, cells,
    );
    validate_cover_with_graph(campaign, &demand, &cells, &graph)?;
    if trace {
        eprintln!("partition validate: {:?}", trace_started.elapsed());
    }
    let effective_budget = cells.len().min(u8::MAX as usize) as u8;
    Ok(ResolutionCover {
        schema: "ghostlight.resolution_cover.v1".into(),
        campaign_id: campaign.id,
        world_revision: campaign.revision,
        resolution_epoch: campaign.resolution_policy.resolution_epoch,
        configured_budget: campaign.resolution_policy.active_cell_budget,
        effective_budget,
        mandatory_overage: effective_budget
            .saturating_sub(campaign.resolution_policy.active_cell_budget),
        cells,
        demand,
    })
}

fn refine_boundaries(
    campaign: &Campaign,
    profiles: &BTreeMap<String, AgencyProfile>,
    graph: &DerivedAgencyGraph,
    scoring: &ScoringCache,
    demand: &ResolutionDemand,
    mandatory: &BTreeSet<String>,
    groups: &mut HashMap<String, BTreeSet<String>>,
) {
    let mut subject_group = HashMap::new();
    for (group_id, subjects) in groups.iter() {
        for subject in subjects {
            subject_group.insert(subject.clone(), group_id.clone());
        }
    }
    let subjects: Vec<_> = profiles.keys().cloned().collect();
    for subject in subjects {
        if mandatory.contains(&subject) {
            continue;
        }
        let Some(source_id) = subject_group.get(&subject).cloned() else {
            continue;
        };
        if groups
            .get(&source_id)
            .is_none_or(|values| values.len() <= 1)
        {
            continue;
        }
        let adjacent_groups: BTreeSet<_> = graph
            .neighbors
            .get(&subject)
            .into_iter()
            .flatten()
            .filter_map(|neighbor| subject_group.get(neighbor).cloned())
            .filter(|id| id != &source_id)
            .collect();
        if adjacent_groups.is_empty() {
            continue;
        }
        let Some(source) = groups.get(&source_id).cloned() else {
            continue;
        };
        for destination_id in adjacent_groups {
            let Some(destination) = groups.get(&destination_id).cloned() else {
                continue;
            };
            let mut next_source = source.clone();
            next_source.remove(&subject);
            let mut next_destination = destination.clone();
            next_destination.insert(subject.clone());
            if !connected(graph, &next_source)
                || !connected(graph, &next_destination)
                || violates_separate_pin(
                    &BTreeSet::from([subject.clone()]),
                    &destination,
                    &campaign.resolution_pins,
                )
                || breaks_together_pin(
                    &source_id,
                    &destination_id,
                    &subject,
                    groups,
                    &campaign.resolution_pins,
                )
            {
                continue;
            }
            let old_loss = compression_cost(
                &classify_and_score(campaign, profiles, scoring, &source, demand).1,
                source.len(),
            ) + compression_cost(
                &classify_and_score(campaign, profiles, scoring, &destination, demand).1,
                destination.len(),
            );
            let new_loss = compression_cost(
                &classify_and_score(campaign, profiles, scoring, &next_source, demand).1,
                next_source.len(),
            ) + compression_cost(
                &classify_and_score(campaign, profiles, scoring, &next_destination, demand).1,
                next_destination.len(),
            );
            if old_loss > f32::EPSILON && new_loss <= old_loss * 0.95 {
                groups.insert(source_id.clone(), next_source);
                groups.insert(destination_id.clone(), next_destination);
                subject_group.insert(subject.clone(), destination_id);
                break;
            }
        }
    }
}

fn breaks_together_pin(
    source_id: &str,
    destination_id: &str,
    subject: &str,
    groups: &HashMap<String, BTreeSet<String>>,
    pins: &BTreeMap<String, ResolutionPin>,
) -> bool {
    pins.values()
        .filter(|pin| {
            pin.kind == ResolutionPinKind::KeepTogether && pin.subject_ids.contains(subject)
        })
        .any(|pin| {
            pin.subject_ids.iter().any(|other| {
                other != subject
                    && groups
                        .get(source_id)
                        .is_some_and(|values| values.contains(other))
                    && !groups
                        .get(destination_id)
                        .is_some_and(|values| values.contains(other))
            })
        })
}

fn preserve_previous_cover(
    campaign: &Campaign,
    profiles: &BTreeMap<String, AgencyProfile>,
    scoring: &ScoringCache,
    demand: &ResolutionDemand,
    mandatory: &BTreeSet<String>,
    target: usize,
    candidate: Vec<SimulationCell>,
) -> Vec<SimulationCell> {
    let Some(previous) = campaign.resolution_cover.as_ref().filter(|cover| {
        cover.resolution_epoch == campaign.resolution_policy.resolution_epoch
            && cover.configured_budget == campaign.resolution_policy.active_cell_budget
            && cover.cells.len() == target
    }) else {
        return candidate;
    };
    if previous
        .cells
        .iter()
        .any(|cell| cell.subject_ids.len() > 1 && !cell.subject_ids.is_disjoint(mandatory))
    {
        return candidate;
    }
    let expected: BTreeSet<_> = profiles.keys().cloned().collect();
    let actual: BTreeSet<_> = previous
        .cells
        .iter()
        .flat_map(|cell| cell.subject_ids.iter().cloned())
        .collect();
    if actual != expected {
        return candidate;
    }
    let old_total: f32 = previous
        .cells
        .iter()
        .map(|cell| {
            compression_cost(
                &classify_and_score(campaign, profiles, scoring, &cell.subject_ids, demand).1,
                cell.subject_ids.len(),
            )
        })
        .sum();
    let new_total: f32 = candidate
        .iter()
        .map(|cell| compression_cost(&cell.merge_loss, cell.subject_ids.len()))
        .sum();
    let improvement = if old_total <= f32::EPSILON {
        0.0
    } else {
        ((old_total - new_total) / old_total).max(0.0)
    };
    let lease_active = previous.cells.iter().any(|cell| {
        campaign.revision < cell.lease_until_world_revision
            && campaign.strategic_tick_count < cell.lease_until_strategic_tick
    });
    if !lease_active && improvement >= 0.10 {
        return candidate;
    }
    let mut preserved: Vec<_> = previous
        .cells
        .iter()
        .map(|cell| {
            let (mode, merge_loss) =
                classify_and_score(campaign, profiles, scoring, &cell.subject_ids, demand);
            SimulationCell {
                schema: "ghostlight.simulation_cell.v1".into(),
                id: cell_id(&cell.subject_ids, &mode),
                mode,
                subject_ids: cell.subject_ids.clone(),
                merge_loss,
                rationale: format!("preserved partition: {}", cell.rationale),
                lease_until_world_revision: cell.lease_until_world_revision,
                lease_until_strategic_tick: cell.lease_until_strategic_tick,
                detail_focus_subject_id: None,
            }
        })
        .collect();
    if let Some(highest_debt) = profiles
        .values()
        .max_by(|left, right| {
            left.detail_debt
                .cmp(&right.detail_debt)
                .then_with(|| right.subject_id.cmp(&left.subject_id))
        })
        .map(|profile| profile.subject_id.as_str())
        && let Some(cell) = preserved
            .iter_mut()
            .find(|cell| cell.subject_ids.contains(highest_debt))
    {
        cell.detail_focus_subject_id = Some(highest_debt.to_owned());
    }
    preserved.sort_by(|left, right| left.id.cmp(&right.id));
    preserved
}

fn compression_cost(loss: &MergeLoss, constituent_count: usize) -> f32 {
    loss.total * constituent_count.saturating_sub(1) as f32
}

pub fn validate_demand(campaign: &Campaign, demand: &ResolutionDemand) -> Result<()> {
    let axes = [
        AgencyAxis::Geography,
        AgencyAxis::Ideology,
        AgencyAxis::Authority,
        AgencyAxis::EconomyRole,
        AgencyAxis::SpeciesBody,
        AgencyAxis::Information,
    ];
    if demand.campaign_id != campaign.id
        || demand.world_revision != campaign.revision
        || demand.resolution_epoch != campaign.resolution_policy.resolution_epoch
        || demand.horizon_minutes == 0
        || demand.rationale.trim().is_empty()
        || axes.iter().any(|axis| {
            demand
                .axis_weights
                .get(axis)
                .is_none_or(|weight| !weight.is_finite() || !(0.0..=1.0).contains(weight))
        })
        || demand
            .focal_subject_ids
            .iter()
            .any(|id| !campaign.agency_profiles.contains_key(id))
    {
        return Err(anyhow!("resolution demand is stale or malformed"));
    }
    let sum: f32 = demand.axis_weights.values().sum();
    if (sum - 1.0).abs() > 0.02 {
        return Err(anyhow!("resolution demand axis weights must sum to one"));
    }
    Ok(())
}

pub fn cell_action_limit(cell: &SimulationCell) -> usize {
    match cell.mode {
        SimulationCellMode::Cohesive => 1,
        SimulationCellMode::Arena => {
            let count = cell.subject_ids.len().max(1);
            let ceil_log = if count <= 1 {
                0
            } else {
                usize::BITS - (count - 1).leading_zeros()
            };
            4usize.min(1 + ceil_log as usize)
        }
    }
}

fn mandatory_subjects(campaign: &Campaign) -> BTreeSet<String> {
    let mut mandatory = BTreeSet::new();
    for pin in campaign.resolution_pins.values() {
        if pin.kind == ResolutionPinKind::MinimumIndividualDetail {
            mandatory.extend(pin.subject_ids.iter().cloned());
        }
    }
    for member in campaign.gestalt_members.values() {
        if member.relevance_lease_until_revision > campaign.revision
            && let Some(actor_id) = &member.materialized_actor_id
        {
            mandatory.insert(actor_id.clone());
        }
    }
    mandatory
}

fn contract_together_pins(
    groups: &mut HashMap<String, BTreeSet<String>>,
    pins: &BTreeMap<String, ResolutionPin>,
    mandatory: &BTreeSet<String>,
) {
    for pin in pins
        .values()
        .filter(|pin| pin.kind == ResolutionPinKind::KeepTogether)
    {
        if !pin.subject_ids.is_disjoint(mandatory) {
            continue;
        }
        let keys: Vec<_> = groups
            .iter()
            .filter(|(_, subjects)| !subjects.is_disjoint(&pin.subject_ids))
            .map(|(key, _)| key.clone())
            .collect();
        let mut joined = BTreeSet::new();
        for key in keys {
            if let Some(subjects) = groups.remove(&key) {
                joined.extend(subjects);
            }
        }
        if !joined.is_empty() {
            groups.insert(cell_id(&joined, &SimulationCellMode::Arena), joined);
        }
    }
}

fn candidates(
    campaign: &Campaign,
    profiles: &BTreeMap<String, AgencyProfile>,
    groups: &HashMap<String, BTreeSet<String>>,
    group_samples: &HashMap<String, Vec<String>>,
    neighbors: &HashMap<String, BTreeSet<String>>,
    scoring: &ScoringCache,
    demand: &ResolutionDemand,
    mandatory: &BTreeSet<String>,
) -> BinaryHeap<Candidate> {
    let mut heap = BinaryHeap::new();
    for (left, adjacent) in neighbors {
        for right in adjacent.iter().filter(|right| left < *right) {
            if let Some(candidate) = candidate_for(
                campaign,
                profiles,
                groups,
                group_samples,
                left,
                right,
                scoring,
                demand,
                mandatory,
            ) {
                heap.push(candidate);
            }
        }
    }
    heap
}

fn candidate_for(
    campaign: &Campaign,
    profiles: &BTreeMap<String, AgencyProfile>,
    groups: &HashMap<String, BTreeSet<String>>,
    group_samples: &HashMap<String, Vec<String>>,
    left_id: &str,
    right_id: &str,
    scoring: &ScoringCache,
    demand: &ResolutionDemand,
    mandatory: &BTreeSet<String>,
) -> Option<Candidate> {
    let left = groups.get(left_id)?;
    let right = groups.get(right_id)?;
    if !left.is_disjoint(mandatory)
        || !right.is_disjoint(mandatory)
        || violates_separate_pin(left, right, &campaign.resolution_pins)
    {
        return None;
    }
    let sample = representative_sample(
        group_samples
            .get(left_id)?
            .iter()
            .chain(group_samples.get(right_id)?.iter()),
    );
    let loss = score_merge(campaign, profiles, scoring, left, right, &sample, demand);
    let (left, right) = if left_id <= right_id {
        (left_id.to_owned(), right_id.to_owned())
    } else {
        (right_id.to_owned(), left_id.to_owned())
    };
    Some(Candidate { left, right, loss })
}

fn group_neighbors(
    groups: &HashMap<String, BTreeSet<String>>,
    graph: &DerivedAgencyGraph,
) -> HashMap<String, BTreeSet<String>> {
    let mut subject_group = HashMap::new();
    for (group_id, subjects) in groups {
        for subject in subjects {
            subject_group.insert(subject.clone(), group_id.clone());
        }
    }
    let mut result: HashMap<_, _> = groups
        .keys()
        .map(|id| (id.clone(), BTreeSet::new()))
        .collect();
    for (subject, adjacent) in &graph.neighbors {
        let Some(left) = subject_group.get(subject) else {
            continue;
        };
        for other in adjacent {
            let Some(right) = subject_group.get(other) else {
                continue;
            };
            if left != right {
                result
                    .entry(left.clone())
                    .or_default()
                    .insert(right.clone());
                result
                    .entry(right.clone())
                    .or_default()
                    .insert(left.clone());
            }
        }
    }
    result
}

fn violates_separate_pin(
    left: &BTreeSet<String>,
    right: &BTreeSet<String>,
    pins: &BTreeMap<String, ResolutionPin>,
) -> bool {
    pins.values()
        .filter(|pin| pin.kind == ResolutionPinKind::KeepSeparate)
        .any(|pin| !left.is_disjoint(&pin.subject_ids) && !right.is_disjoint(&pin.subject_ids))
}

fn classify_and_score(
    campaign: &Campaign,
    profiles: &BTreeMap<String, AgencyProfile>,
    scoring: &ScoringCache,
    subjects: &BTreeSet<String>,
    demand: &ResolutionDemand,
) -> (SimulationCellMode, MergeLoss) {
    if subjects.len() <= 1 {
        return (SimulationCellMode::Cohesive, MergeLoss::default());
    }
    let sampled = bounded_profiles(profiles, subjects);
    classify_selection(
        campaign,
        profiles,
        scoring,
        sampled,
        subjects.iter(),
        |id| subjects.contains(id),
        subjects.len(),
        demand,
    )
}

fn score_merge(
    campaign: &Campaign,
    profiles: &BTreeMap<String, AgencyProfile>,
    scoring: &ScoringCache,
    left: &BTreeSet<String>,
    right: &BTreeSet<String>,
    sample_subject_ids: &[String],
    demand: &ResolutionDemand,
) -> MergeLoss {
    let constituent_count = left.len() + right.len();
    let sampled = sample_subject_ids
        .iter()
        .filter_map(|id| profiles.get(id))
        .collect();
    classify_selection(
        campaign,
        profiles,
        scoring,
        sampled,
        std::iter::empty::<&String>(),
        |id| left.contains(id) || right.contains(id),
        constituent_count,
        demand,
    )
    .1
}

fn classify_selection<'a, I, F>(
    campaign: &Campaign,
    profiles: &BTreeMap<String, AgencyProfile>,
    scoring: &ScoringCache,
    sampled: Vec<&AgencyProfile>,
    authority_subjects: I,
    contains: F,
    constituent_count: usize,
    demand: &ResolutionDemand,
) -> (SimulationCellMode, MergeLoss)
where
    I: IntoIterator<Item = &'a String>,
    F: Fn(&str) -> bool,
{
    let facet = weighted_facet_divergence(&sampled, demand);
    let information = set_divergence(
        &sampled
            .iter()
            .map(|profile| &profile.information_channels)
            .collect::<Vec<_>>(),
    );
    let spatial = spatial_divergence(campaign, &sampled, demand.horizon_minutes);
    let obligation_sets: Vec<_> = sampled
        .iter()
        .filter_map(|profile| scoring.pressure_tokens.get(&profile.subject_id))
        .collect();
    let clock_obligation = set_divergence(&obligation_sets);
    let mut authorities = BTreeSet::new();
    for subject in authority_subjects {
        if let Some(authority) = profiles
            .get(subject)
            .and_then(|profile| profile.collective_authority_id.as_deref())
        {
            authorities.insert(authority);
            if authorities.len() > 1 {
                break;
            }
        }
    }
    let hostile = campaign.agency_relations.values().any(|relation| {
        relation.active
            && matches!(
                relation.kind,
                AgencyRelationKind::Rivalry | AgencyRelationKind::Coercion
            )
            && contains(&relation.from_subject_id)
            && contains(&relation.to_subject_id)
    });
    let boundary = hidden_boundary_mass_where(campaign, &contains);
    let salience = sampled
        .iter()
        .filter_map(|profile| scoring.salience.get(&profile.subject_id).copied())
        .fold(0.0_f32, f32::max);
    let churn = campaign.resolution_cover.as_ref().is_some_and(|cover| {
        !cover.cells.iter().any(|cell| {
            cell.subject_ids.len() == constituent_count
                && cell.subject_ids.iter().all(|id| contains(id))
        })
    }) as u8 as f32;
    let total = 0.25 * facet
        + 0.20 * boundary
        + 0.15 * information
        + 0.15 * spatial
        + 0.10 * clock_obligation
        + 0.10 * salience
        + 0.05 * churn;
    let loss = MergeLoss {
        facet_divergence: facet,
        hidden_boundary_mass: boundary,
        information_divergence: information,
        spatial_divergence: spatial,
        clock_obligation_divergence: clock_obligation,
        salience_burial: salience,
        partition_churn: churn,
        total,
    };
    let cohesive = authorities.len() == 1
        && !hostile
        && information <= 0.20
        && facet <= 0.25
        && clock_obligation <= 0.25;
    (
        if cohesive {
            SimulationCellMode::Cohesive
        } else {
            SimulationCellMode::Arena
        },
        loss,
    )
}

fn bounded_profiles<'a>(
    profiles: &'a BTreeMap<String, AgencyProfile>,
    subjects: &BTreeSet<String>,
) -> Vec<&'a AgencyProfile> {
    let stride = subjects.len().div_ceil(PROFILE_SAMPLE_LIMIT).max(1);
    subjects
        .iter()
        .step_by(stride)
        .take(PROFILE_SAMPLE_LIMIT)
        .filter_map(|id| profiles.get(id))
        .collect()
}

const PROFILE_SAMPLE_LIMIT: usize = 8;

fn representative_sample<'a>(subjects: impl Iterator<Item = &'a String>) -> Vec<String> {
    let mut ranked = subjects
        .map(|id| (sample_rank(id), id.clone()))
        .collect::<Vec<_>>();
    ranked.sort_unstable();
    ranked.dedup_by(|left, right| left.1 == right.1);
    ranked.truncate(PROFILE_SAMPLE_LIMIT);
    ranked.into_iter().map(|(_, id)| id).collect()
}

fn sample_rank(id: &str) -> u64 {
    id.as_bytes().iter().fold(0xcbf29ce484222325, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(0x100000001b3)
    })
}

fn pressure_tokens(campaign: &Campaign, subject_id: &str) -> BTreeSet<String> {
    if let Some(actor) = campaign.actors.get(subject_id) {
        return actor
            .obligations
            .iter()
            .cloned()
            .chain(actor.goals.iter().map(|value| format!("goal:{value}")))
            .chain(
                actor
                    .conditions
                    .iter()
                    .map(|value| format!("condition:{value}")),
            )
            .collect();
    }
    if let Some(institution) = campaign.institutions.get(subject_id) {
        return institution
            .goals
            .iter()
            .map(|value| format!("goal:{value}"))
            .chain(std::iter::once(format!("posture:{}", institution.posture)))
            .collect();
    }
    campaign
        .gestalts
        .get(subject_id)
        .map(|gestalt| {
            gestalt
                .goals
                .iter()
                .map(|value| format!("goal:{value}"))
                .chain(
                    gestalt
                        .pressures
                        .iter()
                        .map(|value| format!("pressure:{value}")),
                )
                .collect()
        })
        .unwrap_or_default()
}

fn subject_salience(campaign: &Campaign, profile: &AgencyProfile) -> f32 {
    let debt = profile.detail_debt.min(64) as f32 / 64.0;
    let event = campaign.events.iter().rev().take(16).any(|event| {
        event.actor_ids.contains(&profile.subject_id)
            || event.institution_ids.contains(&profile.subject_id)
    }) as u8 as f32;
    let conflict = campaign.agency_relations.values().any(|relation| {
        relation.active
            && matches!(
                relation.kind,
                AgencyRelationKind::Rivalry | AgencyRelationKind::Coercion
            )
            && (relation.from_subject_id == profile.subject_id
                || relation.to_subject_id == profile.subject_id)
    }) as u8 as f32;
    (0.45 * debt + 0.30 * event + 0.25 * conflict).clamp(0.0, 1.0)
}

fn spatial_divergence(
    campaign: &Campaign,
    profiles: &[&AgencyProfile],
    horizon_minutes: u32,
) -> f32 {
    if profiles.len() <= 1 {
        return 0.0;
    }
    let mut total = 0.0;
    let mut pairs = 0u32;
    for (index, left) in profiles.iter().enumerate() {
        for right in profiles.iter().skip(index + 1) {
            let distance = if !left.location_ids.is_disjoint(&right.location_ids) {
                0.0
            } else {
                let minutes = left
                    .location_ids
                    .iter()
                    .flat_map(|location| {
                        campaign
                            .locations
                            .get(location)
                            .into_iter()
                            .flat_map(|value| value.routes.values())
                    })
                    .filter(|route| right.location_ids.contains(&route.destination_id))
                    .map(|route| route.travel_minutes)
                    .min();
                minutes
                    .map(|minutes| minutes as f32 / horizon_minutes.max(1) as f32)
                    .unwrap_or(1.0)
                    .clamp(0.0, 1.0)
            };
            total += distance;
            pairs += 1;
        }
    }
    total / pairs.max(1) as f32
}

fn weighted_facet_divergence(profiles: &[&AgencyProfile], demand: &ResolutionDemand) -> f32 {
    demand
        .axis_weights
        .iter()
        .map(|(axis, weight)| {
            let empty = BTreeSet::new();
            let sets: Vec<_> = profiles
                .iter()
                .map(|profile| profile.facets.get(axis).unwrap_or(&empty))
                .collect();
            weight * set_divergence(&sets)
        })
        .sum::<f32>()
        .clamp(0.0, 1.0)
}

fn set_divergence(sets: &[&BTreeSet<String>]) -> f32 {
    if sets.len() <= 1 {
        return 0.0;
    }
    let mut total = 0.0;
    let mut pairs = 0;
    for (index, left) in sets.iter().enumerate() {
        for right in sets.iter().skip(index + 1) {
            let union = left.union(right).count();
            let similarity = if union == 0 {
                1.0
            } else {
                left.intersection(right).count() as f32 / union as f32
            };
            total += 1.0 - similarity;
            pairs += 1;
        }
    }
    if pairs == 0 {
        0.0
    } else {
        total / pairs as f32
    }
}

fn hidden_boundary_mass_where<F>(campaign: &Campaign, contains: &F) -> f32
where
    F: Fn(&str) -> bool,
{
    let mut mass = 0u32;
    let mut possible = 0u32;
    for relation in campaign
        .agency_relations
        .values()
        .filter(|relation| relation.active)
    {
        if contains(&relation.from_subject_id) && contains(&relation.to_subject_id) {
            possible += 100;
            if !matches!(
                relation.kind,
                AgencyRelationKind::Containment | AgencyRelationKind::SharedLocation
            ) {
                mass += u32::from(relation.strength);
            }
        }
    }
    if possible == 0 {
        0.0
    } else {
        mass as f32 / possible as f32
    }
}

fn cut_rationale(subjects: &BTreeSet<String>, demand: &ResolutionDemand) -> String {
    let axis = demand
        .axis_weights
        .iter()
        .max_by(|left, right| left.1.total_cmp(right.1).then_with(|| left.0.cmp(right.0)))
        .map(|(axis, _)| format!("{axis:?}").to_lowercase())
        .unwrap_or_else(|| "mixed".into());
    format!(
        "{} subjects grouped under {axis}-weighted demand",
        subjects.len()
    )
}

fn cell_id(subjects: &BTreeSet<String>, mode: &SimulationCellMode) -> String {
    let material = format!(
        "{mode:?}:{}",
        subjects.iter().cloned().collect::<Vec<_>>().join("|")
    );
    format!("cell:{:x}", Sha256::digest(material.as_bytes()))
}

pub fn validate_cover(
    campaign: &Campaign,
    demand: &ResolutionDemand,
    cells: &[SimulationCell],
) -> Result<()> {
    let graph = DerivedAgencyGraph::build(
        campaign,
        campaign
            .agency_profiles
            .values()
            .filter(|profile| profile.active_leaf && profile.simulation_eligible)
            .map(|profile| profile.subject_id.clone())
            .collect(),
    );
    validate_cover_with_graph(campaign, demand, cells, &graph)
}

fn validate_cover_with_graph(
    campaign: &Campaign,
    demand: &ResolutionDemand,
    cells: &[SimulationCell],
    graph: &DerivedAgencyGraph,
) -> Result<()> {
    let scoring = ScoringCache::new(campaign, &campaign.agency_profiles);
    let expected: BTreeSet<_> = campaign
        .agency_profiles
        .values()
        .filter(|profile| profile.active_leaf && profile.simulation_eligible)
        .map(|profile| profile.subject_id.clone())
        .collect();
    let mut actual = BTreeSet::new();
    for cell in cells {
        if cell.subject_ids.is_empty()
            || cell.subject_ids.iter().any(|id| !actual.insert(id.clone()))
            || (cell.subject_ids.len() > 1 && !connected(graph, &cell.subject_ids))
            || cell.id != cell_id(&cell.subject_ids, &cell.mode)
            || (cell.mode == SimulationCellMode::Cohesive
                && classify_and_score(
                    campaign,
                    &campaign.agency_profiles,
                    &scoring,
                    &cell.subject_ids,
                    demand,
                )
                .0 != SimulationCellMode::Cohesive)
        {
            return Err(anyhow!("resolution cover contains an invalid cell"));
        }
    }
    if actual != expected {
        return Err(anyhow!(
            "resolution cover does not cover every active subject exactly once"
        ));
    }
    Ok(())
}

fn connected(graph: &DerivedAgencyGraph, subjects: &BTreeSet<String>) -> bool {
    let Some(start) = subjects.iter().next() else {
        return false;
    };
    let mut seen = BTreeSet::from([start.clone()]);
    let mut queue = VecDeque::from([start.clone()]);
    while let Some(current) = queue.pop_front() {
        for next in graph.neighbors.get(&current).into_iter().flatten() {
            if subjects.contains(next) && seen.insert(next.clone()) {
                queue.push_back(next.clone());
            }
        }
    }
    seen == *subjects
}

pub fn advance_detail_debt(campaign: &mut Campaign, cover: &ResolutionCover) {
    let focused: BTreeSet<_> = cover
        .cells
        .iter()
        .filter_map(|cell| cell.detail_focus_subject_id.clone())
        .chain(
            cover
                .cells
                .iter()
                .filter(|cell| cell.subject_ids.len() == 1)
                .flat_map(|cell| cell.subject_ids.iter().cloned()),
        )
        .collect();
    for profile in campaign
        .agency_profiles
        .values_mut()
        .filter(|profile| profile.active_leaf)
    {
        if focused.contains(&profile.subject_id) {
            profile.detail_debt = 0;
            profile.last_detail_tick = campaign.strategic_tick_count.saturating_add(1);
        } else {
            profile.detail_debt = profile.detail_debt.saturating_add(1);
        }
        profile.profile_version = profile.profile_version.saturating_add(1);
    }
}

pub fn plan_receipt(campaign: &Campaign, cover: &ResolutionCover) -> ResolutionPlanReceipt {
    let previous: BTreeSet<_> = campaign
        .resolution_cover
        .as_ref()
        .map(|value| value.cells.iter().map(|cell| cell.id.clone()).collect())
        .unwrap_or_default();
    let current: BTreeSet<_> = cover.cells.iter().map(|cell| cell.id.clone()).collect();
    let preserved_cell_ids = current.intersection(&previous).cloned().collect::<Vec<_>>();
    let collapsed_boundaries = cover
        .cells
        .iter()
        .filter(|cell| cell.subject_ids.len() > 1)
        .map(|cell| {
            format!(
                "{}:{}",
                match cell.mode {
                    SimulationCellMode::Cohesive => "cohesive",
                    SimulationCellMode::Arena => "arena",
                },
                cell.subject_ids
                    .iter()
                    .cloned()
                    .collect::<Vec<_>>()
                    .join("|")
            )
        })
        .collect();
    ResolutionPlanReceipt {
        schema: "ghostlight.resolution_plan_receipt.v1".into(),
        campaign_id: campaign.id,
        world_revision: campaign.revision,
        resolution_epoch: campaign.resolution_policy.resolution_epoch,
        configured_budget: cover.configured_budget,
        effective_budget: cover.effective_budget,
        cell_ids: cover.cells.iter().map(|cell| cell.id.clone()).collect(),
        mandatory_overage: cover.mandatory_overage,
        preserved_cell_ids,
        collapsed_boundaries,
        merge_losses: cover
            .cells
            .iter()
            .map(|cell| (cell.id.clone(), cell.merge_loss.clone()))
            .collect(),
        rationale: format!(
            "{} active cells under {:?}-weighted pressure; {} mandatory overage",
            cover.effective_budget,
            cover
                .demand
                .axis_weights
                .iter()
                .max_by(|left, right| left.1.total_cmp(right.1))
                .map(|(axis, _)| axis),
            cover.mandatory_overage
        ),
        created_at: chrono::Utc::now(),
    }
}

pub fn validate_and_resolve_wave(
    campaign: &Campaign,
    wave: &ResolutionWaveCommit,
) -> Result<StrategicTickPlan> {
    if wave.world_revision != campaign.revision
        || wave.resolution_epoch != campaign.resolution_policy.resolution_epoch
        || wave.cover.world_revision != campaign.revision
        || wave.cover.resolution_epoch != campaign.resolution_policy.resolution_epoch
        || wave.plan_receipt.world_revision != campaign.revision
        || wave.plan_receipt.resolution_epoch != campaign.resolution_policy.resolution_epoch
        || wave
            .model_receipt_hashes
            .iter()
            .any(|hash| !is_sha256(hash))
    {
        return Err(anyhow!("resolution wave is stale or malformed"));
    }
    validate_demand(campaign, &wave.cover.demand)?;
    validate_cover(campaign, &wave.cover.demand, &wave.cover.cells)?;
    if wave.plan_receipt.cell_ids
        != wave
            .cover
            .cells
            .iter()
            .map(|cell| cell.id.clone())
            .collect::<Vec<_>>()
    {
        return Err(anyhow!("resolution receipt does not bind the active cover"));
    }
    let cells: BTreeMap<_, _> = wave
        .cover
        .cells
        .iter()
        .map(|cell| (cell.id.as_str(), cell))
        .collect();
    if wave.appraisals.len() != cells.len() {
        return Err(anyhow!(
            "every active cell must appraise the wave exactly once"
        ));
    }
    let mut seen_cells = BTreeSet::new();
    let mut proposals = Vec::new();
    for appraisal in &wave.appraisals {
        let cell = cells
            .get(appraisal.cell_id.as_str())
            .ok_or_else(|| anyhow!("cell appraisal references an inactive cell"))?;
        if !seen_cells.insert(appraisal.cell_id.as_str())
            || appraisal.world_revision != campaign.revision
            || appraisal.resolution_epoch != campaign.resolution_policy.resolution_epoch
            || appraisal.considered_subject_ids != cell.subject_ids
            || (appraisal.actions.is_empty()
                && appraisal
                    .inaction_reason
                    .as_deref()
                    .is_none_or(|reason| reason.trim().is_empty()))
        {
            return Err(anyhow!("cell appraisal is incomplete or stale"));
        }
        let quota = cell_action_limit(cell);
        if appraisal.actions.len() > quota {
            return Err(anyhow!("cell appraisal exceeds its action quota"));
        }
        for proposal in &appraisal.actions {
            validate_cell_proposal(campaign, cell, proposal)?;
            proposals.push(proposal.clone());
        }
    }
    proposals.sort_by(|left, right| {
        right
            .priority
            .cmp(&left.priority)
            .then_with(|| left.subject_id.cmp(&right.subject_id))
            .then_with(|| left.intent.cmp(&right.intent))
    });
    let consequence_limit = 16usize.min(usize::from(wave.cover.effective_budget) * 2);
    let mut used = BTreeSet::new();
    let mut plan = StrategicTickPlan::default();
    for proposal in proposals {
        let key = proposal_target_key(&proposal.effect);
        if !used.insert(key) {
            continue;
        }
        match proposal.effect {
            StrategicCellEffect::Institution {
                institution_id,
                posture,
                location_ids,
            } => plan.institution_actions.push(StrategicInstitutionAction {
                institution_id,
                posture,
                summary: proposal.intended_effect,
                location_ids,
                public_channels: proposal.public_channels,
            }),
            StrategicCellEffect::Gestalt {
                gestalt_id,
                pressure_additions,
            } => plan.gestalt_actions.push(StrategicGestaltAction {
                gestalt_id,
                summary: proposal.intended_effect,
                pressure_additions,
                public_channels: proposal.public_channels,
            }),
            StrategicCellEffect::ActorMove {
                actor_id,
                destination_id,
            } => plan.actor_moves.push(StrategicActorMove {
                actor_id,
                destination_id,
                summary: proposal.intended_effect,
                public_channels: proposal.public_channels,
            }),
            StrategicCellEffect::MemberMigration {
                member_id,
                source_gestalt_id,
                destination_gestalt_id,
                destination_location_id,
            } => plan.member_migrations.push(StrategicMemberMigration {
                member_id,
                source_gestalt_id,
                destination_gestalt_id,
                destination_location_id,
                summary: proposal.intended_effect,
                public_channels: proposal.public_channels,
            }),
        }
        if plan.institution_actions.len()
            + plan.gestalt_actions.len()
            + plan.actor_moves.len()
            + plan.member_migrations.len()
            >= consequence_limit
        {
            break;
        }
    }
    Ok(plan)
}

fn validate_cell_proposal(
    campaign: &Campaign,
    cell: &SimulationCell,
    proposal: &CellActionProposal,
) -> Result<()> {
    if proposal.subject_id == campaign.player_actor_id
        || proposal.intent.trim().is_empty()
        || proposal.intended_effect.trim().is_empty()
    {
        return Err(anyhow!("cell proposal has no exact constituent authority"));
    }
    if let Some(member_id) = proposal.subject_id.strip_prefix("member:") {
        return validate_member_cell_proposal(campaign, cell, member_id, proposal);
    }
    if !cell.subject_ids.contains(&proposal.subject_id) {
        return Err(anyhow!("cell proposal has no exact constituent authority"));
    }
    let profile = campaign
        .agency_profiles
        .get(&proposal.subject_id)
        .ok_or_else(|| anyhow!("cell proposal constituent lacks an agency profile"))?;
    let permitted_references = subject_state_references(campaign, &proposal.subject_id)?;
    if proposal
        .state_references
        .iter()
        .any(|reference| !permitted_references.contains(reference))
        || proposal
            .public_channels
            .iter()
            .any(|channel| !profile.information_channels.contains(channel))
    {
        return Err(anyhow!(
            "cell proposal exceeds constituent knowledge, resources, or information scope"
        ));
    }
    match &proposal.effect {
        StrategicCellEffect::Institution {
            institution_id,
            location_ids,
            ..
        } => {
            if institution_id != &proposal.subject_id
                || !campaign.institutions.contains_key(institution_id)
                || location_ids
                    .iter()
                    .any(|id| !campaign.locations.contains_key(id))
                || location_ids
                    .iter()
                    .any(|id| !profile.location_ids.contains(id))
            {
                return Err(anyhow!("institution proposal exceeds constituent state"));
            }
        }
        StrategicCellEffect::Gestalt { gestalt_id, .. } => {
            if gestalt_id != &proposal.subject_id || !campaign.gestalts.contains_key(gestalt_id) {
                return Err(anyhow!("gestalt proposal exceeds constituent state"));
            }
        }
        StrategicCellEffect::ActorMove {
            actor_id,
            destination_id,
        } => {
            let actor = campaign
                .actors
                .get(actor_id)
                .filter(|_| actor_id == &proposal.subject_id)
                .ok_or_else(|| anyhow!("actor proposal exceeds constituent state"))?;
            let reachable = campaign
                .locations
                .get(&actor.location_id)
                .is_some_and(|location| {
                    location.routes.values().any(|route| {
                        route.destination_id == *destination_id
                            && route.travel_minutes <= campaign.tick_hours.saturating_mul(60)
                    })
                });
            if !reachable {
                return Err(anyhow!("actor proposal exceeds spatial reach"));
            }
        }
        StrategicCellEffect::MemberMigration { .. } => {
            return Err(anyhow!(
                "a population, institution, actor, or arena cannot migrate a named member"
            ));
        }
    }
    Ok(())
}

fn validate_member_cell_proposal(
    campaign: &Campaign,
    cell: &SimulationCell,
    member_id: &str,
    proposal: &CellActionProposal,
) -> Result<()> {
    let member = campaign
        .gestalt_members
        .get(member_id)
        .filter(|member| {
            member.materialized_actor_id.is_none() && cell.subject_ids.contains(&member.gestalt_id)
        })
        .ok_or_else(|| anyhow!("named member is not a dematerialized exception of this cell"))?;
    let permitted_references = member_state_references(campaign, member_id)?;
    let information_channels = effective_member_knowledge(campaign, member_id)?;
    if proposal
        .state_references
        .iter()
        .any(|reference| !permitted_references.contains(reference))
        || proposal
            .public_channels
            .iter()
            .any(|channel| !information_channels.contains(channel))
    {
        return Err(anyhow!(
            "named member proposal borrowed another subject's state or information channel"
        ));
    }
    match &proposal.effect {
        StrategicCellEffect::MemberMigration {
            member_id: effect_member_id,
            source_gestalt_id,
            destination_gestalt_id,
            destination_location_id,
        } if effect_member_id == member_id && source_gestalt_id == &member.gestalt_id => {
            validate_member_migration(
                campaign,
                member_id,
                source_gestalt_id,
                destination_gestalt_id,
                destination_location_id,
            )
        }
        _ => Err(anyhow!(
            "named member exception may propose only its own validated migration"
        )),
    }
}

pub fn validate_member_migration(
    campaign: &Campaign,
    member_id: &str,
    source_gestalt_id: &str,
    destination_gestalt_id: &str,
    destination_location_id: &str,
) -> Result<()> {
    let member = campaign
        .gestalt_members
        .get(member_id)
        .filter(|member| {
            member.materialized_actor_id.is_none() && member.gestalt_id == source_gestalt_id
        })
        .ok_or_else(|| anyhow!("member migration source is stale or individually active"))?;
    if source_gestalt_id == destination_gestalt_id {
        return Err(anyhow!(
            "member migration must change active population leaf"
        ));
    }
    let source = campaign
        .gestalts
        .get(source_gestalt_id)
        .ok_or_else(|| anyhow!("member migration source gestalt is unknown"))?;
    let destination = campaign
        .gestalts
        .get(destination_gestalt_id)
        .filter(|destination| destination.home_location_id == destination_location_id)
        .ok_or_else(|| anyhow!("member migration destination gestalt or location is unknown"))?;
    for gestalt_id in [source_gestalt_id, destination_gestalt_id] {
        let profile = campaign
            .agency_profiles
            .get(gestalt_id)
            .filter(|profile| {
                profile.subject_kind == AgencySubjectKind::Gestalt
                    && profile.active_leaf
                    && profile.simulation_eligible
            })
            .ok_or_else(|| anyhow!("member migration endpoint is not an active gestalt leaf"))?;
        if !profile
            .location_ids
            .contains(if gestalt_id == source_gestalt_id {
                &source.home_location_id
            } else {
                &destination.home_location_id
            })
        {
            return Err(anyhow!(
                "member migration endpoint has incoherent location state"
            ));
        }
    }
    let relation_exists = campaign.agency_relations.values().any(|relation| {
        relation.active
            && relation.kind == AgencyRelationKind::Migration
            && relation.from_subject_id == source_gestalt_id
            && relation.to_subject_id == destination_gestalt_id
    });
    if !relation_exists {
        return Err(anyhow!(
            "member migration lacks an explicit source-to-destination migration relation"
        ));
    }
    let origin = member
        .last_location_id
        .as_deref()
        .unwrap_or(&source.home_location_id);
    let reachable = origin == destination_location_id
        || campaign.locations.get(origin).is_some_and(|location| {
            location.routes.values().any(|route| {
                route.destination_id == destination_location_id
                    && route.travel_minutes <= campaign.tick_hours.saturating_mul(60)
            })
        });
    if !reachable {
        return Err(anyhow!("member migration exceeds the strategic horizon"));
    }
    Ok(())
}

pub fn member_state_references(campaign: &Campaign, member_id: &str) -> Result<BTreeSet<String>> {
    let member = campaign
        .gestalt_members
        .get(member_id)
        .ok_or_else(|| anyhow!("gestalt member is unknown"))?;
    let gestalt = campaign
        .gestalts
        .get(&member.gestalt_id)
        .ok_or_else(|| anyhow!("gestalt member baseline is unknown"))?;
    let mut references = BTreeSet::from([
        format!("member:{member_id}"),
        format!("gestalt:{}", member.gestalt_id),
        format!(
            "location:{}",
            member
                .last_location_id
                .as_deref()
                .unwrap_or(&gestalt.home_location_id)
        ),
    ]);
    references.extend(
        effective_member_capabilities(campaign, member_id)?
            .into_iter()
            .map(|value| format!("capability:{value}")),
    );
    references.extend(
        effective_member_knowledge(campaign, member_id)?
            .into_iter()
            .map(|value| format!("knowledge:{value}")),
    );
    references.extend(
        member
            .equipment
            .iter()
            .map(|value| format!("resource:{value}")),
    );
    for relation in campaign.agency_relations.values().filter(|relation| {
        relation.active
            && relation.kind == AgencyRelationKind::Migration
            && relation.from_subject_id == member.gestalt_id
    }) {
        if let Some(destination) = campaign.gestalts.get(&relation.to_subject_id) {
            references.insert(format!("gestalt:{}", destination.id));
            references.insert(format!("location:{}", destination.home_location_id));
        }
    }
    Ok(references)
}

pub fn effective_member_capabilities(
    campaign: &Campaign,
    member_id: &str,
) -> Result<BTreeSet<String>> {
    let member = campaign
        .gestalt_members
        .get(member_id)
        .ok_or_else(|| anyhow!("gestalt member is unknown"))?;
    let gestalt = campaign
        .gestalts
        .get(&member.gestalt_id)
        .ok_or_else(|| anyhow!("gestalt member baseline is unknown"))?;
    Ok(gestalt
        .shared_capabilities
        .union(&member.capability_additions)
        .filter(|value| !member.capability_removals.contains(*value))
        .cloned()
        .collect())
}

pub fn effective_member_knowledge(
    campaign: &Campaign,
    member_id: &str,
) -> Result<BTreeSet<String>> {
    let member = campaign
        .gestalt_members
        .get(member_id)
        .ok_or_else(|| anyhow!("gestalt member is unknown"))?;
    let gestalt = campaign
        .gestalts
        .get(&member.gestalt_id)
        .ok_or_else(|| anyhow!("gestalt member baseline is unknown"))?;
    Ok(gestalt
        .shared_knowledge
        .union(&member.knowledge_additions)
        .filter(|value| !member.knowledge_removals.contains(*value))
        .cloned()
        .collect())
}

pub fn subject_state_references(campaign: &Campaign, subject_id: &str) -> Result<BTreeSet<String>> {
    let profile = campaign
        .agency_profiles
        .get(subject_id)
        .ok_or_else(|| anyhow!("agency subject is unknown"))?;
    let mut references = BTreeSet::from([format!("subject:{subject_id}")]);
    references.extend(
        profile
            .location_ids
            .iter()
            .map(|value| format!("location:{value}")),
    );
    references.extend(profile.facets.iter().flat_map(|(axis, values)| {
        values
            .iter()
            .map(move |value| format!("facet:{axis:?}:{value}"))
    }));
    if let Some(actor) = campaign.actors.get(subject_id) {
        references.extend(
            actor
                .capabilities
                .iter()
                .map(|value| format!("capability:{value}"))
                .chain(
                    actor
                        .knowledge
                        .iter()
                        .map(|value| format!("knowledge:{value}")),
                )
                .chain(
                    actor
                        .equipment
                        .iter()
                        .map(|value| format!("equipment:{value}")),
                )
                .chain(
                    actor
                        .obligations
                        .iter()
                        .map(|value| format!("obligation:{value}")),
                ),
        );
    }
    if let Some(institution) = campaign.institutions.get(subject_id) {
        references.extend(
            institution
                .resources
                .iter()
                .map(|value| format!("resource:{value}"))
                .chain(
                    institution
                        .goals
                        .iter()
                        .map(|value| format!("goal:{value}")),
                ),
        );
    }
    if let Some(gestalt) = campaign.gestalts.get(subject_id) {
        references.extend(
            gestalt
                .shared_capabilities
                .iter()
                .map(|value| format!("capability:{value}"))
                .chain(
                    gestalt
                        .shared_knowledge
                        .iter()
                        .map(|value| format!("knowledge:{value}")),
                )
                .chain(
                    gestalt
                        .resources
                        .iter()
                        .map(|value| format!("resource:{value}")),
                ),
        );
    }
    Ok(references)
}

fn proposal_target_key(effect: &StrategicCellEffect) -> String {
    match effect {
        StrategicCellEffect::Institution { institution_id, .. } => {
            format!("institution:{institution_id}")
        }
        StrategicCellEffect::Gestalt { gestalt_id, .. } => format!("gestalt:{gestalt_id}"),
        StrategicCellEffect::ActorMove { actor_id, .. } => format!("actor:{actor_id}"),
        StrategicCellEffect::MemberMigration { member_id, .. } => {
            format!("member:{member_id}")
        }
    }
}

fn is_sha256(value: &str) -> bool {
    value
        .strip_prefix("sha256:")
        .is_some_and(|hex| hex.len() == 64 && hex.bytes().all(|byte| byte.is_ascii_hexdigit()))
}

pub fn apply_fission(campaign: &mut Campaign, preview: &GestaltFissionPreview) -> Result<()> {
    validate_fission(campaign, preview)?;
    let parent = campaign
        .gestalts
        .get_mut(&preview.parent_gestalt_id)
        .ok_or_else(|| anyhow!("fission parent vanished"))?;
    parent.version = parent.version.saturating_add(1);
    let parent_profile = campaign
        .agency_profiles
        .get_mut(&preview.parent_gestalt_id)
        .ok_or_else(|| anyhow!("fission parent lacks an agency profile"))?;
    parent_profile.active_leaf = false;
    parent_profile.simulation_eligible = false;
    parent_profile.profile_version = parent_profile.profile_version.saturating_add(1);
    let inherited = parent_profile.clone();
    for child in &preview.children {
        campaign.gestalts.insert(child.id.clone(), child.clone());
        let mut profile = profile_for_gestalt(child, &preview.evidence_receipt_ids);
        profile.parent_subject_id = Some(preview.parent_gestalt_id.clone());
        profile.facets = inherited.facets.clone();
        profile
            .facets
            .entry(preview.partition_axis.clone())
            .or_default()
            .insert(
                preview
                    .child_partition_values
                    .get(&child.id)
                    .expect("validated child partition value")
                    .clone(),
            );
        for receipt_id in &inherited.evidence_receipt_ids {
            if !profile.evidence_receipt_ids.contains(receipt_id) {
                profile.evidence_receipt_ids.push(receipt_id.clone());
            }
        }
        campaign.agency_profiles.insert(child.id.clone(), profile);
    }
    for member in campaign
        .gestalt_members
        .values_mut()
        .filter(|member| member.gestalt_id == preview.parent_gestalt_id)
    {
        member.gestalt_id = preview
            .member_child_assignments
            .get(&member.id)
            .cloned()
            .unwrap_or_else(|| preview.residual_child_id.clone());
        member.version = member.version.saturating_add(1);
    }
    let inherited_relations: Vec<_> = campaign
        .agency_relations
        .values()
        .filter(|relation| {
            relation.active
                && (relation.from_subject_id == preview.parent_gestalt_id
                    || relation.to_subject_id == preview.parent_gestalt_id)
        })
        .cloned()
        .collect();
    for relation in inherited_relations {
        for child in &preview.children {
            let mut inherited_relation = relation.clone();
            inherited_relation.id = format!("{}:fission:{}", relation.id, child.id);
            if inherited_relation.from_subject_id == preview.parent_gestalt_id {
                inherited_relation.from_subject_id = child.id.clone();
            }
            if inherited_relation.to_subject_id == preview.parent_gestalt_id {
                inherited_relation.to_subject_id = child.id.clone();
            }
            campaign
                .agency_relations
                .insert(inherited_relation.id.clone(), inherited_relation);
        }
    }
    campaign.gestalt_lineages.insert(
        preview.parent_gestalt_id.clone(),
        GestaltLineage {
            schema: "ghostlight.gestalt_lineage.v1".into(),
            parent_gestalt_id: preview.parent_gestalt_id.clone(),
            child_gestalt_ids: preview
                .children
                .iter()
                .map(|child| child.id.clone())
                .collect(),
            partition_axis: preview.partition_axis.clone(),
            partition_values: preview.child_partition_values.clone(),
            residual_child_id: preview.residual_child_id.clone(),
            source_revision: campaign.revision,
        },
    );
    campaign.resolution_cover = None;
    campaign.resolution_policy.resolution_epoch = campaign
        .resolution_policy
        .resolution_epoch
        .saturating_add(1);
    Ok(())
}

pub fn validate_fission(campaign: &Campaign, preview: &GestaltFissionPreview) -> Result<()> {
    let parent = campaign
        .gestalts
        .get(&preview.parent_gestalt_id)
        .ok_or_else(|| anyhow!("fission parent is unknown"))?;
    let parent_profile = campaign
        .agency_profiles
        .get(&preview.parent_gestalt_id)
        .filter(|profile| profile.active_leaf)
        .ok_or_else(|| anyhow!("fission parent is not an active leaf gestalt"))?;
    let child_ids: BTreeSet<_> = preview
        .children
        .iter()
        .map(|child| child.id.as_str())
        .collect();
    let residual_value = preview
        .child_partition_values
        .get(&preview.residual_child_id)
        .map(|value| value.trim().to_ascii_lowercase());
    if preview.campaign_id != campaign.id
        || preview.expected_world_revision != campaign.revision
        || !preview.requires_approval
        || preview.children.len() < 2
        || child_ids.len() != preview.children.len()
        || !child_ids.contains(preview.residual_child_id.as_str())
        || residual_value.as_deref() != Some("other/unknown")
        || preview.child_partition_values.len() != preview.children.len()
        || preview.children.iter().any(|child| {
            child.id.trim().is_empty()
                || child.name.trim().is_empty()
                || child.version != 0
                || campaign.gestalts.contains_key(&child.id)
                || !campaign.locations.contains_key(&child.home_location_id)
                || (preview.partition_axis != AgencyAxis::Geography
                    && child.home_location_id != parent.home_location_id)
                || !preview.child_partition_values.contains_key(&child.id)
        })
        || preview
            .member_child_assignments
            .iter()
            .any(|(member, child)| {
                campaign
                    .gestalt_members
                    .get(member)
                    .is_none_or(|value| value.gestalt_id != parent.id)
                    || !child_ids.contains(child.as_str())
            })
        || parent_profile.subject_kind != AgencySubjectKind::Gestalt
    {
        return Err(anyhow!("gestalt fission preview is stale or malformed"));
    }
    Ok(())
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use chrono::Utc;
    use std::time::Instant;
    use uuid::Uuid;

    pub(crate) fn campaign(subjects: usize, budget: u8) -> Campaign {
        let player_id = "player".to_owned();
        let actors = BTreeMap::from([(
            player_id.clone(),
            ActorState {
                id: player_id.clone(),
                name: "Player".into(),
                location_id: "center".into(),
                capabilities: BTreeSet::new(),
                knowledge: BTreeSet::new(),
                equipment: BTreeSet::new(),
                conditions: BTreeSet::new(),
                obligations: BTreeSet::new(),
                relationships: BTreeMap::new(),
                goals: vec![],
                memories: vec![],
            },
        )]);
        let institutions = (0..subjects)
            .map(|index| {
                let id = format!("faction-{index:04}");
                (
                    id.clone(),
                    InstitutionState {
                        id,
                        name: format!("Faction {index}"),
                        resources: vec![format!("resource-{}", index % 7)],
                        goals: vec![format!("goal-{}", index % 5)],
                        posture: format!("posture-{}", index % 3),
                    },
                )
            })
            .collect();
        let mut value = Campaign {
            schema: "ghostlight.campaign.v1".into(),
            id: Uuid::new_v4(),
            name: "Resolution fixture".into(),
            revision: 0,
            branch_origin: BranchOrigin {
                canon_cutoff: "fixture".into(),
                evidence_receipt_ids: vec![],
            },
            world_time: Utc::now(),
            tick_hours: 6,
            player_actor_id: player_id,
            locations: BTreeMap::from([(
                "center".into(),
                Location {
                    id: "center".into(),
                    name: "Center".into(),
                    container_id: None,
                    routes: BTreeMap::new(),
                    persistent_features: vec![],
                },
            )]),
            actors,
            institutions,
            clocks: BTreeMap::new(),
            facts: BTreeMap::new(),
            transcript: vec![],
            last_player_activity: Utc::now(),
            pending_ticks: 0,
            away_ticks_processed: 0,
            events: vec![],
            news: vec![],
            canon_candidates: BTreeMap::new(),
            gestalts: BTreeMap::new(),
            gestalt_members: BTreeMap::new(),
            pending_world_proposals: vec![],
            agency_profiles: BTreeMap::new(),
            agency_relations: BTreeMap::new(),
            gestalt_lineages: BTreeMap::new(),
            resolution_policy: ResolutionPolicy {
                active_cell_budget: budget,
                ..ResolutionPolicy::default()
            },
            resolution_pins: BTreeMap::new(),
            resolution_cover: None,
            strategic_tick_count: 0,
        };
        ensure_agency_profiles(&mut value);
        for (index, profile) in value
            .agency_profiles
            .values_mut()
            .filter(|profile| profile.simulation_eligible)
            .enumerate()
        {
            profile.facets.insert(
                AgencyAxis::Geography,
                BTreeSet::from([format!("region-{}", index % 4)]),
            );
            profile.facets.insert(
                AgencyAxis::Ideology,
                BTreeSet::from([format!("ideology-{}", index % 6)]),
            );
            profile.facets.insert(
                AgencyAxis::SpeciesBody,
                BTreeSet::from([format!("body-{}", index % 2)]),
            );
        }
        value
    }

    #[test]
    fn aetheria_scale_cover_is_complete_unique_and_budgeted() {
        for budget in [1, 4, 8, 32] {
            let value = campaign(24, budget);
            let cover = plan_cover(&value, default_demand(&value, "test pressure")).unwrap();
            assert_eq!(cover.cells.len(), usize::from(budget).min(24));
            let covered: Vec<_> = cover
                .cells
                .iter()
                .flat_map(|cell| cell.subject_ids.iter())
                .collect();
            assert_eq!(covered.len(), 24);
            assert_eq!(covered.iter().copied().collect::<BTreeSet<_>>().len(), 24);
            validate_cover(&value, &cover.demand, &cover.cells).unwrap();
        }
    }

    #[test]
    fn results_and_tie_breaking_are_deterministic() {
        let value = campaign(24, 8);
        let demand = default_demand(&value, "same pressure");
        assert_eq!(
            plan_cover(&value, demand.clone()).unwrap(),
            plan_cover(&value, demand).unwrap()
        );
    }

    #[test]
    fn rivals_at_budget_one_form_an_arena_not_a_false_collective() {
        let mut value = campaign(2, 1);
        value.agency_relations.insert(
            "rivalry".into(),
            AgencyRelation {
                schema: "ghostlight.agency_relation.v1".into(),
                id: "rivalry".into(),
                from_subject_id: "faction-0000".into(),
                to_subject_id: "faction-0001".into(),
                kind: AgencyRelationKind::Rivalry,
                strength: 100,
                active: true,
                evidence_receipt_ids: vec![],
            },
        );
        let cover = plan_cover(&value, default_demand(&value, "war")).unwrap();
        assert_eq!(cover.cells.len(), 1);
        assert_eq!(cover.cells[0].mode, SimulationCellMode::Arena);
    }

    #[test]
    fn foreground_subjects_create_reported_temporary_overage() {
        let mut value = campaign(5, 1);
        for id in ["faction-0000", "faction-0001", "faction-0002"] {
            value.resolution_pins.insert(
                format!("foreground:{id}"),
                ResolutionPin {
                    schema: "ghostlight.resolution_pin.v1".into(),
                    id: format!("foreground:{id}"),
                    kind: ResolutionPinKind::MinimumIndividualDetail,
                    subject_ids: BTreeSet::from([id.into()]),
                    reason: "directly engaged foreground subject".into(),
                    created_world_revision: value.revision,
                },
            );
        }
        let demand = default_demand(&value, "foreground");
        let cover = plan_cover(&value, demand).unwrap();
        assert_eq!(cover.effective_budget, 4);
        assert_eq!(cover.mandatory_overage, 3);
        for id in ["faction-0000", "faction-0001", "faction-0002"] {
            assert!(
                cover
                    .cells
                    .iter()
                    .any(|cell| { cell.subject_ids == BTreeSet::from([id.to_owned()]) })
            );
        }
    }

    #[test]
    fn model_focal_subjects_raise_salience_without_overriding_the_budget() {
        let value = campaign(5, 1);
        let mut demand = default_demand(&value, "broad pressure");
        demand.focal_subject_ids = value
            .agency_profiles
            .values()
            .filter(|profile| profile.simulation_eligible)
            .map(|profile| profile.subject_id.clone())
            .collect();
        let cover = plan_cover(&value, demand).unwrap();
        assert_eq!(cover.effective_budget, 1);
        assert_eq!(cover.mandatory_overage, 0);
        assert_eq!(cover.cells[0].subject_ids.len(), 5);
    }

    #[test]
    fn contradictory_pins_are_rejected() {
        let mut value = campaign(3, 2);
        value.resolution_pins.insert(
            "together".into(),
            ResolutionPin {
                schema: "ghostlight.resolution_pin.v1".into(),
                id: "together".into(),
                kind: ResolutionPinKind::KeepTogether,
                subject_ids: BTreeSet::from(["faction-0000".into(), "faction-0001".into()]),
                reason: "test".into(),
                created_world_revision: 0,
            },
        );
        value.resolution_pins.insert(
            "apart".into(),
            ResolutionPin {
                schema: "ghostlight.resolution_pin.v1".into(),
                id: "apart".into(),
                kind: ResolutionPinKind::KeepSeparate,
                subject_ids: BTreeSet::from(["faction-0000".into(), "faction-0001".into()]),
                reason: "test".into(),
                created_world_revision: 0,
            },
        );
        assert!(plan_cover(&value, default_demand(&value, "pins")).is_err());
    }

    #[test]
    fn detail_debt_rotates_low_budget_attention() {
        let mut value = campaign(6, 1);
        let mut focused = BTreeSet::new();
        for _ in 0..6 {
            let cover = plan_cover(&value, default_demand(&value, "quiet world")).unwrap();
            focused.insert(
                cover.cells[0]
                    .detail_focus_subject_id
                    .clone()
                    .expect("aggregate must have a detail focus"),
            );
            advance_detail_debt(&mut value, &cover);
            value.resolution_cover = Some(cover);
            value.strategic_tick_count += 1;
            value.revision += 1;
        }
        assert_eq!(focused.len(), 6);
    }

    #[test]
    fn planning_never_mutates_canonical_state() {
        let value = campaign(24, 4);
        let before = value.clone();
        let _ = plan_cover(&value, default_demand(&value, "read only")).unwrap();
        assert_eq!(value, before);
    }

    #[test]
    fn partitions_one_thousand_subjects_with_bounded_local_work() {
        let value = campaign(1_000, 8);
        let started = Instant::now();
        let cover = plan_cover(&value, default_demand(&value, "large skeleton")).unwrap();
        assert_eq!(cover.cells.len(), 8);
        if cfg!(debug_assertions) {
            assert!(started.elapsed().as_secs_f32() < 6.0);
        } else {
            assert!(started.elapsed().as_millis() < 100);
        }
    }

    #[test]
    fn arena_cannot_act_as_a_collective_or_borrow_a_constituents_secret() {
        let mut value = campaign(2, 1);
        value.agency_relations.insert(
            "rivalry".into(),
            AgencyRelation {
                schema: "ghostlight.agency_relation.v1".into(),
                id: "rivalry".into(),
                from_subject_id: "faction-0000".into(),
                to_subject_id: "faction-0001".into(),
                kind: AgencyRelationKind::Rivalry,
                strength: 100,
                active: true,
                evidence_receipt_ids: vec![],
            },
        );
        value
            .agency_profiles
            .get_mut("faction-0001")
            .unwrap()
            .information_channels
            .insert("secret-courier".into());
        let cover = plan_cover(&value, default_demand(&value, "espionage")).unwrap();
        let cell = cover.cells[0].clone();
        assert_eq!(cell.mode, SimulationCellMode::Arena);
        let make_wave = |proposal: CellActionProposal| ResolutionWaveCommit {
            schema: "ghostlight.resolution_wave_commit.v1".into(),
            world_revision: value.revision,
            resolution_epoch: value.resolution_policy.resolution_epoch,
            plan_receipt: plan_receipt(&value, &cover),
            appraisals: vec![CellAppraisal {
                schema: "ghostlight.cell_appraisal.v1".into(),
                cell_id: cell.id.clone(),
                world_revision: value.revision,
                resolution_epoch: value.resolution_policy.resolution_epoch,
                considered_subject_ids: cell.subject_ids.clone(),
                actions: vec![proposal],
                inaction_reason: None,
            }],
            cover: cover.clone(),
            model_receipt_hashes: vec![],
        };
        let collective = CellActionProposal {
            subject_id: cell.id.clone(),
            intent: "speak for everyone".into(),
            intended_effect: "declare consensus".into(),
            priority: 1,
            state_references: vec![],
            public_channels: vec![],
            effect: StrategicCellEffect::Institution {
                institution_id: "faction-0000".into(),
                posture: "unified".into(),
                location_ids: vec![],
            },
        };
        assert!(validate_and_resolve_wave(&value, &make_wave(collective)).is_err());
        let borrowed_secret = CellActionProposal {
            subject_id: "faction-0000".into(),
            intent: "publish through a rival's courier".into(),
            intended_effect: "send a message".into(),
            priority: 1,
            state_references: vec![],
            public_channels: vec!["secret-courier".into()],
            effect: StrategicCellEffect::Institution {
                institution_id: "faction-0000".into(),
                posture: "messaging".into(),
                location_ids: vec![],
            },
        };
        assert!(validate_and_resolve_wave(&value, &make_wave(borrowed_secret)).is_err());
    }

    #[test]
    fn rival_arena_preserves_named_member_agency_and_exact_private_state() {
        let mut value = campaign(0, 1);
        let gestalt = |id: &str, name: &str, knowledge: &[&str]| GestaltPersonaState {
            schema: "ghostlight.gestalt_persona_state.v1".into(),
            id: id.into(),
            name: name.into(),
            version: 0,
            home_location_id: "center".into(),
            shared_capabilities: BTreeSet::new(),
            shared_knowledge: knowledge.iter().map(|value| (*value).into()).collect(),
            resources: BTreeSet::new(),
            goals: vec![],
            pressures: vec![],
        };
        value.gestalts.insert(
            "refugees".into(),
            gestalt("refugees", "Transit refugees", &["camp schedule"]),
        );
        value.gestalts.insert(
            "dockers".into(),
            gestalt("dockers", "Dock residents", &["private dock code"]),
        );
        value.gestalt_members.insert(
            "mira".into(),
            GestaltMemberDelta {
                schema: "ghostlight.gestalt_member_delta.v1".into(),
                id: "mira".into(),
                gestalt_id: "refugees".into(),
                version: 1,
                name: "Mira".into(),
                capability_additions: BTreeSet::new(),
                capability_removals: BTreeSet::new(),
                knowledge_additions: BTreeSet::from(["the player helped me".into()]),
                knowledge_removals: BTreeSet::new(),
                equipment: BTreeSet::new(),
                conditions: BTreeSet::new(),
                obligations: BTreeSet::new(),
                relationships: BTreeMap::from([("player".into(), "trusted rescuer".into())]),
                goals: vec!["settle somewhere safe".into()],
                memories: vec!["escaped the fire with the player's help".into()],
                last_location_id: Some("center".into()),
                materialized_actor_id: None,
                last_relevant_revision: 1,
                relevance_lease_until_revision: 0,
            },
        );
        ensure_agency_profiles(&mut value);
        value.agency_relations.insert(
            "rivalry".into(),
            AgencyRelation {
                schema: "ghostlight.agency_relation.v1".into(),
                id: "rivalry".into(),
                from_subject_id: "refugees".into(),
                to_subject_id: "dockers".into(),
                kind: AgencyRelationKind::Rivalry,
                strength: 70,
                active: true,
                evidence_receipt_ids: vec![],
            },
        );
        value.agency_relations.insert(
            "migration".into(),
            AgencyRelation {
                schema: "ghostlight.agency_relation.v1".into(),
                id: "migration".into(),
                from_subject_id: "refugees".into(),
                to_subject_id: "dockers".into(),
                kind: AgencyRelationKind::Migration,
                strength: 90,
                active: true,
                evidence_receipt_ids: vec![],
            },
        );
        let demand = default_demand(&value, "resettlement under hostile pressure");
        let cover = plan_cover(&value, demand).unwrap();
        assert_eq!(cover.cells.len(), 1);
        assert_eq!(cover.cells[0].mode, SimulationCellMode::Arena);
        let proposal = CellActionProposal {
            subject_id: "member:mira".into(),
            intent: "take the offered berth".into(),
            intended_effect:
                "Mira joins the dock residents without speaking for either population.".into(),
            priority: 5,
            state_references: vec![
                "member:mira".into(),
                "knowledge:the player helped me".into(),
            ],
            public_channels: vec![],
            effect: StrategicCellEffect::MemberMigration {
                member_id: "mira".into(),
                source_gestalt_id: "refugees".into(),
                destination_gestalt_id: "dockers".into(),
                destination_location_id: "center".into(),
            },
        };
        let make_wave = |proposal: CellActionProposal| ResolutionWaveCommit {
            schema: "ghostlight.resolution_wave_commit.v1".into(),
            world_revision: value.revision,
            resolution_epoch: value.resolution_policy.resolution_epoch,
            plan_receipt: plan_receipt(&value, &cover),
            appraisals: vec![CellAppraisal {
                schema: "ghostlight.cell_appraisal.v1".into(),
                cell_id: cover.cells[0].id.clone(),
                world_revision: value.revision,
                resolution_epoch: value.resolution_policy.resolution_epoch,
                considered_subject_ids: cover.cells[0].subject_ids.clone(),
                actions: vec![proposal],
                inaction_reason: None,
            }],
            cover: cover.clone(),
            model_receipt_hashes: vec![],
        };
        let plan = validate_and_resolve_wave(&value, &make_wave(proposal.clone())).unwrap();
        assert_eq!(plan.member_migrations.len(), 1);

        let mut collective_theft = proposal.clone();
        collective_theft.subject_id = "refugees".into();
        assert!(validate_and_resolve_wave(&value, &make_wave(collective_theft)).is_err());

        let mut borrowed_secret = proposal;
        borrowed_secret
            .state_references
            .push("knowledge:private dock code".into());
        assert!(validate_and_resolve_wave(&value, &make_wave(borrowed_secret)).is_err());
    }

    #[test]
    fn fission_preserves_parent_and_member_delta_while_activating_child_leaves() {
        let mut value = campaign(1, 1);
        value.gestalts.insert(
            "villagers".into(),
            GestaltPersonaState {
                schema: "ghostlight.gestalt_persona_state.v1".into(),
                id: "villagers".into(),
                name: "Villagers".into(),
                version: 0,
                home_location_id: "center".into(),
                shared_capabilities: BTreeSet::from(["farm".into()]),
                shared_knowledge: BTreeSet::from(["local roads".into()]),
                resources: BTreeSet::from(["granary".into()]),
                goals: vec!["survive winter".into()],
                pressures: vec![],
            },
        );
        value.gestalt_members.insert(
            "john".into(),
            GestaltMemberDelta {
                schema: "ghostlight.gestalt_member_delta.v1".into(),
                id: "john".into(),
                gestalt_id: "villagers".into(),
                version: 3,
                name: "John".into(),
                capability_additions: BTreeSet::from(["smith".into()]),
                capability_removals: BTreeSet::new(),
                knowledge_additions: BTreeSet::new(),
                knowledge_removals: BTreeSet::new(),
                equipment: BTreeSet::from(["hammer".into()]),
                conditions: BTreeSet::new(),
                obligations: BTreeSet::new(),
                relationships: BTreeMap::new(),
                goals: vec![],
                memories: vec!["met the traveler".into()],
                last_location_id: Some("center".into()),
                materialized_actor_id: None,
                last_relevant_revision: 0,
                relevance_lease_until_revision: 0,
            },
        );
        ensure_agency_profiles(&mut value);
        let child = |id: &str, name: &str| GestaltPersonaState {
            schema: "ghostlight.gestalt_persona_state.v1".into(),
            id: id.into(),
            name: name.into(),
            version: 0,
            home_location_id: "center".into(),
            shared_capabilities: BTreeSet::from(["farm".into()]),
            shared_knowledge: BTreeSet::from(["local roads".into()]),
            resources: BTreeSet::from(["granary".into()]),
            goals: vec!["survive winter".into()],
            pressures: vec![],
        };
        let preview = GestaltFissionPreview {
            schema: "ghostlight.gestalt_fission_preview.v1".into(),
            campaign_id: value.id,
            expected_world_revision: value.revision,
            parent_gestalt_id: "villagers".into(),
            partition_axis: AgencyAxis::Ideology,
            children: vec![
                child("traditionalists", "Traditionalists"),
                child("other", "Other villagers"),
            ],
            child_partition_values: BTreeMap::from([
                ("traditionalists".into(), "traditionalist".into()),
                ("other".into(), "other/unknown".into()),
            ]),
            residual_child_id: "other".into(),
            member_child_assignments: BTreeMap::new(),
            evidence_receipt_ids: vec![],
            gaps: vec![],
            canon_candidates: vec![],
            requires_approval: true,
        };
        apply_fission(&mut value, &preview).unwrap();
        assert!(value.gestalts.contains_key("villagers"));
        assert!(!value.agency_profiles["villagers"].active_leaf);
        assert!(value.agency_profiles["traditionalists"].active_leaf);
        assert!(value.agency_profiles["other"].active_leaf);
        assert_eq!(value.gestalt_members["john"].gestalt_id, "other");
        assert_eq!(value.gestalt_members["john"].version, 4);
        assert!(
            value.gestalt_members["john"]
                .memories
                .contains(&"met the traveler".into())
        );
        assert_eq!(
            value.gestalt_lineages["villagers"].residual_child_id,
            "other"
        );
    }

    #[test]
    fn golden_pressures_preserve_the_behavioral_boundary_they_weight() {
        for axis in [
            AgencyAxis::Geography,
            AgencyAxis::Ideology,
            AgencyAxis::EconomyRole,
            AgencyAxis::SpeciesBody,
            AgencyAxis::Information,
        ] {
            let mut value = campaign(4, 2);
            for (index, id) in (0..4).map(|index| (index, format!("faction-{index:04}"))) {
                let profile = value.agency_profiles.get_mut(&id).unwrap();
                for current in [
                    AgencyAxis::Geography,
                    AgencyAxis::Ideology,
                    AgencyAxis::Authority,
                    AgencyAxis::EconomyRole,
                    AgencyAxis::SpeciesBody,
                    AgencyAxis::Information,
                ] {
                    profile
                        .facets
                        .insert(current, BTreeSet::from(["shared".into()]));
                }
                profile.facets.insert(
                    axis.clone(),
                    BTreeSet::from([format!("side-{}", index / 2)]),
                );
            }
            let mut demand = default_demand(&value, format!("golden {axis:?}"));
            for weight in demand.axis_weights.values_mut() {
                *weight = 0.02;
            }
            demand.axis_weights.insert(axis.clone(), 0.90);
            let cover = plan_cover(&value, demand).unwrap();
            assert_eq!(cover.cells.len(), 2);
            for cell in cover.cells {
                let preserved_values: BTreeSet<_> = cell
                    .subject_ids
                    .iter()
                    .flat_map(|id| value.agency_profiles[id].facets[&axis].iter().cloned())
                    .collect();
                assert_eq!(preserved_values.len(), 1, "failed golden pressure {axis:?}");
            }
        }
    }
}
