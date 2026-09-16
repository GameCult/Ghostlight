# Ghostlight stock lenses (plan step 15, L1)

## Status

Derived, not maintained: this pass is open until
`ghostlight-stock-lenses-cut.md` names landed commits for every cut and a
Soul verdict covers every Hands promise. Target written 2026-09-16 against
Ghostlight `ef04aa5`.

## Target

Elaboration in the `ghostlight` library gets a flavor the world's creator
controls. The library ships eight stock lenses. A world carries lens weights
as world data. Every elaborator session draws one lens by those weights,
deterministically, and the drawn lens shapes what kind of structure the
session reaches for. Several sessions can elaborate at once.

The lenses are the library's, not Dungeon's. Dungeon supplies weights at
world creation and nothing more in this pass.

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

The design text this target realizes is
`ghostlight-world-ontology.md`, "Elaboration lenses and detail rules
(target)", Lenses and Stock lens set bullets.

## Invariants that must survive

1. **A lens never decides admission.** The same patch submitted under two
   lenses has the same admission result. A lens is not part of the caller's
   identity, confinement ground, or any mismatch. No per-lens verifier and no
   per-lens quota exist.
2. **Every lens sees the whole catalog.** The tool catalog is fixed per
   build; a lens changes what the prompt emphasizes, never which tools exist.
3. **The draw is deterministic and replayable.** Given the same world and the
   same session identity, the same lens is drawn. The library gains no RNG
   dependency; the draw reuses the kernel's digest-based selection recipe.
4. **Weights are world data with honest admission.** A zero weight never
   draws. An all-zero weight set is refused where weights are admitted. A
   weight names only a lens the library knows.
5. **Concurrent sessions are distinct work.** Two sessions running at once are
   distinct commands with distinct checkpoints. Neither collapses into the
   other as "already present", neither admits a patch the other already spent,
   and elaboration respects the runtime's controller permit pool.
6. **The lens is recorded.** The drawn lens is persisted with the session's
   checkpoint and survives resume unchanged. A resumed session never redraws.
7. **Stores refuse, not migrate.** Any schema bump follows the existing rule:
   older rows and states are refused at open, with no migration adapter.
8. **No Dungeon concept enters the library**, and the library/consumer boundary
   stays sealed by admission (ruling Q1-9 of L0).

## Out of scope

- Detail rules, distance-weighted demand, and the permille cut (L3).
- Per-world evidence binding; the Active elaborator keeps `NullEvidenceSource`
  (L2).
- Elaborator sessions in Draft (L3).
- Dungeon's Session Zero surface, slider proposal, and weight UI (D1-D3).
  Dungeon only passes weights, or the library's defaults, at world creation.

## Known constraints from the substrate map

- Session command ids derive from world, jurisdiction and answer digest
  (`crates/ghostlight/src/elaboration.rs:1271-1277`); two concurrent sessions on
  the same answer at the same ancestry would share one id today.
- `ElaboratorSession` is `deny_unknown_fields` inside `controller_work.v15`
  rows (`elaboration.rs:57-66`, `controllers.rs:77-78`), and checkpoint
  progression requires the session to stay equal (`elaboration.rs:205`).
- `ELABORATION_INSTRUCTIONS` is a static string that feeds
  `provider_request_id` (`elaboration.rs:52`, `controllers.rs:6018`);
  `agent_prompt` is persisted and never rebuilt on resume
  (`elaboration.rs:414-435`).
- `PATCH_TOOLS` is fixed per build; counts are pinned at (7, 30, 39) and order
  is not (`patch.rs:6114-6146`, `:6811`, `:6971`).
- `scale_intent` is the only world-level scalar written at genesis, and it is
  write-once by test (`lib.rs:844`, `lib.rs:14330`). No command changes a
  world scalar after creation except time.
- The runtime's elaboration driver runs one sequential sweep every 300 s and
  holds no controller permit (`crates/ghostlight-dungeon/src/runtime.rs:1817-1841`).
- The library has no `rand` dependency; `select_band` over `BandPreimage` is
  "the only entropy in the kernel" (`action.rs:121-127`, `775-800`).

## Questions for Imagination to bring back

These are open until the cut map records a ruling on each.

- What seeds the draw, given that nothing counts sessions: a new ordinal, or
  the session's own identity?
- Where weights live and how they are admitted: a `WorldState` field (a
  `world_state` bump) set at genesis, alongside or instead of `scale_intent`.
- Whether an owner command to change weights after creation belongs in L1,
  given there is no precedent for changing a world scalar after genesis.
- How concurrent sessions stay distinct: the lens and a disambiguator in the
  command identity, sessions claiming different demand entries, or both.
- How "an ordered emphasis over the operation catalog" is expressed without
  reordering `PATCH_TOOLS`.
