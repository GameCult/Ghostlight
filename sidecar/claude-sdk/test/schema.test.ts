// Spec test 17: every schema Ghostlight's one emitter actually produces
// converts, and everything outside that grammar throws. `schemas.json` is
// written by the Rust test `schema_fixtures_match_the_checked_in_sidecar_grammar`,
// so the two halves cannot drift apart.

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import test from "node:test";
import { toolRawShape } from "../src/schema.ts";

const emitted: { name: string; parameters_json: string }[] = JSON.parse(
  readFileSync(fileURLToPath(new URL("./schemas.json", import.meta.url)), "utf8"),
);

test("every emitted tool schema converts", () => {
  assert.ok(emitted.length > 0, "the schema fixture is empty");
  for (const entry of emitted) {
    const shape = toolRawShape(entry.parameters_json, entry.name);
    const declared = Object.keys(
      JSON.parse(entry.parameters_json).properties as Record<string, unknown>,
    ).sort();
    assert.deepEqual(Object.keys(shape).sort(), declared, entry.name);
  }
});

test("a converted shape accepts a value its schema admits", () => {
  const shape = toolRawShape(
    JSON.stringify({
      type: "object",
      additionalProperties: false,
      required: ["detail", "count", "flag", "choice", "maybe", "many"],
      properties: {
        detail: { type: "string" },
        count: { type: "integer", minimum: 1, maximum: 4 },
        flag: { type: "boolean" },
        choice: { type: "string", enum: ["a", "b"] },
        maybe: { anyOf: [{ type: "string" }, { type: "null" }] },
        many: { type: "array", items: { type: "string" } },
      },
    }),
    "sample",
  );
  assert.equal(shape.detail!.safeParse("ok").success, true);
  assert.equal(shape.count!.safeParse(2).success, true);
  assert.equal(shape.count!.safeParse(9).success, false);
  assert.equal(shape.choice!.safeParse("c").success, false);
  assert.equal(shape.maybe!.safeParse(null).success, true);
  assert.equal(shape.many!.safeParse(["a"]).success, true);
});

test("anything outside the grammar throws", () => {
  const refused = [
    { oneOf: [{ type: "string" }] },
    { allOf: [{ type: "string" }] },
    { $ref: "#/definitions/thing" },
    { description: "no type at all" },
    { type: "number" },
  ];
  for (const node of refused) {
    assert.throws(
      () =>
        toolRawShape(
          JSON.stringify({
            type: "object",
            additionalProperties: false,
            required: ["field"],
            properties: { field: node },
          }),
          "refused",
        ),
      `${JSON.stringify(node)} was accepted`,
    );
  }
});

test("an open object or a partly optional one throws", () => {
  assert.throws(() =>
    toolRawShape(
      JSON.stringify({ type: "object", required: [], properties: {} }),
      "open",
    ),
  );
  assert.throws(() =>
    toolRawShape(
      JSON.stringify({
        type: "object",
        additionalProperties: false,
        required: ["a"],
        properties: { a: { type: "string" }, b: { type: "string" } },
      }),
      "partial",
    ),
  );
});
