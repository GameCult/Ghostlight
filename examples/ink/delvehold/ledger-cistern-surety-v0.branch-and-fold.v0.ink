// ghostlight.artifact_id: ledger_cistern_surety_branch_fold_v0
// ghostlight.fixture_id: ledger-cistern-surety-v0
// ghostlight.scene_id: ledger-cistern-surety-v0.renewal-at-cistern-house-nine
// ghostlight.final_ink_path: examples/ink/delvehold/ledger-cistern-surety-v0.branch-and-fold.v0.ink

VAR water_margin = 2
VAR reserve_crystal = 2
VAR claim_strength = 1
VAR ecology_evidence = 0
VAR public_record = 1
VAR workshop_credit = 2
VAR household_burden = 1
VAR repair_integrity = 2
VAR relief_priority = 1

-> start

=== start ===
// ghostlight.scene: surety_cistern_establishing
Cistern House Nine has three pumps, two floors, one public landing, and a debt large enough to require its own inspection chair.

The dry rune gallery overlooks the wet intake chamber by a grated stair. Three brass-and-iron engines stand behind a waist-high safety rail. Their rods descend into warm mist and black water. A locked arch in the gallery wall carries the municipal crystal feed. On the public side of the rail, a terrace gauge measures the district's remaining water in calm white marks and an alarming red one.

Today is the pump surety reading. The Forge Consortium still owns most of the matched rune assemblies. The district pays for them through its water assessment. Master Talla Venn's workshop seal promises the machinery is maintained. Osric Vale, a reserve-house assessor, decides whether the promise may be insured for another term.

The water, having no legal training, waits in the lower chamber.

-> renewal_routine

=== renewal_routine ===
// ghostlight.scene: surety_renewal_table
Talla is a broad, gray-braided dwarf with a square brass seal at her belt and old gold-channel burns across two fingers. Her workshop operates the cistern. It does not own the district's thirst, although every contract on Osric's table has made a serious attempt to phrase matters otherwise.

Journeyworker Neri Ash cleans Engine One's pressure float with a rag that began the morning white and has since entered industry. Neri has repair skill, wages, and no civic seal.

Landing clerk Poma Reed chalks three figures onto the public slate: water in the upper tanks, crystal remaining in the emergency locker, and the assessment still owed on the pump note. Householders can read all three from the yellow line. Poma cannot cross the rail or vote a seal, but she can make a private number socially inconvenient.

Osric lays a red measuring rule beside the engine ledger. "Open one casing, show one honest hour, and let us all go back to pretending risk has edges."

Talla has time for one preparation before Engine One's renewal test.

-> ledger_choice

=== ledger_choice ===
// ghostlight.choice_layer: renewal_preparation
+ [Open Engine One's return housing under Osric's eye and have Neri call every seal aloud.]
    // ghostlight.action: open_inspection_housing
    // ghostlight.branch: prime_joint_inspection
    // ghostlight.intent: strengthen_the_claim_with_shared_custody_and_sound_repair
    ~ claim_strength = claim_strength + 1
    ~ repair_integrity = repair_integrity + 1
    ~ relief_priority = relief_priority + 1
    Talla unlocks the brass housing. Neri removes each bolt, places it on white cloth, and calls the seal marks in order. Osric repeats them into his ledger.

    "You sound almost pleased," Talla says.

    "Shared custody," Osric says. "The closest finance permits itself to friendship."
    -> renewal_fold
+ [Ask Poma to post the tank margin beside the unpaid assessment before testing.]
    // ghostlight.action: publish_resource_clock
    // ghostlight.branch: prime_public_water_clock
    // ghostlight.intent: make_delay_and_debt_visible_to_the_people_who_carry_them
    ~ public_record = public_record + 2
    ~ relief_priority = relief_priority + 1
    Poma writes TWO SAFE HOURS in letters large enough for the landing queue and the assessor to dislike equally.

    Under it she copies the unpaid assessment without rounding down.

    Osric taps the number with his red rule. "Public chalk does not improve credit."

    "It improves eyesight," Poma says.
    -> renewal_fold
+ [Feed Engine One a shaving from the emergency crystal locker before the test.]
    // ghostlight.action: spend_reserve
    // ghostlight.branch: prime_temporary_flow
    // ghostlight.intent: buy_water_margin_at_the_cost_of_reserve_and_unsealed_operation
    ~ water_margin = water_margin + 2
    ~ reserve_crystal = reserve_crystal - 1
    ~ repair_integrity = repair_integrity - 1
    ~ household_burden = household_burden + 1
    Neri seats the blue crystal shaving in the starter fork. Engine One takes the extra mana with the bright appetite of machinery consuming something marked emergency.

    The terrace gauge climbs one mark. Poma adds a surcharge line to the slate.

    "A temporary kindness," Osric says, "is often a permanent invoice."
    -> renewal_fold
+ [Send Neri down the grated stair to check the intake grilles before opening the casing.]
    // ghostlight.action: inspect_intake
    // ghostlight.branch: prime_ecology_check
    // ghostlight.intent: look_for_living_causes_before_the_contract_names_the_failure
    ~ ecology_evidence = ecology_evidence + 1
    ~ water_margin = water_margin - 1
    ~ public_record = public_record + 1
    Neri descends into warm mist with a hooded lamp and a hooked scraper. From above, Talla can see the lamp pass three pump rods and stop at Engine One's intake grille.

    "Shells," Neri calls. "Small. Blue-black. Grooved."

    Osric closes the surety book over one finger to keep his place. The gesture is neat enough to count as concern in his profession.
    -> renewal_fold

=== renewal_fold ===
// ghostlight.fold: routine_accounts_before_failure
Neri returns to the service floor. Poma stands at the public slate. Osric sits at the inspection table with the red rule and the surety book. Talla's seal remains at her belt.

{claim_strength >= 2: Engine One's opened housing, called seals, and shared ledger give an ordinary mechanical claim a respectable spine.}
{public_record >= 3: The water clock and debt now face the landing. Nobody can later improve the morning by moving a decimal indoors.}
{reserve_crystal <= 1: The emergency locker holds one useful blue gleam and a great deal of official darkness.}
{water_margin <= 1: The terrace gauge settles one white mark above red. The queue begins counting vessels instead of minutes.}
{water_margin >= 4: Water reaches the upper tanks, bought with reserve crystal and a new surcharge line.}
{repair_integrity >= 3: Engine One's fasteners, seals, and return plate sit in clean inspection order.}
{repair_integrity <= 1: Engine One is running on a temporary feed before its housing has earned the word closed.}
{relief_priority >= 2: Osric has enough shared evidence to request a life-service freight slot if the claim holds.}
{household_burden >= 2: Poma has already added an emergency charge to connections whose occupants did not choose the preparation.}
{workshop_credit >= 2: Talla's workshop still has enough standing to ask another sealed practice to carry part of a repair order.}

Osric nods toward the start lever. "One honest hour."

Talla opens Engine One.

-> failure

=== failure ===
// ghostlight.scene: surety_rune_mite_reveal
The pump completes eleven strokes.

On the twelfth, its gold return channel dims. The rod stops halfway into the wet chamber. Water falls back through the outlet with a sound like a room reconsidering its promises.

Neri throws the isolation wedge. Talla closes the feed with a null stroke. Engine One goes dark behind real iron.

When the return housing opens, six blue-black mites cling along the standardized branching rune. Each shell bears raised grooves matching the gold channels beneath it. Mana bleeds into those grooves and expires as dull violet heat.

Pipe mites are named local vermin. The surety covers them. Organisms adapted to a standardized rune array are deep-pattern interference. The surety excludes that.

Osric does not touch the insects. "Your claim has become literate."

Above them, the terrace gauge loses a mark.

-> cause_choice

=== cause_choice ===
// ghostlight.choice_layer: cause_and_claim
+ [Seal two live mites in an inspection phial and copy their groove pattern onto Poma's public slate.]
    // ghostlight.action: preserve_specimen
    // ghostlight.branch: record_adaptive_pattern
    // ghostlight.intent: preserve_ecological_evidence_even_if_it_voids_the_surety
    ~ ecology_evidence = ecology_evidence + 2
    ~ public_record = public_record + 1
    ~ claim_strength = claim_strength - 1
    ~ workshop_credit = workshop_credit - 1
    Talla holds the phial while Neri taps two mites inside. Their ridged shells drink the last light from the glass rim.

    Poma copies the branching pattern beside the engine number and tank level.

    Osric turns a copper refusal plate face-up on the table. He has not stamped it yet. Courtesy survives in the brief interval before policy catches up.
    -> cost_fold
+ [Brush the mites into the lime tray and enter them as ordinary pipe vermin.]
    // ghostlight.action: classify_for_claim
    // ghostlight.branch: claim_ordinary_vermin
    // ghostlight.intent: keep_the_failure_inside_covered_language_and_restore_water_fast
    ~ claim_strength = claim_strength + 2
    ~ ecology_evidence = ecology_evidence - 1
    ~ repair_integrity = repair_integrity + 1
    ~ household_burden = household_burden + 1
    Neri pauses with the brush above the shells.

    Talla says, "Pipe mites. Named, local, cleared."

    The lime takes them. Osric writes the words exactly. Poma writes exactly who supplied them.

    A covered cause has entered the room. Whether it is the true one has become somebody else's future shift.
    -> cost_fold
+ [Open Engine Two and compare its unused return rune before naming the mites.]
    // ghostlight.action: compare_standard_array
    // ghostlight.branch: test_repeated_adaptation
    // ghostlight.intent: distinguish_local_fouling_from_a_pattern_targeting_the_standard_design
    ~ ecology_evidence = ecology_evidence + 2
    ~ water_margin = water_margin - 1
    ~ repair_integrity = repair_integrity + 1
    Neri opens Engine Two. Three pale scratches cross its return plate at the same branch points. The pump has not run today.

    Osric measures the distance between scratches with his red rule.

    "Coincidence remains available," he says.

    "At what premium?" Poma asks.

    Osric turns the copper refusal plate face-up.
    -> cost_fold
+ [Run Engine Three above its catalogue duty while the tanks fall.]
    // ghostlight.action: overrun_backup_pump
    // ghostlight.branch: buy_water_with_wear
    // ghostlight.intent: protect_immediate_service_by_spending_reserve_and_future_repair_integrity
    ~ water_margin = water_margin + 2
    ~ reserve_crystal = reserve_crystal - 1
    ~ repair_integrity = repair_integrity - 2
    ~ relief_priority = relief_priority - 1
    Talla opens Engine Three past the black catalogue notch. Its rod drives hard enough to throw warm spray through the grate.

    The terrace gauge steadies. A hairline of blue light crawls along the backup pump's hottest gold channel.

    Osric writes OVERDUTY beside the claim. He does not sound pleased, but assessors are trained until pleasure cannot be entered as evidence.
    -> cost_fold

=== cost_fold ===
// ghostlight.scene: surety_cost_arrives
// ghostlight.fold: failure_reaches_the_district
By the time the copper refusal plate lies on the table, the public landing is full.

A bakery porter waits with a wheeled trough. Clinic runners hold stoppered cans. Laundry workers have left wet sheets cooling in carts. Householders stand behind the yellow line with vessels and the disciplined expressions of people watching experts discover that thirst is interdisciplinary.

{ecology_evidence >= 2: The ridged shells or matching scratches give the adaptive pattern a physical record. The exclusion is no longer merely a word Osric brought with him.}
{ecology_evidence <= 0: No specimen remains. The surety book contains ordinary vermin and the drain contains everything that might have argued.}
{claim_strength >= 3: Osric has a covered cause, shared custody, and enough repair order to request a matched return assembly.}
{claim_strength <= 0: The copper refusal plate is ready for Talla's seal.}
{public_record >= 3: Poma's outward-facing slate binds water, debt, cause, and custody into one public account.}
{public_record <= 1: The only complete account remains inside Osric's book and Talla's workshop.}
{workshop_credit <= 1: Recording an excluded cause has made other workshops cautious about joining Talla's order.}
{reserve_crystal <= 0: The emergency locker is empty. It continues to have a lock, because institutions enjoy preserving the shape of a resource.}
{repair_integrity <= 1: Engine Three carries the district on a stressed channel while Engine One sits isolated.}
{relief_priority <= 0: An unsealed overrun has pushed the cistern's relief freight behind cleaner claims.}
{household_burden >= 3: Two emergency charges already sit on the public slate before anyone has purchased a replacement part.}

Osric explains the four costs without drama. That is what makes them sound expensive.

The consortium can send a matched assembly on a covered claim. A district moot can pledge other workshop seals. The winter reserve can buy a relief load if the lift office grants priority. A delving party can trace the mites through the intake and recover compatible material through contested space.

Talla owns one seal. She can choose which cost to place beneath it.

-> payment_choice

=== payment_choice ===
// ghostlight.choice_layer: allocate_failure_cost
+ [Sign the ordinary-vermin declaration and call the consortium's matched assembly.]
    // ghostlight.action: sign_claim
    // ghostlight.branch: accept_covered_cause
    // ghostlight.intent: restore_service_fast_by_binding_the_workshop_to_the_insurer_s_classification
    {claim_strength >= 3 && ecology_evidence <= 1 && repair_integrity >= 2:
        -> ending_claim_paid
    - else:
        -> ending_claim_refused
    }
+ [Leave the refusal visible and carry Poma's slate to a district moot of sealed workshops.]
    // ghostlight.action: widen_jurisdiction
    // ghostlight.branch: assess_sealed_workshops
    // ghostlight.intent: distribute_the_repair_cost_to_civic_owners_instead_of_water_connections
    {public_record >= 3 && workshop_credit >= 1:
        -> ending_workshop_assessment
    - else:
        -> ending_moot_failure
    }
+ [Break the winter-reserve seal and buy an emergency array plus a relief freight slot.]
    // ghostlight.action: spend_strategic_reserve
    // ghostlight.branch: buy_emergency_repair
    // ghostlight.intent: preserve_water_now_by_moving_the_shortage_into_winter_heat_and_freight
    {reserve_crystal >= 2 && relief_priority >= 2:
        -> ending_reserve_repair
    - else:
        -> ending_reserve_shortfall
    }
+ [Post a delving contract to trace the mites and recover compatible gold through the intake galleries.]
    // ghostlight.action: commission_delving_party
    // ghostlight.branch: pay_for_bounded_contest
    // ghostlight.intent: preserve_the_ecological_warning_and_seek_material_through_a_contested_route
    {ecology_evidence >= 2 && water_margin >= 1 && workshop_credit >= 1:
        -> ending_delving_contract
    - else:
        -> ending_delving_delay
    }

=== ending_claim_paid ===
// ghostlight.ending_label: covered_claim_success
// ghostlight.training_hook: insurance_language_buys_speed_and_blindness
Osric stamps ORDINARY VERMIN. Talla seals beneath it.

The life-service freight mark moves the cistern's matched assembly ahead of a solvent foundry order. By second shift, Neri bolts the new return plate into Engine One. Water climbs the terrace gauge.

The service note remains. The replacement's uncovered share joins the water assessment.

{household_burden >= 2: Poma posts the higher connection charge beside the restored gauge. Relief and resentment arrive together and begin sharing a bench.}
{public_record >= 3: Poma also leaves the original shell sketch visible. The claim is settled; the doubt is not.}

Talla has bought water at the price the contract knew how to name.
-> END

=== ending_claim_refused ===
// ghostlight.ending_label: covered_claim_cost
// ghostlight.training_hook: weak_or_false_claim_compounds_default
Talla signs. Osric does not.

The groove measurements, the stressed backup, or the broken inspection custody leaves him enough reason to stamp the copper refusal plate instead. The consortium moves the matched assembly behind covered orders. The service note continues accruing its assessment while the pump remains dark.

Poma writes CLAIM REFUSED below the falling water clock. A failed cistern can owe for the machinery that failed and the water it no longer supplies. Finance has achieved simultaneity.
-> END

=== ending_workshop_assessment ===
// ghostlight.ending_label: workshop_assessment_success
// ghostlight.training_hook: jurisdiction_follows_material_consequence
Poma carries one end of the slate. Talla carries the other and keeps her seal visible.

The district moot gathers bakery ovens, lift repair, clinic stores, laundries, and the cistern into one argument. Sealed workshops pledge shares of the repair order because all five lose work when the taps fail. The household surcharge is held at its current line.

No one calls this charity. The workshops are paying to keep their own district capable of buying, healing, washing, and arriving.

Neri remains outside the franchise and inside the repair crew. The distinction survives the emergency, which is precisely why it bites.
-> END

=== ending_moot_failure ===
// ghostlight.ending_label: workshop_assessment_cost
// ghostlight.training_hook: public_claim_without_credit_cannot_move_parts
The slate reaches the moot before the seals do.

With an incomplete public record or Talla's credit already weakened by the exclusion, neighbouring Masters will witness the fault but not pledge their orders behind it. The lift office cannot rank testimony as freight.

The district returns to Cistern House Nine with more agreement and no matched part. Poma adds a temporary household assessment because the water carts still require crews, rails, and crystal.
-> END

=== ending_reserve_repair ===
// ghostlight.ending_label: reserve_repair_success
// ghostlight.training_hook: immediate_service_moves_scarcity_into_winter
Talla breaks the winter-reserve seal. Blue crystal light fills the locker grille.

Osric's joint inspection gives the lift office enough evidence to displace a decorative-stone wagon. The emergency array arrives under guard. Neri and Talla repair Engine One before the upper tanks empty.

Water runs. The district heat reserve enters winter one allotment short.

Poma posts both gauges. Nobody on the landing is permitted the comfort of calling the repair free.
-> END

=== ending_reserve_shortfall ===
// ghostlight.ending_label: reserve_repair_cost
// ghostlight.training_hook: strategic_reserve_is_not_a_bottomless_fund
The seal breaks over an insufficient blue glow.

Earlier temporary flow consumed the useful crystal, or the relief claim lacks enough priority to move a matched array ahead of cleaner cargo. The district has spent the reserve politically before it can spend it physically.

Talla reseals an emptier locker. The workshop still owes the pump note. The terraces still owe the water assessment. The lift office sends carts instead of parts.
-> END

=== ending_delving_contract ===
// ghostlight.ending_label: delving_contract_success
// ghostlight.training_hook: material_recovery_preserves_bounded_contest
Talla seals a contract for a small party, not a bore.

Neri gives the delvers the living mites, the copied groove map, and the intake route. The party may retreat, bargain with the chamber, abandon the salvage, or return with compatible gold and a better account of where the insects learned the rune.

The terrace accepts rationing while they are below. Workshop credit buys wages and provisions instead of an insurer's certainty.

At dusk, the pump is still dark. The cause is still allowed to answer back.
-> END

=== ending_delving_delay ===
// ghostlight.ending_label: delving_contract_cost
// ghostlight.training_hook: truthful_method_still_requires_time_evidence_and_credit
Talla posts the contract. No competent party takes it before the water clock enters red.

There is too little preserved evidence to price the route, too little workshop credit to provision it, or too little stored water to wait for a contested return. Refusing the industrial shortcut does not manufacture a safe alternative on command.

Poma begins household rationing. Neri keeps Engine One isolated. Talla's seal remains attached to a truthful plan that cannot yet move water.
-> END
