# Ghostlight Fresh Workspace Handoff

Updated: 2026-09-01

This is the compact re-entry packet. It carries current authority and the next
gate. Git owns chronology, the system map owns teardown detail, and evidence
records own durable findings.

## Immediate Re-entry Instruction

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

Do not continue implementation automatically from a rehydrate-only request.
Do not trust this file for the exact live HEAD. Git and live runtime witnesses
own volatile identity.

## Current authority

The operator stopped Run 115 and ordered a whole-machine authority rebuild.
The adopted target is `docs/architecture/ghostlight-dungeon-mvp.md`.

One per-world `WorldKernel` owns one revisioned `WorldState` across Draft and
Active phases. It alone derives authority, applies the typed
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
- `6bb6869` makes the replacement mailbox/kernel architecture the crate and
  executable runtime identity and removes the legacy runtime owner.
- `13d5136` unifies app sessions, world journal, and controller custody under
  one vendored CultCache implementation and removes the duplicate legacy
  dependency. The committed daemon tree contains no old Session Zero, legacy
  kernel, scheduler, assessor, verifier/reconciliation, or legacy-transition
  module path.
- Source subtraction is complete. Production cutover and runtime legacy purge
  are not.

Do not start another world acceptance run while Yggdrasil still serves the
legacy executable and state layout.

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

Yggdrasil still serves legacy Ghostlight release `a4080d4` from an enabled
`Restart=always` unit with health v1, campaign/Session Zero state, and no
`world.cc` or `app-sessions-v2.cc` witness. The Connector is also still on its
legacy enabled body. Idunn r21 is live, but its current deploy seam is not safe
to invoke: target policy is repeated across compiled Rust, raw shell command
records, a privileged dispatcher, and large gamecult-ops target programs. Its
root-side source-floor validator cannot inspect correctly Idunn-owned Git
mirrors, and the current fixed-port scripts stop the incumbent before a fresh
candidate is green.

The adopted cut puts a visible recipe and unit template in each target repo.
Idunn operator bindings own the admitted ref, runner/container image and
affordances, host paths, secrets, routing endpoint, rollout, retention, and
desired replica placement. Idunn freezes and materializes source as its own UID,
executes the configured build/test recipe, seals the candidate, waits for signed
staged readiness on a private endpoint, then alone revokes the incumbent
process/write grant, admits the candidate, moves the stable route, and drains
the old generation. Deployment and continuity remain separate authorities; a
deployment brake cannot suspend restart of the already-admitted body.

Heimdall owns account identity. Eve owns command invocation and lowering.
VoidBot owns Vault retrieval and evidence. Idunn owns deployment and daemon
continuity. Odin owns discovery. None owns world mutation. External consumers,
including Delvehold and Epiphany, own their state; Ghostlight may publish views
and proposals but cannot commit on their behalf.

## Next gate

Rebuild the Idunn deployment seam before releasing another target brake. Define
one typed target recipe plus operator binding, remove raw executable command
authority and root Git inspection, and move Ghostlight and Connector build,
test, package, unit, health, and state semantics into visible target-owned
recipes. Keep fleet affordances and exact branch admission in Idunn-owned
configuration. Introduce a stable router data plane whose membership only
Idunn can change; candidate health must be green before the process/write lease
and route move.

Deploy the Connector and then Ghostlight through that contract. Runtime restart,
route continuity, signed health, exact receipts, exclusive world-v2 state, and
negative checks must agree before deleting old services, releases, state,
acceptance roots, and local run scaffolding.

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
- Do not invoke the current bounded Connector or Ghostlight redeploy helpers;
  they fail at the root/Idunn Git boundary and still embody duplicate target
  deployment authority.
- Do not prove world-v2 purity with a short deny list. Archive the complete old
  state root or validate a complete allowed-path contract.
- Keep this handoff compact. Move chronology to Git, evidence, or the frozen
  system map.
