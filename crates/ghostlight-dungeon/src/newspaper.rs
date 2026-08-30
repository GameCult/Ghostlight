use crate::{
    agent::{
        ModelAgentProgress, ModelAgentSpec, ModelAgentTool, ModelAgentToolContext,
        ModelAgentToolOutcome,
    },
    domain::{
        Campaign, Event, MAX_PUBLIC_EVENT_SUMMARY_CHARS, NewsIssue, PublicEventAssertionStatus,
    },
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
const MAX_NARRATIVE_SELECTION_STEPS: usize = 12;
const NEWSROOM_CONTRACT_VERSION: &str = "character-newsroom.v10";
const EDITION_LABEL: &str = "Current Edition";
const ALLOWED_SECTIONS: [&str; 6] = [
    "Front Page",
    "Realm Affairs",
    "Courts & Councils",
    "Guilds & Trade",
    "Dispatches",
    "Comment",
];

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct WorldNewspaperStaffMember {
    pub id: String,
    pub name: String,
    #[schemars(length(min = 1, max = 500))]
    pub character: String,
    #[schemars(length(min = 1, max = 6))]
    pub biases: Vec<String>,
    #[schemars(length(min = 1, max = 6))]
    pub preferences: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct WorldNewspaperJournalist {
    pub staff: WorldNewspaperStaffMember,
    #[schemars(length(min = 1, max = 60))]
    pub byline: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct WorldNewspaperNewsroom {
    pub assignment_editor: WorldNewspaperStaffMember,
    #[schemars(length(min = 1, max = 6))]
    pub journalists: Vec<WorldNewspaperJournalist>,
    pub copy_editor: WorldNewspaperStaffMember,
    pub night_editor: WorldNewspaperStaffMember,
}

pub fn canopy_ledger_newsroom() -> WorldNewspaperNewsroom {
    WorldNewspaperNewsroom {
        assignment_editor: WorldNewspaperStaffMember {
            id: "veyra-kest".into(),
            name: "Veyra Kest".into(),
            character: "A veteran city editor who treats official vagueness as a confession delivered without the useful nouns.".into(),
            biases: vec![
                "Institutional euphemism usually protects whoever already has a seal and a chair.".into(),
                "A public consequence outranks an announced intention.".into(),
            ],
            preferences: vec![
                "Conflict with named opposing actors and visible stakes.".into(),
                "Continuing stories whose new turn changes what readers should fear or demand.".into(),
            ],
        },
        journalists: vec![
            WorldNewspaperJournalist {
                staff: WorldNewspaperStaffMember {
                    id: "aven-tarl".into(),
                    name: "Aven Tarl".into(),
                    character: "A court reporter with immaculate cuffs, a long memory for humiliation, and no respect for inherited dignity once it becomes funny.".into(),
                    biases: vec!["Power is most legible when it is embarrassed in public.".into()],
                    preferences: vec!["Court scandal, succession, hypocrisy, and status reversals.".into()],
                },
                byline: "Aven Tarl".into(),
            },
            WorldNewspaperJournalist {
                staff: WorldNewspaperStaffMember {
                    id: "mera-quill".into(),
                    name: "Mera Quill".into(),
                    character: "A street correspondent who listens first to witnesses, children, mourners, and whoever must clean the room after policy leaves it.".into(),
                    biases: vec!["Procedure is often the name authority gives to somebody else's pain.".into()],
                    preferences: vec!["Bodies, witnesses, private cost, public anger, and grotesque physical evidence.".into()],
                },
                byline: "Mera Quill".into(),
            },
            WorldNewspaperJournalist {
                staff: WorldNewspaperStaffMember {
                    id: "ossan-reed".into(),
                    name: "Ossan Reed".into(),
                    character: "A former works clerk who can smell a failing pump, a cooked ledger, and management's preferred order for blaming the shift.".into(),
                    biases: vec!["Infrastructure failures are political decisions with tools attached.".into()],
                    preferences: vec!["Labor, shortages, hazardous works, guild conflict, and material consequences.".into()],
                },
                byline: "Ossan Reed".into(),
            },
            WorldNewspaperJournalist {
                staff: WorldNewspaperStaffMember {
                    id: "lysa-fen".into(),
                    name: "Lysa Fen".into(),
                    character: "A border correspondent who reads every truce as a maneuver and every maneuver as somebody else's future funeral.".into(),
                    biases: vec!["Security claims deserve to be read from the exposed side of the boundary.".into()],
                    preferences: vec!["Borders, armed orders, treaties, migrations, and rival countermoves.".into()],
                },
                byline: "Lysa Fen".into(),
            },
            WorldNewspaperJournalist {
                staff: WorldNewspaperStaffMember {
                    id: "corin-vey".into(),
                    name: "Corin Vey".into(),
                    character: "A trade columnist who believes every noble abstraction eventually arrives as a toll, an empty stall, or a favor owed.".into(),
                    biases: vec!["Markets reveal alliances that proclamations try to hide.".into()],
                    preferences: vec!["Trade, tolls, communes, debt, patronage, and who profits from disorder.".into()],
                },
                byline: "Corin Vey".into(),
            },
        ],
        copy_editor: WorldNewspaperStaffMember {
            id: "dalen-marr".into(),
            name: "Dalen Marr".into(),
            character: "A copy editor with a jeweller's patience for exact language and a personal grudge against causation smuggled through a comma.".into(),
            biases: vec!["Certainty is expensive and reporters spend it too freely.".into()],
            preferences: vec!["Complete factual query lists, exact quoted passages, and no proposed prose.".into()],
        },
        night_editor: WorldNewspaperStaffMember {
            id: "meret-sorn".into(),
            name: "Meret Sorn".into(),
            character: "A deadline editor who can remove a libelous hinge without sanding the blade off the sentence around it.".into(),
            biases: vec!["A late correction is survivable; a page that says nothing is not worth printing.".into()],
            preferences: vec!["The smallest repair that clears every lodged objection and preserves the reporter's case.".into()],
        },
    }
}

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

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct WorldNewspaperComposition {
    pub schema: String,
    pub issue: WorldNewspaperIssue,
    pub copy_desk: WorldNewspaperCopyDeskReport,
    pub press_close: WorldNewspaperPressClose,
    pub model_receipts: Vec<ModelStageReceipt>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct WorldNewspaperCopyDeskReport {
    #[schemars(length(min = 1, max = 500))]
    pub assessment: String,
    #[schemars(length(max = 24))]
    pub queries: Vec<WorldNewspaperGroundingFinding>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct WorldNewspaperPressClose {
    pub schema: String,
    pub source_checkpoint_id: String,
    pub copy_desk_receipt_id: String,
    pub night_editor_receipt_id: String,
    pub night_editor_action_applied: bool,
    pub addressed_query_indices: Vec<u16>,
    pub changed_article_indices: Vec<u16>,
    pub source_page_digest: String,
    pub printed_page_digest: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum WorldNewspaperCloseOrigin {
    InitialCopyDesk,
    LegacyV7Checkpoint,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct WorldNewspaperCloseCheckpoint {
    schema: String,
    id: String,
    publication_task_binding: String,
    editorial_binding: String,
    origin: WorldNewspaperCloseOrigin,
    source_checkpoint_id: Option<String>,
    editorial_agenda: WorldNewspaperEditorialAgenda,
    draft: EditorialPageDraft,
    copy_desk: WorldNewspaperCopyDeskReport,
    model_receipt_ids: Vec<String>,
    receipt_chain_digest: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "stage", rename_all = "snake_case", deny_unknown_fields)]
enum WorldNewspaperProductionContent {
    EditorialAgenda {
        editorial_agenda: WorldNewspaperEditorialAgenda,
    },
    FiledArticle {
        agenda_checkpoint_id: String,
        article_index: u16,
        assignment_digest: String,
        journalist_id: String,
        article: EditorialArticleDraft,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct WorldNewspaperProductionCheckpoint {
    schema: String,
    id: String,
    publication_task_binding: String,
    editorial_binding: String,
    content: WorldNewspaperProductionContent,
    model_receipt_ids: Vec<String>,
    receipt_chain_digest: String,
    content_digest: String,
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
    #[serde(default)]
    pub section: String,
    #[serde(default)]
    pub journalist_id: String,
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
    FetchPublicRecords {
        #[schemars(length(min = 1, max = 24))]
        record_ids: Vec<String>,
    },
    ProposeAgenda {
        #[schemars(length(min = 1, max = 500))]
        dominant_throughline: String,
        #[schemars(length(min = 1, max = 500))]
        reader_stake: String,
        #[schemars(length(min = 1, max = 6))]
        story_pitches: Vec<WorldNewspaperStoryPitch>,
    },
    CommitAgenda {
        #[schemars(length(min = 71, max = 71))]
        candidate_digest: String,
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
    AgendaProposed {
        candidate_digest: String,
        front_page: EditorialPitchProof,
        below_fold: Vec<EditorialPitchProof>,
        review_questions: Vec<String>,
    },
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
struct EditorialPitchProof {
    page_position: String,
    narrative_claim: String,
    tension: String,
    public_question: String,
    focus_record: PublicRecordProjection,
    supporting_records: Vec<EditorialSupportingRecord>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
struct EditorialSupportingRecord {
    record_id: String,
    at: DateTime<Utc>,
    headline: String,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
struct EditorialRecordIndexEntry {
    record_id: String,
    at: DateTime<Utc>,
    channel: String,
    headline: String,
    named_entities: Vec<String>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
struct NarrativeSelectionContextSnapshot {
    inspected_record_count: usize,
    completed_query_count: usize,
    inspected_records: Vec<EditorialRecordIndexEntry>,
    pending_agenda_token: Option<String>,
}

struct NarrativeSelectionWorkbench<'a> {
    records: &'a [PublicRecordProjection],
    newsroom: &'a WorldNewspaperNewsroom,
    max_articles: usize,
    visible_record_ids: BTreeSet<String>,
    completed_queries: BTreeSet<PublicRecordQuery>,
    pending_agenda: Option<(String, WorldNewspaperEditorialAgenda)>,
}

impl NarrativeSelectionWorkbench<'_> {
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
        self.pending_agenda = None;
        let next_cursor = (matching.len() > records.len())
            .then(|| records.last().map(|record| record.record_id.clone()))
            .flatten();
        Ok(NarrativeSelectionFinding::QueryResult {
            records,
            next_cursor,
        })
    }

    fn fetch_records(&mut self, mut record_ids: Vec<String>) -> Result<NarrativeSelectionFinding> {
        for record_id in &mut record_ids {
            *record_id = record_id.trim().to_owned();
            if record_id.is_empty()
                || record_id.chars().count() > 240
                || record_id.chars().any(char::is_control)
            {
                return Err(anyhow!("public record ID is malformed"));
            }
        }
        record_ids.sort();
        record_ids.dedup();
        if record_ids.is_empty() || record_ids.len() > MAX_PUBLIC_RECORD_QUERY_RESULTS {
            return Err(anyhow!(
                "exact public-record fetch is outside the bounded response"
            ));
        }
        let known = self
            .records
            .iter()
            .map(|record| (record.record_id.as_str(), record))
            .collect::<BTreeMap<_, _>>();
        let records = record_ids
            .iter()
            .map(|record_id| {
                known
                    .get(record_id.as_str())
                    .map(|record| (*record).clone())
                    .ok_or_else(|| {
                        anyhow!("exact public-record fetch names unknown record {record_id}")
                    })
            })
            .collect::<Result<Vec<_>>>()?;
        self.visible_record_ids
            .extend(records.iter().map(|record| record.record_id.clone()));
        self.pending_agenda = None;
        Ok(NarrativeSelectionFinding::QueryResult {
            records,
            next_cursor: None,
        })
    }

    fn context(&self) -> NarrativeSelectionContextSnapshot {
        let inspected_records = self
            .records
            .iter()
            .filter(|record| self.visible_record_ids.contains(&record.record_id))
            .map(|record| {
                let named_entities = record
                    .facts
                    .iter()
                    .flat_map(|fact| {
                        fact.named_people
                            .iter()
                            .map(|person| person.name.clone())
                            .chain(fact.institutions.iter().cloned())
                            .chain(fact.populations.iter().cloned())
                            .chain(fact.places.iter().cloned())
                    })
                    .collect::<BTreeSet<_>>()
                    .into_iter()
                    .collect();
                EditorialRecordIndexEntry {
                    record_id: record.record_id.clone(),
                    at: record.at,
                    channel: record.channel.clone(),
                    headline: record.headline.clone(),
                    named_entities,
                }
            })
            .collect();
        NarrativeSelectionContextSnapshot {
            inspected_record_count: self.visible_record_ids.len(),
            completed_query_count: self.completed_queries.len(),
            inspected_records,
            pending_agenda_token: self
                .pending_agenda
                .as_ref()
                .map(|(digest, _)| digest.clone()),
        }
    }

    fn propose_agenda(
        &mut self,
        agenda: WorldNewspaperEditorialAgenda,
    ) -> Result<NarrativeSelectionFinding> {
        let uninspected_record = agenda
            .story_pitches
            .iter()
            .flat_map(|pitch| &pitch.citations)
            .find(|record_id| !self.visible_record_ids.contains(*record_id));
        if let Some(record_id) = uninspected_record {
            return Err(anyhow!(
                "agenda cites public record {record_id} without querying it"
            ));
        }
        validate_editorial_agenda(self.records, self.newsroom, &agenda, self.max_articles)?;
        let candidate_digest = editorial_agenda_candidate_digest(&agenda)?;
        let mut pitches = agenda
            .story_pitches
            .iter()
            .enumerate()
            .map(|(index, pitch)| self.pitch_proof(index, pitch))
            .collect::<Result<Vec<_>>>()?;
        let front_page = pitches.remove(0);
        self.pending_agenda = Some((candidate_digest.clone(), agenda));
        Ok(NarrativeSelectionFinding::AgendaProposed {
            candidate_digest,
            front_page,
            below_fold: pitches,
            review_questions: vec![
                "Compare the lead focus fact with every below-fold focus fact. Would a reader reasonably call another story more vivid, consequential, scandalous, or urgent? If so, propose a revised agenda rather than committing this one."
                    .into(),
                "Does each continuing story foreground the incident and then identify what changed, or has routine handling displaced the thing readers actually care about?"
                    .into(),
                "Do the pitches expose people or institutions in conflict and make public or material stakes legible, or do they mainly explain what records do not prove? Query for missing opposition, countermoves, reaction, or lived cost when the ledger may contain them."
                    .into(),
            ],
        })
    }

    fn pitch_proof(
        &self,
        index: usize,
        pitch: &WorldNewspaperStoryPitch,
    ) -> Result<EditorialPitchProof> {
        let focus_record = self
            .records
            .iter()
            .find(|record| record.record_id == pitch.focus_citation)
            .cloned()
            .ok_or_else(|| anyhow!("agenda focus record is absent from the public ledger"))?;
        let supporting_records = pitch
            .citations
            .iter()
            .filter(|record_id| **record_id != pitch.focus_citation)
            .map(|record_id| {
                let record = self
                    .records
                    .iter()
                    .find(|record| &record.record_id == record_id)
                    .ok_or_else(|| {
                        anyhow!("agenda supporting record is absent from the public ledger")
                    })?;
                Ok(EditorialSupportingRecord {
                    record_id: record.record_id.clone(),
                    at: record.at,
                    headline: record.headline.clone(),
                })
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(EditorialPitchProof {
            page_position: if index == 0 {
                "front_page_lead".into()
            } else {
                format!("inside_article_{}", index + 1)
            },
            narrative_claim: pitch.narrative_claim.clone(),
            tension: pitch.tension.clone(),
            public_question: pitch.public_question.clone(),
            focus_record,
            supporting_records,
        })
    }
}

fn editorial_agenda_candidate_digest(agenda: &WorldNewspaperEditorialAgenda) -> Result<String> {
    Ok(format!(
        "sha256:{:x}",
        Sha256::digest(rmp_serde::to_vec_named(agenda)?)
    ))
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
        let story_budget = self.max_articles;
        let journalist_ids = self
            .newsroom
            .journalists
            .iter()
            .map(|journalist| journalist.staff.id.clone())
            .collect::<Vec<_>>();
        let propose_schema = serde_json::json!({
            "type":"object",
            "additionalProperties":false,
            "required":[
                "tool",
                "dominant_throughline",
                "reader_stake",
                "story_pitches"
            ],
            "properties":{
                "tool":{"const":"propose_agenda"},
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
                            "section",
                            "journalist_id",
                            "citations",
                            "focus_citation",
                            "narrative_claim",
                            "tension",
                            "public_question"
                        ],
                        "properties":{
                            "lead":{"type":"boolean"},
                            "section":{"type":"string","enum":ALLOWED_SECTIONS},
                            "journalist_id":{"type":"string","enum":journalist_ids},
                            "citations":{
                                "type":"array",
                                "minItems":1,
                                "uniqueItems":true,
                                "items":{"type":"string","minLength":1,"maxLength":240}
                            },
                            "focus_citation":{"type":"string","minLength":1,"maxLength":240},
                            "narrative_claim":{"type":"string","minLength":1,"maxLength":500},
                            "tension":{"type":"string","minLength":1,"maxLength":500},
                            "public_question":{"type":"string","minLength":1,"maxLength":500}
                        }
                    }
                }
            }
        });
        let cursor_schema = serde_json::json!({"anyOf":[
            {"type":"string","minLength":1,"maxLength":240},
            {"type":"null"}
        ]});
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
                    "items":{"type":"string","minLength":1,"maxLength":240}},
                "order":{"type":"string","enum":["newest","oldest"]},
                "cursor":cursor_schema,
                "limit":{"type":"integer","minimum":1,"maximum":MAX_PUBLIC_RECORD_QUERY_RESULTS}
            }
        });
        let commands = vec![
            query_schema,
            serde_json::json!({
                "type":"object",
                "additionalProperties":false,
                "required":["tool","record_ids"],
                "properties":{
                    "tool":{"const":"fetch_public_records"},
                    "record_ids":{
                        "type":"array",
                        "minItems":1,
                        "maxItems":MAX_PUBLIC_RECORD_QUERY_RESULTS,
                        "uniqueItems":true,
                        "items":{"type":"string","minLength":1,"maxLength":240}
                    }
                }
            }),
            propose_schema,
            serde_json::json!({
                "type":"object",
                "additionalProperties":false,
                "required":["tool","candidate_digest"],
                "properties":{
                    "tool":{"const":"commit_agenda"},
                    "candidate_digest":{"type":"string","minLength":71,"maxLength":71}
                }
            }),
        ];
        let command_schema = serde_json::json!({"oneOf":commands});
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

    fn context_snapshot(&self) -> Option<serde_json::Value> {
        serde_json::to_value(self.context()).ok()
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
            NarrativeSelectionCommand::FetchPublicRecords { record_ids } => {
                match self.fetch_records(record_ids) {
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
                }
            }
            NarrativeSelectionCommand::ProposeAgenda {
                dominant_throughline,
                reader_stake,
                story_pitches,
            } => {
                let agenda = WorldNewspaperEditorialAgenda {
                    dominant_throughline,
                    reader_stake,
                    story_pitches,
                };
                match self.propose_agenda(agenda) {
                    Ok(observation) => ModelAgentToolOutcome::Continue {
                        observation,
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
            NarrativeSelectionCommand::CommitAgenda { candidate_digest } => {
                match &self.pending_agenda {
                    Some((expected_digest, agenda)) if &candidate_digest == expected_digest => {
                        ModelAgentToolOutcome::Accepted {
                            output: agenda.clone(),
                            receipts: Vec::new(),
                        }
                    }
                    Some((expected_digest, _)) => ModelAgentToolOutcome::Rejected {
                        finding: NarrativeSelectionFinding::AgendaRejected {
                            reason: format!(
                                "agenda commit names stale candidate {candidate_digest}; current candidate is {expected_digest}"
                            ),
                        },
                        receipts: Vec::new(),
                    },
                    None => ModelAgentToolOutcome::Rejected {
                        finding: NarrativeSelectionFinding::AgendaRejected {
                            reason: "agenda commit requires one reviewed proposal".into(),
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

fn validate_staff_member(member: &WorldNewspaperStaffMember, label: &str) -> Result<()> {
    validate_single_line(&member.id, 80, &format!("{label} ID"))?;
    validate_single_line(&member.name, 80, &format!("{label} name"))?;
    validate_editorial_frame(&member.character, &format!("{label} character"))?;
    if member.biases.is_empty()
        || member.biases.len() > 6
        || member.preferences.is_empty()
        || member.preferences.len() > 6
    {
        return Err(anyhow!("{label} has an invalid disposition"));
    }
    for bias in &member.biases {
        validate_editorial_frame(bias, &format!("{label} bias"))?;
    }
    for preference in &member.preferences {
        validate_editorial_frame(preference, &format!("{label} preference"))?;
    }
    Ok(())
}

fn validate_newsroom(newsroom: &WorldNewspaperNewsroom) -> Result<()> {
    if newsroom.journalists.is_empty() || newsroom.journalists.len() > MAX_FRONT_PAGE_ARTICLES {
        return Err(anyhow!("newsroom must employ one to six journalists"));
    }
    validate_staff_member(&newsroom.assignment_editor, "assignment editor")?;
    validate_staff_member(&newsroom.copy_editor, "copy editor")?;
    validate_staff_member(&newsroom.night_editor, "Night Editor")?;
    let mut staff_ids = BTreeSet::from([
        newsroom.assignment_editor.id.as_str(),
        newsroom.copy_editor.id.as_str(),
        newsroom.night_editor.id.as_str(),
    ]);
    let mut staff_names = BTreeSet::from([
        newsroom.assignment_editor.name.as_str(),
        newsroom.copy_editor.name.as_str(),
        newsroom.night_editor.name.as_str(),
    ]);
    if staff_ids.len() != 3 || staff_names.len() != 3 {
        return Err(anyhow!("newsroom staff identities must be unique"));
    }
    let mut bylines = BTreeSet::new();
    for journalist in &newsroom.journalists {
        validate_staff_member(&journalist.staff, "journalist")?;
        validate_single_line(&journalist.byline, 60, "journalist byline")?;
        if !staff_ids.insert(&journalist.staff.id)
            || !staff_names.insert(&journalist.staff.name)
            || !bylines.insert(journalist.byline.as_str())
        {
            return Err(anyhow!(
                "newsroom staff identities and bylines must be unique"
            ));
        }
    }
    Ok(())
}

fn journalist_by_id<'a>(
    newsroom: &'a WorldNewspaperNewsroom,
    journalist_id: &str,
) -> Result<&'a WorldNewspaperJournalist> {
    newsroom
        .journalists
        .iter()
        .find(|journalist| journalist.staff.id == journalist_id)
        .ok_or_else(|| anyhow!("editorial pitch assigned an unknown journalist {journalist_id}"))
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
    newsroom: &WorldNewspaperNewsroom,
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
        if (index == 0 && pitch.section != "Front Page")
            || (index > 0 && pitch.section == "Front Page")
            || !ALLOWED_SECTIONS.contains(&pitch.section.as_str())
        {
            return Err(anyhow!("editorial pitch {index} chose an invalid section"));
        }
        journalist_by_id(newsroom, &pitch.journalist_id)?;
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
    newsroom: &WorldNewspaperNewsroom,
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
        let journalist = journalist_by_id(newsroom, &pitch.journalist_id)?;
        if article.section != pitch.section || article.byline != journalist.byline {
            return Err(anyhow!(
                "article {index} changed its assigned section or journalist"
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

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "tool", rename_all = "snake_case", deny_unknown_fields)]
enum JournalistAction {
    FileStory {
        #[schemars(length(min = 1, max = 100))]
        headline: String,
        #[schemars(length(min = 1, max = 220))]
        deck: String,
        #[schemars(length(max = 100))]
        dateline: String,
        #[schemars(length(min = 2, max = 5))]
        paragraphs: Vec<String>,
    },
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum JournalistFinding {
    StoryRejected { reason: String },
}

struct JournalistWorkbench<'a> {
    records: &'a [PublicRecordProjection],
    article_index: usize,
    section: &'a str,
    byline: &'a str,
    citations: &'a [String],
}

#[async_trait]
impl ModelAgentTool for JournalistWorkbench<'_> {
    type Action = JournalistAction;
    type Output = EditorialArticleDraft;
    type Finding = JournalistFinding;

    fn action_schema(&self) -> std::result::Result<serde_json::Value, String> {
        let mut datelines = self
            .records
            .iter()
            .filter(|record| self.citations.contains(&record.record_id))
            .flat_map(|record| record.facts.iter())
            .flat_map(|fact| fact.places.iter().cloned())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        if self.article_index > 0 || datelines.is_empty() {
            datelines.insert(0, String::new());
        }
        let mut schema = serde_json::json!({
            "type":"object",
            "additionalProperties":false,
            "required":["tool","headline","deck","dateline","paragraphs"],
            "properties":{
                "tool":{"const":"file_story"},
                "headline":{"type":"string","minLength":1,"maxLength":100},
                "deck":{"type":"string","minLength":1,"maxLength":220},
                "dateline":{"type":"string","enum":datelines},
                "paragraphs":{
                    "type":"array",
                    "minItems":2,
                    "maxItems":5,
                    "items":{"type":"string"}
                }
            }
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
        let JournalistAction::FileStory {
            headline,
            deck,
            dateline,
            paragraphs,
        } = action;
        let article = EditorialArticleDraft {
            section: self.section.into(),
            headline,
            deck,
            byline: self.byline.into(),
            dateline,
            citations: self.citations.to_vec(),
            paragraphs,
        };
        match validate_editorial_article(self.records, &article, self.article_index) {
            Ok(()) => ModelAgentToolOutcome::Accepted {
                output: article,
                receipts: Vec::new(),
            },
            Err(error) => ModelAgentToolOutcome::Rejected {
                finding: JournalistFinding::StoryRejected {
                    reason: error.to_string().chars().take(500).collect(),
                },
                receipts: Vec::new(),
            },
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "tool", rename_all = "snake_case", deny_unknown_fields)]
enum NightEditorCloseAction {
    SubmitClose {
        #[schemars(length(max = 24))]
        addressed_query_indices: Vec<u16>,
        #[schemars(length(max = 6))]
        rewrites: Vec<NightEditorArticleClose>,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct NightEditorArticleClose {
    article_index: u16,
    #[schemars(length(min = 1, max = 100))]
    headline: String,
    #[schemars(length(min = 1, max = 220))]
    deck: String,
    #[schemars(length(max = 100))]
    dateline: String,
    #[schemars(length(min = 2, max = 5))]
    paragraphs: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum NightEditorCloseFinding {
    CloseRejected {
        draft: EditorialPageDraft,
        copy_desk: WorldNewspaperCopyDeskReport,
        reason: String,
    },
}

fn night_editor_close_finding(
    draft: EditorialPageDraft,
    copy_desk: WorldNewspaperCopyDeskReport,
    reason: impl Into<String>,
) -> NightEditorCloseFinding {
    NightEditorCloseFinding::CloseRejected {
        draft,
        copy_desk,
        reason: reason.into(),
    }
}

#[derive(Debug)]
struct NightEditorCloseOutput {
    draft: EditorialPageDraft,
    addressed_query_indices: Vec<u16>,
    changed_article_indices: Vec<u16>,
}

struct PreparedNewspaper {
    title: String,
    editorial_voice: String,
    newsroom: WorldNewspaperNewsroom,
    records: Vec<PublicRecordProjection>,
    source_receipt_ids: Vec<String>,
    publication_task_binding: String,
    binding: String,
}

fn prepare_newspaper(
    campaign: &Campaign,
    title: impl Into<String>,
    editorial_voice: impl Into<String>,
    newsroom: &WorldNewspaperNewsroom,
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
    validate_newsroom(newsroom)?;
    let records = public_news_records(campaign)?;
    let source_receipt_ids = records
        .iter()
        .map(|record| record.record_id.clone())
        .collect::<Vec<_>>();
    let publication_task_binding =
        publication_task_binding(campaign, &title, &editorial_voice, newsroom, max_articles)?;
    let binding = editorial_binding(
        campaign,
        &title,
        &editorial_voice,
        newsroom,
        max_articles,
        &records,
    )?;
    Ok(PreparedNewspaper {
        title,
        editorial_voice,
        newsroom: newsroom.clone(),
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
    let assignment_staff = serde_json::json!({
        "assignment_editor": &prepared.newsroom.assignment_editor,
        "journalists": &prepared.newsroom.journalists,
    });
    let instructions = format!(
        "You are {editor}, assignment editor of `{title}`. Investigate its frozen public ledger with query_public_records, then assign a page. An empty query browses; terms, exact entity names, status, channel, and an inspected cursor narrow or page. The workbench keeps a compact index of inspected records instead of replaying old query pages; use fetch_public_records with exact IDs when you need their full facts again. Cite only returned record IDs. Search backward when procedure hides the rupture and sideways for opposition, countermoves, reaction, scandal, and lived cost.\n\nChoose one dominant throughline, then only stories that sharpen or complicate it. The first pitch is the Front Page lead; later pitches use another allowed section. For every pitch choose the staff journalist whose biases and preferences will produce the strongest treatment. Give that journalist the exact record grouping, one focus record that the lede cannot bury, a pointed narrative claim, the live tension, and the public question. A routine update to a vivid continuing incident should cite both and say what changed. Remembering, filing, warning, and planning are context unless they themselves produce a public consequence. Editorial frames may insinuate and judge but cannot invent concrete facts. The copy editor, not you or the reporters, owns fact checking.\n\nPropose before committing. The workbench returns the lead beside what it would bury; revise or query again if the proof exposes a stronger page.\n\nPUBLICATION VOICE:\n{voice}\n\nSTAFF BOOK:\n{staff}",
        editor = prepared.newsroom.assignment_editor.name,
        title = prepared.title,
        voice = prepared.editorial_voice,
        staff = serde_json::to_string(&assignment_staff).unwrap_or_default(),
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
        newsroom: &prepared.newsroom,
        max_articles,
        visible_record_ids: BTreeSet::new(),
        completed_queries: BTreeSet::new(),
        pending_agenda: None,
    };
    crate::agent::run_model_agent(model, &spec, &mut tool).await
}

fn checkpoint_receipt_chain_digest(receipt_ids: &[String]) -> Result<String> {
    Ok(format!(
        "sha256:{:x}",
        Sha256::digest(rmp_serde::to_vec_named(receipt_ids)?)
    ))
}

fn editorial_page_digest(draft: &EditorialPageDraft) -> Result<String> {
    Ok(format!(
        "sha256:{:x}",
        Sha256::digest(rmp_serde::to_vec_named(draft)?)
    ))
}

fn production_agenda_checkpoint_id(prepared: &PreparedNewspaper) -> Result<String> {
    Ok(format!(
        "newspaper-production-agenda:sha256:{:x}",
        Sha256::digest(rmp_serde::to_vec_named(&(
            "editorial_agenda",
            &prepared.publication_task_binding,
            &prepared.binding,
        ))?)
    ))
}

fn reporter_assignment_digest(
    prepared: &PreparedNewspaper,
    agenda_checkpoint_id: &str,
    article_index: usize,
    pitch: &WorldNewspaperStoryPitch,
    journalist: &WorldNewspaperJournalist,
) -> Result<String> {
    Ok(format!(
        "sha256:{:x}",
        Sha256::digest(rmp_serde::to_vec_named(&(
            &prepared.publication_task_binding,
            &prepared.binding,
            agenda_checkpoint_id,
            article_index,
            pitch,
            journalist,
        ))?)
    ))
}

fn production_article_checkpoint_id(
    prepared: &PreparedNewspaper,
    agenda_checkpoint_id: &str,
    article_index: usize,
    assignment_digest: &str,
) -> Result<String> {
    Ok(format!(
        "newspaper-production-article:sha256:{:x}",
        Sha256::digest(rmp_serde::to_vec_named(&(
            &prepared.publication_task_binding,
            &prepared.binding,
            agenda_checkpoint_id,
            article_index,
            assignment_digest,
        ))?)
    ))
}

fn load_model_receipts(
    store: &CampaignStore,
    receipt_ids: &[String],
    owner: &str,
) -> Result<Vec<ModelStageReceipt>> {
    receipt_ids
        .iter()
        .map(|receipt_id| {
            store
                .load::<ModelStageReceipt>("persona_stage_receipt.v1", receipt_id)?
                .map(|(_, receipt)| receipt)
                .ok_or_else(|| anyhow!("{owner} lost model receipt {receipt_id}"))
        })
        .collect()
}

fn validate_production_receipts(
    checkpoint: &WorldNewspaperProductionCheckpoint,
    receipts: &[ModelStageReceipt],
    expected_stage: &str,
    expected_snapshot_binding: &str,
) -> Result<()> {
    let receipt_ids = receipts
        .iter()
        .map(|receipt| receipt.storage_key().to_owned())
        .collect::<Vec<_>>();
    if receipts.is_empty()
        || receipt_ids != checkpoint.model_receipt_ids
        || receipt_ids.iter().collect::<BTreeSet<_>>().len() != receipt_ids.len()
        || checkpoint_receipt_chain_digest(&receipt_ids)? != checkpoint.receipt_chain_digest
        || production_checkpoint_content_digest(
            &checkpoint.publication_task_binding,
            &checkpoint.editorial_binding,
            &checkpoint.content,
            &checkpoint.model_receipt_ids,
            &checkpoint.receipt_chain_digest,
        )? != checkpoint.content_digest
        || receipts.iter().any(|receipt| {
            receipt.stage != expected_stage || receipt.snapshot_binding != expected_snapshot_binding
        })
        || receipts
            .last()
            .is_none_or(|receipt| receipt.validation_result != "valid")
    {
        return Err(anyhow!(
            "newspaper production checkpoint receipt binding is invalid"
        ));
    }
    Ok(())
}

fn production_checkpoint_content_digest(
    publication_task_binding: &str,
    editorial_binding: &str,
    content: &WorldNewspaperProductionContent,
    model_receipt_ids: &[String],
    receipt_chain_digest: &str,
) -> Result<String> {
    Ok(format!(
        "sha256:{:x}",
        Sha256::digest(rmp_serde::to_vec_named(&(
            publication_task_binding,
            editorial_binding,
            content,
            model_receipt_ids,
            receipt_chain_digest,
        ))?)
    ))
}

fn persist_production_checkpoint(
    store: &CampaignStore,
    checkpoint: &WorldNewspaperProductionCheckpoint,
    receipts: &[ModelStageReceipt],
) -> Result<()> {
    store.persist_model_stage_receipts(receipts)?;
    store.insert(
        "world_newspaper_production_checkpoint.v1",
        "ghostlight.world_newspaper_production_checkpoint.v1",
        &checkpoint.id,
        checkpoint,
    )?;
    Ok(())
}

fn new_agenda_checkpoint(
    prepared: &PreparedNewspaper,
    agenda: WorldNewspaperEditorialAgenda,
    receipts: &[ModelStageReceipt],
) -> Result<WorldNewspaperProductionCheckpoint> {
    let model_receipt_ids = receipts
        .iter()
        .map(|receipt| receipt.storage_key().to_owned())
        .collect::<Vec<_>>();
    let content = WorldNewspaperProductionContent::EditorialAgenda {
        editorial_agenda: agenda,
    };
    let receipt_chain_digest = checkpoint_receipt_chain_digest(&model_receipt_ids)?;
    let content_digest = production_checkpoint_content_digest(
        &prepared.publication_task_binding,
        &prepared.binding,
        &content,
        &model_receipt_ids,
        &receipt_chain_digest,
    )?;
    Ok(WorldNewspaperProductionCheckpoint {
        schema: "ghostlight.world_newspaper_production_checkpoint.v1".into(),
        id: production_agenda_checkpoint_id(prepared)?,
        publication_task_binding: prepared.publication_task_binding.clone(),
        editorial_binding: prepared.binding.clone(),
        content,
        receipt_chain_digest,
        model_receipt_ids,
        content_digest,
    })
}

fn load_agenda_checkpoint(
    store: &CampaignStore,
    prepared: &PreparedNewspaper,
    max_articles: usize,
) -> Result<
    Option<(
        String,
        WorldNewspaperEditorialAgenda,
        Vec<ModelStageReceipt>,
    )>,
> {
    let id = production_agenda_checkpoint_id(prepared)?;
    let Some((_, checkpoint)) = store.load::<WorldNewspaperProductionCheckpoint>(
        "world_newspaper_production_checkpoint.v1",
        &id,
    )?
    else {
        return Ok(None);
    };
    let receipts = load_model_receipts(store, &checkpoint.model_receipt_ids, "agenda checkpoint")?;
    let WorldNewspaperProductionContent::EditorialAgenda { editorial_agenda } = &checkpoint.content
    else {
        return Err(anyhow!(
            "agenda checkpoint occupies the wrong production slot"
        ));
    };
    if checkpoint.schema != "ghostlight.world_newspaper_production_checkpoint.v1"
        || checkpoint.id != id
        || checkpoint.publication_task_binding != prepared.publication_task_binding
        || checkpoint.editorial_binding != prepared.binding
    {
        return Err(anyhow!("newspaper agenda checkpoint identity is invalid"));
    }
    validate_production_receipts(
        &checkpoint,
        &receipts,
        "newspaper_narrative_selection_agent_action",
        &prepared.binding,
    )?;
    validate_editorial_agenda(
        &prepared.records,
        &prepared.newsroom,
        editorial_agenda,
        max_articles,
    )?;
    Ok(Some((id, editorial_agenda.clone(), receipts)))
}

fn new_article_checkpoint(
    prepared: &PreparedNewspaper,
    agenda_checkpoint_id: &str,
    article_index: usize,
    pitch: &WorldNewspaperStoryPitch,
    journalist: &WorldNewspaperJournalist,
    article: EditorialArticleDraft,
    receipts: &[ModelStageReceipt],
) -> Result<WorldNewspaperProductionCheckpoint> {
    let assignment_digest = reporter_assignment_digest(
        prepared,
        agenda_checkpoint_id,
        article_index,
        pitch,
        journalist,
    )?;
    let model_receipt_ids = receipts
        .iter()
        .map(|receipt| receipt.storage_key().to_owned())
        .collect::<Vec<_>>();
    let id = production_article_checkpoint_id(
        prepared,
        agenda_checkpoint_id,
        article_index,
        &assignment_digest,
    )?;
    let content = WorldNewspaperProductionContent::FiledArticle {
        agenda_checkpoint_id: agenda_checkpoint_id.into(),
        article_index: article_index
            .try_into()
            .map_err(|_| anyhow!("newspaper article index overflow"))?,
        assignment_digest,
        journalist_id: journalist.staff.id.clone(),
        article,
    };
    let receipt_chain_digest = checkpoint_receipt_chain_digest(&model_receipt_ids)?;
    let content_digest = production_checkpoint_content_digest(
        &prepared.publication_task_binding,
        &prepared.binding,
        &content,
        &model_receipt_ids,
        &receipt_chain_digest,
    )?;
    Ok(WorldNewspaperProductionCheckpoint {
        schema: "ghostlight.world_newspaper_production_checkpoint.v1".into(),
        id,
        publication_task_binding: prepared.publication_task_binding.clone(),
        editorial_binding: prepared.binding.clone(),
        content,
        receipt_chain_digest,
        model_receipt_ids,
        content_digest,
    })
}

fn load_article_checkpoint(
    store: &CampaignStore,
    prepared: &PreparedNewspaper,
    agenda_checkpoint_id: &str,
    article_index: usize,
    pitch: &WorldNewspaperStoryPitch,
    journalist: &WorldNewspaperJournalist,
) -> Result<Option<(EditorialArticleDraft, Vec<ModelStageReceipt>)>> {
    let assignment_digest = reporter_assignment_digest(
        prepared,
        agenda_checkpoint_id,
        article_index,
        pitch,
        journalist,
    )?;
    let id = production_article_checkpoint_id(
        prepared,
        agenda_checkpoint_id,
        article_index,
        &assignment_digest,
    )?;
    let Some((_, checkpoint)) = store.load::<WorldNewspaperProductionCheckpoint>(
        "world_newspaper_production_checkpoint.v1",
        &id,
    )?
    else {
        return Ok(None);
    };
    let receipts = load_model_receipts(store, &checkpoint.model_receipt_ids, "article checkpoint")?;
    let WorldNewspaperProductionContent::FiledArticle {
        agenda_checkpoint_id: stored_agenda_checkpoint_id,
        article_index: stored_article_index,
        assignment_digest: stored_assignment_digest,
        journalist_id,
        article,
    } = &checkpoint.content
    else {
        return Err(anyhow!(
            "article checkpoint occupies the wrong production slot"
        ));
    };
    let snapshot_binding = format!(
        "{}:journalist:{}:article:{}",
        prepared.binding, journalist.staff.id, article_index
    );
    if checkpoint.schema != "ghostlight.world_newspaper_production_checkpoint.v1"
        || checkpoint.id != id
        || checkpoint.publication_task_binding != prepared.publication_task_binding
        || checkpoint.editorial_binding != prepared.binding
        || stored_agenda_checkpoint_id != agenda_checkpoint_id
        || usize::from(*stored_article_index) != article_index
        || stored_assignment_digest != &assignment_digest
        || journalist_id != &journalist.staff.id
    {
        return Err(anyhow!("newspaper article checkpoint identity is invalid"));
    }
    validate_production_receipts(
        &checkpoint,
        &receipts,
        "newspaper_journalist_agent_action",
        &snapshot_binding,
    )?;
    validate_editorial_article(&prepared.records, article, article_index)?;
    if article.section != pitch.section
        || article.byline != journalist.byline
        || article.citations.iter().collect::<BTreeSet<_>>()
            != pitch.citations.iter().collect::<BTreeSet<_>>()
    {
        return Err(anyhow!(
            "newspaper article checkpoint changed its exact assignment"
        ));
    }
    Ok(Some((article.clone(), receipts)))
}

async fn run_assigned_journalists(
    model: &dyn ModelPort,
    prepared: &PreparedNewspaper,
    agenda: &WorldNewspaperEditorialAgenda,
    agenda_checkpoint_id: &str,
    selection_receipts: &[ModelStageReceipt],
    store: &CampaignStore,
) -> std::result::Result<
    (EditorialPageDraft, Vec<ModelStageReceipt>),
    crate::agent::ModelAgentFailure,
> {
    let mut articles = Vec::with_capacity(agenda.story_pitches.len());
    let mut receipts = Vec::new();
    for (index, pitch) in agenda.story_pitches.iter().enumerate() {
        let journalist = match journalist_by_id(&prepared.newsroom, &pitch.journalist_id) {
            Ok(journalist) => journalist,
            Err(error) => {
                return Err(crate::agent::ModelAgentFailure {
                    message: error.to_string(),
                    receipts,
                });
            }
        };
        match load_article_checkpoint(
            store,
            prepared,
            agenda_checkpoint_id,
            index,
            pitch,
            journalist,
        ) {
            Ok(Some((article, article_receipts))) => {
                articles.push(article);
                receipts.extend(article_receipts);
                continue;
            }
            Ok(None) => {}
            Err(error) => {
                return Err(crate::agent::ModelAgentFailure {
                    message: error.to_string(),
                    receipts,
                });
            }
        }
        let selected = pitch.citations.iter().cloned().collect::<BTreeSet<_>>();
        let source_json =
            serde_json::to_string(&public_records_for_ids(&prepared.records, &selected))
                .unwrap_or_else(|_| "[]".into());
        let assignment = serde_json::json!({
            "dominant_throughline": agenda.dominant_throughline,
            "reader_stake": agenda.reader_stake,
            "pitch": pitch,
            "journalist": journalist,
            "records": serde_json::from_str::<serde_json::Value>(&source_json)
                .unwrap_or_else(|_| serde_json::json!([])),
        });
        let instructions = format!(
            "You are {name}, filing one assigned story for `{title}`. You are a partisan storyteller with a nose for the material your character and biases care about, not the newsroom's fact checker. Make the strongest memorable case this assignment supports: lead with the focus event, use selection and juxtaposition to imply meaning, expose hypocrisy or conflict, and make public or bodily cost vivid. A continuing story should say what changed without treating routine handling as the lede.\n\nUse only the supplied records for concrete events, people, institutions, places, identity attributes, chronology, quotations, motives, and outcomes. Preserve disputes through natural attribution. You may be cutting, emotional, insinuating, metaphorical, or politically judgmental without pretending the insinuation is a sourced causal fact. Do not discuss records, citations, verification, assertion statuses, or missing evidence in reader copy. The copy editor will fact-check the filed story later. File two to five substantial paragraphs. The lead needs a supplied place as dateline when one exists; otherwise leave dateline empty.\n\nPUBLICATION VOICE:\n{voice}\n\nASSIGNMENT PACKET:\n{assignment}",
            name = journalist.staff.name,
            title = prepared.title,
            voice = prepared.editorial_voice,
            assignment = serde_json::to_string(&assignment).unwrap_or_default(),
        );
        let mut causal_sources = pitch
            .citations
            .iter()
            .cloned()
            .chain(
                selection_receipts
                    .iter()
                    .map(|receipt| receipt.storage_key().to_owned()),
            )
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        causal_sources.sort();
        let spec = ModelAgentSpec {
            stage: "newspaper_journalist_agent_action".into(),
            model: MODEL_CAPABLE.into(),
            snapshot_binding: format!(
                "{}:journalist:{}:article:{}",
                prepared.binding, journalist.staff.id, index
            ),
            instructions,
            source_receipt_ids: causal_sources,
            temperature: Some(0.9),
            max_output_tokens: Some(2_200),
            max_steps: 2,
        };
        let mut workbench = JournalistWorkbench {
            records: &prepared.records,
            article_index: index,
            section: &pitch.section,
            byline: &journalist.byline,
            citations: &pitch.citations,
        };
        match crate::agent::run_model_agent(model, &spec, &mut workbench).await {
            Ok(run) => {
                let checkpoint = match new_article_checkpoint(
                    prepared,
                    agenda_checkpoint_id,
                    index,
                    pitch,
                    journalist,
                    run.output.clone(),
                    &run.receipts,
                ) {
                    Ok(checkpoint) => checkpoint,
                    Err(error) => {
                        let mut failed_receipts = receipts;
                        failed_receipts.extend(run.receipts);
                        return Err(crate::agent::ModelAgentFailure {
                            message: error.to_string(),
                            receipts: failed_receipts,
                        });
                    }
                };
                if let Err(error) = persist_production_checkpoint(store, &checkpoint, &run.receipts)
                {
                    let mut failed_receipts = receipts;
                    failed_receipts.extend(run.receipts);
                    return Err(crate::agent::ModelAgentFailure {
                        message: error.to_string(),
                        receipts: failed_receipts,
                    });
                }
                articles.push(run.output);
                receipts.extend(run.receipts);
            }
            Err(mut failure) => {
                let persistence_error = store
                    .persist_model_stage_receipts(&failure.receipts)
                    .err()
                    .map(|error| error.to_string());
                receipts.append(&mut failure.receipts);
                return Err(crate::agent::ModelAgentFailure {
                    message: persistence_error.map_or(failure.message.clone(), |error| {
                        format!(
                            "{}; failed to persist rejected journalist receipts: {error}",
                            failure.message
                        )
                    }),
                    receipts,
                });
            }
        }
    }
    Ok((EditorialPageDraft { articles }, receipts))
}

fn close_checkpoint_identity(
    publication_task_binding: &str,
    editorial_binding: &str,
    origin: &WorldNewspaperCloseOrigin,
    source_checkpoint_id: Option<&str>,
    editorial_agenda: &WorldNewspaperEditorialAgenda,
    draft: &EditorialPageDraft,
    copy_desk: &WorldNewspaperCopyDeskReport,
    model_receipt_ids: &[String],
    receipt_chain_digest: &str,
) -> Result<String> {
    Ok(format!(
        "newspaper-close:sha256:{:x}",
        Sha256::digest(rmp_serde::to_vec_named(&(
            publication_task_binding,
            editorial_binding,
            origin,
            source_checkpoint_id,
            editorial_agenda,
            draft,
            copy_desk,
            model_receipt_ids,
            receipt_chain_digest,
        ))?)
    ))
}

fn new_close_checkpoint(
    prepared: &PreparedNewspaper,
    origin: WorldNewspaperCloseOrigin,
    source_checkpoint_id: Option<String>,
    editorial_agenda: WorldNewspaperEditorialAgenda,
    draft: EditorialPageDraft,
    copy_desk: WorldNewspaperCopyDeskReport,
    receipts: &[ModelStageReceipt],
) -> Result<WorldNewspaperCloseCheckpoint> {
    let model_receipt_ids = receipts
        .iter()
        .map(|receipt| receipt.storage_key().to_owned())
        .collect::<Vec<_>>();
    let receipt_chain_digest = checkpoint_receipt_chain_digest(&model_receipt_ids)?;
    let id = close_checkpoint_identity(
        &prepared.publication_task_binding,
        &prepared.binding,
        &origin,
        source_checkpoint_id.as_deref(),
        &editorial_agenda,
        &draft,
        &copy_desk,
        &model_receipt_ids,
        &receipt_chain_digest,
    )?;
    Ok(WorldNewspaperCloseCheckpoint {
        schema: "ghostlight.world_newspaper_close_checkpoint.v1".into(),
        id,
        publication_task_binding: prepared.publication_task_binding.clone(),
        editorial_binding: prepared.binding.clone(),
        origin,
        source_checkpoint_id,
        editorial_agenda,
        draft,
        copy_desk,
        model_receipt_ids,
        receipt_chain_digest,
    })
}

fn persist_close_checkpoint(
    store: &CampaignStore,
    checkpoint: &WorldNewspaperCloseCheckpoint,
) -> Result<()> {
    let kind = "world_newspaper_close_checkpoint.v1";
    if let Some((_, existing)) =
        store.load::<WorldNewspaperCloseCheckpoint>(kind, &checkpoint.id)?
    {
        if existing != *checkpoint {
            return Err(anyhow!(
                "immutable newspaper close checkpoint conflict: {}",
                checkpoint.id
            ));
        }
        return Ok(());
    }
    store.insert(
        kind,
        "ghostlight.world_newspaper_close_checkpoint.v1",
        &checkpoint.id,
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
    let kind = "world_newspaper_composition.v3";
    let record = PersistedWorldNewspaperComposition {
        schema: "ghostlight.persisted_world_newspaper_composition.v3".into(),
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
        "ghostlight.persisted_world_newspaper_composition.v3",
        publication_task_binding,
        &record,
    )?;
    Ok(())
}

fn validate_copy_desk_report(
    draft: &EditorialPageDraft,
    report: &WorldNewspaperCopyDeskReport,
) -> Result<()> {
    validate_single_line(&report.assessment, 500, "copy-desk assessment")?;
    for query in &report.queries {
        validate_single_line(&query.claim_or_phrase, 500, "copy-desk claim")?;
        validate_single_line(&query.reason, 500, "copy-desk reason")?;
        validate_grounding_finding_target(draft, query)?;
    }
    Ok(())
}

fn validate_close_checkpoint(
    checkpoint: &WorldNewspaperCloseCheckpoint,
    prepared: &PreparedNewspaper,
    max_articles: usize,
    receipts: &[ModelStageReceipt],
) -> Result<()> {
    if checkpoint.schema != "ghostlight.world_newspaper_close_checkpoint.v1"
        || checkpoint.publication_task_binding != prepared.publication_task_binding
        || checkpoint.editorial_binding != prepared.binding
        || receipts.len() != checkpoint.model_receipt_ids.len()
    {
        return Err(anyhow!("newspaper close checkpoint is invalid"));
    }
    if checkpoint.origin != WorldNewspaperCloseOrigin::InitialCopyDesk
        || checkpoint.source_checkpoint_id.is_some()
    {
        return Err(anyhow!(
            "newspaper close checkpoint does not belong to the current newsroom"
        ));
    }
    let receipt_ids = receipts
        .iter()
        .map(|receipt| receipt.storage_key().to_owned())
        .collect::<Vec<_>>();
    if receipt_ids != checkpoint.model_receipt_ids
        || receipt_ids.iter().collect::<BTreeSet<_>>().len() != receipt_ids.len()
        || checkpoint_receipt_chain_digest(&receipt_ids)? != checkpoint.receipt_chain_digest
        || close_checkpoint_identity(
            &checkpoint.publication_task_binding,
            &checkpoint.editorial_binding,
            &checkpoint.origin,
            checkpoint.source_checkpoint_id.as_deref(),
            &checkpoint.editorial_agenda,
            &checkpoint.draft,
            &checkpoint.copy_desk,
            &checkpoint.model_receipt_ids,
            &checkpoint.receipt_chain_digest,
        )? != checkpoint.id
    {
        return Err(anyhow!(
            "newspaper close checkpoint receipt binding is invalid"
        ));
    }
    if receipts
        .iter()
        .rev()
        .find(|receipt| receipt.stage == "newspaper_copy_desk")
        .map(|receipt| receipt.storage_key())
        != receipt_ids.last().map(String::as_str)
    {
        return Err(anyhow!(
            "newspaper close checkpoint lost its final copy-desk receipt"
        ));
    }
    validate_editorial_agenda(
        &prepared.records,
        &prepared.newsroom,
        &checkpoint.editorial_agenda,
        max_articles,
    )?;
    validate_editorial_draft(&prepared.records, &checkpoint.draft, max_articles)?;
    validate_editorial_alignment(
        &checkpoint.draft,
        &prepared.newsroom,
        &checkpoint.editorial_agenda,
    )?;
    validate_copy_desk_report(&checkpoint.draft, &checkpoint.copy_desk)?;
    Ok(())
}

fn load_close_checkpoint(
    store: &CampaignStore,
    prepared: &PreparedNewspaper,
    max_articles: usize,
) -> Result<Option<(WorldNewspaperCloseCheckpoint, Vec<ModelStageReceipt>)>> {
    let checkpoints = store
        .load_all::<WorldNewspaperCloseCheckpoint>("world_newspaper_close_checkpoint.v1")?
        .into_iter()
        .filter(|checkpoint| {
            checkpoint.origin == WorldNewspaperCloseOrigin::InitialCopyDesk
                && checkpoint.publication_task_binding == prepared.publication_task_binding
        })
        .collect::<Vec<_>>();
    if checkpoints.is_empty() {
        return Ok(None);
    }
    if checkpoints.len() != 1 {
        return Err(anyhow!(
            "newspaper close task has multiple immutable checkpoints"
        ));
    }
    let checkpoint = checkpoints.into_iter().next().expect("one checkpoint");
    let receipts = load_model_receipts(store, &checkpoint.model_receipt_ids, "newspaper close")?;
    validate_close_checkpoint(&checkpoint, prepared, max_articles, &receipts)?;
    Ok(Some((checkpoint, receipts)))
}

fn editorial_draft_from_issue(issue: &WorldNewspaperIssue) -> Result<EditorialPageDraft> {
    let articles = issue
        .articles
        .iter()
        .map(|article| {
            let citations = article
                .sources
                .iter()
                .map(|source| {
                    if source.source_news_ids.len() != 1 {
                        return Err(anyhow!(
                            "printed newspaper source does not name exactly one public record"
                        ));
                    }
                    Ok(source.source_news_ids[0].clone())
                })
                .collect::<Result<Vec<_>>>()?;
            Ok(EditorialArticleDraft {
                section: article.section.clone(),
                headline: article.headline.clone(),
                deck: article.deck.clone(),
                byline: article.byline.clone(),
                dateline: article.dateline.clone().unwrap_or_default(),
                citations,
                paragraphs: article.paragraphs.clone(),
            })
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(EditorialPageDraft { articles })
}

fn validate_persisted_composition(
    store: &CampaignStore,
    campaign: &Campaign,
    prepared: &PreparedNewspaper,
    max_articles: usize,
    persisted: &PersistedWorldNewspaperComposition,
) -> Result<()> {
    let composition = &persisted.composition;
    if persisted.schema != "ghostlight.persisted_world_newspaper_composition.v3"
        || persisted.publication_task_binding != prepared.publication_task_binding
        || persisted.editorial_binding != prepared.binding
        || composition.schema != "ghostlight.world_newspaper_composition.v3"
        || composition.issue.schema != "ghostlight.world_newspaper_issue.v3"
        || composition.issue.source_world_revision != campaign.revision
        || composition.issue.title != prepared.title
        || composition.issue.edition_label != EDITION_LABEL
    {
        return Err(anyhow!("persisted newspaper composition is invalid"));
    }
    if composition.issue.articles.is_empty() {
        if persisted.source_checkpoint_id.is_some()
            || composition.issue.id
                != empty_issue_id(campaign, &prepared.title, &prepared.newsroom)?
            || composition.issue.at != campaign.world_time
            || composition.issue.lead_article_id.is_some()
            || composition.issue.editorial_agenda.is_some()
            || !composition.issue.editorial_receipt_ids.is_empty()
            || !composition.model_receipts.is_empty()
            || !composition.copy_desk.queries.is_empty()
            || composition.press_close.schema != "ghostlight.world_newspaper_press_close.v1"
            || composition.press_close.source_checkpoint_id != "no-edition"
            || !composition.press_close.copy_desk_receipt_id.is_empty()
            || !composition.press_close.night_editor_receipt_id.is_empty()
            || composition.press_close.night_editor_action_applied
            || !composition.press_close.addressed_query_indices.is_empty()
            || !composition.press_close.changed_article_indices.is_empty()
            || composition.press_close.source_page_digest != "no-edition"
            || composition.press_close.printed_page_digest != "no-edition"
        {
            return Err(anyhow!("persisted no-edition newspaper is invalid"));
        }
        return Ok(());
    }

    let source_checkpoint_id = persisted
        .source_checkpoint_id
        .as_deref()
        .ok_or_else(|| anyhow!("persisted newspaper lost its close checkpoint"))?;
    let checkpoint = store
        .load::<WorldNewspaperCloseCheckpoint>(
            "world_newspaper_close_checkpoint.v1",
            source_checkpoint_id,
        )?
        .map(|(_, checkpoint)| checkpoint)
        .ok_or_else(|| anyhow!("persisted newspaper close checkpoint is missing"))?;
    let checkpoint_receipts =
        load_model_receipts(store, &checkpoint.model_receipt_ids, "persisted newspaper")?;
    validate_close_checkpoint(&checkpoint, prepared, max_articles, &checkpoint_receipts)?;
    if composition.model_receipts.len() != checkpoint_receipts.len() + 1
        || !composition.model_receipts.starts_with(&checkpoint_receipts)
        || composition.copy_desk != checkpoint.copy_desk
        || composition.issue.editorial_agenda.as_ref() != Some(&checkpoint.editorial_agenda)
        || composition.press_close.schema != "ghostlight.world_newspaper_press_close.v1"
        || composition.press_close.source_checkpoint_id != checkpoint.id
        || composition.press_close.copy_desk_receipt_id
            != checkpoint
                .model_receipt_ids
                .last()
                .cloned()
                .unwrap_or_default()
    {
        return Err(anyhow!("persisted newspaper press lineage is invalid"));
    }
    for receipt in &composition.model_receipts {
        let stored = store
            .load::<ModelStageReceipt>("persona_stage_receipt.v1", receipt.storage_key())?
            .map(|(_, stored)| stored)
            .ok_or_else(|| anyhow!("persisted newspaper model receipt is missing"))?;
        if !stored.same_receipted_content(receipt) {
            return Err(anyhow!("persisted newspaper model receipt changed"));
        }
    }
    let night_editor_receipt = composition
        .model_receipts
        .last()
        .expect("non-empty edition has one closing receipt");
    if night_editor_receipt.stage != "newspaper_night_editor_close_agent_action"
        || composition.press_close.night_editor_receipt_id != night_editor_receipt.storage_key()
    {
        return Err(anyhow!("persisted newspaper lost its Night Editor close"));
    }

    let printed_draft = editorial_draft_from_issue(&composition.issue)?;
    let addressed_queries = composition
        .press_close
        .addressed_query_indices
        .iter()
        .map(|index| usize::from(*index))
        .collect::<BTreeSet<_>>();
    let expected_queries = (0..checkpoint.copy_desk.queries.len()).collect::<BTreeSet<_>>();
    let changed_articles = composition
        .press_close
        .changed_article_indices
        .iter()
        .map(|index| usize::from(*index))
        .collect::<BTreeSet<_>>();
    let required_changed_articles = checkpoint
        .copy_desk
        .queries
        .iter()
        .map(|query| usize::from(query.article_index))
        .collect::<BTreeSet<_>>();
    if editorial_page_digest(&checkpoint.draft)? != composition.press_close.source_page_digest
        || editorial_page_digest(&printed_draft)? != composition.press_close.printed_page_digest
        || composition.issue.editorial_receipt_ids
            != composition
                .model_receipts
                .iter()
                .map(|receipt| receipt.storage_key().to_owned())
                .collect::<Vec<_>>()
        || (composition.press_close.night_editor_action_applied
            && (addressed_queries != expected_queries
                || addressed_queries.len()
                    != composition.press_close.addressed_query_indices.len()
                || changed_articles != required_changed_articles
                || changed_articles.len() != composition.press_close.changed_article_indices.len()
                || changed_articles
                    .iter()
                    .any(|index| *index >= checkpoint.draft.articles.len())))
        || (!composition.press_close.night_editor_action_applied
            && (!addressed_queries.is_empty()
                || !changed_articles.is_empty()
                || printed_draft != checkpoint.draft
                || night_editor_receipt.validation_result != "semantic_invalid"))
    {
        return Err(anyhow!("persisted newspaper press witness is invalid"));
    }
    let expected_issue = lower_editorial_page(
        campaign,
        prepared.title.clone(),
        &prepared.records,
        Some(checkpoint.editorial_agenda),
        printed_draft,
        &composition.model_receipts,
    )?;
    if composition.issue != expected_issue {
        return Err(anyhow!("persisted newspaper issue cannot be re-derived"));
    }
    Ok(())
}

async fn close_world_newspaper(
    model: &dyn ModelPort,
    campaign: &Campaign,
    prepared: &PreparedNewspaper,
    max_articles: usize,
    store: &CampaignStore,
    checkpoint: WorldNewspaperCloseCheckpoint,
    mut receipts: Vec<ModelStageReceipt>,
) -> Result<WorldNewspaperComposition> {
    validate_close_checkpoint(&checkpoint, prepared, max_articles, &receipts)?;
    let source_json = source_json_for_copy_queries(
        &prepared.records,
        &checkpoint.editorial_agenda,
        &checkpoint.copy_desk,
    )?;
    let progress = match run_night_editor_close(
        model,
        &prepared.records,
        max_articles,
        &prepared.binding,
        &prepared.title,
        &prepared.editorial_voice,
        &prepared.newsroom,
        &checkpoint.editorial_agenda,
        &source_json,
        &prepared.source_receipt_ids,
        &receipts,
        &checkpoint.id,
        checkpoint.draft.clone(),
        checkpoint.copy_desk.clone(),
    )
    .await
    {
        Ok(progress) => progress,
        Err(failure) => {
            store.persist_model_stage_receipts(&failure.receipts)?;
            receipts.extend(failure.receipts);
            return Err(composition_failure(failure.message, receipts));
        }
    };
    let (
        closing_receipts,
        final_draft,
        action_applied,
        addressed_query_indices,
        changed_article_indices,
    ) = match progress {
        ModelAgentProgress::Accepted(run) => (
            run.receipts,
            run.output.draft,
            true,
            run.output.addressed_query_indices,
            run.output.changed_article_indices,
        ),
        ModelAgentProgress::Exhausted(exhausted) => (
            exhausted.receipts,
            checkpoint.draft.clone(),
            false,
            Vec::new(),
            Vec::new(),
        ),
    };
    let night_editor_receipt_id = closing_receipts
        .last()
        .map(|receipt| receipt.storage_key().to_owned())
        .ok_or_else(|| anyhow!("Night Editor close produced no model receipt"))?;
    store.persist_model_stage_receipts(&closing_receipts)?;
    receipts.extend(closing_receipts);
    let issue = lower_editorial_page(
        campaign,
        prepared.title.clone(),
        &prepared.records,
        Some(checkpoint.editorial_agenda.clone()),
        final_draft.clone(),
        &receipts,
    )?;
    let press_close = WorldNewspaperPressClose {
        schema: "ghostlight.world_newspaper_press_close.v1".into(),
        source_checkpoint_id: checkpoint.id.clone(),
        copy_desk_receipt_id: checkpoint
            .model_receipt_ids
            .last()
            .cloned()
            .ok_or_else(|| anyhow!("newspaper close checkpoint has no copy-desk receipt"))?,
        night_editor_receipt_id,
        night_editor_action_applied: action_applied,
        addressed_query_indices,
        changed_article_indices,
        source_page_digest: editorial_page_digest(&checkpoint.draft)?,
        printed_page_digest: editorial_page_digest(&final_draft)?,
    };
    let composition = WorldNewspaperComposition {
        schema: "ghostlight.world_newspaper_composition.v3".into(),
        issue,
        copy_desk: checkpoint.copy_desk,
        press_close,
        model_receipts: receipts,
    };
    persist_newspaper_completion(
        store,
        &prepared.publication_task_binding,
        &prepared.binding,
        Some(checkpoint.id),
        &composition,
    )?;
    Ok(composition)
}

pub async fn advance_world_newspaper(
    model: &dyn ModelPort,
    campaign: &Campaign,
    title: impl Into<String>,
    editorial_voice: impl Into<String>,
    newsroom: &WorldNewspaperNewsroom,
    max_articles: usize,
    store: &CampaignStore,
) -> Result<WorldNewspaperComposition> {
    let prepared = prepare_newspaper(campaign, title, editorial_voice, newsroom, max_articles)?;
    if let Some((_, persisted)) = store.load::<PersistedWorldNewspaperComposition>(
        "world_newspaper_composition.v3",
        &prepared.publication_task_binding,
    )? {
        validate_persisted_composition(store, campaign, &prepared, max_articles, &persisted)?;
        return Ok(persisted.composition);
    }
    if let Some((checkpoint, receipts)) = load_close_checkpoint(store, &prepared, max_articles)? {
        return close_world_newspaper(
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
            id: empty_issue_id(campaign, &prepared.title, &prepared.newsroom)?,
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
            schema: "ghostlight.world_newspaper_composition.v3".into(),
            issue,
            copy_desk: WorldNewspaperCopyDeskReport {
                assessment: "No public source material was available, so no edition was issued."
                    .into(),
                queries: Vec::new(),
            },
            press_close: WorldNewspaperPressClose {
                schema: "ghostlight.world_newspaper_press_close.v1".into(),
                source_checkpoint_id: "no-edition".into(),
                copy_desk_receipt_id: String::new(),
                night_editor_receipt_id: String::new(),
                night_editor_action_applied: false,
                addressed_query_indices: Vec::new(),
                changed_article_indices: Vec::new(),
                source_page_digest: "no-edition".into(),
                printed_page_digest: "no-edition".into(),
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
        return Ok(composition);
    }
    let (agenda_checkpoint_id, agenda, mut receipts) =
        if let Some(checkpoint) = load_agenda_checkpoint(store, &prepared, max_articles)? {
            checkpoint
        } else {
            let agenda_run = match select_editorial_agenda(model, &prepared, max_articles).await {
                Ok(run) => run,
                Err(failure) => {
                    store.persist_model_stage_receipts(&failure.receipts)?;
                    return Err(composition_failure(failure.message, failure.receipts));
                }
            };
            let checkpoint =
                new_agenda_checkpoint(&prepared, agenda_run.output.clone(), &agenda_run.receipts)?;
            persist_production_checkpoint(store, &checkpoint, &agenda_run.receipts)?;
            (checkpoint.id, agenda_run.output, agenda_run.receipts)
        };
    let selected_source_json = source_json_for_agenda(&prepared.records, Some(&agenda))?;
    let (draft, journalist_receipts) = run_assigned_journalists(
        model,
        &prepared,
        &agenda,
        &agenda_checkpoint_id,
        &receipts,
        store,
    )
    .await
    .map_err(|failure| {
        let mut all_receipts = receipts.clone();
        all_receipts.extend(failure.receipts);
        composition_failure(failure.message, all_receipts)
    })?;
    receipts.extend(journalist_receipts);
    validate_editorial_draft(&prepared.records, &draft, max_articles)?;
    validate_editorial_alignment(&draft, &prepared.newsroom, &agenda)?;
    store.persist_model_stage_receipts(&receipts)?;
    let page_digest = editorial_page_digest(&draft)?;
    let journalist_receipt_ids = receipts
        .iter()
        .filter(|receipt| receipt.stage == "newspaper_journalist_agent_action")
        .map(|receipt| receipt.storage_key().to_owned())
        .collect::<Vec<_>>();
    let copy_desk = match run_copy_desk(
        model,
        format!("{}:draft:{page_digest}", prepared.binding),
        &selected_source_json,
        &prepared.newsroom.copy_editor,
        &prepared.source_receipt_ids,
        &journalist_receipt_ids,
        &draft,
        &mut receipts,
    )
    .await
    {
        Ok(report) => report,
        Err(error) => {
            store.persist_model_stage_receipts(&receipts)?;
            return Err(composition_failure(error.to_string(), receipts));
        }
    };
    store.persist_model_stage_receipts(&receipts)?;
    let checkpoint = new_close_checkpoint(
        &prepared,
        WorldNewspaperCloseOrigin::InitialCopyDesk,
        None,
        agenda,
        draft,
        copy_desk,
        &receipts,
    )?;
    validate_close_checkpoint(&checkpoint, &prepared, max_articles, &receipts)?;
    persist_close_checkpoint(store, &checkpoint)?;
    close_world_newspaper(
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
    advance_world_newspaper(
        model,
        campaign,
        title,
        editorial_voice,
        &canopy_ledger_newsroom(),
        max_articles,
        &store,
    )
    .await
}

struct NightEditorCloseWorkbench<'a> {
    records: &'a [PublicRecordProjection],
    max_articles: usize,
    draft: EditorialPageDraft,
    copy_desk: WorldNewspaperCopyDeskReport,
}

#[async_trait]
impl ModelAgentTool for NightEditorCloseWorkbench<'_> {
    type Action = NightEditorCloseAction;
    type Output = NightEditorCloseOutput;
    type Finding = NightEditorCloseFinding;

    fn action_schema(&self) -> std::result::Result<serde_json::Value, String> {
        let mut rewrite_schema = serde_json::to_value(schema_for!(NightEditorArticleClose))
            .map_err(|error| error.to_string())?;
        let definitions = rewrite_schema
            .as_object_mut()
            .and_then(|schema| schema.remove("$defs"))
            .unwrap_or_else(|| serde_json::json!({}));
        if let Some(schema) = rewrite_schema.as_object_mut() {
            schema.remove("$schema");
        }
        let article_indices = (0..self.max_articles)
            .map(serde_json::Value::from)
            .collect::<Vec<_>>();
        rewrite_schema["properties"]["article_index"] = serde_json::json!({
            "type":"integer",
            "enum":article_indices
        });
        let mut schema = serde_json::json!({
            "type":"object",
            "additionalProperties":false,
            "required":["tool", "addressed_query_indices", "rewrites"],
            "properties":{
                "tool":{"const":"submit_close"},
                "addressed_query_indices":{
                    "type":"array",
                    "uniqueItems":true,
                    "items":{"type":"integer", "minimum":0, "maximum":65535}
                },
                "rewrites":{
                    "type":"array",
                    "minItems":0,
                    "maxItems":self.max_articles,
                    "uniqueItems":true,
                    "items":rewrite_schema
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
        _context: &ModelAgentToolContext,
    ) -> ModelAgentToolOutcome<Self::Output, Self::Finding> {
        let output = match apply_night_editor_close(&self.draft, &self.copy_desk, action) {
            Ok(output) => output,
            Err(error) => {
                return ModelAgentToolOutcome::Rejected {
                    finding: night_editor_close_finding(
                        self.draft.clone(),
                        self.copy_desk.clone(),
                        format!("The proposed deadline close was not admitted: {error}"),
                    ),
                    receipts: Vec::new(),
                };
            }
        };
        if let Err(error) = validate_editorial_draft(self.records, &output.draft, self.max_articles)
        {
            return ModelAgentToolOutcome::Rejected {
                finding: night_editor_close_finding(
                    self.draft.clone(),
                    self.copy_desk.clone(),
                    format!("The proposed deadline close produced invalid copy: {error}"),
                ),
                receipts: Vec::new(),
            };
        }
        ModelAgentToolOutcome::Accepted {
            output,
            receipts: Vec::new(),
        }
    }
}

fn apply_night_editor_close(
    original: &EditorialPageDraft,
    copy_desk: &WorldNewspaperCopyDeskReport,
    action: NightEditorCloseAction,
) -> Result<NightEditorCloseOutput> {
    let NightEditorCloseAction::SubmitClose {
        addressed_query_indices,
        rewrites,
    } = action;
    let expected_queries = (0..copy_desk.queries.len()).collect::<BTreeSet<_>>();
    let addressed_queries = addressed_query_indices
        .iter()
        .map(|index| usize::from(*index))
        .collect::<BTreeSet<_>>();
    if addressed_queries != expected_queries
        || addressed_queries.len() != addressed_query_indices.len()
    {
        return Err(anyhow!(
            "the Night Editor must disposition every copy-desk query exactly once"
        ));
    }
    let affected = copy_desk
        .queries
        .iter()
        .map(|query| usize::from(query.article_index))
        .collect::<BTreeSet<_>>();
    if affected
        .iter()
        .any(|article_index| *article_index >= original.articles.len())
    {
        return Err(anyhow!("rejecting desk named an invalid article"));
    }
    let mut draft = original.clone();
    let mut rewritten = BTreeSet::new();
    for rewrite in rewrites {
        let article_index = usize::from(rewrite.article_index);
        if article_index >= original.articles.len() || !rewritten.insert(article_index) {
            return Err(anyhow!(
                "deadline close names an invalid or duplicate article"
            ));
        }
        let article = draft
            .articles
            .get_mut(article_index)
            .ok_or_else(|| anyhow!("deadline close names an invalid article"))?;
        article.headline = rewrite.headline;
        article.deck = rewrite.deck;
        article.dateline = rewrite.dateline;
        article.paragraphs = rewrite.paragraphs;
    }
    if rewritten != affected {
        return Err(anyhow!(
            "the Night Editor must rewrite exactly the copy-queried articles in one bounded pass"
        ));
    }
    Ok(NightEditorCloseOutput {
        draft,
        addressed_query_indices,
        changed_article_indices: rewritten.into_iter().map(|index| index as u16).collect(),
    })
}

#[allow(clippy::too_many_arguments)]
async fn run_night_editor_close(
    model: &dyn ModelPort,
    records: &[PublicRecordProjection],
    max_articles: usize,
    binding: &str,
    title: &str,
    editorial_voice: &str,
    newsroom: &WorldNewspaperNewsroom,
    agenda: &WorldNewspaperEditorialAgenda,
    source_json: &str,
    source_receipt_ids: &[String],
    prior_receipts: &[ModelStageReceipt],
    checkpoint_id: &str,
    draft: EditorialPageDraft,
    copy_desk: WorldNewspaperCopyDeskReport,
) -> std::result::Result<ModelAgentProgress<NightEditorCloseOutput>, crate::agent::ModelAgentFailure>
{
    let numbered_queries = copy_desk
        .queries
        .iter()
        .enumerate()
        .map(|(query_index, query)| serde_json::json!({"query_index":query_index, "query":query}))
        .collect::<Vec<_>>();
    let affected_article_indices = copy_desk
        .queries
        .iter()
        .map(|query| usize::from(query.article_index))
        .collect::<BTreeSet<_>>();
    let affected_assignments = agenda
        .story_pitches
        .iter()
        .enumerate()
        .filter(|(article_index, _)| affected_article_indices.contains(article_index))
        .map(|(article_index, pitch)| {
            serde_json::json!({"article_index": article_index, "assignment": pitch})
        })
        .collect::<Vec<_>>();
    let queried_articles = draft
        .articles
        .iter()
        .enumerate()
        .filter(|(article_index, _)| affected_article_indices.contains(article_index))
        .map(|(article_index, article)| {
            serde_json::json!({"article_index": article_index, "article": article})
        })
        .collect::<Vec<_>>();
    let affected_journalist_ids = copy_desk
        .queries
        .iter()
        .filter_map(|query| agenda.story_pitches.get(usize::from(query.article_index)))
        .map(|pitch| pitch.journalist_id.as_str())
        .collect::<BTreeSet<_>>();
    let affected_journalists = newsroom
        .journalists
        .iter()
        .filter(|journalist| affected_journalist_ids.contains(journalist.staff.id.as_str()))
        .collect::<Vec<_>>();
    let instructions = format!(
        "You are {night_editor}, Night Editor of `{title}`. The copy editor has lodged one complete numbered factual checklist and the press closes after this action. Address every item, replacing exactly the articles named by that checklist. Preserve each reporter's argument, urgency, style, and insinuation while making the smallest changes that resolve the cited defect. Do not select stories, widen sources, or improve unrelated articles. Sections, bylines, citations, story order, and assignments are frozen. Nobody rereads the final prose before printing; a later edition may correct a mistake.\n\nKeep distinct reports grammatically distinct when causation is not established and attribute disputes naturally. Do not expose the checklist or verification machinery in reader copy. If there are no queries, submit an unchanged close with empty lists.\n\nNIGHT EDITOR:\n{night_profile}\n\nAFFECTED REPORTERS:\n{reporters}\n\nPUBLICATION VOICE:\n{editorial_voice}\n\nAFFECTED ASSIGNMENTS:\n{agenda_json}\n\nFACTS FOR QUERIED ARTICLES:\n{source_json}\n\nQUERIED ARTICLES:\n{draft_json}\n\nNUMBERED QUERIES:\n{query_json}",
        night_editor = newsroom.night_editor.name,
        title = title,
        night_profile = serde_json::to_string(&newsroom.night_editor).unwrap_or_default(),
        reporters = serde_json::to_string(&affected_journalists).unwrap_or_default(),
        editorial_voice = editorial_voice,
        agenda_json = serde_json::to_string_pretty(&serde_json::json!({
            "dominant_throughline": agenda.dominant_throughline,
            "reader_stake": agenda.reader_stake,
            "assignments": affected_assignments,
        }))
        .unwrap_or_default(),
        source_json = source_json,
        draft_json = serde_json::to_string_pretty(&queried_articles).unwrap_or_default(),
        query_json = serde_json::to_string_pretty(&numbered_queries).unwrap_or_default(),
    );
    let mut causal_sources = source_receipt_ids
        .iter()
        .cloned()
        .chain(
            prior_receipts
                .iter()
                .map(|receipt| receipt.storage_key().to_owned()),
        )
        .chain(std::iter::once(checkpoint_id.to_owned()))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    causal_sources.sort();
    let spec = ModelAgentSpec {
        stage: "newspaper_night_editor_close_agent_action".into(),
        model: MODEL_BALANCED.into(),
        snapshot_binding: format!("{binding}:night-editor-close:{checkpoint_id}"),
        instructions,
        source_receipt_ids: causal_sources,
        temperature: Some(0.35),
        max_output_tokens: Some(4_500),
        max_steps: 1,
    };
    let mut tool = NightEditorCloseWorkbench {
        records,
        max_articles,
        draft,
        copy_desk,
    };
    crate::agent::run_model_agent_progress(model, &spec, &mut tool).await
}

async fn run_copy_desk(
    model: &dyn ModelPort,
    snapshot_binding: String,
    source_json: &str,
    copy_editor: &WorldNewspaperStaffMember,
    source_receipt_ids: &[String],
    editorial_source_receipt_ids: &[String],
    draft: &EditorialPageDraft,
    receipts: &mut Vec<ModelStageReceipt>,
) -> Result<WorldNewspaperCopyDeskReport> {
    let verifier_schema = serde_json::to_value(schema_for!(WorldNewspaperCopyDeskReport))?;
    let verifier_request = ModelStageRequest {
        stage: "newspaper_copy_desk".into(),
        model: MODEL_CAPABLE.into(),
        snapshot_binding,
        lived_stream: format!(
            "You are {}, the publication's copy editor. Act as the sole factual query desk, not a rewriting model or publication judge. Compare every concrete reader-facing claim with only its cited notes. Query invented or overconfident facts, quotations, identities, offices, places, numbers, motives, outcomes, chronology, spatial relationships, affiliations, or private knowledge. Assertion status is exhaustive: an attempted or adopted course does not prove embedded acts completed, and a public declaration proves only the declaration. Supported identity attributes are exhaustive; an empty list supports no pronoun, title, kinship, office, membership, or affiliation. Shared location does not establish unstated geometry. Preserve disputes through attribution.\n\nMetaphor, wit, contrast, opinion, political characterization, and juxtaposition are editorial language unless they smuggle in a concrete fact or causal outcome. Return the complete query set at once. Each claim_or_phrase must be an exact unique substring of its article. Never propose replacement copy and never query a staff byline merely because the world ledger does not name the employee.\n\nCOPY EDITOR:\n{}\n\nFACT DESK:\n{}\n\nPROPOSED PAGE:\n{}",
            copy_editor.name,
            serde_json::to_string(copy_editor)?,
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
    let report: WorldNewspaperCopyDeskReport = match serde_json::from_value(verifier_structured) {
        Ok(report) => report,
        Err(error) => {
            let error = anyhow!("newspaper copy desk returned an invalid report: {error}");
            mark_semantic_invalid(&mut receipts[receipt_index], &error);
            return Err(error);
        }
    };
    if let Err(error) = validate_copy_desk_report(draft, &report) {
        mark_semantic_invalid(&mut receipts[receipt_index], &error);
        return Err(error);
    }
    Ok(report)
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
                let account_boundary = if fact.account.chars().count()
                    == MAX_PUBLIC_EVENT_SUMMARY_CHARS
                {
                    format!(
                        "\n- Account boundary: This immutable source account reaches the owned {}-character public-event summary limit; the audit shows the complete stored account and asserts nothing beyond it.",
                        MAX_PUBLIC_EVENT_SUMMARY_CHARS
                    )
                } else {
                    String::new()
                };
                rendered.push_str(&format!(
                    "\n#### Fact {}\n\n- Exact committed account: {}{}\n- Assertion status: {}\n- Committed events: {}\n- Named people: {}\n- Institutions: {}\n- Populations: {}\n- Places: {}\n",
                    fact_index + 1,
                    escape_markdown_text(&fact.account),
                    account_boundary,
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

fn source_json_for_copy_queries(
    records: &[PublicRecordProjection],
    agenda: &WorldNewspaperEditorialAgenda,
    copy_desk: &WorldNewspaperCopyDeskReport,
) -> Result<String> {
    let mut record_ids = BTreeSet::new();
    for query in &copy_desk.queries {
        let pitch = agenda
            .story_pitches
            .get(usize::from(query.article_index))
            .ok_or_else(|| anyhow!("copy-desk query names an unknown article"))?;
        record_ids.extend(pitch.citations.iter().cloned());
    }
    Ok(serde_json::to_string_pretty(&public_records_for_ids(
        records,
        &record_ids,
    ))?)
}

fn validate_editorial_draft(
    records: &[PublicRecordProjection],
    draft: &EditorialPageDraft,
    max_articles: usize,
) -> Result<()> {
    if draft.articles.is_empty() || draft.articles.len() > max_articles.min(records.len()) {
        return Err(anyhow!("editorial page exceeded its story budget"));
    }
    let mut headlines = BTreeSet::new();
    for (index, article) in draft.articles.iter().enumerate() {
        validate_editorial_article(records, article, index)?;
        if !headlines.insert(article.headline.to_lowercase()) {
            return Err(anyhow!("front page repeats a headline"));
        }
    }
    Ok(())
}

fn validate_editorial_article(
    records: &[PublicRecordProjection],
    article: &EditorialArticleDraft,
    index: usize,
) -> Result<()> {
    if (index == 0 && article.section != "Front Page")
        || (index > 0 && article.section == "Front Page")
        || !ALLOWED_SECTIONS.contains(&article.section.as_str())
    {
        return Err(anyhow!("article {index} used an invalid section"));
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
    let known_sources = records
        .iter()
        .map(|record| record.record_id.as_str())
        .collect::<BTreeSet<_>>();
    for citation in selected_record_ids(&article.citations, &format!("article {index}"))? {
        if !known_sources.contains(citation) {
            return Err(anyhow!(
                "article {index} cites unknown public record {citation}"
            ));
        }
    }
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
    let cited_dateline_supported = article
        .citations
        .iter()
        .any(|citation| source_datelines[citation.as_str()].contains(article.dateline.as_str()));
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
            records
                .iter()
                .find(|record| record.record_id == *citation)
                .is_some_and(|record| record.facts.iter().any(|fact| fact.account.trim() == text))
        }) {
            return Err(anyhow!(
                "article {index} printed a source summary as final copy"
            ));
        }
    }
    validate_no_reader_plumbing(
        &format!(
            "{} {} {} {} {}",
            article.headline,
            article.deck,
            article.byline,
            article.dateline,
            article.paragraphs.join(" ")
        ),
        &format!("article {index}"),
    )
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

fn validate_grounding_finding_target(
    draft: &EditorialPageDraft,
    finding: &WorldNewspaperGroundingFinding,
) -> Result<()> {
    let occurrence_count =
        article_passage_occurrence_count(draft, finding.article_index, &finding.claim_or_phrase)
            .ok_or_else(|| anyhow!("copy desk returned an invalid article index"))?;
    if occurrence_count != 1 {
        return Err(anyhow!(
            "copy-desk claim_or_phrase must be one exact contiguous phrase occurring once in the named article"
        ));
    }
    Ok(())
}

fn article_passage_occurrence_count(
    draft: &EditorialPageDraft,
    article_index: u16,
    passage: &str,
) -> Option<usize> {
    let article = draft.articles.get(usize::from(article_index))?;
    Some(
        [
            article.headline.as_str(),
            article.deck.as_str(),
            article.dateline.as_str(),
        ]
        .into_iter()
        .chain(article.paragraphs.iter().map(String::as_str))
        .map(|text| text.match_indices(passage).count())
        .sum::<usize>(),
    )
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
    newsroom: &WorldNewspaperNewsroom,
    max_articles: usize,
) -> Result<String> {
    let bytes = rmp_serde::to_vec_named(&(
        NEWSROOM_CONTRACT_VERSION,
        campaign.id,
        campaign.revision,
        title,
        voice,
        newsroom,
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
    newsroom: &WorldNewspaperNewsroom,
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
        newsroom,
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

fn empty_issue_id(
    campaign: &Campaign,
    title: &str,
    newsroom: &WorldNewspaperNewsroom,
) -> Result<String> {
    let identity = rmp_serde::to_vec_named(&(
        NEWSROOM_CONTRACT_VERSION,
        campaign.id,
        campaign.revision,
        title,
        newsroom,
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
            if request.stage == "newspaper_narrative_selection_agent_action"
                && request.lived_stream.contains("candidate_digest")
            {
                let digest = request
                    .lived_stream
                    .match_indices("sha256:")
                    .filter_map(|(index, _)| request.lived_stream.get(index..index + 71))
                    .filter(|candidate| {
                        candidate[7..]
                            .chars()
                            .all(|character| character.is_ascii_hexdigit())
                    })
                    .last()
                    .ok_or_else(|| anyhow!("fixture transcript omitted candidate digest"))?;
                return Ok(serde_json::json!({
                    "command":{
                        "tool":"commit_agenda",
                        "candidate_digest":digest,
                    }
                })
                .to_string());
            }
            let response = self
                .responses
                .lock()
                .unwrap()
                .pop_front()
                .ok_or_else(|| anyhow!("fixture newspaper model exhausted"))?;
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
    const ONE_STORY_AGENDA: &str = r#"{"command":{"tool":"propose_agenda","dominant_throughline":"The court made its private gambling debt a public crisis of custody and punished the official who exposed it.","reader_stake":"Readers must decide whether dismissal protects the seal or merely the court from its own confession.","story_pitches":[{"lead":true,"section":"Front Page","journalist_id":"aven-tarl","citations":["news:seal-scandal"],"focus_citation":"news:seal-scandal","narrative_claim":"The pawned royal seal and the treasurer's dismissal are one scandal.","tension":"The court admits the loss while directing the immediate consequence at the bearer of that admission.","public_question":"Who is being held accountable for the missing seal?"}]}}"#;
    const TWO_STORY_AGENDA: &str = r#"{"command":{"tool":"propose_agenda","dominant_throughline":"Court custody fails at both the royal seal and the western gate.","reader_stake":"Readers depend on institutions that announce damage only after access or authority has already been compromised.","story_pitches":[{"lead":true,"section":"Front Page","journalist_id":"aven-tarl","citations":["news:seal-scandal"],"focus_citation":"news:seal-scandal","narrative_claim":"The pawned seal and dismissal are the court's crisis of custody.","tension":"The confession exposes the loss while the treasurer absorbs the immediate institutional consequence.","public_question":"Who is being held accountable for the missing seal?"},{"lead":false,"section":"Dispatches","journalist_id":"lysa-fen","citations":["news:west-gate"],"focus_citation":"news:west-gate","narrative_claim":"The gate closure is a practical echo of neglected custody.","tension":"A cracked hinge will close a route at moonrise while travellers wait for a reopening time.","public_question":"How long will the western route remain closed?"}]}}"#;
    const ACCEPTED_STORY_ACTION: &str = r#"{"tool":"file_story","headline":"Court Sells the Crown's Seal, Then the Treasurer","deck":"A gambling debt reaches the throne room and leaves one official carrying the blame.","dateline":"Room","paragraphs":["The Thorn Court has admitted that its royal seal was pawned to cover a dragon's gambling debt, a confession that turns private embarrassment into a public question of custody.","The treasurer who delivered that admission in open court was dismissed soon afterward. The court has explained the firing; it has not made the seal any less pawned."]}"#;
    const MALFORMED_LYSA_STORY_ACTION: &str = r#"{"tool":"file_story","headline":"Gate Trouble","deck":"The western route will close at moonrise.","dateline":"Yard","paragraphs":["Too short.","Still short."]}"#;
    const ACCEPTED_LYSA_STORY_ACTION: &str = r#"{"tool":"file_story","headline":"West Gate to Close at Moonrise","deck":"Masons will replace a cracked hinge after the palace bell keeper's warning.","dateline":"Yard","paragraphs":["Officials warn the west gate is unsafe, and the palace bell keeper says it will close at moonrise while masons replace its cracked hinge.","Travellers using the gate have been told when it will close, though no reopening hour was included in the announcement."]}"#;
    const ACCEPTED_PAGE: &str = r#"{"articles":[{"section":"Front Page","headline":"Court Sells the Crown's Seal, Then the Treasurer","deck":"A gambling debt reaches the throne room and leaves one official carrying the blame.","byline":"Aven Tarl","dateline":"Room","citations":["news:seal-scandal"],"paragraphs":["The Thorn Court has admitted that its royal seal was pawned to cover a dragon's gambling debt, a confession that turns private embarrassment into a public question of custody.","The treasurer who delivered that admission in open court was dismissed soon afterward. The court has explained the firing; it has not made the seal any less pawned."]}]}"#;
    const TWO_ARTICLE_PAGE: &str = r#"{"articles":[{"section":"Front Page","headline":"Court Sells the Crown's Seal, Then the Treasurer","deck":"A gambling debt reaches the throne room and leaves one official carrying the blame.","byline":"Aven Tarl","dateline":"Room","citations":["news:seal-scandal"],"paragraphs":["The Thorn Court has admitted that its royal seal was pawned to cover a dragon's gambling debt, a confession that turns private embarrassment into a public question of custody.","The treasurer who delivered that admission in open court was dismissed soon afterward. The court has explained the firing; it has not made the seal any less pawned."]},{"section":"Dispatches","headline":"West Gate to Close at Moonrise","deck":"Masons will replace a cracked hinge after the palace bell keeper's warning.","byline":"Lysa Fen","dateline":"Yard","citations":["news:west-gate"],"paragraphs":["Officials warn the west gate is unsafe, and the palace bell keeper says it will close at moonrise while masons replace its cracked hinge.","Travellers using the gate have been told when it will close, though no reopening hour was included in the announcement."]}]}"#;
    const UNCHANGED_NIGHT_CLOSE: &str =
        r#"{"tool":"submit_close","addressed_query_indices":[],"rewrites":[]}"#;
    const QUERY_DECK_CLOSE: &str = r#"{"tool":"submit_close","addressed_query_indices":[0],"rewrites":[{"article_index":0,"headline":"Court Sells the Crown's Seal, Then the Treasurer","deck":"The court admits the pawned seal and dismisses the official who carried the confession.","dateline":"Room","paragraphs":["The Thorn Court has admitted that its royal seal was pawned to cover a dragon's gambling debt, a confession that turns private embarrassment into a public question of custody.","The treasurer who delivered that admission in open court was dismissed soon afterward. The court has explained the firing; it has not made the seal any less pawned."]}]}"#;
    const ACCEPTING_COPY_DESK: &str = r#"{"assessment":"The copy is fully supported by its cited public source and reads as attributed court reporting.","queries":[]}"#;

    #[test]
    fn copy_desk_schema_owns_the_complete_finding_category_set() {
        let schema = serde_json::to_string(&schema_for!(WorldNewspaperCopyDeskReport)).unwrap();
        assert!(schema.contains("\"queries\""));
        assert!(!schema.contains("\"accepted\""));
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
    fn copy_desk_may_lodge_distinct_objections_to_one_exact_phrase() {
        let draft: EditorialPageDraft = serde_json::from_str(ACCEPTED_PAGE).unwrap();
        let phrase =
            "A gambling debt reaches the throne room and leaves one official carrying the blame.";
        let report = WorldNewspaperCopyDeskReport {
            assessment: "The deck has both a factual and a copy-level defect.".into(),
            queries: vec![
                WorldNewspaperGroundingFinding {
                    article_index: 0,
                    category: WorldNewspaperGroundingCategory::UnsupportedFact,
                    claim_or_phrase: phrase.into(),
                    reason: "The cited record does not locate the debt in the throne room.".into(),
                },
                WorldNewspaperGroundingFinding {
                    article_index: 0,
                    category: WorldNewspaperGroundingCategory::MechanicalCopy,
                    claim_or_phrase: phrase.into(),
                    reason: "The deck states blame abstractly instead of naming the court action."
                        .into(),
                },
            ],
        };

        validate_copy_desk_report(&draft, &report).unwrap();
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
                    section: "Front Page".into(),
                    journalist_id: "aven-tarl".into(),
                    citations: vec!["news:seal-scandal".into()],
                    focus_citation: "news:seal-scandal".into(),
                    narrative_claim: "The pawned seal opens the custody crisis.".into(),
                    tension: "Admission and dismissal point in different directions.".into(),
                    public_question: "Who answers for the seal?".into(),
                },
                WorldNewspaperStoryPitch {
                    lead: true,
                    section: "Dispatches".into(),
                    journalist_id: "lysa-fen".into(),
                    citations: vec!["news:seal-scandal".into()],
                    focus_citation: "news:seal-scandal".into(),
                    narrative_claim: "The gate repeats the pattern.".into(),
                    tension: "Access closes while repair begins.".into(),
                    public_question: "When will the route reopen?".into(),
                },
            ],
        };

        assert!(
            validate_editorial_agenda(&records, &canopy_ledger_newsroom(), &invalid, 4)
                .unwrap_err()
                .to_string()
                .contains("exactly its first pitch as lead")
        );

        let agenda: NarrativeSelectionAction = serde_json::from_str(TWO_STORY_AGENDA).unwrap();
        let NarrativeSelectionCommand::ProposeAgenda {
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
        validate_editorial_agenda(&records, &canopy_ledger_newsroom(), &admitted, 4).unwrap();
        let mut duplicate = admitted.clone();
        duplicate.story_pitches[1]
            .citations
            .push("news:seal-scandal".into());
        validate_editorial_agenda(&records, &canopy_ledger_newsroom(), &duplicate, 4).unwrap();
        let mut repeated_within_pitch = admitted.clone();
        repeated_within_pitch.story_pitches[0]
            .citations
            .push("news:seal-scandal".into());
        assert!(
            validate_editorial_agenda(
                &records,
                &canopy_ledger_newsroom(),
                &repeated_within_pitch,
                4,
            )
            .unwrap_err()
            .to_string()
            .contains("repeats a public record ID")
        );

        let mut widened: EditorialPageDraft = serde_json::from_str(ACCEPTED_PAGE).unwrap();
        widened.articles[0].citations = vec!["news:west-gate".into()];
        assert!(
            validate_editorial_alignment(
                &widened,
                &canopy_ledger_newsroom(),
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
        let newsroom = canopy_ledger_newsroom();
        assert_eq!(records.len(), 45);
        assert!(
            records
                .iter()
                .any(|record| record.facts[0].account.contains("speaking child"))
        );
        let mut tool = NarrativeSelectionWorkbench {
            records: &records,
            newsroom: &newsroom,
            max_articles: 3,
            visible_record_ids: BTreeSet::new(),
            completed_queries: BTreeSet::new(),
            pending_agenda: None,
        };
        let initial_schema = tool.action_schema().unwrap().to_string();
        assert!(initial_schema.contains("propose_agenda"));
        assert!(initial_schema.contains("commit_agenda"));
        assert!(initial_schema.contains("fetch_public_records"));

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
        assert_eq!(tool.action_schema().unwrap().to_string(), initial_schema);
        let bounded_context = serde_json::to_string(&tool.context()).unwrap();
        assert!(bounded_context.contains("news:archive:00"));
        assert!(!bounded_context.contains("\"facts\""));
        assert!(!bounded_context.contains("\"assertion_status\""));

        let exact_fetch = tool.fetch_records(vec!["news:archive:00".into()]).unwrap();
        assert!(matches!(
            exact_fetch,
            NarrativeSelectionFinding::QueryResult { records, next_cursor }
                if records.len() == 1
                    && records[0].facts[0].account.contains("speaking child")
                    && next_cursor.is_none()
        ));

        let ModelAgentToolOutcome::Rejected { finding, .. } = tool
            .invoke(
                NarrativeSelectionAction {
                    command: NarrativeSelectionCommand::CommitAgenda {
                        candidate_digest: format!("sha256:{}", "0".repeat(64)),
                    },
                },
                &ModelAgentToolContext {
                    source_receipt_ids: Vec::new(),
                    current_model_receipt: None,
                },
            )
            .await
        else {
            panic!("an unreviewed agenda must not be committable")
        };
        assert!(matches!(
            finding,
            NarrativeSelectionFinding::AgendaRejected { reason }
                if reason.contains("requires one reviewed proposal")
        ));

        let action = NarrativeSelectionAction {
            command: NarrativeSelectionCommand::ProposeAgenda {
                dominant_throughline: "The original crisis survived its administrative aftermath."
                    .into(),
                reader_stake: "Readers still bear the consequences of the founding rupture.".into(),
                story_pitches: vec![WorldNewspaperStoryPitch {
                    lead: true,
                    section: "Front Page".into(),
                    journalist_id: "mera-quill".into(),
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
        let ModelAgentToolOutcome::Continue { observation, .. } = tool
            .invoke(
                action,
                &ModelAgentToolContext {
                    source_receipt_ids: Vec::new(),
                    current_model_receipt: None,
                },
            )
            .await
        else {
            panic!("queried foundational record must enter editorial review")
        };
        let NarrativeSelectionFinding::AgendaProposed {
            candidate_digest,
            front_page,
            below_fold,
            ..
        } = observation
        else {
            panic!("proposal must return a front-page proof")
        };
        assert_eq!(front_page.focus_record.record_id, "news:archive:00");
        assert!(below_fold.is_empty());
        assert!(
            tool.action_schema()
                .unwrap()
                .to_string()
                .contains("commit_agenda")
        );

        tool.fetch_records(vec!["news:archive:00".into()]).unwrap();
        let ModelAgentToolOutcome::Rejected { finding, .. } = tool
            .invoke(
                NarrativeSelectionAction {
                    command: NarrativeSelectionCommand::CommitAgenda {
                        candidate_digest: candidate_digest.clone(),
                    },
                },
                &ModelAgentToolContext {
                    source_receipt_ids: Vec::new(),
                    current_model_receipt: None,
                },
            )
            .await
        else {
            panic!("new research must retire the previously reviewed agenda")
        };
        assert!(matches!(
            finding,
            NarrativeSelectionFinding::AgendaRejected { reason }
                if reason.contains("requires one reviewed proposal")
        ));

        let revised_action = NarrativeSelectionAction {
            command: NarrativeSelectionCommand::ProposeAgenda {
                dominant_throughline:
                    "The founding rupture still governs the administrative aftermath.".into(),
                reader_stake: "Readers still bear the consequences of the founding rupture.".into(),
                story_pitches: vec![WorldNewspaperStoryPitch {
                    lead: true,
                    section: "Front Page".into(),
                    journalist_id: "mera-quill".into(),
                    citations: vec!["news:archive:00".into()],
                    focus_citation: "news:archive:00".into(),
                    narrative_claim:
                        "The original crisis remains more consequential than its paperwork.".into(),
                    tension: "Institutions administer consequences without resolving the cause."
                        .into(),
                    public_question: "Who benefits when the founding rupture leaves the page?"
                        .into(),
                }],
            },
        };
        let ModelAgentToolOutcome::Continue { observation, .. } = tool
            .invoke(
                revised_action,
                &ModelAgentToolContext {
                    source_receipt_ids: Vec::new(),
                    current_model_receipt: None,
                },
            )
            .await
        else {
            panic!("a revised agenda must receive a new editorial review")
        };
        let NarrativeSelectionFinding::AgendaProposed {
            candidate_digest: revised_candidate_digest,
            ..
        } = observation
        else {
            panic!("revised proposal must return a front-page proof")
        };
        assert_ne!(candidate_digest, revised_candidate_digest);
        let ModelAgentToolOutcome::Rejected { finding, .. } = tool
            .invoke(
                NarrativeSelectionAction {
                    command: NarrativeSelectionCommand::CommitAgenda {
                        candidate_digest: candidate_digest.clone(),
                    },
                },
                &ModelAgentToolContext {
                    source_receipt_ids: Vec::new(),
                    current_model_receipt: None,
                },
            )
            .await
        else {
            panic!("a superseded agenda digest must not be committable")
        };
        assert!(matches!(
            finding,
            NarrativeSelectionFinding::AgendaRejected { reason }
                if reason.contains("stale candidate")
        ));
        let ModelAgentToolOutcome::Accepted { output: agenda, .. } = tool
            .invoke(
                NarrativeSelectionAction {
                    command: NarrativeSelectionCommand::CommitAgenda {
                        candidate_digest: revised_candidate_digest,
                    },
                },
                &ModelAgentToolContext {
                    source_receipt_ids: Vec::new(),
                    current_model_receipt: None,
                },
            )
            .await
        else {
            panic!("reviewed proposal must be committable")
        };
        let editor_desk = source_json_for_agenda(&records, Some(&agenda)).unwrap();
        assert!(editor_desk.contains("\"record_id\": \"news:archive:00\""));
        assert!(!editor_desk.contains("news:archive:44"));
    }

    #[test]
    fn narrative_workbench_pages_the_complete_ledger_from_an_inspected_cursor() {
        let records = public_news_records(&campaign_with_archive_news()).unwrap();
        let newsroom = canopy_ledger_newsroom();
        let mut tool = NarrativeSelectionWorkbench {
            records: &records,
            newsroom: &newsroom,
            max_articles: 3,
            visible_record_ids: BTreeSet::new(),
            completed_queries: BTreeSet::new(),
            pending_agenda: None,
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
    async fn narrative_agent_queries_and_reviews_before_committing_an_agenda() {
        let prepared = prepare_newspaper(
            &campaign_with_archive_news(),
            "The Canopy Ledger",
            "Independent and pointed.",
            &canopy_ledger_newsroom(),
            3,
        )
        .unwrap();
        let model = ScriptedNewspaperModel::new([
            QUERY_FOUNDING_CRISIS,
            r#"{"command":{"tool":"propose_agenda","dominant_throughline":"The founding crisis survived its administrative aftermath.","reader_stake":"Readers still bear the consequences while institutions manage the paperwork.","story_pitches":[{"lead":true,"section":"Front Page","journalist_id":"mera-quill","citations":["news:archive:00"],"focus_citation":"news:archive:00","narrative_claim":"The original public record is the fact later notices cannot domesticate.","tension":"Administrative responses multiply while the founding rupture remains unresolved.","public_question":"Who benefits when the cause leaves the front page?"}]}}"#,
        ]);

        let run = select_editorial_agenda(&model, &prepared, 3).await.unwrap();

        assert_eq!(run.receipts.len(), 3);
        assert_eq!(run.output.story_pitches.len(), 1);
        assert_eq!(run.output.story_pitches[0].citations, ["news:archive:00"]);
        let requests = model.requests();
        assert_eq!(requests.len(), 3);
        assert!(requests[1].lived_stream.contains("news:archive:00"));
    }

    #[tokio::test]
    async fn assignment_editor_keeps_a_stable_prefix_without_replaying_old_record_bodies() {
        let prepared = prepare_newspaper(
            &campaign_with_two_news(),
            "The Canopy Ledger",
            "Independent and pointed.",
            &canopy_ledger_newsroom(),
            3,
        )
        .unwrap();
        let model = ScriptedNewspaperModel::new([QUERY_ALL_RECORDS, ONE_STORY_AGENDA]);

        select_editorial_agenda(&model, &prepared, 3).await.unwrap();

        let requests = model.requests();
        assert_eq!(requests.len(), 3);
        let stable_prefix = requests[0]
            .lived_stream
            .split("\n\nUSER:\nAGENT STEP")
            .next()
            .unwrap();
        assert!(stable_prefix.contains("STAFF BOOK"));
        assert!(
            requests[1..]
                .iter()
                .all(|request| request.lived_stream.starts_with(stable_prefix))
        );
        assert!(requests[1].lived_stream.contains("cracked hinge"));
        assert!(requests[2].lived_stream.contains("news:west-gate"));
        assert!(!requests[2].lived_stream.contains("cracked hinge"));
    }

    #[test]
    fn narrative_workbench_has_no_source_count_hole_and_reads_legacy_agendas() {
        let records = public_news_records(&campaign_with_archive_news()).unwrap();
        let pitch = WorldNewspaperStoryPitch {
            lead: true,
            section: "Front Page".into(),
            journalist_id: "aven-tarl".into(),
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
        validate_editorial_agenda(&records, &canopy_ledger_newsroom(), &agenda, 2).unwrap();

        let mut unfocused = agenda.clone();
        unfocused.story_pitches[0].citations.truncate(4);
        unfocused.story_pitches[0].focus_citation = "news:archive:07".into();
        assert!(
            validate_editorial_agenda(&records, &canopy_ledger_newsroom(), &unfocused, 2)
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
    fn journalist_action_schema_compiles_exact_cited_datelines() {
        let records = public_news_records(&campaign_with_two_news()).unwrap();
        let lead_citations = vec!["news:seal-scandal".to_owned()];
        let lead = JournalistWorkbench {
            records: &records,
            article_index: 0,
            section: "Front Page",
            byline: "Aven Tarl",
            citations: &lead_citations,
        };
        assert_eq!(
            lead.action_schema().unwrap()["properties"]["dateline"]["enum"],
            serde_json::json!(["Room"])
        );

        let dispatch_citations = vec!["news:west-gate".to_owned()];
        let dispatch = JournalistWorkbench {
            records: &records,
            article_index: 1,
            section: "Dispatches",
            byline: "Lysa Fen",
            citations: &dispatch_citations,
        };
        assert_eq!(
            dispatch.action_schema().unwrap()["properties"]["dateline"]["enum"],
            serde_json::json!(["", "Yard"])
        );
    }

    #[test]
    fn night_editor_close_owns_complete_query_disposition_and_frozen_fields() {
        let draft: EditorialPageDraft = serde_json::from_str(TWO_ARTICLE_PAGE).unwrap();
        let report = WorldNewspaperCopyDeskReport {
            assessment: "The lead deck contains two unsupported implications.".into(),
            queries: vec![
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
        let records = public_news_records(&campaign_with_two_news()).unwrap();
        let action: NarrativeSelectionAction = serde_json::from_str(TWO_STORY_AGENDA).unwrap();
        let NarrativeSelectionCommand::ProposeAgenda {
            dominant_throughline,
            reader_stake,
            story_pitches,
        } = action.command
        else {
            panic!("fixture must propose an agenda")
        };
        let agenda = WorldNewspaperEditorialAgenda {
            dominant_throughline,
            reader_stake,
            story_pitches,
        };
        let night_sources = source_json_for_copy_queries(&records, &agenda, &report).unwrap();
        assert!(night_sources.contains("news:seal-scandal"));
        assert!(!night_sources.contains("news:west-gate"));
        let schema = NightEditorCloseWorkbench {
            records: &records,
            max_articles: 2,
            draft: draft.clone(),
            copy_desk: report.clone(),
        }
        .action_schema()
        .unwrap();
        assert_eq!(schema["properties"]["rewrites"]["minItems"], 0);
        assert_eq!(schema["properties"]["rewrites"]["maxItems"], 2);
        assert_eq!(
            schema["properties"]["rewrites"]["items"]["properties"]["article_index"]["enum"],
            serde_json::json!([0, 1])
        );
        let stable_schema = NightEditorCloseWorkbench {
            records: &records,
            max_articles: 2,
            draft: draft.clone(),
            copy_desk: WorldNewspaperCopyDeskReport {
                assessment: "No queries.".into(),
                queries: Vec::new(),
            },
        }
        .action_schema()
        .unwrap();
        assert_eq!(schema, stable_schema);
        let closed = apply_night_editor_close(
            &draft,
            &report,
            NightEditorCloseAction::SubmitClose {
                addressed_query_indices: vec![0, 1],
                rewrites: vec![NightEditorArticleClose {
                    article_index: 0,
                    headline: "Pawned Seal, Dismissed Messenger".into(),
                    deck: "The court admitted one loss and imposed another.".into(),
                    dateline: "Room".into(),
                    paragraphs: vec![
                        "The Thorn Court admitted that its royal seal was pawned to cover a dragon's gambling debt.".into(),
                        "The treasurer carried that admission into open court and was dismissed soon afterward. Readers may decide which act embarrassed the court more.".into(),
                    ],
                }],
            },
        )
        .unwrap();

        assert_eq!(closed.addressed_query_indices, [0, 1]);
        assert_eq!(closed.changed_article_indices, [0]);
        assert_eq!(closed.draft.articles[0].section, draft.articles[0].section);
        assert_eq!(closed.draft.articles[0].byline, draft.articles[0].byline);
        assert_eq!(
            closed.draft.articles[0].citations,
            draft.articles[0].citations
        );
        assert_eq!(closed.draft.articles[1].section, draft.articles[1].section);
        assert_eq!(closed.draft.articles[1].byline, draft.articles[1].byline);
        assert_eq!(
            closed.draft.articles[1].citations,
            draft.articles[1].citations
        );
        assert_eq!(
            closed.draft.articles[1].headline,
            draft.articles[1].headline
        );

        let missed = apply_night_editor_close(
            &draft,
            &report,
            NightEditorCloseAction::SubmitClose {
                addressed_query_indices: vec![0],
                rewrites: Vec::new(),
            },
        )
        .unwrap_err();
        assert!(missed.to_string().contains("every copy-desk query"));
    }

    #[tokio::test]
    async fn newspaper_is_editorial_copy_with_separate_provenance() {
        let campaign = campaign_with_news();
        let model = ScriptedNewspaperModel::new([
            QUERY_ALL_RECORDS,
            ONE_STORY_AGENDA,
            ACCEPTED_STORY_ACTION,
            ACCEPTING_COPY_DESK,
            UNCHANGED_NIGHT_CLOSE,
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
        assert_eq!(
            composition
                .issue
                .editorial_agenda
                .as_ref()
                .unwrap()
                .story_pitches[0]
                .journalist_id,
            "aven-tarl"
        );
        assert_eq!(composition.issue.articles[0].byline, "Aven Tarl");
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
        let mut boundary_issue = composition.issue.clone();
        boundary_issue.articles[0].sources[0].facts[0].account =
            "x".repeat(MAX_PUBLIC_EVENT_SUMMARY_CHARS);
        assert!(
            render_world_newspaper_audit_markdown(&boundary_issue)
                .contains("audit shows the complete stored account and asserts nothing beyond it")
        );
        assert_eq!(composition.model_receipts.len(), 6);
        assert!(composition.copy_desk.queries.is_empty());
        assert!(composition.press_close.night_editor_action_applied);
        assert!(composition.press_close.changed_article_indices.is_empty());
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
        assert_eq!(
            requests[2].stage,
            "newspaper_narrative_selection_agent_action"
        );
        assert_eq!(requests[3].stage, "newspaper_journalist_agent_action");
        assert_eq!(requests[4].stage, "newspaper_copy_desk");
        assert_eq!(
            requests[5].stage,
            "newspaper_night_editor_close_agent_action"
        );
        assert!(requests[3].lived_stream.contains("ASSIGNMENT PACKET"));
        assert!(requests[0].lived_stream.contains("Veyra Kest"));
        assert!(requests[0].lived_stream.contains("Mera Quill"));
        assert!(requests[3].lived_stream.contains("Aven Tarl"));
        assert!(!requests[3].lived_stream.contains("Mera Quill"));
        assert!(requests[4].lived_stream.contains("Dalen Marr"));
        assert!(requests[5].lived_stream.contains("Meret Sorn"));
        assert!(!requests[5].lived_stream.contains("recover drama"));
        let stable_selector_prefix = requests[0]
            .lived_stream
            .split("\n\nUSER:\nAGENT STEP")
            .next()
            .unwrap();
        assert!(requests[1].lived_stream.starts_with(stable_selector_prefix));
        assert!(requests[2].lived_stream.starts_with(stable_selector_prefix));
        assert!(
            !requests[2]
                .lived_stream
                .starts_with(&requests[1].lived_stream)
        );
        assert_eq!(requests[0].output_schema, requests[1].output_schema);
        assert_eq!(requests[1].output_schema, requests[2].output_schema);
        assert!(
            requests[3]
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
    async fn copy_desk_queries_feed_one_night_close_without_rereview() {
        const REJECTED: &str = r#"{"assessment":"The deck overstates where the admitted debt reached.","queries":[{"article_index":0,"category":"unsupported_fact","claim_or_phrase":"A gambling debt reaches the throne room and leaves one official carrying the blame.","reason":"The cited source records the pawned seal and dismissal but does not locate the debt in the throne room."}]}"#;
        let campaign = campaign_with_news();
        let model = ScriptedNewspaperModel::new([
            QUERY_ALL_RECORDS,
            ONE_STORY_AGENDA,
            ACCEPTED_STORY_ACTION,
            REJECTED,
            QUERY_DECK_CLOSE,
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

        assert_eq!(composition.copy_desk.queries.len(), 1);
        assert!(composition.press_close.night_editor_action_applied);
        assert_eq!(composition.press_close.addressed_query_indices, [0]);
        assert_eq!(composition.press_close.changed_article_indices, [0]);
        assert_eq!(
            composition.issue.articles[0].deck,
            "The court admits the pawned seal and dismisses the official who carried the confession."
        );
        let requests = model.requests();
        assert_eq!(requests.len(), 6);
        assert_eq!(
            requests
                .iter()
                .filter(|request| request.stage == "newspaper_copy_desk")
                .count(),
            1
        );
        assert_eq!(
            requests
                .iter()
                .filter(|request| { request.stage == "newspaper_night_editor_close_agent_action" })
                .count(),
            1
        );
        assert!(requests.iter().all(|request| {
            request.stage != "newspaper_night_editor"
                && request.stage != "newspaper_rewrite_desk_agent_action"
        }));
        assert!(composition.model_receipts[..5].iter().all(|receipt| {
            requests[5]
                .source_receipt_ids
                .contains(&receipt.storage_key().to_owned())
        }));
    }

    #[tokio::test]
    async fn rejected_night_close_prints_the_checkpointed_page_at_deadline() {
        const REJECTED: &str = r#"{"assessment":"The deck overstates where the admitted debt reached.","queries":[{"article_index":0,"category":"unsupported_fact","claim_or_phrase":"A gambling debt reaches the throne room and leaves one official carrying the blame.","reason":"The cited source records the pawned seal and dismissal but does not locate the debt in the throne room."}]}"#;
        const INVALID_CLOSE: &str = r#"{"tool":"submit_close","addressed_query_indices":[0],"rewrites":[{"article_index":0,"headline":"Court Sells the Crown's Seal, Then the Treasurer","deck":"A gambling debt reaches the throne room and leaves one official carrying the blame.","dateline":"Elsewhere","paragraphs":["The Thorn Court has admitted that its royal seal was pawned to cover a dragon's gambling debt, a confession that turns private embarrassment into a public question of custody.","The treasurer who delivered that admission in open court was dismissed soon afterward. The court has explained the firing; it has not made the seal any less pawned."]}]}"#;
        let model = ScriptedNewspaperModel::new([
            QUERY_ALL_RECORDS,
            ONE_STORY_AGENDA,
            ACCEPTED_STORY_ACTION,
            REJECTED,
            INVALID_CLOSE,
        ]);
        let composition = compose_world_newspaper(
            &model,
            &campaign_with_news(),
            "The Underdeep Clarion",
            "A skeptical court broadsheet with dry restraint.",
            4,
        )
        .await
        .unwrap();

        assert!(!composition.press_close.night_editor_action_applied);
        assert!(composition.press_close.addressed_query_indices.is_empty());
        assert_eq!(
            composition.issue.articles[0].deck,
            "A gambling debt reaches the throne room and leaves one official carrying the blame."
        );
        assert_eq!(
            composition
                .model_receipts
                .iter()
                .filter(|receipt| receipt.stage == "newspaper_copy_desk")
                .count(),
            1
        );
        assert_eq!(
            composition
                .model_receipts
                .iter()
                .filter(|receipt| { receipt.stage == "newspaper_night_editor_close_agent_action" })
                .count(),
            1
        );
        assert_eq!(
            composition.model_receipts.last().unwrap().validation_result,
            "semantic_invalid"
        );
    }

    #[tokio::test]
    async fn close_checkpoint_resumes_without_replaying_reporter_or_copy_desk() {
        const REJECTED: &str = r#"{"assessment":"The deck overstates where the admitted debt reached.","queries":[{"article_index":0,"category":"unsupported_fact","claim_or_phrase":"A gambling debt reaches the throne room and leaves one official carrying the blame.","reason":"The cited source records the pawned seal and dismissal but does not locate the debt in the throne room."}]}"#;
        let campaign = campaign_with_news();
        let directory = tempfile::tempdir().unwrap();
        let store = CampaignStore::open(directory.path().join("campaign.cc")).unwrap();
        let first_model = ScriptedNewspaperModel::new([
            QUERY_ALL_RECORDS,
            ONE_STORY_AGENDA,
            ACCEPTED_STORY_ACTION,
            REJECTED,
        ]);
        let error = advance_world_newspaper(
            &first_model,
            &campaign,
            "The Underdeep Clarion",
            "A skeptical court broadsheet with dry restraint.",
            &canopy_ledger_newsroom(),
            4,
            &store,
        )
        .await
        .unwrap_err();
        assert!(
            error
                .to_string()
                .contains("fixture newspaper model exhausted")
        );
        assert_eq!(
            store
                .keys("world_newspaper_close_checkpoint.v1")
                .unwrap()
                .len(),
            1
        );

        let resume_model = ScriptedNewspaperModel::new([QUERY_DECK_CLOSE]);
        let accepted = advance_world_newspaper(
            &resume_model,
            &campaign,
            "The Underdeep Clarion",
            "A skeptical court broadsheet with dry restraint.",
            &canopy_ledger_newsroom(),
            4,
            &store,
        )
        .await
        .unwrap();
        let requests = resume_model.requests();
        assert_eq!(requests.len(), 1);
        assert_eq!(
            requests[0].stage,
            "newspaper_night_editor_close_agent_action"
        );
        assert_eq!(accepted.model_receipts.len(), 6);

        let idempotent_model = ScriptedNewspaperModel::new([]);
        let repeated = advance_world_newspaper(
            &idempotent_model,
            &campaign,
            "The Underdeep Clarion",
            "A skeptical court broadsheet with dry restraint.",
            &canopy_ledger_newsroom(),
            4,
            &store,
        )
        .await
        .unwrap();
        assert_eq!(repeated, accepted);
        assert!(idempotent_model.requests().is_empty());

        let prepared = prepare_newspaper(
            &campaign,
            "The Underdeep Clarion",
            "A skeptical court broadsheet with dry restraint.",
            &canopy_ledger_newsroom(),
            4,
        )
        .unwrap();
        let (_, mut persisted) = store
            .load::<PersistedWorldNewspaperComposition>(
                "world_newspaper_composition.v3",
                &prepared.publication_task_binding,
            )
            .unwrap()
            .unwrap();
        persisted.composition.issue.articles[0].deck = "Tampered after press close.".into();
        assert!(
            validate_persisted_composition(&store, &campaign, &prepared, 4, &persisted)
                .unwrap_err()
                .to_string()
                .contains("press witness")
        );
    }

    #[tokio::test]
    async fn production_checkpoints_resume_at_the_failed_reporter() {
        let campaign = campaign_with_two_news();
        let directory = tempfile::tempdir().unwrap();
        let store = CampaignStore::open(directory.path().join("campaign.cc")).unwrap();
        let first_model = ScriptedNewspaperModel::new([
            QUERY_ALL_RECORDS,
            TWO_STORY_AGENDA,
            ACCEPTED_STORY_ACTION,
            MALFORMED_LYSA_STORY_ACTION,
            MALFORMED_LYSA_STORY_ACTION,
        ]);
        let error = advance_world_newspaper(
            &first_model,
            &campaign,
            "The Underdeep Clarion",
            "A skeptical court broadsheet with dry restraint.",
            &canopy_ledger_newsroom(),
            4,
            &store,
        )
        .await
        .unwrap_err();
        assert!(error.to_string().contains("malformed paragraph"));
        assert_eq!(
            store
                .keys("world_newspaper_production_checkpoint.v1")
                .unwrap()
                .len(),
            2,
            "the admitted agenda and Aven's accepted filing must survive Lysa's failure"
        );
        let mut filed_checkpoint = store
            .load_all::<WorldNewspaperProductionCheckpoint>(
                "world_newspaper_production_checkpoint.v1",
            )
            .unwrap()
            .into_iter()
            .find(|checkpoint| {
                matches!(
                    &checkpoint.content,
                    WorldNewspaperProductionContent::FiledArticle { .. }
                )
            })
            .unwrap();
        let filed_receipts = load_model_receipts(
            &store,
            &filed_checkpoint.model_receipt_ids,
            "tamper fixture",
        )
        .unwrap();
        if let WorldNewspaperProductionContent::FiledArticle { article, .. } =
            &mut filed_checkpoint.content
        {
            article.deck = "Tampered after filing.".into();
        }
        assert!(
            validate_production_receipts(
                &filed_checkpoint,
                &filed_receipts,
                "newspaper_journalist_agent_action",
                &filed_receipts[0].snapshot_binding,
            )
            .unwrap_err()
            .to_string()
            .contains("receipt binding")
        );

        let resume_model = ScriptedNewspaperModel::new([
            ACCEPTED_LYSA_STORY_ACTION,
            ACCEPTING_COPY_DESK,
            UNCHANGED_NIGHT_CLOSE,
        ]);
        let composition = advance_world_newspaper(
            &resume_model,
            &campaign,
            "The Underdeep Clarion",
            "A skeptical court broadsheet with dry restraint.",
            &canopy_ledger_newsroom(),
            4,
            &store,
        )
        .await
        .unwrap();
        let requests = resume_model.requests();
        assert_eq!(requests.len(), 3);
        assert_eq!(requests[0].stage, "newspaper_journalist_agent_action");
        assert!(requests[0].lived_stream.contains("Lysa Fen"));
        assert!(!requests[0].lived_stream.contains("Aven Tarl"));
        assert_eq!(requests[1].stage, "newspaper_copy_desk");
        assert_eq!(
            requests[2].stage,
            "newspaper_night_editor_close_agent_action"
        );
        assert_eq!(composition.issue.articles.len(), 2);
        assert_eq!(
            store
                .keys("world_newspaper_production_checkpoint.v1")
                .unwrap()
                .len(),
            3
        );
    }

    #[tokio::test]
    async fn legacy_close_checkpoint_decodes_as_inert_history() {
        let campaign = campaign_with_news();
        let source_directory = tempfile::tempdir().unwrap();
        let source_store =
            CampaignStore::open(source_directory.path().join("campaign.cc")).unwrap();
        let interrupted = ScriptedNewspaperModel::new([
            QUERY_ALL_RECORDS,
            ONE_STORY_AGENDA,
            ACCEPTED_STORY_ACTION,
            ACCEPTING_COPY_DESK,
        ]);
        advance_world_newspaper(
            &interrupted,
            &campaign,
            "The Underdeep Clarion",
            "A skeptical court broadsheet with dry restraint.",
            &canopy_ledger_newsroom(),
            4,
            &source_store,
        )
        .await
        .unwrap_err();
        let mut legacy = source_store
            .load_all::<WorldNewspaperCloseCheckpoint>("world_newspaper_close_checkpoint.v1")
            .unwrap()
            .into_iter()
            .next()
            .unwrap();
        legacy.id = "newspaper-close:legacy-v7".into();
        legacy.origin = WorldNewspaperCloseOrigin::LegacyV7Checkpoint;
        legacy.source_checkpoint_id = Some("historical-v7-tip".into());

        let target_directory = tempfile::tempdir().unwrap();
        let target_store =
            CampaignStore::open(target_directory.path().join("campaign.cc")).unwrap();
        target_store
            .insert(
                "world_newspaper_close_checkpoint.v1",
                "ghostlight.world_newspaper_close_checkpoint.v1",
                &legacy.id,
                &legacy,
            )
            .unwrap();

        let model = ScriptedNewspaperModel::new([
            QUERY_ALL_RECORDS,
            ONE_STORY_AGENDA,
            ACCEPTED_STORY_ACTION,
            ACCEPTING_COPY_DESK,
            UNCHANGED_NIGHT_CLOSE,
        ]);
        let composition = advance_world_newspaper(
            &model,
            &campaign,
            "The Underdeep Clarion",
            "A skeptical court broadsheet with dry restraint.",
            &canopy_ledger_newsroom(),
            4,
            &target_store,
        )
        .await
        .unwrap();

        assert_eq!(model.requests().len(), 6);
        assert_eq!(composition.issue.articles.len(), 1);
        assert_eq!(
            target_store
                .keys("world_newspaper_close_checkpoint.v1")
                .unwrap()
                .len(),
            2
        );
    }

    #[tokio::test]
    async fn reader_projection_escapes_model_and_consumer_markdown() {
        const MARKDOWN_PAGE: &str = r#"{"tool":"file_story","headline":"[Court](https://headline.invalid) Faces <Reckoning>","deck":"The *royal* debt now reaches every keeper of the seal.","dateline":"Room","paragraphs":["- The Thorn Court admitted the royal seal was pawned to cover a dragon's gambling debt; <img src=x> cannot make the confession ~~prettier~~.","1. The dismissed treasurer leaves readers with a [public record](https://copy.invalid) and the court with a seal that remains pawned."]}"#;
        let campaign = campaign_with_news();
        let model = ScriptedNewspaperModel::new([
            QUERY_ALL_RECORDS,
            ONE_STORY_AGENDA,
            MARKDOWN_PAGE,
            ACCEPTING_COPY_DESK,
            UNCHANGED_NIGHT_CLOSE,
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
        const ALTERNATE_PAGE: &str = r#"{"tool":"file_story","headline":"Court Sells the Crown's Seal, Then the Treasurer","deck":"The same scandal leaves the throne defending both its custody and its judgment.","dateline":"Room","paragraphs":["The Thorn Court has admitted that its royal seal was pawned to cover a dragon's gambling debt, a confession that turns private embarrassment into a public question of custody.","The treasurer who delivered that admission in open court was dismissed soon afterward. The court has explained the firing; it has not made the seal any less pawned."]}"#;
        let campaign = campaign_with_news();
        let first = compose_world_newspaper(
            &ScriptedNewspaperModel::new([
                QUERY_ALL_RECORDS,
                ONE_STORY_AGENDA,
                ACCEPTED_STORY_ACTION,
                ACCEPTING_COPY_DESK,
                UNCHANGED_NIGHT_CLOSE,
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
                UNCHANGED_NIGHT_CLOSE,
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
