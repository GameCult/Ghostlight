# Ghostlight Fresh Workspace Handoff

Do not trust this file for the exact live HEAD. Use git for volatile truth.

Do not continue implementation automatically from a rehydrate-only request.

## Immediate Re-entry Instruction

1. Read `state/map.yaml`.
2. Read `notes/ghostlight-current-system-map.md`.
3. Read `notes/ghostlight-implementation-plan.md`.
4. Run `npm run state:status`.
5. Restate the persisted next action before touching code or docs.

## Mission

Ghostlight exists to build socially persistent generative agents for Aetheria:
characters and institutions whose actions emerge from personality, culture,
memory, perception, incentives, material constraints, ordinary life, tonal mode,
and scene pressure.

The live product-facing goal is procedural branching scene generation for games:
speech and non-speech actions, NPC responses, consequences, future hooks, and
state/memory/social-perception mutation. The wider ambition includes narrated
story scaffolding, Elysium branch exploration, psychologically grounded consumer
behavior, major faction decision-making, and technology/item manifest generation.

## Current Focus

The active path is not the archived local-model prototype runner. The active
path is a source-grounded branching-scene data loop:

1. Source-ground Aetheria context.
2. Project canonical state into character-local operating context.
3. Build an exact responder packet.
4. Run a sandboxed responder with only the allowed packet/context/research scope.
5. Capture raw output, consulted refs, research summary, reviewed output,
   leakage audit, and coordinator interventions.
6. Apply only reviewed mutations into state, memory, relationships, perceived
   overlays, and unresolved hooks.
7. Let the coordinator carry continuity, unresolved hooks, glue prose, and the
   next scene/beat plan.
8. Let the branch compiler materialize playable Ink plus sidecar and compiler
   notes from reviewed state and branch decisions.
9. Run IF artifact review before accepting the fixture as training data.

## Live Seams

- Canonical map: `state/map.yaml`
- Current system map: `notes/ghostlight-current-system-map.md`
- Implementation plan: `notes/ghostlight-implementation-plan.md`
- Documentation inventory: `notes/documentation-inventory.md`
- Agent-state schema: `schemas/agent-state.schema.json`
- Projected local context schema: `schemas/projected-local-context.schema.json`
- Coordinator artifact schema: `schemas/coordinator-artifact.schema.json`
- Responder packet schema: `schemas/responder-packet.schema.json`
- Responder output schema: `schemas/responder-output.schema.json`
- Prompt folder: `prompts/`
- Sandboxed responder prompt template: `prompts/sandboxed-responder-packet.md`
- Research sanity responder prompt template: `prompts/research-sanity-responder.md`
- Narrative quality reviewer prompt: `prompts/narrative-quality-reviewer.md`
- Visual scene continuity reviewer prompt:
  `prompts/visual-scene-continuity-reviewer.md`
- Research-enabled Sella packet: `examples/responder-packets/scene-02-sanctuary-intake.sella_ren.packet.research.v0.json`
- Research-enabled Sella capture: `experiments/responder-packets/cold-wake-sanctuary-intake-sella-research-enabled-v0.capture.json`
- Research-enabled Sella mutation: `experiments/responder-packets/cold-wake-sanctuary-intake-sella-research-enabled-v0.mutation.json`
- Research-enabled aftermath state: `examples/agent-state.cold-wake-story-lab.after-sella-research-conditions.json`
- Trace-backed Veyr projected context: `examples/projected-contexts/scene-03-callisto-provenance.veyr_oss.projected-context.json`
- Trace-backed Veyr packet: `examples/responder-packets/scene-03-callisto-provenance.veyr_oss.packet.trace.v0.json`
- Trace-backed Veyr coordinator artifact: `examples/coordinator/cold-wake-callisto-provenance.v0.json`
- Trace-backed Veyr capture: `experiments/responder-packets/cold-wake-callisto-provenance-veyr-trace-v0.capture.json`
- Trace-backed Veyr mutation: `experiments/responder-packets/cold-wake-callisto-provenance-veyr-trace-v0.mutation.json`
- Trace-backed Veyr aftermath state: `examples/agent-state.cold-wake-story-lab.after-veyr-category-offer.json`
- Trace-backed Isdra projected context: `examples/projected-contexts/scene-04-convoy-threshold.isdra_vel.projected-context.json`
- Trace-backed Isdra packet: `examples/responder-packets/scene-04-convoy-threshold.isdra_vel.packet.trace.v0.json`
- Trace-backed Isdra coordinator artifact: `examples/coordinator/cold-wake-convoy-threshold.v0.json`
- Trace-backed Isdra capture: `experiments/responder-packets/cold-wake-convoy-threshold-isdra-trace-v0.capture.json`
- Trace-backed Isdra mutation: `experiments/responder-packets/cold-wake-convoy-threshold-isdra-trace-v0.mutation.json`
- Trace-backed Isdra aftermath state: `examples/agent-state.cold-wake-story-lab.after-isdra-limited-contact.json`
- Trace-backed Sella authorized-contact projected context: `examples/projected-contexts/scene-05-sanctuary-authorized-contact.sella_ren.projected-context.json`
- Trace-backed Sella authorized-contact packet: `examples/responder-packets/scene-05-sanctuary-authorized-contact.sella_ren.packet.trace.v0.json`
- Trace-backed Sella authorized-contact coordinator artifact: `examples/coordinator/cold-wake-sanctuary-authorized-contact.v0.json`
- Trace-backed Sella authorized-contact capture: `experiments/responder-packets/cold-wake-sanctuary-authorized-contact-sella-trace-v0.capture.json`
- Trace-backed Sella authorized-contact mutation: `experiments/responder-packets/cold-wake-sanctuary-authorized-contact-sella-trace-v0.mutation.json`
- Trace-backed Sella authorized-contact aftermath state: `examples/agent-state.cold-wake-story-lab.after-sella-bay-opened.json`
- Research sanity probe: `experiments/research-sanity/navigator-embodiment-sanity-v0.capture.json`
- Packet builder: `tools/build_responder_packet.py`
- Packet/output validators: `tools/validate_responder_packets.py`, `tools/validate_responder_outputs.py`
- Clean Cold Wake IF scaffold: `examples/ink/cold-wake-branch-and-fold.v0.ink`
- Clean Cold Wake IF sidecar: `examples/ink/cold-wake-branch-and-fold.v0.training.json`
- Clean Cold Wake coordinator artifact: `examples/coordinator/cold-wake-branch-and-fold.v0.json`
- Clean Cold Wake readable run: `experiments/cold-wake-story-lab/cold-wake-branch-and-fold-clean-run.v0.md`
- Pallas Species Strikes lore digest: `examples/lore-grounding/pallas-species-strikes.awakened-labor.v0.json`
- Pallas Species Strikes agent state: `examples/agent-state.pallas-species-strikes.v0.json`
- Pallas Species Strikes coordinator artifact: `examples/coordinator/pallas-species-strikes.branch-and-fold.v0.json`
- Pallas Species Strikes Ink scaffold: `examples/ink/pallas-species-strikes.branch-and-fold.v0.ink`
- Pallas Species Strikes Ink sidecar: `examples/ink/pallas-species-strikes.branch-and-fold.v0.training.json`
- Pallas Species Strikes visual plan: `examples/visual/pallas-species-strikes.branch-and-fold.v0.visual.json`
- Pallas Species Strikes readable run: `experiments/pallas-species-strikes/pallas-species-strikes.branch-and-fold-clean-run.v0.md`
- Branch compiler and IF reviewer contract: `docs/architecture/ink-branching-scenes.md`

Archived prototype captures and tools remain as evidence under `experiments/`,
`examples/ink/`, and old `run_qwen_*` helpers. They are useful receipts, not the
current gold-data route.

## Aetheria Doctrine

- Cold Wake is a historical pre-Elysium grounding fixture, not the product target.
- Call of the Void remains a consumer target for content generation and dialogue scaffolding.
- Elysium futures are branch-local canon indexed by lineage.
- Aetheria writing should not collapse into constant crisis or unmarked dry
  technical severity. Adams/Pratchett-style wit-with-stakes is a useful default
  touchstone: warmth, absurdity, humane observation, and institutional comedy
  can carry real consequence. Other controlled modes are valid too, including
  domestic comedy, noir, horror, wonder, romance, dour poetry, procedural
  investigation, and dry systems prose.
- Before writing conflict, establish the life being interrupted when the fixture
  calls for it: routines, work texture, relationships, private jokes, rituals,
  status games, habits, small desires, and ordinary accommodations.
- Before generating Aetheria scenes, check lore for factions, movements,
  institutions, species/body affordances, location, time period, infrastructure,
  and material constraints.
- If a blocking lore gap appears, patch AetheriaLore narrowly before treating the
  detail as available to Ghostlight.
- Technology exploration should emit database-shaped item/assembly/component/
  supply-chain candidates mapped toward Aetheria-Economy's CultCache item model.
  `CompoundCommodityData` includes colony-consumed manufactured trade goods, not
  only gear assemblies or manufacturing inputs; a specialized habitat support
  rig or species-specific workstation is the current example of generated
  texture that should become a possible demand-profiled trade-good candidate.
- Corpus completeness requires broad Aetheria coverage, not repeated Cold Wake
  refinement. Generate stories for every major faction, minor faction,
  movement, and major flashpoint. Major factions need both founding-era and
  day-in-the-life stories. All coverage stories should involve cultural
  collision so faction and movement dynamics appear in the training data.
- Current rough corpus counts from AetheriaLore: 12 major powers, 21 minor
  powers, 23 movements, 24 Pre-Elysium flashpoints, and 5 defunct/absorbed
  powers. Baseline coverage is 92 story fixtures, or 97 with defunct powers;
  the practical first broad target is 100-150 reviewed fixtures, not exhaustive
  pairwise combinations.

## Current Next Action

Pallas Species Strikes now has a reviewer-revised accepted-as-draft
interactive-fiction scaffold:

- `examples/lore-grounding/pallas-species-strikes.awakened-labor.v0.json`
- `examples/agent-state.pallas-species-strikes.v0.json`
- `examples/coordinator/pallas-species-strikes.branch-and-fold.v0.json`
- `examples/ink/pallas-species-strikes.branch-and-fold.v0.ink`
- `examples/ink/pallas-species-strikes.branch-and-fold.v0.training.json`
- `examples/visual/pallas-species-strikes.branch-and-fold.v0.visual.json`
- `experiments/pallas-species-strikes/pallas-species-strikes.branch-and-fold-clean-run.v0.md`

It starts in ordinary AU Bloom cavity-shell maintenance texture, introduces the
major participants before branch-only deepening, then escalates through Nara-7's
coordinated hazard refusal, baseline rigger solidarity pressure, BioElevate
liability framing, AU dry-operation cephalopod crawl leverage, and a folded
recognition/safety climax. The Ink now gates Nara's marked path and Teth's
bypass, gives `evidence_of_sentience`, `nara_centered`, and
`formal_stoppage` later reads, and gives visual plan states explicit trigger
mappings. It now segments the introduction, branch locations, Kappa red alert,
climax decision, and aftermath into click-through visual sections with
`visual_scene_id` anchors, stable `visual_character_refs`, a global Media
Molecule Dreams-style volumetric brushstroke cue, and imagegen-ready prompts
that explain fictional equipment in visible terms. Its location grounding now
uses the Bloom process correctly: engineered rotating shell, consolidated
asteroid rubble/aggregate, TCS substrate, seals, supports, and service systems,
not a cave carved into a solid asteroid. Treat it as coordinator/IF scaffold
training data, not raw responder gold.

Next: add the corpus coverage ledger seam or choose the next Aetheria fixture
from the 100-150 broad coverage target.

## Warnings

- Keep persistent state small. Git history and artifacts own chronology.
- Do not let notes, map, handoff, and evidence become four versions of the same brain.
- Do not train on coordinator-repaired responder prose unless the repair is labeled.
- Do not accept branch fixtures without branch compiler notes and IF artifact
  review once that seam is available.
- Do not let tracked variables, authority pressure, heat, evidence quality,
  relationship bands, or visual modifiers remain decorative. If they are
  tracked, they need consequence or an explicit telemetry-only label.
- Do not let Elysium branch futures overwrite Sol single-history facts.
- Do not rebuild the older corridor-crisis game slice by accident.
- Do not let "state-grounded" become "everyone speaks in crisis-procedure
  clauses." State also explains jokes, quiet, tenderness, ritual, awkwardness,
  boredom, and comfort.
- Do not treat parent-visible subagent output as proof of research. If the
  parent cannot see tool calls, require explicit research trace entries or use a
  runner that captures calls. Accepted research-enabled gold data needs
  `runner_captured` trace status.
