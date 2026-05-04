# Documentation Inventory

Use this file as the navigation map for Ghostlight's docs. It classifies files
by job so architecture, planning, archived receipts, raw imports, research
backlog, and state ledgers do not slowly melt into one cursed binder.

## Canonical Re-Entry And Planning

- `state/map.yaml`: canonical project map and current status.
- `notes/fresh-workspace-handoff.md`: re-entry summary for fresh sessions.
- `notes/ghostlight-current-system-map.md`: current runtime and artifact map.
- `notes/ghostlight-implementation-plan.md`: live implementation plan and
  next-action surface.
- `notes/documentation-inventory.md`: this navigation/status index.

## Core Architecture Contracts

- `docs/architecture/canonical-agent-state-schema.md`: canonical state model.
- `docs/architecture/agent-state-variable-glossary.md`: meanings for state
  axes, voice style, overlays, and emergent behaviors.
- `docs/architecture/agent-state-distributions-and-prompt-projection.md`:
  deeper state-distribution and projection rationale.
- `docs/architecture/prompt-projection-contract.md`: projection boundary and
  renderer contract.
- `docs/architecture/projected-local-context.md`: character-local context
  shape.
- `docs/architecture/sequential-agent-runtime.md`: sequential action/reaction
  runtime architecture.
- `docs/architecture/sandboxed-responder-packets.md`: isolated responder packet
  contract and capture requirements.
- `docs/architecture/soft-model-training-artifacts.md`: rule for turning soft
  judgments into training artifacts.
- `docs/architecture/ink-branching-scenes.md`: branch-and-fold, branch
  compiler, visual prompt, and IF reviewer contract.
- `docs/architecture/illustrated-if-visual-pipeline.md`: visual replay
  contract, geometry authority, scene-set/blockout role, and imagegen boundary.
- `docs/architecture/lore-grounding-digest-format.md`: source-grounding digest
  format.
- `docs/architecture/aetheria-lore-grounding-architecture.md`: Aetheria
  historical, future-branch, and procedural grounding boundaries.
- `docs/architecture/technology-item-manifest-plan.md`: item, assembly,
  supply-chain, trade-good, and blueprint data target.
- `docs/architecture/training-plan.md`: trainable organs, datasets, gates, and
  rough scale targets.

## Supporting Architecture And Future Mechanisms

- `docs/architecture/aetheria-agent-population-model.md`: population and
  faction-scale agent modeling.
- `docs/architecture/aetheria-content-generation-targets.md`: Aetheria content
  targets and consumer use cases.
- `docs/architecture/aetheria-persistent-agent-architecture.md`: broader
  persistent-agent architecture.
- `docs/architecture/projection-distillation-plan.md`: student projector
  training concept.
- `docs/architecture/schema-future-mechanisms.md`: future schema organs waiting
  for more evidence.
- `docs/architecture/aetheria-cold-wake-training-fixture.md`: Cold Wake fixture
  notes and source-grounded room candidates.
- `notes/architecture-rationale.md`: rationale notes for major design
  boundaries.

## Archived Experiments And Plumbing

- `docs/experiments/cold-wake-story-lab.md`: archived local-model/projection
  experiment and failure lessons.
- `docs/architecture/qwen-invocation-notes.md`: archived local-model invocation
  plumbing notes.

## Raw Imported Source

- `docs/aetheria/call-of-the-void-brainstorm.md`: raw imported brainstorming
  document. Encoding scars are preserved; do not treat this as canonical prose.

## Research Backlog

- `docs/research/personality-model-roadmap.md`: personality-model research
  roadmap.
- `docs/research/personality-signal-classifier.md`: personality signal
  classifier research.
- `docs/research/things-to-steal.md`: research backlog and candidate ideas.

## State Ledgers

- `state/evidence.jsonl`: distilled current evidence ledger.
- `state/evidence.archive.jsonl`: archival evidence ledger.
- `state/branches.json`: branch ledger.
- `state/scratch.md`: disposable one-subgoal scratch surface.

## Maintenance Rule

Architecture docs describe durable contracts. Live planning belongs in
`notes/ghostlight-implementation-plan.md`, `notes/fresh-workspace-handoff.md`,
and `state/map.yaml`. Archived experiments can keep history, but they must be
labeled as archives when their path stops being the active route.
