"""Validate GameCult PersonaState examples.

This is a focused dependency-free smoke validator. The JSON Schema remains the
canonical contract; this keeps examples honest in normal repo workflows without
dragging a validator dependency into Ghostlight.
"""

from __future__ import annotations

import argparse
import json
from datetime import datetime
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
DEFAULT_EXAMPLES = sorted((ROOT / "examples").glob("persona-state*.json"))
SCHEMA_VERSION = "gamecult.persona_state.v0"
TARGET_KINDS = {
    "person",
    "repo",
    "scene",
    "system",
    "room",
    "artifact",
    "concept",
    "relationship",
    "self",
    "community",
    "thread",
    "document",
    "runtime",
    "custom",
}


class ValidationError(Exception):
    pass


def load_json(path: Path) -> Any:
    with path.open("r", encoding="utf-8") as handle:
        return json.load(handle)


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValidationError(message)


def require_keys(obj: dict[str, Any], keys: list[str], path: str) -> None:
    missing = [key for key in keys if key not in obj]
    require(not missing, f"{path} missing required keys: {', '.join(missing)}")


def require_timestamp(value: Any, path: str) -> None:
    require(isinstance(value, str) and value, f"{path} must be a timestamp string")
    normalized = value[:-1] + "+00:00" if value.endswith("Z") else value
    try:
        datetime.fromisoformat(normalized)
    except ValueError as error:
        raise ValidationError(f"{path} must be ISO-8601 parseable: {value}") from error


def require_0_1(value: Any, path: str) -> None:
    require(isinstance(value, (int, float)), f"{path} must be numeric")
    require(0 <= value <= 1, f"{path} must be between 0 and 1")


def validate_target(obj: Any, path: str) -> None:
    require(isinstance(obj, dict), f"{path} must be an object")
    require_keys(obj, ["kind", "id"], path)
    require(obj["kind"] in TARGET_KINDS, f"{path}.kind is not a shared PersonaState target kind")
    if obj["kind"] == "custom":
        require(isinstance(obj.get("customKind"), str) and obj["customKind"], f"{path}.customKind is required for custom targets")


def validate_trait_map(obj: Any, path: str) -> None:
    require(isinstance(obj, dict), f"{path} must be an object")
    for name, vector in obj.items():
        vector_path = f"{path}.{name}"
        require(isinstance(vector, dict), f"{vector_path} must be an object")
        require_keys(vector, ["mean", "plasticity", "currentActivation"], vector_path)
        require("current_activation" not in vector, f"{vector_path}.current_activation is a native Ghostlight/VoidBot field; PersonaState uses currentActivation")
        for key in ["mean", "plasticity", "currentActivation"]:
            require_0_1(vector[key], f"{vector_path}.{key}")


def validate_activation_profile(obj: Any, path: str) -> None:
    require(isinstance(obj, dict), f"{path} must be an object")
    groups = [
        "underlyingOrganization",
        "stableDispositions",
        "behavioralDimensions",
        "presentationStrategy",
        "voiceStyle",
        "situationalState",
    ]
    require_keys(obj, groups, path)
    for group in groups:
        validate_trait_map(obj[group], f"{path}.{group}")


def validate_anchored_thought(obj: Any, path: str) -> None:
    require(isinstance(obj, dict), f"{path} must be an object")
    require_keys(obj, ["id", "status", "target", "summary", "tension", "actionImplication", "createdAt", "updatedAt"], path)
    require(obj["status"] in {"draft", "active", "cooling", "crystallized", "resolved", "retired"}, f"{path}.status is invalid")
    validate_target(obj["target"], f"{path}.target")
    require_timestamp(obj["createdAt"], f"{path}.createdAt")
    require_timestamp(obj["updatedAt"], f"{path}.updatedAt")
    if "retiredAt" in obj:
        require_timestamp(obj["retiredAt"], f"{path}.retiredAt")
    if "intensity" in obj:
        require_0_1(obj["intensity"], f"{path}.intensity")
    if "valence" in obj:
        require(isinstance(obj["valence"], (int, float)), f"{path}.valence must be numeric")
        require(-1 <= obj["valence"] <= 1, f"{path}.valence must be between -1 and 1")


def validate_candidate_action(obj: Any, path: str) -> None:
    require(isinstance(obj, dict), f"{path} must be an object")
    require_keys(obj, ["id", "status", "actionType", "readiness", "riskLevel", "target", "summary", "createdAt", "updatedAt"], path)
    validate_target(obj["target"], f"{path}.target")
    if "deliveryTarget" in obj:
        validate_target(obj["deliveryTarget"], f"{path}.deliveryTarget")
    require_timestamp(obj["createdAt"], f"{path}.createdAt")
    require_timestamp(obj["updatedAt"], f"{path}.updatedAt")
    if obj["actionType"] == "custom":
        require(isinstance(obj.get("customActionType"), str) and obj["customActionType"], f"{path}.customActionType is required for custom actions")


def validate_document(document: dict[str, Any], source: Path) -> None:
    require(document.get("schemaVersion") == SCHEMA_VERSION, f"{source} has wrong schemaVersion")
    require_keys(
        document,
        [
            "provenance",
            "personaId",
            "publicName",
            "presentation",
            "activationProfile",
            "thoughtMemory",
            "agencyPressure",
            "candidateActions",
            "affect",
            "updatedAt",
        ],
        str(source),
    )
    provenance = document["provenance"]
    require_keys(provenance, ["sourceSystem", "sourceDocumentId", "sourceUpdatedAt", "exportedAt", "authority"], f"{source}.provenance")
    require(provenance["authority"] in {"canonical", "projection", "import"}, f"{source}.provenance.authority is invalid")
    require_timestamp(provenance["sourceUpdatedAt"], f"{source}.provenance.sourceUpdatedAt")
    require_timestamp(provenance["exportedAt"], f"{source}.provenance.exportedAt")
    require_keys(document["presentation"], ["voiceSummary"], f"{source}.presentation")
    if "homeContext" in document["presentation"]:
        validate_target(document["presentation"]["homeContext"], f"{source}.presentation.homeContext")

    validate_activation_profile(document["activationProfile"], f"{source}.activationProfile")
    for group_name in ["shortTerm", "memories", "incubation"]:
        entries = document["thoughtMemory"].get(group_name)
        require(isinstance(entries, list), f"{source}.thoughtMemory.{group_name} must be an array")
        for index, thought in enumerate(entries):
            validate_anchored_thought(thought, f"{source}.thoughtMemory.{group_name}[{index}]")
    for index, pressure in enumerate(document["agencyPressure"].get("pressures", [])):
        validate_anchored_thought(pressure, f"{source}.agencyPressure.pressures[{index}]")
    for index, action in enumerate(document["candidateActions"].get("actions", [])):
        validate_candidate_action(action, f"{source}.candidateActions.actions[{index}]")
    if "voidbotProjection" in document:
        for index, action in enumerate(document["voidbotProjection"].get("candidateInterventions", [])):
            validate_candidate_action(action, f"{source}.voidbotProjection.candidateInterventions[{index}]")

    affect = document["affect"]
    for key in ["needs", "socialBonds", "statusReads", "moodDimensions", "socialBiases", "doctrineStances"]:
        require(isinstance(affect.get(key), list), f"{source}.affect.{key} must be an array")
    for index, need in enumerate(affect["needs"]):
        validate_anchored_thought(need, f"{source}.affect.needs[{index}]")
    require_timestamp(document["updatedAt"], f"{source}.updatedAt")


def main() -> int:
    parser = argparse.ArgumentParser(description="Validate GameCult PersonaState examples.")
    parser.add_argument("paths", nargs="*", type=Path, help="PersonaState JSON files to validate.")
    args = parser.parse_args()
    paths = args.paths or DEFAULT_EXAMPLES

    require(paths, "No PersonaState examples found.")
    for path in paths:
        validate_document(load_json(path), path)
        print(f"ok: {path}")

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
