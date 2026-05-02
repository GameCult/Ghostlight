"""Apply reviewed mutation from a sandboxed responder output capture.

This consumes a reviewed responder-output capture and writes a new agent-state
fixture plus a mutation receipt. It is manual-reviewed transition data, not an
automatic relationship engine wearing a lab coat it stole from a chair.
"""

from __future__ import annotations

import argparse
import copy
import json
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
DEFAULT_FIXTURE = ROOT / "examples" / "agent-state.cold-wake-story-lab.json"
DEFAULT_CAPTURE = ROOT / "experiments" / "responder-packets" / "cold-wake-sanctuary-intake-sella-v0.capture.json"
DEFAULT_OUT_STATE = ROOT / "examples" / "agent-state.cold-wake-story-lab.after-sella-conditions.json"
DEFAULT_RECEIPT = ROOT / "experiments" / "responder-packets" / "cold-wake-sanctuary-intake-sella-v0.mutation.json"
EVENT_ID = "event-sanctuary-conditional-repair-bay"
MAER_MEMORY_ID = "memory-maer-sella-conditional-bay"
SELLA_MEMORY_ID = "memory-sella-maer-conditional-ledger"


def load_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8-sig"))


def write_json(path: Path, payload: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")


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


def main() -> int:
    parser = argparse.ArgumentParser(description="Apply reviewed mutation from a responder output capture.")
    parser.add_argument("--fixture", type=Path, default=DEFAULT_FIXTURE)
    parser.add_argument("--capture", type=Path, default=DEFAULT_CAPTURE)
    parser.add_argument("--out-state", type=Path, default=DEFAULT_OUT_STATE)
    parser.add_argument("--receipt", type=Path, default=DEFAULT_RECEIPT)
    args = parser.parse_args()
    args.fixture = args.fixture.resolve()
    args.capture = args.capture.resolve()
    args.out_state = args.out_state.resolve()
    args.receipt = args.receipt.resolve()

    source_state = load_json(args.fixture)
    capture = load_json(args.capture)
    response = capture["parsed_output"]
    state = copy.deepcopy(source_state)

    maer = find_agent(state, "maer_tidecall")
    sella = find_agent(state, "sella_ren")
    rel_maer_sella = find_relationship(state, "rel-maer-sella")
    rel_sella_maer = find_relationship(state, "rel-sella-maer")
    scene = find_scene(state, "scene-02-sanctuary-intake")

    observed_action = {
        "speaker_id": "maer_tidecall",
        "action_type": "show_object",
        "utterance_summary": "Maer displays the corrupted packet through shared clinic systems and asks Sella to judge whether the math supports intake.",
        "dialogue_function": "present_ambiguous_evidence",
        "observable_cues": [
            "shared clinic display shows routing terms and telemetry gaps",
            "packet remains unresolved evidence rather than proof",
        ],
    }
    observed_response = {
        "speaker_id": "sella_ren",
        "action_type": response["action_label"],
        "utterance_summary": response["spoken_text"],
        "dialogue_function": "conditional_capacity_offer",
        "observable_cues": [response["visible_action"]],
    }

    maer_appraisal = {
        "schema_version": "ghostlight.manual_participant_appraisal.v0",
        "appraiser": "codex_manual_review",
        "participant_agent_id": "maer_tidecall",
        "trigger_observable_action": observed_response,
        "private_interpretation": "Sella has not accepted the claimant into sanctuary, but she has created a narrow material path if Maer accepts ledger responsibility.",
        "attributed_motive": "Protect care capacity while refusing to erase a possible person into cargo language.",
        "misread_risk": "Medium. Maer may overread the condition as moral refusal if urgency outruns his respect for Sella's capacity boundary.",
        "state_deltas": [
            {
                "agent_id": "maer_tidecall",
                "path": "canonical_state.behavioral_dimensions.control_pressure.current_activation",
                "delta": 0.03,
                "rationale": "The next move now requires ledger backing and labor ownership rather than another moral appeal.",
            },
            {
                "agent_id": "maer_tidecall",
                "path": "canonical_state.situational_state.grievance_activation.current_activation",
                "delta": -0.02,
                "rationale": "Sella did not call the packet cargo or close the door entirely.",
            },
        ],
        "relationship_deltas": [
            {
                "relationship_id": "rel-maer-sella",
                "path": "stance.trust.current_activation",
                "delta": 0.02,
                "rationale": "Sella offered a narrow path instead of hiding behind capacity math alone.",
            },
            {
                "relationship_id": "rel-maer-sella",
                "path": "stance.respect.current_activation",
                "delta": 0.03,
                "rationale": "She made care concrete without pretending it was free.",
            },
            {
                "relationship_id": "rel-maer-sella",
                "path": "stance.obligation.current_activation",
                "delta": 0.04,
                "rationale": "Maer now owes material backing if he wants the cold bay held.",
            },
        ],
        "next_action_constraints": [
            "Maer must answer the ledger, labor, and custody conditions if he wants the repair bay held.",
            "Maer should not treat the conditional bay as sanctuary acceptance.",
            "Packet personhood remains unresolved.",
        ],
    }

    sella_appraisal = {
        "schema_version": "ghostlight.manual_participant_appraisal.v0",
        "appraiser": "codex_manual_review",
        "participant_agent_id": "sella_ren",
        "trigger_observable_action": observed_action,
        "private_interpretation": response["private_interpretation"],
        "attributed_motive": "Maer is trying to keep rescue possible, but may be letting the clinic inherit costs his own chain has not accepted.",
        "misread_risk": "Medium. Sella may overread moral pressure as an attempt to spend sanctuary capacity without ownership.",
        "state_deltas": [
            {
                "agent_id": "sella_ren",
                "path": "canonical_state.behavioral_dimensions.control_pressure.current_activation",
                "delta": 0.04,
                "rationale": "She has to make the boundary operational, not merely verbal.",
            },
            {
                "agent_id": "sella_ren",
                "path": "canonical_state.situational_state.scarcity_pressure.current_activation",
                "delta": 0.03,
                "rationale": "Holding a cold repair bay still consumes capacity even if no bed is opened.",
            },
            {
                "agent_id": "sella_ren",
                "path": "canonical_state.situational_state.overstimulation.current_activation",
                "delta": 0.02,
                "rationale": "The conditional path creates more tracking, not less.",
            },
        ],
        "relationship_deltas": [
            {
                "relationship_id": "rel-sella-maer",
                "path": "stance.trust.current_activation",
                "delta": 0.01,
                "rationale": "Maer provided evidence rather than only pressure.",
            },
            {
                "relationship_id": "rel-sella-maer",
                "path": "stance.resentment.current_activation",
                "delta": 0.03,
                "rationale": "The hidden bill still lands near her clinic.",
            },
            {
                "relationship_id": "rel-sella-maer",
                "path": "stance.obligation.current_activation",
                "delta": 0.03,
                "rationale": "She has conditionally placed sanctuary capacity at risk.",
            },
        ],
        "next_action_constraints": [
            "Sella has not admitted the packet into patient flow.",
            "Sella has conditionally held a bounded cold repair bay for a short window.",
            "Ledger backing, isolation, decon, failed extraction cost, and body-work ownership are live conditions.",
            "Packet personhood remains unresolved.",
        ],
    }

    event = {
        "event_id": EVENT_ID,
        "kind": "dialogue",
        "summary": "Sella set conditions for a bounded cold repair bay without accepting the corrupted packet into sanctuary patient flow.",
        "participants": ["maer_tidecall", "sella_ren"],
        "pressure_tags": [
            "cold_wake",
            "sanctuary_capacity",
            "repair_bay_condition",
            "ledger_backing",
            "personhood_uncertainty",
        ],
        "observed_exchange": [observed_action, observed_response],
        "private_interpretations": [
            {
                "agent_id": "maer_tidecall",
                "interpretation": maer_appraisal["private_interpretation"],
                "attributed_motive": maer_appraisal["attributed_motive"],
                "appraisal_tags": ["conditional_path", "capacity_boundary", "rescuer_obligation"],
                "confidence": 0.64,
            },
            {
                "agent_id": "sella_ren",
                "interpretation": sella_appraisal["private_interpretation"],
                "attributed_motive": sella_appraisal["attributed_motive"],
                "appraisal_tags": ["moral_pressure", "ledger_boundary", "capacity_risk"],
                "confidence": 0.69,
            },
        ],
        "event_effects": [
            {
                "agent_id": "maer_tidecall",
                "summary": "Maer now has a narrow rescue path contingent on ledger backing and labor ownership.",
                "state_deltas": maer_appraisal["state_deltas"],
                "memory_update_ids": [MAER_MEMORY_ID],
            },
            {
                "agent_id": "sella_ren",
                "summary": "Sella preserves triage authority while conditionally risking a bounded repair bay.",
                "state_deltas": sella_appraisal["state_deltas"],
                "memory_update_ids": [SELLA_MEMORY_ID],
            },
        ],
    }
    if not any(existing["event_id"] == EVENT_ID for existing in state["events"]):
        state["events"].append(event)

    applied_changes: list[dict[str, Any]] = []
    for path, delta in [
        (["canonical_state", "behavioral_dimensions", "control_pressure"], 0.03),
        (["canonical_state", "situational_state", "grievance_activation"], -0.02),
    ]:
        before, after = apply_delta(maer, path, delta)
        applied_changes.append({"target": "maer_tidecall", "path": ".".join(path) + ".current_activation", "before": before, "after": after})

    for path, delta in [
        (["canonical_state", "behavioral_dimensions", "control_pressure"], 0.04),
        (["canonical_state", "situational_state", "scarcity_pressure"], 0.03),
        (["canonical_state", "situational_state", "overstimulation"], 0.02),
    ]:
        before, after = apply_delta(sella, path, delta)
        applied_changes.append({"target": "sella_ren", "path": ".".join(path) + ".current_activation", "before": before, "after": after})

    for rel, deltas in [
        (rel_maer_sella, [("trust", 0.02), ("respect", 0.03), ("obligation", 0.04)]),
        (rel_sella_maer, [("trust", 0.01), ("resentment", 0.03), ("obligation", 0.03)]),
    ]:
        for name, delta in deltas:
            before, after = apply_delta(rel, ["stance", name], delta)
            applied_changes.append({"target": rel["relationship_id"], "path": f"stance.{name}.current_activation", "before": before, "after": after})
        ensure_value(rel.setdefault("unresolved_incident_ids", []), EVENT_ID)

    rel_maer_sella["summary"] = "Maer trusts Sella slightly more because she made a narrow rescue path material, but he now owes ledger backing before that path can stay open."
    rel_sella_maer["summary"] = "Sella respects Maer's rescue ethic and is willing to hold a narrow conditional bay, but she remains alert to how his requests can move hidden costs onto sanctuary."

    ensure_list_item(
        maer["memories"]["relationship_summaries"],
        "memory_id",
        MAER_MEMORY_ID,
        {
            "memory_id": MAER_MEMORY_ID,
            "summary": "Sella would not call the packet cargo, but she would only hold a cold repair bay if Maer attached ledger backing and labor ownership.",
            "salience": 0.82,
            "confidence": 0.71,
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
            "summary": "Maer brought evidence and urgency; Sella held a bounded repair bay cold only under ledger, isolation, decon, and failed-extraction conditions.",
            "salience": 0.84,
            "confidence": 0.74,
            "linked_event_ids": [EVENT_ID],
            "linked_relationship_id": "rel-sella-maer",
        },
    )

    maer_overlay = ensure_overlay(maer, "maer_tidecall", "sella_ren")
    update_perceived_dimension(maer_overlay, "interpersonal_warmth", 0.7, 0.75, 0.62)
    update_perceived_dimension(maer_overlay, "control_pressure", 0.7, 0.61, 0.62)
    ensure_list_item(
        maer_overlay["beliefs"],
        "belief_id",
        "belief-maer-sella-care-has-ledger-price",
        {
            "belief_id": "belief-maer-sella-care-has-ledger-price",
            "subject_id": "sella_ren",
            "claim_type": "motive_inference",
            "claim": "Sella will keep a possible person from being called cargo, but only if the rescue path has material backing.",
            "confidence": 0.64,
            "evidence_event_ids": [EVENT_ID],
            "visibility": "private",
            "emotional_charge": 0.52,
        },
    )

    sella_overlay = ensure_overlay(sella, "sella_ren", "maer_tidecall")
    update_perceived_dimension(sella_overlay, "control_pressure", 0.74, 0.66, 0.68)
    update_perceived_dimension(sella_overlay, "interpersonal_warmth", 0.61, 0.66, 0.62)
    ensure_list_item(
        sella_overlay["beliefs"],
        "belief_id",
        "belief-sella-maer-needs-material-ownership",
        {
            "belief_id": "belief-sella-maer-needs-material-ownership",
            "subject_id": "maer_tidecall",
            "claim_type": "motive_inference",
            "claim": "Maer means rescue, but he must be forced to attach cost ownership before his rescue ethic spends sanctuary capacity.",
            "confidence": 0.69,
            "evidence_event_ids": [EVENT_ID],
            "visibility": "private",
            "emotional_charge": 0.6,
        },
    )
    ensure_value(sella_overlay["distortions"], "cost_pressure_overread")

    for pack in scene["dialogue_context_packs"]:
        if pack["speaker_agent_id"] == "maer_tidecall":
            ensure_value(pack["active_memories"], MAER_MEMORY_ID)
            ensure_value(pack["speaker_beliefs"], "Sella has opened a narrow repair-bay path, but only if Maer owns the ledger cost.")
        if pack["speaker_agent_id"] == "sella_ren":
            ensure_value(pack["active_memories"], SELLA_MEMORY_ID)
            ensure_value(pack["speaker_beliefs"], "The packet is still evidence at the door, not a patient in the room.")

    receipt = {
        "schema_version": "ghostlight.reviewed_state_mutation.v0",
        "mutation_id": "mutation-sanctuary-conditional-repair-bay-v0",
        "source_fixture_ref": str(args.fixture.relative_to(ROOT)).replace("\\", "/"),
        "source_capture_ref": str(args.capture.relative_to(ROOT)).replace("\\", "/"),
        "mutated_fixture_ref": str(args.out_state.relative_to(ROOT)).replace("\\", "/"),
        "event_id": EVENT_ID,
        "responder_output_ref": str(args.capture.relative_to(ROOT)).replace("\\", "/") + "#parsed_output",
        "participant_appraisals": [maer_appraisal, sella_appraisal],
        "observed_action": observed_action,
        "observed_response": observed_response,
        "applied_mutations": applied_changes,
        "accepted_world_state_candidates": [
            "A bounded cold repair bay may be held for a short window if ledger backing is attached.",
            "The corrupted packet remains evidence at the door and is not admitted into patient flow.",
            "Ledger backing is required for air, isolation, decon, repair labor, and failed extraction before escalation.",
        ],
        "manual_review_notes": [
            "The responder output's candidate updates were reviewed before becoming fixture state.",
            "The repair-bay condition is accepted as scene-local world state, not as sanctuary bed intake.",
            "Both Maer and Sella receive state, relationship, memory, and perceived-overlay updates.",
            "Packet personhood remains unresolved.",
        ],
        "deferred_mutations": [
            "No sanctuary bed is opened or consumed.",
            "No personhood claim is proven or disproven.",
            "No automatic appraiser or mutator is trained yet.",
        ],
    }

    write_json(args.out_state, state)
    write_json(args.receipt, receipt)
    print(f"wrote {args.out_state.relative_to(ROOT)}")
    print(f"wrote {args.receipt.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
