use crate::{
    agent::{
        ModelAgentProgress, ModelAgentSpec, ModelAgentTool, ModelAgentToolContext,
        ModelAgentToolOutcome,
    },
    domain::{Campaign, Event, NewsIssue, PublicEventAssertionStatus},
    model::{
        MODEL_BALANCED, MODEL_CAPABLE, ModelPort, ModelStageReceipt, ModelStageRequest,
        run_validated_stage,
    },
    persistence::CampaignStore,
};
use anyhow::{Result, anyhow};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use schemars::{JsonSchema, schema_for};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error as StdError,
    fmt,
};

const MAX_FRONT_PAGE_ARTICLES: usize = 6;
const MAX_PUBLIC_RECORD_QUERY_RESULTS: usize = 24;
const MAX_NARRATIVE_SELECTION_STEPS: usize = 8;
const GROUNDING_RECONCILIATION_ACTIONS_PER_ADVANCE: usize = 3;
const NEWSROOM_CONTRACT_VERSION: &str = "canopy-ledger-public-ledger-query.v4";
const EDITION_LABEL: &str = "Current Edition";
const ALLOWED_SECTIONS: [&str; 6] = [
    "Front Page",
    "Realm Affairs",
    "Courts & Councils",
    "Guilds & Trade",
    "Dispatches",
    "Comment",
];
const ALLOWED_BYLINES: [&str; 5] = [
    "By our own correspondent",
    "By the political editor",
    "By the trade correspondent",
    "Staff report",
    "Editorial",
];

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct WorldNewspaperIssue {
    pub schema: String,
    pub id: String,
    pub title: String,
    pub edition_label: String,
    pub at: DateTime<Utc>,
    pub source_world_revision: u64,
    pub lead_article_id: Option<String>,
    pub editorial_agenda: Option<WorldNewspaperEditorialAgenda>,
    pub articles: Vec<WorldNewspaperArticle>,
    pub editorial_receipt_ids: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct WorldNewspaperArticle {
    pub id: String,
    pub section: String,
    pub headline: String,
    pub deck: String,
    pub byline: String,
    pub dateline: Option<String>,
    pub paragraphs: Vec<String>,
    pub sources: Vec<WorldNewspaperSourceCitation>,
}

impl WorldNewspaperArticle {
    pub fn source_news_ids(&self) -> Vec<String> {
        self.sources
            .iter()
            .flat_map(|source| source.source_news_ids.iter().cloned())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }

    pub fn event_ids(&self) -> Vec<String> {
        self.sources
            .iter()
            .flat_map(|source| source.facts.iter())
            .flat_map(|fact| fact.event_ids.iter().cloned())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct WorldNewspaperSourceCitation {
    pub citation: String,
    pub source_news_ids: Vec<String>,
    pub source_channels: Vec<String>,
    pub source_reliability: Vec<String>,
    pub facts: Vec<WorldNewspaperSourceFact>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct WorldNewspaperSourceFact {
    pub event_ids: Vec<String>,
    pub account: String,
    pub assertion_status: WorldNewspaperAssertionStatus,
    pub named_people: Vec<WorldNewspaperNamedPerson>,
    pub institutions: Vec<String>,
    pub populations: Vec<String>,
    pub places: Vec<String>,
}

pub type WorldNewspaperAssertionStatus = PublicEventAssertionStatus;

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct WorldNewspaperNamedPerson {
    pub name: String,
    /// Empty until the canonical subject state owns an explicit attribute.
    pub supported_identity_attributes: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct WorldNewspaperGroundingFinding {
    pub article_index: u16,
    pub category: WorldNewspaperGroundingCategory,
    pub claim_or_phrase: String,
    pub reason: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorldNewspaperGroundingCategory {
    UnsupportedFact,
    UnearnedAttribution,
    ProceduralLeakage,
    MechanicalCopy,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct WorldNewspaperGroundingVerdict {
    pub accepted: bool,
    pub assessment: String,
    pub findings: Vec<WorldNewspaperGroundingFinding>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct WorldNewspaperComposition {
    pub schema: String,
    pub issue: WorldNewspaperIssue,
    pub grounding: WorldNewspaperGroundingVerdict,
    pub model_receipts: Vec<ModelStageReceipt>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum WorldNewspaperAdvance {
    Accepted {
        composition: WorldNewspaperComposition,
    },
    Pending {
        checkpoint: WorldNewspaperReconciliationCheckpoint,
        model_receipts: Vec<ModelStageReceipt>,
    },
}

impl WorldNewspaperAdvance {
    pub fn model_receipts(&self) -> &[ModelStageReceipt] {
        match self {
            Self::Accepted { composition } => &composition.model_receipts,
            Self::Pending { model_receipts, .. } => model_receipts,
        }
    }

    pub fn into_accepted(self) -> Option<WorldNewspaperComposition> {
        match self {
            Self::Accepted { composition } => Some(composition),
            Self::Pending { .. } => None,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorldNewspaperCheckpointOrigin {
    InitialCopyDesk,
    Reconciliation,
    LegacyTerminalImport,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct WorldNewspaperReconciliationCheckpoint {
    schema: String,
    id: String,
    publication_task_binding: String,
    editorial_binding: String,
    generation: u32,
    previous_checkpoint_id: Option<String>,
    origin: WorldNewspaperCheckpointOrigin,
    source_witness_digest: Option<String>,
    #[serde(default)]
    editorial_agenda: Option<WorldNewspaperEditorialAgenda>,
    draft: EditorialPageDraft,
    verdict: WorldNewspaperGroundingVerdict,
    model_receipt_ids: Vec<String>,
    receipt_chain_digest: String,
}

impl WorldNewspaperReconciliationCheckpoint {
    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn editorial_binding(&self) -> &str {
        &self.editorial_binding
    }

    pub fn generation(&self) -> u32 {
        self.generation
    }

    pub fn previous_checkpoint_id(&self) -> Option<&str> {
        self.previous_checkpoint_id.as_deref()
    }

    pub fn model_receipt_ids(&self) -> &[String] {
        &self.model_receipt_ids
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct WorldNewspaperReconciliationImport {
    schema: String,
    source_witness_digest: String,
    #[serde(default)]
    editorial_agenda: Option<WorldNewspaperEditorialAgenda>,
    draft: EditorialPageDraft,
    verdict: WorldNewspaperGroundingVerdict,
    model_receipts: Vec<ModelStageReceipt>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct PersistedWorldNewspaperComposition {
    schema: String,
    publication_task_binding: String,
    editorial_binding: String,
    source_checkpoint_id: Option<String>,
    composition: WorldNewspaperComposition,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct WorldNewspaperCompositionFailure {
    pub message: String,
    pub model_receipts: Vec<ModelStageReceipt>,
}

impl fmt::Display for WorldNewspaperCompositionFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl StdError for WorldNewspaperCompositionFailure {}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
struct PublicRecordProjection {
    record_id: String,
    at: DateTime<Utc>,
    channel: String,
    headline: String,
    reliability: String,
    facts: Vec<WorldNewspaperSourceFact>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct WorldNewspaperEditorialAgenda {
    #[schemars(length(min = 1, max = 500))]
    pub dominant_throughline: String,
    #[schemars(length(min = 1, max = 500))]
    pub reader_stake: String,
    #[schemars(length(min = 1, max = 6))]
    pub story_pitches: Vec<WorldNewspaperStoryPitch>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct WorldNewspaperStoryPitch {
    pub lead: bool,
    #[schemars(length(min = 1))]
    pub citations: Vec<String>,
    #[serde(default)]
    pub focus_citation: String,
    #[schemars(length(min = 1, max = 500))]
    #[serde(alias = "angle")]
    pub narrative_claim: String,
    #[schemars(length(min = 1, max = 500))]
    pub tension: String,
    #[schemars(length(min = 1, max = 500))]
    pub public_question: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct NarrativeSelectionAction {
    command: NarrativeSelectionCommand,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "tool", rename_all = "snake_case", deny_unknown_fields)]
enum NarrativeSelectionCommand {
    QueryPublicRecords {
        #[schemars(length(max = 6))]
        terms: Vec<String>,
        match_terms: PublicRecordTermMatch,
        #[schemars(length(max = 6))]
        entity_names: Vec<String>,
        #[schemars(length(max = 5))]
        assertion_statuses: Vec<WorldNewspaperAssertionStatus>,
        #[schemars(length(max = 6))]
        channels: Vec<String>,
        order: PublicRecordOrder,
        cursor: Option<String>,
        #[schemars(range(min = 1, max = 24))]
        limit: u8,
    },
    SubmitAgenda {
        #[schemars(length(min = 1, max = 500))]
        dominant_throughline: String,
        #[schemars(length(min = 1, max = 500))]
        reader_stake: String,
        #[schemars(length(min = 1, max = 6))]
        story_pitches: Vec<WorldNewspaperStoryPitch>,
    },
}

#[derive(
    Clone, Copy, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq, PartialOrd, Ord,
)]
#[serde(rename_all = "snake_case")]
enum PublicRecordTermMatch {
    Any,
    All,
}

#[derive(
    Clone, Copy, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq, PartialOrd, Ord,
)]
#[serde(rename_all = "snake_case")]
enum PublicRecordOrder {
    Newest,
    Oldest,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq, PartialOrd, Ord)]
struct PublicRecordQuery {
    terms: Vec<String>,
    match_terms: PublicRecordTermMatch,
    entity_names: Vec<String>,
    assertion_statuses: Vec<WorldNewspaperAssertionStatus>,
    channels: Vec<String>,
    order: PublicRecordOrder,
    cursor: Option<String>,
    limit: u8,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum NarrativeSelectionFinding {
    AgendaRejected {
        reason: String,
    },
    QueryRejected {
        reason: String,
    },
    QueryResult {
        records: Vec<PublicRecordProjection>,
        next_cursor: Option<String>,
    },
}

struct NarrativeSelectionWorkbench<'a> {
    records: &'a [PublicRecordProjection],
    max_articles: usize,
    visible_record_ids: BTreeSet<String>,
    completed_queries: BTreeSet<PublicRecordQuery>,
}

impl NarrativeSelectionWorkbench<'_> {
    fn known_channels(&self) -> Vec<String> {
        self.records
            .iter()
            .map(|record| record.channel.clone())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }

    fn query(&mut self, mut query: PublicRecordQuery) -> Result<NarrativeSelectionFinding> {
        normalize_query_terms(&mut query.terms, "query term")?;
        normalize_query_terms(&mut query.entity_names, "query entity")?;
        query.channels.sort();
        query.channels.dedup();
        query.assertion_statuses.sort();
        query.assertion_statuses.dedup();
        if !(1..=MAX_PUBLIC_RECORD_QUERY_RESULTS).contains(&usize::from(query.limit)) {
            return Err(anyhow!(
                "public-record query limit is outside the bounded response"
            ));
        }
        if query
            .channels
            .iter()
            .any(|channel| !self.records.iter().any(|record| &record.channel == channel))
        {
            return Err(anyhow!("public-record query names an unknown channel"));
        }
        if query
            .cursor
            .as_ref()
            .is_some_and(|cursor| !self.visible_record_ids.contains(cursor))
        {
            return Err(anyhow!(
                "public-record query cursor was not previously inspected"
            ));
        }
        if !self.completed_queries.insert(query.clone()) {
            return Err(anyhow!(
                "public-record query exactly repeats an earlier query"
            ));
        }

        let ordered = match query.order {
            PublicRecordOrder::Newest => self.records.iter().collect::<Vec<_>>(),
            PublicRecordOrder::Oldest => self.records.iter().rev().collect::<Vec<_>>(),
        };
        let start = query.cursor.as_ref().map_or(0, |cursor| {
            ordered
                .iter()
                .position(|record| &record.record_id == cursor)
                .map_or(ordered.len(), |index| index.saturating_add(1))
        });
        let matching = ordered
            .into_iter()
            .skip(start)
            .filter(|record| public_record_matches(record, &query))
            .collect::<Vec<_>>();
        let limit = usize::from(query.limit);
        let records = matching
            .iter()
            .take(limit)
            .map(|record| (*record).clone())
            .collect::<Vec<_>>();
        self.visible_record_ids
            .extend(records.iter().map(|record| record.record_id.clone()));
        let next_cursor = (matching.len() > records.len())
            .then(|| records.last().map(|record| record.record_id.clone()))
            .flatten();
        Ok(NarrativeSelectionFinding::QueryResult {
            records,
            next_cursor,
        })
    }
}

fn normalize_query_terms(values: &mut Vec<String>, label: &str) -> Result<()> {
    for value in values.iter_mut() {
        *value = value.trim().to_owned();
        if value.is_empty() || value.chars().count() > 80 || value.chars().any(char::is_control) {
            return Err(anyhow!("{label} is malformed"));
        }
    }
    values.sort_by_key(|value| value.to_lowercase());
    values.dedup_by(|left, right| left.eq_ignore_ascii_case(right));
    Ok(())
}

fn public_record_matches(record: &PublicRecordProjection, query: &PublicRecordQuery) -> bool {
    if !query.channels.is_empty() && !query.channels.contains(&record.channel) {
        return false;
    }
    if !query.assertion_statuses.is_empty()
        && !record
            .facts
            .iter()
            .any(|fact| query.assertion_statuses.contains(&fact.assertion_status))
    {
        return false;
    }
    let entity_names = record
        .facts
        .iter()
        .flat_map(|fact| {
            fact.named_people
                .iter()
                .map(|person| person.name.as_str())
                .chain(fact.institutions.iter().map(String::as_str))
                .chain(fact.populations.iter().map(String::as_str))
                .chain(fact.places.iter().map(String::as_str))
        })
        .collect::<Vec<_>>();
    if !query.entity_names.is_empty()
        && !query.entity_names.iter().any(|expected| {
            entity_names
                .iter()
                .any(|actual| actual.eq_ignore_ascii_case(expected))
        })
    {
        return false;
    }
    if query.terms.is_empty() {
        return true;
    }
    let searchable = std::iter::once(record.headline.as_str())
        .chain(std::iter::once(record.channel.as_str()))
        .chain(std::iter::once(record.reliability.as_str()))
        .chain(record.facts.iter().map(|fact| fact.account.as_str()))
        .chain(entity_names)
        .collect::<Vec<_>>()
        .join("\n")
        .to_lowercase();
    match query.match_terms {
        PublicRecordTermMatch::Any => query
            .terms
            .iter()
            .any(|term| searchable.contains(&term.to_lowercase())),
        PublicRecordTermMatch::All => query
            .terms
            .iter()
            .all(|term| searchable.contains(&term.to_lowercase())),
    }
}

#[async_trait]
impl ModelAgentTool for NarrativeSelectionWorkbench<'_> {
    type Action = NarrativeSelectionAction;
    type Output = WorldNewspaperEditorialAgenda;
    type Finding = NarrativeSelectionFinding;

    fn action_schema(&self) -> std::result::Result<serde_json::Value, String> {
        let citations = self.visible_record_ids.iter().cloned().collect::<Vec<_>>();
        let story_budget = self.max_articles.min(self.records.len());
        let submit_schema = serde_json::json!({
            "type":"object",
            "additionalProperties":false,
            "required":[
                "tool",
                "dominant_throughline",
                "reader_stake",
                "story_pitches"
            ],
            "properties":{
                "tool":{"const":"submit_agenda"},
                "dominant_throughline":{"type":"string","minLength":1,"maxLength":500},
                "reader_stake":{"type":"string","minLength":1,"maxLength":500},
                "story_pitches":{
                    "type":"array",
                    "minItems":1,
                    "maxItems":story_budget,
                    "items":{
                        "type":"object",
                        "additionalProperties":false,
                        "required":[
                            "lead",
                            "citations",
                            "focus_citation",
                            "narrative_claim",
                            "tension",
                            "public_question"
                        ],
                        "properties":{
                            "lead":{"type":"boolean"},
                            "citations":{
                                "type":"array",
                                "minItems":1,
                                "maxItems":citations.len(),
                                "uniqueItems":true,
                                "items":{"type":"string","enum":citations}
                            },
                            "focus_citation":{"type":"string","enum":citations},
                            "narrative_claim":{"type":"string","minLength":1,"maxLength":500},
                            "tension":{"type":"string","minLength":1,"maxLength":500},
                            "public_question":{"type":"string","minLength":1,"maxLength":500}
                        }
                    }
                }
            }
        });
        let cursor_schema = if citations.is_empty() {
            serde_json::json!({"type":"null"})
        } else {
            serde_json::json!({"anyOf":[
                {"type":"string","enum":citations},
                {"type":"null"}
            ]})
        };
        let assertion_status_schema =
            serde_json::to_value(schema_for!(WorldNewspaperAssertionStatus))
                .map_err(|error| error.to_string())?;
        let assertion_statuses = assertion_status_schema
            .get("enum")
            .cloned()
            .ok_or_else(|| "public assertion status schema omitted its enum".to_owned())?;
        let query_schema = serde_json::json!({
            "type":"object",
            "additionalProperties":false,
            "required":[
                "tool","terms","match_terms","entity_names",
                "assertion_statuses","channels","order","cursor","limit"
            ],
            "properties":{
                "tool":{"const":"query_public_records"},
                "terms":{"type":"array","maxItems":6,"uniqueItems":true,
                    "items":{"type":"string","minLength":1,"maxLength":80}},
                "match_terms":{"type":"string","enum":["any","all"]},
                "entity_names":{"type":"array","maxItems":6,"uniqueItems":true,
                    "items":{"type":"string","minLength":1,"maxLength":80}},
                "assertion_statuses":{"type":"array","maxItems":5,"uniqueItems":true,
                    "items":{"type":"string","enum":assertion_statuses}},
                "channels":{"type":"array","maxItems":6,"uniqueItems":true,
                    "items":{"type":"string","enum":self.known_channels()}},
                "order":{"type":"string","enum":["newest","oldest"]},
                "cursor":cursor_schema,
                "limit":{"type":"integer","minimum":1,"maximum":MAX_PUBLIC_RECORD_QUERY_RESULTS}
            }
        });
        let command_schema = if citations.is_empty() {
            query_schema
        } else {
            serde_json::json!({"oneOf":[query_schema,submit_schema]})
        };
        let mut schema = serde_json::json!({
            "type":"object",
            "additionalProperties":false,
            "required":["command"],
            "properties":{"command":command_schema}
        });
        crate::model_connector::project_strict_responses_schema(&mut schema)
            .map_err(|error| error.to_string())?;
        Ok(schema)
    }

    async fn invoke(
        &mut self,
        action: Self::Action,
        _context: &ModelAgentToolContext,
    ) -> ModelAgentToolOutcome<Self::Output, Self::Finding> {
        match action.command {
            NarrativeSelectionCommand::QueryPublicRecords {
                terms,
                match_terms,
                entity_names,
                assertion_statuses,
                channels,
                order,
                cursor,
                limit,
            } => match self.query(PublicRecordQuery {
                terms,
                match_terms,
                entity_names,
                assertion_statuses,
                channels,
                order,
                cursor,
                limit,
            }) {
                Ok(observation) => ModelAgentToolOutcome::Continue {
                    observation,
                    receipts: Vec::new(),
                },
                Err(error) => ModelAgentToolOutcome::Rejected {
                    finding: NarrativeSelectionFinding::QueryRejected {
                        reason: error.to_string().chars().take(500).collect(),
                    },
                    receipts: Vec::new(),
                },
            },
            NarrativeSelectionCommand::SubmitAgenda {
                dominant_throughline,
                reader_stake,
                story_pitches,
            } => {
                let agenda = WorldNewspaperEditorialAgenda {
                    dominant_throughline,
                    reader_stake,
                    story_pitches,
                };
                let uninspected_record = agenda
                    .story_pitches
                    .iter()
                    .flat_map(|pitch| &pitch.citations)
                    .find(|record_id| !self.visible_record_ids.contains(*record_id));
                if let Some(record_id) = uninspected_record {
                    return ModelAgentToolOutcome::Rejected {
                        finding: NarrativeSelectionFinding::AgendaRejected {
                            reason: format!(
                                "agenda cites public record {record_id} without querying it"
                            ),
                        },
                        receipts: Vec::new(),
                    };
                }
                match validate_editorial_agenda(self.records, &agenda, self.max_articles) {
                    Ok(()) => ModelAgentToolOutcome::Accepted {
                        output: agenda,
                        receipts: Vec::new(),
                    },
                    Err(error) => ModelAgentToolOutcome::Rejected {
                        finding: NarrativeSelectionFinding::AgendaRejected {
                            reason: error.to_string().chars().take(500).collect(),
                        },
                        receipts: Vec::new(),
                    },
                }
            }
        }
    }
}

fn validate_editorial_frame(value: &str, label: &str) -> Result<()> {
    if value.trim().is_empty()
        || value.trim() != value
        || value.chars().count() > 500
        || value.chars().any(char::is_control)
        || value.contains(['\r', '\n', '`'])
    {
        return Err(anyhow!("{label} is not a bounded editorial frame"));
    }
    Ok(())
}

fn selected_record_ids<'a>(record_ids: &'a [String], label: &str) -> Result<BTreeSet<&'a str>> {
    if record_ids.is_empty() {
        return Err(anyhow!("{label} selected no public record"));
    }
    let selected = record_ids
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    if selected.len() != record_ids.len() {
        return Err(anyhow!("{label} repeats a public record ID"));
    }
    Ok(selected)
}

fn validate_editorial_agenda(
    records: &[PublicRecordProjection],
    agenda: &WorldNewspaperEditorialAgenda,
    max_articles: usize,
) -> Result<()> {
    validate_editorial_frame(&agenda.dominant_throughline, "dominant throughline")?;
    validate_editorial_frame(&agenda.reader_stake, "reader stake")?;
    if agenda.story_pitches.is_empty()
        || agenda.story_pitches.len() > max_articles.min(records.len())
    {
        return Err(anyhow!("editorial agenda exceeded its story budget"));
    }
    let known_sources = records
        .iter()
        .map(|record| record.record_id.as_str())
        .collect::<BTreeSet<_>>();
    for (index, pitch) in agenda.story_pitches.iter().enumerate() {
        if pitch.lead != (index == 0) {
            return Err(anyhow!(
                "editorial agenda must designate exactly its first pitch as lead"
            ));
        }
        let selected = selected_record_ids(&pitch.citations, &format!("editorial pitch {index}"))?;
        if !selected.contains(pitch.focus_citation.as_str()) {
            return Err(anyhow!(
                "editorial pitch {index} focus record is not in its selected record set"
            ));
        }
        for citation in &pitch.citations {
            if !known_sources.contains(citation.as_str()) {
                return Err(anyhow!(
                    "editorial pitch {index} cites unknown public record {citation}"
                ));
            }
        }
        validate_editorial_frame(&pitch.narrative_claim, "story narrative claim")?;
        validate_editorial_frame(&pitch.tension, "story tension")?;
        validate_editorial_frame(&pitch.public_question, "public question")?;
    }
    Ok(())
}

fn validate_editorial_alignment(
    draft: &EditorialPageDraft,
    agenda: &WorldNewspaperEditorialAgenda,
) -> Result<()> {
    if draft.articles.len() != agenda.story_pitches.len() {
        return Err(anyhow!(
            "editorial page does not implement the admitted story selection"
        ));
    }
    for (index, (article, pitch)) in draft.articles.iter().zip(&agenda.story_pitches).enumerate() {
        let article_citations = article.citations.iter().collect::<BTreeSet<_>>();
        let pitch_citations = pitch.citations.iter().collect::<BTreeSet<_>>();
        if article_citations != pitch_citations {
            return Err(anyhow!(
                "article {index} does not implement its admitted citation grouping"
            ));
        }
    }
    Ok(())
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct EditorialPageDraft {
    #[schemars(length(min = 1, max = 6))]
    articles: Vec<EditorialArticleDraft>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct EditorialArticleDraft {
    #[schemars(length(min = 1, max = 30))]
    section: String,
    #[schemars(length(min = 1, max = 100))]
    headline: String,
    #[schemars(length(min = 1, max = 220))]
    deck: String,
    #[schemars(length(min = 1, max = 60))]
    byline: String,
    #[schemars(length(max = 100))]
    dateline: String,
    #[schemars(length(min = 1))]
    citations: Vec<String>,
    #[schemars(length(min = 2, max = 5))]
    paragraphs: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct GroundingVerdictDraft {
    accepted: bool,
    #[schemars(length(min = 1, max = 500))]
    assessment: String,
    #[schemars(length(max = 24))]
    findings: Vec<WorldNewspaperGroundingFinding>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "tool", rename_all = "snake_case", deny_unknown_fields)]
enum GroundingReconciliationAction {
    SubmitEdits {
        #[schemars(length(max = 24))]
        replacements: Vec<GroundingTextReplacement>,
        #[schemars(length(max = 6))]
        delete_finding_refs: Vec<GroundingFindingRef>,
    },
}

#[derive(
    Clone, Copy, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq, PartialOrd, Ord,
)]
#[serde(transparent)]
struct GroundingFindingRef(u16);

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct GroundingTextReplacement {
    finding_ref: GroundingFindingRef,
    #[schemars(length(max = 900))]
    replacement: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
enum GroundingEditableField {
    Headline,
    Deck,
    Dateline,
    Paragraph,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum GroundingReconciliationFinding {
    CopyDeskRejected {
        draft: EditorialPageDraft,
        verdict: WorldNewspaperGroundingVerdict,
        finding_catalog: Vec<GroundingFindingCatalogEntry>,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
struct GroundingFindingCatalogEntry {
    finding_ref: GroundingFindingRef,
    finding: WorldNewspaperGroundingFinding,
}

fn grounding_finding_catalog(
    verdict: &WorldNewspaperGroundingVerdict,
) -> Vec<GroundingFindingCatalogEntry> {
    verdict
        .findings
        .iter()
        .enumerate()
        .map(|(index, finding)| GroundingFindingCatalogEntry {
            finding_ref: GroundingFindingRef(
                u16::try_from(index).expect("bounded grounding finding index"),
            ),
            finding: finding.clone(),
        })
        .collect()
}

fn grounding_reconciliation_finding(
    draft: EditorialPageDraft,
    verdict: WorldNewspaperGroundingVerdict,
) -> GroundingReconciliationFinding {
    let finding_catalog = grounding_finding_catalog(&verdict);
    GroundingReconciliationFinding::CopyDeskRejected {
        draft,
        verdict,
        finding_catalog,
    }
}

struct GroundingReconciliationOutput {
    draft: EditorialPageDraft,
    verdict: WorldNewspaperGroundingVerdict,
}

struct PreparedNewspaper {
    title: String,
    editorial_voice: String,
    records: Vec<PublicRecordProjection>,
    source_receipt_ids: Vec<String>,
    publication_task_binding: String,
    binding: String,
}

fn prepare_newspaper(
    campaign: &Campaign,
    title: impl Into<String>,
    editorial_voice: impl Into<String>,
    max_articles: usize,
) -> Result<PreparedNewspaper> {
    if !(1..=MAX_FRONT_PAGE_ARTICLES).contains(&max_articles) {
        return Err(anyhow!(
            "newspaper front-page article budget must be between 1 and {MAX_FRONT_PAGE_ARTICLES}"
        ));
    }
    let title = title.into();
    if title.trim().is_empty()
        || title.trim() != title
        || title.chars().count() > 120
        || title.chars().any(char::is_control)
    {
        return Err(anyhow!("newspaper title is invalid"));
    }
    let editorial_voice = editorial_voice.into();
    if editorial_voice.trim().is_empty()
        || editorial_voice.trim() != editorial_voice
        || editorial_voice.chars().count() > 600
    {
        return Err(anyhow!("newspaper editorial voice is invalid"));
    }
    let records = public_news_records(campaign)?;
    let source_receipt_ids = records
        .iter()
        .map(|record| record.record_id.clone())
        .collect::<Vec<_>>();
    let publication_task_binding =
        publication_task_binding(campaign, &title, &editorial_voice, max_articles)?;
    let binding = editorial_binding(campaign, &title, &editorial_voice, max_articles, &records)?;
    Ok(PreparedNewspaper {
        title,
        editorial_voice,
        records,
        source_receipt_ids,
        publication_task_binding,
        binding,
    })
}

async fn select_editorial_agenda(
    model: &dyn ModelPort,
    prepared: &PreparedNewspaper,
    max_articles: usize,
) -> std::result::Result<
    crate::agent::ModelAgentRun<WorldNewspaperEditorialAgenda>,
    crate::agent::ModelAgentFailure,
> {
    let instructions = format!(
        "You are the narrative editor of `{}`. Construct one compelling editorial agenda by investigating a frozen public ledger containing {} canonical news records. No intermediate source object owns these facts and no initial viewport is privileged. Use query_public_records to inspect the ledger. An empty query browses it in the requested order; literal terms, exact public entity names, assertion status, channel, and an inspected cursor may narrow or page it. Query responses are bounded context over stable record IDs, not summaries that replace the ledger. Search backward for causes when recent administrative responses hide the original rupture; search by actors, places, objects, institutions, or consequences to find opposition and countermoves. Never cite a record until the workbench has returned it.\n\nA newspaper is not a neutral transcript. Select a dominant throughline that matters to this publication's readers; connect records that expose conflict, responsibility, hypocrisy, lived stakes, scandal, named opposition, public reaction, or material consequence. Omitted true facts remain true and unreported. Use every relevant inspected record that the actual story needs; article count and bounded copy define page space. Do not cover a record merely because it exists. Treat bookkeeping about someone retaining a memory as context unless the act of remembering itself caused a public consequence. The first pitch is the lead and must set lead=true; every later pitch must set lead=false. A foundational public record may support more than one continuing story when each pitch uses it for a distinct throughline. Every pitch must choose one focus_citation from its stable record IDs: the concrete fact the headline and lede should make impossible to ignore. When a later procedural update belongs to an older vivid incident, query and cite both: identify what is newly changed, acknowledge the continuing incident, and do not let routine handling impersonate the original news. narrative_claim states the pointed story the publication can responsibly construct from those records. narrative_claim, tension, public_question, dominant_throughline, and reader_stake are editorial framing and hypotheses, not evidence. They may be pointed, skeptical, or insinuating, but they must not invent an event, person, institution, place, motive, quotation, outcome, private knowledge, or factual status. The downstream editor may state facts only from the admitted record IDs and will be constrained to the exact groupings. The copy desk judges factual claims independently. Submit the agenda only after the query results support a coherent story.\n\nPUBLICATION VOICE:\n{}",
        prepared.title,
        prepared.records.len(),
        prepared.editorial_voice,
    );
    let spec = ModelAgentSpec {
        stage: "newspaper_narrative_selection_agent_action".into(),
        model: MODEL_CAPABLE.into(),
        snapshot_binding: prepared.binding.clone(),
        instructions,
        source_receipt_ids: prepared.source_receipt_ids.clone(),
        temperature: Some(0.8),
        max_output_tokens: Some(2_000),
        max_steps: MAX_NARRATIVE_SELECTION_STEPS,
    };
    let mut tool = NarrativeSelectionWorkbench {
        records: &prepared.records,
        max_articles,
        visible_record_ids: BTreeSet::new(),
        completed_queries: BTreeSet::new(),
    };
    crate::agent::run_model_agent(model, &spec, &mut tool).await
}

fn checkpoint_receipt_chain_digest(receipt_ids: &[String]) -> Result<String> {
    Ok(format!(
        "sha256:{:x}",
        Sha256::digest(rmp_serde::to_vec_named(receipt_ids)?)
    ))
}

fn checkpoint_identity(
    publication_task_binding: &str,
    binding: &str,
    generation: u32,
    previous_checkpoint_id: Option<&str>,
    origin: &WorldNewspaperCheckpointOrigin,
    source_witness_digest: Option<&str>,
    editorial_agenda: Option<&WorldNewspaperEditorialAgenda>,
    draft: &EditorialPageDraft,
    verdict: &WorldNewspaperGroundingVerdict,
    model_receipt_ids: &[String],
    receipt_chain_digest: &str,
) -> Result<String> {
    Ok(format!(
        "newspaper-reconciliation:sha256:{:x}",
        Sha256::digest(rmp_serde::to_vec_named(&(
            publication_task_binding,
            binding,
            generation,
            previous_checkpoint_id,
            origin,
            source_witness_digest,
            editorial_agenda,
            draft,
            verdict,
            model_receipt_ids,
            receipt_chain_digest,
        ))?)
    ))
}

#[allow(clippy::too_many_arguments)]
fn new_reconciliation_checkpoint(
    publication_task_binding: &str,
    binding: &str,
    generation: u32,
    previous_checkpoint_id: Option<String>,
    origin: WorldNewspaperCheckpointOrigin,
    source_witness_digest: Option<String>,
    editorial_agenda: Option<WorldNewspaperEditorialAgenda>,
    draft: EditorialPageDraft,
    verdict: WorldNewspaperGroundingVerdict,
    receipts: &[ModelStageReceipt],
) -> Result<WorldNewspaperReconciliationCheckpoint> {
    let model_receipt_ids = receipts
        .iter()
        .map(|receipt| receipt.storage_key().to_owned())
        .collect::<Vec<_>>();
    let receipt_chain_digest = checkpoint_receipt_chain_digest(&model_receipt_ids)?;
    let id = checkpoint_identity(
        publication_task_binding,
        binding,
        generation,
        previous_checkpoint_id.as_deref(),
        &origin,
        source_witness_digest.as_deref(),
        editorial_agenda.as_ref(),
        &draft,
        &verdict,
        &model_receipt_ids,
        &receipt_chain_digest,
    )?;
    Ok(WorldNewspaperReconciliationCheckpoint {
        schema: "ghostlight.world_newspaper_reconciliation_checkpoint.v2".into(),
        id,
        publication_task_binding: publication_task_binding.into(),
        editorial_binding: binding.into(),
        generation,
        previous_checkpoint_id,
        origin,
        source_witness_digest,
        editorial_agenda,
        draft,
        verdict,
        model_receipt_ids,
        receipt_chain_digest,
    })
}

fn persist_reconciliation_checkpoint(
    store: &CampaignStore,
    checkpoint: &WorldNewspaperReconciliationCheckpoint,
) -> Result<()> {
    let kind = "world_newspaper_reconciliation_checkpoint.v2";
    if let Some((_, existing)) =
        store.load::<WorldNewspaperReconciliationCheckpoint>(kind, checkpoint.id())?
    {
        if existing != *checkpoint {
            return Err(anyhow!(
                "immutable newspaper reconciliation checkpoint conflict: {}",
                checkpoint.id()
            ));
        }
        return Ok(());
    }
    store.insert(
        kind,
        "ghostlight.world_newspaper_reconciliation_checkpoint.v2",
        checkpoint.id(),
        checkpoint,
    )?;
    Ok(())
}

fn persist_newspaper_completion(
    store: &CampaignStore,
    publication_task_binding: &str,
    binding: &str,
    source_checkpoint_id: Option<String>,
    composition: &WorldNewspaperComposition,
) -> Result<()> {
    let kind = "world_newspaper_composition.v1";
    let record = PersistedWorldNewspaperComposition {
        schema: "ghostlight.persisted_world_newspaper_composition.v1".into(),
        publication_task_binding: publication_task_binding.into(),
        editorial_binding: binding.into(),
        source_checkpoint_id,
        composition: composition.clone(),
    };
    if let Some((_, existing)) =
        store.load::<PersistedWorldNewspaperComposition>(kind, publication_task_binding)?
    {
        if existing != record {
            return Err(anyhow!(
                "immutable accepted newspaper composition conflict for {binding}"
            ));
        }
        return Ok(());
    }
    store.insert(
        kind,
        "ghostlight.persisted_world_newspaper_composition.v1",
        publication_task_binding,
        &record,
    )?;
    Ok(())
}

fn load_checkpoint_receipts(
    store: &CampaignStore,
    checkpoint: &WorldNewspaperReconciliationCheckpoint,
) -> Result<Vec<ModelStageReceipt>> {
    checkpoint
        .model_receipt_ids
        .iter()
        .map(|receipt_id| {
            store
                .load::<ModelStageReceipt>("persona_stage_receipt.v1", receipt_id)?
                .map(|(_, receipt)| receipt)
                .ok_or_else(|| anyhow!("newspaper checkpoint lost model receipt {receipt_id}"))
        })
        .collect()
}

fn validate_reconciliation_checkpoint(
    checkpoint: &WorldNewspaperReconciliationCheckpoint,
    prepared: &PreparedNewspaper,
    max_articles: usize,
    receipts: &[ModelStageReceipt],
) -> Result<()> {
    if checkpoint.schema != "ghostlight.world_newspaper_reconciliation_checkpoint.v2"
        || checkpoint.publication_task_binding != prepared.publication_task_binding
        || checkpoint.editorial_binding != prepared.binding
        || checkpoint.verdict.accepted
        || checkpoint.verdict.findings.is_empty()
        || receipts.len() != checkpoint.model_receipt_ids.len()
    {
        return Err(anyhow!("newspaper reconciliation checkpoint is invalid"));
    }
    if checkpoint.generation == 0 && checkpoint.previous_checkpoint_id.is_some()
        || checkpoint.generation > 0 && checkpoint.previous_checkpoint_id.is_none()
    {
        return Err(anyhow!(
            "newspaper reconciliation checkpoint lineage is invalid"
        ));
    }
    let receipt_ids = receipts
        .iter()
        .map(|receipt| receipt.storage_key().to_owned())
        .collect::<Vec<_>>();
    if receipt_ids != checkpoint.model_receipt_ids
        || receipt_ids.iter().collect::<BTreeSet<_>>().len() != receipt_ids.len()
        || checkpoint_receipt_chain_digest(&receipt_ids)? != checkpoint.receipt_chain_digest
        || checkpoint_identity(
            &checkpoint.publication_task_binding,
            &checkpoint.editorial_binding,
            checkpoint.generation,
            checkpoint.previous_checkpoint_id.as_deref(),
            &checkpoint.origin,
            checkpoint.source_witness_digest.as_deref(),
            checkpoint.editorial_agenda.as_ref(),
            &checkpoint.draft,
            &checkpoint.verdict,
            &checkpoint.model_receipt_ids,
            &checkpoint.receipt_chain_digest,
        )? != checkpoint.id
    {
        return Err(anyhow!(
            "newspaper reconciliation checkpoint receipt binding is invalid"
        ));
    }
    let ancestry_valid = match checkpoint.origin {
        WorldNewspaperCheckpointOrigin::LegacyTerminalImport => {
            if checkpoint.editorial_agenda.is_some() {
                let editor_index = receipts
                    .iter()
                    .position(|receipt| receipt.stage == "newspaper_editor");
                editor_index.is_some_and(|index| {
                    index > 0
                        && receipts[..index].iter().all(|receipt| {
                            receipt.stage == "newspaper_narrative_selection_agent_action"
                        })
                        && receipts
                            .get(index + 1)
                            .map(|receipt| receipt.stage.as_str())
                            == Some("newspaper_copy_desk")
                })
            } else {
                receipts.first().map(|receipt| receipt.stage.as_str()) == Some("newspaper_editor")
                    && receipts.get(1).map(|receipt| receipt.stage.as_str())
                        == Some("newspaper_copy_desk")
            }
        }
        WorldNewspaperCheckpointOrigin::InitialCopyDesk
        | WorldNewspaperCheckpointOrigin::Reconciliation => {
            let editor_index = receipts
                .iter()
                .position(|receipt| receipt.stage == "newspaper_editor");
            checkpoint.editorial_agenda.is_some()
                && editor_index.is_some_and(|index| {
                    index > 0
                        && receipts[..index].iter().all(|receipt| {
                            receipt.stage == "newspaper_narrative_selection_agent_action"
                        })
                        && receipts
                            .get(index + 1)
                            .map(|receipt| receipt.stage.as_str())
                            == Some("newspaper_copy_desk")
                })
        }
    };
    if !ancestry_valid
        || receipts
            .first()
            .map(|receipt| receipt.snapshot_binding.as_str())
            != Some(prepared.binding.as_str())
    {
        return Err(anyhow!(
            "newspaper reconciliation checkpoint lost its narrative, editor, or copy-desk ancestry"
        ));
    }
    validate_editorial_draft(&prepared.records, &checkpoint.draft, max_articles)?;
    if let Some(agenda) = &checkpoint.editorial_agenda {
        validate_editorial_agenda(&prepared.records, agenda, max_articles)?;
        validate_editorial_alignment(&checkpoint.draft, agenda)?;
    }
    let verdict = GroundingVerdictDraft {
        accepted: checkpoint.verdict.accepted,
        assessment: checkpoint.verdict.assessment.clone(),
        findings: checkpoint.verdict.findings.clone(),
    };
    validate_grounding_verdict(&checkpoint.draft, &verdict)?;
    Ok(())
}

fn load_reconciliation_tip(
    store: &CampaignStore,
    prepared: &PreparedNewspaper,
    max_articles: usize,
) -> Result<
    Option<(
        WorldNewspaperReconciliationCheckpoint,
        Vec<ModelStageReceipt>,
    )>,
> {
    let checkpoints = store
        .load_all::<WorldNewspaperReconciliationCheckpoint>(
            "world_newspaper_reconciliation_checkpoint.v2",
        )?
        .into_iter()
        .filter(|checkpoint| {
            checkpoint.publication_task_binding == prepared.publication_task_binding
        })
        .collect::<Vec<_>>();
    if checkpoints.is_empty() {
        return Ok(None);
    }
    let by_id = checkpoints
        .iter()
        .map(|checkpoint| (checkpoint.id.as_str(), checkpoint))
        .collect::<BTreeMap<_, _>>();
    let referenced = checkpoints
        .iter()
        .filter_map(|checkpoint| checkpoint.previous_checkpoint_id.as_deref())
        .collect::<BTreeSet<_>>();
    let tips = checkpoints
        .iter()
        .filter(|checkpoint| !referenced.contains(checkpoint.id.as_str()))
        .collect::<Vec<_>>();
    if tips.len() != 1 {
        return Err(anyhow!(
            "newspaper reconciliation checkpoint chain has multiple or missing tips"
        ));
    }
    for checkpoint in &checkpoints {
        let receipts = load_checkpoint_receipts(store, checkpoint)?;
        validate_reconciliation_checkpoint(checkpoint, prepared, max_articles, &receipts)?;
        if let Some(previous_id) = checkpoint.previous_checkpoint_id.as_deref() {
            let previous = by_id
                .get(previous_id)
                .ok_or_else(|| anyhow!("newspaper reconciliation checkpoint parent is missing"))?;
            if checkpoint.generation != previous.generation + 1
                || !checkpoint
                    .model_receipt_ids
                    .starts_with(&previous.model_receipt_ids)
                || checkpoint.model_receipt_ids.len() <= previous.model_receipt_ids.len()
            {
                return Err(anyhow!(
                    "newspaper reconciliation checkpoint chain is not an exact receipt-prefix extension"
                ));
            }
        }
    }
    let tip = (*tips[0]).clone();
    let receipts = load_checkpoint_receipts(store, &tip)?;
    Ok(Some((tip, receipts)))
}

pub async fn advance_world_newspaper(
    model: &dyn ModelPort,
    campaign: &Campaign,
    title: impl Into<String>,
    editorial_voice: impl Into<String>,
    max_articles: usize,
    store: &CampaignStore,
) -> Result<WorldNewspaperAdvance> {
    let prepared = prepare_newspaper(campaign, title, editorial_voice, max_articles)?;
    if let Some((_, persisted)) = store.load::<PersistedWorldNewspaperComposition>(
        "world_newspaper_composition.v1",
        &prepared.publication_task_binding,
    )? {
        if persisted.schema != "ghostlight.persisted_world_newspaper_composition.v1"
            || persisted.publication_task_binding != prepared.publication_task_binding
            || persisted.editorial_binding != prepared.binding
            || !persisted.composition.grounding.accepted
            || persisted.composition.issue.source_world_revision != campaign.revision
            || persisted.composition.issue.title != prepared.title
        {
            return Err(anyhow!("persisted newspaper composition is invalid"));
        }
        return Ok(WorldNewspaperAdvance::Accepted {
            composition: persisted.composition,
        });
    }
    if let Some((checkpoint, receipts)) = load_reconciliation_tip(store, &prepared, max_articles)? {
        return advance_reconciliation(
            model,
            campaign,
            &prepared,
            max_articles,
            store,
            checkpoint,
            receipts,
        )
        .await;
    }
    if prepared.records.is_empty() {
        let issue = WorldNewspaperIssue {
            schema: "ghostlight.world_newspaper_issue.v3".into(),
            id: empty_issue_id(campaign, &prepared.title)?,
            title: prepared.title.clone(),
            edition_label: "No edition issued".into(),
            at: campaign.world_time,
            source_world_revision: campaign.revision,
            lead_article_id: None,
            editorial_agenda: None,
            articles: Vec::new(),
            editorial_receipt_ids: Vec::new(),
        };
        let composition = WorldNewspaperComposition {
            schema: "ghostlight.world_newspaper_composition.v1".into(),
            issue,
            grounding: WorldNewspaperGroundingVerdict {
                accepted: true,
                assessment: "No public source material was available, so no edition was issued."
                    .into(),
                findings: Vec::new(),
            },
            model_receipts: Vec::new(),
        };
        persist_newspaper_completion(
            store,
            &prepared.publication_task_binding,
            &prepared.binding,
            None,
            &composition,
        )?;
        return Ok(WorldNewspaperAdvance::Accepted { composition });
    }
    let agenda_run = select_editorial_agenda(model, &prepared, max_articles)
        .await
        .map_err(|failure| composition_failure(failure.message, failure.receipts))?;
    let agenda = agenda_run.output;
    let editorial_schema = editorial_schema(&prepared.records, &agenda)?;
    let agenda_json = serde_json::to_string_pretty(&agenda)?;
    let selected_source_json = source_json_for_agenda(&prepared.records, Some(&agenda))?;
    let base_prompt = format!(
        "OUTPUT JSON SCHEMA (follow exactly):\n{}\n\nYou are the accountable copy editor of an in-world newspaper. Turn the admitted narrative agenda and bounded newsroom fact desk below into one convincing front page for `{title}`. The narrative agenda owns selection, article order, focus citation, narrative claim, and exact citation groupings. Implement every pitch in order and use exactly that pitch's citations for the corresponding article. Center each headline and lede on the most consequential concrete fact supported by its focus_citation, then use the other selected records to substantiate or complicate the narrative_claim. Record bookkeeping, memory retention, maintained warnings, and routine procedure are not automatically the news merely because they are recent. When a later update belongs to an older vivid incident, distinguish what changed from what persists and tell readers plainly that this is a continuing story; do not parade the handling protocol in front of the incident itself. The agenda's narrative_claim, tension, public_question, dominant_throughline, and reader_stake are framing instructions, not source evidence. You may express their interpretation or insinuation as clearly editorial language, but every concrete factual assertion must come from the cited public records. Do not recover omitted sources or invent a connective fact merely because the agenda suggests one. The first article must use section `Front Page` and, when its citations name a place, use one of those supplied place names as its dateline. Later articles use the other supplied newspaper sections.\n\nRewrite completely in the publication voice for readers who live in this world. Build a readable throughline: lead with the vivid consequence, identify opposing named actors and countermoves when the cited facts supply them, make lived material stakes legible, and let later stories echo or complicate the lead. Attribute claims and evidence to the named institution, notice, witness, or public act that supplied them. Report a published notice as a notice about physical evidence; never say an institution published the teeth, seal, corpse, or other object itself. When records dispute a document, accusation, identity, outcome, or authority, preserve the dispute with explicit attribution or words such as alleged or disputed instead of selecting one claim as settled fact. Never invent quotations to simulate reportage.\n\nHeadlines report consequences rather than state transitions. Decks add context instead of repeating headlines. Paragraphs explain why events matter to local readers, connect institutional moves, and vary their rhythm without explaining proper nouns like a setting guide. Keep evidence inventories plain and attributed. Dry barbs, metaphor, political characterization, and rhetorical judgment are welcome when they introduce no new concrete entity, occurrence, status, motive, quotation, number, or private knowledge. This is a newspaper with a point of view, not parody and not a world-state transcript.\n\nEvery factual assertion must be supported by the cited records for that article. You may synthesize implications plainly supported by several citations, but do not invent quotations, people, offices, places, numbers, documents, motives, chronology, outcomes, or private knowledge. Treat assertion_status as authoritative: an attempt has no result, a committed course does not complete actions embedded in its agenda, a public declaration does not prove its demand succeeded, and only material_change_committed supports a completed material consequence. A named person's supported_identity_attributes list is exhaustive; when empty, use their name or identity-neutral wording rather than inventing pronouns, gender, title, kinship, or office. Language such as attempts, tries, plans, prepares, readies, seeks, or investigates records activity, not outcome: preserve that uncertainty and do not turn it into an established or official inquiry, public availability, completion, or success unless a citation states that consequence. Use only the allowed generic bylines; they are presentation labels, not new people. Use only a supplied place name as a dateline, or the empty string. The newspaper contract owns a neutral edition label; do not invent or print a calendar, date, price, circulation claim, weather report, advertisement, or notice absent from the desk. Do not make the fact desk, citations, agenda, or verification process part of the reader-facing copy. Never end a headline with an ellipsis.\n\nPUBLICATION VOICE:\n{editorial_voice}\n\nADMITTED NARRATIVE AGENDA:\n{agenda_json}\n\nNEWSROOM FACT DESK:\n{source_json}",
        serde_json::to_string(&editorial_schema)?,
        title = prepared.title,
        editorial_voice = prepared.editorial_voice,
        agenda_json = agenda_json,
        source_json = selected_source_json,
    );
    let mut receipts = agenda_run.receipts;
    let editor_sources = prepared
        .source_receipt_ids
        .iter()
        .cloned()
        .chain(
            receipts
                .iter()
                .map(|receipt| receipt.storage_key().to_owned()),
        )
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    let request = ModelStageRequest {
        stage: "newspaper_editor".into(),
        model: MODEL_CAPABLE.into(),
        snapshot_binding: prepared.binding.clone(),
        lived_stream: base_prompt,
        output_schema: Some(editorial_schema),
        source_receipt_ids: editor_sources,
        temperature: Some(0.75),
        max_output_tokens: Some(4_500),
    };
    let editor_output = run_validated_stage(model, &request).await?;
    receipts.push(editor_output.receipt);
    let editor_receipt_index = receipts.len() - 1;
    let editor_structured = match editor_output.structured {
        Some(structured) => structured,
        None => {
            let error = anyhow!("newspaper editor returned no structured output");
            mark_semantic_invalid(&mut receipts[editor_receipt_index], &error);
            return Err(composition_failure(error.to_string(), receipts));
        }
    };
    let draft: EditorialPageDraft = match serde_json::from_value(editor_structured) {
        Ok(draft) => draft,
        Err(error) => {
            let error = anyhow!("newspaper editor returned an invalid page: {error}");
            mark_semantic_invalid(&mut receipts[editor_receipt_index], &error);
            return Err(composition_failure(error.to_string(), receipts));
        }
    };

    if let Err(error) = validate_editorial_draft(&prepared.records, &draft, max_articles) {
        mark_semantic_invalid(&mut receipts[editor_receipt_index], &error);
        return Err(composition_failure(error.to_string(), receipts));
    }
    if let Err(error) = validate_editorial_alignment(&draft, &agenda) {
        mark_semantic_invalid(&mut receipts[editor_receipt_index], &error);
        return Err(composition_failure(error.to_string(), receipts));
    }
    store.persist_model_stage_receipts(&receipts)?;
    let editor_receipt_id = receipts[editor_receipt_index].storage_key().to_owned();
    let editor_output_hash = receipts[editor_receipt_index].output_hash.clone();
    let verdict = match run_copy_desk(
        model,
        format!("{}:draft:{editor_output_hash}", prepared.binding),
        &selected_source_json,
        &prepared.source_receipt_ids,
        std::slice::from_ref(&editor_receipt_id),
        &draft,
        &mut receipts,
    )
    .await
    {
        Ok(verdict) => verdict,
        Err(error) => {
            store.persist_model_stage_receipts(&receipts)?;
            return Err(composition_failure(error.to_string(), receipts));
        }
    };
    store.persist_model_stage_receipts(&receipts)?;
    if verdict.accepted {
        let issue = lower_editorial_page(
            campaign,
            prepared.title.clone(),
            &prepared.records,
            Some(agenda.clone()),
            draft,
            &receipts,
        )?;
        let composition = WorldNewspaperComposition {
            schema: "ghostlight.world_newspaper_composition.v1".into(),
            issue,
            grounding: verdict,
            model_receipts: receipts,
        };
        persist_newspaper_completion(
            store,
            &prepared.publication_task_binding,
            &prepared.binding,
            None,
            &composition,
        )?;
        return Ok(WorldNewspaperAdvance::Accepted { composition });
    }
    let checkpoint = new_reconciliation_checkpoint(
        &prepared.publication_task_binding,
        &prepared.binding,
        0,
        None,
        WorldNewspaperCheckpointOrigin::InitialCopyDesk,
        None,
        Some(agenda),
        draft,
        verdict,
        &receipts,
    )?;
    persist_reconciliation_checkpoint(store, &checkpoint)?;
    advance_reconciliation(
        model,
        campaign,
        &prepared,
        max_articles,
        store,
        checkpoint,
        receipts,
    )
    .await
}

#[cfg(test)]
async fn compose_world_newspaper(
    model: &dyn ModelPort,
    campaign: &Campaign,
    title: impl Into<String>,
    editorial_voice: impl Into<String>,
    max_articles: usize,
) -> Result<WorldNewspaperComposition> {
    let directory = tempfile::tempdir()?;
    let store = CampaignStore::open(directory.path().join("campaign.cc"))?;
    match advance_world_newspaper(
        model,
        campaign,
        title,
        editorial_voice,
        max_articles,
        &store,
    )
    .await?
    {
        WorldNewspaperAdvance::Accepted { composition } => Ok(composition),
        WorldNewspaperAdvance::Pending { model_receipts, .. } => Err(composition_failure(
            format!(
                "grounding reconciliation pending after {} semantic steps",
                GROUNDING_RECONCILIATION_ACTIONS_PER_ADVANCE
            ),
            model_receipts,
        )),
    }
}

pub fn admit_world_newspaper_reconciliation_import(
    campaign: &Campaign,
    title: impl Into<String>,
    editorial_voice: impl Into<String>,
    max_articles: usize,
    store: &CampaignStore,
    import: WorldNewspaperReconciliationImport,
) -> Result<WorldNewspaperReconciliationCheckpoint> {
    let prepared = prepare_newspaper(campaign, title, editorial_voice, max_articles)?;
    let existing_tip = load_reconciliation_tip(store, &prepared, max_articles)?;
    if import.schema != "ghostlight.world_newspaper_reconciliation_import.v1"
        || !import.source_witness_digest.starts_with("sha256:")
        || import.source_witness_digest.len() != 71
        || store
            .load::<PersistedWorldNewspaperComposition>(
                "world_newspaper_composition.v1",
                &prepared.publication_task_binding,
            )?
            .is_some()
    {
        return Err(anyhow!(
            "world newspaper reconciliation import is not admissible"
        ));
    }
    let checkpoint = new_reconciliation_checkpoint(
        &prepared.publication_task_binding,
        &prepared.binding,
        0,
        None,
        WorldNewspaperCheckpointOrigin::LegacyTerminalImport,
        Some(import.source_witness_digest),
        import.editorial_agenda,
        import.draft,
        import.verdict,
        &import.model_receipts,
    )?;
    validate_reconciliation_checkpoint(
        &checkpoint,
        &prepared,
        max_articles,
        &import.model_receipts,
    )?;
    if let Some((existing, receipts)) = existing_tip {
        if existing == checkpoint && receipts == import.model_receipts {
            return Ok(existing);
        }
        return Err(anyhow!(
            "world newspaper reconciliation import would fork an existing checkpoint chain"
        ));
    }
    store.persist_model_stage_receipts(&import.model_receipts)?;
    persist_reconciliation_checkpoint(store, &checkpoint)?;
    Ok(checkpoint)
}

#[allow(clippy::too_many_arguments)]
async fn advance_reconciliation(
    model: &dyn ModelPort,
    campaign: &Campaign,
    prepared: &PreparedNewspaper,
    max_articles: usize,
    store: &CampaignStore,
    mut checkpoint: WorldNewspaperReconciliationCheckpoint,
    mut receipts: Vec<ModelStageReceipt>,
) -> Result<WorldNewspaperAdvance> {
    validate_reconciliation_checkpoint(&checkpoint, prepared, max_articles, &receipts)?;
    for _ in 0..GROUNDING_RECONCILIATION_ACTIONS_PER_ADVANCE {
        let selected_source_json =
            source_json_for_agenda(&prepared.records, checkpoint.editorial_agenda.as_ref())?;
        let progress = run_grounding_reconciliation_step(
            model,
            &prepared.records,
            max_articles,
            &prepared.binding,
            &selected_source_json,
            &prepared.source_receipt_ids,
            &receipts,
            checkpoint.id(),
            checkpoint.draft.clone(),
            checkpoint.verdict.clone(),
        )
        .await;
        match progress {
            Ok((ModelAgentProgress::Accepted(run), _, _)) => {
                store.persist_model_stage_receipts(&run.receipts)?;
                receipts.extend(run.receipts);
                let issue = lower_editorial_page(
                    campaign,
                    prepared.title.clone(),
                    &prepared.records,
                    checkpoint.editorial_agenda.clone(),
                    run.output.draft,
                    &receipts,
                )?;
                let composition = WorldNewspaperComposition {
                    schema: "ghostlight.world_newspaper_composition.v1".into(),
                    issue,
                    grounding: run.output.verdict,
                    model_receipts: receipts,
                };
                persist_newspaper_completion(
                    store,
                    &prepared.publication_task_binding,
                    &prepared.binding,
                    Some(checkpoint.id.clone()),
                    &composition,
                )?;
                return Ok(WorldNewspaperAdvance::Accepted { composition });
            }
            Ok((ModelAgentProgress::Exhausted(exhausted), draft, verdict)) => {
                store.persist_model_stage_receipts(&exhausted.receipts)?;
                receipts.extend(exhausted.receipts);
                let next = new_reconciliation_checkpoint(
                    &prepared.publication_task_binding,
                    &prepared.binding,
                    checkpoint.generation + 1,
                    Some(checkpoint.id.clone()),
                    WorldNewspaperCheckpointOrigin::Reconciliation,
                    None,
                    checkpoint.editorial_agenda.clone(),
                    draft,
                    verdict,
                    &receipts,
                )?;
                persist_reconciliation_checkpoint(store, &next)?;
                checkpoint = next;
            }
            Err(failure) => {
                store.persist_model_stage_receipts(&failure.receipts)?;
                receipts.extend(failure.receipts);
                return Err(composition_failure(failure.message, receipts));
            }
        }
    }
    Ok(WorldNewspaperAdvance::Pending {
        checkpoint,
        model_receipts: receipts,
    })
}

struct GroundingReconciliationWorkbench<'a> {
    model: &'a dyn ModelPort,
    records: &'a [PublicRecordProjection],
    max_articles: usize,
    binding: &'a str,
    source_json: &'a str,
    source_receipt_ids: &'a [String],
    draft: EditorialPageDraft,
    verdict: WorldNewspaperGroundingVerdict,
}

#[async_trait]
impl ModelAgentTool for GroundingReconciliationWorkbench<'_> {
    type Action = GroundingReconciliationAction;
    type Output = GroundingReconciliationOutput;
    type Finding = GroundingReconciliationFinding;

    fn action_schema(&self) -> std::result::Result<serde_json::Value, String> {
        let mut replacement_schema = serde_json::to_value(schema_for!(GroundingTextReplacement))
            .map_err(|error| error.to_string())?;
        let definitions = replacement_schema
            .as_object_mut()
            .and_then(|schema| schema.remove("$defs"))
            .unwrap_or_else(|| serde_json::json!({}));
        if let Some(schema) = replacement_schema.as_object_mut() {
            schema.remove("$schema");
        }
        let finding_indices = (0..self.verdict.findings.len())
            .map(serde_json::Value::from)
            .collect::<Vec<_>>();
        replacement_schema["properties"]["finding_ref"] = serde_json::json!({
            "type":"integer",
            "enum":finding_indices
        });
        let mut schema = serde_json::json!({
            "type":"object",
            "additionalProperties":false,
            "required":["tool", "replacements", "delete_finding_refs"],
            "properties":{
                "tool":{"const":"submit_edits"},
                "replacements":{
                    "type":"array",
                    "maxItems":24,
                    "items":replacement_schema
                },
                "delete_finding_refs":{
                    "type":"array",
                    "maxItems":6,
                    "items":{"type":"integer","enum":finding_indices}
                }
            },
            "$defs":definitions
        });
        crate::model_connector::project_strict_responses_schema(&mut schema)
            .map_err(|error| error.to_string())?;
        Ok(schema)
    }

    async fn invoke(
        &mut self,
        action: Self::Action,
        context: &ModelAgentToolContext,
    ) -> ModelAgentToolOutcome<Self::Output, Self::Finding> {
        let draft = match apply_grounding_edits(&self.draft, &self.verdict, action) {
            Ok(draft) => draft,
            Err(error) => {
                let verdict = WorldNewspaperGroundingVerdict {
                    accepted: false,
                    assessment: format!("The proposed narrow repair was not admitted: {error}")
                        .chars()
                        .take(500)
                        .collect(),
                    findings: self.verdict.findings.clone(),
                };
                self.verdict = verdict.clone();
                return ModelAgentToolOutcome::Rejected {
                    finding: grounding_reconciliation_finding(self.draft.clone(), verdict),
                    receipts: Vec::new(),
                };
            }
        };
        if let Err(error) = validate_editorial_draft(self.records, &draft, self.max_articles) {
            let verdict = WorldNewspaperGroundingVerdict {
                accepted: false,
                assessment: format!("The narrow repair produced invalid copy: {error}")
                    .chars()
                    .take(500)
                    .collect(),
                findings: self.verdict.findings.clone(),
            };
            self.verdict = verdict.clone();
            return ModelAgentToolOutcome::Rejected {
                finding: grounding_reconciliation_finding(self.draft.clone(), verdict),
                receipts: Vec::new(),
            };
        }
        let draft_digest = match rmp_serde::to_vec_named(&draft) {
            Ok(bytes) => format!("sha256:{:x}", Sha256::digest(bytes)),
            Err(error) => {
                return ModelAgentToolOutcome::Failed {
                    message: format!("grounding reconciliation could not bind its draft: {error}"),
                    receipts: Vec::new(),
                };
            }
        };
        let editorial_receipt_ids = context
            .source_receipt_ids
            .iter()
            .filter(|receipt_id| !self.source_receipt_ids.contains(receipt_id))
            .cloned()
            .collect::<Vec<_>>();
        let mut copy_desk_receipts = Vec::new();
        let verdict = match run_copy_desk(
            self.model,
            format!("{}:grounding-reconciliation:{draft_digest}", self.binding),
            self.source_json,
            self.source_receipt_ids,
            &editorial_receipt_ids,
            &draft,
            &mut copy_desk_receipts,
        )
        .await
        {
            Ok(verdict) => verdict,
            Err(error) => {
                return ModelAgentToolOutcome::Failed {
                    message: error.to_string(),
                    receipts: copy_desk_receipts,
                };
            }
        };
        if verdict.accepted {
            ModelAgentToolOutcome::Accepted {
                output: GroundingReconciliationOutput { draft, verdict },
                receipts: copy_desk_receipts,
            }
        } else {
            self.draft = draft.clone();
            self.verdict = verdict.clone();
            ModelAgentToolOutcome::Rejected {
                finding: grounding_reconciliation_finding(draft, verdict),
                receipts: copy_desk_receipts,
            }
        }
    }
}

fn apply_grounding_edits(
    original: &EditorialPageDraft,
    verdict: &WorldNewspaperGroundingVerdict,
    action: GroundingReconciliationAction,
) -> Result<EditorialPageDraft> {
    let GroundingReconciliationAction::SubmitEdits {
        replacements,
        delete_finding_refs,
    } = action;
    if replacements.is_empty() && delete_finding_refs.is_empty() {
        return Err(anyhow!(
            "a grounding repair must address at least one exact finding"
        ));
    }
    let mut deletion_findings = BTreeSet::new();
    let mut deletions = BTreeSet::new();
    for finding_ref in delete_finding_refs {
        let finding_index = usize::from(finding_ref.0);
        let finding = verdict
            .findings
            .get(finding_index)
            .ok_or_else(|| anyhow!("grounding repair names an invalid finding"))?;
        let article_index = usize::from(finding.article_index);
        if !deletion_findings.insert(finding_index) || !deletions.insert(article_index) {
            return Err(anyhow!(
                "grounding repair names an invalid or duplicate article deletion"
            ));
        }
    }
    if deletions.len() >= original.articles.len() {
        return Err(anyhow!("grounding repair cannot delete the entire edition"));
    }

    let expected_findings = (0..verdict.findings.len()).collect::<BTreeSet<_>>();
    let mut addressed = deletions
        .iter()
        .flat_map(|article_index| {
            verdict
                .findings
                .iter()
                .enumerate()
                .filter_map(move |(finding_index, finding)| {
                    (usize::from(finding.article_index) == *article_index).then_some(finding_index)
                })
        })
        .collect::<BTreeSet<_>>();
    let mut edits_by_target = BTreeMap::<
        (usize, GroundingEditableField, Option<u16>),
        Vec<(usize, usize, usize, String)>,
    >::new();
    let mut draft = original.clone();
    for replacement in replacements {
        let finding_index = usize::from(replacement.finding_ref.0);
        let finding = verdict
            .findings
            .get(finding_index)
            .ok_or_else(|| anyhow!("grounding repair names an invalid finding"))?;
        let resolved = resolve_grounding_finding_target(original, finding)?;
        let article_index = resolved.article_index;
        if deletions.contains(&article_index) {
            return Err(anyhow!(
                "grounding repair cannot both rewrite and delete one article"
            ));
        }
        if !addressed.insert(finding_index) {
            return Err(anyhow!("grounding repair repeats one exact finding"));
        }
        let target_key = (article_index, resolved.field, resolved.paragraph_index);
        if replacement.replacement.trim() != replacement.replacement
            || replacement.replacement.contains(['\r', '\n'])
            || replacement.replacement.chars().any(char::is_control)
        {
            return Err(anyhow!(
                "grounding replacement is not bounded reader-facing text"
            ));
        }
        edits_by_target.entry(target_key).or_default().push((
            resolved.start,
            resolved.end,
            finding_index,
            replacement.replacement,
        ));
    }
    for ((article_index, field, paragraph_index), mut edits) in edits_by_target {
        edits.sort_by_key(|(start, end, finding_index, _)| {
            (*start, std::cmp::Reverse(*end), *finding_index)
        });
        let mut disjoint_edits = Vec::with_capacity(edits.len());
        for edit in edits {
            if let Some((_, previous_end, _, _)) = disjoint_edits.last() {
                if edit.0 < *previous_end {
                    if edit.1 <= *previous_end {
                        continue;
                    }
                    return Err(anyhow!(
                        "grounding repair contains partially overlapping exact phrases in one text target"
                    ));
                }
            }
            disjoint_edits.push(edit);
        }
        let article = draft
            .articles
            .get_mut(article_index)
            .ok_or_else(|| anyhow!("grounding repair names an invalid article"))?;
        let target = grounding_edit_target(article, &field, paragraph_index)?;
        for (mut start, mut end, _, replacement) in disjoint_edits.into_iter().rev() {
            if replacement.is_empty() {
                let bytes = target.as_bytes();
                if start > 0
                    && bytes[start - 1] == b' '
                    && (end == bytes.len() || bytes[end] == b' ')
                {
                    start -= 1;
                } else if start == 0 && end < bytes.len() && bytes[end] == b' ' {
                    end += 1;
                }
            }
            target.replace_range(start..end, &replacement);
        }
    }
    if addressed != expected_findings {
        return Err(anyhow!(
            "grounding repair must address the complete current finding set in one bounded pass"
        ));
    }

    let lead_deleted = deletions.contains(&0);
    for index in deletions.into_iter().rev() {
        draft.articles.remove(index);
    }
    if lead_deleted {
        draft.articles[0].section = "Front Page".into();
    }
    Ok(draft)
}

fn grounding_edit_target<'a>(
    article: &'a mut EditorialArticleDraft,
    field: &GroundingEditableField,
    paragraph_index: Option<u16>,
) -> Result<&'a mut String> {
    match (field, paragraph_index) {
        (GroundingEditableField::Headline, None) => Ok(&mut article.headline),
        (GroundingEditableField::Deck, None) => Ok(&mut article.deck),
        (GroundingEditableField::Dateline, None) => Ok(&mut article.dateline),
        (GroundingEditableField::Paragraph, Some(index)) => article
            .paragraphs
            .get_mut(usize::from(index))
            .ok_or_else(|| anyhow!("grounding repair names an invalid paragraph")),
        _ => Err(anyhow!(
            "paragraph_index is required only for paragraph replacements"
        )),
    }
}

struct ResolvedGroundingFinding {
    article_index: usize,
    field: GroundingEditableField,
    paragraph_index: Option<u16>,
    start: usize,
    end: usize,
}

fn resolve_grounding_finding_target(
    draft: &EditorialPageDraft,
    finding: &WorldNewspaperGroundingFinding,
) -> Result<ResolvedGroundingFinding> {
    let article_index = usize::from(finding.article_index);
    let article = draft
        .articles
        .get(article_index)
        .ok_or_else(|| anyhow!("copy desk returned an invalid finding"))?;
    let mut matches = Vec::new();
    for (field, paragraph_index, target) in [
        (GroundingEditableField::Headline, None, &article.headline),
        (GroundingEditableField::Deck, None, &article.deck),
        (GroundingEditableField::Dateline, None, &article.dateline),
    ]
    .into_iter()
    .chain(
        article
            .paragraphs
            .iter()
            .enumerate()
            .map(|(index, paragraph)| {
                (
                    GroundingEditableField::Paragraph,
                    Some(u16::try_from(index).expect("bounded newspaper paragraph index")),
                    paragraph,
                )
            }),
    ) {
        matches.extend(
            target
                .match_indices(&finding.claim_or_phrase)
                .map(|(start, _)| (field.clone(), paragraph_index, start)),
        );
    }
    if matches.len() != 1 {
        return Err(anyhow!(
            "copy-desk claim_or_phrase must be one exact contiguous phrase occurring once in the named article"
        ));
    }
    let (field, paragraph_index, start) = matches.remove(0);
    Ok(ResolvedGroundingFinding {
        article_index,
        field,
        paragraph_index,
        start,
        end: start + finding.claim_or_phrase.len(),
    })
}

#[allow(clippy::too_many_arguments)]
async fn run_grounding_reconciliation_step(
    model: &dyn ModelPort,
    records: &[PublicRecordProjection],
    max_articles: usize,
    binding: &str,
    source_json: &str,
    source_receipt_ids: &[String],
    reconciliation_receipts: &[ModelStageReceipt],
    checkpoint_id: &str,
    draft: EditorialPageDraft,
    verdict: WorldNewspaperGroundingVerdict,
) -> std::result::Result<
    (
        ModelAgentProgress<GroundingReconciliationOutput>,
        EditorialPageDraft,
        WorldNewspaperGroundingVerdict,
    ),
    crate::agent::ModelAgentFailure,
> {
    let indexed_findings = grounding_finding_catalog(&verdict);
    let instructions = format!(
        "You are Ghostlight's grounding reconciliation agent. Repair one already-edited newspaper page against the same frozen source desk and exact rejection. You do not report new events, add spice, improve the simulation, or rerun the editor. The workbench freezes article selection, order, sections, bylines, citations, and all unaffected copy. Submit only replacements for the numbered current findings, or delete the article containing a numbered finding when its cited notes cannot support a grounded story. Each replacement selects one finding_ref and supplies only its replacement text. The deterministic workbench owns the selected finding's article, text field, paragraph, exact phrase, and byte span. replacement may be empty when phrase deletion is the honest repair. Address the complete finding set in this one pass. When exact findings are nested in one text field, still answer every finding_ref; the workbench deterministically applies the outermost replacement because it owns the complete affected span and treats contained replacements as covered. Preserve source status exactly: an attempt is not a result, a committed course does not complete its embedded plans, and a public declaration is not evidence that its demand succeeded. A named person's supported_identity_attributes is exhaustive; when it is empty, use the name or identity-neutral wording and do not invent pronouns, gender, title, kinship, or office. Delete an unsupported phrase rather than replacing it with an adjacent invention. The deterministic workbench applies the edits transactionally and reruns the same whole-page copy desk.\n\nFROZEN NEWSROOM FACT DESK:\n{}\n\nCURRENT CHECKPOINTED DRAFT:\n{}\n\nCURRENT REJECTION VERDICT:\n{}\n\nNUMBERED EXACT FINDINGS:\n{}",
        source_json,
        serde_json::to_string_pretty(&draft).unwrap_or_default(),
        serde_json::to_string_pretty(&verdict).unwrap_or_default(),
        serde_json::to_string_pretty(&indexed_findings).unwrap_or_default(),
    );
    let mut causal_sources = source_receipt_ids
        .iter()
        .cloned()
        .chain(
            reconciliation_receipts
                .iter()
                .map(|receipt| receipt.storage_key().to_owned()),
        )
        .chain(std::iter::once(checkpoint_id.to_owned()))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    causal_sources.sort();
    let spec = ModelAgentSpec {
        stage: "newspaper_grounding_reconciliation_agent_action".into(),
        model: MODEL_BALANCED.into(),
        snapshot_binding: format!("{binding}:grounding-reconciliation"),
        instructions,
        source_receipt_ids: causal_sources,
        temperature: Some(0.1),
        max_output_tokens: Some(1_200),
        max_steps: 1,
    };
    let mut tool = GroundingReconciliationWorkbench {
        model,
        records,
        max_articles,
        binding,
        source_json,
        source_receipt_ids,
        draft,
        verdict,
    };
    let progress = crate::agent::run_model_agent_progress(model, &spec, &mut tool).await?;
    Ok((progress, tool.draft, tool.verdict))
}

async fn run_copy_desk(
    model: &dyn ModelPort,
    snapshot_binding: String,
    source_json: &str,
    source_receipt_ids: &[String],
    editorial_source_receipt_ids: &[String],
    draft: &EditorialPageDraft,
    receipts: &mut Vec<ModelStageReceipt>,
) -> Result<WorldNewspaperGroundingVerdict> {
    let verifier_schema = serde_json::to_value(schema_for!(GroundingVerdictDraft))?;
    let verifier_request = ModelStageRequest {
        stage: "newspaper_copy_desk".into(),
        model: MODEL_CAPABLE.into(),
        snapshot_binding,
        lived_stream: format!(
            "OUTPUT JSON SCHEMA (follow exactly):\n{}\n\nAct as a strict copy desk, not a rewriting model. Compare every reader-facing factual claim in the proposed fantasy newspaper page with only its cited notes in the bounded newsroom fact desk. Reject invented or overconfident facts, quotations, identities, offices, places, numbers, motives, outcomes, chronology, or private knowledge. Treat each fact's assertion_status as an exhaustive boundary: attempt_committed_outcome_unknown never supports completion; course_committed_embedded_actions_not_completed supports the adopted course but not completion of acts named inside it; public_declaration supports only the declaration; material_change_committed supports the stated material change. Treat each named person's supported_identity_attributes as exhaustive. If it is empty, reject pronouns, gender, title, kinship, or office not independently stated in that fact, and require the name or identity-neutral wording. When cited notes dispute a document, accusation, identity, outcome, or authority, require explicit attribution or qualified language; one public claim does not silently settle another note's dispute. Distinguish an institution publishing a notice about evidence from displaying, releasing, or publishing the physical objects themselves. Reject copy that exposes the fact desk, citations, verification work, or state transitions instead of reporting news.\n\nThe five allowed generic bylines are publication role labels supplied by the newspaper contract, not claims about new people, witnesses, or reporting acts; never reject an allowed byline for lacking source evidence. Metaphor, dry wit, rhetorical contrast, plainly signalled opinion, and political characterization are editorial language rather than world facts when they introduce no concrete entity, occurrence, status, motive, quotation, number, or private knowledge. Do not demand a source sentence for such language. Still reject a rhetorical phrase when it smuggles in a concrete outcome, such as treating a proposed kiln closure as completed, or turns missing evidence into proof that something did not happen. Return the complete bounded finding set in one verdict rather than revealing one defect per pass. Every claim_or_phrase must copy one exact contiguous reader-facing substring that occurs exactly once in its named article; never paraphrase the defect in that field. A neutral contract-owned edition label is not part of the proposed model copy. `accepted` may be true only when findings is empty. Return findings only; never propose replacement copy.\n\nNEWSROOM FACT DESK:\n{}\n\nPROPOSED PAGE:\n{}",
            serde_json::to_string(&verifier_schema)?,
            source_json,
            serde_json::to_string_pretty(draft)?,
        ),
        output_schema: Some(verifier_schema),
        source_receipt_ids: source_receipt_ids
            .iter()
            .cloned()
            .chain(editorial_source_receipt_ids.iter().cloned())
            .collect(),
        temperature: Some(0.0),
        max_output_tokens: Some(1_500),
    };
    let verifier_output = run_validated_stage(model, &verifier_request)
        .await
        .map_err(|error| anyhow!("newspaper copy-desk inference failed: {error}"))?;
    receipts.push(verifier_output.receipt);
    let receipt_index = receipts.len() - 1;
    let verifier_structured = verifier_output
        .structured
        .ok_or_else(|| anyhow!("newspaper copy desk returned no structured output"));
    let verifier_structured = match verifier_structured {
        Ok(structured) => structured,
        Err(error) => {
            mark_semantic_invalid(&mut receipts[receipt_index], &error);
            return Err(error);
        }
    };
    let verdict_draft: GroundingVerdictDraft = match serde_json::from_value(verifier_structured) {
        Ok(verdict) => verdict,
        Err(error) => {
            let error = anyhow!("newspaper copy desk returned an invalid verdict: {error}");
            mark_semantic_invalid(&mut receipts[receipt_index], &error);
            return Err(error);
        }
    };
    if let Err(error) = validate_grounding_verdict(draft, &verdict_draft) {
        mark_semantic_invalid(&mut receipts[receipt_index], &error);
        return Err(error);
    }
    Ok(WorldNewspaperGroundingVerdict {
        accepted: verdict_draft.accepted,
        assessment: verdict_draft.assessment,
        findings: verdict_draft.findings,
    })
}

pub fn render_world_newspaper_markdown(issue: &WorldNewspaperIssue) -> String {
    let mut rendered = format!(
        "# {}\n\n*{}*\n\n---\n",
        escape_markdown_text(&issue.title),
        escape_markdown_text(&issue.edition_label)
    );
    for (index, article) in issue.articles.iter().enumerate() {
        if index > 0 {
            rendered.push_str(&format!(
                "\n---\n\n### {}\n",
                escape_markdown_text(&article.section)
            ));
        }
        rendered.push_str(&format!(
            "\n## {}\n\n",
            escape_markdown_text(&article.headline)
        ));
        rendered.push_str(&format!("*{}*\n\n", escape_markdown_text(&article.deck)));
        rendered.push_str(&format!(
            "**{}**\n\n",
            escape_markdown_text(&article.byline)
        ));
        for (paragraph_index, paragraph) in article.paragraphs.iter().enumerate() {
            if paragraph_index == 0
                && let Some(dateline) = &article.dateline
            {
                rendered.push_str(&format!(
                    "**{} —** {}\n\n",
                    escape_markdown_text(dateline),
                    escape_markdown_text(paragraph)
                ));
            } else {
                rendered.push_str(&escape_markdown_text(paragraph));
                rendered.push_str("\n\n");
            }
        }
    }
    rendered.trim_end().to_owned() + "\n"
}

fn escape_markdown_text(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    let characters = value.chars().collect::<Vec<_>>();
    let line_prefix_escape = markdown_line_prefix_escape(&characters);
    for (index, character) in characters.into_iter().enumerate() {
        if line_prefix_escape == Some(index) {
            escaped.push('\\');
        }
        match character {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '\\' | '`' | '*' | '_' | '{' | '}' | '[' | ']' | '#' | '|' | '~' => {
                escaped.push('\\');
                escaped.push(character);
            }
            _ => escaped.push(character),
        }
    }
    escaped
}

fn markdown_line_prefix_escape(characters: &[char]) -> Option<usize> {
    if matches!(characters.first(), Some('-' | '+')) {
        return Some(0);
    }
    let digit_count = characters
        .iter()
        .take(9)
        .take_while(|character| character.is_ascii_digit())
        .count();
    if digit_count > 0
        && matches!(characters.get(digit_count), Some('.' | ')'))
        && characters
            .get(digit_count + 1)
            .is_some_and(|character| character.is_whitespace())
    {
        return Some(digit_count);
    }
    None
}

pub fn render_world_newspaper_audit_markdown(issue: &WorldNewspaperIssue) -> String {
    let mut rendered = format!(
        "# Editorial provenance: {}\n\n- Source world revision: {}\n- Editorial receipts: {}\n",
        escape_markdown_text(&issue.title),
        issue.source_world_revision,
        escaped_join(&issue.editorial_receipt_ids)
    );
    if let Some(agenda) = &issue.editorial_agenda {
        rendered.push_str(&format!(
            "\n## Editorial agenda\n\n- Dominant throughline: {}\n- Reader stake: {}\n",
            escape_markdown_text(&agenda.dominant_throughline),
            escape_markdown_text(&agenda.reader_stake),
        ));
        for (index, pitch) in agenda.story_pitches.iter().enumerate() {
            rendered.push_str(&format!(
                "\n### Pitch {}{}\n\n- Public records: {}\n- Focus record: {}\n- Narrative claim: {}\n- Tension: {}\n- Public question: {}\n",
                index + 1,
                if pitch.lead { " (lead)" } else { "" },
                escaped_join(&pitch.citations),
                escape_markdown_text(&pitch.focus_citation),
                escape_markdown_text(&pitch.narrative_claim),
                escape_markdown_text(&pitch.tension),
                escape_markdown_text(&pitch.public_question),
            ));
        }
    }
    for article in &issue.articles {
        rendered.push_str(&format!(
            "\n## {}\n\n- Article ID: {}\n",
            escape_markdown_text(&article.headline),
            escape_markdown_text(&article.id),
        ));
        for source in &article.sources {
            rendered.push_str(&format!(
                "\n### Citation {}\n\n- Source news: {}\n- Source channels: {}\n- Source reliability: {}\n",
                escape_markdown_text(&source.citation),
                escaped_join(&source.source_news_ids),
                escaped_join(&source.source_channels),
                escaped_join(&source.source_reliability),
            ));
            for (fact_index, fact) in source.facts.iter().enumerate() {
                let named_people = if fact.named_people.is_empty() {
                    "None asserted".into()
                } else {
                    fact.named_people
                        .iter()
                        .map(|person| {
                            let attributes = if person.supported_identity_attributes.is_empty() {
                                "none asserted".into()
                            } else {
                                escaped_join(&person.supported_identity_attributes)
                            };
                            format!(
                                "{} (supported identity attributes: {})",
                                escape_markdown_text(&person.name),
                                attributes
                            )
                        })
                        .collect::<Vec<_>>()
                        .join(", ")
                };
                rendered.push_str(&format!(
                    "\n#### Fact {}\n\n- Exact committed account: {}\n- Assertion status: {}\n- Committed events: {}\n- Named people: {}\n- Institutions: {}\n- Populations: {}\n- Places: {}\n",
                    fact_index + 1,
                    escape_markdown_text(&fact.account),
                    serde_json::to_value(&fact.assertion_status)
                        .ok()
                        .and_then(|value| value.as_str().map(str::to_owned))
                        .unwrap_or_else(|| "invalid_assertion_status".into()),
                    escaped_join(&fact.event_ids),
                    named_people,
                    escaped_or_none(&fact.institutions),
                    escaped_or_none(&fact.populations),
                    escaped_or_none(&fact.places),
                ));
            }
        }
    }
    rendered
}

fn escaped_or_none(values: &[String]) -> String {
    if values.is_empty() {
        "None asserted".into()
    } else {
        escaped_join(values)
    }
}

fn escaped_join(values: &[String]) -> String {
    values
        .iter()
        .map(|value| escape_markdown_text(value))
        .collect::<Vec<_>>()
        .join(", ")
}

fn public_news_records(campaign: &Campaign) -> Result<Vec<PublicRecordProjection>> {
    let events = campaign
        .events
        .iter()
        .map(|event| (event.id.as_str(), event))
        .collect::<BTreeMap<_, _>>();
    let mut news = campaign.news.iter().collect::<Vec<_>>();
    news.sort_by(|left, right| right.at.cmp(&left.at).then_with(|| left.id.cmp(&right.id)));
    let mut record_ids = BTreeSet::new();
    for issue in &news {
        if !record_ids.insert(issue.id.as_str()) {
            return Err(anyhow!("public news ledger repeats record ID {}", issue.id));
        }
    }
    news.into_iter()
        .map(|issue| public_news_record(campaign, issue, &events))
        .collect()
}

fn public_news_record(
    campaign: &Campaign,
    issue: &NewsIssue,
    events: &BTreeMap<&str, &Event>,
) -> Result<PublicRecordProjection> {
    if issue.event_ids.is_empty() {
        return Err(anyhow!("news item {} has no committed event", issue.id));
    }
    let facts = issue
        .event_ids
        .iter()
        .map(|event_id| {
            let event = events
                .get(event_id.as_str())
                .copied()
                .ok_or_else(|| anyhow!("news item {} cites unknown event {event_id}", issue.id))?;
            if !event.public_channels.contains(&issue.channel) {
                return Err(anyhow!(
                    "news item {} uses channel absent from event {event_id}",
                    issue.id
                ));
            }
            let summary = event.summary.trim().to_owned();
            Ok(WorldNewspaperSourceFact {
                event_ids: vec![event.id.clone()],
                assertion_status: event.public_assertion_status(),
                account: summary.clone(),
                named_people: event
                    .actor_ids
                    .iter()
                    .filter_map(|id| campaign.actors.get(id))
                    .filter(|actor| summary_mentions_name(&summary, &actor.name))
                    .map(|actor| WorldNewspaperNamedPerson {
                        name: actor.name.clone(),
                        supported_identity_attributes: Vec::new(),
                    })
                    .collect(),
                institutions: event
                    .institution_ids
                    .iter()
                    .filter_map(|id| campaign.institutions.get(id))
                    .filter(|institution| summary_mentions_name(&summary, &institution.name))
                    .map(|institution| institution.name.clone())
                    .collect(),
                populations: event
                    .gestalt_ids
                    .iter()
                    .filter_map(|id| campaign.gestalts.get(id))
                    .filter(|gestalt| summary_mentions_name(&summary, &gestalt.name))
                    .map(|gestalt| gestalt.name.clone())
                    .collect(),
                places: event
                    .location_ids
                    .iter()
                    .filter_map(|id| campaign.locations.get(id))
                    .filter(|place| {
                        event.location_ids.len() == 1
                            || summary_mentions_name(&summary, &place.name)
                    })
                    .map(|place| place.name.clone())
                    .collect(),
            })
        })
        .collect::<Result<Vec<_>>>()?;
    if facts.iter().any(|fact| fact.account.is_empty()) {
        return Err(anyhow!("news item {} cites an empty event", issue.id));
    }
    Ok(PublicRecordProjection {
        record_id: issue.id.clone(),
        at: issue.at,
        channel: issue.channel.clone(),
        headline: issue.headline.clone(),
        reliability: issue.reliability.clone(),
        facts,
    })
}

fn summary_mentions_name(summary: &str, name: &str) -> bool {
    let summary = summary.to_lowercase();
    let name = name.to_lowercase();
    if name.is_empty() {
        return false;
    }
    summary.match_indices(&name).any(|(start, matched)| {
        let before = summary[..start].chars().next_back();
        let after = summary[start + matched.len()..].chars().next();
        before.is_none_or(|character| !character.is_alphanumeric())
            && after.is_none_or(|character| !character.is_alphanumeric())
    })
}

fn public_records_for_ids<'a>(
    records: &'a [PublicRecordProjection],
    record_ids: &BTreeSet<String>,
) -> Vec<&'a PublicRecordProjection> {
    records
        .iter()
        .filter(|record| record_ids.contains(&record.record_id))
        .collect()
}

fn source_json_for_agenda(
    records: &[PublicRecordProjection],
    agenda: Option<&WorldNewspaperEditorialAgenda>,
) -> Result<String> {
    let record_ids = agenda.map_or_else(
        || {
            records
                .iter()
                .map(|record| record.record_id.clone())
                .collect()
        },
        |agenda| {
            agenda
                .story_pitches
                .iter()
                .flat_map(|pitch| pitch.citations.iter().cloned())
                .collect()
        },
    );
    Ok(serde_json::to_string_pretty(&public_records_for_ids(
        records,
        &record_ids,
    ))?)
}

fn editorial_schema(
    records: &[PublicRecordProjection],
    agenda: &WorldNewspaperEditorialAgenda,
) -> Result<serde_json::Value> {
    let mut schema = serde_json::to_value(schema_for!(EditorialPageDraft))?;
    let selected_citations = agenda
        .story_pitches
        .iter()
        .flat_map(|pitch| pitch.citations.iter().cloned())
        .collect::<BTreeSet<_>>();
    let citations = selected_citations.iter().cloned().collect::<Vec<_>>();
    let mut datelines = records
        .iter()
        .filter(|record| selected_citations.contains(&record.record_id))
        .flat_map(|record| record.facts.iter())
        .flat_map(|fact| fact.places.iter().cloned())
        .collect::<BTreeSet<_>>();
    datelines.insert(String::new());
    *schema
        .pointer_mut("/properties/articles/maxItems")
        .ok_or_else(|| anyhow!("editorial schema omitted article budget"))? =
        agenda.story_pitches.len().into();
    *schema
        .pointer_mut("/properties/articles/minItems")
        .ok_or_else(|| anyhow!("editorial schema omitted minimum article count"))? =
        agenda.story_pitches.len().into();
    *schema
        .pointer_mut("/$defs/EditorialArticleDraft/properties/citations/items")
        .ok_or_else(|| anyhow!("editorial schema omitted citation items"))? =
        serde_json::json!({"type":"string","enum":citations});
    if let Some(citation_array) = schema
        .pointer_mut("/$defs/EditorialArticleDraft/properties/citations")
        .and_then(serde_json::Value::as_object_mut)
    {
        citation_array.insert("uniqueItems".into(), true.into());
    }
    *schema
        .pointer_mut("/$defs/EditorialArticleDraft/properties/dateline")
        .ok_or_else(|| anyhow!("editorial schema omitted dateline"))? =
        serde_json::json!({"type":"string","enum":datelines});
    *schema
        .pointer_mut("/$defs/EditorialArticleDraft/properties/section")
        .ok_or_else(|| anyhow!("editorial schema omitted section"))? =
        serde_json::json!({"type":"string","enum":ALLOWED_SECTIONS});
    *schema
        .pointer_mut("/$defs/EditorialArticleDraft/properties/byline")
        .ok_or_else(|| anyhow!("editorial schema omitted byline"))? =
        serde_json::json!({"type":"string","enum":ALLOWED_BYLINES});
    Ok(schema)
}

fn validate_editorial_draft(
    records: &[PublicRecordProjection],
    draft: &EditorialPageDraft,
    max_articles: usize,
) -> Result<()> {
    if draft.articles.is_empty() || draft.articles.len() > max_articles.min(records.len()) {
        return Err(anyhow!("editorial page exceeded its story budget"));
    }
    let known_sources = records
        .iter()
        .map(|record| record.record_id.as_str())
        .collect::<BTreeSet<_>>();
    let source_datelines = records
        .iter()
        .map(|record| {
            (
                record.record_id.as_str(),
                record
                    .facts
                    .iter()
                    .flat_map(|fact| fact.places.iter().map(String::as_str))
                    .collect::<BTreeSet<_>>(),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let source_summaries = records
        .iter()
        .map(|record| {
            (
                record.record_id.as_str(),
                record
                    .facts
                    .iter()
                    .map(|fact| fact.account.trim())
                    .collect::<Vec<_>>(),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let mut headlines = BTreeSet::new();
    for (index, article) in draft.articles.iter().enumerate() {
        if (index == 0 && article.section != "Front Page")
            || (index > 0 && article.section == "Front Page")
        {
            return Err(anyhow!(
                "only the lead article may use the Front Page section"
            ));
        }
        if !ALLOWED_SECTIONS.contains(&article.section.as_str())
            || !ALLOWED_BYLINES.contains(&article.byline.as_str())
        {
            return Err(anyhow!(
                "article {index} used an unowned presentation label"
            ));
        }
        validate_single_line(&article.headline, 100, "headline")?;
        validate_single_line(&article.deck, 220, "deck")?;
        validate_single_line(&article.byline, 60, "byline")?;
        if article.headline.ends_with('…') || article.headline.ends_with("...") {
            return Err(anyhow!("article {index} headline ends in truncation"));
        }
        if article.headline.eq_ignore_ascii_case(&article.deck) {
            return Err(anyhow!("article {index} deck repeats its headline"));
        }
        if !headlines.insert(article.headline.to_lowercase()) {
            return Err(anyhow!("front page repeats a headline"));
        }
        let selected = selected_record_ids(&article.citations, &format!("article {index}"))?;
        for citation in selected {
            if !known_sources.contains(citation) {
                return Err(anyhow!(
                    "article {index} cites unknown public record {citation}"
                ));
            }
        }
        let cited_dateline_supported = article.citations.iter().any(|citation| {
            source_datelines[citation.as_str()].contains(article.dateline.as_str())
        });
        let cited_place_available = article
            .citations
            .iter()
            .any(|citation| !source_datelines[citation.as_str()].is_empty());
        if !article.dateline.is_empty() && !cited_dateline_supported {
            return Err(anyhow!(
                "article {index} invented or misattributed a dateline"
            ));
        }
        if index == 0 && article.dateline.is_empty() && cited_place_available {
            return Err(anyhow!(
                "the lead article must use a cited source place as its dateline"
            ));
        }
        if !(2..=5).contains(&article.paragraphs.len()) {
            return Err(anyhow!(
                "article {index} must contain two to five paragraphs"
            ));
        }
        for paragraph in &article.paragraphs {
            let text = paragraph.trim();
            if text != paragraph
                || text.chars().count() < 40
                || text.chars().count() > 900
                || text.contains(['\r', '\n'])
            {
                return Err(anyhow!("article {index} contains a malformed paragraph"));
            }
            if article.citations.iter().any(|citation| {
                source_summaries[citation.as_str()]
                    .iter()
                    .any(|summary| *summary == text)
            }) {
                return Err(anyhow!(
                    "article {index} printed a source summary as final copy"
                ));
            }
        }
        let reader_copy = format!(
            "{} {} {} {} {}",
            article.headline,
            article.deck,
            article.byline,
            article.dateline,
            article.paragraphs.join(" ")
        );
        validate_no_reader_plumbing(&reader_copy, &format!("article {index}"))?;
    }
    Ok(())
}

fn validate_no_reader_plumbing(value: &str, label: &str) -> Result<()> {
    let reader_copy = value.to_lowercase();
    for leaked in [
        "world revision",
        "event:",
        "strategic:",
        "committed public channel",
        "selected action",
        "supplied gestalt",
        "resolution cover",
        "source_news_id",
        "source news id",
    ] {
        if reader_copy.contains(leaked) {
            return Err(anyhow!("{label} leaked newsroom plumbing: {leaked}"));
        }
    }
    Ok(())
}

fn validate_single_line(value: &str, max_chars: usize, label: &str) -> Result<()> {
    if value.trim().is_empty()
        || value.trim() != value
        || value.chars().count() > max_chars
        || value.contains(['\r', '\n'])
        || value.contains('`')
        || value.starts_with('#')
    {
        return Err(anyhow!("{label} is not valid reader-facing copy"));
    }
    Ok(())
}

fn validate_grounding_verdict(
    draft: &EditorialPageDraft,
    verdict: &GroundingVerdictDraft,
) -> Result<()> {
    validate_single_line(&verdict.assessment, 500, "copy-desk assessment")?;
    if verdict.accepted != verdict.findings.is_empty() {
        return Err(anyhow!(
            "copy-desk acceptance must be true exactly when findings are empty"
        ));
    }
    let mut exact_claims = BTreeSet::new();
    for finding in &verdict.findings {
        let article_index = usize::from(finding.article_index);
        validate_single_line(&finding.claim_or_phrase, 500, "copy-desk claim")?;
        validate_single_line(&finding.reason, 500, "copy-desk reason")?;
        if !exact_claims.insert((article_index, finding.claim_or_phrase.as_str())) {
            return Err(anyhow!("copy desk repeated one exact finding phrase"));
        }
        resolve_grounding_finding_target(draft, finding)?;
    }
    Ok(())
}

fn lower_editorial_page(
    campaign: &Campaign,
    title: String,
    records: &[PublicRecordProjection],
    editorial_agenda: Option<WorldNewspaperEditorialAgenda>,
    draft: EditorialPageDraft,
    receipts: &[ModelStageReceipt],
) -> Result<WorldNewspaperIssue> {
    let source_map = records
        .iter()
        .map(|record| (record.record_id.as_str(), record))
        .collect::<BTreeMap<_, _>>();
    let selected_record_ids = draft
        .articles
        .iter()
        .flat_map(|article| article.citations.iter().cloned())
        .collect::<BTreeSet<_>>();
    let citation_labels = records
        .iter()
        .filter(|record| selected_record_ids.contains(&record.record_id))
        .enumerate()
        .map(|(index, record)| (record.record_id.as_str(), (index + 1).to_string()))
        .collect::<BTreeMap<_, _>>();
    let mut articles = Vec::with_capacity(draft.articles.len());
    for (index, article) in draft.articles.into_iter().enumerate() {
        let selected_records = article
            .citations
            .iter()
            .map(|record_id| {
                source_map
                    .get(record_id.as_str())
                    .copied()
                    .ok_or_else(|| anyhow!("editorial lowering lost public record {record_id}"))
            })
            .collect::<Result<Vec<_>>>()?;
        let audit_sources = selected_records
            .iter()
            .map(|record| WorldNewspaperSourceCitation {
                citation: citation_labels[record.record_id.as_str()].clone(),
                source_news_ids: vec![record.record_id.clone()],
                source_channels: vec![record.channel.clone()],
                source_reliability: vec![record.reliability.clone()],
                facts: record.facts.clone(),
            })
            .collect::<Vec<_>>();
        let identity = rmp_serde::to_vec_named(&(
            NEWSROOM_CONTRACT_VERSION,
            campaign.id,
            campaign.revision,
            index,
            &article.section,
            &article.headline,
            &article.deck,
            &article.byline,
            &article.dateline,
            &article.paragraphs,
            &audit_sources,
        ))?;
        articles.push(WorldNewspaperArticle {
            id: format!("article:sha256:{:x}", Sha256::digest(identity)),
            section: article.section,
            headline: article.headline,
            deck: article.deck,
            byline: article.byline,
            dateline: (!article.dateline.is_empty()).then_some(article.dateline),
            paragraphs: article.paragraphs,
            sources: audit_sources,
        });
    }
    let receipt_ids = receipts
        .iter()
        .map(|receipt| receipt.storage_key().to_owned())
        .collect::<Vec<_>>();
    let at = records
        .iter()
        .map(|record| record.at)
        .max()
        .unwrap_or(campaign.world_time);
    let identity = rmp_serde::to_vec_named(&(
        campaign.id,
        campaign.revision,
        &title,
        EDITION_LABEL,
        &editorial_agenda,
        articles
            .iter()
            .map(|article| &article.id)
            .collect::<Vec<_>>(),
        &receipt_ids,
    ))?;
    Ok(WorldNewspaperIssue {
        schema: "ghostlight.world_newspaper_issue.v3".into(),
        id: format!("newspaper:sha256:{:x}", Sha256::digest(identity)),
        title,
        edition_label: EDITION_LABEL.into(),
        at,
        source_world_revision: campaign.revision,
        lead_article_id: articles.first().map(|article| article.id.clone()),
        editorial_agenda,
        articles,
        editorial_receipt_ids: receipt_ids,
    })
}

fn publication_task_binding(
    campaign: &Campaign,
    title: &str,
    voice: &str,
    max_articles: usize,
) -> Result<String> {
    let bytes = rmp_serde::to_vec_named(&(
        NEWSROOM_CONTRACT_VERSION,
        campaign.id,
        campaign.revision,
        title,
        voice,
        max_articles,
    ))?;
    Ok(format!(
        "campaign:{}:revision:{}:newspaper-task:sha256:{:x}",
        campaign.id,
        campaign.revision,
        Sha256::digest(bytes)
    ))
}

fn editorial_binding(
    campaign: &Campaign,
    title: &str,
    voice: &str,
    max_articles: usize,
    records: &[PublicRecordProjection],
) -> Result<String> {
    let bytes = rmp_serde::to_vec_named(&(
        NEWSROOM_CONTRACT_VERSION,
        campaign.id,
        campaign.revision,
        campaign.world_time,
        title,
        voice,
        max_articles,
        records,
    ))?;
    Ok(format!(
        "campaign:{}:revision:{}:newspaper:sha256:{:x}",
        campaign.id,
        campaign.revision,
        Sha256::digest(bytes)
    ))
}

fn empty_issue_id(campaign: &Campaign, title: &str) -> Result<String> {
    let identity = rmp_serde::to_vec_named(&(
        NEWSROOM_CONTRACT_VERSION,
        campaign.id,
        campaign.revision,
        title,
        "no-edition",
    ))?;
    Ok(format!("newspaper:sha256:{:x}", Sha256::digest(identity)))
}

fn mark_semantic_invalid(receipt: &mut ModelStageReceipt, error: &anyhow::Error) {
    let error: String = error.to_string().chars().take(1_000).collect();
    let original_binding = receipt.snapshot_binding.clone();
    let source_chain_digest = Sha256::digest(
        rmp_serde::to_vec_named(&receipt.source_receipt_ids)
            .expect("model receipt source chains must be serializable"),
    );
    receipt.rebind_snapshot(format!(
        "{original_binding}:semantic-invalid:sources:sha256:{source_chain_digest:x}:error:sha256:{:x}",
        Sha256::digest(error.as_bytes()),
    ));
    receipt.validation_result = "semantic_invalid".into();
    receipt.local_validation_error = Some(error);
}

fn composition_failure(
    message: impl Into<String>,
    model_receipts: Vec<ModelStageReceipt>,
) -> anyhow::Error {
    anyhow::Error::new(WorldNewspaperCompositionFailure {
        message: message.into(),
        model_receipts,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use std::{collections::VecDeque, sync::Mutex};

    struct ScriptedNewspaperModel {
        responses: Mutex<VecDeque<String>>,
        requests: Mutex<Vec<ModelStageRequest>>,
    }

    impl ScriptedNewspaperModel {
        fn new(responses: impl IntoIterator<Item = &'static str>) -> Self {
            Self {
                responses: Mutex::new(responses.into_iter().map(str::to_owned).collect()),
                requests: Mutex::new(Vec::new()),
            }
        }

        fn requests(&self) -> Vec<ModelStageRequest> {
            self.requests.lock().unwrap().clone()
        }
    }

    #[async_trait]
    impl ModelPort for ScriptedNewspaperModel {
        async fn run(&self, request: &ModelStageRequest) -> Result<String> {
            self.requests.lock().unwrap().push(request.clone());
            let response = self
                .responses
                .lock()
                .unwrap()
                .pop_front()
                .ok_or_else(|| anyhow!("fixture newspaper model exhausted"))?;
            if request.stage == "newspaper_grounding_reconciliation_agent_action"
                && !response.contains("\"tool\"")
            {
                let draft = serde_json::from_str::<EditorialPageDraft>(&response)?;
                return Ok(serde_json::json!({
                    "tool":"submit_revision",
                    "draft":draft,
                })
                .to_string());
            }
            Ok(response)
        }

        fn provider(&self) -> &'static str {
            "newspaper-fixture"
        }
    }

    fn campaign_with_news() -> Campaign {
        let mut campaign = crate::kernel::tests::campaign();
        campaign.revision = 3;
        let summary = "The Thorn Court admits its royal seal was pawned to pay a dragon's gambling debt, then dismisses the treasurer who carried the confession into open court.";
        campaign.events.push(Event {
            id: "event:seal-scandal".into(),
            at: campaign.world_time,
            kind: "institution_action".into(),
            summary: summary.into(),
            actor_ids: vec![],
            institution_ids: vec![],
            gestalt_ids: vec![],
            location_ids: vec!["room".into()],
            public_channels: vec!["court broadsheet".into()],
        });
        campaign.news.push(NewsIssue {
            id: "news:seal-scandal".into(),
            at: campaign.world_time,
            channel: "court broadsheet".into(),
            headline: crate::domain::committed_news_headline(summary),
            event_ids: vec!["event:seal-scandal".into()],
            reliability: "committed public channel".into(),
        });
        campaign
    }

    fn campaign_with_two_news() -> Campaign {
        let mut campaign = campaign_with_news();
        campaign.locations.insert(
            "yard".into(),
            crate::domain::Location {
                id: "yard".into(),
                name: "Yard".into(),
                container_id: None,
                routes: BTreeMap::new(),
                persistent_features: vec![],
            },
        );
        let summary = "The palace bell keeper announces that the west gate will close at moonrise while masons replace its cracked hinge.";
        campaign.events.push(Event {
            id: "event:west-gate".into(),
            at: campaign.world_time,
            kind: "institution_action".into(),
            summary: summary.into(),
            actor_ids: vec![],
            institution_ids: vec![],
            gestalt_ids: vec![],
            location_ids: vec!["yard".into()],
            public_channels: vec!["court broadsheet".into()],
        });
        campaign.news.push(NewsIssue {
            id: "news:west-gate".into(),
            at: campaign.world_time,
            channel: "court broadsheet".into(),
            headline: crate::domain::committed_news_headline(summary),
            event_ids: vec!["event:west-gate".into()],
            reliability: "committed public channel".into(),
        });
        campaign
    }

    fn campaign_with_archive_news() -> Campaign {
        let mut campaign = crate::kernel::tests::campaign();
        campaign.revision = 45;
        for index in 0..45 {
            let at = campaign.world_time + chrono::Duration::minutes(index);
            let summary = if index == 0 {
                "The oldest public dossier records the speaking child, severed delegation hands, failed grain pumps, a legion refusal, and occupied toll bridges."
                    .to_owned()
            } else {
                format!(
                    "Public aftermath dossier {index} records one distinct administrative response."
                )
            };
            let event_id = format!("event:archive:{index:02}");
            campaign.events.push(Event {
                id: event_id.clone(),
                at,
                kind: "public_notice".into(),
                summary: summary.clone(),
                actor_ids: vec![],
                institution_ids: vec![],
                gestalt_ids: vec![],
                location_ids: vec!["room".into()],
                public_channels: vec!["court broadsheet".into()],
            });
            campaign.news.push(NewsIssue {
                id: format!("news:archive:{index:02}"),
                at,
                channel: "court broadsheet".into(),
                headline: crate::domain::committed_news_headline(&summary),
                event_ids: vec![event_id],
                reliability: "committed public channel".into(),
            });
        }
        campaign
    }

    fn campaign_with_typed_and_duplicate_news() -> Campaign {
        let mut campaign = campaign_with_news();
        let duplicate = campaign.events.last().unwrap().clone();
        campaign.events.push(Event {
            id: "strategic:3:activity-outcome:relation-delta".into(),
            at: campaign.world_time,
            kind: "strategic_activity_outcome".into(),
            summary:
                "An action by the court strains the command tie between the Crown and its auditors."
                    .into(),
            actor_ids: vec![],
            institution_ids: vec![],
            gestalt_ids: vec![],
            location_ids: vec!["room".into()],
            public_channels: vec!["court broadsheet".into()],
        });
        campaign.news.push(NewsIssue {
            id: "news:relation-delta".into(),
            at: campaign.world_time,
            channel: "court broadsheet".into(),
            headline: "Relation delta".into(),
            event_ids: vec!["strategic:3:activity-outcome:relation-delta".into()],
            reliability: "committed public channel".into(),
        });
        campaign.events.push(Event {
            id: "strategic:3:gestalt:auditors".into(),
            at: campaign.world_time,
            kind: "gestalt_action".into(),
            summary:
                "The auditors take up a new public demand: replace crown tallies with an elected grain compact."
                    .into(),
            actor_ids: vec![],
            institution_ids: vec![],
            gestalt_ids: vec![],
            location_ids: vec!["room".into()],
            public_channels: vec!["court broadsheet".into()],
        });
        campaign.news.push(NewsIssue {
            id: "news:pressure-transition".into(),
            at: campaign.world_time,
            channel: "court broadsheet".into(),
            headline: "Pressure transition".into(),
            event_ids: vec!["strategic:3:gestalt:auditors".into()],
            reliability: "committed public channel".into(),
        });
        campaign.events.push(Event {
            id: "event:seal-scandal-duplicate".into(),
            ..duplicate
        });
        campaign.news.push(NewsIssue {
            id: "news:seal-scandal-duplicate".into(),
            at: campaign.world_time,
            channel: "court broadsheet".into(),
            headline: "Duplicate desk notice".into(),
            event_ids: vec!["event:seal-scandal-duplicate".into()],
            reliability: "committed public channel".into(),
        });
        campaign
    }

    const QUERY_ALL_RECORDS: &str = r#"{"command":{"tool":"query_public_records","terms":[],"match_terms":"all","entity_names":[],"assertion_statuses":[],"channels":[],"order":"newest","cursor":null,"limit":24}}"#;
    const QUERY_FOUNDING_CRISIS: &str = r#"{"command":{"tool":"query_public_records","terms":["speaking child","severed delegation hands","failed grain pumps"],"match_terms":"any","entity_names":[],"assertion_statuses":[],"channels":[],"order":"oldest","cursor":null,"limit":24}}"#;
    const ONE_STORY_AGENDA: &str = r#"{"command":{"tool":"submit_agenda","dominant_throughline":"The court made its private gambling debt a public crisis of custody and punished the official who exposed it.","reader_stake":"Readers must decide whether dismissal protects the seal or merely the court from its own confession.","story_pitches":[{"lead":true,"citations":["news:seal-scandal"],"focus_citation":"news:seal-scandal","narrative_claim":"The pawned royal seal and the treasurer's dismissal are one scandal.","tension":"The court admits the loss while directing the immediate consequence at the bearer of that admission.","public_question":"Who is being held accountable for the missing seal?"}]}}"#;
    const TWO_STORY_AGENDA: &str = r#"{"command":{"tool":"submit_agenda","dominant_throughline":"Court custody fails at both the royal seal and the western gate.","reader_stake":"Readers depend on institutions that announce damage only after access or authority has already been compromised.","story_pitches":[{"lead":true,"citations":["news:seal-scandal"],"focus_citation":"news:seal-scandal","narrative_claim":"The pawned seal and dismissal are the court's crisis of custody.","tension":"The confession exposes the loss while the treasurer absorbs the immediate institutional consequence.","public_question":"Who is being held accountable for the missing seal?"},{"lead":false,"citations":["news:west-gate"],"focus_citation":"news:west-gate","narrative_claim":"The gate closure is a practical echo of neglected custody.","tension":"A cracked hinge will close a route at moonrise while travellers wait for a reopening time.","public_question":"How long will the western route remain closed?"}]}}"#;
    const ACCEPTED_PAGE: &str = r#"{"articles":[{"section":"Front Page","headline":"Court Sells the Crown's Seal, Then the Treasurer","deck":"A gambling debt reaches the throne room and leaves one official carrying the blame.","byline":"By the political editor","dateline":"Room","citations":["news:seal-scandal"],"paragraphs":["The Thorn Court has admitted that its royal seal was pawned to cover a dragon's gambling debt, a confession that turns private embarrassment into a public question of custody.","The treasurer who delivered that admission in open court was dismissed soon afterward. The court has explained the firing; it has not made the seal any less pawned."]}]}"#;
    const TWO_ARTICLE_PAGE: &str = r#"{"articles":[{"section":"Front Page","headline":"Court Sells the Crown's Seal, Then the Treasurer","deck":"A gambling debt reaches the throne room and leaves one official carrying the blame.","byline":"By the political editor","dateline":"Room","citations":["news:seal-scandal"],"paragraphs":["The Thorn Court has admitted that its royal seal was pawned to cover a dragon's gambling debt, a confession that turns private embarrassment into a public question of custody.","The treasurer who delivered that admission in open court was dismissed soon afterward. The court has explained the firing; it has not made the seal any less pawned."]},{"section":"Dispatches","headline":"West Gate to Close at Moonrise","deck":"Masons will replace a cracked hinge after the palace bell keeper's warning.","byline":"Staff report","dateline":"Yard","citations":["news:west-gate"],"paragraphs":["Officials warn the west gate is unsafe, and the palace bell keeper says it will close at moonrise while masons replace its cracked hinge.","Travellers using the gate have been told when it will close, though no reopening hour was included in the announcement."]}]}"#;
    const EMPTY_REPAIR_ACTION: &str =
        r#"{"tool":"submit_edits","replacements":[],"delete_finding_refs":[]}"#;
    const REPAIR_DECK_ACTION: &str = r#"{"tool":"submit_edits","replacements":[{"finding_ref":0,"replacement":"The court's admission leaves one official carrying the blame for the pawned seal."}],"delete_finding_refs":[]}"#;
    const SECOND_REPAIR_DECK_ACTION: &str = r#"{"tool":"submit_edits","replacements":[{"finding_ref":0,"replacement":"the dismissed treasurer named in the public record"}],"delete_finding_refs":[]}"#;
    const THIRD_REPAIR_DECK_ACTION: &str = r#"{"tool":"submit_edits","replacements":[{"finding_ref":0,"replacement":"identified as the dismissed treasurer"}],"delete_finding_refs":[]}"#;
    const DELETE_SECOND_ARTICLE_ACTION: &str =
        r#"{"tool":"submit_edits","replacements":[],"delete_finding_refs":[0]}"#;
    const ACCEPTING_COPY_DESK: &str = r#"{"accepted":true,"assessment":"The copy is fully supported by its cited public source and reads as attributed court reporting.","findings":[]}"#;

    async fn pending_local_rejection_fixture() -> (
        Campaign,
        tempfile::TempDir,
        CampaignStore,
        WorldNewspaperReconciliationCheckpoint,
    ) {
        const REJECTED: &str = r#"{"accepted":false,"assessment":"The deck overstates where the admitted debt reached.","findings":[{"article_index":0,"category":"unsupported_fact","claim_or_phrase":"A gambling debt reaches the throne room and leaves one official carrying the blame.","reason":"The cited source records the pawned seal and dismissal but does not locate the debt in the throne room."}]}"#;
        let campaign = campaign_with_news();
        let directory = tempfile::tempdir().unwrap();
        let store = CampaignStore::open(directory.path().join("campaign.cc")).unwrap();
        let model = ScriptedNewspaperModel::new([
            QUERY_ALL_RECORDS,
            ONE_STORY_AGENDA,
            ACCEPTED_PAGE,
            REJECTED,
            EMPTY_REPAIR_ACTION,
            EMPTY_REPAIR_ACTION,
            EMPTY_REPAIR_ACTION,
        ]);
        let WorldNewspaperAdvance::Pending { checkpoint, .. } = advance_world_newspaper(
            &model,
            &campaign,
            "The Underdeep Clarion",
            "A skeptical court broadsheet with dry restraint.",
            4,
            &store,
        )
        .await
        .unwrap() else {
            panic!("fixture must produce a pending reconciliation")
        };
        (campaign, directory, store, checkpoint)
    }

    #[test]
    fn copy_desk_schema_owns_the_complete_finding_category_set() {
        let schema = serde_json::to_string(&schema_for!(GroundingVerdictDraft)).unwrap();
        for category in [
            "unsupported_fact",
            "unearned_attribution",
            "procedural_leakage",
            "mechanical_copy",
        ] {
            assert!(schema.contains(category));
        }
    }

    #[test]
    fn narrative_workbench_owns_lead_and_allows_shared_foundational_records() {
        let records = public_news_records(&campaign_with_two_news()).unwrap();
        let invalid = WorldNewspaperEditorialAgenda {
            dominant_throughline: "Two failures of court custody.".into(),
            reader_stake: "Readers depend on both institutions.".into(),
            story_pitches: vec![
                WorldNewspaperStoryPitch {
                    lead: false,
                    citations: vec!["news:seal-scandal".into()],
                    focus_citation: "news:seal-scandal".into(),
                    narrative_claim: "The pawned seal opens the custody crisis.".into(),
                    tension: "Admission and dismissal point in different directions.".into(),
                    public_question: "Who answers for the seal?".into(),
                },
                WorldNewspaperStoryPitch {
                    lead: true,
                    citations: vec!["news:seal-scandal".into()],
                    focus_citation: "news:seal-scandal".into(),
                    narrative_claim: "The gate repeats the pattern.".into(),
                    tension: "Access closes while repair begins.".into(),
                    public_question: "When will the route reopen?".into(),
                },
            ],
        };

        assert!(
            validate_editorial_agenda(&records, &invalid, 4)
                .unwrap_err()
                .to_string()
                .contains("exactly its first pitch as lead")
        );

        let agenda: NarrativeSelectionAction = serde_json::from_str(TWO_STORY_AGENDA).unwrap();
        let NarrativeSelectionCommand::SubmitAgenda {
            dominant_throughline,
            reader_stake,
            story_pitches,
        } = agenda.command
        else {
            panic!("fixture must submit an agenda")
        };
        let admitted = WorldNewspaperEditorialAgenda {
            dominant_throughline,
            reader_stake,
            story_pitches,
        };
        validate_editorial_agenda(&records, &admitted, 4).unwrap();
        let mut duplicate = admitted.clone();
        duplicate.story_pitches[1]
            .citations
            .push("news:seal-scandal".into());
        validate_editorial_agenda(&records, &duplicate, 4).unwrap();
        let mut repeated_within_pitch = admitted.clone();
        repeated_within_pitch.story_pitches[0]
            .citations
            .push("news:seal-scandal".into());
        assert!(
            validate_editorial_agenda(&records, &repeated_within_pitch, 4)
                .unwrap_err()
                .to_string()
                .contains("repeats a public record ID")
        );

        let mut widened: EditorialPageDraft = serde_json::from_str(ACCEPTED_PAGE).unwrap();
        widened.articles[0].citations = vec!["news:west-gate".into()];
        assert!(
            validate_editorial_alignment(
                &widened,
                &WorldNewspaperEditorialAgenda {
                    dominant_throughline: admitted.dominant_throughline.clone(),
                    reader_stake: admitted.reader_stake.clone(),
                    story_pitches: vec![admitted.story_pitches[0].clone()],
                },
            )
            .unwrap_err()
            .to_string()
            .contains("admitted citation grouping")
        );
    }

    #[tokio::test]
    async fn narrative_workbench_queries_foundational_context_by_stable_record_id() {
        let records = public_news_records(&campaign_with_archive_news()).unwrap();
        assert_eq!(records.len(), 45);
        assert!(
            records
                .iter()
                .any(|record| record.facts[0].account.contains("speaking child"))
        );
        let mut tool = NarrativeSelectionWorkbench {
            records: &records,
            max_articles: 3,
            visible_record_ids: BTreeSet::new(),
            completed_queries: BTreeSet::new(),
        };
        let initial_schema = tool.action_schema().unwrap().to_string();
        assert!(!initial_schema.contains("submit_agenda"));

        let retrieval = tool
            .invoke(
                NarrativeSelectionAction {
                    command: NarrativeSelectionCommand::QueryPublicRecords {
                        terms: vec!["speaking child".into()],
                        match_terms: PublicRecordTermMatch::All,
                        entity_names: Vec::new(),
                        assertion_statuses: Vec::new(),
                        channels: Vec::new(),
                        order: PublicRecordOrder::Oldest,
                        cursor: None,
                        limit: 24,
                    },
                },
                &ModelAgentToolContext {
                    source_receipt_ids: Vec::new(),
                    current_model_receipt: None,
                },
            )
            .await;
        let ModelAgentToolOutcome::Continue { observation, .. } = retrieval else {
            panic!("public-record query must continue the same agent run")
        };
        let NarrativeSelectionFinding::QueryResult {
            records: found,
            next_cursor,
        } = observation
        else {
            panic!("query must return exact public records")
        };
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].record_id, "news:archive:00");
        assert!(found[0].facts[0].account.contains("speaking child"));
        assert!(next_cursor.is_none());
        assert!(
            tool.action_schema()
                .unwrap()
                .to_string()
                .contains("news:archive:00")
        );

        let action = NarrativeSelectionAction {
            command: NarrativeSelectionCommand::SubmitAgenda {
                dominant_throughline: "The original crisis survived its administrative aftermath."
                    .into(),
                reader_stake: "Readers still bear the consequences of the founding rupture.".into(),
                story_pitches: vec![WorldNewspaperStoryPitch {
                    lead: true,
                    citations: vec!["news:archive:00".into()],
                    focus_citation: "news:archive:00".into(),
                    narrative_claim: "The oldest dossier is the fact later notices avoid.".into(),
                    tension: "Institutions administer consequences without resolving the cause."
                        .into(),
                    public_question: "Who benefits when the founding rupture leaves the page?"
                        .into(),
                }],
            },
        };
        let ModelAgentToolOutcome::Accepted { output: agenda, .. } = tool
            .invoke(
                action,
                &ModelAgentToolContext {
                    source_receipt_ids: Vec::new(),
                    current_model_receipt: None,
                },
            )
            .await
        else {
            panic!("queried foundational record must be admissible")
        };
        let editor_desk = source_json_for_agenda(&records, Some(&agenda)).unwrap();
        assert!(editor_desk.contains("\"record_id\": \"news:archive:00\""));
        assert!(!editor_desk.contains("news:archive:44"));
    }

    #[test]
    fn narrative_workbench_pages_the_complete_ledger_from_an_inspected_cursor() {
        let records = public_news_records(&campaign_with_archive_news()).unwrap();
        let mut tool = NarrativeSelectionWorkbench {
            records: &records,
            max_articles: 3,
            visible_record_ids: BTreeSet::new(),
            completed_queries: BTreeSet::new(),
        };
        let query = |cursor| PublicRecordQuery {
            terms: Vec::new(),
            match_terms: PublicRecordTermMatch::All,
            entity_names: Vec::new(),
            assertion_statuses: Vec::new(),
            channels: Vec::new(),
            order: PublicRecordOrder::Oldest,
            cursor,
            limit: 24,
        };

        assert!(tool.query(query(Some("news:archive:24".into()))).is_err());
        let NarrativeSelectionFinding::QueryResult {
            records: first_page,
            next_cursor,
        } = tool.query(query(None)).unwrap()
        else {
            panic!("first query must return a public-record page")
        };
        assert_eq!(first_page.len(), 24);
        assert_eq!(first_page[0].record_id, "news:archive:00");
        assert_eq!(first_page[23].record_id, "news:archive:23");
        assert_eq!(next_cursor.as_deref(), Some("news:archive:23"));

        let NarrativeSelectionFinding::QueryResult {
            records: second_page,
            next_cursor,
        } = tool.query(query(next_cursor)).unwrap()
        else {
            panic!("second query must return a public-record page")
        };
        assert_eq!(second_page.len(), 21);
        assert_eq!(second_page[0].record_id, "news:archive:24");
        assert_eq!(second_page[20].record_id, "news:archive:44");
        assert!(next_cursor.is_none());
    }

    #[tokio::test]
    async fn narrative_agent_queries_before_submitting_an_agenda() {
        let prepared = prepare_newspaper(
            &campaign_with_archive_news(),
            "The Canopy Ledger",
            "Independent and pointed.",
            3,
        )
        .unwrap();
        let model = ScriptedNewspaperModel::new([
            QUERY_FOUNDING_CRISIS,
            r#"{"command":{"tool":"submit_agenda","dominant_throughline":"The founding crisis survived its administrative aftermath.","reader_stake":"Readers still bear the consequences while institutions manage the paperwork.","story_pitches":[{"lead":true,"citations":["news:archive:00"],"focus_citation":"news:archive:00","narrative_claim":"The original public record is the fact later notices cannot domesticate.","tension":"Administrative responses multiply while the founding rupture remains unresolved.","public_question":"Who benefits when the cause leaves the front page?"}]}}"#,
        ]);

        let run = select_editorial_agenda(&model, &prepared, 3).await.unwrap();

        assert_eq!(run.receipts.len(), 2);
        assert_eq!(run.output.story_pitches.len(), 1);
        assert_eq!(run.output.story_pitches[0].citations, ["news:archive:00"]);
        let requests = model.requests();
        assert_eq!(requests.len(), 2);
        assert!(requests[1].lived_stream.contains("news:archive:00"));
    }

    #[test]
    fn narrative_workbench_has_no_source_count_hole_and_reads_legacy_agendas() {
        let records = public_news_records(&campaign_with_archive_news()).unwrap();
        let pitch = WorldNewspaperStoryPitch {
            lead: true,
            citations: (0..8)
                .map(|index| format!("news:archive:{index:02}"))
                .collect(),
            focus_citation: "news:archive:00".into(),
            narrative_claim: "One sharp custody failure links the selected record.".into(),
            tension: "Public consequence runs ahead of public accountability.".into(),
            public_question: "Who answers for the admitted failure?".into(),
        };
        let agenda = WorldNewspaperEditorialAgenda {
            dominant_throughline: "Custody failed in public.".into(),
            reader_stake: "Readers bear the consequences of that failure.".into(),
            story_pitches: vec![pitch],
        };
        validate_editorial_agenda(&records, &agenda, 2).unwrap();

        let mut unfocused = agenda.clone();
        unfocused.story_pitches[0].citations.truncate(4);
        unfocused.story_pitches[0].focus_citation = "news:archive:07".into();
        assert!(
            validate_editorial_agenda(&records, &unfocused, 2)
                .unwrap_err()
                .to_string()
                .contains("focus record is not in its selected record set")
        );

        let legacy: WorldNewspaperStoryPitch = serde_json::from_str(
            r#"{"lead":true,"citations":["news:seal-scandal"],"angle":"A legacy framing field.","tension":"A live tension.","public_question":"What follows?"}"#,
        )
        .unwrap();
        assert_eq!(legacy.focus_citation, "");
        assert_eq!(legacy.narrative_claim, "A legacy framing field.");
    }

    #[test]
    fn grounding_repair_applies_distinct_same_target_findings_atomically() {
        let draft: EditorialPageDraft = serde_json::from_str(ACCEPTED_PAGE).unwrap();
        let verdict = WorldNewspaperGroundingVerdict {
            accepted: false,
            assessment: "Two unsupported claims share one deck.".into(),
            findings: vec![
                WorldNewspaperGroundingFinding {
                    article_index: 0,
                    category: WorldNewspaperGroundingCategory::UnsupportedFact,
                    claim_or_phrase: "A gambling debt reaches the throne room".into(),
                    reason: "The source does not establish that location.".into(),
                },
                WorldNewspaperGroundingFinding {
                    article_index: 0,
                    category: WorldNewspaperGroundingCategory::UnsupportedFact,
                    claim_or_phrase: "one official carrying the blame".into(),
                    reason: "The source establishes dismissal, not blame allocation.".into(),
                },
            ],
        };
        let mut replacements = vec![
            GroundingTextReplacement {
                finding_ref: GroundingFindingRef(0),
                replacement: "The court's admission reaches the public".into(),
            },
            GroundingTextReplacement {
                finding_ref: GroundingFindingRef(1),
                replacement: "the dismissed treasurer carrying the record".into(),
            },
        ];
        let repaired = apply_grounding_edits(
            &draft,
            &verdict,
            GroundingReconciliationAction::SubmitEdits {
                replacements: replacements.clone(),
                delete_finding_refs: Vec::new(),
            },
        )
        .unwrap();
        replacements.reverse();
        let repaired_reversed = apply_grounding_edits(
            &draft,
            &verdict,
            GroundingReconciliationAction::SubmitEdits {
                replacements,
                delete_finding_refs: Vec::new(),
            },
        )
        .unwrap();

        assert_eq!(repaired, repaired_reversed);
        assert_eq!(
            repaired.articles[0].deck,
            "The court's admission reaches the public and leaves the dismissed treasurer carrying the record."
        );
    }

    #[test]
    fn grounding_repair_collapses_nested_findings_to_the_outer_span() {
        let draft: EditorialPageDraft = serde_json::from_str(ACCEPTED_PAGE).unwrap();
        let outer = "The treasurer who delivered that admission in open court was dismissed soon afterward. The court has explained the firing; it has not made the seal any less pawned.";
        let verdict = WorldNewspaperGroundingVerdict {
            accepted: false,
            assessment: "One unsupported clause sits inside repetitive copy.".into(),
            findings: vec![
                WorldNewspaperGroundingFinding {
                    article_index: 0,
                    category: WorldNewspaperGroundingCategory::MechanicalCopy,
                    claim_or_phrase: outer.into(),
                    reason: "Rewrite the paragraph once.".into(),
                },
                WorldNewspaperGroundingFinding {
                    article_index: 0,
                    category: WorldNewspaperGroundingCategory::UnsupportedFact,
                    claim_or_phrase: "The court has explained the firing".into(),
                    reason: "The source establishes dismissal, not an explanation.".into(),
                },
            ],
        };
        let repaired = apply_grounding_edits(
            &draft,
            &verdict,
            GroundingReconciliationAction::SubmitEdits {
                replacements: vec![
                    GroundingTextReplacement {
                        finding_ref: GroundingFindingRef(1),
                        replacement: "The record gives no reason for the firing".into(),
                    },
                    GroundingTextReplacement {
                        finding_ref: GroundingFindingRef(0),
                        replacement: "The treasurer delivered the admission in open court and was dismissed soon afterward; the seal remained pawned.".into(),
                    },
                ],
                delete_finding_refs: Vec::new(),
            },
        )
        .unwrap();

        assert_eq!(
            repaired.articles[0].paragraphs[1],
            "The treasurer delivered the admission in open court and was dismissed soon afterward; the seal remained pawned."
        );
    }

    #[test]
    fn grounding_repair_empty_replacement_owns_its_space_seam() {
        let draft: EditorialPageDraft = serde_json::from_str(ACCEPTED_PAGE).unwrap();
        let verdict = WorldNewspaperGroundingVerdict {
            accepted: false,
            assessment: "One unsupported trailing sentence.".into(),
            findings: vec![WorldNewspaperGroundingFinding {
                article_index: 0,
                category: WorldNewspaperGroundingCategory::UnsupportedFact,
                claim_or_phrase:
                    "The court has explained the firing; it has not made the seal any less pawned."
                        .into(),
                reason: "The source establishes dismissal, not an explanation.".into(),
            }],
        };
        let repaired = apply_grounding_edits(
            &draft,
            &verdict,
            GroundingReconciliationAction::SubmitEdits {
                replacements: vec![GroundingTextReplacement {
                    finding_ref: GroundingFindingRef(0),
                    replacement: String::new(),
                }],
                delete_finding_refs: Vec::new(),
            },
        )
        .unwrap();

        assert_eq!(
            repaired.articles[0].paragraphs[1],
            "The treasurer who delivered that admission in open court was dismissed soon afterward."
        );
    }

    #[test]
    fn grounding_repair_rejects_duplicate_findings_and_overlapping_spans() {
        let draft: EditorialPageDraft = serde_json::from_str(ACCEPTED_PAGE).unwrap();
        let finding = WorldNewspaperGroundingFinding {
            article_index: 0,
            category: WorldNewspaperGroundingCategory::UnsupportedFact,
            claim_or_phrase: "A gambling debt reaches".into(),
            reason: "The source does not establish that movement.".into(),
        };
        let replacement = GroundingTextReplacement {
            finding_ref: GroundingFindingRef(0),
            replacement: "The court's admission reaches".into(),
        };
        let duplicate = apply_grounding_edits(
            &draft,
            &WorldNewspaperGroundingVerdict {
                accepted: false,
                assessment: "One finding.".into(),
                findings: vec![finding.clone()],
            },
            GroundingReconciliationAction::SubmitEdits {
                replacements: vec![replacement.clone(), replacement],
                delete_finding_refs: Vec::new(),
            },
        )
        .unwrap_err();
        assert!(duplicate.to_string().contains("repeats one exact finding"));

        let overlapping_finding = WorldNewspaperGroundingFinding {
            article_index: 0,
            category: WorldNewspaperGroundingCategory::UnsupportedFact,
            claim_or_phrase: "gambling debt reaches the throne room".into(),
            reason: "The source does not establish that location.".into(),
        };
        let overlap = apply_grounding_edits(
            &draft,
            &WorldNewspaperGroundingVerdict {
                accepted: false,
                assessment: "Overlapping findings.".into(),
                findings: vec![finding.clone(), overlapping_finding.clone()],
            },
            GroundingReconciliationAction::SubmitEdits {
                replacements: vec![
                    GroundingTextReplacement {
                        finding_ref: GroundingFindingRef(0),
                        replacement: "The admission reaches".into(),
                    },
                    GroundingTextReplacement {
                        finding_ref: GroundingFindingRef(1),
                        replacement: "the public record".into(),
                    },
                ],
                delete_finding_refs: Vec::new(),
            },
        )
        .unwrap_err();
        assert!(overlap.to_string().contains("overlapping exact phrases"));
    }

    #[tokio::test]
    async fn newspaper_is_editorial_copy_with_separate_provenance() {
        let campaign = campaign_with_news();
        let model = ScriptedNewspaperModel::new([
            QUERY_ALL_RECORDS,
            ONE_STORY_AGENDA,
            ACCEPTED_PAGE,
            ACCEPTING_COPY_DESK,
        ]);
        let composition = compose_world_newspaper(
            &model,
            &campaign,
            "The Underdeep Clarion",
            "A skeptical court broadsheet with dry restraint.",
            4,
        )
        .await
        .unwrap();
        let page = render_world_newspaper_markdown(&composition.issue);
        let audit = render_world_newspaper_audit_markdown(&composition.issue);

        assert_eq!(
            composition.issue.schema,
            "ghostlight.world_newspaper_issue.v3"
        );
        assert_eq!(
            composition.issue.articles[0].event_ids(),
            ["event:seal-scandal"]
        );
        assert_eq!(
            composition.issue.articles[0].source_news_ids(),
            ["news:seal-scandal"]
        );
        assert_eq!(composition.issue.edition_label, EDITION_LABEL);
        assert!(page.contains("Court Sells the Crown's Seal"));
        assert!(page.contains("*Current Edition*"));
        assert!(page.contains("The court has explained the firing"));
        assert!(!page.contains("World revision"));
        assert!(!page.contains("event:seal-scandal"));
        assert!(!page.contains("committed public channel"));
        assert!(audit.contains("event:seal-scandal"));
        assert!(audit.contains("Source world revision: 3"));
        assert!(audit.contains("Editorial agenda"));
        assert!(audit.contains("court made its private gambling debt"));
        assert!(audit.contains("Exact committed account: The Thorn Court admits"));
        assert!(
            audit.contains("Assertion status: course_committed_embedded_actions_not_completed")
        );
        assert!(audit.contains("Named people: None asserted"));
        assert_eq!(composition.model_receipts.len(), 4);
        let requests = model.requests();
        assert_eq!(
            requests[0].stage,
            "newspaper_narrative_selection_agent_action"
        );
        assert_eq!(requests[0].model, MODEL_CAPABLE);
        assert_eq!(
            requests[1].stage,
            "newspaper_narrative_selection_agent_action"
        );
        assert_eq!(requests[2].stage, "newspaper_editor");
        assert!(
            requests[2]
                .lived_stream
                .contains("ADMITTED NARRATIVE AGENDA")
        );
        assert!(
            requests[2]
                .source_receipt_ids
                .contains(&composition.model_receipts[0].storage_key().to_owned())
        );
        assert!(requests[1].lived_stream.contains("news:seal-scandal"));
        assert!(requests[1].lived_stream.contains("event:seal-scandal"));
        assert!(requests[1].lived_stream.contains("\"channel\""));
        assert!(requests[1].lived_stream.contains("\"reliability\""));
        assert!(!requests[1].lived_stream.contains("institution_action"));
        assert!(!requests[1].lived_stream.contains("actor_ids"));
        assert!(!requests[1].lived_stream.contains("institution_ids"));
        assert!(
            !serde_json::to_string(requests[0].output_schema.as_ref().unwrap())
                .unwrap()
                .contains("edition_label")
        );
    }

    #[test]
    fn public_record_projection_preserves_canonical_records_without_packet_authority() {
        let campaign = campaign_with_typed_and_duplicate_news();
        let records = public_news_records(&campaign).unwrap();
        let desk = serde_json::to_string_pretty(&records).unwrap();
        let original = records
            .iter()
            .find(|record| record.record_id == "news:seal-scandal")
            .unwrap();

        assert_eq!(records.len(), 4);
        assert_eq!(desk.matches("royal seal was pawned").count(), 3);
        assert_eq!(original.facts[0].event_ids, ["event:seal-scandal"]);
        assert!(desk.contains("strains the command tie"));
        assert!(desk.contains("take up a new public demand"));
        assert!(desk.contains("course_committed_embedded_actions_not_completed"));
        assert!(desk.contains("material_change_committed"));
        assert!(desk.contains("public_declaration"));
        assert!(!desk.contains("strategic_activity_outcome"));
        assert!(desk.contains("event:seal-scandal"));
        assert!(desk.contains("news:seal-scandal"));
        assert!(desk.contains("channel"));
        assert!(desk.contains("reliability"));
        assert!(desk.contains("\"at\""));
        assert!(!desk.contains("actor_ids"));
        assert!(!desk.contains("institution_ids"));

        let mut draft: EditorialPageDraft = serde_json::from_str(ACCEPTED_PAGE).unwrap();
        draft.articles[0].citations = vec![original.record_id.clone()];
        let issue = lower_editorial_page(
            &campaign,
            "The Underdeep Clarion".into(),
            &records,
            None,
            draft,
            &[],
        )
        .unwrap();
        assert_eq!(issue.articles[0].source_news_ids(), ["news:seal-scandal"]);
        assert_eq!(issue.articles[0].event_ids(), ["event:seal-scandal"]);
    }

    #[test]
    fn public_records_do_not_promote_involved_metadata_to_asserted_names_or_datelines() {
        let mut campaign = campaign_with_two_news();
        let mut ann = campaign.actors["player"].clone();
        ann.id = "ann".into();
        ann.name = "Ann".into();
        campaign.actors.insert(ann.id.clone(), ann);
        campaign.locations.insert(
            "hall".into(),
            crate::domain::Location {
                id: "hall".into(),
                name: "Hall".into(),
                container_id: None,
                routes: BTreeMap::new(),
                persistent_features: vec![],
            },
        );
        campaign.institutions.insert(
            "unmentioned-regency".into(),
            crate::domain::InstitutionState {
                id: "unmentioned-regency".into(),
                name: "Mossglass Regency".into(),
                resources: vec![],
                goals: vec![],
                posture: "watching".into(),
            },
        );
        let event = campaign
            .events
            .iter_mut()
            .find(|event| event.id == "event:seal-scandal")
            .unwrap();
        event.summary = "The Thorn Court announces that every officer shall answer.".into();
        event.actor_ids = vec!["ann".into()];
        event.institution_ids = vec!["unmentioned-regency".into()];
        event.location_ids = vec!["hall".into(), "yard".into()];

        let records = public_news_records(&campaign).unwrap();
        let record = records
            .iter()
            .find(|record| record.record_id == "news:seal-scandal")
            .unwrap();
        assert!(record.facts[0].named_people.is_empty());
        assert!(record.facts[0].institutions.is_empty());
        assert!(record.facts[0].places.is_empty());
    }

    #[test]
    fn public_records_name_people_without_inventing_identity_attributes() {
        let mut campaign = campaign_with_news();
        let mut ann = campaign.actors["player"].clone();
        ann.id = "ann".into();
        ann.name = "Ann Vey".into();
        campaign.actors.insert(ann.id.clone(), ann);
        let event = campaign
            .events
            .iter_mut()
            .find(|event| event.id == "event:seal-scandal")
            .unwrap();
        event.summary = "Ann Vey publicly contests the Thorn Court's seal account.".into();
        event.actor_ids = vec!["ann".into()];

        let records = public_news_records(&campaign).unwrap();
        let named = &records[0].facts[0].named_people;
        assert_eq!(named.len(), 1);
        assert_eq!(named[0].name, "Ann Vey");
        assert!(named[0].supported_identity_attributes.is_empty());
        let desk = serde_json::to_string(&records).unwrap();
        assert!(desk.contains("supported_identity_attributes"));
    }

    #[tokio::test]
    async fn newspaper_rejects_a_headline_without_a_committed_event_before_inference() {
        let mut campaign = campaign_with_news();
        campaign.news[0].event_ids = vec!["event:missing".into()];
        let model = ScriptedNewspaperModel::new([]);
        let error = compose_world_newspaper(
            &model,
            &campaign,
            "The Clarion",
            "A sober regional paper.",
            4,
        )
        .await
        .unwrap_err();
        assert!(error.to_string().contains("unknown event"));
    }

    #[tokio::test]
    async fn newspaper_rejects_ambiguous_public_record_ids_before_inference() {
        let mut campaign = campaign_with_two_news();
        campaign.news[1].id = campaign.news[0].id.clone();
        let model = ScriptedNewspaperModel::new([]);
        let error = compose_world_newspaper(
            &model,
            &campaign,
            "The Clarion",
            "A sober regional paper.",
            4,
        )
        .await
        .unwrap_err();
        assert!(error.to_string().contains("repeats record ID"));
        assert!(model.requests().is_empty());
    }

    #[tokio::test]
    async fn copy_desk_rejection_enters_the_grounding_agent_on_the_same_selected_records() {
        const REJECTED: &str = r#"{"accepted":false,"assessment":"The deck overstates where the admitted debt reached.","findings":[{"article_index":0,"category":"unsupported_fact","claim_or_phrase":"A gambling debt reaches the throne room and leaves one official carrying the blame.","reason":"The cited source records the pawned seal and dismissal but does not locate the debt in the throne room."}]}"#;
        let campaign = campaign_with_news();
        let model = ScriptedNewspaperModel::new([
            QUERY_ALL_RECORDS,
            ONE_STORY_AGENDA,
            ACCEPTED_PAGE,
            REJECTED,
            REPAIR_DECK_ACTION,
            ACCEPTING_COPY_DESK,
        ]);
        let composition = compose_world_newspaper(
            &model,
            &campaign,
            "The Underdeep Clarion",
            "A skeptical court broadsheet with dry restraint.",
            4,
        )
        .await
        .unwrap();

        assert_eq!(composition.model_receipts.len(), 6);
        let requests = model.requests();
        assert_eq!(requests[0].temperature, Some(0.8));
        assert_eq!(requests[1].temperature, Some(0.8));
        assert_eq!(requests[2].temperature, Some(0.75));
        assert_eq!(requests[4].temperature, Some(0.1));
        assert_eq!(requests[4].model, MODEL_BALANCED);
        assert_eq!(
            requests[4].stage,
            "newspaper_grounding_reconciliation_agent_action"
        );
        assert!(
            requests[4]
                .lived_stream
                .contains("A gambling debt reaches the throne room")
        );
        let repair_schema_value = requests[4].output_schema.as_ref().unwrap();
        assert_eq!(
            repair_schema_value
                .pointer("/properties/replacements/items/properties/finding_ref/enum"),
            Some(&serde_json::json!([0]))
        );
        assert_eq!(
            repair_schema_value.pointer("/properties/delete_finding_refs/items/enum"),
            Some(&serde_json::json!([0]))
        );
        let repair_schema = serde_json::to_string(repair_schema_value).unwrap();
        assert!(!repair_schema.contains("citations"));
        assert!(!repair_schema.contains("byline"));
        assert!(!repair_schema.contains("section"));
        assert!(repair_schema.contains("finding_ref"));
        assert!(!repair_schema.contains("expected_phrase"));
        assert!(!repair_schema.contains("article_index"));
        assert!(!repair_schema.contains("paragraph_index"));
        assert_eq!(
            composition.issue.articles[0].source_news_ids(),
            ["news:seal-scandal"]
        );
        assert!(requests[3].lived_stream.contains("publication role labels"));
        assert!(requests[3].lived_stream.contains("Metaphor, dry wit"));
        assert!(composition.grounding.accepted);
    }

    #[tokio::test]
    async fn grounding_agent_may_kill_a_rejected_article_but_must_recheck_survivors() {
        const REJECTED_SECOND: &str = r#"{"accepted":false,"assessment":"The dispatch adds a claim absent from its source.","findings":[{"article_index":1,"category":"unsupported_fact","claim_or_phrase":"the west gate is unsafe","reason":"The cited source records a cracked hinge and closure, not a safety finding."}]}"#;
        let campaign = campaign_with_two_news();
        let model = ScriptedNewspaperModel::new([
            QUERY_ALL_RECORDS,
            TWO_STORY_AGENDA,
            TWO_ARTICLE_PAGE,
            REJECTED_SECOND,
            DELETE_SECOND_ARTICLE_ACTION,
            ACCEPTING_COPY_DESK,
        ]);
        let composition = compose_world_newspaper(
            &model,
            &campaign,
            "The Underdeep Clarion",
            "A skeptical court broadsheet with dry restraint.",
            4,
        )
        .await
        .unwrap();

        assert_eq!(composition.issue.articles.len(), 1);
        assert_eq!(
            composition.issue.articles[0].headline,
            "Court Sells the Crown's Seal, Then the Treasurer"
        );
        assert!(
            !render_world_newspaper_markdown(&composition.issue).contains("west gate is unsafe")
        );
        assert_eq!(composition.model_receipts.len(), 6);
        let final_copy_desk = composition.model_receipts.last().unwrap();
        assert_eq!(final_copy_desk.stage, "newspaper_copy_desk");
        assert!(
            final_copy_desk
                .snapshot_binding
                .contains("grounding-reconciliation")
        );
        assert!(composition.model_receipts[..5].iter().any(|receipt| {
            receipt.stage == "newspaper_copy_desk"
                && final_copy_desk
                    .source_receipt_ids
                    .contains(&receipt.storage_key().to_owned())
        }));
        assert!(composition.model_receipts.iter().any(|receipt| {
            receipt.stage == "newspaper_grounding_reconciliation_agent_action"
                && final_copy_desk
                    .source_receipt_ids
                    .contains(&receipt.storage_key().to_owned())
        }));
        assert!(composition.grounding.accepted);
    }

    #[tokio::test]
    async fn grounding_agent_may_react_once_to_the_copy_desks_repair_observation() {
        const REJECTED: &str = r#"{"accepted":false,"assessment":"The deck overstates where the admitted debt reached.","findings":[{"article_index":0,"category":"unsupported_fact","claim_or_phrase":"A gambling debt reaches the throne room and leaves one official carrying the blame.","reason":"The cited source records the pawned seal and dismissal but does not locate the debt in the throne room."}]}"#;
        const REJECTED_AFTER_REPAIR: &str = r#"{"accepted":false,"assessment":"The revised deck still overstates blame.","findings":[{"article_index":0,"category":"unsupported_fact","claim_or_phrase":"one official carrying the blame","reason":"The cited source records dismissal but does not establish how blame was allocated."}]}"#;
        let campaign = campaign_with_news();
        let model = ScriptedNewspaperModel::new([
            QUERY_ALL_RECORDS,
            ONE_STORY_AGENDA,
            ACCEPTED_PAGE,
            REJECTED,
            REPAIR_DECK_ACTION,
            REJECTED_AFTER_REPAIR,
            SECOND_REPAIR_DECK_ACTION,
            ACCEPTING_COPY_DESK,
        ]);
        let composition = compose_world_newspaper(
            &model,
            &campaign,
            "The Underdeep Clarion",
            "A skeptical court broadsheet with dry restraint.",
            4,
        )
        .await
        .unwrap();
        assert_eq!(composition.model_receipts.len(), 8);
        assert!(composition.grounding.accepted);
        let requests = model.requests();
        assert_eq!(
            requests
                .iter()
                .filter(|request| request.stage == "newspaper_editor")
                .count(),
            1
        );
        assert_eq!(
            requests
                .iter()
                .filter(|request| {
                    request.stage == "newspaper_grounding_reconciliation_agent_action"
                })
                .count(),
            2
        );
        assert!(
            requests[6]
                .lived_stream
                .contains("revised deck still overstates blame")
        );
        assert!(requests[6].lived_stream.contains("\"finding_ref\""));
    }

    #[tokio::test]
    async fn grounding_agent_can_correct_admission_then_copy_desk_within_three_actions() {
        const REJECTED: &str = r#"{"accepted":false,"assessment":"The deck overstates where the admitted debt reached.","findings":[{"article_index":0,"category":"unsupported_fact","claim_or_phrase":"A gambling debt reaches the throne room and leaves one official carrying the blame.","reason":"The cited source records the pawned seal and dismissal but does not locate the debt in the throne room."}]}"#;
        const REJECTED_AFTER_REPAIR: &str = r#"{"accepted":false,"assessment":"The revised deck still overstates blame.","findings":[{"article_index":0,"category":"unsupported_fact","claim_or_phrase":"one official carrying the blame","reason":"The cited source records dismissal but does not establish how blame was allocated."}]}"#;
        let campaign = campaign_with_news();
        let model = ScriptedNewspaperModel::new([
            QUERY_ALL_RECORDS,
            ONE_STORY_AGENDA,
            ACCEPTED_PAGE,
            REJECTED,
            EMPTY_REPAIR_ACTION,
            REPAIR_DECK_ACTION,
            REJECTED_AFTER_REPAIR,
            SECOND_REPAIR_DECK_ACTION,
            ACCEPTING_COPY_DESK,
        ]);
        let composition = compose_world_newspaper(
            &model,
            &campaign,
            "The Underdeep Clarion",
            "A skeptical court broadsheet with dry restraint.",
            4,
        )
        .await
        .unwrap();

        assert_eq!(composition.model_receipts.len(), 9);
        assert!(composition.grounding.accepted);
        let requests = model.requests();
        assert_eq!(
            requests
                .iter()
                .filter(|request| request.stage == "newspaper_editor")
                .count(),
            1
        );
        assert_eq!(
            requests
                .iter()
                .filter(|request| {
                    request.stage == "newspaper_grounding_reconciliation_agent_action"
                })
                .count(),
            3
        );
        assert_eq!(
            composition
                .model_receipts
                .iter()
                .filter(|receipt| receipt.stage
                    == "newspaper_grounding_reconciliation_agent_action"
                    && receipt.validation_result == "semantic_invalid")
                .count(),
            2
        );
        assert!(requests[5].lived_stream.contains("was not admitted"));
        assert!(
            requests[7]
                .lived_stream
                .contains("revised deck still overstates blame")
        );
        let final_copy_desk = composition.model_receipts.last().unwrap();
        assert_eq!(final_copy_desk.stage, "newspaper_copy_desk");
        assert_eq!(final_copy_desk.validation_result, "valid");
        assert!(composition.model_receipts[..8].iter().all(|receipt| {
            final_copy_desk
                .source_receipt_ids
                .contains(&receipt.storage_key().to_owned())
        }));
    }

    #[tokio::test]
    async fn terminal_copy_desk_rejection_carries_every_completed_receipt() {
        const REJECTED: &str = r#"{"accepted":false,"assessment":"The deck overstates where the admitted debt reached.","findings":[{"article_index":0,"category":"unsupported_fact","claim_or_phrase":"A gambling debt reaches the throne room and leaves one official carrying the blame.","reason":"The cited source records the pawned seal and dismissal but does not locate the debt in the throne room."}]}"#;
        const REJECTED_AFTER_REPAIR: &str = r#"{"accepted":false,"assessment":"The revised deck still overstates blame.","findings":[{"article_index":0,"category":"unsupported_fact","claim_or_phrase":"one official carrying the blame","reason":"The cited source records dismissal but does not establish how blame was allocated."}]}"#;
        const REJECTED_AFTER_SECOND_REPAIR: &str = r#"{"accepted":false,"assessment":"The second revision still overstates the record.","findings":[{"article_index":0,"category":"unsupported_fact","claim_or_phrase":"named in the public record","reason":"The source records a dismissal but does not say the treasurer was named in the record."}]}"#;
        const REJECTED_AFTER_THIRD_REPAIR: &str = r#"{"accepted":false,"assessment":"The third revision still overstates identification.","findings":[{"article_index":0,"category":"unsupported_fact","claim_or_phrase":"identified as the dismissed treasurer","reason":"The source records a dismissal but does not establish that the deck's subject was identified in this way."}]}"#;
        let campaign = campaign_with_news();
        let model = ScriptedNewspaperModel::new([
            QUERY_ALL_RECORDS,
            ONE_STORY_AGENDA,
            ACCEPTED_PAGE,
            REJECTED,
            REPAIR_DECK_ACTION,
            REJECTED_AFTER_REPAIR,
            SECOND_REPAIR_DECK_ACTION,
            REJECTED_AFTER_SECOND_REPAIR,
            THIRD_REPAIR_DECK_ACTION,
            REJECTED_AFTER_THIRD_REPAIR,
        ]);
        let error = compose_world_newspaper(
            &model,
            &campaign,
            "The Underdeep Clarion",
            "A skeptical court broadsheet with dry restraint.",
            4,
        )
        .await
        .unwrap_err();
        let failure = error
            .downcast_ref::<WorldNewspaperCompositionFailure>()
            .expect("terminal editorial rejection must retain its receipts");

        assert!(failure.message.contains("pending after 3 semantic steps"));
        assert_eq!(failure.model_receipts.len(), 10);
        assert_eq!(
            failure
                .model_receipts
                .iter()
                .filter(|receipt| receipt.validation_result == "semantic_invalid")
                .count(),
            3
        );
        for rejected in failure
            .model_receipts
            .iter()
            .filter(|receipt| receipt.validation_result == "semantic_invalid")
        {
            assert_eq!(
                rejected.stage,
                "newspaper_grounding_reconciliation_agent_action"
            );
            assert!(
                rejected
                    .local_validation_error
                    .as_deref()
                    .is_some_and(|finding| finding.contains("copy_desk_rejected"))
            );
        }
    }

    #[tokio::test]
    async fn pending_reconciliation_resumes_its_exact_chain_without_an_editor() {
        const REJECTED: &str = r#"{"accepted":false,"assessment":"The deck overstates where the admitted debt reached.","findings":[{"article_index":0,"category":"unsupported_fact","claim_or_phrase":"A gambling debt reaches the throne room and leaves one official carrying the blame.","reason":"The cited source records the pawned seal and dismissal but does not locate the debt in the throne room."}]}"#;
        const REJECTED_AFTER_REPAIR: &str = r#"{"accepted":false,"assessment":"The revised deck still overstates blame.","findings":[{"article_index":0,"category":"unsupported_fact","claim_or_phrase":"one official carrying the blame","reason":"The cited source records dismissal but does not establish how blame was allocated."}]}"#;
        const REJECTED_AFTER_SECOND_REPAIR: &str = r#"{"accepted":false,"assessment":"The second revision still overstates the record.","findings":[{"article_index":0,"category":"unsupported_fact","claim_or_phrase":"named in the public record","reason":"The source records a dismissal but does not say the treasurer was named in the record."}]}"#;
        const REJECTED_AFTER_THIRD_REPAIR: &str = r#"{"accepted":false,"assessment":"The third revision still overstates identification.","findings":[{"article_index":0,"category":"unsupported_fact","claim_or_phrase":"identified as the dismissed treasurer","reason":"The source records a dismissal but does not establish that the deck's subject was identified in this way."}]}"#;
        const RESUME_REPAIR_ACTION: &str = r#"{"tool":"submit_edits","replacements":[{"finding_ref":0,"replacement":"the dismissed treasurer"}],"delete_finding_refs":[]}"#;
        let campaign = campaign_with_news();
        let directory = tempfile::tempdir().unwrap();
        let store = CampaignStore::open(directory.path().join("campaign.cc")).unwrap();
        let first_model = ScriptedNewspaperModel::new([
            QUERY_ALL_RECORDS,
            ONE_STORY_AGENDA,
            ACCEPTED_PAGE,
            REJECTED,
            REPAIR_DECK_ACTION,
            REJECTED_AFTER_REPAIR,
            SECOND_REPAIR_DECK_ACTION,
            REJECTED_AFTER_SECOND_REPAIR,
            THIRD_REPAIR_DECK_ACTION,
            REJECTED_AFTER_THIRD_REPAIR,
        ]);

        let pending = advance_world_newspaper(
            &first_model,
            &campaign,
            "The Underdeep Clarion",
            "A skeptical court broadsheet with dry restraint.",
            4,
            &store,
        )
        .await
        .unwrap();
        let WorldNewspaperAdvance::Pending {
            checkpoint,
            model_receipts: prior_receipts,
        } = pending
        else {
            panic!("three rejected actions must yield typed pending state")
        };
        assert_eq!(checkpoint.generation(), 3);
        assert_eq!(prior_receipts.len(), 10);
        assert_eq!(
            store
                .keys("world_newspaper_reconciliation_checkpoint.v2")
                .unwrap()
                .len(),
            4
        );
        assert!(prior_receipts.iter().all(|receipt| {
            store
                .load::<ModelStageReceipt>("persona_stage_receipt.v1", receipt.storage_key())
                .unwrap()
                .is_some()
        }));

        let resume_model = ScriptedNewspaperModel::new([RESUME_REPAIR_ACTION, ACCEPTING_COPY_DESK]);
        let accepted = advance_world_newspaper(
            &resume_model,
            &campaign,
            "The Underdeep Clarion",
            "A skeptical court broadsheet with dry restraint.",
            4,
            &store,
        )
        .await
        .unwrap()
        .into_accepted()
        .unwrap();
        let resumed_requests = resume_model.requests();
        assert_eq!(resumed_requests.len(), 2);
        assert!(
            resumed_requests
                .iter()
                .all(|request| { request.stage != "newspaper_editor" })
        );
        assert!(prior_receipts.iter().all(|receipt| {
            resumed_requests[0]
                .source_receipt_ids
                .contains(&receipt.storage_key().to_owned())
        }));
        assert!(
            resumed_requests[0]
                .source_receipt_ids
                .contains(&checkpoint.id().to_owned())
        );
        assert_eq!(accepted.model_receipts.len(), 12);
        assert_eq!(
            &accepted.model_receipts[..prior_receipts.len()],
            prior_receipts.as_slice()
        );
        let final_copy_desk = accepted.model_receipts.last().unwrap();
        assert!(accepted.model_receipts[..11].iter().all(|receipt| {
            final_copy_desk
                .source_receipt_ids
                .contains(&receipt.storage_key().to_owned())
        }));

        let idempotent_model = ScriptedNewspaperModel::new([]);
        let repeated = advance_world_newspaper(
            &idempotent_model,
            &campaign,
            "The Underdeep Clarion",
            "A skeptical court broadsheet with dry restraint.",
            4,
            &store,
        )
        .await
        .unwrap()
        .into_accepted()
        .unwrap();
        assert_eq!(repeated, accepted);
        assert!(idempotent_model.requests().is_empty());
    }

    #[tokio::test]
    async fn typed_legacy_import_enters_the_common_resume_validator() {
        const REJECTED: &str = r#"{"accepted":false,"assessment":"The deck overstates where the admitted debt reached.","findings":[{"article_index":0,"category":"unsupported_fact","claim_or_phrase":"A gambling debt reaches the throne room and leaves one official carrying the blame.","reason":"The cited source records the pawned seal and dismissal but does not locate the debt in the throne room."}]}"#;
        const REJECTED_AFTER_REPAIR: &str = r#"{"accepted":false,"assessment":"The revised deck still overstates blame.","findings":[{"article_index":0,"category":"unsupported_fact","claim_or_phrase":"one official carrying the blame","reason":"The cited source records dismissal but does not establish how blame was allocated."}]}"#;
        const REJECTED_AFTER_SECOND_REPAIR: &str = r#"{"accepted":false,"assessment":"The second revision still overstates the record.","findings":[{"article_index":0,"category":"unsupported_fact","claim_or_phrase":"named in the public record","reason":"The source records a dismissal but does not say the treasurer was named in the record."}]}"#;
        const REJECTED_AFTER_THIRD_REPAIR: &str = r#"{"accepted":false,"assessment":"The third revision still overstates identification.","findings":[{"article_index":0,"category":"unsupported_fact","claim_or_phrase":"identified as the dismissed treasurer","reason":"The source records a dismissal but does not establish that the deck's subject was identified in this way."}]}"#;
        const RESUME_REPAIR_ACTION: &str = r#"{"tool":"submit_edits","replacements":[{"finding_ref":0,"replacement":"the dismissed treasurer"}],"delete_finding_refs":[]}"#;
        let campaign = campaign_with_news();
        let source_directory = tempfile::tempdir().unwrap();
        let source_store =
            CampaignStore::open(source_directory.path().join("campaign.cc")).unwrap();
        let source_model = ScriptedNewspaperModel::new([
            QUERY_ALL_RECORDS,
            ONE_STORY_AGENDA,
            ACCEPTED_PAGE,
            REJECTED,
            REPAIR_DECK_ACTION,
            REJECTED_AFTER_REPAIR,
            SECOND_REPAIR_DECK_ACTION,
            REJECTED_AFTER_SECOND_REPAIR,
            THIRD_REPAIR_DECK_ACTION,
            REJECTED_AFTER_THIRD_REPAIR,
        ]);
        let WorldNewspaperAdvance::Pending {
            checkpoint,
            model_receipts,
        } = advance_world_newspaper(
            &source_model,
            &campaign,
            "The Underdeep Clarion",
            "A skeptical court broadsheet with dry restraint.",
            4,
            &source_store,
        )
        .await
        .unwrap()
        else {
            panic!("fixture must produce importable pending state")
        };
        let import = WorldNewspaperReconciliationImport {
            schema: "ghostlight.world_newspaper_reconciliation_import.v1".into(),
            source_witness_digest: format!("sha256:{}", "a".repeat(64)),
            editorial_agenda: checkpoint.editorial_agenda.clone(),
            draft: checkpoint.draft.clone(),
            verdict: checkpoint.verdict.clone(),
            model_receipts: model_receipts.clone(),
        };
        let imported_directory = tempfile::tempdir().unwrap();
        let imported_store =
            CampaignStore::open(imported_directory.path().join("campaign.cc")).unwrap();
        let imported = admit_world_newspaper_reconciliation_import(
            &campaign,
            "The Underdeep Clarion",
            "A skeptical court broadsheet with dry restraint.",
            4,
            &imported_store,
            import,
        )
        .unwrap();
        assert_eq!(imported.generation(), 0);
        assert_eq!(
            imported.origin,
            WorldNewspaperCheckpointOrigin::LegacyTerminalImport
        );

        let resume_model = ScriptedNewspaperModel::new([RESUME_REPAIR_ACTION, ACCEPTING_COPY_DESK]);
        let accepted = advance_world_newspaper(
            &resume_model,
            &campaign,
            "The Underdeep Clarion",
            "A skeptical court broadsheet with dry restraint.",
            4,
            &imported_store,
        )
        .await
        .unwrap()
        .into_accepted()
        .unwrap();
        assert_eq!(accepted.model_receipts.len(), model_receipts.len() + 2);
        assert!(
            resume_model
                .requests()
                .iter()
                .all(|request| request.stage != "newspaper_editor")
        );
    }

    #[tokio::test]
    async fn pending_reconciliation_rejects_same_task_source_drift_before_inference() {
        let (mut campaign, _directory, store, _) = pending_local_rejection_fixture().await;
        campaign.events[0]
            .summary
            .push_str(" A changed public record.");
        let model = ScriptedNewspaperModel::new([]);

        let error = advance_world_newspaper(
            &model,
            &campaign,
            "The Underdeep Clarion",
            "A skeptical court broadsheet with dry restraint.",
            4,
            &store,
        )
        .await
        .unwrap_err();

        assert!(error.to_string().contains("checkpoint is invalid"));
        assert!(model.requests().is_empty());
    }

    #[tokio::test]
    async fn pending_reconciliation_rejects_a_fork_before_inference() {
        let (campaign, _directory, store, checkpoint) = pending_local_rejection_fixture().await;
        let mut sibling = checkpoint.clone();
        sibling.draft.articles[0].headline = "A rival checkpoint branch".into();
        sibling.id = checkpoint_identity(
            &sibling.publication_task_binding,
            &sibling.editorial_binding,
            sibling.generation,
            sibling.previous_checkpoint_id.as_deref(),
            &sibling.origin,
            sibling.source_witness_digest.as_deref(),
            sibling.editorial_agenda.as_ref(),
            &sibling.draft,
            &sibling.verdict,
            &sibling.model_receipt_ids,
            &sibling.receipt_chain_digest,
        )
        .unwrap();
        persist_reconciliation_checkpoint(&store, &sibling).unwrap();
        let model = ScriptedNewspaperModel::new([]);

        let error = advance_world_newspaper(
            &model,
            &campaign,
            "The Underdeep Clarion",
            "A skeptical court broadsheet with dry restraint.",
            4,
            &store,
        )
        .await
        .unwrap_err();

        assert!(error.to_string().contains("multiple or missing tips"));
        assert!(model.requests().is_empty());
    }

    #[tokio::test]
    async fn reader_projection_escapes_model_and_consumer_markdown() {
        const MARKDOWN_PAGE: &str = r#"{"articles":[{"section":"Front Page","headline":"[Court](https://headline.invalid) Faces <Reckoning>","deck":"The *royal* debt now reaches every keeper of the seal.","byline":"By the political editor","dateline":"Room","citations":["news:seal-scandal"],"paragraphs":["- The Thorn Court admitted the royal seal was pawned to cover a dragon's gambling debt; <img src=x> cannot make the confession ~~prettier~~.","1. The dismissed treasurer leaves readers with a [public record](https://copy.invalid) and the court with a seal that remains pawned."]}]}"#;
        let campaign = campaign_with_news();
        let model = ScriptedNewspaperModel::new([
            QUERY_ALL_RECORDS,
            ONE_STORY_AGENDA,
            MARKDOWN_PAGE,
            ACCEPTING_COPY_DESK,
        ]);
        let composition = compose_world_newspaper(
            &model,
            &campaign,
            "[The Clarion](https://masthead.invalid)",
            "A skeptical court broadsheet with dry restraint.",
            4,
        )
        .await
        .unwrap();
        let page = render_world_newspaper_markdown(&composition.issue);
        let audit = render_world_newspaper_audit_markdown(&composition.issue);

        assert!(!page.contains("[The Clarion](https://masthead.invalid)"));
        assert!(!page.contains("[Court](https://headline.invalid)"));
        assert!(!page.contains("<img src=x>"));
        assert!(!page.contains("\n- The Thorn Court"));
        assert!(!page.contains("\n1. The dismissed treasurer"));
        assert!(!page.contains("~~prettier~~"));
        assert!(page.contains("\\[The Clarion\\](https://masthead.invalid)"));
        assert!(page.contains("&lt;img src=x&gt;"));
        assert!(page.contains("\\- The Thorn Court"));
        assert!(page.contains("1\\. The dismissed treasurer"));
        assert!(page.contains("*Current Edition*"));
        assert!(!page.contains("edition.invalid"));
        assert!(!audit.contains("[The Clarion](https://masthead.invalid)"));
        assert!(!audit.contains("[Court](https://headline.invalid)"));
    }

    #[test]
    fn lead_dateline_must_belong_to_its_cited_source() {
        let campaign = campaign_with_two_news();
        let records = public_news_records(&campaign).unwrap();
        let mut draft: EditorialPageDraft = serde_json::from_str(ACCEPTED_PAGE).unwrap();
        draft.articles[0].dateline.clear();
        let missing = validate_editorial_draft(&records, &draft, 4).unwrap_err();
        assert!(missing.to_string().contains("lead article must use"));

        draft.articles[0].dateline = "Yard".into();
        let unsupported = validate_editorial_draft(&records, &draft, 4).unwrap_err();
        assert!(unsupported.to_string().contains("misattributed a dateline"));

        draft.articles[0].dateline = "Room".into();
        draft.articles[0].citations.push("news:seal-scandal".into());
        let duplicate = validate_editorial_draft(&records, &draft, 4).unwrap_err();
        assert!(duplicate.to_string().contains("repeats a public record ID"));
    }

    #[tokio::test]
    async fn article_identity_covers_the_published_copy() {
        const ALTERNATE_PAGE: &str = r#"{"articles":[{"section":"Front Page","headline":"Court Sells the Crown's Seal, Then the Treasurer","deck":"The same scandal leaves the throne defending both its custody and its judgment.","byline":"By the political editor","dateline":"Room","citations":["news:seal-scandal"],"paragraphs":["The Thorn Court has admitted that its royal seal was pawned to cover a dragon's gambling debt, a confession that turns private embarrassment into a public question of custody.","The treasurer who delivered that admission in open court was dismissed soon afterward. The court has explained the firing; it has not made the seal any less pawned."]}]}"#;
        let campaign = campaign_with_news();
        let first = compose_world_newspaper(
            &ScriptedNewspaperModel::new([
                QUERY_ALL_RECORDS,
                ONE_STORY_AGENDA,
                ACCEPTED_PAGE,
                ACCEPTING_COPY_DESK,
            ]),
            &campaign,
            "The Underdeep Clarion",
            "A skeptical court broadsheet with dry restraint.",
            4,
        )
        .await
        .unwrap();
        let second = compose_world_newspaper(
            &ScriptedNewspaperModel::new([
                QUERY_ALL_RECORDS,
                ONE_STORY_AGENDA,
                ALTERNATE_PAGE,
                ACCEPTING_COPY_DESK,
            ]),
            &campaign,
            "The Underdeep Clarion",
            "A skeptical court broadsheet with dry restraint.",
            4,
        )
        .await
        .unwrap();

        assert_ne!(first.issue.articles[0].id, second.issue.articles[0].id);
    }
}
