# DELVE/HOLD Consumer Profile for the Ghostlight World API

## Status

This document is a consumer binding and conformance profile. It records how
Delvehold maps its authored world and owned state onto Ghostlight's generic API;
it does not create a Delvehold organ, schema family, subject ID, or authority
branch inside Ghostlight core.

## Objective

Use Ghostlight's live external-world simulation as an ordinary API consumer
without making it the owner of DELVE/HOLD players, workshops, dungeons, civic
state, or quantitative economy. DELVE/HOLD provides an authored ontology and
fixed seed for one canonical outside world through consumer-neutral Ghostlight
operations. This bypasses Session Zero world generation, not `WorldKernel`,
typed validation, the mutation algebra, or atomic CultCache commits.

## Core and consumer cut

Ghostlight admits generic world seeds, externally owned subject projections,
consumer-supplied domain effect schemas, proposals, and receipts. Delvehold's
adapter owns the names `delvehold.greathold_boundary_state.v0`,
`delvehold.greathold_effect_batch.v0`, `ghostlight.external_response_batch.v0`,
`delvehold.boundary_receipt.v0`, and the configured subject ID `greathold`. It
lowers and raises those documents at the API boundary. Ghostlight never
switches on those names or that ID.

## Authority map

- **Delvehold owner:** Greathold players, workshops, parties, civic seals, graph, dungeon ecology, and its own quantitative economy: recipes, facilities, capacity, inventories, orders, prices, exchange rates, contracts, expeditions, and local consequences. Custody of Greathold-owned resources changes only through Delvehold.
- **Ghostlight owner:** external regions, actors, institutions, Gestalts, their knowledge, relationships, goals, posture, pressures, strategic choices, external events, and news.
- **Configured boundary subject:** Delvehold configures stable ID `greathold`
  through Ghostlight's generic external-subject authority contract. The ID has
  no privileged meaning in Ghostlight core.
- **Ghostlight API:** consumer-neutral CultMesh operations admit authored seeds,
  externally controlled subject observations and realized effects, and return
  attributed intents, news, state projections, and receipts. They own no
  consumer ontology or decision.
- **Delvehold adapter:** Delvehold owns translation between its detailed
  Greathold state and the generic Ghostlight API. The adapter owns no truth and
  cannot repair either canonical domain after rejection.
- **Forbidden writers:** Ghostlight Personas and strategic cells cannot mutate `greathold`; Delvehold cannot mutate foreign private state; the adapter cannot repair either owner after rejection.

Ghostlight owns a narrative-scale conserved quantity for the subjects it simulates: `Custody` holds a unitless `Quantity` per subject and resource, `Transfer`/`Consume`/`Transform` conserve it, and `Admit` creates it only with evidence (`docs/architecture/ghostlight-world-ontology.md`, Invariant 5). That is a ledger, not an economy. `Transform` is one-to-one with no unit, rate, yield, facility, or capacity field anywhere on a Ghostlight type, so prices, production, orders, and contracts are unrepresentable in Ghostlight rather than merely unwritten; they remain Delvehold-owned, and any resource an external mirror holds changes only through its admitted owner.

## Greathold boundary subject

`greathold` is externally controlled. It is not a person, sovereign government, averaged population will, or alternate player-state store.

- It receives no Persona turn or autonomous institution effect.
- Only generic API operations lowered by the Delvehold adapter from an accepted
  `delvehold.greathold_boundary_state.v0` or
  `delvehold.greathold_effect_batch.v0` document may update its projection.
- External subjects may observe it, relate to it, and direct attributed actions toward it.
- Any proposed change to Greathold-owned truth exits as `ghostlight.external_response_batch.v0` and remains pending until Delvehold returns `delvehold.boundary_receipt.v0`.

The projection excludes player identities, private workshop state, pending votes, unaccepted contracts, private communications, and inferred collective desire. It reports committed macro effects and authorized institutional acts.

## Consumer flow

```text
Delvehold commit
  -> Delvehold-owned boundary projection or realized-effect batch
  -> Delvehold adapter lowers to a generic Ghostlight API operation
  -> WorldKernel validation and atomic commit
  -> strategic resolution over external subjects
  -> generic attributed intent/news/state response
  -> Delvehold adapter raises to its consumer contract
  -> Delvehold validation and atomic commit
  -> admission receipt
```

Each document carries world ID, UTC epoch ID, source revision, effective time, causal refs, provenance, payload digest, and idempotency key. `delvehold.boundary_receipt.v0` is direction-neutral: the receiving domain returns it to the sender with sender and recipient IDs, per-item results, reason codes, resulting local revision, and committed event refs. Revisions remain local. The source may retry; the recipient must return the same receipt for the same key and digest.

Malformed envelopes, stale revisions, digest conflicts, and invalid causal structure reject the whole document without mutation. After envelope validation, semantically independent items may resolve to an explicit accepted subset, but that subset commits atomically or not at all. The receipt records every item result.

`greathold_boundary_state.v0` is an absolute replacement of its declared projection fields at a Delvehold source-revision watermark. `greathold_effect_batch.v0` carries deltas over explicit preceding and resulting revisions. A snapshot supersedes deltas at or below its watermark; delayed deltas below either accepted watermark are stale. Greathold ingress access and capacity are Delvehold-owned. External route topology and status remain Ghostlight-owned even when Delvehold mirrors their IDs in local admitted state.

Major committed events cross promptly. Ordinary material flow closes hourly. Recovery replays at most eight missed epochs and represents older time as one explicitly coarse horizon. A summarized horizon may reduce detail; it cannot invent player action or hide an unprocessed interval.

## Required generic Ghostlight API capabilities

Ghostlight needs:

1. one seed admission primitive (`AdmitPatch` in Draft) shared by the seed
   producer and consumer seed authors;
2. generic external-subject ownership metadata enforced by selection and
   mutation gates;
3. generic consumer effect-schema registration or supply;
4. generic proposal and idempotent receipt envelopes.

All four delegate to existing WorldKernel validation and atomic persistence.
The adapter lives in Delvehold and owns all Delvehold-specific projection,
economy translation, document names, and configured IDs. No Delvehold-specific
economy, civic, dungeon, or contract schema belongs in Ghostlight core. No
second kernel, economy store, Session Zero compatibility fiction, or direct
campaign-row writer is admitted.

Acceptance requires negative proof that ordinary strategic waves cannot select `greathold` as an acting subject, malformed or stale batches commit nothing, and foreign effects become local consequences only after a Delvehold receipt.
