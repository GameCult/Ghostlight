VAR route_debt = 0
VAR isdra_trust = 1
VAR sella_trust = 1
VAR heat_time = 3
VAR evidence_quality = 1
VAR authority_pressure = 1
VAR clinic_exhaustion = 0
VAR provisional_category = false
VAR veyr_source_exposed = false
VAR bay_open = false
VAR signal_preserved = false

-> start

=== start ===
// ghostlight.scene: scene-01-route-freeze-tribunal
// ghostlight.fixture: examples/agent-state.cold-wake-story-lab.json
// aetheria.flashpoint: cold-wake-panic

Ganymede Route Tribunal Room Seven is built like a compromise nobody loves.

The dryside gallery stacks PSC insurers in pale tiers behind glass. The wet
channel runs beneath them in a long pressure-lit arc, deep enough for Navigator
bodies and bright with acoustic notation. Between the two, a shared table
projects the silent vessel as a cold blue absence with a timer burning beside
it.

The tribunal has frozen the vessel's corridor status until someone can name it:
pirate lure, refugee craft, illegal cognition package, privateer decoy, or a
category the forms have not yet learned to fear.

Maer Tidecall watches the heat window count down. Isdra Vel, PSC risk arbiter,
keeps one hand over the freeze order. The room smells of chilled plastic, wet
metal, and people trying to make cowardice audit-safe.

What does Maer do?

* [Put the Navigator rescue ledger under the freeze.]
    // ghostlight.branch: branch-01-ledger-under-freeze
    // ghostlight.action: spend_resource
    // ghostlight.intent: make rescue materially accountable without claiming proof
    -> ledger_under_freeze

* [Make Isdra own the timer in public.]
    // ghostlight.branch: branch-01-timer-in-public
    // ghostlight.action: use_object
    // ghostlight.intent: turn delay into an observable tribunal cost
    -> timer_in_public

* [Ask for a narrow provisional category.]
    // ghostlight.branch: branch-01-provisional-category
    // ghostlight.action: speak
    // ghostlight.intent: create a legal aperture without resolving personhood
    -> provisional_category_request

* [Leave the tribunal and find Veyr before the heat window closes.]
    // ghostlight.branch: branch-01-find-veyr-first
    // ghostlight.action: move
    // ghostlight.intent: improve evidence before asking the system to move
    -> find_veyr_first

=== ledger_under_freeze ===
~ route_debt = route_debt + 2
~ isdra_trust = isdra_trust - 1

Maer's translated voice arrives low and level through the wet channel.

"Put my ledger on the waiting cost. Heat, escort drift, failed extraction. If
this is nothing, I will pay for learning it late. If it is someone, your freeze
will not be the only thing with a signature."

Isdra's mouth tightens. Not anger. Accounting.

"That buys liability, Tidecall. Not permission."

"Liability is where permission starts lying less."

The tribunal records the pledge. The timer keeps eating the room.

-> provenance_gate

=== timer_in_public ===
~ authority_pressure = authority_pressure + 1

Maer does not argue first. He routes the heat-debt countdown from the shared
table to every gallery pane.

The numbers bloom across the glass over each insurer's reflection.

"No new claim," he says. "Only the time your category repair is spending."

Isdra looks up at her own face behind the timer and does not thank him.

"Public pressure is still pressure."

"Yes. This kind leaves a transcript."

-> provenance_gate

=== provisional_category_request ===
~ provisional_category = true
~ isdra_trust = isdra_trust + 1

Maer keeps the packet closed.

"Not person. Not cargo. Not custody. Register it as unresolved distress-bearing
thermal evidence and let the corridor touch it without pretending touch is
adoption."

For the first time, Isdra's hand leaves the freeze order.

"You rehearsed that."

"I survived institutions. Same injury, different scar."

She marks a provisional lane in gray. It is not open. It is no longer sealed.

-> provenance_gate

=== find_veyr_first ===
~ heat_time = heat_time - 1
~ evidence_quality = evidence_quality + 1

Maer leaves the tribunal with the freeze still active and the room pleased to
have mistaken absence for compliance.

The Callisto relay annex keeps its gray-market desks behind vending alcoves and
laundry heat. Veyr Oss waits there with three clean identities and one dirty
one folded under their tongue.

"You are late," Veyr says.

"The law was early. Show me what it broke."

Veyr smiles like a receipt learning to bleed.

-> veyr_gate

=== provenance_gate ===

The next problem waits in the Callisto relay shadow: Veyr Oss, the only source
with a partial chain on the packet's routing behavior.

The tribunal can be moved now, but not safely. The packet still has no clean
origin, no clean claimant, and no clean lie.

How does Maer handle Veyr?

* [Ask Veyr for a usable chain without exposing the source.]
    // ghostlight.branch: branch-02-protect-veyr-source
    // ghostlight.action: speak
    // ghostlight.intent: preserve source safety while improving evidence
    -> protect_veyr_source

* [Demand a signed provenance disclosure.]
    // ghostlight.branch: branch-02-demand-signed-chain
    // ghostlight.action: speak
    // ghostlight.intent: maximize legal usefulness at personal and source cost
    -> demand_signed_chain

* [Take the partial pattern and move.]
    // ghostlight.branch: branch-02-move-with-partial
    // ghostlight.action: transfer_object
    // ghostlight.intent: spend ambiguity rather than time
    -> move_with_partial

=== veyr_gate ===

Veyr's booth is too small for the number of jurisdictions hiding in it. Callisto
light leaks through frosted polymer. Every surface pretends it is not listening.

The packet fragment turns in the air between Maer and Veyr, a little knot of
routing behavior that may be distress, camouflage, or a dead system imitating a
voice because nobody taught it a kinder failure mode.

What does Maer ask of Veyr?

* [Ask Veyr for a usable chain without exposing the source.]
    // ghostlight.branch: branch-02-protect-veyr-source
    // ghostlight.action: speak
    // ghostlight.intent: preserve source safety while improving evidence
    -> protect_veyr_source

* [Demand a signed provenance disclosure.]
    // ghostlight.branch: branch-02-demand-signed-chain
    // ghostlight.action: speak
    // ghostlight.intent: maximize legal usefulness at personal and source cost
    -> demand_signed_chain

* [Take the partial pattern and move.]
    // ghostlight.branch: branch-02-move-with-partial
    // ghostlight.action: transfer_object
    // ghostlight.intent: spend ambiguity rather than time
    -> move_with_partial

=== protect_veyr_source ===
~ evidence_quality = evidence_quality + 1

"I need enough to route care," Maer says. "Not enough to hang you."

Veyr studies him. "That sentence has gotten people killed with excellent
manners."

"Then give me a worse one that works."

They strip the chain down to behavior: repeats, acoustic drift, heat-window
correlation, a routing reflex that keeps curling back toward Ganymede like a
hand closing in sleep.

It will not satisfy a court. It might move a clinic.

-> clinic_gate

=== demand_signed_chain ===
~ evidence_quality = evidence_quality + 2
~ veyr_source_exposed = true
~ authority_pressure = authority_pressure + 1

"Sign it," Maer says.

Veyr goes still.

"You want a clean chain from a dirty room. That is adorable in the way pressure
leaks are adorable."

"I want Sella to spend a bed on something better than my grief."

After a long moment, Veyr burns one identity and signs with another. The packet
grows more useful and more dangerous in the same breath.

-> clinic_gate

=== move_with_partial ===
~ heat_time = heat_time + 0

Maer accepts the partial pattern without making Veyr bleed for a cleaner one.

"That will not carry a tribunal," Veyr says.

"I am not taking it to a tribunal first."

"Ah. A clinic. Much worse. They know what things cost."

-> clinic_gate

=== clinic_gate ===

The Aya-aligned sanctuary clinic sits behind Ganymede's dockside pumpworks,
where warm intake pipes sweat onto repaired floor panels and every public table
has outlived three funding models.

Sella Ren has one monitored bay that can become available if several people
agree to make their own lives worse with precision.

Maer arrives through the Navigator wet channel. The packet waits on a shared
display. Its pattern is stronger now if Veyr helped, uglier if Veyr signed, and
still unresolved either way.

Sella looks at Maer, then at the timer, then at the staff board.

"If this is mercy theater," she says, "I will drown it in paperwork and bill
the funeral."

What does Maer bring to the clinic?

* [Offer ledger backing for a bounded diagnostic sleeve.]
    // ghostlight.branch: branch-03-ledger-for-sleeve
    // ghostlight.action: spend_resource
    // ghostlight.intent: make care possible without demanding full intake
    -> ledger_for_sleeve

* [Show the strongest evidence and let Sella set the terms.]
    // ghostlight.branch: branch-03-evidence-to-sella
    // ghostlight.action: show_object
    // ghostlight.intent: respect clinical authority while making ambiguity visible
    -> evidence_to_sella

* [Press Sella to treat the packet as a possible person now.]
    // ghostlight.branch: branch-03-press-personhood
    // ghostlight.action: speak
    // ghostlight.intent: force moral risk to outrank capacity language
    -> press_personhood

=== ledger_for_sleeve ===
~ route_debt = route_debt + 2
~ clinic_exhaustion = clinic_exhaustion + 1
~ sella_trust = sella_trust + 1
~ bay_open = true

Maer keeps the packet visible but does not push it forward.

"No full intake. No custody. A diagnostic sleeve, heat burden on my ledger,
staff time witnessed, failure paid for."

Sella's eyes narrow, then move to the board.

"You learned. Irritating. Useful."

She marks the bay amber-white: contact permitted, interpretation forbidden.

-> contact_climax

=== evidence_to_sella ===
~ bay_open = true

Maer gives Sella the pattern and waits.

The room does not become noble. It becomes busy. Sella sends one tech to the wet
interface, one recorder behind glass, and nobody extra from warm intake.

"I will not call it someone," she says.

"I did not ask you to."

"Good. Then I can listen."

-> contact_climax

=== press_personhood ===
~ sella_trust = sella_trust - 1
~ clinic_exhaustion = clinic_exhaustion + 1

"If the only safe word is debris," Maer says, "the system will teach itself to
kill quietly."

Sella's hand leaves the bay control.

"Do not make my clinic the stage for your clean sentence. Give me terms I can
staff."

The bay remains closed until Maer steps back from the word he wants most.

-> contact_climax

=== contact_climax ===

The diagnostic sleeve cycles once.

Not a voice. Not proof. A rhythm catches in the preservation channel: three
repeats, a thermal hitch, a routing reflex that answers the Navigator ping by
trying to become smaller.

The vessel dumps heat in a thin white flare beyond the dock cameras. Not enough
to save itself. Enough to stop being invisible.

{bay_open:
    Sella opens the sleeve for bounded contact. Nobody in the room says rescue
    like it is clean. Nobody says cargo. The staff work fast and look older by
    the minute.
- else:
    Sella opens the sleeve late and angry, because the alternative is letting a
    possible claimant vanish while everyone writes excellent minutes.
}

{route_debt >= 4:
    Maer's ledger panel blooms red with live debt lines. Future escorts will
    remember this, and not all of them kindly.
}
{veyr_source_exposed:
    On the side display, Veyr's sacrificed identity pings once under PSC seal.
    The room gains another casualty without a body.
}
{sella_trust <= 0:
    Sella does not look at Maer when the signal stabilizes. Her silence is not
    contempt. It is triage applied to trust.
}

When the first salvage craft reaches the vessel, it finds no answer that makes
the tribunal happy. It finds a damaged cognition scaffold, three living bodies
in thermal shock, and a routing artifact still repeating the same failed
question through a system never built to hear it.

Cold Wake does not end in Room Seven, or in Sella's clinic, or in the rescue
craft's floodlights. It ends later, badly, everywhere, when the PSC discovers
that a category can be repaired faster than trust.

But on this night, one vessel survives the minute it was supposed to disappear.
That is not justice. It is a receipt.

-> END

