# Sandboxed Responder Packets

Responder packets are the handoff surface between the omniscient coordinator and
a character-local responder model or one-turn worker.

The coordinator is allowed to know the scene, the branch, hidden state, source
material, pending hooks, and author intent. The responder is not. The responder
gets one packet, acts as one character, and returns a visible action plus any
private interpretation it can justify from that packet.

No clairvoyance. No undeclared repo spelunking. No "I accidentally remembered
the whole chat" aristocratic nonsense.

## Generation Lanes

Responder examples have two valid lanes.

`packet_only` is the runtime-parity lane. The responder sees the packet,
including any curated source excerpts the coordinator or lore retriever placed
there. It does not search the lore archive. Use this lane to test whether a
future game-runtime responder can act from bounded scene context.

`retrieval_augmented` is the lore-absorption lane. The coordinator or a
dedicated retrieval worker searches a declared AetheriaLore scope, then builds a
visible packet from the selected refs. The responder still answers only from the
packet it receives. Use this lane to generate stronger Aetheria-native final
responses and to bake setting assumptions, faction pressures, species
affordances, and tone into later fine-tuning data without pretending the
character model did autonomous scholarship in the walls.

There are two retrieval-augmented modes:

- `coordinator_scoped_retrieval`: the coordinator or retriever consults lore,
  then hands the responder a bounded packet. The responder has no repo access.
- `responder_scoped_repository_search`: the responder itself is explicitly
  allowed to inspect the declared lore scope before answering. This is the
  research-enabled lane. The packet must include a `Required Lore Research`
  section, `allowed_scope`, and `research_instructions`.

In `responder_scoped_repository_search`, the responder must consult the seed
docs before answering and may follow relevant links inside the traversal policy.
It should use retrieved lore to constrain behavior, affordances, institutions,
vocabulary, material stakes, territory, social movements, and local assumptions.
It must not use research access to infer hidden state, author-only answers,
future branches, or private motives outside the packet. The output capture must
preserve consulted seed refs, followed refs, a research trace, and a brief
research summary. No consulted refs means the research-enabled response failed
at the first hurdle, tragic but at least easy to grade.

`research_trace` is the receipt surface. Each entry records the query or lookup,
source ref, source path, chunk index, line range, extracted constraint, and how
that constraint shaped the response. Trace status matters:

- `runner_captured`: the runner or retrieval worker captured the lookup as it happened. This is required for accepted gold research-enabled captures.
- `coordinator_reconstructed`: the coordinator rebuilt the trace after the fact from source search and review. Useful draft data, not proof of the responder's actual research path.
- `responder_reported`: the responder claimed the trace directly. This is texture, not audit.

Training-time research scope should be materially wider than runtime context.
The point is not to feed every responder a lore pantry forever. The point is to
let high-quality training samples absorb the world: PSC risk language, Jovian
territorial texture, resident factions, linked social movements, technology
constraints, and the odd little institutional grudges that make the universe
smell lived in. Runtime can later feed compact retrieval. Fine-tuning should
have already eaten its vegetables.

Do not mix these silently. Retrieval-augmented output can be excellent training
material for an Aetheria-tuned responder, but it does not prove packet-only
runtime competence. Packet-only failures are also useful because they reveal
which lore facts the retriever or projector needs to feed at runtime.

## Current Artifact

The packet seam is:

- `schemas/responder-packet.schema.json`
- `tools/validate_responder_packets.py`
- `npm run responder-packets:validate`
- `schemas/responder-output.schema.json`
- `tools/validate_responder_outputs.py`
- `npm run responder-outputs:validate`

Current live fixtures may not include responder packet examples at all times.
That is acceptable. The contract remains live; obsolete prototype captures do
not need to stay in the working tree once their lessons are distilled.

## Packet Contents

A packet contains:

- `generation_lane`: `packet_only` or `retrieval_augmented`
- `lore_access`: curated excerpts, coordinator-scoped retrieval, or
  responder-scoped repository search
- `traversal_policy`: for responder-scoped research, the permitted link depth,
  allowed repo prefixes, required seed refs, link classes to follow, and stop
  conditions
- `local_context_prompt`: the projected character-local context
- `observed_event`: what the character can see or hear from the prior action
- `allowed_action_labels`: concrete action labels, including speech and non-speech moves
- `source_excerpts`: short source-backed facts the responder may use, copied
  from active runtime retrieval requirements plus local packet invariants
- `output_contract`: the required JSON response shape and rules
- `hidden_context_audit`: refs and inference classes deliberately omitted
- `isolation_requirements`: how the worker/model must be run
- `packet_prompt_text`: the exact prompt text sent to the responder

The packet prompt must not expose raw state internals such as
`current_activation`, `plasticity`, raw dimension scores, coordinator rationale,
author-only answers, or future branch plans.

## Response Boundary

The responder may output:

- a visible action
- optional spoken text
- private interpretation explicitly framed as interpretation
- candidate state updates
- candidate relationship updates
- unresolved hooks

`spoken_text` is not the place to dump the packet contract. Characters should
not recite every legal, procedural, or safety constraint simply because the
packet made those constraints visible. Keep speech to what the character would
actually say under pressure. Put the full checklist in `visible_action`,
`state_update_candidates`, `relationship_update_candidates`, and the mutation
receipt.

Good future-institutional speech is compressed and situated:

```text
I hold the line shallow. Your bay, your mark; my ledger stays awake for every
cost the tide takes.
```

Bad future-institutional speech is the output contract wearing a mask:

```text
Bounded diagnostic contact only. No full intake, no custody, no personhood
finding, no cargo designation...
```

That second kind of line may be useful as a failure sample. It is not gold
responder behavior unless the scene specifically calls for formal oath,
courtroom language, ritual compliance, or someone deliberately reading terms
aloud.

Those candidates are not canonical. The coordinator, event resolver, appraiser,
state mutator, and reviewer decide what becomes truth.

## Training Use

Gold responder data requires declared isolation. Valid approaches include:

- a one-turn subagent started without inherited context
- an external model call that receives only the packet prompt
- a manual packet-only pass, clearly labeled as manual

Any coordinator rewrite, schema repair, lore correction, or leakage removal must
be preserved as a coordinator intervention. Repaired prose is useful training
data for the coordinator, but it is not raw responder behavior.

Retrieval-augmented responder data requires scoped lore access before the
responder acts. The coordinator, retrieval worker, or explicitly research-enabled
responder may search only the declared source scope, and the output capture must
list consulted refs and trace entries. The responder itself should not receive repo access unless
the artifact uses `responder_scoped_repository_search`, sets `no_repo_access` to
false, and includes visible research instructions in the prompt.

Retrieval can also make the packet worse if it floods the scene with faction or
event context and starves the character's own relevant pressures. Review must
verify that packet-visible history, role strain, and local relationships remain
available as behavioral pressure. The point is not to make a character
trauma-dump during a mission-critical exchange. The point is that the responder
cannot be shaped by a pressure it cannot see.

## Output Captures

A responder output capture preserves:

- the packet ref
- the coordinator artifact ref
- generation lane
- lore access mode and consulted refs
- the isolation method
- the exact visible input ref
- hidden context refs that were withheld
- raw responder output
- parsed responder output
- leakage audit
- coordinator review and intervention labels

The raw output must parse to the reviewed `parsed_output` exactly. If the
coordinator edits prose, repairs schema, removes leakage, or corrects lore, the
capture must say so directly.

For `responder_scoped_repository_search`, the raw responder object may also
include `consulted_refs`, `followed_refs`, `research_trace`, and
`research_summary`. Those fields should be preserved inside `parsed_output`
when the responder itself produced them; consulted refs, followed refs, summary,
trace status, and trace entries should also be mirrored into top-level
`lore_access` so the capture can be indexed without stripping the original
response. If the runner cannot expose a machine-verifiable tool-call or
retrieval transcript, label the sample as draft and say so in review notes.
Trust, but make the trust write receipts like everyone else.

The current packet builder consumes `runtime_retrieval_requirements` from the
projected local context. Packet-only source excerpts should come from the same
retrieval/projector seam that would feed runtime context. Static lore sandwiches
are useful only as rejected-path examples and emergency snacks.
