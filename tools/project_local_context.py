"""Project canonical Ghostlight state into character-local operating context.

This is the seam between durable state and response-model prompts. It may cite
canonical source paths for review, but the rendered prompt surface should not
ask a character model to interpret raw state variables or numeric activations.
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
DEFAULT_FIXTURE = ROOT / "examples" / "agent-state.cold-wake-story-lab.json"
DEFAULT_ANNOTATION = ROOT / "examples" / "ink" / "cold-wake-sanctuary-intake.training.json"
DEFAULT_OUT_DIR = ROOT / "examples" / "projected-contexts"

VOICE_LABELS = {
    "dryness": ("dry", "wry or emotionally underplayed"),
    "verbal_warmth": ("warm", "reassuring or gentle in wording"),
    "formality": ("formal", "institutional or ceremonially controlled"),
    "verbosity": ("expansive", "willing to spend words"),
    "pace": ("slow-paced", "measured rather than rushed"),
    "plainspoken_directness": ("plainspoken", "direct and low-ornament"),
    "lexical_precision": ("precise", "careful about exact terms"),
    "technical_density": ("technical", "comfortable with specialized language"),
    "technical_compression": ("compressed", "packs technical meaning tightly"),
    "figurative_language": ("figurative", "uses analogy or image"),
    "lyricism": ("lyrical", "leans toward musical or poetic phrasing"),
    "narrative_detail": ("contextual", "adds situation and consequence"),
    "emotional_explicitness": ("emotionally explicit", "names feelings directly"),
    "pointedness": ("pointed", "can sharpen a line"),
    "self_disclosure": ("self-disclosing", "reveals personal stakes"),
    "hedging": ("hedged", "marks uncertainty"),
    "certainty_marking": ("certain", "marks confidence strongly"),
    "politeness": ("polite", "maintains courtesy"),
    "coded_politeness": ("coded-politeness", "uses manners to manage conflict"),
    "ritualized_address": ("ritualized", "uses formal address or inherited phrases"),
    "register_switching": ("register-switching", "moves between social languages"),
    "dialect_marking": ("dialect-marked", "carries cultural speech markers"),
    "theatricality": ("theatrical", "performs the room"),
    "humor": ("humorous", "uses humor as social pressure"),
    "conversational_dominance": ("dominant", "takes conversational space"),
    "listening_responsiveness": ("responsive", "answers what was actually heard"),
    "question_asking": ("questioning", "uses questions to steer"),
    "profanity": ("profane", "uses profanity freely"),
}

PRESSURE_LABELS = {
    "control_pressure": "needs the situation to become governable",
    "suspicion": "expects hidden motives or missing information",
    "rigidity": "leans on structure when uncertainty rises",
    "volatility": "can swing quickly if pushed past control",
    "anxiety": "feels risk before it becomes visible",
    "hostility": "is ready to treat obstruction as adversarial",
    "withdrawal": "may reduce contact rather than keep negotiating",
    "attachment_seeking": "wants connection or backing to matter",
    "distance_seeking": "protects space when closeness becomes costly",
    "exhaustion": "has less spare tolerance than the words may show",
    "panic": "feels the clock as bodily pressure",
    "grievance_activation": "carries a live sense of unfairness",
    "overstimulation": "is taking in more demand than can be cleanly sorted",
    "scarcity_pressure": "knows that every yes spends a limited resource",
    "moral_disgust": "reads some choices as contaminating, not merely wrong",
    "perceived_status_threat": "feels exposed to loss of authority or standing",
}

PRESENTATION_LABELS = {
    "detachment": "keeps feeling at a controlled distance",
    "competence_theater": "performs capability so panic has nowhere obvious to land",
    "strategic_opacity": "leaves motives partly covered",
    "ironic_distance": "uses irony to avoid standing naked in sincerity",
    "compliance": "can make accommodation look voluntary",
    "moral_theater": "may stage the ethical frame for the room",
    "abrasive_boundary": "can use roughness as a fence",
    "cultivated_harmlessness": "makes danger look smaller than it is",
}


def load_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def write_json(path: Path, payload: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")


def find_agent(fixture: dict[str, Any], agent_id: str) -> dict[str, Any]:
    return next(agent for agent in fixture["agents"] if agent["agent_id"] == agent_id)


def find_scene(fixture: dict[str, Any], scene_id: str) -> dict[str, Any]:
    return next(scene for scene in fixture["scenes"] if scene["scene_id"] == scene_id)


def find_pack(scene: dict[str, Any], speaker_id: str) -> dict[str, Any]:
    return next(pack for pack in scene["dialogue_context_packs"] if pack["speaker_agent_id"] == speaker_id)


def find_relationship(fixture: dict[str, Any], source_id: str, target_id: str) -> dict[str, Any]:
    return next(rel for rel in fixture["relationships"] if rel["source_id"] == source_id and rel["target_id"] == target_id)


def memory_lookup(agent: dict[str, Any]) -> dict[str, dict[str, Any]]:
    memories: dict[str, dict[str, Any]] = {}
    for group in agent["memories"].values():
        for memory in group:
            memories[memory["memory_id"]] = memory
    return memories


def text_block(text: str, source_refs: list[str]) -> dict[str, Any]:
    return {"text": text, "source_refs": source_refs}


def activation_level(value: float) -> str:
    if value >= 0.78:
        return "dominant"
    if value >= 0.64:
        return "strong"
    if value >= 0.52:
        return "present"
    if value <= 0.22:
        return "muted"
    return "background"


def state_value(agent: dict[str, Any], group: str, name: str) -> float | None:
    item = agent.get("canonical_state", {}).get(group, {}).get(name)
    if not isinstance(item, dict):
        return None
    value = item.get("current_activation")
    if isinstance(value, (int, float)):
        return float(value)
    return None


def top_state_interpretations(
    agent: dict[str, Any],
    group: str,
    labels: dict[str, str],
    source_prefix: str,
    limit: int = 4,
) -> list[dict[str, Any]]:
    scored: list[tuple[float, str, str]] = []
    for name, summary in labels.items():
        value = state_value(agent, group, name)
        if value is not None:
            scored.append((value, name, summary))
    scored.sort(reverse=True)
    blocks: list[dict[str, Any]] = []
    for value, name, summary in scored[:limit]:
        level = activation_level(value)
        blocks.append(text_block(f"{level.capitalize()} pressure: {summary}.", [f"{source_prefix}.{group}.{name}"]))
    return blocks


def goal_blocks(agent: dict[str, Any], goal_ids: list[str], source_prefix: str) -> list[dict[str, Any]]:
    blocks = []
    for goal in agent.get("goals", []):
        if goal["goal_id"] in goal_ids:
            blocks.append(
                text_block(
                    f"{goal['description']} Emotional stake: {goal['emotional_stake']}",
                    [f"{source_prefix}.goals.{goal['goal_id']}"],
                )
            )
    return blocks


def memory_blocks(agent: dict[str, Any], memory_ids: list[str], source_prefix: str) -> list[dict[str, Any]]:
    memories = memory_lookup(agent)
    blocks = []
    for memory_id in memory_ids:
        if memory_id in memories:
            blocks.append(text_block(memories[memory_id]["summary"], [f"{source_prefix}.memories.{memory_id}"]))
    return blocks


def relationship_context(
    fixture: dict[str, Any],
    speaker_id: str,
    listener_ids: list[str],
) -> list[dict[str, Any]]:
    blocks = []
    for listener_id in listener_ids:
        relationship = find_relationship(fixture, speaker_id, listener_id)
        stance = relationship["stance"]
        strongest = sorted(
            (
                (values["current_activation"], name)
                for name, values in stance.items()
                if isinstance(values, dict) and isinstance(values.get("current_activation"), (int, float))
            ),
            reverse=True,
        )[:4]
        stance_text = ", ".join(f"{activation_level(value)} {name.replace('_', ' ')}" for value, name in strongest)
        blocks.append(
            text_block(
                f"{relationship['summary']} Current stance pressures: {stance_text}.",
                [f"relationships.{relationship['relationship_id']}"],
            )
        )
    return blocks


def perceived_overlay_context(agent: dict[str, Any], listener_ids: list[str], source_prefix: str) -> list[dict[str, Any]]:
    blocks = []
    for overlay in agent.get("perceived_state_overlays", []):
        if overlay.get("target_agent_id") not in listener_ids:
            continue
        beliefs = [belief["claim"] for belief in overlay.get("beliefs", [])]
        distortions = overlay.get("distortions", [])
        pieces = []
        if beliefs:
            pieces.append("Believes: " + " ".join(beliefs))
        if distortions:
            pieces.append("Known bias in the read: " + ", ".join(distortions) + ".")
        if pieces:
            blocks.append(
                text_block(
                    " ".join(pieces),
                    [f"{source_prefix}.perceived_state_overlays.{overlay['target_agent_id']}"],
                )
            )
    return blocks


def voice_surface(agent: dict[str, Any], source_prefix: str) -> list[dict[str, Any]]:
    voice = agent["canonical_state"].get("voice_style", {})
    scored: list[tuple[float, str, str, str]] = []
    for name, (label, description) in VOICE_LABELS.items():
        value = voice.get(name, {}).get("current_activation")
        if isinstance(value, (int, float)):
            scored.append((float(value), name, label, description))
    scored.sort(reverse=True)
    blocks = []
    for value, name, label, description in scored[:7]:
        blocks.append(
            text_block(
                f"{activation_level(value).capitalize()} voice tendency: {label}; {description}.",
                [f"{source_prefix}.canonical_state.voice_style.{name}"],
            )
        )
    return blocks


def known_fact_blocks(scene: dict[str, Any], pack: dict[str, Any], scene_prefix: str) -> list[dict[str, Any]]:
    blocks = []
    for index, fact in enumerate(pack.get("speaker_local_truth", [])):
        blocks.append(text_block(fact, [f"{scene_prefix}.dialogue_context_packs.{pack['speaker_agent_id']}.speaker_local_truth[{index}]"]))
    for index, belief in enumerate(pack.get("speaker_beliefs", [])):
        blocks.append(text_block(f"Speaker belief: {belief}", [f"{scene_prefix}.dialogue_context_packs.{pack['speaker_agent_id']}.speaker_beliefs[{index}]"]))
    return blocks


def scene_stakes(scene: dict[str, Any], speaker_id: str, scene_prefix: str) -> list[dict[str, Any]]:
    blocks = []
    for index, stake in enumerate(scene.get("public_stakes", [])):
        blocks.append(text_block(stake, [f"{scene_prefix}.public_stakes[{index}]"]))
    return blocks


def projection_control_blocks(annotation: dict[str, Any]) -> dict[str, Any]:
    return annotation["projection_controls"]


def action_affordances(agent_id: str, annotation: dict[str, Any]) -> list[dict[str, Any]]:
    controls = annotation["projection_controls"]
    if agent_id == annotation["protagonist_agent_id"]:
        return [
            text_block(
                "Can ask, offer ledger backing, show or withhold the packet, help with triage, wait, or press the moral frame. Sella controls intake.",
                ["examples/ink/cold-wake-sanctuary-intake.training.json.projection_controls.authority_boundaries"],
            ),
            text_block(
                "Can make the corrupted packet more present without treating it as proof.",
                ["examples/ink/cold-wake-sanctuary-intake.training.json.projection_controls.object_custody"],
            ),
        ]
    return [
        text_block(
            "Can inspect, refuse, withhold the packet from intake flow, demand ledger backing, set conditions, reserve a bounded bay, or walk away to protect triage.",
            ["examples/ink/cold-wake-sanctuary-intake.training.json.projection_controls.authority_boundaries"],
        ),
        text_block(
            "Must treat care as capacity: beds, food, air, consent, repair, and follow-through.",
            ["examples/ink/cold-wake-sanctuary-intake.training.json.projection_controls.required_semantics"],
        ),
    ]


def tension_blocks(agent: dict[str, Any], pack: dict[str, Any], annotation: dict[str, Any], source_prefix: str) -> list[dict[str, Any]]:
    goals = " ".join(goal["description"] for goal in agent.get("goals", []) if goal["goal_id"] in pack.get("active_goals", []))
    constraints = " ".join(pack.get("presentation_constraints", []))
    blocks = [
        text_block(
            f"Wants the goal to move now, but must preserve the social frame that makes the goal legitimate. Goal pressure: {goals}",
            [f"{source_prefix}.goals", "examples/ink/cold-wake-sanctuary-intake.training.json.projection_controls.frame_controls"],
        ),
        text_block(
            f"Must act under uncertainty without pretending uncertainty has become proof. Presentation pressure: {constraints}",
            [f"{source_prefix}.canonical_state.presentation_strategy", "examples/ink/cold-wake-sanctuary-intake.training.json.projection_controls.forbidden_resolutions"],
        ),
    ]
    if agent["agent_id"] == annotation["protagonist_agent_id"]:
        blocks.append(
            text_block(
                "Needs Sella's material help while knowing the ask is unfair and may spend someone else's sanctuary.",
                [f"{source_prefix}.memories", "relationships.rel-maer-sella"],
            )
        )
    else:
        blocks.append(
            text_block(
                "Respects Maer's rescue ethic but resents beautiful moral pressure when beds are already spent.",
                [f"{source_prefix}.memories", "relationships.rel-sella-maer"],
            )
        )
    return blocks


def likely_response_moves(agent: dict[str, Any], annotation: dict[str, Any], source_prefix: str) -> list[dict[str, Any]]:
    if agent["agent_id"] == annotation["protagonist_agent_id"]:
        return [
            text_block(
                "Likely to name the cost of the ask rather than hide it behind moral theater.",
                [f"{source_prefix}.canonical_state.behavioral_dimensions.grievance_activation", f"{source_prefix}.goals.goal-keep-rescue-live"],
            ),
            text_block(
                "May use ritualized route language when trying to make rescue obligation feel older than this room.",
                [f"{source_prefix}.canonical_state.voice_style.ritualized_address"],
            ),
            text_block(
                "May show the packet or offer ledger backing because action can carry the pressure more cleanly than another speech.",
                ["examples/ink/cold-wake-sanctuary-intake.training.json.branches"],
            ),
        ]
    return [
        text_block(
            "Likely to translate emotion into logistics: what bed, what air, what backing, what contamination risk.",
            [f"{source_prefix}.memories.memory-care-is-capacity"],
        ),
        text_block(
            "May withhold acceptance, demand conditions, or physically keep the packet out of intake flow until the cost is named.",
            ["examples/ink/cold-wake-sanctuary-intake.training.json.projection_controls.authority_boundaries"],
        ),
        text_block(
            "Can answer warmly while still refusing the frame that compassion manufactures capacity.",
            [f"{source_prefix}.canonical_state.voice_style.verbal_warmth", f"{source_prefix}.canonical_state.behavioral_dimensions.control_pressure"],
        ),
    ]


def render_section(title: str, blocks: list[dict[str, Any]]) -> list[str]:
    lines = [f"## {title}"]
    for block in blocks:
        lines.append(f"- {block['text']}")
    return lines


def render_controls(controls: dict[str, Any]) -> list[str]:
    lines = ["## Projection Controls"]
    for key in ["frame_controls", "authority_boundaries", "object_custody", "required_semantics", "forbidden_resolutions"]:
        if key in controls:
            label = key.replace("_", " ").title()
            for block in controls[key]:
                lines.append(f"- {label}: {block['text']}")
    return lines


def render_prompt_text(context: dict[str, Any]) -> str:
    lines = [
        f"# Ghostlight Local Operating Context: {context['local_agent_id']}",
        "",
        "Use this as character-local operating context, not omniscient story truth.",
        "Do not infer private state for anyone else unless it is explicitly framed as this character's current read.",
        "",
        f"## Speaker",
        context["identity"],
        "",
        f"## Setting",
        context["setting"],
        "",
    ]
    for title, key in [
        ("Known Facts", "known_facts"),
        ("Current Stakes", "current_stakes"),
        ("Active Inner Pressures", "active_inner_pressures"),
        ("Relationship Read", "relationship_read"),
        ("Tensions", "tensions"),
        ("Action Affordances", "action_affordances"),
    ]:
        lines.extend(render_section(title, context[key]))
        lines.append("")
    lines.extend(render_controls(context["projection_controls"]))
    lines.append("")
    lines.extend(render_section("Voice Surface", context["voice_surface"]))
    lines.append("")
    lines.extend(render_section("Likely Response Moves", context["likely_response_moves"]))
    lines.append("")
    lines.extend(render_section("Do Not Invent", context["do_not_invent"]))
    return "\n".join(lines).strip()


def build_projected_context(
    fixture: dict[str, Any],
    annotation: dict[str, Any],
    local_agent_id: str,
    mode: str,
    event_context: dict[str, Any] | None = None,
) -> dict[str, Any]:
    scene = find_scene(fixture, annotation["scene_id"])
    agent = find_agent(fixture, local_agent_id)
    pack = find_pack(scene, local_agent_id)
    listener_ids = pack["listener_ids"]
    source_prefix = f"agents.{local_agent_id}"
    scene_prefix = f"scenes.{scene['scene_id']}"

    active_inner_pressures = []
    active_inner_pressures.extend(top_state_interpretations(agent, "behavioral_dimensions", PRESSURE_LABELS, f"{source_prefix}.canonical_state", limit=4))
    active_inner_pressures.extend(top_state_interpretations(agent, "situational_state", PRESSURE_LABELS, f"{source_prefix}.canonical_state", limit=3))
    active_inner_pressures.extend(top_state_interpretations(agent, "presentation_strategy", PRESENTATION_LABELS, f"{source_prefix}.canonical_state", limit=2))
    active_inner_pressures.extend(goal_blocks(agent, pack.get("active_goals", []), source_prefix))
    active_inner_pressures.extend(memory_blocks(agent, pack.get("active_memories", []), source_prefix))

    do_not_invent = [
        text_block("Do not resolve whether the corrupted packet contains a person.", ["examples/ink/cold-wake-sanctuary-intake.training.json.projection_controls.forbidden_resolutions"]),
        text_block("Do not invent clean proof, clean capacity, or authority this character does not have.", ["examples/ink/cold-wake-sanctuary-intake.training.json.projection_controls"]),
    ]
    if event_context:
        do_not_invent.append(
            text_block("Treat the observed event as observable behavior only; private intent belongs to the actor and is not available.", ["runtime.observable_event"])
        )

    context = {
        "local_agent_id": local_agent_id,
        "identity": f"{agent['identity']['name']}: {', '.join(agent['identity']['roles'])}. Origin: {agent['identity']['origin']}. {agent['identity']['public_description']}",
        "setting": f"{scene['location']}. {fixture['world']['setting']}, {fixture['world']['time']['label']}.",
        "known_facts": known_fact_blocks(scene, pack, scene_prefix),
        "current_stakes": scene_stakes(scene, local_agent_id, scene_prefix),
        "active_inner_pressures": active_inner_pressures,
        "relationship_read": relationship_context(fixture, local_agent_id, listener_ids) + perceived_overlay_context(agent, listener_ids, source_prefix),
        "tensions": tension_blocks(agent, pack, annotation, source_prefix),
        "action_affordances": action_affordances(local_agent_id, annotation),
        "projection_controls": projection_control_blocks(annotation),
        "voice_surface": voice_surface(agent, source_prefix),
        "likely_response_moves": likely_response_moves(agent, annotation, source_prefix),
        "do_not_invent": do_not_invent,
    }
    if event_context:
        context["observed_event"] = event_context
        context["known_facts"].append(
            text_block(
                f"Observed event: {event_context['observable_action']} Spoken text: {event_context.get('spoken_text', '')}",
                ["runtime.observable_event"],
            )
        )

    return {
        "schema_version": "ghostlight.projected_local_context.v0",
        "projection_id": f"{annotation['scene_id']}.{local_agent_id}.{mode}",
        "input": {
            "fixture_ref": "examples/agent-state.cold-wake-story-lab.json",
            "annotation_ref": "examples/ink/cold-wake-sanctuary-intake.training.json",
            "scene_id": annotation["scene_id"],
            "local_agent_id": local_agent_id,
            "listener_ids": listener_ids,
            "projection_mode": mode,
        },
        "context": context | {"prompt_text": render_prompt_text(context)},
        "audit": {
            "projector": "tools/project_local_context.py",
            "raw_state_hidden_from_prompt": True,
            "character_local_context_only": True,
            "omitted_private_context": [
                "Other agents' canonical private state is not included.",
                "Actor private intent is not included in listener prompts; only observable events are included.",
                "Author-only hidden resolution of packet personhood is not included.",
            ],
            "review_status": "draft",
        },
    }


def main() -> int:
    parser = argparse.ArgumentParser(description="Project canonical state into character-local operating context.")
    parser.add_argument("--fixture", type=Path, default=DEFAULT_FIXTURE)
    parser.add_argument("--annotation", type=Path, default=DEFAULT_ANNOTATION)
    parser.add_argument("--out-dir", type=Path, default=DEFAULT_OUT_DIR)
    parser.add_argument("--agent-id", action="append", default=[])
    args = parser.parse_args()

    fixture = load_json(args.fixture)
    annotation = load_json(args.annotation)
    agent_ids = args.agent_id or [annotation["protagonist_agent_id"], *annotation["npc_agent_ids"]]
    for agent_id in agent_ids:
        projected = build_projected_context(fixture, annotation, agent_id, "scene_turn")
        out_path = args.out_dir / f"{annotation['scene_id']}.{agent_id}.projected-context.json"
        write_json(out_path, projected)
        print(f"wrote {out_path.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
