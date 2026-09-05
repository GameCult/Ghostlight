// The wire between Rust and this sidecar: a 4-byte big-endian length prefix
// followed by MessagePack with string keys, matching `rmp_serde::to_vec_named`.
// Rust owns the shapes; this file only encodes and decodes them.

import { decode, encode } from "@msgpack/msgpack";

/** The connector's own frame cap. Rust refuses anything larger on its side too. */
export const MAX_FRAME_BYTES = 1_052_672;

export interface SidecarTool {
  name: string;
  description: string;
  parameters_json: string;
}

export type SidecarEvent =
  | { kind: "text"; text: string }
  | {
      kind: "tool_call";
      call_id: string;
      name: string;
      arguments: string;
      dispatched: boolean;
    };

export interface SdkModelUsage {
  model: string;
  input_tokens: number;
  output_tokens: number;
  cache_read_input_tokens: number;
  cache_creation_input_tokens: number;
}

export interface SdkResultMaterial {
  session_id: string;
  result_uuid: string;
  subtype: string;
  stop_reason: string | null;
  num_turns: number;
  assistant_message_uuids: string[];
  assistant_request_ids: string[];
  usage: SdkModelUsage[];
  total_cost_usd_estimate: string;
}

/**
 * The closed set of failures this sidecar may name. It reports a reason; Rust
 * assigns the disposition, so nothing here can quarantine the world.
 */
export type SidecarFaultReason =
  | "rate_limited"
  | "overloaded"
  | "server_error"
  | "api_timeout"
  | "authentication_failed"
  | "org_not_allowed"
  | "billing_error"
  | "invalid_request"
  | "model_not_found"
  | "max_output_tokens"
  | "max_budget_usd"
  | "execution_error"
  | "unknown"
  | "protocol_violation"
  | "tool_registration_failed"
  | "turn_cap_refused";

export type SidecarFrame =
  | {
      kind: "query";
      query_id: number;
      model: string;
      instructions: string;
      prompt: string;
      transcript: string[];
      tools: SidecarTool[];
      effort: string | null;
      max_output_tokens: number | null;
      turn_cap: number;
    }
  | {
      kind: "tool_call";
      query_id: number;
      call_id: string;
      name: string;
      arguments: string;
    }
  | {
      kind: "tool_result";
      query_id: number;
      call_id: string;
      output: string;
    }
  | {
      kind: "output";
      query_id: number;
      events: SidecarEvent[];
      receipt: SdkResultMaterial;
    }
  | {
      kind: "fault";
      query_id: number;
      reason: SidecarFaultReason;
      detail: string;
    };

export function encodeFrame(frame: SidecarFrame): Uint8Array {
  const body = encode(frame, { useBigInt64: false });
  if (body.length > MAX_FRAME_BYTES) {
    throw new Error(`frame of ${body.length} bytes exceeds ${MAX_FRAME_BYTES}`);
  }
  const framed = new Uint8Array(4 + body.length);
  new DataView(framed.buffer).setUint32(0, body.length, false);
  framed.set(body, 4);
  return framed;
}

export function decodeFrameBody(body: Uint8Array): SidecarFrame {
  const value = decode(body);
  if (typeof value !== "object" || value === null || !("kind" in value)) {
    throw new Error("a sidecar frame decoded to something without a kind");
  }
  return value as SidecarFrame;
}

/** Reassembles frames from an arbitrarily chunked byte stream. */
export class FrameReader {
  #buffer = new Uint8Array(0);

  push(chunk: Uint8Array): void {
    const grown = new Uint8Array(this.#buffer.length + chunk.length);
    grown.set(this.#buffer, 0);
    grown.set(chunk, this.#buffer.length);
    this.#buffer = grown;
  }

  /** The next whole frame, or null while one is still arriving. */
  next(): SidecarFrame | null {
    if (this.#buffer.length < 4) {
      return null;
    }
    const length = new DataView(
      this.#buffer.buffer,
      this.#buffer.byteOffset,
      4,
    ).getUint32(0, false);
    if (length === 0 || length > MAX_FRAME_BYTES) {
      throw new Error(`frame length ${length} is outside 1..=${MAX_FRAME_BYTES}`);
    }
    if (this.#buffer.length < 4 + length) {
      return null;
    }
    const body = this.#buffer.subarray(4, 4 + length);
    const frame = decodeFrameBody(body);
    this.#buffer = this.#buffer.slice(4 + length);
    return frame;
  }
}
