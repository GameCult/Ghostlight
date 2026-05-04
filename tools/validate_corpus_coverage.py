"""Validate and summarize the Ghostlight corpus coverage ledger."""

from __future__ import annotations

import argparse
import json
from collections import Counter, defaultdict
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
DEFAULT_LEDGER = ROOT / "state" / "corpus-coverage.json"
VALID_SCHEMA_VERSION = "ghostlight.corpus_coverage.v0"
VALID_STATUSES = {"planned", "draft", "accepted_as_draft", "accepted", "needs_revision", "rejected", "superseded"}
ACCEPTED_STATUSES = {"accepted", "accepted_as_draft"}
VALID_TIMELINE_LANES = {"historical_grounded", "transition_grounded", "future_branch"}
VALID_CANON_STATUSES = {"single_history", "branch_local", "promoted_branch", "template", "draft"}
VALID_STORY_TYPES = {
    "founding_era_story",
    "day_in_the_life_story",
    "flashpoint_story",
    "movement_story",
    "future_branch_story",
    "trade_life_story",
}
VALID_REVIEW_STATUSES = {"not_run", "accepted", "accepted_as_draft", "needs_revision", "rejected", "blocked"}
REQUIRED_REVIEW_KEYS = {"schema", "narrative", "lore", "spatial", "visual", "if_artifact"}
REQUIRED_ARTIFACTS_FOR_ACCEPTED = {"lore_digest", "agent_state", "coordinator", "ink", "training_sidecar"}


class ValidationError(Exception):
    pass


def load_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8-sig"))


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValidationError(message)


def require_string(value: Any, path: str) -> None:
    require(isinstance(value, str) and value.strip(), f"{path} must be a non-empty string")


def require_string_array(value: Any, path: str, *, nonempty: bool = False) -> None:
    require(isinstance(value, list), f"{path} must be an array")
    require(not nonempty or bool(value), f"{path} must not be empty")
    for index, item in enumerate(value):
        require_string(item, f"{path}[{index}]")


def require_keys(obj: dict[str, Any], keys: set[str], path: str) -> None:
    missing = sorted(keys - set(obj))
    require(not missing, f"{path} missing required keys: {', '.join(missing)}")


def validate_targets(targets: Any, path: str) -> None:
    require(isinstance(targets, dict), f"{path} must be an object")
    required = {
        "major_powers",
        "minor_powers",
        "movements",
        "pre_elysium_flashpoints",
        "defunct_absorbed_powers",
        "practical_first_broad_target",
        "major_power_requirement",
        "collision_requirement",
    }
    require_keys(targets, required, path)
    for key in ["major_powers", "minor_powers", "movements", "pre_elysium_flashpoints", "defunct_absorbed_powers"]:
        require(isinstance(targets[key], int) and targets[key] >= 0, f"{path}.{key} must be a non-negative integer")
    broad = targets["practical_first_broad_target"]
    require(isinstance(broad, dict), f"{path}.practical_first_broad_target must be an object")
    require_keys(broad, {"minimum", "maximum"}, f"{path}.practical_first_broad_target")
    require(isinstance(broad["minimum"], int) and broad["minimum"] >= 0, f"{path}.practical_first_broad_target.minimum must be a non-negative integer")
    require(isinstance(broad["maximum"], int) and broad["maximum"] >= broad["minimum"], f"{path}.practical_first_broad_target.maximum must be >= minimum")
    require_string(targets["major_power_requirement"], f"{path}.major_power_requirement")
    require_string(targets["collision_requirement"], f"{path}.collision_requirement")


def validate_entry(entry: Any, path: str) -> list[str]:
    require(isinstance(entry, dict), f"{path} must be an object")
    required = {
        "fixture_id",
        "status",
        "timeline_lane",
        "story_type",
        "primary_subjects",
        "secondary_subjects",
        "collision_axes",
        "tonal_modes",
        "training_targets",
        "artifacts",
        "review",
        "economy_hooks",
        "open_lore_gaps",
        "notes",
    }
    require_keys(entry, required, path)
    require_string(entry["fixture_id"], f"{path}.fixture_id")
    require(entry["status"] in VALID_STATUSES, f"{path}.status is invalid")
    require(entry["timeline_lane"] in VALID_TIMELINE_LANES, f"{path}.timeline_lane is invalid")
    if "canon_status" in entry:
        require(entry["canon_status"] in VALID_CANON_STATUSES, f"{path}.canon_status is invalid")
    require(entry["story_type"] in VALID_STORY_TYPES, f"{path}.story_type is invalid")
    for optional in ["era", "location", "flashpoint", "notes"]:
        if optional in entry:
            require(isinstance(entry[optional], str), f"{path}.{optional} must be a string")
    require_string_array(entry["primary_subjects"], f"{path}.primary_subjects", nonempty=True)
    require_string_array(entry["secondary_subjects"], f"{path}.secondary_subjects")
    require_string_array(entry["collision_axes"], f"{path}.collision_axes", nonempty=True)
    require_string_array(entry["tonal_modes"], f"{path}.tonal_modes", nonempty=True)
    require_string_array(entry["training_targets"], f"{path}.training_targets", nonempty=True)
    require_string_array(entry["economy_hooks"], f"{path}.economy_hooks")
    require_string_array(entry["open_lore_gaps"], f"{path}.open_lore_gaps")

    artifacts = entry["artifacts"]
    require(isinstance(artifacts, dict), f"{path}.artifacts must be an object")
    for key, ref in artifacts.items():
        require_string(key, f"{path}.artifacts key")
        require_string(ref, f"{path}.artifacts.{key}")
        require((ROOT / ref).exists(), f"{path}.artifacts.{key} does not exist: {ref}")
    if entry["status"] in ACCEPTED_STATUSES:
        missing_artifacts = sorted(REQUIRED_ARTIFACTS_FOR_ACCEPTED - set(artifacts))
        require(not missing_artifacts, f"{path}.artifacts missing accepted fixture refs: {', '.join(missing_artifacts)}")

    review = entry["review"]
    require(isinstance(review, dict), f"{path}.review must be an object")
    for key, value in review.items():
        require_string(key, f"{path}.review key")
        require(value in VALID_REVIEW_STATUSES, f"{path}.review.{key} is invalid")
    if entry["status"] in ACCEPTED_STATUSES:
        missing_review = sorted(REQUIRED_REVIEW_KEYS - set(review))
        require(not missing_review, f"{path}.review missing accepted fixture review keys: {', '.join(missing_review)}")

    warnings: list[str] = []
    if entry["status"] in ACCEPTED_STATUSES:
        weak_reviews = sorted(key for key, value in review.items() if value in {"not_run", "needs_revision", "rejected", "blocked"})
        if weak_reviews:
            warnings.append(f"{entry['fixture_id']}: accepted fixture has weak review fields: {', '.join(weak_reviews)}")
        if len(set(entry["primary_subjects"] + entry["secondary_subjects"])) < 2:
            warnings.append(f"{entry['fixture_id']}: accepted fixture has fewer than two represented subjects")
    if entry["story_type"] == "future_branch_story" and entry.get("canon_status") != "branch_local":
        warnings.append(f"{entry['fixture_id']}: future_branch_story should usually be canon_status branch_local")
    return warnings


def validate_ledger(document: Any) -> tuple[list[dict[str, Any]], list[str]]:
    require(isinstance(document, dict), "ledger must be an object")
    require(document.get("schema_version") == VALID_SCHEMA_VERSION, "ledger schema_version is wrong")
    require_string(document.get("coverage_id"), "ledger.coverage_id")
    validate_targets(document.get("coverage_targets"), "ledger.coverage_targets")
    entries = document.get("entries")
    require(isinstance(entries, list), "ledger.entries must be an array")
    seen: set[str] = set()
    warnings: list[str] = []
    for index, entry in enumerate(entries):
        entry_warnings = validate_entry(entry, f"ledger.entries[{index}]")
        fixture_id = entry["fixture_id"]
        require(fixture_id not in seen, f"ledger.entries[{index}].fixture_id is duplicated: {fixture_id}")
        seen.add(fixture_id)
        warnings.extend(entry_warnings)
    return entries, warnings


def summarize(document: dict[str, Any], entries: list[dict[str, Any]], warnings: list[str]) -> str:
    targets = document["coverage_targets"]
    status_counts = Counter(entry["status"] for entry in entries)
    lane_counts = Counter(entry["timeline_lane"] for entry in entries)
    story_counts = Counter(entry["story_type"] for entry in entries)
    collision_counts = Counter(axis for entry in entries for axis in entry["collision_axes"])
    tonal_counts = Counter(mode for entry in entries for mode in entry["tonal_modes"])
    subject_counts = Counter(subject for entry in entries for subject in entry["primary_subjects"] + entry["secondary_subjects"])
    accepted_entries = [entry for entry in entries if entry["status"] in ACCEPTED_STATUSES]
    accepted_count = len(accepted_entries)
    target_min = targets["practical_first_broad_target"]["minimum"]
    target_max = targets["practical_first_broad_target"]["maximum"]

    lines = ["Ghostlight corpus coverage", ""]
    lines.append(f"Ledger: {document['coverage_id']}")
    lines.append(f"Entries: {len(entries)} total, {accepted_count} accepted/accepted-as-draft")
    lines.append(f"Practical first broad target: {target_min}-{target_max} accepted fixtures")
    if target_min:
        lines.append(f"Progress to minimum: {accepted_count}/{target_min} ({accepted_count / target_min:.1%})")
    lines.append("")

    def append_counter(title: str, counter: Counter[str]) -> None:
        lines.append(title)
        if counter:
            for key, count in counter.most_common():
                lines.append(f"- {key}: {count}")
        else:
            lines.append("- none")
        lines.append("")

    append_counter("Status", status_counts)
    append_counter("Timeline lanes", lane_counts)
    append_counter("Story types", story_counts)
    append_counter("Top collision axes", collision_counts)
    append_counter("Tonal modes", tonal_counts)
    append_counter("Subjects represented", subject_counts)

    major_story_types: dict[str, set[str]] = defaultdict(set)
    for entry in entries:
        if entry["story_type"] in {"founding_era_story", "day_in_the_life_story"}:
            for subject in entry["primary_subjects"]:
                major_story_types[subject].add(entry["story_type"])
    if major_story_types:
        lines.append("Major-power story coverage candidates")
        for subject, covered in sorted(major_story_types.items()):
            missing = sorted({"founding_era_story", "day_in_the_life_story"} - covered)
            if missing:
                lines.append(f"- {subject}: missing {', '.join(missing)}")
            else:
                lines.append(f"- {subject}: founding and day-in-life covered")
        lines.append("")

    lines.append("Gap signal")
    if accepted_count == 0:
        lines.append("- No accepted fixtures yet.")
    if accepted_count < target_min:
        lines.append(f"- Need {target_min - accepted_count} more accepted fixtures to reach the minimum broad target.")
    if not any(entry["timeline_lane"] == "future_branch" for entry in entries):
        lines.append("- No future_branch fixture coverage yet.")
    if not any(entry["story_type"] == "founding_era_story" for entry in entries):
        lines.append("- No founding-era story coverage yet.")
    if not any(entry["story_type"] == "day_in_the_life_story" for entry in entries):
        lines.append("- No day-in-the-life story coverage yet.")
    if not any(entry["economy_hooks"] for entry in entries):
        lines.append("- No economy hooks captured yet.")
    if warnings:
        lines.append("")
        lines.append("Warnings")
        for warning in warnings:
            lines.append(f"- {warning}")
    return "\n".join(lines)


def main() -> int:
    parser = argparse.ArgumentParser(description="Validate and summarize the Ghostlight corpus coverage ledger.")
    parser.add_argument("path", nargs="?", type=Path, default=DEFAULT_LEDGER, help="Coverage ledger JSON path.")
    parser.add_argument("--json", action="store_true", help="Print the validated ledger JSON instead of a text summary.")
    args = parser.parse_args()

    document = load_json(args.path)
    entries, warnings = validate_ledger(document)
    if args.json:
        print(json.dumps(document, indent=2))
    else:
        print(summarize(document, entries, warnings))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
