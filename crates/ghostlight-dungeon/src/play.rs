//! The play table's store and loop (Cut 8a).
//!
//! One play agent runs a role-playing game over a Ghostlight world: it reads
//! `table_view`, interprets the player's prose, dispatches Personas, commits
//! consequences through the kernel's own `table.rs` vocabulary, and ends the
//! turn or asks the player a question. The kernel is the only writer; this
//! table carries no world state between turns, only the turn's own inference
//! record.
//!
//! Wiring this into `AppState`, `GHOSTLIGHT_PLAY_MODEL`, `world.play`
//! ingress, and the Eve schema is Cut 8b. This cut is the store and the
//! round loop alone, so nothing here is reachable from `runtime.rs`.

use std::{
    collections::VecDeque,
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

use anyhow::{Context, bail};
use chrono::Utc;
use cultcache_rs::{CacheBackingStore, CultCacheEnvelope, OwnedRedbMessagePackBackingStore};
use ghostlight::{
    CommandBody, CommandId, ControllerError, ControllerMode, ControllerPort, DecisionInvocation,
    DecisionOpportunity, InferenceEvent, InferenceFault, InferenceFaultDisposition, InferenceOutput,
    InferencePort, InferenceRequest, KernelError, MailboxError, PersonaLane, PlayPort,
    PrincipalCommandIntent, Statement, SubjectId, TickMinutes, VerifiedPrincipalEvidence,
    WorldMailbox, WorldPatch, WorldSnapshot, actor_tools, authoring_tools, decode_actor_call,
    decode_authoring_call, decode_authoring_calls, describe_refusal, id_text, id_text_matches,
    table_view,
};
use ghostlight_persona_projection::{PersonaTurn, SourceSpan};
use codex_connector::{CodexInputItem, CodexToolDefinition};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use tokio::sync::{Mutex, Semaphore};

const STORE_TYPE: &str = "ghostlight.play_turn_store.v1";
const STORE_SCHEMA: &str = "ghostlight.play_turn_store.v1";
const STORE_KEY: &str = "primary";

/// The one round budget: exhausting it closes the turn and narrates, rather
/// than leaving it `Running` with nothing said (PA-Q8, Cut 8, PA.f66a/PA.f95).
const ROUND_BUDGET: usize = 12;

/// Bounded so a single round cannot spend the whole response budget one
/// grouped-agent tool spree; matches the grouped agent lane's own order of
/// magnitude (`controllers.rs`'s `OperationalAgent` request).
const PLAY_MAX_OUTPUT_TOKENS: u32 = 8_000;

/// How many `Retryable` inference faults one round's own request may absorb
/// (PA.f94): only `InferenceFaultDisposition::Retryable` retries at all — a
/// `RecoveryRequired` or `IntegrityViolation` fault closes the turn on its
/// first occurrence, through `close_with_fault`, since retrying either kind
/// again cannot succeed differently.
const ROUND_RETRY_BUDGET: usize = ROUND_BUDGET;

/// The key ledger's own retention window (PA.f83): a request key survives in
/// the ledger for the most recent 64 closed turns after the one that carried
/// it, then is forgotten — the store keeps only the current turn's own full
/// record, so an ancient replay past the window opens a fresh turn rather
/// than being recognized.
const KEY_LEDGER_WINDOW: usize = 64;

/// Dungeon's own subset of the library's `PATCH_TOOLS`, named in Cut 8's
/// table (PA-Q7 B). The library hard-codes none of these; this is policy, and
/// policy belongs to the caller.
const PLAY_TOOLS: &[&str] = &[
    "relocate",
    "transfer",
    "consume",
    "mint",
    "bind",
    "release",
    "witness",
    "acquire_knowledge",
    "forget",
    "create_commitment",
    "discharge_commitment",
    "advance_pressure",
    "reduce_pressure",
    "open_route",
    "close_route",
    "declare_place",
    "declare_route",
    "declare_resource",
    "declare_fact",
    "declare_subject",
    "set_persona_material",
    "retire",
    "grant_affordance",
    "revoke_affordance",
];

const DISPATCH_TOOL: &str = "dispatch";
const ADVANCE_TIME_TOOL: &str = "advance_time";
const ASK_PLAYER_TOOL: &str = "ask_player";
const END_TURN_TOOL: &str = "end_turn";

/// A handle names the player or a subject dispatched this turn; nothing else
/// carries the `__` separator into a tool name a model may call.
const HANDLE_SEPARATOR: &str = "__";

/// The table rules, sent once per round as the request's instructions. The
/// turn's own conversation is rebuilt each round from the recorded
/// `opening_prompt` — `table_view` of the snapshot as it stood the moment the
/// turn opened, captured once, not re-rendered on a later round — plus the
/// recorded `rounds` and their recorded results (PA.f111, S6): a resumed
/// round replays this exact recorded opening, never a fresh `table_view` of
/// whatever the world looks like by the time it resumes.
const PLAY_INSTRUCTIONS: &str = "You are the table: the one player-facing role-playing agent for this Ghostlight world. \
The player writes prose; you read the whole-world table view and decide what happens.\n\
\n\
Rules:\n\
- Speech and display are quotes: a `<handle>__<kind>` call's `text` and `display` must be an exact, \
verbatim quote copied from the player's own words or from a dispatched Persona's own turn prose. A \
paraphrase is refused.\n\
- An actor acts only through its own tools: use `<handle>__<kind>` for the player's handle or a \
subject you dispatched this turn, never another subject's tool under a borrowed handle.\n\
- Rulings are authoring tools: mint, transfer, relocate, and the rest of the authoring catalog are \
your own hand as the table, not an actor's act.\n\
- Ask the player with `ask_player` when an outcome needs the player's own choice or negotiation, and \
stop the turn there.\n\
- A fact you invent uses `declare_fact` with `ruled` standing; you reach a subject's knowledge of it \
through `witness` or `acquire_knowledge`.\n\
- You may `mint` quantity into existence; nothing else may.\n\
- Call `dispatch` with the subjects who should act this round before calling their tools, naming each \
subject by its full id exactly as table_view prints it in brackets — never the short <handle>__ prefix \
an actor tool's own name carries. An id naming no dispatchable subject is refused, quoting the text \
given, rather than silently doing nothing.\n\
- Call `end_turn` when the round's consequences are settled.";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
enum PlayTurnState {
    Running,
    AwaitingPlayer,
    Closed,
}

/// One `run` request. `answers` names the question this request answers, by
/// the same `QuestionId` an `AwaitingPlayer` turn currently exposes; `None`
/// is the plain continue/opening shape (PA.f84). `&str`/`String` still
/// convert directly (`answers: None`), so most call sites are unaffected by
/// this replacing the bare `text` parameter `run` used to take.
#[derive(Clone, Debug)]
pub(crate) struct PlayRequest {
    pub(crate) text: String,
    pub(crate) answers: Option<QuestionId>,
}

impl From<&str> for PlayRequest {
    fn from(text: &str) -> Self {
        Self {
            text: text.to_owned(),
            answers: None,
        }
    }
}

impl From<String> for PlayRequest {
    fn from(text: String) -> Self {
        Self { text, answers: None }
    }
}

/// The small typed value one open question is identified by (PA.f84): a
/// `run` request whose `answers` does not match the turn's own currently
/// open `QuestionId` is refused as stale rather than being applied to
/// whatever question happens to be open when the request is finally
/// processed — a late retry naming an already-answered or already-superseded
/// question must never land on a different one.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct QuestionId {
    turn_id: String,
    round: usize,
    slot: usize,
}

/// One tool call's recorded body, persisted before submission (PA-Q8): a
/// resumed call resubmits this exact value, never a value rebuilt from a
/// fresh snapshot (mutation M8.3).
#[derive(Clone, Debug, Serialize, Deserialize)]
enum RecordedCall {
    Authoring(WorldPatch),
    PlayerAct(DecisionOpportunity, DecisionInvocation),
    PersonaAct(SubjectId, DecisionOpportunity, DecisionInvocation),
    AdvanceTime(TickMinutes),
    Dispatch(Vec<SubjectId>),
    AskPlayer(String),
    EndTurn,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct CallRecord {
    call_id: String,
    round: usize,
    slot: usize,
    /// Absent only for a call this table refused before it ever reached a
    /// door: a decode failure, an unknown tool, a span that would not
    /// locate. Nothing with a body here was ever silently dropped; its
    /// `result` is always the reason.
    body: Option<RecordedCall>,
    result: Option<String>,
}

/// The play turn row (PA-Q8): one row, over `service/play-turn-v1.cc`,
/// following `app_session.rs`'s pattern. The agent's conversation is
/// re-derived each round from `opening_prompt`, `rounds`, and the recorded
/// `calls`/`persona_turns` results — the connector posture — so nothing
/// carries forward between turns except world state.
#[derive(Clone, Debug, Serialize, Deserialize)]
struct PlayTurn {
    /// Minted fresh (`Uuid::new_v4`) when the turn opens (PA.f83): never
    /// taken from any request key. A request key and a turn's own identity
    /// are different things — see `applied_keys` below and the store's own
    /// `KeyLedger` — so a stale retry of the key that opened this turn can
    /// never reproduce this turn's own command ids by opening a second turn
    /// under the same id.
    turn_id: String,
    /// Every request key this open turn has already applied (PA.f62/PA.f63):
    /// a key already here is a replay — a no-op — for as long as this turn
    /// stays the store's current one. Once the turn closes, this list is
    /// archived into the store's own `KeyLedger` (PA.f83), which is where a
    /// replay of one of these keys is recognized after a new turn has opened
    /// in this one's place. No `#[serde(default)]`: the canonical re-encode
    /// check in `PlayTurnStore::open` refuses a row missing this field, so a
    /// default here could never actually apply (PA.f95).
    applied_keys: Vec<String>,
    opening_prompt: String,
    player_prose: Vec<String>,
    rounds: Vec<InferenceOutput>,
    calls: Vec<CallRecord>,
    persona_turns: Vec<(SubjectId, PersonaTurn)>,
    question: Option<String>,
    refusal: Option<String>,
    narration: Option<String>,
    /// Set when the turn closed on a fault rather than the player's own
    /// consequences settling: an inference fault that does not retry, an
    /// inference exhausted its retry budget (PA.f65/PA.f94), a request-build,
    /// snapshot, or narrate failure after the turn opened (PA.f85/PA.f93), or
    /// a kernel `Invariant` error poisoned the table (PA.f66b). `None` for a
    /// turn that closed through `end_turn` or the round budget. No
    /// `#[serde(default)]`: same reasoning as `applied_keys` above (PA.f95).
    fault: Option<String>,
    state: PlayTurnState,
}

impl PlayTurn {
    /// Every subject a dispatch call this turn's own record names has
    /// *successfully* acted (PA.f90): a dispatch whose Persona turn faulted,
    /// or found no live opportunity, never produced prose this turn can
    /// quote, so it does not count as dispatched — offering its actor tools
    /// next round would let the agent call them with nothing behind them.
    /// `persona_turns` is the one record of an actually-recorded turn; a
    /// subject's own presence there, not merely its presence in a
    /// `Dispatch` call's own subject list, is what this reads.
    fn dispatched_subjects(&self) -> Vec<SubjectId> {
        let mut subjects = Vec::new();
        for call in &self.calls {
            if let Some(RecordedCall::Dispatch(list)) = &call.body {
                for subject in list {
                    if !subjects.contains(subject) && self.persona_turns.iter().any(|(id, _)| id == subject) {
                        subjects.push(*subject);
                    }
                }
            }
        }
        subjects
    }

    fn call_record(&self, round: usize, slot: usize) -> Option<&CallRecord> {
        self.calls
            .iter()
            .find(|call| call.round == round && call.slot == slot)
    }

    fn call_record_mut(&mut self, round: usize, slot: usize) -> Option<&mut CallRecord> {
        self.calls
            .iter_mut()
            .find(|call| call.round == round && call.slot == slot)
    }

    /// The currently open question's own `QuestionId`, when the turn is
    /// `AwaitingPlayer` (PA.f84): derived from the last round's own
    /// unresolved `ask_player` call, the same slot `find_ask_player_slot`
    /// locates to record an answer's result.
    fn open_question_id(&self) -> Option<QuestionId> {
        if self.state != PlayTurnState::AwaitingPlayer {
            return None;
        }
        let round = self.rounds.len().checked_sub(1)?;
        let slot = find_ask_player_slot(self, round)?;
        Some(QuestionId {
            turn_id: self.turn_id.clone(),
            round,
            slot,
        })
    }
}

#[derive(Clone, Serialize, Deserialize)]
struct PlayTurnStoreState {
    schema: String,
    turn: Option<PlayTurn>,
    /// A closed turn's own applied request keys, kept for `KEY_LEDGER_WINDOW`
    /// more closed turns after it (PA.f83). No `#[serde(default)]`, matching
    /// `PlayTurn::applied_keys`/`fault` (PA.f95): `PlayTurnStore::open`'s own
    /// canonical re-encode check refuses any row missing a declared field
    /// regardless, so a default here could never actually apply either.
    ledger: KeyLedger,
}

/// One closed turn's own applied request keys, oldest closed turn first.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
struct KeyLedger {
    closed: VecDeque<ClosedTurnKeys>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct ClosedTurnKeys {
    turn_id: String,
    keys: Vec<String>,
}

impl KeyLedger {
    fn contains(&self, key: &str) -> bool {
        self.closed.iter().any(|entry| entry.keys.iter().any(|existing| existing == key))
    }

    /// The `turn_id` of the closed turn `key` was applied to, if it is still
    /// within the window (PA.f109): `ClosedTurnKeys::turn_id` is written on
    /// every archive but was never read anywhere until this — a replay
    /// recognized through the ledger can now say which turn it replayed.
    fn turn_id_for(&self, key: &str) -> Option<String> {
        self.closed
            .iter()
            .find(|entry| entry.keys.iter().any(|existing| existing == key))
            .map(|entry| entry.turn_id.clone())
    }

    /// Archives one turn's own applied keys as it closes out of the store's
    /// single current-turn row, then evicts down to `KEY_LEDGER_WINDOW`
    /// closed turns, oldest first.
    fn push_closed(&mut self, turn_id: String, keys: Vec<String>) {
        if keys.is_empty() {
            return;
        }
        self.closed.push_back(ClosedTurnKeys { turn_id, keys });
        while self.closed.len() > KEY_LEDGER_WINDOW {
            self.closed.pop_front();
        }
    }
}

/// `PlayTurnStore` over `service/play-turn-v1.cc`: one `OwnedRedbMessagePackBackingStore`
/// row, the same custody shape `app_session.rs`'s `AppSessionOwner` uses.
struct PlayTurnStore {
    store: OwnedRedbMessagePackBackingStore,
    row: CultCacheEnvelope,
    state: PlayTurnStoreState,
    healthy: bool,
}

impl PlayTurnStore {
    fn open(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let store = OwnedRedbMessagePackBackingStore::new(path.as_ref())?;
        store
            .validate_path_identity()
            .context("play-turn store path identity is invalid")?;
        let rows = store.pull_all()?;
        let (row, state) = match rows.as_slice() {
            [] => {
                let state = PlayTurnStoreState {
                    schema: STORE_SCHEMA.into(),
                    turn: None,
                    ledger: KeyLedger::default(),
                };
                let row = envelope(&state)?;
                if !store.compare_and_swap_batch(&[], vec![row.clone()])? {
                    bail!("play-turn store changed during initialization");
                }
                (row, state)
            }
            [row]
                if row.r#type == STORE_TYPE
                    && row.key == STORE_KEY
                    && row.schema_id.as_deref() == Some(STORE_SCHEMA) =>
            {
                let state: PlayTurnStoreState =
                    rmp_serde::from_slice(&row.payload).context("play-turn state is corrupt")?;
                if rmp_serde::to_vec_named(&state)? != row.payload {
                    bail!("play-turn state is not canonical");
                }
                if state.schema != STORE_SCHEMA {
                    bail!("play-turn state schema is invalid");
                }
                (row.clone(), state)
            }
            _ => bail!("play-turn v1 store contains foreign or legacy records; start with a fresh store"),
        };
        Ok(Self {
            store,
            row,
            state,
            healthy: true,
        })
    }

    fn current(&self) -> Option<&PlayTurn> {
        self.state.turn.as_ref()
    }

    /// PA.f99's own test seam: re-reads the current turn through a fresh
    /// `pull_all` against the *same already-owned* redb `Database` this
    /// store already holds — never the cached `self.state` field a mutation
    /// could leave stale, and never a second `OwnedRedbMessagePackBackingStore::new`
    /// on the same path, which CultCache's own single-owner lock refuses
    /// outright (`redb CultCache <path> already has an active owner`): this
    /// crate learned that the hard way building this very seam. `pull_all`
    /// opens its own fresh redb read transaction against the one open
    /// `Database`, so this reflects exactly what the last successful
    /// `compare_and_swap_batch` durably committed, independent of whether
    /// `self.state` was ever updated at all.
    #[cfg(test)]
    fn pull_current_from_disk_for_test(&self) -> Option<PlayTurn> {
        let rows = self.store.pull_all().ok()?;
        let row = rows.into_iter().find(|row| row.r#type == STORE_TYPE && row.key == STORE_KEY)?;
        let state: PlayTurnStoreState = rmp_serde::from_slice(&row.payload).ok()?;
        state.turn
    }

    /// The `turn_id` of the turn `key` was already applied to, if any
    /// (PA.f109): the still-open current turn, when `key` is recorded on it,
    /// or the ledger's own archived record of one of the most recent
    /// `KEY_LEDGER_WINDOW` closed turns (PA.f83). Either way, `run` treats a
    /// known key as a no-op — a `Replayed` outcome naming the turn it
    /// replayed, not a fresh open.
    fn turn_id_for_key(&self, key: &str) -> Option<String> {
        if let Some(turn) = &self.state.turn
            && turn.applied_keys.iter().any(|existing| existing == key)
        {
            return Some(turn.turn_id.clone());
        }
        self.state.ledger.turn_id_for(key)
    }

    /// Persists `turn` as an update to the store's own still-open current
    /// turn: the ledger is carried forward unchanged, because no turn is
    /// closing out of the row here.
    fn commit(&mut self, turn: PlayTurn) -> anyhow::Result<()> {
        self.ensure_owned()?;
        let next = PlayTurnStoreState {
            schema: STORE_SCHEMA.into(),
            turn: Some(turn),
            ledger: self.state.ledger.clone(),
        };
        self.swap_in(next)
    }

    /// Opens `turn` as the store's new current turn (PA.f83): whatever turn
    /// currently occupies the row — always `Closed`, by `PlayTable::run`'s
    /// own invariant that only a closed turn (or no turn) is ever replaced —
    /// has its own applied keys archived into the ledger first, so a later
    /// replay of one of them is still recognized after its full record is
    /// gone.
    fn open_new_turn(&mut self, turn: PlayTurn) -> anyhow::Result<()> {
        self.ensure_owned()?;
        let mut ledger = self.state.ledger.clone();
        if let Some(previous) = &self.state.turn {
            ledger.push_closed(previous.turn_id.clone(), previous.applied_keys.clone());
        }
        let next = PlayTurnStoreState {
            schema: STORE_SCHEMA.into(),
            turn: Some(turn),
            ledger,
        };
        self.swap_in(next)
    }

    fn swap_in(&mut self, next: PlayTurnStoreState) -> anyhow::Result<()> {
        let next_row = envelope(&next)?;
        let swapped = self
            .store
            .compare_and_swap_batch(std::slice::from_ref(&self.row), vec![next_row.clone()]);
        if !matches!(swapped, Ok(true)) {
            self.healthy = false;
            bail!("play-turn commit outcome is uncertain; sole ownership was lost");
        }
        self.row = next_row;
        self.state = next;
        Ok(())
    }

    fn ensure_owned(&mut self) -> anyhow::Result<()> {
        if !self.healthy {
            bail!("play-turn custody is poisoned; reopen the owner");
        }
        if self.store.validate_path_identity().is_err() {
            self.healthy = false;
            bail!("play-turn store ownership was lost");
        }
        Ok(())
    }
}

fn envelope(state: &PlayTurnStoreState) -> anyhow::Result<CultCacheEnvelope> {
    Ok(CultCacheEnvelope {
        key: STORE_KEY.into(),
        r#type: STORE_TYPE.into(),
        payload: rmp_serde::to_vec_named(state)?,
        stored_at: Utc::now().to_rfc3339(),
        schema_id: Some(STORE_SCHEMA.into()),
    })
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum PlayError {
    #[error("the play table is poisoned by a prior kernel invariant fault; restart to clear it")]
    Poisoned,
    #[error("world mailbox error: {0}")]
    World(#[from] MailboxError),
    #[error("controller error: {0}")]
    Controller(#[from] ControllerError),
    #[error("play store error: {0}")]
    Store(#[from] anyhow::Error),
    #[error("no live human opportunity for the player")]
    NoPlayerOpportunity,
    #[error("an empty or whitespace-only answer is refused")]
    EmptyAnswer,
    /// PA.f84: `request.answers` did not name the turn's own currently open
    /// `QuestionId` — either it named a different, no-longer-open question
    /// (a late retry), or no question is open at all while the turn is
    /// `AwaitingPlayer`, which cannot actually happen but is refused the
    /// same way rather than assumed away.
    #[error("the answer does not name the currently open question; it is refused as stale")]
    StaleAnswer,
    /// PA.f84: `request.answers` was given while no question is open at all
    /// — the turn is `Running`, or there is no open turn to answer.
    #[error("an answer was given but no question is currently open")]
    NoQuestionOpen,
    /// PA.f84: the turn is `Running` and `request.text` is non-empty; only
    /// an empty continue is accepted while a prior request's own rounds are
    /// still being resolved.
    #[error("the turn is still running its own prior request; only an empty continue is accepted")]
    TurnStillRunning,
    /// PA.f108: empty or whitespace-only text with no turn currently open to
    /// continue (no turn at all, or the current one is `Closed`) used to
    /// open a fresh turn anyway, with prose `[""]`, and run a full inference
    /// over nothing the player actually said. A new turn needs the player's
    /// own words.
    #[error("a new turn needs non-empty text; an empty or whitespace-only opening is refused")]
    EmptyOpening,
}

/// One `run` call's own outcome (PA.f109): `Replayed` names the turn a
/// request key already known — recorded on the still-open current turn, or
/// archived in the store's own `KeyLedger` — replayed, so `ClosedTurnKeys`'s
/// own `turn_id` field, written on every archive, finally has a reader.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum RunOutcome {
    Ran,
    Replayed { turn_id: String },
}

/// One outcome of executing a round's already-decided tool calls.
#[derive(Clone, Copy)]
enum RoundOutcome {
    /// Every call in the round resolved; the loop should infer the next one.
    Continue,
    AwaitingPlayer,
    Closed,
    /// A kernel `Invariant` fault closed the turn and poisoned the table.
    Poisoned,
}

/// The play table (Cut 8): one operational model running a role-playing game
/// over a Ghostlight world. `play` is Dungeon's one `PlayPort`, minted here
/// and nowhere else in production (negative grep, matching `PlayPort`'s own
/// doc comment).
pub(crate) struct PlayTable {
    play: PlayPort,
    world: WorldMailbox,
    personas: PersonaLane,
    inference: Arc<dyn InferencePort>,
    model: String,
    permits: Arc<Semaphore>,
    store: Mutex<PlayTurnStore>,
    poisoned: AtomicBool,
    /// PA.f99's own test seam: fired immediately before each of the three
    /// doors that submit a call's already-persisted body
    /// (`commit_authoring_run`, `execute_advance_time`, `execute_actor_call`)
    /// actually submits it. It hands the hook the current turn as a fresh
    /// `pull_all` against the store's own already-owned redb `Database`
    /// finds it (`PlayTurnStore::pull_current_from_disk_for_test`) — not the
    /// store's cached `self.state`, and not a second, independently opened
    /// store, which CultCache's own single-owner lock refuses outright. No
    /// production call installs one.
    #[cfg(test)]
    before_submit_hook: std::sync::Mutex<Option<Box<dyn Fn(Option<PlayTurn>) + Send + Sync>>>,
}

impl PlayTable {
    pub(crate) fn new(
        world: WorldMailbox,
        personas: PersonaLane,
        inference: Arc<dyn InferencePort>,
        model: String,
        permits: Arc<Semaphore>,
        store_path: impl AsRef<Path>,
    ) -> anyhow::Result<Self> {
        let play = PlayPort::new(world.clone());
        let store = PlayTurnStore::open(store_path)?;
        Ok(Self {
            play,
            world,
            personas,
            inference,
            model,
            permits,
            store: Mutex::new(store),
            poisoned: AtomicBool::new(false),
            #[cfg(test)]
            before_submit_hook: std::sync::Mutex::new(None),
        })
    }

    /// Installs PA.f99's own before-submit test hook (see the field's own
    /// doc comment). Test-only: no production call site exists.
    #[cfg(test)]
    fn set_before_submit_hook(&self, hook: impl Fn(Option<PlayTurn>) + Send + Sync + 'static) {
        *self.before_submit_hook.lock().unwrap() = Some(Box::new(hook));
    }

    #[cfg(test)]
    async fn fire_before_submit_hook(&self) {
        let installed = self.before_submit_hook.lock().unwrap().is_some();
        if !installed {
            return;
        }
        let durable_turn = self.store.lock().await.pull_current_from_disk_for_test();
        if let Some(hook) = self.before_submit_hook.lock().unwrap().as_ref() {
            hook(durable_turn);
        }
    }

    #[cfg(not(test))]
    async fn fire_before_submit_hook(&self) {}

    /// One `world.play` turn. `key` is the caller's own idempotency key
    /// (PA-Q8, PA.f62/PA.f63/PA.f83): it is not the turn's identity — the
    /// turn mints its own `turn_id` when it opens. A key already known —
    /// recorded on the currently open turn, or archived in the store's own
    /// `KeyLedger` from one of the most recent 64 closed turns — is a replay
    /// and is a no-op, making zero inference calls. A fresh key reaching an
    /// open turn continues it: as the player's own answer, when it is
    /// `AwaitingPlayer` and `request.answers` names that turn's own currently
    /// open question (PA.f84; any other `answers`, or none at all, is
    /// refused as stale rather than being applied to whatever question
    /// happens to be open), or as a plain continue when it is `Running` and
    /// `request.text` is empty (non-empty text is refused: this turn is
    /// still running its own prior request and drops nothing silently). A
    /// fresh key reaching no turn, or a closed one, opens a new turn under
    /// that key (Cut 8a's "one row": only the current turn's own record is
    /// ever kept, so the closed turn's record is archived into the ledger
    /// and then discarded).
    pub(crate) async fn run(
        &self,
        principal: &VerifiedPrincipalEvidence,
        key: String,
        request: PlayRequest,
    ) -> Result<RunOutcome, PlayError> {
        if self.poisoned.load(Ordering::SeqCst) {
            return Err(PlayError::Poisoned);
        }
        let PlayRequest { text, answers } = request;
        let (stored, replayed_turn_id) = {
            let store = self.store.lock().await;
            (store.current().cloned(), store.turn_id_for_key(&key))
        };
        if let Some(turn_id) = replayed_turn_id {
            return Ok(RunOutcome::Replayed { turn_id });
        }

        let (mut turn, is_new_turn) = match stored {
            Some(existing) if existing.state == PlayTurnState::AwaitingPlayer => {
                let open_question = existing.open_question_id();
                match (answers, open_question) {
                    (Some(given), Some(open)) if given == open => {
                        if text.trim().is_empty() {
                            // Refused before anything is recorded (PA.f65):
                            // the question stays open, the key is never
                            // marked applied, and nothing about the stored
                            // turn changes.
                            return Err(PlayError::EmptyAnswer);
                        }
                        (self.answer_turn(existing, key, text), false)
                    }
                    // A missing or mismatched `answers` is refused as stale
                    // (PA.f84): a late retry naming an old question must
                    // never land on whatever question happens to be open now.
                    _ => return Err(PlayError::StaleAnswer),
                }
            }
            Some(existing) if existing.state == PlayTurnState::Running => {
                if answers.is_some() {
                    return Err(PlayError::NoQuestionOpen);
                }
                if text.trim().is_empty() {
                    (self.continue_turn(existing, key), false)
                } else {
                    return Err(PlayError::TurnStillRunning);
                }
            }
            _ => {
                if answers.is_some() {
                    return Err(PlayError::NoQuestionOpen);
                }
                // PA.f108: no turn is open to continue (none at all, or the
                // current one is `Closed`), so this text opens a fresh one —
                // and a fresh turn needs the player's own words, not an
                // empty or whitespace-only prompt that would run a full
                // inference over nothing said.
                if text.trim().is_empty() {
                    return Err(PlayError::EmptyOpening);
                }
                (self.begin_turn(key, text).await?, true)
            }
        };
        if is_new_turn {
            self.store.lock().await.open_new_turn(turn.clone())?;
        } else {
            self.persist(&turn).await?;
        }

        loop {
            match turn.state {
                PlayTurnState::AwaitingPlayer | PlayTurnState::Closed => break,
                PlayTurnState::Running => {}
            }
            let round = turn.rounds.len().saturating_sub(1);
            let round_is_open = !turn.rounds.is_empty() && round_incomplete(&turn, round);
            if round_is_open {
                match self.execute_round(&mut turn, round, principal).await? {
                    RoundOutcome::Continue => {
                        self.persist(&turn).await?;
                        continue;
                    }
                    RoundOutcome::AwaitingPlayer | RoundOutcome::Closed => {
                        self.persist(&turn).await?;
                        break;
                    }
                    RoundOutcome::Poisoned => {
                        // `execute_round` has already closed `turn` and
                        // recorded the fault (PA.f66b): this door only marks
                        // the table poisoned and persists that state.
                        self.poisoned.store(true, Ordering::SeqCst);
                        self.persist(&turn).await?;
                        return Err(PlayError::Poisoned);
                    }
                }
            }
            if turn.rounds.len() >= ROUND_BUDGET {
                // The round budget is exhausted (PA.f66a): the turn closes
                // and narrates through the same door `end_turn` uses, rather
                // than being left open with nothing said. A snapshot or
                // narrate failure inside `close_turn` still closes the turn,
                // with the fault recorded, through `close_with_fault`
                // (PA.f85/PA.f93).
                match self.close_turn(&turn).await {
                    Ok(narration) => {
                        turn.narration = Some(narration);
                        turn.state = PlayTurnState::Closed;
                        self.persist(&turn).await?;
                    }
                    Err(error) => self.close_with_fault(&mut turn, error.to_string()).await?,
                }
                break;
            }
            let round = turn.rounds.len();
            match self.infer_round(&turn, round).await {
                Ok(output) => {
                    turn.rounds.push(output);
                    self.persist(&turn).await?;
                }
                Err(detail) => {
                    // PA.f85/PA.f93/PA.f94: a request-build failure, an
                    // inference fault whose own disposition never retries
                    // (`RecoveryRequired`/`IntegrityViolation`), or a
                    // `Retryable` fault that exhausted its retry budget, all
                    // close the turn with the fault recorded through the one
                    // shared door, rather than leaving it `Running`.
                    self.close_with_fault(&mut turn, detail).await?;
                    break;
                }
            }
        }
        Ok(RunOutcome::Ran)
    }

    /// Opens a fresh turn. Takes no principal (PA.f95: the parameter was
    /// never read) — the player's own subject is resolved per-round from the
    /// live snapshot, not from the caller's own evidence.
    async fn begin_turn(&self, key: String, text: String) -> Result<PlayTurn, PlayError> {
        let snapshot = self.play.snapshot().await?;
        let opening_prompt = format!(
            "{}\n\nThe player writes:\n{}\n",
            table_view(&snapshot),
            text
        );
        Ok(PlayTurn {
            turn_id: uuid::Uuid::new_v4().to_string(),
            applied_keys: vec![key],
            opening_prompt,
            player_prose: vec![text],
            rounds: Vec::new(),
            calls: Vec::new(),
            persona_turns: Vec::new(),
            question: None,
            refusal: None,
            narration: None,
            fault: None,
            state: PlayTurnState::Running,
        })
    }

    /// Applies the player's own answer to the currently open question,
    /// already checked by `run` to name that question's own `QuestionId`.
    fn answer_turn(&self, mut turn: PlayTurn, key: String, text: String) -> PlayTurn {
        turn.applied_keys.push(key);
        turn.player_prose.push(text.clone());
        let round = turn.rounds.len().saturating_sub(1);
        if let Some(slot) = find_ask_player_slot(&turn, round) {
            if let Some(record) = turn.call_record_mut(round, slot) {
                record.result = Some(text);
            }
        }
        turn.question = None;
        turn.state = PlayTurnState::Running;
        turn
    }

    /// A plain continue of a `Running` turn under a fresh key: nothing about
    /// the turn's own record changes beyond the key itself (PA.f84).
    fn continue_turn(&self, mut turn: PlayTurn, key: String) -> PlayTurn {
        turn.applied_keys.push(key);
        turn
    }

    /// Closes `turn` with `detail` as its recorded fault and persists it:
    /// the one door every failure reachable only after a turn has already
    /// opened goes through (PA.f85/PA.f93) — a request-build failure, a
    /// snapshot or narrate failure, an inference fault whose disposition
    /// does not retry or that exhausted its retry budget, or a kernel
    /// `Invariant` fault. Distinct from poisoning the whole table: that
    /// stays each caller's own decision (`RoundOutcome::Poisoned`), because
    /// this helper only ever writes the one turn's own record.
    async fn close_with_fault(&self, turn: &mut PlayTurn, detail: String) -> Result<(), PlayError> {
        turn.state = PlayTurnState::Closed;
        turn.fault = Some(detail);
        self.persist(turn).await
    }

    /// Runs one round's own inference to completion, or returns the fault
    /// detail the caller closes the turn with (PA.f85/PA.f93/PA.f94): a
    /// request-build failure (the snapshot, `round_tools`, or
    /// `InferenceRequest::play` itself), or an `InferenceFault` whose own
    /// disposition does not retry, or that exhausted its bounded retry
    /// budget.
    async fn infer_round(&self, turn: &PlayTurn, round: usize) -> Result<InferenceOutput, String> {
        let snapshot = self.play.snapshot().await.map_err(|error| error.to_string())?;
        let dispatched = turn.dispatched_subjects();
        let tools = self.round_tools(&snapshot, &dispatched)?;
        let conversation = rebuild_conversation(turn, round);
        let command_id = CommandId::parse_uuid(&turn.turn_id).map_err(|error| error.to_string())?;

        let mut attempt = 0usize;
        loop {
            let request = InferenceRequest::play(
                command_id,
                round,
                &self.model,
                PLAY_INSTRUCTIONS,
                conversation.clone(),
                tools.clone(),
                PLAY_MAX_OUTPUT_TOKENS,
            )
            .map_err(|error| error.to_string())?;
            let prepared = match self.inference.prepare(request) {
                Ok(prepared) => prepared,
                Err(fault) => {
                    self.absorb_or_close(fault, &mut attempt).await?;
                    continue;
                }
            };
            match self.inference.infer(prepared).await {
                Ok(output) => return Ok(output),
                Err(fault) => {
                    self.absorb_or_close(fault, &mut attempt).await?;
                    continue;
                }
            }
        }
    }

    /// One inference fault's own disposition decides its fate (PA.f94): only
    /// `Retryable` retries at all, bounded by `ROUND_RETRY_BUDGET` and a
    /// small capped exponential backoff (`retry_delay`, zero under test);
    /// `RecoveryRequired` and `IntegrityViolation` never retry, closing on
    /// their first occurrence — retrying either again cannot succeed
    /// differently. `Ok(())` means the caller's loop should try again;
    /// `Err(detail)` is the fault's own detail text, to close the turn with.
    async fn absorb_or_close(&self, fault: InferenceFault, attempt: &mut usize) -> Result<(), String> {
        match fault.disposition() {
            InferenceFaultDisposition::Retryable if *attempt < ROUND_RETRY_BUDGET => {
                let delay = self.retry_delay(*attempt);
                *attempt += 1;
                if !delay.is_zero() {
                    tokio::time::sleep(delay).await;
                }
                Ok(())
            }
            InferenceFaultDisposition::Retryable
            | InferenceFaultDisposition::RecoveryRequired
            | InferenceFaultDisposition::IntegrityViolation => Err(fault.to_string()),
        }
    }

    /// A small capped exponential backoff between retries of a `Retryable`
    /// inference fault, zero in test builds so a test never actually sleeps
    /// (PA.f94). PA.f102: the skip is its own small injected decision
    /// (`skip_retry_delay_in_test`), never a `#[cfg(test)]` swap over the
    /// formula itself — `backoff_delay` below is a free function, compiled
    /// and directly unit-testable in every build, production included.
    fn retry_delay(&self, attempt: usize) -> Duration {
        if self.skip_retry_delay_in_test() {
            Duration::ZERO
        } else {
            backoff_delay(attempt)
        }
    }

    #[cfg(test)]
    fn skip_retry_delay_in_test(&self) -> bool {
        true
    }

    #[cfg(not(test))]
    fn skip_retry_delay_in_test(&self) -> bool {
        false
    }

    fn round_tools(
        &self,
        snapshot: &WorldSnapshot,
        dispatched: &[SubjectId],
    ) -> Result<Vec<CodexToolDefinition>, String> {
        let mut tools = authoring_tools(PLAY_TOOLS).map_err(|error| error.to_string())?;
        tools.extend(control_tools());
        // PA.f97: a handle collision anywhere in the snapshot's own subjects
        // is a turn fault — `parse_dispatch` and `resolve_handle` both match
        // by handle against every subject the snapshot carries, not only the
        // ones this round happens to offer tools for.
        if let Some(handle) = find_handle_collision(snapshot.subjects.iter().map(|row| (handle_for(row.id), row.id))) {
            return Err(format!(
                "two subjects share the handle `{handle}`; the play table cannot address them safely"
            ));
        }
        let Some(player) = player_subject(snapshot) else {
            return Ok(tools);
        };
        let mut player_tools = actor_tools(&actor_prefix(player.id), snapshot, player.id);
        annotate_actor_tools(&mut player_tools, snapshot, player.id);
        tools.extend(player_tools);
        for subject in dispatched {
            let mut subject_tools = actor_tools(&actor_prefix(*subject), snapshot, *subject);
            annotate_actor_tools(&mut subject_tools, snapshot, *subject);
            tools.extend(subject_tools);
        }
        Ok(tools)
    }

    /// Executes every call in `round` that has not yet resolved, in order.
    async fn execute_round(
        &self,
        turn: &mut PlayTurn,
        round: usize,
        principal: &VerifiedPrincipalEvidence,
    ) -> Result<RoundOutcome, PlayError> {
        let output = turn.rounds[round].clone();
        // PA.f103: this door's own `close_with_fault` still calls `persist`
        // under the hood, so a store/persist failure right here has no lower
        // layer left to record it in — it propagates through the `?` on
        // `close_with_fault`'s own `Result` instead of being written into
        // the turn, which is exactly the failure that would need writing.
        // No deterministic, in-process way to force `self.play.snapshot()`
        // itself to fail exists in this crate's own test harness — it fails
        // only when the world mailbox's own background actor is gone, which
        // no fixture here tears down independently of the `WorldMailbox` the
        // whole test depends on — so this site is reported, not covered by
        // a test, and stays on this one shared helper rather than growing
        // its own bespoke handling.
        let snapshot = match self.play.snapshot().await {
            Ok(snapshot) => snapshot,
            Err(error) => {
                self.close_with_fault(turn, error.to_string()).await?;
                return Ok(RoundOutcome::Closed);
            }
        };
        // PA.f105: the same handle-collision check `round_tools` runs before
        // inference must also run here — a round resumed after a crash
        // executes its already-decided calls through this door directly,
        // never through `round_tools` at all, so a collision that appeared
        // in the snapshot since the round was first inferred must still
        // refuse the whole round as a turn fault rather than resolving calls
        // against a handle that no longer names one subject unambiguously.
        if let Some(handle) =
            find_handle_collision(snapshot.subjects.iter().map(|row| (handle_for(row.id), row.id)))
        {
            let detail = format!(
                "two subjects share the handle `{handle}`; the play table cannot address them safely"
            );
            self.close_with_fault(turn, detail).await?;
            return Ok(RoundOutcome::Closed);
        }
        let player = player_subject(&snapshot);

        // Slotted, unresolved tool calls only: `slot` must still count a
        // call this loop skips as already resolved, so slot numbering never
        // shifts between a round's first pass and its resumed one.
        let mut calls: Vec<(usize, &str, &str, &str)> = Vec::new();
        let mut slot = 0usize;
        for event in output.events() {
            let InferenceEvent::ToolCall {
                call_id,
                name,
                arguments,
            } = event
            else {
                continue;
            };
            let this_slot = slot;
            slot += 1;
            if turn
                .call_record(round, this_slot)
                .is_some_and(|call| call.result.is_some())
            {
                continue;
            }
            calls.push((this_slot, call_id.as_str(), name.as_str(), arguments.as_str()));
        }

        let mut index = 0usize;
        while index < calls.len() {
            let (this_slot, call_id, name, arguments) = calls[index];

            // A maximal run of consecutive authoring calls commits together
            // as one shared patch under one command id (PA.f54): actor calls
            // and table tools break a run.
            if PLAY_TOOLS.contains(&name) {
                let mut run = Vec::new();
                while index < calls.len() && PLAY_TOOLS.contains(&calls[index].2) {
                    let (slot, call_id, name, arguments) = calls[index];
                    run.push((slot, call_id.to_owned(), name.to_owned(), arguments.to_owned()));
                    index += 1;
                }
                let results = self.commit_authoring_run(turn, round, &run, &snapshot).await?;
                let mut fault = None;
                for (slot, call_id, outcome, result) in results {
                    if matches!(outcome, RoundOutcome::Poisoned) {
                        fault = Some(result.clone());
                    }
                    record_call(turn, &call_id, round, slot, None, Some(result));
                }
                self.persist(turn).await?;
                if let Some(detail) = fault {
                    // PA.f66b: a kernel `Invariant` fault closes the turn
                    // with the fault recorded, not left `Running`.
                    self.close_with_fault(turn, detail).await?;
                    return Ok(RoundOutcome::Poisoned);
                }
                continue;
            }
            index += 1;

            if name == END_TURN_TOOL {
                record_call(turn, call_id, round, this_slot, Some(RecordedCall::EndTurn), None);
                match self.close_turn(turn).await {
                    Ok(narration) => {
                        if let Some(record) = turn.call_record_mut(round, this_slot) {
                            record.result = Some("the turn ended".to_owned());
                        }
                        turn.narration = Some(narration);
                        turn.state = PlayTurnState::Closed;
                    }
                    Err(error) => {
                        // PA.f85/PA.f93: a narrate failure still closes the
                        // turn, with the fault recorded, rather than leaving
                        // it stuck `Running` behind a `?`.
                        self.close_with_fault(turn, error.to_string()).await?;
                    }
                }
                return Ok(RoundOutcome::Closed);
            }

            if name == ASK_PLAYER_TOOL {
                let question = parse_ask_player(arguments);
                record_call(
                    turn,
                    call_id,
                    round,
                    this_slot,
                    Some(RecordedCall::AskPlayer(question.clone())),
                    None,
                );
                turn.question = Some(question);
                turn.state = PlayTurnState::AwaitingPlayer;
                return Ok(RoundOutcome::AwaitingPlayer);
            }

            if name == DISPATCH_TOOL {
                let (subjects, refusals) = parse_dispatch(arguments, &snapshot);
                let (outcome, dispatch_summary) =
                    self.execute_dispatch(turn, round, this_slot, &subjects).await?;
                // PA.f98, PA.f114: bad input — an unknown id, a non-string
                // entry, a malformed body, or a missing `subjects` key — is
                // refused in the tool result, quoting what was given, never
                // dropped silently.
                let result = if refusals.is_empty() {
                    dispatch_summary
                } else {
                    let refusals = refusals.join("; ");
                    if dispatch_summary.is_empty() {
                        refusals
                    } else {
                        format!("{dispatch_summary}; {refusals}")
                    }
                };
                record_call(
                    turn,
                    call_id,
                    round,
                    this_slot,
                    Some(RecordedCall::Dispatch(subjects)),
                    Some(result),
                );
                if matches!(outcome, RoundOutcome::Closed) {
                    // PA.f85/PA.f93: `execute_dispatch` already closed the
                    // turn with its fault recorded, mid-loop; nothing later
                    // in this round resolves.
                    return Ok(RoundOutcome::Closed);
                }
                continue;
            }

            if name == ADVANCE_TIME_TOOL {
                let (outcome, result) = self
                    .execute_advance_time(turn, call_id, round, this_slot, arguments)
                    .await?;
                let fault = matches!(outcome, RoundOutcome::Poisoned).then(|| result.clone());
                record_call(turn, call_id, round, this_slot, None, Some(result));
                self.persist(turn).await?;
                if let Some(detail) = fault {
                    self.close_with_fault(turn, detail).await?;
                    return Ok(RoundOutcome::Poisoned);
                }
                continue;
            }

            // `<handle>__<kind>` actor call: `handle` alone picks the actor
            // (Dungeon's own vocabulary — the library owns no notion of
            // which subject was dispatched this turn); the full `name` is
            // then handed to `execute_actor_call` unstripped, so the
            // library's own `decode_actor_call` owns the prefix strip.
            if let Some((handle, _)) = name.split_once(HANDLE_SEPARATOR) {
                // PA.f104: derived fresh from the turn's own record right
                // here, on every actor call, rather than snapshotted once
                // before this loop began — an earlier call in this same
                // round's own call order (including a dispatch this round
                // already executed) must make its subject's actor tools
                // valid on the first pass exactly as it does on a resume,
                // since a resume recomputes this same derivation from the
                // same persisted record and would otherwise disagree.
                let dispatched = turn.dispatched_subjects();
                let actor = resolve_handle(handle, player.as_ref(), &dispatched);
                let Some(actor) = actor else {
                    record_call(
                        turn,
                        call_id,
                        round,
                        this_slot,
                        None,
                        Some(format!("`{handle}` is not the player or a subject dispatched this turn")),
                    );
                    continue;
                };
                let (outcome, result) = self
                    .execute_actor_call(
                        turn, call_id, round, this_slot, actor, name, arguments, &snapshot, principal,
                    )
                    .await?;
                let fault = matches!(outcome, RoundOutcome::Poisoned).then(|| result.clone());
                record_call(turn, call_id, round, this_slot, None, Some(result));
                self.persist(turn).await?;
                if let Some(detail) = fault {
                    self.close_with_fault(turn, detail).await?;
                    return Ok(RoundOutcome::Poisoned);
                }
                continue;
            }

            record_call(
                turn,
                call_id,
                round,
                this_slot,
                None,
                Some(format!("`{name}` is not a tool this table offers")),
            );
        }

        Ok(RoundOutcome::Continue)
    }

    /// One maximal run of consecutive authoring calls (PA.f54): inside one
    /// round, a run of authoring calls with no actor call or table tool
    /// between them decodes as one shared patch, through the library's own
    /// `decode_authoring_calls`, and commits atomically under one command id
    /// derived from the run's own first (anchor) slot. A draft handle a
    /// call in the run declares resolves against a later call in the same
    /// run, because every call's item lands in that one shared `WorldPatch`
    /// before any of it is submitted.
    ///
    /// PA.f61: a run whose shared patch would rewrite an existing subject's
    /// persona material is refused whole — `WorldPatch::sets_persona_of_existing_subject`
    /// is the one read-only query the library exposes for this, because
    /// `WorldPatch`'s own fields are `pub(crate)` and Dungeon may not parse
    /// its JSON to decide the question itself. `set_persona_material` still
    /// stands for a subject this same run's own `declare_subject` call
    /// declares (a `Ref::Draft`, not `Ref::Existing`).
    async fn commit_authoring_run(
        &self,
        turn: &mut PlayTurn,
        round: usize,
        run: &[(usize, String, String, String)],
        snapshot: &WorldSnapshot,
    ) -> Result<Vec<(usize, String, RoundOutcome, String)>, PlayError> {
        let anchor_slot = run[0].0;
        let anchor_call_id = run[0].1.clone();
        let recorded = turn.call_record(round, anchor_slot).and_then(|call| call.body.clone());
        let (patch, batch) = match recorded {
            Some(RecordedCall::Authoring(patch)) => (patch, None),
            _ => {
                let calls: Vec<(&str, &str)> =
                    run.iter().map(|(_, _, name, arguments)| (name.as_str(), arguments.as_str())).collect();
                let decoded = match decode_authoring_calls(&calls) {
                    Ok(decoded) => decoded,
                    Err(detail) => return Ok(refuse_run(run, format!("refused: {detail}"))),
                };
                if decoded.patch.sets_persona_of_existing_subject() {
                    return Ok(refuse_run(
                        run,
                        "refused: set_persona_material may not target an existing subject; \
                         only a subject this same run declares"
                            .to_owned(),
                    ));
                }
                record_call(
                    turn,
                    &anchor_call_id,
                    round,
                    anchor_slot,
                    Some(RecordedCall::Authoring(decoded.patch.clone())),
                    None,
                );
                self.persist(turn).await?;
                let patch = decoded.patch.clone();
                (patch, Some(decoded))
            }
        };
        let command_id = derived_command_id(&turn.turn_id, round, anchor_slot);
        self.fire_before_submit_hook().await;
        let (outcome, result) = match self.play.submit_patch(command_id, patch).await {
            Ok(_receipt) => (RoundOutcome::Continue, "applied".to_owned()),
            Err(MailboxError::Kernel(KernelError::Invariant(detail))) => (RoundOutcome::Poisoned, detail),
            Err(MailboxError::Kernel(kernel_error @ KernelError::PatchRejected(_))) => {
                // PA.f54: the batch's own site map names the exact call a
                // mismatch belongs to, when one decoded this call (a resumed
                // resubmission has no fresh `DecodedBatch` and falls back to
                // `describe_refusal`'s own raw-index wording).
                let text = describe_refusal(snapshot, None, None, batch.as_ref(), &kernel_error);
                (RoundOutcome::Continue, format!("refused: {text}"))
            }
            Err(error) => (RoundOutcome::Continue, format!("refused: {error}")),
        };
        Ok(run
            .iter()
            .map(|(slot, call_id, _, _)| (*slot, call_id.clone(), outcome, result.clone()))
            .collect())
    }

    /// Persists `turn` to the store: the one durable-write door every
    /// `execute_*`/`commit_*` method uses to record a call's body *before*
    /// attempting its submission (PA-Q8), so a crash between the two leaves
    /// the exact submitted body on disk for `execute_round` to resubmit on
    /// resume.
    async fn persist(&self, turn: &PlayTurn) -> Result<(), PlayError> {
        self.store.lock().await.commit(turn.clone())?;
        Ok(())
    }

    async fn execute_advance_time(
        &self,
        turn: &mut PlayTurn,
        call_id: &str,
        round: usize,
        slot: usize,
        arguments: &str,
    ) -> Result<(RoundOutcome, String), PlayError> {
        let recorded = turn.call_record(round, slot).and_then(|call| call.body.clone());
        let minutes = match recorded {
            Some(RecordedCall::AdvanceTime(minutes)) => minutes,
            _ => match parse_minutes(arguments) {
                Some(minutes) => {
                    record_call(turn, call_id, round, slot, Some(RecordedCall::AdvanceTime(minutes)), None);
                    self.persist(turn).await?;
                    minutes
                }
                None => {
                    return Ok((
                        RoundOutcome::Continue,
                        "refused: minutes must be a positive integer within bounds".to_owned(),
                    ));
                }
            },
        };
        let command_id = derived_command_id(&turn.turn_id, round, slot);
        self.fire_before_submit_hook().await;
        match self.play.advance_time(command_id, minutes).await {
            Ok(_) => Ok((RoundOutcome::Continue, "the clock advanced".to_owned())),
            Err(MailboxError::Kernel(KernelError::Invariant(detail))) => Ok((RoundOutcome::Poisoned, detail)),
            Err(error) => Ok((RoundOutcome::Continue, format!("refused: {error}"))),
        }
    }

    async fn execute_actor_call(
        &self,
        turn: &mut PlayTurn,
        call_id: &str,
        round: usize,
        slot: usize,
        actor: Actor,
        name: &str,
        arguments: &str,
        snapshot: &WorldSnapshot,
        principal: &VerifiedPrincipalEvidence,
    ) -> Result<(RoundOutcome, String), PlayError> {
        let recorded = turn.call_record(round, slot).and_then(|call| call.body.clone());
        let (opportunity, invocation) = match recorded {
            Some(RecordedCall::PlayerAct(opportunity, invocation)) => (opportunity, invocation),
            Some(RecordedCall::PersonaAct(_, opportunity, invocation)) => (opportunity, invocation),
            _ => {
                // The real handle prefix `round_tools` handed `actor_tools`
                // when it generated this actor's own tools, paired with the
                // full, unstripped call `name`, so `decode_actor_call`'s own
                // prefix strip is the one and only place that decides what
                // the prefix removes.
                let prefix = actor_prefix(actor.subject);
                let (opportunity, invocation) =
                    match decode_actor_call(snapshot, actor.subject, &prefix, name, arguments) {
                        Ok(decoded) => decoded,
                        Err(detail) => {
                            return Ok((RoundOutcome::Continue, format!("refused: {detail}")));
                        }
                    };
                if !span_is_exact(&invocation, &actor, turn) {
                    return Ok((
                        RoundOutcome::Continue,
                        "refused: the quoted text is not an exact quote from the acting subject's own prose"
                            .to_owned(),
                    ));
                }
                let body = if actor.is_player {
                    RecordedCall::PlayerAct(opportunity.clone(), invocation.clone())
                } else {
                    RecordedCall::PersonaAct(actor.subject, opportunity.clone(), invocation.clone())
                };
                record_call(turn, call_id, round, slot, Some(body), None);
                self.persist(turn).await?;
                (opportunity, invocation)
            }
        };

        let command_id = derived_command_id(&turn.turn_id, round, slot);
        self.fire_before_submit_hook().await;
        let submission = if actor.is_player {
            self.world
                .submit_principal(
                    PrincipalCommandIntent {
                        id: command_id,
                        world_id: opportunity.world_id,
                        expected_revision: 0,
                        body: CommandBody::ExerciseDecision {
                            opportunity: opportunity.clone(),
                            invocation: invocation.clone(),
                        },
                    },
                    principal,
                )
                .await
        } else {
            ControllerPort::new(self.world.clone())
                .submit_controller(command_id, &opportunity, invocation.clone())
                .await
        };

        match submission {
            Ok(_) => Ok((RoundOutcome::Continue, "applied".to_owned())),
            Err(MailboxError::Kernel(KernelError::ActionRejected(mismatches))) => {
                // `entry`, `invocation`, and a run's site map are not
                // threaded through this call site yet; Cut 8b is the one
                // that wires this refusal path to the new parameters (and,
                // per PA.f57, to `describe_refusal_to_actor` for what the
                // player specifically sees). Passing `None` for all three
                // keeps this call's behaviour exactly what it was.
                let text =
                    describe_refusal(snapshot, None, None, None, &KernelError::ActionRejected(mismatches));
                if actor.is_player {
                    turn.refusal = Some(text.clone());
                }
                Ok((RoundOutcome::Continue, format!("refused: {text}")))
            }
            Err(MailboxError::Kernel(KernelError::Invariant(detail))) => Ok((RoundOutcome::Poisoned, detail)),
            Err(error) => Ok((RoundOutcome::Continue, format!("refused: {error}"))),
        }
    }

    async fn execute_dispatch(
        &self,
        turn: &mut PlayTurn,
        round: usize,
        call_slot: usize,
        subjects: &[SubjectId],
    ) -> Result<(RoundOutcome, String), PlayError> {
        let mut summary = Vec::new();
        for subject in subjects {
            if turn.persona_turns.iter().any(|(id, _)| id == subject) {
                summary.push(format!("{subject:?} already acted"));
                continue;
            }
            let snapshot = match self.play.snapshot().await {
                Ok(snapshot) => snapshot,
                Err(error) => {
                    // PA.f85/PA.f93: a snapshot failure mid-dispatch still
                    // closes the turn with the fault recorded, rather than
                    // leaving it `Running` behind a `?`.
                    self.close_with_fault(turn, error.to_string()).await?;
                    return Ok((RoundOutcome::Closed, summary.join("; ")));
                }
            };
            let Some(opportunity) = snapshot
                .opportunities
                .iter()
                .find(|opportunity| {
                    opportunity.scope.subject_id == *subject
                        && opportunity.controller_mode == ControllerMode::NarrativePersona
                })
                .cloned()
            else {
                summary.push(format!("{subject:?} holds no live Persona opportunity"));
                continue;
            };
            let permit = self
                .permits
                .clone()
                .acquire_owned()
                .await
                .expect("permit semaphore is never closed");
            // Derived from the dispatch call's own slot and the subject, not
            // from the subject's position in this call's own JSON array
            // (PA.f66d): two subjects named by one dispatch call, or the
            // same subject named by two dispatch calls in one round, must
            // never collide, in whatever order either call lists them.
            let command_id = dispatch_command_id(&turn.turn_id, round, call_slot, *subject);
            let result = self.personas.turn(command_id, &opportunity).await;
            drop(permit);
            match result {
                Ok(persona_turn) => {
                    let label = subject_label(&snapshot, *subject);
                    // PA.f64: the dispatched Persona's own prose reaches the
                    // agent as this call's own tool result, not a bare
                    // "acted" acknowledgement — `rebuild_conversation` is
                    // already generic over every call's recorded `result`,
                    // so recording the prose here is the whole fix.
                    let prose = persona_turn.source_prose().to_owned();
                    summary.push(format!("{label}: {prose}"));
                    turn.persona_turns.push((*subject, persona_turn));
                }
                Err(error) => {
                    let label = subject_label(&snapshot, *subject);
                    summary.push(format!("{label} could not act: {error}"));
                }
            }
        }
        Ok((RoundOutcome::Continue, summary.join("; ")))
    }

    /// Narrates the turn's own close to the player. Takes no snapshot or
    /// principal of its own (PA.f95: neither parameter was ever read) —
    /// it always re-fetches its own fresh snapshot below, since the round's
    /// own calls may have committed since any snapshot a caller might have
    /// held.
    async fn close_turn(&self, turn: &PlayTurn) -> Result<String, PlayError> {
        // A fresh snapshot, not the round's own: this round's calls may have
        // committed since it was taken, and `PersonaLane::narrate` refuses
        // an opportunity that is not the kernel's own currently issued copy.
        let snapshot = self.play.snapshot().await?;
        let Some(player) = player_subject(&snapshot) else {
            return Ok(String::new());
        };
        let Some(opportunity) = snapshot
            .opportunities
            .iter()
            .find(|opportunity| {
                opportunity.scope.subject_id == player.id
                    && opportunity.controller_mode == ControllerMode::Human
            })
            .cloned()
        else {
            return Ok(String::new());
        };
        let command_id = derived_command_id(&turn.turn_id, turn.rounds.len(), usize::MAX);
        Ok(self.personas.narrate(command_id, &opportunity).await?)
    }
}

struct Actor {
    subject: SubjectId,
    is_player: bool,
}

fn resolve_handle(handle: &str, player: Option<&SubjectRow>, dispatched: &[SubjectId]) -> Option<Actor> {
    if let Some(player) = player
        && handle_for(player.id) == handle
    {
        return Some(Actor {
            subject: player.id,
            is_player: true,
        });
    }
    for subject in dispatched {
        if handle_for(*subject) == handle {
            return Some(Actor {
                subject: *subject,
                is_player: false,
            });
        }
    }
    None
}

struct SubjectRow {
    id: SubjectId,
}

fn player_subject(snapshot: &WorldSnapshot) -> Option<SubjectRow> {
    snapshot
        .subjects
        .iter()
        .find(|subject| subject.controller_mode == Some(ControllerMode::Human))
        .map(|subject| SubjectRow { id: subject.id })
}

fn subject_label(snapshot: &WorldSnapshot, subject: SubjectId) -> String {
    snapshot
        .subjects
        .iter()
        .find(|row| row.id == subject)
        .map_or_else(|| "an unknown subject".to_owned(), |row| row.label.clone())
}

/// A subject's own short handle: the first 8 hex digits of its id
/// (PA.f97). Actor tool names are `<handle>__<kind>`, and a kind may run up
/// to 48 chars, so the full 36-char uuid text a handle used to carry could
/// reach 86 chars against a common 64-char function-name limit; 8 hex
/// digits plus the `__` separator keeps every name well under that limit
/// (`the_longest_possible_actor_tool_name_is_64_chars_or_fewer`) while still
/// reading as the same bracketed id prefix `table_view` prints (the same
/// debug-paren strip `table.rs`'s own `id_text` uses, before truncating).
/// Collision between two subjects sharing a handle is astronomically
/// unlikely but not impossible; `round_tools` refuses the whole round as a
/// turn fault rather than silently addressing the wrong subject
/// (`find_handle_collision`).
fn handle_for(id: SubjectId) -> String {
    let text = format!("{id:?}");
    let inner = match (text.find('('), text.rfind(')')) {
        (Some(open), Some(close)) if open < close => &text[open + 1..close],
        _ => text.as_str(),
    };
    inner.chars().filter(char::is_ascii_hexdigit).take(8).collect()
}

/// Appends one actor tool's own acting subject's label and full canonical id
/// to its description (PA.f98): `dispatch` takes the id exactly as
/// `table_view` prints it, never the short `<handle>__` prefix these tools'
/// own names carry, so every actor tool now names the id `dispatch` actually
/// expects, right where the agent reads it. `actor_tools` (`table.rs`) knows
/// nothing of a subject's label; annotating here, over the definitions it
/// already returned, is the smallest fix that does not teach the library
/// vocabulary a Dungeon-only presentation concern. The id itself comes from
/// the library's own `id_text` (PA.f113) — Dungeon no longer keeps its own
/// copy of the debug-paren strip `table_view` uses to print it.
fn annotate_actor_tools(tools: &mut [CodexToolDefinition], snapshot: &WorldSnapshot, subject: SubjectId) {
    let label = subject_label(snapshot, subject);
    let id = id_text(subject);
    for tool in tools {
        tool.description = format!("{} Acting subject: {label} [{id}].", tool.description);
    }
}

/// The first colliding handle among `handles`, if any (PA.f97): two entries
/// naming the *same* handle for the *same* subject is not a collision (a
/// subject may legitimately appear once as the player and, in principle,
/// once more via `dispatched`); two entries naming the same handle for
/// *different* subjects is. Pulled apart from `handle_for` itself so the
/// detection algorithm is directly testable without needing two real
/// subject ids that actually collide — vanishingly unlikely to construct
/// from genuine, randomly issued ids, and `SubjectId` mints nothing a
/// consumer crate can construct by hand.
fn find_handle_collision(handles: impl Iterator<Item = (String, SubjectId)>) -> Option<String> {
    let mut seen: std::collections::HashMap<String, SubjectId> = std::collections::HashMap::new();
    for (handle, subject) in handles {
        match seen.get(&handle) {
            Some(existing) if *existing != subject => return Some(handle),
            _ => {
                seen.insert(handle, subject);
            }
        }
    }
    None
}

/// The tool-name prefix one subject's own actor tools carry: exactly what
/// `round_tools` hands `actor_tools` to generate them, and what
/// `execute_actor_call` hands `decode_actor_call` to parse them back — one
/// prefix, shared, so generation and decode can never drift apart.
fn actor_prefix(subject: SubjectId) -> String {
    format!("{}{HANDLE_SEPARATOR}", handle_for(subject))
}

fn control_tools() -> Vec<CodexToolDefinition> {
    vec![
        CodexToolDefinition {
            name: DISPATCH_TOOL.into(),
            description: "Dispatch one or more subjects to act this round, each named by its full id \
exactly as table_view prints it in brackets — never the short <handle>__ prefix an actor tool's own \
name carries. An id naming no dispatchable subject is refused, quoting the text given."
                .into(),
            parameters_json: r#"{"type":"object","properties":{"subjects":{"type":"array","items":{"type":"string"}}},"required":["subjects"]}"#.into(),
        },
        CodexToolDefinition {
            name: ADVANCE_TIME_TOOL.into(),
            description: "Advance the world clock by a whole number of fictional minutes.".into(),
            parameters_json: r#"{"type":"object","properties":{"minutes":{"type":"integer","minimum":1}},"required":["minutes"]}"#.into(),
        },
        CodexToolDefinition {
            name: ASK_PLAYER_TOOL.into(),
            description: "Stop the turn and ask the player a question.".into(),
            parameters_json: r#"{"type":"object","properties":{"question":{"type":"string"}},"required":["question"]}"#.into(),
        },
        CodexToolDefinition {
            name: END_TURN_TOOL.into(),
            description: "End the turn: its consequences are settled.".into(),
            parameters_json: r#"{"type":"object","properties":{},"required":[]}"#.into(),
        },
    ]
}

/// Resolves `dispatch`'s own `subjects` argument (PA.f98, PA.f114): each
/// entry is the *full* id `table_view` prints in brackets, matched through
/// the library's own `id_text_matches` — never the short `<handle>__`
/// actor-tool prefix, which exists only to keep a generated tool name under a
/// common function-name length limit and was never meant to be an id a caller
/// supplies back.
///
/// `PLAY_INSTRUCTIONS` promises dispatch refuses bad input rather than
/// silently doing nothing with it, so every shape of bad input returns a
/// refusal in `refusals` that quotes what was given, instead of being
/// dropped: a body that is not valid JSON, a body that is not a JSON object,
/// a missing `subjects` key, a `subjects` that is not an array, a non-string
/// entry, and an entry naming no subject in `snapshot`.
fn parse_dispatch(arguments: &str, snapshot: &WorldSnapshot) -> (Vec<SubjectId>, Vec<String>) {
    let value = match serde_json::from_str::<Value>(arguments) {
        Ok(value) => value,
        Err(_) => {
            return (
                Vec::new(),
                vec![format!("dispatch's arguments are not valid JSON: `{arguments}`")],
            );
        }
    };
    let Value::Object(fields) = value else {
        return (
            Vec::new(),
            vec![format!("dispatch's arguments are not a JSON object: `{arguments}`")],
        );
    };
    let Some(subjects_value) = fields.get("subjects") else {
        return (
            Vec::new(),
            vec![format!("dispatch's arguments have no `subjects` key: `{arguments}`")],
        );
    };
    let Value::Array(items) = subjects_value else {
        return (
            Vec::new(),
            vec![format!("dispatch's `subjects` is not an array: `{subjects_value}`")],
        );
    };
    let mut subjects = Vec::new();
    let mut refusals = Vec::new();
    for item in items {
        match item.as_str() {
            Some(text) => match snapshot.subjects.iter().find(|subject| id_text_matches(subject.id, text)) {
                Some(subject) => subjects.push(subject.id),
                None => refusals.push(format!("`{text}` names no dispatchable subject")),
            },
            None => refusals.push(format!("`{item}` in `subjects` is not a string")),
        }
    }
    (subjects, refusals)
}

fn parse_ask_player(arguments: &str) -> String {
    serde_json::from_str::<Value>(arguments)
        .ok()
        .and_then(|value| value.get("question").and_then(Value::as_str).map(str::to_owned))
        .unwrap_or_default()
}

fn parse_minutes(arguments: &str) -> Option<TickMinutes> {
    let value = serde_json::from_str::<Value>(arguments).ok()?;
    let minutes = value.get("minutes")?.as_u64()?;
    TickMinutes::new(u32::try_from(minutes).ok()?)
}

/// The derived command id for one dispatched Persona's own turn (PA.f66d):
/// hashed from the dispatch tool call's own round and slot plus the
/// subject's own handle, never from the subject's position in that call's
/// own JSON `subjects` array. Two subjects one dispatch call names, or the
/// same subject two dispatch calls in one round name, must never collide —
/// the old `dispatch_slot` collided exactly there, since it read only a
/// position within one call's own list and dropped the call's own identity
/// entirely.
fn dispatch_command_id(turn_id: &str, round: usize, call_slot: usize, subject: SubjectId) -> CommandId {
    dispatch_command_id_for_handle(turn_id, round, call_slot, &handle_for(subject))
}

/// `dispatch_command_id`'s own hash, taking a bare handle so it is directly
/// testable without minting a `SubjectId` (Dungeon holds no public
/// constructor for one).
fn dispatch_command_id_for_handle(turn_id: &str, round: usize, call_slot: usize, handle: &str) -> CommandId {
    let mut hasher = Sha256::new();
    hasher.update(b"ghostlight.dungeon.play.dispatch.v1\n");
    hasher.update(turn_id.as_bytes());
    hasher.update(b"\n");
    hasher.update(round.to_le_bytes());
    hasher.update(call_slot.to_le_bytes());
    hasher.update(b"\n");
    hasher.update(handle.as_bytes());
    let digest = hasher.finalize();
    let uuid = uuid::Uuid::from_slice(&digest[..16]).expect("sha256 digest is at least 16 bytes");
    CommandId::parse_uuid(&uuid.to_string()).expect("a formatted uuid always parses")
}

/// `PlayTable::retry_delay`'s own capped exponential backoff formula
/// (PA.f102): `BASE * 2^attempt` milliseconds, capped at `CAP`, with
/// `attempt` clamped before the shift so a very large attempt count can
/// never overflow or panic. A free function, always compiled — never hidden
/// behind `#[cfg(test)]` — so it is directly unit-testable in every build.
fn backoff_delay(attempt: usize) -> Duration {
    const BASE: u64 = 100;
    const CAP: u64 = 5_000;
    Duration::from_millis(BASE.saturating_mul(1u64 << attempt.min(6)).min(CAP))
}

/// A round is incomplete when it holds a tool call this table has not yet
/// resolved to a result.
fn round_incomplete(turn: &PlayTurn, round: usize) -> bool {
    let Some(output) = turn.rounds.get(round) else {
        return false;
    };
    let mut slot = 0usize;
    for event in output.events() {
        if !matches!(event, InferenceEvent::ToolCall { .. }) {
            continue;
        }
        let this_slot = slot;
        slot += 1;
        if !turn
            .call_record(round, this_slot)
            .is_some_and(|call| call.result.is_some())
        {
            return true;
        }
    }
    false
}

fn find_ask_player_slot(turn: &PlayTurn, round: usize) -> Option<usize> {
    turn.calls
        .iter()
        .find(|call| {
            call.round == round && matches!(call.body, Some(RecordedCall::AskPlayer(_))) && call.result.is_none()
        })
        .map(|call| call.slot)
}

/// One refusal, attributed to every call in a run (PA.f54): the run commits
/// atomically, so a decode-time refusal — before any patch is even
/// assembled — is every one of the run's calls own result alike, not just
/// the anchor's.
fn refuse_run(run: &[(usize, String, String, String)], detail: String) -> Vec<(usize, String, RoundOutcome, String)> {
    run.iter()
        .map(|(slot, call_id, _, _)| (*slot, call_id.clone(), RoundOutcome::Continue, detail.clone()))
        .collect()
}

fn record_call(
    turn: &mut PlayTurn,
    call_id: &str,
    round: usize,
    slot: usize,
    body: Option<RecordedCall>,
    result: Option<String>,
) {
    if let Some(existing) = turn.call_record_mut(round, slot) {
        if body.is_some() {
            existing.body = body;
        }
        if result.is_some() {
            existing.result = result;
        }
        return;
    }
    turn.calls.push(CallRecord {
        call_id: call_id.to_owned(),
        round,
        slot,
        body,
        result,
    });
}

/// The connector posture: `[UserText(opening_prompt), ...]` replaying every
/// fully-resolved round before `round` as `AssistantText`/`ToolCall`/`ToolResult`
/// items, exactly as `controllers.rs`'s own multi-round lanes rebuild their
/// conversation from `completed` outputs.
fn rebuild_conversation(turn: &PlayTurn, round: usize) -> Vec<CodexInputItem> {
    let mut conversation = vec![CodexInputItem::UserText {
        text: turn.opening_prompt.clone(),
    }];
    for (index, output) in turn.rounds.iter().enumerate() {
        if index >= round {
            break;
        }
        let mut slot = 0usize;
        for event in output.events() {
            match event {
                InferenceEvent::Text(text) => {
                    if !text.is_empty() {
                        conversation.push(CodexInputItem::AssistantText { text: text.clone() });
                    }
                }
                InferenceEvent::ToolCall {
                    call_id,
                    name,
                    arguments,
                } => {
                    let this_slot = slot;
                    slot += 1;
                    conversation.push(CodexInputItem::ToolCall {
                        call_id: call_id.clone(),
                        name: name.clone(),
                        arguments: arguments.clone(),
                    });
                    let output_text = turn
                        .call_record(index, this_slot)
                        .and_then(|call| call.result.clone())
                        .unwrap_or_default();
                    conversation.push(CodexInputItem::ToolResult {
                        call_id: call_id.clone(),
                        output: output_text,
                    });
                }
            }
        }
    }
    conversation
}

/// Whether an actor call's `text`/`display` are exact quotes from the
/// acting subject's own preserved prose: the player's submitted texts, or a
/// dispatched Persona's own recorded turn (PA-Q8 invariant 4). A player's
/// display need not be spoken; an empty invocation (neither) is exact by
/// vacuity.
fn span_is_exact(invocation: &DecisionInvocation, actor: &Actor, turn: &PlayTurn) -> bool {
    let sources: Vec<&str> = if actor.is_player {
        turn.player_prose.iter().map(String::as_str).collect()
    } else {
        turn.persona_turns
            .iter()
            .filter(|(id, _)| *id == actor.subject)
            .map(|(_, persona_turn)| persona_turn.source_prose())
            .collect()
    };
    let quotes = [
        invocation.speech.as_ref().map(Statement::as_str),
        invocation.display.as_ref().map(Statement::as_str),
    ];
    for quote in quotes.into_iter().flatten() {
        if !sources.iter().any(|source| SourceSpan::locate(source, quote).is_some()) {
            return false;
        }
    }
    true
}

/// sha256 over the turn id, round, and slot; the first 16 bytes read back as
/// a UUID. Deterministic over `(turn_id, round, slot)`, so a resumed call
/// re-derives the identical command id the kernel's idempotency ledger
/// already knows (`AlreadyApplied`), never a fresh one.
fn derived_command_id(turn_id: &str, round: usize, slot: usize) -> CommandId {
    let mut hasher = Sha256::new();
    hasher.update(b"ghostlight.dungeon.play.command.v1\n");
    hasher.update(turn_id.as_bytes());
    hasher.update(b"\n");
    hasher.update(round.to_le_bytes());
    hasher.update(slot.to_le_bytes());
    let digest = hasher.finalize();
    let uuid = uuid::Uuid::from_slice(&digest[..16]).expect("sha256 digest is at least 16 bytes");
    CommandId::parse_uuid(&uuid.to_string()).expect("a formatted uuid always parses")
}

#[cfg(test)]
mod tests {
    use super::*;
    use ghostlight::{
        CreateWorldIntent, InferenceFault, Lens, LensWeights, PreparedInference,
    };
    use ghostlight_persona_projection::PersonaTurnBinding;
    use std::{
        collections::{BTreeMap, VecDeque},
        sync::Mutex as StdMutex,
    };

    /// Every `.rs` file under `src`, with everything a `#[cfg(test)]` item
    /// removed, the same extent-by-indent scan `app_session.rs`'s own
    /// forbidden-writer test uses.
    fn production_sources() -> Vec<(std::path::PathBuf, String)> {
        fn indent(line: &str) -> usize {
            line.len() - line.trim_start().len()
        }
        fn strip_cfg_test(text: &str) -> String {
            let lines: Vec<&str> = text.lines().collect();
            let mut kept: Vec<&str> = Vec::new();
            let mut index = 0;
            while index < lines.len() {
                let line = lines[index];
                if line.trim() != "#[cfg(test)]" {
                    kept.push(line);
                    index += 1;
                    continue;
                }
                let guard = indent(line);
                index += 1;
                let mut opened = lines
                    .get(index)
                    .is_some_and(|next| next.trim_end().ends_with('{'));
                while index < lines.len() {
                    let body = lines[index];
                    index += 1;
                    let trimmed = body.trim();
                    if trimmed.is_empty() {
                        continue;
                    }
                    if opened {
                        if indent(body) == guard && (trimmed == "}" || trimmed == "};" || trimmed == "},") {
                            break;
                        }
                    } else if indent(body) == guard && (trimmed.ends_with(';') || trimmed.ends_with(',')) {
                        break;
                    } else if indent(body) == guard && trimmed.ends_with('{') {
                        opened = true;
                    }
                }
            }
            kept.join("\n")
        }
        let mut sources = Vec::new();
        let mut pending = vec![std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src")];
        while let Some(directory) = pending.pop() {
            for entry in std::fs::read_dir(&directory).expect("the source tree reads") {
                let path = entry.expect("a readable entry").path();
                if path.is_dir() {
                    pending.push(path);
                    continue;
                }
                if path.extension().and_then(|value| value.to_str()) != Some("rs") {
                    continue;
                }
                let text = std::fs::read_to_string(&path)
                    .expect("the source reads")
                    .replace("\r\n", "\n");
                sources.push((path, strip_cfg_test(&text)));
            }
        }
        sources
    }

    /// Pin: Dungeon never builds a `ComponentOp`, `Declaration`, `RoleBinding`,
    /// or `ProposedEffect` by hand; everything goes through `table.rs`.
    #[test]
    fn soul_no_forbidden_writer_name_appears_in_dungeons_own_source() {
        for forbidden in ["ComponentOp", "Declaration", "RoleBinding", "ProposedEffect"] {
            for (path, source) in production_sources() {
                assert!(
                    !source.contains(forbidden),
                    "{} names `{forbidden}` directly; Dungeon must compose through table.rs",
                    path.display()
                );
            }
        }
    }

    /// Pin: `PlayPort`'s constructor is minted in exactly one production
    /// site, matching the library's own doc comment on `PlayPort`. The
    /// needle is assembled from halves so this assertion's own source, and
    /// this file's doc comments about the rule, are never themselves a
    /// match — the same precedent `app_session.rs`'s own single-minter test
    /// uses for `VerifiedPrincipalEvidence`.
    #[test]
    fn soul_exactly_one_production_site_mints_play_port() {
        let needle = format!("{}::{}(", "PlayPort", "new");
        let mut sites = Vec::new();
        for (path, source) in production_sources() {
            for _ in source.match_indices(needle.as_str()) {
                sites.push(path.clone());
            }
        }
        assert_eq!(sites.len(), 1, "the play port constructor appears in: {sites:?}");
        assert!(sites[0].ends_with("play.rs"));
    }

    /// Rule: `PLAY_TOOLS` is a real subset of the library's own catalog.
    #[test]
    fn play_tools_are_all_real_authoring_tools() {
        authoring_tools(PLAY_TOOLS).expect("every PLAY_TOOLS name is a known authoring tool");
        assert!(!PLAY_TOOLS.contains(&"admit"), "admit is not a play tool");
    }

    fn test_turn_id(n: u128) -> String {
        uuid::Uuid::from_u128(n).to_string()
    }

    fn far_future() -> chrono::DateTime<Utc> {
        Utc::now() + chrono::Duration::hours(4)
    }

    struct ScriptedPort {
        queue: StdMutex<VecDeque<Result<InferenceOutput, String>>>,
        seen: StdMutex<Vec<InferenceRequest>>,
    }

    impl ScriptedPort {
        fn new(outputs: Vec<InferenceOutput>) -> Arc<Self> {
            Arc::new(Self {
                queue: StdMutex::new(outputs.into_iter().map(Ok).collect()),
                seen: StdMutex::new(Vec::new()),
            })
        }

        fn seen_requests(&self) -> Vec<InferenceRequest> {
            self.seen.lock().unwrap().clone()
        }
    }

    #[async_trait::async_trait]
    impl InferencePort for ScriptedPort {
        fn prepare(&self, request: InferenceRequest) -> Result<PreparedInference, InferenceFault> {
            self.seen.lock().unwrap().push(request.clone());
            PreparedInference::prepare("play-test-runtime", far_future_unix_ms(), request)
        }

        async fn infer(&self, _request: PreparedInference) -> Result<InferenceOutput, InferenceFault> {
            match self.queue.lock().unwrap().pop_front() {
                Some(Ok(output)) => Ok(output),
                Some(Err(detail)) => Err(InferenceFault::recovery_required(detail)),
                None => Err(InferenceFault::recovery_required("scripted port exhausted")),
            }
        }
    }

    fn far_future_unix_ms() -> u64 {
        u64::try_from(far_future().timestamp_millis()).unwrap_or(u64::MAX)
    }

    fn output(receipt: &str, events: Vec<InferenceEvent>) -> InferenceOutput {
        InferenceOutput::new(events, receipt.to_owned())
    }

    fn text_event(text: &str) -> InferenceEvent {
        InferenceEvent::Text(text.to_owned())
    }

    fn call_event(call_id: &str, name: &str, arguments: serde_json::Value) -> InferenceEvent {
        InferenceEvent::ToolCall {
            call_id: call_id.to_owned(),
            name: name.to_owned(),
            arguments: arguments.to_string(),
        }
    }

    struct WorldFixture {
        _directory: tempfile::TempDir,
        world: WorldMailbox,
        principal: VerifiedPrincipalEvidence,
    }

    /// A genesis world, activated: one Human "first-person" subject and,
    /// when `persona_label` is given, one NarrativePersona subject, both
    /// standing in the commons with the kernel's own `speak` grant. Built
    /// entirely through `WorldMailbox`'s public door — the same one Cut 8's
    /// own play agent uses — so no test fixture reaches into the kernel's
    /// private typed vocabulary.
    async fn play_world(persona_label: Option<&str>, account_hash: &str) -> WorldFixture {
        let directory = tempfile::tempdir().unwrap();
        let (world, _owner) = WorldMailbox::open(directory.path().join("world.cc")).unwrap();
        let principal = VerifiedPrincipalEvidence::new(account_hash, far_future());
        let receipt = world
            .create(
                CreateWorldIntent {
                    id: CommandId::new(),
                    title: "Fixture World".into(),
                    brief: "A small table for Cut 8a's own tests.".into(),
                    human_subject_label: "Player".into(),
                    narrative_persona_label: persona_label.map(str::to_owned),
                    operational_agent_label: None,
                    targets: BTreeMap::new(),
                    jurisdictions: Vec::new(),
                    lens_weights: LensWeights::new(BTreeMap::from([(Lens::Hearth, 1)])),
                },
                &principal,
            )
            .await
            .unwrap();
        let snapshot = world.snapshot().await.unwrap();
        world
            .submit_principal(
                PrincipalCommandIntent {
                    id: CommandId::new(),
                    world_id: receipt.world_id,
                    expected_revision: snapshot.revision,
                    body: CommandBody::ApproveDraft,
                },
                &principal,
            )
            .await
            .unwrap();
        let snapshot = world.snapshot().await.unwrap();
        world
            .submit_principal(
                PrincipalCommandIntent {
                    id: CommandId::new(),
                    world_id: receipt.world_id,
                    expected_revision: snapshot.revision,
                    body: CommandBody::ActivateWorld,
                },
                &principal,
            )
            .await
            .unwrap();
        WorldFixture {
            _directory: directory,
            world,
            principal,
        }
    }

    fn persona_model() -> Arc<dyn InferencePort> {
        ScriptedPort::new(Vec::new())
    }

    struct TableFixture {
        table: PlayTable,
        _directory: tempfile::TempDir,
    }

    fn table_with(world: &WorldMailbox, play_port: Arc<dyn InferencePort>) -> TableFixture {
        let directory = tempfile::tempdir().unwrap();
        let personas = PersonaLane::new(
            ControllerPort::new(world.clone()),
            persona_model(),
            "gpt-5.6-sol".into(),
            "gpt-5.6-sol".into(),
        )
        .unwrap();
        let table = PlayTable::new(
            world.clone(),
            personas,
            play_port,
            "gpt-5.6-terra".into(),
            Arc::new(Semaphore::new(2)),
            directory.path().join("play-turn-v1.cc"),
        )
        .unwrap();
        TableFixture {
            table,
            _directory: directory,
        }
    }

    fn player_id(snapshot: &WorldSnapshot) -> SubjectId {
        player_subject(snapshot).unwrap().id
    }

    fn persona_id(snapshot: &WorldSnapshot) -> SubjectId {
        snapshot
            .subjects
            .iter()
            .find(|subject| subject.controller_mode == Some(ControllerMode::NarrativePersona))
            .unwrap()
            .id
    }

    /// A subject's own full canonical id text, read out of a *real*
    /// `table_view` render rather than a test-local copy of its printer
    /// (PA.f113): a change to what `table_view` actually prints must break
    /// every test that reads an id through this helper, which a fourth
    /// hand-spelled debug-paren strip could not do — that copy passed every
    /// dispatch test while `table_view`'s own print regressed and every real
    /// dispatch an agent could make was refused. Distinct from `handle_for`'s
    /// short 8-hex-digit actor handle (PA.f97). A `Ref::Existing` JSON value
    /// needs the real id; only a `<handle>__<kind>` tool name needs the
    /// truncated handle.
    fn subject_id_text(snapshot: &WorldSnapshot, id: SubjectId) -> String {
        let label = snapshot
            .subjects
            .iter()
            .find(|subject| subject.id == id)
            .map_or_else(|| "an unknown subject".to_owned(), |subject| subject.label.clone());
        let view = table_view(snapshot);
        // The `Places:` section prints each place's occupants by the same
        // "label [id]" shape (PA.f74) before the `Subjects:` section proper
        // renders the subject's own canonical entry — the entry
        // `PLAY_INSTRUCTIONS` and `dispatch` actually mean. Search only from
        // the `Subjects:` header so a subject that also occupies a place
        // does not read its id back out of the wrong section.
        let subjects_section = view
            .find("\n  Subjects:")
            .map_or(view.as_str(), |at| &view[at..]);
        let marker = format!("{label} [");
        let start = subjects_section
            .find(&marker)
            .unwrap_or_else(|| panic!("{label} not found in table_view's Subjects section: {view}"))
            + marker.len();
        let end = subjects_section[start..]
            .find(']')
            .unwrap_or_else(|| panic!("unterminated id bracket after {label} in table_view"));
        subjects_section[start..start + end].to_owned()
    }

    #[tokio::test]
    async fn the_turn_closes_with_the_players_narration() {
        let fixture = play_world(None, "player-closes").await;
        let end = output("r0", vec![call_event("c0", END_TURN_TOOL, serde_json::json!({}))]);
        // The player's own narration draws one Projector inference.
        let projector = output("proj-0", vec![text_event("The room settles.")]);
        let personas = PersonaLane::new(
            ControllerPort::new(fixture.world.clone()),
            ScriptedPort::new(vec![projector]),
            "gpt-5.6-sol".into(),
            "gpt-5.6-sol".into(),
        )
        .unwrap();
        let directory = tempfile::tempdir().unwrap();
        let table = PlayTable::new(
            fixture.world.clone(),
            personas,
            ScriptedPort::new(vec![end]),
            "gpt-5.6-terra".into(),
            Arc::new(Semaphore::new(2)),
            directory.path().join("play-turn-v1.cc"),
        )
        .unwrap();

        table
            .run(&fixture.principal, test_turn_id(1), "I look around.".into())
            .await
            .unwrap();

        // PA.f69: the narration text itself, not merely its presence.
        let stored = table.store.lock().await;
        let turn = stored.current().unwrap();
        assert_eq!(turn.narration.as_deref(), Some("The room settles."));
    }

    #[tokio::test]
    async fn a_raw_op_is_journaled_as_play_and_never_as_an_actor() {
        let fixture = play_world(None, "player-raw-op").await;
        let declare = output(
            "r0",
            vec![
                call_event(
                    "c0",
                    "declare_resource",
                    serde_json::json!({"handle": "gold", "label": "Gold"}),
                ),
                call_event("c1", END_TURN_TOOL, serde_json::json!({})),
            ],
        );
        let personas = PersonaLane::new(
            ControllerPort::new(fixture.world.clone()),
            ScriptedPort::new(vec![output("proj-0", vec![text_event("Quiet.")])]),
            "gpt-5.6-sol".into(),
            "gpt-5.6-sol".into(),
        )
        .unwrap();
        let directory = tempfile::tempdir().unwrap();
        let table = PlayTable::new(
            fixture.world.clone(),
            personas,
            ScriptedPort::new(vec![declare]),
            "gpt-5.6-terra".into(),
            Arc::new(Semaphore::new(2)),
            directory.path().join("play-turn-v1.cc"),
        )
        .unwrap();
        table
            .run(&fixture.principal, test_turn_id(2), "Declare a resource.".into())
            .await
            .unwrap();

        // In Active phase, only the `Play` capability may admit an
        // unanswered declare with no `PatchAnswer` (`require_answer` in
        // `lib.rs`); every other caller is refused `AnswerRequired`. This
        // resource's presence in the next snapshot is therefore possible
        // only if the commit's caller was `System(Play)`.
        let snapshot = fixture.world.snapshot().await.unwrap();
        assert!(
            table_view(&snapshot).contains("Gold"),
            "the declared resource did not commit through the Play door"
        );
    }

    #[tokio::test]
    async fn an_act_for_an_undispatched_subject_is_refused() {
        let fixture = play_world(Some("Mara"), "player-undispatched").await;
        let before = fixture.world.snapshot().await.unwrap();
        let mara = handle_for(persona_id(&before));
        let round = output(
            "r0",
            vec![
                call_event(
                    "c0",
                    &format!("{mara}__speak"),
                    serde_json::json!({"text": "never dispatched"}),
                ),
                call_event("c1", END_TURN_TOOL, serde_json::json!({})),
            ],
        );
        let personas = PersonaLane::new(
            ControllerPort::new(fixture.world.clone()),
            ScriptedPort::new(vec![output("proj-0", vec![text_event("Silence.")])]),
            "gpt-5.6-sol".into(),
            "gpt-5.6-sol".into(),
        )
        .unwrap();
        let directory = tempfile::tempdir().unwrap();
        let table = PlayTable::new(
            fixture.world.clone(),
            personas,
            ScriptedPort::new(vec![round]),
            "gpt-5.6-terra".into(),
            Arc::new(Semaphore::new(2)),
            directory.path().join("play-turn-v1.cc"),
        )
        .unwrap();
        table
            .run(&fixture.principal, test_turn_id(3), "Hello?".into())
            .await
            .unwrap();

        // PA.f69 S2: `resolve_handle` refuses the undispatched handle before
        // any door is touched — asserted, not merely "no panic" — and
        // nothing about the world committed from it.
        let stored = table.store.lock().await;
        let turn = stored.current().unwrap();
        let result = turn
            .calls
            .iter()
            .find(|call| call.round == 0 && call.slot == 0)
            .and_then(|call| call.result.clone())
            .unwrap();
        assert!(
            result.contains("not the player or a subject dispatched this turn"),
            "{result}"
        );
        let after = fixture.world.snapshot().await.unwrap();
        assert_eq!(after.revision, before.revision, "an undispatched act must commit nothing");
    }

    #[tokio::test]
    async fn a_paraphrased_quote_is_refused() {
        let fixture = play_world(None, "player-paraphrase").await;
        let snapshot = fixture.world.snapshot().await.unwrap();
        let player = handle_for(player_id(&snapshot));
        let round = output(
            "r0",
            vec![
                call_event(
                    "c0",
                    &format!("{player}{HANDLE_SEPARATOR}speak"),
                    serde_json::json!({"text": "a whispered WARNING"}),
                ),
                call_event("c1", END_TURN_TOOL, serde_json::json!({})),
            ],
        );
        let personas = PersonaLane::new(
            ControllerPort::new(fixture.world.clone()),
            ScriptedPort::new(vec![output("proj-0", vec![text_event("Quiet.")])]),
            "gpt-5.6-sol".into(),
            "gpt-5.6-sol".into(),
        )
        .unwrap();
        let directory = tempfile::tempdir().unwrap();
        let table = PlayTable::new(
            fixture.world.clone(),
            personas,
            ScriptedPort::new(vec![round]),
            "gpt-5.6-terra".into(),
            Arc::new(Semaphore::new(2)),
            directory.path().join("play-turn-v1.cc"),
        )
        .unwrap();
        table
            .run(&fixture.principal, test_turn_id(100), "a whispered warning".into())
            .await
            .unwrap();

        let stored = table.store.lock().await;
        let turn = stored.current().unwrap();
        let result = turn
            .calls
            .iter()
            .find(|call| call.round == 0 && call.slot == 0)
            .and_then(|call| call.result.clone())
            .unwrap();
        assert!(result.contains("not an exact quote"), "{result}");
    }

    /// Mutation M8.1: if `span_is_exact` stopped checking a Persona's own
    /// quotes, this would no longer hold — the paraphrase would commit.
    #[tokio::test]
    async fn a_paraphrased_quote_is_refused_for_a_persona() {
        let fixture = play_world(Some("Mara"), "player-persona-paraphrase").await;
        let snapshot = fixture.world.snapshot().await.unwrap();
        let mara = handle_for(persona_id(&snapshot));
        let mara_id = subject_id_text(&snapshot, persona_id(&snapshot));
        let dispatch_round = output(
            "r0",
            vec![call_event(
                "c0",
                DISPATCH_TOOL,
                serde_json::json!({"subjects": [mara_id]}),
            )],
        );
        let speak_round = output(
            "r1",
            vec![
                call_event(
                    "c1",
                    &format!("{mara}{HANDLE_SEPARATOR}speak"),
                    serde_json::json!({"text": "I nod and SAY nothing"}),
                ),
                call_event("c2", END_TURN_TOOL, serde_json::json!({})),
            ],
        );
        let personas = PersonaLane::new(
            ControllerPort::new(fixture.world.clone()),
            ScriptedPort::new(vec![
                output("proj-mara", vec![text_event("Mara considers the player.")]),
                output("persona-mara", vec![text_event("I nod and say nothing")]),
                output("proj-narrate", vec![text_event("The room settles.")]),
            ]),
            "gpt-5.6-sol".into(),
            "gpt-5.6-sol".into(),
        )
        .unwrap();
        let directory = tempfile::tempdir().unwrap();
        let table = PlayTable::new(
            fixture.world.clone(),
            personas,
            ScriptedPort::new(vec![dispatch_round, speak_round]),
            "gpt-5.6-terra".into(),
            Arc::new(Semaphore::new(2)),
            directory.path().join("play-turn-v1.cc"),
        )
        .unwrap();
        table
            .run(&fixture.principal, test_turn_id(120), "Who's there?".into())
            .await
            .unwrap();

        let stored = table.store.lock().await;
        let turn = stored.current().unwrap();
        let result = turn
            .calls
            .iter()
            .find(|call| call.round == 1 && call.slot == 0)
            .and_then(|call| call.result.clone())
            .unwrap();
        assert!(result.contains("not an exact quote"), "{result}");
    }

    // `a_refused_player_act_sets_the_refusal_line` (PA-Q8) is not covered by
    // a scripted-port test in this cut: genesis's `WorldMailbox::create`
    // grants only the kernel's own zero-role `speak` affordance
    // (`kernel_speak_grant`/`kernel_speak_entry` in `patch.rs`), whose sole
    // precondition — `CanBroadcast(Colocated)` — is satisfied even by a
    // subject alone in a room, and `declare_affordance` (the only door to a
    // custom precondition) is not in `PLAY_TOOLS`. Forcing a genuine
    // `ActionRejected` therefore needs either a `PLAY_TOOLS` affordance with
    // a real precondition or a second world fixture this cut does not build.
    // The wiring itself (`execute_actor_call`'s `ActionRejected` arm setting
    // `turn.refusal` from `describe_refusal`) is a few lines, directly
    // inspectable, and shares its code path with every other refusal this
    // module's other tests already exercise.

    #[tokio::test]
    async fn a_mint_and_a_ruled_fact_commit_as_play() {
        let fixture = play_world(None, "player-mint").await;
        let directory = tempfile::tempdir().unwrap();
        let store_path = directory.path().join("play-turn-v1.cc");

        {
            let personas = PersonaLane::new(
                ControllerPort::new(fixture.world.clone()),
                ScriptedPort::new(vec![output("proj-a", vec![text_event("Fine.")])]),
                "gpt-5.6-sol".into(),
                "gpt-5.6-sol".into(),
            )
            .unwrap();
            let round = output(
                "r0",
                vec![
                    call_event(
                        "c0",
                        "declare_resource",
                        serde_json::json!({"handle": "gold", "label": "Gold"}),
                    ),
                    call_event("c1", END_TURN_TOOL, serde_json::json!({})),
                ],
            );
            let table = PlayTable::new(
                fixture.world.clone(),
                personas,
                ScriptedPort::new(vec![round]),
                "gpt-5.6-terra".into(),
                Arc::new(Semaphore::new(2)),
                &store_path,
            )
            .unwrap();
            table
                .run(&fixture.principal, test_turn_id(90), "Declare gold.".into())
                .await
                .unwrap();
        }

        let snapshot = fixture.world.snapshot().await.unwrap();
        let view = table_view(&snapshot);
        let resource_id = view
            .split("Gold [")
            .nth(1)
            .and_then(|rest| rest.split(']').next())
            .expect("Gold's bracketed id is printed in table_view")
            .to_owned();
        let player = subject_id_text(&snapshot, player_id(&snapshot));

        {
            let personas = PersonaLane::new(
                ControllerPort::new(fixture.world.clone()),
                ScriptedPort::new(vec![output("proj-b", vec![text_event("Fine.")])]),
                "gpt-5.6-sol".into(),
                "gpt-5.6-sol".into(),
            )
            .unwrap();
            let round = output(
                "r0",
                vec![
                    call_event(
                        "c0",
                        "mint",
                        serde_json::json!({
                            "holder": {"ref": "existing", "value": player},
                            "resource": {"ref": "existing", "value": resource_id},
                            "qty": 5,
                        }),
                    ),
                    call_event(
                        "c1",
                        "declare_fact",
                        serde_json::json!({
                            "handle": "ruling",
                            "label": "The Ruling",
                            "statement": "Five bars of gold sit in the vault.",
                            "standing": {"standing": "ruled"},
                        }),
                    ),
                    call_event("c2", END_TURN_TOOL, serde_json::json!({})),
                ],
            );
            let table = PlayTable::new(
                fixture.world.clone(),
                personas,
                ScriptedPort::new(vec![round]),
                "gpt-5.6-terra".into(),
                Arc::new(Semaphore::new(2)),
                &store_path,
            )
            .unwrap();
            table
                .run(
                    &fixture.principal,
                    test_turn_id(91),
                    "Mint gold and rule it so.".into(),
                )
                .await
                .unwrap();
        }

        let snapshot = fixture.world.snapshot().await.unwrap();
        let view = table_view(&snapshot);
        assert!(view.contains("5 of Gold"), "the mint did not commit: {view}");
        // `table_view`'s Facts section lists only facts some subject knows
        // (built from `subject.knowledge`, PA-Q7's own doc comment on
        // `table_view`); a `declare_fact` with nobody granted knowledge of
        // it is invisible there by design, so this checks the call's own
        // recorded result instead of the rendered view.
        let stored = PlayTurnStore::open(&store_path).unwrap();
        let turn = stored.current().unwrap();
        let fact_result = turn
            .calls
            .iter()
            .find(|call| call.round == 0 && call.slot == 1)
            .and_then(|call| call.result.clone())
            .unwrap();
        assert_eq!(fact_result, "applied", "the ruled fact did not commit: {fact_result}");
        assert!(!PLAY_TOOLS.contains(&"admit"), "admit is not a play tool");
    }

    #[tokio::test]
    async fn a_persona_fault_becomes_a_tool_result_and_the_turn_continues() {
        let fixture = play_world(Some("Mara"), "player-persona-fault").await;
        let snapshot = fixture.world.snapshot().await.unwrap();
        let mara = subject_id_text(&snapshot, persona_id(&snapshot));
        let round = output(
            "r0",
            vec![
                call_event(
                    "c0",
                    DISPATCH_TOOL,
                    serde_json::json!({"subjects": [mara]}),
                ),
                call_event("c1", END_TURN_TOOL, serde_json::json!({})),
            ],
        );
        let personas_port = ScriptedPort::new(vec![]);
        // Faults the dispatch's own Projector inference; one clean output is
        // reserved for the player's own closing narration.
        personas_port
            .queue
            .lock()
            .unwrap()
            .push_back(Err("provider unavailable".into()));
        personas_port
            .queue
            .lock()
            .unwrap()
            .push_back(Ok(output("proj-narrate", vec![text_event("Quiet.")])));
        let personas = PersonaLane::new(
            ControllerPort::new(fixture.world.clone()),
            personas_port,
            "gpt-5.6-sol".into(),
            "gpt-5.6-sol".into(),
        )
        .unwrap();
        let directory = tempfile::tempdir().unwrap();
        let table = PlayTable::new(
            fixture.world.clone(),
            personas,
            ScriptedPort::new(vec![round]),
            "gpt-5.6-terra".into(),
            Arc::new(Semaphore::new(2)),
            directory.path().join("play-turn-v1.cc"),
        )
        .unwrap();
        table
            .run(&fixture.principal, test_turn_id(80), "Hello.".into())
            .await
            .unwrap();

        let stored = table.store.lock().await;
        let turn = stored.current().unwrap();
        assert_eq!(turn.state, PlayTurnState::Closed);
        let dispatch_result = turn
            .calls
            .iter()
            .find(|call| call.round == 0 && call.slot == 0)
            .and_then(|call| call.result.clone())
            .unwrap();
        assert!(dispatch_result.contains("could not act"), "{dispatch_result}");
    }

    #[tokio::test]
    async fn persona_dispatch_draws_only_from_the_permit_pool() {
        let fixture = play_world(Some("Mara"), "player-permits").await;
        let snapshot = fixture.world.snapshot().await.unwrap();
        let mara = subject_id_text(&snapshot, persona_id(&snapshot));
        let round = output(
            "r0",
            vec![
                call_event(
                    "c0",
                    DISPATCH_TOOL,
                    serde_json::json!({"subjects": [mara]}),
                ),
                call_event("c1", END_TURN_TOOL, serde_json::json!({})),
            ],
        );
        let personas = PersonaLane::new(
            ControllerPort::new(fixture.world.clone()),
            ScriptedPort::new(vec![
                output("proj-mara", vec![text_event("Mara nods.")]),
                output("persona-mara", vec![text_event("I nod back.")]),
                output("proj-narrate", vec![text_event("The room is quiet.")]),
            ]),
            "gpt-5.6-sol".into(),
            "gpt-5.6-sol".into(),
        )
        .unwrap();
        let directory = tempfile::tempdir().unwrap();
        let permits = Arc::new(Semaphore::new(3));
        let table = PlayTable::new(
            fixture.world.clone(),
            personas,
            ScriptedPort::new(vec![round]),
            "gpt-5.6-terra".into(),
            permits.clone(),
            directory.path().join("play-turn-v1.cc"),
        )
        .unwrap();

        table
            .run(&fixture.principal, test_turn_id(70), "Who's there?".into())
            .await
            .unwrap();

        assert_eq!(
            permits.available_permits(),
            3,
            "a dispatch must release every permit it acquires"
        );
    }

    /// PA.f98: `dispatch` takes the subject id exactly as `table_view` prints
    /// it in brackets (the full canonical id), not the short `<handle>__`
    /// actor-tool prefix. Mutation: reverting `parse_dispatch` to match
    /// `handle_for` instead of `full_id_text` fails this, because the id
    /// given here is longer than 8 hex digits.
    #[tokio::test]
    async fn dispatch_by_the_printed_full_id_runs_the_persona() {
        let fixture = play_world(Some("Mara"), "player-dispatch-full-id").await;
        let snapshot = fixture.world.snapshot().await.unwrap();
        let mara_id = subject_id_text(&snapshot, persona_id(&snapshot));
        let round = output(
            "r0",
            vec![
                call_event(
                    "c0",
                    DISPATCH_TOOL,
                    serde_json::json!({"subjects": [mara_id]}),
                ),
                call_event("c1", END_TURN_TOOL, serde_json::json!({})),
            ],
        );
        let personas = PersonaLane::new(
            ControllerPort::new(fixture.world.clone()),
            ScriptedPort::new(vec![
                output("proj-mara", vec![text_event("Mara considers the player.")]),
                output("persona-mara", vec![text_event("Mara nods once.")]),
                output("proj-narrate", vec![text_event("Quiet.")]),
            ]),
            "gpt-5.6-sol".into(),
            "gpt-5.6-sol".into(),
        )
        .unwrap();
        let directory = tempfile::tempdir().unwrap();
        let table = PlayTable::new(
            fixture.world.clone(),
            personas,
            ScriptedPort::new(vec![round]),
            "gpt-5.6-terra".into(),
            Arc::new(Semaphore::new(2)),
            directory.path().join("play-turn-v1.cc"),
        )
        .unwrap();
        table
            .run(&fixture.principal, test_turn_id(71), "Who's there?".into())
            .await
            .unwrap();

        let stored = table.store.lock().await;
        let turn = stored.current().unwrap();
        let dispatch_result = turn
            .calls
            .iter()
            .find(|call| call.round == 0 && call.slot == 0)
            .and_then(|call| call.result.clone())
            .unwrap();
        assert!(dispatch_result.contains("Mara nods once."), "{dispatch_result}");
    }

    /// PA.f98: an id naming no dispatchable subject is refused in the tool
    /// result, quoting the text given, rather than being silently dropped.
    #[tokio::test]
    async fn an_unknown_dispatch_id_is_refused_naming_it() {
        let fixture = play_world(Some("Mara"), "player-dispatch-unknown-id").await;
        let round = output(
            "r0",
            vec![
                call_event(
                    "c0",
                    DISPATCH_TOOL,
                    serde_json::json!({"subjects": ["not-a-real-id"]}),
                ),
                call_event("c1", END_TURN_TOOL, serde_json::json!({})),
            ],
        );
        let personas = PersonaLane::new(
            ControllerPort::new(fixture.world.clone()),
            // No queued output: if the unknown id were somehow dispatched
            // anyway, the persona lane would fault trying to pop from an
            // empty queue, and this test would fail loudly rather than
            // silently accepting a dropped id.
            ScriptedPort::new(vec![output("proj-narrate", vec![text_event("Quiet.")])]),
            "gpt-5.6-sol".into(),
            "gpt-5.6-sol".into(),
        )
        .unwrap();
        let directory = tempfile::tempdir().unwrap();
        let table = PlayTable::new(
            fixture.world.clone(),
            personas,
            ScriptedPort::new(vec![round]),
            "gpt-5.6-terra".into(),
            Arc::new(Semaphore::new(2)),
            directory.path().join("play-turn-v1.cc"),
        )
        .unwrap();
        table
            .run(&fixture.principal, test_turn_id(72), "Who's there?".into())
            .await
            .unwrap();

        let stored = table.store.lock().await;
        let turn = stored.current().unwrap();
        let dispatch_result = turn
            .calls
            .iter()
            .find(|call| call.round == 0 && call.slot == 0)
            .and_then(|call| call.result.clone())
            .unwrap();
        assert!(
            dispatch_result.contains("`not-a-real-id` names no dispatchable subject"),
            "{dispatch_result}"
        );
    }

    /// PA.f98: a mixed dispatch list runs every valid id and names every
    /// invalid one in the same call's result — neither is dropped, and the
    /// valid one still actually acts.
    #[tokio::test]
    async fn a_mixed_dispatch_list_runs_the_valid_id_and_names_the_invalid_one() {
        let fixture = play_world(Some("Mara"), "player-dispatch-mixed").await;
        let snapshot = fixture.world.snapshot().await.unwrap();
        let mara_id = subject_id_text(&snapshot, persona_id(&snapshot));
        let round = output(
            "r0",
            vec![
                call_event(
                    "c0",
                    DISPATCH_TOOL,
                    serde_json::json!({"subjects": [mara_id, "not-a-real-id"]}),
                ),
                call_event("c1", END_TURN_TOOL, serde_json::json!({})),
            ],
        );
        let personas = PersonaLane::new(
            ControllerPort::new(fixture.world.clone()),
            ScriptedPort::new(vec![
                output("proj-mara", vec![text_event("Mara considers the player.")]),
                output("persona-mara", vec![text_event("Mara nods once.")]),
                output("proj-narrate", vec![text_event("Quiet.")]),
            ]),
            "gpt-5.6-sol".into(),
            "gpt-5.6-sol".into(),
        )
        .unwrap();
        let directory = tempfile::tempdir().unwrap();
        let table = PlayTable::new(
            fixture.world.clone(),
            personas,
            ScriptedPort::new(vec![round]),
            "gpt-5.6-terra".into(),
            Arc::new(Semaphore::new(2)),
            directory.path().join("play-turn-v1.cc"),
        )
        .unwrap();
        table
            .run(&fixture.principal, test_turn_id(73), "Who's there?".into())
            .await
            .unwrap();

        let stored = table.store.lock().await;
        let turn = stored.current().unwrap();
        let dispatch_result = turn
            .calls
            .iter()
            .find(|call| call.round == 0 && call.slot == 0)
            .and_then(|call| call.result.clone())
            .unwrap();
        assert!(dispatch_result.contains("Mara nods once."), "{dispatch_result}");
        assert!(
            dispatch_result.contains("`not-a-real-id` names no dispatchable subject"),
            "{dispatch_result}"
        );
    }

    /// PA.f114: `PLAY_INSTRUCTIONS` promises an id naming no dispatchable
    /// subject is refused, quoting what was given, rather than silently
    /// doing nothing. `parse_dispatch` used to drop a non-string entry with
    /// `Value::as_str` filtering it out unseen; it is now refused by name.
    /// Tested directly against `parse_dispatch`, the exact function the
    /// dispatch tool call handler runs the arguments through.
    #[tokio::test]
    async fn a_non_string_dispatch_entry_is_refused_quoting_it() {
        let fixture = play_world(Some("Mara"), "player-dispatch-non-string").await;
        let snapshot = fixture.world.snapshot().await.unwrap();
        let (subjects, refusals) = parse_dispatch(r#"{"subjects": [42, true]}"#, &snapshot);
        assert!(subjects.is_empty());
        assert_eq!(
            refusals,
            vec![
                "`42` in `subjects` is not a string".to_owned(),
                "`true` in `subjects` is not a string".to_owned(),
            ]
        );
    }

    /// PA.f114: a body that isn't valid JSON, and a body whose JSON isn't an
    /// object, are each refused quoting the text given — not the silent
    /// empty dispatch `parse_dispatch` used to return for both.
    #[tokio::test]
    async fn a_malformed_dispatch_body_is_refused_quoting_it() {
        let fixture = play_world(Some("Mara"), "player-dispatch-malformed").await;
        let snapshot = fixture.world.snapshot().await.unwrap();

        let (subjects, refusals) = parse_dispatch("not json at all", &snapshot);
        assert!(subjects.is_empty());
        assert_eq!(refusals.len(), 1, "{refusals:?}");
        assert!(refusals[0].contains("not valid JSON"), "{refusals:?}");
        assert!(refusals[0].contains("not json at all"), "{refusals:?}");

        let (subjects, refusals) = parse_dispatch(r#"["a", "b"]"#, &snapshot);
        assert!(subjects.is_empty());
        assert_eq!(refusals.len(), 1, "{refusals:?}");
        assert!(refusals[0].contains("not a JSON object"), "{refusals:?}");
    }

    /// PA.f114: `dispatch`'s arguments missing the `subjects` key entirely
    /// are refused rather than read as an empty dispatch list.
    #[tokio::test]
    async fn a_dispatch_body_missing_subjects_key_is_refused() {
        let fixture = play_world(Some("Mara"), "player-dispatch-missing-key").await;
        let snapshot = fixture.world.snapshot().await.unwrap();
        let (subjects, refusals) = parse_dispatch("{}", &snapshot);
        assert!(subjects.is_empty());
        assert_eq!(refusals.len(), 1, "{refusals:?}");
        assert!(refusals[0].contains("no `subjects` key"), "{refusals:?}");
    }

    #[tokio::test]
    async fn a_question_holds_the_turn_open_across_a_restart() {
        let fixture = play_world(None, "player-question").await;
        let directory = tempfile::tempdir().unwrap();
        let store_path = directory.path().join("play-turn-v1.cc");
        let opening_key = test_turn_id(60);

        let ask = output(
            "r0",
            vec![call_event(
                "c0",
                ASK_PLAYER_TOOL,
                serde_json::json!({"question": "Which way?"}),
            )],
        );
        {
            let personas = PersonaLane::new(
                ControllerPort::new(fixture.world.clone()),
                ScriptedPort::new(vec![]),
                "gpt-5.6-sol".into(),
                "gpt-5.6-sol".into(),
            )
            .unwrap();
            let table = PlayTable::new(
                fixture.world.clone(),
                personas,
                ScriptedPort::new(vec![ask]),
                "gpt-5.6-terra".into(),
                Arc::new(Semaphore::new(2)),
                &store_path,
            )
            .unwrap();
            table
                .run(
                    &fixture.principal,
                    opening_key.clone(),
                    "I stand at a crossroads.".into(),
                )
                .await
                .unwrap();
        }
        // "Restart": a fresh `PlayTable` reopening the same store path.
        // `PlayTable::new` is the one production door that mints `PlayPort`;
        // this fixture reuses this table's own `play` field for the
        // world-drift commit below rather than minting a second one.
        let end = output("r1", vec![call_event("c1", END_TURN_TOOL, serde_json::json!({}))]);
        let personas = PersonaLane::new(
            ControllerPort::new(fixture.world.clone()),
            ScriptedPort::new(vec![output("proj-after", vec![text_event("You choose left.")])]),
            "gpt-5.6-sol".into(),
            "gpt-5.6-sol".into(),
        )
        .unwrap();
        let table = PlayTable::new(
            fixture.world.clone(),
            personas,
            ScriptedPort::new(vec![end]),
            "gpt-5.6-terra".into(),
            Arc::new(Semaphore::new(2)),
            &store_path,
        )
        .unwrap();

        let (opening_prompt_before, turn_id_before, question_id) = {
            let stored = table.store.lock().await;
            let turn = stored.current().unwrap();
            (turn.opening_prompt.clone(), turn.turn_id.clone(), turn.open_question_id().unwrap())
        };

        // World drift between the question and the answer: a real snapshot
        // taken now would print differently. Mutation M8.2 (rebuilding
        // `opening_prompt` on resume) is caught by the assertion below.
        table
            .play
            .submit_patch(
                CommandId::new(),
                decode_authoring_call(
                    "declare_resource",
                    &serde_json::json!({"handle": "silver", "label": "Silver"}).to_string(),
                )
                .unwrap(),
            )
            .await
            .unwrap();

        // The answer's own request key (PA.f62/PA.f63): a fresh key, not the
        // turn's own identity — the whole point of separating the two is
        // that a caller answering a question need not carry the turn's
        // original opening key forward.
        let answer_key = test_turn_id(61);
        table
            .run(
                &fixture.principal,
                answer_key,
                PlayRequest {
                    text: "left".into(),
                    answers: Some(question_id),
                },
            )
            .await
            .unwrap();

        let stored = table.store.lock().await;
        let turn = stored.current().unwrap();
        assert_eq!(
            turn.opening_prompt, opening_prompt_before,
            "opening_prompt must never be rebuilt on resume (mutation M8.2)"
        );
        assert_eq!(
            turn.turn_id, turn_id_before,
            "the turn's own identity, minted once when it opened, never moves"
        );
        assert_ne!(
            turn.turn_id, opening_key,
            "turn_id is minted fresh (PA.f83), never taken from the opening request key"
        );
        assert_eq!(turn.state, PlayTurnState::Closed);
        assert_eq!(
            turn.player_prose,
            vec!["I stand at a crossroads.".to_owned(), "left".to_owned()]
        );
        assert!(turn.narration.is_some());
    }

    #[tokio::test]
    async fn a_resumed_call_resubmits_its_recorded_body() {
        let fixture = play_world(None, "player-resume-body").await;
        let directory = tempfile::tempdir().unwrap();
        let store_path = directory.path().join("play-turn-v1.cc");
        let turn_id = test_turn_id(50);

        // `PlayTable::new` is the one production door that mints `PlayPort`
        // (pinned by `soul_exactly_one_production_site_mints_play_port`);
        // this fixture reuses the table's own `play` field for the
        // out-of-band pre-crash commit below rather than minting a second
        // one.
        let personas = PersonaLane::new(
            ControllerPort::new(fixture.world.clone()),
            ScriptedPort::new(vec![output("proj-resume", vec![text_event("Still here.")])]),
            "gpt-5.6-sol".into(),
            "gpt-5.6-sol".into(),
        )
        .unwrap();
        let table = PlayTable::new(
            fixture.world.clone(),
            personas,
            ScriptedPort::new(vec![]),
            "gpt-5.6-terra".into(),
            Arc::new(Semaphore::new(2)),
            &store_path,
        )
        .unwrap();

        let patch = decode_authoring_call(
            "declare_resource",
            &serde_json::json!({"handle": "gold", "label": "Gold"}).to_string(),
        )
        .unwrap();
        let command_id = derived_command_id(&turn_id, 0, 0);
        // The exact crash window this rule protects: the kernel already
        // committed the call, but the store never recorded its result.
        table.play.submit_patch(command_id, patch.clone()).await.unwrap();

        let snapshot = fixture.world.snapshot().await.unwrap();
        let opening_prompt = format!(
            "{}\n\nThe player writes:\n{}\n",
            table_view(&snapshot),
            "Make gold."
        );
        let call_id = "c0".to_owned();
        let round = output(
            "r0",
            vec![
                call_event(&call_id, "declare_resource", serde_json::json!({"handle": "gold", "label": "Gold"})),
                call_event("c1", END_TURN_TOOL, serde_json::json!({})),
            ],
        );
        let crashed_turn = PlayTurn {
            turn_id: turn_id.clone(),
            applied_keys: Vec::new(),
            opening_prompt,
            player_prose: vec!["Make gold.".into()],
            rounds: vec![round],
            calls: vec![CallRecord {
                call_id,
                round: 0,
                slot: 0,
                body: Some(RecordedCall::Authoring(patch)),
                result: None,
            }],
            persona_turns: Vec::new(),
            question: None,
            refusal: None,
            narration: None,
            fault: None,
            state: PlayTurnState::Running,
        };
        table.store.lock().await.commit(crashed_turn).unwrap();

        table
            .run(&fixture.principal, turn_id, String::new().into())
            .await
            .unwrap();

        let snapshot = fixture.world.snapshot().await.unwrap();
        let view = table_view(&snapshot);
        assert_eq!(
            view.matches("Gold").count(),
            1,
            "the resumed call must resubmit its recorded body idempotently, not double-commit: {view}"
        );
    }

    /// Mutation M8.3: rebuilding the body from a fresh snapshot on resume,
    /// instead of resubmitting the recorded `(opportunity, invocation)`
    /// verbatim, fails the kernel's own opportunity-identity check —
    /// `PA.f41`'s rule that a caller's opportunity must equal the kernel's
    /// currently issued copy exactly — the moment a second `declare_resource`
    /// between the crash and the resume has changed the world's revision.
    #[tokio::test]
    async fn a_resumed_actor_call_resubmits_its_recorded_body() {
        let fixture = play_world(None, "player-resume-actor").await;
        let directory = tempfile::tempdir().unwrap();
        let store_path = directory.path().join("play-turn-v1.cc");
        let turn_id = test_turn_id(51);

        // `PlayTable::new` is the one production door that mints `PlayPort`;
        // this fixture reuses the table's own `play` field below instead of
        // minting a second one.
        let personas = PersonaLane::new(
            ControllerPort::new(fixture.world.clone()),
            ScriptedPort::new(vec![output("proj-resume", vec![text_event("Still here.")])]),
            "gpt-5.6-sol".into(),
            "gpt-5.6-sol".into(),
        )
        .unwrap();
        let table = PlayTable::new(
            fixture.world.clone(),
            personas,
            ScriptedPort::new(vec![]),
            "gpt-5.6-terra".into(),
            Arc::new(Semaphore::new(2)),
            &store_path,
        )
        .unwrap();

        let snapshot = fixture.world.snapshot().await.unwrap();
        let player = player_id(&snapshot);
        let (opportunity, invocation) =
            decode_actor_call(&snapshot, player, "", "speak", &serde_json::json!({"text": "hold fast"}).to_string())
                .unwrap();
        let command_id = derived_command_id(&turn_id, 0, 0);
        // The crash window: the kernel already committed the exercise, but
        // the store never recorded its result. The world then drifts
        // (a second, unrelated commit) before the resume runs.
        fixture
            .world
            .submit_principal(
                PrincipalCommandIntent {
                    id: command_id,
                    world_id: opportunity.world_id,
                    expected_revision: 0,
                    body: CommandBody::ExerciseDecision {
                        opportunity: opportunity.clone(),
                        invocation: invocation.clone(),
                    },
                },
                &fixture.principal,
            )
            .await
            .unwrap();
        table
            .play
            .submit_patch(
                CommandId::new(),
                decode_authoring_call(
                    "declare_resource",
                    &serde_json::json!({"handle": "silver", "label": "Silver"}).to_string(),
                )
                .unwrap(),
            )
            .await
            .unwrap();

        let opening_prompt = format!(
            "{}\n\nThe player writes:\n{}\n",
            table_view(&snapshot),
            "hold fast"
        );
        let call_id = "c0".to_owned();
        let player_handle = handle_for(player);
        let round = output(
            "r0",
            vec![
                call_event(
                    &call_id,
                    &format!("{player_handle}{HANDLE_SEPARATOR}speak"),
                    serde_json::json!({"text": "hold fast"}),
                ),
                call_event("c1", END_TURN_TOOL, serde_json::json!({})),
            ],
        );
        let crashed_turn = PlayTurn {
            turn_id: turn_id.clone(),
            applied_keys: Vec::new(),
            opening_prompt,
            player_prose: vec!["hold fast".into()],
            rounds: vec![round],
            calls: vec![CallRecord {
                call_id,
                round: 0,
                slot: 0,
                body: Some(RecordedCall::PlayerAct(opportunity, invocation)),
                result: None,
            }],
            persona_turns: Vec::new(),
            question: None,
            refusal: None,
            narration: None,
            fault: None,
            state: PlayTurnState::Running,
        };
        table.store.lock().await.commit(crashed_turn).unwrap();

        table
            .run(&fixture.principal, turn_id, String::new().into())
            .await
            .unwrap();

        let stored = table.store.lock().await;
        let turn = stored.current().unwrap();
        assert_eq!(turn.state, PlayTurnState::Closed);
        let result = turn
            .calls
            .iter()
            .find(|call| call.round == 0 && call.slot == 0)
            .and_then(|call| call.result.clone())
            .unwrap();
        assert_eq!(result, "applied", "a resubmitted actor call must resolve to AlreadyApplied, not a fresh refusal: {result}");
    }

    #[tokio::test]
    async fn restart_equivalence_of_the_agent_input() {
        let directory = tempfile::tempdir().unwrap();
        let world_path = directory.path().join("world.cc");
        let store_path = directory.path().join("play-turn-v1.cc");
        let backup_path = directory.path().join("play-turn-v1.backup.cc");
        let principal = VerifiedPrincipalEvidence::new("player-restart-equiv", far_future());

        let (world, owner) = WorldMailbox::open(&world_path).unwrap();
        let receipt = world
            .create(
                CreateWorldIntent {
                    id: CommandId::new(),
                    title: "Restart Equivalence".into(),
                    brief: String::new(),
                    human_subject_label: "Player".into(),
                    narrative_persona_label: None,
                    operational_agent_label: None,
                    targets: BTreeMap::new(),
                    jurisdictions: Vec::new(),
                    lens_weights: LensWeights::new(BTreeMap::from([(Lens::Hearth, 1)])),
                },
                &principal,
            )
            .await
            .unwrap();
        let snapshot = world.snapshot().await.unwrap();
        world
            .submit_principal(
                PrincipalCommandIntent {
                    id: CommandId::new(),
                    world_id: receipt.world_id,
                    expected_revision: snapshot.revision,
                    body: CommandBody::ApproveDraft,
                },
                &principal,
            )
            .await
            .unwrap();
        let snapshot = world.snapshot().await.unwrap();
        world
            .submit_principal(
                PrincipalCommandIntent {
                    id: CommandId::new(),
                    world_id: receipt.world_id,
                    expected_revision: snapshot.revision,
                    body: CommandBody::ActivateWorld,
                },
                &principal,
            )
            .await
            .unwrap();

        {
            let personas = PersonaLane::new(
                ControllerPort::new(world.clone()),
                ScriptedPort::new(vec![output("proj-1", vec![text_event("Fine.")])]),
                "gpt-5.6-sol".into(),
                "gpt-5.6-sol".into(),
            )
            .unwrap();
            let end = output("r0", vec![call_event("c0", END_TURN_TOOL, serde_json::json!({}))]);
            let table = PlayTable::new(
                world.clone(),
                personas,
                ScriptedPort::new(vec![end]),
                "gpt-5.6-terra".into(),
                Arc::new(Semaphore::new(2)),
                &store_path,
            )
            .unwrap();
            table
                .run(&principal, test_turn_id(30), "Turn one.".into())
                .await
                .unwrap();
        }
        // Turn two opens and asks a question, fixing its own `turn_id` in
        // the store (PA.f83: minted fresh, not derived from the key) before
        // the backup is taken — a fresh `begin_turn` mints a new random
        // `turn_id` every time it runs, so two *independent* openings from
        // the same prior state are no longer expected to produce identical
        // requests. Resuming one *already-open* turn across a restart still
        // must, and that is what this test now checks.
        let (question_id, turn2_id) = {
            let personas = PersonaLane::new(
                ControllerPort::new(world.clone()),
                ScriptedPort::new(vec![]),
                "gpt-5.6-sol".into(),
                "gpt-5.6-sol".into(),
            )
            .unwrap();
            let ask = output(
                "r0",
                vec![call_event("c0", ASK_PLAYER_TOOL, serde_json::json!({"question": "Which path?"}))],
            );
            let table = PlayTable::new(
                world.clone(),
                personas,
                ScriptedPort::new(vec![ask]),
                "gpt-5.6-terra".into(),
                Arc::new(Semaphore::new(2)),
                &store_path,
            )
            .unwrap();
            table
                .run(&principal, test_turn_id(31), "Turn two.".into())
                .await
                .unwrap();
            let stored = table.store.lock().await;
            let turn = stored.current().unwrap();
            (turn.open_question_id().unwrap(), turn.turn_id.clone())
        };
        std::fs::copy(&store_path, &backup_path).unwrap();
        let answer_end = || output("r1", vec![call_event("c1", END_TURN_TOOL, serde_json::json!({}))]);

        // Path A: no restart, answer the already-open question directly.
        let request_a = {
            let personas = PersonaLane::new(
                ControllerPort::new(world.clone()),
                ScriptedPort::new(vec![output("proj-2a", vec![text_event("Still fine.")])]),
                "gpt-5.6-sol".into(),
                "gpt-5.6-sol".into(),
            )
            .unwrap();
            let port = ScriptedPort::new(vec![answer_end()]);
            let table = PlayTable::new(
                world.clone(),
                personas,
                port.clone(),
                "gpt-5.6-terra".into(),
                Arc::new(Semaphore::new(2)),
                &store_path,
            )
            .unwrap();
            table
                .run(
                    &principal,
                    test_turn_id(32),
                    PlayRequest {
                        text: "left".into(),
                        answers: Some(question_id.clone()),
                    },
                )
                .await
                .unwrap();
            port.seen_requests().into_iter().next().unwrap()
        };

        // Restart: drop every live clone of the world mailbox, join its
        // owner task, restore the store to its post-question state, and
        // reopen both.
        std::fs::copy(&backup_path, &store_path).unwrap();
        drop(world);
        owner.await.unwrap();
        let (world, _owner) = WorldMailbox::open(&world_path).unwrap();

        let request_b = {
            let personas = PersonaLane::new(
                ControllerPort::new(world.clone()),
                ScriptedPort::new(vec![output("proj-2b", vec![text_event("Still fine.")])]),
                "gpt-5.6-sol".into(),
                "gpt-5.6-sol".into(),
            )
            .unwrap();
            let port = ScriptedPort::new(vec![answer_end()]);
            let table = PlayTable::new(
                world.clone(),
                personas,
                port.clone(),
                "gpt-5.6-terra".into(),
                Arc::new(Semaphore::new(2)),
                &store_path,
            )
            .unwrap();
            table
                .run(
                    &principal,
                    test_turn_id(32),
                    PlayRequest {
                        text: "left".into(),
                        answers: Some(question_id.clone()),
                    },
                )
                .await
                .unwrap();
            let stored = table.store.lock().await;
            assert_eq!(stored.current().unwrap().turn_id, turn2_id, "the resumed turn's own id must survive the restart unchanged");
            drop(stored);
            port.seen_requests().into_iter().next().unwrap()
        };

        assert_eq!(request_a, request_b);
    }

    // --- Fix batch 8a-fix ------------------------------------------------

    /// The bracketed canonical id following one label in `table_view`'s own
    /// rendering, the same parsing `a_mint_and_a_ruled_fact_commit_as_play`
    /// already relies on for a declared resource's id.
    fn bracketed_id_after(view: &str, needle: &str) -> String {
        view.split(needle)
            .nth(1)
            .and_then(|rest| rest.split(']').next())
            .unwrap_or_else(|| panic!("`{needle}` not found in table_view: {view}"))
            .to_owned()
    }

    /// PA.f61: `set_persona_material` refuses an existing subject.
    #[tokio::test]
    async fn set_persona_material_on_an_existing_subject_is_refused() {
        let fixture = play_world(None, "player-persona-existing").await;
        let before = fixture.world.snapshot().await.unwrap();
        let player = subject_id_text(&before, player_id(&before));
        let round = output(
            "r0",
            vec![
                call_event(
                    "c0",
                    "set_persona_material",
                    serde_json::json!({
                        "subject": {"ref": "existing", "value": player},
                        "values": ["steadfast"],
                        "voice": "flat and even",
                        "memories": [],
                        "reads": [],
                    }),
                ),
                call_event("c1", END_TURN_TOOL, serde_json::json!({})),
            ],
        );
        let personas = PersonaLane::new(
            ControllerPort::new(fixture.world.clone()),
            ScriptedPort::new(vec![output("proj", vec![text_event("Quiet.")])]),
            "gpt-5.6-sol".into(),
            "gpt-5.6-sol".into(),
        )
        .unwrap();
        let directory = tempfile::tempdir().unwrap();
        let table = PlayTable::new(
            fixture.world.clone(),
            personas,
            ScriptedPort::new(vec![round]),
            "gpt-5.6-terra".into(),
            Arc::new(Semaphore::new(2)),
            directory.path().join("play-turn-v1.cc"),
        )
        .unwrap();
        table
            .run(&fixture.principal, test_turn_id(400), "Rewrite the player.".into())
            .await
            .unwrap();

        let stored = table.store.lock().await;
        let turn = stored.current().unwrap();
        let result = turn
            .calls
            .iter()
            .find(|call| call.round == 0 && call.slot == 0)
            .and_then(|call| call.result.clone())
            .unwrap();
        assert!(result.contains("existing subject"), "{result}");
        let after = fixture.world.snapshot().await.unwrap();
        assert_eq!(after.revision, before.revision, "a refused run must commit nothing");
    }

    /// PA.f61 + PA.f54: `set_persona_material` stands for a subject the same
    /// authoring run declares — the draft handle resolves only because the
    /// whole run decodes as one shared patch.
    #[tokio::test]
    async fn set_persona_material_on_a_same_run_declared_subject_commits() {
        let fixture = play_world(None, "player-persona-declared").await;
        let snapshot = fixture.world.snapshot().await.unwrap();
        let affordance_id = bracketed_id_after(&table_view(&snapshot), "speak [");
        let round = output(
            "r0",
            vec![
                call_event(
                    "c0",
                    "declare_subject",
                    serde_json::json!({
                        "handle": "npc",
                        "label": "A Stranger",
                        "kind": "person",
                        "controller": {"type": "narrative_persona"},
                        "affordances": [{"ref": "existing", "value": affordance_id}],
                        "position": null,
                    }),
                ),
                call_event(
                    "c1",
                    "set_persona_material",
                    serde_json::json!({
                        "subject": {"ref": "draft", "value": "npc"},
                        "values": ["steadfast"],
                        "voice": "flat and even",
                        "memories": ["the first watch"],
                        "reads": [],
                    }),
                ),
                call_event("c2", END_TURN_TOOL, serde_json::json!({})),
            ],
        );
        let personas = PersonaLane::new(
            ControllerPort::new(fixture.world.clone()),
            ScriptedPort::new(vec![output("proj", vec![text_event("Quiet.")])]),
            "gpt-5.6-sol".into(),
            "gpt-5.6-sol".into(),
        )
        .unwrap();
        let directory = tempfile::tempdir().unwrap();
        let table = PlayTable::new(
            fixture.world.clone(),
            personas,
            ScriptedPort::new(vec![round]),
            "gpt-5.6-terra".into(),
            Arc::new(Semaphore::new(2)),
            directory.path().join("play-turn-v1.cc"),
        )
        .unwrap();
        table
            .run(&fixture.principal, test_turn_id(401), "Introduce a stranger.".into())
            .await
            .unwrap();

        let after = fixture.world.snapshot().await.unwrap();
        let view = table_view(&after);
        assert!(view.contains("A Stranger"), "{view}");
        assert!(view.contains("flat and even"), "the persona material did not commit: {view}");
    }

    /// PA.f54: a maximal run of authoring calls commits atomically, as one
    /// kernel command — a later call's declaration resolving an earlier
    /// call's draft handle in the same run is possible only because both
    /// land in one shared patch.
    #[tokio::test]
    async fn an_authoring_run_commits_as_one_patch() {
        let fixture = play_world(None, "player-run-atomic").await;
        let before = fixture.world.snapshot().await.unwrap();
        let commons_id = bracketed_id_after(&table_view(&before), "The Commons [");

        let round = output(
            "r0",
            vec![
                call_event(
                    "c0",
                    "declare_place",
                    serde_json::json!({"handle": "cell", "label": "A Narrow Cell", "container": null}),
                ),
                call_event(
                    "c1",
                    "declare_route",
                    serde_json::json!({
                        "handle": "hall",
                        "label": "The Hall",
                        "from": {"ref": "existing", "value": commons_id},
                        "to": {"ref": "draft", "value": "cell"},
                        "access": {"access": "public"},
                        "cost": 1,
                    }),
                ),
                call_event("c2", END_TURN_TOOL, serde_json::json!({})),
            ],
        );
        let personas = PersonaLane::new(
            ControllerPort::new(fixture.world.clone()),
            ScriptedPort::new(vec![output("proj", vec![text_event("Quiet.")])]),
            "gpt-5.6-sol".into(),
            "gpt-5.6-sol".into(),
        )
        .unwrap();
        let directory = tempfile::tempdir().unwrap();
        let table = PlayTable::new(
            fixture.world.clone(),
            personas,
            ScriptedPort::new(vec![round]),
            "gpt-5.6-terra".into(),
            Arc::new(Semaphore::new(2)),
            directory.path().join("play-turn-v1.cc"),
        )
        .unwrap();
        table
            .run(&fixture.principal, test_turn_id(402), "Build a hall to a cell.".into())
            .await
            .unwrap();

        let after = fixture.world.snapshot().await.unwrap();
        assert_eq!(
            after.revision,
            before.revision + 1,
            "the whole run must commit as one kernel command, not two"
        );
        let view = table_view(&after);
        assert!(view.contains("A Narrow Cell"), "{view}");
        assert!(view.contains("The Hall"), "{view}");
    }

    /// PA.f54: a refusal inside a run names the exact call that caused it,
    /// through the run's own `DecodedBatch` site map, and nothing in the run
    /// commits — not even an earlier call's own declaration.
    #[tokio::test]
    async fn a_refusal_in_a_run_names_the_failing_call_and_nothing_commits() {
        let fixture = play_world(None, "player-run-refusal").await;
        let before = fixture.world.snapshot().await.unwrap();
        let player = handle_for(player_id(&before));

        // `declare_place` (call #0) decodes cleanly; `relocate` (call #1) —
        // an *operation*, not a declaration — references a route draft
        // handle nothing in this run ever declares. An operation mismatch's
        // site is attributed by exact call index through the run's own
        // `DecodedBatch`, unlike a declaration's own (handle-named) site.
        let round = output(
            "r0",
            vec![
                call_event(
                    "c0",
                    "declare_place",
                    serde_json::json!({"handle": "cell", "label": "A Narrow Cell", "container": null}),
                ),
                call_event(
                    "c1",
                    "relocate",
                    serde_json::json!({
                        "subject": {"ref": "existing", "value": player},
                        "via": {"ref": "draft", "value": "no_such_route"},
                    }),
                ),
                call_event("c2", END_TURN_TOOL, serde_json::json!({})),
            ],
        );
        let personas = PersonaLane::new(
            ControllerPort::new(fixture.world.clone()),
            ScriptedPort::new(vec![output("proj", vec![text_event("Quiet.")])]),
            "gpt-5.6-sol".into(),
            "gpt-5.6-sol".into(),
        )
        .unwrap();
        let directory = tempfile::tempdir().unwrap();
        let table = PlayTable::new(
            fixture.world.clone(),
            personas,
            ScriptedPort::new(vec![round]),
            "gpt-5.6-terra".into(),
            Arc::new(Semaphore::new(2)),
            directory.path().join("play-turn-v1.cc"),
        )
        .unwrap();
        table
            .run(&fixture.principal, test_turn_id(403), "Build a hall to nowhere.".into())
            .await
            .unwrap();

        let after = fixture.world.snapshot().await.unwrap();
        assert_eq!(after.revision, before.revision, "a refused run must commit nothing");
        assert!(
            !table_view(&after).contains("A Narrow Cell"),
            "the first call's own declaration must not commit either"
        );

        let stored = table.store.lock().await;
        let turn = stored.current().unwrap();
        let result0 = turn
            .calls
            .iter()
            .find(|call| call.round == 0 && call.slot == 0)
            .and_then(|call| call.result.clone())
            .unwrap();
        let result1 = turn
            .calls
            .iter()
            .find(|call| call.round == 0 && call.slot == 1)
            .and_then(|call| call.result.clone())
            .unwrap();
        assert!(result1.contains("call #1"), "{result1}");
        assert_eq!(result0, result1, "every call in a run shares the run's own single outcome");
    }

    /// One half of PA.f87's replacement for the old source-text-position
    /// pin: given a store that already holds a run's own recorded body from
    /// before a crash (built directly here, the same "crash window" shape
    /// `a_resumed_call_resubmits_its_recorded_body` uses for a single
    /// call), resume must resubmit that exact recorded body idempotently —
    /// one kernel commit, not two. This alone does not prove the record
    /// happens *before* the submit in a first, non-resumed attempt; see
    /// `commit_authoring_run_records_the_run_even_when_its_submit_is_refused`
    /// for that half.
    #[tokio::test]
    async fn a_resumed_authoring_run_resubmits_its_recorded_shared_patch() {
        let fixture = play_world(None, "player-resume-run").await;
        let directory = tempfile::tempdir().unwrap();
        let store_path = directory.path().join("play-turn-v1.cc");
        let turn_id = test_turn_id(52);

        // `PlayTable::new` is the one production door that mints `PlayPort`;
        // this fixture reuses the table's own `play` field for the
        // out-of-band pre-crash commit below rather than minting a second
        // one.
        let personas = PersonaLane::new(
            ControllerPort::new(fixture.world.clone()),
            ScriptedPort::new(vec![output("proj-resume-run", vec![text_event("Still here.")])]),
            "gpt-5.6-sol".into(),
            "gpt-5.6-sol".into(),
        )
        .unwrap();
        let table = PlayTable::new(
            fixture.world.clone(),
            personas,
            ScriptedPort::new(vec![]),
            "gpt-5.6-terra".into(),
            Arc::new(Semaphore::new(2)),
            &store_path,
        )
        .unwrap();

        let before = fixture.world.snapshot().await.unwrap();
        let commons_id = bracketed_id_after(&table_view(&before), "The Commons [");
        let declare_place_args =
            serde_json::json!({"handle": "cell", "label": "A Narrow Cell", "container": null}).to_string();
        let declare_route_args = serde_json::json!({
            "handle": "hall",
            "label": "The Hall",
            "from": {"ref": "existing", "value": commons_id},
            "to": {"ref": "draft", "value": "cell"},
            "access": {"access": "public"},
            "cost": 1,
        })
        .to_string();
        let decoded = decode_authoring_calls(&[
            ("declare_place", declare_place_args.as_str()),
            ("declare_route", declare_route_args.as_str()),
        ])
        .unwrap();
        let patch = decoded.patch.clone();
        let command_id = derived_command_id(&turn_id, 0, 0);
        // The exact crash window this rule protects: the kernel already
        // committed the run's own shared patch, but the store never
        // recorded its result.
        table.play.submit_patch(command_id, patch.clone()).await.unwrap();

        let opening_prompt = format!("{}\n\nThe player writes:\n{}\n", table_view(&before), "Build a hall.");
        let round = output(
            "r0",
            vec![
                call_event(
                    "c0",
                    "declare_place",
                    serde_json::json!({"handle": "cell", "label": "A Narrow Cell", "container": null}),
                ),
                call_event(
                    "c1",
                    "declare_route",
                    serde_json::json!({
                        "handle": "hall",
                        "label": "The Hall",
                        "from": {"ref": "existing", "value": commons_id},
                        "to": {"ref": "draft", "value": "cell"},
                        "access": {"access": "public"},
                        "cost": 1,
                    }),
                ),
                call_event("c2", END_TURN_TOOL, serde_json::json!({})),
            ],
        );
        let crashed_turn = PlayTurn {
            turn_id: turn_id.clone(),
            applied_keys: Vec::new(),
            opening_prompt,
            player_prose: vec!["Build a hall.".into()],
            rounds: vec![round],
            calls: vec![CallRecord {
                call_id: "c0".into(),
                round: 0,
                slot: 0,
                body: Some(RecordedCall::Authoring(patch)),
                result: None,
            }],
            persona_turns: Vec::new(),
            question: None,
            refusal: None,
            narration: None,
            fault: None,
            state: PlayTurnState::Running,
        };
        table.store.lock().await.commit(crashed_turn).unwrap();

        table.run(&fixture.principal, turn_id, String::new().into()).await.unwrap();

        let after = fixture.world.snapshot().await.unwrap();
        // One commit, not two: the world's own revision counter is the
        // unambiguous signal — `table_view` prints a place's own label a
        // second time wherever a route mentions it as an endpoint, so a
        // substring count is not (this mirrors `an_authoring_run_commits_as_one_patch`'s
        // own revision-based check).
        assert_eq!(
            after.revision,
            before.revision + 1,
            "the resumed run must resubmit its recorded shared patch idempotently, not double-commit"
        );
        let view = table_view(&after);
        assert!(view.contains("A Narrow Cell"), "{view}");
        assert!(view.contains("The Hall"), "{view}");
    }

    /// The other half of PA.f87: `commit_authoring_run` must record the
    /// run's own decoded patch *before* it ever finds out whether
    /// `submit_patch` will succeed — not only after a success, and not only
    /// when a caller happens to resume a pre-crashed turn. Proven with a
    /// run whose kernel submission is genuinely refused (the same
    /// `relocate`-to-a-nonexistent-route shape
    /// `a_refusal_in_a_run_names_the_failing_call_and_nothing_commits`
    /// uses): the decode itself succeeds, so a correct `commit_authoring_run`
    /// records the run's own patch regardless of what the kernel later says
    /// about it. Mutations: delete
    /// `record_call(..., Some(RecordedCall::Authoring(...)), ...)`, or move
    /// it inside the `Ok(_receipt) =>` success arm of the `submit_patch`
    /// match (making the record conditional on success); both leave the
    /// anchor slot's own `body` unset after this refusal, failing this test.
    #[tokio::test]
    async fn commit_authoring_run_records_the_run_even_when_its_submit_is_refused() {
        let fixture = play_world(None, "player-record-before-refused-submit").await;
        let before = fixture.world.snapshot().await.unwrap();
        let player = subject_id_text(&before, player_id(&before));

        let round = output(
            "r0",
            vec![
                call_event(
                    "c0",
                    "declare_place",
                    serde_json::json!({"handle": "cell", "label": "A Narrow Cell", "container": null}),
                ),
                call_event(
                    "c1",
                    "relocate",
                    serde_json::json!({
                        "subject": {"ref": "existing", "value": player},
                        "via": {"ref": "draft", "value": "no_such_route"},
                    }),
                ),
                call_event("c2", END_TURN_TOOL, serde_json::json!({})),
            ],
        );
        let personas = PersonaLane::new(
            ControllerPort::new(fixture.world.clone()),
            ScriptedPort::new(vec![output("proj", vec![text_event("Quiet.")])]),
            "gpt-5.6-sol".into(),
            "gpt-5.6-sol".into(),
        )
        .unwrap();
        let directory = tempfile::tempdir().unwrap();
        let table = PlayTable::new(
            fixture.world.clone(),
            personas,
            ScriptedPort::new(vec![round]),
            "gpt-5.6-terra".into(),
            Arc::new(Semaphore::new(2)),
            directory.path().join("play-turn-v1.cc"),
        )
        .unwrap();
        table
            .run(&fixture.principal, test_turn_id(53), "Build a hall to nowhere.".into())
            .await
            .unwrap();

        let after = fixture.world.snapshot().await.unwrap();
        assert_eq!(after.revision, before.revision, "a refused run must commit nothing to the world");

        let stored = table.store.lock().await;
        let turn = stored.current().unwrap();
        let anchor = turn.calls.iter().find(|call| call.round == 0 && call.slot == 0).unwrap();
        assert!(
            matches!(anchor.body, Some(RecordedCall::Authoring(_))),
            "the run's own decoded patch must be recorded even though the kernel refused it: {:?}",
            anchor.body
        );
        assert!(
            anchor.result.as_deref().unwrap_or_default().contains("refused"),
            "{:?}",
            anchor.result
        );
    }

    /// PA.f64: a dispatched Persona's own prose reaches the agent as the
    /// dispatch call's own tool result, verbatim, both on the round it
    /// happened and after a restart resumes a later round.
    #[tokio::test]
    async fn a_dispatched_personas_prose_reaches_the_agents_next_request() {
        let fixture = play_world(Some("Mara"), "player-dispatch-prose").await;
        let snapshot = fixture.world.snapshot().await.unwrap();
        let mara = subject_id_text(&snapshot, persona_id(&snapshot));
        let dispatch_round = output(
            "r0",
            vec![call_event(
                "c0",
                DISPATCH_TOOL,
                serde_json::json!({"subjects": [mara.clone()]}),
            )],
        );
        let ask_round = output(
            "r1",
            vec![call_event(
                "c1",
                ASK_PLAYER_TOOL,
                serde_json::json!({"question": "Well?"}),
            )],
        );
        let personas = PersonaLane::new(
            ControllerPort::new(fixture.world.clone()),
            ScriptedPort::new(vec![
                output("proj-mara", vec![text_event("Mara considers the player.")]),
                output("persona-mara", vec![text_event("The bolt is thrown; nobody enters here.")]),
            ]),
            "gpt-5.6-sol".into(),
            "gpt-5.6-sol".into(),
        )
        .unwrap();
        let directory = tempfile::tempdir().unwrap();
        let port = ScriptedPort::new(vec![dispatch_round, ask_round]);
        let table = PlayTable::new(
            fixture.world.clone(),
            personas,
            port.clone(),
            "gpt-5.6-terra".into(),
            Arc::new(Semaphore::new(2)),
            directory.path().join("play-turn-v1.cc"),
        )
        .unwrap();
        table
            .run(&fixture.principal, test_turn_id(404), "Who's there?".into())
            .await
            .unwrap();

        let seen = port.seen_requests();
        let round1_request = format!("{:?}", seen.get(1).expect("round 1's own request was seen"));
        assert!(
            round1_request.contains("The bolt is thrown; nobody enters here."),
            "round 1's request must carry Mara's own prose verbatim: {round1_request}"
        );
        let question_id = table.store.lock().await.current().unwrap().open_question_id().unwrap();
        // Release the store's exclusive custody before reopening it below —
        // the same "restart" precedent `a_question_holds_the_turn_open_across_a_restart`
        // relies on: a fresh `PlayTable` is the only production door onto a
        // reopened store.
        drop(table);

        // The same request, rebuilt after a restart reopening the same
        // store, must carry the same prose (PA.f64's "including after a
        // resume").
        let personas2 = PersonaLane::new(
            ControllerPort::new(fixture.world.clone()),
            ScriptedPort::new(vec![output("proj-after", vec![text_event("The hall goes still.")])]),
            "gpt-5.6-sol".into(),
            "gpt-5.6-sol".into(),
        )
        .unwrap();
        let port2 = ScriptedPort::new(vec![ask_round_repeat()]);
        let table2 = PlayTable::new(
            fixture.world.clone(),
            personas2,
            port2.clone(),
            "gpt-5.6-terra".into(),
            Arc::new(Semaphore::new(2)),
            directory.path().join("play-turn-v1.cc"),
        )
        .unwrap();
        // Answering the still-open question forces a fresh round to be
        // inferred, rebuilding the conversation from the stored record.
        table2
            .run(
                &fixture.principal,
                test_turn_id(405),
                PlayRequest {
                    text: "Answer.".into(),
                    answers: Some(question_id),
                },
            )
            .await
            .unwrap();
        let seen2 = port2.seen_requests();
        let resumed_request = format!("{:?}", seen2.first().unwrap());
        assert!(
            resumed_request.contains("The bolt is thrown; nobody enters here."),
            "a resumed request must still carry the recorded Persona prose: {resumed_request}"
        );
    }

    fn ask_round_repeat() -> InferenceOutput {
        output(
            "r2",
            vec![call_event("c2", END_TURN_TOOL, serde_json::json!({}))],
        )
    }

    /// PA.f69 S1: a Persona's own exact quote of its own prose is accepted —
    /// mutating `span_is_exact` to check the *player's* prose instead of the
    /// acting Persona's own recorded turn fails this, since the quote below
    /// never appears in the player's own words.
    #[tokio::test]
    async fn a_persona_acting_on_its_own_exact_quote_is_accepted() {
        let fixture = play_world(Some("Mara"), "player-persona-exact-quote").await;
        let snapshot = fixture.world.snapshot().await.unwrap();
        let mara = handle_for(persona_id(&snapshot));
        let mara_id = subject_id_text(&snapshot, persona_id(&snapshot));
        let dispatch_round = output(
            "r0",
            vec![call_event(
                "c0",
                DISPATCH_TOOL,
                serde_json::json!({"subjects": [mara_id]}),
            )],
        );
        let speak_round = output(
            "r1",
            vec![
                call_event(
                    "c1",
                    &format!("{mara}{HANDLE_SEPARATOR}speak"),
                    serde_json::json!({"text": "I nod and say nothing"}),
                ),
                call_event("c2", END_TURN_TOOL, serde_json::json!({})),
            ],
        );
        let personas = PersonaLane::new(
            ControllerPort::new(fixture.world.clone()),
            ScriptedPort::new(vec![
                output("proj-mara", vec![text_event("Mara considers the player.")]),
                output("persona-mara", vec![text_event("I nod and say nothing")]),
                output("proj-narrate", vec![text_event("The room settles.")]),
            ]),
            "gpt-5.6-sol".into(),
            "gpt-5.6-sol".into(),
        )
        .unwrap();
        let directory = tempfile::tempdir().unwrap();
        let table = PlayTable::new(
            fixture.world.clone(),
            personas,
            ScriptedPort::new(vec![dispatch_round, speak_round]),
            "gpt-5.6-terra".into(),
            Arc::new(Semaphore::new(2)),
            directory.path().join("play-turn-v1.cc"),
        )
        .unwrap();
        table
            .run(&fixture.principal, test_turn_id(406), "Who's there?".into())
            .await
            .unwrap();

        let stored = table.store.lock().await;
        let turn = stored.current().unwrap();
        let result = turn
            .calls
            .iter()
            .find(|call| call.round == 1 && call.slot == 0)
            .and_then(|call| call.result.clone())
            .unwrap();
        assert_eq!(result, "applied", "an exact quote of the Persona's own prose must be accepted: {result}");
    }

    /// PA.f65: an empty or whitespace-only answer is refused before it is
    /// recorded; the turn stays exactly as it was.
    #[tokio::test]
    async fn an_empty_answer_is_refused_before_it_is_recorded() {
        let fixture = play_world(None, "player-empty-answer").await;
        let directory = tempfile::tempdir().unwrap();
        let store_path = directory.path().join("play-turn-v1.cc");
        let turn_id = test_turn_id(410);
        let ask = output(
            "r0",
            vec![call_event(
                "c0",
                ASK_PLAYER_TOOL,
                serde_json::json!({"question": "Which way?"}),
            )],
        );
        let personas = PersonaLane::new(
            ControllerPort::new(fixture.world.clone()),
            ScriptedPort::new(vec![]),
            "gpt-5.6-sol".into(),
            "gpt-5.6-sol".into(),
        )
        .unwrap();
        let table = PlayTable::new(
            fixture.world.clone(),
            personas,
            ScriptedPort::new(vec![ask]),
            "gpt-5.6-terra".into(),
            Arc::new(Semaphore::new(2)),
            &store_path,
        )
        .unwrap();
        table
            .run(&fixture.principal, turn_id.clone(), "I stand at a crossroads.".into())
            .await
            .unwrap();
        let question_id = table.store.lock().await.current().unwrap().open_question_id().unwrap();

        let error = table
            .run(
                &fixture.principal,
                test_turn_id(411),
                PlayRequest {
                    text: "   ".into(),
                    answers: Some(question_id),
                },
            )
            .await
            .unwrap_err();
        assert!(matches!(error, PlayError::EmptyAnswer), "{error:?}");

        let stored = table.store.lock().await;
        let turn = stored.current().unwrap();
        assert_eq!(turn.state, PlayTurnState::AwaitingPlayer, "an empty answer must not resume the turn");
        assert_eq!(turn.question.as_deref(), Some("Which way?"));
    }

    struct AlwaysFaultyPort {
        calls: std::sync::atomic::AtomicUsize,
    }

    impl AlwaysFaultyPort {
        fn new() -> Self {
            Self {
                calls: std::sync::atomic::AtomicUsize::new(0),
            }
        }

        fn call_count(&self) -> usize {
            self.calls.load(std::sync::atomic::Ordering::SeqCst)
        }
    }

    #[async_trait::async_trait]
    impl InferencePort for AlwaysFaultyPort {
        fn prepare(&self, _request: InferenceRequest) -> Result<PreparedInference, InferenceFault> {
            self.calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Err(InferenceFault::recovery_required("provider unavailable"))
        }

        async fn infer(&self, _request: PreparedInference) -> Result<InferenceOutput, InferenceFault> {
            Err(InferenceFault::recovery_required("provider unavailable"))
        }
    }

    /// PA.f102: `RecoveryRequired` is renamed honestly from the prior
    /// "exhausted beyond retries" framing this test used to carry — it never
    /// actually drove a `Retryable` fault's own retry-and-exhaust path at
    /// all; every one of its faults was `RecoveryRequired`, which never
    /// retries and closes on its first occurrence. Asserts the call count
    /// directly: exactly one `prepare` call, not the whole `ROUND_RETRY_BUDGET`
    /// a `Retryable` fault would have spent. Mutation: making
    /// `RecoveryRequired` retry (see `a_retryable_fault_exhausts_its_budget_then_closes`
    /// for the sibling budget-exhaustion mutation) fails this by driving the
    /// call count above one.
    #[tokio::test]
    async fn a_recovery_required_fault_makes_one_call_then_closes_with_its_detail() {
        let fixture = play_world(None, "player-recovery-required").await;
        let personas = PersonaLane::new(
            ControllerPort::new(fixture.world.clone()),
            ScriptedPort::new(vec![]),
            "gpt-5.6-sol".into(),
            "gpt-5.6-sol".into(),
        )
        .unwrap();
        let directory = tempfile::tempdir().unwrap();
        let port = Arc::new(AlwaysFaultyPort::new());
        let table = PlayTable::new(
            fixture.world.clone(),
            personas,
            port.clone(),
            "gpt-5.6-terra".into(),
            Arc::new(Semaphore::new(2)),
            directory.path().join("play-turn-v1.cc"),
        )
        .unwrap();

        table
            .run(&fixture.principal, test_turn_id(420), "Hello?".into())
            .await
            .unwrap();

        let stored = table.store.lock().await;
        let turn = stored.current().unwrap();
        assert_eq!(
            turn.state,
            PlayTurnState::Closed,
            "a RecoveryRequired fault must close the turn, not leave it Running"
        );
        assert_eq!(
            turn.fault.as_deref(),
            Some("provider unavailable"),
            "the fault's own detail must be recorded"
        );
        assert_eq!(
            port.call_count(),
            1,
            "RecoveryRequired must never retry: exactly one call, not the whole retry budget"
        );
    }

    /// PA.f102: a persistent `Retryable` fault, unlike `RecoveryRequired`,
    /// does retry — but only up to `ROUND_RETRY_BUDGET` times, then closes
    /// with the fault's own detail. Total calls: the initial attempt plus
    /// one retry per absorbed fault, `ROUND_RETRY_BUDGET + 1`. Mutations:
    /// raising the budget or removing the cap both fail this by changing the
    /// exact call count the test pins.
    #[tokio::test]
    async fn a_retryable_fault_exhausts_its_budget_then_closes() {
        let fixture = play_world(None, "player-retryable-exhausted").await;
        let personas = PersonaLane::new(
            ControllerPort::new(fixture.world.clone()),
            ScriptedPort::new(vec![]),
            "gpt-5.6-sol".into(),
            "gpt-5.6-sol".into(),
        )
        .unwrap();
        // Pinned expectation, deliberately *not* expressed as
        // `ROUND_RETRY_BUDGET + 1`: computing the expected count from the
        // same symbol the production code reads would let a mutation that
        // raises the budget silently drag this test's own expectation along
        // with it. The queue below supplies comfortably more faults than
        // today's real budget can consume, so a raised or uncapped budget
        // shows up as a call count above the pinned `13`.
        const PINNED_EXPECTED_CALLS: usize = ROUND_BUDGET + 1;
        assert_eq!(ROUND_RETRY_BUDGET, ROUND_BUDGET, "this pin assumes today's retry budget; update it deliberately if that changes");
        let directory = tempfile::tempdir().unwrap();
        let port = DispositionPort::new(
            std::iter::repeat_with(|| Err(InferenceFault::retryable("transient provider hiccup")))
                .take(PINNED_EXPECTED_CALLS + 10)
                .collect(),
        );
        let table = PlayTable::new(
            fixture.world.clone(),
            personas,
            port.clone(),
            "gpt-5.6-terra".into(),
            Arc::new(Semaphore::new(2)),
            directory.path().join("play-turn-v1.cc"),
        )
        .unwrap();

        table
            .run(&fixture.principal, test_turn_id(421), "Hello?".into())
            .await
            .unwrap();

        let stored = table.store.lock().await;
        let turn = stored.current().unwrap();
        assert_eq!(
            turn.state,
            PlayTurnState::Closed,
            "a Retryable fault exhausting its budget must close the turn"
        );
        assert_eq!(turn.fault.as_deref(), Some("transient provider hiccup"));
        assert_eq!(
            port.call_count(),
            PINNED_EXPECTED_CALLS,
            "exactly the initial attempt plus ROUND_RETRY_BUDGET retries, no more and no fewer"
        );
    }

    /// PA.f102: the production backoff formula itself, never the
    /// `#[cfg(test)]`-hidden zero delay: it grows, and it caps.
    #[test]
    fn backoff_delay_grows_then_caps() {
        assert_eq!(backoff_delay(0), Duration::from_millis(100));
        assert_eq!(backoff_delay(1), Duration::from_millis(200));
        assert_eq!(backoff_delay(2), Duration::from_millis(400));
        assert_eq!(backoff_delay(5), Duration::from_millis(3_200), "not yet capped");
        assert_eq!(backoff_delay(6), Duration::from_millis(5_000), "capped");
        assert_eq!(
            backoff_delay(1_000_000),
            Duration::from_millis(5_000),
            "a very large attempt count must stay capped, never overflow or panic"
        );
    }

    /// PA.f66a: exhausting the round budget closes the turn and narrates,
    /// rather than leaving it `Running` with nothing said.
    #[tokio::test]
    async fn exhausting_the_round_budget_closes_and_narrates() {
        let fixture = play_world(None, "player-round-budget").await;
        let personas = PersonaLane::new(
            ControllerPort::new(fixture.world.clone()),
            ScriptedPort::new(vec![output("proj-budget", vec![text_event("The moment passes.")])]),
            "gpt-5.6-sol".into(),
            "gpt-5.6-sol".into(),
        )
        .unwrap();
        let rounds: Vec<InferenceOutput> = (0..ROUND_BUDGET)
            .map(|index| output(&format!("r{index}"), vec![text_event("Thinking.")]))
            .collect();
        let directory = tempfile::tempdir().unwrap();
        let table = PlayTable::new(
            fixture.world.clone(),
            personas,
            ScriptedPort::new(rounds),
            "gpt-5.6-terra".into(),
            Arc::new(Semaphore::new(2)),
            directory.path().join("play-turn-v1.cc"),
        )
        .unwrap();

        table
            .run(&fixture.principal, test_turn_id(430), "Stall.".into())
            .await
            .unwrap();

        let stored = table.store.lock().await;
        let turn = stored.current().unwrap();
        assert_eq!(
            turn.state,
            PlayTurnState::Closed,
            "the round budget must close the turn, not leave it Running"
        );
        assert_eq!(turn.narration.as_deref(), Some("The moment passes."));
    }

    /// PA.f103: a narrate failure right after `end_turn` still closes the
    /// turn with the fault recorded — Soul's own probe: `end_turn` with a
    /// starved Persona script that yields no narration at all. Mutation:
    /// turning `close_turn`'s own call site in the `end_turn` branch back
    /// into a bare `self.close_turn(turn).await?` fails this, leaving the
    /// turn `Running` behind the propagated error instead of `Closed` with
    /// `fault` recorded.
    #[tokio::test]
    async fn a_narrate_failure_after_end_turn_closes_with_the_fault_recorded() {
        let fixture = play_world(None, "player-narrate-fault-end-turn").await;
        let personas = PersonaLane::new(
            ControllerPort::new(fixture.world.clone()),
            // Starved: `end_turn`'s own narrate call finds nothing queued.
            ScriptedPort::new(vec![]),
            "gpt-5.6-sol".into(),
            "gpt-5.6-sol".into(),
        )
        .unwrap();
        let round = output("r0", vec![call_event("c0", END_TURN_TOOL, serde_json::json!({}))]);
        let directory = tempfile::tempdir().unwrap();
        let table = PlayTable::new(
            fixture.world.clone(),
            personas,
            ScriptedPort::new(vec![round]),
            "gpt-5.6-terra".into(),
            Arc::new(Semaphore::new(2)),
            directory.path().join("play-turn-v1.cc"),
        )
        .unwrap();

        table
            .run(&fixture.principal, test_turn_id(431), "Hello?".into())
            .await
            .unwrap();

        let stored = table.store.lock().await;
        let turn = stored.current().unwrap();
        assert_eq!(
            turn.state,
            PlayTurnState::Closed,
            "a narrate failure after end_turn must still close the turn"
        );
        assert!(turn.fault.is_some(), "the narrate failure's own fault must be recorded");
        assert!(turn.narration.is_none(), "no narration must be recorded when narrate itself faulted");
    }

    /// PA.f103: a narrate failure at the round budget, not only at
    /// `end_turn`, still closes the turn with the fault recorded — the round
    /// budget's own `close_turn` call site is a second, independent door
    /// onto the same helper. Mutation: turning that call site back into a
    /// bare `?` fails this the same way the sibling `end_turn` mutation does.
    #[tokio::test]
    async fn a_narrate_failure_at_the_round_budget_closes_with_the_fault_recorded() {
        let fixture = play_world(None, "player-narrate-fault-round-budget").await;
        let personas = PersonaLane::new(
            ControllerPort::new(fixture.world.clone()),
            // Starved: the round budget's own narrate call finds nothing
            // queued either.
            ScriptedPort::new(vec![]),
            "gpt-5.6-sol".into(),
            "gpt-5.6-sol".into(),
        )
        .unwrap();
        let rounds: Vec<InferenceOutput> = (0..ROUND_BUDGET)
            .map(|index| output(&format!("r{index}"), vec![text_event("Thinking.")]))
            .collect();
        let directory = tempfile::tempdir().unwrap();
        let table = PlayTable::new(
            fixture.world.clone(),
            personas,
            ScriptedPort::new(rounds),
            "gpt-5.6-terra".into(),
            Arc::new(Semaphore::new(2)),
            directory.path().join("play-turn-v1.cc"),
        )
        .unwrap();

        table
            .run(&fixture.principal, test_turn_id(432), "Stall.".into())
            .await
            .unwrap();

        let stored = table.store.lock().await;
        let turn = stored.current().unwrap();
        assert_eq!(
            turn.state,
            PlayTurnState::Closed,
            "the round budget must still close the turn when its own narrate faults"
        );
        assert!(turn.fault.is_some(), "the narrate failure's own fault must be recorded");
        assert!(turn.narration.is_none(), "no narration must be recorded when narrate itself faulted");
    }

    /// PA.f66c: `dispatched_subjects` is derived from the whole record, in
    /// call order, regardless of which round each dispatch call landed in —
    /// the same value both `infer_round` (building next round's tools) and
    /// `execute_round` (resolving a `<handle>__<kind>` call) read, on a
    /// first pass and after a resume alike.
    #[tokio::test]
    async fn dispatched_subjects_reflects_the_whole_record_in_call_order() {
        let fixture = play_world(Some("Mara"), "player-dispatch-order").await;
        let snapshot = fixture.world.snapshot().await.unwrap();
        let a = player_id(&snapshot);
        let b = persona_id(&snapshot);
        let turn = PlayTurn {
            turn_id: test_turn_id(440),
            applied_keys: Vec::new(),
            opening_prompt: String::new(),
            player_prose: Vec::new(),
            rounds: Vec::new(),
            calls: vec![
                CallRecord {
                    call_id: "c0".into(),
                    round: 0,
                    slot: 0,
                    body: Some(RecordedCall::Dispatch(vec![a])),
                    result: Some("applied".into()),
                },
                CallRecord {
                    call_id: "c1".into(),
                    round: 1,
                    slot: 0,
                    body: Some(RecordedCall::Dispatch(vec![b])),
                    result: Some("applied".into()),
                },
            ],
            // PA.f90: a dispatch counts as dispatched only once its Persona
            // turn actually produced and recorded prose, so both subjects
            // need a `persona_turns` entry here.
            persona_turns: vec![(a, fake_persona_turn("a acted")), (b, fake_persona_turn("b acted"))],
            question: None,
            refusal: None,
            narration: None,
            fault: None,
            state: PlayTurnState::Running,
        };
        assert_eq!(turn.dispatched_subjects(), vec![a, b]);
    }

    /// PA.f90: a dispatch whose Persona turn faulted, or found no live
    /// opportunity, never produced prose — it must not count as dispatched,
    /// so its actor tools are not offered next round and an actor call for
    /// it is refused, even though the `Dispatch` call itself recorded
    /// success.
    #[tokio::test]
    async fn a_dispatch_with_no_recorded_persona_turn_does_not_count_as_dispatched() {
        let fixture = play_world(Some("Mara"), "player-dispatch-no-turn").await;
        let snapshot = fixture.world.snapshot().await.unwrap();
        let subject = persona_id(&snapshot);
        let turn = PlayTurn {
            turn_id: test_turn_id(441),
            applied_keys: Vec::new(),
            opening_prompt: String::new(),
            player_prose: Vec::new(),
            rounds: Vec::new(),
            calls: vec![CallRecord {
                call_id: "c0".into(),
                round: 0,
                slot: 0,
                body: Some(RecordedCall::Dispatch(vec![subject])),
                result: Some("holds no live Persona opportunity".into()),
            }],
            persona_turns: Vec::new(),
            question: None,
            refusal: None,
            narration: None,
            fault: None,
            state: PlayTurnState::Running,
        };
        assert_eq!(turn.dispatched_subjects(), Vec::new());
    }

    fn fake_persona_turn(prose: &str) -> PersonaTurn {
        PersonaTurn::record(
            PersonaTurnBinding {
                world_id: "world".into(),
                controller_id: "controller".into(),
                opportunity_digest: "opportunity".into(),
                world_revision: 0,
                scope_digest: "scope".into(),
                projector_receipt_digest: "projector".into(),
                persona_inference_receipt_digest: "persona".into(),
                interrupted_from: None,
            },
            prose,
        )
    }

    /// PA.f66d: the derived command id for one dispatched Persona's own turn
    /// depends on the dispatch call's own slot and the subject, never on the
    /// subject's position within that call's own JSON `subjects` array —
    /// two subjects one call names, or the same subject two calls in one
    /// round name, must never collide.
    #[test]
    fn dispatch_command_id_is_distinct_per_subject_and_call_and_stable_regardless_of_list_order() {
        let turn_id = test_turn_id(999);
        let a = dispatch_command_id_for_handle(&turn_id, 0, 5, "subject-a");
        let b = dispatch_command_id_for_handle(&turn_id, 0, 5, "subject-b");
        assert_ne!(
            format!("{a:?}"),
            format!("{b:?}"),
            "two subjects named by the same dispatch call must not collide"
        );
        let a_again = dispatch_command_id_for_handle(&turn_id, 0, 5, "subject-a");
        assert_eq!(
            format!("{a:?}"),
            format!("{a_again:?}"),
            "the same subject, round, and call slot must derive the same id"
        );
        let a_other_call = dispatch_command_id_for_handle(&turn_id, 0, 6, "subject-a");
        assert_ne!(
            format!("{a:?}"),
            format!("{a_other_call:?}"),
            "two dispatch calls in one round naming the same subject must not collide"
        );
    }

    /// PA.f104: `[dispatch(Mara), <mara>__speak, end_turn]` in one round must
    /// resolve the speak call identically whether it is executed straight
    /// through on a first pass, or resumed after a crash that landed only
    /// the dispatch call's own result. `dispatched` used to be snapshotted
    /// once before a round's own calls executed (before the dispatch call
    /// even ran), refusing the speak call on the first pass while a resume's
    /// fresh derivation — taken after the dispatch was already recorded —
    /// applied it. Mutation: reintroducing that per-round snapshot fails
    /// this by making the first-pass branch disagree with the resumed one.
    #[tokio::test]
    async fn a_same_round_dispatch_then_speak_agrees_on_first_pass_and_resume() {
        const PROSE: &str = "Mara considers the player.";
        const QUOTE: &str = "I nod once.";

        let first_pass = {
            let fixture = play_world(Some("Mara"), "player-f104-first-pass").await;
            let snapshot = fixture.world.snapshot().await.unwrap();
            let mara = handle_for(persona_id(&snapshot));
            let mara_id = subject_id_text(&snapshot, persona_id(&snapshot));
            let round = output(
                "r0",
                vec![
                    call_event("c0", DISPATCH_TOOL, serde_json::json!({"subjects": [mara_id]})),
                    call_event(
                        "c1",
                        &format!("{mara}{HANDLE_SEPARATOR}speak"),
                        serde_json::json!({"text": QUOTE}),
                    ),
                    call_event("c2", END_TURN_TOOL, serde_json::json!({})),
                ],
            );
            let personas = PersonaLane::new(
                ControllerPort::new(fixture.world.clone()),
                // Two inference calls per dispatch: the Projector (`PROSE`)
                // then the Persona itself, whose own output — `QUOTE` here —
                // becomes `PersonaTurn::source_prose()`, the text the speak
                // call's own quote is checked against.
                ScriptedPort::new(vec![output("proj-mara", vec![text_event(PROSE)]), output("persona-mara", vec![text_event(QUOTE)])]),
                "gpt-5.6-sol".into(),
                "gpt-5.6-sol".into(),
            )
            .unwrap();
            let directory = tempfile::tempdir().unwrap();
            let table = PlayTable::new(
                fixture.world.clone(),
                personas,
                ScriptedPort::new(vec![round]),
                "gpt-5.6-terra".into(),
                Arc::new(Semaphore::new(2)),
                directory.path().join("play-turn-v1.cc"),
            )
            .unwrap();
            table
                .run(&fixture.principal, test_turn_id(740), "Who's there?".into())
                .await
                .unwrap();
            let stored = table.store.lock().await;
            let turn = stored.current().unwrap();
            let speak_result = turn
                .calls
                .iter()
                .find(|call| call.round == 0 && call.slot == 1)
                .and_then(|call| call.result.clone())
                .unwrap();
            drop(stored);
            let after = fixture.world.snapshot().await.unwrap();
            (speak_result, after.revision)
        };

        let resumed = {
            let fixture = play_world(Some("Mara"), "player-f104-resume").await;
            let snapshot = fixture.world.snapshot().await.unwrap();
            let mara_subject = persona_id(&snapshot);
            let mara = handle_for(mara_subject);
            let round = output(
                "r0",
                vec![
                    call_event(
                        "c0",
                        DISPATCH_TOOL,
                        serde_json::json!({"subjects": [subject_id_text(&snapshot, mara_subject)]}),
                    ),
                    call_event(
                        "c1",
                        &format!("{mara}{HANDLE_SEPARATOR}speak"),
                        serde_json::json!({"text": QUOTE}),
                    ),
                    call_event("c2", END_TURN_TOOL, serde_json::json!({})),
                ],
            );
            let turn_id = test_turn_id(741);
            // The state a crash right after the dispatch call resolved, but
            // before the speak call did, would have left on disk: the
            // dispatch call already carries its own recorded body and
            // result, and its Persona turn is already recorded — exactly
            // what `execute_dispatch`/`record_call`/`persist` would have
            // written, never a value this test reconstructs by re-deriving
            // anything `dispatched_subjects` itself computes.
            let crashed_turn = PlayTurn {
                turn_id: turn_id.clone(),
                applied_keys: vec![test_turn_id(742)],
                opening_prompt: format!(
                    "{}\n\nThe player writes:\n{}\n",
                    table_view(&snapshot),
                    "Who's there?"
                ),
                player_prose: vec!["Who's there?".into()],
                rounds: vec![round],
                calls: vec![CallRecord {
                    call_id: "c0".into(),
                    round: 0,
                    slot: 0,
                    body: Some(RecordedCall::Dispatch(vec![mara_subject])),
                    result: Some(format!("Mara: {QUOTE}")),
                }],
                // The Persona's own recorded turn: its `source_prose()` is
                // `QUOTE` (the actual Persona response), matching what a
                // real `execute_dispatch` would have written, and what the
                // speak call below quotes from.
                persona_turns: vec![(mara_subject, fake_persona_turn(QUOTE))],
                question: None,
                refusal: None,
                narration: None,
                fault: None,
                state: PlayTurnState::Running,
            };
            let directory = tempfile::tempdir().unwrap();
            let store_path = directory.path().join("play-turn-v1.cc");
            {
                let mut store = PlayTurnStore::open(&store_path).unwrap();
                store.open_new_turn(crashed_turn).unwrap();
            }
            let personas = PersonaLane::new(
                ControllerPort::new(fixture.world.clone()),
                ScriptedPort::new(vec![]),
                "gpt-5.6-sol".into(),
                "gpt-5.6-sol".into(),
            )
            .unwrap();
            let table = PlayTable::new(
                fixture.world.clone(),
                personas,
                ScriptedPort::new(vec![]),
                "gpt-5.6-terra".into(),
                Arc::new(Semaphore::new(2)),
                &store_path,
            )
            .unwrap();
            table
                .run(&fixture.principal, test_turn_id(743), String::new().into())
                .await
                .unwrap();
            let stored = table.store.lock().await;
            let turn = stored.current().unwrap();
            let speak_result = turn
                .calls
                .iter()
                .find(|call| call.round == 0 && call.slot == 1)
                .and_then(|call| call.result.clone())
                .unwrap();
            drop(stored);
            let after = fixture.world.snapshot().await.unwrap();
            (speak_result, after.revision)
        };

        assert_eq!(first_pass.0, "applied", "first pass: {}", first_pass.0);
        assert_eq!(
            first_pass, resumed,
            "the same round script must resolve the speak call and the world revision identically \
             on a first pass and on resume"
        );
    }

    // --- PA.f99: the persisted body is on disk before submission ----------
    //
    // Each test below installs `PlayTable`'s own `#[cfg(test)]`
    // `before_submit_hook` (see its doc comment): fired right before the one
    // production call that submits a call's already-decoded body, it hands
    // the hook a fresh `pull_all` against the store's own already-owned
    // redb `Database` (`PlayTurnStore::pull_current_from_disk_for_test`) —
    // never the store's cached `self.state`, which a mutation could leave
    // stale relative to disk, and never a second, independently opened
    // store: CultCache's own single-owner lock refuses a second
    // `OwnedRedbMessagePackBackingStore::new` on the same path outright
    // (discovered building this very seam — see the method's own doc
    // comment). Each test then asserts the exact recorded body is already
    // durably there. Deleting the `persist` call at any of the three record
    // sites, or moving it to after its own submit call, must fail the
    // matching test here.

    #[tokio::test]
    async fn the_authoring_patch_is_persisted_before_it_is_submitted() {
        let fixture = play_world(None, "player-f99-authoring").await;
        let directory = tempfile::tempdir().unwrap();
        let store_path = directory.path().join("play-turn-v1.cc");
        let round = output(
            "r0",
            vec![
                call_event(
                    "c0",
                    "declare_resource",
                    serde_json::json!({"handle": "silver", "label": "Silver"}),
                ),
                call_event("c1", END_TURN_TOOL, serde_json::json!({})),
            ],
        );
        let personas = PersonaLane::new(
            ControllerPort::new(fixture.world.clone()),
            ScriptedPort::new(vec![output("proj-0", vec![text_event("Quiet.")])]),
            "gpt-5.6-sol".into(),
            "gpt-5.6-sol".into(),
        )
        .unwrap();
        let table = PlayTable::new(
            fixture.world.clone(),
            personas,
            ScriptedPort::new(vec![round]),
            "gpt-5.6-terra".into(),
            Arc::new(Semaphore::new(2)),
            &store_path,
        )
        .unwrap();
        table.set_before_submit_hook(move |durable_turn| {
            let turn = durable_turn.expect("a turn is durably on disk at submission time");
            let recorded = turn
                .calls
                .iter()
                .find(|call| call.round == 0 && call.slot == 0)
                .and_then(|call| call.body.clone());
            assert!(
                matches!(recorded, Some(RecordedCall::Authoring(_))),
                "the authoring patch must already be on disk when submission happens: {recorded:?}"
            );
        });
        table
            .run(&fixture.principal, test_turn_id(750), "Declare silver.".into())
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn the_advance_time_minutes_are_persisted_before_they_are_submitted() {
        let fixture = play_world(None, "player-f99-advance").await;
        let directory = tempfile::tempdir().unwrap();
        let store_path = directory.path().join("play-turn-v1.cc");
        let round = output(
            "r0",
            vec![
                call_event("c0", ADVANCE_TIME_TOOL, serde_json::json!({"minutes": 5})),
                call_event("c1", END_TURN_TOOL, serde_json::json!({})),
            ],
        );
        let personas = PersonaLane::new(
            ControllerPort::new(fixture.world.clone()),
            ScriptedPort::new(vec![output("proj-0", vec![text_event("Quiet.")])]),
            "gpt-5.6-sol".into(),
            "gpt-5.6-sol".into(),
        )
        .unwrap();
        let table = PlayTable::new(
            fixture.world.clone(),
            personas,
            ScriptedPort::new(vec![round]),
            "gpt-5.6-terra".into(),
            Arc::new(Semaphore::new(2)),
            &store_path,
        )
        .unwrap();
        table.set_before_submit_hook(move |durable_turn| {
            let turn = durable_turn.expect("a turn is durably on disk at submission time");
            let recorded = turn
                .calls
                .iter()
                .find(|call| call.round == 0 && call.slot == 0)
                .and_then(|call| call.body.clone());
            assert!(
                matches!(recorded, Some(RecordedCall::AdvanceTime(_))),
                "the advance-time minutes must already be on disk when submission happens: {recorded:?}"
            );
        });
        table
            .run(&fixture.principal, test_turn_id(751), "Advance time.".into())
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn the_actor_calls_invocation_is_persisted_before_it_is_submitted() {
        let fixture = play_world(None, "player-f99-actor").await;
        let snapshot = fixture.world.snapshot().await.unwrap();
        let player = handle_for(player_id(&snapshot));
        let directory = tempfile::tempdir().unwrap();
        let store_path = directory.path().join("play-turn-v1.cc");
        let round = output(
            "r0",
            vec![
                call_event(
                    "c0",
                    &format!("{player}{HANDLE_SEPARATOR}speak"),
                    serde_json::json!({"text": "hold fast"}),
                ),
                call_event("c1", END_TURN_TOOL, serde_json::json!({})),
            ],
        );
        let personas = PersonaLane::new(
            ControllerPort::new(fixture.world.clone()),
            ScriptedPort::new(vec![output("proj-0", vec![text_event("Quiet.")])]),
            "gpt-5.6-sol".into(),
            "gpt-5.6-sol".into(),
        )
        .unwrap();
        let table = PlayTable::new(
            fixture.world.clone(),
            personas,
            ScriptedPort::new(vec![round]),
            "gpt-5.6-terra".into(),
            Arc::new(Semaphore::new(2)),
            &store_path,
        )
        .unwrap();
        table.set_before_submit_hook(move |durable_turn| {
            let turn = durable_turn.expect("a turn is durably on disk at submission time");
            let recorded = turn
                .calls
                .iter()
                .find(|call| call.round == 0 && call.slot == 0)
                .and_then(|call| call.body.clone());
            assert!(
                matches!(recorded, Some(RecordedCall::PlayerAct(_, _))),
                "the actor call's own invocation must already be on disk when submission happens: {recorded:?}"
            );
        });
        table
            .run(&fixture.principal, test_turn_id(752), "hold fast".into())
            .await
            .unwrap();
    }

    /// PA.f62/PA.f63: an answer under a fresh request key resumes the
    /// waiting turn — already exercised end to end by
    /// `a_question_holds_the_turn_open_across_a_restart`, which now uses a
    /// distinct key for its own answer.
    ///
    /// This test covers the sibling rule: replaying that same answer key a
    /// second time, while the turn it answered is still open (it asks a
    /// second question), is a no-op — it must not re-infer or re-answer.
    #[tokio::test]
    async fn a_replayed_answer_key_is_a_no_op_while_the_turn_is_still_open() {
        let fixture = play_world(None, "player-replay-open").await;
        let directory = tempfile::tempdir().unwrap();
        let store_path = directory.path().join("play-turn-v1.cc");

        let ask_one = output(
            "r0",
            vec![call_event(
                "c0",
                ASK_PLAYER_TOOL,
                serde_json::json!({"question": "Which way?"}),
            )],
        );
        let ask_two = output(
            "r1",
            vec![call_event(
                "c1",
                ASK_PLAYER_TOOL,
                serde_json::json!({"question": "Are you sure?"}),
            )],
        );
        let personas = PersonaLane::new(
            ControllerPort::new(fixture.world.clone()),
            ScriptedPort::new(vec![]),
            "gpt-5.6-sol".into(),
            "gpt-5.6-sol".into(),
        )
        .unwrap();
        let table = PlayTable::new(
            fixture.world.clone(),
            personas,
            ScriptedPort::new(vec![ask_one, ask_two]),
            "gpt-5.6-terra".into(),
            Arc::new(Semaphore::new(2)),
            &store_path,
        )
        .unwrap();

        table
            .run(&fixture.principal, test_turn_id(450), "start".into())
            .await
            .unwrap();
        let first_question = table.store.lock().await.current().unwrap().open_question_id().unwrap();
        let answer_key = test_turn_id(451);
        let answer = || PlayRequest {
            text: "left".into(),
            answers: Some(first_question.clone()),
        };
        table.run(&fixture.principal, answer_key.clone(), answer()).await.unwrap();

        // The inference queue is now exhausted: a broken replay that treats
        // this key as unseen would try to answer round 1's own question and
        // infer a third round, failing loudly. A correct no-op leaves round
        // 1's question exactly as it was.
        table.run(&fixture.principal, answer_key, answer()).await.unwrap();

        let stored = table.store.lock().await;
        let turn = stored.current().unwrap();
        assert_eq!(turn.state, PlayTurnState::AwaitingPlayer);
        assert_eq!(turn.question.as_deref(), Some("Are you sure?"));
        assert!(turn.fault.is_none(), "a replayed key must never touch inference: {:?}", turn.fault);
    }

    /// PA.f62/PA.f63: a replayed key from a closed turn returns that turn's
    /// state and makes zero new inference calls.
    #[tokio::test]
    async fn a_replayed_key_from_a_closed_turn_makes_zero_inference_calls() {
        let fixture = play_world(None, "player-replay-closed").await;
        let directory = tempfile::tempdir().unwrap();
        let store_path = directory.path().join("play-turn-v1.cc");
        let key = test_turn_id(460);
        let end = output("r0", vec![call_event("c0", END_TURN_TOOL, serde_json::json!({}))]);
        let personas = PersonaLane::new(
            ControllerPort::new(fixture.world.clone()),
            ScriptedPort::new(vec![output("proj", vec![text_event("Fine.")])]),
            "gpt-5.6-sol".into(),
            "gpt-5.6-sol".into(),
        )
        .unwrap();
        // Exactly one output: a second inference attempt would exhaust it.
        let port = ScriptedPort::new(vec![end]);
        let table = PlayTable::new(
            fixture.world.clone(),
            personas,
            port.clone(),
            "gpt-5.6-terra".into(),
            Arc::new(Semaphore::new(2)),
            &store_path,
        )
        .unwrap();
        table
            .run(&fixture.principal, key.clone(), "Go.".into())
            .await
            .unwrap();
        assert_eq!(port.seen_requests().len(), 1);

        table
            .run(&fixture.principal, key.clone(), "Go.".into())
            .await
            .unwrap();

        assert_eq!(
            port.seen_requests().len(),
            1,
            "a replayed key from a closed turn must make zero new inference calls"
        );
        let stored = table.store.lock().await;
        let turn = stored.current().unwrap();
        assert!(turn.applied_keys.contains(&key));
        assert_eq!(turn.state, PlayTurnState::Closed);
    }

    /// PA.f63 + PA.f69 S7 + invariant 1: a genuinely new key after a closed
    /// turn opens a fresh turn under that key, and nothing from the closed
    /// turn's own record crosses into it — checked against the actual
    /// inference request the fresh turn's own round sent, not merely the
    /// stored `opening_prompt`.
    #[tokio::test]
    async fn a_new_key_after_a_closed_turn_starts_a_fresh_turn_with_no_prior_transcript() {
        let fixture = play_world(None, "player-fresh-after-close").await;
        let directory = tempfile::tempdir().unwrap();
        let store_path = directory.path().join("play-turn-v1.cc");

        {
            let personas = PersonaLane::new(
                ControllerPort::new(fixture.world.clone()),
                ScriptedPort::new(vec![output("proj-1", vec![text_event("Fine.")])]),
                "gpt-5.6-sol".into(),
                "gpt-5.6-sol".into(),
            )
            .unwrap();
            let end = output("r0", vec![call_event("c0", END_TURN_TOOL, serde_json::json!({}))]);
            let table = PlayTable::new(
                fixture.world.clone(),
                personas,
                ScriptedPort::new(vec![end]),
                "gpt-5.6-terra".into(),
                Arc::new(Semaphore::new(2)),
                &store_path,
            )
            .unwrap();
            table
                .run(&fixture.principal, test_turn_id(470), "UNIQUE_TURN_ONE_PROSE".into())
                .await
                .unwrap();
        }

        let personas2 = PersonaLane::new(
            ControllerPort::new(fixture.world.clone()),
            ScriptedPort::new(vec![output("proj-2", vec![text_event("Also fine.")])]),
            "gpt-5.6-sol".into(),
            "gpt-5.6-sol".into(),
        )
        .unwrap();
        let end2 = output("r0", vec![call_event("c0", END_TURN_TOOL, serde_json::json!({}))]);
        let port = ScriptedPort::new(vec![end2]);
        let table = PlayTable::new(
            fixture.world.clone(),
            personas2,
            port.clone(),
            "gpt-5.6-terra".into(),
            Arc::new(Semaphore::new(2)),
            &store_path,
        )
        .unwrap();
        let fresh_key = test_turn_id(471);
        table
            .run(&fixture.principal, fresh_key.clone(), "UNIQUE_TURN_TWO_PROSE".into())
            .await
            .unwrap();

        let stored = table.store.lock().await;
        let turn = stored.current().unwrap();
        assert!(
            turn.applied_keys.contains(&fresh_key),
            "a new key after a closed turn must open a fresh turn recording that key"
        );
        assert!(turn.opening_prompt.contains("UNIQUE_TURN_TWO_PROSE"));
        assert!(
            !turn.opening_prompt.contains("UNIQUE_TURN_ONE_PROSE"),
            "no transcript may cross from a closed turn into a new one"
        );

        let seen = port.seen_requests();
        let request_text = format!("{:?}", seen.first().unwrap());
        assert!(
            !request_text.contains("UNIQUE_TURN_ONE_PROSE"),
            "the new turn's own request must carry nothing from the closed turn: {request_text}"
        );
    }

    // --- Fix batch 8a-fix2 (PA.f83-PA.f97) --------------------------------

    /// PA.f83: two consecutive turns mint distinct `turn_id`s (never taken
    /// from either request key), and their derived command ids never
    /// collide.
    #[tokio::test]
    async fn two_consecutive_turns_mint_distinct_ids_and_non_colliding_command_ids() {
        let fixture = play_world(None, "player-distinct-turn-ids").await;
        let directory = tempfile::tempdir().unwrap();
        let store_path = directory.path().join("play-turn-v1.cc");

        let personas1 = PersonaLane::new(
            ControllerPort::new(fixture.world.clone()),
            ScriptedPort::new(vec![output("proj-1", vec![text_event("Fine.")])]),
            "gpt-5.6-sol".into(),
            "gpt-5.6-sol".into(),
        )
        .unwrap();
        let end1 = output("r0", vec![call_event("c0", END_TURN_TOOL, serde_json::json!({}))]);
        let table = PlayTable::new(
            fixture.world.clone(),
            personas1,
            ScriptedPort::new(vec![end1]),
            "gpt-5.6-terra".into(),
            Arc::new(Semaphore::new(2)),
            &store_path,
        )
        .unwrap();
        table.run(&fixture.principal, test_turn_id(600), "Turn one.".into()).await.unwrap();
        let turn1_id = table.store.lock().await.current().unwrap().turn_id.clone();
        drop(table);

        let personas2 = PersonaLane::new(
            ControllerPort::new(fixture.world.clone()),
            ScriptedPort::new(vec![output("proj-2", vec![text_event("Fine.")])]),
            "gpt-5.6-sol".into(),
            "gpt-5.6-sol".into(),
        )
        .unwrap();
        let end2 = output("r0", vec![call_event("c0", END_TURN_TOOL, serde_json::json!({}))]);
        let table = PlayTable::new(
            fixture.world.clone(),
            personas2,
            ScriptedPort::new(vec![end2]),
            "gpt-5.6-terra".into(),
            Arc::new(Semaphore::new(2)),
            &store_path,
        )
        .unwrap();
        table.run(&fixture.principal, test_turn_id(601), "Turn two.".into()).await.unwrap();
        let turn2_id = table.store.lock().await.current().unwrap().turn_id.clone();

        assert_ne!(turn1_id, turn2_id, "each turn mints its own fresh turn_id (PA.f83)");
        let turn1_command0 = derived_command_id(&turn1_id, 0, 0);
        let turn2_command0 = derived_command_id(&turn2_id, 0, 0);
        assert_ne!(
            format!("{turn1_command0:?}"),
            format!("{turn2_command0:?}"),
            "command ids derived from distinct turn_ids must not collide"
        );
    }

    fn minimal_closed_turn(turn_id: String, keys: Vec<String>) -> PlayTurn {
        PlayTurn {
            turn_id,
            applied_keys: keys,
            opening_prompt: String::new(),
            player_prose: Vec::new(),
            rounds: Vec::new(),
            calls: Vec::new(),
            persona_turns: Vec::new(),
            question: None,
            refusal: None,
            narration: None,
            fault: None,
            state: PlayTurnState::Closed,
        }
    }

    /// PA.f100: `turn_id` is minted fresh (`Uuid::new_v4`), never derived
    /// from the request key — the same key, once evicted from the store's
    /// own `KeyLedger` window and so no longer recognized as a replay, opens
    /// a genuinely new turn with a fresh id each time. Mutation: deriving
    /// `turn_id` as a hash of `key` fails this, since both opens below share
    /// exactly one key and a hash of one value cannot differ from itself.
    #[tokio::test]
    async fn the_same_evicted_key_opens_two_turns_with_distinct_fresh_ids() {
        let fixture = play_world(None, "player-f100-evicted-key").await;
        let directory = tempfile::tempdir().unwrap();
        let store_path = directory.path().join("play-turn-v1.cc");
        let key = "reused-opening-key".to_owned();

        let personas1 = PersonaLane::new(
            ControllerPort::new(fixture.world.clone()),
            ScriptedPort::new(vec![output("proj-1", vec![text_event("Fine.")])]),
            "gpt-5.6-sol".into(),
            "gpt-5.6-sol".into(),
        )
        .unwrap();
        let end1 = output("r0", vec![call_event("c0", END_TURN_TOOL, serde_json::json!({}))]);
        let table = PlayTable::new(
            fixture.world.clone(),
            personas1,
            ScriptedPort::new(vec![end1]),
            "gpt-5.6-terra".into(),
            Arc::new(Semaphore::new(2)),
            &store_path,
        )
        .unwrap();
        table.run(&fixture.principal, key.clone(), "Turn one.".into()).await.unwrap();
        let turn1_id = table.store.lock().await.current().unwrap().turn_id.clone();

        // Evict `key` from the ledger: turn one is already the store's
        // current (closed) turn, so the first of `KEY_LEDGER_WINDOW + 1`
        // more closed turns, written directly through the store (no
        // inference needed to close an already-`Closed` fixture turn),
        // archives it as the oldest entry, filling the window exactly; the
        // one past that finally pushes it out (matching
        // `the_ledger_forgets_a_closed_turns_keys_past_the_window`'s own
        // window-plus-one shape against the pure `KeyLedger` value).
        {
            let mut store = table.store.lock().await;
            for index in 0..=KEY_LEDGER_WINDOW {
                store
                    .open_new_turn(minimal_closed_turn(format!("filler-{index}"), vec![format!("filler-key-{index}")]))
                    .unwrap();
            }
            assert!(
                store.turn_id_for_key(&key).is_none(),
                "the original key must be evicted past the window before the second open"
            );
        }
        drop(table);

        let personas2 = PersonaLane::new(
            ControllerPort::new(fixture.world.clone()),
            ScriptedPort::new(vec![output("proj-2", vec![text_event("Fine.")])]),
            "gpt-5.6-sol".into(),
            "gpt-5.6-sol".into(),
        )
        .unwrap();
        let end2 = output("r0", vec![call_event("c0", END_TURN_TOOL, serde_json::json!({}))]);
        let table = PlayTable::new(
            fixture.world.clone(),
            personas2,
            ScriptedPort::new(vec![end2]),
            "gpt-5.6-terra".into(),
            Arc::new(Semaphore::new(2)),
            &store_path,
        )
        .unwrap();
        let outcome = table.run(&fixture.principal, key.clone(), "Turn two.".into()).await.unwrap();
        assert_eq!(outcome, RunOutcome::Ran, "the evicted key must open a fresh turn, not replay");
        let turn2_id = table.store.lock().await.current().unwrap().turn_id.clone();

        assert_ne!(
            turn1_id, turn2_id,
            "the same request key, reused only after eviction, must still mint a fresh turn_id each open"
        );
    }

    /// PA.f83: K1 opens and closes a turn; K2 opens and closes a second
    /// turn; a retry of K1 is a no-op recognized through the store's own
    /// `KeyLedger` — zero inference calls, and it does not open a third
    /// turn.
    #[tokio::test]
    async fn a_retried_key_from_a_turn_before_the_most_recent_is_a_no_op() {
        let fixture = play_world(None, "player-ledger-retry").await;
        let directory = tempfile::tempdir().unwrap();
        let store_path = directory.path().join("play-turn-v1.cc");
        let end = || output("r0", vec![call_event("c0", END_TURN_TOOL, serde_json::json!({}))]);
        fn personas(world: &WorldMailbox, proj: &'static str) -> PersonaLane {
            PersonaLane::new(
                ControllerPort::new(world.clone()),
                ScriptedPort::new(vec![output(proj, vec![text_event("Fine.")])]),
                "gpt-5.6-sol".into(),
                "gpt-5.6-sol".into(),
            )
            .unwrap()
        }

        let k1 = test_turn_id(610);
        {
            let table = PlayTable::new(
                fixture.world.clone(),
                personas(&fixture.world, "proj-k1"),
                ScriptedPort::new(vec![end()]),
                "gpt-5.6-terra".into(),
                Arc::new(Semaphore::new(2)),
                &store_path,
            )
            .unwrap();
            table.run(&fixture.principal, k1.clone(), "K1.".into()).await.unwrap();
        }
        let turn1_id = PlayTurnStore::open(&store_path).unwrap().current().unwrap().turn_id.clone();
        let k2 = test_turn_id(611);
        {
            let table = PlayTable::new(
                fixture.world.clone(),
                personas(&fixture.world, "proj-k2"),
                ScriptedPort::new(vec![end()]),
                "gpt-5.6-terra".into(),
                Arc::new(Semaphore::new(2)),
                &store_path,
            )
            .unwrap();
            table.run(&fixture.principal, k2.clone(), "K2.".into()).await.unwrap();
        }
        let turn2_id = PlayTurnStore::open(&store_path).unwrap().current().unwrap().turn_id.clone();

        // A retry of K1: an empty scripted port means any inference attempt
        // fails loudly, so this stays a genuine "zero inference calls" check.
        let port = ScriptedPort::new(vec![]);
        let table = PlayTable::new(
            fixture.world.clone(),
            personas(&fixture.world, "proj-k1-retry"),
            port.clone(),
            "gpt-5.6-terra".into(),
            Arc::new(Semaphore::new(2)),
            &store_path,
        )
        .unwrap();
        let outcome = table.run(&fixture.principal, k1, "K1.".into()).await.unwrap();

        assert_eq!(
            port.seen_requests().len(),
            0,
            "a retried key already known to the ledger must make zero inference calls"
        );
        let stored = table.store.lock().await;
        let turn = stored.current().unwrap();
        assert_eq!(turn.turn_id, turn2_id, "the retry must not open a third turn");
        // PA.f109: the replay's own outcome names the turn it replayed —
        // K1's own turn, not K2's, the store's current one — reading
        // `ClosedTurnKeys::turn_id` for the first time anywhere in
        // production.
        assert_eq!(
            outcome,
            RunOutcome::Replayed { turn_id: turn1_id },
            "a replayed key's own outcome must name the turn it originally opened"
        );
    }

    /// PA.f83: the ledger keeps a closed turn's own keys only while it
    /// remains among the most recent `KEY_LEDGER_WINDOW` closed turns; one
    /// more closed turn past the window evicts the oldest one's keys. A
    /// small helper against the pure `KeyLedger` value directly, so this
    /// does not run 65 real turns.
    #[test]
    fn the_ledger_forgets_a_closed_turns_keys_past_the_window() {
        let mut ledger = KeyLedger::default();
        ledger.push_closed("oldest-turn".into(), vec!["oldest-key".into()]);
        for index in 0..KEY_LEDGER_WINDOW - 1 {
            ledger.push_closed(format!("turn-{index}"), vec![format!("key-{index}")]);
        }
        assert!(
            ledger.contains("oldest-key"),
            "the oldest turn's own key must still be known while it is within the window"
        );
        // The 65th closed turn: exactly one past the window, evicting the
        // oldest.
        ledger.push_closed("one-too-many".into(), vec!["newest-key".into()]);
        assert!(!ledger.contains("oldest-key"), "a key older than the window must be forgotten");
        assert!(ledger.contains("newest-key"));
    }

    /// PA.f84 probe: K1 opens the turn and asks Q1; K2 answers Q1 and the
    /// next round asks Q2; a retry of K1 is recognized as a replay and is a
    /// no-op — it does not, and cannot, answer Q2.
    #[tokio::test]
    async fn a_retried_opening_key_never_answers_a_later_question() {
        let fixture = play_world(None, "player-probe-late-retry").await;
        let directory = tempfile::tempdir().unwrap();
        let store_path = directory.path().join("play-turn-v1.cc");
        let ask_one = output(
            "r0",
            vec![call_event("c0", ASK_PLAYER_TOOL, serde_json::json!({"question": "Which way?"}))],
        );
        let ask_two = output(
            "r1",
            vec![call_event("c1", ASK_PLAYER_TOOL, serde_json::json!({"question": "Are you sure?"}))],
        );
        let personas = PersonaLane::new(
            ControllerPort::new(fixture.world.clone()),
            ScriptedPort::new(vec![]),
            "gpt-5.6-sol".into(),
            "gpt-5.6-sol".into(),
        )
        .unwrap();
        let port = ScriptedPort::new(vec![ask_one, ask_two]);
        let table = PlayTable::new(
            fixture.world.clone(),
            personas,
            port.clone(),
            "gpt-5.6-terra".into(),
            Arc::new(Semaphore::new(2)),
            &store_path,
        )
        .unwrap();

        let k1 = test_turn_id(630);
        // K1 closes: the turn is now `AwaitingPlayer` on Q1.
        table.run(&fixture.principal, k1.clone(), "start".into()).await.unwrap();
        let q1 = table.store.lock().await.current().unwrap().open_question_id().unwrap();
        let k2 = test_turn_id(631);
        // K2 asks a question: it answers Q1, and the next round asks Q2.
        table
            .run(
                &fixture.principal,
                k2,
                PlayRequest {
                    text: "left".into(),
                    answers: Some(q1),
                },
            )
            .await
            .unwrap();
        assert_eq!(table.store.lock().await.current().unwrap().question.as_deref(), Some("Are you sure?"));
        let seen_before_retry = port.seen_requests().len();

        // K1 is retried: it opened the turn and is already known, so this is
        // a no-op.
        table.run(&fixture.principal, k1, "left".into()).await.unwrap();

        assert_eq!(
            port.seen_requests().len(),
            seen_before_retry,
            "a retried key must make zero new inference calls"
        );
        let stored = table.store.lock().await;
        let turn = stored.current().unwrap();
        assert_eq!(turn.state, PlayTurnState::AwaitingPlayer);
        assert_eq!(turn.question.as_deref(), Some("Are you sure?"), "K1's own retry must not answer Q2");
    }

    /// PA.f84: an answer naming a different, no-longer-open question is
    /// refused as stale rather than being applied to whatever question is
    /// open now.
    #[tokio::test]
    async fn an_answer_naming_a_different_question_is_refused_as_stale() {
        let fixture = play_world(None, "player-stale-answer").await;
        let directory = tempfile::tempdir().unwrap();
        let store_path = directory.path().join("play-turn-v1.cc");
        let ask_one = output(
            "r0",
            vec![call_event("c0", ASK_PLAYER_TOOL, serde_json::json!({"question": "Which way?"}))],
        );
        let ask_two = output(
            "r1",
            vec![call_event("c1", ASK_PLAYER_TOOL, serde_json::json!({"question": "Are you sure?"}))],
        );
        let personas = PersonaLane::new(
            ControllerPort::new(fixture.world.clone()),
            ScriptedPort::new(vec![]),
            "gpt-5.6-sol".into(),
            "gpt-5.6-sol".into(),
        )
        .unwrap();
        let table = PlayTable::new(
            fixture.world.clone(),
            personas,
            ScriptedPort::new(vec![ask_one, ask_two]),
            "gpt-5.6-terra".into(),
            Arc::new(Semaphore::new(2)),
            &store_path,
        )
        .unwrap();

        table.run(&fixture.principal, test_turn_id(620), "start".into()).await.unwrap();
        let stale_question = table.store.lock().await.current().unwrap().open_question_id().unwrap();
        table
            .run(
                &fixture.principal,
                test_turn_id(621),
                PlayRequest {
                    text: "left".into(),
                    answers: Some(stale_question.clone()),
                },
            )
            .await
            .unwrap();

        // Q2 is now open; a fresh key answering with Q1's own (now stale)
        // question id must be refused, not silently applied to Q2.
        let error = table
            .run(
                &fixture.principal,
                test_turn_id(622),
                PlayRequest {
                    text: "sure".into(),
                    answers: Some(stale_question),
                },
            )
            .await
            .unwrap_err();
        assert!(matches!(error, PlayError::StaleAnswer), "{error:?}");
        let stored = table.store.lock().await;
        let turn = stored.current().unwrap();
        assert_eq!(
            turn.question.as_deref(),
            Some("Are you sure?"),
            "the stale answer must not touch the currently open question"
        );
        assert!(
            !turn.applied_keys.contains(&test_turn_id(622)),
            "a refused stale answer's key must never be marked applied"
        );
    }

    /// PA.f101: `answers: None` while a question genuinely is open is
    /// refused exactly like a stale, wrongly-named one — the match at
    /// `run`'s own open-question arm only ever applies the `(Some(given),
    /// Some(open)) if given == open` case; every other combination,
    /// including `None`, falls to the same `StaleAnswer` refusal. Mutation:
    /// accepting `None` there (treating an absent answer as "answer
    /// whatever is open") fails this, since nothing here supplied any text
    /// naming an actual answer.
    #[tokio::test]
    async fn an_answer_of_none_while_a_question_is_open_is_refused_and_records_nothing() {
        let fixture = play_world(None, "player-none-answer").await;
        let directory = tempfile::tempdir().unwrap();
        let store_path = directory.path().join("play-turn-v1.cc");
        let ask = output(
            "r0",
            vec![call_event("c0", ASK_PLAYER_TOOL, serde_json::json!({"question": "Which way?"}))],
        );
        let personas = PersonaLane::new(
            ControllerPort::new(fixture.world.clone()),
            ScriptedPort::new(vec![]),
            "gpt-5.6-sol".into(),
            "gpt-5.6-sol".into(),
        )
        .unwrap();
        let table = PlayTable::new(
            fixture.world.clone(),
            personas,
            ScriptedPort::new(vec![ask]),
            "gpt-5.6-terra".into(),
            Arc::new(Semaphore::new(2)),
            &store_path,
        )
        .unwrap();
        table.run(&fixture.principal, test_turn_id(623), "start".into()).await.unwrap();
        let before = table.store.lock().await.current().unwrap().clone();
        assert_eq!(before.state, PlayTurnState::AwaitingPlayer);

        let error = table
            .run(
                &fixture.principal,
                test_turn_id(624),
                PlayRequest {
                    text: "left".into(),
                    answers: None,
                },
            )
            .await
            .unwrap_err();
        assert!(matches!(error, PlayError::StaleAnswer), "{error:?}");

        let stored = table.store.lock().await;
        let after = stored.current().unwrap();
        assert_eq!(after.state, PlayTurnState::AwaitingPlayer, "the turn must stay AwaitingPlayer");
        assert_eq!(after.question, before.question, "the open question must not change");
        assert_eq!(after.calls.len(), before.calls.len(), "nothing must be recorded from a refused None answer");
        assert!(
            !after.applied_keys.contains(&test_turn_id(624)),
            "a refused None answer's key must never be marked applied"
        );
    }

    /// PA.f84: `answers` given while the turn is `Running` (no question
    /// open at all) is refused.
    #[tokio::test]
    async fn an_answer_while_the_turn_is_running_is_refused() {
        let fixture = play_world(None, "player-answers-while-running").await;
        let directory = tempfile::tempdir().unwrap();
        let store_path = directory.path().join("play-turn-v1.cc");
        let turn_id = test_turn_id(640);
        let personas = PersonaLane::new(
            ControllerPort::new(fixture.world.clone()),
            ScriptedPort::new(vec![]),
            "gpt-5.6-sol".into(),
            "gpt-5.6-sol".into(),
        )
        .unwrap();
        let table = PlayTable::new(
            fixture.world.clone(),
            personas,
            ScriptedPort::new(vec![]),
            "gpt-5.6-terra".into(),
            Arc::new(Semaphore::new(2)),
            &store_path,
        )
        .unwrap();
        // A turn caught mid-loop, `Running`, between rounds — the exact
        // shape a real crash could leave persisted (`run` itself persists
        // right after each round with `RoundOutcome::Continue`, before its
        // own loop condition re-checks `turn.state`).
        let running_turn = PlayTurn {
            turn_id: turn_id.clone(),
            applied_keys: vec![test_turn_id(639)],
            opening_prompt: "opening".into(),
            player_prose: vec!["hello".into()],
            rounds: Vec::new(),
            calls: Vec::new(),
            persona_turns: Vec::new(),
            question: None,
            refusal: None,
            narration: None,
            fault: None,
            state: PlayTurnState::Running,
        };
        table.store.lock().await.commit(running_turn).unwrap();

        let bogus_question = QuestionId {
            turn_id,
            round: 0,
            slot: 0,
        };
        let error = table
            .run(
                &fixture.principal,
                test_turn_id(641),
                PlayRequest {
                    text: String::new(),
                    answers: Some(bogus_question),
                },
            )
            .await
            .unwrap_err();
        assert!(matches!(error, PlayError::NoQuestionOpen), "{error:?}");
    }

    /// PA.f84: `answers` given when there is no open turn at all is refused.
    #[tokio::test]
    async fn an_answer_with_no_open_turn_at_all_is_refused() {
        let fixture = play_world(None, "player-answers-no-turn").await;
        let directory = tempfile::tempdir().unwrap();
        let personas = PersonaLane::new(
            ControllerPort::new(fixture.world.clone()),
            ScriptedPort::new(vec![]),
            "gpt-5.6-sol".into(),
            "gpt-5.6-sol".into(),
        )
        .unwrap();
        let table = PlayTable::new(
            fixture.world.clone(),
            personas,
            ScriptedPort::new(vec![]),
            "gpt-5.6-terra".into(),
            Arc::new(Semaphore::new(2)),
            directory.path().join("play-turn-v1.cc"),
        )
        .unwrap();
        let bogus_question = QuestionId {
            turn_id: "nonexistent".into(),
            round: 0,
            slot: 0,
        };
        let error = table
            .run(
                &fixture.principal,
                test_turn_id(650),
                PlayRequest {
                    text: "hi".into(),
                    answers: Some(bogus_question),
                },
            )
            .await
            .unwrap_err();
        assert!(matches!(error, PlayError::NoQuestionOpen), "{error:?}");
    }

    /// PA.f84: while the turn is `Running`, non-empty text is refused
    /// rather than dropped silently — the turn is still resolving its own
    /// prior request's rounds.
    #[tokio::test]
    async fn nonempty_text_while_the_turn_is_running_is_refused() {
        let fixture = play_world(None, "player-turn-still-running").await;
        let directory = tempfile::tempdir().unwrap();
        let store_path = directory.path().join("play-turn-v1.cc");
        let turn_id = test_turn_id(660);
        let personas = PersonaLane::new(
            ControllerPort::new(fixture.world.clone()),
            ScriptedPort::new(vec![]),
            "gpt-5.6-sol".into(),
            "gpt-5.6-sol".into(),
        )
        .unwrap();
        let table = PlayTable::new(
            fixture.world.clone(),
            personas,
            ScriptedPort::new(vec![]),
            "gpt-5.6-terra".into(),
            Arc::new(Semaphore::new(2)),
            &store_path,
        )
        .unwrap();
        let running_turn = PlayTurn {
            turn_id: turn_id.clone(),
            applied_keys: vec![test_turn_id(659)],
            opening_prompt: "opening".into(),
            player_prose: vec!["hello".into()],
            rounds: Vec::new(),
            calls: Vec::new(),
            persona_turns: Vec::new(),
            question: None,
            refusal: None,
            narration: None,
            fault: None,
            state: PlayTurnState::Running,
        };
        table.store.lock().await.commit(running_turn).unwrap();

        let error = table.run(&fixture.principal, test_turn_id(661), "more text".into()).await.unwrap_err();
        assert!(matches!(error, PlayError::TurnStillRunning), "{error:?}");
        let stored = table.store.lock().await;
        let turn = stored.current().unwrap();
        assert_eq!(turn.state, PlayTurnState::Running);
        assert!(!turn.applied_keys.contains(&test_turn_id(661)));
    }

    /// PA.f84: while the turn is `Running`, empty text continues it.
    #[tokio::test]
    async fn empty_text_while_the_turn_is_running_continues_it() {
        let fixture = play_world(None, "player-turn-continues").await;
        let directory = tempfile::tempdir().unwrap();
        let store_path = directory.path().join("play-turn-v1.cc");
        let turn_id = test_turn_id(670);
        let personas = PersonaLane::new(
            ControllerPort::new(fixture.world.clone()),
            ScriptedPort::new(vec![output("proj", vec![text_event("Fine.")])]),
            "gpt-5.6-sol".into(),
            "gpt-5.6-sol".into(),
        )
        .unwrap();
        let end = output("r0", vec![call_event("c0", END_TURN_TOOL, serde_json::json!({}))]);
        let table = PlayTable::new(
            fixture.world.clone(),
            personas,
            ScriptedPort::new(vec![end]),
            "gpt-5.6-terra".into(),
            Arc::new(Semaphore::new(2)),
            &store_path,
        )
        .unwrap();
        let snapshot = fixture.world.snapshot().await.unwrap();
        let running_turn = PlayTurn {
            turn_id: turn_id.clone(),
            applied_keys: vec![test_turn_id(669)],
            opening_prompt: format!("{}\n\nThe player writes:\n{}\n", table_view(&snapshot), "hi"),
            player_prose: vec!["hi".into()],
            rounds: Vec::new(),
            calls: Vec::new(),
            persona_turns: Vec::new(),
            question: None,
            refusal: None,
            narration: None,
            fault: None,
            state: PlayTurnState::Running,
        };
        table.store.lock().await.commit(running_turn).unwrap();

        table.run(&fixture.principal, test_turn_id(671), String::new().into()).await.unwrap();

        let stored = table.store.lock().await;
        let turn = stored.current().unwrap();
        assert_eq!(turn.state, PlayTurnState::Closed);
        assert!(turn.applied_keys.contains(&test_turn_id(671)));
    }

    /// PA.f108: empty or whitespace-only text with no turn open to continue
    /// — no turn at all, or the current one already `Closed` — is refused
    /// before a fresh turn opens, rather than opening one with prose `[""]`
    /// and running a full inference over nothing the player said. Checked
    /// both with no stored turn at all and with a `Closed` one on record,
    /// and in each case zero inference calls are made and the store's own
    /// current turn is untouched.
    #[tokio::test]
    async fn empty_text_with_no_turn_open_is_refused_and_infers_nothing() {
        let fixture = play_world(None, "player-empty-opening").await;
        let directory = tempfile::tempdir().unwrap();
        let store_path = directory.path().join("play-turn-v1.cc");
        let play_port = ScriptedPort::new(vec![]);
        let personas = PersonaLane::new(
            ControllerPort::new(fixture.world.clone()),
            ScriptedPort::new(vec![]),
            "gpt-5.6-sol".into(),
            "gpt-5.6-sol".into(),
        )
        .unwrap();
        let table = PlayTable::new(
            fixture.world.clone(),
            personas,
            play_port.clone(),
            "gpt-5.6-terra".into(),
            Arc::new(Semaphore::new(2)),
            &store_path,
        )
        .unwrap();

        // No turn on record at all.
        let error = table
            .run(&fixture.principal, test_turn_id(672), String::new().into())
            .await
            .unwrap_err();
        assert!(matches!(error, PlayError::EmptyOpening), "{error:?}");
        assert!(table.store.lock().await.current().is_none(), "no turn must have opened");
        assert!(play_port.seen_requests().is_empty(), "an empty opening must make zero inference calls");

        // A whitespace-only text with a `Closed` turn already on record must
        // be refused the same way, not treated as a continue of the closed
        // turn or as a fresh (empty) opening.
        let snapshot = fixture.world.snapshot().await.unwrap();
        let closed_turn = PlayTurn {
            turn_id: test_turn_id(673),
            applied_keys: vec![test_turn_id(674)],
            opening_prompt: format!("{}\n\nThe player writes:\n{}\n", table_view(&snapshot), "done"),
            player_prose: vec!["done".into()],
            rounds: Vec::new(),
            calls: Vec::new(),
            persona_turns: Vec::new(),
            question: None,
            refusal: None,
            narration: Some("The end.".into()),
            fault: None,
            state: PlayTurnState::Closed,
        };
        table.store.lock().await.commit(closed_turn).unwrap();

        let error = table
            .run(&fixture.principal, test_turn_id(675), "   ".into())
            .await
            .unwrap_err();
        assert!(matches!(error, PlayError::EmptyOpening), "{error:?}");
        assert!(play_port.seen_requests().is_empty(), "still zero inference calls");
        let stored = table.store.lock().await;
        let turn = stored.current().unwrap();
        assert_eq!(turn.turn_id, test_turn_id(673), "the closed turn on record must be untouched");
        assert!(
            !turn.applied_keys.contains(&test_turn_id(675)),
            "a refused empty opening's key must never be marked applied"
        );
    }

    struct DispositionPort {
        queue: StdMutex<VecDeque<Result<InferenceOutput, InferenceFault>>>,
        calls: StdMutex<usize>,
    }

    impl DispositionPort {
        fn new(items: Vec<Result<InferenceOutput, InferenceFault>>) -> Arc<Self> {
            Arc::new(Self {
                queue: StdMutex::new(items.into_iter().collect()),
                calls: StdMutex::new(0),
            })
        }

        fn call_count(&self) -> usize {
            *self.calls.lock().unwrap()
        }
    }

    #[async_trait::async_trait]
    impl InferencePort for DispositionPort {
        fn prepare(&self, request: InferenceRequest) -> Result<PreparedInference, InferenceFault> {
            PreparedInference::prepare("play-test-runtime", far_future_unix_ms(), request)
        }

        async fn infer(&self, _prepared: PreparedInference) -> Result<InferenceOutput, InferenceFault> {
            *self.calls.lock().unwrap() += 1;
            match self.queue.lock().unwrap().pop_front() {
                Some(item) => item,
                None => Err(InferenceFault::recovery_required("disposition port exhausted")),
            }
        }
    }

    /// PA.f85/PA.f93/PA.f94: an `IntegrityViolation` fault never retries —
    /// exactly one inference call — and closes the turn with the fault
    /// recorded through `close_with_fault`, rather than being left
    /// `Running`. The next request then opens a fresh turn.
    #[tokio::test]
    async fn an_integrity_violation_makes_one_call_then_closes_and_the_next_request_opens_a_fresh_turn() {
        let fixture = play_world(None, "player-integrity-violation").await;
        let directory = tempfile::tempdir().unwrap();
        let store_path = directory.path().join("play-turn-v1.cc");
        let personas = PersonaLane::new(
            ControllerPort::new(fixture.world.clone()),
            ScriptedPort::new(vec![]),
            "gpt-5.6-sol".into(),
            "gpt-5.6-sol".into(),
        )
        .unwrap();
        let port = DispositionPort::new(vec![Err(InferenceFault::integrity_violation("bad tool schema"))]);
        let table = PlayTable::new(
            fixture.world.clone(),
            personas,
            port.clone(),
            "gpt-5.6-terra".into(),
            Arc::new(Semaphore::new(2)),
            &store_path,
        )
        .unwrap();

        table.run(&fixture.principal, test_turn_id(680), "Hello?".into()).await.unwrap();

        assert_eq!(port.call_count(), 1, "an IntegrityViolation must not retry");
        let closed_turn_id = {
            let stored = table.store.lock().await;
            let turn = stored.current().unwrap();
            assert_eq!(turn.state, PlayTurnState::Closed);
            assert!(turn.fault.as_deref().unwrap().contains("bad tool schema"), "{:?}", turn.fault);
            turn.turn_id.clone()
        };
        // Release the store's exclusive custody before reopening it below —
        // the same "restart" precedent `a_question_holds_the_turn_open_across_a_restart`
        // relies on.
        drop(table);

        // The next request opens a fresh turn rather than resuming the
        // fault-closed one.
        let personas2 = PersonaLane::new(
            ControllerPort::new(fixture.world.clone()),
            ScriptedPort::new(vec![output("proj-after", vec![text_event("Fine.")])]),
            "gpt-5.6-sol".into(),
            "gpt-5.6-sol".into(),
        )
        .unwrap();
        let end = output("r0", vec![call_event("c0", END_TURN_TOOL, serde_json::json!({}))]);
        let table2 = PlayTable::new(
            fixture.world.clone(),
            personas2,
            ScriptedPort::new(vec![end]),
            "gpt-5.6-terra".into(),
            Arc::new(Semaphore::new(2)),
            &store_path,
        )
        .unwrap();
        table2.run(&fixture.principal, test_turn_id(681), "Try again.".into()).await.unwrap();
        let stored = table2.store.lock().await;
        let turn = stored.current().unwrap();
        assert_ne!(turn.turn_id, closed_turn_id, "the next request must open a fresh turn, not resume the fault-closed one");
        assert_eq!(turn.state, PlayTurnState::Closed);
        assert!(turn.fault.is_none());
    }

    /// PA.f94: a `Retryable` fault retries and, on success, the turn
    /// proceeds normally.
    #[tokio::test]
    async fn a_retryable_fault_followed_by_success_succeeds() {
        let fixture = play_world(None, "player-retry-success").await;
        let personas = PersonaLane::new(
            ControllerPort::new(fixture.world.clone()),
            ScriptedPort::new(vec![output("proj", vec![text_event("Fine.")])]),
            "gpt-5.6-sol".into(),
            "gpt-5.6-sol".into(),
        )
        .unwrap();
        let directory = tempfile::tempdir().unwrap();
        let end = output("r0", vec![call_event("c0", END_TURN_TOOL, serde_json::json!({}))]);
        let port = DispositionPort::new(vec![Err(InferenceFault::retryable("transient")), Ok(end)]);
        let table = PlayTable::new(
            fixture.world.clone(),
            personas,
            port.clone(),
            "gpt-5.6-terra".into(),
            Arc::new(Semaphore::new(2)),
            directory.path().join("play-turn-v1.cc"),
        )
        .unwrap();

        table.run(&fixture.principal, test_turn_id(682), "Hello?".into()).await.unwrap();

        assert_eq!(port.call_count(), 2, "a Retryable fault must retry once, then succeed");
        let stored = table.store.lock().await;
        let turn = stored.current().unwrap();
        assert_eq!(turn.state, PlayTurnState::Closed);
        assert!(turn.fault.is_none());
    }

    /// PA.f86 (S6): a resumed round's opening conversation item must be the
    /// turn's own recorded `opening_prompt`, never a `table_view` of a
    /// fresh snapshot taken at resume time — the world may have drifted (a
    /// patch committed between rounds) by then. Mutation: rebuilding
    /// `conversation[0]` from `table_view(&fresh_snapshot)` in
    /// `rebuild_conversation`/`infer_round` fails this.
    #[tokio::test]
    async fn a_resumed_rounds_opening_item_is_the_recorded_prompt_not_a_fresh_view() {
        let fixture = play_world(None, "player-s6-fresh-view").await;
        let directory = tempfile::tempdir().unwrap();
        let store_path = directory.path().join("play-turn-v1.cc");
        let ask = output(
            "r0",
            vec![call_event("c0", ASK_PLAYER_TOOL, serde_json::json!({"question": "Which way?"}))],
        );
        let personas = PersonaLane::new(
            ControllerPort::new(fixture.world.clone()),
            ScriptedPort::new(vec![]),
            "gpt-5.6-sol".into(),
            "gpt-5.6-sol".into(),
        )
        .unwrap();
        let table = PlayTable::new(
            fixture.world.clone(),
            personas,
            ScriptedPort::new(vec![ask]),
            "gpt-5.6-terra".into(),
            Arc::new(Semaphore::new(2)),
            &store_path,
        )
        .unwrap();
        table
            .run(&fixture.principal, test_turn_id(510), "I stand at a crossroads.".into())
            .await
            .unwrap();
        let (opening_prompt, question_id) = {
            let stored = table.store.lock().await;
            let turn = stored.current().unwrap();
            (turn.opening_prompt.clone(), turn.open_question_id().unwrap())
        };
        drop(table);

        // World drift between the question and the answer.
        let personas2 = PersonaLane::new(
            ControllerPort::new(fixture.world.clone()),
            ScriptedPort::new(vec![output("proj", vec![text_event("You choose left.")])]),
            "gpt-5.6-sol".into(),
            "gpt-5.6-sol".into(),
        )
        .unwrap();
        let end = output("r1", vec![call_event("c1", END_TURN_TOOL, serde_json::json!({}))]);
        let port = ScriptedPort::new(vec![end]);
        let table = PlayTable::new(
            fixture.world.clone(),
            personas2,
            port.clone(),
            "gpt-5.6-terra".into(),
            Arc::new(Semaphore::new(2)),
            &store_path,
        )
        .unwrap();
        table
            .play
            .submit_patch(
                CommandId::new(),
                decode_authoring_call(
                    "declare_resource",
                    &serde_json::json!({"handle": "silver", "label": "Silver"}).to_string(),
                )
                .unwrap(),
            )
            .await
            .unwrap();

        table
            .run(
                &fixture.principal,
                test_turn_id(511),
                PlayRequest {
                    text: "left".into(),
                    answers: Some(question_id),
                },
            )
            .await
            .unwrap();

        let seen = port.seen_requests();
        let request_text = format!("{:?}", seen.first().unwrap());
        // `request_text` is `Debug`-formatted, so a literal newline inside
        // the recorded `opening_prompt` prints as the two characters `\n`;
        // match that same escaping rather than the raw string.
        let opening_prompt_escaped = opening_prompt.replace('\n', "\\n");
        assert!(
            request_text.contains(&opening_prompt_escaped),
            "the resumed round's opening item must be the recorded opening_prompt: {request_text}"
        );
        assert!(
            !request_text.contains("Silver"),
            "a fresh table_view would leak the mid-turn patch into the opening item: {request_text}"
        );
    }

    /// PA.f88: Dungeon's own `PLAY_TOOLS` is the single policy owner of
    /// what the play agent may call (the library's own copy in
    /// `ghostlight::table` is its test fixture only) — pinned here at its
    /// exact 24-tool contents, `communicate` excluded (PA.f75). Mutation:
    /// re-adding `communicate`.
    #[test]
    fn play_tools_is_pinned_to_its_exact_contents_with_no_communicate() {
        const EXPECTED: &[&str] = &[
            "relocate",
            "transfer",
            "consume",
            "mint",
            "bind",
            "release",
            "witness",
            "acquire_knowledge",
            "forget",
            "create_commitment",
            "discharge_commitment",
            "advance_pressure",
            "reduce_pressure",
            "open_route",
            "close_route",
            "declare_place",
            "declare_route",
            "declare_resource",
            "declare_fact",
            "declare_subject",
            "set_persona_material",
            "retire",
            "grant_affordance",
            "revoke_affordance",
        ];
        assert_eq!(PLAY_TOOLS, EXPECTED);
        assert_eq!(PLAY_TOOLS.len(), 24);
        assert!(!PLAY_TOOLS.contains(&"communicate"));
    }

    /// The `request_id` an `InferenceRequest` `Debug`-prints, extracted so
    /// two requests can be compared on that field alone — it is the one
    /// place `dispatch_command_id`'s own output value actually reaches
    /// something this test can observe from outside `execute_dispatch`.
    fn request_id_of(request: &InferenceRequest) -> String {
        let text = format!("{request:?}");
        let key = "request_id: \"";
        let start = text.find(key).expect("a request carries its own request_id") + key.len();
        let end = start + text[start..].find('"').expect("the request_id value is a quoted string");
        text[start..end].to_owned()
    }

    /// PA.f96: replaces the old test that called `dispatch_command_id_for_handle`
    /// directly by observing the *actual* production path instead. Two
    /// Persona subjects are dispatched together under one shared `turn_id`,
    /// once as `[Mara, Borin]` and once as `[Borin, Mara]`. Each dispatched
    /// subject's own inference `request_id` carries `dispatch_command_id`'s
    /// own output — the same value `execute_dispatch` itself derives and
    /// hands to `personas.turn` — so comparing Mara's own `request_id`
    /// across the two orders observes whether that id actually depended on
    /// her position in the JSON list, without recomputing the hash
    /// ourselves. Mutation: hashing the subject's list position (`position`)
    /// instead of the subject itself at the `execute_dispatch` call site
    /// changes Mara's own `request_id` between the two runs, failing this.
    #[tokio::test]
    async fn dispatch_order_never_changes_the_request_id_a_subject_gets() {
        let fixture = play_world(Some("Mara"), "player-dispatch-order-ids").await;
        let snapshot = fixture.world.snapshot().await.unwrap();
        let mara = persona_id(&snapshot);
        let affordance_id = bracketed_id_after(&table_view(&snapshot), "speak [");

        // A second Persona subject, declared through the play table's own
        // authoring door — the same mechanism
        // `set_persona_material_on_a_same_run_declared_subject_commits` uses.
        let declare = output(
            "r0",
            vec![
                call_event(
                    "c0",
                    "declare_subject",
                    serde_json::json!({
                        "handle": "borin",
                        "label": "Borin",
                        "kind": "person",
                        "controller": {"type": "narrative_persona"},
                        "affordances": [{"ref": "existing", "value": affordance_id}],
                        "position": null,
                    }),
                ),
                call_event("c1", END_TURN_TOOL, serde_json::json!({})),
            ],
        );
        let directory = tempfile::tempdir().unwrap();
        let store_path = directory.path().join("play-turn-v1.cc");
        {
            let personas = PersonaLane::new(
                ControllerPort::new(fixture.world.clone()),
                ScriptedPort::new(vec![output("proj-declare", vec![text_event("Quiet.")])]),
                "gpt-5.6-sol".into(),
                "gpt-5.6-sol".into(),
            )
            .unwrap();
            let table = PlayTable::new(
                fixture.world.clone(),
                personas,
                ScriptedPort::new(vec![declare]),
                "gpt-5.6-terra".into(),
                Arc::new(Semaphore::new(2)),
                &store_path,
            )
            .unwrap();
            table.run(&fixture.principal, test_turn_id(690), "Introduce Borin.".into()).await.unwrap();
        }
        let snapshot = fixture.world.snapshot().await.unwrap();
        let borin = snapshot.subjects.iter().find(|row| row.label == "Borin").unwrap().id;

        // One turn_id shared by both orders: `dispatch_command_id` derives
        // from `(turn_id, round, slot, subject)`, so holding turn_id, round,
        // and slot fixed isolates subject-vs-position as the only variable.
        let shared_turn_id = test_turn_id(691);

        async fn dispatch_pair(
            world: &WorldMailbox,
            turn_id: &str,
            order: [SubjectId; 2],
        ) -> (PlayTurn, Arc<ScriptedPort>) {
            let port = ScriptedPort::new(vec![
                output("proj-0", vec![text_event("considers.")]),
                output("persona-0", vec![text_event("speaks first.")]),
                output("proj-1", vec![text_event("waits.")]),
                output("persona-1", vec![text_event("speaks second.")]),
            ]);
            let personas = PersonaLane::new(
                ControllerPort::new(world.clone()),
                port.clone(),
                "gpt-5.6-sol".into(),
                "gpt-5.6-sol".into(),
            )
            .unwrap();
            let directory = tempfile::tempdir().unwrap();
            let table = PlayTable::new(
                world.clone(),
                personas,
                ScriptedPort::new(vec![]),
                "gpt-5.6-terra".into(),
                Arc::new(Semaphore::new(2)),
                directory.path().join("play-turn-v1.cc"),
            )
            .unwrap();
            let mut turn = PlayTurn {
                turn_id: turn_id.to_owned(),
                applied_keys: Vec::new(),
                opening_prompt: String::new(),
                player_prose: Vec::new(),
                rounds: Vec::new(),
                calls: Vec::new(),
                persona_turns: Vec::new(),
                question: None,
                refusal: None,
                narration: None,
                fault: None,
                state: PlayTurnState::Running,
            };
            let (outcome, summary) = table.execute_dispatch(&mut turn, 0, 0, &order).await.unwrap();
            assert!(matches!(outcome, RoundOutcome::Continue), "dispatch must not fault: {summary}");
            assert!(!summary.contains("could not act"), "{summary}");
            (turn, port)
        }

        let (forward, forward_port) = dispatch_pair(&fixture.world, &shared_turn_id, [mara, borin]).await;
        let (reverse, reverse_port) = dispatch_pair(&fixture.world, &shared_turn_id, [borin, mara]).await;

        for (label, turn, subject) in [
            ("forward", &forward, mara),
            ("forward", &forward, borin),
            ("reverse", &reverse, mara),
            ("reverse", &reverse, borin),
        ] {
            assert!(
                turn.persona_turns.iter().any(|(id, _)| *id == subject),
                "{label} dispatch must have recorded a Persona turn for the subject"
            );
        }

        // Forward dispatched [Mara, Borin]: Mara's own Projector request is
        // seen first (index 0); reverse dispatched [Borin, Mara]: Mara's own
        // is seen third (index 2), after Borin's own pair.
        let forward_seen = forward_port.seen_requests();
        let reverse_seen = reverse_port.seen_requests();
        let mara_request_id_at_position_0 = request_id_of(&forward_seen[0]);
        let mara_request_id_at_position_1 = request_id_of(&reverse_seen[2]);
        assert_eq!(
            mara_request_id_at_position_0, mara_request_id_at_position_1,
            "Mara's own request id must depend only on (turn_id, round, slot, subject), never her position in the dispatch list"
        );
        let borin_request_id_at_position_1 = request_id_of(&forward_seen[2]);
        let borin_request_id_at_position_0 = request_id_of(&reverse_seen[0]);
        assert_eq!(
            borin_request_id_at_position_1, borin_request_id_at_position_0,
            "Borin's own request id must depend only on (turn_id, round, slot, subject), never his position in the dispatch list"
        );
        assert_ne!(
            mara_request_id_at_position_0, borin_request_id_at_position_0,
            "two distinct subjects dispatched together must not share a request id"
        );
    }

    // --- PA.f97: short handles and collision detection --------------------

    /// The longest possible actor tool name (`<8-hex-handle>__<48-char kind>`)
    /// stays well under a common 64-char function-name limit.
    #[tokio::test]
    async fn the_longest_possible_actor_tool_name_is_64_chars_or_fewer() {
        let fixture = play_world(Some("Mara"), "player-handle-length").await;
        let snapshot = fixture.world.snapshot().await.unwrap();
        let prefix = actor_prefix(player_id(&snapshot));
        assert!(prefix.len() <= 10, "the handle prefix must be short: {prefix}");
        let longest_kind = "x".repeat(48);
        let name = format!("{prefix}{longest_kind}");
        assert!(name.len() <= 64, "an actor tool name of {} chars exceeds a common 64-char limit: {name}", name.len());
    }

    /// The player's and a Persona's own short handles round-trip through
    /// `resolve_handle` unchanged.
    #[tokio::test]
    async fn player_and_persona_handles_round_trip() {
        let fixture = play_world(Some("Mara"), "player-handle-roundtrip").await;
        let snapshot = fixture.world.snapshot().await.unwrap();
        let player = player_id(&snapshot);
        let persona = persona_id(&snapshot);
        assert_eq!(handle_for(player).len(), 8);
        assert_eq!(handle_for(persona).len(), 8);
        assert_ne!(handle_for(player), handle_for(persona));

        let player_row = SubjectRow { id: player };
        let resolved_player = resolve_handle(&handle_for(player), Some(&player_row), &[persona]);
        let resolved = resolved_player.expect("the player's own handle resolves");
        assert_eq!(resolved.subject, player);
        assert!(resolved.is_player);

        let resolved_persona = resolve_handle(&handle_for(persona), Some(&player_row), &[persona]);
        let resolved = resolved_persona.expect("the dispatched Persona's own handle resolves");
        assert_eq!(resolved.subject, persona);
        assert!(!resolved.is_player);
    }

    /// PA.f97: two distinct subjects sharing one handle is a collision;
    /// `find_handle_collision` is the pure function `round_tools` calls to
    /// refuse the whole round as a turn fault. `SubjectId` mints nothing a
    /// consumer crate can construct by hand and two real, randomly issued
    /// ids cannot feasibly be forced to collide, so this drives the
    /// detection algorithm directly with two real (distinct) subject ids
    /// under a shared, hand-assigned handle string, rather than attempting
    /// to force a genuine 8-hex-digit collision through a fixture — which
    /// is the infeasible half of this rule's own test list; reported here
    /// rather than faked.
    #[tokio::test]
    async fn a_handle_collision_between_distinct_subjects_is_detected() {
        let fixture = play_world(Some("Mara"), "player-handle-collision").await;
        let snapshot = fixture.world.snapshot().await.unwrap();
        let a = player_id(&snapshot);
        let b = persona_id(&snapshot);
        let collision =
            find_handle_collision(vec![("SAME1234".to_owned(), a), ("SAME1234".to_owned(), b)].into_iter());
        assert_eq!(collision.as_deref(), Some("SAME1234"));
    }

    /// The same subject appearing twice under its own handle (a subject
    /// that could in principle be named more than once) is not a collision.
    #[tokio::test]
    async fn the_same_subject_listed_twice_under_one_handle_is_not_a_collision() {
        let fixture = play_world(None, "player-handle-no-collision").await;
        let snapshot = fixture.world.snapshot().await.unwrap();
        let a = player_id(&snapshot);
        let collision = find_handle_collision(vec![(handle_for(a), a), (handle_for(a), a)].into_iter());
        assert_eq!(collision, None);
    }

    /// PA.f105: a genuine collision between two *real* `SubjectId`s, not a
    /// hand-assigned string pair — `SubjectId` is `#[serde(transparent)]`
    /// over a `Uuid` and `WorldSnapshot::subjects[i].id` is `pub`, so a real
    /// id can be forged by re-deriving its text with the player's own first
    /// hex group spliced in, then deserializing that text back into a real
    /// `SubjectId` through the one door this crate has for constructing one
    /// at all: `serde_json`. No kernel-level backdoor is needed; this was
    /// wrongly reported infeasible in the prior pass. `round_tools` is
    /// called directly with the mutated snapshot, exactly as `infer_round`
    /// calls it before every round's own inference.
    #[tokio::test]
    async fn round_tools_refuses_a_genuine_handle_collision_between_two_real_subjects() {
        let fixture = play_world(Some("Mara"), "player-real-handle-collision").await;
        let mut snapshot = fixture.world.snapshot().await.unwrap();
        let player = player_id(&snapshot);
        let persona_index = snapshot
            .subjects
            .iter()
            .position(|row| row.controller_mode == Some(ControllerMode::NarrativePersona))
            .unwrap();

        let player_text = subject_id_text(&snapshot, player);
        let persona_text = subject_id_text(&snapshot, snapshot.subjects[persona_index].id);
        // The player's own first 8-hex-digit group, spliced onto the
        // persona's own remaining groups: a distinct, real, well-formed
        // uuid string that still shares `handle_for`'s whole extent with
        // the player.
        let colliding_text = format!("{}{}", &player_text[..8], &persona_text[8..]);
        let colliding_id: SubjectId =
            serde_json::from_value(serde_json::Value::String(colliding_text)).expect("a valid uuid deserializes");
        assert_ne!(colliding_id, snapshot.subjects[persona_index].id, "the forged id must be a distinct subject");
        snapshot.subjects[persona_index].id = colliding_id;

        let directory = tempfile::tempdir().unwrap();
        let personas = PersonaLane::new(
            ControllerPort::new(fixture.world.clone()),
            ScriptedPort::new(vec![]),
            "gpt-5.6-sol".into(),
            "gpt-5.6-sol".into(),
        )
        .unwrap();
        let table = PlayTable::new(
            fixture.world.clone(),
            personas,
            ScriptedPort::new(vec![]),
            "gpt-5.6-terra".into(),
            Arc::new(Semaphore::new(2)),
            directory.path().join("play-turn-v1.cc"),
        )
        .unwrap();

        let error = table.round_tools(&snapshot, &[]).unwrap_err();
        assert!(error.contains("share the handle"), "{error}");
    }

    // PA.f105: the resume path through `execute_round` runs the identical
    // `find_handle_collision` check over the identical
    // `snapshot.subjects.iter().map(|row| (handle_for(row.id), row.id))`
    // source `round_tools` uses (see the doc comment on that call site).
    // Forcing a live collision through `execute_round` itself would need
    // the *kernel's* own snapshot to already carry one, and the kernel
    // mints every `SubjectId` internally — no fixture here has a way to
    // hand it a forged one the way `round_tools` above can be called
    // directly with a mutated snapshot value. This is reported, not
    // covered by its own live-collision test, on the same precedent
    // `a_refused_player_act_sets_the_refusal_line`'s own doc comment
    // already uses for a structurally identical gap: the wiring is a few
    // lines, directly inspectable, and shares its exact code path —
    // `find_handle_collision`, `handle_for`, the same error text — with the
    // `round_tools` call site the test above does cover for real.
}
