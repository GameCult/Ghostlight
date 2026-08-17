# Ghostlight Current System Map

GhostlightDungeon is the active hosted runtime. Its authority map is
`docs/architecture/ghostlight-dungeon-mvp.md`. The implemented Rust daemon,
CultCache campaign stores, CultMesh/Eve surfaces, browser lowerer, and Starfire
process/release scripts are the live machine; the older validated artifact
seams remain regression evidence only.

## GhostlightDungeon target flow

```text
Vault evidence receipts + typed campaign snapshot
  -> permissioned projection
  -> Persona receives private lived narrative only
  -> Interpreter emits typed deltas and action proposals
  -> deterministic gates + expected revision
  -> one WorldCommand enters the campaign mailbox
  -> atomic CultCache commit + receipt
  -> parallel affected-participant appraisal
  -> CultMesh/Eve projection
```

Ghostlight owns the generalized projection organ. Epiphany and other consumers
own their canonical Persona state and consequence commits.

Build provenance has one owner: `crates/ghostlight-dungeon/build.rs` binds the
exact source commit into the binary. Its inputs are either the release tool's
explicit clean-tree commit or Git HEAD; it watches the actual symbolic ref,
HEAD reflog, and packed refs so an ordinary commit invalidates Cargo's cached
value. Health derives its commit from that embedded value. Release manifests,
launch scripts, and CultCache deployment receipts are verifiers and records,
not alternate writers. Local runs and immutable releases use this same binding,
and the regression test compares it to the checkout's live `git rev-parse
HEAD`. A mismatch blocks activation rather than being repaired after launch.

Away-time agency follows a separate proposal/commit seam:

```text
exact campaign revision
  -> flash-model six-axis resolution demand
  -> connected agency graph + budget/pins/leases/detail debt
  -> cohesive or arena simulation-cell cover
  -> one private Projector/Persona/Interpreter membrane per cell
  -> model proposes exact constituent-attributed actions or explicit inaction
  -> runtime binds complete cell membership + world/resolution revisions
  -> WorldKernel validates cover, receipts, knowledge, scope, topology, and bounds
  -> AdvanceStrategicTick through the campaign mailbox
  -> one atomic campaign/event/news/cover/appraisal commit
```

The model owns no tick mutation. A provider failure or invalid proposal leaves
the campaign revision and world time untouched. Background inference checks
live-turn pressure before launch and again before commit; return catch-up uses
the same command path with player-turn priority.

A live request also interrupts an in-flight scheduler wave. Dropping that wave
aborts its parallel cell tasks before they can launch later Persona stages, and
a shared/exclusive commit gate makes scheduler commit impossible while any live
request is active. Return catch-up is intentionally exempt because it is part of
the live request and must finish before the requested player action.

Resolution-demand focal IDs are salience hints, not partition commands. They
cannot create mandatory singleton cells or exceed the configured budget. Cell
Projectors receive decision-relevant situation state; cell Interpreters receive
exact permissions and the narrative products. Membership and revision bindings
are derived by the runtime, so a model is never asked to copy an invariant that
the planner already owns. Stable prompt prefixes are deliberately placed before
dynamic state, and provider receipts expose per-attempt token/cache usage plus
bounded local validation failures.

Arena projection preserves spatial partitions as well as identity partitions.
Each remote view carries its actual location; a relation grants potential
reach, not co-presence. Persona turns cannot stage direct sight, speech, or
response across locations unless the lived stream establishes a shared place
or communication channel. Interpreter activity labels then describe the
narrow attempted act, while hoped-for success remains outside the effect.
Ghostlight deterministically prepends the exact name/location partition to the
learned Projector's narrative, so model omission cannot erase those boundaries.
Only supplied cell constituents and selected member exceptions may own a
perspective. Event mentions do not transfer a person into another cell, and an
unnamed or unsupplied entity cannot become an activity target merely because a
Persona found it plausible.

Institution slices carry committed `current_posture` separately from unresolved
pressure. Projectors present the posture as existing state; Interpreters and
validators compare any new commitment directly against it. The old overloaded
`pressures[0]` convention is not an authority path.

Model-facing actions contain one owner, `subject_id`. Nested effect owner IDs
are runtime-derived during binding and exist only in the canonical downstream
proposal. The model cannot disagree with itself about `member:mira-venn` versus
raw `mira-venn`, and the prompt/output contract spends fewer tokens copying
invariants Ghostlight already owns.

Cell perspective attribution is runtime-bound too. The Projector returns one
to the cell action limit `subject_id → narrative` segments; the schema admits
only exact constituents and selected member exceptions. Ghostlight rejects
duplicates, requires the debt-selected focus, sorts segments stably, and
lowers them to natural name/location headings before the Persona sees them.
The Persona receives prose only. An invented Markdown section can no longer
turn a mentioned outsider into a decision-making perspective.

Gestalt background choices use three distinct typed paths. Pressure transitions
change exact unresolved markers. `gestalt_activity` records an attributed
preparation, coordination, investigation, recruitment, obstruction, trade, or
communication attempt against subjects connected by an explicit agency
relation or exact shared location without claiming the outcome. A selected
dormant member can be addressed by their durable ID; this does not union them
into the source population. The kernel derives the event text and exact
participant IDs; the arena and model prose own neither.

`gestalt_migration` records a collective decision by one exact active leaf to
move along an explicit migration relation and reachable route. WorldKernel
changes the leaf's home location and agency-profile location together. The
destination population remains a separate canonical subject, and named member
deltas are not carried by implication. Approval-gated fission creates
destination/stayer cohorts; the same migration primitive moves any resulting
leaf regardless of lineage depth. A local `investigate` activity may omit a
target when it examines the exact current environment; this admits ordinary
roles such as an unnamed clerk without inventing an actor or claiming an
answer.

Salient dormant members have their own `member_activity` path for ordinary
local attempts. It uses the same bounded verbs, the member's exact location,
their current leaf plus explicitly related or exactly co-located targets, and
their own state/channel permissions. Only the capped salient member exceptions
selected for the cell enter any prompt or can be named there. Migration and
activity conflict on one member key. A destination population cannot inherit
the person's offer, speech, or decision merely because the cover placed them
in the same arena.

A named member's explicit commitment to board, depart, travel, or join a
supplied destination maps to `member_migration`, even when the first narrated
motion is only entering a queue. `member_activity: prepare` is valid only while
departure remains unchosen. The Interpreter and semantic verifier share this
distinction.

Population scale uses reversible individuation:

```text
gestalt baseline + existing member delta, or a first-relevance identity proposal
  -> WorldKernel validates gestalt/member/revision/location
  -> atomic durable member delta + temporary ActorState
  -> ordinary Persona appraisal and world commands
  -> expired relevance lease outside player perception
  -> atomic fold into the member delta; active slot removed
  -> gestalt receives strategic ticks without erasing the individual
```

The temporary actor slot is derived. It never owns identity, relationship,
memory, equipment, injury, or obligation state.

Automatic presence planning projects only active leaves at the player's
location, dormant members whose exact location matches, and materialized
members eligible for folding. The kernel rejects inactive hierarchy nodes and
location mismatches. Nested fission and migration therefore change which
baseline a durable delta composes with, never who the person is.

The active cell budget and provider concurrency limit are separate controls.
Budget and pins increment `resolution_epoch` without advancing world revision or
fictional time. Provider concurrency increments only
`provider_configuration_epoch`; it batches the same cover and cannot repartition
the world. See `docs/architecture/ghostlight-multiresolution-agency.md`.

The browser command boundary is deliberately smaller than `WorldCommand`.
Authenticated HTTP admits only player-owned Speak, unfilled Assess, Attempt,
and Wait requests. Actor identity must match the campaign player. Compiler
approval has its own route. Strategic ticks, region commits, gestalt presence,
reaction waves, NPC initiative, and campaign creation are internal mailbox
commands and cannot be invoked through `/api/command`.

That boundary also projects less state outward than the kernel returns inward.
Player HTTP responses contain only assessment, public commit/roll receipts, and
narration. Canonical campaign state and spoiler-bearing actor or institution
state are operator-only. Informational rolls add only their exact previewed
finding to the acting character and a provisional branch fact. The assessor
deterministically binds typed findings into visible stakes before validation,
so formatting is not delegated to a correction attempt. The compiler
classifies each retrieved source as direct seed, setting background, or excluded
before generation. Only direct-seed source text enters causal world compilation;
background and excluded sources remain coverage provenance and cannot donate
story incidents or cast.

Global strategic context has a separate non-causal lane. Two stable broad Vault
queries run alongside local retrieval, and a Flash extraction stage proposes at
most 32 remote institution names with one short durable mandate each. Local code
binds each mandate string to an exact witness that also names the institution.
Unsupported entries become summarized approval gaps and private receipt detail;
they do not become canonical institutions or canon-candidate records. Admitted
remote institutions receive deterministic coarse profiles with distinct
authority and explicit unknown facets. The Pro agency stage profiles only local
actors, populations, and institutions where semantic subdivision is useful.

Compiler, expansion, fission, fork, and reset routes follow the same projection
rule. Approval previews expose the public decision surface—topology, cast,
pressures, player role, source-use coverage, gaps, and assumptions—without raw
campaign, evidence, model receipts, private goals, memories, or relationships.

Relationship documents in the schema catalog are revision-bound projections
of actor-owned relationship maps; they are not a second relationship writer.
Vault manifests summarize the exact provider/source/authority/temporal lanes
covered by evidence receipts and do not own Vault content. Strategic tick and
gestalt materialization receipts are different: they are atomic commit
companions binding the generic world commit to the causal model output or
baseline/member presence transition.

## Control Flow

1. Rehydrate from `state/map.yaml`, `notes/fresh-workspace-handoff.md`, this
   file, and `notes/ghostlight-implementation-plan.md`.
2. Run `npm run state:status`.
3. Work one bounded organ.
4. Validate the seam that matters.
5. Persist only belief-changing evidence.
6. Commit and push completed work.

Model acceptance also measures useful work per token. Stable contracts precede
revision-bound context for cache reuse; each stage receives only the state it
can legitimately judge; provider receipts expose cache, prompt, completion, and
validation cost per attempt.

## Core Surfaces

- `state/map.yaml`: canonical mission, boundaries, live architecture, next action.
- `state/evidence.jsonl`: distilled belief-changing evidence only.
- `state/evidence.archive.jsonl`: older evidence preserved for archaeology.
- `state/corpus-coverage.json`: accepted/planned fixture coverage ledger.
- `notes/fresh-workspace-handoff.md`: compact re-entry packet.
- `notes/ghostlight-implementation-plan.md`: near-term implementation sequence.
- `docs/architecture/`: detailed contracts and rationale.

## Active Artifact Pipeline

```text
Aetheria lore/source context
  -> lore grounding digest / source notes
  -> canonical agent and scene state
  -> projected local context
  -> responder packet
  -> sandboxed responder output
  -> review and leakage audit
  -> observable event record
  -> participant-local appraisal receipts
  -> mutation receipt
  -> updated scene/world/social state
  -> initiative schedule updates readiness, reaction windows, and next actor
  -> sandboxed coordinator continuity and next-beat plan
  -> meta-coordinator review, wiring, and labeled interventions
  -> branch compiler materializes Ink + sidecar + visual plan + compiler notes
  -> IF artifact reviewer audits consequence, fold, and visual continuity
  -> narrative/lore/spatial/visual reviewers audit player-facing quality
  -> accepted fixture / future training corpus + optional illustrated replay collateral
```

Responder packets are one seam, not the whole project. The branching scene path
also requires coordinator receipts, branch compiler artifacts, and independent
review before a fixture is accepted.

Tone is a live seam. The system should preserve Aetheria's range instead of
defaulting to dry technical crisis prose: comic warmth, domestic routine,
ritual memory, weird bureaucracy, noir suspicion, wonder, horror, poetic dread,
and procedural systems pressure are all valid when source-grounded and
character-local. Adams/Pratchett-style wit-with-stakes is a useful default
touchstone for counterbalancing bleakness without dissolving consequence.

Visual replay is a presentation seam, not a core social-model training seam.
Illustrated IF fixtures need click-through sections with stable
`visual_scene_id` anchors, stable visual character refs for recurring named
characters, a global style cue when the visual language matters, imagegen-ready
base prompts, character visibility and stance controls, and branch/state
modifiers. This data belongs in a separate `.visual.json` artifact referenced
by the training sidecar, not inside the training annotation itself.

Generated images are not geometry authority. They can establish visual mood,
materials, and concept direction, but multi-angle illustrated IF needs a durable
`scene_set` source such as a hand-built 3D blockout, procedural scene file, or
layout asset with named cameras and staging slots. For those scenes, imagegen is
the painter/render pass over a camera-specific blockout, not the architect of
the room.

## Important Contracts

- Agent state: `schemas/agent-state.schema.json`
- Projection examples: `schemas/projection-example.schema.json`
- Lore grounding: `schemas/lore-grounding-digest.schema.json`
- Projected local context: `schemas/projected-local-context.schema.json`
- Coordinator artifacts: `schemas/coordinator-artifact.schema.json`
- Responder packets: `schemas/responder-packet.schema.json`
- Responder outputs: `schemas/responder-output.schema.json`
- Event records: `schemas/event-record.schema.json`
- Participant appraisals: `schemas/participant-appraisal.schema.json`
- Reviewed mutations: `schemas/reviewed-mutation.schema.json`
- Scene-loop bundles: `schemas/scene-loop-bundle.schema.json`
- Initiative schedules: `schemas/initiative-schedule.schema.json`
- Ink branch contract: `docs/architecture/ink-branching-scenes.md`
- Illustrated IF visual pipeline: `docs/architecture/illustrated-if-visual-pipeline.md`
- Training stages and corpus gates: `docs/architecture/training-plan.md`
- Corpus coverage ledger: `docs/architecture/corpus-coverage-ledger.md`

## Current Live Examples

Pallas Species Strikes is the active reference-only story fixture:

- `examples/lore-grounding/pallas-species-strikes.awakened-labor.v0.json`
- `examples/agent-state.pallas-species-strikes.v0.json`
- `examples/coordinator/pallas-species-strikes.branch-and-fold.v0.json`
- `examples/ink/pallas-species-strikes.branch-and-fold.v0.ink`
- `examples/ink/pallas-species-strikes.branch-and-fold.v0.training.json`
- `examples/visual/pallas-species-strikes.branch-and-fold.v0.visual.json`
- `experiments/pallas-species-strikes/pallas-species-strikes.branch-and-fold-clean-run.v0.md`
- `experiments/pallas-species-strikes/pallas-species-strikes.visual-scene-review.v1.json`

The fixture is not training-ready data for Ghostlight's model stages. It was
coordinator-authored without the exact projected-context, sandboxed responder,
appraisal, mutation, relationship-update, and true branch-compiler receipts
those stages require. Keep it as a reference for story shape, grounding,
branch-and-fold presentation, visual planning, and future fixture expectations.

`pallas-training-loop-v0` is the first separate training-shaped derivative. It
rebuilds only the Kappa threshold beat through actual receipts:

- scene digest and initial state under `examples/training-loops/pallas-training-loop-v0/`
- projected contexts under `examples/projected-contexts/`
- responder packets under `examples/responder-packets/`
- raw no-fork subagent captures under `experiments/responder-packets/`
- event, appraisal, mutation, review, and bundle receipts under `examples/training-loops/pallas-training-loop-v0/`
- clean readable transcript under `experiments/pallas-training-loop-v0/`

It is training-shaped for projector, responder, event resolver, participant
appraiser, mutator, relationship/perception updater, and coordinator/story
runtime. It is not branch compiler, IF reviewer, visual, or accepted full-fixture
coverage data.

`lucent-hostage-feed-v0` is the current open-ended training-loop draft. It
tests the same receipt-chain organs in a less reference-bound scene: a Lucent
Media hostage-feed negotiation inside the media-eye counterweight of a tethered
station staring down at a metropolis bubble.

The Lucent bundle includes:

- scene digest, initial state, and branch surface under `examples/training-loops/lucent-hostage-feed-v0/`
- projected contexts under `examples/projected-contexts/`
- responder packets under `examples/responder-packets/`
- raw no-fork subagent captures under `experiments/responder-packets/`
- event, appraisal, mutation, review, and bundle receipts under `examples/training-loops/lucent-hostage-feed-v0/`
- clean readable transcript under `experiments/lucent-hostage-feed-v0/`
- fixture materializer at `scripts/materialize_lucent_loop.py`
- initiative schedule example at
  `examples/initiative/lucent-hostage-feed-v0.turn-20.initiative.json`

Only the fifteen post-restart turns from `turn-06` through `turn-20` are counted
as training-clean responder receipts. The earlier exploratory opening turns are
setup/rehearsal context. The loop demonstrates state carryover: feed evidence
externalization creates breathing room, breathing room creates a bench-edge
substitution, the substitution enables a process-note admission, the admission
enables direct hostage leverage to end, and release folds into safe-line
confirmation plus non-contact protective handoff.

The current coordinator lesson is explicit: nudge through game-ergonomic state
levers rather than forcing plot. Safe lines, evidence locks, security holds,
handling categories, posture constraints, route access, clocks, public proof
objects, and resource pressure are the elastic strings. Actors pull against
those strings from local state; the coordinator records observed action and
folds consequences through reviewed mutation.

The coordinator must now be treated as a sandboxable organ in future
training-shaped loops. Codex remains the meta-coordinator: it prepares visible
input, launches the coordinator worker, reviews the output, labels repairs, and
wires structured state between organs. Raw coordinator training data should come
from sandboxed coordinator prompts, not from omniscient chat steering.

The initiative scheduler is now an explicit mechanical seam. It decides when an
actor is eligible to act from initiative speed, recovery, load, status, and
reaction windows; it does not decide what the actor wants or how other
participants appraise the event. Every affected participant still appraises and
mutates before the scheduler selects the next projected actor.

VoidBot's newer heartbeat routine is the reference shape for turning that seam
into a living swarm routine. For Ghostlight, the swarm is the cast: the same
heartbeat initiative law selects characters taking turns within a scene, while
offstage or maintenance heartbeats handle rumination, sleep consolidation,
memory resonance, and reviewed upkeep. See
`docs/architecture/voidbot-routine-adoption-plan.md`.

## Pruned Receipts

Old prototype fixture receipts and model-runner files have been removed
from the live workspace. Their useful lessons belong in the architecture docs,
state map, evidence ledger, and git history. Do not reintroduce deleted receipt
paths as active state surfaces.

## Missing Or Incomplete Organs

- World compiler browser flow, selected-opening/role path, live provider
  acceptance, material-gap approval, on-demand destination expansion, and
  canon-candidate persistence
- Deterministic speaker-local input slicer
- Full prompt renderer
- Classifier/appraiser pipeline
- Relationship/perception updater
- State mutator model
- Student projector
- Runtime integration for initiative scheduling beyond validated artifacts
- Branch compiler implementation beyond coordinator-authored fixtures
- IF artifact reviewer implementation beyond manual/frontier review
- Visual scene continuity reviewer implementation beyond prompted specialist review
- Full scene/event loop implementation
- Culture prior engine
- Automatic promotion of branch outcomes into canonical state
- Corpus coverage ledger expansion beyond the current draft Pallas and Lucent
  receipt-chain rows

## Current North Star

Generate high-quality, sandboxed, source-grounded branching-scene samples that
can later train specialized Ghostlight models while staying usable by a game
engine: exact inputs, exact outputs, provenance, review labels, state mutation
receipts, branch compiler artifacts, IF review findings, visual scene plans, and
clear branch consequences. The samples should also train tonal control: a
fixture needs an intentional prose mode and enough ordinary life for consequence
to matter.

Use `npm run coverage:status` before choosing new fixtures so the corpus grows
by coverage need instead of proximity to the nearest interesting disaster.
