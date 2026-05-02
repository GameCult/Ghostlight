# Future Schema Mechanisms

This note parks the useful pressure from `notes/evaluation.md` without treating
that critique as doctrine. The current v0 schema is still a character-state and
prompt-projection seam. The future runtime needs mechanisms that turn state into
changing social behavior.

## Current Boundary

The v0 agent-state schema should stay focused on:

- canonical character state
- directional relationship stance
- observer-local perceived overlays
- memories, goals, values, and scene-local dialogue packs
- enough event interpretation to support prompt projection

Do not inflate `schemas/agent-state.schema.json` into the whole simulator before
the projection examples prove what the runtime actually needs.

## Mechanisms To Preserve For Later

### Event Appraisal

Each participant needs a private read of the same event. A single exchange can
be betrayal to one agent, exposure risk to another, and flirting-with-knives to
a third. The current schema now allows `private_interpretations` on events as a
lightweight bridge.

Future work should make appraisal reusable enough to drive state updates,
memory retrieval, and action affordances.

### State Transitions And Consequences

`mean`, `plasticity`, and `current_activation` describe state. They do not yet
define how state changes. Future transition machinery needs:

- activation decay
- event-driven deltas
- sensitization or habituation
- effects on related variables
- rules for when `mean` changes versus only `current_activation`

The current `event_effects` field is a projection-facing placeholder, not the
final transition engine.

### Structured Beliefs And Secrets

String beliefs are useful for authoring, but simulation needs structure:

- holder
- subject
- claim type
- confidence
- evidence
- visibility
- emotional charge
- whether the belief is private, shared, public, rumor, false, or contested

Secrets need similar structure: owner, known-by, suspected-by, exposure cost,
protective strategies, and what can leak through action.

### Action Affordances

The system needs to choose among possible moves instead of jumping from state to
dialogue. Future projection examples should expose candidate affordances such
as probing, evading, bargaining, softening, escalating, repairing, threatening,
or walking away.

The important rule: affordances should be justified by goals, appraisals,
capabilities, costs, relationship stance, and scene pressure.

### Conversation State

Dialogue needs explicit continuity:

- active topic
- open questions
- evasions
- accusations
- face-threats
- commitments
- promises, refusals, and ruptures
- who has the floor

This belongs close to projection because it changes turn shape immediately.

### Resources And Capabilities

Cat's debt, taxi condition, mobility, and institutional exposure are not just
backstory. Oz's drone integrity, signal secrecy, anonymity, and physical safety
are not just flavor. Future schemas need concrete resource and capability
fields so desire does not masquerade as ability.

### Dyads And Shared Relationship History

Directional relationship stance should remain. A separate dyadic layer may also
be needed for shared history:

- relationship type
- phase
- shared events
- shared secrets
- unresolved ruptures
- rituals
- dependency structure
- communication pattern

Do not collapse this into one symmetric relationship object. People are not
that tidy unless the author is lying to save RAM.

### Social Knowledge, Reputation, And Institutions

The Aetheria setting needs institutions with teeth: sanctions, authority,
surveillance, reputation effects, access control, role expectations, and public
knowledge. Future work should distinguish private belief, shared belief, public
fact, rumor, common knowledge, and institutional record.

### Memory Retrieval

Memories need more than salience and confidence once the runtime starts moving:

- recency
- emotional charge
- retrieval triggers
- decay
- reconsolidation
- intrusive, avoided, cherished, rehearsed, or distorted status
- schema effects
- whether another agent knows the memory

The current memory bundle is enough for the first projection seam. It is not yet
the memory engine.

### Embodiment And Nonverbal Channels

Oz proves this one immediately. The operator, drone body, observable channels,
leakage risks, and physical affordances are separate facts. Future projection
needs embodiment because social meaning leaks through movement, timing,
silence, distance, posture, and constrained bodies.

The projected-local-context seam now carries a first embodiment/interface
surface, but the canonical schema still lacks a proper reusable embodiment
model. Future work should promote body form, native habitat, movement channels,
assistive or translation infrastructure, and visible nonverbal signal channels
only after more projection examples prove the required shape.

## Promotion Rule

Promote a future mechanism into the main schema only after it has survived at
least one reviewed projection example or runtime slice where it clearly changes
the output. Until then, document it here, test it in examples, and do not let
the ontology grow antlers for sport.
