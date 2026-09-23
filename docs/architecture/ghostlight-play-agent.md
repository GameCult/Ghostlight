# Ghostlight Dungeon play agent (plan step 16)

## Status

Target, adopted 2026-09-22. Not implemented. The means will be in
`ghostlight-play-agent-cut.md`. This target is the playtest gate. It replaces
library passes L2–L4 and Dungeon passes D1–D3 on the gate's path; those are
deferred by ruling, not dropped (see "Deferred").

## Why

Ghostlight's controller machinery was built for offline narrative generation
and world simulation. It includes the cover partition, grouped cells,
grant-filtered action catalogs per subject, a separate Interpreter pass, and
the elaborator swarm. Those are the wrong compromises for interactive
role-play: today a reply takes minutes, and a player's whole vocabulary is
`world.speak`.

The operator, 2026-09-22:

> the way Ghostlight previously ran is great for offline narrative generation
> and world simulation, but it's the wrong set of compromises for an efficient
> role-playing experience. The Persona stuff is good, we want actors to think
> narratively. All the rest of it can be operated by a single operational
> agent, from dispatching and interpreting Persona turns to prompting the
> player and negotiating the result.

What makes this worth building is where consistency lives. In AI Dungeon it
lives in the context window, so it decays as the story grows. In Ghostlight it
lives in the kernel: typed state, one reducer, affordance preconditions,
outcome bands, and a digest-chained journal. None of that spends a token. A
small local model therefore does only what it is good at, which is judgment
and prose over a bounded context, while the kernel keeps the books.

## Target

A player on one local machine plays a world seeded from a Vault, turn by turn.
The reference machine is Raven's, running Bonsai 2, a Qwen 3.8 quant tuned for
agentic use, with several concurrent slots. One operational agent (the **play
agent**) runs the table: it reads the world, interprets what the player and
the Personas do, commits the consequences through the kernel, dispatches
Personas, and asks the player questions when an outcome needs negotiating.

Personas keep thinking narratively. For each Persona, the existing Projector
renders that subject's own slice of the world into lived prose. The Persona
reads only that prose and writes prose back. Neither the Projector nor the
Persona changes.

### Ownership

- **Dungeon** owns the play agent: its loop, its prompt, its tool catalog, the
  turn lifecycle, and the Eve surface. None of it enters the library
  vocabulary.
- **The library** gains only consumer-neutral pieces:
  - one author capability for a play authority;
  - retirement, and granting and revoking affordances;
  - a callable Persona turn (Projector then Persona) that does not require
    the controller runner;
  - the table vocabulary (`table.rs`: the world view, named tool subsets and
    their decoders, and refusal text), plus the public inference seams a
    consumer needs to build and read a request.

  A consumer other than Dungeon could run its own table on them. (Cut map
  PA-Q10 and PA-Q12, ruled 2026-09-22.)
- The library keeps everything the play path stops using: cover, grouped
  cells, action catalogs, the controller runner, the Interpreter, the elaborator
  sweep, and lenses. Offline consumers and simulation still own them.

### One turn

1. The player types. Their text is prose from an actor, exactly as a Persona's
   is.
2. The play agent reads a projection of world state rebuilt for this turn. It
   interprets the player's prose. Speech commits verbatim through the player
   subject's speaking affordance, and acts commit by exercising the player
   subject's affordances. The agent may instead **ask the player** something
   ("what are you offering him?"). A question commits nothing and holds the
   turn open.
3. The agent dispatches the Personas who react. They run concurrently, each
   through Projector then Persona, reading what they perceived from world
   state.
4. The agent interprets each Persona's prose under the same rules as the
   player's, and commits its own rulings for anything no actor's affordance
   covers.
5. The player sees their projection. The Projector renders the player
   subject's own slice, including what they perceived this turn, into prose,
   exactly as it does for a Persona. Beside that prose the surface shows any
   question the agent is asking.

## Invariants

1. **The kernel is the only writer, and the agent carries no world.** Every
   consequence is a library command. The agent's context each turn is built
   from world state and the current turn; no transcript is carried across
   turns. Test: restart the daemon between two turns, and the second turn's
   agent input is byte-identical to the input it would have received without
   the restart.
2. **Personas never see structured state or the agent's context.** A Persona's
   input is Projector prose. The Projector's input is that subject's own typed
   slice (`controllers.rs` `projector_context`). The agent never writes a
   Persona's input, so a secret in the agent's context cannot reach a Persona
   except through world state that subject holds. The one input every
   subject shares is the owner's world brief. It is world-public framing,
   and the owner keeps secrets out of it (cut map PA.f45). The agent never
   rewrites an existing character's persona material; it authors material
   only for characters it declares. A character's feelings follow from what
   it perceives (cut map PA.f61).
3. **An actor's act goes through that actor's affordance.** An act attributed
   to a subject, whether a Persona or the player, commits only as that
   subject's `ExerciseDecision`, with preconditions checked and the outcome
   band drawn by the kernel. The agent's raw ops are its own rulings: they are
   journaled with the play authority as author and never attributed to an
   actor. (Operator ruling, decision 1.)
4. **Speech is verbatim.** Speech committed for an actor is a literal span of
   that actor's prose, checked in code, not by prompt. This holds for the
   player's typed text and for a Persona's output. It keeps the agent from
   paraphrasing every actor into its own voice.
5. **One play authority, and only one thing holds it.** The library defines one
   consumer-neutral author capability. Its holder may:
   - declare in Active without answering elaboration demand;
   - reach the whole world, unconfined to a ground;
   - advance time;
   - state facts that stand as `Ruled` by it;
   - mint quantity without an evidence receipt.

   Only the play authority gains the last two. Elaborators, consumers and the
   seed keep the evidence gate, so a consumer feeding Njordr keeps quantity
   provenance. The operator: "if the agent can invent people, it can invent
   loot." (Cut map PA-Q15.)

   The elaborator gate is unchanged: elaborator sessions still answer a
   boundary or deficit. The agent is Dungeon's detail demand: when the player
   walks into the tavern, the agent fills it. (Operator ruling, decision 1.)
6. **Removal is retirement.** A retired subject stays in history, holds no
   opportunity, cannot be the actor of a decision, and is never dispatched.
   Replay reproduces retirement. A body that matters afterwards is declared as
   a resource.
7. **One loop, many backends.** Rust drives every tool round, as it already
   does. A local OpenAI-compatible backend returns tool calls inert, as the
   connector lane does, and adds no second loop. Routing stays per model
   name, so each lane (agent, Projector, Persona, seed) is pointed at a backend
   independently.
8. **The player's view is a projection.** The Projector renders the player's
   narration from committed state, as it does for every Persona (operator
   ruling, open question 11). Nothing the player is shown is world truth
   unless it was committed. The surface also shows the kernel's refusal of
   the player's own act, rendered from the refusal and the affordance's
   preconditions. That is the world's verdict, derived from state, not a
   claim of world truth (cut map PA-Q13). The agent's questions are the one
   thing shown that is not derived from state.

## Playtest gate

Passes when a human, on a local daemon with every play lane on a local model,
does all of the following:

1. creates a world and seeds it from a Vault;
2. activates it;
3. plays at least these:
   - speaks to a Persona and gets a reply in the Persona's own words;
   - does something physical that a precondition refuses, and sees why;
   - moves to another place;
   - is asked a question by the agent and answers it;
   - kills or removes a subject, who then never acts again;
4. restarts the daemon mid-session and continues with the world intact.

It proves one player. Hosted play, multiple players, Session Zero's Vault
choice and sliders, and detail rules are not in this gate. Seconds per turn
are **measured and recorded** on Raven's machine. The gate does not set that
number in advance, because nobody has measured Bonsai 2 on this workload yet.
The current baseline is minutes.

## Cut line

Dungeon's play path stops using:

- the owner-only "Let X act" controls and `world.controller.act`;
- automatic tick dispatch through cover cells on the play path;
- the Interpreter pass;
- the per-subject action catalogs;
- elaboration while Active.

Whether these are deleted from Dungeon or merely go unused is for the cut map
to establish, one surface at a time. Deletion is the default where nothing
else consumes the surface.

The library keeps them.

The Claude SDK sidecar stays as the cloud fallback until the local lane has
carried the playtest. It is the only live cloud provider (there is no Codex
subscription and no API budget), so "cloud fallback" means the sidecar. The
decision to delete it comes after the gate. This corrects the 2026-09-22 chat
default, which named the connector lane as the fallback.

## Deferred, by ruling (2026-09-22, decision 3)

Deferred until a consumer that still runs the offline machinery needs them:

- L1.f16, configurable lens sets (operator: yes);
- L1.f1, quarantine-class errors never quarantine;
- L1.f13, seed lane orphans rows;
- L1.f14, controller-work rows never retired;
- L2, per-world evidence binding;
- L3, detail rules;
- L4;
- Session Zero D1–D3.

After the gate, decide whether anything still consumes the controller runner,
cover and lenses. If nothing does, park them at a tag.

Exception: if the cut map finds that L1.f13 bites seeding on the playtest
path, it is in scope, because seeding is on the path.

## Open questions for the cut map

Imagination establishes each of these by probing the Body, and returns forks
with a recommendation.

1. **Where the local backend lives.** The shared seam today is the
   in-process `InferencePort` trait, not a CultNet connector API. The SDK
   sidecar is a private MessagePack child-process pipe that only borrows
   `codex_connector` input types (checked 2026-09-22). No local or
   OpenAI-compatible transport exists, so the gate has none to use yet.

   The options are a third `InferencePort` in `ghostlight` that follows the
   connector posture (~150 lines), or a connector behind a CultNet contract
   that the operator wants and that does not yet exist. The map records "a
   generic OpenAI-compatible connector is required before third parties use
   the deployment", so this pass may be that connector. The handoff says
   nothing goes into CodexConnector itself.
2. **What a Persona perceived.** Does the Projector's context already carry
   what the subject perceived since its last turn (speech heard, acts seen,
   `Seen { by }` knowledge)? If not, what is the smallest addition?
3. **A callable Persona turn.** Projector then Persona currently runs inside
   the narrative lane of `ControllerRunner`, with checkpointing. What is the
   smallest library surface the play agent can call per subject, and does it
   keep resumability?
4. **Exercising on an actor's behalf.** `ExerciseDecision` is admitted for the
   subject's controller principal. How does the play authority exercise an
   affordance for an actor it dispatched without weakening sealed admission,
   given that opportunities and digests are kernel-issued?
5. **The play authority's shape.** Name the capability, where it is granted,
   and how the clock capability's precedent maps onto it.
6. **Retirement's shape.** Is it a component op, a declaration, or a subject
   state? What do preconditions, opportunity derivation and the Projector read?
7. **The agent's catalog.** The kernel's component ops plus light declarations
   (not `declare_affordance`), plus exercise-for-actor, dispatch-Persona,
   ask-player and end-turn. Measure the strict-schema size, and say whether a
   small model plausibly handles it.
8. **The turn lifecycle.** Where does an open turn (a question outstanding, or
   Personas in flight) live, and what survives a restart (invariant 1)?
9. **The Eve play surface.** Place, co-presence, perceived speech with speaker,
   displays, the agent's question, and the player's input.
10. **The player's subject.** Does the genesis human subject at the commons
    serve, with the agent placing it after seeding?
11. **The player's prose.** Ruled 2026-09-22, operator: the player's
    perception is rendered by the same Projector a Persona gets. "Render it
    with the Projector; we're paying for it on every Persona already." The
    Projector renders only committed state, which satisfies invariant 8, and
    it is the player's narration; the play agent writes none. The cut map
    measures the extra serial call rather than deciding on it.

## Verification sketch

The cut map owns exact tests. Each invariant gets a check that fails under its
own mutation.

- **Invariant 1:** restart equivalence of the agent's turn input.
- **Invariant 2:** the Persona's request contains no field of structured state
  and none of the agent's context.
- **Invariant 3:** an act for a subject not present is refused, and a raw op
  never carries an actor attribution.
- **Invariant 4:** a paraphrased quote is refused.
- **Invariant 5:** an elaborator's declaration in Active without an answer is
  still refused, and the play authority's is admitted.
- **Invariant 6:** a retired subject gets no opportunity and no dispatch
  across a replay.
- **Invariant 7:** a scripted local backend drives a full tool round through
  the existing evaluator.
- **Road:** the playtest gate itself, on Raven's machine.


## Reconciliation, 2026-09-23

The cut map records the means and the postmortem records the cost. This section
says how the Body stands against the target above.

**Landed.** All eight invariants hold, each with a test that fails under its own
mutation, and Soul verified each at the layer where it would fail rather than in
a unit test. One operational agent drives a turn through `world.play`; the
kernel is the only writer; the player's card carries a projection, the question,
and the refusal of their own act. `world.speak` and the story card are deleted.

**Changed from the target as written.**

- An answer names its question through an opaque per-question token carried in
  the play button, not through a version comparison. The version machinery that
  preceded it is deleted. Two questions in one turn commit nothing between them,
  so no counter can tell them apart.
- The player's turn is one row keyed per world, with a request-key ledger across
  turns. A replayed key is a no-op; a stale answer and a busy turn are refused
  synchronously, before anything is spawned.
- A row the daemon cannot read is retired to a readable JSON sidecar and play
  starts fresh, reported in readiness. A row that *can* be read is never
  destroyed — that rule is stated on the function that destroys.
- The verbatim-quote check uses Unicode word boundaries (UAX #29), because the
  hand-written rule accepted "I can" out of "I can't" and refused most CJK
  quotes.

**Not built, and named as gaps rather than promises.**

- **Risk resolution.** The kernel resolves an act through an affordance's
  weighted outcome bands, drawn from a digest of the world, revision, command
  and affordance — deterministic, so a replay reproduces it. There is no
  assessment step, no modifiers, no skill and no roll receipt, and the play
  agent cannot author affordances: it may only grant and revoke the catalog's
  existing verbs. An act no granted affordance expresses never draws at all; the
  agent authors the consequence directly as the Play authority. Those two paths
  are indistinguishable from the player's chair, which is the real gap.
- **Gestalt and institution agency.** Institutions exist as structure and
  constrain who may act. Nothing makes them act, and there is no strategic tick.
  Advancing the clock rolls routines to their next due and accumulates pressure
  on overdue obligations and goals; that is the whole of what happens
  off-screen.

**Unproven.** The seed path has never run past its configuration check. No model
has run behind the local inference port. All test evidence is Windows; the
release evidence is Idunn's Linux gate on Yggdrasil, where the browser-client
tests cannot run at all, because that gate materialises source without git
metadata.
