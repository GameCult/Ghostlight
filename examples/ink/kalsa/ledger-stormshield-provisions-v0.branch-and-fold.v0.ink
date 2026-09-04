// ghostlight.artifact_id: kalsa_ledger_stormshield_provisions_branch_fold_v0
// ghostlight.fixture_id: ledger-stormshield-provisions-v0
// ghostlight.scene_id: ledger-stormshield-provisions-v0.outer-road-provision-recall
// ghostlight.final_ink_path: examples/ink/kalsa/ledger-stormshield-provisions-v0.branch-and-fold.v0.ink

VAR paired_record = 1
VAR worker_scope = 1
VAR current_watch_issue = 1
VAR relief_readiness = 1
VAR attendant_issue = 2
VAR central_pressure = 1
VAR shortage_signal = 0
VAR wet_loss_known = 0
VAR kalo_standing = 1
VAR aru_confidence = 2
VAR central_transfer = 0

-> start

=== start ===
The roofed provision landing sits one dry stair below Ti'asantatca's outer-road stormshield station.

It has a broad cart arch on the cityward side, a stone drain at the roadward edge, a slatted rack for wet loads, and a barred dry store between them. A waist-high tally bench guards the distance from rack to store. The bench is less impressive than the barrier above and receives almost as many arguments.

Four store bays face it. One feeds the current watch and its recovery room. One travels with the relief cohort. One feeds tenders, observers, runners, porters, and family caregivers. One holds the contributor return owed to households that supplied grain or lost working hands to the station.

Calling all four bays "shield grain" is quicker. So is falling down the stair.

-> morning_people

=== morning_people ===
Seni Var keeps the landing's station tallies. She can accept a load, refuse an unmarked release, and make a disputed taking survive long enough to embarrass someone with better clothes. She cannot order the shield above, command a tribal store, or improve a wet sack by looking official beside it.

Pera Oth comes down from the recovery room carrying three empty broth bowls and one opinion.

"The outgoing holder ate," Pera says. "The observer forgot. The porter claimed steam counts as breakfast."

Kalo Rei, the porter in question, shoulders a dry grain sack off his household's cart. His aunt supplied this load and one relief worker. He carries her matching clay tally wrapped in waxed cloth at his belt.

"Steam is what breakfast becomes after administration," Kalo says.

The storm makes the arch cloth breathe inward. Above, a bronze handbell marks an ordinary watch correction. Nothing is failing. That is when provisioning is supposed to work.

-> morning_issue_choice

=== morning_issue_choice ===
// ghostlight.choice_layer: ordinary_morning_issue
+ [Set Kalo's contributor half beside the station tally and compare every mark before storing the grain.]
    // ghostlight.action_label: use_object
    // ghostlight.branch_label: pair_contributor_tally
    ~ paired_record = paired_record + 2
    ~ kalo_standing = kalo_standing + 1
    Seni unwraps Kalo's clay half and places it beside the peg-board piece.

    Source marks agree. Dry condition agrees. The load is divided between current watch and contributor return. The promised return names Kalo's aunt's household, not the tribal patron who provided the cart.

    Kalo exhales through his nose. A matching tally is not food, but it has occasionally prevented food from acquiring a more prestigious ancestor.
    -> morning_fold
+ [Cut the sack cord at the drain and test the bottom grain for storm damp.]
    // ghostlight.action_label: touch_object
    // ghostlight.branch_label: inspect_load_condition
    ~ wet_loss_known = wet_loss_known + 2
    ~ paired_record = paired_record + 1
    Seni rolls the sack onto the slatted rack and reaches into the lowest fold.

    Most of the grain runs dry. One corner clumps cold against her fingers where the cart cover leaked. She separates that portion into a shallow drying tray and scratches the condition beside the station mark.

    Pera peers over her shoulder. "Congratulations. You have discovered weather indoors."

    "I will invoice the storm."
    -> morning_fold
+ [Issue Pera and Kalo their meal before the bowls become an argument about whether support work counts.]
    // ghostlight.action_label: transfer_object
    // ghostlight.branch_label: feed_support_workers
    ~ worker_scope = worker_scope + 2
    ~ attendant_issue = attendant_issue - 1
    ~ kalo_standing = kalo_standing + 1
    Seni measures grain into Pera's smallest bowl, then into the one Kalo had hoped was decorative.

    She hangs both meal marks over the attendant bay.

    "Porter," she says.

    "Shield porter?" Kalo asks.

    "Do not become ambitious while chewing."
    -> morning_fold
+ [Carry the relief ration to the stair shelf so the next cohort can take it without opening the dry store.]
    // ghostlight.action_label: move
    // ghostlight.branch_label: stage_relief_ration
    ~ relief_readiness = relief_readiness + 2
    ~ central_pressure = central_pressure + 1
    Seni lifts the tied relief sack and sets it on the raised shelf beside the dry stair.

    It now has a short path to the next cohort and a very clear silhouette to anyone entering through the cart arch.

    Pera nods approval. Kalo looks toward the arch.

    "Useful things become visible," he says.

    "That is why we give them records."
    -> morning_fold

=== morning_fold ===
// ghostlight.fold: ordinary_provision_routine
Pera washes the bowls at the drain. Kalo brushes loose grain from the cart bed into a hand measure. Seni hangs the morning tallies over their separate bays.

{paired_record >= 3: Kalo's household half and the station half sit together long enough for every notch and impressed shape to answer its twin.}
{paired_record <= 1: The contributor half remains at Kalo's belt while the station board carries the only visible account.}
{wet_loss_known >= 2: A shallow tray of darkened grain dries apart from the sound load, its loss visible before anyone can call the whole sack good.}
{worker_scope >= 3: Pera's and Kalo's empty bowls now hang as work evidence above the attendant bay.}
{attendant_issue <= 1: The support bay has fed two bodies and looks correspondingly less theoretical.}
{relief_readiness >= 3: The relief ration waits on the stair shelf, sealed and ready for hands that have not arrived yet.}

The broad cart arch darkens.

-> recall_arrival

=== recall_arrival ===
Aru Venn steps in under a storm cloak with a clay order-board against her chest. She is a city store delegate for this delivery, which gives her authority over the loads named on her copy and no special talent for making one sack become two.

Two planned carts have failed to reach the central refuges. One lost an axle below a runoff cut. The other turned back with a soaked cover. The active central watches still need their next issue.

Aru lays the order-board on Seni's tally bench. Its impressed marks request the outer-road station's grain reserve.

"The central count says you have four bays," Aru says.

"The wall says we have four bays," Seni replies. "The count says what each one is for."

"The central watch cannot eat purposes."

Above them, the handbell rings twice. Not failure. A request to extend one current overlap.

-> recall_order_choice

=== recall_order_choice ===
// ghostlight.choice_layer: reserve_recall
+ [Align Aru's order-board with all four station tallies before opening any bay.]
    // ghostlight.action_label: use_object
    // ghostlight.branch_label: compare_recall_scope
    ~ paired_record = paired_record + 1
    ~ wet_loss_known = wet_loss_known + 1
    ~ central_pressure = central_pressure + 1
    ~ aru_confidence = aru_confidence - 1
    Seni places the order beneath the four pegs.

    The board names station grain reserve. It does not name current issue, relief ration, attendant issue, contributor return, wet loss, or the household whose copy has already left the landing.

    Aru watches a short order grow edges.

    "You know what the central store meant."

    "I know what it pressed into clay."
    -> recall_fold
+ [Open only the current-watch bay and let Aru's carriers take one sound sack immediately.]
    // ghostlight.action_label: transfer_object
    // ghostlight.branch_label: send_current_issue
    ~ current_watch_issue = current_watch_issue - 1
    ~ central_transfer = central_transfer + 2
    ~ aru_confidence = aru_confidence + 1
    ~ central_pressure = central_pressure - 1
    Seni lifts the bar and rolls one current-watch sack to the cart arch.

    Aru marks receipt. The central holders will eat sooner. The outer-road recovery room has one less full issue before its own next handoff.

    Pera counts the remaining sacks without moving her lips. It is somehow louder that way.
    -> recall_fold
+ [Keep the dry-store bar in place and strike the landing's shortage clapper.]
    // ghostlight.action_label: use_object
    // ghostlight.branch_label: hold_and_signal_shortage
    ~ shortage_signal = shortage_signal + 2
    ~ central_pressure = central_pressure + 2
    ~ aru_confidence = aru_confidence - 1
    Seni strikes the small iron clapper beside the arch.

    Its sound goes up the dry stair and down the cart ramp: disputed release, counted shortage, send an owner who can narrow the claim.

    It also tells every waiting body that grain exists behind a bar.

    Aru's mouth tightens. "You have made a store problem public."

    "It arrived public. I have made it audible."
    -> recall_fold
+ [Bring Pera and Kalo to the bench and count every body attached to the four bays.]
    // ghostlight.action_label: speak
    // ghostlight.branch_label: count_supporting_bodies
    ~ worker_scope = worker_scope + 1
    ~ kalo_standing = kalo_standing + 1
    ~ central_pressure = central_pressure + 1
    ~ aru_confidence = aru_confidence - 1
    Seni makes Pera name the people upstairs: two active holders, an incoming relief pair, an observer, two tenders, a runner, and the porter due to carry the next sealed ration.

    Kalo names the household that supplied this morning's grain and the aunt missing a worker from her field.

    Aru taps the order-board. "The request is for shield provision."

    Pera lifts an empty broth bowl. "Excellent. We found some."
    -> recall_fold

=== recall_fold ===
// ghostlight.fold: recall_enters_local_account
The four bays remain physically unchanged by the quality of the argument.

{central_transfer >= 2: One sound sack now waits beside Aru at the cart arch, already receipted for the central watches.}
{shortage_signal >= 2: The iron clapper's last note still trembles in the arch while footsteps answer somewhere above.}
{paired_record >= 3: The recall order lies under a spread of narrower tallies whose marks make each promised use visible.}
{aru_confidence >= 3: Aru stands as if the central meaning has already won and only lifting remains.}
{aru_confidence <= 1: Aru has begun rereading her own order instead of Seni's face.}
{worker_scope >= 3: Empty bowls, porter measure, and tender tokens sit on the bench as evidence that the barrier eats through more than trance.}

Aru touches the attendant peg.

"The central copy counts active holders," she says. "This bay is household support."

Pera's face becomes pleasantly blank. Kalo stops brushing grain.

Then Aru touches the contributor-return peg.

"And this is repayment. Neither is current shield issue."

That classification would feed the central watches now. It would also make the people who cooked, carried, recovered, and contributed disappear from the shield account while their grain remained conveniently present.

-> who_counts_choice

=== who_counts_choice ===
// ghostlight.choice_layer: labor_and_promise_classification
+ [Join Kalo's contributor half to the station half and make Aru read the promised return aloud.]
    // ghostlight.action_label: show_object
    // ghostlight.branch_label: establish_household_claim
    ~ paired_record = paired_record + 2
    ~ kalo_standing = kalo_standing + 2
    ~ central_pressure = central_pressure + 1
    Seni holds out her hand. Kalo gives her the waxed cloth.

    The two clay pieces meet on the bench. Source, dry share, intended use, and household return match.

    Aru reads the marks without ornament. Kalo's aunt supplied grain and a relief worker. The station owes her household a named return.

    "Owes," Kalo repeats softly. The word has acquired weight by being spoken in a better cloak.
    -> classification_fold
+ [Hang Pera's bowls, Kalo's hand measure, and the runner's ration token beneath the attendant peg.]
    // ghostlight.action_label: move
    // ghostlight.branch_label: count_support_as_shield_work
    ~ worker_scope = worker_scope + 2
    ~ kalo_standing = kalo_standing + 1
    ~ aru_confidence = aru_confidence - 1
    Seni moves the work evidence one piece at a time.

    Bowl for the tender who catches a released holder. Measure for the porter who gets grain up the narrow stair. Token for the runner who carries the next warning down it.

    "Household support," Aru says again, but the phrase now has to step around several actual households doing actual support.
    -> classification_fold
+ [Move the staged relief ration to Aru's outgoing cart and mark the relief route spent.]
    // ghostlight.action_label: transfer_object
    // ghostlight.branch_label: spend_relief_ration
    ~ central_transfer = central_transfer + 2
    ~ relief_readiness = relief_readiness - 2
    ~ aru_confidence = aru_confidence + 1
    ~ central_pressure = central_pressure - 1
    Seni lifts the relief sack from the stair shelf and rolls it beside the first transfer.

    She leaves the relief peg empty and scratches one hard line across its tally: reassigned before arrival.

    The central watches gain a meal. The replacement cohort now has a wet road, a climb, and whatever they carried for themselves.

    Pera looks up the stair. "I will tell the next exhausted person that arithmetic ate first."
    -> classification_fold
+ [Leave every peg where it is and strike the shortage clapper a second time.]
    // ghostlight.action_label: use_object
    // ghostlight.branch_label: escalate_shortage_signal
    ~ shortage_signal = shortage_signal + 2
    ~ central_pressure = central_pressure + 2
    ~ aru_confidence = aru_confidence - 1
    The second strike means the first dispute remains and the station will not let silence settle it.

    The sound brings faces to the upper stair and the cart ramp. Not a mob. Witnesses are more troublesome because they can remember the order of things.

    Aru lowers her voice. "If the central issue fails, your clapper will not feed anyone."

    "No," Seni says. "It will tell them who decided where the food went."
    -> classification_fold

=== classification_fold ===
// ghostlight.fold: four_uses_under_one_shortage
Storm light fills the cart arch. Warm stair light falls across the barred store. Between them, the tally bench holds a central order and several smaller promises that refuse to become the same shape.

{paired_record >= 5: Kalo's paired clay pieces sit joined over the contributor-return peg, making the household promise difficult to recite as spare stock.}
{kalo_standing >= 4: Kalo now stands at the bench as the named carrier for a named contributor, not as scenery attached to a cart.}
{worker_scope >= 4: Bowls, measure, and ration token form a blunt little anatomy of the shield's support body.}
{relief_readiness >= 3: The sealed relief ration remains on the stair shelf, ready for the replacement cohort.}
{relief_readiness <= 0: The relief shelf is bare and its crossed tally says why.}
{central_transfer >= 4: Two sound sacks stand at the cart arch for the central watches. The immediate shortage has begun to move.}
{shortage_signal >= 3: Witnesses have gathered at both routes, and the dispute can no longer fit entirely inside Seni's job.}
{wet_loss_known >= 2: The separated drying tray proves which grain is damp and which sacks can travel without lying about their condition.}

Footsteps stop on the dry stair above.

The Circle lead does not descend. A runner calls the operational need from the landing: extend the present overlap, feed the active holders, preserve a viable relief arrival. No instruction names contributor return or erases attendant issue.

Aru's central order remains real. So does the shortage that produced it.

Seni must decide what leaves before a higher argument arrives too late to carry a sack.

-> final_choice

=== final_choice ===
// ghostlight.choice_layer: material_release
+ [Release only a sound current-watch sack; keep relief, attendant, and contributor bays under their own tallies.]
    // ghostlight.action_label: transfer_object
    // ghostlight.branch_label: prioritize_bounded_release
    {paired_record >= 3 && worker_scope >= 2 && current_watch_issue >= 1:
        Seni bars three bays, rolls the remaining current-watch sack to Aru, and makes both copies name the bounded release.
        -> ending_bounded_release_success
    - else:
        Seni names the bounded release, but the local record and body count are too thin to keep the other bays from being called reserve after the cart leaves.
        -> ending_bounded_release_cost
    }
+ [Feed the active watches from current issue and contributor return; preserve relief and attendants.]
    // ghostlight.action_label: transfer_object
    // ghostlight.branch_label: prioritize_immediate_watch
    {wet_loss_known >= 2 && current_watch_issue >= 1:
        Seni selects sound grain from current issue and the promised household return, marking both sources before the sacks cross the arch.
        -> ending_immediate_watch_success
    - else:
        Seni opens current issue and contributor return without enough condition evidence or local depth to know whether the transfer will hold.
        -> ending_immediate_watch_cost
    }
+ [Keep relief and attendant issue intact, ring the shortage publicly, and make the current overlap wait on a smaller meal.]
    // ghostlight.action_label: use_object
    // ghostlight.branch_label: prioritize_support_web
    {relief_readiness >= 3 && worker_scope >= 2 && shortage_signal >= 1:
        Seni leaves the relief sack and support bay sealed, releases a reduced current issue, and strikes the clapper for a witnessed shortfall.
        -> ending_support_web_success
    - else:
        Seni tries to protect the support web, but too little labor, relief, or public evidence has been assembled for the hold to survive the next demand.
        -> ending_support_web_cost
    }
+ [Accept the central classification and release all four bays under Aru's order.]
    // ghostlight.action_label: open_object
    // ghostlight.branch_label: prioritize_total_recall
    {aru_confidence >= 2 && central_pressure <= 3:
        Seni lifts the bar and lets the central order gather every station use into one outgoing count.
        -> ending_total_recall_clean
    - else:
        Seni lifts the bar after the classification has already become a public dispute.
        -> ending_total_recall_contested
    }

=== ending_bounded_release_success ===
// ghostlight.ending_label: bounded_release_success
// ghostlight.training_hook: separate_material_uses_under_pressure
One sound sack leaves under a current-watch mark.

The relief ration remains by the stair. Pera's bay stays barred. Kalo takes his aunt's contributor half home beside a station copy that still promises return.

The central watches receive less than Aru wanted and more than Seni could honestly call spare. Above, the overlap continues on a smaller meal while another store is asked to answer.

Nothing about the result is generous. Its virtue is merely that each cost keeps its own name.
-> END

=== ending_bounded_release_cost ===
// ghostlight.ending_label: bounded_release_cost
// ghostlight.training_hook: bounded_claim_without_evidence_depth
Seni sends one sack and bars three bays.

By the time the cart reaches the central ramp, its receipt says station grain. The narrower uses remain on Seni's board, unsupported by enough paired copies or visible workers to stop the next delegate calling them residue.

The boundary was spoken. It was not yet carried by enough people or records to survive travel.
-> END

=== ending_immediate_watch_success ===
// ghostlight.ending_label: immediate_watch_success
// ghostlight.training_hook: current_protection_spends_household_return
The dry grain goes first.

Seni separates the damp corner, releases current issue, and opens the contributor-return bay under Kalo's joined tally. Aru's receipt names the household promise consumed to feed the active holders.

Upstairs, bowls fill. The present overlap does not fail for hunger.

Kalo carries no grain home. He carries the city copy of a debt his aunt can contest, assuming the next shortage has not learned to eat copies too.
-> END

=== ending_immediate_watch_cost ===
// ghostlight.ending_label: immediate_watch_cost
// ghostlight.training_hook: urgent_transfer_without_condition_or_depth
Seni opens current issue and contributor return.

The cart leaves heavy. At the first turn, damp grain settles against sound grain inside an uninspected sack. Pera keeps enough for thin broth and nothing for error.

The active watches may still eat. Kalo's household has already paid, and the receipt cannot say exactly what quality of promise replaced the load.
-> END

=== ending_support_web_success ===
// ghostlight.ending_label: support_web_success
// ghostlight.training_hook: future_capacity_and_support_labor_preserved
The clapper sounds a third time.

Seni releases a reduced current sack. The relief ration remains sealed by the stair. Pera keeps grain for the hands that will receive exhausted bodies, carry warnings, and climb the wet road with the next issue.

The central overlap gets less food now. It also keeps a credible replacement and the labor needed to turn release into recovery.

Aru leaves with a short load and a crowded witness list. Scarcity has not become kindness. It has merely failed to make half the shield invisible.
-> END

=== ending_support_web_cost ===
// ghostlight.ending_label: support_web_cost
// ghostlight.training_hook: support_claim_without_public_or_material_backing
Seni bars relief and attendant issue.

The landing has too few witnesses, too little staged relief, or too weak an account of support labor to hold the distinction. Aru takes the current sack and returns with someone whose authority reaches farther down the ramp.

Pera begins hiding one bowl of grain before each watch. Kalo advises his aunt to bring no second cart until both clay halves are in the same room.

The support web survives by becoming less legible to the institution that needs it.
-> END

=== ending_total_recall_clean ===
// ghostlight.ending_label: total_recall_clean
// ghostlight.training_hook: central_safety_buys_later_supply_failure
Seni lifts the bar.

Current issue, relief ration, attendant grain, and contributor return become one orderly cart load. Aru's receipt is clean. The central active holders eat before the extended overlap becomes collapse.

The outer-road station has bowls, a dry stair, and four empty bays. The incoming relief cohort reaches the landing hungry. Kalo's household receives no return. Pera cuts the next recovery broth with hot water and professional contempt.

The shield holds tonight by spending the people and promises that were supposed to make it hold tomorrow.
-> END

=== ending_total_recall_contested ===
// ghostlight.ending_label: total_recall_contested
// ghostlight.training_hook: public_recall_exposes_authority_split
Seni lifts the bar in front of witnesses.

Every bay crosses the arch under Aru's order, but Kalo keeps his household half, Pera keeps the empty bowls on the bench, and the shortage clapper has already told the station that a dispute exists.

The central watches receive the grain. The cart also carries a visible claim that the city has fed one part of the shield by renaming the rest.

By the next delivery, three households send witnesses and one sends nothing.
-> END
