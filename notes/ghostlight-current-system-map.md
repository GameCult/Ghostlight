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
- `docs/architecture/sequential-agent-runtime.md`
  - scene-local dialogue/runtime loop where agents act from local awareness,
    generated actions become event proposals, and bounded state mutation drives
    branchable speech or non-speech scene consequences while preserving
    training hooks for later open-world, economic, and faction decision models
- `docs/architecture/ink-branching-scenes.md`
  - integration contract that makes Ink the standard playable branch format
    while Ghostlight owns local awareness, projection rationale, consequence
    review, and state mutation around Ink scenes
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
- `docs/architecture/qwen-invocation-notes.md`
  - verified local Qwen invocation policy: Ollama `/api/chat`, native tools,
    thinking enabled, repair for known nested-argument double-stringify cases
- `docs/architecture/aetheria-cold-wake-training-fixture.md`
  - human-facing note that reframes the recovered Cold Wake scenario as
    historical projection feedstock rather than the active product target
- `schemas/agent-state.schema.json`
  - v0 canonical agent-state contract for agents, relationships, events,
    scenes, perceived overlays, and dialogue context packs
- `schemas/projection-example.schema.json`
  - v0 reviewed projection artifact contract for speaker-local prompt
    projection examples, including projection controls for frame, authority,
    object custody, required semantics, and forbidden resolutions
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
  - `the-narrowest-possible-margin.md`, the first readable short story
    assembled from accepted projection outputs
- `examples/lore-grounding/historical-flashpoint.template.json`
  - fillable template for the first historical Aetheria grounding digest
- `examples/lore-grounding/cold-wake-panic.ganymede-corridor.json`
  - first draft historical grounding digest, centered on Cold Wake Panic in the
    Ganymede/Jovian corridor
- `examples/ink/cold-wake-sanctuary-intake.ink`
  - first playable Ink-backed Cold Wake branching scene, using speech, gesture,
    object revelation, and moral-pressure branches
- `examples/ink/cold-wake-sanctuary-intake.training.json`
  - reviewed sidecar annotation tying Ink branches to Ghostlight local
    awareness, projection controls, branch rationale, consequence surfaces, and
    manual mutation policy
- `examples/ink/cold-wake-sanctuary-intake.qwen-draft.ink`
  - first Qwen-generated playable Ink draft materialized from a Ghostlight
    local-awareness prompt
- `examples/ink/cold-wake-sanctuary-intake.qwen-draft.training.json`
  - sidecar annotation for the generated draft, accepted as training material
    rather than final polished prose
- `experiments/ink/cold-wake-sanctuary-intake-qwen-branch-candidates-v1.capture.json`
  - reviewed Qwen branch-generation receipt, including prompt, parsed response,
    strengths, failure notes, and pipeline lessons
- `experiments/ink/cold-wake-sanctuary-intake-qwen-sequential-v1.capture.json`
  - useful-needs-revision sequential Qwen receipt that separates Maer-local
    choice generation from Sella-local next-action generation, while preserving
    the discovered failure around missing per-turn appraisal/consolidation and
    invalid action labels
- `experiments/ink/cold-wake-sanctuary-intake-qwen-sequential-v2.capture.json`
  - useful-needs-revision sequential Qwen receipt using the symmetrical turn
    model; it produced canonical action labels and a concrete Sella
    `withhold_object` next action, but still needs stricter schema enforcement
    because some array fields came back as strings or objects
- `experiments/ink/cold-wake-sanctuary-intake-qwen-sequential-v3.capture.json`
  - useful-needs-revision thinking-plus-tools receipt that proved Maer could
    call the tool while later passes still fell back to plain JSON content
    because old prompt text conflicted with the tool path
- `experiments/ink/cold-wake-sanctuary-intake-qwen-sequential-v5.capture.json`
  - accepted-as-draft thinking-plus-tools receipt using Ollama `/api/chat`,
    native tools, and `think: true` for Maer choice, Sella appraisal, and Sella
    next action
- `experiments/ink/qwen-chat-tools-smoke.json`
  - smoke receipt proving local `qwen3.5:9b` returns both `message.thinking`
    and native `tool_calls`
- `experiments/ink/cold-wake-sanctuary-intake-qwen-sequential-v1.mutation.json`
  - first reviewed branch mutation receipt, including manual participant
    appraisal/consolidation, normalized action labels, applied state deltas, and
    deferred unresolved facts
- `examples/agent-state.cold-wake-story-lab.after-sanctuary-ledger.json`
  - mutated Cold Wake fixture after the selected sanctuary ledger branch,
    updating Maer and Sella state, relationship stance, memories, perceived
    overlays, event log, and active scene memories
- `tools/ghostlight_state.py`
  - compact state inspection and evidence or branch updates
- `tools/ghostlight_prepare_compaction.py`
  - pre-compaction audit of required files, handoff content, and git hygiene
- `tools/validate_agent_state.py`
  - dependency-free validator for the current schema fixture invariants
- `tools/validate_projection_examples.py`
  - dependency-free validator for projection example JSONL records
- `tools/run_qwen_projection.py`
  - sends a projection prompt to the LAN Qwen box and saves response/capture
    artifacts without PowerShell encoding footguns
- `tools/validate_qwen_captures.py`
  - validates reviewed Qwen response capture receipts
- `tools/validate_lore_grounding.py`
  - dependency-free validator for lore grounding digest examples
- `tools/validate_ink_examples.py`
  - compiles Ink examples with `inkjs` and validates sidecar training
    annotations plus reviewed Qwen Ink-generation captures
- `tools/run_qwen_ink_branch_generation.py`
  - builds a local-awareness prompt from the Cold Wake fixture and asks Qwen
    for Ink branch candidates
- `tools/materialize_qwen_ink_draft.py`
  - turns a reviewed Qwen branch capture into a playable Ink draft and sidecar
    annotation
- `tools/run_qwen_ink_sequential_generation.py`
  - builds a sequential Qwen prototype through Ollama `/api/chat` with native
    tools and thinking enabled: actor-local choices, participant
    appraisal/consolidation, then next-actor action from updated state
- `tools/check_qwen_chat_tools.py`
  - fast smoke test for local Qwen thinking plus native tool calling
- `tools/apply_sequential_ink_branch_mutation.py`
  - applies a reviewed replay of one selected branch into a mutated agent-state
    fixture and mutation receipt without letting Ink variables become
    canonical truth
- `tools/build_cold_wake_story_lab_fixture.py`
  - generator for the verbose Cold Wake story-lab state fixture

## What Does Not Exist Yet

- a classifier pipeline
- a prompt renderer
- a full projection training corpus
- a deterministic projection input slicer
- a student projection model
- a simulation/event loop implementation
- a relationship engine
- a culture prior engine
- an automatic Ink branch generator from local awareness
- automatic promotion of Ink branch outcomes into canonical state

The canonical agent-state schema, projection example schema, lore grounding
digest schema, first draft Cold Wake grounding digest, and first Cold Wake
story-lab Qwen captures now exist as v0 seams. The first complete Cold Wake
story pass exists with receipts, projection controls have been promoted, and
Ink is now the standard branching scene format. The first Qwen-generated Ink
draft now exists with a reviewed capture, the first selected branch now has a
reviewed mutation replay that updates both involved characters, v2 shows the
symmetrical turn model improves action behavior, and v5 proves local
thinking-plus-tools invocation works for the scene runner. The next
implementation target is materializing the accepted v5 capture into Ink and a
sidecar annotation.
