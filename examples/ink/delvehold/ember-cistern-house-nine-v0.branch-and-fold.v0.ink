// ghostlight.artifact_id: ember_cistern_house_nine_branch_fold_v0
// ghostlight.fixture_id: ember-cistern-house-nine-v0
// ghostlight.scene_id: ember-cistern-house-nine-v0.one-shift-reserve
// ghostlight.final_ink_path: examples/ink/delvehold/ember-cistern-house-nine-v0.branch-and-fold.v0.ink

VAR header_hours = 8
VAR repair_progress = 0
VAR bearing_damage = 2
VAR public_trust = 2
VAR ration_seals = 0
VAR evidence_chain = 1
VAR override_exposure = 0
VAR flood_risk = 1

-> start

=== start ===
// ghostlight.scene: house_nine_second_shift_establishing
Cistern House Nine keeps time in water.

Eight chalk squares stand on the header board above the public landing. Each square is one hour the upper terrace can drink if all three pumps stop. At second-shift bell, Journeyworker Rada Venn redraws the box around all eight because the reserve is full and the clock has begun. This is considered more dignified than writing EIGHT HOURS UNTIL EVERYONE BECOMES PERSONALLY INTERESTED.

The pump house occupies two levels beside a warm underground sea. The west street landing opens into an upper dry rune gallery whose south brass rail guards a stair opening to the wet intake chamber below. Three squat iron pump engines stand west to east beneath the gallery rail; their floats, valve extensions, wedge throats, and slates remain within reach from the upper service path while their rods descend into black water. A locked conduit arch brings blue municipal mana through the north gallery wall. A chain hoist hangs beside the grated stair. Three outlet trunks turn west beneath the landing toward the terraces.

-> shift_routine

=== shift_routine ===
// ghostlight.scene: house_nine_shift_routine
Rada is a dwarf pumpwright with journey papers, a leather apron, and no civic seal. She owns the work her hands perform. Master Kelda Moor owns the workshop mark that admits the result to public service.

Kelda stands beside the inspection plate, grey braids tucked into her collar. Landing clerk Iven Chalk keeps a second record on a slate facing the street. The record names what the public can see: Engine One shedding bronze into its intake screen; Engine Two dark behind a seated red isolation wedge after its null test found a route no engineer drew; Engine Three carrying household flow alone.

A narrow brass covenant strip borders the inspection plate: sealed isolation and repair are covered; unstamped emergency damage belongs to the house. Iven's public slate carries the same two marks in chalk. Insurance is a form of magic which works best when nothing interesting happens.

The bakery and laundry peaks are marked as two overlapping red arcs on the header board.

Iven passes Rada the heel of a hot seed loaf through the rail. "Payment from the bakery for not discovering water is optional."

"Tell them bearings are also traditional," Rada says.

Engine One answers with a dry tick from below. Nobody laughs twice.

-> shift_preparation

=== shift_preparation ===
// ghostlight.choice_layer: spend_the_first_hour
+ [Lay out the spare sleeve, lifting yoke, and clean catch trays before draining the wet chamber.]
    // ghostlight.action: stage_repair_tools
    // ghostlight.branch: prime_repair
    // ghostlight.intent: buy_craft_speed_with_reserve_time
    // ghostlight.consequence: repair_progress_up_header_hours_down_evidence_up
    ~ repair_progress = repair_progress + 2
    ~ header_hours = header_hours - 1
    ~ evidence_chain = evidence_chain + 1
    Rada carries the spare bearing sleeve to the yellow repair square, then sets the lifting yoke, drift pins, white catch trays, and dry-chamber lamps in working order.

    Kelda checks the arrangement without touching it. "You have omitted panic."

    "It never stays where I put it."

    Iven clears another chalk square while the district drinks through Engine Three.
    -> routine_fold
+ [Move the header board to the public rail and call every remaining hour aloud.]
    // ghostlight.action: move_public_record
    // ghostlight.branch: prime_public_clock
    // ghostlight.intent: make_the_deadline_common_knowledge
    // ghostlight.consequence: public_trust_and_evidence_chain_up
    ~ public_trust = public_trust + 2
    ~ evidence_chain = evidence_chain + 1
    Rada unhooks the heavy board and carries it to the rail. Iven steadies the lower edge while she turns eight chalk squares toward the waiting terrace.

    "Eight hours at household draw," Rada calls. "Less when ovens and wash drums open together."

    People stop asking whether the house is broken and begin asking what eight hours permits. It is not calmer. It is more useful.
    -> routine_fold
+ [Ask Iven to summon the bakery and laundry seals before their peak valves open.]
    // ghostlight.action: request_emergency_moot
    // ghostlight.branch: prime_ration_authority
    // ghostlight.intent: assemble_district_authority_before_the_workshop_needs_it
    // ghostlight.consequence: ration_seals_up_header_hours_down
    ~ ration_seals = ration_seals + 2
    ~ header_hours = header_hours - 1
    Iven copies the bearing count, the isolated engine, and the header clock onto two route slips.

    "Ration before thirst?" Kelda asks.

    "A civic innovation," Rada says. "It may be illegal."

    The clerk sends runners uphill while the bakery and laundry Masters are still near enough to be annoyed in person.
    -> routine_fold
+ [Hold Engine One at one-third load between the household pulses.]
    // ghostlight.action: feather_damaged_engine
    // ghostlight.branch: prime_water_margin
    // ghostlight.intent: buy_header_time_by_spending_the_bearing
    // ghostlight.consequence: header_hours_up_bearing_damage_and_override_exposure_up
    ~ header_hours = header_hours + 2
    ~ bearing_damage = bearing_damage + 1
    ~ override_exposure = override_exposure + 1
    Rada eases the manual cam until Engine One's rod begins a shallow stroke. The outlet gauge lifts by half a mark.

    So does the sound from the lower bearing: tick, scrape, tick.

    Kelda does not stamp the plate. Iven records that too.
    -> routine_fold

=== routine_fold ===
// ghostlight.fold: preparation_meets_the_clock
Engine Three carries the household pulse. Engine Two's red wedge stays flush. Engine One waits with bronze dust shining in the catch dish like a cheap saint's halo.

{repair_progress >= 2: The spare sleeve, yoke, pins, trays, and lamps stand ready inside the yellow square. The repair has not begun, but its first half-hour no longer belongs to fetching.}
{public_trust >= 4: The header board faces the landing. Neighbours repeat the count accurately, which is a severe kind of solidarity.}
{ration_seals >= 2: Two runners have gone uphill for the bakery and laundry seals before either commercial peak begins.}
{bearing_damage >= 3: Engine One's one-third stroke adds water to the header and a wider crescent of bronze to the catch dish.}
{override_exposure >= 1: The inspection plate remains unstamped beside Iven's written note of the manual cam position.}
{evidence_chain >= 2: Catch trays, clock marks, wedge state, and manual acts now have a time and a witness outside Rada's memory.}
{header_hours <= 5: Five or fewer chalk squares remain. The public landing begins counting vessels instead of people.}

Two hours pass in the old way: one cup, one washbasin, one argument at a time.

~ header_hours = header_hours - 2

-> bearing_failure

=== bearing_failure ===
// ghostlight.scene: house_nine_bearing_failure
Engine One's tick becomes a crack.

The common intake screen jerks sideways. A bronze crescent jumps from the bearing housing and rings against the brass rail. Engine Three's pressure float drops, recovers, then drops again as fragments disturb the water below its pump foot.

From the gallery wall, Engine Two's dark feed line answers with one hair-thin blue loop around the seated wedge.

Kelda catches the emergency lever before habit does. "One is eating the intake. Two is still writing. Three is beginning to drink air."

Iven draws a bronze crescent beside the current square on the header board.

-> triage_choice

=== triage_choice ===
// ghostlight.choice_layer: keep_one_failure_from_becoming_three
+ [Seat Engine One's wedge, lock its cam, and begin the documented shutdown.]
    // ghostlight.action: isolate_damaged_engine
    // ghostlight.branch: commit_to_repair
    // ghostlight.intent: preserve_the_remaining_machinery_and_begin_safe_access
    // ghostlight.consequence: repair_progress_and_public_trust_up_header_hours_down
    ~ repair_progress = repair_progress + 2
    ~ public_trust = public_trust + 1
    ~ header_hours = header_hours - 2
    Rada drives the red wedge home and hangs the cam key on Kelda's seal chain. Iven calls the time. Kelda calls the state. The gallery watches Engine One become honestly unavailable.

    The wet chamber still cannot be drained until Three stops. The repair now has a boundary and a worse clock.
    -> pressure_fold
+ [Use the chain hoist to lift the common intake screen and rake loose bronze into a catch tray.]
    // ghostlight.action: clear_intake_screen
    // ghostlight.branch: buy_clean_water_path
    // ghostlight.intent: reduce_fragment_damage_while_engine_three_holds_flow
    // ghostlight.consequence: bearing_damage_down_repair_progress_and_flood_risk_up
    ~ bearing_damage = bearing_damage - 1
    ~ repair_progress = repair_progress + 1
    ~ flood_risk = flood_risk + 1
    Rada clips her belt to the rail, takes the hoist chain in both hands, and lifts the dripping intake screen through the service slot. Kelda braces the frame while Rada rakes bronze slivers into white ceramic.

    Engine Three keeps stroking beside an open service slot. Warm water slaps the lower grating. The intake clears; the wet floor acquires an opinion.
    -> pressure_fold
+ [Repeat Engine Two's live null test and copy the feed-side loop onto Iven's slate.]
    // ghostlight.action: preserve_reroute_evidence
    // ghostlight.branch: strengthen_claim_chain
    // ghostlight.intent: keep_the_unsafe_second_engine_and_insurance_dispute_inspectable
    // ghostlight.consequence: evidence_chain_up_header_hours_down
    ~ evidence_chain = evidence_chain + 2
    ~ header_hours = header_hours - 1
    Rada draws null right to left. Engine Two goes dark. The thin blue loop forms outside the wedge and reaches toward the common feed.

    She copies the exact curve beside Iven's time mark. It supplies no water. It does make "operator error" work harder for its supper.
    -> pressure_fold
+ [Lift Engine One's cam and drive it hard enough to refill two header squares.]
    // ghostlight.action: run_unstamped_override
    // ghostlight.branch: spend_machinery_for_water
    // ghostlight.intent: protect_immediate_service_by_accepting_mechanical_and_contract_risk
    // ghostlight.consequence: header_hours_up_bearing_damage_override_exposure_and_flood_risk_up_public_trust_down
    ~ header_hours = header_hours + 3
    ~ bearing_damage = bearing_damage + 2
    ~ override_exposure = override_exposure + 2
    ~ flood_risk = flood_risk + 1
    ~ public_trust = public_trust - 1
    Rada lifts the cam. Engine One strikes the water hard enough to shake chalk from the header board.

    Two squares return. The catch dish fills with bronze curls. Beneath the landing, the outlet trunk knocks once like a fist testing a door.

    Kelda's seal stays in her closed hand.
    -> pressure_fold

=== pressure_fold ===
// ghostlight.fold: mechanical_failure_becomes_district_jurisdiction
The three engines now describe the dispute more honestly than anyone's speech.

{repair_progress >= 3: Rada has the tools, access sequence, and first isolation work needed to fit the sleeve inside half a shift.}
{repair_progress <= 1: The spare sleeve is still a piece of hope on a shelf, surrounded by the tools it will eventually require.}
{bearing_damage <= 1: Most loose bronze is in a white catch tray instead of the common intake.}
{bearing_damage >= 4: Engine One's catch dish is no longer counting filings. It is collecting parts.}
{flood_risk >= 2: Warm water shines across the lower grating or knocks inside the buried outlet trunk; the lower service floor has become part of the wager.}
{evidence_chain >= 3: Iven's slate carries the clock, wedge states, bronze count, null loop, and every hand that moved a cam.}
{evidence_chain <= 1: The insurer will receive a story assembled after the machinery chose its own punctuation.}
{public_trust >= 4: The landing has seen the same clock and states as the workshop. Fear has not become agreement, but it has become specific.}
{public_trust <= 1: Every quiet exchange behind the rail now looks like the private portion of a public failure.}
{header_hours <= 3: Three or fewer chalk squares remain. Household draw alone will empty them before next shift.}
{override_exposure >= 2: The unstamped plate and scored cam record make the insurance exception visible from across the rail.}

-> moot_arrival

=== moot_arrival ===
// ghostlight.scene: house_nine_emergency_moot
The bakery and laundry peaks arrive first as sound: oven valves opening in the upper pipes, wash drums taking their first deep fill.

Then the workshops arrive in person. Two Masters stand at the public rail, one flour-dusted in a quilted apron, one damp-sleeved with copper wash tokens at the belt. Each carries a civic seal. They have authority over their own draw and no appetite for owning the pump house's machinery.

{ration_seals >= 2: Iven's runners found them before the valves opened. Their seal boards are already prepared for a one-shift ration.}
{ration_seals < 2: They came because the pipes coughed. Neither brought a written ration mandate; urgency has arrived before procedure and is pretending this is unusual.}

{public_trust >= 4: The public header board gives both Masters the same remaining count.}
{public_trust < 4: They begin by asking whose number is official. One of the remaining chalk squares disappears during the answer.}

~ header_hours = header_hours - 1

Kelda lays the House Nine seal beside the unstamped inspection plate. "We can spend the header on repair, spend commercial work on ration, spend the evidence on a claim, or spend the pumps on pretending none of this costs anything."

Rada looks at the wet stair, the public rail, and the remaining squares.

-> final_state

=== final_state ===
// ghostlight.scene: house_nine_last_shift_threshold
{header_hours >= 5: Five or more chalk squares remain, enough water for a prepared half-shift repair if the district accepts the drawdown.}
{header_hours >= 2 && header_hours < 5: Between two and four squares remain. There is room for one disciplined plan and almost none for correction.}
{header_hours <= 1: One square or less remains. Every choice now begins by admitting someone will open a dry tap.}

{repair_progress >= 3: The repair square is staged and Engine One can be made safe for sleeve work.}
{repair_progress < 3: The sleeve exists, but the access, isolation, or tools are not ready enough to promise a half-shift repair.}
{ration_seals >= 2: Bakery and laundry can bind their own peak valves under a district ration.}
{ration_seals < 2: Commercial ration remains a request with too few seals attached to its consequence.}
{evidence_chain >= 3: The insurer can inspect a sequence rather than choose the cheapest liar.}
{evidence_chain < 3: The evidence has gaps exactly where an unstamped act becomes expensive.}
{override_exposure >= 2: The emergency override is already part of the public record and the insurance covenant is in jeopardy.}
{flood_risk >= 3: Water stands over the lower grating and the buried trunk answers each pump stroke. A failed plan can now drown the repair floor.}

There is no choice that saves water, machinery, wages, proof, and pride together. That is why it is a civic decision instead of a maintenance trick.

-> final_choice

=== final_choice ===
// ghostlight.choice_layer: spend_the_last_shift
+ [Seal all three engines, drain the wet chamber, and fit the spare sleeve.]
    // ghostlight.action: execute_half_shift_repair
    // ghostlight.branch: choose_repair_window
    // ghostlight.intent: spend_stored_water_to_restore_a_safe_two_engine_margin
    {repair_progress >= 3 && header_hours >= 2 && flood_risk <= 2:
        -> ending_repair_success
    - else:
        -> ending_repair_cost
    }
+ [Ask the bakery and laundry Masters to seal their peak valves for one shift.]
    // ghostlight.action: ratify_district_ration
    // ghostlight.branch: choose_civic_ration
    // ghostlight.intent: spend_commercial_output_and_shared_trust_to_keep_engine_three_within_household_load
    {ration_seals >= 2 && public_trust >= 3 && header_hours >= 2:
        -> ending_ration_success
    - else:
        -> ending_ration_cost
    }
+ [Accept dry taps: leave Two isolated, seal the shutdown record, and submit the insurer's emergency claim.]
    // ghostlight.action: preserve_claim_boundary
    // ghostlight.branch: choose_insurance_claim
    // ghostlight.intent: preserve_machinery_and_attribution_even_when_the_terrace_runs_dry
    {evidence_chain >= 3 && override_exposure <= 1:
        -> ending_claim_success
    - else:
        -> ending_claim_cost
    }
+ [Open Engine One under emergency override and fill the header before the commercial peak crests.]
    // ghostlight.action: overdrive_damaged_pump
    // ghostlight.branch: choose_emergency_output
    // ghostlight.intent: buy_immediate_water_by_risking_the_bearing_trunk_and_insurance_covenant
    {bearing_damage <= 3 && flood_risk <= 2:
        ~ override_exposure = override_exposure + 2
        ~ bearing_damage = bearing_damage + 1
        -> ending_override_success
    - else:
        ~ override_exposure = override_exposure + 2
        ~ flood_risk = flood_risk + 2
        -> ending_override_collapse
    }

=== ending_repair_success ===
// ghostlight.ending_label: prepared_repair_success
// ghostlight.training_hook: stored_resource_buys_safe_craft_time
All three wedges show red. Kelda seals the shutdown. Iven turns the header board so the district can watch its own water being spent.

The wet chamber drains below the pump feet. Rada and Kelda lower the yoke, draw Engine One's scored shaft, and drive the spare sleeve home while the last safe squares vanish one by one.

Engine One returns on a clean null test before the header empties. Engine Three gains a partner. Engine Two stays dark and disputed.

House Nine survives because preparation made half a shift mean half a shift.
-> END

=== ending_repair_cost ===
// ghostlight.ending_label: unprepared_repair_cost
// ghostlight.training_hook: repair_promise_without_material_readiness
Kelda seals the shutdown. The pumps stop. The header begins paying for every tool not staged and every wet surface not made safe.

{flood_risk >= 3: The hoist crew must first clear water from the lower grating and secure the open service slot.}
{repair_progress < 3: Rada loses an hour fetching the yoke, lamps, or dry catch trays while the spare sleeve waits uselessly beside Engine One.}
{header_hours <= 1: The upper taps run dry before the old sleeve leaves the shaft.}

The repair remains the right work. It simply arrives after the district needed the right work to have started.
-> END

=== ending_ration_success ===
// ghostlight.ending_label: district_ration_success
// ghostlight.training_hook: local_authority_spends_its_own_output
The bakery Master seals the oven-feed valve at half draw. The laundry Master binds the wash drums to household intervals. Kelda seals Engine Three's restricted service state. Iven writes the three marks on one public board.

Bread output falls by noon. The wash queue doubles. Household taps keep running, and Engine Three's pressure float settles above the air line.

The district buys a repair window with its own production instead of asking one pumpwright to counterfeit one out of iron.
-> END

=== ending_ration_cost ===
// ghostlight.ending_label: ration_without_authority_cost
// ghostlight.training_hook: urgent_request_cannot_impersonate_shared_authority
Rada asks for a one-shift ration. The bakery Master asks whether the laundry is bound. The laundry Master asks whose header count is official. Kelda cannot seal their valves for them.

The red arcs meet while the question travels outward for more seals. Engine Three pulls air. The public gauge falls.

The failure is now district-sized, and so is the queue. Jurisdiction widens after the water has already done the paperwork.
-> END

=== ending_claim_success ===
// ghostlight.ending_label: clean_claim_success
// ghostlight.training_hook: attribution_preserved_at_immediate_service_cost
Kelda seals the shutdown and the preserved isolation state. Rada bags the bronze crescents. Iven signs the clock, cam, wedge, and null-loop sequence as public witness.

The insurer cannot call the failure an unstamped override. The sleeve work and emergency replacement stock become a covered obligation instead of House Nine's private loss.

The terrace still runs dry tonight. The workshop survives the claim, Engine Two remains isolated, and nobody gets to purchase a cleaner story with the district's thirst.
-> END

=== ending_claim_cost ===
// ghostlight.ending_label: contaminated_claim_cost
// ghostlight.training_hook: evidence_gap_moves_loss_to_the_weakest_owner
Kelda seals what remains. The insurer's reader finds the gaps first: an unstamped cam movement, an uncounted bronze curl, a null loop copied after the gauge fell.

The claim is suspended. The pumps stay down. House Nine must finance the sleeve work while the district pays for water it is not receiving.

An evidence chain does not become strong because everyone now desperately requires it to have been strong.
-> END

=== ending_override_success ===
// ghostlight.ending_label: emergency_output_success
// ghostlight.training_hook: immediate_relief_buys_a_worse_next_shift
Rada opens Engine One and holds the cam below the line where the rod begins to buck. Kelda refuses the inspection stamp. Iven records both acts.

Water climbs through the header squares faster than bronze fills the catch dish. The commercial peak passes. The buried trunk holds.

The terrace keeps water for another shift. Engine One will not survive another such mercy, and the insurance covenant now has an exception large enough to live in.
-> END

=== ending_override_collapse ===
// ghostlight.ending_label: outlet_trunk_collapse
// ghostlight.training_hook: emergency_output_crosses_the_mechanical_failure_threshold
Engine One takes the mana and turns it into water, noise, and one final piece of bad arithmetic.

The pump foot uncovers. The next stroke hammers the buried outlet trunk. Stone jumps beneath the landing; a seam opens below the brass rail; warm black water punches across the lower service floor and drowns the rune slates in blue steam.

Kelda throws every wedge. Too late for the trunk. Just in time to keep the municipal feed from joining it.

Cistern House Nine loses all three engines for days. The header gains less than an hour.
-> END
