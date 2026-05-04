# Ghostlight Implementation Plan

## Current Phase

Build a reliable data-generation loop for socially persistent Aetheria agents.
The immediate target is not a full simulation. It is a clean, reviewed,
sandboxed training-data pipeline for branching scenes and state consequences.
The output should preserve Aetheria's tonal range: wit with stakes, ordinary
life before interruption, quiet ritual, domestic warmth, weirdness, dread, and
systems pressure as needed.

Planning belongs in this file and the handoff/map state surfaces. Architecture
docs should describe durable contracts, boundaries, and data shapes; they should
not carry live next-action lists.

## Near-Term Sequence

1. Stabilize canonical state and projection seams.
   - Keep `schemas/agent-state.schema.json` and required vectors coherent.
   - Keep canonical state separate from perceived overlays.
   - Keep voice, presentation, relationship, memory, and situational pressure explainable.
   - Add or preserve tonal-mode and ordinary-life cues where they affect prose.
   - Do not leak raw numeric state internals into responder prompt text.

2. Stabilize responder packet and output seams.
   - Use `schemas/responder-packet.schema.json` and `schemas/responder-output.schema.json`.
   - Build packets from coordinator artifacts plus projected local context.
   - Preserve exact responder-visible input, hidden-context audit, allowed actions, source excerpts, output contract, and lore-access mode.
   - Preserve raw output, parsed output, review labels, consulted refs, research summary, leakage audit, and coordinator interventions.
   - Require `runner_captured` research trace status for accepted research-enabled gold data; coordinator-reconstructed trace is useful draft audit, not proof of the responder's actual research path.

3. Keep the Pallas fixture as the current scaffold reference.
   - Current fixture set:
     `examples/lore-grounding/pallas-species-strikes.awakened-labor.v0.json`,
     `examples/agent-state.pallas-species-strikes.v0.json`,
     `examples/coordinator/pallas-species-strikes.branch-and-fold.v0.json`,
     `examples/ink/pallas-species-strikes.branch-and-fold.v0.ink`,
     `examples/ink/pallas-species-strikes.branch-and-fold.v0.training.json`,
     `examples/visual/pallas-species-strikes.branch-and-fold.v0.visual.json`,
     and `experiments/pallas-species-strikes/` receipts.
   - Treat it as accepted-as-draft coordinator/IF scaffold training data, not raw responder gold.
   - Preserve the lessons it established: ordinary-life onboarding, branch-and-fold discipline, material state variables, source-backed lore elaboration, visual segmentation, scene-set need for illustrated replay, and review before acceptance.

4. Generalize the loop.
   - Use the corpus coverage ledger so future fixtures can be tracked by faction, movement, flashpoint, tonal mode, training target, review status, and cultural collisions.
   - Select the next Aetheria fixture from the 100-150 broad coverage target.
   - Add more historical grounded fixtures from AetheriaLore.
   - Add future-branch Elysium fixtures with branch lineage and constraint labels.
   - Track coverage across every major faction, minor faction, movement, and major flashpoint; major factions require both founding-era and day-in-the-life stories.
   - Require cultural collision in coverage stories so training data captures inter-faction dynamics, mutual misreads, and movement pressure.
   - Vary tonal modes across fixtures so the corpus does not train one flattened Aetheria voice.
   - Emit technology/item manifest deltas when scenes discover or stress gear, assemblies, supply chains, or faction tech bases.
   - Keep artifacts database-shaped enough for future game-engine integration.

5. Train only after schemas stop sliding.
   - Pilot samples are schema shakedown, not robust corpus scale.
   - Specialized future models include coordinator, retriever, projector,
     responder, appraiser, mutator, relationship/perception updater, branch
     compiler, IF artifact reviewer, evaluator, and institution/faction/consumer
     decision models.
   - Deterministic gates remain code: visibility, action legality, object
     custody, resource accounting, schema validation, source provenance,
     mutation authority, prompt leakage checks, and Ink compilation.

## Deferred

- Full world simulation loop
- Autonomous offscreen faction simulation
- Economy simulation loop
- Long-horizon plot generation without author scaffolding
- Fine-tuning before artifact schemas, review criteria, and evaluators stabilize

## Discipline

- Work one bounded organ at a time.
- Prefer explicit maps and contracts over implicit context.
- If the diff grows while understanding shrinks, stop and simplify.
- Keep history in git and distilled evidence; keep the live workspace focused on the mission.
