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
  - `docs/architecture/sequential-agent-runtime.md`
  - `docs/architecture/ink-branching-scenes.md`
- evaluation follow-up docs:
  - `docs/architecture/schema-future-mechanisms.md`
- Aetheria lore grounding docs:
  - `docs/architecture/aetheria-lore-grounding-architecture.md`
  - `docs/architecture/lore-grounding-digest-format.md`
  - `docs/architecture/qwen-invocation-notes.md`
  - `docs/architecture/projected-local-context.md`
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
  - `experiments/cold-wake-story-lab/the-narrowest-possible-margin.md`
  - `experiments/cold-wake-story-lab/scene-03-maer-veyr.qwen3-5-9b.capture.json`
  - `experiments/cold-wake-story-lab/scene-03-maer-veyr.qwen3-5-9b.v2.capture.json`
  - `experiments/cold-wake-story-lab/scene-03-maer-veyr.qwen3-5-9b.v3.capture.json`
  - additional scene 1, 2, 3, and 4 Qwen captures under
    `experiments/cold-wake-story-lab/`
  - `tools/build_cold_wake_story_lab_fixture.py`
  - `tools/run_qwen_projection.py`
  - `tools/validate_qwen_captures.py`
- Ink branching seam:
  - `examples/ink/cold-wake-sanctuary-intake.ink`
  - `examples/ink/cold-wake-sanctuary-intake.training.json`
  - `examples/ink/cold-wake-sanctuary-intake.qwen-draft.ink`
  - `examples/ink/cold-wake-sanctuary-intake.qwen-draft.training.json`
  - `experiments/ink/cold-wake-sanctuary-intake-qwen-branch-candidates-v1.capture.json`
  - `experiments/ink/cold-wake-sanctuary-intake-qwen-branch-candidates-v1.prompt.md`
  - `experiments/ink/cold-wake-sanctuary-intake-qwen-branch-candidates-v1.response.md`
  - `tools/run_qwen_ink_branch_generation.py`
  - `tools/materialize_qwen_ink_draft.py`
  - `tools/validate_ink_examples.py`
  - `tools/run_qwen_ink_sequential_generation.py`
  - `tools/materialize_sequential_capture.py`
  - `tools/apply_sequential_ink_branch_mutation.py`
  - `inkjs` dev dependency and `npm run ink:validate`
- sequential branch mutation seam:
  - `experiments/ink/cold-wake-sanctuary-intake-qwen-sequential-v1.capture.json`
  - `experiments/ink/cold-wake-sanctuary-intake-qwen-sequential-v2.capture.json`
  - `experiments/ink/cold-wake-sanctuary-intake-qwen-sequential-v3.capture.json`
  - `experiments/ink/cold-wake-sanctuary-intake-qwen-sequential-v5.capture.json`
  - `experiments/ink/cold-wake-sanctuary-intake-qwen-sequential-v6-projector.capture.json`
  - `experiments/ink/cold-wake-sanctuary-intake-qwen-sequential-v8-projector-tightened.capture.json`
  - `experiments/ink/cold-wake-sanctuary-intake-qwen-sequential-v9-projector-tightened.capture.json`
  - `experiments/ink/cold-wake-sanctuary-intake-qwen-sequential-v10-no-think.capture.json`
  - `experiments/ink/cold-wake-sanctuary-intake-qwen-sequential-v1.mutation.json`
  - `experiments/ink/qwen-chat-tools-smoke.json`
  - `examples/agent-state.cold-wake-story-lab.after-sanctuary-ledger.json`
- projector-routed sequential materialization seam:
  - `examples/ink/cold-wake-sanctuary-intake.qwen-sequential-v10.ink`
  - `examples/ink/cold-wake-sanctuary-intake.qwen-sequential-v10.training.json`
  - `examples/agent-state.cold-wake-story-lab.after-v10-packet-assessment.json`
  - `experiments/ink/cold-wake-sanctuary-intake-qwen-sequential-v10.mutation.json`
- projected local context seam:
  - `schemas/projected-local-context.schema.json`
  - `examples/projected-contexts/scene-02-sanctuary-intake.maer_tidecall.projected-context.json`
  - `examples/projected-contexts/scene-02-sanctuary-intake.sella_ren.projected-context.json`
  - `tools/project_local_context.py`
  - `tools/validate_projected_contexts.py`
  - includes an embodiment/interface section so Navigator action is projected
    through habitat-native channels, workstations, displays, rails, and
    communication systems instead of default human body assumptions
- Cold Wake fixture note:
  - `docs/architecture/aetheria-cold-wake-training-fixture.md`
- first projection example seam:
  - `schemas/projection-example.schema.json`
  - `examples/projections/call-of-the-void.scene-broken-taxi-oz.jsonl`
  - `examples/projections/call-of-the-void.scene-broken-taxi-oz.pretty.json`
  - `tools/validate_projection_examples.py`
- visitor-facing README refreshed to explain Ghostlight's goals

There is still no full runtime yet. The live machine is design, state
discipline, a cleaner repo boundary, the first executable state contract, a
documented projection path, the first Ink-backed playable branching scene, the
first Qwen-generated Ink draft with reviewed training annotations, and the
first reviewed mutation receipt showing how a selected generated branch changes
both involved characters.

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

Review the embodiment-aware materialized v10 Ink branch and mutation receipt for
narrative quality and state-change plausibility, then either run another Qwen
pass through the updated projector or generalize the sequential materializer for
more accepted captures.

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
- revise the first Maer/Veyr projection through v3 so unresolved
  personhood/claimant suspicion stays unresolved in Qwen output
- save both useful failed captures and the accepted v3 response as training
  material
- complete the first readable Cold Wake story,
  `experiments/cold-wake-story-lab/the-narrowest-possible-margin.md`, with a
  receipts table tying accepted story beats to projection and capture artifacts
- promote projection controls into the projection artifact shape:
  `frame_controls`, `authority_boundaries`, `object_custody`,
  `required_semantics`, and `forbidden_resolutions`
- add a Qwen projection runner and capture validator so response artifacts are
  reviewed training receipts, not loose logs
- adopt Ink as the standard playable branching scene format rather than
  inventing a parallel Ghostlight dialogue-tree format
- add the first Cold Wake Ink scene,
  `examples/ink/cold-wake-sanctuary-intake.ink`, with a reviewed sidecar
  annotation tying branches to Ghostlight state, projection controls,
  consequence surfaces, and manual mutation policy
- add the first local-awareness-to-Qwen-to-Ink pass:
  `tools/run_qwen_ink_branch_generation.py` produced a reviewed capture, and
  `tools/materialize_qwen_ink_draft.py` materialized
  `examples/ink/cold-wake-sanctuary-intake.qwen-draft.ink`
- add the first sequential Qwen branch capture:
  `experiments/ink/cold-wake-sanctuary-intake-qwen-sequential-v1.capture.json`
  is useful-needs-revision because it separated actor and responder context but
  lacked the per-turn participant appraisal boundary and returned non-canonical
  action labels
- add the first reviewed state mutation replay:
  `tools/apply_sequential_ink_branch_mutation.py` writes
  `experiments/ink/cold-wake-sanctuary-intake-qwen-sequential-v1.mutation.json`
  and
  `examples/agent-state.cold-wake-story-lab.after-sanctuary-ledger.json`,
  updating Maer and Sella state, relationship stance, memories, perceived
  overlays, and the event log after the selected event
- add the second sequential Qwen capture:
  `experiments/ink/cold-wake-sanctuary-intake-qwen-sequential-v2.capture.json`
  uses the symmetrical turn framing and produces better action behavior
  (`show_object` followed by Sella's `withhold_object`), but remains
  useful-needs-revision because prose prompting did not force array-typed JSON
  fields
- verify Qwen invocation:
  `tools/check_qwen_chat_tools.py` proves local `qwen3.5:9b` returns both
  `message.thinking` and native `tool_calls` through Ollama `/api/chat`
- update the sequential runner to use `/api/chat` with native tools and
  strict tool calls
- add v3 and v5 thinking-plus-tools captures:
  v3 preserved a useful failure where old prompt text still caused plain JSON
  content in later passes; v5 is accepted as draft and produced valid
  tool-shaped Maer choice, Sella appraisal, and Sella next action
- identify the projector gap:
  the sequential runner is still scaffolding because it sends selected raw
  numeric state variables into prompts. The next real organ is a projector or
  state interpreter that turns canonical variables, memories, relationship
  stance, culture, and scene pressure into readable character-local operating
  context and action affordances. Character agents should not need to interpret
  `current_activation`, `plasticity`, or numeric dimension scores.
- add the first projector seam:
  `tools/project_local_context.py` creates
  `ghostlight.projected_local_context.v0` artifacts and rendered prompt prose
  from canonical state, scene, relationship, memories, projection controls, and
  event context while hiding raw state variables from the prompt text
- add projected-context validation:
  `tools/validate_projected_contexts.py` rejects projected prompt text that
  leaks raw state markers such as `current_activation`, `plasticity`, means, or
  decimal state values
- route the sequential Qwen runner through projected local context:
  `tools/run_qwen_ink_sequential_generation.py` now sends projected operating
  prose instead of selected activation dictionaries
- add v6 projector-routed Qwen capture:
  `experiments/ink/cold-wake-sanctuary-intake-qwen-sequential-v6-projector.capture.json`
  validates the architecture boundary and produces usable Maer/Sella behavior,
  but remains useful-needs-revision because Qwen stringified a nested
  `response_constraints` field in Sella appraisal
- tighten the sequential runner:
  repair notes are separated from fatal validation notes, obvious truncated
  stringified tool arrays can be repaired, failed Maer choice generation writes
  a reviewed capture instead of exiting, and Sella's next-action prompt receives
  appraisal context as prose constraints rather than raw numeric deltas
- add v8 and v9 projector-routed failure receipts:
  v8 shows malformed stringified `choices`; v9 shows a no-tool-call dropout.
  These are Qwen tool reliability failures, so no projector-routed branch was
  materialized into Ink in this pass.
- disable thinking by default for strict sequential generation:
  the v9 thinking trace showed Qwen looping on schema self-checks for minutes
  before returning no tool call, so `tools/run_qwen_ink_sequential_generation.py`
  now defaults to `think: false`; use `--think` only for diagnostics
- add v10 no-thinking projector-routed capture:
  `experiments/ink/cold-wake-sanctuary-intake-qwen-sequential-v10-no-think.capture.json`
  validates as accepted-as-draft with no repair notes or failure notes
- materialize v10:
  `tools/materialize_sequential_capture.py` wrote a playable Ink branch, sidecar
  training annotation, reviewed mutation receipt, and mutated Cold Wake fixture
  from the accepted projector-routed capture
- update v10 capture provenance:
  `accepted_into_ink_ref` now points at
  `examples/ink/cold-wake-sanctuary-intake.qwen-sequential-v10.ink`

Remaining projection path:

- build a deterministic speaker-local input slicer
- build a renderer that emits projection controls before prompt prose
- review the embodiment-aware materialized v10 Ink and mutation receipt for
  narrative quality and state-change plausibility
- run another Qwen pass through the updated projector if Maer's Navigator
  embodiment needs to be tested against generation rather than hand polish
- decide whether to polish this Ink text by hand or generalize the sequential
  materializer for future accepted captures
- preserve the no-thinking default for strict tool-call generation unless a
  diagnostic run explicitly needs `--think`
- improve the local-awareness-to-Ink prompt so semantic `training_hooks` do not
  drift into future branch ids, object custody remains branch-local, and action
  labels come from the exact canonical enum
- materialize a validated sequential branch into Ink
- keep the near-term product target to interactive dialogue trees and
  scene-local consequences
- preserve training-data hooks for future open-world behavior, iterated
  decisions, psychologically grounded consumer choices, scarcity/reputation
  pressure, and major faction or institution decision-making
- defer implementing world-scale simulation loops, autonomous offscreen
  factions, economy loops, and long-horizon plot machinery until the scene-local
  machine works
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
