"""Validate Ghostlight agent-state examples.

This is not a full JSON Schema implementation. It is a focused smoke validator
for the invariants the first Ghostlight schema must not violate.
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
REQUIRED_FIELDS_PATH = ROOT / "schemas" / "agent-state.required-fields.json"
DEFAULT_EXAMPLES = [ROOT / "examples" / "agent-state.call-of-the-void.json"]


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


def validate_state_variable(obj: Any, path: str) -> None:
    require(isinstance(obj, dict), f"{path} must be an object")
    require_keys(obj, ["mean", "certainty", "plasticity", "current_activation"], path)
    for key in ["mean", "certainty", "plasticity", "current_activation"]:
        require_0_1(obj[key], f"{path}.{key}")


def validate_variable_map(
    obj: Any,
    required_names: list[str] | None,
    path: str,
) -> None:
    require(isinstance(obj, dict), f"{path} must be an object")
    if required_names is not None:
        missing = [name for name in required_names if name not in obj]
        require(not missing, f"{path} missing variables: {', '.join(missing)}")
    for name, value in obj.items():
        validate_state_variable(value, f"{path}.{name}")


def validate_agent(agent: dict[str, Any], required: dict[str, list[str]]) -> None:
    agent_id = agent.get("agent_id", "<missing>")
    path = f"agents[{agent_id}]"
    require_keys(
        agent,
        ["agent_id", "identity", "canonical_state", "goals", "memories", "perceived_state_overlays"],
        path,
    )
    canonical = agent["canonical_state"]
    require_keys(
        canonical,
        [
            "underlying_organization",
            "stable_dispositions",
            "behavioral_dimensions",
            "presentation_strategy",
            "voice_style",
            "situational_state",
            "values",
        ],
        f"{path}.canonical_state",
    )
    for group in [
        "underlying_organization",
        "stable_dispositions",
        "behavioral_dimensions",
        "presentation_strategy",
        "voice_style",
        "situational_state",
    ]:
        validate_variable_map(canonical[group], required[group], f"{path}.canonical_state.{group}")

    for index, value in enumerate(canonical["values"]):
        require_keys(value, ["value_id", "label", "priority", "unforgivable_if_betrayed"], f"{path}.values[{index}]")
        require_0_1(value["priority"], f"{path}.values[{index}].priority")

    for index, goal in enumerate(agent["goals"]):
        require_keys(goal, ["goal_id", "description", "scope", "priority", "emotional_stake", "status"], f"{path}.goals[{index}]")
        require_0_1(goal["priority"], f"{path}.goals[{index}].priority")

    memories = agent["memories"]
    require_keys(memories, ["episodic", "semantic", "relationship_summaries"], f"{path}.memories")
    for group_name in ["episodic", "semantic", "relationship_summaries"]:
        for index, memory in enumerate(memories[group_name]):
            memory_path = f"{path}.memories.{group_name}[{index}]"
            require_keys(memory, ["memory_id", "summary", "salience", "confidence"], memory_path)
            require_0_1(memory["salience"], f"{memory_path}.salience")
            require_0_1(memory["confidence"], f"{memory_path}.confidence")

    for index, overlay in enumerate(agent["perceived_state_overlays"]):
        overlay_path = f"{path}.perceived_state_overlays[{index}]"
        require_keys(
            overlay,
            ["observer_agent_id", "target_agent_id", "perceived_dimensions", "beliefs", "distortions"],
            overlay_path,
        )
        validate_variable_map(overlay["perceived_dimensions"], None, f"{overlay_path}.perceived_dimensions")
        for belief_index, belief in enumerate(overlay["beliefs"]):
            belief_path = f"{overlay_path}.beliefs[{belief_index}]"
            require_keys(belief, ["belief_id", "claim", "confidence"], belief_path)
            require_0_1(belief["confidence"], f"{belief_path}.confidence")


def validate_document(document: dict[str, Any], required: dict[str, list[str]], source: Path) -> None:
    require(document.get("schema_version") == "ghostlight.agent_state.v0", f"{source} has wrong schema_version")
    require_keys(document, ["world", "agents", "relationships", "events", "scenes"], str(source))

    agent_ids = [agent["agent_id"] for agent in document["agents"]]
    require(len(agent_ids) == len(set(agent_ids)), f"{source} has duplicate agent_id values")
    agent_id_set = set(agent_ids)

    goal_ids: set[str] = set()
    memory_ids: set[str] = set()
    for agent in document["agents"]:
        validate_agent(agent, required)
        goal_ids.update(goal["goal_id"] for goal in agent["goals"])
        for memories in agent["memories"].values():
            memory_ids.update(memory["memory_id"] for memory in memories)

    event_ids = {event["event_id"] for event in document["events"]}

    for index, relationship in enumerate(document["relationships"]):
        path = f"relationships[{index}]"
        require_keys(relationship, ["relationship_id", "source_id", "target_id", "stance", "summary"], path)
        require(relationship["source_id"] in agent_id_set, f"{path}.source_id references unknown agent")
        require(relationship["target_id"] in agent_id_set, f"{path}.target_id references unknown agent")
        validate_variable_map(relationship["stance"], required["relationship_stance"], f"{path}.stance")
        for event_id in relationship.get("unresolved_incident_ids", []):
            require(event_id in event_ids, f"{path}.unresolved_incident_ids references unknown event: {event_id}")

    for index, event in enumerate(document["events"]):
        path = f"events[{index}]"
        require_keys(event, ["event_id", "kind", "summary", "participants", "pressure_tags"], path)
        for participant_id in event["participants"]:
            require(participant_id in agent_id_set, f"{path}.participants references unknown agent: {participant_id}")

    for index, scene in enumerate(document["scenes"]):
        path = f"scenes[{index}]"
        require_keys(scene, ["scene_id", "location", "participants", "dialogue_context_packs"], path)
        for participant_id in scene["participants"]:
            require(participant_id in agent_id_set, f"{path}.participants references unknown agent: {participant_id}")
        for pack_index, pack in enumerate(scene["dialogue_context_packs"]):
            pack_path = f"{path}.dialogue_context_packs[{pack_index}]"
            require_keys(
                pack,
                [
                    "speaker_agent_id",
                    "listener_ids",
                    "speaker_local_truth",
                    "speaker_beliefs",
                    "active_memories",
                    "active_goals",
                    "presentation_constraints",
                    "forbidden_context",
                ],
                pack_path,
            )
            require(pack["speaker_agent_id"] in agent_id_set, f"{pack_path}.speaker_agent_id references unknown agent")
            for listener_id in pack["listener_ids"]:
                require(listener_id in agent_id_set, f"{pack_path}.listener_ids references unknown agent: {listener_id}")
            for goal_id in pack["active_goals"]:
                require(goal_id in goal_ids, f"{pack_path}.active_goals references unknown goal: {goal_id}")
            for memory_id in pack["active_memories"]:
                require(memory_id in memory_ids, f"{pack_path}.active_memories references unknown memory: {memory_id}")
            forbidden_blob = "\n".join(pack["forbidden_context"]).lower()
            local_blob = "\n".join(pack["speaker_local_truth"] + pack["speaker_beliefs"]).lower()
            for forbidden in ["author-only", "future plot"]:
                if forbidden in forbidden_blob:
                    require(forbidden not in local_blob, f"{pack_path} leaks forbidden context into local state: {forbidden}")


def main() -> int:
    parser = argparse.ArgumentParser(description="Validate Ghostlight agent-state examples.")
    parser.add_argument("paths", nargs="*", type=Path, help="Example JSON files to validate.")
    args = parser.parse_args()

    required_doc = load_json(REQUIRED_FIELDS_PATH)
    required = required_doc["variable_groups"]
    paths = args.paths or DEFAULT_EXAMPLES

    for path in paths:
        document = load_json(path)
        validate_document(document, required, path)
        print(f"ok: {path}")

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
