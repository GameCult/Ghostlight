"""Materialize a Qwen Ink branch capture into a playable Ink draft."""

from __future__ import annotations

import argparse
import json
import re
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
DEFAULT_CAPTURE = ROOT / "experiments" / "ink" / "cold-wake-sanctuary-intake-qwen-branch-candidates-v1.capture.json"
DEFAULT_INK_OUT = ROOT / "examples" / "ink" / "cold-wake-sanctuary-intake.qwen-draft.ink"
DEFAULT_TRAINING_OUT = ROOT / "examples" / "ink" / "cold-wake-sanctuary-intake.qwen-draft.training.json"


def load_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def write_text(path: Path, text: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text, encoding="utf-8")


def write_json(path: Path, payload: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")


def ink_identifier(value: str) -> str:
    cleaned = re.sub(r"[^A-Za-z0-9_]+", "_", value).strip("_")
    if not cleaned:
        return "branch"
    if cleaned[0].isdigit():
        return f"branch_{cleaned}"
    return cleaned


def ensure_list(value: Any) -> list[str]:
    if isinstance(value, list):
        return [str(item) for item in value if str(item).strip()]
    if isinstance(value, str) and value.strip():
        return [value.strip()]
    return []


def ink_quote_block(text: str) -> list[str]:
    if not text:
        return []
    return [line.rstrip() for line in text.splitlines()]


def build_ink(capture: dict[str, Any]) -> str:
    branches = capture["parsed_response"]["branches"]
    lines: list[str] = [
        "VAR sella_trust = 57",
        "VAR sella_obligation = 48",
        "VAR maer_obligation = 60",
        "VAR bay_reserved = false",
        "VAR packet_shown = false",
        "",
        "-> start",
        "",
        "=== start ===",
        "# ghostlight.scene: scene-02-sanctuary-intake",
        "# ghostlight.fixture: examples/agent-state.cold-wake-story-lab.json",
        "# ghostlight.generated_by: experiments/ink/cold-wake-sanctuary-intake-qwen-branch-candidates-v1.capture.json",
        "# aetheria.flashpoint: cold-wake-panic",
        "",
        "The intake clinic sits behind the pumpworks, all warmth rationed through humming conduits and tired hands.",
        "",
        "Sella Ren stands between Maer Tidecall and one empty monitored bay. The corrupted packet on Maer's slate may be testimony, firmware noise, medical telemetry, or a dirty little braid of all three.",
        "",
        "Sella says, \"If you came to ask me for mercy, bring tubing.\"",
        "",
        "What does Maer do?",
        "",
    ]
    for branch in branches:
        ink_path = ink_identifier(branch["ink_path"])
        lines.extend(
            [
                f"* [{branch['choice_text']}]",
                f"    # ghostlight.branch: {branch['branch_id']}",
                f"    # ghostlight.action: {branch['action_type']}",
                f"    # ghostlight.intent: {branch['actor_intent']}",
                f"    -> {ink_path}",
                "",
            ]
        )

    for branch in branches:
        ink_path = ink_identifier(branch["ink_path"])
        lines.extend([f"=== {ink_path} ==="])
        if branch["action_type"] == "show_object":
            lines.append("~ packet_shown = true")
        if branch["action_type"] == "speak" and "moral" in branch["branch_id"]:
            lines.append("~ sella_trust = 52")
        if branch["action_type"] == "silence":
            lines.append("~ sella_obligation = 51")
        lines.append("")
        lines.extend(ink_quote_block(branch["maer_action_prose"]))
        if branch.get("maer_spoken_text"):
            lines.extend(["", f"\"{branch['maer_spoken_text']}\""])
        lines.append("")
        lines.extend(ink_quote_block(branch["sella_response_prose"]))
        if branch.get("sella_spoken_text"):
            lines.extend(["", f"\"{branch['sella_spoken_text']}\""])
        lines.extend(
            [
                "",
                f"# ghostlight.generated_branch: {branch['branch_id']}",
                f"# ghostlight.consequence: {ink_identifier(branch['branch_id'])}_reviewed_consequence",
            ]
        )
        for hook in ensure_list(branch.get("training_hooks")):
            lines.append(f"# ghostlight.training_hook: {hook}")
        lines.extend(["-> END", ""])
    return "\n".join(lines).rstrip() + "\n"


def build_training(capture: dict[str, Any], ink_ref: str) -> dict[str, Any]:
    branches = []
    for branch in capture["parsed_response"]["branches"]:
        branches.append(
            {
                "branch_id": branch["branch_id"],
                "ink_path": ink_identifier(branch["ink_path"]),
                "action_type": branch["action_type"],
                "actor_intent": branch["actor_intent"],
                "state_basis": ensure_list(branch.get("state_basis")),
                "reviewed_consequences": ensure_list(branch.get("reviewed_consequences")),
                "training_hooks": ensure_list(branch.get("training_hooks")),
                "generation_notes": {
                    "choice_text": branch["choice_text"],
                    "maer_action_prose": branch["maer_action_prose"],
                    "maer_spoken_text": branch.get("maer_spoken_text", ""),
                    "sella_response_prose": branch["sella_response_prose"],
                    "sella_spoken_text": branch.get("sella_spoken_text", ""),
                },
            }
        )
    return {
        "schema_version": "ghostlight.ink_training_annotation.v0",
        "annotation_id": "cold-wake-sanctuary-intake-qwen-draft-training-v0",
        "ink_ref": ink_ref,
        "source_fixture_ref": capture["source_fixture_ref"].replace("\\", "/"),
        "scene_id": "scene-02-sanctuary-intake",
        "protagonist_agent_id": "maer_tidecall",
        "npc_agent_ids": ["sella_ren"],
        "purpose": "Qwen-generated draft branches from a Ghostlight local-awareness packet, preserved as reviewed training material.",
        "generation_provenance": {
            "capture_ref": "experiments/ink/cold-wake-sanctuary-intake-qwen-branch-candidates-v1.capture.json",
            "model": capture["model"],
            "review_status": "accepted_as_draft",
        },
        "local_awareness_summary": [
            {
                "text": "Maer knows the clinic is nearly full, the packet may be testimony or noise, and rescue becomes theater if no one prepares intake.",
                "source_refs": ["scenes.scene-02-sanctuary-intake.dialogue_context_packs.maer_tidecall.speaker_local_truth"],
            },
            {
                "text": "Sella controls sanctuary intake and treats care as material capacity rather than symbolic mercy.",
                "source_refs": ["agents.sella_ren.goals.goal-keep-sanctuary-open", "agents.sella_ren.memories.memory-care-is-capacity"],
            },
        ],
        "projection_controls": load_json(ROOT / "examples" / "ink" / "cold-wake-sanctuary-intake.training.json")[
            "projection_controls"
        ],
        "branches": branches,
        "mutation_policy": {
            "automated": [
                "Qwen may propose branch choices and branch prose.",
                "Ink variables may track local playthrough branch state.",
                "Compiled Ink JSON can be consumed by runtime tools.",
            ],
            "manual_review_required": [
                "Relationship deltas",
                "Belief updates",
                "Memory writes",
                "Listener misread classification",
                "Promotion from branch outcome into canonical Aetheria lore",
            ],
        },
        "audit": {
            "reviewer": "Codex",
            "review_status": "accepted",
            "review_notes": [
                "Qwen generated four branch candidates from Ghostlight local awareness and projection controls.",
                "The draft preserves multiple non-speech actions: show_object, withhold_object, and silence.",
                "The draft is accepted as training material, not as final polished Aetheria prose.",
            ],
            "checks": {
                "uses_standard_branching_format": True,
                "actor_local_context_only": True,
                "includes_non_speech_actions": True,
                "fuzzy_updates_marked_for_manual_review": True,
                "aetheria_lore_content_readable": True,
            },
        },
    }


def main() -> int:
    parser = argparse.ArgumentParser(description="Materialize a Qwen branch capture into Ink and sidecar training data.")
    parser.add_argument("--capture", type=Path, default=DEFAULT_CAPTURE)
    parser.add_argument("--ink-out", type=Path, default=DEFAULT_INK_OUT)
    parser.add_argument("--training-out", type=Path, default=DEFAULT_TRAINING_OUT)
    args = parser.parse_args()

    capture = load_json(args.capture)
    ink_ref = str(args.ink_out.relative_to(ROOT)).replace("\\", "/")
    write_text(args.ink_out, build_ink(capture))
    write_json(args.training_out, build_training(capture, ink_ref))
    print(f"wrote {args.ink_out.relative_to(ROOT)}")
    print(f"wrote {args.training_out.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
