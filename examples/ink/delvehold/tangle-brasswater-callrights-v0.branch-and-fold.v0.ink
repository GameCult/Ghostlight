// ghostlight.artifact_id: tangle_brasswater_callrights_branch_fold_v0
// ghostlight.fixture_id: tangle-brasswater-callrights-v0
// ghostlight.scene_id: tangle-brasswater-callrights-v0.winter-call-hearing
// ghostlight.final_ink_path: examples/ink/delvehold/tangle-brasswater-callrights-v0.branch-and-fold.v0.ink

VAR reserve_days = 13
VAR public_trust = 2
VAR company_leverage = 2
VAR temple_cover = 1
VAR rail_solidarity = 1
VAR evidence_quality = 0
VAR deep_debt = 0
VAR export_breach = 0
VAR ration_cost = 0
VAR cargo_state = 0

-> start

=== start ===
// ghostlight.scene: brasswater_counting_gallery
Brasswater Reserve House begins its weekly count with breakfast, because hunger makes arithmetic political before anyone has had the courtesy to open the meeting.

The house is cut into a basalt wedge beside a curving freight line. On the lower floor, a rune-lit weighbridge divides into two rails: the straight road toward the export tunnel and a short municipal siding ending at three barred crystal cells. Above them, a public counting gallery runs behind a bronze rail. Its wall gauge lists thirteen days of water, kitchens, lift service, and district heat at the chartered winter ration.

Master Lysa Harrow keeps Brasswater's one civic seal. She is a broad, iron-grey dwarf in a padded blue counting coat, with square spectacles and chalk on two fingers. The seal at her belt lets her execute the reserve charter. It does not let her invent a better winter.

-> introduce_constituencies

=== introduce_constituencies ===
// ghostlight.scene: brasswater_weekly_count
Dori Pike, a compact dwarf rail delegate in an orange work coat, stands at the bronze rail and counts the cell bars before she counts the crystals. Her crews can move a wagon after a lawful call. They can also decline to become the last link in somebody else's lie.

Weigh-priest Sava Copper sits at the gallery assay niche beneath a bronze balance. She wears a soot-white temple stole over leather work clothes and carries an ash-grey hold token used when weight, title, or hazard cannot yet be insured.

Yorrin Bale waits at the seal desk for the Deep Company that took Brasswater's advance and pledged its next three wagons to a call below fourteen days. He is a lean dwarf in an immaculate plum travelling coat, holding a brass contract tube as if it were a small tame weapon.

Behind the yellow public line, pump hands, kitchen Masters, lift mechanics, householders, and winter-warrant holders share hot rye bread. The warrants are the loans that funded Brasswater's advance; later power bills repay them. Everyone has a different reason to call the same thirteen days alarming.

The old cell is opened, counted, and closed. The new cell is weighed. Sava taps each tally with one brass fingernail. Yorrin corrects no arithmetic and watches every noun.

-> ordinary_count_choice

=== ordinary_count_choice ===
// ghostlight.choice_layer: ordinary_reserve_count
+ [Read each protected service aloud and have Dori repeat it to the rail crews.]
    // ghostlight.action: speak
    // ghostlight.branch: prime_public_schedule
    // ghostlight.intent: attach_the_stock_count_to_the_people_who_use_it
    ~ public_trust = public_trust + 2
    ~ rail_solidarity = rail_solidarity + 1
    Lysa faces the yellow line. "Water. Kitchens. Lifts. Heat."

    Dori repeats each word over the rail to the crews below while Lysa marks thirteen beside it. Nobody cheers. A public number is not comfort; it is merely a lie with fewer places to hide.
    -> routine_fold
+ [Ask Sava to assay the oldest loose crystals before the contracted train arrives.]
    // ghostlight.action: inspect_object
    // ghostlight.branch: prime_temple_assay
    // ghostlight.intent: strengthen_independent_evidence_before_the_dispute
    ~ temple_cover = temple_cover + 1
    ~ evidence_quality = evidence_quality + 1
    ~ ration_cost = ration_cost + 1
    Sava lays three cloudy crystals in the bronze balance and passes a gold-threaded rune plate beneath them.

    "Weight true," she says. "Pattern tired."

    The assay costs half an hour and one more chalk mark on the queue slate. Commerce theology has many mysteries. Most of them can be itemized.
    -> routine_fold
+ [Read the company's disaster clause with Yorrin at the seal desk before the gallery fills.]
    // ghostlight.action: inspect_contract
    // ghostlight.branch: prime_private_clause
    // ghostlight.intent: learn_the_company_escape_route_without_publicly_contesting_it
    ~ company_leverage = company_leverage + 1
    ~ evidence_quality = evidence_quality + 1
    ~ public_trust = public_trust - 1
    Yorrin rolls the contract open between them. Flood, hostile fauna, rail severance, divine act, and adaptive geomantic event each excuse a delayed call if an insurer accepts the classification.

    "You wrote a generous universe," Lysa says.

    "We expected to operate in it."

    By the time the public line notices the private reading, Yorrin has gained the useful appearance of being consulted.
    -> routine_fold
+ [Break the oldest cell seal and issue the morning half-ration before the count is finished.]
    // ghostlight.action: release_resource
    // ghostlight.branch: prime_immediate_service
    // ghostlight.intent: protect_morning_pumps_and_kitchens_at_the_cost_of_reserve_depth
    ~ reserve_days = reserve_days - 2
    ~ public_trust = public_trust + 1
    ~ ration_cost = ration_cost + 1
    Lysa takes the south stair and turns the cell key. Dori and two store hands load crystal baskets for Cistern House Nine and the public kitchens while the count continues above them.

    The bread line relaxes by one shoulder-width. The gauge falls to eleven days.

    Immediate mercy is still subtraction. It merely has witnesses.
    -> routine_fold

=== routine_fold ===
// ghostlight.fold: ordinary_work_before_pressure
Lysa stamps the weekly count, not a release order. Sava warms her hands over the assay lamp. Dori steals the end slice of rye and denies it while chewing. Yorrin polishes a fleck of flour from his contract tube.

{public_trust >= 4: The people behind the yellow line repeat the protected schedules back to one another. The gauge has become a shared claim, not Brasswater's private arithmetic.}
{public_trust <= 1: The private clause reading travels down the queue as a rumor with excellent shoes.}
{temple_cover >= 2: Sava's assay plate is already warm and witnessed when the rail bell sounds.}
{evidence_quality >= 1: Lysa has one independent mark beside the company's immaculate paperwork.}
{reserve_days <= 11: Morning service leaves the oldest cell visibly barer and the wall gauge closer to red.}
{rail_solidarity >= 2: Dori's crews stand where the switch lever and public gallery are both visible.}

Then the inbound bell rings three times and stops in the middle of the fourth.

-> disputed_wagon

=== disputed_wagon ===
// ghostlight.scene: brasswater_disputed_wagon
The Deep Company train enters the lower bay: three iron wagons under black canvas, brake runes steaming blue in the cold air. It should continue straight to a human export buyer. Brasswater's call-right can turn it onto the municipal siding because the gauge has fallen below fourteen days.

The lead canvas is peeled back. The crystals beneath are full-weight, bright, and webbed with pale mould. The nearest weighbridge rune dims around it.

Dori reaches the lower brake line first. Sava takes the south stair to the lead wagon and lowers her assay plate. Its gold test line gutters; the pale threads brighten.

Yorrin says, too quickly, "Harmless pressure bloom."

Dori says, "Then it can remember the export tunnel."

The wall gauge ticks once. Thirteen days becomes twelve as the morning pumps take their contracted draw.

-> cargo_choice

=== cargo_choice ===
// ghostlight.choice_layer: call_right_threshold
+ [Pull the brass switch lever and divert all three wagons onto Brasswater's siding.]
    // ghostlight.action: move_cargo
    // ghostlight.branch: seize_full_call
    // ghostlight.intent: execute_the_public_call_before_the_company_can_reframe_the_trigger
    ~ cargo_state = 2
    ~ export_breach = export_breach + 2
    ~ company_leverage = company_leverage - 1
    ~ public_trust = public_trust + 1
    Lysa pulls. Beneath the gallery, iron points grind left.

    Dori gives the hand signal. The train rolls past the straight export road and stops before the barred cells.

    Yorrin watches a foreign penalty become local heat. "The temple has not insured that load."

    "The charter did not ask whether winter was insured," Lysa says.
    -> hearing_fold
+ [Lay Sava's ash-grey hold token on the weighbridge and keep the train still.]
    // ghostlight.action: place_hold
    // ghostlight.branch: hold_for_public_assay
    // ghostlight.intent: preserve_title_and_hazard_evidence_before_any_release_or_export
    ~ cargo_state = 1
    ~ temple_cover = temple_cover + 1
    ~ evidence_quality = evidence_quality + 2
    ~ ration_cost = ration_cost + 1
    Sava places the flat grey token in the weighbridge socket. The rail runes go dark under the wheels.

    "Held for weight, title, and hazard," she says. "Not condemned. Not cleared."

    Nobody gets the wagon. The distinction is legally magnificent and physically cold.
    -> hearing_fold
+ [Accept Yorrin's offer to split one wagon to Brasswater and keep two moving toward export.]
    // ghostlight.action: negotiate_split
    // ghostlight.branch: accept_early_roll
    // ghostlight.intent: buy_immediate_supply_without_breaking_the_whole_export_contract
    ~ cargo_state = 3
    ~ reserve_days = reserve_days + 3
    ~ deep_debt = deep_debt + 2
    ~ company_leverage = company_leverage + 1
    ~ export_breach = export_breach + 1
    Yorrin produces the prepared page from his tube. One wagon now. Two senior call-rights later, both against a deeper shaft. Municipal power surcharges stand behind them.

    Lysa hates prepared mercy on principle. She signs only the split movement, not the roll.

    One wagon turns left. Two remain pointed at the export tunnel. The proposed debt stays on the desk, already behaving like an ancestor.
    -> hearing_fold
+ [Leave the switch straight and issue today's protected services from Brasswater's cells.]
    // ghostlight.action: preserve_contract
    // ghostlight.branch: spend_reserve_for_peace
    // ghostlight.intent: keep_the_disputed_load_moving_while_existing_public_stock_carries_the_day
    ~ cargo_state = 0
    ~ reserve_days = reserve_days - 2
    ~ company_leverage = company_leverage + 1
    ~ ration_cost = ration_cost + 1
    ~ public_trust = public_trust - 1
    The train remains on the straight road. Store hands open the oldest cell and fill municipal baskets instead.

    Yorrin relaxes. The household queue does not.

    Brasswater has paid for certainty by eating the one certainty it owned.
    -> hearing_fold

=== hearing_fold ===
// ghostlight.fold: public_call_hearing
The weekly count becomes a call hearing without anyone moving the bread.

Lysa stands at the seal desk above the forked rails. Sava has brought the assay plate back up the south stair and controls the niche and ash-grey hold. Dori has returned to the gallery switch station while her crews occupy the lower brake line. Yorrin has the contract, the export clock, and a mine payroll due before second bell. Behind the yellow line, every protected service has sent someone who can describe exactly what failure costs.

{cargo_state == 2: All three wagons wait on the municipal siding before barred cells. The export road is empty and expensive.}
{cargo_state == 1: The train sits on the weighbridge with dark rail runes, owned by its company and immobilized by a public hold.}
{cargo_state == 3: One wagon waits on the municipal siding while two point toward export. The disputed roll lies unsigned at the seal desk.}
{cargo_state == 0: All three wagons remain aligned with the export tunnel while Brasswater's oldest cell feeds the morning.}
{export_breach >= 2: Yorrin's contract clock shows a full foreign breach. He no longer has room to pretend the company loses nothing.}
{deep_debt >= 2: Two deeper-mine call-rights wait in the offered roll, senior to ordinary future purchases.}
{ration_cost >= 2: The queue slate has begun cutting evening heat to keep pumps and kitchens whole.}
{public_trust >= 4: The gallery crowd is orderly because it knows the numbers, not because it trusts the people holding them.}
{company_leverage >= 3: Yorrin speaks as if the Hold asked him to invent the emergency he financed.}

-> bargaining_choice

=== bargaining_choice ===
// ghostlight.choice_layer: constituency_bargain
+ {evidence_quality >= 1} [Ask Sava to read the assay and disaster clause into the outward-facing public ledger.]
    // ghostlight.action: publish_evidence
    // ghostlight.branch: publish_assay_and_clause
    // ghostlight.intent: deny_both_temple_and_company_a_private_definition_of_hazard
    ~ evidence_quality = evidence_quality + 2
    ~ public_trust = public_trust + 1
    ~ company_leverage = company_leverage - 1
    Sava turns the ledger toward the yellow line. Full weight. Mana-eating mould present. Classification unresolved. Disaster clause payable in coin if an insurer accepts the event.

    "Coin does not pump water," a kitchen Master says.

    "That," Sava replies, "is not disputed theology."
    -> decision_threshold
+ {rail_solidarity >= 2} [Give Dori the switch key until the affected workshops gather seals.]
    // ghostlight.action: transfer_custody
    // ghostlight.branch: rail_refusal_pending_moot
    // ghostlight.intent: make_physical_movement_depend_on_the_workers_bearing_its_risk
    ~ cargo_state = 1
    ~ rail_solidarity = rail_solidarity + 1
    ~ company_leverage = company_leverage - 1
    ~ ration_cost = ration_cost + 1
    Lysa passes the iron switch key across the bronze rail.

    Dori hangs it beside the brake roster. "No export. No release. Not until the people who will touch those crystals hear the assay."

    Yorrin calls it seizure. The crew calls it not moving.
    -> decision_threshold
+ [Write a limited roll: one wagon now, two later calls, but no senior lien on household power.]
    // ghostlight.action: counteroffer
    // ghostlight.branch: cap_the_roll
    // ghostlight.intent: trade_future_supply_claims_without_surrendering_household_tariffs
    ~ cargo_state = 3
    ~ reserve_days = reserve_days + 2
    ~ deep_debt = deep_debt + 1
    ~ company_leverage = company_leverage + 1
    ~ export_breach = export_breach + 1
    Lysa scores out the senior tariff lien and leaves two later calls against named wagons.

    Yorrin reads the scar in the contract. "My directors will reject this."

    "Then they may do so in writing, where winter can see them."
    -> decision_threshold
+ [Stamp Brasswater as self-insurer and order the whole load into segregated cells.]
    // ghostlight.action: assume_risk
    // ghostlight.branch: house_carries_disputed_loss
    // ghostlight.intent: secure_the_crystal_while_keeping_the_temple_from_owning_the_release_decision
    ~ cargo_state = 2
    ~ reserve_days = reserve_days + 5
    ~ temple_cover = temple_cover - 1
    ~ evidence_quality = evidence_quality + 1
    ~ export_breach = export_breach + 1
    Lysa stamps a narrow order: quantity received, title disputed, patterns segregated, no engine feed before witnessed test.

    Sava removes the temple's insurance mark. "You understand the house carries the loss."

    "The house is full of people who have been carrying it since breakfast."
    -> decision_threshold

=== decision_threshold ===
// ghostlight.fold: final_public_threshold
The rail bay settles into the shape of what Lysa has spent.

{evidence_quality >= 3: Assay marks and contract clauses face outward on the public ledger. Any later story must now argue with visible entries.}
{evidence_quality <= 1: The crystals remain full-weight, webbed with pallid mould, and politically convenient to describe.}
{rail_solidarity >= 3: Dori's crews hold the switch path together, hands off the controls until a lawful shared decision arrives.}
{temple_cover >= 2: Sava's ash-grey hold can carry the dispute into temple courts and insurers beyond this room.}
{temple_cover <= 0: Brasswater has the cargo and no outside institution willing to price what happens if it is unsafe.}
{company_leverage <= 1: Yorrin has lost control of the room's definition of emergency, though not the company's ability to miss payroll.}
{company_leverage >= 4: The offered roll now looks less like an option than the price of ending the conversation.}
{reserve_days >= 15: Enough crystal is physically inside municipal custody to outlast the immediate hearing.}
{reserve_days <= 9: The wall gauge is in the red band. Every elegant position has become a countdown.}
{deep_debt >= 3: Future call-rights and power bills are crowded with claims from a mine not yet safe enough to trust.}
{export_breach >= 2: A foreign buyer, a mine payroll, and the Hold's reputation for contract law are now on the cost ledger.}
{ration_cost >= 3: Evening heat has joined the meeting by becoming absent in advance.}

The charter gives Lysa four lawful ways to leave the room. None is a way to leave the system.

-> final_choice

=== final_choice ===
// ghostlight.choice_layer: winter_priority
+ [Execute the full call and put the disputed wagons behind Brasswater's bars.]
    // ghostlight.action: execute_call
    // ghostlight.branch: prioritize_current_services
    // ghostlight.intent: secure_present_water_food_transport_and_heat_despite_export_and_insurance_costs
    ~ cargo_state = 2
    ~ export_breach = export_breach + 1
    {evidence_quality >= 3 && (rail_solidarity >= 2 || temple_cover >= 2):
        -> ending_call_legible
    - else:
        -> ending_call_brittle
    }
+ [Sign the roll and bind two later calls to the deeper shaft.]
    // ghostlight.action: sign_contract
    // ghostlight.branch: prioritize_negotiated_supply
    // ghostlight.intent: preserve_wages_and_partial_supply_by_accepting_future_extraction_debt
    ~ cargo_state = 3
    ~ deep_debt = deep_debt + 1
    {company_leverage <= 2 && deep_debt <= 2:
        -> ending_roll_bounded
    - else:
        -> ending_roll_captured
    }
+ [Keep the train held, publish the ration, and carry the assay to a wider moot.]
    // ghostlight.action: widen_jurisdiction
    // ghostlight.branch: prioritize_public_moot
    // ghostlight.intent: preserve_evidence_and_distribute_the_delay_to_the_constituencies_who_must_decide
    ~ cargo_state = 1
    ~ ration_cost = ration_cost + 1
    {public_trust >= 4 && evidence_quality >= 3:
        -> ending_moot_credible
    - else:
        -> ending_moot_cold
    }
+ [Release the train to export and spend Brasswater's own cells to honor every existing contract today.]
    // ghostlight.action: spend_reserve
    // ghostlight.branch: prioritize_contract_continuity
    // ghostlight.intent: avoid_disputed_cargo_and_export_breach_by_burning_down_public_buffer
    ~ cargo_state = 0
    ~ reserve_days = reserve_days - 3
    {reserve_days >= 10 && ration_cost <= 2:
        -> ending_reserve_bridge
    - else:
        -> ending_reserve_exhausted
    }

=== ending_call_legible ===
// ghostlight.ending: full_call_legible
// ghostlight.training_hook: public_emergency_power_with_visible_costs
The three wagons roll onto the municipal siding under Dori's hand signals. Sava's marks remain on the public ledger. The responsive crystals go into a segregated cell behind two locks, one held by Brasswater and one by the test workshop.

The pumps keep their allotment. The kitchens keep theirs. The lift office tears up its first ration timetable.

Yorrin sends the export breach and payroll shortfall into the wider moot as costs, not as excuses. Lysa posts both beneath the wall gauge.

Brasswater has not solved the wound under the mountains. It has made today's choice difficult to steal.
-> END

=== ending_call_brittle ===
// ghostlight.ending: full_call_brittle
// ghostlight.training_hook: authority_without_shared_evidence
The wagons enter Brasswater's cells because Lysa's seal can still make rails move.

The crowd sees heat secured. It does not see enough evidence to agree why the cargo was taken, or enough shared custody to trust what happens next.

Yorrin calls the action opportunistic seizure. Sava declines the loss. Rail crews argue over whether a lawful order was also a safe one.

The Hold gains crystal and loses a common account of the act. That is how an emergency store becomes a faction headquarters by accident.
-> END

=== ending_roll_bounded ===
// ghostlight.ending: roll_bounded
// ghostlight.training_hook: negotiated_debt_with_retained_limits
One wagon enters Brasswater. Two continue toward export. The mine payroll clears.

The roll names two later wagons and no claim on household power tariffs. Sava witnesses the erasure. Dori posts the delivery dates beside the brake roster. If the deeper shaft fails, the company owes a public breach rather than a private winter.

It is still debt tied to expansion. Lysa signs because the terms have edges everyone in the gallery can point at.
-> END

=== ending_roll_captured ===
// ghostlight.ending: roll_captured
// ghostlight.training_hook: emergency_credit_as_expansion_leverage
One wagon turns left. Two go straight. Everyone in the gallery gets enough of what they need to postpone admitting what they traded.

The senior calls settle ahead of ordinary purchases. Future power surcharges stand behind them. The deeper shaft gains a constituency composed largely of people who would prefer it not exist but now require it to produce.

Yorrin leaves with the contract tube under one arm. The Hold keeps the other end of the leash and discovers it is tied around its own waist.
-> END

=== ending_moot_credible ===
// ghostlight.ending: wider_moot_credible
// ghostlight.training_hook: jurisdiction_follows_shared_consequence
The train remains dark on the weighbridge. Lysa posts the ration schedule. Sava carries the assay ledger; Dori carries the switch key; kitchen, pump, lift, warrant, and rail workshops carry the question outward with their seals and petitions.

Evening rooms cool by a degree. The cost is real and named.

By second bell the hearing is too large for Brasswater's keeper, the company factor, or the temple insurer to own alone. The winter has become a moot before it becomes a riot.
-> END

=== ending_moot_cold ===
// ghostlight.ending: wider_moot_cold
// ghostlight.training_hook: delayed_authority_without_constituency_trust
The train stays still while Lysa calls for more seals.

The public ledger is thin or the crowd does not trust it. Rumor moves faster than the ration carts: temple seizure, company fraud, House conspiracy, manufactured shortage. Each version finds a creditor by supper.

The wider moot will meet. It will meet cold, late, and already divided over what happened in the room it is meant to judge.
-> END

=== ending_reserve_bridge ===
// ghostlight.ending: reserve_only_bridge
// ghostlight.training_hook: current_contracts_paid_from_public_buffer
The Deep Company train takes the straight export road. No foreign breach is added. Brasswater opens two cells and keeps every protected schedule alive through the day.

The choice buys time without feeding the disputed crystals into a public engine. It also turns the wall gauge into tomorrow's argument.

Lysa posts the reduced days and the untouched contract side by side. Nobody mistakes the bridge for land.
-> END

=== ending_reserve_exhausted ===
// ghostlight.ending: reserve_only_exhausted
// ghostlight.training_hook: contract_continuity_spends_the_last_buffer
The train leaves cleanly enough to satisfy every signature on it.

Brasswater's own cells feed the pumps, kitchens, lifts, and heat until the gauge drops into red. Evening ration becomes overnight closure planning. Warrant holders ask which surcharge will refill an empty house; the company offers another advance.

The contracts survive the day. The public buffer does not. By morning, debt is the only full warehouse in Brasswater.
-> END
