import { renderEveSurface } from "@gamecult/eve-browser-lowering";
import "@gamecult/eve-browser-lowering/styles.css";
import "./style.css";

const $ = <T extends Element>(selector: string) => document.querySelector<T>(selector)!;
const status = $("#status") as HTMLElement;
const entry = $("#entry") as HTMLElement;
const sessionRoot = $("#session-zero") as HTMLElement;
const surfaceHost = $("#surface") as HTMLElement;
const playControls = $("#play-controls") as HTMLElement;
const receipt = $("#receipt") as HTMLElement;
let revision = 0;
let resolutionEpoch = 0;
let viewerActorId = "";
let sessionId = "";
let viewerMemberId = "";
let activeChannel = "shared:table";
let currentSurface: any = null;
let requestActive = false;

class ServerFailure extends Error {}

function el<K extends keyof HTMLElementTagNameMap>(tag: K, text?: string, className?: string) {
  const value = document.createElement(tag);
  if (text !== undefined) value.textContent = text;
  if (className) value.className = className;
  return value;
}

async function decode(response: Response) {
  if (response.status === 204) return null;
  return (response.headers.get("content-type") ?? "").includes("json") ? response.json() : response.text();
}

function errorText(body: any, fallback: string) {
  if (typeof body === "string" && body.trim()) return body.trim();
  return typeof body?.error === "string" ? body.error : fallback;
}

async function post(path: string, body: unknown) {
  if (requestActive) throw new Error("Another table command is still being admitted.");
  requestActive = true;
  document.body.setAttribute("aria-busy", "true");
  try {
    const response = await fetch(path, { method: "POST", headers: { "content-type": "application/json" }, body: JSON.stringify(body) });
    const value = await decode(response);
    if (!response.ok) throw new ServerFailure(errorText(value, "The server refused the command."));
    return value;
  } finally {
    requestActive = false;
    document.body.removeAttribute("aria-busy");
  }
}

function showError(reason: unknown) {
  const message = reason instanceof Error ? reason.message : "The connection failed.";
  status.textContent = message;
  receipt.hidden = false;
  receipt.replaceChildren(el("h2", reason instanceof ServerFailure ? "Request refused" : "Request interrupted"), el("p", message), el("p", "The server remains authoritative. Refresh before retrying.", "quiet"));
}

window.addEventListener("unhandledrejection", event => { event.preventDefault(); showError(event.reason); });

function showAuthenticationGate() {
  entry.hidden = true; sessionRoot.hidden = true; playControls.hidden = true; surfaceHost.hidden = false;
  const card = el("article", undefined, "card");
  card.append(el("h2", "Ghostlight Dungeon playtest"), el("p", "Sign in through Heimdall with an eligible Discord account."));
  const button = el("button", "Continue with Discord");
  button.addEventListener("click", () => void startLogin().catch(showError));
  card.append(button);
  surfaceHost.replaceChildren(card);
}

async function startLogin() {
  const response = await fetch("/api/auth/heimdall/start", { method: "POST" });
  const body = await decode(response);
  if (!response.ok) throw new ServerFailure(errorText(body, "Heimdall could not begin sign-in."));
  sessionStorage.setItem("ghostlight_heimdall_attempt", body.attempt_id);
  window.location.assign(body.authorization_url);
}

async function resumeLogin() {
  const attempt = sessionStorage.getItem("ghostlight_heimdall_attempt");
  if (!attempt) return;
  const deadline = Date.now() + 600_000;
  while (Date.now() < deadline) {
    await new Promise(resolve => window.setTimeout(resolve, 900));
    const response = await fetch(`/api/auth/heimdall/attempt/${encodeURIComponent(attempt)}`, { cache: "no-store" });
    const body = await decode(response);
    if (!response.ok) throw new ServerFailure(errorText(body, "Heimdall sign-in expired."));
    if (body.status === "pending") continue;
    if (body.status !== "succeeded") throw new ServerFailure(body.error ?? "Discord access was not admitted.");
    const adopted = await fetch(`/api/auth/heimdall/attempt/${encodeURIComponent(attempt)}/adopt`, { method: "POST" });
    if (!adopted.ok) throw new ServerFailure(errorText(await decode(adopted), "Heimdall session adoption failed."));
    sessionStorage.removeItem("ghostlight_heimdall_attempt");
    return;
  }
  throw new Error("Heimdall sign-in timed out.");
}

async function refresh() {
  const response = await fetch("/api/surface", { cache: "no-store" });
  if (response.status === 401) return showAuthenticationGate();
  const surface = await decode(response);
  if (!response.ok) throw new ServerFailure(errorText(surface, "The table surface is unavailable."));
  currentSurface = surface;
  if (surface.surface_id === "ghostlight.session-zero-entry") return renderEntry();
  if (surface.session_zero) return renderSession(surface.session_zero);
  renderCampaign(surface);
}

function renderEntry() {
  status.textContent = "No campaign is selected. Begin or join a Session Zero.";
  entry.hidden = false; sessionRoot.hidden = true; surfaceHost.hidden = true; playControls.hidden = true;
  const invite = new URLSearchParams(location.search).get("invite");
  if (invite) ($("#join-token") as HTMLInputElement).value = invite;
}

function pairs(target: HTMLElement, fields: Record<string, unknown>) {
  target.replaceChildren();
  for (const [label, raw] of Object.entries(fields)) {
    const value = Array.isArray(raw) ? raw.join(" · ") : String(raw || "Not decided yet");
    target.append(el("dt", label.replaceAll("_", " ")), el("dd", value));
  }
}

function renderSession(session: any) {
  entry.hidden = true; sessionRoot.hidden = false; surfaceHost.hidden = true; playControls.hidden = true;
  sessionId = session.id; revision = Number(session.revision); viewerMemberId = session.viewer_member_id;
  if (!session.channels.some((channel: any) => channel.id === activeChannel)) activeChannel = "shared:table";
  $("#session-title").textContent = currentSurface.title;
  $("#session-state").textContent = `${String(session.status).replaceAll("_", " ")} · revision ${revision}`;
  status.textContent = session.status === "compiling" ? "The DM is compiling the agreed brief. You can leave this page." : "Session Zero is persistent. Conversation is not world truth until unanimous publication.";
  const shared = activeChannel === "shared:table";
  $("#tab-shared").setAttribute("aria-pressed", String(shared));
  $("#tab-private").setAttribute("aria-pressed", String(!shared));
  const channel = session.channels.find((item: any) => item.id === activeChannel) ?? session.channels[0];
  const transcript = $("#session-transcript"); transcript.replaceChildren();
  for (const message of channel?.messages ?? []) {
    const item = el("li", undefined, `message ${message.speaker}`);
    item.append(el("strong", message.speaker === "dm" ? "DM" : message.speaker === "system" ? "Table" : "Player"), el("p", message.text));
    transcript.append(item);
  }
  const roster = $("#roster"); roster.replaceChildren();
  for (const member of session.public_party) {
    const row = el("p", `${member.approved ? "Ready" : "Not ready"} · ${member.display_name} as ${member.name || "undecided"}${member.is_host ? " · host" : ""}`);
    if (session.viewer_is_host && !session.roster_locked && !member.is_host) {
      const remove = el("button", "Remove");
      remove.addEventListener("click", async () => { await post(`/api/session-zero/${sessionId}/remove-member`, { expected_revision: revision, member_id: member.member_id }); await refresh(); });
      row.append(" ", remove);
    }
    roster.append(row);
  }
  $("#cell-pool").textContent = `${session.pooled_cell_allowance} pooled active Persona cells · ${session.active_members}/8 players`;
  pairs($("#contract-ledger"), session.contract);
  pairs($("#character-ledger"), session.private_character);
  const boundaryList = $("#boundary-list"); boundaryList.replaceChildren(...session.private_boundaries.map((boundary: any) => {
    const item = el("li", `${boundary.level.replaceAll("_", " ")} · ${boundary.topic} `);
    const remove = el("button", "Remove"); remove.addEventListener("click", async () => { await post(`/api/session-zero/${sessionId}/boundary/${encodeURIComponent(boundary.id)}/remove`, { expected_revision: revision }); await refresh(); }); item.append(remove); return item;
  }));
  const decisions = $("#decision-list"); decisions.replaceChildren();
  for (const decision of session.decisions.filter((item: any) => !item.resolved)) {
    const card = el("article", undefined, "decision");
    card.append(el("h4", decision.prompt), el("p", decision.proposed_resolution));
    const accept = el("button", "Accept"); accept.addEventListener("click", () => void decide(decision.id, true));
    const counter = el("button", "Counter"); counter.addEventListener("click", async () => { const text = window.prompt("What should the DM propose instead?", decision.proposed_resolution); if (!text?.trim()) return; await post(`/api/session-zero/${sessionId}/decision`, { expected_revision: revision, decision_id: decision.id, accept: false, counter: text.trim() }); await refresh(); });
    const discuss = el("button", "Discuss"); discuss.addEventListener("click", () => { ($("#session-message") as HTMLTextAreaElement).value = `I'd like to discuss: ${decision.prompt}`; ($("#session-message") as HTMLTextAreaElement).focus(); });
    card.append(accept, counter, discuss); decisions.append(card);
  }
  const review = $("#review-card") as HTMLElement; review.hidden = !session.preview;
  if (session.preview) {
    const content = $("#review-content"); content.replaceChildren(el("p", session.preview.title));
    if (session.preview.branch_assumptions?.length) content.append(el("p", `Branch-local assumptions: ${session.preview.branch_assumptions.join("; ")}`));
    content.append(el("p", `${Object.keys(session.preview.topology ?? {}).length} locations · ${Object.keys(session.preview.institutions ?? {}).length} institutions · ${Object.keys(session.preview.clocks ?? {}).length} clocks`));
  }
  const host = Boolean(session.viewer_is_host);
  ($("#invite-form") as HTMLFormElement).hidden = !host || session.roster_locked;
  ($("#lock-roster") as HTMLButtonElement).disabled = !host || session.roster_locked;
  ($("#compile-session") as HTMLButtonElement).disabled = !host || !session.roster_locked || ["compiling", "review", "published"].includes(session.status);
  ($("#approve-session") as HTMLButtonElement).disabled = session.status !== "review" || session.approved;
  ($("#publish-session") as HTMLButtonElement).disabled = !host || !session.publish_ready;
}

function renderCampaign(surface: any) {
  entry.hidden = true; sessionRoot.hidden = true; surfaceHost.hidden = false; playControls.hidden = false;
  revision = Number(surface.world_revision); resolutionEpoch = Number(surface.resolution?.policy?.resolution_epoch ?? 0); viewerActorId = surface.viewer_actor_id;
  if (!viewerActorId) throw new Error("Actor-filtered campaign surface omitted the viewer actor binding.");
  renderEveSurface(surface, surfaceHost, { body: document.body, clientId: "ghostlight.browser", statusElement: status });
  status.textContent = `World revision ${revision}. Your surface contains only ${viewerActorId}'s permitted state.`;
  const budget = Number(surface.resolution?.policy?.active_cell_budget ?? 8);
  ($("#active-cell-budget") as HTMLInputElement).value = String(budget); ($("#active-cell-budget-value") as HTMLOutputElement).value = String(budget);
  const travelSelect = $("#travel-destination") as HTMLSelectElement;
  travelSelect.replaceChildren(...(surface.reachable_destinations ?? []).map((destination: any) => {
    const option = document.createElement("option"); option.value = destination.id; option.textContent = `${destination.name} · ${destination.travel_minutes} minutes`; return option;
  }));
  renderGovernance(surface.governance ?? {});
}

function renderGovernance(governance: any) {
  const timeForm = $("#time-form") as HTMLFormElement;
  const travelForm = $("#travel-form") as HTMLFormElement;
  const budgetForm = $("#resolution-form") as HTMLFormElement;
  const pendingTime = governance.time_proposals?.[0];
  const pendingTravel = governance.travel_proposals?.[0];
  const pendingBudget = governance.cell_budget_proposals?.[0];
  const timeButton = timeForm.querySelector("button")!;
  const travelButton = travelForm.querySelector("button")!;
  const budgetButton = budgetForm.querySelector("button")!;
  timeButton.textContent = pendingTime ? `Approve ${pendingTime.minutes} minute advance (${pendingTime.approvals.length}/${governance.active_member_count})` : "Propose time";
  travelButton.textContent = pendingTravel ? `Approve group travel (${pendingTravel.approvals.length}/${governance.active_member_count})` : "Propose travel";
  budgetButton.textContent = pendingBudget ? `Approve ${pendingBudget.active_cell_budget} cells (${pendingBudget.approvals.length}/${governance.active_member_count})` : "Propose budget";
  timeForm.dataset.proposalId = pendingTime?.id ?? ""; travelForm.dataset.proposalId = pendingTravel?.id ?? ""; budgetForm.dataset.proposalId = pendingBudget?.id ?? "";
}

async function decide(decisionId: string, accept: boolean) {
  await post(`/api/session-zero/${sessionId}/decision`, { expected_revision: revision, decision_id: decisionId, accept, counter: null }); await refresh();
}

function showCommand(body: any) {
  receipt.hidden = false;
  if (body.kind === "assessed") {
    const a = body.assessment; receipt.replaceChildren(el("h2", a.admissible ? `Assessment · DC ${a.dc}` : "Attempt not admitted"), el("p", a.admissible ? `Success: ${a.success_stake}` : a.missing_permission));
    if (a.admissible) { const confirm = el("button", "Confirm server roll"); confirm.addEventListener("click", () => void sendWorld({ type: "attempt", actor_id: viewerActorId, assessment_digest: a.digest })); receipt.append(confirm); }
  } else if (body.kind?.includes("pending")) receipt.replaceChildren(el("h2", "Waiting for the table"), el("p", "The proposal is revision-bound and will commit only after every active member approves."));
  else receipt.replaceChildren(el("h2", "World committed"), el("p", body.receipt?.roll ? `d20 ${body.receipt.roll.d20} · ${String(body.receipt.roll.outcome).replaceAll("_", " ")}` : `Revision ${body.revision ?? revision}`));
}

async function sendWorld(command: unknown) { const body = await post("/api/command", command); showCommand(body); await refresh(); return body; }

$("#session-entry-form").addEventListener("submit", async event => { event.preventDefault(); const data = Object.fromEntries(new FormData(event.currentTarget as HTMLFormElement)); delete data.mode; await post("/api/session-zero", data); await refresh(); });
$("#join-form").addEventListener("submit", async event => { event.preventDefault(); const token = ($("#join-token") as HTMLInputElement).value.trim(); await post(`/api/session-zero/join/${encodeURIComponent(token)}`, { display_name: ($("#join-name") as HTMLInputElement).value.trim() }); history.replaceState({}, "", location.pathname); await refresh(); });
$("#tab-shared").addEventListener("click", () => { activeChannel = "shared:table"; renderSession(currentSurface.session_zero); });
$("#tab-private").addEventListener("click", () => { activeChannel = `private:${viewerMemberId}`; renderSession(currentSurface.session_zero); });
$("#session-message-form").addEventListener("submit", async event => { event.preventDefault(); const input = $("#session-message") as HTMLTextAreaElement; await post(`/api/session-zero/${sessionId}/message`, { expected_revision: revision, channel_id: activeChannel, text: input.value.trim() }); input.value = ""; status.textContent = "Your message is durable. The DM is considering it."; await refresh(); });
$("#invite-form").addEventListener("submit", async event => { event.preventDefault(); const body = await post(`/api/session-zero/${sessionId}/invites`, { count: Number(($(
  "#invite-count") as HTMLInputElement).value) }); const output = $("#invite-output"); output.replaceChildren(); for (const token of body.invite_tokens) { const url = `${location.origin}${location.pathname}?invite=${encodeURIComponent(token)}`; const p = el("p"); const link = el("a", url); link.href = url; p.append(link); output.append(p); } await refresh(); });
$("#boundary-form").addEventListener("submit", async event => { event.preventDefault(); await post(`/api/session-zero/${sessionId}/boundary`, { expected_revision: revision, boundary_id: null, topic: ($("#boundary-topic") as HTMLInputElement).value.trim(), level: ($("#boundary-level") as HTMLSelectElement).value }); ($("#boundary-topic") as HTMLInputElement).value = ""; await refresh(); });
$("#lock-roster").addEventListener("click", async () => { await post(`/api/session-zero/${sessionId}/lock`, { expected_revision: revision }); await refresh(); });
$("#compile-session").addEventListener("click", async () => { await post(`/api/session-zero/${sessionId}/compile`, { expected_revision: revision }); await refresh(); });
$("#approve-session").addEventListener("click", async () => { await post(`/api/session-zero/${sessionId}/approve`, { expected_revision: revision }); await refresh(); });
$("#publish-session").addEventListener("click", async () => { await post(`/api/session-zero/${sessionId}/publish`, { expected_revision: revision }); await refresh(); });
$("#composer").addEventListener("submit", async event => { event.preventDefault(); const description = ($("#attempt") as HTMLTextAreaElement).value.trim(); const intended = ($("#intended-effect") as HTMLInputElement).value.trim(); if (!intended) return void (status.textContent = "Name the uncertain effect, or use Speak."); await sendWorld({ type: "assess", expected_revision: revision, intent: { actor_id: viewerActorId, description, intended_effect: intended } }); });
$("#speak").addEventListener("click", async () => { const input = $("#attempt") as HTMLTextAreaElement; await sendWorld({ type: "speak", expected_revision: revision, actor_id: viewerActorId, text: input.value.trim(), intended_effect: null }); input.value = ""; });
$("#contract-review").addEventListener("click", async () => { await post("/api/campaigns/contract-review", {}); await refresh(); });
$("#active-cell-budget").addEventListener("input", event => { ($("#active-cell-budget-value") as HTMLOutputElement).value = (event.currentTarget as HTMLInputElement).value; });
$("#time-form").addEventListener("submit", async event => { event.preventDefault(); const form = event.currentTarget as HTMLFormElement; const id = form.dataset.proposalId; const body = id ? await post(`/api/governance/time/${encodeURIComponent(id)}/approve`, { expected_revision: revision }) : await post("/api/governance/time", { expected_revision: revision, minutes: Number(($(
  "#time-minutes") as HTMLInputElement).value) }); showCommand(body); await refresh(); });
$("#travel-form").addEventListener("submit", async event => { event.preventDefault(); const form = event.currentTarget as HTMLFormElement; const id = form.dataset.proposalId; const destination = ($("#travel-destination") as HTMLSelectElement).value; if (!id && !destination) return void (status.textContent = "No reachable destination is compiled from this scene."); const body = id ? await post(`/api/governance/travel/${encodeURIComponent(id)}/approve`, { expected_revision: revision }) : await post("/api/governance/travel", { expected_revision: revision, destination_location_id: destination }); showCommand(body); await refresh(); });
$("#resolution-form").addEventListener("submit", async event => { event.preventDefault(); const form = event.currentTarget as HTMLFormElement; const id = form.dataset.proposalId; const body = id ? await post(`/api/governance/cell-budget/${encodeURIComponent(id)}/approve`, { expected_revision: revision }) : await post("/api/governance/cell-budget", { expected_revision: revision, expected_resolution_epoch: resolutionEpoch, active_cell_budget: Number(($(
  "#active-cell-budget") as HTMLInputElement).value) }); showCommand(body); await refresh(); });

const events = new EventSource("/api/events");
let refreshQueued = false;
events.addEventListener("revision", () => { if (refreshQueued) return; refreshQueued = true; window.setTimeout(() => { refreshQueued = false; void refresh().catch(showError); }, 100); });
events.onerror = () => { status.textContent = "Realtime revision notices are reconnecting; the server remains authoritative."; };

void resumeLogin().then(refresh).catch(showError);
