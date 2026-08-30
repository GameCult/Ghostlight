# Ghostlight Fresh Workspace Handoff

Updated: 2026-08-29

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

Hosted Dungeon adversarial play remains an active regression and
context-discovery lane through Ghostlight's native CultMesh boundary. The client
has completed Heimdall OAuth, persists only an opaque Ghostlight app-session
bearer, fetches the same actor-filtered `ghostlight.play` surface, and submits
the same canonical Eve invocations as the browser. The 36-case agency corpus
and separate-account multiplayer proof remain open regression work; neither is
the current consumer-fixture gate.

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
  `fdb6352e0dfdd0eeb129020ea9dea0b6225eccf9`
- Executable SHA-256:
  `d4c57cc6df0e2743f2f444bbdf4c2c90961a41b74e2c92cf45fa52b4e63f5714`
- Native client SHA-256:
  `0d54f75ce7fe66b876ae720cdf9d6314778972f2193410fe2979ec51b8d25aba`
- Artifact SHA-256:
  `sha256-feadb845bf999afb09e781f3e81a36fe5bc5b2d1d9eec06c028bbd81680a548a`
- Eve release: `672c0c1e9fc828b63752345385727b06c50491d0`
- Service: `ghostlight-dungeon.service`, running as `ghostlight:ghostlight`
- Listener: Yggdrasil loopback `127.0.0.1:8831`
- Native player boundary: loopback RUDP `127.0.0.1:4102`
- Public route: `https://yggdrasil.gamecult.org/ghostlight/`
- Vault retrieval: local VoidBot at `127.0.0.1:17875/mcp`
- Stored state at the last witness: twelve campaigns and six Session Zero drafts
- Model connector release:
  `b2e76841bfd43afcd0d92546e1cf6dd822129ef1`; binary SHA-256
  `97b690d9f010ea4b70490ca1e4cb09c727cc65499a8ddbe234ceab9f117c50c6`;
  provider `codex-connector`. The Ghostlight caller admission permits
  `gpt-5.6-luna`, `gpt-5.6-terra`, and `gpt-5.6-sol`; the live Dungeon unit
  still routes its logical fast and capable classes to `gpt-5.6-luna`. This is
  the independent `codex-connector.service`; Epiphany is only another consumer.
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

- Idunn admitted exact release `fdb6352…`; 347 locked Linux tests and the
  immutable release build completed before atomic activation. Typed health,
  manifest, embedded source, executable and native-client hashes, provider
  readiness, twelve campaigns, six Session Zero drafts, and zero daemon
  restarts agree. `codex-connector.service` remained independently healthy at
  `2f29b90…` throughout the successful deploy.
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
- Destination compilation resolves identity before generation. A genuinely new
  place uses bounded region expansion; an exact reachable canonical place uses
  in-place locality elaboration and cannot be re-emitted, renamed, or replaced.
  Both paths admit typed places, propositions, populations, institutions,
  political relations, topology, and the civic manifest through the same
  mutation reducer. Compiler-validated agency profiles remain a companion
  projection outside that reducer and are a named component-migration frontier.
  Inhabited proposals bind current authority, selection or succession,
  public resources, and redress facts to every resident population through a
  civic manifest, then pass an independent semantic civic verifier before
  approval. Aggregate state is rebuilt from accepted components.
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
`2f29b90eebd8fc1bbdc07cdd22ac4883f16849a5`, also gives streamed responses the
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

### 2026-08-26 Kalsa adversarial continuity and compiler boundary

Kalsa is a selectable typed Vault with separate player-safe `Public` and
GM-only `Spoilers` lanes. It retains Git/Obsidian provenance and exact evidence
receipts without entering the read-only Aetheria recovery index.

Session Zero `53c4e2ae-3620-42bc-9db7-7b345a544e55` published campaign
`e99e8794-281f-4a82-8b2c-5e6954bd6b16`. Player Asha Vey is bound through
member `member:42559dc743994576a8350528462488b0`. At the last witness the world
was revision 21, resolution epoch 3, configured budget 1, and the player had
returned to `loc:raincross_gate`.

Startup migration repaired Cal Rusk's malformed doubled legacy identity to
canonical `member:cal_rusk` without advancing fictional revision. Cal retained
the exact bypass and warning-mark knowledge learned before folding, answered
from it after rematerialisation, folded while Asha travelled to Veyr Run, and
returned as the same individual with the same delta. A one-hour budget-1
strategic wave advanced Ilya's investigation and Oren's movement without
puppeting Asha.

The live destination compiler then received a deliberately underspecified
request for a defensible emergency refuge. It synthesized eight visible
branch-local assumptions covering route geometry, capacity, supplies, repair,
custody, doctrine, evacuation closure, and inspection procedure while returning
zero material gaps. Its facts remain source-constrained by exact Kalsa receipts.
Preview `9fa863a0-e1fc-4115-bb18-74d523a3c6de` is unapproved and has not mutated
the world. This proves the live boundary: ordinary game-scale silence is
compatible branch elaboration; a material gap is reserved for contradiction,
an explicitly requested but unanchored canon baseline, or a conflict with an
approved capability.

## Next action

The generic world-consumer boundary is implemented. Session Zero and external
producers share one admitted `WorldSeed` transaction; externally authoritative
institutions or Gestalts enter through revisioned WorldKernel snapshots; and strategic
actions aimed at them persist typed proposals for consumer acknowledgement over
the existing loopback CultNet RUDP operation server. Consumers can request an
authority-gated `ghostlight.world.newspaper.compose` projection. It admits only
committed `NewsIssue` rows whose cited committed `Event` rows expose the same
public channel, then gives selection, grouping, tone, and copy to a bounded
editor model. A local validator and separate grounding copy desk gate the
proposal. Successful and identity-bound terminally rejected attempts persist
their model receipts idempotently. Reader Markdown escapes every plain-text field and omits source IDs,
channels, reliability, revision, and receipt IDs; a separate audit projection
retains that provenance. No editorial organ can write world state.

The scale and emergence gate is implemented and locally proven.
Delvehold is a consumer of Ghostlight's generic authored-world API, not a
Ghostlight-owned organ or special runtime. Ghostlight owns the world beyond the
Greathold as a persistent multiresolution political simulation, while Delvehold
retains player sovereignty and quantitative economy. WorldKernel now admits at
most one action-bound strategic individuation per wave through the existing
individuation commit primitive. The selector proposes identity content; the
kernel retains revision, lineage, location, uniqueness, materialization, and
atomic commit authority. The active budget ceiling is 240, independent of the
32-call provider gate. A fixture proves 1,000 subjects across 200 cells and all
200 membrane pipelines under a seven-call gate.

Sparse inhabited destinations now compile a versioned civic apparatus in place.
The first pass establishes institutions, resident populations, political edges,
and public authority, succession, resource, and redress facts. A later pass is
given that exact persisted apparatus and must deepen it without duplicating its
government or population. The independent civic verdict is rebound to the exact
candidate and checked again by `WorldKernel`; structurally plausible politics
without that receipt cannot mutate the world. A two-wave fixture proves an
action-bound named person can emerge publicly, retain identity, enter the next
resolution cover, and act from her own authority. The generated newspaper
contains her public appointment but does not expose her later private movement.

The accepted four-revision Rainless Marches provider witness is recorded in
`state/evidence.jsonl` and
`F:\GameCult\GhostlightDungeon\acceptance\rainless-spicy-20260827-sustained-5\SHA256SUMS`.
Exact Ghostlight source `5e00a7bbc3f540ef35d18abe89f84687b5004664`
ran through Codex Connector
`2f29b90eebd8fc1bbdc07cdd22ac4883f16849a5`, model `gpt-5.6-luna`,
and admitted caller `ghostlight-dungeon-yggdrasil`. Four sequential atomic waves
advanced one campaign from revision 0 through 4, committed 41 unique events and
28 unique news rows, and
rendered 23 distinct provenance-bound articles across four issues reviewed for
political continuity and exact ID provenance; this was not a blind journalistic
review. Ilyra Quill appears in every issue; Mara's testimony persists
into later institutional postures; Tavia and Mara Venn emerge as accountable
organizers. One rejected wave-one pulse did not mutate the world. Player state
and location are unchanged, stderr is empty, and every copied artifact matches
the remote SHA-256 manifest. This proves sustained politics in a sparse provider
world whose derived cover grew from four cells to six as named figures emerged;
the 200-cell path remains a local scale proof and the
Delvehold game-side adapter is not implemented here.

Run 16 used exact source `6e9e7043d95d82ec694c45cdbfbc552a89e74ff7`
at `/var/lib/gamecult/ghostlight-dungeon/acceptance/elven-realms-autonomous-6e9e704-16`.
The strict one-value civic discriminator worked: two locality elaborations
committed before the third was correctly rejected because relation
`rel_inquest_cohort_communication` was not an exact new local-subject edge.
That run moved the blocker from schema constants to bounded civic repair.

Commit `4d96018` gives that repair one owner. Structural and semantic validation
publish one pending finding; one candidate-acquisition boundary may ask Terra
to repair the frozen Sol candidate's exact civic slice; the ordinary structural
validator and a fresh semantic verdict still decide admission. Missing civic
material remains terminal, and neither branch can independently invoke a
second repair. Run 17 at exact source
`4d960188cd6f5495ba808078d3d3b169f1b80790` exercised that path at
`/var/lib/gamecult/ghostlight-dungeon/acceptance/elven-realms-autonomous-4d96018-17`.
It stopped on `destination institution inst:verge-moot has a malformed agency
profile`: the broad finding did not tell the bounded repair owner whether the
location, six facets, or public channel was malformed. Commit `8ee836f` exposes
those exact model-owned diagnostics at the existing validation boundary; it
does not add another reconciliation attempt or owner.

Fresh exact-source run 18 at
`/var/lib/gamecult/ghostlight-dungeon/acceptance/elven-realms-autonomous-8ee836f-18`
completed all four locality elaborations and five strategic waves. Revision 9
contains 101 Events and 232 NewsIssues, and five unedited `The Canopy Ledger`
issues were rendered. Wave 6 did not commit. Its terminal artifact records that
the strategic outcome verifier rejected the corrected bundle identified by
`sha256:534e9fe02556783a0cde2ce7d3cd22d734709e5eb6524c0255c6af09025a6c62`:
the proposed material fact was not established by the witnessed grain sampling
or concrete examination of the document. The result root has no completed
`SHA256SUMS`. Run 18 proves the civic compiler and sustained-world path advanced
well beyond runs 15-17, but it is not a completed acceptance witness. No blind
editorial or grounding pass is claimed. Fantasy-newspaper presentation remains
unaccepted.

Commit `3bd1279e2ae557b17df1483de4f7842cd70866a3` is pushed and owns
same-snapshot strategic outcome reconciliation without replaying an otherwise
accepted wave. Run 19 used that exact source at
`/var/lib/gamecult/ghostlight-dungeon/acceptance/elven-realms-autonomous-3bd1279-19`,
but did not reach the run-18 outcome path. Two locality elaborations committed
through revision 2. The third, Oldest Forest Margin, stopped after the one civic
reconciliation response returned institutions that did not preserve the
bounded unique new canonical subject set; local validation reported
`destination institutions need at most twelve unique new canonical subject
IDs`. No strategic wave or newspaper was produced, and the root has no
completed `SHA256SUMS`. This leaves `3bd1279` pushed but not provider-accepted
on its intended outcome path. Fantasy-newspaper presentation remains
unaccepted.

The run-19 diagnosis found that reconciliation still returned complete
institution replacements, so Terra could discard or invent subjects while
repairing civic operations. Commit
`2875e45b3abd9c8c5e207d85444c797884c57136` freezes every Sol-authored
institution ID, name, and goal. Terra receives an exact-length, exact-ID schema
for civic-operational updates only; the existing merge and admission owner
applies resources, posture, facets, locations, and public channels to the frozen
institutions before ordinary validation. There is no second repair path. All
408 library tests and the strategic smoke check pass, and the commit is pushed.

Exact-source run 20 at
`/var/lib/gamecult/ghostlight-dungeon/acceptance/elven-realms-autonomous-2875e45-20`
committed its first locality elaboration through revision 1. During the second,
at Downstream Grain-Pump Station, Terra's civic reconciliation returned
expansion facts that failed ordinary validation: `destination expansion facts
must have new IDs and non-empty unique statements`. No strategic wave or
newspaper was produced, and the result root had no completed `SHA256SUMS`. The
unchanged root is archived at
`F:\GameCult\GhostlightDungeon\acceptance\elven-realms-autonomous-2875e45-20.tar`,
whose SHA-256 is
`6cc564ec70dd6e0279180fb547610d9355418ecdf1ff2cc4f083480ce7841b43`.

Commit `965c9faadcde9a8dced158b00e37d9491d589cde` centralizes one
maximum-two Terra reconciliation budget across candidate application,
structural validation, and civic-verifier findings. All findings return to the
same frozen-candidate scheduler; no branch owns its own retry and Sol is never
rerun. Ordinary validation and a fresh semantic verdict remain admission
owners. All 409 library tests and the strategic smoke check pass, and the commit
is pushed.

Exact-source run 21 terminated during its second locality elaboration at
`loc:pump_house`. It committed one elaboration and zero strategic waves before
both bounded Terra reconciliation actions still returned destination facts
rejected by ordinary namespace validation. No newspaper or completed
result-root `SHA256SUMS` exists. The unchanged failure root is archived at
`F:\GameCult\GhostlightDungeon\acceptance\elven-realms-autonomous-965c9fa-21.tar`,
whose independently rechecked SHA-256 is
`2f4d48d47acd3733c99ab8899427e1d33bd4b5cde93da52efa67a3baf98da8e3`.
This falsifies the active-run steering claim and demonstrates that a fixed
retry scheduler was not giving the reconciler an agentic observe-act loop with
the exact rejected action and tool finding.

Commit `842e265f2969d7047e6b2a35fcca7cc371de5c6b` makes that ownership
explicit. A generic, provider-neutral harness above `ModelPort` owns bounded
semantic steps, causal receipt chaining, and rejected tool observations. The
destination adapter owns typed candidate application, one canonical
fact-namespace checker, ordinary lowering and validation, and a fresh one-shot
civic-verifier call. The compiler performs one agent dispatch and WorldKernel
admission remains final. This harness is hand-rolled over raw model calls rather
than coupled to the Codex app server.

The focused regression proves the complete repair path: the frozen Sol
candidate collides with an exact existing fact; Terra's first action changes it
to a different collision; the adapter returns that exact typed finding; Terra's
second causally receipted action corrects it; ordinary validation and a fresh
one-shot verifier accept it while institution ID, name, and goals remain
frozen. All 411 library tests pass. A bounded Modeling re-audit passed the
transport, harness, destination-adapter, and kernel authority boundaries. The
local strategic smoke binary builds, but its runtime selected the workstation's
stale DeepSeek route and received HTTP 402 because this machine has neither a
Codex Connector service nor a usable key. That is not provider acceptance or a
harness failure; the provider smoke belongs on Yggdrasil.

Exact-source run 22 exercised commit `842e265` through Codex Connector. The
world compiled and locality 1 committed. At locality 2 the Sol destination
compile, Terra reconciliation action, and fresh Sol civic verifier all
accepted; WorldKernel then rejected admission with `region admission evidence
receipts were not supplied`. The run ended at one of four elaborations and zero
of six strategic waves, so it produced no newspaper. The unchanged root is
archived at
`F:\GameCult\GhostlightDungeon\acceptance\elven-realms-autonomous-842e265-22.tar`,
whose SHA-256 is
`90972df1c90eaff699469357ff5de026838940ac07ec5f5faefbaf5d1d266ed9`.
This retires run 21 as the current blocker while preserving it as prior failure
history. Run 22 proves the provider-backed agentic reconciliation path reached
fresh semantic acceptance; the remaining rejection came from mixing causal
ModelAgent receipt IDs into canonical profile and relation evidence fields.
Commit `c757e5a09d884d263ae3533f537a71ce69010e94` owns the focused
namespace split and is pushed: model-causal ancestry remains available to
the harness and verifier receipts, while profiles and relations lower only the
canonical admission-evidence receipt IDs. Provider verification remains
pending.

Verified pushed commit `71c274b0b8819b52cddb1fadbefbf9c634d1f0b2`
makes the strategic Interpreter another consumer of the same generic harness.
Shared `causal_source_ids` bind both actor
and cell Projector-to-Persona-to-Interpreter chains. One agent incrementally
edits a private draft against the frozen Persona decision. The deterministic
draft compiler owns exact structure, permissions, topology, and bounds; the
independent semantic verifier returns subject-scoped findings; unchanged
accepted actions retain their exact verifier binding. Missing cell decisions
reuse one exact `CellProjectedMoment`: one Projector call feeds two Persona and
two Interpreter attempts, and the second attempts cite the first. Draft
admission rejects whole resubmission and edits to unrelated decisions. The
former bespoke correction prompt, whole-cell retry ownership, and dead retry
trace fields are retired. All 419 library tests pass, and the second bounded
Modeling audit found no P1 or P2 issue.

World elaboration now has a typed catalog of eight named workers: Patina,
Charter, Ledger, Hearth, Tangle, Veil, Ember, and Numen. Persisted semantic
profile weights allocate against the complete enabled profile and control actual
invocation frequency through a deterministic smooth weighted scheduler. An
ineligible or blocked title keeps its scheduled share visibly unused; the
scheduler does not redistribute those slots among eligible titles or accrue
catch-up debt. With Patina 20 and blocked Tangle 80, a 100-slot window therefore
invokes Patina 20 times and reports 80 unused slots. A bounded invocation port
runs scheduled sub-agents in parallel against one frozen snapshot and restores
deterministic ordinal order. Every invocation and proposal carries an exact
`ElaborationWaveBinding`. Typed failures retain the consumed schedule,
completed invocations, and exact failed dispatch for ordinary worker errors and
task panic or cancellation; task-ID mapping preserves dispatch identity even
when the task produces no normal payload.
Pushed implementation commit `0654ba9` makes the local world-domain path an
actual provider-backed titled swarm. Each scheduled title runs through the
generic agent harness against its dispatch-bound frozen projection: Patina,
Ledger, Hearth, Veil, and Ember use the fast class; Charter and Tangle use
balanced; Numen uses capable. Each invocation may submit one assigned typed
operation to a deterministic tool, while retaining the full model/tool receipt
chain. The strategic acceptance runner now schedules seventeen titled calls per
locality after the existing civic-foundation compiler: three Patina calls and
two for every other title under bounded parallelism. This is a hybrid proof of
the new path, not a replacement of the old foundation compiler.

Titled workers emit a closed additive `WorldElaborationOperation` algebra;
deterministic admission verifies the authentic frozen schedule before assigning
first-dispatch write claims, reports exact later conflicts, and merges accepted
operations into one compiler-owned locality candidate. An independent civic
verifier may accept or reject that candidate but cannot rewrite it. Opaque,
serialize-only admission and finalization capabilities prevent callers from
fabricating a convenient schedule or canonical candidate. `WorldKernel` then
revalidates and delegates through the existing `ElaborateLocality` lowering,
mutation reducer, aggregate projection, and atomic CultCache persistence path.
Titled workers never receive a world command, permit, mutation batch, store, or
kernel handle. A worker built against a stale campaign rejects a newer wave
before inference. World admission rejects receiptless successful proposals, and
the semantic verifier must cite exactly the admitted worker-receipt set. Titled
operations cannot mint source-evidence authority. The runner takes custody of
worker receipts once before admission and of the verifier receipt immediately
after inference. The algebra currently elaborates additive locality state only;
  it does not replace existing canonical subjects. All 426 library tests pass.
The exact end-to-end regression commits Patina's weathered bronze duck statue,
locally called Harold, through a real parallel dispatch and the persisted exact
mutation batch. A bounded Modeling/Soul audit of the provider-backed path found
no remaining P1 or P2 issue.
Provider run 23 remains stopped. Exact-source run 24 used executable
`0654ba90702be4de128cfa01a3f064737ec35244` in a new isolated root and failed
before its first titled wave. The old civic-foundation reconciliation agent
exhausted four steps because the verifier demanded `shared_fact_ids` while its
candidate projection exposed only canonical `shared_knowledge` statements.
The unchanged failed root is archived at
`F:\GameCult\GhostlightDungeon\acceptance\elven-realms-autonomous-0654ba9-24.tar`,
independently matching SHA-256
`8ED0A978C3A4088C6E6B26C7F9FF4C648799123983DF1DAF686692D76A086084`.
No strategic wave or newspaper was produced.

Pushed executable source `5f559a6e67bf601f3ab2dabc88b40379c23db954`
repairs that projection seam. Reconciliation still owns compiled resident fact
IDs; lowering still owns canonical statement knowledge; the verifier now
receives an exact derived ID view from the admitted fact namespace and cannot
mistake storage shape for a repairable omission. A bounded Modeling/Soul audit
found no remaining P1 or P2 issue.

Exact-source run 25 used that commit and advanced through its first civic
foundation plus all seventeen provider-backed titled calls without rejecting an
operation. It then stopped at titled civic verification. The runner had supplied
the original foundation count request to an additive titled candidate, while
`civic_context` projected five governing institutions but omitted the admitted
non-governing Gauge and Root Commission referenced by a retained political
relation. No strategic wave or newspaper was produced. The unchanged root is
archived at
`F:\GameCult\GhostlightDungeon\acceptance\elven-realms-autonomous-5f559a6-25.tar`,
independently matching SHA-256
`A4B4E8A0D83E1D4AAEDB0F2E542E9E8A0D2F63026B2D58AAB2C7DBB4AAB2FBC2`.

Pushed executable commit `975ee46` makes the prepared worker the sole titled
task-string owner used by inference, artifacts, and semantic verification; the
foundation request is out of scope before the titled pass. Civic context keeps
governing institutions exact while separately projecting relation-linked
institutions, populations, and named people. All 427 library tests, all five
strategic-smoke tests, and the locked smoke check pass. Bounded Modeling review
found no P1 or P2 issue. This is the exact executable body for run 26. The
current `gamecult-ops` owner at commit
`d59a23837e58ff74a352db4e3223ff5e1abe026b` records the connector release and
three-model Ghostlight allowlist above. Recheck service state, loopback `4103`,
the deployment receipt, and caller admission on Yggdrasil immediately before
dispatch; the ops ledger is not a substitute for a live health witness.

Exact-source run 26 used
`975ee4613932cbac04f01653ed90537846b36ded`. It completed the first foundation
compile, all seventeen provider-backed titled calls, deterministic admission,
and titled semantic verification. The titled preview contains seventeen
accepted operations, no operation rejection, and seventeen model-receipt
hashes. Atomic persistence then failed with
`immutable world-commit companion conflict` for
`persona_stage_receipt.v1/sha256:5cf4deb7b6477f9cbdd603bc34792c26fd3ae8c10026e5f38bdb88a09c608688`.
Status remained world revision 0 with zero of four elaborations and zero of six
strategic waves; no newspaper was produced and `BUILD-WITNESS` records
`live_service_mutated=false`. The unchanged root is archived at
`F:\GameCult\GhostlightDungeon\acceptance\elven-realms-autonomous-975ee46-26.tar`,
independently matching SHA-256
`007717BD13AF2BEF6B1A713BA74A89D065C34B473714F76E3EB73EA059123D70`.
This retires titled semantic verification as the current blocker. The live
defect is now at immutable model-receipt companion persistence; run 26 must not
be resumed or repaired in place.

Pushed executable commit
`8fe7a5563c7813d0a736a108069b236f217c0e07` removes the split receipt-equality
authority exposed by run 26. Direct receipt persistence and atomic world commits
now use `ModelStageReceipt::same_receipted_content`; the first stored envelope
and its observational telemetry remain byte-for-byte immutable, same-key input
is deduplicated before CAS construction, and a semantic collision still fails
before campaign mutation. The focused regression proves both the successful
pre-persisted-receipt path and the negative collision path. All 429 library
tests, all five strategic-smoke tests, and the locked strategic target check
pass. A bounded Modeling/Soul audit is clear.

Exact-source run 27 used
`8fe7a5563c7813d0a736a108069b236f217c0e07`. Its first civic foundation and
titled elaboration committed, advancing world revision to 2 and proving the
run-26 receipt-persistence repair. The second civic foundation completed. Its
titled wave then stopped after Charter dispatch ordinal 27 exhausted two
semantic steps: the deterministic tool returned only `proposal does not
satisfy the exact titled elaboration task assignment`, which did not tell the
agent which exact assigned operation or identity to correct. The other sixteen
invocations completed. Status remained at one of four elaborations and zero of
six strategic waves; no newspaper was produced and `BUILD-WITNESS` records
`live_service_mutated=false`. The unchanged root is archived at
`F:\GameCult\GhostlightDungeon\acceptance\elven-realms-autonomous-8fe7a55-27.tar`,
independently matching SHA-256
`D98C86A2AC173128E6C7D976596C60050FD6143B8B1E70DA0180200D77E92BC0`.
This retires immutable receipt persistence as the current blocker.

Pushed executable commit
`d6cfda23855414d1297b47d6d476a82df98ee43f` keeps assignment validation at its
existing deterministic owner but appends that assignment's exact correction
contract to the tool finding. Its focused Charter regression proves the
finding names the required operation, exact assigned ID, and bounds. All 430
library tests, all five strategic-smoke tests, and the locked strategic target
check pass; bounded Modeling/Soul review is clear. Provider verification
remains pending.

Exact-source run 28 used
`d6cfda23855414d1297b47d6d476a82df98ee43f`. Its first civic foundation and
titled elaboration committed at world revision 2. The second foundation and
all seventeen titled operations completed, proving the run-27 exact-assignment
finding. Before titled semantic verification or commit, deterministic candidate
closure rejected with `candidate needs at least two governing institutions`.
The admitted civic manifest actually declared one governing institution while
six institutions were available at the locality; the validator had counted the
available set instead of the declared governing set. Status remained at one of
four elaborations and zero of six strategic waves; no newspaper was produced
and `BUILD-WITNESS` records `live_service_mutated=false`. The unchanged root is
archived at
`F:\GameCult\GhostlightDungeon\acceptance\elven-realms-autonomous-d6cfda2-28.tar`,
independently matching SHA-256
`E216E2FE387F4EB0BC1E984EC63E963DA001B4E764BA0EE35B92275356523499`.
This retires generic titled assignment correction as the current blocker.

Pushed executable commit
`d71d35bb897db3487ac61ade97160292040bf6d7` keeps civic closure at its existing
deterministic owner but counts `civic.governing_institution_ids` rather than
the available institution namespace. It also corrects the shared valid fixture
and adds a focused regression. All 431 library tests, all five strategic-smoke
tests, and the locked target check pass. Bounded Modeling/Soul audit is clear.

Exact-source run 29 used
`d71d35bb897db3487ac61ade97160292040bf6d7`. Its first civic foundation and
titled elaboration committed at world revision 2. The second foundation
reconciled and emitted `elaboration-02-preview.json`, proving the run-28 civic
closure repair. Sixteen of seventeen titled invocations completed. Charter
dispatch ordinal 27, title count 4 exhausted two semantic steps even though
its final tool observation specified the exact required `add_fact` ID
`elab:loc_grain_pumps:charter-fact:4`, branch-local scope, empty evidence,
exact discoverability, and content bounds. Status remained at one of four
elaborations and zero of six strategic waves; no newspaper was produced and
`BUILD-WITNESS` records `live_service_mutated=false`. The unchanged root is
archived at
`F:\GameCult\GhostlightDungeon\acceptance\elven-realms-autonomous-d71d35b-29.tar`,
independently matching SHA-256
`4A3855314B02CEEDFCBE6345B0DE80DB03D9F97421D0716BB6AF0382D60ACE8D`.
This retires civic closure as the current blocker.

Fresh source inspection falsified generic ModelAgent correction handling as the
run-29 owner. The harness correctly appended the rejected action and exact
finding. `WorldElaborationAssignment` described one assigned operation in its
instruction and frozen validator but gave the model the full eight-variant
`WorldElaborationOperation` union, so the model could keep selecting a
structurally legal operation that could never satisfy this assignment.

Pushed executable commit
`007e63b33768224864c3d32892fa5c8e1f76954b` makes each assignment own its
instruction, generated exact one-branch action schema, and frozen validator.
The schema fixes exact operation identity and every deterministic field before
inference while leaving genuinely authored content open. `AgencyRelation::SCHEMA`
is the sole production relation-schema literal. All 434 library tests, 16
focused elaboration tests, five strategic-smoke tests, the locked target check,
diff check, and bounded Modeling/Soul audit pass. The generic ModelAgent harness
remains consumer-neutral correction transport. Provider verification is pending.

Exact-source run 30 used
`007e63b33768224864c3d32892fa5c8e1f76954b` but failed before its first
locality commit. The foundation compiled and its civic verifier returned
semantic-invalid. `destination_reconciliation_agent_action` then returned
malformed JSON twice with `trailing characters at line 1 column 14903`.
`status.json` records zero of four elaborations, world revision zero, and zero
of six strategic waves; no newspaper was produced and the live daemon was
untouched. The terminal failure artifact preserves four completed model receipt
hashes. The unchanged archive is
`F:\GameCult\GhostlightDungeon\acceptance\elven-realms-autonomous-007e63b-30.tar`,
independently matching local and remote SHA-256
`6B06BA76E06B11E423D79A37A4B4301EB4B32CDAC4D9C3F8578FAD7B1FB0AAF5`.
Run 29's assignment-schema diagnosis and the local verification of `007e63b`
remain valid; run 30 did not reach the titled swarm and therefore did not
provider-verify that cut.

Pushed executable commit
`49a0292464488e1c2f4361042092058baac92dcd` confirms the run-30 ownership
diagnosis and makes reconciliation incremental. The worker receives the exact
typed initial finding and operates a private transactional draft through only
`validate_current` or a bounded `revise_and_validate` edit batch. The action
schema cannot expose the complete candidate. Every action runs the actual fact
namespace, lowering, structural, and independent civic validators; the civic
verifier rationale is structurally bounded to 1..1000 characters. All 438
library tests, 16 focused elaboration tests, five strategic-smoke tests, the
locked target check, and bounded Modeling/Soul audit pass. Provider verification
and newspaper acceptance remain pending.

Exact-source run 31 used
`49a0292464488e1c2f4361042092058baac92dcd`. Its Sol foundation completed and
the run entered titled elaboration wave one. Patina's three weighted dispatches
succeeded. The other fourteen dispatches were rejected by the Codex/OpenAI
provider before model invocation because their strict response schemas used
compound array-valued `const` values, including exact civic-ID arrays and empty
evidence arrays. The titled wave did not commit. Run 30's frozen archive and
hash remain unchanged provenance; provider and newspaper acceptance remain
pending.

Pushed executable commit
`ec153a9e8733b07a5b0827f6519b518a971f8779` lowers compound literals once at
the shared `model_connector` Responses-schema boundary. Exact-source run 32
proved that cut on the real provider path. Canopy Verge completed its civic
foundation and all seventeen titled agents with zero rejections, then committed
world revision 2. Its weighted dispatch was three Patina calls and two calls for
each other title. Grain Pumps completed its civic foundation and sixteen of
seventeen titled calls. Numen ordinal 25 timed out after 300 seconds on 38,339
input characters, so the titled wave did not commit. The unchanged local and
remote archive is
`F:\GameCult\GhostlightDungeon\acceptance\elven-realms-autonomous-ec153a9-32.tar`,
independently matching SHA-256
`484EE490F9054055539B1A65534A714057D06AB27F64B5E5A5DE91844AE3CAEB`.
Run 30's archive remains unchanged provenance. Provider and newspaper
acceptance remain pending.

Grain Pumps' civic foundation is committed. The sixteen successful titled
proposals and their receipts are completed work; only Numen ordinal 25 is
missing. The operator explicitly rejects replaying run 32 from the beginning.
Run 32 established the no-replay checkpoint rule; the later pushed resume chain
consumed that checkpoint rather than replacing it with a fresh invocation.

The pushed resume chain advanced beyond that checkpoint. Exact-source run 36
used `2260665984332bee29bd0a722bb41a149d78c743`, resumed from
`/var/lib/gamecult/ghostlight-dungeon/acceptance/elven-realms-autonomous-c581893-35`,
and reached world revision 8 with all four locality elaborations committed. It
then failed atomically in strategic wave one at resolution epoch 4. Nineteen
cell Interpreter pipelines exhausted five semantic steps: ten ended on
`invalid_action_shape`, five on `effect_mismatch`, two on
`submit_requires_empty_draft`, one on `local_validation`, and one on
`draft_progress`. The wave terminal artifact records 333 model receipt hashes,
zero accepted rejected-pulse retries, zero committed strategic waves, and no
newspaper. `BUILD-WITNESS` records `live_service_mutated=false`. The exact remote
artifact root is
`/var/lib/gamecult/ghostlight-dungeon/acceptance/elven-realms-autonomous-2260665-36`.
No immutable run-36 tar/hash is recorded locally. `status.json` remains a stale
three-elaboration progress projection; the four titled commit files and the
revision-bound wave terminal artifact are the later evidence.

That tagged-command cut was pushed as
`acd544bae6932aa86aca85285a291a342da697a9`. Exact-source run 37 resumed the
run-36 world at revision 8 without replaying any of the four committed locality
elaborations. Strategic-wave pulses one and two each completed zero of 52 cells
and failed all 52 after Interpreter pipelines exhausted five semantic steps
with `draft_progress`/`draft_required`. The unit was stopped before pulse three.
No strategic mutation committed and no newspaper was produced. The preserved
remote root is
`/var/lib/gamecult/ghostlight-dungeon/acceptance/elven-realms-autonomous-acd544b-37`;
its `BUILD-WITNESS` records the run-36 resume source and
`live_service_mutated=false`.

Run 37 falsifies static tagged variants as a sufficient repair. The advertised
schema exposed inspect, remove, upsert, and submit together while the prompt
described a state-dependent hybrid, so the workbench did not own the currently
legal transition. Commit `deaa37eee347d0b6f5d6817c20368ba187fb2d05` moves that authority into each
`ModelAgentTool`'s per-step schema. Interpreter initially exposes only a
complete submit; after rejection it exposes only exact rejected-owner upserts.
Inspect, remove, and the pseudo-schema are deleted. Upsert preserves the full
admitted multi-owner repair set until `compile_draft` derives the next state; a
two-owner regression proves B remains repairable after A changes and submit
stays unavailable. Independent Modeling re-audit is clear after its P2
correction. Final verification is 448 library tests, nine strategic-smoke
tests, two projection tests, the locked strategic target check, rustfmt, and
diff checks.

Exact-source run 38 at
`596e50679244b5d42b4cfdf70d723f815751b8ff` resumed the run-37 world at
revision 8 without replaying compilation or any of the four committed locality
elaborations. Dynamic per-step schemas recovered 51 of 52 strategic-wave cells
on pulse three and the final cell on pulse four. All six waves committed through
world revision 14. Issues 1 and 3 through 6 and a final digest were emitted;
wave 2 was withheld by the tense-inflation guard. The immutable local archive
is
`F:\GameCult\GhostlightDungeon\acceptance\elven-realms-autonomous-596e506-38.tar`,
independently matching SHA-256
`38637A8DC14F04EBF1C7B1BB4675B2456379EECB1D59E7982CFF3279C06E9FAF`.
`BUILD-WITNESS` records `live_service_mutated=false`.

Run 38 accepts the provider execution path, not the newspaper. The emitted
copy is procedural rather than spicy, and the missing per-wave Issue 2 leaves
the sequence incomplete. Editorial acceptance remains failed.

Pushed commit `3a69306f3004e4af2cd57bba612a6ad8d1b8132f`
implements the mapped clock-consequence owner and is verified locally. World
clocks own declared consequence text and exact observable scope. A bounded
agent may propose legacy scope binding; `WorldKernel` alone admits it and
materializes stable threshold-crossing events and public news through the
common mutation path. Binding, emitted-event ancestry, mutation receipts, and
the next newspaper boundary persist atomically.

Exact-source run 39 is terminal complete at world revision 21 with twelve of
twelve waves. It resumed run 38 without replaying its six committed waves,
bound clock consequences, committed six new waves, and emitted Issues 7 through
12 plus a final digest. All inherited wave 1-6 checkpoint hashes match run 38
exactly. The immutable local archive is
`F:\GameCult\GhostlightDungeon\acceptance\elven-realms-autonomous-3a69306-39.tar`,
independently matching SHA-256
`FA259C699464DF1575022DFAAEA51A3D02E1410770369D14BFE570AC71D24838`.
`BUILD-WITNESS` binds exact source
`3a69306f3004e4af2cd57bba612a6ad8d1b8132f`, the run-38 resume root, and
`live_service_mutated=false`.

Mechanical provider acceptance is complete. Independent blind editorial review
returns FAIL: only Issue 7 approaches spicy, while Issues 8 through 12 are
dominated by plans, filings, inspections, warnings, and abstract relationship
deltas without betrayal, scandal, public reaction, sustained reversal, or
material lived cost. Independent grounding
review also returns FAIL: Issue 9 inflates a planned expedited agenda into
completion, while Issue 8 assigns unsupported feminine pronouns to Sela Venn
and Eryn Tal. Citation resolution, temporal bounds, player immutability,
manifest integrity, archive integrity, and live-service isolation otherwise
pass. Editorial and grounding acceptance are terminally failed for run 39.

Pushed source through `92c7647014dc3ffc53bc89d5bd291b81ba053e81`
contains Nemesis causal attention, exact kernel receipt admission and durable
served-pair history, exact causal outcome context, bounded complete mismatch
feedback, and one-pass exact-phrase grounding reconciliation. Nemesis may bind
exact committed pressure anchors to eligible autonomous decision owners or
return an empty judgment; it cannot choose their actions or outcomes.

Newspaper grounding repair is a `ModelAgentTool` transaction over the same
frozen desk and complete finding set. One bounded pass may replace only an
exact uniquely occurring phrase named by a finding or delete its flagged
article, then must undergo fresh whole-page verification. Focused tests pass
for exact-receipt kernel admission, exact causal outcome context, legitimate
empty judgment, durable replay suppression, narrow one-pass exact-phrase
grounding repair, and bounded complete mismatch feedback.

Exact-source run 40 resumed immutable run 39 without replay, committed waves 13
and 14 through revision 23, then exposed a responder-cell identity defect at
wave 15. That materialized-versus-dormant routing fix was pushed in exact source
`46810c31b15c880026ee22c0318a563e638c42bc`.

Run 41 cloned run 40 at committed revision 23, removed only copied wave-15
failure artifacts, and did not replay prior waves. It committed waves 15 through
18 through revision 27 with 363 events and 853 news rows. Issue 14 is accepted;
Issues 13 and 15 through 18 are absent because their old grounding repair failed
after the corresponding immutable world-wave commit. Final newspaper composition
also failed. The remote root is
`/var/lib/gamecult/ghostlight-dungeon/acceptance/elven-realms-autonomous-46810c3-41`.
`BUILD-WITNESS` binds the run-40 resume root and
`live_service_mutated=false`. Simulation mechanics are complete; the derived
publication sequence and final digest are incomplete. Runs 38, 39, and 40
remain preserved, and acceptance is incomplete.

Pushed `19a66a4` groups newspaper grounding replacements by editable target,
validates every exact phrase once against the original copy, rejects overlapping
ranges, and atomically applies distinct non-overlapping replacements in reverse
source order before requiring the complete finding set and fresh whole-page
verification. Its defining regression, all newspaper tests, the full 468-test
library gate, and `git diff --check` pass. Provider and editorial acceptance
remain open.

Pushed `8a971da` adds a presentation-only recovery seam for missing per-wave
editions. It derives each absent issue from that wave's immutable committed
campaign snapshot and exact prior committed news boundary, publishes an
immutable recomposition receipt, and skips accepted issues such as Issue 14. It
fails closed when the exact prior boundary is unavailable, including a missing
first-wave issue. Every recomposition receipt binds source, contract, issue,
typed model receipts, rendered content, and file digests; typed receipt JSON
roundtrip is covered. Its local gate passed 12/12 strategic-smoke binary tests,
468/468 library tests, and `git diff --check`.

Exact-source run 42 at
`8a971daab8d3dfc77c29d69b3d139ff71d53314a` cloned run 41 at revision
27 without replaying or mutating world mechanics. Recovery selected historical
missing Issue 2 before the requested run-41 editions; the copy desk rejected
unsupported `prepares`. Issues 13 and 15 through 18 and the final digest were
not reached. Issue 14, canonical world state, and all wave checkpoints remain
unchanged.

Pushed `fb8f467` adds an explicit bounded newspaper recovery start wave. Its
local gate passed 13/13 strategic-smoke binary tests, 468/468 library tests, and
`git diff --check`.

Exact-source run 43 at
`fb8f4670679446f7f825bf733a4b8b9ea524d2b7` resumed run 41 at revision 27
with recovery start wave 13 and did not replay or mutate world mechanics. Issue
13 recomposition succeeded and Issue 14 remained preserved. Issue 15 failed
presentation-only because the grounding reconciliation agent retyped an
`expected_phrase` outside the exact frozen copy-desk finding set. Issues 16
through 18 and the final digest were not reached; canonical world state and all
wave checkpoints remain unchanged.

The copy desk owns the exact frozen finding set. Pushed `35f56f0` makes
`GroundingFindingRef` page-local and uses one shared deterministic resolver
to derive article, field, paragraph, phrase, and span for verdict validation and
edit application. Agent actions contain only `finding_ref` plus replacement or
`delete_finding_refs`; there is no compatibility path for retyped phrases. The
registry fixture is updated. Gates pass 14/14 newspaper tests, 468/468 library
tests, 13/13 strategic-smoke binary tests, and `git diff --check`.
Plan commit `4916638` separately admits persistent political/editorial
publication Personas. That future architecture is not a fact-check prompt
change and does not own the current acceptance repair. Earn one good Canopy
Ledger first; multiple publication Personas come later, and TeX/layout or
engraved-cut production begins only after editorial acceptance.

Exact-source run 44 at
`35f56f0e5e670025e1cfb031cba8d5a4ca9587d4` resumed revision 27 without
world or successful-publication replay. Issues 13 and 14 remained preserved.
Typed finding references were admitted and the first reconciliation action ran;
the copy desk returned a second exact finding set, then `MAX_STEPS=1` stopped
the agent before it could react. Issues 15 through 18 and the final digest remain
absent. The current uncommitted cut permits exactly two total corrective actions
in the same reconciliation session, preserves workbench state and receipts, and
gives the second action an explicit refreshed `finding_catalog`; it never reruns
the editor. Gates pass 15/15 newspaper tests, 469/469 library tests, 13/13
strategic-smoke binary tests, and `git diff --check`.

Exact-source run 45 at
`1d898489daeb34fc9f53e6c6bddf00738fc66364` resumed revision 27 without
world or successful-publication replay. Issues 13 and 14 remained preserved.
Action one was rejected locally before the copy desk. Action two was admitted
and rechecked, and the copy desk returned four fresh findings, but the
two-action ceiling left no action to answer them. Issues 15 through 18 and the
final digest remain absent. The bounded protocol needs three total actions: the
initial action, at most one local-admission correction, and at most one fresh
copy-desk correction. A rejection after action three is terminal; the same
workbench state and receipts persist, and the editor never reruns. The exact
mixed-path regression and full gates pass: 16/16 newspaper tests, 470/470
library tests, 13/13 strategic-smoke binary tests, and `git diff --check`.

Exact-source run 46 at
`dfe0926041b83d69a4b02451cdab706572522ae3` resumed revision 27 without
world or successful-publication replay. Issues 13 and 14 remained preserved.
Issue 15 exhausted three admitted repair/copy-desk rounds; the final exact
finding is one mechanical duplicated clause. Issues 15 through 18 and the final
digest remain absent. Unit invocation is
`3276392f4a7149fc9d127a966e792633`.

Pushed `db48befa71fcca25f1aae9976110ded5fad7cf2b` persists a typed
generation-zero checkpoint and a new typed checkpoint after every rejection,
binding the exact draft, verdict and finding catalog, model and deterministic
receipts, and source/contract identity. Accepted composition and strategic
artifact records separately bind issue and rendered-file ancestry. Resume
continues that exact pending chain without rerunning the editor. Typed run-46
import converts the preserved terminal evidence into the same checkpoint
contract.

Exact-source run 47 admitted Issue 15 and reached Issue 16 generation three.
Run 48 resumed Issue 16 to generation six. Run 49 resumed and admitted Issue
16, then advanced Issue 17 through generation 24 before its deliberately
bounded eight-call launcher stopped. The final five findings include one
paragraph-sized mechanical finding containing a smaller unsupported phrase.
The reconciliation agent repeatedly supplied both exact repairs, but the
workbench rejected the nested spans as overlap. Runs 46 through 49, world
revision 27, every strategic checkpoint, and accepted Issues 13 through 16
remain immutable. Live codex-connector and ghostlight-dungeon service PIDs and
invocation IDs remained unchanged.

Pushed `8b1db895578a7c800d42a3bc761666720f86456b` keeps this decision in
the grounding workbench: edits sort deterministically by start, outer span, and
finding reference; an outer replacement owns and covers contained repairs,
while partial overlaps still fail closed. Run 50 resumed the exact Issue 17
generation-24 checkpoint, admitted Issue 17 on its first repair, and advanced
Issue 18 to generation 24. Its two remaining findings both call for deletion of
a trailing sentence. The agent repeatedly supplied empty replacements, but raw
span deletion left trailing spaces and reader-copy validation rejected those
workbench-created malformed paragraphs. Runs 46 through 50, world revision 27,
every strategic checkpoint, and accepted Issues 13 through 17 remain immutable.
Live services remained unchanged; run 50 unit invocation is
`6fc5b829f48247419a00bf9b02b54d08`.

Pushed `ccc51df` makes the same workbench own its documented empty-edit
seam: a deletion consumes one adjacent ASCII space only when its phrase begins
or ends the field. The complete finding set remains mandatory, partial overlaps
still fail closed, and the same copy desk rechecks the transaction. Gates pass
four focused grounding-repair tests and 477/477 library tests.

Exact-source run 51 is mechanically complete at world revision 27 with accepted
Issues 13 through 18 and a final newspaper. The immutable local archive is
`F:\GameCult\GhostlightDungeon\acceptance\elven-realms-autonomous-ccc51df-51.tar`
with SHA-256
`FC7A930C73E6B22FDD11D137616B6A38B4EDA19AA837B25DA6DAEF7B7C0611F2`.
Its manifest binds final `newspaper.md` to SHA-256
`250131C321C5D90C48854FDBA032591B51B51AA1D5B7060BB9D419944625CA7D`.
Exact live codex-connector and ghostlight-dungeon service identities remained
unchanged.

Independent blind editorial review returns FAIL: the digest remains procedural
and lacks a sustained throughline, material lived cost, and countermoves.
Independent grounding review returns FAIL: the audit omits semantic source
descriptions, assertion status, and identity evidence, and the copy contains
status inflation. Mechanical completion is proven; editorial and grounding
acceptance are not.

The newsroom-only ownership cut is locally verified. A capable
narrative-selection agent now submits one typed agenda through the generic
agent harness; its deterministic workbench owns lead selection, story budget,
unique exact citations, bounded framing, and exact per-pitch grouping. The
editor may compose only from that admitted selection, while the existing copy
desk and grounding-reconciliation agent retain factual judgment and bounded
repair. Issue v3 persists the agenda and one semantic citation projection with
exact committed accounts, assertion status, supported identity attributes, and
canonical provenance; the duplicate flat audit fields are gone. Contract-bound
checkpoint v2 preserves agenda and narrative/editor/copy-desk ancestry so
reconciliation resume cannot replay either upstream agent. Gates pass 23/23
focused newspaper tests, 478/478 library tests, and 13/13 strategic-smoke tests.
No dependency, crate, binary, daemon, target, or world-simulation pass was
added.

Exact-source run 52 attempt 1 at
`5c07d49edb550bc0047f1b0bae422f186bd25ad6` stopped before any strategic
wave or final newsroom composition. The acceptance driver tried to reinterpret
run 51's immutable accepted per-wave recomposition receipts under the current
Issue v3 recovery contract. Run 51 and every world, wave, and publication
artifact remain unchanged. The run-52 root now durably contains `campaign.cc`
and its failure evidence. Historical accepted per-wave artifacts belong to the
byte and digest bound by their original immutable receipt; current schemas may
not reinterpret them.

Pushed `420034320c07e86c9c618c098f29d319ce113545` repaired that historical
boundary and the same run-52 root completed final-only composition at world
revision 27 without strategic replay. Exact live services remained unchanged.
Final newspaper SHA-256 is
`0070CAC0C173FFCF21B05BBE234EAD752234FF3335B12CC027453A48E26A8C45`;
archive SHA-256 is
`A800F8B4116A5AC7953400F96DC1E764357907D33BA54AD4D8A42E702CC1308C`.
Independent grounding review passes.

Independent blind editorial review fails. Three articles consume all 32 desk
citations, render the survival/legitimacy agenda as administrative minutes,
bury the severed hands, and lack sharp opposing motives, countermoves, and lived
cost.

Pushed commit `6a045bc3327860483f71abd5546b7b215e45a3e5` owns newsroom selection
contract v2 and that structural seam. Deterministic admission caps total
selected citations at `min(source_count, available_articles * 2)` and each
story at six citations.
Every dossier requires one `focus_citation`; `narrative_claim` replaces the
vague angle and travels with the exact admitted plan the editor receives. A
read-only serde alias/default exists solely so unrelated historical checkpoint
rows deserialize; it is not active compatibility authority. Gates pass 24/24
newspaper tests, 479/479 library tests, and 15/15 strategic-smoke tests. Run 52
remains immutable.

Exact-source run 53 at
`d42caca278c467573f0a4b810d562bd86d29a3f1` uses newsroom implementation
`6a045bc3327860483f71abd5546b7b215e45a3e5` and is mechanically complete at
world revision 27 with 18 waves, no world or wave replay, and exact live
services unchanged. Immutable root is
`/var/lib/gamecult/ghostlight-dungeon/acceptance/elven-realms-autonomous-d42caca-53`.
SHA-256: newspaper
`0358c4e82fa716f0364976aabe3ac79a1426596d9bd9eb4f8f4f61654bceb7e2`;
audit `19b9cfa5da0eb3d20614905b68338e071eeaeb076d803cb87832ba0c9592e34f`;
result `dbc1048e7f77db3b7fc7a77c31f03a7af562e90027ac5f0c8c47789266cb7a38`;
archive `F8940F1EEC46344E2D041022B22A52E36F52907AF88F1A7CB992E098BCF8B60D`.
Every manifest row passes and independent grounding review passes.

Independent blind editorial review fails. The paper leads with one
five-citation conduit/severed-hands dossier, then falls into cordons, reports,
and reviews without responsibility, opposing motive or countermove, scandal,
lived consequence, or a next-issue hook.

Source inspection of run 53's exact implementation overturns post-compose
self-review as the current hypothesis. That source caps dossiers at 32, sorts
newest-first, and silently skips each later unique source. Run 53's committed
result still contains the original public `strategic_pressure` complex: the
speaking child, murderer-of-river accusation, forest sale, severed delegation
hands and court seals, pump failures, legion refusal, and bridge seizure. That
material was outside the selector's desk. Post-copy review is not disproven
forever, but it was premature for run 53 because archive context was missing.

Run 54 is immutable mechanically complete evidence at exact source
`01581b8576774f370884e331558605e7ef5e1b9b`, implementation
`2ac74abf12612b2c38b04acd6fcb604acd1c9d28`, and world revision 27,
with 18/18 waves, 363 events, 853 news rows, and no world or successful
per-wave publication replay. The newspaper passed independent grounding and
failed blind editorial review because administrative conditions, filings,
cordons, and recordkeeping buried conflict and bodily evidence. Its archive is
`F:\GameCult\GhostlightDungeon\acceptance\elven-realms-autonomous-01581b8-54.tar`,
SHA-256
`835F72FCB19C38815C543DC1497977288F2F1442F22FDBD32D1B312490B34808`.
Every manifest row passes and live service identities remained unchanged.

Exact implementation commit
`c34c4b1f94706661cdc35d5314ea23347bda6fd7` owns the deliberative newsroom
cut. Canonical `NewsIssue` plus `Event` rows remain the sole public fact
ledger. The selector queries that ledger, proposes an agenda, receives a
deterministic comparison of its proposed lead with every below-fold focus
fact, and may query or revise again. Only an exact-current-digest commit admits
the reviewed agenda to the publication writer. The writer owns narrative
construction, emphasis, juxtaposition, and clearly signalled interpretation
over admitted facts; the independent copy desk alone owns factual judgment.
Direct proposal acceptance is removed. Candidate agendas, proofs, and digests
are ephemeral views and gain no fact, world, or persistence authority. Gates
pass 28/28 focused newspaper tests, including unreviewed and stale-digest
rejection, 483/483 library tests, and 15/15 strategic-smoke tests. Formatting
and diff checks pass. No dependency, crate, binary, daemon, persistent schema,
cache, event writer, service, or simulation pass was added.

Exact-source run 55 at
`c454d13ecd80d0720e072cc94460ccf5b98c843c` is mechanically complete at
world revision 27 with 18/18 waves, 363 events, and 853 news rows and no world
or successful per-wave publication replay. Its immutable artifact root is
`F:\GameCult\GhostlightDungeon\acceptance\elven-realms-autonomous-c454d13-55-reader\elven-realms-autonomous-c454d13-55`.
SHA-256: newspaper
`256a72523c034027fb03db193638ad36479de91be05e39141ba369c22eb87338`;
audit `28ca9ab82a8ee756c32e30f44e2e53886a45f5ee7d2d0fb5cc573c7f04672906`;
result `f04a4591a5a681cecf5f62520bcd8a5341642c58606acdbc86eea7a75be3dd6a`;
status `a7065af10b6d90fe93ffc6d4d66fd7bdabe297f864f99aeeddeb1124962695ed`.
The acceptance witness binds unchanged live-service identities and
`world_or_wave_replay=false`.

Independent grounding review passes. Independent blind editorial review
fails. The query-backed ledger gave the newsroom the relevant material, but
the paper led on the regulator audit rather than its strongest human conflict
and retained an audit-ledger rhythm. Ledger access is no longer the active
blocker; news judgment before agenda admission is.

Exact-source run 56 at
`c34c4b1f94706661cdc35d5314ea23347bda6fd7` is an immutable mechanically
complete newspaper-only run over the frozen revision-27 simulation. Both
independent reviews fail. The query-backed selector and deliberative lead
challenge worked and chose the consequential lead. Phrase-level grounding
reconciliation is now falsified as the repair owner: it can make isolated copy
safer, but it produces compliance prose and cannot express source-safe
journalistic juxtaposition across a whole article.

Pushed exact commit `0c1400985c061c0db44969288846eff79eeed69a`
owns contract `canopy-ledger-publication-rewrite-desk.v6`. Phrase-level repair
is replaced by a publication-owned full-article rewrite desk over the admitted
selection, exact canonical public records, rejected articles, and complete
copy-desk findings. The deterministic workbench freezes article selection, order, sections,
bylines, citations, and unaffected articles. The desk may reconstruct every
reader-facing field of each rejected article, after which it invokes the same
independent copy-desk agent as factual judge. Typed rewrite checkpoints
preserve the exact draft, verdict, selected records, and receipt chain for
resume without replaying the publication writer or simulation. Gates pass
28/28 focused newspaper tests, 483/483 library tests, and 15/15
strategic-smoke tests.

The audit discloses when an immutable source account reaches the shared
240-character public-event summary boundary: it shows the complete stored
account and asserts nothing beyond it. Reviewers must treat that as the owned
world-source boundary, not as newsroom truncation or omitted provenance.

Exact-source run 57 at
`0c1400985c061c0db44969288846eff79eeed69a`, using rewrite-desk contract v6,
is an immutable mechanically complete newspaper-only run over frozen world
revision 27 with 18 waves, 363 events, and 853 news rows. SHA-256: newspaper
`d90758f411193aa6e86ae2e8cb390650d9b8a58eac77ef87018bd61e9a668c57`;
audit `895268bc22aa3e4176b1da31388e85327f0fe11e4ea4ea4ee600b4e3270a8eed`;
archive `241354b669ee2357c9bb7a06e10e0b93378637621ff2b8b27ab7cf35c2e44e28`.
Mechanical grounding accepts.

Independent blind editorial review fails. The lead and other stories remain
administrative, lived stakes are abstract, no named accountable Greathold
opposition appears, severed hands are buried without court responses or
suspects, and the seed-vault story repeats procedure. The admitted editorial
agenda is materially sharper than the rendered copy. That was the Run 57
implementation diagnosis; it is preserved as provenance, not as a current
publication veto or next action.

Pushed exact commit `161ff2f19021249563e192cce9e6f76f00429d48` owns
provider-proven `canopy-ledger-night-editor.v8`. The copy desk emits one
assessment plus complete consequential query set. One single-step Night Editor
action must disposition every query and rewrite every queried article, may
polish unqueried articles, and freezes selection, order, sections, bylines,
citations, and agenda. One deterministic `WorldNewspaperPressClose` witness
binds source and printed pages, both model receipts, addressed queries, changed
articles, source bounds, and exact lineage; no model rereads or grades the
closed page. Gates pass 20/20 newsroom tests, 475/475 library tests, and 16/16
strategic-smoke tests. Durable publication-memory corrections remain
unimplemented.

Run 59 is printed at exact source
`161ff2f19021249563e192cce9e6f76f00429d48` from immutable Run 58 at world
revision 27 with no selection, writer, copy-desk, world, or wave replay. Its
acceptance witness binds one copy report with five queries, one Night Editor
close, `night_editor_action_applied=true`, three changed articles, and unchanged
live-service PID/invocation identities. SHA-256: newspaper
`ee2ce08ceb627d433ac2d4ff7e1ebecb1437bf28b1399d23bfb3d5b066c8ab27`;
audit `b210d50d4ef1a350315d528f6d37e1e7b618926346713f737c4169c49df26b49`;
immutable local archive
`F:\GameCult\GhostlightDungeon\acceptance\elven-realms-autonomous-161ff2f-59.tar`
`ecb4db41e9a40ee4db8b0f83dce42d74e40f940e5930b1d3c1ced227dfec76b3`.
A finalizer verifier incorrectly expected a schema field on the
assessment-plus-queries report; finalization resumed without another model
call. The edition satisfies the press and delivery boundary.

Post-press editorial diagnostics find strong child-seizure reporting and raw
scandal, but also institutional paste, thin lived/public reaction, repetition,
an HTML entity, and lowercase `sella`. Post-press grounding diagnostics identify
two later correction candidates only: line 47 over-scopes “maintained without
disturbing” from cordon and warnings to barriers and path; line 19 changes
historical stalled lifts into “remains stalled.” These findings do not reopen
the printed edition or block delivery.

The source-grounded prompt audit falsified v8 as the target newsroom
architecture. Run 59 narrative selection used ten provider calls, 1,373,208
input characters, 582,184 prompt tokens, and zero cached tokens.
Exact pushed commit `901cfcd58794acc35b24be3d1e149fd2df0f6c3d` owns
`character-newsroom.v9`, which uses typed append-only model items, stable selector
and Night action schemas, stable stage/model/schema cache routing, and no
prompt-embedded tool schemas or repeated prior actions. Veyra Kest sees the
complete embodied publication roster and assigns every story to a journalist whose character,
biases, and preferences inform that choice. Each reporter sees only their own
profile, assignment, and exact cited records. Dalen Marr alone raises one
complete factual checklist. Meret Sorn receives only queried articles, their
assignments and cited facts, and the affected reporter profiles; she gets one
chance to resolve exactly those objections while preserving the story. The
one-shot whole-page writer and generic byline registry are deleted.
Deterministic press admission still owns structure, provenance, query
disposition, and closure.

Run 60 attempts 1 and 2 stopped before any v9 provider call. Attempt 1 exposed
the missing `wave_count + 1` final-only sentinel. Attempt 2 reached final
composition without simulation or per-wave replay, then exposed automatic v7
close import as obsolete live authority. Pushed `c0a001c` deletes that authority,
makes advance return one composition, and leaves the exact-current close
checkpoint as the sole resume owner. Attempt 3 built that exact source and also
stopped before a provider call with an empty receipt list: the private
close-origin enum could not decode historical `legacy_v7_checkpoint`. The
current single-file fix admits that spelling solely as an inert private
tombstone, filters it before exact-current close selection, and proves a fresh
v9 issue runs despite a matching historical row. Gates pass 475/475 library
tests, 16/16 strategic-smoke tests, and all-target checks. Provider and
reverse-Turing acceptance remain unproven.

Run 60 attempt 4 at exact pushed `26d1d87` passed the historical tombstone and
reached live v9 provider work without world or per-wave replay. Ten Sol selector
actions assigned Mera Quill, Ossan Reed, and Lysa Fen; each reporter needed one
semantic correction. The first copy-desk transport attempt timed out at 300
seconds and the second completed. Deterministic validation then rejected two
distinct checklist objections because they named the same exact article phrase.
The current tiny fix deletes only that false uniqueness rule while preserving
exact single-occurrence target validation: distinct factual or copy objections
may target one phrase, and Night must address every query index once. Focused
newspaper tests pass 21/21 and strategic-smoke tests pass 16/16; full library
passes 476/476 and all-target checks pass.

Live cache evidence is mixed, not acceptance: all work used 208,480 prompt
tokens, with 79,616 hits and 128,864 misses. The selector alone used 189,548
tokens across ten calls, with 77,824 hits, 111,724 misses, and 714,366 serialized
input characters. Stable prefixes eventually hit, but selector context remains
too large. Run 60 attempt 5 then passed the same-phrase checklist boundary but
failed deterministic dateline validation on Mera Quill's article. No pre-copy
stage output was durable, so the attempt replayed the selector instead of
resuming the admitted agenda and accepted reporter work. No world or per-wave
work replayed, but no v9 front page or reverse-Turing result is accepted.

The live `character-newsroom.v10` cut owns that boundary. The generic agent
harness now lets a tool project one bounded current workbench snapshot while
preserving the stable instruction prefix. Veyra uses that seam for a compact
inspected-record index and bounded exact-ID fetch instead of accumulated full
tool chatter. Any query or fetch retires a pending agenda, so retrieval can
never leave an old proof eligible for delayed commit.

One exact task/editorial-bound production checkpoint persists the admitted
agenda before any reporter runs. Each accepted article then persists its own
checkpoint with article slot, assignment, complete embodied reporter profile,
and receipt chain. Deterministic cited-place enums own dateline admission. The
context membranes remain person-shaped and bounded: Veyra sees her own profile
and the journalist roster; each reporter sees only their profile, assignment,
and cited facts; Dalen owns the factual page desk; Meret sees only the queried
articles, affected assignments/reporters, cited facts, and one checklist.

The affected
`newspaper::tests::narrative_workbench_queries_foundational_context_by_stable_record_id`
rerun passes. A fresh full library run passes 479/479, strategic-smoke passes
16/16, and all-target checks pass with only the pre-existing native-client
unused `path` warning.

Exact pushed commit `139b8a6` completed Run 60 under
`character-newsroom.v10` without world or per-wave replay. Its acceptance
witness and terminal status bind the completed page and resume lineage. The
independently dispatched blind reviewer saw only the reader-facing fantasy
newspaper and read it as a real left-populist civic broadsheet; it did not
immediately identify procedural generation, so the narrative reverse-Turing
criterion passes. The review's criticism that all three stories feel like
three rewrites of one briefing belongs to reporter-personality differentiation
in the next newsroom run; it does not authorize rewriting accepted Run 60.

Run 60 used 96,902 prompt tokens and reported zero cache hits. The bounded
snapshot initialization had split the stable charter from the first-step
directive, so the first provider request did not form the exact prefix reused
by later steps. Exact pushed commit `8f3c33b` (`Preserve bounded agent cache
prefixes`) collapses those into one exact initial cacheable message while
preserving later bounded workbench replacement; all 479 library tests pass.
This cache-boundary fix does not require resimulation or recomposition of the
accepted edition.

Typesetting is delivered at
`output/pdf/canopy-ledger-run-60-front-page.pdf`. It is one 10-by-12.2-inch
three-column page with embedded Georgia/Times faces, warm press stock, and
wordless rail/gauge cuts. The frozen Markdown source hash remains unchanged.
Region-based extraction finds all 23 expected copy fragments, and full-page
144-DPI visual QA finds no clipping or overlap. Typesetting is complete and no
longer active work.

Run 54 exposed one separate upstream publicity defect: a public strategic
activity propagates its channel to private `MemberMemory` outcomes, which is
how Orin Pell retaining a memory became a public news row. The newsroom should
ignore that bookkeeping, but revision 27 remains immutable. Future kernel work
must require an outcome's own observable or communicated public scope before
materializing a `NewsIssue`.

1. Preserve immutable accepted Run 60 and Run 59; do not rewrite, recompose, or
   resimulate either edition.
2. Preserve the delivered typeset PDF and its frozen-copy verification above.
3. On the next newsroom run, strengthen reporter-personality differentiation
   so assigned journalists produce distinct framing, rhythm, and priorities;
   also capture whether exact pushed `8f3c33b` produces nonzero cache hits. Do
   not replay Run 60 to manufacture either improvement.
4. After that newsroom run, advance the Delvehold-owned adapter against
   Ghostlight's admitted generic consumer contract. Defer multiple publications
   until separately authorized.
5. Per-wave issues may use daily-like reporting windows, while a final
   composition may be an equally current weekly-style issue with a broader
   window. Both are selective journalism; broad fact availability increases
   editorial freedom and does not license recap voice.

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
