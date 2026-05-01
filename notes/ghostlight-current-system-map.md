# Ghostlight Current System Map

Ghostlight does not have a runtime yet. The live system right now is the repo's
state and re-entry discipline.

## Current Control Flow

1. A user or agent enters the repo with a bounded question or implementation
   goal.
2. The session rehydrates from:
   - `state/map.yaml`
   - `notes/fresh-workspace-handoff.md`
   - this file
   - `notes/ghostlight-implementation-plan.md`
3. The session runs `npm run state:status` to recover the current summary, next
   action, and active subgoals.
4. Work happens in one bounded slice.
5. If understanding changes, the map is updated.
6. If future belief changes, distilled evidence is appended.
7. If compaction pressure rises, the helper audits state hygiene before the
   session banks the fire.

## Live Surfaces

- `state/map.yaml`
  - canonical experiment, objective, constraints, invariants, active subgoals,
    accepted design, and current status
- `state/scratch.md`
  - temporary working memory for one bounded organ
- `state/evidence.jsonl`
  - durable belief-changing records only
- `state/branches.json`
  - explicit hypothesis or branch tracking
- `notes/fresh-workspace-handoff.md`
  - re-entry packet
- `notes/ghostlight-implementation-plan.md`
  - larger-organ sequence
- `docs/research/`
  - research backlog and social-model notes
- `docs/architecture/`
  - world, state, and prompt-system architecture notes
- `docs/architecture/prompt-projection-contract.md`
  - contract for turning durable state into speaker-local pressure prose and
    prompt sections
- `docs/architecture/projection-distillation-plan.md`
  - plan for using a frontier teacher model to generate reviewed projection
    artifacts before training a smaller student projector
- `docs/experiments/cold-wake-story-lab.md`
  - four-scene Cold Wake writing experiment using Ghostlight projection as a
    story-generation crutch and training-data source
- `docs/architecture/schema-future-mechanisms.md`
  - parking lot for appraisal, transition, belief, action, conversation,
    resource, dyad, institution, memory, and embodiment mechanisms not yet
    promoted into the v0 schema
- `docs/architecture/aetheria-lore-grounding-architecture.md`
  - boundary and plan for grounding training data in authored Aetheria history
    while treating Elysium gameplay-era outcomes as procedural branch space
- `docs/architecture/lore-grounding-digest-format.md`
  - human-facing format guide for cultural, factional, institutional, role, and
    speaker-boundary lore digests
- `docs/architecture/aetheria-cold-wake-training-fixture.md`
  - human-facing note that reframes the recovered Cold Wake scenario as
    historical projection feedstock rather than the active product target
- `schemas/agent-state.schema.json`
  - v0 canonical agent-state contract for agents, relationships, events,
    scenes, perceived overlays, and dialogue context packs
- `schemas/projection-example.schema.json`
  - v0 reviewed projection artifact contract for speaker-local prompt
    projection examples
- `schemas/lore-grounding-digest.schema.json`
  - v0 contract for grounding historical fixtures in source-backed cultural and
    factional pressure fields
- `schemas/agent-state.required-fields.json`
  - required first-class variable names for canonical state and relationship
    stance
- `examples/agent-state.call-of-the-void.json`
  - first Call of the Void-flavored fixture for schema and projection work
- `examples/agent-state.cold-wake-story-lab.json`
  - first story-shaped historical fixture for Cold Wake projection work
- `examples/projections/call-of-the-void.scene-broken-taxi-oz.jsonl`
  - first reviewed Cat and Oz projection examples
- `examples/projections/call-of-the-void.scene-broken-taxi-oz.pretty.json`
  - pretty-printed mirror for inspecting the first projection examples
- `examples/projections/cold-wake-story-lab.jsonl`
  - response-turn projection examples for the Cold Wake story lab, including
    rejected and accepted Maer/Veyr prompt revisions
- `experiments/cold-wake-story-lab/`
  - Qwen response captures and reviews from the writing experiment
- `examples/lore-grounding/historical-flashpoint.template.json`
  - fillable template for the first historical Aetheria grounding digest
- `examples/lore-grounding/cold-wake-panic.ganymede-corridor.json`
  - first draft historical grounding digest, centered on Cold Wake Panic in the
    Ganymede/Jovian corridor
- `tools/ghostlight_state.py`
  - compact state inspection and evidence or branch updates
- `tools/ghostlight_prepare_compaction.py`
  - pre-compaction audit of required files, handoff content, and git hygiene
- `tools/validate_agent_state.py`
  - dependency-free validator for the current schema fixture invariants
- `tools/validate_projection_examples.py`
  - dependency-free validator for projection example JSONL records
- `tools/validate_lore_grounding.py`
  - dependency-free validator for lore grounding digest examples
- `tools/build_cold_wake_story_lab_fixture.py`
  - generator for the verbose Cold Wake story-lab state fixture

## What Does Not Exist Yet

- a classifier pipeline
- a prompt renderer
- a full projection training corpus
- a deterministic projection input slicer
- a student projection model
- a simulation/event loop
- a relationship engine
- a culture prior engine

The canonical agent-state schema, projection example schema, lore grounding
digest schema, first draft Cold Wake grounding digest, and first Cold Wake
story-lab Qwen captures now exist as v0 seams. The first Maer/Veyr projection
has an accepted v3 prompt/output pair that preserves unresolved claimant status.
The next implementation target is projecting another Cold Wake response turn
before building the deterministic input slicer.
