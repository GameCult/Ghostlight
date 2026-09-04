// ghostlight.artifact_id: kalsa_charter_empty_spindle_review_branch_fold_v0
// ghostlight.fixture_id: charter-empty-spindle-review-v0
// ghostlight.scene_id: charter-empty-spindle-review-v0.cistern-house-review
// ghostlight.final_ink_path: examples/ink/kalsa/charter-empty-spindle-review-v0.branch-and-fold.v0.ink

VAR record_integrity = 1
VAR public_notice = 0
VAR technical_comparison = 0
VAR lower_step_relief = 0
VAR sera_exhaustion = 2
VAR service_margin = 2
VAR return_mark_state = 0
VAR outside_leverage = 0
VAR worker_witness = 1
VAR teren_appeal_visible = 0
VAR spindle_state = 1
VAR claim_misattributed = 0

-> start

=== start ===
Low Sere's water arrives warm, grey, and accompanied by allegations.

Before first broth, Bel Orra climbs from the wet lower steps with two marked vessels knocking against her knees. One holds water taken from her household channel. The other is empty for the Cistern House sample. If the two disagree, the settlement has a problem. If they agree, the settlement will still discuss who was entitled to notice.

The Cistern House sits over paired settling basins. Warm mist beads on blackened beams. Ration boards, work-watch tiles, ash cloths, sampling cups, and duplicate service notices crowd the walls. The west basin can be drained to expose an iron grate. Behind the inner wall, a stair descends to a black pressure door.

-> ordinary_people

=== ordinary_people ===
Maro Seln, cistern reeve, has one hand on the drain wheel and the other on a board full of obligations. He controls surface admission. He does not control the pressure seal, a distinction he honors most carefully when everyone agrees with him.

Sera Venn kneels at the black door with the eight-tooth spindle across her thighs. She has kept the intake on degraded service since her teacher's removal. Low Sere calls her provisional. The water calls her whenever it changes sound.

Ressa Orr, an itinerant priest of the Eighth Baffle lineage, stands beside the sampling ledge comparing Sera's copied marks with her own. She can judge part of the pressure practice. She cannot appoint herself, however useful that would be to people who enjoy conclusions.

Teren Vey waits beyond the household witnesses. Removed custodians are permitted to answer a review and discouraged from behaving as if this means they still own the door.

Tavi Kes stands beside him with the Ash Names' pale cloths looped over one wrist. Tavi speaks for the fellowship tending the Grey Scald dead and injured. The cloths may keep a claim visible. They may not settle a pressure formula.

-> morning_work

=== morning_work ===
Bel dips the empty vessel into the east-basin sampling run. The Cistern House water is warmer than the water at home and carries less ash.

Her eldest is hauling drinking water instead of working the Grey Beds. The room below is wet. Yesterday's reduction notice reached the upper steps before it reached hers.

This is ordinary life at the intake: the old machine changes, the marks follow, and somebody's child learns public administration by carrying a bucket.

Bel has time to strengthen one part of the morning account before the basin table assembles.

-> preparation_choice

=== preparation_choice ===
// ghostlight.choice_layer: morning_account
+ [Set both marked vessels on the basin lip and make Maro record the difference before the water cools.]
    // ghostlight.action_label: use_object
    // ghostlight.branch_label: prepare_paired_vessels
    ~ record_integrity = record_integrity + 2
    ~ worker_witness = worker_witness + 1
    Bel places the household vessel and the Cistern House vessel side by side. Their water lines match. Their ash does not.

    Maro reaches for the clearer cup first.

    "Both," Bel says.

    He records both. It is a small triumph, which means nobody important will admit it was contested.
    -> preparation_fold
+ [Carry the returned delivery tile from the Grey Beds and place it beside the household sample.]
    // ghostlight.action_label: transfer_object
    // ghostlight.branch_label: prepare_grey_bed_tile
    ~ worker_witness = worker_witness + 2
    ~ lower_step_relief = lower_step_relief + 1
    ~ service_margin = service_margin - 1
    Bel crosses the wet apron for the slotted board. The Grey Bed tile sits in the returned notch: flow closed at the lower stop boards, medicinal moss already cooling.

    A bed worker releases it only after Bel names where it will be placed. Evidence travels more safely when it knows which argument intends to eat it.
    -> preparation_fold
+ [Bring Sera the covered broth cup and make the sleepless watch part of the appointment account.]
    // ghostlight.action_label: give_object
    // ghostlight.branch_label: prepare_custodian_relief
    ~ sera_exhaustion = sera_exhaustion - 1
    ~ service_margin = service_margin + 1
    ~ record_integrity = record_integrity + 1
    Bel carries the cup to the pressure door.

    Sera says, "If this is a bribe, it lacks ambition."

    "It is relief coverage. Maro forgot to write any."

    Maro writes it while Sera drinks. Broth acquires an administrative career.
    -> preparation_fold
+ [Ask Teren and Tavi Kes to place the copied Grey Scald warning beside the two names from the disaster.]
    // ghostlight.action_label: speak
    // ghostlight.branch_label: prepare_warning_history
    ~ teren_appeal_visible = teren_appeal_visible + 2
    ~ public_notice = public_notice + 1
    ~ worker_witness = worker_witness + 1
    Tavi knots Jori Kes's and Pell Am's name cloths to the warning board. Teren places the copied interval beneath them and keeps his fingers on the edge.

    "The warning was late," he says.

    "The dead remain punctual," Tavi answers.

    Bel makes Maro leave both claims visible. Neither cloth is asked to settle the formula.
    -> preparation_fold

=== preparation_fold ===
// ghostlight.fold: ordinary_cistern_account
Maro opens the work-watch board. Sera recites the degraded-service limits. Ressa compares one mark, frowns, and compares it again. Teren waits outside the basin lip with the disciplined stillness of a man who has hidden the object everybody needs.

{record_integrity >= 3: The paired observations sit where later hands must move them deliberately rather than forget them by tidying.}
{worker_witness >= 3: Grey Bed and lower-step accounts now stand beside the technical record instead of beneath it.}
{lower_step_relief >= 1: One returned delivery tile already demands a household and crop answer.}
{sera_exhaustion <= 1: Sera's hands have stopped trembling around the spindle, though the missing relief watch remains written.}
{teren_appeal_visible >= 2: Teren's disputed warning and the two name cloths remain in sight without becoming one claim.}

Then the intake knocks beneath the stone.

-> shutter_darkens

=== shutter_darkens ===
One of the three pale shutter marks above the pressure door goes dark.

-> emergency_words

=== emergency_words ===
The east-basin water jumps hot. Maro closes the sampling run before it can spill across Bel's hands.

"Two marks," he says. "We do not have a hearing's worth of water. Ressa compares. Sera turns. We write the appointment cleanly afterward."

Ressa does not look flattered. "My presence is not a signature you can borrow."

Sera lifts the spindle. "I can preserve reduced service. I cannot make provisional mean whatever happens next."

Teren says, from beyond the witnesses, "You managed it for years."

The basin table has reached the old temptation: quietly replace one incomplete office with a sentence that sounds complete.

-> suspension_choice

=== suspension_choice ===
// ghostlight.choice_layer: empty_spindle_response
+ [Ask Sera to put the spindle in its stone cradle and hang the empty-spindle notice before anyone acts.]
    // ghostlight.action_label: speak
    // ghostlight.branch_label: post_empty_spindle
    ~ public_notice = public_notice + 2
    ~ record_integrity = record_integrity + 1
    ~ service_margin = service_margin - 1
    ~ spindle_state = 0
    Sera lays the eight-tooth spindle in the cradle cut into the basin lip.

    Bel hangs the vacancy strip at the Cistern House door and sends a smaller copy board toward the lower Grey Beds. The office is empty in public now, even with its last holder standing beside it.

    The water does not respect the symbolism. It knocks again.
    -> suspension_fold
+ [Ask Sera for one witnessed degraded-service act, with her exact limit read aloud before she takes the spindle.]
    // ghostlight.action_label: speak
    // ghostlight.branch_label: witness_one_service_act
    ~ service_margin = service_margin + 2
    ~ sera_exhaustion = sera_exhaustion + 2
    ~ record_integrity = record_integrity + 1
    ~ spindle_state = 1
    Sera names the act: isolate the second feed, preserve household minimum, leave the black pressure door shut.

    Bel repeats it. Ressa repeats what it does not prove. Maro repeats the part about household minimum because he likes his authority best when it survives the sentence.

    Sera turns the spindle one tooth. The intake's knocking changes pitch.
    -> suspension_fold
+ [Make Ressa compare the live marks before the spindle leaves the cradle.]
    // ghostlight.action_label: use_object
    // ghostlight.branch_label: require_live_comparison
    ~ technical_comparison = technical_comparison + 2
    ~ outside_leverage = outside_leverage + 1
    ~ service_margin = service_margin - 1
    ~ public_notice = public_notice + 1
    ~ spindle_state = 0
    Ressa kneels at the ledge and aligns Sera's copy, Teren's interval, the two water vessels, and the darkened mark.

    "I can compare this state," she says. "I cannot compare the missing Measure by expressing disappointment at the hole where it ought to be."

    Low Sere must feed and lodge her lineage while she works. Useful independence has travel costs.
    -> suspension_fold
+ [Accept Maro's shorthand: record Ressa as overseeing while Sera keeps operating.]
    // ghostlight.action_label: write
    // ghostlight.branch_label: accept_quiet_overseer
    ~ service_margin = service_margin + 2
    ~ outside_leverage = outside_leverage + 2
    ~ record_integrity = record_integrity - 1
    ~ spindle_state = 1
    Maro scores Ressa's lineage mark beside Sera's name and writes a phrase broad enough to survive several later denials.

    Ressa says, "Erase that."

    "After the water settles."

    "Then your lie will have had time to become precedent."

    Sera turns the spindle anyway. Warm water keeps moving, which is how bad records acquire defenders.
    -> suspension_fold

=== suspension_fold ===
// ghostlight.fold: vacancy_and_emergency_scope
The spindle is {spindle_state == 0: visible in its stone cradle beneath the empty-office strip}{spindle_state == 1: in Sera's hands under a provisional claim that the room has not yet agreed how to describe}.

{public_notice >= 2: The Cistern House and lower Grey Beds can now see that emergency work has not settled the office.}
{public_notice == 0: Nothing at the door tells a late-arriving household that the appointment is in dispute.}
{technical_comparison >= 2: Ressa's first comparison separates the live pressure state from the missing calibration claim.}
{outside_leverage >= 2: Ressa's lineage mark has entered the record more deeply than Ressa consented to enter the office.}
{sera_exhaustion >= 4: Sera's shoulders have drawn tight around the spindle. Low Sere is spending tomorrow's keeper on today's water.}
{service_margin >= 3: Warm flow returns to the household channel for the moment.}
{service_margin <= 1: The lower channel cools while the table makes the vacancy legible.}

Bel puts her household vessel back on the lip.

"My room is still wet," she says. "My eldest is still carrying water. Receive the claim before you appoint the explanation."

-> claim_choice

=== claim_choice ===
// ghostlight.choice_layer: commoner_redress
+ [Score a damp clay strip, hang one half beside the appointment record, and keep the matching return mark.]
    // ghostlight.action_label: use_object
    // ghostlight.branch_label: score_return_mark
    ~ return_mark_state = 1
    ~ record_integrity = record_integrity + 1
    ~ public_notice = public_notice + 1
    Bel presses the strip against the basin lip. Maro scores one line through the middle. One half hangs beside the appointment account; the other fits Bel's palm.

    It is not compensation. It is a small material obstacle to being told later that she never asked.
    -> claim_fold
+ [Match the two halves immediately and require household heat and work credit before craft water is assigned.]
    // ghostlight.action_label: present_object
    // ghostlight.branch_label: call_claim_now
    ~ return_mark_state = 2
    ~ lower_step_relief = lower_step_relief + 2
    ~ worker_witness = worker_witness + 1
    ~ service_margin = service_margin - 1
    Bel places the halves together before Maro can hang the first.

    "Household heat. One lost Grey Bed shift. Then you may promise the potters whatever remains."

    Upper-step witnesses mutter. They have brought larger stores and, by coincidence, larger opinions about patience.
    -> claim_fold
+ [Let Maro attach the wet room and lost shift to Sera's suspension charge.]
    // ghostlight.action_label: consent
    // ghostlight.branch_label: attach_claim_to_custodian
    ~ lower_step_relief = lower_step_relief + 1
    ~ claim_misattributed = claim_misattributed + 2
    ~ public_notice = public_notice + 1
    Maro writes Bel's loss beneath Sera's name. The table can now see the injury quickly because it has given the injury a convenient body.

    Sera reads the line. "The late notice came from the ration board."

    Bel knows that. She also knows wet bedding has never once accepted jurisdiction as a substitute for drying.
    -> claim_fold
+ [Bring a Grey Bed worker and a second household witness to hold the claim open at the next allocation.]
    // ghostlight.action_label: call_witness
    // ghostlight.branch_label: widen_commoner_witness
    ~ return_mark_state = 1
    ~ worker_witness = worker_witness + 2
    ~ lower_step_relief = lower_step_relief + 1
    ~ service_margin = service_margin - 1
    Bel calls across the sampling apron. A Grey Bed worker arrives with cold moss in one palm. A second household witness brings a damp sleeping mat rolled beneath one arm.

    The claim becomes harder to misplace and more expensive to postpone. Three people are now not doing the work their losses already interrupted.
    -> claim_fold

=== claim_fold ===
// ghostlight.fold: claim_separated_from_office
The basin lip now carries a technical comparison {technical_comparison >= 2: with a bounded first finding}{technical_comparison < 2: still waiting for a competent finding}, an appointment account {public_notice >= 2: openly marked as unsettled}{public_notice < 2: legible mainly to the people already inside the argument}, and Bel's claim {return_mark_state >= 1: held by matching clay halves}{return_mark_state == 0: attached to another person's proceeding without its own return path}.

{lower_step_relief >= 2: The table has named immediate household heat, work credit, or both before assigning craft water.}
{lower_step_relief == 0: The lower steps have evidence and no present relief.}
{worker_witness >= 4: Lower-step and Grey Bed testimony now occupies enough of the lip that a large hearth must speak around it.}
{claim_misattributed >= 2: Bel's injury has been made evidence against Sera even though notice and allocation belonged to other hands.}
{teren_appeal_visible >= 2: Teren's warning remains beside the appointment record, preserved for a separate technical and disciplinary answer.}

Maro looks at the two pale shutter marks, the spindle, the return mark, Sera's hands, and Ressa's unsigned comparison.

"Name the order you will witness," he tells Bel. "The water will not wait for all of it."

It is not Bel's private decision. It is the consequence of making her claim impossible to tidy away.

-> final_choice

=== final_choice ===
// ghostlight.choice_layer: review_priority
+ [Read one bounded act aloud and offer witness if Sera and the basin table accept its limit.]
    // ghostlight.action_label: speak
    // ghostlight.branch_label: prioritize_bounded_service
    {record_integrity >= 3 && service_margin >= 2 && sera_exhaustion <= 3:
        Bel reads the act, the limit, and the return condition from the board. Sera accepts all three, and the basin table accepts Bel's witness.
        -> ending_bounded_service_success
    - else:
        Bel asks for one more act from an office whose record, service margin, or keeper cannot safely carry it.
        -> ending_bounded_service_cost
    }
+ [Put the sealed vessels under Maro's hand and demand an open review before another hand takes the spindle.]
    // ghostlight.action_label: present_object
    // ghostlight.branch_label: prioritize_full_review
    {public_notice >= 3 && lower_step_relief >= 1:
        Maro takes the drain wheel. Bel raises the paired vessels where every witness can see them.
        -> ending_full_review_success
    - else:
        Maro closes the feed with too little notice or too little provision for the households that will cool first.
        -> ending_full_review_cost
    }
+ [Call for the basin table to fund Ressa's bounded comparison without calling it an appointment.]
    // ghostlight.action_label: speak
    // ghostlight.branch_label: prioritize_independent_comparison
    {technical_comparison >= 2 && outside_leverage <= 2 && record_integrity >= 2:
        Bel points to Ressa's unsigned finding and then to the empty cradle.
        -> ending_comparison_success
    - else:
        The table reaches for outside competence after already writing it into local authority.
        -> ending_comparison_cost
    }
+ [Match Bel's return mark and decide immediate redress before deciding who deserves the office.]
    // ghostlight.action_label: present_object
    // ghostlight.branch_label: prioritize_return_mark
    {return_mark_state >= 1 && worker_witness >= 3:
        Bel sets her half against the half on the board. The seam closes in public.
        -> ending_return_mark_success
    - else:
        Bel calls for the claim, but the table has kept too little witness or no independent mark by which to return it.
        -> ending_return_mark_cost
    }

=== ending_bounded_service_success ===
// ghostlight.ending_label: bounded_service_success
// ghostlight.training_hook: emergency_act_does_not_own_appointment
Sera takes the spindle for one act: hold the second feed closed, preserve household minimum, leave the pressure door shut.

Ressa records that the act fits Sera's witnessed formation. She does not certify the missing calibration. Maro records the warm flow. Bel records that her room is still wet.

When the intake settles, Sera returns the spindle to its cradle. The empty-office strip stays at the door. {return_mark_state >= 1: Bel's matching clay half remains in her pocket.}{return_mark_state == 0: Bel's loss remains attached to the appointment account and vulnerable to its outcome.}

Low Sere has bought another interval. It has not bought a permanent custodian with it.
-> END

=== ending_bounded_service_cost ===
// ghostlight.ending_label: bounded_service_cost
// ghostlight.training_hook: continuity_spends_the_keeper
Sera turns the spindle.

The household channel warms, then shudders. Her hands do the same. {sera_exhaustion >= 4: She misses Ressa's first call to stop and catches the second.}{record_integrity <= 1: Nobody can agree afterward whether the act matched the one read aloud.}

Maro calls the water saved. Teren calls the error inherited. Bel calls for blankets on the lower steps.

The office remains provisional, but the settlement has spent its acting keeper more deeply into it. Emergency continuity is becoming succession by exhaustion.
-> END

=== ending_full_review_success ===
// ghostlight.ending_label: full_review_success
// ghostlight.training_hook: public_vacancy_with_supported_stop
Maro closes the settlement feed. The warm channel falls quiet through the Cistern House.

The empty-spindle account is read in order: live configuration, worker warning, formation limit, household loss, independent comparison, former custodian's answer. {teren_appeal_visible >= 2: Teren's interval stays beside the names of Jori and Pell without swallowing their claim.}

{lower_step_relief >= 2: Blankets, stored water, and work credit move down the steps before the potters receive a craft share.}{lower_step_relief == 1: The lower households receive enough to endure the stop and not enough to forgive it.}

The review may seat Sera, narrow her work, or leave the spindle empty. Tonight it at least begins without pretending that cold rooms are free evidence.
-> END

=== ending_full_review_cost ===
// ghostlight.ending_label: full_review_cost
// ghostlight.training_hook: procedure_without_material_support
The feed closes cleanly.

The lower steps cool first. The Grey Beds lose another delivery. People leave the basin lip to carry water, taking their testimony with them because bodies remain stubbornly committed to needing things.

The empty-spindle review is procedurally pure and materially captured by the hearths rich enough to stay in the room.

Bel keeps the meeting open. She cannot make absence testify.
-> END

=== ending_comparison_success ===
// ghostlight.ending_label: independent_comparison_success
// ghostlight.training_hook: competence_does_not_appoint_itself
Low Sere promises Ressa lodging, assistants, formula copies, and safe conduct for a bounded comparison.

Her finding names what Sera can do, what Teren's record can answer, and what requires the missing Ashen Measure. It grants the Eighth Baffle lineage no water share, threshold title, or permanent chair.

{spindle_state == 0: The spindle remains in its cradle while the comparison is copied.}{spindle_state == 1: Sera returns the spindle after the last witnessed service act.}

Outside competence has entered the review. It has not quietly replaced the settlement.
-> END

=== ending_comparison_cost ===
// ghostlight.ending_label: independent_comparison_cost
// ghostlight.training_hook: paid_comparison_becomes_patronage
Maro points to Ressa's lineage mark as if it were already a commission.

Ressa refuses the office. Her order still controls the only comparison Low Sere can presently obtain, and each delayed answer costs another night of lodging, another assistant, another copied formula, another favor on the road.

{outside_leverage >= 2: The basin table has preserved water by borrowing an institution it cannot compel.}{record_integrity <= 1: Maro's broad overseer phrase survives beside Ressa's refusal and will be quoted by whichever claimant finds it useful.}

No one is seated. The empty chair has acquired a creditor.
-> END

=== ending_return_mark_success ===
// ghostlight.ending_label: return_mark_success
// ghostlight.training_hook: redress_precedes_final_liability
The clay seam fits.

The basin table assigns household heat, one Grey Bed work credit, and dry sleeping space before it returns to the appointment. The aid comes from shared stores. Liability remains open among notice, allocation, support, and technical work.

{claim_misattributed >= 2: Maro strikes Bel's loss from beneath Sera's name and enters it under the basin table's notice account.}{claim_misattributed == 0: The claim keeps its own line and never becomes proof of Sera's guilt.}

Bel leaves with relief, not a verdict. Sera remains provisional. Teren's appeal remains separate. Ressa remains a comparer. The machine keeps knocking beneath all four facts.
-> END

=== ending_return_mark_cost ===
// ghostlight.ending_label: return_mark_cost
// ghostlight.training_hook: received_claim_without_enough_witness
Bel asks the table to return to her claim.

Maro searches the board. A line exists, but {return_mark_state == 0: it sits under Sera's suspension and has no matching half}{return_mark_state >= 1: too few of the workers who made it legible remain at the lip to stop the large hearths from postponing it}.

The appointment argument continues around the wet room until procedure has made the loss very precise and no less wet.

Bel takes the household vessel home. Tomorrow she can bring it back. That is redress with a road still missing from it.
-> END
