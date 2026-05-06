# Epiphany Intrusion Audit

EpiphanyAgent touched this workspace to add the first Ghostlight-facing bridge
to Epiphany's Rust heartbeat initiative store.

Scope:

- no scene fixtures were rewritten
- no accepted narrative artifacts were mutated
- Ghostlight receives a command wrapper that invokes Epiphany's shared Rust
  heartbeat scheduler for scene participants
- initiative schema/validation now accepts scheduler-level `scene_turn`
  receipts and arena metadata emitted by that shared spine

Resident agents should treat this as an audited external intervention, not as
their own invisible local work.
