import { renderEveSurface } from "@gamecult/eve-browser-lowering";
import "@gamecult/eve-browser-lowering/styles.css";
import "./style.css";

const host = document.querySelector<HTMLElement>("#surface")!;
const status = document.querySelector<HTMLElement>("#status")!;
const receipt = document.querySelector<HTMLElement>("#receipt")!;
const compiler = document.querySelector<HTMLElement>("#compiler")!;
const composer = document.querySelector<HTMLFormElement>("#composer")!;
const destinationForm = document.querySelector<HTMLFormElement>("#destination-form")!;
const compilerResults = document.querySelector<HTMLElement>("#compiler-results")!;
const campaignLab = document.querySelector<HTMLElement>("#campaign-lab")!;
const campaignList = document.querySelector<HTMLElement>("#campaign-list")!;
let revision = 0;
let resolutionEpoch = 0;
let providerConfigurationEpoch = 0;
let resolutionPins: any[] = [];

async function refresh() {
  const response = await fetch("/api/surface");
  if (response.status === 401) { status.textContent = "This laboratory requires an unused invite link."; return; }
  const surface = await response.json();
  const needsCompilation = surface.surface_id === "ghostlight.compiler";
  compiler.hidden = !needsCompilation; composer.hidden = needsCompilation; destinationForm.hidden = needsCompilation; host.hidden = needsCompilation; campaignLab.hidden = needsCompilation;
  if (needsCompilation) { status.textContent = "No campaign exists. Retrieve the Vault and approve a world seed."; return; }
  revision = Number(surface.world_revision ?? surface.version ?? 0);
  resolutionEpoch = Number(surface.resolution?.policy?.resolution_epoch ?? 0);
  providerConfigurationEpoch = Number(surface.resolution?.policy?.provider_configuration_epoch ?? 0);
  resolutionPins = surface.resolution?.pins ?? [];
  const budget = Number(surface.resolution?.policy?.active_cell_budget ?? 8);
  const budgetInput = document.querySelector<HTMLInputElement>("#active-cell-budget")!;
  budgetInput.value = String(budget);
  document.querySelector<HTMLOutputElement>("#active-cell-budget-value")!.value = String(budget);
  document.querySelector<HTMLElement>("#resolution-status")!.textContent = `${surface.resolution?.effective_budget ?? budget} effective cells · ${surface.resolution?.mandatory_overage ?? 0} temporary overage · epoch ${resolutionEpoch}`;
  const providerParallelism = Number(surface.resolution?.policy?.provider_parallelism ?? 8);
  document.querySelector<HTMLInputElement>("#provider-parallelism")!.value = String(providerParallelism);
  document.querySelector<HTMLOutputElement>("#provider-parallelism-value")!.value = String(providerParallelism);
  const pinList = document.querySelector<HTMLElement>("#pin-list")!;
  pinList.innerHTML = resolutionPins.map(pin => `<p><code>${pin.kind}</code> ${[...pin.subject_ids].join(", ")} — ${pin.reason} <button type="button" data-remove-pin="${pin.id}">Remove</button></p>`).join("") || "<p>No persistent pins.</p>";
  pinList.querySelectorAll<HTMLButtonElement>("[data-remove-pin]").forEach(button => button.addEventListener("click", async () => {
    await send({ type: "replace_resolution_pins", expected_revision: revision, expected_resolution_epoch: resolutionEpoch, pins: resolutionPins.filter(pin => pin.id !== button.dataset.removePin) });
  }));
  const fissionParent = document.querySelector<HTMLSelectElement>("#fission-parent")!;
  fissionParent.innerHTML = (surface.resolution?.fission_targets ?? []).map((target: any) => `<option value="${target.id}">${target.name} · ${target.id}</option>`).join("");
  document.querySelector<HTMLFormElement>("#fission-form")!.hidden = fissionParent.options.length === 0;
  renderEveSurface(surface, host, { body: document.body, clientId: "ghostlight.browser", statusElement: status });
  const campaigns = await fetch("/api/campaigns").then(response => response.json());
  campaignList.innerHTML = campaigns.campaigns.map((item: any) => `<button data-campaign-id="${item.id}" ${item.selected ? "disabled" : ""}>${item.selected ? "●" : "○"} ${item.name} · revision ${item.revision}</button>`).join("");
  campaignList.querySelectorAll<HTMLButtonElement>("button:not(:disabled)").forEach(button=>button.addEventListener("click",async()=>{await compilerPost(`/api/campaigns/select/${button.dataset.campaignId}`,{});await refresh();}));
}

async function compilerPost(path: string, body: unknown) {
  status.textContent = "Retrieving evidence and compiling…";
  const response = await fetch(path, { method: "POST", headers: { "content-type": "application/json" }, body: JSON.stringify(body) });
  const value = response.headers.get("content-type")?.includes("json") ? await response.json() : await response.text();
  if (!response.ok) { status.textContent = typeof value === "string" ? value : "Compilation failed"; throw new Error(status.textContent); }
  return value;
}

function showCards(items: any[], action: string, choose: (item: any) => void) {
  compilerResults.innerHTML = `<div class="cards">${items.map((item, index) => `<article class="card"><h3>${item.title ?? item.name}</h3><p>${item.player_hook ?? item.premise}</p><p>${item.era ? `${item.era} · ${item.place} · ${item.pressure}` : ""}</p><button data-index="${index}">${action}</button></article>`).join("")}</div>`;
  compilerResults.querySelectorAll<HTMLButtonElement>("button").forEach(button => button.addEventListener("click", () => choose(items[Number(button.dataset.index)])));
}

document.querySelector<HTMLFormElement>("#suggest-form")!.addEventListener("submit", async event => {
  event.preventDefault();
  const result = await compilerPost("/api/compiler/openings", { setting: document.querySelector<HTMLInputElement>("#setting")!.value, constraints: [] });
  showCards(result.openings, "Choose opening", async opening => {
    const roles = await compilerPost("/api/compiler/roles", opening);
    showCards(roles.roles, "Choose role", async role => showPreview(await compilerPost("/api/compiler/selected", { campaign_name: opening.title, opening, role })));
  });
});

document.querySelector<HTMLFormElement>("#custom-form")!.addEventListener("submit", async event => { event.preventDefault(); await showPreview(await compilerPost("/api/compiler/custom", Object.fromEntries(new FormData(event.currentTarget as HTMLFormElement)))); });

async function showPreview(result: any) {
  const preview = result.preview;
  compilerResults.innerHTML = `<article class="card"><h3>${preview.title}</h3><p>${preview.campaign.transcript?.[0]?.text ?? ""}</p><p>${Object.keys(preview.campaign.locations).length} locations · ${Object.keys(preview.campaign.actors).length} actors · ${Object.keys(preview.campaign.institutions).length} institutions</p><p class="warning">${preview.gaps.length ? `Material gaps: ${preview.gaps.join("; ")}` : "No declared material gaps."}</p><p>Branch assumptions: ${preview.branch_assumptions.join("; ") || "none"}</p><button id="approve-world">Approve and enter</button></article>`;
  document.querySelector<HTMLButtonElement>("#approve-world")!.addEventListener("click", async () => { await compilerPost(`/api/compiler/approve/${result.preview_id}`, {}); await refresh(); });
  status.textContent = "Preview compiled. Nothing has entered world state yet.";
}

async function send(command: unknown) { status.textContent = "The world is considering the command…"; const response = await fetch("/api/command", { method: "POST", headers: { "content-type": "application/json" }, body: JSON.stringify(command) }); const body = await response.json(); receipt.hidden = false; receipt.textContent = JSON.stringify(body, null, 2); if (!response.ok) { status.textContent = body.error ?? "The command was refused."; await refresh(); return body; } await refresh(); return body; }
function installAssessment(result: any) {
  if (result?.kind !== "assessed") return;
  const button = document.createElement("button");
  button.type = "button";
  button.textContent = result.assessment.admissible ? "Confirm server roll" : "Revise attempt";
  button.addEventListener("click", async () => {
    if (!result.assessment.admissible) {
      document.querySelector<HTMLTextAreaElement>("#attempt")!.focus();
      return;
    }
    const attempted = await send({ type: "attempt", assessment_digest: result.assessment.digest });
    button.remove();
    // A stale preview is compiled again against the new revision. It still
    // requires explicit confirmation; the server never rebases or auto-rolls it.
    installAssessment(attempted);
  });
  composer.append(button);
}
composer.addEventListener("submit", async event => {
  event.preventDefault();
  const text = document.querySelector<HTMLTextAreaElement>("#attempt")!.value.trim();
  if (!text) return;
  installAssessment(await send({ type: "assess", expected_revision: revision, intent: { actor_id: "player", description: text, intended_effect: text } }));
});
document.querySelector<HTMLButtonElement>("#wait")!.addEventListener("click", () => void send({ type: "wait", expected_revision: revision, minutes: 60 }));
document.querySelector<HTMLInputElement>("#active-cell-budget")!.addEventListener("input", event => {
  document.querySelector<HTMLOutputElement>("#active-cell-budget-value")!.value = (event.currentTarget as HTMLInputElement).value;
});
document.querySelector<HTMLFormElement>("#resolution-form")!.addEventListener("submit", async event => {
  event.preventDefault();
  await send({
    type: "set_resolution_budget",
    expected_revision: revision,
    expected_resolution_epoch: resolutionEpoch,
    active_cell_budget: Number(document.querySelector<HTMLInputElement>("#active-cell-budget")!.value),
  });
});
document.querySelector<HTMLInputElement>("#provider-parallelism")!.addEventListener("input", event => {
  document.querySelector<HTMLOutputElement>("#provider-parallelism-value")!.value = (event.currentTarget as HTMLInputElement).value;
});
document.querySelector<HTMLFormElement>("#provider-parallelism-form")!.addEventListener("submit", async event => {
  event.preventDefault();
  status.textContent = "Applying the provider concurrency limit…";
  const response = await fetch("/api/operator/provider-parallelism", {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({
      expected_provider_configuration_epoch: providerConfigurationEpoch,
      provider_parallelism: Number(document.querySelector<HTMLInputElement>("#provider-parallelism")!.value),
    }),
  });
  const body = await response.json();
  receipt.hidden = false;
  receipt.textContent = JSON.stringify(body, null, 2);
  if (!response.ok) status.textContent = body.error ?? "The operator limit was refused.";
  await refresh();
});
document.querySelector<HTMLFormElement>("#pin-form")!.addEventListener("submit", async event => {
  event.preventDefault();
  const data = new FormData(event.currentTarget as HTMLFormElement);
  const subjectIds = String(data.get("subject_ids") ?? "").split(",").map(value => value.trim()).filter(Boolean);
  const pin = {
    schema: "ghostlight.resolution_pin.v1",
    id: `pin:${crypto.randomUUID()}`,
    kind: String(data.get("kind")),
    subject_ids: subjectIds,
    reason: String(data.get("reason") ?? "").trim(),
    created_world_revision: revision,
  };
  await send({ type: "replace_resolution_pins", expected_revision: revision, expected_resolution_epoch: resolutionEpoch, pins: [...resolutionPins, pin] });
});
destinationForm.addEventListener("submit",async event=>{event.preventDefault();const result=await compilerPost("/api/compiler/destination",Object.fromEntries(new FormData(destinationForm)));const preview=result.preview;receipt.hidden=false;receipt.textContent=JSON.stringify(preview,null,2);status.textContent="Destination preview compiled; topology is unchanged until approval.";const button=document.createElement("button");button.textContent="Approve destination";button.addEventListener("click",async()=>{await compilerPost(`/api/compiler/destination/approve/${result.preview_id}`,{});button.remove();await refresh();});destinationForm.append(button);});
document.querySelector<HTMLFormElement>("#fission-form")!.addEventListener("submit", async event => {
  event.preventDefault();
  const data = new FormData(event.currentTarget as HTMLFormElement);
  const result = await compilerPost("/api/compiler/gestalt/fission", {
    parent_gestalt_id: data.get("parent_gestalt_id"),
    partition_axis: data.get("partition_axis"),
    requested_partition_values: String(data.get("requested_partition_values") ?? "").split(",").map(value => value.trim()).filter(Boolean),
    reason: data.get("reason"),
  });
  receipt.hidden = false;
  receipt.textContent = JSON.stringify(result.preview, null, 2);
  status.textContent = "Fission preview compiled. Canonical leaves are unchanged until approval.";
  const button = document.createElement("button");
  button.type = "button";
  button.textContent = "Approve population fission";
  button.addEventListener("click", async () => {
    await compilerPost(`/api/compiler/gestalt/fission/approve/${result.preview_id}`, {});
    button.remove();
    await refresh();
  });
  (event.currentTarget as HTMLFormElement).append(button);
});
document.querySelector<HTMLFormElement>("#fork-form")!.addEventListener("submit",async event=>{event.preventDefault();await compilerPost("/api/campaigns/fork",Object.fromEntries(new FormData(event.currentTarget as HTMLFormElement)));await refresh();});
document.querySelector<HTMLFormElement>("#reset-form")!.addEventListener("submit",async event=>{event.preventDefault();await compilerPost("/api/campaigns/reset",Object.fromEntries(new FormData(event.currentTarget as HTMLFormElement)));await refresh();});
document.querySelector<HTMLButtonElement>("#load-operator")!.addEventListener("click",async()=>{const response=await fetch("/api/operator");const surface=await response.json();renderEveSurface(surface,document.querySelector<HTMLElement>("#operator-output")!,{body:document.body,clientId:"ghostlight.operator",statusElement:status});});
void refresh();
