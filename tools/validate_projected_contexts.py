"""Validate Ghostlight projected local context artifacts."""

from __future__ import annotations

import argparse
import json
import re
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
DEFAULT_CONTEXTS = sorted((ROOT / "examples" / "projected-contexts").glob("*.json"))
RAW_STATE_LEAK_PATTERNS = [
    "current_activation",
    "plasticity",
    '"mean"',
    "selected_current_activation",
]


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


def validate_text_blocks(items: Any, path: str) -> None:
    require(isinstance(items, list) and items, f"{path} must be a non-empty array")
    for index, item in enumerate(items):
        item_path = f"{path}[{index}]"
        require(isinstance(item, dict), f"{item_path} must be an object")
        require_keys(item, ["text", "source_refs"], item_path)
        require(isinstance(item["text"], str) and item["text"].strip(), f"{item_path}.text must be non-empty")
        require(isinstance(item["source_refs"], list) and item["source_refs"], f"{item_path}.source_refs must be non-empty")


def validate_retrieval_requirements(items: Any, path: str) -> None:
    require(isinstance(items, list), f"{path} must be an array")
    for index, item in enumerate(items):
        item_path = f"{path}[{index}]"
        require(isinstance(item, dict), f"{item_path} must be an object")
        require_keys(item, ["requirement_id", "trigger", "compact_fact", "source_refs", "prompt_role"], item_path)
        require(isinstance(item["requirement_id"], str) and item["requirement_id"].strip(), f"{item_path}.requirement_id must be non-empty")
        require(isinstance(item["trigger"], str) and item["trigger"].strip(), f"{item_path}.trigger must be non-empty")
        require(isinstance(item["compact_fact"], str) and item["compact_fact"].strip(), f"{item_path}.compact_fact must be non-empty")
        require(isinstance(item["source_refs"], list) and item["source_refs"], f"{item_path}.source_refs must be non-empty")
        require(item["prompt_role"] in {"source_excerpt", "setting_constraint", "action_affordance", "do_not_invent"}, f"{item_path}.prompt_role is invalid")


def validate_latent_pressure_requirements(items: Any, path: str) -> None:
    require(isinstance(items, list), f"{path} must be an array")
    for index, item in enumerate(items):
        item_path = f"{path}[{index}]"
        require(isinstance(item, dict), f"{item_path} must be an object")
        require_keys(item, ["pressure_id", "text", "source_refs", "projection_rule", "review_rule"], item_path)
        for key in ["pressure_id", "text", "projection_rule", "review_rule"]:
            require(isinstance(item[key], str) and item[key].strip(), f"{item_path}.{key} must be non-empty")
        require(isinstance(item["source_refs"], list) and item["source_refs"], f"{item_path}.source_refs must be non-empty")
        require(
            "speech" in item["review_rule"].lower() or "spoken" in item["review_rule"].lower(),
            f"{item_path}.review_rule must say how speech should be reviewed",
        )


def validate_context(path: Path) -> None:
    data = load_json(path)
    require(isinstance(data, dict), f"{path} must be an object")
    require_keys(data, ["schema_version", "projection_id", "input", "context", "audit"], str(path))
    require(data["schema_version"] == "ghostlight.projected_local_context.v0", f"{path}.schema_version is wrong")

    input_ref = data["input"]
    require_keys(input_ref, ["fixture_ref", "annotation_ref", "scene_id", "local_agent_id", "listener_ids", "projection_mode"], f"{path}.input")
    require((ROOT / input_ref["fixture_ref"]).exists(), f"{path}.input.fixture_ref does not exist")
    require((ROOT / input_ref["annotation_ref"]).exists(), f"{path}.input.annotation_ref does not exist")

    context = data["context"]
    require_keys(
        context,
        [
            "local_agent_id",
            "identity",
            "setting",
            "known_facts",
            "current_stakes",
            "embodiment_and_interface",
            "active_inner_pressures",
            "relationship_read",
            "tensions",
            "runtime_retrieval_requirements",
            "latent_pressure_requirements",
            "retrieved_lore_prompt_blocks",
            "latent_pressure_prompt_blocks",
            "action_affordances",
            "projection_controls",
            "voice_surface",
            "likely_response_moves",
            "do_not_invent",
            "prompt_text",
        ],
        f"{path}.context",
    )
    require(context["local_agent_id"] == input_ref["local_agent_id"], f"{path}.context.local_agent_id mismatches input")
    for key in [
        "known_facts",
        "current_stakes",
        "embodiment_and_interface",
        "active_inner_pressures",
        "tensions",
        "retrieved_lore_prompt_blocks",
        "latent_pressure_prompt_blocks",
        "action_affordances",
        "voice_surface",
        "likely_response_moves",
        "do_not_invent",
    ]:
        validate_text_blocks(context[key], f"{path}.context.{key}")
    require(isinstance(context["relationship_read"], list), f"{path}.context.relationship_read must be an array")
    for index, item in enumerate(context["relationship_read"]):
        validate_text_blocks([item], f"{path}.context.relationship_read[{index}]")
    validate_retrieval_requirements(context["runtime_retrieval_requirements"], f"{path}.context.runtime_retrieval_requirements")
    validate_latent_pressure_requirements(context["latent_pressure_requirements"], f"{path}.context.latent_pressure_requirements")

    prompt_text = context["prompt_text"]
    require(isinstance(prompt_text, str) and prompt_text.strip(), f"{path}.context.prompt_text must be non-empty")
    for pattern in RAW_STATE_LEAK_PATTERNS:
        require(pattern not in prompt_text, f"{path}.context.prompt_text leaks raw state marker: {pattern}")
    require(not re.search(r"\b0\.\d+\b", prompt_text), f"{path}.context.prompt_text appears to contain raw decimal state values")

    audit = data["audit"]
    require_keys(audit, ["projector", "raw_state_hidden_from_prompt", "character_local_context_only", "omitted_private_context", "review_status"], f"{path}.audit")
    require(audit["raw_state_hidden_from_prompt"] is True, f"{path}.audit.raw_state_hidden_from_prompt must be true")
    require(audit["character_local_context_only"] is True, f"{path}.audit.character_local_context_only must be true")
    require(audit["review_status"] in {"draft", "accepted", "needs_revision", "rejected"}, f"{path}.audit.review_status is invalid")
    print(f"ok: {path}")


def main() -> int:
    parser = argparse.ArgumentParser(description="Validate Ghostlight projected local context artifacts.")
    parser.add_argument("paths", nargs="*", type=Path)
    args = parser.parse_args()

    paths = args.paths or DEFAULT_CONTEXTS
    require(paths, "no projected local context artifacts found")
    for path in paths:
        validate_context(path)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
