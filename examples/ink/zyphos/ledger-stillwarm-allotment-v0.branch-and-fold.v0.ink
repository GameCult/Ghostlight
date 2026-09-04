// ghostlight.artifact_id: ledger_stillwarm_allotment_branch_fold_v0
// ghostlight.fixture_id: ledger-stillwarm-allotment-v0
// ghostlight.scene_id: ledger-stillwarm-allotment-v0.heat-pledge-shortfall
// ghostlight.final_ink_path: examples/ink/zyphos/ledger-stillwarm-allotment-v0.branch-and-fold.v0.ink

VAR heat_reserve = 2
VAR herd_trust = 2
VAR road_capacity = 2
VAR ground_credit = 2
VAR caretaker_strain = 1
VAR graft_departure = 2
VAR repair_evidence = 1
VAR cradle_efficiency = 2
VAR eclipse_margin = 3

-> start

=== start ===
Stillwarm Shelf is a breeding ground built on a dark stone terrace below the fixed face of Umbros. Its name is provisional. The infants have nevertheless been using it with confidence.

Three nursery ramps descend from the inner edge into communal hollows. Cold blue lantern knots hang above them for the daily eclipse. At the terrace's outer edge, glassbacks climb a short broad path from the prismwake wetland and kneel in open heat bays while flexible root-and-stone ribs settle over their translucent dorsal plates. The ribs carry stored warmth inward. Between both edges, amber candle fungi mark a waste lane that carries dung, spoiled food, and failed grafts away from the cradles.

Ruun, today's heat steward, folds four tall running legs under a long smoke-gray body at the low allotment shelf. Two smaller chest hands sort mineral brushes, route cords, and three warm stones representing the cradles. It is a serious office. This is why someone has chewed one stone into the shape of an elder's head.

-> introduce_claimants

=== introduce_claimants ===
Vara waits beside the routeward bay. Rust-colored body fibers show through road dust; a balanced flank frame carries sealed medical graft packets meant for the next breeding ground. Vara's family damaged a prismwake mat during a flood crossing and has poor standing with the wetland below. They also carried an infant out of that flood. The continent, displaying its usual administrative tenderness, remembers both facts.

Two other family units tighten pack cords along the routeward edge. Either could carry Vara's graft frame if the packets remain viable and someone accepts the extra weight before eclipse closes the departure window.

Wide-Blue leads the arriving glassback herd. The broad grazer's dorsal plates hold bands of blue and honey warmth from returned light. The herd needs the wetland's sugar-rich mats, protected calving lanes here, mineral washing, and parasite work. The breeding ground needs the heat stored in those plates.

Ruun's task is ordinary: receive the pledge, keep the cradles clean, and send Vara away before eclipse with the grafts still warm enough to survive the next route.

-> routine_choice

=== routine_choice ===
// ghostlight.choice_layer: morning_heat_work
+ [Brush mineral grit from Wide-Blue's plates before the heat transfer.]
    // ghostlight.action_label: groom
    // ghostlight.branch: prime_herd_trust
    // ghostlight.branch_label: prime_herd_trust
    ~ herd_trust = herd_trust + 2
    ~ caretaker_strain = caretaker_strain + 1
    ~ eclipse_margin = eclipse_margin - 1
    Ruun takes a broad mineral brush in both chest hands and works between the warm glass plates. Wide-Blue lowers until the flexible collection ribs touch the clean plate edges.

    One plate clears from cloudy violet to deep blue. The herd shifts closer instead of away.

    Vara watches Ruun clean the animal that is about to become public heating. "Does the office include biting?"

    "Only appeals," Ruun says, prying loose a stubborn crust.
    -> routine_fold
+ [Feed the candle road a sorted basket of dung, spoiled food, and one clean failed graft.]
    // ghostlight.action_label: transfer_object
    // ghostlight.branch: prime_road_capacity
    // ghostlight.branch_label: prime_road_capacity
    ~ road_capacity = road_capacity + 2
    ~ cradle_efficiency = cradle_efficiency + 1
    Ruun separates clean dead tissue from bitter spoilage and tips each portion between different amber candles. The road draws both downward through braided pale strands, keeping the nursery roots clear of rot.

    A fresh line of beads opens beside the infirm cradle. Waste capacity, translated into light.

    "It likes your sorting," Vara says.

    "It likes categories that cannot infect one another. A common professional weakness."
    -> routine_fold
+ [Warm and reseal Vara's graft frame before the route window.]
    // ghostlight.action_label: use_object
    // ghostlight.branch: prime_graft_departure
    // ghostlight.branch_label: prime_graft_departure
    ~ graft_departure = graft_departure + 2
    ~ heat_reserve = heat_reserve - 1
    Ruun slides Vara's lowest graft tray beneath the adult work shelf. Heat creeps through the breathable leaf-skin wraps. Two chest hands retie the pressure cords while Vara braces the balanced flank frame.

    The packets settle into an even amber sheen. Somewhere beyond the next ridge, patients become more likely to receive them alive.

    One of the three cradle stones on Ruun's shelf cools from gold to gray.
    -> routine_fold
+ [Stake breeding-ground credit on an early request for prismwake sugar.]
    // ghostlight.action_label: authorize
    // ghostlight.branch: prime_ground_credit
    // ghostlight.branch_label: prime_ground_credit
    ~ ground_credit = ground_credit - 1
    ~ heat_reserve = heat_reserve + 1
    ~ repair_evidence = repair_evidence + 1
    Ruun knots a mineral cord around the road's request branch: one clean sugar opening for the herd, charged to Stillwarm Shelf.

    Far below, a prismwake mat answers in a pale green sheet. The road sends up a line of amber beads. Wide-Blue's plates brighten at the promise of feed.

    The commons has bought warmth with reputation it may need later. There is no shelf for that stone, so Ruun keeps it in the stomach.
    -> routine_fold

=== routine_fold ===
// ghostlight.fold: routine_before_shortfall
Work settles into the terrace's old circuit: mat sugar into glassback bodies, stored light into cradle roots, waste into fungal digestion, medical craft back out along the road.

{herd_trust >= 4: Wide-Blue kneels willingly under the first collection rib, plates open and clear.}
{road_capacity >= 4: Amber candles keep the waste lane bright and the cradle roots dry.}
{graft_departure >= 4: Vara's graft packets glow evenly inside their leaf-skin wraps, ready for the next route.}
{heat_reserve <= 1: One cradle stone has already gone gray. Routine generosity has spent part of the eclipse reserve.}
{ground_credit <= 1: The prismwake request cord hangs at the road edge, a public promise tied in mineral fiber.}
{caretaker_strain >= 2: Ruun's chest hands shake once after the brushing and become very interested in being still.}

Then Wide-Blue rises before the first cradle finishes charging.

-> shortfall_reveal

=== shortfall_reveal ===
The herd's plates fog dark from front to rear. Warmth remains inside them, visible and withheld.

Below the terrace, the prismwake wetland flashes a long silver scar. Vara's family crossed that regrowth during the flood. The damaged mat has reduced sugar access; the glassbacks will not spend full heat for a breeding ground that cannot keep its feeding promise.

The candle road raises bitter beads around the request branch. Its report carries torn mat tissue, Vara's foot-pressure, and the clean immune trace of the infant they carried. Cause has arrived. Blame is still negotiating transport.

Ruun turns the three cradle stones. At the present transfer, only two will remain warm and clean through totality: the infant hollow, the infirm rest, or the adult work shelf where caretakers keep grafts and their own bodies functional.

Vara looks from the gray stone to the routeward path. "Say it plainly. If the shortage follows my family, so does the bill."

Wide-Blue turns broadside. Blue light pulses once under the fogged plates. The herd is listening. So is everything else.

-> shortfall_choice

=== shortfall_choice ===
// ghostlight.choice_layer: account_for_shortfall
+ [Ask Vara to lay the flood route and the rescued infant's trace on the public shelf.]
    // ghostlight.action_label: show_object
    // ghostlight.branch: hear_vara_testimony
    // ghostlight.branch_label: hear_vara_testimony
    ~ repair_evidence = repair_evidence + 2
    ~ eclipse_margin = eclipse_margin - 1
    Vara opens a flexible route membrane across the low shelf. Pressure marks show the family entering the wetland in rising water, crossing the mat scar, and leaving with one additional infant weight between their bodies.

    The record does not erase the tear. It makes the cost harder to turn into a personality defect.

    Wide-Blue lowers the head to taste the membrane. The candle road opens one narrow sampling bead and no passage.
    -> shortfall_fold
+ [Offer the herd the commons' remaining mineral wash and a full parasite watch.]
    // ghostlight.action_label: transfer_object
    // ghostlight.branch: offer_herd_care
    // ghostlight.branch_label: offer_herd_care
    ~ herd_trust = herd_trust + 1
    ~ ground_credit = ground_credit - 1
    ~ heat_reserve = heat_reserve + 1
    Ruun pushes the mineral basin into Wide-Blue's reach and lays two grooming cords across the shelf: payment now, work owed later.

    Wide-Blue tastes the wash. One rear plate clears and touches the collection rib. A thin band of warmth moves inward.

    The herd has not forgiven the feeding loss. It has accepted that the breeding ground can still make itself useful.
    -> shortfall_fold
+ [Close the adult work shelf and nest all dependent bodies around the two inner cradles.]
    // ghostlight.action_label: move
    // ghostlight.branch: compress_cradles
    // ghostlight.branch_label: compress_cradles
    ~ cradle_efficiency = cradle_efficiency + 2
    ~ heat_reserve = heat_reserve + 1
    ~ caretaker_strain = caretaker_strain + 1
    Ruun rolls the gray work stone into the waste lane and signals down the nursery ramps. Infants and infirm adults fold closer around the two inward cradles. Able-bodied caretakers carry tools into the cold outer light.

    The system becomes more efficient by deciding whose comfort counts as expendable labor input. Machinery has invented that trick before. It remains pleased with itself.
    -> shortfall_fold
+ [Wait for the candle road to separate torn-mat residue from flood-borne sickness.]
    // ghostlight.action_label: wait
    // ghostlight.branch: wait_for_road_sample
    // ghostlight.branch_label: wait_for_road_sample
    ~ road_capacity = road_capacity + 1
    ~ repair_evidence = repair_evidence + 1
    ~ eclipse_margin = eclipse_margin - 1
    ~ caretaker_strain = caretaker_strain + 1
    Ruun holds Vara beyond the waste lane while the fungal body samples dust from the flank frame, shed fiber, and the route membrane's damp edge.

    Amber beads trace two histories. One line carries the prismwake wound. The other carries flood microbes already rejected by Vara's healthy tissue.

    Waiting has produced a cleaner account. Umbros has meanwhile taken another bite from the sun, an accounting practice with tremendous institutional confidence.
    -> shortfall_fold

=== shortfall_fold ===
// ghostlight.fold: shortage_becomes_obligation
The terrace holds its argument in visible bodies.

{repair_evidence >= 3: The route membrane and fungal beads distinguish necessary rescue from the repair still owed to the injured mat.}
{repair_evidence <= 1: The silver scar below and Vara's poor standing have begun to impersonate a complete explanation.}
{herd_trust >= 4: Wide-Blue keeps one clear plate against the collection rib, a narrow offer still open.}
{ground_credit <= 0: Both mineral cords on the request branch have gone dark. Stillwarm Shelf has no unpledged standing left to spend this eclipse.}
{cradle_efficiency >= 4: The two inner cradles hold dependents flank-close while tools and able-bodied workers move into the cold outer court.}
{caretaker_strain >= 3: Ruun's bare throat patch shows a dark band of fatigue visible from every ramp.}
{eclipse_margin <= 1: The lantern knots wake blue along the nursery ramps. Decision time has become the next scarce material.}

Vara's graft frame is ready or it is not. The herd is willing or it is not. The road can carry another promise or it cannot. The infants will be warmed regardless; the question is which future pays for it.

-> final_threshold

=== final_threshold ===
Totality covers the terrace.

Umbros hangs fixed above Stillwarm Shelf, a black world ringed by the dim primary. Cold lantern light marks three nursery ramps. Amber fungal beads divide the waste lane from the heat bays. Wide-Blue waits under a flexible collection rib with warmth still banked in fogged plates. Vara stands beside the routeward exit, graft frame balanced over four running legs.

{heat_reserve >= 3: All three cradle stones hold some gold. The shortage can be moved rather than merely endured.}
{heat_reserve <= 1: Only the two inward stones retain gold at their centers.}
{graft_departure >= 4: The medical packets are route-ready, their leaf-skin seams evenly warm.}
{graft_departure <= 2: The medical packets show uneven amber edges; delay will cost graft viability.}
{road_capacity >= 3: The road keeps a bright branch open toward the wounded wetland.}
{road_capacity <= 2: The fungal waste lane is working at its safe edge and refuses another easy promise.}
{cradle_efficiency >= 4: The gray adult work stone remains beside the waste lane, where Ruun rolled it when the dependents nested inward.}
{cradle_efficiency < 4: All three cradle stones remain on the allotment shelf.}

No choice creates more sun. Ruun can bind a family to repair, spend the commons' future standing, renegotiate with the herd, or put the deficit into caretaker bodies.

-> final_choice

=== final_choice ===
// ghostlight.choice_layer: assign_the_cost
+ [Bind Vara's family to mat repair and transfer the grafts to an eastbound unit.]
    // ghostlight.action_label: authorize
    // ghostlight.branch: assign_mobility_debt
    // ghostlight.branch_label: assign_mobility_debt
    {repair_evidence >= 3 && graft_departure >= 2:
        -> ending_mobility_success
    - else:
        -> ending_mobility_cost
    }
+ [Spend Stillwarm Shelf's remaining ecological credit on sugar access and road transport.]
    // ghostlight.action_label: spend_resource
    // ghostlight.branch: spend_ground_credit
    // ghostlight.branch_label: spend_ground_credit
    {ground_credit >= 1 && road_capacity >= 3:
        ~ ground_credit = ground_credit - 1
        ~ heat_reserve = heat_reserve + 1
        -> ending_credit_success
    - else:
        ~ ground_credit = ground_credit - 1
        -> ending_credit_cost
    }
+ [Ask Wide-Blue for a partial emergency discharge against three future calving-lane watches.]
    // ghostlight.action_label: negotiate
    // ghostlight.branch: renegotiate_herd_pledge
    // ghostlight.branch_label: renegotiate_herd_pledge
    {herd_trust >= 4 && cradle_efficiency >= 3:
        ~ herd_trust = herd_trust - 2
        ~ heat_reserve = heat_reserve + 1
        -> ending_herd_success
    - else:
        ~ herd_trust = herd_trust - 1
        -> ending_herd_cost
    }
+ [Drain the adult work shelf and keep the rotating caretakers through another watch.]
    // ghostlight.action_label: move
    // ghostlight.branch: shift_cost_to_caretakers
    // ghostlight.branch_label: shift_cost_to_caretakers
    {heat_reserve >= 3 && caretaker_strain <= 2:
        ~ heat_reserve = heat_reserve - 1
        ~ caretaker_strain = caretaker_strain + 2
        -> ending_caretaker_success
    - else:
        ~ heat_reserve = heat_reserve - 1
        ~ caretaker_strain = caretaker_strain + 2
        -> ending_caretaker_cost
    }

=== ending_mobility_success ===
// ghostlight.ending_label: mobility_debt_bounded
// ghostlight.training_hook: need_first_care_with_mobility_cost
Ruun places the infant stone on the warm side and knots Vara's family cord to the wetland repair branch.

The nursery receives heat. An eastbound unit takes the sealed graft frame, adding weight and risk to its own route. Vara remains to clean torn prism cells, sort fungal residue, and groom the herd until the mat and road recognize repair.

"Care remains unconditional," Ruun says.

"Departure has discovered conditions," Vara answers.

The rule has prevented cradle priority from becoming property. It has also taken movement from the family with the fewest witnesses to spend. Both facts enter the public shelf.
-> END

=== ending_mobility_cost ===
// ghostlight.ending_label: mobility_debt_scapegoat
// ghostlight.training_hook: weak_evidence_hardens_class_pressure
Ruun ties Vara's cord to the repair branch before the road has separated necessity from damage.

No other unit accepts the graft frame; its heat edges fade while Vara remains under obligation. The prismwake mat keeps its silver scar. Wide-Blue keeps the warmth banked.

The infant is warmed from reserve. The shortfall survives, now wearing Vara's family as an explanation.

By next cycle, Stillwarm Shelf will call this precedent. Precedent is what a hurried guess becomes after enough tired people arrange their work around it.
-> END

=== ending_credit_success ===
// ghostlight.ending_label: commons_credit_spent
// ghostlight.training_hook: ecological_credit_moves_shortage_forward
The candle road carries Stillwarm Shelf's promise downhill. The wounded prismwake mat opens a narrow sugar seam beside its scar, enough for this herd and no more.

Wide-Blue feeds, returns, and lowers clear plates under the collection rib. Gold runs into the third cradle stone.

The nursery is warm. Vara leaves with viable grafts. The breeding ground's request branch goes dark behind them.

The next family will arrive at a commons with less standing and a wetland that has already been generous once. The shortage has been moved into the future, where it will be tempted to introduce itself as somebody else's bad planning.
-> END

=== ending_credit_cost ===
// ghostlight.ending_label: commons_credit_overdrawn
// ghostlight.training_hook: promise_without_transport_capacity
Ruun sends the request downhill.

The prismwake mat flashes pale green, then silver. The fungal road cannot carry more sugar promise while its waste lane is at capacity. Wide-Blue tastes the failed offer and closes the remaining clear plate.

Stillwarm Shelf spends standing without receiving heat. Vara's graft route narrows. The adult work shelf goes cold anyway.

A promise becomes infrastructure only when every body in the chain can carry it. Otherwise it is decoration tied in mineral cord.
-> END

=== ending_herd_success ===
// ghostlight.ending_label: herd_pledge_renegotiated
// ghostlight.training_hook: nonhuman_partner_sets_future_price
Ruun lays three calving-lane cords before Wide-Blue: parasite watch, mineral washing, predator guard.

Wide-Blue presses the one clear plate against the collection rib. The herd follows by degrees, releasing enough warmth to carry the infant and infirm cradles through totality.

The bargain does not repair the prismwake mat. It gives the breeding ground time to do so, and gives the herd first claim on three future work rotations.

Vara leaves with the grafts. Ruun moves three caretaker markers from the route shelf to the calving shelf. The nursery stays open by making tomorrow's labor less free.
-> END

=== ending_herd_cost ===
// ghostlight.ending_label: herd_pledge_refused
// ghostlight.training_hook: emergency_language_cannot_replace_trust
Ruun offers future calving work before the heat bays are clean enough and the herd is willing enough to believe it.

Wide-Blue's plates go opaque. The herd steps out from under the collection ribs and forms a broad wall between the breeding ground and the wounded wetland.

The gesture is not cruelty. The herd will not be converted into a reserve tank whenever Sa'ueia accounting becomes emotional.

The two inward cradles keep their heat. Vara's route closes. The adult work shelf freezes under a lesson nobody can put there to keep warm.
-> END

=== ending_caretaker_success ===
// ghostlight.ending_label: caretaker_bodies_absorb_shortfall
// ghostlight.training_hook: commons_care_can_make_a_stationary_class
Ruun rolls the adult stone into the infant hollow.

The inner cradles hold steady. Vara leaves with the grafts. Wide-Blue keeps the herd's withheld warmth and the wetland keeps its refusal.

Ruun and the outgoing caretakers work totality in the blue cold, chest hands wrapped around tools between tasks. Their replacement watch is delayed because those bodies are now the reserve.

No infant is priced. No infirm adult is displaced. The same tired specialists remain at Stillwarm Shelf for another cycle. A commons can keep its promise and still teach particular bodies that departure is for other people.
-> END

=== ending_caretaker_cost ===
// ghostlight.ending_label: caretaker_capacity_overdrawn
// ghostlight.training_hook: labor_is_a_finite_heat_sink
Ruun drains the adult work shelf after the workers have already spent what warmth they carried.

The infant cradles hold. Then Ruun's chest hands stop sorting clean graft waste from spoilage. The candle road closes the nearest waste branch rather than accept a contaminated mix. Vara's departure waits on a task nobody can finish safely.

The breeding ground has preserved heat by overdrawn labor and lost waste capacity in exchange.

Caretakers are infrastructure only in the sense that infrastructure can fail, remember who overloaded it, and become politically unavailable at the worst possible hour.
-> END
