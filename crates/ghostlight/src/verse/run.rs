//! The manual verse run: the tick driver and its record.
//!
//! One invocation advances the world tick by tick. A tick reads the world,
//! derives the budgeted cover, runs each cell through the controller organ,
//! advances the clock once by the tick span, and checkpoints. The driver owns
//! the order of those steps and nothing else: cells decide what factions do,
//! the recording port owns spend, the kernel owns the world.
//!
//! The world's own clock is the resume point. A tick whose cells did not all
//! finish never advances the clock, so a later run derives the same tick index,
//! the same cells and the same command ids, the controller-work rows pick up
//! where they stopped, and the kernel answers every re-submission with the
//! original receipt. A run record therefore reports a run and is never read to
//! resume one.
//!
//! A run starts only when it is invoked: nothing here waits, schedules or
//! repeats.

use super::calls::{RecordingInferencePort, canonical};
use crate::controllers::{
    CellRun, ControllerError, ControllerPendingReason, ControllerRunner, InferenceFaultClass,
    NarrativeRun, OperationalRun, SubmissionDisposition,
};
use crate::cover::{Cell, Constituent, CoverBudget, CoverBudgetError, derive_cover};
use crate::{
    CommandId, DecisionOpportunity, MailboxError, OperatorEvent, RenderPath, SubmitReceipt,
    TickMinutes, WorldId, WorldMailbox,
};
use chrono::Utc;
use cultcache_rs::{CacheBackingStore, CultCacheEnvelope, OwnedRedbMessagePackBackingStore};
use serde::{Deserialize, Serialize};
use std::{ops::ControlFlow, path::Path, sync::Mutex};
use thiserror::Error;

/// The run record document's schema.
pub const RUN_RECORD_SCHEMA: &str = "ghostlight.verse_run.v1";
const RUN_ROW: &str = "verse_run.v1";
const CLOCK_NAMESPACE: &str = "ghostlight.verse.clock";

/// Why a run could not be recorded or started. Every variant names a kind of
/// fault; none carries a value the run held.
#[derive(Debug, Error)]
pub enum VerseRunError {
    #[error("the run record store could not be read or written")]
    Store,
    #[error("the run record store holds a row that is not a canonical run record")]
    Corrupt,
    #[error("a run record's sequence is not an existing run or the next in its world")]
    OutOfSequence,
    #[error("the recording port belongs to another run")]
    PortNotThisRun,
    #[error("the cover budget is not valid: {0}")]
    Budget(#[from] CoverBudgetError),
    #[error(transparent)]
    Mailbox(#[from] MailboxError),
}

/// Where a run stands. `Running` is only ever the record of a run in flight,
/// or of one whose process died before it could say how it ended.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RunStatus {
    Running,
    Completed,
    /// A provider call could pass the token cap, so the run ended before the
    /// open tick's clock advance.
    BudgetExhausted,
    /// The run ended without finishing its open tick. `reason` is a fixed code
    /// naming the kind of fault, never a value.
    Failed {
        reason: String,
    },
    /// The caller asked the run to end after a finished tick.
    Stopped,
}

/// Why a cell committed nothing and the run went on.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RefusalReason {
    /// A narrative turn the world moved under and could not lower again.
    Interrupted,
    /// The world refused the cell's submission.
    WorldRejected,
    /// The cell's subject no longer holds the opportunity it was derived for.
    OpportunityGone,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Refusal {
    /// The cell's position in the tick's cover.
    pub cell: u32,
    pub reason: RefusalReason,
}

/// One finished tick. The clock has advanced by the run's span.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TickCheckpoint {
    pub tick: u64,
    pub cells: u32,
    /// Every command the tick's cells hold a receipt for, in cell order. A
    /// decline is a commit, so it is listed.
    pub committed: Vec<CommandId>,
    pub refusals: Vec<Refusal>,
    /// Tokens the recording port charged during the tick.
    pub tokens: u64,
}

/// The cover budget as the record carries it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecordedCoverBudget {
    pub cells: u16,
    pub constituent_cap: u16,
    pub urgency_slots: u16,
}

/// One invocation of the verse runner: what it was asked, what it did.
/// Document `ghostlight.verse_run.v1`, keyed (world, run sequence).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerseRunRecord {
    pub world_id: WorldId,
    pub sequence: u64,
    pub token_cap: u64,
    pub ticks_requested: u32,
    pub tick_span_minutes: u32,
    pub cover_budget: RecordedCoverBudget,
    /// The world's revision when the run began.
    pub start_revision: u64,
    pub checkpoints: Vec<TickCheckpoint>,
    pub status: RunStatus,
}

/// What a caller asks of one run. The token cap is not here: the recording
/// port owns it.
#[derive(Clone, Copy, Debug)]
pub struct VerseRunConfig {
    pub ticks: u32,
    pub tick_span: TickMinutes,
    pub cover: CoverBudget,
}

/// A tick as the operator reads it. `closed` is false for the tick a run ended
/// inside, whose cells may have committed before the run stopped.
#[derive(Clone, Debug)]
pub struct TickReport {
    pub checkpoint: TickCheckpoint,
    /// Every act the tick committed, from the operator log.
    pub acts: Vec<OperatorEvent>,
    pub spent: u64,
    pub remaining: u64,
    pub closed: bool,
}

impl std::fmt::Display for RunStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Running => f.write_str("Running"),
            Self::Completed => f.write_str("Completed"),
            Self::BudgetExhausted => f.write_str("BudgetExhausted"),
            Self::Failed { reason } => write!(f, "Failed ({reason})"),
            Self::Stopped => f.write_str("Stopped"),
        }
    }
}

/// The tick as the operator reads it: a header, one line per committed act
/// (who, which verb, how it renders, what its roles are bound to, what was
/// said or shown), one per refusal, and the spend.
impl std::fmt::Display for TickReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let checkpoint = &self.checkpoint;
        writeln!(
            f,
            "tick {} {}: {} cells, {} acts, {} refused",
            checkpoint.tick,
            if self.closed { "closed" } else { "open" },
            checkpoint.cells,
            self.acts.len(),
            checkpoint.refusals.len()
        )?;
        for act in &self.acts {
            let path = match act.render_path {
                Some(RenderPath::ShipAction) => "ship_action",
                Some(RenderPath::Conversation) => "conversation",
                None => "-",
            };
            write!(f, "  {} {} [{path}]", act.speaker_label, act.verb)?;
            for role in &act.roles {
                write!(f, " {}={}", role.role, role.target)?;
            }
            if let Some(speech) = &act.speech {
                write!(f, " says {:?}", speech.as_str())?;
            }
            if let Some(display) = &act.display {
                write!(f, " shows {:?}", display.as_str())?;
            }
            writeln!(f)?;
        }
        for refusal in &checkpoint.refusals {
            let reason = match refusal.reason {
                RefusalReason::Interrupted => "interrupted",
                RefusalReason::WorldRejected => "world_rejected",
                RefusalReason::OpportunityGone => "opportunity_gone",
            };
            writeln!(f, "  refused: cell {} {reason}", refusal.cell)?;
        }
        writeln!(
            f,
            "  tokens: {} this tick, {} spent, {} remaining",
            checkpoint.tokens, self.spent, self.remaining
        )
    }
}

#[derive(Clone, Debug)]
pub struct RunOutcome {
    pub record: VerseRunRecord,
}

fn row_key(world: WorldId, sequence: u64) -> String {
    format!("{}/{sequence:020}", world.key())
}

/// Run records in one CultCache file. A run is saved when it begins and again
/// each time it moves, so a row is replaced by its own later state.
pub struct VerseRunStore {
    store: Mutex<OwnedRedbMessagePackBackingStore>,
}

impl VerseRunStore {
    /// Opens the file, creating it empty when absent. Every row must be a
    /// canonical run record of this schema under its own key.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, VerseRunError> {
        let store = OwnedRedbMessagePackBackingStore::new(path.as_ref())
            .map_err(|_| VerseRunError::Store)?;
        store
            .validate_path_identity()
            .map_err(|_| VerseRunError::Store)?;
        let opened = Self {
            store: Mutex::new(store),
        };
        opened.records()?;
        Ok(opened)
    }

    fn records(&self) -> Result<Vec<VerseRunRecord>, VerseRunError> {
        let rows = self
            .store
            .lock()
            .map_err(|_| VerseRunError::Store)?
            .pull_all()
            .map_err(|_| VerseRunError::Store)?;
        let mut records = Vec::new();
        for row in &rows {
            if row.r#type != RUN_ROW || row.schema_id.as_deref() != Some(RUN_RECORD_SCHEMA) {
                return Err(VerseRunError::Corrupt);
            }
            let record: VerseRunRecord =
                canonical(row).map_err(|_| VerseRunError::Corrupt)?;
            if row.key != row_key(record.world_id, record.sequence) || record.sequence == 0 {
                return Err(VerseRunError::Corrupt);
            }
            records.push(record);
        }
        records.sort_by_key(|record| (record.world_id.key(), record.sequence));
        Ok(records)
    }

    /// The sequence the next run of this world must carry. Sequences count from
    /// one.
    pub fn next_sequence(&self, world: WorldId) -> Result<u64, VerseRunError> {
        Ok(self
            .records()?
            .iter()
            .filter(|record| record.world_id == world)
            .map(|record| record.sequence)
            .max()
            .unwrap_or(0)
            + 1)
    }

    /// Every run of the world, in sequence.
    pub fn runs(&self, world: WorldId) -> Result<Vec<VerseRunRecord>, VerseRunError> {
        Ok(self
            .records()?
            .into_iter()
            .filter(|record| record.world_id == world)
            .collect())
    }

    /// Writes the record: the next run of its world, or a run already saved.
    pub fn save(&self, record: &VerseRunRecord) -> Result<(), VerseRunError> {
        if record.sequence == 0 || record.sequence > self.next_sequence(record.world_id)? {
            return Err(VerseRunError::OutOfSequence);
        }
        let row = CultCacheEnvelope {
            key: row_key(record.world_id, record.sequence),
            r#type: RUN_ROW.into(),
            payload: rmp_serde::to_vec_named(record).map_err(|_| VerseRunError::Store)?,
            stored_at: Utc::now().to_rfc3339(),
            schema_id: Some(RUN_RECORD_SCHEMA.into()),
        };
        self.store
            .lock()
            .map_err(|_| VerseRunError::Store)?
            .push(&row)
            .map_err(|_| VerseRunError::Store)
    }
}

/// How an error ends a cell.
enum CellEnd {
    Budget,
    Fatal(&'static str),
    Refused(RefusalReason),
}

fn classify(error: &ControllerError) -> CellEnd {
    match error {
        ControllerError::Inference { source, .. }
            if source.class() == InferenceFaultClass::BudgetExhausted =>
        {
            CellEnd::Budget
        }
        _ if error.requires_quarantine() => CellEnd::Fatal("controller_quarantined"),
        ControllerError::World(_) => CellEnd::Refused(RefusalReason::WorldRejected),
        ControllerError::NoOpportunity { .. }
        | ControllerError::AmbiguousOpportunity
        | ControllerError::OpportunityMismatch => CellEnd::Refused(RefusalReason::OpportunityGone),
        _ => CellEnd::Fatal("controller_error"),
    }
}

fn pending_code(reason: ControllerPendingReason) -> &'static str {
    match reason {
        ControllerPendingReason::InferenceRetryable => "inference_retryable",
        ControllerPendingReason::InferenceRecoveryRequired => "inference_recovery_required",
        ControllerPendingReason::WorldUnavailable => "world_unavailable",
        ControllerPendingReason::WorldOutcomeUnknown => "world_outcome_unknown",
        ControllerPendingReason::StoreReopenRequired => "store_reopen_required",
    }
}

fn command_of(submission: &SubmissionDisposition) -> CommandId {
    match submission {
        SubmissionDisposition::NoProposal(receipt) | SubmissionDisposition::Completed(receipt) => {
            match receipt {
                SubmitReceipt::Applied(commit) | SubmitReceipt::AlreadyApplied(commit) => {
                    commit.command_id
                }
            }
        }
        SubmissionDisposition::PreviouslyConfirmed(commit) => commit.command_id,
    }
}

/// Folds one finished cell into the tick. Returns the code of the pending state
/// that ends the run, when the cell did not finish.
fn absorb(
    run: CellRun,
    cell: u32,
    committed: &mut Vec<CommandId>,
    refusals: &mut Vec<Refusal>,
) -> Option<&'static str> {
    match run {
        CellRun::Narrative(NarrativeRun::Completed(decision)) => {
            committed.push(command_of(decision.submission()));
            None
        }
        CellRun::Narrative(NarrativeRun::Pending(pending)) => Some(pending_code(pending.reason())),
        CellRun::Narrative(NarrativeRun::Interrupted(_)) => {
            refusals.push(Refusal {
                cell,
                reason: RefusalReason::Interrupted,
            });
            None
        }
        CellRun::Operational(OperationalRun::Completed(decision)) => {
            committed.push(command_of(decision.submission()));
            None
        }
        CellRun::Operational(OperationalRun::Pending(pending)) => {
            Some(pending_code(pending.reason()))
        }
        CellRun::Grouped(run) => {
            committed.extend(run.submissions.iter().map(|entry| command_of(&entry.submission)));
            run.pending.map(pending_code)
        }
    }
}

/// The cell with each member's opportunity read fresh. The cover is a
/// partition of subjects, fixed for the tick; an opportunity is the world's
/// current binding for a subject, and an earlier cell's act can move it (a
/// word spoken in a shared room changes what its listeners know). A subject
/// that no longer holds an opportunity leaves the cell, and an empty cell is
/// gone.
fn refreshed(cell: &Cell, fresh: &[DecisionOpportunity]) -> Option<Cell> {
    let current = |member: &Constituent| {
        fresh
            .iter()
            .find(|opportunity| opportunity.scope == member.opportunity.scope)
            .map(|opportunity| Constituent {
                subject: member.subject,
                opportunity: opportunity.clone(),
            })
    };
    match cell {
        Cell::Singleton { id, tick, member } => current(member).map(|member| Cell::Singleton {
            id: *id,
            tick: *tick,
            member,
        }),
        Cell::Group { id, tick, members } => {
            let members: Vec<Constituent> = members.iter().filter_map(current).collect();
            (!members.is_empty()).then(|| Cell::Group {
                id: *id,
                tick: *tick,
                members,
            })
        }
    }
}

enum TickEnd {
    /// Every cell finished and the clock advanced.
    Closed,
    /// The run ends inside this tick with this status.
    Open(RunStatus),
}

fn failed(reason: &str) -> RunStatus {
    RunStatus::Failed {
        reason: reason.to_owned(),
    }
}

/// Runs `config.ticks` ticks from the world's present clock under the cap the
/// recording port was opened with. `observe` sees each tick as it ends,
/// including the one a run ends inside, and may stop the run after a finished
/// tick by breaking. `sequence` is the run's number from
/// [`VerseRunStore::next_sequence`]; the port must have been opened for it
/// ([`call_run_id`]).
pub async fn run_manual(
    mailbox: &WorldMailbox,
    runner: &ControllerRunner,
    port: &RecordingInferencePort,
    store: &VerseRunStore,
    config: &VerseRunConfig,
    sequence: u64,
    mut observe: impl FnMut(&TickReport) -> ControlFlow<()>,
) -> Result<RunOutcome, VerseRunError> {
    if port.run_id() != &call_run_id(sequence) {
        return Err(VerseRunError::PortNotThisRun);
    }
    let cover_budget = config.cover.validated()?;
    let begun = mailbox.snapshot().await?;
    let mut record = VerseRunRecord {
        world_id: begun.world_id,
        sequence,
        token_cap: port.cap(),
        ticks_requested: config.ticks,
        tick_span_minutes: config.tick_span.minutes(),
        cover_budget: RecordedCoverBudget {
            cells: cover_budget.cells,
            constituent_cap: cover_budget.constituent_cap,
            urgency_slots: cover_budget.urgency_slots,
        },
        start_revision: begun.revision,
        checkpoints: Vec::new(),
        status: RunStatus::Running,
    };
    store.save(&record)?;

    let mut spent_before = port.spent();
    for completed in 0..config.ticks {
        let (end, report) = run_tick(
            mailbox,
            runner,
            port,
            config,
            cover_budget,
            spent_before,
        )
        .await;
        spent_before = port.spent();
        match end {
            TickEnd::Open(status) => {
                record.status = status;
                store.save(&record)?;
                if let Some(report) = report {
                    let _ = observe(&report);
                }
                return Ok(RunOutcome { record });
            }
            TickEnd::Closed => {
                let report = report.expect("a closed tick has its report");
                record.checkpoints.push(report.checkpoint.clone());
                store.save(&record)?;
                if observe(&report).is_break() && completed + 1 < config.ticks {
                    record.status = RunStatus::Stopped;
                    store.save(&record)?;
                    return Ok(RunOutcome { record });
                }
            }
        }
    }
    record.status = RunStatus::Completed;
    store.save(&record)?;
    Ok(RunOutcome { record })
}

/// The key a run's provider calls are recorded under: the run's sequence, so a
/// run record and its call records name each other.
pub fn call_run_id(sequence: u64) -> super::RunId {
    super::RunId::new(&format!("run-{sequence}"))
        .expect("a decimal sequence is a canonical run id")
}

/// One tick. The report is absent only when the world could not be read at all.
async fn run_tick(
    mailbox: &WorldMailbox,
    runner: &ControllerRunner,
    port: &RecordingInferencePort,
    config: &VerseRunConfig,
    cover_budget: CoverBudget,
    spent_before: u64,
) -> (TickEnd, Option<TickReport>) {
    let read = async {
        let snapshot = mailbox.snapshot().await?;
        let opportunities = mailbox.opportunities().await?;
        let graph = mailbox.agency_graph().await?;
        Ok::<_, MailboxError>((snapshot, opportunities, graph))
    }
    .await;
    let Ok((snapshot, opportunities, graph)) = read else {
        return (TickEnd::Open(failed("world_unavailable")), None);
    };
    let span = config.tick_span;
    let cover = derive_cover(
        snapshot.world_id,
        snapshot.now,
        span.minutes(),
        &opportunities,
        &graph,
        cover_budget,
    );

    let mut committed = Vec::new();
    let mut refusals = Vec::new();
    let mut open: Option<RunStatus> = None;
    for (index, cell) in cover.cells.iter().enumerate() {
        let index = index as u32;
        let fresh = match mailbox.opportunities().await {
            Ok(fresh) => fresh,
            Err(_) => {
                open = Some(failed("world_unavailable"));
                break;
            }
        };
        let Some(cell) = refreshed(cell, &fresh) else {
            refusals.push(Refusal {
                cell: index,
                reason: RefusalReason::OpportunityGone,
            });
            continue;
        };
        match runner.run_cell(&cell).await {
            Ok(run) => {
                if let Some(code) = absorb(run, index, &mut committed, &mut refusals) {
                    open = Some(failed(code));
                    break;
                }
            }
            Err(error) => match classify(&error) {
                CellEnd::Budget => {
                    open = Some(RunStatus::BudgetExhausted);
                    break;
                }
                CellEnd::Fatal(code) => {
                    open = Some(failed(code));
                    break;
                }
                CellEnd::Refused(reason) => refusals.push(Refusal {
                    cell: index,
                    reason,
                }),
            },
        }
    }

    let closed = open.is_none();
    if closed {
        let clock = CommandId::derived(
            CLOCK_NAMESPACE,
            &[&snapshot.world_id.text(), &cover.tick.0.to_string()],
        );
        if mailbox.submit_clock(clock, span).await.is_err() {
            open = Some(failed("clock_refused"));
        }
    }

    let spent = port.spent();
    let report = TickReport {
        checkpoint: TickCheckpoint {
            tick: cover.tick.0,
            cells: cover.cells.len() as u32,
            committed,
            refusals,
            tokens: spent.saturating_sub(spent_before),
        },
        acts: mailbox
            .operator_log()
            .await
            .map(|log| {
                log.into_iter()
                    .filter(|event| event.revision > snapshot.revision)
                    .collect()
            })
            .unwrap_or_default(),
        spent,
        remaining: port.cap().saturating_sub(spent),
        closed: open.is_none(),
    };
    (open.map_or(TickEnd::Closed, TickEnd::Open), Some(report))
}

#[cfg(test)]
mod tests;
