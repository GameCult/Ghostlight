# Ghostlight Fresh Workspace Handoff

Updated: 2026-08-25

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

The immediate gate is continued adversarial play through Ghostlight's native
CultMesh boundary. The client has completed Heimdall OAuth, persists only an
opaque Ghostlight app-session bearer, fetches the same actor-filtered
`ghostlight.play` surface, and submits the same canonical Eve invocations as the
browser. Server-side refresh preserves that grant between client launches. The
36-case agency corpus remains only a candidate seed against a 300-reviewed-case
target. Separate-account multiplayer privacy and unanimity proof also remains
unproven.

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
  Projectors, Personas, Interpreters, retrieval, dice previews, and browsers
  may propose or project; none may commit canonical state. The player story is
  a deterministic lowering of exact committed transcript turns; no narrator
  model owns or rewrites it.
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
- The loopback `ghostlight.native.player` RUDP boundary is another client of
  that same app-session and Eve-command authority. It cannot accept browser-
  supplied actor, member, account, or campaign authority fields, and it does
  not create a second gameplay API.
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
  `e89db8bcadcd6e32af8b0349a99f45777c031536`
- Executable SHA-256:
  `eb79caa884ac324a5ac784fd872f785debe6c44cacea361539f0d8307a2bc29f`
- Native client SHA-256:
  `8be63daa65683e01561963d18a3303b998913b1c4b98b53fea31c9b4c663288b`
- Artifact SHA-256:
  `sha256-9f526356193458aad58ad92121cccaba337dd9838ebbf766490bdccfc15eba8d`
- Eve release: `19c3dcf9173dce848a6253e975324ea239a02d24`
- Service: `ghostlight-dungeon.service`, running as `ghostlight:ghostlight`
- Listener: Yggdrasil loopback `127.0.0.1:8831`
- Native player boundary: loopback RUDP `127.0.0.1:4102`
- Public route: `https://yggdrasil.gamecult.org/ghostlight/`
- Vault retrieval: local VoidBot at `127.0.0.1:17875/mcp`
- Stored state at the last witness: nine campaigns and three Session Zero drafts
- Model connector release:
  `80de3acc18e058f649c3b25ce617f8869f038670`; binary SHA-256
  `e8a909121e6bba56dfa4ebd4dec54b05ff8f463972118233556fb5db19858225`;
  provider `codex-connector`,
  physical fast/capable model `gpt-5.4`. This is the independent
  `codex-connector.service`; Epiphany is only another consumer.
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
Ghostlight advertisement whose source witness names `447f4027c37f13c83afb4431421d8d66b9384bcd`.
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

- Idunn admitted exact release `e89db8b…`; 313 locked Linux tests and the
  immutable release build completed before atomic activation. Typed health,
  manifest, embedded source, executable and native-client hashes, provider
  readiness, nine campaigns, three Session Zero drafts, and zero daemon
  restarts agree. `codex-connector.service` remained independently healthy at
  `80de3ac…` throughout the successful deploy.
- Repeated native authentication exposed a client-side custody bug: a second
  begin replaced the only pending handle after Discord had completed it. The
  live client now refuses a second begin, clears terminal owned attempts, and
  accepts an operator-recovered completed handle only through stdin. Three
  focused state-transition tests and Idunn's full Linux package gate pass.
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
- Automatic gestalt presence may recast exact dormant members after a relevant
  foreground event. It may admit one new first-relevance member only when the
  reason exactly matches the immediately committed player-speech turn.
  Resolved-attempt stakes, narration, and event summaries cannot mint people;
  both the model schema and `WorldKernel` close that lane. This cut followed a
  live relay attempt that correctly targeted two refugees but whose plural
  outcome prose caused a third `Relay Volunteer` to be created.
- The live replay held gestalt membership at four and then exposed an NPC
  assessment failure: the model twice returned `null` for an unavailable
  knowledge map. Foreground assessment schemas now remove unavailable
  knowledge, movement, clock, and institution lanes entirely, close the effect
  object to undeclared properties, and default omitted maps to no mutation.
  The focused regression and full 285-test library pass cover the cut.
- That failed assessment left a pending NPC choice which the next live reaction
  wave appended to and later resolved at revision 60. Pending initiative now
  belongs to one exact reaction-wave revision: a new wave replaces the old set,
  and the kernel rejects resolution after any intervening revision. Same-wave
  retry remains valid without allowing stale Persona output to rebase.
- The same live transcript exposed a second derived-authority breach: a
  two-stage narrator invented names, dialogue, and participation after the
  kernel had committed a narrower event. The narrator module, verifier, smoke
  target, runtime calls, response field, mesh snapshot lane, and surface
  replacement logic are deleted. Historical narration rows remain inert in old
  stores and are never projected. Browser and native clients now receive only
  exact chronological committed turns.
- Live refresh pressure reproduced and removed two contract faults: reuse of a
  private-command idempotency key across separately sealed attempts, and
  validation of a refresh receipt as if it carried the initial-login account
  summary. The old due session refreshed after deployment, emitted no refresh
  failure across the next scheduler pulse, and the same browser cookie still
  projected Session Zero revision 18 after daemon replacement.
- Adversarial Session Zero play proved that counters retire stale typed
  proposals before a replacement, generated openings and roles are optional,
  and transcript-only blank drafts cannot enter world compilation.
- Fresh native Session Zero
  `e7f2f947-25fa-4e1b-82e3-4a3e07dc4497` reached revision 36 with the exact
  negotiated Mars/Zhestokost contract and private Corvid character intact.
  Provider schema repair `0b3bd6a`, composite counter repair `f786e30`, and
  eight-player identity bound `4435bcb` each closed a live failure without
  publishing world state. Release `44eec9d` then converted the absent
  Nightwing/VoidBot retrieval path into one stable typed material blocker: no
  preview, world digest, branch assumption, or campaign was published, and the
  player surface did not expose the backend address or Ollama advice.
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

The live canary campaign `34929b8d-7b04-49af-9936-1c798fd79760` is at world
revision 87. Release `99a2508` constrained the trust attempt to its independently
verified relationship lane. Stale confirmation recompiled against revision 71;
the server-side d20 rolled 10, applied +4 against DC 5, and committed Member 1's
increased trust without granting knowledge or mutating the other two present
Personas. All three appraised the public event through their exact perspectives.

The same campaign then exposed a topology authority split: canonical route keys
are local to their origin, while the transition overlay had flattened them into
one global map. A surface-advertised camp-to-junction route therefore collided
with unrelated routes sharing its local key. Release `fc5cce0` derives component
edge identity from `(origin_location_id, local_route_id)` and keeps surface
destinations exact. The old failed command had durably reached unanimous
governance approval without committing the world; release `06ed2c5` makes that
uncommitted unanimous state idempotently finalizable through the same approval
operation. The exact old proposal advanced revision 74 to 75, moved only Ash to
Kostolom Junction, and added fifteen minutes. An Idunn same-release restart
changed the daemon PID while preserving release, app session, campaign, and
revision. The return route then advanced to revision 76 and the encampment
rematerialized with the same persistent location, Refugee Convoy population,
Member 1, Member 2, Relay Volunteer, relationship outcome, and player ledger.

The canary then reached the garrison and pressured institutional authority.
Release `bb5948c` makes the compact mutation-scope stage the owner of structural
admission: an impossible demand for Zhestokost's total surrender, resource
transfer, and permanent obedience returned an explicit no-roll refusal and four
bounded bargains after one model stage; the identical retry used the semantic
cache without advancing world state. Mira later stayed inside her clinic
authority, admitted that she did not know Voss's location or a magic citation,
and failed a separately receipted attempt to find acting authority.

Release `4057e11` keeps directly addressed Persona response generation in the
foreground command but schedules accepted NPC action proposals after the live
turn releases its commit guard. `Campaign.pending_world_proposals` remains the
only durable queue. Startup and the five-minute scheduler rediscover exact
current-reaction-wave proposals; assessment cancels when a player turn starts,
and `ResolveNpcAction` rejects any proposal whose revision window has moved.
Live native CultMesh evidence returned Mira's grounded records answer in 25.8
seconds at revision 86. Twelve seconds later the deferred initiative advanced
to revision 87 with an explicit spatial refusal: Mira could not inspect clinic
records while still at the garrison, and the kernel did not teleport her or
invent the missing findings.

Acceptance witnesses live under `F:\GameCult\GhostlightDungeon\acceptance`.
Operational release and rollback truth remains in `gamecult-ops`.

### 2026-08-25 retrieval and compiler pressure

Nightwing is reachable again at `192.168.178.21` and WireGuard
`10.77.0.3`; Ollama serves `qwen3-embedding:0.6b` to Yggdrasil. The real
VoidBot MCP path at `127.0.0.1:17875/mcp` returned source witnesses including
the richer Corvid sung-name canon. The stale steering claim that Nightwing and
Vault retrieval are absent is retired.

Session Zero `e7f2f947-25fa-4e1b-82e3-4a3e07dc4497` remains unpublished at
revision 60 with its Mars/Zhestokost contract and private poem-length Corvid
identity intact. The obsolete revision-55 preview was discarded through the
typed `SessionZeroKernel` command path before recompilation. The replacement
approval preview digest is
`sha256:6c79562d9a1399a3c9f63940c1c1968caa3a0d3644fe453aac6ab18babceb41c`.
It has four locations joined by an explicit complete directed route graph,
three fresh local actors, one terminal-yard Gestalt, three clocks, three local
institutions, and nine remote institutions including Pan-Solar Consortium.
Local terminal geometry and institutional texture are explicit branch
assumptions. Every admitted remote power has a concise approval-visible
campaign-local doctrine. Exact preview search found no First Exodus, Blackbox
Aviary, Kesh, or Maela Voss borrowing. The first live compile attempt timed out
at the `world_compile` stage after 120 seconds with 26,796 input characters;
the same-snapshot retry succeeded without partial publication. No player has
approved the new digest and no campaign state has been published.

The root cause was at the strict model boundary: open `BTreeMap` schemas were
closed into objects that legally permitted no keys. Prompts asked for routes,
relationships, facets, and assignments while the provider contract required
them to be empty. Model-facing compiler records now carry explicit route IDs,
relationship subject IDs, fixed agency axes, fission assignments, expansion
edges, and demand weights; local validation lowers them into unchanged
canonical maps. The topology validator still requires every supplied location
to be reachable from the player and back; containment remains geometry, never
movement authority. CodexConnector change
`b6b5102cb96e919bc30664ecad9b4701d8207e35`, inherited by live release
`80de3acc18e058f649c3b25ce617f8869f038670`, also gives streamed responses the
full body deadline instead of accidentally capping them at the header timeout.

The retained preview exposed an over-constrained remote-doctrine cut: it treated
the Vault as an exhaustive specification and omitted Pan-Solar Consortium when
its playable policy could not be textually entailed. The compiler now treats
exact institution evidence as canon anchors and synthesizes compatible
campaign-local doctrine around them. The verifier rejects contradiction,
identity conflation, story borrowing, and unanchored setting-breaking power;
mere source silence is not a rejection. Every anchored institution survives,
its generated doctrine appears as an approval-visible branch assumption, and a
second contradictory result aborts the whole compile without mutation.

`F:\Projects\Kalsa` is a purpose-built fantasy Vault candidate with player-safe
`Public` lore and GM-only `Spoilers`. It is not indexed or selectable yet. The
next Vault implementation must preserve those lanes through a typed manifest,
Git/Obsidian sync, retrieval, exact evidence receipts, and player projection;
do not mix Kalsa into the read-only Aetheria recovery index.

## Next action

1. Have the host review the exact revision-60 Mars preview and its material
   branch assumptions. Do not approve or publish digest
   `sha256:6c79562d9a1399a3c9f63940c1c1968caa3a0d3644fe453aac6ab18babceb41c`
   without that human decision.
2. After publication, audit the compiled agency skeleton and first resolution
   demand to prove all six axes survive the strict provider schema and every
   active subject has exactly one cell.
3. Resume away-time strategic activity and character-accessible news pressure
   through the authenticated native CultMesh client. Prove no player puppeting
   and no inaccessible remote-information leakage.
4. Then fork or compile a denser 20-plus-power
   Aetheria skeleton and pressure budgets 1, 4, 8, and 32 with real provider
   output, including nested refugee dispersal and later rematerialization.
5. Expand and review the agency corpus from its current 36 candidate cases
   toward 300 behaviorally distinct cases, and remove remaining legacy model
   effect schemas.
6. Implement selectable Vault manifests and use Kalsa as the first fantasy
   import canary with separate Public and Spoilers evidence lanes.
7. Run a two-person Yggdrasil Session Zero and bounded shared-scene canary using
   separate Heimdall accounts.
8. Then run the eight-account roster, privacy, actor-binding, pooled-budget,
   unanimous-approval, unanimous-time/travel, stale-command, and restart smoke.
9. Continue human pressure on multiresolution Gestalt agency: meaningful
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
