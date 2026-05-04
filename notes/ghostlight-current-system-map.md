# Ghostlight Current System Map

Ghostlight still does not have a full runtime. The live system is a set of
validated artifact seams for producing reviewed training data.

## Control Flow

1. Rehydrate from `state/map.yaml`, `notes/fresh-workspace-handoff.md`, this
   file, and `notes/ghostlight-implementation-plan.md`.
2. Run `npm run state:status`.
3. Work one bounded organ.
4. Validate the seam that matters.
5. Persist only belief-changing evidence.
6. Commit and push completed work.

## Core Surfaces

- `state/map.yaml`: canonical mission, boundaries, live architecture, next action.
- `state/evidence.jsonl`: distilled belief-changing evidence only.
- `state/evidence.archive.jsonl`: older evidence preserved for archaeology.
- `notes/fresh-workspace-handoff.md`: compact re-entry packet.
- `notes/ghostlight-implementation-plan.md`: near-term implementation sequence.
- `docs/architecture/`: detailed contracts and rationale.

## Active Artifact Pipeline

```text
Aetheria lore/source context
  -> lore grounding digest / source notes
  -> canonical agent and scene state
  -> projected local context
  -> responder packet
  -> sandboxed responder output
  -> review and leakage audit
  -> mutation receipt
  -> updated scene/world/social state
  -> coordinator continuity and next-beat plan
  -> branch compiler materializes Ink + sidecar + visual plan + compiler notes
  -> IF artifact reviewer audits consequence, fold, and visual continuity
  -> visual scene reviewer audits click-through segmentation and imagegen-ready prompts
  -> accepted fixture / future training corpus + optional illustrated replay collateral
```

Responder packets are one seam, not the whole project. The current branching
scene path also requires coordinator receipts, branch compiler artifacts, and an
independent IF review pass before a fixture is accepted.

Tone is also a live seam. The system should preserve Aetheria's range instead
of defaulting to dry technical crisis prose: comic warmth, domestic routine,
ritual memory, weird bureaucracy, noir suspicion, wonder, horror, poetic dread,
and procedural systems pressure are all valid when source-grounded and
character-local. Adams/Pratchett-style wit-with-stakes is a useful default
touchstone for counterbalancing bleakness without dissolving consequence.

Visual replay is a presentation seam, not a core social-model training seam.
Illustrated IF fixtures need click-through sections with stable
`visual_scene_id` anchors, stable visual character refs for recurring named
characters, a global style cue when the visual language matters, imagegen-ready
base prompts, character visibility and stance controls, and branch/state
modifiers. This data belongs in a separate `.visual.json` artifact referenced
by the training sidecar, not inside the training annotation itself. Prompt
assembly includes only refs for characters actually visible in the frame, and
continuity notes are not prompt text unless rewritten as concrete visible
description. One whole-fixture image prompt is not enough when the prose moves
through several places, focal objects, alarms, standoffs, and aftermath states.
Names are handles, not faces. The visual reviewer judges the playable Ink
experience and visual plan; clean-run prose receipts are debugging mirrors, not
the reviewed visual surface.

Generated images are not geometry authority. They can establish visual mood,
materials, and concept direction, but multi-angle illustrated IF needs a durable
`scene_set` source such as a hand-built 3D blockout, procedural scene file, or
layout asset with named cameras and staging slots. For those scenes, imagegen is
the painter/render pass over a camera-specific blockout, not the architect of
the room.

## Important Contracts

- Agent state: `schemas/agent-state.schema.json`
- Projection examples: `schemas/projection-example.schema.json`
- Lore grounding: `schemas/lore-grounding-digest.schema.json`
- Projected local context: `schemas/projected-local-context.schema.json`
- Coordinator artifacts: `schemas/coordinator-artifact.schema.json`
- Responder packets: `schemas/responder-packet.schema.json`
- Responder outputs: `schemas/responder-output.schema.json`
- Ink branch contract: `docs/architecture/ink-branching-scenes.md`
- Illustrated IF visual pipeline:
  `docs/architecture/illustrated-if-visual-pipeline.md`
- Training stages and corpus gates: `docs/architecture/training-plan.md`

## Current Live Example

- Research-enabled packet:
  `examples/responder-packets/scene-02-sanctuary-intake.sella_ren.packet.research.v0.json`
- It uses `responder_scoped_repository_search`, now treats `allowed_scope` as
  seed scope, permits bounded traversal through relevant AetheriaLore links,
  and requires `consulted_refs`, `followed_refs`, `research_trace`, plus
  `research_summary`.
- First research-enabled capture:
  `experiments/responder-packets/cold-wake-sanctuary-intake-sella-research-enabled-v0.capture.json`
- First research-enabled mutation receipt:
  `experiments/responder-packets/cold-wake-sanctuary-intake-sella-research-enabled-v0.mutation.json`
- First research-enabled aftermath state:
  `examples/agent-state.cold-wake-story-lab.after-sella-research-conditions.json`
- The capture is useful draft data. Its research trace is
  `coordinator_reconstructed`, not `runner_captured`, so it audits the lore
  constraints but does not prove the responder's actual tool path.
- Future accepted research-enabled gold captures must use `runner_captured`
  research trace status.
- First accepted runner-traced coordinator-scoped sample:
  `experiments/responder-packets/cold-wake-callisto-provenance-veyr-trace-v0.capture.json`
- It starts from `examples/agent-state.cold-wake-story-lab.after-sella-research-conditions.json`,
  projects Veyr for `scene-03-callisto-provenance`, embeds coordinator-retrieved
  lore as packet-visible source excerpts, runs a no-fork packet-only responder,
  and mutates state into
  `examples/agent-state.cold-wake-story-lab.after-veyr-category-offer.json`.
- Second accepted runner-traced coordinator-scoped sample:
  `experiments/responder-packets/cold-wake-convoy-threshold-isdra-trace-v0.capture.json`
- It starts from `examples/agent-state.cold-wake-story-lab.after-veyr-category-offer.json`,
  projects Isdra for `scene-04-convoy-threshold`, embeds coordinator-retrieved
  lore on Cold Wake, PSC category law, Ganymede rescue ledgers, heat debt, and
  Navigator mixed-interface spaces, then mutates state into
  `examples/agent-state.cold-wake-story-lab.after-isdra-limited-contact.json`.
- Third accepted runner-traced coordinator-scoped sample:
  `experiments/responder-packets/cold-wake-sanctuary-authorized-contact-sella-trace-v0.capture.json`
- It starts from `examples/agent-state.cold-wake-story-lab.after-isdra-limited-contact.json`,
  projects Sella for `scene-05-sanctuary-authorized-contact`, embeds
  coordinator-retrieved lore on Aya care capacity, Ganymede rescue ledgers,
  Navigator mixed-interface clinics, Cold Wake category failure, and heat debt,
  then mutates state into
  `examples/agent-state.cold-wake-story-lab.after-sella-bay-opened.json`.
- Parent sessions currently receive only the subagent final message, not a
  visible tool-call transcript. Treat subagent research as self-reported unless
  the worker returns explicit research notes or a future runner captures calls.
- Clean accepted-as-draft IF scaffold:
  `examples/ink/cold-wake-branch-and-fold.v0.ink`
- Its sidecar is
  `examples/ink/cold-wake-branch-and-fold.v0.training.json`.
- Its coordinator artifact is
  `examples/coordinator/cold-wake-branch-and-fold.v0.json`.
- Its readable run is
  `experiments/cold-wake-story-lab/cold-wake-branch-and-fold-clean-run.v0.md`.
- This fixture is coordinator/IF scaffold data. It is not raw responder gold.
- The user review that exposed fake variables and cosmetic state is now seed
  taxonomy for the IF artifact reviewer.

## Archived Receipts

Old local-model prototype files with `qwen` in their paths remain useful for
validation and lessons about structured-output failure, object custody,
embodiment, prompt leakage, and materialization. They are not active direction.
Do not let them steer gold responder data unless the task is explicitly model
plumbing.

## Missing Or Incomplete Organs

- Deterministic speaker-local input slicer
- Full prompt renderer
- Classifier/appraiser pipeline
- Relationship/perception updater
- State mutator model
- Student projector
- Branch compiler implementation beyond coordinator-authored fixtures
- IF artifact reviewer implementation beyond manual/frontier review
- Visual scene continuity reviewer implementation beyond prompted specialist review
- Full scene/event loop implementation
- Culture prior engine
- Automatic promotion of branch outcomes into canonical state

## Current North Star

Generate high-quality, sandboxed, source-grounded branching-scene samples that
can later train specialized Ghostlight models while staying usable by a game
engine: exact inputs, exact outputs, provenance, review labels, state mutation
receipts, branch compiler artifacts, IF review findings, visual scene plans,
and clear branch consequences. The samples should also train tonal control: a
fixture needs an intentional prose mode and enough ordinary life for consequence
to matter.
