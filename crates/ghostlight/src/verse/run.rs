//! The manual verse run: the tick driver and its record.
//!
//! One invocation advances the world tick by tick. A tick reads the world,
//! opens (or finds open) its tick document, runs each frozen cell through the
//! controller organ, advances the clock once by the frozen span, and
//! checkpoints. The driver owns the order of those steps and nothing else:
//! cells decide what factions do, the recording port owns spend, the kernel
//! owns time.
//!
//! A tick is named by the world's clock-advance count, never by `now`. The
//! resume point is the open tick document (world, ordinal), written before the
//! tick's first cell and read by any later run: it froze the span, the cover
//! budget and the cells when the tick opened, so a run under another span or
//! budget finishes the open tick as it was opened and applies its own config
//! from the next tick it opens. A tick whose cells did not all finish never
//! advances the clock, so the same ordinal, cells and command ids come back,
//! the controller-work rows pick up where they stopped, and the kernel answers
//! every re-submission with the original receipt. A run record reports a run
//! and is never read to resume one.
//!
//! A run starts only when it is invoked: nothing here waits, schedules or
//! repeats.

use super::calls::{RecordingInferencePort, canonical};
use crate::controllers::{
    CellRun, ControllerError, ControllerPendingReason, ControllerRunner, InferenceFaultClass,
    NarrativeRun, OperationalRun, SubmissionDisposition,
};
use crate::cover::{
    Cell, CellId, Constituent, CoverBudget, CoverBudgetError, TickIndex, derive_cover,
};
use crate::{
    AgencyGraph, CommandId, DecisionOpportunity, MailboxError, OperatorEvent, RenderPath,
    SubjectId, SubmitReceipt, TickMinutes, WorldId, WorldMailbox, WorldSnapshot,
};
use chrono::Utc;
use cultcache_rs::{CacheBackingStore, CultCacheEnvelope, OwnedRedbMessagePackBackingStore};
use serde::{Deserialize, Serialize};
use std::{ops::ControlFlow, path::Path, sync::Mutex};
use thiserror::Error;

/// The run record document's schema.
pub const RUN_RECORD_SCHEMA: &str = "ghostlight.verse_run.v1";
const RUN_ROW: &str = "verse_run.v1";
/// The tick document's schema.
pub const TICK_RECORD_SCHEMA: &str = "ghostlight.verse_tick.v1";
const TICK_ROW: &str = "verse_tick.v1";
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
    /// The cell's subject no longer holds an opportunity.
    OpportunityGone,
    /// The cell's in-flight work was bound to a scope that has since moved,
    /// so it is refused before any inference. A cell whose command already
    /// committed does not end this way: the controller answers it with its
    /// receipt whatever the scope has done since.
    ScopeMoved,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Refusal {
    /// The cell's position in the tick's cover.
    pub cell: u32,
    pub reason: RefusalReason,
}

/// A tick as the run left it. A closed tick has all its cells run and the
/// clock advanced by the tick's frozen span. The tick a run ended inside is
/// recorded too, unclosed, so what its cells committed is not lost; a later
/// run finds that tick's document open and works in it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TickCheckpoint {
    /// The tick's ordinal, as its tick document names it.
    pub tick: u64,
    pub closed: bool,
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

/// A cell as the tick document froze it: the partition's identity and its
/// members, decided once when the tick opened.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FrozenCell {
    pub id: CellId,
    /// True for a `Cell::Group`, even one a refresh has left a single member.
    pub group: bool,
    pub members: Vec<SubjectId>,
}

impl FrozenCell {
    fn of(cell: &Cell) -> Self {
        let (id, group) = match cell {
            Cell::Singleton { id, .. } => (*id, false),
            Cell::Group { id, .. } => (*id, true),
        };
        Self {
            id,
            group,
            members: cell.members().iter().map(|member| member.subject).collect(),
        }
    }
}

/// What closed a tick.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TickClose {
    /// The run with this sequence submitted the tick's clock advance.
    Clock { run: u64 },
    /// A later run found the world's ordinal past the tick with the document
    /// still open: its clock landed and the close was lost, or another clock
    /// writer moved time.
    World,
}

/// What a tick decided when it opened. Document `ghostlight.verse_tick.v1`,
/// keyed (world, ordinal). Written once before the tick's first cell, closed
/// once, never reopened or deleted.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerseTick {
    pub world_id: WorldId,
    pub ordinal: u64,
    pub opened_now: u64,
    pub opened_revision: u64,
    pub span_minutes: u32,
    pub cover_budget: RecordedCoverBudget,
    pub oversubscribed: bool,
    pub cells: Vec<FrozenCell>,
    pub opened_by_run: u64,
    pub closed: Option<TickClose>,
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

impl VerseRunRecord {
    /// The ticks this run finished: every cell run and the clock advanced.
    pub fn ticks_closed(&self) -> usize {
        self.checkpoints.iter().filter(|c| c.closed).count()
    }
}

/// What a caller asks of one run. The token cap is not here: the recording
/// port owns it.
#[derive(Clone, Copy, Debug)]
pub struct VerseRunConfig {
    pub ticks: u32,
    pub tick_span: TickMinutes,
    pub cover: CoverBudget,
}

/// A tick as the operator reads it, including the one a run ended inside.
#[derive(Clone, Debug)]
pub struct TickReport {
    pub checkpoint: TickCheckpoint,
    /// Every act the tick committed, from the operator log.
    pub acts: Vec<OperatorEvent>,
    pub spent: u64,
    pub remaining: u64,
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
            if checkpoint.closed { "closed" } else { "open" },
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
                RefusalReason::ScopeMoved => "scope_moved",
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

fn tick_key(world: WorldId, ordinal: u64) -> String {
    format!("{}/{ordinal:020}", world.key())
}

fn tick_row(tick: &VerseTick) -> Result<CultCacheEnvelope, VerseRunError> {
    Ok(CultCacheEnvelope {
        key: tick_key(tick.world_id, tick.ordinal),
        r#type: TICK_ROW.into(),
        payload: rmp_serde::to_vec_named(tick).map_err(|_| VerseRunError::Store)?,
        stored_at: Utc::now().to_rfc3339(),
        schema_id: Some(TICK_RECORD_SCHEMA.into()),
    })
}

/// Every row of the file, split by type.
struct Rows {
    runs: Vec<VerseRunRecord>,
    ticks: Vec<VerseTick>,
}

/// Run records and tick documents in one CultCache file. A run is saved when
/// it begins and again each time it moves, so a row is replaced by its own
/// later state. A tick is opened once and closed once, and only by the tick
/// driver: the writers are private to this module.
///
/// ```compile_fail
/// fn wedge(store: &ghostlight::VerseRunStore, world: ghostlight::WorldId) {
///     store.close_ticks_before(world, 1).unwrap();
/// }
/// ```
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

    fn records(&self) -> Result<Rows, VerseRunError> {
        let rows = self
            .store
            .lock()
            .map_err(|_| VerseRunError::Store)?
            .pull_all()
            .map_err(|_| VerseRunError::Store)?;
        let mut runs = Vec::new();
        let mut ticks = Vec::new();
        for row in &rows {
            match (row.r#type.as_str(), row.schema_id.as_deref()) {
                (RUN_ROW, Some(RUN_RECORD_SCHEMA)) => {
                    let record: VerseRunRecord =
                        canonical(row).map_err(|_| VerseRunError::Corrupt)?;
                    if row.key != row_key(record.world_id, record.sequence)
                        || record.sequence == 0
                    {
                        return Err(VerseRunError::Corrupt);
                    }
                    runs.push(record);
                }
                (TICK_ROW, Some(TICK_RECORD_SCHEMA)) => {
                    let tick: VerseTick = canonical(row).map_err(|_| VerseRunError::Corrupt)?;
                    if row.key != tick_key(tick.world_id, tick.ordinal) {
                        return Err(VerseRunError::Corrupt);
                    }
                    ticks.push(tick);
                }
                _ => return Err(VerseRunError::Corrupt),
            }
        }
        runs.sort_by_key(|record| (record.world_id.key(), record.sequence));
        ticks.sort_by_key(|tick| (tick.world_id.key(), tick.ordinal));
        Ok(Rows { runs, ticks })
    }

    /// The sequence the next run of this world must carry. Sequences count from
    /// one.
    pub fn next_sequence(&self, world: WorldId) -> Result<u64, VerseRunError> {
        Ok(self
            .records()?
            .runs
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
            .runs
            .into_iter()
            .filter(|record| record.world_id == world)
            .collect())
    }

    /// The tick document of (world, ordinal), when one was opened.
    pub fn tick(&self, world: WorldId, ordinal: u64) -> Result<Option<VerseTick>, VerseRunError> {
        Ok(self
            .records()?
            .ticks
            .into_iter()
            .find(|tick| tick.world_id == world && tick.ordinal == ordinal))
    }

    /// Writes a tick document that does not exist yet, open. The store's own
    /// insert-if-absent decides, so two opens of one ordinal cannot both win.
    /// Private: the tick driver is the only writer of tick rows.
    fn open_tick(&self, tick: &VerseTick) -> Result<(), VerseRunError> {
        if tick.closed.is_some() {
            return Err(VerseRunError::OutOfSequence);
        }
        let row = tick_row(tick)?;
        let inserted = self
            .store
            .lock()
            .map_err(|_| VerseRunError::Store)?
            .insert_entry_if_absent(row)
            .map_err(|_| VerseRunError::Store)?;
        if inserted {
            Ok(())
        } else {
            Err(VerseRunError::OutOfSequence)
        }
    }

    /// Closes an open tick document. Private, like every tick writer.
    fn close_tick(
        &self,
        world: WorldId,
        ordinal: u64,
        close: TickClose,
    ) -> Result<(), VerseRunError> {
        let Some(mut tick) = self.tick(world, ordinal)? else {
            return Err(VerseRunError::OutOfSequence);
        };
        if tick.closed.is_some() {
            return Err(VerseRunError::OutOfSequence);
        }
        tick.closed = Some(close);
        self.put_tick(&tick)
    }

    /// Closes, as the world's, every open tick of the world below `ordinal`.
    fn close_ticks_before(&self, world: WorldId, ordinal: u64) -> Result<(), VerseRunError> {
        for mut tick in self.records()?.ticks {
            if tick.world_id == world && tick.ordinal < ordinal && tick.closed.is_none() {
                tick.closed = Some(TickClose::World);
                self.put_tick(&tick)?;
            }
        }
        Ok(())
    }

    fn put_tick(&self, tick: &VerseTick) -> Result<(), VerseRunError> {
        let row = tick_row(tick)?;
        self.store
            .lock()
            .map_err(|_| VerseRunError::Store)?
            .push(&row)
            .map_err(|_| VerseRunError::Store)
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
        ControllerError::NoOpportunity { .. } | ControllerError::AmbiguousOpportunity => {
            CellEnd::Refused(RefusalReason::OpportunityGone)
        }
        ControllerError::OpportunityMismatch => CellEnd::Refused(RefusalReason::ScopeMoved),
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

/// The frozen cell with each member bound to the opportunity the world holds
/// for that subject now. The partition is fixed when the tick opens; an
/// opportunity is the world's current binding for a subject, and an earlier
/// cell's act can move it (a word spoken in a shared room changes what its
/// listeners know). A subject that no longer holds an opportunity leaves the
/// cell, and an empty cell is gone.
fn refreshed(frozen: &FrozenCell, tick: TickIndex, fresh: &[DecisionOpportunity]) -> Option<Cell> {
    let members: Vec<Constituent> = frozen
        .members
        .iter()
        .filter_map(|subject| {
            fresh
                .iter()
                .find(|opportunity| opportunity.scope.subject_id == *subject)
                .map(|opportunity| Constituent {
                    subject: *subject,
                    opportunity: opportunity.clone(),
                })
        })
        .collect();
    if frozen.group {
        (!members.is_empty()).then(|| Cell::Group {
            id: frozen.id,
            tick,
            members,
        })
    } else {
        members.into_iter().next().map(|member| Cell::Singleton {
            id: frozen.id,
            tick,
            member,
        })
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
            store,
            sequence,
            config,
            cover_budget,
            spent_before,
        )
        .await;
        spent_before = port.spent();
        match end {
            TickEnd::Open(status) => {
                record.status = status;
                if let Some(report) = report {
                    record.checkpoints.push(report.checkpoint.clone());
                    store.save(&record)?;
                    let _ = observe(&report);
                } else {
                    store.save(&record)?;
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

/// The id of a tick's clock advance: the world and the ordinal, nothing the
/// run holds. A retry in the same tick has the same body and is answered
/// `AlreadyApplied`.
pub(crate) fn clock_command_id(world: WorldId, ordinal: u64) -> CommandId {
    CommandId::derived(CLOCK_NAMESPACE, &[&world.text(), &ordinal.to_string()])
}

/// One tick. The report is absent only when the world or the tick document
/// could not be read at all.
#[allow(clippy::too_many_arguments)]
async fn run_tick(
    mailbox: &WorldMailbox,
    runner: &ControllerRunner,
    port: &RecordingInferencePort,
    store: &VerseRunStore,
    sequence: u64,
    config: &VerseRunConfig,
    cover_budget: CoverBudget,
    spent_before: u64,
) -> (TickEnd, Option<TickReport>) {
    let read = async {
        let snapshot = mailbox.snapshot().await?;
        let ordinal = mailbox.clock_advances().await?;
        let opportunities = mailbox.opportunities().await?;
        let graph = mailbox.agency_graph().await?;
        Ok::<_, MailboxError>((snapshot, ordinal, opportunities, graph))
    }
    .await;
    let Ok((snapshot, ordinal, opportunities, graph)) = read else {
        return (TickEnd::Open(failed("world_unavailable")), None);
    };
    let world = snapshot.world_id;
    let Ok(tick) = open_or_find(
        store,
        ordinal,
        sequence,
        config,
        cover_budget,
        &snapshot,
        &opportunities,
        &graph,
    ) else {
        return (TickEnd::Open(failed("run_store")), None);
    };
    let index = TickIndex(ordinal);
    let Some(span) = TickMinutes::new(tick.span_minutes) else {
        return (TickEnd::Open(failed("run_store")), None);
    };

    let mut committed = Vec::new();
    let mut refusals = Vec::new();
    let mut open: Option<RunStatus> = None;
    for (position, frozen) in tick.cells.iter().enumerate() {
        let position = position as u32;
        let fresh = match mailbox.opportunities().await {
            Ok(fresh) => fresh,
            Err(_) => {
                open = Some(failed("world_unavailable"));
                break;
            }
        };
        let Some(cell) = refreshed(frozen, index, &fresh) else {
            refusals.push(Refusal {
                cell: position,
                reason: RefusalReason::OpportunityGone,
            });
            continue;
        };
        match runner.run_cell(&cell).await {
            Ok(run) => {
                if let Some(code) = absorb(run, position, &mut committed, &mut refusals) {
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
                    cell: position,
                    reason,
                }),
            },
        }
    }

    let closed = open.is_none();
    if closed {
        if mailbox
            .submit_clock(clock_command_id(world, ordinal), span)
            .await
            .is_err()
        {
            open = Some(failed("clock_refused"));
        } else if store
            .close_tick(world, ordinal, TickClose::Clock { run: sequence })
            .is_err()
        {
            open = Some(failed("run_store"));
        }
    }

    let spent = port.spent();
    // The act listing is the tick's report to the operator: a log that cannot
    // be read ends the run after the tick's checkpoint, closed or not.
    let checkpoint_closed = open.is_none();
    let acts = match acts_since(mailbox.operator_log().await, snapshot.revision) {
        Some(acts) => acts,
        None => {
            open = Some(failed("operator_log_unavailable"));
            Vec::new()
        }
    };
    let report = TickReport {
        checkpoint: TickCheckpoint {
            tick: ordinal,
            closed: checkpoint_closed,
            cells: tick.cells.len() as u32,
            committed,
            refusals,
            tokens: spent.saturating_sub(spent_before),
        },
        acts,
        spent,
        remaining: port.cap().saturating_sub(spent),
    };
    (open.map_or(TickEnd::Closed, TickEnd::Open), Some(report))
}

/// The acts a tick committed: the operator log's events past the tick's
/// opening revision. A log that cannot be read is `None`, never an empty tick.
fn acts_since(
    log: Result<Vec<OperatorEvent>, MailboxError>,
    since: u64,
) -> Option<Vec<OperatorEvent>> {
    log.ok().map(|log| {
        log.into_iter()
            .filter(|event| event.revision > since)
            .collect()
    })
}

/// The tick document for `ordinal`: the open one a run left, or a new one
/// frozen from a cover derived now. An open tick the world has moved past is
/// closed as the world's first. A document already closed at the world's
/// current ordinal is a store that disagrees with the world.
#[allow(clippy::too_many_arguments)]
fn open_or_find(
    store: &VerseRunStore,
    ordinal: u64,
    sequence: u64,
    config: &VerseRunConfig,
    cover_budget: CoverBudget,
    snapshot: &WorldSnapshot,
    opportunities: &[DecisionOpportunity],
    graph: &AgencyGraph,
) -> Result<VerseTick, VerseRunError> {
    let world = snapshot.world_id;
    store.close_ticks_before(world, ordinal)?;
    if let Some(tick) = store.tick(world, ordinal)? {
        return if tick.closed.is_none() {
            Ok(tick)
        } else {
            Err(VerseRunError::OutOfSequence)
        };
    }
    let cover = derive_cover(world, TickIndex(ordinal), opportunities, graph, cover_budget);
    let tick = VerseTick {
        world_id: world,
        ordinal,
        opened_now: snapshot.now.0,
        opened_revision: snapshot.revision,
        span_minutes: config.tick_span.minutes(),
        cover_budget: RecordedCoverBudget {
            cells: cover_budget.cells,
            constituent_cap: cover_budget.constituent_cap,
            urgency_slots: cover_budget.urgency_slots,
        },
        oversubscribed: cover.oversubscribed,
        cells: cover.cells.iter().map(FrozenCell::of).collect(),
        opened_by_run: sequence,
        closed: None,
    };
    store.open_tick(&tick)?;
    Ok(tick)
}

#[cfg(test)]
mod tests;
