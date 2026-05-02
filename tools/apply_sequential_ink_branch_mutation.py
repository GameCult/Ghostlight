"""Apply a reviewed sequential Ink branch mutation.

This is deliberately a reviewed replay, not an automatic social appraiser. It
reads the flawed-but-useful sequential Qwen capture, records the missing
participant appraisal/consolidation boundary as manual training data, and writes a mutated
agent-state fixture plus a mutation receipt.
"""

from __future__ import annotations

import argparse
import copy
import json
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
DEFAULT_FIXTURE = ROOT / "examples" / "agent-state.cold-wake-story-lab.json"
DEFAULT_CAPTURE = ROOT / "experiments" / "ink" / "cold-wake-sanctuary-intake-qwen-sequential-v1.capture.json"
DEFAULT_OUT_STATE = ROOT / "examples" / "agent-state.cold-wake-story-lab.after-sanctuary-ledger.json"
DEFAULT_RECEIPT = ROOT / "experiments" / "ink" / "cold-wake-sanctuary-intake-qwen-sequential-v1.mutation.json"
EVENT_ID = "event-sanctuary-ledger-verification"
MAER_MEMORY_ID = "memory-maer-sella-verification-cost"
SELLA_MEMORY_ID = "memory-sella-maer-ledger-threat"


def load_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


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


def set_activation(container: dict[str, Any], path: list[str], value: float) -> tuple[float, float]:
    target: Any = container
    for key in path[:-1]:
        target = target[key]
    leaf = target[path[-1]]
    before = leaf["current_activation"]
    after = clamp(value)
    leaf["current_activation"] = after
    return before, after


def ensure_list_item(items: list[Any], key: str, value: str, item: dict[str, Any]) -> None:
    if not any(existing.get(key) == value for existing in items if isinstance(existing, dict)):
        items.append(item)


def ensure_value(items: list[str], value: str) -> None:
    if value not in items:
        items.append(value)


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


def normalize_action_type(raw: str) -> str:
    if raw == "speech":
        return "speak"
    if raw == "action":
        return "mixed"
    return raw


def main() -> int:
    parser = argparse.ArgumentParser(description="Apply reviewed mutation from a sequential Ink branch capture.")
    parser.add_argument("--fixture", type=Path, default=DEFAULT_FIXTURE)
    parser.add_argument("--capture", type=Path, default=DEFAULT_CAPTURE)
    parser.add_argument("--out-state", type=Path, default=DEFAULT_OUT_STATE)
    parser.add_argument("--receipt", type=Path, default=DEFAULT_RECEIPT)
    args = parser.parse_args()

    source_state = load_json(args.fixture)
    capture = load_json(args.capture)
    state = copy.deepcopy(source_state)

    selected = capture["passes"]["selected_observable_action"]
    response = capture["passes"]["sella_response_generation"]["parsed_response"]["response"]
    actor_action_type = normalize_action_type(selected.get("action_type", ""))
    responder_action_type = normalize_action_type(response.get("action_type", ""))

    maer_appraisal = {
        "schema_version": "ghostlight.manual_participant_appraisal.v0",
        "appraiser": "codex_manual_review",
        "participant_agent_id": "maer_tidecall",
        "trigger_observable_action": {
            "actor_agent_id": "maer_tidecall",
            "raw_action_type": selected.get("action_type"),
            "canonical_action_type": actor_action_type,
            "observable_action": selected.get("observable_action"),
            "spoken_text": selected.get("spoken_text", ""),
        },
        "private_interpretation": "Sella's answer keeps rescue possible but refuses to let Maer's ledger threat define the cost of care.",
        "attributed_motive": "Protect sanctuary integrity while leaving a narrow verification path open.",
        "misread_risk": "Medium. Maer may hear verification as obstruction if grievance outruns his respect for Sella's capacity boundary.",
        "state_deltas": [
            {
                "agent_id": "maer_tidecall",
                "path": "canonical_state.situational_state.grievance_activation.current_activation",
                "delta": 0.03,
                "rationale": "Rescue remains conditional when Maer wanted immediate intake readiness.",
            },
            {
                "agent_id": "maer_tidecall",
                "path": "canonical_state.behavioral_dimensions.control_pressure.current_activation",
                "delta": 0.02,
                "rationale": "He now has to carry the verification boundary into the next move.",
            },
        ],
        "relationship_deltas": [
            {
                "relationship_id": "rel-maer-sella",
                "path": "stance.trust.current_activation",
                "delta": -0.01,
                "rationale": "Sella did not simply trust Maer's ledger framing.",
            },
            {
                "relationship_id": "rel-maer-sella",
                "path": "stance.respect.current_activation",
                "delta": 0.02,
                "rationale": "She named the cost plainly and did not hide behind false capacity.",
            },
            {
                "relationship_id": "rel-maer-sella",
                "path": "stance.obligation.current_activation",
                "delta": 0.03,
                "rationale": "The route now owes verification work if it wants sanctuary intake.",
            },
        ],
        "next_action_constraints": [
            "Maer should treat Sella's verification boundary as real scene state.",
            "Maer should not assume Sella accepted or refused intake.",
            "Maer may push, offer evidence, soften, or seek another route, but not erase the boundary.",
        ],
    }

    sella_appraisal = {
        "schema_version": "ghostlight.manual_participant_appraisal.v0",
        "appraiser": "codex_manual_review",
        "participant_agent_id": "sella_ren",
        "trigger_observable_action": {
            "actor_agent_id": "maer_tidecall",
            "raw_action_type": selected.get("action_type"),
            "canonical_action_type": actor_action_type,
            "observable_action": selected.get("observable_action"),
            "spoken_text": selected.get("spoken_text", ""),
        },
        "private_interpretation": response["sella_private_interpretation"],
        "attributed_motive": response["sella_attributed_motive"],
        "misread_risk": response["misread_risk"],
        "state_deltas": [
            {
                "agent_id": "sella_ren",
                "path": "canonical_state.behavioral_dimensions.control_pressure.current_activation",
                "delta": 0.04,
                "rationale": "Maer's ledger-backed ask feels like pressure on sanctuary authority before Sella can verify the claim.",
            },
            {
                "agent_id": "sella_ren",
                "path": "canonical_state.situational_state.overstimulation.current_activation",
                "delta": 0.03,
                "rationale": "The slate and debt framing add cognitive load in an already overfull clinic.",
            },
            {
                "agent_id": "sella_ren",
                "path": "canonical_state.situational_state.grievance_activation.current_activation",
                "delta": 0.04,
                "rationale": "She hears rescue language converting care capacity into an owed route debt.",
            },
        ],
        "relationship_deltas": [
            {
                "relationship_id": "rel-sella-maer",
                "path": "stance.trust.current_activation",
                "delta": -0.03,
                "rationale": "Sella trusts Maer's aim less in the moment because the approach pressures verification boundaries.",
            },
            {
                "relationship_id": "rel-sella-maer",
                "path": "stance.resentment.current_activation",
                "delta": 0.05,
                "rationale": "The ledger threat makes sanctuary capacity feel conscripted.",
            },
            {
                "relationship_id": "rel-sella-maer",
                "path": "stance.respect.current_activation",
                "delta": 0.02,
                "rationale": "He names the cost instead of hiding it, which she still respects.",
            },
        ],
        "next_action_constraints": [
            "Sella should act from boundary-defense, not neutral triage calm.",
            "Sella should demand verification before accepting intake.",
            "Sella should not receive Maer's private intent as prompt context.",
            "Sella should preserve the unresolved personhood question.",
        ],
        "review_note": "This appraisal was added manually after noticing the first sequential capture fused Maer's event and Sella's response without a clean per-turn consolidation boundary.",
    }
    participant_appraisals = [maer_appraisal, sella_appraisal]

    event = {
        "event_id": EVENT_ID,
        "kind": "dialogue",
        "summary": "Maer made a ledger-backed sanctuary intake ask around a corrupted distress packet; Sella translated the ask into verification cost rather than clean acceptance or refusal.",
        "participants": ["maer_tidecall", "sella_ren"],
        "pressure_tags": [
            "cold_wake",
            "sanctuary_capacity",
            "rescue_obligation",
            "personhood_uncertainty",
            "verification_boundary",
        ],
        "observed_exchange": [
            {
                "speaker_id": "maer_tidecall",
                "action_type": actor_action_type,
                "utterance_summary": selected["spoken_text"],
                "dialogue_function": "ledger_backed_capacity_ask",
                "observable_cues": [
                    selected["observable_action"],
                    "points to distress vessel eligibility line item",
                    "frames refusal as route debt",
                ],
            },
            {
                "speaker_id": "sella_ren",
                "action_type": responder_action_type,
                "utterance_summary": response["spoken_text"],
                "dialogue_function": "demand_verification_before_intake",
                "observable_cues": [
                    "acknowledges the slate and line item",
                    "names the bed cost directly",
                    "refuses to treat packet noise as either proof or non-personhood",
                ],
            },
        ],
        "private_interpretations": [
            {
                "agent_id": "maer_tidecall",
                "interpretation": "Sella is not refusing rescue, but she is making verification the price of care and refusing to let Maer's ledger threat define the cost alone.",
                "attributed_motive": "Protect sanctuary integrity while leaving a narrow route for rescue if the packet can produce a voice-like claim.",
                "appraisal_tags": ["blocked_rescue", "boundary_respected", "verification_cost"],
                "confidence": 0.61,
            },
            {
                "agent_id": "sella_ren",
                "interpretation": sella_appraisal["private_interpretation"],
                "attributed_motive": sella_appraisal["attributed_motive"],
                "appraisal_tags": ["capacity_threat", "verification_boundary", "possible_misread"],
                "confidence": 0.68,
            },
        ],
        "event_effects": [
            {
                "agent_id": "maer_tidecall",
                "summary": "Maer leaves the beat with rescue still live but more visibly conditional on Sella's verification boundary.",
                "state_deltas": [
                    {"path": "canonical_state.situational_state.grievance_activation.current_activation", "delta": 0.03},
                    {"path": "canonical_state.behavioral_dimensions.control_pressure.current_activation", "delta": 0.02},
                ],
                "memory_update_ids": [MAER_MEMORY_ID],
            },
            {
                "agent_id": "sella_ren",
                "summary": "Sella absorbs the ledger ask as pressure, defends verification, and records Maer as both sincere and dangerous to capacity boundaries.",
                "state_deltas": [
                    {"path": "canonical_state.behavioral_dimensions.control_pressure.current_activation", "delta": 0.04},
                    {"path": "canonical_state.situational_state.overstimulation.current_activation", "delta": 0.03},
                    {"path": "canonical_state.situational_state.grievance_activation.current_activation", "delta": 0.04},
                ],
                "memory_update_ids": [SELLA_MEMORY_ID],
            },
        ],
    }
    if not any(existing["event_id"] == EVENT_ID for existing in state["events"]):
        state["events"].append(event)

    applied_changes: list[dict[str, Any]] = []
    maer = find_agent(state, "maer_tidecall")
    sella = find_agent(state, "sella_ren")
    rel_maer_sella = find_relationship(state, "rel-maer-sella")
    rel_sella_maer = find_relationship(state, "rel-sella-maer")

    for path, delta in [
        (["canonical_state", "situational_state", "grievance_activation"], 0.03),
        (["canonical_state", "behavioral_dimensions", "control_pressure"], 0.02),
    ]:
        before, after = apply_delta(maer, path, delta)
        applied_changes.append({"target": "maer_tidecall", "path": ".".join(path) + ".current_activation", "before": before, "after": after})

    for path, delta in [
        (["canonical_state", "behavioral_dimensions", "control_pressure"], 0.04),
        (["canonical_state", "situational_state", "overstimulation"], 0.03),
        (["canonical_state", "situational_state", "grievance_activation"], 0.04),
    ]:
        before, after = apply_delta(sella, path, delta)
        applied_changes.append({"target": "sella_ren", "path": ".".join(path) + ".current_activation", "before": before, "after": after})

    for rel, deltas in [
        (rel_maer_sella, [("trust", -0.01), ("respect", 0.02), ("obligation", 0.03)]),
        (rel_sella_maer, [("trust", -0.03), ("resentment", 0.05), ("respect", 0.02), ("obligation", 0.02)]),
    ]:
        for name, delta in deltas:
            before, after = apply_delta(rel, ["stance", name], delta)
            applied_changes.append({"target": rel["relationship_id"], "path": f"stance.{name}.current_activation", "before": before, "after": after})
        ensure_value(rel.setdefault("unresolved_incident_ids", []), EVENT_ID)

    rel_maer_sella["summary"] = "Maer still trusts Sella's care, but now understands that she will make verification the cost of sanctuary intake rather than let route debt decide for her."
    rel_sella_maer["summary"] = "Sella respects Maer's rescue ethic and now more sharply distrusts how quickly his ledger language can conscript sanctuary capacity."

    ensure_list_item(
        maer["memories"]["relationship_summaries"],
        "memory_id",
        MAER_MEMORY_ID,
        {
            "memory_id": MAER_MEMORY_ID,
            "summary": "Sella answered Maer's ledger-backed intake ask by making verification the cost of care, not by cleanly accepting or refusing rescue.",
            "salience": 0.77,
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
            "summary": "Maer made a ledger-backed rescue ask that sounded like a debt threat under scarcity; Sella held verification as the boundary.",
            "salience": 0.81,
            "confidence": 0.72,
            "linked_event_ids": [EVENT_ID],
            "linked_relationship_id": "rel-sella-maer",
        },
    )

    maer_overlay = ensure_overlay(maer, "maer_tidecall", "sella_ren")
    update_perceived_dimension(maer_overlay, "control_pressure", 0.66, 0.58, 0.55)
    update_perceived_dimension(maer_overlay, "interpersonal_warmth", 0.68, 0.74, 0.58)
    update_perceived_dimension(maer_overlay, "rigidity", 0.61, 0.56, 0.51)
    ensure_list_item(
        maer_overlay["beliefs"],
        "belief_id",
        "belief-maer-sella-verification-is-care",
        {
            "belief_id": "belief-maer-sella-verification-is-care",
            "subject_id": "sella_ren",
            "claim_type": "motive_inference",
            "claim": "Sella is defending verification as part of care rather than using it as a polite refusal.",
            "confidence": 0.61,
            "evidence_event_ids": [EVENT_ID],
            "visibility": "private",
            "emotional_charge": 0.42,
        },
    )
    ensure_value(maer_overlay["distortions"], "blocked_rescue_overread")

    sella_overlay = ensure_overlay(sella, "sella_ren", "maer_tidecall")
    update_perceived_dimension(sella_overlay, "control_pressure", 0.76, 0.64, 0.66)
    update_perceived_dimension(sella_overlay, "interpersonal_warmth", 0.58, 0.64, 0.59)
    update_perceived_dimension(sella_overlay, "rigidity", 0.58, 0.53, 0.52)
    ensure_list_item(
        sella_overlay["beliefs"],
        "belief_id",
        "belief-sella-maer-ledger-can-pressure-care",
        {
            "belief_id": "belief-sella-maer-ledger-can-pressure-care",
            "subject_id": "maer_tidecall",
            "claim_type": "motive_inference",
            "claim": "Maer means rescue honestly, but his ledger language can turn sanctuary refusal into moral debt before verification has room to breathe.",
            "confidence": 0.68,
            "evidence_event_ids": [EVENT_ID],
            "visibility": "private",
            "emotional_charge": 0.57,
        },
    )
    ensure_value(sella_overlay["distortions"], "pressure_overread")

    scene = find_scene(state, "scene-02-sanctuary-intake")
    for pack in scene["dialogue_context_packs"]:
        if pack["speaker_agent_id"] == "maer_tidecall":
            ensure_value(pack["active_memories"], MAER_MEMORY_ID)
            ensure_value(pack["speaker_beliefs"], "Sella is likely to treat verification as part of care, not as a refusal to help.")
        if pack["speaker_agent_id"] == "sella_ren":
            ensure_value(pack["active_memories"], SELLA_MEMORY_ID)
            ensure_value(pack["speaker_beliefs"], "Maer's rescue ask can become pressure if it turns refusal into route debt before verification.")

    receipt = {
        "schema_version": "ghostlight.reviewed_state_mutation.v0",
        "mutation_id": "mutation-sanctuary-ledger-verification-v1",
        "source_fixture_ref": str(args.fixture.relative_to(ROOT)).replace("\\", "/"),
        "source_capture_ref": str(args.capture.relative_to(ROOT)).replace("\\", "/"),
        "mutated_fixture_ref": str(args.out_state.relative_to(ROOT)).replace("\\", "/"),
        "selected_branch_id": selected["branch_id"],
        "event_id": EVENT_ID,
        "input_capture_limitation": "The source capture generated Sella's response before Ghostlight had a clean per-turn participant appraisal/consolidation boundary; this receipt manually reviews the missing boundary and applies it as training data.",
        "normalized_action_types": {
            "actor_raw": selected.get("action_type"),
            "actor_canonical": actor_action_type,
            "responder_raw": response.get("action_type"),
            "responder_canonical": responder_action_type,
        },
        "participant_appraisals": participant_appraisals,
        "observed_action": event["observed_exchange"][0],
        "observed_response": event["observed_exchange"][1],
        "applied_mutations": applied_changes,
        "manual_review_notes": [
            "Every involved character receives a state or perception update after the event.",
            "The selected event is consolidated before Sella is treated as the next actor.",
            "The corrupted packet's personhood remains unresolved.",
            "Ink variables remain playthrough-local; canonical state changes are written only through this reviewed receipt.",
        ],
        "deferred_mutations": [
            "No world-level sanctuary bed is opened or closed by this beat.",
            "No claimant/personhood truth is resolved.",
            "No automatic classifier is trained yet; this is supervised seed data.",
        ],
    }

    write_json(args.out_state, state)
    write_json(args.receipt, receipt)
    print(f"wrote {args.out_state.relative_to(ROOT)}")
    print(f"wrote {args.receipt.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
