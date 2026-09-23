# Heimdall access browser adapter provenance

- Owner: `GameCult/Heimdall`
- Plugin: `gamecult.heimdall.access`
- Source commit: `e8e2832`
- Source path: `plugins/gamecult.heimdall.access/browser-adapter.ts`
- Vendoring mode: exact semantic copy with formatting only

Ghostlight embeds this adapter so immutable releases do not depend on a sibling
checkout at browser-build time. Authentication behavior remains Heimdall-owned;
changes are imported from the owner and this provenance is advanced with them.
