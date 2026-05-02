"""Smoke-test Qwen thinking plus native Ollama tool calls."""

from __future__ import annotations

import argparse
import json
import urllib.request
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
DEFAULT_ENDPOINT = "http://192.168.1.84:11434/api/chat"


def post_json(endpoint: str, payload: dict[str, Any]) -> dict[str, Any]:
    data = json.dumps(payload, ensure_ascii=False).encode("utf-8")
    request = urllib.request.Request(
        endpoint,
        data=data,
        headers={"Content-Type": "application/json"},
        method="POST",
    )
    with urllib.request.urlopen(request, timeout=120) as response:
        return json.loads(response.read().decode("utf-8"))


def main() -> int:
    parser = argparse.ArgumentParser(description="Check Qwen /api/chat thinking plus tool calling.")
    parser.add_argument("--endpoint", default=DEFAULT_ENDPOINT)
    parser.add_argument("--model", default="qwen3.5:9b")
    parser.add_argument("--think", action=argparse.BooleanOptionalAction, default=True)
    parser.add_argument("--out", type=Path, default=ROOT / "experiments" / "ink" / "qwen-chat-tools-smoke.json")
    args = parser.parse_args()

    payload = {
        "model": args.model,
        "stream": False,
        "think": args.think,
        "messages": [
            {
                "role": "user",
                "content": "Think privately if supported, then call the tool with action_type show_object and note qwen-chat-tools-smoke.",
            }
        ],
        "tools": [
            {
                "type": "function",
                "function": {
                    "name": "record_action",
                    "description": "Record one canonical Ghostlight action.",
                    "parameters": {
                        "type": "object",
                        "required": ["action_type", "note"],
                        "properties": {
                            "action_type": {"type": "string", "enum": ["speak", "show_object", "withhold_object"]},
                            "note": {"type": "string"},
                        },
                    },
                },
            }
        ],
    }
    raw = post_json(args.endpoint, payload)
    message = raw.get("message", {})
    tool_calls = message.get("tool_calls") or []
    if not tool_calls:
        raise SystemExit("Qwen did not return tool_calls")
    arguments = tool_calls[0].get("function", {}).get("arguments", {})
    if arguments.get("action_type") != "show_object":
        raise SystemExit(f"Unexpected tool arguments: {arguments}")
    if args.think and not message.get("thinking"):
        raise SystemExit("Qwen did not return a thinking field while think=true")

    receipt = {
        "schema_version": "ghostlight.qwen_chat_tools_smoke.v0",
        "endpoint": args.endpoint,
        "model": args.model,
        "think_requested": args.think,
        "has_thinking": bool(message.get("thinking")),
        "tool_name": tool_calls[0].get("function", {}).get("name"),
        "tool_arguments": arguments,
        "raw": raw,
    }
    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(json.dumps(receipt, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    print(f"ok: thinking={receipt['has_thinking']} tool={receipt['tool_name']} args={receipt['tool_arguments']}")
    print(f"wrote {args.out.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
