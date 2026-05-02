"""Generate Ink branch candidates from a Ghostlight local-awareness packet.

This is the first honest local-awareness-to-branch-generation seam. Qwen
proposes branches; humans/Ghostlight review them before they become accepted
Ink content or state mutation.
"""

from __future__ import annotations

import argparse
import json
import re
import urllib.request
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
DEFAULT_ENDPOINT = "http://192.168.1.84:11434/api/generate"
DEFAULT_FIXTURE = ROOT / "examples" / "agent-state.cold-wake-story-lab.json"
DEFAULT_ANNOTATION = ROOT / "examples" / "ink" / "cold-wake-sanctuary-intake.training.json"
DEFAULT_OUT_DIR = ROOT / "experiments" / "ink"


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


def find_pack(scene: dict[str, Any], actor_id: str) -> dict[str, Any]:
    return next(pack for pack in scene["dialogue_context_packs"] if pack["speaker_agent_id"] == actor_id)


def find_relationship(fixture: dict[str, Any], source_id: str, target_id: str) -> dict[str, Any]:
    return next(rel for rel in fixture["relationships"] if rel["source_id"] == source_id and rel["target_id"] == target_id)


def memory_summaries(agent: dict[str, Any], memory_ids: list[str]) -> list[str]:
    summaries: list[str] = []
    for memory_group in agent["memories"].values():
        for memory in memory_group:
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
            variable_name: groups[group_name][variable_name]["current_activation"]
            for variable_name in variable_names
            if variable_name in groups[group_name]
        }
    return selected


def build_prompt(fixture: dict[str, Any], annotation: dict[str, Any]) -> str:
    scene_id = annotation["scene_id"]
    actor_id = annotation["protagonist_agent_id"]
    npc_ids = annotation["npc_agent_ids"]
    scene = find_scene(fixture, scene_id)
    actor = find_agent(fixture, actor_id)
    pack = find_pack(scene, actor_id)
    npcs = [find_agent(fixture, npc_id) for npc_id in npc_ids]
    relationships = []
    for npc_id in npc_ids:
        relationships.append(find_relationship(fixture, actor_id, npc_id)["summary"])
        relationships.append(find_relationship(fixture, npc_id, actor_id)["summary"])

    payload = {
        "task": "Generate candidate Ink branches for one Aetheria interactive fiction scene.",
        "output_contract": {
            "format": "Return only JSON. No markdown. No commentary.",
            "root_keys": ["branches"],
            "branch_count": 4,
            "branch_fields": [
                "branch_id",
                "ink_path",
                "choice_text",
                "action_type",
                "actor_intent",
                "maer_action_prose",
                "maer_spoken_text",
                "sella_response_prose",
                "sella_spoken_text",
                "state_basis",
                "reviewed_consequences",
                "training_hooks",
            ],
            "valid_action_types": [
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
            ],
        },
        "rules": [
            "Generate player-facing choices for Maer, not a linear story.",
            "Include at least two non-speech action types.",
            "Do not resolve whether the corrupted packet contains a person.",
            "Do not let Sella accept without cost, condition, authority defense, or resentment risk.",
            "Keep Sella's response separate from Maer's intent; she may misread or bound his move.",
            "Do not invent new lore facts beyond the provided context.",
            "Make each branch playable in Ink as a knot reached from a choice.",
            "Use concise, readable prose. The scene should be usable Aetheria lore content, not a schema wearing boots.",
        ],
        "scene": {
            "scene_id": scene_id,
            "location": scene["location"],
            "public_stakes": scene["public_stakes"],
            "hidden_stakes": scene["hidden_stakes"],
        },
        "actor": {
            "agent_id": actor_id,
            "identity": actor["identity"],
            "local_truth": pack["speaker_local_truth"],
            "beliefs": pack["speaker_beliefs"],
            "goals": goal_summaries(actor, pack["active_goals"]),
            "memories": memory_summaries(actor, pack["active_memories"]),
            "presentation_constraints": pack["presentation_constraints"],
            "selected_current_activation": selected_variables(actor),
        },
        "npcs": [
            {
                "agent_id": npc["agent_id"],
                "identity": npc["identity"],
                "selected_current_activation": selected_variables(npc),
            }
            for npc in npcs
        ],
        "relationships": relationships,
        "projection_controls": annotation["projection_controls"],
        "existing_human_authored_branches_for_reference_not_copying": [
            branch["branch_id"] for branch in annotation["branches"]
        ],
    }
    return (
        "You are helping Ghostlight generate branch candidates for an Ink interactive fiction scene.\n"
        "Return only valid JSON matching the provided contract.\n\n"
        + json.dumps(payload, indent=2, ensure_ascii=False)
    )


def post_json(endpoint: str, payload: dict[str, Any]) -> dict[str, Any]:
    data = json.dumps(payload, ensure_ascii=False).encode("utf-8")
    request = urllib.request.Request(
        endpoint,
        data=data,
        headers={"Content-Type": "application/json"},
        method="POST",
    )
    with urllib.request.urlopen(request, timeout=180) as response:
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


def validate_generated_payload(payload: Any) -> list[str]:
    notes: list[str] = []
    if not isinstance(payload, dict):
        return ["response was not a JSON object"]
    branches = payload.get("branches")
    if not isinstance(branches, list):
        return ["response did not contain a branches array"]
    if len(branches) != 4:
        notes.append(f"expected 4 branches, got {len(branches)}")
    non_speech_count = 0
    required = [
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
    ]
    for index, branch in enumerate(branches):
        if not isinstance(branch, dict):
            notes.append(f"branches[{index}] was not an object")
            continue
        missing = [key for key in required if key not in branch]
        if missing:
            notes.append(f"branches[{index}] missing keys: {', '.join(missing)}")
        if branch.get("action_type") != "speak":
            non_speech_count += 1
    if non_speech_count < 2:
        notes.append("response had fewer than two non-speech branches")
    return notes


def main() -> int:
    parser = argparse.ArgumentParser(description="Ask Qwen for Ink branch candidates from Ghostlight state.")
    parser.add_argument("--fixture", type=Path, default=DEFAULT_FIXTURE)
    parser.add_argument("--annotation", type=Path, default=DEFAULT_ANNOTATION)
    parser.add_argument("--out-dir", type=Path, default=DEFAULT_OUT_DIR)
    parser.add_argument("--endpoint", default=DEFAULT_ENDPOINT)
    parser.add_argument("--model", default="qwen3.5:9b")
    parser.add_argument("--temperature", type=float, default=0.65)
    parser.add_argument("--top-p", type=float, default=0.9)
    parser.add_argument("--num-ctx", type=int, default=8192)
    parser.add_argument("--capture-id", default="cold-wake-sanctuary-intake.qwen-branch-candidates.v1")
    args = parser.parse_args()

    fixture = load_json(args.fixture)
    annotation = load_json(args.annotation)
    prompt = build_prompt(fixture, annotation)
    request_payload = {
        "model": args.model,
        "prompt": prompt,
        "stream": False,
        "think": False,
        "keep_alive": "10m",
        "options": {
            "num_ctx": args.num_ctx,
            "temperature": args.temperature,
            "top_p": args.top_p,
        },
    }
    raw = post_json(args.endpoint, request_payload)
    response_text = raw.get("response", "").strip()
    parsed_payload = None
    parse_error = None
    try:
        parsed_payload = extract_json_object(response_text)
    except (json.JSONDecodeError, TypeError) as exc:
        parse_error = str(exc)

    validation_notes = validate_generated_payload(parsed_payload) if parsed_payload is not None else ["could not parse JSON"]
    if parse_error:
        validation_notes.append(f"parse error: {parse_error}")

    capture = {
        "schema_version": "ghostlight.qwen_ink_branch_capture.v0",
        "capture_id": args.capture_id,
        "source_fixture_ref": str(args.fixture.relative_to(ROOT)),
        "source_annotation_ref": str(args.annotation.relative_to(ROOT)),
        "model": raw.get("model", args.model),
        "endpoint": args.endpoint,
        "request_options": {
            "think": False,
            "num_ctx": args.num_ctx,
            "temperature": args.temperature,
            "top_p": args.top_p,
        },
        "prompt_text": prompt,
        "response_text": response_text,
        "parsed_response": parsed_payload,
        "review": {
            "status": "needs_review",
            "strengths": [],
            "failure_notes": validation_notes,
            "accepted_into_ink_ref": None,
            "pipeline_lessons": [],
        },
    }

    args.out_dir.mkdir(parents=True, exist_ok=True)
    base = args.capture_id.replace(".", "-")
    capture_path = args.out_dir / f"{base}.capture.json"
    prompt_path = args.out_dir / f"{base}.prompt.md"
    response_path = args.out_dir / f"{base}.response.md"
    write_json(capture_path, capture)
    write_text(prompt_path, prompt + "\n")
    write_text(response_path, response_text + "\n")
    print(f"wrote {capture_path.relative_to(ROOT)}")
    print(response_text)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
