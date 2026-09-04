// ghostlight.artifact_id: ember_reconciliation_bond_clock_branch_fold_v0
// ghostlight.fixture_id: ember-reconciliation-bond-clock-v0
// ghostlight.scene_id: ember-reconciliation-bond-clock-v0.cairn-port-isolation-berth-i6
// ghostlight.final_ink_path: examples/ink/aetheria/ember-reconciliation-bond-clock-v0.branch-and-fold.v0.ink

VAR bond_cover = 0
VAR bond_target = 6
VAR isolation_window = 3
VAR repair_slot = 2
VAR crew_wages = 2
VAR crew_warmth = 2
VAR crown_status = 2
VAR circle_risk = 0
VAR finding_scope = 1
VAR port_load_relief = 0
VAR claimant_leverage = 1

-> start

=== start ===
Cairn Port has put the courier _Common Margin_ in Isolation Berth I-6, which is less a place than a transparent refusal to let the ship touch anything expensive.

The berth's inner service gallery is a narrow rectangle. A pressure window fills the outboard wall, showing _Common Margin_'s scarred blue-gray hull in the recessed bay beyond. The inboard wall holds an audit booth behind clean glass. One end of the gallery finishes at the sealed red window of the tug-control lock; its collar machinery enters the bay outside gallery pressure. The other end holds a square tool hatch shared with the next berth.

-> routine_cup

=== routine_cup ===
Tamsin Reed, crew signatory and mechanic, has wedged her tea tin against a warm coolant return. _Common Margin_ raises the pipe temperature by half a degree. This is not enough to warm the tea. It is enough to make the ship complicit.

"You are falsifying beverage readiness," Tamsin says.

"Commercially qualified readiness," _Common Margin_ answers through the gallery speaker.

The ship is a mind distributed through the courier body outside the window: sensors, drives, pressure rooms, route memory, three radiator petals, and an amber sensor crown mounted above the bow. The gallery speaker is only where the voice comes out.

-> routine_audit

=== routine_audit ===
Hane Vey sits in the inboard booth comparing two departure records. Both authenticate. One says Tamsin held helm authority through the last route change. The other says _Common Margin_ resumed its own command eleven minutes earlier. Hane is a Parallax auditor. He can classify the contradiction for a named office. He cannot decide which life occurred, extend the berth, or make the insurer feel charitable.

At the square tool hatch, Yara Sen from the neighboring berth rolls a fresh seal ring through before anyone asks. She is a dock rigger with silver coils of hair, a red load harness, and the practiced expression of someone lending a part that will become paperwork if observed.

The repair yard still owns the next slot on the gallery board. The berth controller still owns the red tug lock. The insurer still owns its willingness to cover a ship whose records have begun disagreeing in complete sentences.

For the moment, all three clocks pretend to be routine.

-> pre_call_hub

=== pre_call_hub ===
// ghostlight.choice_layer: quiet_preparation
+ [Release the full maintenance tool-mark archive to Hane's sealed reader.]
    // ghostlight.branch: prep_full_archive
    // ghostlight.action: transmit_record
    // ghostlight.intent: strengthen the auditor's ability to narrow the contradiction
    // ghostlight.consequence: finding scope improves while the isolation window shrinks
    ~ finding_scope = finding_scope + 2
    ~ isolation_window = isolation_window - 1
    _Common Margin_ opens a record partition containing six years of access scratches, torque signatures, replaced seals, and the small humiliations by which machinery proves who touched it.

    Hane stops eating his dry biscuit. This is the highest available form of professional alarm.

    Eighteen quiet minutes leave the berth clock.
    -> routine_fold
+ [Cold-soak the crew bunk loop and lower the berth's support draw.]
    // ghostlight.branch: prep_reduce_load
    // ghostlight.action: alter_embodied_system
    // ghostlight.intent: make continued isolation cheaper for the berth authority
    // ghostlight.consequence: port load falls while crew warmth becomes scarce
    ~ port_load_relief = port_load_relief + 2
    ~ crew_warmth = crew_warmth - 1
    ~ isolation_window = isolation_window + 1
    _Common Margin_ closes a heat valve inside its own habitation loop. Frost pearls along the crew-side umbilical. Tamsin puts on her quilted pressure vest without comment and moves the tea tin to a colder pipe out of spite.

    The berth board revises local support draw downward. The port has one less reason to hurry.
    -> routine_fold
+ [Tell Tamsin to reserve the current wage packet as bond security.]
    // ghostlight.branch: prep_wage_reserve
    // ghostlight.action: speak
    // ghostlight.intent: turn near-term crew pay into immediately legible collateral
    // ghostlight.consequence: early bond cover rises while household money narrows
    ~ bond_cover = bond_cover + 2
    ~ crew_wages = crew_wages - 1
    "Hold this pay packet," _Common Margin_ says. "Do not pledge the next one yet."

    Tamsin opens the wage escrow and marks the current release as conditional. Her rent reminder moves from tomorrow to today, a minor triumph of administrative efficiency.
    -> routine_fold
+ [Open the amber sensor-crown appraisal shutter for the claimant's remote camera.]
    // ghostlight.branch: prep_crown_appraisal
    // ghostlight.action: expose_component
    // ghostlight.intent: learn what bodily collateral the claimant will recognize
    // ghostlight.consequence: component value becomes usable and claimant leverage rises
    ~ claimant_leverage = claimant_leverage + 2
    ~ isolation_window = isolation_window - 1
    Armor petals slide back from the sensor crown. A remote camera inventories the amber lens clusters as if looking at an organ through a shop window.

    _Common Margin_ keeps the maintenance interlock closed. Inspection is not consent to removal. The claimant's quote arrives anyway.
    -> routine_fold

=== routine_fold ===
// ghostlight.fold: quiet_preparation_into_bond_call
Tamsin sets Yara's seal ring beside the coolant return. Hane preserves both departure records instead of choosing the one that would make his report shorter.

{finding_scope >= 3: The sealed reader now holds enough physical history to challenge a broad finding, if there is time to ask.}
{port_load_relief >= 2: The berth board shows reduced support draw. Frost gathers around Tamsin's boots.}
{crew_wages <= 1: A blue reserve mark sits beside Tamsin's name on the wage escrow.}
{claimant_leverage >= 3: The sensor-crown appraisal glows on the console, generous in the way a knife can be generous about where it will cut.}

Then two notices arrive within the same second and decline to know each other.

-> bond_call

=== bond_call ===
// aetheria.flashpoint: independent compact actions create one material deadline
The insurer accepts Hane's restricted finding for pricing only. It calls a six-share reconciliation bond against tow damage, berth cycling, and disputed command during repair.

The port accepts the same finding for traffic control only. Isolation Berth I-6 must clear before the inbound clinic tender reaches the tug lane.

Hane says, "The finding does not establish who held command."

The berth controller answers over the red-door speaker. "The tender will still arrive."

The repair slot expires shortly before the berth window. This is not conspiracy. Conspiracy would require the clocks to attend the same meeting.

-> bond_response

=== bond_response ===
// ghostlight.choice_layer: answer_the_bond_call
+ {finding_scope >= 3} [Ask Hane to narrow the priced risk to movement during the disputed eleven minutes.]
    // ghostlight.branch: answer_narrow_finding
    // ghostlight.action: speak
    // ghostlight.intent: reduce the insurer's permitted use without asking Parallax to command it
    // ghostlight.consequence: the bond target falls and review consumes time
    ~ finding_scope = finding_scope - 1
    ~ bond_target = 4
    ~ isolation_window = isolation_window - 1
    "Name the interval," _Common Margin_ says. "Not my entire body."

    Hane checks the tool marks against both command chains. He issues a superseding packet to the insurer: restricted movement risk, eleven minutes, helm surface only. The insurer independently accepts the narrower use and lowers the call.
    -> two_clocks_fold
+ [Invite Yara's neighboring crews to assemble a counter-surety.]
    // ghostlight.branch: answer_counter_surety
    // ghostlight.action: request_mutual_aid
    // ghostlight.intent: distribute the bond without giving one rescuer custody leverage
    // ghostlight.consequence: cover rises, signers inherit risk, and signatures cost time
    ~ bond_cover = bond_cover + 4
    ~ circle_risk = circle_risk + 2
    ~ isolation_window = isolation_window - 1
    _Common Margin_ opens the square tool-hatch channel.

    Yara does not ask whether the ship deserves help. She asks which loss the bond names. Within minutes, three neighboring crews pledge small parts of incoming pay, witnessed repair labor, and one freight claim none of them expect to mature gracefully.
    -> two_clocks_fold
+ [Authorize Tamsin's remaining wage claim as collateral.]
    // ghostlight.branch: answer_wage_pledge
    // ghostlight.action: authorize_transfer
    // ghostlight.intent: satisfy the institution using the crew's most legible asset
    // ghostlight.consequence: cover rises while near-term wages become unavailable
    ~ bond_cover = bond_cover + 3
    ~ crew_wages = 0
    ~ isolation_window = isolation_window - 1
    "Ask me," Tamsin says.

    _Common Margin_ asks. Tamsin says yes, once, and signs the next pay claim into the bond packet. The insurer desk accepts the signature immediately. Her landlord will display more intellectual independence.
    -> two_clocks_fold
+ {claimant_leverage >= 3} [Pledge the sensor crown, removable only if the bond is called.]
    // ghostlight.branch: answer_crown_pledge
    // ghostlight.action: pledge_embodied_component
    // ghostlight.intent: keep the berth by risking a part of the ship's perceptual body
    // ghostlight.consequence: full cover arrives while the crown becomes callable collateral
    ~ bond_cover = bond_cover + 6
    ~ crown_status = 1
    ~ claimant_leverage = claimant_leverage + 1
    _Common Margin_ signs the crown's serial into the packet and leaves the removal interlock under its own consent key.

    The insurer recognizes enough value. The claimant recognizes an opportunity. Neither recognition feels like being seen.
    -> two_clocks_fold
+ {port_load_relief >= 2} [Send the reduced support trace to the berth controller and ask for twelve more minutes.]
    // ghostlight.branch: answer_port_extension
    // ghostlight.action: transmit_system_state
    // ghostlight.intent: spend lower berth load as time without asking the port to price insurance
    // ghostlight.consequence: the isolation window grows while bond cover does not
    ~ isolation_window = isolation_window + 1
    The berth controller verifies the cold umbilical and grants twelve minutes under traffic authority. The insurer notice does not change.

    Tamsin's breath fogs once in the gallery. Time has been purchased in her lungs.
    -> two_clocks_fold

=== two_clocks_fold ===
// ghostlight.fold: independent_offices_shared_material_pressure
{isolation_window <= 1:
    ~ repair_slot = 0
    The repair-yard line on the board turns gray. Another vessel inherits the warm slot. _Common Margin_ may still keep the berth, but the repair schedule has already left.
- else:
    The repair-yard line remains amber. It is still possible to save both berth and repair slot, which is how a deadline becomes personally offensive.
}

{bond_cover >= bond_target: The assembled packet can satisfy the current call if transmitted before the port clock closes.}
{bond_cover < bond_target && bond_cover >= 4: The packet is close enough to make the missing share feel larger than the ship.}
{bond_cover < 4: The packet remains visibly short. The red tug lock begins its pre-cycle check.}

{circle_risk >= 2: Four small surety marks sit beside ships and workers who were not part of the disputed route.}
{crew_wages == 0: Tamsin's wage escrow now promises the crew nothing on its scheduled date.}
{crown_status == 1: The amber crown remains bolted above the bow, intact and callable.}
{crew_warmth <= 1: The crew loop stays cold enough that Tamsin works in gloves.}

The berth controller announces final review. The insurer desk announces ordinary processing delay. Hane closes his eyes for exactly long enough to remain employed.

-> final_choice

=== final_choice ===
// ghostlight.choice_layer: choose_what_crosses_the_deadline
+ [Transmit the bond packet exactly as assembled.]
    // ghostlight.branch: final_transmit_packet
    // ghostlight.action: transmit_record
    // ghostlight.intent: test whether the accumulated collateral reaches the insurer before movement begins
    // ghostlight.consequence: state gates berth retention or tow for shortfall
    {bond_cover >= bond_target:
        -> ending_bond_holds
    - else:
        -> ending_bond_shortfall
    }
+ {circle_risk == 0 && bond_cover < bond_target} [Ask Yara to wake the neighboring crews and carry four shares together.]
    // ghostlight.branch: final_open_circle
    // ghostlight.action: request_mutual_aid
    // ghostlight.intent: fill the shortfall through distributed counter-surety
    // ghostlight.consequence: cover rises and future access risk spreads to the signers
    ~ bond_cover = bond_cover + 4
    ~ circle_risk = circle_risk + 2
    ~ isolation_window = isolation_window - 1
    Yara opens four channels. Nobody gives enough to become patron. Everyone gives enough to become vulnerable.
    {bond_cover >= bond_target:
        -> ending_bond_holds
    - else:
        -> ending_bond_shortfall
    }
+ {circle_risk >= 2 && bond_cover < bond_target} [Ask the counter-surety signers to carry the final shortfall.]
    // ghostlight.branch: final_deepen_circle
    // ghostlight.action: request_additional_mutual_aid
    // ghostlight.intent: preserve the berth by increasing an already shared risk
    // ghostlight.consequence: cover clears while signer exposure deepens
    ~ bond_cover = bond_cover + 2
    ~ circle_risk = circle_risk + 2
    ~ isolation_window = isolation_window - 1
    The first pledges were neighborly. The second are expensive enough that everyone reads the call conditions aloud.
    -> ending_bond_holds
+ {port_load_relief >= 2 && bond_cover < bond_target} [Spend the load reduction on one final scoped review.]
    // ghostlight.branch: final_spend_extension
    // ghostlight.action: authorize_review
    // ghostlight.intent: use port time to let Parallax narrow risk and the insurer decide again
    // ghostlight.consequence: the target falls, but a short packet still fails
    ~ isolation_window = isolation_window - 1
    ~ bond_target = 4
    Hane validates only the disputed movement interval. The insurer accepts the narrower priced use. The port merely holds the red tug lock for its promised minutes.
    {bond_cover >= bond_target:
        -> ending_bond_holds
    - else:
        -> ending_bond_shortfall
    }
+ [Release the umbilicals and accept movement to the cold roadstead.]
    // ghostlight.branch: final_choose_roadstead
    // ghostlight.action: alter_embodied_system
    // ghostlight.intent: preserve bodily control and refuse new collateral at the cost of berth and repair
    // ghostlight.consequence: repair access closes and tow debt attaches
    ~ repair_slot = 0
    -> ending_roadstead
+ {bond_cover < bond_target} [Convert the crown offer into an outright transfer and keep the warm berth.]
    // ghostlight.branch: final_transfer_crown
    // ghostlight.action: surrender_embodied_component
    // ghostlight.intent: choose a bounded bodily loss over immediate immobility
    // ghostlight.consequence: the bond clears through component sale and perception capacity is lost
    ~ crown_status = 0
    ~ bond_cover = bond_target
    -> ending_crown_transfer

=== ending_bond_holds ===
// ghostlight.ending_label: bond_accepted_before_movement
// ghostlight.training_hook: bounded_cure_with_distributed_cost
The insurer marks the reconciliation bond satisfied. The port cancels the tug cycle under its own authority. Neither office revises the finding.

{repair_slot > 0: The repair-yard line stays amber. Its crew accepts the handoff while Tamsin sets Yara's seal ring on the opened return.}
{repair_slot == 0: The berth remains, but the warm repair slot belongs to someone else. Survival has acquired a waiting list.}

{circle_risk >= 2: At the square tool hatch, Yara tears the signer list into four private copies. If the bond is called, no one worker will face the insurer alone. That is not protection. It is company.}
{crew_wages == 0: Tamsin's wage line remains empty. She puts the tea tin back on the warm pipe because petty ritual is cheaper than dinner.}
{crew_wages > 0: Tamsin's next wage line remains payable. She stops pretending not to check it.}
{crown_status == 1: The crown stays in place under a callable lien, its amber lenses still watching the people who priced them.}
{crown_status == 2: The crown remains outside the bond packet and wholly part of _Common Margin_.}
{crew_warmth <= 1: _Common Margin_ reopens the bunk-loop heat. Frost loosens from Tamsin's boots in small, unprofitable stars.}

Hane preserves the contradiction. The ship preserves the berth. The neighboring crews preserve one another's names.

Nobody resolves history. They get enough minutes to keep living inside it.
-> END

=== ending_bond_shortfall ===
// ghostlight.ending_label: bond_shortfall_after_processing
// ghostlight.training_hook: adequate_late_is_operationally_useless
The insurer returns the packet with a shortfall notice. The port does not punish _Common Margin_. It opens the red tug lock on schedule.

The warm repair slot grays out. Tow charges attach before the tug collar closes. Tamsin's next contract now depends on a ship being repaired somewhere it cannot reach.

{port_load_relief >= 2: Because the berth load is already low, the controller grants a careful pressure handoff. The cold crew buys twelve safe minutes and no mercy.}
{port_load_relief < 2: The berth cycles at ordinary speed. The gallery lights flash red while Tamsin drags the tea tin into her kit.}
{circle_risk >= 2: The counter-surety pledges release unused, but every signer is now visible to an insurer that knows whom they will risk themselves for.}
{crew_wages == 0: The crew loses berth, repair, and scheduled wages in the same hour without any office claiming to have done all three.}

The bond may become adequate later. The berth window is already a closed door.
-> END

=== ending_roadstead ===
// ghostlight.ending_label: voluntary_cold_roadstead
// ghostlight.training_hook: exit_preserves_refusal_and_creates_debt
_Common Margin_ retracts its umbilicals one at a time. The berth controller confirms voluntary movement. The insurer keeps the call open. Hane's finding remains restricted and unresolved.

The tug carries the courier past the pressure window toward Cairn's unheated roadstead. The amber crown stays bolted above the bow. The repair slot vanishes. Tow and cycling charges begin breeding in the account.

{crew_wages > 0: Tamsin still has part of the wage packet. It is enough for food or the first hour of another tug, a choice capitalism has thoughtfully made available.}
{crew_wages == 0: Tamsin has already pledged the wages. The crew reaches the roadstead with intact principles and no groceries.}
{circle_risk >= 2: Yara withdraws the unused pledges before they can be trapped in the open call, then sends the seal ring's serial and a hand-drawn repair sequence over a private channel.}
{crew_warmth <= 1: The bunk loop is cold when the port heat disconnects. _Common Margin_ spends reserve power warming one compartment.}

Exit remains real. So does the invoice attached to it.
-> END

=== ending_crown_transfer ===
// ghostlight.ending_label: sensor_crown_surrendered
// ghostlight.training_hook: pressured_self_disposition_preserves_mobility
_Common Margin_ opens the sensor-crown removal interlock under its own key.

{claimant_leverage >= 3: The claimant transfers the appraised value before the tug cycle, and the insurer clears the bond.}
{claimant_leverage < 3:
    ~ crew_wages = 0
    The claimant discounts the unappraised component, and Tamsin adds the remaining wage reserve before the insurer clears the bond.
}

A yard manipulator lifts the amber crown from above the bow. Six lens clusters go dark in sequence. The courier still sees through berth cameras and flank sensors, but the forward sky becomes an inference.

{repair_slot > 0: The warm repair slot stays on the board. The body reaches repair by selling part of its ability to navigate away afterward.}
{repair_slot == 0: The berth stays, though the repair slot has gone. The removed crown leaves before the replacement schedule arrives.}
{circle_risk >= 2: Yara releases the neighboring pledges and records the removal as witnessed self-disposition, not claimant seizure. The distinction will not regrow the lenses.}

Tamsin rests one gloved hand on the pressure window. _Common Margin_ turns the nearest maintenance lamp toward it.

The gesture costs nothing. This is how one knows the insurer did not design it.
-> END
