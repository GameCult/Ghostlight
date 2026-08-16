# GhostlightDungeon MVP Architecture

## Objective

GhostlightDungeon is a persistent single-player narrative simulation grounded
in a source-owned worldbuilding Vault. It gives the world stable geography,
bounded knowledge, durable actors and institutions, explicit consequence, and
motion beyond the player's attention. The player describes attempts. The world
decides what is possible, what is opposed, and what changes.

The MVP runs as a Rust daemon on Starfire and publishes a private Eve/CultUI
surface for two invited testers. Aetheria is the bundled setting adapter. The
core world, Persona, and Vault contracts remain setting-neutral.

Ghostlight's existing v0 schemas and Pallas, Lucent, and Corvid fixtures are
evidence and regression scenarios. They do not own runtime state or commits.

## Body and faculty map

| Faculty | Owner | Body | Authority |
| --- | --- | --- | --- |
| Self | `WorldKernel` campaign mailbox | One campaign `.cc` store | Orders commands and owns the campaign revision |
| Eyes | `VaultProvider` and actor perception slices | VoidBot MCP receipts, topology, occupancy, knowledge | Retrieves and witnesses; never commits world truth |
| Modeling | World compiler and action assessor | Typed campaign snapshot | Proposes topology, facts, affordances, DC, stakes, and gaps |
| Imagination | Persona and world proposal stages | Private narrative projections | Proposes speech, private deltas, reactions, and actions |
| Hands | `WorldCommand` commit path | CultCache/redb transaction | Performs the single allowed state transition |
| Soul | Validators, version gates, receipts, tests | Schemas, invariants, state and visible-path probes | Refuses stale, malformed, impossible, or unsupported mutation |
| Persona | Ghostlight Persona runtime | Narrative stream in, natural narrative out | Acts as a person without seeing schemas or raw canonical state |
| Nervous system | Scheduler, live-priority gate, CultMesh | Mailboxes, pulses, typed publications | Carries commands, pressure, progress, and projections |

## Authority map

### Owner

The per-campaign `WorldKernel` is the sole owner of canonical campaign state and
revision numbers. Each kernel is reached through one in-process mailbox. There
is no alternate scheduler writer, HTTP writer, model writer, import writer, or
reload repair path.

### Inputs

The kernel may read:

- the current typed campaign snapshot from its owned CultCache store;
- one typed `WorldCommand` carrying an expected revision where applicable;
- validated proposal documents and exact evidence receipts referenced by that
  command;
- server-owned clock and random-number ports;
- deterministic capability, custody, spatial, knowledge, resource, opposition,
  and topology rules.

Model prose is never itself a kernel input. An Interpreter must first convert
it to a typed proposal, which local validation can reject without mutation.

### Outputs

An accepted command atomically replaces the campaign snapshot and appends a
`world_commit_receipt.v1`. A resolved attempt also appends its
`roll_receipt.v1`. Other stages receive the new committed revision only after
the transaction succeeds.

A rejected command produces a typed rejection receipt or ephemeral assessment
result. It does not modify the campaign.

### Derived state

- A scene is derived from persistent occupancy, containment, route topology,
  perception, and current events. A scene is not a location owner.
- The transcript is a projection of committed events and speech. It is not
  memory or world truth.
- Eve surfaces, chat cards, ledgers, news drawers, operator views, and HTTP
  health are projections of CultMesh-visible provider state.
- Assessments are expiring previews bound to one campaign revision. They are
  not commands and are never rebased.
- Initiative is a compatibility and opportunity decision over proposals. It
  cannot commit.
- News is a character-accessible projection of committed offscreen events. It
  cannot reveal or alter omniscient state.
- Model-stage records are private receipts. They do not become facts merely
  because a model said them.

### Forbidden writers

The following may never decide or repair canonical campaign truth:

- browser handlers and chat transport;
- Eve renderers and command lowering;
- DeepSeek responses;
- Projectors, Personas, Interpreters, narrators, retrievers, rerankers, and
  verifiers;
- initiative and reaction selection;
- scheduler loops and return-time catch-up calculations;
- Vault providers and evidence caches;
- import, fork, export, reset, or reload handlers;
- health endpoints, operator tools, and debug probes.

Each must submit the same typed command accepted by the campaign mailbox or
remain a read-only projection.

### Shared paths

Player speech, assessed attempts, confirmed rolls, waits, travel, NPC actions,
institution actions, strategic ticks, campaign creation approval, imports, and
reload recovery all use:

```text
source intent
  -> typed WorldCommand
  -> mailbox ordering
  -> expected-revision check
  -> deterministic validation
  -> atomic CultCache commit
  -> commit receipt
  -> post-commit appraisal/projection wave
```

The scheduler may calculate pending work, but `AdvanceStrategicTick` owns the
transition. Return catch-up invokes that same command before the next player
action is admitted.

### Cut line

Ghostlight has no existing hosted runtime authority to preserve. The v0
training pipeline remains intact as evidence. New runtime code must not route
through its JSON fixtures or promote fixture documents into live state.

The Persona machinery currently embedded in Epiphany is a separate ownership
cut. Ghostlight will extract and generalize the projection protocol; Epiphany's
domain state and public Persona remain Epiphany-owned. No compatibility path may
allow Epiphany's old local projection implementation and Ghostlight's shared
implementation to make competing decisions after migration.

## Persona projection ownership

### The ownership decision

Ghostlight owns the reusable Persona projection machinery:

1. permissioned typed-state slicing;
2. typed slice to private lived-narrative projection;
3. Persona invocation using only that lived narrative;
4. natural narrative output to typed private deltas, explicit speech, reaction
   priority, and `WorldActionProposal` interpretation;
5. stage receipts, snapshot binding, validation, and failure isolation;
6. parallel appraisal waves for every affected present participant.

This organ is useful outside games. Its invariant is that a Persona experiences
a bounded narrative world without receiving the machinery's schemas or raw
state, while the surrounding system receives only typed proposals rather than
unreviewed narrative mutation.

Ghostlight does **not** own a consuming Persona's canonical mind. Epiphany owns
Epiphany's Persona state, voice, memories, relationships, values, permissions,
public identity, and accepted consequences. VoidBot owns its repo and companion
Persona state. Ghostlight supplies the reusable organ through narrow typed
ports.

### Extraction from Epiphany

Before moving code, map Epiphany's current projection body:

- owner of canonical Persona state;
- projector inputs and hidden prompt inputs;
- narrative stream contract;
- Persona model invocation and tool boundary;
- Interpreter outputs;
- mutation/commit authority;
- model transport, retries, receipts, and telemetry;
- Epiphany-specific assumptions embedded in otherwise general code.

Classify each part:

- **Move to Ghostlight:** general narrative projection, Persona invocation,
  interpretation, schema validation, stage receipt, and concurrency machinery.
- **Stay in Epiphany:** Epiphany state schemas, state store, relationship and
  memory authority, public voice policy, tools, schedules, and consequence
  decisions.
- **Split at a port:** state slicing, model selection, retrieval, clocks,
  receipts, and commit callbacks where both projects need control but only one
  owns each decision.
- **Delete:** duplicate local orchestration or adapters that preserve two
  projection authorities.

### Handing the organ back to Epiphany

Epiphany consumes the Ghostlight projection crate/service through a pinned
contract. Her adapter supplies:

- an Epiphany-owned permissioned state slice;
- a snapshot version and actor identity;
- Epiphany-owned model and retrieval policy where the port permits it;
- a callback or command boundary that accepts typed proposals for Epiphany's
  own validation and commit path.

Ghostlight returns narrative output, typed proposals, and stage receipts.
Ghostlight never writes Epiphany's canonical state. Epiphany's existing local
projection writer is deleted or reduced to the adapter before the shared path
is considered live.

Migration acceptance requires a replay corpus showing that the shared organ
preserves or improves character performance, plus negative tests proving the
old Epiphany path cannot still commit, override, or repair the new path.

## Runtime organs

### Campaign store

Each campaign has one row-oriented MessagePack CultCache `.cc` store backed by
redb. The daemon holds the store's exclusive owner lock for the kernel lifetime.
The runtime root is `F:\GameCult\GhostlightDungeon` with separate service,
campaign, receipt, export, and release directories. Service catalog and control
state use their own `.cc` store.

JSON exists only for published JSON Schemas and browser, MCP, and model-provider
boundaries. It is not load-bearing runtime state.

### Vault provider and world compiler

`VaultProvider` exposes source search, surrounding context, exact documents,
source witnesses, authority lanes, and temporal scope. The Aetheria adapter
calls VoidBot's canonical MCP retrieval endpoint at Starfire loopback. It stores
exact evidence receipts used by the campaign, never vectors or a rival index.

Compilation is staged and approval-gated:

```text
opening request
  -> retrieval plan
  -> exact evidence receipts
  -> three distinct openings or custom opening
  -> three grounded roles or custom role
  -> bounded region + institutional pressure graph
  -> coverage/gap/assumption preview
  -> explicit approval
  -> CreateCampaign commit
```

Reversible local texture may enter as provisional branch fact. Material gaps in
geography, mechanics, institutions, or extraordinary capability require
approval. Canon candidates are review/export records and cannot edit the Vault.

### Fiction-first resolution

`Assess` normalizes intent and intended effect, checks admissibility, selects a
DC from 5/10/15/20/25/30, itemizes referenced context modifiers, caps their sum
at ±10, states the effect ceiling and outcome stakes, and issues an expiring
digest bound to the current revision.

`Attempt` consumes that exact assessment. It obtains an OS-random d20 inside the
server command path and atomically stores roll and transition. Impossible acts
receive no roll. Natural 20 and 1 shift one band and cannot cross impossibility
or the effect ceiling. Speech occurs as speech; persuasion, deception, and
intimidation are separate intended effects.

### Actor and world action loop

After a committed event, every affected present actor receives a parallel
appraisal wave. Each actor gets only its perceived slice, private memories,
relationships, goals, and retrieved knowledge. Actor-private deltas are bound
to their own snapshot versions. Initiative selects compatible reactions for a
later `WorldCommand`; it does not suppress perception for actors outside the
current conversational focus.

### Away-time agency

The service pulses every five minutes. After fifteen minutes without player
activity, each real hour earns one pending strategic tick, capped at eight.
Live play prevents new background model calls from launching. The absent player
is not puppeted or directly harmed. Remote actors and institutions may move,
prepare, recruit, investigate, bargain, obstruct, spend resources, and advance
clocks. Information-channel-aware news is projected only if the player
character can access its channel.

### Eve/CultMesh and browser

Ghostlight publishes `gamecult.eve.surface.v1` and accepts typed Eve commands.
The TypeScript browser host pins Eve's browser lowering package and owns only
transport, sessions, and local rendering. It does not invent a parallel UI
state model.

The player surface includes compilation, transcript, composer, Assess/Attempt,
roll confirmation, Wait, character ledger, current place/time/pressure, news,
campaign management, fork/reset/export, and optional operator inspection.
CultMesh state is the source of the UI projection. HTTP health is a probe over
the same service-state document.

## Model boundary

The provider-neutral model port records provider, model, request hash, source
receipt ids, latency, output hash, validation result, and state version.
Structured stages request JSON Output and then validate locally. Empty or
malformed output gets one retry against the same snapshot. A second failure,
timeout, retrieval outage, or stale result produces no mutation.

Live model allocation:

- `deepseek-v4-flash`, non-thinking: projection, interpretation, retrieval
  planning/reranking, verification, and offscreen actors;
- `deepseek-v4-pro`, non-thinking: compilation, action assessment, live
  Personas, and narration.

`reasoning_content` is neither persisted nor displayed.

## Hosting and security

The daemon serves the embedded browser bundle on TCP 8831. Firewall scope is
Starfire's private LAN and `10.77.0.0/24`; there is no public listener or
Yggdrasil web gateway. Two single-use invites establish separate HttpOnly
sessions. Campaigns remain single-player.

The Windows service uses a dedicated virtual service identity. Runtime ACLs
permit only administrators and that identity. Setup reads the DeepSeek key from
stdin and writes a machine-scoped DPAPI secret with restricted ACLs. The key is
absent from source, arguments, logs, environment projections, and exports.

Releases are immutable directories built from exact commits. Activation is an
atomic pointer switch, recorded in CultCache. The service has restart recovery
and the rollback runbook lives in `gamecult-ops`.

Live VoidBot integration remains gated on independent verification of the SSH
host fingerprint, reconciliation of Yggdrasil inventory/DNS, repair of the
Starfire tunnel task from `E:\Projects` to `F:\Projects`, and restoration of
`127.0.0.1:17875/mcp`. Retired local VoidBot/Qdrant writers stay retired.

## Public contracts

Ghostlight publishes JSON Schema for:

- `ghostlight.vault_manifest.v1`
- `ghostlight.vault_evidence_receipt.v1`
- `ghostlight.world_compile_preview.v1`
- `ghostlight.campaign.v1`
- `ghostlight.world_fact.v1`
- `ghostlight.location.v1`
- `ghostlight.actor_state.v1`
- `ghostlight.institution_state.v1`
- `ghostlight.relationship_state.v1`
- `ghostlight.world_clock.v1`
- `ghostlight.event.v1`
- `ghostlight.player_action_assessment.v1`
- `ghostlight.roll_receipt.v1`
- `ghostlight.world_commit_receipt.v1`
- `ghostlight.persona_stage_receipt.v1`
- `ghostlight.actor_state_delta.v1`
- `ghostlight.world_action_proposal.v1`
- `ghostlight.strategic_tick.v1`
- `ghostlight.news_issue.v1`
- `ghostlight.canon_candidate.v1`

The UI consumes existing `gamecult.eve.surface.v1` and
`gamecult.eve.command.v1` contracts.

## Implementation order

1. Freeze this authority map and publish v1 schemas.
2. Map Epiphany's current projection machinery and record move/stay/split/delete
   decisions before extraction.
3. Build CultCache campaign persistence and the mailbox-owned kernel.
4. Prove shared command paths, revision checks, topology, knowledge, custody,
   assessment, roll, and atomic failure invariants with injected ports.
5. Extract the generalized Persona projection organ into Ghostlight and replay
   Epiphany/Ghostlight fixtures through it.
6. Add fixture Vault/model ports, compiler approval, actor appraisal waves, and
   away-time commands.
7. Add the VoidBot and DeepSeek production adapters without granting either
   commit authority.
8. Publish the CultMesh/Eve surface and thin authenticated browser host.
9. Add service installation, DPAPI setup, release activation, firewall, and
   rollback tooling under the correct repository owners.
10. Run fixture regressions, invariant fault tests, browser acceptance, and the
    final Starfire deployment smoke.

## Acceptance and negative proof

Acceptance follows the user-approved MVP scenarios. In addition, every claimed
invariant needs a negative proof at its visible layer:

- stale HTTP, scheduler, model, Persona, import, and reload paths cannot commit;
- leaving a scene cannot delete or reconstruct its location;
- an actor without perceived evidence cannot gain the corresponding knowledge;
- malformed or empty model output cannot partially update actor-private or
  world state;
- the old Epiphany projection path cannot commit or repair after migration;
- browser state and operator probes report the same campaign revision and
  deployed commit as CultMesh/CultCache;
- background launch pressure yields before a live player inference wave;
- unauthenticated, reused-invite, public-network, and cross-session campaign
  access are rejected.

