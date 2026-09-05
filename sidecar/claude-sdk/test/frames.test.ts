// Spec test 18: the frame bytes Rust writes decode here, and the bytes this
// side encodes are byte-identical to Rust's. The `.bin` fixtures are written by
// the Rust test `frame_fixtures_match_the_checked_in_sidecar_bytes`.

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import test from "node:test";
import {
  FrameReader,
  MAX_FRAME_BYTES,
  decodeFrameBody,
  encodeFrame,
  type SidecarFrame,
} from "../src/frames.ts";

function fixture(name: string): Uint8Array {
  return new Uint8Array(
    readFileSync(fileURLToPath(new URL(`./frames/${name}.bin`, import.meta.url))),
  );
}

const NAMES = ["query", "tool_call", "tool_result", "output", "fault"];

test("every Rust frame fixture decodes and re-encodes to the same bytes", () => {
  for (const name of NAMES) {
    const bytes = fixture(name);
    const frame = decodeFrameBody(bytes.subarray(4));
    assert.equal(frame.kind, name, `${name} decoded to the wrong kind`);
    assert.deepEqual(
      Array.from(encodeFrame(frame)),
      Array.from(bytes),
      `${name} did not re-encode to Rust's bytes`,
    );
  }
});

test("the query fixture carries the fields the lowering reads", () => {
  const frame = decodeFrameBody(fixture("query").subarray(4)) as Extract<
    SidecarFrame,
    { kind: "query" }
  >;
  assert.equal(frame.query_id, 7);
  assert.equal(frame.model, "claude-opus-5");
  assert.equal(frame.turn_cap, 24);
  assert.equal(frame.effort, "medium");
  assert.equal(frame.max_output_tokens, 4000);
  assert.deepEqual(frame.transcript, ["assistant: thinking", "tool result: ok"]);
  assert.equal(frame.tools.length, 1);
  assert.equal(frame.tools[0]!.name, "submit");
});

test("the output fixture reports dispatch honestly", () => {
  const frame = decodeFrameBody(fixture("output").subarray(4)) as Extract<
    SidecarFrame,
    { kind: "output" }
  >;
  assert.deepEqual(frame.events[0], { kind: "text", text: "Done." });
  assert.deepEqual(frame.events[1], {
    kind: "tool_call",
    call_id: "call-0",
    name: "submit",
    arguments: "{}",
    dispatched: true,
  });
  assert.equal(frame.receipt.session_id, "session-one");
  assert.equal(frame.receipt.usage[0]!.output_tokens, 34);
});

test("a chunked stream reassembles into whole frames", () => {
  const reader = new FrameReader();
  const stream = new Uint8Array([
    ...fixture("tool_call"),
    ...fixture("tool_result"),
  ]);
  const seen: SidecarFrame[] = [];
  for (let at = 0; at < stream.length; at += 3) {
    reader.push(stream.subarray(at, Math.min(at + 3, stream.length)));
    for (;;) {
      const frame = reader.next();
      if (!frame) {
        break;
      }
      seen.push(frame);
    }
  }
  assert.deepEqual(
    seen.map((frame) => frame.kind),
    ["tool_call", "tool_result"],
  );
});

test("a length prefix outside the cap is refused", () => {
  const reader = new FrameReader();
  const prefix = new Uint8Array(4);
  new DataView(prefix.buffer).setUint32(0, MAX_FRAME_BYTES + 1, false);
  reader.push(prefix);
  assert.throws(() => reader.next());
});
