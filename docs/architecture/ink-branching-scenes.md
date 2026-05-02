# Ink Branching Scenes

Ink is the branching scene format.

Ghostlight should not invent a parallel dialogue-tree language unless Ink
fails a concrete requirement. Ink already gives us readable interactive fiction,
choices, knots, stitches, variables, compiled JSON, and a format writers
can inspect without needing to wear a compiler helmet indoors.

Ghostlight's job is the machinery around Ink:

- build actor-local awareness from agent state and scene state
- project plausible speech and non-speech choices
- save generation receipts when a model proposes branches
- annotate why those choices make sense
- preserve listener interpretation as separate from actor intent
- record consequence surfaces for later state mutation
- keep fuzzy social updates reviewed until there is training data
- compile Ink into game-consumable JSON

Ink owns the playable branch structure. Ghostlight owns the psychological,
cultural, memory, relationship, and consequence rationale.

## Runtime Shape

The near-term authoring path is:

```text
agent state + scene state + lore grounding
  -> local awareness
  -> projected branch choices and NPC responses
  -> Ink scene
  -> compiled Ink JSON
  -> reviewed training annotations
  -> selected branch consequences
  -> state, memory, belief, and relationship updates
```

The Ink file should be readable as Aetheria content on its own. The sidecar
training annotation should explain which Ghostlight state produced each branch
and which consequences are automatic, reviewed, or deferred.

The first generated draft path is:

```text
examples/agent-state.cold-wake-story-lab.json
  + examples/ink/cold-wake-sanctuary-intake.training.json
  -> tools/run_qwen_ink_branch_generation.py
  -> experiments/ink/cold-wake-sanctuary-intake-qwen-branch-candidates-v1.capture.json
  -> tools/materialize_qwen_ink_draft.py
  -> examples/ink/cold-wake-sanctuary-intake.qwen-draft.ink
  -> examples/ink/cold-wake-sanctuary-intake.qwen-draft.training.json
```

The Qwen draft is accepted as training material, not as final polished prose.
That distinction matters. A generated branch can be useful because it exposes a
decision shape, an action type, a misread, or a prompt failure even when the
line writing still needs a human with a knife.

## Ink Comments

Use Ink comments for lightweight traceability in writer-facing drafts:

- `ghostlight.scene`
- `ghostlight.fixture`
- `ghostlight.branch`
- `ghostlight.action`
- `ghostlight.intent`
- `ghostlight.npc_response`
- `ghostlight.consequence`
- `ghostlight.training_hook`
- `aetheria.flashpoint`

Use `//`, not Ink tags (`#`), for this metadata in drafts. Inky exposes tags in
preview, which makes Ghostlight state leak into the readable scene. The
sidecar remains the machine-readable receipt; comments are only local handles
for humans and simple tools.

## Sidecar Annotations

Each Ink scene should have a `.training.json` sidecar with:

- source fixture reference
- protagonist and NPC ids
- actor-local awareness summary
- projection controls
- branch ids and Ink knot paths
- action type and actor intent
- state basis
- reviewed consequences
- training hooks
- mutation policy

This sidecar is intentionally not a new branching format. It is a receipt. Ink
remains the scene tree; the sidecar says why the tree grew that way and which
state changes a reviewer accepted.

## Mutation Policy

Ink variables can record local playthrough outcomes. They should not silently
become canonical Ghostlight state.

Automatic:

- Ink compilation
- branch id and knot validation
- local variables for the current playthrough
- mechanical comment handles and training hook checks

Manual reviewed until trained:

- relationship deltas
- belief updates
- memory writes
- listener misread classification
- activation changes
- promotion from branch outcome into durable Aetheria lore

The first prototype lives in
`examples/ink/cold-wake-sanctuary-intake.ink`.

The first Qwen-generated draft lives in
`examples/ink/cold-wake-sanctuary-intake.qwen-draft.ink`.
