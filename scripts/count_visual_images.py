"""Count base images and image variants declared by Ghostlight visual plans."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
DEFAULT_VISUAL_GLOB = "examples/visual/*.visual.json"


def load_json(path: Path) -> dict[str, Any]:
    return json.loads(path.read_text(encoding="utf-8"))


def plural(count: int, singular: str, plural_form: str | None = None) -> str:
    return f"{count} {singular if count == 1 else (plural_form or singular + 's')}"


def count_visual_plan(path: Path) -> dict[str, Any]:
    data = load_json(path)
    if "visual_art_direction" in data:
        raise ValueError(f"{path} uses legacy visual_art_direction; count visual_scene_plan instead")
    scene_plan = data.get("visual_scene_plan", {})
    scenes = scene_plan.get("scenes", [])
    global_branch_modifiers = scene_plan.get("global_branch_image_modifiers", [])
    scene_variant_counts = []
    for scene in scenes:
        modifiers = scene.get("state_image_modifiers", [])
        scene_variant_counts.append(
            {
                "visual_scene_id": scene.get("visual_scene_id", "<missing>"),
                "ink_anchor": scene.get("ink_anchor", "<missing>"),
                "base_images": 1,
                "scene_variants": len(modifiers),
            }
        )

    base_images = len(scenes)
    scene_variants = sum(item["scene_variants"] for item in scene_variant_counts)
    global_variants = len(global_branch_modifiers)

    return {
        "path": path,
        "visual_plan_id": data.get("visual_plan_id", path.stem),
        "base_images": base_images,
        "scene_variants": scene_variants,
        "global_branch_variants": global_variants,
        "total_variants": scene_variants + global_variants,
        "max_exhaustive_renders": base_images + scene_variants + global_variants,
        "scene_variant_counts": scene_variant_counts,
    }


def format_report(results: list[dict[str, Any]], verbose: bool) -> str:
    lines: list[str] = []
    total_base = sum(item["base_images"] for item in results)
    total_scene_variants = sum(item["scene_variants"] for item in results)
    total_global_variants = sum(item["global_branch_variants"] for item in results)
    total_variants = total_scene_variants + total_global_variants

    lines.append("Ghostlight visual image inventory")
    lines.append("")
    for item in results:
        lines.append(f"- {item['path'].as_posix()}")
        lines.append(f"  visual_plan_id: {item['visual_plan_id']}")
        lines.append(f"  base images: {item['base_images']}")
        lines.append(f"  variants: {item['total_variants']}")
        lines.append(f"    scene-local variants: {item['scene_variants']}")
        lines.append(f"    global branch variants: {item['global_branch_variants']}")
        lines.append(f"  max exhaustive renders: {item['max_exhaustive_renders']}")
        if verbose:
            lines.append("  scene breakdown:")
            for scene in item["scene_variant_counts"]:
                lines.append(
                    f"    - {scene['visual_scene_id']} ({scene['ink_anchor']}): "
                    f"{plural(scene['scene_variants'], 'variant')}"
                )
        lines.append("")

    lines.append("Totals")
    lines.append(f"- base images: {total_base}")
    lines.append(f"- variants: {total_variants}")
    lines.append(f"  scene-local variants: {total_scene_variants}")
    lines.append(f"  global branch variants: {total_global_variants}")
    lines.append(f"- max exhaustive renders: {total_base + total_variants}")
    return "\n".join(lines)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("paths", nargs="*", type=Path, help="Visual plan JSON files to count.")
    parser.add_argument("--verbose", "-v", action="store_true", help="Print per-scene variant counts.")
    args = parser.parse_args()

    paths = args.paths or sorted(ROOT.glob(DEFAULT_VISUAL_GLOB))
    if not paths:
        raise SystemExit(f"No visual plans found with {DEFAULT_VISUAL_GLOB}")

    results = [count_visual_plan(path if path.is_absolute() else ROOT / path) for path in paths]
    print(format_report(results, verbose=args.verbose))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
