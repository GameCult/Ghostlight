# Cold Wake: The Minute That Would Not Classify

Status: accepted coordinator/IF scaffold v0. This is not raw responder gold data.
It is the clean branch-and-fold run used to prove the story shape before we drag
another model through the tank.

## Source Grounding

- `E:/Projects/AetheriaLore/Aetheria/Worldbuilding/Pre-Elysium/Timeline/Events/Cold Wake Panic.md`
- `E:/Projects/AetheriaLore/Aetheria/Worldbuilding/Politics/Thermal Signature Warfare.md`
- `E:/Projects/AetheriaLore/Aetheria/Worldbuilding/Pre-Elysium/Factions/Powers/Major/Pan-Solar Consortium.md`
- `E:/Projects/AetheriaLore/Aetheria/Worldbuilding/Pre-Elysium/Factions/Powers/Major/Cetacean Navigators.md`
- `E:/Projects/AetheriaLore/Aetheria/Worldbuilding/Pre-Elysium/Factions/Powers/Major/Aya Collective.md`
- `E:/Projects/AetheriaLore/Aetheria/Worldbuilding/Pre-Elysium/Territories.md`

## Branch Plan

Spine:

1. Route freeze tribunal on Ganymede.
2. Provenance gate through Veyr in a Callisto relay shadow.
3. Aya-aligned clinic gate behind dockside pumpworks.
4. Diagnostic contact climax.
5. Ambiguous rescue resolution.

Fold variables:

- `route_debt`
- `isdra_trust`
- `sella_trust`
- `heat_time`
- `evidence_quality`
- `authority_pressure`
- `clinic_exhaustion`
- `provisional_category`
- `veyr_source_exposed`
- `bay_open`
- `signal_preserved`

Fold policy: every branch in this v0 converges through the same provenance,
clinic, and contact scenes. Choices alter state, available texture, later lines,
image modifiers, and final scars. A larger version may split if the vessel is
lost, Veyr is burned badly enough to become a retaliation route, Sella refuses
contact entirely, or Isdra converts the freeze into active sanction.

## Scene 01: Route Freeze Tribunal

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

Branches:

- `branch-01-ledger-under-freeze`: Maer puts Navigator rescue ledger liability under the waiting cost. Increases `route_debt`, lowers `isdra_trust`, creates red ledger imagery later.
- `branch-01-timer-in-public`: Maer projects the heat timer across every gallery pane. Increases `authority_pressure`, leaves a transcript-visible cost.
- `branch-01-provisional-category`: Maer asks for unresolved distress-bearing thermal evidence. Sets `provisional_category`, improves `isdra_trust` slightly.
- `branch-01-find-veyr-first`: Maer leaves to improve provenance. Improves `evidence_quality`, reduces `heat_time`.

## Scene 02: Callisto Relay Shadow

The next problem waits in the Callisto relay shadow: Veyr Oss, the only source
with a partial chain on the packet's routing behavior.

Veyr's booth is too small for the number of jurisdictions hiding in it. Callisto
light leaks through frosted polymer. Every surface pretends it is not listening.

The packet fragment turns in the air between Maer and Veyr, a little knot of
routing behavior that may be distress, camouflage, or a dead system imitating a
voice because nobody taught it a kinder failure mode.

Branches:

- `branch-02-protect-veyr-source`: Maer asks for behavior without identity. Improves `evidence_quality` modestly and protects Veyr.
- `branch-02-demand-signed-chain`: Maer demands a signed chain. Greatly improves `evidence_quality`, sets `veyr_source_exposed`, increases `authority_pressure`.
- `branch-02-move-with-partial`: Maer takes partial evidence and moves. Preserves time, keeps ambiguity high.

## Scene 03: Sanctuary Clinic Gate

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

Branches:

- `branch-03-ledger-for-sleeve`: Maer offers ledger backing for bounded diagnostic contact. Increases `route_debt` and `clinic_exhaustion`, improves `sella_trust`, sets `bay_open`.
- `branch-03-evidence-to-sella`: Maer shows evidence and lets Sella set terms. Sets `bay_open`, preserves Sella authority.
- `branch-03-press-personhood`: Maer forces the moral frame. Lowers `sella_trust`, increases `clinic_exhaustion`, makes the climax colder and later.

## Scene 04: Diagnostic Contact

The diagnostic sleeve cycles once.

Not a voice. Not proof. A rhythm catches in the preservation channel: three
repeats, a thermal hitch, a routing reflex that answers the Navigator ping by
trying to become smaller.

The vessel dumps heat in a thin white flare beyond the dock cameras. Not enough
to save itself. Enough to stop being invisible.

If the bay opened cleanly, Sella permits bounded contact. Nobody in the room
says rescue like it is clean. Nobody says cargo. The staff work fast and look
older by the minute.

If the bay opened late or under pressure, Sella still cycles the sleeve, but the
room knows who spent the delay. Maer's ledger may bloom red. Veyr's sacrificed
identity may ping under PSC seal. Sella may refuse to look at Maer when the
signal stabilizes.

When the first salvage craft reaches the vessel, it finds no answer that makes
the tribunal happy. It finds a damaged cognition scaffold, three living bodies
in thermal shock, and a routing artifact still repeating the same failed
question through a system never built to hear it.

Cold Wake does not end in Room Seven, or in Sella's clinic, or in the rescue
craft's floodlights. It ends later, badly, everywhere, when the PSC discovers
that a category can be repaired faster than trust.

But on this night, one vessel survives the minute it was supposed to disappear.
That is not justice. It is a receipt.

## Visual Plan

Each scene has one durable base image prompt in the Ink training sidecar, plus
branch/state modifiers. The branch imagery should be edited onto the stable base
scene instead of regenerated from scratch.

Durable anchors:

- Tribunal: split wet/dry tribunal, blue vessel projection, heat countdown.
- Callisto booth: cramped relay shadow, frosted polymer, Veyr identity slates.
- Clinic: Aya repair aesthetics, wet/dry table, amber monitored bay, staff board.
- Contact: diagnostic sleeve, preservation waveform, rescue craft monitor, thin white heat flare.

Branch-visible marks:

- Red ledger debt lines.
- Public heat timer over insurer reflections.
- Gray provisional route lane.
- PSC-sealed Veyr identity ping.
- Amber-white bay state.
- Exhausted clinic staff and guarded Sella posture.

## Review

Accepted as a clean IF scaffold and coordinator/story-runtime sample. Not raw
responder gold. The fixture demonstrates source grounding, branch-and-fold,
non-speech actions, resource consequences, conditional callbacks, and durable
visual prompt structure without making Cold Wake become the whole project again.

