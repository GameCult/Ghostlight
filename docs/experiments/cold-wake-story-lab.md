# Cold Wake Story Lab

This experiment uses Ghostlight as a writing crutch on purpose.

The goal is to write a small Cold Wake story while producing reusable projection
training data. The story should have a protagonist, themes, locations, an arc,
and enough room for characters to act badly for reasons the state can explain.

This is not a benchmark in a clean little lab coat. It is a knife test.

## Story Premise

During the Cold Wake Panic, a quiet-running vessel enters a Jovian convoy
corridor under contradictory thermal classification. The PSC tribunal freezes
its corridor status. A Navigator route steward pushes to keep rescue live. An
Aya-aligned sanctuary coordinator sees a possible personhood claim in a corrupted
packet. A Cognitum shell-vendor liaison tries to keep an implicated routing
artifact framed as derivative firmware instead of testimony.

The protagonist is **Maer Tidecall**, a Cetacean Navigator route steward whose
core problem is not whether to be kind. Maer's problem is that rescue obligation
has to survive contact with institutions that can punish ambiguity faster than
they can understand it.

## Themes

- rescue obligation versus category law
- personhood trying not to get filed as firmware
- care as material capacity, not decorative mercy
- institutional fear disguised as neutral procedure
- language as a battlefield where a person can be erased without anyone raising
  their voice

## Arc

### 1. Freeze

Location: Ganymede relay tribunal chamber.

Maer challenges PSC adjudicator Isdra Vel after she issues a conditional risk
hold on the vessel. The scene should establish the central wound: the corridor
needs categories, but categories can turn rescue delay into lawful abandonment.

### 2. Intake

Location: Aya-aligned clinic behind Ganymede dockside pumpworks.

Maer asks Sella Ren to hold sanctuary capacity for a rescue that may never be
authorized. Sella refuses to let care become fantasy. The story should force
Maer to see that rescue language can push costs onto care workers unless it names
the material burden.

### 3. Provenance

Location: Callisto spillover compliance office.

Maer confronts Veyr Oss, a Cognitum shell-vendor liaison whose routing artifact
may explain the vessel's contradictory behavior. Veyr tries to keep the artifact
inside product language. This is the scene most likely to produce nonverbal
action if the pressure earns it: silence, leaving, blocking a terminal, grabbing
a wrist, or violence. The system must not assume one more clever line is always
the answer.

### 4. Threshold

Location: Jovian convoy threshold as the vessel's heat-debt window closes.

Maer returns to Isdra with enough suspicion to complicate the freeze but not
enough proof to make the room clean. The ending should not be a total victory.
The viable resolution is a narrow, institutionally usable rescue contact that
preserves evidence and life without pretending the category system has healed.

## Experiment Loop

For each scene:

1. Define character-local state and scene pressure.
2. Project the next response prompt for one character.
3. Send only `prompt_text` to the Qwen response model.
4. Save the model output.
5. Review whether the output served the story, respected speaker-local context,
   and chose speech/action/silence plausibly.
6. Tweak the projection pipeline if the output fails in a way the pipeline
   should have prevented.
7. Save accepted and rejected examples as training material.

## Qwen Target

Use the LAN Ollama endpoint from `gamecult-ops`:

- base URL: `http://192.168.1.84:11434`
- model: `qwen3.5:9b`
- default context: `8192`
- use request files for `curl.exe` calls so PowerShell does not eat the JSON and
  burp out theater.

## Output Folders

- agent-state fixture: `examples/agent-state.cold-wake-story-lab.json`
- projection examples: `examples/projections/cold-wake-story-lab.jsonl`
- readable projection mirror: `examples/projections/cold-wake-story-lab.pretty.json`
- generated response captures: `experiments/cold-wake-story-lab/`

## Acceptance Criteria

- The story has a visible arc, not just disconnected argument samples.
- Projection prompts include enough setting and cultural pressure for Qwen to
  write without seeing the lore archive.
- At least one projected turn permits action instead of dialogue.
- Review notes explicitly mark whether Qwen chose speech, action, silence, or a
  mixed beat.
- Failures are kept when they teach a useful boundary.
