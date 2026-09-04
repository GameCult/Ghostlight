// ghostlight.artifact_id: kalsa_ember_final_depot_audit_branch_fold_v0
// ghostlight.fixture_id: ember-final-depot-audit-v0
// ghostlight.scene_id: ember-final-depot-audit-v0.wrong-ledger-at-final-depot
// ghostlight.final_ink_path: examples/ink/kalsa/ember-final-depot-audit-v0.branch-and-fold.v0.ink

VAR claim_copy = 0
VAR road_warning = 1
VAR office_pressure = 1
VAR depot_trust = 2
VAR ledger_custody = 1
VAR wall_evidence = 0
VAR train_exposure = 1
VAR shelter_capacity = 1
VAR seal_scope = 0

-> start

=== start ===
There is no morning at the final depot. There is a watch in which the fog thins enough to admit that the mountain has been there all along.

The depot occupies a shelf below the last imperial cut toward the Luck Crown: a roofed stone court, an open lower arch, and an upper stair passing beneath a retaining wall dark with seepage. Cold light without sunrise presses through the fog. Above the roof, one numbered road stone has been turned to face the plateau. A knotted closure cord runs from it to the lintel where any arriving porter can touch the warning before seeing it.

Families adjoining the road open the court for announced hosted crossings, small trade, and shelter. The empire left the table. Everybody else found uses for the roof.

-> depot_table

=== depot_table ===
Ari Vesk is the depot witness for this crossing watch. The title is local and narrow: keep the guest entry, preserve the guide-claim copies, maintain the warning agreed by the host families, and certify no more than the record can bear.

The old stone weigh table divides the court. Current guest tablets lie at its lower end. At the upper end, beneath an oilcloth, waits a waxed volume carrying the seal of the last Ju'onai road office.

Tera Sen, gray-haired guide claimant and daughter of one of the unpaid porters, has brought three witness cords and forty years of disapproval. Mara Il, the depot keeper, fits dry trade bundles under the eaves and heats broth in a black pot.

"A depot," Mara says, "is a tax demand that eventually develops a roof."

"This one developed soup," Ari says.

"That was resistance."

-> ordinary_watch

=== ordinary_watch ===
The audit is due before the lower carrier bell. Oren Jai, a Ju'onai recovery auditor, is climbing from the hosted path to inspect the sealed volume for a claimant who wants the old road portfolio recognized again.

Until he arrives, ordinary work remains rude enough to require doing. Fog wets the threshold. A handcart wheel complains under the lower arch. Water beads through one mortar joint in the wall above the upper stair.

-> preparation_choice

=== preparation_choice ===
// ghostlight.choice_layer: ordinary_depot_preparation
+ [Copy Tera's clearest unpaid tally onto a fresh wax tablet before the audit.]
    // ghostlight.action_label: use_object
    // ghostlight.branch_label: prepare_claim_copy
    ~ claim_copy = claim_copy + 2
    ~ office_pressure = office_pressure + 1
    Ari warms the tablet by the broth pot and presses the old names into new wax, one load and one withheld payment at a time.

    Tera checks each mark against a witness cord. "You write like a road office."

    "Slowly and under accusation?"

    "I was going to say legibly. Do not ruin the compliment."

    The copy will survive separation from the book. It will also show the auditor exactly which claimants prepared before his arrival.
    -> preparation_fold
+ [Climb the upper stair and inspect the wet retaining wall without moving the closure cord.]
    // ghostlight.action_label: move
    // ghostlight.branch_label: inspect_upper_cut
    ~ wall_evidence = wall_evidence + 2
    ~ train_exposure = train_exposure + 1
    Ari follows the cord to the upper landing. Three chalk beads set by the catchment maintainers have crept apart across a mortar seam. Fine grit lies fresh on the stair.

    Beyond the wall, the imperial cut vanishes into white fog. A side path departs behind a screen of rough stones toward the refuge the host families do not put on road-office maps.

    Ari returns with wet sleeves and a precise reason to distrust anyone who calls the closure ceremonial.
    -> preparation_fold
+ [Clear the deep eaves and fit extra load rails before the porter train arrives.]
    // ghostlight.action_label: move_object
    // ghostlight.branch_label: prepare_shelter_hold
    ~ shelter_capacity = shelter_capacity + 2
    ~ depot_trust = depot_trust + 1
    Ari and Mara shift empty baskets, set two low timber rails, and mark a dry lane from the lower arch to the store wall.

    "If the road stays closed," Mara says, "we shelter the people and stack what will spoil."

    "And what will not spoil?"

    "Outside, where it can improve its character."

    The court becomes able to receive a held train without pretending it has passed onward.
    -> preparation_fold
+ [Test the lower warning chime and record the turned stone in the current guest entry.]
    // ghostlight.action_label: touch_object
    // ghostlight.branch_label: prepare_road_warning
    ~ road_warning = road_warning + 2
    ~ depot_trust = depot_trust + 1
    Ari pulls the short cord beside the lower arch. The bronze chime answers once down the hosted path and once, faintly, from a relay stone in the fog.

    On the guest tablet Ari records: upper cut closed; numbered stone turned plateau-ward; retaining wall awaiting inspection.

    A warning that is tested becomes present work. An antique warning is merely something later officials call ambiguous.
    -> preparation_fold

=== preparation_fold ===
// ghostlight.fold: ordinary_work_before_audit
The watch resumes around the prepared choice.

Mara stirs the pot. Tera separates witness cords by touch. Ari keeps the sealed volume at the upper end of the table and the current guest record at the lower. The arrangement is not law. It is furniture doing its best.

{claim_copy >= 2: A fresh wax copy of the clearest unpaid tally cools beside Tera's cords.}
{wall_evidence >= 2: Wet grit from the upper stair darkens Ari's cuff; the shifted chalk beads are now witnessed evidence.}
{shelter_capacity >= 3: The deep eaves stand clear, with low rails ready to receive handcart loads.}
{road_warning >= 3: The lower chime has answered through the fog, and the current guest tablet names the closure.}

Boot rings touch stone outside the lower arch.

-> auditor_arrival

=== auditor_arrival ===
Oren Jai enters under the roof with an ochre travel coat wet to the knees, a brass edge gauge at his belt, and a recovery warrant wrapped in oiled cloth. He bows first to the host marks on the lintel, then to the road-office seal on the book. The order is noticed by everyone and trusted by no one.

"I am authorized to recover the final road-office ledger," he says.

"You are authorized to look for it here," Ari says. "The difference has already survived one empire."

Oren places the warrant at the visitor side of the table. Tera lays one witness cord beside it. Mara moves the broth pot farther from the paperwork, which is the closest the room comes to a neutral act.

-> wrong_ledger

=== wrong_ledger ===
Ari breaks the depot's outer thread and opens the volume beneath the old seal.

The first leaf lists guide shifts.

The second lists porter loads and payment promised.

The third lists grain taken from depot stores and never replaced.

Oren turns toward the later leaves. There, in a different hand, detour orders and supply movements occupy space left blank by the pay clerk. No closing inventory follows them.

"This is the wrong ledger," Oren says.

Tera leans over the witness rail. "For your search. It is precisely the book for mine."

Ari feels the court reorganize around the open book. Tera sees unpaid labor. Oren sees a road office that may never have closed. Mara sees a roof about to fill with people summoned by somebody else's claim. The host families still refuse passage above them.

-> audit_choice

=== audit_choice ===
// ghostlight.choice_layer: wrong_ledger_custody
+ [Keep the original in both hands and make Oren read each disputed leaf across the witness rail.]
    // ghostlight.action_label: hold_object
    // ghostlight.branch_label: retain_depot_custody
    ~ claim_copy = claim_copy + 1
    ~ office_pressure = office_pressure + 2
    ~ seal_scope = seal_scope + 1
    Ari turns each leaf without surrendering the spine.

    Oren reads the old seal, the unpaid tallies, and the later orders from the visitor side. He cannot pretend the pay leaves are absent. Ari cannot pretend the detour orders are.

    "You are obstructing recovery," Oren says.

    "I am furnishing it with a table."

    The original remains local. So does the accusation.
    -> custody_fold
+ [Pass the volume to Oren under a signed receipt naming every open claim.]
    // ghostlight.action_label: transfer_object
    // ghostlight.branch_label: yield_to_audit_receipt
    ~ ledger_custody = 2
    ~ claim_copy = claim_copy + 2
    ~ seal_scope = seal_scope + 2
    ~ office_pressure = office_pressure - 1
    ~ depot_trust = depot_trust - 1
    Ari makes Oren press his seal beneath four headings: guide pay, depot stores, later road orders, missing closing inventory. Before the seal cools, Ari copies the open unpaid tally onto an annex tablet and Tera closes one witness cord around it.

    Then Ari passes him the book.

    Tera's face closes by one careful degree. A receipt is evidence that the original left. It is not the names inside it.

    Oren wraps the volume in his oiled cloth. The old road seal disappears under the new audit seal, a small administrative eclipse.
    -> custody_fold
+ [Set the book between Tera and Oren and require alternating readers with a joint custody thread.]
    // ghostlight.action_label: place_object
    // ghostlight.branch_label: establish_joint_custody
    ~ ledger_custody = 3
    ~ claim_copy = claim_copy + 1
    ~ depot_trust = depot_trust + 1
    ~ office_pressure = office_pressure + 1
    ~ train_exposure = train_exposure + 1
    Ari lays the open volume on the center seam of the table.

    Oren reads one office entry. Tera answers with the work tally beneath it. Ari threads the depot cord through both witness loops and leaves the knot unfinished until each accepts the wording.

    It is slower than seizure and less satisfying than refusal. This is why it has a chance of surviving both.
    -> custody_fold
+ [Close the ledger and pull the road-warning cord before discussing what the seal might mean.]
    // ghostlight.action_label: pull_signal
    // ghostlight.branch_label: warn_before_title
    ~ road_warning = road_warning + 2
    ~ train_exposure = train_exposure - 1
    ~ office_pressure = office_pressure + 2
    Ari closes the cover on empire and wages alike, steps to the lower arch, and pulls twice.

    Bronze answers down the fog path: hold below; upper cut closed.

    Oren says, "You have issued a road order while denying the road office."

    "I have maintained a host warning while denying you the convenience of synonyms."

    Tera almost smiles. Mara does not. Signals spend stores when they stop people under your roof.
    -> custody_fold

=== custody_fold ===
// ghostlight.fold: evidence_and_authority_dispute
The audit settles into no shape polite enough to call agreement.

{ledger_custody == 1: The waxed volume remains in Ari's hands at the depot side of the table.}
{ledger_custody == 2: The volume sits wrapped beneath Oren's new audit seal; the old office seal is visible only in the receipt.}
{ledger_custody == 3: The volume lies open across the table seam, joined by an unfinished thread that names two readers and one witness.}

{seal_scope >= 2: Oren has copied enough later orders to argue that the portfolio survived withdrawal.}
{seal_scope == 1: The later orders are exposed but not yet attached to an accepted office chain.}
{claim_copy >= 2: Tera can point to a separate copy carrying names, loads, and payment withheld.}
{claim_copy < 2: The unpaid names remain concentrated in the original. Whoever leaves with it leaves carrying most of their voice.}
{office_pressure >= 3: Oren removes a narrow red office cord from his warrant case. Formal obstruction is now close enough to tie.}

The lower carrier bell sounds through the fog.

-> lower_bell

=== lower_bell ===
One strike means the porter train has entered the last ascent.

Then the upper wall answers with a quiet stone click.

{wall_evidence >= 2: Ari knows the sound belongs to the mortar seam between the shifted chalk beads.}
{wall_evidence < 2: Nobody in the court can yet say whether the click came from wet stone, a load above, or the old road enjoying the argument.}

{road_warning >= 3: A reply chime sounds below. The porters have received a hold signal, but the exposed lower shelf has little cover.}
{road_warning < 3: No reply comes. The train continues upward under the route sequence the claimant sent before the audit.}

Mara takes the broth off the hook. Tera gathers her witness cords. Oren lays one palm on the seal he controls, or wants to.

"The wall first," Mara says.

"The record decides who may order the wall first," Oren says.

"The wall has entered a dissenting opinion," Tera says.

-> final_choice

=== final_choice ===
// ghostlight.choice_layer: urgent_consequence
+ [Certify the wage leaves only, keep the upper cut closed, and spend depot stores on the waiting porters.]
    // ghostlight.action_label: write_and_signal
    // ghostlight.branch_label: prioritize_guide_claims
        {claim_copy >= 2 && depot_trust >= 3:
        Ari places the fresh copy with Tera's cords, records its witness chain, and pulls the hold signal. Mara opens the dry store under the names of the host families, not the vanished office.
        -> ending_claims_preserved
    - else:
        Ari tries to separate wages from title with too little copied evidence and too little consent around the table.
        -> ending_claims_cost
    }
+ [Maintain the closure, finish the joint thread, and send copies upward and sunward while the original stays shared.]
    // ghostlight.action_label: mixed
    // ghostlight.branch_label: prioritize_bounded_review
        {ledger_custody == 3 && road_warning >= 3:
        Ari tightens the joint thread only around custody, not title. Oren and Tera each receive a copy route. The hold signal remains in force while maintainers take weight off the wall.
        -> ending_bounded_review
    - else:
        Ari names a bounded review, but the custody and warning needed to make it real are missing.
        -> ending_review_cost
    }
+ [Let Oren take the original, but put the wall evidence and current closure on his receipt before any office seal moves the road.]
    // ghostlight.action_label: transfer_object
    // ghostlight.branch_label: prioritize_audit_chain
        {ledger_custody == 2 && wall_evidence >= 2 && claim_copy >= 2:
        Ari adds wet grit, shifted chalk beads, and the tested closure to the receipt. Oren can carry the original; he cannot honestly carry an open road in the same package.
        -> ending_audit_bounded
    - else:
        The audit chain closes around the book before the local evidence has enough separate body to resist it.
        -> ending_audit_capture
    }
+ [Bring the train into the depot court, keep the upper cut shut, and charge the claimant's bond for the hold.]
    // ghostlight.action_label: open_shelter
    // ghostlight.branch_label: prioritize_porter_shelter
        {shelter_capacity >= 3 && road_warning >= 3:
        Ari changes the signal from hold-below to depot-only entry. Porters climb under warning, unload into the marked dry lane, and stop before the upper stair.
        -> ending_shelter_hold
    - else:
        Ari calls the train into a court that cannot receive its full burden cleanly.
        -> ending_shelter_cost
    }

=== ending_claims_preserved ===
// ghostlight.ending_label: guide_claims_preserved
// ghostlight.training_hook: narrow_certification_preserves_unpaid_labor
The wage copy leaves the table with names still attached to loads and witnesses still attached to names.

The upper cut remains closed. Below, the porters receive broth, dry cloth, and a debt entry against the claimant who sent them climbing under an old route sequence.

Oren ties the red cord around his warrant instead of across the ledger. He will report obstruction. Tera will report custody. Mara will report exactly how much broth resistance the depot spent keeping strangers alive.

No title is settled. The unpaid work can now survive the original book leaving later.
-> END

=== ending_claims_cost ===
// ghostlight.ending_label: guide_claims_exposed
// ghostlight.training_hook: labor_priority_without_record_or_support
Ari certifies wages the room cannot yet separate from the old volume.

Oren challenges the copy. Tera refuses the book's removal. The hold signal reaches the train late, and depot stores open without an agreed account.

The people are sheltered by argument and fed on credit. The claim remains alive, but concentrated in the same object every faction now has reason to seize.
-> END

=== ending_bounded_review ===
// ghostlight.ending_label: shared_custody_road_closed
// ghostlight.training_hook: shared_record_custody_without_title_transfer
The joint thread closes around one promise: neither reader removes or alters the original before duplicate copies depart by separate routes.

The turned stone remains plateau-ward. The porters wait below while maintainers unload the retaining wall from the safe side. Oren's office receives the later detour orders. Tera's claimants receive the pay leaves. Host families receive the current closure record.

The archive has become slower and harder to capture. On this road, that counts as emergency work.
-> END

=== ending_review_cost ===
// ghostlight.ending_label: review_named_but_unowned
// ghostlight.training_hook: procedure_without_custody_or_warning
Ari names every correct boundary and owns too few of the acts that would make them hold.

Oren refuses the unfinished thread. Tera bars removal. The train climbs without a clear reply while the wall sheds another line of grit.

The proposed review survives in the guest tablet. The road does not owe the tablet obedience.
-> END

=== ending_audit_bounded ===
// ghostlight.ending_label: audit_chain_with_material_stop
// ghostlight.training_hook: audit_custody_cannot_swallow_local_hazard
Oren takes the original beneath two seals and a receipt crowded with inconvenient facts.

The copy at Tera's side preserves the unpaid names. The wet grit and shifted beads make the present closure part of the recovered chain. Oren can argue for the old portfolio before his superior; he cannot tell the waiting porters that the wall was open when he arrived.

The book goes sunward. The road stays shut. Their appeals begin in different directions.
-> END

=== ending_audit_capture ===
// ghostlight.ending_label: audit_capture_under_old_seal
// ghostlight.training_hook: clean_chain_erases_local_claims
The volume passes under Oren's new seal with too little left behind.

He reads the later orders as continuity and the current closure as obstruction. The claimant's route signal remains active. Porters reach the depot while Tera has a receipt where her names used to be.

The upper stair is barred by people, not yet by an office the Hegemony recognizes. The next contest will be over who can keep standing there when the wall, the weather, and the warrant all begin spending bodies.
-> END

=== ending_shelter_hold ===
// ghostlight.ending_label: porter_train_held_under_roof
// ghostlight.training_hook: shelter_without_passage_or_title
The reply chime changes. Depot entry only. Upper road closed.

Porters bring the first handcarts through the lower arch, unload them into Mara's marked dry lane, and stack the goods below the witness table. The retaining wall carries no new weight. The claimant's bond acquires food, storage, and delay costs in a hand everyone can inspect.

Oren still wants the book. Tera still wants wages. The host families still withhold passage. For one watch, the people those offices endangered have a roof.
-> END

=== ending_shelter_cost ===
// ghostlight.ending_label: shelter_overflow_in_fog
// ghostlight.training_hook: humane_intent_without_material_capacity
Ari calls the porter train into the depot and discovers that a roof also has a system boundary.

Handcarts jam the lower arch. Half the goods remain in cold fog. Mara spends stores faster than the guest tablet can assign them. Oren calls the congestion proof that local custody cannot manage the road; Tera calls it proof the claimant sent loads without consent.

Both arguments will travel. So will the damp, if anyone can get the carts turned around.
-> END
