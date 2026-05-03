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

## Geometry Script Direction

The first generator uses straightforward Blender Python primitives so the seam
can run without a Blender add-on. The plan is shaped for Geometry Script / Geometry
Nodes replacement:

- every procedural organ has a `geometry_script_tree` handle
- every organ has parameter JSON
- every organ names its material and object class
- the generator can replace a Python primitive builder with a Geometry Script
  implementation one module at a time

This keeps the early blockout useful while leaving room for real procedural
elaboration: repeated ribs, pipe runs, tunnel cutouts, cable trays, bolts,
route panels, catwalk rails, work-support rigs, and crowd clusters.

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
