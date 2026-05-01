import argparse
import json
import urllib.request
from pathlib import Path


DEFAULT_ENDPOINT = "http://192.168.1.84:11434/api/generate"


def load_projection(path: Path, example_id: str) -> dict:
    for line in path.read_text(encoding="utf-8").splitlines():
        if not line.strip():
            continue
        record = json.loads(line)
        if record.get("example_id") == example_id:
            return record
    raise SystemExit(f"projection example not found: {example_id}")


def post_json(endpoint: str, payload: dict) -> dict:
    data = json.dumps(payload, ensure_ascii=False).encode("utf-8")
    request = urllib.request.Request(
        endpoint,
        data=data,
        headers={"Content-Type": "application/json"},
        method="POST",
    )
    with urllib.request.urlopen(request, timeout=180) as response:
        return json.loads(response.read().decode("utf-8"))


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Send a projection prompt_text to the LAN Qwen box and save response artifacts."
    )
    parser.add_argument("example_id")
    parser.add_argument(
        "--projection-file",
        default="examples/projections/cold-wake-story-lab.jsonl",
    )
    parser.add_argument("--out-dir", default="experiments/cold-wake-story-lab")
    parser.add_argument("--endpoint", default=DEFAULT_ENDPOINT)
    parser.add_argument("--model", default="qwen3.5:9b")
    parser.add_argument("--temperature", type=float, default=0.75)
    parser.add_argument("--top-p", type=float, default=0.9)
    parser.add_argument("--num-ctx", type=int, default=8192)
    parser.add_argument("--suffix", default=None)
    args = parser.parse_args()

    projection = load_projection(Path(args.projection_file), args.example_id)
    prompt = projection["output"]["prompt_text"]
    payload = {
        "model": args.model,
        "prompt": prompt,
        "stream": False,
        "think": False,
        "keep_alive": "10m",
        "options": {
            "num_ctx": args.num_ctx,
            "temperature": args.temperature,
            "top_p": args.top_p,
        },
    }
    raw = post_json(args.endpoint, payload)
    response_text = raw.get("response", "").strip()

    suffix = args.suffix or args.example_id.replace("cold-wake.story-lab.", "")
    safe_suffix = suffix.replace(".", "-")
    out_dir = Path(args.out_dir)
    out_dir.mkdir(parents=True, exist_ok=True)

    capture = {
        "schema_version": "ghostlight.qwen_response_capture.v0",
        "capture_id": f"{args.example_id}.qwen3-5-9b",
        "projection_example_id": args.example_id,
        "model": raw.get("model", args.model),
        "endpoint": args.endpoint,
        "request_options": {
            "think": False,
            "num_ctx": args.num_ctx,
            "temperature": args.temperature,
            "top_p": args.top_p,
        },
        "response_text": response_text,
        "review": {
            "status": "unreviewed",
            "response_kind_observed": "unknown",
            "strengths": [],
            "failure_notes": [],
            "pipeline_lessons": [],
        },
    }

    capture_path = out_dir / f"{safe_suffix}.qwen3-5-9b.capture.json"
    response_path = out_dir / f"{safe_suffix}.qwen3-5-9b.response.md"
    capture_path.write_text(json.dumps(capture, indent=2), encoding="utf-8")
    response_path.write_text(response_text + "\n", encoding="utf-8")
    print(f"wrote {capture_path}")
    print(response_text)


if __name__ == "__main__":
    main()
