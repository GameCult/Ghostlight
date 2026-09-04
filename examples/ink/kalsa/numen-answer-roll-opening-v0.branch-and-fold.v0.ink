// ghostlight.artifact_id: kalsa_numen_answer_roll_opening_branch_fold_v0
// ghostlight.fixture_id: numen-answer-roll-opening-v0
// ghostlight.scene_id: numen-answer-roll-opening-v0.first-and-second-horn
// ghostlight.final_ink_path: examples/ink/kalsa/numen-answer-roll-opening-v0.branch-and-fold.v0.ink

VAR roll_copy_strength = 1
VAR outside_copy = 0
VAR route_trace = 0
VAR registrar_access = 1
VAR witness_support = 1
VAR branch_pressure = 2
VAR opening_delay = 0
VAR shrine_exposure = 2
VAR feeder_attention = 1
VAR claimant_standing = 1
VAR sedren_stop = 0
VAR request_kind = 0

-> start

=== start ===
Ninth Furnace has prepared for a god with the confidence of people who have measured every piece of iron and none of the god.

The Ash-Halo Court is a long basalt room sunk into Jamnai's geothermal Crown. Furnace mouths burn along the east wall. Iron scales, copper wire, resin, ceramic splints, and quench buckets stand on marked racks around the central fitting dais. Counterweight tracks cross the ceiling. At the south wall, a stone trough drains through a grated throat toward the old slag cistern beneath the floor.

Two stairs leave the court. The north stair climbs past the registrar's threshold table. The southwest stair descends beside the quench channel to the under-gallery where the dead have answered before.

Melka, a mortuary witness, owns the sealed answer roll resting under both hands. Ownership here means that everyone important has explained why she should surrender it.

-> threshold_routine

=== threshold_routine ===
// ghostlight.scene: ordinary_threshold_work
Melka stands on the court side of the threshold table. Taru sits across from her on the stair side, the north exit at his back, with a household copy, a wax tablet, and three travel cakes built to survive weather, siege, and apparently chewing. He is the only person present who knew the defeated champion's shrine signs before that shrine went silent.

Nema, Ninth Furnace's registrar, checks the case seal without touching it. Her branch copy records Iresa's order: the roll may enter before the second horn only if its claimant asks for one bounded act.

"A beautiful rule," Taru says. "Was it written before or after they decided who counts as bounded?"

"After," Nema says. "That is when rules become beautiful."

-> court_roles

=== court_roles ===
// ghostlight.scene: prepared_court_roles

Beyond them, Sedren inspects the rack marks and quench fall. Vael counts a divine grant borrowed from a branch hostel and a furnace-family shrine. Orsa waits beside the fitting dais to speak for the Iron Shelter. Daro, the fasting champion, lies beneath padded anchor wires while healers test his breath and hands.

The first horn has not sounded. There is still time for one ordinary kindness or one ordinary precaution, which are often the same thing after the receipts arrive.

-> preparation_choice

=== preparation_choice ===
// ghostlight.choice_layer: threshold_preparation
+ [Make a fresh copy with Taru and leave each correction visible.]
    // ghostlight.action_label: copy_object
    // ghostlight.branch_label: prime_household_copy
    ~ roll_copy_strength = roll_copy_strength + 2
    ~ branch_pressure = branch_pressure + 1
    Melka reads each entry only as far as sign, place, and last appearance. Taru scratches his household correction beside it. Neither writes a name where the roll has none.

    Nema watches the second copy become another thing her office will have to defeat honestly.
    -> routine_fold
+ [Walk the quench path with Sedren before opening the case.]
    // ghostlight.action_label: move
    // ghostlight.branch_label: prime_route_trace
    ~ route_trace = route_trace + 2
    ~ sedren_stop = sedren_stop + 1
    ~ opening_delay = opening_delay + 1
    ~ shrine_exposure = shrine_exposure + 1
    Melka follows Sedren from the trough to the southwest stair. He chalks the drain lip, the maintenance throat, and the point where the newer channel crosses above the old cistern.

    "Water goes down," he says.

    "Comforting."

    "Nothing else here has signed that promise."
    -> routine_fold
+ [Invite Nema to compare the seal and custody marks before witnesses arrive.]
    // ghostlight.action_label: show_object
    // ghostlight.branch_label: prime_registrar_seal
    ~ registrar_access = registrar_access + 2
    ~ roll_copy_strength = roll_copy_strength + 1
    ~ branch_pressure = branch_pressure - 1
    Melka turns the case until Nema can see the mortuary cord, Taru's household wax, and the delvers' transit mark.

    Nema copies all three. She also copies that Melka did not hand her the case. A registrar can make restraint sound like a claim of title if given enough margins.
    -> routine_fold
+ [Break one travel cake with Taru and rehearse where each witness stands.]
    // ghostlight.action_label: use_object
    // ghostlight.branch_label: prime_witness_support
    ~ witness_support = witness_support + 2
    ~ claimant_standing = claimant_standing + 1
    ~ opening_delay = opening_delay + 1
    Taru drinks. Melka places him north of the table, outside the rack line, with the stair at his back and the quench trough in view.

    He performs the old sign once: two knuckles touched by the thumb, then an open palm. Once is memory. Repetition beside gathered power is an invitation.
    -> routine_fold

=== routine_fold ===
// ghostlight.fold: prepared_threshold_before_first_horn
Workers open charcoal bins and oil the counterweight brakes. A healer wipes Daro's palms. Vael's clerk carries notices to the hostel and shrine whose protections have been withdrawn.

{roll_copy_strength >= 3: Two answer rolls now disagree in public ink where Taru corrected the household signs. The disagreement is safer than a perfect copy.}
{route_trace >= 2: Three white chalk bars mark the physical route from trough to old cistern. Sedren can point to where his authority ends.}
{registrar_access >= 3: Nema has enough custody detail to certify the opening later, and enough knowledge to challenge it now.}
{witness_support >= 3: Taru stands fed, watered, and positioned beside the north stair rather than trapped between the branch and its miracle.}
{branch_pressure >= 3: Two armed branch attendants drift closer to the threshold table while pretending to inspect floor marks.}
{shrine_exposure >= 3: A third absence notice joins Vael's docket. Delay is already becoming somebody else's cold room.}

Sedren lifts one hand.

The first horn sounds.

-> first_answer

=== first_answer ===
// ghostlight.scene: the_answer_before_the_opening
The quench water rises against its stone lip although no bucket has emptied.

A face forms in the black reflection beneath the grate.

Its hand touches two knuckles with its thumb. Before Taru can answer, the fingers open toward Melka's sealed case.

Orsa does not name it. Taru stops breathing for one count. Nema says, very softly, "The roll remains closed."

The rite has received an answer before anyone asked a question.

-> first_answer_choice

=== first_answer_choice ===
// ghostlight.choice_layer: answer_before_opening
+ [Unseal the answer roll at the threshold with Taru and the delver witness watching.]
    // ghostlight.action_label: open_object
    // ghostlight.branch_label: answer_open_now
    ~ claimant_standing = claimant_standing + 2
    ~ feeder_attention = feeder_attention + 1
    ~ opening_delay = opening_delay + 1
    ~ branch_pressure = branch_pressure + 1
    Melka cuts the mortuary cord. Taru lays his household copy beside the original. The delver at the stair writes down who can leave with which record.

    The face in the trough watches the page upside down.
    -> opening_fold
+ [Cover the trough, trace its drain with Sedren, then open the roll.]
    // ghostlight.action_label: block_object
    // ghostlight.branch_label: answer_cover_and_trace
    ~ route_trace = route_trace + 1
    ~ sedren_stop = sedren_stop + 2
    ~ feeder_attention = feeder_attention - 1
    ~ opening_delay = opening_delay + 2
    ~ shrine_exposure = shrine_exposure + 1
    Sedren drops the fitted stone cover across the trough. Melka keeps one hand on the case while they watch a cold line travel under the cover toward the southwest stair.

    "Within my perimeter," Sedren says, "water. Beyond it, an argument with excellent timing."

    They return to the threshold and open the roll.
    -> opening_fold
+ [Send a corrected copy up the north stair before opening the original.]
    // ghostlight.action_label: transfer_object
    // ghostlight.branch_label: answer_send_copy_out
    ~ outside_copy = outside_copy + 2
    ~ roll_copy_strength = roll_copy_strength + 1
    ~ branch_pressure = branch_pressure + 1
    ~ opening_delay = opening_delay + 1
    A delver carries the corrected copy beyond the branch threshold. Nema records the departure. One attendant objects too late and then objects to the lateness as well.

    Melka opens the original where the empty space proves a second record survived.
    -> opening_fold
+ [Let Nema compare every seal before Melka opens the case herself.]
    // ghostlight.action_label: show_object
    // ghostlight.branch_label: answer_registrar_comparison
    ~ registrar_access = registrar_access + 1
    ~ branch_pressure = branch_pressure - 1
    ~ claimant_standing = claimant_standing - 1
    ~ opening_delay = opening_delay + 1
    Nema names the mortuary cord, household wax, delver mark, and unbroken hinge. Melka answers each item, then opens the case without moving it across the line.

    The branch receives procedure. Taru receives the reminder that procedure can still make a claimant wait while the water learns their sign.
    -> opening_fold

=== opening_fold ===
// ghostlight.fold: answer_roll_opened
Melka reads the first entry as a sequence of observations: scarred hand, furnace-family pressure warning, last appearance before the first armament. Taru reads the household correction. Nema records that they do not agree on the second motion.

Melka turns to the defeated champion's entry.

Before she reads its sign, something taps twice beneath the stone trough cover.

{feeder_attention >= 2: Ash lifts from the nearest floor seam and sketches an open palm with one finger too many. Taru says only, "Related." Orsa says nothing at all.}
{feeder_attention <= 1: The cover stays still. A wet thumbprint appears on the dry margin beside an entry nobody has read.}
{outside_copy >= 2: Somewhere above the north stair, the corrected copy is already outside the court's control.}
{registrar_access >= 3: Nema can certify the seals. She cannot certify why the unread entry answered.}

Orsa steps to the threshold, iron-thread mantle dull in the furnace light.

"The hierarchy will hear one bounded request before the route opens," she says. "Name the act. Do not name the answer for it."

-> request_choice

=== request_choice ===
// ghostlight.choice_layer: bounded_claim
+ [Enter a request for warning if the recorded sign reaches the grant.]
    // ghostlight.action_label: speak
    // ghostlight.branch_label: request_warning
    ~ request_kind = 1
    ~ claimant_standing = claimant_standing + 1
    Melka records warning: one gesture or voice carried back to the threshold, no claim of identity, no duty imposed on descendants.

    Taru signs. Orsa repeats only the requested act.
    -> request_fold
+ [Enter a recognized share so the claimant may approach without being struck as an intruder.]
    // ghostlight.action_label: write
    // ghostlight.branch_label: request_share
    ~ request_kind = 2
    ~ feeder_attention = feeder_attention + 2
    ~ shrine_exposure = shrine_exposure + 1
    ~ branch_pressure = branch_pressure + 1
    Melka records one measured fitting interval. Vael writes the share beside the protection already withdrawn from the hostel and shrine.

    Taru refuses service and binding in the claimant's name. He accepts only the chance to answer.
    -> request_fold
+ [Enter withdrawal if the sign appears, with no claim that silence means release.]
    // ghostlight.action_label: speak
    // ghostlight.branch_label: request_withdrawal
    ~ request_kind = 3
    ~ branch_pressure = branch_pressure + 2
    ~ claimant_standing = claimant_standing + 1
    Melka records withdrawal from this working, not surrender to the Iron Shelter and not proof of metaphysical severance.

    Nema's stylus pauses over the branch copy. The pause is small enough to deny later and large enough to govern a funeral.
    -> request_fold
+ [Enter interruption: Sedren stops the machinery if the sign crosses his marked route.]
    // ghostlight.action_label: show_object
    // ghostlight.branch_label: request_interruption
    ~ request_kind = 4
    ~ sedren_stop = sedren_stop + 2
    ~ opening_delay = opening_delay + 1
    ~ shrine_exposure = shrine_exposure + 1
    Melka lays Sedren's three chalk marks beside the entry. Sedren signs for the material perimeter and nothing beyond it.

    Orsa does not promise the God will stop. She records that the forge will.
    -> request_fold

=== request_fold ===
// ghostlight.fold: bounded_request_before_second_horn
Vael enters the delay, the borrowed grant, and the households still waiting on its return. Nema records the branch order. Sedren keeps one palm on the counterweight brake. Orsa speaks the request toward the fitting dais without translating the silence that follows.

{request_kind == 1: Warning is now an admitted act. Any voice or gesture still needs a witness who can say what, not who.}
{request_kind == 2: A narrow share sits inside the allocation. It may permit testimony, feeding, negotiation, or a better attack.}
{request_kind == 3: Withdrawal is now a request the living offices must preserve even if the route answers by pulling harder.}
{request_kind == 4: The forge stop is tied to a visible crossing at Sedren's marks. The God has received no corresponding command.}
{opening_delay >= 4: The furnace loses its first clean fitting interval. Vael marks another span of borrowed protection.}
{shrine_exposure >= 4: Vael adds a second dark edge to the shrine withdrawal. The cost of the opening now has a place and a household route.}
{branch_pressure >= 4: The attendants stop pretending to inspect the floor and stand openly between Taru and the fitting dais.}
{witness_support >= 3: Taru remains steady enough to watch the trough and the register at once.}
{claimant_standing >= 3: Orsa enters Taru's request as recognized before repeating it toward the dais.}
{claimant_standing <= 1: Nema marks Taru's standing challenged. Orsa carries the request as reported speech, not a recognized share.}

The second horn rope lifts in the gallery above.

Melka must decide where the mortuary record will stand when the grant enters the room.

-> second_horn_choice

=== second_horn_choice ===
// ghostlight.choice_layer: record_position_at_route_opening
+ {route_trace >= 2} [Carry the opened original down the southwest stair to the under-gallery.]
    // ghostlight.action_label: move
    // ghostlight.branch_label: carry_roll_below
    {route_trace >= 3 || outside_copy >= 2:
        Melka takes the chalked route with Taru one step above her and the delver holding the stair mouth. The second horn sounds above.
        -> ending_gallery_witnessed
    - else:
        Melka takes the opened case below. The route is known in pieces, and the witnesses are not all where the record needs them.
        -> ending_gallery_cost
    }
+ {roll_copy_strength >= 3} [Keep the original at the threshold and give Taru the corrected copy at the quench line.]
    // ghostlight.action_label: transfer_object
    // ghostlight.branch_label: divide_roll_custody
    Melka presses the corrected copy into Taru's hands. He moves only as far as Sedren's chalk line. The original remains open under mortuary custody where Nema and the delver can see it.
    -> ending_threshold_petition
+ {sedren_stop >= 2} [Lay the interruption entry under Sedren's hand and call his material stop.]
    // ghostlight.action_label: show_object
    // ghostlight.branch_label: call_material_stop
    Melka slides the open entry beside the brake lever. Sedren reads the marked route, not the claimed identity, and closes his fist.
    -> ending_stop_recorded
+ [Close the case and leave by the north stair with Taru.]
    // ghostlight.action_label: withdraw
    // ghostlight.branch_label: withdraw_mortuary_custody
    Melka ties the cut cord around the case as proof that it was opened. Taru gathers the household copy. They turn their backs on the fitting dais before the horn can make departure look like panic.
    -> ending_departure
+ [Record the unresolved entry and let the second horn sound with the original at the threshold.]
    // ghostlight.action_label: wait
    // ghostlight.branch_label: witness_route_from_threshold
    Melka keeps the page open. She neither blesses the grant nor obstructs it. The horn sounds.
    {feeder_attention <= 1 && route_trace >= 2:
        -> ending_proceed_observed
    - else:
        -> ending_proceed_hungry
    }

=== ending_gallery_witnessed ===
// ghostlight.ending_label: gallery_route_witnessed
// ghostlight.training_hook: strange_answer_preserved_without_identity_verdict
The grant enters above as heat, falling weight, and a pressure in the teeth.

At the old cistern grate, water climbs against gravity and touches the edge of the opened roll. The unread sign forms first. Taru answers once. Melka records sequence, position, witnesses, and the fact that Orsa's offered terms cannot be heard below.

Then the water falls.

The copy outside the court survives. The original survives. The face does not stay for a name.

Ninth Furnace has performed the rite correctly and learned almost nothing about what stood inside it. This is not failure. It is the shape honesty takes when a god is in the machinery.
-> END

=== ending_gallery_cost ===
// ghostlight.ending_label: gallery_route_cost
// ghostlight.training_hook: witness_route_without_complete_support
The grant enters before the witnesses settle.

Heat crosses the chalk line. The quench channel kicks hard enough to drive Taru against the stair wall. Melka saves the original case; the open wax leaf takes water and loses half a correction.

Sedren stops one rack fall above. Vael later records the lost fitting interval and another delay in restoring the shrine grant.

Something beneath the grate repeats the erased half of the sign.

Nobody present can prove whether it remembered the page, the dead, or Taru's body braced against stone.
-> END

=== ending_threshold_petition ===
// ghostlight.ending_label: divided_custody_petition
// ghostlight.training_hook: lower_world_owners_preserve_bounded_claim
The second horn sounds with the original under Melka's hand and Taru's copy at the edge of Sedren's marked route.

{request_kind == 1: A voice moves through the quench trough and stops before becoming a word. Melka records attempted warning. Taru records resemblance. Neither writes identity.}
{request_kind == 2: The grant thins for one fitting interval. The face rises far enough to look toward Orsa, then toward the north stair. Vael records the share as spent; Taru refuses the offered binding again.}
{request_kind == 3: The water flattens. The recorded shrine sign does not appear. Nema must preserve the withdrawal request beside a silence that proves nothing.}
{request_kind == 4: Cold crosses Sedren's chalk line. He stops the first weight before the hierarchy decides what the interruption means.}

{registrar_access >= 3: Nema certifies both custody positions and adds Ninth Furnace's objection in her own hand.}
{registrar_access < 3: Nema challenges the household copy, but the original and the delver witness keep the challenge from becoming erasure.}

The branches will argue over whether the rite admitted a claimant. The two records will at least force them to argue about the same acts.
-> END

=== ending_stop_recorded ===
// ghostlight.ending_label: material_stop_before_route
// ghostlight.training_hook: ritual_request_routes_to_real_stop_authority
Sedren arrests the counterweights before the second horn finishes sounding. Iron scales sway above Daro and do not fall.

Orsa says the Iron Shelter was ready. Sedren says the quench route was not. Melka records both statements and the entry under his hand. Vael records charcoal wasted, fitting time lost, and protection still absent from the hostel and shrine.

At the covered trough, three knocks travel from the cistern toward the court, pause beneath Sedren's chalk line, and stop.

The assault has lost its promised hour. The court has not learned whether the stop obeyed the dead, frustrated a feeder, or merely closed a physical route at the right moment.
-> END

=== ending_departure ===
// ghostlight.ending_label: mortuary_custody_withdrawn
// ghostlight.training_hook: refusal_preserves_evidence_but_not_safety
Melka and Taru reach the north stair before the second horn.

{outside_copy >= 2: The corrected copy meets them above. Three records now preserve why the mortuary office withdrew.}
{outside_copy < 2: Taru's household wax and Melka's opened original are the only records leaving. Nema keeps the branch account below.}

The grant opens behind them. The branch gains its armament interval without a mortuary witness at the route. The hostel and shrine remain exposed until Vael can restore their shares.

From behind the closed court door comes the sound of a hand striking wet stone twice.

Leaving keeps the record free. It does not keep the working innocent.
-> END

=== ending_proceed_observed ===
// ghostlight.ending_label: route_opened_under_observation
// ghostlight.training_hook: procedure_preserves_consequence_without_controlling_god
The grant enters Daro and the prepared court. Iron moves in its marked order. Sedren watches the chalked route. Melka watches the roll.

{request_kind == 1: A wet pressure forms around the warning entry, but no gesture completes.}
{request_kind == 2: The share line darkens on Vael's wax, though nothing visible accepts Orsa's terms.}
{request_kind == 3: The shrine sign remains absent while the quench water pulls toward the fitting dais.}
{request_kind == 4: No crossing reaches Sedren's stop before the first rack settles.}

The armour begins to close. The original remains at the threshold. Every office can say what it did, and none can say what the God understood.
-> END

=== ending_proceed_hungry ===
// ghostlight.ending_label: hungry_route_opened
// ghostlight.training_hook: concentrated_working_crowds_its_own_horizon
The second horn opens the grant.

The quench trough empties upward.

Ash lifts in several faces around Daro's fitting dais. One repeats Taru's sign. Another turns toward the north stair. A third has no features except a mouth moving around Orsa's offer.

Sedren reaches for the brake. Vael reaches for the reserve. Nema reaches for the branch copy. Melka keeps both hands on the open roll because custody is the one miracle she can still perform without lying.

The Iron Shelter pulls the grant inward. Whether it defends, binds, feeds, or destroys any presence will belong to what survives the next interval.
-> END
