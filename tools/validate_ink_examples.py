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
DEFAULT_QWEN_INK_CAPTURES = sorted((ROOT / "experiments" / "ink").glob("*.capture.json"))
DEFAULT_MUTATION_RECEIPTS = sorted((ROOT / "experiments" / "ink").glob("*.mutation.json"))
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
VALID_CAPTURE_REVIEW_STATUSES = {"accepted", "accepted_as_draft", "rejected", "needs_revision", "useful_needs_revision"}
VALID_VISUAL_SCENE_KEYS = {
    "visual_scene_id",
    "ink_anchor",
    "base_image_prompt",
    "visible_characters",
    "state_image_modifiers",
    "visual_continuity_notes",
}


class ValidationError(Exception):
    pass


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValidationError(message)


def require_keys(obj: dict[str, Any], keys: list[str], path: str) -> None:
    missing = [key for key in keys if key not in obj]
    require(not missing, f"{path} missing required keys: {', '.join(missing)}")


def require_0_1(value: Any, path: str) -> None:
    require(isinstance(value, (int, float)), f"{path} must be numeric")
    require(0 <= value <= 1, f"{path} must be between 0 and 1")


def load_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def npx_command() -> str:
    command = shutil.which("npx.cmd") or shutil.which("npx")
    require(command is not None, "npx is required to compile Ink examples")
    return command


def node_command() -> str:
    command = shutil.which("node.exe") or shutil.which("node")
    require(command is not None, "node is required to smoke-test compiled Ink examples")
    return command


def compile_ink(path: Path) -> None:
    validate_no_preview_leaking_metadata_tags(path)
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
        validate_story_reaches_initial_choices(path, output_path)


def validate_no_preview_leaking_metadata_tags(path: Path) -> None:
    for line_number, line in enumerate(path.read_text(encoding="utf-8").splitlines(), start=1):
        stripped = line.lstrip()
        if stripped.startswith("# ghostlight.") or stripped.startswith("# aetheria."):
            raise ValidationError(
                f"{path}:{line_number} uses an Ink tag for Ghostlight/Aetheria metadata; use // comments so Inky preview stays clean"
            )


def validate_story_reaches_initial_choices(source_path: Path, compiled_path: Path) -> None:
    script = """
const fs = require("fs");
const { Story } = require("inkjs");
const storyJson = fs.readFileSync(process.argv[1], "utf8").replace(/^\\uFEFF/, "");
const story = new Story(storyJson);
let safety = 0;
while (story.canContinue && safety < 1000) {
  story.Continue();
  safety += 1;
}
if (safety >= 1000) {
  throw new Error("story did not settle before safety limit");
}
if (story.currentChoices.length === 0) {
  throw new Error("story starts without any available choices");
}
"""
    result = subprocess.run(
        [node_command(), "-e", script, str(compiled_path)],
        cwd=ROOT,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=False,
    )
    require(
        result.returncode == 0,
        f"{source_path} compiled, but does not reach initial choices\nSTDOUT:\n{result.stdout}\nSTDERR:\n{result.stderr}",
    )


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
    if "visual_plan_ref" in data:
        visual_plan_path = ROOT / data["visual_plan_ref"]
        require(visual_plan_path.exists(), f"{path}.visual_plan_ref does not exist")
        validate_ink_visual_plan(visual_plan_path, ink_path, path)
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


def validate_ink_visual_plan(path: Path, ink_path: Path, annotation_path: Path) -> None:
    data = load_json(path)
    require(isinstance(data, dict), f"{path} must be an object")
    require_keys(
        data,
        [
            "schema_version",
            "visual_plan_id",
            "ink_ref",
            "source_training_annotation_ref",
            "scene_id",
            "purpose",
            "visual_scene_plan",
            "visual_character_refs",
        ],
        str(path),
    )
    require(data["schema_version"] == "ghostlight.ink_visual_plan.v0", f"{path}.schema_version is wrong")
    require((ROOT / data["ink_ref"]).resolve() == ink_path.resolve(), f"{path}.ink_ref does not match {ink_path}")
    require((ROOT / data["source_training_annotation_ref"]).resolve() == annotation_path.resolve(), f"{path}.source_training_annotation_ref does not match {annotation_path}")
    require("visual_art_direction" not in data, f"{path}.visual_art_direction is legacy cruft; use visual_scene_plan instead")

    plan = data["visual_scene_plan"]
    require_keys(plan, ["global_style_cue", "segmentation_rule", "scenes", "global_branch_image_modifiers", "prompt_assembly_rule"], f"{path}.visual_scene_plan")
    require(isinstance(plan["global_style_cue"], str) and plan["global_style_cue"].strip(), f"{path}.visual_scene_plan.global_style_cue must be non-empty")
    require(isinstance(plan["scenes"], list) and plan["scenes"], f"{path}.visual_scene_plan.scenes must be non-empty")
    seen_scene_ids: set[str] = set()
    for index, scene in enumerate(plan["scenes"]):
        scene_path = f"{path}.visual_scene_plan.scenes[{index}]"
        require(isinstance(scene, dict), f"{scene_path} must be an object")
        require(set(scene.keys()).issuperset(VALID_VISUAL_SCENE_KEYS), f"{scene_path} missing visual scene keys")
        scene_id = scene["visual_scene_id"]
        require(isinstance(scene_id, str) and scene_id.strip(), f"{scene_path}.visual_scene_id must be non-empty")
        require(scene_id not in seen_scene_ids, f"{scene_path}.visual_scene_id duplicates {scene_id}")
        seen_scene_ids.add(scene_id)
        require(isinstance(scene["ink_anchor"], str) and scene["ink_anchor"].strip(), f"{scene_path}.ink_anchor must be non-empty")
        require(isinstance(scene["base_image_prompt"], str) and scene["base_image_prompt"].strip(), f"{scene_path}.base_image_prompt must be non-empty")
        require(isinstance(scene["visible_characters"], list), f"{scene_path}.visible_characters must be an array")
        require(isinstance(scene["state_image_modifiers"], list), f"{scene_path}.state_image_modifiers must be an array")
        require(isinstance(scene["visual_continuity_notes"], list), f"{scene_path}.visual_continuity_notes must be an array")

    refs = data["visual_character_refs"]
    require_keys(refs, ["purpose", "prompt_assembly_rule", "characters"], f"{path}.visual_character_refs")
    require(isinstance(refs["characters"], dict) and refs["characters"], f"{path}.visual_character_refs.characters must be non-empty")
    for character_id, ref in refs["characters"].items():
        ref_path = f"{path}.visual_character_refs.characters.{character_id}"
        require_keys(ref, ["display_name", "species_or_body", "stable_visual_description", "silhouette_notes", "do_not_change"], ref_path)


def validate_qwen_ink_capture(path: Path) -> None:
    data = load_json(path)
    require(isinstance(data, dict), f"{path} must be an object")
    if data.get("schema_version") == "ghostlight.qwen_ink_sequential_capture.v0":
        validate_qwen_ink_sequential_capture(path, data)
        return
    require_keys(
        data,
        [
            "schema_version",
            "capture_id",
            "source_fixture_ref",
            "source_annotation_ref",
            "model",
            "endpoint",
            "request_options",
            "prompt_text",
            "response_text",
            "parsed_response",
            "review",
        ],
        str(path),
    )
    require(data["schema_version"] == "ghostlight.qwen_ink_branch_capture.v0", f"{path}.schema_version is wrong")
    require((ROOT / data["source_fixture_ref"]).exists(), f"{path}.source_fixture_ref does not exist")
    require((ROOT / data["source_annotation_ref"]).exists(), f"{path}.source_annotation_ref does not exist")
    require(isinstance(data["prompt_text"], str) and data["prompt_text"].strip(), f"{path}.prompt_text must be non-empty")
    require(isinstance(data["response_text"], str) and data["response_text"].strip(), f"{path}.response_text must be non-empty")

    parsed = data["parsed_response"]
    require(isinstance(parsed, dict), f"{path}.parsed_response must be an object")
    branches = parsed.get("branches")
    require(isinstance(branches, list) and branches, f"{path}.parsed_response.branches must be non-empty")
    non_speech_count = 0
    for index, branch in enumerate(branches):
        branch_path = f"{path}.parsed_response.branches[{index}]"
        require(isinstance(branch, dict), f"{branch_path} must be an object")
        require_keys(
            branch,
            [
                "branch_id",
                "ink_path",
                "choice_text",
                "action_type",
                "actor_intent",
                "maer_action_prose",
                "sella_response_prose",
                "state_basis",
                "reviewed_consequences",
                "training_hooks",
            ],
            branch_path,
        )
        require(branch["action_type"] in VALID_ACTION_TYPES, f"{branch_path}.action_type is invalid")
        non_speech_count += 0 if branch["action_type"] == "speak" else 1
    require(non_speech_count >= 2, f"{path}.parsed_response.branches must include at least two non-speech branches")

    review = data["review"]
    require(isinstance(review, dict), f"{path}.review must be an object")
    require_keys(review, ["status", "strengths", "failure_notes", "accepted_into_ink_ref", "pipeline_lessons"], f"{path}.review")
    require(review["status"] in VALID_CAPTURE_REVIEW_STATUSES, f"{path}.review.status is invalid or unreviewed")
    for list_key in ["strengths", "failure_notes", "pipeline_lessons"]:
        require(isinstance(review[list_key], list), f"{path}.review.{list_key} must be an array")
    accepted_ref = review["accepted_into_ink_ref"]
    if accepted_ref is not None:
        require((ROOT / accepted_ref).exists(), f"{path}.review.accepted_into_ink_ref does not exist")


def validate_action_type_or_review_note(action_type: Any, review_status: str, path: str) -> None:
    if action_type in VALID_ACTION_TYPES:
        return
    require(
        review_status in {"needs_revision", "useful_needs_revision", "rejected"},
        f"{path} has invalid action_type but review.status is not a revision/rejection status",
    )


def validate_qwen_ink_sequential_capture(path: Path, data: dict[str, Any]) -> None:
    require_keys(
        data,
        [
            "schema_version",
            "capture_id",
            "source_fixture_ref",
            "source_annotation_ref",
            "model",
            "endpoint",
            "request_options",
            "passes",
            "review",
        ],
        str(path),
    )
    require((ROOT / data["source_fixture_ref"]).exists(), f"{path}.source_fixture_ref does not exist")
    require((ROOT / data["source_annotation_ref"]).exists(), f"{path}.source_annotation_ref does not exist")
    review = data["review"]
    require(isinstance(review, dict), f"{path}.review must be an object")
    require_keys(review, ["status", "strengths", "failure_notes", "accepted_into_ink_ref", "pipeline_lessons"], f"{path}.review")
    review_status = review["status"]
    require(review_status in VALID_CAPTURE_REVIEW_STATUSES, f"{path}.review.status is invalid or unreviewed")

    passes = data["passes"]
    require_keys(passes, ["maer_choice_generation", "selected_observable_action", "sella_response_generation"], f"{path}.passes")
    if "sella_immediate_appraisal" not in passes:
        require(
            review_status in {"needs_revision", "useful_needs_revision", "rejected"},
            f"{path}.passes missing sella_immediate_appraisal but review.status is not a revision/rejection status",
        )
    for pass_key in ["maer_choice_generation", "sella_response_generation"]:
        pass_payload = passes[pass_key]
        require_keys(pass_payload, ["local_agent_id", "prompt_text", "response_text", "parsed_response", "validation_notes"], f"{path}.passes.{pass_key}")
        require(isinstance(pass_payload["prompt_text"], str) and pass_payload["prompt_text"].strip(), f"{path}.passes.{pass_key}.prompt_text must be non-empty")
        require(isinstance(pass_payload["response_text"], str) and pass_payload["response_text"].strip(), f"{path}.passes.{pass_key}.response_text must be non-empty")
        require(isinstance(pass_payload["validation_notes"], list), f"{path}.passes.{pass_key}.validation_notes must be an array")
    if "sella_immediate_appraisal" in passes:
        pass_payload = passes["sella_immediate_appraisal"]
        require_keys(pass_payload, ["local_agent_id", "prompt_text", "response_text", "parsed_response", "validation_notes"], f"{path}.passes.sella_immediate_appraisal")
        if review_status in {"accepted", "accepted_as_draft"}:
            appraisal_root = pass_payload["parsed_response"]
            require(isinstance(appraisal_root, dict), f"{path}.passes.sella_immediate_appraisal.parsed_response must be an object")
            appraisal = appraisal_root.get("appraisal")
            require(isinstance(appraisal, dict), f"{path}.passes.sella_immediate_appraisal.parsed_response.appraisal must be an object")
            for list_key in ["immediate_state_deltas", "relationship_deltas", "response_constraints"]:
                require(isinstance(appraisal.get(list_key), list), f"{path}.passes.sella_immediate_appraisal.appraisal.{list_key} must be an array for accepted captures")

    choice = passes["selected_observable_action"]
    require(isinstance(choice, dict), f"{path}.passes.selected_observable_action must be an object")
    require_keys(choice, ["branch_id", "ink_path", "choice_text", "action_type", "observable_action"], f"{path}.passes.selected_observable_action")
    validate_action_type_or_review_note(choice["action_type"], review_status, f"{path}.passes.selected_observable_action.action_type")
    if review_status in {"accepted", "accepted_as_draft"}:
        for list_key in ["expected_consequence_surfaces", "training_hooks"]:
            require(isinstance(choice.get(list_key), list), f"{path}.passes.selected_observable_action.{list_key} must be an array for accepted captures")

    response = passes["sella_response_generation"]["parsed_response"]
    if not isinstance(response, dict):
        require(
            review_status in {"needs_revision", "useful_needs_revision", "rejected"},
            f"{path}.passes.sella_response_generation.parsed_response must be an object",
        )
        return
    response_obj = response.get("response")
    require(isinstance(response_obj, dict), f"{path}.passes.sella_response_generation.parsed_response.response must be an object")
    require_keys(response_obj, ["responder_agent_id", "action_type", "observable_response", "sella_private_interpretation", "misread_risk"], f"{path}.passes.sella_response_generation.response")
    validate_action_type_or_review_note(response_obj["action_type"], review_status, f"{path}.passes.sella_response_generation.response.action_type")
    if review_status in {"accepted", "accepted_as_draft"}:
        require(isinstance(response_obj.get("training_hooks"), list), f"{path}.passes.sella_response_generation.response.training_hooks must be an array for accepted captures")


def validate_mutation_receipt(path: Path) -> None:
    data = load_json(path)
    require(isinstance(data, dict), f"{path} must be an object")
    require_keys(
        data,
        [
            "schema_version",
            "mutation_id",
            "source_fixture_ref",
            "source_capture_ref",
            "mutated_fixture_ref",
            "selected_branch_id",
            "event_id",
            "participant_appraisals",
            "applied_mutations",
            "manual_review_notes",
            "deferred_mutations",
        ],
        str(path),
    )
    require(data["schema_version"] == "ghostlight.reviewed_state_mutation.v0", f"{path}.schema_version is wrong")
    require((ROOT / data["source_fixture_ref"]).exists(), f"{path}.source_fixture_ref does not exist")
    require((ROOT / data["source_capture_ref"]).exists(), f"{path}.source_capture_ref does not exist")
    require((ROOT / data["mutated_fixture_ref"]).exists(), f"{path}.mutated_fixture_ref does not exist")
    appraisals = data["participant_appraisals"]
    require(isinstance(appraisals, list) and appraisals, f"{path}.participant_appraisals must be a non-empty array")
    for index, appraisal in enumerate(appraisals):
        appraisal_path = f"{path}.participant_appraisals[{index}]"
        require(isinstance(appraisal, dict), f"{appraisal_path} must be an object")
        require_keys(
            appraisal,
            ["participant_agent_id", "trigger_observable_action", "private_interpretation", "state_deltas", "next_action_constraints"],
            appraisal_path,
        )
    require(isinstance(data["applied_mutations"], list) and data["applied_mutations"], f"{path}.applied_mutations must be non-empty")
    for index, mutation in enumerate(data["applied_mutations"]):
        mutation_path = f"{path}.applied_mutations[{index}]"
        require_keys(mutation, ["target", "path", "before", "after"], mutation_path)
        require_0_1(mutation["before"], f"{mutation_path}.before")
        require_0_1(mutation["after"], f"{mutation_path}.after")


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
    for path in DEFAULT_QWEN_INK_CAPTURES:
        validate_qwen_ink_capture(path)
        print(f"ok: {path}")
    for path in DEFAULT_MUTATION_RECEIPTS:
        validate_mutation_receipt(path)
        print(f"ok: {path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
