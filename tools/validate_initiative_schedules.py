"""Validate Ghostlight initiative schedule artifacts."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
DEFAULT_PATHS = sorted((ROOT / "examples" / "initiative").glob("*.json"))

ACTION_TYPES = {
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
ACTION_SCALES = {"micro", "short", "standard", "major", "committed"}
PARTICIPANT_STATUSES = {"active", "blocked", "withdrawn", "incapacitated", "offscreen"}
SELECTION_KINDS = {"scheduled_turn", "reaction_interrupt", "coordinator_override"}
TIE_BREAKERS = {
    "reaction_readiness_desc",
    "next_ready_at_asc",
    "initiative_speed_desc",
    "stable_actor_id_asc",
}


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


def require_number(value: Any, path: str, *, minimum: float | None = None, maximum: float | None = None, exclusive_minimum: float | None = None) -> None:
    require(isinstance(value, (int, float)) and not isinstance(value, bool), f"{path} must be numeric")
    if minimum is not None:
        require(value >= minimum, f"{path} must be >= {minimum}")
    if maximum is not None:
        require(value <= maximum, f"{path} must be <= {maximum}")
    if exclusive_minimum is not None:
        require(value > exclusive_minimum, f"{path} must be > {exclusive_minimum}")


def require_string_array(value: Any, path: str, *, nonempty: bool = False) -> None:
    require(isinstance(value, list), f"{path} must be an array")
    require(not nonempty or bool(value), f"{path} must not be empty")
    for index, item in enumerate(value):
        require_string(item, f"{path}[{index}]")


def validate_participant(value: Any, path: str) -> dict[str, Any]:
    require(isinstance(value, dict), f"{path} must be an object")
    require_keys(
        value,
        [
            "agent_id",
            "display_name",
            "initiative_speed",
            "next_ready_at",
            "reaction_bias",
            "interrupt_threshold",
            "current_load",
            "status",
            "constraints",
        ],
        path,
    )
    require_string(value["agent_id"], f"{path}.agent_id")
    require_string(value["display_name"], f"{path}.display_name")
    require_number(value["initiative_speed"], f"{path}.initiative_speed", exclusive_minimum=0)
    require_number(value["next_ready_at"], f"{path}.next_ready_at", minimum=0)
    require_number(value["reaction_bias"], f"{path}.reaction_bias", minimum=0, maximum=1)
    require_number(value["interrupt_threshold"], f"{path}.interrupt_threshold", minimum=0, maximum=1)
    require_number(value["current_load"], f"{path}.current_load", minimum=0, maximum=1)
    require(value["status"] in PARTICIPANT_STATUSES, f"{path}.status is invalid")
    require_string_array(value["constraints"], f"{path}.constraints")
    return value


def validate_action(value: Any, path: str, participant_ids: set[str]) -> dict[str, Any]:
    require(isinstance(value, dict), f"{path} must be an object")
    require_keys(
        value,
        [
            "action_id",
            "actor_id",
            "action_type",
            "action_scale",
            "base_recovery",
            "initiative_cost",
            "interruptibility",
            "commitment",
            "local_affordance_basis",
        ],
        path,
    )
    require_string(value["action_id"], f"{path}.action_id")
    require_string(value["actor_id"], f"{path}.actor_id")
    require(value["actor_id"] in participant_ids, f"{path}.actor_id is not a participant")
    require(value["action_type"] in ACTION_TYPES, f"{path}.action_type is invalid")
    require(value["action_scale"] in ACTION_SCALES, f"{path}.action_scale is invalid")
    require_number(value["base_recovery"], f"{path}.base_recovery", minimum=0)
    require_number(value["initiative_cost"], f"{path}.initiative_cost", minimum=0)
    require_number(value["interruptibility"], f"{path}.interruptibility", minimum=0, maximum=1)
    require_number(value["commitment"], f"{path}.commitment", minimum=0, maximum=1)
    require_string_array(value["local_affordance_basis"], f"{path}.local_affordance_basis", nonempty=True)
    return value


def validate_reaction_window(value: Any, path: str, participant_ids: set[str]) -> dict[str, Any]:
    require(isinstance(value, dict), f"{path} must be an object")
    require_keys(
        value,
        [
            "window_id",
            "trigger_event_ref",
            "urgency",
            "eligible_actor_ids",
            "allowed_action_scales",
            "expires_at",
            "notes",
        ],
        path,
    )
    require_string(value["window_id"], f"{path}.window_id")
    require_string(value["trigger_event_ref"], f"{path}.trigger_event_ref")
    require_number(value["urgency"], f"{path}.urgency", minimum=0, maximum=1)
    require_string_array(value["eligible_actor_ids"], f"{path}.eligible_actor_ids", nonempty=True)
    for actor_id in value["eligible_actor_ids"]:
        require(actor_id in participant_ids, f"{path}.eligible_actor_ids contains unknown actor {actor_id}")
    require(isinstance(value["allowed_action_scales"], list) and value["allowed_action_scales"], f"{path}.allowed_action_scales must be a non-empty array")
    for index, scale in enumerate(value["allowed_action_scales"]):
        require(scale in ACTION_SCALES, f"{path}.allowed_action_scales[{index}] is invalid")
    require_number(value["expires_at"], f"{path}.expires_at", minimum=0)
    require_string(value["notes"], f"{path}.notes")
    return value


def reaction_readiness(participant: dict[str, Any], urgency: float) -> float:
    return (participant["reaction_bias"] * urgency) - participant["current_load"]


def validate_schedule(document: dict[str, Any], source: Path) -> None:
    base = str(source)
    require(document.get("schema_version") == "ghostlight.initiative_schedule.v0", f"{base}.schema_version is wrong")
    require_keys(
        document,
        [
            "schedule_id",
            "source_scene_ref",
            "scene_clock",
            "participants",
            "action_catalog",
            "reaction_windows",
            "selection_policy",
            "next_actor_selection",
            "review_notes",
        ],
        base,
    )
    require_string(document["schedule_id"], f"{base}.schedule_id")
    require_string(document["source_scene_ref"], f"{base}.source_scene_ref")
    require_number(document["scene_clock"], f"{base}.scene_clock", minimum=0)
    require((ROOT / document["source_scene_ref"]).exists(), f"{base}.source_scene_ref does not exist")

    require(isinstance(document["participants"], list) and document["participants"], f"{base}.participants must be non-empty")
    participants = [validate_participant(item, f"{base}.participants[{index}]") for index, item in enumerate(document["participants"])]
    participant_ids = [participant["agent_id"] for participant in participants]
    require(len(participant_ids) == len(set(participant_ids)), f"{base}.participants contains duplicate agent_id")
    participants_by_id = {participant["agent_id"]: participant for participant in participants}
    active_participants = [participant for participant in participants if participant["status"] == "active"]
    require(active_participants, f"{base}.participants must include at least one active participant")

    require(isinstance(document["action_catalog"], list), f"{base}.action_catalog must be an array")
    actions = [validate_action(item, f"{base}.action_catalog[{index}]", set(participant_ids)) for index, item in enumerate(document["action_catalog"])]
    action_ids = [action["action_id"] for action in actions]
    require(len(action_ids) == len(set(action_ids)), f"{base}.action_catalog contains duplicate action_id")
    actions_by_id = {action["action_id"]: action for action in actions}

    require(isinstance(document["reaction_windows"], list), f"{base}.reaction_windows must be an array")
    windows = [validate_reaction_window(item, f"{base}.reaction_windows[{index}]", set(participant_ids)) for index, item in enumerate(document["reaction_windows"])]
    window_ids = [window["window_id"] for window in windows]
    require(len(window_ids) == len(set(window_ids)), f"{base}.reaction_windows contains duplicate window_id")

    policy = document["selection_policy"]
    require(isinstance(policy, dict), f"{base}.selection_policy must be an object")
    require_keys(policy, ["mode", "reaction_precedence", "minimum_speed", "tie_breakers"], f"{base}.selection_policy")
    require(policy["mode"] == "readiness_queue", f"{base}.selection_policy.mode is invalid")
    require(isinstance(policy["reaction_precedence"], bool), f"{base}.selection_policy.reaction_precedence must be boolean")
    require_number(policy["minimum_speed"], f"{base}.selection_policy.minimum_speed", exclusive_minimum=0)
    require(isinstance(policy["tie_breakers"], list) and policy["tie_breakers"], f"{base}.selection_policy.tie_breakers must be non-empty")
    for index, tie_breaker in enumerate(policy["tie_breakers"]):
        require(tie_breaker in TIE_BREAKERS, f"{base}.selection_policy.tie_breakers[{index}] is invalid")

    selection = document["next_actor_selection"]
    require(isinstance(selection, dict), f"{base}.next_actor_selection must be an object")
    require_keys(
        selection,
        [
            "selection_kind",
            "selected_actor_id",
            "selected_action_ids",
            "scene_clock_after_selection",
            "selection_reason",
            "readiness_snapshot",
        ],
        f"{base}.next_actor_selection",
    )
    require(selection["selection_kind"] in SELECTION_KINDS, f"{base}.next_actor_selection.selection_kind is invalid")
    require_string(selection["selected_actor_id"], f"{base}.next_actor_selection.selected_actor_id")
    require(selection["selected_actor_id"] in participants_by_id, f"{base}.next_actor_selection.selected_actor_id is not a participant")
    require(participants_by_id[selection["selected_actor_id"]]["status"] == "active", f"{base}.next_actor_selection.selected_actor_id is not active")
    require_string_array(selection["selected_action_ids"], f"{base}.next_actor_selection.selected_action_ids")
    for action_id in selection["selected_action_ids"]:
        require(action_id in actions_by_id, f"{base}.next_actor_selection.selected_action_ids contains unknown action {action_id}")
        require(actions_by_id[action_id]["actor_id"] == selection["selected_actor_id"], f"{base}.next_actor_selection action {action_id} belongs to a different actor")
    require_number(selection["scene_clock_after_selection"], f"{base}.next_actor_selection.scene_clock_after_selection", minimum=0)
    require(selection["scene_clock_after_selection"] >= document["scene_clock"], f"{base}.next_actor_selection.scene_clock_after_selection must not move backward")
    require_string(selection["selection_reason"], f"{base}.next_actor_selection.selection_reason")

    snapshot = selection["readiness_snapshot"]
    require(isinstance(snapshot, list) and snapshot, f"{base}.next_actor_selection.readiness_snapshot must be non-empty")
    snapshot_ids: set[str] = set()
    for index, item in enumerate(snapshot):
        item_path = f"{base}.next_actor_selection.readiness_snapshot[{index}]"
        require(isinstance(item, dict), f"{item_path} must be an object")
        require_keys(item, ["agent_id", "next_ready_at", "reaction_readiness", "eligible_for_reaction"], item_path)
        require_string(item["agent_id"], f"{item_path}.agent_id")
        require(item["agent_id"] in participants_by_id, f"{item_path}.agent_id is not a participant")
        snapshot_ids.add(item["agent_id"])
        require_number(item["next_ready_at"], f"{item_path}.next_ready_at", minimum=0)
        if item["reaction_readiness"] is not None:
            require_number(item["reaction_readiness"], f"{item_path}.reaction_readiness")
        require(isinstance(item["eligible_for_reaction"], bool), f"{item_path}.eligible_for_reaction must be boolean")
    require(snapshot_ids == set(participant_ids), f"{base}.next_actor_selection.readiness_snapshot must include every participant")

    if selection["selection_kind"] == "coordinator_override":
        require_string(selection.get("override_reason"), f"{base}.next_actor_selection.override_reason")
    else:
        require(selection.get("override_reason") in (None, ""), f"{base}.next_actor_selection.override_reason must be empty without override")

    if selection["selection_kind"] == "scheduled_turn":
        earliest = min(active_participants, key=lambda item: (item["next_ready_at"], -item["initiative_speed"], item["agent_id"]))
        require(selection["selected_actor_id"] == earliest["agent_id"], f"{base}.scheduled_turn did not select earliest active participant")

    if selection["selection_kind"] == "reaction_interrupt":
        require(windows, f"{base}.reaction_interrupt requires at least one reaction window")
        selected = participants_by_id[selection["selected_actor_id"]]
        valid_window = False
        for window in windows:
            if selected["agent_id"] not in window["eligible_actor_ids"]:
                continue
            readiness = reaction_readiness(selected, window["urgency"])
            if readiness >= selected["interrupt_threshold"]:
                valid_window = True
                break
        require(valid_window, f"{base}.reaction_interrupt selected actor does not clear any reaction threshold")

    require_string_array(document["review_notes"], f"{base}.review_notes")


def main() -> int:
    parser = argparse.ArgumentParser(description="Validate Ghostlight initiative schedule artifacts.")
    parser.add_argument("paths", nargs="*", type=Path, help="Initiative schedule JSON files to validate.")
    args = parser.parse_args()

    paths = args.paths or DEFAULT_PATHS
    if not paths:
        print("ok: no initiative schedules found")
        return 0

    for path in paths:
        validate_schedule(load_json(path), path)
        print(f"ok: {path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
