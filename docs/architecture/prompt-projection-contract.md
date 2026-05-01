# Prompt Projection Contract

Ghostlight should not prompt a response model with the full schema. The schema is
state storage. A prompt is a temporary scene instrument.

Projection is not the whole Ghostlight runtime. It is the local lens used by a
sequential agent loop. The larger machine gives a character awareness of its
surroundings, options, goals, and constraints; lets it act; resolves the action;
mutates state; and then moves to the next agent.

The projection layer turns durable state into a compact character-local pressure
model, then into prompt prose. The prose should describe what the character
knows, wants, fears, protects, misreads, can physically do, and sounds like right
now. It should not recite taxonomy labels unless a debugging mode explicitly asks
for them.

For the sequential runtime target, see
`docs/architecture/sequential-agent-runtime.md`.

## Projection Pipeline

### 1. Select Speaker-Local Inputs

For one acting character in one scene, gather only the state that can affect this
turn:

- character identity and public role
- character canonical state
- character goals and protected values
- character memories selected by scene, relationship, and active pressure
- character perceived overlay of the listener or target
- character-to-listener relationship stance
- listener-to-character relationship stance only if the character has reason to infer it
- scene public facts
- scene hidden facts known to the character
- active public and private stakes
- culture, faction, or institution priors relevant to the scene
- physical affordances, constraints, distance, tools, weapons, bodies, access,
  and social permissions relevant to possible action

Unknown or author-only material is omitted. Known secrets are included only as
character-local pressure: what the character is guarding, why it matters, and
what would happen if it leaked.

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

Do not store defensive speech moves or action moves as canonical traits. Derive
likely response affordances from state intersections.

Responses can be:

- speech
- action
- silence
- withdrawal
- violence
- mixed speech and action

The renderer must not assume a line of dialogue is the right output. Sometimes
the right next move is leaving the room, taking someone's wrist off a console,
opening the airlock interlock, refusing to answer, or slapping someone in the
face because the relationship, status threat, bodily proximity, and inhibition
profile finally made language too slow.

Examples:

| Target behavior | State interaction that can produce it |
| --- | --- |
| Sarcastic deflection | Shame sensitivity + ironic distance + suspicion + exposure pressure + dry or pointed voice. |
| Sincerity evasion | Mask rigidity + low authenticity tolerance + strategic opacity + low emotional explicitness. |
| Emotional evasion | Distance seeking + withdrawal + acute shame or control pressure + low self-disclosure. |
| Banter as boundary | Humor + ironic distance + attachment pressure + distance seeking. |
| Verbal aggression | Hostility + grievance activation + perceived status threat + abrasive boundary. |
| Physical withdrawal | Threat sensitivity + distance seeking + low trust + available exit + low obligation to stay. |
| Coercive interruption | Control pressure + urgency + authority role + high stakes + reachable control surface. |
| Slap or shove | Acute shame or moral disgust + proximity + low inhibition + perceived violation + limited verbal trust. |

These are test targets, not prompt commands. The rendered prompt should describe
the pressure in ordinary language:

```text
If Oz presses too close to the real fear, Cat is likely to answer the useful
part of the question while making the emotional part sound ridiculous.
```

That gives the model a behavioral slope without saying `use sarcastic_deflection`.

For action, use the same rule:

```text
If Veyr calmly describes a possible person as a diagnostic residue while Sella
is close enough to touch him, Sella may stop negotiating and physically break
the rhythm of the room. This should be rare, costly, and grounded in her care
ethic under overload, not random spice.
```

### 5. Compile Projection Controls

Some failures are not fixed by louder prompt scolding. If the response model
keeps adopting the wrong moral frame, inverting who holds an object, giving a
character authority they do not have, or resolving a hidden fact too early, the
projection artifact is missing control structure.

Before rendering prompt prose, compile explicit controls for:

- `frame_controls`: whose language and moral frame the speaker should or should
  not adopt
- `authority_boundaries`: what the speaker can order, request, propose, refuse,
  or only witness
- `object_custody`: who currently holds the flag, weapon, terminal, document,
  body, door, feed, or other consequential surface
- `required_semantics`: compromise meanings that must survive compression, such
  as "hold remains but review starts now"
- `forbidden_resolutions`: unresolved facts or moral categories the turn must
  not settle

These controls are still speaker-local. They are not author omniscience handed
to the response model; they are the projection layer preserving what the
speaker's current state, scene position, and institutional role allow.

The Cold Wake story lab exposed the need for this layer:

- Isdra's first tribunal prompt needed a control saying she should preserve the
  possibility of limited rescue contact without adopting Maer's abandonment
  frame.
- Maer's clinic prompt needed an authority boundary saying he does not own
  Sella's beds.
- Maer's threshold prompt needed object custody saying he holds the
  claimant-status flag and can hand it to Isdra, not ask her for it.
- The Maer/Veyr provenance prompt needed forbidden resolutions fencing off
  person, passenger, victim, witness, and mind as settled nouns.

### 6. Render Prompt Sections

A response prompt should be compact and scene-bound.

Recommended sections:

- `Speaker`: who they are in this scene
- `Setting`: where and when the scene is happening, including only historical
  or cultural context the speaker should have
- `Background`: speaker-local origin, role, class, culture, or embodiment
  context needed to write the turn
- `Known Facts`: only speaker-local facts
- `Current Stakes`: what changes if this goes well or badly
- `Active Inner Pressures`: the strongest motives, fears, wounds, and needs
- `Relationship Read`: how the speaker currently interprets the listener
- `Tensions`: contradictions the speaker is managing
- `Action Affordances`: what the speaker can physically do and what it would
  cost
- `Projection Controls`: frame, authority, object-custody, required semantic,
  and forbidden-resolution constraints that must survive rendering
- `Voice Surface`: baseline speech texture for this moment
- `Likely Response Moves`: ordinary-language pressure outcomes, not schema labels
- `Do Not Invent`: boundaries about facts absent from the pack
- `Task`: the narrow generation request

`Do Not Invent` is not a replacement for positive context. It only tells the
model not to create new facts beyond the pack. It should not list author-only
secrets. If the dialogue model needs setting, historical, cultural,
socioeconomic, or embodiment context to write the line, put that context in the
positive prompt text instead of hoping the model infers it from source refs it
will not receive.

### 7. Generate, Then Evaluate

Generation and evaluation are separate passes.

The response model receives the rendered prompt and writes from the character's
local pressure model. A later evaluator can check whether target emergent
behaviors appeared when the state predicted them. The output may be dialogue,
action, silence, or a beat combining them.

Evaluation questions can include:

- Did the line protect the speaker's known secret without naming unknown facts?
- Did the line reflect the active relationship stance?
- Did the line preserve the voice surface without flattening into a generic bit?
- If the output used action, was the action justified by physical affordances,
  relationship pressure, inhibition, and stakes?
- If the output avoided speech, was silence or movement more plausible than
  another line?
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
