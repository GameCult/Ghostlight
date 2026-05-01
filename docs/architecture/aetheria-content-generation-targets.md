# Aetheria Content Generation Targets

This is the first concrete target note for applying Ghostlight to Aetheria.

The build target is not a standalone corridor simulation or local crisis game
slice. That older shape was fertile story terrain, but it is not the current
product direction. Do not let it shape the runtime unless the user explicitly
revives it.

Ghostlight should first serve two Aetheria uses:

1. content generation inside the **Call of the Void** slice described in the
   `AetheriaLore` repository
2. dialogue scaffolding and procedural drama for narrated stories across the
   Aetheria timeline

The goal is a story and content-generation organ: a machine that can hold
characters, factions, pressures, memories, masks, and conflict arcs in explicit
state, then project that state into usable scenes, dialogue beats, outlines,
and dramatic complications.

## Primary Use: Call Of The Void Content

Call of the Void should be treated as the first consumer context for Ghostlight,
not as an excuse to build a full social MMO simulation immediately.

The local source of truth is:

- `E:\Projects\AetheriaLore\Aetheria\Game Design\Call of the Void.md`
- `docs/aetheria/call-of-the-void-brainstorm.md` for raw imported brainstorm
  material

The slice is a story-first Aetheria scope cut about Catastrophe "Cat" Marrigan:
an out-of-work private investigator making ends meet with a failure-prone space
taxi while people trapped inside Elysium slowly realize the old world is not
coming back intact.

Its useful shape is authored density:

- episodic investigations
- taxi fares as social x-rays
- station districts
- recurring characters
- faction contacts
- campaign consequence
- the social aftermath of return becoming fantasy

Ghostlight should help generate and manage:

- character dossiers
- faction stances
- relationship tensions
- scene premises
- dialogue scaffolds
- procedural complications
- continuity-aware reactions
- memory-shaped follow-up scenes
- local cultural and institutional pressure
- case premises and clue chains
- passenger encounters that reveal denial, opportunism, homesickness, fear, or reckless celebration
- continuity pressure from the sealed-domain premise

The important thing is that generated content should emerge from structured
state instead of a prompt asking for "dramatic dialogue" and hoping the machine
does not start juggling knives in the pantry.

## Secondary Use: Narrated Timeline Stories

Ghostlight should also support story generation across Aetheria's broader
timeline.

That means it should be able to produce:

- scene outlines
- relationship escalation chains
- factional pressure maps
- alternate dialogue passes
- conflict beats
- character-specific reactions to historical events
- continuity summaries for later story work

This does not require every story to run as a live simulation. Some outputs can
be generated as authoring scaffolds: structured suggestions, scene cards,
argument maps, motive webs, and dialogue skeletons for a human author to shape.

## What Ghostlight Should Build For This

The first useful organs are:

1. canonical agent state schema
2. perceived-state overlay schema
3. relationship and faction stance records
4. event and scene records
5. prompt projection renderer for content generation
6. drama beat generator driven by goals, values, pressure, and memory
7. dialogue scaffold renderer that preserves stance, mask, and relationship
   asymmetry

The output should be inspectable. The author should be able to see why the
machine thinks a character would deflect, confess, threaten, soften, bargain,
or lie.

## What This Is Not

This is not yet:

- a standalone corridor crisis game
- a persistent corridor simulator
- a full MMO social runtime
- a requirement to simulate every offscreen person
- a replacement for authorial judgment

Ghostlight is an authoring and social-state engine first. The live game-shaped
organs can come later if they earn the trouble.

## Success Criteria

The first implementation is useful if it can:

- keep character state distinct from prompt prose
- generate dialogue scaffolds that reflect relationship history
- create procedural drama from incompatible goals instead of random hostility
- preserve cultural and factional pressure without making clones
- produce story beats a human author can actually use
- explain its choices in state terms rather than vibes in a rented coat

The real test is not whether the system can improvise a cool scene once. The
real test is whether it can help generate the tenth scene without forgetting
what the first nine did to everyone involved.
