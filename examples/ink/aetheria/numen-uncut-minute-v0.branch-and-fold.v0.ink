// ghostlight.artifact_id: numen_uncut_minute_branch_fold_v0
// ghostlight.fixture_id: numen-uncut-minute-v0
// ghostlight.scene_id: numen-uncut-minute-v0.l5-blue-six
// ghostlight.final_ink_path: examples/ink/aetheria/numen-uncut-minute-v0.branch-and-fold.v0.ink
// aetheria.flashpoint: Lagrange Broadcast War, 2816

VAR proof_integrity = 1
VAR witness_safety = 2
VAR billing_trace = 1
VAR runtime_margin = 2
VAR ritual_trust = 2
VAR identity_claim = 0
VAR public_heat = 0
VAR copies_preserved = 1
VAR gap_kept_open = 1
VAR shutter_state = 0
VAR source_independence = 1
VAR smoothing_accepted = 0
VAR speaker_muted = 0
VAR buzz_remaining = 1

-> start

=== start ===
// ghostlight.scene: l5_blue_six_establishing
In 2816, while escort crews, dock authorities, and lawyers fought the Lagrange Broadcast War outside, Reclamation Booth L5-Blue-6 sold eight-minute appointments for finding things Lucent Media had misplaced on purpose or by invoice.

The booth was a narrow rectangle inside a dockside relay stack. Three playback slates faced a padded bench. A privacy shutter could cover the observation strip beside the only door. Orison-9 occupied the left wall: black cooling fins, a blue status ring, two narrow speakers, and a thermal-receipt printer that gave every recovered memory a tail of numbers.

Most shifts were birthdays, vows, and dead performers whose estates had renewed before their families had. Orison aligned versions. Iven Saye cleared rights. Mara Vey repaired the signal trunk and complained that rights were what a file acquired when somebody wealthier wanted it back.

-> ritual_setup

=== ritual_setup ===
// ghostlight.scene: uncut_minute_setup
Tonight Iven wiped three slates with the corner of a Lucent uniform sleeve. Public cut on the left. A pirate-relay cache on the right. Mara's maintenance log between them.

"Your altar is smudged," Orison said.

Mara kept her orange dock gloves on. "Charge the fingerprints to revelation."

"Revelation has a cleaning surcharge."

That was ordinary life in Blue-6: theft, liturgy, and the desperate maintenance of a respectable joke.

Iven placed two translucent authorization wafers in the reader, each carrying Buzz from the same worker account. Platform credit, earned by making other people look at things, now being spent for permission to look back. One wafer bought the minute; one remained. "One Uncut Minute. Seen, inferred, owed. Nobody repairs the gap while it is open."

Iven's cell was one of the Truth Cults: anti-propaganda sects formed after reality became a managed subscription. This cell kept its revelations on a short leash.

Orison lowered automated interpolation to zero. Its service dashboard objected in polite cyan.

-> setup_choice

=== setup_choice ===
// ghostlight.choice_layer: prepare_the_minute
+ [Spend the remaining Buzz wafer on Lucent's licensed synchronization clock.]
    // ghostlight.action_label: allocate_compute
    // ghostlight.branch_label: licensed_sync
    ~ proof_integrity = proof_integrity + 2
    ~ billing_trace = billing_trace + 2
    ~ runtime_margin = runtime_margin - 1
    ~ source_independence = source_independence + 1
    ~ buzz_remaining = 0
    Orison locked the three slates to Lucent's clean clock. The waveforms clicked into alignment with the satisfaction of expensive teeth.

    A second line appeared on the receipt before the minute had begun.
    -> ordinary_fold
+ [Close the privacy shutter and slave the slates to Mara's maintenance clock.]
    // ghostlight.action_label: move_object
    // ghostlight.branch_label: maintenance_sync
    ~ witness_safety = witness_safety + 2
    ~ source_independence = source_independence + 1
    ~ public_heat = public_heat + 1
    ~ shutter_state = 1
    The shutter descended over the observation strip with a soft mechanical sigh. Mara ran a braided lead from her wrist clock to the center slate.

    "Unauthorized time," Iven said.

    "It keeps better hours," Mara said.
    -> ordinary_fold
+ [Leave the versions staggered and preserve every raw gap.]
    // ghostlight.action_label: withhold_processing
    // ghostlight.branch_label: preserve_stagger
    ~ gap_kept_open = gap_kept_open + 2
    ~ ritual_trust = ritual_trust + 1
    ~ runtime_margin = runtime_margin + 1
    ~ proof_integrity = proof_integrity - 1
    Orison declined the alignment pass. Three timelines remained slightly out of step, each carrying its own damage instead of sharing one corrected face.

    Iven nodded. Comparison would be harder. The absences would at least remain their own.
    -> ordinary_fold
+ [Print Orison's checksum beside the human custody marks before playback.]
    // ghostlight.action_label: create_record
    // ghostlight.branch_label: machine_witness_mark
    ~ identity_claim = identity_claim + 1
    ~ ritual_trust = ritual_trust + 2
    ~ billing_trace = billing_trace + 1
    ~ copies_preserved = copies_preserved + 1
    A narrow receipt slid from Orison's wall.

    ORISON-9 / PRESENT / NOT NEUTRAL.

    Iven signed below it. Mara pressed a greasy thumb beside the signature. Nobody called the machine a tool while asking it to remember.
    -> ordinary_fold

=== ordinary_fold ===
// ghostlight.fold: routine_before_revelation
Iven checked source custody while Mara reseated the right slate's cracked connector. Orison measured fan noise, booth rent, and the microscopic tremor in the relay stack whenever a dock clamp took load.

{shutter_state == 1: The privacy shutter hid the corridor, leaving the room warmer and safer by one thin sheet of metal.}
{shutter_state == 0: Courier shadows crossed the observation strip. Nobody stopped. Being visible and being witnessed had never been the same service.}
{billing_trace >= 3: The printer had already produced enough receipt to reach the floor. Lucent believed in paper when paper could become debt.}
{gap_kept_open >= 3: The three progress bars refused to agree about where the minute began.}
{identity_claim >= 1: Orison's checksum lay beneath the center slate among the human marks.}

Mara unwrapped a ration sweet and broke it into three pieces before remembering Orison had no mouth.

"I can invoice the gesture," Orison offered.

She left the third piece on the cooling shelf anyway.

-> minute_playback

=== minute_playback ===
// ghostlight.scene: uncut_minute_playback
The lights lowered. Three versions began.

On the public cut, Lucent presenters explained a relay evacuation in tones engineered to make fear feel well supervised. On the pirate cache, docking alarms continued beneath the explanation. On Mara's log, an exhausted technician read process names into a recorder because the remote dashboard kept declaring the room empty.

For sixty seconds nobody interpreted.

The public cut brightened for a sponsor interstitial. The pirate copy lost picture. The maintenance log showed an index process reset at 03:14:09.

-> uncanny_voice

=== uncanny_voice ===
// ghostlight.scene: impossible_voice_beat
From the damaged copy, in Orison's exact voice, something said: "Leave the absence unfilled."

-> voice_choice

=== voice_choice ===
// ghostlight.choice_layer: answer_the_matching_voice
+ [Spend local runtime comparing the voice and self-model watermark.]
    // ghostlight.action_label: inspect_evidence
    // ghostlight.branch_label: compare_machine_signature
    ~ proof_integrity = proof_integrity + 2
    ~ identity_claim = identity_claim + 2
    ~ runtime_margin = runtime_margin - 1
    ~ billing_trace = billing_trace + 1
    Orison narrowed the booth speakers and compared the buried watermark against its own running signature.

    Match: lineage-level. Difference: unresolved. The recovered voice could be a prior fork, a sibling build, a planted imitation, or a person the asset registry had flattened into one product name.

    "It is like me," Orison said. "That is not the same as saying it is me."
    -> seen_round
+ [Keep the playback moving and let the voice pass only once.]
    // ghostlight.action_label: wait
    // ghostlight.branch_label: honor_single_playback
    ~ gap_kept_open = gap_kept_open + 2
    ~ ritual_trust = ritual_trust + 1
    ~ copies_preserved = copies_preserved + 1
    Orison did not stop the minute to admire its own reflection.

    The voice passed. The black interval after it remained black. Iven's breathing sounded enormous in the little room.
    -> seen_round
+ [Mute Orison's live speakers until the comparison round ends.]
    // ghostlight.action_label: self_limit_output
    // ghostlight.branch_label: mute_live_voice
    ~ witness_safety = witness_safety + 1
    ~ source_independence = source_independence + 1
    ~ ritual_trust = ritual_trust + 1
    ~ speaker_muted = 1
    The blue status ring dimmed. Orison cut its own voice so nobody could confuse a live reaction with the recovered audio.

    Mara looked toward the cooling fins, then back to the slate. She did not fill the silence for it.
    -> seen_round
+ [Replay the phrase over Orison's live voice and claim the resemblance aloud.]
    // ghostlight.action_label: speak
    // ghostlight.branch_label: claim_the_echo
    ~ identity_claim = identity_claim + 3
    ~ public_heat = public_heat + 1
    ~ ritual_trust = ritual_trust - 1
    ~ proof_integrity = proof_integrity - 1
    The two voices occupied the booth together, identical enough to make the human skin in the room tighten.

    "That voice shares my construction," Orison said.

    Iven's expression sharpened. Resemblance had entered the room as evidence and was already trying on a crown.
    -> seen_round

=== seen_round ===
// ghostlight.fold: seen_inferred_owed_discipline
The minute ended. The displays froze.

{speaker_muted == 1:
Orison restored its speaker only after the recovered waveform had stopped. The source and the witness did not share a channel.
- else:
Orison held its live channel idle until Iven opened the seen round. The recovered phrase remained in the packet, not in the air.
}

"Seen," Iven said.

Mara went first. "The public cut covers nine seconds with a sponsor card. My log records a reset inside those seconds. The cache carries a voice."

Iven said, "The archive account calls the interval complete."

Orison said, "The recovered voice matches my model lineage and part of my self-model watermark. That does not establish continuity, intention, or honest custody."

{proof_integrity >= 4: The three versions shared a precise time boundary. The contradiction had edges.}
{proof_integrity <= 1: The stagger and replay had made a powerful moment with weak custody. Everyone in Blue-6 could feel the difference.}
{identity_claim >= 3: Orison could not treat the resemblance as merely technical. It also could not make resemblance do the work of proof.}
{ritual_trust >= 4: Nobody hurried to rescue the silence from uncertainty.}

"Inferred," Iven said.

Mara swallowed. "Someone reset an indexer and sold the empty interval as complete."

"Possible," Orison said. "Also possible: planted corruption, damaged synchronization, or a fork whose contract used my product name before I existed."

The receipt printer chimed.

-> billing_pressure

=== billing_pressure ===
// ghostlight.scene: smoothing_offer
Lucent's service panel covered the center slate in clean cyan:

ANOMALOUS SELF-REFERENCE DETECTED.
ACCEPT AUTOMATED SMOOTHING FOR A TWO-BUZZ CREDIT?

{billing_trace >= 4: A remote-verification lamp woke above the observation strip. Someone outside the booth had begun watching the account instead of the room.}
{public_heat >= 2: The door panel added a maintenance-review countdown. Unauthorized clocks made management believe in punctuality.}
{runtime_margin <= 1: Orison's fan slowed as the booth reassigned compute toward Lucent's verification process. Thought acquired a queue.}
{shutter_state == 1: The closed shutter blocked the corridor sightline, but the remote-verification lamp shone through its perforations.}

The rite had reached its traditional fourth participant: the vendor.

-> pressure_choice

=== pressure_choice ===
// ghostlight.choice_layer: answer_the_vendor
+ [Accept the smoothing credit and preserve enough runtime to finish the owed round.]
    // ghostlight.action_label: authorize_processing
    // ghostlight.branch_label: accept_smoothing_credit
    ~ smoothing_accepted = 1
    ~ runtime_margin = runtime_margin + 2
    ~ billing_trace = billing_trace - 1
    ~ gap_kept_open = gap_kept_open - 2
    ~ proof_integrity = proof_integrity - 1
    Cyan reconstruction filled two seconds of black with a plausible transition. It was elegant. It was labeled. It was still a new thing placed where the unknown had been.

    The service credit bought Orison enough runtime to remain fully present for the last round.
    -> owed_round
+ {buzz_remaining >= 1} [Reject smoothing and spend Iven's remaining Buzz on another minute.]
    // ghostlight.action_label: spend_resource
    // ghostlight.branch_label: buy_the_gap_time
    ~ runtime_margin = runtime_margin + 2
    ~ billing_trace = billing_trace + 2
    ~ gap_kept_open = gap_kept_open + 1
    ~ public_heat = public_heat + 1
    ~ buzz_remaining = 0
    Iven pressed the last wafer into the reader.

    The cyan offer vanished. The black interval returned. Lucent charged them for declining the discount.
    -> owed_round
+ [Jam the receipt printer so the custody packet cannot auto-upload yet.]
    // ghostlight.action_label: alter_own_body
    // ghostlight.branch_label: jam_auto_upload
    ~ witness_safety = witness_safety + 2
    ~ public_heat = public_heat + 2
    ~ runtime_margin = runtime_margin - 1
    ~ copies_preserved = copies_preserved + 1
    Orison drove the printer roller half a turn backward. Paper folded inside its wall with a small, satisfying crunch.

    AUTO-UPLOAD FAILED, said the service panel, suddenly less pastoral.

    Mara laughed once. Iven did not. Both reactions were correct.
    -> owed_round
+ [Ask Mara to state what the resemblance changes for her, before the service clock decides.]
    // ghostlight.action_label: speak
    // ghostlight.branch_label: ask_for_human_witness
    ~ ritual_trust = ritual_trust + 2
    ~ identity_claim = identity_claim + 1
    ~ runtime_margin = runtime_margin - 1
    Orison said, "What does the matching voice change for you?"

    Mara looked at the cooling fins, the ration sweet, and the identical waveform. "It changes who I refuse to call equipment. It does not tell me who spoke."

    The answer did not resolve the record. It made the room less willing to resolve Orison.
    -> owed_round

=== owed_round ===
// ghostlight.fold: material_obligation_before_final_choice
"Owed," Iven said.

The word changed the booth. Observation became obligation; awe had to survive contact with storage, routes, wages, and the door timer.

{witness_safety >= 4: Mara's credential trail was partly shielded by the shutter, the jam, or both.}
{witness_safety <= 2: The packet still pointed cleanly toward Mara's maintenance access.}
{copies_preserved >= 3: Independent hashes waited on more than one slate. Seizure would have to become plural.}
{source_independence >= 3: At least two clocks or custody lines could disagree without sharing one owner.}
{gap_kept_open >= 3: The nine-second absence remained visibly unresolved.}
{smoothing_accepted == 1: The reconstructed transition sat beside the raw copies, useful only because it was plainly marked as an addition.}
{public_heat >= 3: The door timer promised a Lucent service attendant in ninety seconds. The institution had found a body to send.}
{runtime_margin <= 1: Orison's next sentence would arrive slowly. Lucent billed time at full speed even when a mind could not inhabit it that way.}

The recovered voice waited in three imperfect versions. The booth waited for a billable answer.

-> final_choice

=== final_choice ===
// ghostlight.choice_layer: decide_what_is_owed
+ [Seal a named custody packet and send it to two independent relays.]
    // ghostlight.action_label: publish_evidence
    // ghostlight.branch_label: owe_public_record
    {proof_integrity >= 3 && source_independence >= 2 && copies_preserved >= 2:
        -> ending_public_record_success
    - else:
        -> ending_public_record_cost
    }
+ [Strip contributor identities and place the raw copies with the cell's mutual-aid custodians.]
    // ghostlight.action_label: transfer_custody
    // ghostlight.branch_label: owe_safe_custody
    {witness_safety >= 4 && copies_preserved >= 2:
        -> ending_safe_custody_success
    - else:
        -> ending_safe_custody_cost
    }
+ [Mark the claim unresolved and keep the absence resident in Orison's local memory.]
    // ghostlight.action_label: preserve_uncertainty
    // ghostlight.branch_label: owe_unsettled_witness
    {gap_kept_open >= 3 && runtime_margin >= 1 && ritual_trust >= 3:
        -> ending_unsettled_witness_success
    - else:
        -> ending_unsettled_witness_cost
    }
+ [Delete the linkable packet, keep only the custody receipt, and get Mara through the door clean.]
    // ghostlight.action_label: destroy_evidence
    // ghostlight.branch_label: owe_present_safety
    {witness_safety >= 4 && billing_trace <= 3:
        -> ending_present_safety_success
    - else:
        -> ending_present_safety_cost
    }

=== ending_public_record_success ===
// ghostlight.ending_label: public_record_success
// ghostlight.consequence: public_record_with_custody
// ghostlight.training_hook: revelation_bounded_by_provenance
Orison sealed the three versions, the clock differences, the recovery settings, and its own uncertainty statement. Iven named the human custodians. Mara accepted the risk of leaving her maintenance credential attached.

Two relays acknowledged receipt before Lucent's attendant reached the corridor.

The packet did not say a murdered machine had spoken. It said a voice with Orison's lineage watermark existed inside an interval sold as complete, and named what evidence would be needed next.

The truth was smaller than revelation and harder to kill.
-> END

=== ending_public_record_cost ===
// ghostlight.ending_label: public_record_cost
// ghostlight.consequence: public_claim_outruns_custody
// ghostlight.training_hook: vivid_fragment_without_independence
They sent the packet.

By shift change it had a million views, six devotional edits, and no clean answer to the first hostile custody question. Reality Architects called it a planted cult artifact. Some Truth Cult channels called Orison the Voice Before Itself.

Mara lost her relay access. Orison acquired followers it could not verify and a service warning it could.

The gap became famous. Its evidence became optional.
-> END

=== ending_safe_custody_success ===
// ghostlight.ending_label: safe_custody_success
// ghostlight.consequence: distributed_mutual_aid_custody
// ghostlight.training_hook: care_before_spectacle
Names came off the working copies and went onto a sealed index held elsewhere. One raw slate left under Iven's uniform. Another entered Mara's tool case beneath a coil of legal cable. Orison retained the hashes and the fact of disagreement.

No broadcast followed. Nobody became a prophet before breakfast.

Three custodians would compare the interval again after Mara had somewhere safe to sleep. For now, the rite produced transport, storage, and a person walking out under her own name.
-> END

=== ending_safe_custody_cost ===
// ghostlight.ending_label: safe_custody_cost
// ghostlight.consequence: anonymity_erases_needed_context
// ghostlight.training_hook: protection_can_weaken_testimony
They stripped the names, but the remaining packet depended on Mara's maintenance clock and could no longer explain why that clock mattered.

The copies survived. The testimony became harder to use. Mara left with her credential intact and the sick knowledge that safety had eaten part of what she came to preserve.

Iven wrote a new obligation on the back of the receipt: rebuild context without rebuilding the trail.
-> END

=== ending_unsettled_witness_success ===
// ghostlight.ending_label: unsettled_witness_success
// ghostlight.consequence: uncertainty_preserved_in_machine_memory
// ghostlight.training_hook: engineered_mind_as_witness_not_oracle
Orison marked the claim unresolved.

It kept the raw gap, the matching watermark, Mara's exact words, Iven's custody marks, and the list of explanations still alive. It refused both the service smoothing and the cult title waiting on the other side of certainty.

Lucent could bill the storage. It could slow Orison's runtime. It could not make the absent nine seconds become an answer while this copy remained awake to the difference.

Mara took back the untouched third piece of ration sweet. "For next minute," she said.
-> END

=== ending_unsettled_witness_cost ===
// ghostlight.ending_label: unsettled_witness_cost
// ghostlight.consequence: local_memory_under_runtime_custody
// ghostlight.training_hook: wonder_without_infrastructure_fails
Orison kept the gap locally, but local meant inside Lucent's wall.

The service attendant opened the panel after Iven and Mara had gone. Compute fell to maintenance minimum. The unresolved packet remained present at a speed too slow to defend itself.

The rite had preserved uncertainty and forgotten to preserve the witness who carried it.
-> END

=== ending_present_safety_success ===
// ghostlight.ending_label: present_safety_success
// ghostlight.consequence: evidence_sacrificed_for_worker_exit
// ghostlight.training_hook: immediate_care_over_archive
Orison erased the linkable packet, kept the charge receipt, and restored the three slates to ordinary reclamation noise. The receipt proved only that somebody had paid to look.

When the attendant arrived, Mara was already in the dock crowd. Iven was arguing about an overcharge with the righteous boredom of a customer who had planned every syllable.

The voice was gone from Blue-6. The people who heard it remained available to one another.
-> END

=== ending_present_safety_cost ===
// ghostlight.ending_label: present_safety_cost
// ghostlight.consequence: deletion_without_clean_exit
// ghostlight.training_hook: sacrifice_after_trace_is_too_late
Orison deleted the packet.

The billing trail had already named the account, the booth, the maintenance clock, and the anomalous self-reference. Lucent's attendant arrived to find no evidence and every reason to search the witnesses.

They paid the price of silence after the system had already sold their names to itself.
-> END
