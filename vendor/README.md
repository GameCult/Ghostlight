# Vendored GameCult Rust substrate

`cultcache-rs`, `cultnet-rs`, and `cultmesh-rs` are a locally owned compatible
protocol generation. Ghostlight vendors the set so an immutable release can be
rebuilt from its exact Git commit.

The vendored CultCache crate owns both the snapshot store and the pinned redb
store used by application sessions, the world journal, and controller-work
custody. Its redb table, key, and envelope encoding remain compatible with the
existing Ghostlight stores. There is no second CultCache package or persistence
authority in the workspace.
