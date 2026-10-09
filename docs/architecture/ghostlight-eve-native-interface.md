# Ghostlight Eve-native interface

## Objective

Ghostlight Dungeon exposes one logical surface, `ghostlight.play`, from the
provider `gamecult.ghostlight.dungeon` (`crates/ghostlight-dungeon/src/mesh.rs`).
Eve owns editable bindings, command invocation, receipts, plugin composition,
and browser lowering. Ghostlight owns world creation, seeding, activation, play,
and world admission; Heimdall owns authentication and entitlement decisions.

```text
Ghostlight projection (eve.rs)
  -> EveBrowserProviderHost
  -> canonical Eve lowering
  -> gamecult.eve.command_invocation.v1
  -> Ghostlight command ingress (runtime.rs) or Heimdall private command plane
  -> gamecult.eve.command_result.v1
  -> authoritative surface refresh
```

## Authority map

- Owner: the world kernel owns world changes (through `WorldMailbox`, `SeedPort`
  and the play table), Heimdall owns OAuth attempts and claims, and Ghostlight's
  app-session owner (`app_session.rs`) binds a verified Heimdall subject to a
  local cookie.
- Inputs: the projector reads only the caller's app session, the world
  snapshot, and the caller's play-turn view. Command ingress reads one canonical
  Eve invocation and derives the principal server-side.
- Outputs: one `gamecult.eve.surface.v1` document and one
  `gamecult.eve.command_receipt.v1` inside a command result.
- Derived state: browser drafts, focus, command status, and rendered HTML are
  projections or local interaction state. None grants access or mutates
  fiction.
- Forbidden writers: the browser, the Eve lowerer, the Heimdall plugin, and the
  SSE stream cannot choose a principal or commit world state.
- Shared path: every operation resolves the authenticated account, then calls
  the world mailbox, the seed port, or the play port.
  SSE carries invalidation only and the host refetches the same surface.

## The surface

`GET /api/eve/surfaces/ghostlight.play` returns a different document by caller
and world phase (`anonymous_surface`, `authenticated_surface` in
`crates/ghostlight-dungeon/src/eve.rs`):

| Caller and world | Content | Operations |
|---|---|---|
| Anonymous | `heimdall.access_gate` | `heimdall.auth.begin`, `heimdall.auth.complete` |
| Authenticated, any world state | a `Sign out` button on every authenticated surface | `app.auth.logout` (`ghostlight.app_logout.v2`) |
| Authenticated, no world | the create form: title, brief, your name, optional Narrative persona and Operational agent labels, scale target, jurisdiction roots, lens weights | `world.create` (`ghostlight.world_create.v4`) |
| Draft world | world summary card; approve button for a required approver who has not approved; for the owner, the seed form and Seed card, and the activate button once every required approver has approved | `world.approve` (`ghostlight.world_approve.v0`), `world.seed` (`ghostlight.world_seed.v1`), `world.activate` (`ghostlight.world_activate.v0`) |
| Active world, owner | advance-time control and the Play card: narration, the open question, the refusal of the player's own act, one free-text control | `world.advance_time` (`ghostlight.world_advance_time.v0`), `world.play` (`ghostlight.world_play.v0`) |

`world.play` is the only human play action. The open question's identity rides
the play button's own action as an opaque token, never as an editable field.
`operation_schema` in `eve.rs` is the single table from operation id to schema.

## Editable values

Eve bindings are the input model. A component binds a typed value by stable
binding name, document identity, field path, value kind, and access mode.
Renderer-local composers use `local-draft`. An operation captures one or more
named binding values atomically.

The browser lowerer may use an HTML form for keyboard and accessibility
behavior. HTML form structure never enters Eve state or command payload
semantics. Accepted receipts may clear named drafts; rejection and stale
conflict preserve them. An omitted clear-binding list means clear the surface's
drafts; an explicit empty list means clear nothing. Ghostlight validates every
payload at ingress regardless of what the control advertises.

## Authentication membrane

Anonymous projection contains only `heimdall.access_gate`. Its begin and
complete operations cross Heimdall's encrypted, loopback-only CultNet boundary
(`heimdall.rs`). Ghostlight reads the redacted `heimdall:command-boundary`
record from Odin, validates it, and invokes the discovered route. Odin does not
proxy the command and never receives claims or completion payloads. The browser
retains only the opaque attempt handle (`web/src/heimdall-access-adapter.ts`).
Ghostlight redeems the completion, validates the access claim, and creates a
local HttpOnly session cookie (`ghostlight_session`).

Routine requests use local verified session state, persisted in
`app-sessions-v2.cc` (`app_session.rs`): the cookie hash, the Heimdall
session and `access_revision`, expiries, and a wrapped refresh claim.
`access_revision` is Heimdall's revocation epoch.

## Public boundary

- `GET /api/eve/provider`
- `GET /api/eve/surfaces/{surface_id}`
- `POST /api/eve/commands`
- `GET /api/eve/events`
- `GET /health`, a probe of the service's typed CultMesh health
- `POST /cultnet/snapshot` and `POST /cultnet/world-patch`, the loopback
  CultNet doors (see `ghostlight-world-consumer-api.md` for the second)

Static assets in `web/` contain only the Eve provider host, the transport, the
Heimdall adapter, and the SSE invalidation hook.
