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

function node<K extends keyof HTMLElementTagNameMap>(
  tag: K,
  text?: string,
  className?: string,
): HTMLElementTagNameMap[K] {
  const value = document.createElement(tag);
  if (text !== undefined) value.textContent = text;
  if (className) value.className = className;
  return value;
}

function showSummary(title: string, lines: string[]) {
  const children: HTMLElement[] = [node("h3", title)];
  for (const line of lines.filter(Boolean)) children.push(node("p", line));
  receipt.replaceChildren(...children);
  receipt.hidden = false;
}

function renderCommandReceipt(body: any) {
  if (body?.kind === "assessed") {
    const assessment = body.assessment;
    const children: HTMLElement[] = [node("h3", assessment.admissible ? "Action assessment" : "That attempt is not possible")];
    if (assessment.admissible) {
      children.push(node("p", `DC ${assessment.dc} · modifier ${assessment.modifier_total >= 0 ? "+" : ""}${assessment.modifier_total}`));
      const modifiers = node("ul");
      for (const modifier of assessment.modifiers ?? []) {
        modifiers.append(node("li", `${modifier.label}: ${modifier.value >= 0 ? "+" : ""}${modifier.value}`));
      }
      children.push(modifiers);
      children.push(node("p", `Strong/success: ${assessment.success_stake}`));
      children.push(node("p", `Mixed: ${assessment.mixed_stake}`));
      children.push(node("p", `Failure: ${assessment.failure_stake}`));
      children.push(node("p", `Effect ceiling: ${assessment.effect_ceiling}`, "quiet"));
    } else {
      children.push(node("p", assessment.missing_permission ?? "The attempt has no admissible path from current state."));
      for (const bargain of assessment.bargains ?? []) children.push(node("p", `Possible bargain: ${bargain}`));
    }
    receipt.replaceChildren(...children);
    receipt.hidden = false;
    return;
  }
  if (body?.kind === "committed") {
    const roll = body.receipt?.roll;
    if (roll) {
      const outcome = String(roll.outcome).replaceAll("_", " ");
      showSummary(outcome.charAt(0).toUpperCase() + outcome.slice(1), [
        `d20 ${roll.d20} ${roll.modifier_total >= 0 ? "+" : "−"} ${Math.abs(roll.modifier_total)} = ${roll.total} against DC ${roll.dc}`,
      ]);
    } else {
      showSummary("World advanced", [`Revision ${body.revision}`]);
    }
    return;
  }
  if (body?.kind === "resolution_updated") {
    showSummary("World resolution updated", [`Resolution epoch ${body.receipt?.resolution_epoch ?? "advanced"}`]);
    return;
  }
  showSummary("Command result", [body?.error ?? "The world returned no player-facing detail."]);
}

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
  pinList.replaceChildren();
  if (resolutionPins.length === 0) pinList.append(node("p", "No persistent pins."));
  for (const pin of resolutionPins) {
    const row = node("p");
    row.append(node("code", String(pin.kind)), document.createTextNode(` ${[...pin.subject_ids].join(", ")} — ${pin.reason} `));
    const remove = node("button", "Remove");
    remove.type = "button";
    remove.addEventListener("click", async () => {
      await send({ type: "replace_resolution_pins", expected_revision: revision, expected_resolution_epoch: resolutionEpoch, pins: resolutionPins.filter(candidate => candidate.id !== pin.id) });
    });
    row.append(remove);
    pinList.append(row);
  }
  const fissionParent = document.querySelector<HTMLSelectElement>("#fission-parent")!;
  fissionParent.replaceChildren(...(surface.resolution?.fission_targets ?? []).map((target: any) => {
    const option = node("option", `${target.name} · ${target.id}`);
    option.value = target.id;
    return option;
  }));
  document.querySelector<HTMLFormElement>("#fission-form")!.hidden = fissionParent.options.length === 0;
  renderEveSurface(surface, host, { body: document.body, clientId: "ghostlight.browser", statusElement: status });
  const campaigns = await fetch("/api/campaigns").then(response => response.json());
  campaignList.replaceChildren(...campaigns.campaigns.map((item: any) => {
    const button = node("button", `${item.selected ? "●" : "○"} ${item.name} · revision ${item.revision}`);
    button.type = "button";
    button.disabled = Boolean(item.selected);
    if (!item.selected) button.addEventListener("click", async () => { await compilerPost(`/api/campaigns/select/${item.id}`, {}); await refresh(); });
    return button;
  }));
}

async function compilerPost(path: string, body: unknown) {
  status.textContent = "Retrieving evidence and compiling…";
  const response = await fetch(path, { method: "POST", headers: { "content-type": "application/json" }, body: JSON.stringify(body) });
  const value = response.headers.get("content-type")?.includes("json") ? await response.json() : await response.text();
  if (!response.ok) { status.textContent = typeof value === "string" ? value : "Compilation failed"; throw new Error(status.textContent); }
  return value;
}

function showCards(items: any[], action: string, choose: (item: any) => void) {
  const cards = node("div", undefined, "cards");
  items.forEach((item, index) => {
    const card = node("article", undefined, "card");
    card.append(node("h3", item.title ?? item.name), node("p", item.player_hook ?? item.premise));
    if (item.era) card.append(node("p", `${item.era} · ${item.place} · ${item.pressure}`));
    const button = node("button", action);
    button.type = "button";
    button.addEventListener("click", () => choose(items[index]));
    card.append(button);
    cards.append(card);
  });
  compilerResults.replaceChildren(cards);
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
  const coverage = Array.isArray(preview.evidence_coverage) ? preview.evidence_coverage : [];
  const evidenceCounts = coverage.reduce((counts: Record<string, number>, item: any) => {
    const lane = String(item?.lane ?? "excluded");
    counts[lane] = (counts[lane] ?? 0) + 1;
    return counts;
  }, {});
  const card = node("article", undefined, "card");
  card.append(
    node("h3", preview.title),
    node("p", preview.opening ?? ""),
    node("p", `${preview.locations?.length ?? 0} locations · ${(preview.cast?.length ?? 0) + 1} actors · ${preview.institutions?.length ?? 0} institutions`),
    node("p", preview.gaps.length ? `Material gaps: ${preview.gaps.join("; ")}` : "No declared material gaps.", "warning"),
    node("p", `Branch assumptions: ${preview.branch_assumptions.join("; ") || "none"}`),
    node("p", coverage.length
      ? `Evidence use: ${evidenceCounts.direct_seed ?? 0} direct · ${evidenceCounts.setting_background ?? 0} background · ${evidenceCounts.excluded ?? 0} excluded`
      : "Evidence use: no retrieved sources were admitted to this seed.", "quiet"),
  );
  const approve = node("button", "Approve and enter");
  approve.type = "button";
  approve.addEventListener("click", async () => { await compilerPost(`/api/compiler/approve/${result.preview_id}`, {}); await refresh(); });
  card.append(approve);
  compilerResults.replaceChildren(card);
  status.textContent = "Preview compiled. Nothing has entered world state yet.";
}

async function send(command: unknown) { status.textContent = "The world is considering the command…"; const response = await fetch("/api/command", { method: "POST", headers: { "content-type": "application/json" }, body: JSON.stringify(command) }); const body = await response.json(); renderCommandReceipt(body); if (!response.ok) { status.textContent = body.error ?? "The command was refused."; await refresh(); return body; } await refresh(); return body; }
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
  showSummary(response.ok ? "Provider concurrency updated" : "Provider concurrency refused", [response.ok ? `Parallel requests: ${body.receipt?.provider_parallelism ?? body.provider_parallelism ?? "updated"}` : body.error ?? "The operator limit was refused."]);
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
destinationForm.addEventListener("submit",async event=>{event.preventDefault();const result=await compilerPost("/api/compiler/destination",Object.fromEntries(new FormData(destinationForm)));const preview=result.preview;showSummary("Destination preview",[...(preview.locations ?? []).map((location:any)=>location.name),...(preview.gaps ?? []).map((gap:string)=>`Material gap: ${gap}`)]);status.textContent="Destination preview compiled; topology is unchanged until approval.";const button=node("button","Approve destination");button.type="button";button.addEventListener("click",async()=>{await compilerPost(`/api/compiler/destination/approve/${result.preview_id}`,{});button.remove();await refresh();});destinationForm.append(button);});
document.querySelector<HTMLFormElement>("#fission-form")!.addEventListener("submit", async event => {
  event.preventDefault();
  const data = new FormData(event.currentTarget as HTMLFormElement);
  const result = await compilerPost("/api/compiler/gestalt/fission", {
    parent_gestalt_id: data.get("parent_gestalt_id"),
    partition_axis: data.get("partition_axis"),
    requested_partition_values: String(data.get("requested_partition_values") ?? "").split(",").map(value => value.trim()).filter(Boolean),
    reason: data.get("reason"),
  });
  showSummary("Population fission preview", [
    ...(result.preview?.children ?? []).map((child: any) => `${child.name} · ${child.id}`),
    ...(result.preview?.gaps ?? []).map((gap: string) => `Material gap: ${gap}`),
  ]);
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
