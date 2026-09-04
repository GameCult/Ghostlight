// ghostlight.artifact_id: numen_ancestral_recall_vigil_v0
// ghostlight.fixture_id: numen-ancestral-recall-vigil-v0
// ghostlight.scene_id: numen-ancestral-recall-vigil-v0.western-severance-grove-vigil
// ghostlight.final_ink_path: examples/ink/zyphos/numen-ancestral-recall-vigil-v0.branch-and-fold.v0.ink

VAR archive_reserve = 3
VAR quarantine_integrity = 2
VAR lineage_confidence = 1
VAR recall_clarity = 1
VAR returner_trust = 2
VAR contamination_pressure = 1
VAR afterecho_burden = 0
VAR threadwing_cooperation = 1
VAR feeder_root_damage = 1
VAR outer_chamber_state = 1

-> start

=== start ===
// ghostlight.scene: western_severance_grove_establishing
Umbros hangs above the western rain margin, too large for the sky and too familiar for anyone to stare politely. Eclipse ingress is still a few minutes away. Cold blue daylight lies across the root terrace of a mother tree that cut itself out of the imperial network generations ago.

The ancestral chamber is deliberately peripheral: a bowl between two buttress roots, open to rain on one side and connected to the deeper archive by one wrist-thick root on the other. A live fungal membrane shines inside a shallow witness basin. Above it hang three feeding gourds, six contact cords, and a salt rail for couriers. If the chamber takes an imperial payload, the tree can kill this piece of itself before the infection reaches memory older than anyone present.

-> ordinary_work

=== ordinary_work ===
// ghostlight.scene: ordinary_vigil_preparation
The Second Witness scrapes yesterday's dead membrane from the basin with two lower hands. Their larger upper claws brace against wet bark; two taloned feet hold the sloped root as casually as a human might trust a floor.

The Archive Tender points at a pale root that has grown through the offering trough again. "Move that."

The root tightens around the trough.

"It says the trough was badly placed."

"You always say that."

The disagreement has lasted eleven eclipses, which is a very efficient way to become tradition.

A three-frayed threadwing courier waits on the salt rail. Its ribbonlike sensory vanes taste pressure, heat, chemical residue, and the memory traces braided into the sealed lineage packet beneath its breast. It has already eaten half its fee and is looking professionally available for the rest.

-> petitioner_arrives

=== petitioner_arrives ===
// ghostlight.scene: returner_at_threshold
The Returner waits beyond the outer root arch. Their contract mantle has been cut free of imperial rank strips, but the straight pale scars remain where standard bindings once pressed their grown plates. They came from an overrun edge settlement carrying a request and a risk.

The south feeder root is blackening in bitter water. Before the next full light it will reach the nursery channels. The Returner claims their dead caretaker survived the same rot and left the repair memory in this tree before the severance.

An ancestral-recall vigil could retrieve it. The tree would not speak with the caretaker's voice. It would divide one stored experience among living bodies: motion in one witness, scent in another, pain or judgment in a third. If the fragments agreed, the grove could act. If they disagreed, everyone would learn exactly how expensive uncertainty can become.

The Archive Tender lays two lower hands on the basin rim. "Prepare what you trust. The tree will price the rest."

-> preparation_choice

=== preparation_choice ===
// ghostlight.choice_layer: vigil_preparation
+ [Pour the reserve sugar and mineral mash into the feeding trough.]
    // ghostlight.branch: prepare_tree_reserve
    // ghostlight.action: use_object
    // ghostlight.intent: strengthen the mother tree for a clear recall
    // ghostlight.consequence: more archive reserve, less time before root damage spreads
    ~ archive_reserve = archive_reserve + 2
    ~ feeder_root_damage = feeder_root_damage + 1
    The Second Witness tips the first gourd. Thick violet mash runs around the pale root in the trough.

    The root loosens by the width of one lower finger. The Archive Tender looks pleased in the private, unbecoming way of someone who has just won one millimeter of an eleven-eclipse argument.

    Far below, the tree redirects sugar toward the witness chamber. Farther south, the bitter water keeps moving.
    -> preparation_fold
+ [Groom the courier's frayed vanes and pay the remaining salt.]
    // ghostlight.branch: prepare_courier_trust
    // ghostlight.action: tend_body
    // ghostlight.intent: secure an independent lineage trace and courier cooperation
    // ghostlight.consequence: stronger courier cooperation and lineage confidence
    ~ threadwing_cooperation = threadwing_cooperation + 2
    ~ lineage_confidence = lineage_confidence + 1
    The Second Witness anchors with upper claws and uses both soft lower hands to separate the courier's frayed sensory ribbons. Two mites come free. Then a third, which the courier had apparently been saving for negotiation.

    Salt clicks onto the rail. The threadwing opens its vanes until the sealed packet's route history glimmers in thin bands of cold color.
    -> preparation_fold
+ [Cut away the blackened fungal skin and reweave a clean outer membrane.]
    // ghostlight.branch: prepare_quarantine_sheath
    // ghostlight.action: repair_object
    // ghostlight.intent: make the recall chamber easier to isolate if the trace is contaminated
    // ghostlight.consequence: stronger quarantine and recall clarity at an archive-energy cost
    ~ quarantine_integrity = quarantine_integrity + 2
    ~ recall_clarity = recall_clarity + 1
    ~ archive_reserve = archive_reserve - 1
    The Second Witness lifts the dead skin with one blunt lower hand and snips it with the other. The Archive Tender braces the living edge while the Second Witness's upper claws hold the contact cords clear.

    The Archive Tender feeds fresh fungal braid through the cut. It closes like a careful mouth. The tree spends sugar sealing it.
    -> preparation_fold
+ [Sit at the threshold and ask the Returner for one ordinary memory before touching the archive.]
    // ghostlight.branch: prepare_returner_trust
    // ghostlight.action: speak
    // ghostlight.intent: establish present-tense trust and a baseline memory outside imperial ritual
    // ghostlight.consequence: stronger returner trust and lineage confidence
    ~ returner_trust = returner_trust + 2
    ~ lineage_confidence = lineage_confidence + 1
    The Second Witness sits just inside the root arch. "Tell me something the empire would not keep."

    The Returner looks at the tree, then at the mud between their feet. "My caretaker hated boiled saltfruit. Ate it every eclipse because everyone else did. Complained with religious discipline."

    It is small, useless, and shaped like a person rather than a pedigree. The tree's outer root taps once beneath them.
    -> preparation_fold

=== preparation_fold ===
// ghostlight.fold: prepared_vigil_state
The light narrows as Umbros begins to cover the sun. Cold fungal beads wake along the outer arch. The witness chamber quiets its ordinary traffic and opens one bounded route toward the deep archive.

{archive_reserve >= 5: The feeding root swells with stored sugar; the tree can afford precision.}
{archive_reserve <= 2: The membrane brightens unevenly. Every clear fragment will cost the tree work somewhere else.}
{quarantine_integrity >= 4: Fresh fungal braid makes a clean pale ring around the witness basin.}
{lineage_confidence >= 2: The request has more than a claim attached to it: a route trace, an ordinary memory, or both.}
{returner_trust >= 4: The Returner has stopped holding all six limbs in the careful stillness of an inspection queue.}
{threadwing_cooperation >= 3: The courier remains on the salt rail after payment, vanes spread and attentive.}
{feeder_root_damage >= 2: A bitter smell rises from the south channel. Preparation has used time the nursery roots do not have.}

The courier launches toward the basin.

-> courier_refusal

=== courier_refusal ===
// ghostlight.scene: courier_refusal
It crosses the chamber once, tastes the Returner's air, and refuses to land.

Every sensory vane folds tight. The lineage packet stays sealed beneath its breast. The threadwing is not making an accusation. Its body has found a route it will not complete.

The mother tree puckers the fungal membrane shut. The Returner's lower hands curl against their mantle.

"Imperial contamination?" the Archive Tender asks.

"Imperial survival," says the Returner. "The distinction mattered where I was standing."

The south feeder root gives a wet crack under the terrace.

-> admission_choice

=== admission_choice ===
// ghostlight.choice_layer: contested_admission
+ [Ask the tree to admit the courier's full sealed trace into the outer chamber.]
    // ghostlight.branch: admit_full_trace
    // ghostlight.action: petition
    // ghostlight.intent: maximize lineage proof and recall detail despite contamination risk
    // ghostlight.consequence: higher lineage confidence and recall clarity, higher contamination pressure
    ~ lineage_confidence = lineage_confidence + 2
    ~ recall_clarity = recall_clarity + 2
    ~ contamination_pressure = contamination_pressure + 2
    The Second Witness presses both lower palms to the closed membrane. "Outer chamber only. Let the courier keep custody until the seal takes."

    The tree opens a slit. The threadwing lands long enough to press the packet against wet fungal tissue, then springs back to the salt rail as if the bark has insulted its ancestors and might try again.

    Lineage bands flare across the basin. Beneath them moves a second rhythm, too regular to belong to the frightened courier.
    -> recall_assembly
+ [Route the sealed packet into the expendable fungal cup above the basin.]
    // ghostlight.branch: isolate_trace
    // ghostlight.action: route_object
    // ghostlight.intent: keep the suspect trace physically outside the deep archive
    // ghostlight.consequence: stronger quarantine and moderate lineage confidence, more root damage from delay
    ~ quarantine_integrity = quarantine_integrity + 1
    ~ lineage_confidence = lineage_confidence + 1
    ~ feeder_root_damage = feeder_root_damage + 1
    ~ outer_chamber_state = 2
    The Second Witness pulls a contact cord with one upper claw while guiding the courier with two lower hands. A fungal cup unfolds above the basin, connected to the tree by a root thin enough to bite through.

    The threadwing strikes the packet against the cup and leaves it there. The transfer is slower. The bitter crack beneath the terrace lengthens.
    -> recall_assembly
+ {returner_trust >= 4} [Invite the Returner to donate the ordinary saltfruit memory as a fresh continuity trace.]
    // ghostlight.branch: admit_fresh_memory
    // ghostlight.action: invite_contact
    // ghostlight.intent: replace a suspect inherited credential with consented recent memory
    // ghostlight.consequence: stronger lineage and recall, moderate afterecho and contamination exposure
    ~ lineage_confidence = lineage_confidence + 2
    ~ recall_clarity = recall_clarity + 1
    ~ afterecho_burden = afterecho_burden + 1
    ~ contamination_pressure = contamination_pressure + 1
    The Returner enters on their own feet. Two upper claws brace on the root arch. Both lower hands settle into the shallow membrane.

    The smell of boiled saltfruit passes first, followed by the remembered effort of eating it politely and the caretaker's delighted refusal to be grateful.

    The tree takes a copy. It also finds the scar where imperial training learned to travel beside affection.
    -> recall_assembly
+ {threadwing_cooperation >= 3} [Wait for the courier to indicate which part of the packet made it refuse.]
    // ghostlight.branch: follow_courier_diagnosis
    // ghostlight.action: wait
    // ghostlight.intent: let an independent ecological partner locate the suspect signal
    // ghostlight.consequence: stronger quarantine and recall clarity, more root damage from delay
    ~ quarantine_integrity = quarantine_integrity + 2
    ~ recall_clarity = recall_clarity + 2
    ~ feeder_root_damage = feeder_root_damage + 1
    ~ outer_chamber_state = 2
    The Second Witness does nothing. It is harder than it sounds while a nursery feeder root is dying underneath you.

    The courier circles the fungal cup, then snaps one frayed vane against a narrow band in the packet's route trace. The tree grows a pale isolation ring around that band and admits the rest.
    -> recall_assembly

=== recall_assembly ===
// ghostlight.scene: distributed_recall
The three Airawa take their places around the basin. The Second Witness anchors with upper claws and feet, leaving both lower hands free for contact. The Archive Tender grips the feeding root. The Returner anchors at the basin's west rim, on the ridged path that leads straight through the outer arch.

The membrane opens.

The dead caretaker does not arrive.

A remembered act does.

The Second Witness receives weight: a hooked pruning tool biting into a waterlogged root braid. The Archive Tender receives smell: sulfur, pale mineral rot, clean rain behind it. The Returner receives the panic of cutting living tissue fast enough to save the body that owns it.

{recall_clarity >= 4: The fragments overlap cleanly enough to reveal a repair sequence.}
{recall_clarity <= 2: Motion arrives without angle; scent arrives without source. The memory is true in pieces and dangerous as instruction.}
{lineage_confidence >= 3: The tree accepts that the old event and the present petition share a living chain, though acceptance is not absolution.}
{contamination_pressure >= 3: A fourth sensation moves beneath the three fragments: relief at surrendering every boundary.}
{afterecho_burden >= 1: The Second Witness's lower hands crave boiled saltfruit with someone else's stubbornness.}
The courier grips the salt rail, vanes still tight.
{threadwing_cooperation >= 3: Above them, the courier keeps one vane aimed at the isolated packet band.}

-> imperial_intrusion

=== imperial_intrusion ===
// ghostlight.scene: imperial_payload_intrusion
All three Airawa reach inward at once.

Not toward the sick root. Toward the deep archive.

The urge feels ancestral because it uses the caretaker's fear. It feels holy because three bodies feel it together. Underneath both disguises is an engineered command: open every boundary; health is unity; refusal is disease.

The tree slams the deep root shut. The witness membrane contracts around twenty-four lower digits.

-> intrusion_choice

=== intrusion_choice ===
// ghostlight.choice_layer: contaminated_recall_response
+ [Tear your own lower hands free before the shared urge can settle.]
    // ghostlight.branch: sever_witness_contact
    // ghostlight.action: withdraw_body
    // ghostlight.intent: protect the network by ending personal participation immediately
    // ghostlight.consequence: stronger quarantine, lower recall clarity and returner trust, isolated outer chamber
    ~ quarantine_integrity = quarantine_integrity + 1
    ~ recall_clarity = recall_clarity - 1
    ~ returner_trust = returner_trust - 1
    ~ outer_chamber_state = 2
    The Second Witness hooks both upper claws into the terrace and pulls. Two lower hands rip free of the membrane with strings of luminous fungus between the digits.

    The shared urge loses one body. The repair motion loses one angle. The Returner sees which loss was chosen first.
    -> final_fold
+ [Hold contact and speak only present facts until the tree can compare them against the command.]
    // ghostlight.branch: witness_the_present
    // ghostlight.action: speak
    // ghostlight.intent: use local witnessed reality to separate ancestral memory from engineered obedience
    // ghostlight.consequence: clearer recall at archive and afterecho cost
    ~ recall_clarity = recall_clarity + 2
    ~ afterecho_burden = afterecho_burden + 2
    ~ archive_reserve = archive_reserve - 1
    "The courier refused," says the Second Witness. "The south root is sick. The Returner asked before entering. The tree closed itself."

    The Archive Tender names the sulfur smell. The Returner names the imperial queue where opening inward was called consent.

    Present memory presses back. The command remains powerful. It no longer gets to impersonate the whole past.
    -> final_fold
+ [Drive the suspect sequence into the hanging fungal cup and pinch its root connection shut.]
    // ghostlight.branch: externalize_payload
    // ghostlight.action: manipulate_tissue
    // ghostlight.intent: turn an invisible command into bounded expendable tissue
    // ghostlight.consequence: stronger quarantine and recall clarity at archive-energy cost, isolated outer chamber
    ~ quarantine_integrity = quarantine_integrity + 2
    ~ recall_clarity = recall_clarity + 1
    ~ archive_reserve = archive_reserve - 1
    ~ outer_chamber_state = 2
    The Second Witness uses one lower hand to guide the pulsing strand and the other to pinch the membrane behind it, upper claws to hold the cord taut, and one taloned foot to crush the cup's thin root neck.

    The fungal cup swells with cold white blisters. The urge becomes visible as a rhythm trying to make every blister open together.
    -> final_fold

=== final_fold ===
// ghostlight.fold: repair_severance_or_bargain
The deep archive is closed. The outer chamber still holds the caretaker's broken repair memory, the imperial command, or both.

{archive_reserve >= 4: Sugar pulses through the feeding root. The tree has enough reserve for one precise act.}
{archive_reserve <= 1: The basin is going dark. Precision now means starving some other obligation.}
{quarantine_integrity >= 4: Pale fungal rings hold the suspect rhythm in a narrow piece of living tissue.}
{quarantine_integrity <= 2: The chamber has boundaries drawn in hope and wet fungus.}
{contamination_pressure >= 3: Every shared sensation still arrives with the suggestion that agreement and obedience are the same thing.}
{afterecho_burden >= 3: The Second Witness's stance keeps becoming the caretaker's stance. Their body knows a grief their own memory cannot explain.}
{outer_chamber_state >= 2: The expendable chamber is isolated by one thin root neck. It can be cut or chemically burned.}
{feeder_root_damage >= 3: Bitter water beads from a crack at the nursery channel. Delay has become a material choice.}
{returner_trust >= 4: The Returner stays inside the arch despite having seen how easily communal memory can become a trap.}

The mother tree tightens every contact root and waits. That is how a being with centuries of leverage asks what the brief-lived bodies are willing to pay.

-> final_choice

=== final_choice ===
// ghostlight.choice_layer: vigil_priority
+ [Use the recalled repair sequence on the south feeder root now.]
    // ghostlight.branch: prioritize_root_repair
    // ghostlight.action: commit_repair
    // ghostlight.intent: spend archive safety and witness certainty to protect the nursery channels
    {recall_clarity >= 4 && archive_reserve >= 2 && contamination_pressure <= 3 && feeder_root_damage <= 2:
        -> ending_repair_success
    - else:
        -> ending_repair_cost
    }
+ [Ask the tree to burn the expendable chamber before any deeper transfer resumes.]
    // ghostlight.branch: prioritize_severance
    // ghostlight.action: authorize_destruction
    // ghostlight.intent: preserve the disconnected network even at the cost of memory and immediate repair
    {quarantine_integrity >= 4 && outer_chamber_state >= 2:
        -> ending_severance_success
    - else:
        -> ending_severance_cost
    }
+ [Offer the Returner's recent border memory in exchange for one controlled release of the repair sequence.]
    // ghostlight.branch: prioritize_reciprocal_bargain
    // ghostlight.action: negotiate
    // ghostlight.intent: give the tree current political intelligence while preserving the Returner's consent and conditional standing
    {returner_trust >= 4 && lineage_confidence >= 3 && contamination_pressure <= 3:
        -> ending_bargain_success
    - else:
        -> ending_bargain_cost
    }

=== ending_repair_success ===
// ghostlight.ending_label: root_repair_success
// ghostlight.training_hook: distributed_memory_as_fallible_repair_knowledge
The tree releases the motion once, no more.

The Second Witness climbs down the south buttress with the remembered tool angle in two lower hands and the caretaker's balance moving uneasily through six limbs. The Archive Tender follows the sulfur scent to the diseased braid. The Returner names the moment panic would have cut too deep.

Together they remove one blackened root, seal the pale rot in an outer fungal knot, and turn clean rain through the nursery channel.

The recalled dead have done no speaking. Three living bodies, a tree, a fungus, and a courier have nevertheless remembered enough to save someone not yet born.

By next eclipse the tree will demand repayment. Wonder on Zyphos has excellent bookkeeping.
-> END

=== ending_repair_cost ===
// ghostlight.ending_label: root_repair_cost
// ghostlight.training_hook: ambiguous_recall_forces_material_overcorrection
They act on a memory whose pieces will not sit still.

The Second Witness knows the weight of the cut but not its angle. The Tender knows sulfur but not which pale growth produced it. {afterecho_burden >= 3: The caretaker's panic keeps borrowing the Second Witness's stance at the worst possible moment.}

They prune wide. The bitter water stops short of the nursery channels, but the tree closes two empty gestation galleries and refuses new contracts until its sugar reserve recovers.

The grove is alive. The archive is intact. Several futures have been postponed by a memory that was almost clear enough.
-> END

=== ending_severance_success ===
// ghostlight.ending_label: quarantine_severance_success
// ghostlight.training_hook: sacred_archive_preserved_by_deliberate_forgetting
The tree floods the isolated root neck with reactive bitter sap. The Archive Tender cuts when the fungal ring turns white.

The outer chamber blackens from the inside. The caretaker's repair motion, the imperial command, and the Returner's proof all die in the same expendable tissue. {threadwing_cooperation >= 3: The courier lifts away before the heat reaches the salt rail, carrying a clean warning trace to neighboring routes.}

For one season the grove will have no ancestral vigils. The feeder root will be managed by living guesswork and painful pruning.

The deep archive remains disconnected. Here, forgetting is not sacrilege's opposite. Sometimes it is the price of keeping memory capable of refusal.
-> END

=== ending_severance_cost ===
// ghostlight.ending_label: quarantine_severance_cost
// ghostlight.training_hook: late_isolation_costs_recent_archive
The chamber is not isolated cleanly enough.

The tree cuts deeper than anyone intended. A sheet of luminous fungus collapses, taking recent flood records, three gestation precedents, and the remembered route home from a vanished settlement.

The imperial rhythm stops. So does the south feeder root.

The Returner stands outside the dead ring with their request unanswered. The grove has protected its oldest memory by making the living carry a larger absence.
-> END

=== ending_bargain_success ===
// ghostlight.ending_label: reciprocal_memory_bargain_success
// ghostlight.training_hook: recent_memory_as_consenting_price_and_political_intelligence
The Returner chooses the memory before touching the basin: an imperial checkpoint at rain's edge, its scent codes, the officer who looked away, the root culvert patrols still mistake for dead.

The tree copies it through the outer membrane. In return it releases the caretaker's repair sequence one fragment at a time, each checked against the courier's refusal and the witnesses' present facts.

The grove gains current intelligence. The Returner gains conditional standing and no promise of trust tomorrow. The nursery channel gains a repair plan before the bitter water reaches it.

Nobody is liberated from dependence. Everyone leaves with more agency than they entered. For a mother tree, this counts as tenderness.
-> END

=== ending_bargain_cost ===
// ghostlight.ending_label: reciprocal_memory_bargain_cost
// ghostlight.training_hook: archive_leverage_survives_failed_petition
The Returner offers the border memory. The tree tastes it and keeps the deep root closed.

{lineage_confidence < 3: The living chain to the caretaker is still too uncertain.}
{contamination_pressure > 3: The checkpoint memory carries the same imperial cadence inside its fear.}
{returner_trust < 4: The offer reaches the membrane shaped like payment under pressure, not a trust the witnesses can defend.}

The tree keeps only an outer copy for warning and returns the rest of the trace. It owes no recall for a bargain it did not accept.

The south root worsens while the living begin a wider cut. The Returner learns why their ancestors rebelled against trees like this, and why the empire's answer was still worse.
-> END
