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

## Current Next Action

Run a sandboxed no-fork research-enabled responder against
`examples/responder-packets/scene-02-sanctuary-intake.sella_ren.packet.research.v0.json`,
allowing only the declared AetheriaLore scope. Capture exact visible input,
consulted refs, research summary, raw output, review labels, leakage audit, and
any coordinator intervention. Review whether the output uses institutional lore
and latent pressure without prompt parroting, omniscience, or mission-critical
trauma-dumping. Do not use archived local-model prototype runners for gold
responder data.

## Warnings

- Keep persistent state small. Git history and artifacts own chronology.
- Do not let notes, map, handoff, and evidence become four versions of the same brain.
- Do not train on coordinator-repaired responder prose unless the repair is labeled.
- Do not let Elysium branch futures overwrite Sol single-history facts.
- Do not rebuild the older corridor-crisis game slice by accident.
