// ghostlight.artifact_id: numen_witnessed_share_branch_fold_v0
// ghostlight.fixture_id: numen-witnessed-share-v0
// ghostlight.scene_id: numen-witnessed-share-v0.ash-halo-share-before-second-horn
// ghostlight.final_ink_path: examples/ink/kalsa/numen-witnessed-share-v0.branch-and-fold.v0.ink

VAR share_clarity = 1
VAR route_safety = 1
VAR branch_pressure = 1
VAR divine_terms = 0
VAR answer_roll_open = 0
VAR petition_status = 0
VAR daro_condition = 2
VAR support_due = 0
VAR manifestation_pressure = 1

-> start

=== start ===
Before the first horn, the Ash-Halo Court is mostly a place where people count things.

Forge workers count iron scales onto four waist-high racks. Sedren, the forge custodian, counts the counterweight teeth and finds one more than yesterday, which is how he knows yesterday's return was copied by someone who had never met a gear. He marks the disputed tooth in yellow chalk and moves on. There is a siege above; perfection may file an appeal later.

The rectangular stone court is crossed by a live heat trunk. The fitting circle lies in its middle. Racks and hanging weights line the north wall. Along the south wall, a black iron quench trough drains through a grated channel into the under-gallery below.

-> witness_landing

=== witness_landing ===
Taru waits on the west stair landing with a household copy of the answer roll inside his coat. He is here to recognize the sign of a defeated champion whose shrine went silent after the first armament. He is not here to name whatever makes the sign now. Melka, the death witness, has made him repeat that distinction until it sounds almost like wisdom.

Melka checks the red witness cords on her sealed roll. Orsa, intermediary of the Iron Shelter, rehearses the three offers the divine hierarchy permits: service, departure, or binding. Vael balances the champion's grant against the two outer shrines already stripped of ordinary protection. Daro stands in padded cloth at the fitting circle while healers oil the anchor wires against his ribs.

At a narrow desk east of the fitting circle, Nema, the branch registrar, sharpens a reed pen. She records refusals exactly and interpretations in Ninth Furnace's favor. Both practices fit comfortably on the same page.

Iresa, holder of Ninth Furnace, watches the east heat-gate arch and the horn keeper beside it. If the armour closes, Daro can lead the assault to reclaim the East Collar. If the rite delays, the branch hostel and tributary bakehouse keep receiving cold water.

-> prepare_choice

=== prepare_choice ===
// ghostlight.choice_layer: ordinary_preparation
+ [Open the household copy beside Melka and read the champion's recorded sign into the court return.]
    // ghostlight.action_label: show_object
    // ghostlight.branch_label: prepare_open_copy
    ~ answer_roll_open = 1
    ~ share_clarity = share_clarity + 2
    ~ branch_pressure = branch_pressure + 1
    Taru lays the copy flat beneath Melka's seal weights. The old entry describes the champion's left hand closing over an empty right wrist, twice, before the face turns away.

    Nema, the branch registrar, records the reading and adds, "Claimed sign, claimant unresolved."

    Iresa says, "Your ink has now delayed an army by one sentence. Spend the next one carefully."
    -> preparation_fold
+ [Walk the quench route with Sedren and chalk the outlet into the empty cooling bay.]
    // ghostlight.action_label: move_and_mark
    // ghostlight.branch_label: prepare_safe_outlet
    ~ route_safety = route_safety + 2
    ~ share_clarity = share_clarity + 1
    ~ support_due = support_due + 1
    Taru follows Sedren down three steps to the trough grate. Sedren lifts it with a hook. The newer drain bends toward an empty cooling bay; the old throat drops under the court toward the slag cistern.

    Taru draws a yellow line around the newer outlet. Sedren adds his square stop mark across the old throat.

    "Now the dead have a preferred door," Sedren says. "They are welcome to respect workmanship for once."
    -> preparation_fold
+ [Make Orsa state the Iron Shelter's three offers before the horn makes them sound inevitable.]
    // ghostlight.action_label: speak
    // ghostlight.branch_label: prepare_public_terms
    ~ divine_terms = divine_terms + 2
    ~ share_clarity = share_clarity + 1
    ~ branch_pressure = branch_pressure + 1
    "Service by a living promisor," Orsa says. "Departure along a marked outlet. Binding if the claimant can answer and the Shelter accepts. None of these proves a name."

    The last sentence costs her. Everyone hears it cost her.

    Daro says, "Good. Gods should have to hear their own small print before they wear me."
    -> preparation_fold
+ [Keep the copy folded and watch the trough, the wires, and the people who keep looking away from each.]
    // ghostlight.action_label: wait
    // ghostlight.branch_label: prepare_watch_silently
    ~ branch_pressure = branch_pressure - 1
    ~ share_clarity = share_clarity - 1
    ~ manifestation_pressure = manifestation_pressure + 1
    Taru leaves the copy inside his coat.

    The quench water is still. The anchor wires are slack. Nema's pen waits above an empty line.

    Waiting reveals no doctrine. It does reveal that Vael checks the covered trough whenever he subtracts from the outer shrines.
    -> preparation_fold

=== preparation_fold ===
// ghostlight.fold: preparation_enters_first_horn
The horn keeper lifts the first horn.

{answer_roll_open == 1: Melka's original remains sealed, but Taru's household copy lies open beneath two witnesses.}
{answer_roll_open == 0: Both copies remain closed. Taru can feel the household roll's hard edge against his ribs.}
{route_safety >= 3: A yellow route line runs from the trough to the empty cooling bay; the old cistern throat bears Sedren's square stop mark.}
{divine_terms >= 2: Orsa's offer has entered Nema's return before the God can alter it.}
{branch_pressure >= 3: Iresa has stopped watching the east arch. She watches Taru instead.}

Taru returns to the west landing before the horn sounds.

The first horn sounds. Workers open the source racks. Furnace draft lifts ash from the floor in narrow grey sheets.

-> first_face

=== first_face ===
The face appears in the black quench water before anyone pours heat into it.

-> face_holds

=== face_holds ===
It is a face only while seen from the west landing. From the fitting circle, Daro sees an empty trough. In the polished copper splints, every witness sees the east heat-gate arch reflected where the face should be.

Then a left hand rises under the water and closes over an empty right wrist. Twice.

Taru knows the gesture. That is not the same as knowing the hand.

Orsa orders the trough covered. Sedren does not move until someone names whose authority reaches the grate.

-> face_choice

=== face_choice ===
// ghostlight.choice_layer: first_manifestation_response
+ [File the defeated champion's petition with Melka before the second horn.]
    // ghostlight.action_label: speak_and_transfer_record
    // ghostlight.branch_label: petition_formally
    ~ petition_status = 2
    ~ share_clarity = share_clarity + 1
    ~ branch_pressure = branch_pressure + 1
    ~ support_due = support_due + 1
    Taru gives Melka the household copy.

    "Recorded sign," he says. "One warning or withdrawal. No binding accepted in the champion's name."

    Melka repeats only what she owns: prior sign, present match, living recognizer, requested act. Nema writes the refusal in full and underlines nothing.
    -> share_hearing
+ [Repeat the gesture above the trough and ask the presence for one warning.]
    // ghostlight.action_label: gesture_and_speak
    // ghostlight.branch_label: petition_by_sign
    ~ petition_status = 1
    ~ share_clarity = share_clarity + 1
    ~ manifestation_pressure = manifestation_pressure + 1
    Taru closes his left hand over his bare right wrist. Once. Twice.

    "One warning," he says. "Then be still while we decide what heard us."

    The hand beneath the water performs the gesture a third time.

    Melka says, "The copy records two."
    -> share_hearing
+ [Help Sedren bar the old throat before anyone offers the presence a share.]
    // ghostlight.action_label: use_object
    // ghostlight.branch_label: secure_route_before_hearing
    ~ route_safety = route_safety + 1
    ~ branch_pressure = branch_pressure + 2
    ~ daro_condition = daro_condition - 1
    ~ support_due = support_due + 1
    Taru takes the long grate hook while Sedren drives the stop plate across the old throat. The work costs half the interval between horns. Daro waits in tightening anchor wires while a healer counts his breaths.

    Iresa says, "If the east gate breaks while you mend etiquette, I will enter that in every copy."

    Sedren says, "Use waterproof ink."
    -> share_hearing
+ [Let Orsa cover the trough and keep the household copy in your coat.]
    // ghostlight.action_label: withhold_object
    // ghostlight.branch_label: suppress_claim
    ~ petition_status = 0
    ~ branch_pressure = branch_pressure - 1
    ~ manifestation_pressure = manifestation_pressure + 1
    Taru steps back.

    Orsa lowers the iron cover. Water taps once against its underside, although the trough is still.

    Nema writes, "No petition presented."

    Taru keeps the copy. He also keeps the knowledge that this is how a record becomes a secret while everybody watches.
    -> share_hearing

=== share_hearing ===
// ghostlight.fold: petition_review_before_route
{petition_status >= 1 && share_clarity >= 3:
    ~ petition_status = 3
    Orsa enters one witnessed share: a single answer, a marked departure, or service promised only by the living. She refuses to enter binding as accepted on behalf of the dead.
- else:
    {petition_status >= 1:
        ~ petition_status = 1
        Orsa records a challenged petition. The sign is present; the terms and identity are not clean enough for the branch to call it admitted.
    - else:
        The docket carries no share. Whatever is under the trough remains an unrecognized diversion in the Iron Shelter's route.
    }
}

{route_safety >= 3: Sedren leaves the cooling-bay outlet open and keeps his hand on the lever that isolates the slag cistern.}
{route_safety < 3: The new drain and old cistern throat remain connected beneath the grate. Nobody can see which channel a diverted flow will take.}
{daro_condition <= 1: Daro's hands have begun to tremble inside the anchor loops. The delay has become bodily.}
{manifestation_pressure >= 3: Under the iron cover, two different rhythms knock the same recorded gesture.}

Taru takes the lower west step, one stride from Sedren's lever and clear of the fitting circle.

The second horn sounds.

-> route_opens

=== route_opens ===
White heat enters Daro's slack wires. The first rack leans inward. Across the court, every loose iron scale turns its edge toward him.

{petition_status == 3:
Orsa speaks into the route. "You are recognized for one act. Answer, depart, or hear terms of service. Binding remains unaccepted."

The Iron Shelter answers through the scales around Daro: "I admit the interval. I do not know the hand."
- else:
The Iron Shelter answers through the scales around Daro: "Diversion."
}

The face is gone from the trough. Its gesture moves instead through the anchor wires, one loop after another, closing over an absence no forge worker put there.

-> route_choice

=== route_choice ===
// ghostlight.choice_layer: route_open_response
+ {petition_status == 3 && divine_terms >= 2} [Promise your own service: carry one warning to the two withdrawn shrines, then demand one answer.]
    // ghostlight.action_label: speak
    // ghostlight.branch_label: accept_living_service
    ~ support_due = support_due + 1
    "My feet, my voice, one journey," Taru says. "Not the dead champion's obedience. One warning, then the share closes."

    Orsa repeats the promise exactly. The Iron Shelter does not repeat it at all.
    -> ending_witnessed_warning
+ {petition_status == 3 && route_safety >= 3} [Pull Sedren's marked lever and give the presence the prepared departure route.]
    // ghostlight.action_label: use_object
    // ghostlight.branch_label: open_marked_departure
    ~ route_safety = route_safety + 1
    ~ support_due = support_due + 1
    Taru and Sedren pull together. The old throat closes. The cooling-bay outlet opens with a groan that shakes soot from the south wall.

    Orsa names departure. Vael retracts enough of the grant to keep the route from becoming a second armament.
    -> ending_marked_departure
+ [Give Sedren the square stop signal before the second rack falls.]
    // ghostlight.action_label: gesture
    // ghostlight.branch_label: use_material_stop
    ~ branch_pressure = branch_pressure + 2
    ~ support_due = support_due + 1
    ~ daro_condition = daro_condition - 1
    Taru closes one fist and cuts it downward.

    Sedren drops the stop bar. Counterweights strike their catches. The second rack freezes half a handspan above the fitting circle.
    -> ending_material_stop
+ [Refuse binding in the dead champion's name and force the hierarchy to answer under witness.]
    // ghostlight.action_label: speak
    // ghostlight.branch_label: refuse_dead_binding
    ~ branch_pressure = branch_pressure + 1
    ~ support_due = support_due + 1
    "No living mouth here accepts binding for that dead," Taru says. "Withdraw, negotiate, or strike where the return can name it."

    Orsa goes pale, but she carries the refusal into the route.
    -> ending_refusal_witnessed
+ [Say nothing and let the Iron Shelter pull the diverted grant inward.]
    // ghostlight.action_label: silence
    // ghostlight.branch_label: permit_interception
    ~ daro_condition = daro_condition - 1
    Taru keeps both hands visible and offers neither sign nor stop.

    The Iron Shelter takes silence for an empty docket.
    -> ending_interception

=== ending_witnessed_warning ===
// ghostlight.ending_label: witnessed_warning
// ghostlight.training_hook: living_service_does_not_consent_for_dead
Every slack anchor wire speaks in Taru's childhood voice.

"The breach begins behind the gate."

The wires fall quiet. The household copy and Nema's fresh return acquire the same wet thumbprint on their blank identity lines. A third matching print darkens the outside of Melka's still-sealed case at the height of the line inside. None belongs to anyone present. Melka checks anyway.

Vael disperses the remaining fitting. Daro receives lighter raiment and keeps enough movement to lead a delayed assault. One outer shrine can recover part of its ordinary protection before the third horn.

{support_due >= 3: Nema's return owes witness costs, safe lodging, and restored shrine work before it records celebration.}
{branch_pressure >= 3: Iresa tells Taru the branch will review his sponsorship after the gate. She does not order the wet copies destroyed.}

Orsa says the Iron Shelter admitted an interval. She does not say it understood the warning.
-> END

=== ending_marked_departure ===
// ghostlight.ending_label: marked_departure
// ghostlight.training_hook: negotiated_route_preserves_uncertainty
Black quench water climbs the dry inner wall of the trough, crosses the iron lip, and enters the yellow-marked channel toward the empty cooling bay.

It climbs before it falls.

Sedren watches the lever. Vael watches the grant. Orsa watches Daro. Taru watches the covered trough, where the same face remains visible beneath iron while the dark water leaves somewhere else.

The presence reaches the cooling bay as a handprint in steam. The face in the trough closes its eyes at the same instant. No office present owns the claim that these were one thing.

The armour closes in three slower sections. Daro can march, but the third horn finds him with one shoulder still bare and the assault loses its clean spectacle.

{support_due >= 3: The return funds a cooling-bay watch and the households displaced from the route.}
{branch_pressure >= 3: Iresa preserves the result because it worked, then opens a review because it worked in public.}
-> END

=== ending_material_stop ===
// ghostlight.ending_label: material_stop
// ghostlight.training_hook: ritual_does_not_erase_custodian_authority
The stop bar holds.

Hot scales hang above Daro without closing. Healers pull his anchor cloth wet against his ribs. Vael retracts the grant in pieces while Orsa names each withdrawal so the Iron Shelter does not mistake the failing armour for surrender.

{route_safety >= 3: The marked cooling route carries the black water away from the fitting circle while the old cistern throat stays shut.}
{route_safety < 3: Water strikes both drains below the grate. Something knocks from the slag cistern after the divine light has gone.}

The third horn passes unanswered. Iresa loses her champion's assault and gains a forge return bearing Sedren's lawful stop, Taru's signal, and Daro's shaking hands.

{support_due >= 2: Nema marks treatment, stopped forge labor, and a cooling watch before Iresa can dictate the heading.}

On the thin ceramic run, yellow chalk has become a line of tiny wet fingerprints. The pipe is hot enough to boil spit. Melka records the prints and declines three explanations before anyone offers a fourth.
-> END

=== ending_refusal_witnessed ===
// ghostlight.ending_label: refused_binding_under_witness
// ghostlight.training_hook: divine_response_is_powerful_but_not_omniscient
The Iron Shelter pulls the grant tight around Daro.

"Then it is not mine," the scales say.

The hand in the wires answers by closing over Daro's right wrist instead of its own absent one.

{petition_status == 3: Orsa calls the act outside the admitted share. Melka calls it a changed sign. Nema writes both.}
{petition_status < 3: Orsa calls it an attack. Melka records that no recognized interval was offered. Nema writes both.}

The armour closes, but Daro refuses the march until a healer cuts the wrist loop free. The assault begins without its promised champion. The God has answered a refusal with force and still has not shown that it knew what touched its route.

{support_due >= 3: The return owes treatment, witness protection, and restoration to the stripped shrines.}
{branch_pressure >= 3: Iresa orders a review of Taru, Orsa, and the registrar's copies; separated custody prevents the order from becoming one convenient fire.}
-> END

=== ending_interception ===
// ghostlight.ending_label: forceful_interception
// ghostlight.training_hook: victory_record_and_mortuary_record_diverge
The furnace draft folds inward.

The gesture passes through every anchor loop at once. White fire follows it. For an instant, the face appears in the gaps between the iron scales, looking toward neither Taru nor the champion.

Then the armour closes.

{daro_condition >= 2: Daro stays upright. He crosses the east arch in complete raiment as the third horn sounds.}
{daro_condition < 2: Daro drops to one knee before forcing the armoured body upright. The healers' protest vanishes under the third horn.}

The high-branch return will begin with the held gate if the assault succeeds. Melka's return begins now, with the broken gesture and the sound that comes from inside her still-sealed roll.

Three knocks. A pause. Three knocks again.

The recorded sign uses two.

Nobody in the court knows what has counted them.
-> END
