// ghostlight.artifact_id: charter_route_assembly_v0_branch_fold_v0
// ghostlight.fixture_id: charter-route-assembly-v0
// ghostlight.scene_id: charter-route-assembly-v0.low-bend-standing-and-redress
// ghostlight.final_ink_path: examples/ink/zyphos/charter-route-assembly-v0.branch-and-fold.v0.ink

VAR road_evidence = 1
VAR river_evidence = 1
VAR grove_consent = 1
VAR family_record = 1
VAR bearer_standing = 1
VAR source_trace = 0
VAR split_lane = 0
VAR appeal_ready = 0
VAR eclipse_time = 3
VAR herd_strain = 2

-> start

=== start ===
Low Bend is where four authorities meet and none of them has had the courtesy to become furniture.

An amber candle road descends from the upland between the roots of three lantern trees. It reaches a dry, crescent-shaped hearing shelf above a shallow ford. Prismwake mats tile the ford in silver-green sheets. The current runs right to left toward a choir reef at the river mouth. A thumb-wide stone runnel carries that pressure song from the river into a black basin at the shelf's edge.

Two narrow passages leave the shelf. The root lane stays dry beneath the lantern trees. The ford lane crosses the living mats. Beyond both, the road rises again toward the breeding grounds.

Seyr folds four running legs beneath a long striped body at the center of the shelf. The smaller chest limbs remain free to place route cords, taste cups, and archive membranes. For this hearing Seyr is the path-opener: keeper of speaking order, safe lanes, and visible boundaries. Not judge. The distinction is the institution.

-> introduce_parties

=== introduce_parties ===
Ili, old and crooked-fanned, opens the relevant layers of the assembly archive on a low frame of pale ribs. Ili is record-keeper until the hearing ends, at which point the office will disappear and Ili will remain a person with too many opinions.

Varet waits routeward beside a glassback herd. Translucent plates along the grazers' spines hold eclipse warmth in cloudy bands. One pregnant dam keeps turning toward the ford. Yesterday Varet's family fed the candle road a bundle of failed medical grafts at its corpse pocket. The road accepted the bundle. Overnight floodwater reached the pocket. Downstream, the choir reef soured its nursery channel and sent a grievance upriver.

Two route readers from families with no claim at Low Bend serve as bearers. One kneels by the live candles. The other keeps both facial fans above the fresh river basin. They may translate current signals. They may not improve them.

A threadwing courier worries a salt bead on the highest route cord. It has brought dried mineral taste from the river mouth and is being paid to wait. This arrangement satisfies nobody, which is often a promising start for public law.

-> routine_choice

=== routine_choice ===
// ghostlight.choice_layer: prepare_public_record
+ [Feed the candle road a clean strip from the same graft batch and ask it to repeat yesterday's acceptance.]
    // ghostlight.action_label: transfer_object
    // ghostlight.branch_label: prime_road_evidence
    ~ road_evidence = road_evidence + 2
    Seyr lifts the sealed strip with two chest digits and lays it inside a ring of amber fruiting beads.

    The candles lean inward. A sweet mineral scent rises, then the road opens one bright line to its corpse pocket.

    The road-bearer says, "Accepted as clean dead matter. Route credit granted."

    Varet exhales.

    Ili touches one archive membrane. "Acceptance is not absolution. Otherwise every rubbish pit would be a priest."
    -> routine_fold
+ [Empty and refill the river basin so the choir reef's pressure answer is visibly fresh.]
    // ghostlight.action_label: use_object
    // ghostlight.branch_label: prime_river_evidence
    ~ river_evidence = river_evidence + 2
    ~ eclipse_time = eclipse_time - 1
    Seyr lifts the black stone basin from its runnel, tips the old water downslope, refills it from the ford, and seats it against the channel again.

    Three low pulses wrinkle the surface. A pause. Then a tight spiral forms against the current: nursery closed, foreign memory present, return what entered.

    The river-bearer repeats the pattern and names the uncertainty. "The reef reports injury. It has not named the carrier."

    "It has excellent restraint," Varet says. "Everyone should be delighted."
    -> routine_fold
+ [Ask the lantern grove to light only the routes it is willing to recognize during the hearing.]
    // ghostlight.action_label: gesture
    // ghostlight.branch_label: prime_grove_consent
    ~ grove_consent = grove_consent + 2
    Seyr opens both facial fans toward the three trunks and points first to the root lane, then the ford.

    Cold blue knots wake under the nearest canopy. The dry root lane glows. The ford lane receives one white warning pulse and no invitation.

    The grove-bearer is unnecessary; the answer is visible enough to embarrass interpretation.

    Ili records both the light and the embarrassment.
    -> routine_fold
+ [Open Varet's offered archive layer and compare the graft custody marks without exposing family memory.]
    // ghostlight.action_label: inspect_object
    // ghostlight.branch_label: prime_family_record
    ~ family_record = family_record + 2
    Seyr and Ili lift one flexible membrane from Varet's flank case. Pressure knots show the graft station, the rejected tissues, the sealed carry, and the road's bright acceptance. Deeper lineage layers stay folded beneath a clasp.

    "Relevant custody only," Seyr says.

    "A tragic setback for everyone who came hoping to inspect Varet's ancestors," Ili says.

    Varet's facial fans flatten. "They would object to the company."
    -> routine_fold

=== routine_fold ===
// ghostlight.fold: routine_hearing_before_pressure
The hearing settles into its ordinary metabolism.

Ili records signals on the open membrane. The bearers state source, freshness, and doubt before every translation. Seyr moves a pale route cord after each speaker so nobody can pretend volume was standing.

{road_evidence >= 3: The candle road keeps its corpse-pocket line bright: yesterday's acceptance remains part of the public record.}
{river_evidence >= 3: The fresh basin continues to carry the reef's tight nursery-closure spiral.}
{grove_consent >= 3: Cold lantern light makes the dry root lane an explicit offer and leaves the ford visibly uninvited.}
{family_record >= 3: Varet's custody membrane shows an unbroken chain from failed graft station to accepted road pocket.}
{eclipse_time <= 2: Umbros has already taken a dark bite from the sun. Every careful repetition now spends safe travel light.}

The threadwing swallows its salt bead.

Then the ford shines silver.

-> contamination_arrives

=== contamination_arrives ===
// ghostlight.scene: contamination_pressure
Mirror amoebae shimmer between the prismwake sheets: memory-copying cells loose in water, bright as a thought that has entered the wrong head.

The mats fold away from the current. The choir reef's pressure song hardens through the basin. The candle road extinguishes every bead nearest the wet bank.

At the routeward edge, the pregnant glassback dam stamps once. Her dorsal plates fog dark. The herd has spent most of its stored warmth waiting, and eclipse shadow is crossing the slope.

Varet steps to the speaking cord. "Close the cargo lane. Let the herd use the roots. The reef does not raise calves and the grove does not filter water. They can both be right without killing her."

The river-bearer looks to Seyr. The path-opener cannot decide the remedy. The path-opener can decide what the assembly is allowed to call evidence.

-> evidence_choice

=== evidence_choice ===
// ghostlight.choice_layer: establish_injury_and_source
+ [Pay the lattice ants in sugar and place their diagnostic bridge across the wet sediment line.]
    // ghostlight.action_label: transfer_object
    // ghostlight.branch_label: trace_upstream_seep
    ~ source_trace = 2
    ~ bearer_standing = bearer_standing + 1
    ~ river_evidence = river_evidence + 1
    ~ eclipse_time = eclipse_time - 1
    A thumb-sized sugar cake goes onto dry stone. Lattice ants boil from a root seam, eat half the fee, and become architecture with the rest.

    Their bodies lock into a black mesh across the sediment. Microbial glue clouds silver where floodwater crossed. The trace runs from an older upstream wound pocket, beneath yesterday's graft deposit, and only then into the river.

    Varet's bundle joined the leak. It did not begin it.

    Ili records this distinction before relief can turn it into innocence.
    -> evidence_fold
+ [Move Varet's remaining graft bundle into a dry quarantine spur and watch which signals follow it.]
    // ghostlight.action_label: move_object
    // ghostlight.branch_label: trace_family_bundle
    ~ source_trace = 1
    ~ road_evidence = road_evidence + 1
    ~ river_evidence = river_evidence + 1
    ~ herd_strain = herd_strain + 1
    Seyr draws a hook in the candle beads. Varet uses the chest limbs to drag the sealed bundle onto a dry fungal spur while the herd holds back.

    The nearest candles pale. Silver moisture beads around the wrapping seam. In the basin, the reef repeats its closure pulse.

    The road accepted the dead graft. The flood made that acceptance everybody's problem.

    The pregnant dam stamps again, plates dimmer now.
    -> evidence_fold
+ [Mark a provisional dry lane for the herd while all graft cargo remains behind the speaking cord.]
    // ghostlight.action_label: move
    // ghostlight.branch_label: prepare_split_passage
    ~ split_lane = split_lane + 2
    ~ grove_consent = grove_consent + 1
    ~ road_evidence = road_evidence + 1
    ~ herd_strain = herd_strain - 1
    Seyr shifts the route cords so the root lane opens to empty flanks and living bodies but not packs. Varet removes the graft frame from the pregnant dam. Two lantern trees answer with blue knots. The third stays white.

    The glassbacks turn broadside. Their plates show fear, fatigue, and a willingness to test the dry ground one careful body at a time.

    A provisional lane is not a ruling. It is time purchased with visible limits.
    -> evidence_fold
+ [Send the threadwing back to the river mouth carrying fresh ford water and the first hearing record.]
    // ghostlight.action_label: transfer_object
    // ghostlight.branch_label: prepare_second_crossing
    ~ appeal_ready = appeal_ready + 2
    ~ river_evidence = river_evidence + 1
    ~ eclipse_time = eclipse_time - 1
    Seyr seals a drop of silver-flecked ford water beside the first record membrane and ties both to the courier's route braid.

    The threadwing tastes the offered salt, judges it adequate, and launches downcurrent through the lantern branches.

    The assembly now has an external witness and less daylight. Procedure remains offensively material.
    -> evidence_fold

=== evidence_fold ===
// ghostlight.fold: evidence_enters_public_record
The assembly reforms around the changed ford.

{source_trace == 2: The ant lattice points upstream past Varet's deposit. The family is implicated in spread, not origin.}
{source_trace == 1: Silver moisture on the bundle gives the reef's grievance a specific custody trail.}
{source_trace == 0: The source remains ambiguous. Everyone has a theory, which is what evidence is meant to prevent from becoming a government.}
{split_lane >= 2: The root lane stands open to unladen herd bodies while cargo remains behind the cord.}
{appeal_ready >= 2: A threadwing carries fresh water and the first record toward the downstream reef, making a second crossing possible after the eclipse.}
{eclipse_time <= 1: The fixed dark world covers most of the sun. Lantern knots and glassback heat are becoming survival infrastructure.}
{herd_strain >= 3: The pregnant dam's plates have gone almost opaque; another long delay may turn procedure into injury.}

Ili taps the open archive. "We have injury, possible source, and several parties who can ruin each other's week. Standing next."

-> standing_test

=== standing_test ===
The road-bearer has eaten from Varet's family twice this season. The river-bearer learned reef pressure song among Varet's rivals. Neither fact disqualifies a reading by itself. Hidden debt does.

The candle road pulses amber beside its bearer. The river basin tightens under its own pattern. The lantern grove keeps the root lane blue. The glassback herd turns flank-on to all three and lets its plate state be seen.

No one speaks for the landscape. They carry one answer from one body through one channel, under conditions the rest can inspect.

Seyr moves the speaking cord.

-> standing_choice

=== standing_choice ===
// ghostlight.choice_layer: test_public_standing
+ [Replace the road-bearer for this hearing and have an uninvolved reader repeat the live candle sequence.]
    // ghostlight.action_label: procedural_request
    // ghostlight.branch_label: replace_indebted_bearer
    ~ bearer_standing = bearer_standing + 2
    ~ road_evidence = road_evidence - 1
    The first bearer names the meals and steps back. No disgrace is declared; the signal simply loses a mouth that owed one claimant.

    A second reader approaches from the far cord and repeats the candle taste. The road still says it accepted the graft. Its answer is weaker on whether acceptance included flood risk.

    Varet dislikes the correction and cannot call it exclusion. That is the point.
    -> remedy_threshold
+ [Require the reef grievance to agree in fresh basin pressure and courier-carried mineral taste.]
    // ghostlight.action_label: compare_evidence
    // ghostlight.branch_label: crosscheck_river_channels
    ~ bearer_standing = bearer_standing + 1
    ~ river_evidence = river_evidence + 1
    ~ eclipse_time = eclipse_time - 1
    Ili lays the courier's dried mineral thread beside the basin. The river-bearer reads the pressure spiral; another reader tastes the thread.

    Both report nursery closure and foreign memory. Only the live water asks for immediate passage refusal.

    The difference enters the record instead of being polished away.
    -> remedy_threshold
+ [Open only the custody layer of Varet's archive and make every bearer point to the claim it supports.]
    // ghostlight.action_label: show_object
    // ghostlight.branch_label: bound_archive_testimony
    ~ family_record = family_record + 1
    ~ bearer_standing = bearer_standing + 1
    The membrane lies flat: graft station, seal, carrier, road pocket, flood interval. Each bearer touches only the segment supporting their translation.

    The lineage clasp stays shut.

    "Public evidence," Ili says, "is not a festival where privacy is eaten until everyone feels honest."
    -> remedy_threshold
+ [Let the glassbacks accept or refuse the provisional root lane before treating it as a remedy.]
    // ghostlight.action_label: wait
    // ghostlight.branch_label: ask_herd_consent
    ~ split_lane = split_lane + 1
    ~ grove_consent = grove_consent + 1
    ~ herd_strain = herd_strain - 1
    Seyr lowers the route cord and steps aside.

    The pregnant dam approaches the blue-lit roots, tastes the air, and places one forefoot on dry soil. The herd's plates brighten by degrees. They accept the lane and stop short of the white-lit ford.

    The assembly records a herd answer without inventing herd speech.
    -> remedy_threshold

=== remedy_threshold ===
// ghostlight.fold: remedies_presented_to_parties
The assembly has reached the dangerous part: a remedy must become action.

{road_evidence >= 3: The candle road's acceptance is strong enough that any remedy must name its responsibility, not use it as scenery.}
{river_evidence >= 4: The choir reef's nursery closure is confirmed strongly enough to bind water passage if the bearers retain standing.}
{grove_consent >= 3: The lantern grove offers the dry root lane as a separate channel, visibly bounded by blue and white knots.}
{family_record >= 3: Varet's custody record is precise enough to support liability or exoneration without opening lineage memory.}
{bearer_standing >= 3: The translated claims have survived debt disclosure and cross-check strongly enough for a recognized ruling.}
{bearer_standing <= 2: At least one translated claim remains too entangled for a severe remedy to travel well beyond this shelf.}
{source_trace == 2: The strongest physical trace places the original leak upstream and Varet's deposit later in the chain.}
{source_trace == 1: The strongest physical trace follows Varet's remaining graft bundle.}
{split_lane >= 3: Herd, grove, and route geometry now agree on a cargo-free dry passage.}
{appeal_ready >= 2: Fresh evidence is already moving toward a second crossing after the eclipse.}

Seyr cannot pronounce judgment. Seyr can put one bounded remedy before the parties and ask which bodies will recognize it.

-> final_choice

=== final_choice ===
// ghostlight.choice_layer: propose_bounded_remedy
+ [Put immediate ford closure and downstream restitution before the parties.]
    // ghostlight.action_label: propose_remedy
    // ghostlight.branch_label: remedy_close_and_restore
    {river_evidence >= 4 && bearer_standing >= 3:
        -> ending_closure_recognized
    - else:
        -> ending_closure_contested
    }
+ [Put split passage before them: unladen herd by the roots, graft cargo quarantined, family and road restoring the river.]
    // ghostlight.action_label: propose_remedy
    // ghostlight.branch_label: remedy_split_channels
    {split_lane >= 3 && grove_consent >= 3 && road_evidence >= 2:
        -> ending_split_recognized
    - else:
        -> ending_split_fails
    }
+ [Put Varet's limited exoneration before them while the ford stays closed: preserve route credit, keep repair duty.]
    // ghostlight.action_label: propose_remedy
    // ghostlight.branch_label: remedy_exonerate_and_open
    {source_trace == 2 && family_record >= 3:
        -> ending_exoneration_recognized
    - else:
        -> ending_exoneration_fails
    }
+ [Suspend the local ruling and invoke a second crossing after eclipse with a fresh reef answer and a different set of bearers.]
    // ghostlight.action_label: invoke_redress
    // ghostlight.branch_label: remedy_second_crossing
    {appeal_ready >= 2 && bearer_standing >= 3:
        -> ending_redress_ready
    - else:
        -> ending_redress_thin
    }

=== ending_closure_recognized ===
// ghostlight.ending_label: closure_recognized
// ghostlight.training_hook: ecological_injury_binds_passage
The basin twists tight. The ford mats fold shut. The road extinguishes its wet-bank candles and opens a dry spur for the quarantined grafts.

Varet's family must carry mineral replacement downstream, return contaminated tissue under reef instruction, and spend the next safe light tending the prismwake injury. {source_trace == 2: The record names the older upstream seep as first source, so the family pays for spread rather than origin.}{source_trace == 1: The record names the family's accepted bundle as the live custody source.}{source_trace == 0: The record leaves origin unresolved and limits liability to containment work.}

{split_lane >= 3 && grove_consent >= 3: The glassback herd uses the accepted root lane, packs left behind.}{split_lane < 3 || grove_consent < 3: The herd turns uphill toward a longer shelter route, stored heat visibly draining from its plates.}

The ruling hurts. That does not make it illegitimate. The river keeps its nursery closed, and everyone can point to the signals that made the closure public.
-> END

=== ending_closure_contested ===
// ghostlight.ending_label: closure_contested
// ghostlight.training_hook: severe_remedy_without_standing
Seyr puts closure forward. The river tightens. The road does not answer. One bearer repeats a signal the other will not certify.

The ford still goes dark because water and mats can enforce what the assembly cannot yet make portable. Varet calls it capture by interpreters. The next family arriving will hear three versions before they see the record.

{herd_strain >= 3: The pregnant dam is led uphill with opaque plates and a stumbling gait; procedure has produced a new injury while naming the first.}

Local force has outrun recognized authority. Ili marks the remedy disputed and leaves the lineage clasp shut.
-> END

=== ending_split_recognized ===
// ghostlight.ending_label: split_passage_recognized
// ghostlight.training_hook: channel_specific_compromise
Blue lantern knots carry the glassbacks under the roots one unladen body at a time. White knots keep every pack and graft frame behind the cord. The ford remains silver and empty.

The candle road accepts the quarantined dead tissue back into a dry spur. Varet's family owes mineral return and care labor downstream. The road owes filtration and a rebuilt corpse pocket above flood reach. The reef owes no passage until its nursery answers clean.

The pregnant dam crosses first. Her plates brighten when the herd closes around her on the far side.

No party gets everything. More importantly, no party is made to pretend it consented to a channel it refused.
-> END

=== ending_split_fails ===
// ghostlight.ending_label: split_passage_failed
// ghostlight.training_hook: compromise_without_material_consent
Seyr offers the root lane as if geometry alone were agreement.

{grove_consent < 3: The lantern knots stay white. The grove has not offered enough shelter to carry the herd safely.}
{split_lane < 3: The glassbacks bunch at the speaking cord; nobody has asked the herd whether the narrow route fits fear, pregnancy, and four running legs.}
{road_evidence < 2: The candle road darkens its dry spur, refusing custody of a remedy built on an unread account.}

Varet hears compromise. The other bodies hear an impatient family wearing Seyr's voice.

The proposal fails on the shelf. That is cheaper than having it fail beneath a living herd.
-> END

=== ending_exoneration_recognized ===
// ghostlight.ending_label: exoneration_recognized
// ghostlight.training_hook: evidence_separates_origin_from_spread
The ant lattice holds its black line upstream. Ili lays Varet's custody membrane beside it. The family carried an accepted bundle into a system already leaking silver memory.

The record clears Varet's family of causing the first wound. It does not clear them of helping floodwater spread it. Their route credit survives; their repair duty remains.

The road opens its dry candles. The grove lights the root lane. The river keeps the ford closed until the old wound pocket is burned clean.

Exoneration proves to be smaller than innocence and more useful.
-> END

=== ending_exoneration_fails ===
// ghostlight.ending_label: exoneration_failed
// ghostlight.training_hook: archive_claim_without_trace
Seyr puts exoneration forward. Varet's archive shows careful custody. The ground does not show where the first silver cells entered.

{family_record < 3: Even the offered membrane has gaps between graft station and road pocket.}
{source_trace != 2: No independent trail places origin upstream of the family deposit.}

The river refuses the ford. The road keeps yesterday's credit bright. Two true records stare at each other and fail to become one truth.

Ili marks the proposal unrecognized. Varet can appeal, but not by asking the same evidence to become louder.
-> END

=== ending_redress_ready ===
// ghostlight.ending_label: second_crossing_ready
// ghostlight.training_hook: appeal_as_costly_fresh_hearing
The local remedy remains: ford closed, root lane provisional, cargo held above the water.

But the first record is already in flight. After eclipse, another route junction will hear fresh reef pressure through a different basin. Different bearers will disclose different debts. Varet must feed the road again, carry Ili's sealed membrane, and wait while the injured nursery answers on its own time.

The delay costs warmth, food, witness labor, and pride. It also prevents Low Bend from becoming the only mouth the river is allowed to have.

The threadwing vanishes downstream with the record, having negotiated double salt for the return journey. Constitutional principle continues to attract contractors.
-> END

=== ending_redress_thin ===
// ghostlight.ending_label: second_crossing_unready
// ghostlight.training_hook: appeal_without_fresh_signal
Seyr invokes a second crossing. Nothing has yet carried fresh evidence there, and the disputed bearers remain attached to the record.

The words offer delay without correction. The river closes anyway. The road prices every waiting body. The herd turns uphill under deepening eclipse.

Ili records the request but not the promise. "Redress is another hearing," the old keeper says. "It is not this hearing becoming immortal."

Varet must decide whether to pay for witnesses, food, and a fresh answer after the light returns. Until then, the appeal is an intention with excellent posture.
-> END
