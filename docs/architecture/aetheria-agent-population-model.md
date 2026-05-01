# Aetheria Agent Population Model

This note explains how a socially dense Aetheria region can feel inhabited
without simulating every implied person as a fully persistent agent.

The short answer is that population and agency are not the same thing.

A corridor can imply hundreds of thousands or millions of people while only a
few dozen individuals need to exist as full long-lived social nodes.

## Design Goal

We want a region that feels:

- crowded
- economically alive
- politically pressured
- socially stratified
- full of rumor, labor, traffic, and institutional drag

We do **not** want:

- ten thousand expensive little chatbots idling in a warehouse
- fake scale achieved by silent cardboard extras
- a world where only named characters seem to exist

So the population model needs to separate:

- who exists as a persistent social mind
- who exists as a grouped social force
- who exists only as local scene color
- how offscreen population pressure becomes visible

## Four Population Layers

The MVP should use four layers.

### 1. Persistent Agents

These are the full agents.

They have:

- identity
- memory
- relationships
- goals
- obligations
- changing stance toward institutions and people

These are the characters who accumulate history and can plausibly remember what
happened last week, who lied to them, who saved them, and which institution
just humiliated them.

Examples in an Aetheria content-generation slice:

- a faction liaison
- a ship officer
- a labor organizer
- a corporate handler
- a sanctuary coordinator
- a black-market broker
- a cutout vendor
- a technical specialist with inconvenient loyalties

These are not the population. They are the pressure points where
population-scale systems become legible.

### 2. Cohorts

Cohorts are grouped social bodies that matter but do not need full individual
cognition.

They should represent classes of people whose collective condition can change
over time.

Examples:

- dock labor shifts
- convoy deck crews
- insurer back-office clerks
- sanctuary residents
- clinic queues
- pirate route crews
- private security contractors
- debt pilgrims
- repair-bay technicians
- refugee transit waves

Each cohort should have state such as:

- size
- location
- stress
- trust in institutions
- rumor exposure
- faction sympathy
- fatigue
- scarcity pressure
- protest potential
- desertion / mutiny risk

This is how the world gets to have labor, crowds, queues, shortages, and
unrest without pretending every forklift operator needs a hundred memories and
a private speech cadence.

### 3. Ephemeral Scene Actors

These are temporary people instantiated only when a scene needs bodies,
witnesses, clerks, mechanics, medics, bystanders, or small bits of friction.

They should have:

- a role
- a scene goal
- local awareness of current events
- a little voice
- minimal short-term memory

They should **not** be treated as deep persistent minds by default.

Examples:

- a berth marshal waving traffic through a panic checkpoint
- a frightened passenger arguing with a clerk
- a dockhand spreading a fresh rumor
- a medic triaging one more body than the shift can really handle
- a sanctimonious insurer junior who disappears once the scene ends

If an ephemeral keeps mattering, the system can promote them into a persistent
agent later.

### 4. Ambient Population Pressure

This is the layer that makes the world feel bigger than the visible cast.

It is not represented by people at all. It is represented by signals.

Examples:

- berth queues growing or shrinking
- cargo backlogs
- corridor delay maps
- sanctuary occupancy
- clinic wait times
- food prices
- insurance repricing
- labor slowdown notices
- protest bulletins
- casualty lists
- rescue requests
- media chatter
- tribunal rulings
- route closures

This is where the implied millions live in practice.

## How The Layers Interact

The model should feel like this:

- ambient pressure changes cohort state
- cohort state changes the incentives around persistent agents
- persistent agents make decisions that alter ambient pressure
- ephemeral actors express the local scene consequences

That loop lets a single adjudicator decision or pirate rescue cascade outward
into visible population-level effects without simulating every single person
affected.

## Why A Few Dozen Persistent Agents Is Enough

Because the simulation is not trying to create a census. It is trying to create
a socially legible machine.

In a dense Aetheria slice, one persistent agent can sit at the hinge point for
a great many people:

- one dispatcher affects dozens of ships and hundreds of crew
- one sanctuary coordinator gates a network under pressure from thousands of vulnerable lives
- one corporate official can reshape access, risk, and reputation
- one faction delegate can legitimize or delegitimize a compromise
- one broker can redirect shadow traffic across many dependent cells

The persistent cast is therefore best understood as:

- decision-makers
- bottlenecks
- brokers
- organizers
- interpreters of institutional rules
- people with unusual leverage over larger flows

They are the knots in the rope, not every fiber.

## Cohort Model

Cohorts should be tractable and opinionated.

Each cohort record should include:

- `cohortId`
- `type`
- `sizeBand`
- `region`
- `factionAffinity`
- `currentPressures`
- `resourceDependency`
- `confidence`
- `grievance`
- `discipline`
- `mobilizationPotential`
- `recentIncidents`

Useful cohort types for Aetheria story and content generation:

- dock labor
- ship crews
- sanctuary residents
- corporate workforce
- shadow-market crews
- repair-yard workers
- shadow clinic clients
- private security detachments
- local merchant bodies
- transient passenger pools

Cohorts do not need rich dialogue. They need believable collective reactions.

## Ephemeral Actor Rules

Ephemeral actors should be generated from:

- scene location
- cohort state
- current world pressures
- factional tone
- time of day / shift

That means the same dock concourse can produce different incidental people
depending on whether the corridor is calm, panicking, starving, over-inspected,
or politically inflamed.

Ephemerals should support:

- local exposition
- scene texture
- small moral friction
- witness testimony
- short disputes
- visible consequences

They should expire cheaply unless the world keeps pulling them back into
relevance.

## Promotion Rules

Some ephemerals should be promoted into persistent agents when:

- a player or major agent forms an ongoing relationship with them
- they become central to repeated incidents
- they accumulate leverage or notoriety
- they are the surviving witness to something politically explosive
- they become the face of a cohort-level shift

Promotion is how the world grows specific people organically instead of
requiring the whole cast to be authored up front.

## Population Signals That Make A Place Feel Busy

If the corridor is meant to feel inhabited, the system should constantly
surface signs of offscreen life.

That means scenes, dashboards, and local UI should be fed with things like:

- arrivals and departures
- route advisories
- late manifests
- dockside rumor digests
- occupancy warnings
- strike whispers
- inspection delays
- tribunal alerts
- debt-claim notices
- casualty and rescue numbers
- commodity shocks
- public feeds and scandal bursts

The player does not need to meet everyone. They need to feel surrounded by
consequences.

## Content-Generation Population Recipe

For the first authoring slice, a sensible starting mix is:

- **20-40 persistent agents**
- **10-25 cohorts**
- **0-20 ephemerals per active scene**, generated on demand

Persistent clusters should be chosen from the story context, but the useful
categories are:

- primary faction representatives
- ship or station operators
- labor and logistics figures
- care, sanctuary, or rights workers
- criminal or deniable-economy brokers
- corporate cutouts
- technical specialists
- witnesses with leverage

Cohorts:

- dock labor
- crews
- sanctuary or civilian overflow
- clinic queues
- administrative or inspection staff
- security contractors
- shadow-market cells
- repair techs
- cargo handlers
- passengers / transients

That is enough to make generated scenes feel socially populated without turning
the authoring runtime into a screaming furnace.

## Simulation Tier Mapping

The population layers should map onto the simulation tiers cleanly.

### Tier 0

- dormant persistent agents
- slowly changing cohorts
- coarse ambient indicators

### Tier 1

- active persistent agents making strategic moves
- cohorts shifting under pressure
- route, labor, and sanctuary state updating

### Tier 2

- a handful of persistent agents in scene
- relevant ephemerals generated for the local moment
- cohort state informing mood, risk, and crowd behavior

### Tier 3

- full interactive dialogue for the most relevant persistent agents
- ephemerals only as needed for texture or interruption
- world pressure visible through feeds, queues, alerts, and consequences

This keeps expensive cognition focused where it matters.

## What Not To Do

Do not:

- simulate every resident as a full agent
- let cohorts become inert statistical wallpaper
- use ephemerals as consequence-free decoration
- make named agents behave as if the rest of the world does not exist
- make ambient scale purely visual with no mechanical pressure

That path leads to a world that looks crowded and feels dead.

## Success Criteria

The model is working if:

- the player can feel institutional pressure without being told about it in lore prose
- small decisions by key agents visibly affect larger populations
- cohorts can become restless, relieved, fearful, or politically active
- scenes feel crowded when they should
- the same corridor can feel different under calm, panic, shortage, scandal, or repression
- the named cast feels embedded in a larger society rather than posed in a diorama

The real test is simple:

If the player leaves the room, do they still believe thousands of people are
having a bad day because of what just happened?

If yes, the world is inhabited.
