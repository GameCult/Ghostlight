# Ghostlight Instructions

## Project Purpose

Ghostlight is a research and implementation home for socially persistent
generative agents. The core problem is not "make an LLM sound like a person."
The core problem is building an architecture where personality, culture,
memory, masks, perception, and incentives can actually survive contact with
time, relationships, routine, tonal variation, and pressure.

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

1. immediately write hot live context from memory into a new
   `state/scratch-compaction-<guid>.md` file before reading files, running
   status checks, or doing tidy persistence work
2. run `tools/ghostlight_prepare_compaction.py` before editing persistence
   surfaces
3. use its warnings as the checklist for map, handoff, scratch, evidence, and
   git hygiene
4. update only the state that actually needs to change
5. after hot context has been folded into canonical persisted state, delete the
   `state/scratch-compaction-<guid>.md` file so stale emergency memory does not
   linger
6. run `tools/ghostlight_prepare_compaction.py` again after edits and scratch
   cleanup
7. fix errors, address warnings, and commit the completed persistence pass
   unless the work is deliberately mid-surgery

## Operating Discipline

- Steward persistent state as part of the work. If a lesson changes future
  behavior, update the smallest correct memory surface before moving on; if a
  memory surface is stale, duplicated, or making re-entry slower, cut or
  consolidate it.
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
- Treat `state/map.yaml` as a working memory map, not an archive. Preserve
  current mission, live seams, boundaries, and next action; let git history,
  evidence, and artifact files carry chronology.
- In documentation, describe the live system and current rule directly. Do not
  write victory laps, scar tours, clever autopsies, or contrastive little
  boasts about what the project no longer does. If a sentence congratulates the
  design for not being a previous wrong design, cut it or rewrite it as a plain
  current rule. If rejected history matters, keep it short, explicit, and
  decision-relevant; otherwise cut it. The docs are not a trophy case for
  yesterday's wrong machine.

## Aetheria Grounding Doctrine

- Before writing or generating Aetheria scenes, check AetheriaLore for the
  factions, movements, institutions, species or body types, location, and time
  period involved.
- Before drafting story prose, choose an intentional tonal mode or blend for the
  scene. Aetheria should support wit-with-stakes, warmth, absurdity, domestic
  texture, ritual quiet, wonder, horror, noir, dour poetry, and dry systems
  prose as needed. Do not default to constant crisis or procedural severity.
- Use Adams/Pratchett-style comic precision and humane levity as a useful
  default touchstone when no stronger scene-specific style is called for:
  jokes and absurdity should reveal stakes, not dissolve them.
- Establish ordinary life before interruption when the fixture calls for it:
  routines, work texture, relationships, private jokes, rituals, small desires,
  and local accommodations are part of worldbuilding, not filler.
- For illustrated Ink fixtures, segment prose into click-through visual
  sections. Distinct places, focal areas, object closeups, arrivals, alarms,
  pivotal one-sentence beats, and aftermath states need stable visual scene ids
  and imagegen-ready prompts.
- Treat visual artifacts as illustrated replay and website collateral, not core
  social-model training signal. Visual review judges the playable Ink
  experience plus the visual plan, not clean-run prose receipts.
- Write visual prompts for a general image model, not a lore expert. Explain
  fictional objects and body affordances in visible terms: geometry, materials,
  lighting, stance, interfaces, tools, supports, cables, fluids, screens, and
  crowd positions.
- Do the same for environments. A location label like "manifold line," "cavity
  yard," "supervisor glass," or "anchor rail" is not a drawable scene until the
  prompt describes its visible geometry, materials, interfaces, lights, and
  scale.
- If a fixture has a target visual style, store that style cue once in the
  sidecar and include it in every assembled image prompt. Do not rely on memory
  or unstated website context to carry the look.
- Do not treat a character name as a visual description. Illustrated fixtures
  need stable character visual refs for every recurring named character, and
  scene prompts should either include those refs during prompt assembly or carry
  enough appearance detail to stand alone.
- Include visual character refs only for characters actually visible in the
  rendered frame. Do not feed imagegen refs for absent, off-frame, merely
  mentioned, or object-represented characters.
- Treat visual continuity notes as tool/reviewer constraints, not prompt text.
  If imagegen needs the fact, rewrite it as concrete visible description in the
  base prompt, character ref, or branch/state modifier.
- Treat generated environment images as concept and material reference, not
  spatial truth. Multi-angle or branch-consistent illustrated IF needs a durable
  scene-set source such as a hand-built blockout, procedural scene file, layout
  asset, or camera map. Imagegen can paint over camera-specific visual input;
  it should not be asked to preserve room topology from a prior generated
  image or text memory.
- Ground scene writing in the source's material details: habitat form,
  infrastructure, law, class pressure, money, route access, surveillance,
  communication channels, body affordances, and local failure costs.
- If the source is too vague for a concrete scene detail, do not bluff. Mark
  the lore gap, patch the AetheriaLore source with a narrow elaboration, then
  regenerate or revise the Ghostlight artifact from that patched source.
- Treat this as part of data generation. Ghostlight is expected to stress-test
  Aetheria at room scale, find missing granularity, and push useful
  elaborations back into the lore vault.
