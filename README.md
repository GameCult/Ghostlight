# Ghostlight

Ghostlight is a Rust library that keeps a simulated world honest, and a play
daemon built on it. A language model proposes; the library's kernel checks each
proposal against typed world state and admits or refuses it. Places, people,
knowledge, commitments, and time live in the kernel's journal, not in a model's
context window, so they survive restarts and long stories.

## What is here

| Part | Path | What it is |
|---|---|---|
| World kernel | `crates/ghostlight/` | The sealed `ghostlight` library: world state, the closed operation set and reducer, the CultCache journal, the mailbox that is its only door, controllers, elaboration, lenses, the consumer ingress, and the inference ports. |
| Persona projection | `crates/ghostlight-persona-projection/` | The membrane between structured state and a prose-only Persona: projection, Persona turn receipts, and the controller descriptors the kernel consumes. |
| Dungeon | `crates/ghostlight-dungeon/` | The `ghostlight-dungeon` daemon: single-player, DM-led play over one world. HTTP and Eve routes, Heimdall sign-in, app sessions, the play table, CultMesh publication, and Idunn health. |
| Browser | `web/` | A thin Eve browser host and the Heimdall access adapter. It renders the server's surface and owns no product state. |
| Claude SDK sidecar | `sidecar/claude-sdk/` | A Node process that serves `claude`-prefixed models to the kernel through the Claude Agent SDK. It is a stopgap for a direct Messages API port. |
| Deploy recipe | `deployment/idunn/recipe.toml` | The steps Idunn runs to test, build, and seal a release. |
| Eve | `vendor/eve/` | A git submodule (`.gitmodules`) supplying the Eve browser lowering and contracts. |

## How it works

- The kernel is sealed. A consumer reaches it through create/open, immutable
  snapshots, command submission, typed receipts, and the runner and port entry
  points. There is no public mutable state, ID issuer, reducer, or journal
  writer, and the crate declares no cargo features. The seal is proven by
  `compile_fail` doc-tests in `crates/ghostlight/src/lib.rs` and by
  `crates/ghostlight/tests/external_admission.rs`, which forges IDs, digests,
  and opportunities and asserts that each is refused with nothing committed.
- Every change enters one mailbox (`crates/ghostlight/src/mailbox.rs`) as a
  command, is decided by one reducer, and commits atomically to a
  digest-chained journal (`crates/ghostlight/src/journal.rs`, CultCache on redb).
- Models author through typed tools and a repair loop
  (`crates/ghostlight/src/elaboration.rs`, `tool_schema.rs`, `patch.rs`). A
  refused proposal returns its mismatches to the model; it never edits state.
- A Persona speaks only prose. A Projector builds its view, and an Interpreter
  lowers its words into typed proposals with exact source quotes
  (`crates/ghostlight/src/controllers.rs`).
- Vault evidence for seeding comes from a read-only markdown directory reader
  (`crates/ghostlight/src/vault.rs`). The Dungeon's seed lane reads the
  directory named by `GHOSTLIGHT_SEED_VAULT_ROOT`, read once at startup, so
  changing it needs a restart
  (`crates/ghostlight-dungeon/src/runtime.rs`).
- Other programs can propose changes to the characters they control through
  `POST /cultnet/world-patch` (`crates/ghostlight-dungeon/src/runtime.rs`,
  `crates/ghostlight/src/consumer.rs`). The contract is
  `docs/architecture/ghostlight-world-consumer-api.md`.

## Ghostlight Dungeon

Dungeon is the single-player, DM-led client. A player signs in with Heimdall,
creates a world, seeds it from a vault, approves and activates it, and plays in
free text. The play table (`crates/ghostlight-dungeon/src/play.rs`) owns the
turn lifecycle; the interface is one Eve surface, `ghostlight.play`
(`crates/ghostlight-dungeon/src/eve.rs`, described in
`docs/architecture/ghostlight-eve-native-interface.md`). Because its player is
not a developer, Dungeon is judged by whether someone can sit down and play.

Where it runs:

- **Yggdrasil**, under Idunn, built from `main` by `deployment/idunn/recipe.toml`.
  Idunn owns the daemon's lifecycle and the write lease. Odin supplies discovery
  and Heimdall's command-boundary record. The operating runbook is
  `gamecult-ops/runbooks/ghostlight-dungeon-yggdrasil.md`.
- **Inference** is routed by model name: a `local/` prefix goes to an
  OpenAI-compatible loopback endpoint
  (`GHOSTLIGHT_LOCAL_ENDPOINT`, `crates/ghostlight/src/local_inference.rs`),
  which in production is Bonsai on Raven's GPU reached through a loopback
  tunnel; a `claude` prefix goes to the SDK sidecar
  (`GHOSTLIGHT_SDK_SIDECAR`, `crates/ghostlight/src/sdk_inference.rs`).
  Ghostlight never reads, stores, or forwards a model credential.
- **Sign-in** is Heimdall's. Ghostlight keeps a hashed HttpOnly session and
  derives authority from the world's owner and approvers.

## Direction

The next work makes Ghostlight the simulation behind Aetheria's public story.
The operator's rulings (campaign `ghostlight-verse` in the Eureka mind) ask for:

- a world loaded from the Aetheria verse, with factions and a cast;
- storylets for Aetheria;
- a persistent cast whose characters keep identity, memory, and commitments
  across sessions, so serialized episodes can follow them;
- episodes a cheap pipeline can turn into a video stream, and channel
  characters who later appear in the game.

None of this exists yet. Nothing under `crates/` loads Aetheria lore, models
storylets, or produces an episode. Every event the kernel admits is meant to
have exactly one render path: ship action filmed in Aetheria, or a social verb
staged as an Aetheria conversation. Today a world can declare any affordance
kind, and the kernel does not enforce that restriction.

## Build and test

The release target is Linux on Yggdrasil; a Windows build says nothing about
it. Idunn runs the steps in `deployment/idunn/recipe.toml`, which are the
verification:

```sh
git submodule update --init vendor/eve
cargo test --locked -p ghostlight-persona-projection
cargo test --locked -p ghostlight
cargo test --locked -p ghostlight-dungeon --bin ghostlight-dungeon
cargo build --locked --release -p ghostlight-dungeon --bin ghostlight-dungeon
npm --prefix web ci
npm --prefix web test
npm --prefix web run build
```

The sidecar has its own `npm --prefix sidecar/claude-sdk run build` and `test`
(also the root scripts `sdk:build` and `sdk:test`). Four Dungeon tests are
`#[ignore]`d because they drive the vendored browser lowering through
`tools/eve_client_bridge.mjs` and need Node beside cargo. The release
acceptance step runs `controllers::tests::real_local_model_cognition_modes_commit_speech`
(ignored by default) against a real model.

To run Dungeon locally: `npm run dungeon:run`, or
`cargo run -p ghostlight-dungeon -- --state-root PATH`. Configuration is by
`GHOSTLIGHT_*` environment variables; the recipe's `required_environment`
lists the ones a deployment sets.

## Documents

- `AGENTS.md`: repository doctrine for agents.
- `docs/architecture/ghostlight-dungeon-mvp.md`: the authority architecture
  of the world machine.
- `docs/architecture/ghostlight-world-ontology.md`: the vocabulary and the
  kernel's current mechanism.
- `docs/architecture/ghostlight-world-consumer-api.md`: the consumer contract.
- `docs/architecture/ghostlight-eve-native-interface.md`: the Eve surface.
- `docs/architecture/ghostlight-library-extraction.md`,
  `ghostlight-stock-lenses.md`: closed records of landed work. Pages ending in
  `-cut.md` and `-postmortem.md` are dated history.
- `docs/architecture/ghostlight-play-agent.md`: historical. Its status line
  predates the play-agent cuts; read it with `-cut.md` beside it, not as the
  current play surface (`crates/ghostlight-dungeon/src/play.rs` is).
- `docs/architecture/ghostlight-session-zero.md`: unbuilt design, superseded by
  the play agent. Nothing in `crates/` implements it.
- `notes/fresh-workspace-handoff.md`: where things run and where open work is
  recorded.
- `docs/public-architecture/`, `docs/articles/`: unbuilt design and essays
  written before the rebuild. They describe campaigns, co-op, Nemesis
  and strategic resolvers as organs; none of that exists in `crates/`. They do not
  describe the running machine.
- `docs/product/lore-vault-entitlements.md`: a product proposal for hosted
  vaults; nothing in `crates/` implements it.

Runtime documents use MessagePack-backed CultCache. JSON appears only at schema
publication, browser, model-provider, and diagnostic boundaries.
