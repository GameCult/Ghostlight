# Lore Vault Entitlements

## Product decision

Ghostlight Dungeon will accept Git-synchronized, Obsidian-compatible lore
Vaults. A compatible Vault is a folder hierarchy of Markdown documents; it does
not require an Obsidian runtime or a proprietary export format. GameCult's
Aetheria and Zyphos lore sites use this shape and serve as reference corpora.

| Plan | Active custom Vaults | Combined indexed source allowance | Full imports per month |
| --- | ---: | ---: | ---: |
| Contributor | 1 | 10 million source tokens | 2 |
| Contributor Plus | 3 | 30 million source tokens | 6 |
| Private | 1 | 10 million source tokens | 2 |
| Private Plus | 3 | 30 million source tokens | 6 |

Prices, allowances, and import accounting remain provisional until service-rig
benchmarks establish ingestion, storage, and retrieval cost. Plus increases
both active Persona-cell capacity and lore capacity. Contributor versus Private
continues to select provider data policy; it does not change Vault ownership.

## Authority map

- **Owner:** the Vault service owns source checkout, parsing, chunking,
  embeddings, indexes, incremental synchronization, deletion, and retrieval.
- **Inputs:** an authorized Git repository or synchronized directory containing
  Markdown lore, its revision, and the subscriber's entitlement.
- **Outputs:** a versioned `ghostlight.vault_manifest.v1` binding and exact
  retrieval witnesses through the generic `VaultProvider` contract.
- **Derived state:** campaigns retain the bound manifest revision and exact
  evidence receipts used during play. They do not own a second semantic index.
- **Forbidden writers:** campaigns, model stages, and the browser cannot modify
  source lore or silently promote branch inventions into the Vault.
- **Shared path:** import, Git update, reindex, campaign compilation, destination
  expansion, and runtime retrieval resolve through one tenant-scoped Vault
  identity and revision.
- **Cut line:** arbitrary document upload and campaign-local vector stores do
  not become competing authorities.

## Resource accounting

The marketed Vault count is a comprehensible entitlement, not the resource
boundary. Enforcement also measures indexed source tokens, stored chunks and
embeddings, full imports, incremental document changes, and retrieval load.

Imports are content-addressed. Unchanged documents retain their chunks and
embeddings; a Git update replaces only changed document partitions. An update
counts as a full import only when Ghostlight must rebuild the whole Vault.
Oversized or unsupported imports receive a compatibility and size preview
before work begins. They do not create surprise usage charges.

Campaign Vault precedence is explicit:

```text
campaign branch facts
  > campaign-specific lore Vault
  > setting lore Vault
  > optional rules/reference Vault
```

Conflicts are reported during compilation or retrieval. Sources are not blended
into an unattributed answer.

## Product and security gates

This entitlement is a product decision, not evidence that arbitrary tenant
imports are accepted by the current tester harness. Hosted import opens only
after tenant isolation, repository authorization and revocation, deletion,
provenance, size admission, Markdown/link parsing, prompt-injection resistance,
copyright reporting, and incremental-index benchmarks are verified.
