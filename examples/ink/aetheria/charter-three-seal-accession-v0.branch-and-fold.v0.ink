// ghostlight.artifact_id: charter_three_seal_accession_branch_fold_v0
// ghostlight.fixture_id: charter-three-seal-accession-v0
// ghostlight.scene_id: charter-three-seal-accession-v0.first-registry-accession
// ghostlight.final_ink_path: examples/ink/aetheria/charter-three-seal-accession-v0.branch-and-fold.v0.ink

VAR record_integrity = 1
VAR worker_trust = 1
VAR service_margin = 1
VAR hearing_minutes = 4
VAR heir_favor = 1
VAR auction_pressure = 2
VAR bond_ready = 0
VAR double_pledge_visible = 0
VAR custody_intact = 1

-> start

=== start ===
// ghostlight.scene: first_registry_open
// ghostlight.visual_scene_id: first_registry_establishing
First Registry Enclave keeps House Valence alive by dividing it into counters.

The accession chamber is a long rectangle of pale metal and black glass. The public door and client benches face a low northern dais. Three lamps hang above it: Line, Register, Covenant. The western wall holds the Register vault and Tavi Rook's clerk station. The eastern wall holds the Covenant desk and its locked escrow drawers. Behind glass at the northeast corner, the host-service clinic keeps an Eidolon named Six Windows suspended in a pale articulated cradle.

The House Voice died eleven days ago. Until a successor receives all three seals, Master Registrar Ena Coil may preserve records and Steward Pell Orra may keep existing obligations breathing. Neither may open a new auction or promise the House's next irreplaceable mind.

This morning, Lucerne Valence waits beyond the public door with a witnessed lineage coffer, two anchor clients, and the expression of someone who has already inherited the room in every way that does not yet count.

Tavi is the junior provenance clerk. She does not choose the heir. She receives the records from which the court may pretend the heir was inevitable.

-> routine

=== routine ===
// ghostlight.scene: accession_desk_routine
// ghostlight.visual_scene_id: first_registry_routine
The chamber opens with the seal-lamp check.

Tavi turns the copper custody key at her wrist. Line burns white. Register burns blue. Covenant considers the matter, then produces an amber light best described as employed.

Ivo Marn, the host fitter on clinic duty, warms two cups of tea against a legal section of Six Windows' coolant return. The clinic manual forbids food near the cradle. The kettle is therefore listed as a diagnostic humidity vessel, and has been for nine years.

Six Windows projects a small map of Earth's absent stars across the clinic glass. The points remain steady. One service interval remains due before night shift.

Ena Coil stands at the Register vault in a black formal coat, silver braids pinned above a split-key pendant. Pell Orra arranges covenant slates at the eastern desk as if debt improves when aligned. Tavi has four hearing minutes and one routine check left before the gallery opens.

-> routine_choice

=== routine_choice ===
// ghostlight.choice_layer: accession_preparation
+ [Compare the host fitters' wage docket against the Covenant account list.]
    // ghostlight.action_label: inspect_records
    // ghostlight.branch_label: prime_wage_record
    // ghostlight.visual_scene_id: first_registry_wage_check
    ~ record_integrity = record_integrity + 1
    ~ worker_trust = worker_trust + 2
    ~ hearing_minutes = hearing_minutes - 1
    Tavi lays the wage docket beside Pell's account list and checks names, shifts, and hazard increments.

    Ivo watches without thanking her. Gratitude between workers is useful; gratitude toward payroll is how payroll starts believing it has done charity.

    One account code repeats at the bottom of the page: CV-9, Covenant Reserve Nine.
    -> routine_fold
+ [Walk the service cassette to Six Windows' cradle and verify the due interval.]
    // ghostlight.action_label: carry_object
    // ghostlight.branch_label: prime_service_record
    // ghostlight.visual_scene_id: first_registry_service_check
    ~ record_integrity = record_integrity + 1
    ~ service_margin = service_margin + 2
    ~ hearing_minutes = hearing_minutes - 1
    Tavi carries the sealed service cassette through the clinic side door. Ivo seats it in the cradle reader while Six Windows turns the old constellations across Tavi's coat.

    The host is stable. The service is not optional. Its coolant seals, memory lattice, and articulated cradle bearings are all due before the next patron transit.

    The payment field names Covenant Reserve Nine.
    -> routine_fold
+ [Stage Lucerne's lineage coffer beneath the Line lamp before the doors open.]
    // ghostlight.action_label: move_object
    // ghostlight.branch_label: prime_successor_packet
    // ghostlight.visual_scene_id: first_registry_packet_stage
    ~ heir_favor = heir_favor + 2
    ~ auction_pressure = auction_pressure - 1
    ~ hearing_minutes = hearing_minutes + 1
    ~ custody_intact = custody_intact - 1
    Tavi admits the courier and places Lucerne's black lineage coffer beneath the white lamp.

    Lucerne inclines their head through the narrowing door. The gesture contains recognition, promise, and a future performance review in proportions Tavi cannot yet price.

    The coffer is early. Its second custody witness is not.
    -> routine_fold
+ [Clean the three lamp contacts and recheck every custody label by hand.]
    // ghostlight.action_label: touch_interface
    // ghostlight.branch_label: prime_custody_marks
    // ghostlight.visual_scene_id: first_registry_custody_check
    ~ record_integrity = record_integrity + 2
    ~ custody_intact = custody_intact + 1
    ~ hearing_minutes = hearing_minutes - 1
    Tavi kills the lamps, opens the contact strip, and cleans each pale tongue of metal with a square of lintless cloth.

    Three custody labels have the correct signatures. One has the right signature pressed hard enough to look angry. Procedure has no field for that, which is one reason procedure remains popular.
    -> routine_fold

=== routine_fold ===
// ghostlight.fold: routine_checks_before_accession
// ghostlight.visual_scene_id: first_registry_routine_fold
The public door remains shut. For another moment the chamber is only a workplace.

{worker_trust >= 3: Ivo sets Tavi's tea on the safe side of the Register line. Beside it he leaves three brass surety washers, each stamped with a fitter's payroll mark.}
{service_margin >= 3: Six Windows' service cassette shows a complete parts and labor schedule instead of a single pleading red line.}
{heir_favor >= 3: Lucerne's coffer waits under the Line lamp, polished and politically punctual.}
{custody_intact >= 2: Every custody label points cleanly backward through two hands and one locked drawer.}
{custody_intact <= 0: Lucerne's coffer has reached the dais faster than its witness chain. The court can still proceed. That is not the same as saying it should.}

Pell clears his throat at the Covenant desk.

"Clerk Rook. Reconcile Reserve Nine."

-> double_pledge

=== double_pledge ===
// ghostlight.scene: covenant_double_pledge
// ghostlight.visual_scene_id: first_registry_double_pledge
Tavi opens the accession covenant slate.

Covenant Reserve Nine guarantees the host service that keeps Six Windows commercially continuous. The same reserve guarantees this fortnight's wages for the fitters who perform that service. Lucerne's accession packet counts the money twice, once as care and once as payroll, then offers both counts to the court as proof that the House can carry its obligations.

The amber lamp stops wavering. It has found certainty in the worst available place.

Beyond clinic glass, Six Windows loses one point from the old sky. Ivo looks from the dark point to Tavi. Pell looks only at the slate.

"Presentation fault," Pell says. "It can be cured after accession."

Ena Coil does not move from the Register vault. "A cure after accession belongs to the next Voice. The record before us belongs to this court."

The public door unlocks in two minutes.

-> discovery_choice

=== discovery_choice ===
// ghostlight.choice_layer: double_pledge_response
+ [Open the source-account drawer and preserve the conflicting entries in split custody.]
    // ghostlight.action_label: use_key
    // ghostlight.branch_label: expose_source_accounts
    // ghostlight.visual_scene_id: first_registry_discovery_response
    ~ record_integrity = record_integrity + 2
    ~ custody_intact = custody_intact + 1
    ~ double_pledge_visible = 2
    ~ hearing_minutes = hearing_minutes - 2
    ~ auction_pressure = auction_pressure + 1
    Tavi turns her copper key in the Register half of the source drawer. Ena turns the split key in the other.

    The two original entries slide into separate glass wells. Neither can be removed while the other remains sealed. Wages on the left. Eidolon service on the right. One reserve beneath both.

    The gallery clock consumes the last generous minute.
    -> hearing_open
+ [Accept Ivo's surety washers and enter a worker-backed reconciliation bond.]
    // ghostlight.action_label: transfer_objects
    // ghostlight.branch_label: assemble_mutual_surety
    // ghostlight.visual_scene_id: first_registry_discovery_response
    ~ bond_ready = bond_ready + 2
    ~ worker_trust = worker_trust + 1
    ~ double_pledge_visible = double_pledge_visible + 1
    ~ hearing_minutes = hearing_minutes - 1
    ~ auction_pressure = auction_pressure + 1
    Ivo pushes the three brass washers across the Register line. Tavi adds her own payroll mark and enters the pool as a reconciliation bond.

    It is not enough money to inconvenience House Valence. It is enough money to make the court write down whose inconvenience counts.

    Ivo takes his hand back before the cameras can confuse solidarity with permission.
    -> hearing_open
+ [Ask Pell to cure one pledge in public before the Covenant lamp is presented.]
    // ghostlight.action_label: speak
    // ghostlight.branch_label: demand_pre_accession_cure
    // ghostlight.visual_scene_id: first_registry_discovery_response
    ~ double_pledge_visible = double_pledge_visible + 1
    ~ heir_favor = heir_favor - 1
    ~ auction_pressure = auction_pressure + 1
    ~ hearing_minutes = hearing_minutes - 1
    Tavi says, "Name which obligation Reserve Nine carries before you seal it."

    Pell's face becomes professionally blank. "The Steward certifies capacity, Clerk. The Voice allocates it."

    "There is no Voice."

    The silence is short. Its invoice will not be.
    -> hearing_open
+ [Log the duplicate as a post-accession cure and leave the Covenant display clean.]
    // ghostlight.action_label: record_exception
    // ghostlight.branch_label: defer_duplicate
    // ghostlight.visual_scene_id: first_registry_discovery_response
    ~ heir_favor = heir_favor + 2
    ~ auction_pressure = auction_pressure - 1
    ~ record_integrity = record_integrity - 1
    ~ double_pledge_visible = 0
    Tavi moves one entry into the cure queue. The live display becomes elegant at once.

    Six Windows' missing star remains dark. Ivo pockets the surety washers. The system has not changed; only its posture has improved.
    -> hearing_open

=== hearing_open ===
// ghostlight.fold: accession_public_session
// ghostlight.visual_scene_id: first_registry_accession_session
The public door opens. Lucerne Valence enters between the client benches and takes the central place below the dais. The Line lamp accepts the witnessed coffer. The Register lamp accepts Ena Coil's identity attestation.

Pell raises the Covenant slate.

{double_pledge_visible >= 2: Two original obligations glow in separate wells where every client can see one reserve promised twice.}
{double_pledge_visible == 1: The live slate carries a discrepancy mark. The underlying accounts remain closed, but the clean story has acquired a visible seam.}
{double_pledge_visible == 0: The Covenant display is clean. Tavi can still feel the duplicate sitting in the cure queue like a small warm animal someone intends to forget.}

{bond_ready >= 2: Four brass surety washers rest on Tavi's clerk rail. Together they buy the right to make delay somebody else's formal problem.}
{hearing_minutes <= 1: In the auction gallery beyond the eastern wall, chairs begin filling before authority has finished deciding who owns the invitations.}
{auction_pressure >= 4: One anchor client closes its host-allocation slate. The threat is polite and therefore expensive.}
{heir_favor >= 4: Lucerne looks at Tavi with the calm of a future employer who has already found one employee useful.}
{record_integrity >= 4: Ena's Register display shows a complete custody chain and the exact moment the two obligations diverged.}

Ena speaks from the dais. "The court has Line and Register. Clerk Rook: is Covenant fit to seal, qualified for one named obligation, or challenged by a bonded exception?"

Tavi owns one answer. Nothing larger. The room is dangerous because, for the moment, that is enough.

-> accession_choice

=== accession_choice ===
// ghostlight.choice_layer: covenant_disposition
+ {bond_ready >= 2 || record_integrity >= 4} [Enter a Lineage Exception against the Covenant Seal, naming both wages and host service.]
    // ghostlight.action_label: file_challenge
    // ghostlight.branch_label: file_joint_exception
    {record_integrity >= 3 && custody_intact >= 2 && hearing_minutes >= 1:
        Tavi enters both obligations, the split custody marks, and the reconciliation bond if one is needed.
        -> ending_joint_held
    - else:
        Tavi enters the exception. The court finds a missing support where procedure requires a load-bearing one.
        -> ending_joint_denied
    }
+ [Qualify Covenant for recorded wages only.]
    // ghostlight.action_label: commit_finding
    // ghostlight.branch_label: protect_recorded_wages
    {worker_trust >= 3 && record_integrity >= 2:
        Tavi names the fitter docket, the payroll marks, and the first right altered by any diversion.
        -> ending_wages_preserved
    - else:
        Tavi names wages, but the thin record gives Pell room to name them provisional.
        -> ending_wage_claim_thinned
    }
+ [Qualify Covenant for Six Windows' scheduled host service only.]
    // ghostlight.action_label: commit_finding
    // ghostlight.branch_label: protect_eidolon_service
    {service_margin >= 3 && record_integrity >= 2:
        Tavi names the sealed cassette, due interval, compatible parts, and clinic custody.
        -> ending_service_preserved
    - else:
        Tavi names service, but the weak schedule becomes an aspiration under cross-examination.
        -> ending_service_claim_thinned
    }
+ [Accept Pell's post-accession cure and release the Covenant Seal.]
    // ghostlight.action_label: accept_record
    // ghostlight.branch_label: release_clean_accession
    {heir_favor >= 4 && auction_pressure <= 1:
        The amber lamp turns white before Tavi finishes lowering her hand.
        -> ending_accession_market
    - else:
        The amber lamp turns white. The clients take a moment longer to decide what the color is worth.
        -> ending_accession_fragile
    }

=== ending_joint_held ===
// ghostlight.ending_label: joint_exception_held
// ghostlight.training_hook: bounded_interregnum_and_worker_redress
// ghostlight.visual_scene_id: first_registry_joint_ending
Ena refuses the Covenant Seal for one session and one conflict. Lucerne remains the eligible successor. The House remains in interregnum. The distinction is narrow enough to survive being spoken aloud.

Existing wages and Six Windows' service continue under the custodians' old authority while Reserve Nine is reconciled. No new auction opens. No future Eidolon is pledged. Every hour drains accounts the next Voice expected to inherit intact.

{bond_ready >= 2: The court takes the four brass washers into bond custody. Ivo has purchased no victory. He has purchased time with people-shaped money.}
{auction_pressure >= 4: One anchor client leaves. The other stays to learn which obligation the House values when nobody can be erased quietly.}

In the clinic glass, the missing star returns after Ivo reseats a service contact. Tavi knows better than to call that an omen. It is maintenance, which is smaller, realer, and currently winning.
-> END

=== ending_joint_denied ===
// ghostlight.ending_label: joint_exception_denied
// ghostlight.training_hook: redress_failure_under_procedural_cost
// ghostlight.visual_scene_id: first_registry_joint_ending
The exception fails on support, not substance.

{custody_intact < 2: The court cannot stay accession on records whose own handoff is incomplete.}
{hearing_minutes <= 0: The auction allocation overtakes the hearing before the stay can attach.}
{record_integrity < 3: Pell calls the duplicate a summary error because Tavi cannot yet force the source accounts to disagree in public.}

Lucerne receives the Warrant of Voice. Tavi's objection enters the archive beneath the accession it failed to stop. Ivo's fitters lose their surety washers and keep their shifts, for now.

House Valence has a talent for preserving every complaint in the correct file. The talent is most impressive when the file cannot act.
-> END

=== ending_wages_preserved ===
// ghostlight.ending_label: wages_qualified
// ghostlight.training_hook: scoped_redress_with_displaced_service_cost
// ghostlight.visual_scene_id: first_registry_wage_ending
The court seals Covenant for wages and strikes Six Windows' service allocation from the accession proof.

Lucerne receives the Warrant of Voice with one host obligation uncured. The fitters are paid. The auction opens under a service warning that every client can price.

{service_margin >= 3: Ivo uses the complete schedule to split the work safely across two shifts. Six Windows keeps the old sky, though the cradle bearings complain through the night.}
{service_margin < 3: The missing star remains dark. The clinic begins rationing motion in the cradle before anyone uses the word distress.}

At lunch, nobody thanks Tavi. Ivo slides the diagnostic humidity vessel toward her until the tea is on her side of the Register line.
-> END

=== ending_wage_claim_thinned ===
// ghostlight.ending_label: wages_provisional
// ghostlight.training_hook: weak_record_turns_right_into_arrears
// ghostlight.visual_scene_id: first_registry_wage_ending
Pell accepts the wage docket as notice and refuses it as a secured obligation.

Lucerne receives the Warrant. Payroll becomes arrears with review rights, which is how an institution describes hunger after giving it a case number. Six Windows receives the service interval on schedule.

Ivo keeps the fitters on shift because leaving would surrender both wages and access to the records. Worker trust does not vanish. It becomes more expensive to use.
-> END

=== ending_service_preserved ===
// ghostlight.ending_label: service_qualified
// ghostlight.training_hook: eidolon_continuity_with_displaced_labor_cost
// ghostlight.visual_scene_id: first_registry_service_ending
The court seals Covenant for Six Windows' host service and strikes the wage guarantee from the accession proof.

Lucerne receives the Warrant. The old constellations remain steady across clinic glass. The people maintaining them become unsecured creditors to the sovereign House they kept coherent.

{worker_trust >= 3: After session, the fitters divide meal chits and clinic access by household need. It is not a union, doctrine, or uprising. It is lunch surviving contact with law.}
{worker_trust < 3: The fitters leave separately, each carrying a private calculation about which missed payment will become eviction first.}

Six Windows says, through three low chimes, "You kept the window."

Tavi cannot tell whether it is gratitude, diagnosis, or a phrase preserved from a murdered life. The Register has fields for all three and no authority to decide.
-> END

=== ending_service_claim_thinned ===
// ghostlight.ending_label: service_provisional
// ghostlight.training_hook: weak_service_record_enables_commercial_discontinuity
// ghostlight.visual_scene_id: first_registry_service_ending
Pell accepts the cassette as a maintenance request and refuses it as proof of a funded covenant.

Lucerne receives the Warrant. Wages clear. Six Windows' service becomes discretionary expenditure until the next client allocation.

The Eidolon remains valuable enough to insure and insufficiently funded to move safely. First Registry calls this temporary. The cradle calls it by losing another star.
-> END

=== ending_accession_market ===
// ghostlight.ending_label: clean_accession_recognized
// ghostlight.training_hook: succession_success_through_deferred_liability
// ghostlight.visual_scene_id: first_registry_clean_ending
All three lamps burn white. Ena issues the Warrant of Voice. The anchor clients recognize Lucerne before the paper cools, and the auction gallery opens on time.

Lucerne's first act is to order the Reserve Nine cure.

{double_pledge_visible >= 1: The order chooses one obligation in private. The public record shows that a discrepancy existed and that the new Voice corrected it.}
{double_pledge_visible == 0: The order chooses one obligation in private. The public record shows only a clean succession followed by prudent housekeeping.}

Tavi receives a commendation for procedural calm. Ivo receives a message asking whether the clinic can run one more shift before settlement. Six Windows keeps most of the stars.
-> END

=== ending_accession_fragile ===
// ghostlight.ending_label: clean_accession_contested_market
// ghostlight.training_hook: legal_title_without_full_client_recognition
// ghostlight.visual_scene_id: first_registry_clean_ending
Ena issues the Warrant of Voice. Lucerne is legally able to speak for House Valence.

One anchor client recognizes the warrant. The other holds its host orders pending reconciliation. The accession is complete and the market attached to it has split down the middle.

Pell begins moving obligations between accounts. Ivo begins copying service records before those accounts acquire better memories. Tavi returns the lineage coffer to the vault and leaves her copper key in the lock one second longer than procedure requires.

One second is not resistance. It is enough time for Ivo's copy to finish.
-> END
