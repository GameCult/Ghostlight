# Soft Model Training Artifacts

Ghostlight has two kinds of machinery.

Some parts should be boring code:

- schema validation
- visibility filtering
- action legality
- object custody
- resource accounting
- Ink materialization
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

## Soft Organs

These organs should be treated as model-training targets, not permanent
hand-authored rule nests.

### Coordinator Or Story Runtime

Glues scenes together, maintains world state, chooses which machinery runs, and
emits the connective prose around character turns.

For now, the coordinator is the authoring agent in the loop. In practice, that
means Codex or another frontier model reads the current fixture, source
grounding, prior receipts, and author constraints, then decides what scene beat
comes next, which character acts, which deterministic tools or soft organs are
needed, and what prose connects the generated turns.

This is a trainable artifact too. Do not let coordinator judgment evaporate into
chat history just because a large model is currently doing the stage management
with jazz hands and plausible deniability.

Training artifacts:

- scene setup receipts
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
how that participant interprets it.

Training artifacts:

- observed event
- participant-local knowledge
- private interpretation
- misread or correct-read label
- emotional appraisal
- relationship implications
- belief implications
- memory candidate
- reviewer corrections

The appraiser should not know canonical private state except where that
participant knows or believes it. Fundamental attribution bias belongs here.

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

Updates perceived overlays and relationship stance from repeated evidence.

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

This corpus trains the character agent/responder directly. It is how a
Qwen-class base or another local model learns Aetheria's native assumptions
rather than waiting for every prompt to drag the whole lore vault behind it in a
little wagon.

The tuned responder should learn the smell of the setting: Dominion severity,
Aya-coded care pressure, Navigator embodiment, corporate certification culture,
insurer cruelty, upload personhood fights, factional misreads, and the dry
little horrors of systems pretending to be neutral.

It still must receive character-local context at runtime. Fine-tuning gives it
priors, not omniscience.

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
