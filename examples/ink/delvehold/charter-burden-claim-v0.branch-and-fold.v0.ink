// ghostlight.artifact_id: charter_burden_claim_v0_branch_fold_v0
// ghostlight.fixture_id: charter-burden-claim-v0
// ghostlight.scene_id: charter-burden-claim-v0.rillgate-answering-table
// ghostlight.final_ink_path: examples/ink/delvehold/charter-burden-claim-v0.branch-and-fold.v0.ink

VAR evidence_chain = 1
VAR provenance_depth = 0
VAR public_record = 1
VAR town_copy = 0
VAR reserve_capacity = 2
VAR crop_harm = 2
VAR conflict_exposed = 0
VAR supplier_pressure = 1
VAR warranty_public = 0
VAR pump_service = 2

-> start

=== start ===
// ghostlight.scene: rillgate_contract_hall_establishing
Rillgate Contract Hall opens every morning by proving that yesterday happened.

The long basalt room lies between a rain-bright wagon arch and a barred freight lift descending into the Greathold. Contract cords run along the south wall: copper braid for power, blue flax for water, black horsehair for insurance. Each cord passes through stamped metal discs naming the hands that sold, carried, insured, and received what moved.

A public counter cuts across the middle of the hall. One section is low enough for halflings and goblins to use without conducting law through somebody else's armpit. A shallow comparison basin sits there, circled by plain runes. Beyond it wait three stone seats for civic seals and, against the east wall, a bonded locker of pale mana crystal.

Pella Reedbank reaches the low counter at first bell with rain on her blue-green cloak, mud on her broad boots, and two stoppered jars on a handcart. She is the chosen speaker of Brackenwash, a halfling farming town whose dwarven pump still lifts water while glassy weeds spread through the irrigated fields.

-> morning_clerk

=== morning_clerk ===
// ghostlight.scene: rillgate_morning_clerk
Nali Stoneclip, the dwarf receiving clerk, is hanging fresh date-discs on the contract cords. She has iron-grey hair bound close, square spectacles, and an expression trained by twenty years of people arriving with disasters that believed themselves unprecedented.

"Power, water, warranty, or accusation?" Nali asks.

"Yes," says Pella.

Nali looks at the two jars, the swollen leather field book, and the pump contract tied in oilskin. "Low counter. We may need the good blotting sand."

This is ordinary work at Rillgate: freight entered, promises traced, harms given a place to stand. Pella's first choice is not whether Brackenwash suffered. It is which part of the suffering the hall must touch first.

-> filing_choice

=== filing_choice ===
// ghostlight.choice_layer: opening_evidence
+ [Seat the first water jar in the comparison basin and ask Nali to countersign the intact stopper.]
    // ghostlight.action: lodge_physical_evidence
    // ghostlight.branch: prime_sample_chain
    // ghostlight.intent: make_the_sample_custody_legible_before_anyone_tests_it
    ~ evidence_chain = evidence_chain + 2
    ~ public_record = public_record + 1
    Pella lifts the green-glass jar with both hands and sets it into the basin's brass cradle. Brown irrigation water slumps against one side. A hair-thin silver root has grown through the cork from inside.

    Nali does not call it proof of dwarven fault. She calls three porters to witness the stopper, scratches the time on a tin strip, and crimps the strip around the jar's neck.

    "Now anyone may doubt the cause," she says. "They may not improve the jar."
    -> filing_fold
+ [Unroll the pump contract and make Nali trace its copper-and-blue cord before opening a sample.]
    // ghostlight.action: trace_contract
    // ghostlight.branch: prime_provenance
    // ghostlight.intent: attach_the_claim_to_the_issuing_seal_fuel_lot_route_and_surety
    ~ provenance_depth = provenance_depth + 2
    ~ supplier_pressure = supplier_pressure + 1
    ~ crop_harm = crop_harm + 1
    Pella lays the oilskin packet flat. Nali finds its little stamped sunwheel, then follows copper braid through the wall rings: Brackenwash pump, Rillgate exporter, Deep Company fuel lot, Red Mantle surety.

    The tracing takes most of an hour. Outside, rain taps the wagon arch. In Brackenwash, another scheduled irrigation is running through the suspect engine.

    "There," Nali says, pinning four names to one cord. "The promise has acquired relatives."
    -> filing_fold
+ [Open the field book at the first glassweed sketch and ask for a public copy of every dated page.]
    // ghostlight.action: request_public_copy
    // ghostlight.branch: prime_field_record
    // ghostlight.intent: keep_years_of_local_observation_outside_the_supplier_archive
    ~ evidence_chain = evidence_chain + 1
    ~ public_record = public_record + 2
    ~ crop_harm = crop_harm + 1
    Nali brings out thin slate sheets and a pressure frame. Pella turns the pages: planting dates, hoof lesions, rainfall, pump hours, careful drawings of weeds whose leaves end in translucent edges.

    Each pressed copy takes a minute. The people waiting behind Pella begin doing queue arithmetic with their faces.

    Nali copies the irritation too. Public records are allowed to remember that evidence costs other people time.
    -> filing_fold
+ [Keep the second jar and duplicate field book on the town's side of the counter.]
    // ghostlight.action: retain_duplicate_custody
    // ghostlight.branch: prime_town_copy
    // ghostlight.intent: preserve_an_independent_copy_if_the_hall_or_supplier_loses_the_first
    ~ town_copy = town_copy + 2
    ~ evidence_chain = evidence_chain + 1
    ~ supplier_pressure = supplier_pressure + 1
    Pella moves one jar and the duplicate book beneath her handcart's leather flap.

    "Suspicious?" Nali asks.

    "Agricultural," Pella says. "We keep two of anything a committee can mislay."

    Nali marks the retained copy on the public docket. Suspicion, once receipted, becomes a custody arrangement.
    -> filing_fold

=== filing_fold ===
// ghostlight.fold: evidence_enters_public_custody
Nali hangs a white claim disc across the copper-and-blue contract cord. The routine queue sees it. Porters carry on weighing freight. Nobody rings an alarm. A burden claim begins by making ordinary business proceed around a fact it can no longer call private.

{evidence_chain >= 3: The sealed jar, dated pages, or both now have enough witnessed custody to survive a later accusation of convenient invention.}
{provenance_depth >= 2: Four stamped parties share one visible cord: pump buyer, exporter, fuel supplier, and surety.}
{public_record >= 3: The wall docket carries more of Brackenwash than a clerk's summary can politely shrink.}
{town_copy >= 2: A second jar and duplicate book remain under Pella's hand, beyond the hall's custody.}
{crop_harm >= 3: The filing has consumed another irrigation turn. Procedure has not stopped the pump or the weeds.}

By the rule of smallest useful jurisdiction, Rillgate can open the answering table: the disputed fuel lots are in its bonded locker and the exporter keeps a workshop here.

-> table_draw

=== table_draw ===
// ghostlight.scene: rillgate_answering_table_draw
Nali turns the three stone seats toward the public counter and draws eligible seals from a slotted bronze drum.

The first belongs to a flour-dusted dwarf who runs a public bakery. The second belongs to a long-limbed human lift mechanic in a red scarf. The third belongs to a grey-green goblin chart-printer with long ears and blue ink ground into every fingertip. Mastership makes all three civic equals here. Their workshops, not their ancestries or their wealth, carry the seals.

Master Orik Vane arrives before the third seal is set down. He represents the Rillgate exporter: a broad dwarf in a rust-red coat, with a silver-clipped beard and the clean gloves of a person whose products become dirty elsewhere.

"We can replace the pump lining under warranty," Orik says. "Today. No admission, no interruption, no spectacle."

Pella notices the same blue ink on the goblin printer's fingers and on two consignment discs in Orik's contract cord.

The pump still runs. The crops still change. The table must decide what its next hour is for.

-> pressure_choice

=== pressure_choice ===
// ghostlight.choice_layer: warranty_and_conflict_pressure
+ [Accept a warranty inspection only if its report stays on the public claim cord.]
    // ghostlight.action: condition_offer
    // ghostlight.branch: condition_public_warranty
    // ghostlight.intent: gain_immediate_service_without_letting_the_claim_become_private
    ~ warranty_public = warranty_public + 1
    ~ public_record = public_record + 1
    ~ pump_service = pump_service + 1
    ~ supplier_pressure = supplier_pressure - 1
    "Send the engineer," Pella says. "Their report hangs here before it goes anywhere else."

    Orik smiles like a door being shut gently. "Warranty reports belong to the parties."

    Nali taps the white claim disc. "It has parties now."

    The three drawn seals leave the offer on the docket. Brackenwash may get a working lining quickly; Orik may discover that quick work has witnesses.
    -> hearing_fold
+ [Slide every stamped custody disc into one line and ask Nali to read the crystal lot aloud.]
    // ghostlight.action: expose_provenance
    // ghostlight.branch: deepen_lot_trace
    // ghostlight.intent: identify_the_specific_fuel_lot_a_bounded_order_could_isolate
    ~ provenance_depth = provenance_depth + 2
    ~ supplier_pressure = supplier_pressure + 2
    ~ crop_harm = crop_harm + 1
    Pella draws the cord across the low counter. Nali reads each disc without emphasis: extraction gallery, sorting house, insurer, Rillgate locker, Brackenwash engine.

    Orik objects at the third disc. Then the fifth. By the seventh, his objections have formed their own provenance chain.

    The bonded locker at the east wall holds six crates from the same lot. Isolating them would mean touching contracts beyond Brackenwash.
    -> hearing_fold
+ [Lay the field book open toward the public benches and read the first farmer's name, not the first number.]
    // ghostlight.action: publish_testimony
    // ghostlight.branch: name_the_burden
    // ghostlight.intent: keep_technical_uncertainty_attached_to_the_people_absorbing_it
    ~ evidence_chain = evidence_chain + 1
    ~ public_record = public_record + 2
    ~ supplier_pressure = supplier_pressure + 1
    "Dena Marr," Pella reads. "South field. Pump turn at third bell. Four sheep with crystal lesions around the mouth."

    Orik begins to say that lesions are not provenance.

    "Correct," says the flour-dusted Master. "They are burden. We are deciding whether the two touch."

    Pella turns the book so the public benches can see the drawing. Uncertainty remains. Anonymity does not.
    -> hearing_fold
+ [Point to the printer's blue fingers and challenge the third seal for concealed interest.]
    // ghostlight.action: challenge_seal
    // ghostlight.branch: expose_bench_conflict
    // ghostlight.intent: prevent_a_paid_contract_supplier_from_judging_the_contract
    ~ conflict_exposed = conflict_exposed + 2
    ~ public_record = public_record + 1
    ~ supplier_pressure = supplier_pressure + 1
    Pella points from inked fingers to inked consignment discs.

    The goblin Master bares small square teeth, not quite a smile. "My workshop prints his route charts. We print half the hall."

    "Half is enough," Nali says. Claimants may challenge one drawn seal. The printer lifts the civic seal from the stone seat and steps into the witness benches, annoyed but not silenced.

    The replacement is a soot-dark dwarf chimneywright with no mark on Orik's cord. The hearing loses twenty minutes and gains an argument it can survive.
    -> hearing_fold

=== hearing_fold ===
// ghostlight.fold: public_claim_meets_service_dependency
Nali pours a spoonful from the admitted jar into the comparison basin. The runes do not name a culprit. They lift a blue-green filament through the brown water, the same hue that rims the glassweed drawings.

{evidence_chain >= 3: Sample, dates, witnesses, and symptoms now form a chain strong enough to require an answer, though not a cosmology.}
{provenance_depth >= 2: The table can point to a named crystal lot instead of threatening every dwarven engine at once.}
{public_record >= 3: Porters, waiting traders, and three civic seals can all see which facts and doubts have entered the docket.}
{town_copy >= 2: Pella still owns a duplicate beyond the counter, insurance against a clean official loss.}
{conflict_exposed >= 2: The paid chart-printer sits among witnesses, and an unbound chimneywright holds the third deciding seal.}
{conflict_exposed == 0: Blue ink remains on the third sealkeeper's fingers. The connection has not entered the record.}
{warranty_public >= 1: A fast repair route exists, but Orik's engineer will have to return a report to the white claim cord.}
{pump_service >= 3: The offered lining could put a repair cart on the Brackenwash road before dusk.}
{supplier_pressure >= 3: Orik has stopped offering reassurance and begun counting which other buyers share the lot.}
{crop_harm >= 4: Another irrigation turn is passing while the table works. In Brackenwash, water remains useful enough to fear losing and suspect enough to fear using.}

The three seals can bind Greathold workshops, crystal custody, surety, and reserve use. They cannot declare what the deep world is, command Brackenwash, or produce power from a stamp.

-> remedy_choice

=== remedy_choice ===
// ghostlight.scene: rillgate_remedy_threshold
The bonded locker holds the disputed lot. The smaller reserve chest beside it holds enough clean crystal to run Brackenwash's pump for nine days, or Rillgate's public hoist for six.

Nali chalks both numbers where the room can see them.

"Speaker," says the human lift mechanic, one hand on the red scarf at his throat. "Name the remedy you want us to spend."

-> final_choice

=== final_choice ===
// ghostlight.choice_layer: requested_redress
+ [Ask for a marked drawdown: isolate the named lot and feed Brackenwash from the reserve for nine days.]
    // ghostlight.action: request_interim_order
    // ghostlight.branch: seek_marked_drawdown
    // ghostlight.intent: stop_the_suspected_exposure_without_turning_off_the_town_pump
    {evidence_chain >= 3 && provenance_depth >= 2 && reserve_capacity >= 1:
        ~ reserve_capacity = reserve_capacity - 1
        -> ending_drawdown_granted
    - else:
        ~ crop_harm = crop_harm + 1
        -> ending_drawdown_refused
    }
+ [Ask the Red Mantle surety to pay for ruined seed, livestock care, and an independent pump inspection.]
    // ghostlight.action: claim_surety
    // ghostlight.branch: seek_restitution
    // ghostlight.intent: turn_documented_harm_into_material_relief_without_claiming_the_ecology_is_settled
    {evidence_chain >= 3 && public_record >= 3:
        -> ending_restitution_granted
    - else:
        -> ending_restitution_deferred
    }
+ [Widen the claim to every connected Hold sharing the lot and carry the present order forward.]
    // ghostlight.action: widen_jurisdiction
    // ghostlight.branch: seek_widened_moot
    // ghostlight.intent: move_authority_outward_because_the_contract_burden_and_conflicts_cross_routes
    {conflict_exposed >= 2 || public_record >= 4:
        ~ supplier_pressure = supplier_pressure + 1
        -> ending_widening_admitted
    - else:
        ~ crop_harm = crop_harm + 1
        -> ending_widening_stalled
    }
+ {warranty_public >= 1} [Take the public warranty repair and keep the white claim disc hanging after the engineer leaves.]
    // ghostlight.action: accept_bounded_repair
    // ghostlight.branch: take_public_warranty
    // ghostlight.intent: restore_service_quickly_without_surrendering_the_public_claim
    {public_record >= 2 && town_copy >= 2:
        -> ending_warranty_public
    - else:
        -> ending_warranty_absorbs_claim
    }

=== ending_drawdown_granted ===
// ghostlight.ending_label: marked_drawdown_granted
// ghostlight.training_hook: bounded_interim_relief_has_a_visible_cost
The three seals stamp one strip and leave the rest of the contract intact.

Porters roll the six disputed crates behind an iron grate. Nali threads a white cord from the reserve chest to Brackenwash's contract disc. Nine days of pump power travel outward. Six days of public-hoist power disappear from Rillgate's easy future.

{reserve_capacity == 1: The reserve chest is visibly half-spent, which is how the hall keeps mercy from impersonating abundance.}

{conflict_exposed >= 2: The substitute chimneywright stamps last. The chart-printer records the order from the witness bench, which is a smaller authority and a cleaner one.}
{conflict_exposed == 0: Blue ink marks the third sealkeeper's fingertips. Pella gets the drawdown and carries the unanswered conflict home with it.}

The pump does not stop. The suspected lot does. Relief is a boundary made of crystal somebody else can now count.
-> END

=== ending_drawdown_refused ===
// ghostlight.ending_label: marked_drawdown_refused
// ghostlight.training_hook: remedy_fails_without_specific_cause_evidence_and_capacity
The table cannot isolate what the record has not named.

Orik offers a broad shutdown to protect the hall from blame. The seals refuse to spend the reserve against an untraced lot. Between those clean positions, Brackenwash loses another pump turn.

Pella leaves with the white claim disc still hanging and the maddening assignment to return with more: a tighter sample chain, a deeper custody trace, or a substitute supply that exists outside rhetoric.
-> END

=== ending_restitution_granted ===
// ghostlight.ending_label: restitution_granted
// ghostlight.training_hook: surety_relief_repairs_people_not_cosmology
The Red Mantle surety is ordered to release seed grain, veterinary fees, and the cost of an inspector chosen outside Orik's contract cord.

Brackenwash can replant. Dena Marr can treat the sheep. The pump remains under question, because money can answer a burden without proving what injured the water.

{crop_harm >= 4: The award includes the second lost irrigation turn, copied from Nali's docket instead of edited out as clerical delay.}
{town_copy >= 2: Pella carries the town's duplicate home beside the award, preserving evidence for the next field rather than surrendering it to this victory.}

Orik calls the order generous. Pella calls it numbered.
-> END

=== ending_restitution_deferred ===
// ghostlight.ending_label: restitution_deferred
// ghostlight.training_hook: harm_without_custody_chain_cannot_reach_surety
The seals believe that Brackenwash is hurt. Belief is not yet a charge against the surety chest.

The field book lacks a public copy, or the jar lacks witnessed custody, or both. Red Mantle's advocate does not need to disprove the glassweed. She only needs to show where the claim changed hands in darkness.

Pella keeps the docket open. Sympathy enters the minutes and purchases nothing.
-> END

=== ending_widening_admitted ===
// ghostlight.ending_label: widening_admitted
// ghostlight.training_hook: jurisdiction_follows_shared_contract_burden
Nali adds every Hold receiving the named lot to the white cord and copies the present evidence outward.

{conflict_exposed >= 2: The recused chart-printer must disclose which route books his workshop supplied. The conflict becomes a map instead of a rumor.}
{public_record >= 4: So many witnessed copies exist that the next moot cannot begin by shrinking Brackenwash back into a warranty number.}

Rillgate's order stays visible while the wider seals gather. That does not make the appeal quick. It makes delay attributable.

Pella leaves beneath the wagon arch with rain ahead and an argument now larger than one town, which is either progress or a more expensive species of weather.
-> END

=== ending_widening_stalled ===
// ghostlight.ending_label: widening_stalled
// ghostlight.training_hook: appeal_stalls_without_public_conflict_or_shared_record
The request points outward. The record does not yet show why authority must follow.

No conflict has been entered against the drawn seals. Too little of the evidence sits in public custody. The next Hold receives a petition, not the present table's obligation.

The white disc stays at Rillgate. Brackenwash keeps pumping while its appeal learns the mountain one clerk at a time.
-> END

=== ending_warranty_public ===
// ghostlight.ending_label: public_warranty_repair
// ghostlight.training_hook: quick_repair_does_not_erase_public_claim
Orik's engineer leaves for Brackenwash with a replacement lining and two observers named by the table.

Nali keeps the white disc across the power-and-water cord. The engineer's report must return to it. Pella's duplicate jar and field book travel home on the handcart, still outside the exporter's custody.

The town may get clean water fastest this way. The claim remains alive long enough to learn whether the lining, the fuel, or something deeper was at fault.

A warranty can be a door. The trick is leaving it open in both directions.
-> END

=== ending_warranty_absorbs_claim ===
// ghostlight.ending_label: private_warranty_absorbs_claim
// ghostlight.training_hook: service_recovery_can_privatize_public_harm
The engineer takes the admitted jar and field book as service materials. The pump receives a new lining before dusk.

Brackenwash gets water. Orik gets the only complete record. The white claim disc comes down because every remaining question has been renamed inspection work.

Weeks later, when glassweed reaches the north field, Pella will have a repaired pump and a story whose custody belongs to the people denying it.
-> END
