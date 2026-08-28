use crate::{
    agent::{ModelAgentSpec, ModelAgentTool, ModelAgentToolContext, ModelAgentToolOutcome},
    domain::{Campaign, Event, WorldEventScope},
    model::{MODEL_BALANCED, ModelPort, ModelStageReceipt},
};
use anyhow::{Result, anyhow};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use schemars::{JsonSchema, schema_for};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use uuid::Uuid;

pub const CLOCK_CONSEQUENCE_BINDING_STAGE: &str = "clock_consequence_binding_agent_action";

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ClockConsequenceBinding {
    pub clock_id: String,
    pub scope: WorldEventScope,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ClockConsequenceBindingAdmission {
    pub schema: String,
    pub campaign_id: Uuid,
    pub expected_revision: u64,
    pub snapshot_binding: String,
    pub binding_batch_digest: String,
    pub bindings: Vec<ClockConsequenceBinding>,
    pub accepted_model_receipt_id: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct ClockConsequenceBindingReceipt {
    pub schema: String,
    pub campaign_id: Uuid,
    pub previous_revision: u64,
    pub revision: u64,
    pub snapshot_binding: String,
    pub binding_batch_digest: String,
    pub bindings: Vec<ClockConsequenceBinding>,
    pub model_receipt_ids: Vec<String>,
    pub accepted_model_receipt_id: String,
    pub emitted_event_ids: Vec<String>,
    pub emitted_news_ids: Vec<String>,
    pub news_count_before: usize,
    pub next_wave_index: usize,
    pub committed_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct ClockConsequenceBindingAction {
    bindings: Vec<ClockConsequenceBinding>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(deny_unknown_fields)]
struct ClockConsequenceBindingFinding {
    diagnostic: String,
}

struct ClockConsequenceBindingTool<'a> {
    campaign: &'a Campaign,
    schema: serde_json::Value,
}

#[async_trait]
impl ModelAgentTool for ClockConsequenceBindingTool<'_> {
    type Action = ClockConsequenceBindingAction;
    type Output = ClockConsequenceBindingAdmission;
    type Finding = ClockConsequenceBindingFinding;

    fn action_schema(&self) -> std::result::Result<serde_json::Value, String> {
        Ok(self.schema.clone())
    }

    async fn invoke(
        &mut self,
        action: Self::Action,
        context: &ModelAgentToolContext,
    ) -> ModelAgentToolOutcome<Self::Output, Self::Finding> {
        match validate_clock_consequence_bindings(self.campaign, &action.bindings) {
            Ok(()) => {
                let Some(mut accepted_receipt) = context.current_model_receipt.clone() else {
                    return ModelAgentToolOutcome::Failed {
                        message: "clock binding tool lacks the current model receipt".into(),
                        receipts: Vec::new(),
                    };
                };
                let snapshot_binding = match clock_consequence_binding_snapshot(self.campaign) {
                    Ok(binding) => binding,
                    Err(error) => {
                        return ModelAgentToolOutcome::Failed {
                            message: error.to_string(),
                            receipts: Vec::new(),
                        };
                    }
                };
                let binding_batch_digest =
                    match clock_consequence_binding_batch_digest(self.campaign, &action.bindings) {
                        Ok(digest) => digest,
                        Err(error) => {
                            return ModelAgentToolOutcome::Failed {
                                message: error.to_string(),
                                receipts: Vec::new(),
                            };
                        }
                    };
                accepted_receipt.rebind_snapshot(clock_consequence_admission_binding(
                    &snapshot_binding,
                    &binding_batch_digest,
                ));
                let accepted_model_receipt_id = accepted_receipt.storage_key().to_owned();
                ModelAgentToolOutcome::Accepted {
                    output: ClockConsequenceBindingAdmission {
                        schema: "ghostlight.clock_consequence_binding_admission.v1".into(),
                        campaign_id: self.campaign.id,
                        expected_revision: self.campaign.revision,
                        snapshot_binding,
                        binding_batch_digest,
                        bindings: action.bindings,
                        accepted_model_receipt_id,
                    },
                    receipts: vec![accepted_receipt],
                }
            }
            Err(error) => ModelAgentToolOutcome::Rejected {
                finding: ClockConsequenceBindingFinding {
                    diagnostic: error.to_string(),
                },
                receipts: Vec::new(),
            },
        }
    }
}

pub fn clock_consequence_binding_snapshot(campaign: &Campaign) -> Result<String> {
    crate::legacy_transition::digest_serializable(&serde_json::json!({
        "campaign_id":campaign.id,
        "revision":campaign.revision,
        "clocks":campaign.clocks,
        "actor_ids":campaign.actors.keys().collect::<Vec<_>>(),
        "institution_ids":campaign.institutions.keys().collect::<Vec<_>>(),
        "gestalt_ids":campaign.gestalts.keys().collect::<Vec<_>>(),
        "locations":campaign.locations.iter().map(|(id, location)| serde_json::json!({
            "id":id,
            "name":location.name,
        })).collect::<Vec<_>>(),
        "public_channels":admitted_public_channels(campaign),
    }))
}

pub async fn propose_clock_consequence_bindings(
    model: &dyn ModelPort,
    campaign: &Campaign,
) -> std::result::Result<
    crate::agent::ModelAgentRun<ClockConsequenceBindingAdmission>,
    crate::agent::ModelAgentFailure,
> {
    let unbound = campaign
        .clocks
        .values()
        .filter(|clock| clock.consequence_scope.is_unbound())
        .collect::<Vec<_>>();
    if unbound.is_empty() {
        return Err(crate::agent::ModelAgentFailure {
            message: "clock binding agent was invoked without unbound clocks".into(),
            receipts: Vec::new(),
        });
    }
    let projection = serde_json::json!({
        "clocks":unbound,
        "actors":campaign.actors.values().filter(|actor| stable_simulation_actor(campaign, &actor.id)).map(|actor| serde_json::json!({
            "id":actor.id,
            "name":actor.name,
            "location_id":actor.location_id,
        })).collect::<Vec<_>>(),
        "institutions":campaign.institutions.values().filter(|institution| simulation_eligible_subject(campaign, &institution.id)).map(|institution| serde_json::json!({
            "id":institution.id,
            "name":institution.name,
            "posture":institution.posture,
        })).collect::<Vec<_>>(),
        "gestalts":campaign.gestalts.values().filter(|gestalt| simulation_eligible_subject(campaign, &gestalt.id)).map(|gestalt| serde_json::json!({
            "id":gestalt.id,
            "name":gestalt.name,
            "home_location_id":gestalt.home_location_id,
            "pressures":gestalt.pressures,
        })).collect::<Vec<_>>(),
        "locations":campaign.locations.values().map(|location| serde_json::json!({
            "id":location.id,
            "name":location.name,
        })).collect::<Vec<_>>(),
        "public_channels":admitted_public_channels(campaign),
    });
    let instructions = format!(
        "You are the autonomous clock-consequence binding worker. The world and each clock's consequence are frozen. Bind every supplied clock exactly once to the existing people, institutions, populations, places, and public information routes that would actually experience or learn its declared consequence. This is scope reconciliation, not invention: do not rewrite consequences, add entities, include the player-controlled subject, or indiscriminately select the whole world. Choose the smallest causally sufficient scope. A public material crisis should normally include at least one exact place, one affected or responsible subject, and one supplied public channel so subsequent autonomous cells can perceive and respond to it. Return the complete binding batch through the typed tool. The deterministic tool will reject unknown, duplicate, empty, or overbroad references.\n\nFROZEN WORLD PROJECTION:\n{}",
        serde_json::to_string(&projection).map_err(|error| crate::agent::ModelAgentFailure {
            message: error.to_string(),
            receipts: Vec::new(),
        })?
    );
    let mut schema =
        serde_json::to_value(schema_for!(ClockConsequenceBindingAction)).map_err(|error| {
            crate::agent::ModelAgentFailure {
                message: error.to_string(),
                receipts: Vec::new(),
            }
        })?;
    crate::model_connector::project_strict_responses_schema(&mut schema).map_err(|error| {
        crate::agent::ModelAgentFailure {
            message: error.to_string(),
            receipts: Vec::new(),
        }
    })?;
    let binding = clock_consequence_binding_snapshot(campaign).map_err(|error| {
        crate::agent::ModelAgentFailure {
            message: error.to_string(),
            receipts: Vec::new(),
        }
    })?;
    let spec = ModelAgentSpec {
        stage: CLOCK_CONSEQUENCE_BINDING_STAGE.into(),
        model: MODEL_BALANCED.into(),
        snapshot_binding: binding,
        instructions,
        source_receipt_ids: Vec::new(),
        temperature: Some(0.1),
        max_output_tokens: Some(3_000),
        max_steps: 3,
    };
    let mut tool = ClockConsequenceBindingTool { campaign, schema };
    crate::agent::run_model_agent(model, &spec, &mut tool).await
}

pub fn clock_consequence_binding_batch_digest(
    campaign: &Campaign,
    bindings: &[ClockConsequenceBinding],
) -> Result<String> {
    crate::legacy_transition::digest_serializable(&serde_json::json!({
        "schema":"ghostlight.clock_consequence_binding_batch.v1",
        "campaign_id":campaign.id,
        "expected_revision":campaign.revision,
        "bindings":bindings,
    }))
}

pub fn clock_consequence_admission_binding(
    snapshot_binding: &str,
    binding_batch_digest: &str,
) -> String {
    format!("{snapshot_binding}:clock-consequence-binding-batch:{binding_batch_digest}")
}

pub fn validate_clock_consequence_bindings(
    campaign: &Campaign,
    bindings: &[ClockConsequenceBinding],
) -> Result<()> {
    let expected = campaign
        .clocks
        .values()
        .filter(|clock| clock.consequence_scope.is_unbound())
        .map(|clock| clock.id.as_str())
        .collect::<BTreeSet<_>>();
    let supplied = bindings
        .iter()
        .map(|binding| binding.clock_id.as_str())
        .collect::<BTreeSet<_>>();
    if supplied.len() != bindings.len() || supplied != expected {
        return Err(anyhow!(
            "clock consequence bindings must name every unbound clock exactly once; expected {expected:?}, received {supplied:?}"
        ));
    }
    for binding in bindings {
        validate_clock_consequence_scope(campaign, &binding.scope)?;
    }
    Ok(())
}

pub fn validate_clock_consequence_scope(
    campaign: &Campaign,
    scope: &WorldEventScope,
) -> Result<()> {
    if scope.is_unbound() || scope.location_ids.is_empty() {
        return Err(anyhow!(
            "a clock consequence requires an exact affected location and observable scope"
        ));
    }
    if scope.actor_ids.is_empty()
        && scope.institution_ids.is_empty()
        && scope.gestalt_ids.is_empty()
    {
        return Err(anyhow!(
            "a clock consequence requires at least one affected actor, institution, or population"
        ));
    }
    for (kind, values, maximum) in [
        ("actor", &scope.actor_ids, 8usize),
        ("institution", &scope.institution_ids, 8usize),
        ("population", &scope.gestalt_ids, 8usize),
        ("location", &scope.location_ids, 8usize),
        ("public channel", &scope.public_channels, 8usize),
    ] {
        if values.len() > maximum || values.iter().collect::<BTreeSet<_>>().len() != values.len() {
            return Err(anyhow!(
                "clock consequence {kind} scope is duplicate or overbroad"
            ));
        }
    }
    if scope
        .actor_ids
        .iter()
        .any(|id| !stable_simulation_actor(campaign, id))
        || scope
            .institution_ids
            .iter()
            .any(|id| !simulation_eligible_subject(campaign, id))
        || scope
            .gestalt_ids
            .iter()
            .any(|id| !simulation_eligible_subject(campaign, id))
    {
        return Err(anyhow!(
            "clock consequence scope cannot bind an externally controlled or ephemeral subject"
        ));
    }
    if scope
        .actor_ids
        .iter()
        .any(|id| !campaign.actors.contains_key(id))
        || scope
            .institution_ids
            .iter()
            .any(|id| !campaign.institutions.contains_key(id))
        || scope
            .gestalt_ids
            .iter()
            .any(|id| !campaign.gestalts.contains_key(id))
        || scope
            .location_ids
            .iter()
            .any(|id| !campaign.locations.contains_key(id))
    {
        return Err(anyhow!(
            "clock consequence scope references an unknown subject or place"
        ));
    }
    let channels = admitted_public_channels(campaign);
    if scope
        .public_channels
        .iter()
        .any(|channel| !channels.contains(channel))
    {
        return Err(anyhow!(
            "clock consequence scope references an unadmitted public channel"
        ));
    }
    Ok(())
}

fn simulation_eligible_subject(campaign: &Campaign, subject_id: &str) -> bool {
    campaign
        .agency_profiles
        .get(subject_id)
        .is_none_or(|profile| profile.simulation_eligible)
}

fn stable_simulation_actor(campaign: &Campaign, actor_id: &str) -> bool {
    actor_id != campaign.player_actor_id
        && simulation_eligible_subject(campaign, actor_id)
        && !campaign
            .gestalt_members
            .values()
            .any(|member| member.materialized_actor_id.as_deref() == Some(actor_id))
}

pub fn apply_clock_consequence_bindings(
    campaign: &mut Campaign,
    bindings: &[ClockConsequenceBinding],
) -> Result<Vec<String>> {
    validate_clock_consequence_bindings(campaign, bindings)?;
    for binding in bindings {
        campaign
            .clocks
            .get_mut(&binding.clock_id)
            .expect("binding set was validated")
            .consequence_scope = binding.scope.clone();
    }
    materialize_due_clock_consequences(campaign)
}

/// Deterministically derives the once-only canonical event owned by a clock's
/// first threshold crossing. Callers invoke this after any admitted mutation
/// batch so every command path shares the same trigger behavior.
pub fn materialize_due_clock_consequences(campaign: &mut Campaign) -> Result<Vec<String>> {
    let mut events = Vec::new();
    for clock in campaign.clocks.values() {
        if clock.progress < clock.threshold || clock.consequence_scope.is_unbound() {
            continue;
        }
        validate_clock_consequence_scope(campaign, &clock.consequence_scope)?;
        let event_id = format!("clock-consequence:{}", clock.id);
        if campaign.events.iter().any(|event| event.id == event_id) {
            continue;
        }
        let event = Event {
            id: event_id.clone(),
            at: campaign.world_time,
            kind: "clock_consequence".into(),
            summary: clock.consequence.trim().to_owned(),
            actor_ids: clock.consequence_scope.actor_ids.clone(),
            institution_ids: clock.consequence_scope.institution_ids.clone(),
            gestalt_ids: clock.consequence_scope.gestalt_ids.clone(),
            location_ids: clock.consequence_scope.location_ids.clone(),
            public_channels: clock.consequence_scope.public_channels.clone(),
        };
        events.push(event);
    }
    let emitted = events
        .iter()
        .map(|event| event.id.clone())
        .collect::<Vec<_>>();
    for event in events {
        crate::domain::append_event_with_publications(campaign, event);
    }
    Ok(emitted)
}

fn admitted_public_channels(campaign: &Campaign) -> BTreeSet<String> {
    campaign
        .agency_profiles
        .values()
        .flat_map(|profile| profile.information_channels.iter().cloned())
        .chain(campaign.news.iter().map(|issue| issue.channel.clone()))
        .filter(|channel| crate::resolution::information_channel_is_concrete(channel))
        .collect()
}

pub fn validate_binding_receipts(
    campaign: &Campaign,
    admission: &ClockConsequenceBindingAdmission,
    receipts: &[ModelStageReceipt],
) -> Result<()> {
    let snapshot = clock_consequence_binding_snapshot(campaign)?;
    let binding_batch_digest =
        clock_consequence_binding_batch_digest(campaign, &admission.bindings)?;
    if admission.schema != "ghostlight.clock_consequence_binding_admission.v1"
        || admission.campaign_id != campaign.id
        || admission.expected_revision != campaign.revision
        || admission.snapshot_binding != snapshot
        || admission.binding_batch_digest != binding_batch_digest
    {
        return Err(anyhow!(
            "clock consequence binding admission does not match the frozen campaign and exact batch"
        ));
    }
    validate_clock_consequence_bindings(campaign, &admission.bindings)?;
    let expected_receipt_binding =
        clock_consequence_admission_binding(&snapshot, &binding_batch_digest);
    let accepted = receipts
        .iter()
        .find(|receipt| receipt.storage_key() == admission.accepted_model_receipt_id)
        .ok_or_else(|| anyhow!("clock consequence binding lacks its accepted model receipt"))?;
    let mut rebound = accepted.clone();
    rebound.rebind_snapshot(accepted.snapshot_binding.clone());
    if accepted.schema != "ghostlight.persona_stage_receipt.v1"
        || accepted.stage != CLOCK_CONSEQUENCE_BINDING_STAGE
        || accepted.snapshot_binding != expected_receipt_binding
        || accepted.validation_result != "valid"
        || accepted.local_validation_error.is_some()
        || rebound.storage_key() != accepted.storage_key()
    {
        return Err(anyhow!(
            "clock consequence binding accepted-model receipt is invalid or not bound to the exact batch"
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::{Digest, Sha256};

    #[test]
    fn binding_agent_schema_is_provider_strict() {
        let mut schema = serde_json::to_value(schema_for!(ClockConsequenceBindingAction)).unwrap();
        crate::model_connector::project_strict_responses_schema(&mut schema).unwrap();
    }

    fn bound_scope() -> WorldEventScope {
        WorldEventScope {
            actor_ids: Vec::new(),
            institution_ids: vec!["court".into()],
            gestalt_ids: Vec::new(),
            location_ids: vec!["room".into()],
            public_channels: vec!["court broadsheet".into()],
        }
    }

    fn accepted_admission(
        campaign: &Campaign,
        bindings: Vec<ClockConsequenceBinding>,
    ) -> (ClockConsequenceBindingAdmission, ModelStageReceipt) {
        let snapshot_binding = clock_consequence_binding_snapshot(campaign).unwrap();
        let binding_batch_digest =
            clock_consequence_binding_batch_digest(campaign, &bindings).unwrap();
        let mut receipt = ModelStageReceipt {
            schema: "ghostlight.persona_stage_receipt.v1".into(),
            receipt_hash: String::new(),
            provider: "fixture".into(),
            model: "fixture-terra".into(),
            stage: CLOCK_CONSEQUENCE_BINDING_STAGE.into(),
            snapshot_binding: String::new(),
            request_hash: format!("sha256:{:x}", Sha256::digest(b"request")),
            output_hash: format!("sha256:{:x}", Sha256::digest(b"output")),
            source_receipt_ids: Vec::new(),
            latency_ms: 1,
            validation_result: "valid".into(),
            local_validation_error: None,
            input_chars: 1,
            output_chars: 1,
            provider_attempts: Vec::new(),
        };
        receipt.rebind_snapshot(clock_consequence_admission_binding(
            &snapshot_binding,
            &binding_batch_digest,
        ));
        let admission = ClockConsequenceBindingAdmission {
            schema: "ghostlight.clock_consequence_binding_admission.v1".into(),
            campaign_id: campaign.id,
            expected_revision: campaign.revision,
            snapshot_binding,
            binding_batch_digest,
            bindings,
            accepted_model_receipt_id: receipt.storage_key().to_owned(),
        };
        (admission, receipt)
    }

    #[test]
    fn due_clock_emits_exactly_one_grounded_public_event() {
        let mut campaign = crate::kernel::tests::campaign();
        campaign.institutions.insert(
            "court".into(),
            crate::domain::InstitutionState {
                id: "court".into(),
                name: "Court".into(),
                resources: Vec::new(),
                goals: Vec::new(),
                posture: "watching".into(),
            },
        );
        crate::resolution::ensure_agency_profiles(&mut campaign);
        campaign
            .agency_profiles
            .get_mut("court")
            .unwrap()
            .information_channels
            .insert("court broadsheet".into());
        campaign.clocks.insert(
            "coup".into(),
            crate::domain::WorldClock {
                id: "coup".into(),
                label: "Coup".into(),
                progress: 3,
                threshold: 3,
                consequence: "The palace guard arrests the regent at breakfast.".into(),
                consequence_scope: bound_scope(),
            },
        );

        assert_eq!(
            materialize_due_clock_consequences(&mut campaign).unwrap(),
            vec!["clock-consequence:coup"]
        );
        assert!(
            materialize_due_clock_consequences(&mut campaign)
                .unwrap()
                .is_empty()
        );
        assert_eq!(
            campaign
                .events
                .iter()
                .filter(|event| event.kind == "clock_consequence")
                .count(),
            1
        );
        assert_eq!(
            campaign.news.last().unwrap().event_ids,
            ["clock-consequence:coup"]
        );
    }

    #[test]
    fn shared_mutation_application_owns_threshold_crossing() {
        let mut campaign = crate::kernel::tests::campaign();
        campaign.institutions.insert(
            "court".into(),
            crate::domain::InstitutionState {
                id: "court".into(),
                name: "Court".into(),
                resources: Vec::new(),
                goals: Vec::new(),
                posture: "watching".into(),
            },
        );
        crate::resolution::ensure_agency_profiles(&mut campaign);
        campaign
            .agency_profiles
            .get_mut("court")
            .unwrap()
            .information_channels
            .insert("court broadsheet".into());
        campaign.clocks.insert(
            "coup".into(),
            crate::domain::WorldClock {
                id: "coup".into(),
                label: "Coup".into(),
                progress: 0,
                threshold: 1,
                consequence: "The palace guard arrests the regent at breakfast.".into(),
                consequence_scope: bound_scope(),
            },
        );
        let transition = crate::legacy_transition::lower_strategic_wave(
            &campaign,
            &crate::domain::StrategicTickPlan::default(),
            "test-wave",
            chrono::Utc::now() + chrono::Duration::minutes(1),
        )
        .unwrap()
        .unwrap();

        let receipt = crate::legacy_transition::apply_lowered_transition(
            &mut campaign,
            &transition,
            chrono::Utc::now(),
        )
        .unwrap();

        assert_eq!(campaign.clocks["coup"].progress, 1);
        assert_eq!(receipt.derived_event_ids, ["clock-consequence:coup"]);
        assert!(campaign.events.iter().any(|event| {
            event.id == "clock-consequence:coup" && event.kind == "clock_consequence"
        }));
        assert!(
            campaign
                .news
                .iter()
                .any(|news| news.event_ids == ["clock-consequence:coup"])
        );
    }

    #[test]
    fn accepted_receipt_cannot_authorize_a_different_binding_batch() {
        let mut campaign = crate::kernel::tests::campaign();
        campaign.institutions.insert(
            "court".into(),
            crate::domain::InstitutionState {
                id: "court".into(),
                name: "Court".into(),
                resources: Vec::new(),
                goals: Vec::new(),
                posture: "watching".into(),
            },
        );
        crate::resolution::ensure_agency_profiles(&mut campaign);
        campaign
            .agency_profiles
            .get_mut("court")
            .unwrap()
            .information_channels
            .extend(["court broadsheet".into(), "court gazette".into()]);
        campaign.clocks.insert(
            "coup".into(),
            crate::domain::WorldClock {
                id: "coup".into(),
                label: "Coup".into(),
                progress: 0,
                threshold: 1,
                consequence: "The palace guard arrests the regent at breakfast.".into(),
                consequence_scope: WorldEventScope::default(),
            },
        );
        let bindings = vec![ClockConsequenceBinding {
            clock_id: "coup".into(),
            scope: bound_scope(),
        }];
        let (mut admission, receipt) = accepted_admission(&campaign, bindings);
        validate_binding_receipts(&campaign, &admission, std::slice::from_ref(&receipt)).unwrap();

        admission.bindings[0].scope.public_channels = vec!["court gazette".into()];
        assert!(
            validate_binding_receipts(&campaign, &admission, &[receipt])
                .unwrap_err()
                .to_string()
                .contains("exact batch")
        );
    }

    #[test]
    fn consequence_scope_rejects_externally_controlled_subjects() {
        let mut campaign = crate::kernel::tests::campaign();
        campaign.institutions.insert(
            "greathold".into(),
            crate::domain::InstitutionState {
                id: "greathold".into(),
                name: "Greathold".into(),
                resources: Vec::new(),
                goals: Vec::new(),
                posture: "player controlled".into(),
            },
        );
        crate::resolution::ensure_agency_profiles(&mut campaign);
        campaign
            .agency_profiles
            .get_mut("greathold")
            .unwrap()
            .simulation_eligible = false;
        let scope = WorldEventScope {
            actor_ids: Vec::new(),
            institution_ids: vec!["greathold".into()],
            gestalt_ids: Vec::new(),
            location_ids: vec!["room".into()],
            public_channels: Vec::new(),
        };

        assert!(
            validate_clock_consequence_scope(&campaign, &scope)
                .unwrap_err()
                .to_string()
                .contains("externally controlled")
        );
    }
}
