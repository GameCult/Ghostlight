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

  async providerAdvertisement(): Promise<EveProviderAdvertisement> {
    this.provider = await fetchJson<EveProviderAdvertisement>("api/eve/provider");
    return this.provider;
  }

  async surface(surface: EveProviderSurfaceAdvertisement): Promise<EveSurfaceDocument> {
    const surfaceId = surface.surfaceId || "ghostlight.play";
    const pendingInvite = sessionStorage.getItem("ghostlight.pendingInvite");
    const query = pendingInvite ? `?invite=${encodeURIComponent(pendingInvite)}` : "";
    const document = await fetchJson<EveSurfaceDocument>(`api/eve/surfaces/${encodeURIComponent(surfaceId)}${query}`);
    this.sourceVersion = Math.max(0, Math.trunc(document.version || 0));
    return document;
  }

  async submitCommand(intent: EveCommandIntent): Promise<unknown> {
    const result = await fetchJson<any>("api/eve/commands", {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify(intent),
    });
    if (intent.operation.operationId === "session_zero.join" && result.receipt?.state === "accepted") {
      sessionStorage.removeItem("ghostlight.pendingInvite");
    }
    return result;
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
        routeHint: { sourceVersion: this.sourceVersion },
      },
      payload: { handle },
      issuedAt: new Date().toISOString(),
      clientId: "ghostlight.browser",
      commandBoundary: "ghostlight.eve.commands",
      receiptSchema: "gamecult.eve.command_result.v1",
    });
  }
}

const linkedInvite = new URL(window.location.href).searchParams.get("invite");
if (linkedInvite) {
  sessionStorage.setItem("ghostlight.pendingInvite", linkedInvite);
  history.replaceState(null, "", `${window.location.pathname}${window.location.hash}`);
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
  pollMs: 0,
  requestedSurfaceId: "ghostlight.play",
  source: "Ghostlight",
  statusElement: status,
  pluginAdapters: [heimdallAccessBrowserAdapter],
});

window.addEventListener("unhandledrejection", event => {
  event.preventDefault();
  status.textContent = event.reason instanceof Error ? event.reason.message : String(event.reason);
});

await host.start();
await resumeHeimdallAccess({
  complete: async handle => {
    const result = await api.completeAuthentication(handle);
    if (result.pluginPayload?.pluginId === heimdallAccessBrowserAdapter.pluginId) {
      await heimdallAccessBrowserAdapter.consumeCommandResult?.(result.pluginPayload);
    }
    await host.refresh();
  },
});

const events = new EventSource("api/eve/events");
events.addEventListener("revision", () => void host.refresh());
events.addEventListener("error", () => {
  status.textContent = "Live revision notices are unavailable; your authoritative surface remains safe to refresh.";
});
