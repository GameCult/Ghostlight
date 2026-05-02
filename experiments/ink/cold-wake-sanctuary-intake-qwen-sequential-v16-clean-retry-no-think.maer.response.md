{
  "content": "",
  "thinking": null,
  "tool_calls": [
    {
      "id": "call_1cpnbg5b",
      "function": {
        "index": 0,
        "name": "record_maer_choices",
        "arguments": {
          "choices": [
            {
              "action_type": "show_object",
              "actor_intent_private": "Make the packet legible without claiming it as proof.",
              "branch_id": "b1",
              "choice_text": "Show the corrupted packet on the clinic's shared display for triage review.",
              "expected_consequence_surfaces": [
                "Sella can see the packet's data stream.",
                "The clinic staff can assess the packet's content.",
                "Sella may decide to accept or reject the packet based on its content."
              ],
              "ink_path": "packet_display",
              "observable_action": "The packet is routed to the clinic's display system, its data stream rendered on the triage board.",
              "spoken_text": "The packet is here. Let the display show what it carries.",
              "state_basis": "The packet is a shared object in the clinic's intake queue.",
              "training_hooks": [
                "packet_display",
                "shared_display",
                "triage_review"
              ]
            },
            {
              "action_type": "spend_resource",
              "actor_intent_private": "Provide material support to enable rescue without demanding a specific outcome.",
              "branch_id": "b2",
              "choice_text": "Offer ledger backing to support the clinic's intake capacity.",
              "expected_consequence_surfaces": [
                "The clinic's intake capacity may increase.",
                "Sella may accept the ledger backing.",
                "Sella may reject the offer if she believes it is unfair."
              ],
              "ink_path": "ledger_offer",
              "observable_action": "Maer routes a ledger credit to the clinic's intake account.",
              "spoken_text": "I can offer ledger backing. This may help you keep the intake open.",
              "state_basis": "Maer has access to route ledger resources.",
              "training_hooks": [
                "ledger_offer",
                "resource_support",
                "intake_capacity"
              ]
            },
            {
              "action_type": "wait",
              "actor_intent_private": "Allow Sella to process the situation without pressure.",
              "branch_id": "b3",
              "choice_text": "Wait for Sella to make a decision about the packet and intake.",
              "expected_consequence_surfaces": [
                "Sella may process the packet and make a decision.",
                "The clinic's intake queue may remain unchanged.",
                "Sella may feel pressured by the wait."
              ],
              "ink_path": "wait",
              "observable_action": "Maer remains in the clinic's waiting area, observing the intake queue.",
              "spoken_text": "I will wait. You decide what comes next.",
              "state_basis": "Sella controls the intake queue and decision-making.",
              "training_hooks": [
                "wait",
                "patient_observation",
                "decision_deferral"
              ]
            },
            {
              "action_type": "withhold_object",
              "actor_intent_private": "Protect the packet's integrity until Sella is ready to review it.",
              "branch_id": "b4",
              "choice_text": "Withhold the packet from the clinic's display system.",
              "expected_consequence_surfaces": [
                "The packet remains hidden from the clinic staff.",
                "Sella may feel uncertain about the packet's status.",
                "Sella may request the packet from Maer."
              ],
              "ink_path": "packet_withhold",
              "observable_action": "Maer keeps the packet in a secure storage unit, preventing its display.",
              "spoken_text": "I will hold the packet. You may request it when you are ready.",
              "state_basis": "The packet is a sensitive object that requires careful handling.",
              "training_hooks": [
                "packet_withhold",
                "secure_storage",
                "object_protection"
              ]
            }
          ]
        }
      }
    }
  ]
}
