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

The replacement owner is sealed. Its runtime boundary remains create/open,
immutable snapshot, submit, and typed receipts. Mutable aggregate state,
canonical ID issuance, the reducer, and the CultCache journal writer stay
private to the kernel. Controller assignment and affordance grants are
canonical; current executability and exact revision-bound decision
opportunities are derived and revalidated by the kernel.

Autonomous cognition has two explicit modes under disjoint controller scopes:

- `NarrativePersona` receives narrative projection and replies only in prose.
  The runner records that prose as receipt-bound noncanonical source evidence;
  a total Interpreter emits every faithful typed proposal it can lower with an
  exact source capture and records exact translation gaps for the rest.
- `OperationalAgent` receives a permissioned typed view and tools when
  operator-shaped cognition benefits from direct state work.
- Mode changes representation, never authority. A decision opportunity cannot
  fall through between modes or act twice.

The Interpreter cannot fail semantically. Missing referents, ambiguous intent,
missing affordances, and missing command primitives become translation gaps.
Malformed capture spans, raw tool decode failure, and normal step exhaustion
fall back to an exact whole-source gap rather than losing meaning. An
infrastructure interruption discards partial captures and returns the immutable
source pending for a fresh attempt. Source prose is not speech: only actually
spoken words may become a typed speech proposal. Only the world kernel may
accept or reject a proposed mutation. Translation gaps remain non-fictional
inference/evaluation evidence outside `WorldState`; recording one cannot itself
request elaboration or advance a world revision.

World validity is sparse and causal. Actor counts, coverage ratios,
interestingness, political diversity, name quality, and prose quality are
load or evaluation evidence; none admits ontology or declares a world complete.

## Landed and in flight

- `4aebe17` adopts the target authority map.
- `64f50de` removes seven obsolete pipeline-smoke binaries and two live-fire
  orchestration scripts.
- `a144c33` establishes the two disjoint cognition modes in
  `ghostlight-persona-projection`.
- `d49b907` refines total Persona interpretation: source prose is receipt-bound
  noncanonical evidence, typed captures cite exact source spans, malformed spans,
  raw decode failure, and normal exhaustion receive whole-source gap fallback,
  infrastructure interruption returns the source pending, and only actual
  spoken words qualify for a speech proposal. This contract is not wired into
  the live legacy Persona runners; they still terminalize and remain deletion
  targets.
- An uncommitted broad `world.rs` prototype was rejected and deleted. It
  exposed mutable state and ID issuance, admitted translation-gap evidence into
  canonical types, allowed no-op commits, and left affordance and opportunity
  authority undefined. None of it is current authority.
- `12ee9f4` deletes the dead legacy Elaboration and Mutation kernel ingresses
  and their component-write helpers: 686 lines removed without a replacement
  compatibility path.
- `66ed2ec` completes the sealed private `foundation.v0` authority loop. Draft
  creation, approval, activation, canonical controller assignments, affordance
  grants, revision-bound opportunities, and one shared human/autonomous action
  reducer all commit through one aggregate and journal. One mailbox serializes
  submission and snapshot access. Eighteen focused tests prove lifecycle,
  controller and opportunity fail-closed behavior, restart, idempotency,
  one-owner, authentication, non-commit, mailbox cancellation, lost replies,
  immutable genesis, and journal forgery rejection.
- The replacement remains crate-private. The legacy exported
  `kernel::WorldKernel`, executable startup, Session Zero, player turn,
  autonomous scheduling, Campaign/component, verifier/reconciliation, and
  checkpoint paths remain live deletion targets. The next move exposes and
  wires only the replacement runtime facade and mailbox, then cuts those owners
  without dual-write.

Do not deploy the rebuild branch or start another acceptance run while the
legacy executable still owns startup, Draft, player, autonomous, or recovery
mutation paths.

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

Expose the sealed replacement as the crate and executable runtime identity,
with `WorldMailbox` as the only live world command boundary. Bind authenticated
runtime identity to its sealed caller evidence, make startup create or open one
owner and spawn one mailbox, then route Draft creation, approval, activation,
player decisions, and autonomous controller proposals through the existing
command path.

Cut each old startup, Session Zero, player, autonomous, Campaign/component, and
recovery writer before enabling its replacement path. Do not dual-write, mirror
canonical state, or add a compatibility router. Runtime restart and negative
checks must prove the old paths cannot mutate or repair the replacement owner
before any live acceptance run.

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
- Do not publish mutable world types, an ID issuer, a reducer entry point, or a
  generic journal handle for the convenience of tests or adapters.
- Do not append a commit when reduction produces no canonical mutation.
- Do not resume Run 115.
- Keep this handoff compact. Move chronology to Git, evidence, or the frozen
  system map.
