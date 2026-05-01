# Ghostlight

Ghostlight is where the stranger agent work gets to live.

The project is about socially persistent generative agents: layered personality,
cultural priors, masks, memory, misreadings, incentives, and prompt projection
that grows out of structured latent state instead of one wet persona paragraph
trying to hold up the ceiling.

The immediate goal is not a finished runtime. The immediate goal is a repo that
can carry the architecture cleanly enough that the runtime does not become a
beautiful little superstition.

## What Lives Here

- Epiphany-style persistent state surfaces for re-entry, compaction, and
  distilled evidence.
- Architecture notes for agent state, signal classification, culture-shaped
  distributions, and prompt projection.
- A bounded working discipline that keeps maps, scratch, evidence, and plans
  from dissolving into one confident soup.

## Repo Tour

- `AGENTS.md`: operating discipline for humans and agents
- `state/map.yaml`: canonical project map
- `state/scratch.md`: disposable local working memory
- `state/evidence.jsonl`: distilled belief-changing evidence
- `notes/fresh-workspace-handoff.md`: compact re-entry packet
- `notes/ghostlight-current-system-map.md`: source-grounded explanation of the
  live repo machinery
- `notes/ghostlight-implementation-plan.md`: current larger-organ sequence
- `notes/architecture-rationale.md`: why the state split exists at all
- `tools/ghostlight_state.py`: compact state helper
- `tools/ghostlight_prepare_compaction.py`: pre-compaction audit helper

## Quick Commands

```powershell
npm run state:status
npm run state:prepare-compaction
& 'C:\Users\Meta\.cache\codex-runtimes\codex-primary-runtime\dependencies\python\python.exe' '.\tools\ghostlight_state.py' add-evidence --type research --status ok --note '...'
```

If you are steering the repo through Codex, read `AGENTS.md` first. That is the
operating manual. The README is just the sign on the door.

