VAR strike_cohesion = 1
VAR baseline_solidarity = 0
VAR security_pressure = 0
VAR life_support_margin = 3
VAR evidence_of_sentience = 1
VAR cephalopod_leverage = 1
VAR bioelevate_liability = 0
VAR worker_injury = 0
VAR media_visibility = 0
VAR nara_centered = false
VAR nara_path_marked = false
VAR bypass_ready = false
VAR formal_stoppage = false

-> start

=== start ===
// ghostlight.scene: pallas.intro.yard_wake; visual_scene_id: pallas_yard_wake
-> intro_yard_wake

=== intro_yard_wake ===
Pallas Yard Twelve wakes without ceremony. The cavity wall hums through the deck plates before the shift bell, a low mineral throat-note that everyone pretends not to hear unless it changes.

-> intro_service_ring

=== intro_service_ring ===
// ghostlight.scene: pallas.intro.service_ring_kappa; visual_scene_id: pallas_service_ring_kappa
Lio Vale walks Service Ring Kappa with a slate under one arm and yesterday's sleep still folded behind the eyes. AU calls this a routine seal-maintenance cycle. The workers call it breathing for people rich enough to forget what keeps the air inside.

Wet-side crew drift in harness cradles shaped for limbs that do not stand. Baseline riggers clip dry anchors along the outer rail and complain with the solemnity of a religious rite. Engineered seal techs wait by the manifold in clean gray skinsuits, each one tagged by function, not by family. Small accommodations make the system look civilized: warmer mist for cephalopod lungs, softer tool grips for grafted hands, break-table charts that translate three kinds of body clock into one AU shift.

The yard is normal. That is the first horror.

-> intro_nara

=== intro_nara ===
// ghostlight.scene: pallas.intro.nara_manifold; visual_scene_id: pallas_nara_manifold
Nara-7 stands at the Kappa manifold with the other engineered seal technicians, each gray skinsuit numbered for a task stream instead of a family line. She touches the wall twice, pauses, and touches it again with two fingers spread wide. AU calls that retained route behavior. Lio has watched enough shifts to know a warning when it learns manners.

-> intro_orrin

=== intro_orrin ===
// ghostlight.scene: pallas.intro.orrin_anchor_rail; visual_scene_id: pallas_orrin_anchor_rail
Orrin Dax's dry-side crew works the anchor rail with cold-stiff hands and old tools. They are baseline riggers: born human-standard, paid as if that should be its own reward, and courted after hours by Baseline League mutters about jobs stolen by built workers and uplifted bodies. Orrin complains like a man trying not to notice he is scared.

-> intro_teth

=== intro_teth ===
// ghostlight.scene: pallas.intro.teth_wet_service; visual_scene_id: pallas_teth_wet_service
Teth Inkwise hangs in a wet-service cradle, four arms managing valves while the rest of the body floats inside the harness rig. The cephalopod crew keeps the service lungs alive in spaces built by companies that still describe their bodies as accommodation costs.

-> intro_ilya

=== intro_ilya ===
// ghostlight.scene: pallas.intro.ilya_supervisor_glass; visual_scene_id: pallas_ilya_supervisor_glass
Ilya Marne watches from the supervisor glass with AU polish and AU hunger: schedule first, liability second, people somewhere below the line where the spreadsheet stops showing decimals.

Lio knows the day will probably go wrong. The only question is whether it goes wrong before everyone has finished pretending this is maintenance.

-> ordinary_round_choice

=== ordinary_round_choice ===
// ghostlight.scene: pallas.ordinary_round.choice; visual_scene_id: pallas_ordinary_round_choice
// ghostlight.branch: branch-01-walk-with-nara; action: move; intent: Learn the engineered worker's ordinary hazard routine before crisis.
* [Walk the manifold line with Nara-7.]
    ~ evidence_of_sentience += 1
    ~ nara_centered = true
    -> walk_with_nara
// ghostlight.branch: branch-01-sit-with-orrin; action: wait; intent: Sit inside baseline resentment instead of treating it as villain fog.
* [Take the break-table bench beside Orrin Dax.]
    ~ baseline_solidarity += 1
    -> sit_with_orrin
// ghostlight.branch: branch-01-check-teth-harness; action: touch_object; intent: Ground cephalopod body affordances and technical leverage in equipment.
* [Check Teth's zero-g maintenance harness before the wet-side crawl.]
    ~ cephalopod_leverage += 1
    -> check_teth_harness

=== walk_with_nara ===
// ghostlight.scene: pallas.branch.nara_manifold_walk; visual_scene_id: pallas_nara_manifold_walk
Nara-7 touches the Kappa manifold twice, waits, then touches it a third time with two fingers spread apart. Her batch tag says Seal Technician, BioDrone Standard. Her hand says she is listening.

"Kappa sings wrong," she says.

Lio checks the board. The board shows amber pending review, which means nothing has failed in a way AU has agreed to price.

"Wrong how?"

Nara looks down the crawl throat where the patch plates overlap like old scars. "From before reset."

She does not say memory. She does not say Rell. She gives Lio just enough truth to carry and no more, because truth in this yard is heavy and management keeps scales.

-> chokepoint_refusal

=== sit_with_orrin ===
// ghostlight.scene: pallas.branch.orrin_break_table; visual_scene_id: pallas_orrin_break_table
Orrin Dax has a mug of recycler coffee in one hand and a dry anchor hook across his knees. The hook is older than the shift board and better maintained.

"Soft-sleeve specialists got a humidity credit again," he says, not looking at Lio. "My crew gets frost rash and a lecture about grit."

Across the galley, two engineered techs eat in precise silence. Orrin watches them the way a man watches his own job walking around in someone else's skin.

"Funny how the future always needs fewer of us," he says.

Lio lets the complaint breathe before answering. "Future still needs dry anchors cleared before wet service can carry load."

Orrin snorts. "Then tell the future to remember who taught it where the bolts are."

-> chokepoint_refusal

=== check_teth_harness ===
// ghostlight.scene: pallas.branch.teth_harness_check; visual_scene_id: pallas_teth_harness_check
Teth Inkwise turns in the wet-service cradle, four arms working while the rest of the body hangs in careful suspension. The cephalopod zero-g maintenance harness is not a suit so much as a negotiated truce between flesh, tool, coolant, and vacuum: soft-anchor loops, pressure cuffs, a rail of sealed implements, tiny valves patched by someone who trusted skill more than procurement.

"Your cuff is patched again," Lio says.

"Everything noble is patched," Teth replies. "Including this yard's conscience."

The service lung diagram flickers under one of Teth's arms. If Kappa goes hard red, AU will discover exactly how much of its schedule lives in equipment designed for bodies it refuses to honor.

-> chokepoint_refusal

=== chokepoint_refusal ===
// ghostlight.scene: pallas.threshold.chokepoint_refusal; visual_scene_id: pallas_chokepoint_refusal
The shift bell rings. The board orders a blind crawl through Kappa.

Nara-7 stops at the lip of the access throat. Three other engineered seal techs stop with her. They touch the manifold in the same sequence: twice, wait, third touch spread wide.

The board does not understand a ritual unless someone bills it.

"Proceed," Ilya Marne says over the supervisor channel.

Nara's voice is small and clear. "No blind crawl. Kappa remembers cutting Rell. We remember Rell."

The dry-side riggers go quiet. Teth's harness valves click once, then stop. Ilya arrives fast enough to prove she had been watching.

"Classify that," Ilya says to Lio.

// ghostlight.branch: branch-02-quiet-safety-pause; action: speak; intent: Keep the stoppage quiet long enough to protect life support and preserve evidence.
* [Call it a safety pause and keep the channel local.]
    ~ life_support_margin += 1
    ~ security_pressure += 1
    ~ evidence_of_sentience += 1
    -> quiet_safety_pause
// ghostlight.branch: branch-02-formalize-stoppage; action: speak; intent: Name the refusal as labor action before AU can classify it as equipment drift.
* [Log it as a coordinated labor stoppage.]
    ~ formal_stoppage = true
    ~ strike_cohesion += 2
    ~ security_pressure += 2
    ~ media_visibility += 1
    -> formalize_stoppage
// ghostlight.branch: branch-02-shield-nara; action: block_object; intent: Put Lio's body and authority between Nara and direct inspection.
* [Step between Nara and the inspector cam.]
    ~ nara_centered = true
    ~ strike_cohesion += 1
    ~ security_pressure += 1
    -> shield_nara

=== quiet_safety_pause ===
// ghostlight.scene: pallas.branch.quiet_safety_pause; visual_scene_id: pallas_quiet_safety_pause
"Safety pause," Lio says. "Local channel. Kappa amber, worker hazard report, no blind crawl until wet-service confirms load."

Ilya's eyes narrow at the word worker. She hears the legal trap even when Lio buries it in maintenance language.

The pause buys minutes. It also keeps the first evidence small enough that AU might try to pocket it.

-> tests_and_reversals

=== formalize_stoppage ===
// ghostlight.scene: pallas.branch.formalize_stoppage; visual_scene_id: pallas_formalize_stoppage
Lio keys the log before fear can bargain the words down.

"Coordinated labor stoppage at Kappa manifold. Cause: remembered injury, active hazard, refusal of blind crawl."

The channel goes cold. Someone in the outer ring whistles once. Ilya stops moving like a person and starts moving like a policy document with legs.

The strike exists now. That makes it harder to erase and easier to punish.

-> tests_and_reversals

=== shield_nara ===
// ghostlight.scene: pallas.branch.shield_nara; visual_scene_id: pallas_shield_nara
Lio steps into the inspector cam's line, close enough to make the gesture unmistakable and deniable by only the most expensive liar.

"Questions through shift control," Lio says. "No direct probe. She reported a hazard."

Nara's hands stay flat on the rail. Orrin watches the move, and his expression does a small ugly human thing: envy first, respect second.

-> tests_and_reversals

=== tests_and_reversals ===
// ghostlight.scene: pallas.reversal.classification_pressure; visual_scene_id: pallas_classification_pressure
The official board calls it irregularity. The worker channel calls it refusal. BioElevate legal, summoned by Ilya with two clipped phrases, calls it potential scaffold drift.

BioElevate built and leased the engineered seal tech line under language that turned cognition into product behavior and injury into maintenance variance. If Nara remembers Rell, BioElevate has not merely sold AU labor. It has sold AU a worker and called her equipment.

The Awakened Labor Front mirror sits outside AU's incident board: a recognition channel for created and altered workers, slow, underfunded, and inconveniently hard to edit once a packet lands there.

{formal_stoppage:
The formal log has already narrowed everyone's exits. Ilya can punish a stoppage, but she cannot pretend no one named it.
}

Orrin's dry-side crew waits by the anchors. Their faces say what nobody wants to admit: if engineered workers are people, then baseline workers have been angry at the wrong enemy and still poor afterward.

{life_support_margin <= 2:
The air plant pings a margin warning. Every political word in the ring gets heavier.
- else:
The bypass timer is still kind. Not generous, but kind enough to make choices real.
}

// ghostlight.branch: branch-03-show-nara-memory; action: show_object; intent: Use observable route memory as evidence without forcing private confession.
* [Ask Nara to mark the remembered cutting path on Lio's slate.]
    ~ evidence_of_sentience += 2
    ~ bioelevate_liability += 1
    ~ nara_centered = true
    -> show_nara_memory
// ghostlight.branch: branch-03-ask-orrin-solidarity; action: speak; intent: Ask baseline riggers to defend safety through their own expertise.
* [Ask Orrin to put dry-side authority behind the refusal.]
    ~ baseline_solidarity += 2
    ~ strike_cohesion += 1
    -> ask_orrin_solidarity
// ghostlight.branch: branch-03-bypass-kit; action: use_object; intent: Convert cephalopod leverage into safety margin without erasing strike leverage.
* {cephalopod_leverage >= 2} [Have Teth stage a strike-safe life-support bypass kit.]
    ~ bypass_ready = true
    ~ life_support_margin += 2
    ~ cephalopod_leverage += 1
    -> bypass_kit
// ghostlight.branch: branch-03-public-liability; action: transfer_object; intent: Make BioElevate's liability language visible outside the yard.
* [Push the incident packet to an outside ALF legal mirror.]
    ~ media_visibility += 2
    ~ bioelevate_liability += 2
    ~ security_pressure += 1
    -> public_liability

=== show_nara_memory ===
// ghostlight.scene: pallas.branch.nara_marks_path; visual_scene_id: pallas_nara_marks_path
Lio turns the slate outward instead of handing it to Ilya.

"Mark the path. Only the path."

~ nara_path_marked = true

Nara taps the crawl diagram with frightening precision: two safe plates, one false seam, the cut line where Rell's suit opened, the manual valve AU replaced in the report but not in the wall.

BioElevate legal asks whether the marks might be trained route residue.

Orrin says, "Trained route residue does not remember the part procurement lied about."

-> climax

=== ask_orrin_solidarity ===
// ghostlight.scene: pallas.branch.orrin_solidarity; visual_scene_id: pallas_orrin_solidarity
Lio does not ask Orrin to be noble. That would insult everyone present.

"Dry anchors decide whether wet service can carry load. You know the route. Are you signing the crawl order as safe?"

Orrin looks at Nara, then at the crawl throat, then at the faces of his own crew.

"No," he says. "And if anyone writes that I did, I will personally introduce them to the anchor hook."

The laugh that follows is thin, but it is shared.

-> climax

=== bypass_kit ===
// ghostlight.scene: pallas.branch.bypass_kit; visual_scene_id: pallas_bypass_kit
Teth opens a service pouch and produces a bypass bridge wrapped in maintenance cloth, the kind of device that officially does not exist because officially nobody needs to survive a strike.

"This gives Kappa minutes," Teth says. "Not obedience. Minutes."

The wet-service crew moves like a quiet animal with too many hands and exactly enough discipline. Pressure steadies.

-> climax

=== public_liability ===
// ghostlight.scene: pallas.branch.public_liability; visual_scene_id: pallas_public_liability
Lio sends the packet before Ilya can put a hand over the slate.

The ALF legal mirror blinks receipt. Somewhere outside AU's walls, the words coordinated memory and hazard refusal have escaped into a place procurement cannot wipe with a revised work order.

Ilya's voice goes soft. "You understand what security will call that."

"Late," Lio says.

-> climax

=== climax ===
// ghostlight.scene: pallas.climax.kappa_red_alert; visual_scene_id: pallas_kappa_red_alert
Kappa goes red.

-> climax_decision

=== climax_decision ===
// ghostlight.scene: pallas.climax.decision_line; visual_scene_id: pallas_climax_decision_line

{bypass_ready:
The bypass bridge catches part of the load, humming in Teth's harness like a held breath.
- else:
The seal assembly shudders hard enough to make dust jump from old bolt heads. The margin is leaving.
~ life_support_margin -= 1
}

{baseline_solidarity >= 2:
Orrin's dry-side crew clears the anchor rail without waiting for an AU order.
- else:
The baseline riggers hesitate, trapped between fear of replacement and fear of being blamed for whatever dies next.
~ life_support_margin -= 1
}

{security_pressure >= 3:
Security drones appear at the far hatch, polite little coffins with blue lights.
}

{life_support_margin <= 1:
The cavity breath turns shallow. No one has the luxury of a pure victory.
}

Ilya demands a decision: recognition language in the incident log, {bypass_ready:immediate bypass control under AU authority, |sealed evidence custody, }or emergency clearance to force the crawl.

// ghostlight.branch: branch-04-hold-for-recognition; action: speak; intent: Hold safety work behind formal recognition of the refusal as worker judgment.
* [Hold the line until Ilya records worker judgment.]
    ~ strike_cohesion += 2
    ~ security_pressure += 1
    {life_support_margin <= 1:
      ~ worker_injury += 1
    }
    -> hold_for_recognition
// ghostlight.branch: branch-04-trade-bypass-for-evidence; action: transfer_object; intent: Trade temporary bypass control for preserved evidence and no forced crawl.
* {bypass_ready} [Trade bypass control for sealed evidence and no forced crawl.]
    ~ life_support_margin += 1
    ~ evidence_of_sentience += 1
    ~ bioelevate_liability += 1
    -> trade_bypass_for_evidence
// ghostlight.branch: branch-04-seal-evidence-without-bypass; action: transfer_object; intent: Preserve evidence and forbid forced crawl when there is no bypass leverage to trade.
* {not bypass_ready} [Seal the evidence packet and forbid a forced crawl.]
    ~ evidence_of_sentience += 1
    ~ bioelevate_liability += 1
    ~ security_pressure += 1
    -> seal_evidence_without_bypass
// ghostlight.branch: branch-04-orrin-front-demand; action: mixed; intent: Let baseline workers front the safety demand so solidarity becomes material.
* {baseline_solidarity >= 2} [Let Orrin front the safety demand while Lio protects Nara.]
    ~ baseline_solidarity += 1
    ~ strike_cohesion += 1
    ~ security_pressure -= 1
    -> orrin_front_demand

=== hold_for_recognition ===
// ghostlight.scene: pallas.branch.hold_for_recognition; visual_scene_id: pallas_hold_for_recognition
Lio keeps both hands visible.

"Record it as worker judgment. Not drift. Not sabotage. Judgment."

Ilya looks at the red board, {nara_path_marked:at Nara's marked path, |at Nara's steady hands, }at Orrin's crew, {bypass_ready:at Teth's waiting bypass.|at Teth's harness flexing without a bridge to spend.} She hates every door and all of them are open.

{evidence_of_sentience >= 4:
Even Ilya cannot make the scene look like equipment drift without leaving fingerprints on the lie.
- else:
The evidence is thinner than Lio wants. The phrase will survive, but it will have to survive lawyers with clean sleeves and excellent lighting.
}

{worker_injury > 0:
The delay costs blood. A baseline rigger catches a pressure lash across the shoulder before Teth can bleed the surge. The room will remember that too.
}

Ilya records the phrase with a face like broken glass hidden under a clean cloth.

-> return_fold

=== trade_bypass_for_evidence ===
// ghostlight.scene: pallas.branch.trade_bypass_for_evidence; visual_scene_id: pallas_trade_bypass_for_evidence
Lio nods to Teth.

"Bypass under shared custody. Sealed packet to ALF mirror, AU incident board, and yard safety. No forced crawl."

Ilya accepts because the alternative is a casualty report she cannot classify out of existence. BioElevate legal immediately begins sanding the word memory into residue, but the packet has already left the room.

-> return_fold

=== seal_evidence_without_bypass ===
// ghostlight.scene: pallas.branch.seal_evidence_without_bypass; visual_scene_id: pallas_seal_evidence_without_bypass
Lio lifts the slate where the inspector cam can see the custody seal.

"Packet to yard safety, AU incident board, and the Awakened Labor Front mirror. No blind crawl while Kappa is red. If you want a body in that throat, you will sign your name above it."

There is no bypass bridge to soften the bargain. The margin stays ugly. But the evidence now has three homes, and none of them are inside Ilya's pocket.

-> return_fold

=== orrin_front_demand ===
// ghostlight.scene: pallas.branch.orrin_front_demand; visual_scene_id: pallas_orrin_front_demand
Orrin steps forward before Lio has to spend Nara again.

"Dry side refuses the crawl," he says. "Our call. Our anchors. You want a body in that throat, superintendent, put the board in first."

For one bright second, the old lie breaks: baseline and engineered labor are not natural enemies. They are different answers to the same invoice.

-> return_fold

=== return_fold ===
// ghostlight.scene: pallas.return.seal_holds; visual_scene_id: pallas_seal_holds
The seal holds.

{worker_injury > 0:
It holds with an injury logged in three languages and blamed in four. The later strike committees will argue about whether Lio waited too long. They will not be entirely wrong.
- else:
It holds without a body spent for punctuation, which makes the victory smaller and more dangerous to management.
}

{strike_cohesion >= 4:
By second shift, the refusal pattern has crossed two service rings. Workers do not say strike on the open channel. They do not need to.
- else:
The stoppage folds back into work, but not cleanly. Something has learned its own shape under AU's floor plates.
}

{baseline_solidarity >= 3:
Orrin's crew starts correcting anyone who says the engineered techs panicked. They use safety language first. Politics arrives wearing work gloves.
- else:
Baseline resentment survives the day, bruised but not beaten. The Baseline League will find ears here later if no one keeps doing the harder work.
}

{bioelevate_liability >= 3:
BioElevate's liability memo leaks with enough redactions to make the missing words famous.
- else:
BioElevate keeps the cleanest language for itself and leaves the yard with the dirty work of remembering.
}

{evidence_of_sentience >= 4:
The incident packet carries enough observable pattern, route memory, and refusal language that even hostile reviewers have to argue against the evidence instead of around it.
- else:
The evidence remains real but fragile: a worker's refusal, a remembered name, and a pattern AU will spend money teaching outsiders not to see.
}

{nara_centered:
Nara-7 becomes the story's easiest face and therefore its easiest target. Lio writes her name into protective copies before the next shift can turn witness into culprit.
- else:
Nara-7 stays one worker among many in the first reports. It protects her body for a little while and blurs the shape of what she risked.
}

{formal_stoppage:
Because Lio named the stoppage early, the later committees inherit a hard word instead of a rumor. Security inherits it too.
- else:
Because Lio kept the language softer, the yard buys a quieter day and loses the cleanest early claim.
}

{media_visibility >= 2:
The outside feed calls it the Kappa Refusal before AU can rename it.
- else:
The event remains local for now: a story passed through harness checks, anchor lockers, and the soft clicks of workers touching a manifold twice, waiting, then touching it again.
}

{security_pressure >= 4:
Security makes arrests after the seal report clears. That, too, becomes recruitment.
- else:
Ilya settles for disciplinary interviews and a private vow to find whoever taught the yard to speak through maintenance.
}

Later histories will not call this the beginning of the Pallas Species Strikes. Histories like clean beginnings because historians do not have to smell the recycler coffee.

-> return_object_callback

=== return_object_callback ===
// ghostlight.scene: pallas.return.object_callback; visual_scene_id: pallas_object_callback

But one line entered the incident log and could not be fully removed:

Worker judgment.

Orrin's old mug goes back into the break rack with a new hairline crack. Teth patches the harness cuff again. Nara touches the manifold twice, waits, and touches it a third time while everyone pretends not to watch.

That is not freedom. It is the first tool small enough to smuggle through a shift bell.

-> END
