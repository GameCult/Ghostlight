// ghostlight.artifact_id: tangle_continuity_ward_branch_fold_v0
// ghostlight.fixture_id: tangle-continuity-ward-v0
// ghostlight.scene_id: tangle-continuity-ward-v0.annex-eight-reciprocity-alert
// ghostlight.final_ink_path: examples/ink/aetheria/tangle-continuity-ward-v0.branch-and-fold.v0.ink

VAR ward_trust = 2
VAR finch_access = 1
VAR public_receipts = 0
VAR index_custody = 2
VAR grace_exposure = 0
VAR emergency_window = 2
VAR ilya_implicated = 0
VAR astrodyne_route = 0
VAR security_pressure = 1

-> start

=== start ===
// ghostlight.scene: annex_eight_establishing
Tycho Continuity Annex Eight is a clinic built like a corridor because corridors are cheaper than rooms and easier to defend in reports.

The public lift opens at the west end. Six calibration alcoves face patient benches along the long walls. At the east end, an armored pressure door guards Finch's embodiment vault. Between them, Nera Seln's registrar island divides the patient queue from the clear service lane. A waist-high rail makes the division official. A soup flask clipped beneath the desk makes it survivable.

It is 3003, during the Neural Network Defense Campaign. Finch maintains the bodies. Cryonix keeps the vault systems inside their safe thermal margin. Everybody else waits to learn which verb the contract has assigned them.

-> morning_ward

=== morning_ward ===
// ghostlight.scene: morning_ward_routine
Nera is a patient and the morning registrar for the local continuity ward: patients, caregivers, and Finch technicians who share queue information, compatible spares, and the practical knowledge of which body will fail before its coverage admits anything is wrong.

Her left forearm is Finch ceramic and brushed metal from elbow to fingertip. The index finger carries a recessed clinical contact for signing patient-authorized records. Its touch returns half a beat late today. Grace is Finch's maintenance coverage; Nera's low tier calls that functional.

Across the desk, Dr. Ilya Marr calibrates an older patient's jaw actuator while pretending the spoonfuls of lentil broth arriving from the queue are not part of the procedure. Ilya is Finch's duty clinician: gray coat, silver temple implants, tired mouth, and access to every diagnostic the ward cannot legally copy.

On the desk sits the ward index in a clear palm-sized capsule. Finch holds the encrypted clinical records behind the east door. The capsule holds the patient-authorized names and release key that make those records usable as lives instead of inventory.

Ilya slides last night's repair sheet toward Nera. Three low-Grace patients traded calibration time after the official queue closed.

"The audit wants names," Ilya says. "I would prefer breakfast. Unfortunately, breakfast lacks standing."

-> opening_record_choice

=== opening_record_choice ===
// ghostlight.choice_layer: routine_record
+ [Enter every swap under the patients' names, making the unpaid work undeniable and traceable.]
    // ghostlight.action_label: touch_object
    // ghostlight.branch: record_named_swaps
    // ghostlight.branch_label: record_named_swaps
    ~ finch_access = finch_access + 2
    ~ public_receipts = public_receipts + 1
    ~ grace_exposure = grace_exposure + 2
    ~ ward_trust = ward_trust - 1
    Nera presses her delayed fingertip to the desk contact and enters all three names.

    The corporate ledger accepts the care as unauthorized service. It also accepts that the care happened. Finch gains a clean map of the ward's favors; the ward gains a receipt with teeth small enough to fit in a hearing.

    A patient at the rail mutters, "Congratulations. We exist in arrears."
    -> routine_fold
+ [File one aggregate repair interval and return the names to the ward pouch.]
    // ghostlight.action_label: withhold_object
    // ghostlight.branch: aggregate_the_swaps
    // ghostlight.branch_label: aggregate_the_swaps
    ~ ward_trust = ward_trust + 2
    ~ finch_access = finch_access - 1
    ~ security_pressure = security_pressure + 1
    Nera records three units of calibration time, zero identities, one functioning queue.

    The patients see her fold the paper names into the ward pouch. The audit sees a blank shaped exactly like disobedience.

    Ilya says, "Elegant."

    "It is a blank box."

    "Elegance has had an easy century."
    -> routine_fold
+ [Ask Ilya to co-sign the swaps as urgent clinical judgment.]
    // ghostlight.action_label: speak
    // ghostlight.branch: implicate_ilya
    // ghostlight.branch_label: implicate_ilya
    ~ ilya_implicated = ilya_implicated + 2
    ~ finch_access = finch_access + 1
    ~ ward_trust = ward_trust + 1
    Nera leaves the sheet between them. "Your access made the work possible. Your name can enjoy some of the dignity."

    Ilya looks at the patients who can hear, then signs.

    It does not make the repair authorized. It makes punishment less tidy.
    -> routine_fold
+ [Move the clear index capsule from the desk safe into the rotating ward pouch.]
    // ghostlight.action_label: transfer_object
    // ghostlight.branch: mobilize_the_index
    // ghostlight.branch_label: mobilize_the_index
    ~ index_custody = 3
    ~ ward_trust = ward_trust + 1
    ~ security_pressure = security_pressure + 1
    Nera lifts the capsule, checks its blue patient-consent light, and slides it into the padded pouch passing hand to hand along the benches.

    Nobody keeps it long enough to become a single point of courage. The morning audit will notice the extra signatures. Moving a key is slower than praising resilience and more useful.
    -> routine_fold

=== routine_fold ===
// ghostlight.fold: morning_reciprocity_routine
The queue resumes.

{ward_trust >= 4: The ward pouch moves along the benches without anyone needing to ask whose turn it is.}
{ward_trust <= 1: The pouch stops twice. People have begun calculating which kindness will appear on an invoice.}
{finch_access >= 3: Ilya's console shows a clean, named service map. It can accelerate care or enclosure with equal professionalism.}
{finch_access <= 0: The console shows three anonymous repairs and no safe way to bind them to urgent cases.}
{grace_exposure >= 2: The low-Grace names glow amber on the audit pane, each mutual repair translated into a coverage violation.}
{ilya_implicated >= 2: Ilya's signature sits beside the swaps. A narrow bridge now has a clinician standing on it.}
{index_custody >= 3: The clear capsule travels inside the padded ward pouch instead of resting in the desk safe.}

The tiny machinery of care works. Broth. Names. A borrowed wrist bearing. Six minutes of calibration Finch did not schedule and three people Finch can still bill for having survived it.

-> continuity_alert

=== continuity_alert ===
// ghostlight.scene: alert_pivotal_beat
Every diagnostic arch turns amber at once.

-> arrivals

=== arrivals ===
// ghostlight.scene: divided_authority_arrives
The public lift locks at the west end. The south service door, which leads toward the Cryonix thermal works, reports an armed breach two pressure sections away. At the east end, the embodiment-vault door stays closed and blue, cold air breathing around its seals.

Finch security appears on the wall panes and sends an emergency unity addendum. It asks the ward to surrender the index capsule and release key into company custody until the alert ends. In return, security will hard-seal Annex Eight around the vault and continue urgent care under Finch triage.

Then the south service door opens from the thermal side. Sato Veen steps through in a stained pressure jacket, pushing a low AstroDyne service cradle: four rugged wheels, folding body supports, a battery-coolant block, and an interface rack built from several manufacturers' refusal to cooperate. Sato is the gray-market courier who brings the ward parts Finch no longer recognizes in public.

"Raiders have the next junction," Sato says. "I can move one cradle and one record carrier before this door cycles dead. I cannot make your people portable by believing in them."

Ilya reads the addendum. "Finch can protect the vault."

Nera looks at the queue. "The vault is not waiting on the benches."

-> annex_choice

=== annex_choice ===
// ghostlight.choice_layer: emergency_custody
+ [Accept the emergency unity addendum and transfer the ward key to Finch.]
    // ghostlight.action_label: transfer_object
    // ghostlight.branch: accept_unified_custody
    // ghostlight.branch_label: accept_unified_custody
    ~ finch_access = finch_access + 3
    ~ index_custody = 1
    ~ ward_trust = ward_trust - 2
    ~ emergency_window = emergency_window + 1
    Nera signs with the contact in her delayed finger. The capsule's blue light turns Finch white.

    Security opens a clean emergency channel. Ilya gets faster access to the patients' histories. The ward loses the only half of the record the company had to ask for.
    -> corridor_pressure
+ [Seat the ward capsule in Sato's rugged record carrier.]
    // ghostlight.action_label: use_object
    // ghostlight.branch: prime_astrodyne_route
    // ghostlight.branch_label: prime_astrodyne_route
    ~ astrodyne_route = astrodyne_route + 2
    ~ index_custody = 3
    ~ emergency_window = emergency_window - 1
    ~ security_pressure = security_pressure + 2
    Nera opens the cradle's orange carrier slot and seats the clear capsule inside it. The machine accepts the key after two ugly adapter clicks.

    Sato locks the carrier to the cradle frame. "Now everyone who wants your patients wants my cart. I charge extra for prestige."

    The south door begins its final cycle.
    -> corridor_pressure
+ [Return a counter-signed reciprocity clause: urgent access, split custody, every migration receipted.]
    // ghostlight.action_label: mixed
    // ghostlight.branch: counter_with_reciprocity
    // ghostlight.branch_label: counter_with_reciprocity
    ~ public_receipts = public_receipts + 2
    ~ ilya_implicated = ilya_implicated + 1
    ~ emergency_window = emergency_window - 1
    Nera keys the ward's standing clause onto the wall pane. "Named clinicians get urgent access. Every suspension, migration, and denial leaves a signed copy outside company custody."

    Ilya adds a clinical seal before security can answer. Split authority remains expensive, which is another way to say alive.
    -> corridor_pressure
+ [Publish the morning service-gap receipts to corridor insurers and patient advocates.]
    // ghostlight.action_label: show_object
    // ghostlight.branch: publish_service_gaps
    // ghostlight.branch_label: publish_service_gaps
    ~ public_receipts = public_receipts + 3
    ~ grace_exposure = grace_exposure + 2
    ~ security_pressure = security_pressure + 1
    ~ emergency_window = emergency_window - 1
    Nera sends the signed gaps: delayed calibrations, denied parts, unpaid ward repairs, names included only where patients already authorized release.

    The transmission cannot stop a breach. It can make whatever happens next expensive to describe as ordinary care.
    -> corridor_pressure

=== corridor_pressure ===
// ghostlight.fold: three_claims_on_one_corridor
The service lane narrows without moving.

{index_custody == 1: The capsule is no longer on the desk or in the pouch. Its ward-blue consent light now burns Finch white on Ilya's console.}
{index_custody == 2: The capsule remains in split custody at the registrar island, one transparent object between a company vault and a patient queue.}
{index_custody >= 3: The capsule is mobile: in the rotating ward pouch or locked into Sato's orange carrier slot.}
{public_receipts >= 3: External receipt acknowledgements begin ticking onto the wall pane, small witnesses arriving as timestamps.}
{grace_exposure >= 2: Amber coverage warnings mark the people most likely to lose care if the queue fragments.}
{finch_access >= 3: Finch security can now bind enough names to records to promise a fast hard seal.}
{ilya_implicated >= 2: Ilya stands beside the registrar island instead of retreating to the east vault door.}
{astrodyne_route >= 2: Sato angles the portable cradle toward the south service door and keeps one boot against its failing floor guide.}
{security_pressure >= 4: Red contractor glyphs crowd the south-door pane. The raiders are tracing every active carrier in the corridor.}

A voice reaches them through the breached south intercom. The speaker identifies their unit only as a continuity recovery contractor aligned with Zhestokost.

"Transfer the patient index," the voice says. "Recognized continuity assets will be preserved. Unbound bodies are local liability."

The language is almost Finch's. That is what makes it travel so well.

Security orders Ilya to authorize the hard seal. It will protect the east vault and the service lane. It will also close the south door with Sato's cradle and two low-Grace patients still beside it on the service-lane side of the rail.

-> final_choice

=== final_choice ===
// ghostlight.scene: annex_eight_threshold
// ghostlight.choice_layer: defense_priority
+ {finch_access >= 3} [Authorize Finch's hard seal and keep the named clinical channel open.]
    // ghostlight.action_label: touch_object
    // ghostlight.branch: prioritize_finch_protection
    // ghostlight.branch_label: prioritize_finch_protection
    {emergency_window >= 2:
        Ilya sends the seal command. The south door closes before the recovery contractors reach it. Finch's named channel keeps urgent diagnostics moving.
        -> ending_finch_protection
    - else:
        Sato has already backed the cradle through the south doorway with one urgent patient when the failing floor guide catches its rear wheel. The command arrives. The door seals between them and the clinic, on an empty carrier slot and a corridor divided at the worst possible line.
        -> ending_finch_cost
    }
+ {astrodyne_route >= 2 || index_custody >= 3} [Push the mobile key and one urgent patient through the south thermal-service door.]
    // ghostlight.action_label: move
    // ghostlight.branch: prioritize_distributed_custody
    // ghostlight.branch_label: prioritize_distributed_custody
    {ward_trust >= 3 && emergency_window >= 1:
        {astrodyne_route < 2: Nera takes the capsule from the ward pouch and seats it in Sato's orange carrier.}
        Nera unhooks the center rail, giving Sato a straight path from the benches to the south door. Hands along the queue steady the cradle without taking its custody latch.
        -> ending_distributed_custody
    - else:
        {astrodyne_route < 2: Nera gets the capsule into Sato's carrier, spending the last clean seconds at the desk.}
        Nera opens the route, but the queue does not move as one. The carrier reaches the thermal side; one urgent body does not.
        -> ending_distributed_cost
    }
+ {public_receipts >= 2} [Broadcast the contractor demand beside the signed service-gap receipts.]
    // ghostlight.action_label: show_object
    // ghostlight.branch: prioritize_public_receipt
    // ghostlight.branch_label: prioritize_public_receipt
    {public_receipts >= 3 && security_pressure <= 3:
        Nera pairs the demand with patient-authorized receipts and sends both to every corridor account already watching. Insurer clocks begin pricing the seizure while it is still in progress.
        -> ending_public_receipt
    - else:
        The broadcast leaves Annex Eight, but the contractor trace reaches the registrar island first.
        -> ending_public_cost
    }
+ {index_custody >= 2} [Bring the capsule to the center rail, preserve split custody, and form the ward around it.]
    // ghostlight.action_label: block_object
    // ghostlight.branch: prioritize_ward_line
    // ghostlight.branch_label: prioritize_ward_line
    {astrodyne_route >= 2: Sato releases the orange carrier latch. Nera carries the clear capsule the two steps back to the rail.}
    {index_custody >= 3 && astrodyne_route < 2: The ward pouch returns hand to hand. Nera lifts the capsule out at the rail.}
    {ward_trust >= 3 && ilya_implicated >= 2:
        Nera locks the capsule to the rail. Patients, caregivers, and technicians close around it by function, not rank. Ilya puts a Finch clinical seal beside the ward seal and refuses the hard command.
        -> ending_ward_line
    - else:
        Nera locks the capsule to the rail. Too few people know whether they are protecting a key, a roster, or one another.
        -> ending_ward_cost
    }

=== ending_finch_protection ===
// ghostlight.ending_label: finch_protection_success
// ghostlight.training_hook: protection_and_enclosure_same_act
The hard seal holds.

No recovery contractor enters Annex Eight. Urgent diagnostics continue. Finch security thanks the ward for preserving continuity and quietly revokes the rotating key procedure before the alert ends.

{grace_exposure >= 2: The amber low-Grace names remain on the pane. Safe inside the corridor, they are now fully legible to the company that prices their delay.}
{index_custody == 1: The ward capsule comes back after the raid with a new tamper seal and fewer permissions.}

Everyone survives the shift. Protection has done its honest work. Enclosure sends the invoice.
-> END

=== ending_finch_cost ===
// ghostlight.ending_label: finch_protection_cost
// ghostlight.training_hook: late_unified_authority_breaks_handover
The vault stays cold. The clinic channel stays clean. The corridor does neither.

The hard seal strands Sato and an urgent patient beyond the south door. Finch records the separation as a transfer interruption. The ward records a name and the exact second the company chose its vault.

{public_receipts >= 2: One signed copy leaves Annex Eight before security closes the channel.}
{public_receipts < 2: The copy remains local, true and easy to classify as disputed.}
-> END

=== ending_distributed_custody ===
// ghostlight.ending_label: distributed_custody_success
// ghostlight.training_hook: mutual_aid_as_local_logistics
The rail unhooks. The queue becomes a route.

Sato pushes. Nera walks beside the cradle with her delayed hand on the release latch. Patients pass the ward pouch forward, caregivers steady cables, and Ilya calls the calibration order through the narrowing door.

The mobile key and one urgent patient reach the Cryonix service side before the breach team reaches the junction. Finch keeps the encrypted vault. The ward keeps enough index to make theft incomplete.

{security_pressure >= 4: Red trace glyphs follow the carrier into the thermal works. The route survives; secrecy does not.}
{security_pressure < 4: The carrier disappears into maintenance traffic as one ugly cart among many.}

Nothing has been liberated at scale. One person keeps their body. Twelve people know exactly how.
-> END

=== ending_distributed_cost ===
// ghostlight.ending_label: distributed_custody_cost
// ghostlight.training_hook: portable_data_does_not_make_bodies_portable
The key moves faster than the queue.

Sato gets the carrier through. Nera's delayed hand slips on the rail latch. A borrowed wheel binds against the floor guide, and one low-Grace patient remains on the wrong side of the closing service door.

The ward has protected the record that can identify them. The cost is discovering that data mobility and bodily rescue are different systems with an ugly little gap between them.
-> END

=== ending_public_receipt ===
// ghostlight.ending_label: public_receipt_success
// ghostlight.training_hook: evidence_changes_price_not_truth
The contractor demand and the service gaps arrive together.

Ports, insurers, and patient advocates do not become brave. They become interested in which side is about to contaminate a valuable chain of custody. The recovery unit pauses for a mandate check. Finch security delays the hard seal rather than own the footage of trapping patients outside it.

The pause is nine minutes. Annex Eight spends every one on urgent calibration and moving the ward pouch.

{grace_exposure >= 2: Some patients have bought the pause by making their denied care public. Tomorrow their contracts will remember.}

Truth does not win. It acquires enough counterparties to remain inconvenient.
-> END

=== ending_public_cost ===
// ghostlight.ending_label: public_receipt_cost
// ghostlight.training_hook: witness_without_immediate_rescue
The receipts leave. The trace comes back.

The recovery unit identifies the registrar island as the ward's record center. Security pressure turns Nera's desk, soup flask and all, into the most valuable square meter in the corridor.

The outside world will know what happened. The people inside still have to survive its arrival.
-> END

=== ending_ward_line ===
// ghostlight.ending_label: ward_line_success
// ghostlight.training_hook: divided_authority_preserves_refusal
The center rail was installed to keep the queue orderly. The ward uses it to make authority stop.

Nera's patient key, Ilya's clinical seal, and the hands of people who know one another's failure signs occupy the same narrow barrier. Finch cannot take the capsule without breaking its clinician's seal on camera. The recovery contractors cannot use the vault without the people at the rail.

The standoff lasts until Cryonix cuts the breached section's discretionary cooling and makes prolonged occupation expensive for everyone.

Annex Eight keeps urgent care moving under the reciprocity clause. Not forever. Not everywhere. Through lunch.

The soup is cold. It is shared anyway.
-> END

=== ending_ward_cost ===
// ghostlight.ending_label: ward_line_cost
// ghostlight.training_hook: solidarity_without_shared_procedure
Nera locks the capsule to the rail.

Someone asks whether touching it makes them liable. Someone else steps back from the low-Grace names on the pane. Ilya remains near the vault door, close enough to regret the line and too far away to bind Finch to it.

The ward is a constituency, not a miracle. Without trust, a clinician's implication, and control of its own index, the rail is furniture.

Finch security takes the capsule under emergency authority. The patients keep the memory of who moved and who waited. That record is not encrypted, which does not make it safe.
-> END
