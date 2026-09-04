// ghostlight.artifact_id: cistern_house_nine_breaker_test_branch_fold_v0
// ghostlight.fixture_id: cistern-house-nine-breaker-test
// ghostlight.scene_id: cistern-house-nine-breaker-test.morning-handover
// ghostlight.final_ink_path: examples/ink/delvehold/cistern-house-nine-breaker-test.branch-and-fold.v0.ink

VAR isolation_integrity = 2
VAR district_flow = 1
VAR seal_trust = 2
VAR anomaly_evidence = 0
VAR apprentice_standing = 2
VAR witness_level = 0
VAR network_pressure = 1

-> start

=== start ===
// ghostlight.scene: cistern_house_nine_establishing
Cistern House Nine begins every morning by stopping.

The public pump workshop sits between a terraced Hold and its warm underground sea. A street landing opens into the dry rune gallery. Below, past a grated stair and a brass hoist, black water breathes in the wet intake chamber. Three squat pump engines stand behind safety rails on the service floor, each with a pressure float, an outlet valve, and a removable iron isolation wedge that can break its mana feed in a way even a committee can see.

Orsa Rill is an apprentice, so today she is entitled to food, instruction, tools, appeal, and exactly no civic seal of her own. She has been trusted with Engine Two's wedge and is trying not to look grateful in a manner that might encourage anyone.

-> morning_people

=== morning_people ===
// ghostlight.scene: cistern_house_nine_handover
Master Hessa Cairn waits at the inspection plate with her workshop seal hanging from a chain at her belt. Hessa is short even for a dwarf, square in the shoulders, and capable of making silence sound itemized.

Journeyworker Brin Olt leans over the brass rail to read the three glass pressure floats. His beard is tied into two practical forks because once, years ago, it met a valve spindle and became briefly theoretical.

On the public side of the rail, landing clerk Dema Sorn chalks yesterday's terrace levels onto a black slate. Anyone on the landing may watch the test. This is not generosity. It is how a workshop teaches a district where blame lives.

The routine is simple: seat every wedge, draw a live null rune to close the active pattern, verify that the floats fall, stamp the plate, remove the wedges, and open district flow.

Simple routines are civilization's way of hiding all the mornings on which they were invented.

-> pre_test_choice

=== pre_test_choice ===
// ghostlight.choice_layer: ordinary_handover_role
+ [Seat Engine Two's isolation wedge and make Brin call the feed break aloud.]
    // ghostlight.action: seat_wedge
    // ghostlight.branch: prime_visible_isolation
    // ghostlight.intent: make_the_safety_state_publicly_legible
    ~ isolation_integrity = isolation_integrity + 1
    ~ apprentice_standing = apprentice_standing + 1
    ~ witness_level = witness_level + 1
    Orsa slides the iron wedge into the conduit throat until its red-painted shoulder sits flush with the engine casing.

    "Two isolated," Brin calls.

    Dema repeats it for the landing slate. Hessa checks the seam with one thumbnail and gives Orsa the smallest possible nod, carefully rationed for winter.
    -> routine_fold
+ [Ask Dema to bring the terrace slate beside the rail before the test.]
    // ghostlight.action: request_witness
    // ghostlight.branch: prime_public_record
    // ghostlight.intent: put_district_need_and_workshop_procedure_in_one_frame
    ~ witness_level = witness_level + 2
    ~ seal_trust = seal_trust + 1
    ~ apprentice_standing = apprentice_standing - 1
    Dema lifts the public slate from its wall hook and carries it to the yellow line beside the rail.

    Hessa looks at Orsa. "Were you appointed clerk while I was oiling the seal?"

    "No, Master."

    "Then enjoy the novelty while it lasts."

    The rebuke is mild. The slate remains where Orsa asked for it.
    -> routine_fold
+ [Rehearse the null stroke dry on the slate before taking live mana.]
    // ghostlight.action: rehearse_gesture
    // ghostlight.branch: prime_null_precision
    // ghostlight.intent: protect_the_termination_by_spending_time_on_craft
    ~ apprentice_standing = apprentice_standing + 2
    ~ district_flow = district_flow - 1
    Orsa draws the null rune in chalk from right to left. The closing hook lands true. Brin wipes it away and makes her draw it again.

    "A rune remembers confidence," he says.

    "A rune remembers shape," Hessa says. "Confidence is how apprentices sign accidents."

    The upper terrace gauge ticks lower while they argue. Dema marks the lost minute.
    -> routine_fold
+ [Check the wet chamber through the grated stair before touching the feed.]
    // ghostlight.action: inspect_space
    // ghostlight.branch: prime_intake_awareness
    // ghostlight.intent: ground_the_test_in_the_water_and_machinery_below
    ~ district_flow = district_flow + 1
    ~ network_pressure = network_pressure + 1
    Orsa kneels at the top of the grated stair. Warm mist rises through the iron squares. Below, intake chains vanish into black water; three vertical pump rods stand still; the brass hoist hook hangs clear of them.

    Engine Three's outlet pipe knocks once, though every wedge is seated.

    Brin hears it too. He puts one hand on the rail and says nothing.
    -> routine_fold

=== routine_fold ===
// ghostlight.fold: ordinary_handover_before_anomaly
The three wedges show red shoulders. The three outlet valves show shut. Dema stands where the public can see her slate. Hessa lays two fingers on the inspection plate but does not stamp it.

{isolation_integrity >= 3: Engine Two's wedge sits so cleanly that the paint lines on casing and iron form one unbroken bar.}
{witness_level >= 2: Dema has yesterday's levels, today's callouts, and the unsealed inspection plate in the same chalk frame.}
{apprentice_standing >= 4: Orsa's hand knows the null stroke before fear gets a vote.}
{apprentice_standing <= 1: Hessa watches Orsa instead of the engines.}
{district_flow <= 0: Somewhere above, a terrace pipe coughs itself empty. The landing queue shifts from patience to arithmetic.}
{network_pressure >= 2: Engine Three's outlet pipe knocks again, a pressure pulse arriving from somewhere beyond the closed valves.}

Hessa says, "Live termination. Engine Two."

-> null_test

=== null_test ===
// ghostlight.scene: cistern_house_nine_null_closeup
Orsa touches two fingers to the municipal crystal feed on the gallery wall. Mana enters cold and organized. She draws the null rune from right to left across Engine Two's slate.

The closing hook should make the active pattern go dark.

It does.

Then a thin blue line lights on the feed side of the isolation wedge. It curls around the iron break, writes a loop no one in the room has drawn, and reaches toward Engine Three.

Engine Three's pressure float rises.

Engine Two's stays dead.

Up on the street landing, the district gauge drops another mark.

-> first_anomaly_choice

=== first_anomaly_choice ===
// ghostlight.choice_layer: undocumented_reroute
+ [Keep the wedge seated and repeat the live null exactly.]
    // ghostlight.action: repeat_termination
    // ghostlight.branch: test_repeatable_refusal
    // ghostlight.intent: distinguish_repeatable_machine_behavior_from_a_bad_stroke
    ~ isolation_integrity = isolation_integrity + 1
    ~ anomaly_evidence = anomaly_evidence + 2
    ~ district_flow = district_flow - 1
    ~ network_pressure = network_pressure + 1
    Orsa draws the rune again. Right to left. Close. Release.

    Engine Two goes dark. The blue loop forms again, slower this time, routing around the wedge toward Engine Three.

    Brin stops leaning on the rail.

    "That wasn't your hand," he says.

    It is the kindest thing anyone has said all morning.
    -> pressure_fold
+ [Chalk the undocumented loop onto Dema's public slate before it fades.]
    // ghostlight.action: record_pattern
    // ghostlight.branch: make_anomaly_public
    // ghostlight.intent: preserve_visible_evidence_outside_the_workshop_archive
    ~ anomaly_evidence = anomaly_evidence + 2
    ~ witness_level = witness_level + 2
    ~ seal_trust = seal_trust - 1
    Orsa copies the loop while it is still bright: feed line, iron break, return toward Three.

    Hessa says her name once.

    Dema turns the slate so the people at the landing can see. "Was that in the test?"

    "It is now," Orsa says.

    Hessa's mouth hardens. She has not stamped the plate, but her workshop name is already attached to the room.
    -> pressure_fold
+ [Lift Engine Two's wedge and restore the expected path before the district gauge falls again.]
    // ghostlight.action: remove_wedge
    // ghostlight.branch: restore_service_early
    // ghostlight.intent: protect_water_service_at_the_cost_of_a_clean_test
    ~ district_flow = district_flow + 2
    ~ isolation_integrity = isolation_integrity - 1
    ~ anomaly_evidence = anomaly_evidence - 1
    ~ seal_trust = seal_trust - 1
    Orsa grips the wedge handle and pulls.

    Mana takes the familiar route through Engine Two. Its pressure float rises. The upper terrace gauge steadies.

    The thin blue loop vanishes so completely that Brin swears at it.

    Hessa looks first at the water gauge, then at the unsealed plate, then at Orsa's hand still holding the wedge.
    -> pressure_fold
+ [Throw Engine Three's outlet valve shut and listen from the brass rail.]
    // ghostlight.action: close_valve_and_listen
    // ghostlight.branch: test_distant_pressure
    // ghostlight.intent: use_body_and_tool_feedback_to_locate_the_pressure_source
    ~ network_pressure = network_pressure + 2
    ~ anomaly_evidence = anomaly_evidence + 1
    ~ district_flow = district_flow - 1
    Orsa leans across the rail, catches the long valve handle with both hands, and drags it shut.

    The pipe knocks beneath her palms. Once from the wet chamber. Once from the gallery wall. Then three quick answers farther up the municipal feed.

    She cannot know what made them. She can know they came from beyond the house.

    "Distant load," Brin says.

    Hessa says, "Or coupled feedback. Do not promote a sound before examination."
    -> pressure_fold

=== pressure_fold ===
// ghostlight.fold: anomaly_meets_civic_pressure
The street landing has filled while they worked. Householders hold empty copper cans. A bakery porter has brought a trough on wheels. Nobody crosses Dema's yellow line, which is how people demonstrate patience while making sure patience has witnesses.

Master Hessa unhooks her civic seal.

{anomaly_evidence >= 3: Two matching blue loops now exist in chalk or memory, enough to make "bad apprentice stroke" an expensive explanation.}
{anomaly_evidence <= 0: The loop is gone. Only Brin's oath and Orsa's account remain, both of which a clean engine can embarrass later.}
{witness_level >= 3: Dema's slate faces the landing, and the landing has begun reading back.}
{witness_level <= 1: The rail still separates workshop knowledge from public need.}
{district_flow <= 0: The terrace gauge enters its red lower band. A child in the queue shakes an empty can to confirm what everyone can already see.}
{district_flow >= 3: Engine Two holds the district gauge just above the red band, but it is running ahead of the stamped inspection.}
{network_pressure >= 4: All three outlet pipes answer one another through closed valves, a slow sequence moving toward the wall conduit.}

Hessa says quietly, "If I stamp, I own the opening. If I do not, the terraces own the thirst. Apprentice—tell me what you are actually asking me to risk."

-> seal_choice

=== seal_choice ===
// ghostlight.choice_layer: seal_and_service
+ {isolation_integrity >= 3} [Point to the seated wedge. "Keep Two dead. Run One alone under your seal while Brin watches the wall feed."]
    // ghostlight.action: propose_bounded_service
    // ghostlight.branch: isolate_two_run_one
    // ghostlight.intent: preserve_a_physical_safety_boundary_while_restoring_partial_water
    ~ district_flow = district_flow + 2
    ~ seal_trust = seal_trust + 1
    ~ network_pressure = network_pressure - 1
    Hessa looks from Orsa to the red shoulder of Engine Two's wedge.

    Brin moves to Engine One. Dema clears one line on the slate for a partial-flow seal.

    "That is a real boundary," Hessa says. "I can put my name on a boundary."
    -> final_state
+ {anomaly_evidence >= 2} [Hand Dema the chalked loop and ask Hessa to leave the plate unstamped.]
    // ghostlight.action: transfer_record
    // ghostlight.branch: preserve_public_evidence
    // ghostlight.intent: keep_the_anomaly_inspectable_even_if_water_rationing_worsens
    ~ anomaly_evidence = anomaly_evidence + 1
    ~ witness_level = witness_level + 1
    ~ district_flow = district_flow - 1
    ~ apprentice_standing = apprentice_standing - 1
    Dema takes the slate with both hands. It is not evidence because she believes Orsa. It is evidence because the marks, levels, time, and absent seal now have different owners.

    Hessa leaves the plate bare.

    The queue groans. Orsa feels every empty vessel on the landing acquire her name.
    -> final_state
+ [Set Engine Two's wedge on the inspection plate and refuse to clear it without a sealed exception.]
    // ghostlight.action: withhold_object
    // ghostlight.branch: force_seal_accountability
    // ghostlight.intent: make_the_master_explicitly_own_any_unsafe_override
    ~ seal_trust = seal_trust + 2
    ~ apprentice_standing = apprentice_standing - 1
    ~ district_flow = district_flow - 1
    Orsa lays the iron wedge across the brass plate.

    "Seal an exception or leave it closed," she says.

    Hessa's expression does not change. Brin's does; he has just watched an apprentice turn a safety tool into a civic sentence.

    Dema records the words exactly. Kindness would be editing them.
    -> final_state
+ [Pull the emergency lever for all three pumps and restore the terrace gauges.]
    // ghostlight.action: pull_emergency_lever
    // ghostlight.branch: prioritize_immediate_water
    // ghostlight.intent: accept_uncertain_network_behavior_to_prevent_immediate_service_failure
    ~ district_flow = district_flow + 3
    ~ isolation_integrity = isolation_integrity - 2
    ~ seal_trust = seal_trust - 2
    ~ network_pressure = network_pressure + 2
    Orsa pulls the red lever.

    The isolation cams lift. Three wedges loosen. The pump rods drive downward into warm mist, and water strikes the outlet pipes hard enough to make the brass rails sing.

    The street gauge climbs.

    So does Engine Three's float, one mark higher than the lever position calls for.

    Hessa catches Orsa by the sleeve before she can touch anything else.
    -> final_state

=== final_state ===
// ghostlight.scene: cistern_house_nine_threshold
The morning has become an argument with plumbing attached.

{isolation_integrity >= 3: Engine Two remains visibly isolated: red wedge shoulder flush, null slate dark, no route for a hidden hand to pretend the test was complete.}
{isolation_integrity == 2: The wedges are seated, but the room has handled them enough that every seam now needs another witness.}
{isolation_integrity <= 1: The emergency cams have broken the clean isolation state. Water moves, and so may the undocumented pattern.}

{district_flow >= 3: The terrace gauge climbs out of red. Relief travels through the landing faster than trust.}
{district_flow <= 0: The district remains dry. Copper cans and the bakery trough wait in a line that has stopped being patient.}

{seal_trust >= 4: Hessa holds the seal as shared evidence, not private permission.}
{seal_trust <= 0: The civic seal hangs at Hessa's belt like a tool everyone has just discovered can arrive too late.}

{witness_level >= 4: Dema's public slate carries times, levels, callouts, and the copied loop where no workshop archive can quietly misplace them.}
{anomaly_evidence >= 3: The blue reroute has a repeatable shape. It may be coupled feedback, undocumented technician work, or something harder to name; it is no longer nothing.}
{network_pressure >= 4: The wall conduit answers in a slow pulse that continues after local hands have gone still.}
{apprentice_standing >= 4: Hessa asks Orsa for the sequence in craft terms, giving her work the dignity of examination before judgment.}
{apprentice_standing <= 1: Hessa keeps Orsa in the room but takes the emergency controls out of her reach. Instruction survives; confidence does not.}

Hessa must decide whether this remains a workshop fault or widens into a district matter. Orsa must decide what she will stand behind when the story leaves the pump house.

-> final_choice

=== final_choice ===
// ghostlight.choice_layer: attributed_aftermath
+ [Stay beside the isolated engine and sign the apprentice line under Hessa's seal.]
    // ghostlight.action: sign_record
    // ghostlight.branch: own_bounded_service
    // ghostlight.intent: accept_personal_accountability_for_the_safest_service_state_available
    {isolation_integrity >= 3 && district_flow >= 2:
        -> ending_bounded_service
    - else:
        -> ending_bounded_service_cost
    }
+ [Carry Dema's slate to the stranded district moot before anyone recopies it.]
    // ghostlight.action: carry_evidence
    // ghostlight.branch: widen_jurisdiction
    // ghostlight.intent: move_the_dispute_to_the_people_bearing_the_service_consequence
    {witness_level >= 3 && anomaly_evidence >= 2:
        -> ending_public_hearing
    - else:
        -> ending_public_hearing_cost
    }
+ [Remain at the wall conduit with Brin and trace the distant knocks.]
    // ghostlight.action: investigate
    // ghostlight.branch: follow_network_pressure
    // ghostlight.intent: preserve_uncertainty_and_find_the_boundary_of_the_behavior
    {network_pressure >= 3 && anomaly_evidence >= 1:
        -> ending_network_trace
    - else:
        -> ending_network_trace_cost
    }
+ [Join the landing queue and help ration what water reached the troughs.]
    // ghostlight.action: distribute_resource
    // ghostlight.branch: accept_district_cost
    // ghostlight.intent: keep_the_procedural_choice_attached_to_the_people_who_pay_for_it
    {district_flow >= 2 || seal_trust >= 3:
        -> ending_rationed_trust
    - else:
        -> ending_rationed_distrust
    }

=== ending_bounded_service ===
// ghostlight.ending_label: bounded_service_success
// ghostlight.training_hook: safety_boundary_with_partial_service
Engine One carries the terraces. Engine Two stays dark behind its red wedge. Engine Three is left to sulk in measurements.

Hessa stamps a partial-flow exception, then places Orsa's apprentice mark beneath it. The mark grants no vote. It does make the morning harder to rewrite.

The bakery porter receives half a trough and complains with the grave precision of a person who will still have bread by noon. Brin begins a wall-feed watch. Dema posts the gauge schedule at the landing.

Cistern House Nine opens imperfectly, which is different from opening blind.
-> END

=== ending_bounded_service_cost ===
// ghostlight.ending_label: bounded_service_cost
// ghostlight.training_hook: claimed_boundary_without_material_support
Orsa offers her name to a boundary the room no longer possesses.

One wedge sits crooked. The district gauge stays low. Hessa stamps only the shutdown, not the service plan.

"A principle is not isolation iron," she tells Orsa. "Bring me both next time."

The workshop closes with its fault honestly named and its neighbours honestly thirsty.
-> END

=== ending_public_hearing ===
// ghostlight.ending_label: public_hearing_success
// ghostlight.training_hook: jurisdiction_expands_with_consequence
Dema carries one side of the slate. Orsa carries the other. Neither can claim the record alone.

At the stranded district's moot, householders can see the falling levels, the repeated loop, the unsealed plate, and the exact minute the workshop stopped pretending the fault was private.

Other seals begin arriving from kitchens, laundries, bathhouses, and liftworks whose pipes share the feed. The question widens because the consequence already did.

No one calls the conduit alive. No one is allowed to call it nothing.
-> END

=== ending_public_hearing_cost ===
// ghostlight.ending_label: public_hearing_cost
// ghostlight.training_hook: weak_evidence_meets_public_need
Orsa brings the district a story with too little chalk under it.

The landing remembers the dry taps more clearly than a blue line only three workers saw. Consortium clerks will later call it apprentice panic. Some neighbours will agree because water arrived late and explanation arrived first.

The moot opens anyway, but around service failure, not the undocumented reroute. The stranger question waits outside with the empty cans.
-> END

=== ending_network_trace ===
// ghostlight.ending_label: network_trace_success
// ghostlight.training_hook: anomaly_preserved_as_observed_boundary
Brin ties a copper listening cup to the wall conduit. Orsa marks each knock by time and direction.

The sequence travels beyond Cistern House Nine toward a heating spur and returns after the district load changes. It protects something, or merely behaves as if protection and feedback share a shape.

Hessa sends sealed notices along the route. Dema keeps the public copy. By noon, three workshops are listening to the same wall.

The machine has not spoken. The Hold has learned where to put its ear.
-> END

=== ending_network_trace_cost ===
// ghostlight.ending_label: network_trace_cost
// ghostlight.training_hook: uncertainty_without_repeatable_signal
Orsa and Brin wait beside the conduit until warm stone numbs their palms.

No second sequence comes. The gauges recover or fail for reasons too ordinary to respect suspense.

Brin records the silence. Hessa records the unproven fault. Orsa learns the discipline nobody sings about: an anomaly that does not repeat is still not permission to improve it into a revelation.
-> END

=== ending_rationed_trust ===
// ghostlight.ending_label: rationed_trust
// ghostlight.training_hook: infrastructure_choice_paid_in_public
The troughs receive enough water to make rationing possible rather than decorative.

Orsa carries copper cans to the upper terrace while Hessa and Dema post the seal record together. People grumble at the workshop, the pipes, the queue, and one another. This is not collapse. It is civic life discovering the exact weight of a pump decision.

When Orsa returns, someone has left a cup of water beside Engine Two's dark slate.
-> END

=== ending_rationed_distrust ===
// ghostlight.ending_label: rationed_distrust
// ghostlight.training_hook: service_failure_erodes_workshop_legitimacy
There is too little water to ration and too little record to explain why.

Orsa joins the line anyway. The gesture earns no forgiveness. A workshop seal is valuable because it binds useful work to a name; today the name is attached mostly to empty copper.

Above them, the terrace gauge rests in red. Behind the wall, one pipe knocks once, too late to count as testimony.
-> END
