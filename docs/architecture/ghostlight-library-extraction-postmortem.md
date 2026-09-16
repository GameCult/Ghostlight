# Ghostlight library extraction (L0) postmortem

Written 2026-09-16. Evidence: the cut map
(`ghostlight-library-extraction-cut.md`), the target
(`ghostlight-library-extraction.md`), commits in Ghostlight and gamecult-ops,
and the Soul reports recorded in the map.

## Summary

The world kernel moved out of the `ghostlight-dungeon` binary into the
`ghostlight` library crate, and Dungeon became a consumer across a public
boundary. Thirteen files (about 52k lines) moved by rename with no content
change. The boundary is sealed by admission: external code may build a
syntactically valid ID, digest or opportunity, and the kernel rejects every
one it did not issue, proven by rejection tests written as an external crate.
gamecult-ops' build commands run the library tests they would otherwise have
skipped.

State at writing: landed and pushed in both repos, Windows-verified. The Linux
release is Idunn's recipe run on Yggdrasil and has not been exercised; no
Windows result proves it. The operator owes nothing for L0.

## Scope and invariants

The target set seven invariants: sealed kernel at the crate boundary, no
behavior or schema change, no Dungeon concept in the library, dependencies
pointing one way, a working release path, unaffected external pins, and one
new crate with no feature matrix. Excluded: every library capability of plan
step 15 L1–L4, schema renames, the network path for outside consumers, and
deployment cutover.

Invariant 1 changed meaning during the migration (ruling Q1-9). It began as
"sealed items are unreachable from outside" and ended as "the kernel rejects
anything it did not issue", because the first reading was not true of the
code and could not be made true without a design change L0 excluded.

## Timeline

| Cut | Repo, commits | Soul passes | Notable finding |
|---|---|---|---|
| Map | Ghostlight `02bbf62`, `6ddabca` | — | Classifying the fixture tests found no kernel test to move; Q1-7 and Q1-8 followed |
| 1 | Ghostlight `108b691..f859d2c` | 1 | F1: public types deriving `Deserialize` are constructible from outside; the compile_fail tests pin names only |
| 2 | gamecult-ops `6aba281` | 1 | The specified assertion was vacuous (substring match); the acceptance path had an unlisted fourth site |
| 2 fix | gamecult-ops `803f2c7` | 1 | Exact-line pins prove presence, not execution (`if false`, `true \|\|`, heredoc) |
| Q1-9 | Ghostlight `7c1d1e9` | — | Operator ruled unforgeable admission |
| 1 fix | Ghostlight `a8d3377..2865ea7` | 1 | F3's premise was wrong; forgeries differed from real digests in length |
| S batch | Ghostlight `bde05de..af9c95a` | 1 | Case-insensitive and hex-only comparisons survived |
| T batch | Ghostlight `b14507c`, `ab95d78` | 1 | None |

## Structural delta

| Area | Estimate | Actual |
|---|---|---|
| Crate sources, including in-module tests | about +65 net | +1202 / −992, +210 net |
| Integration tests | not planned | +479 (`crates/ghostlight/tests/external_admission.rs`) |
| Manifests and lock | about +50 | +50 / −2 |
| Ghostlight recipe | about +6 | +10 / −5 |
| gamecult-ops | about +5 | +61 / −9 |

The source overrun has two causes. The single-minter scanner in
`crates/ghostlight-dungeon/src/app_session.rs` grew about 110 lines chasing
Soul's evasions, and it is still evadable (S4). The public boundary needed
roughly 90 more items than the map enumerated, each forced by a compile error
or a `private_interfaces` warning. The integration tests were not in the
estimate because Q1-9, which requires them, was ruled after it.

Deleted: `mod world;`, nine unused re-exports, the `cfg(test)` re-export
block, five inference fixtures, `fixture_controller_opportunities`, and
Dungeon's own principal evidence struct. Added: one library crate, a public
`pub use` boundary, port constructors, ten compile_fail doc-tests, the
external admission tests, and two Dungeon soul tests. Parked: nothing.
Features added: none. Binaries added: none.

## What Soul caught

- **Untested rulings.** Ruling Q1-4 said sealed constructors must be
  unreachable under every configuration. The code satisfied the letter
  (no feature gate) and missed the intent: `Deserialize` is a public
  constructor. Ten green compile_fail tests could not see it, and adding an
  ID-minting `Default` impl left all ten green. It would have failed as a
  false guarantee in any later review that trusted the target doc.
- **Tests pinning spelling, not behavior.** Every text scanner lost to a short
  evasion: the minter count to an alias, a subdirectory, a type alias and a
  function value; the gamecult-ops wiring test to a two-line deletion that
  reattached the pinned line to the wrong container; its fix to control flow
  around exact lines. Each would have let a real regression through with a
  green check.
- **Forgery shape.** Rejection tests that used short fake digests passed
  against a kernel comparing only length, only a prefix, all but the last
  character, case-insensitively, or only the hex after the label. Each would
  have admitted a digest the kernel never issued.
- **Spec errors.** The map's vacuous `require_text`, its unlisted acceptance
  site, its miscount of Dungeon tests, and Soul's own F3 premise (the two
  accessors it called uncalled are called) were all wrong, and all were
  caught before landing a fix built on them.
- **Unmeasured claims.** "No new package in `Cargo.lock`", "zero `fixture_`
  hits", "a guard that never fired", and a 13-warning ceiling nobody could
  reproduce.

## Operator corrections

- **Session Zero is Dungeon authority.** The first design placed a player
  flow on the kernel. Missing context: no document named which features
  belonged to the library and which to a consumer.
- **Focus was too narrow.** Focus as a set of subjects became a typed detail
  rule, a function from place to detail level. Missing context: Delvehold's
  constraints as a consumer; the design generalized from Dungeon alone.
- **Extract first.** The library capabilities were designed before the crate
  they belong to existed.
- **The titled elaborators** had been cut without an operator decision; the
  cut conflated a verifier with the lenses it verified.
- **Proportion.** Asked how much Q1-10 mattered, the operator said not at
  all. It was a tripwire against the project's own code, and Self should have
  defaulted it instead of posing it.

## Incidents

- **A Hands agent returned its private to-do list as its report** while its
  own background cargo job held the locks. Cost: one resume round. Rule:
  briefs now say to wait for long jobs and never end a turn with a to-do list.
- **A mutation run was invalidated by `git checkout`** reverting uncommitted
  fix code. Cost: two mutations rerun. Rule: run mutations against committed
  code, or restore with a diff rather than a checkout.
- **Cached no-op builds reported zero warnings.** Cargo replays warnings only
  when it rebuilds. Rule: measure warnings from a forced rebuild, and compare
  distinct messages by `cmp`.
- **A commit message described code replaced before commit** (`fac90e6`).
  Left in history; recorded in the map.
- **A pre-existing `cargo fmt --check` failure** across the library, found
  in passing.

## What worked

- Classifying tests before moving them. Q1-4's "move the tests" had nothing
  to move, and the classification turned a vague ruling into two concrete
  ones.
- Soul building an external scratch crate to break the seal. The serde route
  was invisible from inside the crate and to every doc-test.
- Holding map edits until Soul finished, so every Soul pass read a clean tree.
- Having Hands report spec discrepancies instead of redesigning: the vacuous
  assertion, the fourth acceptance site, the recovery_required collision.
- Parallel Hands in separate repos only when the second did not depend on the
  first landing.

## What to change in the pipeline

- **Brief Soul to attack constructibility, not names, on any sealed boundary.**
  Evidence: F1 on Cut 1, found only because the root agent added the serde
  route to the brief by hand.
- **Treat a text scanner as a tripwire with stated limits, never as proof of a
  semantic property.** Prefer a semantic tool (Clippy lints, the type system)
  or record the property as unenforced. Evidence: every scanner in L0 lost.
- **Rejection tests must use forgeries shaped like real values.** Evidence:
  S1 and the two survivors after it.
- **Default proportionate questions instead of asking.** A fork that guards
  only against the project's own code is a default with a recorded follow-up.
  Evidence: Q1-10.

## Open follow-ups

| Item | Owner | Where | Why it can wait |
|---|---|---|---|
| S4 minter scanner evadable; Clippy `disallowed-methods` recorded as the fix | Dungeon | `crates/ghostlight-dungeon/src/app_session.rs:523-668` | Guards only against the project's own code |
| S5 credential-name test splits on one literal marker | Dungeon | `crates/ghostlight-dungeon/src/runtime.rs:3584` | Same class as S4, pre-existing |
| S6 stable rustc does not enforce compile_fail error codes | Library | `crates/ghostlight/src/lib.rs:15-57` | Admission is proven by the external tests; codes need a snapshot tool |
| S2 world_id check redundant, S3 decline path unreachable from outside | Library | `lib.rs:4193`, `lib.rs:1603` | Redundant by construction and crate-private respectively |
| `ScopeDigest`, `WorkLane`, `ScopeComponents` public through public enums | Library | `KernelError`, `ControllerWorkCustody`, `NarrativeCheckpoint` | Closing them reshapes public enums, a design change |
| `cargo fmt --check` fails across the library | Library | `crates/ghostlight` | Pre-existing; formatting only |
| Recipe schema drift (`foundation.v0`, `controller_work.v3` against `consumer.v4`, `controller_work.v15`) | Deployment gate | `deployment/idunn/recipe.toml` | Changes no behavior in L0 |
| `build_provenance` not run by the recipe | Deployment gate | `deployment/idunn/recipe.toml` | A recipe change with its own reason |
| Redeploy of a sealed commit runs no tests; exact-line pins cannot see control flow; `set -e` unpinned; duplicate blocks pass; web container and Connector steps pinned by substring; 18-line option window | Deployment gate | `gamecult-ops/scripts/deploy-ghostlight-yggdrasil.sh`, `test-ghostlight-yggdrasil-wiring.sh` | Legacy actuator under a do-not-invoke warning, retired at the gate |
| Linux release unverified | Operator, via Idunn | Yggdrasil | Needs the deployment gate |
