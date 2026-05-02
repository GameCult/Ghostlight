{
  "content": "",
  "thinking": null,
  "tool_calls": [
    {
      "id": "call_lf2a8gyw",
      "function": {
        "index": 0,
        "name": "record_maer_choices",
        "arguments": {
          "choices": [
            {
              "action_type": "spend_resource",
              "actor_intent_private": "I need to show Sella that rescue is a logistical necessity, not just a moral appeal. The packet's status is uncertain, but the corridor's obligation remains.",
              "branch_id": "route_01",
              "choice_text": "Offer ledger backing to secure intake capacity for the unidentified vessel.",
              "expected_consequence_surfaces": [
                "Sella may accept the backing and open a slot for the unidentified vessel.",
                "Sella may reject the backing as insufficient or inappropriate for the packet's status.",
                "The packet remains unresolved; its content is not determined by the offer.",
                "The clinic's capacity negotiation shifts to a resource-based decision."
              ],
              "ink_path": "ledger_backing",
              "observable_action": "Maer projects a ledger overlay onto the shared clinic display, highlighting available route funds and sanctuary allocation rules.",
              "spoken_text": "If we accept this packet, I can back the intake with route funds. The ledger shows capacity exists if we authorize the transfer.",
              "state_basis": "The clinic's intake board is nearly full, but ledger backing can open a slot. The packet's content is unresolved, but the corridor's duty to rescue remains.",
              "training_hooks": [
                "ledger_backing",
                "resource_offer",
                "capacity_negotiation",
                "route_obligation",
                "uncertain_packet"
              ]
            },
            {
              "action_type": "show_object",
              "actor_intent_private": "I want Sella to see the packet's raw data without interpreting its content. The display will render the telemetry, noise, or testimony without resolving whether it contains a person.",
              "branch_id": "route_02",
              "choice_text": "Show the corrupted packet on the shared clinic display for triage review.",
              "expected_consequence_surfaces": [
                "Sella may review the packet's raw data and make a triage decision based on what is visible.",
                "Sella may reject the packet as noise or irrelevant.",
                "The packet's content remains unresolved; its status is not determined by the display.",
                "The clinic's triage board updates with the packet's rendered data."
              ],
              "ink_path": "packet_display",
              "observable_action": "Maer routes the packet through the clinic's mixed wet/dry work surface display, rendering the corrupted data stream on the shared screen.",
              "spoken_text": "Here is the packet as it arrived. The display shows the telemetry, the noise, and whatever else is in the stream. You decide what to do with it.",
              "state_basis": "The packet is a corrupted stream that may contain medical data, firmware noise, or testimony. The clinic's display can render it without resolving its content.",
              "training_hooks": [
                "packet_display",
                "raw_data_review",
                "triage_decision",
                "corrupted_stream",
                "display_rendering"
              ]
            },
            {
              "action_type": "wait",
              "actor_intent_private": "I need to give Sella space to assess the packet without pressure. The corridor's obligation remains, but forcing a decision may compromise the social frame.",
              "branch_id": "route_03",
              "choice_text": "Wait for Sella to process the packet and make an intake decision.",
              "expected_consequence_surfaces": [
                "Sella may process the packet and make a decision based on her assessment.",
                "Sella may reject the packet as noise or irrelevant.",
                "The packet remains unresolved; its status is not determined by the wait.",
                "The clinic's capacity negotiation remains open for further input."
              ],
              "ink_path": "wait",
              "observable_action": "Maer remains at the shared clinic display, allowing Sella to review the packet and decide on intake.",
              "spoken_text": "Take your time. The corridor will wait. The packet's status is not decided until you act.",
              "state_basis": "The clinic's intake board is nearly full, but Sella controls the decision. Waiting preserves the social frame that makes the goal legitimate.",
              "training_hooks": [
                "wait",
                "social_frame",
                "capacity_negotiation",
                "packet_status",
                "corridor_obligation"
              ]
            },
            {
              "action_type": "gesture",
              "actor_intent_private": "I want to make the corridor's obligation feel older than this room. The packet's status is uncertain, but the duty to rescue remains.",
              "branch_id": "route_04",
              "choice_text": "Press the moral frame by emphasizing the corridor's duty to rescue the unidentified vessel.",
              "expected_consequence_surfaces": [
                "Sella may accept the moral frame and open a slot for the unidentified vessel.",
                "Sella may reject the moral frame as irrelevant or inappropriate.",
                "The packet remains unresolved; its status is not determined by the moral appeal.",
                "The clinic's capacity negotiation shifts to a moral-based decision."
              ],
              "ink_path": "moral_press",
              "observable_action": "Maer projects a route law overlay onto the shared clinic display, highlighting the corridor's duty to rescue and the risks of silence.",
              "spoken_text": "The corridor's duty to rescue is not optional. If we wait for perfect categories, the corridor teaches fear to go silent.",
              "state_basis": "The corridor's duty to rescue is a fundamental obligation. The packet's status is uncertain, but the duty remains.",
              "training_hooks": [
                "moral_press",
                "route_obligation",
                "duty_to_rescue",
                "corridor_law",
                "social_frame"
              ]
            }
          ]
        }
      }
    }
  ]
}
