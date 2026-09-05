// Spec test 16: the hygiene the query options must carry, asserted against the
// same function `main.ts` uses. No credential, no network, no query.

import assert from "node:assert/strict";
import test from "node:test";
import { QuerySession, lowerQuery, SidecarFault } from "../src/main.ts";
import type { SidecarFrame } from "../src/frames.ts";

const EMPTY_SCHEMA = JSON.stringify({
  type: "object",
  additionalProperties: false,
  required: [],
  properties: {},
});

function queryFrame(
  overrides: Partial<Extract<SidecarFrame, { kind: "query" }>> = {},
): Extract<SidecarFrame, { kind: "query" }> {
  return {
    kind: "query",
    query_id: 1,
    model: "claude-opus-5",
    instructions: "Author the shortfall.",
    prompt: "Answer the deficit.",
    transcript: [],
    tools: [
      { name: "submit", description: "Submit the draft.", parameters_json: EMPTY_SCHEMA },
      { name: "record_gap", description: "Record a gap.", parameters_json: EMPTY_SCHEMA },
    ],
    effort: "medium",
    max_output_tokens: 4000,
    turn_cap: 24,
    ...overrides,
  };
}

function session(frame: Extract<SidecarFrame, { kind: "query" }>): QuerySession {
  return new QuerySession(frame.query_id, () => {});
}

test("lowers a query to SDK options that load nothing ambient", () => {
  const frame = queryFrame();
  const { prompt, options } = lowerQuery(frame, session(frame));
  const raw = options as unknown as Record<string, unknown>;

  assert.equal(prompt, frame.prompt, "a first round carried a transcript header");
  assert.deepEqual(raw.systemPrompt, {
    type: "custom",
    prompt: frame.instructions,
  });
  assert.deepEqual(raw.settingSources, [], "settings sources were not emptied");
  assert.deepEqual(raw.tools, [], "built-in tools were not stripped");
  assert.equal(raw.strictMcpConfig, true);
  assert.equal(raw.persistSession, false, "the caller's conversation would persist");
  assert.equal(raw.includePartialMessages, false);
  assert.equal(raw.maxTurns, frame.turn_cap);
  assert.equal(raw.model, frame.model);
  assert.equal(raw.effort, frame.effort);
  assert.deepEqual(raw.allowedTools, ["mcp__ghostlight__*"]);

  const servers = raw.mcpServers as Record<string, Record<string, unknown>>;
  assert.ok(servers.ghostlight, "the ghostlight MCP server was not registered");
  const registered = servers.ghostlight.instance ?? servers.ghostlight;
  assert.ok(registered, "the in-process server carries no instance");
});

test("a request with no tools registers no MCP server", () => {
  const frame = queryFrame({ tools: [], turn_cap: 1 });
  const { options } = lowerQuery(frame, session(frame));
  const raw = options as unknown as Record<string, unknown>;
  assert.equal(raw.mcpServers, undefined);
  assert.equal(raw.allowedTools, undefined);
  assert.equal(raw.maxTurns, 1);
});

test("a prior round's transcript is folded under one fixed header", () => {
  const frame = queryFrame({
    transcript: ["assistant: thinking", "tool result: ok"],
  });
  const { prompt } = lowerQuery(frame, session(frame));
  assert.equal(
    prompt,
    "Answer the deficit.\n\nEarlier turns in this same request, as they happened:\nassistant: thinking\ntool result: ok",
  );
});

test("a turn cap outside the lane budgets is refused", () => {
  for (const turn_cap of [0, 25]) {
    const frame = queryFrame({ turn_cap });
    assert.throws(
      () => lowerQuery(frame, session(frame)),
      (error: unknown) =>
        error instanceof SidecarFault && error.reason === "turn_cap_refused",
    );
  }
});

test("a tool schema outside the emitted grammar refuses registration", () => {
  const frame = queryFrame({
    tools: [
      {
        name: "submit",
        description: "Submit the draft.",
        parameters_json: JSON.stringify({ oneOf: [{ type: "string" }] }),
      },
    ],
  });
  assert.throws(
    () => lowerQuery(frame, session(frame)),
    (error: unknown) =>
      error instanceof SidecarFault && error.reason === "tool_registration_failed",
  );
});
