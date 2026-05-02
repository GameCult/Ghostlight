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
  - boundary and plan for grounding single-history training data in authored
    Aetheria history while treating Elysium gameplay-era outcomes as
    conditional branch canon;
    includes the source-first doctrine for checking AetheriaLore and patching
    blocking lore gaps before generation, plus a `future_branch` lane for
    reviewed possible post-Rupture Elysium futures
- `docs/architecture/lore-grounding-digest-format.md`
  - human-facing format guide for cultural, factional, institutional, role, and
    speaker-boundary lore digests
- `docs/architecture/qwen-invocation-notes.md`
  - verified local Qwen invocation policy: Ollama `/api/chat`, native tools,
    thinking enabled, repair for known nested-argument double-stringify cases
- `docs/architecture/projected-local-context.md`
  - contract for the projector seam that turns canonical state into
    character-local operating context before a response model sees it,
    including source-checked embodiment and interface affordances
- `docs/architecture/soft-model-training-artifacts.md`
  - doctrine for treating every non-deterministic judgment as reviewed
    training material for future specialized models, including
    coordinator/story runtime, projector, character agent/responder, appraiser,
    state mutator, relationship/perception classifier, Aetheria responder, and
    institution/faction/consumer decision models
- `docs/architecture/training-plan.md`
  - concrete enumeration of trainable Ghostlight stages, including each stage's
    inputs, outputs, likely model architecture, artifact family, first training
    gate, and timeline coverage across historical grounded, transition
    grounded, and future-branch examples
- `docs/architecture/technology-item-manifest-plan.md`
  - plan for turning coordinator-led Aetheria exploration into game-usable
    item, component, assembly, faction tech-base, prerequisite, supply-chain,
    and upgrade-path data, including elaboration of existing nebulous named
    technologies into blueprint candidates; now explicitly targets the
    Aetheria-Economy CultCache `ItemData` technological blueprint classes
    instead of a parallel Ghostlight item database
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
- `schemas/projected-local-context.schema.json`
  - v0 contract for projected character-local operating context artifacts
- `schemas/coordinator-artifact.schema.json`
  - v0 contract for coordinator/story-runtime receipts: scene setup,
    next-beat choice, acting-agent choice, machinery invocations, sandboxed
    responder handoffs, world-state refs, proposed deltas, branch constraints,
    item-manifest deltas, unresolved hooks, glue prose, and review notes
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
- `examples/coordinator/cold-wake-sanctuary-intake.v0.json`
  - first coordinator artifact schema-shakedown example, backfilled from the
    accepted-as-draft sanctuary packet-assessment beat and explicitly marked as
    not gold responder data
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
- `examples/ink/cold-wake-sanctuary-intake.qwen-sequential-v10.ink`
  - first playable Ink branch materialized from an accepted projector-routed
    sequential Qwen capture
- `examples/ink/cold-wake-sanctuary-intake.qwen-sequential-v10.training.json`
  - sidecar annotation for the v10 materialized sequential branch
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
- `experiments/ink/cold-wake-sanctuary-intake-qwen-sequential-v6-projector.capture.json`
  - useful-needs-revision projector-routed receipt proving Qwen prompts can use
    character-local operating prose instead of raw state variables; still
    exposed nested `response_constraints` stringification in appraisal output
- `experiments/ink/cold-wake-sanctuary-intake-qwen-sequential-v8-projector-tightened.capture.json`
  - useful-needs-revision receipt proving failed Maer choice generation is now
    captured instead of crashing the runner; exposed malformed stringified
    `choices`
- `experiments/ink/cold-wake-sanctuary-intake-qwen-sequential-v9-projector-tightened.capture.json`
  - useful-needs-revision receipt showing Qwen can still drop tool calls
    entirely under projector-routed prompting
- `experiments/ink/cold-wake-sanctuary-intake-qwen-sequential-v10-no-think.capture.json`
  - accepted-as-draft receipt showing projector-routed sequential generation
    works cleanly when strict tool calls run with thinking disabled
- `examples/projected-contexts/`
  - current rendered projected local context artifacts for Maer and Sella in
    the sanctuary intake scene
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
- `examples/agent-state.cold-wake-story-lab.after-v10-packet-assessment.json`
  - mutated Cold Wake fixture after the v10 packet-assessment branch,
    preserving unresolved packet personhood while updating state, relationship
    stance, memories, perceived overlays, event log, and active scene memories
- `experiments/ink/cold-wake-sanctuary-intake-qwen-sequential-v10.mutation.json`
  - reviewed mutation receipt for v10 materialization, keeping fuzzy appraisal
    and relationship movement manual and auditable
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
- `tools/materialize_sequential_capture.py`
  - turns an accepted sequential capture into playable Ink, sidecar annotation,
    reviewed mutation receipt, and a mutated fixture
- `tools/run_qwen_ink_sequential_generation.py`
  - builds a sequential Qwen prototype through Ollama `/api/chat` with native
    tools and thinking disabled by default: actor-local choices, participant
    appraisal/consolidation, then next-actor action from updated state; now
    routes character context through the projected-local-context seam, captures
    failed Maer choice calls, separates repair notes from fatal validation
    notes, and keeps numeric appraisal deltas out of next-action prompt context
- `tools/check_qwen_chat_tools.py`
  - fast smoke test for local Qwen thinking plus native tool calling
- `tools/project_local_context.py`
  - deterministic first projector that compiles canonical state, scene facts,
    relationships, memories, projection controls, and optional observed event
    into character-local operating prose; for cetacean Navigators it projects
    source-checked habitat affordances from AetheriaLore instead of generic
    human body defaults
- `tools/validate_projected_contexts.py`
  - validates projected context artifacts and rejects prompt text that leaks raw
    canonical state internals
- `tools/validate_coordinator_artifacts.py`
  - validates v0 coordinator artifacts, including sandboxed responder handoff
    fields, world-state refs, branch constraints, proposed deltas, unresolved
    hooks, and review status
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
digest schema, projected-local-context schema, coordinator artifact schema,
first draft Cold Wake grounding digest, and first Cold Wake story-lab Qwen
captures now exist as v0 seams. The
first complete Cold Wake story pass exists with receipts, projection controls
have been promoted, and Ink is now the standard branching scene format. The
first Qwen-generated Ink draft now exists with a reviewed capture, the first
selected branch now has a reviewed mutation replay that updates both involved
characters, v2 shows the symmetrical turn model improves action behavior, v5
proves local thinking-plus-tools invocation works for the scene runner, and v6
routes that runner through projected character-local operating context. v8 and
v9 show that Qwen tool-call reliability, not the projector seam, is now the
immediate blocker. The v9 thinking trace also showed schema self-check looping
instead of tool-call completion, so strict sequential generation now disables
thinking by default. v10 validated that correction with an accepted-as-draft
projector-routed no-thinking capture, which has now been materialized into Ink
plus reviewed mutation training data. The next implementation target is
fixing the Sella next-action prompt/rendering path after source-checked v11-v17
retries improved Maer's Navigator embodiment but exposed object-custody drift,
invalid appraisal paths, Qwen tool dropout, and prompt-constraint leakage into
responder prose. The soft-model artifact doctrine now makes the broader
training boundary explicit: deterministic gates stay code, while fuzzy
coordination, projection, action choice, appraisal, mutation, relationship
movement, Aetheria responder training, and economic/faction decision judgments
become reviewed artifacts for later fine-tuning or distillation. Gold responder
data now requires a sandbox boundary: the coordinator may be omniscient, but the
responder sees only the exact projected local packet, visible event, allowed
actions, and explicitly included source excerpts; artifacts preserve raw output,
reviewed output, hidden-context refs, leakage audit, isolation method, and
coordinator intervention labels. The concrete training plan now enumerates
eleven trainable stages and their likely model
families: generative decoder LLMs for coordinator, responder, and structured
soft outputs; classifiers or cross-encoders for appraisal, evaluation, and
relationship movement; embedding/retrieval models for memory and lore selection;
and deterministic code for gates. The listed per-stage counts are pilot
schema-shakedown gates, not robust training targets; review-assistant and runtime
targets are larger corpus tiers meant to avoid declaring victory after the first
small pile of sacrificial data merely shows where the schema is wrong. The
first coordinator artifact seam now validates through `npm run schema:validate`
and backfills the sanctuary packet-assessment beat as an accepted-as-draft
schema shakedown example. The coordinator is currently the authoring
agent: it glues scenes together, carries world-state refs and unresolved hooks,
decides which machinery runs, and emits glue prose. That output should stay
game-engine-shaped even while the prose lives in model imagination. The
lore/tone goal is not a separate adapter; it is a responder trained on strong
timeline-spread Aetheria responses until the setting's assumptions become
native priors. The timeline plan now includes generated possible futures in
Elysium as reviewed `future_branch` artifacts so models learn post-Rupture
concepts as branch-local canon indexed by lineage. It also distinguishes
branch attractors, fated events, technology-order constraints, quest injections,
and ordinary branch-local events so Elysium can support likely faction births,
authored rails, discovery order, and quest injection without flattening the
timeline into equal-probability soup. The technology/item manifest plan now
requires exploration to emit item families, variants, assemblies,
subassemblies, components, materials, faction tech-base access, prerequisites,
bottlenecks, compatibility rules, supply-chain consequences, and quest hooks for
both pre-Elysium starting tech and post-Rupture innovations, mapped onto
Aetheria-Economy's existing CultCache technological blueprint classes:
`SimpleCommodityData`, `CompoundCommodityData`, `ConsumableItemData`, `GearData`,
and other `EquippableItemData` subclasses. Runtime `ItemInstance` classes remain
simulation, inventory, cargo, provenance, branding, and world-state objects.
Recipes, assembly trees, compatibility rules, tooling, facilities, process
requirements, and supply-chain dependencies are blueprint metadata or
engine-schema gaps attached to the relevant `*Data` class until the economy code
has explicit storage for them. The player-facing gear fantasy is provenance
hunting: players chasing an ultimate item should trace exceptional assemblies,
manufacturer processes, repair history, counterfeit substitutions, supply-chain
lineage, and operating constraints through the galaxy. The technology pipeline is
therefore a content refinery: it grounds ideas in lore, decomposes them into
parts and supply chains, maps them to economy blueprint classes, defines
performance and instance variation, preserves review labels, and emits
database-shaped candidates instead of spreadsheet sludge. Do not materialize
another capture until it has no failure or repair notes.
