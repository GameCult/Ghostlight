// ghostlight.artifact_id: ledger_breeding_ground_pulse_niche_v0_branch_fold_v0
// ghostlight.fixture_id: ledger-breeding-ground-pulse-niche-v0
// ghostlight.scene_id: ledger-breeding-ground-pulse-niche-v0.four-return-feeding-apron
// ghostlight.final_ink_path: examples/ink/zyphos/ledger-breeding-ground-pulse-niche-v0.branch-and-fold.v0.ink

VAR road_credit = 2
VAR lantern_reserve = 2
VAR herd_trust = 2
VAR dependent_reserve = 3
VAR pulse_integrity = 4
VAR salt_stock = 2
VAR family_standing = 3
VAR caretaker_legitimacy = 2
VAR evidence_chain = 1
VAR shared_obligation = 0
VAR route_delay = 0

-> start

=== start ===
At the south feeding apron, wealth arrives warm, dirty, or late.

The apron is a crescent of dark root-stone between nursery and wetland. Four shallow ramps descend north into low communal hollows under lantern trees. A candle fungal road reaches the west point in two rows of amber beads. Prismwake mats flash beyond the south edge in the returning light. East of the central pulse basin, glassbacks fold beside a ribbed heat braid, their translucent dorsal plates glowing like windows that have developed opinions about rent.

Umbros still covers most of the dim sun. The fixed black world has begun to release it, and every organism on the apron is counting the bright interval in a different currency.

Tesh, the rotating caretaker, folds four running legs beside the basin's low circular work rail. Two smaller chest hands stir mineral paste through warm water. The basin does not make food. It is where road minerals, stored sugars, herd heat, clean tissue, and skilled hands briefly agree to become nursery care.

-> people_and_work

=== people_and_work ===
Oru rests at the north rail, pale with age and infirmity, one facial fan drooping over the archive membranes. Oru has occupied the commons long enough to distrust anyone who uses the word temporary twice.

"West road is hungry," Oru says.

"It is a road," Tesh says. "Hunger is its principal civic program."

At the east heat braid, the ash-striped glassback herd stands broadside. Adults share warmth flank to flank while calves nose the prismwake edge and are corrected by three different species at once.

This is the ordinary first-return tally. Tesh has one work interval before scheduled draws begin.

-> routine_choice

=== routine_choice ===
// ghostlight.choice_layer: first_return_work
+ [Feed the west road a clean failed graft and name its handling history.]
    // ghostlight.action_label: transfer_object
    // ghostlight.branch_label: prime_road_and_salt
    ~ road_credit = road_credit + 2
    ~ salt_stock = salt_stock + 1
    ~ caretaker_legitimacy = caretaker_legitimacy + 1
    Tesh lifts a rejected graft strip from a sealed leaf-skin tray. It carries no admitted memory, only clean tissue, mineral residue, and the exact record of why it failed.

    "Cold separation at the outer shelter," Tesh says, laying it between the amber beads. "No nursery contact. No hidden rot."

    The road draws the strip below. A minute later, pale mineral grains rise around one candle in a neat ring.

    "It has paid you in seasoning," Oru says.

    "Then it has mistaken me for lunch again."
    -> routine_fold
+ [Groom the ash-striped herd and guide two warm adults onto the heat braid.]
    // ghostlight.action_label: gesture
    // ghostlight.branch_label: prime_herd_heat
    ~ herd_trust = herd_trust + 2
    ~ dependent_reserve = dependent_reserve + 1
    ~ pulse_integrity = pulse_integrity + 1
    Tesh walks east, opens both facial fans in calm invitation, and uses the soft chest digits to lift three burden-flower rootlets from a glassback's plate seam.

    The animal turns broadside by choice. A second follows. Amber heat moves through the ribbed braid toward the nursery hollows, while two calves press into the protected center of the herd.

    Oru records the contribution as herd heat, not caretaker skill. Tesh records this as an outrageous lapse in professional recognition.
    -> routine_fold
+ [Carry a mineral cloth across the prismwake edge and lantern-root sinks.]
    // ghostlight.action_label: move
    // ghostlight.branch_label: prime_producer_reserve
    ~ lantern_reserve = lantern_reserve + 2
    ~ pulse_integrity = pulse_integrity + 1
    ~ salt_stock = salt_stock - 1
    Tesh takes the south ramp with a folded mineral cloth balanced across the flank frame. The prismwake mat opens silver-green pores beneath careful feet. At the north end, lantern roots draw the diluted salts out of the cloth and answer with cold blue knots above the infant ramps.

    Light, mineral, path, shelter. Nothing on the circuit considers itself a supplier. Every participant considers itself the reason the others are still solvent.
    -> routine_fold
+ [Audit the coming draws against the portable archive before touching the basin.]
    // ghostlight.action_label: inspect_object
    // ghostlight.branch_label: prime_draw_evidence
    ~ evidence_chain = evidence_chain + 2
    ~ caretaker_legitimacy = caretaker_legitimacy + 1
    ~ route_delay = route_delay + 1
    Tesh spreads flexible archive membranes along the north rail. Pressure scars and chemical edges show the next claims: infant heat, infirm recovery paste, one planned reproductive draw, two graft baths, and a western mineral return marked promised rather than received.

    Oru taps the missing return with one chest digit.

    "The archive is being pessimistic," Tesh says.

    "That is why we let it near children."
    -> routine_fold

=== routine_fold ===
// ghostlight.fold: ordinary_pulse_work
The first-return work settles into the basin.

{road_credit >= 4: The west road raises a second amber line, and clean mineral grains collect beside the work rail.}
{lantern_reserve >= 4: Blue lantern knots brighten above all four nursery ramps; stored sugar and shelter are visibly available.}
{herd_trust >= 4: Two glassbacks remain willingly on the east heat braid while calves rest inside the herd's warm geometry.}
{dependent_reserve >= 4: Warmth reaches the infant hollows with enough margin for the infirm recovery beds.}
{evidence_chain >= 3: The archive shows each promised draw and contribution on separate readable membranes.}
{route_delay >= 1: The basin is well counted and one work interval behind. Accuracy has once again failed to apologize for taking time.}

Then Varo arrives from the west with a route frame of salt packets and a reproductive draw token tied to the outer rail.

-> draw_arrival

=== draw_arrival ===
Varo is the advocate for a family remembered across more routes than Tesh has personally walked. Deep umber fibers lie sleek over a long six-limbed body; two chest hands keep the salt frame level. The paired facial fans are open in formal confidence.

Two of Varo's family wait beyond the fungal beads. Their scheduled reproductive draw needs the clean graft bath, stored heat, mineral paste, and specialist interval immediately after full light returns.

-> shortfall_reveal

=== shortfall_reveal ===

The west road tastes the frame and extinguishes half its candles.

One salt packet hangs full. The other is mostly woven rind.

Varo says, "We spent the missing share stabilizing a glassback calving lane after the south mat tore. The herd carries witness."

From the north hollows comes the thin sour chorus of infants whose heat reserve is falling. Oru checks the basin membrane.

"Cold reached the first ramp early," Oru says. "Dependent draw rises now. The scheduled draw no longer fits unless someone replaces the missing pulse."

The shortage is not a moral lesson. It is sugar, heat, mineral, tissue, time, and several interested parties waiting to see who gets described as necessary.

~ pulse_integrity = pulse_integrity - 1

-> pressure_choice

=== pressure_choice ===
// ghostlight.choice_layer: shortfall_testimony_and_cost
+ [Ask Varo to open only the archive layer covering the claimed calving rescue.]
    // ghostlight.action_label: speak
    // ghostlight.branch_label: verify_family_claim
    ~ evidence_chain = evidence_chain + 2
    ~ family_standing = family_standing + 1
    ~ route_delay = route_delay + 1
    Tesh points to the empty packet and then to Varo's closed archive case.

    "One layer," Tesh says. "The rescue, the material spent, and its witnesses. Nothing beneath."

    Varo's fans narrow. Then one membrane opens over the west rail: calf heat, torn silver mat, three family bodies hauling mineral cloth, and a route mark that confirms aid while disputing the amount.

    It proves generosity. It does not manufacture salt.
    -> allocation_fold
+ [Invite the ash-striped herd to answer the claim through its plate memory.]
    // ghostlight.action_label: gesture
    // ghostlight.branch_label: seek_herd_witness
    ~ herd_trust = herd_trust + 1
    ~ route_delay = route_delay + 1
    Tesh turns east, opens both facial fans, and lays the empty salt rind where the nearest glassback can smell it.

    {herd_trust >= 4:
    The adults arrange flank to flank. Heat and low memory gradients move through their plates: alarm, torn mat, trapped calf, Sa'auei'a bodies working until the lane opened.
    ~ evidence_chain = evidence_chain + 2
    - else:
    One adult exposes a cloudy plate and the rest keep their calves inward. The answer confirms distress but withholds the family-sized detail Tesh wants.
    ~ evidence_chain = evidence_chain + 1
    }

    Herd testimony is excellent at proving that a rescue mattered and poor at becoming a receipt on command.
    -> allocation_fold
+ [Replace the missing share from the nursery's sealed salt stock.]
    // ghostlight.action_label: transfer_object
    // ghostlight.branch_label: spend_commons_stock
    ~ salt_stock = salt_stock - 2
    ~ road_credit = road_credit + 1
    ~ pulse_integrity = pulse_integrity + 1
    ~ dependent_reserve = dependent_reserve - 1
    Tesh breaks two seals at the basin's inner rail and pours stored mineral paste into the road's pale ring.

    The west candles relight. The nursery membrane drops from safe green into thin amber.

    Varo looks relieved. Oru looks at Tesh.

    "You have made the shortfall invisible," Oru says.

    "Only to anyone who refuses to look at the reserve."
    -> allocation_fold
+ [Pause the scheduled draw and call for a shared repair pledge from every family present.]
    // ghostlight.action_label: speak
    // ghostlight.branch_label: open_shared_obligation
    ~ shared_obligation = 1
    ~ caretaker_legitimacy = caretaker_legitimacy + 1
    ~ family_standing = family_standing - 1
    ~ route_delay = route_delay + 2
    Tesh strikes the basin rail with a route cord. The note travels west through fungal beads, east through glassback plates, and north into the nursery hollows.

    "The dependent draw holds first," Tesh announces. "The scheduled draw pauses. Anyone who wants this pulse preserved may put work beside the deficit."

    Varo's fans close halfway. A public invitation turns a prestigious family's private shortage into everybody's arithmetic.

    Around the apron, flank frames begin opening anyway.
    -> allocation_fold

=== allocation_fold ===
// ghostlight.fold: plural_evidence_before_allocation
The second tally has more truth in it and no additional sunlight.

{evidence_chain >= 4: Archive, road, and herd traces support Varo's account strongly enough to separate rescue work from the remaining shortfall.}
{evidence_chain <= 2: The claimed rescue remains plausible, public, and inconveniently unpriced.}
{family_standing >= 4: Varo's family credit draws several route travelers close enough to offer help before being asked.}
{family_standing <= 2: Some families watch Varo with the quick attention usually reserved for a large animal discovering that the bridge also has opinions.}
{salt_stock <= 0: The sealed salt niche at the inner rail is empty; both today's margin and tomorrow's explanation have been spent.}
{dependent_reserve <= 2: The infant-ramp membrane stays amber. Covering one claim has made dependent care visibly thin.}
{shared_obligation == 1: Work packets, spore bundles, and route cords accumulate on the outer rail under many family marks.}
{route_delay >= 3: Full light has returned. The best reproductive interval is narrowing while the nursery consumes heat continuously.}

The living partners do not vote. They make offers physically. The road holds or closes minerals. The lantern trees brighten or ration shelter. The mats open or sour. The herd stays or walks.

Tesh can declare the draw. Tesh cannot make those offers exist.

-> final_threshold

=== final_threshold ===
// ghostlight.fold: draw_allocation_threshold
The pulse basin shows the whole class argument as plumbing.

North: four nursery ramps and the dependent membrane. West: candle road and missing salt. East: herd heat. South: silver-green producer mats in hard-won light. At the center, the low work rail holds Varo's token, Oru's archive, sealed graft vessels, and whatever contributions the choices have left alive.

{road_credit >= 4: The west candles lean toward the basin, willing to carry another mineral exchange.}
{lantern_reserve >= 4: All four northern ramps remain lit in cold blue, with stored sugar still behind the signal.}
{herd_trust >= 4: The ash-striped adults keep their broad warm plates against the eastern braid.}
{pulse_integrity <= 2: Gaps have appeared in the apron circuit: dim candles, cold ribs, closed mat pores, and people pretending those are separate problems.}
{caretaker_legitimacy >= 4: Oru moves the archive beside Tesh instead of behind the nursery rail. The decision will be witnessed as stewardship, not private preference.}
{caretaker_legitimacy <= 2: Oru stays north of the basin, guarding the dependent ramp from the caretaker's arithmetic.}

One decision remains: who receives this pulse, who owes the next one, and whether the answer becomes a precedent with descendants.

-> final_choice

=== final_choice ===
// ghostlight.choice_layer: draw_allocation
+ [Protect the dependent baseline and move the reproductive token to the next pulse.]
    // ghostlight.action_label: refuse
    // ghostlight.branch_label: protect_dependent_baseline
    {dependent_reserve >= 3 && caretaker_legitimacy >= 3:
        Tesh slides the token to the archive's next-return edge and opens the north basin valves first.
        -> ending_dependent_success
    - else:
        Tesh delays the scheduled draw without enough reserve or shared confidence to make the refusal hold cleanly.
        -> ending_dependent_cost
    }
+ [Admit the scheduled draw against a family-bound repair obligation.]
    // ghostlight.action_label: authorize
    // ghostlight.branch_label: bind_family_repair
    {road_credit >= 3 && evidence_chain >= 3 && pulse_integrity >= 3:
        Tesh knots the draw token to two west-route cords and leaves both ends in Varo's chest hands.
        -> ending_family_debt_success
    - else:
        Tesh admits the draw on reputation and a repair promise the apron cannot yet digest.
        -> ending_family_debt_cost
    }
+ [Spread the repair obligation across every family that placed work on the outer rail.]
    // ghostlight.action_label: use_object
    // ghostlight.branch_label: share_repair_labor
    {shared_obligation == 1 && caretaker_legitimacy >= 3 && evidence_chain >= 2:
        Tesh divides the deficit across the offered route cords, each contribution legible and separately recallable.
        -> ending_shared_success
    - else:
        Tesh announces a shared burden before enough families have actually consented to carry it.
        -> ending_shared_cost
    }
+ [Spend the remaining heat, sugar, and salt margin to cover both draws now.]
    // ghostlight.action_label: transfer_object
    // ghostlight.branch_label: cover_both_from_reserve
    {dependent_reserve >= 4 && salt_stock >= 1 && pulse_integrity >= 3:
        Tesh opens the east heat braid and the remaining mineral seals together.
        -> ending_reserve_success
    - else:
        {lantern_reserve >= 4 && salt_stock >= 1 && pulse_integrity >= 3:
            Tesh opens the lantern-root sugar sink and the remaining mineral seals together.
            -> ending_reserve_success
        - else:
            Tesh opens margins that exist separately and calls them one reserve.
            -> ending_reserve_cost
        }
    }

=== ending_dependent_success ===
// ghostlight.ending_label: dependent_baseline_success
// ghostlight.training_hook: care_priority_with_future_claim_preserved
Warm paste reaches the infant hollows first. The sour chorus eases one voice at a time.

Varo's token remains in the archive, delayed but not erased. {family_standing >= 4: Several route-rich families witness the new date, which makes retaliation more expensive and assistance more likely.}{family_standing < 4: Varo must leave with a promise whose value depends on whether thinner routes still remember it tomorrow.}

The commons has refused a draw, not a lineage. The distinction costs Tesh the rest of the shift explaining it to people who understood perfectly the first time.

Oru records: dependents protected; scheduled claim carried forward; no owner discovered.
-> END

=== ending_dependent_cost ===
// ghostlight.ending_label: dependent_baseline_cost
// ghostlight.training_hook: triage_without_legitimacy_shrinks_future_supply
The north valves open. The infants get heat. Today is not allowed to become a funeral just because yesterday had poor accounts.

But the refusal has no broad witness and the reserves are already thin. Varo takes the family token, the remaining salt frame, and several future specialists west.

The dependent baseline survives the hour. The next pulse loses skilled hands and a route-rich contributor. Tesh has protected the rule and weakened the machinery that lets the rule keep winning.
-> END

=== ending_family_debt_success ===
// ghostlight.ending_label: family_bound_repair_success
// ghostlight.training_hook: route_credit_buys_access_and_future_work
The west road opens a narrow mineral lane. Varo's family receives the scheduled graft bath and heat interval after the dependent valves stabilize.

The price is not a number. Two future west-route repairs, one clean body return, three mineral carries, and public witness that the debt belongs to the family that can afford to travel far enough to pay it.

{route_delay >= 3: The best interval is brief; the specialists work with fast hands and no ceremonial margin.}{route_delay < 3: Enough light remains for the specialists to work without borrowing time from the next watch.}

It is a fair exchange by every current rule. It is also how route-rich families keep turning capacity into access. Both facts fit in the archive. Neither eats the other.
-> END

=== ending_family_debt_cost ===
// ghostlight.ending_label: family_bound_repair_cost
// ghostlight.training_hook: reputation_cannot_replace_material_capacity
Varo's token crosses the rail. The basin warms. The west road lets the first mineral wash pass and then closes around the second.

Stored sugar meets insufficient salt. The graft bath clouds. The specialists seal it before harm enters the waiting bodies, but the wasted heat cannot be folded back into a promise.

Varo's family leaves owing a repair for a draw it did not receive. The infants lose margin. The road records attempted overdraw under every witness present.

Reputation got them to the basin. It could not make the basin larger.
-> END

=== ending_shared_success ===
// ghostlight.ending_label: shared_repair_success
// ghostlight.training_hook: mutual_aid_interrupts_inherited_access
The outer rail becomes a small, untidy federation.

One family offers spore carrying. Another takes the western repair turn. A thin-standing pair contributes three nights guarding the prismwake regrowth edge. Varo's family keeps the longest mineral carry because rescue does not abolish arithmetic.

The road relights by sections. The herd stays on the braid. The reproductive draw begins late, after dependent care, under obligations no single lineage can convert into exclusive ownership.

Oru studies the crowded archive.

"This will be difficult to collect," Oru says.

"Yes," Tesh says. "That is how we know nobody has quietly become the treasury."
-> END

=== ending_shared_cost ===
// ghostlight.ending_label: shared_repair_cost
// ghostlight.training_hook: solidarity_without_consent_becomes_assessment
Tesh divides the debt before the offers arrive.

Families find obligations beside their marks that they did not place there. Several take their work packets back. The road dims at the edge of each withdrawal. Varo's shortage has become a commons levy with excellent intentions and the manners of theft.

The scheduled draw pauses anyway. So does a mineral convoy that would have fed the next pulse.

Oru crosses the short north gap and removes Tesh's hand from the archive with two patient chest digits.

"Shared," Oru says, "is a description of consent. Not a spell cast over a shortage."
-> END

=== ending_reserve_success ===
// ghostlight.ending_label: reserve_bridge_success
// ghostlight.training_hook: accumulated_ecological_credit_bridges_one_shortfall
The apron spends its margin.

Herd heat and lantern sugar hold the northern ramps while salt and clean graft capacity cover Varo's scheduled interval. The road keeps one amber line open. Prismwake pores close slowly instead of all at once.

Both draws finish. Tomorrow's basin will start poorer.

The archive names exactly whose earlier work made the bridge possible: grazers that stayed, families that carried, roads that digested, trees that stored, mats that tolerated, caretakers who did not mistake the final valve for authorship.

It is a success small enough to require repayment. Those are usually the durable kind.
-> END

=== ending_reserve_cost ===
// ghostlight.ending_label: reserve_bridge_cost
// ghostlight.training_hook: separate_margins_do_not_make_a_common_surplus
Tesh opens everything.

For a few breaths the basin looks abundant: blue lantern light, amber candles, warm ribs, silver-green mats, two care circuits running at once.

Then the east braid cools. The road takes minerals back into its body. Lantern knots above the fourth ramp go dark. The reproductive bath is sealed halfway through, and the dependent membrane drops to red before the herd chooses another corridor.

The failure is not one empty store. It is five partners learning that this apron spends promises faster than it repairs them.

By the next eclipse, the south feeding apron will still exist. The pulse niche may not.
-> END
