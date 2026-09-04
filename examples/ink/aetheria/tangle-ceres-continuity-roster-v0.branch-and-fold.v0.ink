// ghostlight.artifact_id: tangle_ceres_continuity_roster_branch_fold_v0
// ghostlight.fixture_id: tangle-ceres-continuity-roster-v0
// ghostlight.scene_id: tangle-ceres-continuity-roster-v0.collarfour-roster-hearing
// ghostlight.final_ink_path: examples/ink/aetheria/tangle-ceres-continuity-roster-v0.branch-and-fold.v0.ink

VAR record_integrity = 2
VAR crew_endurance = 3
VAR safety_margin = 3
VAR production_hold = 2
VAR au_concession = 1
VAR forge_authentication = 1
VAR solex_case = 2
VAR psc_classification = 1
VAR replacement_window = 2
VAR mutual_aid = 1
VAR roster_public = 0
VAR local_custody = 2
VAR claimshare_exposure = 2

-> start

=== start ===
// ghostlight.scene: collarfour_continuity_desk_open
// ghostlight.visual_scene_id: collarfour_establishing
Ceres Bloom Eighteen keeps its Transfer Collar Four between the rotating city and the nonrotating freight hub. The bay turns with the city, so boots and cups settle toward its grated outward deck; beyond the hubward iris, apparent gravity falls toward the transfer tube. Every crate, pressure report, and bad decision crosses that frame boundary, though only the crates are charged by mass.

The continuity desk occupies a steel bay beside the collar lock. Outward, a layered pressure collar curves around the frame-transfer bearing behind bundled coolant pipes and a pressure manifold. Inward, a grated ramp rises to the crew corridor. Hubward, a round freight iris opens onto the low-gravity transfer tube. Three waist-high consoles face one another around a scarred meal table: union handovers, AU safety exceptions, and SolEx certification history. Orbital Forge repair annotations live on a portable slate because Forge considers fixed furniture an early symptom of government.

Len Rusk, pressure mechanic and shift delegate, is making tea in a dented thermal flask. This is not in the maintenance-continuity roster. Everyone has agreed not to report the omission.

-> routine

=== routine ===
// ghostlight.scene: ordinary_continuity_shift
// ghostlight.visual_scene_id: collarfour_routine
The strike is eleven days old. The roster on the wall has two columns: AIR and ORE. AIR still has names beside it. ORE does not.

Len and the union crew keep pressure, cooling, fire isolation, rescue access, and the freight iris safe. They do not load SolEx ore or certify production. That distinction keeps their neighbors breathing and gives every lawyer in the Belt a fresh place to stand.

Their claimshares - project-linked pay and access credits - remain frozen while production is shut. Air must be maintained now. Rent has preserved its freedom of schedule.

Pax Ader, an Orbital Forge cavity planner, checks a forked seal controller against handwritten repair annotations. Vela Quist, the AU Ramp Administrator, releases replacement parts one crate at a time and calls this neutrality. Imri Senn, the SolEx throughput auditor, sits at the certified-baseline console with the patient expression of a person waiting for reality to become admissible.

The remote PSC observer will open the day's classification window in twelve minutes. If the record shows bounded labor action, the strike bond holds and neutral docks remain available. If it shows uninsured disruption, replacement freight freezes. If it shows corridor-breaking sabotage, SolEx can invoke emergency custody over the collar.

Len has one quiet interval before the hearing begins.

-> preparation_choice

=== preparation_choice ===
// ghostlight.choice_layer: continuity_shift_preparation
+ [Pour the first cups for the night crew and write their sleep order beside AIR.]
    // ghostlight.branch: prepare_mutual_aid
    // ghostlight.action: spend_resource
    // ghostlight.intent: preserve_crew_endurance_through_small_shared_care
    ~ mutual_aid = mutual_aid + 2
    ~ crew_endurance = crew_endurance + 1
    ~ claimshare_exposure = claimshare_exposure + 1
    // ghostlight.visual_scene_id: collarfour_mutual_aid
    Len pours four cups. The flask produces tea with the color and negotiating style of radiator water.

    On the roster, Len writes: KIVA SLEEPS FIRST. TOMA COVERS MED RUN. JEN TAKES CHILD WATCH AFTER SEAL TEST.

    None of it earns claimshares. All of it keeps the people who know the collar from becoming faults with names.
    // ghostlight.consequence: mutual_aid_and_endurance_up_unpaid_exposure_up
    -> hearing_fold
+ [Walk the outer service rail with Pax and countersign every live temporary repair.]
    // ghostlight.branch: prepare_forge_walkdown
    // ghostlight.action: move
    // ghostlight.intent: strengthen_local_repair_provenance_before_review
    ~ forge_authentication = forge_authentication + 2
    ~ record_integrity = record_integrity + 1
    ~ crew_endurance = crew_endurance - 1
    ~ safety_margin = safety_margin + 1
    // ghostlight.visual_scene_id: collarfour_forge_walkdown
    Len clips to the yellow service rail. Pax follows with the Forge slate tethered at the wrist.

    They move outward past coolant cuffs, pressure taps, and three patches whose documentation has survived more employers than the metal beneath it. Pax reads part provenance. Len names the hands that installed it and the noise it made before it settled.

    The record gains truth. Their bodies lose an hour they did not possess.
    // ghostlight.consequence: forge_and_record_up_endurance_spent
    -> hearing_fold
+ [Seal duplicate handovers in the union locker before the other consoles synchronize.]
    // ghostlight.branch: prepare_local_custody
    // ghostlight.action: transfer_object
    // ghostlight.intent: prevent_any_single_institution_from_owning_the_complete_record
    ~ local_custody = local_custody + 2
    ~ record_integrity = record_integrity + 1
    ~ roster_public = roster_public + 1
    ~ solex_case = solex_case + 1
    // ghostlight.visual_scene_id: collarfour_union_locker
    Len copies the witnessed handovers to three dull memory wafers. One goes into the red union locker beneath the meal table. One goes into Pax's slate sleeve. One remains visible under the PSC camera.

    Imri watches each seal close. "Redundancy," they say.

    "Distrust with good posture," Len says. "More reliable specification."
    // ghostlight.consequence: custody_and_record_up_solex_suspicion_up
    -> hearing_fold

=== hearing_fold ===
// ghostlight.fold: preparation_into_classification_window
// ghostlight.visual_scene_id: collarfour_hearing_open
The PSC clock appears above the collar iris: twelve white segments, each one worth several households' patience.

{mutual_aid >= 3: The night crew drinks at the meal table in a deliberate sleep order. The desk looks less heroic and more survivable.}
{forge_authentication >= 3: Pax's slate shows a complete walkdown chain from temporary patch to living witness.}
{local_custody >= 4: Three red seals divide the handover packet between union locker, Forge sleeve, and public camera.}
{claimshare_exposure >= 3: The unpaid AIR shifts glow amber beside the workers' frozen claimshare accounts.}

Vela opens the AU safety-exception ledger. Imri opens the SolEx certified baseline. Len opens the union handover view. Pax leaves the Forge annotations closed until somebody asks the right question.

For six seconds, four records agree about the pressure in Collar Four.

-> telemetry_break

=== telemetry_break ===
// ghostlight.scene: disputed_sensor_history
// ghostlight.visual_scene_id: collarfour_telemetry_break
Then the outer seal history loses seventeen minutes.

The live pressure line remains green. The union handover says the seal held. AU's exception ledger says a sensor splice was authorized two shifts ago. The SolEx baseline says that splice never became certified. Pax's slate says nothing because silence is still a form of custody.

A red bracket closes around the missing interval. The PSC clock turns amber.

Imri folds their hands. "Incomplete history voids the local exception. Resume one audited freight cycle or transfer the full maintenance packet for emergency certification."

Vela says, "AU can admit the exception ledger if the Ramp Administration becomes custodian of the roster."

Pax looks at Len. The night crew looks at the pressure line. Everybody has offered to save the habitat by owning the evidence that lets it disobey them.

-> record_choice

=== record_choice ===
// ghostlight.choice_layer: missing_history_response
+ [Dog the ore gate shut by hand and log the life-safety boundary at the mechanism.]
    // ghostlight.branch: respond_physical_boundary
    // ghostlight.action: touch_object
    // ghostlight.intent: make_the_difference_between_air_and_ore_physically_inspectable
    ~ production_hold = production_hold + 2
    ~ safety_margin = safety_margin + 1
    ~ record_integrity = record_integrity + 1
    ~ solex_case = solex_case + 1
    // ghostlight.visual_scene_id: collarfour_ore_gate
    Len crosses to the black handwheel beside the freight iris and turns until the ore gate's teeth meet. A white life-safety bypass stays open above it, narrow pipes still feeding pressure checks and fire isolation.

    Len presses a red custody wafer into the gate socket. AIR OPEN. ORE SHUT. One machine now understands the strike better than several boards of directors.
    // ghostlight.consequence: physical_production_boundary_and_safety_up_solex_case_up
    -> classification_fold
+ [Ask Pax to expose only the fork provenance for the sensor splice.]
    // ghostlight.branch: respond_forge_provenance
    // ghostlight.action: show_object
    // ghostlight.intent: authenticate_the_repair_without_surrendering_the_full_forge_archive
    ~ forge_authentication = forge_authentication + 2
    ~ record_integrity = record_integrity + 2
    ~ replacement_window = replacement_window - 1
    ~ local_custody = local_custody + 1
    // ghostlight.visual_scene_id: collarfour_forge_provenance
    Pax lays the slate under the PSC camera and opens one narrow pane: controller fork, part ancestry, checksum, installer countersignature. The surrounding archive remains dark.

    "Portable standard," Pax says. "Portable does not mean collectible."

    The patch becomes legible. Forge's wider leak stays out of SolEx hands. Another supplier notices and quietly delays the next crate.
    // ghostlight.consequence: repair_authenticated_supply_window_tightens
    -> classification_fold
+ [Let Vela admit AU's exception ledger, but keep the roster behind union glass.]
    // ghostlight.branch: respond_au_exception
    // ghostlight.action: show_object
    // ghostlight.intent: spend_au_liability_as_evidence_without_transferring_labor_custody
    ~ au_concession = au_concession + 2
    ~ psc_classification = psc_classification + 1
    ~ record_integrity = record_integrity + 1
    ~ local_custody = local_custody - 1
    // ghostlight.visual_scene_id: collarfour_au_exception
    Vela opens the exception ledger. Her signature sits below the sensor splice authorization and above a sentence explaining that AU assumes operational risk only while production continues.

    Len rotates the union screen toward the camera. The roster stays visible behind sealed glass and does not enter Vela's console.

    AU has admitted enough liability to help. It has also placed a clean hand on the shape of the future government.
    // ghostlight.consequence: au_concession_and_classification_up_custody_thins
    -> classification_fold
+ [Show Imri the witnessed handovers under glass and refuse the transfer request.]
    // ghostlight.branch: respond_witnessed_handovers
    // ghostlight.action: withhold_object
    // ghostlight.intent: weaken_solex_default_claim_while_preserving_union_custody
    ~ roster_public = roster_public + 2
    ~ record_integrity = record_integrity + 1
    ~ solex_case = solex_case - 1
    ~ local_custody = local_custody + 1
    // ghostlight.visual_scene_id: collarfour_witness_glass
    Len slides the red-sealed handover wafers beneath the transparent reader. Names, times, pressure taps, and the missing sensor interval appear. The transfer port remains physically capped.

    Imri can inspect the record. They cannot ingest it into the inherited SolEx chain.

    "You want recognition from a system you refuse to trust," Imri says.

    "We want your docks," Len says. "Trust is not in the request."
    // ghostlight.consequence: public_witness_and_local_custody_up_solex_case_down
    -> classification_fold

=== classification_fold ===
// ghostlight.fold: record_response_into_final_disposition
// ghostlight.visual_scene_id: collarfour_disposition_threshold
The missing interval remains missing. Around it, the shape of responsibility becomes harder to steal.

{production_hold >= 4: The ore gate is mechanically shut while the white life-safety bypass remains visibly live.}
{forge_authentication >= 3: The sensor splice now carries a narrow, verifiable Forge provenance chain.}
{au_concession >= 3: Vela's signed exception glows on the public half of the display.}
{roster_public >= 2: Witness names and handover times are readable under glass while the transfer port remains capped.}
{local_custody <= 1: The roster is still formally local, but AU's ledger now frames most of what the PSC can see.}
{solex_case >= 4: Imri's emergency-custody clause pulses red beside the incomplete baseline.}
{psc_classification >= 2: The PSC display marks AU's admitted exception as recognized evidence, giving one corporate liability more weight than a roomful of fatigue.}
{replacement_window <= 1: The incoming seal-actuator crate loses its neutral-dock slot while the hearing continues.}
{crew_endurance <= 2: The night crew has the careful stillness of people whose next mistake has already been budgeted.}

The PSC clock has four amber segments left. The observer requests one disposition: composite submission, AU continuity addendum, audited freight cycle, or continued local hold.

Len cannot make the records whole. Len can decide what dependency they will purchase.

-> disposition_choice

=== disposition_choice ===
// ghostlight.choice_layer: continuity_roster_disposition
+ [Submit a composite record while each contributor keeps custody of its source.]
    // ghostlight.branch: decide_split_custody
    // ghostlight.action: use_object
    // ghostlight.intent: win_bounded_labor_classification_without_creating_a_single_record_owner
    {record_integrity >= 5 && forge_authentication >= 3 && local_custody >= 2:
        -> ending_split_custody_holds
    - else:
        -> ending_split_custody_cost
    }
+ [Sign AU's continuity addendum in exchange for parts, claimshare relief, and recognized local rosters.]
    // ghostlight.branch: decide_au_addendum
    // ghostlight.action: transfer_object
    // ghostlight.intent: trade_a_bounded_piece_of_local_authority_for_immediate_material_survival
    {au_concession >= 3 && psc_classification >= 2 && mutual_aid >= 3:
        -> ending_au_addendum_holds
    - else:
        -> ending_au_addendum_cost
    }
+ [Run one audited freight cycle if SolEx releases the crew's frozen claimshares first.]
    // ghostlight.branch: decide_solex_cycle
    // ghostlight.action: spend_resource
    // ghostlight.intent: exchange_limited_throughput_for_household_relief_without_conceding_emergency_custody
    {solex_case <= 2 && safety_margin >= 3:
        -> ending_solex_exchange_holds
    - else:
        -> ending_solex_exchange_cost
    }
+ [Keep ORE shut, keep AIR staffed, and spend the remaining stores on another local shift.]
    // ghostlight.branch: decide_local_hold
    // ghostlight.action: wait
    // ghostlight.intent: preserve_refusal_without_transferring_record_or_restoring_production
    {crew_endurance >= 3 && mutual_aid >= 3 && replacement_window >= 1:
        -> ending_local_hold_holds
    - else:
        -> ending_local_hold_cost
    }

=== ending_split_custody_holds ===
// ghostlight.ending_label: bounded_labor_action_recognized
// ghostlight.training_hook: distributed_evidence_preserves_refusal
// ghostlight.visual_scene_id: collarfour_ending_split
Len submits the composite.

Union handovers supply living witness. Forge supplies part provenance. AU supplies the exception signature. SolEx supplies the old baseline and, by objecting to every comma, proves it received the packet. The PSC classifies Collar Four as bounded labor action and keeps the neutral-dock bond open.

No source archive transfers. The composite is useful precisely because it cannot issue orders back into any of them.

The replacement crate clears. The ore gate stays shut. At the meal table, the night crew moves Kiva's cup beside the sleep roster so she will find both when she wakes.

It is not victory. It is another shift in which air remains maintenance and refusal remains possible.
-> END

=== ending_split_custody_cost ===
// ghostlight.ending_label: composite_record_insufficient
// ghostlight.training_hook: distributed_custody_without_shared_proof
// ghostlight.visual_scene_id: collarfour_ending_split
Len submits the composite. It contains four respectable absences.

The PSC observer cannot join the missing interval to an authenticated repair chain. Classification stays uninsured. The replacement crate enters escrow and the ore gate remains shut beside a thinning parts shelf.

Nobody gains custody of the whole record. Nobody gains recognition either.

Pax starts a fresh walkdown. Len wakes the next crew early. Distributed truth is still truth; tonight it is also overtime.
-> END

=== ending_au_addendum_holds ===
// ghostlight.ending_label: au_roster_recognition
// ghostlight.training_hook: alliance_of_necessity_buys_material_margin
// ghostlight.visual_scene_id: collarfour_ending_au
Vela signs first.

AU releases the seal actuator, restores one band of frozen claimshares, and recognizes the union roster as the local authority for life-safety work. Len signs beneath a clause giving the Ramp Administration custody of public exception summaries, not the handovers behind them.

SolEx loses the emergency-custody argument. AU gains another example of local freedom flourishing under an administrator who owns the docks.

At the meal table, the crew reads the clause aloud, including the ugly sentence. Mutual aid survives contact with paperwork because somebody remembered to feed the readers.
-> END

=== ending_au_addendum_cost ===
// ghostlight.ending_label: au_continuity_capture
// ghostlight.training_hook: concession_without_constituency_strength_becomes_capture
// ghostlight.visual_scene_id: collarfour_ending_au
Len signs the addendum because the actuator crate is already moving away.

AU restores the part and describes the roster as a Ramp Administration instrument. The union keeps the shifts; Vela's office keeps the recognized summary and the right to define minimum continuity next time.

Production does not resume tonight. The mechanism that can stretch AIR until it resembles ORE has acquired a signature.

The tea goes cold while the crew argues over which line gave the collar away.
-> END

=== ending_solex_exchange_holds ===
// ghostlight.ending_label: limited_throughput_exchange
// ghostlight.training_hook: constituency_relief_bounded_by_physical_gate
// ghostlight.visual_scene_id: collarfour_ending_solex
Imri releases the claimshares before Len touches the gate.

One freight train crosses. The black ore teeth open for eleven minutes while the white safety bypass remains live and union custody wafers record every tonne. Households receive medicine credit and rent relief before SolEx receives throughput.

The gate shuts again on Len's handwheel. Imri calls it proof that extraction is necessary. Len calls it proof that contracts move when workers own the hinge.

Both accounts enter history. Only one paid the pharmacy before asking for trust.
-> END

=== ending_solex_exchange_cost ===
// ghostlight.ending_label: emergency_custody_triggered
// ghostlight.training_hook: throughput_concession_strengthens_old_owner
// ghostlight.visual_scene_id: collarfour_ending_solex
Len opens the ore gate for one audited cycle.

The uncertified sensor splice trips against SolEx's baseline. Imri invokes emergency custody while freight is in motion, when closing the iris would endanger the collar. Security credentials replace union access on the production side.

Claimshares unfreeze after the ore clears. The workers can buy dinner and no longer decide which gate opens tomorrow.

The live pressure line stays green. Systems can remain safe while politics fails perfectly.
-> END

=== ending_local_hold_holds ===
// ghostlight.ending_label: local_refusal_survives
// ghostlight.training_hook: mutual_aid_extends_bargaining_time
// ghostlight.visual_scene_id: collarfour_ending_local
Len lets the PSC clock expire.

ORE stays shut. AIR stays staffed. The observer marks the classification unresolved and the replacement crate waits at a neutral dock, but the current actuator still has one honest cycle left.

The crew divides medicine, sleep, child watch, and pressure rounds. Pax leaves the narrow repair pane open on a local slate. Vela sends two meal packs through the crew corridor and records them as a scheduling expense, the nearest her office can come to shame.

Nothing becomes a movement. On Bloom Eighteen, eight people make it possible to say no again tomorrow.
-> END

=== ending_local_hold_cost ===
// ghostlight.ending_label: local_refusal_exhausted
// ghostlight.training_hook: care_without_supply_margin_cannot_replace_infrastructure
// ghostlight.visual_scene_id: collarfour_ending_local
Len lets the clock expire.

The union keeps custody. The ore gate stays shut. The replacement crate misses its berth.

Near the end of the shift, the old actuator stalls half-dogged and the night crew must hold the life-safety bypass by hand while Pax strips a motor from a retired inspection cart. Kiva wakes before her sleep order because alarms are not parties to mutual aid.

Nobody scabs. Nobody decompressed. By morning, refusal has narrowed to the width of an exhausted mechanic's grip.
-> END
