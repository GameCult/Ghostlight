# GhostlightDungeon MVP Architecture

## Objective

GhostlightDungeon is a persistent solo and bounded-co-op narrative simulation
grounded in a source-owned worldbuilding Vault. It gives the world stable
geography, bounded knowledge, durable actors and institutions, explicit
consequence, and motion beyond the players' attention. Players describe
attempts. The world decides what is possible, what is opposed, and what changes.

The MVP runs as a native Rust daemon on Yggdrasil and publishes a
Heimdall-authenticated Eve/CultUI surface for one to eight campaign members.
Idunn owns daemon continuity and Odin owns Verse discovery; neither can mutate
campaign state. Aetheria is the bundled setting adapter. The core world,
Persona, and Vault contracts remain setting-neutral.

Ghostlight's existing v0 schemas and Pallas, Lucent, and Corvid fixtures are
evidence and regression scenarios. They do not own runtime state or commits.

## Implementation frontier

The Rust runtime contains a Session Zero-owned world-compiler seam:

- `VaultProvider` search results become exact, hashed evidence witnesses from
  VoidBot's live streamable MCP response shape;
- opening generation requires exactly three distinct eras, places, and
  pressures and installs them as shared Session Zero decisions;
- role generation requires exactly three grounded roles and installs them in
  each player's private channel;
- custom compilation emits a bounded typed campaign, evidence receipts, gaps,
  branch assumptions, and an approval-required preview;
- custom compilation classifies every exact witness as direct seed, setting
  background, or excluded before world generation, so a nearby story cannot
  donate its cast, incident, clocks, or institutional posture to a new branch;
- a separate Flash-model lane uses two stable broad retrieval queries to
  extract witnessed remote institutions in parallel with local evidence
  classification. A Pro synthesis stage turns exact supporting claims into a
  concise strategic doctrine; local verification rejects unsupported doctrine;
- remote institutions begin with deterministic six-axis profiles: their own
  authority boundary and explicit unknown geography, ideology, economic role,
  body, and information scope. Fine resources, channels, relations, and current
  posture compile only when causal relevance supplies evidence for them;
- deterministic seed validation rejects missing occupancy, dangling or
  zero-time routes, invalid containment, invalid clocks, and missing players;
- unanimous current-digest approval plus explicit host publication atomically
  stores campaign, membership, contract, DM Persona, approved brief, exact
  Vault receipts, model receipts, and approval evidence in the campaign `.cc`.

The Session Zero DM exposes generated openings, grounded role selection,
unrestricted custom starts, approval-gated material gaps, exact-document
witnesses, on-demand destination expansion, and persistent canon candidates.
A live custom-start acceptance run on 2026-08-16 consumed 30 VoidBot witnesses
across three exact receipts and produced three locations, three actors, two
institutions, two clocks, explicit gaps, and branch assumptions. It remained
an uncommitted revision-0 preview with `requires_approval: true` until the
tester explicitly approved it.

Heimdall owns public identity, Discord OAuth, and the KLTST guild-role decision.
Ghostlight creates a short-lived login attempt and asks Heimdall for a trusted
backend callback. Heimdall returns the result and token directly to Ghostlight;
the browser can only poll and adopt that completed attempt. Ghostlight verifies
the EdDSA `aud=ghostlight` access claim against Heimdall's published JWKS, maps
the account to stable app-local campaign authority, and discards the provider
token. Only hashed local session aliases enter `service/auth.cc`.

## Body and faculty map

| Faculty | Owner | Body | Authority |
| --- | --- | --- | --- |
| Self | `SessionZeroKernel` / `WorldKernel` mailboxes | One draft `.cc` / one campaign `.cc` | Own draft negotiation or canonical world revision, never both |
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

### NPC initiative authority

- **Owner:** `WorldKernel` owns the transition from a pending reaction proposal
  to the single NPC action opportunity for that revision.
- **Inputs:** the complete validated proposal set committed by the reaction
  wave, expected campaign revision, deterministic initiative winner, and a
  locally validated assessment bound to that same revision.
- **Outputs:** one `npc_action_resolved` commit that consumes the proposal set,
  obtains the server-side roll when admissible, applies the selected typed
  outcome, and stores the roll and commit receipts atomically.
- **Derived state:** priority ordering and the selected winner are calculations,
  not world truth and not commits.
- **Forbidden writers:** Persona output, the initiative selector, narrator, and
  HTTP handler cannot begin or resolve an NPC action directly.
- **Shared paths:** live reactions, interrupts, and future offscreen
  individualized actions all submit `ResolveNpcAction`; it applies the same
  assessment validation, d20 bands, and typed outcome rules as player attempts.
- **Cut line:** pending proposals are no longer a durable action queue. Once one
  proposal gains the opportunity, every sibling proposal from that snapshot is
  consumed in the same commit as its resolution; actors reappraise that result
  before another action. Model or validation failure before this command cannot
  leave an action-begun half-state.

This deliberately serializes canonical consequence while keeping perception
parallel. It prevents two proposals assessed against the same world snapshot
from both committing incompatible outcomes.

Each assessment also carries four bounded typed outcome deltas. The accepted
MVP delta vocabulary is deliberately narrow: conditions and relationships on
actors within the acting actor's location, self-movement over an existing
route, advancement of existing clocks, and posture changes to existing
institutions. It cannot create actors, places, capabilities, knowledge,
equipment, clocks, institutions, custody, or branch facts. The kernel validates
all four bands before storing an assessment and applies only the OS-random
roll's selected band in the same atomic commit as the roll receipt. Narrative
stakes describe that transition; they are no longer the transition.

The scheduler may calculate pending work, but `AdvanceStrategicTick` owns the
transition. Return catch-up invokes that same command before the next player
action is admitted.

### Narration projection authority

- **Owner:** committed campaign state and events own what happened; the
  narrator owns only a readable projection of one exact revision.
- **Inputs:** a player-visible slice of location, visible actors, explicit
  speech/stakes, recent committed events, and evidence receipts.
- **Outputs:** immutable `narration_projection.v1` CultCache rows bound to a
  campaign ID and source revision, plus private model-stage receipts.
- **Derived state:** story prose and the Eve transcript card are projections;
  neither is a world fact, actor memory, action, or correction path.
- **Forbidden writers:** narrator output cannot append events, speech, facts,
  deltas, memories, clocks, topology, or campaign revision.
- **Shared paths:** player attempts, NPC attempts, waits, strategic ticks, and
  other committed transitions invoke the same post-commit narrator projection.
- **Cut line:** the existing campaign transcript retains explicit speech and
  deterministic stake text only. Generated connective prose lives outside the
  campaign row and cannot be read back as canonical input except as a display
  projection.

The narrator rechecks the campaign revision after inference. Stale or malformed
prose is discarded. Successful projections survive refresh without acquiring
write authority over the world they describe.

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

The population-scale partition and cell-wave authority is specified in
`ghostlight-multiresolution-agency.md`. It replaces a flat list of strategic
gestalts with a dynamic, budget-aware connected cover while preserving the
materialization rules below for named people.

### Gestalt population materialization

Large low-focus populations use a gestalt Persona without sacrificing durable
individual identity.

- **Owner:** `WorldKernel` owns whether a member currently occupies an active
  actor slot. The gestalt owns shared baseline state. The member delta owns
  individual divergence.
- **Inputs:** relevance/perception proposal, expected campaign revision,
  expected gestalt version, optional existing member-delta version, current
  scene, and reviewed consequences. A first encounter may propose a bounded
  identity and initial delta for a member not yet present in the catalog.
- **Outputs:** a materialized actor derived from baseline plus delta, or an
  updated persistent delta plus optional reviewed aggregate gestalt changes and
  a materialization receipt.
- **Derived state:** the active actor is a projection/composition. It is not a
  second owner of the gestalt or member identity.
- **Forbidden writers:** proximity checks, Persona output, scheduler, scene
  rendering, and relevance heuristics may propose promotion or demotion but
  cannot create, erase, merge, or rewrite a person.
- **Shared paths:** first encounter uses `IndividuateGestaltMember`; return to a
  known member uses `MaterializeGestaltMember`; leaving perception uses
  `DematerializeGestaltMember`. Scene teardown, reload, and offscreen catch-up
  reach those commands through the campaign mailbox.
- **Cut line:** dematerialization never deletes the member delta. Gestalt ticks
  never overwrite member-specific memories, relationships, possessions,
  injuries, promises, or identity.

The composition order is `gestalt baseline -> persistent member delta ->
current scene`. Dematerialization writes individual consequences to the delta;
only explicitly reviewed population-level learning may update the gestalt.
This allows a corporation or village to take one strategic turn while John the
blacksmith remains John when encountered again.

After each committed player event, a cheap structured relevance stage receives
only the current gestalt/member catalog, materialized member IDs, player
location, and event summary. It proposes one `GestaltPresencePlan`, including
at most the bounded first-relevance identities needed by the scene. The kernel
validates exact gestalt/member versions, identity uniqueness, current
materialization state, and scene location, then applies the whole plan through
revisioned commands before the participant appraisal wave. Automatic plans
cannot write aggregate gestalt learning. A malformed, conflicting, stale, or
partially invalid plan changes nothing. The compiler may seed likely members,
but it is not required to predict every future person at campaign creation.

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

### Source-grounded Epiphany extraction map

The current implementation lives across these Epiphany source owners:

| Current source | Current responsibility | Decision |
| --- | --- | --- |
| `epiphany-core/src/persona_turn.rs` | Epiphany-specific input documents, prompt rendering, effect schema, effect validation, stage/terminal receipt documents, and atomic terminal-decision insertion | Split |
| `epiphany-openai-runtime/src/persona_executor.rs` | Three-stage ordering, model-runner port, stage replay, brake checks, causal reasoning contexts, output hashing, and terminal receipt creation | Move/generalize |
| `epiphany-openai-runtime/src/bin/epiphany-persona-service.rs` | Reserves Epiphany heartbeat work, reads Epiphany memory and transcript, observes repo activity, assembles the execution plan, admits failure/terminal state, and routes accepted speech toward Epiphany's signed Discord crossing | Stay/adapt |
| `epiphany-core/src/persona_conversation.rs` | Epiphany conversation lifecycle, retention, terminal reconciliation, and downstream state/mouth routing | Stay |
| `epiphany-core/src/persona_discord_crossing.rs` and `persona_discord_permit.rs` | Signed Epiphany public-consequence crossing and receipt verification | Stay |
| Epiphany runtime spine, Mind, heartbeat, memory, and CultMesh brake documents | Canonical Epiphany state, admission, scheduling, memory, and inference permission | Stay |

#### Move to Ghostlight

The shared Ghostlight organ owns:

- `PersonaProjectionPlan` with actor id, snapshot/version binding, stage model
  choices, permitted typed input slice, and consumer receipt context;
- `ProjectorInput` as a consumer-supplied typed slice whose generic categories
  are identity experience, memories, perceived events, relationships, goals,
  knowledge, capabilities, pressures, and affordances;
- Projector rendering and the rule that its output is one private lived
  narrative stream without JSON, action syntax, raw state, or substrate
  instructions;
- Persona invocation whose domain input is only that lived narrative stream;
- Interpreter invocation over the lived stream plus Persona output;
- generic `PersonaStageReceipt`, causal predecessor binding, snapshot binding,
  request/output hashes, exact replay, empty-output refusal, schema validation,
  and terminal receipt;
- generic typed outputs: private delta proposals, explicit speech, reaction
  priority, and world/action proposals;
- concurrency primitives for parallel participant appraisal waves.

The current `PersonaModelRunner` trait and `execute_persona_model_turn_with_runner`
shape in `persona_executor.rs` are the strongest extraction seed. They already
separate inference transport from orchestration and prove exact replay. The
Epiphany reasoning-basis and decision-context types are consumer-specific;
Ghostlight instead exposes receipt-context hooks so Epiphany can keep generating
those documents without placing her Mind inside the shared crate.

#### Stay in Epiphany

Epiphany keeps:

- `PersonaIdentity` meaning and the canonical `gamecult.persona_state.v0`
  projection of her Mind;
- `EpiphanyAgentMemoryEntry`, semantic-memory retrieval, pending Discord
  mentions, repo activity observation, social affordances, organ dependencies,
  and raw transcript ownership;
- heartbeat reservation, swarm brake state, runtime sessions/jobs, reasoning
  basis, decision contexts, and Epiphany terminal conversation lifecycle;
- the `state_note`, `say`, and `drop` mapping used by her current public Persona;
- Mind admission of memory effects and all signed Bifrost/Discord request,
  permit, delivery, and receipt paths;
- policy about allowed channels, speech acts, safety notes, retention, and
  whether a candidate becomes a public consequence.

`put_persona_terminal_decision` currently atomically stores Epiphany's effect
document and terminal receipt. That storage remains Epiphany-owned. The shared
organ returns a terminal bundle; Epiphany translates it to her documents and
performs her own exact CAS.

#### Split at narrow ports

- **Typed state slicer:** Ghostlight defines generic categories and redaction
  invariants. Epiphany reads her state and constructs the permitted slice.
- **Narrative rendering:** Ghostlight owns the generic membrane and stage
  contract. Epiphany supplies Persona-specific doctrine and authored voice
  material as typed inputs, not an alternate orchestration path.
- **Model transport:** Ghostlight defines an async stage-runner port. Epiphany
  implements it with `EpiphanyModelRequest`, Codex/OpenAI-compatible auth,
  runtime jobs, and private model-event recovery. GhostlightDungeon implements
  it with DeepSeek.
- **Receipts:** Ghostlight owns portable stage and terminal fields. Epiphany's
  adapter augments them with `EpiphanyReasoningBasis`, sealed decision contexts,
  brake evidence, and runtime-spine identities.
- **Interpreter vocabulary:** Ghostlight owns the portable proposal envelope.
  Each consumer registers or supplies its typed effect payload schema. The
  consumer validates domain policy again before admission.
- **Execution permission:** the shared executor asks a consumer-owned permit
  port before each inference stage and before terminalization. Epiphany binds
  it to the CultMesh swarm brake; GhostlightDungeon binds it to live-priority
  and campaign snapshot gates.

#### Delete or demote in Epiphany after parity

- Delete local ownership of three-stage ordering, stage id generation, hashing,
  exact replay, and terminal bundle construction from
  `persona_executor.rs`; its replacement is an Epiphany adapter around the
  Ghostlight crate.
- Delete the second injection of semantic memory, repo activity, pending
  mentions, social affordances, and raw transcript into the Persona turn.
  Those inputs belong to the Projector and must reach Persona only through the
  lived narrative stream.
- Demote Epiphany's prompt builders to Persona-specific policy/templates fed to
  Ghostlight, or delete them when the generic renderer fully owns the membrane.
- Keep the old effect document reader only as a migration reader for already
  persisted receipts. It cannot execute new turns or admit new effects.
- Do not retain an environment flag, fallback runner, or compatibility mode
  capable of choosing the old local executor for a new turn.

#### Strict narrative-stream correction

The current Epiphany `PersonaTurnInput` contains `projected_state` **and**
identity, semantic recall, pending mentions, repo activity, social affordances,
and transcript. Its prompt renders those channels again. That means the Persona
does not currently receive only the Projector's narrative stream.

The shared v1 contract removes those parallel inputs:

```text
consumer-owned typed slice
  -> Ghostlight Projector
  -> LivedNarrativeStream { text, snapshot_binding, receipt_ref }
  -> Persona model receives text only as domain context
  -> natural Persona narrative
  -> Ghostlight Interpreter receives stream + output + consumer effect schema
  -> typed proposal bundle
  -> consumer-owned validation and commit
```

Model transport may carry provider metadata and a fixed system instruction that
identifies the model as the Persona, but it may not smuggle state, transcript,
tools, routing instructions, or effect syntax around the lived stream.

#### Extraction verification

Before deleting the Epiphany executor, capture replay fixtures from its existing
tests and representative real terminal receipts. Run old and shared organs over
the same typed projector inputs with frozen model outputs. Verify:

- identical stage order, causal chain, hashes, replay refusal, and terminal
  idempotency;
- channel escape, malformed effects, empty output, wrong-model replay, brake
  engagement, and stale snapshot all fail without a terminal admission;
- Epiphany's adapter produces the same admissible `state_note`, `say`, and
  `drop` documents from the shared proposal bundle;
- the Persona request contains no second state or transcript channel;
- no new Epiphany turn can invoke the old executor after cutover;
- signed mouth delivery remains entirely downstream of Epiphany Mind admission
  and is never inferred from Ghostlight completion.

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

### Campaign registry and session authority

- **Owner:** the service `CampaignRegistry` owns the mapping from authenticated
  account hash to selected campaign ID and from campaign ID to its sole
  `CampaignRuntime` (`CampaignStore` plus `WorldKernel` mailbox).
- **Inputs:** persisted hashed session authority, named campaign lifecycle
  commands, runtime-root inventory, and exact campaign IDs.
- **Outputs:** one selected campaign runtime per entitled member, exact
  member-to-actor bindings, isolated campaign `.cc` paths, and lifecycle
  receipts. Group fork/reset/export remains disabled pending consent policy.
- **Derived state:** browser selection, campaign lists, filenames, download
  responses, and operator cards are projections of registry/control state.
- **Forbidden writers:** route handlers, invite cookies, filesystem discovery,
  fork/reset/export helpers, and the browser cannot select or mutate a campaign
  except through the registry's typed lifecycle operations.
- **Shared paths:** campaign creation approval, selection, fork, reset, reload,
  scheduler lookup, command dispatch, and export all resolve the same
  session-owned registry entry before touching a kernel.
- **Cut line:** `campaigns/default/campaign.cc` and the process-global kernel are
  removed as authorities. A session token is authentication, not campaign
  identity; its persisted control record explicitly selects a campaign.

Fork creates a new campaign ID and store from a consistent source snapshot,
records lineage, then starts a fresh mailbox over the fork. Reset creates a new
branch from the campaign's approved seed/export rather than rewriting history
in place. Export snapshots the selected `.cc` plus manifest and evidence
receipts without granting the export path write authority over live state.

The service auth row stores only hashed session IDs, each session's owned
campaign-ID set, and its selected campaign ID. Routes resolve that selection to
the registry before loading state or dispatching commands. Preview ownership is
also session-bound; a different authenticated tester receives `403` without
consuming the owner's preview. The scheduler enumerates registry runtimes and
submits ticks to each campaign's own mailbox.

### Vault provider and world compiler

`VaultProvider` exposes source search, surrounding context, exact documents,
source witnesses, authority lanes, and temporal scope. The Aetheria adapter
calls VoidBot's canonical MCP retrieval endpoint at Yggdrasil loopback. It stores
exact evidence receipts used by the campaign, never vectors or a rival index.

Compilation is staged and approval-gated:

```text
Begin Session Zero
  -> retrieval plan
  -> local exact evidence receipts || stable remote-agency retrieval
  -> source-use classification || witnessed remote institution extraction
  -> three distinct opening decisions or custom discussion
  -> three private role decisions or custom character negotiation
  -> typed campaign contract + private character drafts + exact approvals
  -> direct-evidence bounded region + deterministic coarse remote profiles
  -> semantic agency profiles only for locally materialized subjects
  -> coverage/gap/assumption preview
  -> explicit approval
  -> CreateCampaign commit
```

Reversible local texture may enter as provisional branch fact. Material gaps in
geography, mechanics, institutions, or extraordinary capability require
approval. Canon candidates are review/export records and cannot edit the Vault.
The approval preview exposes the source-use coverage. Only direct evidence may
shape the local seed. Background evidence remains receipted as setting coverage
for the tester and future non-causal lore projections, but its source prose does
not enter the world-seed prompt; excluded evidence is likewise absent. This
separation preserves provenance without making retrieved adjacency into
fictional causality.

The remote catalog is not a back door into the local seed. Extraction proposes
named institutions plus exact supporting claims. Synthesis writes one concise
strategic doctrine from those claims, and a separate verifier must accept it.
Unsupported or oversized entries become approval-preview gaps rather than
institutions. This lets the campaign represent distant powers without importing
another story's current cast, incident, or arbitrary excerpt as behavior.

The coarse profile is intentionally sparse. Asking the Pro agency compiler to
repeat six semantic axes for every remote power exhausted output tokens while
adding no authority. Ghostlight now derives those remote profiles locally and
asks the model only about locally materialized subjects whose behavioral cuts
need semantic judgment. This is both cheaper and stricter: unknown remote state
stays unknown until on-demand compilation earns a sharper claim.

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

Informational attempts may reveal an existing `WorldFact`; they may not author
one. `WorldFact` owns branch truth and the location boundaries at which a fact
can be discovered. `ActorState.knowledge` owns only which exact statements that
actor has learned. The compiler or an approval-gated region expansion must
therefore establish a clue before a roll can expose it.

The assessor receives only facts already known by the acting actor or marked
discoverable at that actor's current location. Every proposed knowledge
addition must exactly match one of those statements. A fact known by the acting
actor may be communicated to another present actor; a location-bound fact may
be discovered only by the acting actor. Player attempts and NPC initiative use
the same validator. The kernel atomically updates actor knowledge but never
creates a fact from model prose.

This ownership makes the negative invariant structural: an invented protocol
number, hidden culprit, remote event, or clue cannot become true because an
assessor wrote it into a successful stake. Unsupported information causes one
same-snapshot correction and then aborts without mutation. A vague promise such
as “identify any faults” is not a valid informational outcome, and an unstated
hidden finding cannot appear after the roll. Player-readable stakes still carry
the exact declarative statement; fact IDs, keys, and slugs remain private typed
references.

Authority map:

- Owner: `WorldFact` owns truth and discovery locations; `ActorState` owns
  learned access.
- Inputs: approved compiler or region-expansion facts, current occupancy,
  actor knowledge, and the attempted effect.
- Output: an assessment whose information deltas are a subset of existing
  accessible facts.
- Derived state: the visible stake and narration are projections of that exact
  assessment and committed revision.
- Forbidden writers: Personas, Interpreters, assessors, narrators, and
  `apply_world_effect` cannot create facts.
- Shared path: player `Attempt` and `ResolveNpcAction` call the same fact-access
  validator and atomic commit primitive.
- Cut line: the former promotion of arbitrary knowledge prose into a new
  branch-local `WorldFact` is deleted.

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
character can access its channel. Each tick first projects current pressure,
builds a connected budgeted agency cover, and runs one private Persona membrane
per cell. Every appraisal and stage receipt must validate before the kernel
commits the strategic wave atomically.

Live priority is an interrupt, not a polling promise. A live request publishes a
notification that drops the in-flight background wave future, aborting its cell
tasks and provider requests before further stages launch. Live requests hold a
shared commit gate; scheduler commits require the exclusive side and therefore
cannot cross a live request. Return catch-up deliberately uses the live command
path and completes required ticks before admitting the player's next action.

### Eve/CultMesh and browser

Ghostlight publishes `gamecult.eve.surface.v1` and accepts typed Eve commands.
The TypeScript browser host pins Eve's browser lowering package and owns only
transport, sessions, and local rendering. It does not invent a parallel UI
state model.

The player HTTP command boundary returns a dedicated spoiler-safe projection:
assessments, public commit receipts, roll results, and narration. It never
serializes the canonical campaign snapshot. Full actor, institution, evidence,
and model-stage state belongs to the authenticated operator projection only.
Player and model strings enter the DOM through text nodes rather than HTML
insertion, and all laboratory inputs have programmatic labels.

The same rule covers Session Zero and campaign-management routes. Opening and
role suggestions are filtered typed decisions inside shared or private
channels; raw retrieval/model receipts remain private. The review projection
contains promised topology, institutions, populations, clocks, evidence-use
coverage, gaps, branch assumptions, and only the current viewer's private
character state. Publication, expansion, and fission return small public
receipts. Group fork/reset/export is rejected until its consent policy exists.

The player surface includes Session Zero channels and ledgers, transcript,
composer, Assess/Attempt, roll confirmation, actor-specific character/news
state, unanimous time/travel/budget proposals, Contract Review, and optional
operator inspection.
CultMesh state is the source of the UI projection. HTTP health is a probe over
the same service-state document.

The authenticated operator inspector projects the selected campaign's full
typed state, topology, evidence receipts, commit receipts, model-stage receipts,
rejected-proposal receipts, and scheduler live-turn pressure. It contains no
provider reasoning content or secret material. Kernel command refusals append a
private receipt without changing campaign revision, allowing the laboratory to
inspect why impossible, stale, or malformed proposals were rejected.

Live compiler and player command paths hold a process-local inference-pressure
lease. The five-minute scheduler observes that pressure and launches no new
campaign work while any live lease exists. Already-committed campaign state
does not require a repair pass when background launch is skipped; the next
scheduler pulse or return catch-up uses the same strategic-tick command.

## Model boundary

The provider-neutral model port records provider, model, request hash, source
receipt ids, latency, output hash, validation result, state version, provider
request id/fingerprint/finish reason, and prompt/completion/cache-hit/cache-miss
token counts for every provider attempt. Local schema and semantic failures carry
a bounded exact error. The runtime never records provider reasoning content.
Structured stages request JSON Output and then validate locally. Empty or
malformed output gets one retry against the same snapshot. A second failure,
timeout, retrieval outage, or stale result produces no mutation.

Prompt projection follows the authority boundary instead of shipping one large
state packet to every stage:

- stable instructions and schemas precede changing campaign context so provider
  prefix caches can do useful work;
- Projectors receive only the typed facts needed to form the actor or cell's
  lived situation;
- Personas receive only the resulting narrative stream;
- Interpreters receive the narrative output plus exact action permissions, not
  a second copy of the entire state slice;
- deterministic bindings already owned by the runtime—cell membership, cell id,
  world revision, and resolution epoch—are attached locally after interpretation
  rather than copied by a model.
- remote institution IDs, coarse unknown facets, and evidence bindings are also
  derived locally; the model spends completion tokens only on witnessed mandate
  selection and behaviorally meaningful local profiles.

This is both a safety and cognitive-efficiency rule. Models spend tokens on
judgment and voice; the kernel spends deterministic work on identity, versions,
coverage, references, and commit authority. A resolution-demand model may raise
subject salience, but only pins and active relevance leases may force singleton
cells or an effective-budget overage.

Prompt quality is judged by useful decision work per token, not prompt brevity
alone. Each stage receives one stable, cacheable contract followed by the
smallest revision-bound context that can support its decision. Receipts make
prompt, completion, cache-hit, and cache-miss tokens visible per attempt so
acceptance can reject stages that spend heavily while producing no meaningful
appraisal, state proposal, or player-facing consequence.

Live model allocation:

- `deepseek-v4-flash`, non-thinking: projection, interpretation, retrieval
  planning/reranking, verification, and offscreen actors;
- `deepseek-v4-pro`, non-thinking: compilation, action assessment, live
  Personas, and narration.

`reasoning_content` is neither persisted nor displayed.

## Hosting and security

The daemon serves the embedded browser bundle on TCP 8831. Public access is
lowered through the existing Heimdall-authenticated `/ghostlight/` reverse
proxy path; the application does not add a rival identity system. One to eight
members map to distinct player-controlled actors while every command continues
through the same campaign mailbox and atomic commit path. The bounded milestone
keeps the party in one scene; the multiplayer-intention document owns the
remaining split-party and social-governance work.

The MVP runs as `ghostlight:ghostlight` under native
`ghostlight-dungeon.service`. Idunn admits its signed typed health and owns
same-release restart continuity; its deployment brake governs mutation of the
installed artifact, not ordinary survival. Runtime directories are restricted
to the service identity and administrators. Setup installs the DeepSeek key as
a systemd encrypted credential bound to the exact credential name. The key is
absent from source, arguments, logs, environment projections, and exports.

Releases are immutable directories built from exact commits. Activation is an
atomic pointer switch, recorded in CultCache, and the rollback runbook lives in
`gamecult-ops`. Systemd starts the admitted release at boot and restarts that
same installed body through Idunn continuity. Changing the artifact,
configuration, schema, unit, or authority binding remains a separate
deployment operation.

The binary owns its build provenance. Release tooling injects the clean-tree
commit through `GHOSTLIGHT_BUILD_COMMIT`; local builds derive it from Git while
the build script watches `HEAD`, its symbolic branch ref, the HEAD reflog, and
packed refs. The health projection only publishes that compile-time value.
Launchers and manifests may verify it but may not rewrite it. Activation is
rejected unless checkout HEAD, embedded commit, immutable manifest, binary
hash, and the running health projection agree.

Live VoidBot integration is host-local on Yggdrasil at
`127.0.0.1:17875/mcp`. VoidBot remains the Vault retrieval owner; Ghostlight
stores exact evidence receipts rather than a rival index. Odin provides
discovery at `10.77.0.1:17871`. The old Starfire crossing is no longer a live
runtime dependency, and Starfire's retired VoidBot/Qdrant writers stay retired.

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
- `ghostlight.agency_profile.v1`
- `ghostlight.agency_relation.v1`
- `ghostlight.gestalt_lineage.v1`
- `ghostlight.resolution_policy.v1`
- `ghostlight.resolution_pin.v1`
- `ghostlight.resolution_demand.v1`
- `ghostlight.simulation_cell.v1`
- `ghostlight.resolution_cover.v1`
- `ghostlight.resolution_plan_receipt.v1`
- `ghostlight.resolution_control_receipt.v1`
- `ghostlight.cell_appraisal.v1`
- `ghostlight.cell_action_proposal.v1`
- `ghostlight.strategic_activity_outcome.v1`
- `ghostlight.gestalt_fission_preview.v1`

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
9. Add native systemd lifecycle, encrypted-credential setup, release
   activation, reverse-proxy, and rollback tooling under the correct repository
   owners.
10. Run fixture regressions, invariant fault tests, browser acceptance, and the
    final native Yggdrasil deployment smoke.

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
