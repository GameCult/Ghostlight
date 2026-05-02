# Projected Local Context

The projected local context is the missing seam between canonical Ghostlight
state and a character agent.

Canonical state is storage. It can contain means, plasticity, activation,
relationship stance, memories, culture, goals, and author-facing source paths.
A character response model should not have to interpret that machinery. It
should receive a compact operating context: what this character knows, wants,
misreads, can do, sounds like, and must not resolve.

## Contract

Pipeline:

```text
canonical state + scene + reviewed annotation
  -> projector/state interpreter
  -> ghostlight.projected_local_context.v0
  -> response-model prompt
  -> generated action proposal
```

The projected context may retain source references for audit. The rendered
prompt text must not expose raw state internals such as `current_activation`,
`plasticity`, means, or selected numeric dimensions.

## Artifact Shape

Schema: `schemas/projected-local-context.schema.json`

Validator: `tools/validate_projected_contexts.py`

Generator: `tools/project_local_context.py`

Current examples:

- `examples/projected-contexts/scene-02-sanctuary-intake.maer_tidecall.projected-context.json`
- `examples/projected-contexts/scene-02-sanctuary-intake.sella_ren.projected-context.json`

The context contains:

- speaker identity and setting
- known facts and speaker beliefs
- current stakes
- active inner pressures rendered as ordinary language
- relationship read and known biases
- tensions
- action affordances
- projection controls
- voice surface
- likely response moves
- do-not-invent boundaries
- rendered prompt text

## Runtime Boundary

The character model receives local operating context, not omniscient truth. If
Maer acts, Sella's next pass receives only the observable event and Sella's own
local context. Maer's private intent is not included.

The sequential runtime still owns:

- tool contracts
- action enums
- event proposal capture
- participant appraisal
- reviewed mutation
- Ink materialization

The projector owns the translation from Ghostlight's internal state language to
character-local operating prose.

## Current Limit

The first projector is deterministic and deliberately simple. It uses threshold
language like "dominant", "strong", "present", and "background" rather than
numeric scores. This is enough to remove raw state soup from Qwen prompts and
create inspectable training artifacts.

Later versions can become smarter, but they should still preserve this boundary:
response models act from projected local context; they do not become ad hoc
interpreters of the canonical schema.
