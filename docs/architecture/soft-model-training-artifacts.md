# Soft Model Training Artifacts

Ghostlight has two kinds of machinery.

Some parts should be boring code:

- schema validation
- visibility filtering
- action legality
- object custody
- resource accounting
- Ink syntax validation and compilation
- event append
- provenance checks
- deterministic context slicing
- mechanical mutation gates

Other parts are soft. They require judgment, taste, social inference, cultural
pressure, and narrative sense. Do not fake those with brittle rules just because
a function-shaped hole is sitting there looking smug.

Soft parts should emit reviewed training artifacts from the beginning. Manual
review is not a temporary embarrassment. It is the dataset.

## Core Rule

If a decision cannot be made reliably by deterministic code, Ghostlight should
save it as a training example.

That includes accepted outputs, rejected outputs, repaired outputs, and weird
half-successes. Especially weird half-successes. They are where the machine is
showing the fracture line.

A useful soft artifact should preserve:

- the deterministic input slice
- the model or human output
- the reviewer decision
- accepted labels
- rejected labels, if any
- rationale grounded in state, scene, culture, memory, relationship, and lore
- failure notes
- provenance to source lore or fixture state
- whether the artifact is grounded canon, transition-grounded, procedural
  branch material, or promoted branch material

## Sandboxed Responder Rule

Responder training data must not be authored from an omniscient view.

The coordinator may know the whole scene, hidden state, future branch plan,
author constraints, and private facts. The responder model we eventually train
will not. If a responder example only works because the authoring agent quietly
used omniscient context, the artifact is poisoned. It teaches the future model to
perform an impossible trick, then looks offended when reality asks for receipts.

For gold responder or character-action data:

- the coordinator prepares the scene and selects the acting character
- the projector emits the exact character-local context
- the responder is run from only that projected packet, visible event, allowed
  action space, and any explicitly included source excerpts
- the responder does not see other agents' private state, hidden canonical
  facts, author-only plot beats, future branch plans, or coordinator rationale
- any isolated worker or sub-agent used for responder generation should be
  started without inherited conversation context when the harness supports it
- repo or lore browsing is disabled for the responder unless the projected
  packet explicitly includes the excerpts or references the future model will
  also receive
- the coordinator reviews after generation, not during it

Artifact requirements:

- `responder_visible_input`: the exact prompt or structured packet the responder
  saw
- `responder_raw_output`: the unedited output
- `reviewed_output`: the accepted or repaired turn
- `review_status`: accepted, accepted-as-draft, useful-needs-revision, rejected,
  or superseded
- `coordinator_interventions`: every edit, repair, deletion, or normalization
  made after raw generation
- `hidden_context_refs`: ids or paths for context intentionally withheld, not
  the hidden content itself
- `leakage_audit`: checks for private-state leakage, future-branch leakage,
  author-only leakage, raw-state soup, prompt-constraint parroting, and
  impossible body or object knowledge

Coordinator-written replacement text can be valuable, but it is not raw
responder output. Mark it as coordinator repair or frontier-curated final. Do
not smuggle repaired prose into the responder corpus as if the sandbox produced
it. That is how a dataset learns to cheat with a straight face.

## Training-Ready Scene Loop Bundle

A scene loop is the smallest useful sequential training unit before branch
compilation. It is not enough to have a nice transcript. The bundle needs the
machine trail that made the transcript possible.

A training-ready scene-loop bundle contains:

- scene-local digest or lore slice
- initial scene/world state
- current character state refs for every participant
- projected local context per acting turn
- responder packet per acting turn
- raw responder output capture per acting turn
- observable event record per accepted action
- participant appraisal receipt for every affected participant
- reviewed mutation receipt for character state, memory, relationship,
  perceived overlays, object state, scene/world state, and unresolved hooks
- coordinator turn receipt carrying next-beat selection, world-state reads,
  proposed deltas, glue prose, and review labels
- review summary naming which organs are training-ready, reference-only, or not
  training data

Training readiness is per organ. A bundle can be useful for responder,
appraiser, mutator, relationship/perception updater, and coordinator training
while still being useless for branch compiler, IF artifact reviewer, or visual
replay training. Count only the receipts that actually exist.

`pallas-training-loop-v0` is the first pilot of this convention. It rebuilds one
Pallas threshold beat as a separate derivative and leaves the Pallas reference
fixture immutable.

## Soft Organs

These organs should be treated as model-training targets, not permanent
hand-authored rule nests.

### Coordinator Or Story Runtime

Glues scenes together, maintains world state, chooses which machinery runs, and
emits the connective prose around character turns.

In training-shaped loops, the coordinator should be sandboxed as its own
promptable organ. A coordinator worker receives only the scene state, accepted
receipts, unresolved hooks, available machinery, branch affordances, tonal
intent, author constraints, and game-engine-shaped state surfaces it is allowed
to use. It then proposes the next beat, acting character, machinery calls, glue
prose, and candidate state changes.

Codex remains the meta-coordinator while the system is being built: it scopes
the task, prepares the coordinator-visible input, launches the sandboxed
coordinator, reviews the output, labels interventions, and wires structured
state between organs. Codex-authored coordinator artifacts are useful draft
material, but they are not raw coordinator training data unless the coordinator
input/output seam was actually sandboxed.

This is a trainable artifact too. Do not let coordinator judgment evaporate into
chat history just because a large model is currently doing the stage management
with jazz hands and plausible deniability.

Training artifacts:

- scene setup receipts
- exact coordinator-visible input packet
- raw sandboxed coordinator output
- parsed and reviewed coordinator artifact
- coordinator plans
- selected next scene or next beat
- chosen acting agent
- selected tools or soft organs invoked
- world-state refs read
- world-state changes proposed
- unresolved hooks carried forward
- branch merge or branch split decisions
- authored glue prose
- rejected glue prose
- reviewer notes about pacing, legibility, state continuity, lore grounding,
  game usefulness, and machinery misuse

The coordinator output should be shaped for a future game engine. Even while the
prototype lives in model imagination and authored prose, it should preserve
structured refs for:

- scene id
- location id
- time or sequence marker
- participating agents
- active objects
- active resources
- public world facts
- unresolved facts
- branch flags
- event log refs
- state fixture refs
- consequences awaiting review

Glue prose is part of the artifact. It is not canonical state, but it is useful
training signal for how scenes are bridged, how pressure is restated without
exposition sludge, and how a procedural branch becomes readable narrative.

The coordinator may propose state changes, scene transitions, and tool calls. It
does not bypass deterministic gates. It does not silently decide hidden facts. It
does not mutate another organ's private output after review. It conducts the
little orchestra; it does not eat the violins.

### Projector

Turns canonical state, perceived state, memory, culture, scene pressure, and
affordances into character-local operating context.

Training artifacts:

- projected local context records
- projection examples
- prompt-rendering audits
- leakage failures
- source-grounding failures

The projector does not write final prose. It prepares the local pressure surface
for an acting model.

### Character Agent Or Responder

Chooses what a character does next from local awareness.

This includes speech and non-speech action. The agent may speak, wait, leave,
hide evidence, reveal evidence, spend a resource, attack, comply, defect,
bargain, comfort, or do nothing useful because people are built badly.

Training artifacts:

- exact responder-visible input packet
- hidden context refs, without hidden content
- raw responder output
- coordinator interventions or repairs
- action-choice receipts
- candidate branch sets
- accepted and rejected character turns
- spoken text when speech occurs
- observable non-speech action
- actor intent
- constraints used
- constraints violated
- reviewer notes about voice, lore, embodiment, agency, and scene fit

This model should be fine-tuned or adapted on a corpus of strong Aetheria
responses spread across the timeline. The goal is not a bolt-on lore module.
The goal is for the responder to internalize the shape of the setting: what
people assume, what institutions feel like, which pressures are normal, which
metaphors appear, what bodies can do, and how the universe tends to hurt people
while insisting it is only doing math.

That reduces lore handholding at runtime. It does not remove character-local
context. The tuned responder still needs the specific room, speaker, listener,
state, memory, affordances, and known facts for the current turn.

### Appraiser

Reads an observable event from one participant's local perspective and proposes
how that participant interprets it. This organ is not only for fictional scene
mutation. It is also one of the core paths by which VoidBot becomes better at
reading people: noticing what a human did or said, forming a grounded opinion
without pretending certainty, and revising that opinion as evidence accumulates.

Training artifacts:

- observed event
- participant-local knowledge
- private interpretation
- misread or correct-read label
- emotional appraisal
- relationship implications
- belief implications
- memory candidate
- human-facing opinion update when applicable: trust, concern, irritation,
  respect, caution, attachment, boundary pressure, expected future behavior, and
  what evidence would change the read
- reviewer corrections

The appraiser should not know canonical private state except where that
participant knows or believes it. Fundamental attribution bias belongs here. For
VoidBot, this means the model must distinguish an observed human behavior from a
durable claim about who that human is, and should preserve uncertainty, repair
signals, and evidence needed to revise the opinion later.

### State Mutator

Turns reviewed appraisal into bounded state changes.

Training artifacts:

- prior state refs
- event refs
- appraisal refs
- proposed state deltas
- accepted deltas
- rejected deltas
- mutation authority notes
- reviewer rationale

The early system should keep this manual and audited. Later a model can learn
common deltas, but deterministic gates still own schema, bounds, and authority.

### Relationship And Social Perception Classifier

Updates perceived overlays and relationship stance from repeated evidence. This
classifier should serve both Aetheria agents and VoidBot's long-running human
relationships; the same machinery that lets Sella update her read on Maer should
also let VoidBot update its read on a specific user without collapsing into
either sycophancy or a permanent first-impression mugshot.

Training artifacts:

- prior relationship stance
- event and appraisal history
- proposed movement
- confidence movement
- stale or reinforced impressions
- prejudice, stakes, memory, and social-reinforcement drivers
- reviewer notes

This is where confidence belongs. Do not grow a second correction-resistance
organ because the first one got lonely.

### Aetheria Responder Corpus

Training artifacts:

- source-grounded scene examples
- factional speech and pressure examples
- institution decision examples
- embodiment/interface examples
- negative examples where generic sci-fi assumptions were rejected
- reviewer notes tied to AetheriaLore paths

This corpus trains the character agent/responder directly. It is how a chosen
base model learns Aetheria's native assumptions rather than waiting for every
prompt to drag the whole lore vault behind it in a little wagon.

The tuned responder should learn the smell of the setting: Dominion severity,
Aya-coded care pressure, Navigator embodiment, corporate certification culture,
insurer cruelty, upload personhood fights, factional misreads, and the dry
little horrors of systems pretending to be neutral.

It still must receive character-local context at runtime. Fine-tuning gives it
priors, not omniscience.

### Branch Compiler

Turns scene intent, reviewed branch candidates, consequence packets, fold plans,
and visual continuity requirements into playable Ink plus sidecar receipts and,
when illustrated replay is in scope, a separate visual plan artifact.

The Branch Compiler is trainable because good branching structure is not just
syntax. It needs to decide where choices belong, which consequences can fold
into compact variables, which choices deserve route splits, which callbacks must
exist later, and how visual modifiers attach to actual state. Deterministic code
can validate Ink and refs. It cannot, by itself, know whether a branch feels
meaningful or whether convergence is lying.

Training artifacts:

- fixture brief
- source-grounded lore digest refs
- input branch candidates
- reviewed consequence packets
- fold plan
- generated Ink
- generated sidecar
- generated visual plan
- compiler notes
- variable set/read map
- route split or fold rationale
- visual prompt handles
- reviewer findings and repair labels

The compiler does not own hidden character truth, participant appraisal, or
durable social mutation. It materializes structure from already reviewed or
explicitly marked draft material.

### IF Artifact Reviewer

Audits playable branch artifacts before they enter the accepted corpus.

This reviewer is separate from the branch compiler because compilers are very
good at producing valid little lies. The reviewer checks whether tracked state
actually matters, whether choices change future affordances or outcomes, whether
folds preserve callbacks, whether visual modifiers correspond to real branch
state, and whether endings read the variables the story told us were important.

Training artifacts:

- Ink fixture under review
- sidecar under review
- coordinator artifact
- compiler notes
- validation output
- accepted/rejected decision
- severity-ranked findings
- failure labels such as `potemkin_state`, `cosmetic_choice`, `fake_fold`,
  `missing_state_read`, `state_named_instead_of_checked`,
  `visual_callback_missing`, and `ending_ignores_major_state`
- required fixes
- repaired artifact refs, when available

Early branch-and-fold reviews are seed material for this organ because they
caught decorative variables before they were allowed to become training sludge.

### Institution, Faction, And Consumer Decision Models

Eventually Ghostlight should support economic behavior and major faction
decision-making inside a simulated game world.

Training artifacts should preserve why agents:

- buy
- refuse
- trade
- hoard
- comply
- defect
- conceal
- reveal
- punish
- help
- lobby
- exploit
- delay
- sacrifice

The artifact should preserve perceived cost, scarcity, status pressure,
reputation risk, expected future contact, faction norms, institutional authority,
and moral injury when present.

Do not train a dialogue-only pet if the adult machine needs to decide whether a
clinic buys counterfeit cryo valves under blockade.

## Artifact Status

Soft artifacts should use explicit review status:

- `accepted`
- `accepted_as_draft`
- `useful_needs_revision`
- `rejected`
- `superseded`

Rejected artifacts remain valuable when the failure label is clear. A rejected
Navigator action that makes a cetacean walk across a human hallway is not trash.
It is a negative example with teeth.

## Training Corpus Families

Ghostlight should expect several related corpora rather than one heroic pile of
JSONL wearing a crown.

- projection corpus
- projected-local-context corpus
- coordinator/story-runtime corpus
- character-action corpus
- dialogue/prose response corpus
- participant-appraisal corpus
- state-mutation corpus
- relationship/perception corpus
- lore-grounding digest corpus
- Aetheria responder corpus
- Ink branch annotation corpus
- Branch compiler corpus
- IF artifact review corpus
- institution/faction/consumer decision corpus
- negative-example corpus

Some examples can belong to multiple corpora through references. Do not duplicate
truth; link artifacts by ids and source refs.

## Aetheria-Tuned Agent Model

The character agent itself is a training target.

A sensible long-term stack looks like this:

```text
AetheriaLore source corpus
  -> source-grounded digests and scene fixtures
  -> coordinator scene plans and glue-prose receipts
  -> reviewed Ghostlight action/dialogue/appraisal artifacts
  -> Aetheria-tuned responder or agent model
  -> specialized coordinator/projector/appraiser/mutator students
  -> deterministic runtime gates and validators
```

The Aetheria-tuned agent model should learn setting priors and voice ecology:
what institutions feel like, how factions misread each other, what bodies can do,
what pressure sounds like when nobody in the room has enough authority to fix the
machine.

It should not replace Ghostlight state. It should not decide hidden facts. It
should not mutate world state directly. It acts inside the local context and
affordances Ghostlight provides.

## Deterministic Gates Stay Deterministic

Training models does not remove hard gates.

Keep these as code:

- schema validation
- source provenance checks
- speaker-local visibility
- prompt leakage checks
- action enum validation
- body-affordance validation when source-backed
- mutation authority
- object custody
- resource and world-state legality
- Ink compilation
- artifact review status requirements

A model can propose. The runtime decides what is legal, what is visible, what is
accepted, and what becomes state.

## Near-Term Policy

During prototype work:

1. Keep fuzzy updates manual and reviewed.
2. Emit every model call as a receipt when it teaches something.
3. Store accepted and rejected examples with failure labels.
4. Prefer narrow artifacts over one vague transcript blob.
5. When a prompt fix reveals a reusable failure mode, promote it into validator
   logic or artifact schema.
6. When a model repeatedly performs a soft judgment well, mark that organ as a
   candidate for distillation or fine-tuning.
7. When it repeatedly fails in a patterned way, save the failures. The beast is
   drawing us a map in crayon.

The point is not to automate everything today. The point is to stop losing the
manual intelligence we are already spending.
