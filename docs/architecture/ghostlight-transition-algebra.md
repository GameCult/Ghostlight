# Ghostlight World Transition Algebra

Status: adopted architecture; foreground, reaction, strategic, time, travel,
approval-gated fission, and bounded region expansion use the mutation reducer.
Initial compiler publication is classified as a one-time creation transaction,
and named-person materialisation is classified as a resolution transaction.
Aggregate-storage removal remains migration work.

## Objective

Ghostlight needs one physical world. A player action, an NPC reaction, a
strategic institution, and a low-resolution Gestalt cell may use different
admission and resolution procedures, but a successful outcome must change the
same canonical components through the same kernel-owned commit primitive.

The transition algebra exists so that:

- freeform action remains possible without model-authored structural writes;
- impossible actions are absent from the available operation space;
- every committed change has exact subjects, authority, provenance, and
  version bindings;
- foreground and background simulation cannot acquire asymmetric powers;
- new play scenarios compose existing semantic mutations instead of growing a
  bespoke effect enum for every story beat.

This design adapts the useful core of action-language and planning literature:
states are typed assignments, actions have explicit applicability conditions,
and transitions have bounded effects. It does not adopt PDDL, STRIPS, or a
general logic-programming runtime as Ghostlight's public ontology. BC+ provides
the causal-law distinction, STRIPS and PDDL provide the precondition/effect
discipline, and REBA provides the important reminder that coarse and fine
simulation must refine the same transition system rather than inventing two
worlds. See [BC+](https://academic.oup.com/logcom/article/30/4/899/2917837),
[PDDL2.1](https://arxiv.org/abs/1106.4561), and
[REBA](https://arxiv.org/abs/1508.03891).

## Authority map

### Owner

The per-campaign `WorldKernel` owns one `WorldMutationBatch` validation and
commit primitive. It is the only subsystem allowed to change canonical world
components or advance fictional time.

### Inputs

The owner may read:

- the exact canonical campaign revision;
- the exact resolution epoch when a strategic wave is involved;
- typed subjects and their component versions;
- an accepted action, reaction, lifecycle, governance, or strategic-outcome
  receipt;
- a digest-bound `MutationAuthorityEnvelope` compiled from current state;
- a proposed batch containing only semantic mutations named by that envelope;
- exact evidence, permission, roll, and model-stage receipts required by the
  originating procedure.

### Outputs

One successful commit produces, atomically:

- the next canonical component state;
- append-only world events describing the observable means and outcome;
- a private exact `world_mutation_receipt.v1`;
- the command or strategic receipt that caused the transition;
- any knowledge-channel-aware news inputs derivable from committed events.

Eve, CultMesh, story, and news projections refresh only after the commit.

### Derived and demoted state

- `WorldEffectDelta`, `ActorStateDelta`, `StrategicCellEffect`, and
  `StrategicOutcomeEffect` are requirements evidence and migration inputs. They
  are no longer allowed to own a production write after their resolver moves.
- `StrategicActivityKind` describes proposed means. It is not a mutation.
- An `ActionIntent` and its intended effect are proposals. They are not facts
  and do not become true because a model narrated them.
- Simulation cells, covers, leases, salience, and detail debt choose inference
  resolution. They are not fictional world components.
- The transcript contains exact committed speech and outcome prose. Eve lowers
  it chronologically without a generative rewrite; display text cannot repair
  or create state.
- Actor equipment lists, institution resource lists, and Gestalt resource sets
  become projections over resource subjects plus custody. A resource does not
  change ontology when its owner changes.
- A dormant named Gestalt member and its materialized foreground form are one
  actor subject. Materialization changes simulation resolution, not identity or
  physical existence.

### Forbidden writers

The migration is not complete while any of these can decide canonical state:

- `apply_world_effect`;
- `apply_strategic_outcome_effect`;
- the direct private-delta loop in `ResolveReactionWave`;
- bespoke actor, Gestalt, or member movement and migration writers;
- any Gestalt folding code that unions private knowledge, possessions, or
  pressure into a population;
- model output, browser payloads, story rendering, news generation, retrieval, or a
  derived simulation cell.

Bounded region expansion now lowers to the same batch primitive through typed
place and proposition admission plus exact topology changes. It may possess a
broader administrative authority envelope, but not another state-writing
mechanism. Approval-gated population fission likewise lowers to entity
admission, lineage, custody, and membership mutations; the fission projector
owns only the derived resolution profiles and cover epoch. Initial compiler
publication is not a transition over an existing world; its separate creation
authority is bounded below.

### Shared paths

All of these lower to `WorldMutationBatch` before state changes:

- player `Attempt` confirmation;
- NPC initiative resolution;
- actor reactions and private appraisal consequences;
- strategic individual, institution, cohesive Gestalt, and arena-constituent
  outcomes;
- travel, waits, and strategic time advancement;
- resource production, use, damage, repair, and exchange;
- named-person joining, leaving, and migrating between populations;
- population migration and approval-gated fission;
- contract amendments that are permitted to change forward-looking world
  state;
- compiler-admitted entities added after publication and later bounded region
  expansion.

### Cut line

The new validator applies a whole batch to an isolated transactional overlay,
checks local and cross-mutation invariants, and returns either one complete next
state or an error. Resolvers then emit the new mutation vocabulary directly.
When the last production caller moves, the old committing functions and their
model schemas are deleted. Persisted legacy receipts remain readable history;
they never re-enter a commit path.

No repair loop, converter with independent policy, or post-commit reconciler is
allowed to preserve a second opinion about the result.

## Transactions outside the mutation algebra

Two operations change canonical storage without claiming to be fictional
world effects. They remain narrow because forcing them into `WorldMutation`
would lie about what happened.

### Initial campaign publication

There is no prior campaign state to mutate at revision zero. The
`CampaignRegistry` therefore owns discoverability of an approved seed, while
`CampaignStore` owns its atomic installation:

- input is one validated `approved_campaign_brief.v1`, its exact evidence and
  model receipts, membership, governance, DM state, and approval digest;
- every row is written to a fresh `.creating-<campaign>-<nonce>` store through
  one empty-store compare-and-swap batch;
- the staging directory becomes discoverable only through one atomic rename;
- retry is idempotent by campaign ID plus approved seed digest;
- a nonempty store, conflicting digest, failed row write, or failed rename
  publishes nothing.

The seed installer may establish initial subjects and components because no
world exists yet. It cannot target a published campaign, advance fictional
time, or serve imports, reloads, expansion, or later compiler output. The
legacy internal `CreateCampaign` message is initialization plumbing, not a
runtime action authority; public player ingress rejects it. Once the directory
is published, every fictional change belongs to the normal kernel path.

### Named-person materialisation and folding

A dormant `GestaltMemberDelta` and its foreground `ActorState` are two
resolutions of one persistent person. `WorldKernel` owns the resolution
transaction at a safe command boundary:

- materialisation derives the foreground actor projection from the Gestalt
  baseline plus the exact member delta;
- folding writes changed individual capabilities, knowledge, equipment,
  conditions, obligations, relationships, goals, memories, and location back
  to that member delta, then retires only the foreground projection;
- the transaction advances world revision so stale actor-bound commands and
  surfaces fail, advances `resolution_epoch`, and clears the old derived cover;
- it does not advance fictional time, relocate the person, transfer custody,
  communicate knowledge, change population pressure, or alter the Gestalt
  baseline;
- one atomic world receipt plus materialisation receipt binds the member,
  projection actor, baseline/member versions, world revision, and resolution
  epoch.

The removed `GestaltAggregateDelta` was an obsolete writer: direct demotion
could previously smuggle private knowledge, resources, and pressures into the
population while automatic demotion rejected the same payload. Those changes
now require explicit semantic mutations under their own authority. Presence
resolution has no vocabulary with which to perform them.

## Canonical state model

### Subjects

Every durable entity has a stable typed `SubjectRef`:

| Subject kind | Examples | Important rule |
| --- | --- | --- |
| `actor` | player, NPC, named refugee | Identity survives foreground materialization and Gestalt folding. |
| `population` | villagers, refugee convoy, corporation workforce | May own a cohesive Gestalt Persona baseline; never owns member secrets by implication. |
| `institution` | Zhestokost command, a clinic, a guild | Collective authority is explicit, not inferred from proximity. |
| `place` | room, settlement, route node, region | Persists independently of current observation. |
| `resource` | a key, medicine lot, ship, credits tranche | Has exact custody and resource state; it is not a string copied between owner lists. |
| `pressure` | blockade clock, debt deadline, epidemic load | Carries bounded progress and consequence semantics. |
| `proposition` | a source-grounded fact, a witnessed claim, a branch-local finding | Knowledge points to the proposition; prose duplication is not knowledge authority. |
| `channel` | courier route, broadcast, private conversation | Governs who can receive information without making it public. |

Campaign time is a component of the campaign root. Relationships and topology
edges are keyed component records between subjects, not synthetic actors.

IDs are opaque and stable. Prefixes may remain a debugging convention, but no
validator infers subject kind or authority from string shape.

### Components

The target store is component-oriented even when a compatibility migration
temporarily reads old aggregate rows:

- `identity`: canonical identity plus adopted handles and their disclosure
  state;
- `occupancy`: exact place anchors and travel state;
- `custody`: exact custodian of a resource or controlled subject;
- `resource_state`: type, quantity, integrity, qualities, and lifecycle;
- `capability`: supported abilities and their limits;
- `condition`: bodily, material, social, or environmental state;
- `knowledge`: proposition, epistemic status, source, channel, and revision;
- `memory`: private episodic record and provenance;
- `relationship`: directed subject-to-subject state;
- `commitment`: goals and obligations, including creditor or beneficiary;
- `pressure`: bounded progress, threshold, consequence, and status;
- `posture`: an agentive subject's current strategic stance;
- `population_membership`: exact member, population, role, and interval;
- `population_lineage`: refinement, fission, merge, and remainder ancestry;
- `topology`: containment and route edges with travel cost and availability;
- `lifecycle`: admitted, active, transformed, consumed, retired;
- `world_time`: the campaign's monotonic fictional coordinate.

Components are sparse. A place does not need knowledge; a proposition does not
need occupancy. Component applicability is determined by subject kind and the
specific admitted entity, not by a universal bag of optional fields.

Route keys are scoped to their owning origin location. Canonical route identity
is therefore the pair `(origin_location_id, local_route_id)`, never the bare
map key. The transition overlay encodes that pair into a collision-free edge
identity before validation and maps accepted edges back to the original local
keys. Surface projections use each route's exact `destination_id`; they do not
reinterpret a local route key as a destination or global edge ID.

## The mutation vocabulary

`WorldMutation` is a closed tagged union. Each variant owns one semantic
component family and names the invariants that family can enforce. There is no
`SetField`, arbitrary path, embedded patch, or model-authored component name.

| Mutation | Canonical effect | Essential validation |
| --- | --- | --- |
| `Relocate` | Changes occupancy for an exact subject. | Origin matches, route/reach exists, mover has authority, group custody and travel approval hold. |
| `TransferCustody` | Moves an exact resource between custodians. | Source has custody, recipient exists and can receive it, quantity/identity is conserved. |
| `MutateResource` | Creates, transforms, consumes, damages, repairs, splits, or combines exact resources. | Recipe/capability, custody, conservation, lifecycle, quantity, and effect ceiling hold. |
| `ChangeCapability` | Grants, alters, suspends, or retires a supported capability. | Source of capability and duration are admitted; no narration-only expertise. |
| `ChangeCondition` | Applies, alters, or clears a condition. | Target is reachable, condition is compatible, opposition and outcome band permit it. |
| `ChangeCommitment` | Creates, alters, fulfills, defaults, or retires a goal or obligation. | Exact owner, counterparty, consent or coercive authority, and lifecycle hold. |
| `ChangeRelationship` | Creates, alters, or retires a directed relationship component. | Exact endpoints exist; one actor cannot author another's private appraisal without its own reaction authority. |
| `ChangePressure` | Creates, advances, reduces, resolves, or retires a bounded pressure. | Owner, bounds, current value, threshold, and consequence are exact. |
| `ChangeKnowledge` | Acquires, communicates, conceals, corrects, or invalidates a proposition for exact knowers. | Source knows or can observe it, channel and recipients are exact, concealment does not erase another mind. |
| `ChangeMemory` | Records, revises, or retires an exact private episodic memory. | Witness or authorized private appraisal exists; another actor cannot write it. |
| `ChangePosture` | Alters the posture of an exact agentive subject. | The acting authority can speak or decide for that subject; arenas cannot use it collectively. |
| `ChangePopulationMembership` | Joins, leaves, or transfers an exact actor between populations. | Member identity is stable, source membership exists, destination is compatible, no duplicate coverage. |
| `ChangePopulationLineage` | Records an admitted split, merge, or remainder relation. | Fission preview and approval exist; leaves remain non-overlapping and complete. |
| `ChangeIdentity` | Adopts, discloses, restricts, or retires an exact identity handle. | Self-authority or explicit custody exists; disclosure is separate from renaming. |
| `ChangeTopology` | Adds, alters, opens, closes, or retires a containment or route edge. | Evidence/branch admission, endpoint existence, reciprocal rules, and geometry invariants hold. |
| `AdmitEntity` | Creates an explicitly permitted non-resource subject and initial components. | Admission class, evidence or branch-local authority, collision freedom, and required components hold. |
| `RetireEntity` | Retires an exact subject without erasing history. | No dangling custody, occupancy, membership, topology, or active obligation remains. |
| `AdvanceWorldTime` | Advances campaign time by an exact duration. | Monotonicity, governance, no-puppeting, tick budget, and scheduled obligations hold. |

`NoMaterialChange` is not a mutation. It is an outcome with an empty mutation
batch and an explicit explanation. Speech is an event in the means record;
persuasion, obligation, relationship, and knowledge consequences are separate
mutations.

Governed time, travel, and resolution-budget proposals persist approvals before
their unanimous finalization. A fully approved but uncommitted proposal is a
retryable commit request: any already-approved active member may invoke its
advertised approval operation again, and the kernel revalidates the exact world
and governance boundary before attempting the same atomic finalization. A
duplicate vote on a proposal that is not unanimous is still rejected, and a
committed or stale proposal can never replay.

Complex effects are batches, not new verbs. Giving half a medicine lot to a
clinic composes resource split, custody transfer, and perhaps an obligation.
Evacuating a named refugee composes relocation, population-membership transfer,
and accessible knowledge. Population fission composes entity admission,
lineage, exact member transfers, and one custody transfer per scarce resource
while preserving an `other/unknown` remainder. Capabilities, shared knowledge,
goals, and active pressures inherit through the lineage reducer. Resources do
not: every parent resource must be assigned to exactly one child in the
approved preview, so aggregation cannot mint a granary for every ideological
subgroup.

Foreground assessment uses the same authority-shaped contract principle. Its
model schema contains only mutation lanes available in the current snapshot:
no route means no movement lane, no accessible undisclosed proposition means
no existing-fact acquisition lane, and absent clocks or institutions remove
their lanes. A separate observation lane is always structurally available to
the acting actor but is semantically admitted only when the exact local means
can directly perceive, measure, inspect, or test the proposed result. It lowers
to one branch-local proposition admission followed by exact knowledge
acquisition in the same batch. The effect ceiling, semantic verifier, current
location, acting actor, assessment receipt, and means digest bind that finding;
the lane cannot author remote events, hidden motives, unsupported identities,
or omniscient conclusions. Empty or unused maps may be omitted and deserialize
to no mutation. The closed dynamic schema rejects an unavailable lane even when
the model tries to emit it as `null`; provider formatting cannot manufacture
authority.

## Means, intended effect, and committed mutation

### Means

`ActionMeans` records what is attempted and may become observable:

- exact acting subject;
- bounded natural-language method;
- exact targets, instruments, resources, places, routes, and channels cited
  from the actor's permitted slice;
- explicit speech, if any;
- state and evidence references supporting the method.

The means taxonomy is intentionally open at the prose level. Ghostlight does
not need a rulebook verb for every clever action. The exact typed references
make reach, custody, knowledge, and capability testable.

### Intended effect

`MutationIntent` names desired component families, targets, and qualitative
direction. It is useful for assessment and cognitive focus, but it is not an
assertion that the change occurred. A player may intend to persuade a guard,
destroy a reactor, learn a route, or move a crowd without receiving any of
those mutations.

### Committed mutation

The admitted outcome procedure emits a concrete `WorldMutationBatch` bounded
by its authority envelope. The batch is the only one of the three objects that
can become state. A roll changes which digest-bound outcome plan is eligible;
it does not allow the resolver to invent a stronger mutation afterward.

For Sable's self-presentation:

- means: the refugee speaks an exact self-presentation to present listeners;
- intended effect: those listeners may address this person using `Sable`;
- committed mutations: `ChangeIdentity::AdoptHandle` only if the speaker is
  choosing a new handle, followed by `ChangeIdentity::DiscloseHandle` or
  `ChangeKnowledge::Communicate` to exact listeners;
- derived projection: each listener's Eve surface displays the best identity
  handle that listener actually knows.

The speech never authorizes a global actor rename, and a display label never
becomes identity authority.

## Mutation authority envelopes

An admission policy compiles `mutation_authority_envelope.v1` from the current
snapshot. It contains exact permits, not broad roles:

- campaign revision and relevant component versions;
- originating subject and resolution procedure;
- permitted mutation kind and operation;
- exact subjects, routes, resources, propositions, channels, and relationships
  available to that operation;
- numeric and cardinality bounds;
- extraordinary-permission or approval receipts;
- effect ceiling and outcome-band binding;
- expiry and digest.

The envelope is capability-like but not transferable. It can authorize one
assessment outcome, one reaction appraisal, one strategic action, or one
administrative command. It cannot be rebased onto a new revision.

### Action-specific model schemas

The model never receives the complete world mutation schema. Ghostlight first
selects the component families relevant to the intended effect, then generates
a compact schema from the exact permits:

- if no resource is transferable, no custody-transfer branch exists;
- if the actor cannot reach a target, that target does not appear;
- if the actor knows no communicable proposition, communication is absent;
- if an arena has no collective actor, collective posture and speech are
  absent;
- if the outcome cannot create an entity, admission is absent;
- scalar fields carry local limits rather than universal ranges.

Each generated branch carries an opaque permit ID. The kernel resolves that ID
back to the exact permit and revalidates it against the snapshot. Stable
instructions and mutation definitions precede the dynamic permit table so
provider caching remains useful; the dynamic prompt contains only the state
needed for this decision.

Schema generation improves model ergonomics. It does not replace local
validation, and schema presence is not authority by itself.

## Validation and atomic commit

The commit primitive performs this sequence:

1. Verify campaign revision, resolution epoch when present, originating receipt,
   envelope digest, expiry, and idempotency.
2. Resolve every subject and component against the same snapshot.
3. Verify every mutation has a matching unused permit and remains inside its
   exact IDs, operations, bounds, and effect ceiling.
4. Validate local semantics: reach, custody, knowledge provenance, authority,
   component applicability, and lifecycle.
5. Validate batch semantics: no contradictory writes, no resource duplication,
   no dangling entity retirement, no duplicate population membership, and no
   hidden cross-actor private writes.
6. Apply the entire batch to an isolated in-memory overlay.
7. Re-run global topology, custody, population coverage, player-control,
   knowledge, and temporal invariants on the overlay.
8. Persist component rows, events, mutation receipt, command receipt, and
   required private proof records in one CultCache transaction.
9. Publish refreshed Eve/CultMesh projections after the transaction commits.

Any error returns the original state byte-for-byte. A malformed model output
gets one same-snapshot correction and then aborts. No valid sibling mutation
survives an invalid batch member.

## Admission policies are separate from physics

The shared algebra does not make every actor equally powerful. Each lane has a
different authority compiler:

- foreground fiction-first d20 assessment considers declared means, local
  affordances, opposition, permissions, stakes, and effect ceiling;
- reaction appraisal can write only the reacting subject's private components
  plus exact speech/action proposals;
- NPC initiative uses the same foreground assessment after initiative chooses
  an eligible proposal;
- strategic cells use horizon, graph reach, institutional authority, resources,
  and resolution-cell attribution;
- cohesive Gestalts may receive collective permits only from real collective
  authority;
- arena cells receive constituent permits and cannot emit one arena-owned
  mutation;
- compiler and approval commands may admit entities or topology only through
  explicit evidence and branch-local gap policy.

Different admission does not imply different mutations. A player and an
institution transferring the same medicine use the same custody mutation.

## Migration from the empirical vocabularies

The accumulated variants are test requirements, not discarded knowledge:

| Existing behavior | Algebraic lowering |
| --- | --- |
| actor move / member migration / Gestalt migration | `Relocate` plus membership change when population affiliation changes |
| actor equipment / institution resources / Gestalt resources / member equipment | resource subjects plus `TransferCustody` and `MutateResource` |
| actor or member condition | `ChangeCondition` |
| actor goal / member obligation | `ChangeCommitment` |
| actor or member relationship | `ChangeRelationship` |
| world clock and Gestalt pressure | pressure subjects plus `ChangePressure` |
| actor knowledge / Gestalt shared knowledge / member knowledge | `ChangeKnowledge` with exact proposition and knower |
| member memory / reaction memory | `ChangeMemory` |
| institution posture / strategic posture | `ChangePosture` |
| Gestalt promotion and folding | resolution change over one stable actor plus explicit component mutations only |

Migration order:

1. Publish types, component applicability, permits, batch validation, receipts,
   and property tests without changing live state.
2. Lower foreground assessment and NPC resolution directly to mutation batches;
   delete `apply_world_effect`.
3. Lower reaction Interpreters to actor-private permits and mutation batches;
   delete direct `ActorStateDelta` writes.
4. Lower strategic outcome generation to the same operations; delete
   `apply_strategic_outcome_effect` and string-resource helpers.
5. Classify named-person materialisation as a resolution transaction, remove
   aggregate mutation from folding, and invalidate the active cover. Complete.
   Normalizing remaining legacy member fields into component rows remains.
6. Lower travel, waits, fission, and bounded expansion. Classify initial
   publication as an empty-store creation transaction rather than a world
   mutation. Complete.
7. Migrate persisted campaign rows transactionally, quarantine ambiguous legacy
   resources or identities, and make the old campaign fields projection-only.
8. Remove legacy schemas from model boundaries. Retain schema readers solely
   for historical receipt inspection.

At every stage, the old and new writers cannot both be active for the migrated
lane.

## Agency corpus

Ghostlight does not declare the algebra complete from enum aesthetics. It owns
an inspectable `agency_attempt_case.v1` corpus compiled into a MessagePack
CultCache store. JSON Schema publishes the record shape; authored source may be
reviewed Markdown, but runtime and test fixtures load `.cc`.

Each case records:

- domain and scenario;
- minimal world fixture and exact subjects;
- means and intended effect;
- expected admission: admissible, impossible, or bargain-required;
- required mutation composition by outcome band;
- forbidden mutations and invariant witnesses;
- expected bargains for missing permission or affordance;
- whether foreground, NPC, strategic, and Gestalt resolutions should be
  physically equivalent;
- reviewer status and any genuinely missing primitive.

The initial target is at least 300 reviewed attempts: 25 each across social,
physical, investigative, economic, political, technological, extraordinary or
magical, collective, identity and privacy, population and migration,
infrastructure and logistics, and bodily or medical play. Cases include both
ordinary and adversarial attempts, partial successes, indirect means, consent
boundaries, rival actors inside arenas, and low-resolution background outcomes.

Completeness gates:

- every case maps compositionally, yields a comprehensible bargain, or names a
  genuinely missing primitive;
- no case falls through to a generic patch or arbitrary component write;
- the same physical change uses the same mutation in foreground, NPC, member,
  population, institution, and arena-constituent contexts;
- property tests prove atomicity, deterministic permit/schema generation,
  resource conservation, stable identity, exact knowledge ownership, population
  coverage, topology integrity, and no puppeting;
- the final 100 newly reviewed admissible cases require no new primitive. This
  is a stability signal, not a mathematical proof of completeness.

The Sable self-presentation and later rematerialization is a mandatory first
case. It must prove observer-relative identity projection without a global
rename or loss across Gestalt folding.

## Public contracts

Add:

- `typed_subject.v1`
- `world_component_ref.v1`
- `action_means.v1`
- `mutation_intent.v1`
- `mutation_permit.v1`
- `mutation_authority_envelope.v1`
- `world_mutation.v1`
- `world_mutation_batch.v1`
- `world_mutation_receipt.v1`
- `agency_attempt_case.v1`

JSON Schema is the catalog boundary. Canonical component rows, envelopes,
batches, receipts, and corpus fixtures use MessagePack-backed CultCache `.cc`.
JSON remains acceptable only at browser, MCP, and model-provider boundaries.

## Acceptance

The algebra cut is complete only when:

- no production path calls a legacy effect writer;
- every effectful command persists an exact mutation receipt;
- malformed, stale, over-broad, contradictory, or partially valid batches leave
  state, fictional time, and projections unchanged;
- background and foreground versions of the same action pass the same component
  invariants;
- arena rivals cannot speak or mutate as one subject and cannot acquire one
  another's knowledge;
- resource custody and quantity are conserved across transfers and failures;
- named people remain the same subjects through Gestalt merge, split, migration,
  folding, and rematerialization;
- Sable's disclosed handle appears only to entitled observers and survives the
  complete round trip;
- action-specific schemas omit impossible operations before inference;
- the agency corpus meets its coverage and stability gates;
- adversarial play through the typed Eve surface still proves the homepage
  promises after migration.
