// Soul: the two sidecar gates that live in the stdin dispatcher rather than in
// a pure function, driven against the real built entry point.
//
// Neither case reaches `query()`, so no Claude Code subprocess is spawned, no
// credential is looked for, and no network is touched. The gate that needs a
// live query — a second `query` frame while one is in flight — is deliberately
// not exercised here for that reason.

import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { existsSync } from "node:fs";
import { fileURLToPath } from "node:url";
import test from "node:test";
import { FrameReader, encodeFrame, type SidecarFrame } from "../src/frames.ts";
import { QuerySession, SidecarFault } from "../src/main.ts";
import { encode } from "@msgpack/msgpack";

const entry = fileURLToPath(new URL("../dist/main.js", import.meta.url));

/** Writes raw bytes to a fresh sidecar child and returns the frames it emits. */
async function exchange(bytes: Uint8Array): Promise<SidecarFrame[]> {
  const child = spawn(process.execPath, [entry], {
    stdio: ["pipe", "pipe", "inherit"],
  });
  const reader = new FrameReader();
  const frames: SidecarFrame[] = [];
  child.stdout.on("data", (chunk: Buffer) => {
    reader.push(new Uint8Array(chunk));
    for (;;) {
      const frame = reader.next();
      if (!frame) {
        break;
      }
      frames.push(frame);
    }
  });
  child.stdin.write(bytes);
  child.stdin.end();
  await new Promise<void>((done) => child.on("close", () => done()));
  return frames;
}

function framed(body: Uint8Array): Uint8Array {
  const out = new Uint8Array(4 + body.length);
  new DataView(out.buffer).setUint32(0, body.length, false);
  out.set(body, 4);
  return out;
}

test("a tool result for a call the session never issued is a protocol violation", async () => {
  const sent: SidecarFrame[] = [];
  const session = new QuerySession(4, (frame) => sent.push(frame));
  const pending = session.ask("record_gap", { detail: "no route" });
  assert.deepEqual(session.dispatches, [
    { call_id: "c0", name: "record_gap", arguments: '{"detail":"no route"}' },
  ]);
  assert.throws(
    () => session.answer("c9", "gap recorded"),
    (error: unknown) =>
      error instanceof SidecarFault && error.reason === "protocol_violation",
  );
  session.answer("c0", "gap recorded");
  assert.equal(await pending, "gap recorded");
  // And a second answer for a call already resolved is refused too, so one
  // dispatch can never be answered twice.
  assert.throws(
    () => session.answer("c0", "gap recorded"),
    (error: unknown) =>
      error instanceof SidecarFault && error.reason === "protocol_violation",
  );
  assert.equal(sent.length, 1);
});

test("a frame kind the sidecar does not accept is a protocol violation", async (t) => {
  if (!existsSync(entry)) {
    t.skip("run `npm run build` first");
    return;
  }
  // A well-formed MessagePack map with a kind that is not on the wire.
  const frames = await exchange(
    framed(encode({ kind: "hallucination", query_id: 3 }, { useBigInt64: false })),
  );
  assert.equal(frames.length, 1);
  assert.deepEqual(frames[0], {
    kind: "fault",
    query_id: 0,
    reason: "protocol_violation",
    detail: "frame kind hallucination is not one this sidecar accepts",
  });
});

test("a tool result outside any query is a protocol violation", async (t) => {
  if (!existsSync(entry)) {
    t.skip("run `npm run build` first");
    return;
  }
  const frames = await exchange(
    encodeFrame({
      kind: "tool_result",
      query_id: 7,
      call_id: "c0",
      output: "gap recorded",
    }),
  );
  assert.equal(frames.length, 1);
  const [fault] = frames;
  assert.ok(fault && fault.kind === "fault");
  assert.equal(fault.reason, "protocol_violation");
  assert.equal(fault.query_id, 7);
});

test("a length prefix over the cap ends the sidecar rather than buffering it", async (t) => {
  if (!existsSync(entry)) {
    t.skip("run `npm run build` first");
    return;
  }
  const over = new Uint8Array(8);
  new DataView(over.buffer).setUint32(0, 1_052_673, false);
  const frames = await exchange(over);
  assert.equal(frames.length, 1);
  const [fault] = frames;
  assert.ok(fault && fault.kind === "fault");
  assert.equal(fault.reason, "protocol_violation");
});
