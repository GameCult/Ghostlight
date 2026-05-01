# Personality Signal Classifier

This note describes the classifier shape that should sit underneath future
personality and relationship modeling for conversational and Aetheria agents.

The key design decision is simple:

- keep a canonical read of what a line actually expresses
- then let each listener distort that read according to their own mind

If we skip that split and only model subjective interpretation, the whole
system turns into epistemic soup. Nobody shares a reality and every agent
becomes a floating bag of projections.

Another key design decision is almost as important:

- treat concealment, performance, and impression management as part of the signal
- do not assume dialogue is a clean window into the underlying person

## What Problem This Solves

Right now the live interaction model mostly works by:

- lexical phrase matching
- hand-written tags
- thresholded aggregation into dimensions like `Warmth`, `Drive`, `Grandiosity`, `Validation-Seeking`, `Anxiety`, `Control Pressure`, `Hostility`, `Suspicion`, `Rigidity`, and `Withdrawal`

That is cheap and inspectable, but brittle.

It misses:

- paraphrase
- subtle stance
- masked insecurity
- bluffing
- sarcasm
- mixed motives
- meaningful misunderstanding between different listeners
- defensive self-performance
- strategic concealment of traits
- the difference between genuine expression and shell maintenance

The next step is not a generic personality classifier. It is a targeted
social-signal classifier that predicts the dimensions we actually care about.

## Core Principle

The generated line is not the ground truth.

The speaker's hidden state is the ground truth source.

More precisely:

- stable personality
- current emotional state
- active goals
- relationship state
- scene context
- intended speech act

all determine what a line is trying to express.

That does **not** mean every utterance should be labeled with the speaker's
full personality vector. A high-drive, high-anxiety speaker will sometimes say
something that expresses neither. The unit of prediction is the **signal
expressed in this utterance or event**, not the total soul of the speaker.

It also means that what is expressed may itself be shaped by a defensive mask.

So the model should not ask only:

- what does this line reveal

It should also ask:

- what does this line conceal
- what identity is being performed
- what kind of injury is being defended against

## Model Layers

### 1. Speaker State

The speaker model should include:

- underlying organization
  - self-coherence
  - contingent worth
  - shame sensitivity
  - reciprocity capacity
  - mentalization quality
  - authenticity tolerance
  - mask rigidity
- stable dispositions
- behavioral dimensions
- situational modifiers
- active goals
- relationship state toward the target
- presentation strategy
  - charm
  - compliance
  - superiority
  - detachment
  - seductiveness
  - competence theater
  - moral theater
  - cultivated opacity
- intended act
  - reassure
  - threaten
  - deflect
  - impress
  - conceal
  - probe
  - submit
  - bargain

This is the hidden state that governs generation.

### 2. Utterance Generator

The LLM generates the line from the speaker state plus scene context.

That line is the observable artifact.

### 3. Canonical Signal Reader

This classifier predicts what the line actually expresses in behavioral terms.

This is the closest thing to the objective layer.

Targets should be event-level or utterance-level signals such as:

- `warmth`
- `drive`
- `grandiosity`
- `validation_seeking`
- `anxiety`
- `control_pressure`
- `hostility`
- `suspicion`
- `rigidity`
- `withdrawal`
- `volatility`
- `attachment_seeking`
- `distance_seeking`

In practice, the canonical reader may need to emit both:

- expressed interpersonal dimensions
- presentation-style signals

for example:

- genuine warmth vs performative warmth
- true withdrawal vs strategic detachment
- organic confidence vs compensatory grandiosity
- actual reciprocity vs manipulative agreeableness

Additional useful targets worth supporting:

- `status_hunger`
- `submission`
- `dominance`
- project-thinking-style facets such as:
  - `abstractness`
  - `long_arc_orientation`
  - `evidence_orientation`
  - `fixation_proneness`

Each output should carry:

- score
- confidence
- optionally a brief rationale span or supporting cues for debugging

### 4. Listener Perception Layer

This is where organic misunderstanding happens.

The listener does not see the canonical signal cleanly. They perceive it
through:

- emotional intelligence
- personality vector
- relationship baggage
- current stress and scarcity
- factional priors
- status assumptions

So the listener-specific read should be something like:

`perceived_signal = f(canonical_signal, listener_profile, relationship_state, situational_load)`

This is how you get:

- suspicious agents overreading threat
- attachment-hungry agents overreading warmth
- avoidant agents underreading bids for intimacy
- grandiose agents misreading correction as insult
- low-EQ agents flattening nuance into crude categories

That is the good stuff. That is where drama, conflict, and pathos come from.

It is also where agents can fail to see through a mask, or incorrectly think
they have seen through one.

## High-EQ Special Cases

A high-EQ entity should not be a literal telepath.

It should instead be:

- extremely high emotional intelligence
- low distortion
- high calibration
- good at expressing uncertainty when the read is weak

In practice:

- a high-EQ perceived read should usually stay close to the canonical signal
- it should still be able to say, in effect, "I may be reading you as X, but the evidence is thin"

That feels more honest and less creepy than omniscience.

## Training Data Shape

We will probably need to train this ourselves. Off-the-shelf sentiment or
personality classifiers are the wrong beast.

The best bootstrap source is synthetic or simulated dialogue where we control
the hidden speaker state.

### Good Bootstrap Inputs

For each generated scene or event, record:

- speaker identity
- stable personality state
- current emotional state
- active goals
- relationship state toward listener
- scene context
- intended act
- generated utterance

Then label:

- canonical expressed dimensions
- canonical act strength
- presentation strategy in use
- likely listener perceptions for one or more listener profiles

### Important Warning

Do **not** train the model to infer the speaker's full personality vector from
one line.

Train it to infer:

- what dimensions are being expressed here
- what act is being attempted here
- what presentation strategy is being used here
- what a given listener would likely perceive here

The hidden personality vector is upstream supervision, not the direct per-line
label.

## Label Schema

Each event should ideally carry several related label sets.

### A. Canonical Expression Labels

Multi-label scores for dimensions such as:

- warmth
- drive
- grandiosity
- validation-seeking
- anxiety
- control pressure
- hostility
- suspicion
- rigidity
- withdrawal

Scale options:

- `0..3`
- `0..1` continuous
- ordinal buckets with confidence

### B. Canonical Interaction Labels

Things like:

- reassurance bid
- status bid
- dominance move
- attachment bid
- distancing move
- concealment
- persuasion
- compliance
- manipulation
- apology
- gratitude

### C. Canonical Presentation Labels

Things like:

- genuine disclosure
- impression management
- appeasement
- superiority display
- competence theater
- strategic opacity
- moral theater
- shell maintenance under threat

### D. Listener Perception Labels

For each selected listener profile:

- perceived warmth
- perceived threat
- perceived contempt
- perceived neediness
- perceived honesty
- perceived status move
- perceived flirtation or intimacy bid
- perceived disrespect

### E. Error Labels

It is useful to mark where the listener is wrong:

- overread
- underread
- misattributed motive
- hostile attribution error
- false reassurance read
- false contempt read
- failed mask detection
- false mask attribution

That makes misunderstanding legible instead of magical.

## How To Generate Training Data Without Sniffing Our Own Farts

If the same prompt stack generates all dialogue and all labels, we risk
building a self-licking ice cream cone.

Reduce that risk by:

- using multiple generation prompts
- varying surface style heavily
- generating masking, bluffing, sarcasm, and mixed-motive scenes on purpose
- using different models or prompt regimes for generation vs labeling when practical
- mixing in human-written or manually authored scenes
- keeping hard-edge heuristic checks for explicit things like:
  - insults
  - ownership language
  - repeated badgering
  - direct threats

The model should learn the soft social weather, not be trusted as sole judge of
every obvious profanity.

## Hybrid Runtime Strategy

Do not throw away heuristics.

Use a hybrid:

- heuristics for explicit hard edges
- classifier for softer dimensions
- classifier or heuristics for overt presentation strategies when they are obvious
- aggregation layer for profile updates
- listener perception layer for agent-specific misreading

That gives us:

- reliability where language is explicit
- nuance where language is messy
- inspectability when we need to debug why an agent thinks what it thinks

## Aggregation And Profile Updates

The classifier should not immediately rewrite a profile from one utterance.

Use:

- recency weighting
- repeated-evidence thresholds
- relationship context
- severity overrides for extreme incidents

One brutal betrayal should matter fast.

One odd line should not convince the system that somebody is now permanently
grandiose, avoidant, or in love.

Likewise, one polished or ingratiating line should not convince the system that
a performed self is the whole person.

## Evaluation

We should test three separate things.

### 1. Canonical Classification

Can the system accurately read what the line expresses?

### 2. Perception Modeling

Can different listeners plausibly misread the same line in different ways?

### 3. Downstream Behavioral Usefulness

Does the resulting read produce better behavior?

For example:

- better boundary enforcement
- more believable tenderness
- more coherent grudges
- more plausible market and cohort drift
- more interesting multi-agent friction

If the classifier is accurate but nothing downstream improves, congratulations,
we built a lab rat with a diploma.

## Why This Matters For Aetheria

This is not only for conversational entities.

The same stack can power:

- interpersonal drama
- factional distrust
- consumer susceptibility
- propaganda response
- labor radicalization
- prestige signaling
- institutional conditioning
- cohort emotional drift

That means Aetheria populations can be modeled as:

- slow dispositions
- pressure-sensitive dimensions
- relationship state toward institutions and factions
- collective memory
- biased perception of propaganda, leadership, and crisis

This is a far better fit than asking whether a colony is "high in
extraversion" and pretending that captures how it responds to scarcity,
humiliation, media pressure, and prestige markets.

It is also a better fit for modeling institutions that train presentation
itself:

- prestige cultures
- fear states
- audit-heavy regimes
- influencer or media ecologies
- environments where people survive by performing the correct self

## Near-Term Implementation Path

1. Keep the current heuristic pipeline as the hard-edge fallback.
2. Add an event-level multi-label classifier for the existing dimensions.
3. Persist per-event classifier scores and confidence alongside the event record.
4. Rework profile synthesis to aggregate classifier outputs instead of just tag counts.
5. Add a listener-perception layer driven by:
   - listener EQ
   - listener personality
   - relationship state
   - situational load
6. Add synthetic bootstrap scene generation with hidden-state labels.
7. Add adversarial evaluation scenes with:
   - bluffing
   - masking
   - sarcasm
   - mixed motives
   - asymmetric status

## Bottom Line

The right shape is not:

- one global personality classifier
- one single truthless perception model
- one line mapped directly to the speaker's whole personality

The right shape is:

- hidden speaker state
- generated utterance
- canonical social-signal read
- canonical presentation-style read
- listener-biased perception of that signal
- profile updates based on repeated evidence

That gives us honest reading, organic misunderstanding, and enough structure to
let agents become socially interesting instead of merely chatty.

For the broader state-distribution and prompt-projection architecture that
should feed this classifier, see
[Agent State Distributions And Prompt Projection](../architecture/agent-state-distributions-and-prompt-projection.md).
