# Ghostlight Verse: Map

Campaign `ghostlight-verse`, target `ghostlight-verse:target:r2`
(`notes/ghostlight-verse-target.md`). This page holds body facts, the model
page and rationale only. Cut specs, questions, rulings and follow-ups live in
the Eureka mind; query them there, never copy them here.

Phase 1 map: Imagination, session `self-2026-10-09-morning`, 2026-10-09.
Base for every phase-1 anchor: Ghostlight `origin/main` `4fee97f`.
Times are UTC unless marked local (UTC+2).

## Body facts (probed 2026-10-09)

### Does main run on Yggdrasil?

- **The code on `main` is running.** The live unit
  `idunn-ghostlight-dda9de02…` (PID 818485, up since 2026-09-23 20:02:48) runs
  release `sha256-51b014be…`, whose binary embeds commit `454fc33` (grep of
  the binary for the stamped commit: `454fc33` 5 hits, every later commit 0).
  `git diff --stat 454fc33 origin/main` touches only
  `docs/architecture/ghostlight-play-agent-cut.md`,
  `notes/fresh-workspace-handoff.md` and `state/map.yaml`. So `main`'s code is
  live; only its provenance stamp is a week old.
- **Why the 2026-09-30 deploy died.** Unit `idunn-ghostlight-0e4f20f5…`
  (release `sha256-ec17cae9…`, binary stamped `4fee97f`) logged
  `publishing initial Warming runtime presence failed; retrying error=timed
  out connecting runtime-presence publisher to 10.77.0.1:17871` every ~2.5 s
  from 10:58:35, then at 11:00:36 `Error: publishing initial Warming runtime
  presence timed out` and exit 1 (`journalctl -u` on the unit). The bound is
  `WRITE_LEASE_WAIT_TIMEOUT` = 120 s
  (`crates/ghostlight-dungeon/src/runtime.rs:1939`, used at `:1985-1992`).
  `10.77.0.1:17871` is nginx's UDP stream route to Odin's catalog
  (`/etc/nginx/idunn-stream-routes/odin.conf`: listen `10.77.0.1:17871`,
  upstream `127.0.0.1:17973`, the `odin-daemon` socket). Idunn itself could not
  reach it in the same minutes: `Idunn rejected admitted odin route
  continuity: timed out connecting CultMesh RUDP client idunn-route-observer
  to 10.77.0.1:17871` (idunn-yggdrasil journal, 10:57-11:08). Idunn recorded
  the transaction `up-855ddf0c…` as failed and kept the 09-23 generation:
  daemon survival worked as designed.
- **The Odin route outage window.** Idunn's "odin route continuity"
  rejections per hour run from 2026-09-28 01:19 to 2026-09-30 12:xx, peaking
  at 1,326 in 2026-09-29 23:00, and stop after that. The live Ghostlight's
  failed presence republications per day: 3,186 (09-28), 3,793 (09-29),
  2,512 (09-30), then 1-7 per day from 10-01. The peak matches the scar in
  Eureka's `tools/stopgap/ygg-verify.sh` header: at 8 verify slots on
  2026-09-30 01:50 local, `/proc/pressure/io` full avg60 was 41% and Odin's
  fsyncs stalled long enough for Idunn's route challenges to time out. That
  explains the peak; the start on 09-28 is not proven to share the cause.
  Odin's unit failures in the window include `RUDP packet connection id …
  does not match` and `timed out waiting for reliable acknowledgement from
  CultMesh RUDP catalog 127.0.0.1:17973` (journal of `idunn-odin-*`).
- **Consequence for deploys.** Ghostlight cannot start without Odin: the
  Warming presence and the process write lease both go through Odin
  (`runtime.rs:1974-2003`). That coupling is the lease design, not a defect
  in this campaign's scope. A deploy is preflighted on Odin's route health
  and on Yggdrasil's IO pressure.

### Does main build and pass?

Two jobs through `ygg-verify.sh` at `4fee97f`, with `vendor/eve` initialised:

- Rust (image `eureka-verify-rust`), the recipe's own steps:
  `ghostlight-persona-projection` 35 passed; `ghostlight` lib 621 passed,
  1 ignored (`real_local_model_cognition_modes_commit_speech`, needs a model);
  `tests/external_admission.rs` 13 passed; doc-tests 10 passed;
  `ghostlight-dungeon --bin` 198 passed, 4 ignored; release build green in
  3 m 08 s. Total 877 passed, 0 failed. Warnings: unused imports
  (`RECORD_GAP_PATCH_TOOL`, `WorldPhase`, `DEFAULT_LOCAL_MODEL_PREFIX`,
  `DateTime`).
- Node (image `eureka-verify-kotlin`, Node 24.21): `sidecar/claude-sdk`
  27 passed, 3 skipped ("run `npm run build` first"); `web` 3 passed;
  `web` build green.
- **The four ignored Dungeon tests are the Eve client-bridge tests**
  (`runtime.rs:3054`, `:3677`, `:3798`, `:3904`), the only tests that drive
  the real vendored browser lowering. They need Node beside cargo. The rust
  verify image has no `node` (probe: `command -v node` exit 127), and the
  recipe runs them in no step, so the release gate never runs them. PA.f214
  is the class of defect they exist to catch.

### The play-session findings against main today

No commit after `454fc33` touches code, so every PA.f211-PA.f216 is open on
`main`. Mechanisms, probed:

- **PA.f211 and the tunnel.** Bonsai's endpoint on Yggdrasil is the reverse
  tunnel `127.0.0.1:18080`. With Bonsai down (as now), `curl` to it returns an
  empty reply (exit 52): sshd accepts, then closes. `local_inference.rs:166-181`
  classifies only `error.is_connect()` as retryable, so a Bonsai restart reaches
  the play loop as `InferenceFault::new` (not retryable) and closes the turn on
  the first call. Even a retryable fault gets
  `backoff_delay` (`play.rs:2970-2973`, base 100 ms, cap 5 s) times
  `ROUND_RETRY_BUDGET` 12 (`play.rs:89`), about 36 s in all, against a reload
  the Raven runbook puts at about 30 s plus load. Raven's
  `/opt/gamecult/bonsai/rss-watchdog.sh` (systemd `bonsai-rss.timer`, every
  2 min) restarts past 4.5 GiB RSS unless `/slots` shows `is_processing`;
  between the calls of a turn nothing is processing. Raven's
  `idle-reaper.sh` (`bonsai-idle.timer`) stops Bonsai and SillyTavern after
  30 minutes with no `launch_slot_` in the container log. Neither script is in
  version control: gamecult-ops has only the runbook describing them.
- **PA.f212.** A closed turn records its fault (`PlayTurn.fault`,
  `play.rs:298`; `close_with_fault`, `play.rs:1753-1765`), but
  `PlayTurnView` (`play.rs:1224-1246`) has no fault field, the card
  (`eve.rs:455-543`) renders narration, question, refusal, running and
  resolving rows and nothing for a fault, and `close_with_fault` logs nothing.
  A local call can also wait `RESPONSE_TIMEOUT` = 900 s
  (`controllers.rs:68`, used by `local_inference.rs:157`) with "Resolving your
  turn" on the card the whole time.
- **PA.f213.** The owner's Draft card always offers "Seed the world"
  (`eve.rs:366-399`); the handler refuses when `GHOSTLIGHT_SEED_VAULT_ROOT` is
  unset (`runtime.rs:1701`, `:1739-1745`). Yggdrasil sets none.
- **PA.f214.** `world.play.text` (`eve.rs:588-594`) carries no authored
  `value`. The vendored lowering captures only `value`, never `placeholder`
  (the PA.f164 comment at `eve.rs:288-296`), so Play clicked on an untouched
  box sends no `text`, and `PlayPayload.text` (`runtime.rs:288-292`) is
  required. The "submit an empty message to continue" hint (`eve.rs:514-520`)
  asks the player for exactly that payload.
- **PA.f215.** The acceptance step runs
  `controllers::tests::real_local_model_cognition_modes_commit_speech`
  (`controllers.rs:11231`): NarrativePersona and OperationalAgent each must
  commit speech in one shot against Bonsai. It exercises the library's
  controller path, not the Dungeon play loop. Failing runs' prose is written to
  `GHOSTLIGHT_ACCEPTANCE_PERSONA_PROSE_LOG`; Idunn's staging dirs on Yggdrasil
  hold `.runner-rust-acceptance` workspaces per transaction.
- **PA.f216.** Creation is one form of eight fields, three of them raw JSON
  (`targets` `{}`, `jurisdictions` `[]`, lens weights; `eve.rs:244-331`), then
  a Draft card with Approve, Seed and Activate as separate buttons
  (`eve.rs:343-415`).

### The Bonsai host under ruling `bonsai-idle-unload-host`

Probed 2026-10-09 by Imagination (`imagination-gl-bonsai`, session
`self-2026-10-09-ag`). Fork sources were fetched raw from
`PrismML-Eng/llama.cpp` at `1a07bfa5f4144274c8f1c9963821dd9d9a51854b`; paths
below are `tools/server/` in that tree unless named.

- **B1. Ghostlight base.** `origin/main` is `4474718`. `git diff --stat
  4fee97f 4474718` over `controllers.rs`, `local_inference.rs`, `play.rs`,
  `eve.rs` and `runtime.rs` is empty: every phase-1 anchor in those files holds
  at `4474718` (the commits between are the docs-subtract cuts).
- **B2. Who owns the host scripts.** `gamecult-ops` `origin/main` is
  `9c12adb`. Its tree has no Bonsai script, unit, Dockerfile or template: the
  only Bonsai file is `runbooks/bonsai-local-llm-raven.md`. No copy of
  `rss-watchdog.sh` or `idle-reaper.sh` exists anywhere under `F:\Projects`.
  `run.sh`, `start-all.sh`, the `Dockerfile`, the template and both scripts
  exist only on Raven, in WSL under `/opt/gamecult/bonsai`. Their repo-to-be is
  `gamecult-ops`, which is a campaign repo.
- **B3. Raven unreachable during this pass.** `ssh raven` (10.77.0.4:22) and
  the LAN address 192.168.178.165:22 both timed out. On Yggdrasil, `curl
  http://127.0.0.1:18080/v1/models` exited 7 (connection refused: the tunnel's
  listener is not bound). That differs from the exit 52 (accept and close)
  recorded above, so both shapes occur. Raven was then reported under live
  repair (bootloader damaged by an accidental format of `E:`), and no
  further probe of Raven or the tunnel was made. Every Raven claim on this
  page is therefore **unprobed** at mapping time. The
  contents of `run.sh`, the watchdog and the reaper come from the Eyes file
  `F:\Projects\bonsai-host-2026-10-09.md` §1, which read them in full at about
  17:19 local. Residual VRAM while sleeping and the time to wake are
  **not measured**.
- **B4. The flag exists at the pin.** `common/arg.cpp:3779-3785` defines
  `--sleep-idle-seconds SECONDS`, default -1 (disabled), and rejects 0 and
  values below -1. The runbook confirms the image pins `1a07bfa5`. The argv of
  `run.sh` in the Eyes file does not pass the flag.
- **B5. What sleep does.** `server-queue.cpp:278-360`: when the queue has had
  no task for `idle_sleep_ms`, it marks itself sleeping and runs the sleeping
  callbacks. `server-context.cpp:906-921`: entering sleep calls `destroy()`,
  which frees the model, contexts and mmproj. Leaving sleep calls
  `load_model`, and a failed reload is `GGML_ABORT` (the process dies). A
  request that needs the model calls `wait_until_no_sleep()`
  (`server-queue.cpp:115-130`, from `server_res_generator`,
  `server-context.cpp:4100-4110`) and blocks until the reload returns. So the
  first request after sleep is one HTTP call that waits out the reload. It
  gets no 503 and no transport error.
- **B6. Endpoints that never wake it** (each one starts with
  `create_response(true)`):
  - `GET /health` (`server-context.cpp:4526-4537`) is always 200
    `{"status":"ok"}` once the server has started.
  - `GET /props` (`:4676-4686`) returns cached props while sleeping, with
    `is_sleeping: true` (`get_res_props`, `:4471-4506`; the cache is filled on
    entering sleep, `:5411-5418`).
  - `GET /v1/models` (`:4946-4955`) is answered from the cache.
  - `GET /metrics` is exempt too.

  `README.md:2063-2075` ("Sleeping on Idle") lists the same exemptions and
  says they do not reset the idle timer. The queue's sleeping flag clears only
  after `load_model` returns (`server-queue.cpp:339-350`), so `/props` reports
  `is_sleeping: true` for the whole of a wake.
- **B7. An endpoint that wakes it.** `GET /slots` (`server-context.cpp:4601-4613`)
  calls `create_response()` without the bypass, so it waits for a reload. Its
  `SLOT_GET` task also resets the idle timer: `server-queue.cpp:24-26` exempts
  only `METRICS`. A watchdog that polls `/slots` therefore wakes a sleeping
  model, and it keeps an awake one from ever sleeping.
- **B8. Before the first load.** Until the first load completes, every
  non-frontend path answers 503 with `{"error":{"message":"Loading
  model","type":"unavailable_error","code":503}}` (`server-http.cpp:253-272`).
  This covers a fresh start and a Docker restart. Ghostlight already classifies
  503 as retryable (`local_inference.rs:183-189` at `4474718`).
- **B9. Log lines.** A slot starts in `launch_slot_with_task`
  (`server-context.cpp:1614`); the reaper greps for `launch_slot_`. A slot's
  release logs `stop processing: n_tokens = ...` at info (`:498-502`).
- **B10. Ghostlight with a sleeping or stopped Bonsai today** (at `4474718`):
  - `InferencePort` (`controllers.rs:299-311`) has no readiness method.
  - `LocalInferencePort` has one client whose overall timeout is
    `RESPONSE_TIMEOUT`, 900 s (`local_inference.rs:145-164`,
    `controllers.rs:68`). A sleeping model therefore costs the first call its
    reload inside one call, well under 900 s. Meanwhile the card shows
    "Resolving your turn…" (`eve.rs:530-543`) and nothing says the model is
    waking. This is a bounded wait, not a hang, but it is not an honest one.
  - A stopped container behind a live tunnel accepts and closes, which today
    becomes `InferenceFault::new`; play-faults R3 makes it retryable. A dead
    tunnel refuses the connection, which is already retryable.
  - A `world.play` admission error reaches the player as `PlayError`'s
    `Display` text, through `RuntimeCommandError::Payload`
    (`runtime.rs:1399-1402`).
  - `admit` returns a key replay at `play.rs:1476`, before it opens a turn or
    applies an answer (`:1479`).
- **B11. Raven's lifecycle today** (from the Eyes file; not re-probed):
  - The reaper (`bonsai-idle.timer`, every 5 min) runs `docker stop bonsai
    sillytavern` when the container is older than 30 min and its log has no
    `launch_slot_` in the last 30 min.
  - The watchdog (`bonsai-rss.timer`, every 2 min) runs `docker restart bonsai`
    when RSS is over 4.5 GiB, deferring while `/slots` shows `is_processing`.
  - `run.sh` sets no restart policy unless given `--resident`.
  - The container last ran 2026-09-30 10:38:19Z to 11:28:08Z after a desktop
    launch. The launcher returns once the stack is up, so WSL stayed up for
    about 50 minutes with no attached session.
  - Raven's Idunn runs Muninn only, and Ollama and LM Studio cannot load PQ2_0
    (Eyes §2 C and D).

### A live defect not yet in the findings

The live process has logged `Heimdall refresh transport unavailable
error=Heimdall denied the private command: Idempotency key was reused with
different command content.` once a minute since 2026-09-23 21:04 (1,440 a
day from 10-01). Sequence: 21:02:03 discovery timed out; 21:03:00 `local app
session rejected Heimdall refresh error=local session changed during refresh`;
from 21:04 the denial forever. Mechanism:

- Ghostlight's refresh key is `refresh:{heimdall_session_id}:{access_revision}`
  (`runtime.rs:1877-1880`), stable until the local session advances.
- Every attempt is resealed with a fresh nonce, IV and issue time
  (`heimdall.rs:880-895`).
- Heimdall fingerprints the sealed request bytes, not the command
  (`Heimdall/src/private-command-plane.ts:88`,
  `createHash("sha256").update(request.payload)`), and refuses a known key with
  a different fingerprint (`:93`).
- So once Heimdall has executed one refresh (it rotated the token at 21:03)
  and the local commit refused it (`app_session.rs:293-300`), no retry under
  that key can ever succeed, and `sessions_due_for_refresh`
  (`app_session.rs:247-268`) keeps offering the same candidate while
  `refresh_expires_at > now`. Which clause of `app_session.rs:293-299` fired at
  21:03 is not logged.

### The host

- `ghostlight-dungeon.service` (the v1 body) is **enabled**, inactive since
  2026-09-23 14:31. It binds `127.0.0.1:8831`, runs `/srv/ghostlight/current`,
  targets CodexConnector, and would start at the next boot beside Idunn's
  generation. `/srv/ghostlight` holds 46 GiB (226 releases, acceptance
  build scripts), `/var/lib/gamecult/ghostlight-dungeon` 17 GiB of v1 world
  state (the v2 kernel refuses earlier schemas; nothing migrates it),
  `ghostlight-dungeon-quarantine` 44 KiB, `ghostlight-import-20260821T1858Z`
  8 KiB. Disk `/` is 20% of 2 TB used.
- The v1 actuator and its tests live in gamecult-ops:
  `scripts/deploy-ghostlight-yggdrasil.sh`,
  `scripts/test-ghostlight-yggdrasil-wiring.sh`; the runbook's "One-time state
  cut" section (`runbooks/ghostlight-dungeon-yggdrasil.md:456-499`) describes
  that path.
- Idunn's leftovers: 12 failed `idunn-ghostlight-*` transient units (11 from
  the 09-23 bring-up, 1 from 09-30), 15 release dirs under
  `/srv/ghostlight-idunn/releases` (403 MiB), per-transaction staging dirs
  under `/var/lib/gamecult/idunn/staging`. These are Idunn's: it launches the
  units and owns release retention. Phase 1 does not hand-clean them.
- Hostname is `yggdrasil-candidate`. Heimdall still runs as the legacy
  `heimdall.service`; every Idunn v2 deploy of it has failed (`idunn status`).

### The repo

- The pre-rebuild research machine is still on `main`: `tools/` (17 files,
  4,146 lines, including the Python state store), `schemas/` (14, 4,057),
  `examples/` (194, 37,421), `experiments/` (172, 12,325), `acceptance/`,
  `prompts/`, `scripts/`, `output/`, `assets/newspaper/`, `state/` (map.yaml,
  evidence ledger and archive, a CultCache JSONL, corpus coverage, branches),
  `docs/aetheria/`, root `package.json` scripts pointing at a Codex Python
  runtime, and about 30 pre-rebuild architecture and note pages. No file
  outside Ghostlight references `Ghostlight/schemas`,
  `ghostlight_state_store` or `ghostlight.agent_state` (grep over
  `F:\Projects`, excluding build output).
- `AGENTS.md` (234 lines) instructs agents to treat `state/` and the Python
  store as canonical state and the implementation plan as current.
- Live docs that cite the old `world/*.rs` paths:
  `ghostlight-world-consumer-api.md:19,31,33,38,70`;
  `ghostlight-world-ontology.md:53,74,167,202,203,215,232,277`;
  `notes/fresh-workspace-handoff.md:253`; `notes/local-live-smoke.md:56`. The
  code is flat under `crates/ghostlight/src/`. Dated cut documents and
  postmortems keep their paths: they are history.

## Model page

One row per persistent kind phase 1 touches. No cell may be empty.

| Kind | Named by | Life over time | Who decides |
|---|---|---|---|
| World state and commit journal (`.cc`, redb) | `WorldId`; `revision`; digest chain | created Draft, approved, activated; one commit row per admitted command; earlier schemas refused, never migrated | the kernel, through `WorldMailbox`'s closed operation set; untouched in phase 1 |
| Play turn row (play-turn store `.cc`) | `turn_id` (UUID); per-turn `question_token` | `Running` → `AwaitingPlayer` ↔ `Running` → `Closed` (narration or `fault`); an unreadable row is retired to a sidecar and play starts fresh | `PlayTable` (`play.rs`); the card is a projection of `PlayTurnView`, which phase 1 extends with the fault |
| App session (`app-sessions-v2.cc`) | cookie hash; `heimdall_session_id` + `access_revision` | issued at sign-in; refreshed when access is near expiry; revoked on logout, custody change, invalid receipt, and (phase 1) on a refresh that can no longer succeed | `AppSessionOwner`; Heimdall decides the upstream session and rotates refresh tokens |
| Heimdall private command receipt (Heimdall Postgres) | `(app_slug, idempotency_key)` + request fingerprint | written once per executed command; replayed for an identical request; a different request under the key is refused | Heimdall; its fingerprint rule is a Heimdall follow-up, not a phase-1 cut |
| Idunn generation (transient unit, release dir) | `sha256-<release>` | staged → started → Warming → lease → Active → superseded or failed; the previous Active stays until a new one is Ready | Idunn; retention and failed-unit collection are Idunn's follow-up |
| v1 body (`ghostlight-dungeon.service`, `/srv/ghostlight`, v1 state) | unit name; paths | retired in fact 2026-09-23; unit still enabled | nobody today; phase 1 assigns its removal to a gamecult-ops cut, data by operator ruling |
| Bonsai lifecycle (Raven container and model) | container `bonsai`; installed from gamecult-ops into `/opt/gamecult/bonsai` (raven-bonsai-scripts) | today: started by hand, restarted by the RSS watchdog, stopped by the idle reaper. Ruled: the server stays up (Docker `unless-stopped`); the model loads on start and on demand and unloads after `BONSAI_SLEEP_IDLE_SECONDS` idle (llama-server sleep); the watchdog restarts only an awake, quiet server; only a human stops it ("Free the GPU"); no reaper | llama-server decides load and unload; Docker decides process restart; the watchdog decides leak restarts; gamecult-ops owns the scripts; Dungeon learns readiness only through its inference port (model-asleep) |
| Repository docs (README, AGENTS, live architecture pages, handoff) | path on `main` | rewritten to the live machine; pre-rebuild material leaves `main` by operator ruling | the Ghostlight repo; history in git, dated cut documents and postmortems |

### Where phase 2's verb sets plug in

Ruling `verse-verbs-two-render-paths` and invariant `every-event-renderable`
bound what the kernel may admit to two render paths. Today:

- The thirty operations (`ghostlight-world-ontology.md:451-478`) are
  state transitions, not verbs. They stay the closed substrate.
- What an actor *does* is an affordance: a world-authored catalog entry whose
  kind is `AffordanceKindName` (`patch.rs:247-254`), free canonical text that
  "the kernel carries and branches on nowhere", plus the kernel-built `speak`
  (`controllers.rs:94`). Admitted acts surface as `DecisionEvents` on the
  commit.

So phase 2's two verb sets plug in at the affordance kind: a closed
vocabulary of kinds, each tagged with its one render path, with effects that
still compile to the thirty operations. Today the Play authority can declare
any affordance kind, so nothing on `main` enforces the invariant yet. Phase 1
neither widens nor narrows that, and its docs must not describe open-ended
verbs as the target.

## Rationale

- **"main dies" was a misreading.** The deploy that died carried no code
  change, and it died on Odin's route, not on Ghostlight. So phase 1's deploy
  work is mostly preflight and proof: redeploy `main` after the play fixes,
  with Odin's route and IO pressure checked first. The Odin outage is
  recorded against its owner.
- **Subtraction goes first and alone.** The pre-rebuild tree is about 60,000
  lines that the live machine never reads. It is cut before the docs are
  rewritten, so the rewrite describes only what is left. Parking it at a tag
  rather than deleting it outright is the operator's call (question).
- **Faults belong to the card, not the log alone.** PA.f212 is an authority
  gap: the turn store knew the fault and the projection dropped it. The fix
  extends the projection; it adds no second record.
- **Bonsai sleeps inside its own server.** The ruling named hosts that
  unload on idle. Ollama and LM Studio cannot load PQ2_0, and the pinned fork
  already sleeps (B4, B5), so the cut is a flag plus deleting the reaper.
  llama-swap would add a process and a second owner of load and unload. It
  earns its place only if a sleeping server's leftover VRAM (its CUDA context)
  proves too large, and raven-bonsai-scripts measures that before anything
  else changes. The watchdog must stop calling `/slots` on a sleeping server,
  because that call wakes the model (B7). The waking and away rows belong to
  model-asleep and the fault row to play-faults, so neither cut decides the
  other's row.
- **Retrying inference is safe.** A model call is a proposal the kernel
  admits or refuses, so a transport fault before any reply is retryable. That
  is the Ghostlight half of PA.f211. The Raven half (the watchdog's quiet
  window) belongs to gamecult-ops, which also takes the scripts into version
  control.
- **The refresh loop is a contract split.** Ghostlight assumes a stable key
  makes a retry safe; Heimdall's fingerprint makes every retry new. Ghostlight
  stops looping (a refresh that can no longer succeed revokes the local
  session). Heimdall's fingerprint rule goes to Heimdall as a follow-up.
- **The bridge tests are the gate's blind spot.** They are the only
  Dungeon tests that drive the real client, and nothing runs them. Fixing the
  gate means a runner with both Rust and Node, which is Idunn's recipe and
  runner surface: a follow-up, with each phase-1 Hands run doing them by hand.
