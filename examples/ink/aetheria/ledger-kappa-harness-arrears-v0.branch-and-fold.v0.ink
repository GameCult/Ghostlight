// ghostlight.artifact_id: ledger_kappa_harness_arrears_branch_fold_v0
// ghostlight.fixture_id: ledger-kappa-harness-arrears-v0
// ghostlight.scene_id: ledger-kappa-harness-arrears-v0.kappa-last-current-shift
// ghostlight.final_ink_path: examples/ink/aetheria/ledger-kappa-harness-arrears-v0.branch-and-fold.v0.ink

VAR rig_margin = 2
VAR crew_reserve = 2
VAR arrears_evidence = 0
VAR worker_cohesion = 1
VAR yard_margin = 3
VAR claimshare_pressure = 1
VAR route_knowledge = 1
VAR superintendent_pressure = 1
VAR limited_entry_record = 0
VAR production_exception = 0
VAR lease_balance_signed = 0

-> start

=== start ===
// ghostlight.scene: kappa_support_prep_open
// ghostlight.visual_scene_id: kappa_prep_establishing
// aetheria.flashpoint: Pallas Species Strikes lead-up
Service Ring Kappa starts each shift by asking its workers whether they are still compatible with the room.

The rig-prep bay opens from the counterspinward end of a curved maintenance gallery inside an Aeronautics Unlimited industrial Bloom at Pallas. Beyond an inward safety rail, the yard's broad atmosphere rises toward work lights and the distant axis. Outward, behind ribbed shell plating and consolidated asteroid shielding, Kappa's narrow crawl throats reach seal lungs, coolant cuffs, condensate traps, and sensor nests.

The bay smells of warm metal, wet polymer, and the citrus disinfectant AU buys because it is cheaper than making dread odorless.

-> introduce_tavi

=== introduce_tavi ===
// ghostlight.scene: tavi_rig_routine
// ghostlight.visual_scene_id: kappa_harness_fit
Tavi settles onto the low work-support rail while Sela Jori fits the dry-operation harness around their mantle. Tavi is an uplifted octopoid maintenance worker: eight flexible arms, mottled umber skin paling around old cuff marks, two dark lateral eyes, and a body designed before anyone asked what shape a shift should have.

The harness is mobile work support, not a tank. A pliable body loop lies close around the mantle. A misting humidity collar beads the skin. A compact oxygenation canister pulses at the back of the harness. Soft pressure cuffs and conductive contact bands leave the arms free to brace, crawl, and use tools. A narrow tool rail rides along one side.

Sela is the licensed fitter, a baseline human in faded green coveralls. She checks seals, oxygen flow, cuff fit, and the signed route envelope. Her employer can certify the harness. It cannot certify the crawl.

"Comfort margin?" Sela asks.

Tavi raises two arms.

"That is not a number."

Tavi raises a third.

"That is the spirit in which accounting was invented."

-> introduce_shift_line

=== introduce_shift_line ===
// ghostlight.scene: ordinary_shift_line
// ghostlight.visual_scene_id: kappa_shift_line
Rook Venn waits beside the equipment cart, a baseline rigger in orange-gray coveralls with a manual bypass plate strapped to the side. He has spent eleven years being exactly human-sized and still cannot fit through Kappa-7.

At the next support rail, an unnamed octopoid worker tests a patched contact band by passing a steel nut from sucker to sucker. A transparent custody sleeve on the wall holds two sealed humidity membranes and one dry-operation adapter from the crew's strike-safe bypass kit.

Above the ring, the glassed operations gallery looks down through a long inward-facing window. Superintendent Ione Malk is a small dark silhouette behind it. Her badge stair reaches the ring directly. Authority gets the short commute.

The shift board calls Kappa-7's sensor-nest inspection routine. If it clears before the heavy yard cycle, freight leaves on time and the shift's claimshares—the project-linked labor claims used for resource access—remain usable. If it does not, the seal-lung cluster stays on temporary bypass and spends pressure and thermal margin.

First, the harness.

-> prep_choice

=== prep_choice ===
// ghostlight.choice_layer: harness_preparation
// ghostlight.visual_scene_id: kappa_prep_choice
+ [Take one of the last sealed humidity membranes and fit it now.]
    // ghostlight.action_label: use_object
    // ghostlight.branch_label: prep_fresh_membrane
    // ghostlight.intent: buy bodily margin before the crawl
    ~ rig_margin = rig_margin + 2
    ~ crew_reserve = crew_reserve - 1
    ~ claimshare_pressure = claimshare_pressure + 1
    Tavi draws the sealed membrane from the custody sleeve with two careful arms. Sela fits it beneath the misting collar and logs the issue against Tavi's equipment line.

    Cool moisture spreads over the mantle. The relief is immediate. So is the new charge on the shift board.
    -> routine_fold
+ [Fit the pooled dry-operation adapter and leave both sealed membranes for whoever comes back hurt.]
    // ghostlight.action_label: use_object
    // ghostlight.branch_label: prep_pooled_adapter
    // ghostlight.intent: preserve individual support through a crew-held part
    ~ rig_margin = rig_margin + 1
    ~ worker_cohesion = worker_cohesion + 2
    ~ arrears_evidence = arrears_evidence + 1
    Tavi takes the unbranded adapter. Sela checks its physical fit, then Rook closes the tamper-evident custody strip by hand.

    No supplier mark appears on the seal. Three workers' marks do. The adapter is less elegant than the licensed coupling and much more willing to exist.
    -> routine_fold
+ [Hold out the arm with the abraded cuff mark and make Sela record it before the shift clock starts.]
    // ghostlight.action_label: show_body
    // ghostlight.branch_label: prep_record_cuff_wear
    // ghostlight.intent: turn pain into inspectable route evidence before management can rename it
    ~ route_knowledge = route_knowledge + 1
    ~ arrears_evidence = arrears_evidence + 1
    ~ superintendent_pressure = superintendent_pressure + 1
    Tavi lays one arm across the inspection pad. The skin beneath yesterday's cuff is pale and ridged.

    Sela photographs the fit, presses a physical imprint into Tavi's manual record card, and lets the clock complain for both of them.

    The gallery window brightens. Ione has noticed delay, which is what management calls evidence before it has read it.
    -> routine_fold
+ [Spend five minutes helping the next worker calibrate a patched contact band.]
    // ghostlight.action_label: assist
    // ghostlight.branch_label: prep_peer_band
    // ghostlight.intent: improve shared survival at the cost of counted time
    ~ worker_cohesion = worker_cohesion + 1
    ~ route_knowledge = route_knowledge + 1
    ~ claimshare_pressure = claimshare_pressure + 1
    Tavi braces the patched band between three arms while the other worker rolls the steel nut along the contact points.

    Green. Green. Amber. Tavi rotates the band one sucker-width. Green.

    Rook marks five unpaid minutes on the manual card. "We have committed maintenance at the scene of maintenance," he says. "Try to look remorseful."
    -> routine_fold

=== routine_fold ===
// ghostlight.fold: ordinary_rig_preparation
// ghostlight.visual_scene_id: kappa_routine_fold
The bay settles into its familiar choreography. Sela checks oxygen flow. Rook inventories clamps. Tavi tests each contact band against the rail, one arm after another, while the second octopoid worker folds toward the neighboring throat.

{rig_margin >= 4: The new membrane lays a cool, even mist across Tavi's mantle. Their breathing loop does not have to argue with the room.}
{rig_margin == 3: The pooled adapter sits slightly crooked and holds. Three worker marks shine on its custody strip.}
{arrears_evidence >= 1: Tavi's manual record card carries a fresh physical mark no remote ledger can quietly revise.}
{worker_cohesion >= 3: The sealed membranes remain in the wall sleeve where every worker can see them. Nobody calls that ownership.}
{claimshare_pressure >= 2: The shift clock has already placed a small red deduction beside Tavi's name.}
{route_knowledge >= 2: Tavi has checked both the body's fit and the route's remembered hazards before the first crawl release.}

Then every harness light in the bay turns amber at once.

-> arrears_notice

=== arrears_notice ===
// ghostlight.scene: batch_service_arrears
// ghostlight.visual_scene_id: kappa_arrears_notice
The support remains on. Air still moves through Tavi's oxygenation loop. Mist still beads their skin. Nothing dramatic shuts down, because the body is not the invoice's most convenient victim yet.

The shift board posts a quieter sentence:

BIOELEVATE BATCH SERVICE: ELEVEN DAYS IN ARREARS. CURRENT SUPPORT CONTINUES. NEW CONSUMABLE ISSUE AND RECERTIFICATION PAUSED.

Below it, AU adds an equipment-balance form assigning each fitted harness to its worker until return, payment, or review.

Sela's jaw goes still. "My signature remains good until end of shift," she says. "It covers the rig in its recorded envelope. It does not cover Kappa-7, your judgment, or AU's debt."

Rook looks at the two sealed membranes. "How reassuring. The air is financed in separate departments."

-> arrears_choice

=== arrears_choice ===
// ghostlight.choice_layer: arrears_response
// ghostlight.visual_scene_id: kappa_arrears_choice
+ [Press a sucker to the copy pad and refuse the equipment-balance form.]
    // ghostlight.action_label: refuse_document
    // ghostlight.branch_label: arrears_refuse_balance
    // ghostlight.intent: deny conversion of AU's invoice into personal equipment debt
    ~ arrears_evidence = arrears_evidence + 2
    ~ superintendent_pressure = superintendent_pressure + 1
    Tavi leaves a clean ringed print on the refusal field and transfers the receipt to the manual record card.

    The central ledger receives the refusal. The manual card keeps it too. One of those records belongs to the people being billed.
    -> gate_release
+ [Sign the balance so the assigned harness cannot be collected from the prep rail today.]
    // ghostlight.action_label: sign_document
    // ghostlight.branch_label: arrears_sign_balance
    // ghostlight.intent: preserve immediate bodily access by accepting future claim pressure
    ~ lease_balance_signed = 1
    ~ claimshare_pressure = claimshare_pressure + 2
    ~ superintendent_pressure = superintendent_pressure - 1
    Tavi touches the signature pad.

    The harness assignment stays green. A new balance settles against Tavi's claimshare line, polite as dust.
    -> gate_release
+ [Move one sealed refill into the crew sleeve and close the custody strip with Sela and Rook.]
    // ghostlight.action_label: transfer_object
    // ghostlight.branch_label: arrears_pool_refill
    // ghostlight.intent: keep life support available outside supplier release
    ~ crew_reserve = crew_reserve - 1
    ~ rig_margin = rig_margin + 1
    ~ worker_cohesion = worker_cohesion + 1
    ~ arrears_evidence = arrears_evidence + 1
    Three marks close the custody strip: sucker ring, fitter stamp, grease pencil.

    The refill is now fully accounted for and much harder to confiscate by accident.
    -> gate_release
+ [Ask Sela to state exactly what her signature covers while Rook records it.]
    // ghostlight.action_label: speak
    // ghostlight.branch_label: arrears_bound_signature
    // ghostlight.intent: separate rig condition, route safety, and operator debt in the record
    ~ arrears_evidence = arrears_evidence + 1
    ~ route_knowledge = route_knowledge + 1
    ~ superintendent_pressure = superintendent_pressure + 1
    "Rig fit," Tavi says through the harness speaker. Their voice is low and wet around the consonants. "Route?"

    "Not mine," Sela says.

    "Debt?"

    "Also not mine. I am having an excellent morning."

    Rook records all three answers.
    -> gate_release

=== gate_release ===
// ghostlight.scene: kappa_gate_rejection
// ghostlight.visual_scene_id: kappa_gate_rejection
Tavi crosses from the prep bay to the outward crawl-throat gate. The harness crawls and braces with them: two arms on the low guide rail, two around the tool case, four free to work.

They touch the utility interlock socket to the release plate.

The gate stays amber.

RIG SERVICE CURRENT UNTIL SHIFT END. NEW KAPPA CRAWL RELEASE NOT ACCEPTED UNDER YARD COVERAGE.

The distinction is beautifully maintained. Tavi may continue breathing in the dry yard. Tavi may not enter the route AU fitted their body to reach.

-> default_cascade

=== default_cascade ===
// ghostlight.fold: invoice_to_habitat_margin
// ghostlight.visual_scene_id: kappa_default_cascade
The Kappa board recruits one default at a time.

No accepted crawl release means no Kappa-7 sensor-nest inspection. No inspection means the seal-lung cluster stays on temporary bypass. The amber pressure reserve loses an hour. The thermal bar loses two. The heavy yard cycle slides past its freight transfer. Paid hours flicker. Claimshare conversion moves from green to review.

{claimshare_pressure >= 3: Tavi's own line now shows harness balance, lost release time, and reduced conversion in three separate columns. The sum is not displayed.}
{arrears_evidence >= 3: The manual card already joins the unpaid batch invoice, current support condition, and rejected crawl release into one physical chain.}
{worker_cohesion >= 3: Workers from the neighboring throat stop pretending the amber lights belong to somebody else's contract.}
{lease_balance_signed == 1: AU's board lists Tavi as the responsible holder of an assembly AU has not paid to service.}
{route_knowledge >= 2: Tavi knows the bypass can preserve the yard atmosphere without clearing the heavy cycle. The narrower mercy is still physically available.}

The operations-gallery door opens onto the badge stair.

-> superintendent_arrival

=== superintendent_arrival ===
// ghostlight.scene: authority_short_route
// ghostlight.visual_scene_id: kappa_superintendent_arrival
Ione Malk descends in a clean charcoal pressure jacket while the workers remain held at the throat gate. She reaches the ring before the freight clock loses another minute. The building has kept its promise to her.

"Temporary calibration exception," she says. "Tavi enters Kappa-7. We clear the sensor nest. Payroll restores the shift when the cycle posts."

Sela looks up at the gallery window, then at her own fitter mark. "My signature does not become larger because your schedule got smaller."

"I can own the exception."

Tavi feels the oxygenation canister pulse against their mantle.

Rook unstraps the manual bypass plate. "You can own a field in the incident form. The seal lung may decline your promotion."

-> response_choice

=== response_choice ===
// ghostlight.choice_layer: gate_and_bypass_response
// ghostlight.visual_scene_id: kappa_response_choice
+ [Offer Sela the manual card and request a witnessed entry only as far as the crawl throat.]
    // ghostlight.action_label: show_document
    // ghostlight.branch_label: response_limited_entry
    // ghostlight.intent: preserve a bounded support action without laundering it into full route clearance
    ~ limited_entry_record = 1
    ~ yard_margin = yard_margin + 1
    ~ arrears_evidence = arrears_evidence + 1
    Tavi slides the manual card beneath the gate scanner but does not connect it to the corporate ledger.

    Sela marks a witnessed limit: throat mouth, visible coupling, no blind crawl, no Kappa-7 acceptance.

    Ione says, "That does not clear production."

    Tavi says, "Air first."
    -> shift_threshold
+ {crew_reserve >= 1} [Pass the strike-safe bypass plate to Rook and guide him from the work-support rail.]
    // ghostlight.action_label: transfer_object
    // ghostlight.branch_label: response_human_bypass
    // ghostlight.intent: use shared route knowledge without putting the unsupported body in the crawl
    ~ worker_cohesion = worker_cohesion + 2
    ~ yard_margin = yard_margin + 1
    ~ crew_reserve = crew_reserve - 1
    Rook locks the manual plate onto the human-scale manifold. Tavi holds the route diagram with two arms and signals valve timing with two more.

    The work is slower from outside the throat. It is also work a baseline body can survive.

    Behind them, another worker moves the remaining refill closer to Tavi without waiting for a request.
    -> shift_threshold
+ [Accept Ione's exception and connect the harness to the Kappa-7 release plate.]
    // ghostlight.action_label: accept_override
    // ghostlight.branch_label: response_production_exception
    // ghostlight.intent: preserve wages and freight by taking the route risk into the worker's body
    ~ production_exception = 1
    ~ rig_margin = rig_margin - 1
    ~ claimshare_pressure = claimshare_pressure - 1
    The gate turns white under Ione's override.

    The insurer line remains amber. Sela's signature remains the same size. Tavi folds through the throat anyway, carrying the schedule on their back beside the oxygenation canister.
    -> shift_threshold
+ [Press the rejected release, unpaid invoice, and nonattendance code into one manual receipt.]
    // ghostlight.action_label: create_record
    // ghostlight.branch_label: response_join_default
    // ghostlight.intent: make the cascade legible before management separates it into harmless categories
    ~ arrears_evidence = arrears_evidence + 2
    ~ superintendent_pressure = superintendent_pressure + 2
    ~ claimshare_pressure = claimshare_pressure + 1
    Tavi braces the manual card against the rail. Sela stamps current support. Rook copies the rejected release. Tavi adds the nonattendance code AU has prepared for a body standing visibly at work.

    The card is crowded now. The truth has poor layout discipline.
    -> shift_threshold

=== shift_threshold ===
// ghostlight.fold: final_cost_selection
// ghostlight.visual_scene_id: kappa_shift_threshold
Kappa's temporary bypass exhales through the manifold wall. The pressure sound is felt first through the rail, then heard: a deep cough moving around the ring.

{yard_margin >= 4: The manual bypass has bought the industrial atmosphere another hour. The heavy cycle remains uncleared.}
{yard_margin <= 3: The reserve bars shorten. The next isolation step will close the yard edge and send the cost into housing airflow and freight.}
{limited_entry_record == 1: Sela's witnessed limit glows on the manual card: throat mouth, visible coupling, no blind crawl.}
{production_exception == 1: Kappa-7 is open under Ione's override, white gate light laid over an amber coverage line.}
{arrears_evidence >= 4: The joined receipt shows one chain where AU's dashboard shows departments.}
{worker_cohesion >= 4: The other shift lines have stopped moving toward their gates. Nobody has called a meeting. They have simply become difficult to schedule separately.}
{crew_reserve <= 0: The custody sleeve is empty. Mutual aid has spent its last compatible part.}
{rig_margin <= 1: Tavi's humidity collar pulses unevenly. The decision is now being made inside the body faster than anyone can word it.}

The freight clock, the pressure reserve, the claimshare board, and Tavi's breathing loop all continue counting in different units.

Tavi chooses which account gets to be late.

-> final_choice

=== final_choice ===
// ghostlight.choice_layer: dependency_priority
// ghostlight.visual_scene_id: kappa_final_choice
+ [Spend the crew kit on the manual bypass and preserve civilian air margin.]
    // ghostlight.action_label: use_object
    // ghostlight.branch_label: prioritize_air_margin
        {yard_margin >= 4 && crew_reserve >= 1:
        Tavi coils two arms around the support rail and gives Rook the slow valve sequence. The crew opens the kit under shared custody.
        -> ending_air_margin_success
    - else:
        Tavi reaches for the kit. The remaining parts cannot buy the full hour the board demands.
        -> ending_air_margin_cost
    }
+ {production_exception == 1} [Finish the Kappa-7 crawl under Ione's production exception.]
    // ghostlight.action_label: move
    // ghostlight.branch_label: prioritize_production
        {rig_margin >= 2 && route_knowledge >= 2:
        Tavi folds past the throat ribs with enough bodily margin to keep judgment ahead of the schedule.
        -> ending_production_success
    - else:
        Tavi enters on a thin rig margin or an exception nobody has made materially safer.
        -> ending_production_cost
    }
+ [Seal the joined receipt into crew custody and refuse the blind crawl.]
    // ghostlight.action_label: refuse_and_transfer
    // ghostlight.branch_label: prioritize_proof
        {arrears_evidence >= 4 && superintendent_pressure >= 2:
        Sela closes the receipt sleeve. Rook adds the bypass reading. Tavi leaves the release socket untouched.
        -> ending_proof_success
    - else:
        Tavi refuses, but the records remain separated across systems that each prefer innocence.
        -> ending_proof_cost
    }
+ [Return to the prep bay with the other fitted workers and keep the harnesses alive together.]
    // ghostlight.action_label: withdraw
    // ghostlight.branch_label: prioritize_workers
        {worker_cohesion >= 4 && rig_margin >= 2:
        Tavi turns from the throat. The next worker turns too. Then the next.
        -> ending_workers_success
    - else:
        Tavi turns, but the line does not yet move as one body.
        -> ending_workers_cost
    }

=== ending_air_margin_success ===
// ghostlight.ending_label: air_margin_success
// ghostlight.visual_scene_id: kappa_ending_air
// ghostlight.training_hook: mutual_aid_preserves_life_without_clearing_production
The bypass plate seats against the manifold. Rook works the human-scale valves. Tavi calls timing from the rail. Sela watches the harness, not the production board.

The pressure reserve climbs out of red. Housing airflow stays open. The heavy cycle remains cancelled, the freight transfer leaves without AU's load, and nobody pretends preserving air has also preserved profit.

The crew sleeve holds one compatible refill. It is enough for the next body, perhaps. An hour of mercy, itemized by three workers and owned by none of their employers.
-> END

=== ending_air_margin_cost ===
// ghostlight.ending_label: air_margin_cost
// ghostlight.visual_scene_id: kappa_ending_air
// ghostlight.training_hook: finite_mutual_aid_cannot_cover_missing_system_margin
The manual plate opens the bypass. The pressure bar rises, stalls, and begins falling again.

The kit can furnish a plate and perhaps one compatible refill. It cannot furnish the missing service margin for a full hour. Sela keeps Tavi's oxygenation loop stable while Rook closes the yard edge one section at a time. Housing air survives. Freight does not. The next shift will inherit a smaller margin and a painfully accurate custody sleeve.

Mutual aid is real. So is inventory.
-> END

=== ending_production_success ===
// ghostlight.ending_label: production_success_with_personalized_risk
// ghostlight.visual_scene_id: kappa_ending_production
// ghostlight.training_hook: short_term_throughput_personalizes_operator_default
Inside Kappa-7, Tavi braces across the curved ribs and finds the shifted sensor bracket before the seal lung flexes. They mark it, withdraw, and refuse Ione's request to call the route clear until the bracket is physically reset.

The heavy cycle runs late. Wages post. The freight load catches the last transfer.

AU's ledger credits the exception to management and the exposure to Tavi. The batch invoice remains unpaid. The machine has purchased one day by borrowing it from the same body.
-> END

=== ending_production_cost ===
// ghostlight.ending_label: production_cost
// ghostlight.visual_scene_id: kappa_ending_production
// ghostlight.training_hook: exception_does_not_create_bodily_margin
The seal lung coughs while Tavi is between ribs.

Their humidity collar slips. One cuff bites pale into an arm. Tavi gets the tool rail across the valve nest before the membrane flex reaches the mantle, but the withdrawal costs skin, breath, and the rest of the shift.

The inspection posts incomplete. Freight still misses transfer. AU opens an incident review to determine whether support-rig noncompliance caused the delay.
-> END

=== ending_proof_success ===
// ghostlight.ending_label: joined_default_proof
// ghostlight.visual_scene_id: kappa_ending_proof
// ghostlight.training_hook: workers_preserve_dependency_chain_as_evidence
Sela seals the sleeve. Unpaid batch invoice. Current rig condition. Rejected crawl release. Prepared nonattendance code. Falling bypass margin. Rook's grease-pencil transfer time.

Ione can still stop the shift. She can no longer make each consequence arrive alone.

The crew sends copies outward by different routes. Nothing is won in the ring. But when AU calls the next missed inspection a worker failure, the failure will already have its creditors attached.
-> END

=== ending_proof_cost ===
// ghostlight.ending_label: separated_records_cost
// ghostlight.visual_scene_id: kappa_ending_proof
// ghostlight.training_hook: administrative_partition_absorbs_partial_truth
Tavi refuses the throat. Sela preserves the fit record. Rook preserves the bypass reading.

AU keeps the invoice in procurement, the rejected badge in access control, and the lost hours in attendance. Each record is accurate. Together they are absent.

By end of shift, Tavi has a clean refusal receipt and a dirtier claimshare line. The yard has learned nothing it is required to admit.
-> END

=== ending_workers_success ===
// ghostlight.ending_label: worker_support_line
// ghostlight.visual_scene_id: kappa_ending_workers
// ghostlight.training_hook: cross_role_mutual_aid_before_formal_movement
Tavi returns to the prep bay. The second octopoid worker follows. Rook rolls the bypass cart away from the production throat and toward the civilian manifold. Sela keeps every already-fitted harness under observation without issuing a false route signature.

No speech begins it. Nobody has a banner. The line simply stops sorting itself into fitter, rigger, uplift, and equipment holder long enough to keep the breathing loops wet.

Kappa remains amber. The workers remain present. For one shift, those are separate facts AU cannot reconcile.
-> END

=== ending_workers_cost ===
// ghostlight.ending_label: isolated_worker_cost
// ghostlight.visual_scene_id: kappa_ending_workers
// ghostlight.training_hook: bodily_refusal_before_solidarity_is_ready
Tavi returns to the prep bay alone.

Rook stays at the cart because his household needs the posted hours. The next octopoid worker signs the equipment balance because their humidity collar has already begun to pulse dry. Sela keeps Tavi's rig alive and cannot make the others able to refuse.

Kappa remains amber. AU records one nonattendance. The tiny room for hope is still there, but today it contains two people and one compatible refill.
-> END
