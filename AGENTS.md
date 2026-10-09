# Ghostlight Instructions

## Project Purpose

Ghostlight is a Rust world kernel and the play daemon built on it. Models
propose; the kernel admits or refuses. The same kernel is meant to run the
simulation behind Aetheria's public story, a persistent cast and storylets, as
well as Ghostlight Dungeon, the single-player DM-led client. `README.md` says
what exists today and what does not.

## Where State Lives

- Campaign state (targets, rulings, cut specs, reports, verdicts, findings,
  follow-ups) lives in the Eureka mind, not in this repo. The current campaign
  is `ghostlight-verse`; query it through the `eureka-state` tools.
- `notes/ghostlight-verse-target.md` and `notes/ghostlight-verse-map.md` are the
  campaign's prose target and map. Read the map for probed body facts.
- This repo has no state directory, no state CLI, and no evidence ledger. Do not
  create one. History belongs in git; durable decisions belong in the mind.
- `notes/fresh-workspace-handoff.md` is a pointer: what runs, how it deploys,
  where open work is queried.
- Dated cut documents and postmortems (`docs/architecture/*-cut.md`,
  `*-postmortem.md`) are history. Do not edit them, including their old paths.

## Invariants

- **The kernel decides.** The kernel owns world state through its closed
  operation set. Models, tools, controllers, and consumers produce proposals
  only. No code outside the kernel commits world state, and no compatibility
  writer exists.
- **The library is sealed.** `crates/ghostlight` exposes create/open,
  immutable snapshots, command submission, typed receipts, and the runner and
  port entry points. Do not publish mutable world types, an ID issuer, a reducer
  entry, or a journal handle for the convenience of tests or adapters. The seal
  is proven by the `compile_fail` doc-tests in `crates/ghostlight/src/lib.rs`
  and by `crates/ghostlight/tests/external_admission.rs`; keep both passing and
  extend them when the public surface changes.
- **Every commit is a command.** Everything enters through the mailbox
  (`crates/ghostlight/src/mailbox.rs`) as a command. Do not append a commit when
  reduction produces no canonical mutation.
- **A Persona never sees structured state.** Persona turns are prose; an
  Interpreter lowers them to typed proposals, and anything it cannot lower
  becomes a recorded gap, not invented state.
- **Typed state.** Runtime documents are CultCache `.cc`. JSON is for schema
  publication and boundaries with other systems.
- **Credentials.** Ghostlight never reads, copies, forwards, or logs a model
  credential. A consumer never restates a foreign owner's document shape as its
  own strict struct; read the owner's published contract and test against the
  real counterpart.
- **Canon flows one way.** Simulation output never writes back into AetheriaLore
  or Aetheria's catalog without operator review.

## Where Things Are

- Kernel: `crates/ghostlight/src/`. Its vocabulary and current mechanism:
  `docs/architecture/ghostlight-world-ontology.md`. The consumer contract:
  `docs/architecture/ghostlight-world-consumer-api.md`.
- Dungeon daemon: `crates/ghostlight-dungeon/src/` (`runtime.rs` for routes and
  commands, `play.rs` for the play table, `eve.rs` for the surface).
  Interface authority: `docs/architecture/ghostlight-eve-native-interface.md`.
- Persona membrane: `crates/ghostlight-persona-projection/src/lib.rs`.
- Browser host: `web/`. Claude SDK sidecar: `sidecar/claude-sdk/`.
- Architecture rationale: `docs/architecture/ghostlight-dungeon-mvp.md`.
  `notes/ghostlight-implementation-plan.md` is the historical plan of the 2026-09
  rebuild, not the current plan.

## Build, Verify, Deploy

Ghostlight Dungeon ships to Linux on Yggdrasil. Idunn freezes an exact commit,
runs the locked native Linux tests, builds the daemon and web projection in
pinned containers, and seals the release. The steps are
`deployment/idunn/recipe.toml`; the owning runbook is
`gamecult-ops/runbooks/ghostlight-dungeon-yggdrasil.md`.

You are probably reading this on a Windows workstation. That is where the shell
is, not where the artifact runs. A Windows compile is not evidence about the
Linux release; build for the target, or let the deploy path's builder do it, and
say which one you used. Heavy builds and test runs go to Yggdrasil, not the
workstation. Deploy only through Idunn (`idunn up ghostlight`).

For infrastructure, SSH, Idunn, Odin, or Heimdall work, consult
`F:\Projects\gamecult-ops` before acting.

## Operating Discipline

- Restate the current mechanism and intended change before substantial edits.
  Prefer one clear hypothesis per iteration.
- Verify with checks that reflect the real goal, not proxy success. Revert
  changes that do not clearly improve the target.
- If the diff grows while understanding shrinks, stop and diagnose.
- Delete obsolete authority before adding a new path. Do not keep an old writer
  alive behind a shim.
- Commit completed work and push it. Merge, confirm the tip by SHA, then delete
  the branch; push only after reading the test result line.
- In documentation, describe the live system and the current rule directly. No
  claim of capability without a file that implements it. Keep rejected history
  short and decision-relevant, or leave it in git.

## Aetheria Grounding

- Before writing or generating Aetheria material, check AetheriaLore for the
  factions, institutions, species or body types, location, and time period
  involved. Ground detail in the source's material facts: habitat, law, money,
  route access, surveillance, communication channels, and local failure costs.
- If the source is too vague for a concrete detail, do not bluff. Mark the gap
  and propose a narrow elaboration for AetheriaLore, subject to operator review.
- Choose a tonal mode on purpose. Aetheria supports wit with stakes, warmth,
  absurdity, domestic texture, ritual quiet, wonder, horror, noir, and dry
  systems prose; do not default to constant crisis. Comic precision in the
  Adams/Pratchett manner is a good default, where jokes reveal stakes instead of
  dissolving them.
- Establish ordinary life before interruption: routines, work, relationships,
  small desires, and local accommodations are worldbuilding, not filler.
