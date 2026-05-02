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

3. Tighten research-enabled responder capture.
   - The first no-fork research-enabled Sella sample and reviewed mutation receipt now exist.
   - Preserve `consulted_refs`, `followed_refs`, `research_trace`, and `research_summary` inside raw responder output when the responder or runner produces them; mirror indexable provenance into capture metadata.
   - Treat research scope as seed docs plus bounded relevant traversal, not a four-file pinhole.
   - Require PSC, territory, resident-faction, technology, and social-movement context when it materially affects the scene.
   - Require `runner_captured` research trace status for accepted research-enabled gold data; coordinator-reconstructed trace is useful draft audit, not proof of the responder's actual research path.
   - The Veyr/Callisto provenance sample is the first accepted coordinator-scoped retrieval lane with `runner_captured` trace. Use it as the current pattern for trace-backed gold candidates.
   - Continue rejecting archived local-model prototype runners for gold responder data unless the task is explicitly model plumbing.

4. Generate the next reviewed sample.
   - Start from `examples/agent-state.cold-wake-story-lab.after-isdra-limited-contact.json`.
   - Build the next projected local context and responder packet from reviewed state, not from coordinator memory.
   - The next likely beat is Sella receiving Isdra's limited-contact authorization plus Maer's Navigator ledger signature and deciding whether the conditional bay path can receive contact under isolation/decon constraints.
   - Run the next no-fork sandboxed responder/coordinator step with exact visible input, raw output, runner-captured research trace, leakage audit, and review labels.
   - Convert accepted output into mutation receipts.
   - Keep fuzzy social changes manual and auditable until appraiser/classifier models exist.

5. Generalize the loop.
   - Add more historical grounded fixtures from AetheriaLore.
   - Add future-branch Elysium fixtures with branch lineage and constraint labels.
   - Track coverage across every major faction, minor faction, movement, and
     major flashpoint; major factions require both founding-era and
     day-in-the-life stories.
   - Require cultural collision in coverage stories so training data captures
     inter-faction dynamics, mutual misreads, and movement pressure instead of
     sealed faction portraits.
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
