# Persistence Spine

Ghostlight's machine-managed state is moving behind a small CultCache-style
persistence layer instead of letting every tool paw raw files directly.

The Python port lives in `vendor/cultcache-py`, backed by the standalone
GameCult repository `cultcache-py`.

## Current Store

- Cache store: `state/ghostlight-state.cultcache.jsonl`
- Ghostlight seam: `tools/ghostlight_state_store.py`
- Library submodule: `vendor/cultcache-py`
- Readable exports: `state/branches.json`, `state/evidence.jsonl`

The JSONL store is deliberate bootstrap plumbing. The package also exposes a
MessagePack store for environments that install the optional `msgpack`
dependency, but Ghostlight's state CLI must run on the bundled Python runtime
without global package assumptions.

## Live Rule

New machine-managed state should go through registered document definitions in
`tools/ghostlight_state_store.py` or a successor module. Direct writes to
`state/branches.json` and `state/evidence.jsonl` are compatibility debt.

Human-readable exports remain useful because git diffs, handoff review, and
emergency recovery should not require decoding a cache file while the house is
on fire.

## Migrated Documents

- `ghostlight.branches.v0`
  - global document
  - exported to `state/branches.json`
- `ghostlight.evidence_record.v0`
  - one record per evidence entry
  - indexed by `type`, `status`, and `branch`
  - exported to `state/evidence.jsonl`

## Expansion Path

Move compact control-plane state next:

- corpus coverage ledger metadata
- active fixture run manifests
- persistent reviewer receipt indexes
- compaction scratch registration
- branch and initiative run ledgers

Keep large corpora, generated artifacts, images, and full story payloads outside
the single state store unless they are represented by compact refs. A cache
file is not a warehouse just because it has pockets.
