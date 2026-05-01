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
- `docs/architecture/schema-future-mechanisms.md`
  - parking lot for appraisal, transition, belief, action, conversation,
    resource, dyad, institution, memory, and embodiment mechanisms not yet
    promoted into the v0 schema
- `schemas/agent-state.schema.json`
  - v0 canonical agent-state contract for agents, relationships, events,
    scenes, perceived overlays, and dialogue context packs
- `schemas/projection-example.schema.json`
  - v0 reviewed projection artifact contract for speaker-local prompt
    projection examples
- `schemas/agent-state.required-fields.json`
  - required first-class variable names for canonical state and relationship
    stance
- `examples/agent-state.call-of-the-void.json`
  - first Call of the Void-flavored fixture for schema and projection work
- `examples/projections/call-of-the-void.scene-broken-taxi-oz.jsonl`
  - first reviewed Cat and Oz projection examples
- `tools/ghostlight_state.py`
  - compact state inspection and evidence or branch updates
- `tools/ghostlight_prepare_compaction.py`
  - pre-compaction audit of required files, handoff content, and git hygiene
- `tools/validate_agent_state.py`
  - dependency-free validator for the current schema fixture invariants
- `tools/validate_projection_examples.py`
  - dependency-free validator for projection example JSONL records

## What Does Not Exist Yet

- a classifier pipeline
- a prompt renderer
- a full projection training corpus
- a deterministic projection input slicer
- a student projection model
- a simulation/event loop
- a relationship engine
- a culture prior engine

The canonical agent-state schema and first projection example schema now exist
as v0 seams. The next implementation target is the deterministic input slicer
that emits speaker-local state before a renderer turns that state into compact
pressure prose.
