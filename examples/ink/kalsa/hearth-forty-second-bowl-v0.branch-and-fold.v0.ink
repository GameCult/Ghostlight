// ghostlight.artifact_id: kalsa_hearth_forty_second_bowl_branch_fold_v0
// ghostlight.fixture_id: hearth-forty-second-bowl-v0
// ghostlight.scene_id: hearth-forty-second-bowl-v0.evening-custodian-support
// ghostlight.final_ink_path: examples/ink/kalsa/hearth-forty-second-bowl-v0.branch-and-fold.v0.ink

VAR home_security = 2
VAR sera_recovery = 1
VAR record_honesty = 1
VAR watch_coverage = 1
VAR eldest_load = 2
VAR neighbor_credit = 1
VAR pressure_load = 1
VAR bowl_state = 0
VAR service_scope = 2

-> start

=== start ===
Low Sere has forty-one occupied hearths and, every evening, forty-two suppers.

The extra one belongs to whoever is keeping the Ashen Intake from putting hot grit through everybody's floor. It is called the forty-second bowl, because calling it unpaid support labor would make the ration board difficult to decorate.

Tonight the turn belongs to Bel Orra's lower-step hearth.

-> lower_step_hearth

=== lower_step_hearth ===
Bel's room leans into the Warm Steps like a shoulder into bad weather. A shallow channel runs under the threshold, carrying heat down from the Cistern House. The stone beside it is dark with damp. Bedding hangs from a shared rail outside, taking warmth from the channel and mist from everything else.

Nim Orra, Bel's eldest, returns with two drinking vessels balanced against one hip. Nim should have spent the watch thinning greyroot in the beds. Instead there is water to carry, a wet wall to scrape, and the household's own pot making the strained little sound that means supper has become arithmetic.

Bel sets the communal covered bowl beside three things: their pot, a strip marked with the wall's new wet line, and the clay turn-token from the basin table.

"How much loyalty are we cooking?" Nim asks.

"Enough to stop it sticking to the pot."

-> portion_choice

=== portion_choice ===
// ghostlight.choice_layer: household_portion
+ [Fill the forty-second bowl before serving the hearth.]
    // ghostlight.action_label: transfer_object
    // ghostlight.branch_label: portion_full_first
    ~ bowl_state = 2
    ~ sera_recovery = sera_recovery + 1
    ~ home_security = home_security - 1
    ~ eldest_load = eldest_load + 1
    Bel fills the covered bowl first: greyroot, reed greens, and the soft middle of the last ash loaf.

    Nim looks into the family pot afterward. "Good. I had been worried the bottom might go unfed."

    "The bottom has an appointment."

    "Permanent?"

    "Do not start."
    -> home_fold
+ [Serve the hearth, then mark an honest short portion for Sera.]
    // ghostlight.action_label: record_and_transfer
    // ghostlight.branch_label: portion_marked_short
    ~ bowl_state = 1
    ~ record_honesty = record_honesty + 2
    ~ home_security = home_security + 1
    Bel serves their own supper and scores the remaining portion onto the dampness strip before lidding it.

    The bowl is short. The mark is not apologetic.

    Nim nods. "Will Maro count the writing as food?"

    "If he does, he may eat it."
    -> home_fold
+ [Ask the next-door hearth to cover half, and mark the exchange on both tokens.]
    // ghostlight.action_label: negotiate
    // ghostlight.branch_label: portion_neighbor_exchange
    ~ bowl_state = 2
    ~ neighbor_credit = neighbor_credit + 2
    ~ record_honesty = record_honesty + 1
    ~ eldest_load = eldest_load - 1
    Bel crosses the shared drying rail and comes back with a ladle of moss broth from the next hearth. She marks both clay tokens before mixing it with her own pot.

    The meal becomes respectable by committee.

    "What do we owe?" Nim asks.

    "One bed watch when their reed cough returns."

    That is how neighbors keep accounts when coins would only get wet.
    -> home_fold
+ [Carry the empty communal bowl with the household's food and labor marks inside it.]
    // ghostlight.action_label: withhold_object
    // ghostlight.branch_label: portion_return_empty
    ~ bowl_state = -1
    ~ record_honesty = record_honesty + 2
    ~ pressure_load = pressure_load + 1
    ~ home_security = home_security + 1
    Bel leaves the household pot intact. Into the communal bowl she places the dampness strip, the descent-food mark, and the work chit showing Nim's missed Grey Bed watch.

    The lid closes over a meal made entirely of evidence.

    Nim lifts it. "Light."

    "Maro likes a burden he can carry."
    -> home_fold

=== home_fold ===
// ghostlight.fold: supper_and_household_cost
The Warm Steps are entering evening by the time Bel leaves. In Low Sere, evening is less a change in sunlight than a change in which tasks people admit they will not finish.

{bowl_state >= 2: The covered bowl is heavy and leaks a promising thread of broth down Bel's thumb.}
{bowl_state == 1: The bowl is light enough to accuse the arm carrying it; the short portion is plainly scored on the strip beneath the lid.}
{bowl_state < 0: The empty bowl clicks around its clay and cloth evidence with every step.}
{home_security <= 1: Behind Bel, Nim divides a thin household supper and puts another vessel beneath the wettest wall.}
{home_security >= 3: Their own pot remains enough for the hearth, if the wall does not worsen.}
{neighbor_credit >= 3: A second hearth has put food into the bowl and a future claim into Bel's hands.}
{eldest_load >= 3: Nim is carrying the water, the wall, and the joke about it. The joke is the lightest item.}

The Cistern House waits uphill, mist beading along its beams.

-> warm_steps_route

=== warm_steps_route ===
// ghostlight.visual_scene: warm_steps_route
Bel climbs past steaming channels, shared bedding rails, and doors whose occupants know the shape of the communal bowl from across the Steps.

The Cistern House grows through the mist one heavy beam at a time.

-> cistern_arrival

=== cistern_arrival ===
The broad stone lip between the settling basins is dry enough for a ration board, six witnesses, or one argument with excellent posture.

Warm grey water moves through the east basin. The west basin is drained. Behind its iron grate, the black pressure door shows three pale shutter marks. Sera Venn kneels at the sampling shelf outside the grate, one hand around a warm ceramic vessel, the other copying ash color onto a service strip.

Maro Seln stands at the drain wheel with the supper ledger. As cistern reeve, he can move water and marks. Only one of those obeys him reliably.

"Full turn?" he asks Bel.

Sera does not look up. "Ask after you see the bowl. That is why bowls have lids."

-> delivery_choice

=== delivery_choice ===
// ghostlight.choice_layer: custodian_delivery
+ {bowl_state >= 1} [Put the bowl into Sera's hands before anyone discusses the ledger.]
    // ghostlight.action_label: transfer_object
    // ghostlight.branch_label: delivery_feed_first
    ~ sera_recovery = sera_recovery + 2
    ~ pressure_load = pressure_load + 1
    Bel carries the bowl around the dry edge of the west basin and puts it against Sera's free palm.

    "Hot," Sera says.

    "That was among its ambitions."

    Sera drinks before Maro can turn care into a completed line. The sampling vessel cools beside her while she does.
    -> delivery_fold
+ [Read the wet-wall report aloud while Sera compares the water sample.]
    // ghostlight.action_label: speak
    // ghostlight.branch_label: delivery_household_report
    ~ record_honesty = record_honesty + 1
    ~ home_security = home_security + 1
    ~ watch_coverage = watch_coverage + 1
    Bel reads the new wet line, the two carried vessels, and Nim's missed bed watch.

    Sera lifts the sample toward the lamp. "Lower channel is taking ash again. Not much. Enough that tomorrow will lie about tonight if we do not mark it."

    Maro writes. His stylus has the wounded expression of a tool being used as intended.
    -> delivery_fold
+ [Make Maro mark the bowl's real state before lifting the lid.]
    // ghostlight.action_label: show_object
    // ghostlight.branch_label: delivery_mark_first
    ~ record_honesty = record_honesty + 2
    ~ pressure_load = pressure_load + 1
    Bel sets the bowl on the ledger board and keeps one palm on the lid.

    "Full, short, exchanged, refused, or returned," she says. "Pick the one that happened."

    {bowl_state >= 2: Maro marks full.}
    {bowl_state == 1: Maro marks short, pressing hard enough to insult the clay.}
    {bowl_state < 0: Maro marks returned. The word makes the room colder without changing the water.}

    Only then does Bel move her hand.
    -> delivery_fold
+ {bowl_state >= 1} [Take Sera's screen scraper and clear the safe outer screen while she eats.]
    // ghostlight.action_label: use_object
    // ghostlight.branch_label: delivery_work_for_rest
    ~ sera_recovery = sera_recovery + 1
    ~ watch_coverage = watch_coverage + 2
    ~ eldest_load = eldest_load + 1
    ~ pressure_load = pressure_load - 1
    Bel takes the long-handled scraper from its peg. She stays outside the grate and draws mineral paste from the outer screen into a catch tray, work any instructed maintainer may do.

    Sera sits on the low ledge and eats with both hands. For three minutes the custodian's most technical act is chewing.

    "You missed a corner," Maro says.

    Bel offers him the scraper without turning. He discovers the corner was ceremonial.
    -> delivery_fold

=== delivery_fold ===
// ghostlight.fold: meal_record_and_safe_work
Sera checks taste, heat, ash, and sound. Bel watches from the basin lip. Maro finishes one mark and leaves the next space empty.

{sera_recovery >= 3: Color has returned beneath Sera's eyes; she holds the sample steady instead of bracing it against the shelf.}
{sera_recovery <= 1: The bowl is still shut or still empty, and Sera's hand trembles once when she reaches for the sampling vessel.}
{record_honesty >= 4: The board now shows household damp, missed labor, and the bowl's actual state in separate marks.}
{watch_coverage >= 3: Outer-screen ash sits in the catch tray and Bel can describe exactly what Sera inspected.}
{pressure_load >= 3: Delay and argument have collected at the black door like another mineral crust.}

Then the pressure seal knocks twice.

-> pressure_knock

=== pressure_knock ===
// ghostlight.visual_pivotal_beat: double_pressure_knock
The first knock stills the room. The second makes the sample ring against its ceramic dish.

No shutter mark closes. All three pale marks remain above the door. Sera smells the sample, touches one finger to the ledge, and points at the water-cut bell.

"Lower channels off for one short interval. I need a hearth witness while I isolate the west basin."

Maro reaches for the drain wheel.

-> nim_arrival

=== nim_arrival ===
// ghostlight.visual_scene: nim_arrival
Nim arrives at the outer doorway with damp knees and no drinking vessels. "The wall is running now," Nim says. "I moved the bedding. I cannot move the bed alone."

The forty-second bowl has purchased exactly the interval everyone hoped would remain theoretical.

-> witness_choice

=== witness_choice ===
// ghostlight.choice_layer: short_witness_interval
+ [Stay at Sera's floor mark and send Nim home to protect the bedding.]
    // ghostlight.action_label: stay
    // ghostlight.branch_label: witness_stay_send_eldest
    ~ watch_coverage = watch_coverage + 2
    ~ home_security = home_security - 1
    ~ eldest_load = eldest_load + 2
    Bel plants both feet at the chalk witness mark outside the grate.

    "Keep the water off the blankets," she tells Nim. "Wait for another pair of hands before you lift the bed."

    Nim opens their mouth, thinks better of spending breath on whether the box is dry, and runs downhill.

    Sera closes the isolation catch. Bel names the sound and the work interval. Maro turns the wheel.
    -> witness_fold
+ [Send Nim along the lower rail to call the neighboring hearths while Bel witnesses.]
    // ghostlight.action_label: delegate
    // ghostlight.branch_label: witness_call_neighbors
    ~ neighbor_credit = neighbor_credit + 2
    ~ watch_coverage = watch_coverage + 1
    ~ eldest_load = eldest_load + 2
    ~ home_security = home_security + 1
    "Two people to the wall, one to the bed, nobody touches the channel," Bel says.

    Nim repeats it exactly and runs along the lower rail where every hearth can hear a heel strike.

    Bel remains at the chalk mark. Help will reach the room. The debt will reach it a little earlier.
    -> witness_fold
+ [Go home with Nim and require Sera to narrow service until another witness arrives.]
    // ghostlight.action_label: withdraw
    // ghostlight.branch_label: witness_protect_home
    ~ home_security = home_security + 2
    ~ watch_coverage = watch_coverage - 1
    ~ record_honesty = record_honesty + 1
    ~ service_scope = 1
    ~ pressure_load = pressure_load - 1
    "My witness interval ends here," Bel says. "Write why."

    Sera nods before Maro can object. She leaves the west basin isolated and narrows the outflow instead of attempting the fuller service check alone.

    Lower rooms will cool. Bel's bed may still be worth saving.
    -> witness_fold
+ [Make Maro call a dry-step replacement before Bel leaves the doorway.]
    // ghostlight.action_label: compel_recorded_support
    // ghostlight.branch_label: witness_call_dry_replacement
    ~ record_honesty = record_honesty + 2
    ~ watch_coverage = watch_coverage + 1
    ~ home_security = home_security + 1
    ~ pressure_load = pressure_load + 1
    Bel stands in the outer doorway where Maro must look past her to see Nim's soaked knees.

    "The upper steps receive the same heat," she says. "Call one of them before I move."

    Maro strikes the smaller clay turn bell in the dry-step pattern. The delay enters the record because Sera makes him say the time aloud.
    -> witness_fold

=== witness_fold ===
// ghostlight.fold: bounded_care_under_pressure
The west basin quiets. Maro's wheel chain stops shaking. Sera keeps the pressure door shut.

{watch_coverage >= 3: The isolation has a witness account: two knocks, three pale marks, one closed catch, one turned wheel, no heroic additions.}
{watch_coverage <= 1: Sera has narrowed the service because a provisional custodian working alone is not evidence of safe continuity.}
{service_scope <= 1: Grey Bed flow and lower-step wash water remain cut; the basin is safer and the households are colder.}
{home_security >= 4: Help or Bel herself has reached the running wall before the bed takes the full leak.}
{home_security <= 1: Bel can picture the room downhill: wet bedding, a thin pot, and Nim trying to be several adults in a very small space.}
{eldest_load >= 4: Nim's help has become the household's hidden relief shift.}
{neighbor_credit >= 3: Other lower-step hearths have entered the problem. Their care arrives with names and future claims, not as weather.}
{pressure_load >= 3: The machinery is stable for the moment; the social pressure has merely changed vessels.}

Maro sets the clay turn-token beside the ledger.

"I can close this as a full turn," he says. "Sera was fed. The basin was witnessed. Nobody needs one more argument before the descent."

Sera wipes ash from her fingers. "Something being true in pieces does not make the whole sentence true."

Bel can hear water running where it should not, far down the steps.

-> final_choice

=== final_choice ===
// ghostlight.choice_layer: obligation_record
+ [Mark the turn full and protect Sera's office from one more provisional failure.]
    // ghostlight.action_label: sign_record
    // ghostlight.branch_label: close_for_personal_loyalty
    {bowl_state >= 2 && sera_recovery >= 3 && watch_coverage >= 3:
        Bel presses the full-turn edge into the clay. Tonight, at least, the mark describes food delivered, rest purchased, and work witnessed.
        -> ending_loyalty_held
    - else:
        Bel presses the full-turn edge into the clay. The mark covers what the household could not supply and what the office asked anyway.
        -> ending_loyalty_hidden_cost
    }
+ [Mark the bowl, witness interval, missed work, and wet wall separately.]
    // ghostlight.action_label: correct_record
    // ghostlight.branch_label: close_with_honest_care
    {record_honesty >= 4 && watch_coverage >= 2:
        Bel makes Maro leave four marks where he wanted one. The record becomes uglier and more useful.
        -> ending_honest_care_held
    - else:
        Bel tries to separate the costs, but too much of the evening was carried in speech and tired memory.
        -> ending_honest_care_cost
    }
+ [Take the turn-token home and hold the lower-step witness until the room is safe.]
    // ghostlight.action_label: withdraw_authority
    // ghostlight.branch_label: close_for_home
    {home_security >= 4 && pressure_load <= 3:
        Bel lifts the token from the board. "You may have our witness again when our room has a floor."
        -> ending_home_held
    - else:
        Bel takes the token, but the leak and the intake have both outrun the clean edge of refusal.
        -> ending_home_cost
    }
+ [Divide the remaining watch into named quarter-turns among the lower-step hearths.]
    // ghostlight.action_label: organize_mutual_aid
    // ghostlight.branch_label: close_with_neighbors
    {neighbor_credit >= 3 && record_honesty >= 3:
        Bel names the hearths, the intervals, and the debts. Maro has to write quickly enough to become briefly democratic.
        -> ending_neighbors_held
    - else:
        Bel asks for shared quarters, but the neighbors have not been called or the debts were never made legible.
        -> ending_neighbors_cost
    }

=== ending_loyalty_held ===
// ghostlight.ending_label: personal_loyalty_supported
// ghostlight.training_hook: care_can_support_without_appointing
Sera eats the last spoonful cold. The basin remains isolated. Bel's witness account has edges another person can inspect.

"The bowl does not appoint you," Bel says.

"Good," Sera says. "It would be overqualified."

Bel goes home with an empty vessel and a full-turn mark that happens, for once, to be true. Nim has blocked the bed on fired clay. Supper is thin. The wall is not winning yet.

Loyalty cost them food and work. It did not cost them the right to name either.
-> END

=== ending_loyalty_hidden_cost ===
// ghostlight.ending_label: personal_loyalty_hidden_cost
// ghostlight.training_hook: affection_launders_support_failure
The board says full turn.

The bowl was short, or Sera barely ate, or Bel left before the witness interval had a safe end. The mark takes these different failures and gives them one respectable coat.

Sera watches Bel's hand lift from the clay. "They will use that against the next short bowl."

Downhill, Nim is still moving furniture through water. Bel has protected Sera from tonight's accusation by making tomorrow's household harder to believe.
-> END

=== ending_honest_care_held ===
// ghostlight.ending_label: honest_care_supported
// ghostlight.training_hook: separate_records_preserve_both_care_and_cost
The board shows a bowl, a portion, a witness interval, a wet wall, and missed Grey Bed labor as five facts with no appetite for becoming one.

Maro stares at it. "This will take half the basin table to review."

"Then half the basin table may finally attend supper," Bel says.

Sera's next support turn goes to a dry-step hearth. Nim's missed bed watch is credited. Bel goes downhill while there is still a bed to save.

The record does not love anyone. It simply refuses to eat them.
-> END

=== ending_honest_care_cost ===
// ghostlight.ending_label: honest_care_incomplete
// ghostlight.training_hook: unpreserved_care_becomes_contested_memory
Bel can name the costs, but cannot prove which part of the watch held and which part only looked held from the ledger side.

Maro marks short turn. Sera adds a note in her own hand. Neither record compels a replacement before the next interval.

At home, Nim asks whether telling the truth helped.

"It survived," Bel says.

For tonight, that is not the same thing.
-> END

=== ending_home_held ===
// ghostlight.ending_label: household_boundary_supported
// ghostlight.training_hook: bounded_withdrawal_forces_service_scope
Bel and Nim reach the room before the bed frame settles into the leak. They lift it onto fired blocks, roll the bedding tighter, and eat from the household pot while standing.

Above, Sera keeps the west basin isolated. Grey Bed flow remains cut. The intake serves less because the people supporting it had less to give.

No one calls this sabotage while Bel still holds the clay token.

The room is colder. It is also theirs enough to defend.
-> END

=== ending_home_cost ===
// ghostlight.ending_label: household_boundary_too_late
// ghostlight.training_hook: private_rescue_after_hidden_overload
Bel takes the token home as if authority were light enough to outrun water.

The bed leg has slipped. The lower blanket is soaked. Nim is furious in the careful way of someone too tired to spend the whole anger.

Above, the Cistern House has neither Bel's witness nor a replacement. Sera narrows service late.

Two emergencies survive. Neither becomes smaller merely because Bel chose one.
-> END

=== ending_neighbors_held ===
// ghostlight.ending_label: named_mutual_aid_supported
// ghostlight.training_hook: belonging_as_recorded_reciprocity
The remaining interval breaks into four named pieces.

One neighbor witnesses the catch. Another brings broth. A third goes downhill with Nim to lift the bed. Bel holds the fourth piece until the replacement arrives. Each debt is marked beside the turn it actually bought.

Sera receives care without becoming a private dependent of Bel's hearth. Bel receives help without pretending it fell from the mist.

By midnight the communal bowl has visited three rooms and acquired a chip. Low Sere will mend the bowl. The debts are supposed to remain visible.
-> END

=== ending_neighbors_cost ===
// ghostlight.ending_label: unnamed_mutual_aid_failure
// ghostlight.training_hook: solidarity_requires_prior_signal_and_account
Bel names quarter-turns into a room that has not heard itself asked.

One neighbor comes late. Another thought the bowl was full. Maro cannot tell whether he is recording exchange, refusal, or wishful arithmetic.

Nim returns home alone. Sera remains at the black door with a cooling portion and a support plan made mostly of good character.

Bel learns the mean little difference between a neighborhood and people who happen to live close enough to hear you fail.
-> END
