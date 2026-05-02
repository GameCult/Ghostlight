# Ghostlight Implementation Plan

## Current Phase

Build the first reliable data-generation loop for socially persistent Aetheria
agents. The immediate target is not a full simulation. It is a clean, reviewed,
sandboxed training-data pipeline for branching scenes and state consequences.

## Near-Term Sequence

1. Stabilize canonical state and projection seams.
   - Keep `schemas/agent-state.schema.json` and required vectors coherent.
   - Keep canonical state separate from perceived overlays.
   - Keep voice, presentation, relationship, memory, and situational pressure explainable.
   - Do not leak raw numeric state internals into responder prompt text.

2. Stabilize responder packet and output seams.
   - Use `schemas/responder-packet.schema.json` and `schemas/responder-output.schema.json`.
   - Build packets from coordinator artifacts plus projected local context.
   - Preserve exact responder-visible input, hidden-context audit, allowed actions, source excerpts, output contract, and lore-access mode.
   - Preserve raw output, parsed output, review labels, consulted refs, research summary, leakage audit, and coordinator interventions.

3. Generate the next research-enabled responder sample.
   - Use `examples/responder-packets/scene-02-sanctuary-intake.sella_ren.packet.research.v0.json`.
   - Run a no-fork sandboxed responder with only the packet and declared lore scope.
   - Require actual lore consultation before answering.
   - Review for institutional grounding, latent character pressure, non-omniscience, prompt parroting, and mission-critical trauma-dumping.
   - Do not use archived local-model prototype runners for gold responder data.

4. Materialize reviewed consequences.
   - Convert accepted responder output into a mutation receipt.
   - Update only reviewed scene state, memories, relationship stance, perceived overlays, and unresolved hooks.
   - Keep fuzzy social changes manual and auditable until appraiser/classifier models exist.

5. Generalize the loop.
   - Add more historical grounded fixtures from AetheriaLore.
   - Add future-branch Elysium fixtures with branch lineage and constraint labels.
   - Emit technology/item manifest deltas when scenes discover or stress gear, assemblies, supply chains, or faction tech bases.
   - Keep artifacts database-shaped enough for future game-engine integration.

6. Train only after schemas stop sliding.
   - Pilot samples are schema shakedown, not robust corpus scale.
   - Specialized future models include coordinator, retriever, projector,
     responder, appraiser, mutator, relationship/perception updater, evaluator,
     and institution/faction/consumer decision models.
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
- Keep history in git and archived receipts; keep persisted state focused on the mission.
