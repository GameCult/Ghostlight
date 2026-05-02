{
  "choices": [
    {
      "branch_id": "branch-ask-ledger-cost",
      "ink_path": "ledger-cost",
      "choice_text": "Present the corrupted packet as a ledger entry and ask Sella to triage it against current bed capacity, explicitly stating that accepting it now creates a specific obligation she must be willing to incur.",
      "action_type": "speak",
      "actor_intent_private": "Force Sella to name the cost of rescue rather than hiding behind false capacity; if she accepts, the debt is real, not a gift.",
      "observable_action": "Maer extends the data packet toward Sella's intake console, voice steady, listing the specific resource drain required.",
      "spoken_text": "I am offering you the packet. It requires intake. You must tell me if your beds can hold it without breaking your own rules. Do not hide the cost behind 'capacity.'",
      "state_basis": "Maer fears the tribunal will teach fear to go silent; she needs Sella to name costs instead of hiding them.",
      "expected_consequence_surfaces": "Sella may refuse due to exhaustion, accept with resentment, or accept with a new condition. The packet remains corrupted.",
      "training_hooks": [
        "ledger-cost",
        "obligation-naming"
      ]
    },
    {
      "branch_id": "branch-withhold-for-now",
      "ink_path": "withhold",
      "choice_text": "Withhold the packet from Sella's direct view, summarizing the telemetry as 'firmware noise' to protect her from the uncertainty until the distress vessel stabilizes.",
      "action_type": "withhold_object",
      "actor_intent_private": "Shield Sella from the ambiguity of the packet to prevent panic, accepting that this delays the decision but preserves her current workflow.",
      "observable_action": "Maer steps back, keeping the packet in a secure slot, and gestures for Sella to continue her current triage of other patients.",
      "spoken_text": "The packet is unstable. I will summarize the noise for you. Focus on the beds you have now.",
      "state_basis": "Maer trusts Sella's care but fears the packet's ambiguity will break her focus; the packet may be testimony or noise.",
      "expected_consequence_surfaces": "Sella may feel patronized or relieved; the rescue timeline shifts as the vessel waits. The packet is not treated as clean proof.",
      "training_hooks": [
        "withhold-uncertainty",
        "firmware-noise"
      ]
    },
    {
      "branch_id": "branch-show-noise",
      "ink_path": "show-noise",
      "choice_text": "Show the packet to Sella but emphasize its corruption, treating it strictly as a logistics problem to be solved by her capacity, not a moral plea.",
      "action_type": "show_object",
      "actor_intent_private": "Demonstrate that care is logistics and obligation, not just compassion; force Sella to decide based on resources, not emotion.",
      "observable_action": "Maer projects the packet's readout onto the wall, highlighting the red error bars and missing telemetry fields.",
      "spoken_text": "Look. The packet is broken. It is a logistics problem. If you take it, you take the risk. Decide by the ledger.",
      "state_basis": "Maer wants to keep the unidentified vessel eligible for rescue while telemetry is incomplete; care must remain logistics.",
      "expected_consequence_surfaces": "Sella may reject the corrupted data, demand a repair, or accept it despite the risk. The packet remains unverified.",
      "training_hooks": [
        "care-is-logistics",
        "ledger-decision"
      ]
    },
    {
      "branch_id": "branch-move-to-other",
      "ink_path": "move-priority",
      "choice_text": "Move the packet aside and direct Sella to prioritize existing patients, effectively deciding that the rescue cannot happen until the sanctuary is stabilized.",
      "action_type": "move",
      "actor_intent_private": "Prioritize the exhausted beds over the unknown packet; if Sella agrees, the rescue waits. If she refuses, the tension rises.",
      "observable_action": "Maer physically shifts the packet to a secondary intake slot and points to the full beds in the background.",
      "spoken_text": "The beds are full. Move the packet to the side. We stabilize what we have first.",
      "state_basis": "Sanctuary beds and repair capacity are nearly exhausted; Maer fears rescue becomes theater if no one prepares intake.",
      "expected_consequence_surfaces": "Sella may argue for the packet's priority, revealing her own stakes, or agree and delay the rescue. The packet is deprioritized.",
      "training_hooks": [
        "capacity-negotiation",
        "stabilize-first"
      ]
    }
  ]
}
