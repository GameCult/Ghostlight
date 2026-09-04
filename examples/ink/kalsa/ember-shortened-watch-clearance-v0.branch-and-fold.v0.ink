// ghostlight.artifact_id: kalsa_ember_shortened_watch_clearance_branch_fold_v0
// ghostlight.fixture_id: ember-shortened-watch-clearance-v0
// ghostlight.scene_id: ember-shortened-watch-clearance-v0.last-clearance-bell
// ghostlight.final_ink_path: examples/ink/kalsa/ember-shortened-watch-clearance-v0.branch-and-fold.v0.ink

VAR crew_readiness = 2
VAR process_warning = 0
VAR record_separation = 0
VAR warning_state = 0
VAR upstream_protest = 0
VAR lower_route_clear = 1
VAR rescue_capacity = 1
VAR equipment_secured = 0
VAR isca_credibility = 2
VAR pel_position = 0
VAR copy_custody = 0
VAR stair_congestion = 0
VAR flow_deflection = 0

-> start

=== start ===
// ghostlight.scene: clearance_house_routine
The last clearance house stands where the reservoir's covered service stair opens onto the main outflow channel. Above it, an old metal throat waits inside newer stone. Below it, the channel bends past work yards and occupied ground that today's maintenance order calls empty.

Isca Rel keeps the bell frame, the upstream signal shutter, and the list of people who have actually been warned. She can tell the gatehouse to stop. She cannot make it listen.

Today the channel crew is replacing brace frames under a maintenance closure. Anet Vos, the foreperson, has six workers below. Pel Orra, youngest of them, is discovering that silt weighs more when a senior worker is watching.

Dema Tern, the conversion clerk, has brought the approved schedule. Dema's clean cuffs have survived the stair. This is considered evidence of rank or witchcraft depending on who has just carried the timber.

-> records_and_routine

=== records_and_routine ===
// ghostlight.scene: four_records_one_heading
Four records share one painted heading on Isca's table: a protected copy of the inherited source interval, Dema's signed conversion slip, the managers' work board, and Isca's tally of notices delivered down-channel.

They all use the same old watch word.

"One watch," Dema says.

"Which one?" Isca asks.

"The approved one."

Anet glances up from the stair. "Good. Water respects approval."

The work is ordinary enough to make the joke safe. Isca has time to prepare one thing before the last brace comes loose.

-> preparation_choice

=== preparation_choice ===
// ghostlight.choice_layer: clearance_house_preparation
+ [Copy the four records onto separate wax leaves before anyone smears their shared heading.]
    // ghostlight.action_label: use_object
    // ghostlight.branch_label: prepare_separate_records
    ~ record_separation = record_separation + 2
    ~ copy_custody = copy_custody + 1
    ~ crew_readiness = crew_readiness - 1
    Isca sets four wax leaves edge to edge and gives each record its own title, owner, and mark.

    Dema watches her copy the signed conversion. "You are making four problems."

    "No. I am preventing one problem from wearing four coats."

    Below, Anet shouts for another pair of hands. Isca keeps writing.
    -> routine_fold
+ [Coil the rescue line on the raised shelf and make Anet check both knots.]
    // ghostlight.action_label: move
    // ghostlight.branch_label: prepare_rescue_line
    ~ rescue_capacity = rescue_capacity + 2
    ~ crew_readiness = crew_readiness + 1
    ~ isca_credibility = isca_credibility + 1
    Isca drags the hemp line onto the dry shelf. Anet comes up far enough to test both knots with hands still grey from channel silt.

    "First knot?" Isca asks.

    "For lifting people."

    "Second?"

    "For lifting officials who say the first was unnecessary."

    Dema does not laugh. The line is sound anyway.
    -> routine_fold
+ [Descend to the side hollow and check the stone for early seep and vibration.]
    // ghostlight.action_label: inspect
    // ghostlight.branch_label: prepare_process_check
    ~ process_warning = process_warning + 2
    ~ crew_readiness = crew_readiness - 1
    Isca steps down beside the channel. The side hollow is dry. Grit lies slack in its bottom. The old throat sends no tremor through the wall.

    She presses two fingers to the stone until Pel asks whether the reservoir has said anything interesting.

    "It says you missed silt behind the third brace."

    Pel mutters something about multilingual stone and goes back for it.
    -> routine_fold
+ [Help Pel move the loose tools and spare brace shoes onto the landing.]
    // ghostlight.action_label: move_object
    // ghostlight.branch_label: prepare_equipment
    ~ equipment_secured = equipment_secured + 2
    ~ pel_position = 1
    ~ crew_readiness = crew_readiness + 1
    Isca and Pel lift the iron brace shoes onto the landing, then the mallets, wedges, and the basket of replacement pins.

    Pel leaves the long frame below. "That one needs three."

    "Then it has chosen the channel," Anet says. "We cannot all be upwardly mobile."
    -> routine_fold

=== routine_fold ===
// ghostlight.fold: routine_before_divergence
Anet's crew loosens the last brace. Dema checks the work board. Isca checks the bell rope, the shutter latch, and the people below.

{record_separation >= 2: Four wax leaves cool on the table, each answerable to a different human hand.}
{record_separation < 2: The four records remain pressed together beneath one painted watch heading.}
{rescue_capacity >= 3: The rescue line waits in two clean coils on the raised shelf.}
{process_warning >= 2: Isca already knows the side hollow's dry baseline and the old throat's ordinary silence.}
{equipment_secured >= 2: Tools and brace shoes sit above the channel on the landing.}
{pel_position >= 1: Pel works near the landing instead of at the deepest brace.}
{crew_readiness <= 1: Anet has had to lend Isca's missing hands to the schedule. The crew is behind, though the board insists time remains.}

The first wrong thing is small.

-> early_sign

=== early_sign ===
// ghostlight.scene: process_warning
Grit lifts in the side hollow.

Then a dark seep line draws itself across the stone below the rescue shelf. The old metal throat hums through Isca's boots.

The approved board still shows work time remaining.

Anet looks from the wet stone to Dema. "Your watch is running upstream."

Dema presses a thumb to the signed conversion. "The gatehouse accepted this value."

"The wall has declined it," Isca says.

Nobody below has heard that exchange. The gatehouse is too far upstream for speech. Isca has one interval in which to create a warning, a protest, or a cleaner record before the operator completes the accepted sequence.

-> warning_choice

=== warning_choice ===
// ghostlight.choice_layer: schedule_and_process_diverge
+ [Set the four records apart and make Dema name what each one controls.]
    // ghostlight.action_label: speak
    // ghostlight.branch_label: expose_record_divergence
    ~ record_separation = record_separation + 1
    ~ upstream_protest = upstream_protest + 1
    ~ isca_credibility = isca_credibility + 1
    ~ crew_readiness = crew_readiness - 1
    Isca points in turn: source interval, conversion, work board, delivered notice.

    "Say which one put Pel in the channel."

    Dema looks at the old copy, then at the board. "The managers issued the board."

    "And you converted the interval."

    "Yes."

    The word is quiet. It will survive longer than the neat heading.
    -> signal_fold
+ [Ring the lower warning now and order the work yards clear.]
    // ghostlight.action_label: use_object
    // ghostlight.branch_label: warn_lower_ground
    ~ warning_state = warning_state + 2
    ~ lower_route_clear = lower_route_clear + 2
    ~ stair_congestion = stair_congestion + 1
    ~ isca_credibility = isca_credibility - 1
    Isca strikes the lower bell before Dema can invoke the managers.

    The warning passes down the open channel. Carts turn. Yard workers abandon racks and baskets. People on occupied ground move toward the covered service stair because stone overhead looks like safety from below.

    Anet hears the bell and starts her crew upward.

    Dema says, "That warning was not authorized."

    "Then it should arrive before the authorized water."
    -> signal_fold
+ [Open the upstream protest shutter and pin Dema's signed conversion in its frame.]
    // ghostlight.action_label: use_object
    // ghostlight.branch_label: protest_upstream
    ~ upstream_protest = upstream_protest + 3
    ~ copy_custody = copy_custody + 1
    ~ process_warning = process_warning + 1
    ~ crew_readiness = crew_readiness - 1
    Isca throws the shutter wide. Its pale inner face turns toward the distant gatehouse: local protest, visible even through spray.

    She pins Dema's conversion slip beneath the sighting bar. The gesture says the submitted interval itself is disputed, not merely the crew's pace.

    Dema reaches for the slip, then stops. Removing it in full view of the gatehouse would also be a kind of message.
    -> signal_fold
+ [Send Pel to the rescue shelf and tell Anet to pull every worker off the lowest braces.]
    // ghostlight.action_label: speak
    // ghostlight.branch_label: pull_crew_early
    ~ pel_position = 2
    ~ rescue_capacity = rescue_capacity + 1
    ~ crew_readiness = crew_readiness + 2
    ~ equipment_secured = equipment_secured - 1
    Pel scrambles to the raised shelf. Anet orders the lower workers off the braces.

    The long frame, two mallets, and a basket of pins remain below.

    Dema says the board still grants time.

    Anet plants one silt-black hand on the stair rail. "The board may use it."
    -> signal_fold

=== signal_fold ===
// ghostlight.fold: warning_and_authority
The four clocks have stopped pretending to be one.

{record_separation >= 3: Source, conversion, board, and delivered notice lie visibly apart. Dema can no longer point at the heading without crossing Isca's distinctions.}
{record_separation <= 1: The shared heading still makes the disagreement look like bad speech rather than four accountable acts.}
{warning_state >= 2: The lower bell continues through the channel bends. Its warning moves people and also draws them toward the service stair.}
{upstream_protest >= 2: The pale protest shutter faces the gatehouse with Dema's signed slip fixed under its bar.}
{crew_readiness >= 4: Anet's people are already moving toward the stair and shelf.}
{crew_readiness <= 1: Several workers remain bent over braces because the schedule still has more authority than the wall.}
{pel_position >= 2: Pel crouches on the raised shelf with the rescue line within reach.}
{stair_congestion >= 1: Footsteps sound inside the covered stair: yard workers are climbing into the crew's only dry exit.}

The gatehouse signal changes.

-> release_accepted

=== release_accepted ===
// ghostlight.scene: accepted_release
The old throat answers the formula.

No god speaks. No archive admits fault. Metal shifts inside stone, and the hum becomes a blow felt through teeth.

The operator has submitted the approved sequence. The reservoir has accepted it. Acceptance proves exactly that much.

Water noses around the bend while workers are still inside the channel.

-> first_surge_choice

=== first_surge_choice ===
// ghostlight.choice_layer: first_surge_response
+ [Throw the rescue line to the lowest worker and haul until somebody else takes the rope.]
    // ghostlight.action_label: use_object
    // ghostlight.branch_label: first_surge_rescue
    ~ rescue_capacity = rescue_capacity + 1
    ~ crew_readiness = crew_readiness + 1
    ~ pel_position = 2
    ~ copy_custody = copy_custody - 1
    Isca throws. The coil opens cleanly if it was prepared and snarls around one table leg if it was not.

    Pel catches the second length from the shelf. Anet puts three workers on the rope and one on the stair mouth.

    Behind them, spray reaches the record table.
    -> surge_fold
+ [Keep striking the lower bell until the warning carries past the work yards.]
    // ghostlight.action_label: use_object
    // ghostlight.branch_label: first_surge_bell
    ~ warning_state = warning_state + 2
    ~ lower_route_clear = lower_route_clear + 1
    ~ stair_congestion = stair_congestion + 1
    ~ rescue_capacity = rescue_capacity - 1
    Isca holds the bell rope while the first water reaches her boots.

    Down-channel, people leave the low yards. More of them choose the covered stair. The warning saves distance and spends space.

    Anet shouts for Isca's hands. Isca strikes once more before she lets go.
    -> surge_fold
+ [Lift the source copy, signed conversion, work board, and notice tally onto the rescue shelf.]
    // ghostlight.action_label: move_object
    // ghostlight.branch_label: first_surge_records
    ~ copy_custody = copy_custody + 2
    ~ record_separation = record_separation + 1
    ~ rescue_capacity = rescue_capacity - 1
    ~ equipment_secured = equipment_secured - 1
    Isca gathers the four records without stacking them into agreement and passes them to Pel on the shelf.

    "Keep them apart."

    "They are all wet."

    "Wet is not the same thing as identical."

    Pel spreads them beneath the shelf lip while the channel takes the loose tools.
    -> surge_fold
+ [Wrench the abandoned brace frame across the side hollow to break the first surge.]
    // ghostlight.action_label: move_object
    // ghostlight.branch_label: first_surge_deflection
    ~ flow_deflection = 2
    ~ lower_route_clear = lower_route_clear + 1
    ~ equipment_secured = equipment_secured - 2
    ~ crew_readiness = crew_readiness - 1
    Isca and Anet drag the long frame sideways. It catches the first tongue of water and throws it against the empty service wall.

    The frame twists. Pins become bright little missiles. The pause is real and very short.

    Dema pulls one worker behind the door and loses a clean cuff forever.
    -> surge_fold

=== surge_fold ===
// ghostlight.fold: fixed_failure_variable_rescue
The release reaches the bend in full.

{rescue_capacity >= 3: The rescue line is taut between the channel floor, landing, and raised shelf. Workers climb through moving water instead of trusting the narrowing stair alone.}
{rescue_capacity <= 1: Too few hands and too little prepared line remain for every body below. Anet must choose who reaches the landing first.}
{warning_state >= 3: The bell has carried beyond the work yards; low ground is emptying before the main water arrives.}
{warning_state <= 1: The lower channel has heard work noise and one uncertain signal, not a warning it can safely act upon.}
{copy_custody >= 2: Four wet records remain separately legible beneath the rescue shelf lip.}
{copy_custody <= 0: Spray and hurried hands have left the record packet on the low table.}
{flow_deflection >= 2: The brace frame breaks the first surge against the service wall, buying breaths while destroying itself.}
{equipment_secured >= 2: Most loose tools are above the first water.}
{equipment_secured <= 0: Mallets, pins, and brace shoes turn in the brown current below.}
{stair_congestion >= 2: Yard workers meet the escaping crew inside the covered stair. Warning has moved danger instead of abolishing it.}

The house cannot save people, tools, evidence, and the lower ground with the same pair of hands. Isca chooses which claim must survive the water.

-> final_choice

=== final_choice ===
// ghostlight.choice_layer: irreversible_priority
+ [Put every available hand on the crew exit.]
    // ghostlight.action_label: move
    // ghostlight.branch_label: prioritize_crew_exit
    {crew_readiness + rescue_capacity >= 6 && stair_congestion <= 1:
        Isca gives Anet the landing and takes the rope. The crew comes out in an order built before panic could choose one.
        -> ending_crew_success
    - else:
        Isca calls everyone to the exit, but the route, rope, and bodies do not form one clean answer.
        -> ending_crew_cost
    }
+ [Keep the lower warning alive and clear the occupied ground.]
    // ghostlight.action_label: use_object
    // ghostlight.branch_label: prioritize_lower_warning
    {warning_state >= 3 && lower_route_clear >= 3 && isca_credibility >= 1:
        Isca rings the established warning until answering bells carry it beyond sight.
        -> ending_warning_success
    - else:
        Isca rings, but warning without route, time, or trusted meaning fractures down-channel.
        -> ending_warning_cost
    }
+ [Preserve the four records as four records.]
    // ghostlight.action_label: protect_object
    // ghostlight.branch_label: prioritize_record_chain
    {record_separation >= 3 && copy_custody >= 2:
        Isca climbs to the shelf and keeps source, conversion, board, and delivered notice apart while water takes the table.
        -> ending_records_success
    - else:
        Isca reaches for the record chain after spray, stacking, and hurried hands have already made part of it ambiguous.
        -> ending_records_cost
    }
+ [Hold the upstream protest in the gatehouse sightline.]
    // ghostlight.action_label: signal
    // ghostlight.branch_label: prioritize_upstream_protest
    {upstream_protest >= 3 && process_warning >= 2:
        Isca fixes the protest shutter open and marks the seep line beside it, giving the gatehouse both objection and observable process.
        -> ending_protest_success
    - else:
        Isca holds a protest the gatehouse can see but not yet distinguish from panic, delay, or disobedience.
        -> ending_protest_cost
    }

=== ending_crew_success ===
// ghostlight.ending_label: crew_exit_success
// ghostlight.training_hook: prepared_rescue_changes_who_reaches_safety
The last visible worker reaches the landing before the lower stair chokes with water and frightened people.

Pel's shoulder is bloodied. Two workers cannot stand without help. Elsewhere along the channel, the release has found crews the schedule called absent. The failure keeps its injuries.

{equipment_secured >= 2: The saved tools lie above them, a small mercy with handles.}
{equipment_secured < 2: Brace frames and iron shoes hammer the stone below until one wall gives.}

Anet counts the living before she counts the damage. Isca counts with her.
-> END

=== ending_crew_cost ===
// ghostlight.ending_label: crew_exit_cost
// ghostlight.training_hook: evacuation_without_prepared_capacity
The stair takes too many bodies at once.

The rescue line goes tight around one worker and slack for another. Pel gets a hand under a senior mason's arm and loses his footing. Isca cannot tell whether the sound below is timber, bone, or both.

They pull people out. They do not pull everyone out cleanly. Water carries tools and broken channel work toward ground the board had declared empty.
-> END

=== ending_warning_success ===
// ghostlight.ending_label: lower_warning_success
// ghostlight.training_hook: warning_preserves_people_not_property
Answering bells move down-channel ahead of the main water.

The low yards empty. Families leave baskets, drying racks, carts, and stock where they stand. Water reaches those places anyway and tears the work into the channel. The warning preserves bodies, not the fiction that empty ground costs nothing.

Behind Isca, Anet's crew emerges injured and incomplete in ways the later inquiry will count badly.
-> END

=== ending_warning_cost ===
// ghostlight.ending_label: lower_warning_cost
// ghostlight.training_hook: signal_without_shared_meaning
The bell carries. Its meaning does not travel intact.

One yard clears. Another sends people toward the covered stair. A third waits for the authorized pattern that never comes. Water reaches all three with different preparations and the same indifference.

Dema will later say the warning caused confusion. Isca will say confusion arrived signed.

Behind the bell, channel workers reach the landing injured while broken braces and tools travel down with the water.
-> END

=== ending_records_success ===
// ghostlight.ending_label: record_chain_success
// ghostlight.training_hook: separated_evidence_survives_fixed_disaster
The clearance house floods to the landing. The four records survive above it, wet at the edges and separate in custody and meaning.

Source interval. Signed conversion. Approved board. Delivered notice.

Below, braces break and workers are hurt. Water reaches occupied ground. The records save nobody retroactively. They prevent the first inquiry from making pronunciation the only defendant in the room.
-> END

=== ending_records_cost ===
// ghostlight.ending_label: record_chain_cost
// ghostlight.training_hook: merged_record_invites_wrong_inquiry
Isca saves a packet.

The shared heading survives best. One mark bleeds into another. Dema's signature remains legible while the delivery tally loses its lower edge.

When the first inquiry asks why workers misunderstood the watch word, the surviving paper appears to agree with the question. Injured channel workers will have to carry the missing distinctions in their bodies and testimony.
-> END

=== ending_protest_success ===
// ghostlight.ending_label: upstream_protest_success
// ghostlight.training_hook: observable_process_challenges_machine_acceptance
The pale shutter faces upstream through spray. Beside Dema's signed conversion, Isca marks the dark seep line and the lifted grit that appeared while the board still promised time.

The gatehouse has already released. It cannot pretend nobody downstream reported divergence before the water arrived.

The old throat accepted its formula. The wall supplied a second witness. Later reviewers will have to decide why only one was treated as authority.

Below the shutter, bracework breaks and injured workers are hauled through water already carrying tools toward the lower ground.
-> END

=== ending_protest_cost ===
// ghostlight.ending_label: upstream_protest_cost
// ghostlight.training_hook: unsupported_protest_is_easy_to_discipline
The protest shutter stays open until the spray hides it.

No signed slip remains in its frame. No earlier process mark establishes when the wall changed. The gatehouse can record an unauthorized signal after a valid acceptance and call the sequence complete.

Isca will still testify. So will the injured workers. A protest without its comparison evidence is not false. It is merely easier for an office to punish than to understand.
-> END
