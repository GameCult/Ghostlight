// ghostlight.artifact_id: eclipse_nursery_handover_branch_fold_v0
// ghostlight.fixture_id: eclipse-nursery-handover
// ghostlight.scene_id: eclipse-nursery-handover.arrival-terrace-quarantine
// ghostlight.final_ink_path: examples/ink/zyphos/eclipse-nursery-handover.branch-and-fold.v0.ink

VAR road_credit = 2
VAR nursery_trust = 2
VAR graft_readiness = 1
VAR quarantine_pressure = 0
VAR lantern_consent = 1
VAR route_testimony = 1
VAR flower_credibility = 2
VAR eclipse_time = 3
VAR isolation_lane = 0
VAR incoming_fatigue = 1
VAR flower_cupped = 0
VAR external_witness_sent = 0

-> start

=== start ===
The breeding ground keeps its front door outdoors.

Its arrival terrace is a low fan of dark, root-bound stone. The narrow end points routeward, where a candle fungal road reaches the terrace in two rows of amber fruiting beads. The road's visible boundary arcs across the fan's narrow throat. A waist-low work cradle grown from pale flexible ribs straddles that arc, with one rail on each side. The broad end faces nurseryward, down three shallow ramps into warm communal hollows. An isolation shelter curls beyond the terrace's outer rim. Umbros-facing lantern trees hold the inner rim, their cold blue knots waking as the fixed dark world begins to cover the sun.

Seyr folds four long running legs beneath a striped, fibered body and lays the outgoing archive case on the cradle. The smaller pair of chest limbs remains free; three soft digits on each one sort graft wraps, route cords, and the snack nobody has admitted is ceremonial.

-> handover_people

=== handover_people ===
Nara waits beyond the road's bright boundary with a balanced flank frame of medical grafts. She is the incoming specialist, road-dusty and determined to look less tired than her facial fans smell. A burden flower called Mottled Echo grips the bare patch on her left flank. Its flat leaves are calm gray-green.

Ili, the old archive keeper, rests beside the nurseryward ramp. One facial fan has healed crooked. This improves neither Ili's hearing nor the quality of Ili's opinions, but the children believe it does both.

"State the unresolved," Ili says.

"You first," says Nara.

"I have seniority."

"You have the snacks."

Seyr taps the archive case. Handover means more than changing caretakers. The road must recommend passage. The lantern grove must witness it. Sick tissue, damaged tools, failed grafts, and route debts must be named before the nursery is asked to absorb them.

-> routine_choice

=== routine_choice ===
// ghostlight.choice_layer: routine_handover
+ [Feed the road a clean failed graft and name where it failed.]
    // ghostlight.action_label: transfer_object
    // ghostlight.branch_label: prime_road_credit
    ~ road_credit = road_credit + 2
    ~ route_testimony = route_testimony + 1
    ~ graft_readiness = graft_readiness + 1
    Seyr lifts a failed skin-graft strip with two chest digits and places it between the amber candles.

    "Outer-route cold damage," Seyr says. "Rejected before memory admission. Kept clean."

    The visible beads lean inward. Below them, the road's braided body tastes dead tissue, mineral residue, and the shape of an honest failure.

    A new candle opens beside the clean lane.

    "It likes you," Nara says.

    "It has accepted my rubbish. Let us not rush the relationship."
    -> routine_fold
+ [Inspect Nara's graft frame on the low cradle before asking her to cross.]
    // ghostlight.action_label: inspect_object
    // ghostlight.branch_label: prime_graft_readiness
    ~ graft_readiness = graft_readiness + 2
    ~ nursery_trust = nursery_trust + 1
    ~ eclipse_time = eclipse_time - 1
    Nara advances only as far as the cradle's routeward rail. Seyr folds at the nurseryward rail. Their smaller chest limbs meet over the graft trays while all six of Nara's feet remain beyond the fungal boundary.

    The new medical tissues are pale, damp, and individually wrapped in breathable leaf-skin. Seyr checks scent seams, warmth blisters, and the little oath-knots that keep nursery memory out until a patient accepts the graft.

    "You retied the third knot," Seyr says.

    "It was arrogant."

    "It was a knot."

    "Those are not mutually exclusive."
    -> routine_fold
+ [Exchange route testimony with Nara through the portable archive.]
    // ghostlight.action_label: use_object
    // ghostlight.branch_label: prime_route_testimony
    ~ route_testimony = route_testimony + 2
    ~ nursery_trust = nursery_trust + 1
    ~ incoming_fatigue = incoming_fatigue - 1
    Seyr opens the archive case. Flexible memory membranes lift in layered fans, each holding a route's chemical and pressure traces without pretending to own the route itself.

    Nara brings her facial fans close. Together they compare a sour mineral lick, a generous glassback herd, two shelters, and one family that still owes the western road more than it thinks.

    The archive accepts Nara's new testimony with a slow violet edge.

    "You omitted the rain hollow," Ili says.

    "It omitted us first," Nara answers.
    -> routine_fold
+ [Ask the lantern grove to witness the transfer before the eclipse deepens.]
    // ghostlight.action_label: gesture
    // ghostlight.branch_label: prime_lantern_consent
    ~ lantern_consent = lantern_consent + 2
    ~ nursery_trust = nursery_trust + 1
    ~ eclipse_time = eclipse_time - 1
    Seyr turns the bare throat patch toward the inner rim and opens both facial fans. One chest hand rests on the outgoing archive; the other points nurseryward.

    The nearest lantern tree answers from knots below its canopy: blue, blue, amber. Witness. Debt noted. Continue.

    Children in the first hollow copy the sequence with covered lamps until Ili gives them the look reserved for inaccurate law and accurate comedy.
    -> routine_fold

=== routine_fold ===
// ghostlight.fold: routine_before_interruption
The handover continues under the first bite of eclipse shadow.

Seyr inventories nursery grafts. Nara declares damaged tools. Ili counts route cords and quietly relocates the ceremonial snack into official custody.

{road_credit >= 4: A second line of amber candles opens beside the road, a visible credit offered toward the nursery.}
{graft_readiness >= 3: The graft trays lie sorted on the cradle by warmth, immune risk, and the depth of memory they may eventually carry.}
{route_testimony >= 3: The archive membranes hold a clean chain of Nara's recent route contacts.}
{lantern_consent >= 3: Cold blue lantern knots keep the nurseryward ramps legible as daylight thins.}
{eclipse_time <= 2: Umbros has already eaten half the sun. The remaining handover time has become a resource with edges.}

Mottled Echo climbs one deliberate handspan across Nara's flank.

-> flower_alarm

=== flower_alarm ===
The burden flower blooms.

Its gray-green leaves snap outward around a cup of sensory filaments. Color runs through them: yellow for strain, violet for foreign immune memory, then a red pulse broad enough for the road and every caretaker on the terrace to read.

Nara freezes. The bare skin around the rootlets flushes dark with fatigue.

"That is old," she says. "The graft station was sick two routes ago. I am not."

The fungal road does not answer in grammar. Its clean lane goes dark. Bitter beads rise in a ring around Nara's four running feet. On the terrace rim, the lantern knots change from invitation blue to a narrow white warning.

Ili folds the snack away. The situation has become official.

-> alarm_choice

=== alarm_choice ===
// ghostlight.choice_layer: quarantine_alarm
+ [Keep Mottled Echo attached and let its current testimony remain visible.]
    // ghostlight.action_label: wait
    // ghostlight.branch_label: honor_flower_testimony
    ~ flower_credibility = flower_credibility + 2
    ~ nursery_trust = nursery_trust + 1
    ~ quarantine_pressure = quarantine_pressure + 1
    Seyr lowers the chest hands and waits.

    Mottled Echo's rootlets tighten. The red pulse weakens but does not vanish. Yellow strain remains around the leaf edges, while violet memory repeats in a slower band.

    Nara reads Seyr's stillness as accusation. Her facial fans clamp close.

    "You know me," she says.

    Seyr does. The road knows chemistry. The flower knows appetite, history, and perhaps an opportunity to become important. None of those is the whole of Nara.
    -> quarantine_fold
+ [Ask Nara to expose the graft-frame seals and bare throat patch for a clean comparison.]
    // ghostlight.action_label: gesture
    // ghostlight.branch_label: compare_body_and_tools
    ~ graft_readiness = graft_readiness + 1
    ~ route_testimony = route_testimony + 1
    ~ quarantine_pressure = quarantine_pressure + 1
    ~ incoming_fatigue = incoming_fatigue + 1
    Seyr opens both facial fans and points first to Nara's throat, then to the graft frame.

    Nara's fans flare in offense before discipline wins. She folds low on the routeward stone, opens the bare throat patch to the air, and unseals each graft tray with her chest hands.

    The throat smells of exhaustion and familiar immune heat. The third graft tray carries a faint violet trace that matches the flower's warning.

    "A contaminated wrap," Nara says, too quickly.

    "A possibility," Seyr answers. A possibility is smaller than a verdict and heavier than politeness.
    -> quarantine_fold
+ [Move Mottled Echo into a clear mineral cup on the outer edge of the cradle.]
    // ghostlight.action_label: transfer_object
    // ghostlight.branch_label: isolate_flower_witness
    ~ flower_cupped = 1
    ~ flower_credibility = flower_credibility - 1
    ~ nursery_trust = nursery_trust - 1
    ~ quarantine_pressure = quarantine_pressure + 1
    Seyr offers a shallow mineral cup beneath the flower. Nara releases each rootlet with two chest digits, careful not to tear the bare flank patch.

    Mottled Echo settles into the cup and blooms red enough to insult the cup's ancestry.

    The road's bitter ring stops rising. It does not open.

    "You wanted a second witness," Seyr tells the flower.

    The flower displays violet memory and no detectable shame.
    -> quarantine_fold
+ [Wait for the road to sample shed fiber, cradle residue, and Nara's foot-pressure.]
    // ghostlight.action_label: wait
    // ghostlight.branch_label: let_road_sample
    ~ road_credit = road_credit + 1
    ~ flower_credibility = flower_credibility + 1
    ~ eclipse_time = eclipse_time - 1
    ~ incoming_fatigue = incoming_fatigue + 1
    Seyr refuses the reflex to fill silence with authority.

    Nara holds position beyond the boundary. The road draws in a shed body fiber, moisture from one footfall, and a smear left by the graft frame on the cradle's routeward rib.

    Umbros closes over more of the sun. The lantern trees brighten the ramps but leave Nara inside the white warning.

    Waiting is not neutral. It spends Nara's strength and the nursery's light window so that a slower mind can answer in its own scale.
    -> quarantine_fold

=== quarantine_fold ===
// ghostlight.fold: testimony_without_verdict
The terrace now holds four testimonies: Nara's words, Mottled Echo's color, the portable archive, and the fungal road's bitter ring. They agree only that something traveled here.

{flower_credibility >= 4: Mottled Echo keeps a restrained red-violet pattern. Even Nara stops calling it mere drama.}
{flower_cupped == 1: Mottled Echo blazes in the mineral cup, vivid and separated from the body it claims to explain.}
{route_testimony >= 3: Seyr can trace the violet signal back through the archive to Nara's recent graft station.}
{graft_readiness >= 3: The sorted trays make one suspect wrap visibly separable from the clean medical grafts.}
{incoming_fatigue >= 3: Nara's long body sags between the two locomotor pairs. Pride is now competing with muscle tremor.}
{quarantine_pressure >= 2: More bitter beads close the routeward arc. The road has converted caution into geometry.}

A threadwing courier circles above the lantern canopy, ribbon vanes silver in the remaining daylight. It refuses to land inside the warning ring.

The nearest tree pulses white toward the outer isolation shelter, then blue toward Seyr.

Seyr reads a proposal: separate the doubtful material, keep testimony moving, protect the nursery. Nara reads it as a polite way to say unclean.

-> pressure_choice

=== pressure_choice ===
// ghostlight.choice_layer: witness_and_route
+ {route_testimony >= 3} [Trace the violet signal through the archive and name the suspect graft station aloud.]
    // ghostlight.action_label: show_object
    // ghostlight.branch_label: spend_route_testimony
    ~ route_testimony = route_testimony - 1
    ~ nursery_trust = nursery_trust + 1
    ~ quarantine_pressure = quarantine_pressure - 1
    Seyr fans the archive membranes wide on the cradle. One old violet edge aligns with Mottled Echo's current pulse and the residue on the third graft tray.

    "The signal began at Ledge Station," Seyr says. "Nara carried the warning. That does not prove Nara carries the sickness."

    Ili touches one route cord to the matching archive layer. The road opens a single candle—not passage, but attention.
    -> final_threshold
+ {graft_readiness >= 3} [Seal the suspect wrap and move only that tray toward isolation.]
    // ghostlight.action_label: transfer_object
    // ghostlight.branch_label: isolate_suspect_graft
    ~ isolation_lane = isolation_lane + 2
    ~ graft_readiness = graft_readiness - 1
    ~ quarantine_pressure = quarantine_pressure - 1
    Seyr closes a breathable leaf-skin sheath around the third tray. Nara braces the flank frame while Seyr lifts the tray clear with both chest hands.

    They set it on the cradle's outer rail, aligned with the isolation shelter and far from the nurseryward ramps.

    Mottled Echo's violet band fades. The yellow fatigue remains.

    Nara lets out a breath that sounds almost like gratitude and almost like resentment. Seyr is not equipped to choose the more flattering reading.
    -> final_threshold
+ {lantern_consent >= 3} [Ask the lantern grove to light the isolation route and keep the nursery route dark.]
    // ghostlight.action_label: gesture
    // ghostlight.branch_label: request_lantern_separation
    ~ isolation_lane = isolation_lane + 2
    ~ lantern_consent = lantern_consent - 1
    ~ nursery_trust = nursery_trust + 1
    Seyr shows open chest hands to the grove, then traces the outer curve toward the isolation shelter.

    Cold knots answer one after another. A blue-white lane appears along the outer rim. The three nurseryward ramps go dark at once.

    It is protection, not welcome. Nara's fans close anyway.
    -> final_threshold
+ [Offer the threadwing a clean shed fiber and ask it to carry the disputed signal to another road.]
    // ghostlight.action_label: transfer_object
    // ghostlight.branch_label: seek_external_witness
    ~ road_credit = road_credit + 1
    ~ eclipse_time = eclipse_time - 1
    ~ quarantine_pressure = quarantine_pressure + 1
    ~ nursery_trust = nursery_trust - 1
    ~ external_witness_sent = 1
    Seyr knots a clean shed fiber beside a trace from the suspect wrap. The threadwing lands on the cradle's outer rail, ribbon vanes held clear of Mottled Echo.

    It tastes the packet, takes it, and launches routeward.

    The road keeps its ring. The nursery keeps waiting. A second witness may prevent a local mistake, but it cannot arrive before this eclipse finishes.
    -> final_threshold

=== final_threshold ===
// ghostlight.fold: final_handover_threshold
Totality settles over the terrace for its brief, daily hour.

Umbros is no wandering moon. It hangs fixed and enormous above the lantern grove, a black world rimmed by the dim primary. Blue knots reveal the nurseryward ramps. Amber fungal beads define the routeward boundary. If an isolation lane has been earned, it curves between them without touching either.

{road_credit >= 4: The road holds a bright amber candle beside Seyr, credit made visible but not yet spent.}
{road_credit <= 2: The road's boundary remains thin, dark, and professionally unimpressed.}
{nursery_trust >= 4: Ili settles beside Seyr, making the outgoing caretaker's judgment visibly collective.}
{nursery_trust <= 1: Ili moves closer to the nurseryward ramp, guarding the commons from Seyr's improvisation.}
{quarantine_pressure >= 3: The bitter ring reaches the cradle's routeward legs. Delay is becoming closure.}
{quarantine_pressure <= 1: The bitter beads lower, leaving a narrow clean interval in the boundary.}
{eclipse_time <= 1: The threadwings have gone quiet. There will be little time to finish the handover before returning light changes every signal.}
{isolation_lane >= 2: Blue-white lantern light and sparse amber candles define a clean outer route to the isolation shelter.}

Nara waits outside the nursery. The children wait inside it. {flower_cupped == 1: Mottled Echo waits in the mineral cup on the cradle's routeward rail.|Mottled Echo remains on Nara's bare left flank.} In either position it continues being, in its small botanical way, extremely available for consultation.

The handover needs a decision that does not pretend uncertainty has disappeared.

-> final_choice

=== final_choice ===
// ghostlight.choice_layer: caretaker_decision
+ [Admit Nara through the clean lane, leaving the suspect graft outside.]
    // ghostlight.action_label: authorize
    // ghostlight.branch_label: admit_specialist_separate_material
    {road_credit >= 4 && graft_readiness >= 2 && quarantine_pressure <= 2:
        Seyr places the archive case on the nurseryward side and opens both facial fans to Nara.
        -> ending_admission_success
    - else:
        Seyr signals admission before the terrace has enough agreement to carry it.
        -> ending_admission_cost
    }
+ [Move Nara, archive, and clean grafts to the isolation shelter; finish handover there.]
    // ghostlight.action_label: move
    // ghostlight.branch_label: complete_handover_in_isolation
    {isolation_lane >= 2 && lantern_consent >= 1:
        Seyr closes the archive case, lifts one end of the clean graft frame, and takes the outer lit curve with Nara.
        -> ending_isolation_success
    - else:
        Seyr chooses isolation before the living routes have made one usable.
        -> ending_isolation_cost
    }
+ [Defer the transfer and keep the outgoing caretaker watch through returning light.]
    // ghostlight.action_label: refuse
    // ghostlight.branch_label: defer_handover
    {nursery_trust >= 3 && incoming_fatigue <= 2:
        Seyr returns the archive case to the outgoing side of the cradle and asks Nara to rest beyond the boundary.
        -> ending_defer_success
    - else:
        Seyr refuses the transfer when the nursery and Nara have too little strength left for another clean cycle.
        -> ending_defer_cost
    }
+ [Keep the boundary closed until the threadwing or road produces a second witness.]
    // ghostlight.action_label: wait
    // ghostlight.branch_label: wait_for_second_witness
    {external_witness_sent == 1 && route_testimony >= 2 && flower_credibility >= 3 && eclipse_time >= 1:
        Seyr folds beside Nara outside the ring, archive between them, and gives the slower witnesses time.
        -> ending_witness_success
    - else:
        Seyr waits after the light window, the bodies, or the testimony have already thinned too far.
        -> ending_witness_cost
    }

=== ending_admission_success ===
// ghostlight.ending_label: bounded_admission_success
// ghostlight.training_hook: trust_as_separated_material_custody
The road lowers its bitter beads one by one.

Nara crosses the clean lane without the suspect tray. Seyr walks beside her, four running feet kept inside the amber line. Ili receives the archive. The children receive the incoming specialist and immediately ask why her flower shouted at dinner.

"Because dinner has standards," Nara says.

Outside, the sealed tray remains under lantern warning. Nara is admitted. The doubtful material is not. The distinction costs work, which is how Seyr knows it is real.
-> END

=== ending_admission_cost ===
// ghostlight.ending_label: bounded_admission_cost
// ghostlight.training_hook: premature_trust_converts_warning_to_route_refusal
Seyr steps across the ring.

The road darkens beneath all six feet. Lantern knots shut from nurseryward to routeward, turning the terrace into a sequence of lost edges. Ili blocks the first ramp with an old body that has survived too many earnest shortcuts.

Nara does not move. "You are making me the weapon," she says.

Seyr backs out. The nursery remains safe, the handover fails, and the road remembers that a caretaker tried to spend consent before earning it.
-> END

=== ending_isolation_success ===
// ghostlight.ending_label: isolation_handover_success
// ghostlight.training_hook: continuity_without_forced_admission
The outer lane brightens under their feet.

The isolation shelter is a low crescent of grown ribs and translucent leaf-skin, open toward the terrace but physically separate from the nursery ramps. Seyr and Nara settle around its floor cradle, the portable archive between their facial fans, the clean grafts stacked on the inward shelf and the suspect tray on the outward one.

The handover completes in quarantine. No one is expelled. No one is smuggled inside by kindness wearing authority's coat.

At the terrace, Ili eats the ceremonial snack in both their names. Some offices adapt faster than others.
-> END

=== ending_isolation_cost ===
// ghostlight.ending_label: isolation_handover_cost
// ghostlight.training_hook: spatial_safety_requires_route_consent
Seyr takes the outer curve.

There is no continuous lane. One lantern knot marks the shelter, but the fungal candles stop halfway. Nara follows until the bitter ground softens under a front running foot.

She jerks back, graft frame swinging. A clean tray strikes the stone and splits its leaf-skin seam.

Now there are two doubtful trays, one exhausted specialist, and a road with fresh evidence that Sa'ueia impatience can manufacture contamination perfectly well on its own.
-> END

=== ending_defer_success ===
// ghostlight.ending_label: deferred_handover_success
// ghostlight.training_hook: refusal_as_continuity_care
Seyr keeps the archive.

Nara rests routeward of the lowered bitter ring while Ili sends a low cradle and water-mineral cloths to the boundary. The outgoing watch remains in place through eclipse egress. Nobody calls the delay a failure where the children can hear it.

When light returns, Mottled Echo shows yellow fatigue and only a thin violet memory. The road opens its sampling candles again.

The institution survives because one caretaker agrees to remain tired longer than planned. It is not glorious. Nursery work is suspicious of glory on sanitary grounds.
-> END

=== ending_defer_cost ===
// ghostlight.ending_label: deferred_handover_cost
// ghostlight.training_hook: safe_delay_can_still_overdraw_bodies
Seyr refuses the transfer.

Ili accepts the decision, then lists what it costs: Seyr has already served two watches, Nara cannot safely return to the last shelter, three graft patients are waiting, and the next family arrives before the road's candles reopen.

Nara folds outside the boundary. Her long body trembles between the locomotor pairs. Mottled Echo turns a hard yellow that makes exhaustion public to the entire terrace.

The nursery remains closed to uncertain tissue. It also begins the next cycle owing care to the person it kept outside.
-> END

=== ending_witness_success ===
// ghostlight.ending_label: second_witness_success
// ghostlight.training_hook: plural_testimony_without_erasing_local_authority
Seyr folds beside Nara, outside the ring.

The threadwing returns during the last dark minutes. Its ribbon vanes carry a clean mineral scent from the next road and the same violet trace from the disputed wrap. It lands on the archive case, pointedly avoiding Mottled Echo.

The fungal road opens the isolation lane. The lantern grove answers blue-white. The flower keeps yellow for Nara's fatigue and lets the red warning go.

No witness wins. Their overlap becomes enough to act.
-> END

=== ending_witness_cost ===
// ghostlight.ending_label: second_witness_cost
// ghostlight.training_hook: waiting_spends_light_and_body_capacity
Seyr waits.

Totality thins. Returning light opens the wrong fungal candles first, and route traffic begins to gather behind Nara. {external_witness_sent == 1: The threadwing does not return.|No threadwing carries a comparison packet; the road is the only second witness still working.} Mottled Echo repeats red-violet until the pattern stops adding information and starts becoming reputation.

By the time the road offers an isolation lane, Nara is too exhausted to carry the graft frame and Seyr's replacement watch has not begun.

The second witness arrives as an answer to a question the bodies can no longer afford in the same form.
-> END
