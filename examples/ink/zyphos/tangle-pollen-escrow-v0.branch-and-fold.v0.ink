// ghostlight.artifact_id: tangle_pollen_escrow_branch_fold_v0
// ghostlight.fixture_id: tangle-pollen-escrow-v0
// ghostlight.scene_id: tangle-pollen-escrow-v0.umbros-saddle-handoff
// ghostlight.final_ink_path: examples/ink/zyphos/tangle-pollen-escrow-v0.branch-and-fold.v0.ink

VAR courier_trust = 2
VAR road_credit = 2
VAR lantern_credit = 2
VAR route_witness = 0
VAR cargo_integrity = 3
VAR gestation_time = 3
VAR matriarch_sugar = 3
VAR rival_trust = 1
VAR public_breach_record = 0
VAR matriarch_leverage = 2

-> start

=== start ===
// ghostlight.scene: umbros_saddle_establishing
// ghostlight.visual_scene: tangle_saddle_establishing
Umbros hangs fixed above the saddle, large enough to make the sky look occupied.

Below it, two Matriarch root territories meet at a pollination-escrow station. The receiving Matriarch's roots rise on the west slope around a sealed gestation hollow. A lantern tree leans over the central handoff roost, its cold knots lighting the last crossing before eclipse. An amber-beaded candle fungal road climbs from the eastern fen and ends at the same roost.

No single body owns the route. This is the principle everyone praises when the route is working.

-> ordinary_work

=== ordinary_work ===
// ghostlight.scene: ordinary_escrow_work
// ghostlight.visual_scene: tangle_ordinary_work
The saddle pollen clerk hangs from the lantern tree with both clawed upper arms and both hooked feet. The smaller lower hands sort sealed cargo sleeves, comb shed threadwing fibers into witness cords, and pinch dead candle beads from the road.

Airawa anatomy is excellent for public service. A clerk can be overcommitted in four directions and still have two hands free for paperwork.

The red-vane escrow courier waits above the roost. Its long sensory ribbons taste pressure and residue while a hungry clasping parasite works along one vane. Beneath the clerk, the fungal road raises three amber fruiting beads: fed, listening, and prepared to become offended in the correct order. The lantern tree pulses a thin blue tax-light over a resin cup of Matriarch sugar.

The next cross-grove gestation cargo is due before eclipse ingress. There is time to service one relationship properly.

-> service_choice

=== service_choice ===
// ghostlight.choice_layer: routine_constituency_service
+ [Brace by the upper arms and feet, then use both lower hands to groom the parasite from the courier's red vane.]
    // ghostlight.branch: service_courier
    // ghostlight.action: preen_symbiont
    // ghostlight.intent: Invest routine labor in courier trust before the reproductive crossing.
    ~ courier_trust = courier_trust + 2
    The courier lowers the red ribbon by one careful width. The clerk pins the parasite with blunt lower digits, peels it free, and offers it back as food.

    The courier eats the complaint and leaves one clean vane fiber in the clerk's palm. Among threadwings, this is not affection. It is a receipt that might become affection if nobody embarrasses it.
    // ghostlight.npc_response: The courier accepts grooming and deposits a readable witness fiber.
    // ghostlight.consequence: courier_trust rises, enabling voluntary route testimony under pressure.
    // ghostlight.training_hook: routine_care_as_political_credit
    -> routine_fold
+ [Climb down and press the clean grazer-dung and mineral ration into the road's three waiting beads.]
    // ghostlight.branch: service_road
    // ghostlight.action: feed_route
    // ghostlight.intent: Keep the fungal route solvent enough to request a costly resampling later.
    ~ road_credit = road_credit + 2
    The clerk breaks the ration into three equal pieces. The road absorbs two immediately and leaves the third visible until the clerk looks at it.

    "Yes," the clerk says. "You have reserves. I have eyes. Civilization survives another audit."

    The third piece sinks. Amber runs downhill through the mycelial braid like laughter with excellent bookkeeping.
    // ghostlight.npc_response: The road accepts the clean ration and displays surplus before consuming it.
    // ghostlight.consequence: road_credit rises, enabling an independent immune-trace resample.
    // ghostlight.training_hook: nourishment_as_route_credit
    -> routine_fold
+ [Unseal one sugar pellet and rebind the lantern tree's dim route knot before it can levy the repair itself.]
    // ghostlight.branch: service_lantern
    // ghostlight.action: repair_light_contract
    // ghostlight.intent: Buy reliable eclipse light by spending the receiving Matriarch's sugar reserve.
    ~ lantern_credit = lantern_credit + 2
    ~ matriarch_sugar = matriarch_sugar - 1
    The clerk braces with taloned feet, frees both lower hands, and presses a pearl of root sugar into the split light-knot. Fine blue filaments close around it.

    The lantern tree brightens the eastern approach and dims the clerk's own face. Route before vanity. The tree approves of moral lessons that invoice someone else.
    // ghostlight.npc_response: The lantern tree restores the eastern approach and records the clerk's repair credit.
    // ghostlight.consequence: lantern_credit rises while the Matriarch sugar reserve falls.
    // ghostlight.training_hook: light_tax_and_resource_cost
    -> routine_fold

=== routine_fold ===
// ghostlight.fold: routine_credit_into_arrival
// ghostlight.visual_scene: tangle_routine_fold
The station settles back into work.

{courier_trust >= 4: The red-vane courier keeps its clean witness fiber visible between its gripping feet.}
{road_credit >= 4: The fungal road opens a pale sampling cup beside the handoff stone, a service it does not offer to debtors.}
{lantern_credit >= 4: The lantern knots hold an unusually steady blue corridor toward the eastern ridge.}
{matriarch_sugar <= 2: The receiving Matriarch tightens a root around the sugar cup. Generosity has reached the stage where its owner notices.}

Then the rival archive nurse appears on the east root bridge, climbing fast with four limbs and carrying a translucent gestation sleeve in both lower hands. A second threadwing glides behind, vanes tight after the cold crossing.

-> arrival

=== arrival ===
// ghostlight.scene: rival_cargo_arrival
// ghostlight.visual_scene: tangle_rival_arrival
The nurse anchors at the roost, upper claws biting bark, feet locked around the bridge ridge. The cargo sleeve between the lower hands holds compatible Airawa gametes in pale gel, wrapped in route seals from the rival Matriarch.

"Cross-grove succession cargo," the nurse says. "Agreed window. Agreed witnesses. One household on my slope has already begun pretending not to worry."

The red-vane escrow courier performs the ordinary welcome assay, brushing its longest sensory ribbon across the outer route seal. The lantern tree tastes the transferred residue and extinguishes the eastern half of the roost.

The candle road raises a bitter white quarantine ring around the nurse's feet.

-> accusation

=== accusation ===
// ghostlight.scene: escrow_quarantine
// ghostlight.visual_scene: tangle_quarantine_ring
The receiving Matriarch closes the shallow folds of its gestation hollow. Its root pulse reaches the clerk through the branch: foreign immune distortion; delay admission; collect remedy.

The rival nurse looks at the darkened lanterns, the white road ring, and the sealed hollow.

"Our child," the nurse says, "has acquired a scheduling problem with your Matriarch's opinions."

The eclipse edge is already eating the fen's reflected light. If the final crossing misses the lantern corridor, the cargo must spend another cycle cooling in a roost that was designed for handoff, not custody.

The clerk knows only what the station makes public: the route seals look intact; the road claims distortion; the lantern tree has refused light; the courier has not yet testified.

-> evidence_choice

=== evidence_choice ===
// ghostlight.choice_layer: trace_dispute
+ {courier_trust >= 4} [Ask the red-vane courier to shed its clean witness fiber into the public braid.]
    // ghostlight.branch: take_courier_testimony
    // ghostlight.action: request_testimony
    // ghostlight.intent: Let the courier expose route memory at the cost of time and public entanglement.
    ~ route_witness = route_witness + 2
    ~ public_breach_record = public_breach_record + 1
    ~ rival_trust = rival_trust + 1
    ~ gestation_time = gestation_time - 1
    The clerk opens a lower hand. The courier lands, places the clean fiber across the public braid, and drags its red vane through the lantern tree's outer pollen cup.

    Light runs backward along the fiber. The last nectar stop recorded there was not on the rival slope. It was the receiving Matriarch's own outer root cup.

    The courier clicks once at the sealed cargo. Then twice at the western roots. It does not enjoy being used as a weapon whose handle thought itself invisible.
    // ghostlight.npc_response: The courier volunteers a route-memory fiber implicating the receiving side's nectar stop.
    // ghostlight.consequence: Strong route witness and public record gained; time is spent; rival trust improves.
    // ghostlight.training_hook: pollinator_testimony_against_patron
    -> evidence_fold
+ {road_credit >= 4} [Step into the quarantine ring, cut only the outer sleeve, and offer the road a second sample.]
    // ghostlight.branch: buy_road_resample
    // ghostlight.action: expose_cargo_surface
    // ghostlight.intent: Spend road credit and some cargo protection for an independent trace reading.
    ~ road_credit = road_credit - 1
    ~ cargo_integrity = cargo_integrity - 1
    ~ route_witness = route_witness + 2
    ~ public_breach_record = public_breach_record + 1
    ~ rival_trust = rival_trust + 1
    The clerk enters the white ring. It climbs over the feet like cold foam but stops below the lower hands. The nurse turns the sleeve outward for sampling and does not release it.

    One careful cut opens the travel wrapper while leaving the inner gel sealed. The road drinks a bead of condensation, darkens, then grows a line of amber fruiting bodies westward toward the receiving Matriarch's sugar cup.

    Not infection. A route-marking resin in the western nectar. An opinion with sugar in it.
    // ghostlight.npc_response: The road accepts payment, retests the cargo, and points the residue trail west.
    // ghostlight.consequence: Strong route witness and public record gained; road credit and cargo integrity are spent.
    // ghostlight.training_hook: decomposer_as_forensic_constituency
    -> evidence_fold
+ {lantern_credit >= 4} [Call in the repair credit: keep the crossing lit while the lantern tree replays the disputed route pulse.]
    // ghostlight.branch: spend_lantern_credit
    // ghostlight.action: invoke_light_credit
    // ghostlight.intent: Preserve the gestation window while forcing the light authority to reveal what it sensed.
    ~ lantern_credit = lantern_credit - 1
    ~ matriarch_sugar = matriarch_sugar - 1
    ~ route_witness = route_witness + 1
    ~ public_breach_record = public_breach_record + 1
    ~ gestation_time = gestation_time + 1
    The clerk presses the repaired knot with both lower palms.

    The lantern tree keeps the eastern corridor blue, then replays the arrival pulse in bands of cold light: clean rival seal, clean courier body, western nectar residue blooming only after the final stop.

    The tree does not accuse the receiving Matriarch. It merely illuminates the sequence so thoroughly that accusation becomes unpaid labor.
    // ghostlight.npc_response: The lantern tree honors repair credit, preserves light, and displays the residue sequence without assigning intent.
    // ghostlight.consequence: Time is preserved and a partial public witness gained; lantern credit and Matriarch sugar are spent.
    // ghostlight.training_hook: infrastructure_truth_without_human_speech
    -> evidence_fold
+ [Repeat the quarantine finding exactly and demand a remedy from the rival archive before the hollow reopens.]
    // ghostlight.branch: uphold_quarantine
    // ghostlight.action: enforce_procedure
    // ghostlight.intent: Preserve the receiving Matriarch's leverage by treating the first reading as final.
    ~ matriarch_leverage = matriarch_leverage + 2
    ~ rival_trust = rival_trust - 1
    ~ gestation_time = gestation_time - 2
    The clerk repeats the root pulse word for word: foreign distortion; delayed admission; remedy owed.

    The rival nurse goes still. The courier lifts away from the roost. The fungal road leaves its white ring in place and adds one amber bead outside it, pointed at the clerk.

    Procedure is useful this way. It lets a decision claim it arrived by itself.
    // ghostlight.npc_response: The nurse withdraws trust while the route parties mark the clerk as part of the disputed decision.
    // ghostlight.consequence: Matriarch leverage rises; rival trust and gestation time fall; no independent witness is produced.
    // ghostlight.training_hook: archive_authority_hiding_inside_procedure
    -> evidence_fold

=== evidence_fold ===
// ghostlight.fold: disputed_trace_into_final_bargain
// ghostlight.visual_scene: tangle_evidence_fold
The eclipse advances. Cold lantern light, white quarantine tissue, and amber road beads divide the roost into three jurisdictions that happen to occupy the same few lengths of bark.

{route_witness >= 2:
The public braid now points west. The cargo arrived clean; the suspicious residue came from the receiving Matriarch's nectar stop. The delay was not an immune accident. It was a bargaining move aimed at a rival lineage.
- else:
    {route_witness == 1:
The lantern replay shows the residue appeared after the final western stop, but nobody has supplied a second witness strong enough to make intent public.
    - else:
The first quarantine reading remains the only admissible account. That makes it powerful. It does not make it true.
    }
}

{cargo_integrity <= 2: Condensation pearls on the exposed outer sleeve. The inner gel remains sealed, but there is less protection left for another delay.}
{gestation_time <= 1: The sleeve's pale warmth has begun to thin. Another full argument will decide the route by decay.}
{gestation_time >= 4: The repaired lantern corridor has bought one clean interval in which choice can still pretend to be deliberation.}
{rival_trust >= 2: The rival nurse stands beside the clerk rather than across the quarantine ring.}
{rival_trust <= 0: The rival nurse has shifted the cargo toward the east bridge, ready to leave before refusal becomes confiscation.}
{public_breach_record >= 1: The fungal beads and lantern bands have begun storing the dispute in forms other travelers can read.}
{matriarch_leverage >= 4: The western roots pulse with the confidence of an archive that believes delay is already ownership.}

The clerk can still decide which relationship pays for passage.

-> final_choice

=== final_choice ===
// ghostlight.choice_layer: succession_bargain
+ [Invoke the minimum-traffic promise and carry the cargo across the quarantine ring in public.]
    // ghostlight.branch: enforce_minimum_traffic
    // ghostlight.action: carry_cargo
    // ghostlight.intent: Make the route coalition enforce the reproductive floor against the receiving Matriarch.
    ~ public_breach_record = public_breach_record + 1
    ~ matriarch_leverage = matriarch_leverage - 1
    ~ rival_trust = rival_trust + 1
    {route_witness >= 2 && gestation_time >= 2:
        -> ending_floor_holds
    - else:
        -> ending_floor_breaks
    }
+ [Offer two measures of western root sugar for immediate passage and keep the cause out of the public braid.]
    // ghostlight.branch: purchase_private_passage
    // ghostlight.action: transfer_resource
    // ghostlight.intent: Save the cargo through a private ecological payment while protecting the Matriarch's public standing.
    ~ matriarch_sugar = matriarch_sugar - 2
    ~ rival_trust = rival_trust + 1
    {matriarch_sugar >= 0 && cargo_integrity >= 2:
        -> ending_private_bargain_holds
    - else:
        -> ending_private_bargain_cost
    }
+ [Leave the quarantine ring closed and demand the rival Matriarch surrender next season's courier claim.]
    // ghostlight.branch: hold_embargo
    // ghostlight.action: withhold_access
    // ghostlight.intent: Convert the delay into direct leverage over the rival archive's reproductive route.
    ~ matriarch_leverage = matriarch_leverage + 1
    ~ rival_trust = rival_trust - 1
    {matriarch_leverage >= 4:
        -> ending_embargo_holds
    - else:
        -> ending_embargo_breaks
    }

=== ending_floor_holds ===
// ghostlight.ending_label: minimum_traffic_success
// ghostlight.visual_scene: tangle_ending_floor
// ghostlight.training_hook: route_coalition_checks_archive_power
The clerk takes the sleeve in both lower hands, anchors upper claws and feet across the white ring, and climbs.

The red-vane courier lays its witness fiber over the clerk's wrist. The fungal road turns the ring from white to amber. The lantern tree spends its stored sugar on one hard blue corridor. Three authorities make the same refusal in three incompatible languages.

The receiving Matriarch opens the gestation hollow.

{public_breach_record >= 2: Every fruiting bead on the eastern descent carries the breach. The Matriarch keeps the birth and loses the ability to describe the delay as private housekeeping.}
{rival_trust >= 3: The rival nurse helps guide the sleeve into the living folds, shoulder plates almost touching the clerk's. This is not forgiveness. It is a constituency becoming visible.}

The cargo enters warm. The household on the rival slope will receive a child already claimed by several arguments and owned by none of them completely.

The clerk reseals the sugar cup. It is lighter. So is the Matriarch's certainty.
-> END

=== ending_floor_breaks ===
// ghostlight.ending_label: minimum_traffic_cost
// ghostlight.visual_scene: tangle_ending_floor
// ghostlight.training_hook: principle_without_sufficient_witness
The clerk lifts the sleeve across the ring.

The road stays white. The lantern corridor holds for one pulse, then gutters. The courier circles but does not land. A minimum promise with no shared witness is a sentence spoken to roots that have already decided they are the court.

{gestation_time <= 1: The sleeve cools in the clerk's lower hands. The rival nurse takes it back before principle becomes spoilage.}
{gestation_time >= 2: The cargo remains viable, but the nurse must carry it east and spend another cycle finding a route that has not been made into an argument.}

The receiving Matriarch keeps the hollow shut. The public braid records an attempted crossing and no coalition strong enough to finish it.
-> END

=== ending_private_bargain_holds ===
// ghostlight.ending_label: private_sugar_success
// ghostlight.visual_scene: tangle_ending_private
// ghostlight.training_hook: birth_saved_reputation_preserved
Two measures of western root sugar dissolve into the fungal road. The lantern tree takes light-tax. The courier takes salt and a clean roost promise. The white ring thins to amber.

The receiving Matriarch opens the hollow without admitting error. The rival nurse guides the sleeve inside.

{public_breach_record >= 1: The clerk unbraids the freshest witness cord before it can travel. The route bodies remember the facts locally; the larger public record stays incomplete.}
{rival_trust >= 2: The nurse says, "The child will live. Your Matriarch will call that generosity." The clerk has no efficient answer.}

The birth route survives. So does the power to threaten it again.
-> END

=== ending_private_bargain_cost ===
// ghostlight.ending_label: private_sugar_cost
// ghostlight.visual_scene: tangle_ending_private
// ghostlight.training_hook: exhausted_ecology_cannot_buy_secrecy
The clerk opens the western sugar cup and finds the politics have already eaten most of lunch.

The road takes one measure and remains hungry. The lantern knots take another and dim before the sleeve reaches the ring. If the outer wrapper was cut, condensation now shines along the seal like a second deadline.

The rival nurse pulls the cargo east. The receiving Matriarch has preserved its innocence by failing to purchase the result it wanted hidden.

Below the roost, candle beads go dark one by one. An unpaid ecology is not neutral infrastructure. It is a future alliance being offered elsewhere.
-> END

=== ending_embargo_holds ===
// ghostlight.ending_label: embargo_leverage_success
// ghostlight.visual_scene: tangle_ending_embargo
// ghostlight.training_hook: one_generation_win_food_web_cost
The clerk names the remedy: the rival Matriarch must surrender first claim on next season's courier corridor.

The rival nurse carries the sleeve back toward the east bridge. The receiving roots pulse satisfaction. A politically awkward lineage will miss this hollow, and the rival archive will have to bargain from a narrower future.

{route_witness >= 1: The red-vane courier tears its witness fiber from the public braid and takes it east.}
{road_credit >= 3: The fungal road keeps the clerk's old credit but darkens the western feeder strand. Accounts can remain accurate while alliances change.}
{lantern_credit >= 3: The lantern tree leaves one knot lit for the departing cargo and none for the clerk. Minimum traffic survives as an insult.}

The Matriarch has won the generation. Threadwing nests, road fruiting bodies, lantern saplings, and waiting Airawa households have all learned what that victory costs them.
-> END

=== ending_embargo_breaks ===
// ghostlight.ending_label: embargo_leverage_backfire
// ghostlight.visual_scene: tangle_ending_embargo
// ghostlight.training_hook: unsupported_embargo_realigns_constituencies
The clerk demands next season's courier claim.

Nothing obeys.

The red-vane courier lands beside the rival nurse. The candle road opens a thin amber route back down the east slope. The lantern tree lights that retreat and leaves the western hollow dark.

{rival_trust <= 0: The nurse does not answer the clerk. Silence is cheaper than teaching an opponent which part of the threat failed.}
{rival_trust >= 1: The nurse says, "Your archive has mistaken our dependency for its property."}

The cargo leaves. By next eclipse, Airawa tenders from the western slope are asking which neighboring Matriarch still keeps a birth route open. The receiving tree retains its memory, its roots, and its authority over an increasingly theoretical constituency.
-> END
