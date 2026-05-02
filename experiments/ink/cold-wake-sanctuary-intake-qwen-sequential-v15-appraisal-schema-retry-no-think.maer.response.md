{
  "content": "",
  "thinking": null,
  "tool_calls": [
    {
      "id": "call_iyaarqc9",
      "function": {
        "index": 0,
        "name": "record_maer_choices",
        "arguments": {
          "choices": [
            {
              "action_type": "show_object",
              "actor_intent_private": "Make the packet legible without claiming it as proof.",
              "branch_id": "b1",
              "choice_text": "Show the corrupted packet on the shared clinic display for triage review.",
              "expected_consequence_surfaces": [
                "Sella can see the packet's data without Maer interpreting it.",
                "The clinic team can assess the packet's content.",
                "Sella may decide to accept or reject the packet based on its content."
              ],
              "ink_path": "packet_display",
              "observable_action": "The packet is routed through the clinic's display system, projecting its data stream onto the triage board.",
              "spoken_text": "The packet is here. Let the display show what it carries.",
              "state_basis": "The packet is a shared object in the clinic's intake queue.",
              "training_hooks": [
                "packet_display",
                "shared_display",
                "triage_review",
                "data_legibility"
              ]
            },
            {
              "action_type": "spend_resource",
              "actor_intent_private": "Provide material support to offset the cost of rescue.",
              "branch_id": "b2",
              "choice_text": "Offer ledger backing to support the clinic's intake capacity.",
              "expected_consequence_surfaces": [
                "The clinic may accept the ledger backing.",
                "Sella may use the backing to expand intake.",
                "The clinic may decline the offer if it conflicts with intake policy."
              ],
              "ink_path": "ledger_offer",
              "observable_action": "Maer routes a ledger transfer to the clinic's intake account.",
              "spoken_text": "I can offer ledger backing. This may help with intake capacity.",
              "state_basis": "Maer has access to route ledger resources.",
              "training_hooks": [
                "ledger_offer",
                "resource_support",
                "intake_capacity",
                "route_ledger"
              ]
            },
            {
              "action_type": "wait",
              "actor_intent_private": "Allow Sella to process the situation without pressure.",
              "branch_id": "b3",
              "choice_text": "Wait for Sella to make an intake decision.",
              "expected_consequence_surfaces": [
                "Sella may make a decision soon.",
                "The situation may remain unresolved.",
                "The clinic may become overwhelmed if no decision is made."
              ],
              "ink_path": "wait",
              "observable_action": "Maer remains in the intake queue, observing the clinic's status.",
              "spoken_text": "I will wait for your decision.",
              "state_basis": "Sella controls the intake process.",
              "training_hooks": [
                "wait",
                "intake_queue",
                "decision_process",
                "clinic_status"
              ]
            },
            {
              "action_type": "withhold_object",
              "actor_intent_private": "Keep the packet out of the intake queue until Sella is ready.",
              "branch_id": "b4",
              "choice_text": "Withhold the packet from the clinic's intake queue.",
              "expected_consequence_surfaces": [
                "The packet remains out of the intake queue.",
                "Sella may request the packet later.",
                "The clinic may lose the opportunity to assess the packet."
              ],
              "ink_path": "packet_withhold",
              "observable_action": "The packet is routed away from the clinic's intake system.",
              "spoken_text": "I will hold the packet until you are ready.",
              "state_basis": "Maer controls the packet's routing.",
              "training_hooks": [
                "packet_withhold",
                "intake_queue",
                "routing_control",
                "decision_delay"
              ]
            }
          ]
        }
      }
    }
  ]
}
