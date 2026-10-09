# Ghostlight Handoff

This is a pointer. It does not carry open work; the Eureka mind does.

## What runs

Ghostlight Dungeon runs on Yggdrasil under Idunn at
`https://yggdrasil.gamecult.org/ghostlight/` (runtime `ghostlight-yggdrasil`,
target `ghostlight`, ref `main`, state root `/var/lib/gamecult/ghostlight-world-v2`).
`/health` names the commit it serves (`crates/ghostlight-dungeon/src/runtime.rs`).
The binary is `ghostlight-dungeon`; its inference reaches Bonsai on Raven
through a loopback tunnel, and sign-in goes through Heimdall, discovered via
Odin.

## How it deploys

Idunn is the only deploy path (`idunn up ghostlight`). The recipe is
`deployment/idunn/recipe.toml`; the runbook is
`gamecult-ops/runbooks/ghostlight-dungeon-yggdrasil.md`. The Ghostlight daemon
cannot start without Odin: its Warming presence and process write lease go
through Odin. Preflight Odin's route health before a deploy. Do not invoke the
retired v1 actuator (`gamecult-ops/scripts/deploy-ghostlight-yggdrasil.sh`), its
wiring test, or `ghostlight-dungeon.service`.

## Where open work is

Query the Eureka mind, campaign `ghostlight-verse`: the target in force, the
rulings in force (operator directions included), open questions and follow-ups,
specs with no report, and open findings. The recipes are in
`~/.claude/skills/eureka/references/campaign-state.md`. The campaign's prose
target and probed body facts are `notes/ghostlight-verse-target.md` and
`notes/ghostlight-verse-map.md`.

## Reading order for a new session

1. `README.md`: what exists and what does not.
2. `AGENTS.md`: invariants and where things are.
3. `docs/architecture/ghostlight-dungeon-mvp.md`: the authority architecture.
4. `docs/architecture/ghostlight-world-ontology.md`: the kernel's vocabulary and
   current mechanism.
5. `docs/architecture/ghostlight-play-agent-cut.md`: the dated record of the play
   agent and of the first deploy on 2026-09-23 (findings PA.f198 to PA.f216).
   It is history; the mind says which findings are still open.

`notes/local-live-smoke.md` covers bringing up the Claude SDK sidecar by hand.
`notes/ghostlight-implementation-plan.md` is the historical plan of the 2026-09
rebuild.
