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

The active path is a source-grounded branching-scene data loop:

1. Source-ground Aetheria context.
2. Project canonical state into character-local operating context.
3. Build an exact responder packet.
4. Run a sandboxed responder with only allowed packet/context/research scope.
5. Capture raw output, consulted refs, research summary, reviewed output,
   leakage audit, and coordinator interventions.
6. Apply only reviewed mutations into state, memory, relationships, perceived
   overlays, and unresolved hooks.
7. Let the coordinator carry continuity, unresolved hooks, glue prose, and the
   next scene/beat plan.
8. Let the branch compiler materialize playable Ink plus sidecar and compiler
   notes from reviewed state and branch decisions.
9. Run IF, narrative, lore, spatial, and visual review before accepting a
   fixture as training data.

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
- Corpus coverage ledger: `state/corpus-coverage.json`
- Corpus coverage command: `npm run coverage:status`
- Sandboxed responder prompt template: `prompts/sandboxed-responder-packet.md`
- Research sanity responder prompt template: `prompts/research-sanity-responder.md`
- Narrative quality reviewer prompt: `prompts/narrative-quality-reviewer.md`
- Lore grounding reviewer prompt: `prompts/lore-grounding-reviewer.md`
- Spatial/geometric reviewer prompt: `prompts/spatial-geometric-cohesion-reviewer.md`
- Visual scene continuity reviewer prompt: `prompts/visual-scene-continuity-reviewer.md`
- Illustrated IF visual pipeline: `docs/architecture/illustrated-if-visual-pipeline.md`
- Branch compiler and IF reviewer contract: `docs/architecture/ink-branching-scenes.md`

## Live Fixture

Pallas Species Strikes is the current accepted-as-draft interactive-fiction
scaffold:

- `examples/lore-grounding/pallas-species-strikes.awakened-labor.v0.json`
- `examples/agent-state.pallas-species-strikes.v0.json`
- `examples/coordinator/pallas-species-strikes.branch-and-fold.v0.json`
- `examples/ink/pallas-species-strikes.branch-and-fold.v0.ink`
- `examples/ink/pallas-species-strikes.branch-and-fold.v0.training.json`
- `examples/visual/pallas-species-strikes.branch-and-fold.v0.visual.json`
- `experiments/pallas-species-strikes/pallas-species-strikes.branch-and-fold-clean-run.v0.md`
- `experiments/pallas-species-strikes/pallas-species-strikes.visual-scene-review.v1.json`

It starts in ordinary AU Bloom cavity-shell maintenance texture, introduces the
major participants before branch-only deepening, then escalates through Nara-7's
coordinated hazard refusal, baseline rigger solidarity pressure, supplier
liability framing, AU dry-operation cephalopod shell-artery leverage, and a
folded recognition/safety climax. Treat it as coordinator/IF scaffold training
data, not raw responder gold.

Kappa geometry to preserve: Service Ring Kappa names this specific standardized
industrial spoke manifold service ring, a bounded local access loop around a
dense seal-air-thermal equipment cluster. The loop-inner side exposes manifold
faces and Kappa access ports on the machinery cluster. The loop-outer side holds
worker staging, the baseline anchor rail, lockers, carts, break ledge, and
access back toward yard systems. Kappa-7 False-Seam Artery, worker slang
`Rell's Cut`, is the specific dangerous passage. The Kappa operations gallery
sees the primary manifold/access-port segment, not the whole loop.

Visual pipeline lesson: generated concept images are useful as vibe/material
reference, but they do not preserve room layout. Durable illustrated replay
needs a camera-specific blockout or scene-set source; imagegen handles the
painterly pass.

## Aetheria Doctrine

- Call of the Void remains a consumer target for content generation and dialogue scaffolding.
- Elysium futures are branch-local canon indexed by lineage.
- Aetheria writing should support tonal range: warmth, absurdity, humane
  observation, institutional comedy, domestic texture, noir, horror, wonder,
  romance, dour poetry, procedural investigation, and dry systems prose.
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
- Corpus completeness requires broad Aetheria coverage. Generate stories for
  every major faction, minor faction, movement, and major flashpoint. Major
  factions need both founding-era and day-in-the-life stories. All coverage
  stories should involve cultural collision.
- Current rough corpus counts from AetheriaLore: 12 major powers, 21 minor
  powers, 23 movements, 24 Pre-Elysium flashpoints, and 5 defunct/absorbed
  powers. Baseline coverage is 92 story fixtures, or 97 with defunct powers;
  the practical first broad target is 100-150 reviewed fixtures.

## Cleanup Note

The old prototype fixture receipts and model-runner files were removed
from the live workspace after their useful lessons were distilled into the
architecture docs, state map, and git history. Do not resurrect them as active
steering surfaces unless the user explicitly asks for archaeology.

## Current Next Action

Use the corpus coverage ledger to choose the next Aetheria fixture from the
100-150 broad coverage target. Current coverage begins with Pallas Species
Strikes as the first accepted-as-draft row.

## Warnings

- Keep persistent state small. Git history and artifacts own chronology.
- Do not let notes, map, handoff, and evidence become four versions of the same brain.
- Do not train on coordinator-repaired responder prose unless the repair is labeled.
- Do not accept branch fixtures without branch compiler notes and IF artifact review.
- Do not let tracked variables, relationship bands, or visual modifiers remain decorative.
- Do not let Elysium branch futures overwrite Sol single-history facts.
- Do not let state-grounded prose become one crisis-procedure voice.
- Treat subagent research as self-reported unless the worker returns explicit research notes or a future runner captures calls.
