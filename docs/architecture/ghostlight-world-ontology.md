# Ghostlight World Ontology

## Status

This document is the closed typed vocabulary for world structure and
elaboration. `docs/architecture/ghostlight-dungeon-mvp.md` remains the authority
map: it owns who may write, what the invariants are, and where the boundaries
sit. This document owns only what the vocabulary *is*, and it may not introduce
a writer, a gate, or a second commit path.

`docs/architecture/ghostlight-transition-algebra.md` describes the pre-rebuild
vocabulary and is teardown evidence. Its principle that different admission
lanes share one mutation vocabulary survives here. Its types, subject kinds,
string identifiers, and migration ladder do not.

## Objective

Let a world grow deep enough to constrain decisions, using structure the reducer
can check, so that depth never depends on a model's opinion that the world is
interesting.

## Current mechanism

The sealed kernel holds subjects with a label and a kind
(`Person | Institution | Population`), one affordance kind (`Speak`), and one
action (`Speak { text }`). There is no place, route, resource, relation, fact,
commitment, or pressure, and no ingress that could admit one. A world is a room
of named voices.

## The failure this vocabulary is designed against

Run 115 terminated because `inst:kharad-road-keepers` referenced
`loc:kharad-rhythm-road`, which did not exist. Four reconciliation steps failed
to repair it. Three structural causes, all in the vocabulary rather than in the
models:

1. **Names were keys.** `SubjectRef { kind, id: String }` accepted a
   caller-supplied identifier. A plausible-looking string was indistinguishable
   from a real referent until reduction.
2. **Admission was two-phase.** `AdmitEntity { initial_components: BTreeSet<..> }`
   declared which components a subject *could* have, separately from populating
   them. A referenceable but unpopulated entity was a legal intermediate state.
3. **The batch was flat.** `Vec<PermittedWorldMutation>` permitted each mutation
   independently, so a cross-mutation dangling reference was a runtime discovery
   rather than an unrepresentable input. Reconciliation existed to repair that
   class, which made it a repair loop standing in for an owner.

Depth had the same shape of defect. Texture lived in free strings —
`posture: String`, `resource_kind`, `capability_id`, `persistent_features`,
`description` — which nothing could reason over. A world with many actors still
felt thin, and the answer reached for was more model tribunals:
`ElaboratorTitle`, `WorldComplexitySemanticQualification`,
`WorldComplexityFissionQualification`,
`WorldComplexityIndividuationQualification`,
`WorldComplexitySemanticVerification`. Those were compensators for an ontology
with nowhere to put depth.

The cut: make identity unforgeable, make admission atomic, and give depth a
typed home. Then structural admission is sufficient and no verifier is needed.

## Invariants

1. A reference is either an exact canonical ID issued by the reducer or a draft
   handle resolved inside the same patch. There is no third form.
2. Cross-kind reference is unrepresentable, not rejected at runtime. Typed ID
   newtypes carry the kind.
3. A patch reduces completely or not at all. No canonical ID is issued, revealed,
   or consumed by a rejected patch.
4. Only subjects decide. Places, resources, facts, and channels have no
   controller, no affordance, and no decision opportunity.
5. Custody conserves. A patch may move or transform a declared quantity; it may
   not create one without an explicit, evidenced admission.
6. Knowledge is scoped. A subject knows an exact fact or does not; a patch may
   add knowledge only by citing an accessible fact or admitting an evidenced one.
7. Persona material enriches and never substitutes. Values, voice, memory, and
   relationship reads cannot stand in for authority, custody, topology, or
   knowledge.
8. Elaboration answers a derived causal boundary. There is no wave, quota,
   round budget, coverage ratio, or completeness metric.

## Identity

Three namespaces, three newtypes, issued only by the reducer.

```text
SubjectId   person, institution, population, external mirror   — can decide
EntityId    place, resource, fact, channel                     — cannot decide
EdgeId      route, relation, commitment, pressure              — connects the above
```

Display names are labels. They are never keys, never resolve a reference, and
never bind structure.

## Patch construction

One primitive admits structure, in draft and in active play:

```text
WorldPatch
  declarations: Vec<Declaration>       // new subjects, entities, edges — draft handles only
  components:   Vec<ComponentSpec>     // populate declared or existing referents
  evidence:     Vec<EvidenceRef>       // exact receipts where a fact is admitted

Ref<Id> = Existing(Id) | Draft(DraftHandle)
```

Reduction is one pass with no repair step:

1. Index every declaration by handle. Duplicate or unused handles reject.
2. Resolve every `Ref::Draft` against that index, checking kind. Unresolvable or
   wrong-kind rejects.
3. Resolve every `Ref::Existing` against the current revision, checking kind.
   Unknown or wrong-kind rejects.
4. Check structural admission (below) over the complete candidate graph.
5. Only then allocate canonical IDs for all declarations and commit atomically.

`inst:kharad-road-keepers` referencing a road that is neither declared in the
patch nor already canonical is rejected at step 2, deterministically, before any
ID exists. There is nothing for a reconciler to do, so there is no reconciler.

### Structural admission

The reducer checks only these, and nothing about quality:

- every referent exists or is declared in the same patch;
- containment is acyclic;
- routes have exact place endpoints on both ends and a valid cost;
- relation and edge endpoint kinds match the edge kind;
- custody transfers conserve declared quantity;
- authority and affordance scope covers the proposed mutation;
- knowledge additions cite an accessible or newly evidenced fact;
- population slices are disjoint beneath their declared parent;
- externally owned components change only through their admitted owner.

Interestingness, novelty, political diversity, name quality, prose quality, and
counts do not appear in this validator and may not be added to it.

## Components

Twelve, each earning its place by constraining a decision. Anything that does
not constrain a decision is Persona material or an event, not a component.

| Component | Shape | Decision it constrains |
| --- | --- | --- |
| `Position` | subject to place | who is present, who perceives, who must travel |
| `Route` | place to place, access, cost | who can arrive in time, who can be cut off |
| `Custody` | holder to resource, quantity | what can be spent, seized, withheld |
| `Dependency` | subject to resource, route, or subject | what fails when supply fails |
| `Authority` | subject to scope, kind | who may legitimately command, tax, judge, admit |
| `Selection` | office to method, incumbent, term | how power is lost; succession pressure |
| `Redress` | grievance to forum, standing | where conflict goes when it cannot be fought |
| `Knowledge` | subject to fact, confidence, source | what a subject may act on at all |
| `Channel` | reach, latency, control | how facts travel; who can be silenced |
| `Commitment` | subject to counterparty, kind, due, stake | obligation with a clock; autonomous motion |
| `Pressure` | source to target, magnitude, unresolved | the causal boundary trigger |
| `PersonaMaterial` | values, voice, memories, relationship reads | lived meaning; never authority |

Civic order is the `Authority` + `Selection` + `Redress` + `Custody` subgraph. It
is not a prose manifest and does not require a model's civic verdict.

`Dependency` is the crisis transmitter. It is what makes a failed dwarven pump in
one realm a political problem in another without a narrator deciding that it
should be.

## Elaboration

The world grows only where committed pressure reaches a typed boundary. The
kernel derives boundaries from a revision exactly as it derives decision
opportunities: revision-bound, digest-bound, and never self-issued by a model.

```text
CausalBoundary
  UnelaboratedDestination { route, place }        // a route leads somewhere with no structure
  PolityInCausalRange     { relation, subject }   // an external polity now bears on committed state
  IndividuationRequired   { population, slice }   // a slice must act as a person to resolve a pressure
  MissingStructure        { scope }               // a commitment or pressure has no authority,
                                                  // channel, or redress path that could resolve it
```

One command answers one boundary:

```text
CommandBody::AdmitPatch { boundary: Option<CausalBoundary>, patch: WorldPatch }
```

In `draft`, `boundary` is `None` and draft authority governs seed construction.
In `active`, `boundary` must be `Some` and must match a boundary derived at the
exact submitted revision. A commit clears the boundary; nothing else does.

Seed admission is this command against an empty draft world. There is no
separate seed type, seed registry, publication handoff, or compiler install path.
A consumer-authored seed and a compiled seed are the same patch.

One author inference may fill semantic fields in a proposed patch. It is not a
Persona, receives no prose membrane, and holds no authority. World authoring is
not roleplay and does not borrow Persona standing.

A translation gap never mints a boundary. Gaps are non-fictional inference
evidence; they may inform a later human design decision to extend this
vocabulary, and a vocabulary extension is a code change, not a world patch.

## Sparse validity

A world with enough structure for its current horizon is complete. Actor counts,
cover ratios, and qualitative review are evaluation evidence produced after a
run. They may be severe. They may not become a writer, a gate, or a boundary.

## Cut line

Deleted before replacement behavior is added, with no compatibility path:

- caller-supplied string identifiers and `SubjectRef { kind, id: String }`;
- `AdmitEntity` two-phase admission and `initial_components`;
- the flat `Vec<PermittedWorldMutation>` batch and every reconciliation step;
- eight component kinds that carried no decision constraint: `Capability`
  (subsumed by affordance grants), `Condition` (subsumed by `Pressure`),
  `Posture` (a free string, derivable from commitments and relations),
  `Memory` (Persona material), `CivicSystem` manifest (the typed subgraph),
  `PopulationLineage` (individuation), `Lifecycle` and `WorldTime` (kernel-owned);
- `ElaboratorTitle`, elaborator quotas, complexity rounds, strategic waves,
  fission completion counts, and every `*Qualification` and `*Verification` type;
- `CampaignRegistry`, `WorldSeed`, and `publish_session_zero` as a distinct
  admission path.

`docs/architecture/ghostlight-world-consumer-api.md` still names
`CampaignRegistry`, `WorldSeed`, Session Zero publication, and strategic waves.
It describes the pre-rebuild admission path and must be rewritten against
`AdmitPatch` or demoted to evidence before it is cited again.

## Subtraction budget

The vocabulary must land smaller than what it replaces:

- 20 component kinds to 12;
- 18 `WorldMutation` variants to 1 patch primitive;
- 5 qualification and verification types to 0;
- elaborator titles, waves, quotas, and round budgets to 4 derived boundaries;
- 2 seed admission paths to 1.

The only additions are the patch reducer, the boundary derivation, and the
component types themselves. If a pass ends net-additive beyond those three, it
has bought something the invariants did not ask for.

## Build budget

No new crate, binary, dependency, or service. The work is types and reduction
inside `crates/ghostlight-dungeon/src/world/`, plus one new `patch` module, plus
focused `ghostlight-dungeon` tests. No workspace-wide or release build is
admitted for this stage.

## Verification contract

Beyond the existing kernel proofs, focused tests must prove:

1. A patch declaring an institution whose jurisdiction references an undeclared,
   non-canonical place is rejected, allocates no ID, and commits nothing — the
   exact Run 115 shape.
2. The same patch, with the place declared, commits both atomically at one
   revision.
3. A draft handle resolved to a declaration of the wrong kind is rejected; a
   cross-kind canonical reference does not compile.
4. Custody transfer that does not conserve declared quantity is rejected.
5. Knowledge addition citing an inaccessible fact is rejected; scoped knowledge
   does not leak into another subject's projection.
6. An `AdmitPatch` in `active` carrying no boundary, a stale-revision boundary,
   or a boundary the kernel does not currently derive is rejected.
7. Answering a boundary clears exactly that boundary and no other.
8. A place, resource, fact, or channel can never receive a controller, an
   affordance, or a decision opportunity.
9. A structurally sparse world — few subjects, complete references — activates
   and runs; no count or ratio blocks it.
10. Seed admission and boundary elaboration reach the same reducer and the same
    CAS, with no second path.
