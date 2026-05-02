# Sandboxed Responder Packets

Responder packets are the handoff surface between the omniscient coordinator and
a character-local responder model or one-turn worker.

The coordinator is allowed to know the scene, the branch, hidden state, source
material, pending hooks, and author intent. The responder is not. The responder
gets one packet, acts as one character, and returns a visible action plus any
private interpretation it can justify from that packet.

No clairvoyance. No repo spelunking. No "I accidentally remembered the whole
chat" aristocratic nonsense.

## Current Artifact

The v0 packet seam is:

- `schemas/responder-packet.schema.json`
- `examples/responder-packets/scene-02-sanctuary-intake.sella_ren.packet.v0.json`
- `tools/build_responder_packet.py`
- `tools/validate_responder_packets.py`
- `npm run responder-packets:validate`
- `schemas/responder-output.schema.json`
- `experiments/responder-packets/cold-wake-sanctuary-intake-sella-v0.capture.json`
- `tools/validate_responder_outputs.py`
- `npm run responder-outputs:validate`

The first example is accepted as draft. It is suitable for testing the sandbox
handoff shape. The first no-fork worker output from that packet is also accepted
as draft, because the output is useful and clean but the review protocol is
fresh enough to deserve some suspicion. Tiny ceremonies keep us from eating the
paint.

## Packet Contents

A packet contains:

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

## Output Captures

A responder output capture preserves:

- the packet ref
- the coordinator artifact ref
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
