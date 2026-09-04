// ghostlight.artifact_id: numen_answering_cut_v0
// ghostlight.fixture_id: numen-answering-cut-v0
// ghostlight.scene_id: numen-answering-cut-v0.three-bell-gate
// ghostlight.final_ink_path: examples/ink/delvehold/numen-answering-cut.branch-and-fold.v0.ink

VAR ritual_integrity = 2
VAR district_reserve = 2
VAR goblin_trust = 1
VAR company_pressure = 2
VAR alternate_route = 0
VAR witness_chain = 1
VAR core_recognition = 0
VAR answering_organ_intact = 1
VAR crew_safety = 2
VAR bargain_quality = 0
VAR rest_honored = 0

-> start

=== start ===
Three-Bell Gate is two lift descents below the warm-sea terraces, where the Hold has finished being a city and has not yet admitted it is standing in somebody else's throat.

The listening chamber is a long stone hall. The return lift waits at the west end. A brass withdrawal line crosses the floor before three eastern arches: the intended crystal face in the centre, a narrow alternate gallery to its left, and a sealed rest alcove to its right. Each arch holds a dark stone bell. A direct pump and a folded cutter crouch behind safety rails before the centre arch.

-> offering_bench

=== offering_bench ===
// ghostlight.scene: ordinary_gate_rite
Master Nera Ashmark weighs three portions at the offering bench: one mana crystal, one loaf, one twist of dried mushroom for each arch. Mine priest, gate-workshop Master, civic seal holder. Three jobs are economical until they disagree.

The centre face has closed on two consecutive Answering Cut watches. Today's offering is the third. If the old pattern holds, the route book must decide whether it has recorded a fault, a warning, or a neighbour.

"The crystal portions differ by a shaving," says Pekk-of-Wet-Stone from the listening niche above the left arch.

Pekk is the goblin listener retained by the gate workshop because survey wands have never developed noses, patience, or the humility to ask fungi how their week has been.

Nera shaves the larger crystal. "The core may lodge a procurement complaint."

"It has a better record of being answered than I do."

Captain Sava Tern checks the straps on her four-person delving party. They will carry the offerings through defended chambers, ring each bell, and come back over the brass line. The rite requires the core to meet living visitors who can still retreat.

Dorrik Vane, a Deep Company assessor in a clean iron-grey coat, waits beside the direct-pump gauge. His contract expects the centre face opened before tomorrow's high-water turn. The terraces above expect the crystal to keep their lift pumps and cold stores on schedule.

-> preparation_choice

=== preparation_choice ===
// ghostlight.choice_layer: rite_preparation
+ [Make every offering exact, even if the extra crystal comes out of the district reserve chest.]
    // ghostlight.action_label: use_object
    // ghostlight.branch_label: prepare_equal_offerings
    ~ ritual_integrity = ritual_integrity + 2
    ~ district_reserve = district_reserve - 1
    ~ witness_chain = witness_chain + 1
    Nera takes the reserve key from her belt. Three crystals leave the scale equal enough that even Pekk stops squinting.

    Dorrik writes down the withdrawal. Priests call this witness. Assessors call it finding the correct column.

    Sava divides the bread and mushroom by hand. Equal shares make poor theatre. That is one reason the old rite trusts them.
    -> withdrawal_line
+ [Walk the alternate gallery with Pekk before the party enters.]
    // ghostlight.action_label: move
    // ghostlight.branch_label: survey_alternate_with_pekk
    ~ alternate_route = alternate_route + 2
    ~ goblin_trust = goblin_trust + 2
    ~ crew_safety = crew_safety + 1
    ~ company_pressure = company_pressure + 1
    Nera ducks under the left bell and follows Pekk along the first bend. The ceiling lowers. Warm air leaks through a seam above a shelf of pale fungi.

    Pekk points with two fingers, then smells the stone. "Breathing outward. It wants this way noticed."

    They mark the handholds and a retreat pocket. When they return, Dorrik has turned one page of his schedule into three pages of objection.
    -> withdrawal_line
+ [Keep the direct pump turning until the last possible moment and buy the terraces another hour of reserve.]
    // ghostlight.action_label: touch_object
    // ghostlight.branch_label: preserve_pump_margin
    ~ district_reserve = district_reserve + 1
    ~ company_pressure = company_pressure - 1
    ~ ritual_integrity = ritual_integrity - 2
    ~ witness_chain = witness_chain - 1
    Nera leaves the isolation wedge on its hook. The pump keeps drawing a thin blue current from the centre face.

    Above, one more cistern gauge holds steady. Here, the right bell trembles in the engine noise before anybody touches it.

    Pekk folds both ears back. Dorrik does not write this down, which is also a kind of entry.
    -> withdrawal_line
+ [Ask Pekk to countersign the route page before any priest or assessor does.]
    // ghostlight.action_label: speak
    // ghostlight.branch_label: countersign_goblin_witness
    ~ goblin_trust = goblin_trust + 1
    ~ witness_chain = witness_chain + 2
    ~ company_pressure = company_pressure + 1
    Nera turns the master route slate around. "Your mark first. You will still be here when our instruments decide what they heard."

    Pekk presses a copper-dusted thumb beside the three arches. Dorrik's mouth makes the small straight line common to men who have just watched precedent happen without an appointment.

    Sava adds her expedition mark. Nera seals last, then copies the three marks onto each offering slate.
    -> withdrawal_line

=== withdrawal_line ===
// ghostlight.fold: preparations_become_visible
Sava's party carries the three offerings east. Steel buckles fade into the centre passage. The bells sound in order: centre, left, right.

They return with blood on one sleeve, mud on two boots, and all four names still attached to living people.

{ritual_integrity >= 4: The three offerings have gone from their slates. Fine crystal dust traces three different paths into the rock.}
{ritual_integrity <= 1: Pump vibration runs through the floor after the party crosses the withdrawal line. Whatever answers must speak through the hand already in its mouth.}
{alternate_route >= 2: Chalk handholds show along the left bend, and Sava's party returns by that safer pocket instead of crowding the centre arch.}
{goblin_trust >= 3: Pekk leaves the listening niche and stands beside Nera at the route book.}
{witness_chain >= 3: Goblin thumb, delver mark, and workshop seal sit together on the soft slate where nobody can later subtract a witness politely.}

Nera closes the brass gate. Every mortal body withdraws west of the line.

-> first_answer

=== first_answer ===
// ghostlight.scene: pivotal_answer
The rest bell rings from inside the wall.

-> answering_organ

=== answering_organ ===
The centre arch closes without falling. Warm stone flows across it in overlapping scales. The left arch exhales and opens another arm's breadth. Around the right bell, pale fungus darkens under root-fine threads of copper and silver.

The threads enter the right-hand offering slate.

They lift its marks into the wall: the centre cut, closed by a clean null stroke; the alternate gallery, left open; the old high-water tally, touched three times. Then the marks loosen and form again.

Sava removes her helmet.

Dorrik says, too quickly, "Coupled resonance."

Pekk says, "Main face sleeps for three high waters. Feet may use the left way. The mouth is being painfully clear."

The wall adds Nera's workshop seal beneath the borrowed marks. It gets one corner wrong, corrects it, and waits.

-> interpretation_choice

=== interpretation_choice ===
// ghostlight.choice_layer: interpret_the_answer
+ [Enter Pekk's reading in the route book as the core's answer.]
    // ghostlight.action_label: write
    // ghostlight.branch_label: record_core_answer
    ~ core_recognition = core_recognition + 2
    ~ goblin_trust = goblin_trust + 1
    ~ bargain_quality = bargain_quality + 1
    ~ company_pressure = company_pressure + 1
    Nera writes: CENTRE RESTS THREE HIGH WATERS. LEFT WAY ADMITS LIVING FEET.

    She leaves space for Pekk's wording beside her own. The answering organ copies the gap before it copies the words.

    Dorrik looks up the shaft, as though the district might already be drafting its reply.
    -> answer_fold
+ [Copy every stroke and timing before naming what made them.]
    // ghostlight.action_label: inspect
    // ghostlight.branch_label: preserve_stroke_evidence
    ~ witness_chain = witness_chain + 2
    ~ core_recognition = core_recognition + 1
    ~ district_reserve = district_reserve - 1
    Nera hands thin wax sheets to Sava. After the organ stills, Sava crosses the line alone, presses the sheets to the right-hand offering slate, and returns. Nera counts the interval between changes. Pekk records the smell of the fungal skin before and after each stroke.

    The work takes most of an hour. Somewhere above, a lift office begins spending reserve because nobody below will hurry wonder into admissible evidence.
    -> answer_fold
+ [Ask the wall what work it will permit before the third high water.]
    // ghostlight.action_label: speak
    // ghostlight.branch_label: ask_for_bounded_work
    ~ bargain_quality = bargain_quality + 2
    ~ alternate_route = alternate_route + 1
    ~ ritual_integrity = ritual_integrity + 1
    ~ company_pressure = company_pressure + 1
    Nera places the company's route slate beneath the left-hand mark. "Warm-sea terraces need crystal. Name the work that does not cut your sleep."

    The organ grows a narrow branch from the alternate line to Sava's expedition mark. Small loose crystals bud beside it, each no larger than a thumbnail. The centre null remains dark and absolute.

    Sava says, "A delve, then. Packs, choices, and a road that can bite back."
    -> answer_fold
+ [Let Dorrik enter the event as sensor coupling and feed company contingency crystal into the district line.]
    // ghostlight.action_label: withhold_judgment
    // ghostlight.branch_label: accept_sensor_fault
    ~ company_pressure = company_pressure - 2
    ~ district_reserve = district_reserve + 1
    ~ core_recognition = core_recognition - 1
    ~ witness_chain = witness_chain - 1
    Nera lifts neither seal nor pen.

    Dorrik writes SENSOR COUPLING across his copy before the wall has finished moving. Then he cracks a company contingency crystal into the westbound district feed. It is efficient work. The relief and the explanation arrive together.

    Pekk returns to the listening niche. The distance is only twelve steps. It takes the whole room with it.
    -> answer_fold

=== answer_fold ===
// ghostlight.fold: interpretation_changes_the_hearing
The direct-pump gauge trembles at zero. Far above, households open taps, lift offices read their reserves, and cold-store clerks begin counting how many promises fit inside three high waters.

{core_recognition >= 2: The route book now uses grammar normally reserved for neighbours, contractors, and dangerous saints: it answered, it offered, it refused.}
{core_recognition <= 0: Dorrik's form has turned the moving wall into a fault belonging to the workshop.}
{witness_chain >= 3: Three kinds of witness hold the same event: goblin sense, expedition presence, and marked stone copied under seal.}
{witness_chain <= 1: One company copy and one hesitant workshop page are all that stand between an answer and a maintenance report.}
{bargain_quality >= 2: Thumbnail crystals remain along the left-hand mark, an offered yield small enough to carry through contested space.}
{alternate_route >= 2: The left gallery is mapped to a retreat pocket and a warm-air seam; Sava can price a bounded expedition instead of guessing at a hole.}
{district_reserve <= 1: A red bead drops on the district gauge. The terraces have entered rationing margin.}
{company_pressure >= 4: Dorrik's messenger tube begins knocking with questions from people who have never stood east of the withdrawal line.}

-> company_demand

=== company_demand ===
Dorrik unhooks the cutter key from his chain and sets it on the hearing desk between them. "If the centre face rests, Terrace Nine loses lift water before the third turn. Your workshop sealed tomorrow's supply. It can seal tonight's shortage too."

Sava leans one gauntlet on the safety rail. "My company can walk the left way. The drill cannot. That seems to be the point."

Pekk watches the answering organ borrow the shape of their breathing.

Nera has one civic seal, one reserve chest, one company contract, and a reply in the wall. The old rite has reached the expensive part.

-> commitment_choice

=== commitment_choice ===
// ghostlight.choice_layer: bind_the_answer
+ [Seal the centre face at rest for three high waters and release the district reserve.]
    // ghostlight.action_label: seal_route
    // ghostlight.branch_label: honor_core_rest
    ~ rest_honored = 1
        {ritual_integrity >= 3 && district_reserve >= 2:
        Nera seats the cutter key in the rest alcove, closes the route book over it, and presses her civic seal into warm wax. The pump stays at zero. The reserve tube opens westward with a sound like a long-held breath.
        -> ending_rest_kept
    - else:
        Nera seals the rest order. The district gauge answers by dropping another bead. Somewhere above, baths empty into buckets and a cold store chooses which merchant to disappoint first.
        -> ending_rest_cost
    }
+ {alternate_route >= 2 || bargain_quality >= 2} [Replace the bore contract with a bounded delve through the left gallery.]
    // ghostlight.action_label: issue_contract
    // ghostlight.branch_label: bargain_for_alternate_delve
    ~ rest_honored = 1
    Nera seals the centre cutter key in the rest alcove, then opens the route book to the left-hand contract page.
        {alternate_route >= 3 && goblin_trust >= 2 && crew_safety >= 3:
        Nera strikes the bore clause from Dorrik's slate. Pekk names the safe pocket. Sava prices four packs, one retreat bell, and the right to return with less than the district wants.
        -> ending_bargain_kept
    - else:
        Nera writes a walking contract for the left way. Too much of the route remains smell, guess, and hopeful ink. Sava signs because the terraces need water and because delvers have always been paid partly in other people's urgency.
        -> ending_bargain_cost
    }
+ {core_recognition >= 2 || witness_chain >= 3} [Send sealed copies to the Rune Colleges, union papers, and Forge temples before the company can close the gate.]
    // ghostlight.action_label: publish_evidence
    // ghostlight.branch_label: publish_personhood_claim
    ~ rest_honored = 1
    Nera seals the centre cutter key in the rest alcove before she reaches for the copying wax.
        {core_recognition >= 2 && witness_chain >= 3 && answering_organ_intact == 1:
        Nera presses three copies: moving strokes, witness marks, and the offered left-hand yield. Sava takes one to the delver hall. Pekk takes one by routes omitted from company maps. The temple copy goes up the public lift.
        -> ending_recognition_kept
    - else:
        Nera sends what she has. One copy lacks Pekk's reading; another lacks the organ's first movement; Dorrik already has a cleaner account moving upward.
        -> ending_recognition_cost
    }
+ [Hand Dorrik the cutter key and force the contracted face.]
    // ghostlight.action_label: transfer_object
    // ghostlight.branch_label: force_the_cut
    ~ rest_honored = 0
    ~ answering_organ_intact = 0
        {crew_safety >= 3:
        Nera gives Sava one look before the key changes hands. The delvers move west. Pekk is already off the niche when the cutter unfolds.
        -> ending_force_evacuated
    - else:
        The key passes. Sava's party is still crowded beside the brass swing gate when the cutter unfolds, because schedules move faster than four tired people in armour.
        -> ending_force_caught
    }

=== ending_rest_kept ===
// ghostlight.ending_label: rest_honored_with_reserve
// ghostlight.training_hook: personhood_as_costly_recognition
The centre arch stays closed.

For three high waters, Terrace Nine climbs stairs, rations lift water, and tells the story according to whether its listeners carried buckets. The reserve holds. The cold stores lose one profitable night and no winter food.

At the gate, the answering organ curls around the sealed cutter key. It copies Nera's mark once, then grows a small fourth space beneath it.

The route book has no heading for a signature that arrives through stone. Nera leaves the space open.
-> last_echo

=== ending_rest_cost ===
// ghostlight.ending_label: rest_honored_under_shortage
// ghostlight.training_hook: recognition_cost_reaches_households
The centre arch stays closed and the district pays immediately.

The upper lifts slow. Cistern queues form along the warm-sea terraces. Dorrik's company funds hand carts for one street and photographers for two. Nera's workshop answers complaints under its own seal.

On the third high water, the left arch gives back enough loose crystal to restart one lift line. Nobody agrees whether this is payment, pity, or the first installment of a bargain.
-> last_echo

=== ending_bargain_kept ===
// ghostlight.ending_label: alternate_delve_agreed
// ghostlight.training_hook: delving_bargain_as_mutual_constraint
Sava's party goes left with Pekk's route, Nera's retreat bell, and a contract that pays them even if they return early.

The gallery raises pale shelled things to contest the offered crystals. It also leaves a retreat pocket open after the first wound. The party comes back with two packs instead of the six the terraces wanted.

Dorrik calls it underproduction. Sava calls it four living delvers and a road still speaking to them tomorrow.
-> last_echo

=== ending_bargain_cost ===
// ghostlight.ending_label: alternate_delve_underread
// ghostlight.training_hook: incomplete_interpretation_costs_bodies
The left way admits feet and punishes guesses.

A warm-air seam becomes a steam vent after the second bend. Sava spends the retreat bell before the first crystal pocket. One delver comes back carried, alive and furious. The centre face remains closed.

Pekk enters the missing smell marks beside Nera's seal. The next contract will pay for listening time before armour.
-> last_echo

=== ending_recognition_kept ===
// ghostlight.ending_label: witnessed_core_answer_published
// ghostlight.training_hook: old_core_enters_public_personhood_dispute
By evening, copies of the wall's answer hang in a delver hall, a union press, and a Forge temple.

The Rune College requests the organ for controlled study. The request uses the word removal. Temple bells answer across three Holds before the college corrects its wording.

The route book names the presence at Three-Bell Gate as an answering neighbour. Law has not caught up. Contracts have: no insurer will cover a forced centre cut while the witnessed null remains alive in stone.
-> last_echo

=== ending_recognition_cost ===
// ghostlight.ending_label: personhood_claim_outpaced
// ghostlight.training_hook: evidence_without_custody
Nera's copies reach the upper lift after Dorrik's report.

The papers print a moving wall beside the phrase LATE-STAGE INSTABILITY. The temple keeps its copy. Pekk's account travels farther underground, where nobody asks a Rune College whether a mouth counts as a mouth.

The gate stays closed for three high waters under Nera's seal. The claim survives mainly in temple custody and Pekk's route copy, while the district's supply order remains painfully unanswered.
-> last_echo

=== ending_force_evacuated ===
// ghostlight.ending_label: forced_cut_after_warning
// ghostlight.training_hook: refused_bargain_becomes_defence
The cutter touches the centre face.

The answering organ tears itself free of bell, slate, and wall. Silver-root threads whip into the direct pump. Sava gets everyone across the brass line before the first black water strikes the safety rail.

Terrace Nine receives one violent surge. Then the gauge falls to nothing. The company owns a cut face, a drowned pump, and a contract whose subject has begun revising the terms with pressure.
-> last_echo

=== ending_force_caught ===
// ghostlight.ending_label: forced_cut_with_crew_inside
// ghostlight.training_hook: ritual_invalidity_hides_injury_until_actuation
The cutter bites.

The centre face opens like an eye closing. Black water takes the service floor to the knee, then the waist. Sava drags one delver through the brass swing gate while Nera strikes a live null across the pump casing with her bare casting hand.

The engine stops. The wall does not. Three bells ring under the water in no order any rite taught.
-> last_echo

=== last_echo ===
{rest_honored == 1:
When the chamber empties, the rest bell keeps a slow pulse under the stone. The gate has retained both halves of the exchange: an answer, and people who changed their work because they heard it.
- else:
When the chamber empties, the bell stones are silent. Far inside the mountain, another piece of the route closes.
}

{answering_organ_intact == 1: Copper and silver threads remain around the route slate, warm as skin and patient as a geological argument.}
{answering_organ_intact == 0: Broken fungal skin floats in the pump water. Every fragment still carries part of the null stroke.}

-> END
