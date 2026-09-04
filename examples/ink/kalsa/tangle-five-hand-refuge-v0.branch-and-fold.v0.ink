// ghostlight.artifact_id: kalsa_tangle_five_hand_refuge_branch_fold_v0
// ghostlight.fixture_id: tangle-five-hand-refuge-v0
// ghostlight.scene_id: tangle-five-hand-refuge-v0.ninth-furnace-hostel-before-first-horn
// ghostlight.final_ink_path: examples/ink/kalsa/tangle-five-hand-refuge-v0.branch-and-fold.v0.ink

VAR record_integrity = 2
VAR aid_released = 0
VAR route_ready = 1
VAR office_separation = 1
VAR ring_exposure = 0
VAR cutout_integrity = 1
VAR registrar_record = 0
VAR nema_pressure = 1

-> start

=== start ===
The Ninth Furnace hostel copy room has three exits and five people pretending that this is an innocent number.

The courtward door opens toward the furnace court where a branch champion will soon be fitted in divine iron. The hostel corridor runs west past sleeping rooms and the communal broth pots. A felt curtain on the north wall hides the ash-service stair, a narrow route used by workers who prefer soot to ceremony and generally receive both.

A low heat pipe warms the south bench. The middle of the room belongs to a scarred copy table, two oil lamps, an aid cupboard, and a sealed roll that nobody here is allowed to make less dangerous by calling it paperwork.

-> introduce_ring

=== introduce_ring ===
Veka Orr sits at the table with a pen, the ring's closed membership list, and the professional despair of a memory keeper who has been told to write legibly during sedition.

Melka, a mortuary witness, keeps the sealed answer roll beneath both palms. It lists familiar signs once seen near the under-gallery and the shrines whose protections are being withdrawn for today's armament. She can record a gesture or a silence. She cannot certify which dead person, Beast remnant, or hungry presence produced it.

Taru waits on the heated bench with his household's older witness copy tucked inside his work wrap. He recognized the defeated champion's sign before that shrine went silent. Today he means to ask for warning or withdrawal if the sign returns.

For this ring he also carries the sanctuary hand: he has named the outer Hearth shrine as the next forum if Ninth Furnace closes, while Kera keeps the route by which he could reach it.

Leth Aru counts two shifts of hostel food, a wool sleeping wrap, and witness pay onto the open cupboard shelf. She keeps the sect store. Once she enters aid against its beneficiary rule, a branch officer may challenge the release but cannot honestly call it an unmade promise.

Kera Oss stands by the felt curtain in an ash porter's leather apron and soot-gray headcloth. Her cart route reaches the furnace court below and an outer Hearth shrine beyond Ninth Furnace. A packet on her route receives one destination at a time. The route does not receive the whole plot.

Five hands: memory, witness, sanctuary claim, store, road.

-> ordinary_copy_work

=== ordinary_copy_work ===
The work begins with the ordinary disciplines that make a conspiracy expensive.

Veka reads Taru's household entry aloud. Melka corrects one date and refuses two confident interpretations. Leth checks the hostel rule against Taru's name. Kera knots a plain custody cord around an empty packet and leaves the destination tablet blank.

"We could save time by letting the branch write all of it," Taru says.

"We could save more time by letting the branch remember all of us dead," Veka says.

Leth slides the wool wrap onto the shelf. "The dead are terrible at signing for soup."

Beyond the courtward door, workers test the first horn. The note comes through stone as a low bronze complaint.

There is time for one preparation before the branch registrar arrives.

-> preparation_choice

=== preparation_choice ===
// ghostlight.choice_layer: five_hand_preparation
+ [Copy Taru's adverse margin in full, including the four incompatible readings of the old sign.]
    // ghostlight.action_label: write_record
    // ghostlight.branch_label: prepare_record_integrity
    ~ record_integrity = record_integrity + 2
    ~ ring_exposure = ring_exposure + 1
    Veka copies hunger, warning, petition, and ascent without selecting the useful one.

    The page becomes harder for Ninth Furnace to quote and easier for Ninth Furnace to prosecute. Every named contradiction points back toward the room that preserved it.

    Melka sands the wet ink. "Good. Now nobody can accuse us of knowing what happened."

    "A cherished professional standard," Veka says.
    -> preparation_fold
+ [Have Leth enter Taru's two-shift support in the open hostel account before any hearing.]
    // ghostlight.action_label: use_object
    // ghostlight.branch_label: prepare_released_aid
    ~ aid_released = aid_released + 2
    ~ office_separation = office_separation + 1
    Leth writes Taru's name beside food, a sleeping place, witness pay, and the wool wrap. Then she breaks the store seal on the first grain jar.

    "That one is spent," she says. "Iresa may demand review. She cannot demand that the jar become unbroken out of respect for rank."

    Taru looks at the food as if accepting it might be another oath.

    Leth notices. "Hostel support. Not submission. I have written both words where even a champion can trip over them."
    -> preparation_fold
+ [Walk the ash-service route with Kera and place the outside copy packet beyond the branch door.]
    // ghostlight.action_label: move
    // ghostlight.branch_label: prepare_route_fallback
    ~ route_ready = route_ready + 2
    ~ ring_exposure = ring_exposure + 1
    Veka follows Kera behind the felt curtain. The stair drops between warm masonry and a caged ash chute, turns at a landing, then divides: down to the furnace court, level through a porter door toward the outer shrine road.

    Kera places the packet in a soot-black wall niche beyond the hostel threshold and marks only the next hand.

    Boot grit on the landing shows that someone from the court has already used the stair today.
    -> preparation_fold
+ [Rehearse Taru's public petition and keep him outside the ring's membership list.]
    // ghostlight.action_label: speak
    // ghostlight.branch_label: prepare_public_cutout
    ~ cutout_integrity = cutout_integrity + 2
    ~ office_separation = office_separation + 1
    Veka makes Taru state only what he owns: the earlier household sign, the silence at the shrine, the warning or withdrawal he will request, and his refusal of a binding offered in the defeated champion's name.

    He does not learn who holds the outside list, which store line sustains the hearing, or where Kera takes a seized copy.

    "I dislike being protected by ignorance," Taru says.

    "It is not protection," Melka says. "It is a smaller target."
    -> preparation_fold

=== preparation_fold ===
// ghostlight.fold: five_hand_routine_complete
The five return to their stations around the table.

{record_integrity >= 3: Taru's page now preserves every adverse reading. It can survive a hostile quotation, provided the page itself survives.}
{record_integrity < 3: The household copy is accurate but thin. A registrar could detach Taru's claim from the uncertainty that limits it.}
{aid_released >= 2: One grain jar stands open beside the entered hostel support. Taru can lose the first hearing without losing tonight's meal.}
{aid_released == 0: Food and lodging remain available in principle, which is the most decorative form of refuge.}
{route_ready >= 3: The outside packet waits beyond the hostel threshold, and Kera has checked both turns of the ash stair.}
{route_ready < 3: The packet remains on the central table. Every fallback currently uses the same door as the accusation.}
{cutout_integrity >= 3: Taru carries a complete public claim and no map of the ring supporting it.}
{cutout_integrity < 3: Taru knows his grievance and will have to improvise the boundary between witness and conspiracy.}

The courtward latch lifts.

-> registrar_arrival

=== registrar_arrival ===
Nema, registrar of Ninth Furnace, enters with an iron-bound branch case against her hip. She is compact, middle-aged, and dressed in charcoal wool with a copper seal chain laid openly across her chest. Two furnace clerks wait beyond the door with empty hands and excellent hearing.

Nema sets a certified extract on the table. It names Iresa's armament order, Orsa's ritual authority, and the promise that withdrawn grants will be restored after victory.

"Court-related copies enter the branch case before the first horn," she says. "Refusals enter beside them. I have been asked to discover whether this room contains either."

Her gaze passes from Melka's sealed roll to Taru's work wrap, Leth's cupboard, Kera's curtain, and Veka's closed membership list.

She has not seen the list. She has seen five offices arranged like an answer.

-> registrar_choice

=== registrar_choice ===
// ghostlight.choice_layer: registrar_pressure
+ [Open the membership list and name all five offices before Nema can call the ring imaginary.]
    // ghostlight.action_label: show_object
    // ghostlight.branch_label: expose_membership_list
    ~ ring_exposure = ring_exposure + 3
    ~ office_separation = office_separation + 2
    ~ registrar_record = registrar_record + 2
    ~ nema_pressure = nema_pressure + 1
    Veka opens the list flat beneath both lamps.

    Memory keeper. Mortuary witness. Household sanctuary petitioner. Store steward. Route holder. Each name carries its declared conflict and the office record that can review it.

    Nema reads every line. "You have made discipline wonderfully convenient."

    "We feared discipline might strain itself looking," Veka says.

    Nema's clerk begins a second copy from the threshold.
    -> registrar_fold
+ [Offer Nema the working page under a signed receipt while Melka and Taru retain their separate evidence.]
    // ghostlight.action_label: transfer_object
    // ghostlight.branch_label: receipt_the_working_copy
    ~ record_integrity = record_integrity + 1
    ~ office_separation = office_separation + 2
    ~ registrar_record = registrar_record + 2
    ~ ring_exposure = ring_exposure + 1
    Veka slides the working page across the table and keeps one finger on it until Nema's receipt names the page, its adverse margin, and the fact that Taru's household copy and Melka's sealed roll are not inside the transfer.

    Nema signs. "A receipt is not approval."

    "No," Veka says. "It is what prevents your disapproval from eating the object."

    The page enters the branch case. Its limits enter with it.
    -> registrar_fold
+ [Send Taru forward with the public petition while the other four hands remain at their work.]
    // ghostlight.action_label: speak
    // ghostlight.branch_label: present_claimant_cutout
    ~ cutout_integrity = cutout_integrity + 2
    ~ registrar_record = registrar_record + 1
    ~ nema_pressure = nema_pressure + 1
    Taru steps between the table and the branch case.

    He names the defeated champion's earlier sign, the later silence, and the warning or withdrawal he will request if that sign appears in the court. He refuses to name a conspiracy because Veka never gave him one to recite.

    Nema records his petition and looks past him toward the people who made it possible.

    Taru is a public claimant, not an innocent man. The distinction is narrow enough to be useful.
    -> registrar_fold
+ [Signal Kera to take the outside packet down the ash stair before answering Nema.]
    // ghostlight.action_label: gesture
    // ghostlight.branch_label: move_external_fallback
    ~ route_ready = route_ready + 2
    ~ record_integrity = record_integrity + 1
    ~ ring_exposure = ring_exposure + 2
    ~ nema_pressure = nema_pressure + 2
    Veka touches two fingers to the soot mark at the page corner.

    Kera lifts the packet and disappears behind the felt curtain. The ash stair door closes one turn below.

    Nema hears it. Everyone does.

    "Should I record flight?" she asks.

    "Record porter traffic," Veka says. "Flight usually has cleaner boots."
    -> registrar_fold

=== registrar_fold ===
// ghostlight.fold: branch_case_meets_refuge_ring
Nema leaves the branch case open on the courtward edge of the table and plants herself in the narrow gap between table and east door. Together they constrict the straight courtward path without granting her custody of the room.

{ring_exposure >= 3: The ring is exposed enough that punishment can be assigned by name. Secrecy is no longer the useful resource.}
{ring_exposure < 3: Nema has evidence of coordination but no complete membership list. She must challenge acts and infer relations separately.}
{office_separation >= 4: Every contested act still has a different owner, record, and review path. Nema cannot seize one office and inherit the rest.}
{office_separation <= 2: The room looks less like divided authority than a memory keeper's private scheme with helpful furniture.}
{registrar_record >= 2: Nema's own copy now preserves either the five offices or the limits of the transferred page. Exposure has acquired branch custody.}
{registrar_record == 0: The registrar has heard claims but signed for nothing. Later certainty will belong to whoever writes first.}
{route_ready >= 3: Beyond the felt curtain, the external packet has a checked path toward an outer shrine.}
{nema_pressure >= 3: The two furnace clerks move into the doorway. Nema is still using record authority; the doorway is beginning to look like force wearing a neat chain.}

The first horn sounds from below.

-> first_horn

=== first_horn ===
The note climbs the ash chute, rattles the lamps, and opens the armament stores beneath them.

Melka closes both hands around the sealed answer roll. Taru stands with his household copy. Leth keeps one hand on the broken grain seal. Kera is {route_ready >= 4: already beyond the first turn of the stair}{route_ready < 4: still within call behind the curtain}. Nema waits beside a branch case that can preserve a record and imprison one.

The ring cannot keep every advantage. Veka must choose which leverage survives the walk to the court.

-> final_choice

=== final_choice ===
// ghostlight.choice_layer: refuge_ring_commitment
+ [Read the five names aloud and require Nema to challenge each office separately.]
    // ghostlight.action_label: speak
    // ghostlight.branch_label: prioritize_exposed_offices
    {office_separation >= 4 && registrar_record >= 2:
        Veka turns the membership list toward the threshold clerks and begins with her own name.
        -> ending_exposure_success
    - else:
        Veka names the ring before its separate offices are strong enough to carry the exposure.
        -> ending_exposure_cost
    }
+ [Divide the fallback: Melka keeps the roll, Taru keeps the household copy, Leth keeps aid moving, Kera carries the outside record.]
    // ghostlight.action_label: mixed
    // ghostlight.branch_label: prioritize_distributed_fallback
    {record_integrity >= 3 && aid_released >= 2 && route_ready >= 3:
        Veka assigns no new authority. She simply refuses to put the existing four holdings into one hand.
        -> ending_distribution_success
    - else:
        Veka divides the holdings before every piece has enough support or route to survive alone.
        -> ending_distribution_cost
    }
+ [Let Taru carry only the public grievance while the ring remains a support machine behind him.]
    // ghostlight.action_label: speak
    // ghostlight.branch_label: prioritize_public_cutout
    {cutout_integrity >= 3 && record_integrity >= 2:
        Veka gives Taru the courtward path and keeps the ring's other hands out of his speech.
        -> ending_cutout_success
    - else:
        Veka sends Taru forward with a true grievance and too little boundary around the people sustaining it.
        -> ending_cutout_cost
    }
+ [Withdraw Taru through the ash route to the outer shrine and spend the hostel aid on surviving the lost hearing.]
    // ghostlight.action_label: move
    // ghostlight.branch_label: prioritize_sanctuary_exit
    {aid_released >= 2 && route_ready >= 3:
        Veka folds the membership list, and Leth places the open grain jar in Taru's hands.
        -> ending_sanctuary_success
    - else:
        Veka chooses sanctuary with an unready route or support that still exists only on the shelf.
        -> ending_sanctuary_cost
    }

=== ending_exposure_success ===
// ghostlight.ending_label: exposed_offices_success
// ghostlight.training_hook: leverage_survives_disclosure
Veka reads five names and five offices into the branch record.

Nema marks the ring for review. Then she has to mark what review means. Veka may be challenged for copying. Melka remains custodian of the sealed roll. Leth's released aid requires a store hearing. Taru's petition must be answered as a household claim. Kera's route remains ordinary porter work until somebody orders the road closed.

The conspiracy is public. The leverage is annoyingly alive.

Taru passes the courtward threshold beside Melka. Behind them, Leth pushes the opened grain jar deeper onto the hostel shelf where a seizure will need witnesses.

Nema's victory is real: every hand can now be punished by name. So is the ring's: no punishment can pretend to be one simple act.
-> END

=== ending_exposure_cost ===
// ghostlight.ending_label: exposed_offices_cost
// ghostlight.training_hook: disclosure_without_separated_capacity
Veka reads the names. The room fails to supply five defensible acts behind them.

Nema seals the working table as one disputed archive. The store entry is still only a promise; the outside packet still uses the courtward door. Taru keeps his household copy and Melka keeps the roll, but both now stand inside a conspiracy whose material refuge has not begun.

Exposure has not made the claim false. It has made hunger and custody arrive before the hearing.
-> END

=== ending_distribution_success ===
// ghostlight.ending_label: distributed_fallback_success
// ghostlight.training_hook: divided_authority_as_conspiracy_resilience
Melka carries the sealed answer roll toward the court. Taru carries the household copy and the right to recognize only what he has recognized before. Leth's open account carries food, lodging, and witness pay through the first retaliation. Kera's packet takes the soot stair toward an outer shrine.

Nema can seize a page, stop a person at the threshold, or challenge the store release. She cannot perform all three acts with the same branch case.

Veka remains at the table with the membership list. It is the most dangerous object in the room and, for once, not the only useful one.
-> END

=== ending_distribution_cost ===
// ghostlight.ending_label: distributed_fallback_cost
// ghostlight.training_hook: fallback_fails_when_support_is_decorative
The pieces divide before they can stand.

Melka keeps the original sealed. Taru reaches the threshold with his household claim.

{record_integrity < 3: The adverse readings remain too thin to resist a hostile summary.}
{record_integrity >= 3: The record is strong, but ink cannot supply the missing refuge around it.}
{aid_released < 2: Leth has food on the shelf and no completed interval carrying it through retaliation.}
{aid_released >= 2: Leth's support is real, but it cannot replace the weak copy or blocked route.}
{route_ready < 3: Kera's packet still depends on the same threshold as Taru's claim.}
{route_ready >= 3: Kera has an outside path, but one of the holdings she must carry cannot yet stand when it arrives.}

Nema does not need to destroy the evidence. She needs only to make each holder wait for the next one.

The first horn fades while five hands discover that distance is not the same thing as independence.
-> END

=== ending_cutout_success ===
// ghostlight.ending_label: public_cutout_success
// ghostlight.training_hook: cutout_carries_act_not_hidden_authority
Taru walks to the threshold and states the household claim exactly once.

He names the old sign, the later silence, and the warning or withdrawal he will request if the sign enters the furnace court. He cannot betray the store route or outside copy because he does not know them. He cannot be dismissed as an invented spokesman because the household record is his own.

Nema writes his refusal into the Ninth Furnace copy. Behind him, the ring remains what it was meant to be: food, witness, memory, and a road supporting a claim without climbing into its mouth.

The cutout is exposed. The hands behind the act are not erased by his courage or owned by it.
-> END

=== ending_cutout_cost ===
// ghostlight.ending_label: public_cutout_cost
// ghostlight.training_hook: claimant_bears_hidden_network_cost
Taru speaks truly and alone.

His household copy carries the old sign. It does not carry the ring's boundaries clearly enough to stop Nema from asking which hidden officer instructed every word. The registrar records a claimant whose support looks like an undisclosed patron.

The ring stays out of the branch case by letting Taru absorb its shape. That is secrecy, but it is not refuge.
-> END

=== ending_sanctuary_success ===
// ghostlight.ending_label: sanctuary_exit_success
// ghostlight.training_hook: lost_hearing_preserved_claim
Taru takes the open grain jar, the sleeping wrap, and his household copy through the felt curtain. Kera leads him level at the landing toward the outer shrine road.

He will miss the first petition at the furnace court. The Iron Shelter may open its route without hearing his warning. Melka still holds the answer roll, and Nema still records the absence in Ninth Furnace's favor.

But the claimant, copy, and first two shifts of survival leave together. If the defeated champion's sign appears, another shrine will possess a witness the branch did not feed into silence.

The ring loses the hearing and preserves the argument.
-> END

=== ending_sanctuary_cost ===
// ghostlight.ending_label: sanctuary_exit_cost
// ghostlight.training_hook: route_without_material_refuge
Veka sends Taru behind the felt curtain.

The ash route is uncertain, or the aid has not yet become his. At the first landing he waits while Kera checks the porter door and Leth argues over a store line still unbroken. Above them, Nema closes the courtward case. Below them, the furnace court proceeds toward its second horn.

Leaving the hearing may still keep Taru from immediate custody. It does not yet give him a road, a meal, or an outside forum. A fallback written in future tense is another promise owned by the people least exposed to its failure.
-> END
