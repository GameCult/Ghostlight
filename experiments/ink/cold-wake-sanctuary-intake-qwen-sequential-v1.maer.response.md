{
  "choices": [
    {
      "branch_id": "branch-01-ledger-ask",
      "ink_path": "ledger_ask",
      "choice_text": "Present the corrupted packet to Sella with a ledger entry showing the vessel's exhaustion, explicitly framing the request as a capacity negotiation rather than a plea for charity, and state that the corridor's law requires a decision before the distress signal decays.",
      "action_type": "speech",
      "actor_intent_private": "To force Sella to acknowledge the cost of refusal while maintaining the illusion of a clean choice, ensuring she cannot hide behind 'full beds' without admitting the ledger is already in debt.",
      "observable_action": "Place the data slate on the intake counter and point to the specific line item for 'distress vessel eligibility' before speaking.",
      "spoken_text": "The packet is noise or testimony, we do not resolve that here. The vessel is running on empty. I am asking you to name the cost of opening a bed, not to pretend capacity exists. If you refuse, I will record the refusal as a debt against the route.",
      "state_basis": "goal-keep-rescue-live: 0.91",
      "expected_consequence_surfaces": [
        "Sella may refuse due to exhaustion, increasing grievance.",
        "Sella may accept but demand payment, shifting the dynamic from care to transaction.",
        "Sella may accept without cost, risking her own stability."
      ],
      "training_hooks": [
        "ledger_ask",
        "capacity_negotiation",
        "debt_recognition"
      ]
    },
    {
      "branch_id": "branch-02-withhold-summarize",
      "ink_path": "withhold_summarize",
      "choice_text": "Withhold the full packet from Sella's direct view, instead summarizing the telemetry's uncertainty to her, and offer to act as a buffer between her clinic and the vessel's distress, proposing that she only needs to decide 'yes' or 'no' to the intake protocol without seeing the raw data.",
      "action_type": "action",
      "actor_intent_private": "To protect Sella from the raw fear or confusion of the packet while still pushing for intake, testing whether she trusts the summary enough to act on it.",
      "observable_action": "Cover the packet with a cloth, gesture to the intake protocol form, and wait for her confirmation without revealing the data's contents.",
      "spoken_text": "I will not show you the packet. It is unstable. I will summarize the distress. You need only decide if the intake protocol runs. If you say no, the vessel goes silent. If you say yes, we bear the cost together.",
      "state_basis": "goal-keep-rescue-live: 0.91",
      "expected_consequence_surfaces": [
        "Sella may feel manipulated by the withholding of information.",
        "Sella may appreciate the buffer and agree to the protocol.",
        "Sella may demand to see the data, breaking the buffer."
      ],
      "training_hooks": [
        "withhold_summarize",
        "protocol_buffer",
        "uncertainty_management"
      ]
    },
    {
      "branch_id": "branch-03-show-without-cost",
      "ink_path": "show_without_cost",
      "choice_text": "Show the packet to Sella openly, admit that the decision to accept it carries a risk of overloading her sanctuary, and explicitly state that you are not asking her to be a hero but to follow the route's obligation, accepting that she may resent the burden.",
      "action_type": "action",
      "actor_intent_private": "To test Sella's boundaries by making the cost visible, hoping she will accept the burden willingly rather than being forced into it.",
      "observable_action": "Remove the cloth from the packet, place it in Sella's hands, and step back to let her examine the data while stating the potential for overload.",
      "spoken_text": "Here is the packet. It may be noise. It may be a person. I am not asking you to be kind; I am asking you to follow the obligation. If you take this, you take the cost. I will not hide it from you.",
      "state_basis": "goal-keep-rescue-live: 0.91",
      "expected_consequence_surfaces": [
        "Sella may refuse due to the visible risk.",
        "Sella may accept but express resentment about the burden.",
        "Sella may accept and hide the cost, risking future collapse."
      ],
      "training_hooks": [
        "show_without_cost",
        "obligation_acknowledgment",
        "risk_transparency"
      ]
    },
    {
      "branch_id": "branch-04-decline-with-hold",
      "ink_path": "decline_withhold",
      "choice_text": "Decline to ask Sella for anything, instead holding the packet and the vessel's fate in suspension, and state that the route will wait for a clearer signal, effectively abandoning the vessel to its own devices until further notice.",
      "action_type": "action",
      "actor_intent_private": "To avoid the risk of overloading Sella's sanctuary, prioritizing her stability over the vessel's rescue, even if it means the vessel suffers.",
      "observable_action": "Fold the packet and place it in a secure compartment, turn away from the intake counter, and state the decision to wait.",
      "spoken_text": "The signal is too weak. The sanctuary is full. I will not ask you to risk your stability. We will wait for a clearer signal. The vessel will wait.",
      "state_basis": "goal-keep-rescue-live: 0.91",
      "expected_consequence_surfaces": [
        "Sella may feel abandoned by the route.",
        "Sella may feel relieved that the burden is removed.",
        "Sella may feel guilty for the vessel's fate."
      ],
      "training_hooks": [
        "decline_withhold",
        "risk_aversion",
        "abandonment_protocol"
      ]
    }
  ]
}
