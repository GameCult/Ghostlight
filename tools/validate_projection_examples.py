"""Validate Ghostlight projection example JSONL files.

This is a focused validator for reviewed projection artifacts. It does not try
to implement full JSON Schema. It checks the invariants that matter before the
projection dataset becomes training material.
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
DEFAULT_EXAMPLES = sorted((ROOT / "examples" / "projections").glob("*.jsonl"))


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


def require_0_1(value: Any, path: str) -> None:
    require(isinstance(value, (int, float)), f"{path} must be numeric")
    require(0 <= value <= 1, f"{path} must be between 0 and 1")


def load_jsonl(path: Path) -> list[dict[str, Any]]:
    records: list[dict[str, Any]] = []
    with path.open("r", encoding="utf-8") as handle:
        for line_number, line in enumerate(handle, start=1):
            stripped = line.strip()
            if not stripped:
                continue
            try:
                record = json.loads(stripped)
            except json.JSONDecodeError as exc:
                raise ValidationError(f"{path}:{line_number} invalid JSONL: {exc}") from exc
            require(isinstance(record, dict), f"{path}:{line_number} record must be an object")
            records.append(record)
    require(records, f"{path} has no projection records")
    return records


def fixture_context(fixture_path: Path) -> dict[str, Any]:
    fixture = load_json(fixture_path)
    agent_ids = {agent["agent_id"] for agent in fixture["agents"]}
    scene_ids = {scene["scene_id"] for scene in fixture["scenes"]}
    return {
        "path": fixture_path,
        "agent_ids": agent_ids,
        "scene_ids": scene_ids,
    }


def validate_sourced_items(items: Any, path: str) -> None:
    require(isinstance(items, list), f"{path} must be an array")
    for index, item in enumerate(items):
        item_path = f"{path}[{index}]"
        require(isinstance(item, dict), f"{item_path} must be an object")
        require_keys(item, ["text", "source_refs"], item_path)
        require(isinstance(item["text"], str) and item["text"].strip(), f"{item_path}.text must be non-empty")
        validate_source_refs(item["source_refs"], f"{item_path}.source_refs")


def validate_source_refs(refs: Any, path: str) -> None:
    require(isinstance(refs, list) and refs, f"{path} must be a non-empty array")
    for index, ref in enumerate(refs):
        require(isinstance(ref, str) and ref.strip(), f"{path}[{index}] must be a non-empty string")


def validate_projection_record(record: dict[str, Any], source: Path, index: int) -> None:
    path = f"{source}[{index}]"
    require_keys(record, ["schema_version", "example_id", "input", "output", "audit"], path)
    require(record["schema_version"] == "ghostlight.projection_example.v0", f"{path}.schema_version is wrong")

    input_ref = record["input"]
    require(isinstance(input_ref, dict), f"{path}.input must be an object")
    require_keys(
        input_ref,
        ["fixture_ref", "scene_id", "speaker_agent_id", "listener_ids", "projection_mode"],
        f"{path}.input",
    )
    require(
        input_ref["projection_mode"] in {"dialogue_turn", "response_turn", "action_turn", "scene_beat", "drama_scaffold"},
        f"{path}.input.projection_mode is invalid",
    )

    fixture_path = (ROOT / input_ref["fixture_ref"]).resolve()
    require(fixture_path.exists(), f"{path}.input.fixture_ref does not exist: {input_ref['fixture_ref']}")
    context = fixture_context(fixture_path)
    require(input_ref["scene_id"] in context["scene_ids"], f"{path}.input.scene_id references unknown scene")
    require(input_ref["speaker_agent_id"] in context["agent_ids"], f"{path}.input.speaker_agent_id references unknown agent")
    require(isinstance(input_ref["listener_ids"], list) and input_ref["listener_ids"], f"{path}.input.listener_ids must be non-empty")
    for listener_id in input_ref["listener_ids"]:
        require(listener_id in context["agent_ids"], f"{path}.input.listener_ids references unknown agent: {listener_id}")
        require(listener_id != input_ref["speaker_agent_id"], f"{path}.input.listener_ids includes speaker")

    output = record["output"]
    require(isinstance(output, dict), f"{path}.output must be an object")
    require_keys(
        output,
        [
            "known_facts",
            "active_pressures",
            "tensions",
            "response_affordances",
            "voice_surface",
            "prompt_text",
        ],
        f"{path}.output",
    )
    validate_sourced_items(output["known_facts"], f"{path}.output.known_facts")
    validate_sourced_items(output["voice_surface"], f"{path}.output.voice_surface")
    if "projection_controls" in output:
        validate_projection_controls(output["projection_controls"], f"{path}.output.projection_controls")
    require(isinstance(output["prompt_text"], str) and output["prompt_text"].strip(), f"{path}.output.prompt_text must be non-empty")

    for pressure_index, pressure in enumerate(output["active_pressures"]):
        pressure_path = f"{path}.output.active_pressures[{pressure_index}]"
        require_keys(pressure, ["pressure_id", "summary", "intensity", "source_refs"], pressure_path)
        require_0_1(pressure["intensity"], f"{pressure_path}.intensity")
        validate_source_refs(pressure["source_refs"], f"{pressure_path}.source_refs")

    for tension_index, tension in enumerate(output["tensions"]):
        tension_path = f"{path}.output.tensions[{tension_index}]"
        require_keys(tension, ["tension_id", "summary", "source_refs"], tension_path)
        validate_source_refs(tension["source_refs"], f"{tension_path}.source_refs")

    for affordance_index, affordance in enumerate(output["response_affordances"]):
        affordance_path = f"{path}.output.response_affordances[{affordance_index}]"
        require_keys(affordance, ["affordance_id", "behavior", "pressure_basis", "source_refs"], affordance_path)
        validate_source_refs(affordance["source_refs"], f"{affordance_path}.source_refs")

    audit = record["audit"]
    require_keys(
        audit,
        ["teacher_model", "reviewer", "review_status", "review_notes", "target_emergent_behaviors", "checks"],
        f"{path}.audit",
    )
    require(audit["review_status"] in {"accepted", "rejected", "needs_revision"}, f"{path}.audit.review_status is invalid")
    require(isinstance(audit["target_emergent_behaviors"], list), f"{path}.audit.target_emergent_behaviors must be an array")
    checks = audit["checks"]
    require_keys(
        checks,
        [
            "speaker_local_context_only",
            "state_interactions_cited",
            "voice_separate_from_defense",
            "secrets_as_pressure_not_forbidden_context",
            "compact_runtime_surface",
        ],
        f"{path}.audit.checks",
    )
    for key, value in checks.items():
        require(isinstance(value, bool), f"{path}.audit.checks.{key} must be boolean")


def validate_projection_controls(controls: Any, path: str) -> None:
    require(isinstance(controls, dict), f"{path} must be an object")
    allowed = {
        "frame_controls",
        "authority_boundaries",
        "object_custody",
        "required_semantics",
        "forbidden_resolutions",
    }
    unknown = sorted(set(controls) - allowed)
    require(not unknown, f"{path} has unknown keys: {', '.join(unknown)}")
    require(controls, f"{path} must not be empty when present")
    for key, value in controls.items():
        validate_sourced_items(value, f"{path}.{key}")


def validate_file(path: Path) -> None:
    records = load_jsonl(path)
    seen_ids: set[str] = set()
    for index, record in enumerate(records):
        validate_projection_record(record, path, index)
        example_id = record["example_id"]
        require(example_id not in seen_ids, f"{path}[{index}].example_id is duplicated: {example_id}")
        seen_ids.add(example_id)
    print(f"ok: {path}")


def main() -> int:
    parser = argparse.ArgumentParser(description="Validate Ghostlight projection example JSONL files.")
    parser.add_argument("paths", nargs="*", type=Path, help="Projection JSONL files to validate.")
    args = parser.parse_args()

    paths = args.paths or DEFAULT_EXAMPLES
    for path in paths:
        validate_file(path)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
