# Ghostlight Eve-native interface

## Objective

Ghostlight exposes one actor-filtered logical surface, `ghostlight.play`. Eve
owns its editable bindings, command invocation, receipts, plugin composition,
and browser lowering. Ghostlight continues to own world creation
(`world.create` v2), the Draft seed lane (`world.seed`), and world admission;
Heimdall continues to own authentication and entitlement decisions.

```text
Ghostlight projection
  -> EveBrowserProviderHost
  -> canonical Eve lowering
  -> gamecult.eve.command_invocation.v1
  -> Ghostlight ingress or Heimdall private command plane
  -> gamecult.eve.command_result.v1
  -> authoritative surface refresh
```

## Authority map

- Owner: `SessionZeroKernel` owns draft changes, `WorldKernel` owns campaign
  changes, Heimdall owns OAuth attempts and claims, and Ghostlight's app-session
  owner binds a verified Heimdall subject to a local cookie.
- Inputs: the provider projector reads only the caller's app session,
  membership, actor-filtered campaign or Draft world slice, and public
  Heimdall gate state. Command ingress reads one canonical Eve invocation and
  derives member and actor identity server-side.
- Outputs: one `gamecult.eve.surface.v1` document and one persisted
  `gamecult.eve.command_receipt.v1` inside a command result. Invitation links
  may appear only in a transient projection.
- Derived state: browser drafts, tab selection, focus, command status, the
  selected campaign card, and rendered HTML are projections or local
  interaction state. None grants access or mutates fiction.
- Forbidden writers: the browser, Eve lowerer, Heimdall plugin, SSE stream,
  account preferences, and old auth store cannot choose an actor, establish
  membership, approve a Draft admission, or commit world state.
- Shared paths: every UI operation resolves the authenticated account and
  exact membership, then calls the existing world mailbox.
  SSE carries invalidation only and the host refetches the same surface.
- Cut line: the bespoke browser renderers and product-specific API routes are
  removed from the public router. Ghostlight's backend callback and local
  OAuth-attempt store stop participating. `service/auth.cc` is migration and
  rollback evidence only; `service/app-sessions.cc` owns new local sessions and
  account preferences.

## Editable values

Eve bindings are the input model. A component binds a typed value by stable
binding name, document identity, field path, value kind, access mode,
authority, and optional write command. Renderer-local composers use
`local-draft`; direct edits name provider-owned state and an advertised write
command. An operation captures one or more named binding values atomically.

The browser lowerer may use an HTML form for keyboard and accessibility
behavior. HTML form structure never enters Eve state or command payload
semantics. Accepted receipts may clear named drafts; rejection and stale
conflict preserve them. An omitted clear-binding list means clear the surface's
drafts; an explicit empty list means clear nothing. Operations that capture no
editable bindings therefore cannot erase an unrelated composer, channel
selection, boundary draft, or counterproposal. Provider-authored length limits
lower onto editable controls before submission; Ghostlight still validates the
same bound at ingress. Denied command receipts use an accessible alert region
and survive authoritative surface refreshes alongside the preserved draft.

## Authentication membrane

Anonymous projection contains only `heimdall.access_gate`. Its begin and
complete operations cross Heimdall's encrypted, loopback-only CultNet boundary.
Ghostlight first reads the redacted `heimdall:command-boundary` record from
Odin, validates its schema, runtime, loopback route, operations, and HMAC/AES
contract, and then invokes that discovered route. Odin does not proxy the
command and never receives claims or completion payloads. A discovery outage
fails new authentication closed while already-valid local Ghostlight sessions
continue until their verified expiry.
The browser retains only the opaque attempt handle. Discord returns to
Heimdall; Ghostlight redeems the completion, validates the access claim and
`app_access`, and creates a local HttpOnly session.

Routine requests use local verified session state. The cookie hash, stable
account-subject hash, Heimdall session/revision, capabilities, expiries, and
wrapped refresh claim persist in `app-sessions.cc`. Campaign authorization is
always derived from `campaign_membership.v1`; account preferences contain only
the selected campaign. `campaign.entry` may clear only that preference so the
same authenticated player can return to the campaign list or begin creating
another world with `world.create`; it cannot leave, reset, fork, or mutate any
campaign.
Session Zero registry lookup treats only non-terminal negotiations as active:
`published` and `archived` records remain durable history but cannot replace the
campaign-entry surface after the preference is cleared.

Transient command-result surfaces preserve Eve's composite campaign interface
version. A fresh or recompiled assessment may replace only the campaign-revision
component while retaining the resolution and provider-configuration epochs.
The roll control therefore invokes against the same version namespace as the
authoritative `ghostlight.play` surface instead of collapsing it to a raw world
revision.

## Public boundary

- `GET /api/eve/provider`
- `GET /api/eve/surfaces/ghostlight.play`
- `POST /api/eve/commands`
- `GET /api/eve/events`

`/health` remains a probe of the service's typed CultMesh health. Static assets
contain only the provider host, transport, Heimdall adapter, status mount, and
SSE invalidation. Hermodr is not a runtime dependency.

## Live acceptance witness

The Yggdrasil release at Ghostlight commit
`b515ca90c25573005a616244143803b37f2d06ec` and Eve commit
`6766bee7c14a47144191475e2f35b0343b647b45` serves the anonymous access gate
through the canonical browser lowerer with no console errors. A canonical
`heimdall.auth.begin` invocation was accepted through Odin-discovered Heimdall
and returned only the plugin-scoped navigation receipt with advertised Discord
and Heimdall origins. The deployed unit contains
`GHOSTLIGHT_ODIN_RUDP=10.77.0.1:17871` and no direct Heimdall private endpoint.
