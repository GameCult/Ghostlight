# Technology And Item Manifest Plan

Ghostlight future exploration should produce more than scenes. It should produce
the technical and economic knowledge Aetheria needs for gameplay: what items can
exist, who can build them, what they are made of, which assemblies can be
swapped, what supply chains they depend on, and which factions start with which
tech bases.

This is not separate from story generation. Major-player decisions, faction
births, branch pressures, and technological discoveries shape the economy. If a
branch discovers a better aetheric emitter, that should eventually change ship
builds, market dependencies, quest hooks, factory politics, salvage value, and
what a desperate mechanic can realistically bolt into a hull without creating a
beautiful obituary.

## Source Premise

Aetheria game design wants ships to be tools, homes, status symbols, and
material compromises. Customization is driven by components with distinct
behavior, manufacturers with priorities and specialties, and vessels tuned for
violence, trade, survival, logistics, or prestige.

Ghostlight should turn worldbuilding exploration into the data needed to support
that design.

## Player Fantasy

The deep gear model is not a loot-affix treadmill. The fantasy is provenance.

A player hunting for the ultimate laser should be able to trace the item through
its parts, makers, technologies, and accidents:

- the emitter assembly came from a short-lived manufacturer whose process was
  only stable for nine months
- the beam-control wafer uses a regulatory-buried fabrication step no successor
  company wants to admit worked
- the thermal rejection assembly is sturdy but mediocre, quietly strangling the
  whole weapon
- a pirate rebuild replaced the targeting interface with a counterfeit part
  that improves lock speed while making maintenance ugly
- a factional embargo means the good driver wafers are technically illegal,
  practically everywhere, and therefore excellent quest bait

An "ultimate item" is not just the best blueprint. It is the best realized
artifact: the right blueprint, built from the right assemblies, with the right
quality, provenance, manufacturer practice, replacement history, and operating
conditions. If a player wants to rebuild it as the best it can be, the game
should let them hunt the supply chain: salvage yards, old corporate stockpiles,
specialist shops, faction arsenals, black-market refits, inherited ships,
forgotten factories, and small wars over a beam-control wafer some dead
manufacturer only made for nine months before the board got eaten by regulators.

This is why Ghostlight matters. A spreadsheet can name a laser. Ghostlight should
help explain why that laser exists, who can build it, which parts are worth
killing for, why a particular instance is special, and what stories fall out of
trying to make it better.

## Content Refinery

The old bottleneck was not ideas. Ideas are cheap. Volunteer design goblins can
produce a hill of them before lunch and leave the database work sitting there
like a little corpse.

The real bottleneck is converting taste, lore, mechanics, and future-tech
imagination into loadable, reviewable, schema-shaped content. Ghostlight's
technology pipeline should act as a content refinery:

1. Ground the request in Aetheria lore: faction, era, location, tech base,
   industrial limits, political pressure, and known source constraints.
2. Decompose the concept into item families, assemblies, subassemblies,
   components, materials, processes, tooling, facilities, expertise, and
   supply-chain dependencies.
3. Map each candidate onto the Aetheria-Economy blueprint model:
   `SimpleCommodityData`, `CompoundCommodityData`, `ConsumableItemData`,
   `GearData`, or another `EquippableItemData` subclass.
4. Define performance intent through behaviors, `PerformanceStat` fields,
   quality sensitivity, failure modes, upgrade surfaces, and operating
   constraints.
5. Define realized-instance variation: assembly provenance, manufacturer
   branding, quality, repair history, counterfeit substitutions, salvage scars,
   and ownership chain.
6. Emit reviewable artifacts that separate source facts, inferred engineering,
   branch-local assumptions, engine-schema gaps, rejected ideas, and accepted
   candidates.
7. Export or hand off accepted candidates as database-shaped content rather than
   another swamp of clever prose someone has to manually translate later.

The goal is not infinite content sludge. The goal is volume with preserved taste:
many plausible blueprints, variants, manufacturers, counterfeit versions,
salvage stories, and supply-chain hooks that still feel like Aetheria.

## Economy Schema Target

Ghostlight should target the existing Aetheria-Economy item model instead of
inventing a parallel item taxonomy.

The concrete source is:

- `E:\Projects\Aetheria-Economy\Assets\Scripts\ServerShared\ItemData.cs`
- `E:\Projects\Aetheria-Economy\Assets\Scripts\ServerShared\ItemInstance.cs`
- `E:\Projects\Aetheria-Economy\Assets\Scripts\ServerShared\CultCache\`

The important split:

- `ItemData` and subclasses are CultCache-backed technological blueprint
  records: a way of constructing a thing in the game, including the parameters,
  behaviors, stat curves, and category fields that make that construction
  meaningful.
- `ItemInstance` and subclasses are runtime objects that link to definition
  records and represent manufactured artifacts in the world.
- Ghostlight item manifests should normally propose blueprint/data records, not
  individual runtime artifacts.
- Runtime instances belong in simulation, inventory, scene state, cargo, loot,
  and world-state fixtures. They are where realized quality, supply-chain
  provenance, manufacturer branding, and assembly history can make two items
  built from the same blueprint differ in practice.

Blueprint classes currently relevant to Ghostlight:

- `SimpleCommodityData`
  - basic fungible resources such as steel, hydrogen, volatiles, or feedstock
  - maps to `SimpleCommodity` runtime instances with quantity
- `CompoundCommodityData`
  - technological blueprints for crafted commodities, assemblies,
    subassemblies, processed materials, and other manufactured economic units
  - maps to `CompoundCommodity` runtime instances whose quality and provenance
    can affect downstream items
- `ConsumableItemData`
  - usable consumables with behaviors, duration, stackability, icon, and
    effectiveness curve
  - maps to `ConsumableItem` runtime instances
- `GearData`
  - technological blueprints for usable equippable items with hardpoint,
    behavior, durability, thermal, audio, schematic, and action-bar fields
  - maps through `EquippableItem` runtime instances whose realized quality,
    provenance, manufacturer, and installed assembly lineage can alter actual
    performance
- `WeaponItemData`, `HullData`, `CargoBayData`, `DockingBayData`, and other
  `EquippableItemData` subclasses are specialized top-level usable item
  definitions.

Manifest artifacts should therefore use Ghostlight's conceptual decomposition to
produce reviewed candidate technological blueprints for Aetheria-Economy data
classes:

```text
material/feedstock            -> SimpleCommodityData
component/subassembly/assembly -> CompoundCommodityData
consumable usable item         -> ConsumableItemData
equippable usable item         -> GearData or another EquippableItemData subclass
weapon                         -> WeaponItemData
hull/frame/cargo/docking item  -> HullData, CargoBayData, DockingBayData, etc.
runtime cargo or inventory     -> SimpleCommodity, CompoundCommodity,
                                  ConsumableItem, or EquippableItem instances
```

The conceptual layers below are still useful. They describe how Ghostlight
breaks a technology into reviewable engineering, logistics, and gameplay pieces.
They are not a replacement database schema. The output should be able to say
which Aetheria-Economy blueprint class each candidate wants to become, what
fields are source-backed, what fields are inferred, and which fields require
new engine schema before they can be loaded.

`PerformanceStat` is the core bridge from blueprint to realized item behavior.
Blueprint data defines stat ranges and quality sensitivity; runtime instances
and equipped/active contexts determine actual evaluation through quality,
durability, heat, effectiveness, and modifiers. Ghostlight manifests should
therefore preserve the difference between:

- blueprint-level performance intent
- assembly or subassembly contribution to quality and stat behavior
- realized instance provenance and manufacturer branding
- in-world operating conditions that change evaluated performance

If a Ghostlight manifest needs fields Aetheria-Economy does not currently store,
mark them as manifest-side metadata or source gaps. Do not pretend CultCache
already has slots for every nice idea. The future can have organs; today it has
classes.

## Nebulous Technology Elaboration

Aetheria already has plenty of named technologies. Many of them are currently
high-level lore objects rather than actionable game objects. That is fine for a
timeline note. It is not enough for an item blueprint database.

Ghostlight's exploration pipeline should elaborate existing vague technologies
into blueprint candidates.

For each nebulous technology, produce:

- source summary
- inferred operating principle
- gameplay role
- item families enabled
- assemblies and subassemblies implied
- critical components
- materials
- manufacturing processes
- tooling and facility requirements
- required expertise
- quality tiers
- failure modes
- maintenance needs
- salvage value
- counterfeit risk
- substitute parts
- faction access and restrictions
- supply-chain bottlenecks
- prerequisite technologies
- downstream technologies enabled
- branch-local variants
- open lore questions

This is where a phrase like `super-Planckian emitter` stops being a decorative
sci-fi noun and becomes a logistics problem with teeth.

If Ghostlight cannot yet know the exact real physics, it should still propose a
consistent fictional engineering stack: what must be fabricated, stabilized,
aligned, cooled, calibrated, certified, transported, repaired, and monopolized.
The output should be reviewable, revisable, and eventually loadable into a game
database.

Do not hallucinate certainty. Mark inferred engineering as inferred. Mark source
facts as source facts. Mark branch-local innovations as branch-local. The
manifest is allowed to deepen the world; it is not allowed to pretend an
invention arrived without provenance.

## Item Knowledge Goals

Every explored technology or item family should be able to answer:

- what the item does
- what gameplay role it serves
- what systems it plugs into
- what components and subassemblies it requires
- which assemblies are swappable upgrade surfaces
- which materials, processes, or specialists are bottlenecks
- which factions or manufacturers can build it
- which factions can maintain it but not manufacture it
- which factions can counterfeit, salvage, or adapt it
- which tech prerequisites unlock it
- which branch conditions or discoveries created it
- which quests, markets, or conflicts it enables
- which blueprint records it should create or update

## Manifest Object Types

Use these object layers when producing tech data. Each layer must also declare
its intended Aetheria-Economy target class or state that it is manifest metadata
only:

- `item_family`
- `item_variant`
- `assembly`
- `subassembly`
- `component`
- `material`
- `process`
- `tooling`
- `expertise`
- `facility`
- `manufacturer`
- `supply_node`
- `upgrade_slot`
- `blueprint`
- `tech_prerequisite`
- `quality_tier`
- `failure_mode`

An `item_family` is a broad class, such as laser weapon, radiator, aetheric
drive, cryo valve, sensor mast, or habitat seal.

An `item_variant` is a concrete build with stats, faction/manufacturer flavor,
quality tier, and branch-specific assumptions.

An `assembly` is a replaceable functional unit. For a laser, emitter assembly,
power conditioning assembly, thermal rejection assembly, beam-control assembly,
targeting interface, and housing are likely assemblies.

A `subassembly` is a nested unit inside an assembly. The emitter assembly may
include gain medium, field lattice, focusing microstructure, alignment actuator,
control wafer, and contamination seal.

A `component` is the smallest useful gameplay/logistics unit worth tracking.
Do not split screws unless screws are the point of the quest. The machine has
standards. Barely.

A `blueprint` is the technological construction record represented by the
appropriate Aetheria-Economy `*Data` class. It should be specific enough for a
game system to evaluate whether a faction, factory, ship, or player can build
the item, and how sub-assemblies affect performance through quality-sensitive
stats and behavior parameters.

Ghostlight manifests still need explicit assembly trees, compatibility rules,
supply-chain dependencies, and manufacturing gates. If the current engine schema
does not expose those fields directly, keep them as manifest metadata or
engine-schema gaps attached to the target `*Data` blueprint rather than
pretending they are separate from the blueprint concept.

A `tech_prerequisite` is a required discovery, process, standard, license,
facility, or expertise gate.

A `quality_tier` describes meaningful gameplay variation: crude, standard,
premium, military, experimental, branch-exotic, counterfeit, degraded, salvaged,
or other reviewed tiers.

A `failure_mode` describes what breaks, how it breaks, and what that failure
costs in gameplay, safety, logistics, reputation, or maintenance.

## Example Pattern: Laser Upgrade Surface

A laser design should be represented so a higher-tech emitter assembly can alter
performance without rewriting the whole item.

Example structure:

```text
item_family: laser_weapon
  item_variant: sol_pattern_industrial_laser_mk2
    assembly: emitter_assembly
      subassembly: gain_medium
      subassembly: focusing_lattice
      subassembly: alignment_actuator
    assembly: power_conditioning_assembly
    assembly: thermal_rejection_assembly
    assembly: beam_control_assembly
    assembly: targeting_interface
    assembly: housing_and_mount
```

A branch-local future might discover an aether-stabilized emitter assembly. That
should produce:

- a new assembly or subassembly record
- prerequisite discoveries
- materials and tooling
- compatibility rules with older laser families
- performance changes
- heat, stability, cost, legality, maintenance, or supply-chain consequences
- factions able to manufacture or adapt it
- quests or market events unlocked by scarcity

## Example Pattern: Super-Planckian Emitter

When a source names a technology like `super-Planckian emitter`, Ghostlight
should elaborate it into at least a candidate blueprint stack.

Example working decomposition:

```text
item_family: exotic_emitter
  item_variant: super_planckian_emitter_core_candidate
    assembly: emitter_lattice_assembly
      subassembly: boundary_condition_lattice
      subassembly: phase-lock microstructure
      subassembly: exotic containment frame
    assembly: field_drive_assembly
      subassembly: pulse shaper
      subassembly: high-stability driver wafer
      subassembly: feedback quench circuit
    assembly: thermal_and_causality_safety_assembly
      subassembly: transient heat sink
      subassembly: event-spike monitor
      subassembly: failsafe isolation gate
    assembly: calibration_and_control_assembly
      subassembly: metrology package
      subassembly: firmware governor
      subassembly: certification seal
```

That decomposition is not automatically canon. It is a candidate manifest. A
reviewer can accept, reject, revise, or mark source gaps. The important point is
that the technology now has places for supply chains to attach:

- lattice fabrication
- exotic containment materials
- high-stability driver wafers
- metrology packages
- safety gates
- certification regimes
- specialist calibration expertise
- heat rejection and failure containment

Once those exist, the game can ask useful questions:

- who owns lattice fabrication?
- who can counterfeit the driver wafer?
- what happens if the safety gate is downgraded?
- can a higher-tier emitter assembly fit into an older laser housing?
- which faction can maintain it after three jumps and one bad decision?
- what quest appears when the certification seal supply collapses?

## Pre-Elysium Starting Tech

Pre-Elysium tech is not a monolith. Everyone was shunted into Elysium together,
but not every faction brought the same industrial base, doctrine, standards,
stockpiles, expertise, or manufacturing rights.

Ghostlight should generate starting tech manifests for the single-history Sol
artifact:

- base technologies available before the Rupture
- faction access and restrictions
- manufacturing standards
- dominant manufacturers
- known substitute parts
- logistics bottlenecks
- maintenance dependencies
- doctrine-driven design differences
- legal restrictions and export controls
- black-market or pirate adaptations
- blueprint records for starting equipment, ship systems, weapons, habitats,
  medical systems, industrial tooling, and logistics infrastructure

This matters because Elysium branches inherit different practical capabilities
from the same shared Sol history depending on what each faction controls in that
branch.

## Post-Rupture Innovation

Post-Rupture innovation is branch-local canon indexed by lineage.

When exploration discovers a technology, the manifest should record:

- branch id and lineage
- prior conditions
- triggering event or pressure
- involved factions, inventors, institutions, or nonhuman actors
- source-backed post-Elysium concepts used
- generated branch assumptions
- prerequisite tech and discoveries
- item families affected
- assemblies or subassemblies created
- supply chain changes
- market/faction consequences
- quest hooks opened
- sibling branches where this innovation does not exist or differs
- blueprint records created or modified
- compatibility with pre-Elysium assemblies

Aetheric drives, necrotech interfaces, spirit-mediated sensors, temporal memory
archives, pseudospace logistics, and mutable-body support equipment should all
leave this kind of trail when explored.

## Faction Tech Bases

For each faction or manufacturer, track:

- known technologies
- restricted technologies
- lost or inaccessible technologies
- signature design preferences
- manufacturing strengths
- maintenance weaknesses
- doctrine-driven compromises
- supply dependencies
- export controls
- counterfeiting exposure
- salvage behavior
- branch-local innovations

A faction can understand a technology without being able to build it. It can
install a component without being able to maintain it. It can maintain an item
only by depending on a hated supplier. These distinctions are the gameplay.

## Coordinator Obligations

The coordinator/story runtime must emit technology and economy knowledge when a
scene or branch decision changes the material world.

Coordinator artifacts should include item-manifest deltas when they discover or
alter:

- technology availability
- component compatibility
- assembly upgrade paths
- materials and manufacturing processes
- supply bottlenecks
- faction access
- market pressure
- quest hooks
- salvage opportunities
- maintenance constraints

The coordinator is allowed to propose item knowledge. Deterministic validators
and reviewer gates decide whether it becomes a manifest update.

## Training Artifacts

Tech/item exploration should emit reviewed artifacts for future models:

- item manifest records
- component breakdown records
- assembly compatibility records
- faction tech-base records
- supply-chain dependency records
- technology discovery records
- branch-local innovation records
- economic consequence records
- rejected overpowered or unsupported item designs
- source-gap records requiring AetheriaLore elaboration

These artifacts train:

- coordinator/story runtime
- institution/faction decision models
- consumer and market behavior models
- item-generation models
- lore/source evaluators
- retrieval models for tech prerequisites and dependencies

## First Schema Target

The next concrete schema after coordinator artifacts should be a manifest seam
that maps directly onto Aetheria-Economy's technological blueprint classes:

- `schemas/item-manifest.schema.json`
- `schemas/item-blueprint-candidate.schema.json`
- `schemas/item-instance-provenance-candidate.schema.json`
- `examples/item-manifest/pre-elysium-starting-tech.template.json`
- `examples/item-manifest/future-branch-innovation.template.json`
- `examples/item-manifest/super-planckian-emitter.candidate.json`
- `tools/validate_item_manifests.py`

Minimum fields:

- manifest id
- fixture lane
- branch lineage, if any
- source refs
- source-backed facts
- inferred engineering
- open lore gaps
- item family or technology id
- object layer
- target Aetheria-Economy class
- target blueprint id, when updating an existing record
- candidate blueprint fields
- blueprint candidate ids
- instance provenance fields, when simulating manufactured artifacts
- parent assembly refs
- child component refs
- materials/process/tooling/facility refs
- faction/manufacturer access
- prerequisites
- bottlenecks
- compatibility rules
- quality tiers
- failure modes
- gameplay effects
- economic consequences
- quest hooks
- engine-schema gaps
- review status

Candidate blueprint fields should be shaped around existing data classes first:

- shared `ItemData` blueprint fields: `Name`, `Description`, `Manufacturer`,
  `Mass`, `Shape`,
  `SpecificHeat`, `Conductivity`, `Price`
- `SimpleCommodityData`: `MaxStack`, `Category`
- `CompoundCommodityData`: `DemandProfile`, `Category`
- `ConsumableItemData`: `Behaviors`, `Stackable`, `Duration`, `Icon`,
  `Effectiveness`
- `EquippableItemData` and subclasses: `Schematic`, `Behaviors`, `Durability`,
  temperature fields, heat performance curve, resilience, icons, sound data,
  `HardpointType`, and subclass-specific fields such as weapon range, caliber,
  fire type, hull hardpoints, cargo interiors, or docking size

Ghostlight may propose additional manifest fields, but validators should label
them as metadata, engine-gap, or rejected. Do not silently smuggle non-engine
fields into supposed CultCache records.

## Data Generation Policy

During worldbuilding exploration:

1. If a scene implies a new item, component, supply bottleneck, or technology
   dependency, emit a manifest candidate.
2. If a faction decision changes what can be manufactured, maintained, bought,
   salvaged, counterfeited, or upgraded, emit a faction tech-base delta.
3. If an innovation appears in Elysium, label its branch lineage and prior
   conditions.
4. If existing lore names a technology but lacks blueprint detail, elaborate it
   into a candidate manifest with explicit inferred fields and source-gap notes.
5. If the item relies on unsupported lore, patch AetheriaLore or mark a source
   gap before treating it as stable.
6. If an item is cool but breaks logistics, keep it as a rejected artifact with
   the failure label. Beautiful nonsense is still training data if tagged before
   it bites someone.

Ghostlight should discover the future by exploring it, but every discovery
should leave inventory. The game cannot build a supply chain out of vibes and
one haunted laser.
