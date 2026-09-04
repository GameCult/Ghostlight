// ghostlight.artifact_id: tangle_lower_lantern_shelf_branch_fold_v0
// ghostlight.fixture_id: tangle-lower-lantern-shelf-v0
// ghostlight.scene_id: tangle-lower-lantern-shelf-v0.birth-route-compact
// ghostlight.final_ink_path: examples/ink/zyphos/tangle-lower-lantern-shelf-v0.branch-and-fold.v0.ink

VAR roost_cohesion = 2
VAR route_reputation = 2
VAR nest_safety = 2
VAR disease_evidence = 1
VAR lantern_consent = 1
VAR fungal_credit = 2
VAR gamete_viability = 3
VAR public_witness = 0
VAR matriarch_pressure = 2
VAR embargo_scope = 0
VAR rival_access = 1

-> start

=== start ===
The Lower Lantern Shelf keeps three things warm before eclipse: threadwing eggs, Airawa gestation packets, and the local Matriarch's opinion of itself.

The shelf is a crescent terrace grown between enormous buttress roots. Three roost hollows open in the high rootward wall. A resin-topped contract table and its shallow gamete cradle occupy the center. Along the downhill edge, amber fruiting beads mark a candle fungal road beneath a row of lantern trees. East, an open flight gap drops toward the rival Lowwater grove. West, a narrower gap crosses broken ridges toward less hospitable routes.

Reed-in-Crosswind circles once through both gaps before landing. Reed is a threadwing courier: a small flying animal whose ribbonlike sensory vanes read pressure, heat, static, chemistry, and the memory traces clinging to cargo. The vanes make a soft paper sound in clean air and a rude one in bad contracts.

-> routine_people

=== routine_people ===
Tavi Split-Resin braces against the contract table with clawed upper limbs while the four soft digits of each lower hand sort sealed gamete membranes in the cradle. Tavi is Airawa: six-limbed, fine-scaled, long-legged, made to anchor to roots and do delicate work at the same time. Blue resin bands on Tavi's harness declare service to the Shelf Matriarch, the three courier roosts, and the fungal road. In that order if the tree is listening. In another order if anyone asks.

Lowwater-Blue is due through the eastern gap with a reciprocal gestation packet from the rival grove. Until then, routine is route work: test the nectar, inspect the cradle, feed the road, preen the nursery vanes, and pretend every payment was freely negotiated by everyone who needed it.

Tavi nudges a salt comb onto the table. "Ceremonial ration."

Reed tastes it.

"Officially ceremonial," Tavi corrects.

-> routine_hub

=== routine_hub ===
// ghostlight.choice_layer: routine_compact_work
+ [Preen frayed vane fibers from the nursery roost and bind them into one shared nest marker.]
    // ghostlight.action_label: use_body
    // ghostlight.branch_label: prime_roost_cohesion
    ~ roost_cohesion = roost_cohesion + 2
    ~ nest_safety = nest_safety + 1
    ~ gamete_viability = gamete_viability - 1
    Reed grips the root lip and combs loose sensory ribbons from two juveniles with careful vane strokes. The shed fibers go around all three nest mouths, one marker instead of three claims.

    The work costs time. One gamete membrane in the cradle pales at the edge.

    Tavi turns it toward the cooler resin. "Solidarity always invoices the perishable goods."
    -> routine_fold
+ [Taste the Matriarch's nectar well and compare it with the sealed gestation packets.]
    // ghostlight.action_label: inspect_object
    // ghostlight.branch_label: prime_disease_evidence
    ~ disease_evidence = disease_evidence + 2
    ~ matriarch_pressure = matriarch_pressure + 1
    Reed dips one narrow vane into the nectar well, then fans it above the cradle without touching the sealed membranes.

    The nectar carries warm-root permission, nest shelter, and a fresh bitter editing trace. The gamete packets carry none of that bitterness.

    A pulse travels under Reed's feet. The Matriarch has noticed the comparison.
    -> routine_fold
+ [Drop mineral grit onto the candle road and wait for its disease report.]
    // ghostlight.action_label: transfer_object
    // ghostlight.branch_label: prime_fungal_credit
    ~ fungal_credit = fungal_credit + 2
    ~ disease_evidence = disease_evidence + 1
    Reed lifts a salt grain from the comb, crosses the center table, and drops it between two amber fungal beads.

    The road absorbs payment first. Principles are easier with the invoice settled.

    A clean line brightens east toward Lowwater. No quarantine ring answers it. The road has heard of strain, hunger, and one cracked bridge. It has not heard of plague.
    -> routine_fold
+ [Fly the lantern arcade, carrying pollen from the western trees to the eastern flight gap.]
    // ghostlight.action_label: move
    // ghostlight.branch_label: prime_lantern_consent
    ~ lantern_consent = lantern_consent + 2
    ~ route_reputation = route_reputation + 1
    Reed launches west, crosses the blue-white lantern knots, and returns east with pollen dust held in the outer vanes.

    The trees answer by lighting the Lowwater gap in sequence. Route open. Service witnessed.

    Tavi marks the compact with a lower hand. "The lights have voted before the tree. Bold."
    -> routine_fold

=== routine_fold ===
// ghostlight.fold: routine_compact_before_pressure
Eclipse shadow climbs the lower road. The shelf settles into its ordinary exchange: root warmth for nests, courier flight for births, pollen for route light, minerals for testimony.

{roost_cohesion >= 4: The three roost mouths share one ring of shed fibers. Juveniles cross between them without asking which hollow owns the warmth.}
{nest_safety >= 3: The high root pockets flex open, holding steady heat around the egg clusters.}
{disease_evidence >= 3: Reed's vanes keep returning to the bitter trace in the Matriarch's nectar and the clean chemistry of the sealed cargo.}
{fungal_credit >= 4: A second amber fungal line brightens east, paid attention beside the main road.}
{lantern_consent >= 3: Blue-white knots preserve an illuminated corridor through the eastern flight gap.}
{route_reputation >= 3: Couriers on the high roots angle their vanes toward Reed, acknowledging a route kept legible.}

Then the Shelf Matriarch lifts a violet closure braid through a slit in the contract table.

-> closure_arrives

=== closure_arrives ===
// ghostlight.training_hook: lineage_embargo_disguised_as_quarantine
Tavi does not touch the braid. Its resin fibers carry the Matriarch's root scent, a red refusal trace, and the formal shape of a disease closure.

The eastern lanterns are still lit when Lowwater-Blue enters the gap. The rival courier is smaller than Reed, its sensory vanes stained blue by Lowwater resin. A viable gamete packet hangs in a sling beneath its body. It lands on the eastern rail, outside the center cradle, because couriers who survive politics learn exactly where hospitality stops.

The contract table pulses under Tavi's taloned feet. Close the route. Refuse the cargo. Name Lowwater contaminated.

{disease_evidence >= 3: Reed reads the missing fact immediately: the closure braid contains lineage refusal, but no credible wound scent, immune alarm, or road-borne sickness memory.}
{disease_evidence < 3: The braid tastes official and incomplete. Reed cannot yet tell whether the gap is corruption, urgency, or a lie that expects wings.}

Lowwater-Blue opens its vanes just enough to show the packet's clean seal. "Carry truth," its route song says. "Or carry ownership."

-> first_pressure_hub

=== first_pressure_hub ===
// ghostlight.choice_layer: closure_custody
+ [Lift the closure braid and carry its exact chemistry around all three roost mouths.]
    // ghostlight.action_label: carry_object
    // ghostlight.branch_label: publish_closure_terms
    ~ public_witness = public_witness + 2
    ~ route_reputation = route_reputation + 1
    ~ matriarch_pressure = matriarch_pressure + 1
    ~ embargo_scope = 1
    Reed takes the braid in gripping feet, not mouth, leaving the vanes free to broadcast what it contains.

    One circuit. Three roosts. Every courier receives both layers: public disease closure and private lineage refusal.

    The root wall tightens around the warm hollows. The Matriarch also knows how to publish a term.
    -> pressure_fold
+ [Land beside Lowwater-Blue and open every vane toward the clean packet seal.]
    // ghostlight.action_label: signal
    // ghostlight.branch_label: stand_with_lowwater
    ~ rival_access = rival_access + 2
    ~ public_witness = public_witness + 1
    ~ nest_safety = nest_safety - 1
    ~ matriarch_pressure = matriarch_pressure + 2
    Reed lands on the eastern rail beside Lowwater-Blue. Two sets of ribbon vanes open toward the packet, making its clean chemistry public to the terrace.

    The nearest roost hollow cools. Eggs press closer together in the root pocket.

    Tavi looks up at the contracting wood. "The tree has entered the debate by repossessing the weather."
    -> pressure_fold
+ {lantern_consent >= 3} [Carry the closure braid through the lantern arcade and ask the route lights to witness it.]
    // ghostlight.action_label: move
    // ghostlight.branch_label: seek_lantern_witness
    ~ lantern_consent = lantern_consent - 1
    ~ public_witness = public_witness + 1
    ~ embargo_scope = 1
    ~ gamete_viability = gamete_viability - 1
    Reed flies the crescent of lantern trees with the braid trailing below. Blue-white knots touch the red refusal fibers one by one.

    The lanterns keep the eastern gap lit but darken the path to the gamete cradle. They will witness a narrow cargo hold. They will not yet help starve a road.

    The detour costs another measure of warmth in the waiting packet.
    -> pressure_fold
+ {fungal_credit >= 3} [Set the braid on the paid fungal line and ask the road whether its sickness claim has a body.]
    // ghostlight.action_label: place_object
    // ghostlight.branch_label: seek_road_diagnosis
    ~ fungal_credit = fungal_credit - 1
    ~ disease_evidence = disease_evidence + 2
    ~ gamete_viability = gamete_viability - 1
    ~ public_witness = public_witness + 1
    Reed lowers the braid between the paid amber candles.

    The fungal road tastes root resin, courier oils, and the red refusal trace. It raises no quarantine ring. Instead, one fruiting bead opens toward the clean packet and another toward Tavi's blue contract bands.

    Witness the cargo. Witness the witness.
    -> pressure_fold

=== pressure_fold ===
// ghostlight.fold: closure_becomes_ecological_pressure
The Matriarch does not repeat the request. It changes the price around it.

The nectar wells draw shut. Warm air leaves one roost hollow. Fine roots rise around the center cradle, ready to make a temporary hold look like permanent law.

{public_witness >= 2: Couriers line the three roost mouths. The closure can no longer pretend it was a private misunderstanding.}
{public_witness <= 0: Only Tavi, Reed, Lowwater-Blue, and the Matriarch know the bargain has sharpened. That gives every later account room to become useful.}
{rival_access >= 3: Lowwater-Blue remains on the eastern rail with vanes open, treated as a courier under protection rather than contaminated cargo.}
{nest_safety <= 1: The egg clusters in the cooling hollow press together while nest keepers stare at Reed.}
{matriarch_pressure >= 5: Bark membranes begin sealing the eastern roost mouth. The tree is converting shelter into a vote.}
{embargo_scope >= 1: The route lights distinguish cargo from traffic for now: dark at the cradle, bright at the eastern gap.}

Tavi lays both lower hands flat on the resin table. "A full closure takes births, pollen, and warnings together. If we are doing that, let us at least avoid calling the knife medicinal."

-> coalition_hub

=== coalition_hub ===
// ghostlight.choice_layer: build_or_break_coalition
+ [Call the three roosts into one circling assembly above the terrace.]
    // ghostlight.action_label: speak
    // ghostlight.branch_label: rally_roost_assembly
    ~ roost_cohesion = roost_cohesion + 2
    ~ public_witness = public_witness + 1
    ~ nest_safety = nest_safety - 1
    ~ gamete_viability = gamete_viability - 1
    Reed launches with a low route call. Couriers leave all three hollows and circle beneath the Matriarch canopy, vanes exposing the violet braid's scent to every nest keeper and long-route flyer.

    The eggs lose body warmth while their keepers join the assembly. Solidarity has still not discovered free heat.
    -> compact_threshold
+ [Ask Tavi to braid a narrow quarantine: hold the gamete packet, keep food, pollen, and warnings moving.]
    // ghostlight.action_label: speak
    // ghostlight.branch_label: draft_narrow_quarantine
    ~ embargo_scope = 1
    ~ public_witness = public_witness + 1
    ~ route_reputation = route_reputation + 1
    ~ matriarch_pressure = matriarch_pressure + 1
    Tavi cuts a clean strand from the violet braid and loops it only around the shallow gamete cradle. The eastern gap, lantern arcade, and fungal road remain outside the knot.

    The Matriarch sends a hard pulse through the table. Tavi anchors with upper claws and keeps braiding with the lower hands.
    -> compact_threshold
+ {disease_evidence >= 3 && fungal_credit >= 2} [Move the sealed packet onto the road's diagnostic edge while Lowwater-Blue keeps custody.]
    // ghostlight.action_label: guide_movement
    // ghostlight.branch_label: diagnose_without_seizure
    ~ disease_evidence = disease_evidence + 1
    ~ fungal_credit = fungal_credit - 1
    ~ rival_access = rival_access + 1
    ~ gamete_viability = gamete_viability - 1
    Reed guides Lowwater-Blue from the eastern rail to the downhill edge. The rival courier sets the sling above the amber beads without releasing it.

    The road samples the seal. Clean candles rise beneath the packet. Custody remains with Lowwater. Diagnosis does not become confiscation merely because a root wants the paperwork simplified.
    -> compact_threshold
+ [Accept the full embargo in exchange for reopening every warm roost hollow.]
    // ghostlight.action_label: consent
    // ghostlight.branch_label: accept_full_embargo
    ~ embargo_scope = 3
    ~ nest_safety = nest_safety + 2
    ~ route_reputation = route_reputation - 2
    ~ rival_access = rival_access - 1
    ~ matriarch_pressure = matriarch_pressure - 1
    Reed folds every vane and lands on the Matriarch's violet braid.

    The root hollows open. Warmth returns around the eggs. The eastern lanterns go dark from cradle to flight gap, and the fungal road closes one amber line after another toward Lowwater.

    Lowwater-Blue does not plead. It records Reed's folded vanes in the fibers of its route song.
    -> compact_threshold

=== compact_threshold ===
// ghostlight.fold: final_settlement_threshold
Totality settles over the shelf. The lantern knots become the only cold light. Amber fungal candles mark the downhill edge. The high roosts hold eggs at whatever temperature politics has purchased.

Lowwater-Blue's packet remains viable only while its membranes keep their inner color.

{gamete_viability >= 2: The packet still glows pale green inside its translucent sling. There is time for a careful settlement.}
{gamete_viability <= 1: The packet has faded to gray-green at one edge. Delay has become a reproductive decision.}
{roost_cohesion >= 4: The three roosts hold a common circle under the canopy. The Matriarch can cool nests, but cannot bargain with each hollow separately.}
{route_reputation <= 1: Couriers at the western gap have turned their vanes away from Reed. Any decision now travels with damaged authority.}
{lantern_consent >= 3: The eastern flight gap remains blue-white even under totality.}
{fungal_credit >= 3: A paid amber diagnostic line waits below the cradle.}
{embargo_scope >= 3: Darkness now joins cradle, eastern gap, and Lowwater road into one visible full embargo.}
{embargo_scope == 1: Only the cradle is dark. Nourishment and warning routes remain visibly open.}
{matriarch_pressure >= 5: Fine roots have climbed the contract table legs. The Matriarch is close to making negotiation physically smaller.}

Reed must decide which alliance survives the night: roost with tree, courier with courier, gestator with rival, or the plural compact that makes each of them inconveniently dependent on the rest.

-> final_choice

=== final_choice ===
// ghostlight.choice_layer: birth_route_settlement
+ [Enforce the narrow cargo quarantine and keep nourishment, pollen, and warning traffic open.]
    // ghostlight.action_label: mixed
    // ghostlight.branch_label: settle_narrow_quarantine
    {embargo_scope <= 1 && disease_evidence >= 3 && public_witness >= 1 && gamete_viability >= 1:
        Reed hangs the violet strand only across the center cradle. Tavi marks it. Lowwater-Blue keeps the packet sling. Lantern light and fungal candles remain open east.
        -> ending_narrow_compact
    - else:
        Reed tries to make a narrow closure out of evidence nobody gathered or a packet already failing in the cold.
        -> ending_narrow_cost
    }
+ [Reject the false quarantine and withdraw the three courier roosts from the Shelf compact.]
    // ghostlight.action_label: withdraw
    // ghostlight.branch_label: found_courier_refusal
    {roost_cohesion >= 4 && route_reputation >= 3:
        Reed gives the departure sequence. The three roosts answer as one body and choose the western gap.
        -> ending_courier_compact
    - else:
        Reed calls a collective refusal with neither a collective nor enough reputation to make refusal travel intact.
        -> ending_courier_cost
    }
+ [Honor the full embargo for one nesting season and take the Matriarch's warmth.]
    // ghostlight.action_label: consent
    // ghostlight.branch_label: settle_full_embargo
    {embargo_scope >= 3 && nest_safety >= 4 && matriarch_pressure <= 4:
        Reed lands on the violet braid. The warm hollows open around the eggs while the eastern food web goes dark.
        -> ending_embargo_compact
    - else:
        Reed offers the embargo after the route has already learned what the promise costs, or before the tree has promised enough in return.
        -> ending_embargo_cost
    }
+ [Carry the viable packet through the eastern gap and let Lowwater seek another gestator.]
    // ghostlight.action_label: carry_object
    // ghostlight.branch_label: relay_lowwater_packet
    {rival_access >= 3 && gamete_viability >= 2 && lantern_consent >= 2:
        Lowwater-Blue transfers the packet sling to Reed for the first ridge crossing. The lantern arcade keeps the eastern gap visible.
        -> ending_rival_relay
    - else:
        Reed takes the packet without enough time, route consent, or shared custody to make the crossing safe.
        -> ending_rival_cost
    }

=== ending_narrow_compact ===
// ghostlight.ending_label: narrow_quarantine_success
// ghostlight.training_hook: plural_witness_keeps_ecological_channels_separate
The fungal road samples the packet seal while Lowwater-Blue retains custody. The lantern trees keep pollinators moving. Tavi records a temporary hold on one cargo, not a sentence against a lineage.

The Shelf Matriarch keeps its warm hollows. It loses the more valuable fiction that every closure it requests is an immune response.

At eclipse egress, the packet is still viable. The route is still open. Nobody has won enough to call the arrangement natural.
-> END

=== ending_narrow_cost ===
// ghostlight.ending_label: narrow_quarantine_cost
// ghostlight.training_hook: procedure_without_evidence_spends_viability
The narrow braid looks careful. The packet beneath it turns gray at the seam.

The road will not certify evidence it never received. The lantern trees illuminate an open route carrying nothing useful. The Matriarch calls the failed birth proof of contamination.

Reed learns the vicious elegance of procedure: a small delay can do the work of a large prohibition while every participant insists the route remained open.
-> END

=== ending_courier_compact ===
// ghostlight.ending_label: courier_refusal_success
// ghostlight.training_hook: coerced_roosts_reform_as_route_alliance
The three roosts leave through the western gap carrying eggs, shed fibers, route songs, and the Matriarch's closure chemistry.

Lantern pollination stops first. Nectar follows. The candle road begins redirecting disease reports around the Shelf before returning light reaches the terrace.

The Matriarch compelled one decision and taught every courier constituency why a separate alliance was necessary. By morning, the Shelf still has roots, archives, and an excellent view of the traffic going elsewhere.
-> END

=== ending_courier_cost ===
// ghostlight.ending_label: courier_refusal_cost
// ghostlight.training_hook: collective_refusal_without_cohesion_fractures_nests
One roost follows Reed west. One stays for warm hollows. One circles until the cold chooses for it.

Lowwater-Blue departs east with the fading packet. The Matriarch keeps enough couriers to claim the compact survived and enough enemies to make every future delivery expensive.

A strike called too early becomes a census of who can afford principle.
-> END

=== ending_embargo_compact ===
// ghostlight.ending_label: full_embargo_success
// ghostlight.training_hook: nest_security_purchased_with_rival_food_web
The eggs warm. The nectar wells reopen. Reed's roost receives salt and protected distance from climbing traffic.

East, lantern pollination stops. Lowwater's road loses sugar, then warning traffic, then confidence from grazers who cannot afford a route that goes dark for someone else's lineage quarrel.

The bargain holds for one season. That is success in the strict administrative sense and a wound in every other one.
-> END

=== ending_embargo_cost ===
// ghostlight.ending_label: full_embargo_cost
// ghostlight.training_hook: coerced_alliance_cannot_buy_back_route_reputation
Reed folds every vane. The eastern route closes.

The Matriarch reopens two roost hollows, not three. Warmth was conditional before the bargain and remains conditional after it. The couriers have spent Lowwater trust without purchasing secure nests.

The fungal road turns dark beneath the contract table. It has diagnosed the arrangement, if not the cargo.
-> END

=== ending_rival_relay ===
// ghostlight.ending_label: rival_relay_success
// ghostlight.training_hook: reproductive_cargo_escape_creates_counter_alliance
Reed and Lowwater-Blue cross the eastern gap in staggered flight, one carrying the packet, one carrying its custody song.

The lantern trees light the first descent. The fungal road keeps its warning line open. Behind them, the Shelf Matriarch cools every abandoned roost it can reach.

Another gestator may accept the packet. Another grove may not. What survives the shelf is smaller and more dangerous than victory: proof that reproductive traffic can leave with the couriers who make it real.
-> END

=== ending_rival_cost ===
// ghostlight.ending_label: rival_relay_cost
// ghostlight.training_hook: escape_route_without_time_or_consent_spends_future_births
The eastern gap is dark halfway down. Reed catches one cold updraft, then another that is not there.

The packet membrane fades against Reed's gripping feet. Lowwater-Blue's custody song becomes a mourning route before either courier reaches the first ridge.

The Matriarch did not need to stop the flight. It only needed to make delay, darkness, and hope add up to the same dead future.
-> END
