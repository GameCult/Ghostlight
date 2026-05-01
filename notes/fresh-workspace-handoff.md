# Ghostlight Fresh Workspace Handoff

Do not trust this file for the exact live HEAD. Use git for volatile truth.

Do not continue implementation automatically from a rehydrate-only request.

## Immediate Re-entry Instruction

1. Read `state/map.yaml`.
2. Read `notes/ghostlight-current-system-map.md`.
3. Read `notes/ghostlight-implementation-plan.md`.
4. Run `npm run state:status`.
5. Restate the persisted next action before touching code or docs.

## Current Shape

Ghostlight now has the persistence spine plus the first architecture payload:

- canonical map
- scratch
- evidence ledger
- branch ledger
- compact re-entry note
- system map
- implementation plan
- Python helpers for state summary and compaction audits
- architecture notes for:
  - personality structure
  - signal classification
  - state distributions and prompt projection
  - Aetheria content generation targets
  - population scaling for socially dense authored slices
- first schema seam:
  - `schemas/agent-state.schema.json`
  - `schemas/agent-state.required-fields.json`
  - `examples/agent-state.call-of-the-void.json`
  - `tools/validate_agent_state.py`
- prompt projection and distillation docs:
  - `docs/architecture/prompt-projection-contract.md`
  - `docs/architecture/projection-distillation-plan.md`
- evaluation follow-up docs:
  - `docs/architecture/schema-future-mechanisms.md`
- Aetheria lore grounding docs:
  - `docs/architecture/aetheria-lore-grounding-architecture.md`
  - `docs/architecture/lore-grounding-digest-format.md`
- lore grounding seam:
  - `schemas/lore-grounding-digest.schema.json`
  - `examples/lore-grounding/historical-flashpoint.template.json`
  - `examples/lore-grounding/cold-wake-panic.ganymede-corridor.json`
  - `tools/validate_lore_grounding.py`
- Cold Wake story-lab seam:
  - `docs/experiments/cold-wake-story-lab.md`
  - `examples/agent-state.cold-wake-story-lab.json`
  - `examples/projections/cold-wake-story-lab.jsonl`
  - `examples/projections/cold-wake-story-lab.pretty.json`
  - `experiments/cold-wake-story-lab/scene-03-maer-veyr.qwen3-5-9b.capture.json`
  - `tools/build_cold_wake_story_lab_fixture.py`
- Cold Wake fixture note:
  - `docs/architecture/aetheria-cold-wake-training-fixture.md`
- first projection example seam:
  - `schemas/projection-example.schema.json`
  - `examples/projections/call-of-the-void.scene-broken-taxi-oz.jsonl`
  - `examples/projections/call-of-the-void.scene-broken-taxi-oz.pretty.json`
  - `tools/validate_projection_examples.py`
- visitor-facing README refreshed to explain Ghostlight's goals

There is still no full runtime yet. The live machine is design, state
discipline, a cleaner repo boundary, the first executable state contract, and a
documented projection path.

## Current Direction

Use Ghostlight first for:

- content generation inside the Call of the Void slice described in
  `AetheriaLore`
- dialogue scaffolding and procedural drama for narrated Aetheria timeline
  stories

Cold Wake is now selected as the first historical grounding fixture because it
is authored pre-Elysium lore. It is not the current game target. Treat it as
training feedstock for projection and dialogue scaffolding.

## Current Next Action

Tighten the Cold Wake story-lab projection prompt around unresolved personhood
suspicion, then run the next response turn through Qwen and keep accepted or
rejected outputs as training data.

Cat/Oz remains useful as an Elysium procedural mechanics fixture, but grounded
training data should start from authored historical lore rather than gameplay
era Elysium outcomes.

The evaluation follow-up fixed the immediate v0 schema issues:

- `behavioral_dimensions.warmth` became `interpersonal_warmth`
- `voice_style.warmth` became `verbal_warmth`
- Cat now has a perceived-state overlay for Oz
- Cat and Oz now have relationship summary memories
- `event-drone-lie` now includes observed exchange, private interpretations,
  and event effects

Completed projection path items:

- define `schemas/projection-example.schema.json`
- create reviewed Cat/Oz projection examples
- choose Cold Wake Panic as the first authored historical flashpoint
- fill the first draft lore grounding digest:
  `examples/lore-grounding/cold-wake-panic.ganymede-corridor.json`
- define the Cold Wake story lab as a four-scene arc with Maer Tidecall as
  protagonist
- add the first Cold Wake projection example and first Qwen response capture
- widen projection from dialogue-only turns to response turns that can include
  action, silence, withdrawal, violence, or mixed beats

Remaining projection path:

- revise the first Maer/Veyr projection so unresolved personhood suspicion stays
  unresolved in Qwen output
- run additional story-lab response turns through Qwen
- build a deterministic speaker-local input slicer
- use a frontier teacher model to generate/audit projection artifacts
- train a smaller student projector only after the artifact schema, input
  slicer, and evaluator stabilize

## Warnings

- Do not let this repo become a mirror maze of notes that say the same thing
  in slightly different outfits.
- Keep canonical state small. If evidence starts turning into an activity feed,
  cut it back.
- Keep the prompt-projection doctrine intact: prompt text is projection, not
  truth storage.
- Do not accidentally re-promote the older corridor-crisis idea into a game MVP
  because an old doc had a strong chin and good lighting.
- `notes/evaluation.md` may exist as an untracked user-supplied file for the
  next session. It was intentionally left unread and uncommitted during the
  compaction prep.
