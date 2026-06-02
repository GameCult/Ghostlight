# Ghostlight Verse Service Contract

Ghostlight is a state/projection service candidate for socially persistent
characters and simulated people. Its durable truth is not a dashboard and not a
prompt. Its durable truth is typed character, scene, relationship, event,
perception, and Persona projection state.

Ghostlight already has a CultCache-backed persistence seam. The next GameCult
Verse cut is to expose its inspection and authoring surfaces as Eve GUI/TUI DSL
so local runtimes can browse state, projection, and review artifacts without
turning Ghostlight into a private web app.

## Owner Map

- Owner: Ghostlight owns canonical scene/world/agent state, observer-specific
  perceived state, projection inputs, reviewed mutation receipts, training-loop
  bundles, and Persona projections derived from story agents.
- Inputs: Aetheria lore/source context, canonical agent and scene state,
  projected local context, responder packets, sandboxed responder output,
  reviewer receipts, participant appraisals, and accepted mutation receipts.
- Outputs: CultCache-backed state records, readable migration exports, reviewed
  training artifacts, schema fixtures, event records, participant appraisals,
  and portable `gamecult.persona_state.v0` projections when a public
  person-shaped surface is needed.
- Derived state: `state/branches.json`, `state/evidence.jsonl`, validator
  reports, fixture sidecars, and authoring previews are compatibility or review
  surfaces, not long-term direct-write APIs.
- Forbidden writers: authoring dashboards, visual replay applets, prompt
  renderers, review helpers, and Eve/TUI renderers must not directly mutate
  canonical character truth. They emit reviewed mutation intent or read typed
  projections.
- Shared paths: state CLI, fixture validators, training-loop review, future
  authoring UI, future Eve surface, and future compact TUI must read the same
  CultCache-backed state seam instead of maintaining separate dashboard truth.
- Deletion line: any direct-write JSON state path that survives after a typed
  document exists must be demoted to export/witness or removed.

## Current State

Ghostlight already has these substrate pieces:

- `state/ghostlight-state.cultcache.jsonl` as the current CultCache-backed
  migration spine;
- `vendor/cultcache-py`;
- `tools/ghostlight_state_store.py` as the state access seam;
- `schemas/gamecult.persona_state.v0.schema.json` as the mirrored portable
  Persona contract;
- `schemas/agent-state.schema.json` for Ghostlight-native scene/world/story
  state;
- architecture docs for persistence, projection, training loops, prompt
  projection, and shared Persona/CultNet contracts.

The missing surface is not another readable export. The missing surface is a
provider-owned Eve GUI/TUI composition over the existing state and review seams.

## Eve Surface Target

Ghostlight should publish an Eve GUI/TUI DSL surface with these panels:

1. `State Spine`: CultCache store path, document families, migration status,
   readable export freshness, and direct-write debt.
2. `Scene And Agent State`: active scene fixtures, canonical agent state,
   observer/perceived overlays, and relationship stance availability.
3. `Projection Pipeline`: projected local context, responder packet, responder
   output, review status, leakage audit, and mutation receipt state.
4. `Training Coverage`: accepted/draft/planned/rejected fixture counts,
   coverage gaps, tonal modes, collision axes, and training target families.
5. `Persona Projection`: `gamecult.persona_state.v0` availability for public
   character/person surfaces, with provenance back to Ghostlight-native state.

Each panel must expose source provenance and freshness. If a panel is reading a
readable JSON export during migration, the surface should say that the export is
not the canonical owner.

## Migration Order

1. Promote remaining direct-write state paths behind `tools/ghostlight_state_store.py`
   or its successor typed document seam.
2. Define a `ghostlight.eve_surface.v0` provider contract over the current
   state, projection, review, and Persona documents.
3. Add a read-only command that lowers those documents into Eve DSL.
4. Publish the surface through CultMesh/Odin discovery.
5. Let browser/native/TUI runtimes lower the same surface.
6. Demote old preview/status dashboards to renderers of the provider-owned Eve
   surface.

The invariant: Ghostlight owns story/person state. Eve makes it inspectable.
Authoring surfaces request reviewed mutations; they do not become state owners.
