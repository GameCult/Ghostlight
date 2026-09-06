# Ghostlight Fresh Workspace Handoff

Updated: 2026-09-06

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
`CommandEnvelope` path; there is no Session Zero.

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
  `crates/ghostlight-dungeon/src/world/controllers.rs`.
- The Idunn provider side is complete: the runtime validates Expected and
  activation, publishes signed Warming until the write lease arrives, and
  exposes managed route presence. Production has not cut over.
- Plan steps 6 and 8 through 11 are landed with their follow-ups: the ten
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

Twelve decision-constraining component kinds over a pinned operation catalog
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
deficit. Inference runs through one `InferencePort::prepare` with two
transports: the CodexConnector and, per `claude`-prefixed lane model, the
Node sidecar on the Claude Agent SDK. The Rust decoder is the one validator
on both; the sidecar hands arguments through unvalidated. The sidecar and
its receipt shape are a named stopgap, deleted whole when a Messages-API port
has budget.

Deferred by design: `PolityInCausalRange`, `IndividuationRequired`, and
Verification 13 wait for relations and population slices. Stale: the
pre-pass-6 ignored acceptance test
`real_codex_connector_cognition_modes_commit_speech` in
`world/controllers.rs` fails at `NoAudience` because its subjects are
unplaced; retire it or place its subjects.

### Constraints

- No Codex subscription and no API budget. The Codex connector is stopped;
  the smoke substrate at `F:\Projects\Ghostlight-smoke` stays.
- CodexConnector is a deliberately isolated Codex fork so that Epiphany
  stops compiling Codex; nothing goes into it. Its `main` at `6519289` is
  the line Ghostlight runs.
- Ghostlight never reads, copies, forwards, or logs a credential. The
  connector's Codex home is the operator's real `~/.codex`; the SDK sidecar
  inherits the operator's Claude Code login from the ambient environment.

### In order

1. The operator installs the Claude Code CLI and logs in per the "Claude
   SDK route" in `notes/local-live-smoke.md`. Then the first SDK-backed road
   run, which is the belief-changing record still owed, and the first live
   interrupted cell (the runbook's "Interrupted cell" section names what the
   log must show).
2. The outbound half of the consumer contract: the response batch with a
   non-loopback CultMesh lease.
3. The deployment gate below.
4. Integration of the elaborator swarm's ideas (item 2 below).

### Decisions the operator owns

- Whether a refused coupled constituent may re-submit once inside the same
  tick, decided with the submitted-versus-committed number in hand.
- The authority for an owner-only Eve `world.run_tick` command, not built.
- Routed to the Eve owner: whether the command-result vocabulary needs a
  fourth state for an overtaken turn instead of rendering an interruption as
  `denied`.

### World fixtures and the elaborator swarm

First-generation world fixtures are banked and pushed, one per world, each
with Ink, training sidecar, visual plan, lore grounding, and BFL manifest,
reviewer-accepted except visual replay, which waits on a scene-set blockout:
Delvehold `cistern-house-nine-breaker-test` on `codex/world-delvehold`;
Aetheria `navigator-berth-hearing-v0` on `codex/world-aetheria`; Zyphos
`eclipse-nursery-handover` on `codex/world-zyphos`; Kalsa
`stormshield-handoff-v0` on `codex/world-kalsa`; lore on each vault's
`codex/ghostlight-worlds` branch.

The elaborator swarm is drained and handed off; a Ghostlight kernel session
does not own it. The successor handoff is
`C:\Users\Meta\.claude\worlds\README.md` (98 finished passes). Owed and
deliberately deferred: integration of the `slot/<world>/<title>/<stamp>`
branches into the lore vaults on `codex/ghostlight-worlds` and into
`codex/world-<world>`; the idea index is the four ledger directories
`experiments/elaboration/<world>/ledger/` on those world branches. Clones:
`F:\Projects\Ghostlight-worlds\<world>` and
`F:\Projects\<Lore>-worktrees\ghostlight-worlds` (full clones despite the
name). The retired first-generation loop under
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
evidence transport. Then admit CodexConnector and Ghostlight. Same-generation
continuity, route preservation, split-brain fencing, signed health, Odin-outage
freeze, and negative legacy-authority checks must all agree before purge.

Deploy the Connector and then Ghostlight through that contract. Runtime restart,
route continuity, signed health, exact receipts, exclusive world-v3 state, and
negative checks must agree before deleting old services, releases, state,
acceptance roots, and local run scaffolding.

## Essential references

- Target authority: `docs/architecture/ghostlight-dungeon-mvp.md`
- Vocabulary and current mechanism: `docs/architecture/ghostlight-world-ontology.md`
- Consumer contract: `docs/architecture/ghostlight-world-consumer-api.md`
- Frozen teardown map: `notes/ghostlight-current-system-map.md`
- Current implementation plan: `notes/ghostlight-implementation-plan.md`
- Local live smoke runbook: `notes/local-live-smoke.md`
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
