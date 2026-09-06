// Soul: characterizes what the SDK's own Zod validation does to a tool call
// whose arguments Ghostlight's evaluators would have recorded as a gap, and
// asserts the sidecar names no credential and reads no environment.
//
// Nothing here proposes a fix. The chain is established end to end against the
// real SDK, with no credential and no network, so the divergence from the
// connector path is a recorded fact rather than a guess.

import assert from "node:assert/strict";
import { readFileSync, readdirSync } from "node:fs";
import { fileURLToPath } from "node:url";
import test from "node:test";
import { z } from "zod";
import { createSdkMcpServer, tool } from "@anthropic-ai/claude-agent-sdk";
import { toolRawShape } from "../src/schema.ts";
import { assembleEvents, SidecarFault } from "../src/main.ts";

const emitted: { name: string; parameters_json: string }[] = JSON.parse(
  readFileSync(fileURLToPath(new URL("./schemas.json", import.meta.url)), "utf8"),
);

function schemaFor(name: string): string {
  const entry = emitted.find((candidate) => candidate.name === name);
  assert.ok(entry, `the emitted fixture carries no ${name}`);
  return entry.parameters_json;
}

test("the emitted grammar's top level admits an extra property the connector refuses", () => {
  // `rawShape` builds a bare record; only nested objects are `.strict()`. The
  // SDK's `tool()` wraps the top-level record in a plain `z.object`, which
  // strips unknown keys instead of rejecting them — while every emitted schema
  // says `additionalProperties: false` and every Rust decoder carries
  // `deny_unknown_fields`.
  const gap = z.object(toolRawShape(schemaFor("record_gap"), "record_gap"));
  const admitted = gap.safeParse({ detail: "no route", extra: true });
  assert.equal(admitted.success, true, "the top-level object refused an extra property");
  assert.deepEqual(
    admitted.success && admitted.data,
    { detail: "no route" },
    "the extra property was not stripped",
  );

  // Nested objects are closed, so the leak is exactly one level deep.
  const nested = z.object(toolRawShape(schemaFor("speak"), "speak"));
  assert.equal(nested.safeParse({ text: "hold", extra: 1 }).success, true);
  const declare = z.object(toolRawShape(schemaFor("bind"), "bind"));
  const deep = declare.safeParse({ subject: { ref: "draft", value: "x", extra: true } });
  assert.equal(deep.success, false, "a nested object admitted an extra property");
});

test("the emitted grammar refuses a missing or mistyped required property", () => {
  const gap = z.object(toolRawShape(schemaFor("record_gap"), "record_gap"));
  assert.equal(gap.safeParse({}).success, false);
  assert.equal(gap.safeParse({ detail: 5 }).success, false);
  // A deep emitted shape gives a compliant model many more required leaves to
  // miss.
  const subject = z.object(toolRawShape(schemaFor("declare_subject"), "declare_subject"));
  assert.equal(subject.safeParse({ handle: "smith" }).success, false);
});

test("the SDK answers a refused argument itself and never reaches the handler", async () => {
  // The middle link, against the real in-process MCP server the sidecar
  // registers. No credential, no network, no `query()`.
  let mcp: { Client: new (info: { name: string; version: string }) => never };
  let memory: { InMemoryTransport: { createLinkedPair(): [never, never] } };
  try {
    mcp = await import("@modelcontextprotocol/sdk/client/index.js");
    memory = await import("@modelcontextprotocol/sdk/inMemory.js");
  } catch {
    // The MCP client is the SDK's own transitive dependency, not this
    // package's. If a future install stops hoisting it, the two assertions
    // either side of this test still stand on their own.
    return;
  }
  const seen: unknown[] = [];
  const handle = tool(
    "record_gap",
    "Record a gap.",
    toolRawShape(schemaFor("record_gap"), "record_gap"),
    async (args: unknown) => {
      seen.push(args);
      return { content: [{ type: "text" as const, text: "gap recorded" }] };
    },
    { alwaysLoad: true },
  );
  const server = createSdkMcpServer({ name: "ghostlight", tools: [handle], alwaysLoad: true });
  const [clientSide, serverSide] = memory.InMemoryTransport.createLinkedPair();
  const instance = (server as unknown as { instance: { connect(t: unknown): Promise<void> } })
    .instance;
  await instance.connect(serverSide);
  const client = new mcp.Client({ name: "ghostlight-soul-probe", version: "0" }) as unknown as {
    connect(t: unknown): Promise<void>;
    callTool(request: { name: string; arguments: unknown }): Promise<{
      isError?: boolean;
      content: { text: string }[];
    }>;
    close(): Promise<void>;
  };
  await client.connect(clientSide);
  try {
    const stripped = await client.callTool({
      name: "record_gap",
      arguments: { detail: "no route", extra: true },
    });
    assert.notEqual(stripped.isError, true, "an extra property was refused after all");
    assert.deepEqual(
      seen,
      [{ detail: "no route" }],
      "the handler saw arguments other than the stripped ones",
    );

    for (const refused of [{ detail: 5 }, {}]) {
      const answer = await client.callTool({ name: "record_gap", arguments: refused });
      assert.equal(answer.isError, true, `${JSON.stringify(refused)} reached the handler`);
      assert.match(answer.content[0]!.text, /Input validation error/);
    }
    assert.equal(seen.length, 1, "a refused argument still reached the handler");
  } finally {
    await client.close();
  }
});

test("a registered tool_use block with no dispatch is a protocol violation", () => {
  // What the sidecar then sees: the assistant message still carries a
  // `tool_use` block for a registered name, but `session.ask` was never
  // reached, so there is no dispatch to pair with it.
  const registered = new Set(["record_gap"]);
  assert.throws(
    () =>
      assembleEvents(
        [
          {
            content: [
              { type: "tool_use", name: "mcp__ghostlight__record_gap", input: { detail: 5 } },
            ],
          },
        ],
        registered,
        [],
      ),
    (error: unknown) =>
      error instanceof SidecarFault && error.reason === "protocol_violation",
    "a registered tool_use with no dispatch did not raise protocol_violation",
  );

  // The mirror image: a dispatch the assistant messages never report.
  assert.throws(
    () =>
      assembleEvents([{ content: [] }], registered, [
        { call_id: "c0", name: "record_gap", arguments: "{}" },
      ]),
    (error: unknown) =>
      error instanceof SidecarFault && error.reason === "protocol_violation",
  );

  // An unregistered name needs no dispatch and is reported undispatched, which
  // is the path the evaluators already own as a gap.
  assert.deepEqual(
    assembleEvents(
      [{ content: [{ type: "tool_use", name: "speek", input: {} }] }],
      registered,
      [],
    ),
    [
      {
        kind: "tool_call",
        call_id: "u0",
        name: "speek",
        arguments: "{}",
        dispatched: false,
      },
    ],
  );
});

test("no credential name appears anywhere in the sidecar source", () => {
  const forbidden = [
    ".credentials.json",
    "CLAUDE_CODE_OAUTH_TOKEN",
    "ANTHROPIC_API_KEY",
    "ANTHROPIC_AUTH_TOKEN",
    ".claude.json",
    "apiKeyHelper",
    "USERPROFILE",
    "homedir",
    "--bare",
  ];
  const source = fileURLToPath(new URL("../src", import.meta.url));
  for (const name of readdirSync(source)) {
    const text = readFileSync(`${source}/${name}`, "utf8");
    for (const needle of forbidden) {
      assert.equal(text.includes(needle), false, `${name} names ${needle}`);
    }
  }
});

test("the sidecar reads no environment and logs nothing of its own", () => {
  const source = fileURLToPath(new URL("../src", import.meta.url));
  for (const name of readdirSync(source)) {
    const text = readFileSync(`${source}/${name}`, "utf8");
    assert.equal(text.includes("process.env"), false, `${name} reads the environment`);
    assert.equal(text.includes("console."), false, `${name} logs outside the frame protocol`);
    assert.equal(text.includes("process.stderr"), false, `${name} writes to stderr`);
  }
});
