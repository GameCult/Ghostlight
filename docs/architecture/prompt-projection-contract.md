# Prompt Projection Contract

Ghostlight should not prompt a dialogue model with the full schema. The schema is
state storage. A prompt is a temporary scene instrument.

The projection layer turns durable state into a compact speaker-local pressure
model, then into prompt prose. The prose should describe what the character
knows, wants, fears, protects, misreads, and sounds like right now. It should not
recite taxonomy labels unless a debugging mode explicitly asks for them.

## Projection Pipeline

### 1. Select Speaker-Local Inputs

For one speaker in one scene, gather only the state that can affect this turn:

- speaker identity and public role
- speaker canonical state
- speaker goals and protected values
- speaker memories selected by scene, relationship, and active pressure
- speaker perceived overlay of the listener
- speaker-to-listener relationship stance
- listener-to-speaker relationship stance only if the speaker has reason to infer it
- scene public facts
- scene hidden facts known to the speaker
- active public and private stakes
- culture, faction, or institution priors relevant to the scene

Unknown or author-only material is omitted. Known secrets are included only as
speaker-local pressure: what the speaker is guarding, why it matters, and what
would happen if it leaked.

### 2. Compute Active Pressure

For each candidate variable, compute an effective scene activation from:

- canonical `current_activation`
- active scene pressures
- relationship stance toward the listener
- perceived state overlays
- salient memories
- active goals and protected values
- cultural or institutional scripts

The first implementation can use simple weighted rules. The important contract
is not the exact formula. The important contract is that the projection explains
why a variable matters in this scene.

Example:

```text
Cat has high shame sensitivity, mask rigidity, distance seeking, suspicion,
ironic distance, and acute shame. Oz is useful but opaque. The scene pressures
Cat to accept help while not looking dependent.
```

That intermediate pressure statement is more useful than a dump of numbers.

### 3. Detect Tensions

A good dialogue prompt needs contradictions, not just traits.

Derive scene tensions such as:

- wants help but resents needing it
- wants closeness but treats closeness as leverage against them
- wants to protect a secret while needing cooperation
- wants status while feeling exposed
- wants to be honest but expects honesty to be punished
- wants to soothe the listener without becoming available for inspection

Tensions are where verbal behavior starts to move.

### 4. Derive Response Affordances

Do not store defensive speech moves as canonical traits. Derive likely response
affordances from state intersections.

Examples:

| Target behavior | State interaction that can produce it |
| --- | --- |
| Sarcastic deflection | Shame sensitivity + ironic distance + suspicion + exposure pressure + dry or pointed voice. |
| Sincerity evasion | Mask rigidity + low authenticity tolerance + strategic opacity + low emotional explicitness. |
| Emotional evasion | Distance seeking + withdrawal + acute shame or control pressure + low self-disclosure. |
| Banter as boundary | Humor + ironic distance + attachment pressure + distance seeking. |
| Verbal aggression | Hostility + grievance activation + perceived status threat + abrasive boundary. |

These are test targets, not prompt commands. The rendered prompt should describe
the pressure in ordinary language:

```text
If Oz presses too close to the real fear, Cat is likely to answer the useful
part of the question while making the emotional part sound ridiculous.
```

That gives the model a behavioral slope without saying `use sarcastic_deflection`.

### 5. Render Prompt Sections

A dialogue prompt should be compact and scene-bound.

Recommended sections:

- `Speaker`: who they are in this scene
- `Known Facts`: only speaker-local facts
- `Current Stakes`: what changes if this goes well or badly
- `Active Inner Pressures`: the strongest motives, fears, wounds, and needs
- `Relationship Read`: how the speaker currently interprets the listener
- `Tensions`: contradictions the speaker is managing
- `Voice Surface`: baseline speech texture for this moment
- `Likely Defensive Moves`: ordinary-language pressure outcomes, not schema labels
- `Do Not Invent`: boundaries about facts absent from the pack
- `Task`: the narrow generation request

`Do Not Invent` is not a replacement for positive context. It only tells the
model not to create new facts beyond the pack. It should not list author-only
secrets.

### 6. Generate, Then Evaluate

Generation and evaluation are separate passes.

The dialogue model receives the rendered prompt and writes from the speaker's
local pressure model. A later evaluator can check whether target emergent
behaviors appeared when the state predicted them.

Evaluation questions can include:

- Did the line protect the speaker's known secret without naming unknown facts?
- Did the line reflect the active relationship stance?
- Did the line preserve the voice surface without flattening into a generic bit?
- Did a target defensive behavior appear when the pressure profile called for it?
- Did the response avoid a target behavior when the pressure profile did not call for it?

The evaluator should cite the state interactions that justify its read.

## Example Projection: Cat To Oz

Input pressures:

- Cat needs Oz's help with the taxi.
- Cat suspects Oz is a mask for a person.
- Cat has high suspicion, distance seeking, shame sensitivity, mask rigidity,
  abrasive boundary, ironic distance, dry voice, pointedness, and profanity.
- Cat's relationship stance toward Oz includes low trust, moderate respect,
  high fascination, and active expectation of betrayal.
- The scene makes Cat look dependent if she accepts help too openly.

Rendered pressure prose:

```text
Cat wants the repair, but accepting help makes her feel handled. Oz is useful,
which makes the secrecy more irritating, not less. Cat should probe for the
person behind the drone while pretending she is only annoyed about the repair.
If Oz gets too close to her fear of dependency, Cat is likely to answer the
practical issue and make the emotional issue sound stupid.

Voice for this turn: dry, quick, pointed, plainspoken, low verbal warmth, low
emotional explicitness, some profanity. Humor can appear as a boundary, not as
friendliness.
```

Possible generated behavior:

```text
"Sure, let the mystery box fix my relay. That's never how the paperwork starts."
```

That line is produced from the interaction between need, suspicion, shame
pressure, and dry pointed voice in the same scene.

## Example Projection: Oz To Cat

Input pressures:

- Oz wants to help without exposing the operator.
- Oz reads Cat as suspicious, dangerous, and potentially protective.
- Oz has high mask rigidity, strategic opacity, withdrawal, hedging, coded
  politeness, register switching, listening responsiveness, and low profanity.
- Oz's relationship stance toward Cat includes respect, fear, fascination,
  moderate expectation of care, and moderate expectation of betrayal.
- The scene rewards usefulness but punishes exposure.

Rendered pressure prose:

```text
Oz wants Cat to accept the repair and stop inspecting the mask. Cat's suspicion
is dangerous because it is accurate enough to matter. Oz should stay useful,
slightly warm, and evasive. If Cat asks directly about the operator, Oz should
redirect toward the immediate repair or answer in a way that preserves the mask
without making the evasion look like panic.

Voice for this turn: controlled, concise, technically precise, gently polite,
low emotional explicitness, careful verbal warmth, frequent hedging when the
subject approaches the operator.
```

Possible generated behavior:

```text
"The relay first. Biography after we stop your meter from eating itself, maybe."
```

The line withholds the secret because Oz's goals, mask rigidity, relationship
fear, and scene stakes converge.
