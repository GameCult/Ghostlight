# Weaver-Visible Packet: lucent-hostage-feed.negotiator-branch.v1

## Task

Generate a complete playable Ink artifact and training sidecar for `lucent-hostage-feed.negotiator-branch.v1`.

Return valid JSON only using the `Interactive Fiction Weaver` output contract:

- `weaver_id`
- `artifact_id`
- `ink_text`
- `training_sidecar`
- `review_notes`
- `open_questions`
- `failure_labels`

The artifact must be from Mara Quell's playable negotiator perspective. It should be a fresh branch-and-fold IF draft using the coordinator branch plan below, not a copy of prior hand-authored Ink.

## Output Path Placeholders

Use these refs in the sidecar:

- `ink_ref`: `examples/ink/lucent-hostage-feed.negotiator-branch.v1.ink`
- `source_fixture_ref`: `examples/training-loops/lucent-hostage-feed-v0/lucent-hostage-feed-v0.initial-state.json`
- `scene_id`: `lucent-hostage-feed-negotiator-branch-v1`
- `protagonist_agent_id`: `mara_quell`
- `npc_agent_ids`: `sol_vant`, `juno_ark`, `feed_ops_chorus`, `pippa_lux`, `brant_vee`

## Required Local Awareness Summary

Include at least these facts in sidecar form:

- Mara can see and act through official crisis feed controls, but she is not omniscient.
- Sol holds Juno in the glass transfer lounge near the elevator gate.
- Lucent feed systems, influencers, audience heat, and security pressure are active hazards.
- The city bubble and media-eye station geometry matters as reader orientation and pressure, but this pass does not include visual plan artifacts.

## Required Projection Controls

Use the following categories in the sidecar: `frame_controls`, `authority_boundaries`, `object_custody`, `required_semantics`, `forbidden_resolutions`.

Make each entry an object with `text` and `source_refs`.

## Source / Lore Digest Excerpts

Lucent hostage-feed scene seed:

A hostage negotiation unfolds live inside a Lucent tether station. The station is a spinning tethered counterweight design: a dense media-headquarters counterweight shaped like a giant eye stares down the tether at a bustling metropolis dome sealed in a transparent pressure bubble. A pressurized elevator spine links them like an optic nerve. The incident takes place in a glass-walled transfer lounge inside the media-eye end near the elevator gate.

Tone target: dark comedy with real hostage stakes. Lucent turns danger into format; comedy comes from absurd professionalism, brand panic, clout opportunism, and feed etiquette.

Mara Quell: Lucent crisis negotiator and former intimacy-format host. Warm with scalpel edges; precise showbiz empathy; wry when it lowers temperature; resists letting brand language own the room. Goals: keep Juno alive, give Sol a face-saving exit that does not reward hostage-taking too much, prevent influencer side feeds from becoming the actual negotiators. She understands feeds but should not be omniscient about private intent.

Sol Vant: failed contract creator made into content by debt, demonetization, and arbitration. Dangerous but not predetermined to kill. Wants recognition, debt review, and proof he still exists more than murder. Funny in brittle flashes; aggrieved; overexplains when scared; performs intelligence for hostile viewers. Humiliation destabilizes him.

Juno Ark: junior Reality Architect and hostage. Knows feed control well enough to be dangerous but is physically vulnerable. Controlled panic; technical compression under fear; dark little jokes when cornered. She can signal or exploit overlays if given a gap, but doing so may escalate Sol.

Pippa Lux: empathy influencer. Soft-focus urgency, intimate second-person address, therapeutic theft, brand-safe tears. Can accidentally humanize Sol or catastrophically center herself.

Brant Vee: outrage streamer. Loud clarity, weaponized incredulity, mocking slogans, jokes that sharpen the knife. Can attack the correct target while making violence more likely.

Feed Ops Chorus: Lucent newsroom, moderators, Reality Architects, sponsor-risk engine. Not a person so much as a pressure field: edits visibility, throttles feeds, feeds prompts, reprices everyone by the second.

Hard constraints:

- Sol can harm Juno faster than security can cross the space.
- Every participant knows they are visible and may perform for survival.
- Moderation delay can distort timing and perceived intent.
- Lucent does not want a dead hostage, but it does want an owned narrative.
- Mara's choices should include speech and non-speech actions.
- Branches should fold through state variables, not explode into an unmanageable tree.
- Choices must not be cosmetic. Variables must affect later text/options/outcomes.

## Sandboxed Coordinator Output

Use this as the branch plan and cognition guardrail:

```json
{
  "coordinator_worker_id": "stage_manager",
  "decision_id": "lucent-hostage-feed.negotiator-branch.v1.turn-001.coordinator",
  "selected_next_beat": "Hand the fixture to the branch compiler / Interactive Fiction Weaver to build a branch-and-fold negotiator-perspective Ink draft centered on Mara's concrete feed, evidence, influencer, movement, and breach-pressure levers.",
  "reader_translation": "The playable pressure is not whether Mara says the perfect magic sentence. It is whether she can keep Sol from converting humiliation into violence while Lucent, influencers, security, and the audience all try to make the hostage scene legible, profitable, or punishable. Branches should test feed control, evidence custody, public face-saving, and physical movement, then fold those choices into later options and endings.",
  "branching": {"branch_mode":"branch_and_fold","reason":"Mara's choices should alter shared pressure variables, unlock or close later options, and change ending gates, while the main negotiation sequence remains readable and playable."},
  "character_cognition_checks": [
    {"agent_id":"mara_quell","constraint":"Let Mara infer from visible behavior, public state, feed artifacts, and negotiation training. Do not let her perfectly diagnose every motive or predict every engine move."},
    {"agent_id":"sol_vant","constraint":"Do not make Sol read the whole system like a philosopher-king. His sharpness should come in aggrieved flashes, especially around humiliation, debt, recognition, and being edited."},
    {"agent_id":"juno_ark","constraint":"Do not let Juno calmly run a full counter-operation while held hostage. Any technical action needs a timing gap and should risk Sol noticing."},
    {"agent_id":"pippa_lux","constraint":"She may accidentally humanize Sol or steal focus. Do not make her reliably therapeutic in a clinical or tactical sense."},
    {"agent_id":"brant_vee","constraint":"He can name a real injustice while increasing violence risk. Do not let his clarity become safe."},
    {"agent_id":"feed_ops_chorus","constraint":"Represent it as prompts, cuts, delays, overlays, and permission changes rather than a single scheming villain with clean intent."}
  ]
}
```

## Branch Plan To Materialize

Use these choice layers and fold targets. You may rename knots for Ink clarity, but preserve branch ids in `// ghostlight.branch` comments and sidecar.

1. `first_contact`: Mara chooses opening lane.
   - `branch-01-open-as-witness`: speak to Sol as witness. Increase `mara_trust`; may raise `audience_heat`.
   - `branch-01-custody-demand`: assert evidence custody. Raise `evidence_integrity`; risk lowering `sol_stability`.
   - `branch-01-route-side-panels`: route/mute side panels before addressing Sol. Raise `feed_control`, lower `influencer_noise`; risk Sol mistrusting optics management.

2. `proof_or_face`: Mara handles collapsed arbitration proof.
   - `branch-02-freeze-edit-trail`: freeze the edit trail through official custody. Raise `evidence_integrity`; may raise `security_pressure`.
   - `branch-02-create-gap-for-juno`: create a small gap for Juno to signal or touch overlay controls. Can raise `evidence_integrity` or `feed_control`; risks `hostage_safety` / `sol_stability` if Sol notices.
   - `branch-02-debt-review-for-movement`: offer public debt review in exchange for moving Juno away from immediate harm. Raise `hostage_safety` and `mara_trust`; may lower `evidence_integrity` if Lucent reframes it.

3. `influencer_pressure`: Mara handles Pippa and Brant.
   - `branch-03-pippa-tight-routing`: let Pippa speak under tight routing. May soften `audience_heat`, help Sol feel seen, but raises `influencer_noise` if Pippa centers herself.
   - `branch-03-brant-targets-lucent`: let Brant attack Lucent, not Sol. May redirect heat toward Lucent and improve `mara_trust`; risks `sol_stability`.
   - `branch-03-hard-mute-panels`: hard-mute both panels. Improve `feed_control`, reduce `influencer_noise`; may raise `audience_heat` because silence looks like suppression.

4. `movement_gate`: Mara moves bodies, not just feelings.
   - `branch-04-safe-line-step`: ask for visible safe-line step. Raise `hostage_safety`; reduce breach permission if pressure is moderate.
   - `branch-04-face-saving-handoff`: offer face-saving handoff ritual. Raise `mara_trust` and `sol_stability` if prior recognition exists; can become humiliating if heat/noise high.
   - `branch-04-security-recedes`: make security visibly recede. Raise `sol_stability` and `mara_trust`; risk Feed Ops framing Mara as losing control.

5. `contained_resolution`: endings depend on variables.
   - Juno released safely with evidence preserved.
   - Juno released but evidence compromised.
   - Security breach.
   - Sol contained but narratively devoured.
   - Ambiguous live-feed truce.

## Ink Requirements

- Start with `VAR` declarations.
- Use valid Ink syntax compatible with `inkjs-compiler`.
- Reach initial choices after introductory prose.
- Use `// ghostlight.branch: branch-id; action: action_type; intent: ...` comments.
- Include at least one non-speech branch.
- Branch and fold through shared knots.
- Use conditionals so variables affect later text, options, and endings.
- Keep text player-facing; do not expose schema prose in the preview.
- Do not include visual plan data.

## Training Sidecar Requirements

Return a JSON object under `training_sidecar` matching the required annotation shape:

- `schema_version`: `ghostlight.ink_training_annotation.v0`
- `annotation_id`: `lucent-hostage-feed-negotiator-branch-v1-training`
- `ink_ref`: `examples/ink/lucent-hostage-feed.negotiator-branch.v1.ink`
- `source_fixture_ref`: `examples/training-loops/lucent-hostage-feed-v0/lucent-hostage-feed-v0.initial-state.json`
- `scene_id`: `lucent-hostage-feed-negotiator-branch-v1`
- `protagonist_agent_id`: `mara_quell`
- `npc_agent_ids`: array of all NPC ids
- `local_awareness_summary`: text/source_refs blocks
- `projection_controls`: text/source_refs blocks for all required categories
- `branch_and_fold_plan`: include variables and ending gates
- `branches`: every material branch with required fields
- `mutation_policy`
- `audit` with `review_status`: `needs_revision` until reviewers accept it, and required checks booleans
