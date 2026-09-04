//! Eve/CultUI projection for the one live world owner.

use crate::{
    mesh::{COMMAND_BOUNDARY, COMMAND_RESULT_SCHEMA, PROVIDER_ID, SURFACE_ID},
    world::{ControllerMode, WorldPhase, WorldSnapshot},
};
use anyhow::{Context, bail};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

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

pub(crate) fn surface_version(snapshot: Option<&WorldSnapshot>) -> u64 {
    snapshot
        .map(|world| world.revision.saturating_add(1))
        .unwrap_or(0)
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

pub(crate) fn authenticated_surface(
    account: &str,
    snapshot: Option<&WorldSnapshot>,
) -> anyhow::Result<Value> {
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
            children.extend([
                json!({
                    "id":"world.create.title",
                    "kind":"control.input.text",
                    "props":{"label":"World title","placeholder":"A name for this world"},
                    "stateBindings":[local_draft("title", "string")],
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
                command_button(
                    "world.create",
                    "Create world",
                    "world.create",
                    json!({}),
                    &[
                        "title",
                        "subject_label",
                        "narrative_persona_label",
                        "operational_agent_label",
                    ],
                ),
            ]);
            commands.push(command_descriptor(
                "world.create",
                "ghostlight.world_create.v1",
                &[
                    "title",
                    "subject_label",
                    "narrative_persona_label",
                    "operational_agent_label",
                ],
                "WorldMailbox",
            ));
        }
        Some(world) => {
            children.push(json!({
                "id":"world.summary",
                "kind":"card",
                "props":{"title":world.title,"subtitle":format!("{:?} · revision {}", world.phase, world.revision)},
                "children":[{
                    "id":"world.summary.body",
                    "kind":"text",
                    "props":{"value":format!("{} subject(s), {} committed event(s)", world.subjects.len(), world.events.len())},
                    "children":[]
                }]
            }));
            match world.phase {
                WorldPhase::Draft => {
                    if world
                        .required_approvers
                        .contains(&crate::world::PrincipalId::new(account))
                        && !world
                            .draft_approvals
                            .contains(&crate::world::PrincipalId::new(account))
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
                    if world.owner == crate::world::PrincipalId::new(account)
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
                    let story = world
                        .events
                        .iter()
                        .filter_map(|event| {
                            event.invocation.speech.as_ref().map(|text| {
                                json!({
                                    "id":format!("world.event.{}", event.revision),
                                    "kind":"text",
                                    "props":{"value":text.as_str()},
                                    "children":[]
                                })
                            })
                        })
                        .collect::<Vec<_>>();
                    children.push(json!({
                        "id":"world.story",
                        "kind":"card",
                        "props":{"title":"Story"},
                        "children":story
                    }));

                    let principal = crate::world::PrincipalId::new(account);
                    let opportunity = world.opportunities.iter().find_map(|opportunity| {
                        world.subjects.iter().find_map(|subject| {
                            // The catalog entry is found by kind name among the
                            // subject's granted entries: the control is a
                            // projection of state, not a second vocabulary.
                            let affordance_id = world
                                .affordances
                                .iter()
                                .find(|entry| {
                                    entry.entry.kind.0 == "speak"
                                        && subject.affordances.contains(&entry.id)
                                })
                                .map(|entry| entry.id)?;
                            (subject.id == opportunity.scope.subject_id
                                && subject.human_controller.as_ref() == Some(&principal)
                                && opportunity.affordance_ids.contains(&affordance_id))
                            .then_some((opportunity, affordance_id))
                        })
                    });
                    if let Some((opportunity, affordance_id)) = opportunity {
                        children.extend([
                            json!({
                                "id":"world.speak.text",
                                "kind":"control.input.textarea",
                                "props":{"label":"Say something","rows":3,"placeholder":"Your exact words"},
                                "stateBindings":[local_draft("text", "string")],
                                "children":[]
                            }),
                            command_button(
                                "world.speak",
                                "Speak",
                                "world.speak",
                                json!({"opportunity":opportunity,"affordance_id":affordance_id}),
                                &["text"],
                            ),
                        ]);
                        commands.push(command_descriptor(
                            "world.speak",
                            "ghostlight.world_speak.v0",
                            &["text", "opportunity", "affordance_id"],
                            "WorldMailbox",
                        ));
                    }

                    if world.owner == crate::world::PrincipalId::new(account) {
                        let mut has_controller_command = false;
                        for (index, opportunity) in world
                            .opportunities
                            .iter()
                            .filter(|opportunity| {
                                opportunity.controller_mode != ControllerMode::Human
                            })
                            .enumerate()
                        {
                            let Some(subject) = world
                                .subjects
                                .iter()
                                .find(|subject| subject.id == opportunity.scope.subject_id)
                            else {
                                continue;
                            };
                            has_controller_command = true;
                            children.push(command_button(
                                &format!("world.controller.act.{index}"),
                                &format!("Let {} act", subject.label),
                                "world.controller.act",
                                json!({"opportunity":opportunity}),
                                &[],
                            ));
                        }
                        if has_controller_command {
                            commands.push(command_descriptor(
                                "world.controller.act",
                                "ghostlight.world_controller_act.v0",
                                &["opportunity"],
                                "ControllerRunner → WorldMailbox",
                            ));
                        }
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
        "world.create" => "ghostlight.world_create.v1",
        "world.approve" => "ghostlight.world_approve.v0",
        "world.activate" => "ghostlight.world_activate.v0",
        "world.speak" => "ghostlight.world_speak.v0",
        "world.controller.act" => "ghostlight.world_controller_act.v0",
        _ => return None,
    })
}

pub(crate) fn command_result(
    invocation: &EveCommandInvocation,
    state: &str,
    message: impl Into<String>,
    source_version: Option<u64>,
    plugin_payload: Option<Value>,
    receipt: Option<Value>,
) -> Value {
    json!({
        "schema":COMMAND_RESULT_SCHEMA,
        "providerId":PROVIDER_ID,
        "surfaceId":SURFACE_ID,
        "operationId":invocation.operation.operation_id,
        "idempotencyKey":invocation.operation.idempotency_key,
        "state":state,
        "message":message.into(),
        "sourceVersion":source_version,
        "pluginPayload":plugin_payload,
        "receipt":receipt,
        "updatedAtUtc":Utc::now().to_rfc3339()
    })
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

fn local_draft(key: &str, value_type: &str) -> Value {
    json!({"scope":"local-draft","key":key,"type":value_type})
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
        let surface = authenticated_surface("sha256:owner", None).unwrap();
        let encoded = serde_json::to_string(&surface).unwrap();
        assert!(encoded.contains("world.create"));
        assert!(encoded.contains("narrative_persona_label"));
        assert!(encoded.contains("operational_agent_label"));
        assert!(!encoded.contains("session_zero"));
        assert!(!encoded.contains("campaign"));
        assert_eq!(surface["version"], 0);
    }

    #[test]
    fn payload_cannot_claim_authority() {
        let forged = invocation(
            "world.speak",
            "ghostlight.world_speak.v0",
            json!({"text":"hello","caller":"owner"}),
        );
        assert!(validate_invocation(&forged, "https-json").is_err());
        let bound = invocation(
            "world.speak",
            "ghostlight.world_speak.v0",
            json!({"text":"hello","opportunity":{},"affordance_id":"bound"}),
        );
        assert!(validate_invocation(&bound, "https-json").is_ok());
    }

    #[test]
    fn invocation_metadata_rejects_legacy_authority_extensions() {
        let exact = invocation(
            "world.speak",
            "ghostlight.world_speak.v0",
            json!({"text":"hello","opportunity":{},"affordance_id":"bound"}),
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
    }
}
