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
6. Private knowledge remains scoped until an explicit communication command or
   a witnessed event under the subject's place changes it.
7. External consumers retain sovereignty. Ghostlight views, proposals, and
   acknowledgements cannot mutate external-owned state.
8. Sparse causal sufficiency governs structural validity; counts and
   qualitative review remain evaluation evidence and never gate admission.
   Elaboration additionally pursues the seed's authored `WorldScaleIntent` as
   a derived scale deficit; cover budget is the deliberate choke on
   simulation attention.
9. Restart reconstructs the exact committed world and idempotency history; a
   recovery loop cannot repair or reinterpret it.
10. Canonical IDs are allocated only while a complete private reduction is
    being admitted. Rejected and no-effect commands cannot consume or reveal
    them.
11. Controller assignment and affordance grants are canonical aggregate state.
    Current executability and decision opportunities are deterministic
    derivations bound to a scope digest over the components they read; a
    proposal whose scope digest is unchanged commits at a later revision, and
    one whose scope changed is rejected. A scheduler can order those
    opportunities but cannot mint their authority.

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
entire legacy state root, creates a clean world-v3 root, and validates a complete
allowed live layout. After route, health, process, write lease, restart, and
negative legacy checks agree, delete the old units, releases, state roots,
acceptance debris, gamecult-ops target deploy programs, and local run
scaffolding.

### 6. Widen the ontology to the causal boundary — landed

The operator ordered this stage ahead of the deployment cutover. Step 2 deferred
places, relations, resources, and external ownership until "a live typed causal
boundary" existed; this stage builds that boundary.

The closed vocabulary is `docs/architecture/ghostlight-world-ontology.md`: typed
ID namespaces, draft-handle references resolved inside one closed `WorldPatch`,
twelve decision-constraining components over twenty-seven named operations,
world-authored affordances (preconditions, effect slots, outcome bands) that
make character action a deterministic precondition-effect transition, four
derived `CausalBoundary` kinds (a draft `SeedRequest` is representable, not
inhabited: Draft answers nothing), scope-digest binding
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
8. `AdmitPatch` with boundary and scale-deficit binding (Draft answers
   nothing; `SeedRequest` stays uninhabited); `WorldScaleIntent` with
   structural qualification and per-jurisdiction deficit derivation; and the
   derived elaborator tool catalog.
9. Budgeted connected cover: agency-graph partition under the cell budget,
   singleton detail focus, grouped coarse cells with partitioned views and
   per-constituent attribution, debt rotation. Scheduler-owned; the kernel
   never sees a cell.
10. Consumer ingress: landed in pass 10. `SystemCapability::Consumer` is the
    world's third `AdmitPatch` author, minted only by
    `WorldMailbox::submit_consumer` after `world/consumer.rs` authenticates a
    `ConsumerPatchDocument` against a `ConsumerRegistry` secret digest.
    `require_patch_author`/`confine_to_ground` widen to `PatchGround::Consumer`,
    confining a consumer to the subjects declared
    `NewController::External { consumer }` (`ControllerAssignment::ExternallyControlled`,
    no controller ID, no mode, no opportunity, no affordance). `patch::decode_patch`
    is the one decode bound, shared by the elaborator lane and the consumer
    lane, under `MAX_PATCH_BYTES`/`_DECLARATIONS`/`_OPERATIONS`/`_EVIDENCE`. The
    transport is `POST /cultnet/world-patch`, loopback-only, canonical
    MessagePack, schemas `ghostlight.consumer_patch.v0` and
    `ghostlight.consumer_receipt.v0`; the state/commit schemas bumped to
    `ghostlight.world_state.consumer.v1` / `ghostlight.world_commit.consumer.v1`
    (`world-v3`). The outbound consumer response — a consumer reading its own
    projection or an attributed proposal — is the next seam, not part of this
    pass.

The deployment cutover (step 5) is deferred behind this stage, not cancelled. No
world acceptance run may start while Yggdrasil serves the legacy body.

### 7. Contract verification

### 8. Seed producer — landed

Objective: a world that is alive at genesis rather than three subjects in one
room. The first live smoke (`notes/local-live-smoke.md`) proved the road and
showed that genesis yields thin prose because nothing authored exists to want
or be wanted.

What landed: `world_create.v2` carries `WorldScaleIntent` (targets and
jurisdiction roots) into the genesis patch itself, write-once; a v1-announcing
invocation is refused before any handler runs. `qualifies` lost its
Draft-excluding phase clause, so a Draft world's scale deficit is live and
falls as structure is authored; the Draft-answer refusal that used to hide
behind that clause now lives only in `require_answer`. The Draft seed lane —
`SeedPort` (two methods, `snapshot` and `submit_seed`), `SeedRunner`,
`SeedSession`/`SeedCheckpoint`, `ControllerWork::Seed` and `WorkLane::Seed` on
`controller_work.v10` — runs one authoring session per `world.seed`
invocation on the existing elaboration repair loop, admitting at most one
Draft patch as `CallerId::Principal(owner)` through the unconfined owner lane;
there is no marker in the journal distinguishing a seed-authored patch from a
hand-authored one. `VaultEvidenceSource` (`world/vault.rs`) is the seed lane's
production `EvidenceSource`: a read-only markdown directory reader whose
evidence reference is a note's vault-relative `.md` path.
`world.advance_time`'s missing `operation_schema` entry — every invocation of
it was silently unreachable — was fixed ahead of `world.seed`, and
`every_operation_the_panel_emits_has_a_schema` is the totality test that makes
that class of gap fail loudly instead of silently.

`SeedRequest` is resolved in the negative: Draft answers nothing, and the
seed lane's port cannot express an answered patch, so nothing ever needed
`SeedRequest` to bind. It stays representable in the `CausalBoundary`
vocabulary and uninhabited in practice.

Left undone: the seeded live run (the extended local live smoke seeds a world
from a real Vault before ticking it, but has not yet been run against a real
connector — see `notes/local-live-smoke.md`); `ControllerWorkCustody::Owned`
gained a `seed_commands` count while `Grouped` still has none, an asymmetry
named and not fixed; the Vault's keyword fallback scores by raw token-count
match rather than any real relevance ranking; and `VaultEvidenceSource::open`
reads its whole configured
scope into memory on every invocation rather than caching across sessions.

Cut line: no new reducer, ID allocator, conservation check, or
`SystemCapability`; no second tool surface; no compatibility path for the
three-subject genesis.


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
10. A decision proposal is rejected when its opportunity's scope digest has
    changed or its controller or currently executable affordance is wrong; the
    scheduler cannot manufacture or consume authority.

A 1,200-actor synthetic fixture may measure load after these pass. It cannot
serve as an ontology or completeness gate. No live Delvehold acceptance run is
admitted before the focused suite passes and the old writers are structurally
unable to override the aggregate.

### 9. Interruption — landed

Objective: a subject whose world moved between forming an intent and
committing it is interrupted, not silently refused, and the interruption is
narrated from what it could perceive.

Landed mechanism: a proposal binds to the scope digest of the components its
verification reads; when another commit moves that digest before the
proposal lands, the kernel refuses it with `ScopeChanged`. The narrative
lane's own early scope checks are deleted — the kernel's `exact_opportunity`
is the sole detector for the narrative lane, and `ScopeChanged` on submit is
the one event with one handler, `submit_narrative`'s `interrupted` arm. The
Persona is never re-run: on `ScopeChanged` the runner re-lowers the same
prose once through the Interpreter against a fresh opportunity
(`select_fresh`/`select_scope`), with a second input — an `Interruption`
carrying the fresh `ScopeComponents`, the `Overheard` rows later than the
turn's bound revision, and the discarded first-lowering evidence. The
re-lowered turn is `PersonaTurn::record`ed fresh, never mutated, and carries
`interrupted_from` pointing at the binding it replaced; a turn whose binding
already carries `interrupted_from` is not re-lowered again — it ends as
`NarrativeRun::Interrupted`, structurally, because no progression arm gives a
re-lowered binding a second successor. A fresh opportunity whose granted set
has lost `Speak` also ends the turn as `Interrupted` rather than spending an
Interpreter round on an invocation it cannot express. Interruption is
defined by the digest, so a neighbour's act, a clock tick that rolls a
routine's `due`, an elaborator patch, a consumer document, and a world-scale
event are one case, indistinguishable at the handler. Schema bumps:
`controller_work.v11` and `ghostlight.persona_turn_receipt.v3`; prior rows
are refused, not migrated. Eve reports the interruption through
`ControllerHttpResult::Interrupted`, which renders as `denied` in the
command result today; the receipt text carries the subject, both scope
digests, the persona prose and receipt digest, and the untranslatable gap.

Cut line held: no new kernel arm, bundle, or joint command; no second Persona
turn; no event log reaches a controller; the delta shown to the Interpreter
is exactly what the membrane already lets the subject perceive
(`Overheard` is built only from `Told` rows with a resolved speaker label,
never a raw knowledge row). The joint-per-room Interpreter was considered and
rejected: the room is not the unit of interruption, the digest is.

What is undone: no road run has exercised the re-lowering against a live
provider; none exists yet. Closed since landing: fork D's branch was dead by
construction (grants are insert-only, so a fresh opportunity always carries
speech) and is deleted; transfer, route-closing, and grant-revocation causes
are proven anonymous by Soul; a `run_cover_tick` end-to-end carries an
interruption through a real tick and shows the overtaken turn committing
nothing; a witness landing mid-turn yields the anonymous knowledge line and
no overheard row. Still open: the elaborator-patch cause is proven only by
the shared code path, not by a fixture that drives the elaborator past its
answer gate; and whether Eve's command-result vocabulary needs a fourth
state for an overtaken turn is the Eve owner's question.

### 10. Witnessed events over a place subtree — landed

Objective: stories that carry regional and global effects. An asteroid is
witnessed by everyone under a region's root; the moon going dark by everyone
under a hemisphere's; a subject standing in a village under both sees both.

Landed mechanism: `ComponentOp::Witness { fact, place, confidence }` lands
`Witnessed` knowledge of the fact on every subject positioned anywhere under
the place. Recipients are never stored; `ResolvedOp::Witness`'s apply arm
derives them at apply time through `under_place`, the same subtree walk
`audience`'s `Reach::Place` arm now calls, so a declared broadcast area and a
witnessed event share one definition of "under a place". Whoever already
holds the fact is dropped through `unheld`, factored out of `fan_out`'s old
inlined filter so `Communicate` and `Witness` cannot disagree about who
already knows. A telling still occurs into an empty room, so `Communicate`'s
empty fan-out stays a legal no-op; a witnessed event is only its reception,
so a `Witness` over an empty subtree, or one where every standing subject
already holds the fact, is refused as `Mismatch::NoOperationEffect` at
resolve (over the candidate graph, including this patch's own relocations)
and `KernelError::Invariant("a witness reaches nobody who does not already
hold the fact")` at apply — one rule, both layers. Confinement runs through
one new `operation_ground` arm returning the named place as the whole
ground: an elaborator may witness only under its own jurisdiction root, and a
`PatchGround::Consumer` patch can never witness, because its ground names no
place. Every recipient's scope digest moves and nobody else's does, which is
what makes a regional or global event interrupt whoever was mid-thought
(step 9).

Beyond the operation itself, an affordance slot,
`ComponentOpKind::Witness { confidence }` with roles `[fact, place]` and no
subject role, was not anticipated by this plan's text but was needed to let
a world-authored affordance (a bell, a beacon) cause a witnessed event
without a proposer ever naming a speaker; the source is `Witnessed` by
construction, since `ComponentOp::Witness` has no field a teller could enter.
One `PATCH_TOOLS` entry, `witness`, rounds out the model-facing surface.
Both store schemas moved to `ghostlight.world_state.consumer.v2` and
`ghostlight.world_commit.consumer.v2`; a store written under the prior schema
is refused, not migrated.

Cut line held: one fan-out owner (`under_place`), shared by `audience` and
`Witness`; one never-overwrite owner (`unheld`), shared by `fan_out` and
`Witness`; no stored recipient list; a witness names no speaker, so `Told`
stays unrepresentable outside `Communicate`; no new `Mismatch`,
`KernelError` variant, `CommandBody`, `SystemCapability`, or
`CompositeShape`; `verify_state_shape`, `can_broadcast`, and
`candidate_in_audience` untouched.

What is undone: the affordance-declaration refusal for a slot whose second
role binds a subject is proved directly through `arity` and
`role_kind_fits`, not by submitting a malformed `AffordanceDeclaration`
through admission — the gate is the one gate either way, but the submission
path itself is untested. The grouped lane's cost is untouched: a
region-wide or global witness still discards a grouped cell's in-flight work
for every constituent it touches, which is the debt this pass explicitly
declined to pay.

### 11. Claude SDK inference port — landed

Objective: run Ghostlight's inference lanes on the Claude subscription while
no Codex subscription or API budget exists, without moving the harness.

Landed mechanism: two `InferencePort` implementors behind one
`RoutedInferencePort`, chosen per lane by the configured model's prefix
(`GHOSTLIGHT_SDK_MODEL_PREFIX`, default `claude`) — a `claude`-prefixed model
reaches `SdkInferencePort`, anything else reaches the existing
`CodexConnectorInferencePort` unchanged. `SdkInferencePort` spawns a Node
sidecar (`sidecar/claude-sdk/`, a TypeScript package on
`@anthropic-ai/claude-agent-sdk` `0.3.261`, `@msgpack/msgpack` `3.1.3`, and
`zod` `4.5.4`, all exact-pinned) as a child process over stdio, framed as a
4-byte big-endian length prefix plus `rmp_serde::to_vec_named` bytes. One
request is one SDK query with the system prompt replaced, built-ins
stripped, settings sources empty, the request's tools registered as
in-process SDK tools, a derived turn cap, and the request's model.
Execute-through: each tool call the model makes crosses the pipe and is
answered by `ToolResultOracle`, one trait with four implementors
(`InterpreterOracle`, `OperationalOracle`, `GroupedOracle`,
`ElaborationOracle`) — each is the lane's own evaluator fold, extracted
whole into a free function (`interpreter_tool_result`,
`operational_tool_result`, `grouped_tool_result`, and the already-factored
`apply_tool_call`) and replayed over `completed` at construction, so the
string handed to the model is the same one re-derivation will recompute.
The sidecar computes no tool result itself.

One free function, `prepare_invocation`, is the sole owner of prepared
identity for both ports; `ControllerRunner::open` is injection-only —
`with_test_ports` is gone, and `open` takes `Arc<dyn InferencePort>`,
`Arc<dyn ControllerWorkStore>`, and `ControllerModels` and nothing
transport-specific. `runtime::open_controller` builds `ConnectorBinding`
and `SdkBinding` from the environment and calls `open_inference`, then
`open_controller_work`, then `ControllerRunner::open`; `open_inference`
routes every one of the five `ControllerModels` at open time and refuses a
model no configured backend claims (`ControllerOpenError::UnroutableModel`)
or a sidecar entry that is not a file
(`ControllerOpenError::SdkSidecarMissing`) before the first tick, not after.

Environment: `GHOSTLIGHT_SDK_SIDECAR` (the built sidecar entry path; its
absence means no SDK binding, not a default path) and
`GHOSTLIGHT_SDK_MODEL_PREFIX` (default `claude`) are read once in
`runtime::open_controller`. `GHOSTLIGHT_CONTROLLER_CREDENTIAL` is no longer
unconditionally required — an SDK-only configuration reads no credential
path at all. `GHOSTLIGHT_WRITE_SIDECAR_FIXTURES` regenerates the checked-in
schema and frame fixtures the Rust and TypeScript sides pin against each
other; it is a fixture-authoring switch, not a runtime binding.

Cut line held: no second identity scheme (`prepare_invocation` is the one
owner, both ports call it); no daemon, network port, or CultMesh surface for
the sidecar (it is a plain child process, restarted on fault, holding no
query state between queries); no credential handling in Ghostlight (the
sidecar inherits the ambient Claude Code login and this repo reads, copies,
forwards, or logs none of it); the connector port and its `execute` are
untouched; the port is chosen per lane by the configured model, with no mode
flag. Two forks the original design did not anticipate: `ControllerRunner::open`
became injection-only rather than keeping a `#[cfg(test)]` twin, because
removing the two transport-specific parameters made the twin's only purpose
vanish; and the turn cap rides on the oracle (`remaining_rounds`) rather than
on `InferencePurpose`, because the elaboration and seed lanes share one
purpose but carry different round budgets.

Named liabilities, stated where `SdkInferencePort` is defined, not hidden in
the lowering:

- The receipt is the SDK's message, not wire bytes. Its identity half is
  computed from the invocation Ghostlight already holds; its provenance half
  is a session id and message uuids reported by a child process this port
  spawned. It attests that this exact request produced this exact SDK
  session; it does not attest against a party Ghostlight does not control,
  which is what the connector's receipt does.
- A prior round's conversation reaches the model as prose, not as typed
  turns. The SDK owns the assistant side of its own transcript, so typed
  `tool_call`/`tool_result` turns from an earlier round cannot be replayed
  into it; they are rendered as strings under one fixed header and appended
  to the prompt.
- The Zod validation of a tool call's arguments happens in the sidecar
  before the handler runs, so a model's undecodable arguments never reach
  Rust as an argument string the oracle can fold over; the sidecar reports
  a normal dispatched tool call and Rust's own evaluator recomputes the
  decode failure as an ordinary gap — on the connector path the same
  failure is a gap recorded by the evaluator directly. Both are gaps; the
  divergence in exactly which layer detects an undecodable argument can
  only reach a later round's prompt through the prose transcript.
- `error_max_turns` handling is inferred from the SDK's own documentation,
  not observed: the sidecar is written to catch the throw the TypeScript
  SDK performs after yielding the error result, assemble the events already
  collected, and emit a normal `Output` whose receipt records
  `subtype: "error_max_turns"`, so the lane sees one ordinary round and
  proceeds rather than faulting. No live query has exercised this path.
- Because the turn cap is the lane's *remaining* round budget and the lane
  still counts rounds even when the cap is not reached, a lane whose model
  never terminates in one turn can spend a triangular number of turns across
  a round sequence (seed: 24+23+…, bounded by `SEED_ROUND_BUDGET`) against
  the connector's flat per-round cost. This is unmeasured: no live run has
  produced a number to weigh it against.

What is undone: no live run has exercised the SDK port at all — this
machine has neither a Claude Code CLI on PATH nor a stored credential, so
the ignored ad-hoc smoke (`notes/local-live-smoke.md`) has never been run
against it. Everything above the ignored smoke is proven by scripted-link
unit tests and the checked-in schema/frame fixture pairs, not by a real
sidecar process talking to a real subscription.

Exit condition, unchanged from the port's own doc comment: when an
`ANTHROPIC_API_KEY` and a budget exist, a Messages-API port is a closer
structural match to `InferencePort` than the SDK is — inert `tool_use`
blocks, a caller-appended `tool_result`, a real request id, real
concurrency, no subprocess — and `SdkInferencePort` is deleted rather than
extended, keeping `RoutedInferencePort` and swapping what it routes to.
Rejected: a backend inside CodexConnector (a deliberately isolated Codex
fork).

One validator on both transports: the sidecar's schema conversion is loose
(every property optional, unknown keys kept), so the arguments the model
emitted reach Rust's decoder unstripped and a refused argument is the gap
the evaluator already records, on the SDK path exactly as on the connector
path. Residual: a property whose value has the wrong type reaches Rust as
absent rather than as the wrong value, because the SDK derives the
advertised tool schema from the same object and refuses a dynamic
pass-through; the decoder reports it as missing. The result-subtype mapping
(`error_max_turns` is an output; every error subtype a named fault) is
tested in the sidecar without a credential.

Road follow-ups, 2026-09-10, from the first two SDK-backed seeded runs
(`notes/local-live-smoke.md`, "Claude SDK route"): the sidecar's schema
conversion now carries every field `description` through to the advertised
tool schema (it dropped all of them, so the `claude` lane saw bare types);
`due` describes itself as a clock reading and both authoring prompts print
the clock, because the model wrote twelve commitments due at or before a
clock it was never shown; `declare_subject` states the grant rule at the
point of use; `apply_tool_call` logs each raw tool call at debug so a refused
patch is diagnosed from what the model sent. A kernel refusal now continues
the same conversation instead of opening a fresh draft: `Refusal { after_round,
mismatches }` rows on both authoring checkpoints (`controller_work.v12`;
prior rows refused, not migrated), the refusal rendered as the next user turn
after the rounds that earned it, and the draft rebuilt by one `DraftFold`
shared by the evaluator and the oracle from the calls after it. The harness
takes the connector binding only when `GHOSTLIGHT_CONTROLLER_CONNECTOR` is
set, so an all-`claude` run needs no connector.

From run 2: the Interpreter's `speak` and `record_gap` cite the source by
quoting it (`source_quote`), and the harness locates the quote and derives
the span; the model no longer counts bytes, which cut three of eight
utterances mid-word. The same invariant holds (only words in the preserved
prose become speech; a quote not in the prose is a gap), and a quote that is
not canonical utterance text is a gap at capture rather than a quarantine at
invocation. `RecordGapToolCall` in `ghostlight-persona-projection` changed
shape; Epiphany pins that crate by revision and is unaffected until it moves.

From runs 4 to 6 (the membrane traced end to end): the Projector context now
carries the acting subject's own life by label (place, routes, holdings,
dependencies, authority shape, offices, redress, promises with counterparty,
pressure on self; due as minutes from now), never an id; the Projector and
Persona prompts render only what the context establishes and treat the word
budget as a ceiling; the smoke's `GHOSTLIGHT_SMOKE_TRACE` writes every
request and output. Silence is a legitimate outcome, not a harness failure.

### 12. The seed authors meaning — landed

Landed 2026-09-11 as mapped, with both recommendations taken: the small typed
material record, and a required commitment statement. `PersonaMaterial` is
`world/patch.rs`'s thirteenth-row component (`values`, `voice`, `memories`,
`reads` keyed by subject), set whole by `set_persona_material` (thirty
operations, thirty-nine tools); a commitment carries `statement`; state and
commit schemas are `.consumer.v3`; an affordance-made promise's statement is
"a promise made through <kind>". The Projector context carries the material
by label, each promise's statement, and `present`, the labels of the subjects
standing in the same place; the Persona's "Who you are" is the label, the
voice, and the values; the typed view carries material and statements by id.
Material is not in `ScopeComponents`, pinned by a test that sets it and
watches the scope digest stay. The map that led here follows. Objective:
a seeded person speaks from authored
meaning rather than the model's default voice, which is the hard boundary
"character intelligence is modeled state, not a default model capability".

Current mechanism: the ontology doc names `PersonaMaterial` (values, voice,
memories, reads) as a component and lists `set(subject, material)` in its
operation table, and none of it exists in the kernel; `Commitment` is kind,
counterparty, due, period, checks, with no statement of what is promised.
The seed can only author identical people with contentless debts.

Cut: (1) `create_commitment` gains a required `statement: Statement`; the
catalog count stays 29. (2) `set_persona_material(subject, material)`, a
whole-value set carrying `values: Vec<Statement>`, `voice: Statement`,
`memories: Vec<Statement>`, `reads: Vec<(SubjectId, Statement)>`; the count
becomes 30 and one tool follows from the catalog. Material is not in
`ScopeComponents` (no precondition reads it), so it cannot interrupt a bound
turn. The Projector gains memories and reads by label and the statement
beside each promise; the Persona identity block becomes label, voice, values;
the Operational typed view gains the same by id. The seed brief asks for a
voice, values, memories, and a read of each counterparty per person.
Forbidden: any `persona_state.v0` document inside `WorldState` (the portable
standard is a boundary projection, a later seam); any prompt-side invention.
Schema bumps: `world_state.consumer.v3`, `world_commit.consumer.v3`. Also
owed from run 6: the Projector says "no one before me" for two people in one
house, because the context carries own place and not who else stands there;
co-presence by label is the same reach the kernel already derives for speech
and belongs in the same cut. Decisions the operator owns: the small typed
material record above versus the full portable standard (recommend small),
and whether a commitment statement is required (recommend required).

### 13. The world brief — landed

Landed 2026-09-11. Objective: the people of a world know what world they are
in. `world_create.v3` carries a required `brief`, trimmed at ingress and
stored on `WorldState`; it is projected as the guidance of every lane and
printed in both authoring prompts, and the two narrative stages that rebuild
prompts on resume carry it as `guidance` (`controller_work.v13`). The smoke
titles its world by the seed root label unless `GHOSTLIGHT_SMOKE_WORLD_TITLE`
says otherwise, and the seed brief becomes the world's brief rather than a
per-session sentence. Run 9 is the proof: no fixture name in any vow, and
promises made against the deadline the brief set.

### 14. Nonverbal display by quote — mapped

Ordered 2026-09-11 after run 9. Objective: a person can act without words and
be seen doing it. Run 9's gap column holds "I nod toward the path down", "I
set down whatever's in my hands", "I settle on the warm step", and "I let the
quiet sit a breath longer"; each was seen by everyone in the room and the
world lost it. The false binary is intent versus posture. Intent is a psychic
side channel; posture mechanics force every reader to decode a body. What is
public is neither: it is the display, a visible act at the grain a witness
perceives it, separable from what the actor meant by it. Speech already has
this shape in the kernel: a claim placed before an audience, landed in each
listener's knowledge as `Told { by }`, with the words kept verbatim. A display
is the same act with a different sense: `Seen { by }`, and an audience fixed
by physics rather than by the entry.

Current mechanism: the narrative lane speaks and does nothing else.
`speak_invocation` (`controllers.rs`) finds the granted entry of kind `speak`
and invokes it with `speech: Some(statement)`; `exercise` (`action.rs`)
lowers speech kernel-side into `AssertClaim { fact, statement, by: actor }`
and `Communicate { speaker, fact, to }`, the fact id from the one speech
`derive_id` site with `SPEECH_INDEX = "0"`; `Communicate`'s apply arm fans out
over the entry's audience less the speaker less holders and lands
`Told { by, via }` at `Believed`. `DecisionEvent.speech: Option<EntityId>`
stamps `spoken_at` on every knowledge row at snapshot; `overheard_since`
turns rows newer than a bound turn into the Interpreter's "What was said to
this person since" block; `projector_knowledge` renders each row with a
speaker label for `Told` and none for `Witnessed`/`Evidenced`. The
Interpreter owns two tools, `speak(source_quote)` and `record_gap`; a visible
act is a `missing_affordance` gap, and the fold allows one speech capture.
Interpretation of a display already has a home on the receiving side:
`PersonaMaterial.reads` (Torvin reading Iska's face) is the receiver's own
private reading, rendered only to the receiver. What has no home is the
display itself on the sender's side.

Authority map:

- Owner: the kernel's `exercise` lowering in `action.rs`. It alone turns an
  invocation's `display: Option<Statement>` into
  `AssertClaim { fact, statement, by: actor }` plus
  `ResolvedOp::Display { actor, fact }`, the fact id from the same
  `derive_id` site with a second discriminator (`DISPLAY_INDEX = "1"`; the
  index exists for exactly this, no new allocation site).
- Inputs: the actor, its position, the quoted visible sentence. Nothing
  else. Not the entry's audience: a display cannot go down a horn.
- Outputs: one `Claimed { by: actor }` fact whose statement is the actor's
  own visible sentence, and a `Seen { by: actor }` knowledge row at
  `Believed` on every subject standing in the actor's place, less the actor,
  less anyone who already holds the fact (`unheld`, the never-overwrite rule).
  Fan-out is `audience(state, actor, &Audience::Colocated)`, the derivation
  speech already uses; `can_broadcast` over the same audience refuses a
  placeless actor as it refuses a placeless speaker. A display into an empty
  room is a legal no-op, as speaking alone is.
- Derived state: each recipient's `knows` moves, so its scope digest moves,
  so a display interrupts whoever was mid-thought in the room through the
  interruption mechanism of step 9 unchanged. `spoken_at` becomes
  `minted_at`, stamped from `DecisionEvent.speech` or `.display`, and is the
  sole gate on an overheard row. `Overheard` carries how the row arrived:
  said by, shown by, or known; the Interpreter's interruption block renders
  "X said" and "X, seen doing" and the existing "came to know" line. The
  Projector's knowledge rows render `Seen { by }` with the actor's label and a
  seen marker; the reader's Persona does the social reading itself, in its
  own values, and any reading it commits is a `PersonaRead`, already landed.
- Forbidden writers: no `intent`, `meaning`, `mood`, or `reading` field on
  the operation, the fact, the knowledge row, or the event; the kernel never
  mints a reading for a receiver. The Interpreter never rewrites the display
  into third person or into what it meant; the statement is a verbatim quote
  of the visible clause and a quote not in the prose is a gap. The Projector
  never paraphrases a display. No confidence on the operation: a displayer
  cannot choose how much a viewer believes its body. No display over a
  channel audience, ever; no `Display` in `PATCH_TOOLS`, so no seed or
  elaborator may make a person gesture from outside that person (kernel-only
  lowering, the same standing as `AssertClaim`; the catalog stays thirty
  operations and the pin stays `(7, 30, 39)`).
- Shared paths: the SDK lane and the connector lane share the one
  Interpreter fold; replay re-derives the display fact from the command's
  invocation and must land the same rows; a resumed narrative row rebuilds
  the same interruption block from the persisted `Overheard`.
- Deletion line: `InterpreterFold.captured_speech: bool` and
  `SpeakProposal { text }` are cut for one address proposal carrying
  `speech: Option<String>` and `display: Option<String>`, at most one of each
  per turn, and `NarrativeCapture.proposal: Option<SourceRange>` becomes the
  ranges of both. `ActionMismatch::SpeechRequired` narrows to "a
  speech-carrying entry invoked with neither speech nor display"; a silent
  turn that only displays invokes the `speak` entry with `speech: None`.
  `carries_speech` keeps its name and its meaning: what it gates is the
  utterance and its audience; a display accompanies any invocation because a
  body is visible whatever it is doing.

Interpreter contract: a third tool, `display(source_quote)`, beside `speak`.
The prompt gains one rule and no list: a visible act another person in the
same place could see (a gesture, a posture, a look, a movement, a silence
held) is a display, quoted as the visible clause alone; what the person
hoped, meant, or felt while doing it is not visible and is not quotable as a
display. A non-canonical quote is a gap; a second display is an `ambiguity`
gap, the same rule as a second speech.

Cut line: the `missing_affordance` gaps for gestures in run 9 are the
measured outcome. Run 10 tallies the gap kinds again; the four gesture gaps
must be displays and the private-reasoning gaps ("I don't say the other
thing") must remain gaps, because they are intent and this cut gives intent
no public home on purpose. Memory formation from a turn's private half is a
different cut and is not in this one.

Subtraction budget: net additive by one `ResolvedOp` arm, one
`KnowledgeSource` arm, one invocation field, one event field, one Interpreter
tool, and one `Overheard` manner; cut against it are `captured_speech`,
`SpeakProposal`, and the four `missing_affordance` gap shapes per run. The
smaller surface, routing gestures through `speak`, was rejected: it would
land a visible act as `Told`, a telling that did not happen, and render it to
the room as words. Schema bumps: `world_state.consumer.v4` (the knowledge
source arm and `minted_at`), `world_commit.consumer.v4` (`display` on the
invocation and the event), `controller_work.v14` (`Overheard` manner and the
capture ranges); the consumer API doc and the ontology's speech paragraph,
operation count sentence (line 459 still says twenty-nine; it is thirty and
stays thirty), and Interpreter paragraph follow.

Build budget: crate `ghostlight-dungeon` and `ghostlight-persona-projection`
tests on the Windows workstation as the proving host, sidecar schema
fixtures regenerated if the Interpreter tool set changes the sidecar's
fixture; the Linux release is Idunn's build and is not touched by this cut.

Verification: kernel tests that a display lands `Seen { by }` on the
co-located only, never on the actor, never on a subject elsewhere or on a
channel audience, never over a holder, is refused for a placeless actor, is a
no-op in an empty room, and replays to equal state; an interruption test that
a display after a neighbour's turn bound yields a "seen doing" overheard row
and the "knows changed" line; fold tests that a non-canonical display quote
is a gap, a second display is an ambiguity gap, and speech plus display in
one turn is one invocation carrying both; a Projector test that a seen row
renders with the actor's label and never as speech.

Decisions the operator owns: kernel-only lowering versus a patch-authorable
`display` operation (recommend kernel-only; a person's body is not
authorable from outside the person, and the count stays thirty); the display
statement kept in the actor's own words versus an Interpreter rewrite
(recommend the actor's words; by-quote discipline, and the receiver reads
first person as a stage direction); a silent turn invoking the `speak` entry
versus a fourth narrative tool for a wordless turn (recommend `speak` with
no utterance; one carrier, one command per turn).

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
