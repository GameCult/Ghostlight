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
  -> updated fixture / future training corpus
```

## Important Contracts

- Agent state: `schemas/agent-state.schema.json`
- Projection examples: `schemas/projection-example.schema.json`
- Lore grounding: `schemas/lore-grounding-digest.schema.json`
- Projected local context: `schemas/projected-local-context.schema.json`
- Coordinator artifacts: `schemas/coordinator-artifact.schema.json`
- Responder packets: `schemas/responder-packet.schema.json`
- Responder outputs: `schemas/responder-output.schema.json`

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
- Parent sessions currently receive only the subagent final message, not a
  visible tool-call transcript. Treat subagent research as self-reported unless
  the worker returns explicit research notes or a future runner captures calls.

## Archived Receipts

Old local-model prototype files with `qwen` in their paths remain useful for
validation and lessons about structured-output failure, object custody,
embodiment, prompt leakage, and materialization. They are not active direction.
Do not let them steer gold responder data unless the task is explicitly model
plumbing.

## Missing Organs

- Deterministic speaker-local input slicer
- Full prompt renderer
- Classifier/appraiser pipeline
- Relationship/perception updater
- State mutator model
- Student projector
- Full scene/event loop implementation
- Culture prior engine
- Automatic promotion of branch outcomes into canonical state

## Current North Star

Generate high-quality, sandboxed, source-grounded responder/coordinator samples
that can later train specialized Ghostlight models while staying usable by a
game engine: exact inputs, exact outputs, provenance, review labels, state
mutation receipts, and clear branch consequences.
