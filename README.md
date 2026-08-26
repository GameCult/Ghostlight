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
- Foreground actions, NPC reactions, strategic Gestalt activity, waits, travel,
  and population fission lower into one closed semantic world-mutation algebra.
  Means and intended effects remain proposals; only an admitted mutation batch
  can change canonical components. There is no model-authored JSON Patch path.
- Vault evidence constrains world compilation without pretending every playable
  route, procedure, or local institution is already written down. The compiler
  synthesizes the smallest compatible branch elaboration, exposes it for
  approval, and keeps it distinct from canon. Details that must not vary belong
  in the Vault.
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
- Odin receives Ghostlight's advertised boundary schemas and typed projections
  over ACK-bounded RUDP publication. Internal runtime schemas are not treated as
  discovery contracts merely because Ghostlight owns them.

## Eve-native interface

Ghostlight publishes one stable logical surface, `ghostlight.play`, for
anonymous entry, Session Zero, Contract Review, and campaign play. The browser
contains Eve's canonical provider host and lowering package plus Heimdall's
access-plugin adapter. It does not contain Ghostlight-specific screen renderers
or domain API routing.

Editable Eve bindings are the input model. Text, number, and choice controls
bind either provider-owned typed state or renderer-local draft state; an
operation captures named bindings and submits one canonical Eve command.
Rejected or stale commands preserve local drafts, while an accepted receipt may
clear named bindings before the host refetches the authoritative surface. A
browser may lower this interaction to an HTML form for accessibility, but forms
are not part of Eve's state or command ontology.

Heimdall authentication is composed as the required
`gamecult.heimdall.access` Eve plugin. Heimdall owns OAuth attempts, provider
callbacks, claims, entitlements, and single-use completion redemption.
Ghostlight owns only its hashed HttpOnly app session and derives campaign
authority from canonical membership. The browser sees an opaque attempt handle,
never a claim, refresh credential, account hash, member ID, or actor authority.
See
[`docs/architecture/ghostlight-eve-native-interface.md`](docs/architecture/ghostlight-eve-native-interface.md).

## Current Status

Ghostlight Dungeon is implemented and deployed as a native Yggdrasil,
Heimdall-gated playtest harness. Idunn owns daemon continuity, Odin owns Verse
discovery, and the application remains the sole owner of campaign truth.
Authentication commands resolve Heimdall's redacted private boundary through
Odin; no direct route is browser- or unit-owned, and valid local sessions do
not phone home on routine commands. The
current acceptance surface covers:

- persistent DM-led Session Zero with shared/private channels, typed contracts,
  private boundaries, character bargains, digest-bound unanimous approval, and
  atomic publication;
- source-constrained world compilation from an approved brief: canon evidence
  pins what must remain true, compatible game-scale connective tissue becomes
  disclosed branch-local state, and genuine premise conflicts return to
  negotiation instead of borrowing a nearby story;
- selectable Aetheria and Kalsa Vaults, with Kalsa's player-safe `Public` and
  GM-only `Spoilers` evidence lanes preserved through exact receipts;
- persistent campaigns, forks, resets, exports, and Heimdall-account-isolated sessions;
- parallel affected-character Projector/Persona/Interpreter waves;
- impossible-action refusal, assessed stakes, server-side rolls, and receipted
  commits;
- strategic clocks, away-time catch-up, institution activity, migration, and
  information-channel-aware news;
- bounded shared-scene co-op for up to eight Heimdall members, with exact
  member→actor authority, actor-filtered surfaces, unanimous time/budget
  governance, and no player puppeting or PvP mutation;
- connected cohesive and arena simulation covers at budgets from 1 to 128;
- Gestalt materialisation, folding, member deltas, migration, and later
  rematerialisation of the same person;
- atomic rejection of malformed, stale, or semantically invalid model waves;
- one kernel-owned mutation reducer for foreground, reaction, strategic, time,
  travel, population-fission, and bounded region-expansion consequences, with
  exact component versions and mutation proof receipts;
- provider, token, cache, latency, validation, state-version, and build receipts;
- provider-neutral inference through either direct DeepSeek/OpenRouter
  boundaries or the independent CodexConnector daemon's encrypted loopback
  CultNet boundary, while every prompt, schema, retry, interpretation, and world
  commit remains Ghostlight's;
- exact-build deployment, state migration, public cutover, and restart
  verification on Yggdrasil.

Initial compiler seed publication is a bounded empty-store creation
transaction. Named-person materialisation and folding are resolution
transactions that preserve individual deltas without rewriting their Gestalt.
The remaining forge gates are expansion and review of the agency corpus beyond
its current candidate seed, removal of legacy model effect schemas,
multi-account human pressure testing of Session Zero privacy/publication and
bounded co-op, and continued multiresolution Gestalt agency pressure. The
public site is provisional; checkout is not live.

## Bounded Co-op

Campaign creation now supports one to eight authenticated players. One
`SessionZeroKernel` owns negotiation; after unanimous approval, one
`WorldKernel` owns the shared campaign. Membership binds each account to one
exact actor, and every player receives an actor-filtered Eve projection.

This milestone is intentionally one shared scene with sequential public
actions. PvP, split parties, private in-play actions, delegation, late joining,
and simultaneous declarations remain closed until their consent and governance
contracts exist. See
[`docs/architecture/ghostlight-dungeon-session-zero.md`](docs/architecture/ghostlight-dungeon-session-zero.md)
and the longer-term
[`multiplayer intention`](docs/architecture/ghostlight-dungeon-multiplayer-intention.md).

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
- [`docs/architecture/ghostlight-dungeon-session-zero.md`](docs/architecture/ghostlight-dungeon-session-zero.md): campaign negotiation, privacy, publication, membership, and bounded co-op;
- [`docs/architecture/ghostlight-multiresolution-agency.md`](docs/architecture/ghostlight-multiresolution-agency.md): dynamic Gestalt partitioning, cohesive and arena cells, fairness, and atomic strategic waves;
- [`docs/architecture/ghostlight-eve-native-interface.md`](docs/architecture/ghostlight-eve-native-interface.md): Eve bindings, the stable provider surface, private Heimdall command plane, app-session custody, and browser cut line;
- [`docs/architecture/ghostlight-transition-algebra.md`](docs/architecture/ghostlight-transition-algebra.md): canonical subjects and components, semantic mutations, admission envelopes, atomic reduction, and the remaining writer migration;
- [`notes/ghostlight-current-system-map.md`](notes/ghostlight-current-system-map.md): current implemented pipeline;
- [`notes/ghostlight-implementation-plan.md`](notes/ghostlight-implementation-plan.md): live sequence and next pressure tests;
- [`state/map.yaml`](state/map.yaml): canonical human-readable project state.

## Repository Shape

- `crates/ghostlight-dungeon/`: Rust daemon, kernels, compiler, persistence,
  provider stages, Eve projection/command ingress, and acceptance harnesses;
- `crates/ghostlight-persona-projection/`: generalized projection membrane owned
  by Ghostlight and consumed by Epiphany;
- `docs/architecture/`: durable contracts and authority maps;
- `docs/articles/`: accessible public explanations;
- `notes/`: implementation planning, handoff, and current-system maps;
- `schemas/`: published JSON Schemas for typed boundary documents;
- `examples/`: earlier fixtures and regression material;
- `state/`: human-readable project memory plus older research-pipeline state;
- `tools/`: state, validation, and fixture helpers.
- `web/`: thin Eve browser host, Ghostlight transport, and Heimdall plugin
  adapter; product state and product-specific rendering remain server-owned.

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
