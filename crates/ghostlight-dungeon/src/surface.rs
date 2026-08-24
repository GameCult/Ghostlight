use crate::{
    domain::{
        Campaign, GestaltMaterializationReceipt, RejectedProposalReceipt, StrategicTickReceipt,
        VaultEvidenceReceipt, WorldCommitReceipt,
    },
    model::ModelStageReceipt,
};
use serde_json::{Value, json};

pub const CAMPAIGN_REVISION_SCALE: u64 = 1_000_000_000_000;

pub fn campaign_interface_version(campaign: &Campaign) -> u64 {
    campaign
        .revision
        .saturating_mul(CAMPAIGN_REVISION_SCALE)
        .saturating_add(
            campaign
                .resolution_policy
                .resolution_epoch
                .saturating_mul(1_000_000),
        )
        .saturating_add(campaign.resolution_policy.provider_configuration_epoch)
}

pub fn rebase_campaign_surface_revision(source_version: u64, campaign_revision: u64) -> u64 {
    campaign_revision
        .saturating_mul(CAMPAIGN_REVISION_SCALE)
        .saturating_add(source_version % CAMPAIGN_REVISION_SCALE)
}

pub fn player_surface(campaign: &Campaign) -> Value {
    player_surface_for_actor(campaign, &campaign.player_actor_id)
}

pub fn player_surface_for_actor(campaign: &Campaign, viewer_actor_id: &str) -> Value {
    let interface_version = campaign_interface_version(campaign);
    let story = story_nodes(campaign);
    let player = &campaign.actors[viewer_actor_id];
    let location = &campaign.locations[&player.location_id];
    let mut present_characters = campaign
        .actors
        .values()
        .filter(|actor| actor.id != viewer_actor_id && actor.location_id == player.location_id)
        .map(|actor| actor.name.clone())
        .collect::<Vec<_>>();
    present_characters.sort();
    let mut local_populations = campaign
        .gestalts
        .values()
        .filter(|gestalt| {
            campaign
                .agency_profiles
                .get(&gestalt.id)
                .is_some_and(|profile| profile.location_ids.contains(&player.location_id))
        })
        .map(|gestalt| gestalt.name.clone())
        .collect::<Vec<_>>();
    local_populations.sort();
    let scene_presence = format!(
        "Present characters: {}\nLocal populations: {}",
        if present_characters.is_empty() {
            "none".into()
        } else {
            present_characters.join(", ")
        },
        if local_populations.is_empty() {
            "none".into()
        } else {
            local_populations.join(", ")
        },
    );
    let relationships = if player.relationships.is_empty() {
        "none".into()
    } else {
        player
            .relationships
            .iter()
            .map(|(subject_id, relationship)| {
                let display_name = campaign
                    .actors
                    .get(subject_id)
                    .map(|actor| actor.name.as_str())
                    .or_else(|| {
                        campaign
                            .institutions
                            .get(subject_id)
                            .map(|institution| institution.name.as_str())
                    })
                    .or_else(|| {
                        campaign
                            .gestalts
                            .get(subject_id)
                            .map(|gestalt| gestalt.name.as_str())
                    })
                    .or_else(|| {
                        subject_id
                            .strip_prefix("member:")
                            .and_then(|member_id| campaign.gestalt_members.get(member_id))
                            .map(|member| member.name.as_str())
                    })
                    .unwrap_or(subject_id);
                format!("{display_name}: {relationship}")
            })
            .collect::<Vec<_>>()
            .join(", ")
    };
    let player_profile = campaign.agency_profiles.get(viewer_actor_id);
    let information_channels = player_profile
        .map(|profile| join(&profile.information_channels))
        .unwrap_or_else(|| "none".into());
    let ledger = format!(
        "Capabilities: {}\nEquipment: {}\nConditions: {}\nObligations: {}\nRelationships: {}\nKnown facts: {}\nInformation access: {}",
        join(&player.capabilities),
        join(&player.equipment),
        join(&player.conditions),
        join(&player.obligations),
        relationships,
        join(&player.knowledge),
        information_channels,
    );
    let accessible_channels = player_profile.map(|profile| &profile.information_channels);
    let accessible_news = campaign
        .news
        .iter()
        .filter(|item| accessible_channels.is_some_and(|channels| channels.contains(&item.channel)))
        .collect::<Vec<_>>();
    let reported_event_ids = accessible_news
        .iter()
        .flat_map(|item| item.event_ids.iter().cloned())
        .collect::<std::collections::BTreeSet<_>>();
    let mut news = campaign
        .events
        .iter()
        .rev()
        .filter(|event| !reported_event_ids.contains(&event.id))
        .filter(|event| {
            event.kind != "reaction_wave"
                && event.kind != "group_travel"
                && !event.actor_ids.iter().any(|actor_id| actor_id == viewer_actor_id)
        })
        .filter(|event| {
            player_profile.is_some_and(|profile| {
                crate::scheduler::subject_perceives_event(
                    viewer_actor_id,
                    &profile.location_ids,
                    &profile.information_channels,
                    event,
                )
            })
        })
        .take(12)
        .map(|event| json!({"id":format!("observed:{}",event.id),"kind":"text","props":{"value":format!("[observed] {}",event.summary)},"children":[]}))
        .collect::<Vec<_>>();
    news.extend(accessible_news.into_iter().map(|item|json!({"id":item.id,"kind":"text","props":{"value":format!("[{}] {}",item.channel,item.headline)},"children":[]})));
    let effective_budget = campaign
        .resolution_cover
        .as_ref()
        .map(|cover| cover.effective_budget)
        .unwrap_or(campaign.resolution_policy.active_cell_budget);
    let strategic_wait_minutes = u32::from(campaign.tick_hours).saturating_mul(60);
    let mut children = vec![
        json!({"id":"dungeon.status","kind":"card","props":{"title":format!("{} · revision {} · {}",campaign.name,campaign.revision,campaign.world_time)},"children":[]}),
        json!({"id":"dungeon.location","kind":"card","props":{"title":format!("{} · {}",location.name,player.name)},"children":[]}),
        json!({"id":"dungeon.scene-presence","kind":"card","props":{"title":"Scene presence"},"children":[{"id":"dungeon.scene-presence.text","kind":"text","props":{"value":scene_presence},"children":[]}]}),
        json!({"id":"dungeon.ledger","kind":"card","props":{"title":"Character ledger"},"children":[{"id":"dungeon.ledger.text","kind":"text","props":{"value":ledger},"children":[]}]}),
        json!({"id":"dungeon.resolution","kind":"card","props":{"title":format!("World resolution · {} configured / {} effective",campaign.resolution_policy.active_cell_budget,effective_budget)},"children":[{"id":"dungeon.resolution.text","kind":"text","props":{"value":format!("Resolution epoch {} · {} pins · {} temporary overage",campaign.resolution_policy.resolution_epoch,campaign.resolution_pins.len(),campaign.resolution_cover.as_ref().map(|cover| cover.mandatory_overage).unwrap_or(0))},"children":[]}]}),
        json!({"id":"dungeon.news","kind":"card","props":{"title":"Accessible news and rumors"},"children":news}),
        json!({"id":"dungeon.transcript","kind":"card","props":{"title":"Story"},"children":story}),
        json!({"id":"dungeon.speech","kind":"control.input.textarea","props":{"label":"Say something","rows":3,"placeholder":"Your character's exact words"},"stateBindings":[local_draft("text","string")],"children":[]}),
        command_control(
            "dungeon.speak",
            "Speak",
            "world.speak",
            json!({"expected_revision":campaign.revision}),
            &["text"],
        ),
        json!({"id":"dungeon.attempt.description","kind":"control.input.textarea","props":{"label":"What do you try?","rows":4,"placeholder":"Describe the attempt, not a completed fact."},"stateBindings":[local_draft("description","string")],"children":[]}),
        json!({"id":"dungeon.attempt.effect","kind":"control.input.text","props":{"label":"What effect do you want?","placeholder":"The concrete outcome you are trying to cause"},"stateBindings":[local_draft("intended_effect","string")],"children":[]}),
        command_control(
            "dungeon.assess",
            "Assess stakes",
            "world.assess",
            json!({"expected_revision":campaign.revision}),
            &["description", "intended_effect"],
        ),
        json!({"id":"dungeon.wait.minutes","kind":"control.input.number","props":{"label":format!("Wait minutes ({} max)",strategic_wait_minutes),"value":30_u32.min(strategic_wait_minutes),"min":1,"max":strategic_wait_minutes},"stateBindings":[local_draft("minutes","number")],"children":[]}),
        command_control(
            "dungeon.wait",
            "Wait",
            "world.wait",
            json!({"expected_revision":campaign.revision}),
            &["minutes"],
        ),
        json!({"id":"dungeon.time.minutes","kind":"control.input.number","props":{"label":"Proposed group time advance (minutes)","value":30,"min":1,"max":1440},"stateBindings":[local_draft("time_advance_minutes","number")],"children":[]}),
        command_control(
            "dungeon.time.propose",
            "Propose group time advance",
            "governance.time.propose",
            json!({"expected_revision":campaign.revision}),
            &["time_advance_minutes"],
        ),
        json!({"id":"dungeon.cells.budget","kind":"control.input.number","props":{"label":"Active Persona-cell budget","value":campaign.resolution_policy.active_cell_budget,"min":1,"max":128},"stateBindings":[local_draft("active_cell_budget","number")],"children":[]}),
        command_control(
            "dungeon.cells.propose",
            "Propose cell budget",
            "governance.cells.propose",
            json!({"expected_revision":campaign.revision,"expected_resolution_epoch":campaign.resolution_policy.resolution_epoch}),
            &["active_cell_budget"],
        ),
        command_control(
            "dungeon.contract-review",
            "Review campaign contract",
            "campaign.contract_review.begin",
            json!({}),
            &[],
        ),
        command_control(
            "dungeon.campaign-entry",
            "Campaigns and new Session Zero",
            "campaign.entry",
            json!({}),
            &[],
        ),
        command_control(
            "dungeon.logout",
            "Sign out",
            "app.auth.logout",
            json!({}),
            &[],
        ),
    ];
    if location.routes.is_empty() {
        children.push(json!({"id":"dungeon.travel.none","kind":"text","props":{"value":"No compiled route leaves this location yet."},"children":[]}));
    } else {
        let default_destination_id = location
            .routes
            .keys()
            .next()
            .expect("non-empty routes have a first destination");
        children.push(json!({
            "id":"dungeon.travel.destination",
            "kind":"control.select",
            "props":{"label":"Group destination","value":default_destination_id},
            "stateBindings":[local_draft("destination_location_id","choice")],
            "children":location.routes.iter().filter_map(|(destination_id,route)|campaign.locations.get(destination_id).map(|destination|json!({
                "id":format!("dungeon.travel.option.{destination_id}"),
                "kind":"control.option",
                "props":{"value":destination_id,"label":format!("{} · {} minutes · {}",destination.name,route.travel_minutes,route.distance)},
                "children":[]
            }))).collect::<Vec<_>>()
        }));
        children.push(command_control(
            "dungeon.travel.propose",
            "Propose group travel",
            "governance.travel.propose",
            json!({"expected_revision":campaign.revision}),
            &["destination_location_id"],
        ));
    }
    children.push(json!({"id":"dungeon.destination.name","kind":"control.input.text","props":{"label":"Compile a new destination","placeholder":"Describe where you want to go"},"stateBindings":[local_draft("destination","string")],"children":[]}));
    children.push(command_control(
        "dungeon.destination.compile",
        "Compile destination preview",
        "world.destination.compile",
        json!({}),
        &["destination"],
    ));
    json!({
      "type":"surface-state", "schema":"gamecult.eve.surface.v1", "providerId":"gamecult.ghostlight.dungeon",
      "providerKind":"narrative.simulation", "title":campaign.name, "version":interface_version,
      "updatedAtUtc":chrono::Utc::now().to_rfc3339(),
      "world_revision":campaign.revision,
      "viewer_actor_id":viewer_actor_id,
      "player_location_id":player.location_id,
      "reachable_destinations":location.routes.iter().filter_map(|(destination_id,route)|campaign.locations.get(destination_id).map(|destination|json!({"id":destination_id,"name":destination.name,"travel_minutes":route.travel_minutes,"distance":route.distance}))).collect::<Vec<_>>(),
      "resolution":{
        "policy":campaign.resolution_policy,
        "effective_budget":effective_budget,
        "mandatory_overage":campaign.resolution_cover.as_ref().map(|cover| cover.mandatory_overage).unwrap_or(0),
        "pin_count":campaign.resolution_pins.len(),
        "fission_targets":campaign.agency_profiles.values().filter(|profile| profile.active_leaf && profile.subject_kind == crate::domain::AgencySubjectKind::Gestalt).filter_map(|profile| campaign.gestalts.get(&profile.subject_id)).filter(|gestalt|gestalt.home_location_id==player.location_id).map(|gestalt| json!({"id":gestalt.id,"name":gestalt.name})).collect::<Vec<_>>()
      },
      "surface":{"id":"ghostlight.play","root":{"id":"dungeon.root","kind":"surface","props":{},"children":children},"styles":{"tokens":{"colorBackground":"#0c1110","colorPanel":"#17201d","colorText":"#e8e1cf","colorMuted":"#9aa69f","colorAccent":"#d49b58"}}},
      "commands":[
        eve_command("world.speak","ghostlight.world_speak.v1", &["text"], "WorldKernel"),
        eve_command("world.assess","ghostlight.player_action_assess.v1", &["description","intended_effect"], "WorldKernel"),
        eve_command("world.attempt","ghostlight.player_action_attempt.v1", &[], "WorldKernel"),
        eve_command("world.wait","ghostlight.world_wait.v1", &["minutes"], "WorldKernel"),
        eve_command("governance.cells.propose","ghostlight.cell_budget_proposal.v1", &["active_cell_budget"], "WorldKernel"),
        eve_command("governance.cells.approve","ghostlight.cell_budget_approval.v1", &[], "WorldKernel"),
        eve_command("governance.time.propose","ghostlight.time_advance_proposal.v1", &["time_advance_minutes"], "WorldKernel"),
        eve_command("governance.time.approve","ghostlight.time_advance_approval.v1", &[], "WorldKernel"),
        eve_command("governance.travel.propose","ghostlight.group_travel_proposal.v1", &["destination_location_id"], "WorldKernel"),
        eve_command("governance.travel.approve","ghostlight.group_travel_approval.v1", &[], "WorldKernel"),
        eve_command("world.destination.compile","ghostlight.destination_compile.v1", &["destination"], "WorldCompiler"),
        eve_command("world.destination.approve","ghostlight.destination_approval.v1", &[], "WorldKernel"),
        eve_command("campaign.entry","ghostlight.campaign_entry.v1", &[], "ghostlight.account_preferences.v1"),
        eve_command("campaign.contract_review.begin","ghostlight.contract_review_begin.v1", &[], "SessionZeroKernel"),
        eve_command("app.auth.logout","ghostlight.app_logout.v1", &[], "ghostlight.app_session.v1")
      ]
    })
}

fn local_draft(name: &str, value_kind: &str) -> Value {
    json!({"targetProp":"value","pointerId":format!("draft:{name}"),"sourceId":"renderer","schemaId":"gamecult.eve.local_draft.v1","routeKind":"local","bindingName":name,"documentId":"ghostlight.play.drafts","fieldPath":name,"valueKind":value_kind,"accessMode":"local-draft","authority":"renderer-ephemeral"})
}

fn command_control(
    id: &str,
    label: &str,
    command: &str,
    action: Value,
    bindings: &[&str],
) -> Value {
    let mut action = action.as_object().cloned().unwrap_or_default();
    action.insert("command".into(), Value::String(command.into()));
    json!({"id":id,"kind":"control.button","props":{"label":label,"command":command,"action":action,"captureBindings":bindings},"children":[]})
}

fn eve_command(command: &str, payload_schema: &str, bindings: &[&str], authority: &str) -> Value {
    json!({"schema":"gamecult.eve.command.v1","command":command,"payloadSchema":payload_schema,"captureBindings":bindings,"transport":"https-json","authority":authority})
}

fn story_nodes(campaign: &Campaign) -> Vec<Value> {
    let mut entries = Vec::new();
    for (index, turn) in campaign.transcript.iter().enumerate() {
        entries.push((
            turn.revision,
            index,
            json!({"id":format!("turn-{}-{}-{}",turn.revision,index,turn.speaker),"kind":"text","props":{"value":format!("{}: {}",turn.speaker,turn.text)},"children":[]}),
        ));
    }
    entries.sort_by_key(|(revision, index, _)| (*revision, *index));
    entries.into_iter().map(|(_, _, node)| node).collect()
}

pub fn operator_surface(
    campaign: &Campaign,
    evidence: &[VaultEvidenceReceipt],
    commits: &[WorldCommitReceipt],
    stages: &[ModelStageReceipt],
    strategic_ticks: &[StrategicTickReceipt],
    gestalt_receipts: &[GestaltMaterializationReceipt],
    rejected: &[RejectedProposalReceipt],
    resolution_plans: &[crate::domain::ResolutionPlanReceipt],
    cell_appraisals: &[crate::domain::CellAppraisal],
    activity_outcomes: &[crate::domain::StrategicActivityOutcome],
    resolution_controls: &[crate::domain::ResolutionControlReceipt],
    live_turn_pressure: usize,
) -> Value {
    let interface_version = campaign_interface_version(campaign);
    let cover_text = campaign
        .resolution_cover
        .as_ref()
        .map(|cover| {
            cover
                .cells
                .iter()
                .map(|cell| {
                    format!(
                        "{} {:?} [{}] loss {:.3} debt focus {} — {}",
                        cell.id,
                        cell.mode,
                        cell.subject_ids
                            .iter()
                            .cloned()
                            .collect::<Vec<_>>()
                            .join(", "),
                        cell.merge_loss.total,
                        cell.detail_focus_subject_id.as_deref().unwrap_or("none"),
                        cell.rationale
                    )
                })
                .collect::<Vec<_>>()
                .join("\n")
        })
        .unwrap_or_else(|| "No cover has committed yet.".into());
    let graph_text = campaign
        .agency_relations
        .values()
        .filter(|relation| relation.active)
        .map(|relation| {
            format!(
                "{} --{:?}/{}--> {}",
                relation.from_subject_id, relation.kind, relation.strength, relation.to_subject_id
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    let provider_attempt_count = stages
        .iter()
        .map(|stage| stage.provider_attempts.len())
        .sum::<usize>();
    let retry_count = provider_attempt_count.saturating_sub(stages.len());
    let token_usage = stages
        .iter()
        .flat_map(|stage| &stage.provider_attempts)
        .filter_map(|attempt| attempt.token_usage.as_ref())
        .fold(
            crate::model::ModelTokenUsage::default(),
            |mut total, usage| {
                total.prompt_tokens = total.prompt_tokens.saturating_add(usage.prompt_tokens);
                total.completion_tokens = total
                    .completion_tokens
                    .saturating_add(usage.completion_tokens);
                total.total_tokens = total.total_tokens.saturating_add(usage.total_tokens);
                total.prompt_cache_hit_tokens = total
                    .prompt_cache_hit_tokens
                    .saturating_add(usage.prompt_cache_hit_tokens);
                total.prompt_cache_miss_tokens = total
                    .prompt_cache_miss_tokens
                    .saturating_add(usage.prompt_cache_miss_tokens);
                total.reasoning_tokens = total
                    .reasoning_tokens
                    .saturating_add(usage.reasoning_tokens);
                total
            },
        );
    let typed = json!({
    "campaign": campaign,
    "evidence": evidence,
    "commit_receipts": commits,
    "model_stage_receipts": stages,
    "strategic_ticks": strategic_ticks,
    "gestalt_materialization_receipts": gestalt_receipts,
    "rejected_proposals": rejected,
    "resolution_plan_receipts": resolution_plans,
    "cell_appraisals": cell_appraisals,
    "strategic_activity_outcomes": activity_outcomes,
    "resolution_control_receipts": resolution_controls,
    "scheduler": {"live_turn_pressure": live_turn_pressure}
    });
    let cache_observed_tokens = token_usage
        .prompt_cache_hit_tokens
        .saturating_add(token_usage.prompt_cache_miss_tokens);
    let cache_hit_percent = if cache_observed_tokens == 0 {
        0
    } else {
        token_usage.prompt_cache_hit_tokens.saturating_mul(100) / cache_observed_tokens
    };
    json!({
      "type":"surface-state", "schema":"gamecult.eve.surface.v1", "providerId":"gamecult.ghostlight.dungeon",
      "providerKind":"narrative.simulation.operator", "title":format!("{} operator",campaign.name), "version":interface_version,
      "surface":{"id":format!("ghostlight.operator.{}",campaign.id),"root":{"id":"dungeon.operator.root","kind":"surface","props":{},"children":[
        {"id":"dungeon.operator.status","kind":"card","props":{"title":format!("Revision {} · {} model stages / {} provider attempts · {} tokens · {} rejected proposals",campaign.revision,stages.len(),provider_attempt_count,token_usage.total_tokens,rejected.len())},"children":[{"id":"dungeon.operator.usage","kind":"text","props":{"value":format!("Prompt {} (cache hit {}, miss {}, {}%) · completion {} · reasoning {} · retries {}",token_usage.prompt_tokens,token_usage.prompt_cache_hit_tokens,token_usage.prompt_cache_miss_tokens,cache_hit_percent,token_usage.completion_tokens,token_usage.reasoning_tokens,retry_count)},"children":[]}]},
        {"id":"dungeon.operator.cover","kind":"card","props":{"title":format!("Agency cover · epoch {} · budget {}",campaign.resolution_policy.resolution_epoch,campaign.resolution_policy.active_cell_budget)},"children":[{"id":"dungeon.operator.cover.text","kind":"text","props":{"value":cover_text},"children":[]}]},
        {"id":"dungeon.operator.graph","kind":"card","props":{"title":format!("Agency graph · {} profiles · {} relations",campaign.agency_profiles.len(),campaign.agency_relations.len())},"children":[{"id":"dungeon.operator.graph.text","kind":"text","props":{"value":graph_text},"children":[]}]},
        {"id":"dungeon.operator.outcomes","kind":"card","props":{"title":format!("Strategic activity outcomes · {}",activity_outcomes.len())},"children":[{"id":"dungeon.operator.outcomes.text","kind":"text","props":{"value":activity_outcomes.iter().rev().take(16).map(|outcome|format!("{} {:?} — {}",outcome.source_subject_id,outcome.band,outcome.summary)).collect::<Vec<_>>().join("\n")},"children":[]}]},
        {"id":"dungeon.operator.typed","kind":"code","props":{"language":"json","value":serde_json::to_string_pretty(&typed).unwrap_or_else(|_| "operator projection failed".into())},"children":[]}
      ]},"styles":{"tokens":{"colorBackground":"#0c1110","colorPanel":"#17201d","colorText":"#e8e1cf","colorMuted":"#9aa69f","colorAccent":"#d49b58"}}},
      "commands":[]
    })
}

fn join(values: &std::collections::BTreeSet<String>) -> String {
    if values.is_empty() {
        "none".into()
    } else {
        values.iter().cloned().collect::<Vec<_>>().join(", ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::NarrativeTurn;
    use chrono::Utc;

    #[test]
    fn story_is_chronological_and_uses_only_committed_turns() {
        let mut campaign = crate::resolution::tests::campaign(1, 1);
        campaign.transcript = vec![
            NarrativeTurn {
                revision: 1,
                at: Utc::now(),
                speaker: "player".into(),
                text: "I ask the question.".into(),
                persona_response_actor_ids: Default::default(),
            },
            NarrativeTurn {
                revision: 2,
                at: Utc::now(),
                speaker: "world".into(),
                text: "raw outcome".into(),
                persona_response_actor_ids: Default::default(),
            },
            NarrativeTurn {
                revision: 3,
                at: Utc::now(),
                speaker: "npc".into(),
                text: "I answer directly.".into(),
                persona_response_actor_ids: Default::default(),
            },
        ];
        let story = story_nodes(&campaign);
        let values = story
            .iter()
            .map(|node| node["props"]["value"].as_str().unwrap())
            .collect::<Vec<_>>();
        assert_eq!(
            values,
            vec![
                "player: I ask the question.",
                "world: raw outcome",
                "npc: I answer directly."
            ]
        );
        assert!(values.iter().all(|value| !value.contains("narration")));
    }

    #[test]
    fn player_surface_projects_canonical_player_and_location_ids() {
        let mut campaign = crate::resolution::tests::campaign(1, 1);
        let mut player = campaign.actors.remove("player").unwrap();
        player.id = "pilot-nyx".into();
        player
            .relationships
            .insert("faction-0000".into(), "owes me safe passage".into());
        campaign.player_actor_id = player.id.clone();
        campaign.actors.insert(player.id.clone(), player);
        campaign.actors.insert(
            "local-witness".into(),
            crate::domain::ActorState {
                id: "local-witness".into(),
                name: "Local Witness".into(),
                location_id: "center".into(),
                capabilities: Default::default(),
                knowledge: Default::default(),
                equipment: Default::default(),
                conditions: Default::default(),
                obligations: Default::default(),
                relationships: Default::default(),
                goals: vec![],
                memories: vec![],
            },
        );
        campaign.actors.insert(
            "remote-witness".into(),
            crate::domain::ActorState {
                id: "remote-witness".into(),
                name: "SECRET_REMOTE_WITNESS".into(),
                location_id: "remote".into(),
                capabilities: Default::default(),
                knowledge: Default::default(),
                equipment: Default::default(),
                conditions: Default::default(),
                obligations: Default::default(),
                relationships: Default::default(),
                goals: vec![],
                memories: vec![],
            },
        );

        let surface = player_surface(&campaign);

        assert_eq!(surface["viewer_actor_id"], "pilot-nyx");
        assert_eq!(surface["player_location_id"], "center");
        assert!(
            serde_json::to_string(&surface)
                .unwrap()
                .contains("Relationships: Faction 0: owes me safe passage")
        );
        let encoded = serde_json::to_string(&surface).unwrap();
        assert!(encoded.contains("Present characters: Local Witness"));
        assert!(!encoded.contains("SECRET_REMOTE_WITNESS"));
    }

    #[test]
    fn player_surface_cannot_leak_canonical_remote_pressure() {
        let mut campaign = crate::resolution::tests::campaign(1, 1);
        campaign
            .institutions
            .get_mut("faction-0000")
            .unwrap()
            .posture = "SECRET_REMOTE_COUP_POSTURE".into();
        campaign.clocks.insert(
            "sealed-investigation".into(),
            crate::domain::WorldClock {
                id: "sealed-investigation".into(),
                label: "SECRET_INVESTIGATION_CLOCK".into(),
                progress: 3,
                threshold: 4,
                consequence: "SECRET_ARREST_PLAN".into(),
            },
        );
        campaign.news.push(crate::domain::NewsIssue {
            id: "news:public".into(),
            at: Utc::now(),
            channel: "public bulletin".into(),
            headline: "The public ferry is delayed.".into(),
            event_ids: vec!["event:public".into()],
            reliability: "direct institutional channel".into(),
        });
        campaign
            .agency_profiles
            .get_mut("player")
            .unwrap()
            .information_channels
            .insert("public bulletin".into());
        campaign.resolution_pins.insert(
            "secret-pin".into(),
            crate::domain::ResolutionPin {
                schema: "ghostlight.resolution_pin.v1".into(),
                id: "secret-pin".into(),
                kind: crate::domain::ResolutionPinKind::KeepSeparate,
                subject_ids: std::collections::BTreeSet::from(["SECRET_REMOTE_SUBJECT".into()]),
                reason: "SECRET_OPERATOR_REASON".into(),
                created_world_revision: 0,
            },
        );

        let surface = player_surface(&campaign);
        let encoded = serde_json::to_string(&surface).unwrap();

        assert!(!encoded.contains("SECRET_REMOTE_COUP_POSTURE"));
        assert!(!encoded.contains("SECRET_INVESTIGATION_CLOCK"));
        assert!(!encoded.contains("SECRET_ARREST_PLAN"));
        assert!(!encoded.contains("SECRET_REMOTE_SUBJECT"));
        assert!(!encoded.contains("SECRET_OPERATOR_REASON"));
        assert_eq!(surface["resolution"]["pin_count"], 1);
        assert!(encoded.contains("The public ferry is delayed."));
    }

    #[test]
    fn actor_knowledge_cannot_masquerade_as_news_channel_access() {
        let mut campaign = crate::resolution::tests::campaign(1, 1);
        campaign.news.push(crate::domain::NewsIssue {
            id: "news:sealed".into(),
            at: Utc::now(),
            channel: "sealed command wire".into(),
            headline: "SECRET_COMMAND_MOVEMENT".into(),
            event_ids: vec!["event:sealed".into()],
            reliability: "direct institutional channel".into(),
        });
        campaign
            .actors
            .get_mut("player")
            .unwrap()
            .knowledge
            .insert("sealed command wire".into());

        let encoded = serde_json::to_string(&player_surface(&campaign)).unwrap();

        assert!(!encoded.contains("SECRET_COMMAND_MOVEMENT"));
    }

    #[test]
    fn player_surface_projects_directly_perceived_events_but_not_remote_secrets() {
        let mut campaign = crate::resolution::tests::campaign(1, 1);
        campaign.events.extend([
            crate::domain::Event {
                id: "event:local".into(),
                at: Utc::now(),
                kind: "local-change".into(),
                summary: "The junction workers reopen the water line.".into(),
                actor_ids: vec![],
                institution_ids: vec![],
                gestalt_ids: vec!["faction-0000".into()],
                location_ids: vec!["center".into()],
                public_channels: vec![],
            },
            crate::domain::Event {
                id: "event:remote".into(),
                at: Utc::now(),
                kind: "remote-secret".into(),
                summary: "SECRET_REMOTE_EVENT".into(),
                actor_ids: vec![],
                institution_ids: vec!["faction-0000".into()],
                gestalt_ids: vec![],
                location_ids: vec!["remote".into()],
                public_channels: vec![],
            },
        ]);

        let encoded = serde_json::to_string(&player_surface(&campaign)).unwrap();

        assert!(encoded.contains("The junction workers reopen the water line."));
        assert!(!encoded.contains("SECRET_REMOTE_EVENT"));
    }

    #[test]
    fn player_surface_news_omits_the_viewers_own_turn_and_travel_noise() {
        let mut campaign = crate::resolution::tests::campaign(1, 1);
        campaign.events.extend([
            crate::domain::Event {
                id: "event:own-activity".into(),
                at: Utc::now(),
                kind: "actor_activity".into(),
                summary: "PLAYER_PRIVATE_ACTIVITY_SUMMARY".into(),
                actor_ids: vec!["player".into()],
                institution_ids: vec![],
                gestalt_ids: vec![],
                location_ids: vec!["center".into()],
                public_channels: vec![],
            },
            crate::domain::Event {
                id: "event:reaction".into(),
                at: Utc::now(),
                kind: "reaction_wave".into(),
                summary: "PLAYER_SPEECH_DUPLICATE".into(),
                actor_ids: vec!["faction-0000".into()],
                institution_ids: vec![],
                gestalt_ids: vec![],
                location_ids: vec!["center".into()],
                public_channels: vec![],
            },
            crate::domain::Event {
                id: "event:travel".into(),
                at: Utc::now(),
                kind: "group_travel".into(),
                summary: "PLAYER_TRAVEL_DUPLICATE".into(),
                actor_ids: vec!["player".into()],
                institution_ids: vec![],
                gestalt_ids: vec![],
                location_ids: vec!["center".into()],
                public_channels: vec![],
            },
            crate::domain::Event {
                id: "event:other".into(),
                at: Utc::now(),
                kind: "institution_action".into(),
                summary: "The local watch reinforces the gate.".into(),
                actor_ids: vec![],
                institution_ids: vec!["faction-0000".into()],
                gestalt_ids: vec![],
                location_ids: vec!["center".into()],
                public_channels: vec![],
            },
        ]);

        let encoded = serde_json::to_string(&player_surface(&campaign)).unwrap();

        assert!(!encoded.contains("PLAYER_PRIVATE_ACTIVITY_SUMMARY"));
        assert!(!encoded.contains("PLAYER_SPEECH_DUPLICATE"));
        assert!(!encoded.contains("PLAYER_TRAVEL_DUPLICATE"));
        assert!(encoded.contains("The local watch reinforces the gate."));
    }

    #[test]
    fn player_surface_offers_only_local_population_fission_targets() {
        let mut campaign = crate::resolution::tests::campaign(1, 1);
        for (id, name, location) in [
            ("local-neighbors", "Local neighbors", "center"),
            ("remote-cell", "SECRET_REMOTE_POPULATION", "far-district"),
        ] {
            campaign.gestalts.insert(
                id.into(),
                crate::domain::GestaltPersonaState {
                    schema: "ghostlight.gestalt_persona_state.v1".into(),
                    id: id.into(),
                    name: name.into(),
                    version: 0,
                    home_location_id: location.into(),
                    shared_capabilities: Default::default(),
                    shared_knowledge: Default::default(),
                    resources: Default::default(),
                    goals: vec![],
                    pressures: vec![],
                },
            );
        }
        crate::resolution::ensure_agency_profiles(&mut campaign);

        let encoded = serde_json::to_string(&player_surface(&campaign)).unwrap();

        assert!(encoded.contains("Local neighbors"));
        assert!(!encoded.contains("SECRET_REMOTE_POPULATION"));
    }

    #[test]
    fn player_surface_uses_bound_eve_controls_for_time_travel_and_destination_compilation() {
        let mut campaign = crate::resolution::tests::campaign(1, 1);
        campaign.locations.insert(
            "harbor".into(),
            crate::domain::Location {
                id: "harbor".into(),
                name: "Harbor".into(),
                container_id: None,
                routes: Default::default(),
                persistent_features: vec![],
            },
        );
        campaign.locations.get_mut("center").unwrap().routes.insert(
            "harbor".into(),
            crate::domain::Route {
                destination_id: "harbor".into(),
                distance: "nearby".into(),
                travel_minutes: 20,
            },
        );

        let encoded = serde_json::to_string(&player_surface(&campaign)).unwrap();
        for binding in [
            "time_advance_minutes",
            "destination_location_id",
            "destination",
        ] {
            assert!(encoded.contains(&format!("\"bindingName\":\"{binding}\"")));
        }
        assert!(encoded.contains("governance.travel.propose"));
        assert!(encoded.contains("\"value\":\"harbor\""));
        assert!(encoded.contains("world.destination.compile"));
        assert!(encoded.contains("campaign.entry"));
        assert!(!encoded.contains("payload.fields"));
        assert!(!encoded.contains("\"kind\":\"form\""));
    }
}
