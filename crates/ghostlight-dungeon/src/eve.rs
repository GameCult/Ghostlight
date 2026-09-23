//! Eve/CultUI projection for the one live world owner.

use crate::mesh::{COMMAND_BOUNDARY, COMMAND_RECEIPT_SCHEMA, COMMAND_RESULT_SCHEMA, OWNER_REPO, PROVIDER_ID, SURFACE_ID};
use crate::play::{PlayTurnState, PlayTurnView};
use ghostlight::{JurisdictionKey, WorldPhase, WorldSnapshot};
use anyhow::{Context, bail};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct EveCommandInvocation {
    pub(crate) schema: String,
    #[serde(rename = "providerId")]
    pub(crate) provider_id: String,
    #[serde(rename = "surfaceId")]
    pub(crate) surface_id: String,
    pub(crate) operation: EveOperation,
    pub(crate) payload: Value,
    #[serde(rename = "issuedAt")]
    pub(crate) issued_at: String,
    #[serde(rename = "clientId")]
    pub(crate) client_id: String,
    #[serde(rename = "commandBoundary")]
    pub(crate) command_boundary: String,
    #[serde(rename = "receiptSchema")]
    pub(crate) receipt_schema: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(crate) struct EveOperation {
    pub(crate) operation_id: String,
    pub(crate) schema_id: Option<String>,
    pub(crate) idempotency_key: Option<String>,
    #[serde(default)]
    pub(crate) route_hint: EveRouteHint,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(crate) struct EveRouteHint {
    pub(crate) source_version: Option<u64>,
    pub(crate) transport: Option<String>,
}

/// The `world_create.v4` payload, in one place, so the button that captures it
/// and the descriptor that advertises it cannot drift.
const CREATE_BINDINGS: [&str; 8] = [
    "title",
    "brief",
    "subject_label",
    "narrative_persona_label",
    "operational_agent_label",
    "targets",
    "jurisdictions",
    "lens_weights",
];

const SEED_BINDINGS: [&str; 2] = ["vault_scope", "brief"];

pub(crate) fn surface_version(snapshot: Option<&WorldSnapshot>) -> u64 {
    snapshot
        .map(|world| world.revision.saturating_add(1))
        .unwrap_or(0)
}

/// The wake signal `publish_projection` sends over the `revisions` broadcast
/// channel and `/api/eve/events` relays — never a value a client feeds back
/// as `routeHint.sourceVersion`, and never the surface document's own
/// `version` field (PA.f148/PA.f160), which names `surface_version` alone so
/// the kernel's `expected_revision = source_version - 1` derivation in
/// `runtime.rs` stays sound. It folds `surface_version` with the play row's
/// own `revision`, when a play view is available, so a play-row-only change
/// (a fresh refusal, a newly open question, a closed turn's narration, none
/// of which necessarily commit anything to the world) still wakes a watching
/// client even though `surface_version` alone would not move. Additive, not
/// paired: both counters only ever grow, so the sum only ever grows. No play
/// view (no turn has opened yet, or the table is unavailable) contributes
/// nothing, matching `surface_version`'s own reach when no play card renders
/// at all.
pub(crate) fn authenticated_wake_version(
    snapshot: Option<&WorldSnapshot>,
    play: Option<&PlayTurnView>,
) -> u64 {
    surface_version(snapshot).saturating_add(play.map_or(0, |view| view.revision))
}

pub(crate) fn world_state(snapshot: Option<&WorldSnapshot>) -> &'static str {
    match snapshot.map(|world| world.phase) {
        None => "empty",
        Some(WorldPhase::Draft) => "draft",
        Some(WorldPhase::Active) => "active",
    }
}

pub(crate) fn anonymous_surface() -> Value {
    anonymous_surface_at(0)
}

pub(crate) fn mesh_surface(snapshot: Option<&WorldSnapshot>) -> Value {
    anonymous_surface_at(surface_version(snapshot))
}

fn anonymous_surface_at(version: u64) -> Value {
    surface_document(
        version,
        "Ghostlight Dungeon",
        vec![
            json!({
                "id":"ghostlight.access",
                "kind":"heimdall.access_gate",
                "props":{
                    "state":"anonymous",
                    "title":"Enter Ghostlight",
                    "detail":"Sign in with Discord to reach the world owner."
                },
                "children":[]
            }),
            command_button(
                "ghostlight.auth.begin",
                "Continue with Discord",
                "heimdall.auth.begin",
                json!({}),
                &[],
            ),
        ],
        vec![command_descriptor(
            "heimdall.auth.begin",
            "heimdall.auth_begin_command.v1",
            &[],
            "Heimdall",
        )],
    )
}

/// What the seed lane has done and what it will answer next. Everything here is
/// derived from `scale_deficit` and `revision`: in Draft every commit is a patch
/// admission, so `revision - 1` is exactly the seed patches committed since
/// genesis, and "next shortfall" calls the runner's own `select_row` so the card
/// and the runner cannot disagree about what happens next.
///
/// It shows no in-flight session and no last outcome. Both would need a second
/// reader of the controller-work store from here and a status cache in
/// `AppState`; the last outcome is already in the invocation's own
/// `command_result` receipt, which is where a command's result belongs.
fn seed_card(world: &WorldSnapshot) -> Value {
    let next = ghostlight::select_row(world).map_or_else(
        || "Nothing short.".to_owned(),
        |row| {
            format!(
                "{} · {:?} · short {}",
                jurisdiction_label(world, row.jurisdiction),
                row.kind,
                row.deficit
            )
        },
    );
    let rows = world
        .scale_deficit
        .iter()
        .enumerate()
        .map(|(index, row)| {
            json!({
                "id":format!("world.seed.row.{index}"),
                "kind":"text",
                "props":{"value":format!(
                    "{} · {:?} · target {} · alive {} · short {}",
                    jurisdiction_label(world, row.jurisdiction),
                    row.kind,
                    row.target,
                    row.qualified,
                    row.deficit
                )},
                "children":[]
            })
        })
        .collect::<Vec<_>>();
    json!({
        "id":"world.seed.card",
        "kind":"card",
        "props":{
            "title":"Seed",
            "detail":format!("Draft patches committed: {}", world.revision.saturating_sub(1)),
            "nextShortfall":next
        },
        "children":rows
    })
}

fn jurisdiction_label(world: &WorldSnapshot, jurisdiction: JurisdictionKey) -> String {
    match jurisdiction {
        JurisdictionKey::PlaceSubtree(root) => world
            .places
            .iter()
            .find(|entry| entry.id == root)
            .map_or_else(
                || "an unnamed place subtree".to_owned(),
                |entry| entry.label.clone(),
            ),
        JurisdictionKey::Uncovered => "everything no declared root covers".to_owned(),
    }
}

/// `play` arrives beside the snapshot rather than inside it: `WorldSnapshot`
/// carries no play-turn state of its own. The play card below reads `play`
/// alone, never the operator's own unscoped log of every committed event:
/// invariant 8's projection, question, and refusal, nothing else.
pub(crate) fn authenticated_surface(
    account: &str,
    snapshot: Option<&WorldSnapshot>,
    play: Option<&PlayTurnView>,
) -> anyhow::Result<Value> {
    // The document's own `version` names the world's `surface_version`
    // alone — the one meaning `routeHint.sourceVersion` and the kernel's
    // compare-and-swap require of it (PA.f148/PA.f160). The play row's own
    // revision moves independently of it and is never folded in; it has no
    // wire reader of its own (PA.f165) and stays internal to
    // `eve::authenticated_wake_version`'s SSE wake computation.
    let version = surface_version(snapshot);
    let mut children = vec![json!({
        "id":"ghostlight.identity",
        "kind":"heimdall.identity",
        "props":{
            "state":"authenticated",
            "title":"Authenticated",
            "detail":"Heimdall identity is bound server-side.",
            "displayName":short_principal(account)
        },
        "children":[]
    })];
    let mut commands = Vec::new();

    match snapshot {
        None => {
            // Dungeon policy, editable before submission; the library holds no
            // default weights.
            let default_lens_weights =
                serde_json::to_string(&crate::runtime::uniform_lens_weights())
                    .context("the default lens weights encode")?;
            children.extend([
                json!({
                    "id":"world.create.title",
                    "kind":"control.input.text",
                    "props":{"label":"World title","placeholder":"A name for this world"},
                    "stateBindings":[local_draft("title", "string")],
                    "children":[]
                }),
                // Cut 10 (PA.f149): `CREATE_BINDINGS` has always advertised
                // `brief` — `world_create.v4`'s `CreatePayload.brief` is
                // required, not `#[serde(default)]` — but no control ever
                // captured it. Driving `world.create` with the real
                // vendored lowering surfaced this: nothing the lowering
                // could ever submit through this form would satisfy
                // `world_create.v4`, hand-built test payloads notwithstanding.
                json!({
                    "id":"world.create.brief",
                    "kind":"control.input.textarea",
                    // PA.f164: `brief` is documented "required, may be empty"
                    // (`CreatePayload::brief`), the same deliberate-empty
                    // shape as `targets`/`jurisdictions` below — an authored
                    // `value` so a form left untouched here still submits.
                    "props":{"label":"Brief","rows":2,"value":"","placeholder":"One sentence of what this world is for"},
                    "stateBindings":[local_draft("brief", "string")],
                    "children":[]
                }),
                json!({
                    "id":"world.create.subject",
                    "kind":"control.input.text",
                    "props":{"label":"Your name","placeholder":"The first person in the world"},
                    "stateBindings":[local_draft("subject_label", "string")],
                    "children":[]
                }),
                json!({
                    "id":"world.create.narrative_persona",
                    "kind":"control.input.text",
                    "props":{"label":"Narrative persona (optional)","placeholder":"A person who lives the story in prose"},
                    "stateBindings":[local_draft("narrative_persona_label", "string")],
                    "children":[]
                }),
                json!({
                    "id":"world.create.operational_agent",
                    "kind":"control.input.text",
                    "props":{"label":"Operational agent (optional)","placeholder":"An institution or operator-shaped mind"},
                    "stateBindings":[local_draft("operational_agent_label", "string")],
                    "children":[]
                }),
                json!({
                    "id":"world.create.targets",
                    "kind":"control.input.textarea",
                    // PA.f164: an authored `value`, not only a `placeholder` —
                    // `findAuthoredBindingValue` (the vendored lowering) only
                    // ever captures a control's `value`, never its
                    // `placeholder`, so a form submitted without editing this
                    // field would otherwise be refused as missing. `{}` is a
                    // legitimate `targets` (a world with no scale target is a
                    // deliberate choice, `CreatePayload`'s own doc comment).
                    "props":{"label":"Scale target","rows":2,"value":"{}","placeholder":"{\"person\": 12, \"institution\": 3}"},
                    "stateBindings":[local_draft("targets", "string")],
                    "children":[]
                }),
                json!({
                    "id":"world.create.jurisdictions",
                    "kind":"control.input.textarea",
                    // PA.f164: same gap, `[]` is a legitimate empty root list.
                    "props":{"label":"Jurisdiction roots","rows":3,"value":"[]","placeholder":"[{\"handle\":\"low_sere\",\"label\":\"The Low Sere\",\"permille\":700}]"},
                    "stateBindings":[local_draft("jurisdictions", "string")],
                    "children":[]
                }),
                json!({
                    "id":"world.create.lens_weights",
                    "kind":"control.input.textarea",
                    "props":{"label":"Lens weights","rows":2,"value":default_lens_weights,"placeholder":default_lens_weights},
                    "stateBindings":[local_draft("lens_weights", "string")],
                    "children":[]
                }),
                command_button(
                    "world.create",
                    "Create world",
                    "world.create",
                    json!({}),
                    &CREATE_BINDINGS,
                ),
            ]);
            commands.push(command_descriptor(
                "world.create",
                "ghostlight.world_create.v4",
                &CREATE_BINDINGS,
                "WorldMailbox",
            ));
        }
        Some(world) => {
            // Cut 10 (PA.f159): phase and clock stay; the subject census is
            // gone — nothing about who else is in the world is the player's
            // own projection, invariant 8's own "nothing else."
            children.push(json!({
                "id":"world.summary",
                "kind":"card",
                "props":{"title":world.title,"subtitle":format!("{:?} · revision {} · minute {}", world.phase, world.revision, world.now.0)},
                "children":[]
            }));
            match world.phase {
                WorldPhase::Draft => {
                    if world
                        .required_approvers
                        .contains(&ghostlight::PrincipalId::new(account))
                        && !world
                            .draft_approvals
                            .contains(&ghostlight::PrincipalId::new(account))
                    {
                        children.push(command_button(
                            "world.approve",
                            "Approve draft",
                            "world.approve",
                            json!({}),
                            &[],
                        ));
                        commands.push(command_descriptor(
                            "world.approve",
                            "ghostlight.world_approve.v0",
                            &[],
                            "WorldMailbox",
                        ));
                    }
                    if world.owner == ghostlight::PrincipalId::new(account) {
                        children.extend([
                            json!({
                                "id":"world.seed.vault_scope",
                                "kind":"control.input.text",
                                "props":{"label":"Vault scope (optional)","placeholder":"A subdirectory of the configured vault"},
                                "stateBindings":[local_draft("vault_scope", "string")],
                                "children":[]
                            }),
                            json!({
                                "id":"world.seed.brief",
                                "kind":"control.input.textarea",
                                "props":{"label":"Brief (optional)","rows":2,"placeholder":"One sentence of what this world is for"},
                                "stateBindings":[local_draft("brief", "string")],
                                "children":[]
                            }),
                            command_button(
                                "world.seed",
                                "Seed the world",
                                "world.seed",
                                json!({}),
                                &SEED_BINDINGS,
                            ),
                            seed_card(world),
                        ]);
                        commands.push(command_descriptor(
                            "world.seed",
                            "ghostlight.world_seed.v1",
                            &SEED_BINDINGS,
                            "SeedPort",
                        ));
                    }
                    if world.owner == ghostlight::PrincipalId::new(account)
                        && world.draft_approvals == world.required_approvers
                    {
                        children.push(command_button(
                            "world.activate",
                            "Activate world",
                            "world.activate",
                            json!({}),
                            &[],
                        ));
                        commands.push(command_descriptor(
                            "world.activate",
                            "ghostlight.world_activate.v0",
                            &[],
                            "WorldMailbox",
                        ));
                    }
                }
                WorldPhase::Active => {
                    if world.owner == ghostlight::PrincipalId::new(account) {
                        children.extend([
                            json!({
                                "id":"world.advance_time.minutes",
                                "kind":"control.input.number",
                                "props":{"label":"Advance minutes","min":1,"max":525_600},
                                "stateBindings":[local_draft("minutes", "number")],
                                "children":[]
                            }),
                            command_button(
                                "world.advance_time",
                                "Advance time",
                                "world.advance_time",
                                json!({}),
                                &["minutes"],
                            ),
                        ]);
                        commands.push(command_descriptor(
                            "world.advance_time",
                            "ghostlight.world_advance_time.v0",
                            &["minutes"],
                            "WorldMailbox",
                        ));
                    }
                    // Cut 10 (PA.f152): owner-gated, like `world.advance_time`
                    // and `world.seed` above — the play card, its controls,
                    // and the `world.play` descriptor are the owner's own
                    // play authority over their own world, not a public
                    // affordance.
                    if world.owner == ghostlight::PrincipalId::new(account) {
                        // Cut 9: the play card. Invariant 8 — the player sees
                        // a projection, the question, and the refusal of
                        // their own act, nothing else — so this reads `play`
                        // alone, never the operator's own unscoped event log:
                        // no id, no other subject's state, no speech the
                        // player did not perceive. Always rendered here, with
                        // empty rows before any turn has ever opened.
                        let mut play_rows = Vec::new();
                        if let Some(narration) = play.and_then(|view| view.narration.as_deref()) {
                            play_rows.push(json!({
                                "id":"world.play.narration",
                                "kind":"text",
                                "props":{"value":narration},
                                "children":[]
                            }));
                        }
                        if let Some(question) = play.and_then(|view| view.question.as_ref()) {
                            play_rows.push(json!({
                                "id":"world.play.question",
                                "kind":"text",
                                "props":{"value":question.text.as_str()},
                                "children":[]
                            }));
                        }
                        if let Some(refusal) = play.and_then(|view| view.refusal.as_deref()) {
                            play_rows.push(json!({
                                "id":"world.play.refusal",
                                "kind":"text",
                                "props":{"value":refusal},
                                "children":[]
                            }));
                        }
                        // PA.f193-D ("the interrupted turn"; renumbered from
                        // the PA.f188 duplicate this file's own comment used
                        // to carry — PA.f188 names only `f295a3a`'s
                        // construction-site coverage): a turn a process died
                        // mid-round leaves `Running` forever — nothing
                        // resumes it on its own, and the only way forward is
                        // submitting an empty message (`play.rs`'s own
                        // `Some(existing) if existing.state ==
                        // PlayTurnState::Running` admit arm: empty text
                        // continues it, non-empty text is refused as
                        // `TurnStillRunning`). Before this the card showed
                        // nothing at all in that state — no narration, no
                        // question, no refusal, since none of those are set
                        // on a turn that never got to close, ask, or record a
                        // refused act since it last opened — so a player had
                        // no way to learn the empty-submit continue exists at
                        // all. This is a control hint about the player's own
                        // turn, not world truth, so it stays within
                        // invariant 8 the same way the narration/question/
                        // refusal rows above already do.
                        //
                        // `&& !view.run_in_progress`: `state == Running`
                        // alone is not "nothing is happening" — it is also
                        // true for the seconds a healthy turn is genuinely
                        // running its own round loop. In that window `admit`
                        // fails `run_lock`'s own `try_lock_owned` before
                        // inspecting anything and refuses as `TurnBusy`,
                        // contradicting a hint that told the player an empty
                        // continue would be accepted. `run_in_progress` is
                        // `current_turn_view`'s own non-blocking peek at that
                        // same lock, so the hint now renders only for a
                        // `Running` turn nothing is actively working on.
                        if play.is_some_and(|view| view.state == PlayTurnState::Running && !view.run_in_progress) {
                            play_rows.push(json!({
                                "id":"world.play.running",
                                "kind":"text",
                                "props":{"value":"Your last turn did not finish. If nothing arrives, \
                                    submit an empty message to continue it."},
                                "children":[]
                            }));
                        }
                        // PA.f194-D: with the hint above correctly suppressed
                        // while a round is genuinely in flight
                        // (`run_in_progress`), the card had nothing left to
                        // render for the whole span between submitting a turn
                        // and its own narration/question/refusal landing —
                        // narration/question/refusal are all still whatever
                        // the *previous* turn left behind (or unset, on a
                        // fresh table), and the round in progress does not
                        // touch any of them until it closes or opens a
                        // question. A player who submits and sees an
                        // unchanged or empty card has no sign anything is
                        // happening. This row renders only while this
                        // player's own turn is actively being worked on —
                        // `run_in_progress`, not world truth, and stays
                        // within invariant 8 the same way the hint above
                        // does.
                        if play.is_some_and(|view| view.run_in_progress) {
                            play_rows.push(json!({
                                "id":"world.play.resolving",
                                "kind":"text",
                                "props":{"value":"Resolving your turn…"},
                                "children":[]
                            }));
                        }
                        children.push(json!({
                            "id":"world.play.card",
                            "kind":"card",
                            "props":{"title":"Play"},
                            "children":play_rows
                        }));

                        // Cut 10 (PA.f151): no `answers`/question-id control
                        // any more. The lowering has no authored, non-editable
                        // binding value (`hidden` is not a prop any renderer
                        // reads — Soul's own probe found it rendered as a
                        // plain, editable, visible text box holding the raw
                        // id), so the server resolves the open question
                        // itself instead of trusting a client-supplied id at
                        // all (`runtime.rs::execute_world`'s `world.play`
                        // arm). PA.f170's guarantee now holds by an opaque
                        // per-question token rather than PA.f161's version
                        // comparison (see `play::PlayTurn::question_token`'s
                        // own doc comment for why a version alone cannot tell
                        // two questions in the same turn apart): the button's
                        // own `props.action` carries it DOM-invisibly, the
                        // way `props.action.command` already does — a real
                        // client spreads a button's own `action` fields
                        // straight into the payload beside `bindings`
                        // (`commandPayload`,
                        // `vendor/eve/packages/eve-browser-lowering/src/index.ts:2101`),
                        // never through a renderer that could show or edit
                        // it. `None` while no question is open: the button's
                        // own action then carries no token at all, matching a
                        // plain continue/opening, which needs none.
                        // PA.f179: `question.token` is `Option<String>` — a
                        // `None` here (a question open whose own token was,
                        // for whatever reason, never minted) serializes as a
                        // JSON `null`, which `runtime.rs`'s own
                        // `answer_token_admits` gate refuses unconditionally,
                        // the same as any other mismatch. It is never
                        // rendered as an empty string a forged answer could
                        // match.
                        let play_action = play
                            .and_then(|view| view.question.as_ref())
                            .map(|question| json!({"answerToken": question.token}))
                            .unwrap_or_else(|| json!({}));
                        children.extend([
                            json!({
                                "id":"world.play.text",
                                "kind":"control.input.textarea",
                                "props":{"label":"What do you do?","rows":3,"placeholder":"Write freely"},
                                "stateBindings":[local_draft("text", "string")],
                                "children":[]
                            }),
                            command_button(
                                "world.play",
                                "Play",
                                "world.play",
                                play_action,
                                &["text"],
                            ),
                        ]);
                        commands.push(command_descriptor(
                            "world.play",
                            "ghostlight.world_play.v0",
                            &["text"],
                            "PlayTable",
                        ));
                    }
                }
            }
        }
    }

    children.push(command_button(
        "app.auth.logout",
        "Sign out",
        "app.auth.logout",
        json!({}),
        &[],
    ));
    commands.push(command_descriptor(
        "app.auth.logout",
        "ghostlight.app_logout.v2",
        &[],
        "AppSessionOwner",
    ));
    Ok(surface_document(
        version,
        snapshot
            .map(|world| world.title.as_str())
            .unwrap_or("Ghostlight Dungeon"),
        children,
        commands,
    ))
}

pub(crate) fn validate_invocation(
    invocation: &EveCommandInvocation,
    expected_transport: &str,
) -> anyhow::Result<()> {
    if invocation.schema != "gamecult.eve.command_invocation.v1"
        || invocation.provider_id != PROVIDER_ID
        || invocation.surface_id != SURFACE_ID
        || invocation.command_boundary != COMMAND_BOUNDARY
        || invocation.receipt_schema != COMMAND_RESULT_SCHEMA
    {
        bail!("invocation does not match the Ghostlight Eve boundary");
    }
    if invocation.operation.operation_id.trim().is_empty()
        || invocation
            .operation
            .idempotency_key
            .as_deref()
            .is_none_or(str::is_empty)
        || invocation
            .operation
            .schema_id
            .as_deref()
            .is_none_or(str::is_empty)
        || invocation.client_id.trim().is_empty()
        || invocation.issued_at.parse::<DateTime<Utc>>().is_err()
        || invocation.operation.route_hint.transport.as_deref() != Some(expected_transport)
    {
        bail!("invocation metadata is incomplete");
    }
    let expected = operation_schema(&invocation.operation.operation_id)
        .context("Eve operation is not advertised by Ghostlight")?;
    if invocation.operation.schema_id.as_deref() != Some(expected) {
        bail!("operation payload schema does not match its command descriptor");
    }
    if invocation.payload.get("caller").is_some()
        || invocation.payload.get("caller_id").is_some()
        || invocation.payload.get("callerId").is_some()
    {
        bail!("payload may not supply caller authority");
    }
    Ok(())
}

pub(crate) fn operation_schema(operation: &str) -> Option<&'static str> {
    Some(match operation {
        "heimdall.auth.begin" => "heimdall.auth_begin_command.v1",
        "heimdall.auth.complete" => "heimdall.auth_complete_command.v1",
        "app.auth.logout" => "ghostlight.app_logout.v2",
        "world.create" => "ghostlight.world_create.v4",
        "world.approve" => "ghostlight.world_approve.v0",
        "world.activate" => "ghostlight.world_activate.v0",
        "world.advance_time" => "ghostlight.world_advance_time.v0",
        "world.seed" => "ghostlight.world_seed.v1",
        "world.play" => "ghostlight.world_play.v0",
        _ => return None,
    })
}

/// One command result in the shape `gamecult.eve.command_result.v1` actually
/// admits: `additionalProperties: false` over exactly `schema`, `receipt`,
/// `transientProjection`, `pluginPayload` and `draftDirective`, with the
/// per-command detail (state, message, source version, ids) inside the
/// receipt, which is `gamecult.eve.command_receipt.v1` and does allow its own
/// extra fields.
///
/// The browser validates this envelope before it looks at anything in it, so
/// a flat result is not a lenient dialect of the contract — it is rejected
/// whole, with the client reporting "invalid Eve command result: data must
/// not have additional properties" and no command of any kind completing.
pub(crate) fn command_result(
    invocation: &EveCommandInvocation,
    state: &str,
    message: impl Into<String>,
    source_version: Option<u64>,
    plugin_payload: Option<Value>,
    receipt_extra: Option<Value>,
) -> Value {
    let message = message.into();
    // A refusal is otherwise visible only to the browser that received it.
    // The operator log gets the operation and the refusal's own message,
    // never the payload.
    if !matches!(state, "accepted" | "completed" | "pending") {
        tracing::warn!(
            operation = %invocation.operation.operation_id,
            state,
            message = %message,
            "Eve command not accepted"
        );
    }
    let mut receipt = json!({
        "schema":COMMAND_RECEIPT_SCHEMA,
        "receiptId":Uuid::new_v4().to_string(),
        "commandId":invocation.operation.idempotency_key.clone().unwrap_or_default(),
        "command":invocation.operation.operation_id,
        "state":state,
        "ownerRepo":OWNER_REPO,
        "authority":PROVIDER_ID,
        "providerId":PROVIDER_ID,
        "surfaceId":SURFACE_ID,
        "message":message,
        // The contract requires a source version and refuses a null one. A
        // command that reports no version reports the surface it was decided
        // against, which is 0 before any world exists.
        "sourceVersion":source_version.unwrap_or(0),
        "issuedAtUtc":Utc::now().to_rfc3339()
    });
    if let (Some(map), Some(extra)) = (receipt.as_object_mut(), receipt_extra) {
        if let Some(extra) = extra.as_object() {
            for (key, value) in extra {
                map.insert(key.clone(), value.clone());
            }
        }
    }
    let mut result = json!({
        "schema":COMMAND_RESULT_SCHEMA,
        "receipt":receipt
    });
    if let (Some(map), Some(payload)) = (result.as_object_mut(), plugin_payload) {
        map.insert("pluginPayload".into(), payload);
    }
    result
}

fn surface_document(
    version: u64,
    title: &str,
    children: Vec<Value>,
    commands: Vec<Value>,
) -> Value {
    json!({
        "type":"surface-state",
        "schema":"gamecult.eve.surface.v1",
        "providerId":PROVIDER_ID,
        "providerKind":"narrative.simulation",
        "title":title,
        // The one meaning `routeHint.sourceVersion` and the kernel's
        // compare-and-swap require of it (PA.f148/PA.f160): `surface_version`
        // alone. The play row's own revision has no wire reader (PA.f165)
        // and is never carried here.
        "version":version,
        "updatedAtUtc":Utc::now().to_rfc3339(),
        "surface":{
            "id":SURFACE_ID,
            "root":{"id":"ghostlight.root","kind":"surface","props":{},"children":children},
            "styles":{"tokens":{
                "colorBackground":"#0c1110",
                "colorPanel":"#17201d",
                "colorText":"#e8e1cf",
                "colorMuted":"#9aa69f",
                "colorAccent":"#d49b58"
            }}
        },
        "commands":commands
    })
}

fn command_button(id: &str, label: &str, command: &str, action: Value, bindings: &[&str]) -> Value {
    let mut action = action.as_object().cloned().unwrap_or_default();
    action.insert("command".into(), Value::String(command.into()));
    json!({
        "id":id,
        "kind":"control.button",
        "props":{"label":label,"command":command,"action":action,"captureBindings":bindings},
        "children":[]
    })
}

fn command_descriptor(command: &str, schema: &str, bindings: &[&str], authority: &str) -> Value {
    json!({
        "schema":"gamecult.eve.command.v1",
        "command":command,
        "payloadSchema":schema,
        "captureBindings":bindings,
        "transport":"https-json",
        "authority":authority
    })
}

/// A `captureBindings` entry the vendored browser lowering
/// (`@gamecult/eve-browser-lowering`, `EveStateBindingDescriptor`) actually
/// resolves, field-for-field the same shape its own test suite
/// (`test/host-isolation.test.mjs`) builds for its own `composer.message`
/// local-draft binding: `targetProp`, `pointerId`, `sourceId`, `schemaId`,
/// `routeKind`, `bindingName`, `valueKind`, `accessMode`, `authority`.
///
/// Two fields are load-bearing. `bindingName` must equal `key`, since `key`
/// is also the exact string Dungeon's own `captureBindings` array and
/// `command_descriptor` advertise — `findAuthoredBindingValue` and
/// `editableBindingContext` both match a binding by this field (falling back
/// to `pointerId`, then the control's own node id, only when it is absent).
/// `accessMode` must stay `"local-draft"`: `editableBindingContext`
/// (`src/index.ts:2198`) defaults an absent `accessMode` to `"read"` whenever
/// a binding is present at all, and a `"read"` control is disabled
/// (`:2205`) — an editable field with no `accessMode` here would render
/// disabled, not merely miscaptured. The remaining fields
/// (`targetProp`, `pointerId`, `sourceId`, `schemaId`, `routeKind`,
/// `valueKind`, `authority`) carry no runtime behavior in the current
/// lowering (grep confirms `valueKind` is read nowhere), but are still worth
/// stating correctly rather than leaving absent.
///
/// `value_kind` must be one of the lowering's own `valueKind` union
/// (`"string" | "number" | "boolean" | "choice" | "string-list"`); Dungeon's
/// own JSON-textarea fields (`targets`, `jurisdictions`, `lens_weights`) are,
/// client-side, plain text controls like any other — there is no JSON
/// `valueKind` — so callers pass `"string"` for those too, and the server
/// side parses the captured text as JSON (`deserialize_json_capture` in
/// `runtime.rs`).
fn local_draft(key: &str, value_kind: &str) -> Value {
    json!({
        "targetProp":"value",
        "pointerId":format!("ghostlight.local.{key}"),
        "sourceId":"eve.browser.local",
        "schemaId":"gamecult.eve.local_draft.v1",
        "routeKind":"in-process",
        "bindingName":key,
        "valueKind":value_kind,
        "accessMode":"local-draft",
        "authority":"eve.browser"
    })
}

fn short_principal(principal: &str) -> String {
    principal.chars().take(20).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn invocation(operation: &str, schema: &str, payload: Value) -> EveCommandInvocation {
        EveCommandInvocation {
            schema: "gamecult.eve.command_invocation.v1".into(),
            provider_id: PROVIDER_ID.into(),
            surface_id: SURFACE_ID.into(),
            operation: EveOperation {
                operation_id: operation.into(),
                schema_id: Some(schema.into()),
                idempotency_key: Some(uuid::Uuid::new_v4().to_string()),
                route_hint: EveRouteHint {
                    source_version: Some(0),
                    transport: Some("https-json".into()),
                },
            },
            payload,
            issued_at: Utc::now().to_rfc3339(),
            client_id: "fixture".into(),
            command_boundary: COMMAND_BOUNDARY.into(),
            receipt_schema: COMMAND_RESULT_SCHEMA.into(),
        }
    }

    #[test]
    fn anonymous_surface_advertises_only_authentication() {
        let surface = anonymous_surface();
        let commands = surface["commands"].as_array().unwrap();
        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0]["command"], "heimdall.auth.begin");
        assert_eq!(surface["version"], 0);
    }

    #[test]
    fn empty_authenticated_surface_has_create_without_session_zero() {
        let surface = authenticated_surface("sha256:owner", None, None).unwrap();
        let encoded = serde_json::to_string(&surface).unwrap();
        assert!(encoded.contains("world.create"));
        assert!(encoded.contains("narrative_persona_label"));
        assert!(encoded.contains("operational_agent_label"));
        assert!(!encoded.contains("session_zero"));
        assert!(!encoded.contains("campaign"));
        assert_eq!(surface["version"], 0);

        // The lens control's default is Dungeon's uniform policy, every stock
        // lens named, and it decodes as the weights the payload will carry.
        fn find<'a>(node: &'a Value, id: &str) -> Option<&'a Value> {
            if node["id"] == id {
                return Some(node);
            }
            node.as_object()?
                .values()
                .flat_map(|value| match value {
                    Value::Array(items) => items.iter().collect::<Vec<_>>(),
                    other => vec![other],
                })
                .find_map(|child| find(child, id))
        }
        let control = find(&surface, "world.create.lens_weights").expect("a lens weights control");
        let default = control["props"]["value"].as_str().expect("a default value");
        let decoded: ghostlight::LensWeights = serde_json::from_str(default).unwrap();
        assert_eq!(decoded, crate::runtime::uniform_lens_weights());
        assert_eq!(decoded.iter().count(), ghostlight::Lens::ALL.len());
        assert!(decoded.iter().all(|(_, weight)| weight == 1));
    }

    /// `world.play`'s own operation id resolves to its schema through
    /// `operation_schema`, so an invocation whose payload matches it
    /// validates, and a payload that tries to claim authority is refused —
    /// the same claim this test once made for the now-deleted speak
    /// operation, PA.f146's own precedent (Cut 8b's schema existed here
    /// before Cut 9 published `world.play`'s own descriptor and button,
    /// below).
    #[test]
    fn payload_cannot_claim_authority() {
        assert_eq!(operation_schema("world.play"), Some("ghostlight.world_play.v0"));

        let bound = invocation(
            "world.play",
            "ghostlight.world_play.v0",
            json!({"text":"I look around.", "answers": null}),
        );
        assert!(validate_invocation(&bound, "https-json").is_ok());

        let forged = invocation(
            "world.play",
            "ghostlight.world_play.v0",
            json!({"text":"I look around.", "answers": null, "caller":"owner"}),
        );
        assert!(validate_invocation(&forged, "https-json").is_err());
    }

    #[test]
    fn invocation_metadata_rejects_legacy_authority_extensions() {
        let exact = invocation(
            "world.play",
            "ghostlight.world_play.v0",
            json!({"text":"I look around.", "answers": null}),
        );
        let mut outer = serde_json::to_value(&exact).unwrap();
        outer["callerId"] = json!("legacy-owner");
        assert!(serde_json::from_value::<EveCommandInvocation>(outer).is_err());

        let mut route = serde_json::to_value(&exact).unwrap();
        route["operation"]["routeHint"]["caller"] = json!("legacy-owner");
        assert!(serde_json::from_value::<EveCommandInvocation>(route).is_err());
    }

    #[test]
    fn removed_operation_is_unknown() {
        assert!(operation_schema("session_zero.begin").is_none());
        assert!(operation_schema("world.assess").is_none());
        assert!(operation_schema("governance.time.propose").is_none());
        assert!(operation_schema("world.speak").is_none());
    }
}
