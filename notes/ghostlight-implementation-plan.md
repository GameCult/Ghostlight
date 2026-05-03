# Ghostlight Implementation Plan

## Current Phase

Build the first reliable data-generation loop for socially persistent Aetheria
agents. The immediate target is not a full simulation. It is a clean, reviewed,
sandboxed training-data pipeline for branching scenes and state consequences.
The output should also preserve Aetheria's tonal range: wit with stakes,
ordinary life before interruption, quiet ritual, domestic warmth, weirdness,
dread, and systems pressure as needed. Do not let the pipeline's interest in
state and consequence turn every fixture into the same crisis-procedure voice.

Planning belongs in this file and the handoff/map state surfaces. Architecture
docs should describe durable contracts, boundaries, and data shapes; they should
not carry live next-action lists.

## Near-Term Sequence

1. Stabilize canonical state and projection seams.
   - Keep `schemas/agent-state.schema.json` and required vectors coherent.
   - Keep canonical state separate from perceived overlays.
   - Keep voice, presentation, relationship, memory, and situational pressure explainable.
   - Add or preserve tonal-mode and ordinary-life cues where they affect prose:
     humor, ritual, domesticity, boredom, tenderness, wonder, professional
     routine, and slow dread are not decoration when they explain behavior.
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

4. Run the first full-story artifact pass.
   - Completed clean Cold Wake IF scaffold:
     `examples/ink/cold-wake-branch-and-fold.v0.ink`,
     `examples/ink/cold-wake-branch-and-fold.v0.training.json`,
     `examples/coordinator/cold-wake-branch-and-fold.v0.json`, and
     `experiments/cold-wake-story-lab/cold-wake-branch-and-fold-clean-run.v0.md`.
   - Treat that run as accepted-as-draft coordinator/IF scaffold training data,
     not raw responder gold.
   - Interactive-fiction correction: a full-story pass is not just a linear chain of responder receipts. It needs a beginning-to-ending narrative spine, branch points, and cross-scene consequence carryover so decisions visibly matter later. Responder artifacts should be generated at branch/action moments, while coordinator artifacts track branch flags, relationship deltas, resource costs, unresolved hooks, and later scene consequences.
   - Branch-and-fold correction: do not let every choice create an isolated subtree. Normalize consequences into compact state bands and flags, fold most branches back into shared later scenes, and reserve major route splits for consequences that change the playable world enough that convergence would lie.
   - Branch compiler correction: treat Ink materialization as its own organ. It receives fixture brief, lore digest, scene/world/relationship state, actor-local branch candidates, consequence packets, fold plan, and visual continuity requirements; it emits playable Ink plus a training sidecar and compiler notes. It does not own hidden character truth or social mutation.
   - Website-content correction: each complete scene should include an art-direction block with establishing shot, light/color, bodies/interfaces, material evidence, branch-visible marks, one durable base image prompt, and additive branch/state modification prompts. Visual continuity is part of consequence carryover, not decorative garnish applied after the story escapes containment.
   - Illustrated-IF correction: a narrative scene can contain multiple visual
     sections. The branch compiler should segment introductions, focal areas,
     object closeups, arrivals, alarms, pivotal one-sentence beats, standoffs,
     and aftermaths into click-through Ink sections with stable visual ids.
     Each visual section needs an imagegen-ready prompt that describes
     fictional equipment and body affordances in visible terms. Named
     recurring characters need stable visual character refs; names are handles,
     not faces. When a fixture targets a specific image language, store one
     global style cue and include it in every assembled image prompt. Include
     character refs only for characters actually visible in the rendered frame;
     keep continuity notes as tooling constraints unless rewritten as visible
     prompt text.
   - IF review correction: run an independent artifact reviewer after branch/Ink generation and before acceptance. The reviewer audits fake variables, cosmetic choices, missing state reads, fake folds, unearned convergence, state-named-instead-of-checked prose, visual callback gaps, and endings that ignore major state. The first Cold Wake user review is a seed failure taxonomy for this evaluator path.
   - Visual review correction: run a visual scene continuity reviewer for
     illustrated fixtures. It audits oversized Ink screens, missing visual
     scene anchors, lore-password image prompts, missing character visibility,
     weak stance control, and missing branch/state visual modifiers.
   - The older Cold Wake v0/v1 fragments remain diagnostic seam tests around the clinic climax, not the accepted clean fixture.

   - Still emit training artifacts at material turns: projected local context, responder packet, raw output/capture, leakage audit, reviewed mutation receipt, and aftermath state where relevant.
   - Do not stop for every minor prose flaw; curate as coordinator and continue. Stop to patch the pipeline only for core invariant failures: hidden-state leakage, lore grounding failure, visibility/object/body affordance errors, bad state mutation, or schema break.
   - Keep fuzzy social changes manual and auditable until appraiser/classifier models exist.

5. Generalize the loop.
   - Next work should move away from Cold Wake to a different faction, movement,
     flashpoint, or day-in-the-life fixture so the corpus does not become one
     very cold hallway with delusions of grandeur.
   - Add more historical grounded fixtures from AetheriaLore.
   - Add future-branch Elysium fixtures with branch lineage and constraint labels.
   - Track coverage across every major faction, minor faction, movement, and
     major flashpoint; major factions require both founding-era and
     day-in-the-life stories.
   - Require cultural collision in coverage stories so training data captures
     inter-faction dynamics, mutual misreads, and movement pressure instead of
     sealed faction portraits.
   - Vary tonal modes across fixtures so the corpus does not train one
     flattened Aetheria voice. Use Adams/Pratchett-style wit-with-stakes as a
     useful default touchstone, but allow controlled noir, horror, romance,
     domestic comedy, dour poetry, dry technical systems prose, wonder, and
     other scene-appropriate modes.
   - Emit technology/item manifest deltas when scenes discover or stress gear, assemblies, supply chains, or faction tech bases.
   - Keep artifacts database-shaped enough for future game-engine integration.

6. Train only after schemas stop sliding.
   - Pilot samples are schema shakedown, not robust corpus scale.
   - Specialized future models include coordinator, retriever, projector,
     responder, appraiser, mutator, relationship/perception updater, branch
     compiler, IF artifact reviewer, evaluator, and
     institution/faction/consumer decision models.
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
