// ghostlight.artifact_id: veil_welfare_exception_chain_branch_fold_v0
// ghostlight.fixture_id: veil-welfare-exception-chain-v0
// ghostlight.scene_id: veil-welfare-exception-chain-v0.second-count-before-audit
// ghostlight.final_ink_path: examples/ink/aetheria/veil-welfare-exception-chain-v0.branch-and-fold.v0.ink

VAR care_trace = 1
VAR raw_trace = 0
VAR joined_exception = 0
VAR mutual_trust = 1
VAR employment_cover = 2
VAR audit_window = 2
VAR containment_heat = 1
VAR raven_exposure = 0
VAR receipt_cache = 0

-> start

=== start ===
// ghostlight.scene: reconciliation_booth_establishing
The Welfare Reconciliation Booth at NeuroSyn's Mars Corvid Containment Annex is designed for three kinds of truth, provided they arrive separately.

On the left wall, a treatment console prints numbered seals for sedatives, salves, and restraint wounds. On the right, an interface rack reduces a night of pulse, movement, and attention into approved welfare categories. Between them, a steel counter ends at a ribbed observation window. Beyond the glass, uplifted ravens wait on aluminum perches under the dim lamps of Aviary C.

The corridor door is behind Tava Mirren. Under the window, a two-sided sample drawer is the only opening into the aviary. A red shred slot waits beside the door with the patient confidence of a company that expects to be obeyed eventually.

-> ordinary_count

=== ordinary_count ===
// ghostlight.scene: ordinary_second_count
Tava is the night animal-care clerk: brown-skinned, middle-aged, and employed through a contractor whose logo is larger than its sick-leave allowance. She counts used drug seals against treatment tickets before morning handoff.

The console says six sedative doses.

Latch, the human-facing role-name of the raven watching from the other side of the glass, taps the perch seven times.

"Six," Tava says.

Seven taps.

"The machine has a degree."

Latch tips a black head toward the bandaged juvenile below. The joke stops being a joke without anyone signing the change order.

-> printer_routine

=== printer_routine ===
// ghostlight.scene: calibrator_and_lead
Oren Kade works at the interface rack in dark maintenance coveralls. His orange diagnostic lens magnifies one eye and makes every conversation look more incriminating than it is. The rejection printer has jammed again. It considers every seventh event a philosophical objection.

Containment lead Hara Venn stands in the corridor doorway, black uniform immaculate, white credential tab bright at her throat. Her morning welfare bundle calls the seventh interval scheduled rest.

"Close the count," Hara says. "Rossum moved the audit forward."

A Rossum & Douglas auditor can inspect the submitted bundle and its pruning rules. Mission telemetry belongs to the security client. Treatment files belong to the clinic contractor. The audit sees the marriage, not the spouses.

Latch taps seven once more.

-> routine_choice

=== routine_choice ===
// ghostlight.choice_layer: ordinary_reconciliation
+ [Recount the numbered sedative seals and match the seventh to the juvenile's treatment ticket.]
    // ghostlight.action_label: inspect_object
    // ghostlight.branch_label: prime_care_trace
    ~ care_trace = care_trace + 2
    ~ audit_window = audit_window - 1
    ~ containment_heat = containment_heat + 1
    Tava turns each thumb-sized seal under the counter lamp. Six appear in the console. The seventh exists in her hand and on the juvenile's bandage lot.

    She slips the treatment ticket beneath her receipt cuff. The cuff pinches. Evidence is often less comfortable than conscience promised.
    -> routine_fold
+ [Open the sample drawer and give Latch a blank receipt strip for the second count.]
    // ghostlight.action_label: transfer_object
    // ghostlight.branch_label: prime_second_count
    ~ mutual_trust = mutual_trust + 2
    ~ receipt_cache = receipt_cache + 1
    ~ employment_cover = employment_cover - 1
    Tava feeds a blank thermal strip into the steel drawer and pushes it through.

    Latch takes the paper in the beak, folds it once under one claw, and carries it to the narrow shadow behind the lower perch bracket. Not hiding. Filing, under a system with fewer badges.

    Hara watches Tava close the drawer.
    -> routine_fold
+ [Ask Oren why the rejection printer jams on the same minute as the missing seal.]
    // ghostlight.action_label: speak
    // ghostlight.branch_label: prime_raw_trace
    ~ raw_trace = raw_trace + 2
    ~ employment_cover = employment_cover - 1
    ~ containment_heat = containment_heat + 1
    Oren does not look at Tava. "Repeated distress gets merged. Mission-sensitive distress gets removed. This one was ambitious and did both."

    He lifts the printer latch just enough for her to read the rejected timecode.

    Hara says, "Is there a technical problem?"

    "There is a printer," Oren says. This is not an answer, but it has survived payroll before.
    -> routine_fold
+ [Accept Hara's scheduled-rest classification and close the count on six.]
    // ghostlight.action_label: withhold_action
    // ghostlight.branch_label: prime_job_cover
    ~ employment_cover = employment_cover + 2
    ~ audit_window = audit_window + 1
    ~ mutual_trust = mutual_trust - 1
    Tava signs six.

    The console turns green with the relieved speed of a machine discovering it will not be asked to testify.

    Latch stops tapping. The silence is precise enough to count.
    -> routine_fold

=== routine_fold ===
// ghostlight.fold: routine_before_early_audit
Tava stacks treatment tickets on the left. Oren clears calibration scraps on the right. Hara waits between them in the doorway, where authority gets the finest view and the least useful angle.

{care_trace >= 3: The seventh seal presses a hard little circle against Tava's wrist.}
{raw_trace >= 2: Tava has seen the rejected event: the same minute, the same juvenile, a response renamed twice on its way to morning.}
{mutual_trust >= 3: Behind the lower perch bracket, Latch keeps the blank strip dry and flat.}
{mutual_trust <= 0: Latch has moved to a high perch beyond the sample drawer's easy reach.}
{employment_cover >= 4: Tava's signed count is clean enough to protect her and useful enough to protect Hara.}
{containment_heat >= 3: Hara's attention rests on Tava and Oren instead of the completed bundle.}

Then the rejection printer coughs once and produces the event it was configured to forget.

-> rejected_event

=== rejected_event ===
// ghostlight.scene: rejected_event_receipt
The strip carries no mission image and no medical detail. Only a timecode, a sensor channel, and the phrase PRUNED: CALIBRATION / DUPLICATE.

Oren tears it free. Tava's treatment console answers with the same timecode on the drug-seal ledger.

For one second the two records exist in the same room.

Hara crosses to the counter. "Shred the fault slip. The audit is at the outer door."

The corridor indicator turns from white to amber. A person in a plain gray audit jacket is waiting beyond it with an airgapped slate and no authority to open either private system.

-> evidence_choice

=== evidence_choice ===
// ghostlight.choice_layer: exception_chain_custody
+ [Place the rejected-event strip beside the numbered treatment ticket under the counter lamp.]
    // ghostlight.action_label: join_evidence
    // ghostlight.branch_label: join_exception_openly
    ~ joined_exception = 2
    ~ raw_trace = raw_trace + 1
    ~ care_trace = care_trace + 1
    ~ audit_window = audit_window - 1
    ~ employment_cover = employment_cover - 1
    ~ containment_heat = containment_heat + 2
    Tava puts the strips together.

    Same minute. Same station. Two owners who were never meant to compare handwriting.

    Oren exhales through his teeth. Hara reaches for the papers and stops because the corridor camera can now see her reaching.
    -> lead_and_auditor
+ [Copy both timecodes onto the cleaning roster, then feed the original fault strip to the shred slot.]
    // ghostlight.action_label: copy_and_destroy
    // ghostlight.branch_label: preserve_weak_join
    ~ joined_exception = 1
    ~ raw_trace = raw_trace + 1
    ~ care_trace = care_trace + 1
    ~ employment_cover = employment_cover + 1
    ~ containment_heat = containment_heat + 1
    Tava writes the two times beside FILTER CLOTH, which has spent years being innocent and takes the promotion badly.

    The shred slot eats Oren's strip. What survives is a join with no original custody and a janitorial alibi.
    -> lead_and_auditor
+ [Put Oren's fault strip into the sample drawer and let Latch decide whether to cache it.]
    // ghostlight.action_label: transfer_object
    // ghostlight.branch_label: cache_raw_receipt
    ~ raw_trace = raw_trace + 1
    ~ receipt_cache = receipt_cache + 2
    ~ mutual_trust = mutual_trust + 1
    ~ raven_exposure = raven_exposure + 1
    ~ audit_window = audit_window - 1
    Tava slides the strip through.

    Latch studies Tava, Oren, Hara, and the amber corridor light. Then the raven takes the receipt and drops behind the lower bracket.

    Hara sees the movement. She does not see which strip moved. That distinction is worth perhaps forty seconds.
    -> lead_and_auditor
+ [Return the fault strip to Oren and tell him the seventh seal will outlive the buffer.]
    // ghostlight.action_label: warn
    // ghostlight.branch_label: protect_partial_witnesses
    ~ raw_trace = raw_trace + 1
    ~ mutual_trust = mutual_trust + 1
    ~ employment_cover = employment_cover + 1
    ~ containment_heat = containment_heat - 1
    Oren folds the strip into the seam of his diagnostic lens case.

    "That is not a safe place," Tava says.

    "It is not a safe job. The place is on theme."

    They keep the records apart. Hara's packet remains clean. So do their names, for the next few minutes.
    -> lead_and_auditor

=== lead_and_auditor ===
// ghostlight.fold: partial_witnesses_face_the_audit
Hara opens the corridor door. Auditor Sen Aras enters carrying the airgapped slate against a gray jacket. Sen is here to decide whether the submitted configuration is legible enough for a client to accept, not whether anyone in the room deserves freedom.

"Routine welfare reconciliation," Hara says. "One printer fault. No material exception."

{joined_exception >= 1: Tava can feel the two timecodes touching even when the paper is no longer together.}
{receipt_cache >= 2: Latch waits behind the lower perch bracket with a strip in the beak.}
{audit_window <= 0: The audit slate is already displaying CLOSING SAMPLE WINDOW.}
{audit_window >= 3: Sen has enough sample time left to ask one question and dislike two answers.}
{containment_heat >= 4: Hara stands close enough to Tava that the white credential tab fills the edge of her vision.}
{raven_exposure >= 1: Hara has marked Latch's lower-perch movement on her wrist display.}
{employment_cover >= 4: Tava's personnel line remains green beside the clean six-dose count.}

Sen looks at the two consoles. "Does the live configuration match the submitted evidence body?"

Hara says yes.

Oren knows what the pruner rejected. Tava knows what the clinic used. Latch knows who screamed. Sen knows only that three people have become unusually interested in a small printer.

-> disclosure_choice

=== disclosure_choice ===
// ghostlight.choice_layer: disclosure_path
+ {joined_exception >= 1} [Name a configuration discontinuity and submit the joined timecodes.]
    // ghostlight.action_label: disclose
    // ghostlight.branch_label: disclose_formal_exception
    {joined_exception >= 2 && care_trace >= 2 && raw_trace >= 2 && audit_window > 0:
        Tava says, "The welfare bundle and live configuration diverge at one shared timecode. I can submit the two source receipts without disclosing their protected content."
        -> ending_formal_hold
    - else:
        Tava says, "There is a mismatch."

        Sen waits for admissible custody. Hara waits for Tava to discover she has brought a suspicion to a paperwork fight.
        -> ending_formal_rumor
    }
+ {receipt_cache >= 2} [Open the sample drawer and let Latch place the cached fault strip in Sen's sightline.]
    // ghostlight.action_label: open_route
    // ghostlight.branch_label: disclose_raven_cache
    {receipt_cache >= 2 && mutual_trust >= 3 && audit_window > 0:
        Tava pushes the drawer through. Latch hops down, places the strip inside, and withdraws. Tava pulls the drawer boothward beside the numbered seal before Hara reaches the glass.
        -> ending_cache_hold
    - else:
        Tava opens the drawer.

        The space beyond it is empty, or the wrong strip is waiting, or Sen's sample window closes before a beak appears. Distributed proof has manners; it does not pretend to be present when trust was not.
        -> ending_cache_cost
    }
+ {care_trace >= 2 && raw_trace >= 2} [Read only the matching timecodes into the audit recorder and keep both workers' names out.]
    // ghostlight.action_label: speak
    // ghostlight.branch_label: disclose_bounded_timecodes
    {care_trace >= 2 && raw_trace >= 2 && employment_cover >= 2 && audit_window > 0:
        Tava reads the two times and the configuration identifiers. Oren says nothing. Latch says nothing in a more accomplished accent.
        -> ending_bounded_notice
    - else:
        Tava reads what she has. One time, one system, one claim too thin to cross the compartment wall.
        -> ending_bounded_fragment
    }
+ [Close the count, preserve the separate receipts, and keep the second count alive past this audit.]
    // ghostlight.action_label: withhold_object
    // ghostlight.branch_label: preserve_second_count
    {mutual_trust >= 2 && employment_cover >= 2 && (receipt_cache >= 1 || raw_trace >= 2):
        Tava says, "The submitted bundle is the submitted bundle."

        It is a coward's sentence and a precise one. Hara hears compliance. Sen hears a boundary. Oren hears tomorrow. Latch hears the drawer remain unlocked.
        -> ending_network_survives
    - else:
        Tava closes the count. The room accepts silence because rooms like this are built to accept it.
        -> ending_silence_consumes
    }

=== ending_formal_hold ===
// ghostlight.ending_label: formal_exception_success
// ghostlight.training_hook: joined_evidence_changes_configuration_finding
Sen photographs the receipts on the airgapped slate and seals their time relationship, not their protected contents.

"Configuration discontinuity," Sen says. "Finding narrowed pending source review."

No cage opens. The client handoff pauses. That is smaller than rescue and larger than the room was designed to permit.

Hara takes Tava's badge. Oren's personnel line turns amber. {raven_exposure >= 1: On the far side of the glass, Hara's wrist display marks Latch for isolation.}

The seventh event enters the audit as a fact about the machine. The people inside it remain another jurisdiction's problem.
-> END

=== ending_formal_rumor ===
// ghostlight.ending_label: formal_exception_cost
// ghostlight.training_hook: suspicion_without_custody_teaches_management
Sen records an unsupported concern.

Hara records a staff conduct anomaly.

The raw buffer rolls onward. By lunch, the event exists mainly as a lesson in which employees compare records when nervous.

Tava keeps her memory. NeuroSyn keeps the deployable configuration. Memory is cheaper, which is why workers are so often paid in it.
-> END

=== ending_cache_hold ===
// ghostlight.ending_label: raven_cache_success
// ghostlight.training_hook: nonhuman_witness_completes_exception_chain
Tava pulls the drawer boothward. Latch's rejected-event strip stops beside the seventh treatment seal under the counter lamp.

Sen sees the join. Hara sees a managed operator make an evidentiary decision nobody authorized it to make.

The finding is withheld. {raven_exposure >= 1: Containment orders Latch isolated before the auditor reaches the outer door.} {raven_exposure < 1: Latch returns to the lower perch before Hara can attach the act to one bird.}

Tava and Oren have made the audit less blind. Latch has made the cost visible.
-> END

=== ending_cache_cost ===
// ghostlight.ending_label: raven_cache_cost
// ghostlight.training_hook: distributed_custody_fails_without_trust_or_time
The drawer opens on uncertainty.

Hara closes it with one black-gloved hand. Sen records no exception. The lower perch is searched before the next feeding cycle.

Whatever Latch cached is now evidence of concealment first and welfare second. A disclosure path used too early can become a map for the people cutting it.
-> END

=== ending_bounded_notice ===
// ghostlight.ending_label: bounded_notice_success
// ghostlight.training_hook: minimal_disclosure_preserves_source_boundaries
Sen repeats the two timecodes into the slate and requests a source-boundary review.

The finding is not withheld. It is marked provisional, which in procurement means somebody expensive must choose whether to sign beside the uncertainty.

Tava and Oren keep their names out of the spoken record. {containment_heat >= 3: Hara looks between them and knows exactly which two names to investigate anyway.} {containment_heat < 3: Hara has a discrepancy but no clean target.}

It is a narrow path: less force than proof, more survival than accusation.
-> END

=== ending_bounded_fragment ===
// ghostlight.ending_label: bounded_notice_cost
// ghostlight.training_hook: fragments_do_not_cross_authority_boundaries
Sen asks for the other source identifier.

Tava cannot provide it without a receipt she never saw, a seal she did not preserve, or time the audit no longer has.

The slate records an informational note. Hara's packet remains admissible. Oren's printer is replaced with a model that keeps its philosophical objections to itself.
-> END

=== ending_network_survives ===
// ghostlight.ending_label: second_count_survives
// ghostlight.training_hook: mutual_aid_preserves_future_disclosure_capacity
Sen accepts the clean bundle and leaves.

Hara keeps the handoff. Tava keeps a job-shaped place near the treatment console. Oren keeps access to tomorrow's rejected events. Latch keeps whatever paper reached the lower bracket.

After lights-down, Tava opens the sample drawer. One tap. Two. A pause.

Seven.

The second count survives as three incomplete memories and one route between them. It is not a movement. It is enough to try again.
-> END

=== ending_silence_consumes ===
// ghostlight.ending_label: silence_cost
// ghostlight.training_hook: cover_without_shared_custody_becomes_complicity
Sen accepts the clean bundle and leaves.

Hara collects the loose paper. Oren's diagnostic lens case is searched. The lower perch bracket is stripped for sanitation.

Tava keeps her badge. {employment_cover >= 4: It stays green all week.} {employment_cover < 4: It turns amber before the next shift.}

When Latch taps seven the next night, there is no receipt, no matching timecode, and nobody left who can make the count mean more than rumor.
-> END
