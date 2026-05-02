"""Validate Ghostlight coordinator artifacts.

This is a focused structural validator for the v0 coordinator seam. It checks
the fields that matter before these artifacts become training data: scene refs,
machinery calls, sandboxed responder handoffs, proposed deltas, branch flags,
and review status.
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
DEFAULT_ARTIFACTS = sorted((ROOT / "examples" / "coordinator").glob("*.json"))

REVIEW_STATUSES = {"draft", "accepted", "accepted_as_draft", "useful_needs_revision", "rejected", "superseded"}
FIXTURE_LANES = {"historical_grounded", "transition_grounded", "future_branch"}
CANON_STATUSES = {"single_history", "branch_local", "promoted_branch", "template", "draft"}
ORGANS = {
    "lore_grounding_compiler",
    "memory_lore_retriever",
    "projector",
    "sandboxed_responder",
    "event_resolver",
    "participant_appraiser",
    "state_mutator",
    "relationship_perception_updater",
    "output_evaluator",
    "item_manifest_generator",
    "institution_faction_consumer_model",
    "human_review",
    "other",
}
BRANCH_ACTIONS = {"none", "continue_branch", "split_branch", "merge_branch", "promote_branch_fact"}
EVENT_CONSTRAINT_TYPES = {"branch_attractor", "fated_event", "tech_order_constraint", "quest_injection", "branch_local_event"}
ISOLATION_METHODS = {"subagent_no_fork", "manual_packet_only", "external_model_packet_only", "not_sandboxed"}
DELTA_TYPES = {
    "memory_write",
    "relationship_update",
    "perceived_overlay_update",
    "world_fact_update",
    "object_state_update",
    "resource_delta",
    "branch_flag_update",
    "item_manifest_delta",
    "no_delta",
}
DELTA_AUTHORITIES = {"deterministic_gate", "manual_review", "coordinator_proposal", "blocked"}
DELTA_STATUSES = {"proposed", "accepted", "rejected", "deferred", "blocked"}


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


def validate_text_with_refs(value: Any, path: str) -> None:
    require(isinstance(value, dict), f"{path} must be an object")
    require_keys(value, ["text", "refs"], path)
    require_string(value["text"], f"{path}.text")
    require_string_array(value["refs"], f"{path}.refs", nonempty=True)


def validate_source_ref(value: Any, path: str) -> None:
    require(isinstance(value, dict), f"{path} must be an object")
    require_keys(value, ["ref_id", "path", "purpose"], path)
    for key in ["ref_id", "path", "purpose"]:
        require_string(value[key], f"{path}.{key}")


def validate_invocation(value: Any, path: str) -> None:
    require(isinstance(value, dict), f"{path} must be an object")
    require_keys(value, ["invocation_id", "organ", "purpose", "status"], path)
    require_string(value["invocation_id"], f"{path}.invocation_id")
    require(value["organ"] in ORGANS, f"{path}.organ is invalid: {value['organ']}")
    require_string(value["purpose"], f"{path}.purpose")
    require(value["status"] in REVIEW_STATUSES, f"{path}.status is invalid: {value['status']}")
    if "input_refs" in value:
        require_string_array(value["input_refs"], f"{path}.input_refs")
    if "output_refs" in value:
        require_string_array(value["output_refs"], f"{path}.output_refs")


def validate_sandbox_handoff(value: Any, path: str) -> None:
    require(isinstance(value, dict), f"{path} must be an object")
    require_keys(
        value,
        [
            "handoff_id",
            "responder_agent_id",
            "projected_context_ref",
            "visible_input_ref",
            "hidden_context_refs",
            "isolation_method",
            "raw_output_ref",
            "reviewed_output_ref",
            "coordinator_interventions",
            "leakage_audit",
        ],
        path,
    )
    for key in ["handoff_id", "responder_agent_id", "projected_context_ref", "visible_input_ref", "raw_output_ref", "reviewed_output_ref"]:
        require_string(value[key], f"{path}.{key}")
    require_string_array(value["hidden_context_refs"], f"{path}.hidden_context_refs")
    require(value["isolation_method"] in ISOLATION_METHODS, f"{path}.isolation_method is invalid")
    require(value["isolation_method"] != "not_sandboxed", f"{path}.isolation_method must not be not_sandboxed for training examples")

    interventions = value["coordinator_interventions"]
    require(isinstance(interventions, list), f"{path}.coordinator_interventions must be an array")
    for index, intervention in enumerate(interventions):
        intervention_path = f"{path}.coordinator_interventions[{index}]"
        require_keys(intervention, ["intervention_type", "description"], intervention_path)
        require_string(intervention["description"], f"{intervention_path}.description")

    audit = value["leakage_audit"]
    require_keys(
        audit,
        [
            "private_state_leak",
            "author_only_leak",
            "future_branch_leak",
            "raw_state_soup",
            "prompt_parroting",
            "body_or_object_omniscience",
        ],
        f"{path}.leakage_audit",
    )
    for key in [
        "private_state_leak",
        "author_only_leak",
        "future_branch_leak",
        "raw_state_soup",
        "prompt_parroting",
        "body_or_object_omniscience",
    ]:
        require(isinstance(audit[key], bool), f"{path}.leakage_audit.{key} must be boolean")
    if "notes" in audit:
        require_string_array(audit["notes"], f"{path}.leakage_audit.notes")


def validate_state_ref(value: Any, path: str) -> None:
    require(isinstance(value, dict), f"{path} must be an object")
    require_keys(value, ["ref_id", "ref_type", "path"], path)
    for key in ["ref_id", "ref_type", "path"]:
        require_string(value[key], f"{path}.{key}")


def validate_delta(value: Any, path: str) -> None:
    require(isinstance(value, dict), f"{path} must be an object")
    require_keys(value, ["delta_id", "delta_type", "target_ref", "summary", "authority", "status"], path)
    require_string(value["delta_id"], f"{path}.delta_id")
    require(value["delta_type"] in DELTA_TYPES, f"{path}.delta_type is invalid: {value['delta_type']}")
    require_string(value["target_ref"], f"{path}.target_ref")
    require_string(value["summary"], f"{path}.summary")
    require(value["authority"] in DELTA_AUTHORITIES, f"{path}.authority is invalid: {value['authority']}")
    require(value["status"] in DELTA_STATUSES, f"{path}.status is invalid: {value['status']}")


def validate_artifact(document: dict[str, Any], source: Path) -> None:
    require(document.get("schema_version") == "ghostlight.coordinator_artifact.v0", f"{source}.schema_version is wrong")
    require_keys(
        document,
        [
            "artifact_id",
            "review_status",
            "fixture_lane",
            "canon_status",
            "source_refs",
            "input_refs",
            "scene_context",
            "coordinator_decision",
            "machinery_plan",
            "world_state_refs",
            "proposed_deltas",
            "branching",
            "unresolved_hooks",
            "review",
        ],
        str(source),
    )
    require(document["review_status"] in REVIEW_STATUSES, f"{source}.review_status is invalid")
    require(document["fixture_lane"] in FIXTURE_LANES, f"{source}.fixture_lane is invalid")
    require(document["canon_status"] in CANON_STATUSES, f"{source}.canon_status is invalid")

    require(isinstance(document["source_refs"], list) and document["source_refs"], f"{source}.source_refs must be non-empty")
    for index, ref in enumerate(document["source_refs"]):
        validate_source_ref(ref, f"{source}.source_refs[{index}]")

    input_refs = document["input_refs"]
    require_keys(input_refs, ["agent_state_ref", "lore_grounding_refs"], f"{source}.input_refs")
    require((ROOT / input_refs["agent_state_ref"]).exists(), f"{source}.input_refs.agent_state_ref does not exist")
    require_string_array(input_refs["lore_grounding_refs"], f"{source}.input_refs.lore_grounding_refs")
    for ref in input_refs["lore_grounding_refs"]:
        require((ROOT / ref).exists(), f"{source}.input_refs.lore_grounding_refs missing file: {ref}")

    scene = document["scene_context"]
    require_keys(scene, ["scene_id", "sequence_marker", "location_id", "participant_ids", "active_object_ids", "public_facts", "scene_pressure"], f"{source}.scene_context")
    for key in ["scene_id", "sequence_marker", "location_id"]:
        require_string(scene[key], f"{source}.scene_context.{key}")
    require_string_array(scene["participant_ids"], f"{source}.scene_context.participant_ids", nonempty=True)
    require_string_array(scene["active_object_ids"], f"{source}.scene_context.active_object_ids")
    for key in ["public_facts", "scene_pressure"]:
        require(isinstance(scene[key], list) and scene[key], f"{source}.scene_context.{key} must be non-empty")
        for index, item in enumerate(scene[key]):
            validate_text_with_refs(item, f"{source}.scene_context.{key}[{index}]")

    decision = document["coordinator_decision"]
    require_keys(decision, ["decision_id", "selected_next_beat", "selected_acting_agent_id", "rationale", "glue_prose", "glue_prose_status"], f"{source}.coordinator_decision")
    for key in ["decision_id", "selected_next_beat", "selected_acting_agent_id", "rationale"]:
        require_string(decision[key], f"{source}.coordinator_decision.{key}")
    require(decision["glue_prose_status"] in {"not_canonical", "accepted_as_narration", "needs_revision", "rejected"}, f"{source}.coordinator_decision.glue_prose_status is invalid")

    invocations = document["machinery_plan"].get("invocations")
    require(isinstance(invocations, list), f"{source}.machinery_plan.invocations must be an array")
    for index, invocation in enumerate(invocations):
        validate_invocation(invocation, f"{source}.machinery_plan.invocations[{index}]")

    for index, handoff in enumerate(document.get("sandboxed_responder_handoffs", [])):
        validate_sandbox_handoff(handoff, f"{source}.sandboxed_responder_handoffs[{index}]")

    world_refs = document["world_state_refs"]
    require_keys(world_refs, ["read_refs", "write_candidate_refs"], f"{source}.world_state_refs")
    for key in ["read_refs", "write_candidate_refs"]:
        require(isinstance(world_refs[key], list), f"{source}.world_state_refs.{key} must be an array")
        for index, ref in enumerate(world_refs[key]):
            validate_state_ref(ref, f"{source}.world_state_refs.{key}[{index}]")

    for index, delta in enumerate(document["proposed_deltas"]):
        validate_delta(delta, f"{source}.proposed_deltas[{index}]")

    branching = document["branching"]
    require_keys(branching, ["branch_action", "active_branch_flags", "event_constraints"], f"{source}.branching")
    require(branching["branch_action"] in BRANCH_ACTIONS, f"{source}.branching.branch_action is invalid")
    require_string_array(branching["active_branch_flags"], f"{source}.branching.active_branch_flags")
    require(isinstance(branching["event_constraints"], list), f"{source}.branching.event_constraints must be an array")
    for index, constraint in enumerate(branching["event_constraints"]):
        constraint_path = f"{source}.branching.event_constraints[{index}]"
        require_keys(constraint, ["constraint_id", "constraint_type", "summary"], constraint_path)
        require(constraint["constraint_type"] in EVENT_CONSTRAINT_TYPES, f"{constraint_path}.constraint_type is invalid")
        require_string(constraint["summary"], f"{constraint_path}.summary")

    require(isinstance(document["unresolved_hooks"], list), f"{source}.unresolved_hooks must be an array")
    for index, hook in enumerate(document["unresolved_hooks"]):
        hook_path = f"{source}.unresolved_hooks[{index}]"
        require_keys(hook, ["hook_id", "hook_type", "summary", "owner", "status"], hook_path)
        for key in ["hook_id", "hook_type", "summary", "owner", "status"]:
            require_string(hook[key], f"{hook_path}.{key}")

    review = document["review"]
    require_keys(review, ["reviewer", "review_notes", "accepted_for_training", "failure_labels"], f"{source}.review")
    require_string(review["reviewer"], f"{source}.review.reviewer")
    require_string_array(review["review_notes"], f"{source}.review.review_notes")
    require(isinstance(review["accepted_for_training"], bool), f"{source}.review.accepted_for_training must be boolean")
    require_string_array(review["failure_labels"], f"{source}.review.failure_labels")


def main() -> int:
    parser = argparse.ArgumentParser(description="Validate Ghostlight coordinator artifacts.")
    parser.add_argument("paths", nargs="*", type=Path, help="Coordinator artifact JSON files to validate.")
    args = parser.parse_args()

    paths = args.paths or DEFAULT_ARTIFACTS
    require(paths, "no coordinator artifacts found")
    for path in paths:
        validate_artifact(load_json(path), path)
        print(f"ok: {path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
