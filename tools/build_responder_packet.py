"""Build sandboxed Ghostlight responder packets.

A responder packet is the exact surface a character worker/model may see. It is
not a coordinator artifact, not a lore dump, and not an excuse to smuggle private
state through the side door wearing a little hat.
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
DEFAULT_COORDINATOR = ROOT / "examples" / "coordinator" / "cold-wake-sanctuary-intake.v0.json"
DEFAULT_PROJECTED = ROOT / "examples" / "projected-contexts" / "scene-02-sanctuary-intake.sella_ren.projected-context.json"
DEFAULT_OUT = ROOT / "examples" / "responder-packets" / "scene-02-sanctuary-intake.sella_ren.packet.v1.json"

ALLOWED_ACTION_LABELS = [
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
]


def load_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8-sig"))


def write_json(path: Path, payload: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")


def render_packet_prompt(packet: dict[str, Any]) -> str:
    context = packet["visible_context"]
    event = context["observed_event"]
    contract = packet["output_contract"]
    lore_access = packet["lore_access"]
    lines = [
        "# Ghostlight Sandboxed Responder Packet",
        "",
        "You are acting only as the named responder. Use only this packet.",
        "Do not treat guesses about another character's intent as fact.",
        "If you need to guess intent, mark it as this character's interpretation, not truth.",
        "",
    ]
    if lore_access["mode"] == "responder_scoped_repository_search":
        lines.extend([
            "## Required Lore Research",
            "Before answering, inspect the declared lore scope and ground your action in what those sources make true or likely.",
            "Use lore to constrain behavior, affordances, institutions, vocabulary, and material stakes. Do not use it to gain hidden character state, author-only answers, or future branch knowledge.",
            "If a scoped source contradicts the packet, follow the packet for character-local knowledge and flag the contradiction in unresolved hooks.",
            "Your output capture must list consulted refs and a brief research summary; an answer with no consulted refs is a failed research-enabled response.",
            "",
            "Allowed research scope:",
        ])
        for scope in lore_access["allowed_scope"]:
            lines.append(f"- {scope}")
        if lore_access.get("research_instructions"):
            lines.extend(["", "Research instructions:"])
            for instruction in lore_access["research_instructions"]:
                lines.append(f"- {instruction}")
        lines.append("")
    lines.extend([
        "## Local Operating Context",
        context["local_context_prompt"],
        "",
        "## Observed Event",
        f"- Event id: {event['event_id']}",
        f"- Actor: {event['actor_agent_id']}",
        f"- Observable action: {event['observable_action']}",
        f"- Spoken text: {event['spoken_text']}",
    ])
    for note in event["visibility_notes"]:
        lines.append(f"- Visibility note: {note}")
    lines.extend([
        "",
        "## Allowed Action Labels",
        ", ".join(context["allowed_action_labels"]),
        "",
        "## Source Excerpts",
    ])
    for excerpt in context["source_excerpts"]:
        lines.append(f"- {excerpt['ref_id']}: {excerpt['text']}")
    lines.extend([
        "",
        "## Output Contract",
        contract["response_schema"],
        "",
        "Required fields: " + ", ".join(contract["required_fields"]),
        "",
        "Rules:",
    ])
    for rule in contract["response_rules"]:
        lines.append(f"- {rule}")
    return "\n".join(lines).strip()


def build_source_excerpts(projected: dict[str, Any]) -> list[dict[str, str]]:
    context = projected["context"]
    excerpts = [
        {
            "ref_id": "local_stakes.capacity",
            "source_ref": "examples/projected-contexts/scene-02-sanctuary-intake.sella_ren.projected-context.json#context.current_stakes",
            "text": "Sanctuary beds and repair capacity are nearly exhausted.",
        },
        {
            "ref_id": "local_rule.care_capacity",
            "source_ref": "examples/projected-contexts/scene-02-sanctuary-intake.sella_ren.projected-context.json#context.active_inner_pressures",
            "text": "Care is not a feeling; it is beds, food, air, consent, repair, and someone still doing the list when the room hates them.",
        },
        {
            "ref_id": "local_boundary.packet_unresolved",
            "source_ref": "examples/projected-contexts/scene-02-sanctuary-intake.sella_ren.projected-context.json#context.do_not_invent",
            "text": "Do not resolve whether the corrupted packet contains a person.",
        },
    ]
    for item in context.get("runtime_retrieval_requirements", []):
        if item.get("prompt_role") != "source_excerpt":
            continue
        excerpts.append(
            {
                "ref_id": item["requirement_id"],
                "source_ref": "; ".join(item["source_refs"]),
                "text": item["compact_fact"],
            }
        )
    return excerpts


def build_packet(coordinator: dict[str, Any], projected: dict[str, Any], out_ref: str) -> dict[str, Any]:
    responder_agent_id = projected["input"]["local_agent_id"]
    packet = {
        "schema_version": "ghostlight.responder_packet.v0",
        "packet_id": "packet.cold_wake.sanctuary_intake.sella_response.v1",
        "review_status": "accepted_as_draft",
        "fixture_lane": coordinator["fixture_lane"],
        "canon_status": coordinator["canon_status"],
        "coordinator_artifact_ref": "examples/coordinator/cold-wake-sanctuary-intake.v0.json",
        "projected_context_ref": "examples/projected-contexts/scene-02-sanctuary-intake.sella_ren.projected-context.json",
        "responder_agent_id": responder_agent_id,
        "generation_lane": "packet_only",
        "lore_access": {
            "mode": "curated_excerpts_only",
            "allowed_scope": [
                "AetheriaLore:Aetheria/Worldbuilding/Pre-Elysium/Timeline/Events/Cold Wake Panic.md",
                "AetheriaLore:Aetheria/Worldbuilding/Pre-Elysium/Timeline/Events/Ganymede Route Compact.md",
                "AetheriaLore:Aetheria/Worldbuilding/Pre-Elysium/Factions/Powers/Major/Cetacean Navigators.md",
                "AetheriaLore:Aetheria/Worldbuilding/Pre-Elysium/Factions/Powers/Minor/Lightsail Express.md",
            ],
            "required_provenance": True,
        },
        "visible_context": {
            "local_context_prompt": projected["context"]["prompt_text"],
            "observed_event": {
                "event_id": "event.maer.packet_display_for_triage",
                "actor_agent_id": "maer_tidecall",
                "observable_action": "Maer routes the corrupted distress packet through a shared clinic display so Sella can inspect routing terms, telemetry gaps, and ledger implications without treating the packet as clean proof.",
                "spoken_text": "Here. The telemetry is incomplete, but the routing logic suggests a distress pattern. It is not proof of a person, but it is not noise either. Decide if the math supports intake.",
                "visible_object_refs": ["corrupted_distress_packet", "shared_clinic_display", "sanctuary_ledger"],
                "visibility_notes": [
                    "Sella can observe Maer's displayed packet evidence and spoken request.",
                    "The packet remains unresolved evidence, not proof of personhood."
                ],
            },
            "allowed_action_labels": ALLOWED_ACTION_LABELS,
            "source_excerpts": build_source_excerpts(projected),
        },
        "output_contract": {
            "response_schema": "Return one JSON object. Do not wrap it in Markdown. The object describes this responder's next visible action and optional speech, plus private interpretation clearly labeled as interpretation.",
            "required_fields": [
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
            "response_rules": [
                "Use exactly one allowed action label.",
                "The visible action must be observable by other characters in the scene.",
                "Spoken text may be empty if silence or action is the response.",
                "Private interpretation may describe what Sella suspects or feels, but not what Maer truly intended.",
                "State and relationship updates are candidates only; the coordinator or mutator decides what becomes canonical.",
                "Do not prove or disprove packet personhood.",
                "Do not copy instruction text into in-world speech.",
            ],
        },
        "hidden_context_audit": {
            "omitted_context_refs": [
                "maer_tidecall.canonical_private_state",
                "coordinator_artifact.coordinator_decision.rationale",
                "author_only.packet_personhood_resolution",
                "future_branch_plans",
            ],
            "forbidden_inference_classes": [
                "other_agent_private_state_as_fact",
                "author_only_resolution",
                "future_branch_knowledge",
                "raw_numeric_state_variables",
                "coordinator_strategy",
            ],
            "known_absences": [
                "No hidden proof of packet personhood is present in this packet.",
                "No hidden Sella acceptance outcome is present in this packet.",
                "No omniscient narrator guidance is present in this packet.",
            ],
        },
        "isolation_requirements": {
            "isolation_method": "manual_packet_only",
            "packet_only": True,
            "no_repo_access": True,
            "no_conversation_context": True,
            "no_hidden_state_refs": True,
        },
        "packet_prompt_text": "",
        "review": {
            "reviewer": "Codex",
            "review_notes": [
                "Second responder packet draft for sandboxed gold-data generation.",
                "Adds explicit packet_only lane labels and curated AetheriaLore excerpts while keeping absent hidden context in audit fields, not responder-visible prose.",
            ],
            "accepted_for_sandbox_use": True,
            "failure_labels": [],
        },
    }
    packet["packet_prompt_text"] = render_packet_prompt(packet)
    return packet


def main() -> int:
    parser = argparse.ArgumentParser(description="Build a sandboxed Ghostlight responder packet.")
    parser.add_argument("--coordinator", type=Path, default=DEFAULT_COORDINATOR)
    parser.add_argument("--projected-context", type=Path, default=DEFAULT_PROJECTED)
    parser.add_argument("--out", type=Path, default=DEFAULT_OUT)
    args = parser.parse_args()

    coordinator = load_json(args.coordinator)
    projected = load_json(args.projected_context)
    out_ref = args.out.relative_to(ROOT).as_posix() if args.out.is_relative_to(ROOT) else str(args.out)
    packet = build_packet(coordinator, projected, out_ref)
    write_json(args.out, packet)
    print(f"wrote {args.out.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
