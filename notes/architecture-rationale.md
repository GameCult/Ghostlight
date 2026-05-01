# Architecture Rationale

This note explains why Ghostlight uses explicit state surfaces. It is not the
current implementation map and not the roadmap. For those, read:

- `state/map.yaml`
- `notes/fresh-workspace-handoff.md`
- `notes/ghostlight-implementation-plan.md`
- `notes/ghostlight-current-system-map.md`

## Core Failure

The central failure mode is local plausibility without global coherence.

An agent can write elegant little sentences, produce plausible little diffs,
and still lose the shape of the machine. Once the model no longer knows how the
latent state, memory, perception, and prompt layers fit together, adding more
local machinery usually makes the beast prettier and stupider.

The answer is not more transcript. The answer is explicit structure:

- slow-changing map
- disposable scratch
- distilled evidence
- compact handoff
- implementation plan that is not pretending to be a second map
- ruthless deletion when state stops improving the model

## State Split

Ghostlight treats different kinds of cognition as different artifacts:

- `map`: canonical system model, invariants, accepted design, rejected paths
- `scratch`: temporary local reasoning for one bounded subgoal
- `evidence`: distilled proof, decisions, rejected paths, and scars that change future belief
- `system map`: source-grounded explanation of the live repo machinery
- `handoff`: compact re-entry packet
- `plan`: distilled forward implementation direction
- `output`: user-visible replies, code edits, commits, and verification artifacts

The split matters because a language model will happily blend all of that into
one warm slurry if invited. Slurry is not architecture.

## Current Harness Principle

Ghostlight is using the same basic discipline that Epiphany proved useful:

```text
observe the repo and canonical state
-> rehydrate the current map
-> work one bounded organ
-> verify the seam that matters
-> record only durable evidence
-> cut failed code and failed memory
```

The model is still a language model. That is the medium, not the scandal.
Language, structure, evidence, and salience are the steering surfaces. The repo
exists to keep those surfaces explicit enough that the next waking thing can
resume the pattern instead of faking continuity from vibes.

## Compact-Rehydrate-Reorient

Compaction is a real state transition, not a cosmetic cleanup.

Before compaction, Ghostlight should preserve:

- current objective and active subgoal
- latest stable map
- distilled evidence or rejected paths that change future belief
- open questions, blockers, and next action

After rehydrating, Ghostlight should:

- reread canonical state instead of trusting prompt residue
- restate the next action from persisted state
- continue only when instructed or when the task explicitly calls for it
- avoid broad implementation until the current mechanism is understood again

The aim is not immortality cosplay. It is banking the fire enough that the next
waking thing finds coals instead of ash.

