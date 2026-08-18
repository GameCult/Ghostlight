# Ghostlight

Ghostlight builds persistent generative people and worlds. Its active product is
[Ghostlight Dungeon](https://ghostlight.gamecult.org): a Vault-grounded narrative
simulation in which places, knowledge, relationships, institutions, clocks, and
consequences survive beyond the model's current context.

The project began as a research and fixture pipeline for socially persistent
Aetheria agents. That material remains useful regression evidence. The live
machine is now a Rust runtime with typed campaign state, one canonical world
authority, bounded Persona projection, fiction-first action resolution, and a
multiresolution agency graph for keeping large settings active at finite cost.

## What Ghostlight Owns

- `WorldKernel` is the sole owner of committed campaign state and revisions.
- Every player command, NPC action, strategic tick, wait, import, and reload
  enters the same validated campaign mailbox and atomic CultCache commit path.
- Vault evidence grounds world compilation; branch-local invention remains
  distinct from canon.
- Hosted lore Vaults are planned as Git-synchronized, Obsidian-compatible
  Markdown hierarchies. The Vault service owns their indexes; campaigns retain
  only manifest bindings and exact evidence receipts.
- Projector → Persona → Interpreter turns produce proposals. Models never
  commit world state.
- Knowledge, perception, capability, location, custody, and authority are
  validated against the exact actor proposing an action.
- Fiction-first d20 resolution treats player text as an attempt rather than an
  accomplished fact.
- Multiresolution Gestalts keep people, institutions, and factions active under
  a bounded Persona-cell budget without merging rivals into one mind or erasing
  named individuals.
- Away-time simulation advances clocks and remote agency without puppeting or
  directly harming an absent player character.
- CultMesh publishes typed service and Eve/CultUI state; the browser lowers that
  surface and does not become a second state authority.

## Current Status

Ghostlight Dungeon is implemented and deployed as a private Starfire-hosted
playtest harness. The current acceptance surface covers:

- source-grounded world compilation with approval previews;
- persistent campaigns, forks, resets, exports, and Heimdall-account-isolated sessions;
- parallel affected-character Projector/Persona/Interpreter waves;
- impossible-action refusal, assessed stakes, server-side rolls, and receipted
  commits;
- strategic clocks, away-time catch-up, institution activity, migration, and
  information-channel-aware news;
- connected cohesive and arena simulation covers at budgets from 1 to 32;
- Gestalt materialisation, folding, member deltas, migration, and later
  rematerialisation of the same person;
- atomic rejection of malformed, stale, or semantically invalid model waves;
- provider, token, cache, latency, validation, state-version, and build receipts;
- exact-build deployment and restart verification on Starfire.

The immediate forge work is pressure-testing whether multiresolution Gestalts
produce satisfying setting-wide activity, callbacks, and surprises per token,
then running the two-tester paid-alpha path. The public site is provisional;
checkout is not live.

## Multiplayer Intention

The first paid alpha remains single-player. Multiplayer is an intended extension
of the existing authority model, not a separate chat mode.

A campaign will continue to have one `WorldKernel`. Authenticated human sessions
will control distinct canonical actors, and every human or Persona proposal will
enter the same campaign mailbox. Each player receives a perception-specific
narrative projection; private knowledge is never unioned into a party prompt.
Splitting the party therefore changes occupancy and perception without creating
contradictory copies of the world.

The durable intention and unresolved social-policy questions are recorded in
[`docs/architecture/ghostlight-dungeon-multiplayer-intention.md`](docs/architecture/ghostlight-dungeon-multiplayer-intention.md).
It does not expand the current MVP acceptance promise.

## Lore Vault Product Intention

The hosted product will accept ordinary Obsidian-compatible lore Vaults synced
through Git. Contributor and Private plans provision one active custom Vault;
their Plus variants provision three, subject to combined indexed-source and
import allowances. The entitlement, accounting rules, authority boundary, and
security gates are recorded in
[`docs/product/lore-vault-entitlements.md`](docs/product/lore-vault-entitlements.md).
Custom tenant import is not yet part of the deployed tester harness.

## Architecture

The shortest reliable re-entry path is:

- [`docs/architecture/ghostlight-dungeon-mvp.md`](docs/architecture/ghostlight-dungeon-mvp.md): runtime authority, compiler, action loop, persistence, hosting, and security;
- [`docs/architecture/ghostlight-multiresolution-agency.md`](docs/architecture/ghostlight-multiresolution-agency.md): dynamic Gestalt partitioning, cohesive and arena cells, fairness, and atomic strategic waves;
- [`notes/ghostlight-current-system-map.md`](notes/ghostlight-current-system-map.md): current implemented pipeline;
- [`notes/ghostlight-implementation-plan.md`](notes/ghostlight-implementation-plan.md): live sequence and next pressure tests;
- [`state/map.yaml`](state/map.yaml): canonical human-readable project state.

## Repository Shape

- `crates/ghostlight-dungeon/`: Rust daemon, kernel, compiler, persistence,
  provider stages, web lowerer, and acceptance harnesses;
- `crates/ghostlight-persona-projection/`: generalized projection membrane owned
  by Ghostlight and consumed by Epiphany;
- `docs/architecture/`: durable contracts and authority maps;
- `docs/articles/`: accessible public explanations;
- `notes/`: implementation planning, handoff, and current-system maps;
- `schemas/`: published JSON Schemas for typed boundary documents;
- `examples/`: earlier fixtures and regression material;
- `state/`: human-readable project memory plus older research-pipeline state;
- `tools/`: state, validation, and fixture helpers.

Runtime documents and campaign exports use MessagePack-backed CultCache `.cc`.
JSON exists at schema publication, browser, MCP, model-provider, and diagnostic
boundaries; it is not the canonical hosted state store.

## Useful Commands

```powershell
cargo test --workspace
npm run state:status
npm run state:prepare-compaction
npm run schema:validate
```

For Codex-driven work, read `AGENTS.md` before changing the repository. It owns
the persistence, grounding, verification, and handoff discipline.
