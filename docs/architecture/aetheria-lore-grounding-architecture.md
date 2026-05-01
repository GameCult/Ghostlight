# Aetheria Lore Grounding Architecture

Ghostlight needs two different Aetheria modes:

- grounded historical training
- procedural Elysium generation

Do not blur them. That is how a useful mystery becomes accidental canon with a
badge and a clipboard.

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
  -> optional promoted canon after review
```

The first flow teaches the projector what grounded social pressure looks like.
The second flow uses that learned machinery to generate Elysium outcomes
without pretending those outcomes were already settled lore.

## Source Priority

For projection training, prefer sources in this order:

1. Authored historical lore with explicit institutional, social, and political
   context.
2. Grounded transition material around the FTL trigger and displacement.
3. Historical fiction or scene fragments that remain canon-compatible.
4. Elysium-era procedural fixtures marked as provisional branch material.

Call of the Void and Cat/Oz can remain useful for testing projection mechanics,
speaker-local boundaries, and prompt shape. They should not be treated as the
primary grounded training corpus unless the relevant Elysium branch facts are
intentionally authored or promoted.

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
- `promoted_branch`

`historical_grounded` and `transition_grounded` examples can become primary
training data.

`procedural_branch` examples are useful for testing and generation but should
not train the projector as if they were fixed lore.

`promoted_branch` examples are procedural outputs that have been reviewed and
accepted into canon or into a specific authored continuity.

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

## Immediate Plan

1. Keep the Cat/Oz fixture as an Elysium procedural mechanics fixture.
2. Add a lore grounding digest format. Done.
3. Choose one authored historical flashpoint from the Aetheria archive.
4. Build the first `historical_grounded` agent-state fixture from that slice.
5. Produce reviewed projection examples from that fixture.
6. Compare those examples against Cat/Oz to separate grounded historical
   pressure from procedural Elysium branch generation.

Ground the machine in the past. Let it hallucinate the future under supervision.
