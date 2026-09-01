use crate::{
    agent::{ModelAgentSpec, ModelAgentTool, ModelAgentToolContext, ModelAgentToolOutcome},
    domain::{Campaign, CausalFollowThroughAssignment, Event, ResolutionCover, SimulationCell},
    model::{MODEL_BALANCED, ModelPort, ModelStageReceipt},
    resolution::cell_action_limit,
};
use anyhow::{Result, anyhow};
use async_trait::async_trait;
use schemars::{JsonSchema, schema_for};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

pub const NEMESIS_STAGE: &str = "nemesis_attention_agent_action";
const HISTORY_PAGE_SIZE: usize = 24;
const INITIAL_EVENT_WINDOW: usize = 24;
const CURRENT_ANCHOR_PAGE_SIZE: usize = 24;
const RESPONDER_PAGE_SIZE: usize = 12;
const MAX_FOLLOW_THROUGH_STEPS: usize = 6;

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct CausalAssignmentDraft {
    anchor_reference: String,
    responder_subject_id: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct CausalFollowThroughAction {
    command: CausalFollowThroughCommand,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum CausalFollowThroughCommand {
    InspectWorldHistory {
        cursor: usize,
    },
    InspectSubjectHistory {
        responder_subject_id: String,
        cursor: usize,
    },
    InspectCurrentAnchors {
        cursor: usize,
    },
    InspectAnchorResponders {
        anchor_reference: String,
        cursor: usize,
    },
    Submit {
        assignments: Vec<CausalAssignmentDraft>,
    },
}

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum CausalFollowThroughFinding {
    HistoryPage {
        scope: String,
        anchors: Vec<CausalAnchorSummaryView>,
        next_cursor: Option<usize>,
    },
    ResponderPage {
        anchor: CausalAnchorSummaryView,
        responders: Vec<CausalResponderView>,
        next_cursor: Option<usize>,
    },
    InvalidAgenda {
        diagnostic: String,
    },
}

#[derive(Clone, Debug, Serialize)]
struct CausalAnchorSummaryView {
    anchor_reference: String,
    kind: String,
    account: String,
    originating_subject_ids: Vec<String>,
    eligible_responder_count: usize,
}

#[derive(Clone, Debug, Serialize)]
struct CausalResponderView {
    subject_id: String,
    name: String,
    subject_kind: String,
    stakes: Vec<String>,
    cell_id: String,
    cell_decision_slot_limit: usize,
}

#[derive(Clone, Debug)]
struct CausalAnchorCandidate {
    anchor_reference: String,
    kind: String,
    account: String,
    originating_subject_ids: Vec<String>,
    eligible_responder_ids: BTreeSet<String>,
}

impl CausalAnchorCandidate {
    fn summary(&self) -> CausalAnchorSummaryView {
        CausalAnchorSummaryView {
            anchor_reference: self.anchor_reference.clone(),
            kind: self.kind.clone(),
            account: self.account.clone(),
            originating_subject_ids: self.originating_subject_ids.clone(),
            eligible_responder_count: self.eligible_responder_ids.len(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct CausalFollowThroughAdmission {
    pub schema: String,
    pub expected_revision: u64,
    pub snapshot_binding: String,
    pub assignment_batch_digest: String,
    pub assignments: Vec<CausalFollowThroughAssignment>,
}

pub struct CausalFollowThroughProposal {
    pub admission: CausalFollowThroughAdmission,
    pub receipts: Vec<ModelStageReceipt>,
}

struct CausalFollowThroughTool<'a> {
    campaign: &'a Campaign,
    cover: &'a ResolutionCover,
    all_public_events: Vec<&'a Event>,
    current_anchors: Vec<CausalAnchorCandidate>,
    discovered: BTreeMap<String, CausalAnchorCandidate>,
    responder_views: BTreeMap<String, CausalResponderView>,
}

impl CausalFollowThroughTool<'_> {
    fn inspect_world_page(&mut self, cursor: usize) -> Result<CausalFollowThroughFinding> {
        if cursor > self.all_public_events.len() {
            return Err(anyhow!(
                "world-history cursor is outside the committed event ledger"
            ));
        }
        let end = cursor
            .saturating_add(HISTORY_PAGE_SIZE)
            .min(self.all_public_events.len());
        let events = self.all_public_events[cursor..end].to_vec();
        let anchors = self.discover_events(events)?;
        Ok(CausalFollowThroughFinding::HistoryPage {
            scope: "committed public world history".into(),
            anchors,
            next_cursor: (end < self.all_public_events.len()).then_some(end),
        })
    }

    fn inspect_subject_page(
        &mut self,
        responder_subject_id: &str,
        cursor: usize,
    ) -> Result<CausalFollowThroughFinding> {
        if !self.responder_views.contains_key(responder_subject_id) {
            return Err(anyhow!(
                "subject-history query names an ineligible responder"
            ));
        }
        let perceived = self
            .all_public_events
            .iter()
            .copied()
            .filter(|event| subject_perceives_event(self.campaign, responder_subject_id, event))
            .collect::<Vec<_>>();
        if cursor > perceived.len() {
            return Err(anyhow!(
                "subject-history cursor is outside that subject's committed history"
            ));
        }
        let end = cursor
            .saturating_add(HISTORY_PAGE_SIZE)
            .min(perceived.len());
        let anchors = self.discover_events(perceived[cursor..end].to_vec())?;
        Ok(CausalFollowThroughFinding::HistoryPage {
            scope: format!("committed history perceived by {responder_subject_id}"),
            anchors,
            next_cursor: (end < perceived.len()).then_some(end),
        })
    }

    fn discover_events(&mut self, events: Vec<&Event>) -> Result<Vec<CausalAnchorSummaryView>> {
        let mut views = Vec::new();
        for event in events {
            let candidate =
                event_anchor_candidate(self.campaign, self.cover, event, &self.responder_views)?;
            if candidate.eligible_responder_ids.is_empty() {
                continue;
            }
            views.push(candidate.summary());
            self.discovered
                .insert(candidate.anchor_reference.clone(), candidate);
        }
        Ok(views)
    }

    fn inspect_current_anchor_page(&mut self, cursor: usize) -> Result<CausalFollowThroughFinding> {
        if cursor > self.current_anchors.len() {
            return Err(anyhow!(
                "current-anchor cursor is outside the durable pressure ledger"
            ));
        }
        let end = cursor
            .saturating_add(CURRENT_ANCHOR_PAGE_SIZE)
            .min(self.current_anchors.len());
        let candidates = self.current_anchors[cursor..end].to_vec();
        let mut anchors = Vec::with_capacity(candidates.len());
        for candidate in candidates {
            anchors.push(candidate.summary());
            self.discovered
                .insert(candidate.anchor_reference.clone(), candidate);
        }
        Ok(CausalFollowThroughFinding::HistoryPage {
            scope: "current durable pressure and clocks".into(),
            anchors,
            next_cursor: (end < self.current_anchors.len()).then_some(end),
        })
    }

    fn inspect_anchor_responder_page(
        &self,
        anchor_reference: &str,
        cursor: usize,
    ) -> Result<CausalFollowThroughFinding> {
        let candidate = self
            .discovered
            .get(anchor_reference)
            .ok_or_else(|| anyhow!("responder query names an anchor the agent did not inspect"))?;
        let responder_ids = candidate.eligible_responder_ids.iter().collect::<Vec<_>>();
        if cursor > responder_ids.len() {
            return Err(anyhow!(
                "anchor-responder cursor is outside the eligible responder list"
            ));
        }
        let end = cursor
            .saturating_add(RESPONDER_PAGE_SIZE)
            .min(responder_ids.len());
        let responders = responder_ids[cursor..end]
            .iter()
            .filter_map(|subject_id| self.responder_views.get(*subject_id).cloned())
            .collect();
        Ok(CausalFollowThroughFinding::ResponderPage {
            anchor: candidate.summary(),
            responders,
            next_cursor: (end < responder_ids.len()).then_some(end),
        })
    }

    fn validate_and_bind(
        &self,
        drafts: Vec<CausalAssignmentDraft>,
    ) -> Result<Vec<CausalFollowThroughAssignment>> {
        let maximum = self
            .cover
            .cells
            .iter()
            .map(cell_action_limit)
            .sum::<usize>();
        if drafts.len() > maximum {
            return Err(anyhow!(
                "causal response agenda uses {} decision slots but the cover permits {maximum}",
                drafts.len()
            ));
        }
        let mut pairs = BTreeSet::new();
        let mut responders = BTreeSet::new();
        let mut per_cell = BTreeMap::<String, usize>::new();
        let mut assignments = Vec::with_capacity(drafts.len());
        for draft in drafts {
            let anchor = self
                .discovered
                .get(&draft.anchor_reference)
                .ok_or_else(|| {
                    anyhow!("causal agenda names an anchor the agent did not inspect")
                })?;
            if !anchor
                .eligible_responder_ids
                .contains(&draft.responder_subject_id)
            {
                return Err(anyhow!(
                    "{} cannot perceive or own a response to {}",
                    draft.responder_subject_id,
                    draft.anchor_reference
                ));
            }
            if !pairs.insert((
                draft.anchor_reference.clone(),
                draft.responder_subject_id.clone(),
            )) || !responders.insert(draft.responder_subject_id.clone())
            {
                return Err(anyhow!(
                    "each causal responder may own exactly one distinct anchor in a wave"
                ));
            }
            let cell = responder_cell(self.campaign, self.cover, &draft.responder_subject_id)
                .ok_or_else(|| anyhow!("causal responder is outside the exact cover"))?;
            let count = per_cell.entry(cell.id.clone()).or_default();
            *count += 1;
            if *count > cell_action_limit(cell) {
                return Err(anyhow!(
                    "causal assignments exceed decision quota for cell {}",
                    cell.id
                ));
            }
            assignments.push(CausalFollowThroughAssignment {
                anchor_reference: draft.anchor_reference,
                responder_subject_id: draft.responder_subject_id,
            });
        }
        for cell in &self.cover.cells {
            let assigned = assignments
                .iter()
                .filter(|assignment| {
                    responder_cell(self.campaign, self.cover, &assignment.responder_subject_id)
                        .is_some_and(|owner| owner.id == cell.id)
                })
                .collect::<Vec<_>>();
            let represented_subjects = assigned
                .iter()
                .map(|assignment| {
                    assignment
                        .responder_subject_id
                        .strip_prefix("member:")
                        .and_then(|member_id| self.campaign.gestalt_members.get(member_id))
                        .map(|member| member.gestalt_id.clone())
                        .unwrap_or_else(|| assignment.responder_subject_id.clone())
                })
                .collect::<BTreeSet<_>>();
            if cell.detail_focus_subject_id.as_ref().is_some_and(|focus| {
                !represented_subjects.contains(focus) && assigned.len() >= cell_action_limit(cell)
            }) {
                return Err(anyhow!(
                    "causal assignments consume every decision slot in cell {} without its mandatory detail-focus subject",
                    cell.id
                ));
            }
        }
        assignments.sort_by(|left, right| {
            left.responder_subject_id
                .cmp(&right.responder_subject_id)
                .then_with(|| left.anchor_reference.cmp(&right.anchor_reference))
        });
        Ok(assignments)
    }
}

#[async_trait]
impl ModelAgentTool for CausalFollowThroughTool<'_> {
    type Action = CausalFollowThroughAction;
    type Output = CausalFollowThroughAdmission;
    type Finding = CausalFollowThroughFinding;

    fn action_schema(&self) -> std::result::Result<serde_json::Value, String> {
        let mut schema = serde_json::to_value(schema_for!(CausalFollowThroughAction))
            .map_err(|error| error.to_string())?;
        crate::model_connector::project_strict_responses_schema(&mut schema)
            .map_err(|error| error.to_string())?;
        constrain_action_schema(
            &mut schema,
            self.discovered.keys().cloned().collect(),
            self.responder_views.keys().cloned().collect(),
        )
        .map_err(|error| error.to_string())?;
        Ok(schema)
    }

    async fn invoke(
        &mut self,
        action: Self::Action,
        context: &ModelAgentToolContext,
    ) -> ModelAgentToolOutcome<Self::Output, Self::Finding> {
        match action.command {
            CausalFollowThroughCommand::InspectWorldHistory { cursor } => {
                match self.inspect_world_page(cursor) {
                    Ok(observation) => ModelAgentToolOutcome::Continue {
                        observation,
                        receipts: Vec::new(),
                    },
                    Err(error) => ModelAgentToolOutcome::Rejected {
                        finding: CausalFollowThroughFinding::InvalidAgenda {
                            diagnostic: error.to_string(),
                        },
                        receipts: Vec::new(),
                    },
                }
            }
            CausalFollowThroughCommand::InspectSubjectHistory {
                responder_subject_id,
                cursor,
            } => match self.inspect_subject_page(&responder_subject_id, cursor) {
                Ok(observation) => ModelAgentToolOutcome::Continue {
                    observation,
                    receipts: Vec::new(),
                },
                Err(error) => ModelAgentToolOutcome::Rejected {
                    finding: CausalFollowThroughFinding::InvalidAgenda {
                        diagnostic: error.to_string(),
                    },
                    receipts: Vec::new(),
                },
            },
            CausalFollowThroughCommand::InspectCurrentAnchors { cursor } => {
                match self.inspect_current_anchor_page(cursor) {
                    Ok(observation) => ModelAgentToolOutcome::Continue {
                        observation,
                        receipts: Vec::new(),
                    },
                    Err(error) => ModelAgentToolOutcome::Rejected {
                        finding: CausalFollowThroughFinding::InvalidAgenda {
                            diagnostic: error.to_string(),
                        },
                        receipts: Vec::new(),
                    },
                }
            }
            CausalFollowThroughCommand::InspectAnchorResponders {
                anchor_reference,
                cursor,
            } => match self.inspect_anchor_responder_page(&anchor_reference, cursor) {
                Ok(observation) => ModelAgentToolOutcome::Continue {
                    observation,
                    receipts: Vec::new(),
                },
                Err(error) => ModelAgentToolOutcome::Rejected {
                    finding: CausalFollowThroughFinding::InvalidAgenda {
                        diagnostic: error.to_string(),
                    },
                    receipts: Vec::new(),
                },
            },
            CausalFollowThroughCommand::Submit { assignments } => {
                match self.validate_and_bind(assignments) {
                    Ok(assignments) => {
                        let Some(mut accepted_receipt) = context.current_model_receipt.clone()
                        else {
                            return ModelAgentToolOutcome::Failed {
                                message: "causal follow-through tool lacks its model receipt"
                                    .into(),
                                receipts: Vec::new(),
                            };
                        };
                        let snapshot_binding =
                            match causal_follow_through_snapshot(self.campaign, self.cover) {
                                Ok(binding) => binding,
                                Err(error) => {
                                    return ModelAgentToolOutcome::Failed {
                                        message: error.to_string(),
                                        receipts: Vec::new(),
                                    };
                                }
                            };
                        let assignment_batch_digest =
                            match causal_assignment_batch_digest(&assignments) {
                                Ok(digest) => digest,
                                Err(error) => {
                                    return ModelAgentToolOutcome::Failed {
                                        message: error.to_string(),
                                        receipts: Vec::new(),
                                    };
                                }
                            };
                        accepted_receipt.rebind_snapshot(format!(
                            "{snapshot_binding}:causal-agenda:{assignment_batch_digest}"
                        ));
                        ModelAgentToolOutcome::Accepted {
                            output: CausalFollowThroughAdmission {
                                schema: "ghostlight.causal_follow_through_admission.v1".into(),
                                expected_revision: self.campaign.revision,
                                snapshot_binding,
                                assignment_batch_digest,
                                assignments,
                            },
                            receipts: vec![accepted_receipt],
                        }
                    }
                    Err(error) => ModelAgentToolOutcome::Rejected {
                        finding: CausalFollowThroughFinding::InvalidAgenda {
                            diagnostic: error.to_string(),
                        },
                        receipts: Vec::new(),
                    },
                }
            }
        }
    }
}

pub async fn propose_causal_follow_through(
    model: &dyn ModelPort,
    campaign: &Campaign,
    cover: &ResolutionCover,
) -> std::result::Result<Option<CausalFollowThroughProposal>, crate::agent::ModelAgentFailure> {
    let responder_views = responder_views(campaign, cover);
    let current_anchors = current_anchor_candidates(campaign, cover, &responder_views);
    let all_public_events = campaign
        .events
        .iter()
        .rev()
        .filter(|event| !event.public_channels.is_empty())
        .collect::<Vec<_>>();
    let mut tool = CausalFollowThroughTool {
        campaign,
        cover,
        all_public_events,
        current_anchors,
        discovered: BTreeMap::new(),
        responder_views,
    };
    let initial_events = tool
        .all_public_events
        .iter()
        .copied()
        .take(INITIAL_EVENT_WINDOW)
        .collect::<Vec<_>>();
    let initial_event_anchors =
        tool.discover_events(initial_events)
            .map_err(|error| crate::agent::ModelAgentFailure {
                message: error.to_string(),
                receipts: Vec::new(),
            })?;
    let initial_current_anchors =
        tool.inspect_current_anchor_page(0)
            .map_err(|error| crate::agent::ModelAgentFailure {
                message: error.to_string(),
                receipts: Vec::new(),
            })?;
    if tool.discovered.is_empty() {
        return Ok(None);
    }
    let snapshot_binding = causal_follow_through_snapshot(campaign, cover).map_err(|error| {
        crate::agent::ModelAgentFailure {
            message: error.to_string(),
            receipts: Vec::new(),
        }
    })?;
    let instructions = format!(
        "You are Nemesis, Ghostlight's causal attention agent. Select who receives a decision window because committed world pressure now demands an answer. You never decide their action or its outcome. Bind only exact inspected anchors to exact eligible autonomous responders. Prefer consequential acts, declared crises, active pressure, public humiliation, material loss, contested authority, and promises whose costs should now land. When one public detonation creates incompatible stakes, assign distinct rival responders to the same anchor so their independent Persona cells can make competing countermoves. Do not assign the player. Do not assign one subject twice. Stay within the cover quotas. Inspect a chosen anchor's eligible responders before assigning it. You may page through current pressure/clocks, older world history, one subject's perceived history, or one anchor's eligible responders; the committed ledger is not limited to the initial viewport. An empty assignment list is a valid judgment when nothing currently warrants a response window. Submit the smallest agenda likely to produce genuine causal follow-through. The deterministic tool alone admits it.\n\nCOVER SUMMARY:\n{}\n\nINITIAL RECENT PUBLIC ANCHORS:\n{}\n\nINITIAL CURRENT PRESSURE/CLOCK PAGE:\n{}",
        serde_json::json!({
            "cell_count":cover.cells.len(),
            "eligible_responder_count":tool.responder_views.len(),
            "maximum_decision_slots":cover.cells.iter().map(cell_action_limit).sum::<usize>(),
        }),
        serde_json::to_string(&initial_event_anchors).unwrap_or_default(),
        serde_json::to_string(&initial_current_anchors).unwrap_or_default(),
    );
    let spec = ModelAgentSpec {
        stage: NEMESIS_STAGE.into(),
        model: MODEL_BALANCED.into(),
        snapshot_binding,
        instructions,
        source_receipt_ids: campaign.branch_origin.evidence_receipt_ids.clone(),
        temperature: Some(0.15),
        max_output_tokens: Some(2_400),
        max_steps: MAX_FOLLOW_THROUGH_STEPS,
    };
    let run = crate::agent::run_model_agent(model, &spec, &mut tool).await?;
    Ok(Some(CausalFollowThroughProposal {
        admission: run.output,
        receipts: run.receipts,
    }))
}

pub fn validate_causal_follow_through(campaign: &Campaign, cover: &ResolutionCover) -> Result<()> {
    if cover.causal_follow_through.is_empty() {
        return Ok(());
    }
    let responder_views = responder_views(campaign, cover);
    let mut catalog = current_anchor_candidates(campaign, cover, &responder_views)
        .into_iter()
        .map(|candidate| (candidate.anchor_reference.clone(), candidate))
        .collect::<BTreeMap<_, _>>();
    for event in campaign
        .events
        .iter()
        .filter(|event| !event.public_channels.is_empty())
    {
        let candidate = event_anchor_candidate(campaign, cover, event, &responder_views)?;
        catalog.insert(candidate.anchor_reference.clone(), candidate);
    }
    let tool = CausalFollowThroughTool {
        campaign,
        cover,
        all_public_events: Vec::new(),
        current_anchors: Vec::new(),
        discovered: catalog,
        responder_views,
    };
    let drafts = cover
        .causal_follow_through
        .iter()
        .map(|assignment| CausalAssignmentDraft {
            anchor_reference: assignment.anchor_reference.clone(),
            responder_subject_id: assignment.responder_subject_id.clone(),
        })
        .collect::<Vec<_>>();
    let rebound = tool.validate_and_bind(drafts)?;
    if rebound != cover.causal_follow_through {
        return Err(anyhow!(
            "causal follow-through agenda is not canonically ordered"
        ));
    }
    Ok(())
}

pub fn causal_anchor_summary(campaign: &Campaign, anchor_reference: &str) -> Option<String> {
    if let Some(event_id) = anchor_reference.strip_prefix("event:") {
        return campaign
            .events
            .iter()
            .find(|event| event.id == event_id)
            .map(|event| event.summary.clone());
    }
    for gestalt in campaign.gestalts.values() {
        for pressure in &gestalt.pressures {
            if pressure_anchor_reference(&gestalt.id, pressure) == anchor_reference {
                return Some(format!(
                    "Current unresolved pressure on {}: {pressure}",
                    gestalt.name
                ));
            }
        }
    }
    for clock in campaign.clocks.values() {
        if clock_anchor_reference(&clock.id, clock.progress) == anchor_reference {
            return Some(format!(
                "{} is at {}/{}; declared consequence: {}",
                clock.label, clock.progress, clock.threshold, clock.consequence
            ));
        }
    }
    None
}

pub fn responder_cell<'a>(
    campaign: &Campaign,
    cover: &'a ResolutionCover,
    responder_subject_id: &str,
) -> Option<&'a SimulationCell> {
    let owning_subject = responder_subject_id
        .strip_prefix("member:")
        .and_then(|member_id| campaign.gestalt_members.get(member_id))
        .and_then(|member| {
            member
                .materialized_actor_id
                .is_none()
                .then_some(member.gestalt_id.as_str())
        })
        .unwrap_or(responder_subject_id);
    cover
        .cells
        .iter()
        .find(|cell| cell.subject_ids.contains(owning_subject))
}

pub fn assignments_for_cell<'a>(
    campaign: &Campaign,
    cover: &'a ResolutionCover,
    cell: &SimulationCell,
) -> Vec<&'a CausalFollowThroughAssignment> {
    cover
        .causal_follow_through
        .iter()
        .filter(|assignment| {
            responder_cell(campaign, cover, &assignment.responder_subject_id)
                .is_some_and(|owner| owner.id == cell.id)
        })
        .collect()
}

pub fn causal_follow_through_snapshot(
    campaign: &Campaign,
    cover: &ResolutionCover,
) -> Result<String> {
    crate::legacy_transition::digest_serializable(&serde_json::json!({
        "campaign_id":campaign.id,
        "world_revision":campaign.revision,
        "resolution_epoch":campaign.resolution_policy.resolution_epoch,
        "events":campaign.events,
        "clocks":campaign.clocks,
        "gestalt_pressures":campaign.gestalts.iter().map(|(id, gestalt)| (id, &gestalt.pressures)).collect::<BTreeMap<_, _>>(),
        "agency_profiles":campaign.agency_profiles,
        "agency_relations":campaign.agency_relations,
        "nemesis_attention_history":campaign.nemesis_attention_history,
        "cells":cover.cells,
        "demand":cover.demand,
    }))
}

pub fn causal_assignment_batch_digest(
    assignments: &[CausalFollowThroughAssignment],
) -> Result<String> {
    crate::legacy_transition::digest_serializable(&serde_json::json!({
        "schema":"ghostlight.causal_follow_through_assignment_batch.v1",
        "assignments":assignments,
    }))
}

pub fn nemesis_admission_binding(campaign: &Campaign, cover: &ResolutionCover) -> Result<String> {
    Ok(format!(
        "{}:causal-agenda:{}",
        causal_follow_through_snapshot(campaign, cover)?,
        causal_assignment_batch_digest(&cover.causal_follow_through)?
    ))
}

fn constrain_action_schema(
    schema: &mut serde_json::Value,
    anchors: Vec<String>,
    responders: Vec<String>,
) -> Result<()> {
    let text = serde_json::to_string(schema)?;
    let mut value: serde_json::Value = serde_json::from_str(&text)?;
    replace_property_schema(
        &mut value,
        "anchor_reference",
        serde_json::json!({"type":"string","enum":anchors}),
    );
    replace_property_schema(
        &mut value,
        "responder_subject_id",
        serde_json::json!({"type":"string","enum":responders}),
    );
    *schema = value;
    Ok(())
}

fn replace_property_schema(
    value: &mut serde_json::Value,
    name: &str,
    replacement: serde_json::Value,
) {
    match value {
        serde_json::Value::Object(object) => {
            if let Some(properties) = object
                .get_mut("properties")
                .and_then(serde_json::Value::as_object_mut)
                && properties.contains_key(name)
            {
                properties.insert(name.into(), replacement.clone());
            }
            for child in object.values_mut() {
                replace_property_schema(child, name, replacement.clone());
            }
        }
        serde_json::Value::Array(values) => {
            for child in values {
                replace_property_schema(child, name, replacement.clone());
            }
        }
        _ => {}
    }
}

fn responder_views(
    campaign: &Campaign,
    cover: &ResolutionCover,
) -> BTreeMap<String, CausalResponderView> {
    let mut views = BTreeMap::new();
    for profile in campaign
        .agency_profiles
        .values()
        .filter(|profile| profile.active_leaf && profile.simulation_eligible)
        .filter(|profile| profile.subject_id != campaign.player_actor_id)
    {
        let Some(cell) = responder_cell(campaign, cover, &profile.subject_id) else {
            continue;
        };
        views.insert(
            profile.subject_id.clone(),
            CausalResponderView {
                subject_id: profile.subject_id.clone(),
                name: subject_name(campaign, &profile.subject_id),
                subject_kind: format!("{:?}", profile.subject_kind),
                stakes: subject_stakes(campaign, &profile.subject_id),
                cell_id: cell.id.clone(),
                cell_decision_slot_limit: cell_action_limit(cell),
            },
        );
    }
    for member in campaign
        .gestalt_members
        .values()
        .filter(|member| member.materialized_actor_id.is_none())
    {
        let subject_id = crate::domain::gestalt_member_subject_id(&member.id);
        let Some(cell) = responder_cell(campaign, cover, &subject_id) else {
            continue;
        };
        if campaign
            .agency_profiles
            .get(&member.gestalt_id)
            .is_some_and(|profile| profile.simulation_eligible)
        {
            views.insert(
                subject_id.clone(),
                CausalResponderView {
                    subject_id,
                    name: member.name.clone(),
                    subject_kind: "GestaltMember".into(),
                    stakes: member
                        .goals
                        .iter()
                        .chain(&member.obligations)
                        .chain(&member.conditions)
                        .cloned()
                        .collect(),
                    cell_id: cell.id.clone(),
                    cell_decision_slot_limit: cell_action_limit(cell),
                },
            );
        }
    }
    views
}

fn event_anchor_candidate(
    campaign: &Campaign,
    _cover: &ResolutionCover,
    event: &Event,
    responder_views: &BTreeMap<String, CausalResponderView>,
) -> Result<CausalAnchorCandidate> {
    let eligible_responder_ids = responder_views
        .keys()
        .filter(|subject_id| {
            subject_perceives_event(campaign, subject_id, event)
                && !nemesis_already_served(campaign, &format!("event:{}", event.id), subject_id)
        })
        .cloned()
        .collect::<BTreeSet<_>>();
    let mut originating_subject_ids = event
        .actor_ids
        .iter()
        .chain(&event.institution_ids)
        .chain(&event.gestalt_ids)
        .cloned()
        .collect::<Vec<_>>();
    originating_subject_ids.sort();
    originating_subject_ids.dedup();
    Ok(CausalAnchorCandidate {
        anchor_reference: format!("event:{}", event.id),
        kind: event.kind.clone(),
        account: event.summary.clone(),
        originating_subject_ids,
        eligible_responder_ids,
    })
}

fn current_anchor_candidates(
    campaign: &Campaign,
    _cover: &ResolutionCover,
    responder_views: &BTreeMap<String, CausalResponderView>,
) -> Vec<CausalAnchorCandidate> {
    let mut anchors = Vec::new();
    for gestalt in campaign.gestalts.values() {
        for pressure in &gestalt.pressures {
            let eligible_responder_ids = responder_views
                .keys()
                .filter(|subject_id| {
                    (subject_id.as_str() == gestalt.id
                        || subject_id
                            .strip_prefix("member:")
                            .and_then(|member_id| campaign.gestalt_members.get(member_id))
                            .is_some_and(|member| member.gestalt_id == gestalt.id))
                        && !nemesis_already_served(
                            campaign,
                            &pressure_anchor_reference(&gestalt.id, pressure),
                            subject_id,
                        )
                })
                .cloned()
                .collect::<BTreeSet<_>>();
            if eligible_responder_ids.is_empty() {
                continue;
            }
            anchors.push(CausalAnchorCandidate {
                anchor_reference: pressure_anchor_reference(&gestalt.id, pressure),
                kind: "gestalt_pressure".into(),
                account: format!(
                    "Current unresolved pressure on {}: {pressure}",
                    gestalt.name
                ),
                originating_subject_ids: vec![gestalt.id.clone()],
                eligible_responder_ids,
            });
        }
    }
    for clock in campaign.clocks.values().filter(|clock| clock.progress > 0) {
        let synthetic = Event {
            id: format!("clock:projection:{}", clock.id),
            at: campaign.world_time,
            kind: "world_clock".into(),
            summary: clock.consequence.clone(),
            actor_ids: clock.consequence_scope.actor_ids.clone(),
            institution_ids: clock.consequence_scope.institution_ids.clone(),
            gestalt_ids: clock.consequence_scope.gestalt_ids.clone(),
            location_ids: clock.consequence_scope.location_ids.clone(),
            public_channels: clock.consequence_scope.public_channels.clone(),
        };
        let eligible_responder_ids = responder_views
            .keys()
            .filter(|subject_id| {
                subject_perceives_event(campaign, subject_id, &synthetic)
                    && !nemesis_already_served(
                        campaign,
                        &clock_anchor_reference(&clock.id, clock.progress),
                        subject_id,
                    )
            })
            .cloned()
            .collect::<BTreeSet<_>>();
        if eligible_responder_ids.is_empty() {
            continue;
        }
        anchors.push(CausalAnchorCandidate {
            anchor_reference: clock_anchor_reference(&clock.id, clock.progress),
            kind: "world_clock".into(),
            account: format!(
                "{} is at {}/{}; declared consequence: {}",
                clock.label, clock.progress, clock.threshold, clock.consequence
            ),
            originating_subject_ids: Vec::new(),
            eligible_responder_ids,
        });
    }
    anchors
}

fn pressure_anchor_reference(gestalt_id: &str, pressure: &str) -> String {
    let digest = format!(
        "{:x}",
        Sha256::digest(format!("{gestalt_id}\0{pressure}").as_bytes())
    );
    format!("pressure:{gestalt_id}:{}", &digest[..16])
}

fn clock_anchor_reference(clock_id: &str, progress: u8) -> String {
    format!("clock:{clock_id}:progress:{progress}")
}

fn nemesis_already_served(campaign: &Campaign, anchor: &str, responder: &str) -> bool {
    campaign
        .nemesis_attention_history
        .iter()
        .any(|record| record.anchor_reference == anchor && record.responder_subject_id == responder)
}

fn subject_perceives_event(campaign: &Campaign, subject_id: &str, event: &Event) -> bool {
    if event.actor_ids.iter().any(|id| id == subject_id)
        || event.institution_ids.iter().any(|id| id == subject_id)
        || event.gestalt_ids.iter().any(|id| id == subject_id)
    {
        return true;
    }
    if let Some(member_id) = subject_id.strip_prefix("member:")
        && let Some(member) = campaign.gestalt_members.get(member_id)
        && let Some(gestalt) = campaign.gestalts.get(&member.gestalt_id)
    {
        let location = member
            .last_location_id
            .as_deref()
            .unwrap_or(&gestalt.home_location_id);
        let channels = campaign
            .agency_profiles
            .get(&member.gestalt_id)
            .map(|profile| &profile.information_channels);
        return event.location_ids.iter().any(|id| id == location)
            || channels.is_some_and(|channels| {
                event
                    .public_channels
                    .iter()
                    .any(|channel| channels.contains(channel))
            });
    }
    let Some(profile) = campaign.agency_profiles.get(subject_id) else {
        return false;
    };
    event
        .location_ids
        .iter()
        .any(|location| profile.location_ids.contains(location))
        || event
            .public_channels
            .iter()
            .any(|channel| profile.information_channels.contains(channel))
}

fn subject_name(campaign: &Campaign, subject_id: &str) -> String {
    campaign
        .actors
        .get(subject_id)
        .map(|actor| actor.name.clone())
        .or_else(|| {
            campaign
                .institutions
                .get(subject_id)
                .map(|institution| institution.name.clone())
        })
        .or_else(|| {
            campaign
                .gestalts
                .get(subject_id)
                .map(|gestalt| gestalt.name.clone())
        })
        .or_else(|| {
            subject_id
                .strip_prefix("member:")
                .and_then(|member_id| campaign.gestalt_members.get(member_id))
                .map(|member| member.name.clone())
        })
        .unwrap_or_else(|| subject_id.into())
}

fn subject_stakes(campaign: &Campaign, subject_id: &str) -> Vec<String> {
    if let Some(actor) = campaign.actors.get(subject_id) {
        return actor
            .goals
            .iter()
            .chain(&actor.obligations)
            .chain(&actor.conditions)
            .cloned()
            .collect();
    }
    if let Some(institution) = campaign.institutions.get(subject_id) {
        return institution
            .goals
            .iter()
            .cloned()
            .chain(std::iter::once(institution.posture.clone()))
            .collect();
    }
    if let Some(gestalt) = campaign.gestalts.get(subject_id) {
        return gestalt
            .goals
            .iter()
            .chain(&gestalt.pressures)
            .cloned()
            .collect();
    }
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        domain::{ActorState, Event, GestaltMemberDelta},
        model::ModelStageRequest,
        resolution::{default_demand, ensure_agency_profiles, plan_cover},
    };
    use anyhow::Result;
    use async_trait::async_trait;
    use std::collections::{BTreeMap, BTreeSet, VecDeque};
    use std::sync::Mutex;

    struct ScriptedModel {
        responses: Mutex<VecDeque<String>>,
        requests: Mutex<Vec<ModelStageRequest>>,
    }

    #[async_trait]
    impl ModelPort for ScriptedModel {
        async fn run(&self, request: &ModelStageRequest) -> Result<String> {
            self.requests.lock().unwrap().push(request.clone());
            self.responses
                .lock()
                .unwrap()
                .pop_front()
                .ok_or_else(|| anyhow!("follow-through fixture exhausted"))
        }

        fn provider(&self) -> &'static str {
            "follow-through-fixture"
        }
    }

    fn campaign_and_cover() -> (Campaign, ResolutionCover) {
        let mut campaign = crate::kernel::tests::campaign();
        campaign.actors.insert(
            "npc".into(),
            ActorState {
                id: "npc".into(),
                name: "Nara Venn".into(),
                location_id: "room".into(),
                capabilities: BTreeSet::from(["publish a reply".into()]),
                knowledge: BTreeSet::new(),
                equipment: BTreeSet::new(),
                conditions: BTreeSet::new(),
                obligations: BTreeSet::from(["answer public accusations".into()]),
                relationships: BTreeMap::new(),
                goals: vec!["retain the ward's confidence".into()],
                memories: Vec::new(),
            },
        );
        ensure_agency_profiles(&mut campaign);
        campaign
            .agency_profiles
            .get_mut("npc")
            .unwrap()
            .information_channels
            .insert("ward broadsheet".into());
        campaign.resolution_policy.active_cell_budget = 1;
        campaign.events.push(Event {
            id: "old".into(),
            at: campaign.world_time,
            kind: "institution_action".into(),
            summary: "The Thorn Bench accuses Nara Venn of hiding the winter tally.".into(),
            actor_ids: Vec::new(),
            institution_ids: Vec::new(),
            gestalt_ids: Vec::new(),
            location_ids: vec!["room".into()],
            public_channels: vec!["ward broadsheet".into()],
        });
        for index in 0..29 {
            campaign.events.push(Event {
                id: format!("new-{index:02}"),
                at: campaign.world_time,
                kind: "public_notice".into(),
                summary: format!("Routine notice {index} is posted in the ward."),
                actor_ids: Vec::new(),
                institution_ids: Vec::new(),
                gestalt_ids: Vec::new(),
                location_ids: vec!["room".into()],
                public_channels: vec!["ward broadsheet".into()],
            });
        }
        let cover = plan_cover(
            &campaign,
            default_demand(&campaign, "exercise full committed history"),
        )
        .unwrap();
        (campaign, cover)
    }

    #[tokio::test]
    async fn nemesis_can_retrieve_and_bind_an_anchor_beyond_the_initial_viewport() {
        let (campaign, cover) = campaign_and_cover();
        let model = ScriptedModel {
            responses: Mutex::new(VecDeque::from([
                serde_json::json!({
                    "command":{
                        "kind":"inspect_world_history",
                        "cursor":24
                    }
                })
                .to_string(),
                serde_json::json!({
                    "command":{
                        "kind":"inspect_anchor_responders",
                        "anchor_reference":"event:old",
                        "cursor":0
                    }
                })
                .to_string(),
                serde_json::json!({
                    "command":{
                        "kind":"submit",
                        "assignments":[{
                            "anchor_reference":"event:old",
                            "responder_subject_id":"npc"
                        }]
                    }
                })
                .to_string(),
            ])),
            requests: Mutex::new(Vec::new()),
        };

        let proposal = propose_causal_follow_through(&model, &campaign, &cover)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            proposal.admission.assignments,
            [CausalFollowThroughAssignment {
                anchor_reference: "event:old".into(),
                responder_subject_id: "npc".into(),
            }]
        );
        assert_eq!(proposal.receipts.len(), 4);
        assert!(
            proposal.receipts[1]
                .source_receipt_ids
                .contains(&proposal.receipts[0].storage_key().to_owned())
        );
        assert!(
            proposal
                .receipts
                .last()
                .unwrap()
                .snapshot_binding
                .contains("causal-agenda")
        );
        let requests = model.requests.lock().unwrap();
        assert!(requests[1].lived_stream.contains("event:old"));
        assert!(requests[1].lived_stream.contains("next_cursor"));
        assert!(requests[2].lived_stream.contains("Nara Venn"));
        assert!(
            requests[2]
                .lived_stream
                .contains("cell_decision_slot_limit")
        );
    }

    #[test]
    fn admitted_agenda_is_scheduler_only_and_excludes_the_player() {
        let (campaign, mut cover) = campaign_and_cover();
        cover.causal_follow_through = vec![CausalFollowThroughAssignment {
            anchor_reference: "event:old".into(),
            responder_subject_id: "npc".into(),
        }];
        validate_causal_follow_through(&campaign, &cover).unwrap();
        let cell = responder_cell(&campaign, &cover, "npc").unwrap();
        assert_eq!(assignments_for_cell(&campaign, &cover, cell).len(), 1);
        assert_eq!(
            causal_anchor_summary(&campaign, "event:old").as_deref(),
            Some("The Thorn Bench accuses Nara Venn of hiding the winter tally.")
        );

        cover.causal_follow_through[0].responder_subject_id = "player".into();
        assert!(validate_causal_follow_through(&campaign, &cover).is_err());
    }

    #[test]
    fn materialized_member_responders_use_their_actor_cell() {
        let (mut campaign, mut cover) = campaign_and_cover();
        let member = |id: &str, materialized_actor_id: Option<&str>| GestaltMemberDelta {
            schema: "ghostlight.gestalt_member_delta.v1".into(),
            id: id.into(),
            gestalt_id: "household".into(),
            version: 0,
            name: id.into(),
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
            materialized_actor_id: materialized_actor_id.map(str::to_owned),
            last_relevant_revision: 0,
            relevance_lease_until_revision: 0,
        };
        campaign
            .gestalt_members
            .insert("awake".into(), member("awake", Some("member:awake")));
        campaign
            .gestalt_members
            .insert("folded".into(), member("folded", None));

        let mut actor_cell = cover.cells[0].clone();
        actor_cell.id = "actor-cell".into();
        actor_cell.subject_ids = BTreeSet::from(["member:awake".into()]);
        let mut gestalt_cell = cover.cells[0].clone();
        gestalt_cell.id = "gestalt-cell".into();
        gestalt_cell.subject_ids = BTreeSet::from(["household".into()]);
        cover.cells = vec![actor_cell, gestalt_cell];

        assert_eq!(
            responder_cell(&campaign, &cover, "member:awake").map(|cell| cell.id.as_str()),
            Some("actor-cell")
        );
        assert_eq!(
            responder_cell(&campaign, &cover, "member:folded").map(|cell| cell.id.as_str()),
            Some("gestalt-cell")
        );
    }

    #[tokio::test]
    async fn nemesis_may_decide_that_no_subject_needs_attention_now() {
        let (campaign, cover) = campaign_and_cover();
        let model = ScriptedModel {
            responses: Mutex::new(VecDeque::from([serde_json::json!({
                "command":{"kind":"submit","assignments":[]}
            })
            .to_string()])),
            requests: Mutex::new(Vec::new()),
        };

        let proposal = propose_causal_follow_through(&model, &campaign, &cover)
            .await
            .unwrap()
            .unwrap();
        assert!(proposal.admission.assignments.is_empty());
        assert_eq!(proposal.receipts.len(), 2);
        assert!(
            proposal
                .receipts
                .last()
                .unwrap()
                .snapshot_binding
                .contains("causal-agenda")
        );
    }

    #[tokio::test]
    async fn nemesis_initial_frame_does_not_multiply_anchors_by_responders() {
        let (mut campaign, mut cover) = campaign_and_cover();
        for index in 0..2_400 {
            let id = format!("scale-actor-{index:03}");
            let mut actor = campaign.actors["npc"].clone();
            actor.id = id.clone();
            actor.name = format!("Scale Actor {index:03}");
            actor.goals = vec![format!(
                "preserve constituency {index:03} through the current dispute"
            )];
            actor.obligations = BTreeSet::from([format!(
                "answer pressure {index:03} without abandoning the ward"
            )]);
            campaign.actors.insert(id.clone(), actor);
            let mut profile = campaign.agency_profiles["npc"].clone();
            profile.id = format!("agency:{id}");
            profile.subject_id = id.clone();
            campaign.agency_profiles.insert(id.clone(), profile);
            cover.cells[0].subject_ids.insert(id);
        }
        let model = ScriptedModel {
            responses: Mutex::new(VecDeque::from([serde_json::json!({
                "command":{"kind":"submit","assignments":[]}
            })
            .to_string()])),
            requests: Mutex::new(Vec::new()),
        };

        propose_causal_follow_through(&model, &campaign, &cover)
            .await
            .unwrap()
            .unwrap();

        let requests = model.requests.lock().unwrap();
        let request = &requests[0];
        let frame_chars = request.lived_stream.chars().count()
            + serde_json::to_string(&request.output_schema)
                .unwrap()
                .chars()
                .count();
        assert!(request.lived_stream.contains("eligible_responder_count"));
        assert!(request.lived_stream.contains("next_cursor"));
        assert!(!request.lived_stream.contains("Scale Actor 2399"));
        assert!(
            frame_chars < 500_000,
            "bounded Nemesis frame grew to {frame_chars} characters"
        );
    }

    #[test]
    fn current_pressure_projection_is_paged_without_losing_candidates() {
        let (campaign, cover) = campaign_and_cover();
        let responders = responder_views(&campaign, &cover);
        let current_anchors = (0..30)
            .map(|index| CausalAnchorCandidate {
                anchor_reference: format!("pressure:test:{index:02}"),
                kind: "gestalt_pressure".into(),
                account: format!("Pressure {index:02} remains unresolved."),
                originating_subject_ids: vec!["npc".into()],
                eligible_responder_ids: responders.keys().cloned().collect(),
            })
            .collect();
        let mut tool = CausalFollowThroughTool {
            campaign: &campaign,
            cover: &cover,
            all_public_events: Vec::new(),
            current_anchors,
            discovered: BTreeMap::new(),
            responder_views: responders,
        };

        let CausalFollowThroughFinding::HistoryPage {
            anchors,
            next_cursor,
            ..
        } = tool.inspect_current_anchor_page(0).unwrap()
        else {
            panic!("current anchors returned the wrong workbench view")
        };
        assert_eq!(anchors.len(), CURRENT_ANCHOR_PAGE_SIZE);
        assert_eq!(next_cursor, Some(CURRENT_ANCHOR_PAGE_SIZE));
        assert_eq!(tool.discovered.len(), CURRENT_ANCHOR_PAGE_SIZE);

        let CausalFollowThroughFinding::HistoryPage {
            anchors,
            next_cursor,
            ..
        } = tool
            .inspect_current_anchor_page(CURRENT_ANCHOR_PAGE_SIZE)
            .unwrap()
        else {
            panic!("current anchors returned the wrong workbench view")
        };
        assert_eq!(anchors.len(), 6);
        assert_eq!(next_cursor, None);
        assert_eq!(tool.discovered.len(), 30);
    }

    #[test]
    fn a_served_anchor_responder_pair_cannot_be_scheduled_again() {
        let (mut campaign, mut cover) = campaign_and_cover();
        campaign
            .nemesis_attention_history
            .push(crate::domain::NemesisAttentionRecord {
                anchor_reference: "event:old".into(),
                responder_subject_id: "npc".into(),
                served_world_revision: campaign.revision,
            });
        cover.causal_follow_through = vec![CausalFollowThroughAssignment {
            anchor_reference: "event:old".into(),
            responder_subject_id: "npc".into(),
        }];

        let error = validate_causal_follow_through(&campaign, &cover).unwrap_err();
        assert!(error.to_string().contains("cannot perceive or own"));
    }
}
