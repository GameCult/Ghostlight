import assert from "node:assert/strict";
import { afterEach, test } from "node:test";
import {
  heimdallAccessBrowserAdapter,
  readHeimdallBrowserReturn,
  resumeHeimdallAccess,
} from "../src/heimdall-access-adapter.ts";

const attemptKey = "gamecult.heimdall.access.attempt";
const originalGlobals = {
  history: globalThis.history,
  sessionStorage: globalThis.sessionStorage,
  window: globalThis.window,
};

afterEach(() => {
  restoreGlobal("history", originalGlobals.history);
  restoreGlobal("sessionStorage", originalGlobals.sessionStorage);
  restoreGlobal("window", originalGlobals.window);
});

test("browser return parser accepts only a matching completion witness", () => {
  const handle = "d0a9c1e7-89ac-46f9-8c85-c2484ed18f76";
  assert.deepEqual(readHeimdallBrowserReturn(returnUrl(handle), "ghostlight"), { status: "success", handle });
  assert.equal(readHeimdallBrowserReturn("https://yggdrasil.gamecult.org/ghostlight/", "ghostlight"), undefined);
  assert.equal(readHeimdallBrowserReturn(returnUrl(handle), "another-app")?.status, "error");
  assert.equal(readHeimdallBrowserReturn(returnUrl(handle, "different"), "ghostlight")?.status, "error");
});

test("fresh browser context redeems the URL witness and scrubs it", async () => {
  const handle = "0be30b8c-b599-4920-b8b8-733f5e7bc275";
  const storage = memoryStorage();
  const replaced = [];
  installBrowserGlobals(returnUrl(handle), storage, replaced);
  const completed = [];

  assert.equal(await resumeHeimdallAccess({ complete: async value => completed.push(value) }, { appSlug: "ghostlight" }), true);
  assert.deepEqual(completed, [handle]);
  assert.equal(storage.getItem(attemptKey), handle);
  assert.deepEqual(replaced, ["/ghostlight/"]);

  await heimdallAccessBrowserAdapter.consumeCommandResult({
    schemaId: "heimdall.auth_completion_status.v1",
    payload: { status: "authenticated" },
  });
  assert.equal(storage.getItem(attemptKey), null);
});

test("conflicting stored and returned attempts never reach completion", async () => {
  const storage = memoryStorage([[attemptKey, "stored-handle"]]);
  installBrowserGlobals(returnUrl("returned-handle"), storage, []);
  let completed = false;

  await assert.rejects(
    resumeHeimdallAccess({ complete: async () => { completed = true; } }, { appSlug: "ghostlight" }),
    /other than the one this browser started/,
  );
  assert.equal(completed, false);
  assert.equal(storage.getItem(attemptKey), null);
});

function installBrowserGlobals(href, storage, replaced) {
  Object.defineProperty(globalThis, "sessionStorage", { configurable: true, value: storage });
  Object.defineProperty(globalThis, "history", {
    configurable: true,
    value: { replaceState: (_state, _title, url) => replaced.push(url) },
  });
  Object.defineProperty(globalThis, "window", {
    configurable: true,
    value: { location: { href, pathname: "/ghostlight/", search: "" } },
  });
}

function restoreGlobal(name, value) {
  if (value === undefined) delete globalThis[name];
  else Object.defineProperty(globalThis, name, { configurable: true, value });
}

function returnUrl(attemptId, completionCode = attemptId) {
  const parameters = new URLSearchParams({
    heimdall_status: "success",
    heimdall_provider: "discord",
    heimdall_handoff_kind: "browser_completion",
    heimdall_app_slug: "ghostlight",
    heimdall_mode: "sign_in",
    heimdall_attempt_id: attemptId,
    heimdall_completion_code: completionCode,
  });
  return `https://yggdrasil.gamecult.org/ghostlight/#${parameters}`;
}

function memoryStorage(initial = []) {
  const values = new Map(initial);
  return {
    get length() { return values.size; },
    clear: () => values.clear(),
    getItem: key => values.get(key) ?? null,
    key: index => [...values.keys()][index] ?? null,
    removeItem: key => { values.delete(key); },
    setItem: (key, value) => { values.set(key, value); },
  };
}
