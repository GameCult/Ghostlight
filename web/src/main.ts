import { renderEveSurface } from "@gamecult/eve-browser-lowering";
import "@gamecult/eve-browser-lowering/styles.css";
import "./style.css";

const host = document.querySelector<HTMLElement>("#surface")!;
const status = document.querySelector<HTMLElement>("#status")!;
const receipt = document.querySelector<HTMLElement>("#receipt")!;
let revision = 0;
let actorId = "player";

async function refresh() {
  const response = await fetch("/api/surface");
  if (response.status === 401) { status.textContent = "This laboratory requires an unused invite link."; return; }
  const surface = await response.json();
  revision = Number(surface.version ?? 0);
  renderEveSurface(surface, host, { body: document.body, clientId: "ghostlight.browser", statusElement: status });
}

async function send(command: unknown) {
  status.textContent = "The world is considering the command…";
  const response = await fetch("/api/command", { method: "POST", headers: { "content-type": "application/json" }, body: JSON.stringify(command) });
  const body = await response.json();
  receipt.hidden = false;
  receipt.textContent = JSON.stringify(body, null, 2);
  if (!response.ok) { status.textContent = body.error ?? "The command was refused."; return; }
  await refresh();
}

document.querySelector<HTMLFormElement>("#composer")!.addEventListener("submit", event => {
  event.preventDefault();
  const text = document.querySelector<HTMLTextAreaElement>("#attempt")!.value.trim();
  if (!text) return;
  void send({ type: "assess", expected_revision: revision, intent: { actor_id: actorId, description: text, intended_effect: text } });
});
document.querySelector<HTMLButtonElement>("#wait")!.addEventListener("click", () => void send({ type: "wait", expected_revision: revision, minutes: 60 }));
void refresh();
