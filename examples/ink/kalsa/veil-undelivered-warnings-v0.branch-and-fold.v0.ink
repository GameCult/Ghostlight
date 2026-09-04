// ghostlight.artifact_id: kalsa_veil_undelivered_warnings_branch_fold_v0
// ghostlight.fixture_id: veil-undelivered-warnings-v0
// ghostlight.scene_id: veil-undelivered-warnings-v0.custodian-gallery-disclosure
// ghostlight.final_ink_path: examples/ink/kalsa/veil-undelivered-warnings-v0.branch-and-fold.v0.ink

VAR pale_shutters = 2
VAR seal_integrity = 2
VAR technical_context = 0
VAR mortuary_context = 0
VAR copy_count = 0
VAR shared_record = 0
VAR crew_heat = 1
VAR warning_scope = 0
VAR route_braced = 0

-> start

=== start ===
The Custodian Gallery is the driest room under Low Sere, which is not the same as being cool.

Hot stone presses through Sera Venn's knees. Layered service folios sag on shelves around a black calibration chest. A wall diagram shows seven working baffles and one isolation shutter in strokes polished by generations of pointing fingers.

Two pale shutter marks remain on the repeater gauge. One was lost on the way down.

-> gallery_routine

=== gallery_routine ===
Sera, Low Sere's provisional intake custodian, copies the gauge reading onto the chest's three octal wheels. Ressa Orr, the independent technical witness, watches the teeth align without helping; a witness who helps too early becomes an accomplice with excellent posture.

Tavi Kes, speaking for the kin of the two dead workers, dries their name cloths across the expedition recorder's pack frame. The recorder sharpens charcoal, counts blank sheets, and asks whether the gallery is always this hot.

"No," Sera says. "Sometimes it admits it."

The third wheel settles. Somewhere beyond the dry wall, a relief valve knocks once.

-> chest_open

=== chest_open ===
The mechanical refusal releases.

Inside lie the black ceramic Ashen Measure, Teren Vey's last handoff folio, a pouch of calibration beads, and two palm-sized folds of ashproof service cloth. Cheap grey wax seals each fold around a split-reed routing tag.

One tag is addressed to lower-step water witnesses and the maintenance-platform lead. The other names the lower-step feed and injury stores. Both bear Teren's later counterseal.

Neither bears the mark that would mean a lower-step witness received it.

-> seal_choice

=== seal_choice ===
// ghostlight.choice_layer: preserve_or_open
+ [Rub both routing tags onto blank cloth before opening the wax.]
    // ghostlight.action_label: use_object
    // ghostlight.branch_label: preserve_interception_marks
    ~ seal_integrity = seal_integrity + 2
    ~ shared_record = shared_record + 1
    ~ crew_heat = crew_heat + 1
    ~ pale_shutters = pale_shutters - 1
    Sera lays thin cloth over each brittle tag. Charcoal passes across the raised fibres until two interrupted routes appear: a Cistern House receipt notch ending beside the basin table's dry-store tally, and a custodian-review slit ending before the lower-step mark.

    It takes long enough for sweat to make the charcoal tacky.

    The repeater gauge loses another pale mark.
    -> notices_read
+ [Break both seals now, before the room spends another breath on procedure.]
    // ghostlight.action_label: break_object
    // ghostlight.branch_label: break_seals_for_speed
    ~ seal_integrity = seal_integrity - 1
    ~ warning_scope = warning_scope + 1
    Sera puts a thumbnail under the first wax thread and pulls. The split reed snaps with it.

    Ressa makes a sound that is too small to be called swearing by anyone who has not worked with archives.

    "We still have the words," Sera says.

    "Yes," says Ressa. "And fewer answers about who stopped them."
    -> notices_read
+ [Ask Tavi to bind the seals to the two name cloths before either is opened.]
    // ghostlight.action_label: transfer_object
    // ghostlight.branch_label: attach_mortuary_witness
    ~ mortuary_context = mortuary_context + 2
    ~ seal_integrity = seal_integrity + 1
    ~ crew_heat = crew_heat + 1
    Tavi loops one name cloth under each wax thread, never covering the route marks.

    "Jori and Pell are not evidence decorations," Tavi says. "If the paper travels, the reason it matters travels."

    Sera waits for Tavi's witness cut before opening the folds.
    -> notices_read
+ [Give the closed folds to Ressa for a technical comparison of wax, hand, and routing marks.]
    // ghostlight.action_label: show_object
    // ghostlight.branch_label: attach_technical_witness
    ~ technical_context = technical_context + 2
    ~ seal_integrity = seal_integrity + 1
    ~ crew_heat = crew_heat + 1
    Ressa holds each fold near the wall lamp, then near the dry vent, watching old wax and newer counterseal soften at different rates.

    "Two holds," she says. "Neither made when Teren locked the chest. His seal is later. That is all the wax knows."

    It is not all Sera wants. The wax declines promotion.
    -> notices_read

=== notices_read ===
// ghostlight.fold: the_two_texts
The first notice orders the maintenance platform cleared at the first hammer of the seventh relief baffle. A household work-watch, it says, is not the interval.

The second orders the lower-step feed closed after a pressure surge, sleeping pallets moved above the third water mark, and the first ash, water, and burn cloth preserved for comparison.

{seal_integrity >= 4: The rubbed routing marks survive beside the words. The notice can accuse a route without pretending to identify the hand that stopped it.}
{seal_integrity <= 1: The words survive. One broken reed has taken the interception point with it, leaving every interested office room to blame the next.}
{technical_context >= 2: Ressa can distinguish the earlier hold marks from Teren's counterseal, but not name their makers.}
{mortuary_context >= 2: Tavi keeps one finger on each name cloth while the injury instruction is read. Delayed care remains attached to the two deaths instead of becoming a footnote.}

The recorder lifts three blank sheets.

"Which failure do you want preserved first?" they ask.

-> record_choice

=== record_choice ===
// ghostlight.choice_layer: choose_the_record
+ [Dictate both notices in full and make the compact's three copies here.]
    // ghostlight.action_label: speak
    // ghostlight.branch_label: copy_both_notices
    ~ copy_count = 3
    ~ shared_record = shared_record + 2
    ~ warning_scope = 2
    ~ crew_heat = crew_heat + 1
    ~ pale_shutters = pale_shutters - 1
    Sera dictates every line, including both unsigned holds and Teren's later counterseal.

    Ressa corrects one interval mark. Tavi makes the recorder repeat "burn cloth" instead of shortening it to "injury evidence."

    By the third copy, the room has become a kiln with filing habits.

    The last pale shutter mark gutters on the repeater.
    -> disclosure_fold
+ [Copy the platform-clearance notice first; the technical sequence can still kill people now.]
    // ghostlight.action_label: speak
    // ghostlight.branch_label: privilege_technical_warning
    ~ copy_count = 2
    ~ technical_context = technical_context + 2
    ~ warning_scope = 1
    Sera dictates the first hammer, the seventh relief baffle, and the warning against a household work-watch.

    The later care notice remains folded under her palm.

    Tavi looks at the hidden sheet. "A person can survive the valve and still be omitted to death."
    -> disclosure_fold
+ [Copy the lower-step closure and care notice first; exposed households need an order they can use.]
    // ghostlight.action_label: speak
    // ghostlight.branch_label: privilege_household_warning
    ~ copy_count = 2
    ~ mortuary_context = mortuary_context + 1
    ~ warning_scope = 2
    Sera dictates the feed closure, the third water mark, and the samples that should have been kept.

    Ressa taps the covered platform notice. "This may save the Warm Steps and leave the next operator ignorant of the first hammer."

    "Then the next operator can complain alive," Tavi says.
    -> disclosure_fold
+ [Wrap the originals together and make one fast route copy without interpreting either hold.]
    // ghostlight.action_label: withhold_judgment
    // ghostlight.branch_label: make_route_copy_only
    ~ copy_count = 1
    ~ shared_record = shared_record - 1
    ~ warning_scope = 2
    ~ route_braced = route_braced + 1
    The recorder makes one blunt copy: what to clear, what to close, what to preserve, and which offices never marked receipt.

    No accusation. No comparison. A warning light enough to carry at a run.

    Sera uses the saved time to wedge the archive-crawl latch against the next pressure knock.
    -> disclosure_fold

=== disclosure_fold ===
// ghostlight.fold: evidence_is_not_delivery
The notices now exist in the gallery as more than sealed possibility.

{shared_record >= 2: Three copies lie apart on the dry floor: one for Low Sere, one for custodian review, one for the crew. No single dropped body can erase all of them.}
{shared_record < 0: Only the fast route copy can travel without risking both originals. It carries action cleanly and provenance badly.}
{technical_context >= 2: Ressa can defend the first-hammer distinction before a review, if she reaches that review.}
{mortuary_context >= 2: Tavi can show that the aftercare notice concerns living injury and dead claim together, if the name cloths reach the surface.}
{warning_scope >= 2: The traveling words can tell lower-step households both what to close and what evidence to preserve.}
{warning_scope == 1: The traveling words can prevent one technical repetition while leaving the aftercare omission intact.}

Another knock travels through the wall. The archive crawl leads through the Memorial Sump to the Relief Crawl. The longer return passes the Eight Screens and the corroded Settling Walk.

Someone can carry a warning now. Whoever leaves takes one office out of the work still waiting at the Intake Crown.

Above, Bel Orra is the lower-step water witness able to mark actual receipt. Maro Seln controls the surface feed wheel. Bel can make the notice public to the exposed rooms; Maro can act upon it. Neither can do the other's part by holding the paper first.

-> courier_choice

=== courier_choice ===
// ghostlight.choice_layer: choose_the_disclosure_path
+ [Ask Tavi to carry a copy through the Memorial Sump and up the Relief Crawl.]
    // ghostlight.action_label: speak
    // ghostlight.branch_label: send_tavi_with_names
    {mortuary_context >= 2 && pale_shutters >= 1:
        Tavi knots the route copy between the two name cloths.

        "If something below answers," Tavi says, "I will not call it Jori merely because I need courage."

        Tavi enters the archive crawl alone. The next knock passes without becoming a jet.
        -> ending_tavi_delivery
    - else:
        Tavi takes the copy and both names into the crawl. The gallery hears one spoken name, then the relief line opens like a furnace door.
        -> ending_tavi_cost
    }
+ [Ask Ressa to take the longer route through the Eight Screens and Settling Walk.]
    // ghostlight.action_label: speak
    // ghostlight.branch_label: send_ressa_with_comparison
    {technical_context >= 2 && seal_integrity >= 3 && pale_shutters >= 1:
        Ressa wraps one rubbing, one copy, and no original inside her coat.

        "I can tell them what the marks support," she says. "I cannot make them enjoy it."

        She leaves by the narrow return toward the Eight Screens.
        -> ending_ressa_delivery
    - else:
        Ressa studies the damaged route mark and the mist beginning to leak under the service door.

        "If I carry this alone, I become the missing comparison and the only witness. That is not disclosure. It is a more educated monopoly."
        -> ending_ressa_cost
    }
+ [Ask the expedition recorder and delving lead to send a copy while every specialist stays below.]
    // ghostlight.action_label: transfer_object
    // ghostlight.branch_label: send_recorder_fast
    {copy_count >= 1 && seal_integrity >= 3 && route_braced >= 1:
        The recorder straps one copy flat beneath the pack frame and tests the archive-crawl wedge.

        "Three copies, one bad route," they say. "Almost respectable odds."

        They vanish into the crawl before the pressure can answer.
        -> ending_recorder_delivery
    - else:
        The recorder takes the best copy available. The route has no witness marks, the latch no brace, or the other copies still sit in one room.

        A pulse hits while they cross the Relief Crawl.
        -> ending_recorder_cost
    }
+ [Keep the crew together, carry every record to the Intake Crown, and disclose only after the machine is stable.]
    // ghostlight.action_label: wait
    // ghostlight.branch_label: keep_disclosure_with_repair
    {pale_shutters >= 1 && technical_context >= 2:
        Sera packs the notices beside the Measure. Ressa keeps stop authority. Tavi keeps the mortuary hold. The recorder keeps all three copies apart.

        Nobody gets the warning quickly. Nobody is sent alone to make speed look clean.
        -> ending_repair_first
    - else:
        Sera packs the evidence and keeps every living witness in the gallery.

        The remaining shutter mark goes dark before they reach the Crown.
        -> ending_disclosure_too_late
    }

=== ending_tavi_delivery ===
// ghostlight.ending_label: mortuary_delivery_success
// ghostlight.training_hook: names_carry_warning_without_claiming_identity
Tavi crosses the Memorial Sump with the names visible and the claim bounded.

At the Cistern House, Bel Orra receives the copy before Maro can turn it into a private table matter. Lower-step households begin moving pallets above the third water mark. Tavi keeps the original deaths attached to the care order and refuses to say which dead presence, if any, let the route pass.

The Intake Crown has lost its mortuary witness. Sera will have to stop if the safe repair disturbs the sump.

The warning arrives. The dead do not become a delivery service.
-> END

=== ending_tavi_cost ===
// ghostlight.ending_label: mortuary_delivery_death
// ghostlight.training_hook: disclosure_route_kills_its_carrier
The relief pulse finds Tavi between the archive crawl and the sump grate.

The name cloths reach the upper hatch on steam. Tavi does not.

Sera can still carry the notices out later. They will now explain a third death caused after everyone in the gallery knew exactly why warning mattered.

There is no honest copy in which that becomes a noble price.
-> END

=== ending_ressa_delivery ===
// ghostlight.ending_label: technical_delivery_success
// ghostlight.training_hook: comparison_reaches_households_at_operational_cost
Ressa reaches the Cistern House by the long route with the seals legible and the interval distinction intact.

Bel Orra hears both notices in words she can use. Maro closes the lower-step feed. Teren's appeal gains evidence, not victory.

Below, Sera reaches the Intake Crown without the one independent specialist entitled to stop her there. The settlement is safer. The next act is lonelier and more dangerous.
-> END

=== ending_ressa_cost ===
// ghostlight.ending_label: technical_delivery_refused
// ghostlight.training_hook: witness_refuses_to_become_single_authority
Ressa refuses the route.

She sets the copy between Sera and Tavi. "A comparison that removes every other witness is how this chest happened."

The crew remains capable of repair, but nobody above knows to clear the lower feed. The notices have been opened and not disclosed. Every office in the gallery can now blame urgency for a choice it made together.
-> END

=== ending_recorder_delivery ===
// ghostlight.ending_label: distributed_record_delivery_success
// ghostlight.training_hook: redundant_copies_survive_dangerous_delivery
The recorder reaches the black pressure door with one copy.

{copy_count >= 3: Two further copies remain below under separate hands.}
{copy_count < 3: Both originals remain below under separate hands; the delivered copy is the only one light enough to run with.}

Bel Orra reads the actionable lines aloud while Maro orders the feed closed. The surface receives no single accusation it can conveniently punish and no excuse to say the warning was too complicated to act upon.

The recorder survives because the route was braced, not because record keepers are protected by narrative importance.
-> END

=== ending_recorder_cost ===
// ghostlight.ending_label: record_carrier_death
// ghostlight.training_hook: thin_provenance_and_unbraced_route
The pulse lifts the recorder off the crawl floor.

The copy cooks into a black curl inside the pack frame.

{copy_count >= 3: Two further copies remain in the gallery under separate hands, but no warning reaches the surface.}
{copy_count < 3: The originals remain in the gallery, and so does everyone with authority to explain them.}

At the surface, no warning arrives. Below, the surviving record will have to name the recorder as a casualty of its own delivery plan.
-> END

=== ending_repair_first ===
// ghostlight.ending_label: repair_first_bounded_success
// ghostlight.training_hook: collective_capacity_preserved_at_disclosure_delay
The crew reaches the Intake Crown with every stop authority and every copy intact.

The Measure fits. The first hammer is treated as a material event, not a household interval. Low Sere gets time enough for a witnessed return.

When Bel Orra finally receives the notices, she has already spent another work-watch carrying water through rooms nobody warned her to clear. Repair preserved the disclosure path. It did not refund the delay.
-> END

=== ending_disclosure_too_late ===
// ghostlight.ending_label: disclosure_after_rupture
// ghostlight.training_hook: intact_archive_cannot_rescue_people_after_delay
The lower cistern ruptures before the notices leave the gallery.

The originals survive. The seals survive. Ressa, Tavi, Sera, and the recorder survive long enough to agree on exactly what was not delivered.

Above them, the Warm Steps fill with scalding grey water. An intact archive is carried toward new names.
-> END
