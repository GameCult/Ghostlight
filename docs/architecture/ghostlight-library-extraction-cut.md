# Ghostlight library extraction — cut map (plan step 15, L0)

Status: cut map. Ends are owned by `ghostlight-library-extraction.md`; this
document owns the means. Written 2026-09-15 against `d83534c` on
`codex/ghostlight-dungeon-mvp`; first version committed at `02bbf62`;
refreshed 2026-09-16 with the operator's rulings.

Cut 1 landed at Ghostlight `108b691..f859d2c`. `108b691` is 13 renames with
zero content change; `f859d2c` builds. Verified by Soul's own runs, not the
Hands report: library 419 passed + 1 ignored with test names identical to the
base `world::` set; 10 compile_fail doc-tests pass, also under
`--all-features`; Dungeon 50 + 1 ignored, all 49 base names present and the
+2 exactly the two new soul tests; no `[features]`, `cfg(feature`, or
`cfg(any(test` in the library; no schema string, `_NAMESPACE`, `derive_id`,
serde attribute, or `Preimage` changed; the rewritten tests fail under
mutation of what they pin. Windows only; the Linux release remains Idunn's.

Soul found six defects. Triage:
- **F1 seal is name-based (high, open):** every `pub` ID derives
  `Deserialize` (`crates/ghostlight/src/lib.rs:186-200`), as do `ScopeDigest`
  (`:532`) and `DecisionOpportunity` (`:559`), so an external crate mints IDs,
  reproduces `ScopeDigest::fixture`, and forges a `DecisionOpportunity` into
  `derive_cover` from JSON. The reducer still rejects forged opportunities
  (`lib.rs:4192-4201`), so kernel integrity holds and the invariant's wording
  does not. Dungeon's own Eve payloads deserialize these types, so removing
  the derives is not available. Operator question Q1-9 below.
- **F2 single-minter test is bypassable (medium, fix):** the count at
  `crates/ghostlight-dungeon/src/app_session.rs:530-555` greps a literal and
  misses `<VerifiedPrincipalEvidence>::new(`, an alias, a renamed test module,
  or a subdirectory.
- **F3 surface wider than bought (low, fix):** 71 `pub` identifiers are
  unreferenced by Dungeon; `ScopeDigest`, `WorkLane`, and `ScopeComponents`
  compile as `pub(crate)` and were opened only to quiet `private_interfaces`.
- **F4 "Eve" named in library error strings (low, fix):**
  `crates/ghostlight/src/controllers.rs:1494,2576,2582,2584`. Pre-existing;
  invariant 3 is still unmet.
- **F5 two Hands claims false as stated (cosmetic, recorded):** `Cargo.lock`
  did gain a `ghostlight` package entry (no new external package), and
  `fixture_[a-z]` has 8 benign hits in Dungeon.
- **F6 recipe schema drift (pre-existing, recorded):**
  `deployment/idunn/recipe.toml` declares `world_state.foundation.v0` and
  `controller_work.v3` against code at `consumer.v4` and
  `controller_work.v15`. Outside L0; it changes no behavior here.

Cut 2 landed at gamecult-ops `6aba281` (text only; nothing live was run).
The library test step is in the runbook, the deploy script, and the wiring
test, ordered as `recipe.toml` orders it. Two spec defects Hands reported
rather than redesigned: the assertion this map specified was vacuous
(`require_text` is `grep -Fq`, and the string given is a substring of the
persona-projection line, so it passed with the library step deleted), so
Hands used the file's existing exact-line helper and proved it fails under
deletion; and the acceptance test path lives in `acceptance_test=` at
`scripts/deploy-ghostlight-yggdrasil.sh:34`, a fourth site this map did not
list, pinned by the wiring test and repeated in the runbook prose. Operator
consequence: changing `acceptance_test` changes the acceptance binding, so
sealed witnesses under `/srv/ghostlight/acceptance/<binding-sha>/` from
before this commit will miss and regenerate.

Soul passed on Cut 2 and found the assertions pin text, not invocations:
- **C2-F1 (high, fixing):** deleting the two lines above the pinned cargo
  line in `scripts/deploy-ghostlight-yggdrasil.sh` leaves it byte-identical
  as trailing argv on the *persona-projection* container, so the kernel
  tests never run and the wiring test still exits 0. Prefixing the
  `docker run` line with `: ` does the same.
- **C2-F2 (medium, fixing):** nothing pins the order of the four cargo
  steps; the library block moved after the release build and the wiring
  test passed, which would seal an artifact before the kernel tests ran.
- **C2-F3 (medium, fixing):** the wiring test never reads the runbook, so
  only three of the four acceptance sites are enforced to agree.
- **C2-F4 (medium, pre-existing, recorded):** `deploy-…sh:937` takes a
  reuse branch when the release directory for a commit already exists, so a
  redeploy of a sealed commit runs no cargo steps at all. This bounds "the
  deploy runs the kernel tests" to a commit's first build. Operator owns
  whether that is right.
- **C2-F5 (low, recorded):** `docs/repo-census-2026-09/*` still shows
  `src/world/...`. Dated census; no deploy effect.

Held: the step sits in the same branch, image and mounts as its
neighbours, and `set -euo pipefail` with `trap` means a library-test
failure does abort the deploy. The acceptance test exists at
`crates/ghostlight/src/controllers.rs:8980` under `-p ghostlight --lib`.

Open: **Q1-9 what sealing means.** A: restate invariant 1 as unforgeable
*admission* — external code may hold a syntactically valid ID or opportunity,
and the kernel must reject any it did not issue — and prove it with rejection
tests from an external crate plus the existing compile_fail set. B: make the
types unconstructible from outside, which means Dungeon stops deserializing
`DecisionOpportunity` in its Eve payloads and receives an opaque token
instead, a design change L0 excludes. **Recommended: A**, because the kernel
already enforces it and B moves work into L0 that the target puts out of
scope. Whichever is chosen, F2 and F3 are fixed in the same Hands batch.

Rulings (operator, 2026-09-15):
- **Q1-1 crate name:** "crate name is `ghostlight`."
- **Q1-2 adapters:** "`vault.rs` and `sdk_inference.rs` both go in the
  library."
- **Q1-3 principal seam:** "option A, library-owned
  `VerifiedPrincipalEvidence` with a public `new`, plus the Dungeon count
  test pinning the single minter."
- **Q1-4 fixtures:** "NOT the `test-support` feature. The operator takes the
  alternative: the sealed ID `issue()` constructors and `ScopeDigest::fixture`
  stay unreachable from outside the library under every configuration,
  including feature-enabled builds, and the Dungeon tests that currently
  need them move into the library instead. Rationale: a sealed constructor
  exposed behind a feature is still reachable by any crate that turns the
  feature on, which contradicts invariant 1." (Supersedes the `test-support`
  design in the `02bbf62` map; that text is history and is not repeated
  here. The classification below found that none of the affected tests is
  a kernel test, which raises Q1-7 and Q1-8.)
- **Q1-5 persona projection:** "`ghostlight-persona-projection` stays a
  separate crate."
- **Q1-6 gamecult-ops:** "edit the five gamecult-ops cargo lines in Cut 2;
  do not retire the scripts (retirement belongs to the deployment gate)."
  Correction to the `02bbf62` map: Ghostlight's handoff does call those
  helpers unsafe — `notes/fresh-workspace-handoff.md:389-391`: "Do not
  invoke the current bounded Connector or Ghostlight redeploy helpers; they
  fail at the root/Idunn Git boundary and still embody duplicate target
  deployment authority." The earlier claim that no such doc existed was
  wrong; the ruling to edit rather than retire stands on the deployment
  gate owning retirement, not on the scripts being live.

Rulings (2026-09-16), taken by Self under the operator's standing "Go"
after Q1-4's classification found no kernel test to move. Either may be
reversed by the operator; both are recorded here as the live design:
- **Q1-7 port value constructors:** option A. `PreparedInference::prepare`
  (renamed from `prepare_invocation`), `InferenceOutput::new`, and
  `InferenceFault::{retryable, recovery_required, integrity_violation}`
  become the port's public contract; the five `fixture_*` items are
  deleted and `TracingInferencePort` becomes an ungated decorator. A
  public trait whose return types only the library can construct is not a
  port, none of these touches a sealed item, and the change is a net
  deletion.
- **Q1-8 cover source:** option A. The tick-ordering test derives its
  cover from a real world through `derive_cover` and asserts 2 cells, with
  its ordering and shared-tick assertions unchanged;
  `fixture_controller_opportunities`, the only outside caller of a sealed
  `issue()`/`ScopeDigest::fixture`, is deleted.

Together these satisfy Q1-4's intent: the sealed ID issuers and
`ScopeDigest::fixture` stay private `#[cfg(test)]` items with no feature
gate anywhere, unreachable from outside the library under every
configuration.

Open: nothing blocking Cut 1. Operator checks remain on the Linux release
(Idunn's recipe run), which no Windows probe can prove.
Follow-ups outside this migration: see "Findings not assigned to a cut".

Probe names versus real names: probes 1–4 built the crate as
`ghostlight-world` (`use ghostlight_world::…`); probe 5 rebuilt it as
`ghostlight` (`use ghostlight::…`). The difference is the package name, the
directory `crates/ghostlight`, and the path prefix; every mechanism finding
held in both. This map uses the real name throughout.

## What the probes established

All probes ran in scratch worktrees at `d83534c` (both removed, repo clean)
on the Windows workstation with `cargo check` and `cargo test --doc`; they
prove the crate graph and visibility on Windows only. The Linux proof is
Idunn's recipe run on Yggdrasil. Logs are in the session scratchpad
(`probe1*-check.log`, `probe3*-visibility.log`, `probe5.log`,
`base-test-list.log`); they are not repo artifacts.

- **Base test census** (`cargo test -p ghostlight-dungeon --bin
  ghostlight-dungeon -- --list`, repo target dir): 469 unit tests in the bin,
  of which 420 are under `world::` (action 34, clock_tests 45, consumer 30,
  controllers 81, cover 18, custody_tests 17, elaboration 14, journal 24,
  knowledge_tests 21, mailbox 15, patch 34, sdk_inference 23,
  soul_knowledge_tests 9, tests 32, vault 6, witness_tests 17) and 49 are
  Dungeon's own (app_session 7, eve 5, heimdall 5, idunn_health 4, mesh 4,
  runtime 24). `tests/build_provenance.rs` has 1. `ghostlight-persona-
  projection` has 13 unit, 0 doc. Two tests are `#[ignore]`:
  `world::controllers::tests::real_codex_connector_cognition_modes_commit_speech`
  and `runtime::tests::live_smoke_seeds_then_ticks_a_world_against_the_connector`.
- **Base warning**: `mod.rs:28-32` re-exports nine names nobody in the crate
  uses (`ControllerOpenError`, `NarrativeCapture`, `NarrativeDecision`,
  `NarrativePending`, `OperationalCapture`, `OperationalDecision`,
  `OperationalPending`, `SourceRange`, `TranslationGapSummary`). Their
  deletion is inside Cut 1.
- **The move compiles** (probe 1d). Moving `src/world/*` to the new crate
  (`mod.rs` → `lib.rs`), rewriting `crate::world::` → `crate::` inside it and
  → `ghostlight::` in Dungeon, and moving `VerifiedPrincipalEvidence` into
  the library root: `cargo check` on the library, its tests, the Dungeon
  bin, and the Dungeon tests all pass. Two mechanical faults: the 94
  `pub(super)` sites in the former `mod.rs` are illegal at a crate root
  (become `pub(crate)`), and `consumer.rs:62` needs `serde_bytes` in the
  library manifest.
- **What Dungeon needs public** (probe 3, four compiler rounds from every
  item `pub(crate)`): 54 production items, 22 test-only items, 38 fields on
  10 structs, 36 methods, the opaque IDs `AffordanceId`/`CommandId`/
  `SubjectId`, and `FictionalMinutes(u64)` (one Dungeon test). Nothing
  sealed appears: `WorldKernel`, `WorldState`, `reduce`, `apply_effect`,
  `CommandEnvelope`, `AuthenticatedCaller`, the journal, `*::issue()` and
  `ScopeDigest::fixture` are never named by Dungeon. Under the narrowed
  library `cargo check -p ghostlight` emits 646 dead-code warnings — a
  second enumeration of the same boundary (an item used only by Dungeon is
  dead inside the library until it is `pub`).
- **The sealed set under the Q1-4 ruling** (probe 5, crate `ghostlight`, no
  cargo features, fixtures left `#[cfg(test)]`): ten `compile_fail`
  doc-tests on the library root pass under `cargo test -p ghostlight --doc`
  and again under `cargo test -p ghostlight --all-features --doc` (the crate
  declares no features, so the second run is the same build; it is the
  command Soul re-runs if a feature is ever added). `cargo check -p
  ghostlight-dungeon --bin ghostlight-dungeon` passes: production needs no
  fixture. `cargo check -p ghostlight-dungeon --tests` fails on exactly four
  errors — `runtime.rs:3081` (`ControllerWork…` family, `Inference…`
  types, three `fixture_inference_*`), `:3570`
  (`fixture_controller_opportunities`), `:2582` (`TracingInferencePort`),
  `:2581` (`InferencePort`, because the base re-export at `mod.rs:66` is
  `cfg(test)`). Those four sites are the whole cross-boundary test surface;
  they are classified under Cut 1.
- **Compile-fail mechanism**: doc-tests compile as an external crate, which
  is exactly the boundary; no `trybuild`, no scratch crate. The ten
  (`use ghostlight::WorldKernel`, `reduce`, `WorldState`,
  `CommandEnvelope`, `AuthenticatedCaller`, `journal::WorldJournal`,
  `SubjectId::issue()`, `ScopeDigest::fixture("x")`,
  `fixture_controller_opportunities(&[])`, `fixture_inference_output(..)`)
  are pinned to `E0603`, `E0599`, `E0599`, `E0425`, `E0425`.
- **Negative-grep collisions** checked in `src/world/`: `axum`, `tower_http`,
  `jsonwebtoken`, `heimdall`, `eve::`, `idunn_health`, `Session Zero` have
  zero legitimate hits. `Dungeon` collides with the vault fixture path
  `Spoilers/Dungeons/Provenance.md` (`vault.rs:489,581`): use `\bDungeon\b`.
  `player` collides with `tests::player()` (`consumer.rs:966`): do not
  publish a bare `player` pattern. `app_session` has 5 hits, all the
  principal seam, and must reach 0.
- **Line endings**: index and working tree are LF (`git ls-files --eol`
  reports `i/lf w/lf`); `core.autocrlf=true` rewrote every fresh scratch
  worktree to CRLF. Hands edits in `F:\Projects\Ghostlight` itself, never in
  a fresh worktree; if a worktree is unavoidable, run `git ls-files --eol
  crates` after the move and refuse any `w/crlf`. `sdk_inference.rs:1831`
  normalizes `\r\n` before scanning source.

## Cut 0. Captures

- **Repo/branch:** Ghostlight `codex/ghostlight-dungeon-mvp` at `02bbf62`
  (source identical to `d83534c`). No commit; output lives in the session
  scratchpad.
- **First:**
  - Test census: `cargo test -p ghostlight-dungeon --bin ghostlight-dungeon
    -- --list > base-tests.txt` (PowerShell `>` writes UTF-8 BOM + CRLF;
    compare post-cut with the same redirection and `Compare-Object` on
    trimmed lines after stripping the `world::` prefix). Expected 469.
  - A `world.cc` written at base. No store exists on disk
    (`F:\Projects\Ghostlight-smoke\ghostlight` holds no `world.cc`), so
    produce one: in a scratch copy at `d83534c`, add an ignored test beside
    `mailbox.rs:966 fixture()` that opens `WorldMailbox::open(<capture>/world.cc)`,
    creates through `create_fixture`, approves, activates, and ticks three
    times through `submit_clock`; record `snapshot().revision`, the
    `state_digest`, and the last `commit_digest` from `operator_log` to
    `capture.txt`. Copy `world.cc` and `capture.txt` to the scratchpad.
    After Cut 1 the same test body, in the library's `mailbox` tests,
    reopens that file and asserts the three values are equal. The
    `controller-work.cc` layout is pinned by the untouched
    `CONTROLLER_WORK_SCHEMA`/`CONTROLLER_WORK_ROW` constants
    (`controllers.rs:75-76`) and the existing v7/v8 refusal tests
    (`controllers.rs:6487,10654`); no separate capture.
  - `Get-ChildItem target -Recurse | Measure-Object Length -Sum`: 7.5 GiB at
    the start of this pass.

## Cut 1. Extract the library crate

- **Repo/branch:** Ghostlight `codex/ghostlight-dungeon-mvp` from
  `02bbf62`. One commit; the crate graph does not build half-moved. Depends
  on Cut 0 and rulings Q1-7, Q1-8.
- **First:** Cut 0 captures in hand; `git status` clean; working in the
  main checkout (LF).
- **Deletes first:**
  - `crates/ghostlight-dungeon/src/world/mod.rs:28-32` — the nine unused
    re-exported names above (leave `CellRun`, `ConnectorBinding`,
    `ControllerError`, `ControllerModels`, `ControllerPendingReason`,
    `ControllerRunner`, `ControllerWorkCustody`, `NarrativeRun`,
    `OperationalRun`, `SubmissionDisposition`, `open_controller_work`,
    `open_inference`).
  - `crates/ghostlight-dungeon/src/world/mod.rs:61-72` — the `#[cfg(test)]`
    re-export block (12 lines). The port traits and their value types in it
    return as ungated production `pub use` under Adds; the three
    `fixture_inference_*` names do not return.
  - `crates/ghostlight-dungeon/src/world/mod.rs:483-508` —
    `fixture_controller_opportunities` (26 lines), per Q1-8 A; and
    `controllers.rs:372-382,409-432` — `fixture_prepared_inference`,
    `fixture_inference_output`, `fixture_inference_events` (about 40
    lines) and `controllers.rs:221-237` — `InferenceFault::fixture_*`
    (17 lines), per Q1-7 A. Under Q1-7 B / Q1-8 B these stay `#[cfg(test)]`
    inside the library and the Dungeon tests that used them are deleted
    instead (see the classification).
  - `crates/ghostlight-dungeon/src/app_session.rs:54-91` — the
    `VerifiedPrincipalEvidence` struct, accessors, and `fixture` (38 lines).
  - `crates/ghostlight-dungeon/src/main.rs:7` — `mod world;`.
  - `crates/ghostlight-dungeon/Cargo.toml:27` — the direct
    `ghostlight-persona-projection` dependency (`controllers.rs:35` is the
    only importer and it moves).
- **Keeps and moves:**
  - `git mv crates/ghostlight-dungeon/src/world/{action,clock,consumer,
    controllers,cover,elaboration,journal,mailbox,patch,sdk_inference,
    tool_schema,vault}.rs crates/ghostlight/src/` and
    `git mv .../world/mod.rs crates/ghostlight/src/lib.rs`. Verify with
    `git diff --cached -M --stat` that all 13 show as renames.
  - Inside the moved files: `crate::world::` → `crate::` (199 sites;
    `super::super::` paths need no change); `pub(super)` → `pub(crate)` in
    `lib.rs` only (94 sites; submodules keep theirs);
    `crate::app_session::VerifiedPrincipalEvidence` → `crate::VerifiedPrincipalEvidence`
    (`mailbox.rs:10`, `controllers.rs:10714,10790,10988,11473`).
  - `vault.rs` and `sdk_inference.rs` move with the rest (Q1-2). Their
    fixture writes (`sdk_inference.rs:1900,1983`) resolve from
    `crates/ghostlight` at the same depth.
  - `build.rs`, `tests/build_provenance.rs`, `runtime.rs:375` (web/dist)
    stay in Dungeon; `CARGO_MANIFEST_DIR/../..` still resolves.
  - No test moves between crates. Every test under `world::` was already a
    library test and moves with its file (420); every Dungeon test stays in
    Dungeon (49). See the classification.
- **Adds:**
  - `Cargo.toml` (workspace): member `crates/ghostlight`.
  - `crates/ghostlight/Cargo.toml`: package `ghostlight`; no `[features]`;
    dependencies `async-trait`, `chrono`, `codex-connector`, `cultcache-rs`,
    `ghostlight-persona-projection = { path = "../ghostlight-persona-projection" }`,
    `rmp-serde`, `serde`, `serde_bytes = "0.11"`, `serde_json`, `sha2`,
    `thiserror`, `tokio`, `tracing`, `uuid`, `zeroize` (all `.workspace =
    true` except `serde_bytes`, which the workspace does not declare); dev
    `tempfile = "3"`. Every one is already in `Cargo.lock`; no new package.
  - `crates/ghostlight-dungeon/Cargo.toml`: `ghostlight = { path =
    "../ghostlight" }` in `[dependencies]`; nothing in dev-dependencies.
  - `Cargo.lock`: the new `[[package]] name = "ghostlight"` entry. Commit
    it; Idunn builds `--locked`.
  - `crates/ghostlight/src/lib.rs`, root: `pub struct VerifiedPrincipalEvidence
    { account_subject_hash: String, valid_until: DateTime<Utc> }` with `pub fn
    new`, `pub fn account_subject_hash`, `pub fn valid_until`, and
    `#[cfg(test)] fn fixture` delegating to `new` (Q1-3). Doc comment: the
    consumer verifies; the library bounds by `valid_until` at
    `mailbox.rs:231`.
  - `crates/ghostlight/src/lib.rs`, the public boundary, one ungated list.
    `pub use clock::{FictionalMinutes, TickMinutes}`;
    `consumer::{CONSUMER_BODY_LIMIT, CONSUMER_CREDENTIALS_ENVIRONMENT,
    CONSUMER_PATCH_SCHEMA, CONSUMER_RECEIPT_SCHEMA, ConsumerRegistry,
    admit_document, encode_receipt}`;
    `controllers::{CellRun, ConnectorBinding, ControllerError,
    ControllerModels, ControllerPendingReason, ControllerRunner,
    ControllerWork, ControllerWorkCustody, ControllerWorkLookup,
    ControllerWorkStore, ControllerWorkStoreError, ControllerWorkWrite,
    InferenceEvent, InferenceFault, InferenceOutput, InferencePort,
    InferencePurpose, InferenceRequest, NarrativeRun, OperationalRun,
    PreparedInference, SubmissionDisposition, ToolResultOracle,
    open_controller_work, open_inference}` (plus `TracingInferencePort`
    under Q1-7 A);
    `cover::{AgencyGraph, Cell, Cover, CoverBudget, TickIndex, derive_cover}`;
    `elaboration::{SeedOutcome, select_row}`;
    `mailbox::{ConsumerPort, ControllerPort, MailboxError, SeedPort, WorldMailbox}`;
    `patch::{JurisdictionKey, Statement}`;
    `sdk_inference::{DEFAULT_SDK_MODEL_PREFIX, SdkBinding}`;
    `vault::VaultEvidenceSource`. Root items made `pub`: `STATE_SCHEMA`,
    `state_schema_compatibility_tag`, `AffordanceId`, `CommandId`,
    `SubjectId` (the `opaque_uuid!` macro at `mod.rs:98-106` emits
    `pub(crate) struct`; give it a visibility parameter and emit `pub` for
    these three only — the `issue()` impls stay private and `#[cfg(test)]`),
    `PrincipalId`, `WorldPhase`, `SubjectKind`, `ControllerMode`,
    `CreateWorldIntent`, `CreateJurisdictionIntent`, `DecisionOpportunity`,
    `DecisionInvocation`, `PrincipalCommandIntent`, `CommandBody`,
    `WorldSnapshot`, `OperatorEvent`, `CommitReceipt`, `SubmitReceipt`,
    `KernelError`, `ScaleDeficitRow`, and the snapshot row types Dungeon
    reads through `WorldSnapshot` fields (`SubjectSnapshot`,
    `PlaceSnapshot`, `AffordanceSnapshot`, and whichever others the
    compiler names; probe 3 stopped at the `WorldSnapshot` field layer).
    `GroupedRun` (`controllers.rs`) is reached through `ControllerRunner`
    return values and needs `pub` with its five fields.
  - How the ports stay public without the fixture constructors becoming
    reachable: `InferencePort` (`controllers.rs:259`) and
    `ControllerWorkStore` (`:1541`) are traits whose method signatures name
    `InferenceRequest`, `PreparedInference`, `InferenceOutput`,
    `InferenceFault`, `ToolResultOracle`, `ControllerWork`,
    `ControllerWorkLookup`, `ControllerWorkWrite`, `ControllerWorkCustody`,
    `ControllerWorkStoreError`. Those types become `pub`; their fields stay
    private except where listed below. Nothing about them reaches an ID
    `issue()` or `ScopeDigest::fixture`, which are private free functions
    and `#[cfg(test)]` impls (`mod.rs:200-226,448`) with no `pub` path of any
    kind and no cargo feature — reachable only from inside the library's
    own test build. The `fixture_*` functions are deleted (Q1-7 A / Q1-8 A)
    or stay `#[cfg(test)]` (B), and the ten doc-tests pin both outcomes.
  - Fields that become `pub` (38, from probe 3): `CommitReceipt.{command_id,
    commit_digest, resulting_revision, resulting_state_digest}`,
    `Constituent.subject`, `Cover.{cells, oversubscribed, tick}`,
    `CoverBudget.{cells, constituent_cap, urgency_slots}`,
    `DecisionOpportunity.{controller_mode, world_id}`, `GroupedRun.{cell,
    needs, pending, resolution, submissions}`, `OperatorEvent.{revision,
    speech}`, `PreparedInference.purpose`, `ScaleDeficitRow.{deficit,
    jurisdiction, kind, target}`, `WorldSnapshot.{affordances,
    draft_approvals, now, opportunities, owner, phase, places,
    required_approvers, revision, scale_deficit, subjects, title, world_id}`.
    `FictionalMinutes.0` stays `pub(crate)`: under Q1-8 A the one Dungeon
    use (`runtime.rs:3579`) is replaced by `snapshot.now`. Fields of the row
    structs reached through `WorldSnapshot` follow in the next compiler
    round.
  - Methods that become `pub` (36, from probe 3, by definition site):
    `WorldMailbox::{open, create, submit_principal, submit_clock,
    agency_graph, operator_log, snapshot}` (`mailbox.rs:93,105,226,355,448,
    461,474`), `ConsumerPort::new` (`:552`), `SeedPort::new` (`:594`),
    `ElaborationPort::new` (`:660`), `ControllerPort::{new, snapshot,
    submit_controller}` (`:692,696,721`), `ControllerRunner::{open,
    elaborator, seeder, custody_probe, run_narrative, run_operational,
    run_cell}` (`controllers.rs:2691,2712,2728,2744,2748,2838,3057`),
    the receipt/pending accessors at `controllers.rs:2397,2413,2421,2453,
    2457,2461,2465,2469,2515,2519,2523,2543,2591,2595` (`persona_turn`,
    `re_lowering`, `into_parts`, `subject`, `bound_scope_digest`,
    `fresh_scope_digest`, `gap`, `mode`, `reason`, `persona_prose`),
    `sweep` (`elaboration.rs:607,1746`), `name` (`elaboration.rs:1511`),
    `Cover::{singletons, groups, validated}` (`cover.rs:163,170,198`),
    `ConsumerRegistry::{empty, from_secret_file}` (`consumer.rs:168,176`),
    `TickMinutes::{new, minutes}` (`clock.rs:45,51`), `CommandId::new`
    (`mod.rs:122`), `PrincipalId::new` (`mod.rs:239`), `Statement::new`
    (`patch.rs:281`). Under Q1-7 A add `PreparedInference::prepare`
    (renamed from `prepare_invocation`, `controllers.rs:388`),
    `InferenceOutput::new`, `InferenceFault::{retryable, recovery_required,
    integrity_violation}`, `InferenceRequest::provider_model` (`:120`), and
    `TracingInferencePort::new` (`:285`).
  - `crates/ghostlight/src/lib.rs` crate docs: the ten `compile_fail`
    doc-tests from probe 5, each pinned to its error code.
  - `crates/ghostlight-dungeon/src/runtime.rs` (tests): a ~25-line copy of
    `soul_no_credential_name_appears_in_the_ports_own_source` scanning
    `runtime.rs` only, because the library test cannot read another
    crate's source (see per-file `sdk_inference.rs:1824-1828`).
  - `crates/ghostlight-dungeon/src/app_session.rs` (tests): one test
    asserting `VerifiedPrincipalEvidence::new(` occurs exactly once in the
    crate's production source, at `account_for_cookie` (same shape as the
    `from_secret_file` count at `sdk_inference.rs:1860`). This is the
    single-minter rule the deleted private fields used to enforce (Q1-3).
- **Cross-boundary test classification (Q1-4).** Every Dungeon test that
  reaches an item which is `#[cfg(test)]` in the library today. Method:
  probe 5's four compile errors plus a read of each test body.

  | # | File:line | Test | Exercises | Reaches | Class |
  |---|---|---|---|---|---|
  | 1 | `runtime.rs:3270` | `the_tick_driver_never_exceeds_its_controller_permit_pool` | `run_cover_tick` and `state.controller_permits`: two singleton cells, pool of one, no overlapping `infer` | `fixture_prepared_inference`, `fixture_inference_output` (via `CountingInferencePort`, `:3241`); `AlwaysFreshWorkStore` (`:3194`) uses only `pub` enum variants | Dungeon runtime |
  | 2 | `runtime.rs:3337` | `quarantine_raised_mid_tick_stops_the_sibling_cell_and_every_later_tick` | the driver's `controller_quarantined` flag across sibling cell and next tick | `fixture_prepared_inference`, `InferenceFault::fixture_integrity_violation` (`QuarantiningInferencePort`, `:3308`) | Dungeon runtime |
  | 3 | `runtime.rs:3500` | `a_second_mid_turn_change_reaches_the_drivers_interrupted_arm` | the driver's `Interrupted` arm reached through `run_cover_tick`, commits through `ControllerPort::submit_controller` (production) | `fixture_prepared_inference`, `fixture_inference_output`, `fixture_inference_events`, `InferenceFault::fixture_recovery_required`, `InferenceEvent::ToolCall` (pub variant), `PreparedInference.purpose` (`InterruptingCoverPort`, `:3432`) | Dungeon runtime |
  | 4 | `runtime.rs:3567` | `drive_one_tick_runs_every_cell_before_the_clock_and_all_share_one_tick` | `drive_one_tick` ordering: every cell before the clock, all cells carry the cover's tick | `fixture_controller_opportunities` (the only caller of the sealed `WorldId/SubjectId/ControllerId::issue()` and `ScopeDigest::fixture` from outside `world/`), `FictionalMinutes(u64)`, `AgencyGraph::default`, `Cell`, `TickIndex`, `CommitReceipt` literal (`controller_commit`, `:2492`) | Dungeon runtime |
  | 5 | `runtime.rs:3697` (ignored) | `live_smoke_seeds_then_ticks_a_world_against_the_connector` | the road test: seed from a real Vault, activate, tick against a real connector; `GHOSTLIGHT_SMOKE_TRACE` writes the whole membrane (`notes/local-live-smoke.md:214-246`) | `TracingInferencePort` (`fixture_with`, `:2581-2582`) | Dungeon runtime |

  Counts: 5 tests reach test-only library items; 0 are kernel tests; 5 are
  Dungeon runtime tests. Two other Dungeon tests touch the boundary but
  need nothing test-only: `runtime.rs:2502`
  `no_proposal_projection_carries_the_canonical_world_commit` builds a
  `CommitReceipt` literal (four `pub` fields, in the boundary list) and
  `runtime.rs:4293` `soul_verified_evidence_outlives_the_session_that_minted_it`
  uses `VerifiedPrincipalEvidence::fixture` → `::new` (Q1-3). Every other
  Dungeon test (42) uses production ingress only.

  Consequence: the ruling's "move the tests into the library" has no test
  to move. Tests 1–4 exercise `run_cover_tick`, `drive_one_tick`, the permit
  pool, and quarantine — Dungeon's tick driver — and test 5 is Dungeon's
  road test; moving any of them would move the driver. What they need
  divides into two kinds, and each is an operator question below rather
  than a mechanism this map invents:
  - tests 1, 2, 3, 5 need to construct **port values** (`PreparedInference`,
    `InferenceOutput`, `InferenceFault`) and a port decorator; none of
    these is a sealed item under invariant 1 (mutable state, ID allocator,
    reducer, journal, authenticated-caller constructor) — Q1-7;
  - test 4 needs a **`Cover`** to walk, and today gets it from fabricated
    opportunities minted with sealed IDs — Q1-8.

  Expected post-move counts, under Q1-7 A and Q1-8 A: library 420 unit
  (the base `world::` list, prefix stripped) + 10 doc; Dungeon bin 49
  (the same 49 names; tests 1–4 rewritten in place, test 5 unchanged);
  `build_provenance` 1; persona-projection 13; ignored 1 + 1. Under Q1-7 B
  tests 1–3 and the `trace` half of test 5 are deleted (Dungeon 46, or 45
  if test 4 goes under Q1-8 B); the library count is unchanged either way.
- **Per-file changes** (against `d83534c` = `02bbf62` source):
  - `Cargo.toml:3-5`: add member.
  - `crates/ghostlight-dungeon/Cargo.toml:27`: replace with the library
    path dep.
  - `crates/ghostlight-dungeon/src/main.rs:7`: delete.
  - `crates/ghostlight-dungeon/src/runtime.rs:3-22`: split the `use crate::{}`
    block; `world::{…}` becomes `use ghostlight::{…}` (probe edit); `:4`:
    import `VerifiedPrincipalEvidence` from `ghostlight`, not `app_session`.
    `:518,519,535,1084,1088,1092,1201,1325,1621,2492,2493,2581,2582,3081,
    3501,3568,3658,3666,3675,3677,3777,3782,4267`: `crate::world::` →
    `ghostlight::`. `:4313`: `::fixture(` → `::new(`. `:3081-3086`: drop
    the three `fixture_inference_*` names. `:3243,3310,3434`:
    `PreparedInference::prepare("ghostlight-controller-test",
    4_102_444_800_000, request)` (Q1-7 A; the two literals are the ones
    `fixture_prepared_inference` hard-codes at `controllers.rs:381`).
    `:3255,3442,3447`: `InferenceOutput::new(vec![InferenceEvent::Text(..)],
    "sha256:<receipt>")`; `:3457`: `InferenceOutput::new(events, ..)`;
    `:3318`: `InferenceFault::integrity_violation(..)`; `:3479`:
    `InferenceFault::recovery_required(..)`. `:3567-3644` (Q1-8 A): replace
    `fixture_controller_opportunities` + `FictionalMinutes(..)` with
    `fixture().await`, `active_two_cell_world`, `snapshot()`, and
    `agency_graph()`, then `derive_cover(snapshot.world_id, snapshot.now,
    CLOCK_TICK_MINUTES, &snapshot.opportunities, &graph, budget)`; the
    cell count assertion becomes 2 (a genesis world has two
    controller-bearing subjects — `:3089-3095` already says this is the
    most `world.create` can produce); the ordering and shared-tick
    assertions are unchanged. `:504,2673` doc comments name
    `world::consumer`; reword to the crate.
  - `crates/ghostlight-dungeon/src/eve.rs:3-6` and `:182,335,338,354,386,
    405,449,494`: same rewrite.
  - `crates/ghostlight-dungeon/src/mesh.rs:6-9`: same.
  - `crates/ghostlight-dungeon/src/idunn_health.rs:8`: same.
  - `crates/ghostlight-dungeon/src/app_session.rs:54-91`: delete; `:6`: add
    `use ghostlight::VerifiedPrincipalEvidence;`; `:232-235`: struct literal
    → `VerifiedPrincipalEvidence::new(session.account_subject_hash.clone(),
    session.access_expires_at)`.
  - `crates/ghostlight/src/controllers.rs:221-237,372-382,409-432` (Q1-7 A):
    delete the fixtures; `:388` `prepare_invocation` → `pub fn
    PreparedInference::prepare` (keep the body; the two in-library callers
    at the connector and SDK ports follow); `:150` add `pub fn new(events,
    receipt_digest)`; `:192,206` `new` → `pub fn retryable`,
    `integrity_violation` → `pub`, add `pub fn recovery_required(detail)`
    (the `RecoveryRequired` disposition has a private `new` today; make it
    the named constructor); `:277-370` drop `#[cfg(test)]` from
    `TracingInferencePort` and make it and `new` `pub`.
  - `crates/ghostlight/src/sdk_inference.rs:1824-1828`: the scanned file
    list becomes `["sdk_inference.rs", "controllers.rs"]` (paths are now
    crate-root relative); `:1860` `sources[1]` still indexes
    `controllers.rs`. The `runtime.rs` half moves to Dungeon (Adds).
  - `deployment/idunn/recipe.toml:14-17`: `test-world-owner` becomes two
    steps — `test-world-library`: `["cargo","test","--locked","-p","ghostlight"]`
    (runs the 420 unit tests and the 10 doc-tests) and `test-dungeon`:
    the existing `-p ghostlight-dungeon --bin ghostlight-dungeon` line.
    `:51-53`: acceptance argv becomes `-p ghostlight --lib
    controllers::tests::real_codex_connector_cognition_modes_commit_speech`
    (the `world::` prefix is gone; `--exact` still matches).
  - `package.json:9` (`dungeon:run`): unchanged; `:7-8` use `--workspace`
    and pick the crate up.
  - `notes/local-live-smoke.md:64`: unchanged; the ignored smoke stays in
    `runtime.rs:3697`.
  - Docs that name the old path (owner in brackets): `README.md:165-166`
    [Hands] add the library line; `docs/public-architecture/index.md:46`
    [Hands]; `docs/architecture/ghostlight-session-zero.md:21-23` [Hands]
    rewrite "Source does not yet match" as the current rule;
    `docs/architecture/ghostlight-world-ontology.md:939-941` [Hands] the
    build-budget sentence now names `crates/ghostlight`;
    `notes/ghostlight-current-system-map.md:18,28,289` [Mind steward];
    `notes/fresh-workspace-handoff.md:284-286` [steward];
    `state/map.yaml:57,389-390` [steward].
- **Authority map:**
  - Owner: the library crate `ghostlight` owns the world kernel, its
    ontology, reducer, journal, mailbox, controllers, elaboration, cover,
    clock, consumer admission, inference ports (connector, SDK sidecar,
    and the tracing decorator), and the markdown evidence source. Its
    public surface is the list under Adds and nothing else.
  - Inputs: `VerifiedPrincipalEvidence` values Dungeon constructs from
    `AppSessionOwner::account_for_cookie`; `CreateWorldIntent`,
    `PrincipalCommandIntent`, `DecisionInvocation`; consumer patch bytes via
    `admit_document`; store paths via `WorldMailbox::open` and
    `open_controller_work`; connector and SDK bindings via `open_inference`;
    a consumer-implemented `InferencePort`/`ControllerWorkStore` when the
    consumer supplies its own (Dungeon's tests do; production uses the
    library's).
  - Outputs: immutable `WorldSnapshot`, `OperatorEvent`, `AgencyGraph`,
    typed receipts (`SubmitReceipt`, `CommitReceipt`, `CreationReceipt`
    through `MailboxError`/`KernelError`), controller runs, cover
    partitions, encoded consumer receipts.
  - Derived state: Dungeon's Eve projection, mesh documents, Idunn health
    (which reads `STATE_SCHEMA` and `state_schema_compatibility_tag` and
    derives, never restates), HTTP responses.
  - Forbidden writers: any path in Dungeon naming `WorldKernel`,
    `WorldState`, `reduce`, `apply_effect`, `CommandEnvelope`,
    `AuthenticatedCaller`, the journal, an `issue()`, or
    `ScopeDigest::fixture` — enforced by visibility with no feature that
    can lift it, proven by the doc-tests under `--all-features`. Any second
    minter of `VerifiedPrincipalEvidence` — enforced by the Dungeon count
    test. Any fixture-shaped `DecisionOpportunity` — there is no
    constructor for one outside the reducer.
  - Shared paths: Dungeon production and Dungeon tests reach the kernel
    through the same `pub` items; a Dungeon test port constructs port
    values through the same constructors the library's own ports use
    (Q1-7 A). Library-internal tests keep `pub(crate)`/`super::` access as
    today. Consumers other than Dungeon: none yet; Epiphany consumes only
    `ghostlight-persona-projection`, which is unchanged (Q1-5).
  - Deletion line: `mod world;`, the nine unused re-exports, the
    `#[cfg(test)]` re-export block, the `fixture_*` functions (A) and
    Dungeon's private principal struct are gone before the first `pub` is
    written.
- **Verification** (Windows workstation; proves Windows only):
  - builds: `cargo check -p ghostlight` (no new warnings; the 646
    dead-code warnings of an under-opened boundary must be 0),
    `cargo check -p ghostlight-dungeon --bin ghostlight-dungeon`,
    `cargo check -p ghostlight-dungeon --tests`,
    `cargo build --locked -p ghostlight-dungeon --bin ghostlight-dungeon`
    (proves `Cargo.lock`). Expected target-dir delta: one extra lib crate
    in debug; the scratch probe's check-only target was 2.3 GiB from empty,
    so expect well under 1 GiB incremental on the repo's 7.5 GiB.
  - tests: `cargo test -p ghostlight` = 420 unit + 10 doc (the 420 names
    equal the base `world::` list with the prefix stripped);
    `cargo test -p ghostlight-dungeon --bin ghostlight-dungeon` = 49
    (same names as base);
    `cargo test -p ghostlight-dungeon --test build_provenance` = 1;
    `cargo test -p ghostlight-persona-projection` = 13. Ignored counts
    unchanged (1 + 1). The reopen test from Cut 0 pins invariant 2
    (`world.cc` written at base reopens with equal revision and digests).
    The doc-tests pin invariant 1. The `VerifiedPrincipalEvidence::new`
    count pins the single minter. The split credential-name test pins
    "no credential name in the port's source" on both sides. Tests 1–4
    keep their names and assertions (test 4's cell count is 2).
  - negative:
    `cargo test -p ghostlight --all-features --doc` = 10 passed (same as
    without; the command is the standing check that no later feature
    reopens the seal);
    `rg -n '\[features\]' crates/ghostlight/Cargo.toml` = 0;
    `rg -n 'cfg\(any\(test' crates/ghostlight/src` = 0 (no feature-gated
    test path exists);
    `rg -n 'fixture_' crates/ghostlight-dungeon/src` = 0 outside the
    Heimdall `::fixture(` sites (`app_session.rs:560`, `runtime.rs:2552,
    2611,4126` — use `rg -n 'fixture_[a-z]' …` which they do not match);
    `rg -n 'crate::world::' crates` = 0;
    `rg -n '^mod world;' crates` = 0;
    `rg -n 'app_session|heimdall|eve::|idunn_health|axum|tower_http|jsonwebtoken|\bDungeon\b|Session Zero' crates/ghostlight/src` = 0
    (collisions checked: `Dungeons` in `vault.rs` is excluded by `\b`;
    do not add `player`);
    `rg -n 'pub\(super\)' crates/ghostlight/src/lib.rs` = 0;
    `rg -n 'VerifiedPrincipalEvidence::new\(' crates/ghostlight-dungeon/src`
    = 1 outside `#[cfg(test)]`;
    `cargo tree -p ghostlight` names no `axum`, `tower-http`,
    `jsonwebtoken`, `cultnet-rs`, `cultmesh-rs`, `reqwest`, `aes-gcm`,
    `hmac`, `rand`, `base64`, `libc`;
    `git diff -M --stat 02bbf62..HEAD -- crates` shows 13 renames.
  - operator: Idunn recipe run on Yggdrasil (Linux proof): the
    `test-world-library`, `test-dungeon`, `build-daemon` steps and the
    acceptance step resolve; `/srv/…/ghostlight-dungeon --state-root`
    reopens the live `world.cc`; health reports `consumer-v4`. One
    `GHOSTLIGHT_SMOKE_TRACE` road run after the cut shows the membrane
    trace still writes (test 5).
- **Operator questions:**
  - Q1-7 Port value constructors for consumer-implemented ports. Tests 1,
    2, 3 and 5 implement `InferencePort` outside the library and must
    return `PreparedInference`, `InferenceOutput`, `InferenceFault`; today
    they do it through `#[cfg(test)]` `fixture_*` wrappers that exist only
    because those types have private constructors. A: make the
    constructors the port contract — `PreparedInference::prepare(caller_
    runtime_id, expires_at_unix_ms, request)` (rename of the existing
    `prepare_invocation`, "the one place a `PreparedInference` is built",
    which every in-library port already calls), `InferenceOutput::new(events,
    receipt_digest)`, `InferenceFault::{retryable, recovery_required,
    integrity_violation}`, `InferenceRequest::provider_model`; delete the
    five `fixture_*` items; make `TracingInferencePort` an ungated `pub`
    decorator (it wraps any port and constructs nothing sealed; "production
    never constructs it" becomes a Dungeon rule, which is where it was
    enforced anyway — only the ignored smoke names it). B: keep every
    fixture `#[cfg(test)]` in the library and delete tests 1–3 and the
    `trace` field of `LiveController` (`runtime.rs:2538,3726`) from
    Dungeon; the permit-pool, quarantine, and interrupted-arm invariants of
    the tick driver lose their proofs and the road test loses its membrane
    trace. **Recommended: A.** A public trait whose methods must return a
    type nobody outside the crate can construct is not a port; it is a
    trait only the library can implement. Invariant 1 seals the kernel,
    not the inference transport, and none of these constructors mints an
    ID, mutates state, reduces, or writes the journal. A is a net
    deletion (about 85 fixture lines out, about 25 constructor lines in,
    of which `prepare` is a rename).
  - Q1-8 The cover for `drive_one_tick_runs_every_cell_before_the_clock_and_all_share_one_tick`.
    A: derive it from a real world through production ingress — the
    module's own `fixture()`/`active_two_cell_world`, then `snapshot()` and
    `agency_graph()` into `derive_cover`; delete
    `fixture_controller_opportunities` and the `FictionalMinutes(u64)`
    construction; the test asserts two cells instead of three, and the
    ordering and shared-tick assertions are unchanged. B: delete the test.
    C: keep `fixture_controller_opportunities` `#[cfg(test)]` in the
    library and move the test there — not available, `drive_one_tick` is
    Dungeon. **Recommended: A.** It removes the last caller of the sealed
    constructors from outside `world/`, uses only what tests 1–3 already
    use, and the test's own doc (`:3558-3565`) is about ordering, not
    about three.

## Cut 2. Build and deploy references in gamecult-ops

- **Repo/branch:** gamecult-ops, current default branch; one commit after
  Cut 1 is pushed.
- **First:** `git -C F:\Projects\gamecult-ops status` clean;
  `bash scripts/test-ghostlight-yggdrasil-wiring.sh` passes at base.
- **Deletes first:** none (Q1-6: retirement belongs to the deployment
  gate).
- **Keeps and moves:** none.
- **Adds:** none.
- **Per-file changes:**
  - `runbooks/ghostlight-dungeon-yggdrasil.md:249-251`: insert
    `cargo test --locked -p ghostlight` before the Dungeon test line;
    `:257` "one explicit Rust binary" stays true.
  - `scripts/deploy-ghostlight-yggdrasil.sh:964-970`: add the library test
    invocation in the same container run; `:1308`: acceptance becomes
    `-p ghostlight --lib controllers::tests::real_codex_connector_cognition_modes_commit_speech`.
  - `scripts/test-ghostlight-yggdrasil-wiring.sh:276-278`: add
    `require_text "${deploy}" 'cargo test --locked -p ghostlight'`.
  - `idunn/yggdrasil/bindings/ghostlight.toml.in`: unchanged; it names the
    recipe path, and the recipe carries the commands.
  - The scripts remain, per the Ghostlight handoff (`:389-391`), helpers
    not to invoke: they fail at the root/Idunn Git boundary and hold
    duplicate deployment authority. Editing them keeps their command list
    from lying about the crate graph until the deployment gate retires
    them; it is not an endorsement to run them.
- **Authority map:**
  - Owner: `deployment/idunn/recipe.toml` in Ghostlight owns the build and
    test commands (binding `:11 recipe_path`). The runbook describes; the
    scripts are a second, dormant list pinned to the recipe's strings by
    the wiring test until the deployment gate removes them.
  - Inputs: the admitted ref and the recipe.
  - Outputs: the same one binary plus `web/`.
  - Derived state: runbook text, wiring-test assertions.
  - Forbidden writers: no second list of cargo commands that drifts from
    the recipe; the wiring test's `require_text` lines are that check.
  - Shared paths: n/a.
  - Deletion line: none in this cut.
- **Verification:**
  - builds: none (shell and Markdown).
  - tests: `bash scripts/test-ghostlight-yggdrasil-wiring.sh` passes with
    the new `require_text`.
  - negative: `rg -n 'ghostlight-dungeon --bin ghostlight-dungeon' F:\Projects\gamecult-ops --glob '!docs/**'`
    lists only lines that also sit beside a `-p ghostlight` line.
  - operator: the next Idunn deploy of a post-Cut-1 commit runs the
    library step.
- **Operator questions:** none; Q2-1 was Q1-6 and is ruled.

## Subtraction ledger (estimate)

| Cut | Removed | Added | Deps / formats / targets |
|---|---|---|---|
| 0 | — | — | scratch captures only |
| 1 | `mod world;` (1), 9 unused re-exports (~3 lines), `cfg(test)` re-export block (12), Dungeon principal struct (38), Dungeon's direct persona-projection dep (1), `fixture_controller_opportunities` (26, Q1-8 A), inference fixtures (~60, Q1-7 A); ~140 lines | lib manifest (~28), workspace member (1), Dungeon dep (1), lock entry (~20), principal type in lib (~30), `pub use` boundary (~25), doc-tests (~30), runtime credential test (~25), minter count test (~15), port constructors (~25, Q1-7 A), recipe step (~6); ~205 lines. Visibility edits: 94 `pub(super)`→`pub(crate)`, ~150 `pub(crate)`→`pub` (items, fields, methods), 199 `crate::world::`→`crate::`, 32 `crate::world::`→`ghostlight::`; test 4 rewritten in place (~20 lines) | +1 lib crate, 0 features, 0 binaries, 0 new packages in `Cargo.lock`; 13 files renamed (~52k lines) |
| 2 | — | ~5 lines | 0 |

Net for L0: about +65 source lines, one crate, no feature. The growth is
boundary declaration (an implicit `pub(crate)` becomes an explicit `pub`
list plus its compile-fail proofs); the liability it retires is the
convention "Dungeon does not touch kernel internals", which becomes a
compile error with no feature that can lift it, and five fixture wrappers
that existed only to work around private port constructors.

## Findings not assigned to a cut

- `deployment/idunn/recipe.toml:142-146,169-171` declare the world slot and
  provided capability as `ghostlight.world_state.foundation.v0` /
  `foundation-v0`, and `:150-154` the controller-work slot as
  `ghostlight.controller_work.v3`; source is `consumer.v4`
  (`mod.rs:84-85`, tag `consumer-v4`) and `controller_work.v15`
  (`controllers.rs:75-76`). Pre-existing drift; L0 changes no schema
  string, so it stays.
- `tests/build_provenance.rs` is never run by the recipe or the scripts:
  `--bin ghostlight-dungeon` selects the bin's unit tests only. A
  `cargo test --locked -p ghostlight-dungeon --test build_provenance` step
  would be a recipe change with its own reason.
- `notes/ghostlight-current-system-map.md:18-19` still names the state
  owner as `ghostlight.world_state.foundation.v0`; `:28` says "the private
  `world::WorldKernel`". Both are stale before this cut; the steward owns
  the rewrite.
- `runtime.rs:2492` is a `CommitReceipt` literal in Dungeon tests; it is
  why `CommitReceipt`'s four fields are in the boundary at all. Under
  Q1-8 A the same literal serves `drive_one_tick`'s fake clock submitter.
- `PreparedInference.invocation` (`controllers.rs:131`,
  `CodexTransportInvocation` from `codex-connector`) stays `pub(crate)`. A
  consumer port that talks to a real transport would need it; no such
  consumer exists in L0, and opening it is a port-contract decision for
  the pass that brings one.
- `web/` references nothing in `crates/`; `sidecar/claude-sdk/test/*` is
  written by library tests under `GHOSTLIGHT_WRITE_SIDECAR_FIXTURES` and
  read otherwise — path depth unchanged, nothing to do.
