# Illustrated IF Visual Pipeline

Illustrated interactive-fiction fixtures can use image generation for mood,
presentation, and website replay. They should not treat image generation as the
source of spatial truth.

## Role

The visual pipeline has four separate jobs:

1. Describe the playable visual beat in the Ink and `.visual.json` artifact.
2. Preserve the intended environment, character, and branch state as structured
   visual data.
3. Provide image models with enough concrete visible detail to render the beat.
4. Keep generated art useful as presentation collateral without letting it
   rewrite scene geometry.

Visual artifacts are auxiliary replay and website value. They are not core
training targets for responder, appraiser, mutator, relationship, or social
state models.

## Geometry Authority

For scenes where spatial continuity matters across angles, branches, or later
callbacks, the authoritative visual source is a `scene_set` artifact, not a
generated image.

A `scene_set` may be a hand-built 3D blockout, a procedural scene file, a
layout diagram, or another durable geometry asset. It should define:

- Environment layout and scale.
- Named zones, routes, entrances, exits, and occluders.
- Fixed cameras or camera-slot names for replay beats.
- Character staging slots and allowed movement regions.
- Important props, access points, interfaces, and branch-visible state anchors.
- Which parts of the set are visible from each camera.

Generated environment concept art can inspire the set, but it is not a reliable
layout contract. Image models routinely invent impossible joins, inconsistent
occlusion, false loops, and locally plausible machinery that cannot be staged
from a second angle.

Agent-assisted set construction may live in a separate toolchain such as
VibeGeometry. Ghostlight only requires the resulting durable geometry input and
the metadata needed to connect it to fixture scenes.

## Prompt Assembly

Prompt assembly should use the geometry source appropriate to the fixture:

- Mood concept only: text prompt may be enough.
- Single isolated illustration: a complete standalone prompt may be enough.
- Multi-scene illustrated IF: use `.visual.json` scene ids, character refs, and
  branch modifiers.
- Multi-angle or branch-consistent illustrated IF: use a `scene_set` render or
  blockout for each camera angle, then apply the scene prompt as painterly
  direction.

For the multi-angle path, the image model acts as a painter and material
interpreter. It does not decide the room shape.

## Base Images And Modifiers

Base prompts describe a complete visible frame. Modification prompts describe
only the branch or state delta, but the edit call must receive the base image or
the camera-specific blockout as visual input.

Do not rely on phrases such as "same room" or "as before" unless the edit input
actually supplies the same room. If the image model only receives text, restate
the visible geometry. If it receives a blockout, the prompt should still name
the visible anchors it must preserve.

## Reviewer Expectations

The visual reviewer should ask whether the current fixture needs a `scene_set`.
It should require one when:

- The story returns to the same room from multiple angles.
- Branches move characters around a shared environment.
- Sightlines, occlusion, route access, or body affordances affect choices.
- The same space needs website imagery across several beats.
- A generated concept image is being used as if it were a durable floor plan.

When a `scene_set` is required but missing, the fixture can still be accepted as
prose or IF scaffold, but not as replay-ready illustrated IF.
