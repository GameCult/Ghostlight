use anyhow::{Context, Result, bail};
use ghostlight_dungeon::{domain::Campaign, persistence::CampaignStore};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let store_path = args.next().context(
        "usage: ghostlight-world-explore <campaign.cc> <overview|search|region|subject> [query]",
    )?;
    let command = args.next().unwrap_or_else(|| "overview".into());
    let operand = args.next();
    if args.next().is_some() {
        bail!("too many arguments");
    }

    let campaign = load_campaign(&store_path)?;
    let output = match command.as_str() {
        "overview" => overview(&campaign),
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
        other => bail!("unknown command {other:?}; expected overview, search, region, or subject"),
    };
    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
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
