---
title: Agents and model stages
description: A catalog of Ghostlight's production model calls, their roles, outputs, and deterministic gates.
---

Ghostlight uses three logical model tiers. **Fast** handles bounded projection,
classification, and selection; **Balanced** handles reconciliation, causal
reasoning, and most agentic workbenches; **Capable** handles the hardest world
compilation and numinous elaboration. Deployments may map these aliases to
different physical models without changing stage contracts.

“Agentic” means the stage can inspect a bounded workbench, take several typed
actions, receive deterministic feedback, and revise on the same snapshot. A
one-shot model stage still produces only a proposal.

## Session Zero

| Stage | Tier | Purpose | Output and downstream gate |
|---|---|---|---|
| `session_zero_projector` | Fast/configured | Lowers permitted negotiation state into the DM's private lived context | Projected stream; schema validation only, no state write |
| `session_zero_dm_persona` | Balanced/configured | Embodied persistent DM who discusses premise, boundaries, roles, and bargains | Natural speech; owns no accepted changes |
| `session_zero_interpreter` | Balanced/configured | Extracts only new typed proposals or one focused counterproposal from DM/player speech | Contract, character, or permission proposal; exact owner acceptance and Session Zero kernel gate |

## Retrieval and world compilation

| Stage | Tier | Purpose | Output and downstream gate |
|---|---|---|---|
| `opening_retrieval_plan` | Fast | Plans source queries for possible openings | Queries; Vault boundary executes and receipts results |
| `world_openings` | Balanced | Suggests grounded starting situations | Suggestions with evidence IDs; deterministic evidence validation |
| `role_retrieval_plan` | Fast | Plans source queries for playable roles | Queries; Vault boundary |
| `world_roles` | Balanced | Suggests roles, capabilities, and obligations | Suggestions; evidence and duplication checks |
| `custom_retrieval_plan` | Fast | Plans queries for a custom campaign brief | Queries only |
| `evidence_relevance` | Fast | Separates direct seed evidence from background and excluded material | Classification; deterministic source-lane admission |
| `world_compile` | Capable | Compiles the bounded playable seed: topology, cast, populations, institutions, facts, clocks, and civic state | Candidate seed; full structural and semantic validation before registry publication |
| `private_relationship_actor_compile` | Balanced | Compiles private player-character state and approved relationship anchors | Private actor proposal; identity, account, and relationship binding checks |
| `agency_compile` | Balanced | Profiles local actors, institutions, and populations across the six agency axes | Agency profiles and relations; exact subject coverage validation |
| `global_agency_compile` | Fast/configured | Builds the coarse offscreen strategic skeleton | Major powers, regions, channels, and pressures; schema and evidence checks |
| `global_agency_doctrine_synthesis` | Balanced | Proposes strategic doctrines from exact global agency anchors | Doctrine candidates; cannot rewrite source anchors |
| `global_agency_doctrine_verification` | Balanced | Judges whether synthesized doctrines preserve their anchors | Verdict; deterministic compiler decides admission |

## Destination, civic, and clock workbenches

| Stage | Tier | Purpose | Output and downstream gate |
|---|---|---|---|
| `destination_identity_resolution` | Fast | Decides whether a requested destination is an existing reachable place or a genuinely new region | Identity proposal; deterministic topology check chooses elaboration versus expansion |
| `destination_retrieval_plan` | Fast | Plans exact source queries for a destination | Queries only |
| `destination_compile` | Capable | Proposes geography, routes, residents, institutions, political relations, and civic apparatus | Expansion/elaboration candidate; topology, evidence, knowledge, and civic closure checks |
| `destination_reconciliation_agent_action` | Balanced, agentic | Repairs an exact rejected destination finding through small transactional edits | Revised private candidate; reruns the existing validators, never writes world state |
| `destination_civic_verification` | Balanced | Independently judges whether authority, succession, resources, redress, and resident knowledge are legible | Verdict receipt; kernel rebinds and checks it at admission |
| `gestalt_fission_retrieval_plan` | Fast | Plans evidence retrieval for a requested population subdivision | Queries; fission compiler and approval gate |
| `clock_consequence_binding_agent_action` | Balanced, agentic | Binds legacy clock consequence text to exact observable subjects and public channels | Binding proposal; deterministic threshold/event validation and kernel commit |

## Titled world elaborators

All titled workers use `world_elaboration_<title>`, the common
agent harness, one exact frozen assignment, and at most one additive operation.
The weighted scheduler controls invocation frequency in proportion to the
user's slider shares.

| Agent | Tier | Character of contribution |
|---|---|---|
| **Patina** | Fast | Durable low-stakes texture: objects, nicknames, customs, jokes, and reusable places |
| **Charter** | Balanced | Government, offices, law, succession, selection, procedure, and redress |
| **Ledger** | Fast | Labor, resources, infrastructure, exchange, scarcity, and class pressure |
| **Hearth** | Fast | Kinship, care, neighborhood, obligation, belonging, and private stakes |
| **Tangle** | Balanced | Factions, alliances, rivalries, constituencies, leverage, and plots |
| **Veil** | Fast | Secrets, rumors, misinformation, taboo, mystery, and disclosure paths |
| **Ember** | Fast | Active disputes, hazards, instability, escalation, and urgent pressure |
| **Numen** | Capable | Religion, ritual, magic, cosmology, awe, and bounded strangeness |

The deterministic elaboration tool checks the operation against the title's
exact assignment and frozen namespace. Wave admission resolves conflicts; an
independent semantic verifier reviews the inhabited candidate; `WorldKernel`
alone commits it.

At latent-world scale, the same titled sessions also receive
`world-complexity-<title>-fission` or
`world-complexity-<title>-individuate` assignments. Each call may subdivide one
active Gestalt or promote one grounded consequential person. A fission worker
returns only the partition delta—child identities and partition values plus
exact member/resource assignments. The deterministic tool attaches the frozen
world binding and inherited population state, avoiding repeated constitutions
in model output. An individuation worker similarly supplies the member delta
while the tool owns parent version and location. Deterministic validation and
the existing kernel commands remain the only admission owners. After every productive round,
`world-elaborator-<title>-session-compaction` distills that title's frontier in
the assigned realm jurisdiction and its unresolved leads while the harness
reattaches exact commit ancestry. The title-by-realm key prevents unrelated
political frontiers from becoming one global transcript. Compacted memory can
guide later proposals but cannot create subjects or facts.

## Foreground perception, response, and action

| Stage | Tier | Purpose | Output and downstream gate |
|---|---|---|---|
| `speech_address_resolver` | Fast | Selects exact co-present direct addressees for player speech | Addressee IDs; presence and response-duty validation |
| `projector` | Fast/configured | Projects one actor's permitted typed state and visible event into lived narrative | Private context; never canonical truth |
| `persona` | Balanced/configured | Appraises the event and produces natural response from one actor's perspective | Speech, deliberate silence, private appraisal, or action intent |
| `interpreter` | Capable/configured, agentic | Negotiates natural intent into the closed action algebra | Typed action proposal; authority and semantic validation |
| `assessment_mutation_scope` | Fast/Balanced | Selects the smallest mutation lanes that could realize an attempted effect | Upper-bound scope or precise denial; deterministic lane checks |
| `action_assessment` | Balanced | Sets possibility, DC, stakes, outcome bands, and effect ceiling | Assessment; canonical DC/reference validation and player risk acceptance |
| `assessment_effect_verifier` | Balanced | Checks that a proposed outcome mutation is caused by the exact attempted means | Verdict; invalid effects cannot reach the kernel |
| `gestalt_presence_planner` | Fast/configured | Promotes an earned existing member, admits one speech-authorized first identity, or folds an irrelevant member | Presence plan; exact lineage, location, version, and identity checks |

## Strategic simulation

| Stage | Tier | Purpose | Output and downstream gate |
|---|---|---|---|
| `resolution_demand` | Fast | Weights current causal relevance across geography, ideology, authority, economy, species/body, and information | Demand weights and focal IDs; deterministic connected partitioner owns the cover |
| `nemesis_attention_agent_action` | Balanced, agentic | Searches committed causal anchors and decides which autonomous subjects genuinely need a response window now | Anchor/responder assignments or none; exact subject/anchor validation, no action authority |
| `cell_projector` | Fast/configured | Projects a cell's exact constituents and required perspective owners without merging private state | Cell lived context |
| `cell_persona` | Balanced/configured | Lets each required actor, institution, population, or named member appraise and choose independently | Attributed natural decisions; arenas never speak as synthetic actors |
| `cell_interpreter` | Capable/configured, agentic | Translates each selected strategic decision into one or more typed attempts | Attributed action proposals; graph, target, location, and effect validation |
| `cell_effect_verifier` | Balanced/configured | Checks that each strategic attempt uses only its own constituent's authority and knowledge | Ordered verdicts; correction may reuse the same bounded workbench |
| `strategic_outcome_resolver` | Balanced | Resolves opposition and proposes one durable result per admitted action | Outcome bundle; exact handles and action digests must match |
| `strategic_outcome_verifier` | Fast/Balanced | Independently checks high-risk custody, relation, knowledge, and member-specific consequences | Selective semantic verdict; ordinary structural effects remain deterministic |
| `strategic_individuation_selector` | Fast | Names at most one person when selected Gestalt action creates political work that cannot remain anonymous | Member proposal or none; shared individuation validator and kernel admission |

## Newspaper

| Stage | Tier | Purpose | Output and downstream gate |
|---|---|---|---|
| `newspaper_narrative_selection_agent_action` | Balanced, agentic | Assignment editor queries the frozen ledger, chooses issue shape and throughline, assigns recurring reporters, and binds evidence | Immutable agenda checkpoint; exact record, reporter, lead, and conflict-axis validation |
| `newspaper_journalist_agent_action` | Balanced, agentic | One embodied journalist writes one assigned story through their recurring beat, voice, biases, and source habits | Article copy plus citations; no factual admission authority |
| `newspaper_copy_desk` | Balanced | Performs the single hard factual review over the assembled page | Complete finding checklist; cannot rewrite or publish |
| `newspaper_night_editor_close_agent_action` | Balanced, agentic | Repairs the copy desk checklist once while preserving narrative and closes at deadline | Final page; deterministic press witness checks structure and lineage |

Post-press grounding review is audit, not another same-edition rewrite loop.
Supported corrections become later-edition memory.

## Deterministic stages that deliberately use no model

The `WorldKernel`, mutation reducer, resolution partitioner, dice roller,
identity/version checks, topology and custody validation, information-scope
checks, checkpoint recovery, provenance hashing, archive verification, and Eve
projection lowering are deterministic. Keeping these outside model prompts is
what lets agents negotiate with the engine instead of impersonating it.
