# Ghostlight Implementation Plan

## Near-Term Sequence

1. Freeze the first canonical state model.
   - Define the latent variable families.
   - Define first-class dimensions, including volatility, attachment-seeking,
     and distance-seeking.
   - Define canonical state versus perceived state.
   - Keep authoring outputs explainable from the state, not just generated from
     stylish prompt fog.

2. Freeze the first event and classifier schema.
   - Define event labels.
   - Define canonical expression labels.
   - Define listener perception labels.
   - Define mask or presentation labels.
   - Define confidence, uncertainty, and evidence accumulation rules.

3. Build the prompt projection layer for authoring.
   - Render scene and dialogue scaffolds from structured state, relevant
     memories, scene pressure, cultural defaults, and perceived state.
   - Keep the prompt renderer a projection surface instead of a second brain.
   - Make generated moves inspectable: why this character deflects, confesses,
     threatens, softens, bargains, or lies.

4. Build the first drama-scaffolding loop.
   - memory updates
   - relationship updates
   - goal pressure
   - situational activation
   - cultural and institutional pressure
   - conflict beats derived from incompatible goals and values

5. Build the first Aetheria authoring consumer.
   - use the Call of the Void slice described in `AetheriaLore` as the first
     concrete content-generation context
   - support dialogue scaffolding and procedural drama for narrated stories
     across the Aetheria timeline
   - do not treat an older corridor-crisis game slice as the implementation
     target unless explicitly revived later

## Discipline

- Do not start with an MMO and call it ambition.
- Do not train a classifier before the label schema is stable enough to deserve
  the trouble.
- Do not let prompt-writing outrun the structured state model.
- Do not let elegant theory notes multiply faster than implementation seams.
- Do not accidentally rebuild a game slice when the requested consumer is an
  authoring and story-generation organ.
