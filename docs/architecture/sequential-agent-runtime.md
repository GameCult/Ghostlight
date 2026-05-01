# Sequential Agent Runtime

Ghostlight's target is not a fancy dialogue prompt machine.

The target is a sequential social-agent runtime: set the stage, choose which
character is acting, give that character local awareness, let them act, apply
consequences, mutate state, and continue. The author should not need to control
every beat. The useful machine is the one that makes characters push the story
sideways for reasons their state can explain.

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

The author sets the initial scene and may steer with high-level constraints.
The author should not need to decide every line, interruption, withdrawal,
betrayal, repair attempt, confession, or slap. That is the point of the machine.

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

This is not reader legibility. This is operational state. If the scene itself is
abstract and byzantine, the agent may still behave correctly inside that mess.
The story may be hard to read for separate authoring reasons. Do not shove that
problem into projection.

## Action Selection

The action model must allow more than speech.

Actions can include:

- speak
- stay silent
- ask a question
- refuse
- leave
- follow
- touch or block a control surface
- comfort
- threaten
- attack
- conceal evidence
- reveal evidence
- spend a resource
- call for help
- change topic
- make a promise
- break a promise
- update a public record
- create a private obligation

The agent should choose from affordances produced by the interaction of state,
scene, role, resources, relationship, and pressure. The action should not be
commanded by a prompt label. If a character hits someone, it should fall out of
proximity, inhibition, grievance, fear, role permission, and exhausted trust.

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

Prototype one tiny loop:

1. Load `examples/agent-state.cold-wake-story-lab.json`.
2. Select one scene and one acting agent.
3. Build a local awareness packet from existing state.
4. Compile projection controls.
5. Render a prompt that asks for an action, not just dialogue.
6. Send it to Qwen.
7. Save the response as an event proposal.
8. Apply a small deterministic mutation:
   - append event
   - add one memory or belief update
   - adjust one or two `current_activation` values
   - update one relationship stance if justified
9. Validate the resulting fixture.

The goal is not literary quality. The goal is to prove that a character can act
from local state and leave the world slightly different behind them.
