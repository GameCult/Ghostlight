// The JSON Schema grammar Ghostlight's one emitter produces, converted to the
// Zod object the SDK's `tool()` registers.
//
// The conversion exists for one reason: the model must see the same parameter
// schema on both transports, and the in-process MCP server derives the schema
// it advertises from this Zod object. It is not a validator. Rust's decoder is
// the one validator on both transports, so every object here is loose and every
// property catches: an extra key, a missing required property, and a mistyped
// value all reach the handler, travel to Rust unstripped, and become the gap
// the evaluator already records instead of a tool call the SDK answers itself.
//
// A construct outside the emitted grammar still throws. That is a registration
// failure, not an argument judgement: a schema this module cannot convert is a
// tool the model would otherwise be shown blind.

import { z } from "zod";

type Json = Record<string, unknown>;

const FORBIDDEN = ["oneOf", "allOf", "not", "$ref"] as const;

function isObject(value: unknown): value is Json {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

/**
 * Wraps one property so it can never refuse the call. The catch value is
 * static because Zod refuses to render a dynamic one as JSON Schema, and the
 * advertised schema is the whole point of converting. A value the property's
 * type refuses therefore arrives at Rust as an absent field, which its decoder
 * reports the same way it reports any other missing field.
 */
function keep(schema: z.ZodTypeAny): z.ZodTypeAny {
  return schema.catch(undefined as never);
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
      return objectSchema(node, at);
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

/** One object node, loose so an unknown key survives to Rust with its value. */
function objectSchema(node: Json, at: string): z.ZodObject {
  const properties = node.properties;
  if (!isObject(properties)) {
    throw new Error(`${at}: object without properties`);
  }
  const shape: Record<string, z.ZodTypeAny> = {};
  for (const name of Object.keys(properties).sort()) {
    shape[name] = keep(convert(properties[name], `${at}/${name}`));
  }
  return z.looseObject(shape);
}

/**
 * One tool's `parameters_json` as the schema `tool()` registers. Throws on any
 * node outside the grammar the emitter produces.
 */
export function toolInputSchema(parametersJson: string, at: string): z.ZodObject {
  let parsed: unknown;
  try {
    parsed = JSON.parse(parametersJson);
  } catch (error) {
    throw new Error(`${at}: parameters_json is not JSON (${String(error)})`);
  }
  if (!isObject(parsed) || parsed.type !== "object") {
    throw new Error(`${at}: a tool's parameters must be an object schema`);
  }
  return objectSchema(parsed, at);
}
