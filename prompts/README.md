# Ghostlight Prompts

This folder stores reusable prompts for sandboxed one-turn workers, sub-agents,
and model calls.

The prompt files here are templates or named launch prompts. Output captures may
still preserve exact prompt text inline as receipts, but new prompt surfaces
should be authored here first and referenced from artifacts or tools.

## Files

- `sandboxed-responder-packet.md`: template used by
  `tools/build_responder_packet.py` to render responder-visible packet prompts.
- `research-sanity-responder.md`: reusable one-turn prompt for checking whether
  a research-enabled responder actually grounds behavior in allowed
  AetheriaLore scope.
- `narrative-quality-reviewer.md`: reviewer prompt for reader-facing story and
  IF coherence, including onboarding, character introduction, branch-fold
  clarity, pacing, voice, and lore accessibility.
- `visual-scene-continuity-reviewer.md`: reviewer prompt for illustrated IF
  replay, click-through segmentation, imagegen-ready scene prompts, character
  visibility, stance control, and branch/state visual modifiers.

## Discipline

- Keep prompts character-local unless the file is explicitly for a coordinator,
  reviewer, or compiler organ.
- Do not include parent conversation context in responder prompts.
- Keep hidden state, future branch plans, raw numeric state vectors, and
  coordinator rationale out of responder-visible prompt text.
- Treat tonal intent as prompt-relevant state for story, coordinator, reviewer,
  and branch compiler prompts. Do not let prompt templates imply that every
  character is acting inside a crisis. Routine, wit, warmth, ritual, boredom,
  wonder, and tenderness can be just as state-grounded as fear.
- Treat visual segmentation as prompt-relevant state for illustrated IF. A
  fixture can need several click-through screens inside one narrative scene;
  each distinct place, focal area, pivotal moment, and branch-visible state
  needs an imagegen-ready prompt or modifier.
- Treat visual review as IF replay review, not clean-run review. Clean-run
  renderings are receipts and debugging mirrors; the visual reviewer judges
  playable Ink plus the visual plan artifact.
- Treat visual artifacts as entertainment and website collateral for the
  Ghostlight corpus. They can make the corpus more legible and valuable, but
  they are not core training targets for responder, appraiser, mutator, or
  relationship models.
- Treat visual style as prompt-relevant state when the fixture needs a specific
  look. Store the style cue once in the visual plan artifact and include it
  during prompt assembly for every generated image.
- Treat named characters as handles, not visual descriptions. Illustrated IF
  visual plans need stable visual character refs for recurring named
  characters, and prompt assembly must include those refs only when the
  character is actually visible in the rendered frame.
- Treat continuity notes as tooling constraints, not prompt text. If the image
  model needs a continuity fact, put it in visible terms inside the base prompt,
  character ref, or modifier.
- Treat environment labels as handles, not images. A prompt that names a
  location or fixture must also describe its visible geometry, materials,
  lights, interfaces, and scale.
- If a prompt is used for a capture, preserve the exact rendered prompt in the
  capture and include the prompt file path when the capture schema allows it.
