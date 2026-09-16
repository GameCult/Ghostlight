# Ghostlight library extraction (plan step 15, L0)

Status: closed 2026-09-16. Landed in Ghostlight `108b691..ab95d78` and
gamecult-ops `6aba281..803f2c7`; the means and every Soul verdict are in
`ghostlight-library-extraction-cut.md`, the scars in
`ghostlight-library-extraction-postmortem.md`.

Reconciled with the Body. Invariant 1 holds as restated by Q1-9 and is
proven by `crates/ghostlight/tests/external_admission.rs`; the compile_fail
doc-tests prove "does not compile", not privacy, because stable rustc does
not enforce their error codes. Invariants 2, 4, 6 and 7 hold as written.
Invariant 3 holds for library source and tests: no Eve, Heimdall, AppSession
or player vocabulary remains. Invariant 5 holds on Windows; the Linux release
is unexercised until Idunn builds it. The open questions below were all
ruled (Q1-1 through Q1-10 in the cut map) and are kept as history.

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

   Sealing means **unforgeable admission**, not unconstructible values
   (operator ruling Q1-9, 2026-09-16). Public ID types, `ScopeDigest` and
   `DecisionOpportunity` derive `Deserialize` because Dungeon's own Eve
   payloads carry them, so any caller can build a syntactically valid one.
   The kernel must reject every one it did not issue, and rejection tests
   written from outside the crate are the proof. A compile-fail test pins a
   path name and cannot see constructibility: adding a `Default` impl that
   mints an ID left all ten green.
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
