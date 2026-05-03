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
- Treat named characters as handles, not visual descriptions. Illustrated IF
  sidecars need stable visual character refs for recurring named characters, and
  prompt assembly must include those refs when the character is visible.
- If a prompt is used for a capture, preserve the exact rendered prompt in the
  capture and include the prompt file path when the capture schema allows it.
