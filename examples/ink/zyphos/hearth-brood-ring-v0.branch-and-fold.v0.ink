// ghostlight.artifact_id: hearth_brood_ring_v0_branch_fold_v0
// ghostlight.fixture_id: hearth-brood-ring-v0
// ghostlight.scene_id: hearth-brood-ring-v0.lantern-side-first-route
// ghostlight.final_ink_path: examples/ink/zyphos/hearth-brood-ring-v0.branch-and-fold.v0.ink

VAR brood_trust = 2
VAR herd_consent = 2
VAR family_readiness = 1
VAR child_voice = 1
VAR escort_prepared = 0
VAR route_time = 3
VAR care_debt = 1
VAR separation_pressure = 0
VAR plate_calm = 2
VAR shared_route = 0
VAR handover_witness = 1

-> start

=== start ===
The lantern-side warm hollow is busiest when the sun disappears.

It is a shallow oval of dark, root-bound stone at the edge of a Sa'ueia breeding ground. A broad nursery ramp enters from the inner end. At the outer end, two waist-low root arches open onto paired branches of a living fungal road: the left follows a glassback calving lane, while the right carries Sa'ueia family traffic. A low root-grown work shelf fills the pier between them. Beyond the first bend, the branches can join toward honest water when both travelers and road accept the arrangement. Umbros-facing lantern trees curve around the long sides. Their cold blue knots brighten during the daily eclipse, while the glassbacks resting in the hollow supply the useful heat.

The arrangement works because everyone involved understands that beauty is not a fuel budget.

-> ring_people

=== ring_people ===
Pera is the outgoing brood-ring caretaker. They fold four tall running legs beside the mineral-sand rest bed and keep the smaller chest hands free for grooming. Rust-colored body fibers silver around the muzzle; one facial fan has a bite-shaped notch earned from a patient who objected to medicine on constitutional grounds.

Doro waits at the nursery ramp with an empty flank frame. Doro belongs to the mobile family unit that will take young Tavi onto a first long route after eclipse. The frame is sized for supplies and rest cloths, not for carrying a child who has four perfectly good running legs and several views on being packed.

Tavi lies against Low Ember, a glassback calf whose translucent dorsal plates glow with stored copper warmth. Tavi's smaller chest fingers clean mineral grit from a plate seam. Low Ember leans just enough to make the work easier and not enough to admit enjoying it.

Beyond the left outer arch, Broad Rain waits with the returning glassback herd. Their tall dorsal plates hold slow blue and amber bands: welcome, departure, feeding light soon. Doro must catch the family branch while it remains open. Herd and family travel both depend on the same post-eclipse road window.

The routine handover comes first. Everyone is less wise when hungry.

-> routine_care_choice

=== routine_care_choice ===
// ghostlight.choice_layer: routine_care
+ [Clean Low Ember's plate seams with Tavi and show Broad Rain the discarded parasites.]
    // ghostlight.action_label: groom
    // ghostlight.branch_label: prepare_calf_care
    ~ herd_consent = herd_consent + 1
    ~ plate_calm = plate_calm + 1
    ~ handover_witness = handover_witness + 1
    Pera settles on one side of Low Ember and lets Tavi keep the familiar side.

    Two chest hands lift the edge of a translucent plate. Tavi works a soft mineral comb through the warm seam. Three pale parasites release their grip and curl into a disposal cup with the wounded dignity of officials removed from office.

    Broad Rain turns broadside beyond the arches. Blue crosses the adult's plates: care seen.

    "Do we have to show everyone?" Tavi asks.

    "Only the parasites," Pera says. "They crave recognition."
    -> routine_fold
+ [Ask Tavi to show Doro the facial-fan signal that means stop touching.]
    // ghostlight.action_label: gesture
    // ghostlight.branch_label: rehearse_child_refusal
    ~ child_voice = child_voice + 2
    ~ family_readiness = family_readiness + 1
    ~ brood_trust = brood_trust + 1
    Tavi opens both facial fans, then snaps the left one shut and turns the bare throat patch away.

    Doro reaches toward the travel binding at Tavi's shoulder and stops before contact.

    "Again," Doro says.

    Tavi repeats it faster.

    "That one meant stop," Pera says. "The faster one meant stop before you finish making it educational."

    Doro folds lower until their eyes are level. "Understood."
    -> routine_fold
+ [Practice the paired rest call and let both young decide where Doro belongs in the hollow.]
    // ghostlight.action_label: wait
    // ghostlight.branch_label: rehearse_shared_rest
    ~ brood_trust = brood_trust + 2
    ~ family_readiness = family_readiness + 1
    ~ route_time = route_time - 1
    Pera sounds the low two-note rest call.

    Low Ember lowers onto the mineral sand. Tavi folds into the warm angle beside the calf's plates. Doro chooses a place near Tavi's head.

    Low Ember fogs the nearest plate until Doro moves one body-length farther away.

    The plate clears.

    "Accepted," Pera says.

    "I moved away."

    "Yes. This is often the expensive part of acceptance."
    -> routine_fold
+ [Walk the empty flank frame through the outer arches and ask the road for a shared first-leg lane.]
    // ghostlight.action_label: move_object
    // ghostlight.branch_label: prepare_shared_escort
    ~ escort_prepared = escort_prepared + 2
    ~ family_readiness = family_readiness + 1
    ~ herd_consent = herd_consent + 1
    ~ route_time = route_time - 1
    Pera balances Doro's empty frame across their own flanks and walks it through the left root arch.

    They set one clean rest cloth beside the fungal candles and trace a route that would keep family and herd together until the first honest-water hollow.

    The road opens one amber candle along that line. Broad Rain answers with a narrow blue band.

    Neither is consent. Both are permission to keep asking.
    -> routine_fold

=== routine_fold ===
// ghostlight.fold: ordinary_brood_care_before_departure
The eclipse deepens. Lantern knots turn the hollow blue. Warmth moves through Low Ember's copper plates into Tavi's folded side and the dark mineral sand.

Pera passes Doro the grooming comb, the rest cloths, and the small route archive. The archive can say what happened. It cannot make Doro's hands familiar.

{brood_trust >= 4: Tavi and Low Ember settle in easy contact, each changing posture before the other has to ask.}
{herd_consent >= 3: Broad Rain stands close to the outer arches, plates clear enough for Pera to read welcome beneath the departure bands.}
{family_readiness >= 3: Doro has stopped watching Pera and begun watching the two young for instructions.}
{child_voice >= 3: Tavi tests the stop signal once more. Doro stops once more. The ritual acquires teeth.}
{plate_calm >= 3: Low Ember's plates hold steady copper warmth with no fear flare beneath it.}
{handover_witness >= 2: The discarded parasites and clean comb sit openly on the outer work shelf, evidence of care rather than a claim to ownership.}
{escort_prepared >= 2: One amber candle remains open on the possible shared first-leg route.}
{route_time <= 2: Returning light has begun to edge Umbros. The grazing road will not remain equally useful for long.}

Then Broad Rain gives the herd's departure pulse.

-> departure_pressure

=== departure_pressure ===
Blue runs down Broad Rain's dorsal plates, turns amber at the flanks, and passes through the adults waiting behind. The glassbacks begin to align with the calving lane.

Doro lifts the empty flank frame. "Tavi. Route time."

Tavi rises. Low Ember rises with them.

Doro crosses to the right root arch. Broad Rain aligns with the left. Tavi takes one step toward the family branch. Low Ember follows. Tavi stops. The calf presses a warm plate edge against Tavi's flank and darkens every readable surface.

The two adults have arrived to receive two young people into two different kinds of future. The young have discovered that grammar can be refused by standing in the wrong place.

The fungal road brightens its departure lane. It has freight opinions and no gift for tenderness.

-> departure_choice

=== departure_choice ===
// ghostlight.choice_layer: departure_pressure
+ [Let Tavi answer before any caretaker explains the answer for them.]
    // ghostlight.action_label: wait
    // ghostlight.branch_label: hear_child_choice
    ~ child_voice = child_voice + 2
    ~ brood_trust = brood_trust + 1
    ~ route_time = route_time - 1
    Pera folds low and waits.

    Tavi's facial fans open toward Low Ember, then toward Doro. "I am going. Low Ember is also going. Those are not the same sentence yet."

    Doro's throat patch warms with impatience. Broad Rain keeps the amber departure band moving.

    Pera says nothing. Silence is costing road light, but at least it belongs to the child who needs it.
    -> obligation_fold
+ [Set Doro's empty flank frame beside Low Ember and invite a practice loop through both outer arches.]
    // ghostlight.action_label: move_object
    // ghostlight.branch_label: stage_joint_departure
    ~ escort_prepared = escort_prepared + 2
    ~ family_readiness = family_readiness + 1
    ~ plate_calm = plate_calm + 1
    ~ route_time = route_time - 1
    Pera lowers the empty frame beside Low Ember instead of fastening it onto Doro.

    Tavi walks through the left arch. Low Ember follows. Doro takes the outside of the turn, where a family adult would guard a young traveler. Broad Rain watches from the calving lane.

    They return through the right arch as a temporary four-body circuit.

    The road keeps its amber candle open. Practice has not solved the departure. It has made one possible departure less imaginary.
    -> obligation_fold
+ [Offer Broad Rain the mineral comb and demonstrate Low Ember's privacy signal.]
    // ghostlight.action_label: transfer_object
    // ghostlight.branch_label: witness_herd_care
    ~ herd_consent = herd_consent + 2
    ~ handover_witness = handover_witness + 1
    ~ care_debt = care_debt + 1
    Pera carries the comb to the outer shelf and sets it within reach of Broad Rain's muzzle.

    Low Ember fogs the plate nearest Pera. Pera steps away. The calf clears it. Broad Rain lowers their head, takes the comb by its wrapped grip, and repeats the pause without touching.

    Amber warmth passes between the adult's plates and the calf's.

    The herd has received the care instruction. Pera has also publicly promised that the instruction matters.
    -> obligation_fold
+ [Mark the right family branch and left calving lane separately, and ask both young to rehearse a clean split.]
    // ghostlight.action_label: gesture
    // ghostlight.branch_label: rehearse_separation
    ~ separation_pressure = separation_pressure + 2
    ~ family_readiness = family_readiness + 1
    ~ route_time = route_time + 1
    ~ brood_trust = brood_trust - 1
    Pera traces the right family branch with one chest hand and the left calving lane with the other.

    Doro calls Tavi. Broad Rain pulses Low Ember.

    The young separate by three steps. Low Ember's plates flash yellow. Tavi's fans clamp shut. They both turn back before Pera ends the exercise.

    The attempt was orderly, quick, and wrong. This makes it attractive to adults with schedules.
    -> obligation_fold

=== obligation_fold ===
// ghostlight.fold: two_routes_one_kinship
The warm hollow holds its shape while the social geometry fails to cooperate.

Nursery ramp behind. Family branch through the right arch. Calving lane through the left. Tavi and Low Ember in the middle, where no map has bothered to draw kinship.

{child_voice >= 3: Tavi keeps both facial fans open. Their refusal is legible now, not merely inconvenient.}
{escort_prepared >= 2: Doro's frame and the open amber candle make a shared first leg physically possible.}
{herd_consent >= 4: Broad Rain lowers beside the outer arch instead of pulling the calf into the herd ring.}
{plate_calm >= 3: Low Ember's plates return from dark refusal to a slow copper pulse.}
{separation_pressure >= 2: Tavi and Low Ember stand pressed together, braced against the next clean adult idea.}
{care_debt >= 2: The comb in Broad Rain's custody makes Pera's promise visible: whichever route wins, care work follows.}
{route_time <= 1: Red-orange light is returning around Umbros. The fungal road is beginning to favor feeding traffic over ceremony.}

Doro looks at the open road. "Our family owes the northern shelters before next dark."

Broad Rain flashes a fast amber sequence through the herd: grazing window, calf safety, movement.

Pera has enough authority to shape the first leg. Not enough to erase its price.

-> obligation_choice

=== obligation_choice ===
// ghostlight.choice_layer: obligation_shape
+ {escort_prepared >= 2} [Ask family and herd to share the first leg to the honest-water hollow.]
    // ghostlight.action_label: propose
    // ghostlight.branch_label: accept_shared_route_debt
    ~ shared_route = 1
    ~ care_debt = care_debt + 2
    ~ herd_consent = herd_consent + 1
    ~ family_readiness = family_readiness + 1
    ~ route_time = route_time - 1
    Pera lays one route cord from Doro's frame to the amber candle and another beside Broad Rain's forefeet.

    "Together until honest water," Pera says. "The family guards the calf side. The herd sets the pace. The detour is ours."

    Doro calculates two missed shelters. Broad Rain shows one long blue band, then turns onto the shared line.

    Acceptance does not remove the debt. It chooses who will carry it.
    -> final_threshold
+ {child_voice >= 3} [Let Tavi choose the first goodbye and the body that waits.]
    // ghostlight.action_label: authorize
    // ghostlight.branch_label: give_child_departure_order
    ~ brood_trust = brood_trust + 1
    ~ separation_pressure = separation_pressure - 1
    ~ plate_calm = plate_calm + 1
    ~ route_time = route_time - 1
    Tavi touches Low Ember's warm plate edge with one chest hand.

    "Herd first," they say. "Doro waits here. I walk Low Ember to Broad Rain. I come back when the plate clears."

    Doro starts to object, then reads Tavi's open fans and does the more difficult family work.

    Doro waits.
    -> final_threshold
+ {handover_witness >= 2} [Split adult labor: Doro receives Tavi while Pera escorts Low Ember into the herd ring.]
    // ghostlight.action_label: divide_labor
    // ghostlight.branch_label: extend_caretaker_watch
    ~ handover_witness = handover_witness + 1
    ~ family_readiness = family_readiness + 1
    ~ care_debt = care_debt + 1
    ~ route_time = route_time - 1
    Pera passes the archive and rest cloths to Doro, then takes the mineral comb back from Broad Rain.

    "You receive Tavi," Pera says. "I finish Low Ember's return."

    Doro looks at Pera's silvered muzzle. "Your watch ended."

    "My schedule ended. The calf has not signed it."

    Broad Rain leaves a space at the edge of the herd ring.
    -> final_threshold
+ [Ask the lantern trees and fungal road to witness an immediate species-route separation.]
    // ghostlight.action_label: gesture
    // ghostlight.branch_label: formalize_clean_split
    ~ separation_pressure = separation_pressure + 1
    ~ handover_witness = handover_witness + 1
    ~ route_time = route_time + 1
    Pera opens both facial fans to the lantern canopy, then marks the right family branch and left calving lane with separate chest hands.

    Cold blue knots illuminate the two routes. Amber fungal candles sharpen the outer lane. The geometry is clear enough to satisfy anyone who is not standing in the middle of it.

    Tavi looks at Low Ember. Low Ember fogs the nearest plate.

    A formal witness can prove what the adults attempted. It cannot make the attempt kind.
    -> final_threshold

=== final_threshold ===
// ghostlight.fold: first_route_threshold
Totality loosens. Red-orange light rims the enormous fixed disk of Umbros. The lantern knots remain blue, but the fungal candles have begun to lean toward the road's next work.

Pera stands at the center of the warm hollow. Doro waits near the right family arch. Broad Rain waits near the left calving arch. Neither adult blocks the young; the open space is deliberate.

{shared_route == 1: One route cord and one amber candle join family and herd toward the honest-water hollow.}
{shared_route == 0: The right family branch and left calving lane remain two separate promises.}
{brood_trust >= 4: Tavi and Low Ember can stand apart without losing sight of one another.}
{brood_trust <= 1: Both young crowd the center, reading separation as abandonment.}
{family_readiness >= 3: Doro carries the archive openly and keeps their hands clear for Tavi's signals.}
{herd_consent >= 4: Broad Rain holds a calm blue welcome band beneath the amber need to move.}
{plate_calm >= 3: Low Ember's copper plate glow is steady enough to read around the returning light.}
{child_voice >= 3: Tavi's facial fans remain open toward both routes.}
{separation_pressure >= 3: Every adult movement makes the two young brace closer together.}
{care_debt >= 3: Doro's frame bears an added calving-lane cord: the family will owe time after this departure.}
{handover_witness >= 3: Comb, archive, and witnessed signals show that care has transferred through practice as well as record.}
{route_time <= 0: The shared light window is nearly spent. Whatever leaves together will arrive late somewhere else.}

The brood ring is ending in one place. Pera must decide whether it survives as route, escort, divided labor, or trust.

-> final_choice

=== final_choice ===
// ghostlight.choice_layer: first_route_commit
+ [Send family and herd together for one shared first leg.]
    // ghostlight.action_label: authorize
    // ghostlight.branch_label: commit_shared_first_leg
    {shared_route == 1 && family_readiness >= 3 && herd_consent >= 3:
        Pera lifts the joining route cord and lays it across Doro's frame.
        -> ending_shared_route_success
    - else:
        Pera calls both parties onto a route that has not been made ready.
        -> ending_shared_route_cost
    }
+ [Let Tavi escort Low Ember to Broad Rain, then return to Doro by choice.]
    // ghostlight.action_label: move
    // ghostlight.branch_label: commit_child_escort
    {child_voice >= 3 && plate_calm >= 3 && route_time >= 1:
        Pera opens the outer arch and asks every adult to stay where they are.
        -> ending_child_escort_success
    - else:
        Pera offers Tavi an escort ritual after time or calm has already thinned.
        -> ending_child_escort_cost
    }
+ [Let Doro leave with Tavi while Pera extends the watch and walks Low Ember outward.]
    // ghostlight.action_label: divide_labor
    // ghostlight.branch_label: commit_split_adult_care
    {handover_witness >= 3 && family_readiness >= 2 && separation_pressure <= 2:
        Pera takes the comb. Doro takes the archive.
        -> ending_split_care_success
    - else:
        Pera divides the work before either receiving relationship can carry it.
        -> ending_split_care_cost
    }
+ [Separate the young at the center and trust the witnessed care to survive distance.]
    // ghostlight.action_label: separate
    // ghostlight.branch_label: commit_clean_separation
    {brood_trust >= 4 && child_voice >= 2 && plate_calm >= 3 && separation_pressure <= 2:
        Pera gives the paired rest call once, then ends it with the departure note.
        -> ending_clean_separation_success
    - else:
        Pera gives the departure note before the shared care has become portable.
        -> ending_clean_separation_cost
    }

=== ending_shared_route_success ===
// ghostlight.ending_label: shared_first_leg_success
// ghostlight.training_hook: kinship_becomes_route_debt
The fungal road opens amber toward honest water.

Broad Rain sets the pace. Low Ember follows inside the herd's warm flank. Tavi walks beside the calf on four quick running legs. Doro takes the exposed outer line with chest hands free and the added calving-lane cord visible on the frame.

Doro and Tavi will miss two shelters. The herd will graze late. Pera records both costs because affection that hides its bill becomes someone else's labor.

At the first bend, Tavi looks back. Pera gives the rest call. Low Ember answers with a copper plate pulse that moves through the herd.

The brood ring leaves the hollow as a road with more than one kind of body in it.
-> END

=== ending_shared_route_cost ===
// ghostlight.ending_label: shared_first_leg_cost
// ghostlight.training_hook: shared_route_without_consent_is_congestion
Pera calls the shared departure.

The herd takes the left calving lane. Doro and Tavi take the right family branch. Where both branches reach the first bend, Low Ember stops before the joining candle, plates dark. Tavi stops beside them. The adults behind Broad Rain compress into a hot, anxious line while Doro is stranded on the other side of the join.

The fungal road extinguishes the joining candle. It has interpreted improvised kinship as congestion, which is rude and not entirely wrong.

The grazing window narrows. Doro's family misses its departure. Nobody has separated, but neither group can move. Togetherness without a workable route has become a very warm blockade.
-> END

=== ending_child_escort_success ===
// ghostlight.ending_label: child_escort_success
// ghostlight.training_hook: young_person_controls_separation_order
Tavi walks Low Ember through the outer arch.

Broad Rain lowers until the calf's nearest plate touches the adult's. Copper warmth passes into blue welcome. Tavi waits. Low Ember fogs the plate beside Tavi, then clears it and steps fully into the herd ring.

Tavi turns before any adult calls.

Doro is still waiting by the right root arch. The empty frame remains empty. Tavi crosses the hollow, gives Pera one fierce press of facial fan to cheek, and takes the family branch on their own legs.

Two departures happen. Neither becomes abandonment.
-> END

=== ending_child_escort_cost ===
// ghostlight.ending_label: child_escort_cost
// ghostlight.training_hook: agency_ritual_still_spends_time_and_calm
Tavi leads Low Ember outward as returning light changes the plate colors.

The calf's copper calm flares yellow under the root arch. Tavi cannot read whether it means fear, glare, or both. Broad Rain advances. Doro calls from behind as the family road begins to dim.

Tavi freezes between calls.

Pera retrieves both young into the warm hollow. The escort is not punishment, but it has failed. Doro's unit leaves late, Broad Rain's herd grazes late, and Tavi learns that being asked is not the same as being given enough conditions to succeed.
-> END

=== ending_split_care_success ===
// ghostlight.ending_label: split_adult_care_success
// ghostlight.training_hook: caretaker_handover_divides_labor_without_dividing_kinship
Doro receives Tavi at the right root arch with the archive open and both chest hands still.

Pera walks beside Low Ember through the outer arch. Broad Rain accepts the calf after repeating the fogged-plate pause. Pera stays until the calf settles inside the moving heat line.

By then Doro and Tavi are small shapes on the family road.

Pera has served one watch too many and will sleep through the evening meal. The breeding ground adds the missed sleep to the family's next care debt. The young leave by separate routes, each with an adult who knows how to listen to that body.
-> END

=== ending_split_care_cost ===
// ghostlight.ending_label: split_adult_care_cost
// ghostlight.training_hook: records_cannot_replace_received_care
Pera puts the archive in Doro's chest hands and turns toward Low Ember.

Tavi gives the fast stop signal. Doro misses it and reaches for the travel binding. Low Ember sees Tavi recoil and flashes yellow through every plate. Broad Rain closes the herd into a calf ring at the outer arch.

Pera now has one body and two failed handovers on opposite sides of the hollow.

The archive contains every correct instruction. Doro received the object. Nobody received the care.
-> END

=== ending_clean_separation_success ===
// ghostlight.ending_label: clean_separation_success
// ghostlight.training_hook: belonging_survives_eventual_separation
Pera sounds the paired rest call. Tavi and Low Ember settle one last time in the mineral sand.

Then comes the departure note.

Tavi rises toward Doro. Low Ember rises toward Broad Rain. Halfway apart, the calf fogs the plate facing Tavi. Tavi closes one facial fan. Privacy answered by privacy. The plate clears. The fan opens.

Family and herd leave through different outer arches.

The ring has not kept the bodies together. It has taught each body a refusal the other will still understand when they meet years later.
-> END

=== ending_clean_separation_cost ===
// ghostlight.ending_label: forced_separation_cost
// ghostlight.training_hook: care_without_portable_trust_becomes_custody
Pera gives the departure note.

Doro draws Tavi toward the right family arch with body position. Broad Rain presses Low Ember toward the left calving lane. The young separate because every adult around them becomes a wall.

Low Ember's plates flash panic into the herd. Tavi clamps both facial fans and refuses the family rest call. Amber candles darken first at Doro's feet, then along the route the family meant to take.

The adults meet their schedules for six heartbeats.

After that they inherit a frightened calf, a child who no longer trusts the call, and a road that has recorded the handover as broken care.
-> END
