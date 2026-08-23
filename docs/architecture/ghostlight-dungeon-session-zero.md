# Ghostlight Dungeon Session Zero and Bounded Co-op

## Status

This document is the authority map for campaign creation and the first bounded
co-op runtime. Direct browser-owned compilation is not a production authority.
Generated openings and roles are DM suggestions inside Session Zero; the typed
draft is the only compiler input.

## Owners

### SessionZeroKernel

`SessionZeroKernel` owns one persistent draft mailbox and its revisions. Its
MessagePack CultCache store contains the roster, single-use invitations,
shared/private channels, messages, contract, private character drafts,
boundaries, decisions, approvals, DM Persona, evidence coverage, compilation
preview, and publication receipt.

The store keeps two deliberately different model-receipt collections. The
inference audit contains every committed invocation, including two calls whose
semantic receipt hashes match but whose latency or provider-attempt telemetry
differs. The active-preview proof set contains only the compiler transaction
that produced the currently retained preview. Replacing or retiring a preview
clears that proof set without erasing the audit. A campaign publication carries
only the active proof set; historical DM turns and superseded compiler runs do
not become evidence for a seed they did not produce.

Player messages are durable conversation, not campaign facts. Projector, DM
Persona, Interpreter, retrieval, and compilation stages return proposals. Only
the kernel may change the draft.

Private DM channels use character-component and channel epochs. Independent
private turns can finish in parallel even if an unrelated private component
commits first. Shared changes serialize through the shared epoch. No output is
rebased.

### WorldKernel

`WorldKernel` receives nothing while Session Zero is drafting. Publication
builds a new campaign store in a private staging directory. Campaign seed,
membership, contract, governance, DM Persona, approved brief, approval digests,
Vault evidence, and model receipts enter one empty CultCache batch. The
directory becomes discoverable only after the batch succeeds.

Publication is idempotent by the approved seed digest. Repeating the same
publication recovers the campaign. Reusing the campaign ID for a different
digest is rejected.

## Privacy membrane

Shared DM inference receives the shared contract, aggregate boundary policy,
public party cards, shared decisions, evidence coverage, and recent shared
messages. It cannot receive raw boundaries, another player's private draft, or
private messages.

Private DM inference receives the shared contract, aggregate policy, public
party cards, that player's character draft, that player's attributed
boundaries, visible decisions, and recent messages from that private channel.
It cannot receive another player's private state.

Account hashes and invite-token hashes remain private persistence fields. They
are absent from JSON Schemas, browser projections, CultMesh surfaces, and model
contexts. Actor-specific campaign surfaces use random campaign member IDs and
the exact assigned actor.

The structured Projector and Interpreter receive their exact stable JSON
Schema before any dynamic context. OpenAI-compatible JSON-object modes
constrain syntax but do not communicate the application schema by themselves.
The DM Persona owns the natural utterance. The Interpreter's model-facing
schema has no speech field: it emits only new typed proposals and reply
affordances, after which the runtime binds the Persona output losslessly into
the proposed kernel delta. It cannot rewrite, summarize, or truncate the
Persona while extracting state. The kernel still treats the three stages as
one atomic proposal; an invalid Interpreter commits neither speech nor state.

The Interpreter receives only the current contract, its channel and member
scope, the entitled private character when applicable, unresolved material
decisions, and the exact Persona response. Non-material opening and role cards
remain renderer-visible suggestions but never enter Projector or Interpreter
cognition unless the player explicitly selects or discusses one. Transcript windows, evidence coverage,
boundaries, and public-party narrative already served their purpose in the
Projector and are not replayed into extraction. It cannot restate existing
draft fields or unresolved decisions as new proposals. Stable schema-first
prefixes preserve provider cache reuse. Empty, malformed, schema-invalid, or
stale outputs cannot change the typed draft. If both same-snapshot attempts
fail, the kernel may append a fixed local DM notice to the conversation while
leaving contract, characters, decisions, boundaries, approvals, and model
receipts unchanged.

One Interpreter turn has one owner per typed lane. A shared turn may directly
edit the draft contract or offer a contract decision, never both; a private turn
may directly edit the ordinary character draft or offer a character-patch
decision, never both. An extraordinary-permission bargain is a separate consent
lane, so it may accompany a direct mundane character patch without being
silently granted. The model-facing schema requests a same-snapshot correction for a
split-lane output, and the kernel repeats the invariant before mutation.

Generated opening and role suggestions are non-material decisions: the player
may accept, counter, discuss, or ignore them without blocking a fully custom
draft. They are UI affordances, not implicit DM memory. On daemon load, the registry demotes pre-cut payloadless questions and
generated opening records whose old `material` flags would otherwise veto a
custom draft. A payloadless question cannot own compilation readiness because
it has no typed change to accept. This migration uses an exact CultCache
replacement and does not advance the Session Zero revision or shared epoch.
Material decisions preserve the distinction between discussion and consent.
Every acceptable decision carries at least one non-empty typed contract,
character, or extraordinary-permission payload. A question or a prose promise
without an exact state change remains DM speech or a suggested reply; it cannot
render an Accept control, advance the draft through acceptance, or claim that
the promised materialization occurred. The Interpreter schema enforces the
payload before correction/retry, and the kernel rejects any legacy payloadless
decision that reaches acceptance.
The actor-filtered Eve projection shows that exact payload beside the DM's
explanation before exposing Accept. Its private character ledger projects every
player-entitled draft field, including history, secrets, knowledge, equipment,
relationships, goals, and the complete terms of extraordinary permissions.
Prose explains a bargain; the visible typed payload is the state the player is
actually consenting to.
Character patches are reversible during negotiation. Every list supports exact
removals before additions, and relationship keys support exact removal before a
replacement is installed. This permits spelling, identity, and mistaken-ledger
corrections without duplicate private subjects or transcript replay.
Accept applies only the exact typed payload currently stored in the decision.
Counter preserves that payload as an inert, visible audit and replacement
basis, records the player's text in the decision's durable channel, marks the
unresolved decision as awaiting a DM replacement, and invalidates the affected
projection epoch. `pending_counter` removes the Accept surface and makes the
retired payload uncommittable. The DM response
must produce a fresh material decision against that same channel and component
epoch; installing it and retiring the pending counter is one kernel commit.
Until then the old decision has no Accept surface and forged acceptance fails.
Inference failure, an irrelevant response, or a stale response leaves the
counter unresolved and compilation blocked.
Retry relaunches the membrane from that already persisted counter and unchanged
Session Zero snapshot. It does not ask the player to retype the counter, append
a duplicate message, or advance revision merely to try inference again; only a
later kernel-validated replacement changes state.
The countered decision ID is also part of the model snapshot binding and
permitted turn context. A deterministic Ghostlight Projector renders that one
pending decision, its retired typed payload, exact counter, and aggregate safety
policy into the Persona's narrative stream. This exact projection needs no
model call or model receipt; the focused turn records only Persona and
Interpreter inference. The Interpreter separately receives the same bounded
typed basis. When the retired proposal still has one exact typed lane, the
Interpreter emits only that replacement payload: character patch, contract
patch, or permission terms. Ghostlight binds the fresh decision ID, owner,
materiality, evidence, prompt, and persisted counter locally. Permission IDs,
actor bindings, and evidence receipts are preserved from the retired proposal
and are absent from the model's output authority. The larger union Interpreter
remains only for ordinary conversation and pre-cut payloadless migration.
Conversation history, party
cards, the contract, unrelated decisions, and unrelated private character state
are not part of the focused retry. Pre-cut decisions whose typed payload was
erased receive one bounded current-state basis for migration only. The kernel
accepts exactly one same-lane replacement with stable extraordinary-permission
identity; direct patches or unrelated decisions cannot retire the counter.

## Approval and publication

The host locks the roster before compilation. Material unresolved decisions
block compilation. The kernel also refuses a conversation-only draft: the
typed contract must identify premise, canon horizon, start, pressure, goal,
tone, pacing, consequences, narrative focus, and DM style; every active
character must have a public premise, capability, goal, and at least one
obligation or vulnerability. Group drafts additionally require a party bond.
The compiler receives an `approved_campaign_brief.v1`, not
the transcript. Private character history and secrets are withheld from world
generation; after the grounded seed validates, Ghostlight locally replaces the
provisional compiler player with the exact approved actors at one shared start.
Approved relationships cross that membrane by identity, not by free-form map
key. A relationship naming another approved player resolves to that player's
exact actor ID. An otherwise unresolved named person receives a stable,
server-generated relationship-anchor actor ID; the compiler must materialize
that exact actor and name in the seed, but it never receives the private
relationship description and may not reveal the anchor in opening narration
merely because the private ledger requires their existence. The owning player
sees the compiled subject, placement, and relationship during private review.
Their final approval therefore covers the compiler's placement without
exposing it to the table. Actors may also hold directional relationships to an
exact institution, Gestalt, or named Gestalt member. Such a relationship grants
no knowledge, resource, membership, or collective authority.
If compilation fails before a preview is installed, the roster remains locked
and the typed brief remains authoritative. A host whose current brief still
passes the readiness gate receives the Compile action again; retrying uses the
new revision and never replays or accepts transcript prose.

Evidence gaps return the draft to conversation with explicit questions. A
gap-bearing compiler result is retained as an exact non-canonical preview while
the session remains in `drafting`. The host may move that exact digest into
review, or the table may revise typed state and retire it before another
compile. Moving it to review is not approval. Every active player must approve
the current shared digest, their current private character digest, and the
exact world-preview digest containing topology, cast, institutions,
populations, clocks, evidence lanes, gaps, and branch-local assumptions.
Publication persists the approved preview digest and exact accepted gap and
assumption lists beside the campaign seed. Substituting any preview makes every
prior approval stale. Relevant edits retire the preview and invalidate the
affected approvals. The host has no override.

Pre-cut Session Zero stores used one receipt list for both meanings. Registry
load migrates that list without advancing Session Zero revision or fictional
time: all entries become audit history, while an extant world-seed preview
receives the bounded final compiler transaction from its last
`custom_retrieval_plan` through `agency_compile` as its exact proof set. Later
conversation receipts and Contract Review previews are excluded. The migration
is an atomic CultCache replacement and is idempotent on restart.

Accepted extraordinary bargains become typed permissions. Later assessment
receives their exact scope, prerequisites, costs, limits, exposure, evidence,
and effect ceiling.

## Bounded co-op

`campaign_membership.v1` binds authenticated account hashes to random member
IDs and exact actor IDs. HTTP admission derives the acting actor from this
record. Assessment confirmation includes that actor ID and cannot consume
another actor's assessment.

The first co-op mode deliberately permits one shared scene and sequential
public actions. It rejects player-versus-player state mutation, movement that
would split the party, raw unilateral waits, and unilateral resolution-budget
changes. Time, group travel, and pooled Persona-cell budget changes use
revision-bound proposals; the final approval and campaign transition commit
atomically. Group travel revalidates the shared origin and exact route, moves
every assigned actor, and advances time in one world commit.

Every human-controlled actor has a non-simulatable agency profile. Live NPC
appraisal, multiresolution cells, strategic outcomes, migration, scheduler
ticks, and away-time work all exclude those actors from Persona control and
direct mutation. Disconnection grants no proxy authority.

Each active member contributes their entitlement-provided cell allowance. Test
accounts currently contribute eight. The campaign starts at the pooled total,
capped at 128; physical provider concurrency remains independent.

## Realtime and interface

Ghostlight publishes an actor-filtered Eve surface for each Session Zero member
and campaign member. Server-sent events contain revision notifications only.
The browser refetches the authoritative Eve document after a notice; the
transport owns no messages, drafts, ledgers, approvals, or world state.

The browser exposes keyboard-operable shared/private channel tabs, live status,
public party cards, private ledgers and boundaries, unresolved decisions,
roster readiness, and explicit compile/review/approve/publish actions. The Eve
review tree visibly renders topology, non-private cast, institutions without
private goals or resources, populations, clocks, evidence-use rationales,
gaps, and branch assumptions. Relationship-anchor people remain absent from
the shared cast and appear only in their owner's private relationship card.
Privacy is labeled in text and never conveyed only by color.
The host may discard any non-canonical preview through a typed Session Zero
command and continue drafting without changing the contract, characters, or
fictional time. A retained preview never traps the table into reviewing it.

Session Zero also remains the sole owner of every human-controlled character.
The shared compiler may use player-approved public names and premises as world
context, but those names are reserved: neither ordinary cast nor Gestalt member
output may materialize them. Its singular player document is only a provisional
starting-position marker. After compilation, Ghostlight removes that marker and
installs the approved typed characters. A candidate that duplicates a reserved
player identity is corrected against the same snapshot or rejected atomically.

An approved unresolved relationship person crosses world compilation as a
server-owned identity anchor. The shared world compiler never receives that
person's name, identity handle, or relationship. After the public topology is
validated, a separate private stage receives the approved name and exact
available locations and must synthesize exactly one ordinary actor candidate.
It remains responsible for that person's location, capabilities, knowledge,
equipment, obligations, and goals, but cannot emit IDs, relationships,
narration, facts, gaps, assumptions, or public-world changes. The compiler then
attaches the opaque canonical ID locally and the approved relationship is added
only to its owner's actor. Omission, ambiguity, unknown placement, or ID
collision rejects the whole candidate. Shared output cannot leak private input
that its owning model never received.

## Inline grounded suggestions

Opening and role generation is no longer a browser-owned creation flow. On a
new draft, the compiler retrieves three distinct Vault-grounded opening
suggestions and installs them as shared typed decisions. Accepting one amends
the draft for further discussion; it does not publish a world. Ghostlight then
retrieves three grounded role suggestions and installs separate typed decisions
in each player's private channel. A custom premise uses the same conversation,
draft, approval, and compiler path.

## Contract Review

A launched campaign can enter a focused Contract Review backed by the same
Session Zero kernel and privacy projections. The review is bound to the exact
world revision at which it began. Any intervening world commit makes it stale.
Every active member approves the revised shared digest and their own character
digest before the host can publish.

Publication updates the existing campaign, contract, membership permissions,
DM Persona, approved brief, governance epoch, actors' approved forward-looking
ledger fields, event, and receipt in one CultCache batch. It cannot change
membership custody, locations, topology, knowledge, memories, historical
events, Vault, canon horizon, or established starting geometry. Tightened
private boundaries publish immediately as an anonymous strictest campaign
policy; relaxation waits for unanimous review publication. Narration, action
assessment, resolution demand, cell Projectors, Interpreters, and semantic
verification receive the approved contract and aggregate policy. Actor Personas
still receive only lived narrative, not policy schemas or attributed safety
records.

## Deferred governance

Split parties, private in-play actions, simultaneous declarations, PvP,
delegation, late joining, permanent post-publication departure, group
fork/reset/export, and voice remain unsupported.
