# Ghostlight Current System Map

GhostlightDungeon is the active hosted runtime. Its authority map is
`docs/architecture/ghostlight-dungeon-mvp.md`. The implemented Rust daemon,
CultCache campaign stores, CultMesh/Eve surfaces, browser lowerer, native
Yggdrasil systemd body, Idunn continuity target, and Odin discovery crossing
are the live machine; the older validated artifact seams remain regression
evidence only.

## GhostlightDungeon target flow

Campaign creation now has its own pre-world authority:

```text
Heimdall member messages
  -> SessionZeroKernel durable shared/private conversation
  -> Projector -> persistent DM Persona owns exact speech
  -> Interpreter proposes typed draft changes, never rewritten speech
  -> runtime binds both into one revision-bound DM turn proposal
  -> typed contract, private character drafts, boundaries, decisions
  -> roster lock -> approved_campaign_brief.v1 -> Vault compiler
  -> evidence-gap conversation or digest-bound final review
  -> every player approves shared + own private digest
  -> one staging CultCache batch + atomic directory exposure
  -> campaign_membership.v1 binds each member to one exact actor
```

`WorldKernel` receives no draft state. The transcript is never replayed into the
compiler. Account and invitation hashes remain persistence-only; shared model
turns, browser surfaces, schemas, and CultMesh records cannot see them.

Session Zero extraction is one atomic membrane with separated authorship. For
an ordinary turn, Ghostlight binds one exact current player message separately
from six prior same-channel turns and four shared continuity turns. The
Projector receives that bounded channel-permitted state; the Persona receives
the current turn verbatim plus its lived narrative and owns the complete
natural DM utterance. The Interpreter receives the same current turn, only the
typed state for its channel's authority lane, and no speech field. A private
Interpreter does not receive the shared contract merely to be forbidden from
editing it. Ghostlight binds the exact Persona output to the Interpreter's
typed proposals only after every stage validates against the same component
epoch. Interpreter failure therefore commits neither prose nor draft state.

Material negotiation has one typed path. Accept applies the exact proposal
stored in the decision. Counter preserves that proposal as an inert, visible
audit and replacement basis, records the player's counter in its shared or
private durable channel, removes the Accept control, and leaves compilation
blocked. The DM may replace it only
through a same-epoch `ApplyDmTurn` containing a fresh material decision; the
replacement and retirement of the pending counter commit atomically. Stale,
empty, malformed, or failed counter responses leave the counter pending and the
retired payload uncommittable.
Retry is an inference launch against that persisted counter and exact unchanged
snapshot. It owns no state transition; a replacement still commits only through
`ApplyDmTurn` at the original component and channel epochs.
For a counter replacement, the target decision ID enters the stage binding. A
deterministic Ghostlight Projector turns only the target, retired typed payload,
exact counter, and aggregate safety policy into the Persona's exact lived
stream; it does not spend a model call asking another agent to paraphrase facts
Ghostlight already owns. The Interpreter receives the same bounded typed basis.
For an exact typed counter lane it emits only the replacement payload. Fresh
decision identity, owner, materiality, evidence, prompt, and counter text are
bound locally; permission identity, actor binding, and evidence cannot be
rewritten by the model. The full decision-union Interpreter remains for
ordinary turns and legacy payloadless migration only.
Conversation history, party, contract, and unrelated private state are omitted.
Legacy counters created by the payload-erasing build receive one bounded
current-state basis. The kernel accepts exactly one same-lane decision with
stable permission identity and no direct patch, so unrelated output cannot
retire the counter.

An acceptance card is also a typed-state claim. New Interpreter decisions must
carry a non-empty contract, character, or extraordinary-permission payload;
payloadless questions remain speech. Legacy empty decisions project no Accept
control and the kernel rejects forged acceptance without changing revision.
This prevents polished DM prose such as “materialize the character” from
advancing a revision while leaving the character ledger blank.

The player-facing Eve surface projects the exact typed payload on every
acceptance card and the complete actor-entitled private character draft. The DM
summary is explanatory, never the accepted state. Other members receive only
their own private ledger and the deliberately public party projection.

```text
Vault evidence receipts + typed campaign snapshot
  -> permissioned projection
  -> Persona receives private lived narrative only
  -> Interpreter emits typed deltas and action proposals
  -> deterministic gates + expected revision
  -> one WorldCommand enters the campaign mailbox
  -> atomic CultCache commit + receipt
  -> parallel affected-participant appraisal
  -> CultMesh/Eve projection
```

Ghostlight owns the generalized projection organ. Epiphany and other consumers
own their canonical Persona state and consequence commits.

## DELVE/HOLD consumer profile

Delvehold is the first proving consumer for Ghostlight's generic API for one
canonical world beyond a consumer-owned boundary. Its target contract supplies an authored
ontology and fixed seed rather than invoking Session Zero world generation.
That bypass ends at generation: publication and every later transition must
still enter `WorldKernel` through typed validation and atomic CultCache commit.

Delvehold owns players, workshops, dungeons, civic state, contracts, and its
quantitative economy. Ghostlight owns external actors, institutions, Gestalts,
relationships, knowledge, pressures, and strategic decisions. Consumer
configurations may bind institution or Gestalt subjects through
`ExternalSubjectAuthority`. Those subjects project committed revisioned snapshots into Ghostlight,
receive no Persona or strategic turn, and remain targets of foreign action.
Such actions persist as attributed proposals in the same strategic-wave commit
and return for consumer admission, not direct mutation. Compiler and consumer
producers share one `WorldSeed` admission transaction.

Clock consequences cannot use an externally controlled subject as their
affected or responsible scope. They may still reach that subject through an
exact place or public channel, but only the consumer may decide what happens
inside its sovereign state. Temporary materialized-member actor projections
are also ineligible clock anchors; consequences bind stable simulated people,
institutions, or populations.

The Delvehold adapter remains Delvehold-owned. It lowers committed Greathold
projections and realized effects into generic Ghostlight operations, then raises
attributed Ghostlight intents, news, projections, and receipts into Delvehold's
domain contract. It owns no truth on either side.

The current `InstitutionState` and strategic resource vocabulary use named
resource handles. They do not model quantities, prices, recipes, facilities,
capacity, inventories, orders, contracts, or conservation. Those remain
consumer-owned unless Ghostlight later admits a deliberate economic component
and mutation algebra. The complete contract is
`docs/architecture/delvehold-forced-ontology-integration.md`.
The active next implementation organ is the Delvehold-owned adapter at this
boundary; newsroom cache probing is not on the critical path.

The current checkout admits one to eight campaign members into one shared
scene. `campaign_membership.v1` maps each authenticated account to one exact
actor; HTTP, Eve, assessment confirmation, and CultMesh publication derive from
that binding. Human actors are excluded from Persona and strategic control.
Sequential public actions are accepted; PvP and split-party movement are
rejected. Time, group travel, and pooled cell-budget changes require unanimous
revision-bound approval. Post-launch Contract Review reuses the Session Zero
kernel against an exact world revision and commits approved amendments in one
campaign batch without rewriting location, topology, knowledge, memory, or
history. The authority map is
`docs/architecture/ghostlight-dungeon-session-zero.md`; later multiplayer work
remains in `docs/architecture/ghostlight-dungeon-multiplayer-intention.md`.

Solo `Wait` has two exact meanings. A wait shorter than the campaign horizon is
a simple fictional-time command. A wait equal to one horizon enters the same
`AdvanceStrategicTick` compiler, Persona-cell, outcome, and atomic commit path
as away-time simulation, with a one-horizon maximum per player command. The
bounded co-op time proposal still advances raw time after unanimous approval;
routing governed co-op horizons through strategic simulation remains an open
implementation gate. Group travel updates controlled actor locations and their
canonical agency-profile locations in the same commit, so reload never becomes
a repair owner for partition inputs.

Build provenance has one owner: `crates/ghostlight-dungeon/build.rs` binds the
exact source commit into the binary. Its inputs are either the release tool's
explicit clean-tree commit or Git HEAD; it watches the actual symbolic ref,
HEAD reflog, and packed refs so an ordinary commit invalidates Cargo's cached
value. Health derives its commit from that embedded value. Release manifests,
launch scripts, and CultCache deployment receipts are verifiers and records,
not alternate writers. Local runs and immutable releases use this same binding,
and the regression test compares it to the checkout's live `git rev-parse
HEAD`. A mismatch blocks activation rather than being repaired after launch.

Away-time agency follows a separate proposal/commit seam:

```text
exact campaign revision
  -> flash-model six-axis resolution demand
  -> connected agency graph + budget/pins/leases/detail debt
  -> cohesive or arena simulation-cell cover
  -> scheduler-owned Nemesis agent binds exact committed event, pressure, or clock anchors to eligible decision owners inside that cover
  -> deterministic agenda admission checks anchor visibility, simulation authority, player exclusion, and per-cell decision quota
  -> one private Projector/Persona/Interpreter membrane per cell
  -> Interpreter agent submits or incrementally edits one private exact-decision draft
  -> deterministic draft compiler checks exact ownership, permissions, topology, and bounds
  -> independent per-action effect verifier checks the typed action against the Persona choice
  -> accepted unchanged actions retain their exact verifier binding across local draft repair
  -> runtime selects compatible attempts and content-addresses each activity
  -> parallel per-activity fast outcome candidates choose bounded typed consequences
  -> one local binder plus selective semantic verifier admit or reject each frozen candidate
  -> at most one balanced reconciliation replaces only a rejected outcome bundle
  -> direct and reconciled candidates publish the same terminal outcome-resolver authority
  -> runtime binds complete cell membership + world/resolution revisions
  -> WorldKernel validates cover, stage/outcome receipts, knowledge, scope, custody, topology, and bounds
  -> AdvanceStrategicTick through the campaign mailbox
  -> one atomic campaign/event/news/cover/appraisal commit
```

### Causal follow-through (Nemesis)

- **Owner:** the strategic scheduler owns which already-committed consequence
  deserves another decision window. The bounded model agent serving that
  decision is Nemesis. It does not own what any subject decides.
- **Inputs:** one frozen campaign revision, its committed public events and
  active pressures and clock progress, the durable Nemesis attention history,
  the deterministic resolution cover, exact perception routes, simulation
  eligibility, Gestalt-member materialization state, player-control boundary,
  and each cell's action quota.
- **Output:** one bounded follow-through agenda binding exact event, pressure,
  or clock anchors to exact eligible decision owners already represented by the
  cover, plus the model receipt rebound to the exact campaign snapshot and
  admitted agenda.
  The agenda is scheduler state checkpointed with that cover, not canonical
  world state. An empty agenda is a valid Nemesis judgment.
- **Derived state:** an assigned owner receives the exact anchor in its
  permitted cell slice, takes precedence over ordinary rotation for that wave,
  and supplies that anchor's exact current committed account to the action's
  private outcome context. The anchor is context, not new mutation authority.
  Responder-cell routing preserves identity ownership: a materialized
  `member:*` subject remains an actor and binds only to its actor cell; a dormant
  member keeps its exact member identity but routes through its aggregate
  Gestalt's cell as a selected member exception. Unassigned recent events remain
  an ambient perception projection.
- **Forbidden writers:** the follow-through agent cannot emit events, mutate
  pressures, choose an action or outcome, move a subject, rewrite a cell
  Persona, change member materialization, collapse a materialized actor into its
  aggregate Gestalt, or touch newspaper copy. The outcome organ remains the
  sole assessor of selected attempts; `WorldKernel` remains the sole world and
  event writer and the sole writer of durable served-window records. Member
  materialization changes only through the existing kernel presence transition;
  bounded legacy identity normalization may canonicalize an already materialized
  actor ID but does not decide presence.
- **Shared paths:** first execution and checkpoint resume consume the identical
  accepted agenda and exact receipt binding. `WorldKernel` independently
  requires that exact admitted receipt before it commits a nonempty causal
  agenda, then records each exact anchor/responder pair atomically with the
  wave. Local cell verification and final wave admission both check that
  required owners were considered without requiring them to act rather than
  choose an attributed inaction. Later discovery excludes an already served
  exact pair; a clock's new progress value creates a new anchor. Responder
  discovery, agenda quota/focus admission, validation, and per-cell assignment
  injection all use the same materialization-aware cell resolver.
- **Cut line:** blind tick rotation no longer decides a slot already occupied
  by an admitted causal assignee, and the global last-twelve-event truncation
  cannot hide that assignee's exact anchor. No prompt downstream may compensate
  for a missing scheduler decision window. `response_role` and a claimed
  accepted-receipt ID are not part of the admission: the scheduler admits only
  the exact anchor/responder assignment, and receipt identity comes from the
  actual model invocation.
- **Verification layer:** focused tests prove paged retrieval beyond the initial
  event viewport, valid empty judgment, exact-pair replay suppression, player
  exclusion, exact owner consideration, materialized-member routing to the actor
  cell versus dormant-member routing to the Gestalt cell, exact anchor citation,
  outcome context containing only the assigned anchor and its committed
  account, and kernel rejection of a forged agenda followed by atomic acceptance
  and served-window recording for the exact receipted agenda. Deterministic
  validators reject unknown anchors, ineligible or duplicate responders,
  over-quota agendas, missing exact receipts, and stale anchor accounts. A
  strategic smoke remains the whole-system verification that named competing
  decisions produce
  committed material outcomes before editorial quality is assessed.

The model owns no tick mutation. A provider failure or invalid proposal leaves
the campaign revision and world time untouched. Background inference checks
live-turn pressure before launch and again before commit; return catch-up uses
the same command path with player-turn priority for fictional commands.

The effect and outcome verdict schemas make semantic coherence structural. Each
candidate action receives its own parallel effect-verifier call. Its schema and
local validator both require one index-zero verdict: `match` with no findings,
or `mismatch` with one through six findings whose mismatch kinds are distinct
and whose concrete repair guidance is nonempty, trimmed, and at most 240
characters. The prompt asks for that complete bounded set and the request's
1,200-token ceiling can carry the full schema maximum. Every returned finding
enters one subject-scoped Interpreter repair. Repair prose is ephemeral
diagnostic input, not world state; no verifier chooses the replacement or gains
commit authority, and private cell choices never reach a player error surface.
A rejected wave returns one spoiler-free message while its exact diagnostic
remains operator-only.

The Interpreter is a bounded model agent over a private workbench. Every
model-agent workbench owns the action schema legal at its current state; the
provider-neutral loop asks the tool for that schema before every semantic step
and places the same schema in both prompt guidance and the structured response
boundary. For an empty Interpreter draft, the only command is one complete
exact `submit`. A rejected draft remains private tool state, and the next schema
contains only `upsert_decision` commands for exact missing or rejected owners.
There is no inspection, removal, wholesale resubmit, or unrelated repair branch
for the model to spend a step on. The action wire remains one strict root object
containing one exact typed command. Local validation returns a typed
`local_validation` finding and the semantic verifier returns exact
subject-scoped mismatch findings. Neither validator chooses the replacement or
gains commit authority. Verifier matches are cached by the exact
snapshot-and-action binding, so repairing one action does not repay inference
for unchanged accepted actions. An `undecided` result returns to the Persona
owner instead of letting the Interpreter invent a choice.

The strategic scheduler owns partial-wave recovery. Its typed checkpoint binds
the exact campaign revision, resolution epoch, deterministic cover and plan,
effective campaign contract and aggregate boundaries, planning receipts,
successful cell terminal bundles, and exact failed-cell set. A retry validates
that complete partition, policy, and causal receipt chain before dispatching
only failed cells. The checkpoint is opaque outside the scheduler. The smoke
runner may persist whole checkpoint generations but cannot merge or edit cell
output; it durably publishes the complete cell checkpoint before wave commit.
Within a cell, the engine projects one exact lived moment. If the
Persona supplies no explicit decision, the engine retries only Persona and
Interpreter against that same projected stream; Projector does not rerun. The
second Persona cites the first Persona and Interpreter attempt as causal
ancestry. Provider transport gives both connection failures and attempt
timeouts one separately bounded retry of the exact same request and snapshot;
a later successful receipt preserves the failed attempt's diagnostic.
Interpreter draft repair remains workbench-local. Actor and cell membranes
share one causal-source derivation primitive for Projector to Persona to
Interpreter, while canonical evidence remains a separate namespace.

World clocks own declared consequence text and exact observable scope. New
seeds must supply that scope and begin below threshold. A persisted legacy
clock without scope enters one balanced-model binding agent over a frozen
campaign revision. The agent proposes the smallest causally sufficient stable,
simulation-eligible scope; its accepted action is rebound to the exact batch
digest. `WorldKernel` alone admits the proposal. The common mutation application
path detects threshold crossing and derives one stable
`clock-consequence:<clock-id>` event, then the shared event publisher derives
the exact rows for every admitted public channel. The event IDs enter mutation
and strategic-tick receipts. Binding scope, exact model receipt ancestry,
emitted event and news IDs, and the next newspaper boundary commit atomically
with the campaign in CultCache. Runner proposal and completion checkpoints are
recoverable projections; neither owns the event nor the newspaper slice.

The effect verifier receives the same exact typed permission slice used by the
Interpreter. That slice is its sole map of canonical locations, reachable actor
destinations, population destinations, and target IDs. A pump, desk, queue, or
other place named only in lived prose remains local texture inside the supplied
canonical activity location; the verifier cannot manufacture topology and then
reject a valid local action for omitting travel through it.

A live request also interrupts an in-flight scheduler wave. Live-turn admission
increments pressure and signals cancellation before waiting for an already
admitted background commit to finish. The admission guard exists before that
wait, so cancellation cannot leak false pressure. Dropping the scheduler wave
aborts its parallel cell tasks before they can launch later Persona stages, and
a shared/exclusive commit gate makes scheduler commit impossible while any live
request is active. Return catch-up is intentionally exempt because it is part of
the live request and must finish before the requested fictional action. Private
assessment and resolution-policy edits neither advance fiction nor invoke
catch-up. Session Zero logs the exact draft revision when a DM response is
queued and when its live guard is admitted, making provider latency distinct
from scheduler contention without exposing channel contents.

Entering or dropping that guard republishes the canonical typed health record
immediately. HTTP health reads the same record, so `live_turn_pressure` is an
actual launch brake during an in-flight player command rather than a delayed
status reconstruction.

Resolution-demand focal IDs are salience hints, not partition commands. They
cannot create mandatory singleton cells or exceed the configured budget. Cell
Projectors receive decision-relevant situation state; cell Interpreters receive
exact permissions and the narrative products. Membership and revision bindings
are derived by the runtime, so a model is never asked to copy an invariant that
the planner already owns. Stable prompt prefixes are deliberately placed before
dynamic state, and provider receipts expose per-attempt token/cache usage plus
bounded local validation failures.

`ModelPort` remains Ghostlight's single inference seam. The Codex-backed
implementation is an ordinary typed consumer of the independent CodexConnector
daemon: it emits an encrypted, expiring MessagePack request over bounded
TCP-framed CultNet and accepts only a correlated ordered text stream plus
terminal receipt. A stable cache key is derived from stage, logical model class,
and output schema rather than mutable campaign content. CodexConnector owns
credential custody, caller admission, bounded provider transport, replay, and
transport receipts. Ghostlight owns provider-request derivation, stage
projection, schemas, retry between passes, semantic validation, model-stage
receipts, and every kernel admission. Epiphany is a separate consumer and owns
neither side. Yggdrasil runs CodexConnector as the independent
`codex-connector.service` release
`f9cfa355051ef91d0e7f095b2df2a69fe79f8a7c`. Ghostlight has only a soft
systemd ordering edge to that daemon and cannot build, configure, restart,
select, or roll it back.

Ghostlight exposes three provider-neutral logical classes. `ghostlight.fast.v1`
owns high-volume cells and retrieval, `ghostlight.balanced.v1` owns bounded
reconciliation and routine elaboration, and `ghostlight.capable.v1` owns
frontier invention and editorial judgment. Only the selected `ModelPort` maps
those classes to physical models or assigns their attempt deadlines; compilers
and harnesses never pass physical model IDs as logical stage authority.

Vault availability is typed at the provider boundary. When the configured
Vault cannot return evidence, SessionZeroKernel returns the exact draft to
`drafting`, records one stable resolved material blocker, and publishes no
preview, branch assumption, world digest, or campaign. Operational causes stay
in the private daemon journal. A later retry uses the unchanged approved brief
through the same compiler path; neither the model nor client can inject a
replacement evidence source.

Canonical actor cell slices include that actor's exact goals, obligations,
relationships, and bounded memories. Named Gestalt-member exceptions receive
the same continuity fields. The Projector narrativizes them for the Persona;
the Interpreter still receives only exact effect permissions. A promise does
not become a hard-coded behavior rule, but it cannot disappear merely because
the actor entered offscreen simulation.

The Interpreter decision schema is derived from every exact decision-owner ID.
Institution, actor, Gestalt, and named-member effect variants cannot cross
subjects: complete submits bind decisions by owner-key, while incremental edits
are rebound and validated under the selected owner by the workbench. Target IDs,
canonical locations, pressure resolutions, movement destinations, state
references, and public channels are enumerated from that subject's permitted
slice. Movement and population-migration variants disappear
when the subject has no exact destination; target-requiring activities cannot
emit an empty target list. Semantic correction therefore judges meaning rather
than repairing type combinations local code already knows are impossible.

Each permitted activity target is one runtime-derived descriptor keyed by its
authoritative ID, with the target's exact name and current canonical locations.
Reachable actor destinations map exact location IDs to names; population
migration destinations carry both population and location identity. The
Projector, Interpreter, and effect verifier can distinguish a canonical target
actually named by the Persona from an unnamed local role, and can prove whether
“go to Reed” means movement at all. Opaque permitted IDs are not enough reason
to substitute a containing population, related institution, or unrelated
destination. Descriptor prose is disambiguation context, not a new subject or
fuzzy-match authority.

Targetless local activity is explicit rather than encoded through a convenient
unrelated subject. Preparation, investigation, communication, and obstruction
may act on an unnamed role, infrastructure, terrain, traffic, or other local
texture only at the exact supplied location. The activity records the source's
attempt, never a listener, discovery, damage, disruption, or response. A
canonical target is required whenever the attempt actually addresses one.

`AgencyProfile.information_channels` owns concrete routes through which a
subject can publish or receive reports. Actor knowledge and Gestalt shared
knowledge remain separate state and may justify an action without manufacturing
a news route. Profile maintenance removes legacy overlap and `unknown`
placeholders; the compiler rejects both categories before campaign approval.

Arena projection preserves spatial partitions as well as identity partitions.
Each remote view carries its actual location; a relation grants potential
reach, not co-presence. Persona turns cannot stage direct sight, speech, or
response across locations unless the lived stream establishes a shared place
or communication channel. Interpreter activity labels then describe the
narrow attempted act, while hoped-for success remains outside the effect.
Ghostlight deterministically prepends the exact name/location partition to the
learned Projector's narrative, so model omission cannot erase those boundaries.
Only supplied cell constituents and selected member exceptions may own a
perspective. Event mentions do not transfer a person into another cell, and an
unnamed or unsupplied entity cannot become an activity target merely because a
Persona found it plausible.

Institution slices carry committed `current_posture` separately from unresolved
pressure. Projectors present the posture as existing state; Interpreters and
validators compare any new commitment directly against it. The old overloaded
`pressures[0]` convention is not an authority path.

Model-facing actions contain one owner, `subject_id`. Nested effect owner IDs
are runtime-derived during binding and exist only in the canonical downstream
proposal. The model cannot disagree with itself about `member:mira-venn` versus
raw `mira-venn`, and the prompt/output contract spends fewer tokens copying
invariants Ghostlight already owns.

Canonical constituent references have one owner:
`resolution::subject_state_references`. Scheduler slices consume that set
directly, including active reachable migration destinations, and wave
validation recomputes the same set. Prompt construction cannot append
references the resolver does not recognize.

Cell perspective attribution is runtime-bound too. The schema admits only
exact constituents and selected member exceptions. Ghostlight rejects
duplicates and requires both the debt-selected focus and every selected named
member exception to receive their own perspective segment; an actionable
individual can no longer disappear inside the aggregate before the Persona
turn. The perspective count may exceed the action limit only by the number of
mandatory owners; action consequences remain separately capped. Segments are
sorted and lowered to natural name/location headings before the Persona sees
prose only. An invented Markdown section cannot turn a mentioned outsider into
a decision-making perspective.

Gestalt background choices use three distinct typed paths. Pressure transitions
change exact unresolved markers. `gestalt_activity` records an attributed
preparation, coordination, investigation, recruitment, obstruction, trade, or
communication attempt against subjects connected by an explicit agency
relation or exact shared location without claiming the outcome. After
selection, the outcome organ launches one logical-fast candidate per selected
activity under the configured provider parallelism. Each candidate receives
only that attempt and its precomputed legal consequence handles. It must return one digest-bound
`strategic_activity_outcome.v1` per activity; missing, duplicate, stale,
player-mutating, or invented effects reject the entire wave. Accepted outcomes
may change exact resources, population pressures, incident agency relations,
named-member deltas, or discoverable canonical knowledge, or explicitly record
that no durable material change occurred. A selected
dormant member can be addressed by their durable ID; this does not union them
into the source population. The kernel derives the event text and exact
participant IDs; the arena and model prose own neither.

The outcome organ does not repair or reinterpret Persona intent. The
Interpreter owns the attempt; the terminal outcome resolver owns opposition and
result; WorldKernel alone owns mutation. Local binding failures and independent
semantic-verifier mismatches are findings from the same admission boundary.
Either finding may invoke one logical-balanced reconciliation that can replace
only the frozen outcome bundle. Cell appraisals, selected attempts, action
digests, campaign snapshot, and admissible handles are immutable inputs. A
failed reconciliation closes the wave; it cannot call the logical-fast
candidate again or return to Persona appraisal.

Direct and reconciled outcomes publish the same terminal
`strategic_outcome_resolver` stage authority at the exact activity binding. A
reconciled receipt remains distinguishable by its logical-balanced model and
causal source chain; the rejected logical-fast receipt remains
`semantic_invalid` evidence and cannot satisfy kernel admission. The terminal
verifier cites the admitted candidate receipt, and every material outcome must
carry that exact outcome-bound valid verifier receipt through WorldKernel
admission. Scheduler sequences the boundary, checks cancellation, and aggregates
typed terminal failures and completed receipts for persistence; it owns no
repair prompt or replacement decision.
Every outcome is stored independently in CultCache and projected on the
operator Eve surface. All effects apply to the same private campaign copy as
the rest of the strategic wave, so a late invalid outcome cannot leave an
earlier action, clock, detail debt, or event committed.

Deterministic admissible-effect handles constrain every resolver decision.
Actor possessions, member equipment, institution resources, and Gestalt
resources use one `resource:*` reference ontology. When a model cites another
action's handle or invents a reference, the local owner returns the exact
action digest, offending values, and admissible set to the private same-snapshot
correction. The player receives only a spoiler-free refusal if correction also
fails.

The resolver's JSON Schema is derived from exact action authority. Each action
digest admits only its available effect kinds; unavailable lanes are absent,
not represented by empty placeholder choices. The chosen effect kind makes its
own owner, recipient, resource, relation, member, fact, pressure, and evidence
fields structurally required while forbidding fields owned by another kind.
Exact IDs and existing values come from that action's bounded slice. New
resource or pressure text remains model-proposed but locally bounded and still
requires kernel validation. This keeps deterministic impossibility out of the
semantic retry loop without letting the schema decide narrative causality.

Pressure authority is state-relative. The outcome stage receives each exact
Gestalt owner together with its current unresolved pressure strings. A
resolution must copy one of those strings; an addition must be new and cannot
overlap a resolution or repeat current state. The runtime rejects the whole
wave on a no-op, duplicate, or invented resolution before commit.
These state-transition checks belong to the outcome semantic validator, not the
generic JSON-shape retry. A well-shaped repeated pressure reaches the shared
balanced reconciliation boundary with the exact rejected bundle; the
replacement may propose a genuinely new pressure or `no_material_change`, while
the kernel guard still makes the repeated transition uncommittable.

Named-member private effects are action-relative too. An obligation is
available only for social activity—communication, coordination, recruitment,
or trade—where the attempt can actually create a promise or debt. Physical
preparation, investigation, or obstruction cannot mint an unrelated social
obligation merely because the member is present in the outcome slice.

Structured-stage failures name the exact stage, instance JSON pointer, and
schema pointer. Receipts retain the bounded private diagnostic; player-facing
strategic refusal remains spoiler-free. A model failure can therefore be
located without logging chain-of-thought or dumping the private prompt.
For the one same-snapshot correction, the validator also projects only the
nearest rejected containing object rather than replaying the whole output. This
gives the model the exact failed activity or effect beside the schema pointer,
preserves the stable cacheable prompt prefix, and avoids paying again for
unrelated valid output. Parallel cell failures retain the exact cell ID,
constituent IDs, and bounded anyhow cause chain in operator-only logs.

Ordinary bounded preparation, pressure, and knowledge effects stop at local
validation. Resource consumption/transfers, agency-relation shifts, and named
member private deltas additionally receive an independent same-snapshot
semantic verdict because those effects can be structurally valid while charging
the wrong inventory or person. The verifier proposes no state and cannot
commit. The latest provider-backed four-wave golden produced seven typed
material activity outcomes, ten durable consequences, and zero rejected pulses
while leaving the player unchanged and returning the same Mira.

The service-owned CultMesh store remains the canonical projection body when
Odin rendezvous is temporarily unavailable. RUDP publication is outbound
replication, not daemon-liveness authority: a failed publish is logged with its
key and target while the complete local snapshot persists and HTTP serves that
same typed health and Eve state.

`advance_one_strategic_tick` owns post-commit mesh publication for scheduler,
return catch-up, and explicit player-horizon waits. Callers no longer decide
whether a strategic commit deserves a projection. A publication error is an
operator-visible derived-state failure; it cannot roll back, rewrite, or report
the already committed world transition as failed. This closes the former path
where return catch-up advanced canonical state, the subsequent player command
correctly failed stale, and an early return left the operator surface on the
previous revision.

The RUDP session owns reliable-send pressure. It admits at most 32 packets—the
cumulative-ACK horizon—queues the rest, promotes queued packets only as ACKs
retire admitted packets, and emits an exact ACK when an older retransmit has
fallen behind that horizon. Its explicit flush succeeds only after every
reliable packet is acknowledged. Large CultMesh documents no longer
depend on blasting every fragment and sleeping for a fixed interval. The wire
contract is unchanged, and model, campaign, and application authorities remain
outside the transport.

Ghostlight stages discovery-critical health and provider advertisement before
bulk projections. Its Odin schema catalog contains only the Ghostlight-owned
boundary state types advertised to consumers: `ghostlight.campaign.v1` and
`ghostlight.session_zero.v1`. Internal compiler, Persona, transition, Gestalt,
receipt, and operator documents remain available through their owning private
projection; they are not dumped into rendezvous merely because Ghostlight has
Rust types for them.

Odin registers only the `ghostlight.schema_catalog.v1` public envelope, not
those private contracts. Its provider-document adapter returns and awaits the
single durable `node.put` promise, preserving per-peer write order instead of
launching concurrent CultCache mutations behind transport acknowledgement.
The live typed snapshot returns Ghostlight source
`7522ea8405212b344441f83f502993f525276521` and exactly the two boundary schema
keys above.

Opening retrieval deliberately covers early, transitional, and late historical
frames. These pure retrieval/compiler functions now serve the Session Zero DM;
they are not browser-owned creation routes or transient preview maps. Generated
suggestions enter the typed draft for discussion and use the same final brief as
custom premises. Both rejected and accepted receipts remain private. Invite rotation is owned by
the deployment boundary: it replaces the protected token set and clears the
persisted auth authority together while the daemon is stopped. Campaign stores
survive; old sessions deliberately do not.

Away time does not require invented agency. When a campaign has no active,
simulation-eligible agency profiles, the scheduler enters the same
`AdvanceStrategicTick` kernel command with no model plan or resolution wave.
World time, clocks, pending-tick accounting, and deterministic obligations
advance atomically while the player remains untouched. The scheduler does not
call the partitioner merely to discover an empty graph, and it retains the same
live-turn commit exclusion as model-backed waves.

Compiler approval is a player decision surface, not an operator spoiler
surface. It exposes topology, public cast, institutions, populations, clocks,
player-role capabilities, evidence coverage, gaps, and branch assumptions. It
does not project branch-local or provisional fact statements—even when they are
discoverable later. Those facts remain canonical inputs to knowledge-gated play
and operator inspection; the browser cannot reveal them before discovery.

## Current acceptance body

Yggdrasil serves one native immutable Ghostlight release under
`ghostlight-dungeon.service`. The fresh-workspace handoff and `state/map.yaml`
carry the exact current source, Eve source, executable hash, provider, store
counts, and replay witness. This map describes the acceptance body without
duplicating a volatile release identity in several sections.

The hosted adversarial campaign now pressures the complete eight-cell strategic
path against real provider output. Every failed Projector, Persona,
Interpreter, effect-verifier, outcome, or kernel boundary has left campaign
revision and fictional time unchanged. Human separate-account co-op privacy and
unanimity remain unproven; the expected D&D cohort dispersed before this pass,
so no fixture or solo browser run may be presented as that acceptance.

Campaign `e99e8794-281f-4a82-8b2c-5e6954bd6b16` is the current live Kalsa
witness. At revision 21 and resolution epoch 3 it runs with a configured
one-cell budget. Startup migration repaired Cal Rusk's malformed doubled legacy
identity to exact canonical `member:cal_rusk` without advancing fictional time
or revision. Cal retained his learned bypass and warning-mark knowledge across
materialisation, folding during travel to Veyr Run, and rematerialisation when
Asha returned to Raincross gate. A budget-1 strategic wait advanced Ilya's
investigation and Oren's movement without puppeting Asha.

The earlier campaign `34929b8d-7b04-49af-9936-1c798fd79760` remains the
eight-cell strategic receipt witness. Its revision-12-to-13 catch-up preserved
five exact actor/institution cells, two Gestalt cells, and one arena containing
three distinct remote institutions. The arena acquired no actor ID. Zhestokost's
repeated posture became attributed inaction; two named members made their own
decisions; Reed kept the twelve-patient commitment; and the player plus Reed
remained byte-identical. Five selected actions produced five bounded outcomes.
Fourteen channel-aware reports remained absent because the player had no route
to them.

That tick used 34 stage receipts and 35 provider attempts: 83,838 prompt
tokens, 64,000 cache-hit tokens, 6,192 completion tokens, and 90,030 total
tokens. The extra Interpreter receipt records the rejected repeated institution
posture. The outcome path records an undersized bundle retry and a rejected
repeated pressure before the corrected complete bundle. All diagnostics
remained private; the campaign committed once.

The public `/ghostlight/` path terminates on Yggdrasil nginx and proxies the
native loopback listener at `127.0.0.1:8831`. Anonymous access returns the
actor-free `ghostlight.play` Eve gate with `200`; actor and campaign state stay
unavailable without an app session and canonical membership. The previous
Starfire writer is stopped; its process and tunnel are no longer live
authority. The migration copied campaign and Session Zero stores while
preserving Yggdrasil's native mesh and provider-health identities.

Local CultMesh publication and HTTP readiness do not wait for Odin. Remote
replication is coalesced into one asynchronous RUDP batch. Odin's coordinator
is healthy on Yggdrasil and owns RUDP discovery at `10.77.0.1:17871`; Idunn
admits signed Ghostlight health and owns same-release continuity independently
of its deployment brake. VoidBot's canonical MCP service is local to the host
at `127.0.0.1:17875/mcp`, so live retrieval no longer depends on a Starfire
reverse tunnel.

### Deployment, discovery, and adjacent capacity

Idunn is the only deployment and daemon-survival authority; Odin is the
discovery/rendezvous authority. Their health or availability never grants
either organ a campaign write path. Current release identity for Idunn, Odin,
Heimdall, Bifrost, and Epiphany is operational truth in `gamecult-ops`; do not
copy it here and let it fossilize again.

Release admission is based on the newest executable- or build-affecting commit
reachable from the admitted ref. Documentation, notes, state receipts, and
root Markdown are not executable release selectors. The root actuator proves
that the selected commit is an ancestor of the admitted ref before activation,
then verifies the exact installed witness. Documentation and state-only commits
therefore cannot displace the live executable.

Heimdall publishes four redacted typed discovery records to Odin under globally
unique catalog keys:
the provider, private command boundary, Eve access plugin, and transport
profile. Ghostlight resolves `heimdall:command-boundary` from Odin only for
begin, complete, refresh, and logout; valid local app sessions do not depend on
an Odin round trip. No direct Heimdall endpoint remains in the Ghostlight unit.

Epiphany is adjacent capacity, not part of Ghostlight's campaign authority and
not required by the current provider-backed runtime. Rehydrate her live state
from her own workspace and `gamecult-ops`. Her deployment remains one
Idunn-owned transaction; passive watcher tasks and cross-task exclusivity
messages are not an operational control plane.

The deployed Session Zero canary survived two exact-build restarts with its
private store intact. Heimdall completed a real KLTST Discord round trip and
adopted one HttpOnly Ghostlight session. Aetheria retrieval produced three
grounded opening decisions. Live DM inference initially exposed that JSON mode
had not been given the Interpreter's application schema; the repaired release
places exact stable schemas before dynamic context, emits only new deltas, and
produced a coherent Mars/Zhestokost follow-up without borrowing First Exodus
state. A forced malformed-output path displayed a local retry notice while the
typed contract remained unchanged.

Authenticated acceptance against the exact release has proved generated
opening/role/approval compilation, a nonliteral canonical player actor ID,
private assessment, confirmed server roll, wait, daemon restart continuity,
fork isolation, original re-selection, reset, and a MessagePack-backed `.cc`
export. The multiresolution matrix covered 24 factions exactly once at budgets
1, 4, 8, and 32. The strict nested refugee golden placed all 24 subjects in one
rival arena, preserved Mira Venn as a named decision owner, migrated her into a
different depth-two population lineage, produced material activity across six
other background subjects, preserved her delta and the player, and later
promoted the same Mira for a return encounter. Witnesses live under
`F:\GameCult\GhostlightDungeon\acceptance\player-journey-a92fb7a-20260818`,
`gestalt-scale-a92fb7a-b{1,4,8,32}`, and
`gestalt-dynamics-7916ab0-strict`.

Cell budget and physical provider parallelism are independent controls. The
scale executable accepts `GHOSTLIGHT_SCALE_PROVIDER_PARALLELISM`, defaults it
to the runtime default of eight, and records it in every witness. The corrected
24-subject budget-8 / parallelism-8 wave completed in 18.35 seconds, covered all
subjects exactly once, committed ten material actions plus one attributed
inaction, did not move the player, and required no model retry. Its witness is
`F:\GameCult\GhostlightDungeon\acceptance\gestalt-scale-shakedown-b8-p8`.
The earlier 25.64-second budget-8 witness used four provider slots and must not
be cited as an eight-concurrency performance result.

`gestalt_migration` records a collective decision by one exact active leaf to
move along an explicit migration relation and reachable route. WorldKernel
changes the leaf's home location and agency-profile location together. The
destination population remains a separate canonical subject, and named member
deltas are not carried by implication. Approval-gated fission creates
destination/stayer cohorts; the same migration primitive moves any resulting
leaf regardless of lineage depth. A local `investigate` activity may omit a
target when it examines the exact current environment. A local `communicate`
may likewise omit a target when its exact source speaks, offers, requests
permission, or notifies an unnamed ordinary role. Both admit ordinary texture
without inventing a listener, response, acceptance, discovered fact, or target
agency.

Activity effects operate at canonical-location granularity. Incidental motion
around unnamed local features remains part of the source-attributed attempt
and does not create topology or assert arrival. A different supplied canonical
location or population destination still requires the exact movement path.

Canonical non-player actors have an `actor_activity` path for ordinary
preparation, coordination, investigation, recruitment, obstruction, trade,
and communication. The Interpreter binds the actor's exact ID locally;
WorldKernel validates current location, graph targets, state references,
channels, and human-control exclusion. Actor movement and actor activity share
one action key, while successful consequences remain a separately resolved,
digest-bound outcome.

Salient dormant members have their own `member_activity` path for ordinary
local attempts. It uses the same bounded verbs, the member's exact location,
their current leaf plus explicitly related or exactly co-located targets, and
their own state/channel permissions. Only the capped salient member exceptions
selected for the cell enter any prompt or can be named there. Migration and
activity conflict on one member key. A destination population cannot inherit
the person's offer, speech, or decision merely because the cover placed them
in the same arena.

A named member's explicit commitment to board, depart, travel, or join a
supplied destination maps to `member_migration`, even when the first narrated
motion is only entering a queue. `member_activity: prepare` is valid only while
departure remains unchosen. The Interpreter and semantic verifier share this
distinction.

The semantic verifier emits one ordered verdict per candidate action.
Ghostlight owns exact verdict coverage and derives the aggregate rejected set;
the model cannot reject one arena constituent merely because another
constituent's mapping failed, omit a candidate, duplicate an index, or publish
a global decision that contradicts its per-action rationale. Rejected actions
return to the same private Interpreter draft as exact subject-scoped findings;
unchanged accepted actions retain their verifier binding.

Population scale uses reversible individuation:

```text
gestalt baseline + existing member delta, or an admitted foreground or system-owned identity proposal
  -> WorldKernel validates gestalt/member/revision/location
  -> atomic durable member delta + temporary ActorState
  -> ordinary Persona appraisal and world commands
  -> expired relevance lease outside player perception
  -> atomic fold into the member delta; active slot removed
  -> gestalt receives strategic ticks without erasing the individual
```

The temporary actor slot is derived. It never owns identity, relationship,
memory, equipment, injury, or obligation state.

Automatic presence planning projects only active leaves at the player's
location, dormant members whose exact location matches, and materialized
members eligible for folding. The kernel rejects inactive hierarchy nodes and
location mismatches. Nested fission and migration therefore change which
baseline a durable delta composes with, never who the person is.
The projection also names the player actor's reciprocal relationship to each
nearby dormant member. The planner owns reversible scene casting and may surface
an earned callback without a player prompt; it cannot move a remote person or
commit the cast. Failed live casts retain a private presence-preflight artifact.

Automatic presence has two different authorities. Recasting an existing exact
member is reversible and may follow any relevant foreground event. Admitting a
first-relevance member is canonical entity creation and is available only when
the planner reason exactly matches the immediately committed player-speech
turn. The generated schema closes the individuation lane for resolved attempts,
and `WorldKernel` independently rejects any such plan. Outcome prose and
narration are not entity-admission inputs.

A separate strategic selector may propose one first-relevance identity only for
an exact selected Gestalt-owned action in the current resolution wave. Its
receipt binds the eligible Gestalt action digest set, exact locally admitted
proposal digest, and world revision.
`resolution::validate_strategic_individuation_proposals` is the single owner of
exact strategic-individuation semantics: one-person budget, selected-action
binding, Gestalt action authority, location, version, lineage, bounded identity
payload, relationships, and uniqueness. The scheduler calls that shared owner
immediately after proposal production. An invalid optional person marks the
selector receipt `semantic_invalid`, records the exact local finding, and is
dropped before wave assembly; the otherwise valid strategic wave may continue.
WorldKernel calls the same validator again against the admitted wave snapshot,
then invokes the same `apply_individuation` primitive used by the system-only
direct command. Early validation is proposal hygiene, not mutation authority or
a substitute for kernel revalidation. The proposal and strategic plan commit
atomically; the selector cannot own entity creation, location, revision,
materialization, or commit.

Institution `already_committed_posture` is projected as durable state, distinct
from unresolved pressure and fresh choice. Maintaining it is inaction. A model
may propose only a materially different posture; repeated posture remains a
locally rejected no-op and cannot enter an atomic wave.

Cell appraisals carry exact `actions[]` and exact `inactions[]`. Inactions name
the constituent or selected dormant member who holds and a bounded reason. They
cannot duplicate, cross cell authority, or coexist with an action from the same
subject. They are non-mutating audit/fairness evidence. The former global
inaction string is no longer an owner or runtime path.

Strategic plan application reads authority and permissions from one immutable
command snapshot. All writes land in a private working campaign which replaces
the command-local campaign only after every action validates. Sequential loop
order is no longer a fictional owner: a migration earlier in the apply loop
cannot invalidate another subject's same-snapshot communication, and a late
invalid action cannot leave earlier in-memory mutations behind.

The active cell budget, operator-owned pins, and provider concurrency limit are
separate controls. Budget and pins increment `resolution_epoch` without
advancing world revision or fictional time. Provider concurrency increments only
`provider_configuration_epoch`; it batches the same cover and cannot repartition
the world. See `docs/architecture/ghostlight-multiresolution-agency.md`.

The public application boundary is one stable Eve provider surface:
`GET /api/eve/provider`, `GET /api/eve/surfaces/ghostlight.play`,
`POST /api/eve/commands`, and revision-only `GET /api/eve/events`. Anonymous
surface requests return an Eve Heimdall access gate without campaign,
membership, actor, account, boundary, or Session Zero state. Authenticated
surface requests derive account, selected campaign, membership, and exact actor
from Ghostlight-owned app-session and campaign records before projection.

Native and browser clients share that logical surface and command authority.
The native path is `ghostlight.native.player` over loopback RUDP
`127.0.0.1:4102`; it exposes only Heimdall begin/completion, actor-filtered
surface retrieval, and canonical Eve invocation. Heimdall persists the OAuth
attempt and single-use completion. Ghostlight redeems the completion into the
same app-session record used by HTTP, then derives every actor and campaign
binding server-side. The installed native client stores only the opaque
Ghostlight bearer in a mode-0600 CultCache file. It never receives a Heimdall
access or refresh claim and cannot submit actor, member, account, or authority
identifiers. While a model-backed Eve invocation is pending, the client sends a
bounded transport keepalive every five seconds; the operation and its result
remain owned by the server-side idempotency reservation and kernel path.

Odin advertises the redacted native route and operation names in the provider
document. It does not proxy native commands or persist session tokens, attempt
handles, claims, account state, membership, or player surfaces. The boundary is
loopback-only; remote native use requires a trusted host crossing and still
passes Heimdall admission. A malformed or forged native request returns a typed
denial before product projection and cannot enter either kernel mailbox.

Command ingress accepts only `gamecult.eve.command_invocation.v1`. It validates
provider, logical surface, command boundary, operation schema, source version,
and idempotency before resolving the caller's authority server-side. Browser
payloads containing account, member, or actor authority fields are rejected.
Every admitted operation then uses the existing `SessionZeroKernel` or
`WorldKernel` mailbox and returns a persisted Eve command receipt. Strategic
ticks, region commits, NPC initiative, and raw kernel commands remain outside
the browser capability surface.

The browser is `EveBrowserProviderHost`, Ghostlight transport, Heimdall access
adapter, status mount, and SSE invalidation. Eve owns editable bindings and
operation capture: renderer-local drafts survive authoritative refreshes and
stale refusals, and only an accepted result may request named draft clearance.
HTML forms may appear as an accessibility lowering detail but are absent from
Eve documents and command payload semantics. Session Zero, campaign,
governance, receipt, and login DOM renderers are not product authorities.

Heimdall owns OAuth attempts, Discord callbacks, entitlement evaluation, claim
issuance, and single-use completion. Anonymous Eve operations cross Heimdall's
authenticated CultNet operation boundary on Yggdrasil loopback. Sensitive
completion and refresh fields are message-wrapped and never enter the public
CultMesh catalog, Odin records, Eve surfaces, logs, or browser responses.
Ghostlight validates the exact issuer, audience, app, profile, access revision,
session identity, expiry, and `app_access` before creating a local app session.

`ghostlight.app_session.v1` stores a hash of the random cookie, stable Heimdall
subject hash, exact Heimdall session and access revision, verified capabilities,
expiries, and a locally wrapped refresh claim. Routine requests verify that
state locally. Claim refresh and logout revalidate exact Heimdall session
custody through the private boundary; a stale refresh cannot resurrect a
logged-out session. Heimdall outages leave valid local sessions usable only
until their verified expiry. Campaign access always comes from
`campaign_membership.v1`; account preferences remember only selection. The old
auth store is frozen rollback evidence and does not participate in admission.

That boundary also projects less state outward than the kernel returns inward.
Player command responses contain only assessment and public commit/roll
receipts. Canonical campaign state and spoiler-bearing actor or institution
state are operator-only. Informational rolls add only their exact previewed
finding to the acting character and a provisional branch fact. The assessor
deterministically binds typed findings into visible stakes before validation,
so formatting is not delegated to a correction attempt. The compiler
classifies each retrieved source as direct seed, setting background, or excluded
before generation. Only direct-seed source text enters causal world compilation;
background and excluded sources remain coverage provenance and cannot donate
story incidents or cast.

The player-facing story is the chronological lowering of exact committed
`NarrativeTurn` rows. Player speech, Persona speech, and resolved outcome prose
already enter the transcript through the kernel transaction that owns their
revision. The surface does not run a second narrator or verifier model, replace
world turns, or read historical `narration_projection.v1` rows. This removes two
calls per live turn and makes display-layer invention of names, dialogue,
injuries, participants, or actions structurally impossible.

Player speech address is also typed before it can steer a reaction. A compact
Flash projection receives only the exact co-present simulation-eligible actor
IDs and names, bounded recent public turns, prior conversational focus, and the
new utterance. It returns the exact Persona actors from whom the utterance asks
for a response. `WorldKernel` revalidates those IDs and commits them with the
speech `NarrativeTurn`; neither Eve nor the native client may submit them.
Every present actor continues to appraise the turn, but the private actor slice
now distinguishes `direct_response_expected` from `present_observer`. A direct
addressee must speak or select typed deliberate silence. The kernel lowers
silence to a deterministic visible refusal, so no free-text gesture lane can
claim an unassessed world effect.

`ghostlight-campaign-inspect` is a read-only typed-store witness retained in the
immutable hosted release. Campaign-store ACLs keep it operator-only. It reports
the latest strategic cover, attributed appraisals and inactions, activity
outcomes, events, news, subject continuity, and model receipt/token metadata
without dumping provider reasoning or raw private narrative streams.

`ghostlight-mesh-inspect` provides the same read-only operator projection from
a copied derived `mesh.cc` snapshot. It never opens or copies the live campaign
store. Operators may extract one private JSON witness from that copy, then
delete the temporary mesh file; JSON is diagnostic export, not state authority.

The actor-filtered Eve surface follows the same membrane. It renders the player-owned
ledger—including that actor's own relationships—and already filtered news, but
does not enumerate canonical institution postures or raw world clocks. Those
remain operator state until narration or an exact information channel makes a
development available to the character. A negative regression plants a remote
coup posture and sealed investigation clock, proves all three secret strings
absent from the player surface, and proves an admitted public headline remains.
It likewise exposes only the number of operator pins and local active
population leaves eligible for a fission preview. Pin bodies, remote population
names, and `ReplaceResolutionPins` remain outside the player boundary.

Exact private per-member and operator projections remain CultMesh documents for
authorized Verse consumers. The public provider advertisement exposes only the
subject-scoped logical surface and Heimdall plugin requirement. Hermodr may
materialize that provider for inspection when its plugin is installed; it is
not a Ghostlight runtime dependency and owns no application state.

Global strategic context has a separate non-causal lane. Two stable broad Vault
queries run alongside local retrieval, and a Flash extraction stage proposes a
bounded pool of remote institution names with one to three exact supporting
claims each. Local code binds every retained claim to an institution-specific
witness. Only the grounded candidates, capped at 32, enter a separate Flash
synthesis stage. It preserves those canon anchors while generating the smallest
useful campaign-local doctrine needed for strategic action. An independent
verifier checks compatibility rather than textual entailment: source silence is
allowed, while contradiction, canon erasure, identity conflation, story-specific
borrowing, current branch events, and unanchored setting-breaking power are not.
Verification must still cover every grounded institution exactly once; a
malformed or incomplete verdict retries once and then aborts. After one faithful
synthesis correction, a still-incompatible doctrine aborts compilation rather
than deleting an anchored power. Compatible doctrine becomes campaign state and
an approval-visible branch assumption. Exact receipts anchor the institution and
its canon constraints; they do not falsely certify the generated policy as
Vault text. Admitted remote institutions receive deterministic coarse profiles with distinct
authority and explicit unknown facets. The Pro agency stage profiles only local
actors, populations, and institutions where semantic subdivision is useful.
The model schema permits a bounded pool of 64 candidates so provider overage
and ungrounded index fragments can reach local judgment. Exact grounding runs
before the 32-institution simulation capacity is applied; excess grounded
candidates become an approval-visible on-demand-compilation gap. Raw proposal
count therefore cannot consume canonical capacity or abort an otherwise valid
world compile. A supporting claim is institution-specific only when its quotation names
exactly that candidate or comes from the candidate's dedicated source document;
shared index headings, category descriptions, cross-faction lists, and orphaned
sentence fragments are rejected. Exact claims and receipts remain evidence;
synthesized strategic doctrine becomes branch-local coarse simulation goals.

The Aetheria Vault adapter derives document authority from the exact source
path before any model sees a witness. `Worldbuilding` is reusable canon;
published `Fiction` and legacy `Stories` are historical reference; static
interactive output is a fixture artifact; `Brainstorming` is working draft;
and `Game Design` is design reference. Only reusable canon may enter a new
branch's `direct_seed` lane or the global-agency compiler. The classifier may
further narrow canon by relevance, but it cannot promote a lower-authority
document. Direct-seed projection and receipt selection repeat this local guard,
so a malformed or stale classifier output cannot smuggle a story cast into the
world seed. The live AetheriaLore retrieval archive is correspondingly bounded
to `Aetheria/Worldbuilding/`; durable facts discovered through fiction must be
promoted into Worldbuilding before they become reusable campaign canon.

Strict provider schemas do not carry canonical dynamic maps directly. Compiler
routes and relationships cross the model boundary as explicit ID-bearing
records, then local validation rejects empty or duplicate IDs and lowers them
once into canonical maps. The same cut applies to fixed six-axis agency facets,
destination-expansion routes, population-fission assignments, and strategic
resolution-demand weights. This preserves semantic keys under providers whose
strict object dialect forbids open-ended `additionalProperties`; an empty closed
object is never accepted as a substitute for route, relationship, assignment,
or facet authority. Canonical campaign and resolution documents retain their
map-shaped storage contracts.

The Codex/OpenAI connector owns one further provider-only projection for exact
compound literals. Scalar `const` values survive. Array and object `const`
values lower recursively to the supported strict Responses subset: fixed array
lengths with bounded item literals, or closed objects with every exact key
required. Array ordering and multiplicity are necessarily wider at this
transport layer. The native request schema remains unchanged, and its consumer
validator still owns exact equality before any proposal can reach admission.
No compiler, elaborator title, or generic agent loop carries a provider-specific
repair for this dialect boundary.

`F:\Projects\Kalsa` is the first bundled fantasy Vault canary. Its typed
manifest makes the `Public` tree player-safe world knowledge and the `Spoilers`
tree GM-only canon; workshop material is non-canonical. Selection, Git/Obsidian
provenance, retrieval, exact receipts, and player projection preserve those
lanes without copying Kalsa into the read-only Aetheria recovery index or
creating a Ghostlight-owned semantic index.

Destination compilation treats Vault evidence as canon constraints rather than
an exhaustive game map. Identity resolution sends a genuinely new place to
bounded region expansion and an exact reachable canonical place to locality
elaboration. Locality elaboration preserves the existing place identity while
adding only contained places and locally scoped subjects, public facts, and
relations. A player's question can select the missing domain but cannot dictate
the office, procedure, or answer. Inhabited proposals must expose current
authority, selection or succession, public resources, and redress through a
typed civic manifest whose exact fact references are shared by each resident
population. A separate model stage verifies their semantic legibility; it may
reject but cannot rewrite or commit the proposal.

Destination repair is the first consumer of the generic model-agent harness.
The harness owner is `run_model_agent`, operating only over `ModelPort`, a
`ModelAgentSpec`, and a consumer-owned `ModelAgentTool`. The spec supplies the
stage, logical model class, frozen snapshot binding, instructions, causal
receipt IDs, per-call settings, and semantic step limit. Before every step, the
tool publishes the action schema legal at its current private state. Static
tools may route a contract from their actual domain owner; world elaboration,
for example, delegates schema derivation to `WorldElaborationAssignment`. The
harness outputs one accepted consumer-owned value plus the complete receipt
chain, or one terminal failure plus the receipts completed before failure. Step
transcript, tool observations, causal source-ID accumulation, and
`semantic_invalid` marking are derived run state. The harness has no campaign,
fact, institution, civic, kernel, or persistence type and cannot validate or
mutate world state. Patina or another consumer may supply a different action
and tool without moving that consumer's state authority into the harness.

Once the destination compiler has produced a repairable frozen civic candidate,
that exact seed initializes one private workbench owned by
`DestinationReconciliationAgentTool`. The balanced-model agent may take at most
four small typed actions: `validate_current`, or `revise_and_validate` with a
bounded transactional edit batch. The edit algebra replaces one stable fact
slot, one resident fact-ID set, one frozen institution's operational fields,
individual local relations, or the civic-manifest membership and fact lanes.
The action wire is one root object with a typed tool discriminant and nullable
edit batch; the connector refuses root-union schemas locally before provider
submission. The action schema contains no complete-candidate type. A failed edit batch
changes nothing. An admitted edit persists only in the private workbench and
returns `Continue` when the resulting draft still fails validation; its digest,
changed slots, and exact typed finding become the next observation without
reprojecting the whole draft.

Sol's seed remains the immutable owner of geography, routes, migration,
population identity and ordinary state, institution identity/name/goals and
locations, civic jurisdiction, branch assumptions, and gaps. The initial
compiler finding and every later tool finding use the same typed
`DestinationReconciliationFinding`; civic verdict booleans are not collapsed
into rationale-only text. `destination_fact_findings` remains the sole
candidate-fact namespace owner. Every current or edited draft traverses that
checker, `apply_civic_reconciliation`, ordinary lowering and structural
validation, and a fresh independent civic verdict. The verifier's structured
schema declares the same one-to-1,000-character rationale bound enforced by its
deterministic gate. It may return a verdict and receipt but cannot edit the
workbench. An accepted candidate remains only a compilation preview;
`WorldKernel` and the mutation reducer are the sole world-state admission and
commit authorities.

`ModelAgentTool` has associated typed `Action`, `Finding`, and `Output` contracts.
The harness serializes a consumer's finding only when constructing the next
model observation; it cannot invent or interpret a domain finding. The civic
verifier performs one inference for each exact candidate and returns its typed
verdict and receipt to the tool. Only `run_model_agent` owns semantic action
iteration and its four-step budget. The initial candidate plus four validating
agent actions can therefore produce at most five civic-verifier receipts. Provider
transport and schema retries remain the separately named lower-level authority
of `run_validated_stage`; they do not select or repair a civic candidate.
Accepted verifier receipts are rebound to the exact candidate before preview
admission. Completed compiler, action, tool, and verifier receipts survive
terminal failure and are persisted by both the strategic harness and the
Dungeon API. Generic harness tests prove that a later action cites both the
rejected action and tool receipts, and that exhaustion returns every completed
action and tool receipt. A tool may also return `Continue`: the valid action
updates only consumer-owned private draft state, its receipt remains valid, and
the typed observation requests the next step without pretending the task was
rejected or complete. Accepted civic systems persist as versioned component
state, so a later pass receives the current institutions, populations, facts,
and relations and must extend that apparatus instead of inventing a parallel
government.

World-elaboration tone has one typed catalog of titled workers. `Patina` owns
reusable low-stakes local texture; `Charter` civic institutions and procedure;
`Ledger` material economy; `Hearth` ordinary relationships and belonging;
`Tangle` factions and political leverage; `Veil` uneven knowledge; `Ember`
active pressure; and `Numen` ritual, magic, and wonder. Each user slider is a
zero-to-one-hundred relative dispatch weight, not prompt decoration. The
deterministic smooth weighted scheduler apportions every invocation-budget slot
against the complete enabled profile. It records requested share, actual
dispatch count, and per-title unused allocations; when a selected title is
blocked, that slot remains unused instead of increasing another title's
frequency or creating catch-up debt. A bounded wave invokes the scheduled
sub-agents in parallel. One typed wave binding names the immutable snapshot and
is passed to every invocation and returned with every proposal. Successful
proposals return in deterministic ordinal order. `WorldElaborationAssignment`
owns each dispatch's exact operation contract: it derives the instruction,
projects the generated structured-output schema down to the assigned variant
and deterministic values, and validates the submitted proposal against the
frozen snapshot. The provider-backed worker only routes that assignment through
the generic `ModelAgent`; the connector may structurally widen unsupported
compound literals on its private schema clone, while this unchanged assignment
validator retains exact semantic authority. The harness owns the bounded
two-step transcript/tool loop and receipt chain, not world semantics. Local
rejection returns as the next tool observation. Patina, Ledger, Hearth,
Veil, and Ember use the fast model class; Charter and Tangle use balanced;
Numen uses capable. The worker returns the complete action/tool receipt chain
with its proposal. A failed wave returns the consumed schedule, every completed
proposal, each failed dispatch, and all available worker receipts, so fairness
state cannot advance without an explanatory receipt.
`resume_elaboration_wave` is the sole partial-wave recovery owner. It accepts
that exact failure partition, validates it against the original immutable wave
and complete consumed schedule, retains every completed proposal and receipt,
and invokes only the failed original dispatches. It does not schedule new work
or advance fairness state. A duplicate, missing, unbound, or drifted dispatch
closes the resume seam before any worker call.
Successful wave construction is opaque outside the dispatcher. The admission
boundary rechecks requested and unused slots, per-title and total dispatch
counts, eligible titles, ordinal windows, weights, requested shares, final
scheduler state, the exact invocation partition, and every supplied worker
receipt's dispatch-specific snapshot binding before dispatch order may decide a
conflict. Every successful invocation must carry a terminal accepted model
receipt; generic receiptless scheduler fixtures cannot cross world admission.

Those proposals are not canonical mutations. Each invocation may return one
additive `WorldElaborationOperation`: place, route, branch-local fact,
population/profile pair, institution/profile pair, local or migration relation,
or civic manifest. `admit_world_elaboration_wave` rechecks the revision-bound
wave, rejects malformed operations, gives a colliding write claim to the first
authentic scheduler dispatch, and retains every later conflict with the exact
prior dispatch ordinal. It merges accepted operations into one non-canonical
`LocalityElaboration`; ordinary locality validation owns the resulting
structural diagnostic.

Titled operations cannot claim source-evidence receipt IDs or canon-candidate
authority. Facts are branch-local or provisional; profiles and relations carry
no source receipts. `WorldKernel::commit_elaboration` accepts only the opaque
finalized elaboration capability, not caller-supplied evidence or canon
candidates. Adding source-grounded elaboration later therefore requires an
explicit typed binding rather than an unattached vector at the commit seam.

A titled worker must leave the civic semantic-verifier receipt empty.
`finalize_world_elaboration` binds the immutable admitted candidate to one
independent verifier receipt that may accept or reject but cannot rewrite it.
Its ancestry must equal the complete admitted worker-receipt set, without
omissions, additions, or duplicates. Canonical Gestalt state stores shared
knowledge as fact statements; the civic verifier receives a derived exact-ID
view reconstructed from those statements and the admitted fact namespace, so
its identity-grounding judgment does not mistake the storage projection for a
missing compiler field.
Admission and finalization values are opaque, serialize-only capability
receipts; callers cannot deserialize or reconstruct them as commit authority.
`WorldKernel::commit_elaboration` consumes that final value, revalidates the
admission digest, campaign revision, candidate derivation, and verifier binding
inside its mailbox, then uses the existing `ElaborateLocality` path. That shared
path lowers the complete candidate into exact permitted `WorldMutation`
operations, applies the closed reducer, projects the accepted result, and
persists the aggregate campaign, authority, batch, and mutation receipt
atomically together with every titled-worker and semantic-verifier model
receipt. Titled workers never receive a `WorldCommand`, permit, mutation batch,
store, or kernel write handle. The current proposal algebra is additive
locality elaboration; replacement of existing canonical subjects remains
outside this authority.

#### Acceptance-driver whole-world bootstrap

- **Owner:** the strategic acceptance driver owns orchestration and recovery of
  an optional whole-world bootstrap. It owns neither geography nor canonical
  mutation. The existing destination compiler agent owns each bounded region
  proposal; `WorldKernel` alone may admit it through `ExpandRegion`.
- **Inputs:** an admitted root campaign, its exact authored world description,
  the boundary observer's canonical location, and an ordered list of at most
  eight `GHOSTLIGHT_WORLD_REGION_REQUESTS`. Each request specifies one bounded
  realm; the root compiler remains intentionally bounded-local.
- **Outputs:** one frozen ordered `world-regions-plan.json`, one immutable
  proposal checkpoint before each mutation, one sequential kernel commit per
  genuinely new realm, and an immutable `world-region-NN-checkpoint.json`
  binding request, origin, committed jurisdiction, kernel receipt, and
  model-receipt hashes. Committed
  jurisdiction IDs become the locality set for later elaboration and strategic
  simulation.
- **Derived state:** `status.json` progress and the jurisdiction list are
  orchestration projections, not world truth.
- **Forbidden writers:** the driver, root compiler, destination compiler,
  checkpoints, and elaborators cannot directly write campaign state or invent
  Delvehold-owned player, civic, dungeon, contract, workshop, or quantitative-
  economy effects.
- **Admission policy:** the operator-authorized autonomous run may admit a
  structurally and semantically valid branch-local destination even though the
  Dungeon UI would label its preview `requires_approval`. Unresolved canon gaps
  still stop the run; the driver cannot approve through them.
- **Shared paths:** fresh and resumed runs use the same frozen ordered plan,
  destination compiler, and kernel admission path. Resume rejects omitted or
  reordered requests, skips a region only when its checkpoint and canonical
  receipt agree, and recovers the kernel-commit/checkpoint crash interval from
  the immutable proposal plus canonical campaign and commit receipt.
- **Cut line:** whole-world scope is not overloaded onto the root compiler. The
  driver reuses existing destination compilation and kernel admission; there is
  no second region generator or event authority.
- **Verification layer:** destination validation and reconciliation bind each
  candidate before kernel revalidation and commit. End-to-end acceptance also
  requires all requested realms to enter locality elaboration, iterative
  complexity rounds to satisfy the consumer scale intent, the bounded
  multiresolution cover, strategic waves, and grounded newspaper projection.
  Runs 64 and 65 stopped before that claim was established and remain failure
  evidence rather than accepted full-world runs.

#### Iterative latent-world complexity and elaborator continuity

- **Owner:** the strategic acceptance driver owns round orchestration,
  deterministic titled-elaborator scheduling, checkpoint publication, and
  bounded termination. `WorldScaleIntent` is consumer-authored;
  `derive_world_elaboration_demand` deterministically owns the target, deficit,
  realm shares, and per-round mutation pressure. Each titled elaborator owns
  only its compact title-by-realm working frontier and mutation draft. The
  acceptance driver owns deterministic parent-to-jurisdiction routing;
  `WorldComplexityTool` owns construction of the complete frozen proposal from
  the draft. `WorldKernel` remains the sole canonical mutation owner, and
  Resolution remains the owner of the later active cover.
- **Inputs:** the admitted campaign after region and locality elaboration; its
  active-cell entitlement; a target cover ratio; equal acceptance-driver realm
  weights; a weighted titled-elaborator profile; bounded parallelism and round
  limits; active, simulation-eligible Gestalt leaves without a currently
  materialized member; the prior title-by-realm session checkpoint; and the
  exact frozen parent state, members, profile, and local relations for each assigned
  mutation. Scheduler title deterministically selects the proposal lane:
  Hearth and Veil individuate one consequential member; the other six titles
  fission the assigned population along their fixed agency axis. The model sees
  the assigned parent projection and compacted session, but its action schema
  admits only the lane-specific delta. A fission draft contains child IDs,
  names, partition values, the residual child, and exact member/resource
  assignments. An individuation draft is a compact
  `WorldComplexityMemberDraft`: the model owns only the proposed local ID,
  public name, capability and knowledge deltas, equipment, conditions,
  obligations, exact-ID relationships, goals, and memories.
  `elaboration::unique_containing_jurisdiction` is the single read-only owner
  for deriving parent jurisdiction from canonical topology. For every exact
  operating place in `AgencyProfile.location_ids`, it walks
  `Location.container_id` ancestry until it reaches a consumer realm named by
  `realm_subject_targets`. Exactly one
  distinct realm target returns that jurisdiction. Zero targets or multiple
  distinct targets return no jurisdiction; a containment cycle terminates that
  ancestry walk without inventing one. Strategic candidate allocation and
  title-by-realm session routing call this same resolver, and
  `ModelWorldComplexityWorker` constructor admission calls it again against the
  assigned jurisdiction before accepting the parent-to-jurisdiction map.
- **Outputs:** each round publishes an immutable frozen preview with demand,
  wave and schedule receipts, exact parent bindings, proposals, and model
  receipt hashes; then one immutable mutation checkpoint per sequentially
  admitted fission or individuation; one compacted checkpoint for each title
  that obtained an admitted mutation in that round; and a terminal round
  checkpoint carrying the scheduler state, mutation paths, the accumulated
  title-by-realm session checkpoints keyed by stable
  `<lowercase-title>:<jurisdiction-id>` session ID, and remeasured
  actionable-subject count. Per-round session filenames contain the title plus
  a stable digest suffix of that session ID; filenames are storage addresses,
  while the map key and checkpoint's own `session_id` carry routing identity.
  The final count and round reports are acceptance metadata, not canonical
  world state.
- **Derived state:** actionable complexity is the count of canonical active,
  simulation-eligible agency leaves. Dormant member rows, census texture,
  model calls, dispatches, mutation budget, and compacted prose do not satisfy
  the target. Inherited child capabilities, knowledge, goals, pressures,
  evidence fields, campaign/revision binding, approval requirement, parent
  version, partition axis, and individuation location are deterministic
  projections of the frozen assignment, not model-authored state. An
  individuated member's public name is presentation state attached to its stable local
  member ID and canonical `member:<local-id>` subject ID; name equality does
  not merge subjects, resolve an address, or grant authority over another
  Actor or population member.
  checkpoint's title, jurisdiction, frontier summary, unresolved leads,
  rejection findings, digest chain, and recent commit IDs are bounded steering
  memory. Raw provider sessions, request history, and transcripts are neither
  continuity nor world state; the checkpoint is the only carried elaborator
  context, while the campaign and kernel commit receipts remain truth.
  Resolution cover membership and Nemesis decision windows are downstream
  derivations and do not feed back into complexity admission authority.
- **Forbidden writers:** elaborators, session compaction, the scheduler,
  demand derivation, preview files, round files, and acceptance metadata cannot
  write campaign state, declare a target satisfied, allocate the Resolution
  cover, or turn narrative memory into facts. A fission proposal cannot approve
  or commit itself, invent evidence, promote raw roster rows, or rewrite a
  parent. An individuation cannot invent a population, duplicate an existing
  exact actor or member ID, materialize itself, or acquire relationships to
  nonexistent subject IDs. Public names are not writers of identity or
  authority: no lookup, relationship, action target, or admission decision may
  select or merge a subject by display-name equality. The model cannot emit or
  override the member schema, parent Gestalt ID, version, exact location,
  materialization state, or relevance revisions because those deterministic
  fields are absent from `WorldComplexityMemberDraft`. The model cannot restate
  or alter inherited child
  Persona constitutions, campaign identity, parent identity/version, location,
  partition axis, evidence, or approval authority because those fields do not
  exist in its mutation-draft schema. The model provider cannot select a realm,
  merge title sessions, persist a transcript as memory, or redirect a
  checkpoint to another title or jurisdiction. Sequential commits may update
  only the optimistic revision of a frozen fission proposal after the exact
  parent binding remains unchanged; individuation instead revalidates its exact
  Gestalt version and location against current state.
- **Shared paths:** all round workers use the existing generic elaborator
  dispatcher and weighted schedule. `WorldComplexityTool` first lowers a
  compact draft into the existing full proposal algebra. For fission, it clones
  the frozen parent Persona into every child, replaces only child identity,
  name, version, and assigned resources, builds the child-partition map,
  attaches the exact campaign/revision/parent/axis and empty
  evidence/gap/canon fields, and fixes `requires_approval=true`. For
  individuation, it lowers `WorldComplexityMemberDraft` into the canonical
  `GestaltMemberDelta`: it fixes the member schema, parent Gestalt ID, version
  zero, exact first parent-profile location, unmaterialized state, and zeroed
  relevance revisions around the model-owned semantic member content. Every
  constructed proposal then passes the existing fission or individuation
  validator and the existing
  `WorldCommand::FissionGestalt` or `WorldCommand::IndividuateGestaltMember`
  mailbox. `resolution::validate_gestalt_individuation` is the single local
  authority for exact member-ID uniqueness, parent version, exact location,
  bounds, and exact relationship subject IDs; strategic action-bound selection
  and direct kernel admission both reuse it. Canonical member addressing uses
  `canonical_gestalt_member_local_id` and `gestalt_member_subject_id`, while
  strategic actions and relationships carry exact subject IDs rather than
  public names. It accumulates every failed deterministic clause in
  one rejection so the generic agent loop can repair the exact bounded draft in
  its next semantic step; the tool does not add a second judge or relax kernel
  admission. Direct individuation then follows the ordinary
  presence path: `apply_individuation` inserts the canonical member delta and
  promotes it into an Actor, `commit_gestalt_presence` calls
  `ensure_agency_profiles`, and the new active simulation-eligible actor profile
  becomes visible to `canonical_actionable_subject_count`. Parallel workers
  read one frozen campaign; commits serialize through WorldKernel. Unrelated
  earlier fissions may be rebased,
  while any change to the assigned parent, its profile, or its members rejects
  the later proposal instead of silently reconciling it.
  Compaction runs only after successful mutations have produced admitted commit
  journals routed by the invocation's stable title-by-realm session ID. The
  resulting checkpoint is stored under that ID and supplied only when a later
  dispatch has the same title and deterministically derived jurisdiction.
  The existing `ElaboratorSessionCompactionTool` owns the model-boundary
  compaction contract: its action schema requires a nonempty frontier summary
  of at most 4,000 characters, at most 32 unresolved leads, and each lead to
  contain 1 through 600 characters. Tool admission rechecks those same bounds
  and returns the complete indexed mismatch set; checkpoint construction then
  revalidates the identical shape before hashing and persistence.
  The exact failure partition, frozen schedule, completed proposals, receipts,
  and diagnostics remain in the resumable failure checkpoint; they are not
  folded into successful session compaction.
- **Cut line:** this cut reuses Gestalt fission, strategic Gestalt
  individuation, WorldKernel admission, model receipts, the generic scheduler,
  and immutable acceptance checkpoints. It adds no second population or member
  insertion path, campaign store, parallel commit owner, cover allocator, or
  transcript-shaped session log. `WorldComplexityAction` is a model-boundary
  draft, not a new canonical mutation or persistent state authority; the
  existing `WorldComplexityProposal` remains the checked checkpoint and kernel
  handoff. The prior partial-wave checkpoint remains recovery for one frozen
  dispatch partition; it is not
  elaborator memory. The title-by-realm checkpoint map replaces both the former
  title-only global memory and any temptation to preserve provider transcripts;
  it introduces no provider-session store or second jurisdiction allocator.
  Compact checkpoints replace accumulated working history without replacing
  canonical commits. Individuation admission requires unique exact IDs, not a
  global public-name registry; the cut adds no alias table, fuzzy resolver, or
  name-disambiguation authority. `WorldComplexityMemberDraft` is only the
  model-boundary semantic payload; it is not a second member-state schema or
  persistence owner, and deterministic lifecycle fields do not cross the model
  boundary. The compaction repair adds no compactor, checkpoint type, session
  store, or memory authority; it closes the item-bound seam between the
  existing model schema, tool admission, and checkpoint validator.
- **Verification layer:** checkpoint shape and digest bind title, session,
  campaign, target location, generation, world revision, prior checkpoint, and
  bounded narrative fields. Frozen parent digests make conflicting parallel
  mutations uncommittable. Existing fission validation proves lineage, child
  partitioning, resource custody, and member transfer; shared individuation
  validation proves active parent and location, exact version, unique bounded
  member/Actor IDs, exact valid relationship targets, and non-materialized
  member state. Equal public names on distinct exact IDs remain admissible and
  do not change address resolution or authority. The focused tool regression
  admits a new member whose name matches an established Actor, still rejects a
  colliding canonical Actor ID and unsupported relationship subject, and
  confirms name collision is absent from the rejection. WorldKernel
  revalidates either lane before atomic mutation. The model schema exposes only
  the assigned operation, bounds fission children to two through eight,
  requires exactly the frozen parent resource and dormant-member IDs as
  assignment-map keys while leaving child custody model-authored, and for
  individuation exposes only semantic member fields and contains no `schema`,
  `gestalt_id`, `version`, `last_location_id`, `materialized_actor_id`,
  `last_relevant_revision`, or `relevance_lease_until_revision`; a focused
  schema regression proves those deterministic fields are absent and the tool
  still constructs an admissible canonical individuation. Tool-level
  construction additionally rejects duplicate child IDs/partitions or
  incomplete exact
  member assignment before shared semantic validation. The shared fission
  validator owns one complete deterministic mismatch set, including frozen
  binding, child structure and inheritance, member assignments, resource
  conservation and custody, approval, and parent profile eligibility; every
  caller receives that set rather than a generic first-failure umbrella.
  Worker construction requires exactly one jurisdiction for every assigned
  parent, rejects extra routes, and reuses
  `unique_containing_jurisdiction` to prove that the assigned jurisdiction
  contains that parent's exact profile locations. Resume and
  worker construction require every checkpoint
  map key to equal both the embedded session ID and the session ID recomputed
  from its embedded title and target location; checkpoint validation binds
  those fields to its digest and current campaign. Runtime lookup
  derives the expected ID from the current dispatch title and routed realm.
  Routing tests prove that equal titles in different realms and different
  titles in one realm produce different IDs. Each round must strictly
  increase the canonical actionable count; completion requires a freshly
  derived zero deficit rather than exhaustion of calls or rounds. Focused tests
  cover scale arithmetic, checkpoint tampering and cross-title rejection, exact
  compaction ancestry, assigned-parent validation, and safe revision rebasing.
  A focused compaction regression proves the generated schema carries the
  per-lead 1/600 character bounds and that empty and overlong leads in one
  action produce separate index-addressed findings rather than one generic
  combined diagnostic.
- **Failure and resume:** a provider failure publishes an immutable
  round-failure generation with the exact completed and failed dispatches and
  receipts, then stops that run. Explicit resume rehydrates the latest failure,
  preserves completed invocations, and invokes only the failed original
  dispatches through `resume_elaboration_wave`; repeated failures publish
  numbered generations without changing the frozen schedule. A published
  successful round preview may also be reused on explicit resume. Existing
  mutation checkpoints suppress replay, title-by-realm compaction checkpoints
  suppress recompaction, and consecutive completed round checkpoints restore
  scheduler fairness and session memory before the first unfinished round.
  After mutation admission, independent title-by-realm journals compact in
  parallel under the same bounded complexity parallelism entitlement. Each
  compactor reads the same committed campaign revision and only its own prior
  session checkpoint and journal; the runner persists every successful model
  receipt and immutable session checkpoint before returning any sibling
  failure. Session checkpoints remain independent memory owners rather than a
  shared round transcript, and canonical mutation order remains serial.
  Tool-internal rejection findings are not currently copied into later
  successful compaction. That is a bounded continuity limitation: the compacted
  title-by-realm mind may lose a useful rejected path, but no live proposal is
  orphaned because exact failed dispatches remain owned by the resumable failure
  checkpoint. This limitation was not the Run 93 blocker.
  Missing eligible parents, stale or malformed checkpoints, changed parent
  bindings, zero-growth rounds, and a nonzero deficit at the configured round
  ceiling are terminal failures. Canonical CultCache state, not the checkpoint
  prose, decides whether already admitted mutations exist after interruption.
  Parallel fission proposals may independently choose the same globally scoped
  child identifier. Rebase treats that as syntactic contention rather than a
  semantic contradiction: only an occupied child ID is deterministically moved
  into the assigned parent's content-addressed namespace, with partition,
  residual, member, and resource references rewritten together before the
  ordinary complete fission validator runs. Names, partition meaning, custody,
  and membership remain model-authored; canonical state still admits the result
  only through WorldKernel.

One boundary remains intentionally narrow in this acceptance-driver cut.
Fission and named-member individuation are live, but institution promotion is
not. Dormant member rows still do not count; the individuation lane earns its
increment by atomically promoting the admitted member into an Actor and deriving
that Actor's active agency profile through the shared presence-commit path.
Materializing one person does not retire their population as an elaboration
parent. A later fission transfers every exact member affiliation once. Dormant
members have their deltas rebased against the selected child baseline;
materialized people keep their Actor state byte-identical while their backing
member row moves to the selected child. The complete member-assignment map,
component membership transition, shared fission validator, and WorldKernel
commit remain the single path for both cases.
Architecture claims that complexity has no production caller, no durable
elaborator compaction owner, no partial-failure resume, or only a fission
proposal lane are stale.

The strategic acceptance runner exercises this path as a deliberate hybrid.
The existing destination compiler first commits the complete civic foundation
for one canonical locality. A second revision-bound titled wave then adds one
Patina child place with exact reciprocal routes, preserves the next civic
manifest version through Charter, and admits title-specific facts and a
political relation before independent civic verification and a second kernel
commit. The acceptance profile allocates three Patina calls and two calls to
each other title per locality; a seventeen-slot pass therefore invokes workers
in exact configured proportion under bounded parallelism. This proves the
titled path against a real provider without claiming that the older civic
foundation compiler has already been replaced by the swarm.
The strategic acceptance harness persists compiler, foundation, titled-wave,
and completed strategic-wave checkpoints beside the CultCache campaign. On an
explicit resume it loads canonical world state from CultCache, rehydrates model
receipts by content hash, restores scheduler state from the consumed schedule,
and continues at the first incomplete authority boundary. A committed
foundation is checked against its preserved civic apparatus. A titled preview
is not completion authority: only a post-kernel completion checkpoint bound to
the before/after revisions, semantic verifier, model receipts, exact
`WorldCommitReceipt`, and the kernel-owned mutation authority, batch, and
receipt may skip that commit. New checkpoints also bind the admission digest;
legacy inference instead binds the finalized candidate through the persisted
mutation proof before minting its checkpoint. The batch's
`intended_effect_digest` binds the complete finalized candidate before
deterministic aggregate projection, so the
resume verifier does not duplicate the reducer's set and ordering rules.
Run-32's older committed Canopy pass can be admitted once through that persisted
mutation proof plus its complete civic candidate, then gains the same durable
completion checkpoint. A partial titled wave routes through
`resume_elaboration_wave`. Each later terminal partial result is published as a
new immutable generation by same-directory atomic rename, preserving the prior
valid generation. A committed strategic tick without its matching durable wave
checkpoint is refused rather than guessed or replayed.

Both destination paths synthesize the smallest compatible branch-local routes,
geometry, people, supplies, procedures, capacity, responsibility, and doctrine
needed for play. Those choices remain visible as `branch_assumptions`; they may
vary between campaigns unless the Vault pins them. A `gap` is reserved for
contradictory canon baselines, an explicitly requested exact canon baseline that
cannot be anchored, or conflict with an approved capability. Ordinary source
silence is not a gap. Preview approval remains the only path from this proposal
into canonical world state.

Autonomous strategic waves run the same admitted agency state through parallel
cell membranes under a provider concurrency gate. Action-bound individuation
may create one consequential named person through the existing Gestalt member
commit primitive; that person enters later covers as the same canonical actor.
Public action channels produce committed `NewsIssue` records. The world-
consumer boundary admits an authority-gated editorial-voice request, then
validates the exact `NewsIssue` to committed-`Event` public-channel chain.
The public-event owner derives each committed event's human-readable account
from its typed effect and canonical subject names. Event semantics own the
assertion-status mapping used by every public consumer. Relation coordinates,
numerical deltas, action-selection rationale, and other state machinery do not
become public facts merely because a newsroom exists.

The deliberative newsroom boundary is live under this authority map:

- **Objective:** one publication investigates the immutable public record,
  selects the facts its actual stories require, and constructs a pointed
  narrative without inventing world state or paying for a duplicate source
  ontology.
- **Owner:** canonical `NewsIssue` plus `Event` rows own public provenance and
  facts. `Event::public_assertion_status` owns attempt, declaration,
  committed-course, material-outcome, and unspecified-account semantics. The
  in-world assignment editor owns assignment: story choice, order, section,
  journalist, issue shape, per-story narrative function, context role, focal
  record, exact citation grouping, throughline, tension, assignment rationale,
  and reader question. Its
  deterministic workbench owns derivation of the bounded period directory,
  query admission, inspected-record membership,
  the bounded working-context projection, exact-ID fetch, agenda validation,
  focus-record dereferencing, candidate identity, and exact commit admission.
  The generic agent harness owns message framing: for snapshot tools it freezes
  the complete first request as one user message and replaces only the later
  workbench message. The tool owns the snapshot content. The provider owns
  automatic prefix-cache admission, retention, and hit accounting; neither the
  harness nor the newsroom may declare a hit, and newsroom correctness or
  acceptance cannot require one.
  The production-checkpoint validator owns whether an admitted agenda or
  accepted reporter filing is exact enough to resume; `CampaignStore` owns its
  immutable storage, not its editorial judgment. Each assigned in-world
  journalist owns one reader-facing
  story and receives only that pitch, its exact records, and that journalist's
  stable recurring character: beat, voice, biases, preferences, source
  instincts, and blind spots. The copy editor owns one complete factual
  query report over the assembled page; it neither rewrites nor accepts
  publication. The Night
  Editor owns one deadline close over that page and report. The deterministic
  press witness owns structural admission, lineage, and exact before/after
  evidence for what printed; it does not judge prose. Errors discovered after
  close belong to a later edition's correction memory, not a same-edition
  repair loop. V13 defines that ownership boundary but does not yet persist a
  typed correction record. Publication employees are not world actors or event
  writers. `WorldKernel` remains the only world-event writer.
- **Inputs:** one exact campaign revision, publication title and voice, article
  budget, a validated embodied newsroom roster, source-receipt ancestry, and
  `character-newsroom.v16` contract. Before its first query, the assignment
  editor receives a bounded per-period directory containing only timestamps,
  completed-material-change headlines, the full completed-change count, and
  counts of other records. Headline sampling preserves canonical ledger order
  within a period rather than lexically privileging whichever headline sorts
  first. This is
  navigation context: it cannot be cited, does not make a record inspected, and
  does not replace query or exact-ID fetch. Each reporter receives the common stable
  house charter first, that journalist's stable recurring identity second, and
  the current assignment workbench last. The assignment editor receives its own
  staff profile plus the complete journalist roster; it does not receive the
  copy editor or Night Editor profiles. Through the
  existing generic agent harness, the selector issues deterministic bounded
  queries over stable `NewsIssue` IDs using literal terms, exact asserted
  public entity names, assertion statuses, channels, newest/oldest ordering,
  and an already-inspected cursor. An empty query browses. It may also fetch at
  most 24 complete records by exact stable ID. Each response returns at most 24
  exact projections and the agent has at most twelve semantic steps;
  the complete immutable ledger remains queryable rather than being split into
  privileged recent and foundational windows. The harness receives the stable
  role charter, semantic-step budget and output schema plus the tool's optional
  current snapshot and latest observation; changing workbench state is not an
  input to the frozen first message.
- **Outputs:** an agenda proposal does not finish selection. It yields one exact
  candidate digest plus a deterministic front-page proof: the proposed lead's
  complete focus record, the complete focus record for every below-fold pitch,
  compact supporting-record identities, the proposed narrative claims,
  tensions and public questions, and explicit comparison questions. The agent
  may query again or replace the candidate. Selection finishes only when it
  submits `commit_agenda` with the exact digest of the current reviewed
  candidate. The committed agenda is immediately stored as one immutable
  production checkpoint before any reporter runs. It carries one dominant throughline, reader
  stake, evidence-based special-or-general issue shape, and ordered story
  pitches. Every pitch names stable public-record IDs, chooses one focus record,
  section, journalist, narrative function, and context role, and supplies bounded
  assignment rationale,
  narrative claim, tension, and public question. Accountability, opposition,
  and counter-narrative pitches also carry a typed conflict axis: two distinct
  named parties, two distinct selected exact citations, bounded descriptions of
  each party's public move or position, and the editorially framed conflict.
  Deterministic admission proves both parties are asserted by their cited
  records and rejects one record presented as both sides of a dispute; an
  abstract antagonist or passive filing recipient cannot impersonate an
  opposing move.
  Consequence and independent stories may omit the axis when the ledger has no
  honest adversarial motion. Article count plus bounded paragraphs and output tokens
  define page space. There is no citation-count page budget. A foundational
  record may support several continuing stories when each pitch uses it for a
  distinct throughline. One journalist agent files each assigned story and the
  page is assembled deterministically from assignment-owned section, byline,
  and citations plus journalist-owned prose. Every accepted filing is stored as
  its own immutable production checkpoint before the next reporter runs. The
  cited facts deterministically derive `allowed_datelines` for the reporter's
  dynamic workbench. The generic stable output schema admits the dateline's
  syntactic shape, while `validate_editorial_article` alone admits it against
  places asserted by that filing's cited records: the lead must choose a cited place
  when one exists, while a later article may choose a cited place or remain
  blank. The copy desk emits one
  assessment plus the complete bounded set of exact factual query passages;
  the Night Editor emits one close action; and accepted composition v3 carries
  the printed issue, copy report, press-close witness, and exact receipt chain.
  Each snapshot-agent run also emits a provider request sequence whose first
  request is one stable user message containing the complete role charter and
  step-one directive. Every later request begins with that exact message and
  adds one second user message containing only the current bounded workbench
  state, latest observation, and next-step directive.
- **Derived state:** public-record projections contain their stable ID,
  timestamp, channel, headline, reliability, exact committed accounts,
  assertion statuses, committed event IDs, and only those people,
  institutions, populations, places, and identity attributes asserted by the
  account. The per-period directory groups those projections by exact
  timestamp, retains at most twelve distinct completed-change headlines per
  period, and counts every other record. It is a lossy navigation projection
  with no factual, selection, or citation authority. Query pages, the visible-ID set, pending agenda, candidate digest,
  front-page proof, review questions, and late reader-facing citation numbers
  are transparent derived views. After each nonterminal selector step, the
  workbench derives a compact inspected-record index containing only record ID,
  time, channel, headline, and asserted named entities, plus query/record counts
  and the pending candidate token; full fact bodies return only through the
  bounded query or exact-ID fetch. Production checkpoint IDs, reporter-assignment
  digests, receipt-chain and complete checkpoint-content digests are also
  derived. `allowed_datelines` is deterministic prompt guidance derived from
  the cited facts, not factual admission authority; it remains in the dynamic
  workbench rather than changing the reporter's generic provider schema and
  provider request identity. `validate_editorial_article` owns factual admission
  after the model returns.
  The cacheable prefix is the byte-stable first request plus its unchanged output
  schema; the dynamic suffix is the second workbench message. Provider-reported
  cached-token counts are optional observations of that external cache, not a
  newsroom invariant, acceptance gate, or proof that a particular request will
  hit.
  The query-index set, query-bearing article
  set, changed-article set, source-page digest, printed-page digest, and
  receipt-chain digest are also derived. None is a persisted world subject,
  fact bundle, relevance authority, prose judge, or event writer.
- **Editorial path:** the selector may use the period directory to choose where
  to investigate, but it must inspect a record through query or exact-ID fetch
  before citing it. It
  can search backward from routine updates to the original rupture and can
  acknowledge that later handling is an installment in a continuing story.
  It first proposes an agenda. The workbench validates it, dereferences the
  focus record of every pitch, places the proposed lead beside the stories it
  would bury, and returns the exact candidate digest. This is an observation,
  not acceptance. The agent must reconsider the actual consequences in that
  proof and explicitly commit the current digest; a replacement proposal
  retires the prior candidate as commit authority. Once committed, the exact
  task/editorial-bound agenda checkpoint is the sole reporter-assignment source
  on retry. In a special issue, only the lead may own shared chronology; later
  stories assume it and perform distinct narrative functions. In a general
  issue, every story owns independent context. Each assigned journalist sees
  the common publication house charter first and only that journalist's
  recurring character second in the stable prefix. Issue shape, page
  throughline, one pitch, its exact records, and the deterministically derived
  `allowed_datelines` arrive last in the replaceable workbench message.
  They center their story on its focus record and may not widen or regroup the
  selection. Each accepted filing is bound to that agenda checkpoint, article
  index, exact pitch, journalist identity and profile, and local reporter
  snapshot binding before it becomes reusable. Record bookkeeping, memory retention, maintained
  warnings, and procedure are not automatically the lede merely because they
  are recent. The copy editor sees the assembled page and selected facts. The
  Night Editor sees only queried articles, their assignments and cited facts,
  the numbered checklist, its own profile, and the profiles of the affected
  reporters.
- **Desk and press path:** accepted stable IDs lower once into the existing
  self-contained semantic audit with source news IDs, source timestamps,
  channels, reliability,
  exact account text, assertion status, event IDs, supported identity
  attributes, institutions, populations, and places. Reader-facing numeric
  labels are assigned only during lowering. The copy desk reads the selected
  fact projection and assembled page exactly once, returning all factual queries
  at once as exact unique passages with reasons. The immutable close checkpoint
  binds that report, page, agenda, task, sources, and receipts before the Night
  Editor acts. The Night Editor receives the affected assignments, queried
  articles and facts, and numbered query list, then gets one action to
  disposition every query and provide complete replacement prose for exactly
  each queried article. The
  press workbench freezes article selection, order, section, byline, citations,
  and agenda and forbids changes to every unqueried article. It validates structure and source
  membership, not whether the final language is true or compelling. A structurally invalid
  close exhausts the one action and prints the checkpointed journalist page with an
  explicit non-applied close witness. There is no post-close model reread.
- **Agent context path:** every run begins with one user item combining
  the stable charter and the step-one directive. For a tool that owns a bounded
  current context snapshot, that complete first request remains byte-identical;
  after each nonterminal action the harness discards old model chatter and
  observations and replaces only a second user item with the current snapshot,
  latest finding, and next-step directive. This gives the provider an implicit
  cache breakpoint after request one without creating an application cache or
  explicit breakpoint API. Tools that publish no snapshot retain the ordinary
  append-only conversation path after the same single-message initial request.
  The assignment workbench uses this seam for its compact
  inspected-record index and pending candidate token. Reporter workbenches use
  the same seam: the common house charter precedes the recurring journalist
  identity in the stable first message, and the dynamic assignment snapshot is
  the second message. Exact-ID fetch
  restores only the requested full records. Selector query, fetch, proposal, and
  commit share one stable structured action schema. The Night close schema is
  likewise stable across checklist contents; its private workbench enforces the
  exact queried set. Tool schemas stay in the provider output contract rather
  than being duplicated in prompt text.
  Prompt-cache routing binds stage, model class, and exact output schema;
  changing world context does not mint another routing key, while changing the
  contract does. Cache reuse is meaningful inside repeated calls of one live
  agent conversation, including the append-only path. Independent reporter
  filings are sporadic one-call agents and may outlive provider retention, so
  their cache eligibility is telemetry rather than a design requirement. Each
  journalist receives only its own embodied profile, one
  pitch, and that pitch's records; it does not receive other reporters or desk
  staff. Copy receives its own embodied profile plus the complete selected fact
  desk and assembled page because it alone checks the page. Night receives its
  own embodied profile plus only the queried articles, affected assignments and
  facts, numbered checklist, and affected reporter profiles.
- **Invariants:** private events do not enter the query surface; every selected
  stable ID resolves at the bound revision; story order, selection, citation
  grouping, focus, section, and journalist remain agenda-owned; no proposal reaches the journalists
  without deterministic proof observation and exact-current-digest commit;
  omitted public facts remain true; assignment, writing, copy queries, close,
  lowering, and witness creation cannot mutate the world. There is one bounded
  reporter pass per pitch, one copy report, and one admitted Night Editor close. No model verdict
  rereads the printed page, no rejected close opens an iterative rewrite loop,
  and no external evaluation can reopen an immutable edition. The witness must
  preserve exact source and printed page digests, receipt ancestry, every query
  disposition, every changed article index, and deterministic re-derivation of
  the printed issue. Admitted agenda and accepted filing checkpoints survive a
  later reporter or copy failure without becoming printed truth; reuse requires
  exact current task, editorial, assignment, stage, snapshot, and receipt
  bindings. A directory headline is never citation evidence: every assigned
  source must still have been returned by exact ledger query or exact-ID fetch,
  and every new audit citation exposes the canonical source timestamp.
  Historical v13 citations decode with an empty timestamp list and retain their
  exact old audit rendering and serialized receipt shape; empty legacy time
  lists are omitted from serialization. The source-time line and field appear
  only when canonical timestamps are present.
- **Forbidden writers:** the selector and query workbench cannot create,
  mutate, deduplicate, summarize into, or reclassify `NewsIssue` or `Event`
  state. The deterministic proof cannot rank stories or commit an agenda; it
  exposes the model's proposed ordering against exact focus facts. A proposal
  cannot bypass review by returning as accepted output, and a missing or stale
  candidate digest cannot commit. A journalist cannot change section, byline,
  citations, assignment, another journalist's prose, or the admitted record
  set. The copy desk cannot rewrite, suppress, or accept
  prose, select stories, close the press, or withhold the complete query set for
  a later pass. The Night Editor cannot widen sources, add, remove, or reorder
  stories, alter sections, bylines, citations, or agenda, invoke another model
  judge, reopen a completed close, or touch an article absent from the copy
  checklist. Every copy-query-bearing article and no other article is mandatory.
  The press witness cannot certify
  factual or editorial quality; it can only prove the exact admitted process and
  bytes.
  A context snapshot cannot create or summarize facts into authority. The
  editorial-period directory cannot create a source packet, mark a record
  inspected, satisfy citation admission, or become an event authority. An
  exact-ID fetch cannot mutate the ledger. A production checkpoint cannot change
  the roster, agenda, pitch, reporter, article slot, section, byline, citations,
  source campaign, or receipt ancestry; a checkpoint from another contract,
  task, or editorial binding cannot resume current production. Neither the
  generic dateline schema nor workbench `allowed_datelines` can admit factual
  truth; `validate_editorial_article` rejects an uncited place and rejects a
  blank lead dateline when a cited place is available.
  A snapshot tool cannot rewrite the frozen first request, insert changing state
  ahead of it, or promote provider cache telemetry into semantic authority. A
  cache miss cannot change tool admission, receipts, or newsroom results, and a
  routing key cannot substitute for provider-reported cached tokens.
  Renderers, strategic smoke, Registry, external blind reviewers, and grounding
  reviewers cannot write the page, desk report, close, correction, world state,
  or acceptance state. Caches cannot decide record equivalence, assertion
  status, story relevance, fact completion, or staff assignment.
- **Shared paths:** fresh composition queries stable records, proposes one or
  more agendas, observes the deterministic front-page proof for each proposal,
  and commits exactly one current candidate before invoking the assigned
  journalists. Fresh work then follows one path: one bounded reporter agent per
  pitch, deterministic page assembly, copy report, immutable close checkpoint,
  one Night Editor action, deterministic press witness, lowering, persistence.
  Registry, live per-wave composition, missing-issue recovery, and final digest
  all delegate to `advance_world_newspaper`. Resume checks, in order, an accepted
  composition, an exact-current close checkpoint, the exact agenda production
  checkpoint, and each reporter filing checkpoint. It reruns only the first
  missing stage: an interrupted reporter pass resumes at that reporter; a later
  copy failure reuses assignment and every accepted filing; a close resume does
  not rerun assignment, journalists, or copy desk. A
  persisted composition is revalidated against its close checkpoint and exact
  stored receipts, then returned without model work. Every caller receives one
  completed `WorldNewspaperComposition` or an error; there is no pending
  reconciliation result for a caller to interpret or advance.
  Generic snapshot agents and the assignment editor share the same harness
  framing primitive; non-snapshot agents share its single initial message but
  continue through the append-only observation path. No newsroom-specific cache
  branch owns either route.
- **Persistence and compatibility:** the live internal contract is
  `character-newsroom.v13`. Public request schema v3 carries the publication
  roster and binds it into the task and editorial identities. Private production
  checkpoint v1 stores either the admitted agenda or one accepted reporter
  filing. Agenda identity binds the exact task and editorial identities; filing
  identity additionally binds its agenda checkpoint, article index, exact pitch,
  complete journalist profile, assignment digest, local reporter snapshot, and
  receipt chain. These checkpoints are durable production inputs, not copy-desk
  acceptance, press authority, or world facts. Close checkpoint
  v1 binds the assigned agenda, assembled page, single copy report, task and
  exact editorial bindings, and immutable receipt chain. Its only valid origin
  is `InitialCopyDesk`, and its compatibility-shaped `source_checkpoint_id`
  must be absent. The private `LegacyV7Checkpoint` enum spelling is a read-only
  deserialization tombstone: the loader filters it before task selection,
  receipt loading, validation, or close authority.
  Composition v3 stores
  the printed issue, copy report, press-close witness, and receipts. Public issue
  schema v3 remains unchanged. Uncommitted candidate agendas and front-page
  proofs remain ephemeral. Historical v7 reconciliation rows may remain in archived or
  resumed stores as inert provenance, but the live newspaper module defines no
  reconciliation loader, import envelope, or conversion path and does not
  rebind them. Historical strategic recomposition v1/v2 receipts
  are a separate acceptance-driver verification surface; their typed validators
  can prove original artifacts but cannot write newsroom state. Resume authority
  belongs only to exact-current v13 production and close checkpoints bound to
  the same task, newsroom, facts, assignment, article slot, and receipt prefix.
  Run 58's v7 tip and Run 59's printed edition remain immutable evidence in
  their original artifacts, not v13 resume inputs.
- **Cut line:** the generic byline list and one-shot whole-page writer are
  deleted. Assignment-owned section, journalist, byline, citations, and order
  are no longer regenerated in prose output. Tool schema text and prior-action
  copies are absent from prompts; agent history is no longer flattened and
  retransmitted as a newly rendered user transcript. Selector commands no
  longer mutate the action schema as records become visible. Phrase-level
  repairs, repeated copy calls, Night rereview, post-close model judgment, and
  edits to unqueried articles remain absent. Whole-article prose is the sole
  Night Editor close unit, and exactly the query-bearing articles are mandatory.
  The legacy reconciliation schemas and advance loop, automatic store lookup,
  explicit import API and environment path, v7-to-v9 close conversion, and
  pending consumer branches are deleted from live authority. Existing bytes are
  not migrated or erased; nothing in the live path can make them decide a close.
  The snapshot harness's former two-message first request is also deleted: the
  step-one directive no longer sits in a prefix position that disappears when
  the first workbench snapshot replaces history. The cut adds no cache store,
  cache key registry, manual retention control, or retry compensator.
  V15 adds the cited conflict axis and removes lexical bias from the bounded
  directory. V14 adds only the bounded assignment-navigation directory and source
  timestamps in the audit. It does not add a source-packet layer, second event
  authority, alternate evidence selector, or reporter-context expansion.
  The roster, query projection, front-page proof, and article assembly reuse the
  generic agent harness and existing public-record, agenda, accepted issue, and
  audit shapes. One private persistent production-checkpoint schema buys exact
  pre-copy resume; no dependency, crate, binary, daemon, service, cache, event
  writer, target platform, code-generation path, or world-simulation pass was
  added.
- **Verification and build budget:** focused newspaper tests retain query
  admission, exact focus proof, agenda commit, selected-record alignment, and
  audit grounding. V15 gates prove adversarial assignments bind two distinct
  named parties to selected exact records and that period navigation preserves
  canonical ledger order. V14 gates prove the assignment editor sees completed-change
  headlines across distinct periods before querying while stable record IDs and
  fact bodies remain absent, and that rendered audit citations expose source
  time. Existing V13 gates prove the stable selector instruction prefix with
  old record bodies absent from later snapshots, the compact inspected-record
  index, bounded exact-ID refetch, stable selector and Night schemas,
  assignment-aware reporter context isolation, one complete
  copy report, exact single-occurrence query targets, multiple categorical
  objections to the same phrase, one Night Editor action over only the queried
  articles and facts, complete query disposition, preservation of
  assignment-owned structure and citations, no post-close copy or editorial
  stage, deterministic source/printed digests, exact receipt ancestry,
  close-checkpoint resume without reporter or copy replay, idempotent persisted
  composition validation, printing the checkpointed page after a structurally
  rejected close, a fresh v10 close in a store containing a matching inert
  v7 tombstone. The close validator admits only `InitialCopyDesk` checkpoints
  with no source ancestry, while exact-current resume and idempotence regressions
  exercise the close resume path. Dedicated regressions prove one generic
  reporter schema with no per-assignment dateline enum, exact
  fact-derived `allowed_datelines` in each dynamic reporter snapshot,
  deterministic cited-place admission, plus agenda-and-filing checkpoint recovery at
  the first failed reporter without replaying accepted production. Strategic recomposition v3 binds the
  copy report and press witness by digest while v1/v2 remain historical
  validation inputs. Independent blind editorial and grounding reviews evaluate the
  resulting artifact for the acceptance experiment only; they are deliberately
  outside runtime publication authority. Verification remains bounded to the
  existing newspaper library and strategic-smoke targets plus formatting,
  state, and system-map gates. The cut adds no service, crate, dependency,
  event authority, or simulation pass.

Run 60 is the negative cache witness for the superseded framing: 15 calls used
96,902 prompt tokens and reported zero cached tokens. The selector's first call
used 1,452 prompt tokens, while later calls grew to roughly 7,500–11,800. That
first request was large enough for automatic prompt caching, but the old
two-message shape retained only the charter and replaced the step-one message,
so later requests did not preserve the complete first-request prefix. The
focused harness regression now proves one combined charter-plus-step-one user
message, exact later prefix preservation, a replaced dynamic workbench message,
no replayed assistant chatter, and an unchanged output schema. It proves request
shape only; provider cache hits are non-required runtime telemetry.

Run 62 is preserved as an immutable failed v12 witness: the selector reported a
cache hit, reporter calls reported zero cached tokens, and a reporter supplied
an invalid dateline that deterministic admission rejected. It is not evidence
for the v13 reporter prefix. Run 63 is a stopped v13 observation, not an
acceptance artifact: Issues 17 and 18 completed before the final edition, and
repeated reporter calls still reported zero cached tokens. Its partial artifacts
remain inert provenance; no fresh reporter-cache proof is required.

Run 61 remains accepted evidence for the durable newsroom claims: recurring
journalist voices and evidence-shaped special/general issue design. Those
editorial boundaries do not depend on provider cache retention.

The bounded selector snapshot retains a pending agenda's exact token while its
front-page proof is the latest observation. Any later query or exact-ID fetch
retires that candidate; the assignment editor must propose the page again and
receive a fresh proof before commit. Retrieval cannot leave a proofless agenda
eligible for delayed admission.

Run 54 is immutable mechanically complete evidence at exact source
`01581b8576774f370884e331558605e7ef5e1b9b`, implementation
`2ac74abf12612b2c38b04acd6fcb604acd1c9d28`, and world revision 27.
Its newspaper passed independent grounding and failed independent blind
editorial review because administrative conditions, filings, cordons, and
recordkeeping buried conflict and bodily evidence. The archive is
`F:\GameCult\GhostlightDungeon\acceptance\elven-realms-autonomous-01581b8-54.tar`
with SHA-256
`835F72FCB19C38815C543DC1497977288F2F1442F22FDBD32D1B312490B34808`.
No next run may replay world mechanics or per-wave publications.

Run 54 also exposed a separate upstream publicity defect. A public strategic
activity currently propagates its channel to every materialized outcome,
including a private `MemberMemory` consequence; that is how a public news row
about Orin Pell retaining a memory entered the frozen ledger. The newsroom must
not turn that bookkeeping into a story, but it cannot repair world-event
authority. The future kernel cut is: parent activity publicity does not imply
that every internal consequence is public; an outcome needs its own observable
or communicated public scope before `NewsIssue` materialization. Revision 27
remains immutable evidence and is not rewritten for this newsroom-only
acceptance.

The typed issue retains exact event IDs, channels, reliability, source
revision, and model receipts as audit data. Its internal issue time is derived
from the latest cited publication record without exposing that clock to
editorial inference. Rejected action attempts semantically rebind their
otherwise immutable model receipts to collision-free invalid dispositions; a
structurally rejected deadline action prints the already checkpointed source
page instead of creating another editorial turn. The top-level strategic plan,
receipt hash, and commit always project the final completed wave, matching the
top-level counts and persisted campaign head; per-wave history remains under
`waves`. The reader
renderer has no path to provenance fields and escapes all model- and
consumer-supplied plain text before Markdown emission; the provenance renderer
applies the same plain-text boundary to its audit data. Neither assignment
editor, writer, copy desk, Night Editor, renderer, nor receipt persistence can
write world state.

#### Seeded TeX press projection

- **Owner:** the operator-designated frozen newsroom Markdown owns masthead,
  edition label, article identities, lead designation, sections, headlines,
  decks, bylines, and every paragraph. `tools/typeset_newspaper.py` owns
  presentation only through two independent decisions. `style_seed` owns the
  durable house body: sheet geometry, type, stock, ink, rules, gutters, and
  ornaments. `flow_seed` owns the per-issue display template, secondary-story
  grouping, which eligible secondary closes page one, cut-first versus
  cut-midstory placement, cut scale, and page closure. Both apply the pre-WWI
  mass-circulation grammar distilled in
  `notes/historical-newspaper-layout-grammar.md`; neither owns copy. Woodcut
  bitmaps and their adjacent prompt sidecars own illustration provenance, not
  facts. A future economic layer owns advertiser identity, goods, prices,
  claims, campaign continuity, and payment; the press may only place an already
  admitted ad module in the shared page grid.
- **Inputs:** one frozen Markdown edition, required independent style and flow
  seeds, zero or more article-indexed wordless woodcut PNGs,
  output/TeX/manifest paths, and an optional LuaLaTeX engine or `--no-compile`.
  The current Run 61 projection uses style seed 1847 and flow seed 723 and binds
  `boundary-rail-exchange` to article 0 and
  `sinkroot-gauge-gallery` to article 1. Their matching prompt files preserve
  the exact fresh ImageGen requests and record grayscale, 72-percent threshold,
  and trim preparation. No ad module is currently admitted.
- **Outputs:** escaped TeX, a `ghostlight.seeded_tex_press.v2` TOML manifest, and
  optionally a compiled PDF. Manifest v2 records independent `[style]` and
  `[flow]` projections and binds both seeds, source/output/TeX paths,
  article-indexed woodcuts, and SHA-256 digests for source, TeX, tool, woodcuts,
  engine, and compiled output. The current artifact is
  `output/pdf/canopy-ledger-run-61-style-1847-flow-723.pdf`, bound by
  `output/tex/canopy-ledger-run-61-style-1847-flow-723.manifest.toml` with output
  SHA-256 `a457f4b1702a7a52480e42fce0039f0dbe677ff32958e98f1c412b346237190c`.
  These are press artifacts, not another newsroom composition or correction
  surface.
- **Derived state:** the parsed `Edition`, house `Style`, per-issue `Flow`,
  two-page story grouping, continuous four-column reading flow, unbreakable
  story-header modules, story-local cut placement and widths, TeX markup, and
  PDF pagination are presentation projections. Flow templates include
  `display-plate` and `display-band`; inside-story placement includes
  `cut-first` and `cut-midstory`. TikZ/PDF multiply blending makes monochrome cut
  paper disappear into the seeded stock. The same style seed preserves the
  bounded house body across issues while a different flow seed may change
  grouping and placement without changing that body. Exact portable PDF
  reproduction additionally depends on the named system fonts; source,
  woodcut, engine, tool, TeX, and output bytes are bound by manifest v2.
- **Forbidden writers:** the press, TeX compiler, manifest, woodcuts, and prompt
  sidecars cannot add, remove, summarize, reorder, correct, or reinterpret
  newsroom copy; change bylines, lead designation, or newsroom-owned narrative
  function; write newsroom checkpoints
  or world state; promote an illustration into evidence; fabricate an
  advertiser, offer, price, claim, payment, or campaign; or fill an unadmitted
  ad slot. Flow may choose which eligible secondary closes page one, but it
  cannot rewrite article text, change the lead, reassign an indexed cut, detach
  that cut from its story, or omit it. A successful PDF compile cannot admit
  missing or altered copy.
- **Shared path:** every press run parses the frozen source, derives one style
  from `style_seed` and one issue flow from `flow_seed`, verifies that all frozen
  fields survive, writes TeX, compiles that exact TeX when requested, and then
  writes manifest v2. Each woodcut is keyed to its owning article index and
  remains attached to that story's display field in either cut-first or
  cut-midstory form; accepted copy then flows continuously through the common
  four-column grid. There is no detached image bank or bottom-art reservation.
  Any future admitted ad module must use this same grid without changing
  article-internal copy order or flow-selected grouping.
- **Cut line:** the live press path contains no model rewrite, editorial
  summarizer, copy-fitting loop, or second content owner. Press variation is
  bounded to page geometry, typography, color, rules, fleurons, and illustration
  sizing/placement. The former fixed ReportLab renderer, three-column lead plus
  one-column rail, paired story-minipage layout, and detached image bank are not
  live. Minipages now protect headers only; story bodies remain in continuous
  column flow. The single-seed manifest v1 and prior
  `output/pdf/canopy-ledger-run-61-seed-1847.pdf` artifact are not current press
  authority.
- **Verification layer:** admission requires source-to-TeX/PDF field completeness
  and article-internal order, exact same-pair TeX derivation, independent style-
  and flow-seed variation checks, successful halt-on-error compilation, rendered-page
  inspection for clipping and overlap, one attached rendering for every indexed
  cut, and matching manifest/woodcut prompt provenance. The current manifest v2
  records `display-plate`, `cut-first`, and page-one secondary article 2. Its
  final TeX uses four-column `multicols`, article-indexed cuts, `needspace` plus
  header-only minipages, and TikZ multiply scopes. LuaLaTeX produces a two-page
  PDF with embedded Times New Roman faces; `SOURCE_DATE_EPOCH=0` and
  `FORCE_SOURCE_DATE=1` make the same admitted inputs byte-reproducible under the
  bound engine. Extracted copy includes the closing `Marshal Eryn Tal`
  paragraph. Rendered inspection confirms continuous reading order, attached
  cuts blended into the stock, unstranded headers, and no clipping or overlap.

#### Acceptance-driver per-wave newspaper recovery

- **Owner:** the strategic acceptance driver owns recovery of a missing
  consumer artifact after its world wave has already committed. It owns neither
  the wave nor the newspaper's factual or editorial judgment. For an already
  accepted historical recomposition, the original immutable recomposition
  receipt owns the typed issue value and reader/audit bytes; the current driver only
  validates and projects that artifact.
- **Inputs:** the missing wave's immutable checkpoint and committed `Campaign`
  snapshot, the preceding wave checkpoint's committed news-ledger boundary,
  the configured title and voice, the explicit resume recovery boundary
  (`GHOSTLIGHT_STRATEGIC_NEWSPAPER_RECOVERY_START_WAVE`, defaulting to wave one
  and validated within `1..=wave_count + 1`; selecting `wave_count + 1` is the
  final-only sentinel: it recovers no per-wave editions but leaves final combined
  composition enabled), and the same campaign receipt store
  used by ordinary newspaper composition. When an accepted recomposition
  already exists, its immutable receipt and exact named reader/audit files are
  additional inputs.
- **Output:** acceptance writes one separately named immutable recomposition
  checkpoint containing the issue, copy report, press-close witness, and
  newspaper model receipts, plus the rendered reader and audit files. Its
  receipt binds the
  narrowed source campaign, recovery boundary, title/voice/article-budget
  contract, issue, model-receipt collection, expected filenames, and both
  rendered byte streams by digest. Current recomposition v3 receipts bind the
  complete typed copy report and press-close witness by digest. The configured
  boundary is also
  projected through run status and success or failure result artifacts. The
  driver fills only the missing newspaper fields in its in-memory result
  projection. A pre-close provider or integrity failure stops that invocation;
  canonical close state remains in `CampaignStore`. Existing successful issues
  and original wave checkpoints remain unchanged.
- **Derived state:** the newsroom receives a clone of the completed wave's
  campaign whose news ledger is narrowed to rows appended since the preceding
  committed boundary. All events and canonical names remain available only so
  those exact news rows can resolve their sources; the clone is a consumer view,
  not a campaign mutation. Before that handoff, the driver validates that wave
  reports form a contiguous prefix and selects only reports at or after the
  configured recovery boundary whose issue is absent. Earlier missing reports
  remain historical gaps, while any already successful issue inside the selected
  range is left untouched. The configured recovery boundary owns selection of
  the missing reports in the current invocation; the final-only sentinel derives
  an empty per-wave selection because no report can reach `wave_count + 1`. A
  completed recomposition's
  recorded recovery boundary is immutable provenance for the invocation that
  produced it; it does not become a second selector or invalidate the accepted
  issue when a later invocation selects a narrower range.
- **Forbidden writers:** recovery cannot rerun a strategic tick, Nemesis,
  simulation cells, outcome resolution, clocks, or `WorldKernel`, and it cannot
  alter campaign, event, or news state. Failed recomposition stops consumer
  recovery and cannot repair mechanics.
- **Shared paths:** live per-wave composition, recovery composition, and final
  combined composition all call the thin `compose_persisted_newspaper` delegate
  into `advance_world_newspaper`, including the same assignment, writer, single
  copy report, one deadline close, checkpoint-resume, and receipt-persistence
  owner. The final combined composition still runs when the recovery selector
  returns no per-wave reports. A later invocation resumes a close checkpoint without rerunning the
  writer or copy desk. Once printed, it
  consumes the successful recomposition artifact instead of paying for the
  newspaper again. The driver recomputes its wave, source-campaign, and
  editorial-contract bindings, validates the recorded original recovery
  boundary, and checks the exact issue value, typed model-receipt set, reader
  bytes, and audit bytes against their immutable digests. A current-schema issue
  is deserialized and rendered again. Current recomposition v3 also deserializes
  the copy report and press-close witness and requires their exact digests. A
  historical v2 accepted issue
  is structurally deserialized only as v2 and is not reinterpreted through the
  current issue type or renderer; its original receipt owns those historical
  bytes, and the driver carries its JSON only as an inert result projection.
  Historical issue v2 predates `grounding_digest`, so this local validator requires
  accepted status but does not claim a retroactive digest for its complete
  grounding verdict. Whole-checkpoint integrity instead remains anchored by the
  unchanged Run 51 source-root/archive hash and Run 52 before/after artifact
  manifest.
- **Cut line:** current `WorldNewspaperIssue` deserialization and current
  renderer output are no longer authorities over a historical accepted issue.
  Raw digest verification is not a migration path and cannot create, repair, or
  reaccept a paper: all applicable original source, contract, filename, issue,
  receipt-set, and byte bindings must already be present and exact. Current
  recomposition v3 requires exact copy-report and press-close digests;
  historical recomposition v1/v2 retains its bounded grounding/editorial
  validation, and historical issue v2 keeps the explicitly bounded
  accepted-status plus archive/manifest proof described above.
  Strategic state and per-wave artifacts remain forbidden writers.
- **Verification layer:** the focused boundary regression proves that a second
  wave is recomposed from only its newly appended news row while retaining the
  committed event ledger needed to resolve that row. It also rejects prior-news
  prefix drift and confirms that first-wave recovery fails closed without an
  exact pre-run boundary. Immutable checkpoint publication separately rejects
  overwrite and publishes by synchronized temporary-file rename. A typed
  receipt-digest regression proves that checkpoint JSON is deserialized back to
  `Vec<ModelStageReceipt>` before hashing and rejects an invalid receipt shape,
  so creation and verification share one representation. A scope-selection
  regression proves that earlier missing issues and successful issues are
  skipped while later missing issues are selected, and that the
  `wave_count + 1` boundary selects no per-wave reports for a final-only resume.
  Dedicated regressions prove
  that historical v2 preserves its recorded original recovery boundary and
  accepts exact historical bytes without current rendering while rejecting byte
  drift, that current issue v3 still requires typed deserialization and exact
  rendering, and that current recomposition v3 rejects copy-report or press-close
  witness drift. Historical recomposition v1/v2 validation remains bounded to
  its original verdict shapes.

The first-wave limit is now an explicit closed boundary rather than inferred
recovery: when wave one is selected, the driver refuses to compose without a
preserved pre-run news prefix. A later explicit boundary can instead define an
acceptance continuation whose older missing issues are intentionally out of
scope. The remaining limitation is verification breadth: no focused test yet
executes the complete resume orchestration from fresh recomposition through
later reuse while asserting successful-issue preservation, exact rendered-file
validation, and absence of mechanics calls.

Session Zero compilation, expansion, and fission follow the same projection
rule. Review projections expose topology, pressures, source-use coverage, gaps,
assumptions, public party cards, and only the current viewer's private ledger.
Generated openings are optional shared typed decisions; accepting one amends
the draft for discussion and triggers three optional grounded role decisions in
each private DM channel. Ignoring them cannot block a custom start or character.
At registry load, exact legacy `opening:*` suggestions and payloadless question
records from the pre-cut runtime are demoted from material to optional with a
CultCache compare-and-swap; fictional revision and shared epoch do not move.
Payloadless prose cannot own compilation readiness. The persisted typed draft,
not stale suggestion materiality, owns that gate.
Custom starts and characters use the same draft. The browser returns
only decision commands and cannot echo a rewritten candidate into compilation.
Candidate IDs, text, list sizes, and evidence references receive local
validation. Suggestion retrieval and every committed model invocation remain
in the Session Zero audit, including repeated calls with the same semantic
receipt hash but different attempt telemetry. A separate active-preview receipt
set is replaced by each successful compiler transaction and cleared when that
preview is retired. Publication carries only that exact proof set; it never
treats DM turns or superseded compiler runs as evidence for the approved seed,
and it never compiles the conversation transcript.

Gap-bearing compiler results are retained as exact non-canonical previews while
Session Zero returns to drafting. The host can move that exact preview into
review; this is not approval. Every member approval now binds the shared
contract digest, that member's private character digest, and the complete world
preview digest. Publication persists that digest plus the exact accepted gap
and branch-assumption lists. Replacing topology, cast, institutions,
populations, clocks, evidence lanes, gaps, assumptions, or any other preview
field therefore makes old approvals unusable. The Eve tree visibly lowers the
reviewable world shape. Player JSON excludes institution resources and goals,
and shared cast projection excludes private relationship-anchor people.
The host can discard any non-canonical preview through the SessionZeroKernel;
the command clears only preview state and returns the locked roster to drafting
without editing the contract, character drafts, or fictional time.

Session Zero owns human-controlled actor identity and state. Public player names
and premises may enter the shared compiler as setting context, but those names
are reserved outside world-cast compilation: actors and Gestalt members cannot
materialize them. The compiler's singular player object is a provisional
starting-position marker only. Ghostlight removes it and installs the approved
typed characters locally. Collision receives one same-snapshot correction and
then rejects the candidate; no deduplication or model choice can replace the
Session Zero owner.

Private character relationships now cross compilation through a typed identity
membrane. Exact player references bind to their approved actor IDs. Each
otherwise unresolved named person gets a stable server-generated
`relationship-anchor:*` actor ID. A separate private actor stage receives only
the approved display name after public topology validation; it never receives
the opaque identity handle or relationship. The shared world compiler receives
none of those private inputs. The private
stage returns characterization and one exact supplied location, but has no
schema fields for IDs, relationships, narration, facts, gaps, assumptions, or
public-world changes. Ghostlight attaches the opaque ID locally and later binds
the approved relationship only onto its owner's actor. Omission, ambiguity,
unknown placement, or collision rejects the candidate atomically. Actor,
institution, Gestalt, and `member:*` IDs are all legal directional relationship
targets, but those links never union knowledge or authority. Only the owning
member's review surface projects the compiled target, placement, and private
description before approval. A shared branch assumption exposes the count of
materialized private subjects, never their identities or relationship details.

Approved private character playability crosses a narrower compiler membrane
than private history. The shared world compiler receives bounded capabilities,
equipment, obligations, goals, and extraordinary-permission scope, costs,
limits, exposure, and effect ceilings only so it can seed precise discoverable
environmental facts for the opening problem. It never receives private
history, secrets, relationships, or pre-existing knowledge. Generic crisis
restatements do not satisfy this requirement; a concrete investigative ability
needs a concrete pre-existing finding or an explicit evidence gap.

Roster lock is not readiness. Before world compilation the kernel checks the
typed contract's core play frame and each active character's minimum actionable
shape. Transcript-only detail, blank contract fields, or a character with no
public premise, capability, goal, and stake return explicit missing inputs and
cannot spend a world-compiler call.

Campaign publication has a separate discoverability boundary. Session Zero
publication and solo lifecycle operations initialize a new `.cc` store under a
non-UUID `.creating-*` directory. The CreateCampaign command validates the
seed, then atomically commits campaign state, approved seed, membership,
contract, DM Persona, approvals, Vault manifest, evidence, model receipts,
canon candidates, and the creation lifecycle receipt in one CultCache batch.
Only that complete store is renamed into the UUID
namespace and admitted to the live registry. Failed initialization removes its
exact staging directory and cannot poison daemon reload. Approval previews are
retained until campaign publication, session selection, and reload all succeed;
a retry may recover only an already-published store whose exact seed equals the
preview. It cannot adopt a colliding campaign ID with different state.
Legacy Session Zero stores that used the preview receipt field as an append-only
audit migrate on registry load. The complete list remains audit history and the
final compiler transaction becomes the active proof set. This replacement is
idempotent and does not advance Session Zero revision, epochs, or fictional
time.
The browser composer preserves the assessment contract's distinction between
the character's described act or speech and the effect the player wants it to
cause. The receipt panel is the sole owner of the current assessment,
destination, or fission approval control; rendering a newer receipt removes the
older snapshot-bound control instead of accumulating stale confirmations in
persistent forms. World-seed approval renders expandable public sections for
the player role, initial topology, present cast, institutions, populations,
world clocks, and exact evidence-use rationales instead of asking for approval
from summary counts. A browser-local interaction lock prevents duplicate paid
requests while preserving the kernel mailbox and revision checks as the only
world-mutation authority.
The player surface also projects the canonical current location ID. The
destination compiler binds that value into a read-only origin field and shows
the proposed locations, containment, routes, persistent features, and material
gaps before approval. Population fission approval similarly shows each named
child leaf, partition value (including `other/unknown`), home location, and the
exact child receiving each parent-owned resource. Shared capabilities,
knowledge, goals, pressures, and relationships inherit through population
lineage; custody does not. Each scarce resource crosses exactly one typed
custody transfer, so a fission cannot copy one granary, vehicle, or medicine lot
into every child.
Fission admission runs before parent lookup, retrieval planning, or inference:
one request may contain at most 16 distinct cuts of at most 160 readable
characters and one 500-character reason. The browser mirrors these bounds.
The browser interaction lock covers a canonical mutation and the subsequent
surface refresh as one user interaction. Campaign selection, world approval,
destination approval, fission approval, fork, and reset cannot re-enable stale
controls between the commit response and the refreshed revision. Compiler
preview requests remain non-mutating and release the lock with their returned
preview. Received non-success HTTP responses are rendered as server refusals;
only missing or undecodable responses receive the ambiguous-transport warning
that a commit may have preceded the lost response. The production web build
runs strict TypeScript checking before Vite emits assets.

## World Transition Authority

`WorldKernel` owns one closed semantic `WorldMutationBatch` reducer. Foreground
attempts, NPC reaction waves, strategic actor/institution/Gestalt outcomes,
waits, group travel, and approval-gated population fission all lower their
accepted outcomes into that vocabulary before canonical state changes.
`ActionMeans` records what an actor did, and the intended effect records what
they hoped to change; neither is a committed fact. The mutation batch contains
only exact component changes admitted by a state-derived authority envelope.

The reducer validates the complete batch on an isolated component overlay,
advances each touched subject version once, and persists component state,
mutation authority envelope, mutation batch, mutation receipt, causal command
receipt, events, and aggregate compatibility projection atomically. The
aggregate campaign row is rejected as a component-only mutation target.
`WorldEffectDelta`, `ActorStateDelta`, and strategic effect variants are model
boundary migration inputs, not writers or independent physical ontologies.
The foreground assessor derives a closed effect schema from the exact snapshot
and first removes structurally unavailable knowledge, movement, clock, and
institution lanes. A compact private scope projection then receives only the
exact attempt and those remaining lane names. Its locally validated output can
only subtract: every causally unselected lane disappears from the assessor's
schema, and unselected knowledge prevents scene facts from entering the prompt
at all. The scope also distinguishes lanes required to realize strong and
ordinary success from lanes merely available for direct consequences. Required
success lanes receive a non-empty structural constraint and a post-lowering
local check, so stakes cannot claim a canonical success that the typed effect
omits. Canonical dynamic effect maps cross the strict model boundary as bounded
entry arrays containing only authority-enumerated IDs and values; validated
entries lower once into `WorldEffectDelta`, while empty arrays mean no mutation.
The admission-bound model schema requires those success entries only for an
admissible assessment and structurally forces every inadmissible effect array
empty; local validation repeats both rules before verification or caching.
Scope rows and receipts are derived inference evidence, never assessment or
world authority, and their content-addressed cache avoids repeated scope
inference for an unchanged attempt.
After structural binding, a compact independent verifier compares the complete
four-band typed effect bundle with the player's exact means and intended
effect. Structurally legal state is not automatically causally relevant. The
verifier can only accept or return one bounded mismatch classification and
repair sentence; it cannot reassess difficulty, choose an effect, or commit.
One mismatch returns to the assessor against the same snapshot, while a second
mismatch aborts. Every verifier receipt is private audit state and only a
verified proposal may enter the assessment cache.
Pending NPC actions are revision-scoped reaction-window state. The next
reaction wave atomically replaces the set, and the kernel resolves an action
only when the current last event is the exact current-revision reaction wave.
Same-wave assessment correction remains possible; cross-turn rebasing is not.
The foreground command owns committed player speech, presence reconciliation,
and the directly addressed Persona response. Once that response is committed,
the request returns without waiting for a second NPC action assessment. The
accepted action proposal remains in `Campaign.pending_world_proposals`; an
idle-boundary continuation runs its assessment with background priority and
calls the same `WorldKernel::ResolveNpcAction` command. A new live turn cancels
in-flight background inference. The proposal may commit only against its exact
reaction revision, and startup plus the strategic scheduler rediscover current
reaction-wave proposals after daemon interruption. The continuation is an
actuator, not another queue or state owner.
The same reaction commit validates its response duty against the exact
committed source turn. A missing directly addressed Persona or a null response
aborts the whole wave without private deltas, transcript additions, initiative,
or revision change; observers may still choose to speak from their own goals.

Action assessment separates semantic inference identity from commit identity.
The exact scope, assessor, and verifier models, stable instructions, scoped
action-specific schema, and permitted typed packet form a content-addressed private
`assessment_proposal_cache.v1` key. Revision and expiry still bind the
consumable assessment and roll but do not force the capable model to reconsider
an otherwise byte-identical packet. A cache hit revalidates the complete
proposal against current state, emits a zero-token receipt linked to the source
scope, assessment, and verifier receipts, and creates a fresh digest at the current
revision. Changed authority changes the key. The Eve command-result projection
shows every signed modifier with its exact references.

Population fission now composes admitted child entities, stable child
identities, one lineage split, exact resource custody transfers, and exact
named-member membership transfers. The same fission transaction owns every
relation and civic consequence of retiring the parent: it creates one inherited
child agency relation per child, retires the parent relation, replaces the
parent resident ID with all child IDs in each affected civic manifest, rebinds
each civic political-relation ID to the exact child relation IDs, advances the
manifest version, and binds its semantic verification/source field to the exact
`fission-preview:<digest>` receipt. These writes lower together with child
admission, lineage, custody, and membership into one component-world batch;
either the complete state validates and commits or none of it does.

The aggregate projector may copy the accepted child relations and civic
manifests out of that component result, and `project_fission_resolution` owns
the derived agency-profile, facet, cover-invalidation, and resolution-epoch
projection. `AgencyProfile.location_ids` remains exact presence or operating
places. It does not inherit a parent's political jurisdiction, because
jurisdiction is containment topology rather than subject presence. Later
`ensure_agency_profiles` synchronization is not a cleanup owner and may not
repair missing civic residents, replace stale political relation IDs, recreate
retired parent relations, or make a partially applied fission appear valid. The
old fission function that directly wrote campaign state has been removed.

Immutable Runs 92 and 93 falsify profile-scope inheritance as the routing
repair. Run 92 contained 938 active, simulation-eligible Gestalts, while the
old direct-profile-membership query found zero realm-routable descendants for
Avelune, Veyra, and Bramblewash. Exact canonical topology already contained the
answer: Avelune localities descend from
`loc:avelune-watershed-realm`, Veyra localities from
`loc:veyra-rail-realm`, and Bramblewash localities from
`loc:bramblewash-grain-river-realm`. Run 93 at exact source `31125a2` failed
before any model call because the now-rejected lineage-scope reconciliation
found nothing to change. Preserve both runs as evidence.

The scheduler now calls the shared `unique_containing_jurisdiction` resolver to
derive complexity jurisdiction from canonical `Location.container_id` ancestry
without writing any agency profile; worker admission reuses the same resolver.
There is no `ReconcileFissionAgencyScopes` command, lineage scope-inheritance
helper, or agency-scope reconciliation checkpoint. A scheduler fallback that
guesses a realm, copies jurisdiction into profile presence, or chooses among
ambiguous realm ancestors is forbidden.

Immutable Run 81 at exact source
`e7ceb2c2807fabb2b0e68aeeeb0aab5948c582a5` and root
`/var/lib/gamecult/ghostlight-dungeon/acceptance/full-world-delvehold-e7ceb2c-81`
is failure evidence for this ownership boundary. Patina commit 0001 advanced
revision 15 to 16 and admitted the Hearthcoil children, but left the Hearthcoil
civic manifest pointing at missing parent relation
`rel:hearthcoil-houses-member-blackchalk` while the child relation clones
existed. Charter's already accepted frozen proposal then failed admission with
`invalid command: canonical civic system lost a political relation`. Preserve
the run unchanged; unit `ghostlight-full-world-e7ceb2c-81.service`, invocation
`069f3a5de05b4bd3a972ddf621e29d54`, is terminal with an exit code.

Run 81 recovery is an explicit one-boundary repair, not rollback or fission
replay. `WorldCommand::ReconcileFissionCivicBindings` owns deterministic repair
on a clone of Run 81. It may select only civic manifests whose stale parent
resident or political-relation binding is proven by an existing canonical
Gestalt lineage and, for each retired relation, the complete canonical child
relation set. It lowers only the resulting `SetCivicSystem` mutations through
the shared mutation kernel, increments the affected manifest version, and binds
the repair to a deterministic `fission-civic-reconciliation:<digest>` source.

The repair uses an unchecked component snapshot only as the bounded workbench
needed to represent the known invalid starting state. The mutation application
must produce a normally validated next component world; WorldKernel then
persists an ordinary world commit and mutation receipt under command kind
`reconcile_fission_civic_bindings`. The acceptance driver publishes one
immutable `ghostlight.fission_civic_reconciliation_checkpoint.v1` and refuses a
checkpoint whose canonical repair is absent or whose command/revision binding
disagrees with the campaign. The unchecked snapshot is not a general admission
mode, migration authority, cleanup loop, or permission to repair unrelated
state.

After that checkpoint, the complexity loop recognizes Patina commit 0001 as
already admitted, skips it, and rebases the preserved Charter proposal against
the repaired revision. The Run 81 frozen preview and model receipts remain the
inference authority; recovery performs no model or elaboration replay.

Region expansion and in-place locality elaboration compose typed place,
proposition, population, and institution admission plus exact topology and
political-relation changes against one authority snapshot. Locality elaboration
anchors every added place beneath one existing canonical jurisdiction and
forbids the compiler from duplicating that target. Civic manifests cross-bind
governing institutions, resident populations, public civic facts, and local
political relations before lowering. `CivicSystemSet` is the versioned component
owner; aggregate `Campaign.civic_systems` is reconstructed from its accepted
state. The mutation reducer owns those canonical component changes; the
compiler-validated agency-profile map is still a companion projection and must
not be described as component-owned until its migration is complete. Approval
commits the complete addition atomically or leaves the coarse place untouched. Repeated
Vault retrievals have distinct retrieval receipt identities, while an exact
immutable receipt replay is admitted idempotently under the same CultCache CAS.

Every new destination carries an explicit outbound route from the stable origin
and a reciprocal return route with the same positive travel time. Place
profiles, proposition content, evidence references, discovery locations,
containment, route identity, distance, and topology are validated on the
component overlay; aggregate rows are reconstructed only from accepted
component state. A route map key is local to its exact origin. The component
overlay derives its edge identity from `(origin_location_id, local_route_id)`;
the player surface uses the route's exact destination field, and accepted
region expansion restores the original local key. Reusing `road`, `harbor`, or
a destination-shaped key under another location cannot overwrite an unrelated
edge or make a surface-advertised route fail transition admission.

Governance approval and governed finalization are distinct durable states.
When unanimous time, travel, or Persona-cell approval exists but finalization
did not commit, the advertised approval operation retries the exact revision-
bound finalization even for a member whose approval is already present. This
does not add another writer: the same WorldKernel validation and atomic commit
path runs again. Duplicate non-unanimous votes, stale boundaries, and committed
proposals still reject.

Initial compiler publication is a bounded creation transaction, not a fictional
world transition. `CampaignRegistry` owns discoverability, installs the
approved seed into a fresh staging store with one empty-store CAS, and exposes
it only by atomic directory rename. It cannot target a published campaign.
Named-person materialisation and dematerialisation are resolution transactions
over one persistent member identity. They advance world revision to stale old
actor-bound commands, advance `resolution_epoch`, clear the derived cover, and
atomically preserve the exact individual delta. They cannot change fictional
time or Gestalt-wide knowledge, resources, or pressure; the obsolete
`GestaltAggregateDelta` writer has been deleted. See
`docs/architecture/ghostlight-transition-algebra.md`.

Relationship documents in the schema catalog are revision-bound projections
of actor-owned relationship maps; they are not a second relationship writer.
Vault manifests summarize the exact provider/source/authority/temporal lanes
covered by evidence receipts and do not own Vault content. Strategic tick and
gestalt materialization receipts are different: they are atomic commit
companions binding the generic world commit to the causal model output or
baseline/member presence transition. Materialisation receipts additionally bind
the previous and next resolution epochs.

## Control Flow

1. Rehydrate from `state/map.yaml`, `notes/fresh-workspace-handoff.md`, this
   file, and `notes/ghostlight-implementation-plan.md`.
2. Run `npm run state:status`.
3. Work one bounded organ.
4. Validate the seam that matters.
5. Persist only belief-changing evidence.
6. Commit and push completed work.

Model acceptance also measures useful work per token. Stable contracts precede
revision-bound context for cache reuse; each stage receives only the state it
can legitimately judge; provider receipts expose cache, prompt, completion, and
validation cost per attempt.

## Core Surfaces

- `state/map.yaml`: canonical mission, boundaries, live architecture, next action.
- `state/evidence.jsonl`: distilled belief-changing evidence only.
- `state/evidence.archive.jsonl`: older evidence preserved for archaeology.
- `state/corpus-coverage.json`: accepted/planned fixture coverage ledger.
- `notes/fresh-workspace-handoff.md`: compact re-entry packet.
- `notes/ghostlight-implementation-plan.md`: near-term implementation sequence.
- `docs/architecture/`: detailed contracts and rationale.

## Active Artifact Pipeline

```text
Aetheria lore/source context
  -> lore grounding digest / source notes
  -> canonical agent and scene state
  -> projected local context
  -> responder packet
  -> sandboxed responder output
  -> review and leakage audit
  -> observable event record
  -> participant-local appraisal receipts
  -> mutation receipt
  -> updated scene/world/social state
  -> initiative schedule updates readiness, reaction windows, and next actor
  -> sandboxed coordinator continuity and next-beat plan
  -> meta-coordinator review, wiring, and labeled interventions
  -> branch compiler materializes Ink + sidecar + visual plan + compiler notes
  -> IF artifact reviewer audits consequence, fold, and visual continuity
  -> narrative/lore/spatial/visual reviewers audit player-facing quality
  -> accepted fixture / future training corpus + optional illustrated replay collateral
```

Responder packets are one seam, not the whole project. The branching scene path
also requires coordinator receipts, branch compiler artifacts, and independent
review before a fixture is accepted.

Tone is a live seam. The system should preserve Aetheria's range instead of
defaulting to dry technical crisis prose: comic warmth, domestic routine,
ritual memory, weird bureaucracy, noir suspicion, wonder, horror, poetic dread,
and procedural systems pressure are all valid when source-grounded and
character-local. Adams/Pratchett-style wit-with-stakes is a useful default
touchstone for counterbalancing bleakness without dissolving consequence.

Visual replay is a presentation seam, not a core social-model training seam.
Illustrated IF fixtures need click-through sections with stable
`visual_scene_id` anchors, stable visual character refs for recurring named
characters, a global style cue when the visual language matters, imagegen-ready
base prompts, character visibility and stance controls, and branch/state
modifiers. This data belongs in a separate `.visual.json` artifact referenced
by the training sidecar, not inside the training annotation itself.

Generated images are not geometry authority. They can establish visual mood,
materials, and concept direction, but multi-angle illustrated IF needs a durable
`scene_set` source such as a hand-built 3D blockout, procedural scene file, or
layout asset with named cameras and staging slots. For those scenes, imagegen is
the painter/render pass over a camera-specific blockout, not the architect of
the room.

## Important Contracts

- Agent state: `schemas/agent-state.schema.json`
- Projection examples: `schemas/projection-example.schema.json`
- Lore grounding: `schemas/lore-grounding-digest.schema.json`
- Projected local context: `schemas/projected-local-context.schema.json`
- Coordinator artifacts: `schemas/coordinator-artifact.schema.json`
- Responder packets: `schemas/responder-packet.schema.json`
- Responder outputs: `schemas/responder-output.schema.json`
- Event records: `schemas/event-record.schema.json`
- Participant appraisals: `schemas/participant-appraisal.schema.json`
- Reviewed mutations: `schemas/reviewed-mutation.schema.json`
- Scene-loop bundles: `schemas/scene-loop-bundle.schema.json`
- Initiative schedules: `schemas/initiative-schedule.schema.json`
- Ink branch contract: `docs/architecture/ink-branching-scenes.md`
- Illustrated IF visual pipeline: `docs/architecture/illustrated-if-visual-pipeline.md`
- Training stages and corpus gates: `docs/architecture/training-plan.md`
- Corpus coverage ledger: `docs/architecture/corpus-coverage-ledger.md`

## Current Live Examples

Pallas Species Strikes is the active reference-only story fixture:

- `examples/lore-grounding/pallas-species-strikes.awakened-labor.v0.json`
- `examples/agent-state.pallas-species-strikes.v0.json`
- `examples/coordinator/pallas-species-strikes.branch-and-fold.v0.json`
- `examples/ink/pallas-species-strikes.branch-and-fold.v0.ink`
- `examples/ink/pallas-species-strikes.branch-and-fold.v0.training.json`
- `examples/visual/pallas-species-strikes.branch-and-fold.v0.visual.json`
- `experiments/pallas-species-strikes/pallas-species-strikes.branch-and-fold-clean-run.v0.md`
- `experiments/pallas-species-strikes/pallas-species-strikes.visual-scene-review.v1.json`

The fixture is not training-ready data for Ghostlight's model stages. It was
coordinator-authored without the exact projected-context, sandboxed responder,
appraisal, mutation, relationship-update, and true branch-compiler receipts
those stages require. Keep it as a reference for story shape, grounding,
branch-and-fold presentation, visual planning, and future fixture expectations.

`pallas-training-loop-v0` is the first separate training-shaped derivative. It
rebuilds only the Kappa threshold beat through actual receipts:

- scene digest and initial state under `examples/training-loops/pallas-training-loop-v0/`
- projected contexts under `examples/projected-contexts/`
- responder packets under `examples/responder-packets/`
- raw no-fork subagent captures under `experiments/responder-packets/`
- event, appraisal, mutation, review, and bundle receipts under `examples/training-loops/pallas-training-loop-v0/`
- clean readable transcript under `experiments/pallas-training-loop-v0/`

It is training-shaped for projector, responder, event resolver, participant
appraiser, mutator, relationship/perception updater, and coordinator/story
runtime. It is not branch compiler, IF reviewer, visual, or accepted full-fixture
coverage data.

`lucent-hostage-feed-v0` is the current open-ended training-loop draft. It
tests the same receipt-chain organs in a less reference-bound scene: a Lucent
Media hostage-feed negotiation inside the media-eye counterweight of a tethered
station staring down at a metropolis bubble.

The Lucent bundle includes:

- scene digest, initial state, and branch surface under `examples/training-loops/lucent-hostage-feed-v0/`
- projected contexts under `examples/projected-contexts/`
- responder packets under `examples/responder-packets/`
- raw no-fork subagent captures under `experiments/responder-packets/`
- event, appraisal, mutation, review, and bundle receipts under `examples/training-loops/lucent-hostage-feed-v0/`
- clean readable transcript under `experiments/lucent-hostage-feed-v0/`
- fixture materializer at `scripts/materialize_lucent_loop.py`
- initiative schedule example at
  `examples/initiative/lucent-hostage-feed-v0.turn-20.initiative.json`

Only the fifteen post-restart turns from `turn-06` through `turn-20` are counted
as training-clean responder receipts. The earlier exploratory opening turns are
setup/rehearsal context. The loop demonstrates state carryover: feed evidence
externalization creates breathing room, breathing room creates a bench-edge
substitution, the substitution enables a process-note admission, the admission
enables direct hostage leverage to end, and release folds into safe-line
confirmation plus non-contact protective handoff.

The current coordinator lesson is explicit: nudge through game-ergonomic state
levers rather than forcing plot. Safe lines, evidence locks, security holds,
handling categories, posture constraints, route access, clocks, public proof
objects, and resource pressure are the elastic strings. Actors pull against
those strings from local state; the coordinator records observed action and
folds consequences through reviewed mutation.

The coordinator must now be treated as a sandboxable organ in future
training-shaped loops. Codex remains the meta-coordinator: it prepares visible
input, launches the coordinator worker, reviews the output, labels repairs, and
wires structured state between organs. Raw coordinator training data should come
from sandboxed coordinator prompts, not from omniscient chat steering.

The initiative scheduler is now an explicit mechanical seam. It decides when an
actor is eligible to act from initiative speed, recovery, load, status, and
reaction windows; it does not decide what the actor wants or how other
participants appraise the event. Every affected participant still appraises and
mutates before the scheduler selects the next projected actor.

VoidBot's newer heartbeat routine is the reference shape for turning that seam
into a living swarm routine. For Ghostlight, the swarm is the cast: the same
heartbeat initiative law selects characters taking turns within a scene, while
offstage or maintenance heartbeats handle rumination, sleep consolidation,
memory resonance, and reviewed upkeep. See
`docs/architecture/voidbot-routine-adoption-plan.md`.

## Pruned Receipts

Old prototype fixture receipts and model-runner files have been removed
from the live workspace. Their useful lessons belong in the architecture docs,
state map, evidence ledger, and git history. Do not reintroduce deleted receipt
paths as active state surfaces.

## Missing Or Incomplete Organs

- Contract-governed split parties, private in-play actions, delegation,
  simultaneous declaration windows, PvP, late joining, permanent departure,
  and group fork/reset/export
- Deterministic speaker-local input slicer
- Full prompt renderer
- Classifier/appraiser pipeline
- Relationship/perception updater
- State mutator model
- Student projector
- Runtime integration for initiative scheduling beyond validated artifacts
- Branch compiler implementation beyond coordinator-authored fixtures
- IF artifact reviewer implementation beyond manual/frontier review
- Visual scene continuity reviewer implementation beyond prompted specialist review
- Full scene/event loop implementation
- Culture prior engine
- Automatic promotion of branch outcomes into canonical state
- Corpus coverage ledger expansion beyond the current draft Pallas and Lucent
  receipt-chain rows

## Current North Star

Generate high-quality, sandboxed, source-grounded branching-scene samples that
can later train specialized Ghostlight models while staying usable by a game
engine: exact inputs, exact outputs, provenance, review labels, state mutation
receipts, branch compiler artifacts, IF review findings, visual scene plans, and
clear branch consequences. The samples should also train tonal control: a
fixture needs an intentional prose mode and enough ordinary life for consequence
to matter.

Use `npm run coverage:status` before choosing new fixtures so the corpus grows
by coverage need instead of proximity to the nearest interesting disaster.
