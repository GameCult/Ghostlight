# Ghostlight Fresh Workspace Handoff

Updated: 2026-08-23

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

The immediate engineering gate is completion of the principled world-transition
migration before adversarial hosted play resumes. Bounded region expansion uses
the mutation reducer; seed publication is now a bounded creation transaction;
named-person materialisation/folding is a resolution transaction without
Gestalt-wide effect authority. The 36-case agency corpus is only a candidate
seed against a 300-reviewed-case target. Separate-account multiplayer privacy
and unanimity proof also remains required, but the previously expected D&D test
group is no longer available; record that gate as unproven rather than silently
lowering it.

## Authority map

- `SessionZeroKernel` is the sole owner of pre-publication drafts, members,
  channels, boundaries, decisions, approvals, and the final approved digest.
  It separately owns the complete model-invocation audit and the exact active
  preview proof set; campaign publication may consume only the latter.
- Each campaign `WorldKernel` mailbox is the sole owner of canonical world state
  and revision. Player commands, NPC proposals, ticks, travel, waits, imports,
  reloads, and contract amendments share its validated atomic commit path.
- Accepted foreground, reaction, strategic, time, travel, population-fission,
  and bounded region-expansion outcomes lower into one closed
  `WorldMutationBatch` vocabulary. Means and intended effects remain proposals;
  only the kernel reducer can alter typed canonical components. Legacy effect
  enums are model-boundary migration input, not alternate physical laws.
- Ghostlight owns the generalized Projector → Persona → Interpreter membrane.
  Projectors, Personas, Interpreters, retrieval, narration, dice previews, and
  browsers may propose or project; none may commit canonical state.
- Canonical actors, institutions, Gestalts, member deltas, knowledge, topology,
  and relationships persist independently of the derived simulation cover.
  Arena cells never become synthetic collective actors or union knowledge.
- Heimdall owns account identity. `campaign_membership.v1` binds an authenticated
  member to exactly one canonical actor.
- Eve owns editable bindings, command invocation, command receipts, plugin
  composition, and lowering. Ghostlight publishes one stable
  `ghostlight.play` surface; the browser is a thin provider host and never owns
  login, Session Zero, campaign, governance, or receipt semantics.
- Heimdall's `gamecult.heimdall.access` plugin renders anonymous and identity
  state. Sensitive begin, completion, refresh, and logout operations cross its
  private CultNet command plane; the browser retains only an opaque attempt
  handle and Ghostlight retains only its own hashed app session.
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
  `7522ea8405212b344441f83f502993f525276521`
- Executable SHA-256:
  `1f0c1352aa6778df7f1a501af18ad5f66b34a5c77a34d36ab32ba8c95b14a791`
- Artifact SHA-256:
  `sha256-4ed1fdb9612f159f3b1f5508bca12bbe42a3e233aa477ca2811010abd3434136`
- Eve release: `19c3dcf9173dce848a6253e975324ea239a02d24`
- Service: `ghostlight-dungeon.service`, running as `ghostlight:ghostlight`
- Listener: Yggdrasil loopback `127.0.0.1:8831`
- Public route: `https://yggdrasil.gamecult.org/ghostlight/`
- Vault retrieval: local VoidBot at `127.0.0.1:17875/mcp`
- Stored state at the last witness: nine campaigns and two Session Zero drafts
- Provider state: DeepSeek ready; fast stages use `deepseek-v4-flash` and
  capable stages use `deepseek-v4-pro`
- Access witness: anonymous Eve gate 200; canonical `heimdall.auth.begin`
  accepted through Odin-discovered Heimdall; no actor or campaign state in the
  anonymous surface. A retained Heimdall grant completed fresh authentication,
  and the authenticated Ghostlight app session survived exact release
  replacement. Scheduled refresh now uses a fresh private-command identity per
  attempt, accepts Heimdall's refresh-specific signed claim/session receipt,
  and rotates the locally wrapped claim without returning the browser to the
  access gate.

Manifest, embedded commit, executable hash, typed health, `current` symlink,
zero-restart service state, fresh signed Idunn health, public crossing, and
anonymous rejection agree. CultLib
`75c180782aeba7cfd22d6412877397708a4ed28f` passed its five-runtime local,
cross-runtime, dropped/reordered, fragmentation, and schema-discovery lanes.
Odin `ba76a7239a7bb40aa1774df7d93b9388dc27b222` uses that exact CultLib body,
serializes provider document persistence, and returns a typed persisted
Ghostlight advertisement naming source `7522ea8405212b344441f83f502993f525276521`.
Its persisted `ghostlight.schema_catalog.v1` contains exactly
`ghostlight.campaign.v1` and `ghostlight.session_zero.v1`. The former
large-snapshot RUDP discovery defect is closed; preserve the bounded-window,
hybrid-ACK, explicit-flush, and awaited-persistence regressions.

## Deployment and discovery truth

Idunn, Odin, and Heimdall release identity is operational truth owned by
`gamecult-ops`; verify their current witnesses there rather than trusting an
old commit copied into this handoff. During the 2026-08-23 Ghostlight pressure
run, Idunn continuously admitted signed Ghostlight health and Odin continued
publishing discovery. One Idunn cycle still logged that authenticated
Ghostlight health vanished before projection between fresh samples; the next
signed sample was active and the daemon never restarted. Treat that as an Idunn
projection-clock defect, not a campaign repair signal. Idunn also repeatedly attempted an Odin stale-release
deployment that the deployment brake rejected with `ReleaseMismatch`; this is
an adjacent control-plane fault, not evidence that Ghostlight lost campaign
authority or health. Bifrost persona-feedback health emitted intermittent
dependency-unavailable alarms between fresh diagnostic publications. Rehydrate
those organs in `gamecult-ops` before changing them.

Idunn selects the newest executable- or build-affecting commit reachable from
the admitted ref. Documentation, notes, state receipts, and root Markdown do
not cause a deployment. The root actuator proves the selected commit is an
ancestor of the admitted ref and verifies the exact installed witness after
activation. Therefore later Ghostlight documentation commits do not displace
the selected executable.

## Epiphany capacity gate

Epiphany is adjacent capacity, not a Ghostlight runtime dependency. Rehydrate
in Epiphany and `gamecult-ops` before assuming usable swarm capacity. Do not
manually launch her or coordinate deployment through cross-task exclusivity
messages.

## Current proof

- Idunn admitted exact release `7522ea8…`; its locked Linux package test and
  immutable build completed before atomic activation. Idunn's Linux journal
  reports 265 library tests before the remaining package targets in its
  truncated success receipt; the complete package test and release build
  succeeded. Typed health, manifest, embedded source, executable hash, provider
  readiness, nine campaigns, two Session Zero drafts, and zero daemon restarts
  agree. Odin's raw typed snapshot independently agrees on the exact source
  commit and two advertised boundary schemas.
- Foreground, reaction, strategic, wait, travel, population-fission, and bounded
  region-expansion writes now use the typed mutation reducer. Each successful
  batch advances a touched subject version once and persists its authority,
  mutations, receipt, causal receipt, and aggregate projection atomically.
  Component-only mutation ingress rejects aggregate campaign rows. Initial
  publication is separately bounded to a fresh empty store plus atomic
  discoverability rename; it cannot mutate a published campaign.
- Region expansion admits typed place and proposition profiles, validates
  evidence and containment, and requires an exact reciprocal route between the
  stable origin and every newly compiled destination. Aggregate locations,
  routes, and facts are rebuilt from accepted component state; the direct
  insertion loops have been deleted.
- Named Gestalt members now change foreground resolution without gaining a
  second effect vocabulary. Promotion/folding advances world revision and
  `resolution_epoch`, clears the prior cover, preserves the exact individual
  delta, and leaves fictional time and the Gestalt baseline unchanged. The
  direct `GestaltAggregateDelta` path that could union one person's knowledge,
  resources, or pressures into the whole population has been deleted.
- Population fission cannot copy scarce custody into every child. Approved
  fission assigns every parent resource to exactly one child, transfers each
  named member exactly once, preserves the shared non-scarce baseline through
  lineage, removes the old direct campaign writer, and leaves only derived
  agency-profile/cover projection outside the reducer. The nested regression
  moves one granary through two successive fissions without duplication while
  preserving John's exact identity and delta.
- Live refresh pressure reproduced and removed two contract faults: reuse of a
  private-command idempotency key across separately sealed attempts, and
  validation of a refresh receipt as if it carried the initial-login account
  summary. The old due session refreshed after deployment, emitted no refresh
  failure across the next scheduler pulse, and the same browser cookie still
  projected Session Zero revision 18 after daemon replacement.
- Adversarial Session Zero play proved that counters retire stale typed
  proposals before a replacement, generated openings and roles are optional,
  and transcript-only blank drafts cannot enter world compilation.
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
- Hosted campaign `34929b8d-7b04-49af-9936-1c798fd79760` advanced from revision
  12 to 13 through the shared `return_catch_up` path. Its eight-cell cover
  remained stable: five individual/cohesive actor or institution cells, two
  Gestalt cells with exact member activity, and one three-institution arena.
  Zhestokost's repeated posture was corrected to attributed inaction; Reed held
  the twelve patients; the player and Reed were byte-identical across the
  tick; five exact activities produced five bounded outcomes. The arena emitted
  no collective actor, actor knowledge did not become an information channel,
  and the actor-filtered player surface exposed none of the 14 remote channel
  reports.
- The successful revision-13 wave used 34 model stages and 35 provider attempts:
  83,838 prompt tokens, 64,000 cache-hit tokens (76.3%), 6,192 completion
  tokens, and 90,030 total tokens. One institution action needed semantic
  correction and one outcome bundle needed shape plus pressure-no-op
  correction; neither failed proposal mutated the campaign.
- A return catch-up could previously commit canonical state and then return a
  truthful stale-command receipt before refreshing its derived CultMesh
  operator surface. Strategic publication now lives in the shared tick-commit
  path. The release restart projected revision 13 without another world
  mutation, and the browser's existing Heimdall-backed app session survived.
- Retained operator witness:
  `F:\Projects\gamecult-ops\artifacts\ghostlight-operator-rev13-20260823.json`.
  The temporary local and Yggdrasil mesh copies were deleted after extraction.

Known acceptance limits remain visible. Revision 12 permanently records an
earlier semantic scar in this test branch: Mira Chen moved to the garrison while
trying to approach Reed. Do not repair that history out of band. The global
agency skeleton is still sparse (four institutions rather than a 20-plus-power
Aetheria proof), separate-account human co-op is untested, governed co-op time
still advances raw time rather than strategic cells, and a legacy campaign logs
a `gestalt member is not dormant` scheduler refusal; it is not this canary.
Large operator documents now traverse bounded reliable windows and complete
only after explicit acknowledgement; keep the transport and durable-snapshot
regressions in the fleet gate.

Acceptance witnesses live under `F:\GameCult\GhostlightDungeon\acceptance`.
Operational release and rollback truth remains in `gamecult-ops`.

## Next action

1. Expand and review the agency corpus from its current 36 candidate cases
   toward 300 behaviorally distinct cases; do not inflate it with reskins.
2. Remove the remaining legacy model effect schemas and continue the negative
   writer audit until aggregate campaign fields are projections only.
3. Resume adversarial Eve play, then fork or compile a denser 20-plus-power
   Aetheria skeleton and pressure budgets 1, 4, 8, and 32 with real provider
   output, including nested refugee dispersal and later rematerialization.
4. Run a two-person Yggdrasil Session Zero and bounded shared-scene canary using
   separate Heimdall accounts.
5. Then run the eight-account roster, privacy, actor-binding, pooled-budget,
   unanimous-approval, unanimous-time/travel, stale-command, and restart smoke.
6. Continue human pressure on multiresolution Gestalt agency: meaningful
   background surprises, exact attributed rival activity inside arenas,
   information boundaries, nested fission/folding, migration, and return
   encounters.

Do not weaken actor custody, private projection, unanimous approval,
knowledge gates, no-puppeting, or atomic-wave invariants to make the smoke pass.
Do not gate this testing on Epiphany unless the test explicitly exercises her
capacity.

## Essential references

- MVP authority: `docs/architecture/ghostlight-dungeon-mvp.md`
- Interface authority: `docs/architecture/ghostlight-eve-native-interface.md`
- Gestalt authority: `docs/architecture/ghostlight-multiresolution-agency.md`
- Transition authority: `docs/architecture/ghostlight-transition-algebra.md`
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
