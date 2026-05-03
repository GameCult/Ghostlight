"""Build Ghostlight visual blockout scenes in Blender from a blockout plan.

Run with Blender, for example:

    "C:\\Program Files (x86)\\Steam\\steamapps\\common\\Blender\\blender.exe" --background --python scripts/blender/build_visual_blockout.py -- --plan examples/visual-blockouts/pallas-species-strikes.service-ring-kappa.v0.blockout.json --save-blend --render-guides

This first generator is intentionally low-poly and fully scripted. The module
names and `geometry_script_tree` handles in the JSON are the seam where Carson
Katri's Geometry Script / Geometry Nodes implementations can replace the simple
bpy primitives module by module.
"""

from __future__ import annotations

import argparse
import json
import math
import sys
from pathlib import Path
from typing import Any

try:
    import bpy
    from mathutils import Vector
except Exception as exc:  # pragma: no cover - only importable inside Blender.
    raise SystemExit("This script must be run with Blender's Python interpreter.") from exc


ROOT = Path(__file__).resolve().parents[2]


def parse_script_args() -> argparse.Namespace:
    argv = sys.argv
    if "--" in argv:
        argv = argv[argv.index("--") + 1 :]
    else:
        argv = []
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--plan", type=Path, required=True)
    parser.add_argument("--save-blend", action="store_true")
    parser.add_argument("--render-guides", action="store_true")
    parser.add_argument("--output-blend", type=Path)
    parser.add_argument("--output-dir", type=Path)
    parser.add_argument("--resolution-x", type=int, default=1536)
    parser.add_argument("--resolution-y", type=int, default=864)
    return parser.parse_args(argv)


def load_plan(path: Path) -> dict[str, Any]:
    plan_path = path if path.is_absolute() else ROOT / path
    return json.loads(plan_path.read_text(encoding="utf-8"))


def resolve_project_path(path: str | Path) -> Path:
    value = Path(path)
    return value if value.is_absolute() else ROOT / value


def clear_scene() -> None:
    bpy.ops.object.select_all(action="SELECT")
    bpy.ops.object.delete()
    bpy.context.scene.unit_settings.system = "METRIC"


def make_material(material_id: str, color: list[float]) -> bpy.types.Material:
    material = bpy.data.materials.new(material_id)
    material.diffuse_color = tuple(color)
    return material


def material_map(plan: dict[str, Any]) -> dict[str, bpy.types.Material]:
    return {
        item["material_id"]: make_material(item["material_id"], item["base_color"])
        for item in plan["semantic_materials"]
    }


def add_cube(name: str, location: tuple[float, float, float], scale: tuple[float, float, float], material: bpy.types.Material) -> bpy.types.Object:
    bpy.ops.mesh.primitive_cube_add(size=1.0, location=location)
    obj = bpy.context.object
    obj.name = name
    obj.dimensions = scale
    bpy.ops.object.transform_apply(location=False, rotation=False, scale=True)
    obj.data.materials.append(material)
    return obj


def add_cylinder(name: str, location: tuple[float, float, float], radius: float, depth: float, material: bpy.types.Material, vertices: int = 48, rotation: tuple[float, float, float] = (0, 0, 0)) -> bpy.types.Object:
    bpy.ops.mesh.primitive_cylinder_add(vertices=vertices, radius=radius, depth=depth, location=location, rotation=rotation)
    obj = bpy.context.object
    obj.name = name
    obj.data.materials.append(material)
    return obj


def add_uv_sphere(name: str, location: tuple[float, float, float], scale: tuple[float, float, float], material: bpy.types.Material) -> bpy.types.Object:
    bpy.ops.mesh.primitive_uv_sphere_add(segments=32, ring_count=16, location=location)
    obj = bpy.context.object
    obj.name = name
    obj.scale = scale
    obj.data.materials.append(material)
    return obj


def add_curve_pipe(name: str, points: list[tuple[float, float, float]], material: bpy.types.Material, bevel_depth: float = 0.035) -> bpy.types.Object:
    curve = bpy.data.curves.new(name, "CURVE")
    curve.dimensions = "3D"
    curve.resolution_u = 3
    curve.bevel_depth = bevel_depth
    curve.bevel_resolution = 3
    spl = curve.splines.new("POLY")
    spl.points.add(len(points) - 1)
    for point, coords in zip(spl.points, points):
        point.co = (coords[0], coords[1], coords[2], 1.0)
    obj = bpy.data.objects.new(name, curve)
    bpy.context.collection.objects.link(obj)
    obj.data.materials.append(material)
    return obj


def build_service_ring_shell(module: dict[str, Any], materials: dict[str, bpy.types.Material]) -> None:
    p = module["parameters"]
    material = materials[module["material_id"]]
    length = float(p.get("length", 18.0))
    radius = float(p.get("radius", 4.2))
    rib_count = int(p.get("rib_count", 18))
    # A readable maquette: curved ceiling ribs plus a flat service deck. Full
    # boolean shell carving can arrive when Geometry Script takes over.
    add_cube("service_lane_deck", (0, length / 2 - 1.0, 0.0), (6.4, length, 0.12), material)
    add_cube("left_shell_wall", (-3.25, length / 2 - 1.0, 1.65), (0.18, length, 3.2), material)
    add_cube("right_shell_wall", (3.25, length / 2 - 1.0, 1.65), (0.18, length, 3.2), material)
    for index in range(rib_count):
        y = -1.0 + index * (length / max(rib_count - 1, 1))
        add_curve_pipe(
            f"shell_rib_{index:02d}",
            [(-3.2, y, 0.35), (-2.4, y, 2.6), (0.0, y, radius), (2.4, y, 2.6), (3.2, y, 0.35)],
            material,
            bevel_depth=0.045,
        )
    add_curve_pipe("overhead_service_spine", [(0.0, -1.0, 3.85), (0.0, length - 1.0, 3.85)], material, bevel_depth=0.10)


def build_crawl_throat(module: dict[str, Any], materials: dict[str, bpy.types.Material]) -> None:
    p = module["parameters"]
    material = materials[module["material_id"]]
    pos = tuple(p.get("position", [0, 8.2, 1.15]))
    add_cylinder("kappa_crawl_throat_dark_oval", pos, float(p.get("radius", 1.15)), float(p.get("depth", 0.35)), material, rotation=(math.pi / 2, 0, 0))
    add_cylinder("kappa_crawl_throat_gasket", (pos[0], pos[1] - 0.02, pos[2]), float(p.get("radius", 1.28)), 0.08, material, rotation=(math.pi / 2, 0, 0))


def build_manifold_wall(module: dict[str, Any], materials: dict[str, bpy.types.Material]) -> None:
    p = module["parameters"]
    material = materials[module["material_id"]]
    x, y, z = p.get("position", [-3.0, 1.0, 1.25])
    length = float(p.get("length", 11.0))
    pipe_count = int(p.get("pipe_count", 8))
    panel_count = int(p.get("panel_count", 6))
    for index in range(pipe_count):
        yy = y + index * length / max(pipe_count - 1, 1)
        add_curve_pipe(f"manifold_pipe_{index:02d}", [(x, yy, 0.65), (x, yy, 2.35)], material, bevel_depth=0.035)
    for index in range(panel_count):
        yy = y + 0.5 + index * (length - 1.0) / max(panel_count - 1, 1)
        add_cube(f"manifold_panel_{index:02d}", (x + 0.05, yy, z + 0.25 * (index % 3)), (0.08, 0.65, 0.42), material)
        add_cylinder(f"manifold_valve_{index:02d}", (x + 0.12, yy + 0.38, z), 0.14, 0.05, material, vertices=24, rotation=(0, math.pi / 2, 0))


def build_anchor_rail(module: dict[str, Any], materials: dict[str, bpy.types.Material]) -> None:
    p = module["parameters"]
    material = materials[module["material_id"]]
    x, y, z = p.get("position", [2.9, 0.0, 0.55])
    length = float(p.get("length", 12.0))
    hook_count = int(p.get("hook_count", 5))
    add_cube("baseline_anchor_rail_track", (x, y + length / 2, z), (0.16, length, 0.14), material)
    for index in range(hook_count):
        yy = y + 0.8 + index * (length - 1.6) / max(hook_count - 1, 1)
        add_cylinder(f"anchor_hook_{index:02d}", (x, yy, z + 0.35), 0.12, 0.42, material, vertices=16, rotation=(math.pi / 2, 0, 0))
        add_curve_pipe(f"anchor_tether_{index:02d}", [(x, yy, z + 0.45), (x - 0.4, yy, z + 0.75)], material, bevel_depth=0.018)


def build_supervisor_catwalk(module: dict[str, Any], materials: dict[str, bpy.types.Material]) -> None:
    p = module["parameters"]
    material = materials[module["material_id"]]
    x, y, z = p.get("position", [3.1, 3.2, 2.75])
    length = float(p.get("length", 6.5))
    width = float(p.get("width", 1.5))
    rail_height = float(p.get("rail_height", 0.9))
    add_cube("supervisor_catwalk_deck", (x, y, z), (width, length, 0.12), material)
    add_cube("supervisor_catwalk_rail_top", (x - width / 2, y, z + rail_height), (0.08, length, 0.08), material)
    add_cube("supervisor_catwalk_rail_lower", (x - width / 2, y, z + rail_height * 0.5), (0.05, length, 0.05), material)
    for index in range(5):
        yy = y - length / 2 + index * length / 4
        add_cube(f"supervisor_screen_{index:02d}", (x + 0.05, yy, z + 1.35), (0.06, 0.75, 0.5), material)


def build_cephalopod_work_support_station(module: dict[str, Any], materials: dict[str, bpy.types.Material]) -> None:
    p = module["parameters"]
    material = materials[module["material_id"]]
    x, y, z = p.get("position", [1.8, 4.7, 0.95])
    width = float(p.get("width", 2.5))
    depth = float(p.get("depth", 1.6))
    height = float(p.get("height", 1.2))
    add_cube("teth_work_support_open_frame", (x, y, z), (width, depth, 0.10), material)
    for sx in (-1, 1):
        add_curve_pipe(f"teth_support_loop_{sx}", [(x + sx * width / 2, y - depth / 2, z), (x + sx * width / 2, y, z + height), (x + sx * width / 2, y + depth / 2, z)], material, bevel_depth=0.025)
    for index in range(5):
        angle = (index / 5.0) * math.tau
        add_curve_pipe(
            f"teth_tool_rail_{index:02d}",
            [(x, y, z + 0.25), (x + math.cos(angle) * width * 0.45, y + math.sin(angle) * depth * 0.45, z + 0.55)],
            material,
            bevel_depth=0.018,
        )
    add_curve_pipe("teth_oxygenation_loop", [(x - 0.8, y - 0.75, z + 0.25), (x, y, z + 1.0), (x + 0.8, y - 0.75, z + 0.25)], material, bevel_depth=0.028)


MODULE_BUILDERS = {
    "service_ring_shell": build_service_ring_shell,
    "crawl_throat": build_crawl_throat,
    "manifold_wall": build_manifold_wall,
    "anchor_rail": build_anchor_rail,
    "supervisor_catwalk": build_supervisor_catwalk,
    "cephalopod_work_support_station": build_cephalopod_work_support_station,
}


def add_humanoid_proxy(actor: dict[str, Any], material: bpy.types.Material) -> None:
    x, y, z = actor["default_position"]
    sx, sy, sz = actor["scale"]
    obj = add_uv_sphere(actor["actor_id"], (x, y, z + sz / 2), (sx, sy, sz / 2), material)
    obj.rotation_euler = tuple(actor["default_rotation"])
    if "slate" in actor["proxy_shape"]:
        add_cube(f"{actor['actor_id']}_slate", (x + 0.25, y, z + 1.0), (0.35, 0.04, 0.25), material)
    if "hook" in actor["proxy_shape"]:
        add_curve_pipe(f"{actor['actor_id']}_hook", [(x, y, z + 0.8), (x + 0.45, y, z + 0.65)], material, bevel_depth=0.035)


def add_cephalopod_proxy(actor: dict[str, Any], material: bpy.types.Material) -> None:
    x, y, z = actor["default_position"]
    sx, sy, sz = actor["scale"]
    add_uv_sphere(actor["actor_id"], (x, y, z + 0.25), (sx * 0.45, sy * 0.45, sz * 0.45), material)
    for index in range(8):
        angle = index * math.tau / 8
        add_curve_pipe(
            f"{actor['actor_id']}_tentacle_{index:02d}",
            [
                (x, y, z + 0.22),
                (x + math.cos(angle) * sx * 0.45, y + math.sin(angle) * sy * 0.45, z + 0.15),
                (x + math.cos(angle) * sx * 0.75, y + math.sin(angle) * sy * 0.75, z + 0.08),
            ],
            material,
            bevel_depth=0.035,
        )


def build_proxy_actors(plan: dict[str, Any], materials: dict[str, bpy.types.Material]) -> None:
    for actor in plan.get("proxy_actors", []):
        material = materials[actor["material_id"]]
        if "cephalopod" in actor["proxy_shape"]:
            add_cephalopod_proxy(actor, material)
        else:
            add_humanoid_proxy(actor, material)


def build_cameras(plan: dict[str, Any]) -> None:
    for camera in plan["cameras"]:
        bpy.ops.object.camera_add(location=tuple(camera["position"]), rotation=tuple(camera["rotation"]))
        obj = bpy.context.object
        obj.name = camera["camera_id"]
        if "target" in camera:
            direction = Vector(tuple(camera["target"])) - obj.location
            obj.rotation_euler = direction.to_track_quat("-Z", "Y").to_euler()
        obj.data.lens = float(camera["lens_mm"])
        obj["visual_scene_ids"] = ",".join(camera["visual_scene_ids"])
        obj["shot_role"] = camera["shot_role"]
        obj["framing_notes"] = camera["framing_notes"]


def add_lights() -> None:
    bpy.ops.object.light_add(type="AREA", location=(0, 2.0, 4.2))
    light = bpy.context.object
    light.name = "soft_blockout_overhead_light"
    light.data.energy = 450
    light.data.size = 5.0
    bpy.ops.object.light_add(type="POINT", location=(-2.2, 4.0, 1.8))
    amber = bpy.context.object
    amber.name = "amber_manifold_guide_light"
    amber.data.energy = 160
    amber.data.color = (1.0, 0.55, 0.15)
    bpy.ops.object.light_add(type="POINT", location=(3.2, 3.2, 3.4))
    blue = bpy.context.object
    blue.name = "blue_supervisor_guide_light"
    blue.data.energy = 140
    blue.data.color = (0.25, 0.45, 1.0)


def configure_scene(resolution_x: int, resolution_y: int) -> None:
    scene = bpy.context.scene
    scene.render.engine = "BLENDER_WORKBENCH"
    scene.display.shading.light = "STUDIO"
    scene.display.shading.color_type = "MATERIAL"
    scene.render.resolution_x = resolution_x
    scene.render.resolution_y = resolution_y
    scene.view_settings.view_transform = "Standard"
    scene.view_settings.look = "Medium High Contrast"


def build_scene(plan: dict[str, Any], resolution_x: int, resolution_y: int) -> None:
    clear_scene()
    materials = material_map(plan)
    for module in plan["geometry_modules"]:
        builder = MODULE_BUILDERS.get(module["geometry_script_tree"])
        if not builder:
            print(f"warning: no builder for {module['geometry_script_tree']}")
            continue
        builder(module, materials)
    build_proxy_actors(plan, materials)
    build_cameras(plan)
    add_lights()
    configure_scene(resolution_x, resolution_y)


def save_blend(plan: dict[str, Any], output_blend: Path | None) -> None:
    target = output_blend or resolve_project_path(plan["layout"]["layout_asset"])
    target.parent.mkdir(parents=True, exist_ok=True)
    bpy.ops.wm.save_as_mainfile(filepath=str(target))
    print(f"wrote {target}")


def render_guides(plan: dict[str, Any], output_dir: Path | None) -> None:
    render_root = output_dir or ROOT
    cameras = {obj.name: obj for obj in bpy.data.objects if obj.type == "CAMERA"}
    for render in plan["guide_renders"]:
        camera = cameras.get(render["camera_id"])
        if not camera:
            print(f"warning: missing camera {render['camera_id']} for {render['render_id']}")
            continue
        output_ref = resolve_project_path(render["output_refs"][0])
        if output_dir:
            output_ref = output_dir / Path(render["output_refs"][0]).name
        output_ref.parent.mkdir(parents=True, exist_ok=True)
        bpy.context.scene.camera = camera
        bpy.context.scene.render.filepath = str(output_ref)
        bpy.ops.render.render(write_still=True)
        print(f"wrote {output_ref}")


def main() -> int:
    args = parse_script_args()
    plan = load_plan(args.plan)
    build_scene(plan, args.resolution_x, args.resolution_y)
    if args.save_blend:
        save_blend(plan, args.output_blend)
    if args.render_guides:
        render_guides(plan, args.output_dir)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
