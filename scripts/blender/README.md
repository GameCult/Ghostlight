# Blender Blockout Scripts

These scripts build low-poly spatial guide scenes for Ghostlight illustrated IF.
They are not final art. They are stage maquettes for image generation.

## Blender Path

Blender may not be on `PATH`. The known local install is:

```powershell
C:\Program Files (x86)\Steam\steamapps\common\Blender\blender.exe
```

You can pass another path to the npm wrapper with `--blender`.

## Build Pallas Blockout

```powershell
npm run blockout:validate
npm run blockout:build -- --plan examples\visual-blockouts\pallas-species-strikes.service-ring-kappa.v0.blockout.json --save-blend --render-guides
```

The build script runs Blender in background mode and writes generated files under
`experiments/blender/` by default.

## Geometry Script Direction

The current builder uses plain `bpy` primitives for reliability. The plan format
already names `geometry_script_tree` handles for each procedural organ, so the
primitive builders can be replaced with Geometry Script / Geometry Nodes modules
without changing downstream fixture data.
