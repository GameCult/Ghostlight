# Agent State Variable Glossary

This document explains the current v0 Ghostlight state variables. It is the
human-facing companion to:

- `schemas/agent-state.schema.json`
- `schemas/agent-state.required-fields.json`
- `examples/agent-state.call-of-the-void.json`

The schema is not pretending this vocabulary is final. It is a first stable
core: enough shared axes to compare characters, build prompt projection, and
avoid turning every new scene into a bespoke little fog machine.

## Variable Components

Canonical scalar-like state variables use three numeric components.

| Field | Meaning |
| --- | --- |
| `mean` | The baseline tendency or long-run level for this variable. A high `mean` means this trait, strategy, pressure, or stance is normally strong for the agent. |
| `plasticity` | How easily this variable can change. High plasticity means the value can move quickly under events, relationships, culture, or scene pressure. Low plasticity means it is entrenched. |
| `current_activation` | How active the variable is right now, in this scene or current state. This can spike above or fall below the baseline without rewriting the agent. |

Example:

```json
{
  "mean": 0.35,
  "plasticity": 0.2,
  "current_activation": 0.9
}
```

This means the agent is not generally high in the variable, the baseline is not
very easy to change, but the variable is strongly activated right now.
Translation: this is a situation, not a personality transplant.

Canonical character state does not include `certainty`. If a generated or
imported profile is provisional, track that in workflow provenance, draft
status, imported-source notes, or review status.

Perceived state uses a different shape because observers do not know canonical
baseline or plasticity. They see behavior, then often treat that behavior as
evidence of what the target is like.

```json
{
  "observed_activation": 0.9,
  "attributed_disposition": 0.7,
  "confidence": 0.4
}
```

| Field | Meaning |
| --- | --- |
| `observed_activation` | How strongly the observer thinks the target expressed the dimension in the observed event or scene. |
| `attributed_disposition` | How much the observer generalizes from the observed behavior into "this is what the target is like." |
| `confidence` | How strongly the observer trusts this read. This is where evidence strength, emotional investment, prejudice, relationship stakes, prior memory, and social reinforcement show up. |

This models fundamental attribution bias directly. A character can see someone
behave coldly during a crisis and conclude "they are cold," not "they were
activated toward distance under exceptional pressure." Correcting that
impression takes work because the observer's confidence is supported by memory,
emotion, stakes, prior assumptions, and social reinforcement.

## Label Set Policy

The labels in `agent-state.required-fields.json` are the required v0 core,
not per-character tags. They cover the axes Ghostlight currently treats as
first-class.

The validator requires each canonical agent fixture to include the full core
set so agents can be compared on the same axes. That means Cat's example does
not contain only labels that apply strongly to Cat. It contains all required
labels, with low values for weak or inactive axes and high values for strong or
activated axes.

Variable maps may still grow extra labels where a character, culture, or story
needs them. Do not promote a new label into the required core until it has
proved broadly useful across multiple characters or scenes. Otherwise the
schema becomes a junk drawer with a badge.

## Underlying Organization

| Label | Meaning |
| --- | --- |
| `self_coherence` | How integrated and stable the agent's sense of self is under pressure. |
| `contingent_worth` | How much the agent's felt worth depends on approval, utility, status, purity, or external validation. |
| `shame_sensitivity` | How painful exposure, correction, failure, or diminishment feels. |
| `reciprocity_capacity` | Ability to recognize and sustain mutual obligations rather than purely extractive or avoidant relations. |
| `mentalization_quality` | Ability to model other minds with nuance instead of flattening them into crude threat, need, or utility categories. |
| `authenticity_tolerance` | Capacity to be known without immediately retreating into performance, control, or concealment. |
| `mask_rigidity` | How necessary and inflexible the performed self is. High values mean the mask is load-bearing. |
| `external_regulation_dependence` | How much emotional stability depends on other people, institutions, substances, scripts, or external structures. |

## Stable Dispositions

| Label | Meaning |
| --- | --- |
| `novelty_seeking` | Attraction to new experience, strange opportunities, and unproven paths. |
| `conformity` | Preference for accepted norms, procedures, and group expectations. |
| `status_hunger` | Desire for rank, recognition, prestige, or visible superiority. |
| `risk_tolerance` | Willingness to accept danger, uncertainty, or loss for gain or freedom. |
| `sociability` | Baseline draw toward company, conversation, networks, and social energy. |
| `baseline_threat_sensitivity` | Default readiness to detect danger, exploitation, betrayal, or humiliation. |
| `aesthetic_appetite` | Sensitivity to beauty, style, taste, symbolic form, or expressive environment. |
| `ideological_rigidity` | Resistance to revising beliefs, myths, doctrines, or explanatory frames. |

## Behavioral Dimensions

| Label | Meaning |
| --- | --- |
| `warmth` | Tendency to express care, friendliness, affection, or social welcome. |
| `drive` | Goal pressure, persistence, ambition, and forward force. |
| `grandiosity` | Inflated self-importance, entitlement, exceptionalism, or self-mythologizing. |
| `validation_seeking` | Need for reassurance, approval, recognition, or confirmation of worth. |
| `anxiety` | Anticipatory fear, worry, nervous vigilance, or instability under uncertainty. |
| `control_pressure` | Urge to manage people, situations, information, or outcomes tightly. |
| `hostility` | Readiness toward anger, contempt, attack, punishment, or antagonism. |
| `suspicion` | Expectation that others hide motives, threats, traps, or bad faith. |
| `rigidity` | Difficulty adapting behavior, interpretation, or plans under new evidence. |
| `withdrawal` | Tendency to disengage, retreat, shut down, or reduce availability. |
| `volatility` | Speed and intensity of emotional swings or escalation. |
| `attachment_seeking` | Pull toward closeness, reassurance, fusion, privileged access, or being chosen. |
| `distance_seeking` | Push toward space, opacity, autonomy, and reduced emotional claim from others. |

## Presentation Strategy

Presentation strategy describes the self being performed. This is not identical
to voice style. It is the social face, mask, posture, or survival presentation
the agent is trying to maintain.

| Label | Meaning |
| --- | --- |
| `charm` | Performs warmth, ease, charisma, or pleasantness to influence or smooth contact. |
| `compliance` | Performs agreement, submission, helpfulness, or nonthreatening cooperation. |
| `superiority` | Performs being above others, more competent, more refined, or less vulnerable. |
| `detachment` | Performs cool distance, emotional unavailability, or not-needing. |
| `seductiveness` | Performs desirability, invitation, or intimate leverage. |
| `competence_theater` | Performs capability and control, sometimes beyond what is actually secure. |
| `moral_theater` | Performs righteousness, purity, sacrifice, or ethical authority. |
| `strategic_opacity` | Controls what others can know; hides motives, pain, dependency, or plans. |
| `cultivated_harmlessness` | Performs being safe, small, useful, cute, or beneath concern. |
| `abrasive_boundary` | Performs difficulty, bite, or unpleasantness to keep others from pressing closer. |
| `ironic_distance` | Performs irony, detachment, or joking distance to avoid naked sincerity. |

## Voice Style

Voice style describes how the performed self comes out in language. The v0
required vector covers broad communication mechanics, not only the most obvious
quirks in the current examples.

| Label | Meaning |
| --- | --- |
| `dryness` | Understated, deadpan, low-sentiment phrasing. |
| `warmth` | Inviting, affectionate, reassuring, or socially soft phrasing. |
| `formality` | Uses formal structure, titles, careful address, or institutionally correct phrasing. |
| `verbosity` | Tends toward longer turns, elaboration, qualification, or verbal sprawl. |
| `pace` | Moves quickly through turns, interruptions, replies, or topic shifts. |
| `plainspoken_directness` | Speaks plainly, concretely, or bluntly rather than ornamenting or obscuring. |
| `lexical_precision` | Chooses exact words, distinctions, definitions, or careful terms. |
| `technical_density` | Packs speech with specialist terms, systems language, procedure, or analysis. |
| `technical_compression` | Compresses technical meaning into terse expert shorthand. |
| `figurative_language` | Uses metaphor, image, analogy, poetic compression, or symbolic framing. |
| `lyricism` | Uses musical, poetic, sensuous, or rhythmically heightened language. |
| `narrative_detail` | Explains through context, sequence, anecdote, or concrete scene detail. |
| `emotional_explicitness` | Names feelings, needs, wounds, affection, fear, or attachment directly. |
| `pointedness` | Cuts directly, sharply, or with barbed precision. |
| `self_disclosure` | Volunteers personal history, inner state, motives, or private stakes. |
| `hedging` | Softens claims with uncertainty markers, caveats, deference, or exits. |
| `certainty_marking` | Signals confidence, finality, authority, or refusal to leave claims open. |
| `politeness` | Uses courtesy, mitigation, face-saving language, or social smoothing. |
| `coded_politeness` | Uses polite phrasing to imply criticism, threat, refusal, hierarchy, or hidden meaning. |
| `ritualized_address` | Uses formulaic greetings, titles, honorifics, oaths, prayers, or ceremonial phrases. |
| `register_switching` | Shifts between speech registers depending on audience, status, danger, or role. |
| `dialect_marking` | Shows regional, class, occupational, subcultural, or community-specific speech markers. |
| `theatricality` | Performs speech with heightened drama, staging, persona, flourish, or rhetorical display. |
| `humor` | Uses jokes, wit, absurdity, teasing, or comic framing as a regular speech tool. |
| `conversational_dominance` | Takes, holds, redirects, or controls conversational floor and agenda. |
| `listening_responsiveness` | Reflects, tracks, validates, or adapts to what the other person just said. |
| `question_asking` | Uses questions to probe, invite, corner, teach, test, or keep the other person talking. |
| `profanity` | Uses taboo, vulgar, sacred, or deliberately coarse language. |

## Verbal Response Strategy

Verbal response strategy describes pressure-activated speech moves. These are
not baseline voice texture; they are ways speech manages exposure, threat,
attachment pressure, shame, status, or control.

| Label | Meaning |
| --- | --- |
| `sarcastic_deflection` | Uses sarcasm to redirect, dodge, or blunt sincerity, fear, gratitude, or pain. |
| `sincerity_evasion` | Avoids emotionally plain or vulnerable language even when the feeling is present. |
| `emotional_evasion` | Avoids naming, dwelling in, or accepting emotional baggage. |
| `banter_as_boundary` | Uses joking exchange to maintain distance while staying socially engaged. |
| `verbal_aggression` | Uses language to attack, intimidate, punish, or dominate. |

## Situational State

| Label | Meaning |
| --- | --- |
| `exhaustion` | Current depletion, fatigue, overuse, or low reserve. |
| `scarcity_pressure` | Pressure from money, supplies, access, time, space, safety, or opportunity being scarce. |
| `humiliation` | Current felt diminishment, exposure, embarrassment, or status wound. |
| `panic` | Acute fear, urgency, overwhelm, or threat response. |
| `triumph` | Current victory, vindication, high confidence, or emotional lift. |
| `grief` | Active loss, mourning, sorrow, or ache. |
| `overstimulation` | Current sensory, social, cognitive, or emotional overload. |
| `grievance_activation` | Active resentment, injustice memory, or retaliatory moral charge. |
| `acute_shame` | Immediate shame flare, exposure pain, or self-disgust. |
| `perceived_status_threat` | Current sense that rank, dignity, competence, or face is under threat. |

## Relationship Stance

Relationship stance is directional. `A -> B` can differ sharply from `B -> A`.

| Label | Meaning |
| --- | --- |
| `trust` | Expectation that the target will not exploit vulnerability or act in bad faith. |
| `respect` | Recognition of competence, integrity, status, or seriousness. |
| `resentment` | Accumulated grievance, bitterness, or unpaid emotional debt. |
| `dependence` | Need for the target's resources, care, access, protection, or recognition. |
| `fear` | Expectation that the target can or will cause harm. |
| `fascination` | Captivation, curiosity, attraction, fixation, or unresolved interest. |
| `obligation` | Felt debt, duty, promise, or responsibility toward the target. |
| `envy` | Pain around what the target has, is, receives, or represents. |
| `moral_disgust` | Aversion rooted in perceived violation, corruption, cruelty, or wrongness. |
| `perceived_status_gap` | Felt difference in rank, leverage, dignity, or social position. |
| `expectation_of_care` | Belief that the target may protect, soothe, support, or choose the source. |
| `expectation_of_betrayal` | Belief that the target may abandon, expose, exploit, or turn. |

## Cat Fixture Policy

Cat's example fixture currently includes all required core labels for Cat and
Oz. It is not a sparse profile of only traits that apply to them.

That is deliberate for v0:

- complete vectors make comparison and validation easier
- low `mean` values are meaningful
- low `current_activation` values tell the renderer not to emphasize a variable

Later, Ghostlight may support sparse authoring overlays for convenience. The
canonical fixture should remain explicit until we have enough tooling to avoid
silent defaults becoming accidental canon.
