# Ghostlight Fresh Workspace Handoff

Updated: 2026-09-23

This is the compact re-entry packet. It carries current authority and the next
gate. Git owns chronology, the system map owns teardown detail, and evidence
records own durable findings.

## Immediate Re-entry Instruction

1. Work from `F:\Projects\Ghostlight`.
2. Run `git status --short`, `git log -1 --oneline`, and
   `npm run state:status`.
3. Read `state/map.yaml` (`current_status` first), this handoff, and
   `docs/architecture/ghostlight-dungeon-mvp.md`.
4. Read `notes/ghostlight-current-system-map.md` only when exact pre-rebuild
   ownership or deletion evidence is needed.
5. For deployment, SSH, Idunn, Odin, Heimdall, or host work, consult
   `F:\Projects\gamecult-ops` before acting.

If the operator asks only to rehydrate, report the current gate and stop. A
persisted next action is not permission to begin edits.

Do not continue implementation automatically from a rehydrate-only request.
Do not trust this file for the exact live HEAD. Git and live runtime witnesses
own volatile identity.

## Current authority

The operator stopped Run 115 and ordered a whole-machine authority rebuild.
The adopted target is `docs/architecture/ghostlight-dungeon-mvp.md`.

One per-world `WorldKernel` owns one revisioned `WorldState` across Draft and
Active phases. It alone derives authority, applies the typed
ontology reducer, and commits an atomic CultCache revision. World creation,
autonomous turns, player commands, elaboration, consumer patches, time
advance, and administrative changes all enter through the same
`CommandEnvelope` path; the play agent (plan step 16) holds a named author
capability in that same kernel and has no separate owner.

The replacement owner is sealed. Its runtime boundary remains create/open,
immutable snapshot, submit, and typed receipts. Mutable aggregate state,
canonical ID issuance, the reducer, and the CultCache journal writer stay
private to the kernel. Controller assignment and affordance grants are
canonical; current executability and decision opportunities are derived and
revalidated by the kernel. The landed `foundation.v0` loop binds opportunities
to a revision; ontology v2 replaces that with scope-digest binding, so a
proposal whose scope digest is unchanged commits at a later revision and one
whose digest changed is rejected. Revision binding is current source, not the
adopted rule.

Autonomous cognition has two explicit modes under disjoint controller scopes:

- `NarrativePersona` receives narrative projection and replies only in prose.
  The runner records that prose as receipt-bound noncanonical source evidence;
  a total Interpreter emits every faithful typed proposal it can lower with an
  exact source capture and records exact translation gaps for the rest.
- `OperationalAgent` receives a permissioned typed view and tools when
  operator-shaped cognition benefits from direct state work.
- Mode changes representation, never authority. A decision opportunity cannot
  fall through between modes or act twice.

The Interpreter cannot fail semantically. Missing referents, ambiguous intent,
missing affordances, and missing command primitives become translation gaps.
Malformed capture spans, raw tool decode failure, and normal step exhaustion
fall back to an exact whole-source gap rather than losing meaning. An
infrastructure interruption discards partial captures and returns the immutable
source pending for a fresh attempt. Source prose is not speech: only actually
spoken words may become a typed speech proposal. Only the world kernel may
accept or reject a proposed mutation. Translation gaps remain non-fictional
inference/evaluation evidence outside `WorldState`; recording one cannot itself
request elaboration or advance a world revision.

World validity and world liveness are two different claims, and both are
true. Structural validity is sparse and causal: it never waits on a count, and
no count, cover ratio, interestingness, diversity, or prose-quality judgment
can admit ontology, reject a structurally valid mutation, or declare a world
complete. Liveness is authored: the seed carries a `WorldScaleIntent`, the
elaborators pursue its derived per-jurisdiction scale deficit as a work queue,
and the cover budget is the deliberate choke that makes attention scarce.

Operator directive, 2026-09-04, preserve verbatim in intent: the 2,400-subject
/ 240-cell / 10% cover profile is deliberate design, not a load fixture. A
world feels alive only when actors at every level pursue their own goals; that
detail is generated first, then multiresolution simulation is choked so
attention is scarce and selection is forced. The mvp doc's teardown-era
sentence calling it "test data, not a production elaboration target" was
wrong and is amended. Do not re-cut the scale target as fixture noise.

## Landed

Git owns the commit chronology; `git log` on `codex/ghostlight-dungeon-mvp`
is the witness. What is true of the tree now:

- The replacement mailbox/kernel is the crate and executable runtime
  identity; no legacy Session Zero, kernel, scheduler, assessor,
  verifier/reconciliation, or transition module path survives, and no
  compatibility writer was kept.
- One CultCache implementation, consumed from CultLib `85f7024`, owns app
  sessions, the world journal, and controller custody.
- The Persona interpretation contract is structurally total and live in the
  single `ControllerRunner` in
  `crates/ghostlight/src/controllers.rs`.
- The Idunn provider side is complete: the runtime validates Expected and
  activation, publishes signed Warming until the write lease arrives, and
  exposes managed route presence. Production has not cut over.
- Plan steps 6 and 8 through 14 are landed with their follow-ups: the ten
  ontology passes, the seed producer, interruption, witness, and the Claude
  SDK inference port. `notes/ghostlight-implementation-plan.md` carries each
  step's landed state; the ontology doc carries the mechanism.

Do not start another world acceptance run while Yggdrasil still serves the
legacy executable and state layout.

## Immutable failure evidence

Run 115 is terminal failed at
`/var/lib/gamecult/ghostlight-dungeon/acceptance/full-world-delvehold-0a83034-115`:
semantic revision 2, regions 2/8, waves 0/1. Resume2 ended on provider SSE
timeout. Resume3 exhausted four reconciliation steps after
`inst:kharad-road-keepers` referenced unknown `loc:kharad-rhythm-road`.
Invocation `d9254cd9b77946aebcf7a7fdae821402` is terminal. Preserve the root and
receipts unchanged; its `status.json` is stale derived telemetry.

Runs 108 through 115 are evidence about load and failure properties of the old
body. Those runs were load fixtures; the 2,400-subject / 240-cell profile they
exercised is the authored design target, not fixture noise (see the operator
directive under Current authority). The 1,200-actor fixture remains a load
measure only.

## Operational boundary

Yggdrasil still serves legacy Ghostlight release `a4080d4` from an enabled
`Restart=always` unit with health v1, campaign/Session Zero state, and no
`world.cc` or `app-sessions-v2.cc` witness. The Connector is also still on its
legacy enabled body. CultNet through CultLib `85f7024` owns generation-bound
activation, separate lifecycle brakes, process-write leases, observed
capabilities, explicit disagreement, and routed RUDP incarnations. Odin through
pushed `65cf2b2` owns deterministic recipe and binding admission, exact source
freezing, sealed releases, durable deployment transactions, and the narrow
native actuator ports. Current uncommitted Odin integration moves Idunn-owned
projection and multi-session RUDP transport into those live paths.

Ghostlight pushed `6a79cb0` with the provider side of this cut. Its recipe
requests Idunn's runtime bundle, candidate bind, protected activation and
provider credentials, process-write-lease path, and admitted state-root
binding. The runtime validates Expected and activation, publishes signed
Warming, waits for the exact process lease before opening state, keeps that
lease current through bind, Active publication, and serving, republishes Warming
until the lease arrives, and exposes its managed route presence. CodexConnector
pushed `ede3c30` with the corresponding stateless provider contract. Neither
service has been admitted through rebuilt Idunn yet.

The adopted cut puts a visible recipe with a constrained launch declaration in
each target repo; raw unit or container-runtime templates are rejected. The
operator binding selects the workload driver, which alone lowers that launch
declaration into process-manager configuration.
Idunn operator bindings own the admitted ref, runner/container image and
affordances, host paths, secrets, routing endpoint, rollout, retention, and
desired replica placement. A sealed plan and release publish only Expected. A
service owns its signed runtime presence and health claim. Odin alone
authenticates that observation into Present and derives Ready from exact
Expected/Present agreement; the three states and any disagreement remain
distinct. For writable state, Idunn grants the process-bound lease before state
opens. Stable route membership moves only after Ready. Deployment and
continuity remain separate authorities; a deployment brake cannot suspend
restart of the already-admitted body.

The current Nginx driver observes rendered configuration and `nginx -T`
visibility, not live-worker adoption or packet delivery. A route is not
acceptance-complete, and the incumbent may not be drained, until an independent
data-plane probe binds the exact candidate runtime and membership digest.

Idunn is the GameCult-wide deployment, admission, continuity, and future
swarm-scaling control plane. Systemd, container runtimes, and existing proxies
remain its replaceable actuators; it does not reimplement generic scheduling,
networking, container, service-mesh, cryptographic, or consensus machinery.
Odin owns the discoverable semantic topology; services own their actual signed
capability, health, capacity, and runtime claims.

Idunn starts and recovers from its own durable admitted state; Odin is its first
managed daemon and the semantic graph root, not an Idunn bootstrap dependency.
First-Odin admission still publishes Expected and requires signed Present and
Ready; only its evidence transport is bootstrapped by querying that exact
candidate directly. Idunn publishes desired topology before dependent
promotion. During an Odin outage, it may authenticate private physical evidence
to replace only the process incarnation inside an existing admitted generation
and preserve the current route. It freezes graph-changing deployment,
promotion, scaling, and provider selection; historical Ready cannot authorize
the replacement process, and Idunn never emits Present or Ready.

Heimdall owns account identity. Eve owns command invocation and lowering.
VoidBot owns Vault retrieval and evidence. Idunn owns deployment and daemon
continuity. Odin owns discovery. None owns world mutation. External consumers,
including Delvehold and Epiphany, own their state; Ghostlight may publish views
and proposals but cannot commit on their behalf.

## Next gate

The adopted vocabulary is `docs/architecture/ghostlight-world-ontology.md`;
its "Current mechanism" section describes the kernel as it stands.
`ghostlight-transition-algebra.md` and `ghostlight-multiresolution-agency.md`
are teardown evidence.

### The kernel now

Thirteen decision-constraining component kinds over a pinned operation catalog
under one `WorldPatch`, one reducer, one CAS commit, one decode bound for both
authoring lanes, and three confined `SystemCapability` arms: `Clock`,
`Elaborator { jurisdiction }`, `Consumer { consumer }`. Proposals bind to a
scope digest, and the kernel is the sole `ScopeChanged` detector: a bound
narrative turn whose scope moved is re-lowered once by the Interpreter with
the typed delta, never lost and never checked lane-side. `Witness` fans a
fact out over a place subtree through the one reach owner. External
consumers enter through `POST /cultnet/world-patch` under
`PatchGround::Consumer`; the live contract is
`docs/architecture/ghostlight-world-consumer-api.md`. The seed lane admits a
Draft-only owner patch from a local Vault and pursues the derived scale
deficit. Inference runs through one `InferencePort::prepare`, routed by model name
to three transports: the CodexConnector (not an option for Dungeon, operator
2026-09-23), the Node sidecar on the Claude Agent SDK for `claude`-prefixed
models, and the local OpenAI-compatible port (see Constraints). The Rust
decoder is the one validator on each; the sidecar hands arguments through unvalidated. The sidecar and
its receipt shape are a named stopgap, deleted whole when a Messages-API port
has budget. The SDK transport was proven on the road before the play agent's
Cut 1 (`d69e9d4`) deleted the tick driver those runs exercised: seeded runs on
`claude-sonnet-5` committed ticks end to end (evidence ledger; the runbook
that recorded them, `notes/local-live-smoke.md`, no longer carries that
narrative). A kernel refusal continues the authoring
conversation (`Refusal` rows, `controller_work.v16`); the Interpreter cites
prose by quote (`source_quote`), and a quote not in the prose is a gap. Plan
step 12 is landed: `PersonaMaterial` is the thirteenth component, a promise
carries its statement, the Projector sees who else stands in the room, and
on the road (run 8) six seeded people speak in six voices about a debt with
words in it. Plan step 14 is landed (commit `0582321`): a visible act is
quoted by the Interpreter as the visible clause alone and lands as
`Seen { by }` knowledge on the co-located; the actor's intent stays
private. Knowledge sources are `Witnessed`, `Told`, `Seen`, `Evidenced`.
Run 10 passed (18 displays) and produced the first live interrupted cell.
Each interrupted cell is now logged per cell: the harness prints
`interrupted cell subject=<label> bound=<digest> renewed=<digest>` after
each tick line from `CoverSummary.interrupted` (commit `81a1f53`); run 11
printed three, matching three re-lowerings in the trace.
State and commit schemas are `.consumer.v5`; world creation is
`world_create.v4`, which requires lens weights.

Deferred by design: `PolityInCausalRange`, `IndividuationRequired`, and
Verification 13 wait for relations and population slices. The ignored
acceptance test `real_local_model_cognition_modes_commit_speech` in
`world/controllers.rs` now places its three subjects in one Place
(`roll-hall`), fixing the prior `NoAudience` rejection.

### Constraints

- A local OpenAI-compatible transport exists in source since the play agent's
  Cut 2: `crates/ghostlight/src/local_inference.rs`, one
  `POST /v1/chat/completions` to a loopback endpoint, no credential, inert
  tool calls. It opens only when `GHOSTLIGHT_LOCAL_ENDPOINT` is set
  (`runtime.rs`), and claims models by the `GHOSTLIGHT_LOCAL_MODEL_PREFIX`
  prefix, default `local/`. No model has been run behind it yet.
- Topology (operator ruling 2026-09-23, "put each of the organs on the right
  machine"): the playtest's Dungeon runs on Yggdrasil beside Heimdall,
  deployed by Idunn through the `ghostlight` binding (Dungeon unit only) at
  ref `codex/ghostlight-dungeon-mvp`. Bonsai 2 stays on Raven's GPU and is
  reached at Yggdrasil loopback `127.0.0.1:18080` through a restricted-key
  reverse SSH tunnel over the WireGuard mesh, kept up by a Raven scheduled
  task. Local inference and Heimdall's private plane stay loopback-only.
  The Raven-hosted Dungeon (task `GhostlightDungeon`, `C:\Meta\ghostlight`,
  `deployment/raven/start-ghostlight-raven.ps1`) is a detour being retired.
  `gamecult-ops` owns the actuator, runbook and link; its
  `scripts/deploy-ghostlight-yggdrasil.sh` and
  `runbooks/ghostlight-dungeon-yggdrasil.md` are still bound to
  CodexConnector, and cutting that binding for the local link is the next
  action.
- CodexConnector is not an option for Dungeon inference (operator,
  2026-09-23). It is a deliberately isolated Codex fork so that Epiphany
  stops compiling Codex; nothing goes into it, and its `main` stays at
  `6519289`. Codex remains lapsed: no subscription, no API budget; the
  smoke substrate at `F:\Projects\Ghostlight-smoke` stays.
- The operator's Claude subscription, inherited by the SDK sidecar from the
  ambient Claude Code login on the workstation, is the only provider proven
  live. Its role on Yggdrasil is undecided; deleting it is decided after the
  gate.
- Ghostlight never reads, copies, forwards, or logs a credential. The
  connector's Codex home is the operator's real `~/.codex`; the SDK sidecar
  inherits the operator's Claude Code login from the ambient environment.

### In order

1. The playtest gate below (operator order 2026-09-15; redirected to the
   play agent 2026-09-22).
2. The outbound half of the consumer contract: the response batch with a
   non-loopback CultMesh lease.
3. The deployment gate below.
4. Integration of the elaborator swarm's ideas (see "World fixtures and
   the elaborator swarm" below).

### Playtest gate

The gate is plan step 16, the play agent
(`docs/architecture/ghostlight-play-agent.md`, section "Playtest gate"):
a human on a local daemon, every play lane on a local model, creates and
seeds a world from a Vault, activates it, plays, and restarts mid-session with
the world intact. The target owns the pass list; its cut map is
`docs/architecture/ghostlight-play-agent-cut.md`, which owns each cut's status,
findings PA.f1 onward, and every ruling.

Cuts 1 through 14 are landed or landing on `codex/ghostlight-dungeon-mvp`. What
the pass changed, in one line each: the autonomous drivers and owner controls
are gone from Dungeon (Cut 1); a local inference port exists (Cut 2); the kernel
gained a Play authority, `Ruled` facts, minting, retirement and affordance
grants (Cuts 3 to 5); the Persona lane became callable (Cuts 6, 7); one play
table owns the turn lifecycle (Cut 8); the Eve play surface replaced
`world.speak` (Cut 9); and the client round trip was exercised for the first
time through a real lowering, which is where the answer path, the question
token and the store migrations came from (Cuts 10 to 14). Soul's verdict after
Cuts 12 and 13 is ready to play with one preflight. The unverified leg is
seeding: `GHOSTLIGHT_SEED_VAULT_ROOT` has not been run end to end and appears
in no runbook.

Operator rulings, 2026-09-22. One Dungeon-owned operational agent dispatches
and interprets Persona turns, prompts the player, and negotiates results. It
replaces the Interpreter and the cover/cell/catalog/owner-click orchestration
on Dungeon's play path; the library keeps them for offline simulation. The
Projector -> Persona passes are unchanged and a Persona never sees structured
state. Rejected: Personas reading their own state, which is the
structured-state dump the projector/interpreter sandwich exists to prevent.
The agent holds its own named kernel authority, like the clock capability; an
actor's act commits only by exercising that actor's affordance.

Deferred by ruling, not dropped (the target's "Deferred, by ruling"): L2-L4,
Session Zero D1-D3 (plan step 15, `docs/architecture/ghostlight-session-zero.md`),
and L1 follow-ups f16 (configurable lens sets; operator: yes), f1, f13, f14.

Dungeon is a consumer of the Ghostlight library. The world kernel is
the `ghostlight` library crate and Dungeon consumes it across a public
boundary sealed by admission: the kernel rejects any ID, digest or
opportunity it did not issue. L0 is closed (`108b691..ab95d78`); its
follow-ups are listed in
`docs/architecture/ghostlight-library-extraction-postmortem.md`. L1, the
stock lens set, is implemented (`835ea4d..872ba35`; cut map
`docs/architecture/ghostlight-stock-lenses-cut.md`): lens weights are world
data the owner replaces with `SetLensWeights`, each elaborator session records
the lens it drew and that lens's instruction text, and the library runs a
caller's elaboration sessions concurrently under that caller's own ceiling.
Dungeon runs no Active elaboration since the play agent's Cut 1 (`d69e9d4`)
deleted its sweep and ceiling; nothing in Dungeon calls this path today.

The human play path, as source stands after Cuts 1 through 14:

- `world.play` is the only human play action (`runtime.rs::execute_world`,
  schema `ghostlight.world_play.v0`). `world.speak` and the story card were
  deleted by Cut 9 (`f1732c0`); the only surviving mention is a negative
  assertion in `eve.rs`'s tests.
- The Eve play card (`eve.rs`) is owner-gated and carries narration, the open
  question, and the refusal of the player's own act, plus one free-text
  control. The open question's identity rides the play button's own
  `props.action` as an opaque token (Cut 13, PA.f170), never as an editable
  visible field.
- Active elaboration runs with `NullEvidenceSource` (`controllers.rs`), so no
  Vault reaches it. This is L2's subject and is deferred with L2; the play
  path stops elaborating while Active.
- Dungeon runs no tick driver since the play agent's Cut 1 (`d69e9d4`)
  deleted it; when cells ran, they ran in sequence at 15–50 s each. Cut 8
  owns Persona dispatch.

The eight titles ship as the library's stock lens set. Their 2026-09 removal
had no operator decision; the correction record in `state/evidence.jsonl`
carries the history.

### Decisions the operator owns

- Whether a refused coupled constituent may re-submit once inside the same
  tick, decided with the submitted-versus-committed number in hand.
- The authority for an owner-only Eve `world.run_tick` command, not built.
- Routed to the Eve owner: whether the command-result vocabulary needs a
  fourth state for an overtaken turn instead of rendering an interruption as
  `denied`.

### World fixtures and the elaborator swarm

First-generation world fixtures, one per world, each with Ink, training
sidecar, visual plan, lore grounding, BFL manifest and a RUN note, are
reviewer-accepted except visual replay, which waits on a scene-set blockout:
Delvehold `cistern-house-nine-breaker-test`, Aetheria
`navigator-berth-hearing-v0`, Zyphos `eclipse-nursery-handover`, Kalsa
`stormshield-handoff-v0`. They live under `examples/ink/<world>/`,
`examples/lore-grounding/<world>/`, `examples/visual/<world>/` and
`prompts/image-generation/<world>/`.

The elaborator swarm is drained and a Ghostlight kernel session does not own
it. Its review record and the per-pass idea ledgers are in
`experiments/elaboration/`; start at that README, which carries the approved,
demoted and pending decisions. The proposed vault additions are in local
bundles named there, not on GitHub branches. Integrating ideas into the lore
vaults is still wanted and deliberately deferred until the operator has time to
review them; it is an editorial pass, one idea at a time, never a scripted
merge. The retired first-generation loop under
`C:\Users\Meta\.claude\worlds\retired` must not be launched. Image rendering
is deferred: workers emit imagegen-ready prompts only, and
`scripts/generate_bfl_images.py` has no working default key path.

### Deployment gate

Deferred behind the orders above, not cancelled:

Review and subtract the uncommitted Idunn-owned Expected/activation/anchor/lease
projection and bounded multi-session RUDP document transport. Repoint Odin off
its vendored CultNet fork onto CultLib `85f7024` as Ghostlight did, add UDP
route actuation plus an independently persisted post-reload data-plane
observation through the existing proxy, and verify that focused body on
Yggdrasil. Then implement the directly managed Rust Odin that
alone authenticates runtime observation into Present and derives Ready, plus
exact capability dependency closure frozen inside the existing deployment
transaction. Do not add another scheduler, registry, inbox, or shared-file
correlation owner.

Install rebuilt Idunn and admit Odin first through the same
Expected/Present/Ready contract, using its exact candidate only to bootstrap the
evidence transport. Then admit Ghostlight Dungeon; CodexConnector is not its dependency. Same-generation
continuity, route preservation, split-brain fencing, signed health, Odin-outage
freeze, and negative legacy-authority checks must all agree before purge.

Deploy Ghostlight Dungeon through that contract. Runtime restart,
route continuity, signed health, exact receipts, exclusive world-v3 state, and
negative checks must agree before deleting old services, releases, state,
acceptance roots, and local run scaffolding.

## Essential references

- Target authority: `docs/architecture/ghostlight-dungeon-mvp.md`
- Vocabulary and current mechanism: `docs/architecture/ghostlight-world-ontology.md`
- Consumer contract: `docs/architecture/ghostlight-world-consumer-api.md`
- Frozen teardown map: `notes/ghostlight-current-system-map.md`
- Current implementation plan: `notes/ghostlight-implementation-plan.md`
- Connector and Claude SDK sidecar bring-up (no live smoke exists):
  `notes/local-live-smoke.md`
- Human-readable state: `state/map.yaml`
- Distilled evidence: `state/evidence.jsonl` (full history in
  `state/evidence.archive.jsonl`)
- Machine-managed state: `state/ghostlight-state.cultcache.jsonl`
- Interface authority: `docs/architecture/ghostlight-eve-native-interface.md`
- Operations: `F:\Projects\gamecult-ops`
- Faculty-workflow lessons:
  `F:\Projects\Epiphany\notes\faculty-workflow-lessons-2026-09-04.md`

## Re-entry warnings

- A prompt, transcript, model receipt, scheduler item, browser surface, actor
  count, or derived simulation cover is not canonical world state.
- Do not preserve an old writer through a compatibility path.
- Do not let a translation gap become invented state or an Interpreter error.
- Do not publish mutable world types, an ID issuer, a reducer entry point, or a
  generic journal handle for the convenience of tests or adapters.
- Do not append a commit when reduction produces no canonical mutation.
- Do not resume Run 115.
- A consumer never restates a foreign owner's document shape as its own
  strict struct or fixture. Read the owner's published contract (pinned
  schema, owner-emitted samples) and prove each direction against the real
  counterpart. PA.f196 and PA.f197 in the play-agent cut map are the cost.
- Do not invoke the current bounded Connector or Ghostlight redeploy helpers;
  they fail at the root/Idunn Git boundary and still embody duplicate target
  deployment authority.
- Do not prove world-v3 purity with a short deny list. Archive the complete old
  state root or validate a complete allowed-path contract.
- Keep this handoff compact. Move chronology to Git, evidence, or the frozen
  system map.
- `docs/architecture/ghostlight-transition-algebra.md` is teardown evidence,
  not vocabulary authority.
- Integration gating: merge, confirm the tip by SHA, then delete the branch;
  never chain `merge --ff-only` with `branch -D`. Push only after reading the
  test result line, never on the launch.
