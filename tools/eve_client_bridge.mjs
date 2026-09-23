// Cut 10 (PA.f149): drives the real vendored Eve browser lowering
// (`vendor/eve/packages/eve-browser-lowering`, submodule pin unmoved) over a
// surface document Dungeon actually served, so the command intent this
// prints is what a real client sends -- never a hand-built payload asserting
// its own shape is correct.
//
// stdin: {"surface": <the surface document>, "steps": [{"set": {node_id: text}}, {"click": node_id}]}
// stdout: one JSON line per intent the click(s) produced, in order.
//
// `set` types into a control by its Eve node id (the same id
// `eve.rs`'s own `json!({"id": ...})` calls carry), by finding the rendered
// element via `data-eve-node-id` (assignId) and dispatching a real `input`
// event, exactly as a user's keystroke would. `click` clicks a
// `control.button` node the same way. jsdom is resolved from the vendored
// package's own devDependencies (its own test suite uses the identical
// pattern in `test/host-isolation.test.mjs`) rather than adding a second copy
// to this repo.
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";
import { createRequire } from "node:module";

const here = dirname(fileURLToPath(import.meta.url));
const loweringRoot = join(here, "..", "vendor", "eve", "packages", "eve-browser-lowering");
const vendorRequire = createRequire(join(loweringRoot, "package.json"));
const { JSDOM } = vendorRequire("jsdom");
const { EveBrowserDraftStore, renderEveSurface } = await import(
  "file://" + join(loweringRoot, "dist", "index.js").replace(/\\/g, "/")
);

function installDom() {
  const dom = new JSDOM("<!doctype html><html><body><div id='host'></div></body></html>", {
    url: "https://eve.bridge.test/",
    pretendToBeVisual: true,
  });
  const names = [
    "document", "window", "Element", "HTMLElement", "HTMLButtonElement",
    "HTMLInputElement", "HTMLSelectElement", "HTMLOptionElement",
    "HTMLTextAreaElement", "HTMLImageElement", "Event", "CustomEvent",
  ];
  for (const name of names) globalThis[name] = dom.window[name];
  return dom;
}

function cssEscape(value) {
  return value.replace(/[^a-zA-Z0-9_-]/g, char => `\\${char}`);
}

async function readStdin() {
  return new Promise((resolve, reject) => {
    let data = "";
    process.stdin.setEncoding("utf8");
    process.stdin.on("data", chunk => { data += chunk; });
    process.stdin.on("end", () => resolve(data));
    process.stdin.on("error", reject);
  });
}

async function main() {
  const { surface, steps, provider } = JSON.parse(await readStdin());

  const dom = installDom();
  const host = dom.window.document.getElementById("host");
  const intents = [];
  const draftStore = new EveBrowserDraftStore();
  const options = {
    draftStore,
    clientId: "eve.bridge.test",
    commandSink: intent => { intents.push(intent); },
    // `commandBoundary`/`receiptSchema` are not carried on the surface
    // document itself -- a real client learns them from the provider's own
    // CultMesh advertisement (`mesh.rs::provider_advertisement`,
    // `surfaces[].worldInteraction`), fetched separately. The caller passes
    // that same shape through here.
    provider,
  };
  renderEveSurface(surface, host, options);

  for (const step of steps) {
    if (step.set) {
      for (const [nodeId, value] of Object.entries(step.set)) {
        const wrapper = host.querySelector(`[data-eve-node-id="${cssEscape(nodeId)}"]`);
        if (!wrapper) throw new Error(`bridge: no rendered control for node id ${nodeId}`);
        // `assignId` stamps the Eve node id onto the field's own wrapper
        // (`renderEditableControl`'s `<label>`), not the `<input>`/
        // `<textarea>` inside it, so the real editable element is a
        // descendant when the wrapper itself is not one.
        const element = /^(INPUT|TEXTAREA|SELECT)$/.test(wrapper.tagName)
          ? wrapper
          : wrapper.querySelector("input, textarea, select");
        if (!element) throw new Error(`bridge: node id ${nodeId} has no editable control`);
        // PA.f174: a real browser refuses keystrokes into a disabled
        // control outright -- typing into it here without this check would
        // silently capture a value no real client could ever have entered,
        // hiding exactly the gap PA.f167 named untestable.
        if (element.disabled) {
          throw new Error(`bridge: node id ${nodeId}'s own control is disabled; a real client cannot type into it`);
        }
        element.value = value;
        element.dispatchEvent(new dom.window.Event("input", { bubbles: true }));
        element.dispatchEvent(new dom.window.Event("change", { bubbles: true }));
      }
    }
    if (step.click) {
      const element = host.querySelector(`[data-eve-node-id="${cssEscape(step.click)}"]`);
      if (!element) throw new Error(`bridge: no rendered control for node id ${step.click}`);
      element.dispatchEvent(new dom.window.Event("click", { bubbles: true }));
    }
  }

  dom.window.close();
  process.stdout.write(intents.map(intent => JSON.stringify(intent)).join("\n") + "\n");
}

main().catch(error => {
  process.stderr.write(`${error.stack || error}\n`);
  process.exit(1);
});
