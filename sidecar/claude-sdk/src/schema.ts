// The JSON Schema grammar Ghostlight's one emitter produces, converted to the
// Zod raw shape the SDK's `tool()` takes. It is closed on purpose: anything
// outside the grammar throws, so an emitter that grows a construct fails a test
// here rather than silently registering a looser tool.

import { z } from "zod";

type Json = Record<string, unknown>;

const FORBIDDEN = ["oneOf", "allOf", "not", "$ref"] as const;

function isObject(value: unknown): value is Json {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function convert(node: unknown, at: string): z.ZodTypeAny {
  if (!isObject(node)) {
    throw new Error(`${at}: schema node is not an object`);
  }
  for (const forbidden of FORBIDDEN) {
    if (forbidden in node) {
      throw new Error(`${at}: \`${forbidden}\` is outside the emitted grammar`);
    }
  }
  if ("anyOf" in node) {
    const branches = node.anyOf;
    if (!Array.isArray(branches) || branches.length === 0) {
      throw new Error(`${at}: anyOf must be a nonempty array`);
    }
    // The nullable shape the emitter uses for an optional argument.
    if (
      branches.length === 2 &&
      isObject(branches[1]) &&
      branches[1].type === "null"
    ) {
      return convert(branches[0], `${at}/anyOf/0`).nullable();
    }
    const converted = branches.map((branch, index) =>
      convert(branch, `${at}/anyOf/${index}`),
    );
    if (converted.length === 1) {
      return converted[0]!;
    }
    return z.union(converted as [z.ZodTypeAny, z.ZodTypeAny, ...z.ZodTypeAny[]]);
  }
  const kind = node.type;
  if (typeof kind !== "string") {
    throw new Error(`${at}: schema node names no type`);
  }
  switch (kind) {
    case "object":
      return z.object(rawShape(node, at)).strict();
    case "string": {
      if ("const" in node) {
        if (typeof node.const !== "string") {
          throw new Error(`${at}: a string const must be a string`);
        }
        return z.literal(node.const);
      }
      if ("enum" in node) {
        const values = node.enum;
        if (
          !Array.isArray(values) ||
          values.length === 0 ||
          values.some((value) => typeof value !== "string")
        ) {
          throw new Error(`${at}: enum must be a nonempty array of strings`);
        }
        return z.enum(values as [string, ...string[]]);
      }
      return z.string();
    }
    case "integer": {
      let schema = z.number().int();
      if (typeof node.minimum === "number") {
        schema = schema.min(node.minimum);
      }
      if (typeof node.maximum === "number") {
        schema = schema.max(node.maximum);
      }
      return schema;
    }
    case "boolean":
      return z.boolean();
    case "null":
      return z.null();
    case "array":
      return z.array(convert(node.items, `${at}/items`));
    default:
      throw new Error(`${at}: type \`${kind}\` is outside the emitted grammar`);
  }
}

function rawShape(node: Json, at: string): Record<string, z.ZodTypeAny> {
  if (node.additionalProperties !== false) {
    throw new Error(`${at}: every emitted object is closed`);
  }
  const properties = node.properties;
  if (!isObject(properties)) {
    throw new Error(`${at}: object without properties`);
  }
  const names = Object.keys(properties).sort();
  const required = Array.isArray(node.required)
    ? [...(node.required as unknown[])].map(String).sort()
    : [];
  if (names.length !== required.length || names.some((name, index) => name !== required[index])) {
    throw new Error(`${at}: every emitted property is required`);
  }
  const shape: Record<string, z.ZodTypeAny> = {};
  for (const name of names) {
    shape[name] = convert(properties[name], `${at}/${name}`);
  }
  return shape;
}

/**
 * One tool's `parameters_json` as the raw shape `tool()` takes. Throws on any
 * node outside the grammar the emitter produces.
 */
export function toolRawShape(
  parametersJson: string,
  at: string,
): Record<string, z.ZodTypeAny> {
  let parsed: unknown;
  try {
    parsed = JSON.parse(parametersJson);
  } catch (error) {
    throw new Error(`${at}: parameters_json is not JSON (${String(error)})`);
  }
  if (!isObject(parsed) || parsed.type !== "object") {
    throw new Error(`${at}: a tool's parameters must be an object schema`);
  }
  return rawShape(parsed, at);
}
