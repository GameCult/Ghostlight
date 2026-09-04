// ghostlight.artifact_id: kalsa_veil_marriage_rumor_branch_fold_v0
// ghostlight.fixture_id: veil-marriage-rumor-v0
// ghostlight.scene_id: veil-marriage-rumor-v0.lower-shadow-marriage-desk
// ghostlight.final_ink_path: examples/ink/kalsa/veil-marriage-rumor-v0.branch-and-fold.v0.ink

VAR record_integrity = 1
VAR disclosure_quality = 0
VAR rumor_trace = 0
VAR marriage_momentum = 1
VAR lift_safety = 1
VAR house_pressure = 1
VAR privacy_guard = 2
VAR support_hold = 0

-> start

=== start ===
The lower-shadow marriage desk sits beneath a Sunwall cargo landing, so every vow is accompanied by grain moving overhead and the civic reminder that romance has a load limit.

The long stone gallery has three public openings. A shadow stair descends to the ward street. A barred service hatch leads upward to the lift landing. Between them, the provision wicket keeps tenancy slates and brass sun-court tokens in separate trays, because light can be medicine, privilege, or both and the trays have no opinion worth recording.

Pera Sen works behind the waist-high copying rail. A chained marriage register lies open beside two inkstones. A narrow staff stair rises behind Pera to the duty bench. The wall beside it carries the forecast-review bell and a locked case for sealed records.

-> morning_couple

=== morning_couple ===
Ari Venn and Dema Kor have brought two witnesses, one tenancy transfer, and a breakfast loaf shaped like a radiant sun by someone with only a lower-shadow resident's theoretical understanding of the subject.

Ari is a candidate trained by a Prophetic House. Registering the marriage also registers a move out of house residence. Dema fits brakes on the cargo lifts. Her maintenance strip for today's lower landing says the replacement pawl is serviceable under a reduced load and not under the harvest weight already chalked on the wall board.

Mira Tesh watches from the provision wicket. She can hold the couple's present room and light allotment during a recorded review. She cannot decide their marriage or operate the lift.

The ordinary work is almost finished. Ari and Dema have each spoken consent. Pera has copied the witnesses. The loaf has already lost one ray to civic hunger.

Then Dema points at a folded sheet under the public rail.

-> rumor_leaf

=== rumor_leaf ===
Someone nailed copies across the ward before dawn:

IF ARI VENN MARRIES DEMA KOR, THE LOWER LIFT FALLS BEFORE THE NEXT HARVEST.

The sheet bears no prophet's signature. It does carry the seal-mark used by Ari's house service office, copied badly enough to deny and well enough to frighten a landlord.

Dema's shift was struck from the lift board this morning. Ari's house room is marked for review. Neither office has served a formal notice here.

Pera has time for one careful preparation before the house courier arrives.

-> routine_choice

=== routine_choice ===
// ghostlight.choice_layer: ordinary_record_preparation
+ [Copy the marriage consent and tenancy transfer onto separate leaves before either can become the other's condition.]
    // ghostlight.action_label: write
    // ghostlight.branch_label: separate_consent_and_tenancy
    ~ record_integrity = record_integrity + 2
    ~ privacy_guard = privacy_guard + 1
    ~ marriage_momentum = marriage_momentum + 1
    Pera slides a clean leaf beneath the marriage register and another beneath the tenancy slate.

    "Two promises," Pera says. "They may quarrel without sharing a coffin."

    Dema looks at Ari. "Good. We had planned separate coffins."

    The witnesses sign both margins. Whatever the house sends next will have to name which office it wants to move.
    -> routine_fold
+ [Lay Dema's brake strip beside the chalked harvest load and copy the mismatch into the civic docket.]
    // ghostlight.action_label: use_object
    // ghostlight.branch_label: preserve_material_warning
    ~ lift_safety = lift_safety + 2
    ~ record_integrity = record_integrity + 1
    ~ house_pressure = house_pressure + 1
    Dema places the waxed strip below the load board. The replacement pawl mark ends at the smaller notch. The harvest tally climbs past it with the serene confidence of arithmetic owned by someone else.

    Pera copies both marks.

    "If the lift falls," Dema says, "I would like the record to discover iron before it discovers my marriage."
    -> routine_fold
+ [Flatten the rumor sheet inside an evidence sleeve and list every office that has already acted on it.]
    // ghostlight.action_label: preserve_object
    // ghostlight.branch_label: trace_the_rumor
    ~ rumor_trace = rumor_trace + 3
    ~ privacy_guard = privacy_guard - 1
    Pera smooths the nail tear, sleeves the sheet, and writes three recipients on the cover: lift roster, house residence, ward landlord.

    The rumor has no lawful author. It has excellent distribution.

    Ari watches their names become evidence. Being vindicated later will not make this pleasant now.
    -> routine_fold
+ [Ask Mira to hold the couple's current room and light allotment until the duty bench hears any forecast claim.]
    // ghostlight.action_label: speak
    // ghostlight.branch_label: hold_material_support
    ~ support_hold = support_hold + 3
    ~ house_pressure = house_pressure + 1
    ~ marriage_momentum = marriage_momentum - 1
    Mira turns both tenancy slates face-down in her tray and lays a plain brass hold-ring across them.

    "This holds the room and their bright hour," she says. "It does not hold the house's temper. We lack the shelf space."

    The registration waits while she witnesses the hold.
    -> routine_fold

=== routine_fold ===
// ghostlight.fold: ordinary_work_before_house_pressure
Above the gallery, chains take the weight of another grain cradle. Dust sifts from the old ceiling ribs. The wedding loaf submits a second ray to public service.

{record_integrity >= 3: The marriage, tenancy, brake warning, and rumor now occupy records that can disagree without devouring one another.}
{record_integrity <= 1: Too much still depends on the one open register. One blot or seizure could make several authorities look like one decision.}
{rumor_trace >= 3: Pera can name three offices that acted before any signed notice arrived.}
{rumor_trace == 0: The rumor remains everywhere and nowhere: legible on walls, absent from the civic docket.}
{lift_safety >= 3: Dema's brake strip sits beneath the load chalk, a small material objection to a magnificent future.}
{support_hold >= 3: Mira's brass ring protects the room and light allotment until review, at least inside this ward office.}
{privacy_guard <= 1: The couple's names are now part of a public evidentiary trail whether or not the marriage proceeds.}

The house courier reaches the shadow stair at a run.

-> courier_arrival

=== courier_arrival ===
Tir Osa climbs into the gallery carrying a lacquered record case against her chest. She is an archive copyist wearing a house-gray coat with fresh rain darkening the shoulders. Her fingers are inked to the first knuckle. She has the look of someone who has obeyed every instruction and found one instruction hiding behind the others.

This house separates its packet with cords. Black binds the working forecast. White binds the disclosure prepared for civic use. Red binds the service steward's requested action.

Only the red leaf is displayed.

It asks Pera to delay the marriage, preserve Ari's house residence, and recognize Dema's removal from the lower-lift roster until a forecast of fatal failure expires. The red leaf cites the sealed working. It does not state what the sponsor plans to do if the marriage proceeds.

Tir keeps one hand over the white cord.

"I was told to deliver the case," she says. "I was told the red leaf was enough. Those are not the same sentence."

-> packet_choice

=== packet_choice ===
// ghostlight.choice_layer: sealed_house_packet
+ [Return the red request unstamped until the usable disclosure is placed before the couple.]
    // ghostlight.action_label: refuse
    // ghostlight.branch_label: refuse_undisclosed_burden
    ~ marriage_momentum = marriage_momentum + 2
    ~ record_integrity = record_integrity + 1
    ~ house_pressure = house_pressure + 2
    Pera pushes the red leaf back across the copying rail.

    "A sealed future may visit," Pera says. "It may not take a chair, a spouse, and a lift shift without introducing itself."

    Tir does not smile. House couriers learn early which jokes become evidence. She leaves the red leaf on the public side of the rail.
    -> packet_fold
+ [Open only the white disclosure with Ari, Dema, Tir, and the two witnesses present.]
    // ghostlight.action_label: open_object
    // ghostlight.branch_label: open_usable_disclosure
    ~ disclosure_quality = disclosure_quality + 3
    ~ record_integrity = record_integrity + 1
    ~ privacy_guard = privacy_guard + 1
    ~ house_pressure = house_pressure + 1
    Tir breaks the white wax. Pera leaves the black-cord working forecast sealed in the case.

    The civic leaf names its question, horizon, sponsor, adverse branch, and three intended responses. Two witnesses copy the break in the seal. Ari reads over Pera's shoulder. Dema reads the line about the lower lift twice.

    No one has learned what every future contains. They have learned what the sponsor meant to do to this one.
    -> packet_fold
+ [Open the black working forecast as well and read every intimate branch into the public docket.]
    // ghostlight.action_label: open_object
    // ghostlight.branch_label: expose_raw_forecast
    ~ disclosure_quality = disclosure_quality + 2
    ~ rumor_trace = rumor_trace + 1
    ~ privacy_guard = privacy_guard - 3
    ~ house_pressure = house_pressure + 2
    The black wax cracks.

    Pera finds possible children, separations, illnesses, reconciliations, house defections, and one funeral that may belong to nobody in the room. The working is detailed enough to wound and too changed by this reading to become a clean answer.

    Ari's face closes. Dema pulls the wedding loaf away from the open register as if crumbs are the remaining privacy.
    -> packet_fold
+ [Lock the whole case in civic custody and ring for a forecast reviewer before touching any cord.]
    // ghostlight.action_label: secure_object
    // ghostlight.branch_label: secure_case_for_review
    ~ record_integrity = record_integrity + 2
    ~ support_hold = support_hold + 1
    ~ house_pressure = house_pressure + 2
    ~ marriage_momentum = marriage_momentum - 1
    Pera places the lacquered case in the wall safe, turns both keys with Tir watching, and pulls the review bell.

    Its wire climbs the narrow staff stair. Somewhere above, a duty reviewer acquires a problem before breakfast has finished acquiring them.

    The packet is safe. The people remain under the acts it was sent to justify.
    -> packet_fold

=== packet_fold ===
// ghostlight.fold: sponsor_action_enters_record
The cargo chains overhead settle, then take strain again.

{disclosure_quality >= 3:
The white leaf states the omitted response plainly. If the marriage is entered, the house service steward intends to revoke Ari's training residence, withdraw one forecast watch from the next lower-lift harvest load, and circulate Dema's removal as a settled safety act. The fatal branch follows those responses, a delayed brake replacement, and the full load. The marriage does not cause the fall by itself.
- else:
    {disclosure_quality >= 1:
Pera has enough fragments to see an interested sponsor behind the prediction, but not enough usable disclosure to separate threat from forecast in a hearing.
    - else:
The house's planned response remains under cord. The red request presents the lift's death as something the marriage does unaided.
    }
}

{privacy_guard >= 3: The working forecast remains sealed; only the consequence claimed and sponsor actions enter the challenge.}
{privacy_guard <= 0: The raw branches have entered public custody. Whatever the court excludes, the gallery has already heard.}
{house_pressure >= 4: Tir keeps glancing down the shadow stair. The house did not send guards, but it knows which messenger carried the case.}

The lower-lift bell strikes above them: one heavy note for a harvest cradle entering load.

-> lift_bell

=== lift_bell ===
Dema looks up before the vibration leaves the ceiling.

"That is my crew's cradle," she says. "Or it was yesterday."

The chalked harvest weight still exceeds the brake strip. The house's red request has already removed Dema from the roster. If the white leaf is accurate, the forecast watch is being withdrawn at the same time.

Mira has the provision trays. Pera has the civic docket. Tir has whatever protection remains in the fact that she delivered instead of destroyed the case. The lift steward above has the brake, the load, and the authority to stop them.

No single record can do all four jobs.

-> disclosure_action

=== disclosure_action ===
// ghostlight.choice_layer: disclosure_path_under_load
+ [Ring the lift-service bell and place Dema's brake strip with the available house record in the barred handoff box.]
    // ghostlight.action_label: transfer_object
    // ghostlight.branch_label: route_material_warning
    ~ lift_safety = lift_safety + 3
    ~ disclosure_quality = disclosure_quality + 1
    ~ marriage_momentum = marriage_momentum - 1
    Pera rings twice and opens the civic side of the barred box. Dema's wax strip goes in first. The usable house leaf follows if it has been opened; otherwise Pera adds a copy of the red request and names the missing disclosure on the cover.

    A steward's hand takes the box from the landing side. The bell above answers once. That is acknowledgment, not safety.
    -> action_fold
+ [Post a corrected public notice naming the house's planned withdrawals beside the captured rumor.]
    // ghostlight.action_label: publish
    // ghostlight.branch_label: correct_the_public_path
    ~ rumor_trace = rumor_trace + 3
    ~ disclosure_quality = disclosure_quality + 1
    ~ privacy_guard = privacy_guard - 1
    ~ house_pressure = house_pressure + 1
    Pera hangs the sleeved rumor on the notice rail and pins a civic leaf beside it.

    The correction does not say whether the marriage is wise, whether any child will express Prophecy, or whether the lift will fall. It says the house plans to withdraw residence, labor, and warning if the marriage proceeds.

    People on the shadow stair stop to read. Rumor has acquired an author and an audience at the same time.
    -> action_fold
+ [Send the packet up the staff stair for sealed review and keep the public rail blank.]
    // ghostlight.action_label: transfer_object
    // ghostlight.branch_label: seek_sealed_review
    ~ record_integrity = record_integrity + 2
    ~ privacy_guard = privacy_guard + 2
    ~ support_hold = support_hold + 1
    ~ marriage_momentum = marriage_momentum - 1
    Pera seals the civic copy and passes it through the staff-stair hatch. The review bell sounds once above.

    Mira adds her witness mark to the support hold. Tir adds the delivery time. The street learns nothing new.

    Secrecy now protects a real challenge. It also leaves the house's sentence alone on the walls.
    -> action_fold
+ [Let Tir return the house case, but keep witnessed copies of the red request and Dema's brake strip.]
    // ghostlight.action_label: transfer_object
    // ghostlight.branch_label: preserve_minimum_copy
    ~ record_integrity = record_integrity + 2
    ~ lift_safety = lift_safety + 1
    ~ house_pressure = house_pressure - 1
    ~ disclosure_quality = disclosure_quality - 1
    Pera returns the lacquered case across the rail. Tir ties it under her coat.

    The civic docket keeps the red request, its missing disclosure, Dema's load limit, and four witness marks. That is enough to prove an office acted. It is not enough to show every act that shaped the fatal branch.

    Tir leaves with less immediate danger and more archive to cross.
    -> action_fold

=== action_fold ===
// ghostlight.fold: material_and_information_routes_diverge
The gallery divides its attention among the lift above, the register below, the notice rail, and the locked case.

{lift_safety >= 4: The lift-service bell has carried both a material limit and the interested forecast action to the steward who can stop the load.}
{lift_safety <= 2: The harvest cradle continues taking weight above a warning still trapped at the marriage desk.}
{rumor_trace >= 4: Pera can trace the rumor from wall sheet to roster, residence, and landlord, and the public correction has a path to follow.}
{rumor_trace <= 1: The rumor still outruns every signed answer.}
{record_integrity >= 4: Independent leaves now preserve consent, tenancy, safety, house request, and review custody.}
{support_hold >= 3: Mira's hold-ring keeps room and light allotment from becoming punishment before review.}
{support_hold == 0: No office has yet agreed to carry the couple's material support while the forecast is disputed.}
{house_pressure >= 5: Two house retainers appear at the bottom of the shadow stair and wait where waiting can still pretend not to be force.}

Pera cannot finish every route before the lift bell sounds again.

-> final_choice

=== final_choice ===
// ghostlight.choice_layer: office_priority
+ [Enter the marriage now and mark every house request as a separate contested burden.]
    // ghostlight.action_label: write
    // ghostlight.branch_label: prioritize_marriage_registration
        {marriage_momentum >= 3 && record_integrity >= 3:
        Pera turns the register toward Ari and Dema for the final marks.
        -> ending_registration_success
    - else:
        Pera reaches for the final line with too many authorities still sharing one piece of paper.
        -> ending_registration_cost
    }
+ [Leave the marriage line open and spend the remaining bell on the Sunwall safety path.]
    // ghostlight.action_label: use_object
    // ghostlight.branch_label: prioritize_lift_safety
        {lift_safety >= 4 && disclosure_quality >= 2:
        Pera pulls the lift-service bell again and holds the copied warnings in the barred box.
        -> ending_lift_success
    - else:
        Pera rings with evidence too thin or too late to own the load.
        -> ending_lift_cost
    }
+ [Send a corrected notice through every office that acted on the rumor.]
    // ghostlight.action_label: publish
    // ghostlight.branch_label: prioritize_public_correction
        {rumor_trace >= 4 && disclosure_quality >= 2:
        Pera separates copies for lift roster, house residence, landlord, and public rail.
        -> ending_correction_success
    - else:
        Pera publishes what can be assembled before its path is complete.
        -> ending_correction_cost
    }
+ [Keep the raw forecast sealed, preserve the civic challenge, and make the support hold carry the delay.]
    // ghostlight.action_label: secure_object
    // ghostlight.branch_label: prioritize_bounded_privacy
        {record_integrity >= 4 && privacy_guard >= 3 && support_hold >= 2:
        Pera locks the working copy away and sends only the usable challenge upstairs.
        -> ending_privacy_success
    - else:
        Pera chooses secrecy without enough independent record or support beneath it.
        -> ending_privacy_cost
    }

=== ending_registration_success ===
// ghostlight.ending_label: marriage_registered_with_separate_burdens
// ghostlight.training_hook: consent_is_not_forecast_enforcement
Ari and Dema sign. The witnesses sign. Pera enters the marriage while the house delay, tenancy review, roster removal, and lift warning remain separate contested acts.

The register settles nothing about future children or falling machinery. It settles who consented here.

{support_hold >= 2: Mira keeps the brass hold-ring across the couple's room and light allotment. They leave married and still housed until review.}
{support_hold < 2: The house residence closes before the ward supplies another room. They leave married with the wedding loaf, two witnesses, and nowhere agreed to sleep.}

{lift_safety >= 4: Above them, the harvest cradle stops short of full load.}{lift_safety < 4: Above them, the harvest cradle continues taking weight. Pera has protected one boundary and left another office a warning it may receive too late.}
-> END

=== ending_registration_cost ===
// ghostlight.ending_label: marriage_record_collapses_under_shared_custody
// ghostlight.training_hook: consent_record_without_supporting_separation
The final marks go down, but the red request, tenancy change, and rumor all cling to the same docket.

The house challenges the entry before Ari and Dema reach the shadow stair. Mira cannot tell which support was stayed and which merely assumed. The lift office still treats Dema's removal as settled.

The marriage may survive appeal. The day's record cannot yet explain what else was done in its name.
-> END

=== ending_lift_success ===
// ghostlight.ending_label: material_owner_stops_the_load
// ghostlight.training_hook: disclosure_reaches_actuation_owner
The answer comes from above as iron, not revelation.

The cargo chains slacken. The harvest cradle settles back onto its rests. A Sunwall steward opens the barred box, reads Dema's brake strip beside the house withdrawal, and stops the load under material authority.

No court has ruled the forecast false. No clerk has commanded the lift by prophecy. The owner of the brake has refused a full load until the pawl and crew are inspected.

The marriage register remains open. Ari and Dema must spend another day inside the house's pressure. People keep breathing above them, which is a poor wedding gift but an excellent prerequisite.
-> END

=== ending_lift_cost ===
// ghostlight.ending_label: warning_arrives_without_usable_path
// ghostlight.training_hook: thin_warning_cannot_substitute_for_owner
Pera rings. The steward answers, but the box contains a marriage rumor, a contested removal, and no clean join between the forecast and Dema's material limit.

The lift pauses long enough to empty the first grain cradle. It does not close. Another crew accepts the reduced work under house pressure.

Nobody dies in the gallery. Nobody can yet promise the next load will grant that courtesy. The marriage waits while thin evidence purchases thin safety.
-> END

=== ending_correction_success ===
// ghostlight.ending_label: sponsor_action_follows_rumor_path
// ghostlight.training_hook: correction_travels_with_material_burden
Four corrected notices leave the desk.

The lift roster receives the sponsor's planned withdrawal beside Dema's brake limit. The house residence copy names its eviction as an intended act, not a vision. The landlord receives a stay request. The public rail shows the rumor beside the intervention it omitted.

{privacy_guard >= 2: The black working forecast remains sealed. The ward learns what the house planned, not every private branch in Ari and Dema's possible life.}
{privacy_guard < 2: The correction works, but fragments of the opened working move with it. Strangers now argue over possible children while congratulating themselves for defeating a rumor.}

The house can still defend its forecast. It can no longer claim it never entered the future it sold.
-> END

=== ending_correction_cost ===
// ghostlight.ending_label: public_answer_without_complete_trace
// ghostlight.training_hook: correction_without_custody_chain
Pera posts the strongest answer available.

The ward reads a civic denial beside a house-marked warning. The lift office receives no separate material record. The landlord says the first sheet arrived earlier and looked more certain. Ari's house calls the correction unauthorized.

By evening the rumor has improved: now it says the marriage was so dangerous that the civic desk tried to hide it.

Truth with no custody path makes fine kindling for a better lie.
-> END

=== ending_privacy_success ===
// ghostlight.ending_label: sealed_working_with_supported_challenge
// ghostlight.training_hook: bounded_disclosure_and_material_support
The black-cord working goes into the locked case. The white disclosure and red request go up the staff stair. Mira's hold-ring keeps room and light allotment in place. Independent copies remain with the couple and civic desk.

The duty bench can test the sponsor's omitted intervention without reading possible children, illnesses, or funerals into the street.

The price is visible on the public rail. The rumor remains uncorrected for another sitting, and Dema's name remains absent from the lift roster unless the safety path reached its own owner.

Secrecy has been made narrow enough to protect people. It has not been made harmless.
-> END

=== ending_privacy_cost ===
// ghostlight.ending_label: sealed_record_protects_the_sponsor
// ghostlight.training_hook: secrecy_without_support_or_independent_copy
Pera closes the case.

The house praises the desk's discretion. That is the first bad sign.

Without an independent disclosure, a support hold, and notices sent to the offices already acting, the raw forecast stays private and the sponsor's version stays public. Ari loses the house room. Dema remains off the lift roster. The marriage line waits for a review whose delay the house can afford.

Nothing has leaked except power.
-> END
