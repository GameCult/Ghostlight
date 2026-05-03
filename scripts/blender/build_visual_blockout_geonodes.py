"""Build a Ghostlight visual blockout using an actual Geometry Nodes graph.

This is the replacement path for the first primitive-object prototype. It
creates one visible stage object with a Geometry Nodes modifier and builds the
service-ring maquette inside that node tree: base meshes, transforms, instanced
repetition, material assignment, and final joined geometry all live in the GN
graph.
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


def socket(iface: bpy.types.NodeTreeInterface, *, name: str, in_out: str, socket_type: str, desc: str) -> Any:
    return iface.new_socket(name=name, in_out=in_out, socket_type=socket_type, description=desc)


def node_supported(bl_idname: str) -> bool:
    return hasattr(bpy.types, bl_idname)


def new_node(nodes: bpy.types.Nodes, bl_idname: str, *, location: tuple[int, int]) -> bpy.types.Node:
    if not node_supported(bl_idname):
        raise RuntimeError(f"Unsupported Geometry Nodes type in this Blender build: {bl_idname}")
    node = nodes.new(bl_idname)
    node.location = location
    return node


def make_material(material_id: str, color: list[float]) -> bpy.types.Material:
    material = bpy.data.materials.new(material_id)
    material.diffuse_color = tuple(color)
    return material


def material_map(plan: dict[str, Any]) -> dict[str, bpy.types.Material]:
    return {
        item["material_id"]: make_material(item["material_id"], item["base_color"])
        for item in plan["semantic_materials"]
    }


def create_stage_object() -> bpy.types.Object:
    bpy.ops.mesh.primitive_plane_add(size=0.01, location=(0, 0, 0))
    obj = bpy.context.object
    obj.name = "service_ring_kappa_geonodes_stage"
    obj.hide_render = False
    return obj


def create_node_group(name: str) -> bpy.types.GeometryNodeTree:
    ng = bpy.data.node_groups.new(name, "GeometryNodeTree")
    for node in list(ng.nodes):
        ng.nodes.remove(node)
    socket(ng.interface, name="Geometry", in_out="OUTPUT", socket_type="NodeSocketGeometry", desc="Generated stage geometry")
    return ng


def attach_group_modifier(obj: bpy.types.Object, ng: bpy.types.GeometryNodeTree) -> bpy.types.NodesModifier:
    modifier = obj.modifiers.new(name="Ghostlight Procedural Stage", type="NODES")
    modifier.node_group = ng
    return modifier


class GraphBuilder:
    def __init__(self, node_group: bpy.types.GeometryNodeTree):
        self.ng = node_group
        self.nodes = node_group.nodes
        self.links = node_group.links
        self.join = new_node(self.nodes, "GeometryNodeJoinGeometry", location=(1800, 0))
        self.output = new_node(self.nodes, "NodeGroupOutput", location=(2050, 0))
        self.links.new(self.join.outputs["Geometry"], self.output.inputs["Geometry"])
        self.row = 0

    def _location(self, column: int) -> tuple[int, int]:
        return (column, 460 - self.row * 130)

    def _advance(self) -> None:
        self.row += 1

    def add_to_join(self, geometry_socket: bpy.types.NodeSocket) -> None:
        self.links.new(geometry_socket, self.join.inputs["Geometry"])

    def cube(
        self,
        name: str,
        *,
        size: tuple[float, float, float],
        translation: tuple[float, float, float],
        material: bpy.types.Material,
    ) -> bpy.types.NodeSocket:
        cube = new_node(self.nodes, "GeometryNodeMeshCube", location=self._location(-1400))
        cube.label = f"{name}: source cube"
        cube.inputs["Size"].default_value = size
        cube.inputs["Vertices X"].default_value = 2
        cube.inputs["Vertices Y"].default_value = 2
        cube.inputs["Vertices Z"].default_value = 2

        transform = new_node(self.nodes, "GeometryNodeTransform", location=self._location(-1120))
        transform.label = f"{name}: transform"
        transform.inputs["Translation"].default_value = translation
        self.links.new(cube.outputs["Mesh"], transform.inputs["Geometry"])

        set_material = new_node(self.nodes, "GeometryNodeSetMaterial", location=self._location(-860))
        set_material.label = f"{name}: material"
        set_material.inputs["Material"].default_value = material
        self.links.new(transform.outputs["Geometry"], set_material.inputs["Geometry"])
        self._advance()
        return set_material.outputs["Geometry"]

    def cube_array(
        self,
        name: str,
        *,
        count: int,
        start: tuple[float, float, float],
        offset: tuple[float, float, float],
        instance_size: tuple[float, float, float],
        material: bpy.types.Material,
    ) -> bpy.types.NodeSocket:
        line = new_node(self.nodes, "GeometryNodeMeshLine", location=self._location(-1500))
        line.label = f"{name}: placement line"
        line.mode = "OFFSET"
        line.inputs["Count"].default_value = count
        line.inputs["Start Location"].default_value = start
        line.inputs["Offset"].default_value = offset

        cube = new_node(self.nodes, "GeometryNodeMeshCube", location=self._location(-1500))
        cube.label = f"{name}: instance cube"
        cube.inputs["Size"].default_value = instance_size

        instance = new_node(self.nodes, "GeometryNodeInstanceOnPoints", location=self._location(-1220))
        instance.label = f"{name}: instance on points"
        self.links.new(line.outputs["Mesh"], instance.inputs["Points"])
        self.links.new(cube.outputs["Mesh"], instance.inputs["Instance"])

        realize = new_node(self.nodes, "GeometryNodeRealizeInstances", location=self._location(-980))
        realize.label = f"{name}: realize"
        self.links.new(instance.outputs["Instances"], realize.inputs["Geometry"])

        set_material = new_node(self.nodes, "GeometryNodeSetMaterial", location=self._location(-760))
        set_material.label = f"{name}: material"
        set_material.inputs["Material"].default_value = material
        self.links.new(realize.outputs["Geometry"], set_material.inputs["Geometry"])
        self._advance()
        return set_material.outputs["Geometry"]

    def curve_tube(
        self,
        name: str,
        *,
        start: tuple[float, float, float],
        end: tuple[float, float, float],
        radius: float,
        material: bpy.types.Material,
    ) -> bpy.types.NodeSocket:
        line = new_node(self.nodes, "GeometryNodeCurvePrimitiveLine", location=self._location(-1450))
        line.label = f"{name}: curve path"
        line.mode = "POINTS"
        line.inputs["Start"].default_value = start
        line.inputs["End"].default_value = end

        profile = new_node(self.nodes, "GeometryNodeCurvePrimitiveCircle", location=self._location(-1450))
        profile.label = f"{name}: tube profile"
        profile.inputs["Resolution"].default_value = 12
        profile.inputs["Radius"].default_value = radius

        to_mesh = new_node(self.nodes, "GeometryNodeCurveToMesh", location=self._location(-1180))
        to_mesh.label = f"{name}: curve to mesh"
        to_mesh.inputs["Fill Caps"].default_value = True
        self.links.new(line.outputs["Curve"], to_mesh.inputs["Curve"])
        self.links.new(profile.outputs["Curve"], to_mesh.inputs["Profile Curve"])

        set_material = new_node(self.nodes, "GeometryNodeSetMaterial", location=self._location(-900))
        set_material.label = f"{name}: material"
        set_material.inputs["Material"].default_value = material
        self.links.new(to_mesh.outputs["Mesh"], set_material.inputs["Geometry"])
        self._advance()
        return set_material.outputs["Geometry"]

    def arched_shell_surface(
        self,
        name: str,
        *,
        half_width: float,
        length: float,
        y_mid: float,
        knee_height: float,
        crown_height: float,
        material: bpy.types.Material,
    ) -> bpy.types.NodeSocket:
        grid = new_node(self.nodes, "GeometryNodeMeshGrid", location=self._location(-1900))
        grid.label = f"{name}: cross-section grid"
        grid.inputs["Size X"].default_value = half_width * 2.0
        grid.inputs["Size Y"].default_value = length
        grid.inputs["Vertices X"].default_value = 32
        grid.inputs["Vertices Y"].default_value = 32

        position = new_node(self.nodes, "GeometryNodeInputPosition", location=self._location(-1900))
        separate = new_node(self.nodes, "ShaderNodeSeparateXYZ", location=self._location(-1650))
        self.links.new(position.outputs["Position"], separate.inputs["Vector"])

        divide = new_node(self.nodes, "ShaderNodeMath", location=self._location(-1420))
        divide.label = "x / half_width"
        divide.operation = "DIVIDE"
        divide.inputs[1].default_value = half_width
        self.links.new(separate.outputs["X"], divide.inputs[0])

        square = new_node(self.nodes, "ShaderNodeMath", location=self._location(-1220))
        square.label = "(x / half_width)^2"
        square.operation = "POWER"
        square.inputs[1].default_value = 2.0
        self.links.new(divide.outputs["Value"], square.inputs[0])

        invert = new_node(self.nodes, "ShaderNodeMath", location=self._location(-1020))
        invert.label = "1 - normalized square"
        invert.operation = "SUBTRACT"
        invert.inputs[0].default_value = 1.0
        self.links.new(square.outputs["Value"], invert.inputs[1])

        root = new_node(self.nodes, "ShaderNodeMath", location=self._location(-820))
        root.label = "arch profile"
        root.operation = "SQRT"
        self.links.new(invert.outputs["Value"], root.inputs[0])

        scale_height = new_node(self.nodes, "ShaderNodeMath", location=self._location(-620))
        scale_height.label = "profile height"
        scale_height.operation = "MULTIPLY"
        scale_height.inputs[1].default_value = crown_height - knee_height
        self.links.new(root.outputs["Value"], scale_height.inputs[0])

        add_knee = new_node(self.nodes, "ShaderNodeMath", location=self._location(-420))
        add_knee.label = "raise above knee walls"
        add_knee.operation = "ADD"
        add_knee.inputs[1].default_value = knee_height
        self.links.new(scale_height.outputs["Value"], add_knee.inputs[0])

        combine = new_node(self.nodes, "ShaderNodeCombineXYZ", location=self._location(-220))
        combine.label = "arched shell position"
        self.links.new(separate.outputs["X"], combine.inputs["X"])
        self.links.new(separate.outputs["Y"], combine.inputs["Y"])
        self.links.new(add_knee.outputs["Value"], combine.inputs["Z"])

        set_position = new_node(self.nodes, "GeometryNodeSetPosition", location=self._location(20))
        set_position.label = f"{name}: set arched positions"
        self.links.new(grid.outputs["Mesh"], set_position.inputs["Geometry"])
        self.links.new(combine.outputs["Vector"], set_position.inputs["Position"])

        shift = new_node(self.nodes, "GeometryNodeTransform", location=self._location(260))
        shift.label = f"{name}: shift into lane"
        shift.inputs["Translation"].default_value = (0, y_mid, 0)
        self.links.new(set_position.outputs["Geometry"], shift.inputs["Geometry"])

        set_material = new_node(self.nodes, "GeometryNodeSetMaterial", location=self._location(500))
        set_material.label = f"{name}: material"
        set_material.inputs["Material"].default_value = material
        self.links.new(shift.outputs["Geometry"], set_material.inputs["Geometry"])
        self._advance()
        return set_material.outputs["Geometry"]


def module_params(plan: dict[str, Any], module_id: str) -> dict[str, Any]:
    for module in plan["geometry_modules"]:
        if module["module_id"] == module_id:
            return module["parameters"]
    raise KeyError(module_id)


def build_service_ring_graph(plan: dict[str, Any], materials: dict[str, bpy.types.Material]) -> bpy.types.GeometryNodeTree:
    ng = create_node_group("GN_Service_Ring_Kappa_Stage")
    graph = GraphBuilder(ng)

    shell = module_params(plan, "curved_service_lane_shell")
    length = float(shell.get("length", 18.0))
    half_width = float(shell.get("half_width", 3.35))
    knee_height = float(shell.get("knee_height", 1.05))
    crown_height = float(shell.get("crown_height", 3.25))
    y_mid = length / 2 - 1.0
    shell_mat = materials["shell_aggregate"]
    manifold_mat = materials["manifold_amber"]
    anchor_mat = materials["anchor_orange"]
    catwalk_mat = materials["supervisor_blue"]
    crawl_mat = materials["crawl_red"]
    support_mat = materials["support_rig_steel_blue"]
    callback_mat = materials["callback_ivory"]

    # Major room surfaces.
    for socket_out in [
        graph.cube("deck", size=(half_width * 2, length, 0.08), translation=(0, y_mid, 0.0), material=shell_mat),
        graph.cube("left_knee_wall", size=(0.18, length, knee_height), translation=(-half_width, y_mid, knee_height / 2), material=shell_mat),
        graph.cube("right_knee_wall", size=(0.18, length, knee_height), translation=(half_width, y_mid, knee_height / 2), material=shell_mat),
        graph.arched_shell_surface("arched_shell_surface", half_width=half_width, length=length, y_mid=y_mid, knee_height=knee_height, crown_height=crown_height, material=shell_mat),
    ]:
        graph.add_to_join(socket_out)

    # Procedural repetition driven by placement lines.
    graph.add_to_join(graph.cube_array("shell_ribs_left", count=18, start=(-half_width + 0.08, -1, 1.0), offset=(0, length / 17, 0), instance_size=(0.08, 0.04, 2.1), material=shell_mat))
    graph.add_to_join(graph.cube_array("shell_ribs_right", count=18, start=(half_width - 0.08, -1, 1.0), offset=(0, length / 17, 0), instance_size=(0.08, 0.04, 2.1), material=shell_mat))
    graph.add_to_join(graph.cube_array("deck_crossplates", count=8, start=(0, -0.2, 0.13), offset=(0, 2.1, 0), instance_size=(half_width * 1.65, 0.08, 0.035), material=shell_mat))
    graph.add_to_join(graph.cube_array("manifold_route_panels", count=7, start=(-2.88, 1.0, 1.4), offset=(0, 1.5, 0.08), instance_size=(0.1, 0.72, 0.48), material=manifold_mat))
    graph.add_to_join(graph.cube_array("anchor_hook_blocks", count=5, start=(2.9, 0.8, 0.95), offset=(0, 2.6, 0), instance_size=(0.28, 0.18, 0.22), material=anchor_mat))
    graph.add_to_join(graph.cube_array("catwalk_screen_slabs", count=5, start=(2.02, 0.4, 2.4), offset=(0, 1.45, 0), instance_size=(0.05, 0.55, 0.34), material=catwalk_mat))

    # Functional organs and rails.
    for socket_out in [
        graph.cube("manifold_backplane", size=(0.16, 11.8, 2.4), translation=(-3.04, 6.4, 1.35), material=manifold_mat),
        graph.cube("anchor_rail_track", size=(0.16, 12, 0.14), translation=(2.9, 6.0, 0.55), material=anchor_mat),
        graph.cube("catwalk_deck", size=(0.95, 6.5, 0.1), translation=(2.55, 3.2, 1.55), material=catwalk_mat),
        graph.cube("catwalk_wall_mount", size=(0.14, 6.5, 0.34), translation=(3.1, 3.2, 1.73), material=catwalk_mat),
        graph.cube("kappa_crawl_bulkhead", size=(3.35, 0.16, 2.75), translation=(0, 8.4, 1.15), material=crawl_mat),
        graph.cube("kappa_dark_opening_proxy", size=(1.75, 0.08, 1.05), translation=(0, 8.28, 1.15), material=materials["crawl_void_black"] if "crawl_void_black" in materials else crawl_mat),
        graph.cube("teth_service_sled", size=(2.5, 1.6, 0.12), translation=(1.75, 4.7, 0.26), material=support_mat),
        graph.cube("teth_soft_pad", size=(1.8, 1.0, 0.08), translation=(1.75, 4.7, 0.36), material=support_mat),
        graph.cube("callback_parts_tray", size=(1.2, 0.55, 0.08), translation=(1.05, 5.85, 0.72), material=callback_mat),
        graph.cube("callback_limiter_interface", size=(0.28, 0.16, 0.05), translation=(1.27, 5.99, 0.89), material=callback_mat),
    ]:
        graph.add_to_join(socket_out)

    for x in (-1.1, 0.0, 1.1):
        graph.add_to_join(graph.curve_tube(f"overhead_service_spine_{x:+.1f}", start=(x, -1, crown_height - 0.1), end=(x, length - 1, crown_height - 0.1), radius=0.045, material=shell_mat))
    for y in (2.0, 5.0, 8.0, 11.0):
        graph.add_to_join(graph.curve_tube(f"manifold_vertical_pipe_{y:.1f}", start=(-2.86, y, 0.45), end=(-2.86, y, 2.7), radius=0.035, material=manifold_mat))
    for x in (2.08, 3.02):
        graph.add_to_join(graph.curve_tube(f"catwalk_top_rail_{x:.2f}", start=(x, -0.05, 2.2), end=(x, 6.45, 2.2), radius=0.028, material=catwalk_mat))
        graph.add_to_join(graph.curve_tube(f"catwalk_mid_rail_{x:.2f}", start=(x, -0.05, 1.88), end=(x, 6.45, 1.88), radius=0.018, material=catwalk_mat))
    for x in (0.7, 2.8):
        graph.add_to_join(graph.curve_tube(f"teth_longitudinal_rail_{x:.1f}", start=(x, 3.9, 0.5), end=(x, 5.5, 0.5), radius=0.032, material=support_mat))
    for y in (4.22, 5.18):
        graph.add_to_join(graph.curve_tube(f"teth_cross_support_{y:.2f}", start=(0.8, y, 0.57), end=(2.7, y, 0.57), radius=0.026, material=support_mat))
    graph.add_to_join(graph.curve_tube("callback_oxygenation_tube", start=(0.55, 5.95, 0.86), end=(1.55, 5.9, 0.86), radius=0.032, material=materials["teth_blue_gray"]))

    return ng


def add_runtime_materials(materials: dict[str, bpy.types.Material]) -> None:
    material = bpy.data.materials.new("crawl_void_black")
    material.diffuse_color = (0.02, 0.018, 0.014, 1.0)
    materials["crawl_void_black"] = material


def mark_actor_object(obj: bpy.types.Object, actor_id: str) -> bpy.types.Object:
    obj["ghostlight_actor_id"] = actor_id
    return obj


def add_cube_actor_part(name: str, location: tuple[float, float, float], scale: tuple[float, float, float], material: bpy.types.Material, actor_id: str) -> None:
    bpy.ops.mesh.primitive_cube_add(size=1.0, location=location)
    obj = bpy.context.object
    obj.name = name
    obj.dimensions = scale
    bpy.ops.object.transform_apply(location=False, rotation=False, scale=True)
    obj.data.materials.append(material)
    mark_actor_object(obj, actor_id)


def add_humanoid_proxy(actor: dict[str, Any], material: bpy.types.Material) -> None:
    x, y, z = actor["default_position"]
    sx, sy, sz = actor["scale"]
    bpy.ops.mesh.primitive_uv_sphere_add(segments=24, ring_count=12, location=(x, y, z + sz / 2))
    obj = bpy.context.object
    obj.name = actor["actor_id"]
    obj.scale = (sx, sy, sz / 2)
    obj.rotation_euler = tuple(actor["default_rotation"])
    obj.data.materials.append(material)
    mark_actor_object(obj, actor["actor_id"])


def add_cephalopod_proxy(actor: dict[str, Any], material: bpy.types.Material) -> None:
    x, y, z = actor["default_position"]
    sx, sy, sz = actor["scale"]
    bpy.ops.mesh.primitive_uv_sphere_add(segments=32, ring_count=16, location=(x, y, z + 0.25))
    obj = bpy.context.object
    obj.name = actor["actor_id"]
    obj.scale = (sx * 0.45, sy * 0.45, sz * 0.45)
    obj.data.materials.append(material)
    mark_actor_object(obj, actor["actor_id"])


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
    add_runtime_materials(materials)
    stage = create_stage_object()
    node_group = build_service_ring_graph(plan, materials)
    attach_group_modifier(stage, node_group)
    build_proxy_actors(plan, materials)
    build_cameras(plan)
    add_lights()
    configure_scene(resolution_x, resolution_y)


def save_blend(plan: dict[str, Any], output_blend: Path | None) -> None:
    target = output_blend or resolve_project_path(plan["layout"]["layout_asset"])
    target = target.with_name(f"{target.stem}.geonodes{target.suffix}")
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
        output_ref = output_ref.with_name(f"{output_ref.stem}.geonodes{output_ref.suffix}")
        if output_dir:
            output_ref = output_dir / output_ref.name
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
