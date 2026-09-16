# Ghostlight stock lenses (L1) postmortem

Written 2026-09-16. Evidence: the cut map (`ghostlight-stock-lenses-cut.md`),
the target (`ghostlight-stock-lenses.md`), and the commits on
`codex/ghostlight-dungeon-mvp` from `817c9f4` to `872ba35`, plus the Cut 5 doc
commits.

## Summary

Elaboration in the `ghostlight` library now has a flavor the world's creator
controls:
- eight stock lenses;
- per-world weights in world state, replaceable by the owner;
- a replayable per-session draw that records the lens and its instruction
  text on the session;
- concurrent sessions under their own permit ceiling.

In-flight elaborator work is rediscovered by reading the store.
Scale: 12 crate files, +4,907 / −266; four schema strings replaced
(`consumer.v5`, `controller_work.v16`, `world_create.v4`, and the commit
schema); one environment variable; one required trait method; no dependency,
crate or binary.

State at writing: landed and pushed, Windows-verified. The Linux release is
unexercised until Idunn builds it. The operator owes rulings on L1.f1
(quarantine handling), L1.f13 (seed-lane orphans), L1.f14 (row retirement)
and L1.f16 (consumer lens sets). None blocks L2.

## Scope and invariants

The target set eight invariants: lenses never decide admission, every lens
sees the whole catalog, the draw is replayable, weights are admitted honestly,
concurrent sessions stay distinct, the lens is recorded and survives resume,
stores refuse rather than migrate, and no Dungeon concept enters the library.
All eight hold. Excluded: detail rules, evidence binding, Draft sessions
(L2–L4), and Dungeon's Session Zero surface (D1–D3).

## Timeline

| Cut | Commits | Soul passes | Notable finding |
|---|---|---|---|
| Map | `ff766e8` (with rulings Q1–Q9) | — | Two sessions on one answer collided; a boundary repair was Superseded forever after any commit |
| 1 lens module | `835ea4d` | 1 | No external crash path; duplicate keys last-wins; guard and divisor diverged |
| 2 weights, command | `d06fd7e`, `10be98c`; fixes `afca403`, `acb9b33` | 2 | Identical weights committed a no-op; forged row caught by the digest, not the rule; a padded twin still committed |
| 3 session, store, adoption | `dfb3c52..2bf8164`; fixes `f63dfd1..2c3b513` | 1 | Flaky pin (3/40); the Q7 resume rule untested; the call-site draw unpinned |
| 4 concurrency, pools, rediscovery | `8538fab..3e35b3d`; fixes `6f35229..0764842`, `635123f..872ba35` | 3 | The adoption rule on one test; speak-lane pool untested; a panic killed the driver; the retryable-no-stop leg unpinned |
| 5 docs | `39b2af4`, `04fd004`, `b15ade4` | 1 | The ontology claimed seed requests and Vault evidence for the Active sweep |

## Structural delta

The estimate grew from about +900 to about +1,300 as rulings added the owner
command and rediscovery. The actual was +4,907 / −266 in crates, overwhelmingly
test code (`controllers.rs` +2,807). Production grew by the lens module (379
lines including its tests), weights and the command in the kernel,
session fields and adoption in elaboration, the concurrent sweep and
`SessionPermit`, and the runtime's second pool. Deleted: the sequential
sweep, `step`, `Clean`, `Inactive`, the instruction constant as integrity
reference, whole-session supersession equality, and a proven-dead digest
comparison. Nothing parked.

## What Soul caught

- **Untested operator rulings.** L1-Q7's whole purpose, that a resumed session
  keeps its stored text when a lens's text changes, had no test: rebuilding the
  text from the current lens passed everything. The operator ruled it; nothing
  pinned it.
- **Flaky tests passing by luck.** A test rewrote a session's lens to a fixed
  value while its fixture drew one at random. It failed about one run in eight,
  and Hands' green run was luck.
- **Rules that no longer bear weight.** After Cut 4 routed resume through the
  listing, Cut 3's adoption rule was guarded by a single test. Cut 3's own
  former killers no longer reached it.
- **Semantic equality by spelling.** The no-op check compared raw maps, so
  `{charter:3}` and `{patina:0, charter:3}`, which draw identically, still
  committed revisions both ways.
- **Tests catching the wrong thing.** A forged all-zero journal row was
  refused by a digest mismatch, so weakening the draw rule changed nothing.
- **Failure paths.** One panicking session task unwound out of the sweep,
  aborted its sibling, and silently ended the runtime's elaboration driver
  until restart.
- **Spec errors, caught before they shipped.** Identical weights "still a
  commit"; a Dungeon payload mutation that could never fail; the `NoEffect`
  variant the plan names but the kernel never had.

## Operator corrections

- **"Are you relying on some deterministic code outputting the same id rather
  than just checking the state?"** This was the campaign's best question. It
  came from Self having told the operator the lens "has to be recorded" in the
  command identity. Answering it from the Body exposed the pre-existing
  defect that the elaborator repair loop had never worked across sweeps (L1.f10),
  and led to L1-Q9 (resume reads state). Missing context: Self framed the
  recording question and the identity question as one.
- **Weights and Draft approvals.** Self recommended binding approvals to a
  digest that included lens weights. The operator asked how weights could ever
  alter an approval. They cannot: weights shape future elaboration, and the
  owner may change them in any phase. Missing context: the probe that found the
  approval gap happened to use weights.
- **Proportion.** "Dear Lord, we don't need even more machinery" (L1-Q5), and
  the pools as "a ceiling, not a budget" (L1-Q4). Both shrank designs that Self
  had presented with more options than they needed.

## Incidents

- **Agents ending their turn to wait.** Three times, an agent launched cargo
  detached and ended its turn "to wait", and the work stopped until resumed by
  hand. The brief already said to wait for long jobs. What fixed it was naming
  how: block inside a tool call, with cargo in the foreground or `Wait-Process`
  on the pid. Recorded in the Eureka changelog.
- **Line endings.** The working tree mixed LF and CRLF files over an LF index.
  A naive newline anchor silently matched nothing, and `git apply -R`
  rewrote an LF file to CRLF. Rule: detect each file's ending, require every
  anchor to match exactly once, restore by reverse edit only.
- **A Soul probe left an empty untracked `lib.rs`.** Its permission to delete
  was refused. Found in the report, verified, removed at close-out.
- **A map refresh raced an operator message.** Imagination finished before a
  second ruling arrived, so the map contradicted two rulings until it was
  resumed. Self held the commit until the map was refreshed.

## What worked

- Probing the operator's challenge instead of defending the design: the
  stranding probe turned a question into a fixed pre-existing defect.
- Measuring flakes before and after (3/40, then 120/120) instead of rerunning
  until green.
- Folding small, related findings into the next cut's pass instead of a
  separate Hands and Soul cycle, with each still in its own commit.
- Skipping Soul on a test-only batch with an empty production diff, and
  having the next Soul pass rerun its mutations.
- Derived status: no "Open:" line was ever hand-edited; questions were open
  exactly while they lacked a ruling.

## What to change in the pipeline

- Tell agents how to wait, not only to wait (Eureka changelog, 2026-09-16).
- Treat any test built on a random draw or identity as requiring a loop.
- When the operator asks why a mechanism works, answer from the Body with a
  probe before defending the design.

## Open follow-ups

| Item | Owner | Where | Why it can wait |
|---|---|---|---|
| L1.f1: quarantine-class elaboration errors never set quarantine, and are logged only at debug (c4.s2.f4: a panic in `persist` would wedge the lane silently) | operator | `runtime.rs` elaboration driver | Pre-existing; the lane keeps running |
| L1.f11: the browser create form never lifts bindings into the payload | operator ruled: D2 | `eve-browser-lowering`, `runtime.rs` | Fixed with the Session Zero surface |
| L1.f12: Draft approvals bound to nothing | operator ruled: deferred to multi-player | `lib.rs` `draft_approvals` | Approvals are the owner's own today |
| L1.f13: the seed lane orphans rows like deficits did | operator | `elaboration.rs` seed runner | Outside L1 |
| L1.f14: stored rows are never retired, including the L1-Q10 crash window | operator | controller work store | Growth bounded by world lifetime |
| L1.f15: `Uncovered` pinned only with at most one named root | Library | `elaboration.rs` | Behaviour is correct; the case is untested |
| L1.f16: consumer-supplied lens sets | operator | `lens.rs` | No consumer needs one yet |
| c3.s1.f3 cost: stored instruction text is not tied to the recorded lens | accepted with L1-Q7 | `elaboration.rs` | Only the local store file can write it |
| Linux release | operator via Idunn | Yggdrasil | Needs the deployment gate |
