#!/usr/bin/env node
"use strict";

const fs = require("node:fs");
const Module = require("node:module");
const path = require("node:path");

const repoRoot = path.resolve(__dirname, "..");
process.env.NODE_PATH = [
  path.resolve(repoRoot, "..", "CultLib", "packages"),
  process.env.NODE_PATH || "",
].filter(Boolean).join(path.delimiter);
Module._initPaths();

const { defineDocumentType } = require("cultcache-ts");
const { CultMesh } = require("cultmesh-ts");
const { defineCultNetDocumentBinding } = require("cultnet-ts");

const providerAdvertisementDefinition = defineDocumentType({
  type: "gamecult.eve.provider_advertisement",
  schemaName: "gamecult.eve.provider_advertisement",
  schemaId: "gamecult.eve.provider_advertisement.v1",
  schemaVersion: "gamecult.eve.provider_advertisement.v1",
  global: true,
  name: (value) => value?.providerId || "ghostlight",
  schema: { parse: (value) => value },
  members: [
    { slot: 0, memberName: "providerId", typeName: "string", isName: true },
    { slot: 1, memberName: "serviceId", typeName: "string" },
    { slot: 2, memberName: "verseId", typeName: "string" },
    { slot: 3, memberName: "title", typeName: "string" },
    { slot: 4, memberName: "description", typeName: "string" },
    { slot: 5, memberName: "canonicalService", typeName: "string" },
    { slot: 6, memberName: "locatedService", typeName: "string" },
    { slot: 7, memberName: "cultMeshAddress", typeName: "string" },
    { slot: 8, memberName: "status", typeName: "string" },
    { slot: 9, memberName: "updatedAt", typeName: "string" },
    { slot: 10, memberName: "capabilities", typeName: "array" },
    { slot: 11, memberName: "endpoints", typeName: "array" },
    { slot: 12, memberName: "routes", typeName: "array" },
  ],
});

main().catch((error) => {
  console.error(error?.stack || error?.message || String(error));
  process.exitCode = 1;
});

async function main() {
  const args = parseArgs(process.argv.slice(2));
  const endpoint = args["odin-cultmesh-rudp"] || process.env.GHOSTLIGHT_ODIN_CULTMESH_RUDP || "";
  if (!endpoint) {
    throw new Error("Ghostlight Odin publication requires --odin-cultmesh-rudp <host:port> or GHOSTLIGHT_ODIN_CULTMESH_RUDP.");
  }

  const fixturePath = path.resolve(repoRoot, args.fixture || "examples/ghostlight-provider-advertisement.v0.json");
  if (!isInside(repoRoot, fixturePath)) {
    throw new Error("Ghostlight provider fixture path must stay inside the repo.");
  }

  const fixture = JSON.parse(fs.readFileSync(fixturePath, "utf8"));
  const advertisement = buildOdinAdvertisement(fixture, fixturePath);
  await CultMesh.publishRudpDocumentOnce(
    "ghostlight",
    0x0d1d0002,
    normalizeRudpEndpoint(endpoint),
    defineCultNetDocumentBinding({ definition: providerAdvertisementDefinition }),
    advertisement.providerId,
    advertisement,
    {
      sourceRole: "ghostlight.provider",
      tags: ["startup-respect", "odin-verse-discovery"],
    },
  );

  console.log(JSON.stringify({
    ok: true,
    schemaId: "gamecult.eve.provider_advertisement.v1",
    providerId: advertisement.providerId,
    odinCultMeshRudp: endpoint,
  }, null, 2));
}

function buildOdinAdvertisement(fixture, fixturePath) {
  const observedAt = new Date().toISOString();
  const providerId = fixture.provider_id || "ghostlight";
  const surfaces = Array.isArray(fixture.eve_surfaces) ? fixture.eve_surfaces : [];
  const schemas = Array.isArray(fixture.schemas) ? fixture.schemas : [];
  const witnesses = Array.isArray(fixture.cultcache_witnesses) ? fixture.cultcache_witnesses : [];
  return {
    providerId,
    serviceId: "ghostlight.social-agent-state",
    verseId: "starfire.local",
    title: fixture.service_name || "Ghostlight",
    description: "Socially persistent generative-agent state/projection research service candidate.",
    canonicalService: "asgard.ghostlight",
    locatedService: "asgard.starfire.ghostlight",
    cultMeshAddress: "asgard.starfire.ghostlight/state",
    status: "read-only-provider-advertisement",
    updatedAt: observedAt,
    capabilities: [
      "social-agent-state",
      "persona-state-projection",
      "training-fixture-review",
      "cultcache-witness",
      "eve-surface-target",
    ],
    endpoints: [
      { transport: "cultcache-witness", address: "state/ghostlight-state.cultcache.jsonl" },
      { transport: "repo-fixture", address: path.relative(repoRoot, fixturePath).replace(/\\/g, "/") },
    ],
    routes: [
      { transport: "repo-cli", address: "npm run provider:advertisement", role: "provider-advertisement" },
      { transport: "repo-cli", address: "npm run state:status", role: "status" },
    ],
    schemaCatalog: schemas.map((schema) => ({
      schemaId: schema.schema_version || schema.document_type || "unknown",
      path: schema.path || "",
      owns: Array.isArray(schema.owns) ? schema.owns : [],
    })),
    witnessCatalog: witnesses.map((witness) => ({
      schemaId: witness.document_type || "unknown",
      path: witness.witness_path || "",
      readableExport: witness.readable_export || "",
      authority: witness.authority || "",
    })),
    surfaces: surfaces.map((surface) => ({
      id: surface.surface_id || "ghostlight.surface",
      title: surface.title || surface.surface_id || "Ghostlight Surface",
      panels: Array.isArray(surface.panels) ? surface.panels : [],
    })),
    commandSurface: {
      mode: "read-only",
      commands: ["status", "provider-advertisement", "publish-odin"],
      mutationPolicy: "reviewed mutation intent only",
    },
    authority: fixture.authority || {},
    ghostlightProviderAdvertisement: fixture,
  };
}

function normalizeRudpEndpoint(value) {
  const text = String(value || "").trim();
  if (!text) throw new Error("Odin CultMesh/RUDP endpoint must be non-empty.");
  if (text.startsWith("rudp://")) return text;
  const ipv6 = text.match(/^\[([^\]]+)\]:(\d+)$/);
  if (ipv6) return `rudp://[${ipv6[1]}]:${ipv6[2]}`;
  const index = text.lastIndexOf(":");
  if (index <= 0 || index === text.length - 1) {
    throw new Error(`Odin CultMesh/RUDP endpoint must be host:port, got "${value}".`);
  }
  return `rudp://${text.slice(0, index)}:${text.slice(index + 1)}`;
}

function parseArgs(argv) {
  const parsed = {};
  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    if (!arg.startsWith("--")) continue;
    const key = arg.slice(2);
    const next = argv[index + 1];
    if (!next || next.startsWith("--")) {
      parsed[key] = true;
    } else {
      parsed[key] = next;
      index += 1;
    }
  }
  return parsed;
}

function isInside(root, candidate) {
  const relative = path.relative(root, candidate);
  return relative === "" || (!relative.startsWith("..") && !path.isAbsolute(relative));
}
