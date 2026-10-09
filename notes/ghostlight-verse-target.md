# Ghostlight Verse: Target

Date: 2026-10-09. Campaign `ghostlight-verse` in the Eureka mind. Operator direction:
ruling `ghostlight-verse:ruling:verse-cast-crossmedia` (her words). Evidence for the
starting state: `F:\Projects\ghostlight-state-2026-10-09.md` (Eyes, 2026-10-09) and
`F:\Projects\aetheria-story-sifting-prior-art.md` Part 2.

## Why

Aetheria needs a public narrative hook. Ghostlight is meant to be the machine that
produces it: load the Aetheria verse, let a persistent cast of characters live in it,
and harvest what happens. Three products are wanted from the same simulated world:

1. **Storylets** for the game (ruling `aetheria-release:ruling:storylets-simulate-sift-lift-weave`).
2. **A serialized cast** an audience can follow through episodes.
3. **A cheap cinematic pipeline** that turns episodes into a video stream, so the
   characters and conflicts people follow on the channel later turn up in the game.

This campaign builds the foundation the three share: a Ghostlight that runs, says
truthfully what it is, and can hold the verse with a persistent cast. The stream
pipeline and the in-game crossover are later campaigns that consume it.

## Phases

1. **Honest and running.** The docs describe the live machine. `main` deploys on
   Yggdrasil through Idunn and stays up. The six open play-session findings and the
   owed Soul pass close. A human can play a session end to end.
2. **A world that can hold factions and a cast.** Relations, factions as actors, places
   and persistent characters enter the ontology, designed from prior art and ruled by
   the operator.
3. **The verse loaded.** A genesis built from AetheriaLore and Aetheria's typed
   `Faction` records. Facts the lore does not state are proposed and reviewed, never
   invented silently.

## Invariants

- `honest-docs`: README, maps and handoffs describe the machine on `main`; history lives in history.
- `main-runs`: the deployed unit on Yggdrasil is built from `main` and stays up; Idunn owns its lifecycle.
- `kernel-decides`: the kernel owns world state through its closed operation set; models propose, the kernel admits or refuses.
- `verse-provenance`: every world fact loaded from the verse names its source page or record; a fact with no source is a proposal until reviewed.
- `canon-one-way`: simulation output never writes back into AetheriaLore or Aetheria's catalog without operator review.
- `cast-persists`: a character keeps identity, memory and commitments across sessions, so episodes can follow them.
- `every-event-renderable`: every event the kernel admits maps to exactly one render path: ship action filmed in Aetheria, or a social verb staged as an Aetheria conversation (portraits and choices; docked face to face, undocked radio over the ship). Summary captions are the only exception (ruling `verse-verbs-two-render-paths`).
- `typed-state`: world state and its exchange are CultCache/CultNet documents; JSON only for schema publication and xenos boundaries.

## Not in scope

- The video/stream pipeline (StreamPixels and its renderers) and episode editing.
- Storylet weaving in the Aetheria runtime, and any Aetheria code.
- Restoring the deleted autonomous tick driver as it was; unattended simulation, if wanted, is designed in phase 2.
- The failed `idunn-odin-*` and `idunn-streampixels-*` units on Yggdrasil (not Ghostlight's).
