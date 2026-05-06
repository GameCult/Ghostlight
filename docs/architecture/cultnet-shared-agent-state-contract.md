# CultNet Shared Agent-State Contract

Ghostlight owns the first cross-runtime agent-state payload contract.

That contract is not a new ontology. It is the existing Ghostlight state shape
moving over a wire without turning into bespoke JSON fog in every other repo.

## Canonical Payload

The first shared payload contract is:

- schema id:
  `https://github.com/GameCult/Ghostlight/schemas/agent-state.schema.json`
- schema version:
  `ghostlight.agent_state.v0`
- canonical file:
  `schemas/agent-state.schema.json`

This is the same state shape already used for Ghostlight fixtures and the same
top-level shape Epiphany role dossiers adopted. The contract is:

- `world`
- `agents`
- `relationships`
- `events`
- `scenes`

If another runtime wants to carry socially persistent agent state without
inventing its own private folk religion, this is the first house shape.

## Wire Envelope

CultNet should carry the Ghostlight payload as a typed document replication
message, not as a one-off ad hoc blob.

Current expected document envelope:

- `documentType`: `ghostlight.agent-state`
- `documentKey`: runtime-defined stable key
- `payloadSchemaVersion`: `ghostlight.agent_state.v0`
- `payload`: exact Ghostlight state document

The current TypeScript client library is:

- repo: `GameCult/cultnet-ts`

Its job is not merely to make pipes pretty. It owns:

- direct-pipe MessagePack framing
- typed message contracts
- authentication/session semantics compatible with CultLib's `GameCult.Networking`
- typed replication into CultCache-compatible stores

## Authentication Story

CultNet inherits the useful security shape from CultLib:

- a shared connection key known to client and server/runtime peers
- AES-GCM encrypted auth/session payloads derived from that key
- a server-side session-signing secret
- signed session tokens used for verify/reconnect flows

This matters because "shared agent state" is not only a serialization problem.
It is also a trust boundary problem. We want one family resemblance across C#,
TypeScript, Rust, and Python instead of four mutually unintelligible little
shrines to local convenience.

## Contract Discipline

Ghostlight remains the source of truth for the payload shape.

That means:

- if `schemas/agent-state.schema.json` changes, CultNet mirrors must update in
  lockstep
- consumers may add local document metadata around the payload, but should not
  silently mutate the payload contract itself
- if a runtime needs a narrower projection or local working view, that is a
  derivative contract, not a revision of `ghostlight.agent_state.v0`

The rule is simple:

- prompt projection may differ
- storage engines may differ
- language bindings may differ
- the shared payload contract does not drift just because another repo got
  lonely

## Immediate Consumers

The first intended consumers are:

- Ghostlight fixture/state tooling
- EpiphanyAgent role dossiers and heartbeat-adjacent agent state
- VoidBot companion-state and social memory machinery where the full Ghostlight
  payload is useful

More bindings can come later. They do not get to invent a prettier truth.
