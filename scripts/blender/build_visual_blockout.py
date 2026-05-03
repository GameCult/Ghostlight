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
    return add_bevel(obj, width=0.012, segments=1)


def add_mesh_object(name: str, vertices: list[tuple[float, float, float]], faces: list[tuple[int, ...]], material: bpy.types.Material) -> bpy.types.Object:
    mesh = bpy.data.meshes.new(f"{name}_mesh")
    mesh.from_pydata(vertices, [], faces)
    mesh.update()
    obj = bpy.data.objects.new(name, mesh)
    bpy.context.collection.objects.link(obj)
    obj.data.materials.append(material)
    return obj


def add_bevel(obj: bpy.types.Object, width: float = 0.03, segments: int = 2) -> bpy.types.Object:
    bevel = obj.modifiers.new("readable_blockout_bevel", "BEVEL")
    bevel.width = width
    bevel.segments = segments
    obj.modifiers.new("weighted_blockout_normals", "WEIGHTED_NORMAL")
    return obj


def add_control_empty(name: str, location: tuple[float, float, float]) -> bpy.types.Object:
    obj = bpy.data.objects.new(name, None)
    bpy.context.collection.objects.link(obj)
    obj.empty_display_type = "PLAIN_AXES"
    obj.empty_display_size = 0.25
    obj.location = location
    obj.hide_render = True
    obj.hide_viewport = True
    obj["ghostlight_control"] = True
    return obj


def add_control_curve(name: str, points: list[tuple[float, float, float]], closed: bool = False) -> bpy.types.Object:
    curve = bpy.data.curves.new(name, "CURVE")
    curve.dimensions = "3D"
    curve.resolution_u = 1
    spl = curve.splines.new("POLY")
    point_count = len(points) + (1 if closed else 0)
    spl.points.add(point_count - 1)
    path_points = points + ([points[0]] if closed else [])
    for point, coords in zip(spl.points, path_points):
        point.co = (coords[0], coords[1], coords[2], 1.0)
    obj = bpy.data.objects.new(name, curve)
    bpy.context.collection.objects.link(obj)
    obj.hide_render = True
    obj.hide_viewport = True
    obj["ghostlight_control"] = True
    obj["control_kind"] = "construction_curve"
    return obj


def make_runtime_material(material_id: str, color: tuple[float, float, float, float]) -> bpy.types.Material:
    material = bpy.data.materials.get(material_id) or bpy.data.materials.new(material_id)
    material.diffuse_color = color
    return material


def mark_actor_object(obj: bpy.types.Object, actor_id: str) -> bpy.types.Object:
    obj["ghostlight_actor_id"] = actor_id
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
    rib_count = int(p.get("rib_count", 18))
    y0 = -1.0
    y1 = length - 1.0
    half_width = float(p.get("half_width", 3.35))
    knee_height = float(p.get("knee_height", 1.05))
    crown_height = float(p.get("crown_height", 3.25))
    arch_steps = int(p.get("arch_steps", 20))

    add_control_empty("ctl_lane_center_start", (0, y0, 0))
    add_control_empty("ctl_lane_center_end", (0, y1, 0))
    add_control_empty("ctl_left_manifold_zone", (-half_width + 0.22, (y0 + y1) / 2, knee_height))
    add_control_empty("ctl_right_catwalk_zone", (half_width - 0.55, (y0 + y1) / 2, 1.55))
    add_control_empty("ctl_cephalopod_station_zone", (1.65, 4.75, 0.42))
    add_control_curve("ctl_lane_centerline_curve", [(0, y0, 0.02), (0, y1, 0.02)])
    add_control_curve("ctl_left_manifold_axis_curve", [(-half_width + 0.22, y0, knee_height), (-half_width + 0.22, y1, knee_height)])
    add_control_curve("ctl_right_catwalk_axis_curve", [(half_width - 0.55, y0, 1.55), (half_width - 0.55, y1, 1.55)])

    cross_section: list[tuple[float, float]] = [(-half_width, 0.0), (-half_width, knee_height)]
    for index in range(1, arch_steps):
        t = index / arch_steps
        x = -half_width + (2.0 * half_width * t)
        arch_factor = max(0.0, 1.0 - (x / half_width) ** 2)
        z = knee_height + (crown_height - knee_height) * math.sqrt(arch_factor)
        cross_section.append((x, z))
    cross_section.extend([(half_width, knee_height), (half_width, 0.0)])

    vertices: list[tuple[float, float, float]] = []
    for y in (y0, y1):
        for x, z in cross_section:
            vertices.append((x, y, max(z, 0.0)))
    faces: list[tuple[int, ...]] = []
    row = len(cross_section)
    for index in range(row - 1):
        faces.append((index, index + 1, row + index + 1, row + index))
    shell = add_mesh_object("service_ring_curved_inner_shell", vertices, faces, material)
    add_bevel(shell, width=0.015, segments=1)

    deck_width = half_width * 2.0
    add_cube("service_lane_bolted_deck", (0, length / 2 - 1.0, -0.035), (deck_width, length, 0.07), material)
    add_cube("left_floor_wall_seam", (-half_width + 0.08, length / 2 - 1.0, 0.08), (0.16, length, 0.16), material)
    add_cube("right_floor_wall_seam", (half_width - 0.08, length / 2 - 1.0, 0.08), (0.16, length, 0.16), material)
    for index in range(rib_count):
        y = y0 + index * ((y1 - y0) / max(rib_count - 1, 1))
        add_curve_pipe(
            f"shell_rib_{index:02d}",
            [(x, y, max(z + 0.035, 0.0)) for x, z in cross_section],
            material,
            bevel_depth=0.035,
        )
    for x_offset in (-1.1, 0.0, 1.1):
        name = "overhead_service_spine" if x_offset == 0 else f"overhead_service_spine_{x_offset:+.1f}"
        add_curve_pipe(name, [(x_offset, y0, crown_height - 0.10), (x_offset, y1, crown_height - 0.10)], material, bevel_depth=0.045)
    for index in range(7):
        y = y0 + 1.0 + index * 2.2
        add_cube(f"deck_cross_plate_{index:02d}", (0, y, 0.11), (deck_width * 0.82, 0.08, 0.035), material)


def build_crawl_throat(module: dict[str, Any], materials: dict[str, bpy.types.Material]) -> None:
    p = module["parameters"]
    material = materials[module["material_id"]]
    dark = make_runtime_material("crawl_void_black", (0.02, 0.018, 0.014, 1.0))
    x, y, z = p.get("position", [0, 8.2, 1.15])
    radius = float(p.get("radius", 1.15))
    depth = float(p.get("depth", 0.35))
    add_cube("kappa_end_bulkhead_plate", (x, y + depth * 0.55, z), (3.35, 0.16, 2.75), material)
    bpy.ops.mesh.primitive_torus_add(major_radius=radius, minor_radius=0.13, major_segments=64, minor_segments=10, location=(x, y, z), rotation=(math.pi / 2, 0, 0))
    ring = bpy.context.object
    ring.name = "kappa_crawl_throat_gasket_ring"
    ring.scale.y = 0.68
    ring.data.materials.append(material)
    add_cylinder("kappa_crawl_throat_dark_inset", (x, y - 0.03, z), radius * 0.84, 0.05, dark, vertices=48, rotation=(math.pi / 2, 0, 0))
    for sx in (-1, 1):
        add_curve_pipe(
            f"crawl_warning_conduit_{sx}",
            [(x + sx * (radius + 0.25), y - 0.06, z - 0.8), (x + sx * (radius + 0.45), y - 0.06, z), (x + sx * (radius + 0.25), y - 0.06, z + 0.8)],
            material,
            bevel_depth=0.035,
        )


def build_manifold_wall(module: dict[str, Any], materials: dict[str, bpy.types.Material]) -> None:
    p = module["parameters"]
    material = materials[module["material_id"]]
    x, y, z = p.get("position", [-3.0, 1.0, 1.25])
    length = float(p.get("length", 11.0))
    pipe_count = int(p.get("pipe_count", 8))
    panel_count = int(p.get("panel_count", 6))
    add_cube("manifold_wall_backplane", (x - 0.02, y + length / 2, z + 0.15), (0.16, length + 0.8, 2.45), material)
    for index in range(pipe_count):
        yy = y + 0.25 + index * (length - 0.5) / max(pipe_count - 1, 1)
        add_curve_pipe(f"manifold_vertical_pipe_{index:02d}", [(x + 0.10, yy, 0.45), (x + 0.10, yy, 2.72)], material, bevel_depth=0.032 + 0.006 * (index % 3))
        if index % 2 == 0:
            add_curve_pipe(f"manifold_bypass_loop_{index:02d}", [(x + 0.10, yy, 1.15), (x + 0.38, yy + 0.18, 1.45), (x + 0.10, yy + 0.42, 1.85)], material, bevel_depth=0.026)
    for index in range(panel_count):
        yy = y + 0.6 + index * (length - 1.2) / max(panel_count - 1, 1)
        panel_z = z + 0.22 * (index % 3)
        add_cube(f"manifold_route_panel_{index:02d}", (x + 0.14, yy, panel_z), (0.10, 0.72, 0.48), material)
        add_cube(f"manifold_status_screen_{index:02d}", (x + 0.20, yy - 0.16, panel_z + 0.18), (0.05, 0.26, 0.16), material)
        add_cylinder(f"manifold_valve_wheel_{index:02d}", (x + 0.24, yy + 0.32, z - 0.08), 0.16, 0.06, material, vertices=24, rotation=(0, math.pi / 2, 0))


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
    add_control_empty("ctl_catwalk_centerline", (x, y, z))
    add_control_curve(
        "ctl_catwalk_walkable_footprint",
        [
            (x - width / 2, y - length / 2, z + 0.06),
            (x + width / 2, y - length / 2, z + 0.06),
            (x + width / 2, y + length / 2, z + 0.06),
            (x - width / 2, y + length / 2, z + 0.06),
        ],
        closed=True,
    )
    add_cube("supervisor_catwalk_grated_deck", (x, y, z), (width, length, 0.10), material)
    add_cube("supervisor_catwalk_wall_mount", (x + width / 2 + 0.08, y, z + 0.18), (0.14, length, 0.34), material)
    for side, sx in (("inner", -1), ("outer", 1)):
        rail_x = x + sx * width / 2
        add_curve_pipe(f"supervisor_catwalk_{side}_top_rail", [(rail_x, y - length / 2, z + rail_height), (rail_x, y + length / 2, z + rail_height)], material, bevel_depth=0.028)
        add_curve_pipe(f"supervisor_catwalk_{side}_mid_rail", [(rail_x, y - length / 2, z + rail_height * 0.52), (rail_x, y + length / 2, z + rail_height * 0.52)], material, bevel_depth=0.018)
        for index in range(5):
            yy = y - length / 2 + index * length / 4
            add_curve_pipe(f"supervisor_catwalk_{side}_post_{index:02d}", [(rail_x, yy, z), (rail_x, yy, z + rail_height)], material, bevel_depth=0.022)
    for index in range(5):
        yy = y - length / 2 + 0.45 + index * (length - 0.9) / 4
        add_cube(f"supervisor_screen_{index:02d}", (x - width / 2 - 0.10, yy, z + 0.86), (0.05, 0.58, 0.34), material)
    for index in range(3):
        yy = y - length / 2 + index * length / 2
        add_curve_pipe(f"supervisor_catwalk_diagonal_brace_{index:02d}", [(x + width / 2, yy, z), (x + width / 2 + 0.35, yy + 0.7, 0.65)], material, bevel_depth=0.026)


def build_cephalopod_work_support_station(module: dict[str, Any], materials: dict[str, bpy.types.Material]) -> None:
    p = module["parameters"]
    material = materials[module["material_id"]]
    x, y, z = p.get("position", [1.8, 4.7, 0.95])
    width = float(p.get("width", 2.5))
    depth = float(p.get("depth", 1.6))
    height = float(p.get("height", 1.2))
    add_control_empty("ctl_teth_station_center", (x, y, z))
    add_control_curve(
        "ctl_teth_station_service_footprint",
        [
            (x - width / 2, y - depth / 2, z - 0.18),
            (x + width / 2, y - depth / 2, z - 0.18),
            (x + width / 2, y + depth / 2, z - 0.18),
            (x - width / 2, y + depth / 2, z - 0.18),
        ],
        closed=True,
    )
    add_control_curve("ctl_teth_primary_reach_axis", [(x - width * 0.45, y, z + 0.08), (x + width * 0.45, y, z + 0.08)])
    add_cube("teth_work_support_low_service_sled", (x, y, z - 0.18), (width, depth, 0.12), material)
    pad_material = make_runtime_material("teth_soft_pad_muted_teal", (0.12, 0.24, 0.28, 1.0))
    add_cube("teth_work_support_soft_pad", (x, y, z - 0.08), (width * 0.72, depth * 0.62, 0.08), pad_material)
    for sx in (-1, 1):
        rail_x = x + sx * width * 0.42
        add_curve_pipe(f"teth_longitudinal_side_rail_{sx}", [(rail_x, y - depth / 2, z + 0.06), (rail_x, y + depth / 2, z + 0.06)], material, bevel_depth=0.028)
        add_curve_pipe(f"teth_low_contact_arc_{sx}", [(rail_x, y - depth * 0.30, z + 0.03), (rail_x + sx * 0.06, y, z + height * 0.25), (rail_x, y + depth * 0.30, z + 0.03)], material, bevel_depth=0.024)
    for yy_factor in (-0.30, 0.30):
        yy = y + yy_factor * depth
        add_curve_pipe(f"teth_cross_support_band_{yy_factor:+.2f}", [(x - width * 0.38, yy, z + 0.12), (x, yy, z + 0.22), (x + width * 0.38, yy, z + 0.12)], material, bevel_depth=0.022)
    for index, yy in enumerate((y - depth * 0.45, y + depth * 0.45)):
        add_curve_pipe(f"teth_oxygenation_tube_{index:02d}", [(x - width * 0.48, yy, z + 0.08), (x, yy, z + height * 0.42), (x + width * 0.48, yy, z + 0.08)], material, bevel_depth=0.020)
    add_cube("teth_front_tool_block", (x, y - depth * 0.82, z + 0.18), (1.15, 0.16, 0.16), material)
    for index in range(3):
        offset = -0.35 + index * 0.35
        add_curve_pipe(f"teth_short_tool_nozzle_{index:02d}", [(x + offset, y - depth * 0.86, z + 0.20), (x + offset, y - depth * 1.03, z + 0.28)], material, bevel_depth=0.014)


def build_object_callback_props(module: dict[str, Any], materials: dict[str, bpy.types.Material]) -> None:
    p = module["parameters"]
    material = materials[module["material_id"]]
    teth_material = materials.get("teth_blue_gray", material)
    x, y, z = p.get("position", [1.05, 5.85, 0.72])
    add_cube("callback_low_parts_tray", (x, y, z), (1.2, 0.55, 0.08), material)
    add_cylinder("callback_thermal_mug", (x - 0.35, y - 0.12, z + 0.18), 0.11, 0.28, material, vertices=24)
    add_curve_pipe(
        "callback_repaired_contact_band",
        [
            (x - 0.05, y - 0.20, z + 0.14),
            (x + 0.18, y - 0.10, z + 0.24),
            (x + 0.42, y - 0.20, z + 0.14),
        ],
        teth_material,
        bevel_depth=0.018,
    )
    add_curve_pipe(
        "callback_spare_oxygenation_tube",
        [
            (x - 0.48, y + 0.10, z + 0.13),
            (x - 0.08, y + 0.22, z + 0.20),
            (x + 0.48, y + 0.08, z + 0.13),
        ],
        teth_material,
        bevel_depth=0.026,
    )
    add_cube("callback_tagged_limit_interface", (x + 0.22, y + 0.14, z + 0.17), (0.28, 0.16, 0.05), material)


MODULE_BUILDERS = {
    "service_ring_shell": build_service_ring_shell,
    "crawl_throat": build_crawl_throat,
    "manifold_wall": build_manifold_wall,
    "anchor_rail": build_anchor_rail,
    "supervisor_catwalk": build_supervisor_catwalk,
    "cephalopod_work_support_station": build_cephalopod_work_support_station,
    "object_callback_props": build_object_callback_props,
}


def add_humanoid_proxy(actor: dict[str, Any], material: bpy.types.Material) -> None:
    x, y, z = actor["default_position"]
    sx, sy, sz = actor["scale"]
    obj = mark_actor_object(add_uv_sphere(actor["actor_id"], (x, y, z + sz / 2), (sx, sy, sz / 2), material), actor["actor_id"])
    obj.rotation_euler = tuple(actor["default_rotation"])
    if "slate" in actor["proxy_shape"]:
        mark_actor_object(add_cube(f"{actor['actor_id']}_slate", (x + 0.25, y, z + 1.0), (0.35, 0.04, 0.25), material), actor["actor_id"])
    if "hook" in actor["proxy_shape"]:
        mark_actor_object(add_curve_pipe(f"{actor['actor_id']}_hook", [(x, y, z + 0.8), (x + 0.45, y, z + 0.65)], material, bevel_depth=0.035), actor["actor_id"])


def add_cephalopod_proxy(actor: dict[str, Any], material: bpy.types.Material) -> None:
    x, y, z = actor["default_position"]
    sx, sy, sz = actor["scale"]
    mark_actor_object(add_uv_sphere(actor["actor_id"], (x, y, z + 0.25), (sx * 0.45, sy * 0.45, sz * 0.45), material), actor["actor_id"])
    for index in range(8):
        angle = index * math.tau / 8
        mark_actor_object(add_curve_pipe(
            f"{actor['actor_id']}_tentacle_{index:02d}",
            [
                (x, y, z + 0.22),
                (x + math.cos(angle) * sx * 0.45, y + math.sin(angle) * sy * 0.45, z + 0.15),
                (x + math.cos(angle) * sx * 0.75, y + math.sin(angle) * sy * 0.75, z + 0.08),
            ],
            material,
            bevel_depth=0.035,
        ), actor["actor_id"])


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
    actor_objects = [obj for obj in bpy.data.objects if "ghostlight_actor_id" in obj]
    for render in plan["guide_renders"]:
        camera = cameras.get(render["camera_id"])
        if not camera:
            print(f"warning: missing camera {render['camera_id']} for {render['render_id']}")
            continue
        visible_actor_ids = render.get("visible_actor_ids")
        if visible_actor_ids is not None:
            visible = set(visible_actor_ids)
            for obj in actor_objects:
                hide = obj["ghostlight_actor_id"] not in visible
                obj.hide_viewport = hide
                obj.hide_render = hide
        else:
            for obj in actor_objects:
                obj.hide_viewport = False
                obj.hide_render = False
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
