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
memory, perception, incentives, material constraints, and scene pressure.

The live product-facing goal is procedural branching scene generation for games:
speech and non-speech actions, NPC responses, consequences, future hooks, and
state/memory/social-perception mutation. The wider ambition includes narrated
story scaffolding, Elysium branch exploration, psychologically grounded consumer
behavior, major faction decision-making, and technology/item manifest generation.

## Current Focus

The active path is not the archived local-model prototype runner. The active
path is:

1. Source-ground Aetheria context.
2. Project canonical state into character-local operating context.
3. Build an exact responder packet.
4. Run a sandboxed responder with only the allowed packet/context/research scope.
5. Capture raw output, consulted refs, research summary, reviewed output,
   leakage audit, and coordinator interventions.
6. Apply only reviewed mutations into state, memory, relationships, perceived
   overlays, and unresolved hooks.

## Live Seams

- Canonical map: `state/map.yaml`
- Current system map: `notes/ghostlight-current-system-map.md`
- Implementation plan: `notes/ghostlight-implementation-plan.md`
- Agent-state schema: `schemas/agent-state.schema.json`
- Projected local context schema: `schemas/projected-local-context.schema.json`
- Coordinator artifact schema: `schemas/coordinator-artifact.schema.json`
- Responder packet schema: `schemas/responder-packet.schema.json`
- Responder output schema: `schemas/responder-output.schema.json`
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

Archived prototype captures and tools remain as evidence under `experiments/`,
`examples/ink/`, and old `run_qwen_*` helpers. They are useful receipts, not the
current gold-data route.

## Aetheria Doctrine

- Cold Wake is a historical pre-Elysium grounding fixture, not the product target.
- Call of the Void remains a consumer target for content generation and dialogue scaffolding.
- Elysium futures are branch-local canon indexed by lineage.
- Before generating Aetheria scenes, check lore for factions, movements,
  institutions, species/body affordances, location, time period, infrastructure,
  and material constraints.
- If a blocking lore gap appears, patch AetheriaLore narrowly before treating the
  detail as available to Ghostlight.
- Technology exploration should emit database-shaped item/assembly/component/
  supply-chain candidates mapped toward Aetheria-Economy's CultCache item model.
  `CompoundCommodityData` includes colony-consumed manufactured trade goods, not
  only gear assemblies or manufacturing inputs; a Navigator wet-interface cradle
  is the current example of generated texture that should become a possible
  demand-profiled trade-good candidate.
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

The first no-fork research-enabled Sella responder sample exists and has a reviewed mutation receipt. It carries `coordinator_reconstructed` trace entries and remains draft-grade audit data, not proof of the responder's actual tool path. The Veyr/Callisto, Isdra convoy-threshold, and Sella authorized-contact samples are accepted coordinator-scoped retrieval passes with `runner_captured` traces.

Continue from `examples/agent-state.cold-wake-story-lab.after-sella-bay-opened.json`. The next pass should be a bounded end-to-end Cold Wake short story run, not another isolated beat unless the run exposes a core invariant failure. Start with Maer accepting Sella's bounded diagnostic-contact conditions and initiating actual vessel contact or extraction, then carry the story toward a readable ending. Keep personhood, source-chain provenance, and final rescue outcome unresolved until evidence reaches the scene. Emit training artifacts at material turns and preserve projected context, packet, raw responder output, leakage audit, reviewed mutation receipt, and aftermath state where relevant. Coordinator glue prose is part of the training signal. Do not stop for every minor prose flaw; curate and keep moving unless there is hidden-state leakage, lore grounding failure, visibility/object/body affordance error, bad state mutation, or schema break.

## Warnings

- Keep persistent state small. Git history and artifacts own chronology.
- Do not let notes, map, handoff, and evidence become four versions of the same brain.
- Do not train on coordinator-repaired responder prose unless the repair is labeled.
- Do not let Elysium branch futures overwrite Sol single-history facts.
- Do not rebuild the older corridor-crisis game slice by accident.
- Do not treat parent-visible subagent output as proof of research. If the
  parent cannot see tool calls, require explicit research trace entries or use a
  runner that captures calls. Accepted research-enabled gold data needs
  `runner_captured` trace status.
