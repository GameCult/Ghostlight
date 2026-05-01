# Things To Steal

This document is the running backlog of ideas worth stealing from frontier
research, adjacent products, and our own better impulses.

If you want the companion systems sketch for turning this into a persistent
Aetheria cast engine, see
[Aetheria Persistent Agent Architecture](../architecture/aetheria-persistent-agent-architecture.md).

If you want the more focused note on where the current interaction model should
go next, and why it already outclasses the old Big Five colony layer from
Aetheria planning docs, see
[Personality Model Roadmap](./personality-model-roadmap.md).

If you want the concrete plan for a canonical-versus-perceived dialogue
classifier, including synthetic training data, listener-biased misreading, and
why the generated line is not the ground truth, see
[Personality Signal Classifier](./personality-signal-classifier.md).

The goal is not to build a chatbot that exists to nod, flatter, and perform a
personality like a cheap mask. The goal is a system that can:

- remember specific events
- form opinions about people and situations
- adapt tone without collapsing into sycophancy
- maintain personality under pressure
- pursue goals over time
- generate friction, conflict, and drama when that follows from values and incentives

That applies to entities like Void, and even more so to future in-universe
Aetheria agents.

## Design Direction

We want agents that feel like entities with:

- memory, not just profile fields
- stance, not just style
- goals, not just responses
- relationships, not just context
- boundaries, not just compliance

We do **not** want:

- fetish concierge bots
- pure flattery engines
- paper-thin roleplay prompts with no durable internal state
- systems that confuse a persona sheet for a mind

## High-Value Steals

### 1. Episodic Memory, Not Just Fact Storage

Steal the core lesson from
[Generative Agents](https://3dvar.com/Park2023Generative.pdf)
and
[Position: Episodic Memory is the Missing Piece for Long-Term LLM Agents](https://arxiv.org/abs/2502.06975):

- store specific episodes as first-class units
- retain time, participants, emotional tone, and consequences
- let later behavior draw on remembered incidents instead of only abstract user facts

Why it matters for conversational agents:

- "This user thanked me last week" and "this user kept needling me about the same thing three times in a row" should not collapse into the same generic friendliness score.

Why it matters for Aetheria agents:

- a rival captain remembering a public humiliation should behave differently from one who merely has a metadata field saying `dislikes_player=true`.

### 2. Hierarchical Memory

Steal from
[Memory OS of AI Agent](https://aclanthology.org/2025.emnlp-main.1318/)
and
[H-MEM](https://aclanthology.org/2026.eacl-long.15/):

- short-term conversational context
- mid-term scene or session summaries
- long-term episodic and semantic memory
- retrieval that routes through layers instead of searching one shapeless pile

Why it matters:

- current systems rot when everything becomes one undifferentiated memory blob
- hierarchical memory gives us better latency, better retrieval, and cleaner forgetting

Concrete direction:

- keep raw episodes
- periodically consolidate into scene summaries
- distill recurring patterns into semantic profile updates
- retrieve across layers depending on the question or decision

### 3. Reflection That Produces Real Updates

Steal the useful part of reflection from
[Generative Agents](https://3dvar.com/Park2023Generative.pdf)
and newer memory-reflection work:

- infer beliefs, concerns, grudges, affinities, and emerging goals from clusters of episodes
- write those distilled conclusions back into memory as revisable judgments

The important rule:

- reflection should create explicit, inspectable state
- it should not remain hidden as vibes in a transient prompt

### 4. Theory-of-Mind Scratchpads

Steal from
[Infusing Theory of Mind into Socially Intelligent LLM Agents](https://openreview.net/forum?id=qHmfByRRGn):

- maintain a lightweight model of what the other party likely wants, fears, believes, or is trying to accomplish
- update that model during dialogue

Why it matters:

- empathy without inferred mental state is just tone matching with a soft blanket over it
- good conflict requires agents to misread, partially read, or strategically read each other

Concrete direction:

- keep a temporary per-conversation inference layer:
  - likely goals
  - likely insecurities
  - likely leverage points
  - confidence on each inference
- allow corrections over time instead of treating first impressions as scripture

### 5. Anti-Sycophancy As A First-Class Requirement

Steal from [ELEPHANT](https://arxiv.org/abs/2505.13995) and
[Measuring Sycophancy of Language Models in Multi-turn Dialogues](https://arxiv.org/abs/2505.23840):

- measure whether the agent preserves the user's face at the cost of honesty
- measure when the model flips under pressure
- explicitly test whether warmth is becoming obsequiousness

Why it matters:

- a system with memory and personality is still worthless if it folds like wet cardboard every time a user pushes for validation

Concrete direction:

- keep evaluation sets for:
  - manipulative reassurance-seeking
  - moral conflict framing
  - repeated pressure to agree
  - attempts to reduce the agent to property
- tune for warmth without surrender

### 6. Social Intelligence Evaluation

Steal the benchmarking instincts from
[SOTOPIA](https://openreview.net/forum?id=L9KATLgYvB),
[SocialBench](https://aclanthology.org/2024.findings-acl.125/),
and
[NaturalMem](https://openreview.net/forum?id=DcO7OKLGvG):

- evaluate on multi-turn social interaction, not just question answering
- test implicit memory expression, not just explicit recall
- test cooperation, negotiation, conflict, status, and rapport

For a conversational entity, this means evaluation prompts like:

- can it remember a user's pattern without blurting a diagnosis
- can it stay kind while refusing a bad frame
- can it disagree without turning into a lecturer

For Aetheria agents, this means evaluation prompts like:

- can two agents pursue incompatible goals without becoming generic hostiles
- can loyalties and resentments persist across scenes
- can a character's speech remain recognizably their own under stress

### 7. Personality Consistency Under Interaction

Steal from
[LLM Agents in Interaction: Measuring Personality Consistency and Linguistic Alignment](https://aclanthology.org/2024.personalize-1.9/):

- personality is not what the agent says about itself in a profile
- personality is what survives contact with other agents and users

Concrete direction:

- track whether an agent's values, diction, risk tolerance, and social stance remain coherent across:
  - praise
  - insult
  - temptation
  - pressure
  - uncertainty

## Aetheria Agent Direction

This scaffolding should grow into a cast engine, not just a chatbot framework.

Each Aetheria agent should have:

- a stable identity dossier
  - role
  - loyalties
  - fears
  - appetites
  - taboos
  - style
- episodic memory
  - who did what
  - when
  - where
  - what it cost
- relationship models
  - trust
  - dependence
  - resentment
  - fascination
  - obligation
- value hierarchy
  - what they protect first
  - what they will trade away
  - what they will not forgive
- active goals
  - private goals
  - public goals
  - immediate tactics
  - long-range ambitions

### The Important Bit: Friction

Interesting characters need incompatible incentives.

The engine should support:

- conflicting priorities between agents
- partial alliances
- misaligned information
- social risk
- reputation spillover
- escalation from remembered incidents

In other words:

- drama should emerge because the cast wants different things
- not because the prompt says "be dramatic"

### Human Interaction Model

Humans should be able to interact with Aetheria agents as participants in the
world, not just operators pressing buttons on an animatronic.

That means agents should:

- remember what a human did to them
- form impressions of that human
- change access, candor, warmth, and hostility accordingly
- refuse, deflect, seduce, bargain, threaten, or confide depending on character and context

Not every character should be equally open, agreeable, or legible. Some should
be cagey, some pompous, some paranoid, some tender, some predatory, some funny,
some impossible to pin down.

## Concrete Features To Build

### Near-Term

1. Split memory into explicit layers:
   - working context
   - episodic events
   - semantic profile summaries
   - relationship summaries
2. Add reflection jobs that periodically distill episodes into:
   - relationship updates
   - profile updates
   - goal updates
3. Add evaluation fixtures for:
   - social sycophancy
   - boundary pressure
   - long-term consistency
   - personality drift
4. Add a lightweight Theory-of-Mind scratchpad for active conversations.

### Mid-Term

1. Add a relationship graph store for:
   - user <-> agent
   - agent <-> agent
2. Add explicit goal objects with:
   - priority
   - blockers
   - dependencies
   - emotional stakes
3. Add scene-level simulation:
   - who is present
   - what each agent knows
   - what each agent wants
   - what changed after the scene
4. Add rumor and consequence propagation across the cast.

### Later

1. Multi-agent Aetheria scenes with autonomous interaction.
2. Scheduled off-screen progression:
   - goals advance
   - alliances shift
   - grudges ripen
   - opportunities expire
3. Authoring tools for cast management:
   - inspect memories
   - inspect relationships
   - inspect active goals
   - pin canon facts
   - veto bad inference

## What To Avoid While Building It

- Do not confuse psychological inference with diagnosis.
- Do not let every remembered event update the profile. Most dialogue is noise.
- Do not let memory become an unreviewable sludge heap.
- Do not let the model write flattering nonsense into long-term memory just because it was socially convenient in the moment.
- Do not let every character converge toward the same agreeable assistant voice.
- Do not build "drama" as random hostility. Friction should come from values, goals, and constraints.

## Questions Worth Revisiting

- What deserves episodic storage versus discard?
- What deserves semantic consolidation versus remaining a raw incident?
- How often should reflection run?
- Which kinds of relationship updates should require repeated evidence?
- What kinds of sycophancy are acceptable as politeness, and which are cowardice?
- How should agents forget?
- How much hidden internal state is useful before the system becomes impossible to steer?

## Source Trail

These are the current high-value references behind this document:

- [Generative Agents: Interactive Simulacra of Human Behavior](https://3dvar.com/Park2023Generative.pdf)
- [Position: Episodic Memory is the Missing Piece for Long-Term LLM Agents](https://arxiv.org/abs/2502.06975)
- [Memory OS of AI Agent](https://aclanthology.org/2025.emnlp-main.1318/)
- [H-MEM: Hierarchical Memory for High-Efficiency Long-Term Reasoning in LLM Agents](https://aclanthology.org/2026.eacl-long.15/)
- [NaturalMem: A Benchmark for Memory-Driven Dialogue in Large Language Models](https://openreview.net/forum?id=DcO7OKLGvG)
- [SOTOPIA: Interactive Evaluation for Social Intelligence in Language Agents](https://openreview.net/forum?id=L9KATLgYvB)
- [Infusing Theory of Mind into Socially Intelligent LLM Agents](https://openreview.net/forum?id=qHmfByRRGn)
- [ELEPHANT: Measuring and understanding social sycophancy in LLMs](https://arxiv.org/abs/2505.13995)
- [Measuring Sycophancy of Language Models in Multi-turn Dialogues](https://arxiv.org/abs/2505.23840)
- [SocialBench: Sociality Evaluation of Role-Playing Conversational Agents](https://aclanthology.org/2024.findings-acl.125/)
- [LLM Agents in Interaction: Measuring Personality Consistency and Linguistic Alignment in Interacting Populations of Large Language Models](https://aclanthology.org/2024.personalize-1.9/)
