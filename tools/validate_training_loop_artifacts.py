"""Validate Ghostlight training-loop scene receipt bundles.

This validator covers the stateful receipts that sit between sandboxed
responder turns and eventual training examples: event records, participant
appraisals, reviewed mutations, and scene-loop bundle manifests.
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
DEFAULT_ROOT = ROOT / "examples" / "training-loops"

REVIEW_STATUSES = {"draft", "accepted", "accepted_as_draft", "useful_needs_revision", "rejected", "superseded"}
TRAINING_USABILITY = {"training_ready", "reference_only", "not_training_data"}
VISIBILITY_SCOPES = {"public_room", "participant_local", "camera_mediated", "partial_sightline", "coordinator_only"}
INTERPRETATION_LABELS = {"misread", "correct_read", "ambiguous", "strategic_read", "unknown"}
DELTA_STATUSES = {"accepted", "rejected", "deferred", "blocked"}
DELTA_AUTHORITIES = {"manual_review", "coordinator_review", "deterministic_gate", "blocked"}


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


def require_ref(path_value: str, path: str) -> None:
    require_string(path_value, path)
    require((ROOT / path_value).exists(), f"{path} does not exist: {path_value}")


def validate_text_ref(value: Any, path: str) -> None:
    require(isinstance(value, dict), f"{path} must be an object")
    require_keys(value, ["text", "refs"], path)
    require_string(value["text"], f"{path}.text")
    require_string_array(value["refs"], f"{path}.refs", nonempty=True)


def validate_review(value: Any, path: str, *, training_flag: bool = False) -> None:
    require(isinstance(value, dict), f"{path} must be an object")
    keys = ["reviewer", "rationale", "failure_labels"]
    if training_flag:
        keys.append("accepted_for_training")
    require_keys(value, keys, path)
    require_string(value["reviewer"], f"{path}.reviewer")
    require_string(value["rationale"], f"{path}.rationale")
    require_string_array(value["failure_labels"], f"{path}.failure_labels")
    if training_flag:
        require(isinstance(value["accepted_for_training"], bool), f"{path}.accepted_for_training must be boolean")


def validate_event(document: dict[str, Any], source: Path) -> None:
    base = str(source)
    require(document.get("schema_version") == "ghostlight.event_record.v0", f"{base}.schema_version is wrong")
    require_keys(
        document,
        [
            "event_id",
            "review_status",
            "scene_loop_id",
            "turn_id",
            "source_responder_output_ref",
            "actor_agent_id",
            "observable_action",
            "spoken_text",
            "visible_object_refs",
            "visibility_scope",
            "affected_participant_ids",
            "public_state_changes",
            "hidden_intent_handling",
            "review",
        ],
        base,
    )
    require(document["review_status"] in REVIEW_STATUSES, f"{base}.review_status is invalid")
    for key in ["event_id", "scene_loop_id", "turn_id", "actor_agent_id", "observable_action", "hidden_intent_handling"]:
        require_string(document[key], f"{base}.{key}")
    require_ref(document["source_responder_output_ref"], f"{base}.source_responder_output_ref")
    require(isinstance(document["spoken_text"], str), f"{base}.spoken_text must be a string")
    require_string_array(document["visible_object_refs"], f"{base}.visible_object_refs")
    require_string_array(document["affected_participant_ids"], f"{base}.affected_participant_ids", nonempty=True)
    require(document["visibility_scope"] in VISIBILITY_SCOPES, f"{base}.visibility_scope is invalid")
    require(isinstance(document["public_state_changes"], list), f"{base}.public_state_changes must be an array")
    for index, item in enumerate(document["public_state_changes"]):
        validate_text_ref(item, f"{base}.public_state_changes[{index}]")
    validate_review(document["review"], f"{base}.review", training_flag=True)


def validate_appraisal(document: dict[str, Any], source: Path) -> None:
    base = str(source)
    require(document.get("schema_version") == "ghostlight.participant_appraisal.v0", f"{base}.schema_version is wrong")
    require_keys(
        document,
        [
            "appraisal_id",
            "review_status",
            "scene_loop_id",
            "turn_id",
            "participant_agent_id",
            "event_ref",
            "current_character_state_ref",
            "participant_local_context",
            "observable_event_summary",
            "known_facts",
            "active_pressures",
            "perceived_relationships",
            "interpretation",
            "emotional_appraisal",
            "interpretation_label",
            "confidence_notes",
            "candidate_implications",
            "review",
        ],
        base,
    )
    require(document["review_status"] in REVIEW_STATUSES, f"{base}.review_status is invalid")
    for key in ["appraisal_id", "scene_loop_id", "turn_id", "participant_agent_id", "observable_event_summary", "interpretation", "emotional_appraisal", "confidence_notes"]:
        require_string(document[key], f"{base}.{key}")
    require_ref(document["event_ref"], f"{base}.event_ref")
    require_ref(document["current_character_state_ref"], f"{base}.current_character_state_ref")
    require_string_array(document["participant_local_context"], f"{base}.participant_local_context", nonempty=True)
    require_string_array(document["known_facts"], f"{base}.known_facts", nonempty=True)
    require_string_array(document["active_pressures"], f"{base}.active_pressures", nonempty=True)
    require_string_array(document["perceived_relationships"], f"{base}.perceived_relationships", nonempty=True)
    require(document["interpretation_label"] in INTERPRETATION_LABELS, f"{base}.interpretation_label is invalid")
    implications = document["candidate_implications"]
    require(isinstance(implications, dict), f"{base}.candidate_implications must be an object")
    for key in ["memory", "relationship", "state", "world"]:
        require_string_array(implications.get(key), f"{base}.candidate_implications.{key}")
    validate_review(document["review"], f"{base}.review", training_flag=True)


def validate_delta(value: Any, path: str) -> None:
    require(isinstance(value, dict), f"{path} must be an object")
    require_keys(value, ["delta_id", "target_ref", "delta_type", "summary", "authority", "status", "rationale"], path)
    for key in ["delta_id", "target_ref", "delta_type", "summary", "rationale"]:
        require_string(value[key], f"{path}.{key}")
    require(value["authority"] in DELTA_AUTHORITIES, f"{path}.authority is invalid")
    require(value["status"] in DELTA_STATUSES, f"{path}.status is invalid")


def validate_mutation(document: dict[str, Any], source: Path) -> None:
    base = str(source)
    require(document.get("schema_version") == "ghostlight.reviewed_mutation.v0", f"{base}.schema_version is wrong")
    require_keys(
        document,
        [
            "mutation_id",
            "review_status",
            "scene_loop_id",
            "turn_id",
            "prior_state_refs",
            "event_ref",
            "responder_output_ref",
            "appraisal_refs",
            "accepted_deltas",
            "rejected_deltas",
            "deferred_deltas",
            "authority_notes",
            "post_state_summary",
            "review",
        ],
        base,
    )
    require(document["review_status"] in REVIEW_STATUSES, f"{base}.review_status is invalid")
    for key in ["mutation_id", "scene_loop_id", "turn_id", "authority_notes", "post_state_summary"]:
        require_string(document[key], f"{base}.{key}")
    require_string_array(document["prior_state_refs"], f"{base}.prior_state_refs", nonempty=True)
    for index, ref in enumerate(document["prior_state_refs"]):
        require_ref(ref, f"{base}.prior_state_refs[{index}]")
    require_ref(document["event_ref"], f"{base}.event_ref")
    require_ref(document["responder_output_ref"], f"{base}.responder_output_ref")
    require_string_array(document["appraisal_refs"], f"{base}.appraisal_refs", nonempty=True)
    for index, ref in enumerate(document["appraisal_refs"]):
        require_ref(ref, f"{base}.appraisal_refs[{index}]")
    for key in ["accepted_deltas", "rejected_deltas", "deferred_deltas"]:
        require(isinstance(document[key], list), f"{base}.{key} must be an array")
        for index, delta in enumerate(document[key]):
            validate_delta(delta, f"{base}.{key}[{index}]")
    require(document["accepted_deltas"], f"{base}.accepted_deltas must not be empty")
    validate_review(document["review"], f"{base}.review", training_flag=True)


def validate_bundle(document: dict[str, Any], source: Path) -> None:
    base = str(source)
    require(document.get("schema_version") == "ghostlight.scene_loop_bundle.v0", f"{base}.schema_version is wrong")
    require_keys(
        document,
        [
            "bundle_id",
            "review_status",
            "training_usability",
            "reference_fixture_refs",
            "scene_source",
            "initial_state_ref",
            "turns",
            "review_summary",
        ],
        base,
    )
    require(document["review_status"] in REVIEW_STATUSES, f"{base}.review_status is invalid")
    usability = document["training_usability"]
    require(isinstance(usability, dict), f"{base}.training_usability must be an object")
    require("overall" in usability, f"{base}.training_usability.overall is required")
    for key, value in usability.items():
        require_string(key, f"{base}.training_usability key")
        require(value in TRAINING_USABILITY, f"{base}.training_usability.{key} is invalid")
    require(isinstance(document["reference_fixture_refs"], list) and document["reference_fixture_refs"], f"{base}.reference_fixture_refs must be non-empty")
    for index, ref in enumerate(document["reference_fixture_refs"]):
        require_ref(ref, f"{base}.reference_fixture_refs[{index}]")
    scene = document["scene_source"]
    require_keys(scene, ["source_fixture_id", "source_scene_ids", "immutability_note"], f"{base}.scene_source")
    require_string_array(scene["source_scene_ids"], f"{base}.scene_source.source_scene_ids", nonempty=True)
    require_string(scene["immutability_note"], f"{base}.scene_source.immutability_note")
    require_ref(document["initial_state_ref"], f"{base}.initial_state_ref")
    turns = document["turns"]
    require(isinstance(turns, list) and len(turns) >= 3, f"{base}.turns must include at least three turns")
    for index, turn in enumerate(turns):
        turn_path = f"{base}.turns[{index}]"
        require_keys(
            turn,
            [
                "turn_id",
                "acting_agent_id",
                "coordinator_artifact_ref",
                "projected_context_ref",
                "responder_packet_ref",
                "raw_responder_output_ref",
                "event_ref",
                "appraisal_refs",
                "mutation_ref",
                "training_usability",
            ],
            turn_path,
        )
        for key in ["turn_id", "acting_agent_id"]:
            require_string(turn[key], f"{turn_path}.{key}")
        for key in ["coordinator_artifact_ref", "projected_context_ref", "responder_packet_ref", "raw_responder_output_ref", "event_ref", "mutation_ref"]:
            require_ref(turn[key], f"{turn_path}.{key}")
        require_string_array(turn["appraisal_refs"], f"{turn_path}.appraisal_refs", nonempty=True)
        for ref_index, ref in enumerate(turn["appraisal_refs"]):
            require_ref(ref, f"{turn_path}.appraisal_refs[{ref_index}]")
        require(turn["training_usability"] in TRAINING_USABILITY, f"{turn_path}.training_usability is invalid")
    summary = document["review_summary"]
    require_keys(summary, ["quality_bar_ref", "training_ready_organs", "not_training_data_organs", "failure_labels", "review_notes"], f"{base}.review_summary")
    require_string(summary["quality_bar_ref"], f"{base}.review_summary.quality_bar_ref")
    require_string_array(summary["training_ready_organs"], f"{base}.review_summary.training_ready_organs")
    require_string_array(summary["not_training_data_organs"], f"{base}.review_summary.not_training_data_organs")
    require_string_array(summary["failure_labels"], f"{base}.review_summary.failure_labels")
    require_string_array(summary["review_notes"], f"{base}.review_summary.review_notes", nonempty=True)


VALIDATORS = {
    "ghostlight.event_record.v0": validate_event,
    "ghostlight.participant_appraisal.v0": validate_appraisal,
    "ghostlight.reviewed_mutation.v0": validate_mutation,
    "ghostlight.scene_loop_bundle.v0": validate_bundle,
}


def main() -> int:
    parser = argparse.ArgumentParser(description="Validate Ghostlight training-loop receipt artifacts.")
    parser.add_argument("paths", nargs="*", type=Path)
    args = parser.parse_args()

    paths = args.paths or sorted(DEFAULT_ROOT.glob("**/*.json"))
    if not paths:
        print("ok: no training-loop artifacts found")
        return 0
    for path in paths:
        document = load_json(path)
        if not isinstance(document, dict):
            raise ValidationError(f"{path} must be an object")
        schema_version = document.get("schema_version")
        validator = VALIDATORS.get(schema_version)
        if validator is None:
            # Initial state and readable review artifacts are intentionally plain
            # support documents; they only need to be JSON.
            print(f"ok: {path} (support artifact)")
            continue
        validator(document, path)
        print(f"ok: {path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
