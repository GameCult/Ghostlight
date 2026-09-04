// ghostlight.artifact_id: numen_first_answer_branch_fold_v0
// ghostlight.fixture_id: numen-first-answer-v0
// ghostlight.scene_id: numen-first-answer-v0.fourth-drain-first-answer-vigil
// ghostlight.final_ink_path: examples/ink/delvehold/numen-first-answer-v0.branch-and-fold.v0.ink

VAR answer_legibility = 1
VAR witness_cohesion = 2
VAR district_reserve = 2
VAR company_pressure = 1
VAR refusal_path = 0
VAR gills_integrity = 2
VAR resident_trust = 2
VAR public_record = 1
VAR water_margin = 2
VAR command_terminator_exposed = 0

-> start

=== start ===
// ghostlight.scene: fourth_drain_establishing
The Fourth Drain Gate is a room built around a decision water has been making for centuries.

A broad stair descends from Hearthcoil Hold at the west end. At the east end, an iron grate with a narrow return slot crosses a black brine throat. Between them stands a waist-high rune table. A municipal bypass pipe runs along the south wall; near the throat, a newer Deep Company conduit bites through the north wall under a polished brass inspection plate.

Above the brine throat, pale calcite folds open and close with the water. Their thin edges resemble runes no mason cut. Drain crews call them the gate's gills.

-> morning_preparations

=== morning_preparations ===
// ghostlight.scene: ordinary_ritual_work
Dagna Rill sets three cups on the rune table before she sets down the sacred crystal. She is a dwarven journey-priest and enchanter, qualified to ask the question and very much not qualified to decide what the answer will cost the Hold.

The offering crystal has three civic seals and no handle. In Hearthcoil, this is how one identifies both holiness and municipal property.

Master Brunna Coil, the pump workshop's broad-shouldered dwarf, brings the route book and a ring of iron keys. Sennik Reedcap, a small goblin drain keeper in a patched fungus-dyed coat, arrives from the east ledge with salt on his boots and a paper parcel of hot mushroom cakes.

Assessor Vey Marr stands beneath the northern inspection plate in a clean gray coat. His Deep Company pays the drain lease and the insurer watching it. He declines a cake on the theory that grease compromises evidence.

"Then evidence has lived a narrow life," Sennik says.

-> laying_the_question

=== laying_the_question ===
// ghostlight.scene: answering_array_setup
Dagna opens the route book. The last three disturbances are months apart: the gills closing around a company draw while leaving the southern bypass wet; a lost sounding line returned in a neat coil; the old three-tap warning of Toman Valehand, a route keeper drowned eighty-three years ago, heard beneath an empty ledge.

That is enough to petition for a First Answer. Not enough to promise one.

On the table, Dagna lays the rune question in three visible branches. Exchange ends at a measured valve. Amendment ends at a blank silver socket. Refusal ends at a place prepared for the null rune. Nothing in the array touches the material core. Fine copper leads run instead to the water, the gate stones, and the living calcite folds.

The district cistern can cover one watch with the drain stopped. After that, upper Hearthcoil starts carrying buckets uphill.

-> preparation_choice

=== preparation_choice ===
// ghostlight.choice_layer: prepare_the_petition
+ [Break the civic seal and place a full reserve crystal in the exchange dish.]
    // ghostlight.action_label: transfer_object
    // ghostlight.branch_label: offer_real_reserve
    ~ answer_legibility = answer_legibility + 2
    ~ district_reserve = district_reserve - 1
    ~ water_margin = water_margin - 1
    ~ company_pressure = company_pressure + 1
    Dagna cracks the smallest civic seal. The crystal settles into the stone dish with a blue-white pulse.

    Brunna winces. That crystal could keep three terrace pumps moving through supper.

    "An offering that costs nothing is decoration," Dagna says.

    Vey writes down the cost before the light has finished climbing his spectacles.
    -> preparation_fold
+ [Ask Sennik to map the gills before anyone cleans or measures them.]
    // ghostlight.action_label: speak
    // ghostlight.branch_label: trust_resident_listener
    ~ answer_legibility = answer_legibility + 1
    ~ resident_trust = resident_trust + 2
    ~ company_pressure = company_pressure + 1
    Sennik crouches on the east ledge and holds a strip of dry fungus paper near the calcite edges. The gills breathe damp symbols across it in salt.

    "This fold closes when the north pipe drinks," he says. "This one opens when the south pipe knocks. Your instruments call both pressure."

    Vey asks when this method was calibrated.

    "Mostly during floods. They are wonderfully strict reviewers."
    -> preparation_fold
+ [Enter every lead, seal, and witness in the route book before releasing mana.]
    // ghostlight.action_label: write
    // ghostlight.branch_label: strengthen_public_record
    ~ public_record = public_record + 1
    ~ witness_cohesion = witness_cohesion + 1
    ~ answer_legibility = answer_legibility + 1
    Dagna writes the copper leads one by one. Brunna presses her workshop seal beside them. Sennik signs with the little three-pronged mark the route office still files under miscellaneous.

    Dagna leaves a blank for Vey.

    He signs only after noticing everyone notice the blank.
    -> preparation_fold
+ [Seat the physical isolation wedge beneath the null branch.]
    // ghostlight.action_label: use_object
    // ghostlight.branch_label: make_refusal_safe
    ~ refusal_path = 1
    ~ witness_cohesion = witness_cohesion + 1
    ~ company_pressure = company_pressure + 1
    Dagna kneels and drives a slate-gray wedge into the table's feed slot. If refusal takes the flow, the wedge will break the circuit before any effect-producing terminator can punish it.

    Brunna tests it with both hands. Sennik watches the gills. Vey watches his watch.

    A safe no is slower than a useful yes. That is usually why it is missing.
    -> preparation_fold

=== preparation_fold ===
// ghostlight.fold: petition_preparation
They eat mushroom cakes while the stopped drain counts time in slow drops.

{district_reserve <= 1: The opened reserve cage shows one dark socket. Somewhere uphill, a pump clerk is already revising the evening schedule.}
{resident_trust >= 4: Sennik marks three gill folds on the floor in salt, and Dagna moves her copper leads to match them.}
{public_record >= 2: Four signatures sit beneath the exposed array. Even Vey's is legible.}
{refusal_path == 1: The isolation wedge makes the null branch physically complete: refusal can end the spell instead of merely failing it.}
{company_pressure >= 3: Vey has stopped pretending his watch is a devotional object.}

Brunna recites the names of drain workers killed or lost on this route. Dagna speaks the three terms aloud. Sennik repeats them toward the water in the clipped tunnel cant used when echoes are doing useful work.

Then Dagna releases one measured thread of mana.

-> first_pulse

=== first_pulse ===
// ghostlight.scene: pivotal_first_answer
The three branches light.

Exchange. Amendment. Refusal.

None of them takes the flow.

-> fourth_branch

=== fourth_branch ===
// ghostlight.scene: living_rune_counteroffer
Above the eastern grate, a calcite fold splits along a wet seam. Its edge grows across Dagna's blank silver socket and down the table in a fourth line, slow as frost and deliberate as handwriting.

The north conduit groans shut.

The southern municipal bypass opens one finger's width.

From the brine throat comes three knocks. Then an old brass sounding weight rolls through the grate and stops against the route book. Toman Valehand's seal is green beneath the salt.

Brunna forgets to breathe. Sennik does not. He has the expression of someone whose neighbour has finally spoken loudly enough for the landlord to hear.

-> hidden_terminator

=== hidden_terminator ===
// ghostlight.scene: command_terminator_ignition
The polished inspection plate on the north wall flashes white.

A hidden effect terminator wakes beneath it and drags at the whole array. The company conduit tries to wrench itself open. Blue fire crawls backward through the new calcite line. One of the gate's gills blackens at the edge.

"Emergency continuity provision," Vey says.

"You brought a command into a question," Dagna says.

Water hammers the southern pipe. The district still needs it. The living rune is burning now.

-> intervention_choice

=== intervention_choice ===
// ghostlight.choice_layer: protect_answer_or_flow
+ [Draw null across the entire table and let the district drain stop.]
    // ghostlight.action_label: cast
    // ghostlight.branch_label: null_the_forced_working
    ~ answer_legibility = answer_legibility + 1
    ~ gills_integrity = gills_integrity + 1
    ~ water_margin = water_margin - 1
    ~ company_pressure = company_pressure + 2
    ~ command_terminator_exposed = 1
    ~ public_record = public_record + 1
    Dagna draws right to left through the active pattern. The null rune closes under her hand.

    Light falls out of the table. The southern pipe goes quiet. So does the hidden plate, its company seal split by heat for every witness to see.

    The gill edge stays pale around its wound.
    -> witness_fold
+ [Hand Sennik the insulated hook and hold the blank branch open while he pries the hidden lead free.]
    // ghostlight.action_label: transfer_object
    // ghostlight.branch_label: trust_the_resident_cut
    ~ answer_legibility = answer_legibility + 1
    ~ gills_integrity = gills_integrity + 1
    ~ resident_trust = resident_trust + 1
    ~ witness_cohesion = witness_cohesion + 1
    ~ company_pressure = company_pressure + 1
    ~ command_terminator_exposed = 1
    Dagna braces both palms around the blank silver socket and passes Sennik the hook.

    He slips along the east ledge, ducks beneath the hot copper lead, and reaches the plate from below. The hook bites. Vey lunges once; Brunna's key ring lands against his wrist with the authority of several pounds of municipal iron.

    The hidden lead comes free trailing blue sparks. The fourth branch remains lit.
    -> witness_fold
+ [Reroute the forced draw into the southern bypass and keep Hearthcoil's water moving.]
    // ghostlight.action_label: redirect_magic
    // ghostlight.branch_label: save_the_water_margin
    ~ water_margin = water_margin + 1
    ~ gills_integrity = gills_integrity - 1
    ~ answer_legibility = answer_legibility - 1
    ~ resident_trust = resident_trust - 1
    Dagna turns the exchange valve hard south. The municipal pipe takes the stolen draw and booms with sudden flow.

    Cheers sound faintly in the shaft above. Nobody up there can see the black line spreading across the nearest gill.

    Sennik steps away from Dagna as if distance were a kind of testimony.
    -> witness_fold
+ [Make Brunna read the company seal aloud before she breaks the inspection plate.]
    // ghostlight.action_label: speak
    // ghostlight.branch_label: make_the_breach_public
    ~ public_record = public_record + 2
    ~ witness_cohesion = witness_cohesion + 1
    ~ command_terminator_exposed = 1
    ~ company_pressure = company_pressure + 1
    ~ water_margin = water_margin - 1
    ~ gills_integrity = gills_integrity - 1
    Brunna crosses the three paces from the table and reads the serial, owner, and insurer from the plate while blue fire eats another hairline into the calcite.

    Then she puts a square drain key through the polished brass and leans.

    The plate tears loose. Behind it, the hidden terminator is small, standardized, and still trying to make obedience look like physics.
    -> witness_fold

=== witness_fold ===
// ghostlight.fold: answer_under_witness
When the light settles, the fourth line still points from the brine throat toward the southern bypass and turns its calcite edge away from the company conduit.

{answer_legibility >= 4: The counteroffer is painfully clear: water for Hearthcoil, closure for the deep draw, and the returned weight as proof of remembered parties.}
{answer_legibility <= 2: Heat and hurried rerouting have blurred the line. The distinction between counteroffer and damaged reflex will feed hearings for years.}
{gills_integrity >= 3: The pale folds open and close around one dark wound, their rune-shaped edges still carrying the fourth branch.}
{gills_integrity <= 1: Two folds hang black and rigid above the grate. Whatever spoke has fewer ways to do it now.}
{resident_trust >= 4: Sennik kneels near the salt marks, close enough to listen and far enough not to crowd the gills.}
{resident_trust <= 1: Sennik stands by the west stair. He is still a witness. He is no longer helping Dagna interpret what he heard.}
{public_record >= 3: The route book holds seals, the exposed company device, and an unbroken chain of names.}
{company_pressure >= 4: Vey warns that every idle minute voids coverage, defaults the drain lease, and makes the district personally acquainted with buckets.}
{water_margin <= 1: From the stair comes the first clatter of empty cistern gauges.}
{water_margin >= 3: The southern pipe runs strongly enough to buy Hearthcoil a little time.}
{refusal_path == 1: The null wedge remains seated beneath the fourth line. Whatever the witnesses decide, the offered no was real.}
{refusal_path == 0: The empty null slot is now the most conspicuous object in the room.}

The First Answer does not make a core a person. It marks the moment people lose the excuse of believing nobody answered.

Dagna must decide what she will enter in the route book before Brunna's keys, Vey's contract, or the thirsty district decides for her.

-> final_choice

=== final_choice ===
// ghostlight.choice_layer: record_and_bargain
+ [Record an Answer and accept the fourth branch: municipal flow stays; the company draw closes.]
    // ghostlight.action_label: write
    // ghostlight.branch_label: recognize_the_answer
    {answer_legibility >= 3 && gills_integrity >= 2 && refusal_path == 1:
        -> ending_recognition
    - else:
        -> ending_contested_recognition
    }
+ {district_reserve >= 1} [Offer the remaining reserve crystal for a three-day stay while Hearthcoil gathers a wider moot.]
    // ghostlight.action_label: bargain
    // ghostlight.branch_label: buy_three_days
    ~ district_reserve = district_reserve - 1
    {witness_cohesion >= 3 && gills_integrity >= 2:
        -> ending_stay_accepted
    - else:
        -> ending_stay_uncertain
    }
+ [Seal the gate and record the vigil as inconclusive, preserving the site for another petition.]
    // ghostlight.action_label: use_object
    // ghostlight.branch_label: defer_recognition
    -> ending_deferral
+ [Use the exposed command route to drive the gate open for Hearthcoil now.]
    // ghostlight.action_label: redirect_magic
    // ghostlight.branch_label: force_municipal_flow
    {command_terminator_exposed == 1 && public_record >= 3:
        -> ending_force_exposed
    - else:
        -> ending_force_scar
    }

=== ending_recognition ===
// ghostlight.ending_label: first_answer_recorded
// ghostlight.training_hook: recognition_preserves_refusal_and_cost
Dagna writes: Answer received at Fourth Drain. Party not yet named. Terms: Hearthcoil water admitted; northern draw refused.

Brunna closes the Deep Company valve with her own key. Vey begins listing defaults, exclusions, and the many precise ways a contract can be offended.

The southern bypass opens another finger's width.

The route book has no line for gratitude. Good. Gratitude is not the bargain. Hearthcoil has water for the night, a company claim under seal, and a neighbour old enough to have returned a dead worker's weight.

Sennik puts the fourth mushroom cake beside the gills.

This time, nobody calls it an offering until the water takes it.
-> END

=== ending_contested_recognition ===
// ghostlight.ending_label: first_answer_contested
// ghostlight.training_hook: recognition_without_clean_conditions
Dagna records an Answer anyway.

The null path was incomplete, or the living rune too damaged, or the counteroffer too blurred for clean witness. The entry will be appealed before its ink dries. Brunna closes the company valve and opens the municipal line under protest from both the insurer and half her own workshop.

{resident_trust >= 3: Sennik signs beneath Dagna and adds: Heard before asked.}
{resident_trust < 3: Sennik does not sign. The empty line beneath Dagna's name weighs more than Vey's entire report.}

Perhaps a person spoke. Perhaps the vigil injured a wounded organ and called the flinch a sentence. Hearthcoil must now live inside that uncertainty with less water and no honest way back to innocence.
-> END

=== ending_stay_accepted ===
// ghostlight.ending_label: three_day_stay_accepted
// ghostlight.training_hook: bounded_counteroffer_before_civic_settlement
Dagna places the remaining reserve crystal in the blank branch and asks for three days: municipal flow at a rationed trickle, no company draw, no cutting, a wider moot at the end.

The fourth calcite line bends around the crystal.

The southern bypass opens to the width of two fingers. The north conduit stays dark.

It is not consent to mine. It is not recognition settled. It is three days purchased with winter reserve and a promise made in front of people who disagree about nearly everything except having heard one.

Brunna sends a runner uphill to ration water and gather seals. Vey sends another to gather lawyers. Sennik remains by the gills, counting the new rhythm.
-> END

=== ending_stay_uncertain ===
// ghostlight.ending_label: three_day_stay_uncertain
// ghostlight.training_hook: bargain_attempt_after_damaged_witness
Dagna offers the reserve and asks for three days.

The crystal dims into the wet calcite. The southern pipe coughs once. No fourth line brightens.

They have spent the last reserve without learning whether it was accepted, taken, or needed by an injured body. Brunna orders bucket crews. Vey calls the silence evidence. Sennik calls it silence and refuses to improve it for him.

The gate is sealed until a wider moot arrives. Hearthcoil gets no revelation grander than carrying its own water while it decides what kind of neighbour it has been.
-> END

=== ending_deferral ===
// ghostlight.ending_label: site_sealed_answer_deferred
// ghostlight.training_hook: uncertainty_preserved_at_public_cost
Dagna closes the route book without entering personhood or instability.

Brunna seats the gate wedges. The company conduit and municipal bypass both go dark. The returned sounding weight is wrapped, sealed, and left on the rune table where every later witness must walk around it.

{public_record >= 3: The hidden terminator enters the sealed evidence packet beside the route book. Vey cannot make it disappear without breaking four marks.}
{public_record < 3: Vey keeps his inspection plate. Dagna keeps a sketch and the unpleasant knowledge that sketches lose arguments to polished brass.}

The gills keep moving. Hearthcoil's pumps do not.

Sometimes restraint feels exactly like failure from the bucket line.
-> END

=== ending_force_exposed ===
// ghostlight.ending_label: forced_flow_with_public_breach
// ghostlight.training_hook: service_continuity_at_theological_and_legal_cost
Dagna takes the exposed company lead and turns its command south.

Water slams into the municipal bypass. Upstairs, dead gauges wake. On the rune table, the fourth line fractures under the forced draw.

The route book preserves what happened: the hidden terminator, the company seal, Dagna's hand on the command, the blackening gills. Vey cannot call it a natural failure. Dagna cannot call it a bargain.

Hearthcoil drinks tonight. Tomorrow its moot will decide whether necessity excuses using a person's wounded organ as a pump. The person, if that is who answered, has already received Dagna's position in a language stone remembers.
-> END

=== ending_force_scar ===
// ghostlight.ending_label: forced_flow_unwitnessed_scar
// ghostlight.training_hook: continuity_erases_the_answer_surface
Dagna drives the gate open.

The southern pipe roars. The inspection plate glows clean. The last pale edges of the fourth branch turn black and curl away from the table.

Vey records emergency stabilization. Brunna records restored service. Sennik leaves by the west stair without taking his paper of cakes.

By supper, Hearthcoil has water and no surviving instrument on which the answer can be repeated. The route book contains a sounding weight, three old disturbances, and Dagna's unsigned line.

Far behind the grate, something knocks once.

Or stone settles. The cheaper account wins first.
-> END
