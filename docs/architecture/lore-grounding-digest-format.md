# Lore Grounding Digest Format

The lore grounding digest is the bridge between authored Aetheria history and
Ghostlight projection training.

It distills a specific historical flashpoint into the cultural, institutional,
material, and factional pressures needed to build agent-state fixtures and
speaker-local projection prompts.

It is not a lore replacement. It is a working slice. Do not ask every prompt to
carry the whole vault in its teeth like an exhausted dog.

## Files

The schema lives at:

- `schemas/lore-grounding-digest.schema.json`

The starter template lives at:

- `examples/lore-grounding/historical-flashpoint.template.json`

Use the template for the first authored historical flashpoint, then save real
digests under `examples/lore-grounding/` with names tied to the flashpoint.

## Purpose

A digest should answer:

- What authored source material grounds this flashpoint?
- What history, institutions, factions, classes, roles, and material pressures
  shape the scene?
- What cultural priors should affect how characters appraise events?
- What does each faction or class reward, shame, taboo, admire, and misread?
- What should each speaker know, believe, misread, or not know?
- What pressure should appear in `prompt_text`, and what should remain
  author-only?

## Main Sections

### Source Refs

Use `source_refs` to cite the lore archive slices that constrain the digest.
References should be specific enough that a later reviewer can find the source
without spelunking through the entire vault with a dim headlamp and a worsening
personality.

### Historical Context

This section describes the time, place, public facts, and ambient pressures of
the flashpoint.

Good context is small and hard-edged:

- a clinic policy dispute
- a contract negotiation
- an upload continuity hearing
- an unsafe FTL signoff
- a workplace discipline meeting
- a factional aid intervention

Do not start with "the whole setting." Start with one room where history has
hands.

### Groups

`groups` describes factions, institutions, classes, professions, movements, or
subcultures active in the scene.

The group profile is where broad lore becomes usable social pressure:

- `institutional_logic`: what the group exists to preserve, control, repair, or
  extract
- `material_incentives`: what the system rewards the group for doing
- `internal_virtues`: what insiders admire
- `internal_shames`: what insiders experience as disgrace
- `prestige_markers`: what reads as competence, status, holiness, care, or
  discipline
- `taboo_triggers`: what provokes disgust, sanction, fear, or exclusion
- `emotional_display_norms`: what feelings can be shown, hidden, ritualized, or
  punished
- `trust_defaults`: who the group tends to trust, distrust, pity, envy, use, or
  fear
- `speech_register_cues`: how role and culture shape language

This is where "Dominion types read as cold" becomes something more useful than
a hat with a sneer glued to it.

### Cultural Pressure Fields

`cultural_pressure_fields` are reusable pressure atoms derived from lore.

They should not say:

```text
Dominion people are cold.
```

They should say something closer to:

```text
In this Dominion-coded role, visible personal need reads as loss of command.
Under status threat, the speaker is likely to compress emotion into procedure.
```

Pressure fields can represent:

- virtue
- shame
- prestige
- taboo
- threat model
- trust default
- speech norm
- status trigger
- material incentive
- outsider stereotype

### Factional Misread Matrix

The misread matrix models how groups translate each other's virtues into
insults.

Example shape:

```text
Observer: Dominion-coded institution
Target: Aya-coded care network
Observer read: sentimental, porous, slow to impose hard necessity
Target self-read: durable care infrastructure, consent discipline, refusal to
turn people into convenient losses
Projection use: Under conflict, Dominion-coded speakers may frame Aya restraint
as weakness; Aya-coded speakers may frame Dominion decisiveness as moral
amputation.
```

The point is not to canonize every stereotype as true. The point is to make
misperception available as social fuel.

### Role Pressure Fields

Roles are not the same as factions. A Dominion auditor, a Dominion heir, and a
Dominion maintenance officer may share cultural priors while facing different
obligations and failure modes.

Role fields should describe:

- obligations
- available masks
- failure modes
- what authority looks like
- what disgrace looks like
- what compromises the role can justify

### Speaker Lore Boundaries

This section controls leakage.

For each speaker, define:

- known context
- beliefs
- misreads
- must-not-know facts

If a speaker would not know a fact, it does not enter their prompt text. If a
speaker suspects something, encode it as a belief or misread with source
grounding. No little omniscient narrator gremlin in the vents.

### Author-Only Boundaries

Author-only boundaries constrain the fixture or future continuity but do not
enter speaker prompts.

Examples:

- future outcomes
- hidden institutional motives
- another speaker's private state
- facts known only to the author or simulation supervisor
- Elysium branch outcomes not yet promoted

### Open Questions

If the lore source is too thin, say so. Mark whether the question blocks
training.

Do not generate training data to cover a hole that should be filled by lore
elaboration. That is not creativity; that is drywall over a load-bearing ghost.

## Dominion And Aya Pressure Examples

These are examples of digest structure, not final faction doctrine.

### Dominion-Coded Pressure

Likely ingredients to verify against source:

- administrative continuity
- hierarchy
- AGI governance
- industrial planning
- duty under collapse
- controlled emotional display
- legitimacy through competence and command

Projection shape:

```text
The speaker is Dominion-coded and under status threat. They are likely to turn
distress into procedural language because visible need reads as command leakage.
They may experience compassion as legitimate only when subordinated to duty,
continuity, or strategic necessity.
```

Outsider misreads:

- cold
- arrogant
- authoritarian
- morally vacant
- inhumanly procedural

### Aya-Coded Pressure

Likely ingredients to verify against source:

- care as material politics
- sanctuary
- maintenance
- consent discipline
- expansive personhood
- refusal to solve suffering by writing people out of the ledger

Projection shape:

```text
The speaker is Aya-coded and facing coercive efficiency. They may treat care as
infrastructure rather than sentiment. They are likely to resist decisions that
make personhood conditional on convenience, even when outsiders frame that
resistance as softness.
```

Outsider misreads:

- weak-hearted
- sentimental
- indecisive
- inefficient
- unable to do what reality demands

## Promotion Rule

A digest becomes usable for grounded training when:

- source refs are specific
- major cultural pressure fields are filled
- factional misreads are explicit
- speaker knowledge boundaries are clear
- open questions do not block the scene
- a reviewer agrees the digest reflects authored lore rather than attractive
  fog

Only then should it feed an agent-state fixture or projection example.
