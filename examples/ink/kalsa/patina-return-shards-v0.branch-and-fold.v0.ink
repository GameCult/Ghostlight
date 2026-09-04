// ghostlight.artifact_id: kalsa_patina_return_shards_branch_fold_v0
// ghostlight.fixture_id: patina-return-shards-v0
// ghostlight.scene_id: patina-return-shards-v0.upper-shelf-departure
// ghostlight.final_ink_path: examples/ink/kalsa/patina-return-shards-v0.branch-and-fold.v0.ink

VAR shard_clarity = 1
VAR host_trust = 2
VAR witness_confidence = 1
VAR departure_margin = 2
VAR route_support = 0
VAR claim_custody = 0
VAR mutual_face = 1
VAR water_settled = 0

-> start

=== start ===
The Upper Shelf is where Low Sere keeps everything that has not yet earned a place downhill: guests, handcarts, suspect salvage, and opinions about the price of water.

Three stone guest sheds lean into the slope beneath one patched awning. Below them, warm mist rises from the covered cistern roofs. At the shelf's exposed lip stands the road marker, a waist-high dark slab pricked with shallow sockets. Half-shards of grey pottery sit in some of them like broken teeth.

Sava Ren has spent three work-watches selling rivets, lamp hooks, and two pans that were described as repaired with enough conviction to become repaired by the end of the sale. Now a departure group is tightening load cords on the track above.

If Sava misses them, the next safe company may be another work-watch away.

-> people_and_custom

=== people_and_custom ===
Olan Ves sponsored Sava's stay. He works the Grey Beds downhill, where warm spent water grows food and gives every shirt a permanent argument with moss. Sponsorship put Sava's drinking water against Olan's allotment and made him responsible for the warnings on the promised way out.

Nemi Cal is the water witness closing the claim. Nemi sits on the marker bench with a covered guest vessel at one side and a square of clean cloth at the other. The cloth is for shard halves. The lid on the water is for people who think dust respects procedure.

Sava's half hangs from a cord at the throat: thumb-long greyware, snapped down the middle, Olan's water mark cut across one face and a line scratched toward the exit they named at arrival.

"Let the break remember," Nemi says.

"It has had three watches to improve its handwriting," Sava says.

Olan lifts the guest vessel. "The clay has been very discreet about your second cups."

This is the ordinary part. First they settle the stay. Then the halves meet. Then everybody gets to pretend clay was easier than trust.

-> departure_preparation

=== departure_preparation ===
// ghostlight.choice_layer: ordinary_departure_preparation
+ [Set the carried shard on Nemi's clean cloth with the break edge facing the marker.]
    // ghostlight.action_label: use_object
    // ghostlight.branch_label: prepare_visible_shard
    ~ shard_clarity = shard_clarity + 2
    ~ witness_confidence = witness_confidence + 1
    Sava unties the cord and lays the half-shard down where all three can see its rough edge, water mark, and exit scratch.

    Nemi turns it once with a dry fingertip. "A traveller who presents the break before the story. Civilisation may yet survive us."

    "Do not tell the pans," Sava says. "They are expecting collapse."
    -> preparation_fold
+ [Help Olan count the guest vessel and ash-scrubbing work against the water used.]
    // ghostlight.action_label: touch_object
    // ghostlight.branch_label: settle_water_first
    ~ water_settled = 1
    ~ host_trust = host_trust + 1
    ~ witness_confidence = witness_confidence + 1
    ~ departure_margin = departure_margin - 1
    Sava lifts the vessel while Olan reads the old fill marks and the fresh scrape work.

    The arithmetic is local and practical: drinking, washing, one cracked lid replaced, two guest-shed channels scrubbed clear of ash.

    Olan taps the last mark. "Water settled."

    Above them, a load cord snaps tight. Sava's margin becomes one knot shorter.
    -> preparation_fold
+ [Ask Nemi to read the named exit back from the road marker before closing anything.]
    // ghostlight.action_label: speak
    // ghostlight.branch_label: prepare_route_readback
    ~ route_support = route_support + 1
    ~ witness_confidence = witness_confidence + 1
    Nemi traces the shallow route scratch on Sava's carried half, then points from the marker to the worn track climbing past the sheds.

    "The shelf track to the high cut. Olan gives current warnings or walks you past marks that have gone doubtful."

    Olan nods. "The high cut is still mine to answer for."

    The road has not become safer. It has become harder to rename.
    -> preparation_fold
+ [Ask Olan whether the sponsored water includes enough to wash the custom off afterward.]
    // ghostlight.action_label: speak
    // ghostlight.branch_label: preserve_mutual_face
    ~ mutual_face = mutual_face + 2
    ~ host_trust = host_trust + 1
    Olan considers this with the gravity of a man pricing civic reform.

    "No. That is basin water. Upper Shelf customs leave a mineral film."

    Nemi keeps a straight face for almost a whole breath.
    -> preparation_fold

=== preparation_fold ===
// ghostlight.fold: ordinary_departure_closure
The departure group above moves to the last load. A handcart wheel complains against stone. Nobody hurries the closing; hurried shards have a reputation for remembering only the richest person present.

{shard_clarity >= 3: Sava's half lies exposed on the clean cloth, its water mark, route line, and jagged middle easy to compare.}
{water_settled == 1: The guest vessel stands empty and counted. Whatever happens to the pottery, Sava's water use has an account of its own.}
{route_support >= 1: Nemi's spoken route remains between them: shelf track, high cut, current warnings.}
{mutual_face >= 3: Olan and Sava still have enough humor to disagree without making the first sentence a weapon.}

Nemi reaches into the road marker and draws out the half held under Olan's water mark.

-> mismatch_reveal

=== mismatch_reveal ===
The two halves touch.

They do not fit.

The water mark crosses both faces. The route scratches lean the same way. The broken edges meet at two points and leave a clean little mouth between them.

Olan says, "That is not the half I watched them set."

Nemi says, "Good. We have begun with the only sentence the clay can confirm."

On the upper track, the departure group starts moving. Missing them now means another work-watch on the Shelf, another drawing of water, and a delay Sava's next buyers will improve into a moral flaw.

The mismatch proves no theft. It also closes nothing.

-> mismatch_choice

=== mismatch_choice ===
// ghostlight.choice_layer: mismatched_return_shards
+ [Place both halves in separate marker sockets and ask Nemi to keep them in public view.]
    // ghostlight.action_label: move_object
    // ghostlight.branch_label: place_claim_in_witness
    ~ claim_custody = 1
    ~ shard_clarity = shard_clarity + 2
    ~ witness_confidence = witness_confidence + 2
    ~ departure_margin = departure_margin - 1
    Sava slides the carried half into an empty socket. Nemi places the marker half beside it, not touching.

    "Two pieces, two accounts," Nemi says. "Nobody's pocket gets to become the archive."

    Olan looks up at the departing loads and then back at the visible gap. "That sentence has excellent timing."
    -> mismatch_fold
+ [Hand the carried half to Olan and let the sponsor inspect the break.]
    // ghostlight.action_label: transfer_object
    // ghostlight.branch_label: let_host_inspect
    ~ claim_custody = 2
    ~ host_trust = host_trust + 2
    ~ witness_confidence = witness_confidence - 1
    ~ mutual_face = mutual_face + 1
    Olan cups both halves without forcing them. Grey Bed grit has settled into the lines of his hands.

    He turns the carried piece, tries the opposite angle, and stops before persistence becomes evidence.

    "Still wrong," he says, quieter.

    Nemi holds out the clean cloth. "Then put the wrongness somewhere all three of us can reach."
    -> mismatch_fold
+ [Compare the temper, soot, and scored edge with Nemi before anyone explains the gap.]
    // ghostlight.action_label: inspect_object
    // ghostlight.branch_label: inspect_material_difference
    ~ shard_clarity = shard_clarity + 3
    ~ witness_confidence = witness_confidence + 1
    ~ departure_margin = departure_margin - 1
    Nemi turns both halves under the shelf lamp.

    Sava's piece carries fine black soot in the break. The marker piece carries pale grit packed after firing. One lay against a cooking wall. One spent time near the road.

    That establishes two histories and identifies neither hand.

    The departure group's last cart creaks onto the upper bend.
    -> mismatch_fold
+ [Ask Olan only this: which way out can he competently answer for now?]
    // ghostlight.action_label: speak
    // ghostlight.branch_label: test_present_route
    ~ route_support = route_support + 3
    ~ host_trust = host_trust + 1
    Olan looks away from the clay and toward the track.

    "The high cut is passable, but the first warning stone above the awning has rolled face-down. I can walk you beyond it. The basin stair is longer and marked clean."

    Nemi repeats both routes before either can become the version everyone always knew.
    -> mismatch_fold

=== mismatch_fold ===
// ghostlight.fold: open_claim_before_departure
Cold mist beads on the marker's upper edge. The departure group is nearly beyond easy calling.

{claim_custody == 1: Both halves remain in separate public sockets under Nemi's eye. The gap is now part of the Shelf, not a private accusation.}
{claim_custody == 2: Olan returns both halves to Nemi's cloth. His inspection has cost him the comfort of saying he never held the mismatch.}
{claim_custody == 0: Nemi keeps the marker half on the cloth while Sava still holds the carried piece. The claim remains divided exactly as the custom intended.}
{shard_clarity >= 4: Soot, grit, mark, line, and fracture can be described separately. A later hearing will have more than injured confidence to work with.}
{route_support >= 3: Olan has named one doubtful warning stone, one longer marked route, and the escort he can provide.}
{departure_margin <= 0: The last departing cart has vanished around the upper bend. Speed is no longer available, though people continue proposing it.}
{mutual_face >= 3: Nobody has yet mistaken humiliation for a necessary opening statement.}

Nemi lifts the clean cloth by its corners.

"The break remembers a disagreement," Nemi says. "Now decide what the living are willing to spend on it."

-> final_choice

=== final_choice ===
// ghostlight.choice_layer: claim_or_departure
+ [Stay for a basin-table hearing and leave both halves with Nemi in separate sockets.]
    // ghostlight.action_label: speak
    // ghostlight.branch_label: prioritize_witnessed_hearing
    {claim_custody == 1 && shard_clarity >= 4:
        Sava names the water, route, delay, and two material histories while all three still stand beside the marker.
        -> ending_hearing_success
    - else:
        Sava asks for a hearing with more certainty than the pieces can presently support.
        -> ending_hearing_cost
    }
+ [Break a fresh pair for the route Olan can answer for today, leaving the old mismatch open.]
    // ghostlight.action_label: use_object
    // ghostlight.branch_label: prioritize_amended_departure
    {route_support >= 3 && host_trust >= 3:
        Nemi takes a blank grey-ash tally while Olan points to the route he will walk.
        -> ending_departure_success
    - else:
        Nemi reaches for a blank tally, but the new promise is not yet clearer than the old one.
        -> ending_departure_cost
    }
+ [Spend the remaining margin searching the guest sheds and marker hood for the missing mate.]
    // ghostlight.action_label: search
    // ghostlight.branch_label: prioritize_practical_search
    {departure_margin >= 1 && mutual_face >= 3:
        Sava rolls up both sleeves. Olan takes the middle shed. Nemi checks the marker without surrendering the cloth.
        -> ending_search_success
    - else:
        Sava begins the search after time or patience has already been spent elsewhere.
        -> ending_search_cost
    }
+ [Return the carried half to witness, refuse a substitute, and leave without claiming the sponsorship is closed.]
    // ghostlight.action_label: transfer_object
    // ghostlight.branch_label: prioritize_independent_exit
    {water_settled == 1 && witness_confidence >= 2 && route_support >= 1:
        Sava puts the carried half on Nemi's cloth and shoulders the travel roll.
        -> ending_independent_success
    - else:
        Sava returns the half and turns toward the track before the water and route account can stand on their own.
        -> ending_independent_cost
    }

=== ending_hearing_success ===
// ghostlight.ending_label: witnessed_hearing_success
// ghostlight.training_hook: broken_receipt_preserves_dispute
Nemi leaves the halves in separate sockets and ties the clean cloth across them. The mark remains visible. So does the gap.

Sava misses the departure group. Olan cannot sponsor another outsider until the hearing closes what remains owed. Another work-watch of water goes against the open claim.

The cost is irritating, public, and finite. This is the custom functioning at its least glamorous.

At the basin table, soot will not be asked to prove motive. Pale grit will not be promoted to witness. Three people will have to give accounts beside pieces that refuse to flatter any of them.

"Let the break remember," Sava says.

Olan grimaces. "It could remember faster."
-> END

=== ending_hearing_cost ===
// ghostlight.ending_label: witnessed_hearing_cost
// ghostlight.training_hook: procedure_without_material_clarity
Nemi keeps the halves apart and opens a claim with little beyond a bad fit and three offended memories.

The departure group goes. Sava owes time. Olan loses the standing to host another traveller until the hearing. Nemi will spend a basin-table interval explaining that a mismatch is evidence of mismatch, a distinction institutions enjoy most when someone else must miss supper for it.

The ritual prevents a convenient lie. It cannot manufacture a useful account.
-> END

=== ending_departure_success ===
// ghostlight.ending_label: amended_departure_success
// ghostlight.training_hook: new_route_without_erasing_old_claim
Nemi keeps the mismatched halves separate under cloth. Then a fresh grey-ash tally takes Olan's water mark and a new scratch for the longer basin track.

The new pair breaks cleanly. One half enters the marker. Sava carries the other. It records today's escort; it does not pretend yesterday's pair closed.

Olan walks Sava down the longer basin stair past its clean marks. The face-down stone remains above them. They miss the group but gain the lower crossing before the cold mist thickens.

Behind them, the old gap remains visible. Convenience has been given its own shard instead of being allowed to edit the first.
-> END

=== ending_departure_cost ===
// ghostlight.ending_label: amended_departure_cost
// ghostlight.training_hook: substitute_receipt_compounds_ambiguity
The blank tally waits in Nemi's hand.

Olan cannot yet name whether he is promising the old route, the longer route, or merely movement. Sava wants the departing backs on the upper bend to become a plan.

Nemi breaks nothing.

"A second pair can record a second promise," Nemi says. "It cannot make the first pair less wrong."

By the time they agree with the sentence, the departure group is gone and the water claim is still open.
-> END

=== ending_search_success ===
// ghostlight.ending_label: practical_search_success
// ghostlight.training_hook: low_stakes_repair_exposes_second_claim
They find the mate behind the marker's shallow rain hood, jammed against the stone by a curled shaving of grey clay.

It fits Sava's carried half. The water mark crosses. The route scratch meets. Nemi closes Sava's stay while Olan calls after the last cart until somebody above answers with an impatient wave.

The piece first drawn from Olan's socket still fits nobody present.

Sava leaves with the group. Olan remains unable to sponsor another traveller. A successful search has closed one small account and uncovered another. Low Sere, being inhabited, considers this a mixed result.
-> END

=== ending_search_cost ===
// ghostlight.ending_label: practical_search_cost
// ghostlight.training_hook: search_spends_time_without_forcing_answer
They search the sockets, the marker hood, the guest-shed gutters, the bench joints, and the ash around the covered vessel.

They find one bent hook, two old route cords, and a beetle with no standing before the basin table.

The departure group vanishes. The wrong halves remain wrong. Olan's patience thins; Sava's humor goes with the carts; Nemi wraps the pieces separately.

An honest search is still allowed to fail. This is less satisfying than most customs advertise.
-> END

=== ending_independent_success ===
// ghostlight.ending_label: independent_exit_success
// ghostlight.training_hook: traveller_refuses_false_closure
Nemi accepts Sava's half into witness without calling the sponsorship closed. The settled guest vessel keeps water separate from the broken receipt. The route readback keeps the promised exit separate from Olan's reputation.

Sava leaves under the current marks, alone and later than planned. Olan still owes an account for the marker half. Nemi still owes both of them a hearing that does not turn self-departure into confession.

The Upper Shelf watches Sava go. No half in a pocket is allowed to announce that everybody agreed.
-> END

=== ending_independent_cost ===
// ghostlight.ending_label: independent_exit_cost
// ghostlight.training_hook: autonomy_without_separated_obligations
Sava puts the carried half on Nemi's cloth and walks.

The gesture protects one refusal: nobody may say Sava accepted a substitute. It leaves water, warning, and departure tangled behind.

Olan must answer an open claim. Nemi must record a traveller who left before the account could be separated. Sava reaches the first doubtful mark alone and discovers that independence is not the same material as a route.

Nothing catastrophic happens. The next shelter charges for the delay, which is how small unresolved things learn to travel.
-> END
