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
   - Use `docs/architecture/ink-branching-scenes.md` as the branch-format
     contract. Ink owns playable branching scenes; Ghostlight owns local
     awareness, projection rationale, consequences, and reviewed state mutation.
   - Keep the product target narrow: procedural branching scene trees for games
     first. "Dialogue tree" means speech and non-speech actions, not only
     spoken lines.
   - Do not make the training data dialogue-only. Preserve labels and state
     hooks for open-world contexts, iterated decisions, consumer behavior,
     scarcity, reputation, faction pressure, and institution decision-making.
   - Build local awareness from scene state, agent state, relationships,
     memories, resources, affordances, and known constraints.
   - Consolidate participant appraisals and state mutation after every
     resolved action before selecting the next actor; hurt, threat,
     reassurance, obligation, overload, and misread should alter the state any
     affected participant acts from.
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
   - Emit game-useful artifacts: Ink scene source, compiled Ink JSON, scene
     transcript, candidate player choices, including non-speech actions, NPC response branches,
     state/memory/social-perception deltas, and unresolved hooks.
   - For protagonist scenes, generate several plausible things the protagonist
     could do from state and affordances, then generate NPC responses and branch
     consequences such as trust loss, suspicion, obligation, debt, resource
     cost, information exposure, or future scene hooks.
   - Keep out of implementation scope for now: world-scale simulation loops,
     economy simulation loops, city-scale scheduling, autonomous offscreen
     factions as full actors, and long-horizon plot invention without author
     scaffolding.
   - Still record why agents buy, refuse, trade, hoard, comply, defect, conceal,
     punish, or help when those decisions appear in scenes; those reviewed
     decisions are seed data for later consumer-behavior and faction models.
   - First prototype: express one Cold Wake story-lab beat as Ink, compile it,
     and validate a sidecar training annotation that maps branches to
     Ghostlight state basis, action intent, consequence surfaces, and manual
     mutation policy.
   - First generated draft: use `tools/run_qwen_ink_branch_generation.py` to
     ask Qwen for branch candidates from local awareness, then materialize the
     reviewed result with `tools/materialize_qwen_ink_draft.py`.
   - First sequential draft: use `tools/run_qwen_ink_sequential_generation.py`
     to separate actor-local choice, participant appraisal/consolidation, and
     the next actor's move from updated state. The first saved capture is
     useful-needs-revision because it exposed the missing per-turn appraisal
     boundary and weak action enum discipline.
   - First reviewed branch mutation: use
     `tools/apply_sequential_ink_branch_mutation.py` to apply one selected
     generated branch into
     `examples/agent-state.cold-wake-story-lab.after-sanctuary-ledger.json`
     and a mutation receipt, updating both Maer and Sella without treating Ink
     variables as canonical truth.
   - Second sequential draft: the symmetrical turn framing produced canonical
     action labels and a concrete Sella `withhold_object` next action, but
     still needs stricter schema or tool-call enforcement because array fields
     drifted into strings and objects.
   - Qwen invocation correction: use Ollama `/api/chat` with native tools and
     `think: true`, not `/api/generate` with prose-only JSON instructions.
     Qwen's XML-shaped tool format belongs to the chat-template/tool-call
     layer; Ghostlight artifacts do not need XML wrappers.
   - Third sequential draft: v3 proved the tool path could work for Maer while
     later passes still fell back to plain JSON because old prompt contract
     text conflicted with tool calling.
   - Fifth sequential draft: v5 is accepted as draft. It used thinking plus
     native tools for Maer choice, Sella appraisal, and Sella next action.
   - Next prototype: materialize the accepted v5 capture into Ink and sidecar
     annotation, then apply reviewed state mutation only after validation.

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
