import {
  EveBrowserProviderHost,
  type EveBrowserProviderTransport,
  type EveCommandIntent,
  type EveProviderAdvertisement,
  type EveProviderSurfaceAdvertisement,
  type EveSurfaceDocument,
} from "@gamecult/eve-browser-lowering";
import { heimdallAccessBrowserAdapter, resumeHeimdallAccess } from "./heimdall-access-adapter";
import "@gamecult/eve-browser-lowering/styles.css";
import "./style.css";

class GhostlightEveTransport implements EveBrowserProviderTransport {
  private provider: EveProviderAdvertisement | undefined;
  private sourceVersion = 0;
  private authenticatedSurface = false;

  async providerAdvertisement(): Promise<EveProviderAdvertisement> {
    this.provider = await fetchJson<EveProviderAdvertisement>("api/eve/provider");
    return this.provider;
  }

  async surface(surface: EveProviderSurfaceAdvertisement): Promise<EveSurfaceDocument> {
    const surfaceId = surface.surfaceId || "ghostlight.play";
    const document = await fetchJson<EveSurfaceDocument>(`api/eve/surfaces/${encodeURIComponent(surfaceId)}`);
    this.sourceVersion = Math.max(0, Math.trunc(document.version || 0));
    this.authenticatedSurface = !containsAnonymousAccessGate(document);
    return document;
  }

  hasAuthenticatedSurface(): boolean {
    return this.authenticatedSurface;
  }

  async submitCommand(intent: EveCommandIntent): Promise<unknown> {
    const result = await fetchJson<any>("api/eve/commands", {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify(intent),
    });
    return result;
  }

  private advertisedTransport(): { transport?: string } {
    const commands = (this.provider as { commands?: Array<{ command?: string; transport?: string }> } | undefined)?.commands;
    const transport = commands?.find(command => command.command === "ghostlight.eve.commands")?.transport;
    return transport ? { transport } : {};
  }

  async completeAuthentication(handle: string): Promise<any> {
    if (!this.provider) await this.providerAdvertisement();
    return await this.submitCommand({
      schema: "gamecult.eve.command_invocation.v1",
      providerId: this.provider?.providerId || "gamecult.ghostlight.dungeon",
      surfaceId: "ghostlight.play",
      operation: {
        operationId: "heimdall.auth.complete",
        schemaId: "heimdall.auth_complete_command.v1",
        idempotencyKey: crypto.randomUUID(),
        // The route transport is the one the provider advertises for its
        // command boundary, as the vendored client takes it from a command
        // descriptor; Ghostlight refuses an invocation without it.
        routeHint: { sourceVersion: this.sourceVersion, ...this.advertisedTransport() },
      },
      payload: { handle },
      issuedAt: new Date().toISOString(),
      clientId: "ghostlight.browser",
      commandBoundary: "ghostlight.eve.commands",
      receiptSchema: "gamecult.eve.command_result.v1",
    });
  }
}

async function fetchJson<T = any>(input: RequestInfo | URL, init?: RequestInit): Promise<T> {
  const response = await fetch(input, { cache: "no-store", credentials: "same-origin", ...init });
  const body = await response.json().catch(() => undefined);
  if (!response.ok) {
    const message = body && typeof body.message === "string"
      ? body.message
      : `Ghostlight refused the request (${response.status}).`;
    throw new Error(message);
  }
  return body as T;
}

function containsAnonymousAccessGate(value: unknown): boolean {
  if (!value || typeof value !== "object") return false;
  if (Array.isArray(value)) return value.some(containsAnonymousAccessGate);
  const node = value as Record<string, unknown>;
  if (node.kind === "heimdall.access_gate") {
    const props = node.props && typeof node.props === "object" && !Array.isArray(node.props)
      ? node.props as Record<string, unknown>
      : {};
    return props.state === "anonymous";
  }
  return Object.values(node).some(containsAnonymousAccessGate);
}

function requiredElement<T extends HTMLElement>(id: string): T {
  const element = document.getElementById(id);
  if (!(element instanceof HTMLElement)) throw new Error(`Missing browser host #${id}.`);
  return element as T;
}

const surfaceHost = requiredElement<HTMLElement>("surface");
const status = requiredElement<HTMLElement>("status");
const api = new GhostlightEveTransport();
const host = new EveBrowserProviderHost(surfaceHost, api, {
  body: document.body,
  clientId: "ghostlight.browser",
  // PA.f191: this used to be 0 (no poll at all), relying entirely on the
  // `revision` SSE subscription below for every live update. A dropped
  // stream — closed by an intermediary, or never subscribed at all (see
  // `trySubscribeToRevisions` below) — then froze the card with no recovery
  // short of a manual reload. This poll is deliberately low-frequency: it is
  // a fallback net, not the primary channel, and `host.refresh()` is already
  // a no-op when the surface version has not moved.
  pollMs: 5_000,
  requestedSurfaceId: "ghostlight.play",
  source: "Ghostlight",
  statusElement: status,
  pluginAdapters: [heimdallAccessBrowserAdapter],
});

window.addEventListener("unhandledrejection", event => {
  event.preventDefault();
  status.textContent = event.reason instanceof Error ? event.reason.message : String(event.reason);
});

// PA.f191: `api.hasAuthenticatedSurface()` reflects whatever the *last*
// `surface()` fetch saw, and that flag can flip from false to true well
// after this module's own startup sequence finishes — a player who signs in
// through the in-page gate (`heimdall.auth_completion_status.v1`, resolved
// entirely inside the vendored host's own `submit()`/`refresh()` cycle,
// never observed by this file directly) authenticates on a timer this file
// has no direct hook into. A one-shot check right after startup only ever
// caught the redirect flow (`resumeHeimdallAccess`'s own `complete`
// callback, below); the in-page flow left the tab with no `revision`
// subscription for the rest of the session, so every answer after the first
// was refused as stale and the card looked frozen. `trySubscribeToRevisions`
// is idempotent and cheap to call speculatively — `hasAuthenticatedSurface()`
// is a field read — so it is called at every point this file can observe an
// authentication transition, plus on a short recheck interval to catch the
// in-page flow it cannot observe directly any other way.
let revisionsSubscribed = false;
function trySubscribeToRevisions(): void {
  if (revisionsSubscribed || !api.hasAuthenticatedSurface()) return;
  revisionsSubscribed = true;
  const events = new EventSource("api/eve/events");
  events.addEventListener("revision", () => void host.refresh());
  events.addEventListener("error", () => {
    status.textContent = "Live revision notices are unavailable; your authoritative surface remains safe to refresh.";
  });
}

await host.start();
trySubscribeToRevisions();
const resumeTarget = {
  complete: async (handle: string) => {
    const result = await api.completeAuthentication(handle);
    if (result.pluginPayload?.pluginId === heimdallAccessBrowserAdapter.pluginId) {
      await heimdallAccessBrowserAdapter.consumeCommandResult?.(result.pluginPayload);
    }
    await host.refresh();
    trySubscribeToRevisions();
  },
};
await resumeHeimdallAccess(resumeTarget, { appSlug: "ghostlight" });
trySubscribeToRevisions();

// A player who finishes Discord in another tab or app and switches back to
// this one, rather than following Heimdall's "Return to the app" link, never
// reloads this page, so the load-time resume above never sees the result.
// Re-running the same resume when the tab becomes visible asks Heimdall about
// the attempt this tab started; it answers pending or authenticated, and the
// adapter drops the stored attempt once the answer is final.
let resuming = false;
document.addEventListener("visibilitychange", () => {
  if (document.visibilityState !== "visible" || resuming || api.hasAuthenticatedSurface()) return;
  resuming = true;
  resumeHeimdallAccess(resumeTarget, { appSlug: "ghostlight" })
    .catch(error => {
      status.textContent = error instanceof Error ? error.message : String(error);
    })
    .finally(() => {
      resuming = false;
    });
});

// The in-page gate's own completion never calls back into this file: it
// resolves inside the vendored host's `submit()` (a command dispatch) and
// its own `window.setTimeout(() => void host.refresh(), 100)`, which is the
// one place `api.hasAuthenticatedSurface()` gets re-evaluated for that flow.
// This is the only vantage point this file has on that transition; the
// pollMs fallback above shares the same "cheap, idempotent, safe to run
// speculatively" reasoning.
window.setInterval(trySubscribeToRevisions, 2_000);
