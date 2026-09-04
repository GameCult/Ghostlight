// ghostlight.artifact_id: kalsa_tangle_disclosure_laundry_branch_fold_v0
// ghostlight.fixture_id: tangle-disclosure-laundry-v0
// ghostlight.scene_id: tangle-disclosure-laundry-v0.lower-shadow-copy-room
// ghostlight.final_ink_path: examples/ink/kalsa/tangle-disclosure-laundry-v0.branch-and-fold.v0.ink

VAR link_evidence = 1
VAR talen_trust = 2
VAR cell_pressure = 1
VAR ward_exposure = 1
VAR independent_copy = 0
VAR load_control = 1
VAR cutout_testimony = 0
VAR personal_cover = 1

-> start

=== start ===
The civic copy room sits under a Sunwall cargo gate, where the city keeps paper because grain is too heavy to argue with and empires are not.

One stair climbs to the ward court. One descends to the lift gallery. A barred service door opens onto the Terjamna-protected lane. Between them stand three slanted desks, a wall of clay document tubes, and a waist-high copy chest with two locks held by offices that dislike one another professionally.

Nara Ves sharpens reed pens before first bell. She is a civic copyist: not a prophet, not a judge, and therefore expected to notice everything without developing ambitions about it.

-> routine_introductions

=== routine_introductions ===
Talen Or comes up the lift stair smelling of brake dust and bitter tea. He keeps load marks for the Sunwall crew below.

"The lift is sound for six carts," he says. "Which means someone will shortly order nine and call the other three confidence."

Nara puts his cup on the one corner of her desk not governed by ink. This is their ordinary morning: his measured capacities, her copied orders, and a small conspiracy to keep tea outside the official record.

Beyond the barred door, Nara's sister folds a breakfast stall before taking the protected lane home. Nara can see one corner of the dark wool awning through the bars.

-> three_leaves

=== three_leaves ===

Three leaves wait in separate trays. A house forecast advises a one-bell pause if tomorrow's lower-shadow grain load draws a crowd. A shrine provisioner requests four empty carts at that same bell. A Terjamna garrison receipt records guards assigned to the receiving gallery under standing protection authority.

Each leaf is proper. Together they have begun to look employed.

-> preparation_choice

=== preparation_choice ===
// ghostlight.choice_layer: morning_copy_routine
+ [Lay the three leaves side by side and compare their bell interval, beneficiaries, and cord fibers.]
    // ghostlight.action_label: inspect_objects
    // ghostlight.branch_label: join_three_leaves
    ~ link_evidence = link_evidence + 2
    ~ cell_pressure = cell_pressure + 1
    The forecast names a pause. The cart request names the empty vehicles needed during it. The guard receipt names the force that could call the paused grain endangered and seize it for protection.

    One red fiber crosses all three tying cords. Cheap cord, common dye, no miracle. It is still more relationship than any leaf admits.
    -> routine_fold
+ [Ask Talen to mark the six-cart limit and refuge capacity on an independent operating leaf.]
    // ghostlight.action_label: speak
    // ghostlight.branch_label: establish_material_limit
    ~ talen_trust = talen_trust + 1
    ~ load_control = load_control + 2
    Talen stops joking. He draws six cart boxes, the brake interval, and the refuge that can hold two crews but not a garrison.

    "This is not an opinion," he says.

    "That is why everyone will call it one," Nara says, and seals his leaf as Sunwall work rather than forecast evidence.
    -> routine_fold
+ [Copy the garrison receipt into the ward packet before its courier returns.]
    // ghostlight.action_label: copy_object
    // ghostlight.branch_label: seed_independent_copy
    ~ independent_copy = independent_copy + 1
    ~ ward_exposure = ward_exposure + 1
    Nara copies the receipt by hand, including the smear where a quartermaster's thumb crossed wet ash ink. She slides it into the ward packet bound for the court stair.

    The ward will possess memory. It may also acquire guards.
    -> routine_fold
+ [File each leaf in its proper tube and keep the connection in memory.]
    // ghostlight.action_label: withhold_action
    // ghostlight.branch_label: preserve_personal_cover
    ~ personal_cover = personal_cover + 2
    ~ cell_pressure = cell_pressure - 1
    Nara files forecast, provision, and garrison receipt under separate owners. The room becomes tidy enough to lie under oath.

    She keeps the matching bell in memory, which is an archive with only one vulnerable shelf.
    -> routine_fold

=== routine_fold ===
// ghostlight.fold: ordinary_copy_work_before_pressure
First bell travels down the stone throat of the gate.

Talen returns to the lift stair. Nara sands the house copy, checks the ward packet, and writes three receipts for two locks because civic life has always preferred surplus keys to shared trust.

{link_evidence >= 3: The three leaves remain aligned on her desk, their shared interval visible as a single mechanism.}
{link_evidence < 3: The link exists only in Nara's memory and the order in which the tubes stand open.}
{independent_copy >= 1: The ward packet is heavier by one dangerous page.}
{load_control >= 3: Talen's six-cart operating leaf waits beneath its own plain seal.}
{personal_cover >= 3: Every official tube looks undisturbed, and Nara's hands look innocent.}

-> messenger_arrival

=== messenger_arrival ===
Vek Iras enters through the barred service door with a shrine provisioner's sash, dust on his boots, and no offering for the shrine.

He is the cutout. Nara knows this not because cutouts glow, but because he asks for the forecast receipt before asking whether the forecast is finished.

Vek places a small bronze safe-conduct token on her desk. "For your sister's walk home through the protected lane," he says. "The lanes have become particular about paper."

Below, cart wheels grind against the gallery rails before the pause has been ordered.

-> cutout_choice

=== cutout_choice ===
// ghostlight.choice_layer: cutout_pressure
+ {link_evidence >= 3} [Name the three leaves and ask Vek which patron taught them to share one bell.]
    // ghostlight.action_label: speak
    // ghostlight.branch_label: confront_cutout
    ~ cutout_testimony = cutout_testimony + 2
    ~ cell_pressure = cell_pressure + 2
    Vek looks at the cords, not the text. That glance is an answer with poor legal posture.

    "A house asks a question. A shrine hires carts. Guards protect grain."

    "And you carry the grammar between them."

    He does not deny it. He names no plan-holder. A cutout survives by being the part of a sentence everyone can erase.
    -> pressure_fold
+ [Turn both keys and lock the copy chest while Vek's receipt hand is still extended.]
    // ghostlight.action_label: lock_object
    // ghostlight.branch_label: lock_independent_record
    ~ independent_copy = independent_copy + 1
    ~ cell_pressure = cell_pressure + 1
    ~ personal_cover = personal_cover - 1
    The civic key turns. The Sunwall key Talen left for the morning transfer turns after it. Bronze bolts meet inside oak.

    Vek smiles without warmth. "You have made a box important."

    "No," Nara says. "The people outside it did that."
    -> pressure_fold
+ [Send Talen down the lift stair with the warning: inspect the receiving gallery before any pause.]
    // ghostlight.action_label: move_ally
    // ghostlight.branch_label: warn_material_owner
    ~ load_control = load_control + 2
    ~ talen_trust = talen_trust - 1
    ~ ward_exposure = ward_exposure + 1
    Talen reads her face, takes no paper, and descends. Trust spent on a real errand looks less romantic than trust discussed over tea.

    A moment later the lower warning bell rings once: gallery inspection, no load movement.
    -> pressure_fold
+ [Take the safe-conduct token and issue Vek the separate forecast receipt he requested.]
    // ghostlight.action_label: exchange_object
    // ghostlight.branch_label: accept_safe_conduct
    ~ personal_cover = personal_cover + 2
    ~ ward_exposure = ward_exposure + 2
    ~ cell_pressure = cell_pressure - 1
    The bronze token is warm from Vek's palm. It might get Nara's sister through the lane. It might prove Nara was bought. Useful objects are often bilingual.

    Vek receives a clean receipt for one clean leaf and leaves the other two officially unrelated.
    -> pressure_fold

=== pressure_fold ===
// ghostlight.fold: exposed_join_and_surviving_leverage
Boots gather beyond the barred door. The cell has noticed resistance, or merely reached the hour it bought.

{cutout_testimony >= 2: Vek's careful non-denial sits in Nara's notes beside the shared red cord.}
{independent_copy >= 1: At least one copy now exists beyond the sponsor's direct custody.}
{load_control >= 3: The lower gallery has an operating limit the garrison cannot truthfully call a prophecy.}
{personal_cover >= 3: The safe-conduct token and perfect filing offer Nara a narrow private exit.}
{ward_exposure >= 3: People on the ward stair have begun asking why armed protection arrived before the forecasted delay.}
{cell_pressure >= 4: A mailed fist strikes the barred door. The polite interval is over.}

The house can dismiss a scribe. The shrine can deny Vek. The garrison can cancel one instruction and keep its guards. Exposure will not make those powers evaporate. It can still decide whether this load, this order, and this worker remain theirs to use.

-> final_threshold

=== final_threshold ===
The second bell begins.

Talen appears at the lift stair. Vek stands at the service door. Above, a ward runner calls for the packet. Beyond the bars, the garrison officer demands the forecast receipt and access to the receiving gallery.

Nara has one copying interval before every separate authority starts calling its own piece the whole city.

-> final_choice

=== final_choice ===
// ghostlight.choice_layer: disclosure_commitment
+ [Bind the three leaves as a joined exhibit and send it up the court stair.]
    // ghostlight.action_label: transfer_evidence
    // ghostlight.branch_label: prioritize_court_exposure
    {link_evidence >= 3 && independent_copy >= 1:
        -> ending_court_success
    - else:
        -> ending_court_cost
    }
+ [Give Talen the operating leaf and stage the grain in six-cart loads through the civic side.]
    // ghostlight.action_label: authorize_material_action
    // ghostlight.branch_label: prioritize_material_control
    {load_control >= 3 && talen_trust >= 2:
        -> ending_load_success
    - else:
        -> ending_load_cost
    }
+ [Offer Vek his safe exit in exchange for a signed activation chain.]
    // ghostlight.action_label: bargain
    // ghostlight.branch_label: prioritize_cutout_testimony
    {cutout_testimony >= 2:
        -> ending_testimony_success
    - else:
        -> ending_testimony_cost
    }
+ [Keep the leaves separate, take the safe-conduct, and get your sister through the protected lane.]
    // ghostlight.action_label: withhold_evidence
    // ghostlight.branch_label: prioritize_private_cover
    {personal_cover >= 3:
        -> ending_cover_success
    - else:
        -> ending_cover_cost
    }

=== ending_court_success ===
// ghostlight.ending_label: court_exposure_success
// ghostlight.training_hook: joined_records_make_relationship_admissible
The ward runner carries a bound packet upward: forecast, carts, guards, shared bell, shared cord, and the copy that proves the garrison moved first.

The mixed bench stays the seizure and orders the planned response heard before the forecast can support custody. The guards remain. The house contract remains. Nara's name enters a charge file beside people with better housing.

The load waits one bell, then moves under a narrower order. Exposure has not defeated the patrons. It has made them spend authority where witnesses can see it.
-> END

=== ending_court_cost ===
// ghostlight.ending_label: court_exposure_cost
// ghostlight.training_hook: incomplete_join_exposes_clerk_before_cell
Nara binds suspicion into an exhibit. The leaves share a bell, but no independent copy proves who moved first.

The court preserves the packet and refuses an immediate irreversible seizure. It also leaves the temporary guard claim in place. By dusk the house has suspended a scribe, the shrine has forgotten Vek, and an order officer wants to know why Nara joined records outside routine.

The conspiracy loses speed. Nara loses anonymity. Grain remains behind bars.
-> END

=== ending_load_success ===
// ghostlight.ending_label: material_control_success
// ghostlight.training_hook: material_owner_changes_hidden_plan
Talen takes six carts, not nine. He uses the civic side, keeps the pressure refuge clear, and records every guard who objects to a load they claimed only to protect.

The house forecast expires against a system its sponsor did not brief. The waiting empty carts become useless. The garrison can still claim tribute, but not this staged emergency.

Nara hears the lift descend in measured knocks. The plot survives as patronage and threat. Its lever does not.
-> END

=== ending_load_cost ===
// ghostlight.ending_label: material_control_cost
// ghostlight.training_hook: late_operational_refusal_carries_ward_cost
Talen tries to stage the load with an incomplete leaf and trust already spent elsewhere.

The Sunwall crew closes the gallery rather than accept an unsafe order. The garrison cannot seize moving grain because no grain moves. Lower-shadow ration clerks begin cutting shares by evening.

The plot fails to acquire the load. The ward pays for the refusal, which is how clean principles become dirty politics before supper.
-> END

=== ending_testimony_success ===
// ghostlight.ending_label: cutout_testimony_success
// ghostlight.training_hook: sacrificed_cutout_exposes_activation_chain
Vek signs the sequence: forecast pause, cart position, guard claim, emergency custody. Nara returns his safe-conduct and keeps a copy beyond his reach.

The shrine dismisses him before third bell. The garrison calls him a freelance liar. The house says no prophet saw the whole plan. All three statements may survive review.

Vek gets through the civic stair. The activation chain does too. The cutout burns; the offices that used him remain standing and newly careful.
-> END

=== ending_testimony_cost ===
// ghostlight.ending_label: cutout_testimony_cost
// ghostlight.training_hook: bargaining_without_leverage_strengthens_cutout
Nara offers Vek an exit he does not yet need.

He takes the safe-conduct, declines the ink, and tells the garrison officer that the copyist attempted to purchase false testimony. The charge is thin. Thin charges still occupy a clerk while carts move.

Vek leaves through the court stair. Nara stays with three true leaves and no witness to their grammar.
-> END

=== ending_cover_success ===
// ghostlight.ending_label: private_cover_success
// ghostlight.training_hook: constituency_leverage_survives_exposure
Nara files each leaf under its proper owner. She takes the bronze token and walks her sister through the protected lane before the guards close it.

The garrison receives emergency custody of the paused grain. The house disciplines nobody. The shrine pays Vek. Tomorrow Nara will still have access to the copy room, which means the cell has bought her silence and preserved the person who may expose its next assignment.

Survival is not consent. It is also not a verdict from outside the body that must survive.
-> END

=== ending_cover_cost ===
// ghostlight.ending_label: private_cover_cost
// ghostlight.training_hook: private_safety_offer_without_durable_cover
Nara keeps the leaves separate, but the lane guard does not honor a token whose issuing office has already denied Vek.

Her sister waits outside the bars. The grain goes under garrison seal. By nightfall the ward has both a shortage and a rumor that Nara was paid for it.

The laundry has discarded its cutout and kept the wash.
-> END
