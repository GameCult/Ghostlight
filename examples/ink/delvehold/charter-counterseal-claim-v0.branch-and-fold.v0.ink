// ghostlight.artifact_id: charter_counterseal_claim_v0_branch_fold_v0
// ghostlight.fixture_id: charter-counterseal-claim-v0
// ghostlight.scene_id: charter-counterseal-claim-v0.tansy-ford-hearing
// ghostlight.final_ink_path: examples/ink/delvehold/charter-counterseal-claim-v0.branch-and-fold.v0.ink

VAR docket_strength = 2
VAR sample_integrity = 2
VAR town_water = 2
VAR seal_pressure = 1
VAR public_witness = 1
VAR successor_liability = 1
VAR interim_relief = 0
VAR hearing_trust = 2

-> start

=== start ===
// ghostlight.scene: counterseal_landing_establishing
Cistern House Nine starts its mornings by proving that three pumps are stopped. Today it must also prove that a contract is still alive.

The public landing overlooks the dry rune gallery and, through an iron grating, the warm black intake below. Beyond the yellow line and down the grated stair, three brass-and-iron engines wait on the service floor. Terrace folk queue with copper cans along the street wall. A claims table has been set beside the public gauge, close enough that nobody can discuss water as an abstraction for very long.

Mira Fen, field clerk for the halfling town of Tansy Ford, has brought a green contract ribbon, twelve years of field books, a stoppered jar containing a glassy white root that used to be a turnip, and one thumb-sized reserve vial in her satchel.

She has also brought oatcakes. Dwarven claim procedure recognizes contract, injury, and custody. It has not yet admitted breakfast, despite centuries of evidence.

-> morning_registry

=== morning_registry ===
// ghostlight.scene: counterseal_morning_registry
Dorrin Veld, the dwarven keeper of the local seal roll, lays out three shallow stone trays on the table. Claimant. Registry. Answering workshop. Anything moved between them must be called aloud and marked in chalk.

He is broad, gray-bearded, and dressed in a blue archive apron with more pockets than grace strictly requires. He accepts one oatcake, marks it "consumed before custody," and eats it over an empty tray.

At the far end of the landing, three disinterested Masters take their bench beneath the district route map. None sells pumps, crystal, or freight into Tansy Ford. That is why they may hear the claim.

The answering place remains empty. Stonewake Pumpworks has sent no one yet.

Dorrin turns the public slate outward. "Council notice, marked contract, account of injury. Which do you want the room to understand first?"

-> opening_claim_choice

=== opening_claim_choice ===
// ghostlight.choice_layer: opening_the_counterseal_claim
+ [Press the contract's answering-seal impression into damp clay beside Stonewake's name.]
    // ghostlight.action: register_seal_chain
    // ghostlight.branch: prime_answering_seal
    // ghostlight.intent: attach_the_foreign_injury_to_the_workshop_civic_home
    ~ docket_strength = docket_strength + 2
    ~ successor_liability = successor_liability + 1
    ~ public_witness = public_witness + 1
    Mira lays the green ribbon flat. Its old wax bears a mountain split by a vertical wave: Stonewake Pumpworks, civic home registered here.

    Dorrin presses the same mark from the seal roll into fresh gray clay. Old wax and new clay face one another across the table.

    "One name," he calls.

    The bench answers, "One place to complain."
    -> routine_fold
+ [Divide the glass-root sample between claimant and registry trays.]
    // ghostlight.action: divide_evidence
    // ghostlight.branch: prime_sample_custody
    // ghostlight.intent: preserve_material_evidence_under_separate_keepers
    ~ sample_integrity = sample_integrity + 2
    ~ hearing_trust = hearing_trust + 1
    Mira breaks the pale root along a natural fork. It parts with a sound like a spoon against a wineglass.

    Half remains in her jar. Dorrin seals the other half in a square copper cage, marks the time, and places it in the registry tray.

    A woman in the water queue stops whispering. The root has made the field problem audible.
    -> routine_fold
+ [Ask Dorrin to read Tansy Ford's council notice aloud to the landing.]
    // ghostlight.action: request_public_reading
    // ghostlight.branch: prime_town_voice
    // ghostlight.intent: make_the_claimant_authority_and_requested_redress_public
    ~ public_witness = public_witness + 2
    ~ docket_strength = docket_strength + 1
    ~ hearing_trust = hearing_trust - 1
    Dorrin reads the notice in his registry voice: pump water still delivered, east fields lost to glass-root, two cattle lamed, hauling gangs rehired, replacement crystal requested, harvest restitution reserved.

    Mira corrects his pronunciation of Tansy Ford.

    He corrects her correction on the slate. The town is heard, if not improved by the experience.
    -> routine_fold
+ [Keep the last sealed vial in your satchel for the town's own record.]
    // ghostlight.action: withhold_evidence
    // ghostlight.branch: prime_claimant_custody
    // ghostlight.intent: prevent_the_hearing_from_owning_the_only_surviving_sample
    ~ sample_integrity = sample_integrity + 1
    ~ hearing_trust = hearing_trust - 1
    ~ public_witness = public_witness + 1
    Mira places the field book and one jar on the table. The small sealed vial stays in her satchel.

    Dorrin sees the movement. "Unsubmitted reserve?"

    "Tansy Ford prefers not to put every turnip in one court."

    He writes that down without smiling, which is a clerk's way of admitting defeat.
    -> routine_fold

=== routine_fold ===
// ghostlight.fold: ordinary_claim_routine_before_respondent_arrives
Below the landing, pump apprentices call their isolation states. One. Two. Three. Red wedges sit flush; pressure floats lie quiet. Behind Mira, copper cans knock softly in the queue.

{docket_strength >= 4: Old wax, fresh clay, council notice, and Dorrin's chalk line all point toward the same answering seal.}
{sample_integrity >= 4: Two keepers now hold matching pieces of the glass-root, enough for disagreement without disappearance.}
{public_witness >= 3: The claim is no longer a private argument at the table. The water queue has learned its shape.}
{hearing_trust >= 3: Dorrin handles Mira's evidence as something the room must preserve, not merely tolerate.}
{hearing_trust <= 1: Dorrin's chalk grows careful around every object Mira has kept or corrected.}

-> brakka_arrival

=== brakka_arrival ===
// ghostlight.scene: counterseal_brakka_arrival
The lower gallery door opens. Master Brakka Sorn of Stonewake Pumpworks climbs the grated stair with an iron contract chest under one arm and her civic seal at her belt.

She is a square-built dwarf in a dark green work coat. A silver streak divides her black hair exactly as the parting line divides an account: visible, severe, and difficult to argue with.

"My predecessor signed Tansy Ford," she says. "I inherited working pumps. I did not inherit his weather."

-> succession_dispute

=== succession_dispute ===
// ghostlight.scene: counterseal_succession_dispute
Brakka sets the contract chest in the answering-workshop tray. She does not touch Mira's jar.

Master Iven Chalk, the eldest of the three hearing Masters, points to the seal at Brakka's belt. "Did you take Stonewake's name?"

"Yes."

"Tools?"

"Yes."

"Open orders and place on the roll?"

Brakka's jaw tightens. "Yes."

"Then we can spend less of the morning discussing ghosts."

Brakka opens the chest. It holds maintenance letters, delivery tallies, and a clean water assay from the branch office nearest Tansy Ford. "The pump meets output. The feed meets the contract measure. Their local ditch may be carrying the injury. If I shut the line for a theory, the west field dries before the east field recovers."

She is not wrong about the water. This is inconvenient of her.

-> succession_choice

=== succession_choice ===
// ghostlight.choice_layer: answering_successor_defense
+ [Set the old contract impression beneath Brakka's current seal on the public slate.]
    // ghostlight.action: align_seal_impressions
    // ghostlight.branch: bind_successor_to_seal
    // ghostlight.intent: show_that_the_workshop_identity_outlived_its_previous_master
    ~ successor_liability = successor_liability + 2
    ~ docket_strength = docket_strength + 1
    ~ seal_pressure = seal_pressure + 1
    Mira places the green ribbon under the fresh roll impression, then points to Brakka's belt.

    "Different hand," she says. "Same mountain. Same wave. Same monthly bill."

    Brakka looks at the marks rather than at Mira. That courtesy costs her something.
    -> successor_fold
+ [Open the field books to the seasons before and after Stonewake changed the crystal lot.]
    // ghostlight.action: present_records
    // ghostlight.branch: establish_timing
    // ghostlight.intent: connect_the_injury_to_a_recorded_supply_change_without_claiming_final_cause
    ~ docket_strength = docket_strength + 2
    ~ sample_integrity = sample_integrity + 1
    ~ hearing_trust = hearing_trust + 1
    Mira opens three books. Same east ditch. Same planting. Same pump hours. Then a new crystal lot, followed by the first glass sheen on root tips.

    "Sequence," Brakka says. "Not cause."

    "Agreed," Mira says. "That is why we came to ask for inspection instead of a hymn."

    One of the hearing Masters coughs into his beard. It may be approval. It may be oatcake.
    -> successor_fold
+ [Invite Brakka to inspect the divided root without moving it from the registry tray.]
    // ghostlight.action: share_inspection
    // ghostlight.branch: invite_bounded_inspection
    // ghostlight.intent: create_shared_observation_without_surrendering_evidence_custody
    ~ hearing_trust = hearing_trust + 2
    ~ sample_integrity = sample_integrity + 1
    ~ seal_pressure = seal_pressure - 1
    Dorrin lifts the copper cage by its side handles. Mira and Brakka lean from opposite sides of the table.

    The root's white glass threads stop at a band of living brown tissue. Brakka takes out a lens but keeps both hands above the yellow custody line.

    "I have seen crystal burn," she says. "Not this pattern."

    "Good," Mira says. "Now two of us have not named it."
    -> successor_fold
+ [Refuse Stonewake's offered price settlement until the hearing makes an interim water order.]
    // ghostlight.action: refuse_settlement
    // ghostlight.branch: prioritize_interim_relief
    // ghostlight.intent: prevent_money_from_closing_the_civic_claim_while_harm_continues
    ~ seal_pressure = seal_pressure + 2
    ~ town_water = town_water - 1
    ~ hearing_trust = hearing_trust - 1
    Brakka names a sum for the lost east planting. It would pay the hauling gangs through first frost.

    Mira closes the little account board between them.

    "The money is welcome. The water order comes first. A settlement cannot irrigate."

    Behind her, the public gauge rises as Cistern House Nine opens morning flow. Tansy Ford's gauge is six days away and falling.
    -> successor_fold

=== successor_fold ===
// ghostlight.fold: succession_and_causation_join_service_pressure
The claim now has two questions and one pipe running between them: what Stonewake inherited, and what Tansy Ford can survive while causation is tested.

{successor_liability >= 3: Brakka's current seal, Stonewake's old wax, its name, and its open orders form one visible chain across the slate.}
{successor_liability <= 1: The dead predecessor still occupies too much of the table for a man who brought no testimony.}
{docket_strength >= 5: The field sequence and answering-seal chain make dismissal more expensive than investigation.}
{hearing_trust >= 4: Mira and Brakka can disagree over cause while sharing the same bounded inspection.}
{seal_pressure >= 3: Brakka keeps one hand near her seal as if someone might suspend it by reach.}
{town_water <= 1: Mira counts hauling days in the margins while the hearing counts standards.}

-> tavi_arrival

=== tavi_arrival ===
// ghostlight.scene: counterseal_tavi_arrival
Boots strike the street passage at a run. Tavi Mere, pump tender of Tansy Ford, arrives behind a railway porter with mud to his knees and a canvas-wrapped meter vane in both hands.

The porter points firmly at the yellow line. Even emergencies are expected to know where the floor is.

-> fresh_failure

=== fresh_failure ===
// ghostlight.scene: counterseal_fresh_failure
Tavi unwraps the vane inside the claimant tray. A blue-white crust has grown over its copper teeth. The mark was clean when Mira left home. The west ditch has begun to glitter. The backup cistern holds until tomorrow night if the bakery stops first.

For one breath, only the pumps below speak.

Brakka looks sick, then looks annoyed at having done so in public. "If that came from our feed, the branch meter should show it."

"If," says Mira.

"Yes," Brakka says. "That word is why hearings own chairs."

Master Iven turns to Mira. "The bond can buy testing, substitute crystal, or hauling. Not all three before sunset. What does Tansy Ford ask us to preserve first?"

-> interim_choice

=== interim_choice ===
// ghostlight.choice_layer: interim_redress_under_pressure
+ [Seal the fresh vane beside the divided root and spend the day on independent testing.]
    // ghostlight.action: seal_fresh_evidence
    // ghostlight.branch: preserve_sample_chain
    // ghostlight.intent: preserve_a_repeatable_material_case_at_the_cost_of_immediate_relief
    ~ sample_integrity = sample_integrity + 2
    ~ public_witness = public_witness + 1
    ~ town_water = town_water - 1
    Dorrin lowers a second copper cage over the vane. Mira and Tavi mark one side; Brakka and the registry mark the other.

    The bond clerk sends for an independent rune assay. No substitute crystal leaves with Tavi.

    The hearing has protected the evidence. Tomorrow's cistern has not been consulted.
    -> remedy_threshold
+ [Demand that Stonewake's bond release clean reserve crystal before testing begins.]
    // ghostlight.action: request_bond_draw
    // ghostlight.branch: draw_bond_for_clean_feed
    // ghostlight.intent: stop_the_continuing_exposure_before_settling_final_cause
    ~ interim_relief = interim_relief + 2
    ~ seal_pressure = seal_pressure + 2
    ~ town_water = town_water + 1
    ~ hearing_trust = hearing_trust - 1
    Mira pushes the empty green contract pouch into the bond recess of the registry tray.

    "Fill that with a clean lot number," she says. "Argue cause while our west field drinks something else."

    Brakka objects to the amount, then names a reserve house that can supply half. It is not surrender. It is a number with carts attached.
    -> remedy_threshold
+ [Offer continued measured service through the marked meter while a substitute lot is assembled.]
    // ghostlight.action: propose_measured_continuance
    // ghostlight.branch: keep_water_under_meter
    // ghostlight.intent: preserve_water_service_without_treating_continuing_harm_as_normal_performance
    ~ interim_relief = interim_relief + 1
    ~ town_water = town_water + 2
    ~ sample_integrity = sample_integrity - 1
    ~ hearing_trust = hearing_trust + 1
    Mira asks for each remaining crystal charge to be weighed, marked, and kept below the east-ditch flow rate. Tavi can log the pump hours. Stonewake can send an inspector with the first cart.

    "It may worsen the west ditch," Tavi says.

    "It may keep the bakery open," Mira says.

    Neither sentence defeats the other. Dorrin records both.
    -> remedy_threshold
+ [Give Tavi the town's travel purse and send him to hire water carts while the claim stays open.]
    // ghostlight.action: transfer_resource
    // ghostlight.branch: keep_claimant_capacity
    // ghostlight.intent: buy_local_survival_without_trading_away_the_public_claim
    ~ town_water = town_water + 1
    ~ public_witness = public_witness + 1
    ~ interim_relief = interim_relief - 1
    ~ seal_pressure = seal_pressure + 1
    Mira empties the travel purse into Tavi's muddy hands. Fare home, noon meal, and most of the money reserved for lodging.

    "Hire carts. Take Dorrin's copy of the docket. Do not sign a settlement at the branch office."

    Tavi nods and runs. Mira keeps the chair and loses the bed attached to it.
    -> remedy_threshold

=== remedy_threshold ===
// ghostlight.scene: counterseal_remedy_threshold
The claim moot confers beneath the route map. It cannot reach through six days of road and close Tansy Ford's valves. It can reach Stonewake's bond, reserve contracts, inspection ledgers, and right to seek new public work.

{sample_integrity >= 5: Root, vane, times, lot marks, and divided seals make a material case that can survive travel in both directions.}
{sample_integrity <= 2: The hearing has good reason to worry and too little protected matter to tell worry from cause.}
{town_water >= 4: Tansy Ford has a plausible path through tomorrow: measured flow, carts, or replacement crystal.}
{town_water <= 1: The town's next day has narrowed to a dry cistern and whichever institution blinks first.}
{seal_pressure >= 4: Stonewake's seal may keep existing pumps alive while losing the right to promise another one.}
{successor_liability >= 3: Brakka is answering as Stonewake's current Master, not visiting as the descendant of someone else's mistake.}
{interim_relief >= 2: A reserve allotment now exists on the docket with a house, quantity, and departure time.}
{interim_relief <= 0: Tansy Ford is funding its own survival while Stonewake's bond remains a well-guarded number.}
{public_witness >= 4: The water queue can recite which evidence entered whose custody and what relief has not moved.}
{hearing_trust >= 4: Mira and Brakka have enough shared procedure to propose a remedy without pretending they share a diagnosis.}
{hearing_trust <= 1: Every offer now sounds like an attempt to buy a missing fact.}

Master Iven returns to the bench. "We can preserve service, preserve proof, punish the seal, or settle the loss. We may manage three. We will not pretend we can perfect four. Clerk of Tansy Ford: name the order you will carry home."

-> final_choice

=== final_choice ===
// ghostlight.choice_layer: requested_counterseal_order
+ [Ask for bonded clean crystal first, with measured service only until it arrives.]
    // ghostlight.action: request_clean_feed_order
    // ghostlight.branch: seek_bonded_clean_feed
    // ghostlight.intent: stop_the_suspected_exposure_while_preserving_essential_water
    {interim_relief >= 2 && docket_strength >= 4:
        -> ending_clean_feed
    - else:
        -> ending_clean_feed_cost
    }
+ [Ask the moot to bar Stonewake's seal from new public contracts until this claim is answered.]
    // ghostlight.action: request_seal_suspension
    // ghostlight.branch: seek_seal_suspension
    // ghostlight.intent: make_unanswered_injury_limit_future_public_authority
    {seal_pressure >= 4 && successor_liability >= 3:
        -> ending_seal_suspension
    - else:
        -> ending_seal_suspension_cost
    }
+ [Ask for divided-custody inspection before damages or final blame are fixed.]
    // ghostlight.action: request_joint_inquiry
    // ghostlight.branch: seek_divided_custody_inquiry
    // ghostlight.intent: preserve_a_falsifiable_case_across_foreign_and_greathold_keepers
    {sample_integrity >= 5 && public_witness >= 3:
        -> ending_joint_inquiry
    - else:
        -> ending_joint_inquiry_cost
    }
+ [Take a bounded settlement for carts and lost planting, but keep the civic claim open.]
    // ghostlight.action: accept_bounded_settlement
    // ghostlight.branch: seek_bounded_settlement
    // ghostlight.intent: buy_immediate_survival_without_selling_final_causation_or_future_redress
    {town_water >= 3 && hearing_trust >= 3:
        -> ending_bounded_settlement
    - else:
        -> ending_bounded_settlement_cost
    }

=== ending_clean_feed ===
// ghostlight.ending_label: bonded_clean_feed_success
// ghostlight.training_hook: essential_service_with_bounded_substitution
Stonewake's bond releases a half-load from a reserve house by dusk. The order names the lot, cart seals, meter limit, and hour at which the suspect feed must stop.

Brakka keeps three other towns pumping by dividing her clean reserve more finely than her sales promises ever did. Mira signs for less crystal than Tansy Ford needs and more protection than it had yesterday.

The claim stays open. Water moves under a smaller permission.
-> END

=== ending_clean_feed_cost ===
// ghostlight.ending_label: bonded_clean_feed_cost
// ghostlight.training_hook: remedy_named_without_supply
The moot orders clean replacement power. No reserve house on the docket can load it before tomorrow night.

Tansy Ford wins the shape of relief and loses the race to its cistern. Tavi's carts become the real order; the sealed parchment follows them home later, dry and correct.

Mira learns that a right without a supply schedule is a handsome bucket with no bottom.
-> END

=== ending_seal_suspension ===
// ghostlight.ending_label: seal_suspension_success
// ghostlight.training_hook: civic_authority_limited_by_unanswered_claim
Master Iven turns Stonewake's mark sideways on the public roll. Existing service may continue under meter. No new town, Hold, or district may accept the seal on public work until the sample chain is tested and Tansy Ford's interim order is met.

Brakka does not surrender her seal. She does lose its future tense.

The landing queue watches a distant field injury alter which promises may be made here tomorrow.
-> END

=== ending_seal_suspension_cost ===
// ghostlight.ending_label: seal_suspension_cost
// ghostlight.training_hook: punishment_without_proven_authority_chain
The bench records a warning but will not turn Stonewake's seal. The succession chain is argued; the injury remains plausible; the requested punishment reaches farther than the preserved case.

Brakka leaves with her new-contract right intact and her workshop name publicly bruised. Tansy Ford receives no cleaner water from either result.

On the slate, pressure has become reputation because procedure could not yet make it remedy.
-> END

=== ending_joint_inquiry ===
// ghostlight.ending_label: joint_inquiry_success
// ghostlight.training_hook: divided_custody_preserves_falsifiable_claim
One root half stays with Tansy Ford. One stays on Dorrin's roll. The fresh vane travels to a Rune College examiner under four marks: claimant, registry, Stonewake, and hearing moot.

Brakka opens the branch meter ledgers. Mira opens the field books. Neither side owns the only sequence.

The finding will arrive late. It will at least arrive through a door too narrow for one workshop to carry it away.
-> END

=== ending_joint_inquiry_cost ===
// ghostlight.ending_label: joint_inquiry_cost
// ghostlight.training_hook: evidence_chain_frays_under_service_pressure
The moot orders inspection, but too much has moved without matching custody. The root was divided late. The vane's rail journey lacks a clean mark. Pump hours were kept in a book the branch office calls local.

The inquiry begins as argument over evidence before it reaches argument over cause.

Tansy Ford keeps hauling water while experts decide which missing chalk line deserves to eat the season.
-> END

=== ending_bounded_settlement ===
// ghostlight.ending_label: bounded_settlement_success
// ghostlight.training_hook: immediate_restitution_without_claim_extinction
Stonewake's bond pays for water carts, the lost east planting, and an independent meter watch through harvest. Dorrin writes the dangerous sentence in full: payment settles those costs and does not clear the answering seal, close the sample inquiry, or release future injury.

Brakka signs because the limit is legible. Mira signs because the carts are real.

They share the last oatcake over separate custody trays. Reconciliation would be an ambitious name for it. Lunch is accurate.
-> END

=== ending_bounded_settlement_cost ===
// ghostlight.ending_label: bounded_settlement_cost
// ghostlight.training_hook: urgent_money_consumes_unresolved_claim
The settlement buys carts and seed, but the exception line is vague. Stonewake's clerk calls the payment final before Mira reaches the freight gate.

The civic claim remains on the roll. Its teeth are smaller. Tansy Ford survives the week by spending part of the leverage it needed for the year.

Below the landing, Cistern House Nine's pumps keep perfect time. Mira hates them a little for making continuity look innocent.
-> END
