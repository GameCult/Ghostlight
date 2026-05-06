# Ghostlight

Ghostlight is a research and implementation home for socially persistent
generative agents.

The project is about characters and simulated people who remain coherent across
time, routine, pressure, memory, culture, misreading, masks, incentives, and
changing relationships. The aim is not to make a language model sound charming
for one scene. The aim is to build machinery that can track who someone is,
what they believe, what they hide, how they see other people, how they live when
nothing is exploding, and how their behavior changes when the world leans on
them.

In practical terms, Ghostlight is building the state and projection layer for
character-driven generation:

- durable canonical character state
- observer-specific perceived state
- directional relationship stance
- cultural and institutional pressure
- memory and goal selection
- speaker-local prompt projection
- evaluation targets for emergent verbal behavior
- a future path for training smaller specialist projection models

The first concrete consumer is Aetheria authoring support, especially Call of
the Void content generation, dialogue scaffolding, and procedural drama for
narrated stories in the Aetheria timeline.

## Core Idea

Ghostlight separates three things that are easy to blur together:

1. **Canonical state**
   What is true about a character, relationship, event, memory, or scene.

2. **Perceived state**
   What a particular observer thinks is true, including misreadings,
   attribution bias, suspicion, trust, projection, and partial knowledge.

3. **Prompt projection**
   A temporary speaker-local prompt surface for one scene or turn.

The prompt is not the character. The prompt is a rendered slice of state.

That distinction matters because believable dialogue should come from character
circumstance, not vibes. Sometimes that circumstance is acute pressure. Often it
is habit, boredom, affection, work rhythm, class embarrassment, ritual memory,
professional competence, petty irritation, or ordinary warmth trying to survive
inside a bleak system. Cat being sarcastic should fall out of shame sensitivity,
distance seeking, suspicion, relationship tension, scene stakes, and voice.
Oz withholding information should fall out of mask rigidity, privacy pressure,
relationship fear, and the need to stay useful. A quiet joke, a domestic aside,
or a moment of tenderness should be just as explainable in state terms as a
threat. The system should be able to show its work without making every scene
sound like Greg Egan having a panic attack in a server closet.

## Current Status

Ghostlight is early architecture plus executable schema seams.

Currently present:

- v0 canonical agent-state schema
- required variable lists for state families and relationship stance
- Call of the Void-flavored Cat/Oz fixture
- dependency-free fixture validator
- prompt projection contract
- projection distillation plan for teacher/student projector training
- persistence surfaces for re-entry, evidence, planning, and compaction hygiene

Not present yet:

- runtime prompt renderer
- projection training dataset
- student projection model
- event/classifier pipeline
- simulation loop
- relationship update engine
- culture prior engine

## How Projection Is Supposed To Work

For one speaker in one scene, Ghostlight should gather speaker-local state:

- what the speaker knows
- what they believe
- what they want
- what they are protecting
- what memories are active
- how they read the listener
- what the relationship currently feels like
- what circumstance the scene creates: routine, desire, scarcity, humor,
  tenderness, danger, institutional friction, or open crisis
- what voice surface is active

The projection layer turns that into compact character-local prose and prompt
sections. The dialogue model then writes from the active circumstance and tonal
mode, not from a universal crisis register.

Longer term, a frontier model can act as a teacher projector that generates
reviewed projection artifacts. Those artifacts can train a smaller student
projector once the projection schema, deterministic input slicer, and evaluator
are stable.

## Repo Tour

- `AGENTS.md`: operating discipline for humans and agents
- `state/map.yaml`: canonical project map and current next action
- `state/scratch.md`: disposable working memory for one bounded subgoal
- `state/evidence.jsonl`: distilled belief-changing evidence
- `notes/fresh-workspace-handoff.md`: compact re-entry packet
- `notes/ghostlight-current-system-map.md`: current repo machinery
- `notes/ghostlight-implementation-plan.md`: larger implementation sequence
- `notes/architecture-rationale.md`: why the persistence and state split exists
- `docs/research/`: research backlog and modeling notes
- `docs/aetheria/`: Aetheria source material used by Ghostlight
- `docs/architecture/`: state, projection, and authoring architecture
- `schemas/`: JSON schema contracts and required field lists
- `examples/`: schema fixtures and future projection examples
- `tools/ghostlight_state.py`: state inspection and evidence helper
- `tools/ghostlight_prepare_compaction.py`: pre-compaction audit helper
- `tools/validate_agent_state.py`: focused fixture validator

## Useful Commands

```powershell
npm run state:status
npm run state:prepare-compaction
npm run schema:validate
```

To add distilled evidence:

```powershell
& 'C:\Users\Meta\.cache\codex-runtimes\codex-primary-runtime\dependencies\python\python.exe' '.\tools\ghostlight_state.py' add-evidence --type design --status accepted --note '...'
```

## Best Starting Points

- `docs/architecture/canonical-agent-state-schema.md`
- `docs/architecture/cultnet-shared-agent-state-contract.md`
- `docs/architecture/agent-state-variable-glossary.md`
- `docs/architecture/prompt-projection-contract.md`
- `docs/architecture/projection-distillation-plan.md`
- `examples/agent-state.call-of-the-void.json`

For Codex-driven work, read `AGENTS.md` before changing the repo. It contains
the working rules that keep the project state coherent between sessions.
