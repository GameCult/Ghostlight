# VoidBot Routine Adoption Plan

VoidBot now has the best live reference for the agent routine Ghostlight needs:
work or rumination on heartbeat, periodic sleep for consolidation, typed
MessagePack persistence through a CultCache-style contract, and vector-backed
memory resonance. Ghostlight should adopt the routine without inheriting
VoidBot's moderation job.

## Imported Organs

- **Heartbeat scheduler:** chooses work, rumination, or sleep/consolidation from
  readiness, load, status, reaction windows, and pending turns.
- **Completion-gated cooldown:** a participant cannot act again until its
  previous heartbeat turn finishes; recovery starts at completion.
- **Sleep cycle:** scheduled naps are not absence. They compress memory,
  strengthen useful associations, cool saturated themes, refresh dreams, and
  keep direct speech rare.
- **CultCache persistence:** canonical state is MessagePack, routed by typed
  document definitions, with JSON working projections for inspection/review.
- **Vector memory:** memories carry semantic vector metadata and Qdrant points
  so recall, resonance, incubation, and dreams are grounded in retrievable
  structure.
- **Thought organs:** analytic lane, associative lane, bridge, memory resonance,
  incubation, and candidate intervention/dream queues.

## Ghostlight Translation

Ghostlight's swarm is the cast. The heartbeat initiative system therefore
applies directly to characters taking turns within a scene, not merely to
background maintenance workers.

There are two scopes using the same timing law:

- **scene heartbeat:** selects which in-scene character acts, reacts, waits,
  hesitates, refuses, speaks, withholds, moves, or uses an object.
- **maintenance heartbeat:** wakes offstage agents, cast records, coordinator
  organs, appraisers, mutators, and fixture builders for rumination, sleep
  consolidation, or reviewed maintenance.

The point is one initiative physiology with different arenas, not two unrelated
schedulers wearing each other's hat.

```text
heartbeat initiative
  -> choose scene participant or maintenance organ
  -> scene turn | reaction | wait | rumination | sleep
  -> read typed state from CultCache-compatible store
  -> optional Qdrant recall / memory resonance
  -> produce visible action, event, appraisal, mutation, dream, or maintenance receipt
  -> completion starts cooldown
  -> write canonical MessagePack + JSON working projection
```

Epiphany now owns the first shared Rust heartbeat spine. Ghostlight should call
that spine through `tools/ghostlight_epiphany_heartbeat.mjs` instead of growing a
parallel scheduler. The Ghostlight-side command derives scene participants from
the existing initial/agent-state fixtures, initializes a typed MessagePack
`ghostlight-scene` heartbeat store, and asks Epiphany to emit
`ghostlight.initiative_schedule.v0` receipts with `scene_turn` actions. Those
receipts select the actor and local-turn machinery; responder, appraisal, and
mutation organs still own the actual narrative action.

## Persistence Shape

Ghostlight can start with the CultCacheTS contract directly for Node-side tools,
but the Python-heavy validator/tooling surface needs a compatible file contract:

- canonical store: single-file MessagePack envelope array
- envelope: `key`, `type`, `payload`, `storedAt`
- schema owner: code definitions, not hand-edited loose JSON
- working projection: JSON, human-readable, safe for review, optionally stripped
  of bulky vector values
- write discipline: use an external lock or one coordinator writer; the
  single-file store is not a database wearing a fake mustache

Initial document types:

- `ghostlight.agent_state`
- `ghostlight.scene_state`
- `ghostlight.initiative_schedule`
- `ghostlight.scene_heartbeat_state`
- `ghostlight.training_loop_bundle`
- `ghostlight.runtime_heartbeat_state`
- `ghostlight.memory_resonance`
- `ghostlight.sleep_cycle`

## Memory Vector Shape

Qdrant becomes the retrieval layer for agent memory, not canonical social truth.

Each memory-like record should keep:

- stable memory id
- kind: episodic, semantic, relationship, goal, value, musing, dream,
  scene-memory, appraisal, mutation, or lore-grounding
- summary text used for embedding
- source hash
- embedding backend/model/dimensions
- Qdrant point id and collection
- salience/confidence/update time
- source refs into fixtures, lore, or reviewed artifacts

Ghostlight's existing training docs already point toward retrieval-backed memory.
This plan makes it operational instead of aspirational, the traditional route
by which a research note stops being furniture.

## Sleep Behavior

During sleep turns:

- do not generate ordinary scene turns unless the character is actually in the
  scene and selected by the scene heartbeat
- inspect memory resonance clusters and incubation entries
- merge duplicate memories
- prune stale or low-confidence residue
- create or refresh at least one dream only when it compresses a useful seam
- update `sleep_cycle.activeDreamThemes`, `lastDreamAt`, and
  `lastDistillationSummary`
- write a maintenance receipt for review

Dreams are compressed operational memory, not glitter. If a dream survives, the
next waking turn should be more coherent because of it.

## Migration Slices

1. **Contract fixture**
   - Add a small CultCache-compatible MessagePack fixture and validator.
   - Prove Python can read/write the envelope without losing schema shape.

2. **Runtime heartbeat schema**
   - Add `schemas/runtime-heartbeat-state.schema.json`.
   - Include pending turn, sleep cycle, rumination, and completion receipts.

3. **Agent state migration**
   - Keep current JSON examples as projections.
   - Add canonical MessagePack test store for one small agent-state fixture.

4. **Memory resonance**
   - Add a local memory-organ script that embeds memory summaries, writes Qdrant
     points, builds resonance edges/clusters, and writes incubation state.

5. **Sleep receipt**
   - Add a sleep/consolidation artifact schema and validator.
   - Require sleep receipts to name pruned/merged/strengthened memories.

6. **Training-loop integration**
   - Promote scene initiative schedules into the same heartbeat contract used
     by the swarm.
   - Let characters take scene turns through heartbeat initiative, then route
     appraisal/mutation/coordinator maintenance through the same completion and
     cooldown law.

## Boundaries

- Do not make Qdrant canonical truth.
- Do not store raw vector blobs in every human working projection.
- Do not let sleep turns mutate accepted training fixtures without reviewed
  maintenance receipts.
- Do not make every Ghostlight character into a background daemon before the
  runtime heartbeat store can prove who is awake, asleep, pending, or complete.
