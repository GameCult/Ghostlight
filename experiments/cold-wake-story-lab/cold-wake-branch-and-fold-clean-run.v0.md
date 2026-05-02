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
clinic, and contact scenes, but the variables now actually change the fixture.
They alter available choices, clinic prose, image modifiers, and final outcome
texture. A larger version may split if the vessel is lost, Veyr is burned badly
enough to become a retaliation route, Sella refuses contact entirely, or Isdra
converts the freeze into active sanction.

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
- `branch-01-timer-in-public`: Maer projects the heat timer across every gallery pane. Increases `authority_pressure`, costs `heat_time`, leaves transcript-visible pressure.
- `branch-01-provisional-category`: Maer asks for unresolved distress-bearing thermal evidence. Sets `provisional_category`, improves `isdra_trust`, unlocks a clinic option later.
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

- `branch-02-protect-veyr-source`: Maer asks for behavior without identity. Improves `evidence_quality`, costs one `heat_time`, protects Veyr.
- `branch-02-demand-signed-chain`: Maer demands a signed chain. Greatly improves `evidence_quality`, costs two `heat_time`, sets `veyr_source_exposed`, increases `authority_pressure`. This option is unavailable if too little time remains.
- `branch-02-move-with-partial`: Maer takes partial evidence and moves. Preserves `heat_time`, keeps `evidence_quality` low, narrowing clinic options unless another prior state compensates.

## Scene 03: Sanctuary Clinic Gate

The Aya-aligned sanctuary clinic sits behind Ganymede's dockside pumpworks,
where warm intake pipes sweat onto repaired floor panels and every public table
has outlived three funding models.

Sella Ren has one monitored bay that can become available if several people
agree to make their own lives worse with precision.

Maer arrives through the Navigator wet channel. The packet waits on a shared
display.

The clinic now checks state directly:

- High `evidence_quality` makes the packet harder to dismiss and opens the strong-evidence branch.
- Partial `evidence_quality` opens a weaker branch whose outcome depends on `provisional_category`.
- `provisional_category` appears as a gray legal lane and opens its own option.
- High `authority_pressure` follows the packet as a PSC priority flag; without `provisional_category`, strong evidence can become audit contamination instead of permission.
- Low `heat_time` changes the clinic description and can turn rescue into salvage.
- Prior `route_debt` appears before Sella decides whether Maer brought material terms.

Sella looks at Maer, then at the timer, then at the staff board.

"If this is mercy theater," she says, "I will drown it in paperwork and bill
the funeral."

Branches:

- `branch-03-ledger-for-sleeve`: Maer offers ledger backing for bounded diagnostic contact. Available only while `heat_time` remains. Increases `route_debt` and `clinic_exhaustion`, improves `sella_trust`, sets `bay_open`.
- `branch-03-evidence-to-sella-strong`: Available only at high `evidence_quality`. Sets `bay_open`, improves `sella_trust`, preserves Sella authority.
- `branch-03-evidence-to-sella-partial`: Available at partial `evidence_quality`. If `provisional_category` is true, it opens the bay with exhaustion cost; otherwise Sella refuses and trust drops.
- `branch-03-use-provisional-lane`: Available only if `provisional_category` and remaining time exist. Opens the bay with clinic exhaustion cost.
- `branch-03-press-personhood`: Always available, but weak evidence and low time make it worse. Lowers `sella_trust`, increases `clinic_exhaustion`, and may leave `bay_open` false.

## Scene 04: Diagnostic Contact

The diagnostic sleeve cycles once.

If `heat_time` remains, the preservation channel catches a coherent ambiguous
signal: three repeats, a thermal hitch, a routing reflex that answers the
Navigator ping by trying to become smaller.

If `heat_time` has fallen to zero, the sleeve cycles late. The vessel's heat
dump has already begun outside the safe window, and the channel catches only
fragments.

If `bay_open` is true, Sella opens bounded contact under terms. If false, she
opens late and angry because the alternative is worse, and the delay is not
forgiven just because the door finally moves.

Other variables carry through visibly:

- High `evidence_quality` gives the rescue craft a sharper lock.
- Low `evidence_quality` makes the rescue craft fly half-blind.
- `provisional_category` keeps the tribunal from slamming the channel shut.
- High `authority_pressure` opens a PSC penalty file and can make Maer's future escort petitions pay for this minute.
- High `route_debt` blooms red on Maer's ledger.
- High `clinic_exhaustion` shows staff error and depleted capacity.
- Low `sella_trust` removes eye contact and warmth from the contact beat.
- `veyr_source_exposed` leaves a PSC-sealed identity ping in the room.

Outcome fold:

- If `heat_time` remains, the craft finds a damaged cognition scaffold, three living bodies in thermal shock, and unresolved routing testimony.
- If `heat_time` is gone, the craft finds two living bodies, one body too late for any category, and the same unresolved scaffold.

Cold Wake does not end in Room Seven, or in Sella's clinic, or in the rescue
craft's floodlights. It ends later, badly, everywhere, when the PSC discovers
that a category can be repaired faster than trust.

But the state now changes what kind of receipt survives.

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
- Fragmented waveform and harsher red alarms if `heat_time` runs out.

## Review

Accepted as a clean IF scaffold and coordinator/story-runtime sample. Not raw
responder gold. Revised after review because the first pass tracked variables
that did not matter enough. This version makes `heat_time`, `evidence_quality`,
`provisional_category`, `authority_pressure`, `clinic_exhaustion`, and trust change options, prose,
visual modifiers, and outcome texture.

