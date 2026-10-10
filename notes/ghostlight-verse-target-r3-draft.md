# Ghostlight Verse: Target, revision 3 (draft)

Draft, 2026-10-10. Imagination `imagination-gl-verse-sim-0b`, session
`self-2026-10-10-ag`. Not admitted: Self admits revision 3 after the operator
rules on the questions named below. Supersedes `notes/ghostlight-verse-target.md`
(revision 2) where they differ. Evidence: the port inventory and model page in
`notes/ghostlight-verse-map.md` ("Verse sim: port inventory and model page");
`F:\Projects\gamecult-ops` `docs/research/llm-world-sim-prior-art.md` at
`6b4a874` (cited below as *prior art (a)-(d)*); `F:\Projects\aetheria-story-sifting-prior-art.md`.

## Why

The operator's words govern (ruling `verse-sim-pipeline`): "we are running
Ghostlight with the Aetheria lore, then elaborating it, then running the world
with multi resolution gestalt. Aetheria provides an ontology and action grammar
that matches what we can represent in the game, so that we can easily visualize
and integrate the stories it generates. That narrative sim would run on a
schedule feeding the content pipelines as we guide it with our filthy meddling
(we will want tools for said meddling)."

Ruling `aetheria-sim-first` makes this the campaign's priority, ahead of every
Dungeon affordance: "you build the API we need in Ghostlight and consume it from
Aetheria for world sim". The products are unchanged from revision 2 (ruling
`verse-cast-crossmedia`): storylets for the game, a serialized cast, and stories
a cheap cinematic pipeline can stream.

Two later rulings set how the machine is built. `gestalt-machinery-port`: the
pre-rebuild multiresolution gestalt machinery "was solid, it just was never
ported", so it is ported, not redesigned. `teardown-was-authority-sprawl`: the
September teardown cut "authority like a hydra", the ad hoc mutation rules, and
discarded good ideas with them. The port brings the good ideas back onto the
grammar-governed kernel and leaves the ad hoc rules dead. The map sorts the old
tree (`f9f019b`) into both lists and names sixteen forced changes (F1-F16).

The field agrees with that split. Every working LLM world simulation keeps world
state in a non-LLM store and maps free-text intent onto an enumerated, checked
action (prior art, "What the field converged on", items 1-2). Cost comes down by
calling the model less per agent per tick (item 3), which is what the gestalt
cover does. Replay comes from stored responses, not from seeds (item 5).

## Phases

1. **Honest and running** (revision 2, unchanged): the Dungeon lane closes its
   phase-1 work in parallel. It does not gate the verse.
2. **The kernel can hold the verse.** The relation edge (F6) and the import key
   enter the kernel. Aetheria's grammar is loaded as the world's affordance
   catalog, and the kernel refuses an affordance kind the grammar does not
   carry, along with any kind lacking a render path (`every-event-renderable`).
3. **The verse loaded.** Genesis from AetheriaLore and Aetheria's `Faction`
   records, with factions as institutions, populations as gestalts and named
   characters as persons. Then elaboration to the authored scale.
4. **The gestalt port and the run.** The cover, pins, debt, checkpointed
   parallel cells, strategic individuation and fission, all on main's kernel.
   A verse runner advances the world on a schedule within a budget, and every
   run can be replayed.
5. **Meddling and the first reader.** The operator's Eve surface for
   intervention, and the first pipeline consumer (question
   `verse-first-consumer`).

## Invariants

The ends, not the means. Labels carried from revision 2 keep their meaning.

- `honest-docs`: README, maps and handoffs describe the machine on `main`; history lives in history.
- `main-runs`: every deployed Ghostlight unit is built from `main` and stays up; Idunn owns its lifecycle.
- `kernel-decides`: the kernel owns world state through its closed operation set; models propose, the kernel admits or refuses, and no model decides an outcome.
- `grammar-governs`: every change to world state is a kernel command reduced through the closed operation set, and no writer exists beside it. That includes gestalt individuation, fission, away-time results and meddling. Changing simulation resolution (promotion, demotion, regrouping) is not a world change and writes nothing. The writers allowed after activation are listed by name (map model page): affordance invocations from Aetheria's grammar, clock motion, elaboration patches answering a derived boundary or deficit, and owner acts. Question `grammar-scope-of-mutation` decides whether the last two must also be grammar verbs.
- `resolution-not-identity`: the resolution at which a subject is simulated never changes who exists, what they hold or what they know. A person given up by a gestalt stays a person.
- `one-record-one-subject`: each imported lore or Aetheria record maps to exactly one subject or entity, and the mapping is checked when a patch is admitted.
- `verse-provenance`: every world fact loaded from the verse names its source page or record; a fact with no source is a proposal until reviewed.
- `canon-one-way`: simulation output never writes back into AetheriaLore or Aetheria's catalog without operator review.
- `readers-read`: a pipeline consumer (newspaper, episodes, storylets) reads committed history and writes only its own output.
- `cast-persists`: a character keeps identity, memory and commitments across runs, so episodes can follow them.
- `every-event-renderable`: every event the kernel admits maps to exactly one render path: ship action filmed in Aetheria, or a social verb staged as an Aetheria conversation. Summary captions are the only exception.
- `run-resumable`: an interrupted run resumes from its last checkpoint and commits nothing twice.
- `run-replayable`: a run re-executed from its recorded provider responses produces the same journal.
- `run-bounded`: a run never spends past its configured budget, and its fiction span, calls, tokens and cost are recorded.
- `dungeon-isolated`: a verse run cannot slow, stop or change Dungeon play.
- `meddling-attributed`: every operator intervention is a typed act, recorded with the operator as principal, through the same doors as every other writer.
- `typed-state`: world state, run records, the grammar and pipeline output are CultCache documents; exchange is CultNet; JSON only for schema publication and xenos boundaries.

## Defaults (stated, not forks)

- **The gestalt port follows the old design** at `f9f019b`, except where main's
  ontology forces a change; the map names each one (F1-F16). Ported tests carry
  the invariants listed there, not the old fixtures.
- **A relation edge enters the kernel** (F6): an `EdgeKind` beside `Route`, with
  operations to form and dissolve it. Its kinds are Membership, Command,
  Alliance, Rivalry, Trade, Coercion and Migration. This was revision 2's
  phase-2 relation work.
- **Separate verse process.** The verse runner and its Eve surface run as their
  own daemon (binary crate `ghostlight-verse`), not inside
  `ghostlight-dungeon`. The process is earned by `dungeon-isolated`: a different
  consumer, model, budget and failure domain (ruling `two-ghostlight-consumers`).
  The runner logic is library code in `crates/ghostlight`, beside `cover.rs` and
  `clock.rs`, so Dungeon can drive it later.
- **Idunn** deploys the verse daemon, keeps it alive and starts runs on the
  schedule. The runner owns what a run does, and the kernel owns time. Whether
  Idunn offers a schedule trigger today is unverified; if it does not, that
  belongs to Idunn as a follow-up, and no in-daemon timer stands in for it.
- **Inference lane.** The verse binds the provider-portable connector lane
  (CodexConnector's library transport, unbound in Dungeon) to the cheapest
  hosted model, DeepSeek first (ruling `two-ghostlight-consumers`). Models are
  set per role.
- **Replay record.** Each provider call's request hash, response body, model,
  token counts and cost are stored per run. The kernel draws no randomness
  (outcome bands come from a digest), so stored responses are enough for replay
  (prior art (c), "Determinism and replay").
- **Exchange.** Aetheria reads what the verse publishes (world snapshot, history,
  pipeline output) as CultNet documents through CultMesh. This is the outbound
  consumer read that `ghostlight-world-consumer-api.md` names as "not in this
  pass". The loopback, write-only consumer door is not how Aetheria reads.
- **Meddling is an operator interface**, so doctrine fixes its shape. The verse
  daemon publishes an Eve/CultUI composition graph over CultMesh, lowered to GUI
  for the operator and to TUI for agents. Each control is a typed CultNet
  command. A world-changing control becomes a kernel command under the owner
  principal (`meddling-attributed`, `grammar-governs`). A run control (budget,
  pins, focus, cadence, pause) becomes a revision of the run record at a tick
  boundary, with an epoch, which is the old `ResolutionControlReceipt` rule.
  There is no separate web dashboard and no tool that edits `.cc` files or the
  journal directly. What it does first is question `meddling-first-acts`.
- **The newspaper ports as a reader.** The old newsroom, editorial agenda,
  citations and copy desk become a pipeline consumer bound by `readers-read`.
  Whether it is the first consumer is question `verse-first-consumer`.

## Not in scope

- Dungeon affordances: one-step Begin, vault seeding under Begin, the create
  flow, Dungeon's play surface and Bonsai (ruling `aetheria-sim-first`). They
  continue in the Dungeon lane and do not gate this target.
- The ad hoc mutation rules of the pre-rebuild tree (map, "Ad hoc rule" table):
  the outcome resolver, semantic verifiers and quotas, the presence planner as
  a decider, arena knowledge union, clock-consequence binding, prompt-only
  prohibitions.
- The video and stream pipeline, episode editing, and the people-in-rooms scene
  grammar (ruling `scene-grammar-people-in-rooms`; a later campaign).
- Storylet weaving, radio delivery and any other runtime code in Aetheria (the
  `aetheria-release` campaign). Only the grammar document is authored on
  Aetheria's side, if question `verse-grammar-home` puts it there.
- Counterfactual branching of a baseline over player-action archetypes, the
  "branch ... and propagate" step of ruling `storylets-simulate-sift-lift-weave`.
  The journal must not preclude forking a world at a committed revision, but no
  fork operation is built here.
- A live Ghostlight running beside the game (fork 4 of Aetheria's
  `reactive-world-target.md`).
- The failed `idunn-odin-*` and `idunn-streampixels-*` units on Yggdrasil.

## Canonical implementations

- Kernel, cover, clock, relation edge, import key and the verse runner library:
  `GameCult/Ghostlight` `crates/ghostlight`.
- The verse daemon and its Eve surface: `GameCult/Ghostlight`
  `crates/ghostlight-verse`.
- The port source: `GameCult/Ghostlight` at `f9f019b`,
  `crates/ghostlight-dungeon/src/` (`domain.rs`, `resolution.rs`,
  `scheduler.rs`, `gestalt.rs`, `persona.rs`, `newspaper.rs`, the tick driver in
  `main.rs`).
- The grammar: where question `verse-grammar-home` puts it.
- Deployment and run start: `GameCult/Idunn`, with the recipe in the Ghostlight
  repo.

## Questions for the operator

Admitted to the mind in one batch; query them there. `verse-grammar-home`,
`grammar-scope-of-mutation`, `verse-first-consumer`, `verse-run-cadence-budget`,
`meddling-first-acts`, `verse-world-canonical-or-seeded`,
`gestalt-fission-approval`.
