# World elaboration: review record and idea index

Proposals from the September 2026 elaborator swarm for the four setting vaults
(AetheriaLore, Delvehold, Kalsa, Zyphos). **None of this is canon.** Integrating
ideas into a vault is an editorial pass the operator makes by hand, one idea at
a time; nothing here is to be merged by script or treated as established lore.

## What is here

- `review/world-additions-review.md` — the reviewed additions packet and its
  decisions: **A2 and D1–D3 are approved for integration** (D3's divinity
  formalization is still owed), **A1 is demoted**, and **K1/K2/Z1–Z3 are
  pending**. Start here.
- `review/review-<world>.md`, `review/review-revisions.md`,
  `review/review-sources.md` — the per-world editorial texts and revisions
  behind that packet.
- `review/elaboration-shortlist.md` — 20 reviewer-promoted candidates from the
  2026-09-05 campaign, with links to their full texts in `review/candidates/`.
  Shortlisting is not approval.
- `review/elaboration-wave-00N-review.md` — the comparative review of each
  sixteen-idea wave; `review/elaboration-completion.md` and
  `review/elaboration-progress.md` — campaign summary and limits;
  `review/world-elaboration-evaluation.md` — evaluation of the approach.
- `<world>/ledger/*.md` — one ledger per finished 2026-09-04 pass (92 in total):
  the idea in two sentences, where it would live in the vault, what it attaches
  to, who benefits and who pays, and the hooks it leaves. The Idea lines are the
  index.

## Where the vault text is

Each pass wrote its proposed vault addition on a `slot/<world>/<title>/<stamp>`
branch in the lore repository. Those branches, the `codex/world-<world>`
branches, and the lore repos' `codex/ghostlight-worlds` branches were removed
from GitHub on 2026-09-13 after verified bundling. The bundles are on the
operator workstation at
`F:\Projects\gamecult-ops\.artifacts\ghostlight-world-branches-2026-09-13\`
(`<Repo>-swarm.bundle`, with every branch and tip in `<Repo>-swarm-refs.txt`).
They are a single local copy.

To bring one pass back for integration:

```bash
git -C F:/Projects/<LoreRepo> fetch F:/Projects/gamecult-ops/.artifacts/ghostlight-world-branches-2026-09-13/<LoreRepo>-swarm.bundle refs/remotes/origin/slot/<world>/<title>/<stamp>:refs/heads/slot/<world>/<title>/<stamp>
```

The 2026-09-05 shortlist candidates are also complete in `review/candidates/`,
so reviewing those does not need a bundle.
