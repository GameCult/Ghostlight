# Local Live Smoke

The one place the road is tested: the production tick driver, elaboration
sweep, and clock against a real CodexConnector on a genesis world, for a
configured number of ticks. Everything else proves the machine under fixture
inference ports. The harness is the ignored test
`runtime::tests::live_smoke_ticks_a_genesis_world_against_the_connector`; its
log is the deliverable.

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
  GHOSTLIGHT_CONTROLLER_PERSONA_MODEL=gpt-5.6-sol \
  GHOSTLIGHT_CONTROLLER_INTERPRETER_MODEL=gpt-5.6-terra \
  GHOSTLIGHT_CONTROLLER_OPERATIONAL_MODEL=gpt-5.6-terra \
  GHOSTLIGHT_CONTROLLER_ELABORATOR_MODEL=gpt-5.6-terra \
  GHOSTLIGHT_SMOKE_TICKS=3 \
  GHOSTLIGHT_SMOKE_LOG='F:\Projects\Ghostlight-smoke\logs\smoke-ticks.log'
cargo test -p ghostlight-dungeon --bin ghostlight-dungeon live_smoke_ticks -- --ignored --nocapture
```

The caller runtime id must equal the one the connector config admits.

## First run, 2026-09-05

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
