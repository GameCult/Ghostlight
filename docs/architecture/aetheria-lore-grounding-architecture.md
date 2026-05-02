# Aetheria Lore Grounding Architecture

Ghostlight needs three different Aetheria modes:

- grounded historical training
- post-Rupture procedural future training
- procedural Elysium generation

Do not blur them. That is how a useful mystery becomes a filing cabinet full of
branch facts with the labels scraped off.

## Core Boundary

Planned Aetheria gameplay takes place in Elysium. What happens there is
intentionally not fully fixed in the lore vault because discovering and shaping
that history is part of the games themselves.

The Aetheria timeline begins when humanity is transplanted into Elysium. It can
restart when humanity destroys itself again through negligence, greed,
institutional capture, civilizational momentum, and the familiar psychological
trash fire wearing a crown.

That means Elysium-era gameplay scenes are not the best first source for
grounded Ghostlight training data. They are downstream procedural territory.

For grounded training data, Ghostlight should start from the authored historical
archive: late Sol, Pre-Elysium institutions, factional flashpoints, personhood
conflicts, labor regimes, political collapse, and the FTL trigger.

That does not mean Ghostlight should avoid Elysium futures. The trained models
will need to handle post-Rupture concepts: Aether, pseudospace, temporal
nonlinearity, spirits, altered substrates, necrotech, mutable bodies, and all
the new ways power learns to monetize miracles while pretending it found ethics
in a drawer.

Elysium future data belongs in a separate lane because its canon is conditional,
not because it is fake. Sol history is a single artifact. Elysium history is a
branching state-space: every possible future is canon inside the branch whose
prior conditions produced it. A generated future must therefore carry branch
lineage, dependencies, and local truth boundaries instead of being treated as
scratch material waiting for permission to matter.

## Revised Architecture

Grounded historical training flow:

```text
Aetheria authored historical lore
  -> lore grounding digest
  -> historical scene fixture
  -> agent-state fixture
  -> speaker-local input slice
  -> projection artifact
  -> prompt_text
  -> generated dialogue or drama
  -> reviewed training example
```

Procedural Elysium flow:

```text
grounded pre-Elysium priors
  -> procedural Elysium branch state
  -> Ghostlight agents and institutions
  -> projected dialogue and drama
  -> simulated branch history
  -> branch-local canonical state
```

Post-Rupture future training flow:

```text
authored Elysium concepts and constraints
  -> possible future premise
  -> procedural branch fixture
  -> coordinator plan and projected local contexts
  -> generated action, dialogue, and event consequences
  -> reviewed future-branch training artifact
  -> branch-local canonical history indexed by lineage
```

The first flow teaches the machine what grounded social pressure looks like.
The second and third flows teach it how those pressures mutate under Elysium's
stranger conditions while keeping branch-local canon separate from the Sol
artifact and from sibling Elysium branches.

## Source Priority

For projection training, prefer sources in this order:

1. Authored historical lore with explicit institutional, social, and political
   context.
2. Grounded transition material around the FTL trigger and displacement.
3. Historical fiction or scene fragments that remain canon-compatible.
4. Authored post-Elysium concept documents that define altered conditions,
   technologies, species, metaphysics, or social pressures.
5. Elysium-era procedural fixtures marked with explicit branch lineage and
   conditional truth boundaries.

Call of the Void and Cat/Oz can remain useful for testing projection mechanics,
speaker-local boundaries, and prompt shape. They should not be treated as the
primary linear-history training corpus because their facts belong to Elysium's
conditional branch space.

## Historical Flashpoint Candidates

Good first fixtures should come from parts of Aetheria that are already
pressure-dense and source-grounded:

- contract citizenship and debt regimes
- labor-system enforcement and infrastructure dependency
- upload abuse and contested personhood
- uplift or nonhuman personhood conflicts
- algorithmic governance and predictive policing
- corporate blocs turning infrastructure into leverage
- the FTL program as a political escape hatch from Sol's failure
- the moment around the FTL trigger, where institutions and people discover
  that escape did not mean absolution

The best scenes are small, local, and socially loaded:

- a worker negotiating dangerous contract terms
- an upload arguing over continuity, ownership, or service denial
- a technician asked to sign off on an unsafe FTL dependency
- a debtor deciding whether help is relief or capture
- an institutional agent performing compassion while enforcing a machine
- a family, crew, or workplace discovering that political collapse has become
  intimate

## Lore Grounding Digests

Before producing training projection examples from a historical flashpoint,
create a lore grounding digest.

The digest format is defined in:

- `docs/architecture/lore-grounding-digest-format.md`
- `schemas/lore-grounding-digest.schema.json`
- `examples/lore-grounding/historical-flashpoint.template.json`

A digest should include:

- source files and relevant line or section notes
- time period
- location or institution
- involved factions, classes, and roles
- material constraints
- local norms, taboos, and scripts
- vocabulary or speech-register cues
- what each speaker plausibly knows
- what each speaker misreads
- author-only context that must not enter prompt text
- open questions needing lore elaboration

It should also include cultural and factional pressure fields:

- internal virtues
- internal shames
- prestige markers
- taboo triggers
- emotional display norms
- trust defaults
- speech register cues
- outsider stereotypes
- factional misread patterns
- role obligations and failure modes

The digest is not a replacement for the lore vault. It is a compiled working
slice for one fixture, so projection examples can cite grounded pressure
without dragging the whole archive into every prompt.

## Fixture Types

Use explicit fixture status:

- `historical_grounded`
- `transition_grounded`
- `procedural_branch`
- `future_branch`
- `branch_lineage_canon`

`historical_grounded` and `transition_grounded` examples can become primary
single-history training data.

`procedural_branch` and `future_branch` examples are canonical inside their own
branch lineage. They are useful for testing, generation, responder training,
coordinator training, and post-Rupture concept coverage, but must not be treated
as facts that also apply to sibling branches or to the Sol artifact.

`branch_lineage_canon` examples are reviewed branch histories whose lineage,
prior conditions, and local state have been made explicit enough to serve as
stable training or gameplay continuity.

Use `future_branch` when the point of the fixture is to explore possible
post-Rupture Elysium outcomes and weird-concept behavior. Use
`procedural_branch` for ordinary gameplay-era branch fixtures whose main job is
scene/runtime testing.

## Projection Rules

Grounded projection examples should prove:

- the prompt text is self-sufficient for the dialogue model
- setting and institutional context are speaker-local
- historical pressure appears as lived constraint, not exposition paste
- the speaker does not know author-only future outcomes
- the generated behavior follows from state, culture, memory, relationship, and
  material pressure

If the lore source is too vague for a convincing scene, stop and elaborate the
lore slice first. Do not train the model to pave over missing social reality
with confident fog.

## Post-Rupture Future Branches

Future-branch generation should be deliberate, not a slush pile of glittering
possibilities with no handles.

Each possible future fixture should declare:

- which authored post-Elysium concepts it exercises
- which facts are fixed source constraints
- which facts are generated branch assumptions
- which branch facts are local to this possible future
- which prior conditions make this branch true
- which facts belong to this branch lineage and not to sibling branches
- which generated facts must not leak back into single-history Sol training
- what new social pressure the weird concept creates
- what old pre-Elysium pressure is returning in a new costume

Good future branches should stress the model with conditions it cannot learn
from late-Sol history alone:

- Aether as infrastructure, ecology, religion, weapon, medicine, or status good
- pseudospace as transit, concealment, displacement, exile, or labor terrain
- temporal nonlinearity as memory, debt, evidence, grief, insurance, or law
- spirits as persons, tools, contaminants, witnesses, ancestors, or frauds
- necrotech as care, extraction, continuity, punishment, or labor capture
- mutable bodies and species boundaries as liberation, class marker, liability,
  taboo, or supply chain
- post-Rupture institutions rebuilding old hierarchies with new miracles

These branches should feed:

- Aetheria responder training, so the model learns post-Rupture assumptions
- coordinator training, so the story runtime can handle weird scene premises
- appraiser and relationship training, so characters misread new phenomena in
  socially grounded ways
- institution/faction/consumer training, so economic and political behavior can
  respond to miracles as markets, threats, privileges, and dependencies

Keep source provenance strict. A branch can invent a local future, but it should
say what it invented and what branch conditions make it true. The machine may
generate futures under supervision; it does not get to erase the branch index
while everyone is looking at the shiny ghost.

## Operating Doctrine

Ghostlight will generate a lot of Aetheria dialogue, branching scenes, and
training data. That means it will also be the first tool to stress-test many
timeline details at room scale. Treat that as part of the job, not an
exception.

Before writing an Aetheria scene or generating data from one, do a source pass:

1. Check AetheriaLore for the factions, social movements, institutions, species
   or body types, location, and time period present in the scene.
2. Extract the material constraints that shape behavior: habitat form,
   infrastructure, law, money, route access, labor pressure, class position,
   surveillance, body affordances, communication channels, and local failure
   costs.
3. Distill those details into the lore grounding digest, agent-state fixture,
   projected local context, or Ink sidecar before prompting a response model.
4. Keep the prompt character-local. Source grounding tells Ghostlight what the
   room and culture make possible; it does not grant characters omniscience.
5. If the lore does not support a needed concrete detail, do not fill the gap
   with confident furniture. Mark the gap, elaborate the AetheriaLore source,
   then regenerate or revise the Ghostlight artifact from the patched source.
6. Commit the lore patch in AetheriaLore and cite the source path from
   Ghostlight so the training artifact has a clean provenance trail.

The output goal is not only clean training data. It is also playable, readable,
fun interactive fiction that discovers where the worldbuilding is thin by
leaning on it. If Ghostlight needs to know what a sanctuary intake board looks
like to a cetacean Navigator, that is not trivia. That is worldbuilding under
load.

## Lore Gap Policy

When a gap appears, classify it before writing:

- `blocking`: generation would produce wrong bodies, wrong institutions, wrong
  authority, or wrong material constraints without the missing detail.
- `safe_to_stub`: the scene can proceed with a clearly marked provisional
  detail that does not affect canon-critical behavior.
- `defer`: the detail is decorative and should not enter the prompt yet.

Blocking gaps should be patched in AetheriaLore first. Safe stubs must be
tracked in the digest or sidecar and revisited before promotion. Deferred gaps
should stay out of prompt text so they do not harden by accident.

## Immediate Plan

1. Keep the Cat/Oz fixture as an Elysium procedural mechanics fixture.
2. Add a lore grounding digest format. Done.
3. Choose one authored historical flashpoint from the Aetheria archive.
4. Build the first `historical_grounded` agent-state fixture from that slice.
5. Produce reviewed projection examples from that fixture.
6. Create the first `future_branch` fixture from authored post-Elysium concepts
   after the coordinator artifact schema exists.
7. Compare those examples against Cat/Oz to separate grounded historical
   pressure from procedural Elysium branch generation.

Ground the machine in the past. Then generate possible futures under
supervision, with branch labels sharp enough that infinity does not show up
wearing one nametag.
