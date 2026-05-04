# Projected Local Context

The projected local context is the missing seam between canonical Ghostlight
state and a character agent.

Canonical state is storage. It can contain means, plasticity, activation,
relationship stance, memories, culture, goals, and author-facing source paths.
A character response model should not have to interpret that machinery. It
should receive a compact operating context: what this character knows, wants,
misreads, can do, sounds like, and must not resolve.

## Contract

Pipeline:

```text
canonical state + scene + reviewed annotation
  -> projector/state interpreter
  -> ghostlight.projected_local_context.v0
  -> response-model prompt
  -> generated action proposal
```

The projected context may retain source references for audit. The rendered
prompt text must not expose raw state internals such as `current_activation`,
`plasticity`, means, or selected numeric dimensions.

## Artifact Shape

Schema: `schemas/projected-local-context.schema.json`

Validator: `tools/validate_projected_contexts.py`

Renderer: future projector/state-interpreter implementations should emit this
schema before any character responder sees the scene. Historical prototype
renderers have been removed from the live workspace after their lessons were
folded into this contract.

The context contains:

- speaker identity and setting
- known facts and speaker beliefs
- current stakes
- embodiment and interface affordances
- active inner pressures rendered as ordinary language
- relationship read and known biases
- tensions
- runtime retrieval requirements
- latent pressure requirements
- action affordances
- projection controls
- voice surface
- likely response moves
- do-not-invent boundaries
- rendered prompt text

## Retrieval And Latent Pressure

The projector now emits two explicit requirement lists before the responder
packet is built.

`runtime_retrieval_requirements` are compact lore facts the runtime should
include when their trigger is scene-relevant. They are not a license for a
responder to browse the lore archive. They are coordinator or retriever outputs
with source refs and a prompt role. A Pallas labor scene might require AU
claimshare hierarchy, uplift liability framing, species affordances, Bloom
service-ring geometry, and strike-law pressure before a responder can act
without drifting into generic sci-fi wallpaper.

`latent_pressure_requirements` are character-local history or emotional pressure
that must remain visible to the responder because it can shape behavior. They
are not dialogue requirements. Sella's memory of a sanctuary overpromising
safety should bias her toward capacity suspicion, condition-setting, and hard
triage boundaries. It should not make her announce the memory in the middle of
a mission-critical exchange unless the scene genuinely asks for that.

The rendered prompt includes these as:

- `Retrieved Lore To Keep In View`
- `Latent Pressure Handling`

The responder packet also copies source-excerpt retrieval requirements into its
`source_excerpts`, so packet-only tests can evaluate the same bounded context a
runtime responder would receive.

## Runtime Boundary

The character model receives local operating context, not omniscient truth. If
Maer acts, Sella's next pass receives only the observable event and Sella's own
local context. Maer's private intent is not included.

The sequential runtime still owns:

- tool contracts
- action enums
- event proposal capture
- participant appraisal
- reviewed mutation
- branch compiler / Ink materialization

The projector owns the translation from Ghostlight's internal state language to
character-local operating prose.

## Character Cognitive Ceiling

The response model may be very capable. The character should not inherit that
capability by default.

Projection must carry a character-local cognitive ceiling: how quickly the
character infers, how much social indirection they notice, how well they
manipulate attention, how abstractly they reason under stress, what education or
culture has trained them to see, and where their blind spots live. A Lucent
crisis professional may operate with alarming metacognitive precision because
that is part of the institution's ecology. A tired dock worker, sheltered child,
panic-struck survivor, credulous fan, or blunt military specialist should not
suddenly perform the same level of social chess just because the underlying
model can.

The projector should therefore render:

- strategic acuity and social-reading ability
- education, professional training, and cultural scripts
- stress effects on reasoning
- blind spots, taboos, and habitual misreads
- whether the character can explain their own tactic, only feel toward it, or
  act it out without understanding it

This is separate from narrator intelligence. A coordinator or narrator can help
the reader understand a maneuver after it happens. That explanatory prose must
not be smuggled into the character's private state or future action capacity
unless reviewed mutation says the character actually learned it.

## Current Limit

The first projector is deterministic and deliberately simple. It uses threshold
language like "dominant", "strong", "present", and "background" rather than
numeric scores. This is enough to remove raw state soup from responder prompts and
create inspectable training artifacts.

Later versions can become smarter, but they should still preserve this boundary:
response models act from projected local context; they do not become ad hoc
interpreters of the canonical schema.

## Embodiment Boundary

Embodiment is part of the prompt surface, not flavor garnish. If a character is
a cetacean Navigator, the response model must know what kind of body is acting
and what infrastructure surrounds it.

For Aetheria factions, project faction-specific affordances only after checking
the lore source. The current Navigator projection is grounded in:

- `E:\Projects\AetheriaLore\Aetheria\Worldbuilding\Pre-Elysium\Factions\Powers\Major\Cetacean Navigators.md`
- `E:\Projects\AetheriaLore\Aetheria\Worldbuilding\Pre-Elysium\Technology\Uplift.md`

Those sources support fluid architecture, acoustic signaling, soft navigation
light, layered water motifs, communal orientation chambers, mixed-species
translation/coexistence, specialized environments for cetacean physiology, and
mixed wet/dry interface ergonomics for Navigator-adjacent habitats.

Do not let the model default to hands, pockets, standing, walking, leaning, or
doorway blocking unless the fixture explicitly establishes that visible
interface. Do not patch over missing lore with confident furniture.
