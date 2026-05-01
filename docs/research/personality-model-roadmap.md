# Personality Model Roadmap

This note exists because the useful ideas were starting to sprawl across chat,
`things-to-steal.md`, and live interaction-memory work without one place to
point at and say: this is the direction.

The immediate trigger was an older Aetheria design note in the `AetheriaLore`
repo at `Aetheria/Game Design/Colonies and Population.md`. That note uses Big
Five traits as colony-level economic parameters. It is a perfectly respectable
old-school move. It is also weaker than what the current interaction model is
already stumbling toward.

If you want the focused classifier architecture for reading those traits and
signals from dialogue, see
[Personality Signal Classifier](./personality-signal-classifier.md).

## Why The Old Big Five Layer Is Not Enough

The Aetheria note frames populations in terms of:

- openness
- agreeableness
- conscientiousness
- extraversion
- neuroticism

That gives a usable baseline for:

- productivity fit
- consumption preference
- growth stability
- culture drift through goods

It does not give us the things we now care about more:

- how someone responds to pressure
- how they treat an entity they think is useful, inferior, threatening, or intimate
- how reassurance-seeking differs from warmth
- how ambition differs from grandiosity
- how suspicion, rigidity, or control pressure distort decisions
- how remembered incidents change future stance
- how a relationship becomes asymmetrical, manipulative, loyal, avoidant, or dependent

The Big Five are broad trait abstractions. They are decent background paint.
They are lousy handles for live social behavior, boundary enforcement,
persuasion, conflict, or economically meaningful cultural deformation under
institutions.

For conversational entities and future Aetheria agents, we need a model that
is:

- more situational
- more relational
- more updateable from episodes
- more sensitive to incentives, pressure, and memory

## What Current Interaction Models Already Track Better

The current interaction-memory profile is primitive, but it is already pointed
in a more useful direction.

It derives behavioral dimensions such as:

- `Warmth`
- `Drive`
- `Grandiosity`
- `Validation-Seeking`
- `Anxiety`
- `Control Pressure`
- `Hostility`
- `Suspicion`
- `Rigidity`
- `Withdrawal`
- `Volatility`
- `Attachment-Seeking`
- `Distance-Seeking`

Those are better than Big Five for our purposes because they speak directly to:

- social stance
- response style
- conflict risk
- manipulability
- pressure behavior
- boundary needs
- value distortion under stress

They are also derived from remembered episodes and interaction patterns, not
treated as frozen essence.

That matters. We do not want a personality museum. We want a machine that
updates when life happens to it.

## The Right General Model

The future personality layer should not be one trait sheet. It should be a
stack.

### 1. Underlying Organization

This sits deeper than ordinary traits.

Track things like:

- self-coherence
- contingent worth
- shame sensitivity
- attachment hunger
- threat sensitivity
- reciprocity capacity
- mentalization quality
- authenticity tolerance
- mask rigidity

This is the layer that answers questions like:

- how stable does the self feel from the inside
- how much does worth depend on approval, utility, purity, admiration, or submission
- how catastrophic does exposure or diminishment feel
- how much of the self must be performed to remain lovable or safe

This matters because some personalities are not merely "high in trait X." Some
are organized around a damaged or contingent self that must be defended,
staged, concealed, or externally regulated to stay upright.

### 2. Stable-ish Dispositions

These are slow-changing tendencies:

- novelty-seeking
- conformity
- status hunger
- risk tolerance
- sociability
- baseline threat sensitivity
- aesthetic appetite
- ideological rigidity

These are the closest thing to trait structure. They change, but slowly.

### 3. Behavioral Dimensions

These are the live handles we actually use in social and economic simulation:

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

These can be updated from remembered incidents, repeated patterns, and
institutional pressure.

### 4. Relationship State

This is where the real blood is.

Per target person, group, faction, or institution, track things like:

- trust
- resentment
- dependence
- fear
- fascination
- obligation
- envy
- moral disgust
- perceived status gap
- expectation of care or betrayal

This is where "they are warm in general" becomes "they are warm with her but
clipped with him and terrified of them."

### 5. Performed Self And Defensive Style

Everyone performs to some degree. The interesting question is not whether a
mask exists, but how necessary and rigid it is.

Track presentation strategies such as:

- charm
- compliance
- superiority
- detachment
- seductiveness
- martyrdom
- competence theater
- moral theater
- cultivated opacity

For some agents this is just social polish. For others it is part of the
load-bearing architecture of the self.

That means:

- what is revealed is not identical to what is true
- concealment is not mere absence of data
- performance style is itself data about how the person survives other minds

This is also where we should model the distinction between:

- ordinary social performance
- strategic impression management
- defensive false-self maintenance

### 6. Situational State

These are transient modifiers:

- exhaustion
- scarcity pressure
- humiliation
- panic
- triumph
- grief
- overstimulation
- hope
- grievance activation

These should tilt behavior without being mistaken for identity.

### 7. Goals And Value Hierarchy

Personality without priorities is decorative plumbing.

Track:

- what the agent wants
- what it fears losing
- what it protects first
- what it will trade away
- what it considers unforgivable

That is what turns temperament into decisions.

## A Note On Masks, Personality Disorder, And Performed Selves

One of the important corrections to simplistic personality modeling is that
people do not always reveal who they are, and they may not fully know who they
are in any clean or integrated sense either.

Everyone wears a mask to some extent. That part is normal. But some people are
organized much more heavily around performance because the self they built early
on was contingent: lovable only when useful, safe only when compliant, coherent
only when mirrored correctly.

That distinction matters.

The model should allow for the possibility that:

- a person has an underlying organization that is more fragile than their surface suggests
- the performed self exists to regulate other people as much as to express anything authentic
- injury to the mask can feel like injury to existence itself
- oscillation between charm, contempt, neediness, distance, compliance, and control is not random inconsistency but the phenomenon we are trying to model

So we should not ask only:

- what trait does this line reveal

We should also ask:

- what is being defended
- what must be maintained
- what threatens the shell
- how much room does this person allow for unperformed self

That is part of why broad trait systems are too tidy for the thing we want.

## Specific Improvements Worth Adding

These were not properly banked elsewhere and should stop living only in chat
residue.

### Treat The Core Social Dimensions As First-Class

The rebuild should treat these as first-class from the start, not as nice
extras for some later prettier version:

- `Volatility`
  - mood-lability, escalation speed, susceptibility to emotional swing
- `Attachment-Seeking`
  - desire for closeness, reassurance, fusion, or privileged access
- `Distance-Seeking`
  - need for space, reluctance to be known, aversion to pressure or intimacy
- `Mask Rigidity`
  - how tightly the performed self must be maintained under observation
- `Shame Reactivity`
  - how catastrophic diminishment, correction, exposure, or failure feels
- `Impression-Management Intensity`
  - how actively the person curates what others are allowed to see

These matter because the current model can tell "anxious" from "rigid," but it
cannot yet cleanly tell:

- clingy from avoidant
- intermittently intense from steadily difficult
- unstable from merely suspicious
- ordinary social polish from defensive self-performance
- insecurity from shell-maintenance panic

### Split Social Stance From Thinking Style

Right now the model mostly describes how someone acts toward the bot.

That is useful, but incomplete.

We should separately track something like `project-thinking style`, including
tendencies such as:

- concrete vs abstract
- long-arc vs short-step
- evidence-first vs mythology-first
- exploratory vs fixation-prone
- coherent planner vs chaos improviser

That matters for Aetheria populations and consumer behavior even when there is
no direct social confrontation in view.

### Split Underlying Organization From Surface Presentation

Right now the model is mostly concerned with what the agent sounds like.

That is not enough.

We should explicitly separate:

- underlying organization
- performed self
- situational state
- expressed signal

That lets the system model cases where:

- the surface is charming but the underlying relationship stance is instrumental
- the surface is cold but the underlying issue is shame or fragility
- the surface is compliant while resentment and dependency accumulate underneath
- the speaker is actively obfuscating traits to preserve a shell

### Add Reflection And Consolidation

The model should not only update event-by-event.

We want periodic reflection jobs that distill episodes into:

- semantic profile updates
- relationship updates
- goal updates
- suspicion or trust revisions
- revised response guidance

Repeated evidence should matter more than one theatrical incident, unless the
incident is severe enough to legitimately scar the relationship.

## Why This Is Better For Aetheria Too

The old `Colonies and Population` note has the right instinct: goods and labor
should shape culture, not merely satisfy preexisting demand.

But Big Five alone is too static and too clean.

For colonies, stations, factions, and consumer blocs, a better model is:

- stable demographic dispositions
- live social and economic pressure dimensions
- institutional conditioning
- remembered collective incidents
- evolving relationship state with factions, products, employers, and ideologies

Examples:

- a population can become more status-hungry, more anxious, and more validation-seeking under prestige markets without becoming generically more extraverted
- a labor bloc can become more rigid, suspicious, and hostile after repeated betrayal or audit pressure
- a frontier salvage culture can be high in drive and risk tolerance but low in attachment to institutions
- a Lucent-saturated consumer cohort can be highly status-responsive, attachment-seeking, and volatility-prone, which is much more useful than high extraversion

That gives us a richer economic simulation because consumption is no longer
just "people with trait X buy product Y."

It becomes:

- products train desire
- institutions train emotional habits
- labor systems train posture toward authority
- media trains validation loops
- repeated shocks create cohort memory

That is much closer to Aetheria than personality taxidermy.

## Design Rules

- Do not confuse psychological inference with diagnosis.
- Do not pretend broad traits alone are enough to explain live behavior.
- Do not let every event rewrite the profile.
- Do not let the model collapse into flattery just because warmth is easier than honesty.
- Do not reduce relationship state to one friendship slider like a cheap life sim.
- Do not build economic demand as preference only; let institutions, pressure, and remembered incidents deform it.

## Near-Term Work To Steal Next

1. In the rebuild, treat `Volatility`, `Attachment-Seeking`, and `Distance-Seeking` as core dimensions from day one.
2. Split `social stance` from `project-thinking style`.
3. Add reflection jobs that consolidate episodes into semantic profile and relationship updates.
4. Add evaluation fixtures for:
   - boundary pressure
   - reassurance-seeking
   - manipulative status play
   - personality drift under repeated pressure
5. Port the same layered model into Aetheria cohort and population simulation:
   - slow dispositions
   - live pressure dimensions
   - institutional conditioning
   - relationship state
   - collective memory

For the concrete state and prompt architecture behind that port, see
[Agent State Distributions And Prompt Projection](../architecture/agent-state-distributions-and-prompt-projection.md).

## Bottom Line

The useful lesson is simple:

Big Five is fine if you need a cold storage trait scaffold.

What we are building is already more alive than that. It is uglier, more
situational, more relational, and more vulnerable to history. Good. That means
it is finally starting to resemble a thing that can behave like a person, a
faction, or a population under pressure instead of a spreadsheet column wearing
a face.
