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
- `docs/architecture/projected-local-context.md`
  - contract for the projector seam that turns canonical state into
    character-local operating context before a response model sees it,
    including source-checked embodiment and interface affordances, runtime
    retrieval requirements, and latent pressure handling
- `docs/architecture/soft-model-training-artifacts.md`
  - doctrine for treating every non-deterministic judgment as reviewed
    training material for future specialized models, including
    coordinator/story runtime, projector, character agent/responder, appraiser,
    state mutator, relationship/perception classifier, Aetheria responder, and
    institution/faction/consumer decision models
- `docs/architecture/sandboxed-responder-packets.md`
  - contract for the exact packet-only surface handed from an omniscient
    coordinator to a sandboxed character responder, including visible event,
    local prompt, allowed actions, source excerpts, hidden-context audit, and
    output contract; also defines the separate `packet_only` and
    `retrieval_augmented` responder-data lanes
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
  - v0 contract for projected character-local operating context artifacts,
    including structured runtime retrieval requirements and latent pressure
    requirements
- `schemas/coordinator-artifact.schema.json`
  - v0 contract for coordinator/story-runtime receipts: scene setup,
    next-beat choice, acting-agent choice, machinery invocations, sandboxed
    responder handoffs, world-state refs, proposed deltas, branch constraints,
    item-manifest deltas, unresolved hooks, glue prose, and review notes
- `schemas/responder-packet.schema.json`
  - v0 contract for the exact input a sandboxed character responder may see,
    now lane-labeled as `packet_only` or `retrieval_augmented` with explicit
    lore-access policy; research-enabled packets can require
    responder-scoped repository search with visible research instructions
- `schemas/responder-output.schema.json`
  - v0 contract for reviewed captures from sandboxed responder workers,
    preserving raw output, parsed output, isolation method, leakage audit, and
    coordinator review; output captures also preserve generation lane and
    consulted lore refs, and research-enabled captures can preserve a research
    summary
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
- `examples/responder-packets/scene-02-sanctuary-intake.sella_ren.packet.v0.json`
  - first responder-packet schema-shakedown example for Sella's sanctuary
    response; accepted as draft for sandbox handoff testing, not gold responder
    output
- `examples/responder-packets/scene-02-sanctuary-intake.sella_ren.packet.v1.json`
  - corrected packet-only Sella handoff with absent hidden context moved out of
    responder-visible prose and curated AetheriaLore excerpts added for Cold
    Wake, Ganymede rescue obligations, Navigator rescue ledgers, and Lightsail
    reliability
- `examples/responder-packets/scene-02-sanctuary-intake.sella_ren.packet.research.v0.json`
  - first concrete research-enabled Sella handoff; it uses
    `responder_scoped_repository_search`, declares the allowed AetheriaLore
    scope, permits repo access only for that scope, and requires consulted refs
    plus research summary in the responder object
- `experiments/responder-packets/cold-wake-sanctuary-intake-sella-v0.capture.json`
  - first no-fork sandboxed responder output capture from the Sella packet;
    accepted as draft with no leakage flags and no coordinator prose repair
- `experiments/responder-packets/cold-wake-sanctuary-intake-sella-v0.mutation.json`
  - reviewed mutation receipt that materializes the packet-only Sella response
    into scene-local state, relationship, memory, perceived-overlay, and
    unresolved-hook updates without resolving packet personhood
- `experiments/responder-packets/cold-wake-sanctuary-intake-sella-retrieval-v0.capture.json`
  - first retrieval-augmented responder capture using coordinator-selected
    scoped AetheriaLore refs
    for Aya sanctuary politics, Cold Wake heat-debt pressure, Navigator rescue
    ledgers, Ganymede route obligations, and Lightsail reliability
- `experiments/responder-packets/cold-wake-sanctuary-intake-sella-lane-comparison.v0.json`
  - comparison artifact showing the packet-only lane proves bounded runtime
    competence while the retrieval-augmented lane adds stronger
    Aetheria-native institutional texture, retrieval lessons, and a warning
    that character backstory must remain available as latent pressure
- `experiments/cold-wake-story-lab/`
  - archived local-model response captures and reviews from the writing experiment
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
- archived local-model prototype Ink and capture receipts
  - older files with `qwen` in the path remain in `examples/ink/` and
    `experiments/ink/` as historical receipts; they are not the active
    generation path and should not steer new gold-data work unless model
    plumbing is explicitly being tested
- `tools/ghostlight_state.py`
  - compact state inspection and evidence or branch updates
- `tools/ghostlight_prepare_compaction.py`
  - pre-compaction audit of required files, handoff content, and git hygiene
- `tools/validate_agent_state.py`
  - dependency-free validator for the current schema fixture invariants
- `tools/validate_projection_examples.py`
  - dependency-free validator for projection example JSONL records
- archived local-model prototype tools
  - older `run_qwen_*`, `materialize_qwen_*`, `validate_qwen_captures.py`, and
    `check_qwen_chat_tools.py` helpers remain for validating historical
    receipts and testing model plumbing; they are not the active gold-data path
- `tools/validate_lore_grounding.py`
  - dependency-free validator for lore grounding digest examples
- `tools/validate_ink_examples.py`
  - compiles Ink examples with `inkjs` and validates sidecar training
    annotations plus reviewed prototype Ink-generation captures
- `tools/materialize_sequential_capture.py`
  - turns an accepted sequential capture into playable Ink, sidecar annotation,
    reviewed mutation receipt, and a mutated fixture
- `tools/project_local_context.py`
  - deterministic first projector that compiles canonical state, scene facts,
    relationships, memories, projection controls, and optional observed event
    into character-local operating prose; for cetacean Navigators it projects
    source-checked habitat affordances from AetheriaLore instead of generic
    human body defaults; now emits scene-triggered compact lore retrieval
    requirements and latent character-history pressure requirements
- `tools/validate_projected_contexts.py`
  - validates projected context artifacts and rejects prompt text that leaks raw
    canonical state internals
- `tools/validate_coordinator_artifacts.py`
  - validates v0 coordinator artifacts, including sandboxed responder handoff
    fields, world-state refs, branch constraints, proposed deltas, unresolved
    hooks, and review status
- `tools/build_responder_packet.py`
  - builds packet-only or research-enabled responder handoffs from a coordinator
    artifact and projected local context; source excerpts now come from active
    projected runtime retrieval requirements instead of a static lore list
- `tools/validate_responder_packets.py`
  - validates responder packets and rejects prompt surfaces that leak raw state
    internals or hidden coordinator context markers
- `tools/validate_responder_outputs.py`
  - validates sandboxed responder output captures, including exact raw-to-parsed
    JSON agreement, no-fork isolation, action labels, leakage audit, and review
    metadata; also validates responder mutation receipts
- `tools/apply_responder_output_mutation.py`
  - applies a reviewed responder-output capture into a mutated agent-state
    fixture and mutation receipt
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

The canonical schemas and first Cold Wake fixture seams now exist as v0
contracts. Archived local-model prototype receipts proved useful for discovering
architecture lessons, but the active path is now sandboxed responder packets,
source-grounded local context, reviewed outputs, and audited mutation receipts.
Deterministic gates stay code; fuzzy coordination, projection, action choice,
appraisal, mutation, relationship movement, Aetheria responder training, and
economic/faction decision judgments become reviewed artifacts for later
fine-tuning or distillation. The next gold-data pass should use the concrete
research-enabled Sella packet and preserve exact visible input, consulted refs,
research summary, raw output, reviewed output, hidden-context refs, leakage
audit, isolation method, and coordinator intervention labels.
