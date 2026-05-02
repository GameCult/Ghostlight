"""Validate Ghostlight responder output captures."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
DEFAULT_CAPTURES = sorted((ROOT / "experiments" / "responder-packets").glob("*.capture.json"))
REVIEW_STATUSES = {"draft", "accepted", "accepted_as_draft", "useful_needs_revision", "rejected", "superseded"}
FIXTURE_LANES = {"historical_grounded", "transition_grounded", "future_branch"}
CANON_STATUSES = {"single_history", "branch_local", "promoted_branch", "template", "draft"}
ISOLATION_METHODS = {"subagent_no_fork", "manual_packet_only", "external_model_packet_only"}
ACTION_LABELS = {
    "speak",
    "silence",
    "gesture",
    "movement",
    "inspect_object",
    "show_object",
    "withhold_object",
    "transfer_object",
    "set_condition",
    "refuse",
    "wait",
    "withdraw",
    "attack",
}
LEAK_MARKERS = [
    "current_activation",
    "plasticity",
    "canonical_state",
    "coordinator rationale",
    "author-only",
    "future branch",
]


class ValidationError(Exception):
    pass


def load_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8-sig"))


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValidationError(message)


def require_keys(obj: dict[str, Any], keys: list[str], path: str) -> None:
    missing = [key for key in keys if key not in obj]
    require(not missing, f"{path} missing required keys: {', '.join(missing)}")


def require_string(value: Any, path: str) -> None:
    require(isinstance(value, str) and value.strip(), f"{path} must be a non-empty string")


def require_string_array(value: Any, path: str, *, nonempty: bool = False) -> None:
    require(isinstance(value, list), f"{path} must be an array")
    require(not nonempty or bool(value), f"{path} must not be empty")
    for index, item in enumerate(value):
        require_string(item, f"{path}[{index}]")


def validate_parsed_response(value: Any, path: str) -> None:
    require(isinstance(value, dict), f"{path} must be an object")
    require_keys(
        value,
        [
            "responder_agent_id",
            "action_label",
            "visible_action",
            "spoken_text",
            "private_interpretation",
            "intended_effect",
            "state_update_candidates",
            "relationship_update_candidates",
            "unresolved_hooks",
        ],
        path,
    )
    for key in ["responder_agent_id", "action_label", "visible_action", "private_interpretation", "intended_effect"]:
        require_string(value[key], f"{path}.{key}")
    require(isinstance(value["spoken_text"], str), f"{path}.spoken_text must be a string")
    require(value["action_label"] in ACTION_LABELS, f"{path}.action_label is invalid: {value['action_label']}")
    for key in ["state_update_candidates", "relationship_update_candidates", "unresolved_hooks"]:
        require_string_array(value[key], f"{path}.{key}")


def validate_capture(document: dict[str, Any], source: Path) -> None:
    base = str(source)
    require(document.get("schema_version") == "ghostlight.responder_output.v0", f"{base}.schema_version is wrong")
    require_keys(
        document,
        [
            "capture_id",
            "review_status",
            "fixture_lane",
            "canon_status",
            "packet_ref",
            "coordinator_artifact_ref",
            "responder_agent_id",
            "isolation",
            "raw_output",
            "parsed_output",
            "leakage_audit",
            "coordinator_review",
        ],
        base,
    )
    require(document["review_status"] in REVIEW_STATUSES, f"{base}.review_status is invalid")
    require(document["fixture_lane"] in FIXTURE_LANES, f"{base}.fixture_lane is invalid")
    require(document["canon_status"] in CANON_STATUSES, f"{base}.canon_status is invalid")
    for key in ["capture_id", "packet_ref", "coordinator_artifact_ref", "responder_agent_id", "raw_output"]:
        require_string(document[key], f"{base}.{key}")
    require((ROOT / document["packet_ref"]).exists(), f"{base}.packet_ref does not exist")
    require((ROOT / document["coordinator_artifact_ref"]).exists(), f"{base}.coordinator_artifact_ref does not exist")

    isolation = document["isolation"]
    require_keys(isolation, ["isolation_method", "fork_context", "visible_input_ref", "hidden_context_refs", "worker_model_family"], f"{base}.isolation")
    require(isolation["isolation_method"] in ISOLATION_METHODS, f"{base}.isolation.isolation_method is invalid")
    require(isolation["fork_context"] is False, f"{base}.isolation.fork_context must be false for sandbox captures")
    require_string(isolation["visible_input_ref"], f"{base}.isolation.visible_input_ref")
    require_string(isolation["worker_model_family"], f"{base}.isolation.worker_model_family")
    require_string_array(isolation["hidden_context_refs"], f"{base}.isolation.hidden_context_refs")

    raw_output = document["raw_output"]
    parsed_raw = json.loads(raw_output)
    validate_parsed_response(document["parsed_output"], f"{base}.parsed_output")
    require(parsed_raw == document["parsed_output"], f"{base}.raw_output does not match parsed_output")
    require(document["parsed_output"]["responder_agent_id"] == document["responder_agent_id"], f"{base}.parsed_output.responder_agent_id mismatch")

    lowered_raw = raw_output.lower()
    leaked = [marker for marker in LEAK_MARKERS if marker in lowered_raw]
    require(not leaked, f"{base}.raw_output leaks marker(s): {', '.join(leaked)}")

    audit = document["leakage_audit"]
    require_keys(
        audit,
        [
            "private_state_leak",
            "author_only_leak",
            "future_branch_leak",
            "raw_state_soup",
            "prompt_parroting",
            "body_or_object_omniscience",
            "notes",
        ],
        f"{base}.leakage_audit",
    )
    for key in ["private_state_leak", "author_only_leak", "future_branch_leak", "raw_state_soup", "prompt_parroting", "body_or_object_omniscience"]:
        require(isinstance(audit[key], bool), f"{base}.leakage_audit.{key} must be boolean")
    require_string_array(audit["notes"], f"{base}.leakage_audit.notes")
    require(not any(audit[key] for key in ["private_state_leak", "author_only_leak", "future_branch_leak", "raw_state_soup", "prompt_parroting", "body_or_object_omniscience"]), f"{base}.leakage_audit has leak flags set")

    review = document["coordinator_review"]
    require_keys(review, ["reviewer", "accepted_for_training", "review_notes", "coordinator_interventions", "failure_labels"], f"{base}.coordinator_review")
    require_string(review["reviewer"], f"{base}.coordinator_review.reviewer")
    require(isinstance(review["accepted_for_training"], bool), f"{base}.coordinator_review.accepted_for_training must be boolean")
    require(isinstance(review["review_notes"], list), f"{base}.coordinator_review.review_notes must be an array")
    require(isinstance(review["coordinator_interventions"], list), f"{base}.coordinator_review.coordinator_interventions must be an array")
    require_string_array(review["failure_labels"], f"{base}.coordinator_review.failure_labels")


def main() -> int:
    parser = argparse.ArgumentParser(description="Validate Ghostlight responder output captures.")
    parser.add_argument("paths", nargs="*", type=Path, help="Responder output capture files to validate.")
    args = parser.parse_args()

    paths = args.paths or DEFAULT_CAPTURES
    require(paths, "no responder output captures found")
    for path in paths:
        validate_capture(load_json(path), path)
        print(f"ok: {path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
