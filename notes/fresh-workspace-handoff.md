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
7. Update initiative scheduling from reviewed state: readiness, recovery,
   current load, status, reaction windows, and next actor eligibility.
8. Run the coordinator as its own sandboxed, promptable organ whenever
   generating training-shaped scene loops. Capture exact coordinator input, raw
   coordinator output, parsed artifact, review labels, interventions, and
   rejected alternatives.
9. Let Codex act as meta-coordinator: scope the organs, launch sandboxed
   workers, review outputs, repair only with labels, wire structured state, and
   preserve receipts. Do not silently treat omniscient chat steering as raw
   coordinator training data.
10. Let the accepted coordinator artifact carry continuity, unresolved hooks,
   glue prose, and the next scene/beat plan.
11. Let the branch compiler materialize playable Ink plus sidecar and compiler
   notes from reviewed state and branch decisions.
12. Run IF, narrative, lore, spatial, and visual review before accepting a
   fixture as training data.

## Live Seams

- Canonical map: `state/map.yaml`
- CultCache-backed state spine: `state/ghostlight-state.cultcache.jsonl`
- CultCache-Py submodule: `vendor/cultcache-py`
- Ghostlight state cache seam: `tools/ghostlight_state_store.py`
- Readable state exports during migration: `state/branches.json`, `state/evidence.jsonl`
- Current system map: `notes/ghostlight-current-system-map.md`
- Implementation plan: `notes/ghostlight-implementation-plan.md`
- Documentation inventory: `notes/documentation-inventory.md`
- Agent-state schema: `schemas/agent-state.schema.json`
- Projected local context schema: `schemas/projected-local-context.schema.json`
- Coordinator artifact schema: `schemas/coordinator-artifact.schema.json`
- Responder packet schema: `schemas/responder-packet.schema.json`
- Responder output schema: `schemas/responder-output.schema.json`
- Event record schema: `schemas/event-record.schema.json`
- Participant appraisal schema: `schemas/participant-appraisal.schema.json`
- Reviewed mutation schema: `schemas/reviewed-mutation.schema.json`
- Scene-loop bundle schema: `schemas/scene-loop-bundle.schema.json`
- Initiative schedule schema: `schemas/initiative-schedule.schema.json`
- Training-loop validator: `npm run training-loop:validate`
- Initiative schedule validator: `npm run initiative:validate`
- Prompt folder: `prompts/`
- Corpus coverage ledger: `state/corpus-coverage.json`
- Corpus coverage command: `npm run coverage:status`
- Sandboxed responder prompt template: `prompts/sandboxed-responder-packet.md`
- Research sanity responder prompt template: `prompts/research-sanity-responder.md`
- Narrative quality reviewer prompt: `prompts/narrative-quality-reviewer.md`
- Lore grounding reviewer prompt: `prompts/lore-grounding-reviewer.md`
- Spatial/geometric reviewer prompt: `prompts/spatial-geometric-cohesion-reviewer.md`
- Visual scene continuity reviewer prompt: `prompts/visual-scene-continuity-reviewer.md`
- Interactive Fiction Weaver prompt: `prompts/interactive-fiction-weaver.md`
- Illustrated IF visual pipeline: `docs/architecture/illustrated-if-visual-pipeline.md`
- Branch compiler and IF reviewer contract: `docs/architecture/ink-branching-scenes.md`

## Live Fixtures

Pallas Species Strikes is the current reference-only interactive-fiction story
fixture:

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
folded recognition/safety climax. Treat it as a reference for well-shaped,
source-grounded story and illustrated IF presentation. It is not training-ready
data for the responder, projector, appraiser, mutator, relationship updater,
coordinator, branch compiler, or reviewer organs because it lacks exact
stage-shaped input/output receipts.

The first separate training-shaped derivative is `pallas-training-loop-v0`.
It rebuilds only the Kappa refusal / Ilya arrival threshold segment as a
receipt-chain pilot, leaving the reference fixture untouched:

- `examples/training-loops/pallas-training-loop-v0/pallas-training-loop-v0.scene-digest.json`
- `examples/training-loops/pallas-training-loop-v0/pallas-training-loop-v0.initial-state.json`
- `examples/projected-contexts/pallas-training-loop-v0.turn-01.nara-7.projected-context.json`
- `examples/projected-contexts/pallas-training-loop-v0.turn-02.ilya-marne.projected-context.json`
- `examples/projected-contexts/pallas-training-loop-v0.turn-03.lio-vale.projected-context.json`
- `examples/responder-packets/pallas-training-loop-v0.turn-01.nara-7.packet.json`
- `examples/responder-packets/pallas-training-loop-v0.turn-02.ilya-marne.packet.json`
- `examples/responder-packets/pallas-training-loop-v0.turn-03.lio-vale.packet.json`
- `experiments/responder-packets/pallas-training-loop-v0.turn-01.nara-7.packet.capture.json`
- `experiments/responder-packets/pallas-training-loop-v0.turn-02.ilya-marne.packet.capture.json`
- `experiments/responder-packets/pallas-training-loop-v0.turn-03.lio-vale.packet.capture.json`
- `examples/training-loops/pallas-training-loop-v0/pallas-training-loop-v0.bundle.json`
- `experiments/pallas-training-loop-v0/pallas-training-loop-v0.clean-run.md`

The derivative has exact packet prompts, raw no-fork subagent outputs, event
records, participant-local appraisals from current character state refs,
reviewed mutation receipts, coordinator continuity receipts, and a coverage row.
It is training-shaped for projector, responder, event resolver, appraiser,
mutator, relationship/perception updater, and coordinator/story-runtime seams.
It is not branch compiler, IF reviewer, or visual training data.

`lucent-hostage-feed-v0` is the current open-ended training-loop draft. It moves
the pipeline away from Pallas overfitting into a darkly comic Lucent Media
hostage-feed negotiation set in a tethered counterweight station: a giant
media-headquarters eye staring down the tether at a metropolis bubble.

Live Lucent surfaces:

- `examples/training-loops/lucent-hostage-feed-v0/lucent-hostage-feed-v0.scene-digest.json`
- `examples/training-loops/lucent-hostage-feed-v0/lucent-hostage-feed-v0.initial-state.json`
- `examples/training-loops/lucent-hostage-feed-v0/lucent-hostage-feed-v0.branch-surface.json`
- `examples/training-loops/lucent-hostage-feed-v0/lucent-hostage-feed-v0.bundle.json`
- `examples/training-loops/lucent-hostage-feed-v0/lucent-hostage-feed-v0.review.json`
- `examples/initiative/lucent-hostage-feed-v0.turn-20.initiative.json`
- `experiments/lucent-hostage-feed-v0/lucent-hostage-feed-v0.clean-run.md`
- `experiments/lucent-hostage-feed-v0/lucent-hostage-feed-v0.staging-sketch.md`
- `scripts/materialize_lucent_loop.py`
- `scripts/materialize_lucent_loop_continuation.py`
- `examples/ink/lucent-hostage-feed.branch-and-fold.v0.ink`
- `examples/ink/lucent-hostage-feed.branch-and-fold.v0.training.json`
- `examples/ink/lucent-hostage-feed.negotiator-branch.v1.ink`
- `examples/ink/lucent-hostage-feed.negotiator-branch.v1.training.json`
- `experiments/lucent-hostage-feed-v0/sandbox-packets/lucent-hostage-feed.negotiator-branch.v1.coordinator-packet.md`
- `experiments/lucent-hostage-feed-v0/sandbox-captures/lucent-hostage-feed.negotiator-branch.v1.coordinator.capture.json`
- `experiments/lucent-hostage-feed-v0/sandbox-packets/lucent-hostage-feed.negotiator-branch.v1.weaver-packet.md`
- `experiments/lucent-hostage-feed-v0/reviews/lucent-hostage-feed.negotiator-branch.v1.narrative-review.final.json`
- `experiments/lucent-hostage-feed-v0/reviews/lucent-hostage-feed.negotiator-branch.v1.lore-review.final.json`
- `experiments/lucent-hostage-feed-v0/reviews/lucent-hostage-feed.negotiator-branch.v1.spatial-review.final.json`

The Lucent bundle counts fifteen post-compaction restart turns from
`turn-06` through `turn-20` as training-clean responder receipts. Earlier
opening turns established the situation and proof-object but crossed a
compaction seam, so they are treated as setup/rehearsal context rather than
gold responder data. The accepted loop starts after feed ops externalizes the
proof-object into `EDIT TRAIL HOLD` and resolves when Juno reaches the marked
safe line, security stays behind the collars, Sol is categorized for
non-contact protective handoff, and evidence export remains the live aftermath
hook.

Lucent is training-shaped for projector, responder, event resolver, appraiser,
mutator, relationship/perception updater, and coordinator/story-runtime seams.
It is not branch compiler, IF reviewer, or visual training data yet.

Lucent's strongest machine lesson: the coordinator should not shove the actors
toward a desired plot. It should make the strings fun to play with. Supply
game-ergonomic affordances, visible gates, clocks, evidence objects, posture
constraints, route access, public categories, and resource pressure; let actors
choose from local state; then fold the observed action into reviewed state.
Elastic strings, not rails. Little puppet theater, but with receipts.

Character-intelligence lesson: keep the machinery as smart as possible, but do
not make every character inherit frontier-model metacognition. Characters should
reason, manipulate, misread, explain themselves, and regulate stress only to
the level supported by their stat sheet, culture, role, experience, and current
state. The coordinator/narrator may translate high-context maneuvers for the
reader, especially in Lucent-style reputation warfare, but that explanation is
not automatically character knowledge.

Coordinator seam correction: the Lucent character responders were sandboxed
no-fork agents, but the coordinator was Codex-authored and then captured as
artifacts. Future training-shaped loops should sandbox coordinator turns too.
Codex is the meta-coordinator herding Ghostlight pieces into shape, not the raw
coordinator organ being trained.

Initiative seam: Ghostlight now has a documented and validated initiative
schedule artifact. The scheduler is mechanical: it selects eligible actors from
initiative speed, next-ready time, load, status, action recovery, and reaction
windows. It does not replace participant appraisal. Everyone affected by an
event still updates before the scheduler chooses the next projected actor.

Branch compiler seam correction: `lucent-hostage-feed.branch-and-fold.v0.ink`
is the first meta-coordinator authored branch-and-fold draft for exploring the
Lucent state space. The newer `lucent-hostage-feed.negotiator-branch.v1.ink`
was produced through a sandboxed coordinator packet and a no-fork sandboxed
Interactive Fiction Weaver. Codex then acted as meta-coordinator and applied
labeled repairs: Ink syntax, validator-supported action labels, sidecar shape,
and post-review spatial cleanup. Treat this as draft branch-compiler training
shape with labeled repairs, not pure Weaver gold.

Lucent v1 reviewer result: narrative, lore, and spatial sandboxed reviewers all
returned B-level or better after iteration. Visual review was intentionally not
run because this pass has no visual plan. The lore reviewer identified reusable
backfill candidates: Lucent crisis feed protocols, junior Reality Architect
operational roles, Lucent creator debt/arbitration machinery, and media-eye
tether stations if that station type recurs.

`corvid-collective-founding-v0` is the newest founding-era draft fixture. It
uses a sandboxed coordinator and sandboxed Interactive Fiction Weaver to stage
the 2470 First Exodus seed from the perspective of Kesh-of-Three-Clicks, an
uplifted raven using beak, talon, short flight, mimicry, cached objects, optical
sightlines, contact pads, route memory, and fallible human leverage to open a
path for the flock without letting humans become the founders.

Live Corvid surfaces:

- `examples/lore-grounding/corvid-collective-founding.v0.json`
- `examples/training-loops/corvid-collective-founding-v0/corvid-collective-founding-v0.initial-state.json`
- `examples/ink/corvid-collective-founding.branch-and-fold.v0.ink`
- `examples/ink/corvid-collective-founding.branch-and-fold.v0.training.json`
- `examples/visual/corvid-collective-founding.branch-and-fold.v0.visual.json`
- `experiments/corvid-collective-founding-v0/sandbox-packets/corvid-collective-founding-v0.coordinator-packet.md`
- `experiments/corvid-collective-founding-v0/sandbox-captures/corvid-collective-founding-v0.coordinator.capture.json`
- `experiments/corvid-collective-founding-v0/reviews/corvid-collective-founding-v0.narrative-review.final.json`
- `experiments/corvid-collective-founding-v0/reviews/corvid-collective-founding-v0.lore-review.final.json`
- `experiments/corvid-collective-founding-v0/reviews/corvid-collective-founding-v0.narrative-review.visual-enabled.json`
- `experiments/corvid-collective-founding-v0/reviews/corvid-collective-founding-v0.lore-review.visual-enabled.json`
- `experiments/corvid-collective-founding-v0/reviews/corvid-collective-founding-v0.spatial-review.final.json`
- `experiments/corvid-collective-founding-v0/reviews/corvid-collective-founding-v0.visual-review.final.json`

Corvid result: the Ink compiles, lore grounding validates, and the full-artifact
narrative reviewer accepted it with minor revisions that were applied. Visual
replay is now enabled through a separate `.visual.json` plan with click-through
scene anchors, stable visual character refs, branch/state image modifiers,
ending modifiers, explicit hatch 3C states, and a draft Blackbox Aviary 3C
scene-set map. Visual-enabled narrative and lore reviewers accepted the fixture;
spatial and visual reviewers accepted it as draft after fixes. The draft
scene-set is enough for replay planning, not high-consistency website art; build
a hand-made blockout before expecting multi-angle image generation to behave
like it went to engineering school. A limited-input narrative warning is
preserved as a reviewer-packet lesson because that reviewer received a shortened
climax excerpt and correctly complained about missing endings that did exist in
the real file. Do not count that as a true failure of the Ink.

Corvid usability: this is draft branch-and-fold/reference material for
nonhuman embodiment, concrete leverage, Corvid separatist founding texture, and
future Chaos Weaver outside-interface boundaries. It is not responder,
appraiser, mutator, relationship-updater, projector, or coordinator gold because
it lacks per-turn projected contexts, exact character packets, raw character
outputs, event/appraisal/mutation receipts, and a complete scene-loop bundle.

Corvid backfill candidates: NeuroSyn Corvid Recon room-scale containment/audit
geometry, exact raven-to-telemetry physical interface affordances, Rossum &
Douglas high-risk handoff/audit practice, and the Corvid/Chaos Weaver
outside-interface origin note.

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

Choose the next bounded move: backfill Corvid/NeuroSyn room-scale lore gaps,
convert the Corvid fixture into a true scene-loop receipt chain, backfill the
useful Lucent lore gaps into AetheriaLore, build a visual/scene-set pass for the
Lucent IF, or move to the next coverage fixture. Current accepted full-fixture
coverage is still empty; Pallas, Lucent, and Corvid derivatives are draft rows
with per-organ training readiness or reference-only value, not accepted broad
corpus fixtures.

## Warnings

- Keep persistent state small. Git history and artifacts own chronology.
- Do not let notes, map, handoff, and evidence become four versions of the same brain.
- Do not train on coordinator-repaired responder prose unless the repair is labeled.
- Do not accept branch fixtures without branch compiler notes and IF artifact review.
- Do not let tracked variables, relationship bands, or visual modifiers remain decorative.
- Do not let Elysium branch futures overwrite Sol single-history facts.
- Do not let state-grounded prose become one crisis-procedure voice.
- Treat subagent research as self-reported unless the worker returns explicit research notes or a future runner captures calls.
