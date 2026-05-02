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
   - Projector gap: the sequential runner still sends selected raw numeric
     state variables into prompts. This is acceptable scaffolding for Qwen
     plumbing tests, not the final Ghostlight architecture.
   - Projector prototype: `tools/project_local_context.py` now turns canonical
     variables, memory, relationship stance, culture, and scene pressure into
     compact character-local operating context and action affordances.
   - Projected-context artifacts use
     `ghostlight.projected_local_context.v0` and validate through
     `tools/validate_projected_contexts.py`; rendered prompt text must not leak
     raw state internals such as `current_activation`, `plasticity`, means, or
     decimal state values.
   - Embodiment correction: projected local context now includes a first-class
     embodiment/interface section. Aetheria faction affordances must be checked
     against lore before projection. Current Navigator source supports fluid
     architecture, acoustic signaling, soft navigation light, layered water
     motifs, communal orientation chambers, mixed-species
     translation/coexistence, specialized cetacean environments, and, after a
     source patch, mixed wet/dry habitat and interface ergonomics.
   - Aetheria generation doctrine: before writing scenes or generating training
     data, check AetheriaLore for factions, movements, institutions, species or
     body types, location, and time period. If a blocking room-scale detail is
     missing, patch AetheriaLore narrowly before treating that detail as
     available to projection.
   - Sixth sequential draft: v6 routed thinking-plus-tools generation through
     projected local context instead of selected activation dictionaries. It
     produced usable Maer/Sella behavior but remains useful-needs-revision
     because Qwen stringified nested `response_constraints` in the appraisal
     payload.
   - Runner tightening: failed Maer choice generation now writes reviewed
     receipts instead of exiting, obvious malformed stringified tool arrays can
     be repaired, repair notes are separated from fatal validation notes, and
     Sella's next-action prompt receives appraisal prose instead of raw numeric
     deltas.
   - Eighth and ninth sequential drafts: v8 captured malformed stringified
     `choices`; v9 captured a no-tool-call dropout. Both validate the receipt
     path and show Qwen tool reliability is now the blocker.
   - Thinking policy correction: v9's thinking trace showed minutes of schema
     self-check looping followed by no tool call. Strict sequential generation
     now defaults to `think: false`; `--think` is reserved for diagnostics.
   - Tenth sequential draft: v10 ran projector-routed generation with thinking
     disabled and validated as accepted-as-draft with no repair or failure
     notes.
   - V10 materialization: `tools/materialize_sequential_capture.py`
     materialized the accepted projector-routed capture into Ink, sidecar
     annotation, reviewed mutation receipt, and a mutated Cold Wake fixture.
   - Next prototype: run another Qwen pass through the source-checked Navigator
     projector, review whether the patched AetheriaLore ergonomics produce
     cleaner action proposals, then decide whether to polish the branch or
     generalize the materializer for additional accepted captures.
   - Source-checked retry results: v11-v17 improved Maer's body/interface
     behavior substantially, producing packet actions through shared clinic
     displays and mixed wet/dry affordances instead of humanoid action. No
     capture was materialized because later passes exposed repair notes,
     object-custody drift, invented mutation paths, one Qwen appraisal
     tool-call dropout, and prompt-constraint leakage into Sella prose.
   - Validation correction: the sequential runner now checks Maer embodiment,
     Cold Wake object semantics, Sella appraisal paths/relationship ids, and
     prompt leakage in responder prose. The next implementation move should
     make responder prose less likely to copy constraints verbatim before
     trying to materialize another capture.

5. Build the projection distillation loop. Started.
   - Use `docs/architecture/projection-distillation-plan.md` as the teacher to
     student roadmap.
   - Use `docs/architecture/soft-model-training-artifacts.md` as the boundary
     between deterministic code and soft model-training targets.
   - Use `docs/architecture/training-plan.md` as the concrete stage inventory:
     lore grounding compiler, coordinator/story runtime, memory/lore retriever,
     projector, character agent/responder, event resolver, participant
     appraiser, state mutator, relationship/perception updater, evaluator
     classifiers, and institution/faction/consumer decision models.
   - Use `docs/architecture/aetheria-lore-grounding-architecture.md` to keep
     Aetheria grounded training separate from procedural Elysium branch
     generation.
   - Maintain `schemas/projection-example.schema.json`,
     `examples/projections/`, and `tools/validate_projection_examples.py` as
     the first reviewed artifact seam.
   - Use authored historical Aetheria flashpoints, not malleable Elysium
     gameplay outcomes, as the first grounded lore-training corpus.
   - Also generate reviewed post-Rupture Elysium `future_branch` fixtures so
     models learn Aetheria's weird future concepts under supervision rather
     than treating late-Sol history as the whole training universe.
   - Future branches must label source-backed post-Elysium concepts separately
     from generated branch assumptions, branch lineage, sibling-branch
     exclusions, and branch-local canonical facts.
   - Future branches must also label constrained events: pressure-born
     `branch_attractor`, authorial `fated_event`, discovery-gated
     `tech_order_constraint`, authored `quest_injection`, or ordinary
     `branch_local_event`.
   - Elysium-born factions and institutions, including things like Adrasteia or
     Miss Terri's Sugarrific Snack Company, should be modeled through those
     labels rather than hidden in prose: they may emerge from branch pressure,
     be nudged by fate, arrive through quest injection, or remain local to one
     branch.
   - Use `docs/architecture/technology-item-manifest-plan.md` to make future
     exploration produce game-usable technology data: item families, variants,
     assemblies, subassemblies, components, materials, faction access,
     prerequisites, bottlenecks, compatibility rules, upgrade slots, economic
     consequences, and quest hooks.
   - Pre-Elysium starting tech must be manifested too. Factions do not begin
     with identical tech bases, manufacturing rights, stockpiles, maintenance
     skills, standards, or supply-chain dependencies.
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
   - Treat the character agent/responder as a training target too. A
     Qwen-class or similar local model can eventually be adapted on a corpus of
     strong Aetheria responses spread throughout the timeline, plus reviewed
     Ghostlight receipts, so lore, tone, factional preconceptions, species
     affordances, and institutional pressure become native responder priors
     rather than prompt handholding.
   - Include future-branch post-Rupture responses before expecting the responder
     to handle Elysium concepts such as Aether, pseudospace, temporal
     nonlinearity, spirits, necrotech, mutable bodies, and altered substrates
     without heavy prompt scaffolding.
   - Keep deterministic gates as code even after fine-tuning: visibility,
     action legality, object custody, resource accounting, schema validation,
     source provenance, mutation authority, prompt leakage checks, and Ink
     compilation.
   - Emit reviewed artifacts for every fuzzy organ before training specialized
     models: coordinator/story runtime, projector, character agent/responder,
     participant appraiser, state mutator, relationship/perception classifier,
     Aetheria responder, and later institution/faction/consumer decision
     models.

6. Build the coordinator artifact seam. New target.
   - Treat the current authoring agent as the temporary coordinator: it glues
     scenes together, decides which machinery runs, maintains continuity, emits
     connective prose, and proposes world-state changes for deterministic gates
     and reviewed mutation.
   - Keep coordinator state game-engine-shaped even before a game engine exists:
     scene ids, location ids, participants, active objects, resources, public
     facts, unresolved facts, branch flags, event refs, fixture refs, and
     consequences awaiting review.
   - Save coordinator decisions as training artifacts: scene setup, next-beat
     choice, acting-agent choice, tool/model invocation plan, glue prose,
     branch merge or split decisions, world-state refs read, proposed state
     changes, carried hooks, rejected prose, and reviewer notes.
   - Do not let glue prose become canonical truth. It is narrative connective
     tissue and training signal; structured state and reviewed mutations decide
     what happened.
   - The long-term coordinator model should learn how to stage scenes, preserve
     continuity, call specialist organs, and emit readable connective narration
     while respecting deterministic gates and game-engine constraints.
   - First concrete training-plan target: define the coordinator artifact schema
     before generating another long story pass, so glue prose, next-beat choice,
     world-state refs, branch flags, unresolved hooks, and machinery invocations
     are captured as trainable data instead of living only in chat.
   - Coordinator artifacts should be able to attach item-manifest deltas when a
     scene or branch decision discovers, alters, upgrades, restricts, salvages,
     counterfeits, or economically stresses a technology or item.

7. Build the first drama-scaffolding loop.
   - memory updates
   - relationship updates
   - goal pressure
   - situational activation
   - cultural and institutional pressure
   - conflict beats derived from incompatible goals and values

8. Build the first Aetheria authoring consumer.
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
