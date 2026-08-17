# How Ghostlight Dungeon Keeps a Whole World Moving

An AI dungeon master has an awkward problem: the world should feel larger than
the conversation, but a language model can only attend to so much at once.

If the game simulates only the people currently speaking, everyone else seems
to stop existing. A distant government never exploits an opportunity. The
villagers cease reacting when the player talks to the blacksmith. A location
left behind becomes a vague reconstruction when the player returns.

The brute-force answer would be to give every person and organization its own
AI turn. That answer lasts until the setting contains twenty factions, several
cities, a trading network, a plague, and one unusually opinionated baker. It is
too slow and too expensive, and most of those turns would not matter right now.

Ghostlight Dungeon instead changes the *resolution of the simulation*. It can
consider one person in detail, a village as a group, or several rival powers as
one bounded political situation. The underlying people and facts do not get
merged or discarded. Only the amount of attention used to simulate them
changes.

That is the multiresolution gestalt algorithm.

## What “gestalt” means here

A gestalt is a population that can be simulated through a shared Persona turn.
“The dockworkers,” “the western villages,” or “the biology faculty” might each
begin as a gestalt. They have a common baseline, but that does not mean every
member is identical.

Suppose the player enters a village and asks for the blacksmith. Ghostlight can
materialize John from the villagers’ baseline, then preserve everything that
makes him particular: his name, memories, possessions, grudges, promises, and
relationship with the player. While John is directly involved, he receives his
own simulation attention.

When the player leaves, John does not disappear and his history is not averaged
away. His individual state remains as a durable delta from the villagers’
baseline. The simulator may include him in a cheaper village-level turn until
he matters again. When the player returns, the same John is materialized from
the same baseline plus the same personal delta.

This distinction is the foundation of the system:

> Ghostlight aggregates simulation work, not canonical identity or state.

## The agency graph

At campaign creation, the world compiler builds a coarse global map of agency.
It does not eagerly invent every street and citizen. It records the major
powers, institutions, populations, regions, information channels, and strategic
pressures that make the setting move.

Each actor, institution, and active population gestalt becomes a vertex in an
*agency graph*. Typed edges describe how they can affect one another: command,
membership, containment, alliance, rivalry, trade, migration, communication,
coercion, and shared location.

Each vertex also has an agency profile across six practical axes:

- geography;
- ideology;
- authority;
- economic role;
- species or bodily constraints;
- access to information.

These are not personality statistics. They are clues about where a useful
simulation boundary lies. During a blockade, geography and supply routes
predict behavior. During an election, ideology and authority matter more. For
an epidemic, bodies and transport links may dominate. During an espionage
crisis, the important boundary is often who knows what.

This is why Ghostlight does not use one permanent hierarchy such as “continent,
country, city, person.” The best cut depends on the pressure currently moving
through the world.

## From current pressure to a simulation cover

Before a strategic wave, a cheap structured model reads the committed event or
planning horizon and proposes weights for the six axes. It may also point to
relevant subjects, but only from IDs Ghostlight supplied. This stage can say
“information boundaries matter greatly right now”; it cannot invent a new
faction or alter the world.

The local algorithm then covers every active subject with a set of simulation
cells. A cover obeys three hard rules:

1. Every active subject appears in exactly one cell.
2. Every cell is connected through the agency graph.
3. Foreground subjects that require individual attention remain individual.

The player chooses the desired number of active Persona cells with a budget
slider from 1 to 32; the default is 8. This is a detail budget, not a promise to
erase people until the number fits. If nine named characters are actively
engaged while the budget is four, Ghostlight reports a temporary effective
budget of at least nine. The foreground wins.

Pins provide more deliberate control. A subject can be kept at individual
detail, several subjects can be kept together, or two groups can be forbidden
from sharing a cell. Pins and budget changes apply only at safe boundaries. They
change a separate resolution epoch, not fictional time or the campaign’s world
revision.

The partitioner starts from fine detail and repeatedly merges the lowest-cost
legal pair of adjacent cells. Its loss function asks what would become harder
to predict if those cells shared one simulation turn:

```text
0.25 facet divergence
+ 0.20 hidden causal-boundary mass
+ 0.15 information-scope divergence
+ 0.15 spatial divergence
+ 0.10 clock/obligation divergence
+ 0.10 salience burial
+ 0.05 partition churn
```

In plain language: do these subjects want different things, affect one another
in ways a summary might hide, know different facts, live too far apart, owe
different duties, contain something important, or require a disruptive change
from the previous cover?

The exact weights make the decision reproducible and inspectable. The current
pressure changes the meaning of facet divergence, so a geographic boundary can
be cheap during an ideological dispute and expensive during a flood.

After reaching the budget, Ghostlight performs a small boundary-refinement
pass. It accepts a local move only when it improves the compression cost by at
least five percent. It also prefers the previous cover unless a replacement is
at least ten percent better, apart from forced splits and explicit budget
changes. Merged cells receive short leases. These measures stop the world map
from jittering between equally plausible partitions every turn.

The implementation uses deterministic ordering and content-addressed cell IDs,
so the same state and demand produce the same cover. Candidate scoring samples
large cells to keep local work bounded, but it never samples the rules that
protect authority, hostility, knowledge, pins, connectivity, or complete
coverage. Those are checked exactly. The release test partitions 1,000 subjects
to a budget of 8 in about 50 milliseconds on the development host.

## Two very different kinds of cell

Putting several subjects in one cell does not automatically create a collective
mind. Ghostlight distinguishes two modes:

| Mode | What it represents | What it may do |
| --- | --- | --- |
| **Cohesive** | A real collective with shared authority and sufficiently compatible behavior and information | Speak or propose one action as a plural Persona |
| **Arena** | Several distinct or opposed subjects considered in one situation | Appraise their interaction and propose separately attributed actions |

A cohesive cell might represent a patrol acting through its accepted command
structure. It requires a real common authority, no active hostile relation, and
low enough behavioral and information divergence.

An arena might contain a guild, its striking workers, and a city ministry. It
exists to save inference work, not to pretend they have become one faction. An
arena has no actor ID. It cannot issue collective speech, pool everyone’s
secrets, or spend one member’s resources on behalf of another. Every proposed
action names the exact constituent responsible, and the world kernel validates
that constituent’s knowledge, authority, location, relationships, and
possessions.

Cross-faction aggregation always produces an arena. This single distinction
prevents a great deal of cheap-model political alchemy.

## One cell, one Persona pipeline

Once the cover is selected, every active cell runs through Ghostlight’s Persona
projection machinery:

```mermaid
flowchart LR
    A["Canonical world state"] --> B["Current-pressure demand"]
    B --> C["Agency graph and budgeted cover"]
    C --> D["Private Projector stream per cell"]
    D --> E["Persona response"]
    E --> F["Interpreter appraisal and proposals"]
    F --> G["WorldKernel validation"]
    G --> H["One atomic strategic commit"]
```

The Projector converts permitted typed state into a private lived narrative. A
cohesive cell receives genuinely shared state plus clearly attributed
exceptions. An arena receives a polyphonic account that keeps each
constituent’s perspective and knowledge separate.

The Persona sees only that narrative stream. It does not see database schemas,
action syntax, or raw world state. It responds as a person or group inhabiting
the situation.

The Interpreter converts the response back into a typed appraisal and action
proposals. A cohesive cell may propose one collective action. An arena may
propose a small bounded number of constituent-attributed actions—at most four,
even for a very large cell.

All cells in a wave bind to the same world revision and resolution epoch. Only
after every pipeline completes does the WorldKernel validate reach, knowledge,
authority, resources, clocks, and conflicting proposals. It then commits the
accepted consequences in one transaction. A stale response, invalid cover,
malformed model output, or illegal action aborts the entire wave without a
partial world mutation.

The number of model requests allowed to run simultaneously is a separate
operator setting. Raising concurrency makes an eight-cell wave faster; it does
not change which eight cells exist. Lowering the player’s cell budget changes
simulation resolution; it does not silently change the provider configuration.

## Keeping quiet parts of the world alive

Salience alone would still create a spotlight problem. Important conflicts
would receive attention forever while quiet populations slowly became static
summaries.

Ghostlight prevents that with *detail debt*. A subject accumulates this
non-fictional scheduling value while it is represented only inside an
aggregate. Each strategic wave reserves detailed focus for the eligible subject
with the highest debt. Receiving individual or explicit focus clears it.

Even at a cell budget of 1, the global arena must focus on one overdue subject
while retaining the larger situation around it. With N subjects, deterministic
debt rotation gives every subject direct simulation attention within at most N
strategic waves, unless foreground requirements temporarily take precedence.

Detail debt is not something characters can perceive, and it cannot be spent
inside the fiction. It is bookkeeping for fair computational attention. World
clocks and deterministic obligations advance whether a subject receives focus
or not. If a cell does nothing, that inaction must be an appraisal, not an
omission from a prompt.

## A worked example

Imagine an Aetheria campaign with 24 powers. A river blockade has interrupted
food shipments while an opposition movement tries to turn the shortage into a
general strike.

At a budget of 4, Ghostlight might preserve:

- the player’s location and directly engaged cast as mandatory individual
  cells;
- upstream producers and transport authorities in a geographically shaped
  arena;
- dockworkers, guild leadership, and the ministry in a workplace-and-authority
  arena;
- distant powers in a coarse arena, with the highest-detail-debt power selected
  for explicit focus.

If the player uncovers a secret ministry order, information divergence becomes
more important. The next safe repartition may keep the informed conspirators
separate from people who have not seen the order—even if they share a building
or faction.

If the player then confronts a particular dockworker, that character becomes a
mandatory singleton. Ghostlight may exceed the configured budget rather than
fold a live conversation back into “the workers.” After the relevance lease
expires, the character can return to aggregate simulation while retaining the
same individual history.

The cut changes because the situation changed. The world underneath it does
not.

## Refining the world is different from simulating it finely

Sometimes the Vault establishes that a broad population really contains
durable subdivisions: three ideological tendencies, several species with
different vulnerabilities, or villages on opposite sides of a material route.
That is a canonical fact, not merely a convenient simulation cut.

Ghostlight handles this through approval-gated gestalt fission. The compiler
previews a split along an enumerated facet and always includes an
`other/unknown` remainder. Approved children inherit the parent baseline and
own only their later deltas. The parent remains as inactive lineage, and known
member deltas are reassigned without rewriting anyone’s identity.

Ordinary cover changes are reversible and need no approval because they alter
no fiction. Fission changes the canonical population model, so evidence,
preview, and approval are required. Keeping these operations separate prevents
an optimization decision from quietly becoming lore.

## How it appears in Ghostlight Dungeon

For players, the main control is the active Persona-cell budget. A higher value
allows more simultaneous detail and usually costs more inference time. A lower
value asks Ghostlight to summarize more of the strategic world without freezing
it. The interface also shows the effective budget when foreground obligations
force an overage.

For operators, the inspector exposes the agency graph, current cover, cohesive
and arena modes, merge-loss components, pins, leases, detail debt, and the
reason for each cut. It also shows model-stage and resolution receipts without
publishing private Persona streams or chain-of-thought.

Under the hood, the implementation is divided by authority:

- `domain.rs` defines the persistent subjects, agency contracts, covers,
  appraisals, and control commands;
- `compiler.rs` builds the global agency skeleton and previews canonical
  fission;
- `resolution.rs` plans and validates covers, manages detail debt, validates
  cell proposals, and applies approved fission;
- `persona.rs` owns the Projector → Persona → Interpreter membrane for cells;
- `scheduler.rs` projects demand and runs cell pipelines in bounded parallel
  waves;
- `kernel.rs` is the only authority that changes canonical campaign state;
- `surface.rs` projects player and operator controls through Ghostlight’s Eve
  interface.

The result is not an omniscient simulation of every citizen. It is a controlled
allocation of attention over a persistent world. The dungeon master can zoom
in without inventing a new world and zoom out without erasing the old one.

That is the practical promise of multiresolution gestalts: the world keeps
moving, disagreement survives compression, and the blacksmith is still John
when you come back.
