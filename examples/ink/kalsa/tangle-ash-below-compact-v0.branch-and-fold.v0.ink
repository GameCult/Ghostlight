// ghostlight.artifact_id: kalsa_tangle_ash_below_compact_branch_fold_v0
// ghostlight.fixture_id: tangle-ash-below-compact-v0
// ghostlight.scene_id: tangle-ash-below-compact-v0.one-list-before-second-horn
// ghostlight.final_ink_path: examples/ink/kalsa/tangle-ash-below-compact-v0.branch-and-fold.v0.ink

VAR copy_chain = 2
VAR relief_reserve = 2
VAR compact_cohesion = 2
VAR branch_exposure = 1
VAR horn_pressure = 1
VAR taru_access = 1
VAR melka_custody = 0
VAR route_integrity = 2

-> start

=== start ===
The tributary bakehouse at Ninth Furnace begins its shift by arguing with cold water.

Heat still reaches the ovens. The return pipe beneath the washing trough does not, not for half the shift, because Ash Hook has diverted it above the East Collar gate. The dough bowls are warm. The rinse bucket has the moral temperament of mountain ice.

Ves works at the long scoring table between them. She is the bakehouse tally keeper: every loaf receives a household mark, every missing heat share receives a line, and every official who calls this clerical work eventually discovers that people remember who ate.

-> ordinary_roster

=== ordinary_roster ===
Enka, steward of the Ninth Furnace hostel store, stacks flat ration loaves into reed trays. Her ring of three store keys hangs outside her apron, visible enough to prevent anyone pretending the store opened itself.

Salen waits by the burial-meal shelf in a patched gray carrier cloak. Release workers prefer brief rites and settled obligations. Salen prefers punctual deliveries, which is nearly the same religion before breakfast.

Taru warms both hands over the oven mouth. He is the public recognizer for a defeated champion whose shrine fell silent after the first Ash-Halo armament. Today he means to carry that old sign into the new armament hearing.

Beyond the slatted court door, a steep stair rises toward the reopened furnace court. Melka, an independent death witness, waits on the upper landing with an empty document case. She serves the evidence, not the compact.

The compact itself is smaller than the rumors: Taru, Enka, Ves, and Salen. Four lawful offices coordinated in a way Ninth Furnace finds impolite.

-> compact_routine

=== compact_routine ===
On the scoring table lie three things that are never supposed to look related.

A copy of the answer roll names familiar gestures and voices recorded near the Ash-Halo route. A withdrawal tally names the hostel and furnace-family shrine whose ordinary protections are funding the champion's divine armour. A tray list names which households receive bread if those protections fail.

The office list may be disclosed. The route list may not. No one person keeps the whole path by which copies, witnesses, and food can leave Ninth Furnace.

The first horn will open the court stores. Before it sounds, Ves can strengthen one part of the compact.

-> preparation_choice

=== preparation_choice ===
// ghostlight.choice_layer: compact_preparation
+ [Split the answer-roll copy and score each leaf with a different delivery mark.]
    // ghostlight.action_label: use_object
    // ghostlight.branch_label: prepare_split_copy
    ~ copy_chain = copy_chain + 2
    ~ route_integrity = route_integrity + 1
    ~ branch_exposure = branch_exposure + 1
    Ves lays the thin copied leaves beneath the bread scorer and presses a different household notch into each clay tie.

    One leaf can travel in a hostel tray. One can enter a burial-meal basket. One stays beneath the bakehouse day's ordinary account.

    "Three copies," Enka says. "Now we can lose two with dignity."

    "Dignity costs extra," Ves says, and marks it nowhere.
    -> preparation_fold
+ [Move the compact's reserve into public bread trays before the branch can call it sect property.]
    // ghostlight.action_label: move_object
    // ghostlight.branch_label: prepare_relief_loaves
    ~ relief_reserve = relief_reserve + 2
    ~ compact_cohesion = compact_cohesion + 1
    ~ horn_pressure = horn_pressure + 1
    Enka opens the first and second store locks. Ves counts out bread and lamp oil against named hostel rooms and shrine households.

    The reserve becomes a row of ordinary trays with ordinary marks. Seizing it now would mean taking meals from people the branch already lists as protected.

    The work costs time. Good safeguards have a vulgar appetite for the clock.
    -> preparation_fold
+ [Read the withdrawal tally aloud with Taru and make every household answer its own mark.]
    // ghostlight.action_label: speak
    // ghostlight.branch_label: prepare_public_petition
    ~ taru_access = taru_access + 1
    ~ compact_cohesion = compact_cohesion + 1
    ~ branch_exposure = branch_exposure + 1
    Ves names the hostel rooms and shrine families one by one. Taru answers for the old champion's following. Enka answers for the store. Two bakers answer for households still at the ovens.

    The list becomes harder to dismiss and easier to punish.

    Salen taps the table. "Nothing unites a room like giving an official the correct spelling of everyone in it."
    -> preparation_fold
+ [Send Salen through the burial-meal route with no copied leaf, only the next fallback mark.]
    // ghostlight.action_label: transfer_object
    // ghostlight.branch_label: prepare_quiet_route
    ~ route_integrity = route_integrity + 2
    ~ copy_chain = copy_chain + 1
    ~ compact_cohesion = compact_cohesion - 1
    Ves knots one clay-marked cord around the handle of an empty burial basket. It names a fallback shelf beyond Ninth Furnace and nothing about the dead.

    Salen lifts the basket. To anyone watching, a release carrier is collecting a meal that has not yet been cooked. This is suspicious only to people with experience of breakfast.

    Taru watches Salen leave and dislikes being trusted with less than the route.
    -> preparation_fold

=== preparation_fold ===
// ghostlight.fold: ordinary_bakehouse_conspiracy
The bakehouse completes its first rack. Loaves pass from oven peel to scoring table to reed trays. The compact works inside that traffic because the traffic has to continue whether Ninth Furnace approves of politics or not.

{copy_chain >= 4: Three scored copy leaves now sit in different ordinary holdings. Losing one will leave a thinner case, not silence.}
{copy_chain <= 2: The answer-roll copy remains concentrated enough that one clean seizure could make the hearing depend on branch memory.}
{relief_reserve >= 4: Bread and lamp oil stand openly under household marks, already harder to rename as a private sect hoard.}
{compact_cohesion >= 3: Taru, Enka, and Salen move around Ves's table with the irritating efficiency of people who have already had this argument in private.}
{compact_cohesion <= 1: The compact is still functioning, which is not the same as trusting itself.}
{route_integrity >= 4: Salen's fallback cord has left the room; the route can survive one closed door.}
{branch_exposure >= 3: Too many people have answered their marks for the branch to believe this is an accidental queue.}

-> first_horn

=== first_horn ===
The first horn sounds above the court.

Then the slatted door opens from the stair.

-> registrar_arrival

=== registrar_arrival ===
Nema descends in a dark branch wrap with a bronze registrar's case under one arm. She records for Ninth Furnace and interprets for it, in that order when watched.

She places a torn route strip on Ves's table. It bears Salen's carrier notch and the mark for the hostel shelf. A substitute carrier was stopped at the court threshold.

"Holder Iresa will admit one petition before the second horn," Nema says. "Give me the compact's membership and one bearer responsible for every copy. Unlisted carriers will be barred. Hostel relief issued through this conspiracy will be charged to sect stores."

Enka puts one floury hand over her visible keys. Taru looks toward the court stair.

{route_integrity >= 4: Salen is beyond the room on a route Nema may or may not know.}
{route_integrity < 4: Salen has moved behind the burial shelf, outside Nema's direct sightline but still inside the bakehouse.}

Ves owns no miracle, champion, branch, or mortuary verdict. She does own the day tally Nema needs if the branch means to call bread a sect expense.

-> exposure_choice

=== exposure_choice ===
// ghostlight.choice_layer: registrar_exposure
+ [Give Nema the lawful office list and refuse to invent one owner for the copies.]
    // ghostlight.action_label: speak
    // ghostlight.branch_label: disclose_office_list
    ~ branch_exposure = branch_exposure + 2
    ~ taru_access = taru_access + 1
    ~ compact_cohesion = compact_cohesion - 1
    Ves names Taru the recognizer, Enka the store steward, herself the tally keeper, and Salen the carrier. She names the boundary of each task.

    "That is coordination," Nema says.

    "Yes. You found all four members of four offices."

    Nema records the names. Taru can now demand treatment as a listed petitioner. Salen's cutout route has become much less cut out.
    -> exposure_fold
+ [Attach the branch's own household marks to every relief tray.]
    // ghostlight.action_label: use_object
    // ghostlight.branch_label: attach_ration_claim
    ~ relief_reserve = relief_reserve + 1
    ~ compact_cohesion = compact_cohesion + 1
    ~ horn_pressure = horn_pressure + 1
    Ves turns the day tally around. Each tray already carries a household the branch recognizes as protected and a withdrawal the potential steward entered.

    Nema can call the reserve sect property. She cannot do it neatly while using the same marks to count Ninth Furnace's dependants.

    She begins copying. The second horn does not care about clerical embarrassment.
    -> exposure_fold
+ [Pass a scored copy leaf into the burial basket and send it along Salen's mortuary route to Melka.]
    // ghostlight.action_label: transfer_object
    // ghostlight.branch_label: send_copy_with_salen
    ~ melka_custody = melka_custody + 2
    ~ copy_chain = copy_chain + 1
    ~ route_integrity = route_integrity - 1
    ~ horn_pressure = horn_pressure + 1
    Ves slides one leaf beneath the basket cloth and gives it to the youngest bakehouse porter with an ordinary meal receipt.

    The porter knows Melka's empty case and one return shelf. Nothing else.

    Nema sees the basket go. She does not stop a burial meal in front of six household witnesses. She records the departure instead, which may be more dangerous later.
    -> exposure_fold
+ [Refuse the names, keep scoring bread, and make Nema decide whether to seize the working table.]
    // ghostlight.action_label: wait
    // ghostlight.branch_label: refuse_names_keep_working
    ~ route_integrity = route_integrity + 1
    ~ compact_cohesion = compact_cohesion + 1
    ~ branch_exposure = branch_exposure + 1
    ~ taru_access = taru_access - 1
    ~ horn_pressure = horn_pressure + 1
    Ves scores the next loaf. Enka fills the next tray. Taru says nothing.

    Nema can seize the table, but then the branch loses the tally that separates hostel meals, shrine relief, and bakehouse dues. She can wait, but waiting spends the horn.

    She chooses a third tool. Taru's name disappears from the single admitted petition token.
    -> exposure_fold

=== exposure_fold ===
// ghostlight.fold: exposed_compact_keeps_working
Nema opens her bronze case and sets one fired-clay passage token beside the torn route strip.

"One petition," she says. "One bearer. Before the second horn."

{branch_exposure >= 4: Her case already holds enough names and marks to prosecute coordination. It also holds enough to prove the branch knew which households would lose protection.}
{branch_exposure <= 2: Nema knows the compact exists but cannot yet show how the lawful offices join.}
{taru_access >= 2: Taru's name remains legible as a recognized claimant on the passage token.}
{taru_access <= 0: The token excludes Taru. He can reach the threshold only as somebody else's witness.}
{melka_custody >= 2: Above the slats, a case latch closes once: Melka has received a copy under mortuary custody.}
{melka_custody == 0: Melka's case remains empty on the upper landing.}
{horn_pressure >= 3: Furnace draft changes above. The court racks are opening while the bakehouse argues over the doorway.}

The passage token can carry one person through the slatted door. The compact's leverage depends on what reaches the threshold even when the person carrying it does not.

-> threshold_choice

=== threshold_choice ===
// ghostlight.choice_layer: one_threshold_token
+ [Give the passage token to Taru for the recognized-share petition.]
    // ghostlight.action_label: transfer_object
    // ghostlight.branch_label: file_taru_share
    ~ taru_access = taru_access + 2
    ~ branch_exposure = branch_exposure + 1
    ~ horn_pressure = horn_pressure + 1
    Taru closes his hand around the clay token.

    He carries no claim that the old champion is present. He carries the recorded sign, the silent shrine, and a request for warning or withdrawal if that sign enters the route.

    Nema lets him climb because refusal would now have to be written beside a recognized petitioner.
    -> final_threshold
+ [Give the token to the porter and place the scored copy directly in Melka's case.]
    // ghostlight.action_label: transfer_object
    // ghostlight.branch_label: place_copy_with_melka
    ~ melka_custody = melka_custody + 2
    ~ copy_chain = copy_chain + 1
    ~ horn_pressure = horn_pressure + 1
    The porter climbs with the basket. Melka takes the scored leaf, checks Ves's mark against the meal receipt, and closes it inside her independent case.

    Taru remains below. The dead have gained no advocate inside the rite, but the branch has lost sole custody of what may be seen there.
    -> final_threshold
+ [Set the token in the first relief tray and open the household distribution at the door.]
    // ghostlight.action_label: move_object
    // ghostlight.branch_label: open_relief_table
    ~ relief_reserve = relief_reserve + 1
    ~ compact_cohesion = compact_cohesion + 1
    ~ branch_exposure = branch_exposure + 1
    Enka moves the first marked tray against the slatted door. Ves sets the passage token on top.

    Hostel families and shrine households queue in the bakehouse, not as a demonstration but because this is when the bread is warm.

    Nema now has one admitted object and too many lawful beneficiaries. The court stair remains closed to the petition while the branch's welfare bargain becomes visible below it.
    -> final_threshold
+ [Hand Nema the route list fragment and demand a written bar for every named carrier.]
    // ghostlight.action_label: show_object
    // ghostlight.branch_label: disclose_route_list
    ~ branch_exposure = branch_exposure + 3
    ~ route_integrity = route_integrity - 2
    ~ taru_access = taru_access + 1
    ~ compact_cohesion = compact_cohesion - 1
    Ves places her matching route strip beside Nema's torn piece. Together they name the hostel shelf, burial basket, bakehouse porter, and outer mortuary handoff.

    "Bar them," Ves says. "One by one. In your own record."

    Nema can expose the route. She cannot make each baker, mourner, hosteller, and witness cease being necessary to Ninth Furnace. Salen will hate this argument if Salen remains free to hear it.
    -> final_threshold

=== final_threshold ===
// ghostlight.fold: claims_before_second_horn
The second horn has not sounded. It is close enough that flour trembles on the scoring table when workers test the court weights above.

{copy_chain >= 4: The answer-roll evidence exists in several scored leaves with custody marks that cannot all be seized through one door.}
{copy_chain <= 2: One remaining copy still carries too much of the compact's case.}
{relief_reserve >= 4: Marked bread and lamp oil can keep the withdrawn households through the first failed promise.}
{relief_reserve <= 2: The reserve will force the compact to choose which households can afford principle.}
{compact_cohesion >= 4: Enka, Taru, and the bakehouse workers are still acting as one practical constituency despite the exposed names.}
{compact_cohesion <= 1: The compact has protected its machinery by making several members wonder whether they were expendable parts.}
{branch_exposure >= 5: Nema can name the conspiracy. She can also be made to name every ordinary office the branch would have to break to end it.}
{route_integrity >= 4: A fallback path remains beyond the slatted door and the branch's present list.}
{route_integrity <= 1: The route has become a set of names inside Nema's bronze case.}
{taru_access >= 3: Taru stands where the recognized-share claim must receive an answer before the route opens.}
{melka_custody >= 2: Melka holds a scored answer-roll leaf outside the registrar's sole custody.}
{horn_pressure >= 4: The next weight test will become the second horn unless somebody spends the remaining interval.}
{horn_pressure <= 2: The compact still has a little time, the rarest item in any relief store.}

Ves cannot make the dead speak or the God listen. She can choose what the compact preserves when the branch opens the divine route.

-> final_choice

=== final_choice ===
// ghostlight.choice_layer: compact_priority
+ [Complete the three-claim chain: share petition, household withdrawal, and mortuary copy.]
    // ghostlight.action_label: speak
    // ghostlight.branch_label: prioritize_three_claims
    {taru_access >= 3 && melka_custody >= 2 && copy_chain >= 4:
        Ves reads the withdrawal tally through the slats while Taru and Melka answer from their separate places.
        -> ending_three_claims_success
    - else:
        Ves calls all three claims, but one of the required owners is missing or one copy bears too much of the case.
        -> ending_three_claims_cost
    }
+ [Spend the compact's strength on bread, lamps, and departure for the withdrawn households.]
    // ghostlight.action_label: move_object
    // ghostlight.branch_label: prioritize_household_floor
    {relief_reserve >= 4 && compact_cohesion >= 3:
        Ves turns the scoring table from evidence desk to distribution line.
        -> ending_household_floor_success
    - else:
        Ves opens the reserve before it is large or trusted enough to carry everyone named.
        -> ending_household_floor_cost
    }
+ [Make Nema record the whole exposed compact as lawful offices acting together.]
    // ghostlight.action_label: show_object
    // ghostlight.branch_label: prioritize_exposed_leverage
    {branch_exposure >= 5 && compact_cohesion >= 2:
        Ves puts the office list, household tally, and passage receipt into Nema's line of sight.
        -> ending_exposed_leverage_success
    - else:
        Ves chooses exposure before the compact has enough names or solidarity to survive the branch's answer.
        -> ending_exposed_leverage_cost
    }
+ [Preserve the outer copy route and let the present hearing fail cleanly.]
    // ghostlight.action_label: withhold_object
    // ghostlight.branch_label: prioritize_outer_copy
    {route_integrity >= 3 && copy_chain >= 3:
        Ves withholds the last route mark from Nema and sends the bakehouse's ordinary traffic onward.
        -> ending_outer_copy_success
    - else:
        Ves protects a route that has already narrowed into the registrar's case.
        -> ending_outer_copy_cost
    }

=== ending_three_claims_success ===
// ghostlight.ending_label: three_claims_success
// ghostlight.training_hook: divided_offices_force_recorded_divine_dispute
Taru files the recognized-share petition. Ves places the household withdrawals on the potential steward's allocation line. Melka seals the scored answer-roll copy under mortuary custody.

The claims do not become one authority. That is why they work.

Nema records that Ninth Furnace knew the shrines, dependants, and familiar signs before the route opened. The Iron Shelter's intermediary may still reject the share. The forge custodian may still stop the machinery for reasons none of these records owns. The dead may remain silent.

{horn_pressure >= 4: The second horn sounds while the last receipt is still warm from Ves's hand.}
{horn_pressure < 4: The court holds the second horn long enough to copy the refusal beside all three claims.}

The compact wins no gate. It makes the next denial survive the person who speaks it.
-> END

=== ending_three_claims_cost ===
// ghostlight.ending_label: three_claims_cost
// ghostlight.training_hook: coordinated_claim_fails_when_one_custody_path_is_missing
Ves calls three offices toward a threshold built to admit one.

{taru_access < 3: Taru remains below the slats, a recognizer with no admitted voice.}
{melka_custody < 2: Melka's case closes on empty air.}
{copy_chain < 4: The branch can still make one seized leaf stand for the whole record.}

Nema enters the claim as incomplete. The divine route opens under a cleaner docket than the room deserves.

The compact learns the old lesson in its local dialect: coordination is not the same as custody. Somebody must still be standing in every office when the horn sounds.
-> END

=== ending_household_floor_success ===
// ghostlight.ending_label: household_floor_success
// ghostlight.training_hook: constituency_survives_failed_hearing
Enka opens the third store lock. Ves reads household marks. Bakers pass warm loaves and lamp oil across the scoring table while Taru carries the names of anyone absent.

The petition may miss the second horn. The branch can expose every compact member and still find its hostel fed by the people it meant to discipline.

{melka_custody >= 2: Melka keeps the mortuary copy for the later hearing.}
{melka_custody < 2: The later hearing will begin from household copies and whatever signs witnesses can still recognize.}

When the court weights fall above them, no one calls the bread a miracle. This is partly modesty and partly accurate accounting.
-> END

=== ending_household_floor_cost ===
// ghostlight.ending_label: household_floor_cost
// ghostlight.training_hook: mutual_aid_cannot_cover_every_withdrawn_claim
Ves opens the relief trays and discovers that a protected minimum becomes arithmetic as soon as the door is barred.

There is bread for the hostel rooms or lamp oil for the shrine families through the cold interval. Not both. {compact_cohesion <= 1: The compact's earlier secrecy returns as suspicion over who was counted first.}{compact_cohesion > 1: Enka makes the cut in public and writes who is still owed.}

The second horn sounds above a distribution line that cannot defeat shortage by being righteous about it.

The compact survives. One constituency now knows exactly what survival cost it.
-> END

=== ending_exposed_leverage_success ===
// ghostlight.ending_label: exposed_leverage_success
// ghostlight.training_hook: exposure_cannot_erase_dependency
Ves gives Nema the office list because Nema already has the names. Then she adds the day's bread tally, the hostel key receipt, Taru's claimant mark, Salen's carrier receipt, and Melka's independent custody line.

The conspiracy fits inside Nema's bronze case. The necessary work does not.

To break the compact cleanly, Ninth Furnace would have to dismiss its recognizer, replace its hostel store, discredit its bakehouse account, and conduct a later succession without the mortuary witness it punished today. Nema sees the shape of the problem. She records the coordination as a branch offence and the offices as continuing duties.

Exposure gives the branch targets. It also gives every later court a membership list with consequences attached.
-> END

=== ending_exposed_leverage_cost ===
// ghostlight.ending_label: exposed_leverage_cost
// ghostlight.training_hook: disclosure_without_solidarity_becomes_targeting
Ves makes the compact visible before its members have agreed what they will lose together.

Nema records four names and assigns four separate breaches. Taru keeps limited claimant standing. Enka keeps the hostel keys under a branch watcher. Ves keeps the tally table and loses authority to send its copies. Salen's route becomes a search order.

{route_integrity <= 1: The route list closes inside the registrar's case like a trap built from accurate directions.}
{route_integrity > 1: One fallback remains outside Nema's copy, but nobody in the bakehouse can be certain who still holds it.}

The compact survives as dependants and unfinished duties. Its members survive as easier arrests.
-> END

=== ending_outer_copy_success ===
// ghostlight.ending_label: outer_copy_success
// ghostlight.training_hook: fallback_preserves_future_appeal_at_present_cost
Ves stops building the present case around the single passage token. Whatever it already admitted—Taru, a copy, a relief tray, or only a written bar—remains one partial claim.

An ordinary bread tray leaves through the hostel door. A burial basket leaves by the lower stair. Somewhere beyond Ninth Furnace, one scored leaf and one withdrawal tally will meet a mortuary office the branch does not presently control.

{taru_access >= 3: Taru stays below the court despite having a recognized route in. He gives the hearing up so the copy cannot be traced through him.}
{taru_access < 3: Taru had no clean entry left; preserving the copy makes that defeat useful later rather than less real now.}

The second horn sounds. The present rite proceeds without the compact's full claim. The outer copy cannot protect anyone in the furnace court tonight. It can prevent tonight from becoming the only history anyone is allowed to inherit.
-> END

=== ending_outer_copy_cost ===
// ghostlight.ending_label: outer_copy_cost
// ghostlight.training_hook: fallback_fails_after_route_capture
Ves protects the idea of an outer route after Nema has collected most of its pieces.

The bread tray is stopped at the hostel door. The burial basket reaches the lower stair and finds a branch watcher already waiting. {copy_chain <= 2: The seized leaf leaves the bakehouse with no independent answer-roll copy.}{copy_chain > 2: Another leaf remains, but its next custody mark points back into Ninth Furnace.}

Above, the second horn opens the divine grant. Below, Nema's bronze case acquires a route map.

Fallback is a material practice. Today the compact preserved the word and lost the road.
-> END
