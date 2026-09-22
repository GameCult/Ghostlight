# Connector And Claude SDK Sidecar Bring-Up

No live smoke exists in this tree. The ignored test this file once documented,
`runtime::tests::live_smoke_seeds_then_ticks_a_world_against_the_connector`,
ran the production tick driver and the Active elaboration sweep against a real
CodexConnector; the play agent's Cut 1 (`d69e9d4`) deleted both, and the test
with them. A command that names it now matches nothing and exits 0. No
replacement exists until the play agent's own gate (`ghostlight-play-agent.md`)
lands one.

What remains true and stays here: bringing up a real CodexConnector and a
built Claude SDK sidecar. The seed lane (`world.seed`) still runs real
inference through `ControllerRunner`, and both transports below are how a
session gets real model responses instead of a fixture inference port.

## Substrate

Machine-local, outside the repo: `F:\Projects\Ghostlight-smoke\`.

- `codex-home\auth.json`: a copy of `~/.codex/auth.json`. The connector spawns
  the official `codex app-server` child against this home and may refresh it.
- `connector\ghostlight.key`: 64 hex characters, no trailing newline; the
  shared connection secret. Both sides read the same file.
- `connector\connector.cc`: the daemon's single-caller config.
- `logs\`: connector stdout/stderr and pid.

The connector binary is `F:\Projects\CodexConnector\target\debug\codex-connector.exe`
built from the repo head with the `daemon` feature. Ghostlight's `Cargo.lock`
pins the connector library at an older revision whose daemon does not build
against its own lockfile; the head daemon speaks the same wire law.

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

A session that wants the connector reads
`GHOSTLIGHT_CONTROLLER_CONNECTOR=127.0.0.1:4103` and
`GHOSTLIGHT_CONTROLLER_CREDENTIAL='F:\Projects\Ghostlight-smoke\connector\ghostlight.key'`,
the same variables the deleted harness read.

## Claude SDK route

`world/sdk_inference.rs`'s `SdkInferencePort` gives any lane whose configured
model is `claude`-prefixed (matching `GHOSTLIGHT_SDK_MODEL_PREFIX`, default
`claude`) a second transport: a Node sidecar on the Claude Agent SDK instead of
the CodexConnector, so a session can run on the Claude subscription with no
Codex budget.

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
`claude`-prefixed lane model then fails at open with `UnroutableModel` rather
than silently reaching the connector. Any of the five
`GHOSTLIGHT_CONTROLLER_*_MODEL` variables can be set to a `claude`-prefixed
model (for example `claude-opus-5`) to route that lane through the sidecar;
the connector variables are only required for lanes that still route to the
connector, and an all-`claude` configuration needs neither.
