# Ghostlight stock lenses (plan step 15, L1)

## Status

Closed 2026-09-16. Landed in Ghostlight `835ea4d..872ba35`, plus the Cut 5
doc commits `39b2af4`, `04fd004` and `b15ade4`. The means, every ruling
(L1-Q1 to L1-Q10), and every Soul verdict are in
`ghostlight-stock-lenses-cut.md`; the scars are in
`ghostlight-stock-lenses-postmortem.md`. Windows only: the Linux release is
Idunn's build and is unexercised.

## Target

Elaboration in the `ghostlight` library gets a flavor the world's creator
controls. The library ships eight stock lenses. A world carries lens weights
as world data. Every elaborator session draws one lens by those weights,
replayably, and the drawn lens shapes what kind of structure the session
reaches for. Several sessions elaborate at once.

The lenses are the library's, not Dungeon's. Dungeon supplies weights at
world creation.

| Lens | Reaches for | Leads with |
|---|---|---|
| Patina | ordinary objects, customs, nicknames, local jokes | persona material, ordinary affordances |
| Charter | government, law, offices, selection, succession, redress | authority, selection, redress |
| Ledger | resources, labor, trade, infrastructure, scarcity, class pressure | custody, dependency, route cost |
| Hearth | kinship, daily life, care, obligation, belonging | commitments, reads |
| Tangle | factions, alliances, rivalries, leverage | relations, pressure, authority |
| Veil | secrets, rumors, misinformation, taboos, unevenly held knowledge | knowledge scope, channels |
| Ember | disputes, hazards, instability, escalation, urgent pressure | pressure, near-due commitments |
| Numen | religion, magic, ritual, awe, cosmology, the genuinely strange | facts, affordances, material |

The design text is `ghostlight-world-ontology.md`, "Elaboration lenses and
detail rules", which separates what L1 implemented from what L2 to L4 owe.

## Invariants, reconciled with the Body

1. **A lens never decides admission.** The same patch admits identically under
   all eight lenses, with the mismatch set equal to the pre-L1 capture. A lens
   is not part of the command id, the caller, the confinement ground, or any
   mismatch. Holds.
2. **Every lens sees the whole catalog.** The catalog signatures and digest
   are unchanged from before L1; emphasis is prompt text only (L1-Q5).
   Holds.
3. **The draw is replayable.** The lens is drawn from world id and command
   id with the kernel's `select_band` recipe, and the library has no RNG
   dependency. The operator cares about replayability, not about what seeds
   the draw (L1-Q1). Holds.
4. **Weights are world data with honest admission.** `WorldState` carries
   them (`consumer.v5`); creation requires explicit weights with no library
   default (L1-Q6); the owner replaces them with `SetLensWeights` in any phase
   (L1-Q3). An all-zero set is refused at creation, at the command, at the
   Dungeon payload and in a stored state row. A set that draws the same as the
   current one, however it is spelled, is refused as no canonical change.
   Holds.
5. **Concurrent sessions are distinct work.** The sweep hands concurrently
   running sessions distinct demand entries through an in-memory claim set.
   The lens stays out of the command id, which remains the idempotency key
   (L1-Q4). Elaboration draws from its own pool, a ceiling
   (`GHOSTLIGHT_ELABORATION_MAX_CONCURRENT`, default 2), separate from the
   simulation budget, and saturating either pool never delays the other.
   Dungeon runs no Active elaboration since the play agent's Cut 1
   (`d69e9d4`) deleted its sweep and pool; the claim stands as a library
   capability. Holds.
6. **The lens is recorded and resume reads state.** The drawn lens and its
   instruction text are persisted on the session (`controller_work.v16`,
   L1-Q7). A resumed session keeps both even if the weights or the lens's
   text change. In-flight sessions are rediscovered from the store by what
   they answer (L1-Q9); rows ready to submit are excluded from that listing
   (L1-Q10). Holds.
7. **Stores refuse, not migrate.** `consumer.v4` states and
   `controller_work.v15` rows are refused at open. Holds.
8. **No Dungeon concept enters the library**, and the boundary stays sealed
   by admission. Holds.

## Out of scope, still owed elsewhere

- Detail rules, distance-weighted demand, and the permille cut (L3).
- Per-world evidence binding; the Active elaborator still uses
  `NullEvidenceSource` (L2).
- Elaborator sessions in Draft (L3).
- Dungeon's Session Zero surface, slider proposal, and weight UI (D1-D3).
  Dungeon passes explicit uniform weights through `world_create.v4` today.
