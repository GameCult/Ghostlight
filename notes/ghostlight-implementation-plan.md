# Ghostlight Implementation Plan

## Objective

Rebuild Ghostlight Dungeon as one legible world machine: one lifecycle, one
typed ontology, one command ingress, one authority derivation path, and one
atomic persistence owner. Preserve narrative roleplay where it matters and use
direct operational cognition where it is a better fit, without granting either
mode additional authority.

## Current mechanism

Pushed commit `6bb6869` makes the sealed mailbox/kernel architecture the crate
and executable runtime identity. Pushed commit `13d5136` places app sessions,
the world journal, and controller custody behind the one vendored CultCache
implementation and removes the duplicate persistence dependency. The committed
daemon tree contains no pre-rebuild Session Zero, legacy kernel, scheduler,
assessor, verifier/reconciliation, or legacy-transition module path.

Production has not crossed that source boundary. Yggdrasil still runs legacy
release `a4080d4` under an enabled `Restart=always` unit and a legacy state
root. CultNet through CultLib `85f7024` owns generation-bound activation,
separate lifecycle-brake, process-write-lease, observed-capability, explicit
disagreement, and routed RUDP incarnation admission contracts. Odin through
pushed `65cf2b2` owns deterministic recipe and binding admission, exact source
freezing, sealed releases, Expected projection, the narrow native actuator
ports, and the durable three-record transaction and admitted-generation engine.
Remaining Odin work is Idunn projection, bounded RUDP transport, and stable-route
integration; the active move is to finish and prove those before one-way live
cutover.

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

### 2. Add the minimal private reducer — landed

`66ed2ec` implements the state and command vocabulary required for Draft
creation, approval, activation, one human action, and one autonomous action.
The aggregate owns controller assignments and affordance grants and derives
exact revision-bound opportunities. IDs issue only after complete seed
validation; rejected and empty reductions do not commit. Places, relations,
resources, external ownership, and other ontology components wait for a live
typed causal boundary.

### 3. Prove the private authority slice — landed

Eighteen focused tests exercise Draft creation, approval, activation, one human
decision, one NarrativePersona-controlled decision, and one
OperationalAgent-controlled decision through the same aggregate action command.
They cover exact controller scope and affordance matching, revision-bound
opportunity rejection, restart, immutable genesis, idempotency, serialized
mailbox ownership, cancellation, lost reply semantics, and forged-history
rejection. Controller mode changes representation only; it does not create a
second reducer path.

Live `NarrativePersona` integration still requires the Projector to emit prose
context, the Persona to return prose only, and the runner first to persist an
immutable receipt-bound Persona
turn. The Interpreter returns a completed report containing that exact
noncanonical source prose, zero or more typed proposals, and zero or more
translation gaps. Spoken words require a typed speech proposal. Invalid capture
spans and normal step exhaustion become exact unresolved-source gaps. Any
transport or dispatch fault before explicit finalization discards partial
captures and leaves the immutable source pending for a fresh attempt; it does
not become semantic interpretation failure.

Live `OperationalAgent` integration receives only its permissioned typed
view and tools. Its proposals enter the same command path as narrative
proposals. It does not bypass projection privacy, authority derivation, or the
reducer.

### 4. Expose, wire, and cut — landed in source

Commits `6bb6869` and `13d5136` expose the replacement facade as the crate and
executable runtime identity while keeping mutable state, ID issuance,
reduction, journal access, and authenticated-caller construction sealed.
Startup creates or opens one replacement owner and spawns one mailbox; runtime
consumers receive only the mailbox-facing command and snapshot surface.

Draft creation, approval, activation, player decisions, and autonomous
controller proposals now enter that boundary. The committed daemon tree no
longer contains the old Session Zero, legacy kernel, scheduler, verifier,
reconciliation, or recovery-writer routes. There is no dual-write or live
compatibility router in source.

Resolution, elaboration, transcript/news projection, and external publication
remain future typed consumers at their causal boundaries. Model transport,
Vault retrieval, Heimdall identity, Eve/CultMesh projection, Idunn health, and
external adapters survive only where their ownership remains clean.

### 5. Integrate deployment actuation and cut production — active

The deterministic foundation is landed: target recipes and Idunn operator
bindings compile into private plans and sealed releases, and only a sanitized
Expected incarnation may leave that control plane. Shared CultNet contracts own
generation-bound activation, service-signed Present health, observed
capabilities, explicit disagreement, and process-bound write leases. Odin
through pushed `65cf2b2` lands the exact source path, the narrow actuator ports,
dynamic systemd isolation, Ready-provider selection, protected activation
delivery, and the deployment engine that replaced the command-owner stub.

That engine persists exactly three control record types: one command with
a frozen target order, one crash-resumable transaction per target, and one
CAS-owned admitted generation per target. The transaction phases are Sealing,
Starting, Warming, Fencing, Leasing, AwaitingReady, Routing, Committing, and
Complete; stateless targets skip only fencing and leasing. Activation is
prepared without starting a process, its public identity is persisted, and only
then may the prepared process start. A credential or unit with no persisted
activation owner is an orphan and cannot be adopted. Committing atomically
replaces the exact incumbent and completes the transaction.

Targets own constrained launch declarations, never raw unit or container
templates. The operator binding selects the workload driver, which alone lowers
that declaration into process-manager configuration. Likewise, deterministic
plan validation retains exact commit, tree, recipe, and Gitlink facts but proves
neither ancestry, signatures, nor object custody; the narrow Idunn-owned source
driver must establish those facts before actuation.

The integrated path must publish Expected from the sealed plan and release,
record Idunn's observation of the exact runtime activation, and require the
service's signed Present health. A stateful candidate receives its process-bound
write lease before it opens writable state. Odin alone correlates Expected,
activation, and Present into Ready. Only after Ready may Idunn change stable
route membership and drain the incumbent. The deployment brake gates body
changes; same-release continuity remains separately owned and may be stopped
only by an explicit lifecycle brake.

Idunn starts and recovers from its own durable admitted state. Odin is the first
managed semantic daemon, never an Idunn bootstrap dependency; initial Odin
admission is the sole graph-bootstrap exception and begins from a root-admitted
local binding. During an Odin outage, Idunn may preserve already-admitted
continuity and routes, but it may not start a graph-changing transaction or
promote without the exact frozen Odin Ready receipt for that runtime instance
and presence digest. Idunn never manufactures Ready locally.

Deploy CodexConnector first, then Ghostlight. Ghostlight's live cut archives the
entire legacy state root, creates a clean world-v2 root, and validates a complete
allowed live layout. After route, health, process, write lease, restart, and
negative legacy checks agree, delete the old units, releases, state roots,
acceptance debris, gamecult-ops target deploy programs, and local run
scaffolding.

### 6. Widen the ontology to the causal boundary — designed, not implemented

The operator ordered this stage ahead of the deployment cutover. Step 2 deferred
places, relations, resources, and external ownership until "a live typed causal
boundary" existed; this stage builds that boundary.

The closed vocabulary is `docs/architecture/ghostlight-world-ontology.md`: typed
ID namespaces, draft-handle references resolved inside one closed `WorldPatch`,
twelve decision-constraining components over twenty-nine named operations,
world-authored affordances (preconditions, effect slots, outcome bands) that
make character action a deterministic precondition-effect transition, four
derived `CausalBoundary` kinds plus draft `SeedRequest`, scope-digest binding
for proposals, and one `AdmitPatch` command shared by seed admission and
boundary elaboration. Eight elaborators are one `OperationalAgent` loop whose
tool catalog is a projection of the operation set. The document carries its own
cut line, subtraction budget, build budget, and eighteen-proof verification
contract.

Implementation order, each pass landing tests before the next begins:

1. Typed ID namespaces, `Ref<Id>` with draft handles, closed-patch resolution,
   and the complete mismatch set; no components yet. Proves the Run 115
   rejection shape against an otherwise empty ontology.
2. `Position`, `Route`, containment, topology admission, and scope-digest
   binding for opportunities.
3. `Custody`, `Dependency`, conservation, and evidenced admission.
4. Affordance catalog: preconditions, effect slots, kernel-entropy band
   selection, and the action pipeline replacing `Speak`-only invocation.
5. `Authority`, `Selection`, `Redress`, and institutional affordances.
6. `Knowledge`, `Channel`, `Fact` standing, and scoped-projection non-leakage.
7. `Commitment`, `Pressure`, obligation → pressure → opportunity flow, and
   boundary derivation.
8. `AdmitPatch` with boundary and seed-request binding, and the derived
   elaborator tool catalog.

The deployment cutover (step 5) is deferred behind this stage, not cancelled. No
world acceptance run may start while Yggdrasil serves the legacy body.

### 7. Contract verification

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

- Build host: Yggdrasil through Idunn-configured pinned Linux containers. Do
  not build the deployment body on Windows.
- Control plane: `odin-core` library tests plus `idunn-daemon` library/binary
  checks; Linux x86_64 release artifacts are `idunn` and `idunn-provision` only.
- Targets: `codex-connector` library tests with and without `daemon`, then the
  `codex-connector` release binary with `daemon`; `ghostlight-dungeon` focused
  tests, then the `ghostlight-dungeon` release binary. Default target features
  remain unchanged unless the target-owned recipe names an existing required
  feature.
- Output roots, container digests, cache mounts, current footprints, retention,
  and expected deltas are operator-binding inputs and must be measured on
  Yggdrasil before the first build. No workspace-wide, all-target, clean, or
  cross-platform build is admitted.

## Deferred research lanes

The 36-case agency corpus, separate-account multiplayer proof, visual/Ink
fixtures, newspaper variation, and direct-tool Persona comparison remain useful
research lanes. They do not steer the authority rebuild and cannot reopen a
retired writer. Historical plans and results remain recoverable through Git,
the evidence ledger, and the frozen pre-rebuild system map.
