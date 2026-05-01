"""Validate saved Qwen response captures.

Capture artifacts are not canonical state, but they are training receipts. This
validator keeps them from quietly becoming a drawer full of unreviewed vibes.
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
DEFAULT_CAPTURES = sorted((ROOT / "experiments" / "cold-wake-story-lab").glob("*.capture.json"))
VALID_REVIEW_STATUSES = {"accepted", "rejected", "useful_needs_revision", "needs_revision"}
VALID_RESPONSE_KINDS = {"speech", "action", "silence", "withdrawal", "violence", "mixed", "unknown"}


class ValidationError(Exception):
    pass


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValidationError(message)


def require_keys(obj: dict[str, Any], keys: list[str], path: str) -> None:
    missing = [key for key in keys if key not in obj]
    require(not missing, f"{path} missing required keys: {', '.join(missing)}")


def validate_string_list(value: Any, path: str) -> None:
    require(isinstance(value, list), f"{path} must be an array")
    for index, item in enumerate(value):
        require(isinstance(item, str), f"{path}[{index}] must be a string")


def validate_capture(path: Path) -> None:
    data = json.loads(path.read_text(encoding="utf-8"))
    require(isinstance(data, dict), f"{path} must be an object")
    require_keys(
        data,
        [
            "schema_version",
            "capture_id",
            "projection_example_id",
            "model",
            "endpoint",
            "request_options",
            "response_text",
            "review",
        ],
        str(path),
    )
    require(data["schema_version"] == "ghostlight.qwen_response_capture.v0", f"{path}.schema_version is wrong")
    require(isinstance(data["response_text"], str) and data["response_text"].strip(), f"{path}.response_text must be non-empty")

    review = data["review"]
    require(isinstance(review, dict), f"{path}.review must be an object")
    require_keys(review, ["status", "response_kind_observed", "strengths", "failure_notes", "pipeline_lessons"], f"{path}.review")
    require(review["status"] in VALID_REVIEW_STATUSES, f"{path}.review.status is invalid or unreviewed")
    require(review["response_kind_observed"] in VALID_RESPONSE_KINDS, f"{path}.review.response_kind_observed is invalid")
    validate_string_list(review["strengths"], f"{path}.review.strengths")
    validate_string_list(review["failure_notes"], f"{path}.review.failure_notes")
    validate_string_list(review["pipeline_lessons"], f"{path}.review.pipeline_lessons")


def main() -> int:
    parser = argparse.ArgumentParser(description="Validate Ghostlight Qwen response capture files.")
    parser.add_argument("paths", nargs="*", type=Path, help="Capture JSON files to validate.")
    args = parser.parse_args()

    paths = args.paths or DEFAULT_CAPTURES
    require(paths, "no capture files found")
    for path in paths:
        validate_capture(path)
        print(f"ok: {path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
