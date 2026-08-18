# Ghostlight Implementation Plan

## Current Phase

GhostlightDungeon is now the active implementation program. Its authoritative
architecture and ownership map is
`docs/architecture/ghostlight-dungeon-mvp.md`. The existing data-generation
pipeline and fixtures remain regression evidence; they do not become hosted
runtime authorities.

The Epiphany Projector → Persona → Interpreter extraction and handback are
complete. Ghostlight owns the generalized projection membrane in the
`ghostlight-persona-projection` crate. Epiphany consumes an exact Ghostlight Git
revision and now gives Persona only one lived narrative stream; Epiphany keeps
her canonical Persona mind, receipts, effect admission, brakes, and external
consequence authority.

The hosted runtime owns compiler approval, exact VoidBot evidence, campaign
mailboxes, fiction-first d20 resolution, parallel Persona appraisal, narration,
reversible gestalt materialization, isolated sessions/campaigns, CultMesh/Eve
publication, and away-time scheduling. Strategic ticks now project six-axis
pressure, select a connected budgeted cover over the global agency skeleton,
and run one private Ghostlight Persona membrane per cohesive or arena cell. The
WorldKernel admits the whole cell wave atomically and forbids player puppeting,
synthetic arena actors, borrowed secrets, invented IDs, unreachable movement,
and unbounded information or population-pressure edits. The laboratory exposes
a 1–32 player budget, persistent pins, approval-gated gestalt fission, graph and
cover receipts, and a separate operator provider-concurrency limit. The
specific authority map is
`docs/architecture/ghostlight-multiresolution-agency.md`.
The accepted implementation is hosted on Starfire as immutable release
`f1d55b181ad60ace8e8e92859c4d0b2f1a94fc87`. Exact checkout, embedded commit,
release manifest, binary hash, process, typed health, CultMesh store, persisted
campaign, DeepSeek startup inference, VoidBot retrieval, private-LAN access,
WireGuard access, and unauthenticated rejection agree. This pass makes model
work inspectable and cheaper:
provider attempts expose prompt/completion/cache usage and exact local failures;
stable prefixes precede dynamic context; world and agency compilation are
separate stages; Projectors receive situation state while Interpreters receive
exact permissions; deterministic cell bindings are attached by the runtime.
Remote agency compilation is a separate non-causal Flash lane: exact mandate
strings are bound to witnesses locally, rejected candidates become coverage
gaps, and deterministic coarse profiles keep unknown facets unknown. The Pro
agency stage no longer spends output tokens restating remote profiles.

Strategic activity outcomes now have their own authority between attempt
selection and commit. One batched Flash resolver receives only selected
digest-bound attempts plus exact legal consequence handles. It emits one
bounded typed result per activity; local validation rejects invented custody,
relations, facts, pressure resolutions, member owners, and player mutation.
WorldKernel requires the action-set-bound stage receipt and applies every
outcome with the rest of the wave on one private campaign copy. Accepted
outcomes are stored in CultCache and shown through the operator Eve surface.

The current browser/compiler hardening adds three ownership cuts before the
next isolated acceptance run: compiler evidence is triaged into direct,
background, and excluded lanes before generation, with only direct source text
admitted to the causal world seed; informational action effects
must name the exact finding they commit, which the runtime binds into the
player-visible stake before validation; and player HTTP command
responses are spoiler-safe projections rather than serialized campaigns. The
browser renders these projections as labeled, escaped human-readable controls,
and narration receives only the latest causal turn rather than a route dump.

Live local evidence now covers grounded VoidBot compilation, a four-actor wave,
impossible and receipted d20 actions, grounded narration, strategic cells, and a
24-faction budget-4 wave. The first scale baseline used 37,327 prompt tokens and
17.81 seconds. After the authority/context cuts, the same complete four-cell
cover used 20,760 prompt tokens and 14.38 seconds, with 13 of 13 stages valid on
their first attempt, two material institutional consequences, exact constituent
authority, and no player mutation. Authenticated surface and lifecycle
acceptance now cover spoiler-safe literal HTTP responses, separate invite
ownership, single-use invite rejection, fork/reset/export, four persisted
campaigns, and exact-commit restart. Live priority cancels in-flight scheduler
inference and excludes its commit gate.

The revised provider-backed Gestalt golden now passes across four waves and
budget changes `1 -> 4 -> 8 -> 4`. It committed ten durable consequences,
including seven typed material activity outcomes, while preserving exact
member binding, the absent player, and Mira's persistent identity. All four
pulses committed without retry. The run used 126,826 prompt tokens with a
58.34% cache-hit ratio and 14,477 completion tokens. The harness now requires a
typed material outcome; unrelated direct consequences can no longer hide an
inert outcome resolver.

The next gate is release truth: commit and publish this exact candidate, deploy
an immutable optimized build to Starfire, and repeat authenticated browser and
restart checks against the served commit. Then begin the two-tester playtest.
Treat player confusion, dramatic flatness, composite consequences, and
token-heavy low-consequence stages as evidence about missing context or
misplaced authority. Keep the independent semantic outcome verifier selective:
it checks custody transfers, relation shifts, and named-member private deltas,
while deterministic local validation owns ordinary bounded preparations and
pressure changes.

The selective verifier now has a forced live custody witness. An exact
population-owned resource moved to one exact colocated recipient only after a
separate Flash verdict accepted the causal match; the source campaign snapshot
remained unchanged during resolution. The witness also removed one avoidable
provider retry by making the `resource_recipient_ids` input to
`other_subject_id` output mapping explicit. The remaining release gate is the
authenticated browser journey after rebuilding and deploying this exact commit.

Multiplayer remains outside the current paid-alpha gate but is now an explicit
architectural intention. Do not fork the simulation into a multiplayer room
model. After the solo product and Gestalt agency are proven, extend campaign
membership so authenticated humans control distinct canonical actors through
the existing mailbox, perception gates, initiative, and atomic commit path. The
authority map and unresolved consent/governance questions live in
`docs/architecture/ghostlight-dungeon-multiplayer-intention.md`.

## Prior research-program phase

Build a reliable data-generation loop for socially persistent Aetheria agents.
The immediate target is not a full simulation. It is a clean, reviewed,
sandboxed training-data pipeline for branching scenes and state consequences.
The output should preserve Aetheria's tonal range: wit with stakes, ordinary
life before interruption, quiet ritual, domestic warmth, weirdness, dread, and
systems pressure as needed.

Planning belongs in this file and the handoff/map state surfaces. Architecture
docs should describe durable contracts, boundaries, and data shapes; they should
not carry live next-action lists.

## Near-Term Sequence

1. Stabilize canonical state and projection seams.
   - Keep `schemas/agent-state.schema.json` and required vectors coherent.
   - Keep canonical state separate from perceived overlays.
   - Keep voice, presentation, relationship, memory, and situational pressure explainable.
   - Add or preserve tonal-mode and ordinary-life cues where they affect prose.
   - Do not leak raw numeric state internals into responder prompt text.

2. Stabilize responder packet and output seams.
   - Use `schemas/responder-packet.schema.json` and `schemas/responder-output.schema.json`.
   - Build packets from coordinator artifacts plus projected local context.
   - Preserve exact responder-visible input, hidden-context audit, allowed actions, source excerpts, output contract, and lore-access mode.
   - Preserve raw output, parsed output, review labels, consulted refs, research summary, leakage audit, and coordinator interventions.
   - Require `runner_captured` research trace status for accepted research-enabled gold data; coordinator-reconstructed trace is useful draft audit, not proof of the responder's actual research path.

3. Stabilize scene-loop receipt bundles.
   - Use `schemas/event-record.schema.json`,
     `schemas/participant-appraisal.schema.json`,
     `schemas/reviewed-mutation.schema.json`, and
     `schemas/scene-loop-bundle.schema.json`.
   - Validate with `npm run training-loop:validate`, included in
     `npm run schema:validate`.
   - A training-ready scene loop preserves scene-local digest, initial state,
     coordinator artifacts, projected contexts, responder packets, raw responder
     outputs, event records, participant-local appraisals, reviewed mutations,
     and bundle-level training-usability labels.
   - Training readiness is per organ. A receipt-complete scene loop does not
     automatically count as branch compiler, IF reviewer, visual artifact, or
     accepted full-fixture coverage.
   - Adopt VoidBot's heartbeat routine as the cast initiative substrate:
     characters take scene turns through heartbeat initiative, cooldown starts
     after turn completion, and offstage agents use slower rumination/sleep
     turns for memory consolidation instead of generating fake scene motion.

4. Keep the Pallas fixture as the current scaffold reference.
   - Current fixture set:
     `examples/lore-grounding/pallas-species-strikes.awakened-labor.v0.json`,
     `examples/agent-state.pallas-species-strikes.v0.json`,
     `examples/coordinator/pallas-species-strikes.branch-and-fold.v0.json`,
     `examples/ink/pallas-species-strikes.branch-and-fold.v0.ink`,
     `examples/ink/pallas-species-strikes.branch-and-fold.v0.training.json`,
     `examples/visual/pallas-species-strikes.branch-and-fold.v0.visual.json`,
     and `experiments/pallas-species-strikes/` receipts.
   - Treat it as reference-only story-shape and grounding material, not training-ready data for any soft organ.
   - Preserve the lessons it established: ordinary-life onboarding, branch-and-fold discipline, material state variables, source-backed lore elaboration, visual segmentation, scene-set need for illustrated replay, and review before acceptance.
   - `pallas-training-loop-v0` is the first separate training-shaped derivative:
     it rebuilds the Kappa refusal / Ilya arrival threshold beat through three
     exact no-fork responder turns plus event, appraisal, mutation, and bundle
     receipts. Keep it separate from the immutable reference fixture.

5. Generalize the loop.
   - Use the corpus coverage ledger so future fixtures can be tracked by faction, movement, flashpoint, tonal mode, training target, review status, and cultural collisions.
   - Select the next Aetheria fixture from the 100-150 broad coverage target.
   - Add more historical grounded fixtures from AetheriaLore.
   - Add future-branch Elysium fixtures with branch lineage and constraint labels.
   - Track coverage across every major faction, minor faction, movement, and major flashpoint; major factions require both founding-era and day-in-the-life stories.
   - Require cultural collision in coverage stories so training data captures inter-faction dynamics, mutual misreads, and movement pressure.
   - Vary tonal modes across fixtures so the corpus does not train one flattened Aetheria voice.
   - Emit technology/item manifest deltas when scenes discover or stress gear, assemblies, supply chains, or faction tech bases.
   - Keep artifacts database-shaped enough for future game-engine integration.

5. Train only after schemas stop sliding.
   - Pilot samples are schema shakedown, not robust corpus scale.
   - Specialized future models include coordinator, retriever, projector,
     responder, appraiser, mutator, relationship/perception updater, branch
     compiler, IF artifact reviewer, evaluator, and institution/faction/consumer
     decision models.
   - Deterministic gates remain code: visibility, action legality, object
     custody, resource accounting, schema validation, source provenance,
     mutation authority, prompt leakage checks, and Ink compilation.

## Deferred from the earlier fixture program

- Full world simulation loop
- Economy simulation loop
- Long-horizon plot generation without author scaffolding
- Fine-tuning before artifact schemas, review criteria, and evaluators stabilize

## Discipline

- Work one bounded organ at a time.
- Prefer explicit maps and contracts over implicit context.
- If the diff grows while understanding shrinks, stop and simplify.
- Keep history in git and distilled evidence; keep the live workspace focused on the mission.
