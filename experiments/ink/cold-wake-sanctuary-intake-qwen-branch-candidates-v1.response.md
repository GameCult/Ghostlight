{
  "branches": [
    {
      "branch_id": "branch-01-maer-show-packet",
      "ink_path": "branch-01-maer-show-packet",
      "choice_text": "Show the corrupted packet to Sella.",
      "action_type": "show_object",
      "actor_intent": "Provide data for triage capacity calculation while acknowledging the packet is not clean proof of personhood.",
      "maer_action_prose": "Maer steps closer, holding the glowing packet out so Sella can inspect the routing terms without touching the source.",
      "maer_spoken_text": "Here. The telemetry is incomplete, but the routing logic suggests a distress pattern. It is not proof of a person, but it is not noise either. Decide if the math supports intake.",
      "sella_response_prose": "Sella scans the packet's surface, her eyes narrowing as she parses the firmware noise against the distress pattern. She does not immediately agree to open a bed.",
      "sella_spoken_text": "The packet is unstable. If I accept this, I am betting on a pattern that could be a lie or a glitch. If I say no, the vessel goes dark. What is your ledger backing for this specific intake?",
      "state_basis": "Maer presents evidence; Sella demands ledger backing before committing scarce capacity.",
      "reviewed_consequences": "Sella may reject the packet due to instability or accept it with a condition regarding liability.",
      "training_hooks": [
        "branch-02-sella-demand-ledger",
        "branch-03-sella-conditional-acceptance"
      ]
    },
    {
      "branch_id": "branch-02-maer-withhold-packet",
      "ink_path": "branch-02-maer-withhold-packet",
      "choice_text": "Withhold the packet and ask for a triage assessment based on the signal alone.",
      "action_type": "withhold_object",
      "actor_intent": "Force Sella to rely on her own capacity management without the crutch of the packet's ambiguous data.",
      "maer_action_prose": "Maer keeps the packet in his pocket, focusing on the sound of the vessel's distress rather than the data it carries.",
      "maer_spoken_text": "The packet is too corrupted to be useful right now. Can you assess the signal without it? If the corridor is full, we need to know if we can spare a bed before we commit to the source.",
      "sella_response_prose": "Sella sighs, looking at the empty intake bay and the exhausted staff. She refuses to accept a blind assessment that wastes her limited capacity.",
      "sella_spoken_text": "You want me to guess on a ghost? My capacity is measured in minutes, not faith. If you won't show me the packet, you aren't asking for help; you're asking for a miracle. That is not in the ledger.",
      "state_basis": "Maer refuses to share evidence; Sella refuses to operate on faith without data.",
      "reviewed_consequences": "Tension rises; Maer must offer an alternative form of backing or risk the vessel being turned away.",
      "training_hooks": [
        "branch-04-maer-offer-personal-guarantee"
      ]
    },
    {
      "branch_id": "branch-03-maer-press-moral-frame",
      "ink_path": "branch-03-maer-press-moral-frame",
      "choice_text": "Press Sella on the moral cost of turning away a potential person.",
      "action_type": "speak",
      "actor_intent": "Challenge Sella's rigidity by framing the refusal as a moral failure, risking her resentment.",
      "maer_action_prose": "Maer leans in, his voice soft but firm, addressing the ethical weight of the decision.",
      "maer_spoken_text": "If we wait for perfect categories, the corridor teaches fear to go silent. If you turn this away, you are not protecting capacity; you are teaching crews that rescue waits for certainty that never comes.",
      "sella_response_prose": "Sella's expression hardens, recognizing the trap of moral theater. She defends her triage logic against the accusation of coldness.",
      "sella_spoken_text": "You speak of fear, but you ignore the reality of the math. Every bed I open is a bed closed for someone else. If I take a risk on noise, I am not being merciful; I am being negligent. Do not pretend my ledger is a morality play.",
      "state_basis": "Maer challenges Sella's ethics; Sella defends her logistical necessity against moral pressure.",
      "reviewed_consequences": "Sella may agree to a risky intake but demand Maer take on the moral liability.",
      "training_hooks": [
        "branch-05-sella-moral-responsibility"
      ]
    },
    {
      "branch_id": "branch-04-maer-silent-observation",
      "ink_path": "branch-04-maer-silent-observation",
      "choice_text": "Stand in silence and observe Sella's reaction to the packet's presence.",
      "action_type": "silence",
      "actor_intent": "Allow Sella to initiate the decision process, testing her suspicion levels without verbal interference.",
      "maer_action_prose": "Maer stands still, watching Sella's eyes dart between the packet and the empty intake bay, letting the silence fill the room.",
      "maer_spoken_text": "",
      "sella_response_prose": "Sella stares at the packet for a long moment, her internal suspicion warring with her exhaustion. She eventually makes a decision based on her own thresholds.",
      "sella_spoken_text": "The silence is loud enough. I see the routing terms. They are messy, but they are there. I am not a judge, Maer. I am a triage lead. If you want me to open a bed, you must understand that I am doing it because the math allows it, not because you told me it was right. Now, do you want me to try to parse the noise, or do you want to walk away?",
      "state_basis": "Maer uses silence; Sella resolves her own suspicion and sets conditions.",
      "reviewed_consequences": "Sella takes initiative but imposes strict conditions on the intake process.",
      "training_hooks": [
        "branch-06-sella-initiative-with-conditions"
      ]
    }
  ]
}
