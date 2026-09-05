# Ghostlight authority architecture

## Status

This document is the adopted rebuild target for the Ghostlight runtime. It is
the authority map for implementation work. The pre-rebuild body remains
described in `notes/ghostlight-current-system-map.md` only as teardown evidence.

Ghostlight is a persistent, source-grounded narrative world. Its job is to
preserve people, institutions, places, knowledge, material consequence, and
autonomous motion across time. It is not a pipeline for passing prose among a
committee of models until one model permits another model's text to exist.

## Objective

Build one sparse, open-world causal machine that supports solo and bounded
co-op play, autonomous subject decisions, source-grounded expansion, external
world owners, and actor-private experience.

The world grows when committed pressure reaches a typed boundary: a new
destination becomes relevant, an existing polity or population enters causal
range, a person must be individuated, or an effect needs a missing primitive.
A world also grows toward an authored `WorldScaleIntent`: how many goal-bearing
subjects it should hold at each level and realm. Elaborators work that deficit
down; structural validity never waits on it. Cover budget is the deliberate
choke: with subjects several times the cell budget, the scheduler must choose
where attention goes, and actors pursuing their own goals at every level is
what makes the world feel alive. Neither number gates activation or admits a
mutation.

## Prime invariants

1. One per-world `WorldKernel` owns the complete revisioned `WorldState`, from
   draft through active play and archive.
2. Every accepted change crosses one `CommandEnvelope`, one deterministic
   authority derivation, one ontology reducer, and one atomic CultCache CAS.
3. Structural identity, reference integrity, containment, topology, custody,
   jurisdiction, knowledge scope, lineage, and external ownership are typed
   code invariants.
4. Every autonomous authority scope names one decision controller. A
   `NarrativePersona` uses the Projector-Persona-Interpreter prose membrane; an
   `OperationalAgent` receives typed state and tools directly. Neither can
   validate, reconcile, schedule, repair, or commit another model's proposal.
5. Projection, attention plans, transcripts, news, summaries, dashboards,
   checkpoints, and model receipts do not own world truth.
6. Qualitative interest, political diversity, prose quality, name repetition,
   and actor counts are evaluation evidence. They are never mutation gates.
7. A failed or interrupted inference leaves no half-owned world state. Recovery (infrastructure interruption of an inference; a world-scope interruption between intent and commit is plan step 9 and is narrated, not discarded).
   starts from the last committed revision and derives pending work again.

## Canonical owner

### Owner

One mailbox-backed `WorldKernel` owns a world. The mailbox serializes commands;
the CultCache compare-and-swap protects the same invariant across restart or
multiple process attempts.

`WorldKernel` owns:

- lifecycle phase and revision;
- membership, contract, boundaries, and approvals;
- the typed world ontology;
- committed events and fictional time;
- fictional time, commitments, and pressures;
- command idempotency and the commit digest chain.

There is no separate Session Zero owner, component-world owner, aggregate
campaign owner, elaboration committer, scheduler writer, or recovery writer.

### Inputs

The only mutation input is a `CommandEnvelope` containing:

- a stable command ID;
- the exact world ID and expected revision;
- an authenticated principal or internal system capability;
- one closed command body;
- exact evidence references where the command depends on source material.

The caller supplies identity evidence, not an authority verdict. For a patch
command the kernel derives the caller's authority itself:
`require_patch_author` reads the caller and committed state to decide who may
submit `AdmitPatch` and what ground it holds (the world owner unconfined, an
elaborator confined to a jurisdiction, or a consumer confined to the subjects
it controls), and `confine_to_ground` checks every declaration and operation
in the candidate patch against that ground before it is admitted. Callers
cannot grant themselves scope by serializing a persuasive envelope; the
kernel derives it from state it already committed.

Interpreters and operational agents submit the same typed commands through
narrower schemas. Narrative Personas never see those schemas or tools. No
controller receives a privileged mutation path.

### Outputs

An accepted command produces one `WorldCommit` containing:

- previous and resulting revision;
- previous and resulting state digest;
- command ID and derived authority digest;
- the exact admitted mutation batch;
- committed events;
- evidence references;
- commit time and previous commit digest.

The next `WorldState` and `WorldCommit` are persisted atomically. A rejection
returns typed invariant failures and changes nothing.

### Derived state

The following are recalculated from committed state and may be discarded:

- subject-local projections;
- due-owner queues and attention plans;
- population covers, cells, cell ids, resolution, tick index, the agency
  graph, and the cover budget (the only durable trace of a tick is the
  controller-work row, custody-separate from world custody);
- action-owner counts and causal-capacity summaries;
- transcript, news, narration, and operator summaries;
- Eve/CultUI and CultMesh documents;
- model prompts, outputs, receipts, latency, and provider telemetry;
- qualitative evaluations and load-test reports.

Derived data may guide a later proposal. It may not admit a mutation, repair a
commit, or declare the world complete.

### Forbidden writers

The following cannot decide canonical state:

- HTTP, native, Eve, CultMesh, or chat handlers;
- Projectors, Personas, Interpreters, assessors, verifiers, reconcilers,
  narrators, and copy desks; the elaborator decides nothing directly and
  writes only through its confined `AdmitPatch` capability;
- schedulers, attention planners, initiative selectors, and resolution covers;
- acceptance drivers, smoke binaries, checkpoints, resume journals, caches,
  and compaction summaries;
- Vault retrieval, evidence reranking, external consumers, and provider
  telemetry;
- import, reload, fork, and repair helpers.

Each is either a read-only projection or a producer of one ordinary typed
command.

## Canonical world state

`WorldState` is one aggregate with these owned partitions:

```text
WorldState
  identity: world_id, revision, phase, state_digest, last_commit_digest
  governance: members, actor bindings, contract, boundaries, approvals
  ontology:
    places and directed routes
    subjects and typed components
    relations
    pressures and commitments
    facts, knowledge grants, and provenance
    external-owner grants
  time: fictional clock
  events: committed factual and speech events
  applied_commands: bounded idempotency ledger
```

The lifecycle is data inside the aggregate:

- `draft`: membership, negotiation, private/shared speech, character creation,
  evidence admission, and ontology construction are allowed by draft authority;
- `active`: player and autonomous decisions, time, causal expansion, and
  contract amendments are allowed by active authority;
- `archived`: only read/export operations are allowed.

Activation changes `phase` in place after the current revision has the required
member approvals. It does not copy a preview into another store or hand truth
from one kernel to another.

## Ontology and relational algebra

The component mutation algebra is the foundation. The aggregate `Campaign`
shape is not a second canonical representation.

### Runtime-issued identity

Stable IDs are issued by the reducer. Creation proposals use local draft
handles plus exact existing IDs. Lowering resolves the complete local reference
graph before allocating canonical IDs. Display names never function as keys.

Unknown IDs, cross-kind references, dangling endpoints, and ambiguous handles
are rejected without inference.

### Subjects

A subject is a stable identity with a declared kind:

- person;
- institution;
- autonomous population.

An externally controlled mirror is not a fourth kind. It is an ordinary
subject of one of these three whose `ControllerAssignment` is
`ExternallyControlled { consumer }`: no controller ID, no controller mode, no
decision opportunity, and no affordance, mutable only through its own
consumer and the world owner.

Demographic descriptors and statistical aggregates are not automatically
subjects. A canonical action owner is derived structurally from an active
autonomous subject, one non-overlapping decision controller, explicit decision
authority, and at least one executable typed affordance. Similar names, motives,
or cultures are legal; labels alone cannot create causal capacity.

Population partitions use disjoint typed slice keys. Individuation is an
explicit causal mutation. There is no fission quota and no active-leaf
completion metric.

### Components

Subject and world behavior is composed from narrow typed components:

- position and jurisdiction;
- decision controllers, authority scopes, and executable affordances;
- resource quantity, custody, and dependency;
- knowledge of exact fact IDs and communication access;
- commitments, pressures, and dependency exposure;
- Persona material: values, voice, goals, memories, and relationship reads;
- source provenance and external ownership.

Persona text can enrich lived meaning. It cannot substitute for an authority,
custody, topology, or knowledge component.

### Relations

Relations have typed endpoints and kind-specific constraints. The vocabulary
covers:

- containment and occupancy;
- membership and lineage;
- jurisdiction and representation;
- custody and dependency;
- authority and control;
- supply and service;
- alliance, opposition, and obligation;
- knowledge and communication;
- topology and access;
- causal pressure and exposure.

Civic order is a typed subgraph of authority, selection or succession,
resource access, representation, and redress relations. It is not a prose
manifest with fact-ID lanes and does not require a model's civic verdict.

### Structural admission

The reducer validates only structural promises:

- every referenced entity exists or is created in the same closed batch;
- containment is acyclic;
- routes have exact place endpoints and valid costs;
- relation endpoint kinds match the relation kind;
- custody quantities and transfers conserve declared resources;
- authority and affordance scope covers the proposed mutation;
- knowledge additions cite an existing accessible fact or a newly admitted
  evidenced fact;
- population slices are disjoint beneath their declared parent;
- an externally controlled subject's components can change only through its
  own consumer, confined by `PatchGround::Consumer`;
- a batch either reduces completely or not at all.

Interestingness, novelty, ideological distribution, prose style, and counts do
not appear in this validator.

## One command and commit path

All user and system operations lower through the same path:

```text
authenticated intent or model tool call
  -> CommandEnvelope
  -> mailbox ordering
  -> load exact WorldState revision
  -> derive MutationAuthorityEnvelope
  -> lower command to WorldMutationBatch
  -> validate and reduce the complete candidate
  -> atomic WorldState + WorldCommit CAS
  -> publish invalidation
  -> derive projections and next attention plan
```

The closed command vocabulary includes lifecycle and membership changes,
contract and boundary changes, speech, ontology construction, subject
decisions, time advance, external snapshots, evidence admission, and archive.
Every command lowers to the same mutation vocabulary. There is no
`commit_elaboration`, separate component-batch ingress, or reload repair path.

## Inference boundary

Ghostlight supports two first-class decision interfaces. The ontology assigns
each exact authority scope to one `DecisionController`; the scheduler does not
choose a mode opportunistically. The scheduler does choose resolution: a
`NarrativePersona` controller receives its prose membrane in a singleton cell
and is represented operationally, at coarse resolution, when grouped. That is
a budget decision, not a mode change; the controller, its scope, and its
authority are unchanged. A controller changes representation and model
ergonomics, never permission or commit authority. A `NarrativePersona` in a
grouped cell receives its typed view and nothing from the membrane; it mints
no `PersonaTurn`, so the immutable-turn-receipt rule does not apply to it.
The membrane is not weakened; it is not entered. The agency graph is
scheduler-only and is reachable from no prompt builder; its mailbox request
is port-narrowed (absent from the controller and elaboration ports), not
principal-authenticated.

An opportunity is issued to exactly one controller. An untranslated intent or
infrastructure fault cannot fall through to the other mode, and the same
opportunity cannot be exercised twice.

### NarrativePersona controller

The narrative controller keeps one deliberate three-organ membrane around
roleplay. This is not a verifier chain: each organ owns a different
representation and none can commit.

### Projector

Deterministic code first constructs a permissioned `SubjectView` from exact
occupancy, knowledge, memories, relationships, pressures, contract boundaries,
and currently executable affordances. Private views omit unavailable facts and
other members' private state by construction.

The Projector receives that view and the visible stimulus. It emits one lived
narrative stream: perception, memory, uncertainty, desire, bodily circumstance,
and available possibility in prose. It cannot choose an action, emit a schema,
or claim a consequence. The Projector is a semantic renderer across the typed
state-to-lived-experience boundary; its prose is private derived context, not
world truth.

### Persona

The Persona receives only identity guidance and the lived narrative stream. It
does not see canonical field names, IDs, JSON, tool definitions, mutation
vocabulary, effect schemas, or kernel errors. It emits only natural roleplay
prose and may remain silent, speak, decide, hesitate, or attempt something in
character.

This isolation is an explicit quality invariant. Coding-agent training creates
a powerful schema and tool-use attractor. Ghostlight does not assume that a
model can preserve equivalent roleplay while simultaneously operating tools.

### Interpreter

The Interpreter receives the Persona prose, the lived stream, and the exact
permissioned typed context. Interpretation is a total operation. It always
finishes with an `InterpretationReport` containing:

- the immutable Persona turn receipt and exact prose preserved as noncanonical
  source evidence;
- every typed proposal it could faithfully lower using existing IDs and
  available affordances;
- zero or more `TranslationGap` records for meaningful material it could not
  encode;
- the exact source spans supporting both proposals and gaps.

Before interpretation, the NarrativePersona runner persists one immutable
`PersonaTurn` receipt binding controller, decision opportunity, world
revision/state digest, Projector receipt, Persona inference receipt, exact
source prose, and its digest. The report carries that source receipt rather
than reconstructing provenance from prompt text. Report fields are produced by
the accumulator; arbitrary deserialization is not an admission path.

A translation gap identifies ambiguity, a missing reference, a missing
affordance, a missing mutation primitive, or source left unresolved by the
current attempt. It records what the Interpreter believes the Persona was
trying to express without pretending that the effect occurred. Gaps are
non-fictional inference evidence. They may inform evaluation or an explicit
later design decision; recording one does not request elaboration or mutate the
world.

Interpreter tools provide local structural feedback and a `record_gap` action.
An invalid proposal span is not admitted, but the harness records the complete
source as an unresolved gap. An invalid gap span is likewise rebound to the
complete exact source rather than discarded. Raw tool arguments that cannot be
decoded into either typed contract enter the same total fallback, with their
payload digest retained as attempt evidence. If the step budget ends, the
harness finalizes the valid proposals and accumulated gaps and adds an exact
unresolved-source gap instead of returning semantic failure.

The Interpreter cannot add motivation to the Persona turn, rewrite its source
prose, infer unavailable knowledge, or commit an effect. Spoken words become a
typed speech proposal; wondering, deciding, attempting, and narration do not
become audible speech merely because the Persona wrote them. The kernel remains
the final structural guard for typed proposals. A stale revision or persistence
fault is a command/infrastructure outcome, not an interpretation failure, and
does not summon a semantic verifier or reconciler.

Any model transport or dispatch fault before explicit semantic finalization
produces no report. The immutable Persona turn remains pending for a fresh
interpretation attempt; partial captures are discarded as attempt telemetry,
not committed as a half-report. `StepBudgetExhausted` means the harness reached
its configured semantic stopping point normally. Infrastructure interruption is
unavailable execution, not rejected meaning.

World authoring uses a separate typed proposal surface because it is not
roleplay. It does not pass through Persona or borrow Persona authority.

### OperationalAgent controller

An operational controller receives a permissioned typed view and exact tool
schemas directly. It is suitable when the simulated mind is itself an
operator: an institution, political or administrative Gestalt, logistics organ,
market participant, or other subject whose meaningful cognition is explicit
state reasoning rather than embodied dramatic experience.

The operational agent may inspect only the state and affordances granted to its
controller and may submit zero or one typed decision proposal. It may record an
unrepresented operational need, but it cannot emit a NarrativePersona turn as
fallback, claim facts outside its view, or commit.

Subject kind does not secretly choose the interface. A person normally carries
a narrative controller and an administrative institution often carries an
operational controller, but authored ontology makes the assignment explicit.
If one institution needs both an operational decision organ and a person-shaped
public representative, they are separate controllers with disjoint authority
scopes. Awkward overlap is rejected rather than resolved by precedence.

### Presentation inference

Optional narration renders committed events for a particular audience. It is
display-only. The exact typed events remain available beside the prose, and a
rendering failure cannot block, alter, or repair a commit.

Inference receipts are operational telemetry in a separate store. The kernel
does not require a provider, model tier, stage name, output hash, or verifier
receipt to authorize world state.

## Action and consequence

An action proposal names one subject, one executable affordance, exact targets,
and typed desired effects. The kernel derives authority and feasibility from
the current ontology. If uncertainty matters, server-owned entropy selects a
bounded outcome band. Only the typed effects for that band enter the mutation
batch.

Speech is an event and may accompany an action. Generated speech cannot imply
that an uncommitted effect occurred. Social appraisal and private memory are
ordinary scoped mutations proposed by the affected subject; they do not gain a
separate appraisal or verifier authority.

Player, NPC, institution, population, import, and strategic actions share this
primitive.

## Autonomous scheduling

The scheduler is a pure planner over one revision. It orders eligible subjects
using unresolved pressure and time since the last opportunity, with subject id
as the final tiebreak. Causal exposure is derivable from `dependencies` and is
not an ordering input. It emits `DecisionOpportunity` values and cannot commit.

`drive_cover_tick` is the single owner of tick cadence, cover derivation, and
the clock: it derives the cover, runs the cells under a bounded concurrency
permit pool with a quarantine flag, and advances the clock after the cells so
every cell in a tick shares one `now` and one tick index. Its budget is
configuration (`GHOSTLIGHT_COVER_CELL_BUDGET`, `GHOSTLIGHT_COVER_CONSTITUENT_CAP`,
`GHOSTLIGHT_COVER_URGENCY_SLOTS`, `GHOSTLIGHT_CONTROLLER_MAX_CONCURRENT`,
`GHOSTLIGHT_TICK_INTERVAL_SECONDS`); changing any is a restart. The five
controller models are configuration too (`GHOSTLIGHT_CONTROLLER_PROJECTOR_MODEL`,
`_PERSONA_MODEL`, `_INTERPRETER_MODEL`, `_OPERATIONAL_MODEL`,
`_ELABORATOR_MODEL`), each with a default when absent.

The kernel commits one decision at a time through the mailbox. After each
commit the planner derives a fresh queue. Parallel inference may speculate on
one snapshot; a proposal binds to the scope digest of the components its
verification reads, commits at any later revision where that digest is
unchanged, and is rejected with a typed `ScopeChanged` when it is not. Nothing
is rebased and nothing is discarded merely because the world moved elsewhere.

Resolution covers and grouping are compute budgets for projection. They do not
create, merge, fission, or qualify identities. Every active subject is in the
cover every tick; the budget decides at what resolution. Singleton cells give
detail focus and the prose membrane; grouped cells represent their constituents
operationally at coarse resolution with per-constituent attribution. Debt
rotation guarantees every subject reaches detail focus within bounded ticks.
The 2,400-subject, 240-cell profile is a design target for a living world, not
only a load fixture: it tests the elaborators and the simulation for quality
under scarcity of attention.

## Source grounding and open-world expansion

Vault retrieval returns exact evidence receipts. One author inference may fill
semantic fields in a typed seed or boundary-expansion proposal. The reducer
issues IDs and validates the closed structural graph.

Seed construction, destination expansion, locality detail, causal
individuation, and external import all use the same ontology mutation
vocabulary. Expansion is requested only for a named causal boundary. A sparse
world with enough structure for its current horizon is valid.

Qualitative evaluators may report implausibility, monotony, missing social
texture, or poor narrative leverage after a run. Those reports guide a future
author proposal; they do not become a hidden admission tribunal.

## Persistence and recovery

One world `.cc` store contains:

- one current `world_state.v1` row;
- immutable `world_commit.v1` rows forming a digest chain;
- immutable exact evidence receipts referenced by commits.

Service authentication and model telemetry use separate service-owned stores.
They cannot participate in world authority.

Recovery verifies `(world_id, revision, state_digest, last_commit_digest)`,
starts the mailbox, derives due work, and continues. In-flight inference is
disposable. A repeated command ID is idempotent only when its full digest
matches the recorded commit. Physical filenames, smoke checkpoints, prose
summaries, and status files are not recovery truth.

Pre-rebuild campaign stores remain immutable evidence. The new runtime does not
load them into live authority through a compatibility adapter. Any future
migration must be an explicit, audited import that emits ordinary commands.

## External boundaries and surfaces

- Heimdall owns public identity; Ghostlight stores only the app-local principal
  needed to derive membership authority.
- VoidBot or another Vault provider owns retrieval; evidence receipts grant
  provenance, not mutation authority.
- External world consumers retain sovereignty over their declared subjects.
  Ghostlight imports only owner-signed components through the ordinary command
  path and cannot invent internal state for them.
- Idunn owns deployment and daemon continuity; Odin owns discovery. Neither can
  write a world.
- Eve/CultUI and CultMesh expose typed projections of the same `WorldState` and
  command catalog. Renderers do not own parallel dashboard truth.

## Deletion line

The rebuild removes these authorities before replacing behavior:

- `SessionZeroKernel`, its registry/director state, and publication handoff;
- canonical aggregate `Campaign` and the component-to-Campaign projection
  writer;
- `legacy_transition` and every special construction or repair commit path;
- duplicate or cell-specific Projector-Persona-Interpreter implementations;
  implicit mode switching or fallback between narrative and operational
  controllers; effect verifier, assessment verifier, outcome verifier, civic
  verifier, and reconciliation roles;
- destination identity inference and name-based structural binding;
- model resolution demand, Nemesis selection, and identity invention;
- titled elaborator quotas, complexity qualification, semantic qualification,
  fission completion counts, and model compaction memory;
- newspaper selection/editor/copy-desk authority and checkpoint recovery;
- acceptance-driver gates that decide world validity;
- checkpoints, resume files, caches, and model receipts used as fictional truth.

The retained foundations are the typed component mutation algebra, one mailbox,
CultCache CAS and immutable receipts, exact evidence, deterministic topology and
scope validation, actor-private projection, and the declared service ownership
boundaries.

## Verification contract

Focused tests prove the machine under fixture inference ports. The one place
the road is exercised against a real controller is the local live smoke
(`notes/local-live-smoke.md`): the production tick driver, cover, Persona
membrane, operational lane, clock, and elaboration sweep against a
CodexConnector on a genesis world. Its first run (2026-09-05) proved the path
end to end and showed that a three-subject, one-room genesis yields thin
prose; a seed producer is the gap that finding names. Step 8 landed that seed
producer: `world.create` v2 carries the scale intent at genesis, `world.seed`
runs one `SeedRunner` session per invocation against a `VaultEvidenceSource`,
and the extended live smoke seeds a world before it ticks.

Focused tests must prove:

1. Draft and active commands mutate the same aggregate through the same commit
   primitive.
2. No alternate input can write canonical state.
3. Unknown IDs, dangling routes, invalid relation endpoints, custody creation,
   authority escape, private-knowledge leakage, and external-owner violation
   fail before persistence.
4. Interpreter semantics are total: representable material becomes typed
   proposals, unrepresentable material becomes exact translation gaps, and no
   semantic path terminalizes the interpretation or calls a verifier or
   reconciler model.
5. Scheduler, transcript, news, and surfaces can be discarded and rebuilt from
   the same committed state.
6. Restart resumes from the commit digest chain with no checkpoint or repair
   pass.
7. A sparse structurally supported world activates successfully.
8. A synthetic 1,200-owner fixture exercises scheduling and projection without
   changing production completeness rules.
9. Direct user, model, scheduler, import, and reload paths all reach the same
   reducer and CAS.
10. Every autonomous opportunity resolves through one exact controller; its
    authority scopes do not overlap another controller, and gaps or faults
    cannot trigger cross-mode fallback or double action.

The Projector-Persona-Interpreter membrane has its own behavioral evaluation.
Operational-agent success in its admitted scopes is not evidence that tools are
safe for a person-shaped role. Any proposal to replace a `NarrativePersona`
controller with a tool-using agent must compare both designs on the same
sequential fixtures, model, visible facts, and token budget. It must show no
material regression in voice, embodiment, uncertainty, tonal range, schema
leakage, tool-shaped diction, or continuity while matching typed-effect
fidelity. Until that evidence exists, narrative Personas remain prose-only.

Qualitative world and narrative evaluation is a separate post-run artifact. It
may be severe. It may not become a writer.
