# Ghostlight Dungeon Multiplayer Intention

## Status

Bounded cooperative multiplayer is implemented in the current MVP: one to
eight authenticated players complete Session Zero and begin in one shared
scene with sequential public actions, actor-filtered surfaces, pooled
Persona-cell entitlements, and unanimous time, travel, and budget governance.
The broader model in this document remains the direction for split parties,
private play actions, simultaneous declarations, delegation, and PvP.

The existing simulation is already shaped around multiple bounded actors. The
extension should expose that anatomy to multiple human controllers without
creating a second world authority or a special multiplayer simulation loop.

## Objective

Let several human-controlled characters inhabit one persistent campaign while
preserving the same spatial, epistemic, causal, and commit invariants as solo
play. The implemented milestone keeps the party together. Split locations,
late joining, and private in-play actions remain future governance work.

## Authority map

- **Owner:** one per-campaign `WorldKernel` continues to own canonical state,
  fictional time, revisions, rolls, events, and commits.
- **Inputs:** authenticated commands attributed to one campaign member and one
  exact human-controlled actor; NPC and Gestalt proposals; scheduler commands;
  current campaign and resolution revisions.
- **Outputs:** one atomic world transition plus actor-specific perceived events,
  narrative projections, news access, and command receipts.
- **Derived state:** party rosters, shared transcript views, presence indicators,
  ready state, and client notifications are projections of campaign membership,
  occupancy, perception, and committed events.
- **Forbidden writers:** browsers, sessions, party leaders, Personas,
  Interpreters, narrators, initiative, and realtime transports cannot commit or
  repair campaign state.
- **Shared path:** solo commands, bounded multiplayer commands, unanimous waits
  and travel, NPC reactions, scheduler ticks, and reconnects use the same
  `WorldCommand → validate → atomic commit` primitive. Future simultaneous
  windows must join this path too.
- **Cut line:** do not add a parallel room-state store, party prompt, shared
  narrator memory, or multiplayer-only mutation path.

## Intended model

```text
Player A session ──> actor A command ─┐
Player B session ──> actor B command ─┼─> campaign mailbox
NPC / Gestalt proposals ──────────────┘          │
                                                 v
                                           WorldKernel
                                                 │
                           one committed event + revision
                              │                  │
                 A-perceived projection   B-perceived projection
                              │                  │
                         A's surface        B's surface
```

A human-controlled actor differs from a Persona-controlled actor only in the
source of its candidate action. Human text still describes an attempt. It does
not bypass capability, location, custody, knowledge, opposition, assessment,
initiative, or commit validation.

## Invariants

- Every active canonical subject is represented exactly once in the resolution
  cover regardless of how many humans are connected.
- One authenticated campaign member controls one or more explicitly assigned
  actors according to campaign policy. Control is never inferred from scene
  presence.
- An absent or disconnected player's character is not puppeted. Delegation or
  Persona proxying requires an explicit, revocable campaign permission.
- Perception and knowledge remain actor-local. A shared party transcript may
  contain only events all included viewers are permitted to perceive.
- Private speech, hidden action, secret evidence, and character-local news are
  projected only to entitled sessions.
- Players in different locations receive different narrative streams derived
  from the same committed world.
- A player cannot make another player's action, reaction, injury, belief, or
  relationship true by describing it.
- Simultaneously submitted incompatible attempts resolve from one snapshot and
  one deterministic initiative/conflict pass. Arrival order at HTTP or CultNet
  transport is not fictional initiative.
- Each active member contributes the allowance returned by the entitlement
  port. The campaign pool is capped by the operator ceiling of 128. Human
  actors occupy mandatory cells without Persona inference; provider request
  concurrency remains independent of the chosen cover.
- Reconnect and replay derive the player's view from committed receipts and
  perception rules. A client cache cannot repair or override the kernel.
- One malformed, stale, or unauthorized participant proposal cannot partially
  mutate the shared world.

## Social policy still to design

The architecture makes shared worlds straightforward. A usable multiplayer
game still needs explicit answers for:

- exclusive character control, temporary delegation, and permanent departure;
- host powers over invitation, removal, reset, fork, export, and campaign policy;
- asynchronous pacing and how much fictional time one active player may advance;
- simultaneous declaration windows for conflict, stealth, and split scenes;
- consent for player-versus-player persuasion, deception, theft, restraint,
  injury, extraordinary influence, and character death;
- disclosure of hidden rolls, private messages, secrets, and retrospective logs;
- moderation, blocking, reporting, and campaign-data deletion;
- mixed privacy/provider lanes and whether one campaign may contain them;
- campaign billing, seats, guests, and ownership transfer.

These are campaign-governance decisions, not browser affordances. They must be
typed, persisted, inspectable, and enforced at command admission.

## Product sequence

1. Prove the solo loop and multiresolution setting agency. **Complete.**
2. Add campaign membership and exact actor-control assignments. **Complete.**
3. Support up to eight cooperative players in one scene with actor-specific
   output and unanimous group governance. **Complete in this milestone.**
4. Support split locations, private communication, departure, and late joining.
5. Add snapshot-bound simultaneous declaration windows.
6. Admit adversarial actions only after consent and governance policy is typed
   and tested.

The first multiplayer acceptance target should be cooperative and small. The
runtime should earn social complexity one invariant at a time.

## Acceptance direction

- Two players in one scene see one committed event and their own permitted
  interpretations.
- A private fact learned by one player never appears in the other's projection.
- Split players can act in different locations without duplicating geometry,
  actors, clocks, or institutions.
- Concurrent incompatible attempts resolve once from the same revision.
- A disconnected player remains unpuppeted while world clocks and remote agency
  continue under campaign policy.
- Rejoining reconstructs the player's permitted continuity from exact receipts.
- Gestalt merge, split, migration, and named-member rematerialisation remain
  correct while several human actors create simultaneous foreground pressure.
