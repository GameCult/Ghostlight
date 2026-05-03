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

That organ must also hold tone. Aetheria should not be flattened into one dry
technical crisis register. The setting can carry Adams/Pratchett-style
wit-with-stakes, domestic comedy, noir suspicion, horror, wonder, dour poetry,
romance, workplace absurdity, and procedural systems prose, often inside the
same broad canon. The common rule is not "be grim." The common rule is that
tone must reveal character, setting, and consequence.

## Why This Helps Dialogue

Ghostlight should not ask one LLM prompt to "write good character drama" from
vibes. That path produces clever paste: fluent, energetic, and usually a little
dead behind the eyes.

The useful split is:

- Ghostlight builds the drama, narrative pressure, scene setup, and state.
- The dialogue renderer gives each character a constrained local context.
- The character speaks from who they are, what they have endured, what they
  want, what they are feeling, what they are hiding, and how they perceive the
  person in front of them.

This does not mean every line should sound like emergency procedure. Characters
also speak from routine, fatigue, affection, boredom, private jokes, little
status games, cultural rituals, and ordinary work. Those are not filler; they
are what make later pressure legible.

That means dialogue prompts should be character-local rather than omniscient.
They should include only the state that character plausibly has access to:

- identity and role
- current mood and pressure
- relevant memories
- active goals
- relationship stance toward the listener
- perceived stakes
- cultural or factional scripts
- mask, concealment, or presentation strategy

This lets characters breathe and act inside the scene instead of serving as
mouthpieces for a single plot-generating agent. The agent can shape the drama.
The character projection should preserve subjectivity.

## Primary Use: Call Of The Void Content

Call of the Void should be treated as the first consumer context for Ghostlight,
not as an excuse to build a full social MMO simulation immediately.

The local source of truth is:

- `E:\Projects\AetheriaLore\Aetheria\Game Design\Call of the Void.md`
- `docs/aetheria/call-of-the-void-brainstorm.md` for raw imported brainstorm
  material

The slice is a story-first Aetheria scope cut about Catastrophe "Cat" Marrigan:
an out-of-work private investigator making ends meet with a failure-prone space
taxi inside the aftermath of Sol's sequestration into Elysium.

The older brainstorm frames the core twist as a colony fleet or exploratory
force discovering that nobody is going home. That premise is obsolete. Current
Aetheria canon says the FTL trigger displaced the whole Solar system into the
sealed spacetime domain later called Elysium. Home did not stay behind. Home,
with every corporate bloc, labor regime, upload abuse, political failure, and
unresolved compromise, came through too.

So the active Call of the Void pressure should be refactored from "there is no
return ticket" into something meaner and more useful: there is no outside, no
clean restart, and no frontier innocent of Sol's history. Cat can still try to
outrun debt, coercion, legal pressure, or institutional rot, but Elysium proves
that escape exported the machinery instead of dissolving it.

Its useful shape is authored density:

- episodic investigations
- taxi fares as social x-rays
- station districts
- recurring characters
- faction contacts
- campaign consequence
- the social aftermath of realizing Sol's old failures are now Elysium's local
  weather

Ghostlight should help generate and manage:

- character dossiers
- faction stances
- relationship tensions
- tonal mode and style targets
- ordinary-life texture before conflict
- scene premises
- dialogue scaffolds
- procedural complications
- continuity-aware reactions
- memory-shaped follow-up scenes
- local cultural and institutional pressure
- case premises and clue chains
- passenger encounters that reveal denial, opportunism, homesickness, fear, or reckless celebration
- continuity pressure from the sealed-domain premise
- cases that expose how old Sol institutions mutate when there is no larger
  outside to appeal to

The important thing is that generated content should emerge from structured
state instead of a prompt asking for "dramatic dialogue" and hoping the machine
does not start juggling knives in the pantry.

## Secondary Use: Narrated Timeline Stories

Ghostlight should also support story generation across Aetheria's broader
timeline.

The existing Aetheria stories are tonal evidence. `When We Get Home` makes the
setting legible through jokes, domestic longing, debt, delays, and the slow
recognition that home has moved. `Rain` makes the setting legible through
ritual, memory, sensory repetition, and a patient reversal from wonder to
horror. Future fixtures should be able to choose similarly varied modes instead
of filing every event into one stern technical drawer.

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
6. coordinator/story runtime driven by goals, values, pressure, memory,
   continuity, branch flags, and author constraints
7. branch compiler that turns reviewed branch plans into Ink, sidecars,
   callbacks, consequence variables, and visual prompt handles
8. IF artifact reviewer that catches decorative choices, fake folds, unused
   variables, and missing consequence callbacks
9. dialogue scaffold renderer that preserves stance, mask, and relationship
   asymmetry
10. character-local dialogue context packs that restrict each generated voice to
   what that character knows, feels, wants, fears, and perceives

The output should be inspectable. The author should be able to see why the
machine thinks a character would deflect, confess, threaten, soften, bargain,
lie, leave, spend a resource, conceal evidence, or make a choice that changes a
later scene.

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
- render dialogue from character-local context rather than global plot vibes
- generate branching dialogue/action scaffolds that reflect relationship history
- create procedural drama from incompatible goals instead of random hostility
- preserve cultural and factional pressure without making clones
- produce story beats a human author can actually use
- make branch choices matter through later affordances, callbacks, visual
  modifiers, costs, risks, and endings
- explain its choices in state terms rather than vibes in a rented coat

The real test is not whether the system can improvise a cool scene once. The
real test is whether it can help generate the tenth scene without forgetting
what the first nine did to everyone involved.
