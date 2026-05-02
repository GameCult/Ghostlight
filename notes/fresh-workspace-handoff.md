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
  - `docs/architecture/projected-local-context.md`
  - `docs/architecture/soft-model-training-artifacts.md`
  - `docs/architecture/training-plan.md`
  - `docs/architecture/technology-item-manifest-plan.md`
- lore grounding seam:
  - `schemas/lore-grounding-digest.schema.json`
  - `examples/lore-grounding/historical-flashpoint.template.json`
  - `examples/lore-grounding/cold-wake-panic.ganymede-corridor.json`
  - `tools/validate_lore_grounding.py`
- coordinator artifact seam:
  - `schemas/coordinator-artifact.schema.json`
  - `examples/coordinator/cold-wake-sanctuary-intake.v0.json`
  - `tools/validate_coordinator_artifacts.py`
  - `npm run coordinator:validate`
- sandboxed responder packet seam:
  - `docs/architecture/sandboxed-responder-packets.md`
  - `schemas/responder-packet.schema.json`
  - `schemas/responder-output.schema.json`
  - `examples/responder-packets/scene-02-sanctuary-intake.sella_ren.packet.v0.json`
  - `examples/responder-packets/scene-02-sanctuary-intake.sella_ren.packet.v1.json`
  - `examples/responder-packets/scene-02-sanctuary-intake.sella_ren.packet.research.v0.json`
  - `experiments/responder-packets/cold-wake-sanctuary-intake-sella-v0.capture.json`
  - `experiments/responder-packets/cold-wake-sanctuary-intake-sella-v0.mutation.json`
  - `experiments/responder-packets/cold-wake-sanctuary-intake-sella-retrieval-v0.capture.json`
  - `experiments/responder-packets/cold-wake-sanctuary-intake-sella-lane-comparison.v0.json`
  - `tools/build_responder_packet.py`
  - `tools/validate_responder_packets.py`
  - `tools/validate_responder_outputs.py`
  - `tools/apply_responder_output_mutation.py`
  - `npm run responder-packets:validate`
  - `npm run responder-outputs:validate`
- Cold Wake story-lab seam:
  - `docs/experiments/cold-wake-story-lab.md`
  - `examples/agent-state.cold-wake-story-lab.json`
  - `examples/projections/cold-wake-story-lab.jsonl`
  - `experiments/cold-wake-story-lab/the-narrowest-possible-margin.md`
  - archived local-model prototype captures under `experiments/cold-wake-story-lab/`
  - `tools/build_cold_wake_story_lab_fixture.py`
- Ink branching and mutation seam:
  - `examples/ink/cold-wake-sanctuary-intake.ink`
  - `examples/ink/cold-wake-sanctuary-intake.training.json`
  - archived prototype Ink/capture receipts under `examples/ink/` and `experiments/ink/`
  - `tools/validate_ink_examples.py`
  - `tools/materialize_sequential_capture.py`
  - `tools/apply_sequential_ink_branch_mutation.py`
  - `inkjs` dev dependency and `npm run ink:validate`
- projected local context seam:
  - `schemas/projected-local-context.schema.json`
  - `examples/projected-contexts/scene-02-sanctuary-intake.maer_tidecall.projected-context.json`
  - `examples/projected-contexts/scene-02-sanctuary-intake.sella_ren.projected-context.json`
  - `tools/project_local_context.py`
  - `tools/validate_projected_contexts.py`
  - includes an embodiment/interface section so Navigator action is projected
    from checked Aetheria lore instead of default human body assumptions;
  - now includes `runtime_retrieval_requirements` and
    `latent_pressure_requirements` so compact lore facts and character-local
    scars survive projection without becoming mandatory confession prose;
    current source supports fluid architecture, acoustic signaling, communal
    orientation chambers, mixed-species translation/coexistence, and specialized
    cetacean environments; AetheriaLore has now been patched with mixed
    wet/dry habitat and interface ergonomics
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
first prototype-generated Ink draft with reviewed training annotations, and the
first reviewed mutation receipt showing how a selected generated branch changes
both involved characters. The soft-model training doctrine is now explicit:
deterministic gates stay code, while fuzzy judgments become reviewed artifacts
for future coordinator/story-runtime, projector, character-agent, appraiser,
mutator, relationship/perception, Aetheria responder, and
institution/faction/consumer decision models.
The coordinator is currently the authoring agent: it glues scenes together,
maintains continuity, chooses which machinery runs, emits connective prose, and
proposes world-state changes. Keep those outputs structured enough for future
game-engine integration instead of letting them rot as chat-only intuition.
Gold responder data must be sandboxed. The coordinator may be omniscient, but a
responder example is only valid if the acting worker saw exactly the projected
local packet, visible event, allowed action space, and explicitly included source
excerpts. Do not let responder workers inherit full chat/coordinator context when
generating training data. Preserve exact responder-visible input, raw output,
reviewed output, hidden-context refs, leakage audit, isolation method, and
coordinator intervention labels. Coordinator repairs are allowed, but they must
be labeled as repairs rather than treated as raw responder behavior.
The concrete training plan now enumerates the trainable stages, their inputs,
outputs, likely model families, artifact families, and pilot corpus gates. The
listed pilot counts are schema shakedown thresholds, not robust training
targets. They exist to expose missing fields, bad boundaries, leakage risks, and
validator gaps before large data generation. Robust organs will need roughly an
order of magnitude more examples, with separate review-assistant and runtime
targets.
The first coordinator artifact seam now exists. It captures scene setup,
next-beat choice, acting-agent choice, machinery invocation plan, sandboxed
responder handoffs, world-state refs, proposed deltas, branch constraints,
item-manifest deltas, unresolved hooks, glue prose, and review status. The first
example is a backfilled Cold Wake sanctuary packet-assessment artifact accepted
as draft for schema shakedown, not gold responder data.
The first responder packet seam now exists. It converts a coordinator artifact
and projected local context into the exact packet-only prompt surface a
character responder may see, with visible event, allowed actions, source
excerpts, hidden-context audit, isolation requirements, output contract, and
validator checks against raw-state or coordinator-context leakage. The first
Sella packet is accepted as draft for sandbox testing, not as gold responder
output. The first no-fork responder output capture now also exists and is
accepted as draft: the worker received only the packet prompt, returned valid
JSON, preserved Sella's read as interpretation, left packet personhood
unresolved, and required no coordinator prose repair.
Responder data now has two explicit lanes. `packet_only` is the runtime-parity
lane: curated source excerpts are in the packet and the responder does not
search lore. `retrieval_augmented` is the lore-absorption lane: the coordinator
or a retrieval worker searches a declared AetheriaLore scope, then gives the
responder a bounded packet. Do not use retrieval-augmented examples as proof
that the packet-only runtime prompt has enough context, and do not describe
them as autonomous responder research unless the responder actually had repo
access.
There is now an explicit research-enabled mode:
`responder_scoped_repository_search`. Use it only when the responder is allowed
to inspect the declared lore scope. The packet must include `Required Lore
Research`, `allowed_scope`, and `research_instructions`; the output capture must
include consulted refs and a research summary. A research-enabled responder that
answers without consulting lore has failed the assignment, bless its little
unexamined heart.
The first concrete research-enabled Sella packet now exists at
`examples/responder-packets/scene-02-sanctuary-intake.sella_ren.packet.research.v0.json`.
It is generated with
`tools/build_responder_packet.py --lore-access-mode responder_scoped_repository_search`,
sets `no_repo_access` false, and requires `consulted_refs` plus
`research_summary` in the responder object.
The v1 Sella packet removes responder-visible references to absent coordinator
knowledge and adds curated AetheriaLore excerpts for Cold Wake, the Ganymede
Route Compact, Navigator rescue ledgers, and Lightsail reliability.
The packet builder now gets those source excerpts from projected
`runtime_retrieval_requirements` rather than from a static hand-written list.
For the current Sella packet, that pulls Cold Wake heat-debt, Navigator rescue
ledger, Aya sanctuary-capacity, and Ganymede/Lightsail route-obligation facts
into packet-only context.
The packet-only Sella capture has now been materialized into a reviewed mutation
receipt and mutated fixture:
`examples/agent-state.cold-wake-story-lab.after-sella-conditions.json`. The
mutation accepts a conditional cold repair-bay path as scene-local state, keeps
packet personhood unresolved, and updates Maer and Sella state, relationship
stance, memories, and perceived overlays.
The first retrieval-augmented Sella capture also exists. It uses
coordinator-selected scoped AetheriaLore refs and produces richer
Aetheria-native language around heat-debt timing, rescue ledgers, dockfall
burden, and sanctuary as moral-debt sink. The lane comparison records that this
is good lore-absorption data but not proof of packet-only runtime sufficiency.
Follow-up review clarified the real concern: Sella's packet-visible
sanctuary-collapse scar does not need to surface in speech, but it must remain
visible to the responder as latent behavioral pressure. Future
projector/retrieval review should verify pressure availability, not reward
mission-critical trauma-dumping.
It now uses three timeline lanes: `historical_grounded`,
`transition_grounded`, and `future_branch`. Future branches are required
training/evaluation material for post-Rupture Elysium concepts, but generated
branch assumptions are branch-local canon indexed by lineage, not global facts
shared by every Elysium history.

## Current Direction

Use Ghostlight first for:

- content generation inside the Call of the Void slice described in
  `AetheriaLore`
- dialogue scaffolding and procedural drama for narrated Aetheria timeline
  stories

Cold Wake is now selected as the first historical grounding fixture because it
is authored pre-Elysium lore. It is not the current game target. Treat it as
training feedstock for projection and dialogue scaffolding.

## Aetheria Grounding Doctrine

Before writing or generating Aetheria scenes, check AetheriaLore for the
factions, movements, institutions, species or body types, location, and time
period involved. If a blocking detail is missing, patch the lore source with a
narrow elaboration before treating the detail as available to Ghostlight.

## Current Next Action

Run a sandboxed no-fork research-enabled responder against
`examples/responder-packets/scene-02-sanctuary-intake.sella_ren.packet.research.v0.json`,
allowing only the declared AetheriaLore scope. Capture exact visible input,
consulted refs, research summary, raw output, review labels, and leakage audit.
Review whether the output uses institutional lore and latent pressure without
prompt parroting, omniscience, or mission-critical trauma-dumping. Do not use
archived local-model prototype runners for gold responder data.

Cat/Oz remains useful as an Elysium procedural mechanics fixture, but
single-history grounding should start from authored historical lore rather than
gameplay-era Elysium outcomes.
Post-Rupture future branches should be generated too, under explicit branch
labels, so the models learn Aetheria concepts that only exist after the
Rupture: Aether, pseudospace, temporal nonlinearity, spirits, necrotech,
mutable bodies, altered substrates, and the fresh little institutional crimes
that grow around them.
Future branches now also need event constraint labels:
`branch_attractor`, `fated_event`, `tech_order_constraint`, `quest_injection`,
or `branch_local_event`. This supports likely Elysium faction births, authored
rails, baked-in technology order, and quest injection into active timelines.
Future and historical exploration should also emit technology/item manifest
data: item families, variants, assemblies, subassemblies, components, materials,
faction tech-base access, prerequisites, bottlenecks, compatibility rules,
upgrade surfaces, economic consequences, and quest hooks. Pre-Elysium starting
tech is part of this because factions enter Elysium with asymmetric industrial
bases and supply chains.
Existing named but nebulous technologies should be elaborated into blueprint
candidates during the same process. Separate source facts, inferred
engineering, and open lore gaps so item database candidates do not pretend the
timeline already answered every manufacturing question.
The concrete schema target already exists in Aetheria-Economy: CultCache-backed
`ItemData` subclasses are technological blueprint records. `SimpleCommodityData`
defines fungible resources, `CompoundCommodityData` defines assemblies,
subassemblies, components, processed materials, and crafted economic units,
`ConsumableItemData` defines usable consumables, and `GearData` plus other
`EquippableItemData` subclasses define top-level usable gear. Runtime
`ItemInstance` objects carry realized quality, provenance, manufacturer
branding, assembly history, and world-state placement. Ghostlight should attach
recipes, assembly trees, compatibility, tooling, facilities, process
requirements, and supply-chain dependencies to the relevant blueprint as
manifest metadata or engine-schema gaps until Aetheria-Economy exposes explicit
storage for those details.
The player-facing fantasy is provenance hunting. An ultimate laser is not only a
high-stat blueprint; it is a realized artifact assembled from rare or excellent
subsystems, manufacturer processes, repair history, counterfeit substitutions,
salvage scars, ownership chains, and operating conditions. Players should be
able to follow those threads through trade, salvage, espionage, faction access,
obsolete factories, and miserable little wars over parts like a beam-control
wafer some dead manufacturer only made for nine months before regulators ate the
board.
Ghostlight's job is to make that content scalable without turning it into
sludge. Treat the technology pipeline as a content refinery: ground the concept
in lore, decompose it into parts and supply chains, map it to the economy
blueprint classes, define performance and instance variation, preserve review
labels and source gaps, then emit database-shaped candidates instead of another
spreadsheet swamp.

The evaluation follow-up fixed the immediate v0 schema issues:

- `behavioral_dimensions.warmth` became `interpersonal_warmth`
- `voice_style.warmth` became `verbal_warmth`
- Cat now has a perceived-state overlay for Oz
- Cat and Oz now have relationship summary memories
- `event-drone-lie` now includes observed exchange, private interpretations,
  and event effects

Completed projection path items:

- defined projection examples, Cold Wake grounding, story-lab fixtures, and projection controls
- adopted Ink as the playable branching scene format
- built and validated local-awareness, projected-context, responder-packet, capture, and mutation seams
- preserved archived local-model prototype captures as reviewed receipts, not active guidance
- materialized one accepted projector-routed prototype branch into Ink, sidecar annotation, mutation receipt, and mutated fixture
- extracted the important lessons: character-local context, source-checked embodiment, object custody, per-turn consolidation, structured failure receipts, and no raw-state leakage into responder prompts

Remaining projection path:

- build a deterministic speaker-local input slicer
- build a renderer that emits projection controls before prompt prose
- review the embodiment-aware materialized v10 Ink and mutation receipt for
  narrative quality and state-change plausibility
- keep using sandboxed responder packets and reviewed outputs for fresh
  generation passes
- source-checked retries improved Maer's grounded embodiment, but no capture was
  materialized because Sella appraisal/response still exposed object-custody
  drift, invented mutation paths, structured-output dropout, or
  prompt-constraint leakage
- next pass should focus on separating response constraints from in-world prose
  before chasing another materialized Ink branch
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
- treat the character agent/responder as a training target too: an
  Aetheria-tuned local model can eventually learn lore, tone, factional
  preconceptions, species affordances, and institutional pressure from a corpus
  of strong responses spread throughout the timeline plus reviewed Ghostlight
  receipts, while still acting only inside character-local context and
  deterministic mutation gates
- require responder sandboxing before gold data generation: exact
  responder-visible input, raw output, reviewed output, hidden-context refs,
  leakage audit, isolation method, and coordinator repair labels must be saved

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
