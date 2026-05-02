VAR packet_shown = false
VAR sella_assessment_pending = false
VAR sella_intake_acceptance = false

-> start

=== start ===
// ghostlight.scene: scene-02-sanctuary-intake
// ghostlight.generated_by: experiments/ink/cold-wake-sanctuary-intake-qwen-sequential-v10-no-think.capture.json
// aetheria.flashpoint: cold-wake-panic

The intake clinic sits behind Ganymede pumpworks, warm only where heat has been budgeted.
Sella Ren stands at the edge of an overfull triage board. Maer Tidecall waits in the clinic's Navigator channel with one corrupted packet and no clean proof.

What does Maer do?

* [Show the corrupted packet to Sella without claiming it as proof, allowing her to assess the noise or testimony.]
    // ghostlight.branch: m1
    // ghostlight.action: show_object
    // ghostlight.intent: Make the packet present without treating it as clean evidence, letting Sella decide its weight.
    -> packet_show

=== packet_show ===
~ packet_shown = true
~ sella_assessment_pending = true

Maer routes the corrupted packet onto Sella's workbench display without giving it the status of proof.

"Here is the packet. It is not proof. Assess it."

Sella does not admit the packet into intake flow. She keeps her hands off the board and studies the workbench display for corruption signatures.

"I won't accept this into intake without knowing what's inside. If it's noise, it's noise. If it's a person, I need to know the cost before I spend a bed."

// ghostlight.generated_branch: m1
// ghostlight.consequence: packet_assessment_boundary_reviewed_consequence
// ghostlight.training_hook: packet_presentation
// ghostlight.training_hook: noise_vs_testimony
// ghostlight.training_hook: evidence_ambiguity
// ghostlight.training_hook: intake_control
// ghostlight.training_hook: capacity_negotiation_under_uncertainty
// ghostlight.training_hook: triage_ethics
// ghostlight.training_hook: care_as_logistics
// ghostlight.training_hook: refusal_without_malice
// ghostlight.training_hook: assessing_corruption_risk
-> END
