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
npm run blockout:build-geonodes -- --plan examples\visual-blockouts\pallas-species-strikes.service-ring-kappa.v0.blockout.json --save-blend --render-guides
```

The build script runs Blender in background mode and writes generated files under
`experiments/blender/` by default.

## Builder Paths

- `prototype`: legacy direct-object builder. It is useful as an old receipt, not
  the target production path.
- `geonodes`: current Geometry Nodes builder. It creates a visible stage object
  with a GN modifier and emits the service-ring maquette through a node tree.

The Geometry Nodes builder follows the pattern from procedural GN scripting:
create a `GeometryNodeTree`, build an explicit interface, create nodes with
support checks, use fields and math for generated surfaces, use placement
geometry for repeated details, assign materials in the graph, join the generated
organs, and attach the node group to a modifier.
