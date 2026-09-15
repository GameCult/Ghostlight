# Ghostlight library extraction — cut map (plan step 15, L0)

Status: cut map. Ends are owned by `ghostlight-library-extraction.md`; this
document owns the means. Written 2026-09-15 against `d83534c` on
`codex/ghostlight-dungeon-mvp`. Nothing landed.

Rulings (operator, 2026-09-15, carried from the target):
- Dungeon is a consumer of the library; no Dungeon concept, type, route, or
  dependency enters it.
- Kernel first; lenses, detail rules, evidence binding and Draft sessions
  are L1+.
- L0 moves code and changes no behavior, schema string, digest preimage, or
  store layout.
- Delete before adding; no re-export facade preserving old paths unless the
  map names the external contract it protects.

Open: Q1-1 through Q1-6 below. Q2-1 (gamecult-ops) waits on Q1-6.
Follow-ups outside this migration: see "Findings not assigned to a cut".

## What the probes established (scratch worktree at `d83534c`, removed)

Every mechanism claim below comes from one of these runs. Logs are in the
session scratchpad (`probe1*-check.log`, `probe3*-visibility.log`,
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
- **Base warning**: `mod.rs:28` re-exports nine names nobody in the crate
  uses (`ControllerOpenError`, `NarrativeCapture`, `NarrativeDecision`,
  `NarrativePending`, `OperationalCapture`, `OperationalDecision`,
  `OperationalPending`, `SourceRange`, `TranslationGapSummary`).
- **The move compiles.** Moving `src/world/*` to a new lib crate
  (`mod.rs` → `lib.rs`), rewriting `crate::world::` → `crate::` inside it and
  `crate::world::` → `<lib>::` in Dungeon, and moving
  `VerifiedPrincipalEvidence` into the library root: `cargo check` on the
  library, the library's tests, the Dungeon bin, and the Dungeon tests all
  pass (probe 1d). Two mechanical faults were needed: the 94 `pub(super)`
  sites in the former `mod.rs` are illegal at a crate root (become
  `pub(crate)`), and `consumer.rs:62` needs `serde_bytes` in the library
  manifest.
- **What Dungeon needs public** (probe 3, four compiler rounds starting from
  every item `pub(crate)`): 54 production items, 22 test-only items,
  38 fields on 10 structs, 36 methods, the 3 opaque IDs
  `AffordanceId`/`CommandId`/`SubjectId`, and the `FictionalMinutes(u64)`
  tuple constructor (one Dungeon test). Listed under Cut 1 "Adds". Nothing
  in the list is sealed: `WorldKernel`, `WorldState`, `reduce`,
  `apply_effect`, `CommandEnvelope`, `AuthenticatedCaller`, the journal,
  `*::issue()` and `ScopeDigest::fixture` never appear. Under the narrowed
  library `cargo check -p <lib>` emits 646 dead-code warnings: a second,
  independent enumeration of the same boundary (an item used only by Dungeon
  is dead inside the library until it is `pub`).
- **Cross-boundary test fixtures**: Dungeon's `runtime.rs` tests need
  exactly the `#[cfg(test)]` re-export block at `mod.rs:66-72` plus
  `fixture_controller_opportunities` (`mod.rs:490`),
  `TracingInferencePort` (`controllers.rs:278`),
  `InferenceFault::fixture_integrity_violation` / `fixture_recovery_required`
  (`controllers.rs:225,234`), and the ID `issue()` impls those fixtures call
  (`mod.rs:200-226`, `ScopeDigest::fixture` `mod.rs:448`). Gating all of
  them behind `#[cfg(any(test, feature = "test-support"))]` with the feature
  enabled only from Dungeon's `[dev-dependencies]` compiles (probe 1d). No
  Dungeon test reaches a sealed constructor: the only `::fixture(` calls
  outside `world/` are Heimdall's (`runtime.rs:2552,2611,4126`,
  `app_session.rs:560`) and `VerifiedPrincipalEvidence::fixture`
  (`runtime.rs:4313`).
- **Compile-fail proof**: eight `compile_fail` doc-tests on the library root
  (`use <lib>::WorldKernel`, `reduce`, `WorldState`, `CommandEnvelope`,
  `AuthenticatedCaller`, `journal::WorldJournal`, `SubjectId::issue()`,
  `fixture_controller_opportunities` without the feature) all pass under
  `cargo test -p <lib> --doc` (probe 4). Doc-tests compile as an external
  crate, which is exactly the boundary; no `trybuild`, no scratch crate.
- **Negative-grep collisions checked** in `src/world/`: `axum`,
  `tower_http`, `jwt`/`jsonwebtoken`, `heimdall`, `eve::`, `idunn_health`,
  `Session Zero` have zero legitimate hits. `Dungeon` collides with the vault
  fixture path `Spoilers/Dungeons/Provenance.md` (`vault.rs:489,581`): use
  `\bDungeon\b`. `player` collides with `tests::player()`
  (`consumer.rs:966`): do not publish it. `app_session` has 5 hits, all the
  principal seam, and must reach 0.
- **Line endings**: index and working tree are LF (`git ls-files --eol`);
  `core.autocrlf=true` rewrote a fresh worktree checkout to CRLF. Hands
  edits in `F:\Projects\Ghostlight` itself, not a new worktree, or verifies
  `--eol` after the move. `sdk_inference.rs:1831` normalizes `\r\n`.
- **Build host**: every probe ran on the Windows workstation with
  `cargo check`/`cargo test --doc`; it proves the crate graph and visibility
  on Windows only. The Linux proof is Idunn's recipe run on Yggdrasil.

## Cut 0. Captures

- **Repo/branch:** Ghostlight `codex/ghostlight-dungeon-mvp` at `d83534c`.
  No commit; output lives in the session scratchpad.
- **First:**
  - Test census: `cargo test -p ghostlight-dungeon --bin ghostlight-dungeon
    -- --list > base-tests.txt` (PowerShell `>` writes UTF-8 BOM + CRLF;
    compare post-cut with the same redirection and `Compare-Object` on
    trimmed lines after stripping the `world::` prefix). Expected 469.
  - A `world.cc` written at base. There is no existing store on disk
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
  `d83534c`. One commit; the crate graph does not build half-moved. Depends
  on Cut 0 and rulings Q1-1..Q1-5.
- **First:** Cut 0 captures in hand; `git status` clean.
- **Deletes first:**
  - `crates/ghostlight-dungeon/src/world/mod.rs:28-32` — the nine unused
    re-exported names above (leave `CellRun`, `ConnectorBinding`,
    `ControllerError`, `ControllerModels`, `ControllerPendingReason`,
    `ControllerRunner`, `ControllerWorkCustody`, `NarrativeRun`,
    `OperationalRun`, `SubmissionDisposition`, `open_controller_work`,
    `open_inference`).
  - `crates/ghostlight-dungeon/src/world/mod.rs:61-72` — the `#[cfg(test)]`
    re-export block (12 lines); it returns as the gated `pub use` under
    Adds.
  - `crates/ghostlight-dungeon/src/app_session.rs:54-91` — the
    `VerifiedPrincipalEvidence` struct, accessors, and `fixture` (38 lines).
  - `crates/ghostlight-dungeon/src/main.rs:7` — `mod world;`.
  - `crates/ghostlight-dungeon/Cargo.toml:27` — the direct
    `ghostlight-persona-projection` dependency (Dungeon no longer names the
    crate; `controllers.rs:35` is the only importer and it moves).
- **Keeps and moves:**
  - `git mv crates/ghostlight-dungeon/src/world/{action,clock,consumer,
    controllers,cover,elaboration,journal,mailbox,patch,sdk_inference,
    tool_schema,vault}.rs crates/<lib>/src/` and
    `git mv .../world/mod.rs crates/<lib>/src/lib.rs`. Verify with
    `git diff --cached -M --stat` that all 13 show as renames.
  - Inside the moved files: `crate::world::` → `crate::` (199 sites;
    `super::super::` paths need no change, tests are one module below the
    root as before); `pub(super)` → `pub(crate)` in `lib.rs` only (94
    sites; the submodules keep theirs);
    `crate::app_session::VerifiedPrincipalEvidence` → `crate::VerifiedPrincipalEvidence`
    (`mailbox.rs:10`, `controllers.rs:10714,10790,10988,11473`).
  - `build.rs`, `tests/build_provenance.rs`, `runtime.rs:375` (web/dist)
    stay in Dungeon; their `CARGO_MANIFEST_DIR/../..` still resolves.
    `sdk_inference.rs:1900,1983` move with the file and still resolve from
    `crates/<lib>` (same depth).
- **Adds:**
  - `Cargo.toml` (workspace): member `crates/<lib>`.
  - `crates/<lib>/Cargo.toml`: package `<lib>`; `[features] test-support = []`;
    dependencies `async-trait`, `chrono`, `codex-connector`, `cultcache-rs`,
    `ghostlight-persona-projection = { path = "../ghostlight-persona-projection" }`,
    `rmp-serde`, `serde`, `serde_bytes = "0.11"`, `serde_json`, `sha2`,
    `thiserror`, `tokio`, `tracing`, `uuid`, `zeroize` (all `.workspace =
    true` except `serde_bytes`, which the workspace does not declare); dev
    `tempfile = "3"`. Every one is already in `Cargo.lock`; no new package.
  - `crates/ghostlight-dungeon/Cargo.toml`: `<lib> = { path = "../<lib>" }`
    in dependencies; `<lib> = { path = "../<lib>", features = ["test-support"] }`
    in dev-dependencies.
  - `Cargo.lock`: the new `[[package]] name = "<lib>"` entry. Commit it;
    Idunn builds `--locked`.
  - `crates/<lib>/src/lib.rs`, root: `pub struct VerifiedPrincipalEvidence
    { account_subject_hash: String, valid_until: DateTime<Utc> }` with `pub fn
    new`, `pub fn account_subject_hash`, `pub fn valid_until`, and
    `#[cfg(test)] fn fixture` delegating to `new` (per Q1-3 A). Doc comment:
    the consumer verifies; the library bounds by `valid_until` at
    `mailbox.rs:231`.
  - `crates/<lib>/src/lib.rs`, the public boundary. Production `pub use`:
    `clock::{FictionalMinutes, TickMinutes}`;
    `consumer::{CONSUMER_BODY_LIMIT, CONSUMER_CREDENTIALS_ENVIRONMENT,
    CONSUMER_PATCH_SCHEMA, CONSUMER_RECEIPT_SCHEMA, ConsumerRegistry,
    admit_document, encode_receipt}`;
    `controllers::{CellRun, ConnectorBinding, ControllerError,
    ControllerModels, ControllerPendingReason, ControllerRunner,
    ControllerWorkCustody, NarrativeRun, OperationalRun,
    SubmissionDisposition, open_controller_work, open_inference}`;
    `cover::{AgencyGraph, Cell, Cover, CoverBudget, TickIndex, derive_cover}`;
    `elaboration::{SeedOutcome, select_row}`;
    `mailbox::{ConsumerPort, ControllerPort, MailboxError, SeedPort, WorldMailbox}`;
    `patch::{JurisdictionKey, Statement}`;
    `sdk_inference::{DEFAULT_SDK_MODEL_PREFIX, SdkBinding}`;
    `vault::VaultEvidenceSource`. Root items made `pub`: `STATE_SCHEMA`,
    `state_schema_compatibility_tag`, `AffordanceId`, `CommandId`,
    `SubjectId` (the `opaque_uuid!` macro at `mod.rs:98-106` emits
    `pub(crate) struct`; make the macro take a visibility or emit `pub` for
    these three only), `PrincipalId`, `WorldPhase`, `SubjectKind`,
    `ControllerMode`, `CreateWorldIntent`, `CreateJurisdictionIntent`,
    `DecisionOpportunity`, `DecisionInvocation`, `PrincipalCommandIntent`,
    `CommandBody`, `WorldSnapshot`, `OperatorEvent`, `CommitReceipt`,
    `SubmitReceipt`, `KernelError`, `ScaleDeficitRow`, and the snapshot
    row types Dungeon reads through `WorldSnapshot` fields
    (`SubjectSnapshot`, `PlaceSnapshot`, `AffordanceSnapshot`, and whichever
    others the compiler names; probe 3 stopped at the `WorldSnapshot` field
    layer). `GroupedRun` (`controllers.rs`) is reached through
    `ControllerRunner` return values and needs `pub` with its five fields.
  - Gated `#[cfg(any(test, feature = "test-support"))] pub use
    controllers::{ControllerWork, ControllerWorkLookup, ControllerWorkStore,
    ControllerWorkStoreError, ControllerWorkWrite, InferenceEvent,
    InferenceFault, InferenceOutput, InferencePort, InferencePurpose,
    InferenceRequest, PreparedInference, TracingInferencePort,
    fixture_inference_events, fixture_inference_output,
    fixture_prepared_inference}` and the same gate on
    `fixture_controller_opportunities` and on the seven `#[cfg(test)]`
    sites it and `TracingInferencePort` depend on (`mod.rs:200,207,214,221`
    ID issuance impls, `mod.rs:448` `ScopeDigest::fixture`,
    `controllers.rs:225,234,277,283,344,377,412,423`). `InferencePort` and
    `ControllerWorkStore` are ports; if Q1-4 rules them production API,
    move those two traits and their request/output types to the ungated
    list and leave only the `fixture_*` functions and `TracingInferencePort`
    gated.
  - Fields that become `pub` (38, from probe 3): `CommitReceipt.{command_id,
    commit_digest, resulting_revision, resulting_state_digest}`,
    `Constituent.subject`, `Cover.{cells, oversubscribed, tick}`,
    `CoverBudget.{cells, constituent_cap, urgency_slots}`,
    `DecisionOpportunity.{controller_mode, world_id}`, `GroupedRun.{cell,
    needs, pending, resolution, submissions}`, `OperatorEvent.{revision,
    speech}`, `PreparedInference.purpose`, `ScaleDeficitRow.{deficit,
    jurisdiction, kind, target}`, `WorldSnapshot.{affordances,
    draft_approvals, now, opportunities, owner, phase, places,
    required_approvers, revision, scale_deficit, subjects, title, world_id}`,
    plus `FictionalMinutes.0` or a `pub const fn new(u64)` for
    `runtime.rs:3579`. Fields of the row structs reached through
    `WorldSnapshot` follow in the next compiler round.
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
    `TracingInferencePort::new` (`:285`, gated), `sweep`
    (`elaboration.rs:607,1746`), `name` (`elaboration.rs:1511`),
    `Cover::{singletons, groups, validated}` (`cover.rs:163,170,198`),
    `ConsumerRegistry::{empty, from_secret_file}` (`consumer.rs:168,176`),
    `TickMinutes::{new, minutes}` (`clock.rs:45,51`), `CommandId::new`
    (`mod.rs:122`), `PrincipalId::new` (`mod.rs:239`), `Statement::new`
    (`patch.rs:281`).
  - `crates/<lib>/src/lib.rs` crate docs: the eight `compile_fail`
    doc-tests from probe 4, each pinned to its error code (`E0603` for the
    six private paths, `E0599` for `SubjectId::issue()`, unpinned for the
    feature-gated fixture whose code differs by feature).
  - `crates/ghostlight-dungeon/src/runtime.rs` (tests): a ~25-line copy of
    `soul_no_credential_name_appears_in_the_ports_own_source` scanning
    `runtime.rs` only, because the library test cannot read another
    crate's source (see per-file `sdk_inference.rs:1824-1828`).
  - `crates/ghostlight-dungeon/src/app_session.rs` (tests): one test
    asserting `VerifiedPrincipalEvidence::new(` occurs exactly once in the
    crate's production source, at `account_for_cookie` (same shape as the
    `from_secret_file` count at `sdk_inference.rs:1860`). This is the
    single-minter rule the deleted private fields used to enforce.
- **Per-file changes** (against `d83534c`):
  - `Cargo.toml:3-5`: add member.
  - `crates/ghostlight-dungeon/Cargo.toml:27`: replace with the library
    path dep; `:43-45`: add the dev-dep with `test-support`.
  - `crates/ghostlight-dungeon/src/main.rs:7`: delete.
  - `crates/ghostlight-dungeon/src/runtime.rs:3-22`: split the `use crate::{}`
    block; `world::{…}` becomes `use <lib>::{…}` (probe edit); `:4`: import
    `VerifiedPrincipalEvidence` from `<lib>`, not `app_session`.
    `:518,519,535,1084,1088,1092,1201,1325,1621,2492,2493,2581,2582,3081,
    3501,3568,3658,3666,3675,3677,3777,3782,4267`: `crate::world::` →
    `<lib>::`. `:4313`: `::fixture(` → `::new(`. `:987,1040,1189,1431,1777,
    2110`: type paths follow the import. `:504,2673` doc comments name
    `world::consumer`; reword to the crate.
  - `crates/ghostlight-dungeon/src/eve.rs:3-6` and `:182,335,338,354,386,
    405,449,494`: same rewrite.
  - `crates/ghostlight-dungeon/src/mesh.rs:6-9`: same.
  - `crates/ghostlight-dungeon/src/idunn_health.rs:8`: same.
  - `crates/ghostlight-dungeon/src/app_session.rs:54-91`: delete; `:6`: add
    `use <lib>::VerifiedPrincipalEvidence;`; `:232-235`: struct literal →
    `VerifiedPrincipalEvidence::new(session.account_subject_hash.clone(),
    session.access_expires_at)`.
  - `crates/<lib>/src/sdk_inference.rs:1824-1828`: the scanned file list
    becomes `["sdk_inference.rs", "controllers.rs"]` (paths are now
    crate-root relative); `:1860` `sources[1]` still indexes
    `controllers.rs`. The `runtime.rs` half moves to Dungeon (Adds).
  - `deployment/idunn/recipe.toml:14-17`: `test-world-owner` becomes two
    steps — `test-world-library`: `["cargo","test","--locked","-p","<lib>"]`
    (runs the 420 unit tests and the 8 doc-tests) and `test-dungeon`:
    the existing `-p ghostlight-dungeon --bin ghostlight-dungeon` line.
    `:51-53`: acceptance argv becomes `-p <lib> --lib
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
    build-budget sentence now names the library crate;
    `notes/ghostlight-current-system-map.md:18,28,289` [Mind steward];
    `notes/fresh-workspace-handoff.md:284-286` [steward];
    `state/map.yaml:57,389-390` [steward].
- **Authority map:**
  - Owner: the library crate `<lib>` owns the world kernel, its ontology,
    reducer, journal, mailbox, controllers, elaboration, cover, clock,
    consumer admission, inference ports, and evidence sources. Its public
    surface is the list under Adds and nothing else.
  - Inputs: `VerifiedPrincipalEvidence` values Dungeon constructs from
    `AppSessionOwner::account_for_cookie`; `CreateWorldIntent`,
    `PrincipalCommandIntent`, `DecisionInvocation`; consumer patch bytes via
    `admit_document`; store paths via `WorldMailbox::open` and
    `open_controller_work`; connector and SDK bindings via `open_inference`.
  - Outputs: immutable `WorldSnapshot`, `OperatorEvent`, `AgencyGraph`,
    typed receipts (`SubmitReceipt`, `CommitReceipt`, `CreationReceipt`
    through `MailboxError`/`KernelError`), controller runs, cover
    partitions, encoded consumer receipts.
  - Derived state: Dungeon's Eve projection, mesh documents, Idunn health
    (which reads `STATE_SCHEMA` and `state_schema_compatibility_tag` and
    derives, never restates), HTTP responses.
  - Forbidden writers: any path in Dungeon naming `WorldKernel`,
    `WorldState`, `reduce`, `apply_effect`, `CommandEnvelope`,
    `AuthenticatedCaller`, the journal, or an `issue()` — enforced by
    visibility, proven by the doc-tests. Any second minter of
    `VerifiedPrincipalEvidence` — enforced by the Dungeon count test.
    `test-support` in a production build — checked below.
  - Shared paths: Dungeon production and Dungeon tests reach the kernel
    through the same `pub` items; tests additionally see the gated fixture
    set. Library-internal tests keep `pub(crate)`/`super::` access as
    today. Consumers other than Dungeon: none yet; Epiphany consumes only
    `ghostlight-persona-projection`, which is unchanged.
  - Deletion line: `mod world;`, the nine unused re-exports, the
    `#[cfg(test)]` re-export block, and Dungeon's private principal struct
    are gone before the first `pub` is written.
- **Verification** (Windows workstation; proves Windows only):
  - builds: `cargo check -p <lib>` (no new warnings; the 396–646 dead-code
    warnings of an under-opened boundary must be 0),
    `cargo check -p ghostlight-dungeon --bin ghostlight-dungeon`,
    `cargo check -p ghostlight-dungeon --tests`,
    `cargo build --locked -p ghostlight-dungeon --bin ghostlight-dungeon`
    (proves `Cargo.lock`). Expected target-dir delta: one extra lib crate
    in debug; the scratch probe's check-only target was 2.3 GiB from empty,
    so expect well under 1 GiB incremental on the repo's 7.5 GiB.
  - tests: `cargo test -p <lib>` = 420 unit + 8 doc (the 420 names equal
    the base `world::` list with the prefix stripped);
    `cargo test -p ghostlight-dungeon --bin ghostlight-dungeon` = 49;
    `cargo test -p ghostlight-dungeon --test build_provenance` = 1;
    `cargo test -p ghostlight-persona-projection` = 13. Ignored counts
    unchanged (1 + 1). The reopen test from Cut 0 pins invariant 2
    (`world.cc` written at base reopens with equal revision and digests).
    The doc-tests pin invariant 1. The `VerifiedPrincipalEvidence::new`
    count pins the single minter. The split credential-name test pins
    "no credential name in the port's source" on both sides.
  - negative:
    `rg -n 'crate::world::' crates` = 0;
    `rg -n '^mod world;' crates` = 0;
    `rg -n 'app_session|heimdall|eve::|idunn_health|axum|tower_http|jsonwebtoken|\bDungeon\b|Session Zero' crates/<lib>/src` = 0
    (collisions checked: `Dungeons` in `vault.rs` is excluded by `\b`;
    do not add `player`);
    `rg -n 'pub\(super\)' crates/<lib>/src/lib.rs` = 0;
    `rg -n 'VerifiedPrincipalEvidence::new\(' crates/ghostlight-dungeon/src`
    = 1 outside `#[cfg(test)]`;
    `cargo check -p ghostlight-dungeon --bin ghostlight-dungeon -v 2>&1 | rg 'feature="test-support"'`
    = 0 (resolver 2 keeps dev-dep features out of the bin build);
    `cargo tree -p ghostlight-dungeon -i axum` shows no path through `<lib>`;
    `cargo tree -p <lib>` names no `axum`, `tower-http`, `jsonwebtoken`,
    `cultnet-rs`, `cultmesh-rs`, `reqwest`, `aes-gcm`, `hmac`, `rand`,
    `base64`, `libc`.
    `git diff -M --stat d83534c..HEAD -- crates` shows 13 renames.
  - operator: Idunn recipe run on Yggdrasil (Linux proof): the
    `test-world-library`, `test-dungeon`, `build-daemon` steps and the
    acceptance step resolve; `/srv/…/ghostlight-dungeon --state-root`
    reopens the live `world.cc`; health reports `consumer-v4`.
- **Operator questions:**
  - Q1-1 Library crate name. A: `ghostlight` (`crates/ghostlight`,
    `use ghostlight::WorldMailbox`); B: `ghostlight-world` (the probe name);
    C: `ghostlight-core`. **Recommended: A.** The target, the map, and the
    operator ruling all say "the Ghostlight library"; the crate should be
    what the prose calls it. The sibling `ghostlight-persona-projection` is a
    membrane published for Epiphany, not the library. The repo, workspace,
    and `package.json` names do not collide with a Cargo package name.
    Mechanically A and B are identical (probes used B).
  - Q1-2 `vault.rs` and `sdk_inference.rs`. A: both in the library;
    B: both in Dungeon behind the `EvidenceSource`/`InferencePort` traits;
    C: vault in the library, SDK port in Dungeon. **Recommended: A.**
    `open_inference` (`controllers.rs:2314`) constructs the SDK route
    itself, so B and C change how the port is built — a structure change
    L0 forbids. Neither file names a Dungeon concept: the vault takes a
    root and a scope, the port takes a sidecar entry. Both are named in the
    target's own library list (evidence sources, inference ports). The
    sidecar fixture writes (`sdk_inference.rs:1900,1983`) resolve from the
    library's manifest dir at the same depth. Revisit when a second
    consumer needs a different evidence source or transport.
  - Q1-3 Principal evidence seam. A: library-owned
    `VerifiedPrincipalEvidence` with a public `new`; Dungeon's
    `account_for_cookie` is the single production caller, proven by a count
    test. B: library trait `PrincipalEvidence { account_subject_hash,
    valid_until }`; Dungeon keeps its private-field struct and implements
    it; `SeedPort` (`mailbox.rs:590`) stores `Arc<dyn PrincipalEvidence>`
    and the controllers' seed tests gain a fixture impl. **Recommended: A.**
    It is what invariant 3 of the target says ("a library-owned type that
    Dungeon constructs"), it is four edit sites, and the probe is green. The
    minter seal was module privacy inside one crate; across a crate
    boundary the library cannot know who a principal is, so the rule lives
    with its owner (Dungeon) as a test, not as a trait the library carries
    for one consumer.
  - Q1-4 Cross-boundary fixtures. A: `test-support` cargo feature on the
    library gating the 22 items and the fixture-only `issue()` impls,
    enabled only by Dungeon's dev-dependency. B: move the runtime tests
    into the library — not possible, they drive `run_cover_tick` and
    `drive_one_tick`, which are Dungeon. C: make the four `fixture_*`
    functions ungated `pub` — puts fixture-shaped `DecisionOpportunity`
    minting in the production API. **Recommended: A**, with the port traits
    (`InferencePort`, `ControllerWorkStore`) and their request/output types
    in the ungated production API rather than behind the feature: they are
    the "runner and port entry points" invariant 1 names, `open_inference`
    already returns `Arc<dyn InferencePort>`, and hiding a trait a public
    function returns is a `private_interfaces` smell. Gate only
    `fixture_inference_*`, `fixture_prepared_inference`,
    `fixture_controller_opportunities`, `InferenceFault::fixture_*`,
    `TracingInferencePort`, and the `issue()`/`ScopeDigest::fixture` impls.
    Invariant 7 admits exactly this feature.
  - Q1-5 `ghostlight-persona-projection`. A: stays a separate crate, the
    library depends on it by path as Dungeon does today. B: fold into the
    library. **Recommended: A.** Epiphany pins it by git rev `22281891` with
    `package = "ghostlight-persona-projection"`, which resolves against that
    revision regardless of HEAD, so either option keeps the pin today; but
    folding would make Epiphany's next bump pull the world library (tokio,
    cultcache, codex-connector) for three prompt builders. Wrong dependency
    direction; its own recipe test step stays.
  - Q1-6 `gamecult-ops` deploy scripts — see Cut 2.

## Cut 2. Build and deploy references in gamecult-ops

- **Repo/branch:** gamecult-ops, current default branch; one commit after
  Cut 1 is pushed. Depends on Q1-1 (the crate name) and Q1-6.
- **First:** `git -C F:\Projects\gamecult-ops status` clean;
  `bash scripts/test-ghostlight-yggdrasil-wiring.sh` passes at base.
- **Deletes first:** per Q1-6 B only: `scripts/deploy-ghostlight-yggdrasil.sh`
  (1300+ lines) and `scripts/test-ghostlight-yggdrasil-wiring.sh`
  (`:276-278,282,343` are the only Ghostlight cargo assertions).
- **Keeps and moves:** none.
- **Adds:** none.
- **Per-file changes:**
  - `runbooks/ghostlight-dungeon-yggdrasil.md:249-251`: insert
    `cargo test --locked -p <lib>` before the Dungeon test line; `:257` "one
    explicit Rust binary" stays true.
  - Per Q1-6 A: `scripts/deploy-ghostlight-yggdrasil.sh:964-970` add the
    library test invocation in the same container run; `:1308` acceptance
    becomes `-p <lib> --lib controllers::tests::real_codex_connector_cognition_modes_commit_speech`;
    `scripts/test-ghostlight-yggdrasil-wiring.sh:276-278` add
    `require_text "${deploy}" 'cargo test --locked -p <lib>'`.
  - `idunn/yggdrasil/bindings/ghostlight.toml.in`: unchanged; it names the
    recipe path, and the recipe carries the commands.
- **Authority map:**
  - Owner: `deployment/idunn/recipe.toml` in Ghostlight owns the build and
    test commands (binding `:11 recipe_path`). The runbook describes; the
    scripts either actuate (A) or are retired (B).
  - Inputs: the admitted ref and the recipe.
  - Outputs: the same one binary plus `web/`.
  - Derived state: runbook text, wiring-test assertions.
  - Forbidden writers: no second list of cargo commands that can drift
    from the recipe — under A the wiring test pins the script to the
    recipe's strings; under B there is no second list.
  - Shared paths: n/a.
  - Deletion line: under B, both scripts before the runbook edit.
- **Verification:**
  - builds: none (shell and Markdown).
  - tests: `bash scripts/test-ghostlight-yggdrasil-wiring.sh` (A) passes
    with the new `require_text`; (B) the script is gone and nothing
    references it (`rg -n 'test-ghostlight-yggdrasil-wiring|deploy-ghostlight-yggdrasil' F:\Projects\gamecult-ops` = 0
    outside `docs/repo-census-2026-09`, which is an archive).
  - negative: `rg -n 'ghostlight-dungeon --bin ghostlight-dungeon' F:\Projects\gamecult-ops --glob '!docs/**'`
    lists only lines that also sit beside a `-p <lib>` line.
  - operator: the next Idunn deploy of a post-Cut-1 commit runs the
    library step; before that, the operator confirms which actuator is live
    on Yggdrasil (`systemctl cat ghostlight-dungeon.service` names the
    unit's origin; the v2 binding lives under `/var/lib/gamecult/idunn-v2`).
- **Operator questions:**
  - Q2-1 (= Q1-6) Edit or retire the scripts. A: edit both (five lines);
    B: delete both and point the runbook at the recipe; C: leave them.
    **Recommended: A.** The Body is ambiguous: the runbook (`:243-251`)
    still says `request-idunn-bounded-redeploy-yggdrasil` runs those exact
    cargo lines, the Idunn v2 binding template points at the recipe
    instead, and the Ghostlight handoff (`notes/fresh-workspace-handoff.md:133`)
    says Yggdrasil still serves legacy release `a4080d4`. No Ghostlight
    doc calls the scripts legacy helpers (`rg` finds no mention of either
    script name in the Ghostlight repo), so that claim is unverified.
    Retiring a deploy actuator is a deployment-authority decision outside
    L0; an edited dead script costs five lines, an unedited live script
    would test only Dungeon's 49 tests and silently skip the 420 kernel
    tests. C is the one option that leaves the release path able to lie.

## Subtraction ledger (estimate)

| Cut | Removed | Added | Deps / formats / targets |
|---|---|---|---|
| 0 | — | — | scratch captures only |
| 1 | `mod world;` (1), 9 unused re-exports (~3 lines), `cfg(test)` re-export block (12), Dungeon principal struct (38), Dungeon's direct persona-projection dep (1); ~55 lines | lib manifest (~30), workspace member (1), Dungeon deps (2), lock entry (~20), principal type in lib (~30), `pub use` boundary (~20), doc-tests (~25), runtime credential test (~25), minter count test (~15), recipe step (~6); ~175 lines. Visibility edits: 94 `pub(super)`→`pub(crate)`, ~130 `pub(crate)`→`pub` (items, fields, methods), 199 `crate::world::`→`crate::`, 32 `crate::world::`→`<lib>::` | +1 lib crate, +1 feature (`test-support`), 0 binaries, 0 new packages in `Cargo.lock`; 13 files renamed (~52k lines) |
| 2 | (B) ~1600 lines of shell | (A) ~5 lines | 0 |

Net for L0: about +120 source lines, one crate, one feature. The line
growth is boundary declaration (what was implicit `pub(crate)` becomes an
explicit `pub` list plus its compile-fail proofs); the liability it retires
is the convention "Dungeon does not touch kernel internals", which becomes a
compile error.

## Findings not assigned to a cut

- `deployment/idunn/recipe.toml:142-146,169-171` declare the world slot and
  provided capability as `ghostlight.world_state.foundation.v0` /
  `foundation-v0`, and `:150-154` the controller-work slot as
  `ghostlight.controller_work.v3`; source is `consumer.v4`
  (`mod.rs:84-85`, tag `consumer-v4`) and `controller_work.v15`
  (`controllers.rs:75-76`). Pre-existing drift; L0 changes no schema
  string, so it stays. The census flagged the v3/v4 half already.
- `tests/build_provenance.rs` is never run by the recipe or the scripts:
  `--bin ghostlight-dungeon` selects the bin's unit tests only. A
  `cargo test --locked -p ghostlight-dungeon --test build_provenance` step
  would be a recipe change with its own reason.
- `notes/ghostlight-current-system-map.md:18-19` still names the state
  owner as `ghostlight.world_state.foundation.v0`; `:28` says "the private
  `world::WorldKernel`". Both are stale before this cut; the steward owns
  the rewrite.
- `runtime.rs:2492` is a `CommitReceipt` literal in Dungeon tests; after
  the cut it needs the four `pub` fields listed, which is why
  `CommitReceipt` is in the boundary at all. If Hands prefers not to expose
  receipt fields for one test, the alternative is a library fixture behind
  `test-support`; either is behavior-neutral.
- `web/` references nothing in `crates/`; `sidecar/claude-sdk/test/*` is
  written by library tests under `GHOSTLIGHT_WRITE_SIDECAR_FIXTURES` and
  read otherwise — path depth unchanged, nothing to do.
