# Scratch Working Memory

## Current Subgoal

Claude SDK port (operator go, 2026-09-06: wait for the research, then build).
Shape: a Ghostlight `InferencePort` implementation over the Claude Agent SDK,
credential held by Claude Code, stopgap transport, separate small organ, not a
CodexConnector backend. Persona and Projector are plain completions; the tool
lanes need unexecuted tool-call return, which `research-agent-sdk.md` is
establishing; on failure those lanes stay on Codex (models are per lane).
Harnessed Persona withdrawn. Nothing cut at `06236a4`; Hands waits on the
research result.

## Working Notes

Use this file for one bounded slice. Delete or reset aggressively when the
slice is done.
