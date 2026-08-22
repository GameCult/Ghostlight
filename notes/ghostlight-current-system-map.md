# Ghostlight Current System Map

GhostlightDungeon is the active hosted runtime. Its authority map is
`docs/architecture/ghostlight-dungeon-mvp.md`. The implemented Rust daemon,
CultCache campaign stores, CultMesh/Eve surfaces, browser lowerer, native
Yggdrasil systemd body, Idunn continuity target, and Odin discovery crossing
are the live machine; the older validated artifact seams remain regression
evidence only.

## GhostlightDungeon target flow

Campaign creation now has its own pre-world authority:

```text
Heimdall member messages
  -> SessionZeroKernel durable shared/private conversation
  -> Projector -> persistent DM Persona owns exact speech
  -> Interpreter proposes typed draft changes, never rewritten speech
  -> runtime binds both into one revision-bound DM turn proposal
  -> typed contract, private character drafts, boundaries, decisions
  -> roster lock -> approved_campaign_brief.v1 -> Vault compiler
  -> evidence-gap conversation or digest-bound final review
  -> every player approves shared + own private digest
  -> one staging CultCache batch + atomic directory exposure
  -> campaign_membership.v1 binds each member to one exact actor
```

`WorldKernel` receives no draft state. The transcript is never replayed into the
compiler. Account and invitation hashes remain persistence-only; shared model
turns, browser surfaces, schemas, and CultMesh records cannot see them.

Session Zero extraction is one atomic membrane with separated authorship. The
Projector receives channel-permitted state and recent conversation; the Persona
receives only its lived narrative and owns the complete natural DM utterance;
the Interpreter receives a smaller typed extraction context and cannot emit a
speech field. Ghostlight binds the exact Persona output to the Interpreter's
typed proposals only after every stage validates against the same component
epoch. Interpreter failure therefore commits neither prose nor draft state.

Material negotiation has one typed path. Accept applies the exact proposal
stored in the decision. Counter clears every typed payload from that decision,
records the player's counter in its shared or private durable channel, removes
the Accept control, and leaves compilation blocked. The DM may replace it only
through a same-epoch `ApplyDmTurn` containing a fresh material decision; the
replacement and retirement of the pending counter commit atomically. Stale,
empty, malformed, or failed counter responses leave the counter pending and the
retired payload uncommittable.

An acceptance card is also a typed-state claim. New Interpreter decisions must
carry a non-empty contract, character, or extraordinary-permission payload;
payloadless questions remain speech. Legacy empty decisions project no Accept
control and the kernel rejects forged acceptance without changing revision.
This prevents polished DM prose such as “materialize the character” from
advancing a revision while leaving the character ledger blank.

The player-facing Eve surface projects the exact typed payload on every
acceptance card and the complete actor-entitled private character draft. The DM
summary is explanatory, never the accepted state. Other members receive only
their own private ledger and the deliberately public party projection.

```text
Vault evidence receipts + typed campaign snapshot
  -> permissioned projection
  -> Persona receives private lived narrative only
  -> Interpreter emits typed deltas and action proposals
  -> deterministic gates + expected revision
  -> one WorldCommand enters the campaign mailbox
  -> atomic CultCache commit + receipt
  -> parallel affected-participant appraisal
  -> CultMesh/Eve projection
```

Ghostlight owns the generalized projection organ. Epiphany and other consumers
own their canonical Persona state and consequence commits.

The current checkout admits one to eight campaign members into one shared
scene. `campaign_membership.v1` maps each authenticated account to one exact
actor; HTTP, Eve, assessment confirmation, and CultMesh publication derive from
that binding. Human actors are excluded from Persona and strategic control.
Sequential public actions are accepted; PvP and split-party movement are
rejected. Time, group travel, and pooled cell-budget changes require unanimous
revision-bound approval. Post-launch Contract Review reuses the Session Zero
kernel against an exact world revision and commits approved amendments in one
campaign batch without rewriting location, topology, knowledge, memory, or
history. The authority map is
`docs/architecture/ghostlight-dungeon-session-zero.md`; later multiplayer work
remains in `docs/architecture/ghostlight-dungeon-multiplayer-intention.md`.

Build provenance has one owner: `crates/ghostlight-dungeon/build.rs` binds the
exact source commit into the binary. Its inputs are either the release tool's
explicit clean-tree commit or Git HEAD; it watches the actual symbolic ref,
HEAD reflog, and packed refs so an ordinary commit invalidates Cargo's cached
value. Health derives its commit from that embedded value. Release manifests,
launch scripts, and CultCache deployment receipts are verifiers and records,
not alternate writers. Local runs and immutable releases use this same binding,
and the regression test compares it to the checkout's live `git rev-parse
HEAD`. A mismatch blocks activation rather than being repaired after launch.

Away-time agency follows a separate proposal/commit seam:

```text
exact campaign revision
  -> flash-model six-axis resolution demand
  -> connected agency graph + budget/pins/leases/detail debt
  -> cohesive or arena simulation-cell cover
  -> one private Projector/Persona/Interpreter membrane per cell
  -> model proposes exact constituent-attributed actions or explicit inaction
  -> runtime selects compatible attempts and content-addresses each activity
  -> one batched outcome resolver chooses bounded typed consequences
  -> runtime binds complete cell membership + world/resolution revisions
  -> WorldKernel validates cover, stage/outcome receipts, knowledge, scope, custody, topology, and bounds
  -> AdvanceStrategicTick through the campaign mailbox
  -> one atomic campaign/event/news/cover/appraisal commit
```

The model owns no tick mutation. A provider failure or invalid proposal leaves
the campaign revision and world time untouched. Background inference checks
live-turn pressure before launch and again before commit; return catch-up uses
the same command path with player-turn priority.

A live request also interrupts an in-flight scheduler wave. Dropping that wave
aborts its parallel cell tasks before they can launch later Persona stages, and
a shared/exclusive commit gate makes scheduler commit impossible while any live
request is active. Return catch-up is intentionally exempt because it is part of
the live request and must finish before the requested player action.

Resolution-demand focal IDs are salience hints, not partition commands. They
cannot create mandatory singleton cells or exceed the configured budget. Cell
Projectors receive decision-relevant situation state; cell Interpreters receive
exact permissions and the narrative products. Membership and revision bindings
are derived by the runtime, so a model is never asked to copy an invariant that
the planner already owns. Stable prompt prefixes are deliberately placed before
dynamic state, and provider receipts expose per-attempt token/cache usage plus
bounded local validation failures.

Arena projection preserves spatial partitions as well as identity partitions.
Each remote view carries its actual location; a relation grants potential
reach, not co-presence. Persona turns cannot stage direct sight, speech, or
response across locations unless the lived stream establishes a shared place
or communication channel. Interpreter activity labels then describe the
narrow attempted act, while hoped-for success remains outside the effect.
Ghostlight deterministically prepends the exact name/location partition to the
learned Projector's narrative, so model omission cannot erase those boundaries.
Only supplied cell constituents and selected member exceptions may own a
perspective. Event mentions do not transfer a person into another cell, and an
unnamed or unsupplied entity cannot become an activity target merely because a
Persona found it plausible.

Institution slices carry committed `current_posture` separately from unresolved
pressure. Projectors present the posture as existing state; Interpreters and
validators compare any new commitment directly against it. The old overloaded
`pressures[0]` convention is not an authority path.

Model-facing actions contain one owner, `subject_id`. Nested effect owner IDs
are runtime-derived during binding and exist only in the canonical downstream
proposal. The model cannot disagree with itself about `member:mira-venn` versus
raw `mira-venn`, and the prompt/output contract spends fewer tokens copying
invariants Ghostlight already owns.

Canonical constituent references have one owner:
`resolution::subject_state_references`. Scheduler slices consume that set
directly, including active reachable migration destinations, and wave
validation recomputes the same set. Prompt construction cannot append
references the resolver does not recognize.

Cell perspective attribution is runtime-bound too. The schema admits only
exact constituents and selected member exceptions. Ghostlight rejects
duplicates and requires both the debt-selected focus and every selected named
member exception to receive their own perspective segment; an actionable
individual can no longer disappear inside the aggregate before the Persona
turn. The perspective count may exceed the action limit only by the number of
mandatory owners; action consequences remain separately capped. Segments are
sorted and lowered to natural name/location headings before the Persona sees
prose only. An invented Markdown section cannot turn a mentioned outsider into
a decision-making perspective.

Gestalt background choices use three distinct typed paths. Pressure transitions
change exact unresolved markers. `gestalt_activity` records an attributed
preparation, coordination, investigation, recruitment, obstruction, trade, or
communication attempt against subjects connected by an explicit agency
relation or exact shared location without claiming the outcome. After
selection, one wave-level Flash resolver receives only those attempts and
precomputed legal consequence handles. It must return one digest-bound
`strategic_activity_outcome.v1` per activity; missing, duplicate, stale,
player-mutating, or invented effects reject the entire wave. Accepted outcomes
may change exact resources, population pressures, incident agency relations,
named-member deltas, or discoverable canonical knowledge, or explicitly record
that no durable material change occurred. A selected
dormant member can be addressed by their durable ID; this does not union them
into the source population. The kernel derives the event text and exact
participant IDs; the arena and model prose own neither.

The outcome resolver does not repair or reinterpret Persona intent. The
Interpreter owns the attempt; the resolver owns opposition and result;
WorldKernel alone owns mutation. Its stage receipt binds the sorted set of
MessagePack proposal digests, and every outcome is stored independently in
CultCache and projected on the operator Eve surface. All effects apply to the
same private campaign copy as the rest of the strategic wave, so a late invalid
outcome cannot leave an earlier action, clock, detail debt, or event committed.

Deterministic admissible-effect handles constrain every resolver decision.
Ordinary bounded preparation, pressure, and knowledge effects stop at local
validation. Resource consumption/transfers, agency-relation shifts, and named
member private deltas additionally receive an independent same-snapshot
semantic verdict because those effects can be structurally valid while charging
the wrong inventory or person. The verifier proposes no state and cannot
commit. The latest provider-backed four-wave golden produced seven typed
material activity outcomes, ten durable consequences, and zero rejected pulses
while leaving the player unchanged and returning the same Mira.

The service-owned CultMesh store remains the canonical projection body when
Odin rendezvous is temporarily unavailable. RUDP publication is outbound
replication, not daemon-liveness authority: a failed publish is logged with its
key and target while the complete local snapshot persists and HTTP serves that
same typed health and Eve state.

Opening retrieval deliberately covers early, transitional, and late historical
frames. These pure retrieval/compiler functions now serve the Session Zero DM;
they are not browser-owned creation routes or transient preview maps. Generated
suggestions enter the typed draft for discussion and use the same final brief as
custom premises. Both rejected and accepted receipts remain private. Invite rotation is owned by
the deployment boundary: it replaces the protected token set and clears the
persisted auth authority together while the daemon is stopped. Campaign stores
survive; old sessions deliberately do not.

Away time does not require invented agency. When a campaign has no active,
simulation-eligible agency profiles, the scheduler enters the same
`AdvanceStrategicTick` kernel command with no model plan or resolution wave.
World time, clocks, pending-tick accounting, and deterministic obligations
advance atomically while the player remains untouched. The scheduler does not
call the partitioner merely to discover an empty graph, and it retains the same
live-turn commit exclusion as model-backed waves.

Compiler approval is a player decision surface, not an operator spoiler
surface. It exposes topology, public cast, institutions, populations, clocks,
player-role capabilities, evidence coverage, gaps, and branch assumptions. It
does not project branch-local or provisional fact statements—even when they are
discoverable later. Those facts remain canonical inputs to knowledge-gated play
and operator inspection; the browser cannot reveal them before discovery.

## Current acceptance body

Yggdrasil currently serves native immutable release
`257b6429c52c796167125bd81d923a605ac065df`, executable SHA-256
`dd50de988179818091ad53d957e53d86cb5b098911001d75fbb05252867be8d8`,
with Eve release `23eaf32eae76204357c1406b4a7d01bcece6b815`. The service runs as
`ghostlight:ghostlight` under `ghostlight-dungeon.service`; typed health,
manifest, embedded commit, executable hash, seven imported campaigns, one
Session Zero draft, OpenRouter `stealth/ox-alpha` readiness, and restart
recovery agree. Ghostlight stages request logical fast/capable classes; the
OpenRouter port maps both classes to the test model, uses low/medium reasoning by
class, excludes reasoning from responses, and receipts the resolved physical
provider and model.

The exact hosted gate passed 171 Linux package tests with zero daemon restarts.
The active adversarial journey repaired pending-counter authority, optional
opening/role suggestions, and pre-compiler draft completeness. Its local
Heimdall claim then expired during a private message; the visible Discord flow
is waiting for human reauthentication before live Mars/Hellas compilation
continues.

The public `/ghostlight/` path terminates on Yggdrasil nginx and proxies the
native loopback listener at `127.0.0.1:8831`. Anonymous access returns the
actor-free `ghostlight.play` Eve gate with `200`; actor and campaign state stay
unavailable without an app session and canonical membership. The previous
Starfire writer is stopped; its process and tunnel are no longer live
authority. The migration copied campaign and Session Zero stores while
preserving Yggdrasil's native mesh and provider-health identities.

Local CultMesh publication and HTTP readiness do not wait for Odin. Remote
replication is coalesced into one asynchronous RUDP batch. Odin's coordinator
is healthy on Yggdrasil and owns RUDP discovery at `10.77.0.1:17871`; Idunn
admits signed Ghostlight health and owns same-release continuity independently
of its deployment brake. VoidBot's canonical MCP service is local to the host
at `127.0.0.1:17875/mcp`, so live retrieval no longer depends on a Starfire
reverse tunnel.

### Deployment, discovery, and adjacent capacity

Idunn runs exact source `2a5cb3e08f5f5f40a12f825a9522b31e6af941af`.
Odin runs exact source `b4f9a2e95f0b41cebdeddc49223781d1d3c7b42a` with
CultLib `21163d83fa2670dbcc9cdd57c6c1776b33a91d62`. Idunn is the only
deployment and daemon-survival authority; Odin is the discovery/rendezvous
authority. Their
health or availability never grants either organ a campaign write path.
Idunn's runtime health clock preserves milliseconds, so same-second signed
health samples no longer oscillate into false "health vanished" transitions.

Release admission is based on the newest executable- or build-affecting commit
reachable from the admitted ref. Documentation, notes, state receipts, and
root Markdown are not executable release selectors. The root actuator proves
that the selected commit is an ancestor of the admitted ref before activation,
then verifies the exact installed witness. Documentation and state-only commits
therefore cannot displace the live executable.

Heimdall runs exact source `1086aee01169bf60e8a492b2740db1c6f3e8cabf`
with CultLib `5cefa0db0079a8e3ee22f29d7b9e6e5aa60912a9`. It publishes four
redacted typed discovery records to Odin under globally unique catalog keys:
the provider, private command boundary, Eve access plugin, and transport
profile. Ghostlight resolves `heimdall:command-boundary` from Odin only for
begin, complete, refresh, and logout; valid local app sessions do not depend on
an Odin round trip. No direct Heimdall endpoint remains in the Ghostlight unit.

Epiphany is adjacent capacity, not part of Ghostlight's campaign authority and
not required by the current provider-backed runtime. Epiphany source
`ebc0ffe4f341154d1902f9afe86f0a87f150179c` passed its locked tests and was
sealed as immutable package
`sha256-bb76728653b8e2e872b4da47f917abe4233fd6d4ae1fd573c5971c7db3922a5c`
with witness
`4d8350fac61f90d32a2b8067731308ec3e3672a42804db29c680b0fc68ab9adc`.
Idunn stopped the deployment before publication because the Bifrost operator
runtime identity/substrate and resident Self Codex credentials were absent.
Epiphany's deployment brake remains engaged, its units are inactive and
disabled, no `deployment.env` or signed runtime health was published, and
`/srv/epiphany/app/current` remains recovery release
`267a0257a4938d80d34b7807c66aa5f550b50f2c`. The next Epiphany attempt must be
one Idunn-owned transaction after those prerequisites exist; passive watcher
tasks and cross-task exclusivity messages are not an operational control
plane.

The deployed Session Zero canary survived two exact-build restarts with its
private store intact. Heimdall completed a real KLTST Discord round trip and
adopted one HttpOnly Ghostlight session. Aetheria retrieval produced three
grounded opening decisions. Live DM inference initially exposed that JSON mode
had not been given the Interpreter's application schema; the repaired release
places exact stable schemas before dynamic context, emits only new deltas, and
produced a coherent Mars/Zhestokost follow-up without borrowing First Exodus
state. A forced malformed-output path displayed a local retry notice while the
typed contract remained unchanged.

Authenticated acceptance against the exact release has proved generated
opening/role/approval compilation, a nonliteral canonical player actor ID,
private assessment, confirmed server roll, wait, daemon restart continuity,
fork isolation, original re-selection, reset, and a MessagePack-backed `.cc`
export. The multiresolution matrix covered 24 factions exactly once at budgets
1, 4, 8, and 32. The strict nested refugee golden placed all 24 subjects in one
rival arena, preserved Mira Venn as a named decision owner, migrated her into a
different depth-two population lineage, produced material activity across six
other background subjects, preserved her delta and the player, and later
promoted the same Mira for a return encounter. Witnesses live under
`F:\GameCult\GhostlightDungeon\acceptance\player-journey-a92fb7a-20260818`,
`gestalt-scale-a92fb7a-b{1,4,8,32}`, and
`gestalt-dynamics-7916ab0-strict`.

Cell budget and physical provider parallelism are independent controls. The
scale executable accepts `GHOSTLIGHT_SCALE_PROVIDER_PARALLELISM`, defaults it
to the runtime default of eight, and records it in every witness. The corrected
24-subject budget-8 / parallelism-8 wave completed in 18.35 seconds, covered all
subjects exactly once, committed ten material actions plus one attributed
inaction, did not move the player, and required no model retry. Its witness is
`F:\GameCult\GhostlightDungeon\acceptance\gestalt-scale-shakedown-b8-p8`.
The earlier 25.64-second budget-8 witness used four provider slots and must not
be cited as an eight-concurrency performance result.

`gestalt_migration` records a collective decision by one exact active leaf to
move along an explicit migration relation and reachable route. WorldKernel
changes the leaf's home location and agency-profile location together. The
destination population remains a separate canonical subject, and named member
deltas are not carried by implication. Approval-gated fission creates
destination/stayer cohorts; the same migration primitive moves any resulting
leaf regardless of lineage depth. A local `investigate` activity may omit a
target when it examines the exact current environment. A local `communicate`
may likewise omit a target when its exact source speaks, offers, requests
permission, or notifies an unnamed ordinary role. Both admit ordinary texture
without inventing a listener, response, acceptance, discovered fact, or target
agency.

Activity effects operate at canonical-location granularity. Incidental motion
around unnamed local features remains part of the source-attributed attempt
and does not create topology or assert arrival. A different supplied canonical
location or population destination still requires the exact movement path.

Salient dormant members have their own `member_activity` path for ordinary
local attempts. It uses the same bounded verbs, the member's exact location,
their current leaf plus explicitly related or exactly co-located targets, and
their own state/channel permissions. Only the capped salient member exceptions
selected for the cell enter any prompt or can be named there. Migration and
activity conflict on one member key. A destination population cannot inherit
the person's offer, speech, or decision merely because the cover placed them
in the same arena.

A named member's explicit commitment to board, depart, travel, or join a
supplied destination maps to `member_migration`, even when the first narrated
motion is only entering a queue. `member_activity: prepare` is valid only while
departure remains unchosen. The Interpreter and semantic verifier share this
distinction.

The semantic verifier emits one ordered verdict per candidate action.
Ghostlight owns exact verdict coverage and derives the aggregate rejected set;
the model cannot reject one arena constituent merely because another
constituent's mapping failed, omit a candidate, duplicate an index, or publish
a global decision that contradicts its per-action rationale. Rejected actions
receive one same-snapshot Interpreter correction and may not be returned
unchanged.

Population scale uses reversible individuation:

```text
gestalt baseline + existing member delta, or a first-relevance identity proposal
  -> WorldKernel validates gestalt/member/revision/location
  -> atomic durable member delta + temporary ActorState
  -> ordinary Persona appraisal and world commands
  -> expired relevance lease outside player perception
  -> atomic fold into the member delta; active slot removed
  -> gestalt receives strategic ticks without erasing the individual
```

The temporary actor slot is derived. It never owns identity, relationship,
memory, equipment, injury, or obligation state.

Automatic presence planning projects only active leaves at the player's
location, dormant members whose exact location matches, and materialized
members eligible for folding. The kernel rejects inactive hierarchy nodes and
location mismatches. Nested fission and migration therefore change which
baseline a durable delta composes with, never who the person is.
The projection also names the player actor's reciprocal relationship to each
nearby dormant member. The planner owns reversible scene casting and may surface
an earned callback without a player prompt; it cannot move a remote person or
commit the cast. Failed live casts retain a private presence-preflight artifact.

Institution `already_committed_posture` is projected as durable state, distinct
from unresolved pressure and fresh choice. Maintaining it is inaction. A model
may propose only a materially different posture; repeated posture remains a
locally rejected no-op and cannot enter an atomic wave.

Cell appraisals carry exact `actions[]` and exact `inactions[]`. Inactions name
the constituent or selected dormant member who holds and a bounded reason. They
cannot duplicate, cross cell authority, or coexist with an action from the same
subject. They are non-mutating audit/fairness evidence. The former global
inaction string is no longer an owner or runtime path.

Strategic plan application reads authority and permissions from one immutable
command snapshot. All writes land in a private working campaign which replaces
the command-local campaign only after every action validates. Sequential loop
order is no longer a fictional owner: a migration earlier in the apply loop
cannot invalidate another subject's same-snapshot communication, and a late
invalid action cannot leave earlier in-memory mutations behind.

The active cell budget, operator-owned pins, and provider concurrency limit are
separate controls. Budget and pins increment `resolution_epoch` without
advancing world revision or fictional time. Provider concurrency increments only
`provider_configuration_epoch`; it batches the same cover and cannot repartition
the world. See `docs/architecture/ghostlight-multiresolution-agency.md`.

The public application boundary is one stable Eve provider surface:
`GET /api/eve/provider`, `GET /api/eve/surfaces/ghostlight.play`,
`POST /api/eve/commands`, and revision-only `GET /api/eve/events`. Anonymous
surface requests return an Eve Heimdall access gate without campaign,
membership, actor, account, boundary, or Session Zero state. Authenticated
surface requests derive account, selected campaign, membership, and exact actor
from Ghostlight-owned app-session and campaign records before projection.

Command ingress accepts only `gamecult.eve.command_invocation.v1`. It validates
provider, logical surface, command boundary, operation schema, source version,
and idempotency before resolving the caller's authority server-side. Browser
payloads containing account, member, or actor authority fields are rejected.
Every admitted operation then uses the existing `SessionZeroKernel` or
`WorldKernel` mailbox and returns a persisted Eve command receipt. Strategic
ticks, region commits, NPC initiative, and raw kernel commands remain outside
the browser capability surface.

The browser is `EveBrowserProviderHost`, Ghostlight transport, Heimdall access
adapter, status mount, and SSE invalidation. Eve owns editable bindings and
operation capture: renderer-local drafts survive authoritative refreshes and
stale refusals, and only an accepted result may request named draft clearance.
HTML forms may appear as an accessibility lowering detail but are absent from
Eve documents and command payload semantics. Session Zero, campaign,
governance, receipt, and login DOM renderers are not product authorities.

Heimdall owns OAuth attempts, Discord callbacks, entitlement evaluation, claim
issuance, and single-use completion. Anonymous Eve operations cross Heimdall's
authenticated CultNet operation boundary on Yggdrasil loopback. Sensitive
completion and refresh fields are message-wrapped and never enter the public
CultMesh catalog, Odin records, Eve surfaces, logs, or browser responses.
Ghostlight validates the exact issuer, audience, app, profile, access revision,
session identity, expiry, and `app_access` before creating a local app session.

`ghostlight.app_session.v1` stores a hash of the random cookie, stable Heimdall
subject hash, exact Heimdall session and access revision, verified capabilities,
expiries, and a locally wrapped refresh claim. Routine requests verify that
state locally. Claim refresh and logout revalidate exact Heimdall session
custody through the private boundary; a stale refresh cannot resurrect a
logged-out session. Heimdall outages leave valid local sessions usable only
until their verified expiry. Campaign access always comes from
`campaign_membership.v1`; account preferences remember only selection. The old
auth store is frozen rollback evidence and does not participate in admission.

That boundary also projects less state outward than the kernel returns inward.
Player HTTP responses contain only assessment, public commit/roll receipts, and
narration. Canonical campaign state and spoiler-bearing actor or institution
state are operator-only. Informational rolls add only their exact previewed
finding to the acting character and a provisional branch fact. The assessor
deterministically binds typed findings into visible stakes before validation,
so formatting is not delegated to a correction attempt. The compiler
classifies each retrieved source as direct seed, setting background, or excluded
before generation. Only direct-seed source text enters causal world compilation;
background and excluded sources remain coverage provenance and cannot donate
story incidents or cast.

The actor-filtered Eve surface follows the same membrane. It renders the player-owned
ledger—including that actor's own relationships—and already filtered news, but
does not enumerate canonical institution postures or raw world clocks. Those
remain operator state until narration or an exact information channel makes a
development available to the character. A negative regression plants a remote
coup posture and sealed investigation clock, proves all three secret strings
absent from the player surface, and proves an admitted public headline remains.
It likewise exposes only the number of operator pins and local active
population leaves eligible for a fission preview. Pin bodies, remote population
names, and `ReplaceResolutionPins` remain outside the player boundary.

Exact private per-member and operator projections remain CultMesh documents for
authorized Verse consumers. The public provider advertisement exposes only the
subject-scoped logical surface and Heimdall plugin requirement. Hermodr may
materialize that provider for inspection when its plugin is installed; it is
not a Ghostlight runtime dependency and owns no application state.

Global strategic context has a separate non-causal lane. Two stable broad Vault
queries run alongside local retrieval, and a Flash extraction stage proposes a
bounded pool of remote institution names with one to three exact supporting
claims each. Local code binds every retained claim to an institution-specific
witness. Only the grounded candidates, capped at 32, enter a separate Flash
synthesis stage. It writes concise strategic doctrine from those claims; an
independent verifier rejects unsupported doctrine.
Unsupported entries become summarized approval gaps and private receipt detail;
they do not become canonical institutions or canon-candidate records. Admitted
remote institutions receive deterministic coarse profiles with distinct
authority and explicit unknown facets. The Pro agency stage profiles only local
actors, populations, and institutions where semantic subdivision is useful.
The model schema permits a bounded pool of 64 candidates so provider overage
and ungrounded index fragments can reach local judgment. Exact grounding runs
before the 32-institution simulation capacity is applied; excess grounded
candidates become an approval-visible on-demand-compilation gap. Raw proposal
count therefore cannot consume canonical capacity or abort an otherwise valid
world compile. A supporting claim is institution-specific only when its quotation names
exactly that candidate or comes from the candidate's dedicated source document;
shared index headings, category descriptions, cross-faction lists, and orphaned
sentence fragments are rejected. Exact claims and receipts remain evidence;
synthesized strategic doctrine alone becomes coarse simulation goals.

The Aetheria Vault adapter derives document authority from the exact source
path before any model sees a witness. `Worldbuilding` is reusable canon;
published `Fiction` and legacy `Stories` are historical reference; static
interactive output is a fixture artifact; `Brainstorming` is working draft;
and `Game Design` is design reference. Only reusable canon may enter a new
branch's `direct_seed` lane or the global-agency compiler. The classifier may
further narrow canon by relevance, but it cannot promote a lower-authority
document. Direct-seed projection and receipt selection repeat this local guard,
so a malformed or stale classifier output cannot smuggle a story cast into the
world seed. The live AetheriaLore retrieval archive is correspondingly bounded
to `Aetheria/Worldbuilding/`; durable facts discovered through fiction must be
promoted into Worldbuilding before they become reusable campaign canon.

Session Zero compilation, expansion, and fission follow the same projection
rule. Review projections expose topology, pressures, source-use coverage, gaps,
assumptions, public party cards, and only the current viewer's private ledger.
Generated openings are optional shared typed decisions; accepting one amends
the draft for discussion and triggers three optional grounded role decisions in
each private DM channel. Ignoring them cannot block a custom start or character.
Custom starts and characters use the same draft. The browser returns
only decision commands and cannot echo a rewritten candidate into compilation.
Candidate IDs, text, list sizes, and evidence references receive local
validation. Suggestion retrieval and model receipts remain in the Session Zero
store; publication compiles the unanimously approved brief, never the
conversation transcript.

Roster lock is not readiness. Before world compilation the kernel checks the
typed contract's core play frame and each active character's minimum actionable
shape. Transcript-only detail, blank contract fields, or a character with no
public premise, capability, goal, and stake return explicit missing inputs and
cannot spend a world-compiler call.

Campaign publication has a separate discoverability boundary. Session Zero
publication and solo lifecycle operations initialize a new `.cc` store under a
non-UUID `.creating-*` directory. The CreateCampaign command validates the
seed, then atomically commits campaign state, approved seed, membership,
contract, DM Persona, approvals, Vault manifest, evidence, model receipts,
canon candidates, and the creation lifecycle receipt in one CultCache batch.
Only that complete store is renamed into the UUID
namespace and admitted to the live registry. Failed initialization removes its
exact staging directory and cannot poison daemon reload. Approval previews are
retained until campaign publication, session selection, and reload all succeed;
a retry may recover only an already-published store whose exact seed equals the
preview. It cannot adopt a colliding campaign ID with different state.
The browser composer preserves the assessment contract's distinction between
the character's described act or speech and the effect the player wants it to
cause. The receipt panel is the sole owner of the current assessment,
destination, or fission approval control; rendering a newer receipt removes the
older snapshot-bound control instead of accumulating stale confirmations in
persistent forms. World-seed approval renders expandable public sections for
the player role, initial topology, present cast, institutions, populations,
world clocks, and exact evidence-use rationales instead of asking for approval
from summary counts. A browser-local interaction lock prevents duplicate paid
requests while preserving the kernel mailbox and revision checks as the only
world-mutation authority.
The player surface also projects the canonical current location ID. The
destination compiler binds that value into a read-only origin field and shows
the proposed locations, containment, routes, persistent features, and material
gaps before approval. Population fission approval similarly shows each named
child leaf, partition value (including `other/unknown`), and home location.
Fission admission runs before parent lookup, retrieval planning, or inference:
one request may contain at most 16 distinct cuts of at most 160 readable
characters and one 500-character reason. The browser mirrors these bounds.
The browser interaction lock covers a canonical mutation and the subsequent
surface refresh as one user interaction. Campaign selection, world approval,
destination approval, fission approval, fork, and reset cannot re-enable stale
controls between the commit response and the refreshed revision. Compiler
preview requests remain non-mutating and release the lock with their returned
preview. Received non-success HTTP responses are rendered as server refusals;
only missing or undecodable responses receive the ambiguous-transport warning
that a commit may have preceded the lost response. The production web build
runs strict TypeScript checking before Vite emits assets.

Relationship documents in the schema catalog are revision-bound projections
of actor-owned relationship maps; they are not a second relationship writer.
Vault manifests summarize the exact provider/source/authority/temporal lanes
covered by evidence receipts and do not own Vault content. Strategic tick and
gestalt materialization receipts are different: they are atomic commit
companions binding the generic world commit to the causal model output or
baseline/member presence transition.

## Control Flow

1. Rehydrate from `state/map.yaml`, `notes/fresh-workspace-handoff.md`, this
   file, and `notes/ghostlight-implementation-plan.md`.
2. Run `npm run state:status`.
3. Work one bounded organ.
4. Validate the seam that matters.
5. Persist only belief-changing evidence.
6. Commit and push completed work.

Model acceptance also measures useful work per token. Stable contracts precede
revision-bound context for cache reuse; each stage receives only the state it
can legitimately judge; provider receipts expose cache, prompt, completion, and
validation cost per attempt.

## Core Surfaces

- `state/map.yaml`: canonical mission, boundaries, live architecture, next action.
- `state/evidence.jsonl`: distilled belief-changing evidence only.
- `state/evidence.archive.jsonl`: older evidence preserved for archaeology.
- `state/corpus-coverage.json`: accepted/planned fixture coverage ledger.
- `notes/fresh-workspace-handoff.md`: compact re-entry packet.
- `notes/ghostlight-implementation-plan.md`: near-term implementation sequence.
- `docs/architecture/`: detailed contracts and rationale.

## Active Artifact Pipeline

```text
Aetheria lore/source context
  -> lore grounding digest / source notes
  -> canonical agent and scene state
  -> projected local context
  -> responder packet
  -> sandboxed responder output
  -> review and leakage audit
  -> observable event record
  -> participant-local appraisal receipts
  -> mutation receipt
  -> updated scene/world/social state
  -> initiative schedule updates readiness, reaction windows, and next actor
  -> sandboxed coordinator continuity and next-beat plan
  -> meta-coordinator review, wiring, and labeled interventions
  -> branch compiler materializes Ink + sidecar + visual plan + compiler notes
  -> IF artifact reviewer audits consequence, fold, and visual continuity
  -> narrative/lore/spatial/visual reviewers audit player-facing quality
  -> accepted fixture / future training corpus + optional illustrated replay collateral
```

Responder packets are one seam, not the whole project. The branching scene path
also requires coordinator receipts, branch compiler artifacts, and independent
review before a fixture is accepted.

Tone is a live seam. The system should preserve Aetheria's range instead of
defaulting to dry technical crisis prose: comic warmth, domestic routine,
ritual memory, weird bureaucracy, noir suspicion, wonder, horror, poetic dread,
and procedural systems pressure are all valid when source-grounded and
character-local. Adams/Pratchett-style wit-with-stakes is a useful default
touchstone for counterbalancing bleakness without dissolving consequence.

Visual replay is a presentation seam, not a core social-model training seam.
Illustrated IF fixtures need click-through sections with stable
`visual_scene_id` anchors, stable visual character refs for recurring named
characters, a global style cue when the visual language matters, imagegen-ready
base prompts, character visibility and stance controls, and branch/state
modifiers. This data belongs in a separate `.visual.json` artifact referenced
by the training sidecar, not inside the training annotation itself.

Generated images are not geometry authority. They can establish visual mood,
materials, and concept direction, but multi-angle illustrated IF needs a durable
`scene_set` source such as a hand-built 3D blockout, procedural scene file, or
layout asset with named cameras and staging slots. For those scenes, imagegen is
the painter/render pass over a camera-specific blockout, not the architect of
the room.

## Important Contracts

- Agent state: `schemas/agent-state.schema.json`
- Projection examples: `schemas/projection-example.schema.json`
- Lore grounding: `schemas/lore-grounding-digest.schema.json`
- Projected local context: `schemas/projected-local-context.schema.json`
- Coordinator artifacts: `schemas/coordinator-artifact.schema.json`
- Responder packets: `schemas/responder-packet.schema.json`
- Responder outputs: `schemas/responder-output.schema.json`
- Event records: `schemas/event-record.schema.json`
- Participant appraisals: `schemas/participant-appraisal.schema.json`
- Reviewed mutations: `schemas/reviewed-mutation.schema.json`
- Scene-loop bundles: `schemas/scene-loop-bundle.schema.json`
- Initiative schedules: `schemas/initiative-schedule.schema.json`
- Ink branch contract: `docs/architecture/ink-branching-scenes.md`
- Illustrated IF visual pipeline: `docs/architecture/illustrated-if-visual-pipeline.md`
- Training stages and corpus gates: `docs/architecture/training-plan.md`
- Corpus coverage ledger: `docs/architecture/corpus-coverage-ledger.md`

## Current Live Examples

Pallas Species Strikes is the active reference-only story fixture:

- `examples/lore-grounding/pallas-species-strikes.awakened-labor.v0.json`
- `examples/agent-state.pallas-species-strikes.v0.json`
- `examples/coordinator/pallas-species-strikes.branch-and-fold.v0.json`
- `examples/ink/pallas-species-strikes.branch-and-fold.v0.ink`
- `examples/ink/pallas-species-strikes.branch-and-fold.v0.training.json`
- `examples/visual/pallas-species-strikes.branch-and-fold.v0.visual.json`
- `experiments/pallas-species-strikes/pallas-species-strikes.branch-and-fold-clean-run.v0.md`
- `experiments/pallas-species-strikes/pallas-species-strikes.visual-scene-review.v1.json`

The fixture is not training-ready data for Ghostlight's model stages. It was
coordinator-authored without the exact projected-context, sandboxed responder,
appraisal, mutation, relationship-update, and true branch-compiler receipts
those stages require. Keep it as a reference for story shape, grounding,
branch-and-fold presentation, visual planning, and future fixture expectations.

`pallas-training-loop-v0` is the first separate training-shaped derivative. It
rebuilds only the Kappa threshold beat through actual receipts:

- scene digest and initial state under `examples/training-loops/pallas-training-loop-v0/`
- projected contexts under `examples/projected-contexts/`
- responder packets under `examples/responder-packets/`
- raw no-fork subagent captures under `experiments/responder-packets/`
- event, appraisal, mutation, review, and bundle receipts under `examples/training-loops/pallas-training-loop-v0/`
- clean readable transcript under `experiments/pallas-training-loop-v0/`

It is training-shaped for projector, responder, event resolver, participant
appraiser, mutator, relationship/perception updater, and coordinator/story
runtime. It is not branch compiler, IF reviewer, visual, or accepted full-fixture
coverage data.

`lucent-hostage-feed-v0` is the current open-ended training-loop draft. It
tests the same receipt-chain organs in a less reference-bound scene: a Lucent
Media hostage-feed negotiation inside the media-eye counterweight of a tethered
station staring down at a metropolis bubble.

The Lucent bundle includes:

- scene digest, initial state, and branch surface under `examples/training-loops/lucent-hostage-feed-v0/`
- projected contexts under `examples/projected-contexts/`
- responder packets under `examples/responder-packets/`
- raw no-fork subagent captures under `experiments/responder-packets/`
- event, appraisal, mutation, review, and bundle receipts under `examples/training-loops/lucent-hostage-feed-v0/`
- clean readable transcript under `experiments/lucent-hostage-feed-v0/`
- fixture materializer at `scripts/materialize_lucent_loop.py`
- initiative schedule example at
  `examples/initiative/lucent-hostage-feed-v0.turn-20.initiative.json`

Only the fifteen post-restart turns from `turn-06` through `turn-20` are counted
as training-clean responder receipts. The earlier exploratory opening turns are
setup/rehearsal context. The loop demonstrates state carryover: feed evidence
externalization creates breathing room, breathing room creates a bench-edge
substitution, the substitution enables a process-note admission, the admission
enables direct hostage leverage to end, and release folds into safe-line
confirmation plus non-contact protective handoff.

The current coordinator lesson is explicit: nudge through game-ergonomic state
levers rather than forcing plot. Safe lines, evidence locks, security holds,
handling categories, posture constraints, route access, clocks, public proof
objects, and resource pressure are the elastic strings. Actors pull against
those strings from local state; the coordinator records observed action and
folds consequences through reviewed mutation.

The coordinator must now be treated as a sandboxable organ in future
training-shaped loops. Codex remains the meta-coordinator: it prepares visible
input, launches the coordinator worker, reviews the output, labels repairs, and
wires structured state between organs. Raw coordinator training data should come
from sandboxed coordinator prompts, not from omniscient chat steering.

The initiative scheduler is now an explicit mechanical seam. It decides when an
actor is eligible to act from initiative speed, recovery, load, status, and
reaction windows; it does not decide what the actor wants or how other
participants appraise the event. Every affected participant still appraises and
mutates before the scheduler selects the next projected actor.

VoidBot's newer heartbeat routine is the reference shape for turning that seam
into a living swarm routine. For Ghostlight, the swarm is the cast: the same
heartbeat initiative law selects characters taking turns within a scene, while
offstage or maintenance heartbeats handle rumination, sleep consolidation,
memory resonance, and reviewed upkeep. See
`docs/architecture/voidbot-routine-adoption-plan.md`.

## Pruned Receipts

Old prototype fixture receipts and model-runner files have been removed
from the live workspace. Their useful lessons belong in the architecture docs,
state map, evidence ledger, and git history. Do not reintroduce deleted receipt
paths as active state surfaces.

## Missing Or Incomplete Organs

- Contract-governed split parties, private in-play actions, delegation,
  simultaneous declaration windows, PvP, late joining, permanent departure,
  and group fork/reset/export
- Deterministic speaker-local input slicer
- Full prompt renderer
- Classifier/appraiser pipeline
- Relationship/perception updater
- State mutator model
- Student projector
- Runtime integration for initiative scheduling beyond validated artifacts
- Branch compiler implementation beyond coordinator-authored fixtures
- IF artifact reviewer implementation beyond manual/frontier review
- Visual scene continuity reviewer implementation beyond prompted specialist review
- Full scene/event loop implementation
- Culture prior engine
- Automatic promotion of branch outcomes into canonical state
- Corpus coverage ledger expansion beyond the current draft Pallas and Lucent
  receipt-chain rows

## Current North Star

Generate high-quality, sandboxed, source-grounded branching-scene samples that
can later train specialized Ghostlight models while staying usable by a game
engine: exact inputs, exact outputs, provenance, review labels, state mutation
receipts, branch compiler artifacts, IF review findings, visual scene plans, and
clear branch consequences. The samples should also train tonal control: a
fixture needs an intentional prose mode and enough ordinary life for consequence
to matter.

Use `npm run coverage:status` before choosing new fixtures so the corpus grows
by coverage need instead of proximity to the nearest interesting disaster.
