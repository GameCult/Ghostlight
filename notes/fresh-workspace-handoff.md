# Ghostlight Fresh Workspace Handoff

Updated: 2026-09-01

This is the compact re-entry packet. It carries current authority and the next
gate. Git owns chronology, the system map owns teardown detail, and evidence
records own durable findings.

## Immediate re-entry

1. Work from `F:\Projects\Ghostlight`.
2. Run `git status --short`, `git log -1 --oneline`, and
   `npm run state:status`.
3. Read `state/map.yaml` (`current_status` first), this handoff, and
   `docs/architecture/ghostlight-dungeon-mvp.md`.
4. Read `notes/ghostlight-current-system-map.md` only when exact pre-rebuild
   ownership or deletion evidence is needed.
5. For deployment, SSH, Idunn, Odin, Heimdall, or host work, consult
   `F:\Projects\gamecult-ops` before acting.

If the operator asks only to rehydrate, report the current gate and stop. A
persisted next action is not permission to begin edits.

## Current authority

The operator stopped Run 115 and ordered a whole-machine authority rebuild.
The adopted target is `docs/architecture/ghostlight-dungeon-mvp.md`.

One per-world `WorldKernel` owns one revisioned `WorldState` across Draft,
Active, and Archived phases. It alone derives authority, applies the typed
ontology reducer, and commits an atomic CultCache revision. Session Zero,
autonomous turns, player commands, imports, and administrative changes must
enter through the same `CommandEnvelope` path.

Autonomous cognition has two explicit modes under disjoint controller scopes:

- `NarrativePersona` receives narrative projection and replies only in prose.
  A total Interpreter preserves that prose, emits every faithful typed proposal
  it can lower, and records exact translation gaps for the rest.
- `OperationalAgent` receives a permissioned typed view and tools when
  operator-shaped cognition benefits from direct state work.
- Mode changes representation, never authority. A decision opportunity cannot
  fall through between modes or act twice.

The Interpreter cannot fail semantically. Missing referents, ambiguous intent,
missing affordances, and missing command primitives become translation gaps.
Step exhaustion finalizes accumulated speech, proposals, and gaps. Transport
faults and stale commits remain infrastructure outcomes outside interpretation.
Only the world kernel may accept or reject a proposed mutation.

World validity is sparse and causal. Actor counts, coverage ratios,
interestingness, political diversity, name quality, and prose quality are
load or evaluation evidence; none admits ontology or declares a world complete.

## Landed and in flight

- `4aebe17` adopts the target authority map.
- `64f50de` removes seven obsolete pipeline-smoke binaries and two live-fire
  orchestration scripts.
- `a144c33` adds the two cognition modes and total interpretation contract in
  `ghostlight-persona-projection`.
- The replacement `WorldState` and `CommandEnvelope` foundation is mid-surgery.
  It is not current authority until committed and verified.
- Legacy `WorldKernel`, `SessionZeroKernel`, aggregate Campaign/component state,
  alternate ingresses, semantic verifier/reconciliation stages, model-owned
  scheduling, and checkpoint recovery remain deletion targets.

Do not deploy the rebuild branch or start another acceptance run while the
replacement aggregate lacks its negative, restart, privacy, and sovereignty
proofs.

## Immutable failure evidence

Run 115 is terminal failed at
`/var/lib/gamecult/ghostlight-dungeon/acceptance/full-world-delvehold-0a83034-115`:
semantic revision 2, regions 2/8, waves 0/1. Resume2 ended on provider SSE
timeout. Resume3 exhausted four reconciliation steps after
`inst:kharad-road-keepers` referenced unknown `loc:kharad-rhythm-road`.
Invocation `d9254cd9b77946aebcf7a7fdae821402` is terminal. Preserve the root and
receipts unchanged; its `status.json` is stale derived telemetry.

Runs 108 through 115 are evidence about load and failure properties of the old
body. The 1,200 and 2,400 actor profiles are synthetic fixtures only.

## Operational boundary

The deployed Yggdrasil service remains the pre-rebuild production body. Exact
release, service, discovery, and authentication witnesses belong to
`F:\Projects\gamecult-ops`; the frozen pre-rebuild map records the last
Ghostlight-side evidence. Do not infer deployed identity from workspace HEAD.

Heimdall owns account identity. Eve owns command invocation and lowering.
VoidBot owns Vault retrieval and evidence. Idunn owns deployment and daemon
continuity. Odin owns discovery. None owns world mutation. External consumers,
including Delvehold and Epiphany, own their state; Ghostlight may publish views
and proposals but cannot commit on their behalf.

## Next gate

Finish and verify the typed world ontology, then build the private aggregate
journal with one writer, compare-and-swap revision, digest chaining,
idempotency, and restart recovery. Cut obsolete writers before migrating
Session Zero, turns, resolution, elaboration, persistence, and projection onto
that path. No live acceptance run precedes the focused contract suite.

## Essential references

- Target authority: `docs/architecture/ghostlight-dungeon-mvp.md`
- Frozen teardown map: `notes/ghostlight-current-system-map.md`
- Current implementation plan: `notes/ghostlight-implementation-plan.md`
- Human-readable state: `state/map.yaml`
- Distilled evidence: `state/evidence.jsonl`
- Machine-managed state: `state/ghostlight-state.cultcache.jsonl`
- Interface authority: `docs/architecture/ghostlight-eve-native-interface.md`
- Gestalt authority: `docs/architecture/ghostlight-multiresolution-agency.md`
- Operations: `F:\Projects\gamecult-ops`

## Re-entry warnings

- A prompt, transcript, model receipt, scheduler item, browser surface, actor
  count, or derived simulation cover is not canonical world state.
- Do not preserve an old writer through a compatibility path.
- Do not let a translation gap become invented state or an Interpreter error.
- Do not resume Run 115.
- Keep this handoff compact. Move chronology to Git, evidence, or the frozen
  system map.
