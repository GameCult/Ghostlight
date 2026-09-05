# Scratch Working Memory

## Current Subgoal

Seed producer (operator order, 2026-09-05). Pipeline: Modeling
(`modeling-seed.md` in the session scratchpad) -> Imagination -> Hands in an
isolation worktree -> Soul in that tree -> integrate onto
`codex/ghostlight-dungeon-mvp`. Nothing edited yet at `f9a1c3e`.

Before Hands cuts, the spec must settle: how `WorldScaleIntent` reaches genesis
(`CreateWorldIntent` has none; genesis uses `WorldScaleIntentRef::default()`;
the genesis patch lane carries `Option<WorldScaleIntentRef>`), and how a Draft
patch is bound when `SeedRequest` is uninhabited and Draft answers nothing.

## Working Notes

Use this file for one bounded slice. Delete or reset aggressively when the
slice is done.
