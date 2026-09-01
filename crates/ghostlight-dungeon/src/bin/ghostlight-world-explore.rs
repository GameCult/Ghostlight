use anyhow::{Context, Result, bail};
use ghostlight_dungeon::{domain::Campaign, persistence::CampaignStore};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let store_path = args.next().context(
        "usage: ghostlight-world-explore <campaign.cc> <overview|quality|search|region|subject> [query]",
    )?;
    let command = args.next().unwrap_or_else(|| "overview".into());
    let operand = args.next();
    if args.next().is_some() {
        bail!("too many arguments");
    }

    let campaign = load_campaign(&store_path)?;
    let output = match command.as_str() {
        "overview" => overview(&campaign),
        "quality" => quality(&campaign),
        "search" => search(
            &campaign,
            operand
                .as_deref()
                .context("search requires one case-insensitive query")?,
        ),
        "region" => region(
            &campaign,
            operand
                .as_deref()
                .context("region requires one exact location ID")?,
        )?,
        "subject" => subject(
            &campaign,
            operand
                .as_deref()
                .context("subject requires one exact subject ID")?,
        )?,
        other => bail!(
            "unknown command {other:?}; expected overview, quality, search, region, or subject"
        ),
    };
    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}

fn normalized_text(value: &str) -> String {
    value
        .chars()
        .filter(|character| character.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

fn terminal_name_token(value: &str) -> String {
    value
        .split(|character: char| !character.is_alphanumeric())
        .filter(|token| !token.is_empty())
        .last()
        .map(normalized_text)
        .unwrap_or_default()
}

fn duplicate_groups(entries: impl IntoIterator<Item = (String, String)>) -> Value {
    let mut groups: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (key, id) in entries {
        if !key.is_empty() {
            groups.entry(key).or_default().push(id);
        }
    }
    let mut groups = groups
        .into_iter()
        .filter(|(_, ids)| ids.len() > 1)
        .map(|(key, mut ids)| {
            ids.sort();
            let count = ids.len();
            ids.truncate(20);
            json!({"key":key,"count":count,"subjectIdSample":ids})
        })
        .collect::<Vec<_>>();
    groups.sort_by(|left, right| {
        right["count"]
            .as_u64()
            .cmp(&left["count"].as_u64())
            .then_with(|| left["key"].as_str().cmp(&right["key"].as_str()))
    });
    let group_count = groups.len();
    let occurrence_count = groups
        .iter()
        .filter_map(|group| group["count"].as_u64())
        .sum::<u64>();
    let member_sample_truncated = groups.iter().any(|group| {
        group["count"].as_u64().unwrap_or(0)
            > group["subjectIdSample"]
                .as_array()
                .map_or(0, |sample| sample.len()) as u64
    });
    groups.truncate(20);
    json!({
        "groupCount":group_count,
        "occurrenceCount":occurrence_count,
        "groupsSample":groups,
        "truncated":group_count > 20 || member_sample_truncated,
        "groupSampleLimit":20,
        "subjectIdSampleLimitPerGroup":20,
    })
}

fn gestalt_semantic_key(gestalt: &ghostlight_dungeon::domain::GestaltPersonaState) -> String {
    serde_json::to_string(&(
        &gestalt.shared_capabilities,
        &gestalt.shared_knowledge,
        &gestalt.goals,
        &gestalt.pressures,
    ))
    .unwrap_or_default()
}

fn lineage_depth(campaign: &Campaign, subject_id: &str) -> usize {
    let mut depth = 0;
    let mut current = subject_id;
    let mut visited = BTreeSet::new();
    while visited.insert(current.to_owned()) {
        let Some(parent) = campaign
            .agency_profiles
            .get(current)
            .and_then(|profile| profile.parent_subject_id.as_deref())
        else {
            break;
        };
        depth += 1;
        current = parent;
    }
    depth
}

fn repeated_texts<'a>(values: impl IntoIterator<Item = (&'a str, &'a String)>) -> Value {
    duplicate_groups(
        values
            .into_iter()
            .map(|(id, value)| (normalized_text(value), id.to_owned())),
    )
}

fn has_structurally_complete_named_person_shape(
    has_goal: bool,
    has_memory: bool,
    has_detail_state: bool,
) -> bool {
    has_goal && has_memory && has_detail_state
}

fn quality(campaign: &Campaign) -> Value {
    let residual_ids = campaign
        .gestalt_lineages
        .values()
        .map(|lineage| lineage.residual_child_id.clone())
        .collect::<BTreeSet<_>>();
    let raw_actionable_ids = campaign
        .agency_profiles
        .values()
        .filter(|profile| profile.active_leaf && profile.simulation_eligible)
        .map(|profile| profile.subject_id.clone())
        .collect::<BTreeSet<_>>();
    let qualified_ids = ghostlight_dungeon::elaboration::canonical_actionable_subject_ids(campaign);
    let qualified_count =
        ghostlight_dungeon::elaboration::canonical_actionable_subject_count(campaign);
    let twenty_percent_target = u32::from(campaign.resolution_policy.active_cell_budget) * 5;
    let qualified_target_delta = i64::from(twenty_percent_target) - i64::from(qualified_count);

    let materialized_member_actor_ids = campaign
        .gestalt_members
        .values()
        .flat_map(|member| {
            member
                .materialized_actor_id
                .iter()
                .cloned()
                .chain(std::iter::once(format!("member:{}", member.id)))
        })
        .collect::<BTreeSet<_>>();
    let person_name_records = campaign
        .actors
        .values()
        .filter(|actor| {
            actor.id != campaign.player_actor_id
                && !materialized_member_actor_ids.contains(&actor.id)
        })
        .map(|actor| (actor.name.clone(), format!("actor:{}", actor.id)))
        .chain(
            campaign
                .gestalt_members
                .values()
                .map(|member| (member.name.clone(), format!("member:{}", member.id))),
        )
        .collect::<Vec<_>>();
    let population_name_records = campaign
        .gestalts
        .values()
        .filter(|gestalt| qualified_ids.contains(&gestalt.id))
        .map(|gestalt| (gestalt.name.clone(), gestalt.id.clone()))
        .collect::<Vec<_>>();
    let invalid_person_identity_ids = person_name_records
        .iter()
        .filter(|(name, _)| normalized_text(name).is_empty())
        .map(|(_, id)| id.clone())
        .collect::<Vec<_>>();
    let invalid_population_identity_ids = population_name_records
        .iter()
        .filter(|(name, _)| normalized_text(name).is_empty())
        .map(|(_, id)| id.clone())
        .collect::<Vec<_>>();
    let duplicate_person_names = duplicate_groups(
        person_name_records
            .iter()
            .map(|(name, id)| (normalized_text(name), id.clone())),
    );
    let duplicate_population_names = duplicate_groups(
        population_name_records
            .iter()
            .map(|(name, id)| (normalized_text(name), id.clone())),
    );
    let repeated_person_terminal_tokens = duplicate_groups(
        person_name_records
            .iter()
            .map(|(name, id)| (terminal_name_token(name), id.clone())),
    );
    let repeated_population_terminal_tokens = duplicate_groups(
        population_name_records
            .iter()
            .map(|(name, id)| (terminal_name_token(name), id.clone())),
    );

    // This is a structural record-shape count, not a judgment of semantic quality.
    // A counted standalone actor has at least one goal, at least one memory, and at
    // least one capability, knowledge item, equipment item, condition, obligation,
    // or relationship. The player and actor projections of gestalt members are
    // excluded. A counted gestalt member has at least one goal, at least one memory,
    // and at least one capability addition, knowledge addition, equipment item,
    // condition, obligation, or relationship. Naming validity is measured separately.
    let structurally_complete_actor_count = campaign
        .actors
        .values()
        .filter(|actor| {
            actor.id != campaign.player_actor_id
                && !materialized_member_actor_ids.contains(&actor.id)
        })
        .filter(|actor| {
            has_structurally_complete_named_person_shape(
                !actor.goals.is_empty(),
                !actor.memories.is_empty(),
                !actor.capabilities.is_empty()
                    || !actor.knowledge.is_empty()
                    || !actor.equipment.is_empty()
                    || !actor.conditions.is_empty()
                    || !actor.obligations.is_empty()
                    || !actor.relationships.is_empty(),
            )
        })
        .count();
    let structurally_complete_member_count = campaign
        .gestalt_members
        .values()
        .filter(|member| {
            has_structurally_complete_named_person_shape(
                !member.goals.is_empty(),
                !member.memories.is_empty(),
                !member.capability_additions.is_empty()
                    || !member.knowledge_additions.is_empty()
                    || !member.equipment.is_empty()
                    || !member.conditions.is_empty()
                    || !member.obligations.is_empty()
                    || !member.relationships.is_empty(),
            )
        })
        .count();
    let actor_person_count = campaign
        .actors
        .values()
        .filter(|actor| {
            actor.id != campaign.player_actor_id
                && !materialized_member_actor_ids.contains(&actor.id)
        })
        .count();
    let named_person_count = actor_person_count + campaign.gestalt_members.len();

    let qualified_gestalts = campaign
        .gestalts
        .values()
        .filter(|gestalt| qualified_ids.contains(&gestalt.id))
        .collect::<Vec<_>>();
    let semantic_clone_groups = duplicate_groups(
        qualified_gestalts
            .iter()
            .map(|gestalt| (gestalt_semantic_key(gestalt), gestalt.id.clone())),
    );
    let repeated_goals = repeated_texts(qualified_gestalts.iter().flat_map(|gestalt| {
        gestalt
            .goals
            .iter()
            .map(|value| (gestalt.id.as_str(), value))
    }));
    let repeated_pressures = repeated_texts(qualified_gestalts.iter().flat_map(|gestalt| {
        gestalt
            .pressures
            .iter()
            .map(|value| (gestalt.id.as_str(), value))
    }));
    let direct_fission_deltas = qualified_gestalts
        .iter()
        .filter_map(|gestalt| {
            let parent_id = campaign
                .agency_profiles
                .get(&gestalt.id)?
                .parent_subject_id
                .as_ref()?;
            let parent = campaign.gestalts.get(parent_id)?;
            Some((
                gestalt.id.as_str(),
                gestalt
                    .shared_capabilities
                    .difference(&parent.shared_capabilities)
                    .cloned()
                    .collect::<BTreeSet<_>>(),
                gestalt
                    .shared_knowledge
                    .difference(&parent.shared_knowledge)
                    .cloned()
                    .collect::<BTreeSet<_>>(),
                gestalt
                    .goals
                    .iter()
                    .filter(|goal| !parent.goals.contains(goal))
                    .cloned()
                    .collect::<Vec<_>>(),
                gestalt
                    .pressures
                    .iter()
                    .filter(|pressure| !parent.pressures.contains(pressure))
                    .cloned()
                    .collect::<Vec<_>>(),
            ))
        })
        .collect::<Vec<_>>();
    let repeated_direct_added_goals = repeated_texts(
        direct_fission_deltas
            .iter()
            .flat_map(|(id, _, _, goals, _)| goals.iter().map(|value| (*id, value))),
    );
    let repeated_direct_added_pressures = repeated_texts(
        direct_fission_deltas
            .iter()
            .flat_map(|(id, _, _, _, pressures)| pressures.iter().map(|value| (*id, value))),
    );
    let exact_direct_delta_clone_groups = duplicate_groups(direct_fission_deltas.iter().map(
        |(id, capabilities, knowledge, goals, pressures)| {
            (
                serde_json::to_string(&(capabilities, knowledge, goals, pressures))
                    .unwrap_or_default(),
                (*id).to_owned(),
            )
        },
    ));

    let mut raw_relation_degrees = qualified_ids
        .iter()
        .map(|id| (id.clone(), 0usize))
        .collect::<BTreeMap<_, _>>();
    let mut non_inherited_relation_degrees = raw_relation_degrees.clone();
    let mut raw_relation_kind_counts = BTreeMap::<String, usize>::new();
    let mut non_inherited_relation_kind_counts = BTreeMap::<String, usize>::new();
    let mut mechanically_inherited_relation_count = 0usize;
    for relation in campaign
        .agency_relations
        .values()
        .filter(|relation| relation.active)
    {
        if let Some(degree) = raw_relation_degrees.get_mut(&relation.from_subject_id) {
            *degree += 1;
        }
        if let Some(degree) = raw_relation_degrees.get_mut(&relation.to_subject_id) {
            *degree += 1;
        }
        let kind = format!("{:?}", relation.kind).to_lowercase();
        *raw_relation_kind_counts.entry(kind.clone()).or_default() += 1;
        if relation.id.contains(":fission:") {
            mechanically_inherited_relation_count += 1;
            continue;
        }
        if let Some(degree) = non_inherited_relation_degrees.get_mut(&relation.from_subject_id) {
            *degree += 1;
        }
        if let Some(degree) = non_inherited_relation_degrees.get_mut(&relation.to_subject_id) {
            *degree += 1;
        }
        *non_inherited_relation_kind_counts.entry(kind).or_default() += 1;
    }
    let mut individual_relationship_entries = 0usize;
    let mut record_individual_relationship = |from_subject_id: &str, to_subject_id: &str| {
        individual_relationship_entries += 1;
        if let Some(degree) = raw_relation_degrees.get_mut(from_subject_id) {
            *degree += 1;
        }
        if let Some(degree) = raw_relation_degrees.get_mut(to_subject_id) {
            *degree += 1;
        }
        if let Some(degree) = non_inherited_relation_degrees.get_mut(from_subject_id) {
            *degree += 1;
        }
        if let Some(degree) = non_inherited_relation_degrees.get_mut(to_subject_id) {
            *degree += 1;
        }
        *raw_relation_kind_counts
            .entry("individual_relationship".into())
            .or_default() += 1;
        *non_inherited_relation_kind_counts
            .entry("individual_relationship".into())
            .or_default() += 1;
    };
    for actor in campaign.actors.values() {
        for other_subject_id in actor.relationships.keys() {
            record_individual_relationship(&actor.id, other_subject_id);
        }
    }
    for member in campaign.gestalt_members.values() {
        let default_actor_id = ghostlight_dungeon::domain::gestalt_member_subject_id(&member.id);
        let projected_actor_exists = campaign.actors.contains_key(&default_actor_id)
            || member
                .materialized_actor_id
                .as_ref()
                .is_some_and(|actor_id| campaign.actors.contains_key(actor_id));
        if projected_actor_exists {
            continue;
        }
        for other_subject_id in member.relationships.keys() {
            record_individual_relationship(&default_actor_id, other_subject_id);
        }
    }
    let raw_isolated_subject_ids = raw_relation_degrees
        .iter()
        .filter(|(_, degree)| **degree == 0)
        .map(|(id, _)| id.clone())
        .collect::<Vec<_>>();
    let non_inherited_isolated_subject_ids = non_inherited_relation_degrees
        .iter()
        .filter(|(_, degree)| **degree == 0)
        .map(|(id, _)| id.clone())
        .collect::<Vec<_>>();
    let raw_degree_sum = raw_relation_degrees.values().sum::<usize>();
    let non_inherited_degree_sum = non_inherited_relation_degrees.values().sum::<usize>();

    let civic_signatures = duplicate_groups(campaign.civic_systems.values().map(|civic| {
        let institutions = civic
            .governing_institution_ids
            .iter()
            .filter_map(|id| campaign.institutions.get(id))
            .map(|institution| {
                (
                    normalized_text(&institution.posture),
                    institution
                        .goals
                        .iter()
                        .map(|value| normalized_text(value))
                        .collect::<BTreeSet<_>>(),
                    institution
                        .resources
                        .iter()
                        .map(|value| normalized_text(value))
                        .collect::<BTreeSet<_>>(),
                )
            })
            .collect::<BTreeSet<_>>();
        let residents = civic
            .resident_population_ids
            .iter()
            .filter_map(|id| campaign.gestalts.get(id))
            .map(gestalt_semantic_key)
            .collect::<BTreeSet<_>>();
        (
            serde_json::to_string(&(institutions, residents)).unwrap_or_default(),
            civic.jurisdiction_location_id.clone(),
        )
    }));

    let event_kind_counts =
        campaign
            .events
            .iter()
            .fold(BTreeMap::<String, usize>::new(), |mut counts, event| {
                *counts.entry(event.kind.clone()).or_default() += 1;
                counts
            });
    let repeated_event_summaries = duplicate_groups(
        campaign
            .events
            .iter()
            .map(|event| (normalized_text(&event.summary), event.id.clone())),
    );
    let distribution_by_root = campaign
        .locations
        .values()
        .filter(|location| location.container_id.is_none())
        .map(|location| {
            let places = descendant_location_ids(campaign, &location.id);
            let count = qualified_ids
                .iter()
                .filter(|id| {
                    campaign
                        .agency_profiles
                        .get(*id)
                        .is_some_and(|profile| !profile.location_ids.is_disjoint(&places))
                })
                .count();
            (location.id.clone(), count)
        })
        .collect::<BTreeMap<_, _>>();
    let spread_by_jurisdiction = campaign
        .civic_systems
        .keys()
        .filter(|location_id| campaign.locations.contains_key(*location_id))
        .map(|location_id| {
            let places = descendant_location_ids(campaign, location_id);
            let location_counts = places
                .iter()
                .map(|place_id| {
                    let count = qualified_ids
                        .iter()
                        .filter(|id| {
                            campaign
                                .agency_profiles
                                .get(*id)
                                .is_some_and(|profile| profile.location_ids.contains(place_id))
                        })
                        .count();
                    (place_id.clone(), count)
                })
                .collect::<BTreeMap<_, _>>();
            let occupied_location_count = location_counts
                .values()
                .filter(|count| **count > 0)
                .count();
            let location_assignment_count = location_counts.values().sum::<usize>();
            let largest_location_count = location_counts.values().copied().max().unwrap_or(0);
            (
                location_id.clone(),
                json!({
                    "qualifiedSubjects":qualified_ids.iter().filter(|id|campaign.agency_profiles.get(*id).is_some_and(|profile|!profile.location_ids.is_disjoint(&places))).count(),
                    "occupiedLocationCount":occupied_location_count,
                    "locationAssignmentCount":location_assignment_count,
                    "largestLocationShareBasisPoints":if location_assignment_count == 0 { None } else { Some(largest_location_count * 10_000 / location_assignment_count) },
                    "qualifiedSubjectCountsByLocation":location_counts,
                }),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let occupied_location_count = campaign
        .locations
        .keys()
        .filter(|location_id| {
            qualified_ids.iter().any(|id| {
                campaign
                    .agency_profiles
                    .get(id)
                    .is_some_and(|profile| profile.location_ids.contains(*location_id))
            })
        })
        .count();

    json!({
        "schema":"ghostlight.world_quality.v1",
        "campaign":{"id":campaign.id,"revision":campaign.revision,"strategicTickCount":campaign.strategic_tick_count},
        "scale":{
            "rawActionableSubjects":raw_actionable_ids.len(),
            "qualifiedActionableSubjects":qualified_count,
            "residualActionableSubjects":raw_actionable_ids.intersection(&residual_ids).count(),
            "twentyPercentTarget":twenty_percent_target,
            "qualifiedTargetDelta":qualified_target_delta,
            "qualifiedCoverBasisPoints":if qualified_count == 0 { None } else { Some(u32::from(campaign.resolution_policy.active_cell_budget) * 10_000 / qualified_count) },
            "rawCoverBasisPoints":if raw_actionable_ids.is_empty() { None } else { Some(usize::from(campaign.resolution_policy.active_cell_budget) * 10_000 / raw_actionable_ids.len()) },
            "maximumQualifiedLineageDepth":qualified_ids.iter().map(|id|lineage_depth(campaign,id)).max().unwrap_or(0),
            "qualifiedDistributionByRootLocation":distribution_by_root,
            "occupiedLocationCount":occupied_location_count,
            "qualifiedSpreadByJurisdiction":spread_by_jurisdiction,
            "geographyFissionCount":campaign.gestalt_lineages.values().filter(|lineage|lineage.partition_axis == ghostlight_dungeon::domain::AgencyAxis::Geography).count(),
        },
        "identity":{
            "namedPersons":named_person_count,
            "structurallyCompleteNamedPersons":structurally_complete_actor_count + structurally_complete_member_count,
            "invalidPublicPersonIdentityIds":invalid_person_identity_ids,
            "invalidQualifiedPopulationIdentityIds":invalid_population_identity_ids,
            "duplicatePersonNames":duplicate_person_names,
            "duplicateQualifiedPopulationNames":duplicate_population_names,
            "repeatedPersonTerminalNameTokens":repeated_person_terminal_tokens,
            "repeatedQualifiedPopulationTerminalNameTokens":repeated_population_terminal_tokens,
            "lexicalDiagnosticsOnly":true,
        },
        "causality":{
            "qualifiedGestalts":qualified_gestalts.len(),
            "qualifiedGestaltsWithoutGoals":qualified_gestalts.iter().filter(|gestalt|gestalt.goals.is_empty()).count(),
            "qualifiedGestaltsWithoutPressures":qualified_gestalts.iter().filter(|gestalt|gestalt.pressures.is_empty()).count(),
            "qualifiedGestaltsWithoutCapabilities":qualified_gestalts.iter().filter(|gestalt|gestalt.shared_capabilities.is_empty()).count(),
            "qualifiedGestaltsWithoutKnowledge":qualified_gestalts.iter().filter(|gestalt|gestalt.shared_knowledge.is_empty()).count(),
            "inheritanceInclusiveExactSemanticCloneGroups":semantic_clone_groups,
            "inheritanceInclusiveRepeatedGoals":repeated_goals,
            "inheritanceInclusiveRepeatedPressures":repeated_pressures,
            "exactDirectFissionDeltaCloneGroups":exact_direct_delta_clone_groups,
            "repeatedDirectAddedGoals":repeated_direct_added_goals,
            "repeatedDirectAddedPressures":repeated_direct_added_pressures,
        },
        "relationships":{
            "collectiveAgencyActiveRelations":campaign.agency_relations.values().filter(|relation|relation.active).count(),
            "individualRelationshipEntries":individual_relationship_entries,
            "rawActiveRelationshipEntries":campaign.agency_relations.values().filter(|relation|relation.active).count() + individual_relationship_entries,
            "estimatedMechanicallyInheritedFissionRelationsByIdConvention":mechanically_inherited_relation_count,
            "nonInheritedActiveRelationshipEntries":campaign.agency_relations.values().filter(|relation|relation.active && !relation.id.contains(":fission:")).count() + individual_relationship_entries,
            "rawRelationKindCounts":raw_relation_kind_counts,
            "nonInheritedRelationKindCounts":non_inherited_relation_kind_counts,
            "rawIsolatedQualifiedSubjects":raw_isolated_subject_ids.len(),
            "rawIsolatedQualifiedSubjectSample":raw_isolated_subject_ids.into_iter().take(50).collect::<Vec<_>>(),
            "nonInheritedIsolatedQualifiedSubjects":non_inherited_isolated_subject_ids.len(),
            "nonInheritedIsolatedQualifiedSubjectSample":non_inherited_isolated_subject_ids.into_iter().take(50).collect::<Vec<_>>(),
            "rawMeanQualifiedDegree":if raw_relation_degrees.is_empty() { 0.0 } else { raw_degree_sum as f64 / raw_relation_degrees.len() as f64 },
            "nonInheritedMeanQualifiedDegree":if non_inherited_relation_degrees.is_empty() { 0.0 } else { non_inherited_degree_sum as f64 / non_inherited_relation_degrees.len() as f64 },
            "inheritanceClassification":"estimate based on the current :fission: relation-ID convention; authored IDs can confound it",
        },
        "politics":{
            "civicJurisdictions":campaign.civic_systems.len(),
            "exactCivicSignatureCloneGroups":civic_signatures,
            "exactSignatureDiagnosticsDoNotMeasureSemanticPoliticalDiversity":true,
        },
        "behavior":{
            "eventKindCounts":event_kind_counts,
            "exactRepeatedEventSummaryGroups":repeated_event_summaries,
            "newsIssues":campaign.news.len(),
            "exactSummaryDiagnosticsDoNotMeasureSemanticDramaticDiversity":true,
        },
    })
}

fn load_campaign(path: &str) -> Result<Campaign> {
    let store = CampaignStore::open(path)?;
    let key = store
        .keys("campaign.v1")?
        .into_iter()
        .next()
        .context("store contains no campaign.v1 state")?;
    store
        .load::<Campaign>("campaign.v1", &key)?
        .map(|(_, campaign)| campaign)
        .context("campaign.v1 row disappeared during inspection")
}

fn overview(campaign: &Campaign) -> Value {
    let roots = campaign
        .locations
        .values()
        .filter(|location| location.container_id.is_none())
        .map(|location| location_summary(campaign, &location.id))
        .collect::<Vec<_>>();
    let civic_jurisdictions = campaign
        .civic_systems
        .values()
        .map(|civic| {
            json!({
                "locationId": civic.jurisdiction_location_id,
                "locationName": location_name(campaign, &civic.jurisdiction_location_id),
                "governingInstitutions": civic.governing_institution_ids.iter().map(|id| json!({
                    "id": id,
                    "name": campaign.institutions.get(id).map(|institution| institution.name.as_str()),
                })).collect::<Vec<_>>(),
                "residentPopulationCount": civic.resident_population_ids.len(),
            })
        })
        .collect::<Vec<_>>();
    json!({
        "schema": "ghostlight.world_exploration.v1",
        "campaign": {
            "id": campaign.id,
            "name": campaign.name,
            "revision": campaign.revision,
            "worldTime": campaign.world_time,
            "resolutionEpoch": campaign.resolution_policy.resolution_epoch,
            "activeCellBudget": campaign.resolution_policy.active_cell_budget,
        },
        "counts": {
            "locations": campaign.locations.len(),
            "actors": campaign.actors.len(),
            "institutions": campaign.institutions.len(),
            "gestalts": campaign.gestalts.len(),
            "gestaltMembers": campaign.gestalt_members.len(),
            "facts": campaign.facts.len(),
            "clocks": campaign.clocks.len(),
            "agencyRelations": campaign.agency_relations.len(),
            "civicSystems": campaign.civic_systems.len(),
        },
        "rootLocations": roots,
        "civicJurisdictions": civic_jurisdictions,
        "clocks": campaign.clocks,
    })
}

fn location_summary(campaign: &Campaign, location_id: &str) -> Value {
    let location = &campaign.locations[location_id];
    let descendants = descendant_location_ids(campaign, location_id);
    let subject_counts = subject_counts_in_locations(campaign, &descendants);
    json!({
        "id": location.id,
        "name": location.name,
        "containerId": location.container_id,
        "persistentFeatures": location.persistent_features,
        "directChildren": campaign.locations.values().filter(|candidate| candidate.container_id.as_deref() == Some(location_id)).map(|candidate| json!({"id":candidate.id,"name":candidate.name})).collect::<Vec<_>>(),
        "descendantLocationCount": descendants.len().saturating_sub(1),
        "subjectCounts": subject_counts,
    })
}

fn search(campaign: &Campaign, query: &str) -> Value {
    let needle = query.to_lowercase();
    let mut matches = Vec::new();
    for location in campaign.locations.values() {
        push_match(
            &mut matches,
            &needle,
            "location",
            &location.id,
            &location.name,
            json!(location),
        );
    }
    for actor in campaign.actors.values() {
        push_match(
            &mut matches,
            &needle,
            "actor",
            &actor.id,
            &actor.name,
            json!(actor),
        );
    }
    for institution in campaign.institutions.values() {
        push_match(
            &mut matches,
            &needle,
            "institution",
            &institution.id,
            &institution.name,
            json!(institution),
        );
    }
    for gestalt in campaign.gestalts.values() {
        push_match(
            &mut matches,
            &needle,
            "gestalt",
            &gestalt.id,
            &gestalt.name,
            json!(gestalt),
        );
    }
    for member in campaign.gestalt_members.values() {
        push_match(
            &mut matches,
            &needle,
            "gestalt_member",
            &member.id,
            &member.name,
            json!(member),
        );
    }
    for fact in campaign.facts.values() {
        push_match(
            &mut matches,
            &needle,
            "fact",
            &fact.id,
            &fact.statement,
            json!(fact),
        );
    }
    for clock in campaign.clocks.values() {
        push_match(
            &mut matches,
            &needle,
            "clock",
            &clock.id,
            &clock.label,
            json!(clock),
        );
    }
    let total_match_count = matches.len();
    matches.truncate(200);
    json!({
        "schema": "ghostlight.world_search.v1",
        "query": query,
        "totalMatchCount": total_match_count,
        "returnedMatchCount": matches.len(),
        "truncated": total_match_count > matches.len(),
        "matches": matches,
    })
}

fn push_match(
    matches: &mut Vec<Value>,
    needle: &str,
    kind: &str,
    id: &str,
    label: &str,
    value: Value,
) {
    let haystack = value.to_string().to_lowercase();
    if haystack.contains(needle) {
        matches.push(json!({"kind": kind, "id": id, "label": label, "value": value}));
    }
}

fn region(campaign: &Campaign, location_id: &str) -> Result<Value> {
    if !campaign.locations.contains_key(location_id) {
        bail!("unknown location ID {location_id:?}");
    }
    let location_ids = descendant_location_ids(campaign, location_id);
    let subject_ids = subject_ids_in_locations(campaign, &location_ids);
    let locations = location_ids
        .iter()
        .map(|id| (id.clone(), json!(&campaign.locations[id])))
        .collect::<BTreeMap<_, _>>();
    let subjects = subject_ids
        .iter()
        .filter_map(|id| subject_value(campaign, id).map(|value| (id.clone(), value)))
        .collect::<BTreeMap<_, _>>();
    let relations = campaign
        .agency_relations
        .values()
        .filter(|relation| {
            subject_ids.contains(&relation.from_subject_id)
                || subject_ids.contains(&relation.to_subject_id)
        })
        .collect::<Vec<_>>();
    let civic_systems = campaign
        .civic_systems
        .values()
        .filter(|civic| location_ids.contains(&civic.jurisdiction_location_id))
        .collect::<Vec<_>>();
    let facts = campaign
        .facts
        .values()
        .filter(|fact| {
            let serialized = serde_json::to_string(fact).unwrap_or_default();
            location_ids.iter().any(|id| serialized.contains(id))
                || subject_ids.iter().any(|id| serialized.contains(id))
        })
        .collect::<Vec<_>>();
    Ok(json!({
        "schema": "ghostlight.world_region.v1",
        "root": location_summary(campaign, location_id),
        "locations": locations,
        "subjects": subjects,
        "relations": relations,
        "civicSystems": civic_systems,
        "facts": facts,
    }))
}

fn subject(campaign: &Campaign, subject_id: &str) -> Result<Value> {
    let value = subject_value(campaign, subject_id)
        .with_context(|| format!("unknown subject ID {subject_id:?}"))?;
    let relations = campaign
        .agency_relations
        .values()
        .filter(|relation| {
            relation.from_subject_id == subject_id || relation.to_subject_id == subject_id
        })
        .collect::<Vec<_>>();
    Ok(json!({
        "schema": "ghostlight.world_subject.v1",
        "subject": value,
        "relations": relations,
    }))
}

fn subject_value(campaign: &Campaign, subject_id: &str) -> Option<Value> {
    if let Some(actor) = campaign.actors.get(subject_id) {
        return Some(
            json!({"kind":"actor", "state":actor, "agency":campaign.agency_profiles.get(subject_id)}),
        );
    }
    if let Some(institution) = campaign.institutions.get(subject_id) {
        return Some(
            json!({"kind":"institution", "state":institution, "agency":campaign.agency_profiles.get(subject_id)}),
        );
    }
    if let Some(gestalt) = campaign.gestalts.get(subject_id) {
        return Some(
            json!({"kind":"gestalt", "state":gestalt, "agency":campaign.agency_profiles.get(subject_id), "lineage":campaign.gestalt_lineages.get(subject_id)}),
        );
    }
    let member_id = subject_id.strip_prefix("member:").unwrap_or(subject_id);
    campaign.gestalt_members.get(member_id).map(|member| {
        json!({"kind":"gestalt_member", "state":member, "parent":campaign.gestalts.get(&member.gestalt_id)})
    })
}

fn descendant_location_ids(campaign: &Campaign, root_id: &str) -> BTreeSet<String> {
    let mut ids = BTreeSet::from([root_id.to_owned()]);
    loop {
        let before = ids.len();
        for location in campaign.locations.values() {
            if location
                .container_id
                .as_ref()
                .is_some_and(|container| ids.contains(container))
            {
                ids.insert(location.id.clone());
            }
        }
        if ids.len() == before {
            return ids;
        }
    }
}

fn subject_ids_in_locations(
    campaign: &Campaign,
    location_ids: &BTreeSet<String>,
) -> BTreeSet<String> {
    let mut ids = BTreeSet::new();
    for actor in campaign.actors.values() {
        if location_ids.contains(&actor.location_id) {
            ids.insert(actor.id.clone());
        }
    }
    for gestalt in campaign.gestalts.values() {
        if location_ids.contains(&gestalt.home_location_id) {
            ids.insert(gestalt.id.clone());
        }
    }
    for (id, profile) in &campaign.agency_profiles {
        if !profile.location_ids.is_disjoint(location_ids) {
            ids.insert(id.clone());
        }
    }
    for member in campaign.gestalt_members.values() {
        if ids.contains(&member.gestalt_id)
            || member
                .last_location_id
                .as_ref()
                .is_some_and(|id| location_ids.contains(id))
        {
            ids.insert(format!("member:{}", member.id));
        }
    }
    ids
}

fn subject_counts_in_locations(
    campaign: &Campaign,
    location_ids: &BTreeSet<String>,
) -> BTreeMap<&'static str, usize> {
    let ids = subject_ids_in_locations(campaign, location_ids);
    let mut counts = BTreeMap::from([
        ("actors", 0),
        ("institutions", 0),
        ("gestalts", 0),
        ("gestaltMembers", 0),
    ]);
    for id in ids {
        if id.starts_with("member:") {
            counts
                .entry("gestaltMembers")
                .and_modify(|count| *count += 1);
        } else if campaign.actors.contains_key(&id) {
            counts.entry("actors").and_modify(|count| *count += 1);
        } else if campaign.institutions.contains_key(&id) {
            counts.entry("institutions").and_modify(|count| *count += 1);
        } else if campaign.gestalts.contains_key(&id) {
            counts.entry("gestalts").and_modify(|count| *count += 1);
        }
    }
    counts
}

fn location_name<'a>(campaign: &'a Campaign, id: &str) -> Option<&'a str> {
    campaign
        .locations
        .get(id)
        .map(|location| location.name.as_str())
}

#[cfg(test)]
mod tests {
    use super::{duplicate_groups, has_structurally_complete_named_person_shape};

    #[test]
    fn named_person_structural_completeness_requires_goal_memory_and_detail_state() {
        assert!(has_structurally_complete_named_person_shape(
            true, true, true
        ));
        assert!(!has_structurally_complete_named_person_shape(
            false, true, true
        ));
        assert!(!has_structurally_complete_named_person_shape(
            true, false, true
        ));
        assert!(!has_structurally_complete_named_person_shape(
            true, true, false
        ));
    }

    #[test]
    fn duplicate_report_preserves_complete_prevalence_while_sampling_large_results() {
        let mut entries = (0..25)
            .map(|index| ("shared".to_owned(), format!("shared-{index:02}")))
            .collect::<Vec<_>>();
        for group in 0..21 {
            entries.push((format!("group-{group:02}"), format!("group-{group:02}-a")));
            entries.push((format!("group-{group:02}"), format!("group-{group:02}-b")));
        }

        let report = duplicate_groups(entries);

        assert_eq!(report["groupCount"], 22);
        assert_eq!(report["occurrenceCount"], 67);
        assert_eq!(report["groupsSample"].as_array().unwrap().len(), 20);
        assert_eq!(report["groupsSample"][0]["count"], 25);
        assert_eq!(
            report["groupsSample"][0]["subjectIdSample"]
                .as_array()
                .unwrap()
                .len(),
            20
        );
        assert_eq!(report["truncated"], true);
    }
}
