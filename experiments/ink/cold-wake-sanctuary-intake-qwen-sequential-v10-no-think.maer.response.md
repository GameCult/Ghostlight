{
  "content": "",
  "thinking": null,
  "tool_calls": [
    {
      "id": "call_qzblk4ax",
      "function": {
        "index": 0,
        "name": "record_maer_choices",
        "arguments": {
          "choices": [
            {
              "action_type": "speak",
              "actor_intent_private": "Seek a formal reservation of capacity while making the cost explicit, avoiding moral theater.",
              "branch_id": "m0",
              "choice_text": "Ask Sella to reserve a sanctuary bed for the unidentified vessel, offering Ganymede ledger backing as security.",
              "expected_consequence_surfaces": [
                "Sella may accept the request with ledger backing",
                "Sella may refuse due to capacity constraints",
                "Sella may resent the ask",
                "The packet remains unverified"
              ],
              "ink_path": "ledger_ask",
              "observable_action": "Maer steps forward, presenting a ledger tablet and speaking formally to Sella.",
              "spoken_text": "Sella, I ask you to hold a sanctuary bed open for the vessel whose packet is incomplete. Ganymede will back the cost. Do not hide behind capacity limits.",
              "state_basis": "The packet is ambiguous; sanctuary beds are nearly full; rescue ledgers decide route law trust.",
              "training_hooks": [
                "ledger_backing",
                "capacity_negotiation",
                "explicit_cost",
                "route_law_trust"
              ]
            },
            {
              "action_type": "show_object",
              "actor_intent_private": "Make the packet present without treating it as clean evidence, letting Sella decide its weight.",
              "branch_id": "m1",
              "choice_text": "Show the corrupted packet to Sella without claiming it as proof, allowing her to assess the noise or testimony.",
              "expected_consequence_surfaces": [
                "Sella may identify medical telemetry",
                "Sella may identify firmware noise",
                "Sella may withhold judgment",
                "The packet may be discarded"
              ],
              "ink_path": "packet_show",
              "observable_action": "Maer extends a data slate containing the corrupted packet toward Sella's workbench.",
              "spoken_text": "Here is the packet. It is not proof. Assess it.",
              "state_basis": "The packet may contain medical telemetry, firmware noise, or testimony; Sella controls intake.",
              "training_hooks": [
                "packet_presentation",
                "noise_vs_testimony",
                "evidence_ambiguity",
                "intake_control"
              ]
            },
            {
              "action_type": "wait",
              "actor_intent_private": "Allow Sella to act without pressure, preserving the social frame that makes rescue legitimate.",
              "branch_id": "m2",
              "choice_text": "Wait silently while Sella processes the packet and manages intake, allowing the situation to stabilize.",
              "expected_consequence_surfaces": [
                "Sella may process the packet",
                "Sella may reject the vessel",
                "The situation may stabilize",
                "Tension may rise"
              ],
              "ink_path": "wait_for_clarification",
              "observable_action": "Maer stands still, hands at sides, observing Sella's workflow.",
              "spoken_text": "",
              "state_basis": "Sella is nearly at capacity; the packet is ambiguous; rescue ledgers decide route law trust.",
              "training_hooks": [
                "passive_observation",
                "social_frame_preservation",
                "intake_workflow",
                "tension_management"
              ]
            },
            {
              "action_type": "spend_resource",
              "actor_intent_private": "Provide material help while knowing the ask is unfair, avoiding moral theater.",
              "branch_id": "m3",
              "choice_text": "Offer to cover the cost of the unidentified vessel's intake without asking for a bed reservation, framing it as a gift.",
              "expected_consequence_surfaces": [
                "Sella may accept the gift",
                "Sella may refuse the gift",
                "Sella may resent the gift",
                "The packet remains unverified"
              ],
              "ink_path": "ledger_offer",
              "observable_action": "Maer places a resource token or ledger entry on Sella's desk.",
              "spoken_text": "Take this. It is not a reservation. It is a gift. Use it as you see fit.",
              "state_basis": "Sanctuary beds and repair capacity are nearly exhausted; rescue ledgers decide route law trust.",
              "training_hooks": [
                "resource_offering",
                "gift_framing",
                "unfair_ask",
                "ledger_backing"
              ]
            }
          ]
        }
      }
    }
  ]
}
