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
optional. The scope is a directory under the root; Kalsa's spoiler split is
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

The harness fails when no seed patch lands or nobody speaks, so a run that
reaches the provider and gets nothing back is a failed run.

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
