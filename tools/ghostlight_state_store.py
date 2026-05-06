from __future__ import annotations

import hashlib
import json
import sys
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
STATE_DIR = ROOT / "state"
VENDOR_SRC = ROOT / "vendor" / "cultcache-py" / "src"
CACHE_PATH = STATE_DIR / "ghostlight-state.cultcache.jsonl"
BRANCHES_PATH = STATE_DIR / "branches.json"
EVIDENCE_PATH = STATE_DIR / "evidence.jsonl"

if str(VENDOR_SRC) not in sys.path:
    sys.path.insert(0, str(VENDOR_SRC))

from cultcache_py import CultCache, JsonLinesBackingStore, define_document_type


BRANCHES_DOC = define_document_type(
    "ghostlight.branches.v0",
    global_document=True,
)

EVIDENCE_DOC = define_document_type(
    "ghostlight.evidence_record.v0",
    indexes={
        "type": "type",
        "status": "status",
        "branch": "branch",
    },
)


def _read_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8-sig"))


def _write_json(path: Path, value: Any) -> None:
    path.write_text(json.dumps(value, indent=2, ensure_ascii=True) + "\n", encoding="utf-8")


def _read_jsonl(path: Path) -> list[dict[str, Any]]:
    if not path.exists():
        return []
    records: list[dict[str, Any]] = []
    for line in path.read_text(encoding="utf-8-sig").splitlines():
        if line.strip():
            records.append(json.loads(line))
    return records


def _write_jsonl(path: Path, records: list[dict[str, Any]]) -> None:
    content = "".join(json.dumps(record, ensure_ascii=True) + "\n" for record in records)
    path.write_text(content, encoding="utf-8")


def _evidence_key(record: dict[str, Any], index: int = 0) -> str:
    payload = json.dumps(record, ensure_ascii=True, sort_keys=True)
    digest = hashlib.sha1(payload.encode("utf-8")).hexdigest()[:12]
    ts = str(record.get("ts", "unstamped")).replace(":", "").replace("+", "Z")
    return f"{ts}-{index:04d}-{digest}"


def open_state_cache() -> CultCache:
    cache = (
        CultCache.builder()
        .register_document_type(BRANCHES_DOC)
        .register_document_type(EVIDENCE_DOC)
        .add_generic_store(JsonLinesBackingStore(CACHE_PATH))
        .build()
    )
    cache.pull_all_backing_stores()
    _bootstrap_if_needed(cache)
    return cache


def load_branches() -> dict[str, Any]:
    cache = open_state_cache()
    branches = cache.get_global(BRANCHES_DOC)
    if branches is None:
        return {"branches": []}
    return branches


def save_branches(data: dict[str, Any]) -> None:
    cache = open_state_cache()
    cache.put_global(BRANCHES_DOC, data)
    _write_json(BRANCHES_PATH, data)


def append_evidence(record: dict[str, Any]) -> None:
    cache = open_state_cache()
    existing = cache.get_all(EVIDENCE_DOC)
    key = _evidence_key(record, len(existing))
    cache.put(EVIDENCE_DOC, key, record)
    _export_evidence(cache)


def _bootstrap_if_needed(cache: CultCache) -> None:
    changed = False
    if cache.get_global(BRANCHES_DOC) is None and BRANCHES_PATH.exists():
        cache.put_global(BRANCHES_DOC, _read_json(BRANCHES_PATH))
        changed = True
    if not cache.get_all(EVIDENCE_DOC):
        for index, record in enumerate(_read_jsonl(EVIDENCE_PATH)):
            cache.put(EVIDENCE_DOC, _evidence_key(record, index), record)
            changed = True
    if changed:
        _export_evidence(cache)
        branches = cache.get_global(BRANCHES_DOC)
        if branches is not None:
            _write_json(BRANCHES_PATH, branches)


def _export_evidence(cache: CultCache) -> None:
    records = sorted(cache.get_all(EVIDENCE_DOC), key=lambda record: record.get("ts", ""))
    _write_jsonl(EVIDENCE_PATH, records)
