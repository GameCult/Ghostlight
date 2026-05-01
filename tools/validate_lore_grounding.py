"""Validate Ghostlight lore grounding digest examples.

This focused validator checks the structural invariants needed before lore
digests can feed grounded projection fixtures.
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
DEFAULT_EXAMPLES = sorted((ROOT / "examples" / "lore-grounding").glob("*.json"))


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


def validate_sourced_text(obj: Any, path: str) -> None:
    require(isinstance(obj, dict), f"{path} must be an object")
    require_keys(obj, ["text", "source_refs"], path)
    require_nonempty_string(obj["text"], f"{path}.text")
    require_string_list(obj["source_refs"], f"{path}.source_refs")


def validate_sourced_text_list(value: Any, path: str) -> None:
    require(isinstance(value, list), f"{path} must be an array")
    for index, item in enumerate(value):
        validate_sourced_text(item, f"{path}[{index}]")


def validate_digest(document: dict[str, Any], source: Path) -> None:
    require(document.get("schema_version") == "ghostlight.lore_grounding_digest.v0", f"{source} has wrong schema_version")
    require_keys(
        document,
        [
            "digest_id",
            "source_status",
            "fixture_status",
            "source_refs",
            "historical_context",
            "groups",
            "cultural_pressure_fields",
            "factional_misread_matrix",
            "role_pressure_fields",
            "speaker_lore_boundaries",
            "author_only_boundaries",
            "open_questions",
        ],
        str(source),
    )
    require(document["source_status"] in {"template", "draft", "reviewed", "superseded"}, f"{source}.source_status is invalid")
    require(document["fixture_status"] in {"historical_grounded", "transition_grounded", "procedural_branch", "promoted_branch"}, f"{source}.fixture_status is invalid")

    source_ids: set[str] = set()
    for index, ref in enumerate(document["source_refs"]):
        path = f"{source}.source_refs[{index}]"
        require_keys(ref, ["source_id", "path", "relevance"], path)
        require_nonempty_string(ref["source_id"], f"{path}.source_id")
        require_nonempty_string(ref["path"], f"{path}.path")
        require_nonempty_string(ref["relevance"], f"{path}.relevance")
        source_ids.add(ref["source_id"])
    require(source_ids, f"{source}.source_refs must not be empty")

    context = document["historical_context"]
    require_keys(context, ["time_period", "location", "summary", "public_facts", "ambient_pressures"], f"{source}.historical_context")
    for key in ["time_period", "location", "summary"]:
        require_nonempty_string(context[key], f"{source}.historical_context.{key}")
    validate_sourced_text_list(context["public_facts"], f"{source}.historical_context.public_facts")
    validate_sourced_text_list(context["ambient_pressures"], f"{source}.historical_context.ambient_pressures")

    group_ids: set[str] = set()
    for index, group in enumerate(document["groups"]):
        path = f"{source}.groups[{index}]"
        require_keys(
            group,
            [
                "group_id",
                "label",
                "group_type",
                "institutional_logic",
                "material_incentives",
                "internal_virtues",
                "internal_shames",
                "prestige_markers",
                "taboo_triggers",
                "emotional_display_norms",
                "trust_defaults",
                "speech_register_cues",
                "source_refs",
            ],
            path,
        )
        require_nonempty_string(group["group_id"], f"{path}.group_id")
        group_ids.add(group["group_id"])
        for key in [
            "institutional_logic",
            "material_incentives",
            "internal_virtues",
            "internal_shames",
            "prestige_markers",
            "taboo_triggers",
            "emotional_display_norms",
            "trust_defaults",
            "speech_register_cues",
            "source_refs",
        ]:
            require_string_list(group[key], f"{path}.{key}")
    require(group_ids, f"{source}.groups must not be empty")

    for index, field in enumerate(document["cultural_pressure_fields"]):
        path = f"{source}.cultural_pressure_fields[{index}]"
        require_keys(field, ["field_id", "applies_to", "pressure_type", "summary", "projection_use", "source_refs"], path)
        require_string_list(field["applies_to"], f"{path}.applies_to")
        for group_id in field["applies_to"]:
            require(group_id in group_ids, f"{path}.applies_to references unknown group: {group_id}")
        require_nonempty_string(field["summary"], f"{path}.summary")
        require_nonempty_string(field["projection_use"], f"{path}.projection_use")
        require_string_list(field["source_refs"], f"{path}.source_refs")

    for index, misread in enumerate(document["factional_misread_matrix"]):
        path = f"{source}.factional_misread_matrix[{index}]"
        require_keys(
            misread,
            [
                "observer_group_id",
                "target_group_id",
                "observer_read",
                "target_self_read",
                "misread_patterns",
                "likely_insults",
                "projection_use",
                "source_refs",
            ],
            path,
        )
        for key in ["observer_group_id", "target_group_id"]:
            require(misread[key] in group_ids, f"{path}.{key} references unknown group: {misread[key]}")
        for key in ["observer_read", "target_self_read", "misread_patterns", "likely_insults", "source_refs"]:
            require_string_list(misread[key], f"{path}.{key}")
        require_nonempty_string(misread["projection_use"], f"{path}.projection_use")

    for index, role in enumerate(document["role_pressure_fields"]):
        path = f"{source}.role_pressure_fields[{index}]"
        require_keys(role, ["role_id", "summary", "obligations", "available_masks", "failure_modes", "source_refs"], path)
        require_nonempty_string(role["role_id"], f"{path}.role_id")
        require_nonempty_string(role["summary"], f"{path}.summary")
        for key in ["obligations", "available_masks", "failure_modes", "source_refs"]:
            require_string_list(role[key], f"{path}.{key}")

    for index, boundary in enumerate(document["speaker_lore_boundaries"]):
        path = f"{source}.speaker_lore_boundaries[{index}]"
        require_keys(boundary, ["speaker_id", "known_context", "beliefs", "misreads", "must_not_know"], path)
        require_nonempty_string(boundary["speaker_id"], f"{path}.speaker_id")
        for key in ["known_context", "beliefs", "misreads", "must_not_know"]:
            validate_sourced_text_list(boundary[key], f"{path}.{key}")

    validate_sourced_text_list(document["author_only_boundaries"], f"{source}.author_only_boundaries")
    for index, question in enumerate(document["open_questions"]):
        path = f"{source}.open_questions[{index}]"
        require_keys(question, ["question", "blocks_training", "notes"], path)
        require_nonempty_string(question["question"], f"{path}.question")
        require(isinstance(question["blocks_training"], bool), f"{path}.blocks_training must be boolean")
        require_nonempty_string(question["notes"], f"{path}.notes")


def main() -> int:
    parser = argparse.ArgumentParser(description="Validate Ghostlight lore grounding digest examples.")
    parser.add_argument("paths", nargs="*", type=Path, help="Lore grounding digest JSON files to validate.")
    args = parser.parse_args()

    paths = args.paths or DEFAULT_EXAMPLES
    for path in paths:
        validate_digest(load_json(path), path)
        print(f"ok: {path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
