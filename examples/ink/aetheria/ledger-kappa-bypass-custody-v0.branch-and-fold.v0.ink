// ghostlight.artifact_id: ledger_kappa_bypass_custody_branch_fold_v0
// ghostlight.fixture_id: ledger-kappa-bypass-custody-v0
// ghostlight.scene_id: ledger-kappa-bypass-custody-v0.stores-collar-six
// ghostlight.final_ink_path: examples/ink/aetheria/ledger-kappa-bypass-custody-v0.branch-and-fold.v0.ink
// ghostlight.tonal_mode: dry workplace black comedy with claustrophobic solidarity

VAR reserve_margin = 2
VAR worker_custody = 1
VAR claimshare_debt = 0
VAR recorder_integrity = 1
VAR baseline_solidarity = 1
VAR harness_margin = 2
VAR management_control = 2
VAR parts_distributed = false
VAR nara_sequence_logged = false
VAR bypass_deployed = false
VAR unsafe_restart = false
VAR teth_injury = false
VAR claimshare_hold = false
VAR recall_recorded = false

-> start

=== start ===
// ghostlight.scene: ledger_kappa.start
In 2721, three weeks after Kappa workers refused an unsafe shell route, Stores Collar Six has acquired a morning ritual.

Teth Inkwise counts the pieces of a machine Aeronautics Unlimited insists is not a machine.

-> stores_establishing

=== stores_establishing ===
// ghostlight.scene: ledger_kappa.stores_establishing
The collar is a curved service bay at the counterspinward entrance to Service Ring Kappa in AU Yard Twelve, Pallas Bloom Cluster. Apparent gravity presses boots and carts toward the Bloom-outward floor. The ceiling curves Bloom-inward toward the habitat's open industrial air.

A wire-mesh stores cage runs along the loop-inner wall. Opposite it, an outward-facing service hatch opens into Kappa-6's cramped shell artery. The ring continues spinward toward Kappa-7, where a remembered death recently learned how to stop a shift.

The bypass pieces occupy three respectable ledgers. The portable scrubber bridge is life-support stock. The yellow manual valve plate is maintenance control. Clear reinforced tubing, silver seal tape, and two blue cartridges are incident consumables. The orange custody recorder belongs to safety, which is why safety keeps asking where it went.

-> ordinary_people

=== ordinary_people ===
// ghostlight.scene: ledger_kappa.ordinary_people
Teth moves between the issue counter and the floor anchors in a compact dry-operation harness: mottled blue-gray mantle, dark eyes, bare-minimum humidity ring, oxygenation tubes, padded pressure cuffs, open support loops, and curved tool rails. Several tentacles work. One rests where a worn contact band has rubbed the skin pale.

Orrin Dax rolls in an anchor cart carrying yesterday's used seal tape and today's battered coffee mug. He is a broad baseline rigger in a patched brown-and-orange jacket, with a frost scar at his jaw and the expression of a man returning property to an institution he does not respect.

"Half a roll," Teth says.

"A full roll with management removed."

Nara-7 stands at the issue slate in her gray numbered seal-technician skinsuit. Faint interface lines mark her wrists. She has already sorted the valve plate's test sequence into the order Kappa actually uses, which differs from the order printed on it in ways the manufacturer calls local character.

Lio Vale, lean and tired in a slate-gray maintenance jacket, watches the counter from the worker entrance. Lio coordinates the shift. More importantly, Lio knows when not to improve a joke with policy.

-> morning_count_choice

=== morning_count_choice ===
// ghostlight.choice_layer: ordinary_inventory_count
// ghostlight.branch: count_harness_coupler
// ghostlight.action: touch_object
// ghostlight.intent: Protect Teth's body margin even if the replacement cuff lands on one worker account.
* [Pressure-test the harness coupler before signing the count.]
    ~ harness_margin = harness_margin + 1
    ~ claimshare_debt = claimshare_debt + 1
    Teth braces three tentacles on the floor rail and seats the small utility coupler with a fourth. The gasket shows a white stress line. The replacement cuff fits cleanly.

    The issue slate assigns the cuff to Teth's support account before it admits the cuff is defective.

    "Congratulations," Orrin says. "You personally own the concept of breathing."
    -> routine_fold

// ghostlight.branch: count_orrin_seal_stock
// ghostlight.action: transfer_object
// ghostlight.intent: Make baseline rescue stock part of the shared kit and preserve a countersigned material trail.
* [Slide Orrin's returned seal tape across the counter and ask for his countersignature.]
    ~ baseline_solidarity = baseline_solidarity + 2
    ~ recorder_integrity = recorder_integrity + 1
    ~ claimshare_debt = claimshare_debt + 1
    Orrin plants one heavy glove on the returned roll. Teth lays a tentacle tip beside it. The orange recorder catches both bodies, the torn stock code, and the fact that half a roll did a full roll's work.

    "Charge the missing half to expansion," Orrin says. "It keeps eating everything else."
    -> routine_fold

// ghostlight.branch: count_nara_valve_sequence
// ghostlight.action: show_object
// ghostlight.intent: Preserve Nara's local operating sequence as usable kit knowledge.
* [Give Nara the valve plate and watch the sequence she uses.]
    ~ nara_sequence_logged = true
    ~ reserve_margin = reserve_margin + 1
    ~ recorder_integrity = recorder_integrity + 1
    Nara lays the yellow plate against the issue counter. Tap, pause, double tap, then a thumb across two contacts the diagram treats as unrelated.

    The plate opens its manual path without waking the supervisor bus.

    "Local character," Lio says.

    Nara looks at them. "Local survival."
    -> routine_fold

// ghostlight.branch: count_distributed_parts
// ghostlight.action: move_objects
// ghostlight.intent: Divide component custody so one stores order cannot remove the entire bypass.
* [Split the components across Teth's harness, Orrin's cart, and Nara's locker.]
    ~ parts_distributed = true
    ~ worker_custody = worker_custody + 2
    ~ reserve_margin = reserve_margin - 1
    ~ management_control = management_control - 1
    Teth clips the clean tubing to the harness rail. Orrin takes the scrubber bridge frame under a coil of anchor line. Nara carries the valve plate through the locker gate as ordinary diagnostic stock.

    Assembly will be slower. Seizure will require three arguments in three offices. This is the kind of redundancy AU praises in pressure systems and prosecutes in workers.
    -> routine_fold

=== routine_fold ===
// ghostlight.fold: ordinary_inventory_after_choice
The count closes with most of the pieces still pretending they have never met.

{harness_margin >= 3: Teth's new cuff holds the humidity ring steady. The pale rub mark stops deepening for one shift.}
{claimshare_debt >= 1: A small red line opens on the issue slate under WORK-GROUP RECOVERY. It has no lungs and is already breathing heavily.}
{baseline_solidarity >= 3: Orrin leaves his countersignature visible instead of burying it under the anchor crew's general loss code.}
{nara_sequence_logged: The recorder holds Nara's actual valve sequence, a piece of work knowledge with a person visibly attached.}
{parts_distributed: The cage looks poorer. The workers are safer in the narrow administrative sense.}

For nine minutes, this is merely a workplace.

-> recall_notice

=== recall_notice ===
// ghostlight.scene: ledger_kappa.recall_notice
The stores slate chimes with the politeness reserved for theft performed by a budget.

SERVICE DEFAULT: KAPPA WORK GROUP.

UNCOMMITTED EMERGENCY RESERVE RECALLED TO CENTRAL ISSUE.

CLAIMSHARE RELEASE PENDING PRODUCTIVITY REVIEW.

AU has classified the stoppage as a missed service obligation. The classification freezes replacement cartridges, makes every future kit use chargeable to the last signing account, and turns project-linked claimshares, which gate local resources and transfers, into a door the workers' households can see but not open.

-> ilya_arrival

=== ilya_arrival ===
// ghostlight.scene: ledger_kappa.ilya_arrival

The badge stair unlocks above the loop-inner cage. Ilya Marne descends from the operations gallery in a clean dark superintendent coat, silver-blonde hair pinned tight, polished boots finding the dirty floor without learning anything from it.

"Return the cartridges," she says. "Central issue will preserve life-support continuity."

"Central issue is three spoke gates away," Lio says.

"Then do not create an emergency locally."

-> recall_choice

=== recall_choice ===
// ghostlight.choice_layer: stores_recall_response
// ghostlight.branch: put_kit_on_teth_account
// ghostlight.action: sign_object
// ghostlight.intent: Keep the kit local by accepting concentrated debt and disciplinary exposure.
* [Sign the whole assembly to Teth's support account.]
    ~ worker_custody = worker_custody + 2
    ~ claimshare_debt = claimshare_debt + 2
    ~ management_control = management_control - 1
    ~ claimshare_hold = true
    Teth spreads the issue slate across two tentacles and signs with the harness coupler pressed to its edge.

    The system accepts one biological worker as the debtor for equipment four contracts need.

    Ilya's face does not change. "That account cannot cover it."

    "Then your classification has discovered collective ownership," Teth says.
    -> recall_fold

// ghostlight.branch: move_kit_to_anchor_cart
// ghostlight.action: move_object
// ghostlight.intent: Use Orrin's recognized rescue inventory to keep the kit mobile and spread its cost.
* [Move the scrubber case onto Orrin's anchor cart as rescue rigging.]
    ~ baseline_solidarity = baseline_solidarity + 2
    ~ worker_custody = worker_custody + 1
    ~ claimshare_debt = claimshare_debt + 1
    ~ parts_distributed = true
    Orrin opens the anchor cart. Teth lowers the gray two-handled scrubber case beneath a web of orange haulback line.

    "Rescue rigging," Orrin tells the issue slate.

    The slate requests a shape code.

    "Rectangular. Heroic in poor light."
    -> recall_fold

// ghostlight.branch: record_the_recall
// ghostlight.action: use_object
// ghostlight.intent: Make the recall and its downstream cost survive later liability editing.
* [Start the custody recorder and ask Ilya to repeat the order.]
    ~ recorder_integrity = recorder_integrity + 2
    ~ management_control = management_control + 1
    ~ recall_recorded = true
    Teth rotates the orange recorder so its lens holds Ilya, the clean cartridges, and the blocked replacement line in one frame.

    "Please repeat which continuity improves when local air stock moves three spoke gates away."

    Ilya looks into the lens. "Record that the specialist is refusing a direct inventory instruction."

    The recorder does. It records the instruction too.
    -> recall_fold

// ghostlight.branch: return_clean_cartridges
// ghostlight.action: transfer_object
// ghostlight.intent: Preserve immediate claimshare access by surrendering the kit's clean consumables.
* [Return the clean cartridges and keep the rest of the frame.]
    ~ reserve_margin = reserve_margin - 2
    ~ management_control = management_control + 2
    ~ claimshare_hold = false
    Teth sets the two blue cartridges on the issue counter. Ilya seals them into a central-transfer sleeve.

    The issue slate clears its red hold symbol. The bypass frame remains, now approximately as useful as a promise with the expensive words removed.
    -> recall_fold

=== recall_fold ===
// ghostlight.fold: recall_order_after_choice
Ilya waits beside the cage while the ledgers decide what happened.

{worker_custody >= 3: Enough of the kit remains under worker hands that Ilya must negotiate with bodies, not merely inventory fields.}
{management_control >= 4: The stores slate glows a satisfied corporate blue. Central authority has a clean line through the room.}
{recorder_integrity >= 3: The orange recorder holds the recall order beside the exact equipment it removes.}
{claimshare_hold: Teth's account shows a red claimshare hold. Somewhere outside Kappa, that line has become food, housing, medicine, or a delayed transfer for people the slate does not picture.}
{parts_distributed: The cage cannot show a complete kit because no single cage owns one.}

Then Kappa-6 coughs.

-> amber_fault

=== amber_fault ===
// ghostlight.scene: ledger_kappa.amber_fault
// aetheria.flashpoint: a local condensate fault tests whether the work stoppage can remain safe
Amber light runs along the loop-outer manifold. Condensate backs into the return-air exchanger, and the pressure lung begins cycling heat into a service volume too small to forgive it.

The public yard is still breathing. The next isolation step will shut airflow to two worker dormitory blocks and the shift galley before it touches the fabrication deck.

Ilya checks her console cuff. "Restart the main line."

Nara places two fingers on the yellow valve plate. "The reported clear path crosses the false seam."

Orrin grips the anchor cart. Lio closes the worker entrance so nobody else can be counted into the liability.

Teth can feel the harness humidity drift as the service bay warms. The bypass can buy time. It cannot abolish heat, replace clean cartridges, or make the corporation forgive a safe refusal.

-> bypass_choice

=== bypass_choice ===
// ghostlight.choice_layer: amber_fault_response
// ghostlight.branch: teth_routes_bypass
// ghostlight.action: move_and_use_object
// ghostlight.intent: Use cephalopod reach and local kit custody to create safe margin without restarting the disputed line.
* [Thread the flexible bridge through the Kappa-6 service hatch.]
    ~ bypass_deployed = true
    ~ reserve_margin = reserve_margin + 2
    ~ worker_custody = worker_custody + 1
    ~ harness_margin = harness_margin - 1
    Teth clips two flexible haulback leads from the harness frame to the floor sockets and advances through the outward-facing hatch mantle-first. The support loops follow the body into the artery. Tentacles carry clear reinforced tubes around the curved rib, one sucker line reading vibration while another seats a red cuff on the pressure bypass.

    The scrubber bridge wakes at the human-scale manifold. It is ugly, portable, and immediately more constitutional than the Ramp Administration.
    -> fault_fold

// ghostlight.branch: nara_runs_sequence
// ghostlight.action: coordinate
// ghostlight.intent: Let Nara's preserved work knowledge operate the manual plate while Teth protects the physical route.
* {nara_sequence_logged} [Give Nara the valve plate and call her sequence through the hatch.]
    ~ bypass_deployed = true
    ~ reserve_margin = reserve_margin + 2
    ~ recorder_integrity = recorder_integrity + 1
    ~ worker_custody = worker_custody + 1
    Nara works the contacts in the sequence preserved that morning. Teth answers from inside the artery with pressure taps against the tube.

    The bypass opens without waking the false seam or the supervisor bus. Ilya watches a manufactured worker use memory to keep her habitat alive and looks mainly annoyed about the permissions model.
    -> fault_fold

// ghostlight.branch: lend_harness_oxygenation_loop
// ghostlight.action: transfer_object
// ghostlight.intent: Spend Teth's body-support margin to replace missing clean consumables long enough for dormitory isolation.
* [Bridge the return with Teth's own oxygenation loop.]
    ~ bypass_deployed = true
    ~ reserve_margin = reserve_margin + 1
    ~ harness_margin = harness_margin - 2
    ~ teth_injury = true
    Teth unclips the clean side of the harness oxygenation loop and passes it through the gray scrubber case. The return flow steadies. The humidity ring does not.

    Mottled skin pales around the worn contact band. Orrin notices first because people who spend their lives under load know what a support is doing when it becomes a sacrifice.
    -> fault_fold

// ghostlight.branch: accept_main_line_restart
// ghostlight.action: comply
// ghostlight.intent: Preserve throughput and immediate air service by returning control to AU despite the false-seam warning.
* [Clear Ilya's main-line restart.]
    ~ unsafe_restart = true
    ~ management_control = management_control + 2
    ~ worker_custody = worker_custody - 1
    ~ reserve_margin = reserve_margin + 1
    Teth clears the local interlock. Ilya's console sends the restart.

    The main line shudders through the false-seam route. Dormitory airflow recovers. The Kappa-6 wall answers with a hard metallic click Nara has heard before.

    Nobody calls that sound continuity.
    -> fault_fold

=== fault_fold ===
// ghostlight.fold: fault_response_into_custody_decision
{bypass_deployed:
The temporary bridge holds the local flow in amber. Its clear tubes pulse between the human-scale manifold and the shell artery. The board estimates enough margin to isolate the dormitory return and stop the fabrication load cleanly.
- else:
The main line carries the load. Kappa-6 cools slowly while the false seam keeps its own counsel.
}

{reserve_margin >= 4: The margin board shows a broad amber band: enough time for argument to remain a choice.}
{reserve_margin <= 1: The margin board shows one thin amber bar. The missing cartridges have become time everybody can see.}
{harness_margin <= 1: Teth's humidity ring flashes low; the dry air has begun collecting payment directly from skin.}
{teth_injury: Orrin moves his battered mug under Teth's nearest working tentacle, an absurd offer of water in a system that requires the wrong chemistry and the correct kindness.}
{unsafe_restart: Nara keeps two fingers on the wall, listening to the click propagate toward Kappa-7.}
{recall_recorded: The recorder shows Ilya's recall order next to the live margin board.}

The fault is contained for the moment. The cost is not.

-> custody_threshold

=== custody_threshold ===
// ghostlight.scene: ledger_kappa.custody_threshold
Central issue offers replacement cartridges if Ilya seals the assembled kit in the AU cage and names one accountable operator. The claimshare hold can then become a repayment schedule instead of an immediate freeze.

Lio reads the terms twice. "They will sell us back the ability to stop safely."

"Lease," Ilya says. "Ownership implies maintenance responsibility."

"Excellent," Orrin says. "We have found the shy part of ownership."

{claimshare_debt >= 3: The red recovery line is now large enough to reach past Teth's account into the work group's next distribution.}
{claimshare_hold: The hold remains live; households cannot spend standing while AU reviews the service default.}
{worker_custody >= 4: The kit is physically in worker hands even while the ledger disputes the fact.}
{management_control >= 4: Ilya can release stock quickly, and can close it just as quickly after the next refusal.}
{parts_distributed: Three workers visibly hold different pieces of the assembly.}
{nara_sequence_logged: Nara's sequence remains copied on the recorder and on Lio's slate.}

Teth must decide what this machine will be after the air is ordinary again.

-> final_choice

=== final_choice ===
// ghostlight.choice_layer: post_fault_custody
// ghostlight.branch: choose_shared_custody
// ghostlight.action: sign_and_transfer
// ghostlight.intent: Make three labor categories jointly necessary to authorize and deploy the kit.
* [Record shared custody under Teth, Nara, and Orrin's work groups.]
    {worker_custody >= 3 && baseline_solidarity >= 2 && recorder_integrity >= 2:
        -> ending_shared_custody_success
    - else:
        -> ending_shared_custody_cost
    }

// ghostlight.branch: choose_central_supply
// ghostlight.action: transfer_object
// ghostlight.intent: Accept AU custody to restore consumables and household access now.
* [Seal the kit in AU stores in exchange for cartridges and claimshare release.]
    {management_control >= 4 && reserve_margin >= 2 && not teth_injury:
        -> ending_central_supply_success
    - else:
        -> ending_central_supply_cost
    }

// ghostlight.branch: choose_mutual_aid
// ghostlight.action: move_objects
// ghostlight.intent: Distribute parts and replacement obligations across crews outside the single work-group ledger.
* [Send each component along a different worker route.]
    {parts_distributed && baseline_solidarity >= 2 && reserve_margin >= 2:
        -> ending_mutual_aid_success
    - else:
        -> ending_mutual_aid_cost
    }

// ghostlight.branch: choose_assembly_map
// ghostlight.action: withhold_and_record
// ghostlight.intent: Return components to ordinary stock while preserving the worker-owned knowledge that makes them a kit.
* [Break the kit back into stock and keep only the assembly map.]
    {parts_distributed && nara_sequence_logged && management_control <= 2:
        -> ending_assembly_map_success
    - else:
        -> ending_assembly_map_cost
    }

=== ending_shared_custody_success ===
// ghostlight.ending_label: shared_custody_success
// ghostlight.training_hook: distributed_authority_protects_technical_mercy
Teth signs with the harness coupler. Nara signs with the valve sequence. Orrin signs with the anchor crew's rescue code.

The recorder will not release the kit under fewer than two of the three work-group marks. No signature decides personhood. No signature owns the air. Together they decide whether this particular bridge leaves the cage.

Ilya accepts because Kappa is still amber and because three separate refusals are harder to discipline than one insolvent specialist.

{recall_recorded: Her earlier recall order remains attached to the custody record. The next official who asks why the arrangement exists will have to look directly at the answer.}

The replacement cartridges arrive. So does the repayment schedule. The machine is safer; the workers are poorer; the custody line is real.

At shift end, Orrin paints three small marks on the gray scrubber case. Teth adds a fourth, because the people breathing downstream were also part of the decision even if the ledger had neglected to invite them.
-> END

=== ending_shared_custody_cost ===
// ghostlight.ending_label: shared_custody_cost
// ghostlight.training_hook: collective_language_without_material_support_concentrates_risk
The form accepts three names and assigns the debt to the only support account already open.

Teth's.

Orrin protests. Nara repeats the custody sequence. Lio records both. The machine has learned the language of shared responsibility while preserving a single throat to squeeze.

{claimshare_hold: The red hold remains. Teth's next harness cuff, housing transfer, and work-group distribution all wait behind the service-default review.}
{claimshare_debt >= 3: The recovery line reaches into the next shift before the current one ends.}

The bridge stays local. That is not nothing. It is also exactly how a corporation turns solidarity into one worker's arrears.
-> END

=== ending_central_supply_success ===
// ghostlight.ending_label: central_supply_success
// ghostlight.training_hook: short_term_safety_can_restore_the_owner
Ilya opens central issue. Two clean blue cartridges arrive by spoke cart before the amber band thins.

The dormitory return remains open. Teth's harness stays within safe humidity. The claimshare hold clears.

Then Ilya seals the assembled kit behind the loop-inner cage and changes its issue rule to superintendent authorization.

The workers win air, supplies, and one uninterrupted night. AU wins the right to decide whether the next refusal is allowed to remain safe.

Orrin looks through the mesh at the gray case. "Good news," he says. "Continuity has been preserved somewhere we cannot reach it."
-> END

=== ending_central_supply_cost ===
// ghostlight.ending_label: central_supply_cost
// ghostlight.training_hook: concession_after_depletion_does_not_restore_spent_margin
Ilya takes the kit and releases new stock. The exchange is efficient enough to look merciful on a report.

{teth_injury: The cartridges arrive after Teth has already lent the harness loop to the return line. Medical logs the pale skin and oxygen debt as support misuse.}
{reserve_margin <= 1: The transfer cart waits behind a spoke gate while the margin board falls to one bar.}
{unsafe_restart: Kappa-6's hard click enters the maintenance backlog as an inspection item scheduled after productivity recovery.}

The claimshare hold becomes a repayment schedule. The cage locks. Immediate catastrophe recedes, leaving everyone alive enough to inherit the invoice.
-> END

=== ending_mutual_aid_success ===
// ghostlight.ending_label: mutual_aid_success
// ghostlight.training_hook: quiet_cross_category_exchange_preserves_exit
The gray bridge frame leaves under Orrin's orange haulback line. Nara carries the valve plate as an ordinary diagnostic. Teth carries the clean tubing on the harness rail. Lio copies the recorder chain to three work slates and gives none of them a grand name.

The anchor crew owes seal stock. The engineered technicians owe a tested sequence. Teth's crew owes the next clean cartridge. Each debt is small enough to remember and specific enough to refuse.

AU can freeze the Kappa work-group ledger. It cannot recall an assembly that does not exist until these workers choose one another.

At the counterspinward shift lock, an unknown galley worker leaves a sealed humidity cartridge beside Teth's tool rail and keeps walking. Tiny, practical, deniable. Hope arrives with no branding department at all.
-> END

=== ending_mutual_aid_cost ===
// ghostlight.ending_label: mutual_aid_cost
// ghostlight.training_hook: solidarity_without_distribution_remains_seizable
Teth names three routes. The parts are still sitting together.

Ilya seals the cage before Orrin can move the bridge frame. Nara gets the valve plate through the locker gate, but the cartridges and tubing remain behind mesh.

{baseline_solidarity <= 1: The anchor crew watches from the worker entrance, sympathetic in the expensive and largely decorative way of people who did not sign.}
{reserve_margin <= 1: The next local fault will begin with less margin than this one.}

Mutual aid exists as intention. Central stores inventories it by weight.
-> END

=== ending_assembly_map_success ===
// ghostlight.ending_label: assembly_map_success
// ghostlight.training_hook: portable_work_knowledge_outlives_inventory_control
The scrubber bridge returns to life-support. The valve plate becomes local diagnostics. Orrin's tape goes back to rescue stock. Tubes and recorder resume their harmless bureaucratic childhoods.

Nara's actual sequence remains on three worker slates. Teth adds the coupler geometry and the route through Kappa-6. Orrin adds the anchor loads in handwriting the system cannot search without admitting it needs him.

Ilya gets an empty cage and a complete count.

The workers keep a recipe for minutes.

Nothing in Stores Collar Six looks like a movement. The next shift still knows how to build one breath of refusal from ordinary parts.
-> END

=== ending_assembly_map_cost ===
// ghostlight.ending_label: assembly_map_cost
// ghostlight.training_hook: knowledge_without_custody_or_sequence_cannot_bridge_a_fault
The parts return to their ledgers. The workers keep a diagram.

{not parts_distributed: The diagram assumes components one supervisor can now seal in one cage.}
{not nara_sequence_logged: The manual valve path ends at the printed sequence, just before the false seam.}
{management_control >= 3: Ilya accepts the inventory count and revokes local issue authority.}

The map is true and not yet useful. At the next amber alarm, somebody will have to recover the parts, the route, and the right people before the habitat spends its margin.

Paper can remember a bridge. It cannot carry air.
-> END
