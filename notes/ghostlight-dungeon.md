# GhostlightDungeon MVP — Starfire-Hosted AI Dungeon Master

## Summary

Build `GhostlightDungeon` inside `GameCult/Ghostlight`: a persistent, Vault-grounded, single-player narrative simulation with a private web chat interface for two testers.

The runtime will be a Rust daemon on Starfire. It will own world compilation, typed CultCache state, d20 resolution, NPC Persona loops, offscreen simulation, narration, and Eve/CultUI publication. A thin TypeScript browser client will lower the Eve surface and submit typed commands.

Population-scale agency is implemented by the dynamic connected cover in
`docs/architecture/ghostlight-multiresolution-agency.md`. That authority map
supersedes any reading of the gestalt materialization notes below as a flat
strategic Persona list: materialization preserves named identity, while the
agency graph decides the current simulation resolution.

Ghostlight’s existing v0 schemas and training fixtures remain evidence and regression material; they do not become runtime authorities.

## Authority and invariants

- The per-campaign `WorldKernel` is the sole owner of canonical state and revision numbers.
- Player commands, NPC actions, scheduled ticks, travel, waits, imports, and reloads all enter the same `WorldCommand → validate → atomic commit` path.
- Projectors, Personas, Interpreters, retrieval, initiative, narration, news generation, dice previews, and the browser may propose or project state; none may commit it.
- Scenes are derived from persistent occupancy, topology, perception, and current events. Leaving a location does not delete it.
- RAG evidence remains owned by the Vault provider. For Aetheria, Ghostlight consumes VoidBot’s canonical retrieval service and stores only exact evidence receipts used by each campaign—never a rival semantic index.
- Every campaign branches from canon at creation. Later historical records describe the baseline trajectory, not an outcome the player must be railroaded into.
- Improvised facts are explicitly branch-local. Broadly useful gaps become reviewable canon candidates, never silent Vault edits.
- Player text describes an attempt, not a completed fact. Impossible actions do not receive a roll.
- Model failures, malformed outputs, stale projections, or retrieval outages produce no partial world mutation.

## Implementation changes

### Runtime and persistence

- Add a Rust 2024 workspace using Tokio/Axum, the Rust CultCache/redb backend, CultMesh/CultNet publication, and a provider-neutral model port.
- Store service state under `F:\GameCult\GhostlightDungeon`:
  - service/control and schema catalog `.cc`;
  - one row-oriented `.cc` store per campaign;
  - exact Vault evidence receipts;
  - private model-stage records and commit receipts;
  - exported campaign and canon-candidate bundles.
- Route all writers through an in-process campaign mailbox. Background ticks and HTTP commands cannot race or repair each other afterward.
- Publish the player surface, operator inspector, health, scheduler pressure, model receipts, and command results through CultMesh/Eve. HTTP health remains a probe of that state, not separate truth.

### Vault and world compiler

- Define a generic `VaultProvider` contract for source search, surrounding context, source witnesses, authority lanes, temporal scope, and exact-document retrieval.
- Implement the bundled Aetheria adapter against VoidBot’s streamable MCP endpoint at Starfire loopback.
- Add a hosted custom-Vault path for Git-synchronized, Obsidian-compatible Markdown hierarchies after tenant-isolation and import-security gates are met. The Vault service owns indexing; campaigns store only manifest bindings and exact evidence receipts. Provisional plans include one active Vault and 10 million indexed source tokens for Contributor/Private, or three active Vaults and 30 million combined tokens for Plus, with two or six full imports per month respectively.
- At campaign creation:
  1. Retrieve broad Aetheria evidence and generate three distinct suggested openings across different eras, places, and pressures.
  2. For the selected opening, generate three source-grounded player roles plus a custom-role path.
  3. Alternatively compile a fully custom who/where/when/goal description.
  4. Produce an approval preview containing the initial topology, cast, institutions, capabilities, clocks, evidence coverage, gaps, and branch-local assumptions.
  5. Commit only after approval.
- Materialize a bounded playable region and surrounding institutional pressure graph, not the entire universe. New destinations compile on demand and attach to stable containment, route, distance, and travel-time records.
- Automatically improvise reversible local texture as provisional branch fact. Material gaps in geography, mechanics, institutions, or extraordinary abilities require approval before the world seed or action is admitted.
- Store useful gaps as `CanonCandidate` records with originating campaign, evidence, conflicts, proposed wording, and affected Vault sources. The lab can review and export Markdown plus `.cc` evidence; it cannot edit AetheriaLore or open PRs.

### Persona and world action loop

Population scaling uses reversible gestalt materialization. A crowd, village,
crew, or corporation may act offscreen through one gestalt Persona. When a
specific member becomes relevant, the WorldKernel materializes an individual
from the gestalt baseline plus a persistent member delta and current scene.
When relevance expires, it folds individual consequences back into that delta,
admits only reviewed aggregate learning to the gestalt, and removes the active
simulation slot. The member delta remains durable so returning to the person
reconstructs the same individual rather than a fresh approximation.

Gestalt projection, relevance detection, and merge proposals cannot commit.
`MaterializeGestaltMember` and `DematerializeGestaltMember` use the normal
revisioned WorldCommand path. A named member's relationships, memories,
possessions, injuries, promises, and identity never become disposable gestalt
texture.

Live turns run a structured gestalt-relevance planner after the player event
and before NPC appraisal. Its atomic `ReconcileGestaltPresence` command can
materialize newly relevant members and fold irrelevant members back into their
persistent deltas. The planner may propose presence only; it cannot promote
population-wide learning or mutate state directly.

World compilation may seed collective populations and a bounded roster of durable
member identities, but that roster is not exhaustive. When an anonymous member
first becomes identity-bearing in play, the relevance planner may propose a new
member delta and the WorldKernel atomically creates that durable identity and its
active actor slot. Existing deltas are always preferred when they fit, preventing
the same person from being invented twice. Population-scale capabilities, knowledge, resources, goals,
and pressures belong to the gestalt. A member record owns identity, memories,
relationships, possessions, injuries, obligations, last known location, and
additions to or removals from the shared baseline. Materialization is a derived
composition of those two owners; the temporary actor slot owns neither source.
Dematerialization deletes only that slot after atomically updating the member
delta. Strategic ticks address the gestalt and its dematerialized deltas rather
than iterating every member as a live Persona.

Materialization grants a short revision-bound relevance lease. Speech, private
appraisal changes, and resolved actions refresh it through the WorldKernel.
Neither the model planner nor a direct command may dematerialize a member while
the player can still perceive them or before that lease expires. This prevents
quiet conversational beats from making people flicker between individual and
gestalt state.

Ghostlight owns its own generalized projection machinery:

1. Projector converts the actor’s permitted typed slice, memories, perceived events, relationships, goals, and retrieved knowledge into a private lived narrative stream.
2. Persona receives only that narrative stream and responds naturally—no schemas, action DSL, or raw state.
3. Interpreter extracts typed private deltas, explicit speech, reaction priority, and `WorldActionProposal` candidates.
4. WorldKernel validates spatial reach, knowledge, custody, resources, capability, opposition, and state version before committing anything.

All affected scene participants appraise a committed event in parallel. Actor-private deltas commit against their own snapshot versions; initiative then chooses compatible reactions or actions for world resolution. This ensures present characters continue perceiving and reacting even when another character currently holds conversational focus.

Canonical NPC consequence is serialized one opportunity at a time. The
deterministic initiative projection chooses the highest-priority stable winner;
`ResolveNpcAction` verifies that exact proposal and its same-revision assessment
against the committed wave, then consumes its siblings, obtains the server-side
d20, and applies the typed outcome in one atomic commit. A malformed assessment
therefore leaves no action-begun half-state. NPC resolution never resets
player-activity or away-time state.

Use DeepSeek’s current API directly:

- `deepseek-v4-flash`, non-thinking: Projectors, Interpreters, retrieval planning/reranking, verification, and offscreen actors.
- `deepseek-v4-pro`, non-thinking for live work: world compilation, action assessment, live Personas, and narration.
- Structured stages use JSON Output followed by local schema validation. Empty or malformed output receives one same-snapshot retry, then fails without mutation.
- Record provider, model, request hash, source receipts, latency, output hash, validation result, and state version. Never persist or display `reasoning_content`.

DeepSeek currently documents both V4 model IDs, streaming, and JSON output through its OpenAI-compatible Chat Completions API. [Models](https://api-docs.deepseek.com/quick_start/pricing), [Chat API](https://api-docs.deepseek.com/api/create-chat-completion), [JSON Output](https://api-docs.deepseek.com/guides/json_mode/).

### Fiction-first d20 resolution

- Provide separate `Assess` and `Attempt` commands. Assessment never becomes visible to NPCs or changes state.
- An assessment contains:
  - normalized intent and intended effect;
  - admissibility or the missing permission;
  - DC on the 5/10/15/20/25/30 resistance scale;
  - itemized contextual modifiers with state or evidence references;
  - modifier total capped at ±10;
  - effect ceiling;
  - success, mixed-result, and failure stakes;
  - campaign revision and expiring assessment digest.
- Contextual modifiers come from capabilities, knowledge, tools, access, assistance, leverage, position, opposition, injuries, and conditions. There are no base attributes.
- Confirmation checks the assessment revision, obtains a server-side OS-random d20, and atomically stores the roll receipt and world transition.
- Outcomes:
  - total ≥ DC + 10: strong success;
  - total ≥ DC: success;
  - total within 5 below DC: mixed result;
  - lower: failure with the previewed consequence.
- Natural 20/1 moves one outcome band, but cannot make an impossible action possible or exceed its effect ceiling.
- Stale assessments are recompiled rather than rebased.
- Overreach returns explicit sacrifices or bargains that could admit a new assessment.
- Speech and its intended effect are separated: the character may successfully say something while failing to persuade, deceive, or intimidate.

Every roll band carries a prevalidated `WorldEffectDelta`. The MVP vocabulary
can change local conditions and relationships, move the acting character over
an existing route, advance known clocks, or change a known institution's
posture. The chosen band and its delta commit atomically with the roll receipt;
stake prose alone never impersonates world mutation.

### Away-time world agency

- The daemon owns a five-minute scheduler pulse. Campaigns enter away simulation after fifteen minutes without player activity.
- Each real hour awards one strategic tick, capped at eight pending ticks per absence. The compiled campaign chooses the in-world tick duration; default is six hours.
- Background simulation affects remote actors, institutions, resources, movement, bargains, investigations, preparation, recruitment, obstruction, and clocks. The absent player is never puppeted or directly harmed.
- Each due tick derives a budgeted simulation-cell cover and runs one private
  Projector → Persona → Interpreter pipeline per cell against the same campaign
  revision and resolution epoch. The WorldKernel alone converts admitted typed
  effects into a `StrategicTickPlan`; proposal prose cannot become event truth.
  It accepts only known institutions, gestalts, members, and non-player actors;
  direct-route movement must fit inside the tick duration, information channels
  are bounded per constituent, and population pressure cannot silently become
  canon knowledge. Gestalts may also emit typed preparation, coordination,
  investigation, recruitment, obstruction, trade, or communication attempts
  against subjects with an explicit agency relation or exact shared location.
  A capped salient dormant member may be addressed by durable ID without being
  absorbed into a population voice. These commit only an attributed attempted-
  activity event; success and target response require later admitted effects.
  Durable dematerialized members use the same bounded activity vocabulary with
  exact personal attribution; ordinary action and migration share one per-
  person slot. Invalid, stale, timed-out, or malformed waves leave world state
  unchanged.
- The scheduler and return-time catch-up invoke the same `AdvanceStrategicTick` command. Catch-up processes missed ticks before accepting the next player action.
- Live play has inference priority; background work stops launching new calls while a player turn is active.
- Offscreen events generate information-channel-aware news leads. On return, the player may receive newspapers, reports, messages, or rumors only when the character has access to them; the news layer cannot reveal omniscient state.

### Multiresolution gestalt and institution agency

- Campaign creation compiles a global agency skeleton across every non-player
  actor, institution, and active gestalt leaf. All six partition facets are
  explicit; remote powers do not require eager local geography.
- `ActorState`, `InstitutionState`, gestalt baselines, member deltas,
  relationships, possessions, and knowledge remain canonical. A
  `simulation_cell.v1` is a derived connected cover and may be discarded.
- The player chooses 1–32 active Persona cells, default 8. Foreground subjects,
  active leases, initiative holders, explicit targets, and individual-detail
  pins may create a reported temporary overage.
- Adjacent cells merge by the weighted loss specified in
  `ghostlight-multiresolution-agency.md`; cohesive cells require real common
  authority, while opposed or cross-faction cells are arenas with no synthetic
  actor or shared knowledge.
- Every active cell receives one private Projector → Persona → Interpreter
  pipeline, followed by a semantic effect verifier when the cell proposes an
  action. Arena output is constituent-attributed. The verifier proves that the
  typed effect represents the Persona's actual choice rather than a reversed or
  lossy interpretation. All cells, receipts, and effects validate against one
  world revision and resolution epoch before one atomic strategic commit.
- Persistent detail debt rotates direct attention through quiet subjects. All
  clocks and deterministic obligations advance regardless of selected detail.
- Population fission is a separate approval-gated canonical operation. It
  creates enumerated child leaves plus `other/unknown`, preserves the parent as
  lineage, and reassigns durable member deltas without rewriting identity.
- Presence planning sees only active nearby leaves and people whose exact
  dormant location matches the player. Inactive ancestors and remote rosters
  stay out of the prompt. The WorldKernel rejects inactive-leaf individuation,
  location teleportation, and model-driven promotion outside the active scene.
- Provider request parallelism is an operator control with its own epoch. It
  batches the selected cell pipelines and cannot change the cover or fictional
  state.

### Web laboratory and hosting

- Build the interface as a Ghostlight-owned `gamecult.eve.surface.v1` document and lower it with Eve’s pinned browser renderer.
- Provide:
  - generated-opening and custom-start compiler;
  - narrative transcript and composer;
  - Assess, Attempt, confirm-roll, revise, and Wait controls;
  - character ledger for capabilities, equipment, conditions, obligations, relationships, and known facts;
  - location/time/pressure display;
  - news and rumor drawer;
  - named campaign creation, fork, reset, and `.cc` export;
  - optional operator inspector for evidence, topology, state versions, model receipts, rejected proposals, and private spoiler traces.
- Do not expose model chain-of-thought or secrets.
- Serve the embedded web bundle on TCP `8831`. Firewall access is limited to Starfire’s private LAN and `10.77.0.0/24`; no public listener or Yggdrasil web gateway is added.
- Use provisioned single-use invite links that establish separate HttpOnly sessions. Campaigns are single-player; simultaneous co-op is outside the MVP.
- Run `GhostlightDungeon` as a normal detached process under the Starfire operator account. Record its PID, executable, and logs; explicit start, stop, health, release, and rollback scripts own lifecycle. ACL the runtime root to administrators and that operator account.
- Read the DeepSeek key from stdin during setup, protect it with machine-scoped DPAPI plus file ACLs, and never place it in source, arguments, logs, or environment projections.
- Build immutable release directories from the exact Git commit, switch the active release through the guarded junction, and record provenance in CultCache plus the immutable release manifest. Process restart is explicit; Windows service installation and automatic boot recovery are outside this MVP by operator choice.

### Repository and operational documentation

- Implement on `codex/ghostlight-dungeon-mvp`, with coherent foundation, runtime, interface, and deployment commits pushed to GitHub.
- Add the authoritative GhostlightDungeon architecture/build document and update Ghostlight’s system map, implementation plan, handoff, state map, and evidence ledger.
- Keep Starfire service/firewall/tunnel ownership in `gamecult-ops`, with a deployment and rollback runbook.
- Before live VoidBot integration, resolve the current Yggdrasil inventory/DNS split and independently verify the offered SSH host fingerprint. Then repair the tunnel task’s stale `E:\Projects` path to `F:\Projects`, restore `127.0.0.1:17875/mcp`, and update ops truth. Never bypass host-key verification or revive Starfire’s retired VoidBot/Qdrant writers.

## Public contracts

- `ghostlight.vault_manifest.v1`, `vault_evidence_receipt.v1`, `world_compile_preview.v1`
- `campaign.v1`, `world_fact.v1`, `location.v1`, `actor_state.v1`, `institution_state.v1`, `relationship_state.v1`, `world_clock.v1`, `event.v1`
- `player_action_assessment.v1`, `roll_receipt.v1`, `world_commit_receipt.v1`
- `persona_stage_receipt.v1`, `actor_state_delta.v1`, `world_action_proposal.v1`
- `narration_projection.v1`
- `strategic_tick.v1`, `news_issue.v1`, `canon_candidate.v1`
- `rejected_proposal_receipt.v1`, `campaign_lifecycle_receipt.v1`
- `gestalt_persona_state.v1`, `gestalt_member_delta.v1`,
  `gestalt_materialization_receipt.v1`
- Existing `gamecult.eve.surface.v1` and `gamecult.eve.command.v1`

Publish JSON Schema only as the schema catalog. Runtime documents and exports use MessagePack-backed CultCache `.cc`; JSON exists only at browser, MCP, and model-provider boundaries.

## Test and acceptance plan

- Unit-test every schema, redb/CultCache round trip, optimistic commit, topology invariant, knowledge gate, DC/modifier rule, outcome band, roll receipt, branch fact, and scheduler budget.
- Use injected clocks, RNG, Vault provider, DeepSeek transport, and persistence ports.
- Prove malformed/empty DeepSeek JSON, timeout, retrieval loss, daemon restart, stale assessment, stale Persona output, and concurrent scheduler/player commands cannot partially mutate state.
- Run existing Pallas, Lucent, and Corvid fixtures as regression scenarios, even though hosted opening suggestions are generated.
- Acceptance scenarios:
  - compile an obscure Aetheria time/place or return an honest evidence-gap preview;
  - leave a location, advance the world, and return to the same geometry and inhabitants;
  - approach a gestalt population, automatically materialize a durable member,
    form a member-specific relationship, leave until the relevance lease expires,
    fold the active actor into its persistent delta, tick the population once as a
    gestalt, and later rematerialize the same person with that relationship intact;
  - show reactions from every affected present actor;
  - prevent an NPC from acquiring unearned knowledge or expertise;
  - reject impossible action claims and offer explicit bargains;
  - preview and confirm a fully receipted d20 action;
  - observe institutions exploiting, resisting, or redirecting the protagonist;
  - produce material offscreen changes and accessible news after absence;
  - reload/fork/export a campaign without losing continuity;
  - verify two invite sessions and rejection of unauthenticated access.
- Performance targets under a healthy provider:
  - first progress update within 300 ms;
  - all actor stages execute in parallel waves;
  - a four-actor live turn completes within 20 seconds;
  - background inference yields immediately to live work.
- Final deployment smoke verifies explicit process restart, LAN and WireGuard access, blocked public access, DeepSeek inference, VoidBot-grounded retrieval, scheduler continuation, state persistence, CultMesh/Eve publication, and exact deployed commit provenance.

## Assumptions and exclusions

- Aetheria is the only bundled Vault adapter, but no compiler logic is Aetheria-specific.
- Generated suggestions plus unrestricted custom compilation replace fixed featured openings.
- No D&D classes, spell slots, six-ability statistics, or 5e sheet are canonical in the MVP. The typed character ledger remains sufficient for later D&D-shaped projections.
- No arbitrary Vault upload/import UI in the current MVP; Git-synchronized Obsidian-compatible tenant Vaults are a documented post-gate product intention. No multiplayer campaign, voice, image generation, or direct canon mutation in the MVP. Multiplayer is an intended extension of the existing one-kernel/many-actors authority model and is documented separately; it does not expand this acceptance plan.
- The live deployment requires a DeepSeek key and restoration of the trusted VoidBot retrieval crossing; fixture-backed development may proceed before those two setup gates.
