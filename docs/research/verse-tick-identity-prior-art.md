# Research note: where a turn's identity lives (verse-run re-map, 2026-10-11)

Imagination `imagination-gl-tick-identity`, session `self-2026-10-11-ag`. A short
prior-art check before settling tick identity for cut `verse-run` r2. Appended to the
map's "Verse run identity" section.

## The question

At Ghostlight `06a8373` a tick is named `now / span` (`cover.rs:46`), the span is not in
the name, and every cell id and command id is re-derived from the opportunities,
attention order, span and cover budget the resuming run holds at that moment
(`run.rs:557-565`). Soul s1 showed a different span or budget on resume yields new ids,
so a subject acts twice in one tick (p1, p2), and a different span reuses a closed
tick's index (p4).

## Prior art

- **Durable workflow engines (Temporal, Azure Durable Functions).** The identity of
  each step is assigned when the step is first scheduled and recorded in the
  execution's event history. On resume the code is replayed against that history and
  every command it produces is compared with the recorded one; a difference is a
  non-determinism error, not a second execution
  ([Temporal: execution failures](https://docs.temporal.io/troubleshooting/execution-failures);
  [community thread](https://community.temporal.io/t/non-determinism-error/8279)).
  The forum cases are exactly Soul's: a timer duration or a child id computed
  differently on replay. The lesson: resume reads the recorded decision; it does not
  re-derive it from inputs that may have changed.
- **Deterministic lockstep games.** The turn (frame) number is simulation state,
  advanced only by the simulation, and every command is tagged with the turn it runs
  in, so all peers and every replay agree on which turn a command belongs to whatever
  the wall-clock or turn length
  ([Unity discussion](https://discussions.unity.com/t/deterministic-rts-game-engine/467205);
  [Gaffer On Games](https://gafferongames.com/post/what_every_programmer_needs_to_know_about_game_networking/)).
  The turn count, not elapsed time divided by turn length, is the name.
- **Idempotency keys (event-sourced command handlers, payment APIs).** A key answers a
  retry with the stored result; the same key with a different request is refused, not
  re-executed. Ghostlight's kernel already does this
  (`lib.rs:1633-1644`: same caller and body, `AlreadyApplied`; otherwise
  `CommandIdConflict`), which is why Soul's p4 clock under another span was refused.

## What this decides for verse-run

1. The tick ordinal is world-held: the number of `TimeAdvanced` commits in the world's
   journal when the tick opens. It is span-independent, monotone, injective per world,
   restored with a world snapshot (so ruling `replay-restored-snapshot` needs nothing
   else for a run that starts on a tick boundary), and consecutive (which is what the
   cover's rotation window wants, `cover.rs:340`).
2. What a tick decided when it opened (span, cover budget, the partition and each
   cell's id and members) is recorded once, before the first cell runs, in a tick
   document owned by the run store, and a resuming run reads it instead of deriving a
   cover. This is Temporal's history rule at the granularity the verse needs.
3. Command ids stay derived, but only from that frozen identity (world, ordinal, cell
   id, subject), never from live inputs, so there is no second copy of them to drift.
4. The controller answers a persisted row whose command the world committed with its
   receipt whatever the caller's current scope digest, because the command id is the
   idempotency key; a row still in flight keeps the digest check, so no tokens are spent
   on a scope that moved.

Not adopted: a world-state tick counter (a `STATE_SCHEMA` change, and the kernel would
learn ticks exist, which `cover.rs:1-12` forbids); a run-store-only ordinal (a replay
over a restored world with a fresh run store would number ticks from 1 and diverge).
