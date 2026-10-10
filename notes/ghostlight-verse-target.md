# Ghostlight Verse: Target, revision 3

Date: 2026-10-10. Campaign `ghostlight-verse` in the Eureka mind. Imagination
`imagination-gl-verse-sim-cuts`, session `self-2026-10-10-ag`. Supersedes revision 2
(2026-10-09) where they differ; what revision 2 said that still holds is kept here.
Rulings are cited by id and read in the mind; this page does not restate them.
Evidence: `notes/ghostlight-verse-map.md`, sections "Verse sim: port inventory and
model page" and "Verse sim: phase A body facts"; `gamecult-ops`
`docs/research/llm-world-sim-prior-art.md` at `6b4a874` (*prior art (a)-(d)*);
`F:\Projects\aetheria-story-sifting-prior-art.md`.

## Why

Aetheria needs a public narrative hook, and Ghostlight is the machine that makes it
(ruling `verse-cast-crossmedia`): load the Aetheria verse, let a persistent cast live
in it, and harvest storylets for the game, a serialized cast, and stories a cheap
cinematic pipeline can stream.

Ruling `aetheria-sim-first` makes simulating Aetheria the campaign's priority, ahead
of every Dungeon affordance. Ruling `verse-sim-pipeline` sets the pipeline: the lore
is loaded, elaborated, and run as a multiresolution-gestalt world in Aetheria's
ontology and action grammar, on a schedule, feeding the content pipelines, under
operator meddling. Ruling `gestalt-machinery-port` ports the pre-rebuild gestalt
machinery rather than redesigning it, and ruling `teardown-was-authority-sprawl`
keeps the ad hoc mutation rules dead. The map sorts the old tree (`f9f019b`) into
both lists and names sixteen forced changes (F1-F16).

The field agrees with the split: working LLM world simulations keep world state in a
non-LLM store and map free-text intent onto an enumerated, checked action (prior art,
"What the field converged on", items 1-2). Cost comes down by calling the model less
per agent per tick (item 3), which is what the gestalt cover does. Replay comes from
stored responses, not seeds (item 5).

## Phases

1. **Honest and running** (revision 2): the Dungeon lane closes its phase-1 work in
   parallel. It does not gate the verse.
2. **Phase A, the first watched run.** Aetheria's grammar exists as a CultCache
   document set in Aetheria's catalog; a Ghostlight world binds it at genesis and the
   kernel refuses any affordance kind it lacks. The canonical verse world is created
   from Aetheria's `Faction` records, one subject per record under a checked import
   key. A hosted cheap-model lane records every call with its response body and
   token usage. The operator starts a manual run under a hard token cap and reads,
   per tick, what each faction did and what it cost. Mapped as cut specs (map,
   "Phase A cut order").
3. **Phase B, the verse loaded.** The relation edge (F6) enters the kernel; the
   faction records' allegiances become Alliance and Rivalry edges; elaboration from
   AetheriaLore declares places, populations as gestalts and named characters as
   persons.
4. **Phase C, the gestalt port.** Pins, debt, checkpointed parallel cells, strategic
   individuation and fission (F1-F16) on main's kernel; replay of a whole run.
5. **Phase D, meddling and the newspaper.** The operator's Eve surface (steer, then
   inject), the ported newspaper as the first reader, then nightly runs.

## Invariants

Labels carried from revision 2 keep their meaning.

- `honest-docs`: README, maps and handoffs describe the machine on `main`; history lives in history.
- `main-runs`: every deployed Ghostlight unit is built from `main` and stays up; Idunn owns its lifecycle.
- `kernel-decides`: the kernel owns world state through its closed operation set; models propose, the kernel admits or refuses, and no model decides an outcome.
- `grammar-governs` (ruling `grammar-scope-ruled`): every change to world state is a kernel command reduced through the closed operation set. Every act by a subject is an invocation of an affordance whose kind is in the world's bound grammar. The only other writers are clock motion, elaboration patches answering a derived boundary or deficit, and owner acts recorded with the operator as principal. Nothing else writes. Changing simulation resolution writes nothing.
- `grammar-bound`: a verse world binds one grammar revision by content digest at genesis, and the kernel refuses an affordance kind, or a role signature, that the bound grammar does not carry.
- `resolution-not-identity`: the resolution at which a subject is simulated never changes who exists, what they hold or what they know. A person given up by a gestalt stays a person.
- `one-record-one-subject`: each imported Aetheria or lore record maps to exactly one subject or entity, and the mapping is checked when a patch is admitted.
- `verse-provenance`: every world fact loaded from the verse names its source page or record; a fact with no source is a proposal until reviewed.
- `canon-one-way`: simulation output never writes back into AetheriaLore or Aetheria's catalog without operator review.
- `readers-read`: a pipeline consumer reads committed history and writes only its own output.
- `cast-persists`: a character keeps identity, memory and commitments across runs, so episodes can follow them.
- `every-event-renderable`: every event the kernel admits maps to exactly one render path: ship action filmed in Aetheria, or a social verb staged as an Aetheria conversation (ruling `verse-verbs-two-render-paths`). Summary captions are the only exception, and they cover the named non-verb writers: clock motion, elaboration and owner acts.
- `run-resumable`: an interrupted run resumes from its last checkpoint and commits nothing twice.
- `run-replayable`: a run re-executed from its recorded provider responses produces the same journal.
- `run-bounded`: a run never spends past its token cap, and its fiction span, calls, tokens and cost are recorded.
- `dungeon-isolated`: a verse run cannot slow, stop or change Dungeon play.
- `meddling-attributed`: every operator intervention is a typed act, recorded with the operator as principal, through the same doors as every other writer.
- `typed-state`: world state, run records, call records, the grammar and pipeline output are CultCache documents; exchange is CultNet; JSON only for schema publication and xenos boundaries.

## Defaults (stated, not forks)

- **The grammar** (ruling `verse-grammar-home-ruled`) is two CultCache document
  types in Aetheria's catalog (`GameData/Aetheria.cc`, registered in
  `AetheriaStores.CatalogTypes`): a global `aetheria.verse_grammar` carrying the
  revision, and one `aetheria.verse_verb` per verb, carrying the verb's canonical
  kind name, its one render path (ship action or conversation), its participant
  roles and the referent kind each binds, and a description. Aetheria owns which
  verbs exist and how each renders; Ghostlight owns what each verb does to world
  state (question `verb-effects-owner`). The starter set is authored by an AetherDb
  command: the ship-action verbs of Aetheria's current scenario vocabulary and a
  small social set that includes `speak`, the kernel-built narrative affordance.
- **Grammar binding.** A world created under a grammar stores its schema id,
  revision and content digest (sha256 over the canonical encoding of its verbs,
  sorted by kind). Revising the grammar means a new world, or a later migration
  designed when needed: the kernel today refuses earlier schemas and never migrates.
  A world with no grammar binding (every Dungeon world) keeps today's open catalog.
- **The canonical world** (ruling `verse-world-canonical-ruled`) is one world built
  from Aetheria's catalog and AetheriaLore at named commits. Its import key is
  `(CultCache schema id, record key)` for catalog records and `(vault-relative page
  path)` for lore pages; the subject carries it, and admission refuses a second
  subject or entity with the same key. Lore pages enter in phase B.
- **Cast.** A cast member is a `Person` subject marked by a cast entry in the run
  record's roster, not by a kernel component: being followed by an audience changes
  attention, not the world. The operator curates the roster as a run control
  (ruling `meddling-first-ruled`); the cover keeps cast members at singleton
  resolution (the old `MinimumIndividualDetail` pin).
- **Fission** (ruling `gestalt-fission-auto`) is admitted during a run as an
  elaboration patch answering a derived boundary, one of the named non-verb writers
  under `grammar-scope-ruled`. It is not a grammar verb and not gated. The operator
  sees it afterwards: the run record lists each fission commit, and the meddling
  surface shows it in the run's history beside the injection controls.
- **Separate verse process.** The runner is library code in `crates/ghostlight`; the
  binary crate `ghostlight-verse` drives it. Phase A runs are one-shot CLI
  invocations on Yggdrasil started by the operator or an agent at her word; the
  daemon, its Eve surface and the Idunn schedule arrive with phase D.
- **Inference lane.** The verse uses a hosted OpenAI-compatible lane, DeepSeek
  first (ruling `two-ghostlight-consumers`). The lane is main's local
  OpenAI-compatible port with a second binding (HTTPS base URL, bearer key read from
  a file), not a new transport. The DeepSeek key is the operator's to provision.
- **Cadence** (ruling `verse-cadence-ruled`): manual runs under a hard token cap per
  run, which the operator sets per invocation, until cost per tick is measured; then
  nightly under a monthly cap she sets.
- **Replay record.** Each provider call's request hash, response events, model and
  token usage are stored per run in a CultCache call record. The kernel draws no
  randomness, so stored responses are enough for replay (prior art (c)).
- **Exchange.** Aetheria reads what the verse publishes as CultNet documents through
  CultMesh, the outbound read `ghostlight-world-consumer-api.md` names as "not in
  this pass". The loopback consumer door is not how Aetheria reads.
- **Meddling** (ruling `meddling-first-ruled`) is an Eve/CultUI graph over CultMesh.
  Steering controls (pins, focal subjects, cell budget, cast roster, pause, resume)
  revise the run record at a tick boundary with an epoch and change no world state.
  Injection controls (fact, pressure, commitment, person, ruled fact) are kernel
  commands under the owner principal.
- **The newspaper** (ruling `verse-first-consumer-ruled`) ports as the first reader,
  bound by `readers-read`, with the copy desk checking citations against committed
  facts.

## Not in scope

- Dungeon affordances: one-step Begin, vault seeding under Begin, the create flow,
  Dungeon's play surface and Bonsai (ruling `aetheria-sim-first`). They continue in
  the Dungeon lane and do not gate this target.
- The ad hoc mutation rules of the pre-rebuild tree (map, "Ad hoc rule" table).
- The video and stream pipeline, episode editing, and the people-in-rooms scene
  grammar (ruling `scene-grammar-people-in-rooms`).
- Storylet weaving, radio delivery and any other runtime code in Aetheria. On
  Aetheria's side only the grammar document types, their validation and the AetherDb
  command that authors them are in scope.
- Counterfactual branching of a baseline over player-action archetypes. The journal
  must not preclude forking a world at a committed revision, but no fork operation is
  built here.
- A live Ghostlight running beside the game.
- The failed `idunn-odin-*` and `idunn-streampixels-*` units on Yggdrasil.

## Canonical implementations

- Kernel, cover, clock, grammar binding, import key and the verse runner library:
  `GameCult/Ghostlight` `crates/ghostlight`.
- The verse binary, later daemon and Eve surface: `GameCult/Ghostlight`
  `crates/ghostlight-verse`.
- The grammar: `GameCult/Aetheria`, `Assets/Scripts/ServerShared` and
  `GameData/Aetheria.cc`, authored through `tools/AetherDb`.
- The port source: `GameCult/Ghostlight` at `f9f019b`, `crates/ghostlight-dungeon/src/`.
- Deployment and, from phase D, run start: `GameCult/Idunn`, with the recipe in the
  Ghostlight repo.
