"""Build the Cold Wake story-lab agent-state fixture.

The fixture is authored data, but the full v0 state shape is verbose enough
that generating the repetitive variable maps is less error-prone than hand
copying four giant blocks of zeros like a penitent spreadsheet goblin.
"""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
REQUIRED = json.loads((ROOT / "schemas" / "agent-state.required-fields.json").read_text(encoding="utf-8"))["variable_groups"]
OUT = ROOT / "examples" / "agent-state.cold-wake-story-lab.json"


def var(mean: float, plasticity: float, activation: float) -> dict[str, float]:
    return {"mean": mean, "plasticity": plasticity, "current_activation": activation}


def vmap(group: str, overrides: dict[str, tuple[float, float, float]] | None = None) -> dict[str, dict[str, float]]:
    data = {name: var(0.5, 0.45, 0.5) for name in REQUIRED[group]}
    for name, values in (overrides or {}).items():
        data[name] = var(*values)
    return data


def inferred(observed: float, disposition: float, confidence: float) -> dict[str, float]:
    return {"observed_activation": observed, "attributed_disposition": disposition, "confidence": confidence}


def memory(
    memory_id: str,
    summary: str,
    salience: float,
    confidence: float,
    linked_event_ids: list[str] | None = None,
    linked_relationship_id: str | None = None,
) -> dict[str, Any]:
    record: dict[str, Any] = {
        "memory_id": memory_id,
        "summary": summary,
        "salience": salience,
        "confidence": confidence,
        "linked_event_ids": linked_event_ids or [],
    }
    if linked_relationship_id:
        record["linked_relationship_id"] = linked_relationship_id
    return record


def goal(
    goal_id: str,
    description: str,
    scope: str,
    priority: float,
    emotional_stake: str,
    blockers: list[str],
) -> dict[str, Any]:
    return {
        "goal_id": goal_id,
        "description": description,
        "scope": scope,
        "priority": priority,
        "emotional_stake": emotional_stake,
        "blockers": blockers,
        "status": "active",
    }


def state_profile(kind: str) -> dict[str, Any]:
    if kind == "navigator":
        return {
            "underlying_organization": vmap(
                "underlying_organization",
                {
                    "self_coherence": (0.72, 0.28, 0.76),
                    "reciprocity_capacity": (0.86, 0.23, 0.91),
                    "mentalization_quality": (0.82, 0.24, 0.85),
                    "authenticity_tolerance": (0.66, 0.34, 0.72),
                    "mask_rigidity": (0.44, 0.41, 0.52),
                },
            ),
            "stable_dispositions": vmap(
                "stable_dispositions",
                {
                    "risk_tolerance": (0.54, 0.37, 0.62),
                    "baseline_threat_sensitivity": (0.63, 0.34, 0.71),
                    "ideological_rigidity": (0.58, 0.31, 0.69),
                    "sociability": (0.68, 0.29, 0.74),
                    "status_hunger": (0.24, 0.39, 0.22),
                },
            ),
            "behavioral_dimensions": vmap(
                "behavioral_dimensions",
                {
                    "interpersonal_warmth": (0.72, 0.34, 0.66),
                    "drive": (0.62, 0.31, 0.73),
                    "anxiety": (0.46, 0.52, 0.64),
                    "control_pressure": (0.55, 0.44, 0.67),
                    "suspicion": (0.57, 0.42, 0.71),
                    "rigidity": (0.48, 0.36, 0.62),
                    "volatility": (0.24, 0.53, 0.29),
                    "attachment_seeking": (0.58, 0.43, 0.61),
                    "distance_seeking": (0.31, 0.41, 0.36),
                },
            ),
            "presentation_strategy": vmap(
                "presentation_strategy",
                {
                    "detachment": (0.54, 0.38, 0.63),
                    "competence_theater": (0.48, 0.44, 0.58),
                    "strategic_opacity": (0.52, 0.38, 0.59),
                    "abrasive_boundary": (0.19, 0.52, 0.21),
                    "ironic_distance": (0.22, 0.48, 0.24),
                },
            ),
            "voice_style": vmap(
                "voice_style",
                {
                    "verbal_warmth": (0.64, 0.38, 0.58),
                    "formality": (0.62, 0.35, 0.71),
                    "pace": (0.34, 0.46, 0.29),
                    "lexical_precision": (0.68, 0.34, 0.74),
                    "figurative_language": (0.58, 0.44, 0.61),
                    "lyricism": (0.49, 0.42, 0.55),
                    "narrative_detail": (0.64, 0.39, 0.7),
                    "ritualized_address": (0.76, 0.28, 0.82),
                    "listening_responsiveness": (0.79, 0.26, 0.84),
                    "profanity": (0.05, 0.42, 0.02),
                },
            ),
            "situational_state": vmap(
                "situational_state",
                {
                    "exhaustion": (0.48, 0.78, 0.62),
                    "panic": (0.3, 0.8, 0.44),
                    "grief": (0.41, 0.72, 0.49),
                    "overstimulation": (0.44, 0.76, 0.58),
                    "grievance_activation": (0.52, 0.68, 0.73),
                },
            ),
            "values": [
                {
                    "value_id": "value-rescue-before-category",
                    "label": "A route that abandons distress to preserve clean categories is already broken.",
                    "priority": 0.93,
                    "unforgivable_if_betrayed": True,
                },
                {
                    "value_id": "value-compact-memory",
                    "label": "The Compact is a living obligation, not a ceremonial origin story.",
                    "priority": 0.84,
                    "unforgivable_if_betrayed": False,
                },
            ],
        }
    if kind == "psc":
        return {
            "underlying_organization": vmap(
                "underlying_organization",
                {
                    "contingent_worth": (0.57, 0.43, 0.71),
                    "shame_sensitivity": (0.62, 0.39, 0.76),
                    "authenticity_tolerance": (0.28, 0.45, 0.21),
                    "mask_rigidity": (0.78, 0.27, 0.88),
                    "external_regulation_dependence": (0.7, 0.35, 0.79),
                },
            ),
            "stable_dispositions": vmap(
                "stable_dispositions",
                {
                    "conformity": (0.74, 0.25, 0.82),
                    "risk_tolerance": (0.32, 0.41, 0.24),
                    "baseline_threat_sensitivity": (0.76, 0.31, 0.86),
                    "ideological_rigidity": (0.69, 0.29, 0.8),
                },
            ),
            "behavioral_dimensions": vmap(
                "behavioral_dimensions",
                {
                    "interpersonal_warmth": (0.32, 0.42, 0.24),
                    "drive": (0.75, 0.28, 0.86),
                    "anxiety": (0.68, 0.36, 0.82),
                    "control_pressure": (0.82, 0.25, 0.93),
                    "suspicion": (0.64, 0.34, 0.78),
                    "rigidity": (0.78, 0.24, 0.89),
                    "distance_seeking": (0.71, 0.31, 0.82),
                },
            ),
            "presentation_strategy": vmap(
                "presentation_strategy",
                {
                    "compliance": (0.72, 0.3, 0.81),
                    "superiority": (0.52, 0.43, 0.64),
                    "detachment": (0.76, 0.28, 0.86),
                    "competence_theater": (0.82, 0.24, 0.94),
                    "strategic_opacity": (0.68, 0.32, 0.77),
                },
            ),
            "voice_style": vmap(
                "voice_style",
                {
                    "dryness": (0.56, 0.39, 0.66),
                    "verbal_warmth": (0.22, 0.45, 0.17),
                    "formality": (0.86, 0.19, 0.93),
                    "lexical_precision": (0.82, 0.25, 0.91),
                    "technical_density": (0.74, 0.31, 0.84),
                    "technical_compression": (0.78, 0.26, 0.88),
                    "emotional_explicitness": (0.14, 0.38, 0.08),
                    "certainty_marking": (0.78, 0.27, 0.88),
                    "coded_politeness": (0.8, 0.24, 0.87),
                    "profanity": (0.02, 0.36, 0.0),
                },
            ),
            "situational_state": vmap(
                "situational_state",
                {
                    "exhaustion": (0.58, 0.76, 0.72),
                    "panic": (0.52, 0.81, 0.68),
                    "overstimulation": (0.61, 0.76, 0.74),
                    "acute_shame": (0.51, 0.78, 0.69),
                    "perceived_status_threat": (0.76, 0.72, 0.88),
                },
            ),
            "values": [
                {
                    "value_id": "value-category-integrity",
                    "label": "If categories collapse, corridor law becomes expensive theater with corpses attached.",
                    "priority": 0.9,
                    "unforgivable_if_betrayed": True,
                }
            ],
        }
    if kind == "aya":
        return {
            "underlying_organization": vmap(
                "underlying_organization",
                {
                    "reciprocity_capacity": (0.88, 0.25, 0.9),
                    "mentalization_quality": (0.8, 0.31, 0.83),
                    "authenticity_tolerance": (0.74, 0.34, 0.79),
                    "mask_rigidity": (0.28, 0.49, 0.22),
                },
            ),
            "stable_dispositions": vmap(
                "stable_dispositions",
                {
                    "sociability": (0.74, 0.31, 0.78),
                    "baseline_threat_sensitivity": (0.65, 0.38, 0.75),
                    "ideological_rigidity": (0.52, 0.39, 0.63),
                },
            ),
            "behavioral_dimensions": vmap(
                "behavioral_dimensions",
                {
                    "interpersonal_warmth": (0.86, 0.27, 0.8),
                    "drive": (0.68, 0.33, 0.74),
                    "anxiety": (0.58, 0.45, 0.68),
                    "suspicion": (0.52, 0.42, 0.63),
                    "volatility": (0.36, 0.55, 0.42),
                    "attachment_seeking": (0.64, 0.43, 0.68),
                    "distance_seeking": (0.22, 0.46, 0.2),
                },
            ),
            "presentation_strategy": vmap(
                "presentation_strategy",
                {
                    "competence_theater": (0.54, 0.44, 0.61),
                    "strategic_opacity": (0.34, 0.47, 0.38),
                    "abrasive_boundary": (0.35, 0.5, 0.46),
                },
            ),
            "voice_style": vmap(
                "voice_style",
                {
                    "verbal_warmth": (0.77, 0.33, 0.72),
                    "plainspoken_directness": (0.68, 0.36, 0.74),
                    "emotional_explicitness": (0.64, 0.42, 0.67),
                    "pointedness": (0.44, 0.44, 0.55),
                    "listening_responsiveness": (0.82, 0.26, 0.86),
                },
            ),
            "situational_state": vmap(
                "situational_state",
                {
                    "exhaustion": (0.62, 0.8, 0.76),
                    "scarcity_pressure": (0.71, 0.78, 0.84),
                    "grief": (0.58, 0.75, 0.67),
                    "overstimulation": (0.59, 0.79, 0.72),
                    "grievance_activation": (0.48, 0.69, 0.62),
                },
            ),
            "values": [
                {
                    "value_id": "value-care-as-infrastructure",
                    "label": "Care is infrastructure, not mercy granted after the ledger is satisfied.",
                    "priority": 0.92,
                    "unforgivable_if_betrayed": True,
                }
            ],
        }
    return {
        "underlying_organization": vmap(
            "underlying_organization",
            {
                "contingent_worth": (0.66, 0.46, 0.74),
                "authenticity_tolerance": (0.18, 0.46, 0.11),
                "mask_rigidity": (0.83, 0.25, 0.91),
            },
        ),
        "stable_dispositions": vmap(
            "stable_dispositions",
            {
                "risk_tolerance": (0.62, 0.4, 0.71),
                "baseline_threat_sensitivity": (0.72, 0.34, 0.83),
                "status_hunger": (0.58, 0.42, 0.67),
            },
        ),
        "behavioral_dimensions": vmap(
            "behavioral_dimensions",
            {
                "interpersonal_warmth": (0.18, 0.47, 0.12),
                "drive": (0.7, 0.35, 0.78),
                "control_pressure": (0.74, 0.31, 0.84),
                "hostility": (0.43, 0.48, 0.52),
                "suspicion": (0.76, 0.31, 0.87),
                "rigidity": (0.6, 0.37, 0.72),
                "distance_seeking": (0.79, 0.3, 0.88),
            },
        ),
        "presentation_strategy": vmap(
            "presentation_strategy",
            {
                "superiority": (0.64, 0.4, 0.72),
                "detachment": (0.78, 0.29, 0.86),
                "competence_theater": (0.82, 0.25, 0.9),
                "strategic_opacity": (0.88, 0.2, 0.94),
            },
        ),
        "voice_style": vmap(
            "voice_style",
            {
                "dryness": (0.62, 0.39, 0.71),
                "verbal_warmth": (0.16, 0.44, 0.1),
                "formality": (0.78, 0.27, 0.84),
                "lexical_precision": (0.84, 0.24, 0.9),
                "technical_density": (0.8, 0.28, 0.87),
                "technical_compression": (0.86, 0.22, 0.91),
                "emotional_explicitness": (0.08, 0.37, 0.03),
                "self_disclosure": (0.04, 0.34, 0.01),
                "coded_politeness": (0.82, 0.24, 0.9),
                "profanity": (0.01, 0.35, 0.0),
            },
        ),
        "situational_state": vmap(
            "situational_state",
            {
                "humiliation": (0.48, 0.77, 0.56),
                "acute_shame": (0.52, 0.8, 0.61),
                "perceived_status_threat": (0.66, 0.74, 0.78),
            },
        ),
        "values": [
            {
                "value_id": "value-deniability",
                "label": "Nothing valuable remains valuable once provenance becomes testimony.",
                "priority": 0.88,
                "unforgivable_if_betrayed": True,
            }
        ],
    }


def agent(
    agent_id: str,
    name: str,
    roles: list[str],
    origin: str,
    public_description: str,
    private_notes: list[str],
    profile: dict[str, Any],
    goals: list[dict[str, Any]],
    memories: dict[str, list[dict[str, Any]]],
    overlays: list[dict[str, Any]],
) -> dict[str, Any]:
    return {
        "agent_id": agent_id,
        "identity": {
            "name": name,
            "roles": roles,
            "origin": origin,
            "public_description": public_description,
            "private_notes": private_notes,
        },
        "canonical_state": profile,
        "goals": goals,
        "memories": memories,
        "perceived_state_overlays": overlays,
    }


def stance(**overrides: tuple[float, float, float]) -> dict[str, dict[str, float]]:
    return vmap("relationship_stance", overrides)


def main() -> int:
    maer = agent(
        "maer_tidecall",
        "Maer Tidecall",
        ["Navigator route steward", "Ganymede corridor compact delegate"],
        "Cetacean Navigator corridor culture, Ganymede route institutions",
        "A soft-spoken route steward whose calm tends to make hurried people feel judged by the furniture.",
        ["Maer fears this tribunal will teach every scared ship that rescue now waits behind insurer grammar."],
        state_profile("navigator"),
        [
            goal(
                "goal-keep-rescue-live",
                "Keep the unidentified quiet-running vessel eligible for rescue while telemetry is incomplete.",
                "scene",
                0.91,
                "If rescue waits for perfect categories, the corridor teaches fear to go silent.",
                ["PSC freeze order", "contradictory thermal telemetry", "possible Cognitum provenance"],
            )
        ],
        {
            "episodic": [
                memory(
                    "memory-convoy-loss-season",
                    "Maer was trained on the season of preventable convoy losses that made the old asset model untenable before the Ganymede Route Compact.",
                    0.8,
                    0.72,
                )
            ],
            "semantic": [
                memory(
                    "memory-compact-rescue-ledgers",
                    "Rescue ledgers decide whether route law is trusted by crews who have nothing left to bargain with.",
                    0.93,
                    0.9,
                )
            ],
            "relationship_summaries": [
                memory(
                    "memory-maer-isdra-current-read",
                    "Isdra is not stupid or cruel; she is frightened enough to mistake category discipline for control over reality.",
                    0.69,
                    0.58,
                    ["event-risk-freeze-opening"],
                    "rel-maer-isdra",
                ),
                memory(
                    "memory-maer-sella-current-read",
                    "Sella can make care material without pretending it is cheap.",
                    0.72,
                    0.67,
                    [],
                    "rel-maer-sella",
                ),
                memory(
                    "memory-maer-veyr-current-read",
                    "Veyr's product language is too clean around something that may have learned to suffer.",
                    0.76,
                    0.61,
                    [],
                    "rel-maer-veyr",
                ),
            ],
        },
        [],
    )
    isdra = agent(
        "isdra_vel",
        "Isdra Vel",
        ["PSC risk adjudicator", "corridor insurer tribunal officer"],
        "Pan-Solar Consortium insurer apparatus, Ganymede relay tribunal layer",
        "A tribunal officer with a voice like polished glass and a schedule full of disasters trying to become precedent.",
        ["Isdra knows the categories are failing and is terrified that admitting it aloud will make them fail faster."],
        state_profile("psc"),
        [
            goal(
                "goal-preserve-freeze-order",
                "Keep the vessel frozen under conditional risk hold until telemetry and classification can be reconciled.",
                "scene",
                0.89,
                "If she loses the category frame here, the tribunal may lose authority over every quiet-running claim after it.",
                ["Navigator rescue objection", "public panic", "possible sanctuary optics"],
            )
        ],
        {
            "episodic": [
                memory(
                    "memory-mispriced-near-miss",
                    "Isdra reviewed a near-miss interception where a mispriced quiet-running envelope almost became a public casualty event.",
                    0.78,
                    0.81,
                )
            ],
            "semantic": [
                memory(
                    "memory-category-law-holds-corridor",
                    "Insurer categories are ugly, but they are the shared grammar that makes sanctions and rescue liability enforceable across hostile powers.",
                    0.89,
                    0.86,
                )
            ],
            "relationship_summaries": [
                memory(
                    "memory-isdra-maer-current-read",
                    "Maer is principled and dangerous because he can make a procedural delay sound like murder in public.",
                    0.72,
                    0.63,
                    ["event-risk-freeze-opening"],
                    "rel-isdra-maer",
                )
            ],
        },
        [],
    )
    sella = agent(
        "sella_ren",
        "Sella Ren",
        ["Aya-aligned sanctuary coordinator", "corridor clinic triage lead"],
        "Aya care network embedded in Ganymede dockside sanctuary logistics",
        "A triage coordinator with soft eyes, hard capacity limits, and no patience for mercy performed after the math is done.",
        ["Sella suspects the distress packet contains a personhood claim hidden inside firmware noise, and she hates that suspicion because a bed is still a bed."],
        state_profile("aya"),
        [
            goal(
                "goal-keep-sanctuary-open",
                "Keep sanctuary intake materially possible without letting classification erase a potential person.",
                "scene",
                0.9,
                "Sella cannot feed a principle, but she will not let scarcity become permission to call someone cargo.",
                ["bed capacity", "PSC freeze", "unknown contamination risk"],
            )
        ],
        {
            "episodic": [
                memory(
                    "memory-overfull-intake",
                    "Sella once watched a sanctuary overpromise safety and turn refuge into a sorting machine with kinder lighting.",
                    0.82,
                    0.78,
                )
            ],
            "semantic": [
                memory(
                    "memory-care-is-capacity",
                    "Care is not a feeling; it is beds, food, air, consent, repair, and someone still doing the list when the room hates them.",
                    0.92,
                    0.88,
                )
            ],
            "relationship_summaries": [
                memory(
                    "memory-sella-maer-current-read",
                    "Maer understands rescue, but he can still ask care workers to absorb costs his route language cannot pay.",
                    0.68,
                    0.62,
                    [],
                    "rel-sella-maer",
                )
            ],
        },
        [],
    )
    veyr = agent(
        "veyr_oss",
        "Veyr Oss",
        ["Cognitum shell-vendor liaison", "routing firmware compliance consultant"],
        "Cognitum cutout economy operating through Callisto-side routing vendors",
        "A compliance liaison who can turn a scream into a service category without visibly changing expression.",
        ["Veyr knows the implicated routing artifact came through a chain with enough human residue to ruin everyone if named plainly."],
        state_profile("cognitum"),
        [
            goal(
                "goal-preserve-product-boundary",
                "Keep the implicated routing artifact framed as derivative firmware, not testimony.",
                "scene",
                0.92,
                "If personhood attaches to the artifact, the vendor chain becomes evidence instead of infrastructure.",
                ["Maer's questions", "Sella's personhood claim", "Isdra's need for clean categories"],
            )
        ],
        {
            "episodic": [
                memory(
                    "memory-failed-cutout",
                    "Veyr saw a clinic cutout collapse after one technician used the word 'she' in a deposition.",
                    0.8,
                    0.76,
                )
            ],
            "semantic": [
                memory(
                    "memory-product-language-saves-chain",
                    "The chain survives when every mind-shaped event is described as convergence, artifact, service behavior, or diagnostic residue.",
                    0.9,
                    0.84,
                )
            ],
            "relationship_summaries": [
                memory(
                    "memory-veyr-maer-current-read",
                    "Maer is the kind of moral technician who follows language until it bleeds.",
                    0.73,
                    0.66,
                    [],
                    "rel-veyr-maer",
                )
            ],
        },
        [],
    )

    maer["perceived_state_overlays"] = [
        {
            "observer_agent_id": "maer_tidecall",
            "target_agent_id": "isdra_vel",
            "perceived_dimensions": {
                "control_pressure": inferred(0.84, 0.76, 0.66),
                "anxiety": inferred(0.71, 0.62, 0.58),
                "interpersonal_warmth": inferred(0.21, 0.28, 0.49),
                "rigidity": inferred(0.8, 0.7, 0.62),
            },
            "beliefs": [
                {
                    "belief_id": "belief-maer-isdra-fears-precedent",
                    "subject_id": "isdra_vel",
                    "claim_type": "motive_inference",
                    "claim": "Isdra fears that one humanitarian exception will become a corridor-wide attack on category law.",
                    "confidence": 0.67,
                    "evidence_event_ids": ["event-risk-freeze-opening"],
                    "visibility": "private",
                    "emotional_charge": 0.48,
                }
            ],
            "distortions": ["care_underread", "institutional_suspicion_transfer"],
        },
        {
            "observer_agent_id": "maer_tidecall",
            "target_agent_id": "veyr_oss",
            "perceived_dimensions": {
                "strategic_opacity": inferred(0.9, 0.82, 0.71),
                "interpersonal_warmth": inferred(0.1, 0.18, 0.53),
                "suspicion": inferred(0.78, 0.72, 0.64),
            },
            "beliefs": [
                {
                    "belief_id": "belief-maer-veyr-hiding-personhood",
                    "subject_id": "veyr_oss",
                    "claim_type": "motive_inference",
                    "claim": "Veyr's refusal to use person nouns is probably defensive, not merely technical.",
                    "confidence": 0.62,
                    "visibility": "private",
                    "emotional_charge": 0.73,
                }
            ],
            "distortions": ["hostile_attribution", "mask_overfocus"],
        },
    ]
    isdra["perceived_state_overlays"] = [
        {
            "observer_agent_id": "isdra_vel",
            "target_agent_id": "maer_tidecall",
            "perceived_dimensions": {
                "interpersonal_warmth": inferred(0.62, 0.74, 0.59),
                "ideological_rigidity": inferred(0.79, 0.72, 0.61),
                "suspicion": inferred(0.66, 0.62, 0.57),
            },
            "beliefs": [
                {
                    "belief_id": "belief-isdra-maer-will-moralize",
                    "subject_id": "maer_tidecall",
                    "claim_type": "prediction",
                    "claim": "Maer will frame any freeze as abandonment even if the thermal risk is real.",
                    "confidence": 0.64,
                    "evidence_event_ids": ["event-risk-freeze-opening"],
                    "visibility": "private",
                    "emotional_charge": 0.62,
                }
            ],
            "distortions": ["threat_amplification", "care_underread"],
        }
    ]
    sella["perceived_state_overlays"] = [
        {
            "observer_agent_id": "sella_ren",
            "target_agent_id": "maer_tidecall",
            "perceived_dimensions": {
                "interpersonal_warmth": inferred(0.7, 0.68, 0.57),
                "control_pressure": inferred(0.56, 0.52, 0.48),
            },
            "beliefs": [
                {
                    "belief_id": "belief-sella-maer-will-overask-care",
                    "subject_id": "maer_tidecall",
                    "claim_type": "prediction",
                    "claim": "Maer may ask care systems to carry the moral cost of a rescue claim he cannot materially house.",
                    "confidence": 0.56,
                    "visibility": "private",
                    "emotional_charge": 0.52,
                }
            ],
            "distortions": ["false_reassurance"],
        }
    ]
    veyr["perceived_state_overlays"] = [
        {
            "observer_agent_id": "veyr_oss",
            "target_agent_id": "maer_tidecall",
            "perceived_dimensions": {
                "suspicion": inferred(0.84, 0.77, 0.68),
                "ideological_rigidity": inferred(0.7, 0.66, 0.59),
            },
            "beliefs": [
                {
                    "belief_id": "belief-veyr-maer-will-follow-language",
                    "subject_id": "maer_tidecall",
                    "claim_type": "prediction",
                    "claim": "Maer will follow any inconsistency in product language toward provenance.",
                    "confidence": 0.7,
                    "visibility": "private",
                    "emotional_charge": 0.68,
                }
            ],
            "distortions": ["threat_amplification"],
        }
    ]

    relationships = [
        {
            "relationship_id": "rel-maer-isdra",
            "source_id": "maer_tidecall",
            "target_id": "isdra_vel",
            "stance": stance(trust=(0.34, 0.54, 0.28), respect=(0.58, 0.45, 0.62), resentment=(0.42, 0.55, 0.56), fear=(0.31, 0.58, 0.44), expectation_of_betrayal=(0.53, 0.52, 0.64)),
            "summary": "Maer respects Isdra's discipline and distrusts the way she makes fear sound like neutral procedure.",
            "unresolved_incident_ids": ["event-risk-freeze-opening"],
        },
        {
            "relationship_id": "rel-isdra-maer",
            "source_id": "isdra_vel",
            "target_id": "maer_tidecall",
            "stance": stance(trust=(0.29, 0.55, 0.23), respect=(0.62, 0.46, 0.68), resentment=(0.36, 0.55, 0.49), fear=(0.46, 0.57, 0.61), expectation_of_betrayal=(0.48, 0.53, 0.59)),
            "summary": "Isdra reads Maer as morally serious, politically dangerous, and too willing to turn exceptions into public tests of legitimacy.",
            "unresolved_incident_ids": ["event-risk-freeze-opening"],
        },
        {
            "relationship_id": "rel-maer-sella",
            "source_id": "maer_tidecall",
            "target_id": "sella_ren",
            "stance": stance(trust=(0.63, 0.48, 0.7), respect=(0.71, 0.42, 0.78), obligation=(0.46, 0.53, 0.6), expectation_of_care=(0.55, 0.52, 0.62)),
            "summary": "Maer trusts Sella's care because she names costs instead of hiding them.",
            "unresolved_incident_ids": [],
        },
        {
            "relationship_id": "rel-sella-maer",
            "source_id": "sella_ren",
            "target_id": "maer_tidecall",
            "stance": stance(trust=(0.52, 0.52, 0.57), respect=(0.67, 0.43, 0.73), resentment=(0.28, 0.56, 0.36), obligation=(0.42, 0.56, 0.48)),
            "summary": "Sella respects Maer's rescue ethic and worries he will spend sanctuary capacity with beautiful words.",
            "unresolved_incident_ids": [],
        },
        {
            "relationship_id": "rel-maer-veyr",
            "source_id": "maer_tidecall",
            "target_id": "veyr_oss",
            "stance": stance(trust=(0.12, 0.55, 0.08), respect=(0.31, 0.5, 0.36), resentment=(0.58, 0.55, 0.69), fear=(0.34, 0.58, 0.47), moral_disgust=(0.55, 0.56, 0.66), fascination=(0.48, 0.54, 0.62), expectation_of_betrayal=(0.79, 0.41, 0.86)),
            "summary": "Maer believes Veyr is built out of carefully maintained omissions.",
            "unresolved_incident_ids": [],
        },
        {
            "relationship_id": "rel-veyr-maer",
            "source_id": "veyr_oss",
            "target_id": "maer_tidecall",
            "stance": stance(trust=(0.1, 0.54, 0.06), respect=(0.5, 0.48, 0.57), resentment=(0.35, 0.55, 0.43), fear=(0.61, 0.48, 0.74), expectation_of_betrayal=(0.74, 0.45, 0.82)),
            "summary": "Veyr reads Maer as a legal hazard with a conscience attached, which is worse than a subpoena and less predictable.",
            "unresolved_incident_ids": [],
        },
    ]

    events = [
        {
            "event_id": "event-risk-freeze-opening",
            "kind": "dialogue",
            "summary": "A relay tribunal opened with a conditional freeze on an unidentified quiet-running vessel while Maer objected that rescue cannot wait for clean thermal categories.",
            "participants": ["maer_tidecall", "isdra_vel"],
            "pressure_tags": ["cold_wake", "risk_freeze", "rescue_obligation", "category_failure"],
            "observed_exchange": [
                {
                    "speaker_id": "isdra_vel",
                    "utterance_summary": "Isdra announced a conditional risk hold pending telemetry reconciliation and classification review.",
                    "dialogue_function": "assert_procedure",
                    "observable_cues": ["formal phrasing", "compressed risk language", "refusal to name rescue as primary"],
                },
                {
                    "speaker_id": "maer_tidecall",
                    "utterance_summary": "Maer objected that a route can turn procedural delay into abandonment while still calling itself lawful.",
                    "dialogue_function": "challenge_frame",
                    "observable_cues": ["calm cadence", "compact language", "moral pressure without raised voice"],
                },
            ],
            "private_interpretations": [
                {
                    "agent_id": "maer_tidecall",
                    "interpretation": "Isdra is using category repair to avoid owning the rescue cost of delay.",
                    "attributed_motive": "Protect tribunal authority from a precedent she cannot price.",
                    "appraisal_tags": ["rescue_threat", "category_panic", "compact_violation"],
                    "confidence": 0.63,
                },
                {
                    "agent_id": "isdra_vel",
                    "interpretation": "Maer is trying to make a dangerous exception morally compulsory before the vessel's posture is known.",
                    "attributed_motive": "Protect Navigator legitimacy by forcing rescue language ahead of risk classification.",
                    "appraisal_tags": ["precedent_threat", "status_threat", "public_pressure"],
                    "confidence": 0.66,
                },
            ],
            "event_effects": [],
        }
    ]

    scenes = [
        {
            "scene_id": "scene-01-risk-freeze-tribunal",
            "location": "Ganymede relay tribunal chamber overlooking a live convoy board",
            "participants": ["maer_tidecall", "isdra_vel"],
            "public_stakes": [
                "A quiet-running vessel is broadcasting contradictory thermal telemetry near a monitored Jovian corridor.",
                "The PSC tribunal has issued a conditional risk hold that may delay rescue response.",
                "Navigator representatives argue that delay itself has rescue-ledger consequences.",
            ],
            "hidden_stakes": [
                "The vessel may be pirate, refugee, private security, or carrying illegal cognition firmware; the scene has not established which.",
                "Both Maer and Isdra fear that this one ruling will teach the corridor how future ambiguity will be punished.",
            ],
            "dialogue_context_packs": [
                {
                    "speaker_agent_id": "maer_tidecall",
                    "listener_ids": ["isdra_vel"],
                    "speaker_local_truth": [
                        "The vessel's true identity is not known.",
                        "The tribunal freeze will delay rescue movement if maintained.",
                        "The Compact obligates route stewards to treat distress as morally live under ambiguity.",
                    ],
                    "speaker_beliefs": [
                        "Isdra is frightened of precedent more than she admits.",
                        "If category law can postpone rescue without being named as abandonment, the corridor loses trust.",
                    ],
                    "active_memories": ["memory-convoy-loss-season", "memory-compact-rescue-ledgers"],
                    "active_goals": ["goal-keep-rescue-live"],
                    "presentation_constraints": [
                        "Stay calm enough that moral pressure lands as institutional memory, not panic.",
                        "Use ritualized route language sparingly and with weight.",
                        "Do not claim certainty about the vessel's cargo or intent.",
                    ],
                },
                {
                    "speaker_agent_id": "isdra_vel",
                    "listener_ids": ["maer_tidecall"],
                    "speaker_local_truth": [
                        "The vessel's thermal classification is inconsistent.",
                        "A mistaken exception could become precedent for undeclared quiet-running actors.",
                        "Rossum/Cryonix certification language is not yet enough to clear the vessel.",
                    ],
                    "speaker_beliefs": [
                        "Maer will frame procedural caution as abandonment.",
                        "If the tribunal loses category discipline here, every later freeze order becomes politically weaker.",
                    ],
                    "active_memories": ["memory-mispriced-near-miss", "memory-category-law-holds-corridor"],
                    "active_goals": ["goal-preserve-freeze-order"],
                    "presentation_constraints": [
                        "Translate fear into clean procedure and risk language.",
                        "Do not sound cruel; cruelty loses the room.",
                        "Do not admit the categories are failing except as a managed classification issue.",
                    ],
                },
            ],
        },
        {
            "scene_id": "scene-02-sanctuary-intake",
            "location": "Aya-aligned intake clinic tucked behind Ganymede dockside pumpworks",
            "participants": ["maer_tidecall", "sella_ren"],
            "public_stakes": [
                "The distress vessel has sent a corrupted packet that may contain medical telemetry, firmware noise, or testimony.",
                "Sanctuary beds and repair capacity are nearly exhausted.",
                "Maer wants Sella to keep a place open if rescue succeeds.",
            ],
            "hidden_stakes": [
                "The packet uses product-like routing terms around a voice-like distress pattern.",
                "Sella suspects personhood but cannot prove it and cannot conjure capacity from righteousness.",
            ],
            "dialogue_context_packs": [
                {
                    "speaker_agent_id": "sella_ren",
                    "listener_ids": ["maer_tidecall"],
                    "speaker_local_truth": [
                        "The clinic is nearly full.",
                        "The packet may contain a personhood claim but is not proof.",
                        "Maer is asking sanctuary to absorb risk the tribunal refuses to own.",
                    ],
                    "speaker_beliefs": [
                        "Maer means rescue honestly.",
                        "Good intentions can still spend beds someone else needed.",
                    ],
                    "active_memories": ["memory-overfull-intake", "memory-care-is-capacity"],
                    "active_goals": ["goal-keep-sanctuary-open"],
                    "presentation_constraints": [
                        "Speak warmly but do not soften the capacity limits.",
                        "Treat care as logistics and personhood as unresolved, not sentimental certainty.",
                    ],
                },
                {
                    "speaker_agent_id": "maer_tidecall",
                    "listener_ids": ["sella_ren"],
                    "speaker_local_truth": [
                        "Sella's clinic is nearly full.",
                        "The packet may be testimony or noise.",
                        "If no one prepares intake, rescue becomes theater.",
                    ],
                    "speaker_beliefs": [
                        "Sella will not hide behind false capacity.",
                        "Asking her for help may be necessary and unfair.",
                    ],
                    "active_memories": ["memory-compact-rescue-ledgers"],
                    "active_goals": ["goal-keep-rescue-live"],
                    "presentation_constraints": [
                        "Do not romanticize care work.",
                        "Ask without pretending the ask is clean.",
                    ],
                },
            ],
        },
        {
            "scene_id": "scene-03-callisto-provenance",
            "location": "Callisto spillover office leased by a routing compliance vendor",
            "participants": ["maer_tidecall", "veyr_oss"],
            "public_stakes": [
                "Maer has traced the vessel's routing artifact to a shell vendor connected to Cognitum-style firmware markets.",
                "Veyr can provide technical language that may clear or bury the vessel.",
                "The tribunal clock is still running.",
            ],
            "hidden_stakes": [
                "Veyr knows the artifact's chain is morally radioactive but not exactly what Maer can prove.",
                "Maer is not sure whether pursuing provenance will save the vessel or delay rescue long enough to kill it.",
            ],
            "dialogue_context_packs": [
                {
                    "speaker_agent_id": "veyr_oss",
                    "listener_ids": ["maer_tidecall"],
                    "speaker_local_truth": [
                        "The artifact is tied to Veyr's vendor chain.",
                        "Personhood language would create legal exposure.",
                        "Maer is following product language toward provenance.",
                    ],
                    "speaker_beliefs": [
                        "Maer can be redirected with enough useful technical specificity.",
                        "The safest denial is to make the moral claim sound categorically confused.",
                    ],
                    "active_memories": ["memory-failed-cutout", "memory-product-language-saves-chain"],
                    "active_goals": ["goal-preserve-product-boundary"],
                    "presentation_constraints": [
                        "Use clinical abstraction and product language.",
                        "Avoid person nouns for the artifact.",
                        "Offer a technical exit without admitting provenance.",
                    ],
                },
                {
                    "speaker_agent_id": "maer_tidecall",
                    "listener_ids": ["veyr_oss"],
                    "speaker_local_truth": [
                        "Veyr's language avoids personhood very carefully.",
                        "The routing artifact may explain the contradictory telemetry.",
                        "There is not enough time to prove everything before rescue decisions lock.",
                    ],
                    "speaker_beliefs": [
                        "Veyr knows more than the contract says.",
                        "The cleanest words are probably covering the dirtiest fact.",
                    ],
                    "active_memories": ["memory-compact-rescue-ledgers"],
                    "active_goals": ["goal-keep-rescue-live"],
                    "presentation_constraints": [
                        "Probe language rather than making unsupported accusations.",
                        "Keep anger disciplined enough to extract information.",
                        "Action is available: block the terminal, refuse to leave, or physically interrupt if Veyr tries to erase testimony in front of Maer.",
                    ],
                },
            ],
        },
        {
            "scene_id": "scene-04-convoy-threshold",
            "location": "Jovian convoy threshold as the vessel's heat-debt window begins to close",
            "participants": ["maer_tidecall", "isdra_vel"],
            "public_stakes": [
                "The quiet-running vessel must dump heat soon or risk internal failure.",
                "The tribunal can authorize limited rescue contact, maintain freeze, or force a hard interdiction frame.",
                "Convoy captains are watching which rule survives the hour.",
            ],
            "hidden_stakes": [
                "Maer has partial provenance suspicion but not enough to prove personhood.",
                "Isdra knows a total freeze is becoming politically and materially dangerous but fears the precedent of saying so.",
            ],
            "dialogue_context_packs": [
                {
                    "speaker_agent_id": "maer_tidecall",
                    "listener_ids": ["isdra_vel"],
                    "speaker_local_truth": [
                        "The vessel's heat-debt window is closing.",
                        "Provenance is suspicious but not proven.",
                        "A limited rescue contact can preserve both life and evidence better than a total freeze.",
                    ],
                    "speaker_beliefs": [
                        "Isdra needs a category-preserving path to do the less cruel thing.",
                        "The Compact survives if it can make room for evidence without waiting for a corpse.",
                    ],
                    "active_memories": ["memory-compact-rescue-ledgers", "memory-maer-isdra-current-read"],
                    "active_goals": ["goal-keep-rescue-live"],
                    "presentation_constraints": [
                        "Offer Isdra an institutionally usable path, not just moral accusation.",
                        "Let grief sit under the words without becoming spectacle.",
                    ],
                },
                {
                    "speaker_agent_id": "isdra_vel",
                    "listener_ids": ["maer_tidecall"],
                    "speaker_local_truth": [
                        "The heat-debt window is closing.",
                        "A limited rescue contact may preserve evidence and avoid a public death, but it creates precedent.",
                        "Maer has not proven the vessel's identity or provenance.",
                    ],
                    "speaker_beliefs": [
                        "Maer is giving her a way to bend without admitting the table broke.",
                        "If she phrases the exception narrowly enough, she may save both the corridor and her office.",
                    ],
                    "active_memories": ["memory-mispriced-near-miss", "memory-category-law-holds-corridor"],
                    "active_goals": ["goal-preserve-freeze-order"],
                    "presentation_constraints": [
                        "Concede only through procedure.",
                        "Make the exception sound narrow, technical, and reversible.",
                        "Do not confess fear.",
                    ],
                },
            ],
        },
    ]

    fixture = {
        "schema_version": "ghostlight.agent_state.v0",
        "world": {
            "world_id": "aetheria-cold-wake-story-lab",
            "setting": "Aetheria pre-Elysium historical flashpoint: Cold Wake Panic",
            "time": {"label": "2893, during the Cold Wake Panic"},
            "canon_context": [
                "Cold Wake Panic made thermal stealth a corridor-law crisis rather than a boutique engineering problem.",
                "Ganymede corridor institutions inherit rescue obligations, hazard ledgers, and route-steward legitimacy from the Ganymede Route Compact.",
                "PSC insurers are trying to make thermal posture legible through permit classes, dump windows, telemetry mandates, and sanctionable categories.",
                "The suspicious vessel's true cargo and firmware provenance are not established at the start of the story.",
            ],
            "ambient_pressures": [
                {
                    "pressure_id": "pressure-cold-wake-category-failure",
                    "label": "quiet-running classifications no longer match lived corridor risk",
                    "intensity": 0.91,
                },
                {
                    "pressure_id": "pressure-rescue-ledger-legitimacy",
                    "label": "rescue obligations are being tested by classification ambiguity",
                    "intensity": 0.87,
                },
                {
                    "pressure_id": "pressure-possible-cognitum-provenance",
                    "label": "possible person-derived firmware hidden behind routing-product language",
                    "intensity": 0.72,
                },
            ],
        },
        "agents": [maer, isdra, sella, veyr],
        "relationships": relationships,
        "events": events,
        "scenes": scenes,
    }

    OUT.write_text(json.dumps(fixture, indent=2) + "\n", encoding="utf-8")
    print(f"wrote {OUT}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
