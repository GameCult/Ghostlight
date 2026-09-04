// ghostlight.artifact_id: tangle_reserve_house_covenant_v0_branch_fold_v0
// ghostlight.fixture_id: tangle-reserve-house-covenant-v0
// ghostlight.scene_id: tangle-reserve-house-covenant-v0.cistern-house-nine-renewal
// ghostlight.final_ink_path: examples/ink/delvehold/tangle-reserve-house-covenant-v0.branch-and-fold.v0.ink

VAR public_witness = 1
VAR covenant_evidence = 1
VAR district_water_margin = 2
VAR workshop_debt = 2
VAR open_standard_case = 0
VAR factor_discretion = 2
VAR seal_independence = 2
VAR consortium_lockin = 0
VAR labour_support = 1

-> start

=== start ===
// ghostlight.scene: cistern_house_nine_autumn_establishing
By second bell, Cistern House Nine has already pushed a morning's water up the terraces.

The workshop occupies two levels beside a warm underground sea: a street landing and dry rune gallery above, a wet intake chamber below, and three brass-and-iron pump engines between them behind a safety rail. A locked conduit arch brings municipal crystal through the gallery wall. Beyond the rail, the district gauge rises by chalk-widths beside rows of copper cans waiting to be filled.

Once each month the pumps keep working while Master Hessa Cairn proves to the terrace reserve house that they deserve to keep working next month.

-> covenant_table

=== covenant_table ===
// ghostlight.scene: covenant_table_routine
Hessa lays her workshop's civic seal beside the maintenance book. The seal gives Cistern House Nine one vote and makes Hessa answerable for the pumps. It does not make the reserve house charitable.

Factor Tovan Marr has brought a lacquered tally case, three loan schedules, and the expression of a dwarf who has never been surprised by a column in public. His reserve house holds emergency crystal, replacement parts, and credit for this web of cisterns, lifts, kitchens, and workshops.

Apprentice Orsa Rill reads pressure floats. Journeyworker Brin Olt checks spare collars against a wooden gauge. Landing clerk Dema Sorn keeps the district's copy of every level and callout on the public side of the yellow line.

The waste-heat kettle begins to whistle.

"Unmetered thermal leakage," Tovan says.

"Tea," Brin says. "We repair that first."

This is routine. Routine is when institutions reveal which jokes they have learned to bill for.

-> renewal_choice

=== renewal_choice ===
// ghostlight.choice_layer: ordinary_covenant_review
+ [Fit Brin's wooden gauge to every locally made replacement collar.]
    // ghostlight.action: inspect_parts
    // ghostlight.branch: prime_open_compatibility
    // ghostlight.intent: prove_that_interchangeable_local_parts_can_keep_the_pumps_repairable
    ~ open_standard_case = open_standard_case + 2
    ~ covenant_evidence = covenant_evidence + 1
    ~ district_water_margin = district_water_margin - 1
    ~ labour_support = labour_support + 1
    Hessa carries the gauge from collar to collar while Brin calls the fit. Two seat cleanly. One needs filing. Orsa records all three instead of improving the result with handwriting.

    Tovan taps the delayed maintenance column. "Inspection time is also a cost."

    "So is discovering the wrong bolt circle during a flood," Brin says.

    The pumps keep the terraces supplied, but the spare crew loses an hour to proof.
    -> renewal_fold
+ [Let Tovan copy the private maintenance book into his tally case.]
    // ghostlight.action: transfer_record
    // ghostlight.branch: prime_factor_confidence
    // ghostlight.intent: strengthen_the_claim_by_giving_the_factor_a_complete_audit_trail
    ~ covenant_evidence = covenant_evidence + 2
    ~ factor_discretion = factor_discretion + 1
    ~ seal_independence = seal_independence - 1
    Hessa turns the maintenance book toward Tovan.

    His copy-runes drink dates, failures, part marks, and repair costs into the lacquered case. They also take the names of workers who found each fault.

    Orsa watches her own apprentice mark vanish into a creditor's blue light.

    "A complete record lowers uncertainty," Tovan says.

    "Whose?" Dema asks from beyond the rail.
    -> renewal_fold
+ [Carry the current reserve entitlement to Dema's public slate before reviewing it.]
    // ghostlight.action: publish_record
    // ghostlight.branch: prime_public_entitlement
    // ghostlight.intent: make_the_reserve_promise_legible_to_the_district_that_paid_for_it
    ~ public_witness = public_witness + 2
    ~ factor_discretion = factor_discretion - 1
    ~ seal_independence = seal_independence + 1
    Hessa sets the covenant strip on the slate ledge. Dema chalks its useful nouns: three pumps, two days' emergency crystal, one service interruption, current through first frost.

    She leaves the conditions below them in smaller script, which is where conditions prefer to live.

    Tovan comes to the yellow line and corrects the script until both sides can read it.
    -> renewal_fold

=== renewal_fold ===
// ghostlight.fold: routine_finance_before_supply_pressure
The engines thump below the paperwork. Orsa calls the floats; Brin answers with valve positions; Dema marks the district gauge. Hessa and Tovan make the water legible to two institutions that count it differently.

{open_standard_case >= 2: Three local replacement collars lie beside Brin's gauge, their different makers' marks attached to one working bolt circle.}
{covenant_evidence >= 3: Tovan's tally case glows with enough maintenance history to make ignorance an expensive pose.}
{public_witness >= 3: The reserve entitlement stands on the public slate where the landing queue can read what its earlier levies purchased.}
{factor_discretion >= 3: Tovan now holds the most complete copy of the workshop's risk history, and his private judgment has grown heavier.}
{factor_discretion <= 1: The published entitlement leaves Tovan less room to turn interpretation into a closed door.}
{seal_independence >= 3: Hessa's seal lies beside a covenant the district can inspect, not inside Tovan's tally case.}
{district_water_margin <= 1: The inspection delay has eaten into the upper cisterns' working margin. Dema adds a small downward hook to the gauge line.}

Then Dema receives a black-edged route notice from the street landing.

-> supply_notice

=== supply_notice ===
// ghostlight.scene: deep_company_supply_notice
She brings it through the yellow line only far enough for Hessa to take custody.

A Deep Company has suspended the crystal train after creatures crossed three inhabited galleries to break its standardized substations. The notice offers no diagnosis. It offers half the contracted shipment, an unknown reopening date, and a paragraph explaining that neither fact is technically a breach until winter.

At the locked conduit arch, a reserve-house delivery crew has already lowered one iron crystal coffer onto the dry gallery. Nobody enters. The coffer bears Tovan's house mark across its lid and a second seal from a Forge Consortium.

Tovan opens his tally case. "The house can release this today. The replacement schedule changes with it."

Brin sets down his wooden gauge. "There it is. Water with a preferred bolt circle."

-> notice_response_choice

=== notice_response_choice ===
// ghostlight.choice_layer: supply_interruption_response
+ [Set Hessa's civic seal on the old covenant strip, apart from the sealed coffer.]
    // ghostlight.action: place_seal
    // ghostlight.branch: assert_current_covenant
    // ghostlight.intent: distinguish_the_existing_public_promise_from_the_new_supplier_terms
    ~ covenant_evidence = covenant_evidence + 1
    ~ seal_independence = seal_independence + 1
    ~ factor_discretion = factor_discretion - 1
    Hessa places her seal on the current strip without stamping it.

    "This is the promise the district funded," she says. "The coffer arrived later."

    Tovan looks at the two seals separated by one handspan of stone. Everyone on the landing can now see there are two decisions pretending to be one delivery.
    -> terms_fold
+ [Measure the upper-terrace margin from Dema's gauge before discussing signatures.]
    // ghostlight.action: inspect_public_gauge
    // ghostlight.branch: count_water_before_debt
    // ghostlight.intent: learn_how_long_the_district_can_withstand_negotiation
    ~ district_water_margin = district_water_margin + 1
    ~ public_witness = public_witness + 1
    Hessa crosses to the public gauge. Dema shows her household draw, bakery draw, and the lift cistern's minimum fire reserve.

    "Two days if the laundries close," Dema says. "Three if everyone lies about bathing."

    The landing queue does not laugh. It does begin doing the arithmetic aloud.
    -> terms_fold
+ [Have Brin compare the Consortium collar packed above the crystal to his wooden gauge.]
    // ghostlight.action: inspect_supplier_part
    // ghostlight.branch: test_supplier_compatibility
    // ghostlight.intent: discover_whether_the_financial_condition_is_a_real_engineering_requirement
    ~ open_standard_case = open_standard_case + 2
    ~ labour_support = labour_support + 1
    ~ district_water_margin = district_water_margin - 1
    Brin breaks only the outer freight tie. He does not touch either seal. A polished feed collar sits in the coffer's top rack, its six-bolt pattern visible through wire mesh.

    His gauge fits four bolts and refuses the other two.

    "Compatible," Tovan says, reading the catalogue plate.

    "With the engine they would like us to own," Brin says.
    -> terms_fold
+ [Let the sealed coffer wait beside the conduit arch while the pumps keep running.]
    // ghostlight.action: withhold_acceptance
    // ghostlight.branch: stage_emergency_stock
    // ghostlight.intent: preserve_immediate_delivery_without_accepting_custody_or_terms
    ~ factor_discretion = factor_discretion + 1
    ~ consortium_lockin = consortium_lockin + 1
    Hessa points to a dry square inside the hoist chain but outside the pump rails.

    The coffer waits there, close enough to promise relief and too sealed to provide it. Tovan's crew withdraws up the street stair. Custody remains with the reserve house.

    The pumps run another hour on current stock while the unopened future occupies floor space.
    -> terms_fold

=== terms_fold ===
// ghostlight.fold: covenant_terms_become_public_pressure
Tovan lays a new brass covenant strip between the old promise and Hessa's seal.

The reserve house will release the crystal if Cistern House Nine accepts a Forge Consortium feed retrofit, house inspection rights, first claim on workshop service revenue, and a longer Deep Company supply commitment when the route reopens. The amendment does not purchase Hessa's civic seal. It merely makes everything the seal maintains harder to finance without consent.

{open_standard_case >= 3: Brin's gauge and the half-fitting Consortium collar make "compatibility" look like a decision instead of a property.}
{labour_support >= 2: Brin and Orsa stand beside the locally made spares with the quiet solidarity of people who expect to repair whatever gets signed.}
{covenant_evidence >= 3: The old maintenance record supports a claim that Cistern House Nine met the covenant before the supply route failed.}
{public_witness >= 3: Dema's slate now holds entitlement, route notice, gauge margin, and the names of both seals.}
{workshop_debt >= 3: The first-claim clause would leave the workshop carrying old repairs and new finance in the same bucket.}
{seal_independence <= 1: Too much of the workshop's record and remedy now lives inside Tovan's interpretation.}
{consortium_lockin >= 1: The sealed coffer has made the Consortium pattern physically present before anyone votes or signs.}

Tovan keeps one palm flat on the amendment. "I can price a bounded exception. I cannot promise the house will like it."

"Houses are stone," Dema says. "They survive dislike."

-> leverage_choice

=== leverage_choice ===
// ghostlight.choice_layer: reserve_house_leverage
+ {public_witness >= 3} [Ask Tovan to read an approval or refusal against the published entitlement.]
    // ghostlight.action: compel_public_answer
    // ghostlight.branch: spend_public_witness
    // ghostlight.intent: narrow_factor_discretion_by_attaching_the_decision_to_public_evidence
    ~ public_witness = public_witness + 1
    ~ covenant_evidence = covenant_evidence + 2
    ~ factor_discretion = factor_discretion - 2
    ~ workshop_debt = workshop_debt + 1
    Tovan reads the old promise aloud. He reads the route notice. He reads the clause allowing emergency release when a covered service loses supply without failing inspection.

    Then he reads the price of invoking it: the existing loan extends through another winter.

    Dema writes both sentences at the same size.
    -> final_threshold
+ {open_standard_case >= 2} [Offer a seven-day bridge: current fittings, daily inspection, and no claim on Engine Two.]
    // ghostlight.action: propose_bounded_covenant
    // ghostlight.branch: offer_repairable_bridge
    // ghostlight.intent: buy_time_without_turning_one_supplier_pattern_into_permanent_law
    ~ open_standard_case = open_standard_case + 1
    ~ labour_support = labour_support + 1
    ~ district_water_margin = district_water_margin + 1
    ~ workshop_debt = workshop_debt + 1
    ~ seal_independence = seal_independence + 1
    Hessa draws a line around seven days, one coffer, daily public gauge readings, and the two pumps whose collars Brin's gauge can prove.

    Engine Two stays outside the collateral schedule. Tovan dislikes the gap because it has edges he can be blamed for.

    "A bounded exception," Hessa says. "You just told me you could price one."
    -> final_threshold
+ [Accept the bundled retrofit, inspection, revenue, and supply clauses.]
    // ghostlight.action: accept_terms
    // ghostlight.branch: accept_consortium_bundle
    // ghostlight.intent: secure_immediate_crystal_by_accepting_long_term_supplier_control
    ~ district_water_margin = district_water_margin + 2
    ~ workshop_debt = workshop_debt + 2
    ~ consortium_lockin = consortium_lockin + 3
    ~ seal_independence = seal_independence - 2
    ~ factor_discretion = factor_discretion + 1
    Hessa turns the amendment until its seal notch faces her.

    Tovan relaxes by one professional fraction. The coffer can open as soon as Hessa stamps.

    Brin does not argue. He begins listing which local spares will become scrap, because somebody should count the quiet casualties before the water starts.
    -> final_threshold
+ [Give Dema copies and call the affected workshop seals to a district moot.]
    // ghostlight.action: widen_jurisdiction
    // ghostlight.branch: summon_district_moot
    // ghostlight.intent: move_a_public_reserve_dispute_to_the_seals_and_districts_bound_by_it
    ~ public_witness = public_witness + 2
    ~ factor_discretion = factor_discretion - 1
    ~ seal_independence = seal_independence + 1
    ~ district_water_margin = district_water_margin - 1
    Dema takes old covenant, new amendment, route notice, and gauge copy in separate hands, then recruits Orsa for the fourth item.

    The nearest workshop bells begin calling seals toward the landing. The coffer remains closed. So does every tap fed by water not yet pumped.
    -> final_threshold

=== final_threshold ===
// ghostlight.scene: coffer_covenant_and_gauge
The gallery now contains the whole argument.

Three pumps work behind the brass rail. The public gauge measures how long negotiation can continue. The iron coffer waits by the locked conduit arch under reserve-house and Consortium seals. Brin's wooden gauge lies beside local collars and the polished replacement. Dema's slate carries whatever the public was allowed to witness. Hessa's civic seal remains one object, worth one vote, capable of binding one workshop to a great deal of future.

{district_water_margin >= 4: The upper terraces have enough measured water to make refusal possible without pretending it is free.}
{district_water_margin <= 1: The gauge has entered its red band. Every principled minute now arrives upstairs as an empty vessel.}
{covenant_evidence >= 4: The old covenant, maintenance record, and supply notice form a claim Tovan must either honor or deny by name.}
{covenant_evidence <= 2: The workshop's claim remains plausible and thin, which is a lender's favorite texture.}
{factor_discretion <= 1: Tovan's answer has become public enough that the house, not merely its factor, will own it.}
{factor_discretion >= 4: The decisive record and the available remedy are concentrated in Tovan's tally case.}
{open_standard_case >= 3: Two working local collars and one ill-fitting Consortium collar make a bounded repair standard materially defensible.}
{labour_support >= 3: Orsa and Brin have committed their standing and repair time to the bounded bridge.}
{seal_independence >= 4: Hessa still controls what her seal binds, even while the district can inspect the choice.}
{seal_independence <= 1: The amendment has not bought Hessa's vote; it has surrounded the vote with unaffordable alternatives.}
{workshop_debt >= 4: Future service revenue is already crowded with past repairs and the price of today's coffer.}
{consortium_lockin >= 3: The Consortium retrofit schedule is ready to become the workshop's only insurable future.}

Hessa must decide which institution gets to call the water saved.

-> final_choice

=== final_choice ===
// ghostlight.choice_layer: covenant_resolution
+ [Claim the emergency crystal under the old covenant and leave the retrofit unsigned.]
    // ghostlight.action: claim_reserve
    // ghostlight.branch: enforce_existing_promise
    // ghostlight.intent: make_the_reserve_house_honor_the_service_risk_it_already_insured
    {covenant_evidence >= 4 && factor_discretion <= 2:
        -> ending_old_covenant_honored
    - else:
        -> ending_old_covenant_refused
    }
+ [Stamp only the seven-day repairable bridge and keep Engine Two outside the claim.]
    // ghostlight.action: stamp_bounded_terms
    // ghostlight.branch: bind_repairable_bridge
    // ghostlight.intent: preserve_water_and_local_repair_capacity_through_a_narrow_temporary_covenant
    {open_standard_case >= 3 && labour_support >= 2 && seal_independence >= 3:
        -> ending_bounded_bridge
    - else:
        -> ending_bounded_bridge_cost
    }
+ [Stamp the Consortium amendment and break both seals on the coffer.]
    // ghostlight.action: accept_and_open
    // ghostlight.branch: bind_supplier_future
    // ghostlight.intent: protect_immediate_service_by_accepting_supplier_lockin_and_debt
    {district_water_margin >= 3 && consortium_lockin >= 3:
        -> ending_consortium_water
    - else:
        -> ending_consortium_cost
    }
+ [Carry the strips and Hessa's unstamped seal to the district moot; ration until it answers.]
    // ghostlight.action: carry_dispute
    // ghostlight.branch: let_affected_seals_decide
    // ghostlight.intent: widen_the_decision_to_the_constituencies_whose_reserve_and_services_are_bound
    {public_witness >= 4 && district_water_margin >= 2:
        -> ending_moot_with_margin
    - else:
        -> ending_moot_under_thirst
    }

=== ending_old_covenant_honored ===
// ghostlight.ending_label: old_covenant_honored
// ghostlight.training_hook: public_evidence_constrains_financial_discretion
Tovan stamps release under the old terms. The house keeps its first claim only where it already had one. Hessa signs the delivery record, not the retrofit.

The coffer opens. Orsa and Brin carry crystal to the feed while Dema writes the added winter of debt beside the restored gauge.

Water climbs. The promise holds. It is not forgiveness; it is an institution being made to remember what it sold.
-> END

=== ending_old_covenant_refused ===
// ghostlight.ending_label: old_covenant_refused
// ghostlight.training_hook: thin_claim_exposes_factor_authority
Tovan refuses the claim in the narrow language available to a complete tally case and an incomplete public record.

The coffer remains shut. Hessa keeps her seal. The workshop keeps its debt. The terraces keep none of the water those facts resemble on paper.

Dema records the refusal under Tovan's name. By evening it will reach a moot, but the district gauge is already writing the first draft in red.
-> END

=== ending_bounded_bridge ===
// ghostlight.ending_label: bounded_bridge_honored
// ghostlight.training_hook: repairable_standard_buys_temporary_autonomy
Tovan prices seven days. Hessa stamps seven days. Dema draws a box around the end date large enough that nobody can later mistake it for decoration.

The reserve-house seal breaks. The Consortium seal stays intact around its polished collar. Local fittings take the crystal, and Brin assigns Orsa the first daily inspection.

The bridge costs interest and labour. It also ends before it can quietly become a constitution.
-> END

=== ending_bounded_bridge_cost ===
// ghostlight.ending_label: bounded_bridge_unsupported
// ghostlight.training_hook: bounded_terms_need_material_and_social_support
Hessa stamps a boundary the workshop has not proved it can maintain.

One local collar fails the gauge. Brin cannot promise daily inspection with the current crew. Tovan releases only a fraction of the coffer and writes the rest as unsecured risk.

The pumps alternate through the night. So do the workers. A narrow covenant without parts or hands is merely a wider kind of exhaustion.
-> END

=== ending_consortium_water ===
// ghostlight.ending_label: consortium_bundle_secures_service
// ghostlight.training_hook: immediate_relief_creates_vendor_constituency
Hessa stamps. Tovan breaks the house seal; Hessa breaks the Consortium seal. Blue crystal light fills the gallery before the upper cisterns enter red.

Water rises. The landing cheers because thirst is not obliged to maintain theoretical purity.

Brin hangs the wooden gauge above his bench. Next month the new collar will not fit it. Next year the workers trained on that collar will have practical reasons to defend the contract that narrowed their craft.
-> END

=== ending_consortium_cost ===
// ghostlight.ending_label: consortium_bundle_arrives_late
// ghostlight.training_hook: debt_does_not_retroactively_restore_capacity
Hessa stamps every clause. The coffer opens. The crystal is real.

So is the red gauge. Too much water has already left the upper cisterns for one delivery to restore service without rationing. Tovan's schedule begins today anyway; debt has excellent punctuality.

The district receives the promised technology as empty cans move uphill beside it.
-> END

=== ending_moot_with_margin ===
// ghostlight.ending_label: district_moot_with_time
// ghostlight.training_hook: widened_jurisdiction_preserves_consent_at_a_cost
Hessa leaves the coffer sealed and carries her seal to the landing. Dema carries the records. Orsa carries the public gauge board because evidence, unlike authority, sometimes needs two hands and an apprentice.

Affected Masters arrive from a bakery, a liftworks, a laundry, and a clinic. Households count cistern days while the seals amend the reserve mandate in public.

The pumps slow, but do not stop. The district has purchased time with stored water and spends it governing.
-> END

=== ending_moot_under_thirst ===
// ghostlight.ending_label: district_moot_under_thirst
// ghostlight.training_hook: consent_without_margin_bears_visible_cost
The workshop bells gather seals after the gauge enters red.

The moot is legitimate. The thirst is also legitimate and less patient. Hessa does not let Tovan turn either fact into the other's cancellation.

While Masters argue over the reserve they funded, Orsa and Brin close two pumps to protect them from running dry. Dema turns the public slate outward. The empty cans are already a constituency.
-> END
