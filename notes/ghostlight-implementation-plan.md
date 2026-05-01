# Ghostlight Implementation Plan

## Near-Term Sequence

1. Freeze the first canonical state model. In progress.
   - Define the latent variable families.
   - Define first-class dimensions, including volatility, attachment-seeking,
     and distance-seeking.
   - Define canonical state versus perceived state.
   - Keep authoring outputs explainable from the state, not just generated from
     stylish prompt fog.
   - Maintain `schemas/agent-state.schema.json`,
     `schemas/agent-state.required-fields.json`, and the example fixtures as
     the first executable contract.

2. Freeze the first event and classifier schema.
   - Define event labels.
   - Define canonical expression labels.
   - Define listener perception labels.
   - Define mask or presentation labels.
   - Define confidence, uncertainty, and evidence accumulation rules.

3. Build the prompt projection layer for authoring.
   - Use `docs/architecture/prompt-projection-contract.md` as the first
     renderer contract.
   - Use `docs/architecture/schema-future-mechanisms.md` to keep appraisal,
     transition, belief, action, conversation, resource, dyad, institution,
     memory, and embodiment needs visible without prematurely bloating the v0
     agent-state schema.
   - Render scene and dialogue scaffolds from structured state, relevant
     memories, scene pressure, cultural defaults, and perceived state.
   - Keep the prompt renderer a projection surface instead of a second brain.
   - Make generated moves inspectable: why this character deflects, confesses,
     threatens, softens, bargains, or lies.
   - Treat this as a debug/training seam for the larger sequential runtime, not
     as the final product shape.

4. Build the scene-local sequential dialogue loop. New target.
   - Use `docs/architecture/sequential-agent-runtime.md` as the first runtime
     contract.
   - Keep the product target narrow: procedural interactive dialogue trees for
     games first, then scene-local character action with consequence
     propagation, then longer procedural storyline generation later.
   - Build local awareness from scene state, agent state, relationships,
     memories, resources, affordances, and known constraints.
   - Keep action primitives concrete: speech, silence, movement, gesture,
     object use, object transfer, object blocking, resource spending, waiting,
     and attack. Treat ask/refuse/offer/reveal/conceal/request as intended
     communicative functions, not separate reality-mutating tools.
   - Preserve the gap between actor intent and listener interpretation; a
     listener is free to misread what the actor thought they were doing.
   - Treat generated action as an event proposal, not direct truth mutation.
   - Resolve the event against world state and mutation authority.
   - Automate only mechanical legality, visibility, world/resource/object
     deltas, and event append in the first prototype.
   - Keep fuzzy updates manual at first: appraisals, relationship movement,
     belief changes, memory writes, intent classification, misreads, and
     activation deltas. Save those reviewed decisions as future classifier and
     appraiser training data.
   - Keep author control at stage-setting and high-level constraints; do not
     require the author to script every beat.
   - Emit game-useful artifacts: scene transcript, candidate player choices,
     NPC response branches, state/memory/social-perception deltas, and
     unresolved hooks.
   - Keep out of scope for now: world-scale simulation, economy, city-scale
     scheduling, autonomous offscreen factions, and long-horizon plot invention
     without author scaffolding.
   - First prototype: rerun one Cold Wake story-lab beat through a deterministic
     local-awareness/projection-control renderer, then apply one conservative
     state mutation and validate the resulting fixture.

5. Build the projection distillation loop. Started.
   - Use `docs/architecture/projection-distillation-plan.md` as the teacher to
     student roadmap.
   - Use `docs/architecture/aetheria-lore-grounding-architecture.md` to keep
     Aetheria grounded training separate from procedural Elysium branch
     generation.
   - Maintain `schemas/projection-example.schema.json`,
     `examples/projections/`, and `tools/validate_projection_examples.py` as
     the first reviewed artifact seam.
   - Use authored historical Aetheria flashpoints, not malleable Elysium
     gameplay outcomes, as the first grounded lore-training corpus.
   - Maintain `schemas/lore-grounding-digest.schema.json`,
     `examples/lore-grounding/`, and `tools/validate_lore_grounding.py` as the
     cultural and factional pressure seam for grounded fixtures.
   - Use `examples/lore-grounding/cold-wake-panic.ganymede-corridor.json` as
     the first draft historical grounding digest.
   - Build the first grounded projection fixture from one specific Cold Wake
     room rather than from the whole crisis at once.
   - Use `docs/experiments/cold-wake-story-lab.md` and
     `examples/agent-state.cold-wake-story-lab.json` as the first story-shaped
     writing lab.
   - Treat projection outputs as response turns, not dialogue-only turns:
     characters may speak, act, withdraw, stay silent, use violence, or combine
     speech and action when state and scene pressure justify it.
   - Keep failed and accepted Qwen captures together. The Maer/Veyr v1 and v2
     outputs are useful failed examples; v3 is the first accepted output that
     preserves unresolved claimant status while allowing controlled physical
     action.
   - Treat repeated generation failures as projection architecture signals, not
     only prompt wording problems. Projection examples now include controls for
     frame ownership, authority boundaries, object custody, required semantics,
     and forbidden resolutions.
   - Keep Qwen captures as reviewed receipts. `tools/validate_qwen_captures.py`
     is part of `npm run schema:validate` so unreviewed captures do not quietly
     become training data.
   - Use `experiments/cold-wake-story-lab/the-narrowest-possible-margin.md` as
     the first end-to-end story artifact assembled from accepted response
     captures plus authored connective narration.
   - Use a frontier teacher model to produce reviewed projection artifacts from
     structured state and lore grounding digests.
   - Save accepted and rejected projection examples as supervised data.
   - Train or adapt a smaller student projector only after the artifact schema,
     input slicer, and evaluator are stable.

6. Build the first drama-scaffolding loop.
   - memory updates
   - relationship updates
   - goal pressure
   - situational activation
   - cultural and institutional pressure
   - conflict beats derived from incompatible goals and values

7. Build the first Aetheria authoring consumer.
   - use the Call of the Void slice described in `AetheriaLore` as the first
     concrete content-generation context
   - treat Call of the Void and other Elysium gameplay-era fixtures as
     procedural branch material unless specific outcomes are explicitly
     promoted
   - support dialogue scaffolding and procedural drama for narrated stories
     across the Aetheria timeline
   - do not treat Cold Wake as the implementation target unless explicitly
     revived later; it is currently training feedstock

## Discipline

- Do not start with an MMO and call it ambition.
- Do not train a classifier before the label schema is stable enough to deserve
  the trouble.
- Do not let prompt-writing outrun the structured state model.
- Do not fine-tune a projector before the projection artifact schema, input
  slicer, and evaluator have stopped sliding around.
- Do not let elegant theory notes multiply faster than implementation seams.
- Do not accidentally rebuild a game slice when the requested consumer is an
  authoring and story-generation organ.
