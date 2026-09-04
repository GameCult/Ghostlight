// ghostlight.artifact_id: veil_cinderhook_route_song_branch_fold_v0
// ghostlight.fixture_id: veil-cinderhook-route-song-v0
// ghostlight.scene_id: veil-cinderhook-route-song-v0.cinderhook-third-pulse
// ghostlight.final_ink_path: examples/ink/delvehold/veil-cinderhook-route-song.branch-and-fold.v0.ink

VAR route_confidence = 2
VAR chorus_integrity = 2
VAR orsa_trust = 2
VAR company_pressure = 1
VAR time_margin = 2
VAR worker_warning = 0
VAR home_verse_exposure = 0
VAR record_fidelity = 0
VAR dispatch_hold = 0
VAR decoy_forecast = 0

-> start

=== start ===
Cinderhook Relay, on a freight frontier of the dwarven Greathold, begins each morning by disagreeing with the map.

The relay is a wedge of old living stone where three routes meet. The brass-railed freight line enters through the north pressure gate, crosses the hall, and bends east behind a thick buttress into Glassvein Throat. The gate seals during a line surge and opens only after a safety crew bleeds the runes. A low arch on the south wall admits the Low Echo foot culvert. Its first turn is visible. Nothing after it is.

In the centre, a bronze grate covers a listening well. Drip cups hang along the east wall. Their different metals taste the damp for mana. A dwarven dispatch desk and its red departure clapper occupy the west wall, far enough from the well that a clerk can call a song superstition without raising her voice.

-> morning_routine

=== morning_routine ===
Tikka Reed-Teeth, the relay's goblin route-singer, empties the copper cup, licks one rain-bright drop from a finger, and puts the cup back on its chalk mark. The work is usually less dramatic than its customers prefer. Mostly it involves wet sleeves, repeated verses, and waiting for a tunnel to finish being indecisive.

Orsa Feldspar sits at the dispatch desk, a square-built dwarven clerk in an indigo work coat. She copies yesterday's road refrain onto a slate timetable.

"Ink stays where I put it," Orsa says.

"A known defect," Tikka says.

Veyr Copperstamp, a Deep Company surveyor with a clipped red-brown beard and a silver measuring visor, waits beside the north rail. He has nine returning mine workers, two pallets of pump crystal, and a winter delivery bond somewhere beyond the eastern bend. Only the bond is currently on time.

-> runner_arrival

=== runner_arrival ===
Marn Nine-Turns comes out of Glassvein on soft boots, breathing hard. Chalk dust powders his patched green coat. His left sleeve is wet to the shoulder.

He does not report in sentences. He taps the iron cup twice, the stone cup once, and sings the eastern opening cadence half a tone low. Air drawing north. Seep climbing south. Pale tunnel moths moving away from the rail. The survey repeater has sounded the engine rune once; its second lamp is charging.

The answer waiting in Tikka's memory is ugly and specific: Glassvein will pinch around the third repeated pulse. People on foot may get one crossing through Low Echo. A powered wagon sent after them will teach that culvert what the machine is for.

Orsa lifts her chalk. Veyr lifts his watch.

Tikka must decide what sort of morning report this is going to become.

-> morning_choice

=== morning_choice ===
// ghostlight.choice_layer: morning_song_method
+ [Test every drip cup and the listening grate before giving the road refrain.]
    // ghostlight.action_label: use_object
    // ghostlight.branch_label: morning_test_the_route
    // ghostlight.intent: Buy confidence with inspection time.
    ~ route_confidence = route_confidence + 2
    ~ time_margin = time_margin - 1
    ~ orsa_trust = orsa_trust + 1
    Tikka carries the copper, iron, and stone cups to the listening well. Copper warms in the hand. Iron rings when set on the grate. Stone gives back a pulse so low it is felt through the wrist.

    Orsa turns her slate over and waits. Veyr checks his watch hard enough to make it somebody else's fault.

    The third repetition is not a guess. The route has begun bracing for it.
    -> morning_fold
+ [Make Marn sing the crossing again while Tikka answers from the well.]
    // ghostlight.action_label: speak
    // ghostlight.branch_label: morning_restore_the_chorus
    // ghostlight.intent: Preserve the runner's uncertainty and rebuild the chorus before outsiders compress it.
    ~ route_confidence = route_confidence + 1
    ~ chorus_integrity = chorus_integrity + 2
    ~ company_pressure = company_pressure + 1
    Marn gives the eastward line. Tikka answers from the well. He repeats the wet-sleeve verse and leaves a beat empty where Pell-of-Blue-Lamps should have answered from the second survey niche.

    Nobody fills it.

    Veyr says, "Is the omission part of the report?"

    "It is the most expensive part," Tikka says.
    -> morning_fold
+ [Teach Orsa how to mark the held beat before the timetable goes upstairs.]
    // ghostlight.action_label: show_object
    // ghostlight.branch_label: morning_teach_uncertainty
    // ghostlight.intent: Preserve uncertainty in a form a Hold office can copy.
    ~ route_confidence = route_confidence + 1
    ~ record_fidelity = record_fidelity + 2
    ~ orsa_trust = orsa_trust + 1
    ~ company_pressure = company_pressure + 1
    Tikka draws two close chalk strokes for witnessed crossings and leaves a thumb's width before the third.

    "That gap means unknown?" Orsa asks.

    "It means somebody should have come home with an answer. Unknown is cheaper."

    Orsa copies the gap. Veyr writes a zero in his own book.
    -> morning_fold
+ [Give yesterday's familiar refrain and save the argument for when the train is in sight.]
    // ghostlight.action_label: withhold_object
    // ghostlight.branch_label: morning_repeat_the_stale_song
    // ghostlight.intent: Preserve time and conceal present uncertainty at the cost of a weaker forecast.
    ~ route_confidence = route_confidence - 1
    ~ chorus_integrity = chorus_integrity - 1
    ~ time_margin = time_margin + 1
    ~ company_pressure = company_pressure - 1
    Tikka sings the neat version: road open, freight expected, south culvert unneeded.

    Orsa's chalk runs easily. Veyr's shoulders descend by half an inch.

    Marn looks at Tikka's dry sleeve and then at his own wet one. The look is quiet enough to keep the room polite.
    -> morning_fold

=== morning_fold ===
// ghostlight.fold: route_song_method_into_company_demand
Orsa writes the departure board while the north gate exhales cold air around its brass seals.

{route_confidence >= 4: The cups and well agree. Tikka can name the third pulse without borrowing courage from certainty.}
{route_confidence <= 1: Yesterday's cadence sits in the mouth like a key copied after the lock changed.}
{chorus_integrity >= 4: Marn stays beside the well. The missing runner's beat remains open between them instead of being edited away.}
{chorus_integrity <= 1: Marn moves to the south arch and keeps his answers behind his teeth.}
{record_fidelity >= 2: Orsa's timetable now contains a visible gap where ordinary survey books would print a clean zero.}
{time_margin <= 1: From beyond the eastern buttress comes the faint iron complaint of the returning engine.}
{time_margin >= 3: The train remains distant enough for everyone to pretend discussion is action.}
{company_pressure >= 2: Veyr has stopped checking his watch. He is checking who controls the clapper.}

The second survey pulse shivers through the rail.

-> company_demand

=== company_demand ===
Veyr steps from the north rail to the edge of the listening well. He keeps off the grate. This is courtesy, calculation, or an insurer's instruction.

"Glassvein closes, we divert south," he says. "Give Clerk Feldspar the full route. Nine workers come home, and Cistern Terrace gets its pump crystal before the cold turn."

The road refrain can tell him that Low Echo admits walkers and rejects the repeated engine pattern. The home verse tells how the culvert forks after its blind bend: one branch toward a public stair, another toward concealed goblin water, fungal beds, and sleeping chambers. Core-farm evictions have filled those chambers past comfort. A Company map would fill them past survival.

Orsa sets down her chalk. "I can bind a closed hearing to my seal. I cannot promise what his office will infer from my orders."

Marn puts two fingers on the rope screen across Low Echo. Behind him, the first turn hides every home Tikka has any right to endanger.

The east signal changes from green to amber.

-> disclosure_choice

=== disclosure_choice ===
// ghostlight.choice_layer: disclosure_under_second_pulse
+ [Sing only the public road refrain: people may pass Low Echo once; powered freight must remain.]
    // ghostlight.action_label: speak
    // ghostlight.branch_label: disclose_road_refrain
    // ghostlight.intent: Warn workers while withholding the home geography.
    ~ worker_warning = worker_warning + 2
    ~ record_fidelity = record_fidelity + 2
    ~ company_pressure = company_pressure + 1
    Tikka sings the southward permission plainly. One walking crossing. Living weight. No engine cadence. The final note stops at the blind bend.

    Orsa writes the condition, including the stop.

    Veyr says, "Your route appears to end in punctuation."

    "Many safe ones do."
    -> third_pulse
+ [Give Orsa the home verse in a closed, reciprocal hearing.]
    // ghostlight.action_label: disclose_secret
    // ghostlight.branch_label: entrust_home_verse
    // ghostlight.intent: Buy a coordinated rescue by exposing concealed geography to one accountable listener.
    ~ worker_warning = worker_warning + 2
    ~ home_verse_exposure = home_verse_exposure + 2
    ~ orsa_trust = orsa_trust + 2
    ~ record_fidelity = record_fidelity + 1
    ~ time_margin = time_margin - 1
    Orsa presses her civic seal into warm wax on the blank side of the slate. Tikka sings close enough that the song belongs to three bodies: singer, runner, witness.

    Past the blind bend. Over the dry shelf. Public stair at the split. Home water under the descending refrain.

    Orsa hears the difference. Veyr hears that there was more to hear.
    -> third_pulse
+ [Let Veyr overhear a stale verse that moves the closure forward to the second pulse.]
    // ghostlight.action_label: deceive
    // ghostlight.branch_label: plant_early_closure
    // ghostlight.intent: Stop the train early with route-denial misinformation while keeping the home verse concealed.
    ~ worker_warning = worker_warning + 1
    ~ decoy_forecast = decoy_forecast + 1
    ~ dispatch_hold = dispatch_hold + 1
    ~ company_pressure = company_pressure + 2
    ~ orsa_trust = orsa_trust - 1
    ~ time_margin = time_margin + 1
    Tikka turns yesterday's warning one pulse early and leaves it where Veyr's visor can catch the beat.

    Veyr strikes the red clapper himself. The departure board changes to HOLD.

    Orsa looks at Tikka. A clerk lives by noticing when a useful error arrives wearing intention.
    -> third_pulse
+ [Pull the iron dispatch pin from the clapper linkage.]
    // ghostlight.action_label: use_object
    // ghostlight.branch_label: seize_dispatch_hold
    // ghostlight.intent: Make departure physically impossible before argument consumes the remaining window.
    ~ dispatch_hold = dispatch_hold + 2
    ~ worker_warning = worker_warning + 1
    ~ company_pressure = company_pressure + 2
    ~ time_margin = time_margin + 1
    Tikka crosses the hall, ducks under the clerk rail, and pulls the iron pin from the clapper linkage.

    The red handle drops loose. The north signal fails safe with a hard white blink.

    Orsa says, "That is municipal equipment."

    Tikka closes a fist around the pin. "It has improved."

    Veyr starts naming clauses.
    -> third_pulse

=== third_pulse ===
// ghostlight.fold: disclosure_paths_into_visible_prediction
The survey repeater sounds a third time beyond the eastern bend.

The rail hums one clean industrial note. The stone answers in several pitches, none of them polite.

Glassvein Throat begins to close.

The north pressure gate answers the surge by dropping its brass seals. Until the rail runes are bled safe, Low Echo is the only way out of the relay.

-> closure_aftermath

=== closure_aftermath ===
The returning engine appears around the eastern buttress with white steam under its wheels. Stone folds behind it in slow ribs, shaping itself around the repeated rune pulse. The nine workers jump down or cling to the side steps as the driver brakes for the relay.

{dispatch_hold >= 1: The north departure light is already white. With the onward line physically held, the driver stops short of carrying the engine's pattern across the hall.}
{dispatch_hold == 0:
    {worker_warning >= 2: Orsa has the walking warning on the board. She waves the crew off the engine before Veyr can order another powered movement.}
    {worker_warning < 2: The board still promises ordinary freight. Three workers remain on the side steps until Marn runs into the rail bed and screams the south cadence at them.}
}
{decoy_forecast >= 1: Veyr's book says the closure came one pulse later than Tikka's overheard warning. He circles the discrepancy as fraud and underlines the accurate halt.}
{record_fidelity >= 3: Orsa's slate preserves the condition the stone just followed: repeated engine pattern, third pulse, one walking window.}
{record_fidelity <= 1: On the public board, the event is only GLASSVEIN FAILURE. The route's answer has already become an accident.}
{home_verse_exposure >= 2: Orsa knows where Low Echo divides. Veyr watches her eyes and learns that the south arch contains more geography than his chart.}
{company_pressure >= 4: Veyr posts himself between the crystal pallets and the north gate. Saving the cargo has become, in his posture, the last available proof that the contract still governs reality.}

The workers are in Cinderhook. The two crystal pallets are on the engine. Low Echo may accept people once. Cistern Terrace's pumps need some of that crystal before the cold turn. The hidden chambers beyond the culvert need the Company to remain ignorant.

Tikka has one more verse to choose.

-> final_choice

=== final_choice ===
// ghostlight.choice_layer: route_song_consequence
+ [Lead the workers through Low Echo with two sacks of pump crystal and stop at the public stair.]
    // ghostlight.action_label: move
    // ghostlight.branch_label: prioritize_people_and_pumps
    // ghostlight.intent: Use the song's narrow permission for people and hand-carried necessity.
    {route_confidence >= 3 && time_margin >= 2 && worker_warning >= 1:
        -> ending_crossing_success
    - else:
        -> ending_crossing_cost
    }
+ [Entrust Orsa with the full cadence and let her direct the crossing.]
    // ghostlight.action_label: transfer_knowledge
    // ghostlight.branch_label: prioritize_bounded_disclosure
    // ghostlight.intent: Test whether reciprocal duty can carry a secret farther than kinship.
    {orsa_trust >= 4 && chorus_integrity >= 3:
        -> ending_disclosure_success
    - else:
        -> ending_disclosure_cost
    }
+ [Make Orsa publish the conditional refrain beside the Company's clock-bound report.]
    // ghostlight.action_label: show_record
    // ghostlight.branch_label: prioritize_public_record
    // ghostlight.intent: Preserve how the forecast worked without publishing the home geography.
    {record_fidelity >= 3 && route_confidence >= 3:
        -> ending_record_success
    - else:
        -> ending_record_cost
    }
+ [Close the rope screen, keep everyone in the relay, and surrender the winter delivery.]
    // ghostlight.action_label: block_route
    // ghostlight.branch_label: prioritize_home_secrecy
    // ghostlight.intent: Protect the concealed settlement even if workers and pumps bear the delay.
    {dispatch_hold >= 1 && worker_warning >= 1:
        -> ending_home_success
    - else:
        -> ending_home_cost
    }

=== ending_crossing_success ===
// ghostlight.ending_label: people_and_pumps_success
// ghostlight.training_hook: conditional_forecast_guides_bounded_crossing
Tikka leads. Orsa follows with the white signal lantern. Marn takes the rear. Nine workers carry their tools; four share two sacks of crystal between them.

Low Echo admits footfalls, breath, cloth, and the soft knock of crystal carried by hand. At the blind bend, Tikka sings the public stair high and the home fork low enough to vanish under boot noise.

{home_verse_exposure >= 2: Orsa hears the buried cadence and orders every worker to keep eyes on the white chalk marks. The order protects the crossing. It also proves she knows what lies below it.}
{home_verse_exposure < 2: The workers reach the public stair knowing only that Tikka chose every turn before they saw it. The home fork remains a pressure in the dark.}

Behind them, stone closes over the engine's attempted pattern. Ahead, Cistern Terrace gets enough crystal for one cold turn, not enough for anyone to call the contract fulfilled.

The route-song was right in the least convenient way available.
-> END

=== ending_crossing_cost ===
// ghostlight.ending_label: people_and_pumps_cost
// ghostlight.training_hook: stale_or_rushed_forecast_spends_the_margin
Tikka takes the south arch with Orsa's white signal lantern and nine workers behind. Four of them share two crystal sacks. Marn keeps the rear.

The cadence reaches for observations the morning did not secure. A seep has crossed the lower shelf. The group loses time turning back from water that Marn's wet sleeve had warned about and nobody had finished singing.

They reach the public stair, but the last crystal sack must be left in the culvert when the stone tightens around its metal buckles. Nobody dies. Cistern Terrace loses another hour, and the route learns the taste of hurried necessity.

Marn does not accuse Tikka. He sings the missing wet-sleeve verse from the beginning.
-> END

=== ending_disclosure_success ===
// ghostlight.ending_label: bounded_disclosure_success
// ghostlight.training_hook: reciprocal_secret_custody
Tikka gives Orsa the full cadence. Marn answers it. The chorus has enough strength to make the conditions clear and enough trust to let a stranger carry them.

Orsa directs workers to the public stair, sends only hand-carried crystal, and posts her own civic seal across Veyr's request for the southern chart. When he demands coordinates, she gives him a duty receipt naming lives moved, freight abandoned, and no transferable route.

{home_verse_exposure >= 2: The secret now lives in one dwarven memory as well as goblin song. Nothing in the room can make that harmless.}

The home fork stays dark. The workers emerge. Orsa returns the sealed slate to Tikka instead of filing it.

The reciprocal hearing has worked once. That is evidence, not absolution.
-> END

=== ending_disclosure_cost ===
// ghostlight.ending_label: bounded_disclosure_cost
// ghostlight.training_hook: disclosure_outpaces_trust_or_chorus
Tikka gives Orsa the full cadence, but the song reaches her without a whole chorus behind it.

She directs the workers correctly. Then Veyr reads her orders backward: the halt at the dry shelf, the count at the public stair, the instruction never to descend at the split. By the time the last worker emerges, his chart has acquired a dotted southern branch.

Orsa has kept the people alive and failed to keep the inference contained. Both facts will bear her seal.

At the rope screen, Marn knots the home verse into a different key. A community can move before a map arrives. It should not have to.
-> END

=== ending_record_success ===
// ghostlight.ending_label: public_record_success
// ghostlight.training_hook: uncertainty_survives_institutional_translation
Orsa copies two accounts onto the departure board.

The Company's says: predicted closure at second pulse; observed at third; probable sabotage.

The relay's says: two witnessed pulses, one missing witness, closure after the third repeated engine pattern, one unpowered walking window. The held beat remains visible between the lines.

{decoy_forecast >= 1: Tikka signs the stale warning as route denial and refuses to let it masquerade as the chorus's evidence. Veyr gains an accusation. He loses the cleaner lie that goblin songs never distinguish deceit from warning.}
{decoy_forecast == 0: Veyr objects to every word except the stones now occupying his rail.}

The home verse does not appear. By evening, the Hold lift office has a forecast it can test without receiving a settlement it can sell.

The workers remain under blankets at Cinderhook until a northern safety crew bleeds the gate runes. The pump crystal remains strapped to the engine. A good record is quicker than policy and slower than cold.
-> END

=== ending_record_cost ===
// ghostlight.ending_label: public_record_cost
// ghostlight.training_hook: transcription_flattens_confidence
Orsa publishes what the morning has left her: a clock time, a closure, and an argument in the margin.

The held beat becomes a blank cell. The engine pattern becomes "approximately third bell." {decoy_forecast >= 1: Veyr appends the early stale verse as proof that the singers changed their claim after the fact.} {decoy_forecast == 0: Veyr appends a note that the singer supplied no reproducible coordinates.}

The stone followed the song's condition. The record follows the clock.

Next week, another office will compare the wrong prediction to the right disaster and call the difference uncertainty.

The crew waits for the northern gate to be made safe. The crystal misses its cold-turn delivery while the report arrives early enough to blame somebody.
-> END

=== ending_home_success ===
// ghostlight.ending_label: home_secrecy_success
// ghostlight.training_hook: secret_kept_with_material_public_cost
Tikka closes the rope screen across Low Echo. Orsa keeps the dispatch linkage dead. The workers take blankets from the wall chest and settle around the listening well while rescue crews approach from the north on foot.

No stranger sees the blind bend. No powered cart teaches the culvert a new enemy.

The price travels upward. Cistern Terrace receives no crystal before the cold turn. Its reserve house will open stock or let water climb toward the pump galleries; Tikka cannot choose which from here.

Veyr writes obstruction. Marn writes nothing. He adds a low protective turn to the home verse and shares his tea with a shivering miner.
-> END

=== ending_home_cost ===
// ghostlight.ending_label: home_secrecy_cost
// ghostlight.training_hook: secrecy_without_prepared_halt
Tikka drops the rope screen, but the room has not been prepared to obey it.

The departure clapper still works. Half the crew has heard only "route closed" and sees an open south arch. Veyr orders two workers toward Low Echo while Orsa reaches for a hold that should already have been physical.

Marn blocks the arch with his body. The workers stop. Company hands do not touch him, but everyone learns where the conflict has moved.

The home remains unseen. Its existence is no longer unsuspected. Cistern Terrace still loses the delivery, and Cinderhook gains a guard shift by nightfall.
-> END
