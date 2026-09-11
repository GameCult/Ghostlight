# Local Live Smoke

The one place the road is tested: the seed lane, the production tick driver,
elaboration sweep, and clock against a real CodexConnector on a created
world, for a configured number of seed sessions and ticks. Everything else
proves the machine under fixture inference ports. The harness is the ignored
test
`runtime::tests::live_smoke_seeds_then_ticks_a_world_against_the_connector`;
its log is the deliverable.

## Substrate

Machine-local, outside the repo: `F:\Projects\Ghostlight-smoke\`.

- `codex-home\auth.json`: a copy of `~/.codex/auth.json`. The connector spawns
  the official `codex app-server` child against this home and may refresh it.
- `connector\ghostlight.key`: 64 hex characters, no trailing newline; the
  shared connection secret. Both sides read the same file.
- `connector\connector.cc`: the daemon's single-caller config.
- `logs\`: connector stdout/stderr and pid, smoke logs.

The connector binary is `F:\Projects\CodexConnector\target\debug\codex-connector.exe`
built from the repo head with the `daemon` feature. Ghostlight's `Cargo.lock`
pins the connector library at an older revision whose daemon does not build
against its own lockfile; the head daemon speaks the same wire law, proven by
the handshake in the run below.

## Bring-up

Initialize the config once (the codex executable path and SHA-256 are the
newest `codex.exe` under `%LOCALAPPDATA%\OpenAI\Codex\bin\`):

```powershell
F:\Projects\CodexConnector\target\debug\codex-connector.exe --initialize-single-caller-config F:\Projects\Ghostlight-smoke\connector\connector.cc 127.0.0.1:4103 <codex.exe> <sha256> F:\Projects\Ghostlight-smoke\codex-home F:\Projects\Ghostlight-smoke\connector\replay.cc ghostlight-smoke F:\Projects\Ghostlight-smoke\connector\ghostlight.key 1 gpt-5.6-luna,gpt-5.6-sol,gpt-5.6-terra 4 1052672 16000
```

Start the daemon detached; it prints one ready line to stderr:

```powershell
$R='F:\Projects\Ghostlight-smoke'; $p = Start-Process -FilePath 'F:\Projects\CodexConnector\target\debug\codex-connector.exe' -ArgumentList @('--config', "$R\connector\connector.cc") -RedirectStandardOutput "$R\logs\connector.stdout.log" -RedirectStandardError "$R\logs\connector.stderr.log" -PassThru -WindowStyle Hidden; $p.Id | Out-File "$R\logs\connector.pid"
```

Stop it with `Stop-Process -Id (Get-Content F:\Projects\Ghostlight-smoke\logs\connector.pid)`.

## Run

```bash
export GHOSTLIGHT_CONTROLLER_CONNECTOR=127.0.0.1:4103 \
  GHOSTLIGHT_CONTROLLER_CREDENTIAL='F:\Projects\Ghostlight-smoke\connector\ghostlight.key' \
  GHOSTLIGHT_ACCEPTANCE_RUNTIME_ID=ghostlight-smoke \
  GHOSTLIGHT_CONTROLLER_PROJECTOR_MODEL=gpt-5.6-luna \
  GHOSTLIGHT_CONTROLLER_PERSONA_MODEL=gpt-5.6-terra \
  GHOSTLIGHT_CONTROLLER_INTERPRETER_MODEL=gpt-5.6-terra \
  GHOSTLIGHT_CONTROLLER_OPERATIONAL_MODEL=gpt-5.6-terra \
  GHOSTLIGHT_CONTROLLER_ELABORATOR_MODEL=gpt-5.6-terra \
  GHOSTLIGHT_SMOKE_TICKS=3 \
  GHOSTLIGHT_SMOKE_LOG='F:\Projects\Ghostlight-smoke\logs\smoke-ticks.log' \
  GHOSTLIGHT_SEED_VAULT_ROOT='F:\Projects\Kalsa\Kalsa' \
  GHOSTLIGHT_SMOKE_VAULT_SCOPE='Public' \
  GHOSTLIGHT_SMOKE_SEED_SESSIONS=4 \
  GHOSTLIGHT_SMOKE_SEED_TARGET=6 \
  GHOSTLIGHT_SMOKE_SEED_ROOT_LABEL='Low Sere' \
  GHOSTLIGHT_SMOKE_SEED_BRIEF='A dry basin town that owes its water to the gate above it.'
cargo test -p ghostlight-dungeon --bin ghostlight-dungeon live_smoke_seeds_then_ticks -- --ignored --nocapture
```

`GHOSTLIGHT_SEED_VAULT_ROOT` is required; the runner refuses to seed with it
unset. `GHOSTLIGHT_SMOKE_VAULT_SCOPE` and `GHOSTLIGHT_SMOKE_SEED_BRIEF` are
optional; the brief is the world's brief, set at creation and projected to
every lane as guidance, and the world is titled by the seed root label unless
`GHOSTLIGHT_SMOKE_WORLD_TITLE` names it. The scope is a directory under the root; Kalsa's spoiler split is
that scope, so a world that must not see `Spoilers` is seeded from `Public`.
The root label must name a place the Vault knows; Low Sere has a Public note. The caller runtime id must equal the one the
connector config admits.

## First run, 2026-09-05, before the seed lane

Genesis: Active, three subjects (Operator, Persona, Operational Agent) in
`commons`, three opportunities, no boundaries, no deficit rows (no scale
intent at genesis).

| Tick | Wall time | Cells | Revision | Persona speech |
|---|---|---|---|---|
| 1 | 71 s | 2 singletons | 2 → 5 | "I am here." |
| 2 | 41 s | 2 singletons | 5 → 8 | "I am here. Time 60." |
| 3 | 44 s | 2 singletons | 8 → 11 | "Is anyone there?" |

Each tick committed one narrative command, one operational command, and one
clock advance; the elaboration sweep returned clean every tick because the
world derives nothing to answer. The store's final digest is in the log.

What this proves: the connector handshake, the Persona membrane end to end
(projector, persona, interpreter), the operational lane, the cover, the tick
driver, the clock, and the elaboration sweep all work on the road. What it
does not prove: anything about a world with structure. The prose is thin
because the world is three subjects in one room with one affordance; that is
the seed-producer gap, not a cognition fault.

The older ignored test `real_codex_connector_cognition_modes_commit_speech`
in `world/controllers.rs` predates pass 6 and declares its subjects unplaced,
so it fails at `NoAudience` before reaching the provider; retire or place it.

## Seeded run, 2026-09-05

Kalsa `Public` scope, root Low Sere, target six persons, brief "A dry basin
town that owes its water to the gate above it."

| Stage | Wall time | Result |
|---|---|---|
| Seed session 1 | 39 s | one round, committed: six persons in Low Sere, all qualified, deficit 6 → 0; session 2 skipped, no shortfall left |
| Activate | | nine subjects, nine opportunities, six boundaries for the elaborator |
| Tick 1 | 95 s | 8 singleton cells, revision 3 → 7, Bren Ash: "I'm here." |
| Tick 2 | 95 s | 8 singleton cells, revision 7 → 11, Mara Dene: "Bren?" |
| Tick 3 | 123 s | 8 singleton cells, revision 11 → 15, Tovin Rusk: "Bren?", the Persona asking for a beginning |

Twelve runs were needed to get here. Every failure was on the road, not in
the kernel, and each became a rule the tree now carries:

- The copied Codex credential diverged from the live session and its refresh
  token was invalidated; the connector's Codex home is now the operator's
  real `~/.codex`, one credential family.
- Strict function schemas refuse `oneOf` and a `const` without a `type`; the
  emitter uses `anyOf` and types every tag, and a test walks every tool of
  both authoring lanes against the strict rules offline.
- `gpt-5.6-sol` is not available to a ChatGPT account; the Persona runs on
  `gpt-5.6-terra` here.
- A seed generation exceeded the five-minute socket read; the connector's
  expiry skew stays at five minutes while the response read timeout is
  fifteen.
- The model referenced existing things by label; the brief prints every
  canonical id beside its label.
- One tool call per response spent the six-round budget; the seed lane has
  its own budget of 24, requests parallel tool calls, and submits the draft
  as authored when the budget ends instead of discarding it.
- A repair round re-used the first attempt's provider request id and the
  connector refused it as a replay conflict; request ids are content-addressed.
- Obligations carried a period; the tool description states the commitment
  shape rule the resolver enforces.
- The first landed seed put all six people in the commons; the brief states
  that a subject must stand at the row's root or inside it to count.

The harness fails when no seed patch lands, so a run that reaches the
provider and authors nothing is a failed run. Silence is not failure: a
person given a place, a debt, and a clock and nothing else says nothing, and
since run 6 the prompts render that honestly instead of borrowing a voice.

## Claude SDK route

Run; see "SDK runs, 2026-09-10" below. `world/sdk_inference.rs`'s `SdkInferencePort` gives the same
ignored smoke a second transport: a lane whose configured model is
`claude`-prefixed (matching `GHOSTLIGHT_SDK_MODEL_PREFIX`, default `claude`)
routes to a Node sidecar instead of the CodexConnector, so the road can run
on the Claude subscription with no Codex budget. `LiveController` and
`ControllerRunner::open` (through `open_inference`) already carry this
optional binding; only the operator side and a real run are outstanding.

**Operator prerequisites, none of which Ghostlight performs:**

1. Install the Claude Code CLI.
2. Sign in with `/login`, or mint a token with `claude setup-token`.
3. Verify the login with one plain `claude -p` query.

The harness never reads, copies, forwards, or logs that credential; the
sidecar inherits it from the ambient environment the same way the CLI does.

**Build the sidecar**, once the CLI login is verified:

```bash
npm --prefix sidecar/claude-sdk install
npm run sdk:build
```

`npm run sdk:build` is the root `package.json` alias for
`npm --prefix sidecar/claude-sdk run build`, emitting
`sidecar/claude-sdk/dist/main.js`.

**Environment**, in addition to the connector variables above:

```bash
GHOSTLIGHT_SDK_SIDECAR='F:\Projects\Ghostlight\sidecar\claude-sdk\dist\main.js'
GHOSTLIGHT_SDK_MODEL_PREFIX=claude   # optional; this is already the default
```

`GHOSTLIGHT_SDK_SIDECAR`'s absence means no SDK binding at all — a
`claude`-prefixed lane model then fails at open with `UnroutableModel`
rather than silently reaching the connector. Any of the five
`GHOSTLIGHT_CONTROLLER_*_MODEL` variables can be set to a `claude`-prefixed
model (for example `claude-opus-5`) to route that lane through the sidecar;
the connector variables (`GHOSTLIGHT_CONTROLLER_CONNECTOR`,
`GHOSTLIGHT_CONTROLLER_CREDENTIAL`) are only required for the lanes that
still route to the connector, and an all-`claude` configuration needs
neither.

The harness is the same ignored test,
`runtime::tests::live_smoke_seeds_then_ticks_a_world_against_the_connector`;
its `#[ignore]` reason names both prerequisites this route adds: "requires a
vault plus either a running CodexConnector or a built Claude SDK sidecar
with an ambient Claude Code login; see `GHOSTLIGHT_SMOKE_*` and
`GHOSTLIGHT_SDK_*` environment."

### SDK runs, 2026-09-10

Every lane on `claude-sonnet-5`, Kalsa `Public`, Low Sere, target six, two
seed sessions, three ticks. The operator installed the CLI and logged in
interactively; the sidecar inherited the login. No connector variables were
set.

| Run | Seed | Ticks | Result |
|---|---|---|---|
| 1 | both sessions refused: subjects with no grant, twelve commitments due at or before the clock | 3 committed, 2 cells each, Persona spoke | harness failed on the empty seed; the transport itself worked end to end |
| 2 | one session, one round, 89 s: six qualified persons, deficit 6 to 0 | 456 s, 395 s, 326 s; 8 singleton cells each; revision 3 to 24 | passed; tick 3 quarantined one cell |
| 3 | one session, one round, 109 s: six persons and an institution, deficit 6 to 0 | two ticks, 230 s and 138 s; 9 singleton cells each; revision 3 to 15 | passed; no quarantine; speech cited by quote arrives whole |
| 4 | one round, 87 s | two ticks, 219 s and 235 s; 8 cells | passed; Projector now carries the subject's own life; five people say "Fourteen days", the due on their obligation |
| 5 | one round | two ticks, 316 s and 251 s; 9 cells; whole membrane traced (`GHOSTLIGHT_SMOKE_TRACE`) | passed; first exchange: Sera Venn "Maro first, then the rest.", Maro Seln "First for what, Sera?"; every voice was the model's |
| 6 | one round, 90 s | two ticks, 299 s and 235 s; 9 cells; flat prompts | passed; eighteen cells, nobody spoke; the honest baseline of an unauthored person |
| 7 | refused: six distinct people with memories, reads, and worded promises, every `declare_subject` carrying `"controller": {}` | 2 ticks on genesis alone | harness failed on the empty seed; the repair continuation faulted on the SDK lane (one user turn) |
| 8 | one round, 97 s: six people, each with a voice, values, memories, reads, and a promise in words | two ticks, 191 s and 261 s; 8 cells; 43 speak captures to 9 gaps | passed; seven distinct lines about the grate, the roll, and the crew |
| 9 | one round, 94 s; world titled Low Sere with its brief on every lane | two ticks, 218 s and 209 s; 8 cells; 20 speak captures to 24 gaps | passed; no fixture name anywhere in the trace; promises and speech in the world's own terms (the grate, the ration boards, the kiln on stored ash) |

Rules run 1 imposed are in the implementation plan under step 11 (descriptions
carried through the sidecar, `due` described and the clock printed, the grant
rule on the subject tool, raw tool calls logged, refusal continues the
conversation). Run 2's two findings: three of eight speech captures were cut
mid-word ("just t", "a?\" I") because the Interpreter cited byte spans the
model counted wrong, and the tick-3 quarantine was a mis-spanned utterance
failing the canonical-text rule at invocation, which is a translation gap
wearing an infrastructure fault's coat. Run 3, after speech and gaps moved to
`source_quote`, shows every utterance whole and no quarantine. Ticks are slow
because singleton cells run in sequence at roughly 15 to 50 s per cell on this
lane.

Runs 4 to 6 read the membrane itself. Set `GHOSTLIGHT_SMOKE_TRACE` to a file
and every request is appended with its purpose, model, instructions, input
turns, tool catalog, and output, so a cell can be read end to end: Projector
in and out, Persona in and out, Interpreter in and out. Run 5's trace showed
the machine working as designed (the Interpreter captured only quoted speech
and recorded honest gaps for posture, private priority, and provisional
belief) and two faults of context, not of prose: a `Commitment` carries no
statement of what is promised, so `due: 1000` was read as a thousand owed;
and the ontology's `PersonaMaterial` (values, voice, memories, reads) is
named in the doc and built nowhere, so every person spoke in the model's one
voice. Run 6, after the prompts were flattened to render only what the
context carries, is silent, which is correct. Run 6's trace also shows the
Projector saying "no one before me" for two people in the same house: the
context carries the subject's own place and not who else stands in it.

Runs 7 and 8 are plan step 12 on the road. Run 7 authored the best seed so
far and lost it to a field with no description and a tool result that said
only "recorded as a gap"; the controller variant now describes its shapes,
a decode failure's reason goes back to the model, and the SDK lowering
carries a later user turn (the refusal) in its transcript. Run 8 is the
first run whose people are people: Maro "Sera. Where's the crew.", Sera "Not
yet, Bren. I haven't given Maro the condition of the intake, and until I do,
I won't say send them down.", Iva keeping the roll of names before anyone
goes below. Speech captures went from one per tick to twenty-one, and gaps
from most of the prose to nine. One promise in run 8 read "I promise Maro
Seln I will mend the tick fixture by the appointed hour, though Maro Seln
holds no authority to command my compliance and no forum will hear a suit":
the world was still titled "Cover Tick Fixture" and the brief's boundary
language leaked into a vow. Run 9, after `world_create.v3` gave the world
its title and brief, has no fixture name in its trace and its promises read
"I want every one of the forty-one hearths accounted for on the ration
boards, with no favorites".

## Interrupted cell

Not yet run. The next road run must show a subject whose room changed mid-turn
getting one re-lowering, not a lost turn: its scope digest moves between the
Persona's prose and the commit, the runner re-lowers the same prose once
through the Interpreter against the fresh opportunity, and the resulting
`ghostlight.persona_turn_receipt.v3` row carries `interrupted_from` pointing
at the binding it replaced. The log line for that cell must carry the subject
and both scope digests — the one the turn was bound to and the one it was
re-lowered against — and the operator log must show the act once, not once
per lowering. This has not been exercised against a live provider; the eight
tests landed for step 9 all run against fixture ports.
