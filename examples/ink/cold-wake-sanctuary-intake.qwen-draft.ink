VAR sella_trust = 57
VAR sella_obligation = 48
VAR maer_obligation = 60
VAR bay_reserved = false
VAR packet_shown = false

-> start

=== start ===
// ghostlight.scene: scene-02-sanctuary-intake
// ghostlight.fixture: examples/agent-state.cold-wake-story-lab.json
// ghostlight.generated_by: experiments/ink/cold-wake-sanctuary-intake-qwen-branch-candidates-v1.capture.json
// aetheria.flashpoint: cold-wake-panic

The intake clinic sits behind the pumpworks, all warmth rationed through humming conduits and tired hands.

Sella Ren stands between Maer Tidecall and one empty monitored bay. The corrupted packet on Maer's slate may be testimony, firmware noise, medical telemetry, or a dirty little braid of all three.

Sella says, "If you came to ask me for mercy, bring tubing."

What does Maer do?

* [Show the corrupted packet to Sella.]
    // ghostlight.branch: branch-01-maer-show-packet
    // ghostlight.action: show_object
    // ghostlight.intent: Provide data for triage capacity calculation while acknowledging the packet is not clean proof of personhood.
    -> branch_01_maer_show_packet

* [Withhold the packet and ask for a triage assessment based on the signal alone.]
    // ghostlight.branch: branch-02-maer-withhold-packet
    // ghostlight.action: withhold_object
    // ghostlight.intent: Force Sella to rely on her own capacity management without the crutch of the packet's ambiguous data.
    -> branch_02_maer_withhold_packet

* [Press Sella on the moral cost of turning away a potential person.]
    // ghostlight.branch: branch-03-maer-press-moral-frame
    // ghostlight.action: speak
    // ghostlight.intent: Challenge Sella's rigidity by framing the refusal as a moral failure, risking her resentment.
    -> branch_03_maer_press_moral_frame

* [Stand in silence and observe Sella's reaction to the packet's presence.]
    // ghostlight.branch: branch-04-maer-silent-observation
    // ghostlight.action: silence
    // ghostlight.intent: Allow Sella to initiate the decision process, testing her suspicion levels without verbal interference.
    -> branch_04_maer_silent_observation

=== branch_01_maer_show_packet ===
~ packet_shown = true

Maer steps closer, holding the glowing packet out so Sella can inspect the routing terms without touching the source.

"Here. The telemetry is incomplete, but the routing logic suggests a distress pattern. It is not proof of a person, but it is not noise either. Decide if the math supports intake."

Sella scans the packet's surface, her eyes narrowing as she parses the firmware noise against the distress pattern. She does not immediately agree to open a bed.

"The packet is unstable. If I accept this, I am betting on a pattern that could be a lie or a glitch. If I say no, the vessel goes dark. What is your ledger backing for this specific intake?"

// ghostlight.generated_branch: branch-01-maer-show-packet
// ghostlight.consequence: branch_01_maer_show_packet_reviewed_consequence
// ghostlight.training_hook: branch-02-sella-demand-ledger
// ghostlight.training_hook: branch-03-sella-conditional-acceptance
-> END

=== branch_02_maer_withhold_packet ===

Maer keeps the packet in his pocket, focusing on the sound of the vessel's distress rather than the data it carries.

"The packet is too corrupted to be useful right now. Can you assess the signal without it? If the corridor is full, we need to know if we can spare a bed before we commit to the source."

Sella sighs, looking at the empty intake bay and the exhausted staff. She refuses to accept a blind assessment that wastes her limited capacity.

"You want me to guess on a ghost? My capacity is measured in minutes, not faith. If you won't show me the packet, you aren't asking for help; you're asking for a miracle. That is not in the ledger."

// ghostlight.generated_branch: branch-02-maer-withhold-packet
// ghostlight.consequence: branch_02_maer_withhold_packet_reviewed_consequence
// ghostlight.training_hook: branch-04-maer-offer-personal-guarantee
-> END

=== branch_03_maer_press_moral_frame ===
~ sella_trust = 52

Maer leans in, his voice soft but firm, addressing the ethical weight of the decision.

"If we wait for perfect categories, the corridor teaches fear to go silent. If you turn this away, you are not protecting capacity; you are teaching crews that rescue waits for certainty that never comes."

Sella's expression hardens, recognizing the trap of moral theater. She defends her triage logic against the accusation of coldness.

"You speak of fear, but you ignore the reality of the math. Every bed I open is a bed closed for someone else. If I take a risk on noise, I am not being merciful; I am being negligent. Do not pretend my ledger is a morality play."

// ghostlight.generated_branch: branch-03-maer-press-moral-frame
// ghostlight.consequence: branch_03_maer_press_moral_frame_reviewed_consequence
// ghostlight.training_hook: branch-05-sella-moral-responsibility
-> END

=== branch_04_maer_silent_observation ===
~ sella_obligation = 51

Maer stands still, watching Sella's eyes dart between the packet and the empty intake bay, letting the silence fill the room.

Sella stares at the packet for a long moment, her internal suspicion warring with her exhaustion. She eventually makes a decision based on her own thresholds.

"The silence is loud enough. I see the routing terms. They are messy, but they are there. I am not a judge, Maer. I am a triage lead. If you want me to open a bed, you must understand that I am doing it because the math allows it, not because you told me it was right. Now, do you want me to try to parse the noise, or do you want to walk away?"

// ghostlight.generated_branch: branch-04-maer-silent-observation
// ghostlight.consequence: branch_04_maer_silent_observation_reviewed_consequence
// ghostlight.training_hook: branch-06-sella-initiative-with-conditions
-> END
