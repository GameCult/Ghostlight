"""Generate Ghostlight images through the Black Forest Labs API.

The script can submit a plain prompt or assemble prompts from a Ghostlight
`.visual.json` plan. It writes each downloaded image beside a metadata receipt
that preserves the prompt, request id, polling result, and source scene.
"""

from __future__ import annotations

import argparse
import base64
import json
import mimetypes
import os
import re
import sys
import time
import urllib.error
import urllib.parse
import urllib.request
from dataclasses import dataclass
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
DEFAULT_ENDPOINT = "https://api.bfl.ai/v1/flux-2-pro-preview"
DEFAULT_KEY_FILE = Path("E:/Projects/gamecult-ops/bfl-api.txt")
DEFAULT_OUTPUT_DIR = ROOT / "experiments" / "images" / "bfl"
READY_STATUSES = {"Ready"}
FAILED_STATUSES = {"Error", "Failed", "Request Moderated", "Content Moderated"}


@dataclass(frozen=True)
class RenderJob:
    render_id: str
    prompt: str
    source_ref: str
    scene_id: str | None = None
    modifier_ids: tuple[str, ...] = ()


def load_json(path: Path) -> dict[str, Any]:
    return json.loads(path.read_text(encoding="utf-8"))


def write_json(path: Path, data: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(data, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")


def display_path(path: Path) -> str:
    try:
        return str(path.resolve().relative_to(ROOT))
    except ValueError:
        return str(path)


def slugify(value: str) -> str:
    slug = re.sub(r"[^A-Za-z0-9._-]+", "-", value.strip()).strip("-")
    return slug[:120] or "image"


def read_prompt(args: argparse.Namespace) -> str | None:
    if args.prompt and args.prompt_file:
        raise SystemExit("Use either --prompt or --prompt-file, not both.")
    if args.prompt:
        return args.prompt
    if args.prompt_file:
        return Path(args.prompt_file).read_text(encoding="utf-8").strip()
    return None


def resolve_api_key(args: argparse.Namespace) -> str:
    if args.api_key:
        return args.api_key.strip()
    env_value = os.environ.get("BFL_API_KEY", "").strip()
    if env_value:
        return env_value

    key_file = Path(args.api_key_file) if args.api_key_file else DEFAULT_KEY_FILE
    if key_file.exists():
        return key_file.read_text(encoding="utf-8").strip()

    raise SystemExit(
        "Missing BFL API key. Set BFL_API_KEY or pass --api-key-file. "
        f"Default key file was not found: {key_file}"
    )


def encode_input_image(value: str) -> str:
    """Return URL/base64 value acceptable to BFL."""

    parsed = urllib.parse.urlparse(value)
    if parsed.scheme in {"http", "https"}:
        return value

    path = Path(value)
    if not path.exists():
        raise SystemExit(f"Input image does not exist: {path}")
    return base64.b64encode(path.read_bytes()).decode("ascii")


def visual_ref_from_path(data: dict[str, Any], ref: str) -> dict[str, Any] | None:
    prefix = "visual_character_refs.characters."
    if not ref.startswith(prefix):
        return None
    character_id = ref.removeprefix(prefix)
    return data.get("visual_character_refs", {}).get("characters", {}).get(character_id)


def format_character_ref(agent_id: str, stance: str, ref: dict[str, Any] | None) -> str:
    if not ref:
        return f"- {agent_id}: {stance}"
    display = ref.get("display_name", agent_id)
    description = ref.get("stable_visual_description", "").strip()
    silhouette = ref.get("silhouette_notes", "").strip()
    parts = [f"- {display}: {description}"]
    if silhouette:
        parts.append(f"Silhouette/pose continuity: {silhouette}")
    if stance:
        parts.append(f"Scene stance: {stance}")
    return " ".join(part for part in parts if part)


def selected_scene_modifiers(scene: dict[str, Any], modifier_ids: set[str]) -> list[tuple[str, str]]:
    modifiers: list[tuple[str, str]] = []
    for index, modifier in enumerate(scene.get("state_image_modifiers", []), start=1):
        modifier_id = modifier.get("modifier_id") or f"{scene.get('visual_scene_id')}.state_modifier_{index}"
        if modifier_id in modifier_ids:
            modifiers.append((modifier_id, modifier.get("prompt", "").strip()))
    return modifiers


def selected_global_modifiers(
    scene_id: str,
    plan: dict[str, Any],
    modifier_ids: set[str],
) -> list[tuple[str, str]]:
    modifiers: list[tuple[str, str]] = []
    for modifier in plan.get("global_branch_image_modifiers", []):
        modifier_id = modifier.get("modifier_id")
        apply_to = set(modifier.get("apply_to_visual_scene_ids", []))
        if modifier_id in modifier_ids and scene_id in apply_to:
            modifiers.append((modifier_id, modifier.get("prompt", "").strip()))
    return modifiers


def assemble_visual_prompt(
    data: dict[str, Any],
    scene: dict[str, Any],
    scene_modifier_ids: set[str],
    global_modifier_ids: set[str],
) -> tuple[str, tuple[str, ...]]:
    plan = data.get("visual_scene_plan", {})
    scene_id = scene["visual_scene_id"]
    sections: list[str] = []

    style = plan.get("global_style_cue", "").strip()
    if style:
        sections.append(style)

    character_lines: list[str] = []
    for character in scene.get("visible_characters", []):
        ref = visual_ref_from_path(data, character.get("visual_ref", ""))
        agent_id = character.get("agent_id", "unknown")
        if agent_id == "none_named" and not ref:
            continue
        character_lines.append(
            format_character_ref(agent_id, character.get("default_stance", ""), ref)
        )
    if character_lines:
        sections.append("Visible character references:\n" + "\n".join(character_lines))

    sections.append("Scene base image:\n" + scene.get("base_image_prompt", "").strip())

    applied: list[tuple[str, str]] = []
    applied.extend(selected_scene_modifiers(scene, scene_modifier_ids))
    applied.extend(selected_global_modifiers(scene_id, plan, global_modifier_ids))
    if applied:
        sections.append(
            "Apply these additive visible state changes:\n"
            + "\n".join(f"- {modifier_id}: {prompt}" for modifier_id, prompt in applied if prompt)
        )

    return "\n\n".join(section for section in sections if section.strip()), tuple(
        modifier_id for modifier_id, _ in applied
    )


def build_visual_jobs(args: argparse.Namespace) -> list[RenderJob]:
    data = load_json(Path(args.visual_plan))
    plan = data.get("visual_scene_plan", {})
    scenes = plan.get("scenes", [])
    if not scenes:
        raise SystemExit(f"No visual_scene_plan.scenes found in {args.visual_plan}")

    scene_ids = set(args.scene_id or [])
    if scene_ids:
        selected = [scene for scene in scenes if scene.get("visual_scene_id") in scene_ids]
        missing = scene_ids - {scene.get("visual_scene_id") for scene in selected}
        if missing:
            raise SystemExit(f"Unknown visual_scene_id(s): {', '.join(sorted(missing))}")
    elif args.all_scenes:
        selected = scenes
    else:
        raise SystemExit("Use --scene-id or --all-scenes with --visual-plan.")

    scene_modifier_ids = set(args.scene_modifier_id or [])
    global_modifier_ids = set(args.global_modifier_id or [])
    visual_plan_id = data.get("visual_plan_id", Path(args.visual_plan).stem)
    jobs: list[RenderJob] = []
    for scene in selected:
        scene_id = scene["visual_scene_id"]
        prompt, applied = assemble_visual_prompt(data, scene, scene_modifier_ids, global_modifier_ids)
        suffix = "-".join(slugify(item) for item in applied)
        render_id = slugify(f"{visual_plan_id}-{scene_id}" + (f"-{suffix}" if suffix else ""))
        jobs.append(
            RenderJob(
                render_id=render_id,
                prompt=prompt,
                source_ref=f"{args.visual_plan}#{scene_id}",
                scene_id=scene_id,
                modifier_ids=applied,
            )
        )
    return jobs


def build_jobs(args: argparse.Namespace) -> list[RenderJob]:
    prompt = read_prompt(args)
    if prompt and args.visual_plan:
        raise SystemExit("Use either direct prompt input or --visual-plan, not both.")
    if prompt:
        return [RenderJob(render_id=slugify(args.render_id or "bfl-prompt"), prompt=prompt, source_ref="direct_prompt")]
    if args.visual_plan:
        return build_visual_jobs(args)
    raise SystemExit("Provide --prompt, --prompt-file, or --visual-plan.")


def request_json(url: str, api_key: str, method: str = "GET", payload: dict[str, Any] | None = None) -> dict[str, Any]:
    data = None
    headers = {"accept": "application/json", "x-key": api_key}
    if payload is not None:
        data = json.dumps(payload).encode("utf-8")
        headers["Content-Type"] = "application/json"
    request = urllib.request.Request(url, data=data, headers=headers, method=method)
    try:
        with urllib.request.urlopen(request, timeout=60) as response:
            return json.loads(response.read().decode("utf-8"))
    except urllib.error.HTTPError as error:
        body = error.read().decode("utf-8", errors="replace")
        raise RuntimeError(f"BFL HTTP {error.code} from {url}: {body}") from error


def download_file(url: str, output_path: Path) -> None:
    output_path.parent.mkdir(parents=True, exist_ok=True)
    request = urllib.request.Request(url, headers={"User-Agent": "Ghostlight-BFL-Script/0.1"})
    with urllib.request.urlopen(request, timeout=120) as response:
        output_path.write_bytes(response.read())


def build_payload(args: argparse.Namespace, prompt: str) -> dict[str, Any]:
    payload: dict[str, Any] = {
        "prompt": prompt,
        "output_format": args.output_format,
        "safety_tolerance": args.safety_tolerance,
    }
    if args.width is not None:
        payload["width"] = args.width
    if args.height is not None:
        payload["height"] = args.height
    if args.seed is not None:
        payload["seed"] = args.seed
    if args.disable_pup is not None:
        payload["disable_pup"] = args.disable_pup
    if args.webhook_url:
        payload["webhook_url"] = args.webhook_url
    if args.webhook_secret:
        payload["webhook_secret"] = args.webhook_secret
    for index, value in enumerate(args.input_image or [], start=1):
        key = "input_image" if index == 1 else f"input_image_{index}"
        payload[key] = encode_input_image(value)
    return payload


def poll_until_ready(
    polling_url: str,
    api_key: str,
    interval: float,
    timeout: float,
) -> dict[str, Any]:
    start = time.monotonic()
    while True:
        result = request_json(polling_url, api_key)
        status = result.get("status")
        progress = result.get("progress")
        print(f"status={status}" + (f" progress={progress}" if progress is not None else ""))
        if status in READY_STATUSES:
            return result
        if status in FAILED_STATUSES:
            raise RuntimeError(f"BFL generation failed: {json.dumps(result, ensure_ascii=False)}")
        if time.monotonic() - start > timeout:
            raise TimeoutError(f"Timed out after {timeout:.0f}s waiting for BFL result: {polling_url}")
        time.sleep(interval)


def extension_for(args: argparse.Namespace, sample_url: str | None) -> str:
    if args.output_format:
        return "jpg" if args.output_format == "jpeg" else args.output_format
    if sample_url:
        suffix = Path(urllib.parse.urlparse(sample_url).path).suffix.lstrip(".")
        if suffix:
            return suffix
    guessed = mimetypes.guess_extension(f"image/{args.output_format}") or ".png"
    return guessed.lstrip(".")


def run_job(args: argparse.Namespace, api_key: str | None, job: RenderJob, index: int, total: int) -> None:
    output_dir = Path(args.output_dir)
    output_dir.mkdir(parents=True, exist_ok=True)
    render_base = output_dir / f"{index:03d}-{job.render_id}"
    payload = build_payload(args, job.prompt)

    metadata: dict[str, Any] = {
        "render_id": job.render_id,
        "source_ref": job.source_ref,
        "scene_id": job.scene_id,
        "modifier_ids": list(job.modifier_ids),
        "endpoint": args.endpoint,
        "request_payload": payload,
    }

    prompt_path = render_base.with_suffix(".prompt.txt")
    prompt_path.write_text(job.prompt + "\n", encoding="utf-8")

    if args.dry_run:
        metadata["dry_run"] = True
        write_json(render_base.with_suffix(".metadata.json"), metadata)
        print(f"[{index}/{total}] dry-run wrote {display_path(prompt_path)}")
        return

    if api_key is None:
        raise SystemExit("Internal error: api_key missing outside dry-run mode.")

    print(f"[{index}/{total}] submitting {job.render_id}")
    submit = request_json(args.endpoint, api_key, method="POST", payload=payload)
    metadata["submit_response"] = submit
    polling_url = submit.get("polling_url")
    if not polling_url:
        raise RuntimeError(f"BFL response did not include polling_url: {json.dumps(submit)}")

    result = poll_until_ready(polling_url, api_key, args.poll_interval, args.timeout)
    metadata["poll_result"] = result
    sample_url = (result.get("result") or {}).get("sample")
    if not sample_url:
        raise RuntimeError(f"BFL ready result did not include result.sample: {json.dumps(result)}")

    image_path = render_base.with_suffix("." + extension_for(args, sample_url))
    download_file(sample_url, image_path)
    metadata["downloaded_image"] = display_path(image_path)
    write_json(render_base.with_suffix(".metadata.json"), metadata)
    print(f"[{index}/{total}] wrote {display_path(image_path)}")


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    input_group = parser.add_argument_group("prompt source")
    input_group.add_argument("--prompt", help="Direct prompt text.")
    input_group.add_argument("--prompt-file", type=Path, help="Text file containing one prompt.")
    input_group.add_argument("--visual-plan", type=Path, help="Ghostlight .visual.json plan to render from.")
    input_group.add_argument("--scene-id", action="append", help="Visual scene id to render. Repeat for multiple scenes.")
    input_group.add_argument("--all-scenes", action="store_true", help="Render every base scene from --visual-plan.")
    input_group.add_argument("--scene-modifier-id", action="append", help="Apply a scene-local state modifier id.")
    input_group.add_argument("--global-modifier-id", action="append", help="Apply a global branch image modifier id.")
    input_group.add_argument("--render-id", help="Output slug for direct-prompt renders.")

    api_group = parser.add_argument_group("BFL API")
    api_group.add_argument("--endpoint", default=DEFAULT_ENDPOINT, help=f"BFL endpoint. Default: {DEFAULT_ENDPOINT}")
    api_group.add_argument("--api-key", help="BFL API key. Prefer BFL_API_KEY or --api-key-file.")
    api_group.add_argument("--api-key-file", type=Path, help=f"File containing BFL API key. Default fallback: {DEFAULT_KEY_FILE}")
    api_group.add_argument("--dry-run", action="store_true", help="Assemble prompts and metadata without calling BFL.")
    api_group.add_argument("--yes", action="store_true", help="Confirm paid API calls, especially multi-image batches.")
    api_group.add_argument("--poll-interval", type=float, default=1.0, help="Seconds between polling attempts.")
    api_group.add_argument("--timeout", type=float, default=300.0, help="Seconds before polling gives up.")

    model_group = parser.add_argument_group("model parameters")
    model_group.add_argument("--width", type=int, help="Output width. FLUX.2 supports multiples of 16.")
    model_group.add_argument("--height", type=int, help="Output height. FLUX.2 supports multiples of 16.")
    model_group.add_argument("--seed", type=int, help="Optional seed.")
    model_group.add_argument("--safety-tolerance", type=int, default=2, help="BFL moderation tolerance.")
    model_group.add_argument("--output-format", choices=["jpeg", "png"], default="png")
    model_group.add_argument("--disable-pup", dest="disable_pup", action="store_true", help="Disable prompt upsampling.")
    model_group.add_argument("--enable-pup", dest="disable_pup", action="store_false", help="Enable prompt upsampling.")
    model_group.set_defaults(disable_pup=None)
    model_group.add_argument("--input-image", action="append", help="Input/reference image URL or local path. Repeat up to 8 times.")
    model_group.add_argument("--webhook-url")
    model_group.add_argument("--webhook-secret")

    parser.add_argument("--output-dir", type=Path, default=DEFAULT_OUTPUT_DIR)
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    jobs = build_jobs(args)
    if len(jobs) > 1 and not args.dry_run and not args.yes:
        raise SystemExit(f"Refusing to submit {len(jobs)} paid image jobs without --yes.")
    api_key = None if args.dry_run else resolve_api_key(args)

    for index, job in enumerate(jobs, start=1):
        run_job(args, api_key, job, index, len(jobs))
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except KeyboardInterrupt:
        print("Interrupted.", file=sys.stderr)
        raise SystemExit(130)
