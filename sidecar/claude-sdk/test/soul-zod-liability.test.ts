// One validator, both transports. These assert that the schema the sidecar
// registers judges nothing: an argument Ghostlight's evaluators would record as
// a gap travels through the real in-process MCP server unstripped and reaches
// the handler, so Rust's decoder is the only thing that can refuse it.
//
// Established end to end against the real SDK, with no credential and no
// network. Also asserts the sidecar names no credential and reads no
// environment.

import assert from "node:assert/strict";
import { readFileSync, readdirSync } from "node:fs";
import { fileURLToPath } from "node:url";
import test from "node:test";
import { createSdkMcpServer, tool } from "@anthropic-ai/claude-agent-sdk";
import { toolInputSchema } from "../src/schema.ts";
import { assembleEvents, SidecarFault } from "../src/main.ts";

const emitted: { name: string; parameters_json: string }[] = JSON.parse(
  readFileSync(fileURLToPath(new URL("./schemas.json", import.meta.url)), "utf8"),
);

function schemaFor(name: string): string {
  const entry = emitted.find((candidate) => candidate.name === name);
  assert.ok(entry, `the emitted fixture carries no ${name}`);
  return entry.parameters_json;
}

test("the registered schema keeps an extra property instead of stripping it", () => {
  const gap = toolInputSchema(schemaFor("record_gap"), "record_gap");
  const admitted = gap.safeParse({ detail: "no route", extra: true });
  assert.equal(admitted.success, true);
  assert.deepEqual(
    admitted.success && admitted.data,
    { detail: "no route", extra: true },
    "an extra property was stripped before Rust could refuse it",
  );

  // A nested object keeps its own, so the emitter's depth does not matter.
  const declare = toolInputSchema(schemaFor("bind"), "bind");
  const deep = declare.safeParse({
    subject: { ref: "draft", value: "x", extra: true },
  });
  assert.equal(deep.success, true);
  assert.deepEqual(deep.success && (deep.data as { subject: unknown }).subject, {
    ref: "draft",
    value: "x",
    extra: true,
  });
});

test("the registered schema admits a missing or mistyped required property", () => {
  const gap = toolInputSchema(schemaFor("record_gap"), "record_gap");
  const absent = gap.safeParse({});
  assert.equal(absent.success, true);
  assert.deepEqual(absent.success && absent.data, {});

  // A mistyped value is the one thing a property's catch cannot carry through:
  // it reaches Rust as an absent field, which the decoder reports the way it
  // reports any other missing field, rather than as a call the SDK refused.
  const mistyped = gap.safeParse({ detail: 5 });
  assert.equal(mistyped.success, true);
  assert.equal(
    mistyped.success && (mistyped.data as { detail?: unknown }).detail,
    undefined,
  );

  const subject = toolInputSchema(schemaFor("declare_subject"), "declare_subject");
  assert.equal(
    subject.safeParse({ handle: "smith" }).success,
    true,
    "a partly filled call was refused here instead of in Rust",
  );
});

test("the SDK dispatches a call Rust would refuse and never answers it itself", async () => {
  // The middle link, against the real in-process MCP server the sidecar
  // registers. No credential, no network, no `query()`.
  let mcp: { Client: new (info: { name: string; version: string }) => never };
  let memory: { InMemoryTransport: { createLinkedPair(): [never, never] } };
  try {
    mcp = await import("@modelcontextprotocol/sdk/client/index.js");
    memory = await import("@modelcontextprotocol/sdk/inMemory.js");
  } catch {
    // The MCP client is the SDK's own transitive dependency, not this
    // package's. If a future install stops hoisting it, the assertions either
    // side of this test still stand on their own.
    return;
  }
  const seen: unknown[] = [];
  const handle = tool(
    "record_gap",
    "Record a gap.",
    toolInputSchema(schemaFor("record_gap"), "record_gap") as unknown as Parameters<
      typeof tool
    >[2],
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
    listTools(): Promise<{ tools: { inputSchema: Record<string, unknown> }[] }>;
    callTool(request: { name: string; arguments: unknown }): Promise<{
      isError?: boolean;
      content: { text: string }[];
    }>;
    close(): Promise<void>;
  };
  await client.connect(clientSide);
  try {
    // The model still sees the emitted parameters. Looseness is what the
    // server judges arguments by, not what it advertises.
    const advertised = (await client.listTools()).tools[0]!.inputSchema;
    assert.equal(advertised.type, "object");
    assert.deepEqual(Object.keys(advertised.properties as object), ["detail"]);
    assert.deepEqual(advertised.required, ["detail"]);

    // An extra key reaches the handler with the key present.
    const extra = await client.callTool({
      name: "record_gap",
      arguments: { detail: "no route", extra: true },
    });
    assert.notEqual(extra.isError, true, "an extra property was refused");
    assert.deepEqual(seen.at(-1), { detail: "no route", extra: true });

    // A missing required property reaches the handler too.
    const missing = await client.callTool({ name: "record_gap", arguments: {} });
    assert.notEqual(missing.isError, true, "a missing property was refused");
    assert.deepEqual(seen.at(-1), {});

    assert.equal(seen.length, 2, "a call was answered without reaching the handler");
  } finally {
    await client.close();
  }
});

test("a registered tool_use block with no dispatch is a protocol violation", () => {
  // No argument reaches this any more. What still does is a call never offered
  // to the handler, or an assistant stream and a dispatch log that disagree.
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
