// ghostlight.artifact_id: patina_borrowed_names_branch_fold_v0
// ghostlight.fixture_id: patina-borrowed-names-v0
// ghostlight.scene_id: patina-borrowed-names-v0.kappa-line-issue
// ghostlight.final_ink_path: examples/ink/aetheria/patina-borrowed-names-v0.branch-and-fold.v0.ink

VAR name_chain = 1
VAR route_minutes = 4
VAR fit_confidence = 1
VAR public_exposure = 0
VAR orrin_witness = false
VAR rain_again_found = false

-> start

=== start ===
// ghostlight.scene: patina.opening.line_issue_alcove; visual_scene_id: patina_issue_alcove_establishing
Pallas Yard Twelve is awake but not yet earning.

Off the loop-outer side of Service Ring Kappa, the line-issue alcove receives workers from the muster corridor and releases them through a sliding hatch into the ring. A bolted counter divides the worker lane from caged supplier lockers. VitaForge fittings hang behind the counter in numbered gray rows. Beneath them, almost hidden by the counter lip, a second return board carries grease marks, string knots, and tactile chips.

Nara-7 waits at the yellow foot line. She is a BioDrone Standard seal technician: a slender engineered humanoid body in a numbered gray skinsuit, built for precise work at Kappa's manifold faces. AU records her shift as VitaForge-supplied capacity. The line kit touching her neck and wrists will also remain supplier property.

-> ordinary_issue

=== ordinary_issue ===
// ghostlight.scene: patina.routine.issue_liturgy; visual_scene_id: patina_issue_liturgy
Eli Venn, the baseline issue clerk, has delivered the same call-and-response for eleven years.

"Body?"

"Present," Nara says.

"Kit?"

"Pending."

"Route?"

"Kappa primary."

"Return?"

"Required."

The official answer finishes there. Orrin Dax, waiting with an old anchor hook across one shoulder, adds the worker's fifth line under his breath.

"Owner?"

Teth Inkwise answers from a compact dry-operation harness parked beside the support rack. "Experiencing temporary uncertainty."

Eli does not smile. He moves the corner of his mouth into a position that has never been entered in the timekeeping system.

-> ritual_choice

=== ritual_choice ===
// ghostlight.choice_layer: morning_liturgy
+ [Give the four required answers and keep the queue moving.]
    // ghostlight.action_label: speak
    // ghostlight.branch_label: answer_official_liturgy
    Nara repeats the release sequence at the exact speed the counter expects.

    Eli's slate clears four green fields. The fifth line remains where workers keep it: in breath, out of evidence.
    -> kit_handoff
+ [Add the fifth line: "Owner? Still checking."]
    // ghostlight.action_label: speak
    // ghostlight.branch_label: answer_shift_joke
    ~ name_chain = name_chain + 1
    ~ public_exposure = public_exposure + 1
    ~ route_minutes = route_minutes - 1
    "Owner?" Nara asks.

    Orrin looks solemn. "Still checking."

    Teth's harness translator clicks approval. Eli says, "You are using expensive seconds," which is issue-counter language for *good morning*.
    -> kit_handoff
+ [Press Rain-Again's blue tactile chip into the second board before offering your wrist.]
    // ghostlight.action_label: touch_object
    // ghostlight.branch_label: mark_private_name
    ~ name_chain = name_chain + 2
    ~ fit_confidence = fit_confidence + 1
    ~ route_minutes = route_minutes - 1
    Nara presses the chipped blue marker into the grease-soft board.

    Rain-Again is the coupler that clicks twice after cleaning and sits flat against the old pressure line below her left ear. The ledger calls it LR-441. The ledger has also called three other objects LR-441.

    Eli turns the board half a handspan away from the camera and offers Nara her kit.
    -> kit_handoff

=== kit_handoff ===
// ghostlight.scene: patina.object.wrong_coupler; visual_scene_id: patina_wrong_coupler_closeup
The audit-port seal seats at Nara's right wrist. The recorder closes at her belt. The limiter bus coupler reaches the fitting below her left ear.

It closes silently.

Rain-Again has never closed silently in its life.

Nara opens the catch. The inner rim is smooth where Rain-Again carries a diagonal repair scratch. Eli's slate stays green. Somewhere between local cleaning, VitaForge inventory, and AU issue, a familiar number has arrived on the wrong object.

{route_minutes == 4: The ring hatch shows four amber segments to route release.}
{route_minutes <= 3: The ring hatch has already fallen to three amber segments.}

-> trace_choice

=== trace_choice ===
// ghostlight.choice_layer: trace_the_fitting
+ [Check the repair scratch, double click, and blue tactile chip against the second board.]
    // ghostlight.action_label: inspect_object
    // ghostlight.branch_label: trace_by_private_name
    ~ rain_again_found = true
    ~ fit_confidence = fit_confidence + 2
    ~ name_chain = name_chain + 1
    ~ route_minutes = route_minutes - 1
    Nara checks three things the official scan ignores: scratch, sound, and the blue chip's shallow broken corner. When she points, Eli works the peg-fourteen catch twice.

    The board sends her to peg fourteen. Rain-Again hangs there under a clean yellow seal and a different serial. The wrong coupler in her hand wears Rain-Again's number like a borrowed shirt.
    -> clock_fold
+ [Ask Teth to read the return knots from the harness-side angle.]
    // ghostlight.action_label: request_help
    // ghostlight.branch_label: trace_with_teth
    ~ rain_again_found = true
    ~ name_chain = name_chain + 2
    ~ route_minutes = route_minutes - 1
    Teth turns the harness with four arms resting loose inside the support loops. From that angle, the return strings line up with the supplier pegs.

    "Blue chip, short-short knot, peg fourteen," Teth says. "Your rain has been promoted to a climate it cannot support."

    Rain-Again waits under the wrong serial. Nara's offered coupler belongs two pegs down.
    -> clock_fold
+ [Ask Orrin to witness the physical mismatch before anyone moves a fitting.]
    // ghostlight.action_label: set_condition
    // ghostlight.branch_label: trace_with_baseline_witness
    ~ rain_again_found = true
    ~ orrin_witness = true
    ~ public_exposure = public_exposure + 1
    ~ fit_confidence = fit_confidence + 1
    ~ route_minutes = route_minutes - 1
    Orrin sets his hook against the anchor slot by the yellow line and leans close enough to see the rim.

    "Smooth catch here," he says. "Diagonal scratch on fourteen. I witness two objects and one very ambitious number."

    Eli adds a blank witness field to the slate. He does not yet add a name.
    -> clock_fold
+ [Trust the green serial match and test the silent coupler against your body.]
    // ghostlight.action_label: use_object
    // ghostlight.branch_label: test_official_match
    ~ fit_confidence = fit_confidence - 1
    Nara closes the silent coupler.

    It clears the slate. It also presses the old pressure line below her ear hard enough to make the left hand answer half a beat late.

    She opens and closes that hand. Eli sees. Teth sees. The timekeeping board sees only green.
    -> clock_fold

=== clock_fold ===
// ghostlight.fold: wrong_kit_becomes_shared_problem
// ghostlight.scene: patina.pressure.route_clock; visual_scene_id: patina_route_clock
The ordinary queue notices the delay.

{rain_again_found:
Rain-Again hangs on peg fourteen under the wrong serial, one of Eli's forearm lengths away beyond the counter and administratively farther away than Mars.
- else:
The correct fitting has not been identified. The silent coupler leaves a pale pressure crescent below Nara's ear.
}

{orrin_witness: Orrin's blank witness field waits on Eli's slate. A baseline signature can make the mismatch harder to call product behavior and easier to call everyone's paperwork.}
{name_chain >= 3: The second board is legible across bodies: blue chip, short-short knot, grease slash. Three small ways to mean one object.}
{public_exposure >= 2: The camera above the counter has turned its status eye from idle gray to recording blue.}
{route_minutes >= 4: The route clock still offers enough time to be careful without making care look heroic.}
{route_minutes <= 2: The route clock has entered the red minute where AU converts every pause into an answerable name.}

Eli lowers his voice. "I can open an incident, correct a serial, or issue clean spare stock. I cannot do all three before release."

The sentence is true because the counter was purchased that way.

-> commit_choice

=== commit_choice ===
// ghostlight.choice_layer: name_or_hide_the_mismatch
+ [Put Rain-Again and the private return marks into the incident record.]
    // ghostlight.action_label: commit_record
    // ghostlight.branch_label: record_borrowed_name
    ~ public_exposure = public_exposure + 2
    ~ route_minutes = route_minutes - 2
    {rain_again_found && fit_confidence >= 3:
        Nara places both couplers on the counter, turns the scratched rims toward the camera, and says the private name before the official numbers.
        -> ending_record_protects
    - else:
        Nara gives Eli the name and an incomplete object chain. The ledger accepts the word more readily than the uncertainty around it.
        -> ending_record_cost
    }
+ {rain_again_found} [Exchange the physical fittings by the second board and leave the official ledger clean.]
    // ghostlight.action_label: transfer_object
    // ghostlight.branch_label: quiet_named_exchange
    ~ route_minutes = route_minutes - 1
    {name_chain >= 3:
        Eli moves two couplers between the supplier pegs and the issue surface. Nara and Teth move two tactile chips and two return knots. Then Eli watches the route clock with the disciplined blindness of a man doing one useful thing at work.
        -> ending_quiet_chain_holds
    - else:
        The objects are right. The return marks are not complete enough to protect the next shift from the same mistake.
        -> ending_quiet_chain_frays
    }
+ {rain_again_found} [Ask Orrin to witness only the physical mismatch and let Eli correct the serial.]
    // ghostlight.action_label: request_witness
    // ghostlight.branch_label: narrow_witness_correction
    ~ orrin_witness = true
    ~ public_exposure = public_exposure + 1
    ~ route_minutes = route_minutes - 1
    Nara names no helper and no board. Orrin signs for one smooth rim, one diagonal scratch, and one serial applied twice.

    Eli corrects the issue row. The fifth line stays outside the form.
    -> ending_narrow_witness
+ [Take clean spare stock and request the short manifold route.]
    // ghostlight.action_label: accept_cost
    // ghostlight.branch_label: accept_clean_spare
    ~ public_exposure = public_exposure - 1
    Nara closes the silent coupler and asks for the short manifold route.

    Eli can authorize the route or admit the kit is wrong. The counter has time for one form of honesty.
    -> ending_spare_body_cost

=== ending_record_protects ===
// ghostlight.ending_label: named_record_protects_worker
// ghostlight.training_hook: private_name_as_formal_continuity_evidence
Eli enters LR-441, LR-612, diagonal scratch, double click, blue chipped marker.

Then, after the smallest pause, he enters Rain-Again.

{orrin_witness: Orrin signs beneath it.}{not orrin_witness: Eli signs as the only baseline witness and becomes briefly less employable.}

The hatch releases without Nara. Her shift credit loses the red minutes. The incident record cannot call the mismatch a feeling, and the camera now knows where to look for the second board.

{public_exposure >= 3: The recording eye stays blue while Eli answers the new audit prompt.}{public_exposure < 3: The recording eye turns gray, but the named record remains.}

Teth moves one tactile chip before audit can arrive. Not the whole board. Just enough that help remains a practice instead of a list of culprits.
-> END

=== ending_record_cost ===
// ghostlight.ending_label: named_record_exposes_chain
// ghostlight.training_hook: evidence_without_complete_object_chain
The incident accepts Rain-Again as worker terminology and rejects it as verified identity.

VitaForge quarantines three couplers. AU charges the lost release to Yard Twelve. Eli's slate asks who originated the private label.

Nara leaves that field blank.

{route_minutes <= 1: The queue starts late enough to lose a full shift interval.}{route_minutes > 1: The queue loses minutes but keeps the shift.} The wrong object is no longer on her body, but the record has learned there is another ledger under its counter.
-> END

=== ending_quiet_chain_holds ===
// ghostlight.ending_label: quiet_named_mutual_aid
// ghostlight.training_hook: cross_body_continuity_without_movement_claim
Rain-Again clicks twice below Nara's ear and lies flat against the old pressure line.

The official slate stays green. The second board gains one grease slash connecting yesterday's serial to today's peg.

{route_minutes <= 2: Yard Twelve docks the queue a red minute.}{route_minutes > 2: The queue clears before the clock can decide anyone was late.}

Orrin collects his hook. Teth rolls toward harness release. Nobody calls the arrangement solidarity. At this hour it is still only the habit of returning a thing to the person who knows its private weight.
-> END

=== ending_quiet_chain_frays ===
// ghostlight.ending_label: quiet_exchange_without_durable_mark
// ghostlight.training_hook: mutual_aid_needs_memory_surface
Rain-Again returns to Nara. The other coupler returns to a peg that may or may not belong to it.

Eli clears the shift on time. The ledger is clean enough to repeat itself.

Before leaving, Teth ties a short-short knot beneath the second peg. One more mark. Not enough for certainty. Enough for the next worker to stop before trusting green.
-> END

=== ending_narrow_witness ===
// ghostlight.ending_label: physical_mismatch_narrowly_corrected
// ghostlight.training_hook: bounded_cross_category_witness
The correction enters as duplicate serial attachment. No private names. No return board. No claim about who a worker is.

That narrowness protects the helpers and lets VitaForge describe the event as clerical.

{public_exposure >= 2: The camera records Orrin's signature and the two mismatched rims.}{public_exposure < 2: Eli keeps the witness field local to the issue slate.}

It also puts Orrin's baseline signature beside Nara's body tag on one fact the supplier cannot route into behavioral drift: the metal parts were different.

{route_minutes <= 2: Nara loses the red minutes and keeps the correct fit.}{route_minutes > 2: The hatch releases her into Kappa with one minute left.}
-> END

=== ending_spare_body_cost ===
// ghostlight.ending_label: clean_ledger_body_cost
// ghostlight.training_hook: administrative_cleanliness_exports_cost_to_body
The short route keeps Nara out of the deepest manifold reach. It also pays less claimshare.

The silent coupler presses below her ear all morning. By break, the pale crescent has darkened. Teth puts Rain-Again's blue chip in the lowest row of the second board where a supervisor camera cannot see through the counter lip.

{public_exposure <= 0: The counter camera remains gray.}{public_exposure > 0: The counter camera records the voluntary route change and nothing beneath the lip.}

{route_minutes >= 4: The hatch releases Nara on time.}{route_minutes < 4: The shorter route recovers the clock but not the missing claimshare.}

The official record shows one clean issue, one voluntary route change, and no problem worth owning.
-> END
