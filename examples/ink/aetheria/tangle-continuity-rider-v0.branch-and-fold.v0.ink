// ghostlight.artifact_id: tangle_continuity_rider_branch_fold_v0
// ghostlight.fixture_id: tangle-continuity-rider-v0
// ghostlight.scene_id: tangle-continuity-rider-v0.transfer-wheel-six-galley
// ghostlight.tonal_mode: workplace gallows comedy with procedural dread and domestic warmth
// ghostlight.final_ink_path: examples/ink/aetheria/tangle-continuity-rider-v0.branch-and-fold.v0.ink

VAR mutual_cover = 2
VAR evidence_integrity = 1
VAR filter_reserve = 2
VAR bond_status = 2
VAR tsi_leverage = 1
VAR horizon_pressure = 1
VAR archive_access = 0
VAR work_stoppage = 0
VAR privacy_cost = 0
VAR seal_risk = 2
VAR crew_fatigue = 2
VAR rider_entrenched = 0

-> start

=== start ===
Transfer Wheel Six turns at Earth–Luna L1 with the stately confidence of a machine whose creditors have never stood inside it.

The galley occupies twelve meters of the wheel's spin deck. Its curved outer-hull floor carries a fraction of Earth gravity. Six bolted tables fill the center. A serving counter runs along the forward bulkhead. The port bulkhead holds a hull-status panel and a sealed door to the maintenance corridor. At the aft end, frosted glass encloses Tactical Solutions International's two-chair resilience booth. A starboard vestibule gives management and security their own entrance, because hierarchy dislikes arriving through the same door as breakfast.

It is 2092, one year after Horizon Ventures' convoy failures cost its lenders much of their sense of humor. Filter mesh is rationed. The next bonded resupply ship is due in eleven hours.

-> breakfast_routine

=== breakfast_routine ===
// ghostlight.scene: ordinary_life_before_pressure
Inez Ramires serves mushroom broth after the night maintenance shift. She is the elected rota delegate, which sounds grand until someone learns the office comes with a ladle and no additional oxygen.

Sima Ko, seal technician and keeper of the galley's unofficial repair ledger, sits at table three. The ledger is a grease-marked pad kept outside Horizon's personnel system. It records failed seals, traded shifts, borrowed meal chits, and who sat with whom after a bad alarm. Officially it is a menu draft. It has been a menu draft for seven pages.

"Cup fourteen still leaks," Sima says.

"That makes it management material."

Inez puts the cup inside a second cup. Transfer Wheel Six has solved another systems problem through consolidation.

Beyond the frosted booth glass, Dr. Leena Aro lays out breathing cards and clean towels. She is TSI's continuity assessor. Her sessions really can settle a shaking hand. Her reports can also decide whether that hand remains cleared for work.

On the hull-status panel, seal bay C-12 shows green. Sima's ledger says it has been losing pressure under load for six days.

-> routine_choice

=== routine_choice ===
// ghostlight.choice_layer: ordinary_shift_preparation
+ [Copy Sima's C-12 work order into the galley ledger before breakfast traffic begins.]
    // ghostlight.action_label: write_object
    // ghostlight.branch_label: prime_mechanical_evidence
    ~ evidence_integrity = evidence_integrity + 2
    ~ mutual_cover = mutual_cover + 1
    ~ horizon_pressure = horizon_pressure + 1
    Inez writes the pressure-loss times between MUSHROOM STOCK and CUP FOURTEEN.

    Sima adds the maintenance ticket number, then smears broth over the page corner. The stain is accidental. Its usefulness is not.

    The galley ledger now disagrees with Horizon's green panel in ink two people can recognize.
    -> routine_fold
+ [Take the first resilience appointment and ask Aro who can open the session archive.]
    // ghostlight.action_label: speak
    // ghostlight.branch_label: prime_tsi_relationship
    ~ tsi_leverage = tsi_leverage + 2
    ~ archive_access = archive_access + 1
    ~ crew_fatigue = crew_fatigue - 1
    Inside the booth, Aro gives Inez a warm towel and counts four breaths without once saying optimize.

    "If I dispute a finding," Inez asks, "who opens the recording?"

    Aro's eyes move toward the booth's locked archive dock. "TSI, under claim review. Horizon can request a classification. It cannot rewrite the session."

    "So you keep the hostage."

    "We call it independent custody."

    Neither laughs. The towel is still warm.
    -> routine_fold
+ [Patch the galley return grille, then split the last citrus gel among the dock crew.]
    // ghostlight.action_label: distribute_object
    // ghostlight.branch_label: prime_mutual_cover
    ~ mutual_cover = mutual_cover + 2
    ~ filter_reserve = filter_reserve - 1
    ~ crew_fatigue = crew_fatigue - 1
    Inez opens one emergency filter packet. Sima fits the patch across the whining return grille below the serving counter. The air loses its metal taste by degrees too small for management to invoice.

    Then Inez cuts the gel into eight translucent slivers and puts one on each bowl.

    Citrus is not medicine. It does, however, remind eight tired people that taste exists and management has not yet patented it.

    Sima eats her sliver slowly while writing the patch time on the galley ledger. Care is inefficient like that. It keeps becoming evidence.
    -> routine_fold
+ [Approve an early return to the loading rota so the resupply bond stays on schedule.]
    // ghostlight.action_label: authorize
    // ghostlight.branch_label: prime_bond_continuity
    ~ bond_status = bond_status + 1
    ~ crew_fatigue = crew_fatigue + 1
    ~ mutual_cover = mutual_cover - 1
    ~ seal_risk = seal_risk + 1
    Inez signs six names back onto the loading rota.

    The schedule turns green. Sima looks at it, then at Inez, then drinks from cup fourteen without the second cup.

    It leaks down her sleeve with tremendous moral clarity.
    -> routine_fold

=== routine_fold ===
// ghostlight.fold: breakfast_state_carries_forward
The serving line opens. Workers enter through the port corridor in pairs, mag-soled boots ticking on the curved floor. They trade meal chits and complaints. Aro takes appointments in the frosted booth. The hull-status panel continues its difficult career in optimism.

{evidence_integrity >= 3: The galley ledger lies open at C-12, grease, broth, ticket number, and two witnesses sharing one page.}
{mutual_cover >= 4: Citrus slivers move bowl to bowl. Nobody has enough; nobody eats alone.}
{mutual_cover <= 1: The tables fill in separate islands. The schedule is intact. The room is not.}
{crew_fatigue <= 1: A few hands have stopped shaking after food, heat, and Aro's breathing count.}
{crew_fatigue >= 3: A spoon falls at table five. Three people flinch as if gravity has filed a complaint.}
{bond_status >= 3: The resupply departure stays green on the schedule rail.}
{filter_reserve <= 1: Only one sealed packet of emergency filter patches remains under the serving counter.}

The routine lasts nine minutes. This is respectable for a routine owned by four institutions.

-> pressure_arrival

=== pressure_arrival ===
// ghostlight.scene: contract_pressure_enters
The hull-status panel flashes amber. BAY C-12: LOAD DEVIATION. The port maintenance door seals. The serving line stops with six bowls still empty.

Manager Oren Vey enters through the starboard vestibule with two Horizon security contractors behind him. His collar is immaculate. This means someone else has been crawling through his station.

Leena Aro leaves the booth carrying a TSI slate. On its screen waits the Continuity Assurance Rider: Horizon keeps its insurance discount and bonded resupply priority while TSI classifies each shift as behaviorally stable enough for safe work. If workers revoke access to the session archive, the shift becomes an unverified interval. Insurers may then suspend the discount until they can price the ignorance.

Vey puts the incident form beside Inez's soup pot.

"Collective fatigue response," he says. "Sign as rota delegate. Aro confirms recovery. We reopen C-12, preserve the departure window, and nobody spends eleven hours explaining themselves to a lunar creditor."

Sima looks at the green history on the hull panel. "The seal was bad before we got tired."

Vey does not look at her. "The panel disagrees."

-> incident_choice

=== incident_choice ===
// ghostlight.choice_layer: incident_classification
+ {evidence_integrity >= 3} [Set the galley ledger beside the incident form and declare a mechanical stop.]
    // ghostlight.action_label: show_object
    // ghostlight.branch_label: contest_with_repair_record
    ~ evidence_integrity = evidence_integrity + 1
    ~ work_stoppage = work_stoppage + 1
    ~ horizon_pressure = horizon_pressure + 2
    ~ seal_risk = seal_risk - 1
    Inez turns the ledger so Vey can read the six-day pressure sequence.

    "Ticket number, load times, two signatures. C-12 is mechanical until someone opens it."

    Vey studies the soup stain. "This is a galley pad."

    "Then your maintenance system has been beaten by lunch. Open the bay."

    Security shifts toward the port door. Sima stays seated, which is harder.
    -> insurer_fold
+ {tsi_leverage >= 3} [Ask Aro to open a claim review before Vey can classify the shift.]
    // ghostlight.action_label: speak
    // ghostlight.branch_label: invoke_tsi_custody
    ~ archive_access = archive_access + 2
    ~ tsi_leverage = tsi_leverage - 1
    ~ horizon_pressure = horizon_pressure + 1
    ~ privacy_cost = privacy_cost + 1
    "Leena. You said Horizon cannot rewrite the sessions. Open mine under claim review."

    Vey says, "Doctor Aro is our contractor."

    Aro holds his gaze. "My finding is your contractor. My archive is TSI's."

    It is not solidarity. It is jurisdictional vanity wearing a clean badge. Today it stands in a useful place.
    -> insurer_fold
+ [Lift the crew clearance badges from the roster reader and place them beneath the soup pot.]
    // ghostlight.action_label: move_objects
    // ghostlight.branch_label: create_unverified_interval
    ~ work_stoppage = work_stoppage + 2
    ~ bond_status = bond_status - 2
    ~ mutual_cover = mutual_cover + 1
    ~ horizon_pressure = horizon_pressure + 2
    Inez removes the six dock badges one by one. The roster reader turns amber. She slides the badges under the hot soup pot where security can retrieve them if security is willing to scald its dignity.

    The schedule marks the shift unverified.

    Vey goes pale. The Rider has stopped being a policy and become a missing departure slot.
    -> insurer_fold
+ [Sign only "fatigue observed; mechanical cause unresolved."]
    // ghostlight.action_label: write_object
    // ghostlight.branch_label: preserve_bond_narrowly
    ~ bond_status = bond_status + 1
    ~ rider_entrenched = rider_entrenched + 1
    ~ privacy_cost = privacy_cost + 1
    ~ horizon_pressure = horizon_pressure - 1
    Inez crosses out COLLECTIVE RESPONSE and writes MECHANICAL CAUSE UNRESOLVED in the margin.

    Aro initials the change. Vey signs beneath both of them because the resupply clock is now more persuasive than rank.

    The shift remains classifiable. So does every person in it.
    -> insurer_fold

=== insurer_fold ===
// ghostlight.fold: insurers_need_a_record
The hull-status panel divides. On the left, C-12 pulses amber beside a pressure trace. On the right, lunar shipping underwriter Yara Dast appears from an office one short signal hop and several layers of blame away.

Dast introduces herself before asking why Horizon's insured loading shift has stopped. She represents the underwriters financing the next resupply departure. She does not own Transfer Wheel Six. She merely owns the conditions under which anyone will risk approaching it with cargo.

Vey points to the TSI Rider. Aro points to the unresolved finding. Sima points to the maintenance ticket. Inez notices nobody points to the six empty bowls.

{horizon_pressure >= 4: Vey stands between the starboard vestibule and the roster reader, physically blocking security from touching the badges until he knows which version of events costs less.}
{bond_status <= 0: The resupply schedule has turned red. The wheel has hours of reserve, not minutes, but every room can now feel the price of delay.}
{bond_status >= 3: The resupply schedule remains green, giving Horizon leverage to call the stoppage unnecessary.}
{archive_access >= 2: Aro's slate shows an OPEN CLAIM REVIEW seal over the locked session archive.}
{privacy_cost >= 1: Inez's own session identifier glows beneath that seal.}
{work_stoppage >= 2: Six clearance badges steam gently beneath the soup pot. The scene is ridiculous. The stoppage is not.}
{seal_risk >= 3: The C-12 pressure trace drops again while everyone is watching. The panel's green history acquires the posture of a liar caught mid-sentence.}
{rider_entrenched >= 1: The amended fatigue finding is already feeding the Rider's renewal record.}

Dast says, "I can order an independent seal inspection, preserve the bond on a narrow data release, accept TSI's continuity finding, or suspend cover for an unverified shift. I need to know which record can survive being disliked."

-> final_choice

=== final_choice ===
// ghostlight.choice_layer: choose_whose_record_travels
+ [Transmit the duplicated C-12 log and keep the shift stopped until the seal is opened.]
    // ghostlight.action_label: transmit_object
    // ghostlight.branch_label: prioritize_mechanical_truth
    {evidence_integrity >= 3 && work_stoppage >= 1:
        Inez photographs the galley page with Vey, Aro, Sima, and the amber panel in one frame. Dast receives a repair record with witnesses and an active refusal attached.
        -> ending_mechanical_review
    - else:
        Inez transmits the fragments she has. Dast receives suspicion, not a chain.
        -> ending_mechanical_exposure
    }
+ [Offer one bounded session excerpt if Sima is named co-custodian of the release.]
    // ghostlight.action_label: negotiate_custody
    // ghostlight.branch_label: prioritize_bounded_data
    {archive_access >= 2 && tsi_leverage >= 2 && mutual_cover >= 2:
        Inez authorizes ninety seconds: Aro's breathing count, Inez naming the bad seal, and the timestamp that predates the alarm. Sima's key is required for any replay.
        -> ending_bounded_archive
    - else:
        Inez offers a boundary the contract has not been forced to recognize. Vey accepts the data and forgets the boundary with professional speed.
        -> ending_archive_capture
    }
+ [Accept TSI's continuity finding and protect the next resupply departure.]
    // ghostlight.action_label: authorize
    // ghostlight.branch_label: prioritize_insured_supply
    {bond_status >= 3 && seal_risk <= 2:
        Inez signs the amended finding. Dast leaves the bond green. Vey orders C-12 reopened under reduced load.
        -> ending_supply_preserved
    - else:
        Inez signs, but the unverified interval has already reached the underwriting model.
        -> ending_supply_mislabeled
    }
+ [Withdraw archive consent together and run the wheel from the galley ledger until inspection.]
    // ghostlight.action_label: collective_withhold
    // ghostlight.branch_label: prioritize_local_mutual_refusal
    {mutual_cover >= 4 && filter_reserve >= 1:
        Inez asks each dock worker, one by one. Six refusals enter the roster. Sima closes the galley ledger over the spare filter packet.
        -> ending_mutual_interval
    - else:
        Inez asks for a collective refusal. The room does not have enough trust or reserve left to make the word collective true.
        -> ending_refusal_fracture
    }

=== ending_mechanical_review ===
// ghostlight.ending_label: mechanical_review_won
// ghostlight.training_hook: worker_record_forces_insurer_action
Dast preserves the resupply bond for six hours and orders C-12 opened under remote witness.

The outer seal has a heat-warped retaining ring. It was a mechanical fault before it became a mood.

Vey loses the incident classification. Aro keeps the archive. TSI keeps the contract, because one correct finding is excellent sales material. The crew gets the seal replaced and no apology.

At breakfast the next shift, cup fourteen still leaks. Sima writes RETAINING RING beneath it on the menu draft. The ledger has learned which institutions can be made to read.
-> END

=== ending_mechanical_exposure ===
// ghostlight.ending_label: mechanical_claim_insufficient
// ghostlight.training_hook: evidence_without_chain_becomes_worker_risk
Dast orders the bay held but suspends the bond pending inspection. Horizon calls the delay a delegate intervention.

Security takes the galley pad. Sima has copied only half the ticket sequence. The seal remains closed, the resupply clock turns red, and Inez's name becomes the cleanest object in the incident file.

The next meal is thinner. Nobody pretends the missing calories are a lesson in resilience.
-> END

=== ending_bounded_archive ===
// ghostlight.ending_label: bounded_data_custody_won
// ghostlight.training_hook: contractor_rivalry_creates_worker_custody_seam
Aro and Sima turn their keys together. Ninety seconds leave the booth. The rest stays sealed.

Dast hears Inez report C-12 six hours before Horizon records collective fatigue. She preserves the bond and orders a mechanical review. TSI wins recognition as an independent custodian. The workers win a second key, which is less than ownership and much more than they had at breakfast.

Vey studies the two-key release clause as if contract punctuation has developed labor politics.
-> END

=== ending_archive_capture ===
// ghostlight.ending_label: archive_boundary_failed
// ghostlight.training_hook: privacy_spent_without_custody
The excerpt leaves the booth. Then Vey requests the adjoining five minutes for context. Dast accepts them.

Inez's breathing, anger, joke about cup fourteen, and private estimate of Sima's exhaustion become underwriting evidence. C-12 is inspected eventually. The session archive has already acquired another owner in practice.

Aro folds the warm towels with careful hands. Real care has been entered into discovery.
-> END

=== ending_supply_preserved ===
// ghostlight.ending_label: insured_continuity_preserved
// ghostlight.training_hook: material_supply_preserved_at_contract_cost
The resupply departure remains bonded. Filter mesh, food, and replacement valves arrive eleven hours later.

C-12 runs at reduced load. Nobody dies. That matters.

The insurer records a successfully managed fatigue event. Horizon renews TSI across its transfer operations. The next Rider makes resilience attendance a condition of rota eligibility and cites Inez's amended finding as proof the process works.

The wheel is safer tonight and harder to leave tomorrow.
-> END

=== ending_supply_mislabeled ===
// ghostlight.ending_label: continuity_claim_too_late
// ghostlight.training_hook: concession_without_material_return
{seal_risk >= 3 && bond_status >= 3:
The bond stays green long enough for the loading arm to take pressure. C-12 drops again. The resupply ship aborts its approach, the seal closes hard, and Vey still files the event as collective fatigue while the retaining ring cools behind a locked door.

The crew keeps the bond on paper and loses the cargo in space. Aro's booth opens for mandatory recovery appointments at 0600.
- else:
The signature reaches underwriting after the unverified interval.

Dast suspends the discount. Vey still files the incident as collective fatigue, because a failed concession remains useful to management if it can explain who failed.

The crew loses the bond and the argument. Aro's booth opens for mandatory recovery appointments at 0600.
}
-> END

=== ending_mutual_interval ===
// ghostlight.ending_label: local_mutual_refusal_holds
// ghostlight.training_hook: quiet_mutual_aid_sustains_bounded_exit
The roster turns red. The archive records six separate refusals and cannot reduce them to one unstable delegate.

Dast suspends the discount but keeps the resupply approach on a manual bond after Sima transmits the repair ledger. The price rises. Horizon will collect it later. For six hours, however, workers control the C-12 rota and inspect the seal under their own paired watch.

They eat cold broth, patch one filter from the galley reserve, and copy the ledger page three times. It is not a movement. It is eight people making sure the next person does not face the contract alone.
-> END

=== ending_refusal_fracture ===
// ghostlight.ending_label: local_refusal_fractures
// ghostlight.training_hook: solidarity_requires_material_reserve
Two workers refuse. Two ask whether the next convoy will still carry their children's filter allotments. Two look at Inez and wait for an answer she does not have.

Vey collects four signatures for return to work. TSI classifies the remaining pair as an acute support case. The bond stays uncertain and the crew's disagreement becomes data.

At table three, Sima closes the galley ledger. A shared record cannot manufacture a reserve that was spent before anyone knew they would need to say no.
-> END
