# Ghostlight World Ontology

## Status

This document is the closed typed vocabulary for world structure, character
action, and elaboration, and the deterministic verification that admits all
three. `docs/architecture/ghostlight-dungeon-mvp.md` remains the authority map:
it owns who may write, what the prime invariants are, and where the boundaries
sit. This document owns what the vocabulary *is* and what the reducer checks.
It may not introduce a writer, a semantic gate, or a second commit path.

`docs/architecture/ghostlight-transition-algebra.md` and
`docs/architecture/ghostlight-multiresolution-agency.md` describe the
pre-rebuild machine and are teardown evidence. Their surviving ideas are named
where they are used: one mutation vocabulary across every admission
lane; compact mutation drafts with a complete deterministic mismatch set; and a
resumable per-jurisdiction elaborator session checkpointed against admitted
commit ancestry.

## Objective

One vocabulary and one deterministic verifier for three things that used to be
three machines:

- **world elaboration** — adding structure at a causal boundary;
- **character action** — a subject changing the world through an affordance;
- **institutional flow** — obligations, pressures, and authority propagating
  through layers of collective agency without a narrator.

Verification is structural. The reducer checks reference integrity,
preconditions, authority scope, effect ceilings, conservation, and knowledge
scope. It never checks whether the result is interesting, plausible, diverse,
or well written. Depth comes from typed structure that constrains decisions,
not from a model agreeing that the world is deep.

## Current mechanism

Passes 1 through 10 of the widening are landed (`5e53beb`, `e99af63`,
`d2805fe`, `0f21a49`, `dbe176d`, `cb6a126`, `1852ddd`, `3ed3868`,
`8aebfb6`, `b4d1943`). The sealed kernel holds subjects with a label and a
kind (`Person | Institution | Population`); a `positions` partition with
`Position`; entities with `container` under containment acyclicity; `Route`
as an `EdgeRecord` variant carrying `AccessKind { Public, Restricted }`, a
minute `Cost` in `1..=525_600`, and an open flag; subject-keyed `holdings` of
`Quantity(u64)` per entity, absence meaning zero, checked arithmetic with
u128 ledger accumulators; subject-keyed `dependencies` on
`DependencyTarget { Resource, Route, Subject }`; and an `affordance_catalog`
partition of world-authored entries; `authority`, `selection`, and
`redress` partitions; `facts`, `channels`, and `knowledge` partitions;
`commitments`, `pressures`, and `last_opportunity_at` partitions; and the
scalars `now` and `scale_intent`. There is no relation yet.

`world/patch.rs` owns typed `SubjectId`/`EntityId`/`EdgeId`/`AffordanceId`
namespaces, `Ref<Id>` with `DraftHandle` (adjacently tagged `RefKind`), and a
closed `resolve_patch` that resolves declarations, then operations, and
returns the complete `Vec<Mismatch>` before any canonical ID allocates;
`derive_id` is deterministic over world, command, and handle.
`Declaration::Affordance` admits a catalog entry with preconditions drawn
only from `{ Present, Reachable, Holds }`, effect ceilings, and weighted
outcome bands. The ten inhabited operations are `Relocate`, `OpenRoute`,
`CloseRoute`, `AlterCost`, `Transfer`, `Transform` (one-to-one), `Consume`,
`Admit` (same-patch evidence), `Bind`, and `Release`; `apply_operations` is
the one owner of operation application and conservation, and
`patch::check_ledger` is the single named conservation check.
`CommandBody::AdmitPatch` emits `WorldEffect::PatchAdmitted` through the one
`admit_resolved` insertion owner shared with genesis; it has no phase gate of
its own, because in Active the answer rule below bounds what a patch may
declare. Before pass 10 the only production author of a `WorldPatch` is the
elaborator lane, which decodes items one at a time under its draft caps; the
kernel itself bounds a patch nowhere yet. `EvidenceRef::new` is `pub(super)`
and its production use is bounded by `filter_evidence`.

Character action is a precondition-effect transition. Grants are
`BTreeMap<DecisionScope, BTreeSet<AffordanceId>>`. `world/action.rs` owns
`exercise()`, called from `reduce` and `apply_effect`: it checks the grant,
evaluates each precondition against the actor's own components at the scope
digest, selects an outcome band from
`BandPreimage { world_id, revision, command_id, affordance, band_count }`
through `digest()` with no RNG, bounds the proposed effects by the band's
ceilings, and appends `DecisionEvent { band, effects }`. Rejections are the
complete `ActionMismatch` set (17 variants) under
`KernelError::ActionRejected`. `Speak` is a kernel-built entry with zero
preconditions and an empty band that carries speech, synthesized only at
genesis. A controller's tools, signatures, and permissions are derived from
its granted entries by `catalog_tools`, `catalog_signatures`, and
`catalog_permissions`; no hand-written permission surface remains.

Authority is a grant of `(kind, AuthorityTarget)` where the target is a
`Subject` or a `PlaceSubtree`; one predicate, `covers`, decides membership for
the `Authorized` and `HasStanding` preconditions, `route_admits` on
`AccessKind::Restricted { requires }`, the delegation monotonicity rule, and
the redress projection. Same-kind grants with overlapping targets are rejected
at admission (`targets_overlap` is structural: identity for subjects,
containment for places, plus a same-revision nesting check between the two
shapes) and `verify_state_shape` re-checks only the structural rule. An
`Office` joins an institution to a person; `open_office`, `close_office`,
`install`, and `vacate` are the selection operations, and an incumbent's
delegated grants are copied into its own `ScopeComponents`. A `Forum` admits
petitions from subjects with standing. Authority-writing slots (`grant`,
`revoke`) are gated by one rule: the actor's own authority must cover the
target, otherwise `ActionMismatch::DelegationNotMonotone`.

A `Fact` is an entity with a `Statement` and a `FactStanding`: `Canonical`
with evidence, or `Claimed { by }`. `Knowledge` is subject-keyed per fact with
a `Confidence` and a `KnowledgeSource { Witnessed, Told { by, via }, Evidenced }`;
a telling never overwrites a holder. A `Channel` is an entity with a
`Reach { Subjects, Place }` and a controller. Speech is an affordance whose
entry declares exactly one `Audience { Colocated, Channel }`: `exercise`
mints the `Claimed` fact through `derive_id` (the second of its two call
sites, the first being `resolve_patch`) and lowers a `Communicate` whose
recipients are re-derived by one pure `fan_out` over `audience` at apply
time. Audience means the declared reach; a channel's controller may broadcast
from outside it (`can_broadcast`) but receives nothing unless inside it.
Preconditions `Knows`, `CanBroadcast`, and `CanReach` gate at `exercise` with
`ActionMismatch::{FactUnknown, NoAudience, CannotReach}`. The statement lives
in `facts[fact].statement`; the copy inside the committed `AssertClaim` is
the replay witness and nothing reads it. `WorldSnapshot` carries no event
log; the story feed is `operator_log`, an owner-only projection that the
controller lane cannot name because `ControllerRunner` holds a
`ControllerPort`, not the mailbox. `WorldMailbox::create` declares the genesis
place `commons` and stands genesis subjects there.

Time is `now: FictionalMinutes`, moved only by `CommandBody::AdvanceTime`
from `CallerId::System(SystemCapability::Clock)`; the runtime tick submits
the constant `CLOCK_TICK_MINUTES`, never a measured duration. `world/clock.rs`
owns `derive_motion`, pure over state and tick: routines re-arm by exactly one
`period`, past-due obligations and goals write `Pressure` on their subject by
the `step` table, and an unavailable dependency does the same. A
`Commitment` is `(kind: Routine | Obligation | Goal, counterparty, due,
period, checks)`, created and discharged by two operations; `Pressure` is a
magnitude per source and target with `set_pressure` as its one writer, zero
spelled by key removal, and three operations (`advance`, `reduce`,
`resolve`). `order_opportunities` is total: pressure, then time since last
opportunity, then `SubjectId`. `derive_boundaries` is pure over state and
yields `CausalBoundary::{UnelaboratedDestination, MissingStructure}` with a
`BoundaryDigest` over structure only, never the clock; `IndividuationRequired`
and `PolityInCausalRange` are representable and not yet derived.
`derive_scale_deficit` reads the write-once `WorldScaleIntent` set at genesis
and counts every subject in exactly one jurisdiction row (`Uncovered` for a
subject under no root, placed or not). In Active, a patch
that declares or admits evidence must answer a derived boundary or a nonzero
deficit and satisfy it (`AnswerRequired`, `AnswerNotDerived`,
`AnswerNotSatisfied`); a component-only patch answers nothing; Draft answers
nothing.

`AdmitPatch` has two authors. The owner is unconfined. The elaborator is
`CallerId::System(SystemCapability::Elaborator { jurisdiction })`, a caller
identity whose only door is `WorldMailbox::submit_elaboration`; there is no
HTTP ingress for a patch before pass 10. `require_patch_author` and
`confine_to_jurisdiction` run in `reduce` and again in `apply_effect`: an
elaborator may answer only a boundary or deficit row under its root
(boundaries cover transitively, deficit rows by exact key, so a parent root
cannot answer a child row and double-count it), and may declare places,
relocate, or open routes only inside that root (`Mismatch::OutsideJurisdiction`,
decided by `operation_ground`, which is total over `ResolvedOp`). Placeless
referents (resources, catalog entries, facts) are not confined. The model's
tool surface is `PATCH_TOOLS`: one tool per non-genesis `Declaration` and
`ComponentOp` variant plus two session tools, emitted through
`world/tool_schema.rs` from one schema spelling per type.
`world/elaboration.rs` owns the repair loop: a session keyed by a
deterministic command id (sha256 over world, jurisdiction, answer digest)
submits a draft, persists the resolver's complete mismatch set under that id,
and re-prompts with it, bounded by `ELABORATION_ROUND_BUDGET` and the
`MAX_DRAFT_*` size caps. Evidence a model cites must come from the round's
`EvidenceSource` receipts (`filter_evidence`); with `NullEvidenceSource` no
canonical fact and no `Admit` can land from the elaborator lane.

`world/cover.rs` owns the budgeted connected cover. `derive_cover(world, now,
tick, opportunities, agency_graph, budget)` is pure: it reads the attention
order that `order_opportunities` owns, reserves the urgency slots for its
head, rotates the remaining subjects through singleton cells by debt, and
packs the rest into grouped cells by connected component of the agency graph
(scheduler-only; reachable from no prompt builder). `Cell`, `Cover`,
`CellId`, `Resolution`, `TickIndex`, and `AgencyGraph` are derived and
disposable; the only durable trace of a tick is the `controller_work.v9` row
(`ControllerWork::Grouped`), custody-separate from world custody. A grouped
cell is one inference over partitioned per-constituent views returning zero
or one attributed proposal per constituent; declines are submitted first and
each proposal commits or is refused on its own scope digest; a coarse
`NarrativePersona` receives its typed view and does not enter the membrane.
Cell and constituent command ids are sha256-derived. The runtime's
`drive_cover_tick` is the single owner of tick cadence, cover derivation,
the bounded concurrency permit pool, the quarantine flag, and the clock,
which advances after the cells so every cell in a tick shares one `now`.

`AdmitPatch` has a third author: `CallerId::System(SystemCapability::Consumer
{ consumer })`, minted only by `WorldMailbox::submit_consumer` after the
consumer ingress (`world/consumer.rs`) has authenticated a document against a
configured secret digest. `require_patch_author` and `confine_to_ground`
widen from jurisdiction-only to `PatchGround { Jurisdiction(JurisdictionKey),
Consumer(ConsumerId) }`; a consumer's ground is derived from
`controller_assignments` at decision time, never carried on the wire, and
confines it to the subjects it controls, with no place, route, or `Canonical`
fact ever inside that ground. A subject declared
`NewController::External { consumer }` receives
`ControllerAssignment::ExternallyControlled { consumer }`: it mints no
controller ID or mode (both accessors are `Option`), derives no opportunity,
is excluded from `agency_graph` and the cover, and may hold no affordance
(`admit_resolved`'s controller-shaped pairing rejects a non-empty grant set as
`Mismatch::ControllerGrantMismatch`). It remains an ordinary subject in the
snapshot, related to and targeted like any other, mutable only through its
own consumer and the always-unconfined world owner. `patch::decode_patch` is
the one decode bound for both the elaboration lane and the consumer lane,
under four caps — `MAX_PATCH_BYTES`, `MAX_PATCH_DECLARATIONS`,
`MAX_PATCH_OPERATIONS`, `MAX_PATCH_EVIDENCE` — and the consumer document
travels over two wire schema constants, `CONSUMER_PATCH_SCHEMA` and
`CONSUMER_RECEIPT_SCHEMA`, both `.v0`. The state and commit schemas bump to
`ghostlight.world_state.consumer.v1` and `ghostlight.world_commit.consumer.v1`
(state-schema generation `world-v3`); a store written under an earlier schema
is refused, not migrated.

`WorldScaleIntent` now arrives at creation, not as a later admission:
`world_create.v2` declares targets and jurisdiction roots top-level in the
genesis patch beside `commons`; a v1-announcing invocation is refused before
any handler runs, and the intent is write-once, set once at genesis and
resolved nowhere else. `qualifies` is phase-free — a controller, a non-empty
grant set, and a held `Goal`, checked the same way in Draft and Active — and
the Draft-answer refusal lives only in `require_answer`, which refuses every
Draft answer regardless of what `qualifies` says; so a Draft world's deficit
measures what the seed lane has left to author, not a frozen count. The seed
lane (`SeedPort`'s two methods `snapshot` and `submit_seed`, `SeedRunner`,
`SeedSession`, `ControllerWork::Seed`, `controller_work.v10`) admits seed
patches as `CallerId::Principal(owner)` through the existing unconfined owner
lane, in Draft only: the journal carries no marker distinguishing a
model-authored seed patch from a hand-authored one, because the model is the
owner's Hands during Draft and the owner's `ApproveDraft` plus `ActivateWorld`
remain the only path to Active. Evidence for a seed session comes from
`VaultEvidenceSource`, a read-only markdown directory reader in
`world/vault.rs`: the reference it hands back is the note's vault-relative
`.md` path, and its caps (`MAX_VAULT_RECEIPTS`, `MAX_HITS_PER_REFERENT`,
`MAX_LINK_FANOUT`, `MAX_EXCERPT_CHARS`) bound what one referent can retrieve.
One `world.seed` invocation runs one session and commits at most one patch
within the existing patch caps, driven by `select_row` choosing the first
`ScaleDeficitRow` with a nonzero deficit off the snapshot; `prompt_body`, the
evidence-and-mismatch tail of an authoring prompt, is shared by both the
elaboration lane and the seed lane so the two cannot drift on what a citation
rule says. One correction against the vocabulary above: a `Goal` commitment
carries no counterparty and by itself manufactures no `MissingStructure`
boundary; a commitment with a counterparty does, which is why a seeded
subject's bare goal is exactly the boundary the Active elaborator exists to
answer.

Opportunities bind to a `ScopeDigest` over one `scope_components` owner
(controller assignment, grants, delegated grants, own authority, position,
incident routes, own holdings, own dependencies, known fact ids, controlled
channels, own commitments; never `now`, occupancy, forum state, or another
subject's knowledge); a proposal commits at any later revision with unchanged scope and is
rejected with `KernelError::ScopeChanged` otherwise. State schema is
`ghostlight.world_state.consumer.v1`, commit schema
`ghostlight.world_commit.consumer.v1`, controller work `controller_work.v9`;
earlier stores are refused. Ghostlight owns a conserved narrative ledger;
Delvehold owns the economy (`delvehold-forced-ontology-integration.md`).
Step 6 of the plan is complete. The next seam is the seeded live run against a
real Vault and a real connector, then the outbound consumer response.

Rules the road imposed, carried in the tree: the provider request identity is
content-addressed over command, purpose, round, instructions, and input, so a
resumed round replays and a repair round under the same command names a new
request; every tool schema of both authoring lanes satisfies the provider's
strict function-schema rules (`anyOf`, typed tags, closed objects with every
property required) and a test walks them offline; the seed lane has its own
round budget, asks for several tool calls per response, and submits the draft
as authored when its budget ends; the seed brief prints every canonical id
beside its label and states that a subject counts only if it stands at the
row's root or inside it; the connector's expiry skew bounds an invocation's
validity and a separate response timeout bounds a generation.

## The failure this vocabulary is designed against

Run 115 terminated because `inst:kharad-road-keepers` referenced
`loc:kharad-rhythm-road`, which did not exist, and four reconciliation steps
could not repair it. Three structural causes, all in the vocabulary:

1. **Names were keys.** `SubjectRef { kind, id: String }` accepted a
   caller-supplied identifier. A plausible string was indistinguishable from a
   referent until reduction.
2. **Admission was two-phase.** `AdmitEntity { initial_components }` declared
   which components a subject *could* have separately from populating them. A
   referenceable, empty entity was a legal state.
3. **The batch was flat.** Each mutation was permitted independently, so a
   cross-mutation dangling reference was a runtime discovery. Reconciliation
   existed to repair that class, which made it a repair loop standing in for an
   owner.

Depth had the same defect. Texture lived in free strings — `posture`,
`resource_kind`, `capability_id`, `persistent_features`, `description` — that
nothing could reason over, so a world with many actors still felt thin. The
answer reached for was five model tribunals (`ElaboratorTitle` and the
`WorldComplexity*Qualification`/`*Verification` set): compensators for an
ontology with nowhere to put depth.

Character action had a matching defect: an attempt was an event, and a separate
wave-level outcome resolver asked a model whether it worked. Resources,
relationships, knowledge, and pressures mostly stood still while the prose
stayed busy.

The cut: make identity unforgeable, make admission atomic, give depth a typed
home, and make an action a typed precondition-effect transition that the same
reducer verifies. Then structural admission is sufficient and no verifier,
resolver, or reconciler is needed.

## Invariants

1. A reference is an exact canonical ID issued by the reducer or a draft handle
   resolved inside the same patch. There is no third form.
2. Cross-kind reference is unrepresentable. Typed ID newtypes carry the kind.
3. A patch reduces completely or not at all. No canonical ID is issued,
   revealed, or consumed by a rejected patch.
4. Only subjects decide. Places, resources, facts, and channels have no
   controller, no affordance, and no decision opportunity.
5. Custody conserves. Quantity moves or transforms; it is created only by an
   explicit evidenced admission or an affordance whose effect schema says so.
6. Knowledge is scoped. A subject acts only on facts it holds. A patch adds
   knowledge only by citing an accessible fact or admitting an evidenced one.
7. An action is an affordance invocation. Its preconditions are checked
   against the acting subject's own components at the exact scope digest; its
   effects cannot exceed the affordance's effect ceiling; an uncertain outcome
   is selected by kernel-owned entropy from the affordance's declared bands.
8. Persona material enriches and never substitutes. Values, voice, memory, and
   relationship reads cannot stand in for authority, custody, topology, or
   knowledge.
9. Elaboration answers a derived causal boundary, an authored seed request,
   or an authored scale deficit. Structural validity never waits on a count;
   liveness is an authored target the elaborators pursue. There is no
   semantic qualification, round budget, or model-owned completeness verdict.
10. A proposal binds to the digest of the components its verification reads,
    not to the whole world digest. It commits at any later revision where that
    scope digest is unchanged, and is rejected when it is not.
11. Every tool a model may call is a projection of this vocabulary. The tool
    catalog is derived state and cannot carry an operation the reducer does not
    own. The expansion rule: the tool surface is exactly the variants a
    non-genesis patch may carry, with every always-refused choice removed.

## Identity

Three namespaces, three newtypes, issued only by the reducer.

```text
SubjectId   person, institution, population                    — can decide
EntityId    place, resource, fact, channel                     — cannot decide
EdgeId      route, relation, commitment, pressure              — connects the above
```

Display names are labels. They never resolve a reference or bind structure.
An externally controlled mirror is an ordinary subject of one of these three
kinds; what is external is its `ControllerAssignment`
(`ExternallyControlled { consumer }`), not a fourth `SubjectKind`.

## Components

Twelve, each earning its place by constraining a decision. Some attach to one
referent; some are edge-shaped and attach to a pair. The reducer does not care
which noun a component is; it cares what it constrains.

| Component | Attaches to | Decision it constrains |
| --- | --- | --- |
| `Position` | subject → place | who is present, who perceives, who must travel |
| `Route` | place ↔ place: access, cost | who can arrive in time, who can be cut off |
| `Custody` | holder → resource: quantity | what can be spent, seized, withheld |
| `Dependency` | subject → resource, route, or subject | what fails when supply fails |
| `Authority` | subject → scope: kind | who may legitimately command, tax, judge, admit |
| `Selection` | office → method, incumbent, term | how power is lost; succession pressure |
| `Redress` | grievance kind → forum, standing | where conflict goes when it cannot be fought |
| `Knowledge` | subject → fact: confidence, source | what a subject may act on at all |
| `Channel` | channel: reach set, controller (latency arrives with the clock, pass 7) | how facts travel; who can be silenced |
| `Commitment` | subject → counterparty: kind (`Routine`, `Obligation`, `Goal`), due, period | obligation with a clock; autonomous motion |
| `Pressure` | source → target: magnitude | the causal boundary trigger |
| `PersonaMaterial` | subject: values, voice, memories, reads | lived meaning; never authority |

A `Fact` carries a `standing`: `Canonical` (admitted with evidence) or
`Claimed { by: SubjectId }` (asserted in speech). Knowledge of a claimed fact is
belief. Deception is a subject asserting a claim while holding a contradicting
canonical fact; misread is a listener acquiring a claim the speaker did not
intend. Both are ordinary typed states, not narrator judgments.

Civic order is the `Authority` + `Selection` + `Redress` + `Custody` subgraph.
It is not a prose manifest and needs no civic verdict. `Dependency` is the
crisis transmitter: it is what makes a failed pump in one realm a political
problem in another without anyone deciding that it should be.

## Component operations

This is the closed operation set. Every mutation the kernel will ever apply is
one of these, and every model-facing tool is a projection of one of these.

```text
Declaration      declare_subject, declare_place, declare_resource, declare_fact,
                 declare_channel, declare_route             (draft handles only)
Position         relocate(subject, via: Route)
Route            open, close, alter_cost
Custody          transfer(from, to, resource, qty), transform(resource, into, qty),
                 consume(holder, resource, qty), admit(holder, resource, qty, evidence)
Dependency       bind, release
Authority        grant(subject, scope, kind), revoke
Selection        open_office, close_office, install(office, incumbent), vacate
Redress          open_forum(kind, forum, standing), close_forum
Knowledge        acquire(subject, fact, source, confidence), communicate(speaker, fact,
                 channel), forget
Channel          set_reach, set_controller
Commitment       create(subject, counterparty, kind, due, period, checks), discharge
Pressure         advance, reduce, resolve
PersonaMaterial  set(subject, material)
Retirement       retire(referent, reason)
Time             advance(minutes)
```

Twenty-seven operations. Adding one is a design change to this document and a
code change to the reducer; it is never a runtime patch.

## Patch construction

One primitive admits structure and effects, in draft and in active play:

```text
WorldPatch
  declarations: Vec<Declaration>       // new referents, draft handles only
  operations:   Vec<ComponentOp>       // over declared or existing referents
  evidence:     Vec<EvidenceRef>       // exact receipts where a fact is admitted

Ref<Id> = Existing(Id) | Draft(DraftHandle)
```

Reduction is one pass with no repair step:

1. Index every declaration by handle. Duplicate or unused handles reject.
2. Resolve every `Ref::Draft` against that index, checking kind.
3. Resolve every `Ref::Existing` against the current revision, checking kind.
4. Check structural admission over the complete candidate graph.
5. Only then allocate canonical IDs and commit atomically.

A rejection returns the complete deterministic mismatch set — every failed
check, not the first — so a proposer repairs one compact draft without
guessing which invariant failed first. `inst:kharad-road-keepers` naming an
undeclared road is rejected at step 2, before any ID exists. There is nothing
for a reconciler to do, so there is no reconciler.

### Structural admission

The reducer checks only these:

- every referent exists or is declared in the same patch;
- containment is acyclic;
- routes have exact place endpoints and a valid cost;
- edge endpoint kinds match the edge kind;
- custody operations conserve declared quantity;
- the deriving authority envelope covers every operation in the patch;
- knowledge additions cite an accessible or newly evidenced fact;
- communication reaches only subjects inside the channel's reach set;
- population slices are disjoint beneath their declared parent;
- an externally controlled subject's components change only through its own
  consumer, confined by `PatchGround::Consumer`;
- the patch produces at least one canonical change.

Interestingness, novelty, political diversity, name quality, prose quality, and
counts do not appear here and may not be added.

## Affordances: the action vocabulary a world authors

The kernel owns component operations. A **world** owns its affordance catalog.
This is the generality lever: a setting with spellcraft, insurance claims,
oath-rhythms, or memetic sovereignty authors affordances from the same
twenty-seven operations, and the kernel never learns a genre.

```text
Affordance
  kind:          AffordanceKind        // world-declared name, e.g. Speak, Move, Transfer,
                                       //   Decree, Levy, Petition, Cast, Sabotage
  preconditions: Vec<Precondition>     // over the actor's own components at scope digest
  effect_schema: Vec<EffectSlot>       // which ComponentOps, on which referent roles,
                                       //   within which bounds
  outcome_bands: Vec<OutcomeBand>      // weighted; each band names its effect subset
```

```text
Precondition
  Present { at: Role }                          // actor Position matches target place
  Reachable { to: Role, within: Cost }          // a Route path exists under access and cost
  Holds { resource: Role, at_least: Qty }       // Custody
  Authorized { over: Role, kind }               // Authority covers the target scope
  HasStanding { forum: Role }                   // Redress: the forum admits the actor
  Knows { fact: Role, at_least: Confidence }    // Knowledge
  CanBroadcast { via: Audience }                // the actor is inside the audience
  CanReach { subject: Role, via: Audience }     // the subject is inside the audience
  Committed { to: Role, kind }                  // an existing Commitment
```

Preconditions read only the acting subject's own components and the exact
targets the proposal names. A subject cannot act at a distance, spend what it
does not hold, command outside its authority, act on a fact it does not know,
or speak to someone its channels do not reach. These are checks on typed
state, not a model's opinion of feasibility.

Effect slots bound what the invocation may propose: which operations, on which
roles, with what magnitude ceiling. A proposal exceeding a slot is rejected.
A proposal within its slots is admitted structurally like any patch.

Outcome bands make uncertainty deterministic. When an affordance declares more
than one band, the kernel draws from its own committed entropy — seeded by
world ID, revision, and command ID — selects one band, and applies only that
band's effect subset. The same proposal at the same revision always yields the
same band. Neither the Persona, the Interpreter, nor an operational agent can
choose success; they choose the attempt.

`Speak` is the one kernel-built affordance: precondition `CanReach`, effect
`Knowledge.communicate` of a `Claimed` fact to every subject in reach, plus the
speech event. Everything else is authored per world in the seed and may be
extended by elaboration under the same patch primitive.

## Action verification

```text
controller proposal (typed, from Interpreter or OperationalAgent)
  -> DecisionInvocation { affordance, targets, proposed effects }
  -> scope digest check: actor components + named targets unchanged
  -> precondition check over those components
  -> effect ceiling check against the affordance's slots
  -> band selection from kernel entropy
  -> lower the band's effects to a WorldPatch
  -> structural admission
  -> atomic commit; DecisionEvent records affordance, band, and patch digest
```

Every stage is a pure function of committed state, the proposal, and kernel
entropy. A rejection returns the complete mismatch set. Player, NPC,
institution, population, import, and elaborator actions share this pipeline;
different admission lanes derive different authority envelopes and never
different mutations.

## Institutional layers and flow

Layers of collective agency are relations, not special subjects:

- **Containment** nests places and jurisdictions.
- **Membership** binds persons and slices to institutions and populations.
- **Jurisdiction** is an `Authority` component whose scope is a subject set or
  a place subtree.
- **Representation** is `Selection`: an institution acts through an office
  held by a person, so a decree carries an incumbent's name and a succession
  risk.

An institution normally carries an `OperationalAgent` controller and authored
affordances such as `Decree`, `Levy`, `Contract`, or `Deploy`. Their effects
create `Commitment` on subordinates and counterparties. A due or defaulted
commitment creates or advances `Pressure`. Unresolved pressure on a subject
becomes that subject's next decision opportunity. That is the whole internal
flow: obligation → pressure → opportunity, propagating along typed relations
with no narrator and no wave planner. A population slice under enough pressure
with no person-shaped subject to bear it derives an `IndividuationRequired`
boundary.

An institution that also needs a person-shaped public voice gets a separate
`NarrativePersona` controller on a person subject bound by `Selection`. Two
controllers, disjoint scopes; awkward overlap is rejected, not resolved by
precedence.

## Elaboration

The world grows only where committed state reaches a typed boundary or where an
authored seed request opens a scope. Both are answered by the same patch.

```text
CausalBoundary
  UnelaboratedDestination { route, place }
  PolityInCausalRange     { relation, subject }
  IndividuationRequired   { population, slice, pressure }
  MissingStructure        { scope }     // a commitment or pressure has no authority,
                                        //   channel, or redress path that could resolve it

SeedRequest { jurisdiction: DraftHandle, brief: EvidenceRef }   // draft phase only;
                                        //   representable, not inhabited: Draft answers nothing
```

Boundaries are derived from a revision exactly as opportunities are, and each
carries the digest of the components that derive it. One command answers one:

```text
CommandBody::AdmitPatch { answers: Boundary | SeedRequest, patch: WorldPatch }
```

A commit clears exactly the boundary it answers. Nothing else clears one. A
patch answering a boundary the kernel no longer derives is rejected.

Seed admission is `AdmitPatch` against an empty draft world under draft
authority. A consumer-authored seed and a compiled seed are the same patch.
There is no seed type, registry, publication handoff, or compiler install path.

### Elaborators

Eight elaborators are eight instances of one `OperationalAgent` loop, each
assigned one jurisdiction — a place subtree or realm. Their tool catalog is
generated from this document's operation set and declaration kinds, plus
`record_gap` and `submit`. It cannot contain an operation the reducer does not
own, because it is derived from the reducer's vocabulary.

```text
loop:
  take the oldest open boundary or seed request in my jurisdiction;
    if none, take my jurisdiction's scale deficit
  retrieve evidence from the Vault for its referents; keep exact receipts
  build one WorldPatch with declaration and operation tools
  submit; on rejection, repair the same draft from the complete mismatch set
  on commit, checkpoint: admitted commit ancestry, open leads, exact rejections
  stop when my jurisdiction has no open boundary and no deficit
```

The scale deficit is derived, not owned. The seed carries a `WorldScaleIntent`:
a target count of qualified subjects per level (person, institution,
population) and per realm, with realm weights that distribute the target and
never raise it. A subject qualifies structurally: active, one controller, at
least one `Goal` commitment, at least one executable affordance. The kernel
recounts after every commit and publishes the deficit per jurisdiction as
derived state. Only admitted subjects reduce it; a rejected patch leaves it
visible. Elaborators answering a deficit declare goal-bearing subjects at the
level the deficit names — a person with a want, an office with a mandate, a
slice with a grievance — grounded in evidence like any other patch. Detail
first, then scarcity of attention; that order is what makes actors at every
level pursue their own ends rather than wait to be noticed.

An elaborator session is resumable and checkpointed against admitted commit
ancestry; raw conversation is never authority. Elaborators own no fictional
truth and no completeness verdict; the target is authored, the count is
structural, and the reducer alone admits. One elaborator may fill semantic
fields — names, labels, Persona material — inside a typed patch; it is not a
Persona, receives no prose membrane, and holds no authority.

Eight loops do not serialize on one another. A boundary binds to its own scope
digest, so a patch answering it commits at any revision where that digest is
unchanged; only the mailbox serializes the physical commit. Two elaborators
touching overlapping components conflict on scope digest and one repairs.

A translation gap never mints a boundary. Gaps are non-fictional inference
evidence that may inform a later human decision to extend this vocabulary, and
that extension is a code change, not a world patch.

## Scale: many subjects, few inferences

A living world holds subjects several times its cell budget on purpose. The
2,400-subject, 240-cell profile is the design target: enough goal-bearing
actors at every level that the world has its own ends, and few enough cells
that attention is scarce and the scheduler must choose. Scarcity is the
choke that produces selection, offscreen consequence, and the feeling that
things happen whether or not anyone is watching. Four mechanisms carry that,
none of which is a second identity layer:

1. **Budgeted connected cover.** Every active subject is in the cover every
   tick. The scheduler partitions the agency graph — containment, membership,
   jurisdiction, shared place, relation, pressure — into at most the cell
   budget, giving singleton cells to the highest-pressure and highest-debt
   subjects and grouping the rest by adjacency. The cover is derived and
   disposable; the kernel never sees it. Populations and institutions are
   subjects in their own right, and `IndividuationRequired` still fires when
   a pressure needs a person no slice can bear.
2. **Zero-inference motion.** `Time.advance` applies every due clock
   deterministically: a `Routine` commitment whose preconditions hold
   auto-fulfills, so ordinary life proceeds for unattended subjects; an
   `Obligation` or `Goal` past due advances `Pressure` on its subject. The
   quiet world moves without being looked at and without inventing spurious
   crisis.
3. **Ordered attention.** The scheduler is a pure planner over one revision.
   It orders opportunities by unresolved pressure, then time since last
   opportunity, then subject id, so every subject receives direct attention
   within bounded ticks absent a mandatory foreground override.
4. **Cell resolution.** A singleton cell is detail focus: a `NarrativePersona`
   controller receives its prose membrane, an `OperationalAgent` its full
   permissioned view. A grouped cell is one inference over its constituents'
   partitioned views — each constituent's private knowledge stays labeled as
   its own, never unioned — returning zero or one attributed proposal per
   constituent, each verified separately by the action pipeline. A
   `NarrativePersona` subject in a grouped cell is represented operationally
   at coarse resolution this tick; its controller, scope, and authority do not
   change. The cell can emit no cell-owned mutation, and no subject is ever
   spoken for by an arena.

Debt rotation makes the choke fair. With N active subjects, a cell budget B,
and U urgency slots reserved for the head of the attention order, the
rotation reserve is R = B − U, and a subject continuously active for
ceil(N/R) ticks with stable membership is a singleton at least once. ceil(N/B)
is unreachable whenever N > B, because it would leave subjects uncovered. The
constituent cap yields for two reasons, when the budget's token capacity is
short of the active count and when connected components cannot pack inside
the cell budget even with capacity to spare; the cell budget never yields;
`oversubscribed` is the operator-visible signal that the cap gave way. The
cover is pure over its inputs and the attention order is a real input: the
same subjects in a different order give the same coverage and a different
singleton set, because urgency and backfill read that order. The deterministic clocks keep commitments moving in
between.

A grouped cell buys one inference, never one admission rule. Its constituents
are grouped because they are connected, so they are the subjects most likely
to contend: each attributed proposal binds to its own scope digest, and when
the first act lands (a telling changes a co-located listener's knowledge),
the second is refused `ScopeChanged` rather than committed on a stale scope.
Declines are submitted first so silent constituents consume their turn
without contention, and the driver reports submitted against committed per
cell so the gap is visible. The cover did not create the contention; two
singleton cells for the same pair would contend identically, and a refused
constituent costs one admission check rather than one inference. The grouping
heuristic stays. A grouped constituent has no Persona turn and no
Interpreter, so its refused proposal stays refused. A narrative subject in a
singleton cell is interrupted rather than silently refused: the lane
re-lowers once through the Interpreter with the delta the subject could
perceive since its turn, bound to the fresh digest, at the cost of one
Interpreter inference and no Persona re-run (plan step 9, in design).

## What stays open

The ontology does not bound the stories a world can hold, for three reasons
that are structural rather than promissory:

- **Affordances are data.** A world declares its own action vocabulary over the
  closed operation set. Genre lives in the catalog, not in the kernel.
- **Persona prose is free.** A `NarrativePersona` may say, attempt, or feel
  anything; the Interpreter lowers what the vocabulary can carry and records
  exact gaps for the rest. Unrepresentable meaning is preserved as evidence,
  never rejected or invented.
- **Structure is sparse; liveness is authored.** A world activates with enough
  components for its current horizon, and grows toward the scale it was given.
  Ordinary life is a recurring `Commitment`; a rite is a `Channel` with a
  reach set; a private joke is `PersonaMaterial`. None of that waits on a
  count, and no count admits any of it.

The ontology does bound one thing on purpose: a consequence exists only if a
component changed. Speech cannot imply an uncommitted effect. That is what
makes deterministic verification possible, and it is a constraint on lying,
not on storytelling.

## Cut line

Deleted before replacement behavior is added, with no compatibility path:

- caller-supplied string identifiers and `SubjectRef { kind, id: String }`;
- `AdmitEntity` two-phase admission and `initial_components`;
- the flat `Vec<PermittedWorldMutation>` batch and every reconciliation step;
- the wave-level strategic outcome resolver and the `strategic_activity_outcome`
  flat-JSON proposal boundary;
- eight legacy component kinds that carried no decision constraint:
  `Capability` (subsumed by affordance grants), `Condition` (subsumed by
  `Pressure`), `Posture` (derivable), `Memory` (Persona material),
  `CivicSystem` manifest (the typed subgraph), `PopulationLineage`
  (individuation), `Lifecycle` and `WorldTime` (kernel-owned);
- `ElaboratorTitle`, elaboration demand and deficit, complexity rounds,
  strategic waves, fission completion counts, `detail_debt` rotation, and every
  `*Qualification` and `*Verification` type;
- `CampaignRegistry`, `WorldSeed`, and `publish_session_zero` as a distinct
  admission path;
- the rule that a stale proposal is discarded on any revision change; it is
  replaced by scope-digest binding.

## Subtraction budget

- 20 component kinds → 12; 18 `WorldMutation` variants → 27 named operations
  under 1 patch primitive, replacing both the mutation enum and the separate
  outcome-effect sum;
- 5 qualification and verification types plus 1 outcome resolver → 0;
- elaborator titles, semantic qualification, and round budgets → 4 derived
  boundaries, 1 seed request, and 1 structurally counted scale deficit;
- 2 seed admission paths → 1;
- N model-facing tool schemas hand-written per stage → 1 derived catalog.

The only additions are the patch reducer, precondition and effect-slot
checking, band selection, boundary derivation, the component types, and the
tool projection. A net-additive pass beyond those has bought something the
invariants did not ask for.

## Build budget

No new crate, binary, dependency, or service. Types and reduction inside
`crates/ghostlight-dungeon/src/world/`, one new `patch` module, one new
`affordance` module, focused `ghostlight-dungeon` tests. No workspace-wide or
release build is admitted for this stage.

## Verification contract

Beyond the existing kernel proofs, focused tests must prove:

**Structure**

1. A patch declaring an institution whose jurisdiction references an
   undeclared, non-canonical place is rejected, allocates no ID, and commits
   nothing — the exact Run 115 shape.
2. The same patch with the place declared commits both atomically.
3. A draft handle resolved to the wrong kind is rejected; a cross-kind
   canonical reference does not compile.
4. Custody that does not conserve is rejected; `admit` without evidence is
   rejected.
5. Knowledge citing an inaccessible fact is rejected; scoped knowledge does not
   leak into another subject's projection.
6. A rejection returns every failed check, and repairing exactly those checks
   yields a commit.

**Action**

7. Acting at a distance, spending unheld custody, commanding outside authority,
   referencing an unknown fact, and speaking beyond channel reach are each
   rejected with the exact failed precondition.
8. A proposal exceeding an effect slot is rejected; one inside it commits.
9. The same proposal at the same revision selects the same outcome band; a
   different command ID may select a different one; no caller-supplied value
   influences the draw.
10. Only the selected band's effects appear in the commit.
11. A proposal whose scope digest changed is rejected; one whose scope digest
    is unchanged commits at a later revision.

**Flow**

12. A defaulted commitment advances pressure on its subject; the subject then
    derives an opportunity; a place, resource, fact, or channel never does.
13. A population slice under pressure with no person to bear it derives
    `IndividuationRequired`, and answering it clears exactly that boundary.

**Elaboration**

14. An `AdmitPatch` answering no boundary in `active`, or a boundary the kernel
    does not derive, is rejected.
15. Two patches answering disjoint boundaries at different revisions both
    commit; two touching the same components conflict on scope digest.
16. The generated elaborator tool catalog contains exactly the operation set
    and declaration kinds in this document and nothing else: every variant has
    one tool, every emitted branch decodes, and the counts match the exemplar
    lists.
17. Seed admission and boundary elaboration reach the same reducer and CAS.
18. A structurally sparse world — few subjects, complete references — activates
    and runs; no count or ratio blocks it.
19. `Time.advance` auto-fulfills a due `Routine` whose preconditions hold and
    advances pressure for a due `Obligation`, with no inference and no
    spurious pressure on unattended subjects.
20. A batched inference returning proposals for several controllers commits
    only those that pass their own precondition and effect checks; a proposal
    attributed to a controller outside the batch is rejected.
21. With N qualified subjects and cell budget B, the derived cover contains
    every active subject, at most B cells, and the highest-pressure and
    highest-debt subjects as singletons; every subject reaches a singleton
    within bounded ticks.
22. A subject counts toward the scale deficit only when active with one
    controller, one `Goal`, and one executable affordance; a rejected patch
    leaves the deficit unchanged and visible.
