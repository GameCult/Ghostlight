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
`ghostlight-world-ontology.md` under "Elaboration lenses and focus (target)":
consumer-supplied elaboration lenses with weights, focus-weighted elaboration
demand with a consumer-supplied detail profile, per-world evidence binding,
and elaborator sessions in Draft. Other consumers (Delvehold, Epiphany, future
narrative simulations) use the same capabilities with their own policy.

Source today does not match this boundary: the world kernel lives in
`crates/ghostlight-dungeon/src/world/` beside Dungeon's runtime. Plan step 15
names the extraction question.

## Objective

A player chooses a Vault, says what kind of experience they want, and gets a
world seeded from that Vault. The world is dense where they stand and sketched
where they don't, and it grows detail ahead of them as they travel. The flavor
of the world comes from how often each of the eight titled elaborators works
on it.

## The flow

1. **Choose a Vault.** The player picks from Vaults the Dungeon operator has
   admitted. A Dungeon Vault entry names its root and the scope a player may
   read, for example Kalsa's `Public`. The payload carries a Vault id, never a
   path. Dungeon binds the chosen Vault as the world's evidence source.
2. **Describe the experience.** The player writes what they want. A model
   reads that description and proposes weights for the eight titles. The
   proposal fills the sliders; the player adjusts them. Dungeon submits only
   the numbers and the description, which becomes the world brief.
3. **Create.** Dungeon creates a library world with title, brief, lens set and
   weights, and Dungeon's detail profile. No player subject is declared yet.
4. **Sketch.** Dungeon's seed session authors a coarse world from the Vault:
   regions, routes with travel costs, institutions, populations.
5. **Choose a start.** The player picks a sketch place. Dungeon submits one
   owner patch declaring the player's human-controlled subject there, and sets
   that subject as a focus of the world.
6. **Elaborate.** The library's elaborator sessions run, pulled toward the
   focus by distance.
7. **Activate** when the player chooses. Dungeon never makes activation wait
   on a count.
8. **Play.** As the player travels, the library's derived demand follows the
   focus and detail follows the player.

## Dungeon's lens set

Dungeon uses the eight titles under their original names and meanings as its
lens set:

| Title | Reaches for |
|---|---|
| Patina | ordinary objects, customs, nicknames, local jokes |
| Charter | government, law, offices, selection, succession, redress |
| Ledger | resources, labor, trade, infrastructure, scarcity, class pressure |
| Hearth | kinship, daily life, care, obligation, belonging |
| Tangle | factions, alliances, rivalries, leverage |
| Veil | secrets, rumors, misinformation, taboos, unevenly held knowledge |
| Ember | disputes, hazards, instability, escalation, urgent pressure |
| Numen | religion, magic, ritual, awe, cosmology, the genuinely strange |

Each title is a library lens: brief text plus an emphasis over the operation
catalog. Ledger leads with custody and dependency operations, Veil with
knowledge and channels, Charter with authority and selection. Whether the
eight ship with the library as a stock set or live in Dungeon is an open
decision below.

## Dungeon policy

- **Focus:** the subjects controlled by Dungeon's players.
- **Detail profile:** near 60 minutes (persons, institutions, populations),
  middle 1 day (institutions and populations), far (institutions), lookahead
  120 minutes. These are recommendations, tuned on the road.
- **Weights after creation:** the world owner may change them in Draft and
  Active; later sessions use the new weights.
- **Travel:** Dungeon's seed authors a travel affordance for persons, and the
  Eve surface projects one control per affordance a player's subject holds.

## Authority map (Dungeon)

- **Owner:** Dungeon's runtime owns Session Zero's flow, the Vault registry,
  the weight proposal, the player principal, and the Eve surface. Every world
  change it makes is an ordinary library command.
- **Inputs:** the authenticated player, the admitted Vault list, the player's
  description and slider values, the chosen start place.
- **Outputs:** library world creation, seed and owner patches, focus changes,
  weight changes, activation, and Eve projections.
- **Derived state:** the slider proposal, the Session Zero step a player is on
  (derived from the library snapshot's phase, focus, and structure), and every
  projection.
- **Forbidden writers:** Dungeon writing world state other than through
  library commands; the proposal model submitting weights without the
  player's command; a Vault path in any payload; any Dungeon identifier,
  schema, or policy entering the library vocabulary.

## Cut line (Dungeon)

- `GHOSTLIGHT_SEED_VAULT_ROOT` as the only Vault: Dungeon's Vault registry
  replaces it.
- The `world.create` payload fields for jurisdiction roots and permille, when
  the library's detail profile replaces them.
- The genesis human subject: Dungeon declares its player at the chosen start.

## Verification (Dungeon)

1. A player-chosen Vault id resolves only to an admitted Vault and scope; an
   unknown id or a path is refused before anything is spent.
2. The slider proposal changes nothing until the player submits weights.
3. The start choice declares exactly one player subject, at a sketch place,
   once, in Draft, and sets it as focus.
4. Activation succeeds with nonzero deficits.
5. Road: a seeded world on the SDK route shows persons near the start,
   institutions far from it, and new persons at a place after the player's
   subject travels toward it.

## Decisions the operator owns

- **Where the eight titles live.** Recommend: the library ships them as a
  stock lens set any consumer may use or replace, because they are
  world-agnostic narrative categories; Dungeon selects them and supplies
  weights.
- **Library extraction.** Whether the world kernel moves out of
  `crates/ghostlight-dungeon` into its own library crate before the library
  passes land, so new library capabilities are not built inside a consumer.
  Recommend: extract first.

## Pass order

Library passes are in the ontology doc's target section and plan step 15.
Dungeon passes follow them:

1. Vault registry, world creation with Vault id, brief, lens set, weights, and
   detail profile.
2. The Session Zero Eve surface: Vault choice, description, slider proposal,
   sliders, sketch, start choice, activation.
3. Travel affordance and per-affordance player controls. This pass closes the
   playtest gate.
