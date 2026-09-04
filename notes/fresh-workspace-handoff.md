# Ghostlight Fresh Workspace Handoff

Updated: 2026-09-04

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
ontology reducer, and commits an atomic CultCache revision. Session Zero,
autonomous turns, player commands, imports, and administrative changes must
enter through the same `CommandEnvelope` path.

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

## Landed and in flight

- `4aebe17` adopts the target authority map.
- `64f50de` removes seven obsolete pipeline-smoke binaries and two live-fire
  orchestration scripts.
- `a144c33` establishes the two disjoint cognition modes in
  `ghostlight-persona-projection`.
- `d49b907` refines total Persona interpretation: source prose is receipt-bound
  noncanonical evidence, typed captures cite exact source spans, malformed spans,
  raw decode failure, and normal exhaustion receive whole-source gap fallback,
  infrastructure interruption returns the source pending, and only actual
  spoken words qualify for a speech proposal. This contract is live: the single
  `ControllerRunner` in `crates/ghostlight-dungeon/src/world/controllers.rs`
  carries it, and no legacy Persona runner survives in `crates/`.
- An uncommitted broad `world.rs` prototype was rejected and deleted. It
  exposed mutable state and ID issuance, admitted translation-gap evidence into
  canonical types, allowed no-op commits, and left affordance and opportunity
  authority undefined. None of it is current authority.
- `12ee9f4` deletes the dead legacy Elaboration and Mutation kernel ingresses
  and their component-write helpers: 686 lines removed without a replacement
  compatibility path.
- `66ed2ec` completes the sealed private `foundation.v0` authority loop. Draft
  creation, approval, activation, canonical controller assignments, affordance
  grants, revision-bound opportunities, and one shared human/autonomous action
  reducer all commit through one aggregate and journal. One mailbox serializes
  submission and snapshot access. Eighteen focused tests at that commit (43 in
  `world/*.rs` at `d87cad8`) prove lifecycle,
  controller and opportunity fail-closed behavior, restart, idempotency,
  one-owner, authentication, non-commit, mailbox cancellation, lost replies,
  immutable genesis, and journal forgery rejection.
- `6bb6869` makes the replacement mailbox/kernel architecture the crate and
  executable runtime identity and removes the legacy runtime owner.
- `13d5136` unifies app sessions, world journal, and controller custody under
  one vendored CultCache implementation and removes the duplicate legacy
  dependency. The committed daemon tree contains no old Session Zero, legacy
  kernel, scheduler, assessor, verifier/reconciliation, or legacy-transition
  module path.
- `9256648` exposes managed Ghostlight route presence, and `6a79cb0`
  republishes signed Warming for as long as the write lease is outstanding.
  With `a0b16b9` these are the whole provider side of the Idunn cut.
- Source subtraction is complete. Production cutover and runtime legacy purge
  are not.

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

Ontology v2 is the adopted vocabulary at `docs/architecture/ghostlight-world-ontology.md`
(twelve components, twenty-nine operations under one `WorldPatch`,
world-authored affordances with preconditions, effect slots, and kernel-entropy
outcome bands, four derived `CausalBoundary` kinds, scope-digest binding, one
`AdmitPatch` for seed and elaboration, eighteen proofs).
`ghostlight-transition-algebra.md` and `ghostlight-multiresolution-agency.md`
are teardown evidence. Plan step 6 is nine implementation passes; pass 1 is
landed, passes 2 through 9 are not.

1. No pass beyond 3 is integrated. `codex/ghostlight-dungeon-mvp` tip is
   `bec8004` (steering record) over `6a4f42a`; passes 1 through 3 landed at
   `5e53beb`, `e99af63`, `d2805fe`. Pass 3 gave the kernel subject-keyed
   `holdings` and `dependencies`, `Quantity(u64)` with absence meaning zero,
   checked arithmetic, ops `Transfer`, `Transform` (1:1, no unit/rate/recipe),
   `Consume`, `Admit` (same-patch evidence), `Bind`, `Release`,
   `DependencyTarget { Resource, Route, Subject }`, conservation at the single
   `patch::check_ledger` site, one `scope_components` owner for scope digest
   and snapshot, schemas `world_state.custody.v1` / `world_commit.custody.v1`.
   `EvidenceRef::new` is `#[cfg(test)]` because no production ingress authors
   a `WorldPatch`; the real gate is that nothing deserializes one. The action
   vocabulary is still `AffordanceKind::Speak` / `DecisionAction::Speak`.
   Baseline at `6a4f42a`: 98 test functions in `world/*.rs`, 137 crate
   source, plus one integration test.

   Pass 4 (affordance catalog, `action.rs`, `ActionMismatch`, kernel-entropy
   band draw, derived `OperationalAgent` tool catalog; deletes
   `DecisionAction`, `AffordanceKind`, and three hard-coded permission
   surfaces) is mid-cut with Hands in the locked worktree
   `.claude/worktrees/agent-a8828a054fe108277`: commit `c9fd9bf` "Extract
   apply_operations as the one owner of operation application and
   conservation" is landed there, and `action.rs` is being written with
   uncommitted edits in `eve.rs`, `runtime.rs`, `world/controllers.rs`,
   `world/journal.rs`, `world/mailbox.rs`. Soul verifies in that tree; the
   coordinator integrates. Witness: `git log` on the main branch past
   `bec8004`.

   Specs and maps live in the session scratchpad, not `state/`:
   `imagination-pass4.md` through `imagination-pass8.md` and
   `modeling-pass4.md` through `modeling-pass8.md` are on disk; Modeling pass
   9 is running. Adopted design decisions from those specs that change
   steering: pass 7 gives the world its first clock, `WorldState.now`,
   advanced only by `CommandBody::AdvanceTime` from
   `CallerId::System(SystemCapability::Clock)` minted by the mailbox, and cuts
   the operation set from 30 to 27 (`Commitment` stake/fulfill/default/release
   and `Pressure.create` removed; `Routine.period` required). Pressure is
   stored with two writer families, ops and the tick, never derived at read
   time. Ordering is a pure re-ordering by pressure then debt
   (`last_opportunity_at`), never a filter. Only `UnelaboratedDestination` and
   `MissingStructure` boundaries are derivable; `PolityInCausalRange` and
   `IndividuationRequired` wait for relations and population slices, and
   Verification 13 is explicitly deferred. Pass 8 replaces the pass-1 Active
   declaration ban with "declarations in Active only inside a patch whose
   `PatchAnswer` is currently derived and proven satisfied", adds
   `SystemCapability::Elaborator { jurisdiction }`, the in-process
   `ElaborationRunner`, and a derived elaborator tool catalog sharing one
   property emitter with pass 4. The typed CultNet consumer ingress is a newly
   named pass 10, "consumer ingress"; step 6 is ten passes, and the plan doc
   still lists nine.

   Queued doc edits, owned by the coordinator, to land with pass 7: four
   `ghostlight-dungeon-mvp.md` sentences naming subject readiness as owned
   state (lines 65, 153, 211, 433) and the ontology doc's "Twenty-nine
   operations" count (lines 222, 276, 537) to 27.
2. First-generation world fixtures are banked and pushed, one per world, each
   with Ink, training sidecar, visual plan, lore grounding, and BFL manifest,
   reviewer-accepted except visual replay, which waits on a scene-set
   blockout:
   - Delvehold `cistern-house-nine-breaker-test`: `codex/world-delvehold`
     tip `9374fd0`.
   - Aetheria `navigator-berth-hearing-v0`: `codex/world-aetheria` tip
     `fe679f4` (fixture commit `8860c47`); lore on AetheriaLore
     `codex/ghostlight-worlds` at `ba5c2ce`.
   - Zyphos `eclipse-nursery-handover`: `codex/world-zyphos` tip `055e6ee`;
     lore on Zyphos `codex/ghostlight-worlds` at `2fef909`.
   - Kalsa `stormshield-handoff-v0`: `codex/world-kalsa` tip `489c541`; lore
     on Kalsa `codex/ghostlight-worlds` at `c9f4c5c`.
   The first-generation `Invoke-Worlds.ps1` loop is retired (file still on
   disk, do not launch it). The live fan-out is the 32-slot titled-elaborator
   scheduler `C:\Users\Meta\.claude\worlds\Invoke-Elaborators.ps1`, config
   `worlds.config.psd1`, prompt `elaborator-prompt.md`. Since 18:17 the prompt
   is a creative brief: vault ideas, no Ink. Each slot publishes its ledger to
   the base branch under `experiments/elaboration/<world>/ledger/<stamp>-<title>.md`
   on `codex/world-<world>`. First two ideas: Aetheria Hearth "Keeping The
   Quarrel" (ledger commit `cead2d5` on `codex/world-aetheria`) and Delvehold
   Ember null-rune immune response (ledger commit `ad4fcec` on
   `codex/world-delvehold`). Workers never run git: the Codex workspace-write
   sandbox keeps `.git` read-only even inside the writable root. The scheduler
   commits and pushes each slot clone on exit to `slot/<world>/<title>/<stamp>`
   branches in both the Ghostlight world clone and the lore clone and records
   each completion in `logs\elab-done.csv`, 44 rows as of this
   writing; that file is the witness, not the scheduler log. Integration of
   slot branches into `codex/world-<world>` and `codex/ghostlight-worlds`
   beyond the published ledgers is a deliberate later pass.
   Clone paths:
   `F:\Projects\Ghostlight-worlds\<world>` and
   `F:\Projects\<Lore>-worktrees\ghostlight-worlds` (full clones despite the
   name). Image rendering is deferred: the BFL key drive E: is gone, workers
   emit imagegen-ready prompts only, and `scripts/generate_bfl_images.py` has
   no working default key path.

The deployment gate below is deferred behind both orders, not cancelled:

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
route continuity, signed health, exact receipts, exclusive world-v2 state, and
negative checks must agree before deleting old services, releases, state,
acceptance roots, and local run scaffolding.

## Essential references

- Target authority: `docs/architecture/ghostlight-dungeon-mvp.md`
- Frozen teardown map: `notes/ghostlight-current-system-map.md`
- Current implementation plan: `notes/ghostlight-implementation-plan.md`
- Human-readable state: `state/map.yaml`
- Distilled evidence: `state/evidence.jsonl`
- Machine-managed state: `state/ghostlight-state.cultcache.jsonl`
- Interface authority: `docs/architecture/ghostlight-eve-native-interface.md`
- Gestalt authority: `docs/architecture/ghostlight-multiresolution-agency.md`
- Operations: `F:\Projects\gamecult-ops`

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
- Do not prove world-v2 purity with a short deny list. Archive the complete old
  state root or validate a complete allowed-path contract.
- Keep this handoff compact. Move chronology to Git, evidence, or the frozen
  system map.
- `docs/architecture/ghostlight-world-consumer-api.md` is drift: it names
  `CampaignRegistry`, `WorldSeed`, and `publish_session_zero`, none of which
  exist in the sealed kernel. Pending rewrite against
  `ghostlight-world-ontology.md`; do not design from it.
- `docs/architecture/ghostlight-transition-algebra.md` is teardown evidence,
  not vocabulary authority.
