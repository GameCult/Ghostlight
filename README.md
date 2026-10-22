# Ghostlight

**Ghostlight is a world kernel: typed world state that a language model can
propose changes to and can never write.** The valuable part is
`crates/ghostlight`. The Dungeon game, one client of it, is parked (below).

**The thesis.** World state is typed documents. Every change to it, and every
narrative dependency on it, is an operation in a closed, predefined algebra.
Models propose operations; the kernel checks each against the state (references,
preconditions, authority, conservation, who knows what) and admits it or refuses
it with the complete list of mismatches. Narration describes only what was
committed. A model-as-game-master holds the world in a context window and
drifts; here the world is a digest-chained journal that survives restarts and
cannot be talked into contradicting itself.

**Use it as a structured state backend.** Do not judge it by the parked Dungeon's
prose. The kernel is a library; prose comes from whichever model you plug into
its ports.

- `crates/ghostlight`: the sealed library. A consumer gets create/open
  (`WorldMailbox::open`, `create`), immutable snapshots (`snapshot`), command
  submission through role-scoped ports (`PlayPort`, `ConsumerPort`,
  `ControllerPort`, `SeedPort`), typed receipts, and the runner and inference-port
  entry points (`ControllerRunner`, `ElaborationRunner`, `SeedRunner`,
  `InferencePort`). Mutable state, ID issuers, the reducer and the journal
  writer are not public. `compile_fail` doc-tests in `crates/ghostlight/src/lib.rs`
  and `crates/ghostlight/tests/external_admission.rs` prove the seal.
- `docs/architecture/ghostlight-world-ontology.md`: the vocabulary (subjects,
  places, routes, holdings, knowledge, commitments, pressures), the operation
  algebra, and what the reducer checks.
- `docs/architecture/ghostlight-world-consumer-api.md`: the contract for a
  program that reads a world and proposes changes to the characters it controls
  (`POST /cultnet/world-patch`; no process on main serves that door, a host serves
  `ghostlight::admit_document`).

Start with those two documents, then `cargo doc -p ghostlight --open`.

**Populate the algebra first.** The kernel owns the component operations; a world owns its
affordance catalog, the verbs its actors can attempt. A fresh world knows one: the built-in
Speak, which records an utterance and changes nothing. Until your world declares affordances
(at genesis, by patch, or through elaborators) for moving, trading, taking, deciding and so on,
every intent a model voices lowers to speech or to a recorded gap, and the prose built on that
will be thin no matter how good the model is. The Aetheria verse genesis
(`crates/ghostlight/src/verse/genesis.rs`) declares its grammar's verbs, and today they still
only record the act; their effects are the next phase.

## Elaborators

An elaborator is a model session, confined to one jurisdiction of the world, that
adds structure where the world is thin: it answers a derived causal boundary
(a destination nobody has described yet) or a scale deficit with typed
declarations, and the kernel verifies the result like any other patch. Sessions
are resumable and checkpointed against committed history. Eight stock lenses
(`crates/ghostlight/src/lens.rs`: patina, charter, ledger, hearth, tangle, veil,
ember, numen) give each session a role-specific emphasis, for example customs and
local jokes, or money and ledgers. A lens changes the prompt only, never what the
kernel admits. Because boundaries are derived from state, elaborators can be
dispatched as the player approaches a place or a social structure, instead of
authoring the world up front.

## Direction (planned, not built)

Use the kernel to simulate a whole world with many kinds of actor: Aetheria's
factions and a persistent cast whose members keep identity, memory and
commitments across sessions. Newspaper-staff simulations before the 2026-08
teardown are the earlier proof of the idea; that code is in git history, not in
the tree. Running this at scale is gated by inference cost, not by the design.
`crates/ghostlight-verse` is the first slice: `ghostlight-verse genesis` creates
an Aetheria verse world from the Aetheria catalog and `ghostlight-verse run`
advances it a number of ticks under a token cap, reporting per tick what each
faction did and what it cost. Nothing loads AetheriaLore prose, models
storylets, or produces episodes yet. Targets: `notes/ghostlight-verse-target.md`.

## Ghostlight Dungeon (parked)

The single-player, DM-led client and its daemon, Eve play surface and browser
host are parked at tag `parked/ghostlight-eve-surface`. Nothing is deployed and
nothing on main serves HTTP. The Dungeon returns when Thing 2 gives it a UI
(follow-up `dungeon-ui-on-thing2`); the rebuild starts from the tag.

## Repo map

| Path | What it is |
|---|---|
| `crates/ghostlight/` | The sealed kernel library. |
| `crates/ghostlight-persona-projection/` | Membrane between structured state and a prose-only Persona. |
| `crates/ghostlight-verse/` | The `ghostlight-verse` operator command line (`genesis`, `run`). |
| `sidecar/claude-sdk/` | Node process serving `claude`-prefixed models to the kernel; a stopgap for a direct Messages API port. |
| `AGENTS.md` | Invariants and where things are. |
| `docs/architecture/ghostlight-dungeon-mvp.md` | The parked Dungeon's authority architecture (design record). |
| `notes/` | The verse campaign's target and map, the handoff pointer, local smoke notes. |

`docs/public-architecture/`, `docs/articles/` and `docs/product/` are design
essays and proposals from before the rebuild. They describe organs (campaigns,
co-op, Nemesis, newspapers) that are not in `crates/`; they do not describe the
running machine. Pages ending in `-cut.md` or `-postmortem.md` are dated history.

## Build and test

Nothing deploys from this repo. Linux verification runs on Yggdrasil, not on a
workstation:

```sh
cargo build --locked --workspace
cargo test --locked -p ghostlight-persona-projection
cargo test --locked -p ghostlight
cargo test --locked -p ghostlight-verse
```

The kernel alone needs only `cargo test -p ghostlight`. The sidecar has
`npm run sdk:build` and `npm run sdk:test`. The acceptance test
`controllers::tests::real_local_model_cognition_modes_commit_speech` is ignored
by default and needs a real model.

Inference is routed by model name: `local/` goes to an OpenAI-compatible
loopback endpoint, `claude` to the SDK sidecar, and the verse runner's `hosted/`
to an https endpoint. Ghostlight never reads, stores or forwards a model
credential beyond a key file named at run time.

Runtime documents are MessagePack-backed CultCache. JSON appears only at schema
publication, browser, model-provider and diagnostic boundaries.
