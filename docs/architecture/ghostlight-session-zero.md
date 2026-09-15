# Ghostlight Session Zero

Status: adopted direction, 2026-09-15. Not implemented. Plan step 15 in
`notes/ghostlight-implementation-plan.md` carries the pass order.

## Objective

A player chooses a Vault, says what kind of experience they want, and gets a
world seeded from that Vault. The world is dense where they stand and sketched
where they don't, and it grows detail ahead of them as they travel. The flavor
of the world comes from how often each of eight titled elaborators works on it.

Session Zero is the player-facing name for a world's Draft phase. It is not a
separate owner: `WorldKernel` owns Draft and Active alike
(`ghostlight-dungeon-mvp.md`), and every step below is an ordinary command or
an elaborator patch through the one reducer.

## The flow

1. **Choose a Vault.** The player picks from Vaults the operator has admitted.
   A Vault entry names its root and the scope a player may read, for example
   Kalsa's `Public`. The payload carries a Vault id, never a path.
2. **Describe the experience.** The player writes what they want. A model
   reads that description and proposes eight title weights. The proposal fills
   the sliders; the player adjusts them. The kernel receives only the numbers
   and the description, which becomes the world brief.
3. **Create.** `world.create` stores title, brief, Vault id, title weights, and
   the detail profile (below). The player's subject is not declared yet.
4. **Sketch.** The seed lane authors a coarse world from the Vault: regions,
   routes with travel costs, institutions, populations. It places no detail
   ring because no start exists yet.
5. **Choose a start.** The player picks a place from the sketch. The kernel
   declares the player's subject there, controlled by that player.
6. **Elaborate.** The swarm runs, pulled toward the start by distance.
7. **Activate** when the player chooses. Activation never waits on a count.
8. **Play.** The swarm keeps running in Active. As the player travels, the
   demand around them moves and detail follows.

## Titles are lenses

The eight titles return under their original names and meanings:

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

A title is a lens: brief text plus an emphasis over the operation catalog
(Ledger leads with custody and dependency operations, Veil with knowledge and
channels, Charter with authority and selection). Every title receives the whole
derived tool catalog. A title cannot refuse, verify, or rank another title's
work, and no title has a quota.

The titles owned no depth before the ontology existed; the typed components
give each a real target. Patina writes `PersonaMaterial` and ordinary
affordances. Ledger writes custody, dependencies, and route costs. Hearth
writes commitments and reads. Tangle writes relations, pressure, and authority.
Veil writes knowledge scoping and channels. Ember writes pressure and
commitments with near dues. Charter writes authority, selection, and redress.
Numen writes facts, affordances, and material with the strange in it.

## The swarm

Up to `GHOSTLIGHT_CONTROLLER_MAX_CONCURRENT` elaborator sessions run at once.
Each session:

1. takes the head of the derived elaboration demand (below);
2. draws a title by the world's weights from a sampler seeded by world id and
   session ordinal, recorded on the session;
3. retrieves evidence from the world's Vault for the demand's referents;
4. builds one patch under that title's lens, confined to the demand's ground;
5. submits; a refusal continues the same conversation, as today.

Concurrency conflicts resolve as they do now: patches bind to scope digests,
the mailbox serializes commits, and an overlapping patch repairs.

## Distance-weighted demand

The detail target depends on travel distance from the player, not on a fixed
share per region.

- **Distance** is the least route cost, in travel minutes, from any
  human-controlled subject's position to a place, over open routes that
  subject could traverse. It uses the same route walk as `reachable` in
  `action.rs`, factored to return a cost map. A place without routes takes the
  distance of its nearest routed container. Before a start exists, every place
  is far.
- **Detail profile**, stored at creation, is a small set of bands by distance:

  | Band | Distance | Target |
  |---|---|---|
  | Near | within `near` minutes | persons, institutions, populations |
  | Middle | within `middle` minutes | institutions and populations |
  | Far | beyond | institutions only |

  Each band names a density per place. The profile replaces
  `WorldScaleIntent` jurisdiction permille; overall targets per kind remain.
- **Demand** is derived in `snapshot`, never stored: open causal boundaries
  and per-place deficits against the band target, ordered by distance and then
  by the kernel's existing boundary order. A place approaching the player
  raises its band, which raises its deficit, which puts it at the head.
- **Lookahead**: the near band is measured from where the player could be
  after `lookahead` minutes of travel, so detail arrives before the player.
- **Individuation**: when a population's place enters the near band,
  `IndividuationRequired` derives for it, and a session materializes named
  members from it. A city sketched as a handful of factions gets its people as
  the player approaches.

## Authority map

- **Owner:** `WorldKernel`. It admits every patch, derives distance, demand,
  and deficits in `snapshot`, and declares the player's subject at the chosen
  start.
- **Inputs:** Vault id, brief, title weights, and detail profile from
  `world.create`; the start place from `world.choose_start`; canonical
  positions, routes, and costs; elaborator patches.
- **Outputs:** commits, receipts, and the derived demand and deficit rows on
  the snapshot.
- **Derived state:** distance maps, demand order, deficits, title draws, and
  the model's slider proposal. The title recorded on a session and receipt is
  evaluation evidence.
- **Forbidden writers:** a title as an authority, admission input, or quota; a
  per-title verifier; the slider-proposal model writing weights without the
  player's command; demand or distance stored as world state; an elaborator
  writing outside its demand's ground; a count gating activation; a Vault path
  in any payload.
- **Shared paths:** Draft and Active run the same swarm, the same demand
  derivation, and the same patch admission. The seed lane's sketch is one
  owner patch through the same reducer. Player travel, NPC travel, and time all
  move demand only through committed positions.
- **Confinement ground:** a session's ground is the place subtree of the
  demand it answers: the boundary's place, the deficit row's place, or the
  population's place. `SystemCapability::Elaborator { jurisdiction }` carries
  that ground. Authored jurisdiction roots are no longer the unit of
  confinement.

## Cut line

Deleted or replaced before the new behavior is called complete:

- `WorldScaleIntent` jurisdiction permille and authored jurisdiction roots as
  confinement units; per-kind targets survive inside the detail profile;
- `JurisdictionKey::Uncovered` as a deficit row, since every place has a
  distance;
- the sequential one-session-per-jurisdiction sweep in `elaboration.rs`;
- `NullEvidenceSource` on the Active elaborator lane: Active elaboration reads
  the world's Vault;
- `GHOSTLIGHT_SEED_VAULT_ROOT` as the only Vault: an operator Vault registry
  replaces it;
- genesis declaration of the human subject in The Commons, and The Commons if
  nothing else needs it;
- "Draft answers nothing" for the elaborator lane: Draft sessions answer
  derived demand once a start exists.

## Subtraction budget

Net additive by: the title enum and lens text, a weight vector and detail
profile on world creation, a seeded sampler, a cost-map route walk shared with
`reachable`, demand derivation, one start command, and a Vault registry. Cut
against it: permille distribution, authored jurisdiction roots, the
`Uncovered` row, the per-jurisdiction sweep, the single-root Vault variable,
genesis human placement, and the null evidence source. No verifier, quota,
scheduler service, or stored demand is added.

## Build budget

Proving host: the Windows workstation, `ghostlight-dungeon` crate tests, plus
sidecar schema fixtures if tool descriptions change. The Linux release is
Idunn's build and is not touched by these passes. Road proof uses the local
live smoke with the SDK sidecar.

## Verification

1. The same patch submitted under two titles produces the same admission
   result; a title never appears in a mismatch.
2. The sampler, seeded identically, draws the same title sequence, and its
   frequencies follow the weights over a fixed draw count.
3. A zero weight never draws; all-zero weights are refused at creation.
4. Moving a human subject changes demand order without a commit; demand is
   absent from `WorldState`.
5. A place entering the near band moves to the head of demand; a place leaving
   it keeps its admitted structure.
6. An elaborator patch outside its demand's ground is refused.
7. An Active elaboration patch cites Vault receipts from the world's Vault
   scope and never from outside it.
8. Activation succeeds with nonzero deficits.
9. `world.choose_start` declares exactly one human-controlled subject at a
   sketch place, once, in Draft.
10. Road: a seeded world on the SDK route shows persons near the start,
    institutions far from it, and new persons at a place after the player's
    subject travels toward it.

## Decisions the operator owns

- **Weight changes after creation.** Recommend: the owner may change weights
  in Draft and Active through one command; later sessions use the new weights
  and admitted structure stays.
- **Title emphasis versus a title-restricted catalog.** Recommend emphasis with
  the whole catalog, measured by per-title operation counts on the road. A
  restricted catalog would make the title decide what may be added.
- **Band values.** Recommend near 60 minutes, middle 1 day, lookahead 120
  minutes, tuned on the road.
- **Multiple players.** Distance is already defined over every human subject;
  multi-player membership is a separate cut.

## Pass order

1. Titles as lenses and the weighted swarm over today's demand.
2. Vault registry, `world.create` with Vault id, brief, and weights, and Vault
   evidence on the Active lane.
3. Detail profile, distance-weighted demand, `world.choose_start`, and the
   permille and root cuts.
4. Individuation in the near band.
5. The Session Zero Eve surface: Vault choice, description, slider proposal,
   sliders, sketch, start choice, activate; plus a travel affordance and
   per-affordance player controls. This pass closes the playtest gate.
