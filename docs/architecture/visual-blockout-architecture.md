# Visual Blockout Architecture

Ghostlight illustrated IF needs more than prompt continuity. A prompt can suggest
that two images share a room; it cannot remember the room from another camera.
For multi-angle illustrated replay, the durable artifact is a small spatial
stage set.

## Purpose

A visual blockout plan defines a low-poly Blender scene used as a spatial and
camera reference for image generation. It is not a semantic segmentation mask,
and its flat colors are not a reliable label language for an image model. The
blockout is a maquette: room shape, large object placement, character blocking,
scale, and camera angle.

The image model still receives prose prompts for style, character identity,
lighting, mood, and narrative emphasis. The blockout guide image supplies the
floor plan the model does not actually know.

## Artifact Shape

A complete illustrated fixture may now have three separate visual surfaces:

- `.ink`: playable story structure
- `.training.json`: branch/consequence training sidecar
- `.visual.json`: image prompts, style cue, character refs, visible modifiers
- `.blockout.json`: Blender-backed spatial guide plan for multi-angle replay

The blockout plan stores:

- `layout`: coordinate system, output `.blend`, scale notes, source refs
- `semantic_materials`: readable blockout materials and their intended visual read
- `geometry_modules`: procedural room organs such as shell, tunnel, manifold,
  catwalk, rail, and body-affordance stations
- `proxy_actors`: mannequin or creature proxies with positions and scales
- `cameras`: named shot cameras tied to `visual_scene_id` values
- `guide_renders`: output guide images per camera/visual scene
- `prompt_use_contract`: what the image model should preserve or reinterpret

Guide renders may carry `visible_actor_ids` when a visual scene needs the same
space with a different occupant set. An empty establishing shot should not
inherit every proxy actor just because the shared Blender scene contains them.
The render is the shot contract, not the whole stage dumped into one frame.

## Blockout, Not Mask

Do not treat flat colors as a machine-readable segmentation contract. A general
image model may loosely respect color and shape, but it has not agreed that
orange means anchor rail or blue means supervisor authority. The blockout should
therefore be readable in ordinary visual terms:

- the catwalk should look like a catwalk
- the crawl throat should look like a tunnel mouth
- the manifold should look like pipes and panels
- Teth's station should look like an open work-support rig
- humanoid proxies should mark human scale and blocking
- cephalopod proxies should mark nonhuman body volume and reach

Color helps a human and may help the model, but shape carries the real burden.

## Construction Controls

A useful blockout is not just visible geometry. The `.blend` may contain hidden
construction controls that define the stage:

- lane centerlines
- station footprints
- catwalk walkable zones
- manifold and rail axes
- reach envelopes
- camera-safe staging points

These controls are allowed to be invisible. Their job is to drive placement,
spacing, attachment, repetition, and later Geometry Nodes conversion. The
visible mesh is the output of the construction logic, not the whole graph.

For scripted prototypes, hidden empties and hidden curve paths are acceptable
control objects. For production organs, the same idea should move into authored
Geometry Nodes / Geometry Script modules: curves, points, fields, indices,
attributes, and intermediate geometry can exist purely to place or transform
other geometry. If every artifact in the graph is visible, the graph is probably
doing layout by brute force.

## Geometry Script Direction

The first generator used straightforward Blender Python primitives to prove that
Ghostlight could emit a Blender scene, cameras, and guide renders from fixture
data. That proved the seam, not the final method.

Primitive stacking is not expressive enough for production illustrated IF
blockouts. It creates plausible-looking clutter without readable authored
silhouettes, and it fails quickly on rooms that need clear construction logic,
nonhuman work affordances, and camera-safe spatial staging. Do not keep adding
more cylinders to compensate for weak shape language.

Production blockouts should move to authored Geometry Nodes / Geometry Script
modules or hand-authored Blender kit pieces driven by plan parameters. The plan
format is shaped for that replacement:

- every procedural organ has a `geometry_script_tree` handle
- every organ has parameter JSON
- every organ names its material and object class
- the generator can replace a Python primitive builder with a Geometry Script
  implementation one module at a time

This keeps the early blockout useful as a receipt while making the next real
step explicit: build reusable Geometry Nodes organs for shells, corridors,
cutouts, catwalks, manifold walls, service rigs, and crowd blocking. If an organ
cannot be recognized from silhouette before material detail, the blockout has
failed its job.

## Prompt Use

When rendering with an image model, use the blockout guide as a reference image
with language like:

```text
Use the attached blockout as spatial and camera reference. Preserve the room
layout, camera angle, large object positions, actor blocking, scale, and the
open work-support object class. Interpret the blockout into the requested
painterly style using the text prompt for character identity, lighting, mood,
and material detail.
```

Do not ask the image model to obey the blockout's colors as hidden labels. If a
color matters as light or material, state it in the text prompt.

## Generated Files

Generated `.blend` files and render outputs belong under `experiments/blender/`
unless a fixture intentionally promotes a blockout to a hand-maintained asset.
The JSON plan and generator scripts are source; rendered guides are receipts or
inputs for later image generation.
