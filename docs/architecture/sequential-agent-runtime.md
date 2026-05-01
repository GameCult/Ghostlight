# Sequential Agent Runtime

Ghostlight's near-term target is not a general-purpose life simulator.

The near-term product target is procedural interactive dialogue: set up a scene,
let characters choose believable responses, expose player-facing choices, and
propagate consequences into state, memory, and social perception.

The long-term target is broader. The same classifiers, appraisers, and
projection models should eventually operate inside a simulated game world with
open-world contexts, iterated decisions, economic behavior, and major-faction
decision-making. Do not train a tiny dialogue-only creature that panics the
first time someone buys medicine, defects from a faction, or reacts to a
shortage. Also do not build the whole economy today. Both cliffs are stupid.

The runtime target for now is scene-local: choose which character is acting,
give that character local awareness, let them act, apply consequences, mutate
the relevant state, and continue until the scene produces a usable dialogue
tree, scene transcript, or branch scaffold.

Single-turn projection remains useful, but it is a microscope, not the animal.

## Runtime Shape

One runtime tick should look like this:

1. Select an acting agent.
2. Build that agent's local awareness.
3. Derive available actions and constraints.
4. Let the agent choose a response.
5. Resolve the response against the scene.
6. Mutate world, relationship, memory, belief, and activation state.
7. Append an event record with participant-local interpretations.
8. Choose the next acting agent or end the scene.

The author sets the initial scene, player role if any, and high-level branch
constraints. The author should not need to decide every line, interruption,
withdrawal, refusal, or concession. That is the point of the machine.

For game use, the loop should be able to emit:

- a scene transcript
- candidate player choices
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

## Action Selection

The action model must allow more than speech.

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

Build interactive dialogue generation first, but shape the data for later
open-world use.

In scope now:

- scene-local agent turns
- player/NPC dialogue branches
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
model should grow out of reliable scene machinery and reviewed decision data,
not the other way around.

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
  -> state mutation
  -> next agent
```

## First Prototype

The next implementation should not try to simulate a city. Calm down, wizard.

Prototype one tiny dialogue-scene loop:

1. Load `examples/agent-state.cold-wake-story-lab.json`.
2. Select one scene and one acting agent.
3. Build a local awareness packet from existing state.
4. Compile projection controls.
5. Render a prompt that asks for an action or dialogue response.
6. Send it to Qwen.
7. Save the response as an event proposal.
8. Apply a small deterministic mutation:
   - append event
   - apply only mechanical world/resource/object deltas the resolver can prove
9. Manually review fuzzy state effects:
   - add one memory or belief update
   - adjust one or two `current_activation` values
   - update one relationship stance if justified
   - record the rationale as training data
10. Validate the resulting fixture.

The goal is not literary quality or autonomous plot generation. The goal is to
prove that a character can drive a scene beat, produce branchable dialogue or
action, and leave memory, relationship, social perception, and scene state
slightly different behind them.
