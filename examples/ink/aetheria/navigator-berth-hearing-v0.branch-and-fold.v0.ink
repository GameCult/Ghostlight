// ghostlight.artifact_id: navigator_berth_hearing_branch_fold_v0
// ghostlight.fixture_id: navigator-berth-hearing-v0
// ghostlight.scene_id: navigator-berth-hearing-v0.brineglass-berth-claim
// ghostlight.final_ink_path: examples/ink/aetheria/navigator-berth-hearing-v0.branch-and-fold.v0.ink

VAR record_confidence = 1
VAR witness_access = 1
VAR carrier_trust = 1
VAR berth_minutes = 4
VAR evidence_custody = 1
VAR apprentice_understanding = 0
VAR clock_paused = 0
VAR requester_recorded = 0
VAR manual_translation = 0
VAR embodied_inference = 0
VAR carrier_account_used = 0

-> start

=== start ===
// ghostlight.scene: brineglass_hearing_room_open
// ghostlight.visual_scene_id: brineglass_room_establishing
Brineglass Waystation begins each berth hearing by testing whether everyone can hear the furniture.

The chamber is a long transfer room divided by a waist-high pressure wall. On Sere-in-Return's side, the orientation pool darkens from turquoise shallows to a deep blue working lane. On the other, a grated dry gallery holds six folding seats, a tactile rail, and the clerk's console. Above both hangs the route board: light tracks for eyes, bass notes for ears, pressure pulses through the pool, and raised notation traveling beneath dryside fingertips.

Brineglass calls that redundancy. Richer stations call it the minimum. Brineglass cannot afford either opinion.

For this routine docket, the Waystation Council has delegated Sere one narrow authority: admit the trace for local berth allocation, reject it into standby, or keep the claim open. The carrier's bond and corridor-wide standing belong to later review.

-> routine

=== routine ===
// ghostlight.scene: routine_docket_preparation
// ghostlight.visual_scene_id: brineglass_docket_routine
Sere circles once through the working lane while Mina Osei, the dryside docket clerk, feeds a witness slate into the sealed evidence drawer beneath the board. The drawer shuts with the solemn click of a machine that has never once been asked to pay a berth fee.

Today's claimant is the Lightsail Express carrier _Dawn Ledger_. It crossed a micrometeoroid wake, broke convoy formation, spent rescue propellant on a damaged tender, and arrived late with food and clinic filters. Captain Halden Roe wants the delay entered as a valid hazard diversion so his bond survives. The next convoy wants this berth in nine minutes; the board allows four of them for the hearing. Everybody is correct in a way that has begun charging rent.

Halden stands beside the dry rail in a creased blue carrier coat. His apprentice, Toma Pell, holds the duplicate trace as if the slate might bolt. Mina has already written the hearing number twice because the console rejected the first version for containing a slash it had supplied itself.

Sere rolls one eye toward the board. The low-frequency test tone reaches the left side of his jaw as a clean vibration. The higher acoustic detail breaks into old scar-noise. That is why the pressure-pulse channel matters.

-> preparation_choice

=== preparation_choice ===
// ghostlight.choice_layer: hearing_preparation
+ [Spend one berth minute testing every route-board channel against the sealed trace.]
    // ghostlight.branch: prepare_full_channel_test
    // ghostlight.action: inspect_interface
    // ghostlight.intent: establish_cross_channel_confidence
    ~ record_confidence = record_confidence + 2
    ~ witness_access = witness_access + 1
    ~ berth_minutes = berth_minutes - 1
    // ghostlight.visual_scene_id: brineglass_channel_test
    Sere noses the wet-side test paddle. White route light runs capward across the board. A bass note follows. Three pressure ridges cross the pool and meet his flank in exact sequence.

    Mina walks two fingers over the raised dryside notation. "All four agree. This is the point at which the station develops confidence and the queue develops opinions."

    Halden looks at the berth clock. Toma looks at the board. Only one of them has learned anything.
    // ghostlight.consequence: confidence_and_access_up_time_down
    -> evidence_replay
+ [Accept Halden's practiced summary before replaying the trace.]
    // ghostlight.branch: prepare_carrier_summary
    // ghostlight.action: listen
    // ghostlight.intent: preserve_schedule_margin_through_claimant_context
    ~ carrier_trust = carrier_trust + 1
    ~ berth_minutes = berth_minutes + 1
    ~ record_confidence = record_confidence - 1
    // ghostlight.visual_scene_id: brineglass_carrier_summary
    Halden gives the version built for insurers: wake, deviation, tender contact, recovery burn, late arrival. Each noun is scrubbed and stacked.

    Sere hears no lie. He also hears no frightened tender crew and no moment when a route choice became a rescue obligation. A good summary is a crate with the dangerous corners planed off.

    "Thank you," Sere says through the pool speaker. "Now we will inspect the corners."
    Halden's shoulders loosen by half a uniform seam.
    // ghostlight.consequence: carrier_trust_and_time_up_independent_confidence_down
    -> evidence_replay
+ [Ask Toma to mirror the pressure trace on the dryside tactile rail.]
    // ghostlight.branch: prepare_apprentice_mirror
    // ghostlight.action: set_condition
    // ghostlight.intent: make_cross_body_reading_part_of_routine
    ~ apprentice_understanding = apprentice_understanding + 2
    ~ witness_access = witness_access + 1
    ~ berth_minutes = berth_minutes - 1
    // ghostlight.visual_scene_id: brineglass_apprentice_mirror
    Toma puts both hands on the rail. Sere sends the test pattern from the wet paddle: one long ridge, two short, one long.

    Toma misses the second pulse.

    "Again," Sere says.

    Halden glances at the clock. Mina does not. On the third pass Toma's fingers move with the pool wave. Their grin is brief, private, and much too pleased for a hearing.
    // ghostlight.consequence: apprentice_and_access_up_time_down
    -> evidence_replay
+ [Inspect the sealed drawer and match its custody marks to the board before anyone speaks.]
    // ghostlight.branch: prepare_evidence_custody
    // ghostlight.action: inspect_object
    // ghostlight.intent: strengthen_chain_of_custody_before_claimant_pressure
    ~ evidence_custody = evidence_custody + 2
    ~ record_confidence = record_confidence + 1
    // ghostlight.visual_scene_id: brineglass_evidence_drawer
    Sere sinks until one eye is level with the evidence drawer's wet window. The slate seal glows violet. The board answers with the same three custody marks: carrier, tender, rescue ledger.

    Mina taps each mark from the dry gallery. Halden waits. Toma stops trying to hold the duplicate slate like an innocent object.

    Nobody enjoys custody checks. That is one reason they remain useful.
    // ghostlight.consequence: custody_and_confidence_up
    -> evidence_replay

=== evidence_replay ===
// ghostlight.fold: prepared_hearing_state
// ghostlight.visual_scene_id: brineglass_trace_replay
Mina starts the disputed hazard trace.

Light draws the convoy as six white lines. Sound gives each hull a note. Raised notation moves under Toma's hands. In the pool, pressure pulses build the route as a landscape against Sere's skin: smooth transit, wake warning, formation break, tender tumble, rescue turn.

{record_confidence >= 3: Every channel begins in agreement. Sere has enough redundancy to notice a disagreement rather than merely suffer one.}
{record_confidence <= 0: Halden's summary sits in Sere's mind where an independent baseline should have been.}
{apprentice_understanding >= 2: Toma follows the trace on the rail, lips moving with the pulse count.}
{evidence_custody >= 3: The three violet custody marks remain visible beneath the route lines.}
{berth_minutes <= 3: The berth clock has already turned amber.}

The critical segment approaches: the instant when _Dawn Ledger_ left formation. Before the wake warning, Halden pays the penalty. After it, the rescue diversion belongs in the ledger and the bond survives.

-> channel_failure

=== channel_failure ===
// ghostlight.scene: pressure_channel_failure
// ghostlight.visual_scene_id: brineglass_pressure_failure
The light line bends.

The dry rail ticks.

The pool says nothing.

The missing pressure ridge should carry the timing distinction Sere cannot recover cleanly from the high acoustic track. A red diagnostic bead appears over the wet emitter, very small and very satisfied with itself.

~ record_confidence = record_confidence - 1
~ witness_access = witness_access - 1

Mina's hand moves toward the clock control. Halden's moves toward his duplicate slate. Toma keeps both palms on a rail that still works.

-> failure_choice

=== failure_choice ===
// ghostlight.choice_layer: inaccessible_evidence_response
+ [Strike the wet paddle for an official pause and put your own name on the delay.]
    // ghostlight.branch: respond_pause_clock
    // ghostlight.action: touch_interface
    // ghostlight.intent: stop_procedure_until_access_is_restored
    ~ clock_paused = 1
    ~ requester_recorded = 1
    ~ witness_access = witness_access + 1
    // ghostlight.visual_scene_id: brineglass_clock_pause
    Sere hits the paddle with his rostrum. The amber berth digits freeze. Beside them appears REQUESTED BY: SERE-IN-RETURN, because even an accommodation needs someone convenient to blame.

    Halden reads the name. Mina reads Halden.

    "The queue will see it," Halden says.

    "The queue has excellent eyesight," Sere answers. "That is not the failed channel."
    // ghostlight.npc_response: halden_registers_pause_as_schedule_cost
    // ghostlight.consequence: access_partly_restored_clock_paused_request_public
    -> disposition_fold
+ [Ask Mina to translate the missing interval into the raised rail and accept the docking-time cost.]
    // ghostlight.branch: respond_manual_translation
    // ghostlight.action: request_translation
    // ghostlight.intent: rebuild_shared_evidence_through_staffed_mediation
    ~ manual_translation = 1
    ~ record_confidence = record_confidence + 2
    ~ witness_access = witness_access + 2
    ~ berth_minutes = berth_minutes - 2
    // ghostlight.visual_scene_id: brineglass_manual_translation
    Mina opens the translator pane and drags the raw light timing into a raised rail sequence. She reads each interval aloud; Toma repeats it; Sere answers from the pool with low clicks the speaker can carry.

    It is slower than synchronized replay and faster than pretending synchronization happened.

    {berth_minutes <= 1: By the time the last interval settles under Toma's fingers, the berth clock shows a single amber minute.}
    {berth_minutes > 1: By the time the last interval settles under Toma's fingers, the berth clock has burned deep into amber.}
    // ghostlight.npc_response: mina_translates_toma_witnesses_halden_loses_margin
    // ghostlight.consequence: evidence_and_access_restored_time_spent
    -> disposition_fold
+ [Reconstruct the absent ridge from the water still moving around your body.]
    // ghostlight.branch: respond_embodied_inference
    // ghostlight.action: inspect_environment
    // ghostlight.intent: preserve_time_by_using_embodied_route_skill
    ~ embodied_inference = 1
    ~ record_confidence = record_confidence + 1
    ~ berth_minutes = berth_minutes - 1
    // ghostlight.visual_scene_id: brineglass_embodied_inference
    Sere turns sideways in the working lane and lets the surviving wake cross jaw, flank, fin, and scar tissue in four different timings.

    The absent ridge leaves a hole with edges. He can estimate where it belonged. Estimate is not replay, however much expertise dislikes the syllables.

    "I can reconstruct the interval," Sere says. "I cannot make it unbroken."

    Mina enters INFERENCE beside the segment. Halden does not thank him. Good.
    // ghostlight.consequence: confidence_partly_restored_inference_marked_time_spent
    -> disposition_fold
+ [Let Halden narrate the missing interval while Toma keeps hands on the working rail.]
    // ghostlight.branch: respond_carrier_account
    // ghostlight.action: listen
    // ghostlight.intent: preserve_schedule_by_using_claimant_testimony
    ~ carrier_account_used = 1
    ~ carrier_trust = carrier_trust + 1
    ~ record_confidence = record_confidence + 1
    // ghostlight.visual_scene_id: brineglass_carrier_account
    Halden places the duplicate slate on the rail where Toma can feel each timestamp he names.

    "Wake warning at twelve-point-four. Formation break at twelve-point-nine. Tender distress at thirteen-point-one."

    Toma's fingers stop over the second mark. The rail agrees that the numbers exist. It cannot say why Halden selected them.

    Sere hears a usable account. He also hears the claimant standing inside it.
    // ghostlight.npc_response: halden_supplies_contested_timing_toma_observes
    // ghostlight.consequence: claimant_account_added_without_independent_channel
    -> disposition_fold

=== disposition_fold ===
// ghostlight.fold: channel_response_into_shared_ruling
// ghostlight.visual_scene_id: brineglass_disposition_threshold
The trace ends with _Dawn Ledger_ alongside the damaged tender, rescue propellant falling, convoy geometry ruined, all six hull notes still sounding.

The berth clock keeps running if it was never stopped. The dry gallery smells faintly of warmed cable insulation. The pool carries a small shiver from the failed emitter every time the board tries to clear itself.

{clock_paused == 1: Sere's name remains beside the frozen clock. The delay has an owner before the decision does.}
{manual_translation == 1: The translated interval exists on light, rail, speech, and Sere's low-click confirmation.}
{embodied_inference == 1: INFERENCE marks the critical segment. The board does not pretend expertise is a sensor.}
{carrier_account_used == 1: Halden's three selected timestamps sit inside a blue claimant bracket.}
{carrier_trust >= 3: Halden has spent enough plain account in the room that his urgency reads as more than pressure, though it still does not become a sensor.}
{apprentice_understanding >= 2: Toma now knows exactly where the channels disagree and looks much less comforted by competence.}
{evidence_custody >= 3: The custody marks remain intact; whatever Sere decides will at least be about the trace that arrived.}
{witness_access <= 0: The decisive evidence is still more legible from the dry gallery than from the clerk's own pool.}

Mina waits at the console. Halden waits by the rail. Toma waits with both hands open on the notation.

Sere owns the next local act: admit the trace for this berth decision, reject it and send the carrier to standby, or keep the claim open until the missing interval survives a complete shared replay.

-> disposition_choice

=== disposition_choice ===
// ghostlight.choice_layer: berth_claim_disposition
+ [Admit the hazard trace and allocate _Dawn Ledger_ the berth now.]
    // ghostlight.branch: decide_admit_trace
    // ghostlight.action: commit_ruling
    // ghostlight.intent: protect_carrier_bond_and_cargo_schedule
    {record_confidence >= 2 && witness_access >= 1:
        -> ending_admit_grounded
    - else:
        -> ending_admit_cost
    }
+ [Reject the trace for this hearing and send _Dawn Ledger_ to standby.]
    // ghostlight.branch: decide_reject_trace
    // ghostlight.action: commit_ruling
    // ghostlight.intent: refuse_a_decision_from_inaccessible_or_contested_evidence
    {evidence_custody >= 3 && record_confidence >= 1:
        -> ending_reject_grounded
    - else:
        -> ending_reject_cost
    }
+ [Keep the claim open until the translated interval clears a full replay.]
    // ghostlight.branch: decide_adjourn_translate
    // ghostlight.action: commit_ruling
    // ghostlight.intent: preserve_shared_access_and_verify_translation_before_final_disposition
    ~ manual_translation = 1
    {clock_paused == 1 || berth_minutes >= 2:
        -> ending_adjourn_grounded
    - else:
        -> ending_adjourn_cost
    }

=== ending_admit_grounded ===
// ghostlight.ending_label: admitted_with_shared_record
// ghostlight.training_hook: accessible_evidence_supports_time_sensitive_ruling
// ghostlight.visual_scene_id: brineglass_ending_admit
Sere touches the admit paddle. _Dawn Ledger_ receives the berth; the hazard diversion enters the record; the bond survives review.

{manual_translation == 1: The translated interval travels with the ruling instead of vanishing as staff labor.}
{embodied_inference == 1: Sere's inference mark remains attached, a boundary around expertise rather than a halo.}
{requester_recorded == 1: His name stays on the pause. Halden's ship also carries the reason for it.}

Toma takes one hand from the rail only after the board finishes all four versions of the decision.

The food and clinic filters move toward unloading. Nobody wins back the minutes. At Brineglass, fairness is rarely free; today it is at least itemized beside the people who benefited.
-> END

=== ending_admit_cost ===
// ghostlight.ending_label: admitted_under_access_debt
// ghostlight.training_hook: schedule_preserved_by_weakening_shared_evidence
// ghostlight.visual_scene_id: brineglass_ending_admit
Sere admits the trace.

The berth gate turns white at once. Halden exhales. Toma does not move their hands from the rail.

The ruling protects the bond and cargo schedule, but the critical interval remains a dryside event Sere was asked to authorize from its shadow. Mina attaches the failed-channel diagnostic. That is evidence of the failure, not repair of the hearing.

When _Dawn Ledger_ begins unloading, the board thanks all parties for completing an accessible proceeding. Machines acquire confidence very cheaply.
-> END

=== ending_reject_grounded ===
// ghostlight.ending_label: rejected_with_intact_custody
// ghostlight.training_hook: custody_and_uncertainty_bound_a_local_refusal
// ghostlight.visual_scene_id: brineglass_ending_reject
Sere touches reject.

The carrier moves to standby. Its bond enters review, and another convoy receives the berth. The sealed trace stays intact with the failed segment, the rescue ledger mark, and the reason Sere would not treat one working channel as four agreeing witnesses.

Halden's mouth tightens. "The tender crew is alive."

"That fact is in the ledger," Sere says. "It is not permission to invent the timing."

Toma copies the custody marks before leaving. The lesson may become resentment. It may become practice. Sere cannot select which from the console.
-> END

=== ending_reject_cost ===
// ghostlight.ending_label: rejected_from_thin_record
// ghostlight.training_hook: procedural_refusal_can_export_material_harm
// ghostlight.visual_scene_id: brineglass_ending_reject
Sere rejects the trace because it is incomplete.

It remains incomplete after the rejection. _Dawn Ledger_ loses the berth, the food and filters wait behind a pressure door, and the next station receives Brineglass's doubt without receiving any better evidence.

Halden calls it punishment. Mina calls it a reviewable local decision. Both descriptions fit through the same dry door.

In the pool, the bad emitter shivers once more. It has displaced its failure into a carrier schedule and now reports itself idle.
-> END

=== ending_adjourn_grounded ===
// ghostlight.ending_label: adjourned_with_clock_protection
// ghostlight.training_hook: time_is_bounded_while_access_is_rebuilt
// ghostlight.visual_scene_id: brineglass_ending_adjourn
Sere touches adjourn. Mina freezes the disposition and opens the manual translator, or keeps it open if the first reconstruction is already on the board.

{clock_paused == 1: The berth clock remains stopped under Sere's name. The queue can object to a visible act instead of benefiting from an invisible rush.}
{clock_paused == 0: Two berth minutes remain. Mina spends them aloud, one interval at a time.}

Halden sits. Toma keeps the rail. Sere turns broadside to the replacement pulses while Mina rebuilds the missing segment across light, touch, speech, and water, then runs the complete trace once more.

The hearing will finish late. That is not the same as failing to finish. Brineglass is poor enough to confuse those facts often, but not on this shift.
-> END

=== ending_adjourn_cost ===
// ghostlight.ending_label: adjourned_after_margin_spent
// ghostlight.training_hook: accommodation_arrives_after_schedule_authority_has_bitten
// ghostlight.visual_scene_id: brineglass_ending_adjourn
Sere touches adjourn with no useful minutes left.

The berth passes to the next convoy before Mina can finish the translation. _Dawn Ledger_ goes to standby anyway, now carrying an open claim instead of a rejection. The distinction will matter later to a bond clerk. It does not open a pressure door tonight.

Toma stays at the rail to help rebuild the record. Halden goes to manage the cargo delay. Mina puts fresh time beside every surviving channel.

Sere listens to the repaired test pulse arrive through the pool after the power geometry has already decided the practical result.
-> END
