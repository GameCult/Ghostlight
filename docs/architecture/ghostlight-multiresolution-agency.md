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
reachable destination leaf. At any active leaf, the same member may instead
emit a `member_activity` from their exact location toward their current
population, an explicitly related subject, or another selected named person or
active subject at that exact location. Only the capped salient member
exceptions exposed to the cell may appear as named targets in its prompt;
canonical validation still checks the target's identity and current location.
Migration and ordinary activity share one per-member strategic action slot.
This lets a refugee who mattered during
a crisis choose to travel, offer work after settling, and later rematerialize
as the same person without forcing every refugee to consume an active Persona
cell or giving their agency to the destination Gestalt.

The membrane distinguishes a chosen journey from preparation for one. When a
member commits to board, depart, travel, or join a supplied destination within
the strategic horizon, the Interpreter must emit `member_migration`; approaching
the queue is not allowed to erase that commitment into `member_activity`.
Preparing applies only while departure remains unchosen. The semantic verifier
checks both directions so neither consideration becomes travel nor chosen
travel disappears into local activity.

Presence projection is location- and lineage-bounded. The planner sees only
active gestalt leaves at the player's location, dormant members whose exact
current location is there, and materialized members that might be eligible to
fold. Inactive ancestors and remote dormant rosters do not enter the prompt.
The kernel independently rejects individuation from an inactive leaf,
promotion outside the member's exact location, and automatic reconciliation
outside the player's active location. A direct internal command may preserve a
named offscreen actor, but model-driven presence reconciliation cannot teleport
one into view. Existing member deltas are preferred when their relationship,
memory, obligation, or goal makes an ordinary encounter a meaningful callback.
The player actor's reciprocal relationship to each nearby dormant member is
projected beside that member's exact delta. Presence planning is scene casting,
not roster cleanup: an earned local callback may surface without the player
asking for the person, while an incompatible event or absence of durable
callback evidence may still produce no promotion. The model only proposes the
cast; `WorldKernel` validates exact member, gestalt, location, and versions
before materialization. Live harnesses persist the private presence preflight
even when their acceptance assertion fails.

## Canonical population migration

Population fission and population movement are separate decisions. Fission
answers which non-overlapping cohort exists. Migration answers where one active
cohort goes. Simulation aggregation answers how much inference attention that
cohort receives after arrival. None of those operations is a canonical merge.

- **Owner:** `WorldKernel` exclusively owns an active gestalt leaf's
  `home_location_id` and the matching agency-profile location set.
- **Inputs:** one exact gestalt-attributed proposal, an active source leaf, an
  explicit migration relation to an active destination population leaf, a
  reachable destination, and the unchanged world and resolution revisions.
- **Outputs:** the source leaf and its agency profile move atomically, their
  versions advance, and one attributed population-migration event records the
  source and destination locations and populations.
- **Derived state:** the resolution cover may later place the migrant cohort
  and destination population in one cohesive or arena cell. That cover does
  not change either population's identity, knowledge, authority, or lineage.
- **Forbidden writers:** an arena, destination population, Projector, Persona,
  Interpreter, scheduler, or relation cannot move a population. A population
  move never carries named member deltas by implication; each named person's
  exact location and membership remain unchanged until that person acts.
- **Shared paths:** every active leaf uses the same transition, whether it is a
  root population or a child many levels below it. Approval-gated fission may
  first create destination, stayer, and `other/unknown` cohorts; each resulting
  leaf can then migrate independently.
- **Cut line:** collective departure is neither `prepare` nor
  `member_migration`. The former loses the chosen journey; the latter gives a
  population authority over a person. Canonical population merging remains
  absent: arrival changes geography, while derived covers provide cheap local
  aggregation.

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

An appraisal has two attributed lanes: `actions[]` for proposed transitions and
`inactions[]` for exact subjects that deliberately hold, wait, or continue an
already committed course. The old cell-wide inaction string is gone. This
matters in a mixed arena: one institution can act while two rivals hold without
the Interpreter manufacturing repeated posture transitions merely to preserve
their separately voiced decisions. Inaction IDs use the same constituent or
selected-member permission set, carry a bounded reason, cannot duplicate, and
cannot also appear in the action lane. They are audit/fairness evidence and
never mutate the world. A cell with no actions must still contain at least one
attributed inaction.

An arena may contain simultaneous views from distant locations. The Projector
names each subject's actual location and keeps those views as separate scenes;
a graph relation or activity target grants permission to attempt contact, not
fictional co-presence. The Persona preserves those spatial boundaries unless
the lived stream explicitly establishes an exact shared location or
communication channel. Compression therefore cannot turn a regional arena
into one impossible meeting room.

Perspective attribution is runtime-owned. The learned Projector emits bounded
`subject_id → narrative` segments, where the subject ID must be one exact
constituent or selected member exception from the permitted slice. Ghostlight
then lowers those private structured segments into one natural lived stream,
adding exact names and locations plus a deterministic scene-boundary block. The
Persona receives only that lowered prose. It never sees IDs or schemas.

- **Owner:** the permitted cell slice owns which subjects may have an internal
  perspective; the Projector owns only the lived language inside one bound
  segment.
- **Inputs:** exact cell constituents, selected member exceptions, perceived
  events, locations, private state slices, and the shared snapshot binding.
- **Outputs:** one to the cell action limit unique, non-empty narrative
  segments, lowered in stable order with runtime-owned perspective labels.
- **Derived state:** headings, scene labels, and the combined lived stream are
  disposable projections and own no canonical fact or decision.
- **Forbidden writers:** Projector prose cannot add a perspective owner by
  inventing a heading, voicing a mentioned outsider, or speaking as an arena.
- **Shared paths:** cohesive and arena cells use the same segment binding;
  their mode changes narration guidance, never perspective authority.
- **Cut line:** raw freeform cell Projector output is no longer appended
  directly to the Persona stream. Only runtime-bound segments cross the
  membrane.

The deterministic block contains names and locations, not schemas or state
fields. A poetic omission therefore cannot erase geometry, and an invented
Markdown section cannot grant a subject agency before the Persona turn.

Perspective ownership is equally strict. Only the cell's supplied constituents
and selected member exceptions may receive an internal viewpoint or make a
choice. A named person mentioned by a perceived event but owned by another cell
remains an external observation. Personas may seek an unknown role or office,
but cannot conjure it into available contact; target-requiring effects need one
of the exact supplied subject IDs.

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

`subject_state_references` is the single derivation path for canonical
constituent references used by both prompt slicing and wave admission. For a
Gestalt it includes reachable destinations produced by active migration
relations, along with the destination population and location references. The
scheduler does not append a second opinion after the fact, so a reference
shown to the Interpreter cannot become invalid merely by reaching the wave
resolver.

`subject_id` is the sole model-facing owner attribution. Nested effect owner
IDs are absent from the model contract: Ghostlight derives `institution_id`,
`gestalt_id`, `actor_id`, or the raw member ID during binding. The canonical
`CellActionProposal` still carries those IDs for downstream validation and
receipts, but a model cannot make two copies disagree. This also removes
repeated tokens and one class of correction calls from every actionful cell.

Institution rows distinguish `current_posture` from unresolved `pressures`.
The committed posture is projected as already true and is the exact baseline
against which a new institutional commitment is validated. It is never hidden
in a generic pressure list, where a Projector could mistake a completed choice
for a dilemma still awaiting action.
Institution posture proposals are bounded to 240 characters in both the stable
Interpreter contract and local JSON Schema, matching the canonical validator.
The model sees the limit before generation instead of spending a semantic retry
on an otherwise valid but overlong commitment.

A committed strategic effect must produce an exact canonical consequence.
Institutions cannot re-adopt their current posture. Population pressure effects
carry an explicit lifecycle: `pressure_resolutions` must name exact current
pressure markers, and `pressure_additions` must introduce genuinely new
unresolved constraints, threats, obligations, or conditions. Completed action
prose is not a pressure.

Populations may instead propose a bounded `gestalt_activity`: prepare,
coordinate, investigate, recruit, obstruct, trade, or communicate. Its source
is an exact active gestalt leaf; its targets must be adjacent canonical agency
subjects through an explicit active relation or exact shared location. A
dormant named person is addressable as `member:<id>` without becoming gestalt
state. Its locations and channels must belong to the source's permitted slice.
The effect commits only an attributed attempted-activity event. It does
not claim successful discovery, recruitment, persuasion, delivery, exchange,
obstruction, preparation, or target response. Event records carry exact
gestalt participant IDs so later demand and perception can attribute the
attempt without treating an arena as an actor. A later resolved outcome needs
its own admitted effect.

Two local attempts may omit a canonical target while retaining the source's
exact current location. Targetless `investigate` seeks facts from the
environment or an unnamed ordinary role. Targetless `communicate` records the
source speaking, offering, requesting permission, or notifying such a role.
Neither form creates a listener, reply, acceptance, discovery, or target
agency. All other target-bearing activity uses exact canonical subject IDs.
Activities may include incidental motion around unnamed local features while
remaining inside the source's canonical location. This does not create
topology or establish arrival. Only a clear commitment to a different supplied
canonical location or population destination requires a movement effect.

The shared validators run in the Interpreter membrane, wave resolver, and
kernel. One gestalt can contribute at most one selected pressure transition or
activity per tick; deterministic priority resolution cannot let two effect
forms bypass that limit.

Named member exceptions use the same activity vocabulary through
`member_activity`. The event is attributed to `member:<id>` and the current
leaf; the member's delta, the source Gestalt, and any target remain unchanged.
An arena cannot translate the person's attempt into a collective action, and
the destination population cannot claim the person's voice. `member_activity`
and `member_migration` resolve under one exact member key, so only the
higher-priority compatible choice can commit in a wave.

The Interpreter maps natural attempts narrowly: speech, offers, requests, and
notices are communication; coordination requires an actual attempt to arrange
joint work; preparation is the source's own concrete work; investigation seeks
information; recruitment invites participation; trade offers exchange; and
obstruction attempts interference. `intended_effect` repeats the attempted act,
not the hoped-for outcome. This keeps the semantic verifier focused on agency
and consequence rather than guessing what a broad activity label meant.

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

Verification is per action, not a batch opinion. The model returns exactly one
ordered `action_index` and discriminated result for every candidate: `match`,
or `mismatch` with one enumerated mismatch class.
Ghostlight constrains the verdict count and index range in the stage schema,
rejects omissions, duplicates, reordering, and incoherent discriminators, and derives
the rejected set locally. This keeps one bad arena action from contaminating
independent constituents and prevents a global pass/fail field from
contradicting its own rejected-index list.

## Public state and controls

The schema catalog and CultCache stores publish:

- `agency_profile.v1`, `agency_relation.v1`, `gestalt_lineage.v1`;
- `resolution_policy.v1`, `resolution_pin.v1`, `resolution_demand.v1`;
- `simulation_cell.v1`, `resolution_cover.v1`,
  `resolution_plan_receipt.v1`, and `resolution_control_receipt.v1`;
- `cell_appraisal.v1`, `cell_action_proposal.v1`, and the internal atomic
  `resolution_wave_commit.v1` bundle;
- `gestalt_migration.v1` and `member_migration.v1` as distinct collective and
  individual movement contracts inside strategic plans;
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

The live Gestalt-dynamics harness has two modes. Its golden mode traverses
budgets 1 → 4 → 8 → 4 and proves independent population/member migration plus
automatic rematerialization of the same durable person. Its fairness mode keeps
the whole 24-subject setting in one arena for 24 total strategic ticks. It
requires every runtime-selected detail focus within the bound, rejects inactive
lineage parents as simulation subjects, and repeats the identity/callback check
after sustained compression. Both migration endpoints must be active leaves at
lineage depth two or greater.

Fairness acceptance treats a rejected provider wave the same way the daemon's
scheduler does: the failed pulse records its private trace and commits nothing;
a later pulse may try the same still-current revision again. The harness permits
a bounded, operator-selected number of these fresh pulses per desired committed
tick and reports every rejection in the final result. This does not expand the
single stage's one-correction rule or rebase stale output.

The callback and fairness claims are independently falsifiable. Strict live
mode requires Mira's Persona to choose resettlement and proves the later
callback without forcing that choice. Fairness mode may instead branch from the
exact committed campaign inside that strict golden's result, then spend all 24
new provider-driven ticks on budget-one debt rotation. The baseline path and
SHA-256 are recorded in the result; no migration command or Persona decision is
fabricated during fairness setup.

A deterministic diaspora test drives two durable people from the same
depth-two refugee leaf to two different depth-two destination branches in one
strategic plan. It proves exact effective capabilities, knowledge, goals,
memories, relationships, possessions, conditions, and obligations survive the
rebases; destination populations receive none of that private state. Only the
source and destination population version counters change. Each person can then
materialize from their new local baseline with the same name and private delta.

### Live pressure results

The 24-tick budget-one fairness run `gestalt-dynamics-fairness-a4ea28b-1`
covered all 24 canonical subjects with 24 distinct debt-selected foci. Its
accepted appraisals contained 77 attributed proposals: one population
migration, nine institution posture changes, 15 named-member activities, and 52
Gestalt activities. Consequence capping committed 45 actions from 15 background
subjects. Four
bad scheduler pulses were rejected without revision movement; fresh pulses at
the same current state recovered. Mira retained her exact private and effective
state and was automatically rematerialized at the end. The run took 641.578
seconds. Its committed baseline is bound by SHA-256 in the result.

The 24-faction scale matrix produced:

| Configured cells | Effective cells | Arenas | Committed actions | Seconds | Prompt tokens/action |
| ---: | ---: | ---: | ---: | ---: | ---: |
| 1 | 1 | 1 | 2 | 18.913 | 11,734.5 |
| 4 | 4 | 3 | 8 | 15.619 | 4,682.1 |
| 8 | 8 | 7 | 16 | 26.300 | 3,546.6 |
| 32 | 24 | 0 | 16 | 55.906 | 7,410.5 |

Budget 4 is the latency sweet spot in this fixture. Budget 8 reaches the global
16-consequence cap with the best cognitive efficiency. Resolving every faction
individually doubles prompt cost and wall time relative to budget 8, adds no
committed consequences under the cap, and increases whole-wave tail-failure
risk. Budget 1 remains a valid emergency compression mode, not an efficient
default.

Repeated fairness prompts reached a 47.07% aggregate provider cache-hit ratio;
Projector reached 75.45% and Persona 61.57%. Interpreter remained the dominant
cost at 247,578 prompt tokens and only 34.54% cache hits. Prompt/context work
should therefore target Interpreter permissions and stable-prefix layout before
shrinking the Persona stream.

Resolution control is edge-triggered. `active_cell_budget` owns
`resolution_epoch`; provider parallelism owns only
`provider_configuration_epoch`. An unchanged control value is rejected without
persistence, epoch movement, or cover invalidation. Acceptance harnesses issue
either control only when its value actually differs, so repeated strategic
ticks preserve the previous cover, leases, and churn history. Stress runs may
select cell budget and provider parallelism independently.

A lease preserves a cut, not invalid topology. Before reusing a previous cover,
the partitioner revalidates every multi-subject cell against the current agency
graph. Migration, relation changes, fission, or activation can therefore break
a leased cell and force replanning immediately. Cover validation reports empty,
duplicate, disconnected, content-ID, and false-cohesion failures separately.

Fairness accounting uses the same definition as canonical detail debt: an
explicit aggregate `detail_focus_subject_id` and every singleton cell both
receive direct resolution attention. A named focus is not required for a
subject already receiving its own complete Persona pipeline.

The lease-preserving budget-4 profile committed all 23 requested sustained
waves and recovered from 18 rejected provider pulses without partial mutation.
It then failed its separate callback assertion because the presence planner
returned an empty cast. This isolated a player-experience fault from the agency
invariants: Mira remained the same dormant person in the correct destination,
but continuity was not surfaced. The focused `gestalt-callback-6253f9f-1` run
then passed: after one fresh 24-subject arena wave, the planner promoted
`mira-venn` from `harbor-neighbors` without the player naming her, and the
kernel rematerialized the same private and effective state. The full run took
18.465 seconds. Its strategic membrane used 18,662 prompt and 1,096 completion
tokens; the additional presence decision used 2,504 prompt and 78 completion
tokens.

Institution posture is projected as `already_committed_posture`. It is durable
state already in force, not a new pressure or decision. Projector, Persona, and
Interpreter contracts all treat continuing or restating it as explicit
inaction; only a materially different commitment may emit an institution
posture transition. Local validation remains strict rather than silently
dropping a no-op.

The 31-wave budget-8 / provider-parallelism-4 profile
`gestalt-lease-b8p4-6253f9f-1` passed after 1,211.592 seconds. All 24 subjects
received direct attention. Its sustained appraisals made 199 accepted proposals
from 22 distinct subjects; consequence resolution committed 51 institution
postures, 110 Gestalt activities, one population migration, and 31 named-member
activities, plus the initial arena consequence. Fifteen rejected pulses left
the world unchanged and later recovered. The same Mira callback passed after
the full run.

The profile used 1,468,039 strategic prompt tokens and 165,725 completion
tokens, reached a 60.69% cache-hit ratio, and spent 7,567 prompt tokens per
committed consequence. Interpreter remained the largest stage at 737,148
prompt tokens but reached 67.18% cache hits; Projector reached 70.03%. The
presence decision added 2,606 prompt and 78 completion tokens. Live traces
showed that the remaining repeated-posture failures occurred when several
constituents in one arena held while others acted, motivating the attributed
inaction lane rather than another prompt-only prohibition.

The first attributed-inaction/provider-parallelism-8 comparison committed nine
waves, then exhausted its bounded retries at wave 10. It proved the new lane in
mixed appraisals—across the first nine waves, 72 cells emitted 63 actions and 93
attributed inactions, including 18 action-plus-inaction appraisals—but also
exposed an overprojection failure. The Interpreter invented roster-completion
inactions for subjects absent from the Persona turn, overflowing the
four-perspective limit, and once wrote a reason beyond the 240-character schema
bound. The contract now admits inaction only for a subject that explicitly
holds or waits in the Persona turn, forbids absence-as-inaction, and asks for a
reason of at most 160 characters while retaining the 240-character hard bound.

The run also exposes the next authority gap. Institution postures, population
migration, clocks, and history changed canonically, but most Gestalt activities
remain attributed attempts recorded as events. Over many ticks this can repeat
an unresolved pressure instead of producing enough typed resource, bargain,
relationship, or obligation changes. A strategic outcome resolver must own
those consequences after proposal attribution and before the atomic
WorldKernel commit; the Interpreter must not gain authority to declare its own
hoped-for outcome.

The Starfire deployment smoke ran a live provider-backed wave over three
eligible singleton cells, committed three offscreen events and three
channel-accessible news items in 19.55 seconds, and preserved the absent player.
That fixture proves deployed cell isolation and atomic admission. The
20-plus-faction budget matrix, not the singleton smoke, proves aggregation.
