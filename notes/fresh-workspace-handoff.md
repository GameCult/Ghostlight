# Ghostlight Fresh Workspace Handoff

Updated: 2026-08-21

This is the compact re-entry packet. It records current authority, deployed
truth, proof, and the next decision. Git history owns chronology; the system map
and architecture documents own detail.

## Immediate Re-entry Instruction

Do not continue implementation automatically from a rehydrate-only request.
Reconstruct the current authority map, report the next gate, and wait for the
requested scope. Do not trust this file for the exact live HEAD. Git commands
own volatile workspace identity and the release witness owns deployed identity.

1. Work from `F:\Projects\Ghostlight`.
2. Run `git status --short`, `git log -1 --oneline`, and
   `npm run state:status`.
3. Read `state/map.yaml` (`current_status` first), then
   `notes/ghostlight-current-system-map.md`.
4. Read the architecture document for the organ being changed. Do not infer
   runtime authority from workspace HEAD or from this handoff.
5. For deployment, SSH, firewall, Idunn, Odin, Heimdall, or host work, consult
   `F:\Projects\gamecult-ops` before acting.

## Mission and current product shape

Ghostlight Dungeon is a persistent, Vault-grounded narrative simulation. It
addresses dreamlike world reconstruction, unearned NPC knowledge, inert scene
participants, consequence-free player claims, and worlds that wait passively
for the player.

The hosted forge supports persistent DM-led Session Zero and bounded co-op for
one to eight Heimdall-authenticated players. The approved brief compiles into a
stable world with actor-bound membership, private character state, fiction-first
d20 resolution, persistent locations and institutions, multiresolution Gestalt
agency, offscreen strategic activity, knowledge-filtered news, and actor-filtered
Eve/CultMesh surfaces.

The immediate product gate is human multiplayer proof, not another foundation
rewrite.

## Authority map

- `SessionZeroKernel` is the sole owner of pre-publication drafts, members,
  channels, boundaries, decisions, approvals, and the final approved digest.
- Each campaign `WorldKernel` mailbox is the sole owner of canonical world state
  and revision. Player commands, NPC proposals, ticks, travel, waits, imports,
  reloads, and contract amendments share its validated atomic commit path.
- Ghostlight owns the generalized Projector → Persona → Interpreter membrane.
  Projectors, Personas, Interpreters, retrieval, narration, dice previews, and
  browsers may propose or project; none may commit canonical state.
- Canonical actors, institutions, Gestalts, member deltas, knowledge, topology,
  and relationships persist independently of the derived simulation cover.
  Arena cells never become synthetic collective actors or union knowledge.
- Heimdall owns account identity. `campaign_membership.v1` binds an authenticated
  member to exactly one canonical actor.
- VoidBot owns Vault retrieval and evidence. Ghostlight stores exact evidence
  receipts, not a rival semantic index.
- Idunn owns deployment and same-release daemon continuity. Odin owns discovery.
  Nginx owns the public crossing. None owns campaign mutation.
- Epiphany consumes Ghostlight's pinned projection crate while retaining her own
  Mind and consequence authority. Her runtime is adjacent swarm capacity, not a
  Ghostlight state owner or current dependency.

## Exact live Ghostlight body

Yggdrasil is production. The Starfire writer and its old tunnel are stopped.
Do not start both writers.

- Executable release:
  `10b638e45756e5210aaee1efb5dcf74dbebf83c0`
- Executable SHA-256:
  `327c93b0cddd6b344f85e14e24be51688d74a58e1ea22dd42393b81b07f48f7a`
- Eve release: `076d2124ed476cdaeff540c9d24c2d4fb57d04cf`
- Service: `ghostlight-dungeon.service`, running as `ghostlight:ghostlight`
- Listener: Yggdrasil loopback `127.0.0.1:8831`
- Public route: `https://yggdrasil.gamecult.org/ghostlight/`
- Vault retrieval: local VoidBot at `127.0.0.1:17875/mcp`
- Stored state at the last witness: seven campaigns and one Session Zero draft
- Provider state: DeepSeek startup inference ready
- Access witness: public surface 200; unauthenticated campaign API 401

Manifest, embedded commit, executable hash, typed health, persistent stores,
restart recovery, public crossing, and anonymous rejection agree.

## Deployment and discovery truth

Idunn and Odin are live from exact source
`745e01093c59882ed098b7515ef8921d55fbed15`. Odin discovery is healthy at
`10.77.0.1:17871`. Idunn's health clock preserves millisecond timestamps; the
former same-second false “health vanished” transitions are gone.

Idunn selects the newest executable- or build-affecting commit reachable from
the admitted ref. Documentation, notes, state receipts, and root Markdown do
not cause a deployment. The root actuator proves the selected commit is an
ancestor of the admitted ref and verifies the exact installed witness after
activation. Therefore later Ghostlight documentation commits do not displace
release `10b638e4…`.

## Epiphany capacity gate

Epiphany source `ebc0ffe4f341154d1902f9afe86f0a87f150179c` passed its locked
test suite and was sealed as immutable package
`sha256-bb76728653b8e2e872b4da47f917abe4233fd6d4ae1fd573c5971c7db3922a5c`
with witness
`4d8350fac61f90d32a2b8067731308ec3e3672a42804db29c680b0fc68ab9adc`.

It is not deployed. Idunn stopped before publication because the Bifrost
operator runtime identity/substrate and resident Self Codex credentials were
absent. The deployment brake remains engaged, Epiphany units are inactive and
disabled, no `deployment.env` or signed health was published, and
`/srv/epiphany/app/current` remains recovery release
`267a0257a4938d80d34b7807c66aa5f550b50f2c`.

Ghostlight testing can proceed without Epiphany. If her swarm capacity is
needed, provision the missing substrate and credentials through one Idunn-owned
deployment transaction. Do not manually launch Epiphany, bypass the brake, or
create passive watcher tasks around the operation.

## Current proof

- Full workspace: 161 core tests, 15 daemon tests, strict TypeScript, and Vite
  release build passed for the accepted body.
- One authenticated Session Zero canary survived exact-build restarts, retrieved
  grounded Aetheria evidence, preserved typed state across malformed model
  output, and completed a Mars/Zhestokost follow-up without borrowing the First
  Exodus cast or geometry.
- The solo player journey proved compilation, canonical player identity,
  private assessment, server-side roll, wait, restart, fork, export, reset, and
  continuity.
- The 24-subject Gestalt matrix covers budgets 1, 4, 8, and 32 exactly once.
- The strict nested-refugee golden preserves Mira Venn's exact identity and
  private delta through a budget-one rival arena, nested migration, background
  activity, folding, and later rematerialization.
- A budget-8/provider-parallelism-8 strategic wave covered every subject,
  committed material background activity, and did not puppet the player.

Acceptance witnesses live under `F:\GameCult\GhostlightDungeon\acceptance`.
Operational release and rollback truth remains in `gamecult-ops`.

## Next action

1. Run a two-person Yggdrasil Session Zero and bounded shared-scene canary using
   separate Heimdall accounts.
2. Then run the eight-account roster, privacy, actor-binding, pooled-budget,
   unanimous-approval, unanimous-time/travel, stale-command, and restart smoke.
3. Continue human pressure on multiresolution Gestalt agency: meaningful
   background surprises, exact attributed rival activity inside arenas,
   information boundaries, nested fission/folding, migration, and return
   encounters.

Do not weaken actor custody, private projection, unanimous approval,
knowledge gates, no-puppeting, or atomic-wave invariants to make the smoke pass.
Do not gate this testing on Epiphany unless the test explicitly exercises her
capacity.

## Essential references

- MVP authority: `docs/architecture/ghostlight-dungeon-mvp.md`
- Gestalt authority: `docs/architecture/ghostlight-multiresolution-agency.md`
- Live system map: `notes/ghostlight-current-system-map.md`
- Implementation program: `notes/ghostlight-implementation-plan.md`
- Human-readable state: `state/map.yaml`
- Machine-managed research state: `state/ghostlight-state.cultcache.jsonl`
- Operations: `F:\Projects\gamecult-ops`

## Re-entry warnings

- A prompt, transcript, browser surface, model receipt, or derived simulation
  cover is not canonical world state.
- A green local test is not deployment truth; verify exact manifest, witness,
  process, health, and public behavior.
- Do not infer executable drift from docs-only Git drift.
- Do not revive Starfire's retired writer or VoidBot/Qdrant writers.
- Do not launch multiple tasks to “watch” one deployment. The owning operation
  returns one terminal receipt upward; task chat is not a lock service.
- Keep this file compact. Distill current steering here and leave chronology in
  Git, receipts, and the deeper maps.
