// ghostlight.artifact_id: patina_colour_slips_branch_fold_v0
// ghostlight.fixture_id: patina-colour-slips-v0
// ghostlight.scene_id: patina-colour-slips-v0.morning-lamp-repair
// ghostlight.final_ink_path: examples/ink/delvehold/patina-colour-slips-v0.branch-and-fold.v0.ink

VAR slip_fit = 1
VAR departure_margin = 2
VAR customer_trust = 2
VAR material_spend = 0
VAR apprentice_standing = 2
VAR heat_clue = 0
VAR market_rainbow = 1

-> start

=== start ===
// ghostlight.scene: coppervein_repairs_establishing
Coppervein Repairs opens when Cistern House Nine calls its second pump awake.

The rune shop is wedged beneath the terrace rail stair, one room of carved stone with an arched counter facing the street landing. A sand-filled test cradle is built into the counter's inner edge, where a customer can reach a clamped handle through a shallow slot. Trains murmur overhead. Through the arcade, three pump notes arrive from the cistern house in order, followed by the usual queue of kettles, carriage lamps, warming plates, and people who explain that the damage was present when they became responsible for it.

Kela Ashpin, a young dwarf apprentice with chalk in both cuffs, lifts the shutter. Master Ovra Toll lights the bench mana tap and sets yesterday's tea beside it. The tea has a lid. Ovra believes this makes it workshop equipment.

-> morning_rack

=== morning_rack ===
// ghostlight.scene: coppervein_colour_slip_routine
Thumb-length scraps of stone hang from wire loops above the counter. Each carries one short runic junction in two or three metal fills, a null mark at the end, and a job number scratched into the blank edge.

They are colour slips. A rune-maker cuts one before committing an uncertain mixed-metal seam to a whole object. The little working spends a crystal shaving and a few minutes. If it heats, pits, or stains, it has failed somewhere cheap.

Pump fitters and rail crews bring accepted slips back with the repaired object. At this counter, asking for the same slip means the same substrate and channel sequence as well as the same visible rune.

Kela turns the finished slips inward, job marks facing the room. The pretty failures face the arcade. Ovra calls that education. Kela calls it having a window.

-> senn_arrives

=== senn_arrives ===
// ghostlight.scene: coppervein_rail_lamp_arrival
Senn Brindle ducks under the shutter in a slate-blue rail coat, carrying an inspection lamp by two fingers. He sets the lamp on the counter and its old colour slip beside it.

"Handle warms after three minutes," he says. "Light still behaves. Midday local leaves after the third pump bell."

"The rail office has spares," Kela says.

"The spare has a handle designed by someone who hated gloves. I have gloves. We are doctrinally opposed."

The lamp's rune face has been replaced since its slip was cut. The old witness is pale stone. The new face is dark, close-grained slate carrying the same handsome gold-and-copper turns.

One morning repair, one scheduled train, one object doing its proper work in the wrong part of itself.

-> receiving_choice

=== receiving_choice ===
// ghostlight.choice_layer: ordinary_counter_practice
+ [Lay the old slip against the new rune face and compare the stone, channel depth, and filled turns.]
    // ghostlight.action: compare_objects
    // ghostlight.branch: prime_material_match
    // ghostlight.intent: check_whether_the_returned_witness_matches_the_current_object
    ~ slip_fit = slip_fit + 1
    ~ departure_margin = departure_margin - 1
    ~ apprentice_standing = apprentice_standing + 1
    Kela puts lamp and slip under the same white bench lamp. The runes match at a glance. The cut does not. On the dark slate, the second junction sits half a thumbnail deeper.

    Ovra drinks her tea and lets silence do the marking.

    Senn looks from one stone to the other. "I was told it was the same face."

    "It has the same number of sides," Kela says. "The resemblance is touching."
    -> material_mismatch_fold
+ [Clamp the lamp in the sand cradle and make Senn show where the warmth begins.]
    // ghostlight.action: test_object
    // ghostlight.branch: prime_heat_location
    // ghostlight.intent: turn_the_customer_report_into_a_local_physical_clue
    ~ heat_clue = heat_clue + 2
    ~ departure_margin = departure_margin - 1
    ~ customer_trust = customer_trust + 1
    Kela beds the lamp in the sand cradle with the handle clear. Senn activates it. White light fills the hood above the bench.

    At one minute, nothing. At two, Senn touches the leather wrap. At three, he taps the underside just below the second metal change.

    "There. Warm enough to be annoying. Not warm enough to win an argument with purchasing."

    Kela marks the spot with chalk.
    -> material_mismatch_fold
+ [Put dependable gold fill on the balance and name the fast, expensive repair.]
    // ghostlight.action: quote_material
    // ghostlight.branch: prime_gold_rebuild
    // ghostlight.intent: buy_a_simple_flow_path_with_money_instead_of_more_mixed_metal_diagnosis
    ~ material_spend = material_spend + 2
    ~ departure_margin = departure_margin + 1
    ~ customer_trust = customer_trust - 1
    Kela lays gold wire on the balance until the little brass arm settles.

    Senn reads the weight and removes his cap. "Is this a quote or a ransom note?"

    "A quote. Ransom notes have less arithmetic."

    Gold carries unfamiliar patterns cleanly. It also expects to be paid for having manners.
    -> material_mismatch_fold
+ [Turn the returned slip bright side out in the counter frame and let its coloured seams advertise the repair.]
    // ghostlight.action: display_object
    // ghostlight.branch: prime_market_rainbow
    // ghostlight.intent: use_workshop_display_custom_to_make_the_colourful_job_desirable
    ~ market_rainbow = market_rainbow + 2
    ~ customer_trust = customer_trust + 1
    ~ apprentice_standing = apprentice_standing - 1
    Kela hooks the old slip into the counter frame. Gold and copper catch the lamp light. Two people in the arcade slow down to look.

    Senn brightens. Ovra does not.

    "A market rainbow," Ovra says.

    The phrase can mean lovely work. It can also mean the metals have found employment before their purpose has.
    -> material_mismatch_fold

=== material_mismatch_fold ===
// ghostlight.fold: returned_witness_meets_changed_substrate
Ovra turns the lamp face over. The supplier's replacement mark is cut on the back, small enough to survive a hurried counter.

"Same drawing," she says. "Different stone. The old slip answers the old job."

{slip_fit >= 2: Beside the lamp, pale witness stone and dark replacement slate disagree before any mana enters the room.}
{slip_fit <= 0: The bright old slip has become the easiest object on the counter to believe and the least useful one.}
{heat_clue >= 2: Kela's chalk mark waits below the second change of metal, where the warmth first entered the handle.}
{material_spend >= 2: Enough gold for a complete rebuild rests on the balance, tidy and financially offensive.}
{market_rainbow >= 3: The arcade has acquired an audience for the pretty seams. Beauty has begun charging bench time.}
{apprentice_standing >= 3: Ovra leaves the lamp in Kela's hands.}
{apprentice_standing <= 1: Ovra draws the lamp closer to her side of the bench.}
{departure_margin <= 1: A rail bell sounds above the stair. Senn counts the remaining pump calls under his breath.}

Ovra nods toward the rack of blank scraps. "Which job are you testing?"

-> witness_choice

=== witness_choice ===
// ghostlight.choice_layer: matched_witness_test
+ [Cut a fresh slip from the dark replacement slate and reproduce only the doubtful junction.]
    // ghostlight.action: cut_witness
    // ghostlight.branch: cut_matching_slip
    // ghostlight.intent: test_the_current_substrate_and_seam_before_touching_the_full_graph
    ~ slip_fit = slip_fit + 2
    ~ heat_clue = heat_clue + 1
    ~ material_spend = material_spend + 1
    ~ departure_margin = departure_margin - 1
    ~ apprentice_standing = apprentice_standing + 1
    Kela takes a thumb-length offcut from the replacement slate bin. She cuts the same depth, fills the same gold-and-copper turn, scratches Senn's job number into the blank edge, and closes the little graph with a null.

    The crystal shaving wakes it. A brown heat stain opens beneath the second junction and stops at the null.

    Senn looks at the stain, then at the matching place on his lamp handle.

    "That," he says, "is rude but persuasive."
    -> bench_result_fold
+ [Run the old pale-stone slip once more and use its accepted mark to keep the job moving.]
    // ghostlight.action: reuse_witness
    // ghostlight.branch: trust_old_slip
    // ghostlight.intent: preserve_the_departure_window_by_treating_old_proof_as_current
    ~ slip_fit = slip_fit - 1
    ~ departure_margin = departure_margin + 1
    ~ customer_trust = customer_trust - 1
    Kela clips the old slip to the mana tap. Its short graph lights evenly. The null closes. The stone stays cool.

    Senn reaches for the lamp.

    Ovra puts one finger on its service ring. "You have proved that yesterday remains yesterday. A comfort, I suppose."
    -> bench_result_fold
+ [Lay out a single gold route for the full graph and weigh the metal against the remaining time.]
    // ghostlight.action: redesign_material_path
    // ghostlight.branch: prepare_gold_path
    // ghostlight.intent: replace_the_doubtful_mixed_seam_with_a_dependable_but_costly_carrier
    ~ material_spend = material_spend + 2
    ~ slip_fit = slip_fit + 1
    ~ departure_margin = departure_margin - 1
    ~ market_rainbow = market_rainbow - 1
    Kela traces the lamp's graph on waxed slate and lays one gold wire over every channel. The route is plain, continuous, and heavier than Senn's expression.

    "It will lose the copper colour," Kela says.

    "The train is painted brown," Senn says. "We have endured worse."
    -> bench_result_fold
+ [Set both stones under Ovra's magnifier and ask her to call the first consequential difference.]
    // ghostlight.action: request_instruction
    // ghostlight.branch: compare_with_master
    // ghostlight.intent: spend_schedule_margin_to_turn_the_mismatch_into_teaching
    ~ heat_clue = heat_clue + 2
    ~ slip_fit = slip_fit + 1
    ~ departure_margin = departure_margin - 1
    ~ apprentice_standing = apprentice_standing + 1
    Ovra moves the lens between pale stone and dark slate. Under magnification, the new face shows tiny open grains around the second channel turn.

    She gives Kela a blunt probe. "The drawing crossed. The fill crossed. What did not?"

    Kela touches the porous edge. "The stone held less of the waste. It sent the rest into the housing."

    "Good. Write that before cleverness improves it."
    -> bench_result_fold

=== bench_result_fold ===
// ghostlight.fold: proof_cost_and_departure_pressure
The repair now has three clocks: the lamp's slow warmth, the rail bell above the stair, and Ovra's tea cooling beside the mana tap.

{slip_fit >= 3: A matched dark-stone slip carries Senn's job mark and a visible stain at the doubtful junction.}
{slip_fit <= 1: The accepted pale slip still gleams, but nothing on the bench connects its success to the current lamp face.}
{heat_clue >= 3: Chalk, stain, and porous stone all point to the second mixed-metal turn.}
{heat_clue <= 1: The handle warmed somewhere during use; the bench has not narrowed the cause.}
{material_spend >= 3: The gold route is measured and affordable only in the strict sense that the rail office owns a ledger.}
{market_rainbow >= 3: The old slip remains bright in the counter frame, promising an attractive answer to people who cannot see the mismatch.}
{market_rainbow <= 0: The proposed gold route is almost severe in its plainness.}
{customer_trust >= 3: Senn has stopped watching the clock long enough to watch Kela's hands.}
{customer_trust <= 1: Senn keeps one gloved hand on the lamp as though the workshop might invoice it for being stationary.}
{apprentice_standing >= 4: Ovra slides the job wire and workshop punch across to Kela.}
{apprentice_standing <= 1: Ovra keeps the punch beneath her palm.}
{departure_margin <= 0: The second rail bell sounds. Senn's train has begun taking on passengers.}

The full graph is still open on the sand cradle. Whatever leaves the counter will carry a slip, a price, or an admitted absence of proof.

-> handoff_choice

=== handoff_choice ===
// ghostlight.choice_layer: repair_handoff
+ {slip_fit >= 3 && heat_clue >= 1} [Recut the second junction, run the fresh slip and lamp together, then wire the matched witness to the service ring.]
    // ghostlight.action: complete_matched_repair
    // ghostlight.branch: handoff_fresh_match
    // ghostlight.intent: finish_the_repair_under_current_material_evidence
    {departure_margin >= 1:
        -> ending_fresh_match_on_time
    - else:
        -> ending_fresh_match_late
    }
+ {material_spend >= 2} [Strip the mixed fills, rebuild the channels in gold, and put the weight on the rail office's bill.]
    // ghostlight.action: complete_gold_rebuild
    // ghostlight.branch: handoff_gold_path
    // ghostlight.intent: buy_a_dependable_path_at_visible_material_cost
    {departure_margin >= 1:
        -> ending_gold_on_time
    - else:
        -> ending_gold_late
    }
+ [Leave the lamp open in the sand cradle and refuse to punch an old slip for new stone.]
    // ghostlight.action: withhold_certification
    // ghostlight.branch: handoff_honest_delay
    // ghostlight.intent: protect_the_workshop_mark_when_the_current_job_has_not_been_proved
    {apprentice_standing >= 3 || customer_trust >= 3:
        -> ending_honest_delay
    - else:
        -> ending_counter_quarrel
    }
+ {slip_fit >= 2} [Set old and new slips side by side and ask Senn what the noon departure is worth to him.]
    // ghostlight.action: share_decision_surface
    // ghostlight.branch: handoff_customer_choice
    // ghostlight.intent: let_the_customer_choose_time_or_cost_after_seeing_what_each_witness_proves
    {customer_trust >= 3 && heat_clue >= 2:
        -> ending_shared_choice
    - else:
        -> ending_shared_choice_cost
    }

=== ending_fresh_match_on_time ===
// ghostlight.ending_label: fresh_match_on_time
// ghostlight.training_hook: matched_witness_supports_timely_repair
Kela recuts the second junction a hair shallower and gives its waste somewhere useful to go. Fresh slip and lamp light together. Both close at the null. The handle stays cool through the fourth minute.

After the fourth cool minute, Ovra sets the workshop punch over Kela's job mark. Kela drives it, threads the dark-stone slip through the service ring, and hands lamp and witness across the counter.

Senn reaches the midday local with one bell to spare. The lamp is less colourful at the second turn. Nobody on the platform notices, which is among the better compliments paid to repair work.
-> END

=== ending_fresh_match_late ===
// ghostlight.ending_label: fresh_match_late
// ghostlight.training_hook: correct_work_spends_schedule_margin
Fresh slip and lamp both run cool. Kela wires the matched witness to the service ring just as the midday local rolls above the stair.

Senn watches its carriage lights pass across the arcade ceiling.

"The lamp works," Kela says.

"Splendid. It can help me inspect the next train."

He takes the depot spare back to his office and leaves the repaired lamp for the later local. The lost hour belongs to the job docket beside the new slip, not to a story about careless gloves.
-> END

=== ending_gold_on_time ===
// ghostlight.ending_label: gold_rebuild_on_time
// ghostlight.training_hook: simple_carrier_buys_time_at_material_cost
Gold fills the lamp's channels in one uninterrupted route. The light wakes cleanly. Four minutes pass; the handle keeps the temperature of old leather and Senn's impatience.

Kela weighs the removed mixed fills into the return tray and adds the gold weight to the bill. Senn reads the sum twice.

"This lamp now has a better pension than I do."

He catches the train. The rail office receives a plain lamp, a heavy invoice, and no mystery disguised as thrift.
-> END

=== ending_gold_late ===
// ghostlight.ending_label: gold_rebuild_late
// ghostlight.training_hook: expensive_certainty_can_still_miss_the_clock
The gold route behaves perfectly. It also takes long enough for the second rail bell to become the last one Senn needed.

He holds the cool handle while the train departs overhead, then studies the bill.

"Expensive and late," he says.

Ovra lifts her tea. "Those are separate defects. Keep your records tidy."

Kela wires a plain gold-filled slip to the service ring. The lamp will work on the later local. The morning does not become cheaper because it became certain.
-> END

=== ending_honest_delay ===
// ghostlight.ending_label: honest_delay_supported
// ghostlight.training_hook: withholding_unproved_work_preserves_trust
Kela leaves the workshop punch on its hook.

Ovra turns the old pale slip and the dark lamp face toward Senn. "She is refusing my mark as well as yours. That is why she still has the bench."

Senn groans, takes the short-handled depot spare from his satchel, and puts his gloves on with ceremonial resentment. He catches the train carrying an inferior tool and a repair claim nobody has prettied into certainty.

The lamp remains in the sand cradle. By afternoon it will have a fresh slip or a different graph.
-> END

=== ending_counter_quarrel ===
// ghostlight.ending_label: honest_delay_contested
// ghostlight.training_hook: correct_refusal_without_relational_support
Kela refuses the punch. Senn hears delay, expense, and an apprentice telling him which train to miss.

He asks for the Master.

Ovra comes to the counter, upholds the refusal, and takes the next three sentences herself. Kela keeps the principle and loses counter duty until lunch. Both facts go on the docket because Ovra dislikes lessons that only one person remembers.

Senn leaves with the depot spare. The old colour slip stays beside the new stone, bright enough to show why the argument happened.
-> END

=== ending_shared_choice ===
// ghostlight.ending_label: shared_material_choice
// ghostlight.training_hook: customer_choice_grounded_in_visible_witnesses
Kela sets the pale accepted slip beside the stained dark one. She shows Senn the warm junction on the lamp, the measured gold on the balance, and the remaining bell on the clock.

Senn chooses the depot spare and a fresh matched repair after the train. He signs the delay line himself.

"Same slip next time," he says.

"Same stone too," Kela says.

He points at her with the short spare handle. "Do not ruin a useful phrase by improving it."

The two slips remain on the counter until the choice has names attached to both sides.
-> END

=== ending_shared_choice_cost ===
// ghostlight.ending_label: shared_choice_without_enough_proof
// ghostlight.training_hook: choice_surface_fails_when_evidence_is_weak
Kela puts two slips on the counter. One is accepted and belongs to the old stone. The other has not carried enough of the current fault to answer for it.

Senn sees two pretty scraps and a shrinking departure window. He chooses the old witness, pays for the quick copy, and catches his train.

The lamp returns before evening with the handle warmer and the new job mark hanging beside the old proof. Ovra hangs both bright side inward. Kela spends closing time cutting the slip the morning should have bought.
-> END
