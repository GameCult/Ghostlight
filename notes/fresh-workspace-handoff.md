# Ghostlight Fresh Workspace Handoff

Do not trust this file for the exact live HEAD. Use git for volatile truth.

Do not continue implementation automatically from a rehydrate-only request.

## Immediate Re-entry Instruction

1. Read `state/map.yaml`.
2. Read `notes/ghostlight-current-system-map.md`.
3. Read `notes/ghostlight-implementation-plan.md`.
4. Run `npm run state:status`.
5. Restate the persisted next action before touching code or docs.

## Current Shape

Ghostlight now has the persistence spine plus the first architecture payload:

- canonical map
- scratch
- evidence ledger
- branch ledger
- compact re-entry note
- system map
- implementation plan
- Python helpers for state summary and compaction audits
- architecture notes for:
  - personality structure
  - signal classification
  - state distributions and prompt projection
  - Aetheria content generation targets
  - population scaling for socially dense authored slices

There is still no runtime yet. The live machine is design, state discipline,
and a cleaner repo boundary.

## Current Direction

Use Ghostlight first for:

- content generation inside the Call of the Void slice described in
  `AetheriaLore`
- dialogue scaffolding and procedural drama for narrated Aetheria timeline
  stories

An older corridor-crisis game slice is not the current build target. It can
remain fertile story ground, but it should not steer Ghostlight's implementation
state unless explicitly revived.

## Current Next Action

Pick the first implementation seam to cut:

- state schema
- perceived-state overlay
- classifier/event data model
- prompt projection renderer for authoring outputs

The clean first move is still the canonical agent-state schema.

## Warnings

- Do not let this repo become a mirror maze of notes that say the same thing
  in slightly different outfits.
- Keep canonical state small. If evidence starts turning into an activity feed,
  cut it back.
- Keep the prompt-projection doctrine intact: prompt text is projection, not
  truth storage.
- Do not accidentally re-promote the older corridor-crisis idea into a game MVP
  because an old doc had a strong chin and good lighting.
