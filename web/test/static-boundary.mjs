import { readFileSync, readdirSync, statSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

const forbidden = [
  "/api/session-zero",
  "/api/governance",
  "/api/command",
  "/api/auth/heimdall",
  "renderSession",
  "renderCampaign",
  "payload.fields",
  'kind: "form"',
  '"kind":"form"',
];

function sourceFiles(directory) {
  return readdirSync(directory)
    .flatMap(name => {
      const path = join(directory, name);
      return statSync(path).isDirectory() ? sourceFiles(path) : [path];
    })
    .filter(path => /\.(ts|tsx|js|mjs)$/.test(path));
}

const violations = sourceFiles(fileURLToPath(new URL("../src", import.meta.url)))
  .flatMap(path => {
    const source = readFileSync(path, "utf8");
    return forbidden.filter(token => source.includes(token)).map(token => `${path}: ${token}`);
  });

if (violations.length) {
  throw new Error(`Ghostlight browser crossed its Eve boundary:\n${violations.join("\n")}`);
}

console.log("Ghostlight browser contains only the Eve provider boundary.");
