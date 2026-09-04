// ghostlight.artifact_id: ember_convoy_default_branch_fold_v0
// ghostlight.fixture_id: ember-convoy-default-v0
// ghostlight.scene_id: ember-convoy-default-v0.mare-crisium-continuity-default
// ghostlight.final_ink_path: examples/ink/aetheria/ember-convoy-default-v0.branch-and-fold.v0.ink

VAR mutual_aid = 1
VAR hatch_state = 1
VAR seal_margin = 2
VAR departure_margin = 3
VAR unsupported_air = 1
VAR manifest_leverage = 1
VAR mass_credit = 1
VAR audit_exposure = 1
VAR contract_debt = 0
VAR mika_seal_skill = 0
VAR davin_trust = 1
VAR queue_crossed = 0

-> start

=== start ===
Mare Crisium Receiving Cylinder lies on the lunar surface like a tin loaf somebody remembered to pressurize. Its repair shops and freight gallery serve three Horizon Ventures mining routes. Its inhabitants call it Crisium because calling it Receiving Cylinder makes breakfast taste contractual.

-> gallery_establishing

=== gallery_establishing ===
Inside Transfer Gallery Two, gray floor grating runs straight from the habitatward pressure hatch to the dockward collar. A recessed manifest desk sits along the right wall. A creditor's glass audit booth faces it from the left. Beyond the collar window, the last insured shuttle waits on the pad under hard white floodlights.

-> routine_hatch

=== routine_hatch ===
Seal technician Sera Madani begins every watch with Hatch MCR-2. The panel turns green after one pull of the waist-high dog handle. The lower seal seats only after she braces a boot against the grating and drives the handle through a second stroke.

Green means the computer has become optimistic. The second stroke means the air stays indoors.

Sera marks two strokes on her paper cuff. The official checklist provides one box.

-> routine_people

=== routine_people ===
Mika Osei arrives habitatward with a food-loop cart and two warm lentil cakes balanced under a mesh cover. She maintains the Ewan Hart microbial beds as a subcontractor, which means the cylinder depends on her in a manner payroll can deny with excellent typography.

"Breakfast," Mika says. "One for you, one for whichever creditor still has a mouth."

Davin Rook, Horizon's manifest clerk, lifts his head above the recessed desk. "Creditors eat signatures. Crumbs clog filters. Both are chargeable."

Nara Venn watches from the audit booth. She represents the lunar creditors financing the next cargo. Her slate can keep the shuttle's launch insured or turn it into an expensive sculpture.

For seven minutes, this is an ordinary morning: a bad seal, a food cart, a clerk pretending jokes do not count as unlicensed morale.

-> routine_choice

=== routine_choice ===
// ghostlight.choice_layer: morning_maintenance
+ [Enter the second stroke in the maintenance log, even though it can trigger an insurer inspection.]
    // ghostlight.action_label: record_evidence
    // ghostlight.branch_label: log_second_stroke
    ~ seal_margin = seal_margin + 1
    ~ audit_exposure = audit_exposure + 2
    ~ departure_margin = departure_margin - 1
    Sera adds a second box by hand and initials it.

    Davin looks at the new mark. "You have improved the form past its design tolerance."

    Nara's slate notices. The inspection timer begins quietly, the way expensive trouble prefers.
    -> routine_fold
+ [Show Mika the brace point and let her drive the dog handle through its second stroke.]
    // ghostlight.action_label: teach_physical_skill
    // ghostlight.branch_label: teach_mika_hatch
    ~ mutual_aid = mutual_aid + 2
    ~ mika_seal_skill = 1
    ~ audit_exposure = audit_exposure + 1
    ~ seal_margin = seal_margin + 1
    Mika plants one boot beside Sera's chalk mark and leans into the handle. The second stroke lands with a deep metal clunk.

    "Very advanced," Mika says. "Door closed by closing door."

    Sera redraws the chalk mark where Mika can see it under bad light.
    -> routine_fold
+ [Trade tonight's maintenance bonus for two household oxygen allotments and send them to Mika's sleeping bay.]
    // ghostlight.action_label: transfer_resource
    // ghostlight.branch_label: pool_oxygen_scrip
    ~ unsupported_air = unsupported_air + 2
    ~ mutual_aid = mutual_aid + 1
    Sera touches cuffs with Davin. Her bonus disappears from one line. Two oxygen allotments appear beside Mika's bay number.

    "I object to generosity before breakfast," Davin says. "It leads to arithmetic."

    Mika tears one lentil cake in half and gives him the larger piece.
    -> routine_fold
+ [Stage the outbound tool crates beside the dock collar before anyone can revise their mass codes.]
    // ghostlight.action_label: move_objects
    // ghostlight.branch_label: protect_departure_margin
    ~ departure_margin = departure_margin + 2
    ~ davin_trust = davin_trust + 1
    ~ mutual_aid = mutual_aid - 1
    Sera rolls the tool crates dockward and locks their wheels inside the yellow manifest grid.

    Davin approves the sequence without looking at her. It saves twelve minutes later and helps nobody who cannot fit inside a cargo code.

    Mika watches the cart pass her food-loop spares.
    -> routine_fold

=== routine_fold ===
// ghostlight.fold: routine_before_default
Sera eats standing beside the hatch because maintenance chairs were removed in the last efficiency review. Mika checks condensation under the food-cart lid. Davin clears yesterday's berth fees one line at a time.

{audit_exposure >= 3: A yellow inspection bracket waits beside Sera's handwritten second box.}
{mika_seal_skill >= 1: Mika tests the brace point with her boot until the movement belongs to her too.}
{unsupported_air >= 3: Mika's bay has enough paid air for one more sleep cycle, which is not rescue but can impersonate time.}
{departure_margin >= 5: The outbound crates already sit inside the yellow grid, buying the shuttle a few clean minutes.}
{mutual_aid <= 0: Mika closes the cart mesh carefully and stops offering breakfast.}

-> default_notice

=== default_notice ===
Every light in Transfer Gallery Two turns amber.

The wall prints CONTINUITY DEFAULT in lettering large enough to be seen by people whose credentials have just stopped working.

-> classification

=== classification ===
Davin's desk divides the cylinder into three colors. Green for continuity labor. White for financed transfer. Gray for unsupported occupancy.

Sera is green because Horizon needs the hatch closed. Davin is green because somebody must print gray. Mika is gray because the food loop belongs to a suspended subcontract, although everyone will still need breakfast tomorrow.

Nara opens the audit-booth speaker. "The insured departure window is thirty-six hours. Final gallery cycling begins in nineteen minutes. Any unresolved seal fault closes the window early."

Mika reads her dead credential against the food-cart rail. "Good. I was worried the air might remain emotionally complicated."

-> default_response

=== default_response ===
// ghostlight.choice_layer: continuity_default_response
+ [Sign the continuity amendment, then make Davin classify Mika's food-loop work as necessary labor.]
    // ghostlight.action_label: bargain
    // ghostlight.branch_label: bargain_for_mika
    ~ contract_debt = contract_debt + 2
    ~ manifest_leverage = manifest_leverage + 2
    ~ davin_trust = davin_trust + 1
    ~ audit_exposure = audit_exposure + 1
    Sera signs four more years of assigned service and slides the cuff back across the desk.

    "Food is life support," she says. "You can call Mika a contractor after you learn to photosynthesize."

    Davin opens the labor-code appeal. Nara's slate opens a conflict flag beside it.
    -> queue_fold
+ [Transfer Sera's personal mass allowance to Mika and mark the food-loop manuals as clothing.]
    // ghostlight.action_label: transfer_resource
    // ghostlight.branch_label: transfer_mass_allowance
    ~ mass_credit = mass_credit + 2
    ~ departure_margin = departure_margin - 1
    ~ davin_trust = davin_trust + 1
    Sera moves her departure kilograms to Mika's cuff. Davin weighs the paper manuals, then writes PERSONAL THERMAL LAYERS with the exhausted elegance of a man falsifying a coat.

    "They do keep people alive," he says.

    "Eventually," Mika says.
    -> queue_fold
+ [Copy the scrubber rotation onto paper and send Mika habitatward to organize unpaid seal watches.]
    // ghostlight.action_label: share_operating_knowledge
    // ghostlight.branch_label: copy_scrubber_rotation
    ~ mutual_aid = mutual_aid + 1
    ~ unsupported_air = unsupported_air + 1
    ~ audit_exposure = audit_exposure + 1
    Sera writes cartridge times, valve order, and the false location of the only honest pressure gauge on the back of the breakfast wrapper.

    Mika folds it into her sleeve and pushes the food cart habitatward. The gray sleeping bays acquire a maintenance schedule before Horizon finishes removing them from one.
    -> queue_fold
+ {audit_exposure >= 3} [Raise the handwritten seal log to Nara's booth and demand the departure window remain open through a physical test.]
    // ghostlight.action_label: show_evidence
    // ghostlight.branch_label: force_physical_test
    ~ audit_exposure = audit_exposure + 2
    ~ manifest_leverage = manifest_leverage + 1
    ~ departure_margin = departure_margin - 1
    Sera presses the cuff against the booth glass. One official box. Two actual strokes.

    Nara looks from the cuff to the green hatch panel. "I can require a test. I cannot promise you will like the clock after it."

    "Clocks don't leak air."

    "That is why creditors prefer them."
    -> queue_fold

=== queue_fold ===
// ghostlight.fold: manifest_and_bay_pressure
The gray queue forms habitatward of Hatch MCR-2. People carry sleep bags, pressure masks, family tools, and the volume of their lives translated into kilograms.

{manifest_leverage >= 3: Davin's appeal marks Mika's food-loop work disputed rather than disposable.}
{mass_credit >= 3: Mika's cuff carries enough mass for one body, two manuals, and almost no dignity.}
{unsupported_air >= 3: The gray bay can survive another sleep cycle if somebody keeps the cartridges turning.}
{contract_debt >= 2: Sera's amendment glows green on the desk, making her future secure in the contractual sense of being unable to leave it.}
{davin_trust >= 2: Davin keeps his hand over the final-manifest key whenever Nara looks down at her slate.}
{audit_exposure >= 4: The seal defect and the occupancy dispute now share one bright file. Any office that opens it will see both.}

Nara orders the final pressure cycle.

-> hatch_false_green

=== hatch_false_green ===
~ hatch_state = 1
Sera pulls the dog handle once.

The panel goes green. The pressure gauge keeps falling.

Behind the hatch, the queue hears the hiss before the audit booth admits it exists.

-> hatch_response

=== hatch_response ===
// ghostlight.choice_layer: false_green_hatch
+ [Brace a boot on the chalk mark and drive the handle through the second stroke.]
    // ghostlight.action_label: repair_under_pressure
    // ghostlight.branch_label: drive_second_stroke
    ~ hatch_state = 3
    ~ seal_margin = seal_margin + 2
    ~ departure_margin = departure_margin - 1
    The handle fights, yields, and lands with the clunk the panel had lied about.

    The gauge steadies. Nara restarts the launch clock. The gray queue is still habitatward of a properly closed door.
    -> pressure_fold
+ [Hold the hatch open and wave the gray queue across before the gallery loses more air.]
    // ghostlight.action_label: hold_route
    // ghostlight.branch_label: cross_gray_queue
    ~ queue_crossed = 1
    ~ hatch_state = 2
    ~ mutual_aid = mutual_aid + 1
    ~ departure_margin = departure_margin - 2
    ~ seal_margin = seal_margin - 1
    Sera hauls the dog handle back. Mika moves first, shoving the food cart through while the queue follows low and fast.

    Davin begins counting bodies as freight because freight still has a live field on his screen.

    Nara says, "That is not an authorized manifest."

    "It is an accurate corridor," Sera says.
    -> pressure_fold
+ {mika_seal_skill >= 1} [Give Mika the handle while Sera works the manual pressure bypass.]
    // ghostlight.action_label: delegate_skill
    // ghostlight.branch_label: share_seal_authority
    ~ hatch_state = 3
    ~ mutual_aid = mutual_aid + 1
    ~ unsupported_air = unsupported_air + 1
    ~ audit_exposure = audit_exposure + 1
    Mika finds the chalk brace and drives the second stroke home while Sera opens the waist-high bypass cabinet.

    Two people do the work that Sera's green credential claimed belonged to one.

    Nara records the unauthorized operator. Davin records a successful seal.
    -> pressure_fold
+ [Bridge the old panel contacts so its green signal masks the falling gauge until the shuttle launches.]
    // ghostlight.action_label: falsify_signal
    // ghostlight.branch_label: mask_pressure_loss
    ~ departure_margin = departure_margin + 2
    ~ audit_exposure = audit_exposure - 1
    ~ seal_margin = seal_margin - 2
    ~ hatch_state = 2
    Sera slips a test lead between the contacts. The launch system sees green. The gallery hears the leak grow teeth.

    Nara's slate clears the shuttle while Davin stares at the honest gauge.

    "That buys minutes," he says.

    "With air," Sera says.
    -> pressure_fold

=== pressure_fold ===
// ghostlight.fold: final_priority_threshold
The shuttle clock resumes. The habitat contraction clock does not.

{hatch_state >= 3: Hatch MCR-2 is physically sealed; the second stroke has made the green light true.}
{hatch_state == 2: The panel remains green over a seal that has not fully seated.}
{hatch_state == 1: Hatch MCR-2 remains open or incompletely secured; the green light has no physical authority yet.}
{seal_margin >= 4: The gallery holds pressure with enough margin for a clean dock cycle.}
{seal_margin <= 1: Cold air draws grit toward the lower dog while every breath becomes part of the calculation.}
{departure_margin >= 4: The staged work and paperwork leave minutes enough to choose instead of merely react.}
{departure_margin <= 1: The insured launch window has narrowed to a single ugly sequence.}
{queue_crossed == 1: The gray queue now stands dockward of the hatch, visible to the shuttle and impossible to call an abstract occupancy problem.}
{mika_seal_skill >= 1: Mika knows the second stroke and can carry that knowledge habitatward or dockward.}
{davin_trust >= 2: Davin can still spend his hand on one manifest decision before the creditor locks the desk.}

Sera has one decision left before three separate offices make it for her.

-> final_choice

=== final_choice ===
// ghostlight.choice_layer: contraction_priority
+ [Spend the remaining margin on the independent ferry and put bodies ahead of property.]
    // ghostlight.action_label: dispatch_people
    // ghostlight.branch_label: prioritize_ferry
    {mass_credit >= 3 && departure_margin >= 2 && hatch_state >= 2:
        -> ending_ferry_success
    - else:
        -> ending_ferry_cost
    }
+ [Keep the unsupported bay pressurized long enough for another route to become possible.]
    // ghostlight.action_label: sustain_shelter
    // ghostlight.branch_label: prioritize_bay
    {mutual_aid >= 3 && unsupported_air >= 3 && mika_seal_skill >= 1:
        -> ending_bay_success
    - else:
        -> ending_bay_cost
    }
+ [Bind the false-green hatch and the gray classifications into one creditor-visible incident record.]
    // ghostlight.action_label: publish_evidence
    // ghostlight.branch_label: prioritize_record
    {audit_exposure >= 4 && hatch_state >= 2:
        -> ending_record_success
    - else:
        -> ending_record_cost
    }
+ [Preserve the insured cylinder, even if everyone remaining must sign Horizon's amendments.]
    // ghostlight.action_label: preserve_infrastructure
    // ghostlight.branch_label: prioritize_insurance
    {seal_margin >= 4 && hatch_state >= 3 && contract_debt >= 2:
        -> ending_insurance_success
    - else:
        -> ending_insurance_cost
    }

=== ending_ferry_success ===
// ghostlight.ending_label: ferry_success
// ghostlight.training_hook: departure_mass_as_class_power
Davin spends his last unlocked field. Sera's mass credit becomes Mika, two paper manuals, and as many gray-cuffed people as the shuttle master will accept under disputed cargo supervision.

{queue_crossed == 1: They are already dockward. Nara can call them unauthorized, but she cannot make them invisible through the collar glass.}
{queue_crossed == 0: Sera opens the sealed hatch for one counted passage, closes it with two strokes, and makes Nara watch every body become a delay.}
{manifest_leverage >= 3: Mika boards under a disputed continuity code that will cost Horizon lawyers more than it buys her dignity.}

The shuttle leaves insured, late, and carrying people the first manifest had rendered as atmosphere overhead.
-> END

=== ending_ferry_cost ===
// ghostlight.ending_label: ferry_cost
// ghostlight.training_hook: departure_window_closes
Sera tries to turn kilograms into passage after the arithmetic has hardened.

{mass_credit < 3: Mika's cuff carries hope measured below one body.}
{departure_margin < 2: The shuttle seals while Davin is still finding a field that can contain a person.}
{hatch_state < 2: Nara closes coverage on the uncertain hatch, and the ferry master will not risk the collar.}

The pad floodlights recede across the window. The cylinder keeps everyone it has just declared too expensive to support.
-> END

=== ending_bay_success ===
// ghostlight.ending_label: bay_success
// ghostlight.training_hook: quiet_mutual_aid_buys_exit_time
Mika takes the paper scrubber rotation habitatward. Sera transfers the last oxygen allotments. Davin leaves the gray bay present in the occupancy table long enough to make deletion a dispute instead of an automation.

The residents rotate cartridges, watch the pressure gauge, and sleep in shifts. No banner appears. Nobody has time to invent a movement.

Nine people reach an independent cislunar ferry the next day without signing the revised contracts. The bay goes cold after them. The fact that care worked does not make the cold necessary.
-> END

=== ending_bay_cost ===
// ghostlight.ending_label: bay_cost
// ghostlight.training_hook: care_without_material_margin
The gray bay has volunteers, but not the whole machine.

{mutual_aid < 3: People wait for somebody else to own the schedule, because years of paid permission have made initiative feel like theft.}
{unsupported_air < 3: The oxygen allotments expire before another ferry can accept the route.}
{mika_seal_skill < 1: The scrubber watch knows the cartridge order but not how to hold the transfer seal when pressure changes.}

The residents sign in batches. Horizon records voluntary continuity enrollment. Sera learns how little hope weighs when the filter cart is empty.
-> END

=== ending_record_success ===
// ghostlight.ending_label: record_success
// ghostlight.training_hook: administrative_harm_bound_to_physical_evidence
Sera holds the honest gauge beside the handwritten two-stroke log. Davin appends the gray occupancy table. Mika adds the food-loop dependency list in soil-stained pencil.

Nara accepts the bundle because her office can price a coupled transport and life-support failure even when it cannot recognize the people inside it kindly.

{manifest_leverage >= 3: Mika's disputed labor code keeps the food loop inside the inquiry.}
{davin_trust >= 2: Davin seals the record before the manifest desk locks him out.}

The record will not reopen today's window. It will make every later Horizon default harder to describe as three unrelated prudent decisions.
-> END

=== ending_record_cost ===
// ghostlight.ending_label: record_cost
// ghostlight.training_hook: evidence_without_immediate_rescue
Sera sends what she has.

{audit_exposure < 4: The creditor file receives a pressure anomaly, an occupancy dispute, and no chain strong enough to keep them together.}
{hatch_state < 2: The panel history says green; the physical seal says nothing an office can bill.}

Nara preserves the fragments. Davin loses access. Mika remains gray. Evidence survives the launch window and fails to become rescue inside it.
-> END

=== ending_insurance_success ===
// ghostlight.ending_label: insurance_success
// ghostlight.training_hook: orderly_contraction_as_preserved_harm
Hatch MCR-2 seats on the second stroke. The gallery pressure holds. Nara keeps the shuttle and the contracted cylinder inside coverage.

Sera's amendment becomes the model. Davin prints copies. The gray queue signs because a functioning habitat can still make refusal lethal.

The next cargo will bring filters, membranes, food-loop nutrients, and TerraCore auditors. The cylinder survives. Horizon's method survives inside it.
-> END

=== ending_insurance_cost ===
// ghostlight.ending_label: insurance_cost
// ghostlight.training_hook: infrastructure_saved_without_sufficient_authority
Sera offers the cylinder a clean contraction after the seal margin is already spent.

{seal_margin < 4: The gallery cannot pass Nara's pressure test.}
{hatch_state < 3: The green panel still speaks before the lower dog has seated.}
{contract_debt < 2: Too few households have signed the amendments for the creditors to price the remaining occupancy.}

Coverage closes. The shuttle lifts empty to protect itself. Horizon keeps title to a habitat it can no longer afford to rescue, and the residents inherit the machinery between those two facts.
-> END
