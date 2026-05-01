# Canonical Agent State Schema

Ghostlight's first runtime organ is a state contract, not a generator.

The schema lives in:

- `schemas/agent-state.schema.json`
- `schemas/agent-state.required-fields.json`

The first example lives in:

- `examples/agent-state.call-of-the-void.json`

The label and numeric glossary lives in:

- `docs/architecture/agent-state-variable-glossary.md`

## Objective

The schema defines the minimum state Ghostlight needs before it can safely build
prompt projection, dialogue scaffolding, classifier records, or procedural
drama.

The core rule is unchanged:

- canonical state is what the world treats as true
- perceived state is what a specific observer thinks is true
- dialogue context packs are character-local projections for one speaker in one
  scene

If these layers collapse into each other, the machine turns back into one
omniscient narrator with a pocket full of vibes. This is a known industrial
hazard. We are wearing goggles now.

## State Families

Each agent has:

- `identity`
- `canonical_state`
- `goals`
- `memories`
- `perceived_state_overlays`

`canonical_state` is split into:

- `underlying_organization`
- `stable_dispositions`
- `behavioral_dimensions`
- `presentation_strategy`
- `voice_style`
- `situational_state`
- `values`

Each canonical scalar-like dimension is represented as:

```json
{
  "mean": 0.5,
  "plasticity": 0.5,
  "current_activation": 0.5
}
```

This keeps baseline, change rate, and scene activation separate.
A person can be dispositionally suspicious without being maximally suspicious
in every scene. A person can be highly activated right now without rewriting
their whole soul because someone coughed near a status wound.

Canonical variables do not include confidence or certainty. Perceived and
inferred variables use `confidence` when a read can be wrong.

For full meanings of `mean`, `plasticity`, `current_activation`, perceived
`confidence`, and every required v0 label, see
`docs/architecture/agent-state-variable-glossary.md`.

## Required First-Class Dimensions

`schemas/agent-state.required-fields.json` names the required core variable
families. The validator enforces those fields for canonical agent state and
relationship stance.

The required core is not exhaustive. It is the shared comparison surface. A
character fixture currently includes all required labels, even when many are low
or inactive, so downstream tooling can compare agents without guessing whether a
missing value means "not applicable," "forgotten," or "quietly eating the
floorboards."

The behavioral dimensions include the known must-not-drop set:

- `volatility`
- `attachment_seeking`
- `distance_seeking`

Those are not garnish. They are load-bearing social handles.

## Presentation Strategy Vs Voice Style

`presentation_strategy` describes the self being performed.

That can include charm, detachment, competence theater, cultivated
harmlessness, or deliberately performing someone who is abrasive or difficult.
Sometimes that abrasive performance is not random meanness. It is a boundary,
a threat display, a way to prevent intimacy, or a test to see who flinches.
Human beings are regrettably efficient little porcupines.

`voice_style` describes how that performed self comes out in language.

For Cat, the important voice handles are explicit now:

- `dryness`
- `sarcastic_deflection`
- `pointedness`
- `sincerity_evasion`
- `emotional_evasion`
- `banter_as_boundary`
- `verbal_aggression`
- `plainspoken_directness`

This list is not comprehensive. It is a v0 starter set. Future labels will
likely need to cover formality, verbosity, lyricism, hedging, ritual politeness,
dialect, register switching, technical compression, and other language habits.

So Cat's prickliness is not only a dialogue prompt instruction. It is backed by
canonical state:

- high `presentation_strategy.abrasive_boundary`
- high `presentation_strategy.ironic_distance`
- high `voice_style.sarcastic_deflection`
- high `voice_style.sincerity_evasion`
- high `voice_style.emotional_evasion`

The renderer can then translate that state into lines that dodge sincerity with
pointed sarcasm without making her cruel at random. The goal is not "be snarky."
The goal is "protect the soft tissue by making the other person deal with the
spines first."

## Perceived State

Perceived overlays are observer-specific. They do not replace canonical state.

An overlay records:

- observer
- target
- perceived dimensions
- beliefs
- distortions

This is where misunderstanding lives. Suspicious agents can overread threat.
Attachment-hungry agents can overread warmth. Avoidant agents can underread
care. None of that requires corrupting the canonical state.

## Dialogue Context Packs

Scenes contain `dialogue_context_packs`.

A pack is a character-local prompt payload for one speaker. It includes:

- speaker-local truth
- speaker beliefs
- active memories
- active goals
- presentation constraints
- forbidden context

The point is not to make the dialogue generator smarter by shouting plot at it.
The point is to keep each character inside their own partial reality.

Ghostlight can build the trap. The character projection decides how the person
inside the trap breathes.

## Current Validation

The first validator is intentionally small and dependency-free:

- `tools/validate_agent_state.py`

It does not implement all of JSON Schema. It checks the invariants Ghostlight
currently cares about:

- example files parse as JSON
- schema versions match
- agent IDs are unique
- required canonical variable groups are present
- required relationship stance fields are present
- state variable components stay in `0..1`
- goals, memories, relationships, scenes, and dialogue context packs reference
  existing IDs
- dialogue packs do not leak obvious forbidden context into speaker-local truth

This gives us a real seam before we build the prompt renderer. Not perfect.
Useful. We are allowed to be both.
