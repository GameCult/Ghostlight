// ghostlight.artifact_id: tangle_pollen_tithe_branch_fold_v0
// ghostlight.fixture_id: tangle-pollen-tithe-v0
// ghostlight.scene_id: tangle-pollen-tithe-v0.shared-pollen-terrace
// ghostlight.final_ink_path: examples/ink/zyphos/tangle-pollen-tithe-v0.branch-and-fold.v0.ink

VAR threadwing_trust = 2
VAR road_credit = 2
VAR ant_testimony = 0
VAR matriarch_favor = 2
VAR rival_relief = 0
VAR salt_reserve = 3
VAR household_standing = 2
VAR route_light = 2

-> start

=== start ===
The shared pollen terrace occupies a broad shelf between two buttress roots of the local Matriarch. Its uphill rim belongs to an Umbros-facing lantern tree. Its downhill edge belongs to a candle fungal road, which emerges in amber beads from a root arch and forks toward two canopies that have disliked each other for longer than anyone with legs can remember.

Nobody owns the middle. This is why everyone has left furniture there.

Salt basins stand beneath woven threadwing roosts. A low assay slab interrupts the road fork, already busy with lattice ants. Contract ribbons hang from a public rack where any visitor can inspect who has promised food, light, grooming, road labor, archive access, or the sort of neutrality that lasts until breakfast.

-> routine_bodies

=== routine_bodies ===
The junior tithe-tender climbs down the local Matriarch's bark face with both taloned feet and both clawed upper hands anchored. The smaller lower hands remain free for salt spoons and ribbon knots. This is ordinary competence among Airawa. Falling while carrying a public account is considered both painful and editorial.

The elder contract-tender waits on the terrace, plate seams glowing faintly under a mantle braided with lantern fiber, fungal thread, and the empty loops of obligations wisely declined.

"Morning tithe," the elder says. "A little salt, a little rot, a little light, and several parties pretending this is hospitality."

The local Matriarch answers through a slow pressure in the buttress roots. The lantern tree opens three cold knots above the roosts. The candle road brightens one bead at a time toward the local canopy and, after a pause visible to everyone, one bead toward the rival fork.

-> routine_courier

=== routine_courier ===
A scar-vane threadwing circles the terrace before landing on the public rack. Its narrow gliding body is covered in overlapping sensory ribbons instead of feathers. One vane bears an old pale scar. It is the colony's regular broker here, which does not make it friendly. Familiarity is merely hostility with better records.

The broker taps the salt basin, the roost membrane, and the rival-facing ribbon in that order.

Three invoices. One beak.

-> opening_hub

=== opening_hub ===
// ghostlight.choice_layer: morning_tithe
+ [Pour one household measure of salt into the courier basin before the broker asks twice.]
    // ghostlight.action_label: transfer_resource
    // ghostlight.branch_label: prime_courier_credit
    ~ salt_reserve = salt_reserve - 1
    ~ threadwing_trust = threadwing_trust + 2
    ~ household_standing = household_standing + 1
    The junior tender braces the basin with one lower hand and pours with the other. The scar-vane broker tastes the crystals, then sheds a clean ribbon of fiber beside the household's contract marker.

    The elder nods. "Congratulations. We have purchased the right to be complained at promptly."
    -> routine_fold
+ [Lay clean fruit husks and shed tissue beside the candle road's local fork.]
    // ghostlight.action_label: transfer_resource
    // ghostlight.branch_label: prime_road_credit
    ~ road_credit = road_credit + 2
    ~ matriarch_favor = matriarch_favor + 1
    The offering goes outside the amber beads, where the road can choose it without admitting the tender's weight. White mycelial threads reach up, sort husk from tissue, and leave the poisonous bits untouched with bureaucratic precision.

    The local buttress warms beneath the tender's feet. Approval, or digestion conducted nearby. Context helps.
    -> routine_fold
+ [Retie the lowest roost membrane and ask the lantern tree for one more eclipse knot.]
    // ghostlight.action_label: repair_object
    // ghostlight.branch_label: prime_route_light
    ~ route_light = route_light + 2
    ~ threadwing_trust = threadwing_trust + 1
    Hanging from both feet and one upper hand, the junior tender draws the membrane taut with the lower pair. The lantern tree tastes the repaired fiber through a root contact and opens a fourth cold knot above it.

    The broker hops into the better light, which is how a route authority says thank you while preserving its negotiating position.
    -> routine_fold
+ [Ask that yesterday's rival-facing ribbon remain on the public rack until the ants have read it.]
    // ghostlight.action_label: speak
    // ghostlight.branch_label: prime_public_account
    ~ ant_testimony = ant_testimony + 1
    ~ household_standing = household_standing + 1
    ~ matriarch_favor = matriarch_favor - 1
    "Leave the red-edged fiber," the junior tender says. "A disputed account is still an account."

    The elder stops reaching for it. Lattice ants begin joining their bodies into a narrow loop around the fallen strand.

    The local root flexes once. The Matriarch has heard, and has not mistaken public accounting for affection.
    -> routine_fold

=== routine_fold ===
// ghostlight.fold: morning_tithe_accounted
Routine resumes around the choice.

The elder scrapes yesterday's fungal beads from the assay slab. The junior tender checks roost knots, salt weight, and the contract rack. The scar-vane broker preens one sensory ribbon while watching the four lower hands belonging to the two Airawa, because invoices sometimes develop opinions when unattended.

{threadwing_trust >= 4: The broker eats with both feet planted. The courier colony expects the terrace to honor a visible payment.}
{road_credit >= 4: The candle road opens a brighter local lane beneath the assay slab, rich enough to carry minerals before full eclipse.}
{ant_testimony >= 1: The ants preserve the rival-facing fiber inside a living loop instead of recycling it.}
{matriarch_favor <= 1: The local buttress keeps a cool patch beneath the junior tender's feet: a small archive of displeasure.}
{route_light >= 4: Four cold lantern knots make the roost membranes legible against the coming shadow.}

Then every threadwing above the terrace turns toward the rival fork.

-> grievance_arrival

=== grievance_arrival ===
The arriving flock does not use the local Matriarch's landing markers.

They settle along the unclaimed edge of the assay slab, vanes ragged with fungal bitterness and red root residue. The scar-vane broker drops a pellet between the two road forks: half-digested lantern seed, salt dust, and an aborted curl of pollen tissue.

The lattice ants form an accusation glyph around it before either Airawa asks a question.

-> diversion_order

=== diversion_order ===
The local Matriarch pulses through the terrace. Sap lifts in the buttress. Contract ribbons tremble. The lantern tree darkens every knot facing the rival fork, and the candle road closes two amber beads downhill.

The elder translates the coalition's demand for the junior tender, though no one present needs all of it translated.

"The rival canopy opens the disputed eclipse crossing and allows one ancestry copy. Until then, its pollen and gamete traffic receives no light, no local minerals, and no safe roost. Our household's next gestation petition moves forward if we certify their default."

The scar-vane broker bites the empty salt spoon hard enough to bend it.

-> grievance_choice

=== grievance_choice ===
// ghostlight.choice_layer: grievance_hearing
+ {salt_reserve >= 1} [Invite the broker to the salt basin and let the grievance land before answering the Matriarch.]
    // ghostlight.action_label: offer_resource
    // ghostlight.branch_label: hear_courier_first
    ~ salt_reserve = salt_reserve - 1
    ~ threadwing_trust = threadwing_trust + 1
    ~ ant_testimony = ant_testimony + 1
    The junior tender fills the shallowest basin and steps back beyond beak reach. The broker lands, eats once, and sheds a vane fiber across the ant lattice.

    The fiber carries a route memory: three rival lanterns paid, then darkened from the local side; a fungal branch fed, then pinched shut; gamete pollen cooling in a roost that no longer has light.

    The elder's mantle goes very still.
    -> coalition_muster
+ [Let the lattice ants assay the dropped pellet and both road forks.]
    // ghostlight.action_label: request_diagnosis
    // ghostlight.branch_label: audit_diversion
    ~ ant_testimony = ant_testimony + 2
    ~ route_light = route_light - 1
    The junior tender places a sugar flake at the center of the glyph and asks one precise question: weather, contamination, or deliberate closure?

    Ant bodies bridge pellet, root residue, and fungal glue. The answer takes most of the remaining light window. The glyph resolves into three joined lines: paid route, clean cargo, chosen refusal.

    Truth arrives locally and sends the bill to time.
    -> coalition_muster
+ [Repeat the Matriarch's terms aloud and hang the household marker on the local fork.]
    // ghostlight.action_label: speak
    // ghostlight.branch_label: endorse_local_terms
    ~ matriarch_favor = matriarch_favor + 2
    ~ household_standing = household_standing + 1
    ~ rival_relief = rival_relief - 1
    The junior tender names the crossing, the ancestry copy, and the promised gestation priority. Their lower hands hang the household marker where the local amber beads are brightest.

    The buttress warms. The elder exhales. Several threadwings lift from the rack and land farther away.

    A constituency can look exactly like a family trying not to lose its place in line.
    -> coalition_muster
+ [Braid the rival-facing fiber into the household mantle as temporary collateral.]
    // ghostlight.action_label: take_custody
    // ghostlight.branch_label: shelter_rival_claim
    ~ rival_relief = rival_relief + 2
    ~ household_standing = household_standing + 1
    ~ matriarch_favor = matriarch_favor - 1
    The junior tender threads the red-edged fiber through an empty loop on the mantle. The rival's claim now sits under an Airawa household's public protection until the hearing ends.

    The scar-vane broker releases the spoon.

    The local Matriarch tightens the root shelf just enough to remind every talon which body supports the terrace.
    -> coalition_muster

=== coalition_muster ===
// ghostlight.fold: coalition_positions_visible
The pollen tithe stops pretending to be breakfast.

{threadwing_trust >= 4: The scar-vane broker remains on the public rack, close enough to carry a negotiated answer.}
{threadwing_trust <= 2: The broker retreats to the unclaimed slab edge; every reply will now travel as an accusation.}
{ant_testimony >= 3: The ant glyph shows deliberate diversion in three clean lines. Even the elder cannot file it under weather.}
{ant_testimony <= 1: The pellet remains interpretable in all the politically convenient ways.}
{rival_relief >= 2: The rival-facing fiber is protected from recycling, giving the distant canopy a recognized claim in the hearing.}
{matriarch_favor >= 4: The local buttress warms around the junior tender's talons, promising the household a place nearer the next gestation window.}
{household_standing >= 4: Other household ribbons shift beside the junior tender's marker. Support is not agreement, but it is company.}

The candle road raises bitter beads across the rival fork. The lantern tree dims another knot. The scar-vane broker spreads every sensory ribbon, reading static, root pressure, fungal hunger, and the familiar scent of a powerful tree becoming convinced it acts alone.

-> terms_choice

=== terms_choice ===
// ghostlight.choice_layer: coalition_terms
+ [Press the household seal into the root clay and certify the rival canopy's default.]
    // ghostlight.action_label: certify_claim
    // ghostlight.branch_label: bind_local_coalition
    ~ matriarch_favor = matriarch_favor + 2
    ~ household_standing = household_standing + 1
    ~ rival_relief = rival_relief - 1
    The junior tender braces with taloned feet and one clawed hand, then uses the lower pair to press the household seal cleanly into the clay.

    The local amber lane brightens. The rival fork goes dark.

    The elder touches brow plate to the new mark. "One future moved forward," they say. They do not say which futures moved back.
    -> eclipse_accounting
+ {ant_testimony >= 2} [Read the ant glyph aloud and refuse to call chosen diversion a failed payment.]
    // ghostlight.action_label: speak
    // ghostlight.branch_label: publish_ant_finding
    ~ ant_testimony = ant_testimony + 1
    ~ rival_relief = rival_relief + 2
    ~ matriarch_favor = matriarch_favor - 2
    The junior tender names each joined line: paid route, clean cargo, chosen refusal.

    Lattice ants climb onto the public rack and rebuild the glyph where the whole terrace can see it. The candle road leaves one rival bead lit. The lantern tree holds one rival-facing knot at half brightness, which is plant diplomacy for declining to be an accomplice while avoiding the vulgarity of a speech.

    The local buttress cools hard.
    -> eclipse_accounting
+ {road_credit >= 2} [Spend the household's road credit to keep the rival mineral fork alive through eclipse.]
    // ghostlight.action_label: spend_credit
    // ghostlight.branch_label: fund_fungal_corridor
    ~ road_credit = road_credit - 2
    ~ rival_relief = rival_relief + 2
    ~ route_light = route_light + 1
    ~ matriarch_favor = matriarch_favor - 1
    The junior tender lays the household's road ribbon across the bitter beads. White mycelium reads the accumulated clean offerings and protected fruiting bodies, then opens a narrow amber corridor under the ribbon.

    Minerals begin moving toward the rival canopy. So does the debt. The road has not chosen kindness; it has chosen future traffic over a root's short victory.
    -> eclipse_accounting
+ {salt_reserve >= 2} [Place the remaining salt at the fork and fund a neutral courier passage.]
    // ghostlight.action_label: transfer_resource
    // ghostlight.branch_label: fund_neutral_passage
    ~ salt_reserve = salt_reserve - 2
    ~ threadwing_trust = threadwing_trust + 2
    ~ rival_relief = rival_relief + 2
    ~ household_standing = household_standing + 1
    ~ matriarch_favor = matriarch_favor - 2
    The last two measures fall into separate basins, one on each side of the assay slab. The junior tender turns both household markers faceup.

    "Paid passage," they say. "No canopy owns the carriers."

    The scar-vane broker calls once. The flock descends, one courier to each basin, and makes neutrality look remarkably like organized appetite.
    -> eclipse_accounting

=== eclipse_accounting ===
// ghostlight.fold: eclipse_route_test
Umbros begins to eat the sun. Cold shadow moves across the root shelf.

The terrace resolves into what each party has actually purchased.

{route_light >= 4: Lantern knots hold a clear line above both roosts, enough light for loaded couriers to launch safely.}
{route_light <= 2: The roost membranes fade into shadow; every departure will be slower and easier to deny.}
{road_credit >= 4: The fungal road has enough paid confidence to keep both surface lanes legible.}
{road_credit <= 1: The road's narrow corridor glows under a spent household ribbon, useful now and expensive later.}
{ant_testimony >= 3: The accusation glyph stands on the public rack where no root can quietly compost it.}
{matriarch_favor >= 5: The local Matriarch opens a warm gestation marker beside the household seal.}
{matriarch_favor <= 1: The household's gestation marker remains dark beneath the buttress.}
{rival_relief >= 3: A thin chain of amber beads and courier bodies reconnects the rival fork before its cargo cools.}
{rival_relief <= 0: The rival-facing ribbon curls dry on the rack while the local lane takes all traffic.}
{household_standing >= 4: Neighboring household markers face outward beside the junior tender's seal, making retaliation socially expensive.}

The elder watches the dark move over the contract rack. "Now choose the story this terrace will remember. The food web will choose whether to believe us."

-> final_choice

=== final_choice ===
// ghostlight.choice_layer: pollen_tithe_outcome
+ [Enforce the full diversion and claim the household's promised gestation priority.]
    // ghostlight.action_label: enforce_sanction
    // ghostlight.branch_label: prioritize_local_matriarch
    {matriarch_favor >= 5 && route_light >= 3:
        The junior tender moves the household marker beneath the warm gestation sign. The local coalition closes around it.
        -> ending_local_success
    - else:
        The junior tender reaches for the promised priority, but the coalition has fewer willing organs than the Matriarch assumed.
        -> ending_local_cost
    }
+ [Publish the ant finding and leave the route choice to the couriers.]
    // ghostlight.action_label: disclose_evidence
    // ghostlight.branch_label: prioritize_public_audit
    {ant_testimony >= 3 && threadwing_trust >= 3:
        The junior tender fixes the ant glyph beside the household seal and steps away from both forks.
        -> ending_audit_success
    - else:
        The junior tender offers a finding too thin, or to couriers already tired of being used as punctuation.
        -> ending_audit_cost
    }
+ [Keep the minimum rival corridor open, even if the household loses its place in line.]
    // ghostlight.action_label: hold_route
    // ghostlight.branch_label: prioritize_rival_relief
    {rival_relief >= 3 && household_standing >= 3:
        The junior tender anchors the household ribbon across the open fork and braces for the root's answer.
        -> ending_relief_success
    - else:
        The junior tender takes the rival ribbon, but too few parties have agreed to carry its cost.
        -> ending_relief_cost
    }
+ [Ask lantern, road, ants, and couriers to set terms before either Matriarch speaks again.]
    // ghostlight.action_label: convene_partners
    // ghostlight.branch_label: prioritize_partner_compact
    {road_credit >= 2 && threadwing_trust >= 3 && route_light >= 3 && ant_testimony >= 1:
        The junior tender lays no seal. They open both lower hands toward the route-owning bodies around the terrace.
        -> ending_compact_success
    - else:
        The junior tender calls for a compact among partners whose accounts do not yet meet.
        -> ending_compact_cost
    }

=== ending_local_success ===
// ghostlight.ending_label: local_matriarch_success
// ghostlight.training_hook: ecological_sanction_buys_brittle_priority
The local fork burns amber beneath cold lantern light. Threadwings carry pollen and gametes into the local canopy first. The household's gestation marker opens warm in the buttress, a promise made visible where every rival can read it.

Downhill, the rival fork closes.

The victory is real. So is the invoice. The candle road has less return traffic to digest. The couriers demand more salt for the hostile route. The rival Matriarch now knows which household seal made the sanction legal.

The elder touches the warm marker, then the cooling road. "We moved one future forward," they say. "Try not to stand where the others arrive."
-> END

=== ending_local_cost ===
// ghostlight.ending_label: local_matriarch_cost
// ghostlight.training_hook: overreach_breaks_the_carrier_coalition
The Matriarch opens the household's warm gestation marker.

The scar-vane broker looks at it, looks at the dark rival fork, and launches into eclipse without cargo. The flock follows. The candle road extinguishes its local beads one by one, unwilling to maintain a route emptied by its own patron.

The household has priority in an archive and no carriers willing to complete the contract.

Power has produced a beautiful receipt for something it can no longer obtain.
-> END

=== ending_audit_success ===
// ghostlight.ending_label: public_audit_success
// ghostlight.training_hook: neutral_testimony_reprices_matriarch_power
The ant glyph holds: paid route, clean cargo, chosen refusal.

The scar-vane broker lifts the pellet, flies once around the public rack, and chooses the rival fork. Half the flock follows. The other half takes the local canopy. The lantern tree lights both departures. The candle road reopens one bead on each side, preserving traffic while declining the Matriarch's fiction.

The local buttress stays cold under the junior tender's feet. Their household's petition will not move forward today.

But no one on the terrace can call the diversion weather again. A Matriarch can survive being contradicted. What costs her is making the contradiction portable.
-> END

=== ending_audit_cost ===
// ghostlight.ending_label: public_audit_cost
// ghostlight.training_hook: weak_proof_becomes_faction_ammunition
The junior tender raises the glyph. It offers fragments: bitter fungus, red residue, one courier memory, too much delay.

The elder calls it concerning. The Matriarch calls it contamination. The scar-vane broker calls nothing at all and carries the ambiguous account away.

The rival canopy gains an accusation. The local canopy gains an excuse. The household gains a dark gestation marker and the rare privilege of disappointing both sides at once.
-> END

=== ending_relief_success ===
// ghostlight.ending_label: rival_corridor_success
// ghostlight.training_hook: constituency_spends_priority_to_preserve_food_web
The household ribbon holds the narrow corridor open while amber beads climb toward the rival canopy. Threadwings launch with cooled pollen tucked close to their gut warmth. It is not enough traffic for abundance. It is enough to keep the rival gestation tissue from losing the whole window.

The local Matriarch darkens the household marker.

Other household ribbons remain turned outward beside it. Retaliation will cost her tending labor, salt, and public obedience she still needs. The junior tender has not defeated a Matriarch. They have made one cruelty more expensive than restraint.

The elder offers back the bent salt spoon. "A ceremonial implement," they say, "commemorating the day breakfast developed foreign policy."
-> END

=== ending_relief_cost ===
// ghostlight.ending_label: rival_corridor_cost
// ghostlight.training_hook: unsupported_mercy_becomes_targeted_retaliation
The junior tender braces the ribbon across the rival fork alone.

The local root lifts under one taloned foot. The candle road withdraws before the cloth can tear. Lantern knots darken. The scar-vane broker snatches the rival fiber from the mantle and carries it into eclipse as testimony.

The corridor closes. The household marker closes with it.

Kindness without a solvent constituency is not a policy. Here it is a name the food web can remember and a Matriarch can punish precisely.
-> END

=== ending_compact_success ===
// ghostlight.ending_label: partner_compact_success
// ghostlight.training_hook: trophic_partners_force_renegotiation
The terrace answers without waiting for either great root.

The lantern tree opens two modest lanes. The candle road sets a higher mineral price on both. Lattice ants divide the accusation glyph and attach one half to each canopy's account. Threadwings carry local cargo first, rival cargo second, and both Matriarchs' grievances last.

No one is satisfied. Traffic moves.

The junior tender watches the roots absorb the new terms. Ecology has not become fair. It has remembered that a coalition is made of parties able to leave.
-> END

=== ending_compact_cost ===
// ghostlight.ending_label: partner_compact_cost
// ghostlight.training_hook: divided_partners_are_bought_separately
The junior tender opens both lower hands.

The lantern tree gives one uncertain pulse. The road waits for food. The ants wait for sugar. The threadwings wait for light. Each partner has leverage. None has enough shared credit to spend it together.

The local Matriarch offers them terms one by one.

By full eclipse, the road is lit only toward the local canopy and the flock has split into private bargains. The junior tender learns the ugly arithmetic of a coalition announced before it exists: power gets to negotiate wholesale; everyone else queues at the retail window.
-> END
