# Ghostlight Dungeon Session Zero

Status: adopted direction, 2026-09-15. Not implemented. Plan step 15 in
`notes/ghostlight-implementation-plan.md` carries the pass order.

## Ownership

Session Zero is Ghostlight Dungeon's concept and Dungeon's authority. Dungeon
is a consumer of the Ghostlight library. Session Zero consumes library
capabilities through their generic surfaces and adds nothing Dungeon-shaped
to the library: no Vault registry, player, slider, starting location, or
Session Zero phase exists in the library vocabulary.

The library capabilities it needs are specified, consumer-neutral, in
`ghostlight-world-ontology.md` under "Elaboration lenses and detail rules
(target)": the stock lens set and weights, detail rules, per-world evidence
binding, and elaborator sessions in Draft. Other consumers use the same
capabilities with their own policy; Delvehold, for one, starts from a
`Uniform` detail rule.

Source does not yet match this boundary: the world kernel lives in
`crates/ghostlight-dungeon/src/world/` beside Dungeon's runtime. Extracting it
into a library crate is the first pass of plan step 15.

## Objective

A player chooses a Vault, says what kind of experience they want, and gets a
world seeded from that Vault. The world is dense where they stand and sketched
where they don't, and it grows detail ahead of them as they travel. The flavor
of the world comes from how often each of the eight stock lenses works on it.

## The flow

1. **Choose a Vault.** The player picks from Vaults the Dungeon operator has
   admitted. A Dungeon Vault entry names its root and the scope a player may
   read, for example Kalsa's `Public`. The payload carries a Vault id, never a
   path. Dungeon binds the chosen Vault as the world's evidence source.
2. **Describe the experience.** The player writes what they want. A model
   reads that description and proposes weights for the eight stock lenses. The
   proposal fills the sliders; the player adjusts them. Dungeon submits only
   the numbers and the description, which becomes the world brief.
3. **Create.** Dungeon creates a library world with title, brief, the stock
   lens set and weights, and Dungeon's detail rule with no anchor yet. No
   player subject is declared yet, so every place sits at the rule's floor.
4. **Sketch.** Dungeon's seed session authors a coarse world from the Vault:
   regions, routes with travel costs, institutions, populations.
5. **Choose a start.** The player picks a sketch place. Dungeon submits one
   owner patch declaring the player's human-controlled subject there, and pins
   that subject as the first anchor of its detail rule.
6. **Elaborate.** The library's elaborator sessions run, pulled toward the
   anchors by distance.
7. **Activate** when the player chooses. Dungeon never makes activation wait
   on a count.
8. **Play.** As the player travels, the library's derived demand follows the
   anchor and detail follows the player.

## Dungeon policy

- **Lenses:** the library's stock set of eight, weighted by the player's
  sliders. The owner may change weights in Draft and Active; later sessions use
  the new weights.
- **Detail rule:** `ByDistance`, anchored on important subjects, the player
  first. Recommended levels: within 60 minutes, persons, institutions, and
  populations; within 1 day, institutions and populations; beyond, institutions
  only; lookahead 120 minutes. Tuned on the road. Which other subjects Dungeon
  pins, and when, is Dungeon policy not yet decided.
- **Travel:** Dungeon's seed authors a travel affordance for persons, and the
  Eve surface projects one control per affordance a player's subject holds.

## Authority map (Dungeon)

- **Owner:** Dungeon's runtime owns Session Zero's flow, the Vault registry,
  the weight proposal, the player principal, anchor pinning, and the Eve
  surface. Every world change it makes is an ordinary library command.
- **Inputs:** the authenticated player, the admitted Vault list, the player's
  description and slider values, the chosen start place.
- **Outputs:** library world creation, seed and owner patches, rule and
  weight changes, activation, and Eve projections.
- **Derived state:** the slider proposal, the Session Zero step a player is on
  (derived from the library snapshot's phase, rule anchors, and structure), and
  every projection.
- **Forbidden writers:** Dungeon writing world state other than through
  library commands; the proposal model submitting weights without the
  player's command; a Vault path in any payload; any Dungeon identifier,
  schema, or policy entering the library vocabulary.

## Cut line (Dungeon)

- `GHOSTLIGHT_SEED_VAULT_ROOT` as the only Vault: Dungeon's Vault registry
  replaces it.
- The `world.create` payload fields for jurisdiction roots and permille, when
  the library's detail rule replaces them.
- The genesis human subject: Dungeon declares its player at the chosen start.

## Verification (Dungeon)

1. A player-chosen Vault id resolves only to an admitted Vault and scope; an
   unknown id or a path is refused before anything is spent.
2. The slider proposal changes nothing until the player submits weights.
3. The start choice declares exactly one player subject, at a sketch place,
   once, in Draft, and pins it as the rule's first anchor.
4. Activation succeeds with nonzero deficits.
5. Road: a seeded world on the SDK route shows persons near the start,
   institutions far from it, and new persons at a place after the player's
   subject travels toward it.

## Pass order

Library passes L0 through L4 are in plan step 15; L0 is the kernel
extraction. Dungeon passes follow them:

1. Vault registry, world creation with Vault id, brief, lens weights, and
   detail rule.
2. The Session Zero Eve surface: Vault choice, description, slider proposal,
   sliders, sketch, start choice, activation.
3. Travel affordance and per-affordance player controls. This pass closes the
   playtest gate.
