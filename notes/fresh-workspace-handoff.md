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

The adopted vocabulary is `docs/architecture/ghostlight-world-ontology.md`
(twelve components, twenty-seven operations under one `WorldPatch`,
world-authored affordances, derived `CausalBoundary` kinds, scope-digest
binding, one `AdmitPatch` for seed, elaboration, and consumer ingress).
`ghostlight-transition-algebra.md` and `ghostlight-multiresolution-agency.md`
are teardown evidence. Plan step 6 is ten implementation passes, all landed.

1. Plan step 6, the ontology widening, is complete: all ten passes are
   integrated on `codex/ghostlight-dungeon-mvp`. Tip is `a1fd645` (Soul)
   over `b4d1943` (pass 10 machine) and `e0c6f9c` (patch author widened to
   a confined external consumer), on `1203aa3` (pass-9 docs and handoff) and
   `962f711` (intra-tick quarantine race closed). Pass tips in order:
   `5e53beb`, `e99af63`, `d2805fe`, `0f21a49`, `dbe176d`, `cb6a126`,
   `1852ddd`, `3ed3868`, `8aebfb6`, `a1fd645`. The push follows a
   background crate test and the branch read `ahead 3` at the time of
   writing; `git status -sb` is the witness. The series is distilled as one
   evidence record dated 2026-09-05 in `state/evidence.jsonl`.

   What the kernel is now: twelve decision-constraining component kinds
   over twenty-seven named operations under one `WorldPatch`, one reducer,
   one CAS commit, one decode bound for both authoring lanes
   (`patch::decode_patch` with `MAX_PATCH_BYTES`, `MAX_PATCH_DECLARATIONS`,
   `MAX_PATCH_OPERATIONS`, `MAX_PATCH_EVIDENCE`; the `MAX_DRAFT_*` bounds
   are deleted), and three confined `SystemCapability` arms: `Clock`,
   `Elaborator { jurisdiction }`, `Consumer { consumer }`. Pass 10 added
   `world/consumer.rs`; `PatchGround { Jurisdiction, Consumer }` with
   `confine_to_ground` as the one confinement function (renamed from
   `confine_to_jurisdiction`); `ControllerAssignment::ExternallyControlled
   { consumer }`, with `id()` and `mode()` now `Option` and every reader
   failing closed; `Mismatch::ControllerGrantMismatch` replacing
   `NoAffordances`; `CommandId::derived` shared by `session_command_id`;
   `ConsumerPort` (one method) in `world/mailbox.rs` and
   `WorldMailbox::submit_consumer`; route `POST /cultnet/world-patch`,
   loopback, msgpack, `CONSUMER_BODY_LIMIT`; `ConsumerRegistry` loaded from
   the file named by `GHOSTLIGHT_CONSUMER_CREDENTIALS`, digests only.
   Schemas are `world_state.consumer.v1` / `world_commit.consumer.v1`;
   `controller_work.v9`; `STATE_SCHEMA_GENERATION` in
   `idunn_health.rs` is now `world-v3`, so the Idunn expectation and every
   "world-v2" phrase in the deployment gate below now mean world-v3.
   Baseline at `a1fd645`: 313 test functions in `world/*.rs`, 357 crate
   source, plus one integration test (the focused `world::` run reports
   312 / 352; different counting rules, compare like with like).

   Rulings adopted in pass 10: no migration for the moved
   `session_command_id` derivation; `controller_work` is v9 so the refusal
   is structural; a malformed credentials file is fatal at startup
   (`open_consumer_registry` in `runtime.rs` returns the registry's error;
   a missing file still means no consumers); the outbound consumer response
   batch and a non-loopback CultMesh lease are the next seams, not this
   pass. Soul's series judgment: the eleven plan invariants hold
   structurally at the end of ten passes; the pairing invariant is
   re-decided at three gates by doctrine, not by a repair loop.

   The pass-10 follow-ups are landed (`8173c9c`): `controller_work.v9`;
   fatal malformed credentials; redacting `Debug` on the consumer document;
   the elaboration lane calls `check_patch_caps` so `MAX_PATCH_EVIDENCE`
   binds on both lanes; distinct item-cap refusals in the receipt; one
   duplicate test deleted. Nothing is in flight and no worktree exists.
   Docs landed with them: `ghostlight-world-consumer-api.md` rewritten
   as a live contract (no longer drift); the ontology "Current mechanism"
   through pass 10 closing with "Step 6 of the plan is complete; the
   outbound consumer response is the next seam."; the mvp doc's
   `MutationAuthorityEnvelope` and external-mirror-as-subject-kind
   corrected; the plan's step-6 pass-10 line in past tense. Queued doc
   edits: none.

   First local live smoke, 2026-09-05, committed as `5e52943` and pushed:
   `notes/local-live-smoke.md` is the runbook. The production tick driver,
   cover, Persona membrane, operational lane, clock, and elaboration sweep
   ran three ticks against a real CodexConnector (head daemon with the
   `daemon` feature, ChatGPT auth) on a genesis world: revision 2 to 11, two
   singleton cells per tick, the Persona spoke each tick, the operational
   lane committed each tick, the elaboration sweep was clean, 40 to 70
   seconds per tick. The harness is the ignored test
   `live_smoke_ticks_a_genesis_world_against_the_connector` in `runtime.rs`
   (line 3177) with `fixture_with(Option<LiveController>)`; the evidence
   record is dated 2026-09-05T13:42Z. Two findings: the road works end to
   end; the prose is thin because genesis is three subjects in one room.
   That makes the seed producer the evidenced gap, not an argued one: no
   doc or steering surface named a "seed producer" before this smoke (the
   Delvehold boundary doc names only a seed admission primitive), so it is
   now the obvious next cut once the operator decisions below are taken.
   Stale: the pre-pass-6 ignored acceptance test
   `real_codex_connector_cognition_modes_commit_speech` in
   `world/controllers.rs` (line 6549) fails at `NoAudience` because its
   subjects are unplaced; retire it or place its subjects.

   Still deferred by design: `PolityInCausalRange`, `IndividuationRequired`,
   and Verification 13 wait for relations and population slices.

   The seed producer is landed and proven on the road. Tip is `b7042f1`
   (seed producer recorded, Soul follow-ups closed) over `491b06e` (the six
   seed-lane follow-ups) and `697ed07` (Soul), both pushed; no worktree
   exists. The seeded live smoke passed on the twelfth run, 2026-09-05:
   from the Kalsa Public vault (`GHOSTLIGHT_SEED_VAULT_ROOT` at
   `F:\Projects\Kalsa\Kalsa`, root label "Low Sere"), one seed session
   authored six qualified persons into Low Sere in one round (39 s),
   deficit 6 to 0; the world activated with nine subjects and six
   boundaries; three ticks of eight singleton cells each (95 to 123 s) with
   the seeded people speaking to each other by name. The run block and the
   "Seeded run, 2026-09-05" table are in `notes/local-live-smoke.md`; the
   evidence record is dated 2026-09-05T17:36Z; the ontology "Current
   mechanism" gained a "Rules the road imposed" paragraph (line 256).

   Uncommitted on main at the time of writing, to be committed once the
   unit suite is green: the road fixes in `runtime.rs`, `controllers.rs`,
   `elaboration.rs`, `patch.rs`, `tool_schema.rs` (strict-schema `anyOf`
   and typed const tags with an offline test over both lanes; connector
   `REQUEST_EXPIRY` 300 s and `RESPONSE_TIMEOUT` 900 s; content-addressed
   `provider_request_id`; `SEED_ROUND_BUDGET` 24 with budget-end submitting
   a non-empty draft; `SEED_INSTRUCTIONS` pacing and obligation rule; the
   brief prints ids and a placement rule; classified faults and cell
   outcomes logged at info; the live harness asserts a landed seed and
   speech, counts `NoProgress` as landed, continues through `Rejected`),
   plus the ontology paragraph, the smoke note, and the evidence record.
   Baseline at that tree: 342 test functions in `world/*.rs`, 394 crate
   source, plus one integration test. The connector is stopped after the
   run; the smoke substrate at `F:\Projects\Ghostlight-smoke` stays.

   Plan step 9, the interruption pass, is integrated. Tip is `711f6ec`
   (step-9 docs) over `d20dc51` (Soul), `ad124ea` and `1a19fb2` (Hands),
   `1ef973a` (machine), on `a62708f`. The push follows a background crate
   test and the branch read `ahead 5` at the time of writing; `git status
   -sb` is the witness. What landed: the narrative lane no longer checks
   scope itself; the kernel is the sole `ScopeChanged` detector and the
   lane handles the refusal at `controllers.rs:3537` by interrupting (the
   operational lane keeps its one `ensure_scope_unchanged` at :2689, and
   the check at :1916 is the checkpoint-progression rule, not a lane
   check). `SubjectSnapshot` carries `components: ScopeComponents` and lost
   `holdings`, `dependencies`, `incident_routes`, `authority`, `controls`.
   `PersonaTurnBinding::interrupted_from`; `Overheard` (:1014),
   `Interruption` (:1027), `interpreter_round` (:1043), `select_fresh`
   (:1095), `select_scope` (:1104), `NarrativeRun::Interrupted`;
   `controller_work.v11` and `ghostlight.persona_turn_receipt.v3`;
   `ControllerHttpResult::Interrupted` renders as Eve `denied` with the
   detail in the receipt (`runtime.rs:1111`). Baseline at `711f6ec`: 354
   test functions in `world/*.rs`, 406 crate source, 13 in
   `ghostlight-persona-projection`, plus one integration test (Soul ran
   353 / 400 / 13; the usual counting-rule gap).

   Soul verdict: the leak invariant holds structurally, fixed English per
   changed field, and a counterparty id inside the persisted components
   never reaches the prompt. Fork A corrected: a turn whose scope moved
   before binding is still `NoOpportunity` at zero cost; the cut affects a
   bound turn and costs at most one extra Interpreter round. Fork D is
   dead code, because grants are insert-only. Recipe lesson from the
   integration: the first fast-forward refused because main had moved by
   two doc commits, and the chained command deleted the branch pointer
   before that was noticed; the commits were recovered by SHA, rebased,
   and integrated. Never chain `merge --ff-only` with `branch -D` on one
   line; run the merge, confirm it, then delete.

   Plan step 10, the witness operation, is integrated. Tip is `1244595`
   (step-10 docs) over `70ea6b1` (Soul), `ecdeb92` (the operation),
   `307d8a5` (refactors: one owner for "under a place", one for "already a
   knower"), on `f6cd3b5`. The push follows a background crate test and the
   branch read `ahead 4` at the time of writing; `git status -sb` is the
   witness. Integration was gated this time: rebase, merge, confirm the
   tip, then remove the worktree. What landed: `under_place` is the one
   reach owner and `unheld` the one already-holder filter; `audience`,
   `fan_out`, and the `Witness` apply arm all call them. `ComponentOp::Witness
   { fact, place, confidence }` with resolve, apply, and lowering arms;
   `operation_ground` returns the place, so confinement treats it like any
   other ground and a consumer cannot witness by construction.
   `ComponentOpKind::Witness { confidence }`, so a world-authored
   affordance can cause a witnessed event with no speaker. Tool `witness`;
   the catalog is pinned at 7 declarations, 29 operations, 38 tools, and
   the ontology doc now says twenty-nine (lines 443, 498, 796), aligned to
   the pinned count after the docs writer found it one behind. Schemas are
   `world_state.consumer.v2` / `world_commit.consumer.v2`. An empty
   subtree is `NoOperationEffect` at both layers. Soul pinned two-layer
   agreement over draft subjects and forget-then-witness, and made the
   no-`Told` pin exhaustive over all nineteen operation kinds. Baseline at
   `1244595`: 372 test functions in `world/*.rs`, 424 crate source, 13 in
   `ghostlight-persona-projection`, plus one integration test (Soul ran
   371 / 418).

   The step-9 follow-ups are integrated at `651bfb2` over `5cd8853`,
   gated (rebase, merge, confirm the tip, remove the worktree). The push
   follows a background crate test and the branch read `ahead 1` at the
   time of writing; `git status -sb` is the witness. Landed: the
   unreachable `SPEAK_KIND` guard and its branch deleted from
   `interrupted`, with a comment on why a fresh opportunity always carries
   speech (the constant itself remains in use elsewhere in
   `controllers.rs`); `InferencePurpose` (`controllers.rs:92`),
   `PreparedInference::purpose` (:118), and `InferenceEvent` (:123)
   widened to `pub(crate)`, with two test seams `fixture_inference_events`
   (:274) and `fixture_recovery_required` (:220) re-exported from
   `world/mod.rs`; the driver end-to-end test
   `a_second_mid_turn_change_reaches_the_drivers_interrupted_arm`
   (`runtime.rs:3397`) through `run_cover_tick`, with two Interpreter calls
   and the overtaken turn committing nothing; and
   `a_witness_between_the_turn_and_submit_carries_no_overheard_row`
   (`controllers.rs:11377`), closing the step-10 seam Soul left. Baseline
   at `651bfb2`: 373 test functions in `world/*.rs`, 426 crate source, 13
   in `ghostlight-persona-projection`, plus one integration test (Hands ran
   372 / 420 / 13). Steps 9 and 10 are therefore complete with their
   follow-ups. Nothing is in flight and no worktree exists.

   Open question routed to the Eve owner: whether the command-result
   vocabulary needs a fourth state for "overtaken" instead of rendering an
   interruption as `denied`.

   In flight: the operator ordered "Build the Claude connector" on
   2026-09-06, then overturned both opening assumptions before anything
   landed. Void: (1) an Anthropic API key against the Messages API, because
   no API spend is possible now; (2) a second backend inside the
   CodexConnector repo, because CodexConnector is a deliberate fork of
   Codex cut so that Epiphany stops compiling Codex, and nothing goes into
   it. The Modeling map `modeling-claude-connector.md` was stopped mid-map
   and is void even if partial text is on disk. Plan step 11, the Claude SDK inference port, is integrated. Tip is
   `0cbbf77` (step-11 docs: plan, system map, smoke runbook Claude SDK
   route with operator prerequisites, mvp paragraph) over `4f424a5` and
   `0fa411b` (Soul), `2ff1d43` (sidecar), `b150b5e` (Rust port), `708d621`
   (collapse of the two prepared-identity bodies and two constructors), on
   `feb603b`. The push follows a background crate test and the branch read
   `ahead 6` at the time of writing; `git status -sb` is the witness. The
   research is the evidence record at 2026-09-05T22:25Z. What landed:
   `world/sdk_inference.rs` (`SdkInferencePort`, `RoutedInferencePort`,
   `SidecarLink` with `ChildProcessLink`, the `SidecarFrame` kinds,
   `lower_query`, `assemble_output`, `SdkInferenceReceipt`);
   `ToolResultOracle` (`controllers.rs:247`) with `InterpreterOracle`,
   `GroupedOracle`, `OperationalOracle`, and `ElaborationOracle`
   (`elaboration.rs:976`) over three extracted folds; `prepare_invocation`
   (`controllers.rs:287`) shared by both ports; `open_inference` routing by
   `GHOSTLIGHT_SDK_MODEL_PREFIX` (default `claude`), `GHOSTLIGHT_SDK_SIDECAR`
   with no default (opt-in; the plan sentence claiming a default was
   corrected on main), `GHOSTLIGHT_CONTROLLER_CREDENTIAL` required only
   with a connector binding; `ControllerRunner::open` injection-only,
   `with_test_ports` gone; `sidecar/claude-sdk` pinned to
   `@anthropic-ai/claude-agent-sdk` 0.3.261, `@msgpack/msgpack` 3.1.3,
   `zod` 4.5.4, Node `>=24 <25`; no credential anywhere in the tree.
   Baseline at `0cbbf77`: 408 test functions in `world/*.rs`, 461 crate
   source, 13 in `ghostlight-persona-projection`, 24 sidecar, plus one
   integration test (Soul ran 407 / 455 / 13 / 24).

   Soul verdict: integrate with follow-ups. The port is a retirable
   stopgap: deleting `world/sdk_inference.rs`, the sidecar package,
   `SdkBinding`, and the two `GHOSTLIGHT_SDK_*` variables removes it
   whole, while `prepare_invocation`, `ToolResultOracle` with its folds,
   and `RoutedInferencePort` survive as improvements. Decisive finding:
   the SDK Zod layer strips unknown keys and refuses missing required
   fields before the handler runs, so a `claude` lane would silently admit
   what the connector refuses and quarantine what the connector records as
   a gap. Ruling: one validator on both transports, the Rust decoder; the
   sidecar hands raw arguments through.

   The step-11 follow-ups are integrated at `d72c246` over `a791d55` and
   `3514d76`; the merge was confirmed by SHA equality before the worktree
   was removed, and the push followed the test result line. `a791d55`
   (sidecar): loose schema conversion, every property optional and unknown
   keys kept, so Rust is the one validator on the SDK transport too;
   `protocol_violation` kept with its remaining causes stated;
   `resultFault` exported and tested. `d72c246` (Rust): `take_oracle`
   above the gates; the four `pub(crate)` widenings reverted (two
   `private_interfaces` warnings return, matching the three siblings that
   already warn); `fixture_prepared` collapsed; `DEFAULT_SDK_MODEL_PREFIX`
   used in `runtime.rs`. `3514d76` made the source-scanning test
   line-ending neutral. Residual liability, stated in plan step 11: a
   property whose value has the wrong type reaches Rust as absent, not as
   the wrong value, because the SDK derives the advertised schema from the
   same object and refuses a dynamic pass-through. Baseline at `d72c246`:
   408 test functions in `world/*.rs`, 461 crate source, 13 in
   `ghostlight-persona-projection`, 29 sidecar, plus one integration test
   (Hands ran 407 / 455 / 13 / 29). Step 11 is complete with its
   follow-ups. Nothing is in flight and no worktree exists.

   Gating lesson, the second lapse of this session: `3514d76` was pushed
   red for ten minutes because the coordinator acted on a test run before
   reading its result line. Rule: the push follows the result line, never
   the launch; the first lapse was the chained `merge --ff-only` and
   `branch -D` that deleted a branch pointer. Both are recorded here and
   nowhere else durable.

   Next: the operator installs the Claude Code CLI and logs in (or
   `setup-token`) per `notes/local-live-smoke.md`, then the first SDK road
   run, which is the belief-changing record still owed. Facts that
   survive the overturn: Ghostlight binds Codex-named types in four files
   (`controllers.rs:25`, `elaboration.rs:31`, `patch.rs:20`,
   `tool_schema.rs:9`) behind one `InferencePort::prepare`
   (`controllers.rs:226`); the default model names in `runtime.rs:427-434`
   are `gpt-5.6-luna/sol/terra`; the Cargo pin is `68fe94b`
   (`Cargo.toml:21`); CodexConnector `main` was fast-forwarded to `6519289`
   before the correction, which is the line Ghostlight runs, is harmless,
   and stays; CodexConnector has no state, notes, handoff, or evidence
   surface and its doctrine (`AGENTS.md`) is untouched. The smoke runbook
   describes only the Codex credential path.

   Constraint, which this pass may lift: no road run is possible until a
   provider is available. The Codex subscription lapsed mid-build and the
   twelve seeded runs consumed half the operator monthly free quota. Until
   the Claude backend lands, work proves itself under fixture inference
   ports; the Codex connector is stopped and the smoke substrate at
   `F:\Projects\Ghostlight-smoke` stays.

   Next seams: the outbound consumer response
   batch with the non-loopback CultMesh lease; then the deployment gate
   below. The elaborator swarm's ideas still sit on
   `slot/<world>/<title>/<stamp>` branches and the four ledger directories;
   their integration remains owed and deferred (item 2).

   Decision the operator owns: the authority for an owner-only Eve
   `world.run_tick` command, which is not built.
   The ten specs and maps (`imagination-pass1..10.md`,
   `modeling-pass1..10.md`) remain in the session scratchpad only and are
   history, not steering; the docs own the design.

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
   The elaborator swarm is drained and handed off; this session does not own
   it. The stop flag `C:\Users\Meta\.claude\worlds\logs\elab-stop.flag` was
   set at 2026-09-04 20:10; the drain completed at 21:10 with 98 finished
   passes in `logs\elab-done.csv`, ledgers republished to the
   world branches, and the successor handoff finalized at
   `C:\Users\Meta\.claude\worlds\README.md` with final counts and the Charter
   idea. Resume or restart of the loop waits on operator adjustments and
   belongs to the successor, not to a Ghostlight kernel session. What remains owed and is
   deliberately deferred: integration of the `slot/<world>/<title>/<stamp>`
   branches into the lore vaults on `codex/ghostlight-worlds` and into
   `codex/world-<world>`. The idea index is the four world-branch ledger
   directories `experiments/elaboration/<world>/ledger/` on `codex/world-<world>`
   for aetheria, delvehold, kalsa, zyphos. The substrate lessons (workers never
   run git; a supervisor outside the Codex sandbox commits and pushes each
   slot clone; full clones, not linked worktrees) stay in `state/evidence.jsonl`.
   `Invoke-Worlds.ps1` and the first-generation loop are retired under
   `C:\Users\Meta\.claude\worlds\retired`; do not launch them.
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
route continuity, signed health, exact receipts, exclusive world-v3 state, and
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
- Faculty-workflow lessons (Epiphany): `F:\Projects\Epiphany
- Local live smoke runbook: `notes/local-live-smoke.md` (substrate at
  `F:\Projects\Ghostlight-smoke`, connector from the CodexConnector repo head
  with the `daemon` feature, environment, first-run table)
otesaculty-workflow-lessons-2026-09-04.md`

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
