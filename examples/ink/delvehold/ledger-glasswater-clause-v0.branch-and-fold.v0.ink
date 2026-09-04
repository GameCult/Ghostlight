// ghostlight.artifact_id: ledger_glasswater_clause_branch_fold_v0
// ghostlight.fixture_id: ledger-glasswater-clause-v0
// ghostlight.scene_id: ledger-glasswater-clause-v0.mallowfen-first-furrow
// ghostlight.final_ink_path: examples/ink/delvehold/ledger-glasswater-clause-v0.branch-and-fold.v0.ink

VAR contract_standing = 2
VAR water_security = 2
VAR field_exposure = 0
VAR public_evidence = 0
VAR crystal_trace = 0
VAR fellowship_cohesion = 2
VAR service_guarantee = 2
VAR assessor_scope = 0

-> start

=== start ===
// ghostlight.scene: mallowfen_pump_house_establishing
The Four-Course Pump House wakes before Mallowfen because water has seniority.

It is a low round room of fieldstone built into the canal bank. A spring intake descends through an iron grate on the south side. In the center, one Copper Mantle lifting engine sits on a waist-high stone plinth: brass pump body, black iron flywheel, blue-white crystal cartridge, and a standardized governor plate closed under red sealing wax. On the east wall, clear water crosses a carved delivery lip into a three-way splitter. Its gate wheels feed the village cistern north, the millcourse east, and the Nine-Reed furrows south. Two old stone holding beds lie outside beyond the south door, shallow as soup plates and much less forgiving.

Pella Reedbank, pumpkeeper and ledger hand for the Nine-Reed Water Fellowship, has to stand on the fixed oak tread to reach the governor gauge. This was omitted from Copper Mantle's first installation because the draughtsman had drawn an average customer and accidentally invented a six-foot halfling.

-> morning_routine

=== morning_routine ===
// ghostlight.scene: mallowfen_morning_routine
Pella measures the village cistern, oils the east gate spindle, and writes the cartridge number on a waxed tally board. At first grain auction, nine farms will owe Copper Mantle one joint instalment on the engine. The pump belongs to the consortium for four harvests yet. The thirst already belongs to Mallowfen.

Senn Cloverrow waits by the south gate with a seed satchel and three oatcakes. He grows barley, keeps the fellowship's seed ledger, and considers breakfast a branch of hydraulic engineering.

Dorrin Valevein, Copper Mantle's dwarven journeyworker, checks the sealed governor plate with a magnifying lens. He may clean channels, change bearings, and certify pressure. If he cuts the wax to alter the standardized runes, the guarantee ends before his knife reaches the second stroke.

At noon, Caldris Morren of the human Charterhouse of the Third Bell is due to inspect a hairline crack in the old cistern curb. His policy pays for stone that breaks and engines that stop. Pella has read it often enough to know that fields are expected to possess better lawyers than they generally do.

Senn offers her an oatcake. "Prime the pump or the pumpkeeper?"

"The cheaper one."

"Bad news. They are secured by the same harvest."

-> allocation_choice

=== allocation_choice ===
// ghostlight.choice_layer: ordinary_water_handover
+ [Take paired samples at the spring grate and the delivery lip before opening a course.]
    // ghostlight.action: sample_water
    // ghostlight.branch: prime_paired_record
    // ghostlight.intent: create_a_public_before_and_after_record
    ~ public_evidence = public_evidence + 1
    ~ crystal_trace = crystal_trace + 1
    ~ water_security = water_security - 1
    Pella fills two stoppered glass bottles, one below the south grate and one beneath the east delivery lip. Senn presses the cartridge number into both wax collars with the fellowship stamp.

    Dorrin watches the delayed gauge. "Copper Mantle's manual calls that unnecessary."

    "Then it should be comforted by how little they charge for it," Pella says.

    The village cistern loses a morning mark while they make the comparison legible.
    -> routine_fold
+ [Fill the village cistern and clear both old holding beds before giving the furrows their turn.]
    // ghostlight.action: route_water
    // ghostlight.branch: prime_water_reserve
    // ghostlight.intent: buy_shutdown_time_with_stored_water
    ~ water_security = water_security + 2
    ~ contract_standing = contract_standing - 1
    Pella opens the north wheel while Senn rakes silt from the old holding beds and leaves their drain stones open. Water climbs the cistern gauge. Outside, two empty stone basins wait with their bottoms visible.

    Dorrin taps the contracted-flow schedule. The south course should already be taking water.

    "The schedule has never had to drink," Senn says.

    Pella logs the delay anyway. Debts grow best in unrecorded weather.
    -> routine_fold
+ [Open Senn's first furrow on schedule and let the ordinary morning earn its oatcake.]
    // ghostlight.action: open_gate
    // ghostlight.branch: prime_first_furrow
    // ghostlight.intent: protect_the_crop_schedule_and_fellowship_routine
    ~ fellowship_cohesion = fellowship_cohesion + 1
    ~ contract_standing = contract_standing + 1
    ~ field_exposure = field_exposure + 1
    Senn leans his weight into the south wheel while Pella takes the low handle. The gate opens. Water slips into the first barley furrow exactly on the auction schedule.

    He tears the third oatcake in half. Fellowship law contains no breakfast clause, which is why it remains useful.
    -> routine_fold
+ [Have Dorrin complete the Copper Mantle service checks before any field takes water.]
    // ghostlight.action: request_service
    // ghostlight.branch: prime_vendor_certificate
    // ghostlight.intent: preserve_the_engine_guarantee_and_contract_record
    ~ service_guarantee = service_guarantee + 2
    ~ contract_standing = contract_standing + 1
    ~ water_security = water_security - 1
    Dorrin calls pressure, lift, cartridge seal, bearing heat, and delivery clarity. Pella repeats each measure onto the tally board.

    Every number falls inside Copper Mantle tolerance. The mill wheel waits through the inspection and complains by being still.

    Dorrin signs the service line. "A perfect little engine."

    "I have met perfect things," Pella says. "They usually invoice separately."
    -> routine_fold

=== routine_fold ===
// ghostlight.fold: ordinary_water_before_glass
The lifting engine takes mana from the numbered crystal and turns it into a steady iron heartbeat. Water rises from the spring grate, passes through the brass body, and spills clear across the delivery lip.

{contract_standing >= 3: The fellowship is exactly where the lease wants it: on time, on gauge, and too busy to ask who wrote the categories.}
{contract_standing <= 1: The tally board already carries a delayed-course mark the Copper Mantle factor can price at grain auction.}
{water_security >= 4: The north cistern gauge stands high, and the cleared holding beds preserve a shallow reserve of containment space.}
{water_security <= 1: The cistern sits a mark below comfort; stopping now would turn every bucket into a vote.}
{public_evidence >= 1: Two sealed sample bottles wait on the low shelf with one cartridge number pressed into both collars.}
{fellowship_cohesion >= 3: Senn has shared breakfast and gate work. Nine-Reed feels, briefly, like nine farms instead of one debt.}
{service_guarantee >= 4: Dorrin's signed service line says the engine meets every mechanical measure Copper Mantle promised.}

Then Senn makes a small sound from the south door.

-> glasswater_appears

=== glasswater_appears ===
// ghostlight.scene: mallowfen_glasswater_reveal
~ field_exposure = field_exposure + 1
At the first barley row, the new roots have turned clear at the edges. Sunlight enters them and comes back wrong, split into pale green needles. A glassy weed unfolds beside Senn's boot with the efficient confidence of something arriving under contract.

The pump does not cough. The flywheel holds speed. The delivery water remains clear enough to show Pella's face looking down from the lip.

Dorrin checks the gauge twice. "Pressure is true. Lift is true."

Senn lifts one translucent root on the point of his field knife. "Wonderful. We can eat the pressure report."

Pella knows the Glasswater Clause by its cleanest sentence: water delivered at the contracted height and rate is serviceable unless a mechanical fault is found inside the engine.

-> first_response_choice

=== first_response_choice ===
// ghostlight.choice_layer: first_glasswater_response
+ [Pull the low null chain and stop the engine before another furrow takes water.]
    // ghostlight.action: stop_engine
    // ghostlight.branch: stop_before_spread
    // ghostlight.intent: protect_unwatered_fields_despite_contract_risk
    ~ water_security = water_security - 2
    ~ contract_standing = contract_standing - 1
    ~ service_guarantee = service_guarantee - 1
    Pella steps off the tread and pulls the red-braided null chain beneath the plinth. The runes close from right to left. The flywheel slows. Clear water thins to drops at the delivery lip.

    Dorrin does not touch the sealed plate. "Voluntary stoppage," he says, because the service form has already entered the room in his voice.

    "Write who volunteered," Senn says. He stands between the wet furrow and eight dry ones.
    -> loss_pressure_fold
+ [Turn the south gate into the old stone holding beds and keep suspect water off the crop.]
    // ghostlight.action: divert_water
    // ghostlight.branch: divert_to_holding_beds
    // ghostlight.intent: preserve_engine_operation_while_containing_the_visible_risk
    ~ water_security = water_security - 1
    ~ public_evidence = public_evidence + 1
    ~ contract_standing = contract_standing + 1
    Pella shuts the furrow gate and opens the holding-bed sluice. Water spreads across bare stone where any new growth can be counted.

    The millcourse wheel slows. From beyond the east wall comes the miller's bell: one strike for late flow, then a second for personal insult.

    Dorrin watches the beds. "Engine remains in service."

    "The barley is thrilled for it," Senn says.
    -> loss_pressure_fold
+ [Run one measured furrow farther and stake its wet edge for the assessor.]
    // ghostlight.action: controlled_exposure
    // ghostlight.branch: buy_field_evidence
    // ghostlight.intent: spend_crop_area_to_make_the_harm_measurable
    ~ field_exposure = field_exposure + 2
    ~ public_evidence = public_evidence + 2
    ~ contract_standing = contract_standing + 1
    Pella drives willow stakes along the next dry row. Senn opens the gate only to the marked line.

    Water reaches the first stake. Fine translucent hairs appear along the barley roots behind it. At the second, a pale weed pushes through mud quickly enough for Dorrin to take one step back.

    Pella closes the gate. Two furrows now carry evidence. They also carry the loss.
    -> loss_pressure_fold
+ [Replace the numbered crystal cartridge and lock both lots under fellowship and workshop stamps.]
    // ghostlight.action: transfer_and_replace_fuel
    // ghostlight.branch: trace_crystal_lot
    // ghostlight.intent: preserve_fuel_custody_and_test_whether_the_pattern_follows_the_lot
    ~ crystal_trace = crystal_trace + 2
    ~ public_evidence = public_evidence + 1
    ~ water_security = water_security - 1
    Dorrin closes the feed latch while Pella catches the spent cartridge in its padded cradle. They press his service mark and Nine-Reed's stamp into the custody wax.

    The spare bears a different numbered lot from the same consortium factor. The restart costs a quarter cistern while the lift reprimes.

    Clear water returns to the lip. The marked furrow waits to say whether the difference matters.
    -> loss_pressure_fold

=== loss_pressure_fold ===
// ghostlight.fold: glasswater_meets_the_policy
By the time Caldris Morren arrives, the pump house contains three kinds of truth and insufficient shelving.

The Charterhouse assessor ducks under the round west lintel in a blue travelling coat. A brass bell badge closes his collar; a waxed claim folio rides beneath one arm. He sees the engine first, because it is insured. He sees the field second, because Senn is standing in the doorway holding a luminous root at eye height.

{field_exposure >= 4: Two measured furrows shine with thin glass-green root hairs. The visible loss has crossed from warning into acreage.}
{field_exposure <= 2: The injury remains concentrated around the first wet row, small enough to dispute and large enough to recognize.}
{water_security >= 3: The north cistern buys Mallowfen part of a day in which refusal is still a choice.}
{water_security <= 0: The village cistern gauge enters its red band. The pump dispute has acquired cups, livestock, and a deadline.}
{public_evidence >= 3: Stakes, sample wax, times, and paired marks give the change a route through the room.}
{public_evidence <= 1: The field glitters; the paperwork sees a serviceable engine and one farmer's alarming plant.}
{crystal_trace >= 3: Two numbered crystal lots sit under joint custody where neither Copper Mantle nor Nine-Reed can replace one quietly.}
{service_guarantee >= 4: Dorrin's service certificate is immaculate, which strengthens the guarantee and the insurer's reason to deny a mechanical loss.}
{service_guarantee <= 1: The stopped or disturbed engine has begun to resemble a buyer-made exception.}

Cal opens the folio. "Show me the insured failure."

Senn offers him the root.

Cal does not take it. "That is a crop."

-> assessor_choice

=== assessor_choice ===
// ghostlight.choice_layer: define_the_insured_failure
+ [Keep Cal at the delivery lip and make him record the perfect gauges.]
    // ghostlight.action: narrow_inspection
    // ghostlight.branch: preserve_mechanical_claim
    // ghostlight.intent: protect_title_and_guarantee_by_accepting_the_policy_boundary
    ~ contract_standing = contract_standing + 2
    ~ service_guarantee = service_guarantee + 1
    ~ fellowship_cohesion = fellowship_cohesion - 1
    Pella holds the tally board beneath the gauge while Dorrin calls the measures again.

    Cal writes pressure, lift, delivery clarity, and intact seal. His pen moves with relief. A perfect engine is a short journey home.

    Senn leaves the luminous root on the contract shelf. Nobody agrees to call it evidence.
    -> claim_fold
+ [Walk Cal from spring grate to delivery lip to staked furrow and require one continuous entry.]
    // ghostlight.action: widen_inspection
    // ghostlight.branch: make_field_part_of_claim
    // ghostlight.intent: join_the_mechanical_route_to_the_agrarian_loss
    ~ assessor_scope = assessor_scope + 2
    ~ public_evidence = public_evidence + 1
    ~ contract_standing = contract_standing - 1
    Pella gives Cal the inlet bottle and keeps the outlet bottle herself. She walks him south through the low door, along the stone apron, and to the willow stakes.

    He has to put the folio against his knee to write outdoors. This improves his attention more than argument did.

    "I can record sequence," he says. "I cannot admit cause."

    "Begin with the thing you can do," Pella says.
    -> claim_fold
+ [Ring the fellowship bell and have each affected farm mark the public tally.]
    // ghostlight.action: summon_witnesses
    // ghostlight.branch: collectivize_the_loss_record
    // ghostlight.intent: prevent_one_household_from_carrying_a_joint_contracts_evidence_alone
    ~ fellowship_cohesion = fellowship_cohesion + 2
    ~ public_evidence = public_evidence + 1
    ~ contract_standing = contract_standing - 1
    Pella strikes the little iron bell above the west door nine times.

    Farmers arrive by path and ditch bank: muddy cuffs, irrigation keys, two babies, one furious miller, and every opinion that breakfast had postponed. Each marks the course, time, and loss personally observed.

    Cal's private inspection becomes a public meeting by the ancient legal method of running out of room.
    -> claim_fold
+ {crystal_trace >= 2} [Ask Dorrin to cut Copper Mantle's governor seal and expose the feed channels.]
    // ghostlight.action: break_vendor_seal
    // ghostlight.branch: open_the_governor
    // ghostlight.intent: trade_the_guarantee_for_inspectable_pattern_evidence
    ~ service_guarantee = 0
    ~ public_evidence = public_evidence + 2
    ~ assessor_scope = assessor_scope + 1
    Dorrin rests his knife beneath the red wax. "Once."

    Pella nods.

    The seal parts. Inside the brass plate, fine gold channels carry faint green after-lines downstream of the crystal socket. Cal steps close despite himself. Senn holds both custody boxes where every mark remains visible.

    Dorrin does not name the pattern. He names the cost. "Guarantee void."
    -> claim_fold

=== claim_fold ===
// ghostlight.scene: mallowfen_claim_threshold
The Four-Course Pump House was built to move water uphill. By noon it is moving risk downhill with even greater efficiency.

{assessor_scope >= 2: Cal's folio now contains a route from spring to engine to field. The Charterhouse may dispute cause, but it cannot file the crop outside the visit.}
{assessor_scope == 1: Cal has recorded the opened feed channels as relevant while keeping the field loss at the edge of the policy.}
{assessor_scope == 0: The formal inspection begins and ends at the delivery lip. Everything south of the wall is somebody else's category.}

{contract_standing >= 4: Copper Mantle still sees a compliant borrower, regular flow, and recoverable instalments.}
{contract_standing <= 1: The fellowship is one notice away from giving the consortium both a default and a reason to repossess before sowing.}
{fellowship_cohesion >= 4: Nine-Reed's members have started treating the instalment and evidence as joint burdens rather than Senn's private bad field.}
{fellowship_cohesion <= 1: Senn stands apart from the tally. The joint debt remains collective; the visible injury does not.}
{crystal_trace >= 3: The two crystal lots and their waxed custody marks can survive a journey to a university, court, or customs house.}
{service_guarantee <= 1: Copper Mantle's red seal lies cut on the shelf. The machinery is more inspectable and less insurable.}

Cal points to the contract shelf. "The instalment remains due at grain auction. I can admit a stopped engine, broken stone, or proven internal fault. I cannot insure an argument between water and barley."

Pella looks from the high gauge to Senn's small bright root. The clause has done what good machinery does: assigned every motion to a channel. She has to decide which channel to obstruct.

-> final_choice

=== final_choice ===
// ghostlight.choice_layer: who_carries_the_loss
+ [File the whole route as one Glasswater claim and refuse a delivery-lip-only decision.]
    // ghostlight.action: file_claim
    // ghostlight.branch: contest_glasswater_clause
    // ghostlight.intent: force_the_underwriter_to_carry_the_connected_loss
    {assessor_scope >= 2 && public_evidence >= 3:
        -> ending_claim_admitted
    - else:
        -> ending_claim_denied
    }
+ [Keep the pump under guarantee and send its output only to cistern and holding stone.]
    // ghostlight.action: preserve_contract
    // ghostlight.branch: protect_engine_title
    // ghostlight.intent: avoid_repossession_while_containing_field_exposure
    {contract_standing >= 3 && service_guarantee >= 2 && water_security >= 2:
        -> ending_title_preserved
    - else:
        -> ending_title_cost
    }
+ [Stop the engine, ring the nine farms together, and replace pumped water with a hauling rota.]
    // ghostlight.action: organize_mutual_aid
    // ghostlight.branch: stop_and_share_default
    // ghostlight.intent: protect_the_fields_by_making_thirst_and_debt_collective
    {fellowship_cohesion >= 4 && water_security >= 2:
        -> ending_fellowship_holds
    - else:
        -> ending_fellowship_frays
    }
+ [Break with Copper Mantle: preserve both crystal lots, open the governor, and commission a hand-cast lift test.]
    // ghostlight.action: commission_independent_test
    // ghostlight.branch: trace_the_pattern_outside_contract
    // ghostlight.intent: sacrifice_the_guarantee_to_build_portable_evidence
    {crystal_trace >= 3 && public_evidence >= 3:
        -> ending_trace_survives
    - else:
        -> ending_trace_collapses
    }

=== ending_claim_admitted ===
// ghostlight.ending_label: connected_claim_admitted
// ghostlight.training_hook: policy_boundary_forced_to_follow_material_route
Cal seals the inlet sample, outlet sample, staked-furrow record, and custody numbers into one claim packet. He writes *admitted for dispute*, not *payable*. The words are small and expensive.

The Charterhouse can still refuse. It must now refuse the route in public rather than erase the field at the wall.

Nine-Reed keeps the pump, the debt, and a case large enough for other borrowers to recognize themselves in it. Senn returns the bright root to its jar. "Can we eat the argument?"

"Not yet," Pella says. "But we can make them choke on the filing fee."
-> END

=== ending_claim_denied ===
// ghostlight.ending_label: connected_claim_denied
// ghostlight.training_hook: evidence_without_admitted_scope
Cal records clear delivery, disputed crop injury, and insufficient proof of an internal mechanical fault. The Glasswater Clause closes around the claim exactly where Copper Mantle built the lip.

{field_exposure >= 4: Two furrows are entered as agrarian loss outside cover.}{field_exposure < 4: One shining row is entered as an unverified cultivation event.}

The instalment survives the harvest it may have helped ruin. Pella keeps a copy of the refusal. It is poor water and excellent kindling for a larger argument.
-> END

=== ending_title_preserved ===
// ghostlight.ending_label: engine_title_preserved
// ghostlight.training_hook: contract_survival_through_containment_cost
The engine keeps its red seal and its perfect measures. Pella locks the south gate, fills the village cistern, and sends the rest into bare holding stone under numbered stakes.

Copper Mantle retains its future payments. The Charterhouse retains its narrow policy. Mallowfen retains the physical pump.

The millcourse stops by evening. Nine farms count how long seed can wait while an owned machine performs flawlessly into an empty bed.
-> END

=== ending_title_cost ===
// ghostlight.ending_label: engine_title_cost
// ghostlight.training_hook: contract_strategy_without_material_margin
Pella tries to preserve title with a guarantee already cut, a cistern already low, or a contract already in breach.

Cal records no covered failure. Dorrin cannot restore wax to a seal after witnesses have seen its inside. The Copper Mantle factor gains both a service exception and a late-flow notice.

The pump remains in the room. That is not the same thing as Mallowfen keeping it.
-> END

=== ending_fellowship_holds ===
// ghostlight.ending_label: fellowship_mutual_aid_holds
// ghostlight.training_hook: collective_capacity_against_joint_debt
Pella pulls the null chain. Nine-Reed's bell answers across the ditch paths.

The full cistern is rationed by household, livestock, and seed bed. Handcarts begin moving water from the upstream ford. The miller lends barrels after being permitted three minutes of uninterrupted grievance, a resource he had plainly stored for drought.

The fields stay dry of glasswater for one day. At grain auction the instalment will still be due. For tonight, the joint liability has become joint work instead of nine private failures.
-> END

=== ending_fellowship_frays ===
// ghostlight.ending_label: fellowship_mutual_aid_frays
// ghostlight.training_hook: solidarity_claim_without_reserve_or_trust
The pump stops. The bell rings. Too few carts come.

{water_security <= 1: The cistern gauge enters red before sunset.}{water_security > 1: Stored water lasts through supper and fails before the livestock troughs refill.}

Some farms open their private wells. Others accuse Senn of making one strange furrow into everybody's default. Joint liability remains in the contract after fellowship has left the yard.

Pella spends the evening carrying buckets and learning the precise weight of an institution invoked too late.
-> END

=== ending_trace_survives ===
// ghostlight.ending_label: independent_trace_survives
// ghostlight.training_hook: portable_evidence_bought_with_warranty
Dorrin cuts the governor seal under Pella's fellowship mark, his service mark, and Cal's witnessed note. The two numbered crystal lots remain locked apart. A local mage lifts one barrel by hand through a fresh copper pattern laid beside the engine.

The comparison does not prove a wounded world or an innocent mine. It does show that the measured after-pattern in the lifted water changes with the fuel path while spring and test barrel remain the same.

Copper Mantle cancels the guarantee. Cal cannot make the evidence disappear into mechanical failure. By dusk, a university courier and a customs clerk are both asking for copies. Mallowfen has traded an insured machine for an argument that can travel.
-> END

=== ending_trace_collapses ===
// ghostlight.ending_label: independent_trace_collapses
// ghostlight.training_hook: broken_seal_without_chain_of_custody
The governor opens, the guarantee ends, and the evidence refuses to become orderly.

One crystal lot lacks a paired sample. A custody mark was made after the first furrow. The hand-cast lift differs in three ways nobody recorded before changing it.

Copper Mantle calls the test tampering. The Charterhouse agrees that tampering is at least a category it understands. Nine-Reed keeps the luminous root, the joint instalment, and an uninsured pump with its red wax cut clean through.
-> END
