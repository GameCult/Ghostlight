# Corpus Coverage Ledger

The corpus coverage ledger is the project-level index for accepted and planned
Aetheria training fixtures.

Its job is not to store story content. Its job is to keep the corpus honest:
which factions, movements, flashpoints, timeline lanes, tonal modes, collision
axes, review states, and item/economy hooks are represented, and which are still
missing.

## Live Files

- Ledger: `state/corpus-coverage.json`
- Schema reference: `schemas/corpus-coverage.schema.json`
- Validator/status tool: `tools/validate_corpus_coverage.py`
- Command: `npm run coverage:status`

## Entry Contract

Each fixture entry records:

- `fixture_id`: stable fixture handle.
- `status`: `planned`, `draft`, `accepted_as_draft`, `accepted`,
  `needs_revision`, `rejected`, or `superseded`.
- `timeline_lane`: `historical_grounded`, `transition_grounded`, or
  `future_branch`.
- `canon_status`: single-history, branch-local, promoted branch, template, or
  draft status.
- `story_type`: founding-era, day-in-the-life, flashpoint, movement,
  future-branch, or trade-life story.
- `primary_subjects` and `secondary_subjects`: factions, movements,
  institutions, species/body groups, or other represented subjects.
- `collision_axes`: the pressures the fixture exercises, such as law, labor,
  care, class, logistics, species/body affordance, trade, memory, personhood,
  technology, legitimacy, or infrastructure.
- `tonal_modes`: prose modes the fixture contributes to the corpus.
- `training_targets`: organs this fixture can train or review.
- `artifacts`: paths to fixture files.
- `review`: status of schema, narrative, lore, spatial, visual, and IF review.
- `economy_hooks`: item, assembly, trade-good, supply-chain, or tech hooks.
- `open_lore_gaps`: blocking or useful gaps discovered during generation.

## Acceptance Rule

An accepted or accepted-as-draft fixture must have the core artifact refs:

- lore digest
- agent state
- coordinator artifact
- Ink file
- training sidecar

It must also carry review fields for schema, narrative, lore, spatial, visual,
and IF artifact review. A fixture may still be accepted-as-draft with review
caveats, but the caveats should be visible instead of hidden under the nice rug.

## Report Shape

`npm run coverage:status` validates the ledger and prints:

- total entries and accepted count
- progress toward the 100-150 broad target
- status, timeline lane, story type, collision-axis, tonal-mode, and subject
  counts
- major-power founding/day-in-life candidates when present
- basic gap signals, such as missing future-branch coverage or no day-in-life
  stories yet

This is deliberately a coarse instrument. It should tell us where to point the
next fixture without pretending to be a corpus scientist in a lab coat made of
spreadsheet tabs.
