# DELVE/HOLD Forced-Ontology Integration

## Status

This document is the admitted consumer contract for the DELVE/HOLD persistent-world experiment. It specifies ownership and document flow. Runtime schemas and adapter code are not implemented by this documentation pass.

## Objective

Use Ghostlight's live external-world simulation without making it the owner of DELVE/HOLD players, workshops, dungeons, civic state, or quantitative economy. DELVE/HOLD provides an authored ontology and fixed seed for one canonical outside world. This bypasses Session Zero world generation, not `WorldKernel`, typed validation, the mutation algebra, or atomic CultCache commits.

## Authority map

- **Delvehold owner:** Greathold players, workshops, parties, civic seals, graph, dungeon ecology, quantitative resources, custody, recipes, facilities, capacity, inventories, orders, prices, contracts, expeditions, and local consequences.
- **Ghostlight owner:** external regions, actors, institutions, Gestalts, their knowledge, relationships, goals, posture, pressures, strategic choices, external events, and news.
- **Derived boundary target:** one Ghostlight institution with stable ID `greathold`, rebuilt from accepted Delvehold boundary documents once the external-control seam is implemented.
- **Transport:** CultMesh carries state, effects, intents, and receipts. It owns no decision.
- **Forbidden writers:** Ghostlight Personas and strategic cells cannot mutate `greathold`; Delvehold cannot mutate foreign private state; the adapter cannot repair either owner after rejection.

Ghostlight's current string resource handles are sufficient for strategic narrative context. They are not a quantitative economy. Prices, quantities, production, capacity, orders, contracts, and conservation remain Delvehold-owned unless a future deliberate Ghostlight economic algebra is designed and admitted.

## Greathold boundary subject

`greathold` is externally controlled. It is not a person, sovereign government, averaged population will, or alternate player-state store.

- It receives no Persona turn or autonomous institution effect.
- Only an accepted `delvehold.greathold_boundary_state.v0` or `delvehold.greathold_effect_batch.v0` command may update its projected condition after the external-control policy exists.
- External subjects may observe it, relate to it, and direct attributed actions toward it.
- Any proposed change to Greathold-owned truth exits as `ghostlight.external_response_batch.v0` and remains pending until Delvehold returns `delvehold.boundary_receipt.v0`.

The projection excludes player identities, private workshop state, pending votes, unaccepted contracts, private communications, and inferred collective desire. It reports committed macro effects and authorized institutional acts.

## Adapter flow

```text
Delvehold commit
  -> boundary projection or realized-effect batch
  -> Ghostlight typed command
  -> WorldKernel validation and atomic commit
  -> strategic resolution over external subjects
  -> external response batch
  -> Delvehold validation and atomic commit
  -> admission receipt
```

Each document carries world ID, UTC epoch ID, source revision, effective time, causal refs, provenance, payload digest, and idempotency key. `delvehold.boundary_receipt.v0` is direction-neutral: the receiving domain returns it to the sender with sender and recipient IDs, per-item results, reason codes, resulting local revision, and committed event refs. Revisions remain local. The source may retry; the recipient must return the same receipt for the same key and digest.

Malformed envelopes, stale revisions, digest conflicts, and invalid causal structure reject the whole document without mutation. After envelope validation, semantically independent items may resolve to an explicit accepted subset, but that subset commits atomically or not at all. The receipt records every item result.

`greathold_boundary_state.v0` is an absolute replacement of its declared projection fields at a Delvehold source-revision watermark. `greathold_effect_batch.v0` carries deltas over explicit preceding and resulting revisions. A snapshot supersedes deltas at or below its watermark; delayed deltas below either accepted watermark are stale. Greathold ingress access and capacity are Delvehold-owned. External route topology and status remain Ghostlight-owned even when Delvehold mirrors their IDs in local admitted state.

Major committed events cross promptly. Ordinary material flow closes hourly. Recovery replays at most eight missed epochs and represents older time as one explicitly coarse horizon. A summarized horizon may reduce detail; it cannot invent player action or hide an unprocessed interval.

## Required Ghostlight implementation seam

The eventual adapter needs an authored-seed publication path and an external-control admission policy for one institution. Both must delegate to existing WorldKernel command validation and atomic persistence. No second kernel, economy store, Session Zero compatibility fiction, or direct campaign-row writer is admitted.

Acceptance requires negative proof that ordinary strategic waves cannot select `greathold` as an acting subject, malformed or stale batches commit nothing, and foreign effects become local consequences only after a Delvehold receipt.
