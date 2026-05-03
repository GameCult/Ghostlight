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

## Discipline

- Keep prompts character-local unless the file is explicitly for a coordinator,
  reviewer, or compiler organ.
- Do not include parent conversation context in responder prompts.
- Keep hidden state, future branch plans, raw numeric state vectors, and
  coordinator rationale out of responder-visible prompt text.
- If a prompt is used for a capture, preserve the exact rendered prompt in the
  capture and include the prompt file path when the capture schema allows it.
