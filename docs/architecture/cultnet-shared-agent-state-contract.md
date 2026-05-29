# CultNet Shared Persona-State Contract

Ghostlight no longer treats `ghostlight.agent_state.v0` as the cross-runtime
person-state contract. That schema still belongs here, but it owns scene,
world, relationship, event, and dialogue-context state for storytelling.

The portable person-shaped contract is now `gamecult.persona_state.v0`.

## Canonical Payload

The shared payload contract is:

- schema id:
  `https://gamecult.dev/cultnet/gamecult.persona_state.v0.schema.json`
- schema version:
  `gamecult.persona_state.v0`
- Ghostlight mirror:
  `schemas/gamecult.persona_state.v0.schema.json`

The same schema is mirrored by Epiphany and VoidBot so Faces, repo
representatives, and Ghostlight characters can cross runtime boundaries without
each repo inventing its own local person-shaped blob.

## Ownership

Ghostlight keeps owning the story machine:

- world state
- scenes
- events
- relationships
- perceived overlays
- dialogue context packs

PersonaState owns the portable public-person projection:

- presentation
- values
- activation profile
- thought memory
- agency pressure
- candidate actions
- affect
- source provenance

That projection can be generated from Ghostlight scene agents, VoidBot repo Face
state, or Epiphany Face state. The projection does not erase native state. It is
the wire shape, not a throne.

## Wire Envelope

CultNet should carry PersonaState as a typed document replication message.

Current expected document envelope:

- `documentType`: `gamecult.persona-state`
- `documentKey`: stable runtime-defined persona key
- `payloadSchemaVersion`: `gamecult.persona_state.v0`
- `payload`: exact PersonaState document

Native Ghostlight scene payloads may still travel separately with:

- `documentType`: `ghostlight.agent-state`
- `payloadSchemaVersion`: `ghostlight.agent_state.v0`

Those are different contracts. One carries a story world. One carries a
portable Persona.

## Epiphany Boundary

Epiphany public Faces use PersonaState when they need full persistent
personhood. Epiphany work organs do not. A work organ such as Imagination can
have a light organ state without affect, social bonds, public presentation, or
candidate actions.

This prevents the old muddle where every useful subsystem was tempted to dress
up as a whole person. Useful machines do not all need faces.

## Contract Discipline

The schema lives in multiple repos for local validation, but the contract must
move as one:

- Epiphany keeps the primary CultNet schema copy.
- VoidBot mirrors it and emits PersonaState from repo Face reads.
- Ghostlight mirrors it and validates PersonaState examples.
- Native storage may stay richer than PersonaState.
- Importers must preserve unknown extension data when possible.
- Consumers must treat `provenance.authority` seriously.

Prompt projection may differ. Storage engines may differ. Story scenes may
carry more structure. The shared person-state payload does not drift just
because a runtime got clever at midnight.

## Immediate Consumers

The first intended consumers are:

- Ghostlight character projections for scene/story tooling
- VoidBot repo Face MCP reads and social memory interop
- Epiphany public Face state
- future CultNet replication and inspection tools
