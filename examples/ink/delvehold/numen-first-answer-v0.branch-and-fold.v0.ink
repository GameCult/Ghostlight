// ghostlight.artifact_id: numen_first_answer_v0_branch_fold_v0
// ghostlight.fixture_id: numen-first-answer-v0
// ghostlight.scene_id: numen-first-answer-v0.hallowshaft-final-hearing
// ghostlight.final_ink_path: examples/ink/delvehold/numen-first-answer-v0.branch-and-fold.v0.ink

VAR ritual_integrity = 2
VAR answer_evidence = 0
VAR core_trust = 2
VAR district_reserve = 2
VAR master_support = 2
VAR witness_independence = 1
VAR reset_custody = 2
VAR route_openness = 1
VAR crown_response = 0

-> start

=== start ===
// ghostlight.scene: hallowshaft_establishing
Hallowshaft Farm begins the morning by feeding stone its breakfast.

The farm's oldest cultivation vault is a bowl-shaped cavern under one of the Greathold's winter districts. A straight service gallery crosses its south lip. Below the brass rail, a grated stair descends to three offering plinths on the chamber floor. The west wall holds a meal hopper and a barred crystal-harvest sluice. The east wall holds an iron gate to a side passage where a party can enter and the core can answer intrusion on its own ground.

On the north wall, pale mineral folds spread like a crown of open pages. Blue mana moves through channels that resemble carved runes until one remembers that nobody carved them. The folds heal after sampling, bud after feeding, and sometimes change the order in which the light travels.

Rune College calls the growth conductive morphology. The mine shrine calls it the Listening Crown. Hallowshaft's tenders call it the wall, because a thing can be wondrous and still need brushing before first bell.

-> morning_people

=== morning_people ===
// ghostlight.scene: hallowshaft_morning_tending
Tavra Nineknocks brushes crystal flour out of the harvest teeth while Pip Underrail tips fungal meal and iron shavings into the west hopper. Tavra is a dwarven journeyworker and Hallowshaft's familiar tender. Pip is a goblin route keeper who has survived enough core moods to distrust any revelation that arrives before sweeping.

"Breakfast first," Pip says. "Personhood after. Nobody answers theology on an empty chamber."

Master Borren Keld waits at the upper gallery's reset cabinet with the farm seal on his belt and the memory-reset key still in his hand. Hallowshaft's crystals warm district kitchens, preserve food, and keep rail switches loose in the frost. Borren believes the hearing should be honest. He also knows exactly how many hours of honesty remain in the reserve bin.

Sister Maelin Ash carries the mine shrine's shallow black iron return bowl with yesterday's required uncut shard inside. The reset station has its own lidded iron key coffer. Beside her, Eiravel Senn, an elven spirit-law witness, has brought no incense and no patience for being called decorative.

This is the final quiet interval of a First Answer hearing. Before anyone asks whether the old core is speaking, the chamber must be prepared in a way that lets it refuse.

-> preparation_choice

=== preparation_choice ===
// ghostlight.choice_layer: ordinary_hearing_preparation
+ [Set the first uncut crystal in Maelin's return bowl before Borren tallies it.]
    // ghostlight.action_label: transfer_object
    // ghostlight.branch_label: prime_returned_gift
    // ghostlight.intent: make_the_question_cost_the_keepers_something_real
    ~ core_trust = core_trust + 2
    ~ ritual_integrity = ritual_integrity + 1
    ~ district_reserve = district_reserve - 1
    Tavra lifts the morning's first crystal from the sluice. It is blue-white, warm at the root, and worth enough heat to make Borren calculate in silence.

    She sets it uncut in the black iron bowl.

    Maelin touches two soot-dark fingers to its surface. "A gift returned is not payment yet. It is proof we can stop counting long enough to ask."

    Pip checks the district gauge. "The kitchens will be moved by our restraint. Mostly toward shouting."
    -> preparation_fold
+ [Ask Borren to lock the reset key in the shrine's key coffer under his seal and Eiravel's cord.]
    // ghostlight.action_label: transfer_object
    // ghostlight.branch_label: divide_reset_custody
    // ghostlight.intent: prevent_one_owner_from_erasing_an_unwelcome_answer
    ~ reset_custody = reset_custody + 2
    ~ ritual_integrity = ritual_integrity + 1
    ~ master_support = master_support - 1
    Borren turns the long iron key once in his palm.

    "My seal owns the farm's fault," he says.

    "Your seal can own half a lock," Tavra says. "That is the point."

    Maelin sets the lidded key coffer on the reset shelf. Eiravel threads green witness-cord loosely through its second eye, ready to tighten when Borren closes the farm seal.

    Borren's expression becomes a small private winter.
    -> preparation_fold
+ [Have Pip chalk the living side cracks onto Eiravel's witness plan.]
    // ghostlight.action_label: use_object
    // ghostlight.branch_label: prime_independent_witness
    // ghostlight.intent: record_where_the_core_changes_outside_the_prepared_signs
    ~ witness_independence = witness_independence + 2
    ~ answer_evidence = answer_evidence + 1
    ~ route_openness = route_openness + 1
    Pip kneels at the brass rail and points with the brush handle. One wet crack runs from the Crown toward the east gate. Another disappears under the harvest sluice. A third reaches beneath the gallery where property plans insist the farm ends.

    Eiravel copies all three onto waxed cloth.

    "Which one is Hallowshaft?" Tavra asks.

    Pip bares small square teeth. "Excellent. We have reached the part where the map apologizes."
    -> preparation_fold
+ [Finish the contracted crystal bin before closing the harvest runes.]
    // ghostlight.action_label: use_object
    // ghostlight.branch_label: prime_winter_reserve
    // ghostlight.intent: protect_immediate_heat_and_food_storage_before_the_hearing
    ~ district_reserve = district_reserve + 2
    ~ master_support = master_support + 1
    ~ ritual_integrity = ritual_integrity - 1
    Tavra and Pip work the sluice together. She lifts the grate. Pip rakes loose crystal into the iron bin while the Crown's blue channels pulse against the north wall.

    Borren marks the contract line complete.

    Maelin waits with the null slate. Eiravel watches the pale folds follow Tavra's rake from left to right.

    "It knows the harvest rhythm," Borren says.

    "So does Pip," Tavra says.

    "And yet nobody has offered me a civic seal," Pip says. "A useful standard."
    -> preparation_fold

=== preparation_fold ===
// ghostlight.fold: routine_preparation_before_question
Pip closes the meal hopper. Tavra lowers the harvest grate. Maelin places the return bowl on the middle plinth below. Eiravel's witness plan lies open at the upper lectern. Borren stands between the reserve gauge and the reset cabinet.

{core_trust >= 4: This morning's warm crystal rests beside yesterday's smaller shard in black iron where the Crown can reach both through the chamber's mana flow.}
{core_trust < 4: Yesterday's small uncut shard is the only returned crystal in the bowl.}
{district_reserve >= 4: The contracted bin is full enough to buy the hearing several hours before the district starts choosing which services matter most.}
{district_reserve <= 1: The reserve gauge touches its amber lower band. Somewhere above, a kitchen engine will soon be asked to preserve food with principle.}
Borren lays the reset key in the lidded coffer and closes his farm seal through its first eye.
{reset_custody >= 4: Eiravel draws the green witness cord taut through the second eye and knots it outside Borren's reach. No one hand can take the key quietly.}
{reset_custody <= 2: The coffer's second eye hangs empty. The key has left Borren's hand, but his seal remains its only lock.}
{witness_independence >= 3: Eiravel's plan records living cracks the farm plan omits, including one that crosses under the property line.}
{ritual_integrity <= 1: The chamber has been made quiet, but only after the contracted bin was filled. Even reverence can arrive on management's schedule.}

Maelin draws the null rune right to left. The harvest channels go dark. Pip opens the east gate one handspan: enough for air, scent, and a route the core can widen or close.

-> first_question

=== first_question ===
// ghostlight.scene: hallowshaft_listening_crown
Tavra, Eiravel, and Maelin descend together. Tavra's boots stop at the yellow mineral line around the three plinths.

Earlier in the hearing, Maelin asked from the west plinth while Tavra carried iron and Eiravel carried the returned shard. The Crown sent light toward iron.

Today the witnesses have changed places. The tokens have changed hands. A reflex may follow the object. An answer must preserve a distinction when the prepared signs move.

Tavra lays a delver's worn iron piton on the east plinth. It means a bounded route, entered by living bodies who can retreat. Eiravel lays a sealed season-stone on the west plinth. It means closure until thaw. Maelin sets the returned crystal in the middle, offering continued exchange under new terms.

Eiravel and Maelin climb back to the gallery. Tavra remains alone below.

"Hallowshaft," Maelin says, using the place-name without pretending it settles who hears. "Which taking can you answer?"

-> crown_answer

=== crown_answer ===
// ghostlight.scene: hallowshaft_first_answer
The Listening Crown goes dark.

One blue line wakes at its highest fold. It runs down through channels no chisel opened, crosses the crack beneath Eiravel's witness plan, and enters the chamber floor.

The west season-stone remains cold. The returned crystal sings once in its iron bowl. The piton on the east plinth turns until its worn point faces the side-passage gate.

Then a fourth channel buds from the Crown.

It grows toward Tavra's boots and stops at the yellow line.

-> answer_choice

=== answer_choice ===
// ghostlight.choice_layer: answer_under_uncertainty
+ [Give the private nine-knock tending rhythm and wait for the Crown to choose whether to repeat it.]
    // ghostlight.action_label: use_object
    // ghostlight.branch_label: answer_with_familiar_keeper
    // ghostlight.intent: test_relationship_and_memory_through_a_pattern_the_core_knows
    ~ core_trust = core_trust + 2
    ~ crown_response = crown_response + 2
    ~ witness_independence = witness_independence - 1
    Tavra takes the little iron listening hammer from her belt.

    Three knocks for feed. Two for an open grate. Four for hands clear of the wall. The rhythm is older than the hearing and small enough to be hers.

    The Crown answers with nine pale pulses.

    Eiravel writes nothing for a breath. "It may be answering you."

    "That was the hope," Tavra says.

    "It is also the problem," Borren says from the gallery.
    -> answer_fold
+ [Exchange the piton and season-stone again, then have Eiravel repeat Maelin's question.]
    // ghostlight.action_label: mixed
    // ghostlight.branch_label: answer_through_independent_witness
    // ghostlight.intent: test_whether_meaning_survives_new_positions_and_a_new_speaker
    ~ answer_evidence = answer_evidence + 2
    ~ witness_independence = witness_independence + 1
    ~ core_trust = core_trust - 1
    Eiravel descends while Tavra moves the piton west and the season-stone east. Maelin repeats no cue. Pip keeps both hands visible above the rail.

    Eiravel asks the question in dwarven first, then in an elven ritual register whose words Tavra does not know.

    The fourth channel withdraws from Tavra. Light finds the moved piton anyway. The east gate knocks once from the far side.

    "Meaning transferred," Eiravel says.

    Pip squints at the gate. "Or something over there objects to your accent. Record both."
    -> answer_fold
+ [Crank the east gate wide enough for a delver to pass and make the offered route real.]
    // ghostlight.action_label: use_object
    // ghostlight.branch_label: answer_with_real_route
    // ghostlight.intent: give_the_core_a_live_path_to_accept_divert_or_make_dangerous
    ~ route_openness = route_openness + 2
    ~ answer_evidence = answer_evidence + 1
    ~ district_reserve = district_reserve - 1
    ~ master_support = master_support - 1
    Tavra climbs back to the gallery and leans into the east-gate winch. Pip catches the brake pawl while the iron gate rises from one handspan to shoulder height.

    Cold fungal air enters from the side passage. Something many-legged retreats beyond lamp reach. The blue line in the floor turns toward the opening and grows brighter.

    Borren puts a hand on his seal. "A route is not a metaphor once it can eat a party."

    "That," Maelin says, "is why the old rite requires one."
    -> answer_fold
+ [Keep every hand still until the new channel finishes growing.]
    // ghostlight.action_label: wait
    // ghostlight.branch_label: answer_without_forcing
    // ghostlight.intent: preserve_the_core's_unprompted_action_even_as_the_reserve_clock_runs
    ~ ritual_integrity = ritual_integrity + 2
    ~ crown_response = crown_response + 1
    ~ district_reserve = district_reserve - 1
    Nobody touches a token.

    The new channel reaches the yellow line, turns, and traces its edge around Tavra without crossing it. A second bud forms toward Pip's chalked crack under the gallery.

    The district gauge ticks down in the silence.

    Borren watches it. Tavra watches him watch it. The Crown continues at the speed of wet stone deciding something.
    -> answer_fold

=== answer_fold ===
// ghostlight.fold: first_answer_meets_winter_account
Tavra returns to the upper gallery.

The Crown holds light along the floor. The piton points toward the side passage. The returned crystal has stopped singing. Nobody in the chamber agrees whether the fourth channel is a greeting, a boundary, a threat, or all three wearing the same mineral face.

{crown_response >= 2: Nine pulses remain bright in the Crown's upper folds, recognizably close to Tavra's tending rhythm.}
{crown_response == 1: The unprompted boundary channel stays lit around the yellow line after every human hand goes still.}
{answer_evidence >= 3: The response survives moved tokens, a changed speaker, or both. Eiravel's rubbing and Pip's route marks give the claim more than one custodian.}
{answer_evidence <= 0: The chamber has done something astonishing only in front of the people most prepared to expect it.}
{witness_independence >= 3: Eiravel and Pip can each describe evidence the other did not stage.}
{route_openness >= 3: The east gate stands high enough for a delving party; cold air and moving fungus make the offered bargain physically real.}
{route_openness <= 1: The east gate remains a narrow breath. The offered contest is still mostly a symbol under human control.}

Above Borren's shoulder, the reserve gauge drops into red.

The little hammer in the district pipe begins its shortage call: one strike for kitchens, one for cold stores, one for rail switches. The same three services, repeated until someone adds crystal or makes a choice.

Borren looks at the sealed key coffer. "The rite has had its answer. The district would now like ours."

-> custody_choice

=== custody_choice ===
// ghostlight.choice_layer: answer_and_material_cost
+ {reset_custody >= 4} [Take Eiravel's cord end while Borren takes the farm-seal end, and keep the reset key divided.]
    // ghostlight.action_label: transfer_object
    // ghostlight.branch_label: hold_reset_in_common
    // ghostlight.intent: prevent_urgent_service_pressure_from_restoring_one_hand_erasure
    ~ reset_custody = reset_custody + 1
    ~ witness_independence = witness_independence + 1
    ~ master_support = master_support - 1
    Tavra takes the green cord below Eiravel's knot. Borren holds the iron lid by its sealed ring.

    Either can stop the other from opening the coffer. Neither can solve the red gauge alone.

    "This is an extremely inefficient way to hold a key," Borren says.

    "It is a very efficient way to hold two people," Pip says.
    -> final_state
+ {answer_evidence >= 2} [Press the Crown's channel pattern into Eiravel's wax cloth and give the copy to Pip for the public route book.]
    // ghostlight.action_label: transfer_object
    // ghostlight.branch_label: widen_the_witness
    // ghostlight.intent: keep_the_answer_from_becoming_private_shrine_or_college_property
    ~ answer_evidence = answer_evidence + 1
    ~ witness_independence = witness_independence + 2
    ~ master_support = master_support - 1
    Tavra lays fresh waxed cloth over the bright edge of the floor channel and rubs charcoal across it. Eiravel signs the position. Maelin marks the question. Pip takes the copy without being asked to surrender the original map.

    "Public route book?" Borren asks.

    "Public route book," Pip says. "People become much harder to misfile when they acquire commuters."
    -> final_state
+ {route_openness >= 3} [Leave the east route open and ask Pip to mark the first negotiated delve.]
    // ghostlight.action_label: use_object
    // ghostlight.branch_label: preserve_core_contest
    // ghostlight.intent: convert_the_symbolic_offer_into_a_bounded_route_the_core_can_answer
    ~ route_openness = route_openness + 1
    ~ core_trust = core_trust + 1
    ~ district_reserve = district_reserve - 1
    Pip hangs a blank route tag on the gate chain. No quota. No promised crystal. Only entry, retreat, and the warning that the passage chose to open during a hearing.

    Something knocks once beyond the bend.

    "That is not acceptance," Eiravel says.

    "No," Tavra says. "It is where acceptance would have to happen."
    -> final_state
+ [Open one bounded harvest channel under Borren's seal and send crystal to the district.]
    // ghostlight.action_label: use_object
    // ghostlight.branch_label: answer_winter_need
    // ghostlight.intent: protect_immediate_services_without_restoring_the_full_farm_cycle
    ~ district_reserve = district_reserve + 2
    ~ master_support = master_support + 2
    ~ route_openness = route_openness - 1
    ~ core_trust = core_trust - 1
    Borren stamps a one-channel exception. Tavra opens the smallest harvest rune. The west sluice takes mana from the Crown's outermost fold and precipitates three narrow crystals into the district bin.

    The shortage hammer stops after kitchens and cold stores. Rail switches remain on the list.

    The fourth channel at Tavra's boots dims but does not vanish.
    -> final_state

=== final_state ===
// ghostlight.scene: hallowshaft_threshold
The hearing ends with the chamber still capable of contradicting everyone in it.

{ritual_integrity >= 4: The null slate, unmoved boundary line, and patient record show a question that allowed silence and cost the keepers time.}
{ritual_integrity <= 1: A full bin and a hurried quiet shift make the rite look uncomfortably like extraction wearing shrine clothes.}

{district_reserve >= 4: Enough crystal has reached the upper bin to protect kitchens and cold stores through the next allocation bell.}
{district_reserve <= 0: The reserve gauge is empty. The shortage hammer begins its fourth round, and theology acquires neighbours with cold hands.}

{master_support >= 4: Borren stands beside the hearing record with his farm seal visible, risking the workshop's name on a bounded next step.}
{master_support <= 0: Borren has moved to the reset cabinet. He has not opened it, but his body has chosen a side of the gallery.}

{reset_custody >= 4: The sealed key coffer is held between separate hands and separate authorities; the reset key cannot disappear into urgency.}
{reset_custody <= 2: The reset key has left Borren's hand but remains under his seal alone, one frightened decision away from making the chamber simpler.}

{core_trust >= 4: Pale light gathers toward Tavra, the returned crystal, and the open route instead of the harvest sluice.}
{core_trust <= 1: The Crown shutters its outer folds around the harvest channel.}

{answer_evidence >= 3: The wax rubbing, moved signs, and living boundary marks can travel beyond the people who wanted this answer.}
{witness_independence <= 1: The strongest response belongs to Tavra's private rhythm, powerful as relationship and weak as public proof.}
{route_openness >= 3: The east passage remains a real contested route, cold and dangerous, with a blank tag waiting for terms.}
{crown_response >= 2: Nine lights wait in the Crown like a remembered knock.}

Maelin asks what should enter the route book. Eiravel asks who receives a copy. Pip asks who will feed the wall tomorrow. Borren asks which district service should go dark if the answer requires another quiet day.

Tavra can answer only for the next act.

-> final_choice

=== final_choice ===
// ghostlight.choice_layer: meaning_of_the_first_answer
+ [Address Hallowshaft as a bargaining person and write the first proposed terms beneath the place-name.]
    // ghostlight.action_label: mixed
    // ghostlight.branch_label: recognize_bargaining_person
    // ghostlight.intent: treat_the_answer_as_the_start_of_negotiation_without_declaring_godhood_or_innocence
    {core_trust >= 3 && ritual_integrity >= 3 && crown_response >= 1 && reset_custody >= 4:
        -> ending_recognition
    - else:
        -> ending_recognition_cost
    }
+ [Carry the distributed evidence to the district moot before naming who spoke.]
    // ghostlight.action_label: move
    // ghostlight.branch_label: seek_public_hearing
    // ghostlight.intent: let_people_bearing_the_heat_cost_examine_the_answer_and_its_uncertain_boundary
    {answer_evidence >= 3 && witness_independence >= 3:
        -> ending_moot
    - else:
        -> ending_moot_cost
    }
+ [Offer one warm district and one open delving route as the first bounded exchange.]
    // ghostlight.action_label: mixed
    // ghostlight.branch_label: bargain_through_service_and_delve
    // ghostlight.intent: keep_people_warm_while_restoring_the_core's_power_to_raise_a_price
    {district_reserve >= 3 && master_support >= 2 && route_openness >= 2 && reset_custody >= 4:
        -> ending_exchange
    - else:
        -> ending_exchange_cost
    }
+ [Record no verdict, seal the chamber for another quiet shift, and keep the reset key divided.]
    // ghostlight.action_label: silence
    // ghostlight.branch_label: honor_unresolved_silence
    // ghostlight.intent: refuse_both_forced_personhood_and_forced_instability_while_preserving_the_evidence
    {reset_custody >= 4 && ritual_integrity >= 3:
        -> ending_silence
    - else:
        -> ending_silence_cost
    }

=== ending_recognition ===
// ghostlight.ending_label: bargaining_person_recognized
// ghostlight.training_hook: personhood_begins_obligation_not_certainty
Tavra writes Hallowshaft in the route book where the farm number used to stand alone.

Below it she writes no title. Not god. Not citizen. Not harmless.

She records the offered terms: one bounded delving route, one returned share from each harvest, no memory reset while the hearing remains under moot protection, and another question after thaw.

The Listening Crown sends nine lights down to the yellow line. On the ninth, the east gate opens another finger's width by itself.

Maelin bows. Eiravel does not; she is watching the crack that crosses the property line. Borren marks the winter shortfall beside the bargain. Pip schedules breakfast.

The old core has become a person to the people above in the least glamorous practical sense: its needs can no longer be mistaken for scenery.
-> END

=== ending_recognition_cost ===
// ghostlight.ending_label: bargaining_person_claim_exceeds_evidence
// ghostlight.training_hook: recognition_without_a_supported_relationship
Tavra writes the place-name as a person.

{crown_response < 1: The Crown gives her no ninth light.}
{core_trust < 3: The iron return bowl offers no corroborating light.}
{ritual_integrity < 3: The rite has a gap where patience or an unforced answer should be.}

Maelin closes the route book before the ink dries. "A name can invite an answer. It cannot counterfeit one."

Borren does not reach for the reset key. Eiravel copies the failed claim along with the chamber response. Pip leaves the breakfast measure full.

They have not disproved a person. They have proved that wanting one is not enough.
-> END

=== ending_moot ===
// ghostlight.ending_label: distributed_public_hearing
// ghostlight.training_hook: uncertain_personhood_enters_local_jurisdiction
The evidence leaves Hallowshaft in three hands.

Eiravel carries the wax rubbing. Pip carries the route-book copy with the living cracks. Tavra carries the black return bowl and its untallied crystal so the district can see what the hearing cost before anyone turns sacrifice into a decorative noun.

The moot gathers where the shortage is already visible: kitchen keepers, rail crews, cold-store workers, delvers, shrine hands, and seal-bearing workshops under one stone roof.

Nobody votes on whether spirits exist. They argue over the reset key, tomorrow's heat, the east route, and who must return to ask the next question.

Hallowshaft has not entered the moot. Its answer has entered the jurisdiction of those paying to hear it.
-> END

=== ending_moot_cost ===
// ghostlight.ending_label: public_hearing_with_private_proof
// ghostlight.training_hook: spectacle_outruns_evidence_custody
Tavra reaches the district with a magnificent account and too little that survived its telling.

{witness_independence < 3: The strongest sign belongs to a familiar tender or a witness chain too narrow to travel cleanly.}
{answer_evidence < 3: The moved tokens, fourth channel, and living cracks do not survive as one inspectable record.}
Borren's opponents call it shrine theater. His allies call it an elven closure bid. Both descriptions fit comfortably around an empty reserve gauge.

The moot orders another hearing and temporary rationing. The reset key remains disputed.

Hallowshaft becomes famous one shift before it becomes legible, which is an expensive order for those events.
-> END

=== ending_exchange ===
// ghostlight.ending_label: bounded_winter_exchange
// ghostlight.training_hook: delving_bargain_meets_infrastructure_need
Borren seals one harvest channel. Tavra marks the quantity. Pip hangs a living-party contract on the east gate with retreat written larger than quota.

The district gets enough crystal for kitchens and cold stores. Rail crews take charcoal braziers to the exposed switches and complain with professional inventiveness.

Beyond the east bend, the core moves fungus across the obvious route and leaves a narrow mineral seam visible above a harder one. It can hide a reserve, raise danger, or make the offered taking foolish. The party can turn back.

The exchange is not peace. It is the return of prices both sides can alter.
-> END

=== ending_exchange_cost ===
// ghostlight.ending_label: winter_bargain_without_two_sides
// ghostlight.training_hook: infrastructure_compromise_without_material_boundary
Tavra offers heat and a contested route, but one half of the bargain exists mostly in language.

{district_reserve < 3: The district bin is too low to protect the services named in the offer.}
{master_support < 2: Borren's seal cannot carry the promised boundary beyond this shift.}
{route_openness < 2: The east gate remains a narrow symbol no delver can enter.}
The farm takes three crystals through the bounded channel and calls the result exchange.

The Crown closes its outer folds around the harvest sluice.

Pip removes the blank route tag. "If only one side can change the price," they say, "that is still a bill."
-> END

=== ending_silence ===
// ghostlight.ending_label: uncertainty_preserved_under_divided_custody
// ghostlight.training_hook: refusal_to_force_personhood_or_erasure
Maelin writes no verdict. Eiravel writes the question, positions, moved signs, and every response. Tavra seats the null bar across the harvest sluice. Borren's farm seal and Eiravel's witness cord lock the reset key between authorities.

The district loses another yield shift. The east gate remains open one handspan. Pip leaves the morning feed and marks the side passage dangerous, not hostile.

In the dark after the lamps go, the Listening Crown keeps one pale channel around the yellow line.

Silence has not absolved anyone. It has survived their need to make it convenient.
-> END

=== ending_silence_cost ===
// ghostlight.ending_label: sealed_chamber_without_preserved_authority
// ghostlight.training_hook: closure_does_not_repair_split_ownership
Tavra asks for another quiet shift, but the room has no durable quiet to give her.

{reset_custody < 4: The reset key remains under one owner's seal.}
{ritual_integrity < 3: The quiet interval was shortened or shaped around a completed harvest.}
Borren closes the chamber for the night and posts guards at the sluice.

Below them, stored mana has nowhere agreed to go. The east passage breathes against a gate too narrow for contest. Pale folds thicken around the Crown.

Closure prevents one bad act. It does not become a bargain by waiting.
-> END
