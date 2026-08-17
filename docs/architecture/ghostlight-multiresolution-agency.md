# Ghostlight Multiresolution Agency Graph

## Objective

Let a campaign contain the whole strategic setting without pretending every
person, faction, institution, and population can receive a separate model turn.
Ghostlight preserves canonical fine state and changes only the resolution at
which that state is simulated during a wave.

This document is the authority map for the implemented multiresolution organ.
The wider runtime map remains `ghostlight-dungeon-mvp.md`.

## Authority map

- **Owner:** `WorldKernel` owns canonical subjects, world revision, resolution
  controls, accepted fission, and atomic strategic-wave commits.
- **Inputs:** canonical actors, institutions, gestalt leaves, typed agency
  profiles and relations, resolution pins, current pressure demand, committed
  clocks/events, and the previous accepted cover.
- **Outputs:** a revision-bound `ResolutionCover`, its plan receipt, one
  appraisal per selected cell, exact constituent-attributed proposals, and one
  atomic strategic transition.
- **Derived state:** simulation cells, merge losses, content-addressed cell IDs,
  detail focus, and the active cover. None owns a person, population, secret,
  relationship, possession, or fictional fact.
- **Forbidden writers:** demand projection, graph construction, the partitioner,
  Projectors, Personas, Interpreters, scheduler, browser, and operator inspector
  cannot mutate canonical world state.
- **Shared paths:** scheduled ticks and return catch-up both compile the same
  `AdvanceStrategicTick` command. Budget and pin changes use the campaign
  mailbox at safe boundaries. Provider parallelism uses a separate
  configuration epoch and does not repartition or advance fiction.
- **Cut line:** the old flat strategic-plan proposer is gone. No active runtime
  path may tick a hand-picked list of factions outside the selected cover or
  union arena knowledge into a synthetic actor.

`ActorState`, `InstitutionState`, `GestaltPersonaState`, and durable gestalt
member deltas remain canonical owners. `AgencyProfile` and `AgencyRelation` are
typed partition inputs. `SimulationCell` is always disposable and rebuildable.

## Canonical population resolution

Known population detail is represented by non-overlapping active gestalt
leaves. A material split uses approval-gated `FissionGestalt`:

1. the compiler retrieves exact Vault evidence and produces a fission preview;
2. every requested enumerated facet gets a child and one `other/unknown`
   remainder is mandatory;
3. children inherit the parent baseline and own only later deltas;
4. member deltas are assigned to one child without rewriting identity;
5. the parent remains as inactive lineage rather than being destroyed;
6. the kernel validates the preview, evidence, versions, assignments, and
   residual child before one atomic commit.

Reversible simulation aggregation never invokes fission and never deletes or
reverses this lineage.

## Canonical individual migration

Resolution aggregation and population membership are separate authorities. A
cell may temporarily contain many populations, but that never changes which
population baseline a durable person is composed from. `WorldKernel` owns one
explicit membership transition for that purpose.

- **Owner:** `WorldKernel` owns a durable member's current active-leaf
  `gestalt_id`. The member delta owns their identity, memories, relationships,
  possessions, injuries, obligations, and personal departures from the current
  baseline. `GestaltLineage` owns immutable fission ancestry.
- **Inputs:** one exact member-attributed proposal, the source and destination
  active gestalt leaves, a typed migration relation, a reachable destination,
  and the campaign/resolution snapshot already bound to the strategic wave.
- **Outputs:** the same member identity attached to the destination leaf, with
  capability and knowledge deltas rebased so their effective personal state is
  bit-for-bit unchanged; one attributed migration event is emitted.
- **Derived state:** simulation cells, arena membership, member prompt
  selection, and rematerialized `ActorState` projections do not own population
  membership.
- **Forbidden writers:** a source gestalt, destination gestalt, arena, demand
  model, Projector, Persona prose, or migration relation cannot move a person.
  Only an action attributed to that exact durable member may propose the
  transition, and the kernel validates it.
- **Shared paths:** source and destination may be leaves at any depth under
  unrelated lineage trees. The same rebase primitive handles crisis camps,
  neighborhoods, departments, corporations, species populations, and later
  refinements.
- **Cut line:** changing `GestaltMemberDelta.gestalt_id` directly is forbidden.
  It would reinterpret the old delta against a different baseline and silently
  alter the person's knowledge, capability, or default goals.

A bounded set of salient dematerialized member exceptions may be projected
inside the cell containing their current gestalt. They remain separately
attributed people, not shared gestalt state. Selection favors unresolved
personal conditions, obligations, goals, and relationships to the player and
is capped so a large population does not dump its roster into the prompt. A
member may migrate only along an explicit active `migration` relation to a
reachable destination leaf. This lets a refugee who mattered during a crisis
make an offscreen choice, settle elsewhere, and later rematerialize as the same
person without forcing every refugee to consume an active Persona cell.

## Demand projection

A flash-model structured stage converts the committed event or strategic
horizon into weights for geography, ideology, authority, economy role,
species/body, and information. Focal subjects must be chosen from supplied IDs.
Local semantic validation permits one same-snapshot correction. Failure falls
back to the last accepted demand; the initial fallback favors geography and
authority.

Demand is relevance evidence. It cannot create profiles, pins, cells, or facts.

## Agency graph

Vertices are active canonical actors, institutions, and gestalt leaves. Edges
carry containment, command, membership, alliance, rivalry, trade, migration,
communication, coercion, and shared-location meaning. Subjects sharing a
location receive derived adjacency. Disconnected components receive a
resolution-only Steiner chain so the graph remains coarsenable; those bridge
edges assert no lore relation, common authority, proximity, or shared
knowledge.

The compiler must profile every non-player actor, institution, and gestalt
exactly once across all six axes. It compiles a bounded local play region and a
global skeleton of major powers, coarse regions, institutions, information
channels, and strategic pressures. Runtime migration derives profiles for old
flat campaigns without changing their world revision, actors, institutions,
gestalts, or fictional time.

## Budgeted connected cover

The player selects an active Persona-cell budget from 1 through 32, default 8.
The partitioner:

1. contracts valid `keep_together` pins;
2. forces focal, directly engaged, leased, initiative, targeted, and
   `minimum_individual_detail` subjects into singleton cells;
3. respects `keep_separate` pins and reports mandatory effective-budget
   overage rather than hiding a foreground subject;
4. starts at fine resolution and repeatedly merges the lowest-loss legal
   graph-adjacent pair with deterministic tie-breaking;
5. performs one connected boundary-refinement pass and accepts a move only
   when compression cost improves by at least five percent;
6. preserves a previous cover during its lease, or until a replacement improves
   the weighted objective by at least ten percent, unless a forced split or
   budget change applies;
7. validates complete unique coverage, connectedness, cell identity, mode, and
   epoch before the cover can enter a wave.

Compression cost is merge loss multiplied by the number of collapsed
trajectories. This prevents the objective from favoring one overloaded arena
merely because a singleton has zero local loss.

The normalized merge loss is:

```text
0.25 facet divergence
+ 0.20 hidden causal-boundary mass
+ 0.15 information-scope divergence
+ 0.15 spatial divergence
+ 0.10 clock/obligation divergence
+ 0.10 salience burial
+ 0.05 partition churn
```

Facet divergence is demand-weighted Jaccard distance. Spatial divergence uses
known locations, direct travel time, and the current strategic horizon.
Candidate scoring uses a deterministic mergeable representative sample for
bounded local work; exact authority, hostile relation, coverage, connectivity,
pins, and knowledge validation remain unsampled.

Union-by-size owns constituent-set merges. This is not merely an optimization:
it keeps the 1,000-subject path from degenerating into quadratic copying while
leaving stable merge ordering unchanged. The release property test completes
the 1,000-subject, budget-8 partition in approximately 50 ms on the development
host, under the 100 ms contract. Setting `GHOSTLIGHT_PARTITION_TRACE` enables a
development-only phase timing probe.

## Cell modes

- **Cohesive:** all constituents share one actual collective authority, contain
  no active hostile edge, and remain within information and behavioral
  divergence thresholds. The cell may speak and act plurally.
- **Arena:** constituents remain distinct or opposed. The cell receives an
  attributed polyphonic situation and may simulate interaction, but it has no
  actor ID and cannot speak, know, decide, or act as a collective.

Cross-faction aggregation is therefore an arena. Arena interpreters may emit at
most `min(4, 1 + ceil(log2(constituent_count)))` actions, each attributed to an
exact constituent. The kernel validates that subject's knowledge, information
channel, resources, authority, relationships, location, and reachable
destinations. Secrets are never unioned.

## Fairness and quiet-world agency

Every active profile carries persistent non-fictional `detail_debt`. Aggregate
representation increases debt; singleton or explicit detail focus clears it.
Each strategic wave reserves focus for the highest-debt subject. At budget 1,
the root arena retains the global situation while explicitly appraising that
subject. Deterministic clocks and obligations advance regardless of focus, and
every cell must emit explicit action or inaction.

With N subjects, deterministic debt rotation gives every subject direct
resolution attention within at most N strategic waves, absent a mandatory
foreground override.

## Persona-cell wave

Each selected cell runs one Ghostlight-owned
Projector → Persona → Interpreter membrane:

- cohesive Projectors expose genuinely shared lived state plus attributed
  exceptions;
- arena Projectors expose attributed constituent slices and no synthetic actor;
- Personas receive only the private lived narrative stream;
- Interpreters emit a typed appraisal and exact constituent effects;
- malformed or semantically invalid output gets one same-snapshot correction.

Physical provider concurrency batches these pipelines behind a separate
operator limit, default 8. Changing that limit increments only
`provider_configuration_epoch`; it preserves the selected cover, resolution
epoch, world revision, clocks, and fictional time.

All cell results must bind the same campaign revision and resolution epoch.
The kernel requires a Projector, Persona, and Interpreter receipt for every
cell, validates every proposal, resolves incompatibilities deterministically,
caps committed consequences at `min(16, 2 × effective_budget)`, and commits the
entire wave once. One missing appraisal, stale receipt, malformed stage,
invalid cover, or invalid effect aborts the wave without partial mutation.

Interpreter `intent` and `intended_effect` prose is proposal rationale only. It
never becomes canonical event narration. The kernel derives each committed
event summary from the exact admitted posture, pressure markers, route, or
membership transition after applying it. Strategic plans therefore carry no
model-authored summary field that can assert a consequence outside the typed
effect.

Recent events are projected per constituent. A cell receives an event only for
subjects that participated in it, occupy one of its locations, or possess one
of its public channels; arena streams retain those viewer attributions. There
is no global recent-event recap that can quietly teach every population the
same fact.

Interpreter prompts use a compact stable output contract plus an exact
per-subject permission table. The table names the only allowed effect kind,
state references, public channels, and destinations for each constituent or
member exception. Local validation repeats every slice-expressible constraint
before accepting the terminal appraisal, so a malformed pressure marker or
wrong subject/effect pairing receives the same-snapshot correction while the
kernel still performs the authoritative canonical check. Named-member
knowledge is never treated as an information channel; while dematerialized,
the member can publish only through channels owned by their current leaf
gestalt's agency profile.

A committed strategic effect must change canonical state. Institutions cannot
re-adopt their current posture. Population effects carry an explicit pressure
lifecycle: `pressure_resolutions` must name exact current pressure markers, and
`pressure_additions` must introduce genuinely new unresolved constraints,
threats, obligations, or conditions. Completed action prose is not a pressure.
The shared transition validator runs in the Interpreter membrane, wave
resolver, and kernel; repeated busywork and invented resolutions therefore
fail before mutation rather than generating another event that says nothing
happened.

Semantic correction feedback names the exact failed boundary and current
value. A repeated institution posture receives its present posture; an invalid
population transition receives its exact current pressure set and the failed
transition reason. The correction model is not expected to reconstruct a
validator from a generic rejection.

An actionful cell has a fourth private stage after the Interpreter: the
`cell_effect_verifier`. This cheap structured reader compares the natural
Persona decision with each attributed typed effect. It rejects reversals,
subject swaps, omissions, wishful outcomes, or lossy mappings—for example,
mapping “Mira gives her berth to somebody else” into a migration of Mira
herself. The rejected appraisal and concise rationale return to one
same-snapshot Interpreter correction. Inaction cells skip the call. The kernel
requires a valid verifier receipt for every actionful cell; an invalid or
missing verifier cannot satisfy the wave receipt gate. The verifier receipt's
snapshot binding includes a MessagePack content hash of the ordered action
bundle. The kernel recomputes it, so a verdict for another proposal at the same
world revision and cell cannot be replayed as authority.

## Public state and controls

The schema catalog and CultCache stores publish:

- `agency_profile.v1`, `agency_relation.v1`, `gestalt_lineage.v1`;
- `resolution_policy.v1`, `resolution_pin.v1`, `resolution_demand.v1`;
- `simulation_cell.v1`, `resolution_cover.v1`,
  `resolution_plan_receipt.v1`, and `resolution_control_receipt.v1`;
- `cell_appraisal.v1`, `cell_action_proposal.v1`, and the internal atomic
  `resolution_wave_commit.v1` bundle;
- `gestalt_fission_preview.v1` and approval-gated `FissionGestalt`.

The authenticated campaign laboratory exposes budget, effective overage, pins,
fission preview/approval, and a separate operator provider-parallelism control.
The Eve operator projection carries the agency graph, active cover, cell modes,
loss components, debt focus, pins, plan receipts, cell appraisals, and control
receipts. Browser and Eve surfaces use a composite interface version so
resolution-only and provider-only changes refresh without impersonating a world
revision.

## Verification

Property and golden tests cover budgets 1/4/8/32, complete unique connected
coverage, deterministic ties, mandatory overage, contradictory pins, fairness,
arena isolation, fission identity preservation, stale epochs, atomic waves, and
the 1,000-subject target. Pressure goldens preserve geography under blockade,
ideology under schism, workplace/economic role under strike, species/body and
transport under epidemic, and information boundaries under espionage.

The Starfire deployment smoke ran a live provider-backed wave over three
eligible singleton cells, committed three offscreen events and three
channel-accessible news items in 19.55 seconds, and preserved the absent player.
That fixture proves deployed cell isolation and atomic admission. The
20-plus-faction budget matrix, not the singleton smoke, proves aggregation.
