---
title: Architecture
description: A public reference for Ghostlight's persistent-world simulation, agent pipeline, and authority boundaries.
---

Ghostlight is a persistent generative-agent engine. Models propose perceptions,
decisions, interpretations, and bounded worldbuilding operations; deterministic
code decides what those proposals are allowed to mean; one revisioned
`WorldKernel` commits canonical change.

This reference describes the implemented Ghostlight Dungeon body and the
consumer-neutral machinery being prepared for Delvehold. It distinguishes live
code from planned scale work. In particular, the 240-cell ceiling is an active
attention budget, not a claim that the world contains only 240 subjects. The
current full-world target is approximately 2,400 potentially acting entities
under roughly ten-percent active cover.

## Read the machine

- [[Architecture at a glance]] — the organs and the main dataflow.
- [[Authority and state]] — who is allowed to decide and persist what.
- [[Runtime pipelines]] — Session Zero, compilation, foreground turns,
  strategic simulation, elaboration, and newspapers.
- [[Agents and model stages]] — every production agent and model stage, its
  inputs, output, model class, authority, and downstream gate.

## The compact rule

```text
source evidence + canonical state
            ↓ projection
       model proposal
            ↓ deterministic validation
     typed mutation or refusal
            ↓ WorldKernel
  atomic state + event + receipt commit
```

A model response is never world truth merely because it is well written or
well typed. Receipts bind every accepted inference to its exact stage and world
snapshot. Invalid, stale, or partially illegal work leaves canonical state
unchanged.

## Scope

The public reference follows the current source in `crates/ghostlight-dungeon`.
Research plans and older fixture pipelines remain in the repository but are not
presented as live runtime stages. Physical model names are deployment choices;
the architecture names the stable logical tiers: Fast, Balanced, and Capable.

