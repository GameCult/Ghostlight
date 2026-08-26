//! Temporary aggregate-storage lowering for the world transition migration.
//!
//! `Campaign` remains the persisted aggregate until the component-store
//! migration closes. Legacy effect records may enter here as requirements
//! evidence, but this module lowers them into exact `WorldMutation` permits and
//! runs the canonical component reducer before projecting the accepted values
//! back into the aggregate row. It owns no admission policy of its own.

use crate::{
    domain::{
        ActorReaction, AgencyProfile, AgencyRelation, Campaign, FactScope, GestaltFissionPreview,
        GestaltLineage, GestaltPersonaState, OutcomeBand, StrategicActivityOutcome,
        StrategicOutcomeEffect, StrategicTickPlan, WorldEffectDelta, WorldFact,
    },
    transition::*,
};
use anyhow::{Result, anyhow};
use chrono::{DateTime, Utc};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug)]
pub struct LoweredLegacyTransition {
    pub authority: MutationAuthorityEnvelope,
    pub batch: WorldMutationBatch,
}

pub fn lower_foreground_effect(
    campaign: &Campaign,
    acting_actor_id: &str,
    effect: &WorldEffectDelta,
    outcome: OutcomeBand,
    procedure: MutationProcedure,
    effect_ceiling: &str,
    source_receipt_id: &str,
    means_digest: Option<String>,
    intended_effect_digest: Option<String>,
    expires_at: DateTime<Utc>,
) -> Result<LoweredLegacyTransition> {
    if !campaign.actors.contains_key(acting_actor_id) {
        return Err(anyhow!("transition actor is unknown"));
    }
    let mutations = foreground_mutations(campaign, effect, source_receipt_id)?;
    lower_exact_mutations(
        campaign,
        mutations,
        procedure,
        Some(actor_subject(acting_actor_id)),
        MutationOutcomeBinding::Foreground(outcome),
        None,
        effect_ceiling,
        source_receipt_id,
        means_digest,
        intended_effect_digest,
        expires_at,
    )
}

pub fn lower_time_advance(
    campaign: &Campaign,
    minutes: u32,
    procedure: MutationProcedure,
    source_receipt_id: &str,
    expires_at: DateTime<Utc>,
) -> Result<LoweredLegacyTransition> {
    lower_exact_mutations(
        campaign,
        vec![WorldMutation::AdvanceWorldTime {
            campaign: campaign_subject(campaign),
            minutes: i64::from(minutes),
        }],
        procedure,
        None,
        MutationOutcomeBinding::Deterministic,
        None,
        "Advance canonical world time by exactly the admitted duration without puppeting any actor.",
        source_receipt_id,
        Some(digest_serializable(&("wait", minutes))?),
        Some(digest_serializable(&("world_time_minutes", minutes))?),
        expires_at,
    )
}

pub fn lower_group_travel(
    campaign: &Campaign,
    actor_ids: &BTreeSet<String>,
    origin_location_id: &str,
    destination_location_id: &str,
    travel_minutes: u32,
    source_receipt_id: &str,
    expires_at: DateTime<Utc>,
) -> Result<LoweredLegacyTransition> {
    let route_id = exact_route_id(campaign, origin_location_id, destination_location_id)?;
    let route_minutes = campaign
        .locations
        .get(origin_location_id)
        .and_then(|location| {
            location
                .routes
                .values()
                .find(|route| route.destination_id == destination_location_id)
        })
        .map(|route| route.travel_minutes)
        .ok_or_else(|| anyhow!("group travel route vanished"))?;
    if route_minutes != travel_minutes {
        return Err(anyhow!(
            "group travel duration does not match exact topology"
        ));
    }
    let mut mutations = Vec::with_capacity(actor_ids.len() + 1);
    for actor_id in actor_ids {
        let actor = campaign
            .actors
            .get(actor_id)
            .filter(|actor| actor.location_id == origin_location_id)
            .ok_or_else(|| anyhow!("group travel actor is absent from the shared origin"))?;
        mutations.push(WorldMutation::Relocate {
            subject: actor_subject(&actor.id),
            from_place: place_subject(origin_location_id),
            to_place: place_subject(destination_location_id),
            route_id: route_id.clone(),
        });
    }
    mutations.push(WorldMutation::AdvanceWorldTime {
        campaign: campaign_subject(campaign),
        minutes: i64::from(travel_minutes),
    });
    lower_exact_mutations(
        campaign,
        mutations,
        MutationProcedure::Governance,
        None,
        MutationOutcomeBinding::Deterministic,
        None,
        "Relocate every unanimously bound human actor along one exact route and advance time by that route's duration.",
        source_receipt_id,
        Some(digest_serializable(&(
            actor_ids,
            origin_location_id,
            destination_location_id,
            travel_minutes,
        ))?),
        Some(digest_serializable(&(
            "group_occupancy",
            destination_location_id,
        ))?),
        expires_at,
    )
}

pub fn lower_fission(
    campaign: &Campaign,
    preview: &GestaltFissionPreview,
    expires_at: DateTime<Utc>,
) -> Result<LoweredLegacyTransition> {
    crate::resolution::validate_fission(campaign, preview)?;
    let preview_digest = digest_serializable(preview)?;
    let source_receipt_id = format!(
        "fission-preview:{}",
        preview_digest.trim_start_matches("sha256:")
    );
    let parent = population_subject(&preview.parent_gestalt_id);
    let mut mutations = Vec::new();
    for child in &preview.children {
        let subject = population_subject(&child.id);
        mutations.push(WorldMutation::AdmitEntity {
            subject: subject.clone(),
            initial_components: BTreeSet::from([
                WorldComponentKind::Identity,
                WorldComponentKind::Occupancy,
                WorldComponentKind::Capability,
                WorldComponentKind::Knowledge,
                WorldComponentKind::Commitment,
                WorldComponentKind::Pressure,
                WorldComponentKind::PopulationLineage,
            ]),
            initial_place: Some(place_subject(&child.home_location_id)),
            initial_profile: None,
            admission_receipt_id: source_receipt_id.clone(),
        });
        mutations.push(WorldMutation::ChangeIdentity {
            subject,
            operation: IdentityMutationOperation::Adopt,
            handle_id: format!("identity:canonical:{}", child.id),
            handle_value: Some(child.name.clone()),
            audience: Vec::new(),
        });
    }
    mutations.push(WorldMutation::ChangePopulationLineage {
        operation: PopulationLineageOperation::Split,
        parent_populations: vec![parent.clone()],
        child_populations: preview
            .children
            .iter()
            .map(|child| population_subject(&child.id))
            .collect(),
        remainder_population: Some(population_subject(&preview.residual_child_id)),
    });
    for (resource, child_id) in &preview.resource_child_assignments {
        mutations.push(WorldMutation::TransferCustody {
            resource: resource_subject(campaign, &parent, resource),
            from_custodian: parent.clone(),
            to_custodian: population_subject(child_id),
        });
    }
    for member in campaign
        .gestalt_members
        .values()
        .filter(|member| member.gestalt_id == preview.parent_gestalt_id)
    {
        let destination = preview
            .member_child_assignments
            .get(&member.id)
            .unwrap_or(&preview.residual_child_id);
        mutations.push(WorldMutation::ChangePopulationMembership {
            actor: actor_subject(&crate::domain::gestalt_member_subject_id(&member.id)),
            operation: PopulationMembershipOperation::Transfer,
            source_population: Some(parent.clone()),
            destination_population: Some(population_subject(destination)),
        });
    }
    lower_exact_mutations(
        campaign,
        mutations,
        MutationProcedure::CompilerAdmission,
        None,
        MutationOutcomeBinding::Deterministic,
        Some(campaign.resolution_policy.resolution_epoch),
        "Admit the approved child populations, preserve their inherited baseline, partition each exact resource once, transfer each named member once, and record one population lineage split.",
        &source_receipt_id,
        Some(digest_serializable(&(
            "fission",
            &preview.parent_gestalt_id,
            &preview.partition_axis,
        ))?),
        Some(preview_digest),
        expires_at,
    )
}

pub fn apply_lowered_fission(
    campaign: &mut Campaign,
    preview: &GestaltFissionPreview,
    transition: &LoweredLegacyTransition,
    now: DateTime<Utc>,
) -> Result<WorldMutationReceipt> {
    crate::resolution::validate_fission(campaign, preview)?;
    let snapshot = component_snapshot(campaign)?;
    let application =
        apply_component_world_batch(&snapshot, &transition.authority, &transition.batch, now)?;
    project_accepted_fission(campaign, preview, &application.state)?;
    Ok(application.receipt)
}

pub fn lower_region_expansion(
    campaign: &Campaign,
    expansion: &crate::domain::RegionExpansion,
    expires_at: DateTime<Utc>,
) -> Result<LoweredLegacyTransition> {
    crate::compiler::validate_region_expansion(campaign, expansion)?;
    let expansion_digest = digest_serializable(expansion)?;
    let source_receipt_id = format!(
        "region-expansion:{}",
        expansion_digest.trim_start_matches("sha256:")
    );
    let mut mutations = Vec::new();
    for location in &expansion.locations {
        mutations.push(WorldMutation::AdmitEntity {
            subject: place_subject(&location.id),
            initial_components: BTreeSet::from([
                WorldComponentKind::PlaceProfile,
                WorldComponentKind::Topology,
            ]),
            initial_place: None,
            initial_profile: Some(AdmittedEntityProfile::Place {
                name: location.name.clone(),
                container: location.container_id.as_deref().map(place_subject),
                persistent_features: location.persistent_features.iter().cloned().collect(),
            }),
            admission_receipt_id: source_receipt_id.clone(),
        });
    }
    for fact in &expansion.facts {
        mutations.push(WorldMutation::AdmitEntity {
            subject: proposition_subject(&fact.id),
            initial_components: BTreeSet::from([
                WorldComponentKind::Knowledge,
                WorldComponentKind::PropositionContent,
            ]),
            initial_place: None,
            initial_profile: Some(AdmittedEntityProfile::Proposition {
                statement: fact.statement.clone(),
                scope: fact.scope.clone(),
                evidence_receipt_ids: fact.evidence_receipt_ids.iter().cloned().collect(),
                discoverable_at_places: fact
                    .discoverable_at_location_ids
                    .iter()
                    .map(|id| place_subject(id))
                    .collect(),
            }),
            admission_receipt_id: source_receipt_id.clone(),
        });
    }
    for population in &expansion.populations {
        let subject = population_subject(&population.id);
        mutations.push(WorldMutation::AdmitEntity {
            subject: subject.clone(),
            initial_components: BTreeSet::from([
                WorldComponentKind::Identity,
                WorldComponentKind::Occupancy,
                WorldComponentKind::Capability,
                WorldComponentKind::Knowledge,
                WorldComponentKind::Commitment,
                WorldComponentKind::Pressure,
            ]),
            initial_place: Some(place_subject(&population.home_location_id)),
            initial_profile: None,
            admission_receipt_id: source_receipt_id.clone(),
        });
        mutations.push(WorldMutation::ChangeIdentity {
            subject: subject.clone(),
            operation: IdentityMutationOperation::Adopt,
            handle_id: format!("identity:canonical:{}", population.id),
            handle_value: Some(population.name.clone()),
            audience: Vec::new(),
        });
        for capability in &population.shared_capabilities {
            mutations.push(WorldMutation::ChangeCapability {
                subject: subject.clone(),
                operation: CapabilityMutationOperation::Grant,
                capability_id: capability.clone(),
                description: Some(capability.clone()),
            });
        }
        for statement in &population.shared_knowledge {
            let fact = expansion
                .facts
                .iter()
                .chain(campaign.facts.values())
                .find(|fact| fact.statement == *statement)
                .ok_or_else(|| {
                    anyhow!("destination population knowledge lacks an admitted fact")
                })?;
            mutations.push(WorldMutation::ChangeKnowledge {
                operation: KnowledgeMutationOperation::Acquire,
                proposition: proposition_subject(&fact.id),
                knower: Some(subject.clone()),
                speaker: None,
                recipients: Vec::new(),
                channel: None,
            });
        }
        for (index, goal) in population.goals.iter().enumerate() {
            mutations.push(WorldMutation::ChangeCommitment {
                subject: subject.clone(),
                operation: CommitmentMutationOperation::Create,
                kind: CommitmentKind::Goal,
                commitment_id: format!("goal:compiler:{index}:{}", short_digest(goal)),
                counterparty: None,
                description: Some(goal.clone()),
            });
        }
        for pressure in &population.pressures {
            mutations.push(WorldMutation::ChangePressure {
                pressure: gestalt_pressure_subject(&population.id, pressure),
                owner: subject.clone(),
                operation: PressureMutationOperation::Create,
                amount: Some(4),
                label: Some(pressure.clone()),
            });
        }
        for resource in &population.resources {
            mutations.push(WorldMutation::MutateResource {
                resource: resource_subject(campaign, &subject, resource),
                operation: ResourceMutationOperation::Create,
                custodian: Some(subject.clone()),
                related_resources: Vec::new(),
                resource_kind: Some("population_resource".into()),
                resource_label: Some(resource.clone()),
                recipe_id: None,
                quantity: Some(1),
                integrity: Some(100),
            });
        }
    }
    let origin = place_subject(&expansion.origin_location_id);
    for (route_id, route) in &expansion.origin_routes {
        mutations.push(WorldMutation::ChangeTopology {
            operation: TopologyMutationOperation::Add,
            edge_id: component_route_id(&expansion.origin_location_id, route_id),
            from_place: origin.clone(),
            to_place: place_subject(&route.destination_id),
            distance: Some(route.distance.clone()),
            travel_minutes: Some(i64::from(route.travel_minutes)),
        });
    }
    for location in &expansion.locations {
        for (route_id, route) in &location.routes {
            mutations.push(WorldMutation::ChangeTopology {
                operation: TopologyMutationOperation::Add,
                edge_id: component_route_id(&location.id, route_id),
                from_place: place_subject(&location.id),
                to_place: place_subject(&route.destination_id),
                distance: Some(route.distance.clone()),
                travel_minutes: Some(i64::from(route.travel_minutes)),
            });
        }
    }
    for relation in &expansion.migration_relations {
        mutations.push(WorldMutation::ChangeRelationship {
            source: population_subject(&relation.from_subject_id),
            target: population_subject(&relation.to_subject_id),
            operation: RelationshipMutationOperation::Create,
            relationship_id: relation.id.clone(),
            description: Some("migration".into()),
            strength_delta: Some(i64::from(relation.strength)),
        });
    }
    lower_exact_mutations(
        campaign,
        mutations,
        MutationProcedure::CompilerAdmission,
        None,
        MutationOutcomeBinding::Deterministic,
        None,
        "Admit only the approved place profiles, proposition contents, population leaves, agency inputs, and directed migration relations, then add the exact approved route edges without rewriting existing geography or moving any existing subject.",
        &source_receipt_id,
        Some(digest_serializable(&(
            "compile_destination",
            &expansion.origin_location_id,
        ))?),
        Some(expansion_digest),
        expires_at,
    )
}

pub fn apply_lowered_region_expansion(
    campaign: &mut Campaign,
    expansion: &crate::domain::RegionExpansion,
    transition: &LoweredLegacyTransition,
    now: DateTime<Utc>,
) -> Result<WorldMutationReceipt> {
    crate::compiler::validate_region_expansion(campaign, expansion)?;
    let snapshot = component_snapshot(campaign)?;
    let application =
        apply_component_world_batch(&snapshot, &transition.authority, &transition.batch, now)?;
    project_accepted_region_expansion(campaign, expansion, &application.state)?;
    Ok(application.receipt)
}

fn project_accepted_region_expansion(
    campaign: &mut Campaign,
    expansion: &crate::domain::RegionExpansion,
    next: &ComponentWorldState,
) -> Result<()> {
    for requested in &expansion.locations {
        let subject = place_subject(&requested.id);
        let profile = next
            .place_profiles
            .get(&subject)
            .ok_or_else(|| anyhow!("accepted expansion lost a place profile"))?;
        let routes = requested
            .routes
            .keys()
            .map(|route_id| {
                let edge_id = component_route_id(&requested.id, route_id);
                let edge = next
                    .topology
                    .get(&edge_id)
                    .filter(|edge| edge.from_place == subject && edge.open)
                    .ok_or_else(|| anyhow!("accepted expansion lost a local route"))?;
                Ok((
                    route_id.clone(),
                    crate::domain::Route {
                        destination_id: edge.to_place.id.clone(),
                        distance: edge.distance.clone(),
                        travel_minutes: u32::try_from(edge.travel_minutes).map_err(|_| {
                            anyhow!("accepted expansion travel time exceeds aggregate storage")
                        })?,
                    },
                ))
            })
            .collect::<Result<BTreeMap<_, _>>>()?;
        campaign.locations.insert(
            subject.id.clone(),
            crate::domain::Location {
                id: subject.id,
                name: profile.name.clone(),
                container_id: profile
                    .container
                    .as_ref()
                    .map(|container| container.id.clone()),
                routes,
                persistent_features: profile.persistent_features.iter().cloned().collect(),
            },
        );
    }
    let origin = campaign
        .locations
        .get_mut(&expansion.origin_location_id)
        .ok_or_else(|| anyhow!("accepted expansion origin vanished"))?;
    for route_id in expansion.origin_routes.keys() {
        let edge_id = component_route_id(&expansion.origin_location_id, route_id);
        let edge = next
            .topology
            .get(&edge_id)
            .filter(|edge| edge.from_place.id == expansion.origin_location_id && edge.open)
            .ok_or_else(|| anyhow!("accepted expansion lost an origin route"))?;
        origin.routes.insert(
            route_id.clone(),
            crate::domain::Route {
                destination_id: edge.to_place.id.clone(),
                distance: edge.distance.clone(),
                travel_minutes: u32::try_from(edge.travel_minutes)
                    .map_err(|_| anyhow!("accepted origin route exceeds aggregate storage"))?,
            },
        );
    }
    for requested in &expansion.facts {
        let subject = proposition_subject(&requested.id);
        let value = next
            .propositions
            .get(&subject)
            .ok_or_else(|| anyhow!("accepted expansion lost proposition content"))?;
        campaign.facts.insert(
            subject.id.clone(),
            crate::domain::WorldFact {
                id: subject.id,
                statement: value.statement.clone(),
                scope: value.scope.clone(),
                evidence_receipt_ids: value.evidence_receipt_ids.iter().cloned().collect(),
                discoverable_at_location_ids: value
                    .discoverable_at_places
                    .iter()
                    .map(|place| place.id.clone())
                    .collect(),
            },
        );
    }
    for requested in &expansion.populations {
        let subject = population_subject(&requested.id);
        let place = next
            .occupancy
            .get(&subject)
            .filter(|place| place.kind == SubjectKind::Place)
            .ok_or_else(|| anyhow!("accepted destination population lost occupancy"))?;
        let identity = next
            .identities
            .get(&format!("identity:canonical:{}", requested.id))
            .filter(|identity| identity.subject == subject && identity.active)
            .ok_or_else(|| anyhow!("accepted destination population lost identity"))?;
        let mut population = requested.clone();
        population.name = identity.value.clone();
        population.home_location_id = place.id.clone();
        campaign.gestalts.insert(population.id.clone(), population);
    }
    for requested in &expansion.population_profiles {
        let mut profile: AgencyProfile = requested.clone();
        profile.location_ids = BTreeSet::from([campaign
            .gestalts
            .get(&profile.subject_id)
            .ok_or_else(|| anyhow!("accepted destination profile lost population"))?
            .home_location_id
            .clone()]);
        campaign
            .agency_profiles
            .insert(profile.subject_id.clone(), profile);
    }
    for requested in &expansion.migration_relations {
        let value = next
            .relationships
            .get(&requested.id)
            .filter(|value| {
                value.source == population_subject(&requested.from_subject_id)
                    && value.target == population_subject(&requested.to_subject_id)
            })
            .ok_or_else(|| anyhow!("accepted destination migration relation vanished"))?;
        let mut relation: AgencyRelation = requested.clone();
        relation.strength = u8::try_from(
            value
                .strength
                .ok_or_else(|| anyhow!("accepted destination relation lost strength"))?,
        )
        .map_err(|_| anyhow!("accepted destination relation exceeds aggregate storage"))?;
        campaign
            .agency_relations
            .insert(relation.id.clone(), relation);
    }
    if !expansion.populations.is_empty() {
        campaign.resolution_cover = None;
        campaign.resolution_policy.resolution_epoch = campaign
            .resolution_policy
            .resolution_epoch
            .saturating_add(1);
    }
    Ok(())
}

fn project_accepted_fission(
    campaign: &mut Campaign,
    preview: &GestaltFissionPreview,
    next: &ComponentWorldState,
) -> Result<()> {
    let parent = campaign
        .gestalts
        .get(&preview.parent_gestalt_id)
        .cloned()
        .ok_or_else(|| anyhow!("accepted fission parent vanished"))?;
    let inherited_relations = campaign
        .agency_relations
        .values()
        .filter(|relation| {
            relation.active
                && (relation.from_subject_id == preview.parent_gestalt_id
                    || relation.to_subject_id == preview.parent_gestalt_id)
        })
        .cloned()
        .collect::<Vec<_>>();

    for approved in &preview.children {
        let subject = population_subject(&approved.id);
        let place = next
            .occupancy
            .get(&subject)
            .filter(|place| place.kind == SubjectKind::Place)
            .ok_or_else(|| anyhow!("accepted fission child lacks exact occupancy"))?;
        let identity = next
            .identities
            .get(&format!("identity:canonical:{}", approved.id))
            .filter(|identity| identity.subject == subject && identity.active)
            .ok_or_else(|| anyhow!("accepted fission child lacks exact identity"))?;
        campaign.gestalts.insert(
            approved.id.clone(),
            GestaltPersonaState {
                schema: "ghostlight.gestalt_persona_state.v1".into(),
                id: approved.id.clone(),
                name: identity.value.clone(),
                version: 0,
                home_location_id: place.id.clone(),
                shared_capabilities: parent.shared_capabilities.clone(),
                shared_knowledge: parent.shared_knowledge.clone(),
                resources: BTreeSet::new(),
                goals: parent.goals.clone(),
                pressures: parent.pressures.clone(),
            },
        );
    }

    let member_ids = campaign
        .gestalt_members
        .values()
        .filter(|member| member.gestalt_id == preview.parent_gestalt_id)
        .map(|member| member.id.clone())
        .collect::<Vec<_>>();
    for member_id in member_ids {
        let expected_destination = preview
            .member_child_assignments
            .get(&member_id)
            .unwrap_or(&preview.residual_child_id);
        let actor = actor_subject(&crate::domain::gestalt_member_subject_id(&member_id));
        let active_destinations = next
            .memberships
            .iter()
            .filter(|(key, value)| key.actor == actor && value.active)
            .map(|(key, _)| key.population.id.as_str())
            .collect::<Vec<_>>();
        if active_destinations != [expected_destination.as_str()] {
            return Err(anyhow!(
                "accepted fission member does not have one exact destination"
            ));
        }
        project_member_population_transfer(
            campaign,
            &member_id,
            &preview.parent_gestalt_id,
            expected_destination,
        )?;
        let member = campaign
            .gestalt_members
            .get_mut(&member_id)
            .expect("member was projected");
        member.version = member.version.saturating_add(1);
    }

    project_all_resources(campaign, next)?;
    for child in &preview.children {
        if campaign.gestalts[&child.id].resources != child.resources {
            return Err(anyhow!(
                "accepted fission resource custody does not match the approved partition"
            ));
        }
    }
    campaign
        .gestalts
        .get_mut(&preview.parent_gestalt_id)
        .expect("parent existence checked")
        .version = parent.version.saturating_add(1);

    for relation in inherited_relations {
        for child in &preview.children {
            let id = format!("{}:fission:{}", relation.id, child.id);
            let component = next
                .relationships
                .get(&id)
                .ok_or_else(|| anyhow!("accepted fission lost inherited agency relation"))?;
            let mut inherited = relation.clone();
            inherited.id = id.clone();
            if inherited.from_subject_id == preview.parent_gestalt_id {
                inherited.from_subject_id = child.id.clone();
            }
            if inherited.to_subject_id == preview.parent_gestalt_id {
                inherited.to_subject_id = child.id.clone();
            }
            if component.source.id != inherited.from_subject_id
                || component.target.id != inherited.to_subject_id
                || component.strength != Some(i64::from(inherited.strength))
            {
                return Err(anyhow!("accepted fission agency relation was rewritten"));
            }
            campaign.agency_relations.insert(id, inherited);
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
    crate::resolution::project_fission_resolution(campaign, preview)?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn lower_exact_mutations(
    campaign: &Campaign,
    mutations: Vec<WorldMutation>,
    procedure: MutationProcedure,
    source_subject: Option<SubjectRef>,
    outcome: MutationOutcomeBinding,
    resolution_epoch: Option<u64>,
    effect_ceiling: &str,
    source_receipt_id: &str,
    means_digest: Option<String>,
    intended_effect_digest: Option<String>,
    expires_at: DateTime<Utc>,
) -> Result<LoweredLegacyTransition> {
    let mut permits = Vec::with_capacity(mutations.len());
    let mut permitted = Vec::with_capacity(mutations.len());
    for (index, mutation) in mutations.into_iter().enumerate() {
        let permit_id = format!("permit:{source_receipt_id}:{index}");
        permits.push(exact_mutation_permit(permit_id.clone(), &mutation)?);
        permitted.push(PermittedWorldMutation {
            permit_id,
            mutation,
        });
    }
    let mut authority = MutationAuthorityEnvelope {
        schema: "ghostlight.mutation_authority_envelope.v1".into(),
        id: format!(
            "authority:{}:{}:{}",
            campaign.id,
            campaign.revision,
            short_digest(source_receipt_id)
        ),
        campaign_id: campaign.id,
        world_revision: campaign.revision,
        resolution_epoch,
        procedure,
        source_subject,
        outcome,
        effect_ceiling: effect_ceiling.into(),
        permits,
        authority_receipt_ids: BTreeSet::from([source_receipt_id.into()]),
        expires_at,
        digest: String::new(),
    };
    authority.digest = envelope_digest(&authority)?;
    let mut batch = WorldMutationBatch {
        schema: "ghostlight.world_mutation_batch.v1".into(),
        id: format!(
            "batch:{}:{}:{}",
            campaign.id,
            campaign.revision,
            short_digest(source_receipt_id)
        ),
        campaign_id: campaign.id,
        expected_world_revision: campaign.revision,
        expected_resolution_epoch: resolution_epoch,
        authority_envelope_digest: authority.digest.clone(),
        source_receipt_id: source_receipt_id.into(),
        means_digest,
        intended_effect_digest,
        mutations: permitted,
        digest: String::new(),
    };
    batch.digest = mutation_batch_digest(&batch)?;
    Ok(LoweredLegacyTransition { authority, batch })
}

pub fn apply_lowered_transition(
    campaign: &mut Campaign,
    transition: &LoweredLegacyTransition,
    now: DateTime<Utc>,
) -> Result<WorldMutationReceipt> {
    let snapshot = component_snapshot(campaign)?;
    let application =
        apply_component_world_batch(&snapshot, &transition.authority, &transition.batch, now)?;
    project_mutated_components(campaign, &application.state, &transition.batch)?;
    Ok(application.receipt)
}

pub fn lower_reaction_wave(
    campaign: &Campaign,
    witnessed_memory: &str,
    reactions: &[ActorReaction],
    source_receipt_id: &str,
    expires_at: DateTime<Utc>,
) -> Result<LoweredLegacyTransition> {
    let mut mutations = Vec::new();
    for reaction in reactions {
        let actor = campaign
            .actors
            .get(&reaction.actor_id)
            .ok_or_else(|| anyhow!("reaction actor is unknown"))?;
        let subject = actor_subject(&reaction.actor_id);
        if actor.memories.len() < 64
            && !actor.memories.iter().any(|value| value == witnessed_memory)
        {
            mutations.push(WorldMutation::ChangeMemory {
                subject: subject.clone(),
                operation: MemoryMutationOperation::Record,
                memory_id: format!(
                    "memory:reaction:{}:{}",
                    campaign.revision + 1,
                    short_digest(&reaction.actor_id)
                ),
                event_id: Some(format!("reaction-wave:{}", campaign.revision + 1)),
                summary: Some(witnessed_memory.into()),
            });
        }
        if let Some(identity) = reaction.private_delta.identity_adoption.as_deref()
            && identity != actor.name
        {
            let handle_id = format!(
                "identity:adopted:{}:{}",
                reaction.actor_id,
                short_digest(identity)
            );
            let audience = campaign
                .actors
                .values()
                .filter(|observer| {
                    observer.id != reaction.actor_id && observer.location_id == actor.location_id
                })
                .map(|observer| actor_subject(&observer.id))
                .collect::<Vec<_>>();
            mutations.push(WorldMutation::ChangeIdentity {
                subject: subject.clone(),
                operation: IdentityMutationOperation::Adopt,
                handle_id: handle_id.clone(),
                handle_value: Some(identity.into()),
                audience: Vec::new(),
            });
            if !audience.is_empty() {
                mutations.push(WorldMutation::ChangeIdentity {
                    subject: subject.clone(),
                    operation: IdentityMutationOperation::Disclose,
                    handle_id,
                    handle_value: Some(identity.into()),
                    audience,
                });
            }
        }
        for condition in &reaction.private_delta.conditions_add {
            mutations.push(WorldMutation::ChangeCondition {
                subject: subject.clone(),
                operation: if actor.conditions.contains(condition) {
                    ConditionMutationOperation::Alter
                } else {
                    ConditionMutationOperation::Apply
                },
                condition_id: condition.clone(),
                description: Some(condition.clone()),
                severity: None,
            });
        }
        for condition in &reaction.private_delta.conditions_remove {
            mutations.push(WorldMutation::ChangeCondition {
                subject: subject.clone(),
                operation: ConditionMutationOperation::Clear,
                condition_id: condition.clone(),
                description: None,
                severity: None,
            });
        }
        for goal in &reaction.private_delta.goals_add {
            if actor.goals.contains(goal) {
                continue;
            }
            mutations.push(WorldMutation::ChangeCommitment {
                subject: subject.clone(),
                operation: CommitmentMutationOperation::Create,
                kind: CommitmentKind::Goal,
                commitment_id: format!(
                    "goal:reaction:{}:{}",
                    campaign.revision + 1,
                    short_digest(goal)
                ),
                counterparty: None,
                description: Some(goal.clone()),
            });
        }
        for (target_id, description) in &reaction.private_delta.relationship_updates {
            let target = resolve_subject(campaign, target_id)
                .ok_or_else(|| anyhow!("reaction relationship target is unknown"))?;
            mutations.push(WorldMutation::ChangeRelationship {
                source: subject.clone(),
                target,
                operation: if actor.relationships.contains_key(target_id) {
                    RelationshipMutationOperation::Alter
                } else {
                    RelationshipMutationOperation::Create
                },
                relationship_id: relationship_id(&reaction.actor_id, target_id),
                description: Some(description.clone()),
                strength_delta: None,
            });
        }
    }
    lower_exact_mutations(
        campaign,
        mutations,
        MutationProcedure::ReactionAppraisal,
        None,
        MutationOutcomeBinding::Deterministic,
        None,
        "Each exact reacting actor may update only their own public self-identity, private memory, conditions, goals, and directed relationships.",
        source_receipt_id,
        Some(digest_serializable(reactions)?),
        None,
        expires_at,
    )
}

pub fn lower_strategic_wave(
    campaign: &Campaign,
    plan: &StrategicTickPlan,
    source_receipt_id: &str,
    expires_at: DateTime<Utc>,
) -> Result<Option<LoweredLegacyTransition>> {
    let mut mutations = vec![WorldMutation::AdvanceWorldTime {
        campaign: campaign_subject(campaign),
        minutes: i64::from(campaign.tick_hours).saturating_mul(60),
    }];
    for clock in campaign
        .clocks
        .values()
        .filter(|clock| clock.progress < clock.threshold)
    {
        mutations.push(WorldMutation::ChangePressure {
            pressure: pressure_subject(&clock.id),
            owner: campaign_subject(campaign),
            operation: PressureMutationOperation::Advance,
            amount: Some(1),
            label: None,
        });
    }
    for action in &plan.selected_actions {
        let subject = if campaign.actors.contains_key(&action.subject_id) {
            Some(actor_subject(&action.subject_id))
        } else if let Some(member_id) = action.subject_id.strip_prefix("member:")
            && campaign.gestalt_members.contains_key(member_id)
        {
            Some(actor_subject(&action.subject_id))
        } else {
            None
        };
        let Some(subject) = subject else {
            continue;
        };
        let action_digest = crate::resolution::cell_action_digest(action)?;
        let summary = format!(
            "I chose to {}. I intended to {}.",
            action.intent.trim(),
            action.intended_effect.trim()
        );
        if action.intent.trim().is_empty()
            || action.intended_effect.trim().is_empty()
            || summary.chars().count() > 1_000
        {
            return Err(anyhow!(
                "strategic choice cannot be retained as bounded actor memory"
            ));
        }
        mutations.push(WorldMutation::ChangeMemory {
            subject,
            operation: MemoryMutationOperation::Record,
            memory_id: format!(
                "memory:strategic-choice:{}:{}",
                campaign.revision + 1,
                short_digest(&action_digest)
            ),
            event_id: Some(action_digest),
            summary: Some(summary),
        });
    }
    for action in &plan.institution_actions {
        mutations.push(WorldMutation::ChangePosture {
            subject: institution_subject(&action.institution_id),
            posture: action.posture.clone(),
        });
    }
    for action in &plan.gestalt_actions {
        let owner = population_subject(&action.gestalt_id);
        for label in &action.pressure_additions {
            mutations.push(WorldMutation::ChangePressure {
                pressure: gestalt_pressure_subject(&action.gestalt_id, label),
                owner: owner.clone(),
                operation: PressureMutationOperation::Create,
                amount: Some(4),
                label: Some(label.clone()),
            });
        }
        for label in &action.pressure_resolutions {
            mutations.push(WorldMutation::ChangePressure {
                pressure: gestalt_pressure_subject(&action.gestalt_id, label),
                owner: owner.clone(),
                operation: PressureMutationOperation::Resolve,
                amount: None,
                label: None,
            });
        }
    }
    for action in &plan.gestalt_migrations {
        let source = campaign
            .gestalts
            .get(&action.gestalt_id)
            .ok_or_else(|| anyhow!("strategic population source is unknown"))?;
        mutations.push(WorldMutation::Relocate {
            subject: population_subject(&action.gestalt_id),
            from_place: place_subject(&source.home_location_id),
            to_place: place_subject(&action.destination_location_id),
            route_id: exact_route_id(
                campaign,
                &source.home_location_id,
                &action.destination_location_id,
            )?,
        });
    }
    for action in &plan.actor_moves {
        let actor = campaign
            .actors
            .get(&action.actor_id)
            .ok_or_else(|| anyhow!("strategic movement actor is unknown"))?;
        mutations.push(WorldMutation::Relocate {
            subject: actor_subject(&action.actor_id),
            from_place: place_subject(&actor.location_id),
            to_place: place_subject(&action.destination_id),
            route_id: exact_route_id(campaign, &actor.location_id, &action.destination_id)?,
        });
    }
    for action in &plan.member_migrations {
        let member = campaign
            .gestalt_members
            .get(&action.member_id)
            .ok_or_else(|| anyhow!("strategic member is unknown"))?;
        let source = campaign
            .gestalts
            .get(&action.source_gestalt_id)
            .ok_or_else(|| anyhow!("strategic member source population is unknown"))?;
        let origin = member
            .last_location_id
            .as_deref()
            .unwrap_or(&source.home_location_id);
        let subject = actor_subject(&crate::domain::gestalt_member_subject_id(&action.member_id));
        if origin != action.destination_location_id {
            mutations.push(WorldMutation::Relocate {
                subject: subject.clone(),
                from_place: place_subject(origin),
                to_place: place_subject(&action.destination_location_id),
                route_id: exact_route_id(campaign, origin, &action.destination_location_id)?,
            });
        }
        mutations.push(WorldMutation::ChangePopulationMembership {
            actor: subject,
            operation: PopulationMembershipOperation::Transfer,
            source_population: Some(population_subject(&action.source_gestalt_id)),
            destination_population: Some(population_subject(&action.destination_gestalt_id)),
        });
    }

    let mut bindings = Vec::with_capacity(plan.activity_outcomes.len());
    for outcome in &plan.activity_outcomes {
        bindings.push(StrategicOutcomeSourceBinding {
            action_digest: outcome.action_digest.clone(),
            band: outcome.band.clone(),
        });
        push_strategic_outcome_mutations(campaign, outcome, &mut mutations)?;
    }
    if mutations.is_empty() {
        return Ok(None);
    }
    Ok(Some(lower_exact_mutations(
        campaign,
        mutations,
        MutationProcedure::StrategicOutcome,
        None,
        MutationOutcomeBinding::StrategicWave(bindings),
        Some(campaign.resolution_policy.resolution_epoch),
        "The strategic wave may commit only the exact admitted movements, posture changes, population transitions, and constituent-attributed consequences selected by its resolution receipt.",
        source_receipt_id,
        Some(digest_serializable(plan)?),
        None,
        expires_at,
    )?))
}

fn push_strategic_outcome_mutations(
    campaign: &Campaign,
    outcome: &StrategicActivityOutcome,
    mutations: &mut Vec<WorldMutation>,
) -> Result<()> {
    match &outcome.effect {
        StrategicOutcomeEffect::NoMaterialChange { .. } => {}
        StrategicOutcomeEffect::ResourceCreated {
            owner_subject_id,
            resource,
        } => {
            let owner = resolve_subject(campaign, owner_subject_id)
                .ok_or_else(|| anyhow!("strategic resource owner is unknown"))?;
            mutations.push(WorldMutation::MutateResource {
                resource: resource_subject(campaign, &owner, resource),
                operation: ResourceMutationOperation::Create,
                custodian: Some(owner),
                related_resources: Vec::new(),
                resource_kind: Some("legacy_named_resource".into()),
                resource_label: Some(resource.clone()),
                recipe_id: None,
                quantity: Some(1),
                integrity: Some(100),
            });
        }
        StrategicOutcomeEffect::ResourceConsumed {
            owner_subject_id,
            resource,
        } => {
            let owner = resolve_subject(campaign, owner_subject_id)
                .ok_or_else(|| anyhow!("strategic resource owner is unknown"))?;
            mutations.push(WorldMutation::MutateResource {
                resource: resource_subject(campaign, &owner, resource),
                operation: ResourceMutationOperation::Consume,
                custodian: Some(owner),
                related_resources: Vec::new(),
                resource_kind: None,
                resource_label: None,
                recipe_id: None,
                quantity: Some(1),
                integrity: None,
            });
        }
        StrategicOutcomeEffect::ResourceTransferred {
            from_subject_id,
            to_subject_id,
            resource,
        } => {
            let from = resolve_subject(campaign, from_subject_id)
                .ok_or_else(|| anyhow!("strategic resource source is unknown"))?;
            let to = resolve_subject(campaign, to_subject_id)
                .ok_or_else(|| anyhow!("strategic resource recipient is unknown"))?;
            mutations.push(WorldMutation::TransferCustody {
                resource: resource_subject(campaign, &from, resource),
                from_custodian: from,
                to_custodian: to,
            });
        }
        StrategicOutcomeEffect::GestaltPressure {
            gestalt_id,
            pressure_additions,
            pressure_resolutions,
        } => {
            let owner = population_subject(gestalt_id);
            if !campaign.gestalts.contains_key(gestalt_id) {
                return Err(anyhow!("strategic pressure owner is unknown"));
            }
            for label in pressure_additions {
                mutations.push(WorldMutation::ChangePressure {
                    pressure: gestalt_pressure_subject(gestalt_id, label),
                    owner: owner.clone(),
                    operation: PressureMutationOperation::Create,
                    amount: Some(4),
                    label: Some(label.clone()),
                });
            }
            for label in pressure_resolutions {
                mutations.push(WorldMutation::ChangePressure {
                    pressure: gestalt_pressure_subject(gestalt_id, label),
                    owner: owner.clone(),
                    operation: PressureMutationOperation::Resolve,
                    amount: None,
                    label: None,
                });
            }
        }
        StrategicOutcomeEffect::AgencyRelationShift {
            relation_id,
            strength_delta,
        } => {
            let relation = campaign
                .agency_relations
                .get(relation_id)
                .ok_or_else(|| anyhow!("strategic agency relation is unknown"))?;
            let source = resolve_subject(campaign, &relation.from_subject_id)
                .ok_or_else(|| anyhow!("strategic relation source is unknown"))?;
            let target = resolve_subject(campaign, &relation.to_subject_id)
                .ok_or_else(|| anyhow!("strategic relation target is unknown"))?;
            mutations.push(WorldMutation::ChangeRelationship {
                source,
                target,
                operation: RelationshipMutationOperation::Alter,
                relationship_id: relation_id.clone(),
                description: None,
                strength_delta: Some(i64::from(*strength_delta)),
            });
        }
        StrategicOutcomeEffect::MemberMemory { member_id, memory } => {
            ensure_member(campaign, member_id)?;
            mutations.push(WorldMutation::ChangeMemory {
                subject: actor_subject(&crate::domain::gestalt_member_subject_id(member_id)),
                operation: MemoryMutationOperation::Record,
                memory_id: format!(
                    "memory:strategic:{}:{}",
                    campaign.revision + 1,
                    short_digest(memory)
                ),
                event_id: Some(outcome.action_digest.clone()),
                summary: Some(memory.clone()),
            });
        }
        StrategicOutcomeEffect::MemberObligation {
            member_id,
            obligation,
        } => {
            ensure_member(campaign, member_id)?;
            mutations.push(WorldMutation::ChangeCommitment {
                subject: actor_subject(&crate::domain::gestalt_member_subject_id(member_id)),
                operation: CommitmentMutationOperation::Create,
                kind: CommitmentKind::Obligation,
                commitment_id: format!(
                    "obligation:strategic:{}:{}",
                    campaign.revision + 1,
                    short_digest(obligation)
                ),
                counterparty: None,
                description: Some(obligation.clone()),
            });
        }
        StrategicOutcomeEffect::MemberRelationship {
            member_id,
            other_subject_id,
            description,
        } => {
            let member = ensure_member(campaign, member_id)?;
            let target = resolve_subject(campaign, other_subject_id)
                .ok_or_else(|| anyhow!("strategic member relationship target is unknown"))?;
            let source = actor_subject(&crate::domain::gestalt_member_subject_id(member_id));
            mutations.push(WorldMutation::ChangeRelationship {
                source: source.clone(),
                target,
                operation: if member.relationships.contains_key(other_subject_id) {
                    RelationshipMutationOperation::Alter
                } else {
                    RelationshipMutationOperation::Create
                },
                relationship_id: relationship_id(&source.id, other_subject_id),
                description: Some(description.clone()),
                strength_delta: None,
            });
        }
        StrategicOutcomeEffect::KnowledgeLearned {
            owner_subject_id,
            fact_id,
        } => {
            let knower = resolve_subject(campaign, owner_subject_id)
                .ok_or_else(|| anyhow!("strategic knowledge owner is unknown"))?;
            if !campaign.facts.contains_key(fact_id) {
                return Err(anyhow!("strategic proposition is unknown"));
            }
            mutations.push(WorldMutation::ChangeKnowledge {
                operation: KnowledgeMutationOperation::Acquire,
                proposition: proposition_subject(fact_id),
                knower: Some(knower),
                speaker: None,
                recipients: Vec::new(),
                channel: None,
            });
        }
        StrategicOutcomeEffect::KnowledgeCommunicated {
            from_subject_id,
            to_subject_ids,
            fact_id,
        } => {
            let speaker = resolve_subject(campaign, from_subject_id)
                .ok_or_else(|| anyhow!("strategic knowledge speaker is unknown"))?;
            let recipients = to_subject_ids
                .iter()
                .map(|subject_id| {
                    resolve_subject(campaign, subject_id)
                        .ok_or_else(|| anyhow!("strategic knowledge recipient is unknown"))
                })
                .collect::<Result<Vec<_>>>()?;
            if !campaign.facts.contains_key(fact_id) {
                return Err(anyhow!("strategic proposition is unknown"));
            }
            mutations.push(WorldMutation::ChangeKnowledge {
                operation: KnowledgeMutationOperation::Communicate,
                proposition: proposition_subject(fact_id),
                knower: None,
                speaker: Some(speaker),
                recipients,
                channel: None,
            });
        }
    }
    Ok(())
}

fn exact_route_id(campaign: &Campaign, origin: &str, destination: &str) -> Result<String> {
    campaign
        .locations
        .get(origin)
        .and_then(|location| {
            location.routes.iter().find_map(|(route_id, route)| {
                (route.destination_id == destination).then(|| component_route_id(origin, route_id))
            })
        })
        .ok_or_else(|| anyhow!("strategic relocation has no exact topology edge"))
}

fn component_route_id(origin: &str, local_route_id: &str) -> String {
    format!("route:{}:{origin}:{local_route_id}", origin.len())
}

pub fn digest_serializable<T: serde::Serialize + ?Sized>(value: &T) -> Result<String> {
    let bytes = rmp_serde::to_vec_named(value)?;
    Ok(format!("sha256:{:x}", Sha256::digest(bytes)))
}

fn foreground_mutations(
    campaign: &Campaign,
    effect: &WorldEffectDelta,
    source_receipt_id: &str,
) -> Result<Vec<WorldMutation>> {
    let mut mutations = Vec::new();
    for (actor_id, delta) in &effect.actor_conditions {
        let actor = campaign
            .actors
            .get(actor_id)
            .ok_or_else(|| anyhow!("outcome actor vanished"))?;
        for condition in &delta.add {
            mutations.push(WorldMutation::ChangeCondition {
                subject: actor_subject(actor_id),
                operation: if actor.conditions.contains(condition) {
                    ConditionMutationOperation::Alter
                } else {
                    ConditionMutationOperation::Apply
                },
                condition_id: condition.clone(),
                description: Some(condition.clone()),
                severity: None,
            });
        }
        for condition in &delta.remove {
            mutations.push(WorldMutation::ChangeCondition {
                subject: actor_subject(actor_id),
                operation: ConditionMutationOperation::Clear,
                condition_id: condition.clone(),
                description: None,
                severity: None,
            });
        }
    }
    for (actor_id, delta) in &effect.actor_commitments {
        let actor = campaign
            .actors
            .get(actor_id)
            .ok_or_else(|| anyhow!("outcome actor vanished"))?;
        let subject = actor_subject(actor_id);
        for goal in &delta.goals_add {
            if actor.goals.contains(goal) {
                return Err(anyhow!("outcome commitment already exists"));
            }
            mutations.push(WorldMutation::ChangeCommitment {
                subject: subject.clone(),
                operation: CommitmentMutationOperation::Create,
                kind: CommitmentKind::Goal,
                commitment_id: format!(
                    "goal:foreground:{}:{}",
                    campaign.revision,
                    short_digest(goal)
                ),
                counterparty: None,
                description: Some(goal.clone()),
            });
        }
        for goal in &delta.goals_retire {
            let index = actor
                .goals
                .iter()
                .position(|existing| existing == goal)
                .ok_or_else(|| anyhow!("outcome commitment does not exist"))?;
            mutations.push(WorldMutation::ChangeCommitment {
                subject: subject.clone(),
                operation: CommitmentMutationOperation::Retire,
                kind: CommitmentKind::Goal,
                commitment_id: format!("goal:{index}:{}", short_digest(goal)),
                counterparty: None,
                description: None,
            });
        }
        for obligation in &delta.obligations_add {
            if actor.obligations.contains(obligation) {
                return Err(anyhow!("outcome commitment already exists"));
            }
            mutations.push(WorldMutation::ChangeCommitment {
                subject: subject.clone(),
                operation: CommitmentMutationOperation::Create,
                kind: CommitmentKind::Obligation,
                commitment_id: format!(
                    "obligation:foreground:{}:{}",
                    campaign.revision,
                    short_digest(obligation)
                ),
                counterparty: None,
                description: Some(obligation.clone()),
            });
        }
        for obligation in &delta.obligations_retire {
            if !actor.obligations.contains(obligation) {
                return Err(anyhow!("outcome commitment does not exist"));
            }
            mutations.push(WorldMutation::ChangeCommitment {
                subject: subject.clone(),
                operation: CommitmentMutationOperation::Retire,
                kind: CommitmentKind::Obligation,
                commitment_id: format!("obligation:{}", short_digest(obligation)),
                counterparty: None,
                description: None,
            });
        }
    }
    for (actor_id, additions) in &effect.actor_knowledge_additions {
        if !campaign.actors.contains_key(actor_id) {
            return Err(anyhow!("outcome actor vanished"));
        }
        for statement in additions {
            let fact = campaign
                .facts
                .values()
                .find(|fact| fact.statement == *statement)
                .ok_or_else(|| anyhow!("outcome knowledge has no canonical proposition"))?;
            mutations.push(WorldMutation::ChangeKnowledge {
                operation: KnowledgeMutationOperation::Acquire,
                proposition: proposition_subject(&fact.id),
                knower: Some(actor_subject(actor_id)),
                speaker: None,
                recipients: Vec::new(),
                channel: None,
            });
        }
    }
    for (actor_id, observations) in &effect.actor_observations {
        let actor = campaign
            .actors
            .get(actor_id)
            .ok_or_else(|| anyhow!("outcome observation actor vanished"))?;
        for statement in observations {
            if campaign
                .facts
                .values()
                .any(|fact| fact.statement == statement.as_str())
                || actor.knowledge.contains(statement)
            {
                return Err(anyhow!("outcome observation is not a new proposition"));
            }
            let proposition_id = format!(
                "fact:branch-observation:{:x}",
                Sha256::digest(statement.as_bytes())
            );
            if campaign.facts.contains_key(&proposition_id) {
                return Err(anyhow!("outcome observation proposition id collided"));
            }
            let proposition = proposition_subject(&proposition_id);
            mutations.push(WorldMutation::AdmitEntity {
                subject: proposition.clone(),
                initial_components: BTreeSet::from([
                    WorldComponentKind::Knowledge,
                    WorldComponentKind::PropositionContent,
                ]),
                initial_place: None,
                initial_profile: Some(AdmittedEntityProfile::Proposition {
                    statement: statement.clone(),
                    scope: FactScope::BranchLocal,
                    evidence_receipt_ids: BTreeSet::new(),
                    discoverable_at_places: BTreeSet::from([place_subject(&actor.location_id)]),
                }),
                admission_receipt_id: source_receipt_id.into(),
            });
            mutations.push(WorldMutation::ChangeKnowledge {
                operation: KnowledgeMutationOperation::Acquire,
                proposition,
                knower: Some(actor_subject(actor_id)),
                speaker: None,
                recipients: Vec::new(),
                channel: None,
            });
        }
    }
    for (actor_id, relationships) in &effect.actor_relationship_updates {
        let actor = campaign
            .actors
            .get(actor_id)
            .ok_or_else(|| anyhow!("outcome actor vanished"))?;
        for (target_id, description) in relationships {
            let target = resolve_subject(campaign, target_id)
                .ok_or_else(|| anyhow!("relationship target vanished"))?;
            mutations.push(WorldMutation::ChangeRelationship {
                source: actor_subject(actor_id),
                target,
                operation: if actor.relationships.contains_key(target_id) {
                    RelationshipMutationOperation::Alter
                } else {
                    RelationshipMutationOperation::Create
                },
                relationship_id: relationship_id(actor_id, target_id),
                description: Some(description.clone()),
                strength_delta: None,
            });
        }
    }
    for (actor_id, destination_id) in &effect.actor_moves {
        let actor = campaign
            .actors
            .get(actor_id)
            .ok_or_else(|| anyhow!("outcome actor vanished"))?;
        let route_id = exact_route_id(campaign, &actor.location_id, destination_id)
            .map_err(|_| anyhow!("outcome movement has no exact route"))?;
        mutations.push(WorldMutation::Relocate {
            subject: actor_subject(actor_id),
            from_place: place_subject(&actor.location_id),
            to_place: place_subject(destination_id),
            route_id,
        });
    }
    let campaign_subject = campaign_subject(campaign);
    for (clock_id, amount) in &effect.clock_advances {
        mutations.push(WorldMutation::ChangePressure {
            pressure: pressure_subject(clock_id),
            owner: campaign_subject.clone(),
            operation: PressureMutationOperation::Advance,
            amount: Some(i64::from(*amount)),
            label: None,
        });
    }
    for (clock_id, amount) in &effect.clock_reductions {
        mutations.push(WorldMutation::ChangePressure {
            pressure: pressure_subject(clock_id),
            owner: campaign_subject.clone(),
            operation: PressureMutationOperation::Reduce,
            amount: Some(i64::from(*amount)),
            label: None,
        });
    }
    for (institution_id, posture) in &effect.institution_postures {
        if !campaign.institutions.contains_key(institution_id) {
            return Err(anyhow!("outcome institution vanished"));
        }
        mutations.push(WorldMutation::ChangePosture {
            subject: institution_subject(institution_id),
            posture: posture.clone(),
        });
    }
    Ok(mutations)
}

fn component_snapshot(campaign: &Campaign) -> Result<ComponentWorldState> {
    let version = campaign.revision;
    let mut state = ComponentWorldState {
        schema: "ghostlight.component_world_state.v1".into(),
        campaign_id: campaign.id,
        revision: campaign.revision,
        resolution_epoch: campaign.resolution_policy.resolution_epoch,
        world_time: campaign.world_time,
        subjects: BTreeMap::new(),
        occupancy: BTreeMap::new(),
        custody: BTreeMap::new(),
        resources: BTreeMap::new(),
        capabilities: BTreeMap::new(),
        conditions: BTreeMap::new(),
        commitments: BTreeMap::new(),
        relationships: BTreeMap::new(),
        pressures: BTreeMap::new(),
        knowledge: BTreeMap::new(),
        memories: BTreeMap::new(),
        postures: BTreeMap::new(),
        memberships: BTreeMap::new(),
        population_lineages: BTreeMap::new(),
        identities: BTreeMap::new(),
        place_profiles: BTreeMap::new(),
        propositions: BTreeMap::new(),
        topology: BTreeMap::new(),
    };
    admit(
        &mut state,
        campaign_subject(campaign),
        BTreeSet::from([WorldComponentKind::WorldTime]),
        version,
    );
    for location in campaign.locations.values() {
        admit(
            &mut state,
            place_subject(&location.id),
            BTreeSet::from([
                WorldComponentKind::PlaceProfile,
                WorldComponentKind::Topology,
            ]),
            version,
        );
        let place = place_subject(&location.id);
        state.place_profiles.insert(
            place.clone(),
            PlaceComponentState {
                place,
                name: location.name.clone(),
                container: location.container_id.as_deref().map(place_subject),
                persistent_features: location.persistent_features.iter().cloned().collect(),
                version,
            },
        );
    }
    for actor in campaign.actors.values() {
        let subject = actor_subject(&actor.id);
        admit(
            &mut state,
            subject.clone(),
            BTreeSet::from([
                WorldComponentKind::Identity,
                WorldComponentKind::Occupancy,
                WorldComponentKind::Capability,
                WorldComponentKind::Condition,
                WorldComponentKind::Knowledge,
                WorldComponentKind::Memory,
                WorldComponentKind::Relationship,
                WorldComponentKind::Commitment,
            ]),
            version,
        );
        state
            .occupancy
            .insert(subject.clone(), place_subject(&actor.location_id));
        state.identities.insert(
            format!("identity:canonical:{}", actor.id),
            IdentityHandleState {
                schema: "ghostlight.identity_handle.v1".into(),
                id: format!("identity:canonical:{}", actor.id),
                subject: subject.clone(),
                value: actor.name.clone(),
                active: true,
                known_by: BTreeSet::from([subject.clone()]),
                restricted_to: BTreeSet::new(),
                source_revision: version,
            },
        );
        for capability in &actor.capabilities {
            state.capabilities.insert(
                entry_key(&subject, capability),
                CapabilityComponentState {
                    description: capability.clone(),
                    suspended: false,
                    version,
                },
            );
        }
        for condition in &actor.conditions {
            state.conditions.insert(
                entry_key(&subject, condition),
                ConditionComponentState {
                    description: condition.clone(),
                    severity: None,
                    version,
                },
            );
        }
        for (index, goal) in actor.goals.iter().enumerate() {
            state.commitments.insert(
                entry_key(&subject, &format!("goal:{index}:{}", short_digest(goal))),
                CommitmentComponentState {
                    kind: CommitmentKind::Goal,
                    description: goal.clone(),
                    counterparty: None,
                    status: "active".into(),
                    version,
                },
            );
        }
        for obligation in &actor.obligations {
            state.commitments.insert(
                entry_key(
                    &subject,
                    &format!("obligation:{}", short_digest(obligation)),
                ),
                CommitmentComponentState {
                    kind: CommitmentKind::Obligation,
                    description: obligation.clone(),
                    counterparty: None,
                    status: "active".into(),
                    version,
                },
            );
        }
        for (index, memory) in actor.memories.iter().enumerate() {
            state.memories.insert(
                entry_key(
                    &subject,
                    &format!("memory:{index}:{}", short_digest(memory)),
                ),
                MemoryComponentState {
                    event_id: None,
                    summary: memory.clone(),
                    version,
                },
            );
        }
    }
    for institution in campaign.institutions.values() {
        let subject = institution_subject(&institution.id);
        admit(
            &mut state,
            subject.clone(),
            BTreeSet::from([
                WorldComponentKind::Identity,
                WorldComponentKind::Posture,
                WorldComponentKind::Commitment,
            ]),
            version,
        );
        state.postures.insert(subject, institution.posture.clone());
    }
    for gestalt in campaign.gestalts.values() {
        let subject = population_subject(&gestalt.id);
        admit(
            &mut state,
            subject.clone(),
            BTreeSet::from([
                WorldComponentKind::Identity,
                WorldComponentKind::Occupancy,
                WorldComponentKind::Capability,
                WorldComponentKind::Knowledge,
                WorldComponentKind::Pressure,
            ]),
            version,
        );
        state
            .occupancy
            .insert(subject.clone(), place_subject(&gestalt.home_location_id));
        state.identities.insert(
            format!("identity:canonical:{}", gestalt.id),
            IdentityHandleState {
                schema: "ghostlight.identity_handle.v1".into(),
                id: format!("identity:canonical:{}", gestalt.id),
                subject: subject.clone(),
                value: gestalt.name.clone(),
                active: true,
                known_by: BTreeSet::from([subject.clone()]),
                restricted_to: BTreeSet::new(),
                source_revision: version,
            },
        );
        for capability in &gestalt.shared_capabilities {
            state.capabilities.insert(
                entry_key(&subject, capability),
                CapabilityComponentState {
                    description: capability.clone(),
                    suspended: false,
                    version,
                },
            );
        }
        for (index, goal) in gestalt.goals.iter().enumerate() {
            state.commitments.insert(
                entry_key(&subject, &format!("goal:{index}:{}", short_digest(goal))),
                CommitmentComponentState {
                    kind: CommitmentKind::Goal,
                    description: goal.clone(),
                    counterparty: None,
                    status: "active".into(),
                    version,
                },
            );
        }
        for statement in &gestalt.shared_knowledge {
            let proposition = ensure_proposition(&mut state, campaign, statement, version);
            state.knowledge.insert(
                KnowledgeKey {
                    knower: subject.clone(),
                    proposition,
                },
                KnowledgeComponentState {
                    status: "known".into(),
                    source: None,
                    channel: None,
                    concealed_from: BTreeSet::new(),
                    version,
                },
            );
        }
        for pressure_label in &gestalt.pressures {
            let pressure = gestalt_pressure_subject(&gestalt.id, pressure_label);
            admit(
                &mut state,
                pressure.clone(),
                BTreeSet::from([WorldComponentKind::Pressure]),
                version,
            );
            state.pressures.insert(
                pressure.clone(),
                PressureComponentState {
                    pressure,
                    owner: subject.clone(),
                    label: pressure_label.clone(),
                    progress: 0,
                    threshold: 4,
                    resolved: false,
                    version,
                },
            );
        }
    }
    for member in campaign.gestalt_members.values() {
        let subject = actor_subject(&crate::domain::gestalt_member_subject_id(&member.id));
        let gestalt = campaign
            .gestalts
            .get(&member.gestalt_id)
            .ok_or_else(|| anyhow!("gestalt member baseline vanished"))?;
        admit(
            &mut state,
            subject.clone(),
            BTreeSet::from([
                WorldComponentKind::Identity,
                WorldComponentKind::Occupancy,
                WorldComponentKind::Capability,
                WorldComponentKind::Condition,
                WorldComponentKind::Knowledge,
                WorldComponentKind::Memory,
                WorldComponentKind::Relationship,
                WorldComponentKind::Commitment,
                WorldComponentKind::PopulationMembership,
            ]),
            version,
        );
        state.occupancy.insert(
            subject.clone(),
            place_subject(
                member
                    .last_location_id
                    .as_deref()
                    .unwrap_or(&gestalt.home_location_id),
            ),
        );
        let identity_id = format!("identity:canonical:{}", subject.id);
        state
            .identities
            .entry(identity_id.clone())
            .or_insert_with(|| IdentityHandleState {
                schema: "ghostlight.identity_handle.v1".into(),
                id: identity_id,
                subject: subject.clone(),
                value: member.name.clone(),
                active: true,
                known_by: BTreeSet::from([subject.clone()]),
                restricted_to: BTreeSet::new(),
                source_revision: version,
            });
        state.memberships.insert(
            MembershipKey {
                actor: subject.clone(),
                population: population_subject(&member.gestalt_id),
            },
            PopulationMembershipState {
                active: true,
                version,
            },
        );
        for capability in &member.capability_additions {
            state.capabilities.insert(
                entry_key(&subject, capability),
                CapabilityComponentState {
                    description: capability.clone(),
                    suspended: false,
                    version,
                },
            );
        }
        for condition in &member.conditions {
            state.conditions.insert(
                entry_key(&subject, condition),
                ConditionComponentState {
                    description: condition.clone(),
                    severity: None,
                    version,
                },
            );
        }
        for obligation in &member.obligations {
            state.commitments.insert(
                entry_key(
                    &subject,
                    &format!("obligation:{}", short_digest(obligation)),
                ),
                CommitmentComponentState {
                    kind: CommitmentKind::Obligation,
                    description: obligation.clone(),
                    counterparty: None,
                    status: "active".into(),
                    version,
                },
            );
        }
        for (index, goal) in member.goals.iter().enumerate() {
            state.commitments.insert(
                entry_key(&subject, &format!("goal:{index}:{}", short_digest(goal))),
                CommitmentComponentState {
                    kind: CommitmentKind::Goal,
                    description: goal.clone(),
                    counterparty: None,
                    status: "active".into(),
                    version,
                },
            );
        }
        for (index, memory) in member.memories.iter().enumerate() {
            state.memories.insert(
                entry_key(
                    &subject,
                    &format!("memory:{index}:{}", short_digest(memory)),
                ),
                MemoryComponentState {
                    event_id: None,
                    summary: memory.clone(),
                    version,
                },
            );
        }
        for statement in crate::resolution::effective_member_knowledge(campaign, &member.id)? {
            let proposition = ensure_proposition(&mut state, campaign, &statement, version);
            state.knowledge.insert(
                KnowledgeKey {
                    knower: subject.clone(),
                    proposition,
                },
                KnowledgeComponentState {
                    status: "known".into(),
                    source: None,
                    channel: None,
                    concealed_from: BTreeSet::new(),
                    version,
                },
            );
        }
        for (target_id, description) in &member.relationships {
            let Some(target) = resolve_subject(campaign, target_id) else {
                continue;
            };
            let id = relationship_id(&subject.id, target_id);
            state.relationships.insert(
                id,
                RelationshipComponentState {
                    source: subject.clone(),
                    target,
                    description: description.clone(),
                    strength: None,
                    version,
                },
            );
        }
    }
    for relation in campaign
        .agency_relations
        .values()
        .filter(|relation| relation.active)
    {
        let Some(source) = resolve_subject(campaign, &relation.from_subject_id) else {
            continue;
        };
        let Some(target) = resolve_subject(campaign, &relation.to_subject_id) else {
            continue;
        };
        state.relationships.insert(
            relation.id.clone(),
            RelationshipComponentState {
                source,
                target,
                description: format!("{:?}", relation.kind).to_lowercase(),
                strength: Some(i64::from(relation.strength)),
                version,
            },
        );
    }
    for fact in campaign.facts.values() {
        let proposition = proposition_subject(&fact.id);
        admit(
            &mut state,
            proposition.clone(),
            BTreeSet::from([
                WorldComponentKind::Knowledge,
                WorldComponentKind::PropositionContent,
            ]),
            version,
        );
        state.propositions.insert(
            proposition.clone(),
            PropositionComponentState {
                proposition,
                statement: fact.statement.clone(),
                scope: fact.scope.clone(),
                evidence_receipt_ids: fact.evidence_receipt_ids.iter().cloned().collect(),
                discoverable_at_places: fact
                    .discoverable_at_location_ids
                    .iter()
                    .map(|id| place_subject(id))
                    .collect(),
                version,
            },
        );
    }
    for clock in campaign.clocks.values() {
        let pressure = pressure_subject(&clock.id);
        admit(
            &mut state,
            pressure.clone(),
            BTreeSet::from([WorldComponentKind::Pressure]),
            version,
        );
        state.pressures.insert(
            pressure.clone(),
            PressureComponentState {
                pressure,
                owner: campaign_subject(campaign),
                label: clock.label.clone(),
                progress: i64::from(clock.progress),
                threshold: i64::from(clock.threshold),
                resolved: clock.progress >= clock.threshold,
                version,
            },
        );
    }
    for location in campaign.locations.values() {
        for (route_id, route) in &location.routes {
            let edge_id = component_route_id(&location.id, route_id);
            state.topology.insert(
                edge_id.clone(),
                TopologyComponentState {
                    id: edge_id,
                    from_place: place_subject(&location.id),
                    to_place: place_subject(&route.destination_id),
                    distance: route.distance.clone(),
                    travel_minutes: i64::from(route.travel_minutes),
                    open: true,
                    version,
                },
            );
        }
    }
    for actor in campaign.actors.values() {
        let knower = actor_subject(&actor.id);
        for statement in &actor.knowledge {
            let proposition = if let Some(fact) = campaign
                .facts
                .values()
                .find(|fact| fact.statement == *statement)
            {
                proposition_subject(&fact.id)
            } else {
                ensure_proposition(&mut state, campaign, statement, version)
            };
            state.knowledge.insert(
                KnowledgeKey {
                    knower: knower.clone(),
                    proposition,
                },
                KnowledgeComponentState {
                    status: "known".into(),
                    source: None,
                    channel: None,
                    concealed_from: BTreeSet::new(),
                    version,
                },
            );
        }
        for (target_id, description) in &actor.relationships {
            let Some(target) = resolve_subject(campaign, target_id) else {
                continue;
            };
            let id = relationship_id(&actor.id, target_id);
            state.relationships.insert(
                id,
                RelationshipComponentState {
                    source: knower.clone(),
                    target,
                    description: description.clone(),
                    strength: None,
                    version,
                },
            );
        }
    }
    for (owner, label) in legacy_resources(campaign) {
        let resource = resource_subject(campaign, &owner, &label);
        admit(
            &mut state,
            resource.clone(),
            BTreeSet::from([
                WorldComponentKind::ResourceState,
                WorldComponentKind::Custody,
            ]),
            version,
        );
        state.custody.insert(resource.clone(), owner);
        state.resources.insert(
            resource.clone(),
            ResourceComponentState {
                schema: "ghostlight.resource_component.v1".into(),
                resource,
                resource_kind: "legacy_named_resource".into(),
                label,
                quantity: 1,
                integrity: 100,
                qualities: BTreeSet::new(),
                version,
            },
        );
    }
    validate_component_world(&state)?;
    Ok(state)
}

fn project_mutated_components(
    campaign: &mut Campaign,
    next: &ComponentWorldState,
    batch: &WorldMutationBatch,
) -> Result<()> {
    let mut resources_changed = false;
    let mut resource_owners_changed = BTreeSet::new();
    let mut touched_members = BTreeSet::new();
    let mut touched_gestalts = BTreeSet::new();
    for permitted in &batch.mutations {
        match &permitted.mutation {
            WorldMutation::TransferCustody {
                from_custodian,
                to_custodian,
                ..
            } => {
                resources_changed = true;
                resource_owners_changed.insert(from_custodian.clone());
                resource_owners_changed.insert(to_custodian.clone());
            }
            WorldMutation::MutateResource { custodian, .. } => {
                resources_changed = true;
                resource_owners_changed.extend(custodian.iter().cloned());
            }
            WorldMutation::Relocate { subject, .. } if subject.kind == SubjectKind::Actor => {
                let destination = next
                    .occupancy
                    .get(subject)
                    .ok_or_else(|| anyhow!("accepted relocation lost occupancy"))?;
                if let Some(member_id) = subject.id.strip_prefix("member:") {
                    let member = campaign
                        .gestalt_members
                        .get_mut(member_id)
                        .ok_or_else(|| anyhow!("accepted relocation member vanished"))?;
                    member.last_location_id = Some(destination.id.clone());
                    touched_members.insert(member_id.to_string());
                    if let Some(actor) = campaign.actors.get_mut(&subject.id) {
                        actor.location_id = destination.id.clone();
                    }
                } else {
                    campaign
                        .actors
                        .get_mut(&subject.id)
                        .ok_or_else(|| anyhow!("accepted relocation actor vanished"))?
                        .location_id = destination.id.clone();
                }
            }
            WorldMutation::Relocate { subject, .. } if subject.kind == SubjectKind::Population => {
                let destination = next
                    .occupancy
                    .get(subject)
                    .ok_or_else(|| anyhow!("accepted population relocation lost occupancy"))?;
                let gestalt = campaign
                    .gestalts
                    .get_mut(&subject.id)
                    .ok_or_else(|| anyhow!("accepted relocation population vanished"))?;
                gestalt.home_location_id = destination.id.clone();
                touched_gestalts.insert(subject.id.clone());
                let profile = campaign
                    .agency_profiles
                    .get_mut(&subject.id)
                    .ok_or_else(|| anyhow!("accepted relocation agency profile vanished"))?;
                profile.location_ids = BTreeSet::from([destination.id.clone()]);
                profile.profile_version = profile.profile_version.saturating_add(1);
            }
            WorldMutation::ChangeIdentity {
                subject,
                operation: IdentityMutationOperation::Adopt,
                handle_id,
                ..
            } if subject.kind == SubjectKind::Actor => {
                let identity = next
                    .identities
                    .get(handle_id)
                    .filter(|identity| identity.active && identity.subject == *subject)
                    .ok_or_else(|| anyhow!("accepted identity adoption vanished"))?;
                if let Some(member_id) = subject.id.strip_prefix("member:") {
                    campaign
                        .gestalt_members
                        .get_mut(member_id)
                        .ok_or_else(|| anyhow!("accepted identity member vanished"))?
                        .name = identity.value.clone();
                    touched_members.insert(member_id.to_string());
                }
                campaign
                    .actors
                    .get_mut(&subject.id)
                    .ok_or_else(|| anyhow!("accepted identity actor vanished"))?
                    .name = identity.value.clone();
            }
            WorldMutation::ChangeIdentity {
                subject,
                operation: IdentityMutationOperation::Disclose,
                ..
            } if subject.kind == SubjectKind::Actor => {
                // Exact audience knowledge lives in the component world. The
                // aggregate actor/member projection has no rival audience map.
            }
            WorldMutation::ChangeCondition {
                subject,
                condition_id,
                ..
            } if subject.kind == SubjectKind::Actor => {
                let actor = campaign
                    .actors
                    .get_mut(&subject.id)
                    .ok_or_else(|| anyhow!("accepted condition actor vanished"))?;
                let key = entry_key(subject, condition_id);
                if next.conditions.contains_key(&key) {
                    actor.conditions.insert(condition_id.clone());
                } else {
                    actor.conditions.remove(condition_id);
                }
            }
            WorldMutation::AdmitEntity {
                subject,
                initial_profile: Some(AdmittedEntityProfile::Proposition { .. }),
                ..
            } if subject.kind == SubjectKind::Proposition => {
                let proposition = next
                    .propositions
                    .get(subject)
                    .ok_or_else(|| anyhow!("accepted proposition admission vanished"))?;
                if campaign.facts.contains_key(&subject.id) {
                    return Err(anyhow!(
                        "accepted proposition replaced an existing world fact"
                    ));
                }
                campaign.facts.insert(
                    subject.id.clone(),
                    WorldFact {
                        id: subject.id.clone(),
                        statement: proposition.statement.clone(),
                        scope: proposition.scope.clone(),
                        evidence_receipt_ids: proposition
                            .evidence_receipt_ids
                            .iter()
                            .cloned()
                            .collect(),
                        discoverable_at_location_ids: proposition
                            .discoverable_at_places
                            .iter()
                            .map(|place| place.id.clone())
                            .collect(),
                    },
                );
            }
            WorldMutation::ChangeKnowledge {
                proposition,
                knower,
                recipients,
                ..
            } => {
                let statement = campaign
                    .facts
                    .get(&proposition.id)
                    .map(|fact| fact.statement.clone())
                    .ok_or_else(|| anyhow!("accepted proposition lost its world fact"))?;
                for subject in knower.iter().chain(recipients) {
                    if !next.knowledge.contains_key(&KnowledgeKey {
                        knower: subject.clone(),
                        proposition: proposition.clone(),
                    }) {
                        continue;
                    }
                    match subject.kind {
                        SubjectKind::Actor => {
                            if let Some(member_id) = subject.id.strip_prefix("member:") {
                                let member = campaign
                                    .gestalt_members
                                    .get_mut(member_id)
                                    .ok_or_else(|| anyhow!("accepted knowledge member vanished"))?;
                                member.knowledge_removals.remove(&statement);
                                member.knowledge_additions.insert(statement.clone());
                                touched_members.insert(member_id.to_string());
                                if let Some(actor) = campaign.actors.get_mut(&subject.id) {
                                    actor.knowledge.insert(statement.clone());
                                }
                            } else {
                                campaign
                                    .actors
                                    .get_mut(&subject.id)
                                    .ok_or_else(|| anyhow!("accepted knowledge actor vanished"))?
                                    .knowledge
                                    .insert(statement.clone());
                            }
                        }
                        SubjectKind::Population => {
                            let gestalt = campaign
                                .gestalts
                                .get_mut(&subject.id)
                                .ok_or_else(|| anyhow!("accepted knowledge gestalt vanished"))?;
                            gestalt.shared_knowledge.insert(statement.clone());
                            touched_gestalts.insert(subject.id.clone());
                        }
                        _ => {
                            return Err(anyhow!(
                                "aggregate knowledge projection lacks subject kind {:?}",
                                subject.kind
                            ));
                        }
                    }
                }
            }
            WorldMutation::ChangeRelationship {
                relationship_id, ..
            } if campaign.agency_relations.contains_key(relationship_id) => {
                let value = next
                    .relationships
                    .get(relationship_id)
                    .ok_or_else(|| anyhow!("accepted agency relation vanished"))?;
                let strength = value
                    .strength
                    .ok_or_else(|| anyhow!("accepted agency relation lost strength"))?;
                campaign
                    .agency_relations
                    .get_mut(relationship_id)
                    .expect("existence checked")
                    .strength = u8::try_from(strength)
                    .map_err(|_| anyhow!("accepted agency strength exceeded storage"))?;
            }
            WorldMutation::ChangeRelationship {
                source,
                target,
                relationship_id,
                ..
            } if source.kind == SubjectKind::Actor => {
                if let Some(member_id) = source.id.strip_prefix("member:") {
                    let member = campaign
                        .gestalt_members
                        .get_mut(member_id)
                        .ok_or_else(|| anyhow!("accepted relationship member vanished"))?;
                    if let Some(value) = next.relationships.get(relationship_id) {
                        member
                            .relationships
                            .insert(target.id.clone(), value.description.clone());
                        if let Some(actor) = campaign.actors.get_mut(&source.id) {
                            actor
                                .relationships
                                .insert(target.id.clone(), value.description.clone());
                        }
                    } else {
                        member.relationships.remove(&target.id);
                        if let Some(actor) = campaign.actors.get_mut(&source.id) {
                            actor.relationships.remove(&target.id);
                        }
                    }
                    touched_members.insert(member_id.to_string());
                } else {
                    let actor = campaign
                        .actors
                        .get_mut(&source.id)
                        .ok_or_else(|| anyhow!("accepted relationship actor vanished"))?;
                    if let Some(value) = next.relationships.get(relationship_id) {
                        actor
                            .relationships
                            .insert(target.id.clone(), value.description.clone());
                    } else {
                        actor.relationships.remove(&target.id);
                    }
                }
            }
            WorldMutation::ChangeMemory {
                subject, memory_id, ..
            } if subject.kind == SubjectKind::Actor => {
                let key = entry_key(subject, memory_id);
                let value = next
                    .memories
                    .get(&key)
                    .ok_or_else(|| anyhow!("accepted memory vanished"))?;
                if let Some(member_id) = subject.id.strip_prefix("member:") {
                    let member = campaign
                        .gestalt_members
                        .get_mut(member_id)
                        .ok_or_else(|| anyhow!("accepted memory member vanished"))?;
                    if !member.memories.contains(&value.summary) {
                        member.memories.push(value.summary.clone());
                        touched_members.insert(member_id.to_string());
                    }
                    if let Some(actor) = campaign.actors.get_mut(&subject.id) {
                        if actor.memories.len() < 64 && !actor.memories.contains(&value.summary) {
                            actor.memories.push(value.summary.clone());
                        }
                    }
                } else {
                    let actor = campaign
                        .actors
                        .get_mut(&subject.id)
                        .ok_or_else(|| anyhow!("accepted memory actor vanished"))?;
                    if actor.memories.len() < 64 && !actor.memories.contains(&value.summary) {
                        actor.memories.push(value.summary.clone());
                    }
                }
            }
            WorldMutation::ChangeCommitment {
                subject,
                kind,
                commitment_id,
                ..
            } if subject.kind == SubjectKind::Actor => {
                let key = entry_key(subject, commitment_id);
                let value = next.commitments.get(&key);
                if let Some(member_id) = subject.id.strip_prefix("member:") {
                    let member = campaign
                        .gestalt_members
                        .get_mut(member_id)
                        .ok_or_else(|| anyhow!("accepted commitment member vanished"))?;
                    match (kind, value) {
                        (CommitmentKind::Goal, Some(value)) => {
                            if !member.goals.contains(&value.description) {
                                member.goals.push(value.description.clone());
                            }
                        }
                        (CommitmentKind::Obligation, Some(value)) => {
                            member.obligations.insert(value.description.clone());
                        }
                        (CommitmentKind::Goal, None) => member
                            .goals
                            .retain(|goal| !campaign_commitment_matches(goal, commitment_id)),
                        (CommitmentKind::Obligation, None) => {
                            member.obligations.retain(|obligation| {
                                !campaign_commitment_matches(obligation, commitment_id)
                            });
                        }
                    }
                    touched_members.insert(member_id.to_string());
                    if let Some(actor) = campaign.actors.get_mut(&subject.id) {
                        match (kind, value) {
                            (CommitmentKind::Goal, Some(value)) => {
                                if !actor.goals.contains(&value.description) {
                                    actor.goals.push(value.description.clone());
                                }
                            }
                            (CommitmentKind::Obligation, Some(value)) => {
                                actor.obligations.insert(value.description.clone());
                            }
                            (CommitmentKind::Goal, None) => actor
                                .goals
                                .retain(|goal| !campaign_commitment_matches(goal, commitment_id)),
                            (CommitmentKind::Obligation, None) => {
                                actor.obligations.retain(|obligation| {
                                    !campaign_commitment_matches(obligation, commitment_id)
                                });
                            }
                        }
                    }
                } else {
                    let actor = campaign
                        .actors
                        .get_mut(&subject.id)
                        .ok_or_else(|| anyhow!("accepted commitment actor vanished"))?;
                    match (kind, value) {
                        (CommitmentKind::Goal, Some(value)) => {
                            if !actor.goals.contains(&value.description) {
                                actor.goals.push(value.description.clone());
                            }
                        }
                        (CommitmentKind::Obligation, Some(value)) => {
                            actor.obligations.insert(value.description.clone());
                        }
                        (CommitmentKind::Goal, None) => actor
                            .goals
                            .retain(|goal| !campaign_commitment_matches(goal, commitment_id)),
                        (CommitmentKind::Obligation, None) => {
                            actor.obligations.retain(|obligation| {
                                !campaign_commitment_matches(obligation, commitment_id)
                            });
                        }
                    }
                }
            }
            WorldMutation::ChangePressure { pressure, .. }
                if pressure.kind == SubjectKind::Pressure =>
            {
                let value = next
                    .pressures
                    .get(pressure)
                    .ok_or_else(|| anyhow!("accepted pressure vanished"))?;
                if let Some(clock) = campaign.clocks.get_mut(&pressure.id) {
                    clock.progress = u8::try_from(value.progress)
                        .map_err(|_| anyhow!("accepted clock exceeded aggregate storage"))?;
                } else if value.owner.kind == SubjectKind::Population {
                    let gestalt = campaign
                        .gestalts
                        .get_mut(&value.owner.id)
                        .ok_or_else(|| anyhow!("accepted pressure gestalt vanished"))?;
                    gestalt.pressures.retain(|label| label != &value.label);
                    if !value.resolved {
                        gestalt.pressures.push(value.label.clone());
                    }
                    touched_gestalts.insert(value.owner.id.clone());
                } else {
                    return Err(anyhow!("aggregate pressure owner is unsupported"));
                }
            }
            WorldMutation::ChangePosture { subject, .. }
                if subject.kind == SubjectKind::Institution =>
            {
                let posture = next
                    .postures
                    .get(subject)
                    .ok_or_else(|| anyhow!("accepted posture vanished"))?;
                campaign
                    .institutions
                    .get_mut(&subject.id)
                    .ok_or_else(|| anyhow!("accepted institution vanished"))?
                    .posture = posture.clone();
            }
            WorldMutation::ChangePopulationMembership {
                actor,
                operation: PopulationMembershipOperation::Transfer,
                source_population: Some(source),
                destination_population: Some(destination),
            } if actor.kind == SubjectKind::Actor
                && source.kind == SubjectKind::Population
                && destination.kind == SubjectKind::Population =>
            {
                let member_id = actor.id.strip_prefix("member:").ok_or_else(|| {
                    anyhow!("aggregate membership transfer requires a named member")
                })?;
                let source_key = MembershipKey {
                    actor: actor.clone(),
                    population: source.clone(),
                };
                let destination_key = MembershipKey {
                    actor: actor.clone(),
                    population: destination.clone(),
                };
                if next
                    .memberships
                    .get(&source_key)
                    .is_some_and(|membership| membership.active)
                    || !next
                        .memberships
                        .get(&destination_key)
                        .is_some_and(|membership| membership.active)
                {
                    return Err(anyhow!(
                        "accepted population transfer has incoherent final membership"
                    ));
                }
                project_member_population_transfer(
                    campaign,
                    member_id,
                    &source.id,
                    &destination.id,
                )?;
                touched_members.insert(member_id.to_string());
                touched_gestalts.insert(source.id.clone());
                touched_gestalts.insert(destination.id.clone());
            }
            WorldMutation::AdvanceWorldTime { .. } => {
                campaign.world_time = next.world_time;
            }
            _ => {
                return Err(anyhow!(
                    "aggregate projection does not yet support mutation {:?}",
                    permitted.mutation.operation()
                ));
            }
        }
    }
    if resources_changed {
        project_all_resources(campaign, next)?;
        for owner in resource_owners_changed {
            match owner.kind {
                SubjectKind::Actor if owner.id.starts_with("member:") => {
                    touched_members.insert(owner.id.trim_start_matches("member:").to_string());
                }
                SubjectKind::Population => {
                    touched_gestalts.insert(owner.id);
                }
                _ => {}
            }
        }
    }
    for member_id in touched_members {
        if let Some(member) = campaign.gestalt_members.get_mut(&member_id) {
            member.version = member.version.saturating_add(1);
        }
    }
    for gestalt_id in touched_gestalts {
        if let Some(gestalt) = campaign.gestalts.get_mut(&gestalt_id) {
            gestalt.version = gestalt.version.saturating_add(1);
        }
    }
    Ok(())
}

fn campaign_commitment_matches(value: &str, commitment_id: &str) -> bool {
    commitment_id.ends_with(&short_digest(value))
}

fn project_member_population_transfer(
    campaign: &mut Campaign,
    member_id: &str,
    source_population_id: &str,
    destination_population_id: &str,
) -> Result<()> {
    let member = campaign
        .gestalt_members
        .get(member_id)
        .filter(|member| {
            member.materialized_actor_id.is_none() && member.gestalt_id == source_population_id
        })
        .ok_or_else(|| anyhow!("accepted member transfer has stale aggregate source"))?;
    let source = campaign
        .gestalts
        .get(source_population_id)
        .ok_or_else(|| anyhow!("accepted member transfer source vanished"))?;
    let destination = campaign
        .gestalts
        .get(destination_population_id)
        .ok_or_else(|| anyhow!("accepted member transfer destination vanished"))?
        .clone();

    // The member's exact effective state survives a change of aggregate
    // baseline. Only its delta representation is recomputed.
    let effective_capabilities = source
        .shared_capabilities
        .union(&member.capability_additions)
        .filter(|value| !member.capability_removals.contains(*value))
        .cloned()
        .collect::<BTreeSet<_>>();
    let effective_knowledge = source
        .shared_knowledge
        .union(&member.knowledge_additions)
        .filter(|value| !member.knowledge_removals.contains(*value))
        .cloned()
        .collect::<BTreeSet<_>>();
    let effective_goals = if member.goals.is_empty() {
        source.goals.clone()
    } else {
        member.goals.clone()
    };

    let member = campaign
        .gestalt_members
        .get_mut(member_id)
        .expect("member source was resolved");
    member.capability_additions = effective_capabilities
        .difference(&destination.shared_capabilities)
        .cloned()
        .collect();
    member.capability_removals = destination
        .shared_capabilities
        .difference(&effective_capabilities)
        .cloned()
        .collect();
    member.knowledge_additions = effective_knowledge
        .difference(&destination.shared_knowledge)
        .cloned()
        .collect();
    member.knowledge_removals = destination
        .shared_knowledge
        .difference(&effective_knowledge)
        .cloned()
        .collect();
    member.goals = if effective_goals == destination.goals {
        Vec::new()
    } else {
        effective_goals
    };
    member.gestalt_id = destination_population_id.into();
    Ok(())
}

fn project_all_resources(campaign: &mut Campaign, next: &ComponentWorldState) -> Result<()> {
    for actor in campaign.actors.values_mut() {
        actor.equipment.clear();
    }
    for member in campaign.gestalt_members.values_mut() {
        member.equipment.clear();
    }
    for institution in campaign.institutions.values_mut() {
        institution.resources.clear();
    }
    for gestalt in campaign.gestalts.values_mut() {
        gestalt.resources.clear();
    }
    for location in campaign.locations.values_mut() {
        location.persistent_features.clear();
    }
    for (resource, value) in &next.resources {
        let owner = next
            .custody
            .get(resource)
            .ok_or_else(|| anyhow!("accepted resource lost custody"))?;
        match owner.kind {
            SubjectKind::Actor => {
                if let Some(member_id) = owner.id.strip_prefix("member:") {
                    campaign
                        .gestalt_members
                        .get_mut(member_id)
                        .ok_or_else(|| anyhow!("accepted resource member vanished"))?
                        .equipment
                        .insert(value.label.clone());
                    if let Some(actor) = campaign.actors.get_mut(&owner.id) {
                        actor.equipment.insert(value.label.clone());
                    }
                } else {
                    campaign
                        .actors
                        .get_mut(&owner.id)
                        .ok_or_else(|| anyhow!("accepted resource actor vanished"))?
                        .equipment
                        .insert(value.label.clone());
                }
            }
            SubjectKind::Institution => campaign
                .institutions
                .get_mut(&owner.id)
                .ok_or_else(|| anyhow!("accepted resource institution vanished"))?
                .resources
                .push(value.label.clone()),
            SubjectKind::Population => {
                campaign
                    .gestalts
                    .get_mut(&owner.id)
                    .ok_or_else(|| anyhow!("accepted resource gestalt vanished"))?
                    .resources
                    .insert(value.label.clone());
            }
            SubjectKind::Place => campaign
                .locations
                .get_mut(&owner.id)
                .ok_or_else(|| anyhow!("accepted resource location vanished"))?
                .persistent_features
                .push(value.label.clone()),
            _ => return Err(anyhow!("aggregate resource custodian is unsupported")),
        }
    }
    for institution in campaign.institutions.values_mut() {
        institution.resources.sort();
        institution.resources.dedup();
    }
    for location in campaign.locations.values_mut() {
        location.persistent_features.sort();
        location.persistent_features.dedup();
    }
    Ok(())
}

fn admit(
    state: &mut ComponentWorldState,
    subject: SubjectRef,
    admitted_components: BTreeSet<WorldComponentKind>,
    version: u64,
) {
    if let Some(existing) = state.subjects.get_mut(&subject) {
        existing.admitted_components.extend(admitted_components);
        existing.version = existing.version.max(version);
    } else {
        state.subjects.insert(
            subject.clone(),
            TypedSubject {
                schema: "ghostlight.typed_subject.v1".into(),
                subject,
                lifecycle: LifecycleStatus::Active,
                admitted_components,
                version,
            },
        );
    }
}

fn resolve_subject(campaign: &Campaign, id: &str) -> Option<SubjectRef> {
    if campaign.actors.contains_key(id) {
        Some(actor_subject(id))
    } else if id
        .strip_prefix("member:")
        .is_some_and(|member_id| campaign.gestalt_members.contains_key(member_id))
    {
        Some(actor_subject(id))
    } else if campaign.institutions.contains_key(id) {
        Some(institution_subject(id))
    } else if campaign.gestalts.contains_key(id) {
        Some(population_subject(id))
    } else if campaign.locations.contains_key(id) {
        Some(place_subject(id))
    } else if campaign.facts.contains_key(id) {
        Some(proposition_subject(id))
    } else {
        None
    }
}

fn ensure_member<'a>(
    campaign: &'a Campaign,
    member_id: &str,
) -> Result<&'a crate::domain::GestaltMemberDelta> {
    campaign
        .gestalt_members
        .get(member_id)
        .ok_or_else(|| anyhow!("strategic member is unknown"))
}

fn entry_key(subject: &SubjectRef, entry_id: &str) -> SubjectEntryKey {
    SubjectEntryKey {
        subject: subject.clone(),
        entry_id: entry_id.into(),
    }
}

fn ensure_proposition(
    state: &mut ComponentWorldState,
    campaign: &Campaign,
    statement: &str,
    version: u64,
) -> SubjectRef {
    let proposition = campaign
        .facts
        .values()
        .find(|fact| fact.statement == statement)
        .map(|fact| proposition_subject(&fact.id))
        .unwrap_or_else(|| {
            proposition_subject(&format!("legacy-knowledge:{}", short_digest(statement)))
        });
    admit(
        state,
        proposition.clone(),
        BTreeSet::from([
            WorldComponentKind::Knowledge,
            WorldComponentKind::PropositionContent,
        ]),
        version,
    );
    state
        .propositions
        .entry(proposition.clone())
        .or_insert_with(|| PropositionComponentState {
            proposition: proposition.clone(),
            statement: statement.into(),
            scope: crate::domain::FactScope::ProvisionalLocal,
            evidence_receipt_ids: BTreeSet::new(),
            discoverable_at_places: BTreeSet::new(),
            version,
        });
    proposition
}

fn legacy_resources(campaign: &Campaign) -> BTreeSet<(SubjectRef, String)> {
    let mut resources = BTreeSet::new();
    for actor in campaign.actors.values() {
        for resource in &actor.equipment {
            resources.insert((actor_subject(&actor.id), resource.clone()));
        }
    }
    for institution in campaign.institutions.values() {
        for resource in &institution.resources {
            resources.insert((institution_subject(&institution.id), resource.clone()));
        }
    }
    for gestalt in campaign.gestalts.values() {
        for resource in &gestalt.resources {
            resources.insert((population_subject(&gestalt.id), resource.clone()));
        }
    }
    for member in campaign.gestalt_members.values() {
        let owner = actor_subject(&crate::domain::gestalt_member_subject_id(&member.id));
        for resource in &member.equipment {
            resources.insert((owner.clone(), resource.clone()));
        }
    }
    for location in campaign.locations.values() {
        for resource in &location.persistent_features {
            resources.insert((place_subject(&location.id), resource.clone()));
        }
    }
    resources
}

fn relationship_id(source: &str, target: &str) -> String {
    format!(
        "relationship:{}",
        short_digest(&(source.to_owned() + "\0" + target))
    )
}

fn campaign_subject(campaign: &Campaign) -> SubjectRef {
    SubjectRef {
        kind: SubjectKind::Campaign,
        id: campaign.id.to_string(),
    }
}

fn actor_subject(id: &str) -> SubjectRef {
    SubjectRef {
        kind: SubjectKind::Actor,
        id: id.into(),
    }
}

fn population_subject(id: &str) -> SubjectRef {
    SubjectRef {
        kind: SubjectKind::Population,
        id: id.into(),
    }
}

fn institution_subject(id: &str) -> SubjectRef {
    SubjectRef {
        kind: SubjectKind::Institution,
        id: id.into(),
    }
}

fn place_subject(id: &str) -> SubjectRef {
    SubjectRef {
        kind: SubjectKind::Place,
        id: id.into(),
    }
}

fn pressure_subject(id: &str) -> SubjectRef {
    SubjectRef {
        kind: SubjectKind::Pressure,
        id: id.into(),
    }
}

fn gestalt_pressure_subject(gestalt_id: &str, label: &str) -> SubjectRef {
    pressure_subject(&format!(
        "gestalt-pressure:{}:{}",
        gestalt_id,
        short_digest(label)
    ))
}

fn resource_subject(campaign: &Campaign, custodian: &SubjectRef, label: &str) -> SubjectRef {
    SubjectRef {
        kind: SubjectKind::Resource,
        id: format!(
            "resource:{}",
            short_digest(&format!(
                "{}\0{:?}\0{}\0{}",
                campaign.id, custodian.kind, custodian.id, label
            ))
        ),
    }
}

fn proposition_subject(id: &str) -> SubjectRef {
    SubjectRef {
        kind: SubjectKind::Proposition,
        id: id.into(),
    }
}

fn short_digest(value: &str) -> String {
    format!("{:x}", Sha256::digest(value.as_bytes()))
        .chars()
        .take(16)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{
        ActorState, BranchOrigin, CommitmentDelta, ConditionDelta, GestaltMemberDelta,
        InstitutionState, Location, ResolutionPolicy, Route, StrategicCellEffect,
        StrategicOutcomeBand, WorldClock, WorldFact,
    };
    use chrono::Duration;
    use uuid::Uuid;

    fn campaign() -> Campaign {
        let id = Uuid::new_v4();
        Campaign {
            schema: "ghostlight.campaign.v1".into(),
            id,
            name: "Transition fixture".into(),
            revision: 4,
            branch_origin: BranchOrigin {
                canon_cutoff: "fixture".into(),
                evidence_receipt_ids: vec![],
            },
            world_time: Utc::now(),
            tick_hours: 6,
            player_actor_id: "player".into(),
            locations: BTreeMap::from([
                (
                    "room".into(),
                    Location {
                        id: "room".into(),
                        name: "Room".into(),
                        container_id: None,
                        routes: BTreeMap::from([(
                            "door".into(),
                            Route {
                                destination_id: "hall".into(),
                                distance: "nearby".into(),
                                travel_minutes: 2,
                            },
                        )]),
                        persistent_features: vec![],
                    },
                ),
                (
                    "hall".into(),
                    Location {
                        id: "hall".into(),
                        name: "Hall".into(),
                        container_id: None,
                        routes: BTreeMap::new(),
                        persistent_features: vec![],
                    },
                ),
            ]),
            actors: BTreeMap::from([
                (
                    "player".into(),
                    ActorState {
                        id: "player".into(),
                        name: "Player".into(),
                        location_id: "room".into(),
                        capabilities: BTreeSet::new(),
                        knowledge: BTreeSet::new(),
                        equipment: BTreeSet::new(),
                        conditions: BTreeSet::new(),
                        obligations: BTreeSet::new(),
                        relationships: BTreeMap::new(),
                        goals: vec![],
                        memories: vec![],
                    },
                ),
                (
                    "witness".into(),
                    ActorState {
                        id: "witness".into(),
                        name: "Witness".into(),
                        location_id: "room".into(),
                        capabilities: BTreeSet::new(),
                        knowledge: BTreeSet::new(),
                        equipment: BTreeSet::new(),
                        conditions: BTreeSet::new(),
                        obligations: BTreeSet::new(),
                        relationships: BTreeMap::new(),
                        goals: vec![],
                        memories: vec![],
                    },
                ),
            ]),
            institutions: BTreeMap::from([(
                "watch".into(),
                InstitutionState {
                    id: "watch".into(),
                    name: "Watch".into(),
                    resources: vec![],
                    goals: vec![],
                    posture: "guarded".into(),
                },
            )]),
            clocks: BTreeMap::from([(
                "alarm".into(),
                WorldClock {
                    id: "alarm".into(),
                    label: "Alarm".into(),
                    progress: 2,
                    threshold: 6,
                    consequence: "The doors lock.".into(),
                },
            )]),
            facts: BTreeMap::from([(
                "fact:route".into(),
                WorldFact {
                    id: "fact:route".into(),
                    statement: "The west stair bypasses the checkpoint.".into(),
                    scope: crate::domain::FactScope::BranchLocal,
                    evidence_receipt_ids: vec![],
                    discoverable_at_location_ids: BTreeSet::from(["room".into()]),
                },
            )]),
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
            resolution_policy: ResolutionPolicy::default(),
            resolution_pins: BTreeMap::new(),
            resolution_cover: None,
            strategic_tick_count: 0,
        }
    }

    #[test]
    fn component_snapshot_projects_effective_member_knowledge() {
        let mut campaign = campaign();
        let statement = campaign.facts["fact:route"].statement.clone();
        campaign.gestalts.insert(
            "refugees".into(),
            GestaltPersonaState {
                schema: "ghostlight.gestalt_persona_state.v1".into(),
                id: "refugees".into(),
                name: "Refugees".into(),
                version: 0,
                home_location_id: "room".into(),
                shared_capabilities: BTreeSet::new(),
                shared_knowledge: BTreeSet::from([statement.clone()]),
                resources: BTreeSet::new(),
                goals: vec![],
                pressures: vec![],
            },
        );
        campaign.gestalt_members.insert(
            "messenger".into(),
            GestaltMemberDelta {
                schema: "ghostlight.gestalt_member_delta.v1".into(),
                id: "messenger".into(),
                gestalt_id: "refugees".into(),
                version: 0,
                name: "Messenger".into(),
                capability_additions: BTreeSet::new(),
                capability_removals: BTreeSet::new(),
                knowledge_additions: BTreeSet::new(),
                knowledge_removals: BTreeSet::new(),
                equipment: BTreeSet::new(),
                conditions: BTreeSet::new(),
                relationships: BTreeMap::new(),
                goals: vec![],
                obligations: BTreeSet::new(),
                memories: vec![],
                last_location_id: Some("room".into()),
                materialized_actor_id: None,
                last_relevant_revision: 0,
                relevance_lease_until_revision: 0,
            },
        );

        let snapshot = component_snapshot(&campaign).unwrap();
        let key = KnowledgeKey {
            knower: actor_subject("member:messenger"),
            proposition: proposition_subject("fact:route"),
        };
        assert_eq!(snapshot.knowledge[&key].status, "known");

        campaign
            .gestalt_members
            .get_mut("messenger")
            .unwrap()
            .knowledge_removals
            .insert(statement);
        assert!(
            !component_snapshot(&campaign)
                .unwrap()
                .knowledge
                .contains_key(&key)
        );
    }

    #[test]
    fn reaction_identity_adoption_updates_materialized_member_and_delta() {
        let mut campaign = campaign();
        campaign.gestalts.insert(
            "refugees".into(),
            GestaltPersonaState {
                schema: "ghostlight.gestalt_persona_state.v1".into(),
                id: "refugees".into(),
                name: "Refugees".into(),
                version: 0,
                home_location_id: "room".into(),
                shared_capabilities: BTreeSet::new(),
                shared_knowledge: BTreeSet::new(),
                resources: BTreeSet::new(),
                goals: vec![],
                pressures: vec![],
            },
        );
        campaign.gestalt_members.insert(
            "oxygen_patient".into(),
            GestaltMemberDelta {
                schema: "ghostlight.gestalt_member_delta.v1".into(),
                id: "oxygen_patient".into(),
                gestalt_id: "refugees".into(),
                version: 0,
                name: "Refugee Convoy Oxygen Patient".into(),
                capability_additions: BTreeSet::new(),
                capability_removals: BTreeSet::new(),
                knowledge_additions: BTreeSet::new(),
                knowledge_removals: BTreeSet::new(),
                equipment: BTreeSet::new(),
                conditions: BTreeSet::new(),
                obligations: BTreeSet::new(),
                relationships: BTreeMap::new(),
                goals: vec![],
                memories: vec![],
                last_location_id: Some("room".into()),
                materialized_actor_id: Some("member:oxygen_patient".into()),
                last_relevant_revision: 4,
                relevance_lease_until_revision: 6,
            },
        );
        campaign.actors.insert(
            "member:oxygen_patient".into(),
            ActorState {
                id: "member:oxygen_patient".into(),
                name: "Refugee Convoy Oxygen Patient".into(),
                location_id: "room".into(),
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
        let reactions = vec![ActorReaction {
            actor_id: "member:oxygen_patient".into(),
            speech: Some("My name is Taren.".into()),
            deliberate_silence: false,
            private_delta: crate::domain::ActorStateDelta {
                identity_adoption: Some("Taren".into()),
                ..Default::default()
            },
            action_proposals: vec![],
        }];
        let transition = lower_reaction_wave(
            &campaign,
            "Witnessed: the player asks your name.",
            &reactions,
            "reaction:test",
            Utc::now() + Duration::minutes(5),
        )
        .unwrap();
        assert!(transition.batch.mutations.iter().any(|permitted| matches!(
            &permitted.mutation,
            WorldMutation::ChangeIdentity {
                operation: IdentityMutationOperation::Adopt,
                handle_value: Some(value),
                ..
            } if value == "Taren"
        )));
        assert!(transition.batch.mutations.iter().any(|permitted| matches!(
            &permitted.mutation,
            WorldMutation::ChangeIdentity {
                operation: IdentityMutationOperation::Disclose,
                handle_value: Some(value),
                audience,
                ..
            } if value == "Taren"
                && audience == &vec![actor_subject("player"), actor_subject("witness")]
        )));
        apply_lowered_transition(&mut campaign, &transition, Utc::now()).unwrap();
        assert_eq!(campaign.actors["member:oxygen_patient"].name, "Taren");
        assert_eq!(campaign.gestalt_members["oxygen_patient"].name, "Taren");
    }

    #[test]
    fn foreground_legacy_record_can_only_change_state_through_exact_mutations() {
        let mut campaign = campaign();
        let effect = WorldEffectDelta {
            actor_conditions: BTreeMap::from([(
                "player".into(),
                ConditionDelta {
                    add: BTreeSet::from(["winded".into()]),
                    remove: BTreeSet::new(),
                },
            )]),
            actor_commitments: BTreeMap::from([(
                "witness".into(),
                CommitmentDelta {
                    obligations_add: BTreeSet::from([
                        "permit a supervised inventory inspection".into()
                    ]),
                    ..CommitmentDelta::default()
                },
            )]),
            actor_knowledge_additions: BTreeMap::from([(
                "player".into(),
                BTreeSet::from(["The west stair bypasses the checkpoint.".into()]),
            )]),
            actor_observations: BTreeMap::new(),
            actor_relationship_updates: BTreeMap::from([(
                "player".into(),
                BTreeMap::from([("witness".into(), "owes a candid answer".into())]),
            )]),
            actor_moves: BTreeMap::from([("player".into(), "hall".into())]),
            clock_advances: BTreeMap::from([("alarm".into(), 1)]),
            clock_reductions: BTreeMap::new(),
            institution_postures: BTreeMap::from([("watch".into(), "searching".into())]),
        };
        let transition = lower_foreground_effect(
            &campaign,
            "player",
            &effect,
            OutcomeBand::Success,
            MutationProcedure::ForegroundAttempt,
            "The player reaches the hall but raises the alarm.",
            "assessment:test",
            None,
            None,
            Utc::now() + Duration::minutes(5),
        )
        .unwrap();
        assert_eq!(transition.batch.mutations.len(), 7);
        let receipt = apply_lowered_transition(&mut campaign, &transition, Utc::now()).unwrap();
        assert_eq!(receipt.previous_world_revision, 4);
        assert_eq!(receipt.world_revision, 5);
        assert_eq!(campaign.actors["player"].location_id, "hall");
        assert!(campaign.actors["player"].conditions.contains("winded"));
        assert!(
            campaign.actors["player"]
                .knowledge
                .contains("The west stair bypasses the checkpoint.")
        );
        assert_eq!(
            campaign.actors["player"].relationships["witness"],
            "owes a candid answer"
        );
        assert_eq!(campaign.clocks["alarm"].progress, 3);
        assert_eq!(campaign.institutions["watch"].posture, "searching");
        assert!(
            campaign.actors["witness"]
                .obligations
                .contains("permit a supervised inventory inspection")
        );

        let retirement = WorldEffectDelta {
            actor_commitments: BTreeMap::from([(
                "witness".into(),
                CommitmentDelta {
                    obligations_retire: BTreeSet::from([
                        "permit a supervised inventory inspection".into(),
                    ]),
                    ..CommitmentDelta::default()
                },
            )]),
            ..WorldEffectDelta::default()
        };
        let transition = lower_foreground_effect(
            &campaign,
            "player",
            &retirement,
            OutcomeBand::Success,
            MutationProcedure::ForegroundAttempt,
            "The witness retires the supervised-inspection obligation.",
            "assessment:retire",
            None,
            None,
            Utc::now() + Duration::minutes(5),
        )
        .unwrap();
        apply_lowered_transition(&mut campaign, &transition, Utc::now()).unwrap();
        assert!(
            !campaign.actors["witness"]
                .obligations
                .contains("permit a supervised inventory inspection")
        );
    }

    #[test]
    fn foreground_observation_admits_one_branch_proposition_then_acquires_it() {
        let mut campaign = campaign();
        let statement = "The regulator's left coupling carries the highest current thermal stress.";
        let effect = WorldEffectDelta {
            actor_observations: BTreeMap::from([(
                "player".into(),
                BTreeSet::from([statement.into()]),
            )]),
            ..WorldEffectDelta::default()
        };
        let transition = lower_foreground_effect(
            &campaign,
            "player",
            &effect,
            OutcomeBand::Success,
            MutationProcedure::ForegroundAttempt,
            statement,
            "assessment:bounded-observation",
            Some("sha256:means".into()),
            Some("sha256:intended-effect".into()),
            Utc::now() + Duration::minutes(5),
        )
        .unwrap();

        assert_eq!(transition.batch.mutations.len(), 2);
        let proposition_id = match &transition.batch.mutations[0].mutation {
            WorldMutation::AdmitEntity { subject, .. } => subject.id.clone(),
            mutation => panic!("expected proposition admission, got {mutation:?}"),
        };
        assert!(matches!(
            transition.batch.mutations[1].mutation,
            WorldMutation::ChangeKnowledge { .. }
        ));

        apply_lowered_transition(&mut campaign, &transition, Utc::now()).unwrap();
        let fact = &campaign.facts[&proposition_id];
        assert_eq!(fact.statement, statement);
        assert_eq!(fact.scope, FactScope::BranchLocal);
        assert_eq!(
            fact.discoverable_at_location_ids,
            BTreeSet::from(["room".into()])
        );
        assert!(campaign.actors["player"].knowledge.contains(statement));
    }

    #[test]
    fn identical_resource_labels_remain_isolated_by_exact_custody() {
        let mut campaign = campaign();
        campaign
            .actors
            .get_mut("player")
            .unwrap()
            .equipment
            .insert("communication device".into());
        campaign
            .actors
            .get_mut("witness")
            .unwrap()
            .equipment
            .insert("communication device".into());

        let snapshot = component_snapshot(&campaign).unwrap();
        let matching_resources = snapshot
            .resources
            .iter()
            .filter(|(_, resource)| resource.label == "communication device")
            .map(|(resource, _)| {
                (
                    resource.clone(),
                    snapshot.custody.get(resource).cloned().unwrap(),
                )
            })
            .collect::<BTreeSet<_>>();
        assert_eq!(matching_resources.len(), 2);
        assert_eq!(
            matching_resources
                .iter()
                .map(|(_, owner)| owner.clone())
                .collect::<BTreeSet<_>>(),
            BTreeSet::from([actor_subject("player"), actor_subject("witness")])
        );

        let plan = StrategicTickPlan {
            activity_outcomes: vec![StrategicActivityOutcome {
                schema: "ghostlight.strategic_activity_outcome.v1".into(),
                action_digest: format!("sha256:{}", "a".repeat(64)),
                source_subject_id: "witness".into(),
                band: StrategicOutcomeBand::Success,
                summary: "Witness expends a communication device.".into(),
                supporting_state_references: vec!["resource:communication device".into()],
                effect: StrategicOutcomeEffect::ResourceConsumed {
                    owner_subject_id: "witness".into(),
                    resource: "communication device".into(),
                },
            }],
            ..StrategicTickPlan::default()
        };
        let transition = lower_strategic_wave(
            &campaign,
            &plan,
            "strategic:test",
            Utc::now() + Duration::minutes(5),
        )
        .unwrap()
        .unwrap();
        apply_lowered_transition(&mut campaign, &transition, Utc::now()).unwrap();

        assert!(
            campaign.actors["player"]
                .equipment
                .contains("communication device")
        );
        assert!(
            !campaign.actors["witness"]
                .equipment
                .contains("communication device")
        );
    }

    #[test]
    fn strategic_creation_at_a_place_persists_as_a_place_feature() {
        let mut campaign = campaign();
        campaign
            .locations
            .get_mut("room")
            .unwrap()
            .persistent_features
            .push("fixed wall lamp".into());
        let plan = StrategicTickPlan {
            activity_outcomes: vec![StrategicActivityOutcome {
                schema: "ghostlight.strategic_activity_outcome.v1".into(),
                action_digest: format!("sha256:{}", "b".repeat(64)),
                source_subject_id: "witness".into(),
                band: StrategicOutcomeBand::Success,
                summary: "At Room, Witness establishes a public warning marker.".into(),
                supporting_state_references: vec!["capability:field repairs".into()],
                effect: StrategicOutcomeEffect::ResourceCreated {
                    owner_subject_id: "room".into(),
                    resource: "public warning marker".into(),
                },
            }],
            ..StrategicTickPlan::default()
        };

        let transition = lower_strategic_wave(
            &campaign,
            &plan,
            "strategic:place-resource",
            Utc::now() + Duration::minutes(5),
        )
        .unwrap()
        .unwrap();
        apply_lowered_transition(&mut campaign, &transition, Utc::now()).unwrap();

        assert_eq!(
            campaign.locations["room"].persistent_features,
            ["fixed wall lamp", "public warning marker"]
        );
        assert!(
            !campaign.actors["witness"]
                .equipment
                .contains("public warning marker")
        );
        let snapshot = component_snapshot(&campaign).unwrap();
        let marker = snapshot
            .resources
            .iter()
            .find(|(_, resource)| resource.label == "public warning marker")
            .map(|(subject, _)| subject)
            .unwrap();
        assert_eq!(snapshot.custody.get(marker), Some(&place_subject("room")));
    }

    #[test]
    fn strategic_choices_survive_as_exact_actor_and_member_memory() {
        let mut campaign = campaign();
        campaign.gestalts.insert(
            "refugees".into(),
            GestaltPersonaState {
                schema: "ghostlight.gestalt_persona_state.v1".into(),
                id: "refugees".into(),
                name: "Refugees".into(),
                version: 0,
                home_location_id: "room".into(),
                shared_capabilities: BTreeSet::new(),
                shared_knowledge: BTreeSet::new(),
                resources: BTreeSet::new(),
                goals: vec![],
                pressures: vec![],
            },
        );
        campaign.gestalt_members.insert(
            "scout".into(),
            GestaltMemberDelta {
                schema: "ghostlight.gestalt_member_delta.v1".into(),
                id: "scout".into(),
                gestalt_id: "refugees".into(),
                version: 0,
                name: "Refugee Scout".into(),
                capability_additions: BTreeSet::new(),
                capability_removals: BTreeSet::new(),
                knowledge_additions: BTreeSet::new(),
                knowledge_removals: BTreeSet::new(),
                equipment: BTreeSet::new(),
                conditions: BTreeSet::new(),
                obligations: BTreeSet::new(),
                relationships: BTreeMap::new(),
                goals: vec![],
                memories: vec![],
                last_location_id: Some("room".into()),
                materialized_actor_id: None,
                last_relevant_revision: campaign.revision,
                relevance_lease_until_revision: campaign.revision,
            },
        );
        let actor_intent = "inspect the hall before anyone else enters";
        let actor_effect = "identify immediate hazards without claiming the hall is safe";
        let member_intent = "check the room's marked exits";
        let member_effect = "report only routes personally inspected";
        let selected_actions = vec![
            crate::domain::CellActionProposal {
                subject_id: "witness".into(),
                intent: actor_intent.into(),
                intended_effect: actor_effect.into(),
                priority: 60,
                state_references: vec![],
                public_channels: vec![],
                effects: vec![StrategicCellEffect::ActorMove {
                    actor_id: "witness".into(),
                    destination_id: "hall".into(),
                }],
            },
            crate::domain::CellActionProposal {
                subject_id: "member:scout".into(),
                intent: member_intent.into(),
                intended_effect: member_effect.into(),
                priority: 50,
                state_references: vec![],
                public_channels: vec![],
                effects: vec![StrategicCellEffect::MemberActivity {
                    member_id: "scout".into(),
                    activity: crate::domain::StrategicActivityKind::Investigate,
                    target_subject_ids: vec![],
                    location_ids: vec!["room".into()],
                }],
            },
        ];
        let plan =
            crate::resolution::project_selected_actions(&campaign, selected_actions).unwrap();
        let transition = lower_strategic_wave(
            &campaign,
            &plan,
            "strategic:choice-memory",
            Utc::now() + Duration::minutes(5),
        )
        .unwrap()
        .unwrap();

        apply_lowered_transition(&mut campaign, &transition, Utc::now()).unwrap();

        assert_eq!(campaign.actors["witness"].location_id, "hall");
        assert!(campaign.actors["witness"].memories.contains(&format!(
            "I chose to {actor_intent}. I intended to {actor_effect}."
        )));
        assert!(
            campaign.gestalt_members["scout"]
                .memories
                .contains(&format!(
                    "I chose to {member_intent}. I intended to {member_effect}."
                ))
        );
    }

    #[test]
    fn stale_or_tampered_lowering_cannot_partially_project() {
        let mut campaign = campaign();
        let before = campaign.clone();
        let effect = WorldEffectDelta {
            actor_moves: BTreeMap::from([("player".into(), "hall".into())]),
            ..WorldEffectDelta::default()
        };
        let mut transition = lower_foreground_effect(
            &campaign,
            "player",
            &effect,
            OutcomeBand::Success,
            MutationProcedure::ForegroundAttempt,
            "The player reaches the hall.",
            "assessment:test",
            None,
            None,
            Utc::now() + Duration::minutes(5),
        )
        .unwrap();
        let WorldMutation::Relocate { to_place, .. } = &mut transition.batch.mutations[0].mutation
        else {
            unreachable!();
        };
        to_place.id = "forbidden".into();
        transition.batch.digest = mutation_batch_digest(&transition.batch).unwrap();
        assert!(apply_lowered_transition(&mut campaign, &transition, Utc::now()).is_err());
        assert_eq!(campaign, before);
    }
}
