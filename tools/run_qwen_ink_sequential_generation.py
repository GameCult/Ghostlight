"""Run a sequential Qwen Ink generation prototype.

Pass 1: generate Maer choices from Maer-local awareness only.
Pass 2: generate participant appraisal/consolidation for the selected
observable event.
Pass 3: generate the next actor's action from updated local state.
"""

from __future__ import annotations

import argparse
import json
import re
import urllib.request
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
DEFAULT_ENDPOINT = "http://192.168.1.84:11434/api/chat"
DEFAULT_FIXTURE = ROOT / "examples" / "agent-state.cold-wake-story-lab.json"
DEFAULT_ANNOTATION = ROOT / "examples" / "ink" / "cold-wake-sanctuary-intake.training.json"
DEFAULT_OUT_DIR = ROOT / "experiments" / "ink"
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


def string_array_schema(description: str) -> dict[str, Any]:
    return {"type": "array", "description": description, "items": {"type": "string"}}


def action_type_schema() -> dict[str, Any]:
    return {"type": "string", "enum": sorted(VALID_ACTION_TYPES)}


def maer_choices_tool() -> dict[str, Any]:
    choice_schema = {
        "type": "object",
        "required": [
            "branch_id",
            "ink_path",
            "choice_text",
            "action_type",
            "actor_intent_private",
            "observable_action",
            "spoken_text",
            "state_basis",
            "expected_consequence_surfaces",
            "training_hooks",
        ],
        "properties": {
            "branch_id": {"type": "string"},
            "ink_path": {"type": "string"},
            "choice_text": {"type": "string"},
            "action_type": action_type_schema(),
            "actor_intent_private": {"type": "string"},
            "observable_action": {"type": "string"},
            "spoken_text": {"type": "string"},
            "state_basis": {"type": "string"},
            "expected_consequence_surfaces": string_array_schema("Possible effects if this choice is selected."),
            "training_hooks": string_array_schema("Semantic labels, not branch ids."),
        },
    }
    return {
        "type": "function",
        "function": {
            "name": "record_maer_choices",
            "description": "Record four Maer-local candidate choices for the current scene.",
            "parameters": {
                "type": "object",
                "required": ["choices"],
                "properties": {
                    "choices": {
                        "type": "array",
                        "minItems": 4,
                        "maxItems": 4,
                        "items": choice_schema,
                    }
                },
            },
        },
    }


def sella_appraisal_tool() -> dict[str, Any]:
    delta_schema = {
        "type": "object",
        "required": ["path", "delta", "rationale"],
        "properties": {
            "path": {"type": "string"},
            "delta": {"type": "number"},
            "rationale": {"type": "string"},
        },
    }
    relationship_delta_schema = {
        "type": "object",
        "required": ["relationship_id", "path", "delta", "rationale"],
        "properties": {
            "relationship_id": {"type": "string"},
            "path": {"type": "string"},
            "delta": {"type": "number"},
            "rationale": {"type": "string"},
        },
    }
    return {
        "type": "function",
        "function": {
            "name": "record_sella_appraisal",
            "description": "Record Sella's participant appraisal and proposed state consolidation for the observed Maer event.",
            "parameters": {
                "type": "object",
                "required": [
                    "responder_agent_id",
                    "private_interpretation",
                    "attributed_motive",
                    "misread_risk",
                    "immediate_state_deltas",
                    "relationship_deltas",
                    "belief_updates",
                    "response_constraints",
                ],
                "properties": {
                    "responder_agent_id": {"type": "string", "enum": ["sella_ren"]},
                    "private_interpretation": {"type": "string"},
                    "attributed_motive": {"type": "string"},
                    "misread_risk": {"type": "string"},
                    "immediate_state_deltas": {"type": "array", "items": delta_schema},
                    "relationship_deltas": {"type": "array", "items": relationship_delta_schema},
                    "belief_updates": string_array_schema("Belief changes or reinforced beliefs caused by the event."),
                    "response_constraints": string_array_schema("Constraints that should shape the next action."),
                },
            },
        },
    }


def sella_next_action_tool() -> dict[str, Any]:
    response_schema = {
        "type": "object",
        "required": [
            "responder_agent_id",
            "action_type",
            "observable_response",
            "spoken_text",
            "sella_private_interpretation",
            "sella_attributed_motive",
            "misread_risk",
            "state_basis",
            "reviewed_consequences",
            "training_hooks",
        ],
        "properties": {
            "responder_agent_id": {"type": "string", "enum": ["sella_ren"]},
            "action_type": action_type_schema(),
            "observable_response": {"type": "string"},
            "spoken_text": {"type": "string"},
            "sella_private_interpretation": {"type": "string"},
            "sella_attributed_motive": {"type": "string"},
            "misread_risk": {"type": "string"},
            "state_basis": {"type": "string"},
            "reviewed_consequences": {"type": "string"},
            "training_hooks": string_array_schema("Semantic labels for training."),
        },
    }
    return {
        "type": "function",
        "function": {
            "name": "record_sella_next_action",
            "description": "Record Sella's next action from updated local state.",
            "parameters": {
                "type": "object",
                "required": ["response"],
                "properties": {"response": response_schema},
            },
        },
    }


def load_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def write_text(path: Path, text: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text, encoding="utf-8")


def write_json(path: Path, payload: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")


def find_agent(fixture: dict[str, Any], agent_id: str) -> dict[str, Any]:
    return next(agent for agent in fixture["agents"] if agent["agent_id"] == agent_id)


def find_scene(fixture: dict[str, Any], scene_id: str) -> dict[str, Any]:
    return next(scene for scene in fixture["scenes"] if scene["scene_id"] == scene_id)


def find_pack(scene: dict[str, Any], speaker_id: str) -> dict[str, Any]:
    return next(pack for pack in scene["dialogue_context_packs"] if pack["speaker_agent_id"] == speaker_id)


def find_relationship(fixture: dict[str, Any], source_id: str, target_id: str) -> dict[str, Any]:
    return next(rel for rel in fixture["relationships"] if rel["source_id"] == source_id and rel["target_id"] == target_id)


def memory_summaries(agent: dict[str, Any], memory_ids: list[str]) -> list[str]:
    summaries: list[str] = []
    for group in agent["memories"].values():
        for memory in group:
            if memory["memory_id"] in memory_ids:
                summaries.append(f"{memory['memory_id']}: {memory['summary']}")
    return summaries


def goal_summaries(agent: dict[str, Any], goal_ids: list[str]) -> list[str]:
    return [
        f"{goal['goal_id']}: {goal['description']} Priority {goal['priority']}. Stake: {goal['emotional_stake']}"
        for goal in agent["goals"]
        if goal["goal_id"] in goal_ids
    ]


def selected_variables(agent: dict[str, Any]) -> dict[str, dict[str, float]]:
    groups = agent["canonical_state"]
    names_by_group = {
        "behavioral_dimensions": ["interpersonal_warmth", "control_pressure", "suspicion", "rigidity", "volatility"],
        "presentation_strategy": ["detachment", "competence_theater", "strategic_opacity", "ironic_distance"],
        "voice_style": ["formality", "lexical_precision", "ritualized_address", "verbal_warmth", "pointedness"],
        "situational_state": ["exhaustion", "panic", "grievance_activation", "overstimulation"],
    }
    selected: dict[str, dict[str, float]] = {}
    for group_name, variable_names in names_by_group.items():
        selected[group_name] = {
            name: groups[group_name][name]["current_activation"]
            for name in variable_names
            if name in groups[group_name]
        }
    return selected


def post_json(endpoint: str, payload: dict[str, Any], timeout: int) -> dict[str, Any]:
    data = json.dumps(payload, ensure_ascii=False).encode("utf-8")
    request = urllib.request.Request(
        endpoint,
        data=data,
        headers={"Content-Type": "application/json"},
        method="POST",
    )
    with urllib.request.urlopen(request, timeout=timeout) as response:
        return json.loads(response.read().decode("utf-8"))


def extract_json_object(text: str) -> Any | None:
    stripped = text.strip()
    if stripped.startswith("{"):
        return json.loads(stripped)
    fenced = re.search(r"```(?:json)?\s*(\{.*?\})\s*```", stripped, re.DOTALL)
    if fenced:
        return json.loads(fenced.group(1))
    start = stripped.find("{")
    end = stripped.rfind("}")
    if start != -1 and end != -1 and end > start:
        return json.loads(stripped[start : end + 1])
    return None


def extract_tool_arguments(raw: dict[str, Any], expected_tool_name: str) -> tuple[Any | None, list[str]]:
    message = raw.get("message", {})
    tool_calls = message.get("tool_calls") or []
    notes: list[str] = []
    if not tool_calls:
        return None, ["model returned no tool_calls"]
    first_call = tool_calls[0]
    function = first_call.get("function", {})
    name = function.get("name")
    if name != expected_tool_name:
        notes.append(f"expected tool {expected_tool_name}, got {name}")
    arguments = function.get("arguments")
    if isinstance(arguments, str):
        try:
            arguments = json.loads(arguments)
        except json.JSONDecodeError as exc:
            return None, notes + [f"tool arguments parse error: {exc}"]
    if not isinstance(arguments, dict):
        return None, notes + ["tool arguments were not an object"]
    arguments, repair_notes = repair_tool_arguments(arguments)
    notes.extend(repair_notes)
    return arguments, notes


def repair_tool_arguments(arguments: dict[str, Any]) -> tuple[dict[str, Any], list[str]]:
    repaired = dict(arguments)
    notes: list[str] = []
    for key in ["choices", "response", "appraisal"]:
        value = repaired.get(key)
        if isinstance(value, str):
            try:
                repaired[key] = json.loads(value)
                notes.append(f"repaired stringified tool argument: {key}")
            except json.JSONDecodeError:
                notes.append(f"could not repair stringified tool argument: {key}")
    return repaired, notes


def request_qwen_tool(
    endpoint: str,
    model: str,
    prompt: str,
    tool: dict[str, Any],
    expected_tool_name: str,
    temperature: float,
    top_p: float,
    num_ctx: int,
    think: bool,
    timeout: int,
) -> tuple[dict[str, Any], str, Any, list[str]]:
    request_payload = {
        "model": model,
        "messages": [{"role": "user", "content": prompt}],
        "tools": [tool],
        "stream": False,
        "think": think,
        "keep_alive": "10m",
        "options": {"num_ctx": num_ctx, "temperature": temperature, "top_p": top_p},
    }
    raw = post_json(endpoint, request_payload, timeout)
    message = raw.get("message", {})
    response_text = json.dumps(
        {
            "content": message.get("content", ""),
            "thinking": message.get("thinking"),
            "tool_calls": message.get("tool_calls", []),
        },
        indent=2,
        ensure_ascii=False,
    )
    parsed, notes = extract_tool_arguments(raw, expected_tool_name)
    return raw, response_text, parsed, notes


def observable_action_payload(actor_id: str, selected_choice: dict[str, Any]) -> dict[str, Any]:
    return {
        "actor_id": actor_id,
        "action_type": selected_choice["action_type"],
        "observable_action": selected_choice["observable_action"],
        "spoken_text": selected_choice.get("spoken_text", ""),
    }


def build_maer_prompt(fixture: dict[str, Any], annotation: dict[str, Any]) -> str:
    scene_id = annotation["scene_id"]
    actor_id = annotation["protagonist_agent_id"]
    responder_id = annotation["npc_agent_ids"][0]
    scene = find_scene(fixture, scene_id)
    actor = find_agent(fixture, actor_id)
    pack = find_pack(scene, actor_id)
    rel_actor_to_responder = find_relationship(fixture, actor_id, responder_id)

    payload = {
        "task": "Generate candidate player choices for Maer only. Do not write Sella's response.",
        "output_contract": {
            "format": "Call the provided tool exactly once. Do not write plain assistant content.",
            "root_keys": ["choices"],
            "choice_count": 4,
            "valid_action_types": sorted(VALID_ACTION_TYPES),
            "choice_fields": [
                "branch_id",
                "ink_path",
                "choice_text",
                "action_type",
                "actor_intent_private",
                "observable_action",
                "spoken_text",
                "state_basis",
                "expected_consequence_surfaces",
                "training_hooks",
            ],
        },
        "hard_rules": [
            "Use only Maer's local awareness and scene-observable facts.",
            "Do not stringify nested arrays or objects inside tool arguments.",
            "Do not include Sella's private state, private motives, or future response.",
            "Do not resolve whether the corrupted packet contains a person.",
            "Include at least two non-speech choices.",
            f"Use action_type values only from this exact enum: {', '.join(sorted(VALID_ACTION_TYPES))}.",
            "The selected branch must be reducible to observable action for Sella.",
            "Use semantic training hooks, not branch ids.",
        ],
        "scene_observable_context": {
            "scene_id": scene_id,
            "location": scene["location"],
            "public_stakes": scene["public_stakes"],
        },
        "maer_local_awareness": {
            "identity": actor["identity"],
            "local_truth": pack["speaker_local_truth"],
            "beliefs": pack["speaker_beliefs"],
            "goals": goal_summaries(actor, pack["active_goals"]),
            "memories": memory_summaries(actor, pack["active_memories"]),
            "relationship_read": rel_actor_to_responder["summary"],
            "presentation_constraints": pack["presentation_constraints"],
            "selected_current_activation": selected_variables(actor),
        },
        "projection_controls_visible_to_maer_choice_generation": annotation["projection_controls"],
    }
    return (
        "You are Ghostlight's Maer-local choice generator. Call record_maer_choices exactly once.\n"
        "Do not write Sella's response or private interpretation.\n\n"
        + json.dumps(payload, indent=2, ensure_ascii=False)
    )


def sella_local_payload(fixture: dict[str, Any], annotation: dict[str, Any]) -> dict[str, Any]:
    scene_id = annotation["scene_id"]
    responder_id = annotation["npc_agent_ids"][0]
    actor_id = annotation["protagonist_agent_id"]
    scene = find_scene(fixture, scene_id)
    responder = find_agent(fixture, responder_id)
    pack = find_pack(scene, responder_id)
    rel_responder_to_actor = find_relationship(fixture, responder_id, actor_id)
    return {
        "identity": responder["identity"],
        "local_truth": pack["speaker_local_truth"],
        "beliefs": pack["speaker_beliefs"],
        "goals": goal_summaries(responder, pack["active_goals"]),
        "memories": memory_summaries(responder, pack["active_memories"]),
        "relationship_read": rel_responder_to_actor["summary"],
        "presentation_constraints": pack["presentation_constraints"],
        "selected_current_activation": selected_variables(responder),
    }


def build_sella_appraisal_prompt(fixture: dict[str, Any], annotation: dict[str, Any], selected_choice: dict[str, Any]) -> str:
    scene_id = annotation["scene_id"]
    responder_id = annotation["npc_agent_ids"][0]
    actor_id = annotation["protagonist_agent_id"]
    scene = find_scene(fixture, scene_id)
    payload = {
        "task": "Generate Sella's participant appraisal/consolidation for one observable Maer event.",
        "output_contract": {
            "format": "Call the provided tool exactly once. Do not write plain assistant content.",
            "root_keys": ["appraisal"],
            "appraisal_fields": [
                "responder_agent_id",
                "private_interpretation",
                "attributed_motive",
                "misread_risk",
                "immediate_state_deltas",
                "relationship_deltas",
                "belief_updates",
                "response_constraints",
            ],
        },
        "hard_rules": [
            "Use only Sella's local awareness plus the observable Maer action.",
            "Do not stringify nested arrays or objects inside tool arguments.",
            "Do not read Maer's private intent. Treat actor intent as unavailable.",
            "Sella may correctly read, partially read, or misread Maer's action.",
            "Do not resolve whether the corrupted packet contains a person.",
            "This pass proposes reviewed state changes for Sella caused by the event. Do not write Sella's next action.",
            "Keep deltas small and bounded between -0.20 and 0.20.",
        ],
        "scene_observable_context": {
            "scene_id": scene_id,
            "location": scene["location"],
            "public_stakes": scene["public_stakes"],
        },
        "observable_maer_action": observable_action_payload(actor_id, selected_choice),
        "sella_local_awareness": sella_local_payload(fixture, annotation),
        "projection_controls_visible_to_sella_appraisal": annotation["projection_controls"],
    }
    return (
        "You are Ghostlight's Sella-local appraisal generator. Call record_sella_appraisal exactly once.\n"
        "Do not use Maer's private intent. Do not write her next action.\n\n"
        + json.dumps(payload, indent=2, ensure_ascii=False)
    )


def build_sella_prompt(
    fixture: dict[str, Any],
    annotation: dict[str, Any],
    selected_choice: dict[str, Any],
    appraisal: dict[str, Any] | None,
) -> str:
    scene_id = annotation["scene_id"]
    responder_id = annotation["npc_agent_ids"][0]
    actor_id = annotation["protagonist_agent_id"]
    scene = find_scene(fixture, scene_id)
    payload = {
        "task": "Generate Sella's next action from Sella-local awareness after applying reviewed event appraisal.",
        "output_contract": {
            "format": "Call the provided tool exactly once. Do not write plain assistant content.",
            "root_keys": ["response"],
            "valid_action_types": sorted(VALID_ACTION_TYPES),
            "response_fields": [
                "responder_agent_id",
                "action_type",
                "observable_response",
                "spoken_text",
                "sella_private_interpretation",
                "sella_attributed_motive",
                "misread_risk",
                "state_basis",
                "reviewed_consequences",
                "training_hooks",
            ],
        },
        "hard_rules": [
            "Use only Sella's local awareness, the observable Maer event, and reviewed Sella state changes from that event.",
            "Do not stringify nested arrays or objects inside tool arguments.",
            "Do not read Maer's private intent. Treat actor intent as unavailable.",
            "Sella may correctly read, partially read, or misread Maer's action.",
            "Do not resolve whether the corrupted packet contains a person.",
            "Do not accept without cost, condition, authority defense, or resentment risk.",
            f"Use action_type values only from this exact enum: {', '.join(sorted(VALID_ACTION_TYPES))}.",
            "Use semantic training hooks, not branch ids.",
        ],
        "scene_observable_context": {
            "scene_id": scene_id,
            "location": scene["location"],
            "public_stakes": scene["public_stakes"],
        },
        "observable_maer_action": observable_action_payload(actor_id, selected_choice),
        "sella_immediate_appraisal": appraisal,
        "sella_local_awareness": sella_local_payload(fixture, annotation),
        "projection_controls_visible_to_sella_response_generation": annotation["projection_controls"],
    }
    return (
        "You are Ghostlight's Sella-local next-action generator. Call record_sella_next_action exactly once.\n"
        "Use reviewed event appraisal as the changed state she is acting from.\n\n"
        + json.dumps(payload, indent=2, ensure_ascii=False)
    )


def validate_choice_payload(parsed: Any) -> list[str]:
    notes: list[str] = []
    if not isinstance(parsed, dict):
        return ["choice response was not an object"]
    choices = parsed.get("choices")
    if not isinstance(choices, list):
        return ["choice response missing choices array"]
    if len(choices) != 4:
        notes.append(f"expected 4 choices, got {len(choices)}")
    required = ["branch_id", "ink_path", "choice_text", "action_type", "actor_intent_private", "observable_action", "state_basis", "expected_consequence_surfaces", "training_hooks"]
    non_speech = 0
    for index, choice in enumerate(choices):
        if not isinstance(choice, dict):
            notes.append(f"choices[{index}] was not an object")
            continue
        missing = [key for key in required if key not in choice]
        if missing:
            notes.append(f"choices[{index}] missing keys: {', '.join(missing)}")
        if choice.get("action_type") not in VALID_ACTION_TYPES:
            notes.append(f"choices[{index}] invalid action_type")
        for list_key in ["expected_consequence_surfaces", "training_hooks"]:
            if not isinstance(choice.get(list_key), list):
                notes.append(f"choices[{index}] {list_key} must be an array")
        if choice.get("action_type") != "speak":
            non_speech += 1
    if non_speech < 2:
        notes.append("choices had fewer than two non-speech actions")
    return notes


def validate_appraisal_payload(parsed: Any) -> list[str]:
    if not isinstance(parsed, dict):
        return ["appraisal was not an object"]
    appraisal = parsed.get("appraisal")
    if not isinstance(appraisal, dict):
        return ["appraisal missing appraisal object"]
    required = [
        "responder_agent_id",
        "private_interpretation",
        "attributed_motive",
        "misread_risk",
        "immediate_state_deltas",
        "relationship_deltas",
        "response_constraints",
    ]
    missing = [key for key in required if key not in appraisal]
    notes = [f"appraisal missing keys: {', '.join(missing)}"] if missing else []
    for list_key in ["immediate_state_deltas", "relationship_deltas", "response_constraints"]:
        if list_key in appraisal and not isinstance(appraisal[list_key], list):
            notes.append(f"appraisal.{list_key} must be an array")
    return notes


def validate_response_payload(parsed: Any) -> list[str]:
    if not isinstance(parsed, dict):
        return ["response was not an object"]
    response = parsed.get("response")
    if not isinstance(response, dict):
        return ["response missing response object"]
    required = ["responder_agent_id", "action_type", "observable_response", "sella_private_interpretation", "misread_risk", "state_basis", "reviewed_consequences", "training_hooks"]
    missing = [key for key in required if key not in response]
    notes = [f"response missing keys: {', '.join(missing)}"] if missing else []
    if response.get("action_type") not in VALID_ACTION_TYPES:
        notes.append("response invalid action_type")
    if "training_hooks" in response and not isinstance(response["training_hooks"], list):
        notes.append("response.training_hooks must be an array")
    return notes


def choose_selected_choice(parsed: Any, preferred_action: str) -> dict[str, Any]:
    choices = parsed.get("choices", []) if isinstance(parsed, dict) else []
    for choice in choices:
        if isinstance(choice, dict) and choice.get("action_type") == preferred_action:
            return choice
    for choice in choices:
        if isinstance(choice, dict):
            return choice
    raise SystemExit("No choices available to select")


def main() -> int:
    parser = argparse.ArgumentParser(description="Run a sequential local-awareness Qwen Ink prototype.")
    parser.add_argument("--fixture", type=Path, default=DEFAULT_FIXTURE)
    parser.add_argument("--annotation", type=Path, default=DEFAULT_ANNOTATION)
    parser.add_argument("--out-dir", type=Path, default=DEFAULT_OUT_DIR)
    parser.add_argument("--endpoint", default=DEFAULT_ENDPOINT)
    parser.add_argument("--model", default="qwen3.5:9b")
    parser.add_argument("--temperature", type=float, default=0.55)
    parser.add_argument("--top-p", type=float, default=0.9)
    parser.add_argument("--num-ctx", type=int, default=8192)
    parser.add_argument("--request-timeout", type=int, default=600)
    parser.add_argument("--think", action=argparse.BooleanOptionalAction, default=True)
    parser.add_argument("--capture-id", default="cold-wake-sanctuary-intake.qwen-sequential.v1")
    parser.add_argument("--preferred-action", default="show_object")
    args = parser.parse_args()

    fixture = load_json(args.fixture)
    annotation = load_json(args.annotation)
    maer_prompt = build_maer_prompt(fixture, annotation)
    maer_raw, maer_response_text, maer_parsed, maer_notes = request_qwen_tool(
        args.endpoint,
        args.model,
        maer_prompt,
        maer_choices_tool(),
        "record_maer_choices",
        args.temperature,
        args.top_p,
        args.num_ctx,
        args.think,
        args.request_timeout,
    )
    maer_notes.extend(validate_choice_payload(maer_parsed))
    selected_choice = choose_selected_choice(maer_parsed, args.preferred_action)

    sella_appraisal_prompt = build_sella_appraisal_prompt(fixture, annotation, selected_choice)
    sella_appraisal_raw, sella_appraisal_response_text, sella_appraisal_parsed, sella_appraisal_notes = request_qwen_tool(
        args.endpoint,
        args.model,
        sella_appraisal_prompt,
        sella_appraisal_tool(),
        "record_sella_appraisal",
        args.temperature,
        args.top_p,
        args.num_ctx,
        args.think,
        args.request_timeout,
    )
    sella_appraisal_parsed = {"appraisal": sella_appraisal_parsed} if isinstance(sella_appraisal_parsed, dict) else sella_appraisal_parsed
    sella_appraisal_notes.extend(validate_appraisal_payload(sella_appraisal_parsed))
    appraisal = sella_appraisal_parsed.get("appraisal") if isinstance(sella_appraisal_parsed, dict) else None

    sella_prompt = build_sella_prompt(fixture, annotation, selected_choice, appraisal)
    sella_raw, sella_response_text, sella_parsed, sella_notes = request_qwen_tool(
        args.endpoint,
        args.model,
        sella_prompt,
        sella_next_action_tool(),
        "record_sella_next_action",
        args.temperature,
        args.top_p,
        args.num_ctx,
        args.think,
        args.request_timeout,
    )
    sella_notes.extend(validate_response_payload(sella_parsed))

    all_notes = maer_notes + sella_appraisal_notes + sella_notes
    capture = {
        "schema_version": "ghostlight.qwen_ink_sequential_capture.v0",
        "capture_id": args.capture_id,
        "source_fixture_ref": str(args.fixture.relative_to(ROOT)).replace("\\", "/"),
        "source_annotation_ref": str(args.annotation.relative_to(ROOT)).replace("\\", "/"),
        "model": maer_raw.get("model", args.model),
        "endpoint": args.endpoint,
        "request_options": {
            "think": args.think,
            "api": "ollama_chat_tools",
            "num_ctx": args.num_ctx,
            "request_timeout": args.request_timeout,
            "temperature": args.temperature,
            "top_p": args.top_p,
        },
        "passes": {
            "maer_choice_generation": {
                "local_agent_id": "maer_tidecall",
                "prompt_text": maer_prompt,
                "response_text": maer_response_text,
                "parsed_response": maer_parsed,
                "validation_notes": maer_notes,
            },
            "selected_observable_action": selected_choice,
            "sella_immediate_appraisal": {
                "local_agent_id": "sella_ren",
                "prompt_text": sella_appraisal_prompt,
                "response_text": sella_appraisal_response_text,
                "parsed_response": sella_appraisal_parsed,
                "validation_notes": sella_appraisal_notes,
            },
            "sella_response_generation": {
                "local_agent_id": "sella_ren",
                "prompt_text": sella_prompt,
                "response_text": sella_response_text,
                "parsed_response": sella_parsed,
                "validation_notes": sella_notes,
            },
        },
        "review": {
            "status": "accepted_as_draft" if not all_notes else "useful_needs_revision",
            "strengths": [
                "Separated Maer choice generation from Sella next-action generation.",
                "Inserted participant appraisal/consolidation before next-actor action generation.",
                "Sella prompt received observable Maer event and reviewed Sella state changes, not Maer's private intent.",
            ],
            "failure_notes": all_notes,
            "accepted_into_ink_ref": None,
            "pipeline_lessons": [
                "Sequential generation needs event appraisal/consolidation before next actor selection.",
                "Action type selection should be tool-shaped or schema-constrained, not left to prose compliance.",
                "Next pass should materialize the sequential capture into Ink and mutation training data.",
            ],
        },
    }

    args.out_dir.mkdir(parents=True, exist_ok=True)
    base = args.capture_id.replace(".", "-")
    capture_path = args.out_dir / f"{base}.capture.json"
    maer_prompt_path = args.out_dir / f"{base}.maer.prompt.md"
    maer_response_path = args.out_dir / f"{base}.maer.response.md"
    sella_appraisal_prompt_path = args.out_dir / f"{base}.sella-appraisal.prompt.md"
    sella_appraisal_response_path = args.out_dir / f"{base}.sella-appraisal.response.md"
    sella_prompt_path = args.out_dir / f"{base}.sella.prompt.md"
    sella_response_path = args.out_dir / f"{base}.sella.response.md"
    write_json(capture_path, capture)
    write_text(maer_prompt_path, maer_prompt + "\n")
    write_text(maer_response_path, maer_response_text + "\n")
    write_text(sella_appraisal_prompt_path, sella_appraisal_prompt + "\n")
    write_text(sella_appraisal_response_path, sella_appraisal_response_text + "\n")
    write_text(sella_prompt_path, sella_prompt + "\n")
    write_text(sella_response_path, sella_response_text + "\n")
    print(f"wrote {capture_path.relative_to(ROOT)}")
    print(json.dumps({"selected_observable_action": selected_choice, "sella_appraisal": sella_appraisal_parsed, "sella_response": sella_parsed}, indent=2, ensure_ascii=False))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
