// ghostlight.artifact_id: delvehold_hearth_mossgate_bellhouse_branch_fold_v0
// ghostlight.fixture_id: hearth-mossgate-bellhouse-v0
// ghostlight.scene_id: hearth-mossgate-bellhouse-v0.the-silent-return-bell
// ghostlight.final_ink_path: examples/ink/delvehold/hearth-mossgate-bellhouse-v0.branch-and-fold.v0.ink

VAR pantry_reserve = 2
VAR water_reserve = 2
VAR apprentice_readiness = 2
VAR outage_record = 1
VAR household_strain = 1
VAR family_trust = 2
VAR repair_progress = 1
VAR open_table = 0
VAR company_pressure = 1

-> start

=== start ===
Mossgate East Bellhouse shares one honey-stone wall with the station and most of its opinions with the platform.

On the street side, it is Tamsin Reed's kitchen: long table, black stove, water jars under the serving hatch. On the rail side, a red return bell hangs above two iron tracks and a blue crystal conduit laid between them. The conduit feeds both the dwarf-built locomotives and Mossgate's orchard pumps. This was praised as efficiency when the contract was signed. It remains efficient now, chiefly at making distant trouble local.

-> breakfast_routine

=== breakfast_routine ===
Brunna Slatewise draws the null rune in spilled flour while Tamsin cuts yesterday's loaf and her ten-year-old daughter Nell ties red waiting-cords to the family board.

Brunna is a young dwarf apprenticed to the foreign railway branch. She has lived at Tamsin's table for two years. Mossgate calls that table-kin: no inheritance, no civic seal, just a bed, a share of chores, and the dangerous habit of noticing when someone is late.

Nell holds up a crooked cord. "This one is Jory."

"Your father is less crooked," Tamsin says.

"Only standing up."

The last orchard freight should bring Jory home before supper. Brunna's forewoman, Master Hesta Flint, has meanwhile assigned a lamp-engine inspection after the return bell. The two duties fit perfectly on paper, where nobody has to eat.

// ghostlight.choice_layer: morning_preparation
+ [Knead the remaining flour into two extra travel loaves.]
    // ghostlight.action_label: use_object
    // ghostlight.branch_label: prepare_extra_bread
    ~ pantry_reserve = pantry_reserve + 2
    ~ household_strain = household_strain + 1
    ~ family_trust = family_trust + 1
    Brunna washes the practice rune away and puts both hands into the dough.

    "Rail work," she says, leaning her weight into it.

    Tamsin looks at the flour bin. "Rail work usually leaves more flour than that."

    "Then this is advanced rail work."

    The second loaf costs the week's sweet flour. It also puts eight more slices between the platform and an empty table.
    -> morning_fold
+ [Rehearse a live null termination on the cold lamp engine.]
    // ghostlight.action_label: touch_object
    // ghostlight.branch_label: rehearse_null
    ~ apprentice_readiness = apprentice_readiness + 2
    ~ repair_progress = repair_progress + 1
    Brunna seats a practice crystal in the brass lamp engine, wakes one blue line, then closes the pattern right to left.

    Light folds into darkness without heat or snap.

    Nell applauds. Tamsin does not; Tamsin has seen applause become confidence and confidence become invoices.

    Brunna runs it again until her hand stops hurrying the last stroke.
    -> morning_fold
+ [Retie the waiting board and add every household member's usual train.]
    // ghostlight.action_label: write_record
    // ghostlight.branch_label: prepare_waiting_board
    ~ outage_record = outage_record + 2
    ~ family_trust = family_trust + 1
    Brunna replaces Nell's knots with a clean row: white cord for home, red for expected, black bead for the last confirmed station.

    Jory's red cord runs south to the orchard freight. Two neighbours work the dye-market train. Hesta's crew occupies the short blue loop between the signal cabinet and engine shed.

    "It looks worried," Nell says.

    "It is a board," Brunna says.

    "So it worries in an organized way."
    -> morning_fold
+ [Fill the two spare water jars before the orchard pumps change shift.]
    // ghostlight.action_label: move_object
    // ghostlight.branch_label: bank_household_water
    ~ water_reserve = water_reserve + 2
    ~ household_strain = household_strain + 1
    Brunna carries the blue-glazed jars through the rear door to the alley spout. The pump rune thrums through the stone under her boots.

    Old Emet from the upper landing lowers his own copper pot on a rope. His knees have not forgiven the station lift for existing, because they remember the years before it did.

    Brunna fills his pot too. By breakfast, her shoulders know more municipal policy than parliament.
    -> morning_fold

=== morning_fold ===
// ghostlight.fold: prepared_household_routine
The bellhouse wakes by accumulation. Tamsin opens the serving hatch. Nell moves three bowls to their habitual places. Brunna hangs her tool roll beside the rail door where work and home can both claim it.

{pantry_reserve >= 4: Two travel loaves cool under a cloth, expensive and consoling.}
{apprentice_readiness >= 4: The lamp engine sits dark after three clean null terminations; Brunna's last stroke no longer shakes.}
{outage_record >= 3: The waiting board names who should return on which train, including Jory on the south orchard freight.}
{water_reserve >= 4: Four blue-glazed jars stand full beneath the serving hatch, enough to make a stopped pump inconvenient before it becomes cruel.}
{household_strain >= 2: Tamsin quietly moves the week's coin from the sweet-flour jar to the ordinary-flour jar. Preparation has already sent a bill.}

Then noon passes. The orchard freight does not.

-> silent_bell

=== silent_bell ===
The red return bell lifts half an inch and gives no sound.

On the platform, the green arrival rune drains to gray. Beneath the kitchen floor, the pump-thrum stutters with it. Somewhere south of Mossgate, the foreign rail has stopped carrying fruit, workers, letters, medicine, and one particular father whose soup is getting ideas about abandonment.

Nell looks at Jory's bowl.

Tamsin says, "Bellhouse watch."

That is all. The household has practiced the rest by living here.

-> first_response

=== first_response ===
// ghostlight.choice_layer: first_outage_response
+ [Open the rail door, hang the white ladle, and begin serving whoever reaches the platform.]
    // ghostlight.action_label: open_route
    // ghostlight.branch_label: open_bellhouse_table
    ~ open_table = 1
    ~ pantry_reserve = pantry_reserve - 1
    ~ household_strain = household_strain + 1
    ~ family_trust = family_trust + 1
    Brunna throws the rail door wide and hangs the white wooden ladle from its hook. Across Mossgate, other bellhouses will see it and know this table is taking strangers.

    The first arrivals are two orchard sorters, a wet nurse from the west platform, and a boy carrying three lunch tins for adults who are not behind him.

    Tamsin gives Brunna the knife. Family, in this quarter, is partly the list of people allowed to cut the loaf unevenly.
    -> response_fold
+ [Take the tool roll to the platform signal cabinet and isolate the dead feed.]
    // ghostlight.action_label: inspect_object
    // ghostlight.branch_label: inspect_signal_feed
    ~ repair_progress = repair_progress + 2
    ~ apprentice_readiness = apprentice_readiness + 1
    ~ company_pressure = company_pressure + 1
    ~ family_trust = family_trust - 1
    Brunna crosses the yellow platform stones, unlocks the waist-high cabinet, and draws null before touching the gray conduit.

    The local branch is intact. The dead pressure comes from the south cutting, beyond her authority and beyond sight.

    She chalks the result inside the cabinet door. It is useful knowledge. It does not put Jory at the table, and Tamsin sees which claim on Brunna's hands won first.
    -> response_fold
+ [Carry one full jar up the alley stairs to Old Emet before the lift and spout fail together.]
    // ghostlight.action_label: carry_object
    // ghostlight.branch_label: carry_neighbor_water
    ~ water_reserve = water_reserve - 1
    ~ household_strain = household_strain + 1
    ~ family_trust = family_trust + 2
    ~ outage_record = outage_record + 1
    Brunna shoulders a jar and climbs the narrow alley stairs beside the stalled lift cage.

    Old Emet has already put his copper pot outside. Pride is easier to preserve when everyone agrees the pot made the request.

    From the upper landing Brunna can see the gray arrival rune, the silent bell, and people beginning to gather under the station awning. She notes the time on the pot's chalk tag before descending.
    -> response_fold
+ [Carry the waiting board onto the platform and chalk the exact minute beside the gray signal.]
    // ghostlight.action_label: show_object
    // ghostlight.branch_label: make_public_record
    ~ outage_record = outage_record + 2
    ~ company_pressure = company_pressure + 1
    ~ household_strain = household_strain + 1
    Brunna props the family board against the station's official slate. Red cords hang beside the company's blank service line.

    She writes the minute the arrival rune failed.

    A clerk inside the ticket window closes his little brass shutter. This improves the view of the evidence considerably.
    -> response_fold

=== response_fold ===
// ghostlight.fold: household_and_rail_claims_meet
By the next quarter-hour, Mossgate has become two towns occupying the same platform: the railway's service pause and the neighbourhood's missing afternoon.

{open_table == 1: Steam clouds the serving hatch. The white ladle swings above a table filling faster than bowls can be washed.}
{repair_progress >= 3: Brunna knows the local cabinet is safe and the fault lies south; Hesta's crew will not need to waste its first hour proving that again.}
{outage_record >= 3: The waiting board turns a crowd into named absences. Jory's orchard freight was last confirmed at South Weir before the signal died.}
{outage_record <= 2: Jory remains somewhere south, which is geography performing its least helpful trick.}
{water_reserve >= 4: Full jars line the wall under the hatch.}
{water_reserve <= 1: One jar gives the hollow ceramic note that means care has reached the bottom.}
{family_trust >= 4: Nell gives Brunna Jory's bowl to guard, a small promotion with no appeal process.}
{family_trust <= 1: Tamsin keeps working, but the space beside Brunna at the table has acquired weather.}

The engine-shed gate opens.

-> company_arrival

=== company_arrival ===
Master Hesta Flint crosses the platform with two exhausted dwarven fitters and a company notice under one arm. Hesta is Brunna's forewoman: broad, gray-braided, and able to make an apprenticeship feel like a tool held close to the throat without ever raising her voice.

"South-cut feed fault," she says. "Service pause. I need every trained hand."

Tamsin reads the notice. "An outage releases company meal stock."

"A pause does not."

"How fortunate for the stock."

Hesta looks past her to the table, the water jars, the waiting cords, and the crew who will need all three before morning. She is not blind to the bellhouse. She is carrying the piece of paper that lets the railway depend on it for free.

{company_pressure >= 3: The notice already bears a second seal: refusal now risks Brunna's placement as well as Hesta's schedule.}
{household_strain >= 3: Tamsin has cut the loaf thin enough that the knife taps the board between slices.}
{apprentice_readiness >= 4: Hesta notices the clean null marks on Brunna's glove and gives her the feed key.}

Nell holds Jory's empty bowl with both hands.

Work can restore the line. Care can keep the waiting from becoming damage. The company has arranged for the same apprentice to owe both at once.

-> final_choice

=== final_choice ===
// ghostlight.choice_layer: allocate_the_night
+ [Take Hesta's feed key and join the south-cut repair crew.]
    // ghostlight.action_label: accept_duty
    // ghostlight.branch_label: prioritize_rail_repair
    {apprentice_readiness >= 4 && repair_progress >= 4:
        Brunna takes the key, her tool roll, and one heel of bread.
        -> ending_repair_success
    - else:
        Brunna takes the key because apprenticeship is partly being trusted before anyone has budgeted for the consequences.
        -> ending_repair_cost
    }
+ [Set Hesta's key beside Jory's bowl and keep the bellhouse table open.]
    // ghostlight.action_label: refuse_duty
    // ghostlight.branch_label: prioritize_open_table
    ~ open_table = 1
    {pantry_reserve >= 3 && family_trust >= 3:
        Brunna ties on Tamsin's spare apron.
        -> ending_table_success
    - else:
        Brunna turns from the rail door toward a table already spending its last margin.
        -> ending_table_cost
    }
+ [Carry the waiting board to Hesta and require the pause to acquire its missing name.]
    // ghostlight.action_label: challenge_record
    // ghostlight.branch_label: prioritize_public_outage
    ~ outage_record = outage_record + 1
    {outage_record >= 5:
        Brunna lays the red cords across Hesta's unsigned notice.
        -> ending_record_success
    - else:
        Brunna raises a household record against a company slate that still has more seals than witnesses.
        -> ending_record_cost
    }
+ [Offer the crew beds and supper only if Hesta releases meal stock and rotates apprentices through bellhouse watch.]
    // ghostlight.action_label: negotiate_reciprocity
    // ghostlight.branch_label: prioritize_reciprocal_plan
    {open_table == 1 && (apprentice_readiness >= 4 || outage_record >= 3 || water_reserve >= 3):
        Brunna keeps one hand on the feed key and one on the white ladle's cord.
        -> ending_reciprocal_success
    - else:
        Brunna names the exchange before the household has enough prepared leverage to make it hold.
        -> ending_reciprocal_cost
    }

=== ending_repair_success ===
// ghostlight.ending_label: rail_repair_success
// ghostlight.training_hook: apprenticeship_skill_serves_household_at_a_cost
// ghostlight.scene: repair_south_cut
Brunna reaches the south cutting before dark. The fault is not a broken rune but a starved crystal feed, its pressure diverted to keep the lower orchard pumps alive.

Because she already proved the Mossgate cabinet safe, Hesta lets her isolate the locomotive branch and restore a narrow return current. The orchard freight crawls home after midnight.

// ghostlight.scene: repair_homecoming_success
{outage_record >= 3: The waiting board clears name by name. Jory is the fourth through the bellhouse door.}
{outage_record < 3: Jory arrives out of the dark before anyone can say where the train has been.}

{family_trust >= 3: Tamsin hands Brunna the first hot bowl and complains that table-kin should have the decency to be easier to resent.}
{family_trust < 3: Tamsin feeds her, because anger and supper have never been mutually exclusive in this house.}

The railway records a successful repair. The bellhouse records who came home thin with cold.
-> END

=== ending_repair_cost ===
// ghostlight.ending_label: rail_repair_cost
// ghostlight.training_hook: narrow_training_and_divided_obligation
// ghostlight.scene: repair_south_cut
Hesta puts Brunna on the feed manifold. The model is newer than the lamp engines she services and older than the diagram in her training book.

The crew spends three hours discovering what a broader apprenticeship would have taught in one. The line stays gray.

// ghostlight.scene: repair_homecoming_cost
{household_strain >= 3: At East Bellhouse, Tamsin closes the serving hatch when the last loaf becomes Nell's breakfast.}
{household_strain < 3: At East Bellhouse, the table remains open by borrowing bread from the next bellhouse down.}

Brunna returns before dawn with no train behind her. Nell has fallen asleep around Jory's bowl. Hesta still calls the night useful experience.

Nobody at the table uses that phrase.
-> END

=== ending_table_success ===
// ghostlight.ending_label: open_table_success
// ghostlight.training_hook: household_care_as_infrastructure
Brunna hangs the feed key back on Hesta's finger and opens the serving hatch another handspan.

The loaves become slices, the slices become soup, and the soup becomes enough time for other bellhouses to answer. One takes the orchard sorters. Another sends two full water jars. The wet nurse falls asleep beside the stove while Nell guards both lunch tins and dignity.

{outage_record >= 3: News reaches the board that Jory's train is held safely at South Weir. His red cord stays up, but the black bead moves.}
{outage_record < 3: No notice names Jory's train. Tamsin keeps looking at the rail door whenever a spoon strikes pottery.}

The railway does not move that night. Mossgate does.
-> END

=== ending_table_cost ===
// ghostlight.ending_label: open_table_cost
// ghostlight.training_hook: care_without_material_margin
Brunna chooses the table after the table has run out of choices.

The last loaf feeds six people badly. The last full jar goes upstairs. Tamsin closes the hatch while workers are still visible beneath the awning.

{company_pressure >= 3: Hesta leaves with the feed key and Brunna's placement under review.}
{company_pressure < 3: Hesta leaves with the feed key and the particular silence of someone who may understand later, when understanding is cheaper.}

Nell asks whether closing the door means they stopped being a bellhouse.

"No," Tamsin says. "It means a house is smaller than a railway."
-> END

=== ending_record_success ===
// ghostlight.ending_label: public_outage_success
// ghostlight.training_hook: domestic_records_force_cost_recognition
The board has times, routes, names, and enough witnesses that Hesta cannot make it private again.

She crosses out service pause. She writes outage.

The engine shed releases company meal stock and three sealed water drums. Nobody cheers; the line is still dead and Jory is still south of supper. But the railway begins paying for the night it has placed inside other people's homes.

Brunna's Master marks her late for repair duty and exact in public record. Both marks remain.

By morning, the waiting cords have become evidence fit for the municipal contract hearing. They are still also a family board. This is the indecency of useful records: they must serve strangers without forgetting whose names hurt.
-> END

=== ending_record_cost ===
// ghostlight.ending_label: public_outage_cost
// ghostlight.training_hook: testimony_without_admission
Hesta reads the board and does not dispute a single name.

Then she points to the station slate, where the company has recorded nothing long enough to owe for.

The ticket shutter stays closed. No meal stock comes out. Brunna's chalk time remains visible until rain blows under the awning and makes a gray river of it.

{family_trust >= 4: Tamsin brings the board back inside before the cords soak through. "Evidence can sleep here," she says.}
{family_trust < 4: Brunna carries the board inside alone, careful not to let Jory's cord drag.}

The claim fails tonight. The household has learned exactly which absence the contract refuses to see.
-> END

=== ending_reciprocal_success ===
// ghostlight.ending_label: reciprocal_watch_success
// ghostlight.training_hook: table_kin_obligation_binds_work_and_care
Brunna names the terms in front of the fitters, the platform families, and the open serving hatch.

Beds for the night crew. Supper before the south cutting. One apprentice left in Mossgate each watch to carry water, update the board, and meet returning trains. Company stores opened now, not after a clerk discovers dawn.

Hesta looks at the white ladle, then at two fitters already sitting down without permission. She releases the meal stock.

Brunna works the second repair watch and the first bellhouse watch. It is too much work. It is at least work whose beneficiaries can name one another.

// ghostlight.scene: reciprocal_dawn_return
At dawn the red bell rings once. Jory comes through the rail door as Brunna is setting out his bowl.
-> END

=== ending_reciprocal_cost ===
// ghostlight.ending_label: reciprocal_watch_cost
// ghostlight.training_hook: affectionate_custom_used_as_unpaid_capacity
Hesta accepts the beds and declines the terms.

The fitters are exhausted enough to sleep before anyone can turn hospitality back into leverage. The meal stock remains locked. Brunna joins the repair watch late, then returns to carry water because table-kin is not a shift one clocks out of.

{water_reserve <= 1: By midnight the jars are hollow and the alley spout is dead.}
{water_reserve > 1: By midnight one jar remains, guarded less like water than like an argument.}

The line eventually returns. So do the crews. The costs stay in East Bellhouse, where the company can admire them as community spirit and avoid learning arithmetic.
-> END
