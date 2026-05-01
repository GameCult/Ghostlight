# Agent State Distributions And Prompt Projection

This note describes how personality, culture, memory, and social context should
be represented in structured state, and how that state should be projected into
prompts for dialogue generation.

The central rule is simple:

- the prompt is not the source of truth
- the prompt is a temporary projection of deeper state for one turn or one scene

If we try to store the whole person in prompt prose, the system turns back into
theatrical mush. The real machine has to live in explicit state.

For the related notes on personality structure and dialogue classification, see:

- [Personality Model Roadmap](../research/personality-model-roadmap.md)
- [Personality Signal Classifier](../research/personality-signal-classifier.md)

## Core Principle

We should model agents through:

1. canonical latent state
2. perceived state for each agent
3. prompt projection from perceived state into live generation

That gives us:

- stable world truth
- subjective misunderstanding
- efficient live prompting without pretending the prompt is the database

## State Families

Each agent should have explicit state families, not one giant personality blob.

### 1. Underlying Organization

These are deep structural variables:

- self-coherence
- contingent worth
- shame sensitivity
- reciprocity capacity
- mentalization quality
- authenticity tolerance
- mask rigidity
- external-regulation dependence

These are slow-changing and often heavily shaped by early development and
repeated relational history.

### 2. Stable Dispositions

These are relatively persistent tendencies:

- novelty-seeking
- conformity
- status hunger
- risk tolerance
- sociability
- baseline threat sensitivity
- aesthetic appetite
- ideological rigidity

### 3. Behavioral Dimensions

These are first-class, not optional garnish. They belong in the rebuild from
the start:

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
- volatility
- attachment-seeking
- distance-seeking

These are the main handles for social behavior, institutional pressure, and
consumer or factional drift.

### 4. Presentation Strategy

These describe how the self is performed:

- charm
- compliance
- superiority
- detachment
- seductiveness
- competence theater
- moral theater
- strategic opacity
- cultivated harmlessness
- abrasive boundary
- ironic distance

These are not just decorative style. In some agents they are survival
machinery.

### 5. Voice Style

These describe how the performed self comes out in language:

- dryness
- sarcastic deflection
- pointedness
- sincerity evasion
- emotional evasion
- banter as boundary
- verbal aggression
- plainspoken directness

This layer matters because presentation strategy and voice are related but not
identical. Performing a prickly self may come out as dry sarcasm, clipped
literalism, theatrical disdain, or a flat refusal to offer emotional handles.
If the schema only stores the performed self, the renderer has to guess the
voice. That is how everyone becomes the same quippy little goblin with
different hats.

### 6. Situational State

These are transient activations:

- exhaustion
- scarcity pressure
- humiliation
- panic
- triumph
- grief
- overstimulation
- grievance activation
- acute shame
- perceived status threat

### 7. Relationship State

Per target person, faction, group, product domain, or institution:

- trust
- resentment
- dependence
- fear
- fascination
- obligation
- envy
- moral disgust
- perceived status gap
- expectation of care
- expectation of betrayal

### 8. Goals And Values

- active goals
- protected values
- emotional stakes
- tradeoffs
- unforgivables

## How To Represent Variables

Do not represent most of these as naked scalars if we want culture and history
to shape them properly.

A practical representation is:

- `mean`
- `plasticity`
- `current_activation`

So instead of:

- `status_hunger = 0.72`

use something like:

- `status_hunger.mean = 0.72`
- `status_hunger.plasticity = 0.22`
- `status_hunger.current_activation = 0.89`

This gives us:

- stable baseline personality
- different rates of change
- scene-sensitive activation without rewriting the whole person every turn

Canonical authored state should not include epistemic uncertainty about itself.
If a value is only an observer's read, store it as perceived state rather than
canonical state. Perceived dimensions should not expose canonical `mean` or
`plasticity`; they should track observed activation, attributed disposition,
and confidence. This lets the model represent
fundamental attribution bias: someone can behave a certain way under pressure
and be perceived as simply being that way.

## Culture As A Prior Field

Culture should not create clones. Culture should shape distributions.

For each culture, faction, class stratum, or institution, define:

- prior means over variables
- covariance structure
- reward gradients
- penalty gradients
- preferred masks
- taboo map
- prestige map
- default relationship posture to insiders, outsiders, rivals, inferiors, and superiors
- favored narrative scripts

This means a culture is not "people here are high in trait X."

It is more like:

- this culture rewards controlled presentation
- penalizes spontaneous vulnerability
- elevates status anxiety
- normalizes suspicion of low-status outsiders
- attaches prestige to procedural fluency and emotional composure

That is how a Dominion or Lucent style environment should work: as a field that
shapes what people become profitable or safe enough to be.

## Personal History As A Transform Layer

Culture is only the starting pressure.

Then the prior is transformed by:

- attachment history
- class position
- faction indoctrination
- educational shaping
- labor regime
- remembered humiliations
- remembered betrayals
- remembered rescue, admiration, abandonment, and obligation

So agent state should be read as:

- cultural prior
- transformed by developmental history
- transformed by remembered episodes
- currently activated by scene pressure

## Cohorts And Populations

The same representation can be lifted from individuals to populations.

A colony, district, labor bloc, or consumer cohort can have:

- shared priors
- institutional conditioning
- repeated collective incidents
- relationship state toward factions, employers, media ecosystems, and product classes

This is much better than Big Five-style population sliders because it lets us
model things like:

- prestige hunger
- fear-state compliance
- grievance accumulation
- attachment to institutions
- validation loops induced by media
- suspicion induced by audit or betrayal

That is much closer to Aetheria than a broad trait scaffold alone.

## Canonical State Vs Perceived State

The world should not only track what is true. It should track what each agent
thinks is true.

So we need:

- canonical world state
- canonical agent state
- per-agent perceived state

Perceived state includes:

- what this agent believes about others
- what motives they attribute
- what traits they think they are seeing
- what mask they think someone is wearing
- what threats or opportunities they think are present

This is where emotional intelligence, suspicion, attachment hunger, status
anxiety, and cultural priors distort interpretation.

The acting agent should mostly be prompted from perceived state, not canonical
truth. That is how misunderstanding becomes organic instead of hand-authored.

## Prompt Projection

Prompt rendering should be treated as projection, not storage.

For a live scene, build the prompt from:

### 1. Identity And Role

- who the agent is
- what role they occupy
- what they want to be seen as

### 2. Cultural Default Frame

- prestige rules
- taboo zones
- expected forms of control or intimacy
- factional posture

### 3. Current Presentation Strategy

- how they are trying to appear right now
- what they are concealing
- what they must not look like

### 4. Current Situation

- scene pressure
- public exposure
- scarcity
- humiliation risk
- safety risk

### 5. Relationship Context

- what they think of the other party
- what they need from them
- what wounds or debts are active
- what status relation they perceive

### 6. Relevant Memory

Not full history. Only:

- the few most relevant episodes
- relationship summaries
- current active fears, loyalties, or grievances
- currently salient cultural scripts

### 7. Goals And Constraints

- what they are trying to achieve in this scene
- what they cannot afford
- what values they will not betray

### 8. Listener Model

If the scene involves another agent, include:

- what this agent thinks the other wants
- what they fear the other is doing
- where they may be misreading them

## What The Prompt Should Not Do

Do not make the prompt:

- the sole canonical store of identity
- a full dump of all memories
- a taxonomy recital
- a fake omniscient summary that erases subjective misreading

The prompt should be compact, selective, and scene-bound. The state store
carries the rest.

## Economic Consequences

This model gives us much more useful demand and cultural behavior than old
trait-only systems.

For example:

- high status hunger plus validation-seeking plus fragile self-worth leads toward prestige goods, symbolic upgrades, and visible luxury
- high threat sensitivity plus conformity plus shame reactivity leads toward safety goods, procedural systems, insurance, and respectable signaling
- high drive plus novelty-seeking plus low institutional attachment leads toward frontier gear, gray-market experimentation, and risky migration
- high mask rigidity plus high impression-management intensity leads toward corrective aesthetics, social camouflage, and reputation-maintenance consumption

That is how culture, labor, media, and institutions become market-shaping
forces instead of mere flavor text.

## Recommended Implementation Path

1. Define the state schema with first-class support for:
   - underlying organization
   - stable dispositions
   - all core behavioral dimensions, including `volatility`, `attachment-seeking`, and `distance-seeking`
   - presentation strategies
   - voice styles
   - situational state
   - relationship state
   - goals and values
2. Represent those variables as priors plus activation, not just fixed scalars.
3. Define cultural and institutional prior templates.
4. Add history transforms from remembered episodes.
5. Add per-agent perceived state overlays.
6. Build prompt renderers that project perceived state into live generation.
7. Keep canonical state, perceived state, and prompt projection cleanly separated.

## Bottom Line

The right machine is not:

- a persona prompt with some adjectives taped to it
- a canonical trait sheet with no cultural shaping
- a direct line from "personality value" to dialogue

The right machine is:

- latent state distributions
- shaped by culture
- reshaped by history
- distorted by perception
- projected into prompts for one scene at a time

That gives us agents who can feel culturally situated, personally scarred,
socially biased, and dynamically alive without making the prompt carry the
whole civilization on its back.
