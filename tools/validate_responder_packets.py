"""Validate Ghostlight responder packets."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
DEFAULT_PACKETS = sorted((ROOT / "examples" / "responder-packets").glob("*.json"))
REVIEW_STATUSES = {"draft", "accepted", "accepted_as_draft", "useful_needs_revision", "rejected", "superseded"}
FIXTURE_LANES = {"historical_grounded", "transition_grounded", "future_branch"}
CANON_STATUSES = {"single_history", "branch_local", "promoted_branch", "template", "draft"}
ISOLATION_METHODS = {"subagent_no_fork", "manual_packet_only", "external_model_packet_only"}
RAW_STATE_MARKERS = [
    "current_activation",
    "plasticity",
    "canonical_state",
    "raw state variable",
    "raw numeric state",
    "dimension score",
]
HIDDEN_LEAK_MARKERS = [
    "author_only.packet_personhood_resolution",
    "coordinator_decision.rationale",
    "maer_tidecall.canonical_private_state",
    "future_branch_plans",
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


def reject_markers(text: str, markers: list[str], path: str) -> None:
    lowered = text.lower()
    found = [marker for marker in markers if marker.lower() in lowered]
    require(not found, f"{path} leaks forbidden marker(s): {', '.join(found)}")


def validate_packet(document: dict[str, Any], source: Path) -> None:
    base = str(source)
    require(document.get("schema_version") == "ghostlight.responder_packet.v0", f"{base}.schema_version is wrong")
    require_keys(
        document,
        [
            "packet_id",
            "review_status",
            "fixture_lane",
            "canon_status",
            "coordinator_artifact_ref",
            "projected_context_ref",
            "responder_agent_id",
            "generation_lane",
            "lore_access",
            "visible_context",
            "output_contract",
            "hidden_context_audit",
            "isolation_requirements",
            "packet_prompt_text",
            "review",
        ],
        base,
    )
    require(document["review_status"] in REVIEW_STATUSES, f"{base}.review_status is invalid")
    require(document["fixture_lane"] in FIXTURE_LANES, f"{base}.fixture_lane is invalid")
    require(document["canon_status"] in CANON_STATUSES, f"{base}.canon_status is invalid")
    for key in ["packet_id", "coordinator_artifact_ref", "projected_context_ref", "responder_agent_id", "packet_prompt_text"]:
        require_string(document[key], f"{base}.{key}")
    require(document["generation_lane"] in {"packet_only", "retrieval_augmented"}, f"{base}.generation_lane is invalid")
    require((ROOT / document["coordinator_artifact_ref"]).exists(), f"{base}.coordinator_artifact_ref does not exist")
    require((ROOT / document["projected_context_ref"]).exists(), f"{base}.projected_context_ref does not exist")

    lore_access = document["lore_access"]
    require_keys(lore_access, ["mode", "allowed_scope", "required_provenance"], f"{base}.lore_access")
    require(
        lore_access["mode"]
        in {
            "curated_excerpts_only",
            "coordinator_scoped_retrieval",
            "responder_scoped_repository_search",
        },
        f"{base}.lore_access.mode is invalid",
    )
    require_string_array(lore_access["allowed_scope"], f"{base}.lore_access.allowed_scope")
    require(lore_access["required_provenance"] is True, f"{base}.lore_access.required_provenance must be true")
    if document["generation_lane"] == "packet_only":
        require(lore_access["mode"] == "curated_excerpts_only", f"{base}.packet_only must use curated_excerpts_only lore access")
    if document["generation_lane"] == "retrieval_augmented":
        require(
            lore_access["mode"]
            in {"coordinator_scoped_retrieval", "responder_scoped_repository_search"},
            f"{base}.retrieval_augmented must use scoped retrieval lore access",
        )

    visible = document["visible_context"]
    require_keys(visible, ["local_context_prompt", "observed_event", "allowed_action_labels", "source_excerpts"], f"{base}.visible_context")
    require_string(visible["local_context_prompt"], f"{base}.visible_context.local_context_prompt")
    require_string_array(visible["allowed_action_labels"], f"{base}.visible_context.allowed_action_labels", nonempty=True)

    event = visible["observed_event"]
    require_keys(event, ["event_id", "actor_agent_id", "observable_action", "spoken_text", "visible_object_refs", "visibility_notes"], f"{base}.visible_context.observed_event")
    for key in ["event_id", "actor_agent_id", "observable_action"]:
        require_string(event[key], f"{base}.visible_context.observed_event.{key}")
    require(isinstance(event["spoken_text"], str), f"{base}.visible_context.observed_event.spoken_text must be a string")
    require_string_array(event["visible_object_refs"], f"{base}.visible_context.observed_event.visible_object_refs")
    require_string_array(event["visibility_notes"], f"{base}.visible_context.observed_event.visibility_notes", nonempty=True)

    excerpts = visible["source_excerpts"]
    require(isinstance(excerpts, list), f"{base}.visible_context.source_excerpts must be an array")
    for index, excerpt in enumerate(excerpts):
        path = f"{base}.visible_context.source_excerpts[{index}]"
        require_keys(excerpt, ["ref_id", "source_ref", "text"], path)
        for key in ["ref_id", "source_ref", "text"]:
            require_string(excerpt[key], f"{path}.{key}")

    contract = document["output_contract"]
    require_keys(contract, ["response_schema", "required_fields", "response_rules"], f"{base}.output_contract")
    require_string(contract["response_schema"], f"{base}.output_contract.response_schema")
    require_string_array(contract["required_fields"], f"{base}.output_contract.required_fields", nonempty=True)
    require_string_array(contract["response_rules"], f"{base}.output_contract.response_rules", nonempty=True)

    audit = document["hidden_context_audit"]
    require_keys(audit, ["omitted_context_refs", "forbidden_inference_classes", "known_absences"], f"{base}.hidden_context_audit")
    for key in ["omitted_context_refs", "forbidden_inference_classes", "known_absences"]:
        require_string_array(audit[key], f"{base}.hidden_context_audit.{key}", nonempty=True)

    isolation = document["isolation_requirements"]
    require_keys(isolation, ["isolation_method", "packet_only", "no_repo_access", "no_conversation_context", "no_hidden_state_refs"], f"{base}.isolation_requirements")
    require(isolation["isolation_method"] in ISOLATION_METHODS, f"{base}.isolation_requirements.isolation_method is invalid")
    for key in ["packet_only", "no_conversation_context", "no_hidden_state_refs"]:
        require(isolation[key] is True, f"{base}.isolation_requirements.{key} must be true")
    require(isinstance(isolation["no_repo_access"], bool), f"{base}.isolation_requirements.no_repo_access must be boolean")

    prompt_text = document["packet_prompt_text"]
    reject_markers(prompt_text, RAW_STATE_MARKERS, f"{base}.packet_prompt_text")
    reject_markers(prompt_text, HIDDEN_LEAK_MARKERS, f"{base}.packet_prompt_text")
    require(document["visible_context"]["local_context_prompt"] in prompt_text, f"{base}.packet_prompt_text must contain local context prompt")
    require(event["observable_action"] in prompt_text, f"{base}.packet_prompt_text must contain observed event")

    review = document["review"]
    require_keys(review, ["reviewer", "review_notes", "accepted_for_sandbox_use", "failure_labels"], f"{base}.review")
    require_string(review["reviewer"], f"{base}.review.reviewer")
    require(isinstance(review["review_notes"], list), f"{base}.review.review_notes must be an array")
    require(isinstance(review["accepted_for_sandbox_use"], bool), f"{base}.review.accepted_for_sandbox_use must be boolean")
    require_string_array(review["failure_labels"], f"{base}.review.failure_labels")


def main() -> int:
    parser = argparse.ArgumentParser(description="Validate Ghostlight responder packets.")
    parser.add_argument("paths", nargs="*", type=Path, help="Responder packet JSON files to validate.")
    args = parser.parse_args()

    paths = args.paths or DEFAULT_PACKETS
    require(paths, "no responder packets found")
    for path in paths:
        validate_packet(load_json(path), path)
        print(f"ok: {path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
