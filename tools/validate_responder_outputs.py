"""Validate Ghostlight responder output captures."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
DEFAULT_CAPTURES = sorted((ROOT / "experiments" / "responder-packets").glob("*.capture.json"))
DEFAULT_MUTATIONS = sorted((ROOT / "experiments" / "responder-packets").glob("*.mutation.json"))
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


def require_update_candidate_array(value: Any, path: str) -> None:
    require(isinstance(value, list), f"{path} must be an array")
    for index, item in enumerate(value):
        item_path = f"{path}[{index}]"
        if isinstance(item, str):
            require_string(item, item_path)
            continue
        require(isinstance(item, dict), f"{item_path} must be a string or object")
        require("value" in item, f"{item_path} object must include value")
        require_string(item["value"], f"{item_path}.value")
        if "key" in item:
            require_string(item["key"], f"{item_path}.key")
        if "target_agent_id" in item:
            require_string(item["target_agent_id"], f"{item_path}.target_agent_id")


def require_research_trace(value: Any, path: str) -> None:
    require(isinstance(value, list), f"{path} must be an array")
    require(value, f"{path} must not be empty")
    seen_ids: set[str] = set()
    for index, item in enumerate(value):
        item_path = f"{path}[{index}]"
        require(isinstance(item, dict), f"{item_path} must be an object")
        require_keys(
            item,
            [
                "trace_id",
                "origin",
                "query",
                "source_ref",
                "source_path",
                "chunk_index",
                "line_start",
                "line_end",
                "extracted_constraint",
                "used_for",
            ],
            item_path,
        )
        require_string(item["trace_id"], f"{item_path}.trace_id")
        require(item["trace_id"] not in seen_ids, f"{item_path}.trace_id is duplicated")
        seen_ids.add(item["trace_id"])
        require(
            item["origin"] in {"responder_reported", "coordinator_reconstructed", "runner_captured"},
            f"{item_path}.origin is invalid",
        )
        for key in ["query", "source_ref", "source_path", "extracted_constraint", "used_for"]:
            require_string(item[key], f"{item_path}.{key}")
        for key in ["chunk_index", "line_start", "line_end"]:
            require(isinstance(item[key], int), f"{item_path}.{key} must be an integer")
        require(item["chunk_index"] >= 0, f"{item_path}.chunk_index must be non-negative")
        require(item["line_start"] >= 1, f"{item_path}.line_start must be positive")
        require(item["line_end"] >= item["line_start"], f"{item_path}.line_end must be >= line_start")


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
    for key in ["state_update_candidates", "relationship_update_candidates"]:
        require_update_candidate_array(value[key], f"{path}.{key}")
    require_string_array(value["unresolved_hooks"], f"{path}.unresolved_hooks")
    if "consulted_refs" in value:
        require_string_array(value["consulted_refs"], f"{path}.consulted_refs")
    if "followed_refs" in value:
        require_string_array(value["followed_refs"], f"{path}.followed_refs")
    if "research_summary" in value:
        require(isinstance(value["research_summary"], str), f"{path}.research_summary must be a string")
    if "research_trace" in value:
        require_research_trace(value["research_trace"], f"{path}.research_trace")


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
            "generation_lane",
            "lore_access",
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
    require(document["generation_lane"] in {"packet_only", "retrieval_augmented"}, f"{base}.generation_lane is invalid")
    require((ROOT / document["packet_ref"]).exists(), f"{base}.packet_ref does not exist")
    require((ROOT / document["coordinator_artifact_ref"]).exists(), f"{base}.coordinator_artifact_ref does not exist")

    lore_access = document["lore_access"]
    require_keys(lore_access, ["mode", "consulted_refs"], f"{base}.lore_access")
    require(
        lore_access["mode"]
        in {
            "curated_excerpts_only",
            "coordinator_scoped_retrieval",
            "responder_scoped_repository_search",
        },
        f"{base}.lore_access.mode is invalid",
    )
    require_string_array(lore_access["consulted_refs"], f"{base}.lore_access.consulted_refs")
    if document["generation_lane"] == "packet_only":
        require(lore_access["mode"] == "curated_excerpts_only", f"{base}.packet_only must use curated_excerpts_only lore access")
    if document["generation_lane"] == "retrieval_augmented":
        require(
            lore_access["mode"]
            in {"coordinator_scoped_retrieval", "responder_scoped_repository_search"},
            f"{base}.retrieval_augmented must use scoped retrieval lore access",
        )
    if lore_access["mode"] == "responder_scoped_repository_search":
        require_string_array(lore_access["consulted_refs"], f"{base}.lore_access.consulted_refs", nonempty=True)
        require_string(lore_access.get("research_summary"), f"{base}.lore_access.research_summary")
        if "followed_refs" in lore_access:
            require_string_array(lore_access["followed_refs"], f"{base}.lore_access.followed_refs")
        if "research_trace" in lore_access:
            require_research_trace(lore_access["research_trace"], f"{base}.lore_access.research_trace")
            require(
                lore_access.get("research_trace_status") in {"responder_reported", "coordinator_reconstructed", "runner_captured"},
                f"{base}.lore_access.research_trace_status is invalid",
            )
        elif document["review_status"] == "accepted":
            raise ValidationError(f"{base}.lore_access.research_trace is required for accepted research-enabled captures")

    isolation = document["isolation"]
    require_keys(isolation, ["isolation_method", "fork_context", "visible_input_ref", "hidden_context_refs", "worker_model_family"], f"{base}.isolation")
    require(isolation["isolation_method"] in ISOLATION_METHODS, f"{base}.isolation.isolation_method is invalid")
    require(isolation["fork_context"] is False, f"{base}.isolation.fork_context must be false for sandbox captures")
    require_string(isolation["visible_input_ref"], f"{base}.isolation.visible_input_ref")
    require_string(isolation["worker_model_family"], f"{base}.isolation.worker_model_family")
    require_string_array(isolation["hidden_context_refs"], f"{base}.isolation.hidden_context_refs")
    if lore_access["mode"] == "responder_scoped_repository_search":
        require(
            "repo" in isolation["visible_input_ref"].lower() or "lore" in isolation["visible_input_ref"].lower(),
            f"{base}.isolation.visible_input_ref should describe scoped lore access",
        )

    raw_output = document["raw_output"]
    parsed_raw = json.loads(raw_output)
    validate_parsed_response(document["parsed_output"], f"{base}.parsed_output")
    require(parsed_raw == document["parsed_output"], f"{base}.raw_output does not match parsed_output")
    require(document["parsed_output"]["responder_agent_id"] == document["responder_agent_id"], f"{base}.parsed_output.responder_agent_id mismatch")
    if lore_access["mode"] == "responder_scoped_repository_search":
        require(
            document["parsed_output"].get("consulted_refs") == lore_access["consulted_refs"],
            f"{base}.parsed_output.consulted_refs must match lore_access.consulted_refs",
        )
        require(
            document["parsed_output"].get("research_summary") == lore_access["research_summary"],
            f"{base}.parsed_output.research_summary must match lore_access.research_summary",
        )
        if "followed_refs" in lore_access:
            require(
                document["parsed_output"].get("followed_refs") == lore_access["followed_refs"],
                f"{base}.parsed_output.followed_refs must match lore_access.followed_refs",
            )
        if "research_trace" in document["parsed_output"]:
            require(
                document["parsed_output"]["research_trace"] == lore_access.get("research_trace"),
                f"{base}.parsed_output.research_trace must match lore_access.research_trace when present",
            )
        if document["review_status"] == "accepted":
            require(
                lore_access.get("research_trace_status") == "runner_captured",
                f"{base}.accepted research-enabled captures must use runner_captured research_trace_status",
            )

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


def validate_mutation_receipt(document: dict[str, Any], source: Path) -> None:
    base = str(source)
    require(document.get("schema_version") == "ghostlight.reviewed_state_mutation.v0", f"{base}.schema_version is wrong")
    require_keys(
        document,
        [
            "mutation_id",
            "source_fixture_ref",
            "source_capture_ref",
            "mutated_fixture_ref",
            "event_id",
            "responder_output_ref",
            "participant_appraisals",
            "observed_action",
            "observed_response",
            "applied_mutations",
            "manual_review_notes",
            "deferred_mutations",
        ],
        base,
    )
    for key in ["mutation_id", "source_fixture_ref", "source_capture_ref", "mutated_fixture_ref", "event_id", "responder_output_ref"]:
        require_string(document[key], f"{base}.{key}")
    require((ROOT / document["source_fixture_ref"]).exists(), f"{base}.source_fixture_ref does not exist")
    require((ROOT / document["source_capture_ref"]).exists(), f"{base}.source_capture_ref does not exist")
    require((ROOT / document["mutated_fixture_ref"]).exists(), f"{base}.mutated_fixture_ref does not exist")
    require(isinstance(document["participant_appraisals"], list) and document["participant_appraisals"], f"{base}.participant_appraisals must be non-empty")
    for index, appraisal in enumerate(document["participant_appraisals"]):
        appraisal_path = f"{base}.participant_appraisals[{index}]"
        require_keys(appraisal, ["participant_agent_id", "private_interpretation", "state_deltas", "relationship_deltas", "next_action_constraints"], appraisal_path)
    require(isinstance(document["applied_mutations"], list) and document["applied_mutations"], f"{base}.applied_mutations must be non-empty")
    for index, mutation in enumerate(document["applied_mutations"]):
        mutation_path = f"{base}.applied_mutations[{index}]"
        require_keys(mutation, ["target", "path", "before", "after"], mutation_path)
        require(isinstance(mutation["before"], (int, float)), f"{mutation_path}.before must be numeric")
        require(isinstance(mutation["after"], (int, float)), f"{mutation_path}.after must be numeric")
        require(0 <= mutation["before"] <= 1, f"{mutation_path}.before must be between 0 and 1")
        require(0 <= mutation["after"] <= 1, f"{mutation_path}.after must be between 0 and 1")


def main() -> int:
    parser = argparse.ArgumentParser(description="Validate Ghostlight responder output captures.")
    parser.add_argument("paths", nargs="*", type=Path, help="Responder output capture files to validate.")
    args = parser.parse_args()

    paths = args.paths or DEFAULT_CAPTURES
    require(paths, "no responder output captures found")
    for path in paths:
        validate_capture(load_json(path), path)
        print(f"ok: {path}")
    for path in DEFAULT_MUTATIONS:
        validate_mutation_receipt(load_json(path), path)
        print(f"ok: {path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
