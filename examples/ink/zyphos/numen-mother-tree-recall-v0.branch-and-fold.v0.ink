// ghostlight.artifact_id: numen_mother_tree_recall_v0_branch_fold
// ghostlight.fixture_id: numen-mother-tree-recall-v0
// ghostlight.scene_id: numen-mother-tree-recall-v0.shale-root-recall-audience
// ghostlight.final_ink_path: examples/ink/zyphos/numen-mother-tree-recall-v0.branch-and-fold.v0.ink

VAR archive_energy = 2
VAR route_light = 3
VAR witness_integrity = 1
VAR contamination_risk = 2
VAR tree_consent = 2
VAR tender_boundary = 3
VAR warning_clarity = 1
VAR legal_standing = 2

-> start

=== start ===
Shale-Root Gallery lies between the surface and the deep organs of a matriarch tree on the Airawa Home Continent. It is shallow enough for visitors, old enough to have opinions, and grown around a split in dark stone where the tree first survived a landslide.

Outside, Umbros hangs fixed and enormous. The dim sun has begun to cross behind it. Cold blue lantern knots mark the ledge; amber fungal beads mark the path inward; the gallery's central root aperture waits behind a curtain of translucent archive tissue.

-> introduce_tender

=== introduce_tender ===
The archive tender hooks both taloned feet into bark scars and braces one clawed upper hand against a load root. Her other upper hand carries the mineral bowl. Her two smaller lower hands sort contract tokens, a shed threadwing vane, and a damp fungal witness strand.

This is why Airawa recall work has so many shelves at chest height and so few chairs. The tree designed the room. It has never had knees and remains professionally incurious about them.

-> introduce_witnesses

=== introduce_witnesses ===
The route witness waits on the outer ledge, six limbs folded close from a hard climb. Fresh gray dust lies in the seams of their grown plates. They found a fracture above the lower pollinator path, and the old collapse memory could say which root bridge survived last time.

A silver-vane threadwing perches on a salt cup near the aperture. Beneath it, the candle road raises three amber beads around the fungal strand: admitted, relevant, not yet trusted.

The audience is ordinary work. Feed the archive. Seat the witnesses. Ask the dead one precise question. The tree bills memory in sugar, so anyone calling this worship has never balanced the stores.

-> preparation_choice

=== preparation_choice ===
// ghostlight.choice_layer: audience_preparation
+ [Pour the reserve sugar and mineral mash into the root cups.]
    // ghostlight.action_label: use_object
    // ghostlight.branch_label: prepare_archive_energy
    ~ archive_energy = archive_energy + 2
    ~ route_light = route_light - 1
    ~ tree_consent = tree_consent + 1
    The tender tips the mineral bowl. Thick amber mash enters four root cups and vanishes through contracting pores.

    Outside, one rank of lantern knots dims as stored sugar moves below. Small traffic will have less light during totality. The aperture loosens anyway, pleased in the metabolic sense and therefore politically.
    -> routine_fold
+ [Seat the threadwing's shed vane on the contract token and release its salt payment.]
    // ghostlight.action_label: transfer_object
    // ghostlight.branch_label: prepare_courier_witness
    ~ witness_integrity = witness_integrity + 2
    ~ route_light = route_light - 1
    The tender knots the silver sensory ribbon through a carved plate token. The threadwing tastes the salt, flares its remaining vanes, and lands beside the trace it shed three routes ago.

    The lantern grove answers with a blue pulse allocating the courier a safe departure lane. Evidence has transport costs. Even the sacred paperwork needs a runway.
    -> routine_fold
+ [Add a sliver of your own shed scale to the witness cup.]
    // ghostlight.action_label: offer_body_trace
    // ghostlight.branch_label: prepare_personal_trace
    ~ witness_integrity = witness_integrity + 1
    ~ tender_boundary = tender_boundary - 1
    ~ tree_consent = tree_consent + 1
    A lower hand lifts a loose scale from the tender's wrist seam. It carries recent work: bark pressure, route dust, the taste of the mineral bowl, and the private irritation of being billed by one's oldest relative.

    The shallow roots flex toward it. The tree now has a living comparison and a little more of the tender than it had this morning.
    -> routine_fold
+ [Test the fungal witness strand against the candle road before admitting it.]
    // ghostlight.action_label: inspect_object
    // ghostlight.branch_label: prepare_fungal_crosscheck
    ~ contamination_risk = contamination_risk - 1
    ~ witness_integrity = witness_integrity + 1
    ~ warning_clarity = warning_clarity + 1
    ~ route_light = route_light - 1
    The tender lowers the damp strand into the triangle of amber beads. They close around it, tasting mineral history, foot pressure, and sickness.

    One bead turns bitter red, then amber again. The road has found a foreign regularity in the trace. It cannot name the source, but the warning has cost enough light that the next beacon outside stays dark.
    -> routine_fold

=== routine_fold ===
// ghostlight.fold: prepared_recall_audience
The tender seats the contract tokens along the aperture rim. The route witness grips the outer load root. The threadwing settles or watches from its salt cup. Fungal beads breathe their dim account upward.

{archive_energy >= 4: The fed aperture swells translucent and warm, ready to spend years of stored light on a few useful seconds.}
{archive_energy <= 2: The aperture opens narrowly. The tree will make every image earn its sugar.}
{route_light <= 2: Beyond the gallery, blue route knots go sparse. The cost of this memory is already visible to travelers.}
{witness_integrity >= 3: Courier vane, contract token, and route trace agree on where the claim has traveled.}
{witness_integrity <= 1: The witness set is thin enough that the tree must choose which absence to believe.}
{tender_boundary <= 2: The tender's wrist seams glow faintly where her own trace has made the question personal.}

-> opening_recall

=== opening_recall ===
The archive tissue touches every offering at once.

The remembered body arrives in pieces. A taloned foot finds a hold that died generations ago. Upper claws take the weight. Lower hands cover an infant's facial fans against stone dust. Rain tastes of iron. Someone sings a route count because panic is easier to carry when it has rhythm.

The memory passes through tree, fungus, courier trace, and Airawa symbionts. Nobody sees it from outside. Everyone present becomes one of its temporary organs.

The route witness laughs once, startled. "Our ancestors complained about this climb too."

The tender's borrowed lower hand answers with an obscene old gesture before she can stop it.

Then the remembered left foot kicks toward a ledge that never existed.

-> contaminated_mismatch

=== contaminated_mismatch ===
// ghostlight.training_hook: embodied_recall_mismatch
The error repeats with perfect timing. Foot. Pause. Foot. Pause. Too even for a frightened body, too smooth for a fungal road, stripped of the quarrels that make a native memory alive.

A cadence shaped like an imperial ritual payload—the engineered rhythm used to
turn living memory toward obedience—is trying to enter through the old
collapse. Whether it is residue, probe, bait, or damage, the audience cannot
tell.

The matriarch clamps the deep root channels shut. Archive tissue pales around the tender's hands. The candle road raises a bitter quarantine ring. The threadwing lifts its vanes and refuses the air above the suspect strand.

From the ledge comes a dry crack. Dust falls past a blue lantern knot. The present slope has stopped waiting for the past to become trustworthy.

-> contamination_choice

=== contamination_choice ===
// ghostlight.choice_layer: archive_contamination_response
+ [Press both lower hands to the shallow membrane and carry the mismatch deeper into your own symbionts.]
    // ghostlight.action_label: touch_object
    // ghostlight.branch_label: deepen_embodied_recall
    ~ warning_clarity = warning_clarity + 2
    ~ tender_boundary = tender_boundary - 2
    ~ contamination_risk = contamination_risk + 1
    ~ tree_consent = tree_consent + 1
    The tender spreads four blunt digits on each lower hand. The membrane answers with cold pressure.

    The false cadence enters first. Behind it comes the ancestor's real terror, lurching and contradictory: the lower bridge failed after the third root groan, not the first. Useful truth and authored rhythm occupy the same muscles. Her own foot cannot decide which ledge to seek.
    -> pressure_fold
+ [Ask the threadwing to compare the suspect trace with the memory in its living vanes.]
    // ghostlight.action_label: speak
    // ghostlight.branch_label: call_courier_comparison
    ~ witness_integrity = witness_integrity + 2
    ~ warning_clarity = warning_clarity + 1
    ~ route_light = route_light - 1
    "One circuit," the tender tells the threadwing. "Your route, your refusal."

    The courier accepts the invitation by taking another salt crystal and no oath whatsoever. It circles the aperture, silver vanes rippling through electrical and chemical gradients. A lantern knot outside goes dark to mark the return lane it has been promised.

    On the second circuit the threadwing strikes the suspect strand with a targeted dropping and lands well away from it. An accusation with wings remains an accusation.
    -> pressure_fold
+ [Pull the fungal strand into the road's bitter ring and close that witness channel.]
    // ghostlight.action_label: move_object
    // ghostlight.branch_label: quarantine_fungal_witness
    ~ contamination_risk = contamination_risk - 2
    ~ witness_integrity = witness_integrity - 1
    ~ warning_clarity = warning_clarity + 1
    ~ legal_standing = legal_standing + 1
    ~ tree_consent = tree_consent + 1
    Anchored by feet and upper claws, the tender uses both lower hands to lift the strand clear of the aperture. The bitter beads open a narrow path and close behind it.

    The tree's surface pulse slows. The road accepts custody. The audience now has a cleaner archive and a missing witness; future law will respect the quarantine and argue with the resulting uncertainty for years.
    -> pressure_fold
+ [Withdraw your contract token and ask the matriarch to choose what may pass.]
    // ghostlight.action_label: withhold_object
    // ghostlight.branch_label: defer_to_tree_boundary
    ~ tree_consent = tree_consent + 2
    ~ archive_energy = archive_energy - 1
    ~ warning_clarity = warning_clarity + 1
    ~ legal_standing = legal_standing - 1
    The tender pulls her plate token from the aperture and bows her upper arms away from the load root. It is the posture that says the tree owns this boundary, not the answer.

    Deep tissue seals. A shallow root presses one blunt direction into the tender's ankle: outer ledge, uphill. The matriarch has spent almost no archive energy and offered no precedent anyone else can invoke. It has given survival advice on its own authority.
    -> pressure_fold

=== pressure_fold ===
// ghostlight.fold: contaminated_audience_threshold
The recall audience has become a boundary hearing with a hillside attached.

{contamination_risk <= 1: The bitter quarantine ring holds a clean edge. The false cadence fades when it reaches the separated fungal strand.}
{contamination_risk >= 3: The perfect foot-pulse still repeats in the tender's muscles and along the aperture rim.}
{warning_clarity >= 3: Beneath the contamination, the old memory has yielded one usable fact: the lower root bridge fails after the third groan.}
{warning_clarity <= 2: The memory offers pain, dust, and two possible routes with equal conviction.}
{tree_consent >= 4: The matriarch's shallow roots remain open around the tender, a consent expressed as continued permeability.}
{tree_consent <= 2: The aperture narrows until even the contract tokens look presumptuous.}
{legal_standing >= 3: The fungal road has accepted quarantine custody, making the refusal legible to other local bodies.}
{legal_standing <= 1: Only the tree's directional pressure supports the warning. A living power has spoken; the archive has not testified.}
{route_light <= 1: Outside, the lower path is almost dark. Any warning must now travel by body, root tremor, or debt.}

The route witness looks from the pale aperture to the dust falling off the ledge.

"Full ancestor, bounded warning, tree's choice, or your body," they say. "I miss when routine work was merely coercive."

The gallery cracks again.

-> final_choice

=== final_choice ===
// ghostlight.choice_layer: recall_priority
+ [Spend the remaining reserve and ask for the whole collapse memory.]
    // ghostlight.action_label: spend_resource
    // ghostlight.branch_label: prioritize_full_recall
    {archive_energy >= 3 && witness_integrity >= 3 && contamination_risk <= 2 && route_light >= 1:
        The tender pours the last reserve into the root cups and holds every corroborating trace in place.
        -> ending_full_recall_success
    - else:
        The tender pours what remains. The archive opens farther than the witnesses can safely support.
        -> ending_full_recall_cost
    }
+ [Accept the bounded fragment, close the archive, and warn the lower route.]
    // ghostlight.action_label: close_and_warn
    // ghostlight.branch_label: prioritize_bounded_warning
    {warning_clarity >= 3 && legal_standing >= 2 && contamination_risk <= 2:
        The tender closes the contract tokens over the aperture and stamps the third-groan warning into the load root with upper-claw pressure.
        -> ending_bounded_warning_success
    - else:
        The tender closes the archive around a warning that has not earned enough witnesses.
        -> ending_bounded_warning_cost
    }
+ [Return the decision to the matriarch and follow the route it illuminates.]
    // ghostlight.action_label: yield_authority
    // ghostlight.branch_label: prioritize_tree_authority
    {tree_consent >= 4 && contamination_risk <= 2:
        The tender clears every token from the aperture and waits for the tree to move first.
        -> ending_tree_authority_success
    - else:
        The tender yields the decision after the tree has already begun to close.
        -> ending_tree_authority_cost
    }
+ [Carry the unsettled route reflex out in your own symbionts.]
    // ghostlight.action_label: carry_memory
    // ghostlight.branch_label: prioritize_living_carrier
    {tender_boundary >= 2 && warning_clarity >= 2 && contamination_risk <= 2:
        The tender releases the roots one limb at a time and keeps the borrowed balance alive in her body.
        -> ending_living_carrier_success
    - else:
        The tender steps away before her body can decide which memories belong to it.
        -> ending_living_carrier_cost
    }

=== ending_full_recall_success ===
// ghostlight.ending_label: full_recall_success
// ghostlight.training_hook: multispecies_corroboration_opens_archive
The matriarch spends light accumulated across seasons.

The whole collapse passes through the gallery: infant weight against lower hands, the first false groan, the third true one, threadwing panic above the dust, fungal darkness under the surviving upper bridge. The imperial cadence finds no unwitnessed gap large enough to own.

The route witness leaves uphill with a legally invocable memory and the exact sequence needed to clear the lower path.

{route_light <= 1: Below them, blue lantern knots remain dark. The evacuation succeeds by touch, shouted count, and root tremor; pollinators lose a feeding window to pay for it.}
{route_light > 1: A thin blue lane remains lit long enough for bodies and couriers to climb away from the failing bridge.}

The tender keeps one borrowed sensation until morning: an ancestor's lower hands shielding a child, and the matriarch's roots choosing to remember both.
-> END

=== ending_full_recall_cost ===
// ghostlight.ending_label: full_recall_cost
// ghostlight.training_hook: archive_opened_beyond_witness_support
The root cups empty. The archive membrane opens.

What emerges is complete in the way a wound can be complete: old dust, present fear, route-song, and imperial cadence fused into one persuasive body. The lower bridge is condemned with perfect certainty. So is an upper path the old collapse never touched.

The matriarch tears the shallow tissue shut. A season of stored sugar has become an answer nobody can trust, and the false rhythm now knows the taste of this gallery.

The route witness evacuates both paths. That saves lives and abandons nests, fungal beacons, and a pollinator generation to the slide.
-> END

=== ending_bounded_warning_success ===
// ghostlight.ending_label: bounded_warning_success
// ghostlight.training_hook: uncertainty_preserved_as_law
The aperture seals around the contaminated remainder.

The warning goes out as a narrow fact: after the third root groan, leave the lower bridge. The fungal road repeats it in bitter beads. The matriarch knocks it through load roots. The route witness carries it in spoken count.

Nobody claims to know the whole ancestor. That restraint costs the community a precedent, but preserves the difference between memory and command.

The third groan travels through the hill.

The lower route is empty when the stone comes down.
-> END

=== ending_bounded_warning_cost ===
// ghostlight.ending_label: bounded_warning_cost
// ghostlight.training_hook: warning_without_standing
The tender closes the archive and sends the warning anyway.

Some bodies obey the Airawa voice. The candle road does not repeat a claim it cannot account for. Lantern trees light both exits. Threadwings scatter the message in rival orders.

When the third groan comes, the lower path is thinner but not empty. The route witness turns back for the ones who trusted a different organ.

The archive remains clean enough to ask later. The hillside is less patient.
-> END

=== ending_tree_authority_success ===
// ghostlight.ending_label: tree_authority_success
// ghostlight.training_hook: sacred_deference_as_material_authority
The gallery goes dark except for one blue line.

The matriarch withdraws light from the lower ledge and pushes stored sugar into an uphill chain of lantern trees. Candle beads open along that route. Threadwings follow the electrical gradient before any Airawa gives an order.

The tree has refused the requested memory and still committed its own body to the evacuation. Its answer cannot settle the old law. It can keep the present alive.

The tender bows because gratitude and submission share a posture here. The distinction will require witnesses later.
-> END

=== ending_tree_authority_cost ===
// ghostlight.ending_label: tree_authority_cost
// ghostlight.training_hook: refusal_exposes_hostage_memory
The tender clears the tokens. The matriarch clears the room.

Archive tissue hardens. Root ridges lift under Airawa feet and move every petitioner toward the outer ledge. No route lights change. No remembered sequence arrives.

The tree has judged the open channel more dangerous than the slope. It may be right. It has also left the community to spend bodies proving it.

The route witness runs uphill shouting a warning stripped of ancestral standing. Behind them, Shale-Root keeps its memory and its silence.
-> END

=== ending_living_carrier_success ===
// ghostlight.ending_label: living_carrier_success
// ghostlight.training_hook: person_as_temporary_archive_organ
The tender climbs onto the outer ledge with the old reflex distributed across six limbs.

At each fork she lets the remembered body lean. One route makes her upper claws seize with the ancestor's terror. The other lets all four taloned toes settle against living root. She marks the safer climb while the route witness repeats every choice aloud.

The matriarch seals behind them. The archive has not released a copy. It has loaned the community one moving organ.

By eclipse egress the lower path is clear. For several hours afterward, the tender still reaches with a dead person's lower hand whenever stone cracks.
-> END

=== ending_living_carrier_cost ===
// ghostlight.ending_label: living_carrier_cost
// ghostlight.training_hook: memory_bleed_under_weak_boundary
The tender leaves the roots carrying two gaits.

Her own taloned feet know the gallery. The ancestor's feet know a vanished ledge. The imperial cadence knows only repetition. All three insist on steering.

The route witness catches her with both upper arms before she steps into eclipse-dark air. The warning dies in their throat as the candle road closes a quarantine ring around them.

The tree keeps the deep archive sealed. The tender becomes the dangerous open channel everyone must now protect, question, and perhaps refuse.
-> END
