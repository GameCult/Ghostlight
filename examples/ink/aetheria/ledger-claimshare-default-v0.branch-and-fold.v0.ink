// ghostlight.artifact_id: ledger_claimshare_default_branch_fold_v0
// ghostlight.fixture_id: ledger-claimshare-default-v0
// ghostlight.scene_id: ledger-claimshare-default-v0.kappa-settlement-freeze
// ghostlight.final_ink_path: examples/ink/aetheria/ledger-claimshare-default-v0.branch-and-fold.v0.ink

VAR body_margin = 1
VAR worker_cohesion = 1
VAR default_evidence = 0
VAR queue_minutes = 4
VAR supervisor_pressure = 1
VAR claims_custody = 1
VAR certificate_margin = 3
VAR mutual_book_live = false
VAR cartridge_reserved = false
VAR lease_receipt_checked = false

-> start

=== start ===
// ghostlight.scene: kappa_shift_gate_establishing
Service Ring Kappa curves through the outward shell of a Pallas Bloom, close enough to the open industrial air that every pressure cough can be heard by people eating breakfast two decks inward.

The ring is a human-height maintenance gallery wrapped around seal lungs, coolant cuffs, condensate traps, and two specialist crawl throats. Beyond its inward mesh rail, cranes move under the manufactured habitat's pale light spine. Above the ring, the glassed operations gallery has its own badge stair. Authority dislikes sharing a queue.

At the spoke-side shift gate, Ramp Administration has fitted a claimshare kiosk between the tool lockers and the dry-operation rig rack. Its three chairs belong to Miri Dan, the morning queue, and a printer that jams only when asked to document responsibility.

-> ordinary_queue

=== ordinary_queue ===
// ghostlight.scene: ordinary_claimshare_queue
Miri is the settlement clerk. She can release local stores, authenticate work, and decide which claim enters which queue. She cannot make AU pay a bill.

Forty-three junior claimshares sit in her own household account. They cover half next month's berth rent if Kappa stays recognized, supplied, and open. This is called participation in frontier growth by people whose rent clears in cash.

Oru Loopwise waits at the low counter in a mobile dry-operation harness: rust-red mantle under a clear humidity skin, oxygenation tubes running to a compact pump, padded cuffs supporting several tentacles, tool clips trembling softly with the pump. Oru's cartridge whistles on the inhale.

"It has discovered management," Oru says.

"Does it schedule meetings?" Miri asks.

"Only inside me."

Sef Anwar, a baseline anchor rigger, stands behind Oru with three households' claims folded into a grease-marked book. Food allotment, clinic accompaniment, berth power. Small words doing structural work.

The shift clock shows four minutes before Kappa's morning certificate window.

-> preparation_choice

=== preparation_choice ===
// ghostlight.choice_layer: morning_settlement_routine
+ [Reserve Oru a fresh oxygenation cartridge before opening the general queue.]
    // ghostlight.action_label: allocate_object
    // ghostlight.branch_label: prepare_cartridge
    ~ cartridge_reserved = true
    ~ body_margin = body_margin + 2
    ~ queue_minutes = queue_minutes - 1
    ~ supervisor_pressure = supervisor_pressure + 1
    Miri turns the local-store key and moves one sealed cartridge into Oru's named tray.

    The terminal asks whether the issue is medical, equipment, or productivity support. Miri selects all three. The terminal accepts none, which at least proves it can count.

    Oru lays one tentacle tip against the tray seal. "Mine when opened?"

    "Yours when opened. Mine until the audit."

    "A beautiful system. Everyone gets anxiety."
    -> routine_fold
+ [Open the actuator lease receipt and reconcile it against last night's maintenance log.]
    // ghostlight.action_label: inspect_record
    // ghostlight.branch_label: inspect_lease_receipt
    ~ lease_receipt_checked = true
    ~ default_evidence = default_evidence + 2
    ~ queue_minutes = queue_minutes - 1
    Miri opens the lease pane for Kappa's pressure-equalization actuator.

    The physical unit passed inspection at 03:10. The payment receipt says pending. The supplier service line says current. Three respectable statements occupy the same screen without making eye contact.

    Miri stamps a local copy before the pane can become more current than history.
    -> routine_fold
+ [Balance Sef's three-household book against the ramp queue.]
    // ghostlight.action_label: compare_records
    // ghostlight.branch_label: balance_household_book
    ~ mutual_book_live = true
    ~ worker_cohesion = worker_cohesion + 2
    ~ claims_custody = claims_custody + 1
    ~ queue_minutes = queue_minutes - 1
    Sef opens the book across the counter. Its columns are blunt: who ate, who sat through clinic, who lent a cartridge, who owes repair time.

    Miri checks each amount against the ramp ledger. The household book does not pretend AU has paid. It records who has kept whom alive while waiting.

    "Your totals disagree by one meal," Miri says.

    "My son had seconds. We are appealing the extravagance."
    -> routine_fold
+ [Clear the queue in screen order and protect the certificate window.]
    // ghostlight.action_label: authenticate_queue
    // ghostlight.branch_label: protect_queue_time
    ~ queue_minutes = queue_minutes + 1
    ~ certificate_margin = certificate_margin + 1
    ~ supervisor_pressure = supervisor_pressure - 1
    Miri lets the ramp order the morning: senior tools, certified bodies, current households, everyone else.

    The queue moves. Oru's whistling cartridge remains attached. Sef closes his book without comment.

    For ninety seconds the kiosk achieves the administrative ideal: no visible disagreement and several hidden emergencies.
    -> routine_fold

=== routine_fold ===
// ghostlight.fold: morning_queue_before_default
The printer coughs out shift strips. Magnetic boots strike the curved gallery floor. At the rig rack, cephalopod crews check humidity skins and oxygenation loops while baseline riggers count anchor hooks and VitaForge workers stand for limiter diagnostics.

{cartridge_reserved:
Oru's sealed replacement waits in the named tray, small enough to hold in one hand and expensive enough to require three ownership categories.
- else:
Oru's old cartridge whistles beside the queue. Every third breath sounds like a clerk reconsidering a fee.
}

{lease_receipt_checked:
Miri's stamped local receipt sits under her palm. The actuator works. The payment does not.
}

{mutual_book_live:
Sef leaves the three-household book open at the counter, grease and thumbprints holding a second account of the morning.
}

{queue_minutes >= 5:
The shift clock still has room in it. The queue does not.
- else:
The clock has begun eating the edges of every careful act.
}

Then the lease line turns amber.

-> default_cascade

=== default_cascade ===
// ghostlight.scene: actuator_lease_default
The message is almost shy.

KAPPA PRESSURE-EQUALIZATION ACTUATOR: LEASE PAYMENT NOT SETTLED.

{lease_receipt_checked:
Miri sets her stamped copy beside the notice. Inspection passed before the supplier changed the meaning of access.
- else:
Miri opens the maintenance pane too late to preserve its earlier state. The actuator still works; the diagnostic license has already become read-only.
}

The next notices arrive with the confidence of relatives who heard there was property.

The supplier holds the replacement actuator for cash settlement. The certifier narrows Kappa to one supervised shift. The insurer marks uncovered throughput outside that window. The freight desk moves filters and manifold seals to prepayment.

-> claimshare_freeze

=== claimshare_freeze ===
// ghostlight.scene: junior_claimshare_freeze
The ramp ledger freezes junior claimshares.

Miri's forty-three become visible and unspendable. Sef's food allotment turns gray. Oru's cartridge tray asks for cash custody. Nothing physical has moved. That is the trick: the obligations move first, and bodies discover the new geography afterward.

~ certificate_margin = certificate_margin - 2
~ queue_minutes = queue_minutes - 1
~ supervisor_pressure = supervisor_pressure + 1

Above the counter, the operations gallery glass changes from clear to blue-white command tint. Superintendent Kara Mott is already on the badge stair.

The kiosk gives Miri one local action before Kara reaches the ring.

-> freeze_response

=== freeze_response ===
// ghostlight.choice_layer: freeze_response
+ [Open Oru's cartridge and the waiting clinic packs under local-store custody.]
    // ghostlight.action_label: open_custody
    // ghostlight.branch_label: release_body_support
    ~ cartridge_reserved = true
    ~ body_margin = body_margin + 2
    ~ supervisor_pressure = supervisor_pressure + 2
    ~ queue_minutes = queue_minutes - 1
    Miri breaks the cartridge seal and pushes the compact pump module across the counter. She opens two clinic packs in the same motion.

    Oru unclips the whistling line with three tentacles, braces the harness with two more, and seats the clean cartridge. The pump settles into a low, even hum.

    The ledger calls the stock disputed. Oru calls it breathable.
    -> kara_arrives
+ [Print the entire frozen queue and place authenticated copies in three household books.]
    // ghostlight.action_label: transfer_records
    // ghostlight.branch_label: mirror_claims_locally
    ~ mutual_book_live = true
    ~ claims_custody = claims_custody + 2
    ~ worker_cohesion = worker_cohesion + 1
    ~ queue_minutes = queue_minutes - 2
    Miri asks the printer for the full queue.

    It jams.

    She removes the approved paper tray, feeds the claim roll by hand, and the machine reluctantly documents who was owed what before the freeze. Sef tears the strip into three witnessed copies instead of one convenient original.

    The printer displays UNAUTHORIZED DUPLICATION. Miri signs that too.
    -> kara_arrives
+ [Read the whole cascade over the Kappa work channel.]
    // ghostlight.action_label: speak
    // ghostlight.branch_label: name_default_chain
    ~ default_evidence = default_evidence + 1
    ~ worker_cohesion = worker_cohesion + 1
    ~ supervisor_pressure = supervisor_pressure + 2
    ~ queue_minutes = queue_minutes - 1
    Miri keys the work channel.

    "One missed lease payment. Diagnostics read-only. Replacement cash-only. Certificate narrowed. Coverage conditional. Freight prepaid. Junior claims frozen."

    Along Kappa, tool noise stops in sections as different bodies hear the part that owns them.

    Oru says, "Please repeat the first cause. Management usually starts at our refusal."

    Miri repeats it.
    -> kara_arrives
+ [Publish Kara's proposed senior hazard priority and keep the shift clock alive.]
    // ghostlight.action_label: publish_order
    // ghostlight.branch_label: publish_hazard_priority
    ~ certificate_margin = certificate_margin + 1
    ~ claims_custody = claims_custody - 1
    ~ supervisor_pressure = supervisor_pressure - 1
    ~ queue_minutes = queue_minutes + 1
    Miri accepts the waiting instruction before Kara can deliver it in person.

    SENIOR SETTLEMENT PRIORITY FOR COMPLETED HAZARD CYCLE.

    The shift clock gains one minute. Junior household claims fall another place. The screen calls this restored confidence.
    -> kara_arrives

=== kara_arrives ===
// ghostlight.scene: superintendent_arrival
Kara descends from the glass operations gallery by the badge stair, clean coat first, polished boots second, the rest of the institution arranged behind her.

"The actuator remains functional," she says. "Kappa completes one cycle. Certified output releases settlement."

"For whom?" Sef asks.

"Senior claims first."

{claims_custody >= 3:
Three authenticated paper strips lie in separate hands. Kara can see that the frozen queue has acquired witnesses.
- else:
The ramp screen remains the cleanest account in the room, which is another way of saying it can still forget people alone.
}

{body_margin >= 3:
Oru's pump hums evenly. The harness is ready for safety work, not consented production.
- else:
Oru's pump whistles. Kara glances at it once and returns her eyes to the certificate clock.
}

{supervisor_pressure >= 4:
Two blue-lit security drones settle at the far shift gate. Kara has brought an answer before hearing the question.
- else:
The far shift gate remains open, though its status light has turned watchful amber.
}

{worker_cohesion >= 3:
Baseline riggers, VitaForge workers, and cephalopod crews have stopped standing in supplier groups.
- else:
The queue retains its contract lanes. Everyone is close enough to breathe the same air and still easy to bill separately.
}

The certificate window ticks toward zero. Miri owns the local work authentication. Kara owns the ramp order. Oru and the Kappa crews own whether their bodies enter the throats, though every screen in the room has found a softer verb.

-> settlement_threshold

=== settlement_threshold ===
// ghostlight.fold: default_chain_to_settlement_choice
{default_evidence >= 2:
Miri has a legible first cause: the lease default preceded the labor refusal.
- else:
Miri has the notices as they stand now, not the order in which the machine made them true.
}

{mutual_book_live:
Sef's household book is open beside the frozen ramp ledger. It cannot import a manifold assembly. It can decide who eats tonight.
}

{queue_minutes <= 1:
The remaining minute is too small for caution and exactly large enough for blame.
- else:
There is time for one bounded decision, which is more than the ramp intended to leave them.
}

Miri places her authentication palm above the counter. She can certify production, refuse and seal the chain, or authorize maintenance-only safety work under the local household book.

-> final_choice

=== final_choice ===
// ghostlight.choice_layer: settlement_authority
+ [Authenticate one production cycle under senior hazard priority.]
    // ghostlight.action_label: authenticate_work
    // ghostlight.branch_label: certify_hazard_cycle
    {certificate_margin >= 2 && body_margin >= 3 && queue_minutes >= 1:
        Miri authenticates the shift. The Kappa doors release under supervised production terms.
        -> ending_certificate_holds
    - else:
        Miri authenticates the shift with too little margin in the body, the clock, or the certificate.
        -> ending_certificate_cost
    }
+ [Refuse production and seal the lease-to-claim chain into public custody.]
    // ghostlight.action_label: refuse_and_seal
    // ghostlight.branch_label: record_first_cause
    {default_evidence >= 2 && claims_custody >= 3:
        Miri refuses the production code and seals the stamped lease receipt beside three copies of the frozen queue.
        -> ending_default_recorded
    - else:
        Miri refuses, but the record has gaps and too few independent holders.
        -> ending_refusal_scattered
    }
+ [Authorize a maintenance-only watch and let the households honor food, clinic time, and cartridges themselves.]
    // ghostlight.action_label: limit_work
    // ghostlight.branch_label: household_safety_watch
    {mutual_book_live && worker_cohesion >= 3 && body_margin >= 3:
        Miri marks the shift SAFETY WATCH: NO CERTIFIED OUTPUT. Sef opens the household book; Oru gives the first maintenance limit.
        -> ending_household_bridge
    - else:
        Miri marks the safety watch, but the bodies, records, or relationships needed to carry it are not ready.
        -> ending_household_cost
    }

=== ending_certificate_holds ===
// ghostlight.ending_label: certificate_holds
// ghostlight.training_hook: production_restores_the_claim_that_compels_production
Kappa completes one cycle.

The seal lungs equalize. The industrial yard keeps its open air. Certified output releases enough settlement to clear senior hazard claims and the actuator lease.

Junior balances remain frozen. Miri's forty-three shares are worth something again because workers entered the machinery that had just made them worthless.

{cartridge_reserved:
Oru returns with an even pump and a fresh abrasion where the harness met a pressure cuff.
- else:
Oru returns breathing through the old whistle. The new cartridge remains property with excellent attendance.
}

Kara calls the cycle continuity. Sef writes the cleared lease above three unpaid meals.

The printer produces a receipt without jamming. Nobody trusts it more for the gesture.
-> END

=== ending_certificate_cost ===
// ghostlight.ending_label: certificate_cost
// ghostlight.training_hook: hazard_cycle_under_insufficient_margin
The shift starts because the screen can still release a door.

The first pressure cough drives Kappa into manual bypass. Output fails certification. The insurer keeps the exception and rejects the production claim. Freight remains prepaid. Junior claimshares remain gray.

{body_margin < 3:
Oru leaves the throat early, pump whistling hard, and three workers spend the remaining safety margin getting one body back to the rig rack.
- else:
Oru calls the withdrawal before the actuator loads. The body comes back intact; the certificate does not.
}

Kara records incomplete labor performance. Miri keeps the default notices open beside it until the screen times out.
-> END

=== ending_default_recorded ===
// ghostlight.ending_label: default_chain_preserved
// ghostlight.training_hook: first_cause_survives_distributed_custody
The production window closes.

The operating certificate suspends. The freight desk holds the manifold assembly. The ramp loses a day's output and announces an unauthorized work interruption.

It cannot make the lease default disappear first.

One stamped receipt and three frozen-queue copies leave Kappa by different doors: worker shift gate, support-rig prep route, and spoke transit. No copy can import the missing part. Together they prevent AU from beginning the story with worker refusal.

Kara keeps the ramp. The workers keep the order of events.
-> END

=== ending_refusal_scattered ===
// ghostlight.ending_label: refusal_without_custody
// ghostlight.training_hook: correct_refusal_with_weak_evidence
Miri refuses. The certificate suspends anyway.

Kara's report reaches the ramp office before Miri's partial packet leaves the kiosk. It records a labor delay, an expired window, and a disputed lease status. All three are true. Their order is profitable.

The manifold assembly remains at freight. Sef's household book carries amounts but not the first cause. Oru remembers the pump whistle and the closed throat.

Truth survives in people who cannot yet make the same document.
-> END

=== ending_household_bridge ===
// ghostlight.ending_label: household_safety_bridge
// ghostlight.training_hook: mutual_aid_preserves_bodies_without_claiming_systemic_victory
No production enters the certificate.

Oru assigns two crews to watch the seal lungs from the human-scale ring and refuses every specialist throat. Baseline riggers hold the manual bypass. VitaForge workers relay sensor changes the supplier pane has stopped interpreting.

Sef's three-household book releases one food allotment, two clinic accompaniments, and Oru's cartridge outside the frozen ramp priority. Miri authenticates custody, not value.

The imported actuator remains at freight. Tomorrow is worse. Tonight, the open industrial air holds and nobody enters Kappa-7 to make an invoice feel alive.

Nobody calls it a movement. It is six households, a borrowed cartridge, and a promise small enough for everyone present to inspect.
-> END

=== ending_household_cost ===
// ghostlight.ending_label: household_safety_cost
// ghostlight.training_hook: mutual_aid_without_enough_material_margin
Miri authorizes maintenance-only work. The certificate suspends. The ramp ledger stays frozen.

The crews try to build a watch from whatever survived the queue.

{mutual_book_live:
Sef's book can move food and time, but not enough body support for every specialist route.
- else:
Sef begins a household page while the shift is already failing. Names arrive faster than stores.
}

{body_margin < 3:
Oru cannot take the old cartridge into a pressure throat. The crews preserve the ring and surrender the deeper sensor watch.
- else:
Oru can work, but too few contract lanes have crossed into one crew to hold every bypass.
}

The industrial yard loses pressure margin by degrees. Evacuation begins inward, toward the civic decks. Six households keep their people together in the queue.

It is not enough. It is also not nothing.
-> END
