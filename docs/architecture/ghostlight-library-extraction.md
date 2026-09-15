# Ghostlight library extraction (plan step 15, L0)

Status: target written 2026-09-15. Cut map pending (Imagination). Nothing
landed.

## Target

The Ghostlight library is a crate of its own. It owns the world kernel and
everything consumer-neutral around it: the ontology and reducer, the journal,
the mailbox, controllers and the Projector-Persona-Interpreter membrane,
elaboration and seeding, the cover, the clock, inference ports, evidence
sources, and consumer patch admission. Ghostlight Dungeon is a crate that
consumes it: the daemon binary, HTTP routes, Eve projection, Heimdall and app
sessions, CultMesh publication, and Idunn health.

After the cut, "Dungeon cannot reach kernel internals" is a compile error, not
a `pub(crate)` convention.

## Invariants that must survive

1. **Sealed kernel at the crate boundary.** The library's public API exposes
   create/open, immutable snapshots, command submission, typed receipts, and
   the runner and port entry points Dungeon needs. It exposes no mutable
   `WorldState`, canonical ID allocator, reducer entry, journal writer, or
   authenticated-caller constructor. Negative proofs are compile-fail tests
   from outside the library.
2. **No behavior change.** Every existing test passes unchanged in meaning.
   No schema string, command id namespace, digest preimage, or store layout
   changes: a `world.cc` and controller work store written before the cut open
   and replay after it.
3. **No Dungeon concept in the library.** No Dungeon type, route, Eve or
   Heimdall type, HTTP framework, or principal format enters the library. The
   principal evidence the kernel needs crosses as a library-owned type that
   Dungeon constructs from its own sessions.
4. **Dependencies follow ownership.** The library does not depend on axum,
   tower-http, jsonwebtoken, or the Heimdall and Eve surfaces. Dungeon depends
   on the library, never the reverse.
5. **The release path keeps working.** The `ghostlight-dungeon` binary name,
   its `--state-root` contract, and the Idunn recipe's build and test
   invocations still resolve, or are updated in the same cut with their owners.
6. **Existing pins are unaffected.** Epiphany's pin of
   `ghostlight-persona-projection` by revision still resolves.
7. **Build fan-out stays bounded.** One new library crate, no new binary, no
   new feature matrix beyond what test support strictly needs.

## Out of scope

- Every library capability of plan step 15 L1–L4: lenses, detail rules,
  evidence binding, Draft sessions. L0 moves code; it adds no behavior.
- Renaming schemas, stores, or wire documents.
- Changing how external consumers reach the library over the network. The
  consumer patch HTTP route stays in Dungeon for now.
- Deployment cutover (the deployment gate).

## Open questions for Imagination to bring back

- The library crate's name.
- Whether `vault.rs` (markdown evidence source) and `sdk_inference.rs` (Claude
  SDK port) belong in the library, as consumer-neutral adapters, or in Dungeon.
- The seam for principal evidence (`app_session::VerifiedPrincipalEvidence`).
- How test fixtures that span the boundary are shared without making sealed
  constructors public.
- Whether `ghostlight-persona-projection` stays a separate crate or folds into
  the library.
