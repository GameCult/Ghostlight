"""Validate Ghostlight visual blockout plan examples."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
DEFAULT_EXAMPLES = sorted((ROOT / "examples" / "visual-blockouts").glob("*.json"))


class ValidationError(Exception):
    pass


def load_json(path: Path) -> Any:
    with path.open("r", encoding="utf-8") as handle:
        return json.load(handle)


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValidationError(message)


def require_keys(obj: dict[str, Any], keys: list[str], path: str) -> None:
    missing = [key for key in keys if key not in obj]
    require(not missing, f"{path} missing required keys: {', '.join(missing)}")


def require_nonempty_string(value: Any, path: str) -> None:
    require(isinstance(value, str) and value.strip(), f"{path} must be a non-empty string")


def require_string_list(value: Any, path: str) -> None:
    require(isinstance(value, list), f"{path} must be an array")
    for index, item in enumerate(value):
        require_nonempty_string(item, f"{path}[{index}]")


def require_vector3(value: Any, path: str) -> None:
    require(isinstance(value, list) and len(value) == 3, f"{path} must be a 3-number array")
    for index, item in enumerate(value):
        require(isinstance(item, (int, float)), f"{path}[{index}] must be a number")


def require_color(value: Any, path: str) -> None:
    require(isinstance(value, list) and len(value) == 4, f"{path} must be a 4-number RGBA array")
    for index, item in enumerate(value):
        require(isinstance(item, (int, float)) and 0 <= item <= 1, f"{path}[{index}] must be 0..1")


def validate_blockout_plan(document: dict[str, Any], source: Path) -> None:
    require(document.get("schema_version") == "ghostlight.visual_blockout_plan.v0", f"{source}.schema_version is wrong")
    require_keys(
        document,
        [
            "blockout_plan_id",
            "fixture_id",
            "visual_plan_ref",
            "purpose",
            "layout",
            "semantic_materials",
            "geometry_modules",
            "proxy_actors",
            "cameras",
            "guide_renders",
            "prompt_use_contract",
        ],
        str(source),
    )
    for key in ["blockout_plan_id", "fixture_id", "visual_plan_ref", "purpose"]:
        require_nonempty_string(document[key], f"{source}.{key}")

    visual_plan = ROOT / document["visual_plan_ref"]
    require(visual_plan.exists(), f"{source}.visual_plan_ref does not exist: {visual_plan}")
    visual_data = load_json(visual_plan)
    visual_scene_ids = {
        scene.get("visual_scene_id")
        for scene in visual_data.get("visual_scene_plan", {}).get("scenes", [])
        if isinstance(scene, dict)
    }
    require(visual_scene_ids, f"{source}.visual_plan_ref has no visual_scene_plan.scenes")

    layout = document["layout"]
    require_keys(layout, ["layout_id", "layout_asset", "coordinate_system", "scale_notes", "source_refs"], f"{source}.layout")
    for key in ["layout_id", "layout_asset", "coordinate_system", "scale_notes"]:
        require_nonempty_string(layout[key], f"{source}.layout.{key}")
    require_string_list(layout["source_refs"], f"{source}.layout.source_refs")

    material_ids: set[str] = set()
    require(isinstance(document["semantic_materials"], list) and document["semantic_materials"], f"{source}.semantic_materials must be non-empty")
    for index, material in enumerate(document["semantic_materials"]):
        path = f"{source}.semantic_materials[{index}]"
        require_keys(material, ["material_id", "display_name", "base_color", "intended_read", "use_in_prompt"], path)
        require_nonempty_string(material["material_id"], f"{path}.material_id")
        require(material["material_id"] not in material_ids, f"{path}.material_id duplicates {material['material_id']}")
        material_ids.add(material["material_id"])
        require_color(material["base_color"], f"{path}.base_color")
        for key in ["display_name", "intended_read", "use_in_prompt"]:
            require_nonempty_string(material[key], f"{path}.{key}")

    module_ids: set[str] = set()
    for index, module in enumerate(document["geometry_modules"]):
        path = f"{source}.geometry_modules[{index}]"
        require_keys(module, ["module_id", "object_class", "script_ref", "geometry_script_tree", "parameters", "material_id", "notes"], path)
        require_nonempty_string(module["module_id"], f"{path}.module_id")
        require(module["module_id"] not in module_ids, f"{path}.module_id duplicates {module['module_id']}")
        module_ids.add(module["module_id"])
        require(module["material_id"] in material_ids, f"{path}.material_id references unknown material {module['material_id']}")
        require(isinstance(module["parameters"], dict), f"{path}.parameters must be an object")
        for key in ["object_class", "script_ref", "geometry_script_tree", "notes"]:
            require_nonempty_string(module[key], f"{path}.{key}")

    for index, actor in enumerate(document["proxy_actors"]):
        path = f"{source}.proxy_actors[{index}]"
        require_keys(actor, ["actor_id", "visual_ref", "proxy_shape", "material_id", "default_position", "default_rotation", "scale", "notes"], path)
        require(actor["material_id"] in material_ids, f"{path}.material_id references unknown material {actor['material_id']}")
        for key in ["actor_id", "visual_ref", "proxy_shape", "notes"]:
            require_nonempty_string(actor[key], f"{path}.{key}")
        require_vector3(actor["default_position"], f"{path}.default_position")
        require_vector3(actor["default_rotation"], f"{path}.default_rotation")
        require_vector3(actor["scale"], f"{path}.scale")

    camera_ids: set[str] = set()
    for index, camera in enumerate(document["cameras"]):
        path = f"{source}.cameras[{index}]"
        require_keys(camera, ["camera_id", "visual_scene_ids", "position", "rotation", "lens_mm", "shot_role", "framing_notes"], path)
        require_nonempty_string(camera["camera_id"], f"{path}.camera_id")
        require(camera["camera_id"] not in camera_ids, f"{path}.camera_id duplicates {camera['camera_id']}")
        camera_ids.add(camera["camera_id"])
        require_string_list(camera["visual_scene_ids"], f"{path}.visual_scene_ids")
        for scene_id in camera["visual_scene_ids"]:
            require(scene_id in visual_scene_ids, f"{path}.visual_scene_ids references unknown visual scene {scene_id}")
        require_vector3(camera["position"], f"{path}.position")
        require_vector3(camera["rotation"], f"{path}.rotation")
        if "target" in camera:
            require_vector3(camera["target"], f"{path}.target")
        require(isinstance(camera["lens_mm"], (int, float)) and camera["lens_mm"] > 0, f"{path}.lens_mm must be positive")
        for key in ["shot_role", "framing_notes"]:
            require_nonempty_string(camera[key], f"{path}.{key}")

    for index, render in enumerate(document["guide_renders"]):
        path = f"{source}.guide_renders[{index}]"
        require_keys(render, ["render_id", "camera_id", "visual_scene_id", "render_modes", "output_refs", "imagegen_reference_use"], path)
        require(render["camera_id"] in camera_ids, f"{path}.camera_id references unknown camera {render['camera_id']}")
        require(render["visual_scene_id"] in visual_scene_ids, f"{path}.visual_scene_id references unknown visual scene {render['visual_scene_id']}")
        for key in ["render_id", "imagegen_reference_use"]:
            require_nonempty_string(render[key], f"{path}.{key}")
        require_string_list(render["render_modes"], f"{path}.render_modes")
        require_string_list(render["output_refs"], f"{path}.output_refs")

    contract = document["prompt_use_contract"]
    require_keys(contract, ["reference_image_role", "must_preserve", "may_reinterpret", "must_not_assume"], f"{source}.prompt_use_contract")
    require_nonempty_string(contract["reference_image_role"], f"{source}.prompt_use_contract.reference_image_role")
    for key in ["must_preserve", "may_reinterpret", "must_not_assume"]:
        require_string_list(contract[key], f"{source}.prompt_use_contract.{key}")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("paths", nargs="*", type=Path, help="Visual blockout plan JSON files to validate.")
    args = parser.parse_args()
    paths = args.paths or DEFAULT_EXAMPLES
    require(bool(paths), "No visual blockout plans found.")
    for path in paths:
        validate_blockout_plan(load_json(path), path)
        print(f"ok: {path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
