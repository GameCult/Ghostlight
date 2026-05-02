# Qwen Invocation Notes

Ghostlight should call the local Qwen box through Ollama's chat/tool path when
structured output matters.

## Verified Local Path

Use:

- endpoint: `http://192.168.1.84:11434/api/chat`
- model: `qwen3.5:9b`
- `think: true`
- `tools`: native Ollama function tools with JSON Schema parameters

Do not use `/api/generate` plus prose-only "return JSON" instructions for
strict scene-generation artifacts. That path can produce useful writing, but it
bypasses the tool-call template and leaves schema discipline to vibes. The vibes
have small hands and drop things.

`tools/check_qwen_chat_tools.py` is the fast smoke test. It verifies that the
local model returns both:

- `message.thinking`
- `message.tool_calls[0].function.arguments`

## Why XML Shows Up In Qwen Docs

Qwen3's tool-call chat template is XML-shaped internally: tools are presented
inside `<tools>` tags, and calls are represented inside `<tool_call>` tags. This
is the tool-call protocol, not a general recommendation to wrap every structured
Ghostlight artifact in XML.

The Qwen docs recommend using tokenizer/chat-template formatting or Qwen-Agent
rather than hand-writing the tool-call format. Qwen-Agent is described as the
canonical function-calling implementation for Qwen3 over OpenAI-compatible
backends. Ollama's `/api/chat` with `tools` gives us the same practical seam for
this repo without adding Qwen-Agent as a dependency yet.

Sources:

- Qwen function calling docs: https://qwen.readthedocs.io/en/stable/framework/function_call.html
- Qwen core concepts/tool template: https://qwen.readthedocs.io/zh-cn/latest/getting_started/concepts.html#tool-calling
- Qwen-Agent repository: https://github.com/QwenLM/Qwen-Agent
- Qwen-Agent configuration docs: https://qwenlm.github.io/Qwen-Agent/en/guide/get_started/configuration/
- Ollama tool calling docs: https://docs.ollama.com/capabilities/tool-calling

## Current Findings

- `/api/chat` with `think: true` and `tools` works locally for `qwen3.5:9b`.
- The smoke receipt lives at `experiments/ink/qwen-chat-tools-smoke.json`.
- `tools/run_qwen_ink_sequential_generation.py` now defaults to the chat/tools
  endpoint and keeps thinking enabled unless `--no-think` is passed.
- Tool calling improves action enum discipline over prose JSON prompting.
- Nested tool arguments can still be double-stringified by the model. The
  runner repairs known fields such as `choices`, `appraisal`, and `response`
  when they arrive as JSON strings, and records validation notes when repair
  fails.
- Projector-routed v8 showed a malformed stringified `choices` array with an
  obvious missing closing bracket. The runner can now repair that class of
  truncated tool-string wrapper, but the capture stays useful-needs-revision if
  no valid choice remains.
- Projector-routed v9 showed a no-tool-call dropout. This is not repairable
  from arguments because there are no arguments. The next harness step should
  be an explicit retry or fallback pass, not prompt scolding.

## Current Policy

For structured Ghostlight generation:

1. Prefer native chat/tool calls with thinking enabled.
2. Keep tool schemas narrow and concrete.
3. Preserve captures that fail formatting as `useful_needs_revision` receipts.
4. Use repair only as a reviewed harness layer, not as proof the model obeyed
   the schema perfectly.
5. If a pass returns no tool call, retry or route through a deliberate fallback
   before continuing the sequence.
6. Only materialize Ink or canonical mutation from captures that validate after
   tool parsing and repair.

Qwen-Agent remains a future option if we need its higher-level tool loop,
OpenAI-compatible wrapping, or MCP integration. For now, direct Ollama
`/api/chat` tools are enough for the local Ghostlight runner.
