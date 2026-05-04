# Sequential Agent Runtime

Ghostlight's near-term target is not a general-purpose life simulator.

The near-term product target is procedural interactive dialogue in the game
sense: branching scenes where choices can be speech, silence, movement, object
use, refusal, revelation, concealment, violence, withdrawal, trade, or any other
scene-valid action. "Dialogue tree" does not mean a tree made only of spoken
lines. It means an interactive branch structure for a character encounter.

Ink is the branch format for that structure. Ghostlight should not invent its
own dialogue-tree file format while Ink already exists and fits the job. See
`docs/architecture/ink-branching-scenes.md` for the integration contract.

Ghostlight should set up a scene, generate plausible things the protagonist or
NPCs could do, generate responses from the people present, and propagate the
consequences into world state, memory, belief, and social perception.

The long-term target is broader. The same classifiers, appraisers, and
projection models should eventually operate inside a simulated game world with
open-world contexts, iterated decisions, economic behavior, and major-faction
decision-making. Do not train a tiny dialogue-only creature that panics the
first time someone buys medicine, defects from a faction, or reacts to a
shortage. Also do not build the whole economy today. Both cliffs are stupid.

The runtime target for now is scene-local: choose which character is acting,
give that character local awareness, let them act, apply consequences, mutate
the relevant state, and continue until the scene produces a usable branching
scene tree, scene transcript, or branch scaffold.

Single-turn projection remains useful, but it is a microscope, not the animal.

## Runtime Shape

One runtime tick should look like this:

1. Select an acting agent from the initiative schedule.
2. Build that agent's local awareness.
3. Derive available actions and constraints.
4. Let the agent choose a response.
5. Resolve the action into an observable event.
6. Apply every affected participant's appraisal and state mutation for that
   event.
7. Append the event record with participant-local interpretations.
8. Update initiative timing, reaction windows, and action recovery.
9. Select the next acting agent from the updated schedule, or end the scene.

The author sets the initial scene, player role if any, and high-level branch
constraints. The author should not need to decide every line, interruption,
withdrawal, refusal, or concession. That is the point of the machine.

Every generated character turn must be local to that character.

For branching scenes, this means protagonist/player options and NPC responses
are separate generation steps:

1. Build the protagonist's local awareness.
2. Generate plausible protagonist choices from only what the protagonist knows,
   perceives, wants, can attempt, and can affect.
3. When one choice is selected, reduce it to observable action: words, silence,
   gesture, movement, object use, resource spend, or other perceivable change.
4. Resolve the selected choice into an event visible to affected characters.
5. Apply reviewed participant appraisals and state mutations for that event:
   hurt, threat, reassurance, obligation, resentment, trust movement, overload,
   misread, world/object/resource changes, and scene-ending choices when they
   occur.
6. Update initiative timing and select the next acting agent from the updated
   schedule. The next agent may answer, walk away, escalate, comply, refuse,
   call for help, attack, or end the interaction.
7. Generate that next action from the newly consolidated local state.
8. Store actor intent, listener appraisal, and durable interpretation
   separately.

There is no privileged paired exchange inside the runtime. For authoring
convenience a branch artifact may show `player action -> NPC action`, but
Ghostlight should treat that as two turns stitched together, not one fused
response object. If Maer wounds Sella with a ledger threat, Sella's state
changes before she is considered as the next actor; she is not obligated to
produce a line just because the author expected one.

One-shot paired generation, where a model writes both the player's move and the
NPC response from a shared omniscient packet, is acceptable only as bootstrap
scaffolding. It should be reviewed and labeled as such. Do not treat it as the
target runtime behavior.

## Initiative Scheduler

Not every character gets a turn every round. There is no round.

Ghostlight needs a scene-local initiative scheduler: a mechanical organ that
tracks who is ready to act, who is still recovering from a previous move, and
who has a valid interrupt window. It sits after event appraisal/mutation and
before the next local-awareness projection.

The scheduler does not decide what a character wants. It decides when a
character is eligible to act.

Core rule:

```text
observable event
  -> participant appraisals and reviewed mutation
  -> initiative/reaction update
  -> next actor selection
  -> next actor projection
```

Every affected participant still appraises every event before the next action
is generated. Initiative controls action opportunity, not perception. A hostage
can be terrified by a negotiator's move immediately; that does not mean the
hostage gets a full action before the hostage-taker, unless the schedule or a
reaction window gives them one.

Each participant carries initiative state:

- `initiative_speed`: how quickly the participant recovers action opportunity
  in this scene.
- `next_ready_at`: the scene-clock time when they can take a normal action.
- `reaction_bias`: how likely they are to seize urgent interrupt windows.
- `interrupt_threshold`: how strong a trigger must be before they interrupt.
- `current_load`: stress, injury, cognitive burden, physical constraint, or
  interface lag that makes action harder.
- `status`: whether they are active, blocked, withdrawn, incapacitated, or
  offscreen.

Actions carry recovery shape:

- `action_scale`: micro, short, standard, major, or committed.
- `base_recovery`: how much scene time the move normally costs.
- `initiative_cost`: how much local opportunity the actor spends.
- `interruptibility`: whether others can break in while the action unfolds.
- `commitment`: how much the actor is locked into the move once begun.

The scheduler first checks reaction windows created by the last event. A
reaction window is an interruptible opportunity, not a free omniscient response.
It must name the triggering event, eligible actors, urgency, allowed action
scales, expiry, and local basis. If no reaction clears threshold, the scheduler
selects the active participant with the lowest `next_ready_at`.

Recommended reaction readiness:

```text
reaction_readiness = (reaction_bias * urgency) - current_load
```

An interrupt can fire when `reaction_readiness >= interrupt_threshold`, the
actor is eligible for the window, and the actor is not blocked. Ties are broken
by earlier `next_ready_at`, higher `initiative_speed`, then stable actor id.

After an accepted action, update the actor:

```text
recovery = base_recovery / max(initiative_speed, minimum_speed)
next_ready_at = max(scene_clock, current next_ready_at) + recovery
```

Major or committed actions should usually create larger recovery, stronger
counterplay windows, clearer visible cues, or more durable consequences. Short
actions let a character stay nimble but should not accomplish everything. A
one-word refusal, a glance to a camera, or stepping behind a marked line can be
fast. A public confession, tackle, system override, or hostage release costs
more initiative because it changes the scene.

The coordinator may override the scheduler only by recording the override
reason. Useful overrides include player-facing pacing, hard scene boundaries,
or a game-engine rule that is not yet represented in the schedule. An
unrecorded override is not training-ready coordinator data.

For Ink, the scheduler can be collapsed into branch structure for readability,
but the sidecar should preserve the initiative receipt when the fixture claims
training value. Player choices may be presented together even if, inside the
runtime, they represent alternative actions available at one scheduled moment.

For game use, the loop should be able to emit:

- a scene transcript
- Ink scenes with candidate player choices, including speech and non-speech
  actions
- compiled Ink JSON
- NPC response branches
- state/memory/social-perception deltas for each branch
- unresolved hooks for later scenes

If a feature does not help produce those artifacts, it belongs in the parking
lot until the smaller machine works. If a field or label preserves information
needed for later open-world behavior, consumer choice, faction pressure, or
iterated decisions, keep it visible in the training artifact even if the first
prototype does not simulate it.

## Local Awareness

An acting agent needs more than a dialogue cue.

The local awareness pack should include:

- where the agent is
- who is present
- what the agent can perceive
- what the agent knows, believes, suspects, and misreads
- what the agent wants right now
- what the agent is protecting
- what resources and tools are available
- what social, legal, physical, and emotional constraints apply
- what actions are available
- what each action might cost
- what facts are unresolved
- what the agent is allowed to mutate directly

This is not reader legibility or global story authorship. This is operational
state for scene-local dialogue and action. If the scene itself is abstract and
byzantine, the agent may still behave correctly inside that mess. Do not shove
general prose quality or whole-plot design into this layer.

## Branch Choices And Action Selection

The action model must allow more than speech.

For a protagonist scene, Ghostlight should be able to generate several
plausible choices for what the protagonist can do next. For example, if Cat is
interacting with another character, the generated choices might include:

- say the honest thing badly
- deflect with a joke
- leave before the other person can answer
- hand over evidence
- hide evidence
- accept help while pretending not to need it
- threaten to walk
- spend scarce money
- break a device
- touch someone gently
- slap someone

Those choices should come from Cat's state, scene affordances, resources,
relationship pressure, and goals. They should not be generic verbs glued to a
persona card. Each branch should carry expected consequence surfaces, such as
trust risk, reputation risk, resource cost, information exposure, or future
obligation.

Keep the action vocabulary small and concrete. Do not turn every possible
speech act into a separate tool. `ask`, `refuse`, `offer`, `reveal`, `conceal`,
and `request authority action` are usually communicative intentions carried by
`speak`, `gesture`, `show_object`, `withhold_object`, or `use_interface`.

Initial action primitives:

- `speak`
- `silence`
- `move`
- `gesture`
- `touch_object`
- `block_object`
- `use_object`
- `show_object`
- `withhold_object`
- `transfer_object`
- `spend_resource`
- `attack`
- `wait`

The agent should choose from affordances produced by the interaction of state,
scene, role, resources, relationship, and pressure. The action should not be
commanded by a prompt label. If a character hits someone, it should fall out of
proximity, inhibition, grievance, fear, role permission, and exhausted trust.

An action proposal may include intended function:

- ask
- refuse
- offer
- threaten
- comfort
- reveal
- conceal
- request
- promise
- accuse
- redirect

That intention is not guaranteed truth. A listener can misread it. A character
can claim they were offering help while the listener perceives a threat. A
character can intend restraint and still be read as contempt. The event should
store observable action separately from actor intent and participant-local
interpretation.

Example:

```json
{
  "action_type": "speak",
  "spoken_text": "Name the cost before you call it caution.",
  "actor_intent": "force_cost_acknowledgment",
  "observable_cues": ["calm voice", "still posture", "public challenge"]
}
```

Isdra may perceive that as procedural intimidation. Maer may experience it as
stewardship. The classifier/appraiser later learns from those differences.

## Mutation Authority

Characters should only mutate what their action can actually affect.

Examples:

- A character can mutate their own beliefs, memories, current activation, goals,
  and presentation pressure.
- A character can create observable events others may perceive.
- A character can change world objects they can physically or institutionally
  access.
- A character can affect relationship stance, but the other side's relationship
  update is mediated through that other agent's appraisal.
- A character can propose an order they lack authority to issue.
- A character cannot silently rewrite another agent's private state, decide an
  unresolved hidden fact, or seize institutional authority they do not possess.

This is where the projection-control layer belongs. Frame ownership, authority
boundaries, object custody, required semantics, and forbidden resolutions are
not decorative prompt warnings. They are mutation gates.

## Event Resolution

Generated action is not state mutation by itself. It becomes an event proposal.

The resolver should decide:

- what observable event occurred
- what world objects changed
- what resources were spent or created
- which agents perceived the event
- how each participant appraised it
- which beliefs were reinforced, challenged, or created
- which memories were written
- which relationship dimensions changed
- which current activations changed
- which options opened or closed

In a branching scene tree, each branch should carry its own consequence packet.
If Cat chooses to lie, accept help, shove someone, spend money, or reveal a
secret, that branch should record what changed and what might matter later.
Consequences can include:

- trust lost or gained
- suspicion raised or lowered
- obligation created
- debt created or paid
- resource spent
- information revealed or concealed
- relationship rupture or repair
- faction or reputation pressure changed
- future scene hook opened or closed

The first implementation can be conservative and partly deterministic. A bad
resolver will build narrative sludge faster than a bad line of dialogue will.
No pressure. Tiny little bear trap.

The fuzzy parts should stay manual at first.

Do not invent a fragile rule-based social psychology engine just to feel
automated. Early runtime passes should manually review or author:

- participant appraisals
- relationship deltas
- belief updates
- memory writes
- activation changes
- ambiguous intent classification
- whether a listener misread or correctly read a move

Those manual decisions are the training data. Save the observed action, actor
intent, listener perception, accepted mutation, rejected mutation, and reviewer
notes. Later, train classifiers and appraisers on that corpus. The prototype
should automate mechanical legality and visibility first, not pretend it has
already learned human interpretation from twelve examples and a strong coffee.

## Scope Boundary

Build interactive branching scene generation first, but shape the data for later
open-world use.

In scope now:

- scene-local agent turns
- player/NPC dialogue and action branches
- local action proposals
- event records
- manual reviewed appraisals
- memory, belief, relationship, and activation deltas
- branch consequences that can be carried into later scenes
- action intent and listener interpretation labels
- resource, price, scarcity, obligation, reputation, and faction-pressure tags
  when they affect the decision
- reviewed examples of consumer-like choices and institution/faction pressure,
  even when the prototype only records them as scene state

Training-data obligations now:

- Preserve why an agent chose, bought, refused, hoarded, traded, complied,
  defected, concealed, punished, or helped.
- Preserve what the agent believed the cost, risk, status effect, and social
  consequence would be.
- Preserve faction and institution pressures as perceived by the agent, not
  only as objective setting lore.
- Preserve iterated-decision context: promises, debts, prior betrayals,
  reputation, memory, scarcity, and expected future contact.

Out of implementation scope for now:

- full autonomous world simulation
- economy simulation loops
- city-scale scheduling
- long-horizon plot invention without author scaffolding
- autonomous factions moving offscreen as full actors
- physical simulation beyond scene affordances
- automatic social appraisal rules pretending to be trained classifiers

The eventual storyline generator, economic behavior model, and faction decision
model should grow out of reliable branching-scene machinery and reviewed
decision data, not the other way around.

## Projection's Narrower Job

Projection still matters.

Its job is to turn structured state into a compact local action context for one
agent at one moment. It should support action choice and response rendering. It
should not be treated as the whole story engine.

Projection artifacts are still useful for:

- debugging whether local state produces plausible action
- training a smaller projector
- preserving reviewed examples
- testing state boundaries and mutation gates
- producing dialogue or action beats inside a larger runtime

But the production path is sequential:

```text
agent state + scene state
  -> local awareness
  -> action affordances
  -> agent response/action
  -> event resolution
  -> participant appraisals and state mutation
  -> initiative/reaction scheduling
  -> next actor selection
  -> next agent action from updated local state
```

## First Prototype

The first implementation should not try to simulate a city. Calm down, wizard.

Prototype one small Ink-backed branching scene loop, using the current live
fixture or a new source-grounded fixture:

1. Load a fixture agent-state artifact, such as
   `examples/agent-state.pallas-species-strikes.v0.json`.
2. Select one scene and one acting agent.
3. Build a local awareness packet from existing state.
4. Compile projection controls.
5. Generate or author branch choices and NPC responses from local context.
6. Materialize the branch plan through the Branch Compiler as Ink, sidecar
   annotation, compiler notes, consequence variables, and visual prompt handles.
7. Compile the Ink scene to JSON with `inkjs`.
8. Save a sidecar training annotation that maps Ink branches to Ghostlight
   state basis, actor intent, consequence surfaces, and mutation policy.
9. Apply selected branch consequences only through reviewed mutation:
   - Ink variables can track local playthrough facts
   - canonical state changes require resolver/reviewer approval
10. Run IF artifact review before acceptance:
   - every material variable is read later or marked telemetry-only
   - every material choice changes affordance, risk, callback, visual state, or
     outcome
   - folds preserve consequence rather than hiding it
11. Manually review fuzzy participant appraisals and state effects each turn:
   - update every affected participant before selecting the next actor
   - add one memory or belief update
   - adjust one or two `current_activation` values
   - update one relationship stance if justified
   - record the rationale as training data
12. Validate the resulting fixture.

The goal is not literary quality or autonomous plot generation. The goal is to
prove that a character can drive a scene beat, produce branchable dialogue or
non-dialogue action, trigger plausible responses from other characters, and
leave memory, relationship, social perception, and scene state slightly
different behind them.
