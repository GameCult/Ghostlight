// Ghostlight's Claude Agent SDK sidecar.
//
// One query at a time, one child process for many queries. Every tool call is
// forwarded to Rust and answered by Rust: this process computes no tool result,
// reads no credential, logs no prompt, argument, or result, and writes nothing
// outside its own stdout.
//
// Operator prerequisites, none of which Ghostlight performs: install the Claude
// Code CLI, sign in with `/login` or mint a token with `claude setup-token`,
// verify with one plain `claude -p` query, then `npm --prefix sidecar/claude-sdk
// install` and `npm run sdk:build`. The SDK finds that login on its own through
// the ambient environment. Nothing in this repository reads, copies, forwards,
// or logs the credential.
//
// Runtime binding: `GHOSTLIGHT_SDK_SIDECAR` is this file's built path
// (`sidecar/claude-sdk/dist/main.js`) and `GHOSTLIGHT_SDK_MODEL_PREFIX`
// (default `claude`) is the model-name prefix that routes a lane here.

import {
  createSdkMcpServer,
  query,
  tool,
  type Options,
} from "@anthropic-ai/claude-agent-sdk";
import {
  FrameReader,
  encodeFrame,
  type SdkModelUsage,
  type SdkResultMaterial,
  type SidecarEvent,
  type SidecarFaultReason,
  type SidecarFrame,
  type SidecarTool,
} from "./frames.ts";
import { toolRawShape } from "./schema.ts";
import { resolve } from "node:path";
import { pathToFileURL } from "node:url";

const SERVER_NAME = "ghostlight";
const QUALIFIED_PREFIX = `mcp__${SERVER_NAME}__`;
/** The header a prior round's rendered transcript is folded under. */
const TRANSCRIPT_HEADER =
  "Earlier turns in this same request, as they happened:";
/** No deadline tighter than the Rust side's own response timeout. */
const RENDEZVOUS_TIMEOUT_MS = 900_000;
/** The largest round budget any Ghostlight lane carries. */
const MAX_TURN_CAP = 24;

export class SidecarFault extends Error {
  readonly reason: SidecarFaultReason;

  constructor(reason: SidecarFaultReason, detail: string) {
    super(detail);
    this.reason = reason;
  }
}

interface Dispatch {
  call_id: string;
  name: string;
  arguments: string;
}

/** One in-flight query's rendezvous with Rust. */
export class QuerySession {
  readonly dispatches: Dispatch[] = [];
  readonly queryId: number;
  readonly #pending = new Map<string, (output: string) => void>();
  readonly #write: (frame: SidecarFrame) => void;
  #nextCall = 0;

  constructor(queryId: number, write: (frame: SidecarFrame) => void) {
    this.queryId = queryId;
    this.#write = write;
  }

  /** Sends one call to Rust and resolves when Rust answers it. */
  async ask(name: string, args: unknown): Promise<string> {
    const call_id = `c${this.#nextCall++}`;
    const argumentsJson = JSON.stringify(args ?? {});
    this.dispatches.push({ call_id, name, arguments: argumentsJson });
    const answer = new Promise<string>((resolve) => {
      this.#pending.set(call_id, resolve);
    });
    this.#write({
      kind: "tool_call",
      query_id: this.queryId,
      call_id,
      name,
      arguments: argumentsJson,
    });
    return answer;
  }

  answer(call_id: string, output: string): void {
    const resolve = this.#pending.get(call_id);
    if (!resolve) {
      throw new SidecarFault(
        "protocol_violation",
        `a tool result arrived for the unissued call ${call_id}`,
      );
    }
    this.#pending.delete(call_id);
    resolve(output);
  }
}

function apiRetryReason(category: unknown): SidecarFaultReason {
  switch (category) {
    case "rate_limit":
      return "rate_limited";
    case "overloaded":
      return "overloaded";
    case "server_error":
      return "server_error";
    case "authentication_failed":
      return "authentication_failed";
    case "oauth_org_not_allowed":
      return "org_not_allowed";
    case "billing_error":
      return "billing_error";
    case "invalid_request":
      return "invalid_request";
    case "model_not_found":
      return "model_not_found";
    case "max_output_tokens":
      return "max_output_tokens";
    default:
      return "unknown";
  }
}

function resultReason(subtype: string, apiErrorStatus: unknown): SidecarFaultReason {
  if (typeof apiErrorStatus === "number") {
    if (apiErrorStatus === 429) {
      return "rate_limited";
    }
    if (apiErrorStatus >= 500) {
      return "server_error";
    }
  }
  switch (subtype) {
    case "error_max_budget_usd":
      return "max_budget_usd";
    case "error_during_execution":
    case "error_max_structured_output_retries":
      return "execution_error";
    default:
      return "unknown";
  }
}

function usageRows(modelUsage: unknown): SdkModelUsage[] {
  if (typeof modelUsage !== "object" || modelUsage === null) {
    return [];
  }
  return Object.entries(modelUsage as Record<string, Record<string, unknown>>).map(
    ([model, row]) => ({
      model,
      input_tokens: Number(row?.inputTokens ?? row?.input_tokens ?? 0),
      output_tokens: Number(row?.outputTokens ?? row?.output_tokens ?? 0),
      cache_read_input_tokens: Number(
        row?.cacheReadInputTokens ?? row?.cache_read_input_tokens ?? 0,
      ),
      cache_creation_input_tokens: Number(
        row?.cacheCreationInputTokens ?? row?.cache_creation_input_tokens ?? 0,
      ),
    }),
  );
}

/**
 * The `query()` options one frame lowers to. Exported so a test can assert the
 * hygiene — no CLAUDE.md, no settings, no built-in tools, no persisted session
 * — against a stub instead of a live query.
 */
export function lowerQuery(
  frame: Extract<SidecarFrame, { kind: "query" }>,
  session: QuerySession,
): { prompt: string; options: Options } {
  if (frame.turn_cap < 1 || frame.turn_cap > MAX_TURN_CAP) {
    throw new SidecarFault(
      "turn_cap_refused",
      `turn cap ${frame.turn_cap} is outside 1..=${MAX_TURN_CAP}`,
    );
  }
  const tools = frame.tools.map((entry: SidecarTool) => {
    let shape;
    try {
      shape = toolRawShape(entry.parameters_json, entry.name);
    } catch (error) {
      throw new SidecarFault("tool_registration_failed", String(error));
    }
    return tool(
      entry.name,
      entry.description,
      shape,
      async (args: unknown) => ({
        content: [
          { type: "text" as const, text: await session.ask(entry.name, args) },
        ],
      }),
      { alwaysLoad: true },
    );
  });
  const prompt = frame.transcript.length
    ? `${frame.prompt}\n\n${TRANSCRIPT_HEADER}\n${frame.transcript.join("\n")}`
    : frame.prompt;
  const options: Record<string, unknown> = {
    systemPrompt: { type: "custom", prompt: frame.instructions },
    // No CLAUDE.md, no settings.json, no output styles, no built-in tools.
    settingSources: [],
    tools: [],
    strictMcpConfig: true,
    model: frame.model,
    maxTurns: frame.turn_cap,
    includePartialMessages: false,
    persistSession: false,
  };
  if (frame.effort) {
    options.effort = frame.effort;
  }
  if (tools.length) {
    options.mcpServers = {
      [SERVER_NAME]: createSdkMcpServer({
        name: SERVER_NAME,
        tools,
        alwaysLoad: true,
        timeout: RENDEZVOUS_TIMEOUT_MS,
      }),
    };
    options.allowedTools = [`${QUALIFIED_PREFIX}*`];
  }
  return { prompt, options: options as unknown as Options };
}

function strip(name: string): string {
  return name.startsWith(QUALIFIED_PREFIX)
    ? name.slice(QUALIFIED_PREFIX.length)
    : name;
}

/**
 * Walks the assistant messages in order and pairs each registered tool_use
 * block with the dispatch it caused, so the ids Rust answered and the ids
 * reported here are the same ids.
 */
export function assembleEvents(
  messages: { content: unknown }[],
  registered: Set<string>,
  dispatches: Dispatch[],
): SidecarEvent[] {
  const events: SidecarEvent[] = [];
  let taken = 0;
  let undispatched = 0;
  for (const message of messages) {
    const blocks = Array.isArray(message.content) ? message.content : [];
    for (const block of blocks as Record<string, unknown>[]) {
      if (block.type === "text" && typeof block.text === "string") {
        events.push({ kind: "text", text: block.text });
        continue;
      }
      if (block.type !== "tool_use") {
        continue;
      }
      const name = strip(String(block.name ?? ""));
      if (registered.has(name)) {
        const dispatch = dispatches[taken++];
        if (!dispatch || dispatch.name !== name) {
          throw new SidecarFault(
            "protocol_violation",
            `a registered tool_use block for ${name} has no matching dispatch`,
          );
        }
        events.push({ kind: "tool_call", ...dispatch, dispatched: true });
        continue;
      }
      events.push({
        kind: "tool_call",
        call_id: `u${undispatched++}`,
        name,
        arguments: JSON.stringify(block.input ?? {}),
        dispatched: false,
      });
    }
  }
  if (taken !== dispatches.length) {
    throw new SidecarFault(
      "protocol_violation",
      "more tool calls were dispatched than the assistant messages report",
    );
  }
  return events;
}

async function runQuery(
  frame: Extract<SidecarFrame, { kind: "query" }>,
  write: (frame: SidecarFrame) => void,
  session: QuerySession,
): Promise<void> {
  const registered = new Set(frame.tools.map((entry) => entry.name));
  const { prompt, options } = lowerQuery(frame, session);
  const assistant: { content: unknown }[] = [];
  const material: SdkResultMaterial = {
    session_id: "",
    result_uuid: "",
    subtype: "",
    stop_reason: null,
    num_turns: 0,
    assistant_message_uuids: [],
    assistant_request_ids: [],
    usage: [],
    total_cost_usd_estimate: "0",
  };
  let sawResult = false;
  const stream = query({ prompt, options });
  try {
    for await (const message of stream as AsyncIterable<Record<string, unknown>>) {
      if (message.type === "assistant") {
        const inner = message.message as Record<string, unknown> | undefined;
        assistant.push({ content: inner?.content });
        if (typeof message.uuid === "string") {
          material.assistant_message_uuids.push(message.uuid);
        }
        if (typeof message.request_id === "string") {
          material.assistant_request_ids.push(message.request_id);
        }
        continue;
      }
      if (message.type === "system" && message.subtype === "api_retry") {
        // The harness owns its own retries; only an exhausted one reaches a
        // result, so this is recorded and not raised.
        continue;
      }
      if (message.type !== "result") {
        continue;
      }
      sawResult = true;
      const subtype = String(message.subtype ?? "");
      material.session_id = String(message.session_id ?? "");
      material.result_uuid = String(message.uuid ?? "");
      material.subtype = subtype;
      material.stop_reason =
        typeof message.stop_reason === "string" ? message.stop_reason : null;
      material.num_turns = Number(message.num_turns ?? 0);
      material.usage = usageRows(message.modelUsage);
      material.total_cost_usd_estimate = String(message.total_cost_usd ?? 0);
      // `error_max_turns` is not a fault: the cap is the lane's own remaining
      // round budget, so reaching it means the model spent turns the evaluator
      // would also have allowed. The lane sees one ordinary round.
      if (subtype !== "success" && subtype !== "error_max_turns") {
        const errors = Array.isArray(message.errors)
          ? (message.errors as unknown[]).map(String).join("; ")
          : "";
        const category = Array.isArray(message.errors)
          ? undefined
          : (message as Record<string, unknown>).error;
        const reason =
          category === undefined
            ? resultReason(subtype, message.api_error_status)
            : apiRetryReason(category);
        throw new SidecarFault(reason, `${subtype}: ${errors}`);
      }
    }
  } catch (error) {
    // The TS SDK throws after yielding an error result. A result already
    // collected for `error_max_turns` is the normal path, so only an
    // unaccounted throw becomes a fault.
    if (error instanceof SidecarFault) {
      throw error;
    }
    if (!sawResult) {
      throw new SidecarFault("execution_error", String(error));
    }
  }
  if (!sawResult) {
    throw new SidecarFault("execution_error", "the SDK query produced no result");
  }
  write({
    kind: "output",
    query_id: frame.query_id,
    events: assembleEvents(assistant, registered, session.dispatches),
    receipt: material,
  });
}

async function main(): Promise<void> {
  const reader = new FrameReader();
  const write = (frame: SidecarFrame) => {
    process.stdout.write(encodeFrame(frame));
  };
  let live: QuerySession | null = null;
  let running: Promise<void> = Promise.resolve();

  for await (const chunk of process.stdin) {
    reader.push(chunk as Uint8Array);
    for (;;) {
      let frame: SidecarFrame | null;
      try {
        frame = reader.next();
      } catch (error) {
        write({
          kind: "fault",
          query_id: live?.queryId ?? 0,
          reason: "protocol_violation",
          detail: String(error),
        });
        return;
      }
      if (!frame) {
        break;
      }
      if (frame.kind === "query") {
        if (live) {
          write({
            kind: "fault",
            query_id: frame.query_id,
            reason: "protocol_violation",
            detail: "a second query arrived while one was in flight",
          });
          continue;
        }
        const opening = frame;
        const session = new QuerySession(opening.query_id, write);
        live = session;
        running = (async () => {
          try {
            await runQuery(opening, write, session);
          } catch (error) {
            const fault =
              error instanceof SidecarFault
                ? error
                : new SidecarFault("execution_error", String(error));
            write({
              kind: "fault",
              query_id: opening.query_id,
              reason: fault.reason,
              detail: fault.message,
            });
          } finally {
            live = null;
          }
        })();
        void running;
        continue;
      }
      if (frame.kind === "tool_result") {
        if (!live || live.queryId !== frame.query_id) {
          write({
            kind: "fault",
            query_id: frame.query_id,
            reason: "protocol_violation",
            detail: "a tool result arrived outside its query",
          });
          continue;
        }
        try {
          live.answer(frame.call_id, frame.output);
        } catch (error) {
          const fault =
            error instanceof SidecarFault
              ? error
              : new SidecarFault("protocol_violation", String(error));
          write({
            kind: "fault",
            query_id: frame.query_id,
            reason: fault.reason,
            detail: fault.message,
          });
        }
        continue;
      }
      write({
        kind: "fault",
        query_id: 0,
        reason: "protocol_violation",
        detail: `frame kind ${frame.kind} is not one this sidecar accepts`,
      });
    }
  }
  await running;
}

// Only run the loop when this file is the entry point, so the tests can import
// its pure halves without starting a stdin reader.
if (
  process.argv[1] &&
  import.meta.url === pathToFileURL(resolve(process.argv[1])).href
) {
  await main();
}
