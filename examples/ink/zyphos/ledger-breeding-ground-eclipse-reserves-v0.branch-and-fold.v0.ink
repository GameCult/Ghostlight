// ghostlight.artifact_id: ledger_breeding_ground_eclipse_reserves_branch_fold_v0
// ghostlight.fixture_id: ledger-breeding-ground-eclipse-reserves-v0
// ghostlight.scene_id: ledger-breeding-ground-eclipse-reserves-v0.west-terrace-shortfall
// ghostlight.final_ink_path: examples/ink/zyphos/ledger-breeding-ground-eclipse-reserves-v0.branch-and-fold.v0.ink

VAR reserve_volume = 2
VAR reserve_heat = 2
VAR road_credit = 2
VAR mat_consent = 1
VAR herd_trust = 2
VAR nursery_pressure = 1
VAR public_ledger = 1
VAR newcomer_standing = 1
VAR old_family_claim = 2
VAR eclipse_time = 3

-> start

=== start ===
The west arrival terrace wakes by stirring breakfast.

Its dark, root-bound stone spreads like a low fan. At the narrow western mouth, two candle-fungal intake channels arrive beside the traveling road. At the broad eastern edge, three shallow ramps descend nurseryward. A wet prismwake lobe occupies the mat-side channel. Lantern roots and a glassback heat rail line the grove-side edge. Three round reserve basins sit between them, low enough for a folded Sa'ueia to reach with both chest hands.

The first basin is warm. The second is half full. The third contains a paddle and an institutional optimism unsupported by fluid.

-> routine_keeper

=== routine_keeper ===
Veyr, the reserve keeper, folds four long running legs around the middle basin. Short rust-red fibers cover the long body; the bare throat patch is already dark with work heat. Two smaller chest limbs stir the mineral-sugar culture with a broad leaf paddle while paired facial fans taste temperature, road minerals, and the sharp edge that means a nursery graft has been rinsed too close to breakfast.

Behind the ration rail, caretakers fill low feeding cups for infants, infirm adults, and graft patients. Nobody queues upright. Sa'ueia work happens close to the ground, from several sides, with enough room for long bodies and opinions to overlap.

Scored Blue, an elder glassback grazer, stands at the heat rail. Translucent dorsal plates glow teal over stored warmth. The west road's amber fruiting candles pulse beside clean collection beds of dung, shed fibers, and failed graft strips. The prismwake lobe flashes silver-rose along an uninjured edge.

Ordinary reserve work consists of asking four different living systems to agree that the same breakfast exists.

-> routine_choice

=== routine_choice ===
// ghostlight.choice_layer: routine_intake_work
+ [Place clean failed graft strips in the road's collection bed and name each source.]
    // ghostlight.action_label: transfer_object
    // ghostlight.branch_label: prime_road_intake
    ~ road_credit = road_credit + 2
    ~ reserve_volume = reserve_volume + 1
    Veyr lifts the strips one by one with the three soft digits of a chest hand.

    "Cold rejection. Clean edge. No deep-memory admission."

    The west road draws the strips below its braided surface. Amber candles open along the mineral sluice. Clear liquor beads into the first basin through a root-lined lip.

    One candle leans toward Veyr's paddle.

    "You may inspect it after the shift," Veyr says. "It has confessed nothing."
    -> routine_fold
+ [Lay a woven heat wick across Scored Blue's dorsal plates and wait for consent.]
    // ghostlight.action_label: use_object
    // ghostlight.branch_label: prime_herd_heat
    ~ reserve_heat = reserve_heat + 2
    ~ herd_trust = herd_trust + 1
    ~ eclipse_time = eclipse_time - 1
    Veyr rests the pale wick across the heat rail, not the grazer.

    Scored Blue turns broadside. Teal warmth brightens beneath the translucent plates, then moves through the wick into the basin's stone collar. The grazer could withdraw with one step. It does not.

    A nursery child flashes both facial fans at the plate glow.

    Scored Blue fogs the nearest plate. Dignity survives another admirer.
    -> routine_fold
+ [Repair the prismwake lobe's torn regrowth edge before taking its morning skim.]
    // ghostlight.action_label: repair
    // ghostlight.branch_label: prime_mat_consent
    ~ mat_consent = mat_consent + 2
    ~ reserve_volume = reserve_volume + 1
    ~ eclipse_time = eclipse_time - 1
    Veyr crosses to the shallow mat-side channel and folds beside the injured edge. Two chest hands press a clean mineral mesh over the tear while the four running feet stay on bare stone.

    The mat puckers around the mesh. Silver turns blue, then settles into a pale green allowance. Sugar-rich surface tissue loosens into the waiting skim tray.

    Veyr takes only what the green line releases. Being watched by the floor improves technique.
    -> routine_fold
+ [Open the portable archive and read every outstanding family claim before serving.]
    // ghostlight.action_label: show_object
    // ghostlight.branch_label: prime_public_accounts
    ~ public_ledger = public_ledger + 2
    ~ newcomer_standing = newcomer_standing + 1
    ~ nursery_pressure = nursery_pressure + 1
    Veyr opens a low archive case beside the ration rail. Flexible memory membranes rise in colored layers. Old fungal tastes, mat flashes, herd warmth, work hours, and unpaid shelter obligations become legible to the caretakers nearest the basins.

    A child reaches for the brightest cord.

    "That one is debt," Veyr says.

    The child withdraws with the speed of a scholar who has discovered curriculum.
    -> routine_fold

=== routine_fold ===
// ghostlight.fold: ordinary_reserve_shift
The west terrace settles into its daily metabolism.

Caretakers carry filled cups down the three nursery ramps. The west road passes mineral liquor through one intake channel and samples the collection beds through the other. Scored Blue lends only the warmth the herd can spare before its calving route. The prismwake lobe keeps its regrowth nodes below water and its accounting colors above it.

{road_credit >= 4: Both fungal intake channels hold steady amber. The road has accepted today's material as useful and well named.}
{reserve_volume >= 4: The first two basins stand near their black-glazed fill marks, enough culture to make generosity look briefly cheap.}
{reserve_heat >= 4: Teal heat moves through the woven wick and turns the basin rims faintly gold.}
{mat_consent >= 3: The repaired mat edge keeps a pale green release line beside the skim tray.}
{herd_trust >= 3: Scored Blue rests one plate against the heat rail instead of holding the whole herd a careful body-length away.}
{public_ledger >= 3: The open archive displays claims and obligations where both queues can read them.}
{nursery_pressure >= 2: Feeding cups return from the ramps faster than Veyr can clean them. Public accounting has not reduced anyone's appetite.}
{eclipse_time <= 2: Umbros has begun to bite the sun. The lantern knots wake before the basin work is comfortably ahead.}

Then the prismwake lobe flashes a torn-bite pattern from three days ago.

-> shortfall_bite

=== shortfall_bite ===
Silver. Red. The route color of a bite taken below the regrowth line.

The mat-side sugar release closes. One fungal intake channel goes dark in sympathy or caution; Veyr cannot tell which from the candle spacing alone. Scored Blue darkens two plates and shifts weight toward the road. The herd has registered a contract dispute and an eclipse timetable.

The second basin's surface falls below its fill mark.

-> claimants_arrive

=== claimants_arrive ===

Two arriving families stop at the road boundary.

Ossa stands for a many-return family whose braided archive cords reach back farther than Veyr's appointment. Her long gray-fibered body carries a polished flank case and no fresh offering. She asks for an advance ration for three route-tired breeding adults, to be repaid after the next wetland circuit.

Rin stands for a household divided from a larger family after floodwater spoiled its portable archive. Their ochre flank frame carries clean collection bundles, a rolled mat-repair mesh, and one exhausted provider who needs warmth before doing more work. The road knows the material. It does not yet know the household.

The remaining reserve can carry the nursery through totality, or honor both arrivals at full measure. It cannot do both.

-> shortfall_choice

=== shortfall_choice ===
// ghostlight.choice_layer: shortfall_response
+ [Test Ossa's oldest archive cord against the road's candles and the mat's bite memory.]
    // ghostlight.action_label: inspect_object
    // ghostlight.branch_label: verify_old_family_claim
    ~ old_family_claim = old_family_claim + 2
    ~ public_ledger = public_ledger + 1
    ~ eclipse_time = eclipse_time - 1
    Ossa uncoils the long archive cord and sets its first loop across the fungal boundary. Veyr takes it with two chest digits, lays it across the routeward lip of the empty basin, and holds one end near the road candles and the other above the prismwake water.

    Amber answers the cord's mineral history. The mat repeats the torn-bite color, but not at the cord. Old credit is real. So is present shortage. The two facts decline to eat each other for convenience.

    "Satisfied?" Ossa asks.

    "Documented," Veyr says. Satisfaction is a luxury category.
    -> claims_fold
+ [Place Rin's clean bundles in the sample bed and join the repair work while the road watches.]
    // ghostlight.action_label: repair
    // ghostlight.branch_label: witness_new_household_work
    ~ newcomer_standing = newcomer_standing + 2
    ~ road_credit = road_credit + 1
    ~ eclipse_time = eclipse_time - 1
    Rin opens the bundles: dry shed fiber, clean dung cake, two failed tool grips, and mineral mesh cut for the mat's regrowth edge.

    Veyr folds beside them. Four chest hands fasten the mesh while the road samples each bundle. Amber candles open in a slow sequence. Not trust. Receipt.

    Rin's exhausted provider leans against the heat rail but does not touch it. Scored Blue turns one clear plate toward them and waits.
    -> claims_fold
+ [Ask the prismwake lobe to display the damaged bite history beside both family claims.]
    // ghostlight.action_label: gesture
    // ghostlight.branch_label: publish_mat_damage
    ~ mat_consent = mat_consent + 1
    ~ public_ledger = public_ledger + 2
    ~ old_family_claim = old_family_claim - 1
    ~ nursery_pressure = nursery_pressure + 1
    Veyr opens both facial fans toward the wet channel and places Ossa's cord beside Rin's repair mesh on bare stone.

    The mat lifts a low sheet of prism cells. Silver-red damage repeats along it, followed by a faded color matching a grazer that traveled under Ossa's family protection.

    It is provenance, not guilt. Everyone on the terrace can see how quickly those become cousins.

    Ossa's throat patch goes cold pale. "We paid for that herd's crossing."

    "The mat appears to have itemized the difference," Veyr says.
    -> claims_fold
+ [Cut every adult ration in half now and preserve full cups for infants, infirm residents, and graft patients.]
    // ghostlight.action_label: withhold_resource
    // ghostlight.branch_label: ration_adults_early
    ~ reserve_volume = reserve_volume + 1
    ~ reserve_heat = reserve_heat + 1
    ~ nursery_pressure = nursery_pressure + 2
    ~ public_ledger = public_ledger + 1
    Veyr turns the adult cup stack upside down and sets out the half-depth cups.

    Nobody argues with the order of need. Several adults argue with the size of the object used to express it.

    The full cups go nurseryward. The half cups remain at the rail beside Ossa's old cord and Rin's new bundles. Equality has acquired tableware and is already unpopular.
    -> claims_fold

=== claims_fold ===
// ghostlight.fold: claims_become_public_pressure
The shortfall now has witnesses.

{old_family_claim >= 4: Ossa's oldest cords hold a strong amber echo. The west road recognizes years of repayment even while one intake stays dark.}
{old_family_claim <= 1: Ossa's polished cords lie beside the mat's damage colors, old standing reduced by a current wound it did not personally make.}
{newcomer_standing >= 3: Rin's bundles have been sampled and their repair mesh lies fixed along the regrowth edge. Current work has become visible history.}
{newcomer_standing <= 1: Rin's household remains socially present and ecologically faint: bodies at the boundary, archive mostly gone, promises with nowhere durable to sit.}
{public_ledger >= 3: Road candles, archive membranes, mat colors, and ration cups make the dispute readable from both queues.}
{public_ledger <= 1: Veyr holds the only complete comparison. Every private explanation now risks becoming ownership.}
{nursery_pressure >= 3: The caretaker ring crowds the ration rail. Behind them, a graft patient cries in the first hollow and the sound makes every accounting category feel slightly obscene.}
{reserve_volume >= 4: Two basins still carry enough volume to bargain from something better than panic.}
{reserve_heat <= 2: The culture surface begins to dull. Volume without warmth is an inventory of future refusal.}

The west lantern grove pulses amber-white toward the basins: contribution requested, nursery risk rising. Scored Blue answers with a slow teal flare but keeps the herd angled toward departure. The prismwake lobe remains closed except at the repaired edge. The road opens one small candle beside Ossa's cord and another beside Rin's bundles.

Two credits. Two timescales. One empty basin.

-> bargain_choice

=== bargain_choice ===
// ghostlight.choice_layer: trophic_bargain
+ {old_family_claim >= 4} [Spend part of Ossa's old route credit to request an advance through the dark intake.]
    // ghostlight.action_label: authorize
    // ghostlight.branch_label: spend_old_route_credit
    ~ old_family_claim = old_family_claim - 1
    ~ road_credit = road_credit - 1
    ~ reserve_volume = reserve_volume + 1
    ~ nursery_pressure = nursery_pressure + 1
    Veyr lays Ossa's cord across the dark fungal channel and marks one future repair circuit in the open archive.

    The road opens just wide enough to pass stored mineral liquor into the third basin. The channel stays dark around the cord. Recognition is not enthusiasm.

    Ossa inclines both facial fans. Rin watches an ancestor feed someone who has not yet worked today.
    -> totality_threshold
+ {newcomer_standing >= 3} [Recognize Rin's sampled bundles and completed repair as credit for one warm measure.]
    // ghostlight.action_label: authorize
    // ghostlight.branch_label: recognize_current_contribution
    ~ newcomer_standing = newcomer_standing + 1
    ~ public_ledger = public_ledger + 1
    ~ reserve_volume = reserve_volume + 1
    ~ eclipse_time = eclipse_time - 1
    Veyr knots a fresh claim cord around one sampled fiber and the cut end of Rin's repair mesh. The open archive takes the road's amber receipt and the mat's green edge beside it.

    A new household cannot manufacture a past. It can begin one in public.

    The mat releases a narrow skim. The road passes it to the basin after a long sample. Rin's provider remains at the heat rail, waiting for the rest of the bargain.
    -> totality_threshold
+ {mat_consent >= 3} [Offer another repair watch for one partial sugar release before totality.]
    // ghostlight.action_label: gesture
    // ghostlight.branch_label: trade_labor_for_sugar
    ~ mat_consent = mat_consent - 1
    ~ reserve_volume = reserve_volume + 1
    ~ nursery_pressure = nursery_pressure - 1
    ~ eclipse_time = eclipse_time - 1
    Veyr touches the repaired edge, the next unpatched tear, and then the open archive.

    The prismwake lobe answers pale green along one handspan. Sugar tissue slips into the skim tray. Beside it, a blue line marks the second tear and the future watch now owed.

    The basin gains a measure. Veyr gains tomorrow morning.
    -> totality_threshold
+ [Ask Scored Blue to delay the herd's calving route and lend one last plate of heat.]
    // ghostlight.action_label: gesture
    // ghostlight.branch_label: borrow_herd_heat
    ~ reserve_heat = reserve_heat + 2
    ~ herd_trust = herd_trust - 1
    ~ nursery_pressure = nursery_pressure - 1
    ~ eclipse_time = eclipse_time - 1
    Veyr removes the heat wick, cleans both contact faces, and lays it across the rail again. Then Veyr points from Scored Blue's plates to the cooling basins and finally toward the route the herd meant to take.

    Scored Blue darkens every plate.

    The herd waits long enough for the silence to become expensive. Then the elder turns broadside and gives one fierce teal pulse through the wick. Farther routeward, calves press close to adults who would prefer to be moving.

    Borrowed warmth arrives with a departure debt attached.
    -> totality_threshold

=== totality_threshold ===
// ghostlight.fold: allocation_under_totality
Umbros closes over the dim primary.

The terrace becomes a ledger made of light: amber fungal candles at the west mouth, silver and rose prism cells in the wet channel, teal glassback plates at the grove-side rail, cold blue lantern knots above three nursery ramps. The reserve basins occupy the center. Their black fill marks are visible to everyone.

{reserve_volume >= 4: Two basins stand high enough that a full nursery ration and one arrival claim might both survive honest measurement.}
{reserve_volume <= 2: Only the first basin reaches its fill mark. Every second measure now comes out of somebody else's body.}
{reserve_heat >= 4: Gold warmth rings the basin collars and steam lifts from the culture in thin veils.}
{reserve_heat <= 2: The culture has gone dull at the edges. Graft patients can consume it, but not safely for long.}
{road_credit >= 3: Both road candles beside Veyr burn evenly, offering some latitude in how material crosses the intake.}
{road_credit <= 1: The dark channel remains shut and the open channel narrows to a sampling thread.}
{mat_consent >= 3: The prismwake lobe holds one green release line despite totality.}
{mat_consent <= 1: The mat shows only the blue marks of repair still owed.}
{herd_trust >= 3: Scored Blue remains pressed to the heat rail while the herd forms a calm crescent behind the elder.}
{herd_trust <= 1: Scored Blue has stepped clear. Teal plates face the departure route, warmth retained for calves.}
{nursery_pressure >= 4: The caretaker ring braces all six feet around the ration rail. They will accept a decision; they are past accepting vagueness.}
{public_ledger >= 4: Ossa's old cord and Rin's new receipt hang side by side where neither family can privately improve the story.}
{newcomer_standing >= 4: Rin's household has a fresh claim recognized by road, mat, archive, and visible work.}
{old_family_claim >= 4: Ossa's archive cord still carries enough old agreement to request a genuine advance.}
{eclipse_time <= 1: The lantern knots show late totality. Whatever Veyr chooses must be served before returning light changes the culture again.}

Veyr has authority to ration the nursery reserve. Veyr does not own the road, mat, herd, grove, or the years inside either family's claim.

The cups wait.

-> final_choice

=== final_choice ===
// ghostlight.choice_layer: final_allocation
+ [Feed infants, infirm residents, and graft patients in full; close the reserve to both arrival claims.]
    // ghostlight.action_label: withhold_resource
    // ghostlight.branch_label: protect_nursery_core
    {reserve_volume >= 3 && reserve_heat >= 3 && nursery_pressure <= 3:
        Veyr turns every full cup nurseryward and lowers the routeward ration rail.
        -> ending_core_success
    - else:
        Veyr closes the rail and serves the nursery from what remains.
        -> ending_core_cost
    }
+ [Honor Ossa's old-return advance, with a public repair obligation tied to the measure.]
    // ghostlight.action_label: authorize
    // ghostlight.branch_label: honor_old_credit
    {old_family_claim >= 3 && road_credit >= 2 && reserve_volume >= 3:
        Veyr loops Ossa's oldest cord through a fresh blue repair marker and fills one route cup.
        -> ending_old_credit_success
    - else:
        Veyr reaches for Ossa's cord before the living accounts can carry it.
        -> ending_old_credit_cost
    }
+ [Recognize Rin's present contribution and serve the exhausted provider one warm measure.]
    // ghostlight.action_label: transfer_resource
    // ghostlight.branch_label: honor_new_contribution
    {newcomer_standing >= 3 && public_ledger >= 3 && reserve_heat >= 3:
        Veyr fills a warm cup and sets it beside Rin's new receipt at the boundary.
        -> ending_new_credit_success
    - else:
        Veyr fills the cup before the receipt has enough witnesses.
        -> ending_new_credit_cost
    }
+ [Publish the shortfall and serve one shared thin measure to the nursery and both arrivals.]
    // ghostlight.action_label: distribute_resource
    // ghostlight.branch_label: share_the_shortfall
    {public_ledger >= 3 && mat_consent >= 2 && herd_trust >= 2 && eclipse_time >= 1:
        Veyr places the half-depth cups on both sides of the ration rail and leaves the archive open.
        -> ending_shared_success
    - else:
        Veyr divides the remaining culture before the trophic partners have agreed to the loss.
        -> ending_shared_cost
    }

=== ending_core_success ===
// ghostlight.ending_label: nursery_core_protected
// ghostlight.training_hook: triage_without_pretending_exclusion_is_free
The full cups travel down the three ramps.

Infants drink. The infirm receive warmth before the basin edges dull. Graft patients keep their clean culture through totality. At the boundary, Ossa and Rin receive water-mineral cloths, shelter light, and no reserve measure.

Ossa knots the refusal into an old cord. Rin knots it into a new one.

The nursery survives the eclipse. Tomorrow's supply circuit begins with two families who now know exactly whose need the commons placed after its own. Triage is not cruelty. It is also not absolution, however nicely the cups stack.
-> END

=== ending_core_cost ===
// ghostlight.ending_label: nursery_closure_cannot_create_supply
// ghostlight.training_hook: closure_does_not_conjure_resources
Veyr lowers the rail.

The reserve is still too cool, too shallow, or too contested to become a full ration by being guarded. The last graft cup leaves the basin with a dull rim. A caretaker returns it from the first hollow and asks for heat that is already walking routeward in glassback plates.

Ossa and Rin remain outside. So does the missing breakfast.

The commons has protected its claim to the reserve and discovered that ownership was never the scarce part.
-> END

=== ending_old_credit_success ===
// ghostlight.ending_label: old_credit_advanced_with_obligation
// ghostlight.training_hook: inherited_access_given_a_visible_price
The west road brightens around Ossa's cord.

Veyr fills one route cup. The archive binds it to two mat-repair watches and calving-lane defense before the next wetland circuit. Ossa accepts in front of Rin, the caretakers, Scored Blue, and the damaged prismwake lobe.

An old family receives nourishment before doing today's work because years of prior work still have witnesses. That is what credit is for.

It is also how advantage learns to describe itself as patience. The obligation stays visible because otherwise the cup would teach the wrong lesson perfectly.
-> END

=== ending_old_credit_cost ===
// ghostlight.ending_label: old_credit_overdraw
// ghostlight.training_hook: reputation_is_not_ownership
Veyr lifts Ossa's cord. The road candle beside it goes dark.

The old claim is genuine, but the mat has closed and the road will not convert history into material it does not possess. Veyr serves the advance from the nursery basin anyway.

Ossa's adults drink. A full cup disappears from the eastward stack. Rin watches polished memory perform the oldest trick in class history: arriving first by having arrived often.

At returning light, the prismwake lobe flashes the blue of unpaid repair. The next shortfall has already begun.
-> END

=== ending_new_credit_success ===
// ghostlight.ending_label: present_contribution_recognized
// ghostlight.training_hook: new_household_can_begin_public_credit
Veyr sets the warm cup beside Rin's fresh receipt.

The road opens one candle. The repaired mat edge shows green. Scored Blue keeps a teal plate against the rail until Rin's exhausted provider finishes drinking.

No recovered ancestor arrives to authorize the household. Current work, sampled material, and several living witnesses are enough to begin a history.

Ossa studies the new cord. Old standing has not vanished. It has acquired competition from the present, which is much ruder.
-> END

=== ending_new_credit_cost ===
// ghostlight.ending_label: declared_credit_without_ecological_backing
// ghostlight.training_hook: caretaker_cannot_author_credit_alone
Veyr carries the cup to the boundary.

The road keeps its candle dark. The mat gives no green line. Scored Blue has already stepped away from the heat rail. Rin's provider drinks because Veyr has authority over the cup, not because the trophic chain accepted the claim.

The act may still be right. It is not yet credit.

The nursery ledger records one warm measure missing and one household obligation with no agreed collector. Equality declared by a single keeper has purchased relief and a split account.
-> END

=== ending_shared_success ===
// ghostlight.ending_label: shortfall_shared_with_consent
// ghostlight.training_hook: federated_rationing_preserves_future_supply
Half-depth cups line both sides of the rail.

The nursery receives less, but enough warm culture to cross totality safely. Ossa's adults and Rin's provider receive the same thin measure. The mat holds one green release line. Scored Blue keeps one plate at the wick. The road passes mineral liquor slowly enough to keep sampling.

The archive names every missing measure and every next-cycle obligation. Nobody is made equal by being hungry. They are made participants in the same repair.

When the first edge of the sun returns, the third basin is empty and still warm.
-> END

=== ending_shared_cost ===
// ghostlight.ending_label: distributed_harm
// ghostlight.training_hook: equal_division_without_capacity_can_fail_everyone
Veyr fills every half cup.

The portions look fair in a row. Then the culture cools below the graft threshold. The road narrows its remaining channel. The prismwake lobe folds over its regrowth nodes. Scored Blue turns the herd routeward with heat still needed for calves.

Nobody was privileged. Everyone was underfed. The distinction will comfort exactly the part of the archive that cannot shiver.

By returning light, the nursery owes emergency warmth, both arrivals owe food they did not receive, and the trophic partners have learned that public procedure can distribute damage as neatly as nourishment.
-> END
