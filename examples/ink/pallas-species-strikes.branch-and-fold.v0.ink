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
Pallas Yard Twelve wakes without ceremony. The engineered Bloom shell hums through the deck plates before the shift bell, a low composite note from consolidated asteroid aggregate, TCS substrate, pressure seals, and spin-loaded supports holding a city under air.

It is not a mine and not a factory floor, though Aeronautics Unlimited prices it like both when that helps. It is a built world turned slowly enough to fake down, and Kappa's seals are one of the reasons the air stays on the inside.

-> intro_service_ring

=== intro_service_ring ===
// ghostlight.scene: pallas.intro.service_ring_kappa; visual_scene_id: pallas_service_ring_kappa
Lio Vale walks the primary segment of Service Ring Kappa with a shift-control slate under one arm and yesterday's sleep still folded behind the eyes. Kappa is a bounded local loop around a dense seal-air-thermal equipment cluster: manifold faces on the loop-inner side, anchor hardpoints and staging on the loop-outer side, far-loop lockers and break ledges where the supervisor glass sees less than it pretends. Lio's official job is maintenance coordination: making incompatible bodies, tools, schedules, and liability codes share one dangerous circuit without killing each other before lunch. AU calls this a routine seal-maintenance cycle. The workers call it breathing for people rich enough to forget what keeps the air inside.

Cephalopod shell-artery crews drift in compact dry-operation harnesses shaped for limbs that do not stand: humidity collars, oxygenation loops, pressure cuffs, and neural limiter plugs, all labeled support equipment with AU's usual tenderness for invoices. The rigs keep them alive in dry air, which is not the same as letting them be comfortable. Baseline riggers check sliding anchor blocks along the loop-outer rail and test cross-lane lockout lines against floor sockets and inner hardpoints. Engineered seal techs wait at the loop-inner manifold faces in clean gray skinsuits, each one tagged by function, not by family. Small accommodations make the system look civilized: a humidity credit that keeps gill tissue from cracking, softer tool grips for grafted hands, far-loop break charts that translate three kinds of body clock into one AU shift.

The yard is normal. That is the first horror.

-> intro_nara

=== intro_nara ===
// ghostlight.scene: pallas.intro.nara_manifold; visual_scene_id: pallas_nara_manifold
Nara-7 stands at the Kappa manifold face with the other engineered seal technicians, each gray skinsuit numbered for a task stream instead of a family line. She touches the ribbed service bank twice, pauses, and touches it again with two fingers spread wide beside the Kappa-7 access port. AU calls that retained route behavior. Lio has watched enough shifts to know a warning when it learns manners.

-> intro_orrin

=== intro_orrin ===
// ghostlight.scene: pallas.intro.orrin_anchor_rail; visual_scene_id: pallas_orrin_anchor_rail
Orrin Dax's anchor crew works the loop-outer anchor rail with cold-stiff hands and old tools. They are baseline riggers: born human-standard, paid as if that should be its own reward, and courted after hours by Baseline League mutters about jobs stolen by built workers and uplifted bodies. Their rail does not own the manifold, but no one enters Kappa-7 safely without their cross-lane retrieval geometry: guide lines, haulback tethers, lockout webbing, cart locks, and rescue hooks. Orrin complains like a man trying not to notice he is scared.

-> intro_teth

=== intro_teth ===
// ghostlight.scene: pallas.intro.teth_dry_operation; visual_scene_id: pallas_teth_dry_operation
Teth Inkwise hangs in a dry-operation support harness, four arms managing valves while the rest of the body floats inside compression loops and oxygenation tubes. One cuff has rubbed a pale groove into the mantle where the padding thinned out months ago. Teth never mentions it. The cephalopod crew keeps the service lungs alive in spaces built by companies that modified their bodies to survive dry air, tight corners, and procurement's moral imagination.

-> intro_ilya

=== intro_ilya ===
// ghostlight.scene: pallas.intro.ilya_supervisor_glass; visual_scene_id: pallas_ilya_supervisor_glass
Ilya Marne, Kappa's AU shift superintendent, watches from the glassed operations gallery above the primary manifold segment, its short raised catwalk running back toward the spoke-base control deck. The central equipment cluster blocks direct sightlines to the far loop, so AU fills the gap with cameras, badge logs, and telemetry. She has corporate polish and corporate hunger: schedule first, liability second, people somewhere below the line where the spreadsheet stops showing decimals.

Lio knows the day will probably go wrong. The only question is whether it goes wrong before everyone has finished pretending this is maintenance.

-> ordinary_round_choice

=== ordinary_round_choice ===
// ghostlight.scene: pallas.ordinary_round.choice; visual_scene_id: pallas_ordinary_round_choice
// ghostlight.branch: branch-01-walk-with-nara; action: move; intent: Learn the engineered worker's ordinary hazard routine before crisis.
* [Walk the loop-inner manifold face with Nara-7.]
    ~ evidence_of_sentience += 1
    ~ nara_centered = true
    -> walk_with_nara
// ghostlight.branch: branch-01-sit-with-orrin; action: wait; intent: Sit inside baseline resentment instead of treating it as villain fog.
* [Take the far-loop break ledge beside Orrin Dax.]
    ~ baseline_solidarity += 1
    -> sit_with_orrin
// ghostlight.branch: branch-01-check-teth-harness; action: touch_object; intent: Ground cephalopod body affordances and technical leverage in equipment.
* [Check Teth's dry-operation harness before Kappa-7 entry.]
    ~ cephalopod_leverage += 1
    -> check_teth_harness

=== walk_with_nara ===
// ghostlight.scene: pallas.branch.nara_manifold_walk; visual_scene_id: pallas_nara_manifold_walk
Nara-7 touches the loop-inner Kappa manifold face twice, waits, then touches it a third time with two fingers spread apart beside the Kappa-7 port. Her batch tag says Seal Technician, BioDrone Standard. Her hand says she is listening.

"Kappa sings wrong," she says.

Lio checks the board. The board shows amber pending review, which means nothing has failed in a way AU has agreed to price.

"Wrong how?"

Nara looks at the Kappa-7 False-Seam Artery access port where the patch plates overlap like old scars. "From before reset."

She does not say memory. She does not say Rell. She gives Lio just enough truth to carry and no more, because truth in this yard is heavy and management keeps scales.

-> chokepoint_refusal

=== sit_with_orrin ===
// ghostlight.scene: pallas.branch.orrin_break_table; visual_scene_id: pallas_orrin_break_table
Orrin Dax has a mug of recycler coffee in one hand and a dry anchor hook across his knees. The far-loop break ledge sits beyond the clean reach of the operations glass, close to lockers, cart locks, and the hatches workers use when they want a sentence to stay human for ten seconds. The hook is older than the shift board and better maintained.

"Soft-sleeve specialists got a humidity credit again," he says, not looking at Lio. "My crew gets frost rash and a lecture about grit."

Across the galley, two engineered techs eat in precise silence. Orrin watches them the way a man watches his own job walking around in someone else's skin.

"Funny how the future always needs fewer of us," he says.

Lio lets the complaint breathe before answering. "Future still needs anchor geometry signed before Kappa-7 support can carry load."

Orrin snorts. "Then tell the future to remember who taught it where the bolts are."

-> chokepoint_refusal

=== check_teth_harness ===
// ghostlight.scene: pallas.branch.teth_harness_check; visual_scene_id: pallas_teth_harness_check
Teth Inkwise turns in the dry-operation harness, four arms working while the rest of the body hangs in careful suspension. The cephalopod zero-g maintenance harness is not a suit so much as a negotiated truce between flesh, dry air, tool, coolant, and vacuum: compression anchor loops, humidity collar, oxygenation tubes, pressure cuffs, limiter socket, a rail of sealed implements, tiny valves patched by someone who trusted skill more than procurement. Each motion is smooth enough to look effortless, which is how the yard prefers its suffering: technically impressive and quiet.

"Your cuff is patched again," Lio says.

"Everything noble is patched," Teth replies. "Including this yard's conscience."

The joke lands dry. Teth's eyes stay on the pressure ghost beneath the glass, and one free arm adjusts the oxygenation line before the warning tremor can become visible.

The service lung diagram flickers under one of Teth's arms. Kappa-7 runs through the equipment cluster and then bends Bloom-outward into seal-lung pockets and coolant bypass cuffs. If Kappa goes hard red, AU will discover exactly how much of its schedule lives in equipment designed for bodies it refuses to honor.

-> chokepoint_refusal

=== chokepoint_refusal ===
// ghostlight.scene: pallas.threshold.chokepoint_refusal; visual_scene_id: pallas_chokepoint_refusal
The shift bell rings. The board orders entry into Kappa-7 False-Seam Artery.

Nara-7 stops at the Kappa-7 access port. Three other engineered seal techs stop with her. They touch the loop-inner manifold face in the same sequence: twice, wait, third touch spread wide.

The board does not understand a ritual unless someone bills it.

"Proceed," Ilya Marne says over the supervisor channel.

Nara's voice is small and clear. "No entry into Kappa-7. Rell's Cut remembers. We remember Rell."

The baseline anchor riggers go quiet. Teth's harness valves click once, then stop.

-> ilya_arrival

=== ilya_arrival ===
// ghostlight.scene: pallas.threshold.ilya_arrival; visual_scene_id: pallas_ilya_arrival
Ilya leaves the operations glass fast enough to prove she had been watching the primary segment. She crosses the raised supervisor catwalk from the spoke-side control door and drops into the work ring through a badge-only stair. The far loop is still partly hidden behind machinery, but the Kappa-7 face is under glass, camera, and board. The room changes shape around her: not physically, not enough for a safety board, but enough for every worker to know the yard has stopped being a place where a problem can be fixed quietly.

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
"Safety pause," Lio says. "Local channel. Kappa amber, worker hazard report, no Kappa-7 entry until Teth confirms load and Orrin's crew signs the rigging geometry."

Ilya's eyes narrow at the word worker. She hears the legal trap even when Lio buries it in maintenance language.

The pause buys minutes. It also keeps the first evidence small enough that AU might try to pocket it.

-> tests_and_reversals

=== formalize_stoppage ===
// ghostlight.scene: pallas.branch.formalize_stoppage; visual_scene_id: pallas_formalize_stoppage
Lio keys the log before fear can bargain the words down.

"Coordinated labor stoppage at Kappa-7 access. Cause: remembered injury, active hazard, refusal of unverified False-Seam entry."

The channel goes cold. Someone in the far loop whistles once. Ilya stops moving like a person and starts moving like a policy document with legs.

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
The official board calls it irregularity. The worker channel calls it refusal.

-> bioelevate_liability_frame

=== bioelevate_liability_frame ===
// ghostlight.scene: pallas.reversal.bioelevate_liability_frame; visual_scene_id: pallas_bioelevate_liability_frame
BioElevate legal, summoned by Ilya with two clipped phrases, calls it potential scaffold drift.

BioElevate built and leased the engineered seal tech line under language that turned cognition into product behavior and injury into maintenance variance. If Nara remembers Rell, BioElevate has not merely sold AU labor. It has sold AU a worker and called her equipment.

The Awakened Labor Front mirror sits outside AU's incident board: a recognition channel for created and altered workers, slow, underfunded, and inconveniently hard to edit once a packet lands there. Its little green receipt light means a copy has left the yard for volunteers who cannot stop retaliation, but can make erasure expensive.

{formal_stoppage:
The formal log has already narrowed everyone's exits. Ilya can punish a stoppage, but she cannot pretend no one named it.
}

Orrin's anchor crew waits by the anchors. Their faces say what nobody wants to admit: if engineered workers are people, then baseline workers have been angry at the wrong enemy and still poor afterward.

{life_support_margin <= 2:
    The air plant pings a margin warning. Every political word in the ring gets heavier.
- else:
The Kappa pressure-margin clock is still kind. Not generous, but kind enough to make choices real.
}

// ghostlight.branch: branch-03-show-nara-memory; action: show_object; intent: Use observable route memory as evidence without forcing private confession.
* [Ask Nara to mark the remembered cutting path on Lio's slate.]
    ~ evidence_of_sentience += 2
    ~ bioelevate_liability += 1
    ~ nara_centered = true
    -> show_nara_memory
// ghostlight.branch: branch-03-ask-orrin-solidarity; action: speak; intent: Ask baseline riggers to defend safety through their own expertise.
* [Ask Orrin to put anchor-rail authority behind the refusal.]
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

Nara taps the Kappa-7 diagram with frightening precision: two safe plates, one false seam, the cut line where Rell's suit opened, the manual valve AU replaced in the report but not in the wall.

BioElevate legal asks whether the marks might be trained route residue.

Orrin says, "Trained route residue does not remember the part procurement lied about."

-> climax

=== ask_orrin_solidarity ===
// ghostlight.scene: pallas.branch.orrin_solidarity; visual_scene_id: pallas_orrin_solidarity
Lio does not ask Orrin to be noble. That would insult everyone present.

"Anchor rigging decides whether Kappa-7 access can carry a living body. You know the route. Are you signing the cross-lane lockout and haulback as safe?"

Orrin looks at Nara, then at the Kappa-7 port, then at the faces of his own crew.

"No," he says. "And if anyone writes that I did, I will personally introduce them to the anchor hook."

The laugh that follows is thin, but it is shared.

-> climax

=== bypass_kit ===
// ghostlight.scene: pallas.branch.bypass_kit; visual_scene_id: pallas_bypass_kit
Teth opens a service pouch and produces a bypass bridge wrapped in maintenance cloth, the kind of device that officially does not exist because officially nobody needs to survive a strike.

"This gives Kappa minutes," Teth says. "Not obedience. Minutes."

The dry-operation crew moves like a quiet animal with too many hands and exactly enough discipline, oxygenation tubes flexing as bodies fold through workspaces AU made cheap because they could. Teth takes the ugly part of the load without announcing it, mantle tightening once against the collar before the arms find the next valve. Pressure steadies.

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
Orrin's anchor crew starts throwing the cross-lane lockout web without waiting for an AU order: sliding blocks on the loop-outer rail, bright tethers to floor sockets, haulback lines clipped to inner hardpoints around Kappa-7.
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

Ilya demands a decision: recognition language in the incident log, {bypass_ready:immediate bypass control under AU authority, |sealed evidence custody, }or emergency clearance to force Kappa-7 entry.

// ghostlight.branch: branch-04-hold-for-recognition; action: speak; intent: Hold safety work behind formal recognition of the refusal as worker judgment.
* [Hold the line until Ilya records worker judgment.]
    ~ strike_cohesion += 2
    ~ security_pressure += 1
    {life_support_margin <= 1:
      ~ worker_injury += 1
    }
    -> hold_for_recognition
// ghostlight.branch: branch-04-trade-bypass-for-evidence; action: transfer_object; intent: Trade temporary bypass control for preserved evidence and no forced Kappa-7 entry.
* {bypass_ready} [Trade bypass control for sealed evidence and no forced Kappa-7 entry.]
    ~ life_support_margin += 1
    ~ evidence_of_sentience += 1
    ~ bioelevate_liability += 1
    -> trade_bypass_for_evidence
// ghostlight.branch: branch-04-seal-evidence-without-bypass; action: transfer_object; intent: Preserve evidence and forbid forced Kappa-7 entry when there is no bypass leverage to trade.
* {not bypass_ready} [Seal the evidence packet and forbid forced Kappa-7 entry.]
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

"Bypass under shared custody. Sealed packet to ALF mirror, AU incident board, and yard safety. No forced Kappa-7 entry."

Ilya accepts because the alternative is a casualty report she cannot classify out of existence. BioElevate legal immediately begins sanding the word memory into residue, but the packet has already left the room.

-> return_fold

=== seal_evidence_without_bypass ===
// ghostlight.scene: pallas.branch.seal_evidence_without_bypass; visual_scene_id: pallas_seal_evidence_without_bypass
Lio lifts the slate where the inspector cam can see the custody seal.

"Packet to yard safety, AU incident board, and the Awakened Labor Front mirror. No Kappa-7 entry while Kappa is red. If you want a body in Rell's Cut, you will sign your name above it."

There is no bypass bridge to soften the bargain. The margin stays ugly. But the evidence now has three homes, and none of them are inside Ilya's pocket.

-> return_fold

=== orrin_front_demand ===
// ghostlight.scene: pallas.branch.orrin_front_demand; visual_scene_id: pallas_orrin_front_demand
Orrin steps forward before Lio has to spend Nara again.

"Anchor crew refuses the Kappa-7 rig," he says. "Our call. Our anchors. You want a body in Rell's Cut, superintendent, put the board in first."

For one bright second, the old lie breaks: baseline and engineered labor are not natural enemies. They are different answers to the same invoice.

-> red_alert_worker_judgment

=== red_alert_worker_judgment ===
// ghostlight.scene: pallas.branch.red_alert_worker_judgment; visual_scene_id: pallas_red_alert_worker_judgment
Teth's arms move through the harness in a blur of exact mercy, bleeding surge through hand valves and patched cuffs that procurement will later describe as adequate. The ugly part of the load goes through Teth first; the mantle tightens once against the collar, then the arms find the next valve.

Ilya looks at the red board, at Nara's steady hands, at Orrin's crew, at Teth's harness flexing without a bridge to spend. She hates every door and all of them are open.

"Worker judgment in the incident log," Lio says. "No forced Kappa-7 entry."

"Temporary language," Ilya says.

"Logged language."

The superintendent records the phrase with a face like broken glass hidden under a clean cloth.

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
The event remains local for now: a story passed through harness checks, far-loop lockers, cross-lane anchor lines, and the soft clicks of workers touching a manifold face twice, waiting, then touching it again.
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

Orrin's old mug goes back into the far-loop break rack with a new hairline crack. Teth patches the harness cuff again. Nara touches the Kappa-7 manifold face twice, waits, and touches it a third time while everyone pretends not to watch.

That is not freedom. It is the first tool small enough to smuggle through a shift bell.

-> END
