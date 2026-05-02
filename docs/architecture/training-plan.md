# Ghostlight Training Plan

This document enumerates the trainable stages in Ghostlight so data generation
stays aimed at the machine we are building instead of becoming a heap of
interesting corpses.

The rule is simple: deterministic code owns legality, visibility, schemas,
resource math, source provenance, Ink compilation, and mutation gates. Models
handle judgment, language, social inference, prioritization, and coordination.
Every model-facing judgment should leave a reviewed artifact.

Ghostlight is also the training ground for VoidBot's social intelligence. The
appraiser and relationship/perception organs must remain useful for reading real
human interaction over time: forming grounded opinions, noticing uncertainty and
misread risk, tracking trust, irritation, concern, respect, boundaries, and
expectations, and revising those impressions when later evidence contradicts
the first read. If these organs only work for fictional NPC scene mutation, they
have missed one of the original reasons the project exists.

## Timeline Coverage

Ghostlight training should cover three timeline lanes:

- `historical_grounded`: authored late-Sol and pre-Elysium history, used for
  primary grounded social pressure.
- `transition_grounded`: the rupture, displacement, and early Elysium shock,
  used for continuity between old institutions and altered conditions.
- `future_branch`: possible post-Rupture Elysium futures, used to teach models
  Aetheria's stranger concepts as branch-local canonical histories.

The future branch lane is not optional. The models need to handle post-Rupture
concepts such as Aether, pseudospace, temporal nonlinearity, spirits, necrotech,
mutable bodies, altered substrates, and new species or institutional forms. Late
Sol teaches the old damage. Elysium teaches what the damage does when reality
starts handing out knives with metaphysics on the receipt.

Sol timeline data is a single-history artifact. Elysium timeline data is
conditional canon: every possible future is true inside the branch whose prior
state produced it. Training data must preserve branch lineage so the model
learns conditional continuity instead of mashing incompatible futures into one
omnivorous sludge.

Future-branch examples must label:

- source constraints from authored post-Elysium concepts
- generated branch assumptions
- local branch facts
- branch lineage and prior conditions
- sibling-branch exclusion boundaries
- branch-local facts that must not leak into single-history Sol training or
  unrelated Elysium branches
- event constraint type: `branch_attractor`, `fated_event`,
  `tech_order_constraint`, `quest_injection`, or `branch_local_event`
- prerequisites, exclusions, and pull strength for constrained events
- the post-Rupture concept being exercised
- the old social pressure being re-expressed

## Lore Coverage Matrix

Cold Wake is a fixture, not a corpus. A complete Aetheria training corpus needs
coverage across the setting's institutional ecology before the models can be
trusted to carry tone, conflict, and cultural priors without constant hand
feeding.

Before calling the corpus complete, generate reviewed story/scenario coverage
for:

- every major faction
- every minor faction
- every movement and counter-project
- every major historical flashpoint
- every major post-Rupture concept or branch-attractor family used in gameplay
- faction-adjacent institutions that strongly shape daily life, trade, law,
  care, warfare, media, faith, logistics, or embodiment

Major factions need at least two distinct story types:

- `founding_era_story`: the faction's formative pressure, first stable myth,
  early institutional compromise, or founding betrayal.
- `day_in_the_life_story`: ordinary operating texture after the faction has
  matured, showing how people actually live inside its systems.

Minor factions, movements, and flashpoints need at least one strong story or
branching-scene fixture each, plus negative or rejected examples where the first
attempt collapses into stereotype, lore drift, or generic sci-fi wallpaper.

Every accepted coverage story should involve cultural collision. Do not write
sealed terrariums where one faction monologues at itself and calls that
worldbuilding. The useful data comes from contact:

- major faction versus major faction
- major faction versus minor faction
- faction versus movement
- movement versus movement
- institution versus care network
- local colony versus imported authority
- species/body affordance versus human-default infrastructure
- technical standard versus ritual, law, class, scarcity, or dignity

Coverage records should tag:

- primary faction or movement
- secondary and tertiary factions or movements
- flashpoint or era
- story type: `founding_era_story`, `day_in_the_life_story`,
  `flashpoint_story`, `movement_story`, `future_branch_story`, or
  `trade_life_story`
- collision axis: law, care, class, logistics, species/body, labor, faith,
  media, war, trade, personhood, ecology, memory, technology, or legitimacy
- involved cultures and their mutual misreads
- item/economy hooks discovered, including colony-consumed trade goods
- source refs and open lore gaps
- accepted, rejected, or needs-revision review status

The first pass can be shallow and uneven. The final corpus cannot be. If one
flashpoint starts producing all the examples, stop and rebalance before the
model learns that Aetheria is mostly one corridor argument wearing different
coats.

Some Elysium events are not globally fixed, but they are not equally optional
either. Founding events for Elysium-born factions, technology discoveries,
metaphysical practices, and quest hooks should be represented as attractors,
rails, or injections when appropriate.

- `branch_attractor`: likely under recurring pressures, with local variation.
- `fated_event`: authorially constrained to occur unless the branch disables it.
- `tech_order_constraint`: locked behind prerequisite discoveries or systems.
- `quest_injection`: authored content inserted into compatible branch points.
- `branch_local_event`: true only inside its lineage, with no cross-branch pull.

Future exploration must also emit technology and item knowledge when a branch
discovers or changes material capability. See
`docs/architecture/technology-item-manifest-plan.md` for the manifest seam. That
seam targets the existing Aetheria-Economy CultCache model: Ghostlight proposes
reviewed candidate `ItemData` technological blueprints such as
`SimpleCommodityData`, `CompoundCommodityData`, `ConsumableItemData`, `GearData`,
and specialized `EquippableItemData` subclasses, while runtime `ItemInstance`
objects belong to scene, inventory, cargo, provenance, branding, and world-state
simulation.

`CompoundCommodityData` candidates are not limited to gear assemblies or
manufacturing inputs. They also cover finished manufactured trade goods consumed
by colonies, habitats, institutions, ships, clinics, species communities, and
faction enclaves. A generated detail like a Navigator wet-interface cradle is
therefore both world texture and a possible colony demand item, not just prose
decoration.

## Runtime Stack

Target runtime flow:

```text
world state + author constraints + lore grounding
  -> coordinator/story runtime
  -> deterministic state slice and source checks
  -> projector
  -> sandboxed character agent/responder
  -> event resolver and deterministic gates
  -> participant appraiser
  -> state mutator
  -> relationship/perception updater
  -> coordinator carries hooks, glue prose, and next beat
  -> branch compiler materializes branch-and-fold Ink plus sidecar receipts
  -> IF artifact reviewer accepts, rejects, or sends the fixture back
```

Some stages are trained as generative models. Some are trained as classifiers or
rankers. Some may remain manual longer than the rest because bad automation here
would create narrative sewage with confidence.

## Corpus Scale Tiers

The per-stage counts below are pilot gates, not robust training targets.

Pilot data exists to shake out schema mistakes, artifact shape problems,
leakage boundaries, reviewer workflow, and validator gaps. It is allowed to be
sacrificial. If a pilot proves that the schema is wrong, update the schema and
accept that some pilot artifacts may become obsolete. Better a small controlled
burn than a cathedral built from cursed bricks.

Use three corpus tiers:

- `pilot_schema_shakedown`
  - proves that artifacts can be produced, reviewed, validated, and loaded
  - exposes missing fields, wrong boundaries, bad labels, and impossible
    assumptions
  - may be invalidated by schema changes
- `review_assistant_target`
  - enough examples for a model to draft useful candidates under human or
    frontier-model review
  - still not trusted for unattended runtime mutation or simulation authority
  - expected to include broad negative examples and repair labels
- `runtime_target`
  - enough diversity, sequence coverage, negative coverage, and adversarial
    checks to trust a model inside automated loops with deterministic gates
  - requires held-out evaluation sets, regression fixtures, and failure audits
  - still cannot bypass deterministic legality, visibility, source, or mutation
    gates

Approximate target scale:

| Stage | Pilot Schema Shakedown | Review Assistant Target | Runtime Target |
| --- | ---: | ---: | ---: |
| Lore grounding compiler | 25 accepted / 25 rejected | 500+ digests | 2,000+ digests |
| Coordinator/story runtime | 50 accepted / 25 rejected | 1,000+ turns | 5,000+ turns |
| Memory/lore retriever | 500 labeled pairs | 5,000+ pairs | 50,000+ pairs |
| Projector | 100 accepted / 50 rejected | 1,000+ examples | 5,000+ examples |
| Character agent/responder | 200 accepted / 100 rejected | 2,000-5,000 turns | 10,000+ turns |
| Event resolver normalization | 100 reviewed examples | 1,000+ examples | 5,000+ examples |
| Participant appraiser | 300 appraisals | 3,000+ appraisals | 10,000+ appraisals |
| State mutator | 500 accepted / 200 rejected | 5,000+ patches | 10,000+ patches |
| Relationship/perception updater | 500 updates | 5,000+ updates | 20,000+ sequence updates |
| Branch compiler | 25 fixtures / 25 rejected | 500 fixtures | 2,000+ fixtures |
| IF artifact reviewer | 100 reviewed fixtures / 100 labeled failures | 2,000+ labeled fixtures | 10,000+ labeled fixtures |
| Guardrail/evaluator classifiers | 300 labeled failures | 3,000+ labeled outputs | 20,000+ labeled outputs |
| Institution/faction/consumer decisions | 500 decisions | 5,000+ decisions | 20,000+ decisions |
| Technology/item manifests | 100 records / 50 rejected | 1,000+ manifests | 5,000+ manifests |

The exact numbers will move once we know which organs are narrow classifiers,
which are generative, which can lean on retrieval, and which are mostly
deterministic code wearing a little judgment hat. The important rule is that the
first gate proves the schema and workflow. It does not prove robustness.

## Model Families

Use these as the default architecture assumptions until evidence says otherwise.

### Generative Decoder LLM

Use for stages that must produce structured JSON plus prose or action text.

Likely training method:

- supervised fine-tuning
- LoRA or QLoRA for local experiments
- full fine-tune only after corpus shape stabilizes and the model deserves the
  electricity

Candidate stages:

- coordinator/story runtime
- projector, if deterministic projection becomes too brittle
- character agent/responder
- branch compiler, if rule-assisted Ink materialization is too narrow for
  authored fixture variety
- state mutator, only after the mutation schema is very stable
- institution/faction planner, if it needs explanations and mixed outputs
- item or technology manifest generator, if structured extraction from
  coordinator exploration becomes too broad for deterministic templates

### Classifier Or Cross-Encoder

Use for stages that score or label an event, branch, or output.

Likely training method:

- supervised classification
- multi-label classification
- pairwise ranking when choosing between candidates
- calibrated confidence where downstream decisions need thresholds

Candidate stages:

- participant appraiser
- relationship/perception updater
- output evaluator
- IF artifact reviewer
- candidate action ranker
- lore/source violation detector
- prompt-leak detector
- branch-consequence and fake-fold detector
- technology and item manifest evaluator

### Embedding Or Retrieval Model

Use for selecting relevant memories, lore, prior events, and source snippets.

Likely training method:

- start with off-the-shelf embeddings
- add contrastive or triplet tuning only after enough positive and negative
  retrieval examples exist

Candidate stages:

- memory retrieval
- lore retrieval
- precedent retrieval for coordinator planning
- technology prerequisite and component dependency retrieval

### Deterministic Code

Keep as code, not a model:

- schema validation
- action enum validation
- visibility filtering
- object custody
- resource accounting
- mutation authority
- source provenance requirements
- Ink compilation
- artifact status validation
- world-state patch application
- item manifest validation and compatibility checks

## Trainable Stages

### 1. Lore Grounding Compiler

Purpose: turn Aetheria source material into scene-local grounding digests.

Current implementation:

- mostly manual plus frontier-model assistance
- validated by `schemas/lore-grounding-digest.schema.json`

Inputs:

- AetheriaLore source paths and excerpts
- target time period
- target location or institution
- involved factions, species, roles, and social movements
- author constraints
- fixture lane: `historical_grounded`, `transition_grounded`, or
  `future_branch`

Outputs:

- lore grounding digest
- cultural pressure fields
- factional misread maps
- material constraints
- post-Rupture concept constraints, when applicable
- speaker knowledge boundaries
- author-only exclusions
- open lore gaps
- generated branch assumptions, for future branches
- branch lineage and sibling-branch exclusion boundaries

Training architecture:

- generative decoder LLM for digest drafting
- classifier/evaluator for source-grounding failures
- retrieval model for source passage selection later

Training artifacts:

- accepted digests
- rejected digests
- source-gap records
- source violation labels
- corrections tied to AetheriaLore paths

Pilot schema shakedown gate:

- 25 accepted digests across distinct Aetheria eras or institutions
- 25 rejected/source-violation examples
- evaluator catches unsupported room-scale assumptions
- at least 5 future-branch digests exercising authored post-Elysium concepts
  before training a responder expected to handle Elysium weirdness

### 2. Coordinator Or Story Runtime

Purpose: decide what happens next at the scene/story level and keep the whole
machine pointed at usable game content.

Current implementation:

- Codex/frontier coordinator in the loop
- authored glue prose
- manual world-state continuity

Inputs:

- game-engine-shaped world state
- scene state
- event history
- unresolved hooks
- active branch flags
- author constraints
- lore grounding digest refs
- available deterministic tools
- available soft organs
- prior reviewer notes
- fixture lane and canon status
- post-Rupture concept constraints for future branches
- active attractors, fated events, tech-order gates, and quest-injection hooks
- current technology and item manifest refs
- faction tech-base refs

Outputs:

- next beat or next scene plan
- selected acting agent
- tool/model invocation plan
- world-state refs read
- proposed world-state changes
- branch merge/split decision
- unresolved hooks carried forward
- glue prose
- reviewer-facing rationale
- generated branch assumptions and promotion candidates, for future branches
- branch lineage and conditional truth boundaries
- selected attractor, fate, technology, or quest constraints that shape the next
  beat
- item, component, assembly, supply-chain, or faction tech-base deltas implied
  by the scene or branch decision
- colony-consumed manufactured goods and demand-profile deltas implied by
  habitats, institutions, species communities, interface needs, rituals, or
  living standards

Training architecture:

- generative decoder LLM with tool-use style output
- possible candidate-plan ranker later
- retrieval model for prior event/hook selection

Training artifacts:

- coordinator plan records
- accepted and rejected glue prose
- branch continuity decisions
- tool invocation traces
- world-state proposal receipts
- attractor/rail/injection selection receipts
- item-manifest proposal receipts
- faction tech-base delta receipts
- reviewer labels for pacing, continuity, legibility, game usefulness, and lore
  grounding

Pilot schema shakedown gate:

- 50 accepted coordinator turns across at least 5 scenes
- 25 rejected coordinator turns with labeled failures
- structured world-state refs present in every accepted example
- glue prose never treated as canonical state
- at least 10 accepted future-branch coordinator turns before using the
  coordinator to generate post-Rupture arcs without close review
- at least 20 labeled constrained-event examples before the coordinator is
  trusted to inject quests or advance technology order automatically
- at least 25 reviewed item/faction-tech deltas before coordinator exploration
  is allowed to create technology data without close review

### 3. Memory And Lore Retriever

Purpose: choose what memories, events, and lore snippets are relevant to a turn.

Current implementation:

- deterministic selection and manual source checks
- semantic source search through existing tooling when needed

Inputs:

- acting agent id
- scene id
- current pressure
- event history
- memory store
- lore grounding corpus
- query or coordinator intent
- fixture lane and canon status

Outputs:

- ranked memory refs
- ranked lore/source refs
- rejected irrelevant refs
- retrieval rationale or labels
- source-constraint versus branch-assumption labels when retrieving future
  branch context
- branch-lineage labels so sibling futures are not retrieved as if they were
  shared facts

Training architecture:

- embedding model or bi-encoder for recall
- cross-encoder reranker for precision
- start off-the-shelf; tune only after labels exist

Training artifacts:

- selected refs
- rejected refs
- reviewer relevance labels
- missed-critical-context labels

Pilot schema shakedown gate:

- 500 labeled retrieval pairs before tuning
- evaluation set includes false friends: similar vibe, wrong faction or era

### 4. Projector

Purpose: turn canonical state into character-local operating context.

Current implementation:

- deterministic `tools/project_local_context.py`
- projected context schema and validator

Inputs:

- canonical agent state
- perceived overlays
- relationship stance
- current scene state
- relevant memories
- lore grounding digest
- observed event, when responding
- projection controls
- branch/canon status and generated-assumption boundaries
- active technology gates and quest constraints

Outputs:

- known local facts
- active pressures
- tensions
- action affordances
- constraints
- voice surface cues
- rendered prompt/context text
- audit refs to source state
- explicit distinction between source-backed facts and branch-local assumptions
- branch lineage and conditional truth boundaries
- unavailable action or knowledge constraints derived from tech order

Training architecture:

- keep deterministic while feasible
- generative decoder LLM student if deterministic projection becomes too rigid
- classifier/evaluator for leakage and unsupported pressure

Training artifacts:

- projected-local-context records
- projection examples
- prompt leakage failures
- source grounding failures
- reviewer corrections

Pilot schema shakedown gate:

- 100 accepted projection examples
- 50 rejected examples with clear failure labels
- stable input slicer
- evaluator catches private-state leakage and raw-state soup
- evaluator catches future-branch assumptions leaking into grounded historical
  examples

### 5. Character Agent Or Responder

Purpose: choose and render what a character does next from local context.

Current implementation:

- Qwen/local model calls through structured tools
- manual review and capture receipts
- no-thinking strict calls by default for tool compliance
- future gold responder data should use isolated generation workers or
  sub-agents that receive only the projected local packet, not coordinator
  context

Inputs:

- projected local context
- allowed action primitives
- visible event, if responding
- local affordances
- character-local goals, memories, relationship read, and constraints
- Aetheria responder priors from training corpus, eventually
- post-Rupture concept constraints when the turn is in Elysium future branch
  territory
- explicit source excerpts included in the projected packet, if any

Forbidden inputs:

- coordinator rationale
- other agents' private state
- author-only plot beats
- future branch plan
- hidden canonical facts not visible to the acting character
- raw canonical state variables the final responder model will not receive
- repo or lore browsing beyond excerpts intentionally provided to the responder

Outputs:

- selected action type
- observable action
- spoken text, if any
- actor intent
- candidate alternatives
- constraints satisfied or violated
- response rationale
- raw responder output before coordinator review
- reviewed output after coordinator intervention, if any
- coordinator intervention labels

Training architecture:

- generative decoder LLM
- SFT/LoRA on reviewed Aetheria response/action corpus
- pairwise preference or ranking later for choosing among candidate turns
- optional separate action-type classifier if generation keeps drifting

Training artifacts:

- exact responder-visible input packet
- hidden context refs without hidden content
- sandbox or isolation method
- raw responder output
- coordinator-reviewed output
- coordinator intervention labels
- action-choice receipts
- accepted and rejected turns
- negative examples for wrong body, wrong lore, wrong voice, omniscience, prompt
  leakage, and agency collapse
- timeline-spread Aetheria response corpus
- future-branch responses exercising post-Rupture concepts as branch-local canon
  without claiming branch facts apply to sibling histories

Pilot schema shakedown gate:

- 200 accepted character turns across eras, factions, species, and social roles
- 100 rejected turns with labeled failures
- at least 50 non-speech or mixed-action examples
- evaluation includes speaker-local secret boundaries and body affordances
- every accepted example preserves exact responder-visible input and raw output
- every coordinator repair is labeled instead of being treated as raw responder
  behavior
- leakage evaluator catches outputs that require withheld context
- at least 50 accepted post-Rupture future-branch turns before expecting the
  responder to handle Elysium-specific weirdness with reduced lore handholding

### 6. Event Resolver

Purpose: convert an action proposal into an observable event and mechanical world
changes.

Current implementation:

- mostly manual/reviewed for social consequences
- deterministic gates should own mechanical legality

Inputs:

- action proposal
- scene state
- world objects
- resources
- authority rules
- location constraints
- participant visibility
- branch/canon status for future generated facts
- technology and quest gates relevant to action legality

Outputs:

- observable event record
- legal mechanical world deltas
- perceived-by participant list
- blocked or modified action result
- unresolved facts
- branch-local mechanical facts versus promoted facts
- branch-local mechanical facts versus cross-branch source constraints
- blocked actions caused by missing prerequisites

Training architecture:

- mostly deterministic code
- classifier/evaluator for ambiguous action normalization later
- generative model only for explanatory event narration, not legality

Training artifacts:

- action-to-event receipts
- blocked-action examples
- normalized action labels
- mechanical delta records

Pilot schema shakedown gate:

- do not train until action/event schemas stabilize
- collect 100 reviewed action-to-event examples before considering normalization
  assistance

### 7. Participant Appraiser

Purpose: interpret an observable event from one participant's local perspective.

Current implementation:

- manual reviewed appraisals in mutation receipts
- Qwen appraiser experiments exposed invalid path drift

Inputs:

- observable event
- participant-local state
- participant memories
- perceived relationship stance
- cultural priors
- scene pressure
- known facts and suspected facts
- human interaction history when the appraiser is being used for VoidBot
- conversational tone, timing, repair attempts, recurring preferences, and
  boundary signals for human-facing appraisal
- post-Rupture concept being interpreted, if any

Outputs:

- private interpretation
- emotional appraisal labels
- intent read, including misread possibility
- threat/reassurance/obligation/humiliation/etc. labels
- proposed memory candidate
- proposed relationship implications
- confidence notes
- human-facing opinion updates when applicable: trust, concern, irritation,
  respect, attachment, caution, boundary pressure, expected future behavior,
  and what would change the read
- concept-specific misread labels, for weird Elysium phenomena

Training architecture:

- multi-label classifier or cross-encoder for labels
- generative decoder LLM for explanation and memory wording
- calibrated confidence model only after enough labels exist

Training artifacts:

- appraiser receipts
- misread/correct-read labels
- emotional appraisal labels
- rejected interpretations
- reviewer rationale
- VoidBot-facing human interaction appraisal receipts, including what was
  observed, what was inferred, what stayed uncertain, what opinion changed, and
  what evidence would reverse or soften the read

Pilot schema shakedown gate:

- 300 participant appraisal examples
- balanced set of correct reads, misreads, ambiguous reads, and biased reads
- examples include fundamental attribution bias and confidence movement
- at least 50 human-facing VoidBot appraisal examples before claiming the
  appraiser is useful for companion-style memory, opinion, or user-read updates
- future-branch examples include socially grounded misreads of post-Rupture
  phenomena

### 8. State Mutator

Purpose: apply reviewed appraisal into bounded agent, memory, relationship, and
world-state changes.

Current implementation:

- manual audited mutation receipts
- schema validation for final fixtures

Inputs:

- prior state refs
- event record
- participant appraisals
- mutation authority constraints
- accepted memory candidates
- relationship/perception implications
- canon status and promotion eligibility

Outputs:

- canonical state patch proposal
- memory writes
- perceived overlay updates
- relationship stance deltas
- activation deltas
- rejected or deferred deltas
- reviewer rationale

Training architecture:

- keep manual longest
- generative decoder LLM or structured patch model after schema stabilizes
- deterministic validator applies bounds and authority
- possible classifier for whether a delta is justified

Training artifacts:

- mutation receipts
- accepted patches
- rejected patches
- deferred unresolved facts
- validator failures
- branch-local patches separated from promoted state patches
- branch-local patches separated from cross-branch source constraints

Pilot schema shakedown gate:

- 500 accepted mutation examples
- 200 rejected/deferred mutation examples
- stable patch schema
- validator catches out-of-authority and private-state writes

### 9. Relationship And Social Perception Updater

Purpose: update directional relationship stance and perceived overlays over time.

Current implementation:

- manual relationship movement in mutation receipts

Inputs:

- prior directional relationship stance
- event/appraisal history
- memories
- perceived overlay
- cultural priors
- confidence drivers
- future-contact expectations

Outputs:

- relationship stance deltas
- perceived disposition updates
- confidence movement
- stale impression preservation
- social reinforcement notes
- reviewer rationale

Training architecture:

- multi-label classifier for movement direction and dimensions
- regression or ordinal classifier for magnitude buckets
- cross-encoder/ranker for candidate deltas
- avoid raw unconstrained numeric generation early

Training artifacts:

- relationship update receipts
- perceived-state update receipts
- confidence driver labels
- rejected updates

Pilot schema shakedown gate:

- 500 directional relationship updates
- repeated-contact sequences, not only isolated events
- evaluation includes asymmetric relationships and stubborn misreads

### 10. Branch Compiler

Purpose: turn reviewed scene intent, branch candidates, consequence packets, and
fold plans into playable Ink plus sidecar receipts.

Current implementation:

- coordinator-authored Ink fixtures
- documented branch-and-fold contract
- manual materialization and review

Inputs:

- fixture brief and scene spine
- source-grounded lore digest
- current world, scene, resource, relationship, and branch state
- actor-local branch candidates, including speech and non-speech actions
- responder outputs or coordinator-authored placeholder reactions
- reviewed consequence packets
- branch-and-fold plan
- visual continuity requirements
- schema and Ink formatting constraints

Outputs:

- readable `.ink` scene content
- Ink variables, choices, knots, gathers, and conditionals
- `.training.json` sidecar tying branches to state basis, action intent,
  consequences, callbacks, and review status
- compiler notes for fold decisions, route-split decisions, and known review
  risks
- base image prompt handles and branch/state modification handles

Training architecture:

- start as reviewed structured authoring
- generative decoder LLM for fixture drafting once sidecar shape stabilizes
- deterministic validators for Ink syntax, branch ids, sidecar refs, and schema
  shape

Training artifacts:

- accepted and rejected compiled fixtures
- Ink/sidecar pairs
- compiler notes
- fold and route-split decisions
- state-variable read/write maps
- visual prompt modifier maps
- reviewer findings and repair receipts

Pilot schema shakedown gate:

- 25 accepted fixtures across multiple story shapes
- 25 rejected or needs-revision fixtures with labeled compiler failures
- every accepted fixture has an Ink file, sidecar, compiler notes, and review
  status
- every material variable is read later or marked telemetry-only before
  acceptance
- branch compiler output is reviewed by the IF artifact reviewer before it
  enters the accepted corpus

### 11. IF Artifact Reviewer

Purpose: catch false consequence before branch fixtures become training data.

Current implementation:

- manual/frontier-model review
- failure taxonomy documented in
  `docs/architecture/ink-branching-scenes.md`

Inputs:

- Ink file
- `.training.json` sidecar
- coordinator artifact
- branch compiler notes
- declared variables, resources, risks, relationship bands, and visual
  modifiers
- intended branch-and-fold plan
- validation output

Outputs:

- `accepted`, `needs_revision`, or `rejected`
- severity-ranked findings
- fake-variable audit
- choice-consequence audit
- fold/callback audit
- state-gated prose audit
- visual-continuity audit
- required fixes

Training architecture:

- classifier or cross-encoder for known failure labels
- LLM judge for nuanced IF design failures, with reviewed labels saved
- deterministic support tools for variable set/read maps and branch-id
  consistency

Training artifacts:

- accepted fixtures
- rejected fixtures
- labeled failures such as `potemkin_state`, `cosmetic_choice`, `fake_fold`,
  `missing_state_read`, `state_named_instead_of_checked`,
  `visual_callback_missing`, and `ending_ignores_major_state`
- reviewer rationales
- repaired fixture diffs tied to findings

Pilot schema shakedown gate:

- 100 reviewed fixtures or fixture fragments
- 100 labeled failures, including at least 25 fake-variable or fake-fold cases
- reviewer catches variables that do not affect affordances, risk, appraisal,
  callbacks, visual state, or outcomes
- reviewer catches prose that names alternate branch states instead of checking
  variables
- reviewer catches endings that ignore major prior state

### 12. Output Evaluator And Guardrail Classifiers

Purpose: reject bad model outputs before they become artifacts or state.

Current implementation:

- schema validators
- prompt-leak checks
- body-affordance checks
- object semantics checks

Inputs:

- generated output
- source refs
- local context
- schema contract
- failure taxonomy

Outputs:

- pass/fail
- failure labels
- severity
- repairability
- reviewer notes

Training architecture:

- classifier or cross-encoder
- deterministic regex/schema checks where sufficient
- LLM judge only for nuanced failures, with reviewed labels saved

Training artifacts:

- rejected outputs
- accepted outputs
- false positive/false negative review notes
- failure taxonomy examples

Pilot schema shakedown gate:

- 300 labeled failures across prompt leakage, omniscience, wrong body, wrong
  lore, invalid mutation, schema drift, and flattened character agency

### 13. Institution, Faction, And Consumer Decision Models

Purpose: model non-dialogue decisions in economic, factional, institutional, and
open-world contexts.

Current implementation:

- not built yet
- preserve hooks in scene data now

Inputs:

- actor or institution state
- resources and scarcity
- obligations
- reputation
- faction norms
- expected future contact
- legal authority
- risk and cost models
- local information and misreads
- post-Rupture infrastructure or concept constraints, when applicable
- active technology order, faction attractors, fated events, and quest hooks
- item manifest refs
- faction tech-base refs
- component, assembly, and supply-chain dependencies

Outputs:

- decision/action proposal
- perceived cost and benefit
- moral injury or taboo pressure
- resource deltas proposed
- reputation/faction pressure implications
- rationale
- branch-local policy or market assumptions
- constrained-event response: exploit, resist, comply, delay, adapt, sabotage,
  or organize around the attractor/rail/injection
- manufacture, buy, salvage, counterfeit, maintain, upgrade, embargo, or
  substitute decisions

Training architecture:

- generative decoder LLM for complex decisions with rationale
- classifier/ranker for candidate choices
- possible small policy model later inside simulation loops

Training artifacts:

- buy/refuse/trade/hoard/comply/defect/conceal/punish/help examples
- accepted and rejected decision rationales
- resource and reputation consequence labels
- constrained-event decision examples for faction founding, technology adoption,
  quest hooks, and pressure-born institutions
- item availability and component dependency decisions
- faction access or restriction decisions

Pilot schema shakedown gate:

- do not train until scene-local machinery produces reliable decision examples
- collect at least 500 reviewed decision examples across consumer and
  institutional contexts
- include future-branch decisions where post-Rupture miracles become markets,
  privileges, liabilities, dependencies, or weapons
- include examples where factions or companies emerge from branch pressures,
  fated constraints, or quest injections
- include examples where factions differ in starting pre-Elysium tech base,
  manufacturing rights, maintenance ability, and supply-chain dependency

### 14. Technology And Item Manifest Generator

Purpose: turn worldbuilding exploration into game-usable item, component,
assembly, technology, and supply-chain records.

Current implementation:

- not built yet
- architecture plan exists in
  `docs/architecture/technology-item-manifest-plan.md`
- concrete target schema exists in Aetheria-Economy as CultCache-backed
  `ItemData` technological blueprint classes and linked `ItemInstance` runtime
  classes

Inputs:

- coordinator plan and glue prose
- branch lineage and prior conditions
- source refs from AetheriaLore
- nebulous named technologies from existing timeline/lore notes
- technology discovery events
- faction or manufacturer state
- item family or gameplay role
- existing item manifests
- pre-Elysium starting tech refs
- post-Rupture concept constraints
- existing Aetheria-Economy item definitions, categories, behavior data,
  hardpoint types, and known engine schema gaps

Outputs:

- candidate Aetheria-Economy technological blueprint records
- `SimpleCommodityData` candidates for fungible resources and feedstock
- `CompoundCommodityData` candidates for assemblies, subassemblies,
  components, processed materials, colony-consumed trade goods, and
  manufactured economic units
- `ConsumableItemData` candidates for usable consumables
- `GearData` or other `EquippableItemData` subclass candidates for usable
  equipment, ship systems, hulls, cargo structures, docking systems, and weapons
- assembly-tree, recipe, compatibility, and supply-chain metadata attached to
  the target blueprint record
- instance-provenance candidates when simulating manufactured artifacts,
  including realized quality, supply-chain lineage, manufacturer branding, and
  subassembly contributions to performance
- item family and item variant records as manifest metadata
- process, tooling, expertise, and facility records
- compatibility and upgrade-slot rules
- faction/manufacturer access records
- prerequisites and technology order
- supply-chain bottlenecks
- gameplay effects
- economic consequences
- quest hooks
- rejected unsupported item designs
- source-gap records for named tech that lacks actionable blueprint detail
- engine-schema gap records when a useful manifest concept has no CultCache
  field yet

Training architecture:

- start as reviewed structured authoring
- generative decoder LLM for manifest drafting once schema exists
- classifier/evaluator for unsupported tech, impossible compatibility, bad
  source grounding, and overpowered designs
- retrieval model for prerequisite/component dependency lookup later

Training artifacts:

- item manifest records
- blueprint records
- component breakdown records
- assembly compatibility records
- faction tech-base records
- technology discovery records
- branch-local innovation records
- economic consequence records
- rejected item designs with failure labels
- nebulous-tech elaboration records with inferred/source/gap fields
- instance-provenance records for manufactured artifacts when a scene or branch
  needs realized supply-chain history, not just a blueprint

Pilot schema shakedown gate:

- item manifest schema and validator exist
- validator checks target Aetheria-Economy blueprint class, source-backed
  fields, inferred fields, metadata-only fields, instance-provenance fields, and
  engine-schema gaps
- 100 reviewed item/component/assembly records
- 50 faction tech-base records across pre-Elysium and future branches
- 50 rejected/invalid item examples with clear failure labels
- at least 10 branch-local innovations tied to lineage and prerequisites
- at least 10 existing named technologies elaborated from vague lore into
  candidate blueprints with explicit source facts, inferred engineering, and
  open gaps

## Training Order

Do not train everything at once. That way lies a ceremonial bonfire of GPUs.

Recommended order:

1. Keep deterministic validators and artifact schemas ahead of data volume.
2. Build coordinator artifact schema and start saving coordinator receipts.
3. Expand projected local context and character-turn capture receipts.
4. Stabilize branch compiler artifacts for Ink, sidecars, fold notes, and visual
   prompt handles.
5. Build IF artifact reviewer labels from existing branching failures.
6. Build broader evaluator labels from responder, projector, lore, and mutation
   failures.
7. Fine-tune or LoRA the Aetheria responder first, because strong response data
   directly improves visible output and reduces lore handholding.
8. Add future-branch post-Rupture examples before claiming the responder can
   handle Elysium-era concepts without heavy prompting.
9. Train projector only if deterministic projection becomes a bottleneck.
10. Train appraiser and relationship updater after enough manual mutation receipts
   exist.
11. Train state mutator last among social organs because bad mutation corrupts
   the world state.
12. Build item manifest schema before large future-branch exploration produces
   technology data.
13. Train institution/faction/consumer models after scene-local decisions provide
   enough grounded examples.

## Minimum Corpus Before First Experimental Fine-Tune

These are still pilot numbers. Use them for controlled experiments, schema
pressure tests, and early behavior probes. Do not treat them as enough for
robust runtime use.

A first useful Aetheria responder fine-tune wants:

- 200 accepted character turns
- 50 non-speech or mixed-action turns
- 50 rejected/negative examples with clear labels
- exact responder-visible input for every turn
- raw responder output separated from reviewed or coordinator-repaired output
- hidden-context refs and leakage audits for every turn
- at least 5 factions or institutional contexts
- at least 3 time periods or historical flashpoints
- at least 50 accepted future-branch post-Rupture turns before using the model
  for Elysium-era generation with reduced lore scaffolding
- source refs for every grounded example
- explicit local-context boundary for every turn
- no Elysium branch facts mixed into single-history Sol data or unrelated
  sibling branches
- clear labels separating source-backed Elysium concepts from generated future
  branch facts
- branch lineage for every future-branch example

A first coordinator fine-tune wants:

- 50 accepted coordinator turns
- 25 rejected coordinator turns
- at least 5 complete scene transitions
- world-state refs in every example
- glue prose paired with structured state deltas or explicit no-delta notes

A first branch compiler model wants:

- 25 accepted Ink fixtures with sidecars and compiler notes
- 25 rejected or needs-revision fixtures with specific compiler failure labels
- variable set/read maps for every fixture
- branch-and-fold decisions marked as fold, conditional variant, or route split
- visual prompt handles tied to actual state variables
- IF artifact reviewer decisions for every accepted example

A first IF artifact reviewer wants:

- 100 reviewed fixtures or fixture fragments
- 100 labeled failures
- examples where the human review found fake state or fake folds before repair
- repaired versions paired with the original finding
- held-out fixtures with intentionally subtle cosmetic-choice and callback gaps

A first appraiser/classifier training run wants:

- 300 participant appraisals
- balanced misread/correct/ambiguous/bias examples
- at least 50 VoidBot-facing human interaction appraisals with opinion-update
  labels and uncertainty notes
- labeled confidence drivers
- reviewer rationale

A first item-manifest model wants:

- 100 reviewed item/component/assembly records
- 50 faction tech-base records
- 50 rejected item designs or compatibility failures
- source refs or branch lineage for every record
- explicit component, assembly, prerequisite, bottleneck, and gameplay-effect
  fields
- named-tech elaborations that separate source facts from inferred engineering
  and open lore gaps

## Artifact Shape Rule

Every training artifact should answer:

- What did the model or human see?
- What was hidden from them?
- What did they produce?
- What was accepted?
- What was rejected?
- Why?
- Which source or state refs justify the decision?
- Which future model family is this example for?

If an artifact cannot answer those questions, it is probably a transcript, not a
training example. Transcripts are nice. Training examples are how the beast gets
less stupid.
