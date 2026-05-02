"""Materialize an accepted sequential Qwen capture into Ink and mutation data."""

from __future__ import annotations

import argparse
import copy
import json
import re
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
DEFAULT_CAPTURE = ROOT / "experiments" / "ink" / "cold-wake-sanctuary-intake-qwen-sequential-v10-no-think.capture.json"
DEFAULT_INK_OUT = ROOT / "examples" / "ink" / "cold-wake-sanctuary-intake.qwen-sequential-v10.ink"
DEFAULT_TRAINING_OUT = ROOT / "examples" / "ink" / "cold-wake-sanctuary-intake.qwen-sequential-v10.training.json"
DEFAULT_STATE_OUT = ROOT / "examples" / "agent-state.cold-wake-story-lab.after-v10-packet-assessment.json"
DEFAULT_MUTATION_OUT = ROOT / "experiments" / "ink" / "cold-wake-sanctuary-intake-qwen-sequential-v10.mutation.json"
EVENT_ID = "event-sanctuary-packet-assessment-boundary"
MAER_MEMORY_ID = "memory-maer-sella-assess-before-intake"
SELLA_MEMORY_ID = "memory-sella-maer-packet-assessment"


def load_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def write_json(path: Path, payload: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")


def write_text(path: Path, text: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text, encoding="utf-8")


def rel(path: Path) -> str:
    return str(path.relative_to(ROOT)).replace("\\", "/")


def ink_identifier(value: str) -> str:
    cleaned = re.sub(r"[^A-Za-z0-9_]+", "_", value).strip("_")
    if not cleaned:
        cleaned = "branch"
    if cleaned[0].isdigit():
        return f"branch_{cleaned}"
    return cleaned


def as_list(value: Any) -> list[str]:
    if isinstance(value, list):
        return [str(item) for item in value if str(item).strip()]
    if isinstance(value, str) and value.strip():
        return [value.strip()]
    return []


def find_agent(state: dict[str, Any], agent_id: str) -> dict[str, Any]:
    return next(agent for agent in state["agents"] if agent["agent_id"] == agent_id)


def find_relationship(state: dict[str, Any], relationship_id: str) -> dict[str, Any]:
    return next(rel for rel in state["relationships"] if rel["relationship_id"] == relationship_id)


def find_scene(state: dict[str, Any], scene_id: str) -> dict[str, Any]:
    return next(scene for scene in state["scenes"] if scene["scene_id"] == scene_id)


def clamp(value: float) -> float:
    return max(0.0, min(1.0, round(value, 3)))


def apply_delta(container: dict[str, Any], path: list[str], delta: float) -> tuple[float, float]:
    target: Any = container
    for key in path[:-1]:
        target = target[key]
    leaf = target[path[-1]]
    before = leaf["current_activation"]
    after = clamp(before + delta)
    leaf["current_activation"] = after
    return before, after


def ensure_value(items: list[str], value: str) -> None:
    if value not in items:
        items.append(value)


def ensure_list_item(items: list[Any], key: str, value: str, item: dict[str, Any]) -> None:
    if not any(isinstance(existing, dict) and existing.get(key) == value for existing in items):
        items.append(item)


def ensure_overlay(agent: dict[str, Any], observer_id: str, target_id: str) -> dict[str, Any]:
    for overlay in agent["perceived_state_overlays"]:
        if overlay["observer_agent_id"] == observer_id and overlay["target_agent_id"] == target_id:
            return overlay
    overlay = {
        "observer_agent_id": observer_id,
        "target_agent_id": target_id,
        "perceived_dimensions": {},
        "beliefs": [],
        "distortions": [],
    }
    agent["perceived_state_overlays"].append(overlay)
    return overlay


def update_perceived_dimension(overlay: dict[str, Any], name: str, observed: float, attributed: float, confidence: float) -> None:
    overlay["perceived_dimensions"][name] = {
        "observed_activation": clamp(observed),
        "attributed_disposition": clamp(attributed),
        "confidence": clamp(confidence),
    }


def selected_action(capture: dict[str, Any]) -> dict[str, Any]:
    return capture["passes"]["selected_observable_action"]


def sella_appraisal(capture: dict[str, Any]) -> dict[str, Any]:
    return capture["passes"]["sella_immediate_appraisal"]["parsed_response"]["appraisal"]


def sella_response(capture: dict[str, Any]) -> dict[str, Any]:
    return capture["passes"]["sella_response_generation"]["parsed_response"]["response"]


def build_ink(capture: dict[str, Any]) -> str:
    selected = selected_action(capture)
    response = sella_response(capture)
    knot = ink_identifier(selected["ink_path"])
    lines = [
        "VAR packet_shown = false",
        "VAR sella_assessment_pending = false",
        "VAR sella_intake_acceptance = false",
        "",
        "-> start",
        "",
        "=== start ===",
        "// ghostlight.scene: scene-02-sanctuary-intake",
        f"// ghostlight.generated_by: {rel(DEFAULT_CAPTURE)}",
        "// aetheria.flashpoint: cold-wake-panic",
        "",
        "The intake clinic sits behind Ganymede pumpworks, warm only where heat has been budgeted.",
        "Sella Ren stands at the edge of an overfull triage board. Maer Tidecall has one corrupted packet and no clean proof.",
        "",
        "What does Maer do?",
        "",
        f"* [{selected['choice_text']}]",
        f"    // ghostlight.branch: {selected['branch_id']}",
        f"    // ghostlight.action: {selected['action_type']}",
        f"    // ghostlight.intent: {selected['actor_intent_private']}",
        f"    -> {knot}",
        "",
        f"=== {knot} ===",
        "~ packet_shown = true",
        "~ sella_assessment_pending = true",
        "",
        selected["observable_action"],
        "",
        f"\"{selected['spoken_text']}\"",
        "",
        response["observable_response"],
        "",
        f"\"{response['spoken_text']}\"",
        "",
        f"// ghostlight.generated_branch: {selected['branch_id']}",
        "// ghostlight.consequence: packet_assessment_boundary_reviewed_consequence",
    ]
    for hook in as_list(selected.get("training_hooks")) + as_list(response.get("training_hooks")):
        lines.append(f"// ghostlight.training_hook: {hook}")
    lines.extend(["-> END", ""])
    return "\n".join(lines)


def build_training(capture: dict[str, Any], ink_ref: str) -> dict[str, Any]:
    selected = selected_action(capture)
    response = sella_response(capture)
    source_annotation = load_json(ROOT / capture["source_annotation_ref"])
    return {
        "schema_version": "ghostlight.ink_training_annotation.v0",
        "annotation_id": "cold-wake-sanctuary-intake-qwen-sequential-v10-training-v0",
        "ink_ref": ink_ref,
        "source_fixture_ref": capture["source_fixture_ref"],
        "scene_id": "scene-02-sanctuary-intake",
        "protagonist_agent_id": "maer_tidecall",
        "npc_agent_ids": ["sella_ren"],
        "purpose": "Materialized accepted projector-routed sequential capture as playable Ink and supervised training data.",
        "generation_provenance": {
            "capture_ref": rel(DEFAULT_CAPTURE),
            "model": capture["model"],
            "review_status": capture["review"]["status"],
            "think": capture["request_options"]["think"],
        },
        "local_awareness_summary": source_annotation["local_awareness_summary"],
        "projection_controls": source_annotation["projection_controls"],
        "branches": [
            {
                "branch_id": selected["branch_id"],
                "ink_path": ink_identifier(selected["ink_path"]),
                "action_type": selected["action_type"],
                "actor_intent": selected["actor_intent_private"],
                "state_basis": as_list(selected["state_basis"]),
                "reviewed_consequences": as_list(response["reviewed_consequences"]),
                "training_hooks": as_list(selected.get("training_hooks")) + as_list(response.get("training_hooks")),
                "generation_notes": {
                    "choice_text": selected["choice_text"],
                    "observable_action": selected["observable_action"],
                    "spoken_text": selected["spoken_text"],
                    "sella_observable_response": response["observable_response"],
                    "sella_spoken_text": response["spoken_text"],
                },
            }
        ],
        "mutation_policy": {
            "automated": [
                "Ink variables may record local branch outcomes.",
                "Accepted sequential captures may be materialized as draft playable scenes.",
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
                "Materialized only after the capture validated as accepted-as-draft with no repair notes.",
                "The branch preserves packet personhood uncertainty and Sella's intake authority.",
                "Fuzzy state updates are handled by a separate reviewed mutation receipt.",
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


def apply_reviewed_mutation(capture: dict[str, Any], fixture: dict[str, Any], out_state: Path, mutation_out: Path) -> None:
    state = copy.deepcopy(fixture)
    selected = selected_action(capture)
    appraisal = sella_appraisal(capture)
    response = sella_response(capture)
    maer = find_agent(state, "maer_tidecall")
    sella = find_agent(state, "sella_ren")
    rel_maer_sella = find_relationship(state, "rel-maer-sella")
    rel_sella_maer = find_relationship(state, "rel-sella-maer")
    applied: list[dict[str, Any]] = []

    for target_name, container, path, delta in [
        ("maer_tidecall", maer, ["canonical_state", "behavioral_dimensions", "control_pressure"], 0.01),
        ("maer_tidecall", maer, ["canonical_state", "behavioral_dimensions", "suspicion"], -0.02),
        ("sella_ren", sella, ["canonical_state", "behavioral_dimensions", "control_pressure"], 0.03),
        ("sella_ren", sella, ["canonical_state", "situational_state", "scarcity_pressure"], 0.02),
        ("sella_ren", sella, ["canonical_state", "situational_state", "overstimulation"], 0.02),
    ]:
        before, after = apply_delta(container, path, delta)
        applied.append({"target": target_name, "path": ".".join(path) + ".current_activation", "before": before, "after": after})

    for relationship, deltas in [
        (rel_maer_sella, [("trust", 0.01), ("respect", 0.02), ("obligation", 0.01)]),
        (rel_sella_maer, [("trust", 0.02), ("respect", 0.02), ("resentment", -0.02), ("obligation", 0.01)]),
    ]:
        for name, delta in deltas:
            before, after = apply_delta(relationship, ["stance", name], delta)
            applied.append({"target": relationship["relationship_id"], "path": f"stance.{name}.current_activation", "before": before, "after": after})
        ensure_value(relationship.setdefault("unresolved_incident_ids", []), EVENT_ID)

    rel_maer_sella["summary"] = "Maer reads Sella's refusal to take the packet immediately as a real intake boundary, but not a refusal of rescue."
    rel_sella_maer["summary"] = "Sella respects Maer's honesty about uncertainty and holds him at the boundary where care must become assessment before intake."

    ensure_list_item(
        maer["memories"]["relationship_summaries"],
        "memory_id",
        MAER_MEMORY_ID,
        {
            "memory_id": MAER_MEMORY_ID,
            "summary": "Sella would not take the corrupted packet into intake before assessment, but she left room for rescue if the cost and contents can be named.",
            "salience": 0.74,
            "confidence": 0.69,
            "linked_event_ids": [EVENT_ID],
            "linked_relationship_id": "rel-maer-sella",
        },
    )
    ensure_list_item(
        sella["memories"]["relationship_summaries"],
        "memory_id",
        SELLA_MEMORY_ID,
        {
            "memory_id": SELLA_MEMORY_ID,
            "summary": "Maer showed the corrupted packet without calling it proof, which made assessment possible without surrendering intake authority.",
            "salience": 0.78,
            "confidence": 0.73,
            "linked_event_ids": [EVENT_ID],
            "linked_relationship_id": "rel-sella-maer",
        },
    )

    maer_overlay = ensure_overlay(maer, "maer_tidecall", "sella_ren")
    update_perceived_dimension(maer_overlay, "control_pressure", 0.64, 0.58, 0.56)
    update_perceived_dimension(maer_overlay, "interpersonal_warmth", 0.62, 0.7, 0.57)
    ensure_list_item(
        maer_overlay["beliefs"],
        "belief_id",
        "belief-maer-sella-assessment-is-not-refusal",
        {
            "belief_id": "belief-maer-sella-assessment-is-not-refusal",
            "subject_id": "sella_ren",
            "claim_type": "motive_inference",
            "claim": "Sella's refusal to take the packet immediately is boundary defense, not proof she has abandoned rescue.",
            "confidence": 0.62,
            "evidence_event_ids": [EVENT_ID],
            "visibility": "private",
            "emotional_charge": 0.38,
        },
    )

    sella_overlay = ensure_overlay(sella, "sella_ren", "maer_tidecall")
    update_perceived_dimension(sella_overlay, "control_pressure", 0.58, 0.56, 0.6)
    update_perceived_dimension(sella_overlay, "interpersonal_warmth", 0.61, 0.64, 0.6)
    ensure_list_item(
        sella_overlay["beliefs"],
        "belief_id",
        "belief-sella-maer-can-present-uncertainty-cleanly",
        {
            "belief_id": "belief-sella-maer-can-present-uncertainty-cleanly",
            "subject_id": "maer_tidecall",
            "claim_type": "motive_inference",
            "claim": "Maer can make uncertainty materially present without always turning it into proof or accusation.",
            "confidence": 0.64,
            "evidence_event_ids": [EVENT_ID],
            "visibility": "private",
            "emotional_charge": 0.35,
        },
    )

    scene = find_scene(state, "scene-02-sanctuary-intake")
    for pack in scene["dialogue_context_packs"]:
        if pack["speaker_agent_id"] == "maer_tidecall":
            ensure_value(pack["active_memories"], MAER_MEMORY_ID)
            ensure_value(pack["speaker_beliefs"], "Sella's assessment boundary is not the same as a refusal of rescue.")
        if pack["speaker_agent_id"] == "sella_ren":
            ensure_value(pack["active_memories"], SELLA_MEMORY_ID)
            ensure_value(pack["speaker_beliefs"], "Maer can show uncertainty without demanding that sanctuary call it proof.")

    event = {
        "event_id": EVENT_ID,
        "kind": "mixed",
        "summary": "Maer showed Sella the corrupted packet without claiming it as proof; Sella refused intake acceptance until assessment could name contents, risk, and cost.",
        "participants": ["maer_tidecall", "sella_ren"],
        "pressure_tags": ["cold_wake", "packet_assessment", "sanctuary_capacity", "personhood_uncertainty"],
        "observed_exchange": [
            {
                "speaker_id": "maer_tidecall",
                "action_type": selected["action_type"],
                "utterance_summary": selected["spoken_text"],
                "dialogue_function": "present_ambiguous_evidence",
                "observable_cues": [selected["observable_action"]],
            },
            {
                "speaker_id": "sella_ren",
                "action_type": response["action_type"],
                "utterance_summary": response["spoken_text"],
                "dialogue_function": "withhold_intake_pending_assessment",
                "observable_cues": [response["observable_response"]],
            },
        ],
        "private_interpretations": [
            {
                "agent_id": "sella_ren",
                "interpretation": appraisal["private_interpretation"],
                "attributed_motive": appraisal["attributed_motive"],
                "appraisal_tags": ["assessment_boundary", "capacity_risk", "guarded_trust"],
                "confidence": 0.68,
            },
            {
                "agent_id": "maer_tidecall",
                "interpretation": "Sella is holding the packet at the assessment boundary rather than erasing the rescue claim.",
                "attributed_motive": "Protect sanctuary intake integrity while leaving room for a justified rescue commitment.",
                "appraisal_tags": ["boundary_not_refusal", "rescue_still_live"],
                "confidence": 0.61,
            },
        ],
        "event_effects": [],
    }
    if not any(existing["event_id"] == EVENT_ID for existing in state["events"]):
        state["events"].append(event)

    participant_appraisals = [
        {
            "participant_agent_id": "maer_tidecall",
            "trigger_observable_action": event["observed_exchange"][1],
            "private_interpretation": "Sella is not accepting the packet yet, but she has not collapsed uncertainty into refusal.",
            "state_deltas": [
                {"path": "canonical_state.behavioral_dimensions.control_pressure.current_activation", "delta": 0.01},
                {"path": "canonical_state.behavioral_dimensions.suspicion.current_activation", "delta": -0.02},
            ],
            "next_action_constraints": [
                "Do not treat Sella's assessment boundary as a clean refusal.",
                "Do not resolve packet personhood.",
                "Any next Maer action should answer cost, risk, or provenance rather than demand acceptance.",
            ],
        },
        {
            "participant_agent_id": "sella_ren",
            "trigger_observable_action": event["observed_exchange"][0],
            "private_interpretation": appraisal["private_interpretation"],
            "state_deltas": [
                {"path": "canonical_state.behavioral_dimensions.control_pressure.current_activation", "delta": 0.03},
                {"path": "canonical_state.situational_state.scarcity_pressure.current_activation", "delta": 0.02},
                {"path": "canonical_state.situational_state.overstimulation.current_activation", "delta": 0.02},
            ],
            "next_action_constraints": appraisal["response_constraints"],
        },
    ]

    receipt = {
        "schema_version": "ghostlight.reviewed_state_mutation.v0",
        "mutation_id": "mutation-sanctuary-packet-assessment-v10",
        "source_fixture_ref": capture["source_fixture_ref"],
        "source_capture_ref": rel(DEFAULT_CAPTURE),
        "mutated_fixture_ref": rel(out_state),
        "selected_branch_id": selected["branch_id"],
        "event_id": EVENT_ID,
        "participant_appraisals": participant_appraisals,
        "observed_action": event["observed_exchange"][0],
        "observed_response": event["observed_exchange"][1],
        "applied_mutations": applied,
        "manual_review_notes": [
            "The packet remains unresolved.",
            "Sella's non-acceptance is modeled as assessment boundary, not refusal.",
            "Both Maer and Sella receive state, memory, relationship, or perception updates.",
            "Fuzzy updates are manual reviewed training data.",
        ],
        "deferred_mutations": [
            "No sanctuary bed is opened or consumed.",
            "No personhood claim is proven or disproven.",
            "No automatic relationship classifier is trained yet.",
        ],
    }
    write_json(out_state, state)
    write_json(mutation_out, receipt)


def main() -> int:
    parser = argparse.ArgumentParser(description="Materialize an accepted sequential Qwen capture.")
    parser.add_argument("--capture", type=Path, default=DEFAULT_CAPTURE)
    parser.add_argument("--ink-out", type=Path, default=DEFAULT_INK_OUT)
    parser.add_argument("--training-out", type=Path, default=DEFAULT_TRAINING_OUT)
    parser.add_argument("--state-out", type=Path, default=DEFAULT_STATE_OUT)
    parser.add_argument("--mutation-out", type=Path, default=DEFAULT_MUTATION_OUT)
    args = parser.parse_args()

    capture = load_json(args.capture)
    if capture["review"]["status"] != "accepted_as_draft":
        raise SystemExit("Only accepted_as_draft sequential captures can be materialized")
    if capture["review"].get("failure_notes") or capture["review"].get("repair_notes"):
        raise SystemExit("Materialization requires an accepted capture without failure or repair notes")

    fixture = load_json(ROOT / capture["source_fixture_ref"])
    ink_ref = rel(args.ink_out)
    write_text(args.ink_out, build_ink(capture))
    write_json(args.training_out, build_training(capture, ink_ref))
    apply_reviewed_mutation(capture, fixture, args.state_out, args.mutation_out)
    capture["review"]["accepted_into_ink_ref"] = ink_ref
    write_json(args.capture, capture)
    print(f"wrote {rel(args.ink_out)}")
    print(f"wrote {rel(args.training_out)}")
    print(f"wrote {rel(args.state_out)}")
    print(f"wrote {rel(args.mutation_out)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
