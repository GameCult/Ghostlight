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
let revision = 0;

async function refresh() {
  const response = await fetch("/api/surface");
  if (response.status === 401) { status.textContent = "This laboratory requires an unused invite link."; return; }
  const surface = await response.json();
  const needsCompilation = surface.surface_id === "ghostlight.compiler";
  compiler.hidden = !needsCompilation; composer.hidden = needsCompilation; destinationForm.hidden = needsCompilation; host.hidden = needsCompilation;
  if (needsCompilation) { status.textContent = "No campaign exists. Retrieve the Vault and approve a world seed."; return; }
  revision = Number(surface.version ?? 0);
  renderEveSurface(surface, host, { body: document.body, clientId: "ghostlight.browser", statusElement: status });
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
composer.addEventListener("submit", async event => { event.preventDefault(); const text = document.querySelector<HTMLTextAreaElement>("#attempt")!.value.trim(); if (!text) return; const result=await send({ type: "assess", expected_revision: revision, intent: { actor_id: "player", description: text, intended_effect: text } }); if(result?.kind==="assessed"){const button=document.createElement("button");button.textContent=result.assessment.admissible?"Confirm server roll":"Revise attempt";button.addEventListener("click",async()=>{if(result.assessment.admissible){await send({type:"attempt",assessment_digest:result.assessment.digest});button.remove();}else{document.querySelector<HTMLTextAreaElement>("#attempt")!.focus();}});composer.append(button);} });
document.querySelector<HTMLButtonElement>("#wait")!.addEventListener("click", () => void send({ type: "wait", expected_revision: revision, minutes: 60 }));
destinationForm.addEventListener("submit",async event=>{event.preventDefault();const result=await compilerPost("/api/compiler/destination",Object.fromEntries(new FormData(destinationForm)));const preview=result.preview;receipt.hidden=false;receipt.textContent=JSON.stringify(preview,null,2);status.textContent="Destination preview compiled; topology is unchanged until approval.";const button=document.createElement("button");button.textContent="Approve destination";button.addEventListener("click",async()=>{await compilerPost(`/api/compiler/destination/approve/${result.preview_id}`,{});button.remove();await refresh();});destinationForm.append(button);});
void refresh();
