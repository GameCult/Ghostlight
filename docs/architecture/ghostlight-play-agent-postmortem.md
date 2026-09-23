# Play agent postmortem

Written 2026-09-23 from the cut map, the commits, the Soul reports and an Eyes
pass over all three. The cut map
(`docs/architecture/ghostlight-play-agent-cut.md`) remains the detailed record:
every cut's status, findings PA.f1 to PA.f195, and every ruling.

## Summary

Ghostlight Dungeon now runs on one operational agent. It reads the world through
an omniscient table view, calls kernel tools directly as the Play authority,
dispatches characters for their own narrative turns, asks the player questions,
and closes each turn with narration. `world.speak` and the story card are gone;
`world.play` and a single player card replace them.

Seventeen cuts and about ten fix batches landed over two days. 195 findings were
recorded. The kernel gained a Play authority, ruled facts, minting, retirement,
affordance grants and a callable Persona turn. A loopback-only local inference
port landed so the whole thing can run against a small model on the operator's
own machine.

State at writing: the loop is built and verified as far as tests and a falsifier
reach. **Nobody has played it, and no model has ever run behind the local port.**
The seed path has never executed past its configuration check. All evidence is
Windows; the release evidence is Idunn's Linux gate on Yggdrasil.

Two capabilities the design promises do not exist: fiction-first risk resolution
and gestalt agency. They are named in *Open follow-ups* and carried into the
target doc as gaps rather than promises.

## Scope and invariants

The pivot: the previous machine was built for offline narrative generation and
world simulation, which is the wrong set of compromises for play. The Persona
sandwich stays, because actors should think narratively; everything else — turn
dispatch, interpretation, prompting the player, negotiating the result — becomes
one operational agent's job.

Eight invariants scoped the work. The kernel is the only writer and no
transcript crosses turns; Personas never see structured state, and the agent
never rewrites an existing character; an actor acts only through its own
affordance; speech is verbatim, checked against that actor's own prose; there is
one play authority; removal is retirement; one loop serves every backend; and
the player sees a projection, the question, and the refusal of their own act,
and nothing else.

Explicitly out of scope: the offline simulation lanes, which keep total
interpretation; Njordr's evidence invariants, which the operator ruled do not
bind the play agent ("if the agent can invent people, it can invent loot"); and
the world compiler.

## Timeline

| Cut | What | Commits | Notable finding |
|---|---|---|---|
| 1 | Delete the autonomous drivers and owner controls | `d69e9d4` | −2,184 lines. A re-added tick loop passed 46 of 46 tests |
| 2 | The local inference port | `182f803` + fixes | `HTTP_PROXY` sent the whole request, and a credential, to a proxy (PA.f16) |
| 3 | Play authority, ruled facts, minting | `4c4973b` | The genesis lane never ran `require_ruler` (PA.f26) |
| 4 | Retirement | `3725ba8` | A retired subject was still an audience and a raw speaker (PA.f33) |
| 5 | Grant and revoke affordances | `732b1d3` | Two loosenings survived every test (PA.f30, PA.f35) |
| 6 | The Persona lane and the recency marker | `c4334cc` | The spec's own stated rationale was impossible; Hands corrected the spec |
| 7 | The table vocabulary and inference seams | `7d40393` | `actor_tools` emitted names `decode_actor_call` refused (PA.f55) |
| 8a | The play table and the turn loop | `6b69270` | `communicate` let the agent make any actor "tell" an invented fact (PA.f61) |
| 8a-fix2 | | `74b628d` | Every mutation reported killed; eight rules unguarded |
| 8a-fix3 | | `754c7fe` | The id spelling had four copies; changing the printer passed 735 tests and broke every dispatch (PA.f113) |
| 8a-fix4 | | seven commits | Not ready for 8b: a crash window stranded a dispatched Persona (PA.f125) |
| 8b | Wiring, routing, the Eve schema | `1b20a0f`, `a99fdfe` | `ask_player` wedged the world: no client could build an answer (PA.f134) |
| 9 | The play surface; `world.speak` deleted | `322bdf6`, `f1732c0` | The real client could not submit anything at all |
| 10 | The client round trip | `d9f0159`, `43b6af6` | `world.create` had no field for `brief`: no client could ever create a world |
| 11 | The answer path and the gate | `4155823` | Two of three bridge tests failed instead of skipping without node |
| 12–13 | The store schema; the answer token | `7052644`, `2f2a8b6` | An answer composed for one question resolved another (PA.f170) |
| 14 | Preflight fixes | `58324f8`, `0808c6a` | The seed refusal handed the player the inference endpoint's URL (PA.f187) |
| 15 | The closing fixes | `f295a3a` … `1dea8f5` | An unwritten invariant, stated and broken in one commit (PA.f193) |
| 16–17 | Closing the bar | `63964b0` … `07aae19` | The runbook's weekly check printed `ok` without running (PA.f194-B) |

## Structural delta

New: `local_inference.rs`, `table.rs` (the consumer-neutral vocabulary),
`play.rs` (the turn loop), `tools/eve_client_bridge.mjs` and its committed
fixture. Two dependencies: `reqwest` for the local port, `unicode-segmentation`
for UAX #29 word boundaries.

Deleted: Cut 1's tick driver, Active elaborator and owner "let X act" path
(−2,184 lines, and ten tests whose only subject was deleted code); Cut 9's
`SpeakPayload`, `world.speak` arm, story card, speak control and
`current_operator_view`.

Tests: the library went 466 → 622 and has been flat since the play loop's third
fix batch — all later churn is Dungeon-side. The Dungeon binary went 56 → 46
(Cut 1's deletion) → 182, plus four client-bridge tests that are `#[ignore]`d
behind an opt-in. `ghostlight-persona-projection` went 13 → 35.

The subtraction budget was met once, at Cut 1, and not thereafter. Every fix
batch was net additive, mostly in tests. Soul audited the largest claim (Cut 13's
"~150 lines") and found 29 lines of production code; the rest was comments. That
is defensible, but the ledger stopped being a pressure after Cut 1 and nobody
noticed until this postmortem.

## What Soul caught

Of roughly 195 findings, about 150 were introduced by this pass's own earlier
steps rather than inherited. That is the honest shape of a foundation change:
each fix is a fresh chance to be wrong, which is why the falsifier never grades
its own work.

**Split authority (~14).** Four copies of one id spelling, because the library's
printer was private to its module (PA.f113). Two computations of the retired set
and of the exercise revision (PA.f82). Two owners for round entry (PA.f127). The
resume path's collision check duplicated verbatim (PA.f118).

**Tests that pinned spelling or passed trivially (~13).** A dependency check
asserting `contains("0")`, always true. A test that failed one run in four and
checked the wrong subject when it passed. Two refusal tests whose expected kind
was a substring of their own fixture's handle.

**Mutations aimed at the wrong layer (~4).** Deleting an in-memory record
instead of the on-disk write it stood for; testing a helper instead of its call
site; every test using a fresh key, so deriving the turn id from the key
survived.

**Inputs built from a copy of production logic (2).** The dispatch tests spelled
their subject id with a fourth copy of the printer, so changing what the world
view actually printed left 735 tests green while every dispatch an agent could
make was refused.

**Forged input accepted (~6).** A forged opportunity, a forged revision, a
deserialized colliding subject id, and a ruled fact riding in through world
creation.

**Player-visible leaks (~10).** The question id rendered as a visible, editable
box. A connection-refused error handing the player the inference endpoint's host
and port. A subject census on a card any authenticated account could see.

**Lifecycle and crash windows (~11).** A fresh request key discarding an open
turn; an empty answer wedging it forever; a spent round budget leaving it
running with nothing said; a crash between persisting a Persona's prose and
recording the call that produced it, which stranded that character for the rest
of the turn.

**Things the real client could never do (~5).** The binding shape did not match
what the client sends, so every command submitted an empty payload — including
`world.create`, which also had no field for a value its own contract required.
Nobody had ever driven the real client end to end.

## Operator corrections

Eight questions were ruled by the operator directly, and one correction changed
a design mid-pass. Self had suggested updating an existing character's attitude
through `set_persona_material`; that collapses the no-puppets rule, and the
ruling was reversed so the agent authors new characters and never rewrites an
existing one. The operator's review — *"I saw those calls in your traces, I've
been watching. No objections"* — arrived after the reversal rather than before
it.

The operator also cut a scope error at the root. Self proposed binding the play
agent to the offline lanes' evidence invariants; the ruling was *"if the agent
can invent people, it can invent loot"*, which is what made ruled facts and
minting coherent rather than a hole in the model.

What made the wrong answers look right in both cases: the agent was reasoning
from the library's offline discipline, where those rules are correct, without
noticing that the play path has a different author and a different purpose.

## Incidents

- **The workstation rebooted mid-pipeline** under concurrent cargo builds, mine
  among several sessions'. Rule: one cargo process at a time on the workstation,
  in every brief, and verification moves to Yggdrasil under Idunn.
- **A Hands agent amended and force-pushed** over its own first push of Cut 3.
  Nothing was lost, because it had rebased first. Rule: never amend, never
  force-push; it is now in every brief and in the skill.
- **A Soul pass died on an API 529.** Its worktree was cleaned and the pass
  rerun.
- **A `git pull --rebase` was refused** while Hands had unstaged work in the
  shared tree. Rule: never pull in a tree where Hands is working. (Observed in
  session; not recorded in the cut map.)
- **PowerShell map edits failed** on an overload error and on mismatched
  anchors. Rule: Python scripts that assert each anchor occurs exactly once and
  preserve CRLF. (Observed in session.)
- **A stray `node_modules`** left by an agent's own debugging made a test count
  report success for tests that could not run on a clean checkout (PA.f162).

## What worked

- **Quoting the cut inline in the brief** instead of pointing at a 3,900-line
  map. The one run that read the whole map reached ~400k tokens and produced the
  pass's sloppiest claims.
- **Hands stopping on a real fork.** Cut 9's counter had no honest derivation;
  Hands traced why, stopped, and reported. The ruling took one message and the
  cut resumed correctly.
- **Soul driving the real client.** Every blocker in the last third of the pass
  came from running the vendored browser lowering over the served surface rather
  than from reading it.
- **Recording a finding as closed-not-fixed when the mutation proved the code
  unreachable.** Hands built a guard, mutated it, found an existing guard already
  refused, reverted its own work and kept the test. The mutation decided, not the
  diff.

## What to change in the pipeline

Three lessons are already in the Eureka brief and changelog:

1. **Mutate the production call site, at the layer the rule protects** — a later
   write can hide a deleted one (`fd01545`).
2. **A test's inputs come from the production path**, never from a helper that
   re-spells what production prints (`2e0ea97`).
3. **Name the inference, not the line** (`181e819`). One defect shape ran through
   five instances: a committed file exists, therefore the client works;
   `git` exited zero, therefore this is a checkout; the format check failed,
   therefore the row is unreadable; the schema is unknown, therefore the row
   cannot be read; cargo printed `ok`, therefore the fixture is live. Each local
   fact true, each inference false, each fix scoped to the reported instance. And
   after changing how a test reports, run the owning runbook's commands verbatim
   and read what they print: that surface is the one a test suite structurally
   cannot check.

Two more this pass earns:

4. **A subtraction ledger that stops being updated stops being pressure.** It
   held at Cut 1 and then went unexamined for sixteen cuts.
5. **Soul's closing pass should be briefed for a walk-through, not only a
   verdict.** Asking what happens when a person actually sits down and plays
   produced the most useful output of the pipeline, including two defects no
   invariant covered.

## Open follow-ups

| Item | Where | Why it can wait |
|---|---|---|
| Fiction-first risk resolution | kernel + `play.rs` | The kernel resolves a risky act through an affordance's weighted outcome bands, drawn deterministically. There is no assessment step, no modifiers, and the agent cannot author affordances — so an act no granted verb expresses is authored directly as a ruled consequence with no draw at all. Those two paths are indistinguishable to the player. This is the next pipeline. |
| Gestalt and institution agency | kernel | Institutions exist as structure — offices, grants, jurisdiction — and constrain who may act. Nothing makes them act. Advancing time rolls routines and accumulates pressure; nobody deliberates off-screen. |
| The seed leg | `runtime.rs`, vault | Never executed past its configuration check. `GHOSTLIGHT_SEED_VAULT_ROOT` appears in no runbook. Attempt it alone, first. |
| PA.f194-A's fourth exit | `play.rs` | Closed in Cut 17, listed because its rule now lives on `retire_unreadable_row` as a precondition and every future caller must satisfy it. |
| PA.f195-B, PA.f195-C | `play.rs`, `runtime.rs` | A dead tuple element, and a skip notice cargo swallows without `--nocapture`. Trivial. |
| `state/map.yaml` is not parseable YAML | `state/` | Latent: nothing reads it programmatically. Fixing it is a deliberate sweep, not a drive-by. |
| A read path for a retired row | `play.rs` | The sidecar is JSON with the payload in base64, so an operator can read it. Nothing imports one back. |
