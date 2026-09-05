# Ghostlight World Consumer API

Status: this describes the landed pass-10 consumer ingress — a world's third
patch author, reaching `AdmitPatch` through one loopback-bound typed document.

## Objective

Let an external consumer author and observe the exact subjects a Ghostlight
world has admitted as its mirrors, without becoming part of Ghostlight's
runtime and without the world learning the consumer's vocabulary. A
consumer-authored seed and a compiled seed are the same `AdmitPatch`; there is
no separate seed type, registry, or publication handoff. Delvehold is the
first consumer; no Delvehold identifier, schema, or policy is core authority
(`docs/architecture/delvehold-forced-ontology-integration.md` names the
profile-side binding).

## Authority map

- **Admission owner:** `require_patch_author` (`world/mod.rs`) decides which
  caller may submit `AdmitPatch` and returns that caller's `PatchGround`;
  `confine_to_ground` decides what a confined ground may write. Both run in
  `reduce`'s `AdmitPatch` arm and again in `apply_effect`'s `PatchAdmitted`
  arm — there is no second confinement function and no way for a patch to
  commit without being re-decided.
- **Consumer ground:** `PatchGround::Consumer(ConsumerId)` is derived, never
  carried. The set of subjects a consumer may write is
  `{ s | controller_assignments[s] == ExternallyControlled { consumer } }`,
  read from committed state at decision time. A consumer cannot name its own
  scope on the wire, and revoking a binding in a later commit shrinks its
  authority with no ingress change.
- **Caller minting:** `WorldMailbox::submit_consumer` (`world/mailbox.rs`) is
  the only constructor of `AuthenticatedCaller::verified_system(SystemCapability::Consumer { consumer })`.
- **Ingress owner:** `world/consumer.rs` owns decoding a document, bounding
  it, authenticating the consumer, one port call, and one receipt
  projection. It owns no world truth, holds no opinion about a patch's
  content, and does not pre-validate — a second reducer here could disagree
  with the kernel's, so structure is decided once, by `resolve_patch`.
- **Port:** `ConsumerPort` (`world/mailbox.rs`) has one method,
  `submit_consumer`. It cannot read the world: it does not select an answer,
  does not pre-validate, and returns a receipt rather than state.

## Typed flow

```text
ConsumerPatchDocument (canonical MessagePack)
  -> ConsumerRegistry: name -> ConsumerId, secret verified against a
     stored SHA-256 digest in constant time
  -> patch::decode_patch, bounded by MAX_PATCH_BYTES / _DECLARATIONS /
     _OPERATIONS / _EVIDENCE
  -> WorldMailbox::submit_consumer(world_id, expected_revision, command_id,
     consumer, answers, patch)
  -> reduce / apply_effect: require_patch_author, the answer rule,
     confine_to_ground
  -> ConsumerReceiptDocument: one atomic verdict — a commit reference or
     the complete mismatch set
```

`command_id` and `world_id` on the receipt are `Option`: a frame that fails to
decode or names an unregistered consumer never derives a command key or reads
a world, so the receipt has none to report. The outbound half of this
contract — a consumer reading its own projection, or receiving an
attributed proposal — is not part of this pass; see "Not in this pass" below.

## Transport

`POST /cultnet/world-patch`, declared beside `/cultnet/snapshot` in
`api_router` (`runtime.rs`). The handler gates loopback and content type
(`application/msgpack`) exactly as the snapshot route does, then hands the
body to `world::consumer::admit_document`; everything past those two gates is
`world/consumer.rs`'s to decide. The body limit is
`CONSUMER_BODY_LIMIT = patch::MAX_PATCH_BYTES + CONSUMER_ENVELOPE_SLACK` (an
8 KiB envelope allowance over the 256 KiB patch cap), so the transport guard
cannot become a second opinion about how big a patch may be.

The document and receipt are Ghostlight-owned typed records in canonical
MessagePack, not CultNet control messages, pinned to two schema constants:

```text
CONSUMER_PATCH_SCHEMA   = "ghostlight.consumer_patch.v0"
CONSUMER_RECEIPT_SCHEMA = "ghostlight.consumer_receipt.v0"
```

`ConsumerPatchDocument` carries `schema`, `world_id`, `consumer` (a
configured name, lowered to a `ConsumerId` by the registry and never carried
further), `secret`, `idempotency_key`, `expected_revision`, `answers:
Option<PatchAnswer>`, and `patch: Vec<u8>` — the encoded `WorldPatch` as a
nested canonical-MessagePack byte frame rather than an inline value, so
`MAX_PATCH_BYTES` is checked against the patch itself before any of its items
deserializes. The outer document is decoded first, is tiny, and is bounded by
the route's derived limit.

`mesh.rs`'s advertisement gains the two schema constants and nothing else:
this door is loopback-bound, so it is not a remotely reachable surface and
advertising it as one would claim availability the binary does not have.

## Idempotency and staleness

`expected_revision` is required; the port submits through
`submit_authenticated`, not the stamped path, because a caller that cannot
name the revision it built against cannot be told its batch is stale. The
idempotency ledger is checked before the revision, so an exact resubmission
still returns the original receipt (`AlreadyApplied`) even after the world has
moved; a stale document is `RevisionMismatch { expected, actual }`, and the
receipt carries the live revision so the consumer knows what to build against
next. `CommandId::derived` computes the command key from `(world, consumer,
idempotency_key)`; the same key with a different body is
`CommandIdConflict`, and nothing commits.

There is no receipt-probe request and no separate mailbox `Request` variant
for it. A consumer's document is byte-identical on retry and its `CommandId`
is derived, so resubmission is the probe and the ledger answers it directly.

## The externally controlled subject

A subject declared `NewController::External { consumer }` receives
`ControllerAssignment::ExternallyControlled { consumer }`. It mints no
controller ID and no controller mode — both accessors return `Option` for it,
and every reader fails closed rather than assuming a default. Concretely:

- it derives no decision opportunity and mints no turn;
- it is excluded from `agency_graph` and so from the cover;
- it is visible in the snapshot as an ordinary subject, related to, targeted,
  and counted like any other, with `controller_id: None` and
  `controller_mode: None`;
- it may hold no affordance — declaring it with a non-empty affordance grant
  set is `Mismatch::ControllerGrantMismatch`, and the pairing is checked both
  ways (an ordinary subject with an empty grant set is the same mismatch);
- its components change through exactly two callers: its own consumer, and
  the world owner, who is unconfined and always could write it — that is the
  owner's standing authority, not a leak in the consumer boundary.

## Confinement

A consumer writes only the subjects bound to it and operations grounded on
those subjects (`confine_to_ground`, the `PatchGround::Consumer` arm):

- a declared subject is admitted only if its controller names the same
  consumer; position is unconfined;
- a declared place, a declared route, `Relocate`, `OpenRoute`, `CloseRoute`,
  and `AlterCost` are always refused — they carry a place or a route, and a
  consumer's ground names no place;
- a `Claimed { by }` fact must name a bound subject; a `Canonical` fact
  declaration is refused outright — a consumer's evidence buys an `Admit`
  into its own custody and a `Claimed` fact, never a canonization;
- a channel's reach and controller must all be bound subjects;
- every operation's ground subjects must be bound, and its place and route
  lists must be empty — `Transfer` therefore requires both endpoints bound,
  so a consumer cannot push custody into a Ghostlight subject; `Admit` and
  `Consume` work inside its own custody.

In Active phase a consumer patch that declares answers nothing except a
`CausalBoundary::MissingStructure { subject }` derived on one of its own
mirrors — the world noticing that a subject it can see has structure it
cannot account for. Every other Active declaration is refused by the answer
rule plus this confinement, not by a separate phase gate: a consumer cannot
answer a boundary on a foreign subject (not covered), cannot answer a
`Deficit` row (deficits are jurisdictional; a consumer holds no
jurisdiction), and cannot declare without an answer. A component-only batch —
the overwhelming majority of consumer traffic — answers nothing and commits
through the same `require_answer` path as any other author.

## Operator configuration

`ConsumerRegistry` is loaded once at startup from the file named by
`GHOSTLIGHT_CONSUMER_CREDENTIALS`: one `name = <64 hex digits>` line per
consumer (the SHA-256 of that consumer's shared secret), blank lines and `#`
comments ignored. A missing file means no consumers, and every document is
`Unauthenticated`, the fail-closed default that lets this route exist before
an operator configures anything. A file that exists but does not parse is
fatal at startup, so a mistyped credential can never read as "no consumers". The registry holds only digests; the
plaintext secret never enters the process image beyond the hash and never
enters a log, a receipt, or the journal.

## Deployment

The consumer capability's tag is part of the commit digest and the
externally controlled assignment is part of the state shape, so both bump the
schema: state schema `ghostlight.world_state.consumer.v1` (state-schema
generation `world-v3`), commit schema `ghostlight.world_commit.consumer.v1`.
A store written under an earlier schema is refused, not migrated.

## Not in this pass

- The outbound consumer response — a consumer reading its own projection, or
  receiving an attributed proposal about a Ghostlight subject acting toward
  its mirror. This is the next seam; building it prematurely would grow a
  second projection owner ahead of its use.
- A CultMesh authority lease for a non-loopback consumer. Until that exists,
  this route is loopback-only and offers a consumer no remote door.
