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

Session Zero extraction is one atomic membrane with separated authorship. For
an ordinary turn, Ghostlight binds one exact current player message separately
from six prior same-channel turns and four shared continuity turns. The
Projector receives that bounded channel-permitted state; the Persona receives
the current turn verbatim plus its lived narrative and owns the complete
natural DM utterance. The Interpreter receives the same current turn, only the
typed state for its channel's authority lane, and no speech field. A private
Interpreter does not receive the shared contract merely to be forbidden from
editing it. Ghostlight binds the exact Persona output to the Interpreter's
typed proposals only after every stage validates against the same component
epoch. Interpreter failure therefore commits neither prose nor draft state.

Material negotiation has one typed path. Accept applies the exact proposal
stored in the decision. Counter preserves that proposal as an inert, visible
audit and replacement basis, records the player's counter in its shared or
private durable channel, removes the Accept control, and leaves compilation
blocked. The DM may replace it only
through a same-epoch `ApplyDmTurn` containing a fresh material decision; the
replacement and retirement of the pending counter commit atomically. Stale,
empty, malformed, or failed counter responses leave the counter pending and the
retired payload uncommittable.
Retry is an inference launch against that persisted counter and exact unchanged
snapshot. It owns no state transition; a replacement still commits only through
`ApplyDmTurn` at the original component and channel epochs.
For a counter replacement, the target decision ID enters the stage binding. A
deterministic Ghostlight Projector turns only the target, retired typed payload,
exact counter, and aggregate safety policy into the Persona's exact lived
stream; it does not spend a model call asking another agent to paraphrase facts
Ghostlight already owns. The Interpreter receives the same bounded typed basis.
For an exact typed counter lane it emits only the replacement payload. Fresh
decision identity, owner, materiality, evidence, prompt, and counter text are
bound locally; permission identity, actor binding, and evidence cannot be
rewritten by the model. The full decision-union Interpreter remains for
ordinary turns and legacy payloadless migration only.
Conversation history, party, contract, and unrelated private state are omitted.
Legacy counters created by the payload-erasing build receive one bounded
current-state basis. The kernel accepts exactly one same-lane decision with
stable permission identity and no direct patch, so unrelated output cannot
retire the counter.

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

Solo `Wait` has two exact meanings. A wait shorter than the campaign horizon is
a simple fictional-time command. A wait equal to one horizon enters the same
`AdvanceStrategicTick` compiler, Persona-cell, outcome, and atomic commit path
as away-time simulation, with a one-horizon maximum per player command. The
bounded co-op time proposal still advances raw time after unanimous approval;
routing governed co-op horizons through strategic simulation remains an open
implementation gate. Group travel updates controlled actor locations and their
canonical agency-profile locations in the same commit, so reload never becomes
a repair owner for partition inputs.

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
  -> independent effect verifier checks the typed action against the Persona choice
  -> runtime selects compatible attempts and content-addresses each activity
  -> one batched outcome resolver chooses bounded typed consequences
  -> selective outcome verifier checks high-risk custody and private-state effects
  -> runtime binds complete cell membership + world/resolution revisions
  -> WorldKernel validates cover, stage/outcome receipts, knowledge, scope, custody, topology, and bounds
  -> AdvanceStrategicTick through the campaign mailbox
  -> one atomic campaign/event/news/cover/appraisal commit
```

The model owns no tick mutation. A provider failure or invalid proposal leaves
the campaign revision and world time untouched. Background inference checks
live-turn pressure before launch and again before commit; return catch-up uses
the same command path with player-turn priority for fictional commands.

The effect and outcome verdict schemas make semantic coherence structural:
`match` carries no mismatch fields, while `mismatch` requires its exact kind or
repair text. Repair prose is ephemeral diagnostic input, not world state; local
code bounds it before correction and never publishes private cell choices to a
player error surface. A rejected wave returns one spoiler-free message while
its exact diagnostic remains operator-only.

On the one same-snapshot Interpreter correction, Ghostlight also removes each
exact subject/effect pair rejected by either per-action local validation or the
independent effect verifier from the output schema. Neither validator chooses
the replacement or gains commit authority; the correction constraint only
makes an already-known-invalid mapping impossible to repeat. The Interpreter
must use another permitted typed lane, omit the action, or fail the whole wave
without mutation. Cross-action failures do not guess at an offender and remain
ordinary semantic corrections.

The effect verifier receives the same exact typed permission slice used by the
Interpreter. That slice is its sole map of canonical locations, reachable actor
destinations, population destinations, and target IDs. A pump, desk, queue, or
other place named only in lived prose remains local texture inside the supplied
canonical activity location; the verifier cannot manufacture topology and then
reject a valid local action for omitting travel through it.

A live request also interrupts an in-flight scheduler wave. Live-turn admission
increments pressure and signals cancellation before waiting for an already
admitted background commit to finish. The admission guard exists before that
wait, so cancellation cannot leak false pressure. Dropping the scheduler wave
aborts its parallel cell tasks before they can launch later Persona stages, and
a shared/exclusive commit gate makes scheduler commit impossible while any live
request is active. Return catch-up is intentionally exempt because it is part of
the live request and must finish before the requested fictional action. Private
assessment and resolution-policy edits neither advance fiction nor invoke
catch-up. Session Zero logs the exact draft revision when a DM response is
queued and when its live guard is admitted, making provider latency distinct
from scheduler contention without exposing channel contents.

Entering or dropping that guard republishes the canonical typed health record
immediately. HTTP health reads the same record, so `live_turn_pressure` is an
actual launch brake during an in-flight player command rather than a delayed
status reconstruction.

Resolution-demand focal IDs are salience hints, not partition commands. They
cannot create mandatory singleton cells or exceed the configured budget. Cell
Projectors receive decision-relevant situation state; cell Interpreters receive
exact permissions and the narrative products. Membership and revision bindings
are derived by the runtime, so a model is never asked to copy an invariant that
the planner already owns. Stable prompt prefixes are deliberately placed before
dynamic state, and provider receipts expose per-attempt token/cache usage plus
bounded local validation failures.

`ModelPort` remains Ghostlight's single inference seam. The
`epiphany-codex` implementation is an ordinary typed consumer of Epiphany's
loopback model-provider connector: it emits an encrypted, expiring MessagePack
request over bounded TCP-framed CultNet and accepts only a correlated ordered
text stream plus terminal receipt. A stable cache key is derived from stage,
logical model class, and output schema rather than mutable campaign content.
Epiphany owns Codex credential custody and physical calls; Ghostlight still
owns stage projection, schemas, retries, semantic validation, receipts, and all
kernel admission. Neither side can borrow the other's state authority.

Canonical actor cell slices include that actor's exact goals, obligations,
relationships, and bounded memories. Named Gestalt-member exceptions receive
the same continuity fields. The Projector narrativizes them for the Persona;
the Interpreter still receives only exact effect permissions. A promise does
not become a hard-coded behavior rule, but it cannot disappear merely because
the actor entered offscreen simulation.

The Interpreter output schema is conditional on exact decision-owner ID.
Institution, actor, Gestalt, and named-member effect variants cannot cross
subjects. Target IDs, canonical locations, pressure resolutions, movement
destinations, state references, and public channels are enumerated from that
subject's permitted slice. Movement and population-migration variants disappear
when the subject has no exact destination; target-requiring activities cannot
emit an empty target list. Semantic correction therefore judges meaning rather
than repairing type combinations local code already knows are impossible.

Each permitted activity target is one runtime-derived descriptor keyed by its
authoritative ID, with the target's exact name and current canonical locations.
Reachable actor destinations map exact location IDs to names; population
migration destinations carry both population and location identity. The
Projector, Interpreter, and effect verifier can distinguish a canonical target
actually named by the Persona from an unnamed local role, and can prove whether
“go to Reed” means movement at all. Opaque permitted IDs are not enough reason
to substitute a containing population, related institution, or unrelated
destination. Descriptor prose is disambiguation context, not a new subject or
fuzzy-match authority.

Targetless local activity is explicit rather than encoded through a convenient
unrelated subject. Preparation, investigation, communication, and obstruction
may act on an unnamed role, infrastructure, terrain, traffic, or other local
texture only at the exact supplied location. The activity records the source's
attempt, never a listener, discovery, damage, disruption, or response. A
canonical target is required whenever the attempt actually addresses one.

`AgencyProfile.information_channels` owns concrete routes through which a
subject can publish or receive reports. Actor knowledge and Gestalt shared
knowledge remain separate state and may justify an action without manufacturing
a news route. Profile maintenance removes legacy overlap and `unknown`
placeholders; the compiler rejects both categories before campaign approval.

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
Actor possessions, member equipment, institution resources, and Gestalt
resources use one `resource:*` reference ontology. When a model cites another
action's handle or invents a reference, the local owner returns the exact
action digest, offending values, and admissible set to the private same-snapshot
correction. The player receives only a spoiler-free refusal if correction also
fails.

The resolver's JSON Schema is derived from exact action authority. Each action
digest admits only its available effect kinds; unavailable lanes are absent,
not represented by empty placeholder choices. The chosen effect kind makes its
own owner, recipient, resource, relation, member, fact, pressure, and evidence
fields structurally required while forbidding fields owned by another kind.
Exact IDs and existing values come from that action's bounded slice. New
resource or pressure text remains model-proposed but locally bounded and still
requires kernel validation. This keeps deterministic impossibility out of the
semantic retry loop without letting the schema decide narrative causality.

Pressure authority is state-relative. The outcome stage receives each exact
Gestalt owner together with its current unresolved pressure strings. A
resolution must copy one of those strings; an addition must be new and cannot
overlap a resolution or repeat current state. The runtime rejects the whole
wave on a no-op, duplicate, or invented resolution before commit.
These state-transition checks belong to the outcome semantic validator, not the
generic JSON-shape retry. A well-shaped repeated pressure reaches the bounded
outcome correction with the exact rejected bundle; the correction may propose
a genuinely new pressure or `no_material_change`, while the kernel guard still
makes the repeated transition uncommittable.

Named-member private effects are action-relative too. An obligation is
available only for social activity—communication, coordination, recruitment,
or trade—where the attempt can actually create a promise or debt. Physical
preparation, investigation, or obstruction cannot mint an unrelated social
obligation merely because the member is present in the outcome slice.

Structured-stage failures name the exact stage, instance JSON pointer, and
schema pointer. Receipts retain the bounded private diagnostic; player-facing
strategic refusal remains spoiler-free. A model failure can therefore be
located without logging chain-of-thought or dumping the private prompt.
For the one same-snapshot correction, the validator also projects only the
nearest rejected containing object rather than replaying the whole output. This
gives the model the exact failed activity or effect beside the schema pointer,
preserves the stable cacheable prompt prefix, and avoids paying again for
unrelated valid output. Parallel cell failures retain the exact cell ID,
constituent IDs, and bounded anyhow cause chain in operator-only logs.

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

`advance_one_strategic_tick` owns post-commit mesh publication for scheduler,
return catch-up, and explicit player-horizon waits. Callers no longer decide
whether a strategic commit deserves a projection. A publication error is an
operator-visible derived-state failure; it cannot roll back, rewrite, or report
the already committed world transition as failed. This closes the former path
where return catch-up advanced canonical state, the subsequent player command
correctly failed stale, and an early return left the operator surface on the
previous revision.

The RUDP session owns reliable-send pressure. It admits at most 32 packets—the
cumulative-ACK horizon—queues the rest, promotes queued packets only as ACKs
retire admitted packets, and emits an exact ACK when an older retransmit has
fallen behind that horizon. Its explicit flush succeeds only after every
reliable packet is acknowledged. Large CultMesh documents no longer
depend on blasting every fragment and sleeping for a fixed interval. The wire
contract is unchanged, and model, campaign, and application authorities remain
outside the transport.

Ghostlight stages discovery-critical health and provider advertisement before
bulk projections. Its Odin schema catalog contains only the Ghostlight-owned
boundary state types advertised to consumers: `ghostlight.campaign.v1` and
`ghostlight.session_zero.v1`. Internal compiler, Persona, transition, Gestalt,
receipt, and operator documents remain available through their owning private
projection; they are not dumped into rendezvous merely because Ghostlight has
Rust types for them.

Odin registers only the `ghostlight.schema_catalog.v1` public envelope, not
those private contracts. Its provider-document adapter returns and awaits the
single durable `node.put` promise, preserving per-peer write order instead of
launching concurrent CultCache mutations behind transport acknowledgement.
The live typed snapshot returns Ghostlight source
`7522ea8405212b344441f83f502993f525276521` and exactly the two boundary schema
keys above.

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

Yggdrasil serves one native immutable Ghostlight release under
`ghostlight-dungeon.service`. The fresh-workspace handoff and `state/map.yaml`
carry the exact current source, Eve source, executable hash, provider, store
counts, and replay witness. This map describes the acceptance body without
duplicating a volatile release identity in several sections.

The hosted adversarial campaign now pressures the complete eight-cell strategic
path against real provider output. Every failed Projector, Persona,
Interpreter, effect-verifier, outcome, or kernel boundary has left campaign
revision and fictional time unchanged. Human separate-account co-op privacy and
unanimity remain unproven; the expected D&D cohort dispersed before this pass,
so no fixture or solo browser run may be presented as that acceptance.

Campaign `34929b8d-7b04-49af-9936-1c798fd79760` is the current live strategic
witness. Its revision-12-to-13 return catch-up preserved one eight-cell cover:
five exact actor/institution cells, two Gestalt cells, and one arena containing
three mutually distinct remote institutions. The arena acquired no actor ID.
Zhestokost's exact repeated posture became attributed inaction; two named
members made their own decisions; Reed kept the twelve-patient commitment; and
the player plus Reed remained byte-identical. Five selected actions produced
five bounded outcomes. Fourteen channel-aware reports remained absent from the
player surface because that actor has no route to them.

That tick used 34 stage receipts and 35 provider attempts: 83,838 prompt
tokens, 64,000 cache-hit tokens, 6,192 completion tokens, and 90,030 total
tokens. The extra Interpreter receipt records the rejected repeated institution
posture. The outcome path records an undersized bundle retry and a rejected
repeated pressure before the corrected complete bundle. All diagnostics
remained private; the campaign committed once.

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

Idunn is the only deployment and daemon-survival authority; Odin is the
discovery/rendezvous authority. Their health or availability never grants
either organ a campaign write path. Current release identity for Idunn, Odin,
Heimdall, Bifrost, and Epiphany is operational truth in `gamecult-ops`; do not
copy it here and let it fossilize again.

Release admission is based on the newest executable- or build-affecting commit
reachable from the admitted ref. Documentation, notes, state receipts, and
root Markdown are not executable release selectors. The root actuator proves
that the selected commit is an ancestor of the admitted ref before activation,
then verifies the exact installed witness. Documentation and state-only commits
therefore cannot displace the live executable.

Heimdall publishes four redacted typed discovery records to Odin under globally
unique catalog keys:
the provider, private command boundary, Eve access plugin, and transport
profile. Ghostlight resolves `heimdall:command-boundary` from Odin only for
begin, complete, refresh, and logout; valid local app sessions do not depend on
an Odin round trip. No direct Heimdall endpoint remains in the Ghostlight unit.

Epiphany is adjacent capacity, not part of Ghostlight's campaign authority and
not required by the current provider-backed runtime. Rehydrate her live state
from her own workspace and `gamecult-ops`. Her deployment remains one
Idunn-owned transaction; passive watcher tasks and cross-task exclusivity
messages are not an operational control plane.

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

Canonical non-player actors have an `actor_activity` path for ordinary
preparation, coordination, investigation, recruitment, obstruction, trade,
and communication. The Interpreter binds the actor's exact ID locally;
WorldKernel validates current location, graph targets, state references,
channels, and human-control exclusion. Actor movement and actor activity share
one action key, while successful consequences remain a separately resolved,
digest-bound outcome.

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

Automatic presence has two different authorities. Recasting an existing exact
member is reversible and may follow any relevant foreground event. Admitting a
first-relevance member is canonical entity creation and is available only when
the planner reason exactly matches the immediately committed player-speech
turn. The generated schema closes the individuation lane for resolved attempts,
and `WorldKernel` independently rejects any such plan. Outcome prose and
narration are not entity-admission inputs.

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

Native and browser clients share that logical surface and command authority.
The native path is `ghostlight.native.player` over loopback RUDP
`127.0.0.1:4102`; it exposes only Heimdall begin/completion, actor-filtered
surface retrieval, and canonical Eve invocation. Heimdall persists the OAuth
attempt and single-use completion. Ghostlight redeems the completion into the
same app-session record used by HTTP, then derives every actor and campaign
binding server-side. The installed native client stores only the opaque
Ghostlight bearer in a mode-0600 CultCache file. It never receives a Heimdall
access or refresh claim and cannot submit actor, member, account, or authority
identifiers.

Odin advertises the redacted native route and operation names in the provider
document. It does not proxy native commands or persist session tokens, attempt
handles, claims, account state, membership, or player surfaces. The boundary is
loopback-only; remote native use requires a trusted host crossing and still
passes Heimdall admission. A malformed or forged native request returns a typed
denial before product projection and cannot enter either kernel mailbox.

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
Player command responses contain only assessment and public commit/roll
receipts. Canonical campaign state and spoiler-bearing actor or institution
state are operator-only. Informational rolls add only their exact previewed
finding to the acting character and a provisional branch fact. The assessor
deterministically binds typed findings into visible stakes before validation,
so formatting is not delegated to a correction attempt. The compiler
classifies each retrieved source as direct seed, setting background, or excluded
before generation. Only direct-seed source text enters causal world compilation;
background and excluded sources remain coverage provenance and cannot donate
story incidents or cast.

The player-facing story is the chronological lowering of exact committed
`NarrativeTurn` rows. Player speech, Persona speech, and resolved outcome prose
already enter the transcript through the kernel transaction that owns their
revision. The surface does not run a second narrator or verifier model, replace
world turns, or read historical `narration_projection.v1` rows. This removes two
calls per live turn and makes display-layer invention of names, dialogue,
injuries, participants, or actions structurally impossible.

Player speech address is also typed before it can steer a reaction. A compact
Flash projection receives only the exact co-present simulation-eligible actor
IDs and names, bounded recent public turns, prior conversational focus, and the
new utterance. It returns the exact Persona actors from whom the utterance asks
for a response. `WorldKernel` revalidates those IDs and commits them with the
speech `NarrativeTurn`; neither Eve nor the native client may submit them.
Every present actor continues to appraise the turn, but the private actor slice
now distinguishes `direct_response_expected` from `present_observer`. A direct
addressee must speak or select typed deliberate silence. The kernel lowers
silence to a deterministic visible refusal, so no free-text gesture lane can
claim an unassessed world effect.

`ghostlight-campaign-inspect` is a read-only typed-store witness. It reports the
latest strategic cover, attributed appraisals and inactions, activity outcomes,
events, news, subject continuity, and model receipt/token metadata without
dumping provider reasoning or raw private narrative streams.

`ghostlight-mesh-inspect` provides the same read-only operator projection from
a copied derived `mesh.cc` snapshot. It never opens or copies the live campaign
store. Operators may extract one private JSON witness from that copy, then
delete the temporary mesh file; JSON is diagnostic export, not state authority.

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
At registry load, exact legacy `opening:*` suggestions and payloadless question
records from the pre-cut runtime are demoted from material to optional with a
CultCache compare-and-swap; fictional revision and shared epoch do not move.
Payloadless prose cannot own compilation readiness. The persisted typed draft,
not stale suggestion materiality, owns that gate.
Custom starts and characters use the same draft. The browser returns
only decision commands and cannot echo a rewritten candidate into compilation.
Candidate IDs, text, list sizes, and evidence references receive local
validation. Suggestion retrieval and every committed model invocation remain
in the Session Zero audit, including repeated calls with the same semantic
receipt hash but different attempt telemetry. A separate active-preview receipt
set is replaced by each successful compiler transaction and cleared when that
preview is retired. Publication carries only that exact proof set; it never
treats DM turns or superseded compiler runs as evidence for the approved seed,
and it never compiles the conversation transcript.

Gap-bearing compiler results are retained as exact non-canonical previews while
Session Zero returns to drafting. The host can move that exact preview into
review; this is not approval. Every member approval now binds the shared
contract digest, that member's private character digest, and the complete world
preview digest. Publication persists that digest plus the exact accepted gap
and branch-assumption lists. Replacing topology, cast, institutions,
populations, clocks, evidence lanes, gaps, assumptions, or any other preview
field therefore makes old approvals unusable. The Eve tree visibly lowers the
reviewable world shape. Player JSON excludes institution resources and goals,
and shared cast projection excludes private relationship-anchor people.
The host can discard any non-canonical preview through the SessionZeroKernel;
the command clears only preview state and returns the locked roster to drafting
without editing the contract, character drafts, or fictional time.

Session Zero owns human-controlled actor identity and state. Public player names
and premises may enter the shared compiler as setting context, but those names
are reserved outside world-cast compilation: actors and Gestalt members cannot
materialize them. The compiler's singular player object is a provisional
starting-position marker only. Ghostlight removes it and installs the approved
typed characters locally. Collision receives one same-snapshot correction and
then rejects the candidate; no deduplication or model choice can replace the
Session Zero owner.

Private character relationships now cross compilation through a typed identity
membrane. Exact player references bind to their approved actor IDs. Each
otherwise unresolved named person gets a stable server-generated
`relationship-anchor:*` actor ID. A separate private actor stage receives only
the approved display name after public topology validation; it never receives
the opaque identity handle or relationship. The shared world compiler receives
none of those private inputs. The private
stage returns characterization and one exact supplied location, but has no
schema fields for IDs, relationships, narration, facts, gaps, assumptions, or
public-world changes. Ghostlight attaches the opaque ID locally and later binds
the approved relationship only onto its owner's actor. Omission, ambiguity,
unknown placement, or collision rejects the candidate atomically. Actor,
institution, Gestalt, and `member:*` IDs are all legal directional relationship
targets, but those links never union knowledge or authority. Only the owning
member's review surface projects the compiled target, placement, and private
description before approval. A shared branch assumption exposes the count of
materialized private subjects, never their identities or relationship details.

Approved private character playability crosses a narrower compiler membrane
than private history. The shared world compiler receives bounded capabilities,
equipment, obligations, goals, and extraordinary-permission scope, costs,
limits, exposure, and effect ceilings only so it can seed precise discoverable
environmental facts for the opening problem. It never receives private
history, secrets, relationships, or pre-existing knowledge. Generic crisis
restatements do not satisfy this requirement; a concrete investigative ability
needs a concrete pre-existing finding or an explicit evidence gap.

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
Legacy Session Zero stores that used the preview receipt field as an append-only
audit migrate on registry load. The complete list remains audit history and the
final compiler transaction becomes the active proof set. This replacement is
idempotent and does not advance Session Zero revision, epochs, or fictional
time.
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
child leaf, partition value (including `other/unknown`), home location, and the
exact child receiving each parent-owned resource. Shared capabilities,
knowledge, goals, pressures, and relationships inherit through population
lineage; custody does not. Each scarce resource crosses exactly one typed
custody transfer, so a fission cannot copy one granary, vehicle, or medicine lot
into every child.
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

## World Transition Authority

`WorldKernel` owns one closed semantic `WorldMutationBatch` reducer. Foreground
attempts, NPC reaction waves, strategic actor/institution/Gestalt outcomes,
waits, group travel, and approval-gated population fission all lower their
accepted outcomes into that vocabulary before canonical state changes.
`ActionMeans` records what an actor did, and the intended effect records what
they hoped to change; neither is a committed fact. The mutation batch contains
only exact component changes admitted by a state-derived authority envelope.

The reducer validates the complete batch on an isolated component overlay,
advances each touched subject version once, and persists component state,
mutation authority envelope, mutation batch, mutation receipt, causal command
receipt, events, and aggregate compatibility projection atomically. The
aggregate campaign row is rejected as a component-only mutation target.
`WorldEffectDelta`, `ActorStateDelta`, and strategic effect variants are model
boundary migration inputs, not writers or independent physical ontologies.
The foreground assessor now derives a closed effect schema from the exact
snapshot and removes unavailable knowledge, movement, clock, and institution
lanes before inference. Missing maps lower to no mutation. This keeps both the
prompt and correction surface smaller and prevents provider `null` placeholders
from stalling an NPC initiative on an operation the actor could not perform.
Pending NPC actions are revision-scoped reaction-window state. The next
reaction wave atomically replaces the set, and the kernel resolves an action
only when the current last event is the exact current-revision reaction wave.
Same-wave assessment correction remains possible; cross-turn rebasing is not.
The same reaction commit validates its response duty against the exact
committed source turn. A missing directly addressed Persona or a null response
aborts the whole wave without private deltas, transcript additions, initiative,
or revision change; observers may still choose to speak from their own goals.

Population fission now composes admitted child entities, stable child
identities, one lineage split, exact resource custody transfers, and exact
named-member membership transfers. The old fission function that directly
wrote campaign state has been removed. Its surviving projector updates only
agency profiles, facets, cover invalidation, and `resolution_epoch` after the
canonical mutation batch succeeds.

Bounded region expansion now composes typed place admission, typed proposition
admission, and exact topology changes against one authority snapshot. Every
new destination carries an explicit outbound route from the stable origin and
a reciprocal return route with the same positive travel time. Place profiles,
proposition content, evidence references, discovery locations, containment,
route identity, distance, and topology are validated on the component overlay;
the aggregate `Location`, `Route`, and `WorldFact` rows are reconstructed only
from accepted component state. The previous direct location/fact insertion
loops are gone.

Initial compiler publication is a bounded creation transaction, not a fictional
world transition. `CampaignRegistry` owns discoverability, installs the
approved seed into a fresh staging store with one empty-store CAS, and exposes
it only by atomic directory rename. It cannot target a published campaign.
Named-person materialisation and dematerialisation are resolution transactions
over one persistent member identity. They advance world revision to stale old
actor-bound commands, advance `resolution_epoch`, clear the derived cover, and
atomically preserve the exact individual delta. They cannot change fictional
time or Gestalt-wide knowledge, resources, or pressure; the obsolete
`GestaltAggregateDelta` writer has been deleted. See
`docs/architecture/ghostlight-transition-algebra.md`.

Relationship documents in the schema catalog are revision-bound projections
of actor-owned relationship maps; they are not a second relationship writer.
Vault manifests summarize the exact provider/source/authority/temporal lanes
covered by evidence receipts and do not own Vault content. Strategic tick and
gestalt materialization receipts are different: they are atomic commit
companions binding the generic world commit to the causal model output or
baseline/member presence transition. Materialisation receipts additionally bind
the previous and next resolution epochs.

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
