{
  "content": "",
  "thinking": null,
  "tool_calls": [
    {
      "id": "call_7mwawzpr",
      "function": {
        "index": 0,
        "name": "record_maer_choices",
        "arguments": {
          "choices": [
            {
              "action_type": "show_object",
              "actor_intent_private": "Make the packet's claim visible without resolving its contents.",
              "branch_id": "ledger_backing",
              "choice_text": "Offer the packet's ledger backing to show the packet has weight.",
              "expected_consequence_surfaces": [
                "Sella may see the packet's weight and consider it.",
                "Sella may still refuse intake despite the backing.",
                "The packet may be treated as evidence or noise."
              ],
              "ink_path": "ledger_backing",
              "observable_action": "Project the packet's ledger backing onto the shared clinic display.",
              "spoken_text": "I can show you the packet's ledger backing. It carries weight, even if the packet itself is still unread.",
              "state_basis": "The packet is a tangible object with potential value.",
              "training_hooks": [
                "ledger_backing",
                "packet_weight",
                "visible_claim"
              ]
            },
            {
              "action_type": "wait",
              "actor_intent_private": "Give Sella space to finish her current work.",
              "branch_id": "wait_for_triage",
              "choice_text": "Wait for Sella to complete her current triage before offering the packet.",
              "expected_consequence_surfaces": [
                "Sella may finish triage and be ready for the packet.",
                "Sella may reject the packet before triage finishes.",
                "The packet may be delayed further."
              ],
              "ink_path": "wait_for_triage",
              "observable_action": "Pause and let the clinic's triage board update.",
              "spoken_text": "I will wait for your triage to finish before we proceed.",
              "state_basis": "Sella is currently occupied with intake.",
              "training_hooks": [
                "wait",
                "triage_board",
                "intake_flow"
              ]
            },
            {
              "action_type": "gesture",
              "actor_intent_private": "Make the packet's acoustic notation audible without resolving its contents.",
              "branch_id": "acoustic_summary",
              "choice_text": "Summarize the packet's acoustic notation for Sella to hear.",
              "expected_consequence_surfaces": [
                "Sella may hear the notation and react.",
                "Sella may find the notation disturbing.",
                "The notation may be treated as noise."
              ],
              "ink_path": "acoustic_summary",
              "observable_action": "Activate the clinic's acoustic system to play the packet's notation.",
              "spoken_text": "I can play the packet's acoustic notation for you to hear.",
              "state_basis": "The packet contains acoustic notation.",
              "training_hooks": [
                "acoustic_notation",
                "gesture",
                "packet_sound"
              ]
            },
            {
              "action_type": "speak",
              "actor_intent_private": "Make the moral frame of rescue obligation clear without resolving the packet.",
              "branch_id": "press_moral_frame",
              "choice_text": "Press the moral frame of rescue obligation without resolving the packet.",
              "expected_consequence_surfaces": [
                "Sella may acknowledge the moral frame.",
                "Sella may reject the moral frame.",
                "The moral frame may be treated as theater."
              ],
              "ink_path": "press_moral_frame",
              "observable_action": "Speak formally about rescue obligation and ledger backing.",
              "spoken_text": "Rescue obligation is older than this room. The packet's ledger backing carries that weight.",
              "state_basis": "The moral frame of rescue obligation is active.",
              "training_hooks": [
                "moral_frame",
                "rescue_obligation",
                "ledger_backing"
              ]
            }
          ]
        }
      }
    }
  ]
}
