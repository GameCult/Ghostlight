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
pre-rebuild machine and are teardown evidence. Three of their ideas survive
here, named where they are used: one mutation vocabulary across every admission
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

The sealed kernel holds subjects with a label and a kind
(`Person | Institution | Population`), one affordance (`Speak`), and one action
(`Speak { text }`). `reduce` checks phase, exact revision-bound opportunity,
controller identity, and affordance grant, then appends a `DecisionEvent`. There
is no place, route, resource, relation, fact, commitment, or pressure, no
precondition on any action, no effect beyond the event itself, and no ingress
that could admit structure.

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
9. Elaboration answers a derived causal boundary or an authored seed request.
   There is no wave, quota, round budget, coverage ratio, or completeness
   metric.
10. A proposal binds to the digest of the components its verification reads,
    not to the whole world digest. It commits at any later revision where that
    scope digest is unchanged, and is rejected when it is not.
11. Every tool a model may call is a projection of this vocabulary. The tool
    catalog is derived state and cannot carry an operation the reducer does not
    own.

## Identity

Three namespaces, three newtypes, issued only by the reducer.

```text
SubjectId   person, institution, population, external mirror   — can decide
EntityId    place, resource, fact, channel                     — cannot decide
EdgeId      route, relation, commitment, pressure              — connects the above
```

Display names are labels. They never resolve a reference or bind structure.

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
| `Channel` | channel: reach set, latency, controller | how facts travel; who can be silenced |
| `Commitment` | subject → counterparty: kind (`Routine`, `Obligation`, `Goal`), due, stake | obligation with a clock; autonomous motion |
| `Pressure` | source → target: magnitude, unresolved | the causal boundary trigger |
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
Selection        install(office, incumbent), vacate, set_method
Redress          open_forum(kind, forum, standing), close_forum
Knowledge        acquire(subject, fact, source, confidence), communicate(speaker, fact,
                 channel), forget
Channel          set_reach, set_controller
Commitment       create(subject, counterparty, kind, due, stake), fulfill, default,
                 release
Pressure         create(source, target, magnitude), advance, reduce, resolve
PersonaMaterial  set(subject, material)
Retirement       retire(referent, reason)
Time             advance(minutes)
```

Twenty-nine operations. Adding one is a design change to this document and a
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
- externally owned components change only through their admitted owner;
- the patch produces at least one canonical change.

Interestingness, novelty, political diversity, name quality, prose quality, and
counts do not appear here and may not be added.

## Affordances: the action vocabulary a world authors

The kernel owns component operations. A **world** owns its affordance catalog.
This is the generality lever: a setting with spellcraft, insurance claims,
oath-rhythms, or memetic sovereignty authors affordances from the same
twenty-nine operations, and the kernel never learns a genre.

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
  Knows { fact: Role, at_least: Confidence }    // Knowledge
  CanReach { subject: Role, via: Channel }      // channel reach
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

SeedRequest { jurisdiction: DraftHandle, brief: EvidenceRef }   // draft phase only
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
  take the oldest open boundary (or seed request) in my jurisdiction
  retrieve evidence from the Vault for its referents; keep exact receipts
  build one WorldPatch with declaration and operation tools
  submit; on rejection, repair the same draft from the complete mismatch set
  on commit, checkpoint: admitted commit ancestry, open leads, exact rejections
  stop when my jurisdiction has no open boundary
```

An elaborator session is resumable and checkpointed against admitted commit
ancestry; raw conversation is never authority. Elaborators own no fictional
truth, no target count, no deficit, and no round budget. They stop when the
horizon is structurally sufficient, which is the definition of a valid sparse
world. One elaborator may fill semantic fields — names, labels, Persona
material — inside a typed patch; it is not a Persona, receives no prose
membrane, and holds no authority.

Eight loops do not serialize on one another. A boundary binds to its own scope
digest, so a patch answering it commits at any revision where that digest is
unchanged; only the mailbox serializes the physical commit. Two elaborators
touching overlapping components conflict on scope digest and one repairs.

A translation gap never mints a boundary. Gaps are non-fictional inference
evidence that may inform a later human decision to extend this vocabulary, and
that extension is a code change, not a world patch.

## Scale: many subjects, few inferences

A world may hold thousands of subjects while a tick affords a few hundred
inferences. Four mechanisms carry that, none of which is a second identity
layer:

1. **Coarse by construction.** Populations and institutions are subjects. A
   realm is a handful of deciding subjects until `IndividuationRequired`
   fires, and it fires only when a pressure needs a person to bear it.
   Individuals are not manufactured toward a count and then compressed back.
2. **Zero-inference motion.** `Time.advance` applies every due clock
   deterministically: a `Routine` commitment whose preconditions hold
   auto-fulfills, so ordinary life proceeds for unattended subjects; an
   `Obligation` or `Goal` past due advances `Pressure` on its subject. The
   quiet world moves without being looked at and without inventing spurious
   crisis.
3. **Ordered attention.** The scheduler is a pure planner over one revision.
   It orders opportunities by unresolved pressure, causal exposure,
   readiness, and time since last opportunity, so every subject receives
   direct attention within bounded ticks absent a mandatory foreground
   override.
4. **Batched representation.** The runner may present several
   `OperationalAgent` opportunities that share a place or pressure to one
   inference. Each returned proposal is attributed to its own controller and
   verified separately by the action pipeline. The batch is derived,
   disposable, and invisible to the kernel; it changes representation, never
   authority, and can emit no batch-owned mutation.

A `NarrativePersona` subject not selected for attention does not act this
tick; its collective acts at the collective level and reaches it through
typed commitments. No subject is ever spoken for by an arena.

## What stays open

The ontology does not bound the stories a world can hold, for three reasons
that are structural rather than promissory:

- **Affordances are data.** A world declares its own action vocabulary over the
  closed operation set. Genre lives in the catalog, not in the kernel.
- **Persona prose is free.** A `NarrativePersona` may say, attempt, or feel
  anything; the Interpreter lowers what the vocabulary can carry and records
  exact gaps for the rest. Unrepresentable meaning is preserved as evidence,
  never rejected or invented.
- **Structure is sparse.** A world needs only enough components for its current
  horizon. Ordinary life is a recurring `Commitment`; a rite is a `Channel`
  with a reach set; a private joke is `PersonaMaterial`. None of that requires
  a count to be satisfied first.

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

- 20 component kinds → 12; 18 `WorldMutation` variants → 29 named operations
  under 1 patch primitive, replacing both the mutation enum and the separate
  outcome-effect sum;
- 5 qualification and verification types plus 1 outcome resolver → 0;
- elaborator titles, waves, quotas, demand, deficit, and debt rotation → 4
  derived boundaries and 1 seed request;
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
    and declaration kinds in this document and nothing else.
17. Seed admission and boundary elaboration reach the same reducer and CAS.
18. A structurally sparse world — few subjects, complete references — activates
    and runs; no count or ratio blocks it.
19. `Time.advance` auto-fulfills a due `Routine` whose preconditions hold and
    advances pressure for a due `Obligation`, with no inference and no
    spurious pressure on unattended subjects.
20. A batched inference returning proposals for several controllers commits
    only those that pass their own precondition and effect checks; a proposal
    attributed to a controller outside the batch is rejected.
