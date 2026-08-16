# Vendored GameCult Rust substrate

`cultcache-rs`, `cultnet-rs`, and `cultmesh-rs` are copied together from Odin
commit `d34cb7e914e017e376d6f5fa98d464659baf1d73`. They are one compatible
protocol generation and must be updated as a set. Ghostlight vendors the set so
an immutable Starfire release can be rebuilt from its own exact Git commit.

The older CultCache revision named `cultcache-legacy` remains confined to the
campaign persistence adapter until campaign stores are migrated. It does not
own CultMesh documents or publication.
