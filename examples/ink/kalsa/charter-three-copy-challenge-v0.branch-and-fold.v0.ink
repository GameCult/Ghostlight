// ghostlight.artifact_id: charter_three_copy_challenge_v0
// ghostlight.fixture_id: charter-three-copy-challenge-v0
// ghostlight.scene_id: charter-three-copy-challenge-v0.cistern-house-review
// ghostlight.final_ink_path: examples/ink/kalsa/charter-three-copy-challenge-v0.branch-and-fold.v0.ink

VAR record_strength = 1
VAR worker_support = 1
VAR service_margin = 3
VAR table_pressure = 1
VAR comparison_independence = 1
VAR copies_distributed = 0
VAR override_owned = 0
VAR sera_trust = 2

-> start

=== start ===
Every work-watch at Low Sere begins with three bowls of water and at least four opinions about them.

Bel Orra carries the bowls up from the lower Warm Steps to the Cistern House: one from the drinking channel, one from the Grey Beds, and one from the wall beside her eldest child's sleeping mat. That last water has black grit in it. The grit settles quickly, as if hoping not to be called as a witness.

Warm mist hangs from the roof beams. Repair cloths are knotted on the western beam. Name cloths for Jori Kes and Pell Am hang from the eastern one, distinguishable by touch after years of mineral grey have bullied every color into surrender.

-> routine_watch

=== routine_watch ===
At the live east basin, Grey Bed workers Iven and Tal scrape ash from an iron screen into pottery buckets. The basin table borrowed their crop-watch because the intake is short of hands; institutions often discover that work is essential by assigning it somebody else's morning. Sera Venn listens to the intake knock beneath the floor and marks each sound on the service board. She is Low Sere's provisional custodian. The word provisional has lasted longer than some marriages and receives less honest maintenance.

Maro Seln, the cistern reeve, turns the west-basin drain wheel one tooth at a time. Behind him, the drained stone basin slopes toward an iron grate. A stair below the grate ends at the black pressure door.

Ressa Orr lays three blank service sheets on the broad stone lip between the basins. Her Eighth Baffle lineage recognizes part of the intake's pressure practice. She has come to compare Sera's work, not to own it. This distinction is important enough that everyone keeps checking whether it has wandered off.

Teren Vey waits beyond a waist-high cord at the Cistern House doorway. Low Sere removed him after the Grey Scald and never gave his technical appeal a comparison everyone accepted. He may speak. He may not approach the basin lip or touch the spindle hanging at Sera's belt.

Bel puts down the bowls.

Maro glances at the Grey Bed sample. "Beds wait. Hearths first."

"Then tell your soup," Bel says. "It keeps crossing the priority line."

One maintainer laughs into a sleeve. Routine, therefore, has survived another minute.

-> filing_choice

=== filing_choice ===
// ghostlight.choice_layer: opening_the_challenge
Bel is a lower-step water witness. She can force the black grit into the record. How she does it will decide whose hands the challenge reaches before the intake spends another safe interval.

+ [Set the three bowls on their matching water marks and make Ressa compare the grit before anyone drains further.]
    // ghostlight.branch: file_material_sample
    // ghostlight.branch_label: file_material_sample
    // ghostlight.action_label: show_object
    // ghostlight.intent: make the material difference harder to dismiss than Bel's status
    ~ record_strength = record_strength + 2
    ~ service_margin = service_margin - 1
    ~ comparison_independence = comparison_independence + 1
    Bel places each bowl on the wall mark for its source. Drinking. Beds. Lower rooms.

    Ressa stirs none of them. She waits for the water to settle, then turns the bowls so Sera, Maro, and the two screen workers can see the different deposits.

    "That costs us a reading interval," Maro says.

    "Good," Bel says. "Now the interval has bought something."
    -> copies_fold

+ [Call Iven and Tal away from the screen to name the Grey Bed work already lost.]
    // ghostlight.branch: file_worker_chain
    // ghostlight.branch_label: file_worker_chain
    // ghostlight.action_label: speak
    // ghostlight.intent: give unranked labor enough named witnesses to survive the hearing
    ~ worker_support = worker_support + 2
    ~ table_pressure = table_pressure + 1
    ~ service_margin = service_margin - 1
    Bel calls them from the east screen to the stone lip.

    Iven and Tal set down their ash scrapers. Their reed knives remain safely sheathed and their water poles lean beside the doorway. They describe the black grit, the cut bed-flow, and which lower walls sweated first. Neither claims to understand the buried intake.

    They do understand which work disappeared when the water changed.
    -> copies_fold

+ [Ask Maro to enter the petition under the basin table's seal so the drain work can continue.]
    // ghostlight.branch: file_through_reeve
    // ghostlight.branch_label: file_through_reeve
    // ghostlight.action_label: request
    // ghostlight.intent: buy time by borrowing the reeve's authority
    ~ service_margin = service_margin + 1
    ~ table_pressure = table_pressure + 2
    ~ comparison_independence = comparison_independence - 1
    Maro marks the petition before Bel finishes the second sentence.

    "There," he says. "Heard. Now let the people keeping the water moving keep it moving."

    The challenge has entered quickly. It has also entered wearing Maro's hand.
    -> copies_fold

+ [Take chalk and copy the petition onto the ration board where the lower-step carriers can read it.]
    // ghostlight.branch: file_in_public
    // ghostlight.branch_label: file_in_public
    // ghostlight.action_label: use_object
    // ghostlight.intent: make refusal to record the challenge publicly visible
    ~ copies_distributed = copies_distributed + 1
    ~ record_strength = record_strength + 1
    ~ worker_support = worker_support + 1
    ~ table_pressure = table_pressure + 2
    Bel writes around yesterday's ration marks instead of over them. The petition shares the board with water debt, bed-flow, and a note asking whoever borrowed the long ash rake to develop a conscience or at least return the rake.

    People at the doorway begin reading before Maro can decide whether public notice was meant to include an audience.
    -> copies_fold

=== copies_fold ===
// ghostlight.fold: the_three_copies_are_seated
Ressa makes the basin recorder read the petition aloud. Then the three service sheets receive the same system boundary, alleged harm, requested remedy, and list of evidence.

One copy is for the basin table. One is for Sera. One will travel with the independent comparison.

{record_strength >= 3: The black grit sits beside the writing as material evidence, too plain to improve with rank.}
{worker_support >= 3: Iven and Tal stand behind Bel with their water poles grounded on the stone, two workers making it expensive to call one worker confused.}
{comparison_independence <= 0: Ressa watches Maro's fast seal dry. A petition can be admitted so efficiently that it arrives already owned.}
{copies_distributed >= 1: A fifth, unofficial copy remains on the ration board beyond the steam. The town can count past three when frightened.}

Sera takes her sheet. "My degraded-service grant covers the screens above the pressure door. It does not cover opening the Intake Crown."

Maro says, "Nobody asked to open the Crown."

The intake knocks hard enough to tremble rings across all three bowls.

On the service board, one of the three pale shutter marks dims. Two remain.

-> pressure_response

=== pressure_response ===
// ghostlight.choice_layer: preserving_the_hearing
Maro grips the drain wheel. Sera touches the eight-tooth spindle at her belt but does not draw it. Ressa looks from the dim shutter mark to the three copies.

The challenge now costs time everyone can hear.

+ [Carry the independent copy to Iven at the outer doorway before the next act is taken.]
    // ghostlight.branch: distribute_outside_copy
    // ghostlight.branch_label: distribute_outside_copy
    // ghostlight.action_label: transfer_object
    // ghostlight.intent: keep one copy outside the basin table and appointment line
    ~ copies_distributed = copies_distributed + 2
    ~ worker_support = worker_support + 1
    ~ service_margin = service_margin - 1
    Bel folds the sheet under oilcloth and puts it into Iven's ash-raw hands.

    "Warm Steps copy," Iven says.

    "Independent copy," Ressa corrects.

    Iven looks at the reed knife, the water pole, and the wet people in the doorway. "Today those are in the same direction."
    -> comparison_fold

+ [Join Sera at the west-basin gate and isolate the sample flow before anyone argues over the next reading.]
    // ghostlight.branch: preserve_live_state
    // ghostlight.branch_label: preserve_live_state
    // ghostlight.action_label: manipulate_object
    // ghostlight.intent: protect both service margin and inspectable machine state
    ~ service_margin = service_margin + 2
    ~ record_strength = record_strength + 1
    ~ sera_trust = sera_trust + 1
    Sera points to the lower handle. Bel braces it while Sera turns the upper catch. Neither touches the spindle.

    The west sample flow narrows. The black grit gathers in a clean line against the basin stone instead of vanishing toward the cistern.

    "You have done that before," Ressa says.

    "People below machinery learn the reachable parts," Bel answers.
    -> comparison_fold

+ [Make Ressa declare her competence, payment, and future interest in Low Sere before she reads the first record.]
    // ghostlight.branch: expose_comparer_conflict
    // ghostlight.branch_label: expose_comparer_conflict
    // ghostlight.action_label: speak
    // ghostlight.intent: test whether the neutral seat carries a hidden appointment claim
    ~ comparison_independence = comparison_independence + 2
    ~ table_pressure = table_pressure + 1
    ~ service_margin = service_margin - 1
    Ressa names the pressure systems she has worked, the intake features she has only read, the fee Low Sere owes her, and the support compact her lineage would seek if asked to remain.

    "So you might profit if Sera fails," Bel says.

    "Yes."

    The answer does not make Ressa neutral. It makes the angle visible.
    -> comparison_fold

+ [Let Maro turn the drain wheel now, but put the override and its risk under his name on all three sheets.]
    // ghostlight.branch: record_the_override
    // ghostlight.branch_label: record_the_override
    // ghostlight.action_label: authorize
    // ghostlight.intent: preserve immediate service while preventing emergency authority from becoming anonymous
    ~ override_owned = 1
    ~ service_margin = service_margin + 1
    ~ table_pressure = table_pressure + 2
    ~ comparison_independence = comparison_independence - 1
    Maro signs. Ressa repeats the boundary: drain wheel only, no pressure door, no spindle, no claim that the machine approved.

    The wheel moves. Warm water coughs through the east channel. The lower rooms gain time, and Maro acquires a sentence he will have to answer for later.
    -> comparison_fold

=== comparison_fold ===
// ghostlight.fold: bounded_comparison
Ressa takes the comparison place on the stone lip. Sera lays down her service sheet, the public warning, and the record of degraded flow. Bel's bowls remain in reach. Maro keeps the drain wheel. Teren remains beyond the cord.

{copies_distributed >= 3: One copy has already left the immediate reach of every person who wants this hearing to end neatly.}
{record_strength >= 3: Ressa can compare the grit line, shutter loss, warning, and Sera's stated boundary against one another.}
{record_strength <= 2: The hearing has testimony and urgency, but little that a rival hand could reproduce.}
{comparison_independence >= 3: Ressa's fee and future interest sit in the record before her conclusion does.}
{comparison_independence <= 0: Maro's seal and emergency timing press against the comparison like a thumb on wet clay.}
{sera_trust >= 3: Sera gives Bel the service hook without being asked, trusting her to keep the sample flow isolated.}
{service_margin <= 1: The shutter marks knock in their sockets. Whatever the hearing decides will arrive late.}
{table_pressure >= 4: The doorway has filled with wet coats and held breath. Maro can still speak for the table; he can no longer pretend the table is only the people at the lip.}
{override_owned == 1: Maro's signed drain-wheel override lies beside the technical refusals. The machine's next answer will not be allowed to choose which sheet vanishes.}

Ressa reads Sera's grant twice.

"She is competent to preserve reduced service above the pressure door," Ressa says. "I cannot compare her for the Crown without the Ashen Measure and handoff folio below. I can supervise an isolation. I can stop an unsafe opening. I cannot turn missing evidence into her failure."

Teren laughs once from the doorway.

"And when her provisional office finally cracks," he says, "the Eighth Baffle order will happen to be standing nearest the spindle. A miracle of travel."

Ressa does not look away. "My lineage would seek a support compact. It does not hold this appointment."

Sera says, "Yet."

There it is: the ordinary conspiracy, scarcely even concealed. Low Sere needs the stranger's competence. The stranger's order needs work, stores, and a route. Maro needs a living settlement. Teren needs the review that might make his refusal something other than theft. Bel needs the wall beside her child's bed to stop sweating black grit.

None of these needs is a verdict.

-> remedy_choice

=== remedy_choice ===
// ghostlight.choice_layer: remedy_and_succession
Bel's petition cannot appoint a custodian by itself. It can force the basin table to name what happens to the spindle, the copies, the work, and the open challenge.

+ [Demand supervised service: Sera keeps the spindle; Ressa may compare and stop, but her lineage cannot take the appointment through this hearing.]
    // ghostlight.branch: preserve_bounded_appointment
    // ghostlight.branch_label: preserve_bounded_appointment
    // ghostlight.action_label: speak
    // ghostlight.intent: preserve local competence while separating comparison from succession
    ~ comparison_independence = comparison_independence + 1
    {record_strength >= 3 && comparison_independence >= 2:
        -> ending_bounded_review
    - else:
        -> ending_bounded_review_cost
    }

+ [Place the spindle and the three copies under separate seals until another competent comparer can arrive; begin moving the lower rooms now.]
    // ghostlight.branch: suspend_and_evacuate
    // ghostlight.branch_label: suspend_and_evacuate
    // ghostlight.action_label: transfer_object
    // ghostlight.intent: preserve life and evidence at the cost of Low Sere's lower basin economy
    ~ service_margin = service_margin - 1
    {worker_support >= 3 || copies_distributed >= 3:
        -> ending_evacuation_with_standing
    - else:
        -> ending_evacuation_cost
    }

+ [Ask the basin table to seat Ressa provisionally for the isolation, recording that her comparison becomes interested testimony and that Sera retains her copy and appeal.]
    // ghostlight.branch: make_quiet_replacement_legible
    // ghostlight.branch_label: make_quiet_replacement_legible
    // ghostlight.action_label: authorize
    // ghostlight.intent: buy specialist capacity without disguising succession as neutral review
    ~ service_margin = service_margin + 1
    ~ comparison_independence = comparison_independence - 1
    {copies_distributed >= 2 && record_strength >= 2:
        -> ending_recorded_replacement
    - else:
        -> ending_lineage_capture
    }

+ [Permit Maro's immediate opening order only if he takes the override copy to the wheel and leaves Sera and Ressa's refusals intact.]
    // ghostlight.branch: force_owned_override
    // ghostlight.branch_label: force_owned_override
    // ghostlight.action_label: conditionally_comply
    // ghostlight.intent: make political urgency own its act instead of laundering it through technical obedience
    ~ override_owned = 1
    ~ table_pressure = table_pressure + 2
    {service_margin >= 4 && record_strength >= 3:
        -> ending_owned_override
    - else:
        -> ending_override_cost
    }

=== ending_bounded_review ===
// ghostlight.ending_label: bounded_review_holds
// ghostlight.training_hook: commoner_redress_preserves_separate_authorities
The basin table marks Sera's existing grant in one column and Ressa's temporary comparison in another.

Sera keeps the spindle. Ressa receives the right to inspect and stop at the named boundary, not the right to inherit it. Bel and Iven carry the independent copy down the Warm Steps. The black grit remains attached to the lower-room sample instead of being translated into somebody else's expertise.

Reduced service continues. The descent for the Ashen Measure still has to happen. Teren's appeal remains open. Nobody receives the clean pleasure of becoming correct before supper.

Bel's eldest will carry less water tomorrow if the isolation holds. That is not justice. It is enough room for justice to arrive with its own copy.
-> END

=== ending_bounded_review_cost ===
// ghostlight.ending_label: bounded_review_thin_evidence
// ghostlight.training_hook: procedure_cannot_manufacture_missing_proof
The offices stay separate on the sheet. The evidence does not become stronger out of respect for the headings.

Sera keeps the spindle under Ressa's stop. Maro rations the beds. Teren calls the hearing a performance staged over the folio he hid below. He is not entirely wrong, which remains one of his least lovable habits.

Bel's petition survives, but the next decision still depends on a descent, more food, and another lost work-watch. Redress has prevented a quiet seizure. It has not repaired the intake.
-> END

=== ending_evacuation_with_standing ===
// ghostlight.ending_label: evacuation_with_record
// ghostlight.training_hook: collective_redress_survives_material_loss
Sera wraps the spindle. Ressa seals the comparison copy. Bel gives the outside copy to the people already carrying bedding uphill.

The lower Warm Steps empty by named hearth rather than by whoever can run fastest. Grey Bed workers use their water poles to carry the injured and the old. The east basin cools. The crops will follow.

Low Sere loses work, rooms, and safe interval. It does not lose the record of who ordered what. The next custodian will inherit claims instead of a convenient silence.
-> END

=== ending_evacuation_cost ===
// ghostlight.ending_label: evacuation_without_leverage
// ghostlight.training_hook: formal_right_without_material_support
The spindle goes under seal. The lower-step order goes out.

Upper hearths move stores. Lower renters move beds. Grey Bed workers are told to help everyone else abandon the livelihood that made them useful yesterday.

Bel's copy is accurate and lonely. By nightfall, people call the evacuation prudent, and by morning they argue over whether those who lost rooms still count as Low Sere households. A hearing can preserve evidence while the social body required to use it comes apart.
-> END

=== ending_recorded_replacement ===
// ghostlight.ending_label: provisional_replacement_recorded
// ghostlight.training_hook: quiet_replacement_made_reviewable
Ressa steps down from the comparison place before she accepts the spindle. Her unfinished comparison is marked as interested testimony. Sera's competence record, refusal, pay, and appeal remain on all three copies.

The basin table grants Ressa one isolation, one work-watch, and no claim on water, land, salvage, or the later appointment. Sera stands beside the service board with her own sheet. She looks furious enough to remain extremely legible.

The stranger gains leverage. Low Sere gains time. The handoff is a provisional appointment, not a verdict smuggled through a tool belt.
-> END

=== ending_lineage_capture ===
// ghostlight.ending_label: provisional_replacement_capture
// ghostlight.training_hook: emergency_succession_without_independent_record
Ressa takes the spindle because the shutter marks are failing and no one else in the room can compare the isolation.

The table calls it temporary. The only complete copy remains on the basin lip. Sera's grant becomes a story told about why the stranger had to act. By the next ration meeting, Eighth Baffle lodging and fees are treated as part of keeping the water alive.

Nothing illegal needs to happen. That is the useful thing about a quiet replacement: if the copies stay together, power can cross the room without appearing to move.
-> END

=== ending_owned_override ===
// ghostlight.ending_label: override_buys_interval
// ghostlight.training_hook: political_authority_owns_emergency_risk
Maro takes the marked override to the wheel. Sera and Ressa leave their refusals on the other sheets. Bel keeps the lower-room sample where every later comparison must pass it.

The wheel turns. The isolated flow clears enough grit to buy a work-watch. Nobody mistakes that response for the machine's approval of Maro, because the record has already denied him the costume.

He has bought time with public authority. The cost now has his name instead of Sera's office.
-> END

=== ending_override_cost ===
// ghostlight.ending_label: override_spends_the_hearing
// ghostlight.training_hook: urgency_cannot_replace_material_margin
Maro turns the wheel under his own name.

The intake answers with a pressure knock that throws black water across the west basin stone. Sera closes the upper catch. Ressa calls the stop. Bel drags the three bowls clear before the grit can mix.

The opening order fails, but it fails legibly: political command, technical refusal, material result, and commoner evidence remain separate. Low Sere still has to evacuate the lower rooms. It no longer has the luxury of pretending a custodian caused the choice alone.
-> END
