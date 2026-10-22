# Ghostlight Handoff

This is a pointer. It does not carry open work; the Eureka mind does.

## What exists

Nothing is deployed. The Dungeon (daemon, play table, Eve surface, browser host,
`vendor/eve`, Idunn recipe) is parked at the tag `parked/ghostlight-eve-surface`
and is not on main. A Thing 2 rebuild starts from that tag.

What is in the tree is the kernel (`crates/ghostlight`), the persona projection
(`crates/ghostlight-persona-projection`) and the verse runner
(`crates/ghostlight-verse`). `cargo test` runs them on Linux; use the
`ygg-verify` stopgap on Yggdrasil, not the workstation, and say which host
you used.

The Idunn target `ghostlight` is braked and stopped, not retired. Its state is in
`gamecult-ops/runbooks/ghostlight-dungeon-yggdrasil.md`. Do not run
`idunn up ghostlight`, the v1 actuator, or `ghostlight-dungeon.service`.

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
3. `docs/architecture/ghostlight-world-ontology.md`: the kernel's vocabulary and
   current mechanism.
4. `docs/architecture/ghostlight-dungeon-mvp.md`: the parked Dungeon's authority
   architecture. History until Thing 2 revives it.
5. `docs/architecture/ghostlight-play-agent-cut.md`: the dated record of the play
   agent and of the first deploy on 2026-09-23 (findings PA.f198 to PA.f216).
   It is history; the mind says which findings are still open.

`notes/local-live-smoke.md` covers bringing up the Claude SDK sidecar by hand.
`notes/ghostlight-implementation-plan.md` is the historical plan of the 2026-09
rebuild.
