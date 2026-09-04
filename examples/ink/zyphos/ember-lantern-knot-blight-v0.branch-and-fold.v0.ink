// ghostlight.artifact_id: ember_lantern_knot_blight_v0_branch_fold_v0
// ghostlight.fixture_id: ember-lantern-knot-blight-v0
// ghostlight.scene_id: ember-lantern-knot-blight-v0.eclipse-saddle
// ghostlight.final_ink_path: examples/ink/zyphos/ember-lantern-knot-blight-v0.branch-and-fold.v0.ink

VAR tree_reserve = 3
VAR knot_integrity = 3
VAR archive_contamination = 1
VAR route_trust = 2
VAR road_credit = 2
VAR pollinator_loyalty = 2
VAR ant_testimony = 0
VAR calf_shelter = 2
VAR marsh_pressure = 1
VAR eclipse_time = 4
VAR isolated_sector = 0
VAR detour_ready = 0

-> start

=== start ===
You stand where a dry stone saddle gives way to a shallow prismwake marsh. This is literal. You are an Umbros-facing lantern tree, broad-rooted across the slope, with a trunk of layered memory cambium and cold light knots hanging beneath a low black-green canopy.

A candle fungal road crosses below your northern roots. Uphill, its amber beacons lead toward threadwing roosts. Downhill, they stop at the wetland edge, where flat prism cells are already folding against eclipse shadow. A root hollow on your southern side holds three glassback calves while the herd grazes the last bright sugar from the marsh.

Umbros waits fixed above the saddle, immense and patient. The sun is beginning to pass behind it.

-> routine_arrivals

=== routine_arrivals ===
A threadwing courier descends from the ridge. Long ribbonlike sensory vanes stream from its gliding body, tasting pressure, pollen, static charge, and the promises you made yesterday. It circles once because professionals check the signal before landing, even when the signal is a tree.

Below, the candle road opens amber fruiting beads in the sequence that means fed, clear, willing to discuss terms. The glassback calves press their translucent dorsal plates toward your stored warmth.

This is the ordinary work: light the route, shelter the young, feed the road, pay the courier. Everyone calls it cooperation after the invoices have been digested.

-> routine_choice

=== routine_choice ===
// ghostlight.choice_layer: routine_allocation
+ [Pulse a clean blue landing sequence and open nectar pores for the threadwing.]
    // ghostlight.action_label: speak
    // ghostlight.branch_label: prime_threadwing_route
    ~ pollinator_loyalty = pollinator_loyalty + 2
    ~ route_trust = route_trust + 1
    ~ tree_reserve = tree_reserve - 1
    Blue light travels from the ridgeward knots toward the trunk. The threadwing lands on a low fork, ribbon vanes spread clear of bark, and takes nectar without pretending the exchange was friendship.

    Its gut symbionts leave a clean pollen taste in your outer cambium. Your route memory accepts the report.
    -> routine_fold
+ [Push sugar into the candle-road braid beneath the northern roots.]
    // ghostlight.action_label: spend_resource
    // ghostlight.branch_label: prime_road_credit
    ~ road_credit = road_credit + 2
    ~ route_trust = route_trust + 1
    ~ tree_reserve = tree_reserve - 1
    Sugar sinks through borrowed root sheaths into the fungal braid. Amber candles open farther uphill and at the marsh edge.

    The road returns mineral taste, yesterday's foot pressure, and one admirably concise report about a corpse field. Logistics has no need to be tasteful when it can be accurate.
    -> routine_fold
+ [Lower the warm knot band over the glassback calves' root hollow.]
    // ghostlight.action_label: gesture
    // ghostlight.branch_label: prime_calf_shelter
    ~ calf_shelter = calf_shelter + 2
    ~ route_trust = route_trust + 1
    ~ eclipse_time = eclipse_time - 1
    You bend three light-bearing twigs toward the hollow. Cold blue light marks the safe lip while stored trunk warmth rises through the roots.

    The calves align flank to flank. Their clear dorsal plates exchange heat and a low gradient of relief. The adult herd sees the shelter signal from the marsh and keeps grazing instead of crowding the slope.
    -> routine_fold
+ [Dim the public knots and compare fresh pollen against the outer memory rings.]
    // ghostlight.action_label: wait
    // ghostlight.branch_label: prime_self_audit
    ~ knot_integrity = knot_integrity + 1
    ~ ant_testimony = ant_testimony + 1
    ~ route_trust = route_trust - 1
    ~ eclipse_time = eclipse_time - 1
    You close the landing pattern and press fresh pollen memory inward, ring against ring.

    A scouting strand of lattice ants notices the pause. Small jointed bodies assemble a temporary reading loop across one root scar, tasting pressure and immune residue while the courier circles in visible annoyance.
    -> routine_fold

=== routine_fold ===
// ghostlight.fold: routine_before_blight
Eclipse ingress darkens the saddle from uphill to marsh.

{pollinator_loyalty >= 4: The threadwing preens on your low fork, gut cargo settled and vanes bright with your route pattern.}
{road_credit >= 4: The road's amber beacons reach both exits, credit made into usable geometry.}
{calf_shelter >= 4: Three calves rest under the southern crown while the adult herd keeps its weight off your roots.}
{ant_testimony >= 1: A thin ant loop persists across the root scar, local history briefly given mandibles.}
{tree_reserve <= 2: Your deep sugars have begun moving outward. Generosity is now a finite organ.}
{route_trust <= 1: Traffic waits beyond the saddle while your private inspection spends public time.}

Then the east knot line lights blue for invitation and white for quarantine at the same time.

-> blight_reveal

=== blight_reveal ===
The contradiction travels inward through your cambium.

Fresh pollen arrives twice: once as the threadwing delivered it, once as an older passage memory wearing the same chemical face. The eastern knots repeat yesterday's landing sequence. Beneath them, tissue that should admit current light insists the eclipse has already passed.

The threadwing launches hard enough to shed a sensory fiber. The candle road closes its downhill candles. In the marsh, prismwake skin folds silver and tight. The glassback herd lifts its plates toward your remaining light and begins climbing.

This is knot blight: not yet a cause, only a part of your body giving unsafe directions with complete administrative confidence.

-> first_response_choice

=== first_response_choice ===
// ghostlight.choice_layer: first_blight_response
+ [Darken and isolate the entire eastern crown.]
    // ghostlight.action_label: gesture
    // ghostlight.branch_label: isolate_eastern_crown
    ~ isolated_sector = isolated_sector + 2
    ~ archive_contamination = archive_contamination - 1
    ~ knot_integrity = knot_integrity - 1
    ~ calf_shelter = calf_shelter - 1
    ~ route_trust = route_trust + 1
    You close every knot between the trunk and the marsh, including the ones still behaving.

    The false invitation dies. So does half the shelter geometry. The climbing glassbacks turn toward the southern hollow, and the threadwing gives your dark crown the wide berth reserved for honest danger.
    -> pressure_fold
+ [Spend stored sugar to hold one ridge-to-marsh emergency line.]
    // ghostlight.action_label: gesture
    // ghostlight.branch_label: hold_emergency_line
    ~ tree_reserve = tree_reserve - 1
    ~ route_trust = route_trust + 2
    ~ archive_contamination = archive_contamination + 2
    ~ knot_integrity = knot_integrity - 1
    You force four knots into a simple blue line: ridge, trunk, hollow, marsh. No invitation grammar. Only visible ground.

    The line keeps moving bodies from piling into your roots. It also keeps suspect cambium fed. The old passage signal brightens beneath the clean one, pleased to be mistaken for continuity.
    -> pressure_fold
+ [Feed the lattice ants and invite a diagnostic sheet across the cambium.]
    // ghostlight.action_label: spend_resource
    // ghostlight.branch_label: summon_lattice_ants
    ~ ant_testimony = ant_testimony + 2
    ~ tree_reserve = tree_reserve - 1
    ~ eclipse_time = eclipse_time - 1
    ~ knot_integrity = knot_integrity + 1
    Sugar beads rise from the root scar. Lattice ants arrive from three directions and lock bodies into a net across your eastern bark.

    Their microbial glue reads two incompatible histories. The colony marks the outer one with a broken loop. Useful. Also saleable. Somewhere nearby, another tree will soon know exactly how frightened you are.
    -> pressure_fold
+ [Shed the first lying knot cluster before the pattern reaches deeper rings.]
    // ghostlight.action_label: mixed
    // ghostlight.branch_label: shed_first_cluster
    ~ archive_contamination = archive_contamination - 1
    ~ knot_integrity = knot_integrity - 2
    ~ pollinator_loyalty = pollinator_loyalty - 1
    ~ marsh_pressure = marsh_pressure + 1
    You seal a branch joint and let the eastern cluster fall.

    It lands at the wetland edge in a spray of blue-white light. Recent pollen routes, nest permissions, and one season of insults go with it. The prismwake mat below flashes the impact back at you in colorless silver, billing to follow.
    -> pressure_fold

=== pressure_fold ===
// ghostlight.fold: neighbors_reposition
Totality approaches. The failed niche is already being divided.

{isolated_sector >= 2: Your eastern crown is a dark wall; safe as containment, useless as a road.}
{archive_contamination >= 3: The old invitation keeps moving under the clean signal, deeper than it was one choice ago.}
{archive_contamination <= 0: The repeated passage memory has retreated to the outermost rings, though its cause remains unknown.}
{ant_testimony >= 2: Ant bodies spell a broken loop across your bark: the fault is locally bounded enough to cut.}
{knot_integrity <= 1: Too many eastern knots are dark, damaged, or gone to carry a complete route sequence.}
{pollinator_loyalty <= 1: The threadwing circles above the ridge exit and begins rehearsing somebody else's route.}
{marsh_pressure >= 2: Displaced feet and fallen tissue make the closed prismwake surface ripple with synchronized warning flashes.}

The candle road opens one amber bead beneath your trunk. Payment requested. The glassback herd reaches the northern root braid. The threadwing holds above the western fork, waiting to learn whether your warning deserves carriage.

-> neighbor_choice

=== neighbor_choice ===
// ghostlight.choice_layer: neighbor_bargain
+ {road_credit >= 3} [Spend road credit and another root-sugar pulse on an uphill detour.]
    // ghostlight.action_label: spend_resource
    // ghostlight.branch_label: buy_road_detour
    ~ detour_ready = detour_ready + 2
    ~ road_credit = road_credit - 2
    ~ tree_reserve = tree_reserve - 1
    ~ marsh_pressure = marsh_pressure - 1
    The road takes payment first. Then amber beads climb the dry ridge around your dark crown, sparse but continuous.

    Traffic follows the new curve. The road has saved your roots, protected the marsh, and demonstrated that your former route can be replaced. All three facts are true. Only one is kind.
    -> final_threshold
+ {pollinator_loyalty >= 3} [Give the threadwing a plain white warning pulse and release it toward neighboring groves.]
    // ghostlight.action_label: speak
    // ghostlight.branch_label: send_threadwing_warning
    ~ pollinator_loyalty = pollinator_loyalty - 1
    ~ route_trust = route_trust + 2
    ~ archive_contamination = archive_contamination - 1
    ~ eclipse_time = eclipse_time - 1
    You strip the message of invitation, tax, and dignity: unsafe light, eastern crown, carry no pollen through.

    The threadwing lands long enough to taste the warning, takes no nectar, and launches west. Its vanes hold your failure in a pattern neighboring groves can use. They will also remember who needed warning.
    -> final_threshold
+ [Open the southern root hollow to the calves and turn the adult herd back with white light.]
    // ghostlight.action_label: gesture
    // ghostlight.branch_label: protect_calves_redirect_herd
    ~ calf_shelter = calf_shelter + 2
    ~ route_trust = route_trust + 1
    ~ knot_integrity = knot_integrity - 1
    ~ marsh_pressure = marsh_pressure + 1
    Blue knots lower over the calves. A hard white band faces the adults.

    The herd obeys because the calves are inside your leverage. Adult plates darken with offense, then turn downhill. Their weight leaves your roots and returns to the closed marsh edge, where the mats begin preparing a collective refusal.
    -> final_threshold
+ [Starve the candle-road braid until it drains the suspect moisture from your roots.]
    // ghostlight.action_label: withhold_object
    // ghostlight.branch_label: starve_root_braid
    ~ archive_contamination = archive_contamination - 1
    ~ road_credit = road_credit - 2
    ~ route_trust = route_trust - 1
    ~ knot_integrity = knot_integrity + 1
    ~ detour_ready = detour_ready - 1
    You close sugar pores around the northern braid.

    The road retracts wet filaments from your root sheaths, taking suspect traffic with them. Amber beacons go out at both exits. The road has obeyed the pressure gradient and recorded the insult with equal professionalism.
    -> final_threshold

=== final_threshold ===
// ghostlight.fold: totality_decision
Umbros covers the sun.

Cold knots, amber candles, glassback plates, ant glue, and prismwake flashes become the saddle's only public light. Your body has one eclipse-hour to decide what kind of absence it can survive.

{tree_reserve <= 1: Your outer tissues are spending tomorrow's growth to keep tonight's decisions visible.}
{tree_reserve >= 3: Deep sugar remains available for one costly act.}
{detour_ready >= 2: An amber route now curves around the grove on the dry ridge.}
{road_credit <= 0: The fungal braid has closed both exits and begun moving its attention elsewhere.}
{route_trust >= 4: Courier, herd, and road behavior all show that your warning still means something.}
{route_trust <= 1: Every neighbor is treating your light as an interested claim rather than guidance.}
{calf_shelter >= 4: The calves are compressed safely beneath the southern crown, clear of the suspect eastern knots.}
{calf_shelter <= 1: The root hollow has become too dark or exposed to hold young bodies through totality.}
{marsh_pressure >= 3: The wetland edge has slickened and soured; another step will turn displacement into a second closure.}
{eclipse_time <= 1: The last useful response window is closing before returning light rewrites every visible signal.}

The eastern crown pulses blue-white once more.

It could be a plea from healthy tissue trapped inside your quarantine.

It could be yesterday asking to enter again.

-> final_choice

=== final_choice ===
// ghostlight.choice_layer: grove_survival_decision
+ [Shed the blighted eastern limb along the ants' broken-loop boundary.]
    // ghostlight.action_label: mixed
    // ghostlight.branch_label: sever_eastern_limb
    {ant_testimony >= 2 && isolated_sector >= 2 && knot_integrity >= 1:
        You seal the boundary the ants marked and release the whole eastern limb.
        -> ending_sever_success
    - else:
        You cut where fear suggests because the evidence does not hold still.
        -> ending_sever_cost
    }
+ [Yield the route-light niche to the candle-road detour and keep the grove dark.]
    // ghostlight.action_label: gesture
    // ghostlight.branch_label: yield_to_detour
    {detour_ready >= 2 && road_credit >= 1 && marsh_pressure <= 2:
        You pulse one white closure and let the amber detour become the public route.
        -> ending_detour_success
    - else:
        You withdraw before another body has a safe path ready.
        -> ending_detour_cost
    }
+ [Hold one emergency lane lit through totality.]
    // ghostlight.action_label: gesture
    // ghostlight.branch_label: preserve_emergency_lane
    {tree_reserve >= 2 && archive_contamination <= 2 && route_trust >= 3 && eclipse_time >= 1:
        You feed four knots and no others: ridge, trunk, hollow, marsh.
        -> ending_lane_success
    - else:
        You light a promise your body cannot safely keep.
        -> ending_lane_cost
    }
+ [Close the whole grove and trust courier warning plus sheltered bodies to carry the interval.]
    // ghostlight.action_label: gesture
    // ghostlight.branch_label: close_entire_grove
    {pollinator_loyalty >= 2 && calf_shelter >= 3 && route_trust >= 3:
        You answer the lying pulse with complete darkness.
        -> ending_closure_success
    - else:
        You close before enough neighbors can survive what closure exports.
        -> ending_closure_cost
    }

=== ending_sever_success ===
// ghostlight.ending_label: bounded_severance_success
// ghostlight.training_hook: memory_loss_as_containment_cost
The ant boundary holds.

The eastern limb falls clear of the root hollow and stops pulsing when its stored sugars empty. You lose recent pollen routes, nest permissions, and a season of passage memory. The deep rings remain your own.

The threadwing lands on the surviving western fork. The road opens one cautious amber bead. In the marsh, prismwake warnings subside by degrees rather than forgiveness.

By returning light, you are a smaller archive with an honest dark side. The neighbors remain. They also know what you were willing to forget.
-> END

=== ending_sever_cost ===
// ghostlight.ending_label: blind_severance_cost
// ghostlight.training_hook: containment_without_evidence
The limb breaks through healthy cambium.

Clean knots fall with the lying ones. The glassback herd bolts from the crack. The threadwing carries a warning that now means injury as well as blight. Ants salvage readable fibers from the cut and sell the history uphill.

The repeated invitation continues in one root sheath below the wound.

You have paid in memory and kept the uncertainty.
-> END

=== ending_detour_success ===
// ghostlight.ending_label: niche_yield_success
// ghostlight.training_hook: authority_moves_with_safe_infrastructure
The grove goes dark. The road stays visible.

Amber beads carry traffic around your northern roots and above the closed marsh. Glassback adults follow the dry curve. The calves remain beneath your crown until returning light. The threadwing uses the detour once, which is how temporary arrangements begin applying for permanence.

You contain the blight by surrendering route authority for the night. At dawn, the road will ask what the niche is worth now that it has learned to live without you.
-> END

=== ending_detour_cost ===
// ghostlight.ending_label: premature_yield_cost
// ghostlight.training_hook: closure_exports_unrouted_bodies
Your knots close. No complete amber line replaces them.

The herd splits at the northern root braid. Half climbs blind; half presses downhill onto prismwake tissue that turns slick under their feet. The threadwing leaves without pollen or warning. Candle beads appear farther upslope, where another grove can pay.

Your quarantine contains the eastern crown and collapses the saddle around it.
-> END

=== ending_lane_success ===
// ghostlight.ending_label: emergency_lane_success
// ghostlight.training_hook: bounded_continuity_under_low_energy
Four knots burn cold and plain.

The threadwing crosses without landing. Glassback adults pass one at a time while the calves stay in the hollow. The road keeps its moisture outside your isolated root sheath. Prismwake mats receive no new trampling.

The lane costs a hard measure of stored sugar, but its grammar does not lie. When returning light reaches the saddle, every neighbor knows exactly what remained open and why.
-> END

=== ending_lane_cost ===
// ghostlight.ending_label: false_continuity_cost
// ghostlight.training_hook: unsafe_signal_as_contagion_route
The emergency line brightens. The old invitation brightens inside it.

The threadwing lands where yesterday told it to land. Its vanes brush suspect cambium. The road accepts falling pollen into wet fruiting tissue. Glassbacks follow the visible line and crowd the eastern roots.

Traffic survives the eclipse. So does the fault, now equipped with couriers.
-> END

=== ending_closure_success ===
// ghostlight.ending_label: whole_grove_quarantine_success
// ghostlight.training_hook: relationship_capital_carries_absence
You close every knot.

The threadwing carries the plain warning west. The calves share stored heat beneath the southern crown. Adult glassbacks wait on dry stone because your earlier signals still have credit. The candle road holds its boundary instead of advertising a replacement.

For one eclipse, relationship does the work light cannot. Your darkness remains a trusted instruction, not an abdication.
-> END

=== ending_closure_cost ===
// ghostlight.ending_label: whole_grove_quarantine_cost
// ghostlight.training_hook: containment_exports_niche_failure
You close every knot.

The courier has no reason to carry your version. The root hollow cannot hold the calves. The road withdraws its amber edge, and the adult herd drives downhill into the marsh's synchronized refusal.

Neighboring trees light their own routes brighter. They take pollen, traffic, shelter debt, and the authority that follows.

The blight may be contained. The grove is not the center of this saddle anymore.
-> END
