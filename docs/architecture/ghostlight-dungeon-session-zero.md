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
The Interpreter emits
only new deltas from the current DM response; it cannot restate existing draft
fields or unresolved decisions as new proposals. Stable schema-first prefixes
also preserve provider cache reuse. Empty, malformed, schema-invalid, or stale
outputs cannot change the typed draft. If both same-snapshot attempts fail,
the kernel may append a fixed local DM notice to the conversation while leaving
contract, characters, decisions, boundaries, approvals, and model receipts
unchanged.

Generated opening and role suggestions are non-material decisions: the player
may accept, counter, discuss, or ignore them without blocking a fully custom
draft. Material decisions preserve the distinction between discussion and consent.
Accept applies only the exact typed payload currently stored in the decision.
Counter atomically removes that payload, records the player's text in the
decision's durable channel, marks the unresolved decision as awaiting a DM
replacement, and invalidates the affected projection epoch. The DM response
must produce a fresh material decision against that same channel and component
epoch; installing it and retiring the pending counter is one kernel commit.
Until then the old decision has no Accept surface and forged acceptance fails.
Inference failure, an irrelevant response, or a stale response leaves the
counter unresolved and compilation blocked.

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

Evidence gaps return the draft to conversation with explicit questions. A
review preview records shared and per-character digests. Every active player
must approve the current shared digest and their current private character
digest. Relevant edits retire the preview and invalidate the affected
approvals. The host has no override.

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
evidence/preview state, roster readiness, and explicit compile/approve/publish
actions. Privacy is labeled in text and never conveyed only by color.

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
