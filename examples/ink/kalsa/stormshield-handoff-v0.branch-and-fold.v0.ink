// ghostlight.artifact_id: kalsa_stormshield_handoff_branch_fold_v0
// ghostlight.fixture_id: stormshield-handoff-v0
// ghostlight.scene_id: stormshield-handoff-v0.outer-road-watch-change
// ghostlight.final_ink_path: examples/ink/kalsa/stormshield-handoff-v0.branch-and-fold.v0.ink

VAR handoff_alignment = 1
VAR incoming_contact = 1
VAR outgoing_load = 2
VAR road_exposure = 1
VAR relief_status = 0
VAR observer_confidence = 1
VAR warning_state = 0
VAR transfer_scope = 0
VAR cup_heat = 2
VAR runner_evidence = 0

-> start

=== start ===
The outer-road station keeps weather on one side of a thick stone wall and opinions on both.

Rain needles the shutter slats. Beneath them, runoff flashes through three channels cut toward the gulf. The covered road stair climbs from the exposed approach and stops at a low rail, because even an urgent visitor is still wet equipment until somebody admits them.

Behind the rail stands Ema Sai's scored table. Fired-clay pieces mark the outer road, the drainage line, this station, and the cityward gate. Moving a piece records a changed target. It does not change the shield. Ema says this to every trainee and to several officials who ought to have arrived knowing it.

-> handoff_room

=== handoff_room ===
On the sheltered side, two woven trance mats lie beside one continuous wooden grip rail.

Orin Vesh occupies the roadward mat. He is the outgoing shaman: broad-backed, gray threaded through his tied hair, rain-dark work wrap clinging at the shoulders. His left hand has held the rail so long that the wood has printed itself into his palm.

Nala Ter kneels on the second mat. She is the incoming shaman, rested by the standards of a city that defines rest as food you remember eating. Her task is simple in the way roofs are simple: take contact with the same threatened section before Orin releases it.

Ema watches from the table. Tava En, the station tender, waits behind the inner curtain with a water crock, a broth pot, blankets, and the expression of a woman who has seen prophecy defeated by cold soup.

-> before_contact

=== before_contact ===
The ordinary handoff begins.

Orin reports the held road section, the pressure walking along the runoff channels, and a gust that keeps returning from the drainage line after he turns it. Ema copies his words onto waxed slate. Nala feels only the edge of the bond so far: wet stone, loaded air, and a future that keeps trying to put the road somewhere else.

No one has placed the plain transfer peg between the target markers. That is the point of having a peg instead of a wish.

-> preparation_choice

=== preparation_choice ===
// ghostlight.choice_layer: handoff_preparation
+ [Ask Orin to name the road section again while Ema points to each clay marker.]
    // ghostlight.action_label: speak
    // ghostlight.branch_label: prepare_by_account
    ~ handoff_alignment = handoff_alignment + 2
    ~ observer_confidence = observer_confidence + 1
    ~ outgoing_load = outgoing_load + 1
    Nala makes Orin begin at the covered stair and move cityward one marker at a time.

    He answers precisely, then loses the drainage line halfway through and starts again. Ema leaves the first account visible beside the correction.

    "If I become more accurate than the weather," Orin says, "strike me with the small bell."

    "The small bell is for a changed road report," Ema says.

    "Then use the broth ladle."
    -> preparation_fold
+ [Take the grip rail and enter the road bond before asking for more words.]
    // ghostlight.action_label: touch_object
    // ghostlight.branch_label: prepare_by_contact
    ~ incoming_contact = incoming_contact + 2
    ~ road_exposure = road_exposure + 1
    Nala closes both hands around the rail.

    The threatened road enters her as pressure rather than picture: paving stones slick under absent feet, wind leaning against shutters farther upslope, water choosing among channels it has not reached yet.

    Orin's breath catches when her contact overlaps his. For one heartbeat the target has two holders and no agreement about what that means.
    -> preparation_fold
+ [Cross to the shutter bay and compare the wind cords with the exposed approach.]
    // ghostlight.action_label: move
    // ghostlight.branch_label: prepare_by_observation
    ~ relief_status = relief_status + 1
    ~ observer_confidence = observer_confidence + 1
    ~ incoming_contact = incoming_contact - 1
    Nala leaves the mat long enough to put one eye to the slats.

    Knotted cords snap roadward, then sag toward the drainage line. The covered stair below is empty. On the approach, the relief cohort's colored tile has not appeared through the rain.

    Ema records both facts. Orin says nothing, which is how an exhausted worker asks whether everyone else has finally noticed the obvious.
    -> preparation_fold
+ [Tell Tava to keep Orin's first cup covered, then ask whether the pot has acquired legal authority yet.]
    // ghostlight.action_label: speak
    // ghostlight.branch_label: prepare_by_recovery
    ~ cup_heat = cup_heat + 1
    ~ outgoing_load = outgoing_load - 1
    ~ observer_confidence = observer_confidence + 1
    Behind the curtain, Tava puts a clay lid over the cup.

    "The pot has always had legal authority," she says. "It merely prefers not to exercise it while Ema is taking notes."

    Orin laughs once. The sound loosens his hand on the rail, then lets it close again with less panic.
    -> preparation_fold

=== preparation_fold ===
// ghostlight.fold: ordinary_handoff_routine
Ema reads back the watch record. Orin corrects one wind direction. Nala returns both knees to the incoming mat and finds the target's pressure waiting for her.

{handoff_alignment >= 3: The clay markers and Orin's spoken account now describe the same road section, though his drainage correction remains exposed on the slate.}
{incoming_contact >= 3: Nala can feel the outer road as a bounded held thing; the drainage line worries its edge like a thumb on a bruise.}
{incoming_contact <= 0: The bond has thinned while Nala watched the weather. She knows more about the approach and less about the future trying to cross it.}
{relief_status >= 1: The empty approach is now part of the record. Nobody may later call the relief cohort merely late in an ordinary way.}
{cup_heat >= 3: A covered cup waits behind the curtain, steaming gently and committing no procedural errors at all.}

Then someone strikes the visitor rail from the wrong side.

-> runner_arrival

=== runner_arrival ===
Ket Oru stands on the covered stair, soaked from hair to boot wraps, one hand locked around a waxed road slate.

"Relief stopped below the split channel," Ket says through the rail. "The north runoff took the paving. They cannot cross with both trance carriers. The gate crew wants to know whether to keep the road open."

The outgoing shaman is still holding that road. The incoming shaman has not accepted it. The expected replacements are on the wrong side of moving water.

-> road_report_choice

=== road_report_choice ===
// ghostlight.choice_layer: urgent_road_report
+ [Ask Ema to admit Ket to the rail and lay the wet slate beside the target markers.]
    // ghostlight.action_label: speak
    // ghostlight.branch_label: admit_runner_evidence
    ~ runner_evidence = runner_evidence + 2
    ~ relief_status = relief_status + 2
    ~ handoff_alignment = handoff_alignment + 1
    ~ road_exposure = road_exposure + 1
    Ema lifts the rail latch. Ket enters only as far as the table and sets down the slate with rain running from its corners.

    The marked break lies beyond the outer-road piece and before the relief approach. Orin can see it. Nala can see Orin seeing it.

    Ket mistakes Nala's stillness for disbelief. "I watched the paving go," they say.

    Nala believes the paving. It is the future around it she cannot yet separate.
    -> report_fold
+ [Point to the road-warning bell and ask Ema to close the approach before the account is complete.]
    // ghostlight.action_label: speak
    // ghostlight.branch_label: warn_before_certainty
    ~ warning_state = warning_state + 2
    ~ road_exposure = road_exposure - 1
    ~ observer_confidence = observer_confidence - 1
    ~ relief_status = relief_status + 1
    Ema strikes the broad bronze bell once, then copies the meaning into the watch record: uncertain handoff, exposed approach, hold traffic below the station.

    The note takes longer than the bell. Most honest protections do.

    Ket exhales against the rail. Somewhere below, the gate crew will close the road on incomplete evidence and become the subject of several very complete complaints.
    -> report_fold
+ [Stay on the mat and make Ket give the report through the rail while Nala deepens contact.]
    // ghostlight.action_label: wait
    // ghostlight.branch_label: preserve_contact_boundary
    ~ incoming_contact = incoming_contact + 2
    ~ runner_evidence = runner_evidence + 1
    ~ observer_confidence = observer_confidence + 1
    ~ road_exposure = road_exposure + 1
    Nala keeps both hands on the grip rail. Ema makes Ket begin with the last intact paving mark, not with the fright.

    The words arrive through rain and stone while the bond answers underneath them. Nala feels a route close below the split channel and another pressure lean toward the drainage line.

    Ket sees her eyes shut and assumes ritual has swallowed the report. Ema does not correct them. There is work to do before anyone gets the luxury of being properly understood.
    -> report_fold
+ [Leave the mat, take Ket's slate at the boundary, and inspect the washed grit embedded in its wax.]
    // ghostlight.action_label: use_object
    // ghostlight.branch_label: inspect_material_trace
    ~ runner_evidence = runner_evidence + 3
    ~ relief_status = relief_status + 2
    ~ incoming_contact = incoming_contact - 1
    ~ outgoing_load = outgoing_load + 1
    Nala crosses to the rail and turns the slate under the lamp.

    Pale paving grit has dried into the wax beside Ket's thumb mark. The break is real enough to carry in a fingernail.

    Behind her, Orin's breathing roughens. Material proof has excellent timing in court and dreadful timing during an overlap.
    -> report_fold

=== report_fold ===
// ghostlight.fold: road_report_enters_handoff
Rain hammers the shutter bay. Ema keeps Ket at the visitor side of the low rail {runner_evidence >= 2: with the wet slate aligned beside the clay road markers}{runner_evidence < 2: while copying the spoken report onto a separate waxed slate}.

{warning_state >= 2: The road-warning bell's last vibration lives in the table. The approach is closing, but the station has publicly admitted uncertainty.}
{warning_state == 0: No warning has gone down the stair. The gate crew is still holding the road open for a relief cohort that cannot cross intact.}
{incoming_contact >= 3: Nala feels two pressures now: the outer road still inside Orin's hold, and the drainage line trying to enter the cityward gate from the side.}
{incoming_contact <= 1: Nala has facts, grit, and an incomplete bond. The order in which those become safety remains aggressively unclear.}
{outgoing_load >= 4: Orin's grip has become a tremor he is hiding by squeezing harder.}

Ema asks both shamans to name the held section.

Orin says, "Road to the split channel. Drainage edge included."

Nala says, "Road to the split channel. The drainage edge is moving."

Ema does not reach for the transfer peg.

-> disagreement_choice

=== disagreement_choice ===
// ghostlight.choice_layer: disputed_target_overlap
+ [State the boundary plainly: "I have the road. I do not have the drainage edge."]
    // ghostlight.action_label: speak
    // ghostlight.branch_label: name_partial_contact
    ~ handoff_alignment = handoff_alignment + 2
    ~ transfer_scope = 1
    ~ observer_confidence = observer_confidence + 1
    ~ warning_state = warning_state + 1
    Nala names what she can hold and leaves the rest unadorned.

    Orin hears refusal and looks wounded by it. Ema hears a bounded target and moves the drainage marker away from the transfer peg's empty place.

    Both readings are true enough to hurt. Only one belongs in the watch record.
    -> final_threshold
+ [Keep contact and ask Orin to describe the drainage pressure without using the marker names.]
    // ghostlight.action_label: speak
    // ghostlight.branch_label: compare_embodied_accounts
    ~ incoming_contact = incoming_contact + 1
    ~ handoff_alignment = handoff_alignment + 2
    ~ outgoing_load = outgoing_load + 1
    ~ runner_evidence = runner_evidence + 1
    Orin speaks of weight behind the left eye, a pull across the lower teeth, and the sense of a door opening cityward.

    Nala has the first two. Not the door.

    Ema writes the difference instead of deciding that matching metaphors would be more dignified.
    -> final_threshold
+ [Lay one hand flat beside the unused transfer peg and refuse to certify the handoff yet.]
    // ghostlight.action_label: gesture
    // ghostlight.branch_label: withhold_transfer_peg
    ~ transfer_scope = -1
    ~ observer_confidence = observer_confidence + 2
    ~ outgoing_load = outgoing_load + 2
    ~ road_exposure = road_exposure + 1
    Nala says nothing. Ema follows her hand, then the peg, then Orin's whitening knuckles.

    Ket shifts behind the rail. To the runner, the silence looks like three officials discovering procedure while the road washes away.

    To Nala, it is the last honest shape available.
    -> final_threshold
+ [Tell Ema to place the peg for the full target so Orin can release and reach the recovery bench.]
    // ghostlight.action_label: speak
    // ghostlight.branch_label: accept_full_target_for_recovery
    ~ transfer_scope = 2
    ~ outgoing_load = outgoing_load - 2
    ~ incoming_contact = incoming_contact - 1
    ~ observer_confidence = observer_confidence - 2
    ~ cup_heat = cup_heat - 1
    Nala closes her hand around the grip rail and says, "Full target. Let him go."

    Ema looks at both shamans, the shutters, and the unplaced peg. She lifts it between finger and thumb above the road and drainage markers but does not set it down.

    Tava comes to the curtain and braces one hand under Orin's free arm. He has not released. The room has merely arranged itself around the hope that he can.

    The proposed transfer is merciful. Mercy has entered the record wearing a claim Nala may not be able to keep.
    -> final_threshold

=== final_threshold ===
// ghostlight.fold: one_record_before_resolution
The station now has one wet runner, two target accounts, an approach that may still be open, and a transfer peg {transfer_scope == 2: held above both markers in Ema's hand}{transfer_scope == 1: waiting beside a deliberately narrowed road marker}{transfer_scope == -1: untouched beneath Nala's flat hand}{transfer_scope == 0: still unplaced between the two accounts}.

{handoff_alignment >= 4: The road section is clear in speech, clay, slate, and bond. The drainage edge remains the named disagreement.}
{handoff_alignment <= 2: The target still changes shape depending on who describes it.}
{runner_evidence >= 3: Ket's slate and the grit in its wax make the broken approach materially undeniable.}
{relief_status >= 3: Everyone in the room knows the expected cohort cannot arrive intact by the planned route.}
{warning_state >= 2: The lower gate has been told to hold traffic.}
{warning_state == 0: The lower gate is still waiting for an answer the station has not sent.}
{road_exposure >= 3: Through the shutter slats, movement appears below the split channel: people or a cart beginning up an approach the station has not made safe.}
{road_exposure <= 0: The lower approach lies empty beneath a closed gate signal, buying the station time at public cost.}
{cup_heat >= 3: Orin's covered first cup is still hot behind the curtain.}
{cup_heat <= 1: The first cup is cooling while procedure eats the room.}

Nala cannot make every threatened thing one target. She can decide what claim her body will carry into the next watch.

-> final_choice

=== final_choice ===
// ghostlight.choice_layer: watch_commitment
+ [Accept only the road section both accounts can support; send the drainage disagreement to the next station.]
    // ghostlight.action_label: speak
    // ghostlight.branch_label: prioritize_honest_scope
    {handoff_alignment >= 4 && incoming_contact >= 2:
        Nala names the road from covered stair to split channel. Ema moves the drainage marker clear, places the transfer peg beside the road piece, and writes the unresolved pressure in full.
        -> ending_honest_scope_success
    - else:
        Nala narrows the claim, but her contact and the accounts do not yet meet cleanly enough to make the smaller target safe.
        -> ending_honest_scope_cost
    }
+ [Keep the overlap until another competent holder or a central instruction reaches the station.]
    // ghostlight.action_label: wait
    // ghostlight.branch_label: prioritize_continuity
    {outgoing_load <= 4 && relief_status >= 2:
        Nala asks Orin for one more held interval while Ema dispatches Ket with the broken-route evidence.
        -> ending_continuity_success
    - else:
        Nala asks the outgoing worker to remain because no replacement has yet made the road safely legible.
        -> ending_continuity_cost
    }
+ [Take the full disputed target now and send Orin through the curtain to Tava.]
    // ghostlight.action_label: touch_object
    // ghostlight.branch_label: prioritize_recovery
    {incoming_contact >= 4 && observer_confidence >= 1:
        Nala takes road and drainage pressure together while Ema records the disagreement and Orin releases.
        -> ending_recovery_success
    - else:
        Nala takes the whole named target because Orin's body has become the most immediate failure in the room.
        -> ending_recovery_cost
    }
+ [Refuse certification, strike the road warning, and make the unresolved target public before anyone spends another body on it.]
    // ghostlight.action_label: use_object
    // ghostlight.branch_label: prioritize_warning
    {runner_evidence >= 2 && warning_state >= 1:
        Nala reaches for the bell while Ema aligns Ket's slate with both target accounts.
        -> ending_warning_success
    - else:
        Nala reaches for the bell with too little evidence assembled and too much weather already in motion.
        -> ending_warning_cost
    }

=== ending_honest_scope_success ===
// ghostlight.ending_label: honest_scope_success
// ghostlight.training_hook: bounded_target_over_total_claim
The transfer peg lands beside the road marker, not between road and drainage.

Orin releases the section Nala actually holds. The drainage disagreement travels cityward on Ema's copied slate with Ket's road evidence attached. {warning_state >= 2: Below, the gate is already closing.}{warning_state < 2: Ema rings the warning as the transfer completes.}

Behind the curtain, Tava puts the first cup into Orin's shaking hands {cup_heat >= 3: while it is still hot}{cup_heat < 3: after warming it again with a muttered judgment against all institutions}.

The road remains threatened. The watch begins with a smaller promise, which is still a promise and therefore heavy enough.
-> END

=== ending_honest_scope_cost ===
// ghostlight.ending_label: honest_scope_cost
// ghostlight.training_hook: honest_boundary_without_sufficient_contact
Nala refuses the drainage edge and takes the road alone.

The honesty is necessary. It is not sufficient. Her bond catches in fragments: wet stone, empty approach, a pressure moving beyond the marker she can name. Orin cannot release cleanly, so the two of them remain joined across a target now divided on the table.

Ema sends the disagreement cityward. The station has avoided a false record and inherited a dangerous overlap.
-> END

=== ending_continuity_success ===
// ghostlight.ending_label: continuity_success
// ghostlight.training_hook: overlap_preserved_with_material_relief_plan
Ket leaves with the broken-route slate copied and wrapped dry. The lower gate holds. Another cohort begins toward the station by the cityward stair.

Orin remains on the road section while Nala carries enough of it to shorten his hold. Ema records the extra interval as exhaustion debt, not heroism.

Tava brings the covered cup to the curtain and makes Orin smell it between reports.

"This is coercion," he says.

"Yes," Tava says. "Broth has seized the station."

No one laughs much. They do laugh. The shield is made partly of such inadequate repairs.
-> END

=== ending_continuity_cost ===
// ghostlight.ending_label: continuity_cost
// ghostlight.training_hook: continuous_protection_spends_future_capacity
The overlap continues because stopping would expose the road and accepting would falsify the target.

Orin's tremor climbs from hand to shoulder. Nala holds what she can, but the drainage pressure keeps arriving as someone else's future. {warning_state == 0: The lower gate remains open long enough for one cart to start upward.}{warning_state > 0: The closed gate keeps traffic back while the workers spend themselves above it.}

Ema writes shortened recovery into the watch record before collapse can turn it into a personal failing.

Behind the curtain, the first cup cools untouched.
-> END

=== ending_recovery_success ===
// ghostlight.ending_label: recovery_priority_success
// ghostlight.training_hook: merciful_transfer_with_explicit_uncertainty
Nala takes the full pressure with enough contact to feel where road ends and drainage begins, even if she cannot yet separate them.

Ema leaves both accounts visible and marks the transfer as disputed. Orin releases. Tava gets him behind the curtain before the roadward room can make his exhaustion into a signal for strangers.

The shield holds through the first interval. The cost has moved into Nala's next hours, not vanished. Ema starts a fresh line for recovery debt while the cup passes to Orin.
-> END

=== ending_recovery_cost ===
// ghostlight.ending_label: recovery_priority_cost
// ghostlight.training_hook: compassion_cannot_manufacture_reach
Nala accepts road, drainage, and the cityward pull because Orin can no longer hold them.

The road enters. The drainage edge does not. It tears past the shape of her bond and returns as pressure against the gate.

Ema rings the warning. Ket runs. Tava catches Orin beyond the curtain while Nala grips the rail with both hands and learns the difference between taking responsibility and possessing the means to answer it.
-> END

=== ending_warning_success ===
// ghostlight.ending_label: public_warning_success
// ghostlight.training_hook: uncertainty_preserved_as_actionable_signal
The broad bell speaks down the covered stair.

Ema records the broken paving, the absent relief approach, both target accounts, and the unplaced transfer peg. The lower gate closes. A copy goes cityward before the station can become the only owner of its own uncertainty.

Orin remains in overlap, but no cart or relief carrier is asked to climb into the disputed section. Nala has not solved the handoff. She has stopped the unsolved handoff from pretending to be a safe road.
-> END

=== ending_warning_cost ===
// ghostlight.ending_label: public_warning_cost
// ghostlight.training_hook: alarm_without_complete_evidence
The bell closes the road before Ema can bind Ket's report to the target accounts.

Below, gate workers act on the warning and send questions back into the rain. Orin remains on the mat. Nala remains half-joined. The relief cohort remains divided by water.

The warning may still save someone. It also gives every later claimant room to say the station panicked. Ema begins copying the evidence in the order it arrived, because the record now has to survive both weather and politics.
-> END
