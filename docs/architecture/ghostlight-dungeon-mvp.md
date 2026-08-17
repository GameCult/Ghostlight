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

## Implementation frontier

The Rust runtime now contains the first honest world-compiler seam:

- `VaultProvider` search results become exact, hashed evidence witnesses from
  VoidBot's live streamable MCP response shape;
- opening generation requires exactly three distinct eras, places, and
  pressures;
- role generation requires exactly three grounded roles;
- custom compilation emits a bounded typed campaign, evidence receipts, gaps,
  branch assumptions, and an approval-required preview;
- custom compilation classifies every exact witness as direct seed, setting
  background, or excluded before world generation, so a nearby story cannot
  donate its cast, incident, clocks, or institutional posture to a new branch;
- a separate Flash-model lane uses two stable broad retrieval queries to
  compile a coarse remote agency catalog in parallel with local evidence
  classification. It admits only a proper name plus one short mandate that
  deterministic code can bind verbatim to a witness naming that institution;
- remote institutions begin with deterministic six-axis profiles: their own
  authority boundary and explicit unknown geography, ideology, economic role,
  body, and information scope. Fine resources, channels, relations, and current
  posture compile only when causal relevance supplies evidence for them;
- deterministic seed validation rejects missing occupancy, dangling or
  zero-time routes, invalid containment, invalid clocks, and missing players;
- approval alone submits `CreateCampaign`, which atomically stores the campaign
  and the exact Vault receipts in the campaign `.cc` store.

The hosted compiler now exposes generated openings, grounded role selection,
unrestricted custom starts, approval-gated material gaps, exact-document
witnesses, on-demand destination expansion, and persistent canon candidates.
A live custom-start acceptance run on 2026-08-16 consumed 30 VoidBot witnesses
across three exact receipts and produced three locations, three actors, two
institutions, two clocks, explicit gaps, and branch assumptions. It remained
an uncommitted revision-0 preview with `requires_approval: true` until the
tester explicitly approved it.

The invite/session authority is persisted separately in `service/auth.cc`.
Only hashes of invite and session tokens enter CultCache. Consuming an invite
atomically replaces that auth row, so daemon restart neither resurrects a used
invite nor invalidates an established session. A disposable HTTP acceptance
run verified unauthenticated `401`, pre-approval compiler surface, explicit
approval, campaign/evidence persistence, session survival across restart, and
`401` on consumed-invite reuse.

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
  session hash to selected campaign ID and from campaign ID to its sole
  `CampaignRuntime` (`CampaignStore` plus `WorldKernel` mailbox).
- **Inputs:** persisted hashed session authority, named campaign lifecycle
  commands, runtime-root inventory, and exact campaign IDs.
- **Outputs:** one selected single-player runtime per session, isolated campaign
  `.cc` paths, lifecycle receipts, and immutable export bundles.
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
calls VoidBot's canonical MCP retrieval endpoint at Starfire loopback. It stores
exact evidence receipts used by the campaign, never vectors or a rival index.

Compilation is staged and approval-gated:

```text
opening request
  -> retrieval plan
  -> local exact evidence receipts || stable remote-agency retrieval
  -> source-use classification || witnessed remote mandate extraction
  -> three distinct openings or custom opening
  -> three grounded roles or custom role
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

The remote catalog is not a back door into the local seed. Its model output may
propose at most 32 institutions and one short mandate string for each. Local
code locates that string in the supplied witnesses and requires the same source
to name the institution. Unsupported entries are omitted and summarized as
approval-preview coverage gaps; their exact rejection reasons remain in the
private model-stage receipt. They do not become campaign canon candidates.
This lets the campaign represent distant powers without importing another
story's current cast or incident.

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

Informational attempts must preview the exact finding that becomes knowledge.
On resolution, only the acting character may receive those bounded knowledge
additions; the same statement is committed as a provisional branch fact. A
vague promise such as “identify any faults” is not a valid informational
outcome, and an unstated hidden finding cannot appear after the roll.
The assessor normalizes each bounded typed finding into its visible stake before
validation and digesting. Exact visibility is therefore deterministic; a model
retry is reserved for semantic or authority failure, not punctuation mismatch.
The schema names these values as player-readable declarative statements, and a
fact ID, key, or slug cannot enter the character ledger as knowledge.

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

The same rule covers compiler and campaign-management routes. Opening and role
responses contain selectable suggestions, not retrieval/model receipts. An
approval preview contains the promised topology, public cast, institutions,
populations, clocks, player-role ledger, evidence-use coverage, gaps, and branch
assumptions without private goals, memories, relationships, raw evidence, or a
serialized campaign. Approval, expansion, fission, fork, and reset return small
public receipts; their internal command results remain inside the daemon.

The player surface includes compilation, transcript, composer, Assess/Attempt,
roll confirmation, Wait, character ledger, current place/time/pressure, news,
campaign management, fork/reset/export, and optional operator inspection.
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

The daemon serves the embedded browser bundle on TCP 8831. Firewall scope is
Starfire's private LAN and `10.77.0.0/24`; there is no public listener or
Yggdrasil web gateway. Two single-use invites establish separate HttpOnly
sessions. Campaigns remain single-player.

The MVP runs as a normal detached process under the Starfire operator account.
The launcher records the PID, exact executable path, release commit, and logs;
the stop path refuses an executable mismatch. Runtime ACLs permit the operator,
SYSTEM, and administrators. Setup reads the DeepSeek key once and writes a
machine-scoped DPAPI secret. The key is absent from source, arguments, logs,
environment projections, and exports.

Releases are immutable directories built from exact commits. Activation is an
atomic pointer switch, recorded in CultCache, and the rollback runbook lives in
`gamecult-ops`. The MVP process does not automatically survive logout or reboot.
Task Scheduler may be added when tester availability makes that useful; native
Windows service machinery is outside the MVP.

The binary owns its build provenance. Release tooling injects the clean-tree
commit through `GHOSTLIGHT_BUILD_COMMIT`; local builds derive it from Git while
the build script watches `HEAD`, its symbolic branch ref, the HEAD reflog, and
packed refs. The health projection only publishes that compile-time value.
Launchers and manifests may verify it but may not rewrite it. Activation is
rejected unless checkout HEAD, embedded commit, immutable manifest, binary
hash, and the running health projection agree.

Live VoidBot integration uses the independently verified Yggdrasil host and the
restored Starfire loopback crossing at `127.0.0.1:17875/mcp`. The repaired
tunnel task uses the live `F:\Projects` body. Retired local VoidBot/Qdrant
writers stay retired.

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
9. Add detached-process launch/stop, DPAPI setup, release activation, firewall,
   and rollback tooling under the correct repository owners.
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
