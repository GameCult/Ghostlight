# Scratch Working Memory

## Current Subgoal

Plan step 11, the Claude SDK inference port: Hands in
`F:\Projects\Ghostlight-sdkport` on `hands/sdk-port` from `634e81f`, spec
`imagination-sdk-port.md` (eleven forks), map `modeling-sdk-port.md`; then Soul
in that tree, then gated integration (rebase, merge, confirm tip, remove).
Shape: `InferencePort` over the Agent SDK; sidecar `sidecar/claude-sdk` never
computes a tool result, Rust answers via per-lane `ToolResultOracle`; same
`PreparedInference`; routing by `GHOSTLIGHT_SDK_MODEL_PREFIX`. Operator
credential steps go in the smoke runbook; nothing installed locally yet.

## Working Notes

Use this file for one bounded slice. Delete or reset aggressively when the
slice is done.
