// Spec test 17: every schema Ghostlight's one emitter actually produces
// converts, and everything outside that grammar throws. `schemas.json` is
// written by the Rust test `schema_fixtures_match_the_checked_in_sidecar_grammar`,
// so the two halves cannot drift apart.

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import test from "node:test";
import { toolInputSchema } from "../src/schema.ts";

const emitted: { name: string; parameters_json: string }[] = JSON.parse(
  readFileSync(fileURLToPath(new URL("./schemas.json", import.meta.url)), "utf8"),
);

test("every emitted tool schema converts", () => {
  assert.ok(emitted.length > 0, "the schema fixture is empty");
  for (const entry of emitted) {
    const schema = toolInputSchema(entry.parameters_json, entry.name);
    const declared = Object.keys(
      JSON.parse(entry.parameters_json).properties as Record<string, unknown>,
    ).sort();
    assert.deepEqual(Object.keys(schema.shape).sort(), declared, entry.name);
  }
});

test("a converted schema carries every property's own type", () => {
  const schema = toolInputSchema(
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
  const admitted = schema.parse({
    detail: "ok",
    count: 2,
    flag: true,
    choice: "a",
    maybe: null,
    many: ["a"],
  }) as Record<string, unknown>;
  assert.deepEqual(admitted, {
    detail: "ok",
    count: 2,
    flag: true,
    choice: "a",
    maybe: null,
    many: ["a"],
  });

  // A value outside a property's own type empties that property and nothing
  // else. Rust decides what an absent field means.
  const dropped = schema.parse({ detail: "ok", count: 9, choice: "c" }) as Record<
    string,
    unknown
  >;
  assert.equal(dropped.detail, "ok");
  assert.equal(dropped.count, undefined);
  assert.equal(dropped.choice, undefined);
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
        toolInputSchema(
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

test("an open object and a partly optional one convert and refuse nothing", () => {
  // Openness and optionality are Rust's judgements, so neither is a
  // registration failure here.
  const open = toolInputSchema(
    JSON.stringify({ type: "object", required: [], properties: {} }),
    "open",
  );
  assert.deepEqual(open.parse({ anything: 1 }), { anything: 1 });

  const partial = toolInputSchema(
    JSON.stringify({
      type: "object",
      additionalProperties: false,
      required: ["a"],
      properties: { a: { type: "string" }, b: { type: "string" } },
    }),
    "partial",
  );
  assert.deepEqual(partial.parse({ b: "set" }), { b: "set" });
});
