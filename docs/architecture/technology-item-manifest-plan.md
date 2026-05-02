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

## Manifest Object Types

Use these object layers when producing tech data:

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

The next concrete schema after coordinator artifacts should be a manifest seam:

- `schemas/item-manifest.schema.json`
- `examples/item-manifest/pre-elysium-starting-tech.template.json`
- `examples/item-manifest/future-branch-innovation.template.json`
- `tools/validate_item_manifests.py`

Minimum fields:

- manifest id
- fixture lane
- branch lineage, if any
- source refs
- item family or technology id
- object layer
- parent assembly refs
- child component refs
- faction/manufacturer access
- prerequisites
- bottlenecks
- compatibility rules
- gameplay effects
- economic consequences
- quest hooks
- review status

## Data Generation Policy

During worldbuilding exploration:

1. If a scene implies a new item, component, supply bottleneck, or technology
   dependency, emit a manifest candidate.
2. If a faction decision changes what can be manufactured, maintained, bought,
   salvaged, counterfeited, or upgraded, emit a faction tech-base delta.
3. If an innovation appears in Elysium, label its branch lineage and prior
   conditions.
4. If the item relies on unsupported lore, patch AetheriaLore or mark a source
   gap before treating it as stable.
5. If an item is cool but breaks logistics, keep it as a rejected artifact with
   the failure label. Beautiful nonsense is still training data if tagged before
   it bites someone.

Ghostlight should discover the future by exploring it, but every discovery
should leave inventory. The game cannot build a supply chain out of vibes and
one haunted laser.
