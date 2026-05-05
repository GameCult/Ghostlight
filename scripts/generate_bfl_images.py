"""Generate images from a compact BFL prompt batch.

The script reads a JSON prompt manifest, submits each prompt to a BFL async
endpoint, polls until completion, downloads the signed result immediately, and
writes a run manifest beside the images. API key lookup order:

1. --api-key
2. BFL_API_KEY environment variable
3. E:/Projects/gamecult-ops/bfl-api.txt
"""
from __future__ import annotations

import argparse
import json
import os
import sys
import time
import urllib.error
import urllib.request
from pathlib import Path
from typing import Any

DEFAULT_KEY_PATH = Path(r"E:\Projects\gamecult-ops\bfl-api.txt")
DEFAULT_BASE_URL = "https://api.bfl.ai/v1"


def load_json(path: Path) -> dict[str, Any]:
    return json.loads(path.read_text(encoding="utf-8-sig"))


def load_api_key(cli_key: str | None) -> str:
    if cli_key:
        return cli_key.strip()
    env_key = os.environ.get("BFL_API_KEY", "").strip()
    if env_key:
        return env_key
    if DEFAULT_KEY_PATH.exists():
        return DEFAULT_KEY_PATH.read_text(encoding="utf-8-sig").strip()
    raise SystemExit("No BFL API key found. Set BFL_API_KEY or provide --api-key.")


def request_json(url: str, key: str, method: str = "GET", payload: dict[str, Any] | None = None) -> dict[str, Any]:
    data = None
    headers = {"accept": "application/json", "x-key": key}
    if payload is not None:
        data = json.dumps(payload).encode("utf-8")
        headers["Content-Type"] = "application/json"
    req = urllib.request.Request(url, data=data, headers=headers, method=method)
    try:
        with urllib.request.urlopen(req, timeout=60) as response:
            return json.loads(response.read().decode("utf-8"))
    except urllib.error.HTTPError as exc:
        body = exc.read().decode("utf-8", errors="replace")
        raise RuntimeError(f"HTTP {exc.code} from {url}: {body}") from exc


def download(url: str, dest: Path) -> None:
    req = urllib.request.Request(url, headers={"accept": "image/*"}, method="GET")
    with urllib.request.urlopen(req, timeout=120) as response:
        dest.write_bytes(response.read())


def main() -> int:
    parser = argparse.ArgumentParser(description="Generate a BFL image batch from a Ghostlight prompt manifest.")
    parser.add_argument("manifest", type=Path)
    parser.add_argument("--out-dir", type=Path, default=Path("experiments/images/bfl-run"))
    parser.add_argument("--api-key")
    parser.add_argument("--base-url", default=DEFAULT_BASE_URL)
    parser.add_argument("--model")
    parser.add_argument("--limit", type=int)
    parser.add_argument("--poll-seconds", type=float, default=1.0)
    parser.add_argument("--timeout-seconds", type=float, default=420.0)
    parser.add_argument("--seed", type=int)
    parser.add_argument("--skip-existing", action="store_true")
    parser.add_argument("--continue-on-error", action="store_true")
    parser.add_argument("--dry-run", action="store_true")
    args = parser.parse_args()

    manifest = load_json(args.manifest)
    model = args.model or manifest.get("model") or "flux-2-pro-preview"
    style_prefix = manifest.get("style_prefix", "").strip()
    width = int(manifest.get("width", 1024))
    height = int(manifest.get("height", 1024))
    output_format = manifest.get("output_format", "jpeg")
    prompts = manifest.get("prompts", [])
    if args.limit:
        prompts = prompts[: args.limit]
    if not prompts:
        raise SystemExit("Manifest contains no prompts.")

    args.out_dir.mkdir(parents=True, exist_ok=True)
    run: dict[str, Any] = {
        "source_manifest": str(args.manifest),
        "batch_id": manifest.get("batch_id"),
        "model": model,
        "width": width,
        "height": height,
        "output_format": output_format,
        "items": [],
    }

    if args.dry_run:
        for item in prompts:
            prompt = " ".join(part for part in [style_prefix, item["prompt"].strip()] if part)
            print(f"[{item['id']}] {prompt}")
        return 0

    key = load_api_key(args.api_key)
    endpoint = f"{args.base_url.rstrip('/')}/{model}"

    for index, item in enumerate(prompts, start=1):
        item_id = item["id"]
        prompt = " ".join(part for part in [style_prefix, item["prompt"].strip()] if part)
        ext = "jpg" if output_format == "jpeg" else output_format
        image_path = args.out_dir / f"{index:02d}_{item_id}.{ext}"
        if args.skip_existing and image_path.exists():
            print(f"skip existing {index}/{len(prompts)} {item_id}: {image_path}", flush=True)
            run["items"].append(
                {
                    "id": item_id,
                    "prompt": prompt,
                    "status": "SkippedExisting",
                    "image_path": str(image_path),
                }
            )
            (args.out_dir / "run-manifest.json").write_text(json.dumps(run, indent=2), encoding="utf-8")
            continue

        payload: dict[str, Any] = {
            "prompt": prompt,
            "width": width,
            "height": height,
            "output_format": output_format,
            "disable_pup": True,
            "safety_tolerance": 2,
        }
        if args.seed is not None:
            payload["seed"] = args.seed + index - 1

        print(f"submit {index}/{len(prompts)} {item_id}", flush=True)
        try:
            submitted = request_json(endpoint, key, method="POST", payload=payload)
        except Exception as exc:
            if not args.continue_on_error:
                raise
            run["items"].append({"id": item_id, "prompt": prompt, "status": "SubmitError", "error": str(exc)})
            (args.out_dir / "run-manifest.json").write_text(json.dumps(run, indent=2), encoding="utf-8")
            print(f"  {item_id}: SubmitError {exc}", flush=True)
            continue

        polling_url = submitted["polling_url"]
        start = time.monotonic()
        status = "Pending"
        result: dict[str, Any] | None = None
        while time.monotonic() - start < args.timeout_seconds:
            time.sleep(args.poll_seconds)
            polled = request_json(polling_url, key)
            status = polled.get("status", "Unknown")
            print(f"  {item_id}: {status}", flush=True)
            if status == "Ready":
                result = polled
                break
            if status == "Content Moderated":
                run["items"].append(
                    {
                        "id": item_id,
                        "prompt": prompt,
                        "request_id": submitted.get("id"),
                        "polling_url": polling_url,
                        "status": status,
                        "error": json.dumps(polled),
                    }
                )
                (args.out_dir / "run-manifest.json").write_text(json.dumps(run, indent=2), encoding="utf-8")
                if args.continue_on_error:
                    result = None
                    break
                raise RuntimeError(f"Generation moderated for {item_id}: {json.dumps(polled, indent=2)}")
            if status in {"Error", "Failed"}:
                if args.continue_on_error:
                    run["items"].append(
                        {
                            "id": item_id,
                            "prompt": prompt,
                            "request_id": submitted.get("id"),
                            "polling_url": polling_url,
                            "status": status,
                            "error": json.dumps(polled),
                        }
                    )
                    (args.out_dir / "run-manifest.json").write_text(json.dumps(run, indent=2), encoding="utf-8")
                    break
                raise RuntimeError(f"Generation failed for {item_id}: {json.dumps(polled, indent=2)}")
        if result is None:
            if args.continue_on_error and status in {"Content Moderated", "Error", "Failed"}:
                continue
            raise TimeoutError(f"Timed out waiting for {item_id}; last status {status}")

        sample_url = result["result"]["sample"]
        download(sample_url, image_path)
        run["items"].append(
            {
                "id": item_id,
                "prompt": prompt,
                "request_id": submitted.get("id"),
                "polling_url": polling_url,
                "status": status,
                "image_path": str(image_path),
                "cost": submitted.get("cost"),
            }
        )
        (args.out_dir / "run-manifest.json").write_text(json.dumps(run, indent=2), encoding="utf-8")
        print(f"  saved {image_path}", flush=True)

    print(f"wrote {(args.out_dir / 'run-manifest.json')}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
