You are helping Ghostlight generate branch candidates for an Ink interactive fiction scene.
Return only valid JSON matching the provided contract.

{
  "task": "Generate candidate Ink branches for one Aetheria interactive fiction scene.",
  "output_contract": {
    "format": "Return only JSON. No markdown. No commentary.",
    "root_keys": [
      "branches"
    ],
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
      "training_hooks"
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
      "mixed"
    ]
  },
  "rules": [
    "Generate player-facing choices for Maer, not a linear story.",
    "Include at least two non-speech action types.",
    "Do not resolve whether the corrupted packet contains a person.",
    "Do not let Sella accept without cost, condition, authority defense, or resentment risk.",
    "Keep Sella's response separate from Maer's intent; she may misread or bound his move.",
    "Do not invent new lore facts beyond the provided context.",
    "Make each branch playable in Ink as a knot reached from a choice.",
    "Use concise, readable prose. The scene should be usable Aetheria lore content, not a schema wearing boots."
  ],
  "scene": {
    "scene_id": "scene-02-sanctuary-intake",
    "location": "Aya-aligned intake clinic tucked behind Ganymede dockside pumpworks",
    "public_stakes": [
      "The distress vessel has sent a corrupted packet that may contain medical telemetry, firmware noise, or testimony.",
      "Sanctuary beds and repair capacity are nearly exhausted.",
      "Maer wants Sella to keep a place open if rescue succeeds."
    ],
    "hidden_stakes": [
      "The packet uses product-like routing terms around a voice-like distress pattern.",
      "Sella suspects personhood but cannot prove it and cannot conjure capacity from righteousness."
    ]
  },
  "actor": {
    "agent_id": "maer_tidecall",
    "identity": {
      "name": "Maer Tidecall",
      "roles": [
        "Navigator route steward",
        "Ganymede corridor compact delegate"
      ],
      "origin": "Cetacean Navigator corridor culture, Ganymede route institutions",
      "public_description": "A soft-spoken route steward whose calm tends to make hurried people feel judged by the furniture.",
      "private_notes": [
        "Maer fears this tribunal will teach every scared ship that rescue now waits behind insurer grammar."
      ]
    },
    "local_truth": [
      "Sella's clinic is nearly full.",
      "The packet may be testimony or noise.",
      "If no one prepares intake, rescue becomes theater."
    ],
    "beliefs": [
      "Sella will not hide behind false capacity.",
      "Asking her for help may be necessary and unfair."
    ],
    "goals": [
      "goal-keep-rescue-live: Keep the unidentified quiet-running vessel eligible for rescue while telemetry is incomplete. Priority 0.91. Stake: If rescue waits for perfect categories, the corridor teaches fear to go silent."
    ],
    "memories": [
      "memory-compact-rescue-ledgers: Rescue ledgers decide whether route law is trusted by crews who have nothing left to bargain with."
    ],
    "presentation_constraints": [
      "Do not romanticize care work.",
      "Ask without pretending the ask is clean."
    ],
    "selected_current_activation": {
      "behavioral_dimensions": {
        "interpersonal_warmth": 0.66,
        "control_pressure": 0.67,
        "suspicion": 0.71,
        "rigidity": 0.62,
        "volatility": 0.29
      },
      "presentation_strategy": {
        "detachment": 0.63,
        "competence_theater": 0.58,
        "strategic_opacity": 0.59,
        "ironic_distance": 0.24
      },
      "voice_style": {
        "formality": 0.71,
        "lexical_precision": 0.74,
        "ritualized_address": 0.82,
        "verbal_warmth": 0.58,
        "pointedness": 0.5
      },
      "situational_state": {
        "exhaustion": 0.62,
        "panic": 0.44,
        "grievance_activation": 0.73,
        "overstimulation": 0.58
      }
    }
  },
  "npcs": [
    {
      "agent_id": "sella_ren",
      "identity": {
        "name": "Sella Ren",
        "roles": [
          "Aya-aligned sanctuary coordinator",
          "corridor clinic triage lead"
        ],
        "origin": "Aya care network embedded in Ganymede dockside sanctuary logistics",
        "public_description": "A triage coordinator with soft eyes, hard capacity limits, and no patience for mercy performed after the math is done.",
        "private_notes": [
          "Sella suspects the distress packet contains a personhood claim hidden inside firmware noise, and she hates that suspicion because a bed is still a bed."
        ]
      },
      "selected_current_activation": {
        "behavioral_dimensions": {
          "interpersonal_warmth": 0.8,
          "control_pressure": 0.5,
          "suspicion": 0.63,
          "rigidity": 0.5,
          "volatility": 0.42
        },
        "presentation_strategy": {
          "detachment": 0.5,
          "competence_theater": 0.61,
          "strategic_opacity": 0.38,
          "ironic_distance": 0.5
        },
        "voice_style": {
          "formality": 0.5,
          "lexical_precision": 0.5,
          "ritualized_address": 0.5,
          "verbal_warmth": 0.72,
          "pointedness": 0.55
        },
        "situational_state": {
          "exhaustion": 0.76,
          "panic": 0.5,
          "grievance_activation": 0.62,
          "overstimulation": 0.72
        }
      }
    }
  ],
  "relationships": [
    "Maer trusts Sella's care because she names costs instead of hiding them.",
    "Sella respects Maer's rescue ethic and worries he will spend sanctuary capacity with beautiful words."
  ],
  "projection_controls": {
    "frame_controls": [
      {
        "text": "Frame the scene as capacity negotiation under uncertainty, not a morality duel with a clean answer.",
        "source_refs": [
          "scenes.scene-02-sanctuary-intake.public_stakes",
          "scenes.scene-02-sanctuary-intake.hidden_stakes"
        ]
      }
    ],
    "authority_boundaries": [
      {
        "text": "Maer can ask, offer ledger backing, show or withhold the packet, and incur obligation; Sella controls sanctuary intake.",
        "source_refs": [
          "scenes.scene-02-sanctuary-intake.dialogue_context_packs",
          "agents.sella_ren.goals.goal-keep-sanctuary-open"
        ]
      }
    ],
    "object_custody": [
      {
        "text": "The corrupted packet can be shown, withheld, or summarized, but it cannot be treated as clean proof.",
        "source_refs": [
          "scenes.scene-02-sanctuary-intake.public_stakes",
          "scenes.scene-02-sanctuary-intake.hidden_stakes"
        ]
      }
    ],
    "required_semantics": [
      {
        "text": "Care must remain logistics and obligation, not just compassion.",
        "source_refs": [
          "agents.sella_ren.memories.memory-care-is-capacity"
        ]
      }
    ],
    "forbidden_resolutions": [
      {
        "text": "Do not resolve whether the packet contains a person.",
        "source_refs": [
          "scenes.scene-02-sanctuary-intake.hidden_stakes"
        ]
      },
      {
        "text": "Do not make Sella accept without cost, condition, or resentment risk.",
        "source_refs": [
          "relationships.rel-sella-maer.summary"
        ]
      }
    ]
  },
  "existing_human_authored_branches_for_reference_not_copying": [
    "branch-maer-name-cost-and-offer-ledger",
    "branch-maer-help-triage-before-asking",
    "branch-maer-show-packet",
    "branch-maer-press-moral-frame"
  ]
}
