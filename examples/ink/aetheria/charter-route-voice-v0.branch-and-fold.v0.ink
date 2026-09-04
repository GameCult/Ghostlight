// ghostlight.artifact_id: charter_route_voice_branch_fold_v0
// ghostlight.fixture_id: charter-route-voice-v0
// ghostlight.scene_id: charter-route-voice-v0.ganymede-mandate-desk
// ghostlight.final_ink_path: examples/ink/aetheria/charter-route-voice-v0.branch-and-fold.v0.ink

VAR source_integrity = 1
VAR scope_clarity = 1
VAR navigator_trust = 1
VAR berth_minutes = 6
VAR carrier_pressure = 1
VAR bond_pressure = 1
VAR alternate_ready = 0
VAR union_copy = 0
VAR safety_hold_recorded = 0
VAR clean_translation_relied = 0

-> start

=== start ===
// ghostlight.scene: ganymede_registry_annex_open
// ghostlight.visual_scene_id: ganymede_registry_establishing
In 2478, Ganymede Dock Registry Annex B begins the morning by asking a dolphin whether Sen Ochoa's tea is seaworthy.

The annex was a convoy dispatch bay before the new Route Compact taught it law. A long orientation basin runs beneath the portside windows, deep enough for an uplifted cetacean to turn and accelerate. Across a waist-high pressure wall, the dryside clerk bench carries a source receiver, a translation display, a union duplicate press, and three empty cup clamps. The route board hangs above both sides as light for eyes, low sound for ears, pressure ripples through water, and raised pins along the bench.

Talu-at-Four-Returns sends the morning test pulse. Sen's covered cup rocks in its clamp.

"Hazard confirmed," Talu says through the translated speaker.

"The cup has no standing," Sen says.

This joke has survived eleven shifts, which makes it more established than half the Compact.

-> routine

=== routine ===
// ghostlight.scene: route_voice_routine
// ghostlight.visual_scene_id: ganymede_mandate_routine
Sen is the dock labor syndicate's mandate recorder. The job is narrow: witness the source channel and declared scope, preserve dissent, and refuse a signature that asks a translation to impersonate its speaker. Sen does not choose navigator stewards and cannot authorize a route.

Talu waits in the basin, slate-blue back breaking the water near the pressure wall. Their navigator watch has named them route steward for the _Common Wake_ convoy from Ganymede to the Callisto transfer corridor. Pel-of-Nine-Echoes is the declared alternate at another pool station.

Rem Saye, Lightsail Express dispatcher, needs food cultures and scrubber cartridges moving before the departure throat closes. Vela Quine is both the insurer's bond clerk and the required counterparty witness; she must attest the same source and scope separately before her office can keep the convoy's coverage price from tripling. Both stand on the dry deck where urgency has chairs and the navigator does not.

Six berth minutes remain. The old translation buffer has recently learned the phrase ALL NECESSARY OPERATIONS and is eager to use it.

Sen can prepare one protection before Talu lodges the mandate.

-> preparation_choice

=== preparation_choice ===
// ghostlight.choice_layer: mandate_preparation
+ [Press an empty union duplicate receipt and bind it to Talu's raw source channel.]
    // ghostlight.branch: prepare_union_copy
    // ghostlight.action: use_object
    // ghostlight.intent: preserve_source_signal_outside_carrier_and_insurer_custody
    ~ union_copy = 1
    ~ source_integrity = source_integrity + 2
    ~ navigator_trust = navigator_trust + 1
    ~ berth_minutes = berth_minutes - 1
    // ghostlight.visual_scene_id: ganymede_preparation_desk
    Sen slides a blank ceramic receipt wafer into the union press. The machine punches the docket time, source-channel key, and one inelegant syndicate mark into its edge.

    Talu rolls one eye toward the press. "Your machine sounds injured."

    "It is union equipment. Injury is how it authenticates."

    Vela notes the spent minute. Rem notes that Vela noted it.
    // ghostlight.consequence: independent_source_copy_up_time_down
    -> mandate_arrival
+ [Call Pel's pool station and test the alternate's succession channel.]
    // ghostlight.branch: prepare_alternate_channel
    // ghostlight.action: request_signal
    // ghostlight.intent: make_succession_callable_without_transferring_voice_to_employer
    ~ alternate_ready = 1
    ~ scope_clarity = scope_clarity + 1
    ~ berth_minutes = berth_minutes - 1
    // ghostlight.visual_scene_id: ganymede_preparation_desk
    Pel answers with a low pulse from the remote pool. The board renders the same route segment and expiry Talu has declared, then adds ALTERNATE: READY.

    Rem says, "Excellent. Redundancy."

    Talu answers, "Succession. Redundancy is what owners call a replacement before the body has left."

    Sen records the distinction. The form has a field for the first word and none for the second.
    // ghostlight.consequence: alternate_ready_scope_up_time_down
    -> mandate_arrival
+ [Accept Vela's clean translation preview so the bond clock starts now.]
    // ghostlight.branch: prepare_clean_preview
    // ghostlight.action: accept_interface_default
    // ghostlight.intent: preserve_berth_and_bond_margin_through_standard_language
    ~ clean_translation_relied = 1
    ~ source_integrity = source_integrity - 1
    ~ scope_clarity = scope_clarity - 1
    ~ bond_pressure = bond_pressure - 1
    // ghostlight.visual_scene_id: ganymede_preparation_desk
    Vela loads the insurer's approved wording. It is short, grammatical, and broad enough to shelter a fleet.

    The bond clock begins. Rem's shoulders loosen. Talu's dorsal line does not change, which Sen has learned is not the same as agreement.

    "Preview only," Sen says.

    The display stores the sentence under ACTIVE TEXT.
    // ghostlight.consequence: time_and_bond_margin_up_source_and_scope_down
    -> mandate_arrival
+ [Ask Talu to state the mandate's exclusions through the source channel before the translation starts.]
    // ghostlight.branch: prepare_scope_boundaries
    // ghostlight.action: ask_question
    // ghostlight.intent: make_non_authorities_and_expiry_part_of_the_source_record
    ~ scope_clarity = scope_clarity + 2
    ~ navigator_trust = navigator_trust + 1
    ~ carrier_pressure = carrier_pressure + 1
    // ghostlight.visual_scene_id: ganymede_preparation_desk
    Talu sends four slow pressure pulses. Sen reads their paired notation aloud: route commitment, rescue diversion, one immediate safety hold, convoy arbitration.

    Then the exclusions: no wage waiver, no injury settlement, no habitat-support waiver, no absent council.

    Rem looks at the berth clock. "We were not planning to waive a habitat."

    "Then the exclusion costs you nothing," Sen says.
    // ghostlight.consequence: scope_and_trust_up_carrier_pressure_up
    -> mandate_arrival

=== mandate_arrival ===
// ghostlight.fold: prepared_mandate_state
// ghostlight.visual_scene_id: ganymede_mandate_arrival
Talu enters the mandate.

The source signal travels first: named navigator watch, _Common Wake_, Ganymede to Callisto transfer corridor, two convoy watches, Pel as alternate, expiry at second-watch relief. The route board carries it as pale light, low clicks, four pressure ridges, and raised pins.

{union_copy == 1: The union receipt wafer records the source signal before the translation display touches it.}
{alternate_ready == 1: Pel's remote channel remains green beside the succession field.}
{scope_clarity >= 3: The four powers and four exclusions stand in separate columns.}
{scope_clarity <= 0: The scope field contains a clean blank large enough for someone else's certainty.}
{clean_translation_relied == 1: Vela's approved sentence is already waiting under ACTIVE TEXT.}
{berth_minutes <= 5: The berth clock has turned amber.}

Then the old buffer translates.

-> translation_failure

=== translation_failure ===
// ghostlight.scene: source_translation_divergence
// ghostlight.visual_scene_id: ganymede_translation_failure
The source signal ends at two convoy watches.

The dry display writes: TALU-AT-FOUR-RETURNS ACCEPTS ALL NECESSARY ROUTES AND DIVERSIONS FOR COMMON WAKE OPERATIONS.

The exclusions vanish. Pel becomes a backup operator. Expiry becomes a review date.

The pressure channel still holds the narrower act. The clean sentence is easier to insure.

~ source_integrity = source_integrity - 1
~ bond_pressure = bond_pressure + 1

Vela points to the clock. "I can price the displayed mandate."

Talu sends one sharp click the speaker declines to translate.

Sen owns only the next witness act.

-> divergence_choice

=== divergence_choice ===
// ghostlight.choice_layer: translation_divergence_response
+ [Replay the source signal through water, light, and the raised rail.]
    // ghostlight.branch: respond_source_replay
    // ghostlight.action: operate_interface
    // ghostlight.intent: reestablish_shared_source_before_witnessing
    ~ source_integrity = source_integrity + 2
    ~ scope_clarity = scope_clarity + 1
    ~ berth_minutes = berth_minutes - 2
    // ghostlight.visual_scene_id: ganymede_divergence_response
    Sen drags the source packet back to the board. Talu turns broadside to the pressure ridges while Sen walks the raised pins with both hands. Rem reads the light track. Vela watches the bond clock consume two minutes in an admirably neutral font.

    Four powers. Four exclusions. One expiry.

    The clean translation remains visible beside the source, suddenly less clean.
    // ghostlight.consequence: source_and_scope_up_time_down
    -> departure_alert
+ [Mark the translation nonauthoritative and witness only the named route segment and expiry.]
    // ghostlight.branch: respond_narrow_witness
    // ghostlight.action: annotate_record
    // ghostlight.intent: preserve_uncontested_scope_without_claiming_full_translation
    ~ scope_clarity = scope_clarity + 2
    ~ source_integrity = source_integrity + 1
    ~ carrier_pressure = carrier_pressure + 2
    ~ berth_minutes = berth_minutes - 1
    // ghostlight.visual_scene_id: ganymede_divergence_response
    Sen brackets GANYMEDE-CALLISTO TRANSFER and SECOND-WATCH RELIEF on the dry display. Everything else turns amber: not rejected, not accepted, not allowed to borrow clarity from the font.

    Rem says, "That may not be enough mandate to launch."

    "It is exactly enough mandate to be exactly itself," Talu says.
    // ghostlight.consequence: narrow_scope_preserved_carrier_pressure_up
    -> departure_alert
+ [Witness the approved translation and attach Talu's source signal as dissent.]
    // ghostlight.branch: respond_clean_translation
    // ghostlight.action: sign_record
    // ghostlight.intent: keep_convoy_and_bond_moving_despite_scope_divergence
    ~ clean_translation_relied = 2
    ~ source_integrity = source_integrity - 1
    ~ scope_clarity = scope_clarity - 1
    ~ carrier_pressure = carrier_pressure - 1
    ~ bond_pressure = bond_pressure - 1
    // ghostlight.visual_scene_id: ganymede_divergence_response
    Sen touches the witness square.

    The display accepts the broad sentence and appends the source packet under DISSENT / SUPPORTING MATERIAL. Vela's bond field clears. Talu's actual scope survives in full, safely below the part that governs.

    Nobody lies. The machine has developed more economical methods.
    // ghostlight.consequence: schedule_relief_bought_with_governing_scope_capture
    -> departure_alert
+ {alternate_ready >= 1} [Ask Pel to answer the succession field through the tested alternate channel.]
    // ghostlight.branch: respond_alternate_answer
    // ghostlight.action: request_signal
    // ghostlight.intent: test_whether_predeclared_succession_can_restore_a_valid_voice
    ~ alternate_ready = 2
    ~ source_integrity = source_integrity + 1
    ~ scope_clarity = scope_clarity + 1
    ~ berth_minutes = berth_minutes - 1
    // ghostlight.visual_scene_id: ganymede_divergence_response
    Pel answers from the remote pool with the recorded route segment, powers, exclusions, and expiry. Their signal is independent and slower than Talu's, not because it means less but because distance also gets a vote.

    Talu remains steward. Pel is now callable if succession becomes necessary. The board does not promote readiness into replacement.
    // ghostlight.consequence: alternate_callable_without_displacing_steward
    -> departure_alert

=== departure_alert ===
// ghostlight.fold: mandate_dispute_meets_immediate_hazard
// ghostlight.visual_scene_id: ganymede_departure_alert
The departure throat turns red.

An ore lighter has lost lateral thrust outside the dock collar. Its projected drift crosses _Common Wake_'s launch lane in three minutes. The cargo can wait. The collision cannot become more courteous.

Talu strikes the wet-side safety paddle and sends the Compact hold pattern: stop departure, clear one lane, one decision cycle only.

{source_integrity >= 3: Every channel carries the hold and its limit.}
{source_integrity <= 0: The source signal survives as a rough pulse beneath the clean translation already governing the display.}
{scope_clarity >= 3: ONE DECISION CYCLE appears beside the red route.}
{clean_translation_relied >= 2: The display also suggests the hold proves Talu has accepted all necessary operations.}
{alternate_ready >= 2: Pel's green channel waits beside the succession field, ready but not invoked.}

Rem says, "The hold proves route authority."

Vela says, "The hold is insurable if it proves route authority."

Talu says, "The lighter proves a lighter."

-> safety_choice

=== safety_choice ===
// ghostlight.choice_layer: immediate_safety_hold
+ [Stamp the hold as one decision cycle and close the launch lane.]
    // ghostlight.branch: record_bounded_safety_hold
    // ghostlight.action: touch_interface
    // ghostlight.intent: preserve_immediate_safety_without_enlarging_mandate
    ~ safety_hold_recorded = 1
    ~ source_integrity = source_integrity + 1
    ~ navigator_trust = navigator_trust + 1
    ~ berth_minutes = berth_minutes - 2
    // ghostlight.visual_scene_id: ganymede_safety_hold
    Sen presses the red-rimmed hold square. The launch lane closes. ONE DECISION CYCLE locks beside Talu's source signal.

    The lighter drifts through the empty throat slowly enough to look harmless to anyone who has not priced bent docking metal.

    Two more berth minutes disappear. Their sacrifice receives no memorial.
    // ghostlight.consequence: collision_avoided_hold_bounded_time_spent
    -> final_record
+ [Attach the hold to the approved translation as proof of broad authority.]
    // ghostlight.branch: use_hold_as_broad_proof
    // ghostlight.action: annotate_record
    // ghostlight.intent: satisfy_carrier_and_insurer_with_one_operational_act
    ~ safety_hold_recorded = 1
    ~ clean_translation_relied = 3
    ~ scope_clarity = scope_clarity - 1
    ~ carrier_pressure = carrier_pressure - 1
    ~ bond_pressure = bond_pressure - 1
    // ghostlight.visual_scene_id: ganymede_safety_hold
    Sen links the hold to the approved mandate. The lane closes. The lighter passes safely. The display labels the act CONFIRMING EXERCISE OF ROUTE AUTHORITY.

    Talu's correction appears beneath it before the lighter clears: SAFETY HOLD ONLY.

    The correction remains attached. So does the authorization above it.
    // ghostlight.consequence: safety_preserved_but_emergency_action_enlarges_governing_voice
    -> final_record
+ [Record the hazard warning but refuse to witness the hold while the mandate is disputed.]
    // ghostlight.branch: refuse_hold_witness
    // ghostlight.action: withhold_signature
    // ghostlight.intent: avoid_using_emergency_action_to_settle_contested_voice
    ~ safety_hold_recorded = 0
    ~ navigator_trust = navigator_trust - 1
    ~ carrier_pressure = carrier_pressure + 2
    ~ bond_pressure = bond_pressure + 1
    ~ berth_minutes = berth_minutes - 1
    // ghostlight.visual_scene_id: ganymede_safety_hold
    Sen records the drift warning and leaves the hold square blank.

    Dock traffic control closes the lane under its own collision rule six seconds later. The ships stop. Talu's act does not enter the Compact ledger; the same safe motion appears as a human office correcting an uncertain uplift signal.

    Talu surfaces beside Sen's bench. "You preserved the dispute."

    It is not praise.
    // ghostlight.consequence: physical_safety_preserved_voice_erased_from_record
    -> final_record

=== final_record ===
// ghostlight.fold: bounded_hold_into_mandate_disposition
// ghostlight.visual_scene_id: ganymede_final_record
The ore lighter clears the throat. The berth clock is either alive, wounded, or already a historical document.

{safety_hold_recorded == 1: Talu's safety hold remains visible beside the red lane trace.}
{safety_hold_recorded == 0: The lane closure belongs to traffic control; Talu's earlier pattern survives only in the raw source packet.}
{union_copy == 1: The ceramic union receipt holds a second copy beyond carrier and insurer custody.}
{alternate_ready >= 2: Pel's answered succession channel remains green and scoped.}
{clean_translation_relied >= 2: The approved sentence still governs, with Talu's narrower source attached below it.}
{carrier_pressure >= 3: Rem has stopped looking at the route and started looking only at Sen's witness square.}
{bond_pressure >= 2: Vela's bond field has turned from amber to a very expensive blue.}
{navigator_trust >= 3: Talu waits close to the pressure wall, giving Sen one more chance to make the written voice resemble the living one.}
{navigator_trust <= 0: Talu moves to the far turn of the basin. The interface still works across the distance. The relationship does not have to.}

The Compact offers no clean ending, only named record owners. Sen owns the union witness square. Vela owns the counterparty square. Sen must decide what record to put before her and what the annex will tell the next dock.

-> final_choice

=== final_choice ===
// ghostlight.choice_layer: route_voice_disposition
+ [Certify Talu's narrow mandate from the source record.]
    // ghostlight.branch: decide_narrow_mandate
    // ghostlight.action: sign_record
    // ghostlight.intent: recognize_the_declared_steward_without_enlarging_scope
    {source_integrity >= 2 && scope_clarity >= 2:
        -> ending_narrow_voice
    - else:
        -> ending_narrow_cost
    }
+ {alternate_ready >= 2} [Ask whether Talu withdraws for this interval; if they do, record Pel's answer as the declared alternate.]
    // ghostlight.branch: decide_valid_succession
    // ghostlight.action: record_succession
    // ghostlight.intent: use_predeclared_succession_without_employer_inheritance
    {source_integrity >= 2 && scope_clarity >= 2:
        -> ending_alternate_voice
    - else:
        -> ending_alternate_cost
    }
+ [Leave the mandate open and hold _Common Wake_ at berth for another source review.]
    // ghostlight.branch: decide_honest_delay
    // ghostlight.action: withhold_signature
    // ghostlight.intent: refuse_false_finality_and_export_a_reviewable_open_record
    -> ending_open_record
+ [Certify the approved translation so cargo and coverage move now.]
    // ghostlight.branch: decide_clean_authorization
    // ghostlight.action: sign_record
    // ghostlight.intent: preserve_immediate_logistics_by_allowing_translation_to_govern
    -> ending_clean_capture

=== ending_narrow_voice ===
// ghostlight.ending_label: narrow_route_voice_witnessed
// ghostlight.training_hook: representation_is_scoped_source_and_expiry
// ghostlight.visual_scene_id: ganymede_ending_narrow
Sen certifies Talu-at-Four-Returns as route steward for one segment, two watches, four named acts, four named exclusions. Vela checks the same source channel, countersigns that scope, and only then reprices the bond.

{safety_hold_recorded == 1: The safety hold stays beside the mandate as a bounded emergency act, not evidence of wider consent.}
{safety_hold_recorded == 0: Talu's uncredited hold pattern stays in the source record beside traffic control's credited closure. The omission is visible, which is weaker than repair and stronger than erasure.}

_Common Wake_ loses its first launch slot. Rem begins restacking food cultures around the next departure window. Vela reprices the bond against a smaller authority than she wanted and a cleaner source than she expected.

Talu sends the morning test pulse again. Sen's cup rocks.

"Still hazardous," Talu says.

This time Sen enters the cup under LOCAL MATTERS / UNRESOLVED. The new law survives contact with the old joke by becoming slightly less dignified and considerably more accurate.
-> END

=== ending_narrow_cost ===
// ghostlight.ending_label: narrow_voice_claimed_from_thin_source
// ghostlight.training_hook: good_scope_language_cannot_replace_missing_evidence
// ghostlight.visual_scene_id: ganymede_ending_narrow
Sen certifies the narrow words.

The words are better than the broad translation. The surviving source is not strong enough to prove they are Talu's complete mandate. Vela leaves her counterparty square empty and marks the convoy for source review. Rem cannot launch on Sen's signature alone; cargo ages without ideology.

Talu does not contest the sentence. Sen has learned enough this morning to know that silence is not a cure.

{union_copy == 1: The union receipt preserves what source survived, giving the next challenge something harder than Sen's confidence.}
{union_copy == 0: The carrier and insurer copies become the only records that travel.}
-> END

=== ending_alternate_voice ===
// ghostlight.ending_label: predeclared_alternate_succeeds
// ghostlight.training_hook: succession_requires_answer_not_availability
// ghostlight.visual_scene_id: ganymede_ending_alternate
Talu withdraws for this interval through the source channel. Pel answers through theirs.

The board changes STEWARD only after both acts are recorded. Scope and expiry do not move. Sen witnesses the sequence; Vela countersigns the matching source and scope. Rem gets a voice authorized to finish the route decision. Vela gets a bondable record. Nobody gets to call Pel a replacement asset without producing a different, uglier document.

{safety_hold_recorded == 1: Talu's completed safety hold remains Talu's act; Pel inherits no credit and no blame for it.}
{berth_minutes <= 1: _Common Wake_ still misses the slot. Valid succession is not a time machine.}
{berth_minutes > 1: _Common Wake_ clears on the next lane release with less margin than Rem will later describe.}

Sen gives the duplicate receipt to the next union runner. Small institutions survive by making authority tedious to steal.
-> END

=== ending_alternate_cost ===
// ghostlight.ending_label: alternate_answer_inside_contested_scope
// ghostlight.training_hook: succession_does_not_repair_a_broken_mandate
// ghostlight.visual_scene_id: ganymede_ending_alternate
Pel answers. Sen records succession. The alternate channel is valid; the mandate it inherits is still disputed.

The convoy receives a steward and no trustworthy account of what that steward may bind. Vela leaves the counterparty square open and prices the delay. Rem calls it interrupted operational continuity. Talu circles once at the far end of the basin, where no dryside expression can be mistaken for theirs.

Succession has prevented employer inheritance. It has not repaired the source. A sound procedure can keep one theft from happening while another waits politely in the scope field.
-> END

=== ending_open_record ===
// ghostlight.ending_label: mandate_open_convoy_held
// ghostlight.training_hook: honest_redress_exports_material_cost
// ghostlight.visual_scene_id: ganymede_ending_delay
Sen leaves the witness square empty and records the exact dispute.

_Common Wake_ remains at berth. Its cultures move into borrowed cooling. Scrubber cartridges miss the first Callisto transfer. Vela freezes the price instead of approving it; Rem begins calculating wages against delay.

{union_copy == 1: The union receipt leaves through a different door with a dock runner, preserving source, translation, hold, and challenge outside the two offices paying for the argument.}
{union_copy == 0: Sen presses a duplicate after the decision. It records the open dispute, but not every source edge lost before the press began.}

Dock workers find two spare cooling sockets and a pot of actual tea for the stranded watch. This does not become a movement. It gets the cultures and the people through the next six hours, which is what hope looks like before anyone prints stationery.
-> END

=== ending_clean_capture ===
// ghostlight.ending_label: clean_translation_governs
// ghostlight.training_hook: correction_survives_beneath_enlarged_authority
// ghostlight.visual_scene_id: ganymede_ending_capture
Sen certifies the approved translation. Vela countersigns its displayed scope, despite the narrower source still visible below it.

The bond clears. _Common Wake_ receives the next launch lane. Rem sends the food cultures and scrubber cartridges outward under authority everyone can read and Talu did not grant.

{safety_hold_recorded == 1: The successful safety hold appears as confirming performance.}
{safety_hold_recorded == 0: Traffic control's closure appears as external corroboration of the route warning.}
{union_copy == 1: The union receipt preserves Talu's narrower source for a later challenge.}
{union_copy == 0: Talu's correction travels only as an attachment to the authorization it fails to stop.}
{bond_pressure <= 0: Vela's screen returns to a calm green. The institution has found the exact price at which disagreement becomes supporting material.}

Talu sends no farewell pulse. Sen's cup stays still in its clamp, finally and completely seaworthy.
-> END
