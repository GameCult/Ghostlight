# Sandboxed Responder Packets

Responder packets are the handoff surface between the omniscient coordinator and
a character-local responder model or one-turn worker.

The coordinator is allowed to know the scene, the branch, hidden state, source
material, pending hooks, and author intent. The responder is not. The responder
gets one packet, acts as one character, and returns a visible action plus any
private interpretation it can justify from that packet.

No clairvoyance. No repo spelunking. No "I accidentally remembered the whole
chat" aristocratic nonsense.

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

Do not mix these silently. Retrieval-augmented output can be excellent training
material for an Aetheria-tuned responder, but it does not prove packet-only
runtime competence. Packet-only failures are also useful because they reveal
which lore facts the retriever or projector needs to feed at runtime.

## Current Artifact

The v0 packet seam is:

- `schemas/responder-packet.schema.json`
- `examples/responder-packets/scene-02-sanctuary-intake.sella_ren.packet.v0.json`
- `tools/build_responder_packet.py`
- `tools/validate_responder_packets.py`
- `npm run responder-packets:validate`
- `schemas/responder-output.schema.json`
- `experiments/responder-packets/cold-wake-sanctuary-intake-sella-v0.capture.json`
- `experiments/responder-packets/cold-wake-sanctuary-intake-sella-v0.mutation.json`
- `experiments/responder-packets/cold-wake-sanctuary-intake-sella-retrieval-v0.capture.json`
- `experiments/responder-packets/cold-wake-sanctuary-intake-sella-lane-comparison.v0.json`
- `tools/validate_responder_outputs.py`
- `tools/apply_responder_output_mutation.py`
- `npm run responder-outputs:validate`

The first example is accepted as draft. It is suitable for testing the sandbox
handoff shape. The first no-fork worker output from that packet is also accepted
as draft, because the output is useful and clean but the review protocol is
fresh enough to deserve some suspicion. Tiny ceremonies keep us from eating the
paint.

## Packet Contents

A packet contains:

- `generation_lane`: `packet_only` or `retrieval_augmented`
- `lore_access`: curated excerpts only or coordinator-scoped retrieval
- `local_context_prompt`: the projected character-local context
- `observed_event`: what the character can see or hear from the prior action
- `allowed_action_labels`: concrete action labels, including speech and non-speech moves
- `source_excerpts`: short source-backed facts the responder may use
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

Those candidates are not canonical. The coordinator, event resolver, appraiser,
state mutator, and reviewer decide what becomes truth.

## Training Use

Gold responder data requires packet-only isolation. Valid approaches include:

- a one-turn subagent started without inherited context
- an external model call that receives only the packet prompt
- a manual packet-only pass, clearly labeled as manual

Any coordinator rewrite, schema repair, lore correction, or leakage removal must
be preserved as a coordinator intervention. Repaired prose is useful training
data for the coordinator, but it is not raw responder behavior.

Retrieval-augmented responder data requires scoped lore access before the
responder acts. The coordinator or retrieval worker may search only the declared
source scope, and the output capture must list consulted refs. The responder
itself should not receive repo access unless the artifact is explicitly labeled
as autonomous-research data.

Retrieval can also make the packet worse if it floods the scene with faction or
event context and starves the character's own scars. The Sella v0 comparison is
the first warning shot: the retrieval-augmented output used heat-debt and rescue
ledger texture well, but underused Sella's packet-visible backstory about
watching a sanctuary overpromise safety until refuge became a sorting machine
with kinder lighting. Future review should score both institutional grounding
and character-pressure retention.

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

The first lane comparison found the expected tradeoff. Packet-only output gave a
clean runtime-parity conditional bay decision and stronger character-specific
flavor. Retrieval-augmented output added stronger Aetheria-native institutional
texture: heat-debt timing, rescue-ledger burden, dockfall responsibility, and
Aya sanctuary capacity politics. It also underused Sella's own packet-visible
sanctuary-collapse scar. Both lanes are useful; neither should impersonate the
other.
