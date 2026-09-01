# Ghostlight Implementation Plan

## Objective

Rebuild Ghostlight Dungeon as one legible world machine: one lifecycle, one
typed ontology, one command ingress, one authority derivation path, and one
atomic persistence owner. Preserve narrative roleplay where it matters and use
direct operational cognition where it is a better fit, without granting either
mode additional authority.

## Current mechanism

The repository still contains the pre-rebuild runtime: separate Session Zero
and campaign kernels, aggregate Campaign and component representations,
multiple mutation ingresses, semantic verifier and reconciliation stages,
model-owned scheduling decisions, and persisted recovery/checkpoint paths.
These mechanisms are teardown evidence, not foundations that must survive.

The target map is adopted. The first subtraction removed seven pipeline-smoke
binaries and two live-fire scripts. The projection crate now exposes explicit
`NarrativePersona` and `OperationalAgent` modes plus a total Interpreter report.
An uncommitted broad `world.rs` prototype was rejected and deleted before it
became authority. It exposed mutable aggregate state and ID issuance, placed
translation-gap evidence inside canonical world types, permitted empty commits,
and defined controllers without owning executable affordances or exact decision
opportunities. The sealed aggregate boundary is landed. The active bounded move
is now the minimal private reducer, not a public ontology inventory.

## Authority map

- **Owner:** one sealed per-world `WorldKernel` owns its private aggregate,
  reducer, ID allocator, and journal writer. The public crate boundary exposes
  only create/open, immutable snapshot, command submission, and typed receipts;
  it does not expose mutable `WorldState`, the canonical entity-ID allocator, reducer
  entry points, or generic journal writes.
- **Inputs:** an authenticated `CommandEnvelope` with stable command ID, exact
  world and expected revision, principal evidence, one closed command body, and
  exact source receipts where needed. The kernel privately loads current state,
  derives authority, and supplies deterministic clock or entropy. Human,
  Interpreter, operational-agent, author, scheduler, and external-owner output
  is proposal material only.
- **Outputs:** a rejection receipt changes nothing. An accepted command returns
  one `WorldCommit`, including any reducer-issued ID mapping, and atomically
  appends one digest-chained revision. A structurally valid command that lowers
  to no factual, speech, lifecycle, or time mutation returns `NoEffect` and does
  not advance revision, change either digest, allocate IDs, or append a journal
  row.
- **Derived state:** immutable snapshots and subject views, currently executable
  affordance catalogs, revision-bound `DecisionOpportunity` values, scheduler
  queues, translation gaps, projections, news, transcripts, model receipts,
  checkpoints, coverage counts, and evaluations. Translation gaps may be kept
  as inference/evaluation telemetry or motivate a later explicit author
  proposal; they are not a `WorldState` component, event, or mutation.
- **Forbidden writers:** public state setters or ID issuers; direct reducer
  callers; generic CultCache insert/append handles; schedulers or controllers
  that assign authority, mint executable affordances, consume opportunities, or
  commit; automatic translation-gap lowering; empty commits; the old Session
  Zero publication path, aggregate Campaign writer, legacy transition
  projection, alternate elaboration and mutation inputs,
  verifier/reconciliation agents, recovery repair loops, and external consumer
  callbacks.
- **Shared paths:** draft creation, approval, activation, player action,
  autonomous action, author expansion, time, travel, imports, reload, and
  administration use create/open/snapshot/submit and the same private
  derive/reduce/commit primitive. Controller harnesses submit an exact typed
  proposal as an ordinary command. Recovery opens and verifies the same journal;
  it cannot call a repair reducer.
- **Cut line:** the rejected public ontology stays deleted. Translation-gap
  records stay outside canonical world state. No public constructor or helper
  may issue a canonical ID, mutate an aggregate, invoke the reducer, or write the
  journal. Obsolete writers are deleted or reduced to read-only evidence before
  their replacement path is called complete.
- **Verification layer:** compile-time visibility proves that an external crate
  cannot invoke the ID allocator, mutate `WorldState`, call the reducer, or
  obtain the journal writer. Focused black-box tests prove that a caller-supplied
  unknown ID cannot become canonical, plus atomic rejection, no-op non-commit,
  reducer-only ID allocation, exact restart/digest recovery, translation-gap
  non-authority, and opportunity rejection when revision, controller, scope, or
  executable affordance does not match.

`SessionZeroKernel` is no longer a target owner; Draft is a `WorldState` phase.
Aggregate Campaign state is no longer an owner; aggregate views are derived
from typed components. A checkpoint is no longer a repair authority; it is a
projection of committed history.

## Invariants

1. Every world mutation passes through one deterministic reducer and one
   atomic compare-and-swap commit.
2. Authority is derived inside the kernel from canonical membership,
   governance, custody, jurisdiction, and controller scope.
3. Each autonomous decision scope has exactly one controller. Cognition mode
   changes representation only; there is no fallback or double action.
4. Narrative interpretation is total. Persona prose is preserved; faithful
   typed proposals and exact translation gaps account for the turn. The
   Interpreter does not accept, reject, or commit world state.
5. Structural invalidity is rejected deterministically before persistence and
   without a model verifier. A failed batch changes nothing.
6. Private knowledge remains scoped until an explicit communication command
   changes it.
7. External consumers retain sovereignty. Ghostlight views, proposals, and
   acknowledgements cannot mutate external-owned state.
8. Sparse causal sufficiency controls elaboration. Counts and qualitative
   review remain evaluation evidence only.
9. Restart reconstructs the exact committed world and idempotency history; a
   recovery loop cannot repair or reinterpret it.
10. Canonical IDs are allocated only while a complete private reduction is
    being admitted. Rejected and no-effect commands cannot consume or reveal
    them.
11. Controller assignment and affordance grants are canonical aggregate state.
    Current executability and decision opportunities are deterministic
    revision-bound derivations. A scheduler can order those opportunities but
    cannot mint their authority.

## Implementation sequence

### 1. Seal the aggregate boundary — landed

Establish `WorldKernel` with a private aggregate and private journal child before
publishing ontology types. Expose only create/open, immutable snapshot, submit,
and typed receipts. Make canonical state mutation, ID allocation, reduction,
and CultCache writes unreachable from outside the owning module. Prove the
visibility boundary and the one-writer transaction seam before widening the
state vocabulary.

### 2. Add the minimal private reducer

Behind that boundary, implement only the state and command vocabulary required
for Draft creation, approval, activation, one actor action, and one autonomous
action. The aggregate owns controller assignments and affordance grants; it
derives exact current executability and revision-bound opportunities. Allocate
IDs only after a complete candidate validates, reject empty reductions without
a commit, and persist accepted state plus commit atomically with expected
revision and idempotency. Add places, relations, resources, external ownership,
and other ontology components only when the vertical slice reaches their typed
causal boundary.

### 3. Exercise one vertical slice

Route Draft creation and activation, one player action, and one autonomous
action through the aggregate. Derive a subject-private projection and one
decision opportunity. Exercise both cognition modes without allowing either
to write directly.

For `NarrativePersona`, the Projector emits prose context, the Persona returns
prose only, and the runner first persists an immutable receipt-bound Persona
turn. The Interpreter returns a completed report containing that exact
noncanonical source prose, zero or more typed proposals, and zero or more
translation gaps. Spoken words require a typed speech proposal. Invalid capture
spans and normal step exhaustion become exact unresolved-source gaps. Any
transport or dispatch fault before explicit finalization discards partial
captures and leaves the immutable source pending for a fresh attempt; it does
not become semantic interpretation failure.

For `OperationalAgent`, the controller receives only its permissioned typed
view and tools. Its proposals enter the same command path as narrative
proposals. It does not bypass projection privacy, authority derivation, or the
reducer.

### 4. Migrate and cut

Move Session Zero, turn handling, resolution, elaboration, scheduling,
persistence, transcript/news projection, and external publication onto the
vertical slice. At each move, delete or neuter the old writer first. Retain
model transport, Vault retrieval, Heimdall identity, Eve/CultMesh projection,
Idunn health, and external adapters only where their ownership remains clean.

### 5. Contract verification

Required black-box proofs:

1. Draft → Active → player action → autonomous action → restart preserves the
   exact digest and commit history.
2. Invalid ontology patches are atomic and model-free; a typed causal boundary
   can activate sparse new state.
3. Models, scheduler, projection, and recovery are inert without an explicit
   aggregate submission.
4. Private knowledge stays private until explicit communication.
5. External proposals and acknowledgements cannot mutate consumer-owned state;
   only a fresh owner snapshot can update the local observation.
6. Narrative and operational controllers have exact disjoint scopes, never
   fall through, and never consume one opportunity twice.
7. Interpretation always completes semantically and preserves unlowered
   material as translation gaps.
8. A no-effect command leaves revision, state digest, commit digest, ID
   allocation, and journal length unchanged.
9. An external crate cannot obtain mutable aggregate state, invoke the canonical
   ID allocator, call the reducer, or write the journal; a caller-supplied
   unknown ID is rejected rather than admitted.
10. A decision proposal is rejected when its revision-bound opportunity,
    controller, scope, or currently executable affordance is wrong; the
    scheduler cannot manufacture or consume authority.

A 1,200-actor synthetic fixture may measure load after these pass. It cannot
serve as an ontology or completeness gate. No live Delvehold acceptance run is
admitted before the focused suite passes and the old writers are structurally
unable to override the aggregate.

## Subtraction budget

Prefer deletion, collapse, or reuse before adding surfaces. The replacement
buys explicit capabilities: one lifecycle owner, one structural reducer, one
persistence writer, disjoint cognition controllers, total interpretation, and
restart proof. It must retire more writer and stage liability than it adds.
Review each completed pass for modules, binaries, schemas, dependencies,
processes, and tests added or removed.

## Build budget

- Package: `ghostlight-dungeon`, plus the pinned
  `ghostlight-persona-projection` crate when its contract changes.
- Profile/target: Windows MSVC debug test/check, default features only.
- Focused checks: library checks, named module tests, the rebuild contract test,
  and the daemon binary check after cutover.
- No workspace-wide, all-target, release, cross-platform, or clean build during
  surgery.
- Existing `target` footprint is roughly 30 GiB; stop before 33 GiB and inspect
  artifact fan-out before continuing.

## Deferred research lanes

The 36-case agency corpus, separate-account multiplayer proof, visual/Ink
fixtures, newspaper variation, and direct-tool Persona comparison remain useful
research lanes. They do not steer the authority rebuild and cannot reopen a
retired writer. Historical plans and results remain recoverable through Git,
the evidence ledger, and the frozen pre-rebuild system map.
