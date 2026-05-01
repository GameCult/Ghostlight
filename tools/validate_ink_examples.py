"""Validate Ghostlight Ink examples and sidecar training annotations."""

from __future__ import annotations

import argparse
import json
import shutil
import subprocess
import tempfile
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
DEFAULT_INK_FILES = sorted((ROOT / "examples" / "ink").glob("*.ink"))
VALID_ACTION_TYPES = {
    "speak",
    "silence",
    "move",
    "gesture",
    "touch_object",
    "block_object",
    "use_object",
    "show_object",
    "withhold_object",
    "transfer_object",
    "spend_resource",
    "attack",
    "wait",
    "mixed",
}


class ValidationError(Exception):
    pass


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValidationError(message)


def require_keys(obj: dict[str, Any], keys: list[str], path: str) -> None:
    missing = [key for key in keys if key not in obj]
    require(not missing, f"{path} missing required keys: {', '.join(missing)}")


def load_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def npx_command() -> str:
    command = shutil.which("npx.cmd") or shutil.which("npx")
    require(command is not None, "npx is required to compile Ink examples")
    return command


def compile_ink(path: Path) -> None:
    with tempfile.TemporaryDirectory(prefix="ghostlight-ink-") as temp_dir:
        output_path = Path(temp_dir) / f"{path.stem}.json"
        result = subprocess.run(
            [npx_command(), "inkjs-compiler", str(path), "-o", str(output_path)],
            cwd=ROOT,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            check=False,
        )
        require(
            result.returncode == 0,
            f"{path} failed Ink compilation\nSTDOUT:\n{result.stdout}\nSTDERR:\n{result.stderr}",
        )
        require(output_path.exists(), f"{path} did not produce compiled Ink JSON")
        compiled = output_path.read_text(encoding="utf-8-sig")
        require(compiled.strip().startswith("{"), f"{path} compiled output is not JSON")


def validate_text_blocks(items: Any, path: str) -> None:
    require(isinstance(items, list), f"{path} must be an array")
    for index, item in enumerate(items):
        item_path = f"{path}[{index}]"
        require(isinstance(item, dict), f"{item_path} must be an object")
        require_keys(item, ["text", "source_refs"], item_path)
        require(isinstance(item["text"], str) and item["text"].strip(), f"{item_path}.text must be non-empty")
        require(isinstance(item["source_refs"], list) and item["source_refs"], f"{item_path}.source_refs must be non-empty")


def validate_annotation(path: Path, ink_path: Path) -> None:
    data = load_json(path)
    require(isinstance(data, dict), f"{path} must be an object")
    require_keys(
        data,
        [
            "schema_version",
            "annotation_id",
            "ink_ref",
            "source_fixture_ref",
            "scene_id",
            "protagonist_agent_id",
            "npc_agent_ids",
            "local_awareness_summary",
            "projection_controls",
            "branches",
            "mutation_policy",
            "audit",
        ],
        str(path),
    )
    require(data["schema_version"] == "ghostlight.ink_training_annotation.v0", f"{path}.schema_version is wrong")
    require((ROOT / data["ink_ref"]).resolve() == ink_path.resolve(), f"{path}.ink_ref does not match {ink_path}")
    require((ROOT / data["source_fixture_ref"]).exists(), f"{path}.source_fixture_ref does not exist")
    validate_text_blocks(data["local_awareness_summary"], f"{path}.local_awareness_summary")

    controls = data["projection_controls"]
    require_keys(
        controls,
        ["frame_controls", "authority_boundaries", "object_custody", "required_semantics", "forbidden_resolutions"],
        f"{path}.projection_controls",
    )
    for key, items in controls.items():
        validate_text_blocks(items, f"{path}.projection_controls.{key}")

    ink_text = ink_path.read_text(encoding="utf-8")
    branches = data["branches"]
    require(isinstance(branches, list) and branches, f"{path}.branches must be non-empty")
    saw_non_speech = False
    for index, branch in enumerate(branches):
        branch_path = f"{path}.branches[{index}]"
        require_keys(
            branch,
            [
                "branch_id",
                "ink_path",
                "action_type",
                "actor_intent",
                "state_basis",
                "reviewed_consequences",
                "training_hooks",
            ],
            branch_path,
        )
        require(branch["branch_id"] in ink_text, f"{branch_path}.branch_id is not tagged in Ink")
        require(f"=== {branch['ink_path']} ===" in ink_text, f"{branch_path}.ink_path is not a knot in Ink")
        require(branch["action_type"] in VALID_ACTION_TYPES, f"{branch_path}.action_type is invalid")
        saw_non_speech = saw_non_speech or branch["action_type"] != "speak"
        for list_key in ["state_basis", "reviewed_consequences", "training_hooks"]:
            require(isinstance(branch[list_key], list) and branch[list_key], f"{branch_path}.{list_key} must be non-empty")
    require(saw_non_speech, f"{path} must include at least one non-speech branch")

    mutation_policy = data["mutation_policy"]
    require_keys(mutation_policy, ["automated", "manual_review_required"], f"{path}.mutation_policy")
    require(mutation_policy["manual_review_required"], f"{path}.mutation_policy.manual_review_required must not be empty")

    audit = data["audit"]
    require_keys(audit, ["reviewer", "review_status", "review_notes", "checks"], f"{path}.audit")
    require(audit["review_status"] in {"accepted", "rejected", "needs_revision"}, f"{path}.audit.review_status is invalid")
    require_keys(
        audit["checks"],
        [
            "uses_standard_branching_format",
            "actor_local_context_only",
            "includes_non_speech_actions",
            "fuzzy_updates_marked_for_manual_review",
            "aetheria_lore_content_readable",
        ],
        f"{path}.audit.checks",
    )
    for key, value in audit["checks"].items():
        require(isinstance(value, bool), f"{path}.audit.checks.{key} must be boolean")


def validate_ink(path: Path) -> None:
    compile_ink(path)
    annotation_path = path.with_suffix(".training.json")
    require(annotation_path.exists(), f"{path} is missing sidecar annotation {annotation_path.name}")
    validate_annotation(annotation_path, path)
    print(f"ok: {path}")


def main() -> int:
    parser = argparse.ArgumentParser(description="Validate Ghostlight Ink examples.")
    parser.add_argument("paths", nargs="*", type=Path, help="Ink files to validate.")
    args = parser.parse_args()

    paths = args.paths or DEFAULT_INK_FILES
    require(paths, "no Ink examples found")
    for path in paths:
        validate_ink(path)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
