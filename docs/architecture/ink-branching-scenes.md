# Ink Branching Scenes

Ink is the branching scene format.

Ghostlight should not invent a parallel dialogue-tree language unless Ink
fails a concrete requirement. Ink already gives us readable interactive fiction,
choices, knots, stitches, variables, compiled JSON, and a format writers
can inspect without needing to wear a compiler helmet indoors.

Ghostlight's job is the machinery around Ink:

- build actor-local awareness from agent state and scene state
- project plausible speech and non-speech choices
- save generation receipts when a model proposes branches
- annotate why those choices make sense
- preserve listener interpretation as separate from actor intent
- record consequence surfaces for later state mutation
- keep fuzzy social updates reviewed until there is training data
- compile Ink into game-consumable JSON

Ink owns the playable branch structure. Ghostlight owns the psychological,
cultural, memory, relationship, and consequence rationale.

## Runtime Shape

The artifact flow is:

```text
agent state + scene state + lore grounding
  -> local awareness
  -> projected branch choices for the acting character
  -> selected branch as observable action
  -> event resolution from selected observable action
  -> participant appraisals and reviewed state mutation
  -> next actor local awareness from updated state
  -> next actor action
  -> Ink scene
  -> compiled Ink JSON
  -> reviewed training annotations
  -> selected branch consequences
  -> state, memory, belief, and relationship updates
```

The Ink file should be readable as Aetheria content on its own. The sidecar
training annotation should explain which Ghostlight state produced each branch
and which consequences are automatic, reviewed, or deferred.

Each character turn must be generated from that character's local awareness.
The protagonist-choice generator should not receive the responder's private
state. The responder generator should not receive the protagonist's private
intent except as an observable action, spoken text, gesture, object use,
silence, or other perceivable cue. A listener is allowed to misunderstand.

Participant appraisal is symmetrical. If an action hurts, threatens, reassures,
humiliates, obligates, or overloads anyone present, that change is consolidated
for the affected character before the next actor is selected. The next actor may
reply, walk away, comply, escalate, or end the interaction. A branch artifact can
still display this as `choice -> reaction`, but Ghostlight should model it as
turns over updated state, not as a mandatory paired response.

One-shot generation of both sides is allowed only as bootstrap scaffolding, and
must be marked as such in the capture review. It is not the target runtime
shape.

Archived local-model draft path:

```text
examples/agent-state.cold-wake-story-lab.json
  + examples/ink/cold-wake-sanctuary-intake.training.json
  -> tools/run_qwen_ink_branch_generation.py
  -> experiments/ink/cold-wake-sanctuary-intake-qwen-branch-candidates-v1.capture.json
  -> tools/materialize_qwen_ink_draft.py
  -> examples/ink/cold-wake-sanctuary-intake.qwen-draft.ink
  -> examples/ink/cold-wake-sanctuary-intake.qwen-draft.training.json
```

This path is a receipt from the old local-model materialization experiment. It
is useful for validation and negative examples, not the current gold-data path.
The current target path is source-grounded coordinator state, projected local
context, reviewed character turns, branch compiler materialization, and IF
artifact review.

## Ink Comments

Use Ink comments for lightweight traceability in writer-facing drafts:

- `ghostlight.scene`
- `ghostlight.fixture`
- `ghostlight.branch`
- `ghostlight.action`
- `ghostlight.intent`
- `ghostlight.npc_response`
- `ghostlight.consequence`
- `ghostlight.training_hook`
- `aetheria.flashpoint`

Use `//`, not Ink tags (`#`), for this metadata in drafts. Inky exposes tags in
preview, which makes Ghostlight state leak into the readable scene. The
sidecar remains the machine-readable receipt; comments are only local handles
for humans and simple tools.

## Sidecar Annotations

Each Ink scene should have a `.training.json` sidecar with:

- source fixture reference
- protagonist and NPC ids
- actor-local awareness summary
- projection controls
- branch ids and Ink knot paths
- action type and actor intent
- state basis
- reviewed consequences
- training hooks
- mutation policy

This sidecar is intentionally not a new branching format. It is a receipt. Ink
remains the scene tree; the sidecar says why the tree grew that way and which
state changes a reviewer accepted.

## Mutation Policy

Ink variables can record local playthrough outcomes. They should not silently
become canonical Ghostlight state.

Automatic:

- Ink compilation
- branch id and knot validation
- local variables for the current playthrough
- mechanical comment handles and training hook checks

Manual reviewed until trained:

- relationship deltas
- belief updates
- memory writes
- listener misread classification
- activation changes
- promotion from branch outcome into durable Aetheria lore

The first early prototype lives in
`examples/ink/cold-wake-sanctuary-intake.ink`.

The archived Qwen-generated draft lives in
`examples/ink/cold-wake-sanctuary-intake.qwen-draft.ink`.

The current accepted-as-draft branch-and-fold scaffold lives in
`examples/ink/cold-wake-branch-and-fold.v0.ink`.

## Cross-Scene Consequence Carryover

Ghostlight branching scenes should not behave like disconnected dialogue samples.
Interactive fiction needs consequences that survive the scene boundary. A branch
choice should write durable effects into later context: relationship stance,
perceived motives, resource costs, authority flags, object custody, active
constraints, unresolved hooks, and available future options.

For training data, every material branch should preserve:

- the player/NPC-visible choice or action
- the immediate responder output
- accepted world-state, resource, memory, and relationship deltas
- branch flags that later scenes must read
- deferred consequences that should surface later rather than immediately
- scene text showing how the consequence changes the feel of the next beat

The coordinator owns this continuity for now. Later, the trainable coordinator
model should learn to carry these consequences forward without flattening them
into cosmetic callbacks. If a choice does not change later affordances, trust,
resources, risk, or interpretation, it is probably not a meaningful branch.

Cold Wake lesson: the clinic contact sequence is a useful climax seam, but it is
not a full story by itself. A complete interactive fixture needs an opening
pressure, at least one consequential branch before the clinic, and a resolution
that visibly reflects earlier choices.

## Branch And Fold

Do not model every choice as an indefinitely isolated authored subtree. Strict
worldline splitting creates a combinatoric writing burden after a handful of
choices, which interactive fiction authors have been discovering the painful
way since approximately the first time someone let a player open a second door.

Ghostlight should branch locally and fold structurally:

```text
choice
  -> consequence packet
  -> normalized state deltas
  -> shared later scene reads compact variables
  -> conditional options, callbacks, imagery, and relationship texture
  -> major route split only when convergence would be false
```

Most branches should converge onto shared later scenes. The consequences should
survive as compact variables, bands, and flags rather than bespoke scenes for
every exact history.

Good consequence state looks like this:

- `sella_trust_maer: wary`
- `ledger_debt_level: high`
- `clinic_staff_exhaustion: true`
- `veyr_evidence_disclosed: partial`
- `isdra_reads_maer_as: procedurally_dangerous`
- `route_authority_pressure: escalated`

Those variables should alter later scenes by changing available options,
relationship posture, timing pressure, resource costs, callback prose, visual
state modifiers, and participant appraisals. They should not usually require a
fully separate scene route.

Use coarse bands instead of fake precision. `blocked`, `wary`, `conditional`,
`cooperative`, and `personally_invested` are more useful than pretending the
story can meaningfully maintain 37 decimals of trust. The same applies to heat
debt, legal exposure, institutional pressure, staff exhaustion, reputation, and
social suspicion.

Major route splits are allowed when a consequence changes the playable world so
much that folding would lie. Examples:

- a character dies, leaves, defects, or becomes unavailable
- a location is destroyed, locked, opened, or politically inaccessible
- a faction becomes hostile, allied, exposed, or legally empowered
- a key object is destroyed, stolen, transferred, or transformed
- the protagonist loses or gains institutional authority
- a quest route, rescue path, market, technology, or branch-local future opens
  or closes entirely

The coordinator owns the fold decision for now. Every material branch should
record whether it folds into an existing later scene, opens a conditional
variant inside that scene, or creates a justified route split. If a route split
is created, the artifact should name the reason and the expected future writing
debt. Trees are cheap to grow and expensive to prune. The machine should not
act surprised by botany.

## Branch Compiler

The Branch Compiler is the organ that turns Ghostlight scene intent into a
playable Ink artifact. It is not the character agent, the responder, the
appraiser, or the final reviewer. It assembles branch structure from already
available scene state, projected actor affordances, proposed actions, accepted
consequence packets, and coordinator constraints.

The Branch Compiler receives:

- fixture brief and scene spine
- source-grounded lore digest
- current world, scene, resource, relationship, and branch state
- actor-local branch candidates, including speech and non-speech actions
- responder outputs or coordinator-authored placeholder reactions
- consequence packets from reviewed turns
- branch-and-fold plan
- visual continuity requirements
- schema and Ink formatting constraints

The Branch Compiler emits:

- readable `.ink` scene content
- Ink variables, knots, stitches, choices, gathers, and conditionals
- a `.training.json` sidecar tying each branch to state basis, action intent,
  consequence surfaces, and later callbacks
- compiler notes for fold decisions, route-split decisions, and known review
  risks
- image prompt anchors and branch/state modification handles when the fixture is
  meant for illustrated output

The Branch Compiler may propose structure. It must not invent hidden character
truth, private motivations, or durable social mutations as if compilation made
them real. Character behavior comes from projected local context and responder
or coordinator-authored action proposals. Social interpretation and memory
changes come from appraiser/mutator review. The compiler's job is to make the
branching artifact playable, legible, and traceable.

The compiler should prefer compact state variables, bands, and flags over
runaway branch trees. It should create major route splits only when the
coordinator or reviewed consequence packet says folding would falsify the
world. When it folds branches, it must preserve the variables and callbacks that
make the fold honest.

The Branch Compiler output is draft until the IF Artifact Reviewer accepts it.
That separation matters. A compiler can make a scene valid Ink while still
producing cosmetic choices, fake folds, unused variables, or endings that ignore
the whole moral machinery. Valid syntax is not wisdom. It is barely hygiene.

## IF Artifact Review

Branching fixtures need an independent review pass before acceptance. The
coordinator should not be the only thing grading its own Ink, sidecar, and
branch logic. That is how a scene ends up proudly tracking variables that do
nothing except sit there in a tiny uniform.

The reviewer may start as a prompted specialist sub-agent. Later it should
become evaluator/classifier training data and eventually a trainable review
organ. Its job is not prose polish. Its job is to catch false consequence.

The reviewer receives:

- the Ink file
- the `.training.json` sidecar
- the coordinator artifact
- the intended branch plan, when available
- the declared state variables, flags, bands, resources, risks, and visual
  modifiers
- any compiled Ink or validation output available for the fixture

The reviewer emits:

- `accepted`, `needs_revision`, or `rejected`
- severity-ranked findings
- a fake-variable audit
- a choice-consequence audit
- a fold/callback audit
- a state-gated prose audit
- a visual-continuity audit
- major-route-split justification notes
- specific required fixes

Acceptance requires every material choice to change at least one later surface:

- available options
- scene route or justified route split
- relationship, trust, appraisal, or social standing
- resource cost, time pressure, legal exposure, authority pressure, heat burden,
  exhaustion, or other risk
- object custody, evidence quality, access, or technology state
- later callback prose implemented as an actual conditional
- ending texture or outcome band
- visual state modifier tied to a real branch flag or state variable

Every declared variable must either be read later or be marked as telemetry-only
in the sidecar. Ominous variables need threshold behavior. If a variable named
`authority_pressure` rises and nobody with authority ever reacts, the artifact
fails review. If `heat_time` counts down and no route, ending, resource cost, or
staff condition changes, the artifact fails review. The name is allowed to be
dramatic only if the machinery has teeth.

Any prose that says "if X happened" must be implemented as an actual conditional
or moved into reviewer/coordinator notes. Writer-facing text should not name a
branch possibility while dodging the state check that proves which version of
history occurred.

Common failure labels:

- `potemkin_state`: a tracked variable never materially changes the artifact
- `cosmetic_choice`: a choice changes wording but not affordance, cost, risk,
  appraisal, callback, visual state, or outcome
- `missing_state_read`: a set variable is never read by later Ink, sidecar, or
  coordinator consequence logic
- `fake_fold`: branches claim to fold, but the folded scene ignores their state
- `unearned_convergence`: branches converge despite consequences that should
  make convergence false
- `unjustified_route_split`: a separate route exists without enough world or
  story consequence to justify the writing debt
- `state_named_instead_of_checked`: prose mentions alternate branch states
  instead of checking variables
- `visual_callback_missing`: image modifiers or visual prose ignore branch
  consequences that should be visible
- `authority_without_consequence`: institutional pressure rises but never bites
- `resource_without_cost`: ledgers, heat, staff time, supplies, or access are
  invoked without affecting later choices or outcomes
- `ending_ignores_major_state`: the ending does not respond to major variables
  established earlier

The first Cold Wake branch-and-fold review exposed this exact class of failure:
`heat_time`, `clinic_exhaustion`, `evidence_quality`, `provisional_category`,
and `authority_pressure` initially looked important before review forced them
to alter gates, callbacks, and outcomes. That review belongs in the evaluator
training seed, not in a shame drawer. Shame drawers have terrible indexing.

## Scene Imagery

Ghostlight scenes are also website content. A complete fixture should emit
vivid visual direction alongside prose and training receipts so story branches
can be illustrated without a later model guessing what the room looked like.

Interactive fiction replay also needs screen rhythm. A narrative scene may move
through several visual sections: establishing shot, workstation, break table,
object closeup, alarm state, crowd line, supervisor arrival, aftermath. These
should be segmented so an InkJS replay can click through the scene at the pace
the story needs. A pivotal beat may be a single sentence on its own screen when
the point is to let the image and silence do the work.

Each distinct visual section should preserve an art-direction block with:

- global style cue: the fixture-level visual language when the art direction
  depends on a specific non-default look
- establishing shot: location, scale, camera distance, and dominant geometry
- light and color: practical light sources, palette, exposure, haze, glare,
  reflection, water, vacuum, screen glow, or other atmosphere
- bodies and interfaces: how each relevant body occupies the space, including
  nonhuman access, assistive infrastructure, suits, avatars, wet/dry boundaries,
  displays, tools, and machinery
- material evidence: objects, damage, records, ledgers, cargo, alarms, clothing,
  stains, frost, heat shimmer, or other concrete traces of what has happened
- branch marks: visible changes caused by earlier choices, such as a sealed
  hatch, an exhausted staffer, a live debt ledger, missing equipment, guarded
  posture, or a route marker left blinking
- base image prompt: one stable prompt for the unbranched scene, suitable for
  generating the reusable background or canonical key frame
- branch modification prompts: additive edit prompts for visible consequences,
  changed character positions, opened or sealed routes, damaged objects,
  lighting changes, emotional posture, crowd pressure, or other scene states

Named characters need stable visual references. An image model does not know
what Lio Vale looks like because the prose named Lio Vale. Each illustrated
fixture should define `visual_character_refs` for recurring named characters:
body type, apparent age range if relevant, face/hair/skin or equivalent visual
identity, clothing and equipment, silhouette, posture language, and
`do_not_change` continuity details. Scene prompts may then reference those
character refs instead of repeating the whole description every time.

Prompts must be imagegen-ready. Do not rely on unexplained Aetheria jargon as
if the image model knows the lore. If the prompt says "wet-service cradle" or
"dry-operation harness," it should also describe the visible thing according to
the faction, location, and labor regime. A Navigator comfort habitat and an AU
industrial yard should not quietly become the same room. If a scene uses a
biodrone limiter interface, cavity seal assembly, or life-support bypass kit,
describe the shape, scale, materials, lights, cables, seals, or displays that
make it drawable.

The same rule applies to environments. "Manifold line," "cavity yard,"
"supervisor glass," and "anchor rail" are not images by themselves. A base
prompt should describe the physical space a general image model can draw:
corridor shape, wall/ceiling/floor geometry, pipe runs, cable bundles, panels,
screens, rails, doors, windows, gauges, warning lights, materials, scale, and
where bodies stand inside that geometry.

Every visual section should identify:

- `visual_scene_id`: stable handle for the screen or key frame
- `ink_anchor`: knot, stitch, or comment handle where the visual applies
- `base_image_prompt`: durable scene prompt for the unmodified frame
- `visible_characters`: characters allowed in the frame and their default
  position or stance, plus a `visual_ref` when the character is named
- `state_image_modifiers`: additive prompts tied to explicit variables,
  thresholds, or route flags
- `branch_image_modifiers`: additive prompts tied to specific branch ids
- `visual_continuity_notes`: facts that must remain stable across edits

Prompt assembly for illustrated replay should combine:

```text
global style cue
  + visual_character_refs for named characters actually visible in this rendered frame
  + visual section base_image_prompt
  + triggered state or branch image modifiers
```

The base image prompt is allowed to focus on the room and staging only if the
assembled prompt includes the relevant character refs. If a tool sends the base
prompt alone to imagegen, the base prompt must carry enough character detail by
itself. The same applies to visual style: if a fixture depends on a specific
look, the assembled prompt must carry that style cue every time. Names are
handles, not faces.

Do not include `visual_continuity_notes` verbatim in the image prompt. They are
tooling and review constraints for keeping repeated generations coherent. If a
continuity fact needs to reach the image model, rewrite it as concrete visible
prompt text in the base prompt, a character ref, or a branch/state modifier.
The image generator only sees the prompt you send it. It does not remember the
previous scene unless a real image-editing workflow supplies that image as
input.

The coordinator owns this surface for now. The responder should not receive
omniscient visual art direction unless the character can observe it. Character
packets may receive the local visible subset: what this actor can see, hear,
touch, smell, monitor, or infer from their interface.

Visual continuity is part of consequence carryover. If an earlier branch cost
heat, trust, time, equipment, or social standing, the later scene should show
that cost somewhere on the page or in the frame. Otherwise the choice may be
mechanically recorded while still feeling fake, which is the stupid little
trapdoor interactive fiction keeps setting for itself.

Do not regenerate every branch image from scratch when continuity matters.
Each scene should have one durable base visual prompt and branch/state
modification prompts that describe only what changed. The base prompt owns the
room, palette, dominant geometry, key props, bodies, and interface layout. A
modification prompt owns the delta: Sella moves from the dryside console to the
decon sleeve, the route marker flips from amber to white, a wet access hatch
seals, an exhausted staffer appears behind the glass, Maer's ledger panel gains
a live debt line.

Keep these prompts separate from canonical prose:

- `base_image_prompt`: stable scene anchor
- `state_image_modifiers`: reusable prompts for common scene states
- `branch_image_modifiers`: prompts tied to specific branch ids
- `visual_continuity_notes`: non-prompt reminders about what must remain stable

Image edits are allowed to stylize, compose, and beautify. They are not allowed
to silently change canonical facts. If an edit introduces a new visible object,
body affordance, damage mark, or faction symbol that was not in the scene state,
the coordinator must either reject it, mark it as noncanonical illustration
noise, or promote it through the normal reviewed lore/state path. The picture
does not get to smuggle a gun onto the mantel just because it had a strong
personal brand that morning.
