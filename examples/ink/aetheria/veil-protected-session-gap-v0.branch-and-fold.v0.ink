// ghostlight.artifact_id: veil_protected_session_gap_branch_fold_v0
// ghostlight.fixture_id: veil-protected-session-gap-v0
// ghostlight.scene_id: veil-protected-session-gap-v0.stillwater-six-review
// ghostlight.final_ink_path: examples/ink/aetheria/veil-protected-session-gap-v0.branch-and-fold.v0.ink

VAR route_receipt = false
VAR counter_receipt = false
VAR coworker_cover = false
VAR audit_hold = false
VAR clinic_suspicion = 1
VAR auditor_attention = 1
VAR privacy_intact = 2
VAR buffer_window = 2
VAR sel_control = 2
VAR jon_trust = 1
VAR public_witnesses = 0

-> start

=== start ===
Stillwater Six sells calm between transport connections. The Framgång clinic occupies one crescent of an Enceladus transfer habitat: six pale recovery rooms facing a carpeted waiting bay, with a narrow service spine behind them where the walls are honest about being walls.

Esi Tan resets Room C after the night shift. She folds the blanket into the clinic's approved shape, a leaf no plant has ever consented to resemble. She changes the scent wafer, wipes the neural-contact couch, and logs one cup of sweet mineral water as if cups are a category of spiritual progress.

Esi is a hospitality reset worker. Her job is to make other people's recovery look as though it happened without labor.

-> routine_people

=== routine_people ===
Jon Alis rolls a filter case along the service spine. He maintains NeuroSyn interfaces and possesses the rare industrial gift of knowing which green lights mean danger. The waist-high service counter beside Room C shows four reward pulses, one safety interlock event, and a scheduled purge at the next maintenance tick.

"Busy room," he says.

Esi's route receipt says the room was hers. Her supervisor ordered the session after she refused a third consecutive double shift. Framgång calls it employer-funded recovery. Payroll calls the lost hour unpaid rest.

There is a square bruise behind Esi's left ear where the contact crown sat. She remembers saying no. She also remembers how wonderfully unimportant no became for several minutes afterward.

-> routine_authority

=== routine_authority ===
Dr. Sel Varo, Stillwater Six's alignment lead, crosses the public bay arranging his face into patient weather. He asks Esi for the route receipt. Routine paperwork goes back to the clinic, where routine paperwork has a short and unusually healthy life.

Deka Morn, another reset worker, stacks towels at the staff alcove and watches Sel through the brushed-steel reflection of a drinks cabinet. Deka cannot see the maintenance counter. Jon cannot see Esi's route order. Neither can open the protected clinical buffer.

Beyond the curved entry glass, insurer auditor Kira Doss waits for the clinic to admit her. She can inspect the signed shell around a protected session. She is forbidden to inspect the session itself without Esi's consent or an emergency finding.

The clinic chime says *welcome* in a tone designed by someone who had never needed entry-level wages.

-> routine_choice

=== routine_choice ===
// ghostlight.choice_layer: routine_evidence_custody
+ [Tuck the route receipt into the clean-linen seam before Sel reaches the cart.]
    // ghostlight.action_label: conceal_object
    // ghostlight.branch_label: keep_route_receipt
    ~ route_receipt = true
    ~ clinic_suspicion = clinic_suspicion + 1
    Esi lifts the top blanket, slides the thin route foil under its stitched hem, and smooths the leaf back into corporate botany.

    Sel sees her hand leave the linen. He sees nothing he can name without admitting he watches cleaners hide things.
    -> routine_fold
+ [Help Jon reseat the filter and ask him to print the service counter before it clears.]
    // ghostlight.action_label: use_object
    // ghostlight.branch_label: keep_counter_receipt
    ~ counter_receipt = true
    ~ jon_trust = jon_trust + 1
    ~ buffer_window = buffer_window + 1
    Esi braces the filter case while Jon seats the new cartridge. The task takes two people because Framgång bought the one-person installation tool and assigned it to a better clinic.

    Jon prints the maintenance counter on a narrow service strip. Four pulses. One interlock. Purge pending. No thoughts, no therapy, no patient content.

    He leaves the strip beneath the case handle where a reset worker might find it accidentally and a camera might call it work.

    While Esi's hands are on the filter case, Sel takes the unhidden route foil from the cart clip and stamps it archived.
    -> routine_fold
+ [Ask Deka to cover the bay for ten minutes and say exactly why.]
    // ghostlight.action_label: speak
    // ghostlight.branch_label: make_coworker_witness
    ~ coworker_cover = true
    ~ public_witnesses = public_witnesses + 1
    ~ clinic_suspicion = clinic_suspicion + 1
    "Sel put me in C after I refused the double," Esi says. "If I become inspirational, throw a towel at me."

    Deka does not laugh. She moves Esi's unfinished rooms onto her own slate, which is more expensive than laughing.

    Sel collects the route foil from Esi's open hand while Deka accepts the second rota.
    -> routine_fold
+ [Hand Sel the route receipt and finish the room exactly to checklist.]
    // ghostlight.action_label: comply
    // ghostlight.branch_label: preserve_routine_cover
    ~ sel_control = sel_control + 1
    ~ clinic_suspicion = clinic_suspicion - 1
    Esi gives him the receipt.

    Sel stamps it archived. Esi squares the cup with the painted edge of the side table. The room becomes perfect enough to deny anything happened in it.

    Jon looks at the service counter, then at Esi. He does not rescue her with an idea she did not ask for.
    -> routine_fold

=== routine_fold ===
// ghostlight.fold: routine_before_audit
Kira enters through the curved public doors. Her audit jacket has no clinic logo, which is how the Pan-Solar Consortium expresses intimacy.

Sel meets her at the white floor arc marking the clinical privacy line. On his side: Room C, Esi's cart, Jon's service panel, and the staff alcove. On Kira's side: a portable seal tray and the limited dignity of an invited auditor.

{route_receipt:
The clean-linen seam presses a straight line against Esi's palm. A supervisor order is waiting inside a blanket.
}
{counter_receipt:
The service strip rests beneath Jon's filter-case handle. Four pulses and one interlock have become small enough to lose in a pocket.
}
{coworker_cover:
Deka works Esi's rooms as well as her own. Mutual aid begins, like many durable institutions, with somebody inheriting the worst part of your rota.
}
{clinic_suspicion >= 3:
Sel watches the cart, Jon's case, and Deka's slate in turn. Calm is becoming difficult to staff.
}
{sel_control >= 3:
Sel holds Esi's archived route receipt between two fingers while he welcomes the auditor.
}

-> audit_shell

=== audit_shell ===
Sel places the signed disposition in Kira's seal tray.

"Voluntary recovery session," he says. "No escalation. Normal completion. The patient has returned to duty."

The shell shows authorization, start time, stop time, aggregate safety markers, and Sel's signature. It does not show the supervisor order. It does not show four reward pulses. It does not show Esi's refusal becoming briefly irrelevant.

Kira reads the same five fields twice. "I am not authorized to cross the privacy line."

"That," Sel says warmly, "is why patients trust us."

Esi understands the shape of the trap. If Kira demands everything, the clinic can call the audit a raid on patient minds. If she asks for nothing else, Sel's summary becomes the only lawful story in the room.

-> audit_response_choice

=== audit_response_choice ===
// ghostlight.choice_layer: answer_the_clean_shell
+ {route_receipt} [Place the hidden route receipt in Kira's seal tray, then step back across the privacy line.]
    // ghostlight.action_label: transfer_object
    // ghostlight.branch_label: submit_route_receipt
    ~ auditor_attention = auditor_attention + 2
    ~ clinic_suspicion = clinic_suspicion + 1
    ~ sel_control = sel_control - 1
    The foil lies beside Sel's disposition. Ordered attendance. Unpaid rest. Supervisor authorization.

    Kira does not touch Esi. She seals the tray around both documents.

    Sel's smile survives. It has been through training.
    -> audit_response_fold
+ {counter_receipt} [Ask Jon, in front of Kira, what the service strip counts.]
    // ghostlight.action_label: speak
    // ghostlight.branch_label: name_counter_anomaly
    ~ auditor_attention = auditor_attention + 1
    ~ public_witnesses = public_witnesses + 1
    ~ sel_control = sel_control - 1
    "Four reward pulses," Jon says. "One safety interlock. Those are hardware events. I cannot see why they happened."

    "No escalation," Kira reads from Sel's disposition.

    "Then the couch became ambitious on its own."

    Jon's jokes are usually worse. This one has evidence.
    -> audit_response_fold
+ [Ask Kira to state, for the sealed record, exactly what the audit cannot see.]
    // ghostlight.action_label: speak
    // ghostlight.branch_label: expose_audit_boundary
    ~ auditor_attention = auditor_attention + 1
    ~ privacy_intact = privacy_intact + 1
    ~ buffer_window = buffer_window - 1
    Kira looks at Esi before answering.

    "I can see that a protected session occurred. I can see its administrative shell. I cannot see prompt content, neural response, operator commands, or the patient's moment-to-moment affect without patient consent or an emergency finding."

    The answer spends time. It also puts the missing shape into the record without filling it with Esi's mind.
    -> audit_response_fold
+ [Keep resetting the bay and let Sel's disposition sit alone for another minute.]
    // ghostlight.action_label: wait
    // ghostlight.branch_label: wait_under_clean_shell
    ~ clinic_suspicion = clinic_suspicion - 1
    ~ sel_control = sel_control + 1
    ~ buffer_window = buffer_window - 1
    Esi wipes a table already clean.

    Kira asks about the aggregate safety marker. Sel explains normality in three compatible fonts.

    Waiting protects Esi from becoming the exhibit. It also gives the purge clock another minute to become history.
    -> audit_response_fold

=== audit_response_fold ===
// ghostlight.fold: audit_boundary_named
Behind Room C, the maintenance counter changes from amber to white.

Jon reads it first. "Next tick clears the protected buffer."

Sel turns toward him. "As required."

{privacy_intact >= 3:
Kira's sealed record now names the fields she is forbidden to demand. Privacy is visible as a boundary, not an excuse she has to accept on Sel's authority.
}
{auditor_attention >= 3:
Kira moves the seal tray closer to Esi's side of the arc. The motion is small, procedural, and unmistakably an invitation.
}
{sel_control <= 1:
Sel stops using the word *patient*. He calls Esi *staff* instead, preparing the disciplinary version of events.
}
{sel_control >= 3:
Sel keeps one hand beside the purge control as if deleting a record were a form of bedside manner.
}

-> purge_order

=== purge_order ===
Sel authorizes the scheduled purge.

The protected buffer belongs to Esi for purposes of consent, to Framgång for purposes of custody, and to nobody for long enough to be useful by accident.

Jon can freeze it only by opening a maintenance exception under his credential. Kira can request a hold only if Esi asks her to. Esi can let the raw session die and still try to prove the shell false from records that reveal no thought.

The white indicator begins to contract around the event marks on the service counter.

-> purge_choice

=== purge_choice ===
// ghostlight.choice_layer: preserve_or_release_buffer
+ [Ask Jon to open a maintenance safety exception and freeze the purge.]
    // ghostlight.action_label: speak
    // ghostlight.branch_label: freeze_with_maintenance
    ~ counter_receipt = true
    ~ audit_hold = true
    ~ buffer_window = buffer_window + 2
    ~ jon_trust = jon_trust + 1
    ~ clinic_suspicion = clinic_suspicion + 2
    Jon presses his credential to the service counter. The white ring stops shrinking.

    His name appears beside **UNRESOLVED INTERLOCK — PRESERVE LOCAL STATE**.

    "There," he says. "Now my employability is part of the evidence."
    -> disclosure_threshold
+ [Ask Kira to hold the buffer under Esi's authority, sealed and unopened.]
    // ghostlight.action_label: authorize
    // ghostlight.branch_label: patient_limited_hold
    ~ audit_hold = true
    ~ buffer_window = buffer_window + 1
    ~ auditor_attention = auditor_attention + 1
    ~ clinic_suspicion = clinic_suspicion + 1
    "Hold it," Esi says. "Do not open it."

    Kira repeats the limits into her slate. Sel objects to the hold, the patient, the wording, and time in roughly that order.

    The white ring stops. Privacy has acquired a witness and an expiration time.
    -> disclosure_threshold
+ [Let the raw buffer purge. Keep the session private and make the other records carry the accusation.]
    // ghostlight.action_label: withhold_object
    // ghostlight.branch_label: permit_private_purge
    ~ buffer_window = 0
    ~ privacy_intact = privacy_intact + 1
    ~ sel_control = sel_control + 1
    The white ring closes.

    Four pulses, Sel's commands, Esi's softened refusal: gone from the clinic's live buffer.

    Esi feels relief before she feels anger at the relief. Her private injury will not become an auditor's demonstration. Now the receipts have to do work usually assigned to exposed people.
    -> disclosure_threshold
+ [Tap Jon's service code on the cart rail: copy counters, let content clear.]
    // ghostlight.action_label: signal
    // ghostlight.branch_label: copy_counter_purge_content
    ~ counter_receipt = true
    ~ jon_trust = jon_trust + 1
    ~ buffer_window = 0
    ~ privacy_intact = privacy_intact + 1
    ~ clinic_suspicion = clinic_suspicion + 1
    Esi taps twice on the cart rail and once on the filter case. Jon's gaze drops.

    He prints the counter as the white ring closes. Hardware events survive. Session content does not.

    Sel sees two workers understand each other and discovers a category his forms do not price well.
    -> disclosure_threshold

=== disclosure_threshold ===
// ghostlight.fold: choose_disclosure_path
The clinic bay holds four possible truths, each owned badly.

{buffer_window > 0:
The protected session remains sealed behind a live hold. Esi may open it, limit it, or let it expire.
- else:
The protected session is gone. Nobody can replay Esi's mind. Nobody can use it to prove Sel's commands either.
}

{audit_hold:
Kira's sealed hold marker links her tray to Room C without moving the session stream across the white arc.
}

{route_receipt:
The route receipt can prove ordered attendance and unpaid rest.
- else:
Sel's archive holds the route receipt. The clinic can describe attendance as voluntary unless another worker contradicts it.
}

{counter_receipt:
The maintenance strip can prove four reward pulses, one interlock, and a purge. It cannot say what they meant.
- else:
The service counter has cleared. Jon remembers the numbers, but his memory is easier to price as resentment.
}

{coworker_cover:
Deka remains in the bay, doing two rotas and refusing to become scenery.
}
{public_witnesses >= 2:
The waiting workers and pilgrims have begun watching the white floor arc instead of the calming wall loop.
}
{clinic_suspicion >= 4:
Sel has summoned contract security to the curved entry doors. The room has not become safer. It has become better staffed.
}
{jon_trust >= 2:
Jon keeps his filter case open beside Esi's cart. There is room inside for copies.
}

Esi must choose what the audit receives and what she keeps.

-> final_choice

=== final_choice ===
// ghostlight.choice_layer: disclosure_path
+ [Authorize release of the raw session stream.]
    // ghostlight.action_label: disclose_object
    // ghostlight.branch_label: disclose_raw_session
    ~ privacy_intact = 0
    {buffer_window > 0:
        Esi opens the hold. Kira receives Sel's operator commands, four reward escalations, the interlock, and the moment Esi's refusal stops changing the system's behavior.
        -> ending_raw_proof
    - else:
        Esi gives consent to a buffer that no longer exists.
        -> ending_raw_lost
    }
+ [Authorize only the route receipt and maintenance counter as one joined finding.]
    // ghostlight.action_label: authorize
    // ghostlight.branch_label: disclose_join_only
    {route_receipt && counter_receipt && auditor_attention >= 2:
        Kira seals the two records together. Ordered attendance meets undeclared reward pulses. Neither record contains Esi's thoughts.
        -> ending_join_success
    - else:
        Kira seals what exists. The join has a missing side or an auditor who has not been given enough cause to hold it open.
        -> ending_join_thin
    }
+ [Name Sel's order aloud in the public waiting bay.]
    // ghostlight.action_label: speak
    // ghostlight.branch_label: disclose_public_testimony
    {public_witnesses >= 1 && sel_control <= 2:
        Esi steps over the white arc and makes the private administrative fact public: the session followed a refused double shift and was ordered by Sel.
        -> ending_public_witness
    - else:
        Esi names the order into a room Sel still controls.
        -> ending_public_reframed
    }
+ [Withhold the session from the audit and split the surviving receipts among workers.]
    // ghostlight.action_label: transfer_object
    // ghostlight.branch_label: defer_to_worker_custody
    {jon_trust >= 2 || coworker_cover:
        Esi asks Jon and Deka to split whatever survives. No one person will hold enough to expose her alone.
        -> ending_mutual_aid
    - else:
        Esi keeps what remains on her own body and in her own pocket.
        -> ending_isolated
    }

=== ending_raw_proof ===
// ghostlight.ending_label: raw_session_proof
// ghostlight.training_hook: direct_truth_at_privacy_cost
The lie dies quickly once Kira can replay it.

Sel calls Esi's refusal agitation. The interface answers by making agreement rewarding. The safety interlock fires. Sel orders another pulse. His signed *no escalation* sits beside the commands like a man attending his own disciplinary hearing.

Stillwater Six loses its clean audit. Esi loses the right to decide who will know how relief felt when it was used against her. By the next shift, three managers who cannot remember her surname have reviewed her most obedient minute.

The truth is complete. Completeness has an appetite.
-> END

=== ending_raw_lost ===
// ghostlight.ending_label: raw_session_lost
// ghostlight.training_hook: consent_after_pruning
Kira records Esi's consent and the buffer's absence.

Sel expresses regret that routine retention protected patient privacy before the patient understood her needs. The sentence is so well aligned it should receive a bonus.

Esi has named the harm, but the system can now file her account against Sel's disposition as a disagreement in recollection. Jon's memory and Deka's anger remain outside the audit unless someone builds a second path.

The white floor arc keeps shining. It has survived everything except relevance.
-> END

=== ending_join_success ===
// ghostlight.ending_label: joined_nonclinical_proof
// ghostlight.training_hook: disclosure_without_mind_exposure
Kira seals the route receipt to the maintenance strip.

Ordered attendance. Unpaid rest. Four reward pulses. One interlock. A signed claim of *voluntary, uneventful care*.

"I do not need her thoughts to ask why your records cannot all be true," Kira tells Sel.

The raw buffer may remain sealed or may already be gone. The contradiction survives in records owned by different hands. Kira opens the Review beyond Stillwater Six because one clinic can explain an anomaly and six clinics can only standardize it.

Jon loses his vendor credential before dinner. Deka trades away two rest periods to cover transit credit to a cooperative repair bench off the pilgrimage route. Esi keeps her session private and helps carry the filter case.

The help is small. It works.
-> END

=== ending_join_thin ===
// ghostlight.ending_label: joined_proof_incomplete
// ghostlight.training_hook: partial_disclosure_under_split_custody
Kira seals a route without a counter, or a counter without a route.

Sel supplies the missing explanation. Mandatory attendance was a scheduling convention. Reward pulses were routine personalization. The safety interlock was conservative. Each sentence fits the empty space designed for it.

Kira marks the session for comparison against another site. Esi keeps her privacy and loses the immediate finding.

The evidence is not false. It is lonely.
-> END

=== ending_public_witness ===
// ghostlight.ending_label: public_worker_witness
// ghostlight.training_hook: testimony_becomes_shared_fact
"He ordered the session after I refused the double," Esi says.

{coworker_cover:
Deka answers first. "She told me before the audit. I took her rooms."
- else:
Jon answers first. "The counter did not match the disposition."
}

{counter_receipt:
Jon raises the service strip.
- else:
Jon shows his empty hands. "The counter cleared. I saw four pulses and an interlock before it did."
}

A pilgrim in a borrowed retreat robe lowers their calming headset. Then another hospitality worker names a recovery appointment charged against rest time.

The audit becomes larger than one protected buffer. Sel can challenge Esi's recollection; he cannot put the waiting bay back into private custody.

Security removes Esi from shift. Deka finishes neither rota. For one evening the clinic has spotless rooms, no reset workers, and an unusually honest waiting time.
-> END

=== ending_public_reframed ===
// ghostlight.ending_label: public_testimony_reframed
// ghostlight.training_hook: isolated_disclosure_becomes_symptom
Esi names the order.

Sel does not deny it. He asks Kira to observe that a distressed employee is disclosing protected treatment material in a public care space. Security approaches with soft hands and a privacy screen.

The waiting bay looks away because looking is recorded as participation. Kira writes down the allegation. Sel writes down the episode.

By morning, the clinic's account is longer.
-> END

=== ending_mutual_aid ===
// ghostlight.ending_label: distributed_worker_custody
// ghostlight.training_hook: quiet_mutual_aid_disclosure_path
The audit leaves Stillwater Six with Sel's disposition intact.

{counter_receipt:
Jon carries the counter copy in the false bottom of his filter case.
- else:
Jon carries the event count in memory and leaves space in the false bottom for the next strip.
}

{route_receipt:
Deka carries the route order behind her own shift slate.
- else:
Deka carries the doubled rota and Esi's account of who ordered it on her own shift slate.
}

Esi carries neither. The arrangement is not called a cell, a movement, or a historic first. It is three workers refusing to make one frightened person the single point of failure.

{route_receipt && counter_receipt:
Weeks later, a cooperative neural-repair clinic receives the same four-pulse pattern from another Framgång site. The two complete halves meet without either patient's session content.
- else:
What they hold is not enough. Weeks later, a cooperative neural-repair clinic sends another partial mismatch from a different Framgång site. Their lonely record has acquired a neighbor.
}

The Review acquires a question it cannot unask.
-> END

=== ending_isolated ===
// ghostlight.ending_label: private_but_isolated
// ghostlight.training_hook: privacy_without_shared_custody
Esi keeps what remains hers and says no to the audit.

That no remains hers. Nothing in the room converts it into evidence, diagnosis, or consent. Sel's disposition survives beside it.

On the next shift, Esi discovers her care eligibility under review and her third double already assigned. A secret can protect a person. It cannot cover a rota by itself.

She folds another blanket into the approved leaf and leaves one corner wrong, where Deka will notice.
-> END
