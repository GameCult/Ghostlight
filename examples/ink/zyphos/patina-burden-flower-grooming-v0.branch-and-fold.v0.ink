// ghostlight.artifact_id: patina_burden_flower_grooming_v0_branch_fold_v0
// ghostlight.fixture_id: patina-burden-flower-grooming-v0
// ghostlight.scene_id: patina-burden-flower-grooming-v0.returning-light-wash
// ghostlight.final_ink_path: examples/ink/zyphos/patina-burden-flower-grooming-v0.branch-and-fold.v0.ink

VAR flower_hunger = 2
VAR flower_credibility = 2
VAR host_flower_trust = 2
VAR road_credit = 2
VAR light_window = 3
VAR skin_irritation = 1
VAR witness_quality = 0
VAR flower_separated = 0
VAR alarm_strength = 2
VAR departure_delay = 0
VAR seed_beads_released = 0

-> start

=== start ===
The family is due on the road after returning light, which is why the child's burden flower has selected this morning to become complicated.

The grooming place lies on the outer rim of a Sa'auei'a breeding-ground arrival terrace. Root-bound black stone fans wide toward three nursery ramps and narrows routeward at a living boundary of amber fungal candles. Low lantern-tree branches shade the rim. Umbros still covers most of the dim red sun, but a bright crescent is returning around the fixed dark world.

The child folds four tall running legs under a long, dark-fibered body. Two smaller chest limbs remain free above a portable reed mat. On it wait two shallow stone bowls, a packet of mineral salts, and three soft-pronged combs. The burden flower grips a bare patch on the child's left flank: flat blue-gray leaves, pale clasping rootlets, and a cup of fine sensory filaments currently pretending not to watch the salt packet.

-> caretaker_arrives

=== caretaker_arrives ===
The elder cousin settles on the other side of the mat, facial scent-fans open to the wash bowls and the road. Their family has packed everything except the child, the flower, and the ceremonial certainty that both were already packed.

The cousin lays an unknotted route cord beside the routeward bowl.

"Rootlets, leaves, cup, beads," the cousin says. "In that order."

"You say that every time."

"And every time you grow a new order."

Grooming is not decoration. The flower trades warnings for minerals, light, travel, and tolerable attachment. The child gets early notice of sickness and bad water. The road gets testimony it may choose to believe. Everyone gets an opinion before breakfast, because this is a healthy ecology.

-> routine_choice

=== routine_choice ===
// ghostlight.choice_layer: ordinary_grooming
+ [Wet the comb with mineral wash and offer it beneath the clasping rootlets.]
    // ghostlight.action_label: use_object
    // ghostlight.branch_label: feed_rootlets_first
    ~ flower_hunger = flower_hunger - 1
    ~ host_flower_trust = host_flower_trust + 1
    The child dissolves a thumb-claw of salt in the first bowl, wets the comb, and slides its rounded prongs beneath the lowest rootlets.

    The flower releases one grip at a time. Each rootlet curls around a wet prong, drinks, and settles again without pinching.

    "See?" the child says. "Perfect cooperation."

    The cousin smells the quantity of salt and says nothing with considerable effort.
    -> routine_fold
+ [Turn each flat leaf into the returning strip of light.]
    // ghostlight.action_label: gesture
    // ghostlight.branch_label: give_light_turn
    ~ flower_credibility = flower_credibility + 1
    ~ host_flower_trust = host_flower_trust + 1
    ~ light_window = light_window - 1
    The child cups the flower with both chest hands and turns each blue-gray leaf toward the brightening crescent.

    Clear silver passes along the leaf ribs. The flower leans into the light with the grave restraint of an organism that has never once been accused of leaning into anything.

    The cousin shifts one bowl out of its shadow.
    -> routine_fold
+ [Comb shed body fibers away from the attachment patch.]
    // ghostlight.action_label: touch_object
    // ghostlight.branch_label: clear_attachment_patch
    ~ skin_irritation = skin_irritation - 1
    ~ host_flower_trust = host_flower_trust + 1
    ~ alarm_strength = alarm_strength - 1
    The soft prongs gather loose dark fibers without touching live rootlets. Cool air reaches the bare flank patch. The flower lifts two leaves so the child can work beneath them.

    "It likes that," the cousin says.

    "It likes competent service. We have much in common."
    -> routine_fold
+ [Keep lashing the flank packs and let the flower wait another few minutes.]
    // ghostlight.action_label: use_object
    // ghostlight.branch_label: hurry_the_routine
    ~ flower_hunger = flower_hunger + 1
    ~ road_credit = road_credit - 1
    ~ alarm_strength = alarm_strength + 1
    The child tightens a route cord, checks a pack latch, and checks it again in case repetition can make delay respectable.

    The flower walks one deliberate handspan toward the best light. Its rootlets pinch as they regrip.

    "That was not one of the four things," the cousin says.

    "It was adjacent to them."
    -> routine_fold

=== routine_fold ===
// ghostlight.fold: ordinary_care_before_pressure
Around them, the terrace continues its morning. Caretakers trade empty medicine wraps for clean ones. A fungal candle opens beside a basket of shed fibers. Someone down the nursery ramp is teaching three infants to imitate a lantern pulse; the tree remains diplomatically silent on their accuracy.

{flower_hunger <= 1: The burden flower drinks without crowding the bowl, its leaf edges clear and even.}
{flower_hunger >= 3: The flower crowds both bowls, rootlets tightening whenever the child's hands move away.}
{flower_credibility >= 3: Recent color lies in clean bands along the leaves instead of muddying at the edges.}
{skin_irritation <= 0: The bare attachment patch is cool and unbroken beneath the lifted rootlets.}
{skin_irritation >= 1: A narrow dark-red halo remains where yesterday's grit collected under one grip.}
{road_credit <= 1: The routeward fungal candles stay sparse around the family's waiting packs. The road has noticed the schedule being given priority over care.}

The child lifts the comb toward the cup of sensory filaments.

-> alarm_bloom

=== alarm_bloom ===
The burden flower blooms red.

-> alarm_read

=== alarm_read ===
Red runs from the cup through every leaf, followed by violet memory and a sharp yellow fringe. The child recognizes the order: fright from the rain shelter two routes ago, then foreign immune residue, then present hunger or pain. Probably. A flower is an informant, not a labeled diagram.

The fungal road closes three amber candles at the terrace throat. Two departing adults stop beside the family packs. Nobody panics. Everyone notices, which is often more expensive.

"It is old," the child says.

The cousin keeps both chest hands visible above the mat. "Then let us find out whether it is old and fed, or old and useful to repeat."

-> alarm_choice

=== alarm_choice ===
// ghostlight.choice_layer: disputed_alarm
+ [Ask the cousin to witness the colors before any washing continues.]
    // ghostlight.action_label: speak
    // ghostlight.branch_label: preserve_pre_wash_testimony
    ~ witness_quality = witness_quality + 2
    ~ flower_credibility = flower_credibility + 1
    ~ departure_delay = departure_delay + 1
    ~ light_window = light_window - 1
    "Red, violet, yellow," the child says. "Witness it before I change anything."

    The cousin leans close enough for both facial fans to taste the pattern, then taps the three colors into a knotted route cord.

    The waiting adults can now see the delay has a shape. This does not make the child enjoy having one.
    -> alarm_fold
+ [Feed the rootlets and catch the used wash for the fungal verge.]
    // ghostlight.action_label: use_object
    // ghostlight.branch_label: make_wash_comparison
    ~ flower_hunger = flower_hunger - 1
    ~ witness_quality = witness_quality + 1
    ~ road_credit = road_credit + 1
    ~ alarm_strength = alarm_strength - 1
    ~ departure_delay = departure_delay + 1
    ~ light_window = light_window - 1
    The child holds the second bowl under the attachment patch and works fresh mineral wash through the rootlets with the comb.

    Clouded water collects below: salt, old shelter dust, sweat, and a faint violet cast. The flower keeps the red bloom but loses the yellow fringe.

    One closed road candle reopens. It has not forgiven anything. It has received a usable sample.
    -> alarm_fold
+ [Release the flower into the mineral bowl without cutting a live rootlet.]
    // ghostlight.action_label: transfer_object
    // ghostlight.branch_label: separate_flower_carefully
    ~ flower_separated = 1
    ~ skin_irritation = skin_irritation - 1
    ~ host_flower_trust = host_flower_trust - 1
    ~ witness_quality = witness_quality + 1
    ~ alarm_strength = alarm_strength - 1
    The child wets two chest hands and waits for each grip to loosen before lifting it free. The last rootlet holds long enough to make the opinion legible, then releases.

    In the mineral bowl, the flower spreads its leaves until they overlap the rim. Red fades to violet. The child's flank keeps a tender ring of tiny grip marks.

    "Separated is not silenced," the cousin says.

    "The bowl may explain that to the road."
    -> alarm_fold
+ [Fold the pack flap over the bloom and keep the departure line moving.]
    // ghostlight.action_label: conceal_object
    // ghostlight.branch_label: hide_public_bloom
    ~ host_flower_trust = host_flower_trust - 2
    ~ flower_credibility = flower_credibility - 1
    ~ road_credit = road_credit - 2
    ~ alarm_strength = alarm_strength + 1
    The child lowers a soft pack flap across the leaves. The red light disappears against woven route fiber.

    The rootlets bite down. Red leaks through the fabric in five small points.

    At the terrace throat, every remaining candle closes.

    "Excellent," says the cousin. "Now the road knows only that we covered something."
    -> alarm_fold

=== alarm_fold ===
// ghostlight.fold: grooming_as_public_contract
The two wash bowls remain on the reed mat: one clear or barely used, one holding whatever the grooming has made public. The family packs wait routeward. The nursery ramps remain open behind the child. The flower is still a party to the problem, not a stain to be scrubbed out of it.

{witness_quality >= 2: The cousin's route cord preserves the pre-wash red-violet-yellow order.}
{witness_quality == 1: The cousin has enough scent and wash evidence to describe the disagreement, but not the untouched display.}
{witness_quality <= 0: Only the child and flower retain the bloom's original order; the road has seen concealment or delay without a clean comparison.}
{flower_separated == 1: The burden flower sits in the shallow mineral bowl on the mat's routeward edge, leaves spread above the rim.}
{flower_separated == 0: The burden flower remains attached to the child's left flank, rootlets visible beneath lifted or covered leaves.}
{host_flower_trust >= 3: The flower loosens its grip whenever the child's chest hands approach.}
{host_flower_trust <= 1: Every approach makes the rootlets tighten and the red pattern sharpen.}
{alarm_strength <= 1: Hunger-yellow has vanished, leaving a slower red-violet report that can be compared rather than merely endured.}
{alarm_strength >= 3: The bloom keeps adding yellow and red, a loud report made louder by discomfort.}
{departure_delay >= 1: The returning-light crescent widens while the family's place in the departure line quietly becomes someone else's.}

The cousin nudges the second bowl toward the fungal verge. "We can leave evidence, time, comfort, or credibility here. Choose which cost follows us."

-> final_choice

=== final_choice ===
// ghostlight.choice_layer: departure_contract
+ [Work one final comb of wash through the rootlets, leave the sample at the road verge, and move the flower onto a clean pack strap.]
    // ghostlight.action_label: mixed
    // ghostlight.branch_label: finish_witnessed_contract
    {witness_quality >= 2 && road_credit >= 2 && flower_credibility >= 2:
        The cousin steadies the flower while the child works one final comb of mineral wash through its rootlets and catches the runoff in the second bowl.
        The child carries the sample to the outer line of fungal candles.
        -> ending_witnessed_success
    - else:
        The child makes and offers the wash sample with too little shared evidence to make the road generous.
        -> ending_witnessed_cost
    }
+ [Give the flower the next full light interval and let it drop a seed bead before departure.]
    // ghostlight.action_label: wait
    // ghostlight.branch_label: spend_time_on_repair
    ~ seed_beads_released = 1
    ~ departure_delay = departure_delay + 1
    {light_window >= 2 && flower_hunger <= 2 && host_flower_trust >= 2:
        The child turns the mat toward the returning light and settles in for the missed place in line.
        -> ending_light_success
    - else:
        The child offers time after too much of the useful interval or too much trust has already been spent.
        -> ending_light_cost
    }
+ [Move the flower onto a clean outer pack strap, keep its bloom visible, and accept the road's price for uncertainty.]
    // ghostlight.action_label: authorize
    // ghostlight.branch_label: carry_alarm_openly
    The child waits for each rootlet to release, settles the flower onto a clean outer pack strap, and stands on all four running legs with its bloom visible.
    {flower_credibility >= 3 && alarm_strength <= 2 && road_credit >= 1:
        The bloom clears as the child faces the fungal boundary.
        -> ending_open_alarm_success
    - else:
        The bloom remains loud; visibility cannot restore credibility already spent.
        -> ending_open_alarm_cost
    }
+ [Cut the live sensory filaments, rinse the attachment patch, and leave now.]
    // ghostlight.action_label: attack
    // ghostlight.branch_label: silence_and_depart
    The child takes the narrow pruning blade from the salt packet.
    -> ending_silenced

=== ending_witnessed_success ===
// ghostlight.ending_label: witnessed_contract_success
// ghostlight.training_hook: grooming_preserves_testimony_and_host_boundary
The used wash darkens the fungal verge. Amber returns one candle at a time as the road compares salt, old violet residue, skin heat, and the cousin's knotted record.

The flower transfers to a clean outer pack strap. Its rootlets grip woven fiber instead of the tender flank patch. Red-violet remains visible, quieter now and still allowed to be inconvenient.

The family loses its first place in line and keeps its route. The child calls this an unfair compromise until the cousin points out that this is what a fair compromise usually calls itself from inside.
-> END

=== ending_witnessed_cost ===
// ghostlight.ending_label: witnessed_contract_cost
// ghostlight.training_hook: testimony_without_enough_shared_evidence
The road accepts the wash as matter and refuses it as explanation. Two candles reopen, spaced far enough apart to mark the slower, bitter lane.

The flower rides on the outer pack strap. The child walks where the road can read it, lowering both facial fans to taste every bitter toll bead the family receives for the next stretch.

Nothing catastrophic happens. The departure is simply longer, more public, and salted with the knowledge that credibility cannot be mixed fresh in a bowl.
-> END

=== ending_light_success ===
// ghostlight.ending_label: light_repair_success
// ghostlight.training_hook: time_and_seed_cost_restore_symbiont_contract
Returning light reaches the mat. The flower opens flat and silver-blue, drinks from the comb, then drops one dark seed bead into the second bowl.

{seed_beads_released == 1: The child places the bead beside the fungal verge, where a future host may earn or regret the lineage.}

The family leaves with the later group. The child's flank is cool. The flower's red memory has thinned to a narrow violet seam: still testimony, no longer a performance financed by hunger.
-> END

=== ending_light_cost ===
// ghostlight.ending_label: light_repair_cost
// ghostlight.training_hook: late_care_cannot_recover_spent_window_cleanly
The bright strip has already moved beyond the low lantern branches. The flower turns every leaf after it and receives more shadow than light.

It drops a seed bead anyway. The bead is pale and soft. The cousin wraps it rather than offering it to the road.

They miss the departure group and wait for the next candle opening. Care still counts. It is merely more expensive after being postponed, a discovery with an annoyingly durable research record.
-> END

=== ending_open_alarm_success ===
// ghostlight.ending_label: open_alarm_success
// ghostlight.training_hook: credible_inconvenient_witness_remains_public
The child leaves the flower visible. Its bloom settles into clean red-violet bands as the first mineral-fed steps carry them across the boundary.

The road opens a narrow amber lane. Not approval: priced uncertainty. The child lowers both facial fans toward the ground, tastes a little bitterness in the road's damp air, and keeps moving.

Behind them, the cousin records that the old alarm persisted after care. A later shelter will know to ask what the rain station left in both host and flower.
-> END

=== ending_open_alarm_cost ===
// ghostlight.ending_label: open_alarm_cost
// ghostlight.training_hook: visibility_does_not_replace_credibility
The bloom is visible and loud. The road remains dark.

The family can leave by the unlit ground beside it, but they lose the road's guidance, easy footing, and remembered credit for the stretch. The child carries the flower openly and still looks as if concealment happened, because it did.

The cousin says nothing until the first rough slope. This is worse than a lecture and kinder than pretending the lesson was already learned.
-> END

=== ending_silenced ===
// ghostlight.ending_label: coerced_silence_cost
// ghostlight.training_hook: forced_grooming_becomes_evidence
The blade touches live filaments.

The flower throws bitter red sap across the child's chest hands and sheds three unripe beads onto the stone.

{flower_separated == 1:
Its rootlets lash mineral water over the bowl rim. The child's bare flank keeps the earlier ring of grip marks, while the cut flower curls away from every reaching hand.
- else:
Its rootlets tear free all at once. The bare flank patch rises hot and dark, holding the flower's distress in the same memory-bearing tissue the cutting was meant to quiet.
}

The fungal road opens no candles. The family still leaves, but beside the road rather than through it, carrying the child's sap-stained hands and the flower's cut cup as the report.
-> END
