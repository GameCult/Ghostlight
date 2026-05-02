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
  -> projected branch choices for the acting character
  -> selected branch as observable action
  -> event resolution from selected observable action
  -> participant appraisals and reviewed state mutation
  -> next actor local awareness from updated state
  -> next actor action
  -> Ink scene
  -> compiled Ink JSON
  -> reviewed training annotations
  -> selected branch consequences
  -> state, memory, belief, and relationship updates
```

The Ink file should be readable as Aetheria content on its own. The sidecar
training annotation should explain which Ghostlight state produced each branch
and which consequences are automatic, reviewed, or deferred.

Each character turn must be generated from that character's local awareness.
The protagonist-choice generator should not receive the responder's private
state. The responder generator should not receive the protagonist's private
intent except as an observable action, spoken text, gesture, object use,
silence, or other perceivable cue. A listener is allowed to misunderstand.

Participant appraisal is symmetrical. If an action hurts, threatens, reassures,
humiliates, obligates, or overloads anyone present, that change is consolidated
for the affected character before the next actor is selected. The next actor may
reply, walk away, comply, escalate, or end the interaction. A branch artifact can
still display this as `choice -> reaction`, but Ghostlight should model it as
turns over updated state, not as a mandatory paired response.

One-shot generation of both sides is allowed only as bootstrap scaffolding, and
must be marked as such in the capture review. It is not the target runtime
shape.

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

The Qwen draft is accepted as training material, not as final polished prose or
runtime architecture. It generated Maer's branch options and Sella's responses
in one pass, which is a known shortcut. A generated branch can still be useful
because it exposes a decision shape, an action type, a misread, or a prompt
failure even when the line writing still needs a human with a knife.

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

## Cross-Scene Consequence Carryover

Ghostlight branching scenes should not behave like disconnected dialogue samples.
Interactive fiction needs consequences that survive the scene boundary. A branch
choice should write durable effects into later context: relationship stance,
perceived motives, resource costs, authority flags, object custody, active
constraints, unresolved hooks, and available future options.

For training data, every material branch should preserve:

- the player/NPC-visible choice or action
- the immediate responder output
- accepted world-state, resource, memory, and relationship deltas
- branch flags that later scenes must read
- deferred consequences that should surface later rather than immediately
- scene text showing how the consequence changes the feel of the next beat

The coordinator owns this continuity for now. Later, the trainable coordinator
model should learn to carry these consequences forward without flattening them
into cosmetic callbacks. If a choice does not change later affordances, trust,
resources, risk, or interpretation, it is probably not a meaningful branch.

Cold Wake lesson: the clinic contact sequence is a useful climax seam, but it is
not a full story by itself. A complete interactive fixture needs an opening
pressure, at least one consequential branch before the clinic, and a resolution
that visibly reflects earlier choices.

## Scene Imagery

Ghostlight scenes are also website content. A complete fixture should emit
vivid visual direction alongside prose and training receipts so story branches
can be illustrated without a later model guessing what the room looked like.

Each scene should preserve an art-direction block with:

- establishing shot: location, scale, camera distance, and dominant geometry
- light and color: practical light sources, palette, exposure, haze, glare,
  reflection, water, vacuum, screen glow, or other atmosphere
- bodies and interfaces: how each relevant body occupies the space, including
  nonhuman access, assistive infrastructure, suits, avatars, wet/dry boundaries,
  displays, tools, and machinery
- material evidence: objects, damage, records, ledgers, cargo, alarms, clothing,
  stains, frost, heat shimmer, or other concrete traces of what has happened
- branch marks: visible changes caused by earlier choices, such as a sealed
  hatch, an exhausted staffer, a live debt ledger, missing equipment, guarded
  posture, or a route marker left blinking
- image prompt seed: a compact prompt suitable for an illustrator or image
  model, separated from canonical facts so style polish does not become lore

The coordinator owns this surface for now. The responder should not receive
omniscient visual art direction unless the character can observe it. Character
packets may receive the local visible subset: what this actor can see, hear,
touch, smell, monitor, or infer from their interface.

Visual continuity is part of consequence carryover. If an earlier branch cost
heat, trust, time, equipment, or social standing, the later scene should show
that cost somewhere on the page or in the frame. Otherwise the choice may be
mechanically recorded while still feeling fake, which is the stupid little
trapdoor interactive fiction keeps setting for itself.
