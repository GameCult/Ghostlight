#!/usr/bin/env node
import { spawnSync } from "node:child_process";
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const DEFAULT_INITIAL_STATE =
  "examples/training-loops/pallas-training-loop-v0/pallas-training-loop-v0.initial-state.json";
const DEFAULT_STORE = "state/ghostlight-scene-heartbeats.msgpack";
const DEFAULT_ARTIFACT_DIR = "experiments/heartbeat";

function usage() {
  console.log(`Usage:
  node tools/ghostlight_epiphany_heartbeat.mjs init-scene [options]
  node tools/ghostlight_epiphany_heartbeat.mjs tick-scene [options]
  node tools/ghostlight_epiphany_heartbeat.mjs status [options]

Options:
  --initial-state <path>   Ghostlight scene-loop initial state JSON.
  --agent-state <path>     Ghostlight agent-state JSON. Defaults from initial state.
  --scene-id <id>          Scene heartbeat id. Defaults to scene_loop_id.
  --scene-ref <path>       Source scene ref for receipts. Defaults to initial-state path.
  --store <path>           MessagePack heartbeat store. Defaults to ${DEFAULT_STORE}.
  --artifact-dir <path>    Artifact directory for tick/status projections.
  --schedule-id <id>       Tick schedule id.
  --schedule-out <path>    Write tick schedule JSON projection to this path.
  --epiphany-root <path>   EpiphanyAgent root. Defaults to sibling ../EpiphanyAgent.
  --no-defer-completion    Complete cooldown immediately on tick.
`);
}

function parseArgs(argv) {
  const [command, ...rest] = argv;
  const options = {};
  for (let i = 0; i < rest.length; i += 1) {
    const token = rest[i];
    if (!token.startsWith("--")) {
      throw new Error(`unexpected argument ${token}`);
    }
    const key = token.slice(2);
    if (key === "no-defer-completion") {
      options[key] = true;
      continue;
    }
    const value = rest[i + 1];
    if (!value || value.startsWith("--")) {
      throw new Error(`missing value for ${token}`);
    }
    options[key] = value;
    i += 1;
  }
  return { command, options };
}

function resolveRepoPath(input, fallback) {
  const value = input || fallback;
  const resolved = path.isAbsolute(value) ? value : path.resolve(ROOT, value);
  return resolved;
}

function toRepoRef(filePath) {
  const relative = path.relative(ROOT, filePath).replaceAll("\\", "/");
  return relative.startsWith("..") ? filePath : relative;
}

function readJson(filePath) {
  return JSON.parse(readFileSync(filePath, "utf8").replace(/^\uFEFF/, ""));
}

function cleanRef(ref) {
  return ref.split("#", 1)[0];
}

function clamp(value, min, max) {
  return Math.min(max, Math.max(min, value));
}

function dimensionMean(groups, names, fallback) {
  for (const groupName of Object.keys(groups || {})) {
    const group = groups[groupName] || {};
    for (const name of names) {
      const entry = group[name];
      const value = entry?.current_activation ?? entry?.mean;
      if (typeof value === "number" && Number.isFinite(value)) {
        return value;
      }
    }
  }
  return fallback;
}

function seedForAgent(agent, initialState) {
  const canonical = agent.canonical_state || {};
  const initiativeSource = dimensionMean(
    {
      behavioral_dimensions: canonical.behavioral_dimensions,
      stable_dispositions: canonical.stable_dispositions,
    },
    ["drive", "risk_triage", "cautious_courage", "coalition_building"],
    0.5,
  );
  const reactionSource = dimensionMean(
    {
      situational_state: canonical.situational_state,
      stable_dispositions: canonical.stable_dispositions,
    },
    ["protective_alarm", "anxiety", "baseline_threat_sensitivity", "suspicion"],
    0.5,
  );
  const thresholdSource = dimensionMean(
    {
      stable_dispositions: canonical.stable_dispositions,
      behavioral_dimensions: canonical.behavioral_dimensions,
    },
    ["risk_tolerance", "control_pressure", "rigidity"],
    0.5,
  );
  const speed = clamp(0.7 + initiativeSource * 0.75, 0.35, 1.65);
  const reaction = clamp(0.2 + reactionSource * 0.7, 0.05, 0.95);
  const threshold = clamp(0.75 - thresholdSource * 0.45, 0.15, 0.85);
  const displayName = agent.identity?.name || agent.agent_id;
  const constraints = [
    `Scene loop: ${initialState.scene_loop_id || initialState.state_id || "unknown"}.`,
    "Receives only projected local context; no hidden author omniscience.",
  ];
  return [
    agent.agent_id,
    String(displayName).replaceAll("|", "/"),
    speed.toFixed(3),
    reaction.toFixed(3),
    threshold.toFixed(3),
    constraints.join("; "),
  ].join("|");
}

function sceneSeeds(initialStatePath, agentStatePath) {
  const initialState = readJson(initialStatePath);
  const agentStateRef = agentStatePath
    ? agentStatePath
    : resolveRepoPath(cleanRef(initialState.reference_agent_state_ref), undefined);
  const agentState = readJson(agentStateRef);
  const participantIds = Object.keys(initialState.participant_state_refs || {});
  if (!participantIds.length) {
    throw new Error(`no participant_state_refs found in ${initialStatePath}`);
  }
  const byId = new Map((agentState.agents || []).map((agent) => [agent.agent_id, agent]));
  const seeds = participantIds.map((id) => {
    const agent = byId.get(id);
    if (!agent) {
      throw new Error(`participant ${id} is missing from ${agentStateRef}`);
    }
    return seedForAgent(agent, initialState);
  });
  return { initialState, seeds };
}

function epiphanyCommand(options, args) {
  const epiphanyRoot = path.resolve(
    options["epiphany-root"] || path.join(ROOT, "..", "EpiphanyAgent"),
  );
  const manifest = path.join(epiphanyRoot, "epiphany-core", "Cargo.toml");
  if (!existsSync(manifest)) {
    throw new Error(`Epiphany heartbeat manifest not found at ${manifest}`);
  }
  return ["run", "--manifest-path", manifest, "--bin", "epiphany-heartbeat-store", "--", ...args];
}

function runEpiphany(options, args) {
  const cargoArgs = epiphanyCommand(options, args);
  const result = spawnSync("cargo", cargoArgs, {
    cwd: ROOT,
    encoding: "utf8",
    stdio: ["ignore", "pipe", "pipe"],
    env: {
      ...process.env,
      CARGO_TARGET_DIR:
        process.env.CARGO_TARGET_DIR || "C:\\Users\\Meta\\.cargo-target-codex",
    },
  });
  if (result.stdout) {
    process.stdout.write(result.stdout);
  }
  if (result.stderr) {
    process.stderr.write(result.stderr);
  }
  if (result.status !== 0) {
    throw new Error(`Epiphany heartbeat command failed with exit ${result.status}`);
  }
  const jsonLine = result.stdout
    .trim()
    .split(/\r?\n/)
    .reverse()
    .find((line) => line.trim().startsWith("{"));
  return jsonLine ? JSON.parse(jsonLine) : null;
}

function writeAuditNote() {
  const auditPath = path.join(ROOT, "notes", "epiphany-intrusion-audit.md");
  if (existsSync(auditPath)) {
    return;
  }
  writeFileSync(
    auditPath,
    `# Epiphany Intrusion Audit

EpiphanyAgent touched this workspace to add the first Ghostlight-facing bridge
to Epiphany's Rust heartbeat initiative store.

Scope:

- no scene fixtures were rewritten
- no accepted narrative artifacts were mutated
- Ghostlight receives a command wrapper that invokes Epiphany's shared Rust
  heartbeat scheduler for scene participants
- initiative schema/validation now accepts scheduler-level \`scene_turn\`
  receipts and arena metadata emitted by that shared spine

Resident agents should treat this as an audited external intervention, not as
their own invisible local work.
`,
    "utf8",
  );
}

function initScene(options) {
  const initialStatePath = resolveRepoPath(options["initial-state"], DEFAULT_INITIAL_STATE);
  const agentStatePath = options["agent-state"]
    ? resolveRepoPath(options["agent-state"], undefined)
    : undefined;
  const { initialState, seeds } = sceneSeeds(initialStatePath, agentStatePath);
  const sceneId = options["scene-id"] || initialState.scene_loop_id || initialState.state_id;
  if (!sceneId) {
    throw new Error("scene id is required when initial state lacks scene_loop_id/state_id");
  }
  const store = resolveRepoPath(options.store, DEFAULT_STORE);
  mkdirSync(path.dirname(store), { recursive: true });
  const args = [
    "init",
    "--store",
    store,
    "--profile",
    "ghostlight-scene",
    "--scene-id",
    sceneId,
  ];
  for (const seed of seeds) {
    args.push("--scene-participant", seed);
  }
  return runEpiphany(options, args);
}

function tickScene(options) {
  const initialStatePath = resolveRepoPath(options["initial-state"], DEFAULT_INITIAL_STATE);
  const initialState = readJson(initialStatePath);
  const store = resolveRepoPath(options.store, DEFAULT_STORE);
  const artifactDir = resolveRepoPath(options["artifact-dir"], DEFAULT_ARTIFACT_DIR);
  mkdirSync(artifactDir, { recursive: true });
  const scheduleId =
    options["schedule-id"] ||
    `${options["scene-id"] || initialState.scene_loop_id || "ghostlight-scene"}.heartbeat-${Date.now()}`;
  const sceneRef = options["scene-ref"] || toRepoRef(initialStatePath);
  const args = [
    "tick",
    "--store",
    store,
    "--artifact-dir",
    artifactDir,
    "--schedule-id",
    scheduleId,
    "--source-scene-ref",
    sceneRef,
  ];
  if (!options["no-defer-completion"]) {
    args.push("--defer-completion");
  }
  const output = runEpiphany(options, args);
  if (options["schedule-out"] && output?.schedule) {
    const scheduleOut = resolveRepoPath(options["schedule-out"], undefined);
    mkdirSync(path.dirname(scheduleOut), { recursive: true });
    writeFileSync(scheduleOut, `${JSON.stringify(output.schedule, null, 2)}\n`, "utf8");
    console.log(JSON.stringify({ ok: true, scheduleOut: toRepoRef(scheduleOut) }));
  }
  return output;
}

function status(options) {
  const store = resolveRepoPath(options.store, DEFAULT_STORE);
  const artifactDir = resolveRepoPath(options["artifact-dir"], DEFAULT_ARTIFACT_DIR);
  mkdirSync(artifactDir, { recursive: true });
  return runEpiphany(options, ["status", "--store", store, "--artifact-dir", artifactDir]);
}

try {
  const { command, options } = parseArgs(process.argv.slice(2));
  if (!command || command === "help" || command === "--help") {
    usage();
    process.exit(0);
  }
  writeAuditNote();
  if (command === "init-scene") {
    initScene(options);
  } else if (command === "tick-scene") {
    tickScene(options);
  } else if (command === "status") {
    status(options);
  } else {
    throw new Error(`unknown command ${command}`);
  }
} catch (error) {
  console.error(error instanceof Error ? error.message : String(error));
  process.exit(1);
}
