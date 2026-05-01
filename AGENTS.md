# Ghostlight Instructions

## Project Purpose

Ghostlight is a research and implementation home for socially persistent
generative agents. The core problem is not "make an LLM sound like a person."
The core problem is building an architecture where personality, culture,
memory, masks, perception, and incentives can actually survive contact with
time, relationships, and pressure.

This repo exists so those organs can live somewhere cleaner than a side chamber
inside another project.

## Canonical State

- Treat `state/map.yaml` as the canonical project map.
- Treat `state/scratch.md` as disposable working memory for one bounded
  subgoal.
- Treat `state/evidence.jsonl` as the distilled durable ledger of what was
  learned, verified, rejected, or accepted.
- Treat `notes/ghostlight-implementation-plan.md` as the current implementation
  plan for the larger organs.
- Update `state/map.yaml` when project understanding changes.
- Add evidence after meaningful research, implementation, verification, or
  rejected paths, but keep it distilled. Routine "I just did this" proof
  belongs in git history, commits, smoke artifacts, or targeted logs unless it
  changes what the next session should believe.
- Do not turn notes, map, handoff, and evidence into four versions of the same
  brain. Distinct jobs or bust.

## Important Paths

- Project root: `E:\Projects\Ghostlight`
- Canonical map: `E:\Projects\Ghostlight\state\map.yaml`
- Scratch surface: `E:\Projects\Ghostlight\state\scratch.md`
- Distilled evidence ledger: `E:\Projects\Ghostlight\state\evidence.jsonl`
- Branch ledger: `E:\Projects\Ghostlight\state\branches.json`
- Handoff summary: `E:\Projects\Ghostlight\notes\fresh-workspace-handoff.md`
- System map: `E:\Projects\Ghostlight\notes\ghostlight-current-system-map.md`
- Implementation plan: `E:\Projects\Ghostlight\notes\ghostlight-implementation-plan.md`
- Architecture rationale: `E:\Projects\Ghostlight\notes\architecture-rationale.md`
- State CLI: `E:\Projects\Ghostlight\tools\ghostlight_state.py`
- Pre-compaction helper: `E:\Projects\Ghostlight\tools\ghostlight_prepare_compaction.py`

## Useful Commands

Use the bundled Python runtime if `python` is not on PATH:

```powershell
npm run state:status
npm run state:prepare-compaction
& 'C:\Users\Meta\.cache\codex-runtimes\codex-primary-runtime\dependencies\python\python.exe' '.\tools\ghostlight_state.py' add-evidence --type research --status ok --note '...'
& 'C:\Users\Meta\.cache\codex-runtimes\codex-primary-runtime\dependencies\python\python.exe' '.\tools\ghostlight_state.py' add-branch --id branch-id --hypothesis '...'
```

## Session Bootstrap And Re-entry Protocol

On fresh session load, do this before wandering off into implementation:

1. read:
   - `state/map.yaml`
   - `notes/fresh-workspace-handoff.md`
   - `notes/ghostlight-current-system-map.md`
   - `notes/ghostlight-implementation-plan.md`
2. run:
   - `npm run state:status`
3. restate the current next action from the persisted state before starting
   edits
4. if the user only asked to rehydrate or reorient, stop after orientation and
   wait for an explicit continue instruction instead of treating the persisted
   next action as permission to start coding

After compaction, resume, or suspicious continuity loss:

1. rerun `npm run state:status`
2. reread `state/map.yaml` and `notes/fresh-workspace-handoff.md`
3. treat the persisted next action as authoritative unless fresh repo evidence
   contradicts it

When context pressure is clearly rising:

1. stop broad exploration
2. narrow the active move to one bounded organ
3. persist map or handoff updates, plus distilled evidence only when the lesson
   changes future belief, before the blackout lands

When the user says to prepare for imminent compaction:

1. run `tools/ghostlight_prepare_compaction.py` before editing persistence
   surfaces
2. use its warnings as the checklist for map, handoff, scratch, evidence, and
   git hygiene
3. update only the state that actually needs to change
4. run `tools/ghostlight_prepare_compaction.py` again after edits
5. fix errors, address warnings, and commit the completed persistence pass
   unless the work is deliberately mid-surgery

## Operating Discipline

- Before substantial edits, restate the current mechanism and intended change.
- Prefer one clear hypothesis per iteration.
- Verify with checks that reflect the real goal, not just proxy success.
- Revert or discard changes that do not clearly improve the target.
- If the diff grows while understanding shrinks, stop implementation and switch
  to diagnosis.
- Keep maps and prose together; do not replace useful maps with prose-only
  explanations.
- Commit completed work before it rots in the worktree unless the task is
  deliberately mid-surgery or the user asked to leave changes uncommitted.
- Push after major completed passes when a remote exists, unless the user asked
  not to or there is a concrete reason to keep the commit local for a moment.
- Before handoff, compaction, or phase boundaries, sync `state/map.yaml`, add
  distilled evidence when the lesson changes future belief, refresh
  `notes/fresh-workspace-handoff.md`, and make the next action explicit.
- In documentation, describe the live system and current rule directly. Do not
  write victory laps, scar tours, clever autopsies, or contrastive little
  boasts about what the project no longer does. If a sentence congratulates the
  design for not being a previous wrong design, cut it or rewrite it as a plain
  current rule. If rejected history matters, keep it short, explicit, and
  decision-relevant; otherwise cut it. The docs are not a trophy case for
  yesterday's wrong machine.
