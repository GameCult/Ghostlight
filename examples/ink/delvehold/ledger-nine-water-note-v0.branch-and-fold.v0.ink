// ghostlight.artifact_id: ledger_nine_water_note_v0_branch_fold_v0
// ghostlight.fixture_id: ledger-nine-water-note-v0
// ghostlight.scene_id: ledger-nine-water-note-v0.collar-failure-and-credit-choice
// ghostlight.final_ink_path: examples/ink/delvehold/ledger-nine-water-note-v0.branch-and-fold.v0.ink

VAR crystal_stock = 3
VAR district_buffer = 1
VAR water_flow = 1
VAR note_pressure = 2
VAR warranty_status = 2
VAR public_record = 1
VAR local_part = 0
VAR route_proof = 0
VAR worker_burden = 1
VAR district_cost = 0

-> start

=== start ===
// ghostlight.scene: nine_water_note_opening
Cistern House Nine keeps two morning books.

One is the service book, which records oil, pressure, fractures, null tests, and every noise the pumps make when they would prefer not to be discussed. The other is the water note, which records what the district still owes Basalt Crown Arrays for the privilege of having pumps capable of making those noises.

The public workshop sits between a terraced Hold and its warm underground sea. A street landing opens into a dry rune gallery. Below a grated stair, three brass-and-iron engines draw black water through the wet intake chamber. On the gallery wall, a locked conduit arch brings municipal mana to the pumps in cold blue lines.

It is an hour before handover. Nothing has failed yet. This is when Cistern House Nine does its most ambitious pretending.

-> morning_people

=== morning_people ===
// ghostlight.scene: nine_water_note_people
Orsa Rill, pump-house apprentice, sets calibrated weights beside the low roller scale under the crystal bins. She has food, instruction, tools, appeal, and no civic seal. The distinction is particularly vivid on invoice day.

Master Hessa Cairn has the seal. It hangs from her belt while she checks the water-note statement against a public rate slate held by landing clerk Dema Sorn. The note is one payment late. Its red-cord amendment waits unopened beside Hessa's hand.

Journeyworker Brin Olt kneels by a scarred drive collar from Engine Two. Basalt Crown sells a certified replacement already packed in a gray iron crate behind the safety rail. The consortium also owns first claim on that part until the arrears are settled, which is an efficient way for a spare to be present and unavailable at once.

-> routine_terms

=== routine_terms ===
// ghostlight.scene: nine_water_note_terms
Today's crystal delivery sits in four squat copper-bound bins. The route offices have deducted passage share at every lift and gate. The Reserve House has added winter priority. Dema's tickets say the workshop paid for four full bins. Orsa's eyes suggest the bins have not read the tickets.

Hessa taps the rate slate. "Ten hours in the upper header tanks if we stop. Two days below if nobody becomes imaginative with a bath."

"And if we keep running?" Orsa asks.

"Then the pumps continue their fine tradition of costing less than thirst and more than money."

Before handover, Orsa has time to make one fact harder to hide.

-> invoice_choice

=== invoice_choice ===
// ghostlight.choice_layer: ordinary_invoice_routine
+ [Roll every crystal bin across the low scale while Dema records its route seals.]
    // ghostlight.action: weigh_delivery
    // ghostlight.branch: prime_route_proof
    // ghostlight.intent: separate_route_deductions_from_local_consumption
    ~ route_proof = route_proof + 2
    ~ public_record = public_record + 1
    ~ worker_burden = worker_burden + 1
    Orsa levers each bin onto the scale's iron rollers and walks it across the brass balance plate. Brin reads the weights; Dema copies them beside the gate seals.

    The fourth bin is short by exactly the combined passage shares and then short again by an amount no route ticket admits.

    "A very disciplined theft," Dema says.

    "An arithmetic error," Hessa says.

    "Those are disciplined. They dress for work."
    -> routine_fold
+ [Help Brin recut the discarded drive collar to the old workshop gauge.]
    // ghostlight.action: shape_part
    // ghostlight.branch: prime_local_repair
    // ghostlight.intent: create_a_nonconsortium_repair_option_before_failure
    ~ local_part = local_part + 2
    ~ warranty_status = warranty_status - 1
    ~ worker_burden = worker_burden + 1
    Brin heats the scarred collar in a charcoal brazier. Orsa files its inner teeth against an old brass gauge whose measurements predate Basalt Crown's service catalogue.

    The recut piece is not certified. It is, however, metal shaped to fit metal, a category of magic the consortium has not yet managed to invoice separately.

    Hessa checks the fit and says, "Keep it off the engine until I decide whether water or warranty is lying more expensively."
    -> routine_fold
+ [Chalk the note balance, warranty exclusions, and next payment date beside the terrace gauge.]
    // ghostlight.action: publish_terms
    // ghostlight.branch: prime_public_terms
    // ghostlight.intent: make_the_districts_obligation_visible_before_a_crisis
    ~ public_record = public_record + 2
    ~ note_pressure = note_pressure + 1
    Dema holds the slate steady while Orsa copies the figures where the morning queue will see them: one late payment, one packed replacement collar under consortium claim, and a list of exclusions longer than the clean-water line.

    Hessa reads it twice.

    "You left out nothing."

    "I was trained under supervision."

    The note has not grown larger. It has merely become public-sized.
    -> routine_fold
+ [Spend one crystal measure filling the upper public header tank before reconciling the invoice.]
    // ghostlight.action: fill_reserve
    // ghostlight.branch: prime_district_buffer
    // ghostlight.intent: buy_household_time_with_scarce_fuel
    ~ crystal_stock = crystal_stock - 1
    ~ district_buffer = district_buffer + 2
    ~ water_flow = water_flow + 1
    Orsa opens Engine One long enough to push a measured rise into the upper public tank. The wall gauge climbs. Somewhere above, float bells ring through terrace pipes.

    Dema writes the spent crystal against emergency storage.

    Hessa says, "The invoice will object."

    "It has access to water," Orsa says. "Let it carry some down."
    -> routine_fold

=== routine_fold ===
// ghostlight.fold: routine_books_before_break
The service book lies open beside the water note. The gray replacement crate waits behind the rail with Basalt Crown's red cord intact. Four crystal bins stand beneath the conduit arch. The terrace gauge holds above its red band.

{route_proof >= 2: Dema's slate now ties each short crystal weight to a specific rail, lift, or gate seal. The unexplained shortage has somewhere to stand.}
{local_part >= 2: The recut collar cools beside Brin's old workshop gauge, visibly fitted and visibly uncertified.}
{public_record >= 3: People gathering on the landing can read the arrears and exclusions before anyone translates them into reassurance.}
{district_buffer >= 3: The upper header-tank marker sits two bands higher, buying the steepest streets several ordinary hours.}
{crystal_stock <= 2: One copper-bound bin is already open and light. The district has purchased time by making the next decision poorer.}
{worker_burden >= 2: Orsa's first hour of paid work has acquired a second hour that the ledger calls preparation.}

Brin turns Engine Two by hand for the morning test.

-> collar_failure

=== collar_failure ===
// ghostlight.scene: nine_water_note_collar_failure
The drive collar parts with a noise like a spoon dropped at a funeral.

Nothing explodes. Engine Two simply loses the useful relationship between its turning shaft and its pump rod. The rod stops above the warm mist. The district gauge begins to fall with bureaucratic composure.

Dema opens the delivery pouch that arrived with the crystal. Seven Seals Mutual has refused the workshop's pending machinery claim. The refusal cites an undocumented blue reroute entered in the last service book: adaptive geomantic feedback, expressly excluded.

The same pouch contains Basalt Crown's remedy. Hessa may break the red cord on the certified collar if she seals an accelerated amendment pledging the workshop's winter-priority crystal allotment. Otherwise, the part can remain safe, dry, and socially useless in its crate.

-> failure_choice

=== failure_choice ===
// ghostlight.choice_layer: failed_engine_response
+ {local_part >= 2} [Carry Brin the recut collar and ask Hessa to witness an uncertified installation.]
    // ghostlight.action: install_local_part
    // ghostlight.branch: use_local_repair
    // ghostlight.intent: restore_water_with_local_skill_at_warranty_cost
    ~ water_flow = water_flow + 2
    ~ warranty_status = 0
    ~ worker_burden = worker_burden + 1
    ~ public_record = public_record + 1
    Orsa brings the warm recut collar to Engine Two. Brin seats it against the old gauge. Hessa does not bless the repair with a catalogue number; she records both their names in the service book.

    The shaft catches. The pump rod descends. Water enters the pipe with a rough knock and then a steady one.

    Dema draws a line through the word certified, carefully enough that nobody can mistake honesty for neatness.
    -> cost_fold
+ [Set Basalt Crown's amendment under Hessa's seal hand and cut the red cord only after she stamps.]
    // ghostlight.action: accept_accelerated_note
    // ghostlight.branch: borrow_for_water
    // ghostlight.intent: restore_service_by_pledging_future_crystal_priority
    ~ water_flow = water_flow + 3
    ~ note_pressure = note_pressure + 3
    ~ warranty_status = warranty_status + 1
    ~ crystal_stock = crystal_stock - 1
    ~ public_record = public_record + 1
    Hessa reads the amendment aloud before she stamps it. Winter priority passes to Basalt Crown until the arrears and accelerated charge are paid.

    Orsa cuts the red cord. Brin lifts the certified collar from its felt blocks and fits it to Engine Two. The pump returns with the smooth, expensive confidence of a machine that has just purchased part of December.

    Dema posts the new payment line beneath the climbing water gauge.
    -> cost_fold
+ [Leave Two dead and drive Engines One and Three above their ordinary crystal draw.]
    // ghostlight.action: overload_remaining_pumps
    // ghostlight.branch: spend_crystal_reserve
    // ghostlight.intent: preserve_immediate_flow_by_burning_scarce_fuel_and_worker_attention
    ~ water_flow = water_flow + 2
    ~ crystal_stock = crystal_stock - 2
    ~ warranty_status = warranty_status - 1
    ~ worker_burden = worker_burden + 2
    ~ district_cost = district_cost + 1
    Orsa and Brin open the two sound engines together. Blue mana brightens in their channels. Their pressure floats rise past the green marks and stop just before Hessa says their names in the tone that ends experiments.

    The terrace gauge recovers. The crystal bins empty fast enough to be educational.

    Engine Three begins a thin whine that will require someone to remain beside it. Orsa already knows who the ledger means by someone.
    -> cost_fold
+ [Keep all wedges seated and carry the refusal notice to Dema's public slate.]
    // ghostlight.action: preserve_denial_record
    // ghostlight.branch: stop_and_publish
    // ghostlight.intent: preserve_evidence_and_contract_accountability_at_immediate_service_cost
    ~ public_record = public_record + 2
    ~ water_flow = water_flow - 2
    ~ district_buffer = district_buffer - 1
    ~ district_cost = district_cost + 2
    ~ note_pressure = note_pressure - 1
    Orsa seats the isolation wedges and gives Dema the refusal notice.

    Dema copies the exclusion beside the falling gauge: reported anomaly, excluded loss, certified replacement held against arrears.

    Nobody on the landing needs a lecture on systems. The upper pipe coughs dry while they are still reading.
    -> cost_fold

=== cost_fold ===
// ghostlight.fold: debt_water_and_work_arrive_together
Cistern House Nine now has one failed collar and several competing definitions of solvency.

{water_flow >= 4: Water strikes the outlet pipes hard enough to lift the terrace gauge out of red. The queue hears the result before it sees the account.}
{water_flow >= 2 && water_flow < 4: Partial flow reaches the lower terraces and climbs toward the upper ones slowly enough for every stair to become political.}
{water_flow <= 0: The pumps stay dark. The upper public spout gives one last metal cough.}

{note_pressure >= 5: The accelerated amendment lies under Hessa's fresh seal. Winter crystal priority now belongs first to the creditor.}
{note_pressure <= 1: No new debt has been signed, but the old payment remains and the dry hours are compounding elsewhere.}
{warranty_status <= 0: Basalt Crown's warranty is void in black chalk. The locally shaped collar is either independence or an uninsured future failure, depending on who sends the next letter.}
{crystal_stock <= 1: One shallow crystal bin remains. Heat, rail, and tomorrow's pumping have begun making claims on it without waiting for a moot.}
{public_record >= 4: Dema's outward-facing slate holds the note balance, delivery weights, refusal clause, pump state, and current water level in one public frame.}
{route_proof >= 2: The short delivery can be traced seal by seal to passage deductions and one unexplained loss after the final lift.}
{worker_burden >= 4: Brin's hands shake when he sets down the gauge. Orsa's shift board has no empty square before tomorrow.}
{district_cost >= 2: Bakery porters, laundry workers, infirmary runners, and hand-cart crews have joined the landing queue with the tools of interrupted work.}
{district_buffer >= 3: The high tank bells report stored water. The upper terraces have hours in which to argue before thirst becomes the chair.}

Hessa sets her seal between the service book and the water note.

"A seal can own one decision," she says. "It cannot own winter, the route offices, Basalt Crown, Seven Seals, and every empty cup. Choose where this goes next."

-> allocation_choice

=== allocation_choice ===
// ghostlight.choice_layer: consequence_owner
+ [Carry the delivery tickets, refusal notice, and Dema's slate to a district moot that includes the route offices.]
    // ghostlight.action: widen_accounting_jurisdiction
    // ghostlight.branch: audit_route_and_policy
    // ghostlight.intent: place_route_deductions_insurance_refusal_and_district_cost_before_affected_seals
    {route_proof >= 2 && public_record >= 3:
        -> ending_route_reckoning
    - else:
        -> ending_route_reckoning_cost
    }
+ {local_part >= 2} [Put the old workshop gauge beside the recut collar and ask Hessa to seal local maintenance.]
    // ghostlight.action: found_local_maintenance
    // ghostlight.branch: own_local_repair
    // ghostlight.intent: trade_consortium_support_for_inspectable_local_skill
    {local_part >= 2 && warranty_status <= 1 && water_flow >= 2 && worker_burden <= 4:
        -> ending_local_repair
    - else:
        -> ending_local_repair_cost
    }
+ {note_pressure >= 5} [Keep the certified collar running and post the accelerated winter claim beside the water rate.]
    // ghostlight.action: accept_borrowed_water
    // ghostlight.branch: own_accelerated_debt
    // ghostlight.intent: preserve_current_service_without_hiding_future_scarcity
    {water_flow >= 3 && note_pressure >= 5 && crystal_stock >= 2:
        -> ending_borrowed_water
    - else:
        -> ending_borrowed_water_cost
    }
+ [Join the ration line, protect the last crystal bin, and make the dry hours visible by street.]
    // ghostlight.action: distribute_water
    // ghostlight.branch: ration_in_public
    // ghostlight.intent: keep_scarcity_attached_to_the_people_and_reserves_paying_for_it
    {district_buffer >= 2 || water_flow >= 2:
        -> ending_rationed_time
    - else:
        -> ending_rationed_thirst
    }

=== ending_route_reckoning ===
// ghostlight.ending_label: route_reckoning_success
// ghostlight.training_hook: route_costs_enter_shared_jurisdiction
Dema carries the public slate. Orsa carries the tickets and the refusal. Neither lets Hessa's seal become the only legible thing in the room.

At the district moot, liftworks, kitchens, laundries, the Reserve House, and two route offices put their seals around one table. The missing crystal is traced through each deduction. Seven Seals must explain why truthful service records made the pump less insurable. Basalt Crown must explain why a replacement part inside a public cistern can remain creditor property during a dry hour.

No debt vanishes. It does acquire more owners than the borrower.
-> END

=== ending_route_reckoning_cost ===
// ghostlight.ending_label: route_reckoning_cost
// ghostlight.training_hook: accusation_without_chain
Orsa brings the district a thick accusation and a thin chain of custody.

The route offices can prove their standard passage shares. Seven Seals can point to the service-book exclusion. Basalt Crown can point to Hessa's late payment. The unexplained shortage dissolves among correct stamps.

The moot still hears the dry district, but the machinery of refusal has arrived better documented than the people paying it.
-> END

=== ending_local_repair ===
// ghostlight.ending_label: local_repair_success
// ghostlight.training_hook: portable_skill_replaces_vendor_dependency
Hessa seals the old gauge, the recut dimensions, and a public inspection interval. Brin's workshop method becomes something another journeyworker can learn instead of a private act of desperation.

Engine Two carries partial flow. Basalt Crown suspends support. Seven Seals raises the premium. The district moot redirects one payment from the water note into tools and training for local parts.

Orsa's extra shift becomes instruction rather than invisible debt. That is not free water. It is a different creditor.
-> END

=== ending_local_repair_cost ===
// ghostlight.ending_label: local_repair_cost
// ghostlight.training_hook: independence_claim_without_material_capacity
The old gauge proves only that the workshop remembers how collars used to fit.

The recut part is absent, badly seated, or asked to carry a flow the remaining workers cannot watch. Hessa refuses to seal aspiration as maintenance.

Basalt Crown keeps the crate. The district keeps the arrears. Orsa keeps the file marks on her hands and learns that refusing dependency still requires something sturdy enough to replace it.
-> END

=== ending_borrowed_water ===
// ghostlight.ending_label: borrowed_water_success
// ghostlight.training_hook: immediate_service_with_visible_future_claim
The certified collar runs smoothly. Water reaches the high streets before their stored vessels empty.

Dema posts the accelerated claim beside the restored gauge: winter-priority crystal pledged, two payments compressed into one season, replacement support intact. Relief reads the first line. Anyone responsible for December reads the rest.

Cistern House Nine has prevented a dry day by selling a piece of a cold one. The transaction is rational. That is what makes it dangerous.
-> END

=== ending_borrowed_water_cost ===
// ghostlight.ending_label: borrowed_water_cost
// ghostlight.training_hook: debt_cannot_replace_missing_stock
The amendment promises winter crystal the workshop does not presently possess.

The certified collar turns and water reaches the street today, but only one shallow bin remains for tomorrow's pumps, heat, and rail. Basalt Crown owns priority over the next supply; the district owns the wait for it. Hessa's seal has made the conflict explicit without making it smaller.

By evening the water rate and the debt both rise. The tank level is the only figure moving in the kinder direction.
-> END

=== ending_rationed_time ===
// ghostlight.ending_label: rationed_time
// ghostlight.training_hook: stored_water_buys_deliberation
The header tank and partial flow make rationing real rather than ceremonial.

Orsa carries copper cans uphill beside bakery porters and infirmary runners. Dema marks each street's remaining hours. Brin guards the last crystal bin while Hessa summons the affected seals.

Work stops unevenly. The bathhouse closes first. The infirmary does not. The district has purchased enough time to choose who pays next, and kept the purchase where everyone can see it.
-> END

=== ending_rationed_thirst ===
// ghostlight.ending_label: rationed_thirst
// ghostlight.training_hook: public_scarcity_without_buffer
There is too little stored water to ration and too little flow to replenish it.

The hand-cart price climbs by the stair. Laundry workers lose the shift. Bakers choose which dough to abandon. Infirmary kettles take the clean stock. Orsa writes the sequence by street because somebody must preserve the difference between a district cost and a general tragedy.

The pump-house ledger will show one broken collar. The Hold will spend the day paying for the rest.
-> END
