# Ghostlight Implementation Plan

## Near-Term Sequence

1. Migrate the architecture that already proved worth keeping.
   - Move the relevant research and architecture notes out of VoidBot and into
     Ghostlight.
   - Keep the copies clean and Ghostlight-native instead of leaving them full
     of cross-repo apology notes.

2. Freeze the first canonical state model.
   - Define the latent variable families.
   - Define first-class dimensions, including volatility, attachment-seeking,
     and distance-seeking.
   - Define canonical state versus perceived state.

3. Freeze the first event and classifier schema.
   - Define event labels.
   - Define canonical expression labels.
   - Define listener perception labels.
   - Define mask or presentation labels.
   - Define confidence, uncertainty, and evidence accumulation rules.

4. Build the prompt projection layer.
   - Render prompts from structured state, relevant memories, scene pressure,
     cultural defaults, and perceived state.
   - Keep the prompt renderer a projection surface instead of a second brain.

5. Build the simulation update loop.
   - memory updates
   - relationship updates
   - goal pressure
   - situational activation
   - cultural and institutional pressure

6. Build the first Aetheria slice.
   - use the bounded corridor scenario already outlined elsewhere
   - prove that a small cast, cohort states, and ambient pressure can make a
     place feel inhabited

## Discipline

- Do not start with an MMO and call it ambition.
- Do not train a classifier before the label schema is stable enough to deserve
  the trouble.
- Do not let prompt-writing outrun the structured state model.
- Do not let elegant theory notes multiply faster than implementation seams.

