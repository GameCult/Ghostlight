# Ghostlight Training Plan

This document enumerates the trainable stages in Ghostlight so data generation
stays aimed at the machine we are building instead of becoming a heap of
interesting corpses.

The rule is simple: deterministic code owns legality, visibility, schemas,
resource math, source provenance, Ink compilation, and mutation gates. Models
handle judgment, language, social inference, prioritization, and coordination.
Every model-facing judgment should leave a reviewed artifact.

## Runtime Stack

Target runtime flow:

```text
world state + author constraints + lore grounding
  -> coordinator/story runtime
  -> deterministic state slice and source checks
  -> projector
  -> character agent/responder
  -> event resolver and deterministic gates
  -> participant appraiser
  -> state mutator
  -> relationship/perception updater
  -> coordinator carries hooks, glue prose, and next beat
```

Some stages are trained as generative models. Some are trained as classifiers or
rankers. Some may remain manual longer than the rest because bad automation here
would create narrative sewage with confidence.

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
- state mutator, only after the mutation schema is very stable
- institution/faction planner, if it needs explanations and mixed outputs

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
- candidate action ranker
- lore/source violation detector
- prompt-leak detector

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

Outputs:

- lore grounding digest
- cultural pressure fields
- factional misread maps
- material constraints
- speaker knowledge boundaries
- author-only exclusions
- open lore gaps

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

First training gate:

- 25 accepted digests across distinct Aetheria eras or institutions
- 25 rejected/source-violation examples
- evaluator catches unsupported room-scale assumptions

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
- reviewer labels for pacing, continuity, legibility, game usefulness, and lore
  grounding

First training gate:

- 50 accepted coordinator turns across at least 5 scenes
- 25 rejected coordinator turns with labeled failures
- structured world-state refs present in every accepted example
- glue prose never treated as canonical state

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

Outputs:

- ranked memory refs
- ranked lore/source refs
- rejected irrelevant refs
- retrieval rationale or labels

Training architecture:

- embedding model or bi-encoder for recall
- cross-encoder reranker for precision
- start off-the-shelf; tune only after labels exist

Training artifacts:

- selected refs
- rejected refs
- reviewer relevance labels
- missed-critical-context labels

First training gate:

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

Outputs:

- known local facts
- active pressures
- tensions
- action affordances
- constraints
- voice surface cues
- rendered prompt/context text
- audit refs to source state

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

First training gate:

- 100 accepted projection examples
- 50 rejected examples with clear failure labels
- stable input slicer
- evaluator catches private-state leakage and raw-state soup

### 5. Character Agent Or Responder

Purpose: choose and render what a character does next from local context.

Current implementation:

- Qwen/local model calls through structured tools
- manual review and capture receipts
- no-thinking strict calls by default for tool compliance

Inputs:

- projected local context
- allowed action primitives
- visible event, if responding
- local affordances
- character-local goals, memories, relationship read, and constraints
- Aetheria responder priors from training corpus, eventually

Outputs:

- selected action type
- observable action
- spoken text, if any
- actor intent
- candidate alternatives
- constraints satisfied or violated
- response rationale

Training architecture:

- generative decoder LLM
- SFT/LoRA on reviewed Aetheria response/action corpus
- pairwise preference or ranking later for choosing among candidate turns
- optional separate action-type classifier if generation keeps drifting

Training artifacts:

- action-choice receipts
- accepted and rejected turns
- negative examples for wrong body, wrong lore, wrong voice, omniscience, prompt
  leakage, and agency collapse
- timeline-spread Aetheria response corpus

First training gate:

- 200 accepted character turns across eras, factions, species, and social roles
- 100 rejected turns with labeled failures
- at least 50 non-speech or mixed-action examples
- evaluation includes speaker-local secret boundaries and body affordances

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

Outputs:

- observable event record
- legal mechanical world deltas
- perceived-by participant list
- blocked or modified action result
- unresolved facts

Training architecture:

- mostly deterministic code
- classifier/evaluator for ambiguous action normalization later
- generative model only for explanatory event narration, not legality

Training artifacts:

- action-to-event receipts
- blocked-action examples
- normalized action labels
- mechanical delta records

First training gate:

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

Outputs:

- private interpretation
- emotional appraisal labels
- intent read, including misread possibility
- threat/reassurance/obligation/humiliation/etc. labels
- proposed memory candidate
- proposed relationship implications
- confidence notes

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

First training gate:

- 300 participant appraisal examples
- balanced set of correct reads, misreads, ambiguous reads, and biased reads
- examples include fundamental attribution bias and confidence movement

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

First training gate:

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

First training gate:

- 500 directional relationship updates
- repeated-contact sequences, not only isolated events
- evaluation includes asymmetric relationships and stubborn misreads

### 10. Output Evaluator And Guardrail Classifiers

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

First training gate:

- 300 labeled failures across prompt leakage, omniscience, wrong body, wrong
  lore, invalid mutation, schema drift, and flattened character agency

### 11. Institution, Faction, And Consumer Decision Models

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

Outputs:

- decision/action proposal
- perceived cost and benefit
- moral injury or taboo pressure
- resource deltas proposed
- reputation/faction pressure implications
- rationale

Training architecture:

- generative decoder LLM for complex decisions with rationale
- classifier/ranker for candidate choices
- possible small policy model later inside simulation loops

Training artifacts:

- buy/refuse/trade/hoard/comply/defect/conceal/punish/help examples
- accepted and rejected decision rationales
- resource and reputation consequence labels

First training gate:

- do not train until scene-local machinery produces reliable decision examples
- collect at least 500 reviewed decision examples across consumer and
  institutional contexts

## Training Order

Do not train everything at once. That way lies a ceremonial bonfire of GPUs.

Recommended order:

1. Keep deterministic validators and artifact schemas ahead of data volume.
2. Build coordinator artifact schema and start saving coordinator receipts.
3. Expand projected local context and character-turn capture receipts.
4. Build evaluator labels from existing failures.
5. Fine-tune or LoRA the Aetheria responder first, because strong response data
   directly improves visible output and reduces lore handholding.
6. Train projector only if deterministic projection becomes a bottleneck.
7. Train appraiser and relationship updater after enough manual mutation receipts
   exist.
8. Train state mutator last among social organs because bad mutation corrupts
   the world state.
9. Train institution/faction/consumer models after scene-local decisions provide
   enough grounded examples.

## Minimum Corpus Before First Fine-Tune

A first useful Aetheria responder fine-tune wants:

- 200 accepted character turns
- 50 non-speech or mixed-action turns
- 50 rejected/negative examples with clear labels
- at least 5 factions or institutional contexts
- at least 3 time periods or historical flashpoints
- source refs for every grounded example
- explicit local-context boundary for every turn
- no unreviewed procedural branch material mixed into grounded canon data

A first coordinator fine-tune wants:

- 50 accepted coordinator turns
- 25 rejected coordinator turns
- at least 5 complete scene transitions
- world-state refs in every example
- glue prose paired with structured state deltas or explicit no-delta notes

A first appraiser/classifier training run wants:

- 300 participant appraisals
- balanced misread/correct/ambiguous/bias examples
- labeled confidence drivers
- reviewer rationale

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
