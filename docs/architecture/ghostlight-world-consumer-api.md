# Ghostlight World Consumer API

## Objective

Let a consumer supply and observe one persistent Ghostlight world without
becoming part of Ghostlight's runtime. Session Zero compilation and external
world authoring are two producers of the same admitted seed. Delvehold is the
first demanding consumer; no Delvehold identifier, schema, or policy is core
authority.

## Authority map

- **Seed admission owner:** `CampaignRegistry` validates one `WorldSeed` and
  commits it with one `WorldSeedAdmission` and receipt. Producer-specific
  documents may join that transaction but cannot create another publication
  path.
- **World owner:** `WorldKernel` owns every admitted campaign revision and all
  simulation state.
- **External-subject owner:** `ExternalSubjectAuthority` binds one exact
  consumer owner to one exact subject. The subject remains a visible target but
  receives no Ghostlight Persona turn and cannot act through a strategic cell.
- **Consumer owner:** the consumer decides whether a Ghostlight proposal changes
  its own domain. Its acknowledgement records admission or refusal but cannot
  rewrite Ghostlight state.
- **Inputs:** a consumer-neutral seed, producer and payload digests, exact
  authority descriptors, normalized external-institution snapshots, and
  proposal acknowledgements.
- **Outputs:** seed-admission receipts, revisioned snapshot receipts, attributed
  external proposals, and immutable proposal acknowledgements.
- **Derived state:** Session Zero publication documents, consumer bindings,
  network messages, schema catalogs, and UI projections own no canonical world
  state.
- **Forbidden writers:** seed producers, adapters, models, arenas, transport,
  and acknowledgements cannot write a campaign row directly.
- **Shared paths:** compiler and consumer seeds lower to one registry/store
  transaction. Strategic waves continue through the existing WorldKernel
  command and atomic commit path.
- **Cut line:** `publish_session_zero` no longer owns campaign creation. It is a
  producer-specific wrapper around generic seed admission. Ghostlight never
  switches on `greathold` or a `delvehold.*` schema.

## Typed flow

```text
producer seed
  -> WorldSeed + WorldSeedAdmission
  -> CampaignRegistry validation
  -> one empty-store CultCache transaction
  -> WorldKernel runtime

consumer-owned state
  -> consumer adapter
  -> ExternalSubjectSnapshot (institution or Gestalt projection)
  -> authority, digest, revision, and idempotency validation
  -> WorldKernel atomic campaign commit
  -> ExternalSnapshotReceipt

strategic action directed at an external subject
  -> WorldKernel strategic-wave validation
  -> attributed ExternalWorldProposal persisted with the wave
  -> consumer reads proposal and decides locally
  -> ExternalProposalReceipt records accept, partial accept, or reject
  -> any realized consumer change returns later as a fresh snapshot
```

## Transport

The API uses typed CultNet operation requests over Ghostlight's existing RUDP
server. Seed admission, external snapshots, proposal reads, and proposal
acknowledgements use MessagePack payloads and typed receipts. The service is
loopback-only until CultMesh authority leases and secure remote admission are
implemented. HTTP and Eve remain player/product projections, not the world
consumer boundary.

## Invariants

- The public seed shape excludes runtime revision, transcript, pending work,
  cover state, and other already-authoritative campaign fields.
- The seed digest binds the exact normalized seed. A campaign ID cannot be
  republished from another digest.
- An external authority names one existing institution or Gestalt subject and
  one owner. Its secret is persisted only as SHA-256.
- External subjects are active graph targets but never simulation-eligible.
- A snapshot replaces only the exact externally owned institution or Gestalt projection.
  It cannot create subjects, routes, actors, Gestalts, relations, or private
  knowledge.
- Source revisions increase monotonically. Exact idempotent replay returns the
  original receipt; conflicting replay changes nothing.
- Outbound proposals remain attributed to an exact Ghostlight subject and exact
  target. An arena never becomes the proposer.
- Proposal acknowledgement is not a Ghostlight world mutation. Realized effects
  return through a later snapshot.
- Dungeon campaigns keep their player actor, membership, contract, and Session
  Zero documents. Their adversarial play path does not depend on a consumer.

## First acceptance

1. Session Zero and a consumer-authored fixture persist the same generic seed
   admission and receipt types.
2. A configured external institution never receives a strategic cell or
   Persona action.
3. Wrong owner, wrong secret, stale source revision, digest conflict, and stale
   world revision leave campaign state unchanged.
4. A valid snapshot advances the campaign once and returns an idempotent typed
   receipt.
5. A strategic action aimed at the external institution emits one attributed
   proposal in the same atomic wave commit.
6. Consumer acknowledgement cannot mutate the campaign and can be replayed
   idempotently.
